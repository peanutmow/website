(function () {
    'use strict';

    const RAMP = ' .-:~,^;*=|>+\\/?&$%#@';
    const CELL_PX = 14;
    const FRAME_MS = 1000 / 60;
    const MAX_CATCHUP = 3;

    let wasm = null;
    const wasmReady = (async function () {
        const module = await import('/wasm/wasm_water_sim.js');
        wasm = await module.default();
        return module;
    })();
    window.waterSimReady = wasmReady;

    function createWaterSim(opts) {
        opts = opts || {};

        const mount = opts.mount || document.getElementById('ascii-water');
        if (!mount) {
            console.error('water-sim: no mount element');
            return null;
        }

        const GREETING = opts.greeting || "Hi, I'm Alice";
        const BUTTONS = opts.buttons || ['Socials', 'Gallery', 'Blog', 'Projects'];
        const SIZE = opts.size || function () {
            return { width: window.innerWidth, height: window.innerHeight };
        };

        let sim = null;
        let cols = 0;
        let rows = 0;
        let textNode = null;
        let animFrameId = null;
        let lastTimestamp = 0;
        let pendingMs = 0;
        let charWidth = 0;
        let charSpan = null;
        let typewriterTimer = null;
        let revealed = 0;
        let lastLayout = '';
        let disposed = false;
        const pending = [];

        const reduceMotion = window.matchMedia
            ? window.matchMedia('(prefers-reduced-motion: reduce)')
            : null;

        function fontSize() {
            return parseFloat(mount.style.fontSize) || CELL_PX;
        }

        function measureCharWidth() {
            const size = mount.style.fontSize;
            if (!size) return 8;
            try {
                if (!charSpan) {
                    charSpan = document.createElement('span');
                    charSpan.style.cssText = 'position:fixed;left:0;top:0;visibility:hidden;pointer-events:none;white-space:pre;font-family:"Courier New",monospace;line-height:1;letter-spacing:0;';
                    document.body.appendChild(charSpan);
                }
                charSpan.style.fontSize = size;
                charSpan.textContent = 'M'.repeat(200);
                charWidth = charSpan.offsetWidth / 200;
                return charWidth;
            } catch (error) {
                charWidth = parseFloat(size) * 0.54;
                return charWidth;
            }
        }

        function allocate(width, height) {
            if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) return;
            const newRows = Math.max(10, Math.ceil(height / CELL_PX));
            const fs = height / newRows;
            if (!Number.isFinite(fs) || fs <= 0) return;

            mount.style.fontSize = fs + 'px';
            mount.style.lineHeight = fs + 'px';
            measureCharWidth();

            const cw = charWidth || fs * 0.54;
            if (!Number.isFinite(cw) || cw <= 0) return;
            const newCols = Math.max(20, Math.ceil(width / cw));
            if (!Number.isFinite(newCols)) return;

            const changed = newCols !== cols || newRows !== rows;
            cols = newCols;
            rows = newRows;
            if (sim && changed) sim.resize(cols, rows);
        }

        function updatePreFontSize() {
            const height = SIZE().height;
            if (!height || !rows) return;
            const fs = height / rows;
            mount.style.fontSize = fs + 'px';
            mount.style.lineHeight = fs + 'px';
            measureCharWidth();
        }

        function onResize() {
            const size = SIZE();
            if (!Number.isFinite(size.width) || !Number.isFinite(size.height) || size.width <= 0 || size.height <= 0) return;
            mount.style.width = size.width + 'px';
            mount.style.height = size.height + 'px';
            allocate(size.width, size.height);
            updatePreFontSize();
        }

        function render() {
            if (!sim) return;

            if (opts.pageHooks) {
                const scrolled = Math.max(
                    window.scrollY || 0,
                    document.documentElement.scrollTop || 0,
                    document.body.scrollTop || 0
                );
                document.body.classList.toggle('hide-ui-text', scrolled > window.innerHeight * 0.4);
            }

            const hidden = opts.pageHooks && document.body.classList.contains('hide-ui-text');
            const text = sim.render(hidden);
            const mode = sim.mode();
            if (mount.dataset.mode !== mode) mount.dataset.mode = mode;

            if (textNode && textNode.parentNode === mount) {
                textNode.data = text;
            } else {
                mount.textContent = text;
                textNode = mount.firstChild;
            }

            if (opts.onLayout) {
                const layout = sim.layout_json();
                if (layout && layout !== lastLayout) {
                    lastLayout = layout;
                    try {
                        opts.onLayout(JSON.parse(layout));
                    } catch (error) {}
                }
            }
        }

        function tick(timestamp) {
            if (!timestamp) timestamp = performance.now();
            const elapsed = Math.min(250, Math.max(0, timestamp - lastTimestamp));
            lastTimestamp = timestamp;
            pendingMs = Math.min(pendingMs + elapsed, FRAME_MS * MAX_CATCHUP);

            let stepped = false;
            while (pendingMs >= FRAME_MS - 0.5) {
                if (sim) sim.step();
                pendingMs = Math.max(0, pendingMs - FRAME_MS);
                stepped = true;
            }
            if (stepped) render();
            animFrameId = requestAnimationFrame(tick);
        }

        function localPoint(clientX, clientY) {
            const rect = mount.getBoundingClientRect();
            return { x: clientX - rect.left, y: clientY - rect.top };
        }

        function gridPoint(clientX, clientY) {
            const cw = charWidth || fontSize() * 0.54 || 8;
            const point = localPoint(clientX, clientY);
            return { x: point.x / cw, y: point.y / fontSize() };
        }

        function onMouseMove(event) {
            if (!sim) return;
            const point = gridPoint(event.clientX, event.clientY);
            sim.pointer_move(point.x, point.y);
        }

        function onTouchMove(event) {
            const touch = event.touches[0];
            if (!touch) return;
            onMouseMove({ clientX: touch.clientX, clientY: touch.clientY });
        }

        function onTouchStart(event) {
            const touch = event.touches[0] || event.changedTouches[0];
            if (!touch || !sim) return;
            const point = gridPoint(touch.clientX, touch.clientY);
            sim.pointer_down(point.x, point.y);
        }

        function onMouseDown(event) {
            if (!sim) return;
            const point = gridPoint(event.clientX, event.clientY);
            sim.pointer_down(point.x, point.y);
        }

        function onMouseUp() {}

        function onMouseLeave() {
            if (sim) sim.pointer_leave();
        }

        function injectAt(screenX, screenY, intensity) {
            if (!sim || !cols || !rows) return;
            const cw = charWidth || fontSize() * 0.54 || 8;
            const gi = Math.min(cols - 2, Math.max(1, Math.round(screenX / cw)));
            const gj = Math.min(rows - 2, Math.max(1, Math.round(screenY / fontSize())));
            sim.inject_at(gi, gj, intensity);
        }

        function bindButtonBar() {
            const bar = opts.buttonBar;
            if (!bar) return;
            bar.querySelectorAll('.btn').forEach(function (button) {
                button.addEventListener('mouseenter', function () {
                    const rect = button.getBoundingClientRect();
                    for (let index = 0; index < 3; index++) {
                        injectAt(rect.left + rect.width * (0.2 + index * 0.3), rect.top + rect.height * 0.5, 0.4);
                    }
                });
                button.addEventListener('mousemove', function (event) {
                    injectAt(event.clientX, event.clientY, 0.15);
                });
            });
        }

        function bindEasterEggs() {
            if (!opts.easterEggs) return;

            window.__triggerAboutBlank = function () {
                if (sim) sim.trigger_about_blank();
            };

            document.addEventListener('keydown', function (event) {
                if (!sim) return;
                if (event.target.tagName === 'INPUT' || event.target.tagName === 'TEXTAREA' || event.target.isContentEditable) return;
                if (event.ctrlKey || event.altKey || event.metaKey) return;
                if (document.body.classList.contains('inspect-open')) return;
                if (event.key.length !== 1) return;

                const trigger = sim.key(event.key);
                if (!trigger) return;
                const delay = trigger === '42' ? 3000 : trigger === 'cowsay' ? 4000 : 5000;
                setTimeout(function () {
                    if (sim) sim.clear_egg();
                }, delay);
            });

            window.addEventListener('hashchange', function () {
                if (window.location.hash !== '#about:blank') return;
                if (sim) sim.trigger_about_blank();
                history.replaceState(null, '', window.location.pathname + window.location.search);
            });
        }

        function triggerHashEgg() {
            if (!opts.easterEggs || window.location.hash !== '#about:blank') return;
            if (sim) sim.trigger_about_blank();
            history.replaceState(null, '', window.location.pathname + window.location.search);
        }

        function startTypewriter() {
            revealed = 0;
            if (typewriterTimer) clearInterval(typewriterTimer);
            typewriterTimer = setInterval(function () {
                revealed++;
                if (sim) sim.typewriter_tick();
                if (revealed >= GREETING.length) {
                    clearInterval(typewriterTimer);
                    typewriterTimer = null;
                }
            }, 60);
        }

        function installListeners() {
            window.addEventListener('resize', onResize);

            if (opts.interactive !== false) {
                const target = opts.globalInput ? document : (opts.inputEl || mount);
                target.addEventListener('mousemove', onMouseMove);
                target.addEventListener('mousedown', onMouseDown);
                target.addEventListener('mouseup', onMouseUp);
                target.addEventListener('mouseleave', onMouseLeave);
                target.addEventListener('touchmove', onTouchMove, { passive: true });
                target.addEventListener('touchstart', onTouchStart, { passive: true });
            }

            bindButtonBar();
            bindEasterEggs();

            if (document.fonts && document.fonts.ready) {
                document.fonts.ready.then(function () {
                    const old = charWidth;
                    measureCharWidth();
                    if (Math.abs(charWidth - old) > 0.5) onResize();
                });
            }
            setTimeout(function () {
                const old = charWidth;
                measureCharWidth();
                if (Math.abs(charWidth - old) > 0.5) onResize();
            }, 1500);
        }

        const api = {
            setMode: function (mode, text) {
                if (!sim) {
                    pending.push(function (ready) { ready.set_mode(mode || '', typeof text === 'boolean' ? text : undefined); });
                    return;
                }
                sim.set_mode(mode || '', typeof text === 'boolean' ? text : undefined);
                render();
            },
            setTextVisible: function (visible) {
                if (!sim) {
                    pending.push(function (ready) { ready.set_text_visible(!!visible); });
                    return;
                }
                sim.set_text_visible(!!visible);
                render();
            },
            grid: function () {
                return {
                    cols: cols,
                    rows: rows,
                    charW: charWidth,
                    rowH: fontSize() || 14,
                    cellPx: CELL_PX
                };
            },
            ramp: function () {
                return sim ? sim.ramp() : RAMP;
            },
            field: function () {
                if (!sim || !wasm) return { cols: cols, rows: rows, dens: new Float32Array(0), u: new Float32Array(0), v: new Float32Array(0) };
                const length = cols * rows;
                const buffer = wasm.memory.buffer;
                return {
                    cols: cols,
                    rows: rows,
                    dens: new Float32Array(buffer, sim.dens_ptr(), length),
                    u: new Float32Array(buffer, sim.u_ptr(), length),
                    v: new Float32Array(buffer, sim.v_ptr(), length)
                };
            },
            paintCurrent: function (canvas) {
                if (!canvas || !sim) return;
                const dpr = Math.min(window.devicePixelRatio || 1, 2);
                const width = canvas.clientWidth || window.innerWidth;
                const height = canvas.clientHeight || window.innerHeight;
                const pixelWidth = Math.round(width * dpr);
                const pixelHeight = Math.round(height * dpr);
                if (canvas.width !== pixelWidth) canvas.width = pixelWidth;
                if (canvas.height !== pixelHeight) canvas.height = pixelHeight;

                const context = canvas.getContext('2d');
                context.setTransform(dpr, 0, 0, dpr, 0, 0);
                context.clearRect(0, 0, width, height);

                const cw = charWidth || fontSize() * 0.54 || 8;
                const rh = fontSize() || 14;
                const segments = sim.current_segments();
                const bands = [];
                for (let index = 0; index < 5; index++) bands.push(new Path2D());
                for (let index = 0; index + 4 < segments.length; index += 5) {
                    const band = Math.min(4, Math.max(0, segments[index + 4] | 0));
                    const path = bands[band];
                    path.moveTo(segments[index] * cw, segments[index + 1] * rh);
                    path.lineTo(segments[index + 2] * cw, segments[index + 3] * rh);
                }

                context.lineWidth = Math.max(0.75, Math.min(cw, rh) * 0.1);
                context.lineCap = 'round';
                context.strokeStyle = '#fff';
                for (let index = 0; index < bands.length; index++) {
                    context.globalAlpha = 0.08 + index * 0.065;
                    context.stroke(bands[index]);
                }
                context.globalAlpha = 1;
            },
            sampleAt: function (clientX, clientY) {
                if (!sim || !cols || !rows) return null;
                const cw = charWidth || fontSize() * 0.54 || 8;
                const rh = fontSize() || 14;
                const point = localPoint(clientX, clientY);
                const i = Math.floor(point.x / cw);
                const j = Math.floor(point.y / rh);
                if (i < 0 || j < 0 || i >= cols || j >= rows) return null;
                return JSON.parse(sim.sample_json(i, j));
            },
            clear: function () {
                if (!sim) { pending.push(function (ready) { ready.clear(); }); return; }
                sim.clear();
                render();
            },
            droplet: function (ci, cj, amount, radius) {
                const a = amount === undefined ? 0.9 : amount;
                const r = radius === undefined ? 4 : radius;
                if (!sim) { pending.push(function (ready) { ready.droplet(ci, cj, a, r); }); return; }
                sim.droplet(ci, cj, a, r);
                render();
            },
            vortex: function (ci, cj, strength) {
                const value = strength === undefined ? 0.4 : strength;
                if (!sim) { pending.push(function (ready) { ready.vortex(ci, cj, value); }); return; }
                sim.vortex(ci, cj, value);
                render();
            },
            stir: function () {
                if (!sim) { pending.push(function (ready) { ready.stir(); }); return; }
                sim.stir();
                render();
            },
            pause: function () {
                if (animFrameId !== null) {
                    cancelAnimationFrame(animFrameId);
                    animFrameId = null;
                }
            },
            resume: function () {
                if (animFrameId === null) {
                    lastTimestamp = performance.now();
                    pendingMs = 0;
                    animFrameId = requestAnimationFrame(tick);
                }
            },
            resize: function () {
                onResize();
                render();
            },
            isRunning: function () {
                return animFrameId !== null;
            },
            mask: function (rects) {
                const apply = function (ready) {
                    ready.clear_masks();
                    (Array.isArray(rects) ? rects : []).forEach(function (rect) {
                        if (!rect) return;
                        if (!Number.isFinite(rect.row) || !Number.isFinite(rect.col) || !Number.isFinite(rect.rows) || !Number.isFinite(rect.cols)) return;
                        ready.push_mask(rect.row, rect.col, rect.rows, rect.cols, rect.fade == null ? 2 : rect.fade);
                    });
                    ready.apply_masks();
                };
                if (!sim) { pending.push(apply); return; }
                apply(sim);
                render();
            },
            dispose: function () {
                disposed = true;
                api.pause();
                if (typewriterTimer) {
                    clearInterval(typewriterTimer);
                    typewriterTimer = null;
                }
                window.removeEventListener('resize', onResize);
                if (charSpan && charSpan.parentNode) charSpan.parentNode.removeChild(charSpan);
            }
        };

        onResize();
        installListeners();
        lastTimestamp = performance.now();
        animFrameId = requestAnimationFrame(tick);

        const seed = Number.isFinite(window.__SIM_SEED)
            ? (window.__SIM_SEED >>> 0)
            : ((Math.random() * 4294967296) >>> 0);

        if (window.__PARITY) {
            window.__parity = {
                step: function (count) {
                    for (let index = 0; index < (count || 1); index++) {
                        if (sim) sim.step();
                    }
                    render();
                    return !!sim;
                },
                frame: function () {
                    render();
                    return mount.textContent;
                },
                field: function () {
                    return api.field();
                },
                grid: function () {
                    return api.grid();
                }
            };
        }

        wasmReady.then(function (module) {
            if (disposed) return;
            sim = new module.WaterSim(cols, rows, seed);
            sim.configure(
                GREETING,
                BUTTONS.join('\n'),
                opts.mode || 'ascii',
                opts.text !== false,
                opts.typewriter !== false,
                !!opts.pageHooks,
                !!opts.easterEggs,
                !!(reduceMotion && reduceMotion.matches)
            );
            onResize();
            while (pending.length) pending.shift()(sim);
            triggerHashEgg();
            if (opts.typewriter === false) {
                sim.typewriter_done();
                revealed = GREETING.length;
            } else {
                sim.typewriter_start();
                startTypewriter();
            }
            render();
        }).catch(function (error) {
            console.error('water-sim: wasm failed to load', error);
        });

        return api;
    }

    function bootLanding() {
        const mount = document.getElementById('ascii-water');
        if (!mount) return;
        window.__pageWaterSim = createWaterSim({
            mount: mount,
            greeting: mount.dataset.greeting || undefined,
            buttons: mount.dataset.buttons ? mount.dataset.buttons.split('|') : undefined,
            pageHooks: true,
            globalInput: true,
            easterEggs: true,
            buttonBar: document.getElementById('button-bar'),
            onLayout: function (layout) { window.__asciiLayout = layout; },
            size: function () {
                return { width: window.innerWidth, height: window.innerHeight };
            }
        });
    }

    window.createWaterSim = createWaterSim;

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bootLanding);
    } else {
        bootLanding();
    }
})();
