/// Fluid simulation using the Jos Stam method
pub struct FluidSim {
    pub cols: u32,
    pub rows: u32,
    pub density: Vec<f32>,
    pub prev_density: Vec<f32>,
    pub velocity_u: Vec<f32>,
    pub velocity_v: Vec<f32>,
    pub prev_u: Vec<f32>,
    pub prev_v: Vec<f32>,
    divergence: Vec<f32>,
    pressure: Vec<f32>,
    viscosity: f32,
    dt: f32,
    diffuse_iters: u32,
    pressure_iters: u32,
    dissipation: f32,
    vel_dissipation: f32,
}

impl FluidSim {
    pub fn new(cols: u32, rows: u32) -> Self {
        let total = (cols * rows) as usize;
        FluidSim {
            cols, rows,
            density: vec![0.0; total],
            prev_density: vec![0.0; total],
            velocity_u: vec![0.0; total],
            velocity_v: vec![0.0; total],
            prev_u: vec![0.0; total],
            prev_v: vec![0.0; total],
            divergence: vec![0.0; total],
            pressure: vec![0.0; total],
            viscosity: 0.000_000_2,
            dt: 0.017,
            diffuse_iters: 10,
            pressure_iters: 25,
            dissipation: 0.992,
            vel_dissipation: 0.997,
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.cols = cols; self.rows = rows;
        let total = (cols * rows) as usize;
        self.density.resize(total, 0.0);
        self.prev_density.resize(total, 0.0);
        self.velocity_u.resize(total, 0.0);
        self.velocity_v.resize(total, 0.0);
        self.prev_u.resize(total, 0.0);
        self.prev_v.resize(total, 0.0);
        self.divergence.resize(total, 0.0);
        self.pressure.resize(total, 0.0);
    }

    fn idx(&self, i: u32, j: u32) -> usize { (i + j * self.cols) as usize }

    pub fn add_density(&mut self, x: u32, y: u32, amount: f32) {
        let i = self.idx(x.min(self.cols - 1), y.min(self.rows - 1));
        if i < self.density.len() { self.density[i] += amount; }
    }

    pub fn add_force(&mut self, x: u32, y: u32, fx: f32, fy: f32) {
        let i = self.idx(x.min(self.cols - 1), y.min(self.rows - 1));
        if i < self.velocity_u.len() { self.velocity_u[i] += fx; self.velocity_v[i] += fy; }
    }

    pub fn step(&mut self) {
        let dt = self.dt;
        let cols = self.cols;
        let rows = self.rows;
        let visc = self.viscosity;
        let di = self.diffuse_iters;
        let pi = self.pressure_iters;
        let vd = self.vel_dissipation;
        let dd = self.dissipation;

        // Work with borrowed slices to avoid borrow conflicts
        let (u, pu, v, pv) = (&mut self.velocity_u, &self.prev_u, &mut self.velocity_v, &self.prev_v);
        let (div, pres) = (&mut self.divergence, &mut self.pressure);
        Self::diffuse(u, pu, dt, visc, cols, rows, 1, di);
        Self::diffuse(v, pv, dt, visc, cols, rows, 2, di);
        Self::project(u, v, div, pres, cols, rows, pi);

        let (dst_u, src_u, dst_v, src_v) = (&mut self.prev_u, &self.velocity_u, &mut self.prev_v, &self.velocity_v);
        Self::advect(dst_u, src_u, src_u, src_v, dt, cols, rows, vd);
        Self::advect(dst_v, src_v, src_u, src_v, dt, cols, rows, vd);
        Self::project(dst_u, dst_v, &mut self.divergence, &mut self.pressure, cols, rows, pi);

        std::mem::swap(&mut self.velocity_u, &mut self.prev_u);
        std::mem::swap(&mut self.velocity_v, &mut self.prev_v);

        let (d_u, d_v) = (&self.velocity_u, &self.velocity_v);
        Self::diffuse(&mut self.density, &self.prev_density, dt, 0.0, cols, rows, 0, di);
        Self::advect(&mut self.prev_density, &self.density, d_u, d_v, dt, cols, rows, dd);
        std::mem::swap(&mut self.density, &mut self.prev_density);
    }

    // ── Static helper: set boundary conditions ──
    fn set_bounds(arr: &mut [f32], btype: u32, cols: u32, rows: u32) {
        let lc = (cols - 1) as usize;
        let lr = (rows - 1) as usize;
        let c = cols as usize;

        for i in 1..lc {
            arr[i] = if btype == 2 { -arr[i + c] } else { arr[i + c] };
            arr[i + lr * c] = if btype == 2 { -arr[i + (lr - 1) * c] } else { arr[i + (lr - 1) * c] };
        }
        for j in 1..lr {
            arr[j * c] = if btype == 1 { -arr[1 + j * c] } else { arr[1 + j * c] };
            arr[lc + j * c] = if btype == 1 { -arr[lc - 1 + j * c] } else { arr[lc - 1 + j * c] };
        }
        arr[0] = (arr[1] + arr[c]) * 0.5;
        arr[lc] = (arr[lc - 1] + arr[lc + c]) * 0.5;
        arr[lr * c] = (arr[1 + lr * c] + arr[(lr - 1) * c]) * 0.5;
        arr[lc + lr * c] = (arr[lc - 1 + lr * c] + arr[lc + (lr - 1) * c]) * 0.5;
    }

    // ── Static helpers: diffusion, advection, projection ──
    fn diffuse(dst: &mut [f32], src: &[f32], dt: f32, rate: f32, cols: u32, rows: u32, btype: u32, iters: u32) {
        let a = dt * rate * (cols as f32) * (rows as f32);
        let c = cols as usize;
        for _ in 0..iters {
            for j in 1..(rows - 1) {
                for i in 1..(cols - 1) {
                    let n = i as usize + j as usize * c;
                    dst[n] = (src[n] + a * (dst[n - 1] + dst[n + 1] + dst[n - c] + dst[n + c])) / (1.0 + 4.0 * a);
                }
            }
            Self::set_bounds(dst, btype, cols, rows);
        }
    }

    fn advect(dst: &mut [f32], src: &[f32], u: &[f32], v: &[f32], dt: f32, cols: u32, rows: u32, dissipation: f32) {
        let dt_x = dt * (cols - 2) as f32;
        let dt_y = dt * (rows - 2) as f32;
        let c = cols as usize;
        for j in 1..(rows - 1) {
            for i in 1..(cols - 1) {
                let n = i as usize + j as usize * c;
                let mut x = i as f32 - dt_x * u[n];
                let mut y = j as f32 - dt_y * v[n];
                if x < 0.5 { x = 0.5; }
                if x > (cols - 1) as f32 - 0.5 { x = (cols - 1) as f32 - 0.5; }
                if y < 0.5 { y = 0.5; }
                if y > (rows - 1) as f32 - 0.5 { y = (rows - 1) as f32 - 0.5; }
                let i0 = x.floor() as usize; let i1 = (i0 + 1).min(c - 1);
                let j0 = y.floor() as usize; let j1 = (j0 + 1).min(rows as usize - 1);
                let s1 = x - i0 as f32; let s0 = 1.0 - s1;
                let t1 = y - j0 as f32; let t0 = 1.0 - t1;
                dst[n] = dissipation * (
                    s0 * (t0 * src[i0 + j0 * c] + t1 * src[i0 + j1 * c]) +
                    s1 * (t0 * src[i1 + j0 * c] + t1 * src[i1 + j1 * c])
                );
            }
        }
        Self::set_bounds(dst, 0, cols, rows);
    }

    fn project(u: &mut [f32], v: &mut [f32], div: &mut [f32], p: &mut [f32], cols: u32, rows: u32, iters: u32) {
        let c = cols as usize;
        for val in p.iter_mut() { *val = 0.0; }
        let scale = (cols.max(rows) as f32).max(1.0);
        for j in 1..(rows - 1) {
            for i in 1..(cols - 1) {
                let n = i as usize + j as usize * c;
                div[n] = -0.5 * (u[n + 1] - u[n - 1] + v[n + c] - v[n - c]) / scale;
            }
        }
        Self::set_bounds(div, 0, cols, rows);
        for _ in 0..iters {
            for j in 1..(rows - 1) {
                for i in 1..(cols - 1) {
                    let n = i as usize + j as usize * c;
                    p[n] = (div[n] + p[n - 1] + p[n + 1] + p[n - c] + p[n + c]) * 0.25;
                }
            }
            Self::set_bounds(p, 0, cols, rows);
        }
        for j in 1..(rows - 1) {
            for i in 1..(cols - 1) {
                let n = i as usize + j as usize * c;
                u[n] -= 0.5 * (p[n + 1] - p[n - 1]) * cols as f32;
                v[n] -= 0.5 * (p[n + c] - p[n - c]) * rows as f32;
            }
        }
        Self::set_bounds(u, 1, cols, rows);
        Self::set_bounds(v, 2, cols, rows);
    }
}
