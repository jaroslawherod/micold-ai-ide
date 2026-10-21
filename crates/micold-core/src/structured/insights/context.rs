//! The context reading (data-model.md *ContextReading*, research R2, FR-001–FR-005).

use std::collections::BTreeMap;

/// The context as of the latest main-scope model response held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextReading {
    /// Input, cache and output tokens of the latest main response; `None` before one, or when
    /// its usage was unreadable.
    pub used: Option<u64>,
    /// The model of that response.
    pub model: Option<String>,
    /// That model's context window, from the latest held result listing it.
    pub window: Option<u64>,
    /// The compaction threshold of the latest `autocompact_state`, when compaction is enabled.
    pub threshold: Option<u64>,
}

/// What the meter shows (FR-001–FR-004). A `Measured` reading always has its window
/// (Constitution V): without one there is no share to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    /// No main response held yet.
    NoData,
    /// The window of the response's model is unknown: tokens only.
    TokensOnly { used: u64 },
    Measured {
        used: u64,
        window: u64,
        /// `used × 100 / window`, whole, rounded half up.
        percent: u64,
        threshold: Option<u64>,
        /// At or above the threshold; at or above 90% of the window when there is none.
        warning: bool,
    },
}

impl ContextReading {
    pub fn state(&self) -> ContextState {
        let Some(used) = self.used else {
            return ContextState::NoData;
        };
        let Some(window) = self.window.filter(|w| *w > 0) else {
            return ContextState::TokensOnly { used };
        };
        let (used_wide, window_wide) = (u128::from(used), u128::from(window));
        let percent = (used_wide * 200 + window_wide) / (window_wide * 2);
        let warning = match self.threshold {
            Some(threshold) => used >= threshold,
            None => used_wide * 10 >= window_wide * 9,
        };
        ContextState::Measured {
            used,
            window,
            percent: u64::try_from(percent).unwrap_or(u64::MAX),
            threshold: self.threshold,
            warning,
        }
    }
}

/// The reading and the windows the held results reported, folded digest by digest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ContextFold {
    pub(super) reading: ContextReading,
    /// Each model's window, from the latest held result listing it.
    windows: BTreeMap<String, u64>,
}

impl ContextFold {
    pub(super) fn response(&mut self, model: Option<&str>, used: Option<u64>) {
        self.reading.used = used;
        self.reading.model = model.map(str::to_string);
        self.refresh_window();
    }

    pub(super) fn windows(&mut self, windows: &[(String, u64)]) {
        for (model, window) in windows {
            self.windows.insert(model.clone(), *window);
        }
        self.refresh_window();
    }

    pub(super) fn threshold(&mut self, threshold: Option<u64>) {
        self.reading.threshold = threshold;
    }

    fn refresh_window(&mut self) {
        self.reading.window = self
            .reading
            .model
            .as_ref()
            .and_then(|model| self.windows.get(model).copied());
    }
}
