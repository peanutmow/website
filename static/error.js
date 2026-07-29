/**
 * error.js — Synchro error display with glitch text
 */
(function () {
    'use strict';

    const glitchEl = document.getElementById('glitch-text');
    if (!glitchEl) return;

    const lines = [
        "WARNING: SYNCHRONIZATION RATIO CRITICAL",
        "AT FIELD INSTABILITY DETECTED",
        "UNIT-01 CORE TEMPERATURE: 98.4°C",
        "LCL O2 LEVELS: 12% AND DROPPING",
        "PILOT NEURAL HANDSHAKE: UNSTABLE",
        "SYSTEM: ENGAGING EMERGENCY PROTOCOL 7-G",
        "",
        "01001010 11001101 00101110 10010011",
        "ERROR 0x7F: MEMORY SECTOR 4-G CORRUPT",
        "RE-ROUTING NEURAL BRIDGE... FAILED",
        "ATTEMPTING HARD REBOOT...",
        "",
        "DO NOT DISCONNECT THE PILOT",
        "THIS IS NOT A DRILL",
    ];

    let visibleLines = 0;
    let charIdx = 0;
    const speed = 50;

    function typeLine() {
        if (visibleLines >= lines.length) {
            setTimeout(() => {
                visibleLines = 0;
                charIdx = 0;
                glitchEl.textContent = '';
                typeLine();
            }, 3000);
            return;
        }

        const line = lines[visibleLines];
        if (charIdx < line.length) {
            charIdx++;
            const display = lines.slice(0, visibleLines).join('\n') + '\n' + line.slice(0, charIdx);
            glitchEl.textContent = display;
            setTimeout(typeLine, 20 + Math.random() * 40);
        } else {
            visibleLines++;
            charIdx = 0;
            setTimeout(typeLine, 100 + Math.random() * 200);
        }
    }

    typeLine();
})();
