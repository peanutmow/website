use crate::fluid::{clamp_f64, clamp_i64, Fluid, Mask, TrailSeg};
use crate::render::{render_rows, RenderMode};
use crate::rng::Rng;

const TRAIL_MAX: usize = 512;
const KEY_BUFFER_MAX: usize = 20;
const BIO: &str = "artist & developer";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EggMode {
    None,
    Blank,
    Answer,
    Cowsay,
}

#[derive(Clone, Debug)]
struct Overlay {
    row: i64,
    col: i64,
    ch: char,
    frames: i32,
}

#[derive(Clone, Debug, Default)]
struct Layout {
    rows: u32,
    cols: u32,
    name_row: i64,
    name_col: i64,
    btn_row: i64,
    btn_col: i64,
    bio_row: i64,
    bio_col: i64,
    btn_str: String,
    bio: String,
    buttons: Vec<String>,
    greeting: String,
}

pub struct Sim {
    pub fluid: Fluid,
    mode: RenderMode,
    text_visible: bool,
    greeting: Vec<char>,
    buttons: Vec<String>,
    greeting_revealed: usize,
    greeting_complete: bool,
    typewriter: bool,
    page_hooks: bool,
    easter_eggs: bool,
    reduced_motion: bool,
    mx: f64,
    my: f64,
    mouse_active: bool,
    trail: Vec<TrailSeg>,
    masks: Vec<Mask>,
    overlays: Vec<Overlay>,
    egg_overlays: Vec<Overlay>,
    egg_mode: EggMode,
    key_buffer: Vec<char>,
    spectrum: Vec<u8>,
    spectrum_on: bool,
    rng: Rng,
    layout: Option<Layout>,
}

impl Sim {
    pub fn cols(&self) -> u32 {
        self.fluid.cols
    }

    pub fn rows(&self) -> u32 {
        self.fluid.rows
    }

    pub fn dens_ptr(&self) -> u32 {
        self.fluid.dens.as_ptr() as u32
    }

    pub fn u_ptr(&self) -> u32 {
        self.fluid.u.as_ptr() as u32
    }

    pub fn v_ptr(&self) -> u32 {
        self.fluid.v.as_ptr() as u32
    }

    pub fn new(cols: u32, rows: u32, seed: u32) -> Self {
        Sim {
            fluid: Fluid::new(cols, rows),
            mode: RenderMode::Ascii,
            text_visible: true,
            greeting: "Hi, I'm Alice".chars().collect(),
            buttons: vec![
                "Socials".to_string(),
                "Gallery".to_string(),
                "Blog".to_string(),
                "Projects".to_string(),
            ],
            greeting_revealed: 0,
            greeting_complete: false,
            typewriter: true,
            page_hooks: false,
            easter_eggs: false,
            reduced_motion: false,
            mx: -1.0,
            my: -1.0,
            mouse_active: false,
            trail: Vec::new(),
            masks: Vec::new(),
            overlays: Vec::new(),
            egg_overlays: Vec::new(),
            egg_mode: EggMode::None,
            key_buffer: Vec::new(),
            spectrum: Vec::new(),
            spectrum_on: false,
            rng: Rng::new(seed),
            layout: None,
        }
    }

    pub fn configure(
        &mut self,
        greeting: &str,
        buttons: &[String],
        mode: &str,
        text: bool,
        typewriter: bool,
        page_hooks: bool,
        easter_eggs: bool,
        reduced_motion: bool,
    ) {
        self.greeting = greeting.chars().collect();
        self.buttons = buttons.to_vec();
        self.mode = RenderMode::parse(mode);
        self.text_visible = text;
        self.typewriter = typewriter;
        self.page_hooks = page_hooks;
        self.easter_eggs = easter_eggs;
        self.reduced_motion = reduced_motion;

        if !self.typewriter {
            self.greeting_revealed = self.greeting.len();
            self.greeting_complete = true;
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.fluid.resize(cols, rows);
        self.mx = -1.0;
        self.my = -1.0;
        self.trail.clear();
        self.compute_text_overlays();
    }

    pub fn mode(&self) -> &'static str {
        self.mode.as_str()
    }

    pub fn set_mode(&mut self, mode: &str, text: Option<bool>) {
        self.mode = RenderMode::parse(mode);
        if let Some(text) = text {
            self.text_visible = text;
        }
    }

    pub fn set_text_visible(&mut self, visible: bool) {
        self.text_visible = visible;
    }

