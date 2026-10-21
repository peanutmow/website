/**
 * bunny.js — the rotating bunny, rasterised into the tour's character grid.
 *
 * A small software renderer, after peanutmow/ascii-render: it turns the mesh
 * from bunny-mesh.js, projects it orthographically, and shades every covered
 * cell with the same character ramp the water uses. No canvas, no pixels — the
 * glyph is the pixel.
 */
(function () {
    'use strict';

    const mesh = window.BUNNY_MESH;
    if (!mesh) return;

    const V = mesh.v;
    const N = mesh.n;
    const F = mesh.f;
    const NF = mesh.faces;

    function unit(x, y, z) {
        const l = Math.sqrt(x * x + y * y + z * z) || 1;
        return [x / l, y / l, z / l];
    }

    // One key light, two weak fills. The lights stay put while the model turns,
    // so the shading is what travels across the surface.
    const K = unit(0.6, 0.8, 0.5);
    const A = unit(-0.5, 0.3, 0.7);
    const B = unit(0, -0.5, -0.8);

    let cols = 0;
    let rows = 0;
    let zbuf = new Float32Array(0);
    let nbuf = new Float32Array(0);

    function resize(c, r) {
        if (c === cols && r === rows) return;
        cols = c;
        rows = r;
        zbuf = new Float32Array(cols * rows);
        nbuf = new Float32Array(cols * rows * 3);
    }

    /**
     * Rasterise the mesh at one angle.
     *
     * `cols` and `rows` are counts of character cells: the model's unit cube
     * fills them, so the caller sizes them to the cell aspect to keep the bunny
     * round. Returns one string per row.
     */
    function render(opts) {
        const c = Math.max(6, opts.cols | 0);
        const r = Math.max(6, opts.rows | 0);
        const ramp = opts.ramp || ' .-:~,^;*=|>+\\/?&$%#@';
        const last = ramp.length - 1;
        const invert = opts.invert !== false;
        resize(c, r);

        zbuf.fill(-Infinity);

        const rx = opts.angleX || 0;
        const ry = opts.angleY || 0;
        const sa = Math.sin(rx), ca = Math.cos(rx);
        const sb = Math.sin(ry), cb = Math.cos(ry);
        // rotX * rotY, row-major
        const m0 = cb, m1 = 0, m2 = sb;
        const m3 = sa * sb, m4 = ca, m5 = -sa * cb;
        const m6 = -ca * sb, m7 = sa, m8 = ca * cb;

        const W1 = c - 1;
        const H1 = r - 1;
        // A cell of margin on each side keeps the silhouette off the edge.
        const SX = W1 / (c - 2);
        const SY = H1 / (r - 2);

        for (let f = 0; f < NF; f++) {
            const ai = F[f * 3], bi = F[f * 3 + 1], ci = F[f * 3 + 2];

            const va0 = V[ai * 3], va1 = V[ai * 3 + 1], va2 = V[ai * 3 + 2];
            const vb0 = V[bi * 3], vb1 = V[bi * 3 + 1], vb2 = V[bi * 3 + 2];
            const vc0 = V[ci * 3], vc1 = V[ci * 3 + 1], vc2 = V[ci * 3 + 2];

            // Vertex positions are thousandths; the scale is folded into the
            // projection rather than applied to every component here.
            const ax = (m0 * va0 + m1 * va1 + m2 * va2) * 0.001;
            const ay = (m3 * va0 + m4 * va1 + m5 * va2) * 0.001;
            const az = (m6 * va0 + m7 * va1 + m8 * va2) * 0.001;
            const bx = (m0 * vb0 + m1 * vb1 + m2 * vb2) * 0.001;
            const by = (m3 * vb0 + m4 * vb1 + m5 * vb2) * 0.001;
            const bz = (m6 * vb0 + m7 * vb1 + m8 * vb2) * 0.001;
            const cx = (m0 * vc0 + m1 * vc1 + m2 * vc2) * 0.001;
            const cy = (m3 * vc0 + m4 * vc1 + m5 * vc2) * 0.001;
            const cz = (m6 * vc0 + m7 * vc1 + m8 * vc2) * 0.001;

            // Orthographic, screen Y down. Negative area is the front face:
            // flipping Y reverses the winding.
            const pax = (ax / SX + 1) * 0.5 * W1;
            const pay = (1 - ay / SY) * 0.5 * H1;
            const pbx = (bx / SX + 1) * 0.5 * W1;
            const pby = (1 - by / SY) * 0.5 * H1;
            const pcx = (cx / SX + 1) * 0.5 * W1;
            const pcy = (1 - cy / SY) * 0.5 * H1;

            const area = (pbx - pax) * (pcy - pay) - (pby - pay) * (pcx - pax);
            if (area >= 0) continue;

            const x0 = Math.max(0, Math.floor(Math.min(pax, pbx, pcx)));
            const x1 = Math.min(W1, Math.ceil(Math.max(pax, pbx, pcx)));
            const y0 = Math.max(0, Math.floor(Math.min(pay, pby, pcy)));
            const y1 = Math.min(H1, Math.ceil(Math.max(pay, pby, pcy)));
            if (x0 > x1 || y0 > y1) continue;

            const na0 = N[ai * 3], na1 = N[ai * 3 + 1], na2 = N[ai * 3 + 2];
            const nb0 = N[bi * 3], nb1 = N[bi * 3 + 1], nb2 = N[bi * 3 + 2];
            const nc0 = N[ci * 3], nc1 = N[ci * 3 + 1], nc2 = N[ci * 3 + 2];

            const nax = m0 * na0 + m1 * na1 + m2 * na2;
            const nay = m3 * na0 + m4 * na1 + m5 * na2;
            const naz = m6 * na0 + m7 * na1 + m8 * na2;
            const nbx = m0 * nb0 + m1 * nb1 + m2 * nb2;
            const nby = m3 * nb0 + m4 * nb1 + m5 * nb2;
            const nbz = m6 * nb0 + m7 * nb1 + m8 * nb2;
            const ncx = m0 * nc0 + m1 * nc1 + m2 * nc2;
            const ncy = m3 * nc0 + m4 * nc1 + m5 * nc2;
            const ncz = m6 * nc0 + m7 * nc1 + m8 * nc2;

            const inv = 1 / area;
            for (let py = y0; py <= y1; py++) {
                for (let px = x0; px <= x1; px++) {
                    const w0 = (pcx - pbx) * (py - pby) - (pcy - pby) * (px - pbx);
                    const w1 = (pax - pcx) * (py - pcy) - (pay - pcy) * (px - pcx);
                    const w2 = (pbx - pax) * (py - pay) - (pby - pay) * (px - pax);
                    if (w0 > 0 || w1 > 0 || w2 > 0) continue;

                    const ba = w0 * inv, bb = w1 * inv, bc = w2 * inv;
                    const z = az * ba + bz * bb + cz * bc;
                    const p = py * c + px;
                    if (z <= zbuf[p]) continue;
                    zbuf[p] = z;
                    nbuf[p * 3] = nax * ba + nbx * bb + ncx * bc;
                    nbuf[p * 3 + 1] = nay * ba + nby * bb + ncy * bc;
                    nbuf[p * 3 + 2] = naz * ba + nbz * bb + ncz * bc;
                }
            }
        }

        const lines = new Array(r);
        for (let row = 0; row < r; row++) {
            let line = '';
            for (let col = 0; col < c; col++) {
                const p = row * c + col;
                if (zbuf[p] === -Infinity) {
                    line += ' ';
                    continue;
                }

                let nx = nbuf[p * 3], ny = nbuf[p * 3 + 1], nz = nbuf[p * 3 + 2];
                const nl = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1;
                nx /= nl; ny /= nl; nz /= nl;

                let lum = 0.07
                    + Math.max(0, nx * K[0] + ny * K[1] + nz * K[2]) * 0.78
                    + Math.max(0, nx * A[0] + ny * A[1] + nz * A[2]) * 0.28
                    + Math.max(0, nx * B[0] + ny * B[1] + nz * B[2]) * 0.12;

                const spec = nx * K[0] + ny * K[1] + nz * K[2];
                if (spec > 0) {
                    const rz = 2 * spec * nz - K[2];
                    if (rz > 0) lum += Math.pow(rz, 18) * 0.55;
                }

                const v = invert ? 1 - lum : lum;
                // Never the blank: the silhouette has to stay solid.
                const shade = Math.round(Math.min(1, Math.max(0, v)) * last);
                line += ramp[Math.max(1, Math.min(last, shade))];
            }
            lines[row] = line.replace(/\s+$/, '');
        }

        return lines;
    }

    window.AsciiBunny = { render: render };
})();
