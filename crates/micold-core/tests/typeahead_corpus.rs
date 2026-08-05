//! Does the ranking actually surface the branch the developer meant? (feature 021, contract
//! `match-ranking.md` §4a; SC-003, SC-001.)
//!
//! Every other matching test pins a *rule*: this tier before that one, this offset, this span. None
//! of them can tell you whether the rules add up to a useful list — a set of rules can be
//! individually correct and collectively useless. So this file measures the outcome instead: over a
//! fixed corpus of realistic branch names and a fixed set of searches, how often does the branch the
//! developer had in mind come back in the top five?
//!
//! The corpus and the pairs are **committed data, not generated**. A figure computed from random
//! input would move on its own and mean nothing across changes; this one moves only when the
//! matching does. The assertion is on the *rate*, so a single pair may legitimately fail — a search
//! that is genuinely ambiguous should not be able to hold the build hostage — but a change that
//! drops the rate names every pair that regressed, so the cost is visible rather than averaged away.
//!
//! When one of these fails, the fix is the implementation. Editing the corpus to suit the answer
//! turns a measurement into a mirror (contract §4a.3).

use micold_core::typeahead::{rank, Query};

/// How far down the list still counts as "found it".
const TOP_N: usize = 5;

/// The share of searches that must land inside [`TOP_N`] (SC-003).
const REQUIRED_RATE: f64 = 0.95;

