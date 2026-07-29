/**
 * ghost-typist.js — Typewriter effect for the portfolio archive text
 * Cycles through different text pools with various typing personas
 */

(function () {
  'use strict';

  const POOLS = [
    ["SEELE node confirmed.", "cross-ref EVA-00 pending", "sync ratio: 84.2%", "pilot status: nominal", "awaiting sign-off from DR. A", "unit-01 containment: breach?"],
    ["01001010 11001 ERROR", "file corrupted — retrying", "01110101 00110", "sector 7-G inaccessible", "checksum fail", "re-routing thru node 4"],
    ["i think the pilot is—", "no wait that doesnt—", "ok so if the angel—", "she was there before—", "never mind forget it", "the third one. its—"],
    ["LAT 35.6762 LON 139.6503", "depth: 920m below ref", "temp delta: +4.1 cel", "LCL pressure nominal", "signal lost at 03:44", "re-acquire: failed"],
    ["GEHIRN record #0044-B", "access denied — lvl 3 req", "SEELE brief //SI-ZERO", "PROJECT A.P.W. — classified", "dummy sys engaged", "NERV internal log 2015"],
    ["she said she'd come back", "unit activation without auth", "AT field: full expansion", "this isnt what he wanted", "pattern: BLUE. confirmed.", "the end of eva. what was—"],
  ];

  function getParagraphs() {
    return Array.from(document.querySelectorAll('.archive-column p:not(.data-block)'));
  }

  const personas = [
    { typeMin: 60, typeMax: 180, delMin: 25, delMax: 80,  mistakeChance: 0.08, burstChance: 0.08, pauseChance: 0.04 },
    { typeMin: 20, typeMax: 90,  delMin: 10, delMax: 35,  mistakeChance: 0.04, burstChance: 0.25, pauseChance: 0.02 },
    { typeMin: 80, typeMax: 220, delMin: 40, delMax: 110, mistakeChance: 0.18, burstChance: 0.05, pauseChance: 0.10 },
    { typeMin: 35, typeMax: 110, delMin: 15, delMax: 55,  mistakeChance: 0.12, burstChance: 0.14, pauseChance: 0.06 },
    { typeMin: 50, typeMax: 160, delMin: 20, delMax: 65,  mistakeChance: 0.10, burstChance: 0.10, pauseChance: 0.05 },
    { typeMin: 90, typeMax: 250, delMin: 30, delMax: 90,  mistakeChance: 0.20, burstChance: 0.04, pauseChance: 0.12 },
    { typeMin: 15, typeMax: 60,  delMin: 8,  delMax: 28,  mistakeChance: 0.03, burstChance: 0.35, pauseChance: 0.01 },
    { typeMin: 70, typeMax: 200, delMin: 35, delMax: 100, mistakeChance: 0.15, burstChance: 0.06, pauseChance: 0.08 },
  ];

  class GhostTypist {
    constructor(pool, opts = {}) {
      this.pool = pool;
      this.poolIdx = Math.floor(Math.random() * pool.length);
      this.target = pool[this.poolIdx];
      this.current = '';
      this.pos = 0;
      this.deleting = false;
      this.typeMin = opts.typeMin || 18;
      this.typeMax = opts.typeMax || 80;
      this.delMin = opts.delMin || 8;
      this.delMax = opts.delMax || 30;
      this.mistakeChance = opts.mistakeChance || 0.12;
      this.burstChance = opts.burstChance || 0.10;
      this.pauseChance = opts.pauseChance || 0.06;
      this.hostParent = null;
      this.beforeNode = null;
      this.afterNode = null;
      this.span = null;
      this.cursorSpan = null;
      this.isMounted = false;
      this.isDestroyed = false;
      this.paused = false;
      this.timeoutId = null;
    }

    setPool(pool) {
      this.pool = pool;
      this.poolIdx = Math.floor(Math.random() * pool.length);
      this.target = pool[this.poolIdx];
    }

    mount(hostParent, beforeNode, afterNode) {
      if (this.isMounted) return;
      this.hostParent = hostParent;
      this.beforeNode = beforeNode;
      this.afterNode = afterNode;
      this.span = document.createElement('span');
      this.span.className = 'ghost-field';
      this.cursorSpan = document.createElement('span');
      this.cursorSpan.className = 'ghost-cursor';
      this.hostParent.insertBefore(this.span, this.afterNode);
      this.hostParent.insertBefore(this.cursorSpan, this.afterNode);
      this.isMounted = true;
      this.scheduleNext();
    }

    unmount() {
      this.isMounted = false;
      if (this.timeoutId) { clearTimeout(this.timeoutId); this.timeoutId = null; }
      if (this.span && this.span.parentNode) this.span.parentNode.removeChild(this.span);
      if (this.cursorSpan && this.cursorSpan.parentNode) this.cursorSpan.parentNode.removeChild(this.cursorSpan);
      this.span = null;
      this.cursorSpan = null;
    }

    destroy() {
      this.isDestroyed = true;
      this.unmount();
    }

    scheduleNext() {
      if (this.isDestroyed || !this.isMounted) return;
      if (this.paused) { this.timeoutId = setTimeout(() => this.scheduleNext(), 200); return; }

      if (!this.deleting) {
        // Typing
        const isMistake = Math.random() < this.mistakeChance && this.pos > 0;
        const charsToAdd = Math.random() < this.burstChance ? 2 + Math.floor(Math.random() * 5) : 1;
        if (isMistake) {
          const wrongChar = String.fromCharCode(32 + Math.floor(Math.random() * 95));
          this.current = this.target.slice(0, this.pos) + wrongChar;
          this.span.textContent = this.current;
          this.timeoutId = setTimeout(() => {
            // Delete the mistake + 1-3 extra chars
            const deleteCount = 2 + Math.floor(Math.random() * 3);
            const newLen = Math.max(0, this.current.length - deleteCount);
            this.current = this.current.slice(0, newLen);
            this.pos = Math.min(this.pos, newLen);
            this.span.textContent = this.current;
            this.timeoutId = setTimeout(() => this.scheduleNext(), 30 + Math.random() * 80);
          }, 40 + Math.random() * 100);
          return;
        }

        for (let i = 0; i < charsToAdd && this.pos < this.target.length; i++) {
          this.current += this.target[this.pos];
          this.pos++;
        }

        if (this.pos >= this.target.length) {
          this.deleting = true;
          const pause = 1000 + Math.random() * 3000;
          this.span.textContent = this.current;
          this.timeoutId = setTimeout(() => this.scheduleNext(), pause);
        } else {
          this.span.textContent = this.current;
          const delay = this.typeMin + Math.random() * (this.typeMax - this.typeMin);
          this.timeoutId = setTimeout(() => this.scheduleNext(), delay);
        }
      } else {
        // Deleting
        if (this.current.length <= 0) {
          this.deleting = false;
          this.pos = 0;
          // Pick new target
          const poolLen = this.pool.length;
          this.poolIdx = (this.poolIdx + 1) % poolLen;
          this.target = this.pool[this.poolIdx];
          this.current = '';
          const delay = 200 + Math.random() * 600;
          this.timeoutId = setTimeout(() => this.scheduleNext(), delay);
        } else {
          this.current = this.current.slice(0, -1);
          this.span.textContent = this.current;
          const delay = this.delMin + Math.random() * (this.delMax - this.delMin);
          this.timeoutId = setTimeout(() => this.scheduleNext(), delay);
        }
      }
    }
  }

  // ── Initialize ──
  function init() {
    const paragraphs = getParagraphs();
    if (paragraphs.length === 0) return;

    const typists = [];

    paragraphs.forEach((p, i) => {
      const personaIdx = i % personas.length;
      const persona = personas[personaIdx];
      const poolIdx = i % POOLS.length;
      const typist = new GhostTypist(POOLS[poolIdx], persona);
      typist.mount(p, null, null);
      typists.push(typist);
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
