//! Feature 551, US3 (T028, SC-005): the user guide's provider table has a row for every
//! `AiCli` variant, so a provider added later cannot ship undocumented.

use micold_core::session::AiCli;

const GUIDE: &str = include_str!("../../../docs/user-guide/worktrees-and-sessions.md");

/// The rows of the table under the "Which providers report which session started them" heading.
fn provider_rows() -> Vec<&'static str> {
    let heading = "### Which providers report which session started them";
    let start = GUIDE
        .find(heading)
        .expect("the user guide has the provider table section");
    GUIDE[start + heading.len()..]
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with('#'))
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .collect()
}

#[test]
fn the_provider_table_lists_every_provider() {
    let rows = provider_rows();
    for cli in AiCli::ALL {
        let name = cli.provider().display_name();
        assert!(
            rows.iter()
                .any(|row| row.trim_start_matches('|').trim_start().starts_with(name)),
            "no row for {name}"
        );
    }
    assert_eq!(rows.len(), AiCli::ALL.len(), "one row per provider");
}

#[test]
fn every_row_says_whether_it_reports_and_what_a_gap_does_instead() {
    for row in provider_rows() {
        let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
        assert_eq!(cells.len(), 4, "{row}");
        assert!(
            cells[1] == "Reports" || cells[1] == "No",
            "reports column of {row}"
        );
        assert!(!cells[2].is_empty() && !cells[3].is_empty(), "{row}");
    }
}