/// Realistic branch names: Conventional-Commits types, some with ticket keys, several slugs
/// repeated under different types the way a real repository accumulates them.
const CORPUS: &[&str] = &[
    "feat/login-page",
    "fix/abc-101-password-reset",
    "chore/dev-312-session-timeout",
    "refactor/oauth-callback",
    "docs/abc-204-token-refresh",
    "test/ops-9-reporting-dashboard",
    "perf/export-to-csv",
    "build/dev-77-scheduled-reports",
    "ci/report-filters",
    "style/abc-101-chart-legend",
    "feat/dev-312-worktree-picker",
    "fix/branch-typeahead",
    "chore/abc-204-sidebar-filters",
    "refactor/ops-9-context-menu",
    "docs/keyboard-shortcuts",
    "test/dev-77-terminal-scrollback",
    "perf/terminal-resize",
    "build/abc-101-cursor-blink",
    "ci/dev-312-paste-bracketed",
    "style/ansi-colors",
    "feat/abc-204-daemon-reconnect",
    "fix/ops-9-daemon-handshake",
    "chore/socket-permissions",
    "refactor/dev-77-ipc-framing",
    "docs/heartbeat-interval",
    "test/abc-101-grid-diffing",
    "perf/dev-312-frame-budget",
    "build/paragraph-cache",
    "ci/abc-204-layout-thrash",
    "style/ops-9-idle-redraws",
    "feat/dark-theme",
    "fix/dev-77-light-theme",
    "chore/token-palette",
    "refactor/abc-101-contrast-ratios",
    "docs/dev-312-elevation-levels",
    "test/settings-dialog",
    "perf/abc-204-about-dialog",
    "build/ops-9-notification-banner",
    "ci/progress-stages",
    "style/dev-77-error-copy",
    "feat/project-switcher",
    "fix/abc-101-recent-projects",
    "chore/dev-312-forget-project",
    "refactor/open-project",
    "docs/abc-204-project-root",
    "test/ops-9-packaging-deb",
    "perf/release-notes",
    "build/dev-77-version-bump",
    "ci/changelog-viewer",
    "style/abc-101-signing-key",
    "feat/dev-312-flaky-tests",
    "fix/snapshot-tests",
    "chore/abc-204-golden-files",
    "refactor/ops-9-test-fixtures",
    "docs/coverage-report",
    "test/dev-77-readme-rewrite",
    "perf/architecture-notes",
    "build/abc-101-user-guide",
    "ci/dev-312-contributing",
    "style/api-reference",
    "feat/abc-204-clippy-warnings",
    "fix/ops-9-rustfmt-config",
    "chore/msrv-bump",
    "refactor/dev-77-dependency-audit",
    "docs/cargo-deny",
    "test/abc-101-git-fetch-cache",
    "perf/dev-312-submodule-init",
    "build/worktree-prune",
    "ci/abc-204-branch-cleanup",
    "style/ops-9-stale-refs",
    "build/dev-77-login-page",
    "ci/password-reset",
    "style/abc-101-session-timeout",
    "feat/dev-312-oauth-callback",
    "fix/token-refresh",
    "chore/abc-204-reporting-dashboard",
    "refactor/ops-9-export-to-csv",
    "docs/scheduled-reports",
    "test/dev-77-report-filters",
    "perf/chart-legend",
    "build/abc-101-worktree-picker",
    "ci/dev-312-branch-typeahead",
    "style/sidebar-filters",
    "feat/abc-204-context-menu",
    "fix/ops-9-keyboard-shortcuts",
    "chore/terminal-scrollback",
    "refactor/dev-77-terminal-resize",
    "docs/cursor-blink",
    "test/abc-101-paste-bracketed",
    "perf/dev-312-ansi-colors",
    "build/daemon-reconnect",
    "ci/abc-204-daemon-handshake",
    "style/ops-9-socket-permissions",
    "feat/ipc-framing",
    "fix/dev-77-heartbeat-interval",
    "chore/grid-diffing",
    "refactor/abc-101-frame-budget",
    "docs/dev-312-paragraph-cache",
    "test/layout-thrash",
    "perf/abc-204-idle-redraws",
    "build/ops-9-dark-theme",
    "ci/light-theme",
    "style/dev-77-token-palette",
    "feat/contrast-ratios",
    "fix/abc-101-elevation-levels",
    "chore/dev-312-settings-dialog",
    "refactor/about-dialog",
    "docs/abc-204-notification-banner",
    "test/ops-9-progress-stages",
    "perf/error-copy",
    "build/dev-77-project-switcher",
    "ci/recent-projects",
    "style/abc-101-forget-project",
    "feat/dev-312-open-project",
    "fix/project-root",
    "chore/abc-204-packaging-deb",
    "refactor/ops-9-release-notes",
    "docs/version-bump",
    "test/dev-77-changelog-viewer",
    "perf/signing-key",
    "build/abc-101-flaky-tests",
    "ci/dev-312-snapshot-tests",
    "style/golden-files",
    "feat/abc-204-test-fixtures",
    "fix/ops-9-coverage-report",
    "chore/readme-rewrite",
    "refactor/dev-77-architecture-notes",
    "docs/user-guide",
    "test/abc-101-contributing",
    "perf/dev-312-api-reference",
    "build/clippy-warnings",
    "ci/abc-204-rustfmt-config",
    "style/ops-9-msrv-bump",
    "feat/dependency-audit",
    "fix/dev-77-cargo-deny",
    "chore/git-fetch-cache",
    "refactor/abc-101-submodule-init",
    "docs/dev-312-worktree-prune",
    "test/branch-cleanup",
    "perf/abc-204-stale-refs",
    "docs/login-page",
    "test/dev-77-password-reset",
    "perf/session-timeout",
    "build/abc-101-oauth-callback",
    "ci/dev-312-token-refresh",
    "style/reporting-dashboard",
    "feat/abc-204-export-to-csv",
    "fix/ops-9-scheduled-reports",
    "chore/report-filters",
    "refactor/dev-77-chart-legend",
    "docs/worktree-picker",
    "test/abc-101-branch-typeahead",
    "perf/dev-312-sidebar-filters",
    "build/context-menu",
    "ci/abc-204-keyboard-shortcuts",
    "style/ops-9-terminal-scrollback",
    "feat/terminal-resize",
    "fix/dev-77-cursor-blink",
    "chore/paste-bracketed",
    "refactor/abc-101-ansi-colors",
    "docs/dev-312-daemon-reconnect",
    "test/daemon-handshake",
    "perf/abc-204-socket-permissions",
    "build/ops-9-ipc-framing",
    "ci/heartbeat-interval",
    "style/dev-77-grid-diffing",
    "feat/frame-budget",
    "fix/abc-101-paragraph-cache",
    "chore/dev-312-layout-thrash",
    "refactor/idle-redraws",
    "docs/abc-204-dark-theme",
    "test/ops-9-light-theme",
    "perf/token-palette",
    "build/dev-77-contrast-ratios",
    "ci/elevation-levels",
    "style/abc-101-settings-dialog",
    "feat/dev-312-about-dialog",
    "fix/notification-banner",
    "chore/abc-204-progress-stages",
    "refactor/ops-9-error-copy",
    "docs/project-switcher",
    "test/dev-77-recent-projects",
    "perf/forget-project",
    "build/abc-101-open-project",
    "ci/dev-312-project-root",
    "style/packaging-deb",
    "feat/abc-204-release-notes",
    "fix/ops-9-version-bump",
    "chore/changelog-viewer",
    "refactor/dev-77-signing-key",
    "docs/flaky-tests",
    "test/abc-101-snapshot-tests",
    "perf/dev-312-golden-files",
    "build/test-fixtures",
    "ci/abc-204-coverage-report",
    "style/ops-9-readme-rewrite",
    "feat/architecture-notes",
    "fix/dev-77-user-guide",
    "chore/contributing",
    "refactor/abc-101-api-reference",
    "docs/dev-312-clippy-warnings",
    "test/rustfmt-config",
    "perf/abc-204-msrv-bump",
    "build/ops-9-dependency-audit",
    "ci/cargo-deny",
    "style/dev-77-git-fetch-cache",
    "feat/submodule-init",
    "fix/abc-101-worktree-prune",
    "chore/dev-312-branch-cleanup",
    "refactor/stale-refs",
    "main",
    "develop",
    "release/2.4",
    "release/2.5",
    "hotfix/2.4.1",
];

