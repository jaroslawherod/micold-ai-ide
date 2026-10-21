//! A turn footer's figures as text (research R14, FR-005).

use super::entry::TurnFooter;

/// `"6.7 s"` below a minute, `"1 min 12 s"` from a minute on.
pub fn format_duration(ms: u64) -> String {
    let tenths = (ms + 50) / 100;
    if tenths < 600 {
        format!("{}.{} s", tenths / 10, tenths % 10)
    } else {
        let seconds = (ms + 500) / 1000;
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

/// Four decimals below $1 (`"$0.0372"`), two from $1 on (`"$1.24"`).
pub fn format_cost(usd: f64) -> String {
    if usd < 1.0 {
        format!("${usd:.4}")
    } else {
        format!("${usd:.2}")
    }
}

/// `"1 turn"`, `"2 turns"`.
pub fn format_turns(n: u64) -> String {
    if n == 1 {
        "1 turn".to_string()
    } else {
        format!("{n} turns")
    }
}

/// The footer's duration, cost and turns, in that order, each only when the result reports it.
///
/// A result's `total_cost_usd` is the CLI session's running total, not the turn's, so the cost
/// reads `"session total $0.0372"`; the per-turn delta is #944's.
pub fn footer_figures(footer: &TurnFooter) -> Vec<String> {
    [
        footer.duration_ms.map(format_duration),
        footer
            .cost_usd
            .map(|usd| format!("session total {}", format_cost(usd))),
        footer.num_turns.map(format_turns),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::transcript::Outcome;

    #[test]
    fn a_duration_below_a_minute_is_seconds_with_one_decimal() {
        assert_eq!(format_duration(6_674), "6.7 s");
        assert_eq!(format_duration(12), "0.0 s");
        assert_eq!(format_duration(59_900), "59.9 s");
    }

    #[test]
    fn a_duration_from_a_minute_on_is_minutes_and_seconds() {
        assert_eq!(format_duration(72_400), "1 min 12 s");
        assert_eq!(format_duration(60_000), "1 min 0 s");
        assert_eq!(format_duration(59_990), "1 min 0 s", "no \"60.0 s\"");
        assert_eq!(format_duration(3_725_000), "62 min 5 s");
    }

    #[test]
    fn a_cost_below_a_dollar_has_four_decimals_and_two_above() {
        assert_eq!(format_cost(0.0371549), "$0.0372");
        assert_eq!(format_cost(0.0), "$0.0000");
        assert_eq!(format_cost(1.2449), "$1.24");
        assert_eq!(format_cost(12.0), "$12.00");
    }

    #[test]
    fn turns_are_counted_in_words() {
        assert_eq!(format_turns(1), "1 turn");
        assert_eq!(format_turns(2), "2 turns");
        assert_eq!(format_turns(0), "0 turns");
    }

    fn footer(
        duration_ms: Option<u64>,
        cost_usd: Option<f64>,
        num_turns: Option<u64>,
    ) -> TurnFooter {
        TurnFooter {
            outcome: Outcome::Success,
            duration_ms,
            cost_usd,
            num_turns,
            error_text: None,
        }
    }

    #[test]
    fn the_footer_shows_every_figure_the_result_reports() {
        assert_eq!(
            footer_figures(&footer(Some(6_674), Some(0.0371549), Some(1))),
            vec!["6.7 s", "session total $0.0372", "1 turn"]
        );
    }

    #[test]
    fn a_figure_the_result_omits_is_left_out_not_zero() {
        assert_eq!(
            footer_figures(&footer(None, None, None)),
            Vec::<String>::new()
        );
        assert_eq!(
            footer_figures(&footer(None, Some(1.5), None)),
            vec!["session total $1.50"]
        );
        assert_eq!(
            footer_figures(&footer(Some(1_000), None, Some(3))),
            vec!["1.0 s", "3 turns"]
        );
    }
}
