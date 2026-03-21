

(function () {
  const POOLS = [
    [
      "SEELE node confirmed.",
      "cross-ref EVA-00 pending",
      "sync ratio: 84.2%",
      "pilot status: nominal",
      "SEELE node confirmed.",
      "awaiting sign-off from DR. A",
      "unit-01 containment: breach?",
    ],
    [
      "01001010 11001 ERROR",
      "file corrupted — retrying",
      "01110101 00110",
      "sector 7-G inaccessible",
      "01001010 checksum fail",
      "re-routing thru node 4",
      "11010110 00101 OK",
    ],
    [
      "i think the pilot is—",
      "no wait that doesnt—",
      "ok so if the angel—",
      "she was there before—",
      "never mind forget it",
      "the third one. its—",
      "does anyone else see—",
    ],
    [
      "LAT 35.6762 LON 139.6503",
      "depth: 920m below ref",
      "temp delta: +4.1 cel",
      "LCL pressure nominal",
      "LAT 35.6762 anomaly",
      "signal lost at 03:44",
      "re-acquire: failed",
    ],
    [
      "GEHIRN record #0044-B",
      "access denied — lvl 3 req",
      "SEELE brief //SI-ZERO",
      "PROJECT A.P.W. — classified",
      "dummy sys engaged",
      "NERV internal log 2015",
      "scenario pending approval",
    ],
    [
      "she said she'd come back",
      "unit activation without auth",
      "AT field: full expansion",
      "this isnt what he wanted",
      "pattern: BLUE. confirmed.",
      "nobody told the pilots",
      "the end of eva. what was—",
    ],
  ];

  function getTextNodes() {
    const nodes = [];
    document.querySelectorAll('.archive-column p:not(.data-block)').forEach(p => {
      const walker = document.createTreeWalker(p, NodeFilter.SHOW_TEXT);
      let n;
      while ((n = walker.nextNode())) {
        if (n.textContent.trim().length > 8) nodes.push(n);
      }
    });
    return nodes;
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

      this.typeMin       = opts.typeMin       || 18;
      this.typeMax       = opts.typeMax       || 80;
      this.delMin        = opts.delMin        || 8;
      this.delMax        = opts.delMax        || 30;
      this.mistakeChance = opts.mistakeChance || 0.12;
      this.burstChance   = opts.burstChance   || 0.10;
      this.pauseChance   = opts.pauseChance   || 0.06;

      // DOM anchors — set on each mount
      this.hostParent  = null;
      this.beforeNode  = null;
      this.afterNode   = null;
      this.span        = null;
      this.spanText    = null;
      this.cursor      = null;

      setTimeout(() => this._mount(), Math.random() * 3000);
    }

    _rand(min, max) { return Math.random() * (max - min) + min; }

    _mount() {
      const nodes = getTextNodes();
      if (!nodes.length) { setTimeout(() => this._mount(), 500); return; }

      const hostNode = nodes[Math.floor(Math.random() * nodes.length)];
      const text     = hostNode.textContent;

      const lo  = Math.floor(text.length * 0.1);
      const hi  = Math.floor(text.length * 0.9);
      const pos = lo + Math.floor(Math.random() * (hi - lo + 1));

      this.beforeNode = document.createTextNode(text.slice(0, pos));
      this.afterNode  = document.createTextNode(text.slice(pos));

      this.span = document.createElement('span');
      this.span.className = 'ghost-field';
      this.spanText = document.createTextNode('');
      this.cursor   = document.createElement('span');
      this.cursor.className = 'ghost-cursor';
      this.span.appendChild(this.spanText);
      this.span.appendChild(this.cursor);

      const parent = hostNode.parentNode;
      parent.insertBefore(this.beforeNode, hostNode);
      parent.insertBefore(this.span,       hostNode);
      parent.insertBefore(this.afterNode,  hostNode);
      parent.removeChild(hostNode);

      this.hostParent = parent;
      this.current = '';
      this.pos     = 0;
      this.deleting = false;
      this._nextTarget();
      this._tick();
    }

    _unmount() {
      if (!this.span || !this.span.parentNode) return;
      const parent = this.span.parentNode;
      const merged = document.createTextNode(
        (this.beforeNode ? this.beforeNode.textContent : '') +
        (this.afterNode  ? this.afterNode.textContent  : '')
      );
      parent.insertBefore(merged, this.beforeNode || this.span);
      if (this.beforeNode && this.beforeNode.parentNode) parent.removeChild(this.beforeNode);
      if (this.span       && this.span.parentNode)       parent.removeChild(this.span);
      if (this.afterNode  && this.afterNode.parentNode)  parent.removeChild(this.afterNode);
      this.hostParent = this.beforeNode = this.afterNode = this.span = this.spanText = this.cursor = null;
    }

    _render() { if (this.spanText) this.spanText.textContent = this.current; }

    _nextTarget() {
      this.poolIdx = (this.poolIdx + 1) % this.pool.length;
      if (Math.random() < 0.3) this.poolIdx = Math.floor(Math.random() * this.pool.length);
      this.target = this.pool[this.poolIdx];
    }

    _tick() { this.deleting ? this._doDelete() : this._doType(); }

    _doType() {
      if (this.pos >= this.target.length) {
        setTimeout(() => { this.deleting = true; this._tick(); }, this._rand(120, 500));
        return;
      }
      if (Math.random() < this.pauseChance) {
        setTimeout(() => this._doType(), this._rand(180, 650));
        return;
      }
      if (Math.random() < this.mistakeChance && this.pos > 0) {
        const w = 'qwertyuiopasdfghjklzxcvbnm!@#$%?';
        this.current += w[Math.floor(Math.random() * w.length)];
        this._render();
        setTimeout(() => {
          this.current = this.current.slice(0, -1);
          this._render();
          setTimeout(() => this._doType(), this._rand(25, 70));
        }, this._rand(70, 220));
        return;
      }
      if (Math.random() < this.burstChance && this.pos < this.target.length - 2) {
        const burst = Math.floor(this._rand(2, 6));
        let i = 0;
        const go = () => {
          if (i < burst && this.pos < this.target.length) {
            this.current += this.target[this.pos++];
            this._render(); i++;
            setTimeout(go, this._rand(8, 24));
          } else { setTimeout(() => this._doType(), this._rand(20, 65)); }
        };
        go(); return;
      }
      this.current += this.target[this.pos++];
      this._render();
      setTimeout(() => this._doType(), this._rand(this.typeMin, this.typeMax));
    }

    _doDelete() {
      if (this.current.length === 0) {
        setTimeout(() => {
          this._unmount();
          setTimeout(() => this._mount(), this._rand(80, 400));
        }, this._rand(80, 400));
        return;
      }
      if (Math.random() < 0.15 && this.current.length > 4) {
        const burst = Math.floor(this._rand(2, 5));
        let i = 0;
        const go = () => {
          if (i < burst && this.current.length > 0) {
            this.current = this.current.slice(0, -1);
            this._render(); i++;
            setTimeout(go, this._rand(7, 22));
          } else { setTimeout(() => this._doDelete(), this._rand(this.delMin, this.delMax)); }
        };
        go(); return;
      }
      this.current = this.current.slice(0, -1);
      this._render();
      setTimeout(() => this._doDelete(), this._rand(this.delMin, this.delMax));
    }
  }

  // Spawn 16 independent ghosts, each with its own persona + pool
  const NUM_GHOSTS = 16;
  for (let i = 0; i < NUM_GHOSTS; i++) {
    new GhostTypist(POOLS[i % POOLS.length], personas[i % personas.length]);
  }
})();
