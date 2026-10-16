//! Reading one GitHub issue's relationships (feature 837): the neutralised text, the model's
//! invariants, the parse of recorded GraphQL answers, the request, the timeline paging and the
//! tool result's JSON (data-model.md, contracts/details-read.md, contracts/tool-result.md).

use std::time::{Duration, Instant};

use micold_core::github::{FakeIssueSource, GithubRepo, IssueLoadError};
use micold_core::issue_details::{
    details_args, details_error_json, parse_details, read_details_with, timeline_args,
    IssueDetails, IssueDetailsSource, Kind, PlainText, PrListState, Reference, RelatedIssue,
    RelatedList, DETAILS_BOUND, LIST_CAP,
};
use micold_core::process::RunOutcome;
use serde_json::{json, Value};

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a GitHub remote")
}

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue_details")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parsed(name: &str) -> IssueDetails {
    parse_details(&fixture(name), &repo()).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn read<T>(kind: &Kind<T>) -> &T {
    match kind {
        Kind::Read(value) => value,
        Kind::NotReadable(reason) => panic!("expected a read kind, got not readable: {reason}"),
    }
}

fn refs(list: &RelatedList<RelatedIssue>) -> Vec<(String, bool)> {
    list.shown()
        .iter()
        .map(|i| (i.reference.to_string(), i.open))
        .collect()
}

fn issue(number: u64) -> RelatedIssue {
    RelatedIssue {
        reference: Reference::Here(number),
        title: PlainText::new("t"),
        open: true,
    }
}

// ---- Neutralising (FR-013, data-model §1) ----

#[test]
fn plain_text_drops_control_zero_width_and_direction_overrides() {
    let raw = "a\u{202A}b\u{202B}c\u{202C}d\u{202D}e\u{202E}f\u{2066}g\u{2067}h\u{2068}i\u{2069}j\u{200B}k\u{0007}l";
    assert_eq!(
        PlainText::new(raw).as_str(),
        "abcdefghijkl",
        "every override, isolate, zero-width and control character is dropped"
    );
}

#[test]
fn plain_text_keeps_a_joiner_between_two_visible_characters_only() {
    assert_eq!(
        PlainText::new("\u{200D}a\u{200D}b\u{200C}").as_str(),
        "a\u{200D}b",
        "a joiner shapes the letters on both sides; one at either end shapes nothing"
    );
}

#[test]
fn an_all_invisible_input_is_empty() {
    let text = PlainText::new("\u{202E}\u{200B}\u{0007}\u{200D}");
    assert!(
        text.is_empty(),
        "nothing visible is left: {:?}",
        text.as_str()
    );
}

#[test]
fn plain_text_displays_its_neutralised_text() {
    let text = PlainText::new("Fix\u{202E} it");
    assert_eq!(
        text.to_string(),
        "Fix it",
        "Display shows what as_str holds"
    );
    assert_eq!(text.as_str(), "Fix it");
}

// ---- Model invariants (FR-002, FR-003, FR-008, data-model §2-§5) ----

#[test]
fn a_related_list_keeps_githubs_order_and_shows_at_most_twenty() {
    let items: Vec<RelatedIssue> = (1..=25).map(issue).collect();
    let list = RelatedList::new(items, 25, 0);
    assert_eq!(list.shown().len(), LIST_CAP, "at most 20 are shown");
    assert_eq!(LIST_CAP, 20);
    let first: Vec<String> = list.shown()[..3]
        .iter()
        .map(|i| i.reference.to_string())
        .collect();
    assert_eq!(first, ["#1", "#2", "#3"], "GitHub's order is kept");
    assert_eq!(list.more(), 5, "more is total minus shown");
}

#[test]
fn a_related_lists_total_covers_shown_and_hidden() {
    let list = RelatedList::new(vec![issue(1), issue(2)], 1, 3);
    assert!(
        list.total() as usize >= list.shown().len() + list.hidden() as usize,
        "total {} never counts fewer than shown plus hidden",
        list.total()
    );
    assert_eq!(list.more(), list.total() - 2);
}

