/**
 * ascii-cam.js — Webcam to ASCII converter
 */
(function () {
    'use strict';

    const video = document.getElementById('cam-video');
    const canvas = document.getElementById('cam-canvas');
    const output = document.getElementById('cam-output');
    const startBtn = document.getElementById('cam-start');
    if (!video || !canvas || !output || !startBtn) return;

    const ctx = canvas.getContext('2d');
    const CHARS = '@%#*+=-:. ';
    const W = 120;
    const H = 60;

    canvas.width = W;
    canvas.height = H;

    let stream = null;
    let animId = null;

    function toAscii() {
        ctx.drawImage(video, 0, 0, W, H);
        const imageData = ctx.getImageData(0, 0, W, H);
        const data = imageData.data;
        let ascii = '';
        for (let y = 0; y < H; y++) {
            for (let x = 0; x < W; x++) {
                const idx = (y * W + x) * 4;
                const r = data[idx];
                const g = data[idx + 1];
                const b = data[idx + 2];
                const brightness = (r * 0.299 + g * 0.587 + b * 0.114) / 255;
                const charIdx = Math.floor(brightness * (CHARS.length - 1));
                ascii += CHARS[charIdx];
            }
            ascii += '\n';
        }
        output.textContent = ascii;
        animId = requestAnimationFrame(toAscii);
    }

    startBtn.addEventListener('click', async () => {
        try {
            stream = await navigator.mediaDevices.getUserMedia({ video: { width: 320, height: 240 } });
            video.srcObject = stream;
            await video.play();
            startBtn.textContent = 'RUNNING...';
            startBtn.disabled = true;
            toAscii();
        } catch (err) {
            output.textContent = 'Camera access denied: ' + err.message;
        }
    });
})();
