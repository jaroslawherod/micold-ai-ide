//! The lines printed into a terminal around restored history (data-model §7). They take the
//! already formatted parts, so this crate needs no time crate.

/// The one-row line shown where a restarted session's output begins, for a terminal `columns`
/// wide. `date_time_offset` is the start's local time with its UTC offset, e.g.
/// `2026-10-02 14:31 +02:00`.
pub fn separator_line(date_time_offset: &str, columns: usize) -> String {
    fit(&format!("session restarted at {date_time_offset}"), columns)
}

/// `text` between rules if that fits in `columns`, else `text` alone, cut to `columns` if need be.
/// Widths are counted in characters: the text is the daemon's own, one column per character.
fn fit(text: &str, columns: usize) -> String {
    let ruled = format!("── {text} ──");
    if ruled.chars().count() <= columns {
        ruled
    } else {
        text.chars().take(columns).collect()
    }
}
