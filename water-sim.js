/**
 * water-sim.js — ASCII Fluid Simulation
 *
 * Simplified Navier-Stokes solver (Jos Stam method) rendered as
 * brightness-mapped ASCII characters on a pitch-black background.
 *
 * The pointer pushes the fluid along the path it travelled; the intensity of
 * the fluid maps to character brightness. `createWaterSim` returns an instance bound to a <pre>,
 * which is how the guided tour runs small plates beside the page.
 */
(function () {
    'use strict';

    /**
     * Create a simulation bound to a <pre> element.
     *
     * Every instance owns its solver state, so the page background and the
     * small plates in the guided tour can run side by side.
     *
     * @param {Object} opts
     *   mount       HTMLElement  target <pre> (required)
     *   greeting    string       line typed into the water
     *   buttons     string[]     menu labels stamped into the water
     *   mode        string       'ascii' | 'density' | 'velocity' | 'grid' | 'text'
     *   text        boolean      draw the greeting/menu overlay
     *   bio         string       line under the greeting ('' hides it)
     *   showMenu    boolean      stamp the menu labels into the water
     *   interactive boolean      follow the pointer
     *   globalInput boolean      listen on document instead of the mount
     *   inputEl     HTMLElement  element that receives pointer events
     *   pageHooks   boolean      the landing page's scroll/socials behaviour
     *   easterEggs  boolean      type "42" / "cowsay" / "about:blank"
     *   typewriter  boolean      type the greeting out
     *   buttonBar   HTMLElement  ripple when its buttons are hovered
     *   onLayout    function     receives the live grid layout
     *   size        function     returns { width, height } in CSS pixels
     * @returns {Object} controls
     */
    function createWaterSim(opts) {
    opts = opts || {};

    // ================================================================
    // Configuration
    // ================================================================
    const CFG = {
        cellPx: 14,              // Target pixels per simulation cell
        viscosity: 0.0000002,    // Velocity diffusion rate (slightly higher = smoother)
        dt: 0.017,               // Time step (seconds, ~60fps)
        diffuseIters: 10,        // Gauss-Seidel iterations for diffusion
        pressureIters: 25,       // Gauss-Seidel iterations for pressure solve
        forceStrength: 3.0,      // Velocity impulse from mouse drag
        maxPushCells: 22,        // Longest stretch of one frame's drag that pushes the fluid
        vorticity: 8,            // Vorticity confinement: keeps small eddies spinning (0 = off)
        vorticityCap: 0.05,      // Ceiling on the confinement force per cell per step
        densityPerFrame: 1.0,    // Density emitted per frame from mouse
        brushRadius: 2,          // Radius of density brush in grid cells
        dissipation: 0.992,      // Per-frame density decay (lower = faster fade)
        velDissipation: 0.997,   // Per-frame velocity decay
        ambientForce: 0.0003,    // Tiny random forces (keeps water alive)
        ambientDensity: 0.0005,  // Subtle background shimmer density
        // Ordered by measured ink coverage in Courier New, so each step is a
        // real step up in brightness rather than a jump around it.
        chars: ' .-:~,^;*=|>+\\/?&$%#@',  // 21 levels, space → @ (dark → bright)
        fadeFps: 60              // Target frames per second
    };

    // ================================================================
    // State
    // ================================================================
    let cols, rows, total;
    // Velocity fields
    let u, v, uPrev, vPrev;
    // Density fields
    let dens, densPrev;
    // Projection scratch
    let pressure, divergence;
    // Vorticity confinement scratch
    let curl, curlAbs;

    // Mouse tracking (in grid coordinates)
    let mx = -1, my = -1;
    let mouseActive = false;
    let mouseDown = false;
    // Pointer segments since the last solver step. Browsers coalesce moves and
    // often deliver them as x-only then y-only deltas; keeping every segment
    // means a diagonal drag pushes diagonally instead of snapping to an axis.
    let trail = [];
    const TRAIL_MAX = 512;
    // Patches the caller wants kept clear of ink (see applyMasks).
    let masks = [];

    // DOM references
    let preEl;
    let animFrameId = null;
    let lastTimestamp = 0;
    // Milliseconds carried over between animation frames, so the solver keeps a
    // steady rate whatever the display or the tab throttling does.
    let pendingMs = 0;
    // The <pre>'s text node, reused instead of rebuilt on every frame.
    let textNode = null;
    // Respect the system's reduced-motion preference: no ambient drift.
    const reduceMotion = window.matchMedia
        ? window.matchMedia('(prefers-reduced-motion: reduce)')
        : null;
    const ambientMotion = function () { return !(reduceMotion && reduceMotion.matches); };

    // ── What this instance draws ──
    // The guided tour turns the same solver into separate teaching views:
    // the raw field, the grid, the velocity, the ramp, the type.
    let renderMode = opts.mode || 'ascii';
    let textVisible = opts.text !== false;

    /** CSS-pixel size of the surface this instance paints. */
    const SIZE = opts.size || function () {
        return { width: window.innerWidth, height: window.innerHeight };
    };

    // ── ASCII text overlay ──
    // Text rendered directly in the ASCII grid so it blends seamlessly with the water.
    let textOverlays = []; // Array of {row, col, char}

    // ── Easter egg overlays (temporary, cleared each frame if not re-added) ──
    let easterEggOverlays = []; // Array of {row, col, char, frames}
    let easterEggMode = '';     // 'blank', '42', 'cowsay', or ''

    // Typewriter effect for greeting
    // The page may set window.__SITE__.greeting so the greeting matches the
    // active profile (Alice on the personal site, Dave on the professional
    // one). Falls back to the personal greeting when the script is served raw.
    const GREETING = opts.greeting || (window.__SITE__ && window.__SITE__.greeting) || "Hi, I'm Alice";
    const BUTTONS = opts.buttons || (window.__SITE__ && window.__SITE__.buttons) || ['Socials', 'Gallery', 'Blog', 'Projects'];
    let greetingRevealed = 0;
    let greetingComplete = false;

    // ── Keyboard sequence detection ──
    const KEY_BUFFER_MAX = 20;
    let keyBuffer = [];

    // Sequences to detect
    const SEQUENCES = {
        'about:blank': { length: 11, label: 'blank' },
        'cowsay':      { length: 6,  label: 'cowsay' },
    };

    function checkKeySequences() {
        const buf = keyBuffer.join('');
        // Check exact matches
        if (buf === '42') {
            trigger42();
            keyBuffer = [];
            return;
        }
        for (const [seq, info] of Object.entries(SEQUENCES)) {
            if (buf.endsWith(seq)) {
                if (info.label === 'blank') triggerAboutBlank();
                else if (info.label === 'cowsay') triggerCowsay();
                keyBuffer = [];
                return;
            }
        }
    }

    // ── Easter egg triggers ──

    function triggerAboutBlank() {
        if (easterEggMode === 'blank') return;
        easterEggMode = 'blank';
        textOverlays = []; // Force recompute on next render
        // Clear the density field so the water goes blank
        for (let i = 0; i < total; i++) dens[i] = 0;
        easterEggOverlays = [];

        // Show "MISSION ACCOMPLISHED." centered
        const msg = 'REMOVE';
        const row = Math.floor(rows / 2);
        const col = Math.floor((cols - msg.length) / 2);
        for (let i = 0; i < msg.length; i++) {
            easterEggOverlays.push({ row, col: col + i, char: msg[i], frames: 300 });
        }
        // Subtitle
        const sub = 'TEST';
        const subRow = row + 2;
        const subCol = Math.floor((cols - sub.length) / 2);
        for (let i = 0; i < sub.length; i++) {
            easterEggOverlays.push({ row: subRow, col: subCol + i, char: sub[i], frames: 300 });
        }

        // Fade the egg out after 5 seconds
        setTimeout(function () {
            easterEggMode = '';
            easterEggOverlays = [];
        }, 5000);
    }

    function trigger42() {
        if (easterEggMode !== '') return;
        easterEggMode = '42';
        textOverlays = []; // Force recompute on next render

        const msg = 'The answer to life, the universe, and everything.';
        const row = Math.floor(rows / 2);
        const col = Math.max(0, Math.floor((cols - msg.length) / 2));
        easterEggOverlays = [];
        for (let i = 0; i < msg.length; i++) {
            easterEggOverlays.push({ row, col: col + i, char: msg[i], frames: 180 });
        }

        // Inject some density for visual flair
        for (let j = row - 4; j <= row + 4; j++) {
            for (let i = col - 2; i < col + msg.length + 2; i++) {
                if (i > 0 && i < cols - 1 && j > 0 && j < rows - 1) {
                    dens[idx(i, j)] = 0.3 + Math.random() * 0.2;
                }
            }
        }

        setTimeout(function () {
            easterEggMode = '';
            easterEggOverlays = [];
        }, 3000);
    }

    function triggerCowsay() {
        if (easterEggMode !== '') return;
        easterEggMode = 'cowsay';
        textOverlays = []; // Force recompute on next render

        const quotes = [
            "moo? i'd fork that repo.",
            "undefined is not a function. again.",
            "have you tried turning it off and on?",
            "it's not a bug, it's a feature.",
            "sudo apt-get install clue",
            "there are 10 types of people...",
            "the rust borrow checker says no.",
            "404: sleep not found.",
        ];
        const quote = quotes[Math.floor(Math.random() * quotes.length)];

        // Build ASCII cow
        const balloonTop  = '  ' + '_'.repeat(quote.length + 2);
        const balloonLine = ' < ' + quote + ' >';
        const balloonBot  = '  ' + '-'.repeat(quote.length + 2);
        const cow = [
            balloonTop,
            balloonLine,
            balloonBot,
            '        \\   ^__^',
            '         \\  (oo)\\_______',
            '            (__)\\       )\\/\\',
            '                ||----w |',
            '                ||     ||',
        ];

        const startRow = Math.floor(rows * 0.2);
        const startCol = Math.floor(cols * 0.25);
        easterEggOverlays = [];
        for (let r = 0; r < cow.length; r++) {
            const line = cow[r];
            for (let c = 0; c < line.length; c++) {
                easterEggOverlays.push({
                    row: startRow + r,
                    col: startCol + c,
                    char: line[c],
                    frames: 240
                });
            }
        }

        // Inject some density for visual flair
        for (let j = startRow - 1; j < startRow + cow.length + 1; j++) {
            for (let i = startCol - 1; i < startCol + 50; i++) {
                if (i > 0 && i < cols - 1 && j > 0 && j < rows - 1) {
                    dens[idx(i, j)] = 0.15 + Math.random() * 0.1;
                }
            }
        }

        setTimeout(function () {
            easterEggMode = '';
            easterEggOverlays = [];
        }, 4000);
    }

    // Expose easter egg triggers globally for URL hash detection
    if (opts.easterEggs) window.__triggerAboutBlank = triggerAboutBlank;

    function computeTextOverlays() {
        textOverlays = [];
        easterEggOverlays = easterEggOverlays.filter(function (o) { return o.frames > 0; });
        easterEggOverlays.forEach(function (o) { o.frames--; });

        if (easterEggOverlays.length > 0 || easterEggMode !== '') {
            textOverlays = textOverlays.concat(easterEggOverlays);
            return; // skip normal UI when egg is active
        }

        // Don't render UI text when a content page (like Socials) is open
        if (opts.pageHooks && document.body.classList.contains('hide-ui-text')) return;

        const name    = GREETING;
        const buttons = BUTTONS;
        const btnStr  = buttons.join('  ');
        // The embed scene wants the name and the line under it without the
        // menu, so both are optional here.
        const bio     = opts.bio === undefined ? 'artist & developer' : String(opts.bio);
        const showMenu = opts.showMenu !== false;

        // ---- Name (typewriter reveal) ----
        const nameRow = Math.floor(rows * 0.84);
        const nameCol = Math.floor((cols - name.length) / 2);
        const reveal = Math.min(greetingRevealed, name.length);
        for (let i = 0; i < reveal; i++) {
            textOverlays.push({ row: nameRow, col: nameCol + i, char: name[i] });
        }
        // Blinking cursor while typing
        if (!greetingComplete && reveal < name.length) {
            textOverlays.push({ row: nameRow, col: nameCol + reveal, char: '_' });
        }

        // ---- Buttons (single spaced row) ----
        const btnRow = showMenu ? Math.floor(rows * 0.89) : 0;
        const btnCol = showMenu ? Math.floor((cols - btnStr.length) / 2) : 0;
        if (showMenu) {
            for (let i = 0; i < btnStr.length; i++) {
                textOverlays.push({ row: btnRow, col: btnCol + i, char: btnStr[i] });
            }
        }

        // ---- Bio ----
        const bioRow = bio ? Math.floor(rows * 0.94) : 0;
        const bioCol = bio ? Math.floor((cols - bio.length) / 2) : 0;
        if (bio) {
            for (let i = 0; i < bio.length; i++) {
                textOverlays.push({ row: bioRow, col: bioCol + i, char: bio[i] });
            }
        }

        // Expose the live grid so the DOM overlay can align its click targets exactly
        if (opts.onLayout) {
            try {
                opts.onLayout({
                    rows: rows,
                    cols: cols,
                    charW: _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8,
                    rowH: parseFloat(preEl.style.fontSize) || 14,
                    nameRow: nameRow, nameCol: nameCol,
                    btnRow: btnRow, btnCol: btnCol,
                    bioRow: bioRow, bioCol: bioCol,
                    buttons: buttons,
                    btnStr: btnStr,
                    bio: bio
                });
            } catch (e) {}
        }
    }

    /** Measure the exact character width using a hidden DOM span (more reliable than canvas). */
    let _charWidth = 0;
    let _charSpan = null;
    function measureCharWidth() {
        if (!preEl) return 8;
        const fs = preEl.style.fontSize;
        if (!fs) return 8;
        try {
            if (!_charSpan) {
                _charSpan = document.createElement('span');
                _charSpan.style.cssText = 'position:fixed;left:0;top:0;visibility:hidden;pointer-events:none;white-space:pre;font-family:"Courier New",monospace;line-height:1;letter-spacing:0;';
                document.body.appendChild(_charSpan);
            }
            _charSpan.style.fontSize = fs;
            _charSpan.textContent = 'M'.repeat(200);
            const w = _charSpan.offsetWidth;
            _charWidth = w / 200;
            return _charWidth;
        } catch(e) {
            _charWidth = parseFloat(fs) * 0.54;
            return _charWidth;
        }
    }

    /** Re-measure char width after web fonts have loaded (if not already). */
    function ensureFontMeasurement() {
        if (document.fonts && document.fonts.ready) {
            document.fonts.ready.then(function() {
                // Re-measure now that fonts are definitely loaded
                var old = _charWidth;
                measureCharWidth();
                // If the width changed, the font just landed — recalculate grid
                if (Math.abs(_charWidth - old) > 0.5) {
                    onResize();
                }
            });
        }
    }

    // ================================================================
    // Grid helpers
    // ================================================================
    function idx(i, j) { return i + j * cols; }

    function allocate(width, height) {
        // Compute rows to fill height, then derive cols to fill (or slightly overflow) width.
        // Using actual measured character width for accuracy.
        if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) return;
        const newRows = Math.max(10, Math.ceil(height / CFG.cellPx));
        const fs = height / newRows;
        if (!Number.isFinite(fs) || fs <= 0) return;
        // Temporarily set font size to measure char width
        if (preEl) {
            preEl.style.fontSize = fs + 'px';
            preEl.style.lineHeight = fs + 'px';
            measureCharWidth();
        }
        const cw = _charWidth || fs * 0.54;
        if (!Number.isFinite(cw) || cw <= 0) return;
        const newCols = Math.max(20, Math.ceil(width / cw));
        if (!Number.isFinite(newCols)) return;

        // No-op if size unchanged
        if (newCols === cols && newRows === rows) return;

        // Keep the water through the resize: sample the old fields at the same
        // relative position instead of starting from an empty screen.
        const carry = cols ? { cols: cols, rows: rows, u: u, v: v, dens: dens } : null;

        cols = newCols;
        rows = newRows;
        total = cols * rows;

        u = new Float32Array(total);
        v = new Float32Array(total);
        uPrev = new Float32Array(total);
        vPrev = new Float32Array(total);
        dens = new Float32Array(total);
        densPrev = new Float32Array(total);
        pressure = new Float32Array(total);
        divergence = new Float32Array(total);
        curl = new Float32Array(total);
        curlAbs = new Float32Array(total);

        if (carry) carryField(carry);

        // Cell coordinates on the old grid mean nothing on the new one.
        mx = -1; my = -1;
        trail.length = 0;

        computeTextOverlays();
    }

    /** Bilinearly move a field from an older grid size onto the current one. */
    function carryField(old) {
        const sx = old.cols / cols;
        const sy = old.rows / rows;
        for (let j = 0; j < rows; j++) {
            const fj = clamp((j + 0.5) * sy - 0.5, 0, old.rows - 1);
            const j0 = Math.floor(fj);
            const j1 = Math.min(old.rows - 1, j0 + 1);
            const rj = fj - j0;
            for (let i = 0; i < cols; i++) {
                const fi = clamp((i + 0.5) * sx - 0.5, 0, old.cols - 1);
                const i0 = Math.floor(fi);
                const i1 = Math.min(old.cols - 1, i0 + 1);
                const ri = fi - i0;
                const a = i0 + j0 * old.cols;
                const b = i1 + j0 * old.cols;
                const c = i0 + j1 * old.cols;
                const d = i1 + j1 * old.cols;
                const p = i + j * cols;
                dens[p] = lerp(lerp(old.dens[a], old.dens[b], ri), lerp(old.dens[c], old.dens[d], ri), rj);
                u[p] = lerp(lerp(old.u[a], old.u[b], ri), lerp(old.u[c], old.u[d], ri), rj);
                v[p] = lerp(lerp(old.v[a], old.v[b], ri), lerp(old.v[c], old.v[d], ri), rj);
            }
        }
        densPrev.set(dens);
    }

    /** Recalculate font-size so the ASCII grid exactly fills the surface. */
    function updatePreFontSize() {
        if (!preEl) return;
        const h = SIZE().height;
        if (!h) return;
        const fs = h / rows; // font size based on height constraint
        preEl.style.fontSize = fs + 'px';
        preEl.style.lineHeight = fs + 'px';
        measureCharWidth();
    }

    // ================================================================
    // Boundary conditions
    // ================================================================
    function setBounds(arr, bType) {
        const Lc = cols - 1;
        const Lr = rows - 1;

        // Horizontal edges (top / bottom)
        for (let i = 1; i < Lc; i++) {
            arr[idx(i, 0)] = bType === 2 ? -arr[idx(i, 1)] : arr[idx(i, 1)];
            arr[idx(i, Lr)] = bType === 2 ? -arr[idx(i, Lr - 1)] : arr[idx(i, Lr - 1)];
        }
        // Vertical edges (left / right)
        for (let j = 1; j < Lr; j++) {
            arr[idx(0, j)] = bType === 1 ? -arr[idx(1, j)] : arr[idx(1, j)];
            arr[idx(Lc, j)] = bType === 1 ? -arr[idx(Lc - 1, j)] : arr[idx(Lc - 1, j)];
        }
        // Corners
        arr[idx(0, 0)] = (arr[idx(1, 0)] + arr[idx(0, 1)]) * 0.5;
        arr[idx(Lc, 0)] = (arr[idx(Lc - 1, 0)] + arr[idx(Lc, 1)]) * 0.5;
        arr[idx(0, Lr)] = (arr[idx(1, Lr)] + arr[idx(0, Lr - 1)]) * 0.5;
        arr[idx(Lc, Lr)] = (arr[idx(Lc - 1, Lr)] + arr[idx(Lc, Lr - 1)]) * 0.5;
    }

    // ================================================================
    // Math utilities
    // ================================================================
    function lerp(a, b, t) { return a + (b - a) * t; }

    function clamp(v, lo, hi) { return v < lo ? lo : v > hi ? hi : v; }

    /** Bilinear interpolation at fractional grid coordinates. */
    function bilin(arr, gi, gj) {
        const fi = Math.floor(gi), fj = Math.floor(gj);
        const ri = gi - fi, rj = gj - fj;
        const i0 = clamp(fi, 0, cols - 1);
        const i1 = clamp(fi + 1, 0, cols - 1);
        const j0 = clamp(fj, 0, rows - 1);
        const j1 = clamp(fj + 1, 0, rows - 1);

        const top = lerp(arr[idx(i0, j0)], arr[idx(i1, j0)], ri);
        const bot = lerp(arr[idx(i0, j1)], arr[idx(i1, j1)], ri);
        return lerp(top, bot, rj);
    }

    // ================================================================
    // Fluid solver — Jos Stam method
    // ================================================================

    /**
     * Diffuse: solve (1 - a·∇²)X = X0 via Gauss-Seidel relaxation.
     * @param {Float32Array} dst  Destination array
     * @param {Float32Array} src  Source array (X0)
     * @param {number} diffRate   Diffusion coefficient (viscosity)
     * @param {number} bType      Boundary type (0=scalar, 1=x-vel, 2=y-vel)
     * @param {number} iters      Number of Gauss-Seidel iterations
     */
    function diffuse(dst, src, diffRate, bType, iters) {
        const a = CFG.dt * diffRate * cols * rows;
        // With the shipped viscosity the coefficient is ~3e-5, so the solve
        // returns its input. Copying is the same answer without the iterations.
        if (a * iters < 1e-3) {
            dst.set(src);
            setBounds(dst, bType);
            return;
        }
        const denom = 1 + 4 * a;

        for (let iter = 0; iter < iters; iter++) {
            for (let j = 1; j < rows - 1; j++) {
                const off = j * cols;
                for (let i = 1; i < cols - 1; i++) {
                    const p = off + i;
                    dst[p] = (src[p] + a * (
                        dst[p - 1] + dst[p + 1] +
                        dst[p - cols] + dst[p + cols]
                    )) / denom;
                }
            }
            setBounds(dst, bType);
        }
    }

    /**
     * Project: make the velocity field divergence-free.
     * Solves ∇²p = ∇·w for pressure, then subtracts ∇p from velocity.
     */
    function project(velU, velV) {
        const h = 1.0 / Math.max(cols, rows);
        const invH = 1.0 / h;

        // --- 1. Compute divergence ---
        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                divergence[p] = -0.5 * h * (
                    velU[p + 1] - velU[p - 1] +
                    velV[p + cols] - velV[p - cols]
                );
                pressure[p] = 0;
            }
        }
        setBounds(divergence, 0);
        setBounds(pressure, 0);

        // --- 2. Solve Poisson equation for pressure (Gauss-Seidel) ---
        for (let iter = 0; iter < CFG.pressureIters; iter++) {
            for (let j = 1; j < rows - 1; j++) {
                const off = j * cols;
                for (let i = 1; i < cols - 1; i++) {
                    const p = off + i;
                    pressure[p] = (
                        pressure[p - 1] + pressure[p + 1] +
                        pressure[p - cols] + pressure[p + cols] +
                        divergence[p]
                    ) * 0.25;
                }
            }
            setBounds(pressure, 0);
        }

        // --- 3. Subtract pressure gradient from velocity ---
        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                velU[p] -= 0.5 * invH * (pressure[p + 1] - pressure[p - 1]);
                velV[p] -= 0.5 * invH * (pressure[p + cols] - pressure[p - cols]);
            }
        }
        setBounds(velU, 1);
        setBounds(velV, 2);
    }

    /**
     * Advect: semi-Lagrangian advection.
     * Trace each cell backwards through the velocity field and
     * bilinearly interpolate the source quantity.
     */
    function advect(dst, src, velU, velV) {
        // One scale for both axes. The old code used `cols` across and `rows`
        // down, so on a wide grid a diagonal current flattened into a sideways
        // one as it travelled.
        const step = CFG.dt * Math.sqrt(cols * rows);
        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                // Trace backwards in time
                let x = i - step * velU[p];
                let y = j - step * velV[p];
                x = clamp(x, 0.5, cols - 1.5);
                y = clamp(y, 0.5, rows - 1.5);
                dst[p] = bilin(src, x, y);
            }
        }
        setBounds(dst, 0);
    }

    // ================================================================
    // Forces & interaction
    // ================================================================

    /**
     * One net segment for everything the pointer did since the last solver
     * step. Browsers often report a diagonal drag as a staircase of x-only and
     * y-only events; using every individual segment would therefore push the
     * water on the axes. The net displacement over one frame is the true
     * direction of travel, so that is what the force follows.
     */
    function pointerSegment() {
        if (!trail.length) return null;
        const first = trail[0];
        const last = trail[trail.length - 1];
        const dx = last.x1 - first.x0;
        const dy = last.y1 - first.y0;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 0.01) return null;
        return { x0: first.x0, y0: first.y0, dx: dx, dy: dy, dist: dist };
    }

    /** Add velocity from the pointer's net movement (called BEFORE advection). */
    function addMouseForces() {
        if (!mouseActive) return;
        const seg = pointerSegment();
        if (!seg) return;

        // Only the stretch nearest the pointer pushes the fluid. A quick flick
        // can cover most of the screen in one frame, and handing the solver a
        // whole screen of momentum at once makes the projection ring and the
        // current break up along the axes.
        const span = Math.min(seg.dist, CFG.maxPushCells) / seg.dist;
        const x0 = seg.x0 + seg.dx * (1 - span);
        const y0 = seg.y0 + seg.dy * (1 - span);
        const dx = seg.dx * span;
        const dy = seg.dy * span;

        const steps = Math.max(1, Math.ceil(Math.sqrt(dx * dx + dy * dy)));
        const sx = dx / steps;
        const sy = dy / steps;

        for (let s = 1; s <= steps; s++) {
            const gi = clamp(Math.round(x0 + sx * s), 1, cols - 2);
            const gj = clamp(Math.round(y0 + sy * s), 1, rows - 2);
            const p = idx(gi, gj);

            // The summed impulse equals the pointer displacement, so the
            // wake points exactly where the pointer went. No perpendicular
            // term is added: a swirl is divergence-free, so the solver would
            // preserve it while projecting the real push away, leaving the
            // current running sideways to the mouse.
            u[p] += sx * CFG.forceStrength;
            v[p] += sy * CFG.forceStrength;
        }
    }

    /** Inject density along the net pointer movement (called AFTER advection). */
    function injectMouseDensity() {
        if (!mouseActive) return;
        const seg = pointerSegment();
        if (!seg) {
            trail.length = 0;
            return;
        }

        const steps = Math.max(1, Math.ceil(seg.dist));
        const sx = seg.dx / steps;
        const sy = seg.dy / steps;
        const amount = CFG.densityPerFrame / Math.sqrt(steps);
        const brush = CFG.brushRadius;

        for (let s = 1; s <= steps; s++) {
            const gi = clamp(Math.round(seg.x0 + sx * s), 1, cols - 2);
            const gj = clamp(Math.round(seg.y0 + sy * s), 1, rows - 2);

            for (let dj = -brush; dj <= brush; dj++) {
                for (let di = -brush; di <= brush; di++) {
                    const ni = clamp(gi + di, 1, cols - 2);
                    const nj = clamp(gj + dj, 1, rows - 2);
                    const d2 = di * di + dj * dj;
                    if (d2 <= brush * brush) {
                        const falloff = 1 - (Math.sqrt(d2) / (brush + 1));
                        const at = idx(ni, nj);
                        dens[at] += amount * falloff;
                        if (dens[at] > 1) dens[at] = 1;
                    }
                }
            }
        }

        trail.length = 0;
    }

    /** Random ambient velocity (BEFORE advection). */
    function addAmbientVelocity() {
        for (let i = 0; i < total; i++) {
            u[i] += (Math.random() - 0.5) * CFG.ambientForce;
            v[i] += (Math.random() - 0.5) * CFG.ambientForce;
        }
    }

    /** Subtle ambient density (AFTER advection so it doesn't get swept away). */
    function addAmbientDensity() {
        for (let i = 0; i < total; i++) {
            dens[i] += (Math.random() - 0.5) * CFG.ambientDensity;
            if (dens[i] < 0) dens[i] = 0;
        }
    }

    /**
     * Vorticity confinement. Small eddies are what make the field read as
     * water, but decaying momentum and the two projections flatten them.
     * Nudging each cell along the gradient of |curl| keeps them spinning.
     * It only amplifies rotation the flow already has, so unlike a swirl
     * added to the pointer it cannot turn a drag sideways.
     */
    function addVorticity() {
        if (!CFG.vorticity || !curl) return;

        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                const c = 0.5 * ((v[p + 1] - v[p - 1]) - (u[p + cols] - u[p - cols]));
                curl[p] = c;
                curlAbs[p] = Math.abs(c);
            }
        }
        setBounds(curl, 0);
        setBounds(curlAbs, 0);

        const gain = CFG.vorticity * CFG.dt;
        const cap = CFG.vorticityCap;
        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                const gx = curlAbs[p + 1] - curlAbs[p - 1];
                const gy = curlAbs[p + cols] - curlAbs[p - cols];
                const len = Math.sqrt(gx * gx + gy * gy);
                if (len < 1e-6) continue;
                const nx = gx / len;
                const ny = gy / len;
                // Clamp: confinement is positive feedback, and without a ceiling
                // one strong eddy can spin the whole field apart.
                let fx = gain * ny * curl[p];
                let fy = -gain * nx * curl[p];
                if (fx > cap) fx = cap; else if (fx < -cap) fx = -cap;
                if (fy > cap) fy = cap; else if (fy < -cap) fy = -cap;
                u[p] += fx;
                v[p] += fy;
            }
        }
        setBounds(u, 1);
        setBounds(v, 2);
    }

    // ================================================================
    // Main simulation step
    // ================================================================
    function simulate() {
        if (!cols || !rows || !u || !v || !dens) return;

        // ── Step 1: Add velocity forces (BEFORE advection) ──
        addMouseForces();        // velocity from mouse
        if (ambientMotion()) addAmbientVelocity();    // random ambient velocity

        // 2. Diffuse velocity
        diffuse(uPrev, u, CFG.viscosity, 1, CFG.diffuseIters);
        diffuse(vPrev, v, CFG.viscosity, 2, CFG.diffuseIters);

        // 3. Project (make divergence-free)
        project(uPrev, vPrev);

        // 4. Swap velocity buffers
        [u, uPrev] = [uPrev, u];
        [v, vPrev] = [vPrev, v];

        // 5. Advect velocity
        advect(uPrev, u, u, v);
        advect(vPrev, v, u, v);

        // 5b. Put the curl back that advection and decay have taken out
        addVorticity();

        // 6. Project again
        project(uPrev, vPrev);

        // 7. Swap velocity buffers
        [u, uPrev] = [uPrev, u];
        [v, vPrev] = [vPrev, v];

        // ── Step 8: Advect density (moves old density along velocity field) ──
        advect(densPrev, dens, u, v);

        // 9. Swap density buffers
        [dens, densPrev] = [densPrev, dens];

        // ── Step 10: Inject new density AFTER advection ──
        // This ensures freshly injected density stays at the cursor position
        // instead of being immediately swept away by the velocity field.
        injectMouseDensity();
        if (ambientMotion()) addAmbientDensity();

        // 11. Dissipate density & velocity
        for (let i = 0; i < total; i++) {
            dens[i] *= CFG.dissipation;
            u[i] *= CFG.velDissipation;
            v[i] *= CFG.velDissipation;
            // Keep a minimum threshold to avoid rendering noise
            if (dens[i] < 0.005) dens[i] = 0;
        }

        applyMasks();
    }

    /** Is this grid coordinate inside a masked patch? */
    function maskAt(i, j) {
        for (let m = 0; m < masks.length; m++) {
            const k = masks[m];
            if (i >= k.col && i < k.col + k.cols && j >= k.row && j < k.row + k.rows) return true;
        }
        return false;
    }

    /**
     * Keep patches of the grid clear of ink.
     *
     * The guided tour writes its own words into the character grid; without
     * this they would be printed over moving water and turn to mush. Rather
     * than dimming them with a panel, the water is thinned where the words
     * are, with a soft edge so it looks like it parts around them.
     */
    function applyMasks() {
        for (let m = 0; m < masks.length; m++) {
            const mask = masks[m];
            const r1 = Math.min(rows - 1, mask.row + mask.rows);
            const c1 = Math.min(cols - 1, mask.col + mask.cols);
            const fade = mask.fade == null ? 2 : mask.fade;
            for (let j = Math.max(0, mask.row); j < r1; j++) {
                const edgeJ = Math.min(j - mask.row, r1 - 1 - j);
                for (let i = Math.max(0, mask.col); i < c1; i++) {
                    const edgeI = Math.min(i - mask.col, c1 - 1 - i);
                    const edge = Math.min(edgeJ, edgeI);
                    const keep = fade > 0 ? Math.min(1, Math.max(0, edge / fade)) : 0;
                    const p = i + j * cols;
                    dens[p] *= keep;
                }
            }
        }
    }

    // ================================================================
    // ASCII rendering
    // ================================================================

    /** The glyphs used when the plate shows shade instead of letters. */
    const SHADE = ' ░▒▓█';
    /** Four strokes are enough to read a current without breaking the grid. */
    const STROKES = ['-', '\\', '|', '/'];

    /** The normal view: density mapped onto the character ramp. */
    function renderFluid(rowsArr) {
        const ramp = CFG.chars;
        const rampLen = ramp.length - 1;
        for (let j = 0; j < rows; j++) {
            const off = j * cols;
            let line = '';
            for (let i = 0; i < cols; i++) {
                // A touch of gain: density peaks around 0.85 by the time the
                // frame is drawn, so without it the densest glyphs never show.
                const d = Math.min(1, dens[off + i] * 1.15);
                if (d <= 0) {
                    line += ' ';
                } else {
                    line += ramp[Math.min(rampLen, Math.round(d * rampLen))];
                }
            }
            rowsArr[j] = line;
        }
    }

    /** The field without letters: shade blocks, lightly boosted to be legible. */
    function renderShade(rowsArr) {
        for (let j = 0; j < rows; j++) {
            const off = j * cols;
            let line = '';
            for (let i = 0; i < cols; i++) {
                const d = Math.min(1, dens[off + i] * 1.15);
                const step = d <= 0 ? 0 : Math.min(4, 1 + Math.floor(d * 4));
                line += SHADE[step];
            }
            rowsArr[j] = line;
        }
    }

    /** The velocity field as strokes; speed picks the weight, direction the angle. */
    function renderCurrent(rowsArr) {
        let peak = 0;
        for (let p = 0; p < total; p++) {
            const s = u[p] * u[p] + v[p] * v[p];
            if (s > peak) peak = s;
        }
        const scale = peak > 0 ? 1 / Math.sqrt(peak) : 0;
        for (let j = 0; j < rows; j++) {
            const off = j * cols;
            let line = '';
            for (let i = 0; i < cols; i++) {
                const p = off + i;
                const su = u[p] * scale;
                const sv = v[p] * scale;
                const speed = Math.sqrt(su * su + sv * sv);
                if (speed < 0.08) {
                    line += ' ';
                } else if (speed < 0.32) {
                    line += '.';
                } else {
                    let angle = Math.atan2(sv, su);
                    if (angle < 0) angle += Math.PI;
                    line += STROKES[Math.round(angle / (Math.PI / 4)) % 4];
                }
            }
            rowsArr[j] = line;
        }
    }

    /** The lattice itself: one dot per cell, a cross every fifth. */
    function renderGrid(rowsArr) {
        const hasMasks = masks.length > 0;
        for (let j = 0; j < rows; j++) {
            let line = '';
            for (let i = 0; i < cols; i++) {
                // Under the tour's words the lattice thins to every fourth
                // dot: still a texture, but quiet enough to read through.
                if (hasMasks && maskAt(i, j)) {
                    line += ((i + j) % 4 === 0) ? '.' : ' ';
                    continue;
                }
                line += (i % 5 === 0 && j % 5 === 0) ? '+' : '.';
            }
            rowsArr[j] = line;
        }
    }

    /** Build the character frame for the current mode and update the DOM. */
    function render() {
        if (!preEl || !cols || !rows) return;

        const rowsArr = new Array(rows);

        if (renderMode === 'text') {
            for (let j = 0; j < rows; j++) rowsArr[j] = ' '.repeat(cols);
        } else if (renderMode === 'grid') {
            renderGrid(rowsArr);
        } else if (renderMode === 'velocity') {
            renderCurrent(rowsArr);
        } else if (renderMode === 'density') {
            renderShade(rowsArr);
        } else {
            renderFluid(rowsArr);
        }

        // ── Overlay text directly into the character grid ──
        // Auto-hide text when scrolled past the first viewport (content panel visible)
        if (opts.pageHooks) {
            const socials = document.getElementById('socials-inline');
            if (socials && socials.classList.contains('visible')) {
                if (window.scrollY > window.innerHeight * 0.4) {
                    document.body.classList.add('hide-ui-text');
                } else {
                    document.body.classList.remove('hide-ui-text');
                }
            }
        }

        const hiddenByPage = opts.pageHooks && document.body.classList.contains('hide-ui-text');
        if (textVisible && !hiddenByPage) {
            // Always recompute so erosion/density checks run every frame
            computeTextOverlays();
            for (let t = 0; t < textOverlays.length; t++) {
                const { row, col, char } = textOverlays[t];
                if (row >= 0 && row < rows && col >= 0 && col < cols) {
                    const r = rowsArr[row];
                    rowsArr[row] = r.substring(0, col) + char + r.substring(col + 1);
                }
            }
        }

        if (preEl.dataset.mode !== renderMode) preEl.dataset.mode = renderMode;

        // Reuse the text node: replacing it every frame churns the DOM for a
        // string that is always the same shape.
        const text = rowsArr.join('\n');
        if (textNode && textNode.parentNode === preEl) {
            textNode.data = text;
        } else {
            preEl.textContent = text;
            textNode = preEl.firstChild;
        }
    }

    // ================================================================
    // Animation loop
    // ================================================================
    // The solver keeps a fixed step so its constants stay valid, but the
    // number of steps comes from the wall clock. A 120 Hz display no longer
    // runs the water at double speed, and a throttled tab no longer slows it.
    const FRAME_MS = 1000 / CFG.fadeFps;
    const MAX_CATCHUP = 3;

    function tick(timestamp) {
        if (!timestamp) timestamp = performance.now();
        const elapsed = Math.min(250, Math.max(0, timestamp - lastTimestamp));
        lastTimestamp = timestamp;
        pendingMs = Math.min(pendingMs + elapsed, FRAME_MS * MAX_CATCHUP);

        let stepped = false;
        // Half a millisecond of slack: frame timestamps are not exact, and a
        // step that lands one float epsilon short would never run.
        while (pendingMs >= FRAME_MS - 0.5) {
            simulate();
            pendingMs = Math.max(0, pendingMs - FRAME_MS);
            stepped = true;
        }
        if (stepped) render();
        animFrameId = requestAnimationFrame(tick);
    }

    // ================================================================
    // Mouse / touch event handlers
    // ================================================================
    /** Pointer position in the surface's own coordinates. */
    function localPoint(clientX, clientY) {
        const r = preEl.getBoundingClientRect();
        return { x: clientX - r.left, y: clientY - r.top };
    }

    function onMouseMove(e) {
        // Map surface pixels → grid coords using the measured character width.
        // Characters are left-aligned, so the local offset corresponds directly
        // to a character position (the landing page's pre sits at 0,0).
        const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
        const fSize = parseFloat(preEl.style.fontSize) || 14;
        const pt = localPoint(e.clientX, e.clientY);
        const gx = pt.x / charW;
        const gy = pt.y / fSize;

        if (mx >= 0 && my >= 0) {
            const dx = gx - mx;
            const dy = gy - my;
            if (dx * dx + dy * dy > 1e-6 && trail.length < TRAIL_MAX) {
                trail.push({ x0: mx, y0: my, x1: gx, y1: gy });
            }
        }

        mx = gx;
        my = gy;
        mouseActive = true;
    }

    function onTouchMove(e) {
        // No preventDefault: `touch-action: pan-y` on the surface lets a
        // vertical swipe scroll the page instead of stirring.
        const touch = e.touches[0];
        if (!touch) return;
        onMouseMove({ clientX: touch.clientX, clientY: touch.clientY });
    }

    function onTouchStart(e) {
        const touch = e.touches[0] || e.changedTouches[0];
        if (!touch) return;
        // Initialize mouse position without velocity
        const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
        const fSize = parseFloat(preEl.style.fontSize) || 14;
        const pt = localPoint(touch.clientX, touch.clientY);
        mx = pt.x / charW;
        my = pt.y / fSize;
        trail.length = 0;
        mouseActive = true;
    }

    function onMouseLeave() {
        mouseActive = false;
        mx = -1;
        my = -1;
        trail.length = 0;
    }

    function onMouseDown(e) {
        mouseDown = true;
        if (mx < 0) {
            const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
            const fSize = parseFloat(preEl.style.fontSize) || 14;
            const pt = localPoint(e.clientX, e.clientY);
            mx = pt.x / charW;
            my = pt.y / fSize;
        }
        mouseActive = true;
    }

    function onMouseUp(e) {
        mouseDown = false;
    }

    // ================================================================
    // Resize handler
    // ================================================================
    function onResize() {
        const { width: w, height: h } = SIZE();
        // A surface that is still being laid out reports zero (or, mid-font
        // swap, a non-number). Skip those frames rather than build a NaN grid.
        if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0) return;
        // Set pre dimensions to exactly match the surface
        // (not 100vw/vh which differ when a scrollbar is present)
        if (preEl) {
            preEl.style.width = w + 'px';
            preEl.style.height = h + 'px';
        }
        allocate(w, h);
        updatePreFontSize();
    }

    // ================================================================
    // Typewriter
    // ================================================================

    function startTypewriter() {
        greetingRevealed = 0;
        greetingComplete = false;

        var interval = setInterval(function () {
            greetingRevealed++;
            // Rebuild overlays with new reveal count
            // We recompute without clearing the rest of the overlays
            // by removing old greeting entries and adding new ones
            var nameLen = GREETING.length;
            if (greetingRevealed >= nameLen) {
                greetingRevealed = nameLen;
                greetingComplete = true;
                clearInterval(interval);
            }
            computeTextOverlays();
        }, 60);
    }

    // ================================================================
    // Initialization
    // ================================================================
    function init() {
        // Capture DOM elements
        preEl = opts.mount || document.getElementById('ascii-water');
        if (!preEl) {
            console.error('water-sim: no mount element');
            return;
        }

        // Set up grid
        onResize();

        // ── Re-measure once web fonts are definitely loaded ──
        ensureFontMeasurement();
        // Backup: forced re-measure after fonts should be settled
        setTimeout(function() {
            var old = _charWidth;
            measureCharWidth();
            if (Math.abs(_charWidth - old) > 0.5) {
                onResize();
            }
        }, 1500);

        // ── Typewriter greeting ──
        if (opts.typewriter === false) {
            greetingRevealed = GREETING.length;
            greetingComplete = true;
            computeTextOverlays();
        } else {
            startTypewriter();
        }

        if (opts.easterEggs) {
            // ── Keyboard easter egg listener ──
            document.addEventListener('keydown', function (e) {
                // Only capture keys, ignore if typing in an input
                if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable) return;
                if (e.ctrlKey || e.altKey || e.metaKey) return;
                // Inspect mode owns the keyboard while it is open.
                if (document.body.classList.contains('inspect-open')) return;

                // Printable keys — easter egg sequence detection
                if (e.key.length === 1) {
                    keyBuffer.push(e.key.toLowerCase());
                    if (keyBuffer.length > KEY_BUFFER_MAX) keyBuffer.shift();
                    checkKeySequences();
                }
            });

            // ── Hash change listener (for #about:blank) ──
            function checkHash() {
                if (window.location.hash === '#about:blank') {
                    if (typeof window.__triggerAboutBlank === 'function') {
                        window.__triggerAboutBlank();
                    }
                    // Clear hash without triggering another event
                    history.replaceState(null, '', window.location.pathname + window.location.search);
                }
            }
            checkHash();
            window.addEventListener('hashchange', checkHash);
        }

        // ── Button fluid interaction ──
        // When hovering over UI buttons, inject density at that position
        // so the water reacts to the user navigating the interface.
        function injectAt(screenX, screenY, intensity) {
            if (!preEl || !cols) return;
            const ci = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
            const gi = clamp(Math.round(screenX / ci), 1, cols - 2);
            const gj = clamp(Math.round(screenY / parseFloat(preEl.style.fontSize)), 1, rows - 2);
            const brush = 2;
            for (let dj = -brush; dj <= brush; dj++) {
                for (let di = -brush; di <= brush; di++) {
                    const ni = clamp(gi + di, 1, cols - 2);
                    const nj = clamp(gj + dj, 1, rows - 2);
                    const d2 = di * di + dj * dj;
                    if (d2 <= brush * brush) {
                        const f = Math.exp(-d2 / (brush * 0.7));
                        const np = ni + nj * cols;
                        if (np < dens.length) {
                            dens[np] += intensity * f;
                            if (dens[np] > 1) dens[np] = 1;
                        }
                    }
                }
            }
        }

        var buttonBar = opts.buttonBar || null;
        if (buttonBar) {
            var btns = buttonBar.querySelectorAll('.btn');
            btns.forEach(function(btn) {
                btn.addEventListener('mouseenter', function(e) {
                    var rect = btn.getBoundingClientRect();
                    for (var i = 0; i < 3; i++) {
                        injectAt(rect.left + rect.width * (0.2 + i * 0.3), rect.top + rect.height * 0.5, 0.4);
                    }
                });
                btn.addEventListener('mousemove', function(e) {
                    injectAt(e.clientX, e.clientY, 0.15);
                });
            });
        }

        window.addEventListener('resize', onResize);

        if (opts.interactive !== false) {
            // The landing page follows the pointer anywhere in the window; a
            // plate in the guided tour only reacts while the cursor is on it.
            const target = opts.globalInput ? document : (opts.inputEl || preEl);
            target.addEventListener('mousemove', onMouseMove);
            target.addEventListener('mousedown', onMouseDown);
            target.addEventListener('mouseup', onMouseUp);
            target.addEventListener('mouseleave', onMouseLeave);
            target.addEventListener('touchmove', onTouchMove, { passive: true });
            target.addEventListener('touchstart', onTouchStart, { passive: true });
        }

        // Start animation
        lastTimestamp = performance.now();
        animFrameId = requestAnimationFrame(tick);
    }

    // ================================================================
    // Controls (used by the guided tour)
    // ================================================================
    const api = {
        /** Change what the surface draws; `text` toggles the greeting/menu. */
        setMode: function (mode, text) {
            if (mode) renderMode = mode;
            if (typeof text === 'boolean') textVisible = text;
            render();
        },
        setTextVisible: function (visible) {
            textVisible = !!visible;
            render();
        },
        /** Live dimensions of the grid, for figure captions. */
        grid: function () {
            return {
                cols: cols,
                rows: rows,
                charW: _charWidth,
                rowH: parseFloat(preEl.style.fontSize) || 14,
                cellPx: CFG.cellPx
            };
        },
        /** The brightness ramp, so callers describe the same characters. */
        ramp: function () { return CFG.chars; },
        /** The raw fields, for views that are not drawn as text. */
        field: function () {
            return { cols: cols, rows: rows, dens: dens, u: u, v: v };
        },
        /**
         * Draw the velocity field as streamlines: lines that a mote of ink
         * would follow. The ASCII renderer can only point four ways, and a
         * field of unheaded ticks reads as contour rings, so this traces the
         * flow instead and draws the path.
         */
        paintCurrent: function (canvas) {
            if (!canvas || !cols || !rows || !u || !v || !preEl) return;

            const dpr = Math.min(window.devicePixelRatio || 1, 2);
            const w = canvas.clientWidth || window.innerWidth;
            const h = canvas.clientHeight || window.innerHeight;
            const pw = Math.round(w * dpr);
            const ph = Math.round(h * dpr);
            if (canvas.width !== pw) canvas.width = pw;
            if (canvas.height !== ph) canvas.height = ph;

            const ctx = canvas.getContext('2d');
            ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
            ctx.clearRect(0, 0, w, h);

            const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
            const rowH = parseFloat(preEl.style.fontSize) || 14;

            let peak = 0;
            for (let p = 0; p < total; p++) {
                const s = u[p] * u[p] + v[p] * v[p];
                if (s > peak) peak = s;
            }
            if (peak <= 0) return;
            const invPeak = 1 / Math.sqrt(peak);

            // Bilinear sample of one velocity component, in grid coordinates.
            const sample = function (field, x, y) {
                const i = clamp(Math.floor(x), 1, cols - 2);
                const j = clamp(Math.floor(y), 1, rows - 2);
                const fx = clamp(x - i, 0, 1);
                const fy = clamp(y - j, 0, 1);
                const p = i + j * cols;
                const top = field[p] + (field[p + 1] - field[p]) * fx;
                const bot = field[p + cols] + (field[p + cols + 1] - field[p + cols]) * fx;
                return top + (bot - top) * fy;
            };

            // Five brightness bands, one stroke call each.
            const bands = [];
            for (let b = 0; b < 5; b++) bands.push(new Path2D());

            const STEP = 0.8;        // cells travelled per trace step
            const MAX_STEPS = 30;    // long enough to show the curve, short enough not to close into a ring
            const STOP = 0.06;       // relative speed that ends a line
            const SEED_X = 5;        // seeds per this many cells, with jitter
            const SEED_Y = 4;

            // A line is drawn from both ends of its seed, so each trace runs
            // one way until it dies out and the other until it does the same.
            for (let sj = SEED_Y * 0.5; sj < rows - 1; sj += SEED_Y) {
                for (let si = SEED_X * 0.5; si < cols - 1; si += SEED_X) {
                    // Stable jitter: same seeds every frame, so lines hold still.
                    const hx = Math.sin(si * 12.9898 + sj * 78.233) * 43758.5453;
                    const hy = Math.sin(sj * 39.346 + si * 11.135) * 24634.6345;
                    const seedX = si + (hx - Math.floor(hx) - 0.5) * SEED_X * 0.8;
                    const seedY = sj + (hy - Math.floor(hy) - 0.5) * SEED_Y * 0.8;
                    if (seedX < 1 || seedY < 1 || seedX > cols - 2 || seedY > rows - 2) continue;

                    const su0 = sample(u, seedX, seedY) * invPeak;
                    const sv0 = sample(v, seedX, seedY) * invPeak;
                    if (Math.sqrt(su0 * su0 + sv0 * sv0) < STOP * 2) continue;

                    for (let dir = -1; dir <= 1; dir += 2) {
                        let x = seedX;
                        let y = seedY;
                        for (let step = 0; step < MAX_STEPS; step++) {
                            let ux = sample(u, x, y) * invPeak;
                            let uy = sample(v, x, y) * invPeak;
                            let speed = Math.sqrt(ux * ux + uy * uy);
                            if (speed < STOP) break;
                            ux /= speed;
                            uy /= speed;

                            // Midpoint step keeps the curve smooth where it bends.
                            let mx2 = x + ux * STEP * 0.5 * dir;
                            let my2 = y + uy * STEP * 0.5 * dir;
                            let ux2 = sample(u, mx2, my2) * invPeak;
                            let uy2 = sample(v, mx2, my2) * invPeak;
                            const speed2 = Math.sqrt(ux2 * ux2 + uy2 * uy2);
                            if (speed2 >= STOP) {
                                ux = ux2 / speed2;
                                uy = uy2 / speed2;
                                speed = speed2;
                            }

                            const nx = x + ux * STEP * dir;
                            const ny = y + uy * STEP * dir;
                            if (!Number.isFinite(nx) || !Number.isFinite(ny)) break;
                            if (nx < 1 || nx > cols - 2 || ny < 1 || ny > rows - 2) break;
                            // Lines stop at the tour's own words rather than
                            // running through them.
                            if (masks.length && maskAt(nx, ny)) break;

                            // Grid units map to pixels per axis; the linear map
                            // preserves the shape of the traced curve. The
                            // stroke fades along its length, so it reads as a
                            // streak instead of a closed contour line.
                            const taper = 1 - (step + 1) / MAX_STEPS * 0.75;
                            const band = Math.min(4, Math.floor(speed * 5 * taper));
                            const path = bands[band];
                            path.moveTo(x * charW, y * rowH);
                            path.lineTo(nx * charW, ny * rowH);

                            x = nx;
                            y = ny;
                        }
                    }
                }
            }

            ctx.lineWidth = Math.max(0.75, Math.min(charW, rowH) * 0.1);
            ctx.lineCap = 'round';
            ctx.strokeStyle = '#fff';
            for (let b = 0; b < bands.length; b++) {
                ctx.globalAlpha = 0.08 + b * 0.065;
                ctx.stroke(bands[b]);
            }
            ctx.globalAlpha = 1;
        },
        /** The cell under a pointer, with the glyph it earns. */
        sampleAt: function (clientX, clientY) {
            if (!preEl || !cols) return null;
            const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
            const rowH = parseFloat(preEl.style.fontSize) || 14;
            const pt = localPoint(clientX, clientY);
            const i = Math.floor(pt.x / charW);
            const j = Math.floor(pt.y / rowH);
            if (i < 0 || j < 0 || i >= cols || j >= rows) return null;
            const p = j * cols + i;
            const d = dens[p];
            const ramp = CFG.chars;
            return {
                i: i,
                j: j,
                dens: d,
                char: d <= 0 ? ' ' : ramp[Math.min(ramp.length - 1, Math.round(Math.min(1, d * 1.15) * (ramp.length - 1)))],
                speed: Math.sqrt(u[p] * u[p] + v[p] * v[p])
            };
        },
        /** Empty the field — a fresh plate for the next chapter. */
        clear: function () {
            if (!dens) return;
            dens.fill(0); densPrev.fill(0);
            u.fill(0); v.fill(0); uPrev.fill(0); vPrev.fill(0);
            render();
        },
        /** Drop brightness at a grid coordinate. */
        droplet: function (ci, cj, amount, radius) {
            if (!cols) return;
            const r = radius == null ? 4 : radius;
            const a = amount == null ? 0.9 : amount;
            for (let dj = -r; dj <= r; dj++) {
                for (let di = -r; di <= r; di++) {
                    const dist = Math.sqrt(di * di + dj * dj);
                    if (dist > r) continue;
                    const falloff = Math.cos((dist / (r + 1)) * (Math.PI / 2));
                    const p = idx(clamp(Math.round(ci + di), 1, cols - 2), clamp(Math.round(cj + dj), 1, rows - 2));
                    dens[p] = Math.min(1, dens[p] + a * falloff);
                }
            }
            render();
        },
        /** Spin the field around a grid coordinate. */
        vortex: function (ci, cj, strength) {
            if (!cols) return;
            const radius = 7;
            const s = strength == null ? 0.4 : strength;
            for (let dj = -radius; dj <= radius; dj++) {
                for (let di = -radius; di <= radius; di++) {
                    const dist = Math.sqrt(di * di + dj * dj);
                    if (dist > radius || dist < 0.4) continue;
                    const falloff = 1 - dist / radius;
                    const p = idx(
                        clamp(Math.round(ci + di), 1, cols - 2),
                        clamp(Math.round(cj + dj), 1, rows - 2)
                    );
                    u[p] += (-dj / dist) * s * falloff;
                    v[p] += (di / dist) * s * falloff;
                    dens[p] = Math.min(1, dens[p] + 0.06 * falloff);
                }
            }
            render();
        },
        /** A recognisable wake for a plate that has just been opened. */
        stir: function () {
            if (!cols) return;
            api.vortex(cols * 0.36, rows * 0.42, 0.5);
            api.vortex(cols * 0.63, rows * 0.55, -0.45);
            api.vortex(cols * 0.48, rows * 0.3, 0.32);
            api.droplet(cols * 0.38, rows * 0.45, 0.9);
            api.droplet(cols * 0.61, rows * 0.5, 0.6);
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
        isRunning: function () { return animFrameId !== null; },
        /**
         * Keep rectangles of the grid free of ink, in grid cells:
         * [{ row, col, rows, cols, fade }]. The tour uses this so its own
         * words are not printed over moving water.
         */
        mask: function (rects) {
            masks = Array.isArray(rects) ? rects.filter(function (r) {
                return r && Number.isFinite(r.row) && Number.isFinite(r.col) &&
                    Number.isFinite(r.rows) && Number.isFinite(r.cols);
            }) : [];
            if (!dens) return;
            applyMasks();
            render();
        }
    };

    init();
    return api;
    }

    // ================================================================
    // The landing page's own instance
    // ================================================================
    function bootLanding() {
        const mount = document.getElementById('ascii-water');
        if (!mount) return;
        window.__pageWaterSim = createWaterSim({
            mount: mount,
            greeting: window.__SITE__ && window.__SITE__.greeting,
            buttons: window.__SITE__ && window.__SITE__.buttons,
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
