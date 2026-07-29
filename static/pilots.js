/**
 * pilots.js — Pilot database display
 */
(function () {
    'use strict';

    const grid = document.getElementById('pilots-grid');
    if (!grid) return;

    const pilots = [
        { name: 'PILOT-01', sync: '87.3%', status: 'ACTIVE', unit: 'EVA-00', notes: 'Neural baseline stable. Anomalous LCL readings in session 9. Flagged for observation.' },
        { name: 'PILOT-02', sync: '72.1%', status: 'STANDBY', unit: 'EVA-01', notes: 'Sync ratio declining. Emotional state: unstable. Recommended: reassignment.' },
        { name: 'PILOT-03', sync: '94.8%', status: 'ACTIVE', unit: 'EVA-02', notes: 'Highest sync ratio recorded. AT field expansion within normal parameters. Continue monitoring.' },
    ];

    pilots.forEach(p => {
        const card = document.createElement('div');
        card.className = 'pilot-card';
        card.innerHTML = `
            <div class="pilot-name">${p.name}</div>
            <div class="pilot-stat">UNIT: <span>${p.unit}</span></div>
            <div class="pilot-stat">SYNC RATIO: <span>${p.sync}</span></div>
            <div class="pilot-stat">STATUS: <span>${p.status}</span></div>
            <div class="pilot-stat" style="margin-top:0.8rem;opacity:0.35;font-size:0.6rem;">${p.notes}</div>
        `;
        grid.appendChild(card);
    });
})();
