document.addEventListener("DOMContentLoaded", () => {
    const cmdInput = document.getElementById("cmd-input");
    const terminalScreen = document.getElementById("terminal-screen");
    const homeScreen = document.getElementById("home-screen");
    if (cmdInput && terminalScreen && homeScreen) {
        // Re-focus the terminal input if the user clicks anywhere on the terminal screen
        terminalScreen.addEventListener("click", () => {
            cmdInput.focus();
        });

        // Listen for the "Enter" key on the terminal input
        cmdInput.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
            const inputValue = cmdInput.value.trim();
            
            if (inputValue.length > 0) {
                // Hide terminal screen
                terminalScreen.classList.remove("active");
                terminalScreen.classList.add("hidden");
                
                // Show home screen
                homeScreen.classList.remove("hidden");
                homeScreen.classList.add("active");
                
                // Show ASCII bunny
                const asciiBunny = document.getElementById("ascii-bunny");
                if (asciiBunny) {
                    asciiBunny.style.display = "block";
                }

                // Play a sound effect or do a transition here
            }
        }
    });
    }

    // Prevent navigation
    const disabledLinks = document.querySelectorAll(".disable-click");
    disabledLinks.forEach(link => {
        link.addEventListener("click", (e) => {
            e.preventDefault();
            console.log("ACCESS DENIED: Insufficient clearance level.");
        });
    });

    // randomize flicker further by adjusting screen opacity on irregular intervals
    function randomFlicker() {
        const screens = document.querySelectorAll('.screen');
        screens.forEach(s => {
            const current = parseFloat(getComputedStyle(s).opacity) || 1;
            let delta = (Math.random() * 0.1) - 0.05; // +/-0.05
            let val = current + delta;
            val = Math.max(0.7, Math.min(1, val));
            s.style.opacity = val;
        });
        const delay = 300 + Math.random() * 1000; // 300ms to 1.3s
        setTimeout(randomFlicker, delay);
    }
    randomFlicker();

    const settingsBtn = document.getElementById("settings-btn");
    const settingsScreen = document.getElementById("settings-screen");
    const closeSettingsBtn = document.getElementById("close-settings");

    if (settingsBtn && settingsScreen) {
        settingsBtn.addEventListener("click", () => {
            // show settings, hide home
            homeScreen.classList.add("hidden");
            homeScreen.classList.remove("active");
            settingsScreen.classList.remove("hidden");
            settingsScreen.classList.add("active");
            
            const asciiBunny = document.getElementById("ascii-bunny");
            if (asciiBunny) {
                asciiBunny.style.display = "none";
            }
        });
    }

    if (closeSettingsBtn && settingsScreen) {
        closeSettingsBtn.addEventListener("click", () => {
            settingsScreen.classList.add("hidden");
            settingsScreen.classList.remove("active");
            homeScreen.classList.remove("hidden");
            homeScreen.classList.add("active");
            
            const asciiBunny = document.getElementById("ascii-bunny");
            if (asciiBunny) {
                asciiBunny.style.display = "block";
            }
        });
    }

    const settingsInput = document.getElementById("settings-input");
    const settingsLog = document.getElementById("settings-log");

    // If arriving from another page via ?return, skip the terminal
    if (new URLSearchParams(window.location.search).has('return')) {
        if (terminalScreen) { terminalScreen.classList.remove('active'); terminalScreen.classList.add('hidden'); }
        if (homeScreen)     { homeScreen.classList.remove('hidden');    homeScreen.classList.add('active'); }
        const asciiBunny = document.getElementById('ascii-bunny');
        if (asciiBunny) asciiBunny.style.display = 'block';
        window.history.replaceState(null, '', window.location.pathname);
    }

    // load persisted theme values from localStorage (if any)
    const storedText = localStorage.getItem('theme-text');
    const storedUi = localStorage.getItem('theme-ui');
    const storedClock = localStorage.getItem('theme-clock');
    const storedBg = localStorage.getItem('theme-bg');
    if (storedText) document.documentElement.style.setProperty('--theme-text', storedText);
    if (storedUi) document.documentElement.style.setProperty('--theme-ui', storedUi);
    if (storedClock) document.documentElement.style.setProperty('--theme-clock', storedClock);
    if (storedBg) document.documentElement.style.setProperty('--theme-bg', storedBg);
    updateChromaticAberration(storedText || storedUi || 'var(--magi-orange)');
    function rgbToHsl(r, g, b) {
        r /= 255; g /= 255; b /= 255;
        const max = Math.max(r, g, b), min = Math.min(r, g, b);
        let h = 0, s = 0;
        const l = (max + min) / 2;
        if (max !== min) {
            const d = max - min;
            s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
            switch (max) {
                case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
                case g: h = ((b - r) / d + 2) / 6; break;
                case b: h = ((r - g) / d + 4) / 6; break;
            }
        }
        return [h * 360, s, l];
    }

    function hslToRgb(h, s, l) {
        h /= 360;
        if (s === 0) {
            const v = Math.round(l * 255);
            return [v, v, v];
        }
        const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
        const p = 2 * l - q;
        const hue2rgb = (p, q, t) => {
            if (t < 0) t += 1;
            if (t > 1) t -= 1;
            if (t < 1 / 6) return p + (q - p) * 6 * t;
            if (t < 1 / 2) return q;
            if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
            return p;
        };
        return [
            Math.round(hue2rgb(p, q, h + 1 / 3) * 255),
            Math.round(hue2rgb(p, q, h) * 255),
            Math.round(hue2rgb(p, q, h - 1 / 3) * 255)
        ];
    }

    function updateChromaticAberration(colorStr) {
        // Resolve CSS variables / named colors to an actual rgb() value
        const tmp = document.createElement('div');
        tmp.style.cssText = 'position:absolute;visibility:hidden;pointer-events:none;color:' + colorStr;
        document.documentElement.appendChild(tmp);
        const resolved = getComputedStyle(tmp).color;
        document.documentElement.removeChild(tmp);
        const m = resolved.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
        if (!m) return;
        const [h, s, l] = rgbToHsl(+m[1], +m[2], +m[3]);
        // Keep saturation strong enough to be visible, lightness in readable range
        const sat = Math.max(s, 0.7);
        const lit = Math.min(Math.max(l, 0.4), 0.65);
        const warm = hslToRgb(((h - 30) + 360) % 360, sat, lit);
        const cool = hslToRgb((h + 180) % 360, sat, lit);
        document.documentElement.style.setProperty('--aberr-warm', `rgba(${warm.join(',')},0.6)`);
        document.documentElement.style.setProperty('--aberr-cool', `rgba(${cool.join(',')},0.6)`);
    }

    const availableCommands = {
        'theme': ['ui', 'text', 'clock', 'background', 'reset'],
        'volume': [],
        'brightness': [],
        'network': [],
        'reset': [],
        'help': []
    };

    const colorAliases = {
        'magi-orange': '#ff6600',
        'bg-color': '#0b0811'
    };
    const colorOptions = [
        'magi-orange',
        'bg-color',
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

    function autocompleteSettings(input) {
        if (!input) return '';
        const parts = input.split(' ');
        
        if (parts.length === 1) {
            const match = Object.keys(availableCommands).find(cmd => cmd.startsWith(parts[0]));
            return match || '';
        } else if (parts.length === 2) {
            const baseCmd = parts[0];
            if (availableCommands[baseCmd] && availableCommands[baseCmd].length > 0) {
                if (parts[1] === '') {
                    return baseCmd + ' ' + availableCommands[baseCmd].join(' | ');
                } else {
                    const subMatch = availableCommands[baseCmd].find(sub => sub.startsWith(parts[1]));
                    if (subMatch) {
                        if (baseCmd === 'theme' && ['ui','text','clock','background'].includes(subMatch) && parts[1] === subMatch) {
                            return baseCmd + ' ' + subMatch + ' ' + colorOptions.join(' | ');
                        }
                        return baseCmd + ' ' + subMatch;
                    }
                }
            }
        } else if (parts.length === 3) {
            const baseCmd = parts[0];
            const sub = parts[1];
            if (baseCmd === 'theme' && ['ui', 'text', 'clock', 'background'].includes(sub) && colorOptions.length > 0) {
                if (parts[2] === '') {
                    return baseCmd + ' ' + sub + ' ' + colorOptions.join(' | ');
                } else {
                    const colorMatch = colorOptions.find(c => c.startsWith(parts[2]));
                    if (colorMatch) {
                        return baseCmd + ' ' + sub + ' ' + colorMatch;
                    }
                }
            }
        }
        return '';
    }

    if (settingsInput) {
        const suggestElem = document.getElementById('settings-suggest');
        const promptElem = document.querySelector('#settings-screen .prompt');

        // recalc suggestion position whenever needed
        function updateSuggestionPosition() {
            if (!suggestElem || !settingsInput) return;
            suggestElem.style.left = settingsInput.offsetLeft + 'px';
        }

        updateSuggestionPosition();
        window.addEventListener('resize', updateSuggestionPosition);

        settingsInput.addEventListener('input', () => {
            const val = settingsInput.value;
            const full = autocompleteSettings(val);
            if (full && full !== val) {
                suggestElem.textContent = full.slice(val.length);
            } else {
                suggestElem.textContent = '';
            }
            const textWidth = measureTextWidth(val, settingsInput);
            suggestElem.style.left = (settingsInput.offsetLeft + textWidth) + 'px';
        });

        function measureTextWidth(text, referenceElem) {
            const canvas = measureTextWidth.canvas || (measureTextWidth.canvas = document.createElement('canvas'));
            const ctx = canvas.getContext('2d');
            const style = window.getComputedStyle(referenceElem);
            ctx.font = style.fontSize + ' ' + style.fontFamily;
            return ctx.measureText(text).width;
        }

        function arrayEquals(a, b) {
            if (a.length !== b.length) return false;
            for (let i = 0; i < a.length; i++) {
                if (a[i] !== b[i]) return false;
            }
            return true;
        }

        let cycleState = { lastInput: '', completions: [], index: -1 };
        const commandHistory = [];
        let historyIndex = -1;

        settingsInput.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                const val = settingsInput.value;
                const parts = val.split(' ');

                function allCompletions(input) {
                    const parts = input.split(' ');
                    if (parts.length === 1) {
                        return Object.keys(availableCommands).filter(cmd => cmd.startsWith(parts[0]));
                    } else if (parts.length === 2) {
                        const base = parts[0];
                        if (availableCommands[base]) {
                            if (parts[1] === '') return availableCommands[base];
                            return availableCommands[base].filter(sub => sub.startsWith(parts[1]));
                        }
                    } else if (parts.length === 3 && parts[0] === 'theme') {
                        const sub = parts[1];
                        if (sub === 'reset') return [];
                        if (parts[2] === '') return colorOptions;
                        return colorOptions.filter(c => c.startsWith(parts[2]));
                    }
                    return [];
                }

                const completions = allCompletions(val);
                if (completions.length === 0) return;

                const base = parts.slice(0, parts.length - 1).join(' ');

                // special-case single remaining completion: insert it immediately
                if (completions.length === 1) {
                    const next = completions[0];
                    settingsInput.value = base ? base + ' ' + next : next;
                    settingsInput.dispatchEvent(new Event('input'));
                    cycleState.lastInput = '';
                    if (suggestElem) suggestElem.textContent = '';
                    return;
                }

                if (cycleState.lastInput !== val) {
                    cycleState.lastInput = val;
                    cycleState.completions = completions;
                    cycleState.index = -1;
                }

                cycleState.index = (cycleState.index + 1) % completions.length;
                const next = completions[cycleState.index];

                if (suggestElem) {
                    suggestElem.innerHTML = completions.map((c,i) => i === cycleState.index ? `<span class="current">${c}</span>` : c).join(' | ');
                }
            } else if (e.key === ' ') {
                if (cycleState.index >= 0 && cycleState.completions.length > 0) {
                    e.preventDefault();
                    const parts = settingsInput.value.split(' ');
                    const base = parts.slice(0, parts.length - 1).join(' ');
                    const sel = cycleState.completions[cycleState.index];
                    settingsInput.value = (base ? base + ' ' : '') + sel + ' ';
                    settingsInput.dispatchEvent(new Event('input'));
                    cycleState.index = -1;
                }
            } else if (e.key === 'ArrowUp') {
                if (commandHistory.length > 0) {
                    e.preventDefault();
                    historyIndex = commandHistory.length - 1;
                    settingsInput.value = commandHistory[historyIndex];
                    settingsInput.dispatchEvent(new Event('input'));
                }
            } else if (e.key === 'Enter') {
                e.preventDefault();
                const cmd = settingsInput.value.trim();
                if (cmd.length) {
                    logSettingsEntry('> ' + cmd);
                    commandHistory.push(cmd);
                    historyIndex = -1;
                    const parts = cmd.split(' ');
                    const baseCmd = parts[0].toLowerCase();

                    if (Object.keys(availableCommands).includes(baseCmd)) {
                        if (baseCmd === 'theme') {
                            const target = parts[1] ? parts[1].toLowerCase() : null;
                            if (target === 'reset') {
                                document.documentElement.style.setProperty('--theme-text', 'var(--magi-orange)');
                                document.documentElement.style.setProperty('--theme-ui', 'var(--magi-orange)');
                                document.documentElement.style.setProperty('--theme-clock', 'var(--magi-orange)');
                                document.documentElement.style.setProperty('--theme-bg', 'var(--bg-color)');
                                localStorage.removeItem('theme-text');
                                localStorage.removeItem('theme-ui');
                                localStorage.removeItem('theme-clock');
                                localStorage.removeItem('theme-bg');
                                updateChromaticAberration('var(--magi-orange)');
                                logSettingsEntry('Theme reset to defaults');
                            } else {
                                const color = parts[2];
                                if (target && color) {
                                    const realColor = colorAliases[color] || color;
                                    if (target === 'ui') {
                                        document.documentElement.style.setProperty('--theme-ui', realColor);
                                        document.documentElement.style.setProperty('--theme-text', realColor);
                                        document.documentElement.style.setProperty('--theme-clock', realColor);
                                        localStorage.setItem('theme-ui', realColor);
                                        localStorage.setItem('theme-text', realColor);
                                        localStorage.setItem('theme-clock', realColor);
                                        updateChromaticAberration(realColor);
                                        logSettingsEntry('Theme updated: UI, text and clock to ' + color);
                                    } else if (target === 'text') {
                                        document.documentElement.style.setProperty('--theme-text', realColor);
                                        localStorage.setItem('theme-text', realColor);
                                        updateChromaticAberration(realColor);
                                        logSettingsEntry('Theme updated: text to ' + color);
                                    } else if (target === 'clock') {
                                        document.documentElement.style.setProperty('--theme-clock', realColor);
                                        localStorage.setItem('theme-clock', realColor);
                                        logSettingsEntry('Theme updated: clock to ' + color);
                                    } else if (target === 'background') {
                                        document.documentElement.style.setProperty('--theme-bg', realColor);
                                        localStorage.setItem('theme-bg', realColor);
                                        logSettingsEntry('Theme updated: background to ' + color);
                                    } else {
                                        logSettingsEntry('Invalid target. Use UI, text, clock, background, or reset.');
                                    }
                                } else {
                                    logSettingsEntry('Usage: theme <target> <color> (or theme reset)');
                                }
                            }
                        } else if (baseCmd === 'reset') {
                            // restore defaults defined in CSS root
                            document.documentElement.style.setProperty('--theme-text', 'var(--magi-orange)');
                            document.documentElement.style.setProperty('--theme-ui', 'var(--magi-orange)');
                            document.documentElement.style.setProperty('--theme-clock', 'var(--magi-orange)');
                            document.documentElement.style.setProperty('--theme-bg', 'var(--bg-color)');
                            // clear persisted values
                            localStorage.removeItem('theme-text');
                            localStorage.removeItem('theme-ui');
                            localStorage.removeItem('theme-clock');
                            localStorage.removeItem('theme-bg');
                            updateChromaticAberration('var(--magi-orange)');
                            logSettingsEntry('Theme reset to defaults');
                        } else {
                            logSettingsEntry(baseCmd + ' executed');
                        }
                    } else {
                        logSettingsEntry('Unknown command');
                    }
                }
                settingsInput.value = '';
                suggestElem.textContent = '';
            }
        });
    }

    // Clock update function
    const clockTime = document.querySelector("#system-clock span:first-child");
    const clockMeridian = document.querySelector(".meridian");

    function updateClock() {
        if (!clockTime || !clockMeridian) return;
        const now = new Date();
        let hours = now.getHours();
        const minutes = now.getMinutes().toString().padStart(2, '0');
        const ampm = hours >= 12 ? 'PM' : 'AM';
        
        hours = hours % 12;
        hours = hours ? hours : 12; 
        const hoursStr = hours.toString().padStart(2, '0');
        
        clockTime.textContent = `${hoursStr}:${minutes}`;
        clockMeridian.textContent = ampm;
    }
    
    updateClock();
    setInterval(updateClock, 1000);
});