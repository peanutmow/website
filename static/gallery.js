/**
 * gallery.js — the archive.
 *
 * Renders the whole collection as one sequence of 36-frame contact sheets,
 * filters it through a colour index and a tag index, and opens a print-like
 * lightbox. All data comes from tools/gallery/sync.py via the server.
 */
(function () {
    'use strict';

    const dataEl = document.getElementById('gallery-data');
    if (!dataEl) return;

    let data;
    try {
        data = JSON.parse(dataEl.textContent);
    } catch (err) {
        console.error('gallery: unreadable dataset', err);
        return;
    }

    const photos = (data.photos || []).filter(function (p) {
        return p && p.id && p.sizes;
    });
    if (!photos.length) return;

    const SHEET_SIZE = 36;
    const COLOUR_BUCKET = 30;
    const WIDTHS = { s: 240, m: 500, z: 640, c: 800, b: 1024, l: 1600, h: 2048 };

    const $ = function (sel, root) { return (root || document).querySelector(sel); };
    const pad = function (n) { return String(n).padStart(2, '0'); };
    const pad3 = function (n) { return String(n).padStart(3, '0'); };

    const grid = $('#archive');
    const empty = $('#empty');
    const swatchRow = $('#swatches');
    const tagRow = $('#tags');
    const status = $('#status');
    const resetBtn = $('#reset');
    const lightbox = $('#lightbox');
    const lbImg = $('#lb-img');
    const lbLabel = $('#lb-label');
    const lbCounter = $('#lb-counter');
    const lbPrev = $('#lb-prev');
    const lbNext = $('#lb-next');

    const state = { hue: null, mono: false, tag: null };

    let lastVisible = [];
    let lbList = [];
    let lbIndex = 0;
    let lastFocus = null;

    // ── helpers ────────────────────────────────────────────────────

    function chunk(items, size) {
        const out = [];
        for (let i = 0; i < items.length; i += size) {
            out.push(items.slice(i, i + size));
        }
        return out;
    }

    function isCameraName(title) {
        return /^_?(DSC|DSCN|IMG|PXL|GOPR|DJI|MVI|VID)[-_]?\d/i.test(title || '') ||
            /^\d{6,}$/.test(title || '') ||
            /^photo$/i.test(title || '');
    }

    function primaryLabel(photo) {
        const tags = photo.tags || [];
        if (tags.length) return tags.slice(0, 2).join(' · ');
        if (photo.caption) return photo.caption;
        if (photo.title && !isCameraName(photo.title)) return photo.title;
        return 'untitled';
    }

    function altText(photo) {
        if (photo.alt) return photo.alt;
        if (photo.caption) return photo.caption;
        const tags = photo.tags || [];
        if (tags.length) return 'Photograph — ' + tags.join(', ');
        return 'Photograph';
    }

    function srcset(photo) {
        return Object.keys(WIDTHS)
            .filter(function (key) { return photo.sizes[key]; })
            .map(function (key) { return photo.sizes[key] + ' ' + WIDTHS[key] + 'w'; })
            .join(', ');
    }

    function bestImage(photo) {
        const order = ['l', 'b', 'c', 'z', 'm'];
        for (let i = 0; i < order.length; i++) {
            if (photo.sizes[order[i]]) return photo.sizes[order[i]];
        }
        return '';
    }

    function hexToRgb(hex) {
        const match = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex || '');
        if (!match) return null;
        return [
            parseInt(match[1], 16),
            parseInt(match[2], 16),
            parseInt(match[3], 16),
        ];
    }

    function mostSaturated(palette) {
        let best = null;
        (palette || []).forEach(function (hex) {
            const rgb = hexToRgb(hex);
            if (!rgb) return;
            const score = Math.max(rgb[0], rgb[1], rgb[2]) - Math.min(rgb[0], rgb[1], rgb[2]);
            if (!best || score > best.score) best = { rgb: rgb, score: score };
        });
        return best ? best.rgb : null;
    }

    function hueName(hue) {
        const names = [
            [20, 'red'], [40, 'rust'], [60, 'amber'], [80, 'olive'],
            [110, 'moss'], [150, 'green'], [180, 'pine'], [200, 'teal'],
            [225, 'cyan'], [250, 'steel'], [275, 'indigo'], [305, 'violet'],
            [330, 'magenta'], [350, 'rose'], [361, 'red'],
        ];
        for (let i = 0; i < names.length; i++) {
            if (hue < names[i][0]) return names[i][1];
        }
        return 'red';
    }

    // ── contact sheets ─────────────────────────────────────────────

    function sheetLabel(sheetIndex, start, end) {
        const label = document.createElement('div');
        label.className = 'sheet-label';

        const no = document.createElement('span');
        no.className = 'sheet-no';
        no.textContent = pad(sheetIndex + 1);

        const title = document.createElement('span');
        title.className = 'sheet-title';
        title.textContent = 'contact sheet';

        const meta = document.createElement('span');
        meta.className = 'sheet-meta';
        meta.textContent = (end - start + 1) + ' frames · ' + pad3(start) + '–' + pad3(end);

        label.appendChild(no);
        label.appendChild(title);
        label.appendChild(meta);
        return label;
    }

    function render() {
        const sheets = chunk(photos, SHEET_SIZE);
        const fragment = document.createDocumentFragment();

        sheets.forEach(function (slice, sheetIndex) {
            const sheet = document.createElement('section');
            sheet.className = 'sheet';
            sheet.dataset.total = String(slice.length);
            sheet.dataset.start = pad3(sheetIndex * SHEET_SIZE + 1);
            sheet.dataset.end = pad3(sheetIndex * SHEET_SIZE + slice.length);

            sheet.appendChild(sheetLabel(
                sheetIndex,
                sheetIndex * SHEET_SIZE + 1,
                sheetIndex * SHEET_SIZE + slice.length
            ));

            const frames = document.createElement('div');
            frames.className = 'frames';

            slice.forEach(function (photo, i) {
                const frameNo = sheetIndex * SHEET_SIZE + i + 1;
                frames.appendChild(makeFrame(photo, frameNo, frameNo <= 6));
            });

            sheet.appendChild(frames);
            fragment.appendChild(sheet);
        });

        grid.innerHTML = '';
        grid.appendChild(fragment);
        observeSheets();
        applyFilters();
    }

    function makeFrame(photo, frameNo, eager) {
        const frame = document.createElement('button');
        frame.type = 'button';
        frame.className = 'frame';
        frame.dataset.id = photo.id;

        const shot = document.createElement('span');
        shot.className = 'shot';

        const img = document.createElement('img');
        img.alt = altText(photo);
        img.loading = eager ? 'eager' : 'lazy';
        img.decoding = 'async';
        if (eager) img.fetchPriority = 'high';
        img.sizes = '(max-width: 720px) 45vw, (max-width: 1100px) 30vw, 230px';
        const src = photo.sizes.z || photo.sizes.c || photo.sizes.m || photo.sizes.s;
        if (src) img.src = src;
        const set = srcset(photo);
        if (set) img.srcset = set;

        img.addEventListener('error', function () {
            frame.classList.add('is-broken');
        }, { once: true });

        shot.appendChild(img);

        const meta = document.createElement('span');
        meta.className = 'frame-meta';

        const no = document.createElement('span');
        no.className = 'frame-no';
        no.textContent = pad3(frameNo);

        const tags = document.createElement('span');
        tags.className = 'frame-tags';
        tags.textContent = primaryLabel(photo);

        meta.appendChild(no);
        meta.appendChild(tags);

        frame.appendChild(shot);
        frame.appendChild(meta);

        frame.setAttribute('aria-label', altText(photo) + ' — open frame ' + pad3(frameNo));
        frame.addEventListener('click', function () {
            openLightbox(photo.id);
        });

        return frame;
    }

    function observeSheets() {
        const sheets = Array.prototype.slice.call(document.querySelectorAll('.sheet'));
        const reduce = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        if (!('IntersectionObserver' in window) || reduce) {
            sheets.forEach(function (sheet) { sheet.classList.add('is-in'); });
            return;
        }

        const observer = new IntersectionObserver(function (entries) {
            entries.forEach(function (entry) {
                if (entry.isIntersecting) {
                    entry.target.classList.add('is-in');
                    observer.unobserve(entry.target);
                }
            });
        }, { rootMargin: '240px 0px' });

        sheets.forEach(function (sheet) { observer.observe(sheet); });
    }

    // ── colour index ───────────────────────────────────────────────

    function buildSwatches() {
        const buckets = new Map();

        photos.forEach(function (photo) {
            if (photo.hue == null || (photo.sat || 0) < 0.12) return;
            const bucket = Math.floor(photo.hue / COLOUR_BUCKET) * COLOUR_BUCKET;
            const entry = buckets.get(bucket) || { count: 0, r: 0, g: 0, b: 0, sampled: 0 };
            entry.count += 1;
            const rgb = mostSaturated(photo.palette);
            if (rgb) {
                entry.r += rgb[0];
                entry.g += rgb[1];
                entry.b += rgb[2];
                entry.sampled += 1;
            }
            buckets.set(bucket, entry);
        });

        Array.from(buckets.entries())
            .sort(function (a, b) { return a[0] - b[0]; })
            .forEach(function (entry) {
                const bucket = entry[0];
                const info = entry[1];
                const centre = bucket + COLOUR_BUCKET / 2;

                let colour = 'hsl(' + centre + ' 32% 42%)';
                if (info.sampled) {
                    const r = Math.round(info.r / info.sampled);
                    const g = Math.round(info.g / info.sampled);
                    const b = Math.round(info.b / info.sampled);
                    const lum = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
                    if (lum >= 0.16 && lum <= 0.82) {
                        colour = 'rgb(' + r + ' ' + g + ' ' + b + ')';
                    }
                }

                const button = document.createElement('button');
                button.type = 'button';
                button.className = 'swatch';
                button.dataset.hue = String(bucket);
                button.style.backgroundColor = colour;
                button.title = hueName(centre) + ' · ' + info.count + ' frames';
                button.setAttribute('aria-label', hueName(centre) + ', ' + info.count + ' frames');
                button.setAttribute('aria-pressed', 'false');
                button.addEventListener('click', function () {
                    const active = state.hue === bucket;
                    state.hue = active ? null : bucket;
                    state.mono = false;
                    applyFilters();
                });
                swatchRow.appendChild(button);
            });

        const monoCount = photos.filter(function (photo) {
            return (photo.sat == null ? 1 : photo.sat) < 0.15;
        }).length;

        const mono = document.createElement('button');
        mono.type = 'button';
        mono.className = 'swatch mono';
        mono.title = 'monochrome · ' + monoCount + ' frames';
        mono.setAttribute('aria-label', 'monochrome, ' + monoCount + ' frames');
        mono.setAttribute('aria-pressed', 'false');
        mono.addEventListener('click', function () {
            state.mono = !state.mono;
            if (state.mono) state.hue = null;
            applyFilters();
        });
        swatchRow.appendChild(mono);
    }

    // ── tag index ──────────────────────────────────────────────────

    function buildTags() {
        const counts = new Map();
        photos.forEach(function (photo) {
            (photo.tags || []).forEach(function (tag) {
                counts.set(tag, (counts.get(tag) || 0) + 1);
            });
        });

        const all = makeTag('', 'all', photos.length, true);
        all.addEventListener('click', function () {
            state.tag = null;
            applyFilters();
        });
        tagRow.appendChild(all);

        const untagged = photos.filter(function (photo) {
            return !(photo.tags || []).length;
        }).length;

        if (untagged) {
            const chip = makeTag('__untagged', 'untagged', untagged, false);
            chip.addEventListener('click', function () {
                state.tag = state.tag === '__untagged' ? null : '__untagged';
                applyFilters();
            });
            tagRow.appendChild(chip);
        }

        Array.from(counts.entries())
            .sort(function (a, b) { return b[1] - a[1] || a[0].localeCompare(b[0]); })
            .forEach(function (entry) {
                const tag = entry[0];
                const chip = makeTag(tag, tag, entry[1], false);
                chip.addEventListener('click', function () {
                    state.tag = state.tag === tag ? null : tag;
                    applyFilters();
                });
                tagRow.appendChild(chip);
            });
    }

    function makeTag(tag, label, count, pressed) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'tag';
        button.dataset.tag = tag;
        button.setAttribute('aria-pressed', String(pressed));

        const text = document.createElement('span');
        text.textContent = label;

        const n = document.createElement('span');
        n.className = 'n';
        n.textContent = String(count);

        button.appendChild(text);
        button.appendChild(n);
        return button;
    }

    function syncFilterButtons() {
        swatchRow.querySelectorAll('.swatch[data-hue]').forEach(function (button) {
            button.setAttribute('aria-pressed', String(state.hue === Number(button.dataset.hue)));
        });
        const mono = swatchRow.querySelector('.swatch.mono');
        if (mono) mono.setAttribute('aria-pressed', String(state.mono));
        tagRow.querySelectorAll('.tag').forEach(function (button) {
            button.setAttribute('aria-pressed', String(button.dataset.tag === (state.tag || '')));
        });
    }

    // ── filtering ──────────────────────────────────────────────────

    function matches(photo) {
        const tags = photo.tags || [];

        if (state.tag === '__untagged') {
            if (tags.length) return false;
        } else if (state.tag && tags.indexOf(state.tag) === -1) {
            return false;
        }

        if (state.mono) {
            return (photo.sat == null ? 1 : photo.sat) < 0.15;
        }

        if (state.hue !== null) {
            if ((photo.sat || 0) < 0.12 || photo.hue == null) return false;
            return Math.floor(photo.hue / COLOUR_BUCKET) * COLOUR_BUCKET === state.hue;
        }

        return true;
    }

    function applyFilters() {
        const byId = new Map(photos.map(function (photo) { return [photo.id, photo]; }));
        const visible = [];
        const visibleBySheet = new Map();

        grid.querySelectorAll('.frame').forEach(function (element) {
            const photo = byId.get(element.dataset.id);
            if (!photo) return;
            const ok = matches(photo);
            element.classList.toggle('is-hidden', !ok);
            if (ok) {
                visible.push(photo);
                const sheet = element.closest('.sheet');
                if (sheet) {
                    visibleBySheet.set(sheet, (visibleBySheet.get(sheet) || 0) + 1);
                }
            }
        });

        const bits = [];
        if (state.hue !== null) bits.push(hueName(state.hue + COLOUR_BUCKET / 2));
        if (state.mono) bits.push('monochrome');
        if (state.tag === '__untagged') bits.push('untagged');
        else if (state.tag) bits.push(state.tag);

        grid.querySelectorAll('.sheet').forEach(function (sheet) {
            const shown = visibleBySheet.get(sheet) || 0;
            const total = Number(sheet.dataset.total || 0);
            sheet.classList.toggle('is-empty', shown === 0);

            const meta = sheet.querySelector('.sheet-meta');
            if (meta) {
                const count = bits.length ? shown + ' / ' + total : String(total);
                meta.textContent = count + ' frames · ' + sheet.dataset.start + '–' + sheet.dataset.end;
            }
        });

        lastVisible = visible;

        status.textContent = bits.length
            ? visible.length + (visible.length === 1 ? ' frame · ' : ' frames · ') + bits.join(' · ')
            : 'all ' + photos.length + ' frames';

        resetBtn.hidden = bits.length === 0;
        empty.hidden = visible.length !== 0;
        syncFilterButtons();
    }

    resetBtn.addEventListener('click', function () {
        state.hue = null;
        state.mono = false;
        state.tag = null;
        applyFilters();
    });

    // ── lightbox ───────────────────────────────────────────────────

    function frameOf(photo) {
        const index = photos.findIndex(function (item) { return item.id === photo.id; });
        if (index === -1) return null;
        return {
            frame: index + 1,
            sheet: Math.floor(index / SHEET_SIZE) + 1,
            sheets: Math.ceil(photos.length / SHEET_SIZE),
        };
    }

    function openLightbox(id) {
        const photo = photos.find(function (item) { return item.id === id; });
        if (!photo) return;

        if (!lastVisible.some(function (item) { return item.id === id; })) {
            state.hue = null;
            state.mono = false;
            state.tag = null;
            applyFilters();
        }

        lbList = lastVisible.length ? lastVisible.slice() : [photo];
        lbIndex = Math.max(0, lbList.findIndex(function (item) { return item.id === id; }));
        lastFocus = document.activeElement;

        showLightbox();
        lightbox.hidden = false;
        document.body.classList.add('lb-open');
        $('#lb-close').focus({ preventScroll: true });
        syncHash(photo.id);
    }

    function showLightbox() {
        const photo = lbList[lbIndex];
        if (!photo) return;

        const src = bestImage(photo);
        const set = srcset(photo);

        lbImg.src = src;
        if (set) {
            lbImg.srcset = set;
            lbImg.sizes = '(max-width: 900px) 100vw, 70vw';
        } else {
            lbImg.removeAttribute('srcset');
        }
        lbImg.alt = altText(photo);

        renderLabel(photo);
        lbCounter.textContent = pad3(lbIndex + 1) + ' / ' + pad3(lbList.length);
        lbPrev.disabled = lbList.length < 2;
        lbNext.disabled = lbList.length < 2;

        preload(lbIndex - 1);
        preload(lbIndex + 1);
    }

    function preload(index) {
        if (index < 0 || index >= lbList.length) return;
        const src = bestImage(lbList[index]);
        if (src) {
            const img = new Image();
            img.src = src;
        }
    }

    function renderLabel(photo) {
        lbLabel.innerHTML = '';

        const place = frameOf(photo);

        const heading = document.createElement('div');
        heading.className = 'lb-frame';
        heading.innerHTML = place
            ? 'Frame <span class="hash">' + pad3(place.frame) + '</span> / ' + pad3(photos.length)
            : 'Frame';
        lbLabel.appendChild(heading);

        if (place) {
            const sheet = document.createElement('div');
            sheet.className = 'lb-sheet';
            sheet.textContent = 'sheet ' + pad(place.sheet) + ' / ' + pad(place.sheets);
            lbLabel.appendChild(sheet);
        }

        const tags = photo.tags || [];
        if (tags.length) {
            const wrap = document.createElement('div');
            wrap.className = 'lb-tags';
            tags.forEach(function (tag) {
                const span = document.createElement('span');
                span.className = 'lb-tag';
                span.textContent = tag;
                wrap.appendChild(span);
            });
            lbLabel.appendChild(wrap);
        }

        const fields = document.createElement('div');
        fields.className = 'lb-fields';
        addField(fields, 'file', photo.title && !isCameraName(photo.title) ? photo.title : null);
        addField(fields, 'size', photo.width && photo.height ? photo.width + ' × ' + photo.height : null);
        addField(fields, 'type', photo.type && photo.type !== 'photography' ? photo.type : null);
        if (photo.exif) {
            addField(fields, 'camera', photo.exif.camera);
            addField(fields, 'lens', photo.exif.lens);
            addField(fields, 'focal', photo.exif.focal);
            addField(fields, 'aperture', photo.exif.aperture);
            addField(fields, 'shutter', photo.exif.shutter);
            addField(fields, 'iso', photo.exif.iso);
        }
        if (fields.children.length) lbLabel.appendChild(fields);

        const palette = photo.palette || [];
        if (palette.length) {
            const wrap = document.createElement('div');
            wrap.className = 'lb-palette';
            palette.forEach(function (colour) {
                const swatch = document.createElement('span');
                swatch.className = 'lb-swatch';
                swatch.style.background = colour;
                swatch.title = colour;
                wrap.appendChild(swatch);
            });
            lbLabel.appendChild(wrap);
        }

        if (photo.caption) {
            const caption = document.createElement('div');
            caption.className = 'lb-caption';
            caption.textContent = photo.caption;
            lbLabel.appendChild(caption);
        }

        const link = document.createElement('a');
        link.className = 'lb-link';
        link.href = photo.url || ('https://www.flickr.com/photos/' + (data.owner || '') + '/' + photo.id + '/');
        link.target = '_blank';
        link.rel = 'noopener';
        link.textContent = 'view on flickr ↗';
        lbLabel.appendChild(link);
    }

    function addField(root, name, value) {
        if (!value) return;
        const row = document.createElement('div');
        const key = document.createElement('span');
        key.textContent = name;
        const val = document.createElement('b');
        val.textContent = value;
        row.appendChild(key);
        row.appendChild(val);
        root.appendChild(row);
    }

    function closeLightbox() {
        lightbox.hidden = true;
        document.body.classList.remove('lb-open');
        lbImg.removeAttribute('src');
        lbImg.removeAttribute('srcset');
        clearHash();
        if (lastFocus && lastFocus.focus) lastFocus.focus({ preventScroll: true });
    }

    function step(delta) {
        if (lbList.length < 2) return;
        lbIndex = (lbIndex + delta + lbList.length) % lbList.length;
        showLightbox();
        const photo = lbList[lbIndex];
        if (photo) syncHash(photo.id);
    }

    function syncHash(id) {
        const next = '#p/' + id;
        if (location.hash !== next) {
            history.replaceState(null, '', location.pathname + location.search + next);
        }
    }

    function clearHash() {
        if (location.hash) {
            history.replaceState(null, '', location.pathname + location.search);
        }
    }

    lbPrev.addEventListener('click', function () { step(-1); });
    lbNext.addEventListener('click', function () { step(1); });
    $('#lb-close').addEventListener('click', closeLightbox);
    lightbox.querySelectorAll('[data-close]').forEach(function (element) {
        element.addEventListener('click', closeLightbox);
    });

    document.addEventListener('keydown', function (event) {
        if (lightbox.hidden) return;
        if (event.key === 'Escape') {
            closeLightbox();
            event.preventDefault();
        } else if (event.key === 'ArrowLeft') {
            step(-1);
            event.preventDefault();
        } else if (event.key === 'ArrowRight') {
            step(1);
            event.preventDefault();
        } else if (event.key === 'Tab') {
            const focusables = Array.from(lightbox.querySelectorAll('button:not([disabled]), a[href]'));
            if (!focusables.length) return;
            const first = focusables[0];
            const last = focusables[focusables.length - 1];
            if (event.shiftKey && document.activeElement === first) {
                last.focus();
                event.preventDefault();
            } else if (!event.shiftKey && document.activeElement === last) {
                first.focus();
                event.preventDefault();
            }
        }
    });

    window.addEventListener('hashchange', function () {
        const match = location.hash.match(/^#p\/(\d+)$/);
        if (match) {
            if (lightbox.hidden) openLightbox(match[1]);
        } else if (!lightbox.hidden) {
            closeLightbox();
        }
    });

    // ── boot ───────────────────────────────────────────────────────

    const sheetCount = Math.ceil(photos.length / SHEET_SIZE);
    $('#frame-count').textContent = photos.length;
    $('#sheet-count').textContent = sheetCount;
    $('#foot-count').textContent = photos.length + ' frames · ' + sheetCount + ' contact sheets';
    document.title = document.title.replace('Gallery', 'Gallery · ' + photos.length + ' frames');

    buildSwatches();
    buildTags();
    render();

    const deepLink = location.hash.match(/^#p\/(\d+)$/);
    if (deepLink) {
        requestAnimationFrame(function () {
            openLightbox(deepLink[1]);
        });
    }
})();
