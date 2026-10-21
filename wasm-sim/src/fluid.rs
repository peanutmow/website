use crate::rng::Rng;

pub const VISCOSITY: f64 = 0.000_000_2;
pub const DT: f64 = 0.017;
pub const DIFFUSE_ITERS: u32 = 10;
pub const PRESSURE_ITERS: u32 = 25;
pub const FORCE_STRENGTH: f64 = 3.0;
pub const MAX_PUSH_CELLS: f64 = 22.0;
pub const MOUSE_SPEED_FORCE: f64 = 0.125;
pub const MOUSE_FORCE_CAP: f64 = 3.0;
pub const VORTICITY: f64 = 8.0;
pub const VORTICITY_CAP: f64 = 0.05;
pub const DENSITY_PER_FRAME: f64 = 1.0;
pub const BRUSH_RADIUS: i32 = 2;
pub const DISSIPATION: f64 = 0.992;
pub const VEL_DISSIPATION: f64 = 0.997;
pub const AMBIENT_FORCE: f64 = 0.0003;
pub const AMBIENT_DENSITY: f64 = 0.0005;

#[derive(Clone, Copy, Debug)]
pub struct TrailSeg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Mask {
    pub row: f64,
    pub col: f64,
    pub rows: f64,
    pub cols: f64,
    pub fade: f64,
}

pub struct Fluid {
    pub cols: u32,
    pub rows: u32,
    pub total: usize,
    pub dens: Vec<f32>,
    pub dens_prev: Vec<f32>,
    pub u: Vec<f32>,
    pub v: Vec<f32>,
    pub u_prev: Vec<f32>,
    pub v_prev: Vec<f32>,
    pressure: Vec<f32>,
    divergence: Vec<f32>,
    curl: Vec<f32>,
    curl_abs: Vec<f32>,
}

impl Fluid {
    pub fn new(cols: u32, rows: u32) -> Self {
        let total = (cols as usize).saturating_mul(rows as usize);
        Fluid {
            cols,
            rows,
            total,
            dens: vec![0.0; total],
            dens_prev: vec![0.0; total],
            u: vec![0.0; total],
            v: vec![0.0; total],
            u_prev: vec![0.0; total],
            v_prev: vec![0.0; total],
            pressure: vec![0.0; total],
            divergence: vec![0.0; total],
            curl: vec![0.0; total],
            curl_abs: vec![0.0; total],
        }
    }

    #[inline]
    pub fn idx(cols: u32, i: i64, j: i64) -> usize {
        (i + j * i64::from(cols)) as usize
    }

    #[inline]
    pub fn at(&self, i: i64, j: i64) -> usize {
        Self::idx(self.cols, i, j)
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        let old = if self.cols > 0 && self.rows > 0 {
            Some((
                self.cols,
                self.rows,
                self.dens.clone(),
                self.u.clone(),
                self.v.clone(),
            ))
        } else {
            None
        };

        self.cols = cols;
        self.rows = rows;
        self.total = (cols as usize).saturating_mul(rows as usize);
        self.dens = vec![0.0; self.total];
        self.dens_prev = vec![0.0; self.total];
        self.u = vec![0.0; self.total];
        self.v = vec![0.0; self.total];
        self.u_prev = vec![0.0; self.total];
        self.v_prev = vec![0.0; self.total];
        self.pressure = vec![0.0; self.total];
        self.divergence = vec![0.0; self.total];
        self.curl = vec![0.0; self.total];
        self.curl_abs = vec![0.0; self.total];

        if let Some((old_cols, old_rows, dens, u, v)) = old {
            self.carry_field(old_cols, old_rows, &dens, &u, &v);
        }
    }

