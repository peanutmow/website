/**
 * inspect.js — tutorial mode for the landing page.
 *
 * The page stays full-bleed and keeps running; the layers are the same fluid
 * solver rendered through different eyes (see water-sim.js). The only chrome
 * is a line of layer names and a short note — the page does the rest.
 */
(function () {
    'use strict';

    function el(tag, className, text) {
        const node = document.createElement(tag);
        if (className) node.className = className;
        if (text != null) node.textContent = text;
        return node;
    }

    function seedField(sim) {
        const g = sim.grid();
        [[0.22, 0.3], [0.46, 0.56], [0.7, 0.34], [0.34, 0.72], [0.76, 0.68]].forEach(function (spot, i) {
            sim.droplet(g.cols * spot[0], g.rows * spot[1], 0.95, 7);
            sim.vortex(g.cols * spot[0], g.rows * spot[1], i % 2 ? -0.35 : 0.35);
        });
    }

    function seedCurrent(sim) {
        const g = sim.grid();
        [
            [0.18, 0.24, 0.45], [0.48, 0.36, -0.5], [0.76, 0.3, 0.4],
            [0.28, 0.68, -0.42], [0.62, 0.7, 0.5], [0.86, 0.56, -0.35]
        ].forEach(function (v) {
            sim.vortex(g.cols * v[0], g.rows * v[1], v[2]);
        });
    }

    function seedFrame(sim) {
        seedField(sim);
        seedCurrent(sim);
    }

    // The ramp, read from the running simulation so the tour can never drift
    // from the thing it is describing.
    const FALLBACK_RAMP = ' .-:~,^;*=|>+\\/?&$%#@';

    function rampChars() {
        return (sim && sim.ramp && sim.ramp()) || FALLBACK_RAMP;
    }

    function rampLabel() {
        return 'space ' + rampChars().slice(1).split('').join(' ');
    }

    const STEPS = [
        {
            nav: 'overview',
            label: 'overview',
            mode: 'ascii',
            text: true,
            copy: 'Every character on this screen is drawn by the page as it runs. This tour goes through the parts it is built from. You can still move the pointer the whole time.',
            seed: seedFrame
        },
        {
            nav: 'field',
            label: 'the field',
            mode: 'text',
            text: false,
            canvas: 'field',
            copy: 'Start with the water itself. Each cell holds a shade between empty and full. It spreads out, swirls around, and fades a little every frame, so the dark comes back on its own.',
            seed: seedField
        },
        {
            nav: 'lattice',
            label: 'the lattice',
            mode: 'grid',
            text: false,
            copy: 'Everything here sits on a grid of characters. Rows are about fourteen pixels tall and the columns fit as many characters as the window allows. The water and the words use the same grid.',
            callout: 'cell'
        },
        {
            nav: 'current',
            label: 'the current',
            mode: 'text',
            text: false,
            canvas: 'current',
            copy: 'Each cell also knows which way the water is moving and how fast. The page works that out every frame, then carries the shade along with it, which is what makes the ripples travel.',
            seed: seedCurrent
        },
        {
            nav: 'alphabet',
            label: 'the alphabet',
            mode: 'text',
            text: false,
            copy: 'Brighter cells get heavier characters. That is the whole trick. There is no image underneath, just letters. The bunny is only triangles, and its shading comes from the same set of characters.',
            bunny: true
        },
        {
            nav: 'type',
            label: 'the type',
            mode: 'text',
            text: true,
            copy: 'The name, menu and bio are written into the same grid after the water is drawn, near the bottom. The menu items are real links sitting on the letters they name.',
            callout: 'type'
        },
        {
            nav: 'frame',
            label: 'the frame',
            mode: 'ascii',
            text: true,
            copy: 'That is all of it. One grid, one set of characters, one bit of text. Move the pointer around. Close this and make a mess.',
            cta: 'return to the page',
            seed: seedFrame
        }
    ];

    let root = null;
    let layerBtns = [];
    let prevBtn = null;
    let nextBtn = null;
    let closeBtn = null;
    let ctaBtn = null;
    let noteEl = null;
    let chromeEl = null;
    let layerCanvas = null;
    let hoverNav = -1;
    let hoverClose = false;
    let hoverCta = false;
    // How far the bunny on the alphabet layer has turned, the frame loop that
    // keeps turning it, the raster we last drew, and whether bunny.js is in.
    let bunnyAngle = 0;
    let spinFrame = null;
    let spinStart = 0;
    let spinBucket = -1;
    let bunnyFrame = null;
    let bunnyState = 'idle';
    let lastMaskKey = '';

    let canvasFrame = null;
    let canvasMode = null;
    let fieldImage = null;
    let fieldScratch = null;

    let sim = null;
    let step = 0;
    let open = false;
    let lastFocus = null;
    let resizeTimer = null;
    let closeTimer = null;
    let breatheTimer = null;

    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

    // A layer you are reading should not go still. Small, slow disturbances
    // keep the subject alive without turning the page into a toy.
    function startBreathing() {
        stopBreathing();
        breatheTimer = window.setInterval(function () {
            if (!open || document.hidden || !sim) return;
            const def = STEPS[step];
            if (def.mode === 'grid') return;
            // The type layer has nothing fluid to disturb.
            if (def.mode === 'text' && !def.canvas) return;

            const g = sim.grid();
            if (!g.cols || !g.rows) return;

            if (def.canvas === 'current') {
                const x = g.cols * (0.22 + Math.random() * 0.56);
                const y = g.rows * (0.24 + Math.random() * 0.5);
                sim.vortex(x, y, (Math.random() < 0.5 ? -1 : 1) * 0.24);
            } else {
                for (let n = 0; n < 2; n++) {
                    const x = g.cols * (0.2 + Math.random() * 0.6);
                    const y = g.rows * (0.22 + Math.random() * 0.52);
                    sim.droplet(x, y, 0.55, 5);
                    sim.vortex(x, y, 0.12);
                }
            }
        }, 1700);
    }

    function stopBreathing() {
        if (breatheTimer !== null) {
            clearInterval(breatheTimer);
            breatheTimer = null;
        }
    }

    // The bunny turns while its layer is open, so the render runs on a frame
    // loop of its own and stops the moment the layer changes. Rasterising the
    // mesh takes about a millisecond, so it is redrawn every few degrees rather
    // than every frame — and a hover rebuild lands on the same cached raster.
    const SPIN_RATE = 0.0007; // radians per millisecond, about nine seconds a turn
    const SPIN_STEP = 0.03; // radians between rasters

    function spinAngle() {
        return Math.round(bunnyAngle / SPIN_STEP) * SPIN_STEP;
    }

    function startSpin() {
        stopSpin();
        if (bunnyState !== 'ready') return;
        if (reduceMotion.matches) {
            bunnyAngle = 0.4;
            buildChrome();
            return;
        }
        spinStart = performance.now() - bunnyAngle / SPIN_RATE;
        const turn = function (now) {
            bunnyAngle = (now - spinStart) * SPIN_RATE;
            const bucket = Math.round(bunnyAngle / SPIN_STEP);
            if (bucket !== spinBucket) {
                spinBucket = bucket;
                buildChrome();
            }
            spinFrame = requestAnimationFrame(turn);
        };
        spinFrame = requestAnimationFrame(turn);
    }

    function stopSpin() {
        if (spinFrame !== null) {
            cancelAnimationFrame(spinFrame);
            spinFrame = null;
        }
        spinBucket = -1;
    }

    // The mesh now lives in the wasm binary, so only the loader is fetched,
    // and only when a layer first needs it.
    function loadBunny() {
        if (bunnyState !== 'idle') return;
        bunnyState = 'loading';
        const script = el('script');
        script.src = '/static/bunny.js?v=2';
        script.onload = function () {
            const ready = window.bunnyReady || Promise.resolve();
            ready.then(function () {
                bunnyState = 'ready';
                if (STEPS[step].bunny) startSpin();
            }).catch(function () { bunnyState = 'failed'; });
        };
        script.onerror = function () { bunnyState = 'failed'; };
        document.head.appendChild(script);
    }

    // -- the note --------------------------------------------------
    //
    // The note is announced to screen readers; what you see is drawn as
    // characters in the grid, along with everything else the tour says.

    function renderNote(def) {
        noteEl.textContent = def.label + '. ' + def.copy;
        if (ctaBtn) ctaBtn.hidden = !def.cta;
    }

    function updateLayers(i) {
        layerBtns.forEach(function (btn, n) {
            if (n === i) btn.setAttribute('aria-current', 'true');
            else btn.removeAttribute('aria-current');
        });
        prevBtn.disabled = i <= 0;
        nextBtn.disabled = i >= STEPS.length - 1;
    }

    // -- the chrome, drawn as characters ---------------------------
    //
    // Every word the tour adds is written into the same character grid the
    // water uses, so it belongs to the screen instead of floating over it.
    // The buttons stay in the DOM but are transparent: they are the click
    // targets sitting exactly on the characters that name them.

    /** Curly punctuation is wider than one cell in some fonts; keep it plain. */
    function plain(text) {
        return String(text)
            .replace(/[\u2014\u2013]/g, '-')
            .replace(/[\u2018\u2019]/g, "'")
            .replace(/[\u201c\u201d]/g, '"');
    }

    function wrapText(text, width) {
        const words = plain(text).split(/\s+/);
        const lines = [];
        let line = '';
        for (let i = 0; i < words.length; i++) {
            const word = words[i];
            if (!word) continue;
            if (!line) line = word;
            else if (line.length + 1 + word.length <= width) line += ' ' + word;
            else { lines.push(line); line = word; }
        }
        if (line) lines.push(line);
        return lines;
    }

    function blankGrid(cols, rows) {
        const lines = [];
        for (let j = 0; j < rows; j++) lines.push(new Array(cols).fill(' '));
        return lines;
    }

    function put(grid, row, col, text) {
        if (row < 0 || row >= grid.length || !text) return;
        const line = grid[row];
        for (let i = 0; i < text.length; i++) {
            const c = col + i;
            if (c < 0 || c >= line.length) continue;
            line[c] = text[i];
        }
    }

    function putRight(grid, row, endCol, text) {
        put(grid, row, endCol - text.length, text);
    }

    /**
     * The alphabet layer: a bunny, turning, sampled into the ramp.
     *
     * The mesh is rasterised straight into the character grid the water uses,
     * at whatever size the screen has room for, so the glyph is still the pixel
     * and there is no image underneath. The lights stay put and the model
     * turns, which is what carries the shading across the surface. The luma is
     * inverted before it picks a character: the lit face is thin, the shadow
     * heavy.
     */
    function bunnyRaster(wCells, hCells, angle) {
        const key = wCells + 'x' + hCells + '@' + angle.toFixed(3);
        if (bunnyFrame && bunnyFrame.key === key) return bunnyFrame.lines;
        if (!window.AsciiBunny) return null;
        const lines = window.AsciiBunny.render({
            cols: wCells,
            rows: hCells,
            angleX: 0.08,
            angleY: angle,
            ramp: rampChars(),
            invert: true
        });
        bunnyFrame = { key: key, lines: lines };
        return lines;
    }

    function drawBunny(grid, g, clearTop) {
        const cols = grid[0].length;
        const rows = grid.length;

        // Cells are taller than they are wide, so a square model needs fewer
        // rows than columns or it comes out stretched.
        const aspect = g.rowH / g.charW;
        const topRow = Math.max(0, Math.min(rows - 10, Math.ceil((clearTop || 0) / g.rowH) + 1));

        let hCells = Math.floor(
            Math.min(window.innerHeight * 0.74, window.innerHeight - (clearTop || 0)) / g.rowH
        );
        hCells = Math.max(8, Math.min(rows - topRow - 2, hCells));
        let wCells = Math.max(8, Math.round(hCells * aspect));
        if (wCells > cols - 6) {
            wCells = Math.max(8, cols - 6);
            hCells = Math.max(8, Math.min(rows - topRow - 2, Math.round(wCells / aspect)));
        }

        const lines = bunnyRaster(wCells, hCells, spinAngle());
        if (!lines) return;

        // Centred in what the note leaves free, and never off the bottom.
        const row0 = Math.max(
            topRow,
            Math.min(rows - hCells - 1, Math.round((topRow + rows) / 2 - hCells / 2))
        );
        const col0 = Math.round((cols - wCells) / 2);

        for (let j = 0; j < hCells; j++) {
            const row = row0 + j;
            if (row < 0 || row >= rows) continue;
            const line = lines[j] || '';
            for (let i = 0; i < wCells; i++) {
                const ch = line[i];
                if (!ch || ch === ' ') continue;
                const col = col0 + i;
                if (col < 0 || col >= cols) continue;
                if (grid[row][col] !== ' ') continue;
                grid[row][col] = ch;
            }
        }
    }

    function buildChrome() {
        if (!chromeEl || !sim) return;
        const g = sim.grid();
        const cols = g.cols;
        const rows = g.rows;
        if (!cols || !rows) return;

        chromeEl.style.fontSize = g.rowH + 'px';
        chromeEl.style.lineHeight = g.rowH + 'px';

        const grid = blankGrid(cols, rows);
        const hits = [];
        const masks = [];
        const def = STEPS[step];
        const firstCol = 2;

        // Lay the interface out first, so the ramp lens can be drawn under it
        // and the words are then written on top of a clean band.
        const navItems = [];
        const navStartRow = 2;
        let navRow = navStartRow;
        let navCol = firstCol + 2;
        STEPS.forEach(function (s, i) {
            const text = i === step ? '[' + s.nav + ']' : (i === hoverNav ? s.nav.toUpperCase() : s.nav);
            if (navCol + text.length > cols - 3 && navCol > firstCol + 2) {
                navRow++;
                navCol = firstCol;
            }
            navItems.push({ text: text, row: navRow, col: navCol, index: i });
            navCol += text.length + 3;
        });
        const navEndCol = navCol;

        const closeText = hoverClose ? 'CLOSE  ESC' : 'close  esc';
        const closeCol = cols - 2 - closeText.length;

        // The note's height decides how much room the bunny has.
        const noteTop = navRow + 2;
        const width = Math.max(24, Math.min(46, cols - 6));
        const noteLines = [def.label.toUpperCase(), ''];
        wrapText(def.copy, width).forEach(function (line) { noteLines.push(line); });

        if (def.bunny) {
            noteLines.push('');
            wrapText(rampLabel(), width).forEach(function (line) { noteLines.push(line); });
        }

        let ctaRow = -1;
        let ctaLabel = '';
        if (def.cta) {
            ctaLabel = '[' + (hoverCta ? def.cta.toUpperCase() : def.cta) + ']';
            noteLines.push('');
            ctaRow = noteTop + noteLines.length;
            noteLines.push(ctaLabel);
        }

        // The bunny sits behind everything the tour says.
        if (def.bunny) drawBunny(grid, g, (noteTop + noteLines.length) * g.rowH);

        // Clear a clean band for the layer line and the close label.
        for (let j = 1; j <= navRow; j++) {
            for (let i = 0; i < cols; i++) {
                if (j === 1 && i < closeCol - 2) continue;
                grid[j][i] = ' ';
            }
        }

        put(grid, navStartRow, firstCol, '<');
        hits.push({ kind: 'prev', row: navStartRow, col: firstCol, len: 1 });
        navItems.forEach(function (item) {
            put(grid, item.row, item.col, item.text);
            hits.push({ kind: 'layer', index: item.index, row: item.row, col: item.col, len: item.text.length });
        });
        put(grid, navRow, navCol, '>');
        hits.push({ kind: 'next', row: navRow, col: navCol, len: 1 });
        masks.push({ row: 2, col: 0, rows: navRow - 1, cols: Math.max(navEndCol, navCol) + 3, fade: 2 });

        putRight(grid, 1, cols - 2, closeText);
        hits.push({ kind: 'close', row: 1, col: closeCol, len: closeText.length });
        masks.push({ row: 1, col: closeCol - 1, rows: 1, cols: closeText.length + 3, fade: 2 });

        for (let j = noteTop - 1; j < noteTop + noteLines.length + 1; j++) {
            if (j < 0 || j >= rows) continue;
            for (let i = 0; i < Math.min(cols, width + 5); i++) grid[j][i] = ' ';
        }

        noteLines.forEach(function (line, n) {
            if (line) put(grid, noteTop + n, firstCol, line);
        });

        if (def.cta) hits.push({ kind: 'cta', row: ctaRow, col: firstCol, len: ctaLabel.length });

        masks.push({ row: noteTop - 1, col: 0, rows: noteLines.length + 2, cols: width + 5, fade: 2 });

        // the annotations
        if (def.callout === 'cell') {
            const cCol = Math.floor(cols * 0.52);
            const cRow = Math.floor(rows * 0.42);
            put(grid, cRow, cCol, '+---- one cell');
            masks.push({ row: cRow - 1, col: cCol - 1, rows: 3, cols: 16, fade: 2 });
        } else if (def.callout === 'type') {
            const layout = window.__asciiLayout;
            if (layout && layout.rows) {
                const marks = [
                    { label: 'greeting', row: layout.nameRow, col: layout.nameCol, len: (layout.greeting || '').length },
                    { label: 'menu', row: layout.btnRow, col: layout.btnCol, len: (layout.btnStr || '').length },
                    { label: 'bio', row: layout.bioRow, col: layout.bioCol, len: (layout.bio || '').length }
                ];
                marks.forEach(function (m) {
                    if (!(m.len > 0)) return;
                    put(grid, m.row + 1, m.col, new Array(m.len + 1).join('-'));
                    put(grid, m.row + 1, m.col + m.len + 2, m.label);
                    masks.push({ row: m.row + 1, col: m.col - 1, rows: 1, cols: m.len + 3 + m.label.length, fade: 2 });
                });
            }
        }

        chromeEl.textContent = grid.map(function (line) { return line.join(''); }).join('\n');

        // The masks only change when the layout does; the bunny redraws as it
        // turns, and re-running the solver mask each time is wasted work.
        const maskKey = JSON.stringify(masks);
        if (maskKey !== lastMaskKey) {
            lastMaskKey = maskKey;
            sim.mask(masks);
        }

        // put the invisible hit targets on their characters
        hits.forEach(function (hit) {
            let node = null;
            if (hit.kind === 'prev') node = prevBtn;
            else if (hit.kind === 'next') node = nextBtn;
            else if (hit.kind === 'close') node = closeBtn;
            else if (hit.kind === 'cta') node = ctaBtn;
            else node = layerBtns[hit.index];
            if (!node) return;
            node.style.position = 'absolute';
            node.style.left = (hit.col * g.charW) + 'px';
            node.style.top = (hit.row * g.rowH) + 'px';
            node.style.width = (Math.max(1, hit.len) * g.charW) + 'px';
            node.style.height = g.rowH + 'px';
        });
    }

    // ── layers ───────────────────────────────────────────────────

    function crossfadeTo(def) {
        const pre = document.getElementById('ascii-water');
        if (!reduceMotion.matches && pre && pre.textContent) {
            const ghost = el('pre', 'ix-ghost');
            ghost.style.cssText = pre.style.cssText;
            ghost.textContent = pre.textContent;
            document.body.appendChild(ghost);
            requestAnimationFrame(function () {
                requestAnimationFrame(function () { ghost.classList.add('is-out'); });
            });
            window.setTimeout(function () { ghost.remove(); }, 600);
        }
        sim.setMode(def.mode, def.text);
    }

    // ── canvas layers: the density image, the vector current ────

    function sizeLayerCanvas() {
        if (!layerCanvas) return;
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        const w = Math.round(window.innerWidth * dpr);
        const h = Math.round(window.innerHeight * dpr);
        if (layerCanvas.width !== w) layerCanvas.width = w;
        if (layerCanvas.height !== h) layerCanvas.height = h;
    }

    function drawDensity() {
        const f = sim.field();
        if (!f || !f.cols || !f.rows) return;

        if (!fieldScratch || fieldScratch.width !== f.cols || fieldScratch.height !== f.rows) {
            fieldScratch = document.createElement('canvas');
            fieldScratch.width = f.cols;
            fieldScratch.height = f.rows;
            fieldImage = fieldScratch.getContext('2d').createImageData(f.cols, f.rows);
        }

        const data = fieldImage.data;
        const dens = f.dens;
        for (let p = 0, o = 0; p < dens.length; p++, o += 4) {
            const d = dens[p];
            data[o] = 255;
            data[o + 1] = 255;
            data[o + 2] = 255;
            data[o + 3] = d <= 0 ? 0 : Math.min(255, Math.pow(Math.min(1, d), 0.95) * 255);
        }

        const scratch = fieldScratch.getContext('2d');
        scratch.putImageData(fieldImage, 0, 0);

        const ctx = layerCanvas.getContext('2d');
        ctx.clearRect(0, 0, layerCanvas.width, layerCanvas.height);
        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(
            fieldScratch,
            0, 0, f.cols, f.rows,
            0, 0, layerCanvas.width, layerCanvas.height
        );
    }

    function drawCanvas() {
        canvasFrame = null;
        if (!open || !sim || !layerCanvas || !canvasMode) return;

        sizeLayerCanvas();
        if (canvasMode === 'current') sim.paintCurrent(layerCanvas);
        else drawDensity();

        canvasFrame = requestAnimationFrame(drawCanvas);
    }

    function startCanvas(mode) {
        if (!layerCanvas) return;
        canvasMode = mode;
        layerCanvas.dataset.mode = mode;
        sizeLayerCanvas();
        layerCanvas.classList.add('is-on');
        if (canvasFrame === null) canvasFrame = requestAnimationFrame(drawCanvas);
    }

    function stopCanvas() {
        if (canvasFrame !== null) {
            cancelAnimationFrame(canvasFrame);
            canvasFrame = null;
        }
        canvasMode = null;
        if (layerCanvas) layerCanvas.classList.remove('is-on');
    }

    function applyStep(i, instant) {
        const def = STEPS[i];

        if (instant) sim.setMode(def.mode, def.text);
        else crossfadeTo(def);

        if (def.seed) def.seed(sim);
        if (def.canvas) startCanvas(def.canvas);
        else stopCanvas();

        renderNote(def);
        updateLayers(i);
        buildChrome();
        if (def.bunny) {
            loadBunny();
            startSpin();
        } else {
            stopSpin();
        }
    }

    function goTo(i) {
        if (!open) return;
        const next = Math.max(0, Math.min(STEPS.length - 1, i));
        if (next === step) return;
        step = next;
        applyStep(step, false);
    }

    // ── keys ─────────────────────────────────────────────────────

    function focusables() {
        return Array.prototype.slice.call(root.querySelectorAll('button:not(:disabled)'));
    }

    function onKeydown(e) {
        if (!open) return;
        if (e.ctrlKey || e.metaKey || e.altKey) return;

        if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            closeInspect();
            return;
        }
        if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === 'PageDown') {
            e.preventDefault();
            e.stopPropagation();
            goTo(step + 1);
            return;
        }
        if (e.key === 'ArrowLeft' || e.key === 'ArrowUp' || e.key === 'PageUp') {
            e.preventDefault();
            e.stopPropagation();
            goTo(step - 1);
            return;
        }
        if (e.key === 'Home') {
            e.preventDefault();
            e.stopPropagation();
            goTo(0);
            return;
        }
        if (e.key === 'End') {
            e.preventDefault();
            e.stopPropagation();
            goTo(STEPS.length - 1);
            return;
        }
        if (e.key >= '1' && e.key <= '7') {
            e.preventDefault();
            e.stopPropagation();
            goTo(Number(e.key) - 1);
            return;
        }
        if (e.key === 'Tab') {
            const list = focusables();
            if (!list.length) return;
            const first = list[0];
            const last = list[list.length - 1];
            if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first.focus();
            }
        }
    }

    // ── build / open / close ─────────────────────────────────────

    function build() {
        root = el('div', 'inspect');
        root.hidden = true;
        root.setAttribute('aria-hidden', 'true');
        root.setAttribute('role', 'dialog');
        root.setAttribute('aria-modal', 'true');
        root.setAttribute('aria-label', 'How this page works');
        root.tabIndex = -1;

        prevBtn = el('button', 'ix-arrow', '<');
        prevBtn.type = 'button';
        prevBtn.setAttribute('aria-label', 'Previous layer');
        prevBtn.addEventListener('click', function () { goTo(step - 1); });

        nextBtn = el('button', 'ix-arrow', '>');
        nextBtn.type = 'button';
        nextBtn.setAttribute('aria-label', 'Next layer');
        nextBtn.addEventListener('click', function () { goTo(step + 1); });

        layerBtns = STEPS.map(function (def, i) {
            const btn = el('button', 'ix-layer', def.nav);
            btn.type = 'button';
            btn.addEventListener('click', function () { goTo(i); });
            const on = function () { hoverNav = i; buildChrome(); };
            const off = function () { if (hoverNav === i) { hoverNav = -1; buildChrome(); } };
            btn.addEventListener('mouseenter', on);
            btn.addEventListener('focus', on);
            btn.addEventListener('mouseleave', off);
            btn.addEventListener('blur', off);
            return btn;
        });

        closeBtn = el('button', 'ix-close', 'close esc');
        closeBtn.type = 'button';
        closeBtn.addEventListener('click', closeInspect);
        closeBtn.addEventListener('mouseenter', function () { hoverClose = true; buildChrome(); });
        closeBtn.addEventListener('focus', function () { hoverClose = true; buildChrome(); });
        closeBtn.addEventListener('mouseleave', function () { hoverClose = false; buildChrome(); });
        closeBtn.addEventListener('blur', function () { hoverClose = false; buildChrome(); });

        ctaBtn = el('button', 'ix-cta');
        ctaBtn.type = 'button';
        ctaBtn.hidden = true;
        ctaBtn.addEventListener('click', closeInspect);
        ctaBtn.addEventListener('mouseenter', function () { hoverCta = true; buildChrome(); });
        ctaBtn.addEventListener('focus', function () { hoverCta = true; buildChrome(); });
        ctaBtn.addEventListener('mouseleave', function () { hoverCta = false; buildChrome(); });
        ctaBtn.addEventListener('blur', function () { hoverCta = false; buildChrome(); });

        noteEl = el('aside', 'ix-note');
        noteEl.setAttribute('aria-live', 'polite');

        chromeEl = el('pre', 'ix-ascii');
        chromeEl.setAttribute('aria-hidden', 'true');

        layerCanvas = el('canvas', 'ix-canvas');
        layerCanvas.setAttribute('aria-hidden', 'true');

        root.append.apply(root, [layerCanvas, chromeEl, prevBtn, nextBtn, closeBtn, ctaBtn, noteEl].concat(layerBtns));
        document.body.appendChild(root);
    }

    function openInspect() {
        sim = window.__pageWaterSim;
        if (!sim) return;
        if (!root) build();
        if (open) return;

        open = true;
        step = 0;
        lastFocus = document.activeElement;

        if (closeTimer !== null) {
            clearTimeout(closeTimer);
            closeTimer = null;
        }

        document.body.classList.add('inspect-open');
        root.hidden = false;
        root.setAttribute('aria-hidden', 'false');
        void root.offsetWidth;
        root.classList.add('is-open');

        sim.resume();
        // The bunny's mesh is fetched in the background from here, so the
        // alphabet layer is ready by the time the layers before it are read.
        loadBunny();
        applyStep(0, true);
        root.focus();
        startBreathing();

        document.addEventListener('keydown', onKeydown, true);

        requestAnimationFrame(function () {
            if (open) buildChrome();
        });
    }

    function closeInspect() {
        if (!open) return;
        open = false;

        root.classList.remove('is-open');
        root.setAttribute('aria-hidden', 'true');
        document.removeEventListener('keydown', onKeydown, true);
        stopSpin();
        document.body.classList.remove('inspect-open');
        stopBreathing();
        stopCanvas();

        document.querySelectorAll('.ix-ghost').forEach(function (ghost) { ghost.remove(); });

        if (sim) {
            sim.setMode('ascii', true);
            sim.mask([]);
            sim.stir();
        }

        closeTimer = window.setTimeout(function () {
            closeTimer = null;
            root.hidden = true;
        }, 260);

        if (lastFocus && lastFocus.focus) lastFocus.focus();
    }

    window.addEventListener('resize', function () {
        if (!open || !sim) return;
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(function () {
            // A resize reallocates the field; put the demonstration back.
            const def = STEPS[step];
            if (def.seed) def.seed(sim);
            buildChrome();
        }, 140);
    });

    window.openInspect = openInspect;
    window.closeInspect = closeInspect;
})();
