/**
 * Record the Discord link-preview artwork from the real ASCII water simulator.
 *
 * The site serves a fixed 1200x600 `/embed-scene` page that boots
 * `water-sim.js` at a coarse grid, types the greeting and is stirred by
 * scripted droplets. This script drives headless Chrome over the DevTools
 * protocol, records that page, and hands the frames to ffmpeg.
 *
 * It has no npm dependencies: Node 22+ ships a WebSocket client, and ffmpeg is
 * the only external tool. Run it while the site is up:
 *
 *     cargo run
 *     node tools/embed/record.mjs --slug alice --name "Hi, I'm Alice"
 *     node tools/embed/record.mjs --slug dave  --name "Hi, I'm Dave" --bio developer
 *
 * Outputs (into static/embed/):
 *   water-<slug>.gif   animated OpenGraph image / gallery item
 *
 * Bump ASSET_VERSION in src/embed.rs after regenerating, so Discord's media
 * CDN refetches the files.
 */
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, rmSync, existsSync, statSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const WIDTH = 1200;
const HEIGHT = 600;
const FPS = 20;
const LIMIT_SECONDS = 9.5;

const args = parseArgs(process.argv.slice(2));
const baseUrl = (args['base-url'] || 'http://localhost:8080').replace(/\/$/, '');
const slug = args.slug || 'alice';
const name = args.name || "Hi, I'm Alice";
const bio = args.bio || 'artist & developer';
const seconds = Number(args.seconds || 9.5);
const outDir = path.resolve(ROOT, args['out-dir'] || 'static/embed');
const framesDir = path.join(os.tmpdir(), `embed-frames-${slug}-${Date.now()}`);

if (seconds > LIMIT_SECONDS) {
  throw new Error(`keep the clip at ${LIMIT_SECONDS}s or less`);
}

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const key = argv[i];
    if (!key.startsWith('--')) continue;
    const value = argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[++i] : 'true';
    out[key.slice(2)] = value;
  }
  return out;
}

function chromeBinary() {
  if (process.env.CHROME_PATH) return process.env.CHROME_PATH;
  const candidates = [
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
    'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
    'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
  ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error('No Chrome/Edge found. Set CHROME_PATH to a headless-capable browser.');
  }
  return found;
}

async function checkServer() {
  try {
    const res = await fetch(`${baseUrl}/embed-scene`, { signal: AbortSignal.timeout(5000) });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
  } catch (error) {
    throw new Error(
      `Could not reach ${baseUrl}/embed-scene (${error.message}). Start the site with \`cargo run\` first.`
    );
  }
}

const CHROME_PORT = 9400 + Math.floor(Math.random() * 100);
const profileDir = path.join(os.tmpdir(), `chrome-embed-${Date.now()}`);
const chrome = spawn(chromeBinary(), [
  '--headless=new',
  `--remote-debugging-port=${CHROME_PORT}`,
  `--user-data-dir=${profileDir}`,
  '--no-first-run',
  '--no-default-browser-check',
  '--disable-gpu',
  '--hide-scrollbars',
  '--mute-audio',
  `--window-size=${WIDTH},${HEIGHT}`,
  '--force-device-scale-factor=1',
  '--disable-background-timer-throttling',
  '--disable-renderer-backgrounding',
  '--disable-backgrounding-occluded-windows',
  'about:blank',
], { stdio: 'ignore' });

let ws;
let id = 0;
const pending = new Map();
const frames = [];