/// `(what was typed, the branch it was meant to find, which tier is expected to carry it)`.
///
/// The third element is documentation rather than an assertion: which tier a pair travels through
/// is pinned by `typeahead_match.rs`, and pinning it again here would make this file fail for a
/// reason that has nothing to do with ranking quality. It is recorded so a drop in the rate can be
/// read by tier — "the abbreviations regressed" is a different diagnosis from "the typos did".
const PAIRS: &[(&str, &str, &str)] = &[
    ("login-page", "feat/login-page", "literal"),
    ("password", "fix/abc-101-password-reset", "literal"),
    ("oauth", "refactor/oauth-callback", "literal"),
    ("scrollback", "test/dev-77-terminal-scrollback", "literal"),
    ("bracketed", "ci/dev-312-paste-bracketed", "literal"),
    ("handshake", "fix/ops-9-daemon-handshake", "literal"),
    ("paragraph", "build/paragraph-cache", "literal"),
    ("contrast", "refactor/abc-101-contrast-ratios", "literal"),
    ("banner", "build/ops-9-notification-banner", "literal"),
    ("switcher", "feat/project-switcher", "literal"),
    ("changelog", "ci/changelog-viewer", "literal"),
    ("golden", "chore/abc-204-golden-files", "literal"),
    ("msrv", "chore/msrv-bump", "literal"),
    ("submodule", "perf/dev-312-submodule-init", "literal"),
    ("stale-refs", "style/ops-9-stale-refs", "literal"),
    ("typeahead", "fix/branch-typeahead", "literal"),
    ("heartbeat", "docs/heartbeat-interval", "literal"),
    ("lgnpg", "feat/login-page", "subsequence"),
    ("pwdrst", "fix/abc-101-password-reset", "subsequence"),
    ("sesstmt", "chore/dev-312-session-timeout", "subsequence"),
    ("trmres", "perf/terminal-resize", "subsequence"),
    ("dmnrcn", "feat/abc-204-daemon-reconnect", "subsequence"),
    ("frmbdgt", "perf/dev-312-frame-budget", "subsequence"),
    ("drkthm", "feat/dark-theme", "subsequence"),
    ("stngsdlg", "test/settings-dialog", "subsequence"),
    ("rcntprj", "fix/abc-101-recent-projects", "subsequence"),
    ("vrsnbmp", "build/dev-77-version-bump", "subsequence"),
    ("flkytsts", "feat/dev-312-flaky-tests", "subsequence"),
    ("clppywrn", "feat/abc-204-clippy-warnings", "subsequence"),
    ("brnchclnp", "ci/abc-204-branch-cleanup", "subsequence"),
    ("passwrd-reset", "fix/abc-101-password-reset", "single-edit"),
    ("oauht-callback", "refactor/oauth-callback", "single-edit"),
    (
        "reportng-dashboard",
        "test/ops-9-reporting-dashboard",
        "single-edit",
    ),
    (
        "keyboad-shortcuts",
        "docs/keyboard-shortcuts",
        "single-edit",
    ),
    (
        "scrollbak",
        "test/dev-77-terminal-scrollback",
        "single-edit",
    ),
    (
        "hearbeat-interval",
        "docs/heartbeat-interval",
        "single-edit",
    ),
    ("paragrph-cache", "build/paragraph-cache", "single-edit"),
    (
        "elevatoin-levels",
        "docs/dev-312-elevation-levels",
        "single-edit",
    ),
    (
        "notifcation-banner",
        "build/ops-9-notification-banner",
        "single-edit",
    ),
    ("projct-switcher", "feat/project-switcher", "single-edit"),
    ("changlog-viewer", "ci/changelog-viewer", "single-edit"),
    ("snapsht-tests", "fix/snapshot-tests", "single-edit"),
    (
        "dependecy-audit",
        "refactor/dev-77-dependency-audit",
        "single-edit",
    ),
    ("worktre-prune", "build/worktree-prune", "single-edit"),
];

