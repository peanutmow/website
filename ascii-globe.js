// ── DOM refs ─────────────────────────────────────────────────────────────────
const globeEl   = document.getElementById('ascii-globe');
const zoomHud   = document.getElementById('globe-zoom-hud');
const coordsHud = document.getElementById('globe-coords-hud');
const focusHud  = document.getElementById('globe-focus-hud');
const levelHud  = document.getElementById('globe-level-hud');

// ── ASCII ramp ────────────────────────────────────────────────────────────────
const CHARS = ' `.-:;=+*#%@';
const CL    = CHARS.length - 1;

// ── Grid ──────────────────────────────────────────────────────────────────────
let COLS = 180;
let ROWS = 60;
let FS   = 10;

// ── Globe state ───────────────────────────────────────────────────────────────
let zoom  = 1.0;
// rotY=0: prime meridian (Europe/Africa) faces viewer
let rotX  = 0.25;
let rotY  = 0.0;

let drag = false, dsx = 0, dsy = 0, drx = 0, dry = 0;

// ── World texture ─────────────────────────────────────────────────────────────
const MASK_W = 720, MASK_H = 360; // Increased resolution
let landMask = null;
let cityMask = null;
let districtMask = null;
let regions = []; // To store name of each region ID
let CITY_GRID = Array.from({length: 36}, () => Array.from({length: 72}, () => []));

async function loadTexture() {
    try {
        const world = await fetch(
            'https://cdn.jsdelivr.net/npm/world-atlas@2/countries-110m.json'
        ).then(r => r.json());

        const features = topojson.feature(world, world.objects.countries).features;
        regions = features.map(f => (f.properties.name || "UNKNOWN").toUpperCase());

        const c   = document.createElement('canvas');
        c.width   = MASK_W; c.height = MASK_H;
        const ctx = c.getContext('2d', { willReadFrequently: true });

        // Fill ocean (id = 0)
        ctx.fillStyle = '#000000';
        ctx.fillRect(0, 0, MASK_W, MASK_H);

        const proj = d3.geoEquirectangular()
            .scale(MASK_H / Math.PI)
            .translate([MASK_W / 2, MASK_H / 2]);
        const path = d3.geoPath(proj, ctx);

        // Draw each country with its ID (1-indexed, we use RGB color to encode up to ~255)
        for (let i = 0; i < features.length; i++) {
            const id = i + 1; // 1..177
            // Encode ID into RGB. Since max is ~177, we can just use the R channel
            ctx.fillStyle = `rgb(${id}, 0, 0)`;
            ctx.beginPath();
            path(features[i]);
            ctx.fill();
        }

        const px = ctx.getImageData(0, 0, MASK_W, MASK_H).data;
        landMask = new Uint16Array(MASK_W * MASK_H);
        for (let i = 0; i < MASK_W * MASK_H; i++) {
            landMask[i] = px[i * 4]; // The R channel contains our ID
        }
    } catch (e) {
        console.warn('World map load failed, using procedural fallback:', e);
        landMask = null;
    }
}