    fn carry_field(&mut self, old_cols: u32, old_rows: u32, dens: &[f32], u: &[f32], v: &[f32]) {
        let sx = f64::from(old_cols) / f64::from(self.cols);
        let sy = f64::from(old_rows) / f64::from(self.rows);
        let oc = old_cols as usize;
        let nc = self.cols as usize;

        for j in 0..self.rows as usize {
            let fj = clamp_f64((j as f64 + 0.5) * sy - 0.5, 0.0, f64::from(old_rows) - 1.0);
            let j0 = fj.floor() as usize;
            let j1 = (j0 + 1).min(old_rows as usize - 1);
            let rj = fj - j0 as f64;
            for i in 0..self.cols as usize {
                let fi = clamp_f64((i as f64 + 0.5) * sx - 0.5, 0.0, f64::from(old_cols) - 1.0);
                let i0 = fi.floor() as usize;
                let i1 = (i0 + 1).min(old_cols as usize - 1);
                let ri = fi - i0 as f64;

                let a = i0 + j0 * oc;
                let b = i1 + j0 * oc;
                let c = i0 + j1 * oc;
                let d = i1 + j1 * oc;
                let p = i + j * nc;

                self.dens[p] = mix4(dens[a], dens[b], dens[c], dens[d], ri, rj) as f32;
                self.u[p] = mix4(u[a], u[b], u[c], u[d], ri, rj) as f32;
                self.v[p] = mix4(v[a], v[b], v[c], v[d], ri, rj) as f32;
            }
        }
        self.dens_prev.copy_from_slice(&self.dens);
    }

    pub fn set_bounds(arr: &mut [f32], btype: u8, cols: u32, rows: u32) {
        if cols < 2 || rows < 2 {
            return;
        }
        let lc = (cols - 1) as usize;
        let lr = (rows - 1) as usize;
        let c = cols as usize;

        for i in 1..lc {
            arr[i] = if btype == 2 { -arr[i + c] } else { arr[i + c] };
            arr[i + lr * c] = if btype == 2 {
                -arr[i + (lr - 1) * c]
            } else {
                arr[i + (lr - 1) * c]
            };
        }
        for j in 1..lr {
            arr[j * c] = if btype == 1 {
                -arr[1 + j * c]
            } else {
                arr[1 + j * c]
            };
            arr[lc + j * c] = if btype == 1 {
                -arr[lc - 1 + j * c]
            } else {
                arr[lc - 1 + j * c]
            };
        }
        arr[0] = ((f64::from(arr[1]) + f64::from(arr[c])) * 0.5) as f32;
        arr[lc] = ((f64::from(arr[lc - 1]) + f64::from(arr[lc + c])) * 0.5) as f32;
        arr[lr * c] = ((f64::from(arr[1 + lr * c]) + f64::from(arr[(lr - 1) * c])) * 0.5) as f32;
        arr[lc + lr * c] =
            ((f64::from(arr[lc - 1 + lr * c]) + f64::from(arr[lc + (lr - 1) * c])) * 0.5) as f32;
    }

    pub fn diffuse(
        dst: &mut [f32],
        src: &[f32],
        rate: f64,
        btype: u8,
        iters: u32,
        cols: u32,
        rows: u32,
    ) {
        let a = DT * rate * f64::from(cols) * f64::from(rows);
        if a * f64::from(iters) < 1e-3 {
            dst.copy_from_slice(src);
            Self::set_bounds(dst, btype, cols, rows);
            return;
        }
        let denom = 1.0 + 4.0 * a;
        let c = cols as usize;
        for _ in 0..iters {
            for j in 1..(rows as usize - 1) {
                let off = j * c;
                for i in 1..(cols as usize - 1) {
                    let p = off + i;
                    let sum = f64::from(dst[p - 1])
                        + f64::from(dst[p + 1])
                        + f64::from(dst[p - c])
                        + f64::from(dst[p + c]);
                    dst[p] = ((f64::from(src[p]) + a * sum) / denom) as f32;
                }
            }
            Self::set_bounds(dst, btype, cols, rows);
        }
    }

