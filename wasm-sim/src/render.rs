use crate::fluid::{Fluid, Mask};

pub const RAMP: &str = " .-:~,^;*=|>+\\/?&$%#@";
pub const SHADE: [char; 5] = [' ', '░', '▒', '▓', '█'];
pub const STROKES: [char; 4] = ['-', '\\', '|', '/'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Ascii,
    Density,
    Velocity,
    Grid,
    Text,
}

impl RenderMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "density" => RenderMode::Density,
            "velocity" => RenderMode::Velocity,
            "grid" => RenderMode::Grid,
            "text" => RenderMode::Text,
            _ => RenderMode::Ascii,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RenderMode::Ascii => "ascii",
            RenderMode::Density => "density",
            RenderMode::Velocity => "velocity",
            RenderMode::Grid => "grid",
            RenderMode::Text => "text",
        }
    }
}

pub fn render_rows(fluid: &Fluid, mode: RenderMode, masks: &[Mask]) -> Vec<String> {
    match mode {
        RenderMode::Text => render_text(fluid),
        RenderMode::Grid => render_grid(fluid, masks),
        RenderMode::Velocity => render_current(fluid),
        RenderMode::Density => render_shade(fluid),
        RenderMode::Ascii => render_fluid(fluid),
    }
}

fn render_fluid(fluid: &Fluid) -> Vec<String> {
    let ramp: Vec<char> = RAMP.chars().collect();
    let last = ramp.len() - 1;
    let cols = fluid.cols as usize;
    let mut rows = Vec::with_capacity(fluid.rows as usize);
    for j in 0..fluid.rows as usize {
        let mut line = String::with_capacity(cols);
        for i in 0..cols {
            let d = (f64::from(fluid.dens[j * cols + i]) * 1.15).min(1.0);
            if d <= 0.0 {
                line.push(' ');
            } else {
                let index = ((d * last as f64).round() as usize).min(last);
                line.push(ramp[index]);
            }
        }
        rows.push(line);
    }
    rows
}

fn render_shade(fluid: &Fluid) -> Vec<String> {
    let cols = fluid.cols as usize;
    let mut rows = Vec::with_capacity(fluid.rows as usize);
    for j in 0..fluid.rows as usize {
        let mut line = String::with_capacity(cols);
        for i in 0..cols {
            let d = (f64::from(fluid.dens[j * cols + i]) * 1.15).min(1.0);
            let step = if d <= 0.0 {
                0
            } else {
                (1.0 + (d * 4.0).floor()).min(4.0) as usize
            };
            line.push(SHADE[step]);
        }
        rows.push(line);
    }
    rows
}

fn render_current(fluid: &Fluid) -> Vec<String> {
    let mut peak = 0.0_f64;
    for p in 0..fluid.total {
        let speed = f64::from(fluid.u[p]).powi(2) + f64::from(fluid.v[p]).powi(2);
        if speed > peak {
            peak = speed;
        }
    }
    let scale = if peak > 0.0 { 1.0 / peak.sqrt() } else { 0.0 };
    let cols = fluid.cols as usize;
    let mut rows = Vec::with_capacity(fluid.rows as usize);
    for j in 0..fluid.rows as usize {
        let mut line = String::with_capacity(cols);
        for i in 0..cols {
            let p = j * cols + i;
            let su = f64::from(fluid.u[p]) * scale;
            let sv = f64::from(fluid.v[p]) * scale;
            let speed = (su * su + sv * sv).sqrt();
            if speed < 0.08 {
                line.push(' ');
            } else if speed < 0.32 {
                line.push('.');
            } else {
                let mut angle = sv.atan2(su);
                if angle < 0.0 {
                    angle += std::f64::consts::PI;
                }
                let index = ((angle / (std::f64::consts::PI / 4.0)).round() as i64 % 4) as usize;
                line.push(STROKES[index]);
            }
        }
        rows.push(line);
    }
    rows
}

fn render_grid(fluid: &Fluid, masks: &[Mask]) -> Vec<String> {
    let cols = fluid.cols as usize;
    let mut rows = Vec::with_capacity(fluid.rows as usize);
    for j in 0..fluid.rows as usize {
        let mut line = String::with_capacity(cols);
        for i in 0..cols {
            if !masks.is_empty() && mask_at(masks, i, j) {
                line.push(if (i + j) % 4 == 0 { '.' } else { ' ' });
                continue;
            }
            line.push(if i % 5 == 0 && j % 5 == 0 { '+' } else { '.' });
        }
        rows.push(line);
    }
    rows
}

fn render_text(fluid: &Fluid) -> Vec<String> {
    vec![" ".repeat(fluid.cols as usize); fluid.rows as usize]
}

pub fn mask_at(masks: &[Mask], i: usize, j: usize) -> bool {
    let i = i as f64;
    let j = j as f64;
    masks.iter().any(|mask| {
        i >= mask.col && i < mask.col + mask.cols && j >= mask.row && j < mask.row + mask.rows
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_the_expected_shape() {
        let mut fluid = Fluid::new(12, 8);
        fluid.dens[20] = 0.5;
        for mode in [
            RenderMode::Ascii,
            RenderMode::Density,
            RenderMode::Velocity,
            RenderMode::Grid,
            RenderMode::Text,
        ] {
            let rows = render_rows(&fluid, mode, &[]);
            assert_eq!(rows.len(), 8);
            assert!(rows.iter().all(|row| row.chars().count() == 12));
        }
    }

    #[test]
    fn ascii_zero_density_is_spaces() {
        let fluid = Fluid::new(10, 6);
        let rows = render_rows(&fluid, RenderMode::Ascii, &[]);
        assert!(rows.iter().all(|row| row.trim().is_empty()));
    }
}
