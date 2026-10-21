#[derive(Clone, Copy)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng {
            state: seed.wrapping_mul(2654435761).wrapping_add(1),
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        f64::from(self.state) / 4294967296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..16 {
            assert_eq!(a.next_f64(), b.next_f64());
        }
    }

    #[test]
    fn values_stay_in_range() {
        let mut rng = Rng::new(42);
        for _ in 0..1000 {
            let value = rng.next_f64();
            assert!((0.0..1.0).contains(&value));
        }
    }
}