async function loadCities() {
    try {
        const data = await fetch('cities.json').then(r => r.json());
        for (const c of data) {
            const r = Math.min(35, Math.max(0, Math.floor((c.l + 90) / 5)));
            const col = Math.min(71, Math.max(0, Math.floor((c.L + 180) / 5)));
            c.nameStr = c.n.replace(/\s+/g, '') + '*';
            const latR = c.l * Math.PI / 180;
            const lonR = c.L * Math.PI / 180;
            c.x = Math.sin(lonR) * Math.cos(latR);
            c.y = Math.sin(latR);
            c.z = Math.cos(lonR) * Math.cos(latR);
            CITY_GRID[r][col].push(c);
        }
        for (const p of PLACES) {
            if (p.type !== 'district') continue;
            const r = Math.min(35, Math.max(0, Math.floor((p.lat + 90) / 5)));
            const col = Math.min(71, Math.max(0, Math.floor((p.lon + 180) / 5)));
            const cObj = {
                nameStr: p.name.toUpperCase().replace(/\s+/g, '') + '*',
                type: 'district',
                x: Math.sin(p.lon * Math.PI / 180) * Math.cos(p.lat * Math.PI / 180),
                y: Math.sin(p.lat * Math.PI / 180),
                z: Math.cos(p.lon * Math.PI / 180) * Math.cos(p.lat * Math.PI / 180)
            };
            CITY_GRID[r][col].push(cObj);
        }
        
        cityMask = new Array(MASK_W * MASK_H).fill('');
        districtMask = new Array(MASK_W * MASK_H).fill('');
        
        for (let ty = 0; ty < MASK_H; ty++) {
            const lat = (0.5 - ty / MASK_H) * Math.PI;
            const latDeg = lat * 180 / Math.PI;
            const rMask = Math.min(35, Math.max(0, Math.floor((latDeg + 90) / 5)));
            for (let tx = 0; tx < MASK_W; tx++) {
                const idx = ty * MASK_W + tx;
                const regionId = landMask ? landMask[idx] : 0;
                if (regionId === 0) continue;
                const lon = ((tx / MASK_W) - 0.5) * 2 * Math.PI;
                const lonDeg = lon * 180 / Math.PI;
                const gridCol = Math.min(71, Math.max(0, Math.floor((lonDeg + 180) / 5)));
                const wx = Math.sin(lon) * Math.cos(lat);
                const wy = Math.sin(lat);
                const wz = Math.cos(lon) * Math.cos(lat);
                let bestCityDist = Infinity, bestDistDist = Infinity;
                let bestCity = '', bestDistrict = '';
                for (let dr = -1; dr <= 1; dr++) {
                    for (let dc = -1; dc <= 1; dc++) {
                        const nr = rMask + dr;
                        let nc = gridCol + dc;
                        if (nr < 0 || nr > 35) continue;
                        if (nc < 0) nc += 72;
                        if (nc > 71) nc -= 72;
                        for (const c of CITY_GRID[nr][nc]) {
                            const dist = (c.x - wx)**2 + (c.y - wy)**2 + (c.z - wz)**2;
                            if (c.type === 'district') {
                                if (dist < bestDistDist) {
                                    bestDistDist = dist;
                                    bestDistrict = c.nameStr.replace(/\*/g, '');
                                }
                            } else {
                                if (dist < bestCityDist) {
                                    bestCityDist = dist;
                                    bestCity = c.nameStr.replace(/\*/g, '');
                                }
                            }
                        }
                    }
                }
                const defName = regions && regions.length >= regionId ? regions[regionId - 1].replace(/\s+/g, '') : 'LAND';
                cityMask[idx] = bestCity || defName;
                districtMask[idx] = bestDistrict || cityMask[idx];
            }
        }
        console.log('Text masks built entirely pre-calculated');
    } catch (e) {
        console.warn('Cities detail file failed to load', e);
    }
}

// Returns the region ID at (lat, lon)
function getRegionId(lat, lon) {
    if (!landMask) return 0;
    const tx = (((lon / (2 * Math.PI) + 0.5) * MASK_W | 0) + MASK_W) % MASK_W;
    const ty = Math.min(MASK_H - 1, Math.max(0, (0.5 - lat / Math.PI) * MASK_H | 0));
    return landMask[ty * MASK_W + tx];
}

