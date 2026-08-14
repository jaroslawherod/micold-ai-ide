//! The one place this program asks the operating system anything (feature 021, T054 — FR-019a).
//!
//! The external system is the desktop environment's light/dark preference. It is the codebase's
//! only direct OS branch, which is why T047 put it behind [`OsThemeProbe`] and why Principle VI's
//! "no platform-specific reasoning outside a seam" comes down to this file.
//!
//! # Why the real probe is here and the trait is in the core
//!
//! `micold-core` deliberately has no dependency on `dark-light` — that is stated at the top of
//! `theme.rs` and is the entire reason [`SystemScheme`] mirrors `dark_light::Mode` instead of
//! re-exporting it. Wrapping the call *in* the core to "isolate the OS branch" would have put the
//! OS crate in the render-free half, which is the opposite of isolating it. So the trait and its
//! fake live beside the capability, and the concrete implementation lives in the shell (FR-017).
//!
//! # What is deliberately *not* here
//!
//! The clock that drives the probe is `shell/subscriptions.rs`'s (T053): the runtime schedules,
//! this module asks. And the fallback for a failed probe is neither — deciding what to do about
//! `Err` is not an I/O concern, it is `theme::observe_system_scheme`, pure and tested in the core
//! (FR-016). The split is three ways on purpose.

use micold_core::os_theme::OsThemeProbe;
use micold_core::theme::SystemScheme;

/// Translate the OS crate's own enum into the core's.
///
/// The core cannot do this itself — see the module doc — so the mapping is the seam, and it is
/// exhaustive by construction: a new `dark_light::Mode` variant fails to compile here rather than
/// silently reading as `Unspecified`.
pub(crate) fn map_system_scheme(mode: dark_light::Mode) -> SystemScheme {
    match mode {
        dark_light::Mode::Dark => SystemScheme::Dark,
        dark_light::Mode::Light => SystemScheme::Light,
        dark_light::Mode::Unspecified => SystemScheme::Unspecified,
    }
}

/// Query the OS for its current light/dark preference (FR-005). `dark_light::detect()`'s Linux
/// backend has a hardcoded 25 ms D-Bus timeout and returns `Err` under CPU contention with no
/// relation to the actual OS preference — the caller falls this back to the last-known scheme
/// via `theme::observe_system_scheme` rather than `SystemScheme::Unspecified` (FR-021; BUG-001).
/// Deliberately takes no arguments (bugfix, found by `run` sanity check, 2026-07-23): it used to
/// take `last_known: SystemScheme` and apply the fallback itself, but that meant
/// `os_theme_poll`'s `Subscription::map` closure had to *capture* `last_known` to call it — and
/// iced panics on boot if a subscription's mapping closure captures anything, since a capturing
/// closure can't have the stable identity iced needs to avoid restarting the underlying timer
/// every frame. The fallback now happens in the reducer (`Message::SystemThemeChanged`,
/// `src/app.rs`), which already has the previous scheme in `self.system_scheme`.
///
/// `pub(crate)`, not `pub`, and the difference is a lint rather than a design: in a binary the two
/// reach exactly as far, but clippy's `result_unit_err` fires on an exported `Result<_, ()>`.
/// The `()` is [`OsThemeProbe::detect`]'s own signature — "the OS declined to say", carrying
/// nothing because there is nothing to carry — so narrowing the visibility is the honest fix and
/// widening the error type to satisfy a lint would not be.
pub(crate) fn detect_system_scheme() -> Result<SystemScheme, ()> {
    SystemThemeProbe.detect()
}

/// The real [`OsThemeProbe`] (feature 021, T047): the codebase's only direct operating-system
/// branch, now behind the capability.
///
/// Here rather than in the core, where the trait and its fake live, because `dark-light` is a
/// client dependency and `micold-core` deliberately has none on it — that is why
/// [`SystemScheme`] mirrors `dark_light::Mode` instead of re-exporting it. Moving the call into
/// the core to "isolate the OS branch" would have put the OS crate in the render-free half, which
/// is the opposite of isolating it. The shell owns the concrete implementation; that is FR-017.
struct SystemThemeProbe;

impl OsThemeProbe for SystemThemeProbe {
    fn detect(&self) -> Result<SystemScheme, ()> {
        dark_light::detect().map(map_system_scheme).map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_dark_light_mode_onto_system_scheme() {
        assert_eq!(
            map_system_scheme(dark_light::Mode::Dark),
            SystemScheme::Dark
        );
        assert_eq!(
            map_system_scheme(dark_light::Mode::Light),
            SystemScheme::Light
        );
        assert_eq!(
            map_system_scheme(dark_light::Mode::Unspecified),
            SystemScheme::Unspecified
        );
    }

    /// The probe reaches the real desktop environment, so what it *answers* is not assertable in
    /// CI — a headless runner has no preference to report and a developer's machine has whichever
    /// one they set. What is assertable is that both answers are handled: `Ok` names a scheme the
    /// core understands, `Err` is the "declined to say" the reducer falls back from, and neither
    /// panics. Before T047 this call was inline in a subscription closure and had no test at all.
    #[test]
    fn the_probe_answers_without_panicking_whatever_the_desktop_says() {
        if let Ok(scheme) = SystemThemeProbe.detect() {
            assert!(matches!(
                scheme,
                SystemScheme::Dark | SystemScheme::Light | SystemScheme::Unspecified
            ));
        }
    }
}