#[test]
fn a_related_list_is_empty_exactly_when_its_total_is_zero() {
    assert!(RelatedList::<RelatedIssue>::new(Vec::new(), 0, 0).is_empty());
    assert!(
        !RelatedList::<RelatedIssue>::new(Vec::new(), 0, 2).is_empty(),
        "hidden items still make a list that is not empty"
    );
}

#[test]
fn a_reference_is_local_when_the_repository_matches_ignoring_ascii_case() {
    let here = Reference::of("ACME/Widgets", 12, &repo());
    assert_eq!(
        here,
        Reference::Here(12),
        "GitHub compares names without case"
    );
    assert_eq!(here.to_string(), "#12");
    let other = Reference::of("acme/roadmap", 3, &repo());
    assert_eq!(other.to_string(), "acme/roadmap#3");
}

#[test]
fn a_pull_requests_state_follows_merged_closed_draft_open() {
    assert_eq!(
        PrListState::from_fields("MERGED", false, true),
        PrListState::Merged
    );
    assert_eq!(
        PrListState::from_fields("CLOSED", true, false),
        PrListState::Closed
    );
    assert_eq!(
        PrListState::from_fields("OPEN", true, false),
        PrListState::Draft
    );
    assert_eq!(
        PrListState::from_fields("OPEN", false, false),
        PrListState::Open
    );
}

#[test]
fn an_issue_is_blocked_exactly_when_an_open_blocker_is_counted() {
    assert!(parsed("every_kind.json").is_blocked(), "one open blocker");
    assert!(!parsed("empty.json").is_blocked(), "no blocker");
    let mut closed_only = parsed("every_kind.json");
    if let Kind::Read(blockers) = &mut closed_only.blocked_by {
        blockers.open = 0;
    }
    assert!(
        !closed_only.is_blocked(),
        "closed blockers block nothing (SC-003)"
    );
    let mut unreadable = parsed("every_kind.json");
    unreadable.blocked_by = Kind::NotReadable("GitHub refused access".into());
    assert!(
        !unreadable.is_blocked(),
        "a blocker kind that cannot be read is never reported as blocked"
    );
}

#[test]
fn debug_prints_the_number_and_counts_but_no_title_or_login() {
    let shown = format!("{:?}", parsed("every_kind.json"));
    assert!(shown.contains("12"), "{shown}");
    for secret in [
        "Parser rewrite",
        "octocat",
        "Q4 epic",
        "Roadmap",
        "v1.4",
        "bug",
    ] {
        assert!(!shown.contains(secret), "Debug leaked {secret:?}: {shown}");
    }
}

// ---- Parse (details-read §3, §5) ----

#[test]
fn every_kind_is_read_from_a_full_answer() {
    let d = parsed("every_kind.json");
    assert_eq!(d.number, 12);
    let blockers = read(&d.blocked_by);
    assert_eq!(
        refs(&blockers.list),
        [("#10".into(), true), ("#9".into(), false)]
    );
    assert_eq!(blockers.open, 1, "issueDependenciesSummary.blockedBy");
    assert_eq!(blockers.list.total(), 2);
    assert_eq!(blockers.list.shown()[0].title.as_str(), "Parser rewrite");
    assert_eq!(refs(read(&d.blocking)), [("#14".into(), true)]);
    let parent = read(&d.parent).as_ref().expect("a parent");
    assert_eq!(parent.reference.to_string(), "acme/roadmap#3");
    let subs = read(&d.sub_issues);
    assert_eq!(
        refs(&subs.list),
        [("#15".into(), false), ("#16".into(), true)]
    );
    assert_eq!(
        (subs.closed, subs.list.total()),
        (1, 2),
        "subIssuesSummary.completed and total"
    );
    let labels: Vec<&str> = read(&d.labels)
        .shown()
        .iter()
        .map(PlainText::as_str)
        .collect();
    assert_eq!(labels, ["bug", "parser"]);
    let assignees: Vec<&str> = read(&d.assignees)
        .shown()
        .iter()
        .map(PlainText::as_str)
        .collect();
    assert_eq!(assignees, ["octocat"]);
    assert_eq!(
        read(&d.milestone).as_ref().map(PlainText::as_str),
        Some("v1.4")
    );
}

