/**
 * globe.js — Simple 3D globe rendered on canvas
 */
(function () {
    'use strict';

    const canvas = document.getElementById('globe-canvas');
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    let W, H;
    let rotation = 0;

    function resize() {
        W = window.innerWidth;
        H = window.innerHeight - 60;
        canvas.width = W;
        canvas.height = H;
    }

    function to3D(lat, lon, r, rot) {
        const phi = (90 - lat) * Math.PI / 180;
        const theta = (lon + rot) * Math.PI / 180;
        return {
            x: r * Math.sin(phi) * Math.cos(theta),
            y: r * Math.cos(phi),
            z: r * Math.sin(phi) * Math.sin(theta)
        };
    }

    function drawGlobe() {
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, W, H);

        const cx = W / 2;
        const cy = H / 2;
        const R = Math.min(W, H) * 0.35;

        // Draw grid lines
        ctx.strokeStyle = 'rgba(255,102,0,0.15)';
        ctx.lineWidth = 1;

        // Latitude lines
        for (let lat = -80; lat <= 80; lat += 20) {
            ctx.beginPath();
            for (let lon = 0; lon <= 360; lon += 2) {
                const p = to3D(lat, lon, R, rotation);
                const screenX = cx + p.x;
                const screenY = cy - p.y;
                if (p.z > 0) {
                    if (lon === 0) ctx.moveTo(screenX, screenY);
                    else ctx.lineTo(screenX, screenY);
                }
            }
            ctx.stroke();
        }

        // Longitude lines
        for (let lon = 0; lon < 360; lon += 30) {
            ctx.beginPath();
            for (let lat = -90; lat <= 90; lat += 2) {
                const p = to3D(lat, lon, R, rotation);
                const screenX = cx + p.x;
                const screenY = cy - p.y;
                if (p.z > 0) {
                    if (lat === -90) ctx.moveTo(screenX, screenY);
                    else ctx.lineTo(screenX, screenY);
                }
            }
            ctx.stroke();
        }

        // Glow
        const grad = ctx.createRadialGradient(cx - R * 0.3, cy - R * 0.3, 0, cx, cy, R * 1.2);
        grad.addColorStop(0, 'rgba(255,102,0,0.03)');
        grad.addColorStop(1, 'rgba(255,102,0,0)');
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(cx, cy, R * 1.2, 0, Math.PI * 2);
        ctx.fill();

        rotation += 0.3;
        requestAnimationFrame(drawGlobe);
    }

    window.addEventListener('resize', resize);
    resize();
    drawGlobe();
})();