// ── Places ────────────────────────────────────────────────────────────────────
const PLACES = [
    // Countries
    { name: "RUSSIA",       lat:  61.5, lon:  90.0, type: "country" },
    { name: "CANADA",       lat:  56.1, lon:-106.3, type: "country" },
    { name: "USA",          lat:  37.0, lon: -95.0, type: "country" },
    { name: "BRAZIL",       lat: -14.2, lon: -51.9, type: "country" },
    { name: "ARGENTINA",    lat: -38.4, lon: -63.6, type: "country" },
    { name: "GREENLAND",    lat:  72.0, lon: -42.0, type: "country" },
    { name: "UK",           lat:  55.3, lon:  -3.4, type: "country" },
    { name: "FRANCE",       lat:  46.2, lon:   2.2, type: "country" },
    { name: "GERMANY",      lat:  51.1, lon:  10.4, type: "country" },
    { name: "SPAIN",        lat:  40.4, lon:  -3.7, type: "country" },
    { name: "ITALY",        lat:  41.9, lon:  12.5, type: "country" },
    { name: "UKRAINE",      lat:  48.3, lon:  31.2, type: "country" },
    { name: "TURKEY",       lat:  38.9, lon:  35.2, type: "country" },
    { name: "SAUDI ARABIA", lat:  23.9, lon:  45.1, type: "country" },
    { name: "IRAN",         lat:  32.4, lon:  53.7, type: "country" },
    { name: "INDIA",        lat:  20.5, lon:  78.9, type: "country" },
    { name: "CHINA",        lat:  35.8, lon: 104.1, type: "country" },
    { name: "MONGOLIA",     lat:  46.8, lon: 103.8, type: "country" },
    { name: "KAZAKH.",      lat:  48.0, lon:  66.9, type: "country" },
    { name: "JAPAN",        lat:  36.2, lon: 138.2, type: "country" },
    { name: "INDONESIA",    lat:  -0.8, lon: 113.9, type: "country" },
    { name: "AUSTRALIA",    lat: -25.2, lon: 133.7, type: "country" },
    { name: "NEW ZEALAND",  lat: -40.9, lon: 174.9, type: "country" },
    { name: "EGYPT",        lat:  26.8, lon:  30.8, type: "country" },
    { name: "NIGERIA",      lat:   9.1, lon:   8.7, type: "country" },
    { name: "S. AFRICA",    lat: -30.5, lon:  22.9, type: "country" },
    { name: "ETHIOPIA",     lat:   9.1, lon:  40.5, type: "country" },
    { name: "MEXICO",       lat:  23.6, lon:-102.5, type: "country" },
    { name: "ALASKA",       lat:  64.0, lon:-153.0, type: "country" },
    { name: "ANTARCTICA",   lat: -80.0, lon:   0.0, type: "country" },

    // Cities
    { name: "LONDON",        lat:  51.5074, lon:  -0.1278, type: "city" },
    { name: "PARIS",         lat:  48.8566, lon:   2.3522, type: "city" },
    { name: "BERLIN",        lat:  52.5200, lon:  13.4050, type: "city" },
    { name: "ROME",          lat:  41.9028, lon:  12.4964, type: "city" },
    { name: "MADRID",        lat:  40.4168, lon:  -3.7038, type: "city" },
    { name: "AMSTERDAM",     lat:  52.3676, lon:   4.9041, type: "city" },
    { name: "WARSAW",        lat:  52.2297, lon:  21.0122, type: "city" },
    { name: "MOSCOW",        lat:  55.7522, lon:  37.6156, type: "city" },
    { name: "ISTANBUL",      lat:  41.0082, lon:  28.9784, type: "city" },
    { name: "CAIRO",         lat:  30.0444, lon:  31.2357, type: "city" },
    { name: "NAIROBI",       lat:  -1.2921, lon:  36.8219, type: "city" },
    { name: "CAPE TOWN",     lat: -33.9249, lon:  18.4241, type: "city" },
    { name: "LAGOS",         lat:   6.5244, lon:   3.3792, type: "city" },
    { name: "TEHRAN",        lat:  35.6892, lon:  51.3890, type: "city" },
    { name: "DELHI",         lat:  28.6139, lon:  77.2090, type: "city" },
    { name: "MUMBAI",        lat:  19.0760, lon:  72.8777, type: "city" },
    { name: "BANGALORE",     lat:  12.9716, lon:  77.5946, type: "city" },
    { name: "BANGKOK",       lat:  13.7563, lon: 100.5018, type: "city" },
    { name: "SINGAPORE",     lat:   1.3521, lon: 103.8198, type: "city" },
    { name: "JAKARTA",       lat:  -6.2088, lon: 106.8456, type: "city" },
    { name: "BEIJING",       lat:  39.9042, lon: 116.4074, type: "city" },
    { name: "SHANGHAI",      lat:  31.2304, lon: 121.4737, type: "city" },
    { name: "HONG KONG",     lat:  22.3193, lon: 114.1694, type: "city" },
    { name: "SEOUL",         lat:  37.5665, lon: 126.9780, type: "city" },
    { name: "TOKYO",         lat:  35.6895, lon: 139.6917, type: "city" },
    { name: "OSAKA",         lat:  34.6937, lon: 135.5023, type: "city" },
    { name: "SYDNEY",        lat: -33.8688, lon: 151.2093, type: "city" },
    { name: "MELBOURNE",     lat: -37.8136, lon: 144.9631, type: "city" },
    { name: "TORONTO",       lat:  43.6532, lon: -79.3832, type: "city" },
    { name: "NEW YORK",      lat:  40.7128, lon: -74.0060, type: "city" },
    { name: "CHICAGO",       lat:  41.8781, lon: -87.6298, type: "city" },
    { name: "LOS ANGELES",   lat:  34.0522, lon:-118.2437, type: "city" },
    { name: "MEXICO CITY",   lat:  19.4326, lon: -99.1332, type: "city" },
    { name: "SAO PAULO",     lat: -23.5505, lon: -46.6333, type: "city" },
    { name: "BUENOS AIRES",  lat: -34.6037, lon: -58.3816, type: "city" },

    // Districts (Tokyo)
    { name: "Shinjuku",      lat:  35.6938, lon: 139.7034, type: "district" },
    { name: "Shibuya",       lat:  35.6580, lon: 139.7016, type: "district" },
    { name: "Minato-ku",     lat:  35.6581, lon: 139.7515, type: "district" },
    { name: "Chiyoda-ku",    lat:  35.6940, lon: 139.7536, type: "district" },
    { name: "Akihabara",     lat:  35.6983, lon: 139.7731, type: "district" },
    { name: "Ginza",         lat:  35.6710, lon: 139.7663, type: "district" },
    { name: "Asakusa",       lat:  35.7119, lon: 139.7983, type: "district" },
    { name: "Koto-ku",       lat:  35.6415, lon: 139.8186, type: "district" },
    { name: "Meguro-ku",     lat:  35.6415, lon: 139.6981, type: "district" },
    { name: "Harajuku",      lat:  35.6702, lon: 139.7026, type: "district" },
    { name: "Roppongi",      lat:  35.6627, lon: 139.7310, type: "district" },
    { name: "Ikebukuro",     lat:  35.7295, lon: 139.7109, type: "district" },
    { name: "Odaiba",        lat:  35.6230, lon: 139.7755, type: "district" },
    { name: "Ueno",          lat:  35.7141, lon: 139.7774, type: "district" },
    { name: "Nakameguro",    lat:  35.6440, lon: 139.6982, type: "district" },
    { name: "Shimokitazawa", lat:  35.6614, lon: 139.6674, type: "district" },

    // Districts (Paris)
    { name: "Le Marais",     lat:  48.8590, lon:   2.3590, type: "district" },
    { name: "Montmartre",    lat:  48.8867, lon:   2.3431, type: "district" },
    { name: "Latin Quarter", lat:  48.8539, lon:   2.3470, type: "district" },
    { name: "Saint-Germain", lat:  48.8538, lon:   2.3323, type: "district" },
    { name: "Belleville",    lat:  48.8720, lon:   2.3838, type: "district" },
    { name: "Bastille",      lat:  48.8533, lon:   2.3692, type: "district" },
    { name: "Pigalle",       lat:  48.8814, lon:   2.3328, type: "district" },
    { name: "Oberkampf",     lat:  48.8639, lon:   2.3701, type: "district" },

    // Districts (London)
    { name: "Westminster",   lat:  51.4973, lon:  -0.1372, type: "district" },
    { name: "Camden",        lat:  51.5290, lon:  -0.1255, type: "district" },
    { name: "Shoreditch",    lat:  51.5227, lon:  -0.0784, type: "district" },
    { name: "Greenwich",     lat:  51.4769, lon:  -0.0005, type: "district" },
    { name: "Hackney",       lat:  51.5450, lon:  -0.0553, type: "district" },
    { name: "Brixton",       lat:  51.4614, lon:  -0.1142, type: "district" },

    // Districts (New York)
    { name: "Manhattan",     lat:  40.7831, lon: -73.9712, type: "district" },
    { name: "Brooklyn",      lat:  40.6782, lon: -73.9442, type: "district" },
    { name: "Queens",        lat:  40.7282, lon: -73.7949, type: "district" },
    { name: "The Bronx",     lat:  40.8370, lon: -73.8858, type: "district" },
    { name: "Harlem",        lat:  40.8116, lon: -73.9465, type: "district" },
    { name: "SoHo",          lat:  40.7233, lon: -74.0030, type: "district" },

    // Districts (Seoul)
    { name: "Gangnam-gu",    lat:  37.5172, lon: 127.0473, type: "district" },
    { name: "Hongdae",       lat:  37.5563, lon: 126.9238, type: "district" },
    { name: "Myeongdong",    lat:  37.5631, lon: 126.9820, type: "district" },
    { name: "Insadong",      lat:  37.5742, lon: 126.9853, type: "district" },
    { name: "Itaewon",       lat:  37.5345, lon: 126.9940, type: "district" },
];