#[test]
fn project_fields_keep_non_empty_values_and_an_empty_project_keeps_its_name() {
    let d = parsed("every_kind.json");
    let projects = read(&d.projects).shown();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].name.as_str(), "Roadmap");
    let fields: Vec<(&str, &str)> = projects[0]
        .fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert_eq!(
        fields,
        [("Status", "In progress"), ("Estimate", "3")],
        "the empty Notes value and the label value are left out; 3.0 reads as 3"
    );
    assert_eq!(projects[1].name.as_str(), "Backlog");
    assert!(projects[1].fields.is_empty());
}

#[test]
fn a_pull_request_that_closes_the_issue_is_not_also_linked() {
    let d = parsed("every_kind.json");
    let closing = read(&d.closing_prs);
    assert_eq!(closing.shown().len(), 1);
    assert_eq!(closing.shown()[0].reference.to_string(), "#20");
    assert_eq!(closing.shown()[0].state, PrListState::Draft);
    let linked = read(&d.linked_prs);
    let linked_refs: Vec<String> = linked
        .shown()
        .iter()
        .map(|p| p.reference.to_string())
        .collect();
    assert_eq!(linked_refs, ["#21"], "#20 appears once, under closing");
    assert_eq!(linked.shown()[0].state, PrListState::Merged);
    assert_eq!(linked.total(), 1);
    assert!(
        !linked.total_is_lower_bound(),
        "the timeline was read to its end"
    );
}

#[test]
fn long_lists_show_twenty_and_count_the_rest_and_the_hidden() {
    let d = parsed("many.json");
    let subs = read(&d.sub_issues);
    assert_eq!(subs.list.shown().len(), 20);
    assert_eq!(subs.list.total(), 200, "subIssuesSummary.total");
    assert_eq!(subs.list.more(), 180);
    assert_eq!(subs.list.hidden(), 175, "summary total minus totalCount");
    let blockers = read(&d.blocked_by);
    assert_eq!(blockers.list.shown().len(), 2, "the null node is not shown");
    assert_eq!(
        blockers.list.hidden(),
        3,
        "totalBlockedBy 5 minus totalCount 3, plus one null node"
    );
    assert_eq!(blockers.list.total(), 5);
    assert_eq!(blockers.open, 4, "open counts hidden blockers too");
}

#[test]
fn an_issue_with_nothing_reads_every_kind_empty() {
    let d = parsed("empty.json");
    assert!(read(&d.blocked_by).list.is_empty());
    assert!(read(&d.blocking).is_empty());
    assert!(read(&d.parent).is_none());
    assert!(read(&d.sub_issues).list.is_empty());
    assert!(read(&d.labels).is_empty());
    assert!(read(&d.assignees).is_empty());
    assert!(read(&d.milestone).is_none());
    assert!(read(&d.projects).is_empty());
    assert!(read(&d.closing_prs).is_empty());
    assert!(read(&d.linked_prs).is_empty());
}

#[test]
fn a_missing_scope_marks_only_projects_not_readable() {
    let d = parsed("projects_scope.json");
    assert_eq!(
        d.projects,
        Kind::NotReadable(
            "your GitHub sign-in can't read projects (it lacks the read:project scope)".into()
        )
    );
    assert!(
        matches!(d.blocked_by, Kind::Read(_)),
        "every other kind is read"
    );
    assert!(matches!(d.labels, Kind::Read(_)));
    assert!(matches!(d.linked_prs, Kind::Read(_)));
}

#[test]
fn kinds_the_server_cannot_provide_are_not_readable() {
    let d = parsed("unsupported.json");
    assert!(matches!(&d.blocked_by, Kind::NotReadable(r) if *r == "GitHub could not provide it"));
    assert_eq!(d.sub_issues.reason(), Some("GitHub could not provide it"));
    assert_eq!(d.parent.reason(), Some("GitHub could not provide it"));
    assert!(matches!(d.blocking, Kind::Read(_)), "blocking was answered");
}

#[test]
fn a_forbidden_kind_says_github_refused_access() {
    let answer = json!({
        "data": {"repository": {"issue": {"number": 12, "milestone": null}}},
        "errors": [{"type": "FORBIDDEN", "path": ["repository", "issue", "milestone"],
                    "message": "Resource not accessible"}]
    });
    let d = parse_details(answer.to_string().as_bytes(), &repo()).expect("issue present");
    assert_eq!(d.milestone.reason(), Some("GitHub refused access"));
}

