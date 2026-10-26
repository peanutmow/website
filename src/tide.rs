pub const COLS: u32 = 88;
pub const ROWS: u32 = 28;

const SWELL: usize = 6;
const RAMP: &[char] = &[' ', '.', ',', '-', '~', ':', '=', '+', '*', '#', '%', '@'];

pub fn today() -> (String, u32) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64;
    let date = chrono::DateTime::from_timestamp(days * 86_400, 0)
        .map(|stamp| stamp.date_naive().to_string())
        .unwrap_or_else(|| "1970-01-01".to_string());
    (date, days as u32)
}

pub fn sea(seed: u32) -> String {
    simulate(seed, COLS, ROWS)
}

fn simulate(seed: u32, cols: u32, rows: u32) -> String {
    let cols = cols.max(3);
    let rows = rows.max(3);

    let mut rng = TideRng::new(seed);
    let mut swell = [(0.0f64, 0.0f64, 0.0f64, 0.0f64); SWELL];
    for (index, wave) in swell.iter_mut().enumerate() {
        let base = rng.unit() * std::f64::consts::TAU;
        let crossing = if index % 3 == 0 {
            base + std::f64::consts::FRAC_PI_2
        } else {
            base
        };
        let angle = crossing + (rng.unit() - 0.5) * 0.7;
        let length = 9.0 + rng.unit() * 12.0;
        let k = std::f64::consts::TAU / length;
        wave.0 = k * angle.cos();
        wave.1 = k * angle.sin();
        wave.2 = rng.unit() * std::f64::consts::TAU;
        wave.3 = 0.6 + rng.unit() * 0.8;
    }
    let amplitude: f64 = swell.iter().map(|wave| wave.3).sum();
    let last = (RAMP.len() - 1) as f64;

    let mut lines = Vec::with_capacity(rows as usize);
    for j in 0..rows {
        let mut line = String::with_capacity(cols as usize);
        for i in 0..cols {
            let x = f64::from(i);
            let y = f64::from(j);
            let mut height = 0.0;
            for &(kx, ky, phase, weight) in &swell {
                let warped_x = x + 0.9 * (y * 0.07 + phase).sin();
                let warped_y = y + 0.6 * (x * 0.05 + phase).sin();
                height += weight * (kx * warped_x + ky * warped_y + phase).sin();
            }
            let level = (height / amplitude * 0.5 + 0.5).clamp(0.0, 1.0);
            let ink = ((level - 0.47) / 0.33).clamp(0.0, 1.0);
            let index = (ink.powf(0.9) * last).round() as usize;
            line.push(RAMP[index.min(RAMP.len() - 1)]);
        }
        lines.push(line);
    }
    lines.join("\n")
}

struct TideRng {
    state: u64,
}

impl TideRng {
    fn new(seed: u32) -> Self {
        Self {
            state: (u64::from(seed) << 32) | 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
}

#[cfg(test)]
mod tests {
    use super::{simulate, today};

    fn sea(seed: u32) -> String {
        simulate(seed, 24, 10)
    }

    #[test]
    fn the_same_seed_renders_the_same_sea() {
        assert_eq!(sea(7), sea(7));
    }

    #[test]
    fn different_seeds_render_different_seas() {
        assert_ne!(sea(7), sea(8));
    }

    #[test]
    fn the_sea_keeps_the_shape_of_the_grid() {
        let text = sea(20_700);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 10);
        for line in &lines {
            assert_eq!(line.chars().count(), 24);
        }
        assert!(text.chars().any(|ch| ch != ' '), "the tide came out blank");
    }

    #[test]
    fn today_is_a_date_and_a_seed() {
        let (date, seed) = today();
        assert_eq!(date.len(), 10);
        assert_eq!(date.as_bytes()[4], b'-');
        assert!(seed > 20_000);
    }
}
