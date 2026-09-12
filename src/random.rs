use std::time::{SystemTime, UNIX_EPOCH};

pub trait RandomSource {
    fn next_u64(&mut self) -> u64;

    fn gen_range(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0, "random range must be non-empty");
        (self.next_u64() as usize) % upper_exclusive
    }
}

#[derive(Clone, Debug)]
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn seeded(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn from_system_time() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xA5A5_5A5A_1234_5678);
        Self::seeded(seed)
    }
}

impl RandomSource for XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_generator_is_reproducible() {
        let mut first = XorShift64::seeded(42);
        let mut second = XorShift64::seeded(42);
        for _ in 0..20 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
    }

    #[test]
    fn zero_seed_is_not_stuck() {
        let mut random = XorShift64::seeded(0);
        assert_ne!(random.next_u64(), 0);
    }
}
