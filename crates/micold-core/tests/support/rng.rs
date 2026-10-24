//! A fixed-seed xorshift64 (no new dependency), shared by the property tests that need a
//! deterministic step source: `markdown.rs` re-exports it, and `src/review_tabs.rs`
//! `#[path]`-includes this file in its unit tests.

#![allow(dead_code)]

/// A fixed-seed xorshift64, so a failure names the same input every run.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}
