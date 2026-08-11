/**
 * gallery.js — Gallery filtering and data
 */
(function () {
    'use strict';

    const galleryData = [
        { title: 'ASCII Fluid Simulation', desc: 'Interactive water sim', type: 'interactive', subtype: 'webgl', thumb: null },
    ];

    const grid = document.getElementById('gallery-grid');
    const filters = document.querySelectorAll('.filters button');
    const subFilters = document.getElementById('sub-filters');

    function renderItems(type, subtype) {
        grid.innerHTML = '';
        galleryData.forEach((item, idx) => {
            if (type && type !== 'all' && item.type !== type) return;
            if (subtype && item.subtype !== subtype) return;

            const el = document.createElement('div');
            el.className = 'gallery-item';
            el.innerHTML = `
                <div class="thumb" style="display:flex;align-items:center;justify-content:center;color:rgba(255,255,255,0.08);font-size:2rem;">
                    ${item.type === '3d' ? '&#127758;' : item.type === 'visual' ? '&#127912;' : '&#9000;'}
                </div>
                <div class="info">
                    <div class="title">${item.title}</div>
                    <div class="desc">${item.desc}</div>
                </div>
            `;
            grid.appendChild(el);
        });
    }

    const subtypeMap = {
        'visual': ['ascii', 'canvas', 'ui'],
        '3d': ['webgl'],
        'interactive': ['webgl', 'ui', 'canvas'],
    };

    filters.forEach(btn => {
        btn.addEventListener('click', () => {
            filters.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            const filter = btn.getAttribute('data-filter');

            subFilters.innerHTML = '';
            subFilters.classList.remove('visible');

            if (filter !== 'all' && subtypeMap[filter]) {
                subFilters.classList.add('visible');
                subtypeMap[filter].forEach(st => {
                    const sb = document.createElement('button');
                    sb.textContent = st;
                    sb.setAttribute('data-subtype', st);
                    sb.addEventListener('click', () => {
                        subFilters.querySelectorAll('button').forEach(b => b.classList.remove('active'));
                        sb.classList.add('active');
                        renderItems(filter, st);
                    });
                    subFilters.appendChild(sb);
                });
                renderItems(filter, null);
            } else {
                renderItems('all', null);
            }
        });
    });

    renderItems('all', null);
})();
