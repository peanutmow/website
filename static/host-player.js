(function () {
    'use strict';

    const bar = document.getElementById('host-player');
    const frame = document.getElementById('music-frame');
    if (!bar || !frame) return;

    const titleEl = document.getElementById('hp-title');
    const artistEl = document.getElementById('hp-artist');
    const clockEl = document.getElementById('hp-clock');
    const seekEl = document.getElementById('hp-seek');
    const toggleBtn = document.getElementById('hp-toggle');
    const prevBtn = document.getElementById('hp-prev');
    const nextBtn = document.getElementById('hp-next');
    const expandBtn = document.getElementById('hp-expand');
    const playIcon = document.getElementById('hp-play-icon');
    const pauseIcon = document.getElementById('hp-pause-icon');
    const ascii = document.getElementById('ascii-spectrum');
    const contentPanel = document.getElementById('content-panel');
    const reducedMotion = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    let page = null;
    let dragging = false;
    let lastSpectrum = 0;
    let clearTimer = 0;

    function control() {
        try {
            const win = frame.contentWindow;
            return win && win.__xmmsUI ? win.__xmmsUI : null;
        } catch (error) {
            return null;
        }
    }

    function readState() {
        const ui = control();
        if (!ui) return null;
        try {
            return ui.state();
        } catch (error) {
            return null;
        }
    }

    function pad(value) {
        return String(value).padStart(2, '0');
    }

    function clock(seconds) {
        if (typeof seconds !== 'number' || !isFinite(seconds) || seconds < 0) return '0:00';
        const total = Math.floor(seconds);
        return Math.floor(total / 60) + ':' + pad(total % 60);
    }

    function setFill(percent) {
        seekEl.style.setProperty('--p', Math.max(0, Math.min(100, percent)) + '%');
    }

    function update() {
        const state = readState();
        const show = !!state && state.loaded && page !== 'music';
        bar.hidden = !show;
        if (!state) return;

        titleEl.textContent = state.title || 'no track';
        artistEl.textContent = state.artist || (state.loaded ? 'unknown artist' : '');
        clockEl.textContent = clock(state.time) + ' / ' + clock(state.duration);
        bar.classList.toggle('is-playing', !!state.playing);
        playIcon.hidden = !!state.playing;
        pauseIcon.hidden = !state.playing;
        toggleBtn.setAttribute('aria-label', state.playing ? 'Pause' : 'Play');
        toggleBtn.setAttribute('title', state.playing ? 'Pause' : 'Play');

        if (!dragging) {
            const value = state.duration > 0 ? Math.round((state.time / state.duration) * 1000) : 0;
            seekEl.value = String(value);
            setFill(value / 10);
        }
    }

    function setPage(next) {
        page = next;
        update();
    }

    function invoke(name) {
        const ui = control();
        if (!ui) return;
        try {
            ui[name]();
        } catch (error) {
            return;
        }
        window.setTimeout(update, 120);
    }

    prevBtn.addEventListener('click', function () {
        invoke('prev');
    });

    nextBtn.addEventListener('click', function () {
        invoke('next');
    });

    toggleBtn.addEventListener('click', function () {
        const state = readState();
        if (!state) return;
        invoke(state.playing ? 'pause' : 'play');
    });

    expandBtn.addEventListener('click', function () {
        if (window.__openContentPage) window.__openContentPage('music');
        const ui = control();
        if (ui) ui.setMini(false);
        update();
    });

    seekEl.addEventListener('pointerdown', function () {
        dragging = true;
    });

    seekEl.addEventListener('pointerup', function () {
        dragging = false;
    });

    seekEl.addEventListener('input', function () {
        const ui = control();
        const state = readState();
        if (!ui || !state || !(state.duration > 0)) return;
        try {
            ui.seek((Number(seekEl.value) / 1000) * state.duration);
        } catch (error) {
            return;
        }
        setFill(Number(seekEl.value) / 10);
    });

    seekEl.addEventListener('change', function () {
        dragging = false;
        update();
    });

    frame.addEventListener('load', update);

    function spectrumFrame() {
        window.requestAnimationFrame(spectrumFrame);
        if (!ascii || document.hidden) return;

        const minGap = reducedMotion ? 110 : 42;
        const now = performance.now();
        if (now - lastSpectrum < minGap) return;
        lastSpectrum = now;

        const state = readState();
        const ui = control();
        const panelTop = contentPanel ? contentPanel.getBoundingClientRect().top : window.innerHeight;
        const on = !!(state && state.playing && panelTop > window.innerHeight * 0.5);

        if (!on) {
            if (ascii.classList.contains('is-on')) {
                ascii.classList.remove('is-on');
                window.clearTimeout(clearTimer);
                clearTimer = window.setTimeout(function () {
                    if (!ascii.classList.contains('is-on')) ascii.textContent = '';
                }, 500);
            }
            return;
        }

        let data = null;
        try {
            data = ui.spectrum();
        } catch (error) {
            data = null;
        }
        if (!data || !data.length) return;

        const width = window.innerWidth;
        const mobile = width <= 700;
        const charWidth = mobile ? 4.82 : 6.02;
        const cols = Math.max(32, Math.min(170, Math.floor(width / charWidth)));
        const rows = mobile ? 6 : 8;
        const usable = Math.max(1, Math.floor(data.length * 0.64));
        const ramp = ["'", ':', '-', '=', '+', '*', '#', '%', '@'];
        const lines = [];

        for (let row = 0; row < rows; row++) {
            let line = '';
            for (let col = 0; col < cols; col++) {
                const position = col / cols;
                const bin = Math.min(usable - 1, Math.floor(Math.pow(position, 1.5) * usable));
                const level = Math.pow(data[bin] / 255, 1.3);
                const height = level * rows;
                if (row < height) {
                    const depth = height <= 1 ? 0 : row / (height - 1);
                    const index = Math.min(ramp.length - 1, Math.floor(depth * (ramp.length - 1)));
                    line += ramp[index];
                } else {
                    line += ' ';
                }
            }
            lines.push(line);
        }

        ascii.textContent = lines.join('\n');
        ascii.classList.add('is-on');
    }

    window.__hostPlayer = {
        setPage: setPage,
        refresh: update
    };

    window.setInterval(update, 300);
    update();
    window.requestAnimationFrame(spectrumFrame);
})();