    pub fn project(
        u: &mut [f32],
        v: &mut [f32],
        pressure: &mut [f32],
        divergence: &mut [f32],
        cols: u32,
        rows: u32,
    ) {
        let h = 1.0 / f64::from(cols.max(rows));
        let inv_h = 1.0 / h;
        let c = cols as usize;

        pressure.fill(0.0);
        for j in 1..(rows as usize - 1) {
            let off = j * c;
            for i in 1..(cols as usize - 1) {
                let p = off + i;
                divergence[p] = (-0.5
                    * h
                    * (f64::from(u[p + 1]) - f64::from(u[p - 1]) + f64::from(v[p + c])
                        - f64::from(v[p - c]))) as f32;
                pressure[p] = 0.0;
            }
        }
        Self::set_bounds(divergence, 0, cols, rows);
        Self::set_bounds(pressure, 0, cols, rows);

        for _ in 0..PRESSURE_ITERS {
            for j in 1..(rows as usize - 1) {
                let off = j * c;
                for i in 1..(cols as usize - 1) {
                    let p = off + i;
                    pressure[p] = ((f64::from(pressure[p - 1])
                        + f64::from(pressure[p + 1])
                        + f64::from(pressure[p - c])
                        + f64::from(pressure[p + c])
                        + f64::from(divergence[p]))
                        * 0.25) as f32;
                }
            }
            Self::set_bounds(pressure, 0, cols, rows);
        }

        for j in 1..(rows as usize - 1) {
            let off = j * c;
            for i in 1..(cols as usize - 1) {
                let p = off + i;
                u[p] = (f64::from(u[p])
                    - 0.5 * inv_h * (f64::from(pressure[p + 1]) - f64::from(pressure[p - 1])))
                    as f32;
                v[p] = (f64::from(v[p])
                    - 0.5 * inv_h * (f64::from(pressure[p + c]) - f64::from(pressure[p - c])))
                    as f32;
            }
        }
        Self::set_bounds(u, 1, cols, rows);
        Self::set_bounds(v, 2, cols, rows);
    }

    pub fn advect(
        dst: &mut [f32],
        src: &[f32],
        vel_u: &[f32],
        vel_v: &[f32],
        cols: u32,
        rows: u32,
    ) {
        let step = DT * f64::from(cols * rows).sqrt();
        let c = cols as usize;
        let max_x = f64::from(cols) - 1.5;
        let max_y = f64::from(rows) - 1.5;

        for j in 1..(rows as usize - 1) {
            let off = j * c;
            for i in 1..(cols as usize - 1) {
                let p = off + i;
                let x = clamp_f64(f64::from(i as u32) - step * f64::from(vel_u[p]), 0.5, max_x);
                let y = clamp_f64(f64::from(j as u32) - step * f64::from(vel_v[p]), 0.5, max_y);
                dst[p] = bilin(src, cols, rows, x, y) as f32;
            }
        }
        Self::set_bounds(dst, 0, cols, rows);
    }

