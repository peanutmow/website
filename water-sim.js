/**
 * water-sim.js — ASCII Fluid Simulation
 *
 * Simplified Navier-Stokes solver (Jos Stam method) rendered as
 * brightness-mapped ASCII characters on a pitch-black background.
 *
 * Mouse movement creates swirling vortexes; the intensity of the
 * fluid maps to ASCII character brightness. Click to reveal the
 * page navigation.
 */
(function () {
    'use strict';

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
        swirlStrength: 1.2,      // Perpendicular vortex strength
        densityPerFrame: 1.0,    // Density emitted per frame from mouse
        brushRadius: 2,          // Radius of density brush in grid cells
        dissipation: 0.992,      // Per-frame density decay (lower = faster fade)
        velDissipation: 0.997,   // Per-frame velocity decay
        ambientForce: 0.0003,    // Tiny random forces (keeps water alive)
        ambientDensity: 0.0005,  // Subtle background shimmer density
        chars: ' .,-~:=+*#%@',  // 11-level brightness ramp (dark → bright)
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

    // Mouse tracking (in grid coordinates)
    let mx = -1, my = -1;
    let pmx = -1, pmy = -1;
    let mdx = 0, mdy = 0;
    let mouseActive = false;
    let mouseDown = false;
    let lastMouseTime = 0;

    // DOM references
    let preEl;
    let animFrameId = null;
    let lastTimestamp = 0;

    // ── ASCII text overlay ──
    // Text rendered directly in the ASCII grid so it blends seamlessly with the water.
    let textOverlays = []; // Array of {row, col, char}

    // Typewriter effect for greeting
    const GREETING = "Hi, I'm Alice";
    let greetingRevealed = 0;
    let greetingComplete = false;

    function computeTextOverlays() {
        textOverlays = [];

        // Don't render UI text when a content page (like Socials) is open
        if (document.body.classList.contains('hide-ui-text')) return;

        const name   = GREETING;
        const buttons = ['Socials', 'Gallery', 'Blog'];
        const btnStr  = buttons.join('  ');
        const bio     = 'artist & developer';

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
        const btnRow = Math.floor(rows * 0.89);
        const btnCol = Math.floor((cols - btnStr.length) / 2);
        for (let i = 0; i < btnStr.length; i++) {
            textOverlays.push({ row: btnRow, col: btnCol + i, char: btnStr[i] });
        }

        // ---- Bio ----
        const bioRow = Math.floor(rows * 0.94);
        const bioCol = Math.floor((cols - bio.length) / 2);
        for (let i = 0; i < bio.length; i++) {
            textOverlays.push({ row: bioRow, col: bioCol + i, char: bio[i] });
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
                _charSpan.style.cssText = 'position:fixed;left:0;top:0;visibility:hidden;pointer-events:none;white-space:pre;font-family:"Share Tech Mono","Courier New",monospace;line-height:1;letter-spacing:0;';
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
        const newRows = Math.max(10, Math.ceil(height / CFG.cellPx));
        const fs = height / newRows;
        // Temporarily set font size to measure char width
        if (preEl) {
            preEl.style.fontSize = fs + 'px';
            preEl.style.lineHeight = fs + 'px';
            measureCharWidth();
        }
        const cw = _charWidth || fs * 0.54;
        const newCols = Math.max(20, Math.ceil(width / cw));

        // No-op if size unchanged
        if (newCols === cols && newRows === rows) return;

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

        computeTextOverlays();
    }

    /** Recalculate font-size so the ASCII grid exactly fills the viewport. */
    function updatePreFontSize() {
        if (!preEl) return;
        const h = window.innerHeight;
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
        for (let j = 1; j < rows - 1; j++) {
            const off = j * cols;
            for (let i = 1; i < cols - 1; i++) {
                const p = off + i;
                // Trace backwards in time
                let x = i - CFG.dt * cols * velU[p];
                let y = j - CFG.dt * rows * velV[p];
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

    /** Add velocity from mouse movement (called BEFORE advection). */
    function addMouseForces() {
        if (!mouseActive || mx < 0 || my < 0) return;

        // Only inject when the mouse is actually moving (velocity above threshold).
        const speed = Math.sqrt(mdx * mdx + mdy * mdy);
        if (speed < 0.01) return;

        const gi = clamp(Math.round(mx), 1, cols - 2);
        const gj = clamp(Math.round(my), 1, rows - 2);
        const p = idx(gi, gj);

        // Velocity impulse in the direction of mouse movement
        u[p] += mdx * CFG.forceStrength;
        v[p] += mdy * CFG.forceStrength;

        // Add perpendicular component for vortex generation
        const swirl = speed * CFG.swirlStrength;
        u[p] -= mdy * swirl;
        v[p] += mdx * swirl;

        // Note: mdx/mdy are NOT reset here — injectMouseDensity still needs them.
    }

    /** Inject density at mouse position (called AFTER advection so it stays at cursor). */
    function injectMouseDensity() {
        if (!mouseActive || mx < 0 || my < 0) return;

        const speed = Math.sqrt(mdx * mdx + mdy * mdy);
        if (speed < 0.01) return;

        const gi = clamp(Math.round(mx), 1, cols - 2);
        const gj = clamp(Math.round(my), 1, rows - 2);

        // Inject density at mouse position with a wider brush
        const brush = CFG.brushRadius;
        for (let dj = -brush; dj <= brush; dj++) {
            for (let di = -brush; di <= brush; di++) {
                const ni = clamp(gi + di, 1, cols - 2);
                const nj = clamp(gj + dj, 1, rows - 2);
                const dist = Math.sqrt(di * di + dj * dj);
                if (dist <= brush) {
                    const falloff = 1 - (dist / (brush + 1));
                    dens[idx(ni, nj)] += CFG.densityPerFrame * falloff;
                    if (dens[idx(ni, nj)] > 1) dens[idx(ni, nj)] = 1;
                }
            }
        }

        // Reset velocity delta so stationary mouse doesn't keep injecting next frame
        mdx = 0;
        mdy = 0;
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

    // ================================================================
    // Main simulation step
    // ================================================================
    function simulate() {
        // ── Step 1: Add velocity forces (BEFORE advection) ──
        addMouseForces();        // velocity from mouse
        addAmbientVelocity();    // random ambient velocity

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
        addAmbientDensity();

        // 11. Dissipate density & velocity
        for (let i = 0; i < total; i++) {
            dens[i] *= CFG.dissipation;
            u[i] *= CFG.velDissipation;
            v[i] *= CFG.velDissipation;
            // Keep a minimum threshold to avoid rendering noise
            if (dens[i] < 0.005) dens[i] = 0;
        }
    }

    // ================================================================
    // ASCII rendering
    // ================================================================

    /** Build the ASCII string from the density field and update the DOM. */
    function render() {
        if (!preEl) return;

        const ramp = CFG.chars;
        const rampLen = ramp.length - 1; // skip space at index 0
        // Precompute threshold so density 0 → space, density ~1 → last char
        const threshold = rampLen;

        // Build rows as array of strings (mutable for text overlay)
        const rowsArr = new Array(rows);
        for (let j = 0; j < rows; j++) {
            const off = j * cols;
            let line = '';
            for (let i = 0; i < cols; i++) {
                const d = dens[off + i];
                if (d <= 0) {
                    line += ' ';
                } else {
                    const idx = Math.min(rampLen, Math.round(d * threshold));
                    line += ramp[idx];
                }
            }
            rowsArr[j] = line;
        }

        // ── Overlay text directly into the ASCII grid ──
        // Auto-hide text when scrolled past the first viewport (content panel visible)
        if (document.getElementById('socials-inline').classList.contains('visible')) {
            if (window.scrollY > window.innerHeight * 0.4) {
                document.body.classList.add('hide-ui-text');
            } else {
                document.body.classList.remove('hide-ui-text');
            }
        }

        if (!document.body.classList.contains('hide-ui-text')) {
            // Recompute if previously cleared by hide-ui-text
            if (textOverlays.length === 0) computeTextOverlays();
            for (let t = 0; t < textOverlays.length; t++) {
                const { row, col, char } = textOverlays[t];
                if (row >= 0 && row < rows && col >= 0 && col < cols) {
                    const r = rowsArr[row];
                    rowsArr[row] = r.substring(0, col) + char + r.substring(col + 1);
                }
            }
        }

        preEl.textContent = rowsArr.join('\n');
    }

    // ================================================================
    // Animation loop
    // ================================================================
    function tick(timestamp) {
        if (!timestamp) timestamp = performance.now();

        // Skip frame if we're falling behind (maintain ~30fps minimum)
        if (timestamp - lastTimestamp < 1000 / CFG.fadeFps) {
            animFrameId = requestAnimationFrame(tick);
            return;
        }
        lastTimestamp = timestamp;

        simulate();
        render();
        animFrameId = requestAnimationFrame(tick);
    }

    // ================================================================
    // Mouse / touch event handlers
    // ================================================================
    function onMouseMove(e) {
        // Map screen pixel coords → grid coords using actual measured character width.
        // The pre element is fixed at (0,0) spanning the viewport, and characters are
        // left-aligned, so clientX/Y directly correspond to character positions.
        const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
        const fSize = parseFloat(preEl.style.fontSize) || 14;
        const gx = e.clientX / charW;
        const gy = e.clientY / fSize;

        if (mx >= 0 && my >= 0) {
            mdx = gx - mx;
            mdy = gy - my;
        } else {
            mdx = 0;
            mdy = 0;
        }

        pmx = mx;
        pmy = my;
        mx = gx;
        my = gy;
        mouseActive = true;
        lastMouseTime = performance.now();
    }

    function onTouchMove(e) {
        e.preventDefault();
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
        mx = touch.clientX / charW;
        my = touch.clientY / fSize;
        mdx = 0;
        mdy = 0;
        mouseActive = true;
    }

    function onMouseLeave() {
        mouseActive = false;
        mx = -1;
        my = -1;
    }

    function onMouseDown(e) {
        mouseDown = true;
        if (mx < 0) {
            const charW = _charWidth || parseFloat(preEl.style.fontSize) * 0.54 || 8;
            const fSize = parseFloat(preEl.style.fontSize) || 14;
            mx = e.clientX / charW;
            my = e.clientY / fSize;
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
        const w = window.innerWidth;
        const h = window.innerHeight;
        // Set pre dimensions to exactly match innerWidth/innerHeight
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
        preEl = document.getElementById('ascii-water');
        if (!preEl) {
            console.error('water-sim: #ascii-water element not found');
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
        startTypewriter();

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

        var buttonBar = document.getElementById('button-bar');
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

        // Event listeners
        window.addEventListener('resize', onResize);
        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mousedown', onMouseDown);
        document.addEventListener('mouseup', onMouseUp);
        document.addEventListener('mouseleave', onMouseLeave);
        document.addEventListener('touchmove', onTouchMove, { passive: false });
        document.addEventListener('touchstart', onTouchStart, { passive: false });
        // Start animation
        lastTimestamp = performance.now();
        animFrameId = requestAnimationFrame(tick);
    }

    // Boot when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
