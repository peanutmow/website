(function () {
    'use strict';

    const root = document.getElementById('xmms');
    const dataEl = document.getElementById('music-tracks');
    if (!root || !dataEl) return;

    if (window.self !== window.top) document.body.classList.add('embedded');

    let payload;
    try {
        payload = JSON.parse(dataEl.textContent);
    } catch (error) {
        console.error('music: unreadable track list', error);
        return;
    }

    const tracks = (payload.tracks || [])
        .filter(function (track) {
            return track && track.src;
        })
        .map(function (track) {
            return {
                file: track.file || '',
                src: track.src,
                title: track.title || '',
                artist: track.artist || '',
                album: track.album || '',
                duration: typeof track.duration === 'number' ? track.duration : null,
                bytes: typeof track.bytes === 'number' ? track.bytes : 0,
                soundcloud: typeof track.soundcloud === 'string' ? track.soundcloud : '',
                local: false,
            };
        });

    const EQ_BANDS = [60, 170, 310, 600, 1000, 3000, 6000, 12000, 14000, 16000];
    const PRESETS = [
        { name: 'flat', pre: 0, gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
        { name: 'rock', pre: 2, gains: [5, 4, 3, 1, -1, -1, 1, 3, 4, 4] },
        { name: 'techno', pre: 2, gains: [5, 4, 2, 0, -1, 1, 3, 4, 4, 4] },
        { name: 'vocal', pre: 1, gains: [-2, -1, 1, 3, 4, 4, 3, 1, 0, -1] }
    ];
    const PREFS_KEY = 'music.player.v1';
    const reducedMotion = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    const mainEl = document.getElementById('xmms-main');
    const titlebar = document.getElementById('xmms-titlebar');
    const shadeBtn = document.getElementById('xmms-shade');
    const statusEl = document.getElementById('xmms-status');
    const timeEl = document.getElementById('xmms-time');
    const durationEl = document.getElementById('xmms-duration');
    const kbpsEl = document.getElementById('xmms-kbps');
    const rateEl = document.getElementById('xmms-rate');
    const counterEl = document.getElementById('xmms-counter');
    const statusCountEl = document.getElementById('xmms-status-count');
    const statusTotalEl = document.getElementById('xmms-status-total');
    const marqueeEl = document.getElementById('xmms-marquee');
    const marqueeText = document.getElementById('xmms-marquee-text');
    const liveEl = document.getElementById('xmms-live');
    const seekEl = document.getElementById('xmms-seek');
    const volumeEl = document.getElementById('xmms-volume');
    const balanceEl = document.getElementById('xmms-balance');
    const prevBtn = document.getElementById('xmms-prev');
    const playBtn = document.getElementById('xmms-play');
    const pauseBtn = document.getElementById('xmms-pause');
    const stopBtn = document.getElementById('xmms-stop');
    const nextBtn = document.getElementById('xmms-next');
    const ejectBtn = document.getElementById('xmms-eject');
    const shuffleBtn = document.getElementById('xmms-shuffle');
    const repeatBtn = document.getElementById('xmms-repeat');
    const eqBtn = document.getElementById('xmms-eq-btn');
    const plBtn = document.getElementById('xmms-pl-btn');
    const eqClose = document.getElementById('xmms-eq-close');
    const plClose = document.getElementById('xmms-pl-close');
    const eqToggle = document.getElementById('xmms-eq-toggle');
    const presetBtn = document.getElementById('xmms-eq-preset');
    const listEl = document.getElementById('xmms-pl-list');
    const countEl = document.getElementById('xmms-pl-count');
    const addBtn = document.getElementById('xmms-add');
    const removeBtn = document.getElementById('xmms-remove');
    const sortBtn = document.getElementById('xmms-sort');
    const fileInput = document.getElementById('xmms-file');
    const vis = document.getElementById('xmms-vis');
    const visContext = vis.getContext('2d');
    const eqInputs = Array.prototype.slice.call(
        document.querySelectorAll('#xmms-eq input[type="range"]')
    );

    const audio = new Audio();
    audio.preload = 'metadata';

    const state = {
        index: -1,
        shuffle: false,
        repeat: false,
        eqOn: true,
        vis: reducedMotion ? 'off' : 'bars',
        sort: 'file',
        mini: false,
        remaining: false,
        presetIndex: 0
    };

    let eqGains = EQ_BANDS.map(function () {
        return 0;
    });
    let preampGain = 0;

    let audioContext = null;
    let analyser = null;
    let filters = [];
    let preampNode = null;
    let pannerNode = null;
    let freqData = null;
    let waveData = null;

    let peaks = [];
    let lastFrame = 0;
    let spectrumBuffer = null;

    function pad(value) {
        return String(value).padStart(2, '0');
    }

    function formatTime(seconds) {
        if (typeof seconds !== 'number' || !isFinite(seconds) || seconds < 0) return '--:--';
        const total = Math.round(seconds);
        const minutes = Math.floor(total / 60);
        const rest = total % 60;
        if (minutes >= 60) {
            return Math.floor(minutes / 60) + ':' + pad(minutes % 60) + ':' + pad(rest);
        }
        return minutes + ':' + pad(rest);
    }

    function setFill(input, percent) {
        if (!input) return;
        const clamped = Math.max(0, Math.min(100, percent));
        input.style.setProperty('--p', clamped + '%');
    }

    function currentTrack() {
        return tracks[state.index] || null;
    }

    function setStatus(word) {
        statusEl.textContent = word;
        statusEl.classList.toggle('is-live', word === 'PLAY');
    }

    function announce(text) {
        liveEl.textContent = text;
    }

    function ensureAudioGraph() {
        if (audioContext) {
            if (audioContext.state === 'suspended') audioContext.resume();
            return;
        }
        const AudioContextClass = window.AudioContext || window.webkitAudioContext;
        if (!AudioContextClass) return;

        try {
            audioContext = new AudioContextClass();
        } catch (error) {
            audioContext = null;
            return;
        }

        let node;
        try {
            node = audioContext.createMediaElementSource(audio);
        } catch (error) {
            return;
        }

        preampNode = audioContext.createGain();
        preampNode.gain.value = 1;

        filters = EQ_BANDS.map(function (frequency) {
            const filter = audioContext.createBiquadFilter();
            filter.type = 'peaking';
            filter.frequency.value = frequency;
            filter.Q.value = 1;
            filter.gain.value = 0;
            return filter;
        });

        pannerNode = typeof audioContext.createStereoPanner === 'function'
            ? audioContext.createStereoPanner()
            : null;

        analyser = audioContext.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.78;

        node.connect(preampNode);
        node = preampNode;
        filters.forEach(function (filter) {
            node.connect(filter);
            node = filter;
        });
        if (pannerNode) {
            node.connect(pannerNode);
            node = pannerNode;
            pannerNode.pan.value = Number(balanceEl.value) / 50;
        }
        node.connect(analyser);
        analyser.connect(audioContext.destination);

        freqData = new Uint8Array(analyser.frequencyBinCount);
        waveData = new Uint8Array(analyser.fftSize);
        rateEl.textContent = Math.round(audioContext.sampleRate / 1000) + ' khz';
        applyEq();
    }

    function applyEq() {
        if (!audioContext || !preampNode) return;
        preampNode.gain.value = state.eqOn ? Math.pow(10, preampGain / 20) : 1;
        filters.forEach(function (filter, index) {
            filter.gain.value = state.eqOn ? eqGains[index] : 0;
        });
    }

    function updateTransport() {
        const hasSource = !!audio.currentSrc;
        playBtn.classList.toggle('is-on', hasSource && !audio.paused);
        pauseBtn.classList.toggle('is-on', hasSource && audio.paused && audio.currentTime > 0);
        shuffleBtn.classList.toggle('is-on', state.shuffle);
        shuffleBtn.setAttribute('aria-pressed', String(state.shuffle));
        repeatBtn.classList.toggle('is-on', state.repeat);
        repeatBtn.setAttribute('aria-pressed', String(state.repeat));
        eqBtn.classList.toggle('is-on', root.classList.contains('eq-open'));
        eqBtn.setAttribute('aria-pressed', String(root.classList.contains('eq-open')));
        plBtn.classList.toggle('is-on', root.classList.contains('pl-open'));
        plBtn.setAttribute('aria-pressed', String(root.classList.contains('pl-open')));
    }

    function renderReadout() {
        const track = currentTrack();
        durationEl.textContent = track && track.duration ? formatTime(track.duration) : '--:--';
        const kbps = track && track.duration && track.bytes
            ? Math.round((track.bytes * 8) / track.duration / 1000)
            : 0;
        kbpsEl.textContent = kbps ? kbps + ' kbps' : '-- kbps';
        counterEl.textContent = tracks.length
            ? (state.index >= 0 ? state.index + 1 : 0) + '/' + tracks.length
            : '0/0';
    }

    function updateStatusBar() {
        const total = tracks.reduce(function (sum, track) {
            return sum + (track.duration || 0);
        }, 0);
        const partial = tracks.some(function (track) {
            return !track.duration;
        });
        statusCountEl.textContent = tracks.length + (tracks.length === 1 ? ' track' : ' tracks');
        statusTotalEl.textContent = total
            ? formatTime(total) + (partial ? '+' : '') + ' total'
            : '--:-- total';
    }

    function updateTime() {
        const track = currentTrack();
        const duration = isFinite(audio.duration) && audio.duration > 0
            ? audio.duration
            : (track && track.duration) || 0;
        const position = audio.currentTime || 0;

        timeEl.textContent = state.remaining && duration
            ? '-' + formatTime(Math.max(0, duration - position))
            : formatTime(position);
        timeEl.setAttribute(
            'aria-label',
            state.remaining ? 'Time remaining, click for elapsed' : 'Time elapsed, click for remaining'
        );

        if (duration > 0) {
            const value = Math.round((position / duration) * 1000);
            if (document.activeElement !== seekEl) {
                seekEl.value = String(value);
            }
            setFill(seekEl, value / 10);
        } else {
            seekEl.value = '0';
            setFill(seekEl, 0);
        }
    }

    function updateMarquee() {
        const track = currentTrack();
        let text = 'xmms 1.2 \u2014 no track loaded';
        if (track) {
            const name = track.artist ? track.artist + ' - ' + track.title : track.title;
            const suffix = track.duration ? ' (' + formatTime(track.duration) + ')' : '';
            text = (state.index + 1) + '. ' + name + suffix;
        } else if (!tracks.length) {
            text = 'xmms 1.2 \u2014 playlist is empty';
        }

        marqueeEl.classList.remove('is-scrolling');
        marqueeText.textContent = text;

        if (marqueeText.scrollWidth > marqueeEl.clientWidth) {
            marqueeText.textContent = text + '   \u00b7   ' + text;
            const speed = Math.max(8, Math.round(marqueeText.scrollWidth / 18));
            marqueeEl.style.setProperty('--scroll-duration', speed + 's');
            void marqueeText.offsetWidth;
            marqueeEl.classList.add('is-scrolling');
        }
    }

    function markCurrent() {
        const rows = listEl.querySelectorAll('.xmms-pl-row');
        Array.prototype.forEach.call(rows, function (row) {
            const isCurrent = Number(row.dataset.index) === state.index;
            row.classList.toggle('is-current', isCurrent);
            if (isCurrent) row.setAttribute('aria-current', 'true');
            else row.removeAttribute('aria-current');
        });
        const active = listEl.querySelector('.xmms-pl-row.is-current');
        if (active && typeof active.scrollIntoView === 'function') {
            active.scrollIntoView({ block: 'nearest' });
        }
    }

    function renderList() {
        listEl.textContent = '';

        if (!tracks.length) {
            const empty = document.createElement('li');
            empty.className = 'xmms-pl-empty';
            empty.textContent = 'no tracks yet';
            listEl.appendChild(empty);
        } else {
            const fragment = document.createDocumentFragment();
            tracks.forEach(function (track, index) {
                const item = document.createElement('li');
                const button = document.createElement('button');
                button.type = 'button';
                button.className = 'xmms-pl-row';
                button.dataset.index = String(index);

                const number = document.createElement('span');
                number.className = 'xmms-pl-num';
                number.textContent = (index + 1) + '.';

                const name = document.createElement('span');
                name.className = 'xmms-pl-name';
                name.textContent = track.artist ? track.artist + ' - ' + track.title : track.title;

                const time = document.createElement('span');
                time.className = 'xmms-pl-time';
                time.textContent = track.duration ? formatTime(track.duration) : '--:--';

                button.appendChild(number);
                button.appendChild(name);
                button.appendChild(time);
                button.addEventListener('click', function () {
                    load(index, true);
                });

                item.appendChild(button);

                if (track.soundcloud) {
                    const link = document.createElement('a');
                    link.className = 'xmms-pl-sc';
                    link.href = track.soundcloud;
                    link.target = '_blank';
                    link.rel = 'noopener noreferrer';
                    link.title = 'Listen on SoundCloud';
                    link.setAttribute('aria-label', (track.artist ? track.artist + ' - ' : '') + track.title + ' on SoundCloud');
                    link.innerHTML = '<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M4.7 12.3h7.1a2.6 2.6 0 0 0 .3-5.2A4.3 4.3 0 0 0 4.2 7.5a2.4 2.4 0 0 0 .5 4.8Z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
                    item.appendChild(link);
                }

                fragment.appendChild(item);
            });
            listEl.appendChild(fragment);
        }

        countEl.textContent = tracks.length + (tracks.length === 1 ? ' track' : ' tracks');
        listEl.style.setProperty('--rows', String(tracks.length || 1));
        updateStatusBar();
        markCurrent();
    }

    function select(index) {
        state.index = index;
        updateMarquee();
        renderReadout();
        updateTime();
        markCurrent();
        updateTransport();
    }

    function load(index, autoplay) {
        if (index < 0 || index >= tracks.length) return;
        root.classList.remove('is-playing');
        select(index);
        audio.src = tracks[index].src;
        audio.load();
        seekEl.value = '0';
        setFill(seekEl, 0);
        timeEl.textContent = '0:00';
        setStatus('STOP');
        announce('Loaded ' + tracks[index].title + (tracks[index].artist ? ' by ' + tracks[index].artist : ''));
        updateMediaSession();
        if (autoplay) play();
        savePrefs();
    }

    function play() {
        if (!tracks.length) return;
        if (state.index < 0) {
            load(0, true);
            return;
        }
        if (!audio.src) {
            load(state.index, true);
            return;
        }
        ensureAudioGraph();
        const promise = audio.play();
        if (promise && typeof promise.catch === 'function') {
            promise.catch(function () {
                setStatus('PAUSE');
                updateTransport();
            });
        }
    }

    function pause() {
        audio.pause();
    }

    function stop() {
        audio.pause();
        root.classList.remove('is-playing');
        if (audio.src) {
            try {
                audio.currentTime = 0;
            } catch (error) {
                audio.currentTime = 0;
            }
        }
        setStatus('STOP');
        updateTime();
        updateTransport();
        announce('Stopped');
    }

    function eject() {
        audio.pause();
        root.classList.remove('is-playing');
        audio.removeAttribute('src');
        audio.load();
        state.index = -1;
        seekEl.value = '0';
        setFill(seekEl, 0);
        timeEl.textContent = '0:00';
        durationEl.textContent = '--:--';
        setStatus('STOP');
        updateMarquee();
        updateTransport();
        markCurrent();
        savePrefs();
    }

    function advance(direction, automatic) {
        if (!tracks.length) return;
        if (automatic && state.repeat) {
            if (audio.src) {
                audio.currentTime = 0;
            }
            play();
            return;
        }
        if (automatic && !state.shuffle && state.index === tracks.length - 1) {
            stop();
            return;
        }
        if (state.index < 0) {
            load(direction > 0 ? 0 : tracks.length - 1, true);
            return;
        }
        if (state.shuffle && tracks.length > 1) {
            let index = state.index;
            while (index === state.index) {
                index = Math.floor(Math.random() * tracks.length);
            }
            load(index, true);
            return;
        }
        const nextIndex = (state.index + direction + tracks.length) % tracks.length;
        load(nextIndex, true);
    }

    function previous() {
        if (audio.currentTime > 3) {
            audio.currentTime = 0;
            updateTime();
            return;
        }
        advance(-1, false);
    }

    function nudge(seconds) {
        if (!isFinite(audio.duration) || !audio.duration) return;
        audio.currentTime = Math.max(0, Math.min(audio.duration, audio.currentTime + seconds));
        updateTime();
    }

    function setVolume(value) {
        const clamped = Math.max(0, Math.min(100, Math.round(value)));
        volumeEl.value = String(clamped);
        audio.volume = clamped / 100;
        setFill(volumeEl, clamped);
        savePrefs();
    }

    function toggleShuffle() {
        state.shuffle = !state.shuffle;
        updateTransport();
        savePrefs();
    }

    function toggleRepeat() {
        state.repeat = !state.repeat;
        updateTransport();
        savePrefs();
    }

    function setEqOpen(open) {
        root.classList.toggle('eq-open', open);
        updateTransport();
    }

    function setPlOpen(open) {
        root.classList.toggle('pl-open', open);
        updateTransport();
    }

    function setMini(mini) {
        state.mini = mini;
        root.classList.toggle('is-mini', mini);
        document.body.classList.toggle('xmms-mini', mini);
        shadeBtn.setAttribute('aria-expanded', String(!mini));
        shadeBtn.setAttribute('aria-label', mini ? 'Restore player' : 'Windowshade');
        if (!mini && typeof mainEl.scrollIntoView === 'function') {
            mainEl.scrollIntoView({ block: 'nearest' });
        }
    }

    function toggle() {
        if (audio.paused) play();
        else pause();
    }

    function parseName(name) {
        let stem = name.trim();
        const prefix = /^\d{1,3}\s*[.\-_]\s*/.exec(stem);
        if (prefix && prefix[0].length < stem.length) stem = stem.slice(prefix[0].length);
        const separators = [' - ', ' \u2013 ', ' \u2014 '];
        for (let index = 0; index < separators.length; index++) {
            const at = stem.indexOf(separators[index]);
            if (at > 0) {
                const artist = stem.slice(0, at).trim();
                const title = stem.slice(at + separators[index].length).trim();
                if (artist && title) return { artist: artist, title: title };
            }
        }
        return { artist: '', title: stem };
    }

    function isAudioFile(file) {
        if (file.type && file.type.indexOf('audio/') === 0) return true;
        return /\.(mp3|m4a|aac|ogg|oga|opus|flac|wav)$/i.test(file.name);
    }

    function sortTracks(mode) {
        const current = currentTrack();
        const compare = function (a, b) {
            if (mode === 'artist') {
                return (a.artist || a.title).localeCompare(b.artist || b.title, undefined, { sensitivity: 'base' });
            }
            if (mode === 'title') {
                return a.title.localeCompare(b.title, undefined, { sensitivity: 'base' });
            }
            return a.file.localeCompare(b.file, undefined, { sensitivity: 'base' });
        };

        state.sort = mode;
        tracks.sort(compare);
        sortBtn.textContent = mode;
        state.index = current ? tracks.indexOf(current) : -1;
        renderList();
        updateMarquee();
        renderReadout();
        savePrefs();
    }

    function updateMediaSession() {
        if (!('mediaSession' in navigator) || !window.MediaMetadata) return;
        const track = currentTrack();
        if (!track) return;
        navigator.mediaSession.metadata = new MediaMetadata({
            title: track.title,
            artist: track.artist,
            album: track.album || ''
        });
    }

    function savePrefs() {
        const prefs = {
            volume: Number(volumeEl.value),
            balance: Number(balanceEl.value),
            shuffle: state.shuffle,
            repeat: state.repeat,
            eqOn: state.eqOn,
            preamp: preampGain,
            gains: eqGains,
            preset: state.presetIndex,
            vis: state.vis,
            sort: state.sort,
            index: state.index
        };
        try {
            localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
        } catch (error) {
            return;
        }
    }

    function restorePrefs() {
        let prefs = {};
        try {
            prefs = JSON.parse(localStorage.getItem(PREFS_KEY) || '{}') || {};
        } catch (error) {
            prefs = {};
        }

        if (typeof prefs.volume === 'number') volumeEl.value = String(Math.max(0, Math.min(100, prefs.volume)));
        if (typeof prefs.balance === 'number') balanceEl.value = String(Math.max(-50, Math.min(50, prefs.balance)));
        state.shuffle = !!prefs.shuffle;
        state.repeat = !!prefs.repeat;
        state.eqOn = prefs.eqOn !== false;

        if (Array.isArray(prefs.gains) && prefs.gains.length === EQ_BANDS.length) {
            eqGains = prefs.gains.map(function (value) {
                const number = Number(value);
                return isFinite(number) ? Math.max(-12, Math.min(12, number)) : 0;
            });
        }
        if (typeof prefs.preamp === 'number' && isFinite(prefs.preamp)) {
            preampGain = Math.max(-12, Math.min(12, prefs.preamp));
        }
        state.presetIndex = typeof prefs.preset === 'number' ? prefs.preset : 0;
        if (prefs.vis === 'bars' || prefs.vis === 'scope' || prefs.vis === 'off') state.vis = prefs.vis;
        if (prefs.sort === 'file' || prefs.sort === 'artist' || prefs.sort === 'title') state.sort = prefs.sort;
        if (typeof prefs.index === 'number' && prefs.index >= 0 && prefs.index < tracks.length) {
            state.index = prefs.index;
        }

        audio.volume = Number(volumeEl.value) / 100;
        setFill(volumeEl, Number(volumeEl.value));
        setFill(balanceEl, Number(balanceEl.value) + 50);
        setFill(seekEl, 0);
        sortBtn.textContent = state.sort;

        eqInputs.forEach(function (input) {
            const band = input.dataset.band;
            input.value = String(band === 'pre' ? preampGain : (eqGains[Number(band)] || 0));
        });

        eqToggle.textContent = state.eqOn ? 'on' : 'off';
        eqToggle.setAttribute('aria-pressed', String(state.eqOn));
        eqToggle.classList.toggle('is-on', state.eqOn);
        presetBtn.textContent = state.presetIndex >= 0 && PRESETS[state.presetIndex]
            ? PRESETS[state.presetIndex].name
            : 'custom';
    }

    function applyPreset(index) {
        const preset = PRESETS[index];
        preampGain = preset.pre;
        eqGains = preset.gains.slice();
        state.presetIndex = index;
        presetBtn.textContent = preset.name;
        eqInputs.forEach(function (input) {
            const band = input.dataset.band;
            input.value = String(band === 'pre' ? preampGain : eqGains[Number(band)]);
        });
        applyEq();
        savePrefs();
    }

    function fitCanvas() {
        const ratio = Math.min(window.devicePixelRatio || 1, 2);
        const rect = vis.getBoundingClientRect();
        if (!rect.width || !rect.height) return;
        vis.width = Math.round(rect.width * ratio);
        vis.height = Math.round(rect.height * ratio);
        peaks = [];
    }

    function roundRect(context, x, y, width, height, radius) {
        if (typeof context.roundRect === 'function') {
            context.beginPath();
            context.roundRect(x, y, width, height, radius);
            return;
        }
        context.beginPath();
        context.rect(x, y, width, height);
    }

    function drawBars(context, width, height) {
        analyser.getByteFrequencyData(freqData);
        const barCount = Math.min(48, Math.max(16, Math.round(width / 9)));
        if (peaks.length !== barCount) peaks = new Array(barCount).fill(0);

        const slot = width / barCount;
        const barWidth = Math.max(2, Math.round(slot) - 3);
        const usable = Math.max(1, Math.floor(freqData.length * 0.66));
        const gradient = context.createLinearGradient(0, height, 0, 0);
        gradient.addColorStop(0, '#2f8fe0');
        gradient.addColorStop(0.42, '#63f0c2');
        gradient.addColorStop(1, '#dcfff4');

        context.save();
        context.shadowColor = 'rgba(99, 240, 194, 0.5)';
        context.shadowBlur = 7;
        context.fillStyle = gradient;

        for (let index = 0; index < barCount; index++) {
            const position = index / barCount;
            const bin = Math.min(usable - 1, Math.floor(Math.pow(position, 1.5) * usable));
            const value = freqData[bin] / 255;
            const barHeight = Math.max(1, value * (height - 6));
            const x = Math.round(index * slot + slot * 0.15);
            roundRect(context, x, height - barHeight, barWidth, barHeight, 2);
            context.fill();

            peaks[index] = Math.max(barHeight, peaks[index] - height * 0.02);
        }
        context.restore();

        context.fillStyle = 'rgba(232, 255, 248, 0.85)';
        for (let index = 0; index < barCount; index++) {
            if (peaks[index] <= 0) continue;
            const x = Math.round(index * slot + slot * 0.15);
            const y = height - Math.min(height - 2, peaks[index]) - 1;
            context.fillRect(x, y, barWidth, 2);
        }
    }

    function drawScope(context, width, height) {
        analyser.getByteTimeDomainData(waveData);
        const step = width / Math.max(1, waveData.length - 1);

        context.beginPath();
        context.moveTo(0, height / 2);
        for (let index = 0; index < waveData.length; index++) {
            const value = waveData[index] / 128;
            context.lineTo(index * step, (value * height) / 2);
        }
        context.lineTo(width, height);
        context.lineTo(0, height);
        context.closePath();
        const fill = context.createLinearGradient(0, 0, 0, height);
        fill.addColorStop(0, 'rgba(126, 247, 207, 0.28)');
        fill.addColorStop(1, 'rgba(126, 247, 207, 0.02)');
        context.fillStyle = fill;
        context.fill();

        context.save();
        context.beginPath();
        for (let index = 0; index < waveData.length; index++) {
            const value = waveData[index] / 128;
            const y = (value * height) / 2;
            const x = index * step;
            if (index === 0) context.moveTo(x, y);
            else context.lineTo(x, y);
        }
        context.strokeStyle = '#c8ffe9';
        context.shadowColor = 'rgba(126, 247, 207, 0.9)';
        context.shadowBlur = 8;
        context.lineWidth = Math.max(1, Math.round(height / 48));
        context.stroke();
        context.restore();

        context.fillStyle = 'rgba(255, 255, 255, 0.12)';
        context.fillRect(0, Math.round(height / 2), width, 1);
    }

    function draw() {
        window.requestAnimationFrame(draw);
        if (document.hidden) return;

        const now = performance.now();
        if (now - lastFrame < 33) return;
        lastFrame = now;

        const width = vis.width;
        const height = vis.height;
        if (!width || !height) return;

        visContext.clearRect(0, 0, width, height);

        if (state.vis === 'off') {
            visContext.fillStyle = 'rgba(126, 247, 207, 0.2)';
            visContext.fillRect(0, Math.round(height / 2), width, 1);
            return;
        }
        if (!analyser) {
            const idle = visContext.createLinearGradient(0, height, 0, 0);
            idle.addColorStop(0, 'rgba(47, 143, 224, 0.06)');
            idle.addColorStop(1, 'rgba(126, 247, 207, 0.22)');
            visContext.fillStyle = idle;
            visContext.fillRect(0, height - 2, width, 2);
            return;
        }
        if (state.vis === 'scope') drawScope(visContext, width, height);
        else drawBars(visContext, width, height);
    }

    function cycleVisualizer() {
        state.vis = state.vis === 'bars' ? 'scope' : state.vis === 'scope' ? 'off' : 'bars';
        savePrefs();
    }

    vis.addEventListener('click', cycleVisualizer);
    vis.addEventListener('keydown', function (event) {
        if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            event.stopPropagation();
            cycleVisualizer();
        }
    });

    audio.addEventListener('play', function () {
        root.classList.add('is-playing');
        setStatus('PLAY');
        updateTransport();
    });

    audio.addEventListener('pause', function () {
        root.classList.remove('is-playing');
        setStatus(audio.currentTime > 0 && !audio.ended ? 'PAUSE' : 'STOP');
        updateTransport();
    });

    audio.addEventListener('timeupdate', updateTime);

    audio.addEventListener('durationchange', renderReadout);

    audio.addEventListener('loadedmetadata', function () {
        const track = currentTrack();
        if (track && isFinite(audio.duration) && audio.duration > 0) {
            track.duration = audio.duration;
            const time = listEl.querySelector('.xmms-pl-row[data-index="' + state.index + '"] .xmms-pl-time');
            if (time) time.textContent = formatTime(track.duration);
            renderReadout();
            updateStatusBar();
            updateMarquee();
        }
        updateTime();
    });

    audio.addEventListener('ended', function () {
        advance(1, true);
    });

    audio.addEventListener('error', function () {
        if (!audio.getAttribute('src')) return;
        const track = currentTrack();
        setStatus('ERROR');
        marqueeEl.classList.remove('is-scrolling');
        marqueeText.textContent = 'cannot play ' + (track ? track.file : 'this track');
        announce('This track could not be played');
        window.setTimeout(function () {
            if (tracks.length > 1) advance(1, true);
            else setStatus('STOP');
        }, 1200);
    });

    timeEl.addEventListener('click', function () {
        state.remaining = !state.remaining;
        updateTime();
    });

    seekEl.addEventListener('input', function () {
        const value = Number(seekEl.value);
        setFill(seekEl, value / 10);
        if (isFinite(audio.duration) && audio.duration > 0) {
            audio.currentTime = (value / 1000) * audio.duration;
        }
        updateTime();
    });

    volumeEl.addEventListener('input', function () {
        audio.volume = Number(volumeEl.value) / 100;
        setFill(volumeEl, Number(volumeEl.value));
        savePrefs();
    });

    balanceEl.addEventListener('input', function () {
        if (pannerNode) pannerNode.pan.value = Number(balanceEl.value) / 50;
        setFill(balanceEl, Number(balanceEl.value) + 50);
        savePrefs();
    });

    prevBtn.addEventListener('click', previous);
    playBtn.addEventListener('click', play);
    pauseBtn.addEventListener('click', pause);
    stopBtn.addEventListener('click', stop);
    nextBtn.addEventListener('click', function () {
        advance(1, false);
    });
    ejectBtn.addEventListener('click', eject);

    shuffleBtn.addEventListener('click', toggleShuffle);
    repeatBtn.addEventListener('click', toggleRepeat);

    eqBtn.addEventListener('click', function () {
        setEqOpen(!root.classList.contains('eq-open'));
    });
    plBtn.addEventListener('click', function () {
        setPlOpen(!root.classList.contains('pl-open'));
    });
    eqClose.addEventListener('click', function () {
        setEqOpen(false);
    });
    plClose.addEventListener('click', function () {
        setPlOpen(false);
    });

    shadeBtn.addEventListener('click', function () {
        setMini(!state.mini);
    });
    titlebar.addEventListener('dblclick', function () {
        setMini(!state.mini);
    });

    eqInputs.forEach(function (input) {
        input.addEventListener('input', function () {
            const band = input.dataset.band;
            const value = Number(input.value);
            if (band === 'pre') preampGain = value;
            else eqGains[Number(band)] = value;
            state.presetIndex = -1;
            presetBtn.textContent = 'custom';
            applyEq();
            savePrefs();
        });
    });

    eqToggle.addEventListener('click', function () {
        state.eqOn = !state.eqOn;
        eqToggle.textContent = state.eqOn ? 'on' : 'off';
        eqToggle.setAttribute('aria-pressed', String(state.eqOn));
        eqToggle.classList.toggle('is-on', state.eqOn);
        applyEq();
        savePrefs();
    });

    presetBtn.addEventListener('click', function () {
        const next = state.presetIndex < 0 ? 0 : (state.presetIndex + 1) % PRESETS.length;
        applyPreset(next);
    });

    addBtn.addEventListener('click', function () {
        fileInput.click();
    });

    fileInput.addEventListener('change', function () {
        const files = Array.prototype.slice.call(fileInput.files || []);
        const added = [];

        files.forEach(function (file) {
            if (!isAudioFile(file)) return;
            const parsed = parseName(file.name.replace(/\.[^.]+$/, ''));
            const track = {
                file: file.name,
                src: URL.createObjectURL(file),
                title: parsed.title,
                artist: parsed.artist,
                album: '',
                duration: null,
                bytes: file.size,
                local: true
            };
            tracks.push(track);
            added.push(track);
        });

        fileInput.value = '';
        if (!added.length) return;

        sortTracks(state.sort);
        if (state.index < 0) {
            load(tracks.indexOf(added[0]), false);
        }
        announce('Added ' + added.length + (added.length === 1 ? ' track' : ' tracks'));
    });

    removeBtn.addEventListener('click', function () {
        const track = currentTrack();
        if (!track) return;

        const wasPlaying = !audio.paused;
        tracks.splice(state.index, 1);

        if (!tracks.length) {
            eject();
            if (track.local) URL.revokeObjectURL(track.src);
            renderList();
            return;
        }

        const nextIndex = Math.min(state.index, tracks.length - 1);
        load(nextIndex, wasPlaying);
        if (track.local) URL.revokeObjectURL(track.src);
        renderList();
    });

    sortBtn.addEventListener('click', function () {
        const order = ['file', 'artist', 'title'];
        const next = order[(order.indexOf(state.sort) + 1) % order.length];
        sortTracks(next);
    });

    document.addEventListener('keydown', function (event) {
        if (event.ctrlKey || event.metaKey || event.altKey) return;
        const target = event.target;
        const inField = target && target.closest && target.closest('input, textarea, select, [contenteditable="true"]');
        if (inField) {
            if (event.key === 'Escape' && state.mini) setMini(false);
            return;
        }
        const onButton = target && target.closest && target.closest('button');
        if (onButton && (event.key === ' ' || event.key === 'Enter')) return;

        switch (event.key) {
            case ' ':
                event.preventDefault();
                toggle();
                return;
            case 'ArrowRight':
                event.preventDefault();
                nudge(event.shiftKey ? 30 : 5);
                return;
            case 'ArrowLeft':
                event.preventDefault();
                nudge(event.shiftKey ? -30 : -5);
                return;
            case 'ArrowUp':
                event.preventDefault();
                setVolume(Number(volumeEl.value) + 5);
                return;
            case 'ArrowDown':
                event.preventDefault();
                setVolume(Number(volumeEl.value) - 5);
                return;
            case 'Escape':
                if (state.mini) setMini(false);
                return;
            default:
                break;
        }

        const letter = event.key.toLowerCase();
        if (letter.length !== 1) return;
        const actions = {
            z: previous,
            b: function () {
                advance(1, false);
            },
            x: play,
            c: pause,
            v: stop,
            s: toggleShuffle,
            r: toggleRepeat,
            e: function () {
                setEqOpen(!root.classList.contains('eq-open'));
            },
            l: function () {
                setPlOpen(!root.classList.contains('pl-open'));
            },
            m: function () {
                setMini(!state.mini);
            }
        };
        if (actions[letter]) {
            event.preventDefault();
            actions[letter]();
        }
    });

    if ('mediaSession' in navigator) {
        try {
            navigator.mediaSession.setActionHandler('play', play);
            navigator.mediaSession.setActionHandler('pause', pause);
            navigator.mediaSession.setActionHandler('stop', stop);
            navigator.mediaSession.setActionHandler('previoustrack', previous);
            navigator.mediaSession.setActionHandler('nexttrack', function () {
                advance(1, false);
            });
        } catch (error) {
            return;
        }
    }

    window.addEventListener('beforeunload', function () {
        tracks.forEach(function (track) {
            if (track.local) URL.revokeObjectURL(track.src);
        });
    });

    if (window.ResizeObserver) {
        new ResizeObserver(fitCanvas).observe(vis);
    } else {
        window.addEventListener('resize', fitCanvas);
    }

    restorePrefs();
    sortTracks(state.sort);
    updateMarquee();
    renderReadout();
    updateTime();
    updateTransport();
    setStatus('STOP');
    fitCanvas();
    draw();

    if (window.matchMedia && window.matchMedia('(max-width: 700px)').matches) {
        setEqOpen(false);
    }

    window.__xmmsUI = {
        state: function () {
            const track = currentTrack();
            return {
                loaded: !!track,
                playing: !audio.paused && !!audio.currentSrc,
                time: audio.currentTime || 0,
                duration: isFinite(audio.duration) && audio.duration > 0
                    ? audio.duration
                    : (track && track.duration) || 0,
                title: track ? track.title : '',
                artist: track ? track.artist : '',
                mini: state.mini,
                volume: Number(volumeEl.value) / 100
            };
        },
        play: play,
        pause: pause,
        stop: stop,
        next: function () {
            advance(1, false);
        },
        prev: previous,
        seek: function (seconds) {
            if (!isFinite(audio.duration) || !audio.duration) return;
            audio.currentTime = Math.max(0, Math.min(audio.duration, seconds));
            updateTime();
        },
        setMini: setMini,
        spectrum: function () {
            if (!analyser) return null;
            if (!spectrumBuffer || spectrumBuffer.length !== analyser.frequencyBinCount) {
                spectrumBuffer = new Uint8Array(analyser.frequencyBinCount);
            }
            analyser.getByteFrequencyData(spectrumBuffer);
            return spectrumBuffer;
        }
    };
})();