#[test]
fn whole_answer_failures_are_load_errors() {
    match parse_details(b"<html>", &repo()) {
        Err(IssueLoadError::Other(m)) => {
            assert!(m.starts_with("GitHub's answer could not be read: "), "{m}")
        }
        other => panic!("not JSON: {other:?}"),
    }
    assert_eq!(
        parse_details(&fixture("rate_limited.json"), &repo()).unwrap_err(),
        IssueLoadError::RateLimited,
        "the existing GraphQL error typing"
    );
    assert_eq!(
        parse_details(&fixture("repository_null.json"), &repo()).unwrap_err(),
        IssueLoadError::NoAccess
    );
    assert_eq!(
        parse_details(&fixture("issue_null.json"), &repo()).unwrap_err(),
        IssueLoadError::Other("GitHub's answer had no issue".into())
    );
}

/// Review A M1: a typed error beside a `null` issue names that error, not "no issue".
#[test]
fn a_null_issue_with_a_typed_error_reports_that_error() {
    let answer = json!({"data": {"repository": {"issue": null}},
        "errors": [{"type": "RATE_LIMITED", "path": ["repository", "issue"], "message": "slow down"}]});
    assert_eq!(
        parse_details(answer.to_string().as_bytes(), &repo()).unwrap_err(),
        IssueLoadError::RateLimited
    );
}

#[test]
fn text_from_github_is_neutralised_and_emptied_values_are_left_out() {
    let d = parsed("bidi_titles.json");
    assert_eq!(
        read(&d.blocked_by).list.shown()[0].title.as_str(),
        "Fixevil a\u{200D}b"
    );
    let labels: Vec<&str> = read(&d.labels)
        .shown()
        .iter()
        .map(PlainText::as_str)
        .collect();
    assert_eq!(labels, ["bug"], "a label with nothing visible is left out");
    assert_eq!(read(&d.assignees).shown()[0].as_str(), "octocat");
    assert_eq!(
        read(&d.milestone).as_ref().map(PlainText::as_str),
        Some("v1.4")
    );
    let project = &read(&d.projects).shown()[0];
    assert_eq!(project.name.as_str(), "Roadmap");
    let fields: Vec<(&str, &str)> = project
        .fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert_eq!(
        fields,
        [("Status", "ok!")],
        "a value with nothing visible is left out"
    );
}

// ---- Request, paging, sources (details-read §1, §2, §4) ----

#[test]
fn the_request_sends_only_owner_name_and_number() {
    let args = details_args(&repo(), 12);
    assert_eq!(&args[..4], ["api", "graphql", "--hostname", "github.com"]);
    let vars: Vec<&str> = args[4..]
        .chunks(2)
        .map(|pair| pair[1].split('=').next().unwrap_or_default())
        .collect();
    assert_eq!(
        vars,
        ["query", "owner", "name", "n"],
        "FR-009: nothing else leaves"
    );
    assert!(args.contains(&"owner=acme".to_string()));
    assert!(args.contains(&"name=widgets".to_string()));
    assert!(args.contains(&"n=12".to_string()));
    assert!(
        args.iter().all(|a| !a.contains('\n')),
        "one line per argument"
    );
    let timeline = timeline_args(&repo(), 12, "c1");
    let extra: Vec<&String> = timeline.iter().filter(|a| !args.contains(a)).collect();
    assert_eq!(
        extra
            .iter()
            .filter(|a| !a.starts_with("query="))
            .collect::<Vec<_>>(),
        [&&"after=c1".to_string()],
        "the timeline adds only the cursor"
    );
}