// ── Font sizing ───────────────────────────────────────────────────────────────
function applySize() {
    FS = 10;
    globeEl.style.fontSize    = FS + 'px';
    globeEl.style.lineHeight  = '1.0';
    globeEl.style.letterSpacing = '0';
    
    // Calculate how many characters fit on the screen
    // Monospace aspect ratio (width/height) is generally ~0.6
    COLS = Math.floor(window.innerWidth / (FS * 0.601));
    ROWS = Math.floor(window.innerHeight / FS);
    
    // Ensure minimum size and even numbers for center
    if (COLS < 40) COLS = 40;
    if (ROWS < 20) ROWS = 20;
    if (COLS % 2 !== 0) COLS++;
    if (ROWS % 2 !== 0) ROWS++;
}

// ── Render ────────────────────────────────────────────────────────────────────
function render() {
    const buf = new Array(COLS * ROWS).fill(' ');

    // Light: upper-left toward viewer
    const lx = -0.4, ly = 0.6, lz = 0.7;
    const lm  = Math.hypot(lx, ly, lz);
    const lix = lx/lm, liy = ly/lm, liz = lz/lm;

    // Radius scaled to take up the whole screen at minimum dimension
    const R  = Math.min(ROWS * 0.48, COLS * 0.24) * zoom;
    const R2 = R * R;

    const cX = Math.cos(rotX), sX = Math.sin(rotX);
    const cY = Math.cos(rotY), sY = Math.sin(rotY);
    const ocx = COLS / 2, ocy = ROWS / 2;

    const screenMap = new Array(COLS * ROWS).fill(null);
    const dotsMap = new Float32Array(COLS * ROWS);

    for (let row = 0; row < ROWS; row++) {
        // sy_ positive = downward on screen
        const sy_ = row - ocy;
        for (let col = 0; col < COLS; col++) {
            // ×0.5: chars are ~2× taller than wide
            const sx_ = (col - ocx) * 0.5;
            const d2  = sx_*sx_ + sy_*sy_;
            if (d2 >= R2) continue;

            const sz_c = Math.sqrt(R2 - d2);

            // Camera-space normal (screen Y = down, so flip for world Y = north)
            const nx =  sx_ / R;
            const ny = -sy_ / R;  // flipped: screen-down → world-north
            const nz =  sz_c / R;

            // Lighting with screen-space normals (before flip)
            const dot = Math.max(0.08, Math.min(1.0,
                nx * lix + (-ny) * liy + nz * liz));

            // Un-rotate to world lat/lon:
            // 1) Undo R_X: ty = ny*cX + nz*sX,  tz = -ny*sX + nz*cX
            const t_y =  ny*cX + nz*sX;
            const t_z = -ny*sX + nz*cX;
            // 2) Undo R_Y: wx = nx*cY - tz*sY,  wz = nx*sY + tz*cY
            const wx  =  nx*cY - t_z*sY;
            const wz  =  nx*sY + t_z*cY;
            const wy  =  t_y;   // world Y = north

            const lat = Math.asin(Math.max(-1, Math.min(1, wy)));
            const lon = Math.atan2(wx, wz);

            const tx = (((lon / (2 * Math.PI) + 0.5) * MASK_W | 0) + MASK_W) % MASK_W;
            const ty = Math.min(MASK_H - 1, Math.max(0, (0.5 - lat / Math.PI) * MASK_H | 0));
            const mIdx = ty * MASK_W + tx;

            const regionId = landMask ? landMask[mIdx] : 0;
            
            if (regionId > 0 && regions && regions.length >= regionId) {
                // Find O(1) precalculated text string
                let name = regions[regionId - 1].replace(/\s+/g, '');
                if (zoom > 6.0 && districtMask) name = districtMask[mIdx];
                else if (zoom > 1.8 && cityMask) name = cityMask[mIdx];

                screenMap[row * COLS + col] = name;
                dotsMap[row * COLS + col] = dot;
            } else {
                // Ocean
                let ci = Math.round(dot * 3.5);
                buf[row * COLS + col] = CHARS[Math.max(0, Math.min(CL, ci))];
            }
        }
    }

    // Pass 2: Stamp full words seamlessly onto screenMap land
    for (let row = 0; row < ROWS; row++) {
        let col = 0;
        while (col < COLS) {
            const idx = row * COLS + col;
            const name = screenMap[idx];
            if (name) {
                // Try stamping the full word
                const len = name.length;
                let canFit = true;
                
                // Needs to fit on screen
                if (col + len > COLS) {
                    canFit = false;
                } else {
                    // All chars in the stamp must land on actual land
                    for (let k = 0; k < len; k++) {
                        if (!screenMap[idx + k]) {
                            canFit = false;
                            break;
                        }
                    }
                }

                if (canFit) {
                    for (let k = 0; k < len; k++) {
                        const cDot = dotsMap[idx + k];
                        let c = name[k];
                        if (cDot < 0.15) c = c.toLowerCase();
                        buf[idx + k] = c;
                    }
                    // Add a separator space if there's more land immediately following
                    if (col + len < COLS && screenMap[idx + len]) {
                        buf[idx + len] = ' ';
                        col += len + 1;
                    } else {
                        col += len;
                    }
                } else {
                    // Not enough room for a full word, put a generic dim land hash or dot.
                    const cDot = dotsMap[idx];
                    buf[idx] = cDot < 0.15 ? '.' : '#';
                    col++;
                }
            } else {
                col++;
            }
        }
    }

    // ── Labels ────────────────────────────────────────────────────────────────
    let level;
    if      (zoom < 2.0) level = 'country';
    else if (zoom < 5.0) level = 'city';
    else                 level = 'district';

    let focusName = 'OCEAN', focusDist = Infinity;

    for (const p of PLACES) {
        if (p.type !== level) continue;

        const latR = p.lat * Math.PI / 180;
        const lonR = p.lon * Math.PI / 180;

        // World unit sphere: x=sin(lon)*cos(lat), y=sin(lat)[north], z=cos(lon)*cos(lat)
        const px =  Math.sin(lonR) * Math.cos(latR);
        const py =  Math.sin(latR);
        const pz =  Math.cos(lonR) * Math.cos(latR);

        // Apply R_Y then R_X
        const rx1 =  px*cY + pz*sY;
        const ry1 =  py;
        const rz1 = -px*sY + pz*cY;
        const rx  =  rx1;
        const ry  =  ry1*cX - rz1*sX;
        const rz  =  ry1*sX + rz1*cX;

        if (rz < 0.05) continue;

        // Project: rx east-west (×2 char aspect), ry world-north → flip for screen-up
        const sx = Math.round( rx * R * 2 + ocx);
        const sy = Math.round(-ry * R     + ocy);

        const dist = Math.abs(rx) + Math.abs(ry);
        if (dist < focusDist) { focusDist = dist; focusName = p.name; }

        const lbl = `[${p.name}]`;
        const lx0 = sx - (lbl.length >> 1);
        for (let i = 0; i < lbl.length; i++) {
            const lc = lx0 + i;
            if (lc >= 0 && lc < COLS && sy >= 0 && sy < ROWS)
                buf[sy * COLS + lc] = lbl[i];
        }
    }

    // ── DOM output ────────────────────────────────────────────────────────────
    let out = '';
    for (let r = 0; r < ROWS; r++)
        out += buf.slice(r * COLS, (r+1) * COLS).join('') + '\n';
    globeEl.textContent = out;

    // HUD
    const dispLat = (rotX * 180 / Math.PI).toFixed(1);
    let   dispLon = (-rotY * 180 / Math.PI) % 360;
    if (dispLon >  180) dispLon -= 360;
    if (dispLon < -180) dispLon += 360;
    zoomHud.textContent   = `ZOOM: ${zoom.toFixed(2)}x`;
    coordsHud.textContent = `LAT: ${dispLat}  LON: ${dispLon.toFixed(1)}`;
    focusHud.textContent  = `FOCUS: ${focusName}`;
    levelHud.textContent  = `LEVEL: ${level.toUpperCase()}S`;
}

// ── Events ────────────────────────────────────────────────────────────────────
globeEl.addEventListener('mousedown', e => {
    drag = true; dsx = e.clientX; dsy = e.clientY; drx = rotX; dry = rotY;
    e.preventDefault();
});
window.addEventListener('mousemove', e => {
    if (!drag) return;
    // Invert the rotY delta (+ instead of -) so dragging right rotates the globe right
    rotY = dry + (e.clientX - dsx) * 0.007 / zoom;
    rotX = drx + (e.clientY - dsy) * 0.007 / zoom;
    rotX = Math.max(-Math.PI/2 + 0.01, Math.min(Math.PI/2 - 0.01, rotX));
    render();
});
window.addEventListener('mouseup', () => { drag = false; });
window.addEventListener('wheel', e => {
    e.preventDefault();
    zoom *= e.deltaY > 0 ? 0.88 : 1.14;
    zoom  = Math.max(0.35, Math.min(15.0, zoom));
    render();
}, { passive: false });
window.addEventListener('resize', () => { applySize(); render(); });

// ── Boot ──────────────────────────────────────────────────────────────────────
async function init() {
    await document.fonts.ready;
    applySize();
    await Promise.all([loadTexture(), loadCities()]);
    render();             // render with accurate geography
}
init();
