//! Filter clauses (feature 1185, contracts/jira-filter-source.md §1): the one place a chosen filter
//! value becomes query text, always as one quoted, escaped string sent whole (FR-010, SC-002,
//! SC-003).

use micold_core::jira::{list_jql, search_jql, ProjectKey};
use micold_core::jira_filter::{
    jql_string, narrowed, AccountId, FilterKind, JiraFilters, JiraUser,
};

fn key() -> ProjectKey {
    ProjectKey::parse("ABC").expect("a valid key")
}

fn user(account: &str, name: &str) -> JiraUser {
    JiraUser {
        account: AccountId::parse(account).expect("an account id"),
        name: name.to_owned(),
        is_me: false,
    }
}

/// Undo `jql_string`: `None` unless `quoted` is exactly one JQL string (an opening quote, escapes
/// only of `\` and `"`, an unescaped quote only at the end).
fn unquote(quoted: &str) -> Option<String> {
    let inner = quoted.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                escaped @ ('\\' | '"') => out.push(escaped),
                _ => return None,
            },
            '"' => return chars.next().is_none().then_some(out),
            other => out.push(other),
        }
    }
    None
}

#[test]
fn a_value_is_quoted_with_backslash_then_quote_escaped() {
    assert_eq!(jql_string("Payments"), r#""Payments""#);
    assert_eq!(jql_string(r#"a"b"#), r#""a\"b""#);
    assert_eq!(jql_string(r"a\b"), r#""a\\b""#);
    assert_eq!(jql_string(r#"\""#), r#""\\\"""#);
}

#[test]
fn a_value_is_never_trimmed_cut_or_case_changed() {
    let long = "x".repeat(150);
    for value in ["  Edge ", "MiXeD", long.as_str()] {
        assert_eq!(unquote(&jql_string(value)).as_deref(), Some(value));
    }
}

#[test]
fn every_value_of_the_corpus_is_exactly_one_quoted_string() {
    let long = "Release ".repeat(20);
    let corpus = [
        "Payments",
        r#"a"b"#,
        r"a\b",
        r#"a" OR project = "XYZ"#,
        r#"a" OR project = "XYZ\"#,
        r"\",
        r#"""#,
        r#"\""#,
        " leading",
        "trailing ",
        "  both  ",
        long.as_str(),
        "Ä \"quoted\" \\ name",
        "Zahlungsverkehr Ü",
        "支払い",
        "платежи",
        "😀 emoji",
        "fix'version",
        "a AND b",
        "ORDER BY updated",
        "x)",
        "%25 encoded",
    ];
    assert!(corpus.len() >= 20);
    for value in corpus {
        let quoted = jql_string(value);
        assert_eq!(
            unquote(&quoted).as_deref(),
            Some(value),
            "{value:?} became {quoted}"
        );
    }
}

#[test]
fn setters_refuse_an_empty_value_or_a_control_character_and_keep_the_old_one() {
    let mut filters = JiraFilters::default();
    assert!(filters.set_component(Some("Payments".into())));
    for refused in ["", "a\nb", "\u{7}", "tab\there"] {
        assert!(!filters.set_component(Some(refused.into())), "{refused:?}");
        assert!(!filters.set_label(Some(refused.into())), "{refused:?}");
        assert!(
            !filters.set_fix_version(Some(refused.into())),
            "{refused:?}"
        );
    }
    assert!(!filters.set_reporter(Some(user("acc-1", ""))));
    assert!(!filters.set_reporter(Some(user("acc-1", "a\u{0}b"))));
    assert_eq!(filters.component(), Some("Payments"));
    assert_eq!(filters.label(), None);
    assert_eq!(filters.fix_version(), None);
    assert!(filters.reporter().is_none());
    assert!(filters.set_component(None), "clearing is always accepted");
    assert!(filters.is_empty());
}

fn all_four() -> JiraFilters {
    let mut filters = JiraFilters::default();
    assert!(filters.set_fix_version(Some("2.4".into())));
    assert!(filters.set_component(Some("Payments".into())));
    assert!(filters.set_label(Some("backend".into())));
    assert!(filters.set_reporter(Some(user("acc-1", "Ann"))));
    filters
}

#[test]
fn active_names_the_chosen_filters_in_the_fixed_order_and_clear_all_empties() {
    let mut filters = all_four();
    assert!(!filters.is_empty());
    assert_eq!(
        filters.active(),
        [
            (FilterKind::Reporter, "Ann".to_string()),
            (FilterKind::Label, "backend".to_string()),
            (FilterKind::Component, "Payments".to_string()),
            (FilterKind::FixVersion, "2.4".to_string()),
        ]
    );
    filters.clear(FilterKind::Label);
    assert_eq!(filters.active().len(), 3);
    filters.clear_all();
    assert!(filters.is_empty());
    assert!(filters.active().is_empty());
    assert!(filters.clauses().is_empty());
}

#[test]
fn clauses_follow_the_fixed_order_for_all_sixteen_combinations() {
    let expected = [
        r#"reporter = "acc-1""#,
        r#"labels = "backend""#,
        r#"component = "Payments""#,
        r#"fixVersion = "2.4""#,
    ];
    for mask in 0u8..16 {
        let mut filters = JiraFilters::default();
        if mask & 1 != 0 {
            assert!(filters.set_reporter(Some(user("acc-1", "Ann"))));
        }
        if mask & 2 != 0 {
            assert!(filters.set_label(Some("backend".into())));
        }
        if mask & 4 != 0 {
            assert!(filters.set_component(Some("Payments".into())));
        }
        if mask & 8 != 0 {
            assert!(filters.set_fix_version(Some("2.4".into())));
        }
        let want: Vec<&str> = (0..4)
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| expected[bit])
            .collect();
        assert_eq!(filters.clauses(), want, "combination {mask:04b}");
        assert_eq!(filters.is_empty(), mask == 0);
    }
}

#[test]
fn the_reporter_clause_names_the_account_never_the_display_name() {
    let mut filters = JiraFilters::default();
    assert!(filters.set_reporter(Some(user("5b10ac8d82e05b22cc7d4ef5", "Ann Lee"))));
    assert_eq!(
        filters.clauses(),
        [r#"reporter = "5b10ac8d82e05b22cc7d4ef5""#]
    );
}

#[test]
fn narrowed_without_a_filter_is_the_query_unchanged() {
    let jql = list_jql(&key());
    assert_eq!(narrowed(&jql, &JiraFilters::default()), jql);
}

#[test]
fn narrowed_inserts_the_clauses_before_order_by() {
    let mut filters = JiraFilters::default();
    assert!(filters.set_component(Some("Payments".into())));
    assert!(filters.set_fix_version(Some("2.4".into())));
    assert_eq!(
        narrowed(&list_jql(&key()), &filters),
        r#"project = "ABC" AND statusCategory != Done AND component = "Payments" AND fixVersion = "2.4" ORDER BY updated DESC"#
    );
    let mut injected = JiraFilters::default();
    assert!(injected.set_label(Some(r#"a" OR project = "XYZ\"#.into())));
    assert_eq!(
        narrowed(&list_jql(&key()), &injected),
        r#"project = "ABC" AND statusCategory != Done AND labels = "a\" OR project = \"XYZ\\" ORDER BY updated DESC"#
    );
}

#[test]
fn narrowed_inserts_before_the_last_order_by_and_appends_when_there_is_none() {
    let mut filters = JiraFilters::default();
    assert!(filters.set_component(Some("C".into())));
    let search = search_jql(&key(), "ORDER BY x").expect("a search");
    let narrowed_search = narrowed(&search, &filters);
    assert!(
        narrowed_search.ends_with(r#" AND component = "C" ORDER BY updated DESC"#),
        "{narrowed_search}"
    );
    assert_eq!(
        narrowed("project = ABC", &filters),
        r#"project = ABC AND component = "C""#
    );
}

#[test]
fn the_search_text_is_still_cut_while_a_filter_value_is_sent_whole() {
    let text = "s".repeat(150);
    let value = "v".repeat(150);
    let mut filters = JiraFilters::default();
    assert!(filters.set_component(Some(value.clone())));
    let jql = narrowed(&search_jql(&key(), &text).expect("a search"), &filters);
    assert!(jql.contains(&format!("summary ~ \"{}*\"", "s".repeat(100))));
    assert!(!jql.contains(&"s".repeat(101)));
    assert!(jql.contains(&format!("component = \"{value}\"")));
}
