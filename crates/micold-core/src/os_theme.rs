//! Asking the operating system whether it prefers light or dark (feature 021, T047 — FR-015,
//! FR-019; Principle VI).
//!
//! # Why the real implementation is not here
//!
//! T047 says to declare this capability "wrapping the `dark_light` call". The trait is here; the
//! wrapping is not, and the reason is a property this crate already holds deliberately.
//! `micold-core` has no dependency on the OS-detection crate — that is stated at the top of
//! [`crate::theme`] and is the whole reason [`SystemScheme`] exists as a mirror of
//! `dark_light::Mode` rather than a re-export of it. Moving the call in would add `dark-light` to
//! the render-free core to satisfy a task whose purpose is to *isolate* the OS branch, which is
//! the opposite of what it asks for.
//!
//! So the split follows the port pattern exactly: the **trait and the fake live here**, needing
//! nothing but [`SystemScheme`]; the **real implementation stays in the client**, where
//! `dark-light` is already a dependency and where FR-017 says a concrete implementation belongs.
//! FR-019's "fakes live in the core beside the capability" is satisfied — the capability *is* here.
//!
//! Principle VI is better served this way, not worse: `dark_light::detect()` is the codebase's only
//! direct operating-system branch, and it now sits behind an abstraction that every consumer can be
//! tested against without an OS to ask.
//!
//! # What the fallback is, and where it lives
//!
//! Not here. A failed probe must not overwrite the last-known scheme — `dark_light`'s Linux backend
//! has a hardcoded 25 ms D-Bus timeout and returns `Err` under CPU load with no relation to the
//! user's actual preference (FR-021, BUG-001). That decision is
//! [`crate::theme::observe_system_scheme`], pure and already tested. This capability reports what
//! the OS said, including that it could not be asked; deciding what to do about it is not an I/O
//! concern, and folding it in here would make the port wider than its consumers need (FR-016).

use crate::theme::SystemScheme;
use std::cell::RefCell;

/// Reading the operating system's light/dark preference.
///
/// `Err(())` means the query itself failed — the OS was not reachable or timed out — which is
/// distinct from [`SystemScheme::Unspecified`], where the OS answered and expressed no preference.
/// Collapsing the two is exactly the bug BUG-001 fixed: a timeout was being read as "no
/// preference" and flashing a dark desktop to light.
pub trait OsThemeProbe {
    /// Ask the OS for its current preference.
    ///
    /// `clippy::result_unit_err` would prefer a named error type here, and its reasoning — that
    /// `Result<_, ()>` hands the caller nothing to act on — is usually right. It is not right here,
    /// and the exception is argued rather than silenced. There is deliberately nothing to act on:
    /// BUG-001's lesson is that *why* the query failed is irrelevant and dangerous to interpret,
    /// since `dark_light`'s Linux backend reports a 25 ms D-Bus timeout the same way it reports a
    /// genuine absence. The only consumer is [`crate::theme::observe_system_scheme`], whose
    /// signature takes exactly `Result<SystemScheme, ()>` and is frozen for this feature (FR-027).
    /// A distinct error type would buy a name and cost a `map_err` at every call site whose sole
    /// purpose is to discard it again.
    #[allow(clippy::result_unit_err)]
    fn detect(&self) -> Result<SystemScheme, ()>;
}

/// A probe that answers from a script instead of the operating system (FR-019).
///
/// Answers are consumed in order and the last one repeats, so a test can write a timeline —
/// `Dark`, then a failure, then `Light` — and let a polling consumer walk it. Repeating the last
/// answer rather than running out means a test says how many *distinct* answers it cares about,
/// not how many times something happens to poll.
#[derive(Debug, Default)]
pub struct FakeOsThemeProbe {
    inner: RefCell<FakeProbeState>,
}

#[derive(Debug, Default)]
struct FakeProbeState {
    answers: Vec<Result<SystemScheme, ()>>,
    asked: usize,
}

impl FakeOsThemeProbe {
    /// A probe that always answers `scheme`.
    pub fn always(scheme: SystemScheme) -> Self {
        Self::scripted(vec![Ok(scheme)])
    }

    /// A probe that cannot reach the OS at all.
    pub fn failing() -> Self {
        Self::scripted(vec![Err(())])
    }

    /// A probe answering each of `answers` in turn, then repeating the last.
    pub fn scripted(answers: Vec<Result<SystemScheme, ()>>) -> Self {
        Self {
            inner: RefCell::new(FakeProbeState { answers, asked: 0 }),
        }
    }

    /// How many times the OS has been asked (test assertions).
    ///
    /// The count matters on its own: a consumer that caches when it should poll, or polls when it
    /// should not, produces the right scheme for the wrong reason.
    pub fn times_asked(&self) -> usize {
        self.inner.borrow().asked
    }
}

impl OsThemeProbe for FakeOsThemeProbe {
    fn detect(&self) -> Result<SystemScheme, ()> {
        let mut state = self.inner.borrow_mut();
        let at = state.asked.min(state.answers.len().saturating_sub(1));
        state.asked += 1;
        state.answers.get(at).copied().unwrap_or(Err(()))
    }
}
