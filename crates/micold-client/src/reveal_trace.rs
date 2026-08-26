//! A debug-only trace of feature 024's reveal, behind `MICOLD_REVEAL_TRACE`.
//!
//! BUG-002 was diagnosed, and then refuted, on the strength of two `eprintln!`s patched into the
//! client for a single run and deleted after it. The report asked for them back as something
//! permanent: "what did the drain compute, and what did the scrollable do with it?" is the question
//! every doubt about the reveal reduces to, and re-patching the client to ask it costs a build each
//! time — which is enough friction to make the cheap check the one nobody runs.
//!
//! Silent unless `MICOLD_REVEAL_TRACE` is set to something other than `0` or the empty string, so a
//! normal run pays one already-resolved `bool` read per message and prints nothing.

use std::sync::LazyLock;

static ENABLED: LazyLock<bool> = LazyLock::new(|| match std::env::var("MICOLD_REVEAL_TRACE") {
    Ok(value) => !value.is_empty() && value != "0",
    Err(_) => false,
});

/// Whether the reveal trace is on. Read once, from the environment the process started with.
pub fn enabled() -> bool {
    *ENABLED
}

/// Print one trace line to stderr, prefixed so it can be grepped out of a whole run's output.
///
/// Takes `Arguments` rather than being a macro so the formatting is not built at all when the
/// trace is off — the caller's `format_args!` borrows, it does not allocate.
pub fn line(args: std::fmt::Arguments<'_>) {
    if enabled() {
        eprintln!("reveal: {args}");
    }
}