fn answered(stdout: Vec<u8>) -> RunOutcome {
    RunOutcome::Exited {
        code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn later() -> Instant {
    Instant::now() + DETAILS_BOUND
}

fn linked_refs(d: &IssueDetails) -> Vec<String> {
    read(&d.linked_prs)
        .shown()
        .iter()
        .map(|p| p.reference.to_string())
        .collect()
}

#[test]
fn the_timeline_is_paged_while_github_has_more() {
    let mut pages = vec![
        fixture("timeline_page1.json"),
        fixture("timeline_page2.json"),
    ]
    .into_iter();
    let mut seen: Vec<Vec<String>> = Vec::new();
    let d = read_details_with(&repo(), 12, later(), |args, _left| {
        seen.push(args);
        answered(pages.next().expect("no third run"))
    })
    .expect("read");
    assert_eq!(linked_refs(&d), ["#30", "#31"]);
    assert!(!read(&d.linked_prs).total_is_lower_bound());
    assert_eq!(seen.len(), 2);
    assert!(seen[1].contains(&"after=c1".to_string()), "{:?}", seen[1]);
}

#[test]
fn paging_stops_after_ten_pages_and_marks_a_lower_bound() {
    let mut runs = 0;
    let d = read_details_with(&repo(), 12, later(), |_, _| {
        runs += 1;
        if runs == 1 {
            answered(fixture("timeline_page1.json"))
        } else {
            let page = json!({"data": {"repository": {"issue": {"timelineItems": {
                "pageInfo": {"hasNextPage": true, "endCursor": format!("c{runs}")},
                "nodes": []}}}}});
            answered(page.to_string().into_bytes())
        }
    })
    .expect("read");
    assert_eq!(runs, 10, "the first page and nine more");
    assert!(read(&d.linked_prs).total_is_lower_bound());
}

#[test]
fn paging_stops_at_the_deadline_and_marks_a_lower_bound() {
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut runs = 0;
    let d = read_details_with(&repo(), 12, deadline, |_, left| {
        runs += 1;
        assert!(
            left <= Duration::from_millis(50),
            "each run is bounded by the time left"
        );
        std::thread::sleep(Duration::from_millis(60));
        answered(fixture("timeline_page1.json"))
    })
    .expect("the first page was read");
    assert_eq!(runs, 1, "no run starts after the deadline");
    assert!(read(&d.linked_prs).total_is_lower_bound());
}

#[test]
fn a_failed_page_keeps_what_was_read() {
    let mut runs = 0;
    let d = read_details_with(&repo(), 12, later(), |_, _| {
        runs += 1;
        if runs == 1 {
            answered(fixture("timeline_page1.json"))
        } else {
            RunOutcome::TimedOut {
                stderr: String::new(),
            }
        }
    })
    .expect("a failed page never fails the read");
    assert_eq!(linked_refs(&d), ["#30"]);
    assert!(read(&d.linked_prs).total_is_lower_bound());
}

#[test]
fn a_read_past_its_deadline_starts_nothing() {
    let mut runs = 0;
    let result = read_details_with(&repo(), 12, Instant::now(), |_, _| {
        runs += 1;
        answered(fixture("empty.json"))
    });
    assert_eq!(result.unwrap_err(), IssueLoadError::TimedOut);
    assert_eq!(runs, 0);
}

#[test]
fn the_fake_answers_an_error_when_nothing_is_scripted() {
    let fake = FakeIssueSource::new();
    assert_eq!(
        fake.read_details(&repo(), 12, later()).unwrap_err(),
        IssueLoadError::Other("no details scripted".into())
    );
    assert_eq!(fake.details_calls(), [("acme/widgets".to_string(), 12)]);
}

#[test]
fn the_fake_returns_the_scripted_details_after_its_delay() {
    let fake = FakeIssueSource::new()
        .with_details(Ok(parsed("every_kind.json")))
        .with_details_delay(Duration::from_millis(100));
    let began = Instant::now();
    let d = fake.read_details(&repo(), 12, later()).expect("scripted");
    assert!(
        began.elapsed() >= Duration::from_millis(100),
        "the delay applies"
    );
    assert!(d.is_blocked());
    assert_eq!(fake.details_calls(), [("acme/widgets".to_string(), 12)]);
}

// ---- Tool result (tool-result.md) ----

#[test]
fn every_kind_appears_in_the_tool_result() {
    let value = parsed("every_kind.json").to_json();
    let expected = json!({
        "number": 12,
        "blocked": true,
        "blocked_by": {"open": 1, "total": 2, "more": 0, "items": [
            {"ref": "#10", "title": "Parser rewrite", "state": "open"},
            {"ref": "#9", "title": "Old spike", "state": "closed"}]},
        "blocking": {"total": 1, "more": 0, "items": [
            {"ref": "#14", "title": "Release notes", "state": "open"}]},
        "parent": {"ref": "acme/roadmap#3", "title": "Q4 epic", "state": "open"},
        "sub_issues": {"closed": 1, "total": 2, "more": 0, "items": [
            {"ref": "#15", "title": "Tokenizer", "state": "closed"},
            {"ref": "#16", "title": "Grammar", "state": "open"}]},
        "labels": {"total": 2, "more": 0, "items": ["bug", "parser"]},
        "assignees": {"total": 1, "more": 0, "items": ["octocat"]},
        "milestone": "v1.4",
        "projects": [
            {"name": "Roadmap", "fields": {"Status": "In progress", "Estimate": "3"}},
            {"name": "Backlog", "fields": {}}],
        "closing_pull_requests": {"total": 1, "more": 0, "items": [
            {"ref": "#20", "title": "Fix the parser", "state": "draft"}]},
        "linked_pull_requests": {"total": 1, "more": 0, "items": [
            {"ref": "#21", "title": "Refactor lexer", "state": "merged"}]},
    });
    assert_eq!(value, expected);
}

#[test]
fn an_issue_with_nothing_gives_only_its_number() {
    assert_eq!(parsed("empty.json").to_json(), json!({"number": 12}));
}

#[test]
fn blocked_is_never_false() {
    let mut closed_only = parsed("every_kind.json");
    if let Kind::Read(blockers) = &mut closed_only.blocked_by {
        blockers.open = 0;
    }
    let value = closed_only.to_json();
    assert!(value.get("blocked").is_none(), "absent, not false: {value}");
    assert_eq!(value["blocked_by"]["open"], 0);
}

#[test]
fn lists_cap_items_and_report_more_and_hidden() {
    let value = parsed("many.json").to_json();
    let subs = &value["sub_issues"];
    assert_eq!(subs["items"].as_array().map(Vec::len), Some(20));
    assert_eq!(subs["more"], 180);
    assert_eq!(subs["hidden"], 175);
    assert_eq!(value["blocked_by"]["hidden"], 3);
    let every = parsed("every_kind.json").to_json();
    assert!(
        every["blocking"].get("hidden").is_none(),
        "hidden 0 is omitted"
    );
}

#[test]
fn a_lower_bound_is_marked_on_linked_pull_requests_only() {
    let d = read_details_with(&repo(), 12, later(), |args, _| {
        if args.iter().any(|a| a.starts_with("after=")) {
            RunOutcome::TimedOut {
                stderr: String::new(),
            }
        } else {
            answered(fixture("timeline_page1.json"))
        }
    })
    .expect("read");
    let value = d.to_json();
    assert_eq!(value["linked_pull_requests"]["total_is_lower_bound"], true);
    let every = parsed("every_kind.json").to_json();
    assert!(every["linked_pull_requests"]
        .get("total_is_lower_bound")
        .is_none());
}

#[test]
fn projects_past_twenty_add_projects_more() {
    let nodes: Vec<Value> = (0..23)
        .map(|i| json!({"project": {"title": format!("P{i}")}, "fieldValues": {"nodes": []}}))
        .collect();
    let mut answer: Value = serde_json::from_slice(&fixture("empty.json")).unwrap();
    answer["data"]["repository"]["issue"]["projectItems"] =
        json!({"totalCount": 23, "nodes": nodes});
    let value = parse_details(answer.to_string().as_bytes(), &repo())
        .expect("read")
        .to_json();
    assert_eq!(value["projects"].as_array().map(Vec::len), Some(20));
    assert_eq!(value["projects_more"], 3);
}

#[test]
fn a_kind_that_cannot_be_read_appears_only_under_not_readable() {
    let value = parsed("projects_scope.json").to_json();
    assert!(value.get("projects").is_none(), "{value}");
    assert_eq!(
        value["not_readable"],
        json!({"projects": "your GitHub sign-in can't read projects (it lacks the read:project scope)"})
    );
}

#[test]
fn a_whole_read_failure_is_one_sentence() {
    assert_eq!(
        details_error_json(12, &IssueLoadError::TimedOut, &repo()),
        json!({"number": 12, "details_error": IssueLoadError::TimedOut.message(&repo())})
    );
}
