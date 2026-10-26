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
    const contentPanel = document.getElementById('content-panel');
    const reducedMotion = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    let dragging = false;
    let lastSpectrum = 0;
    let spectrumOn = false;

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

    function visiblePanel() {
        if (!contentPanel) return false;
        return contentPanel.getBoundingClientRect().top < window.innerHeight * 0.5;
    }

    function update() {
        const state = readState();
        const expanded = !frame.hidden && visiblePanel();
        const show = !!state && state.loaded && !expanded;
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

    function setPage() {
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
        if (document.hidden) return;

        const minGap = reducedMotion ? 110 : 42;
        const now = performance.now();
        if (now - lastSpectrum < minGap) return;
        lastSpectrum = now;

        const sim = window.__pageWaterSim;
        if (!sim || typeof sim.spectrum !== 'function') return;

        const state = readState();
        const ui = control();
        const on = !!(state && state.playing && !visiblePanel());

        if (!on) {
            if (spectrumOn) {
                sim.spectrumVisible(false);
                spectrumOn = false;
            }
            return;
        }

        let data = null;
        try {
            data = ui ? ui.spectrum() : null;
        } catch (error) {
            data = null;
        }
        if (!data || !data.length) return;

        if (!spectrumOn) {
            sim.spectrumVisible(true);
            spectrumOn = true;
        }
        sim.spectrum(data);
    }

    const contentFrame = document.getElementById('content-frame');
    const socialsPanel = document.getElementById('socials-inline');
    const musicButton = document.querySelector('.btn[data-music]');

    function openMusic() {
        if (!frame.getAttribute('src')) frame.setAttribute('src', '/music?embed=1');
        frame.hidden = false;
        if (contentFrame) contentFrame.style.display = 'none';
        if (socialsPanel) socialsPanel.classList.remove('visible');
        setPage('music');
        if (contentPanel) contentPanel.scrollIntoView({ behavior: 'smooth' });
        if (window.__pageWaterSim) window.__pageWaterSim.stir();
    }

    window.__openContentPage = function (name) {
        if (name === 'music') openMusic();
    };

    if (musicButton) {
        musicButton.addEventListener('click', function (event) {
            event.preventDefault();
            openMusic();
        });
    }

    Array.prototype.forEach.call(document.querySelectorAll('.btn[data-page]'), function (button) {
        button.addEventListener('click', function () {
            frame.hidden = true;
            const next = button.getAttribute('data-page');
            if (next !== 'socials' && contentFrame) contentFrame.style.display = '';
            setPage(next);
        });
    });

    window.__hostPlayer = {
        setPage: setPage,
        refresh: update
    };

    window.setInterval(update, 300);
    update();
    window.requestAnimationFrame(spectrumFrame);
})();