/// Where `intended` came back in the ranking for `query`, or `None` if it did not come back at all.
fn position(query: &str, intended: &str) -> Option<usize> {
    rank(CORPUS, |n| *n, &Query::new(query))
        .iter()
        .position(|(index, _)| CORPUS[*index] == intended)
}

/// SC-003 — at least 95% of the searches put the branch the developer meant in the top five.
#[test]
fn the_intended_branch_reaches_the_top_five_for_at_least_the_required_share_of_searches() {
    let mut missed = Vec::new();
    for (query, intended, tier) in PAIRS {
        match position(query, intended) {
            Some(at) if at < TOP_N => {}
            Some(at) => missed.push(format!(
                "{query:?} → {intended:?} ({tier}) ranked {}",
                at + 1
            )),
            None => missed.push(format!(
                "{query:?} → {intended:?} ({tier}) did not match at all"
            )),
        }
    }

    let rate = (PAIRS.len() - missed.len()) as f64 / PAIRS.len() as f64;
    assert!(
        rate >= REQUIRED_RATE,
        "{:.1}% of searches found their branch in the top {TOP_N}; {:.0}% is required.\n\
         {} of {} regressed:\n  {}",
        rate * 100.0,
        REQUIRED_RATE * 100.0,
        missed.len(),
        PAIRS.len(),
        missed.join("\n  ")
    );
}

/// SC-001 — "eight characters and it is on screen". The headline claim, measured rather than
/// asserted: a developer who knows the branch types the first eight characters of the part that
/// *names the work* — not the `feat/` that a third of the repository shares, and not the ticket key
/// they would have to look up — and the branch is in the list without scrolling.
#[test]
fn eight_characters_of_a_branch_puts_that_branch_on_screen() {
    let mut missed = Vec::new();
    for (_, intended, _) in PAIRS {
        let typed: String = distinctive_part(intended).chars().take(8).collect();
        match position(&typed, intended) {
            Some(at) if at < TOP_N => {}
            Some(at) => missed.push(format!("{typed:?} → {intended:?} ranked {}", at + 1)),
            None => missed.push(format!("{typed:?} → {intended:?} did not match at all")),
        }
    }

    let rate = (PAIRS.len() - missed.len()) as f64 / PAIRS.len() as f64;
    assert!(
        rate >= REQUIRED_RATE,
        "{:.1}% of eight-character searches put their branch in the top {TOP_N}; \
         {:.0}% is required.\n{} of {} regressed:\n  {}",
        rate * 100.0,
        REQUIRED_RATE * 100.0,
        missed.len(),
        PAIRS.len(),
        missed.join("\n  ")
    );
}

/// The part of a branch name a developer would actually type to find it: what follows the
/// Conventional-Commits type, minus a leading ticket key. `feat/abc-101-login-page` → `login-page`.
///
/// Not a general-purpose parser and not shared with the library — the matcher knows nothing about
/// branches (FR-019) and must not learn. It exists here so the claim being measured is the one
/// SC-001 makes, rather than "eight characters of anything, including a shared prefix".
fn distinctive_part(name: &str) -> &str {
    let rest = name.split_once('/').map_or(name, |(_, rest)| rest);
    // A ticket key looks like `abc-101-` — letters, a dash, digits, a dash.
    let mut parts = rest.splitn(3, '-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(word), Some(number), Some(tail))
            if word.chars().all(|c| c.is_ascii_alphabetic())
                && !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit()) =>
        {
            tail
        }
        _ => rest,
    }
}

/// The corpus and the pairs are data, and data rots quietly: a renamed branch in `CORPUS` would
/// leave a pair pointing at nothing, and both tests above would read that as a ranking failure.
/// This says so directly instead.
#[test]
fn every_pair_names_a_branch_that_is_actually_in_the_corpus() {
    for (query, intended, _) in PAIRS {
        assert!(
            CORPUS.contains(intended),
            "the pair {query:?} → {intended:?} names a branch the corpus does not contain"
        );
    }
    assert!(
        CORPUS.len() >= 200,
        "the corpus is meant to be repository-sized (~200 names); it holds {}",
        CORPUS.len()
    );
    for (i, name) in CORPUS.iter().enumerate() {
        assert!(
            !CORPUS[i + 1..].contains(name),
            "{name:?} appears in the corpus twice"
        );
    }
}

/// All three tiers are represented, so the rate above is a measure of the whole matcher rather than
/// of literal matching with some decoration.
#[test]
fn the_pairs_cover_every_tier() {
    for tier in ["literal", "subsequence", "single-edit"] {
        let n = PAIRS.iter().filter(|(_, _, t)| *t == tier).count();
        assert!(
            n >= 10,
            "only {n} {tier} pairs; the tier needs enough to move the rate"
        );
    }
}
