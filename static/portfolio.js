/**
 * portfolio.js — Terminal, Settings, and navigation for the CRT portfolio
 * Rust-powered rewritten version
 */

(function () {
    'use strict';

    const cmdInput = document.getElementById('cmd-input');
    const terminalScreen = document.getElementById('terminal-screen');
    const homeScreen = document.getElementById('home-screen');

    // ── Terminal entry ──
    if (cmdInput && terminalScreen && homeScreen) {
        terminalScreen.addEventListener('click', () => cmdInput.focus());

        cmdInput.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                const inputValue = cmdInput.value.trim();
                if (inputValue.length > 0) {
                    terminalScreen.classList.remove('active');
                    terminalScreen.classList.add('hidden');
                    homeScreen.classList.remove('hidden');
                    homeScreen.classList.add('active');

                    const asciiBunny = document.getElementById('ascii-bunny');
                    if (asciiBunny) asciiBunny.style.display = 'block';
                }
            }
        });
    }

    // ── Random screen flicker ──
    function randomFlicker() {
        document.querySelectorAll('.screen').forEach(s => {
            const current = parseFloat(getComputedStyle(s).opacity) || 1;
            let delta = (Math.random() * 0.1) - 0.05;
            s.style.opacity = Math.max(0.7, Math.min(1, current + delta));
        });
        setTimeout(randomFlicker, 300 + Math.random() * 1000);
    }
    randomFlicker();

    // ── Disabled links ──
    document.querySelectorAll('.disable-click').forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            console.log('ACCESS DENIED: Insufficient clearance level.');
        });
    });

    // ── Settings screen ──
    const settingsBtn = document.getElementById('settings-btn');
    const settingsScreen = document.getElementById('settings-screen');
    const closeSettingsBtn = document.getElementById('close-settings');

    if (settingsBtn && settingsScreen) {
        settingsBtn.addEventListener('click', () => {
            homeScreen.classList.add('hidden');
            homeScreen.classList.remove('active');
            settingsScreen.classList.remove('hidden');
            settingsScreen.classList.add('active');
            const bunny = document.getElementById('ascii-bunny');
            if (bunny) bunny.style.display = 'none';
        });
    }

    if (closeSettingsBtn && settingsScreen) {
        closeSettingsBtn.addEventListener('click', () => {
            settingsScreen.classList.add('hidden');
            settingsScreen.classList.remove('active');
            homeScreen.classList.remove('hidden');
            homeScreen.classList.add('active');
            const bunny = document.getElementById('ascii-bunny');
            if (bunny) bunny.style.display = 'block';
        });
    }

    // ── Settings terminal ──
    const settingsInput = document.getElementById('settings-input');
    const settingsLog = document.getElementById('settings-log');

    const availableCommands = {
        'theme': ['ui', 'text', 'clock', 'background', 'reset'],
        'volume': [],
        'brightness': [],
        'network': [],
        'reset': [],
        'help': []
    };

    const colorOptions = [
        'magi-orange', 'bg-color',
        'red', 'green', 'blue', 'orange', 'purple', 'yellow',
        'magenta', 'cyan', 'white', 'black', 'pink',
        '#ff0000', '#00ff00', '#0000ff'
    ];

    function logSettingsEntry(text) {
        if (!settingsLog) return;
        const div = document.createElement('div');
        div.className = 'entry';
        div.textContent = text;
        settingsLog.appendChild(div);
        settingsLog.scrollTop = settingsLog.scrollHeight;
    }

    function updateCSSVar(name, value) {
        document.documentElement.style.setProperty(name, value);
        try { localStorage.setItem(name.replace('--', 'theme-').replace('-ui', '-ui'), value); } catch (e) {}
    }

    // Load persisted theme
    ['theme-text', 'theme-ui', 'theme-clock', 'theme-bg'].forEach(k => {
        const v = localStorage.getItem(k);
        if (v) document.documentElement.style.setProperty('--' + k, v);
    });

    if (settingsInput) {
        const suggestElem = document.getElementById('settings-suggest');
        const commandHistory = [];
        let historyIndex = -1;

        settingsInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const cmd = settingsInput.value.trim();
                if (cmd.length) {
                    logSettingsEntry('> ' + cmd);
                    commandHistory.push(cmd);
                    historyIndex = -1;

                    const parts = cmd.split(' ');
                    const baseCmd = parts[0].toLowerCase();

                    if (baseCmd === 'theme') {
                        const target = parts[1] ? parts[1].toLowerCase() : null;
                        if (target === 'reset') {
                            ['--theme-text', '--theme-ui', '--theme-clock'].forEach(p =>
                                document.documentElement.style.setProperty(p, 'var(--magi-orange)'));
                            document.documentElement.style.setProperty('--theme-bg', 'var(--bg-color)');
                            ['theme-text', 'theme-ui', 'theme-clock', 'theme-bg'].forEach(k => localStorage.removeItem(k));
                            logSettingsEntry('Theme reset to defaults');
                        } else if (target && parts[2]) {
                            const color = parts[2];
                            if (target === 'ui' || target === 'text' || target === 'clock') {
                                updateCSSVar('--theme-' + target, color);
                                if (target === 'ui') {
                                    updateCSSVar('--theme-text', color);
                                    updateCSSVar('--theme-clock', color);
                                }
                                logSettingsEntry('Theme updated: ' + target + ' to ' + color);
                            } else if (target === 'background') {
                                updateCSSVar('--theme-bg', color);
                                logSettingsEntry('Background updated to ' + color);
                            }
                        } else {
                            logSettingsEntry('Usage: theme <ui|text|clock|background|reset> <color>');
                        }
                    } else if (baseCmd === 'help') {
                        logSettingsEntry('Available: theme, volume, brightness, network, reset, help');
                    } else if (baseCmd === 'reset' || baseCmd === 'volume' || baseCmd === 'brightness' || baseCmd === 'network') {
                        logSettingsEntry('Command "' + baseCmd + '" not implemented in this build.');
                    } else {
                        logSettingsEntry('Unknown command: ' + baseCmd + '. Type "help" for commands.');
                    }
                }
                settingsInput.value = '';
                if (suggestElem) suggestElem.textContent = '';
            } else if (e.key === 'ArrowUp') {
                if (commandHistory.length > 0) {
                    e.preventDefault();
                    historyIndex = commandHistory.length - 1;
                    settingsInput.value = commandHistory[historyIndex];
                }
            }
        });
    }

    // ── System clock ──
    function updateClock() {
        const now = new Date();
        let h = now.getHours();
        const m = String(now.getMinutes()).padStart(2, '0');
        const ampm = h >= 12 ? 'PM' : 'AM';
        h = h % 12 || 12;
        const clockEl = document.getElementById('system-clock');
        if (clockEl) {
            clockEl.querySelector('span').textContent = String(h).padStart(2, '0') + ':' + m;
            const mer = clockEl.querySelector('.meridian');
            if (mer) mer.textContent = ampm;
        }
    }
    updateClock();
    setInterval(updateClock, 10000);

    // ── Skip terminal if ?return ──
    if (new URLSearchParams(window.location.search).has('return')) {
        if (terminalScreen) { terminalScreen.classList.remove('active'); terminalScreen.classList.add('hidden'); }
        if (homeScreen) { homeScreen.classList.remove('hidden'); homeScreen.classList.add('active'); }
        const bunny = document.getElementById('ascii-bunny');
        if (bunny) bunny.style.display = 'block';
        window.history.replaceState(null, '', window.location.pathname);
    }
})();
