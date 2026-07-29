/**
 * liquid-text.js — Interactive fluid distortion simulation
 */
(function () {
    'use strict';

    const canvas = document.getElementById('sim-canvas');
    const wrap = document.getElementById('sim-wrap');
    const resetBtn = document.getElementById('reset-btn');
    if (!canvas || !wrap) return;

    const ctx = canvas.getContext('2d');
    let W, H;
    let particles = [];
    const PARTICLE_COUNT = 80;
    const BASE_FONT_SIZE = 14;

    function resize() {
        W = wrap.clientWidth;
        H = Math.min(window.innerHeight * 0.75, 800);
        canvas.width = W;
        canvas.height = H;
    }

    class Particle {
        constructor() {
            this.reset();
        }
        reset() {
            this.x = Math.random() * W;
            this.y = Math.random() * H;
            this.vx = (Math.random() - 0.5) * 0.5;
            this.vy = (Math.random() - 0.5) * 0.5;
            this.size = 0.5 + Math.random() * 1.5;
            this.char = String.fromCharCode(0x30A0 + Math.floor(Math.random() * 96));
            this.alpha = 0.1 + Math.random() * 0.4;
        }
        update(mouseX, mouseY) {
            const dx = this.x - mouseX;
            const dy = this.y - mouseY;
            const dist = Math.sqrt(dx * dx + dy * dy);
            if (dist < 150) {
                const force = (150 - dist) / 150;
                this.vx += (dx / dist) * force * 0.5;
                this.vy += (dy / dist) * force * 0.5;
            }
            this.vx *= 0.98;
            this.vy *= 0.98;
            this.x += this.vx;
            this.y += this.vy;
            if (this.x < 0 || this.x > W || this.y < 0 || this.y > H) this.reset();
        }
        draw(ctx) {
            ctx.save();
            ctx.globalAlpha = this.alpha;
            ctx.fillStyle = '#ff6600';
            ctx.shadowColor = '#ff6600';
            ctx.shadowBlur = 6;
            ctx.font = `${BASE_FONT_SIZE * this.size}px 'Share Tech Mono', monospace`;
            ctx.fillText(this.char, this.x, this.y);
            ctx.restore();
        }
    }

    let mouseX = -9999, mouseY = -9999;

    function init() {
        resize();
        particles = [];
        for (let i = 0; i < PARTICLE_COUNT; i++) {
            particles.push(new Particle());
        }
    }

    function animate() {
        ctx.fillStyle = 'rgba(11, 8, 17, 0.15)';
        ctx.fillRect(0, 0, W, H);
        particles.forEach(p => {
            p.update(mouseX, mouseY);
            p.draw(ctx);
        });
        requestAnimationFrame(animate);
    }

    wrap.addEventListener('mousemove', (e) => {
        const rect = canvas.getBoundingClientRect();
        mouseX = e.clientX - rect.left;
        mouseY = e.clientY - rect.top;
    });
    wrap.addEventListener('mouseleave', () => { mouseX = -9999; mouseY = -9999; });

    if (resetBtn) resetBtn.addEventListener('click', init);

    window.addEventListener('resize', init);
    init();
    animate();
})();
