use std::ops::Range;

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(GAMMA),
        }
    }

    pub fn from_state(state: u64) -> Self {
        Self { state }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn gen_range(&mut self, range: Range<usize>) -> usize {
        debug_assert!(range.start < range.end);
        range.start + (self.next_u64() as usize % (range.end - range.start))
    }
}

#[cfg(test)]
#[path = "../tests/unit/rng.rs"]
mod tests;