try {
  await checkServer();
  const wsUrl = await debuggerUrl();
  ws = new WebSocket(wsUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = () => reject(new Error('could not attach to Chrome'));
  });
  ws.onmessage = (message) => {
    const data = JSON.parse(message.data);
    if (data.id && pending.has(data.id)) {
      const { resolve, reject } = pending.get(data.id);
      pending.delete(data.id);
      data.error ? reject(new Error(JSON.stringify(data.error))) : resolve(data.result);
    } else if (data.method === 'Page.screencastFrame') {
      frames.push(data.params);
      send('Page.screencastFrameAck', { sessionId: data.params.sessionId }, session).catch(() => {});
    }
  };

  const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
  const session = sessionId;

  await send('Page.enable', {}, session);
  await send('Emulation.setDeviceMetricsOverride', {
    width: WIDTH, height: HEIGHT, deviceScaleFactor: 1, mobile: false,
  }, session);

  const sceneUrl = `${baseUrl}/embed-scene?name=${encodeURIComponent(name)}&bio=${encodeURIComponent(bio)}`;
  console.log(`==> loading ${sceneUrl}`);
  await send('Page.navigate', { url: sceneUrl }, session);
  await waitFor(() => runtimeValue(session, 'typeof window.__arm === "function"'), 10000, 'scene script');
  await sleep(1200); // fonts, first layout, the initial empty grid

  console.log(`==> recording ${seconds}s at ${FPS}fps`);
  await send('Page.startScreencast', {
    format: 'png',
    maxWidth: WIDTH,
    maxHeight: HEIGHT,
    everyNthFrame: 1,
  }, session);

  const armed = frames.length;
  await runtimeValue(session, 'window.__arm(), true');
  await sleep(seconds * 1000);

  await send('Page.stopScreencast', {}, session);
  await sleep(250);

  // Frames before `__arm()` are blank; the next couple can still be capture
  // latency from that state, so start a hair after the typewriter begins.
  const usable = frames.slice(armed + 2);
  if (usable.length < 20) throw new Error(`only captured ${usable.length} frames`);

  const first = usable[0].metadata.timestamp;
  const count = Math.round(seconds * FPS);
  const picked = [];
  let cursor = 0;
  for (let i = 0; i < count; i++) {
    const target = first + i / FPS;
    while (
      cursor + 1 < usable.length &&
      Math.abs(usable[cursor + 1].metadata.timestamp - target) <=
        Math.abs(usable[cursor].metadata.timestamp - target)
    ) {
      cursor++;
    }
    picked.push(usable[cursor]);
  }

  mkdirSync(framesDir, { recursive: true });
  picked.forEach((frame, index) => {
    writeFileSync(
      path.join(framesDir, `f${String(index + 1).padStart(4, '0')}.png`),
      Buffer.from(frame.data, 'base64')
    );
  });

  mkdirSync(outDir, { recursive: true });
  const gif = path.join(outDir, `water-${slug}.gif`);
  const fadeOut = Math.max(0.1, seconds - 1.0);
  const fade = `fade=t=in:st=0:d=0.35,fade=t=out:st=${fadeOut}:d=1.0`;

  console.log('==> encoding GIF');
  await ffmpeg([
    '-y', '-framerate', String(FPS), '-i', path.join(framesDir, 'f%04d.png'),
    '-vf', `${fade},split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=sierra2_4a`,
    '-loop', '0', gif,
  ]);

  if (existsSync(gif)) {
    console.log(`==> ${path.relative(ROOT, gif)} (${Math.round(statSync(gif).size / 1024)} KiB)`);
  }
} finally {
  try { ws && ws.close(); } catch {}
  chrome.kill();
  await sleep(300);
  try { rmSync(profileDir, { recursive: true, force: true }); } catch {}
  try { rmSync(framesDir, { recursive: true, force: true }); } catch {}
}

// ─── helpers ───────────────────────────────────────────────────────

function send(method, params = {}, sessionId) {
  const message = { id: ++id, method, params };
  if (sessionId) message.sessionId = sessionId;
  ws.send(JSON.stringify(message));
  return new Promise((resolve, reject) => pending.set(message.id, { resolve, reject }));
}

async function debuggerUrl() {
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      const res = await fetch(`http://127.0.0.1:${CHROME_PORT}/json/version`);
      if (res.ok) return (await res.json()).webSocketDebuggerUrl;
    } catch {}
    await sleep(100);
  }
  throw new Error('Chrome did not open a debugging port');
}

async function waitFor(check, timeoutMs, label) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (await check()) return;
    await sleep(100);
  }
  throw new Error(`timed out waiting for ${label}`);
}

async function runtimeValue(session, expression) {
  const result = await send('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true,
  }, session);
  return result.result && result.result.value;
}

function ffmpeg(parameters) {
  return new Promise((resolve, reject) => {
    const proc = spawn(process.env.FFMPEG_PATH || 'ffmpeg', parameters, { stdio: 'inherit' });
    proc.on('error', reject);
    proc.on('close', (code) => (code === 0 ? resolve() : reject(new Error(`ffmpeg exited with ${code}`))));
  });
}