    pub fn ramp(&self) -> &'static str {
        crate::render::RAMP
    }

    pub fn set_spectrum(&mut self, bins: &[u8]) {
        if self.spectrum.len() == bins.len() {
            self.spectrum.copy_from_slice(bins);
        } else {
            self.spectrum = bins.to_vec();
        }
    }

    pub fn set_spectrum_visible(&mut self, visible: bool) {
        self.spectrum_on = visible;
        if !visible {
            self.spectrum.clear();
        }
    }

    pub fn pointer_move(&mut self, gx: f64, gy: f64) {
        if self.mx >= 0.0 && self.my >= 0.0 {
            let dx = gx - self.mx;
            let dy = gy - self.my;
            if dx * dx + dy * dy > 1e-6 && self.trail.len() < TRAIL_MAX {
                self.trail.push(TrailSeg {
                    x0: self.mx,
                    y0: self.my,
                    x1: gx,
                    y1: gy,
                });
            }
        }
        self.mx = gx;
        self.my = gy;
        self.mouse_active = true;
    }

    pub fn pointer_down(&mut self, gx: Option<f64>, gy: Option<f64>) {
        if self.mx < 0.0 {
            if let (Some(gx), Some(gy)) = (gx, gy) {
                self.mx = gx;
                self.my = gy;
            }
        }
        self.mouse_active = true;
    }

    pub fn pointer_leave(&mut self) {
        self.mouse_active = false;
        self.mx = -1.0;
        self.my = -1.0;
        self.trail.clear();
    }

    pub fn pointer_active(&mut self, active: bool) {
        self.mouse_active = active;
    }

    pub fn inject_at(&mut self, gi: f64, gj: f64, intensity: f64) {
        self.fluid.inject_at(gi, gj, intensity);
    }

    pub fn step(&mut self) {
        self.fluid.step(
            &mut self.trail,
            self.mouse_active,
            !self.reduced_motion,
            &mut self.rng,
        );
        self.fluid.apply_masks(&self.masks);
    }

    pub fn render(&mut self, hide_text: bool) -> String {
        let rows = render_rows(&self.fluid, self.mode, &self.masks);

        if self.text_visible && !hide_text {
            self.compute_text_overlays();
            if !self.overlays.is_empty() {
                let mut grid: Vec<Vec<char>> =
                    rows.iter().map(|row| row.chars().collect()).collect();
                for overlay in &self.overlays {
                    if overlay.row < 0
                        || overlay.col < 0
                        || overlay.row >= i64::from(self.fluid.rows)
                        || overlay.col >= i64::from(self.fluid.cols)
                    {
                        continue;
                    }
                    let row = overlay.row as usize;
                    let col = overlay.col as usize;
                    if let Some(cell) = grid[row].get_mut(col) {
                        *cell = overlay.ch;
                    }
                }
                return grid
                    .into_iter()
                    .map(|row| row.into_iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }

        rows.join("\n")
    }

    fn compute_text_overlays(&mut self) {
        self.egg_overlays.retain(|overlay| overlay.frames > 0);
        for overlay in &mut self.egg_overlays {
            overlay.frames -= 1;
        }
        self.overlays.clear();

        if !self.egg_overlays.is_empty() || self.egg_mode != EggMode::None {
            self.overlays.extend(self.egg_overlays.iter().cloned());
            return;
        }

        if self.fluid.cols == 0 || self.fluid.rows == 0 {
            return;
        }

        let name: Vec<char> = self.greeting.clone();
        let btn_str = self.buttons.join("  ");
        let btn_chars: Vec<char> = btn_str.chars().collect();
        let bio_chars: Vec<char> = BIO.chars().collect();
        let cols = self.fluid.cols as i64;
        let rows = self.fluid.rows as i64;

        if self.spectrum_on {
            self.push_spectrum(cols, rows);
        }

        let name_row = (rows as f64 * 0.84).floor() as i64;
        let name_col = ((cols - name.len() as i64) as f64 / 2.0).floor() as i64;
        let reveal = self.greeting_revealed.min(name.len());
        for (index, ch) in name.iter().take(reveal).enumerate() {
            self.overlays.push(Overlay {
                row: name_row,
                col: name_col + index as i64,
                ch: *ch,
                frames: 0,
            });
        }
        if !self.greeting_complete && reveal < name.len() {
            self.overlays.push(Overlay {
                row: name_row,
                col: name_col + reveal as i64,
                ch: '_',
                frames: 0,
            });
        }

        let btn_row = (rows as f64 * 0.89).floor() as i64;
        let btn_col = ((cols - btn_chars.len() as i64) as f64 / 2.0).floor() as i64;
        for (index, ch) in btn_chars.iter().enumerate() {
            self.overlays.push(Overlay {
                row: btn_row,
                col: btn_col + index as i64,
                ch: *ch,
                frames: 0,
            });
        }

        let bio_row = (rows as f64 * 0.94).floor() as i64;
        let bio_col = ((cols - bio_chars.len() as i64) as f64 / 2.0).floor() as i64;
        for (index, ch) in bio_chars.iter().enumerate() {
            self.overlays.push(Overlay {
                row: bio_row,
                col: bio_col + index as i64,
                ch: *ch,
                frames: 0,
            });
        }

        self.layout = Some(Layout {
            rows: self.fluid.rows,
            cols: self.fluid.cols,
            name_row,
            name_col,
            btn_row,
            btn_col,
            bio_row,
            bio_col,
            btn_str,
            bio: BIO.to_string(),
            buttons: self.buttons.clone(),
            greeting: self.greeting.iter().collect(),
        });
    }

    fn push_spectrum(&mut self, cols: i64, rows: i64) {
        if self.spectrum.is_empty() || cols <= 0 || rows <= 0 {
            return;
        }
        let ramp: Vec<char> = crate::render::RAMP.chars().skip(1).collect();
        if ramp.is_empty() {
            return;
        }
        let band = ((rows as f64 * 0.09).round() as i64).clamp(1, 8).min(rows);
        let width = ((cols as f64 * 0.64).round() as i64).clamp(1, cols);
        let start = (cols - width) / 2;
        let bins = self.spectrum.len();

        for x in 0..width {
            let position = (x + 1) as f64 / width as f64;
            let bin = ((position.powf(1.5) * bins as f64).floor() as usize).min(bins - 1);
            let level = (self.spectrum[bin] as f64 / 255.0).powf(1.3);
            let height = level * band as f64;
            if height < 0.5 {
                continue;
            }
            for row in 0..band {
                let distance = (band - 1 - row) as f64;
                if distance >= height {
                    continue;
                }
                let depth = if height <= 1.0 {
                    1.0
                } else {
                    1.0 - distance / height
                };
                let index = ((depth * (ramp.len() - 1) as f64).floor() as usize).min(ramp.len() - 1);
                self.overlays.push(Overlay {
                    row,
                    col: start + x,
                    ch: ramp[index],
                    frames: 0,
                });
            }
        }
    }

    pub fn typewriter_start(&mut self) {
        self.greeting_revealed = 0;
        self.greeting_complete = false;
    }

    pub fn typewriter_tick(&mut self) {
        if self.greeting_complete {
            return;
        }
        self.greeting_revealed += 1;
        if self.greeting_revealed >= self.greeting.len() {
            self.greeting_revealed = self.greeting.len();
            self.greeting_complete = true;
        }
    }

    pub fn typewriter_done(&mut self) {
        self.greeting_revealed = self.greeting.len();
        self.greeting_complete = true;
    }

    pub fn key(&mut self, key: &str) -> &'static str {
        if !self.easter_eggs {
            return "";
        }
        let mut chars = key.chars();
        let Some(ch) = chars.next() else {
            return "";
        };
        if chars.next().is_some() {
            return "";
        }
        self.key_buffer.push(ch.to_ascii_lowercase());
        if self.key_buffer.len() > KEY_BUFFER_MAX {
            self.key_buffer.remove(0);
        }

        let buffer: String = self.key_buffer.iter().collect();
        if buffer == "42" {
            self.key_buffer.clear();
            self.trigger_42();
            return "42";
        }
        if buffer.ends_with("about:blank") {
            self.key_buffer.clear();
            self.trigger_about_blank();
            return "blank";
        }
        if buffer.ends_with("cowsay") {
            self.key_buffer.clear();
            self.trigger_cowsay();
            return "cowsay";
        }
        ""
    }

    pub fn clear_egg(&mut self) {
        self.egg_mode = EggMode::None;
        self.egg_overlays.clear();
    }

    pub fn trigger_about_blank(&mut self) {
        if self.egg_mode == EggMode::Blank {
            return;
        }
        self.egg_mode = EggMode::Blank;
        self.overlays.clear();
        for value in self.fluid.dens.iter_mut() {
            *value = 0.0;
        }
        self.egg_overlays.clear();

        let cols = self.fluid.cols as i64;
        let rows = self.fluid.rows as i64;
        let row = rows / 2;
        push_centered(&mut self.egg_overlays, row, cols, "REMOVE", 300);
        push_centered(&mut self.egg_overlays, row + 2, cols, "TEST", 300);
    }

    fn trigger_42(&mut self) {
        if self.egg_mode != EggMode::None {
            return;
        }
        self.egg_mode = EggMode::Answer;
        self.overlays.clear();

        let msg = "The answer to life, the universe, and everything.";
        let cols = self.fluid.cols as i64;
        let rows = self.fluid.rows as i64;
        let row = rows / 2;
        let col = ((cols - msg.chars().count() as i64) as f64 / 2.0)
            .floor()
            .max(0.0) as i64;
        self.egg_overlays.clear();
        for (index, ch) in msg.chars().enumerate() {
            self.egg_overlays.push(Overlay {
                row,
                col: col + index as i64,
                ch,
                frames: 180,
            });
        }

        for j in (row - 4)..=(row + 4) {
            for i in (col - 2)..(col + msg.chars().count() as i64 + 2) {
                if i > 0 && i < cols - 1 && j > 0 && j < rows - 1 {
                    let p = self.fluid.at(i, j);
                    self.fluid.dens[p] = (0.3 + self.rng.next_f64() * 0.2) as f32;
                }
            }
        }
    }

    fn trigger_cowsay(&mut self) {
        if self.egg_mode != EggMode::None {
            return;
        }
        self.egg_mode = EggMode::Cowsay;
        self.overlays.clear();

        let quotes = [
            "moo? i'd fork that repo.",
            "undefined is not a function. again.",
            "have you tried turning it off and on?",
            "it's not a bug, it's a feature.",
            "sudo apt-get install clue",
            "there are 10 types of people...",
            "the rust borrow checker says no.",
            "404: sleep not found.",
        ];
        let index = (self.rng.next_f64() * quotes.len() as f64).floor() as usize;
        let quote = quotes[index.min(quotes.len() - 1)];

        let top = format!("  {}", "_".repeat(quote.len() + 2));
        let line = format!(" < {quote} >");
        let bottom = format!("  {}", "-".repeat(quote.len() + 2));
        let cow = [
            top.as_str(),
            line.as_str(),
            bottom.as_str(),
            "        \\   ^__^",
            "         \\  (oo)\\_______",
            "            (__)\\       )\\/\\",
            "                ||----w |",
            "                ||     ||",
        ];

        let cols = self.fluid.cols as i64;
        let rows = self.fluid.rows as i64;
        let start_row = (rows as f64 * 0.2).floor() as i64;
        let start_col = (cols as f64 * 0.25).floor() as i64;
        self.egg_overlays.clear();
        for (r, text) in cow.iter().enumerate() {
            for (c, ch) in text.chars().enumerate() {
                self.egg_overlays.push(Overlay {
                    row: start_row + r as i64,
                    col: start_col + c as i64,
                    ch,
                    frames: 240,
                });
            }
        }

        for j in (start_row - 1)..(start_row + cow.len() as i64 + 1) {
            for i in (start_col - 1)..(start_col + 50) {
                if i > 0 && i < cols - 1 && j > 0 && j < rows - 1 {
                    let p = self.fluid.at(i, j);
                    self.fluid.dens[p] = (0.15 + self.rng.next_f64() * 0.1) as f32;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.fluid.dens.fill(0.0);
        self.fluid.dens_prev.fill(0.0);
        self.fluid.u.fill(0.0);
        self.fluid.v.fill(0.0);
        self.fluid.u_prev.fill(0.0);
        self.fluid.v_prev.fill(0.0);
    }

    pub fn droplet(&mut self, ci: f64, cj: f64, amount: f64, radius: f64) {
        self.fluid.droplet(ci, cj, amount, radius);
    }

    pub fn vortex(&mut self, ci: f64, cj: f64, strength: f64) {
        self.fluid.vortex(ci, cj, strength);
    }

    pub fn stir(&mut self) {
        let cols = f64::from(self.fluid.cols);
        let rows = f64::from(self.fluid.rows);
        self.vortex(cols * 0.36, rows * 0.42, 0.5);
        self.vortex(cols * 0.63, rows * 0.55, -0.45);
        self.vortex(cols * 0.48, rows * 0.3, 0.32);
        self.droplet(cols * 0.38, rows * 0.45, 0.9, 4.0);
        self.droplet(cols * 0.61, rows * 0.5, 0.6, 4.0);
    }

    pub fn clear_masks(&mut self) {
        self.masks.clear();
    }

    pub fn push_mask(&mut self, row: f64, col: f64, rows: f64, cols: f64, fade: f64) {
        if row.is_finite() && col.is_finite() && rows.is_finite() && cols.is_finite() {
            self.masks.push(Mask {
                row,
                col,
                rows,
                cols,
                fade,
            });
        }
    }

    pub fn apply_masks(&mut self) {
        self.fluid.apply_masks(&self.masks);
    }

    pub fn layout_json(&self) -> String {
        let Some(layout) = &self.layout else {
            return String::new();
        };
        let buttons = layout
            .buttons
            .iter()
            .map(|button| format!("\"{}\"", json_escape(button)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"rows\":{},\"cols\":{},\"nameRow\":{},\"nameCol\":{},\"btnRow\":{},\"btnCol\":{},\"bioRow\":{},\"bioCol\":{},\"buttons\":[{}],\"btnStr\":\"{}\",\"bio\":\"{}\",\"greeting\":\"{}\"}}",
            layout.rows,
            layout.cols,
            layout.name_row,
            layout.name_col,
            layout.btn_row,
            layout.btn_col,
            layout.bio_row,
            layout.bio_col,
            buttons,
            json_escape(&layout.btn_str),
            json_escape(&layout.bio),
            json_escape(&layout.greeting)
        )
    }

    pub fn sample_json(&self, i: i64, j: i64) -> String {
        if i < 0 || j < 0 || i >= i64::from(self.fluid.cols) || j >= i64::from(self.fluid.rows) {
            return "null".to_string();
        }
        let p = self.fluid.at(i, j);
        let density = f64::from(self.fluid.dens[p]);
        let ramp: Vec<char> = self.ramp().chars().collect();
        let last = ramp.len() - 1;
        let ch = if density <= 0.0 {
            ' '
        } else {
            ramp[((density * 1.15).min(1.0) * last as f64).round() as usize]
        };
        let speed =
            (f64::from(self.fluid.u[p]).powi(2) + f64::from(self.fluid.v[p]).powi(2)).sqrt();
        format!(
            "{{\"i\":{},\"j\":{},\"dens\":{},\"char\":\"{}\",\"speed\":{}}}",
            i,
            j,
            density,
            json_escape(&ch.to_string()),
            speed
        )
    }

    pub fn current_segments(&self) -> Vec<f32> {
        let cols = self.fluid.cols as usize;
        let rows = self.fluid.rows as usize;
        if cols < 4 || rows < 4 {
            return Vec::new();
        }
        let mut peak = 0.0_f64;
        for p in 0..self.fluid.total {
            let speed = f64::from(self.fluid.u[p]).powi(2) + f64::from(self.fluid.v[p]).powi(2);
            if speed > peak {
                peak = speed;
            }
        }
        if peak <= 0.0 {
            return Vec::new();
        }
        let inv_peak = 1.0 / peak.sqrt();

        let sample = |field: &[f32], x: f64, y: f64| -> f64 {
            let i = clamp_i64(x.floor() as i64, 1, cols as i64 - 2) as usize;
            let j = clamp_i64(y.floor() as i64, 1, rows as i64 - 2) as usize;
            let fx = clamp_f64(x - i as f64, 0.0, 1.0);
            let fy = clamp_f64(y - j as f64, 0.0, 1.0);
            let p = i + j * cols;
            let top = f64::from(field[p]) + (f64::from(field[p + 1]) - f64::from(field[p])) * fx;
            let bot = f64::from(field[p + cols])
                + (f64::from(field[p + cols + 1]) - f64::from(field[p + cols])) * fx;
            top + (bot - top) * fy
        };

        let step = 0.8_f64;
        let max_steps = 30_i64;
        let stop = 0.06_f64;
        let seed_x = 5.0_f64;
        let seed_y = 4.0_f64;
        let mut segments = Vec::new();

        let mut sj = seed_y * 0.5;
        while sj < rows as f64 - 1.0 {
            let mut si = seed_x * 0.5;
            while si < cols as f64 - 1.0 {
                let hx = (si * 12.9898 + sj * 78.233).sin() * 43758.5453;
                let hy = (sj * 39.346 + si * 11.135).sin() * 24634.6345;
                let seed_x_pos = si + (hx - hx.floor() - 0.5) * seed_x * 0.8;
                let seed_y_pos = sj + (hy - hy.floor() - 0.5) * seed_y * 0.8;
                if seed_x_pos < 1.0
                    || seed_y_pos < 1.0
                    || seed_x_pos > cols as f64 - 2.0
                    || seed_y_pos > rows as f64 - 2.0
                {
                    si += seed_x;
                    continue;
                }

                let su0 = sample(&self.fluid.u, seed_x_pos, seed_y_pos) * inv_peak;
                let sv0 = sample(&self.fluid.v, seed_x_pos, seed_y_pos) * inv_peak;
                if (su0 * su0 + sv0 * sv0).sqrt() < stop * 2.0 {
                    si += seed_x;
                    continue;
                }

                for dir in [-1.0_f64, 1.0] {
                    let mut x = seed_x_pos;
                    let mut y = seed_y_pos;
                    for step_index in 0..max_steps {
                        let mut ux = sample(&self.fluid.u, x, y) * inv_peak;
                        let mut uy = sample(&self.fluid.v, x, y) * inv_peak;
                        let mut speed = (ux * ux + uy * uy).sqrt();
                        if speed < stop {
                            break;
                        }
                        ux /= speed;
                        uy /= speed;

                        let mid_x = x + ux * step * 0.5 * dir;
                        let mid_y = y + uy * step * 0.5 * dir;
                        let ux2 = sample(&self.fluid.u, mid_x, mid_y) * inv_peak;
                        let uy2 = sample(&self.fluid.v, mid_x, mid_y) * inv_peak;
                        let speed2 = (ux2 * ux2 + uy2 * uy2).sqrt();
                        if speed2 >= stop {
                            ux = ux2 / speed2;
                            uy = uy2 / speed2;
                            speed = speed2;
                        }

                        let nx = x + ux * step * dir;
                        let ny = y + uy * step * dir;
                        if !nx.is_finite() || !ny.is_finite() {
                            break;
                        }
                        if nx < 1.0 || nx > cols as f64 - 2.0 || ny < 1.0 || ny > rows as f64 - 2.0
                        {
                            break;
                        }
                        if !self.masks.is_empty() && mask_at_f64(&self.masks, nx, ny) {
                            break;
                        }

                        let taper = 1.0 - (step_index + 1) as f64 / max_steps as f64 * 0.75;
                        let band = ((speed * 5.0 * taper).floor() as i64).min(4).max(0);
                        segments.push(x as f32);
                        segments.push(y as f32);
                        segments.push(nx as f32);
                        segments.push(ny as f32);
                        segments.push(band as f32);

                        x = nx;
                        y = ny;
                    }
                }
                si += seed_x;
            }
            sj += seed_y;
        }
        segments
    }
}

fn push_centered(overlays: &mut Vec<Overlay>, row: i64, cols: i64, text: &str, frames: i32) {
    let len = text.chars().count() as i64;
    let col = ((cols - len) as f64 / 2.0).floor() as i64;
    for (index, ch) in text.chars().enumerate() {
        overlays.push(Overlay {
            row,
            col: col + index as i64,
            ch,
            frames,
        });
    }
}

fn mask_at_f64(masks: &[Mask], i: f64, j: f64) -> bool {
    masks.iter().any(|mask| {
        i >= mask.col && i < mask.col + mask.cols && j >= mask.row && j < mask.row + mask.rows
    })
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_has_every_row_and_column() {
        let mut sim = Sim::new(30, 20, 1);
        sim.configure("Hi", &[], "ascii", true, false, false, false, false);
        let frame = sim.render(false);
        let lines: Vec<&str> = frame.split('\n').collect();
        assert_eq!(lines.len(), 20);
        assert!(lines.iter().all(|line| line.chars().count() == 30));
    }

    #[test]
    fn easter_egg_buffer_triggers_once() {
        let mut sim = Sim::new(40, 20, 1);
        sim.configure("Hi", &[], "ascii", true, false, false, true, false);
        assert_eq!(sim.key("4"), "");
        assert_eq!(sim.key("2"), "42");
        assert_eq!(sim.key("c"), "");
        assert_eq!(sim.key("o"), "");
        assert_eq!(sim.key("w"), "");
        assert_eq!(sim.key("s"), "");
        assert_eq!(sim.key("a"), "");
        assert_eq!(sim.key("y"), "cowsay");
    }

    #[test]
    fn layout_is_centred() {
        let mut sim = Sim::new(80, 40, 1);
        sim.configure(
            "Hi",
            &["A".to_string()],
            "ascii",
            true,
            false,
            false,
            false,
            false,
        );
        sim.render(false);
        let layout = sim.layout_json();
        assert!(layout.contains("\"cols\":80"));
        assert!(layout.contains("\"buttons\":[\"A\"]"));
    }
}