    fn trail_distance(trail: &[TrailSeg]) -> f64 {
        trail
            .iter()
            .map(|segment| {
                let dx = segment.x1 - segment.x0;
                let dy = segment.y1 - segment.y0;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }

    fn pointer_segment(trail: &[TrailSeg]) -> Option<(f64, f64, f64, f64, f64)> {
        let first = trail.first()?;
        let last = trail.last()?;
        let dx = last.x1 - first.x0;
        let dy = last.y1 - first.y0;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 0.01 {
            return None;
        }
        Some((first.x0, first.y0, dx, dy, dist))
    }

    pub fn add_mouse_forces(&mut self, trail: &[TrailSeg], mouse_active: bool) {
        if !mouse_active {
            return;
        }
        let speed = Self::trail_distance(trail);
        let Some((x0, y0, dx, dy, dist)) = Self::pointer_segment(trail) else {
            return;
        };
        let boost = (1.0 + speed * MOUSE_SPEED_FORCE).min(MOUSE_FORCE_CAP);
        let span = dist.min(MAX_PUSH_CELLS) / dist;
        let start_x = x0 + dx * (1.0 - span);
        let start_y = y0 + dy * (1.0 - span);
        let dx = dx * span;
        let dy = dy * span;
        let steps = 1.0_f64.max((dx * dx + dy * dy).sqrt().ceil());
        let sx = dx / steps;
        let sy = dy / steps;

        for s in 1..=(steps as i64) {
            let gi = clamp_i64(
                (start_x + sx * s as f64).round() as i64,
                1,
                self.cols as i64 - 2,
            );
            let gj = clamp_i64(
                (start_y + sy * s as f64).round() as i64,
                1,
                self.rows as i64 - 2,
            );
            let p = self.at(gi, gj);
            self.u[p] = (f64::from(self.u[p]) + sx * FORCE_STRENGTH * boost) as f32;
            self.v[p] = (f64::from(self.v[p]) + sy * FORCE_STRENGTH * boost) as f32;
        }
    }

    pub fn inject_mouse_density(&mut self, trail: &mut Vec<TrailSeg>, mouse_active: bool) {
        if !mouse_active {
            return;
        }
        let Some((x0, y0, dx, dy, dist)) = Self::pointer_segment(trail) else {
            trail.clear();
            return;
        };
        let steps = 1.0_f64.max(dist.ceil());
        let sx = dx / steps;
        let sy = dy / steps;
        let amount = DENSITY_PER_FRAME / steps.sqrt();
        let brush = BRUSH_RADIUS;

        for s in 1..=(steps as i64) {
            let gi = clamp_i64((x0 + sx * s as f64).round() as i64, 1, self.cols as i64 - 2);
            let gj = clamp_i64((y0 + sy * s as f64).round() as i64, 1, self.rows as i64 - 2);
            for dj in -brush..=brush {
                for di in -brush..=brush {
                    let d2 = di * di + dj * dj;
                    if d2 > brush * brush {
                        continue;
                    }
                    let ni = clamp_i64(gi + i64::from(di), 1, self.cols as i64 - 2);
                    let nj = clamp_i64(gj + i64::from(dj), 1, self.rows as i64 - 2);
                    let falloff = 1.0 - (f64::from(d2).sqrt() / f64::from(brush + 1));
                    let at = self.at(ni, nj);
                    self.dens[at] = (f64::from(self.dens[at]) + amount * falloff).min(1.0) as f32;
                }
            }
        }
        trail.clear();
    }

    pub fn add_ambient_velocity(&mut self, rng: &mut Rng) {
        for i in 0..self.total {
            self.u[i] = (f64::from(self.u[i]) + (rng.next_f64() - 0.5) * AMBIENT_FORCE) as f32;
            self.v[i] = (f64::from(self.v[i]) + (rng.next_f64() - 0.5) * AMBIENT_FORCE) as f32;
        }
    }

    pub fn add_ambient_density(&mut self, rng: &mut Rng) {
        for i in 0..self.total {
            let value = f64::from(self.dens[i]) + (rng.next_f64() - 0.5) * AMBIENT_DENSITY;
            self.dens[i] = value.max(0.0) as f32;
        }
    }

    pub fn add_vorticity(&mut self) {
        if !self.curl.is_empty() {
            let c = self.cols as usize;
            for j in 1..(self.rows as usize - 1) {
                let off = j * c;
                for i in 1..(self.cols as usize - 1) {
                    let p = off + i;
                    let curl = 0.5
                        * ((f64::from(self.v[p + 1]) - f64::from(self.v[p - 1]))
                            - (f64::from(self.u[p + c]) - f64::from(self.u[p - c])));
                    self.curl[p] = curl as f32;
                    self.curl_abs[p] = curl.abs() as f32;
                }
            }
            Self::set_bounds(&mut self.curl, 0, self.cols, self.rows);
            Self::set_bounds(&mut self.curl_abs, 0, self.cols, self.rows);

            let gain = VORTICITY * DT;
            let cap = VORTICITY_CAP;
            let c = self.cols as usize;
            for j in 1..(self.rows as usize - 1) {
                let off = j * c;
                for i in 1..(self.cols as usize - 1) {
                    let p = off + i;
                    let gx = f64::from(self.curl_abs[p + 1]) - f64::from(self.curl_abs[p - 1]);
                    let gy = f64::from(self.curl_abs[p + c]) - f64::from(self.curl_abs[p - c]);
                    let len = (gx * gx + gy * gy).sqrt();
                    if len < 1e-6 {
                        continue;
                    }
                    let nx = gx / len;
                    let ny = gy / len;
                    let curl = f64::from(self.curl[p]);
                    let mut fx = gain * ny * curl;
                    let mut fy = -gain * nx * curl;
                    if fx > cap {
                        fx = cap;
                    } else if fx < -cap {
                        fx = -cap;
                    }
                    if fy > cap {
                        fy = cap;
                    } else if fy < -cap {
                        fy = -cap;
                    }
                    self.u[p] = (f64::from(self.u[p]) + fx) as f32;
                    self.v[p] = (f64::from(self.v[p]) + fy) as f32;
                }
            }
            Self::set_bounds(&mut self.u, 1, self.cols, self.rows);
            Self::set_bounds(&mut self.v, 2, self.cols, self.rows);
        }
    }

    pub fn step(
        &mut self,
        trail: &mut Vec<TrailSeg>,
        mouse_active: bool,
        ambient: bool,
        rng: &mut Rng,
    ) {
        self.add_mouse_forces(trail, mouse_active);
        if ambient {
            self.add_ambient_velocity(rng);
        }

        {
            let (u, v, u_prev, v_prev, pressure, divergence) = (
                &mut self.u,
                &mut self.v,
                &mut self.u_prev,
                &mut self.v_prev,
                &mut self.pressure,
                &mut self.divergence,
            );
            Self::diffuse(u_prev, u, VISCOSITY, 1, DIFFUSE_ITERS, self.cols, self.rows);
            Self::diffuse(v_prev, v, VISCOSITY, 2, DIFFUSE_ITERS, self.cols, self.rows);
            Self::project(u_prev, v_prev, pressure, divergence, self.cols, self.rows);
        }
        std::mem::swap(&mut self.u, &mut self.u_prev);
        std::mem::swap(&mut self.v, &mut self.v_prev);

        {
            let (u, v, u_prev, v_prev) =
                (&mut self.u, &mut self.v, &mut self.u_prev, &mut self.v_prev);
            Self::advect(u_prev, u, u, v, self.cols, self.rows);
            Self::advect(v_prev, v, u, v, self.cols, self.rows);
        }

        self.add_vorticity();

        {
            let (u_prev, v_prev, pressure, divergence) = (
                &mut self.u_prev,
                &mut self.v_prev,
                &mut self.pressure,
                &mut self.divergence,
            );
            Self::project(u_prev, v_prev, pressure, divergence, self.cols, self.rows);
        }
        std::mem::swap(&mut self.u, &mut self.u_prev);
        std::mem::swap(&mut self.v, &mut self.v_prev);

        {
            let (u, v, dens, dens_prev) = (
                &mut self.u,
                &mut self.v,
                &mut self.dens,
                &mut self.dens_prev,
            );
            Self::advect(dens_prev, dens, u, v, self.cols, self.rows);
        }
        std::mem::swap(&mut self.dens, &mut self.dens_prev);

        self.inject_mouse_density(trail, mouse_active);
        if ambient {
            self.add_ambient_density(rng);
        }

        for i in 0..self.total {
            self.dens[i] = (f64::from(self.dens[i]) * DISSIPATION) as f32;
            self.u[i] = (f64::from(self.u[i]) * VEL_DISSIPATION) as f32;
            self.v[i] = (f64::from(self.v[i]) * VEL_DISSIPATION) as f32;
            if self.dens[i] < 0.005 {
                self.dens[i] = 0.0;
            }
        }
    }

    pub fn apply_masks(&mut self, masks: &[Mask]) {
        if self.cols < 2 || self.rows < 2 {
            return;
        }
        for mask in masks {
            let r1 = ((mask.row + mask.rows).min(f64::from(self.rows - 1))).ceil() as i64;
            let c1 = ((mask.col + mask.cols).min(f64::from(self.cols - 1))).ceil() as i64;
            let row0 = (mask.row.max(0.0)).floor() as i64;
            let col0 = (mask.col.max(0.0)).floor() as i64;
            let fade = if mask.fade >= 0.0 { mask.fade } else { 2.0 };

            for j in row0..r1 {
                let edge_j = (j as f64 - mask.row).min((r1 - 1) as f64 - j as f64);
                for i in col0..c1 {
                    let edge_i = (i as f64 - mask.col).min((c1 - 1) as f64 - i as f64);
                    let edge = edge_j.min(edge_i);
                    let keep = if fade > 0.0 {
                        (edge / fade).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let p = self.at(i, j);
                    self.dens[p] = (f64::from(self.dens[p]) * keep) as f32;
                }
            }
        }
    }

    pub fn droplet(&mut self, ci: f64, cj: f64, amount: f64, radius: f64) {
        let r = radius;
        for dj in -(r as i64)..=(r as i64) {
            for di in -(r as i64)..=(r as i64) {
                let dist = ((di * di + dj * dj) as f64).sqrt();
                if dist > r {
                    continue;
                }
                let falloff = ((dist / (r + 1.0)) * (std::f64::consts::PI / 2.0)).cos();
                let i = clamp_i64((ci + di as f64).round() as i64, 1, self.cols as i64 - 2);
                let j = clamp_i64((cj + dj as f64).round() as i64, 1, self.rows as i64 - 2);
                let p = self.at(i, j);
                self.dens[p] = (f64::from(self.dens[p]) + amount * falloff).min(1.0) as f32;
            }
        }
    }

    pub fn vortex(&mut self, ci: f64, cj: f64, strength: f64) {
        let radius = 7_i64;
        for dj in -radius..=radius {
            for di in -radius..=radius {
                let dist = ((di * di + dj * dj) as f64).sqrt();
                if dist > radius as f64 || dist < 0.4 {
                    continue;
                }
                let falloff = 1.0 - dist / radius as f64;
                let i = clamp_i64((ci + di as f64).round() as i64, 1, self.cols as i64 - 2);
                let j = clamp_i64((cj + dj as f64).round() as i64, 1, self.rows as i64 - 2);
                let p = self.at(i, j);
                self.u[p] =
                    (f64::from(self.u[p]) + (-dj as f64 / dist) * strength * falloff) as f32;
                self.v[p] = (f64::from(self.v[p]) + (di as f64 / dist) * strength * falloff) as f32;
                self.dens[p] = (f64::from(self.dens[p]) + 0.06 * falloff).min(1.0) as f32;
            }
        }
    }

    pub fn inject_at(&mut self, gi: f64, gj: f64, intensity: f64) {
        if self.cols < 3 || self.rows < 3 {
            return;
        }
        let brush = 2_i64;
        for dj in -brush..=brush {
            for di in -brush..=brush {
                let d2 = di * di + dj * dj;
                if d2 > brush * brush {
                    continue;
                }
                let f = (-(d2 as f64) / (brush as f64 * 0.7)).exp();
                let ni = clamp_i64(gi as i64 + di, 1, self.cols as i64 - 2);
                let nj = clamp_i64(gj as i64 + dj, 1, self.rows as i64 - 2);
                let p = self.at(ni, nj);
                self.dens[p] = (f64::from(self.dens[p]) + intensity * f).min(1.0) as f32;
            }
        }
    }
}

#[inline]
pub fn clamp_f64(value: f64, lo: f64, hi: f64) -> f64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

#[inline]
pub fn clamp_i64(value: i64, lo: i64, hi: i64) -> i64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

#[inline]
fn mix4(a: f32, b: f32, c: f32, d: f32, ri: f64, rj: f64) -> f64 {
    let top = f64::from(a) + (f64::from(b) - f64::from(a)) * ri;
    let bot = f64::from(c) + (f64::from(d) - f64::from(c)) * ri;
    top + (bot - top) * rj
}

pub fn bilin(arr: &[f32], cols: u32, rows: u32, gi: f64, gj: f64) -> f64 {
    let fi = gi.floor();
    let fj = gj.floor();
    let ri = gi - fi;
    let rj = gj - fj;
    let i0 = clamp_i64(fi as i64, 0, i64::from(cols) - 1);
    let i1 = clamp_i64(fi as i64 + 1, 0, i64::from(cols) - 1);
    let j0 = clamp_i64(fj as i64, 0, i64::from(rows) - 1);
    let j1 = clamp_i64(fj as i64 + 1, 0, i64::from(rows) - 1);

    let top = f64::from(arr[Fluid::idx(cols, i0, j0)])
        + (f64::from(arr[Fluid::idx(cols, i1, j0)]) - f64::from(arr[Fluid::idx(cols, i0, j0)]))
            * ri;
    let bot = f64::from(arr[Fluid::idx(cols, i0, j1)])
        + (f64::from(arr[Fluid::idx(cols, i1, j1)]) - f64::from(arr[Fluid::idx(cols, i0, j1)]))
            * ri;
    top + (bot - top) * rj
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_grid_stays_finite() {
        let mut fluid = Fluid::new(24, 16);
        let mut rng = Rng::new(1);
        let mut trail = Vec::new();
        fluid.droplet(12.0, 8.0, 0.9, 4.0);
        fluid.vortex(12.0, 8.0, 0.4);
        for _ in 0..20 {
            fluid.step(&mut trail, true, true, &mut rng);
        }
        assert!(fluid.dens.iter().all(|value| value.is_finite()));
        assert!(fluid.u.iter().all(|value| value.is_finite()));
        assert!(fluid.v.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn resize_carries_the_field() {
        let mut fluid = Fluid::new(40, 30);
        let center = fluid.at(20, 15);
        fluid.dens[center] = 0.8;
        fluid.resize(80, 60);
        assert!(fluid.dens.iter().any(|value| *value > 0.0));
        assert_eq!(fluid.dens_prev.len(), fluid.total);
    }

    #[test]
    fn faster_pointer_motion_adds_more_force() {
        let mut slow = Fluid::new(40, 30);
        let mut fast = Fluid::new(40, 30);
        slow.add_mouse_forces(
            &[TrailSeg {
                x0: 5.0,
                y0: 5.0,
                x1: 6.0,
                y1: 5.0,
            }],
            true,
        );
        fast.add_mouse_forces(
            &[TrailSeg {
                x0: 5.0,
                y0: 5.0,
                x1: 20.0,
                y1: 5.0,
            }],
            true,
        );
        let slow_force = slow.u[slow.at(6, 5)].abs();
        let fast_force = fast.u[fast.at(6, 5)].abs();
        assert!(fast_force > slow_force * 2.0);
    }

    #[test]
    fn masks_thin_density_at_the_edges() {
        let mut fluid = Fluid::new(20, 20);
        for value in fluid.dens.iter_mut() {
            *value = 1.0;
        }
        fluid.apply_masks(&[Mask {
            row: 5.0,
            col: 5.0,
            rows: 6.0,
            cols: 6.0,
            fade: 2.0,
        }]);
        assert_eq!(fluid.dens[fluid.at(5, 5)], 0.0);
        assert!(fluid.dens[fluid.at(8, 8)] > 0.0);
    }
}
