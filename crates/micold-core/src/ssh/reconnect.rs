//! The delay between automatic reconnect attempts of a remote daemon (feature 489, FR-007).
//!
//! Pure and deterministic: no clock and no randomness, so the 30 s recovery bound of SC-002 (at
//! most the cap, then one dial) is exact and a test can assert the whole sequence.

use std::time::Duration;

/// The first delay after a connection is lost.
pub const FIRST_DELAY: Duration = Duration::from_secs(1);
/// The longest delay between two attempts, however long the host stays away.
pub const MAX_DELAY: Duration = Duration::from_secs(15);

/// The growing delay of one remote daemon's reconnect attempts: 1, 2, 4, 8, 15, 15, ... seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    next: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self { next: FIRST_DELAY }
    }
}

impl Backoff {
    /// A backoff at its first delay.
    pub fn new() -> Self {
        Self::default()
    }

    /// The delay to wait before the next attempt; each call grows the following one up to the cap.
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.next;
        self.next = (self.next * 2).min(MAX_DELAY);
        delay
    }

    /// Back to the first delay: a connection succeeded, or the user asked to reconnect now.
    pub fn reset(&mut self) {
        self.next = FIRST_DELAY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(b: &mut Backoff, n: usize) -> Vec<u64> {
        (0..n).map(|_| b.next_delay().as_secs()).collect()
    }

    /// T039: 1, 2, 4, 8, then the cap, which never grows.
    #[test]
    fn delays_double_from_one_second_and_stop_at_fifteen() {
        assert_eq!(secs(&mut Backoff::new(), 8), [1, 2, 4, 8, 15, 15, 15, 15]);
    }

    /// T039: a success, or an explicit reconnect, starts the sequence again.
    #[test]
    fn reset_returns_to_one_second() {
        let mut b = Backoff::new();
        secs(&mut b, 6);
        b.reset();
        assert_eq!(secs(&mut b, 3), [1, 2, 4]);
    }
}
