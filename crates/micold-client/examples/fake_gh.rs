//! A development-only stand-in for `gh` for the manual pass (feature 1400, research R16,
//! quickstart §B).
//!
//! ```text
//! cargo build -p micold-client --example fake_gh
//! mkdir -p /tmp/fake-gh && ln -sf "$PWD/target-shared/debug/examples/fake_gh" /tmp/fake-gh/gh
//! # put /tmp/fake-gh first on the PATH the app sees (the environment include, or the app's PATH)
//! ```
//!
//! It answers `gh api graphql … -f query=<document> -f <name>=<value>…` from a built-in repository
//! of 30 issues, open and closed, labelled `in-progress`, `blocked`, `bug`, `ui`, `good first issue`
//! (a label with spaces) and `say "hi"` (a label with a quote, which the selector cannot offer).
//! The repository takes the name the request asks for, so any project with a GitHub remote works.
//!
//! - The open-issue listing and its description pass (034): every open issue, most recently
//!   updated first, in pages of 100.
//! - A search (034's search beyond and feature 1400's filtered listing, advanced query and
//!   search): honours `repo:`, `is:open` / `is:closed`, `state:open` / `state:closed`,
//!   `label:"…"`, `-label:"…"`, (US6) `assignee:<login>|@me`, `no:assignee`, `author:<login>|@me`,
//!   `milestone:"…"`, `no:milestone` (`@me` is `octocat`) and `sort:updated-desc|asc`
//!   (`created-…` sorts by number);
//!   parentheses outside quotes are ignored (no `OR`); every other word, quoted or not, must appear
//!   in the title (any case).
//! - A single issue's details (837), with its labels and no relations.
//! - The labels: every label, in one page.
//! - (US6) The open milestones `v2.4`, `Sprint 12: polish`, `Backlog` and `say "hi"` (a title
//!   with a quote, not offered), in one page; issues are spread over the first two and none.
//! - (US6) The assignable users `octocat` (The Octocat), `monalisa` (Mona Lisa), `hubot` and
//!   `octo-dog`, those whose login or name holds the typed `q` (any case); issues are assigned to
//!   the first three or no one, and opened by `octocat` or `monalisa`.
//!
//! Each search's `q` is written to stderr, so a step can check what a filtered request sent.
//! Anything else is answered with a GraphQL error.
//!
//! `FAKE_GH_MODE`, a comma-separated list, sets up the states a pass needs without a wrapper:
//!
//! - `slow`: every answer waits 3 seconds first (the loading states).
//! - `labels-fail`: the labels fail as `gh` does on `HTTP 502` (exit 1, the reason on stderr).
//! - `no-labels`: the repository has no labels.
//! - `labels-cap`: every labels page is full and has a next one, so only the first 1,000 load.
//! - `search-refused`: every search is refused with a GraphQL error at `search`.
//! - `reject` (also as `MICOLD_FAKE_GH=reject`, quickstart B10): a search holding a qualifier the
//!   fake does not know outside quotes (a typo such as `lable:bug`) is refused with a GraphQL error
//!   at `search` carrying two messages, as GitHub refuses an invalid query; fix it and the next
//!   search answers.

use std::collections::BTreeMap;

const LABELS: &[&str] = &[
    "blocked",
    "bug",
    "good first issue",
    "in-progress",
    "say \"hi\"",
    "ui",
];

/// The assignable users: login and display name.
const PEOPLE: &[(&str, Option<&str>)] = &[
    ("octocat", Some("The Octocat")),
    ("monalisa", Some("Mona Lisa")),
    ("hubot", None),
    ("octo-dog", None),
];

/// The open milestones, in GitHub's (unsorted) order.
const MILESTONES: &[&str] = &["v2.4", "Sprint 12: polish", "Backlog", "say \"hi\""];

/// Who `@me` is.
const ME: &str = "octocat";

struct FakeIssue {
    number: u64,
    title: String,
    labels: Vec<&'static str>,
    closed: bool,
    assignee: Option<&'static str>,
    author: &'static str,
    milestone: Option<&'static str>,
    /// Minutes before the newest update: smaller is more recent.
    age: u64,
}

/// The built-in repository: 30 issues whose labels and states cycle, so every combination the
/// pass filters by has members.
fn issues() -> Vec<FakeIssue> {
    const TITLES: &[&str] = &[
        "Crash when opening an empty project",
        "Sidebar flickers on resize",
        "Document the sandbox placement",
        "Crash in the terminal on paste",
        "Settings page loses focus",
        "Slow start on large repositories",
    ];
    (1..=30u64)
        .map(|n| {
            let mut labels = Vec::new();
            if n % 3 == 0 {
                labels.push("in-progress");
            }
            if n % 4 == 0 {
                labels.push("blocked");
            }
            if n % 2 == 1 {
                labels.push("bug");
            }
            if n % 5 == 0 {
                labels.push("ui");
            }
            if n == 7 || n == 11 {
                labels.push("good first issue");
            }
            if n == 13 {
                labels.push("say \"hi\"");
            }
            FakeIssue {
                number: n,
                title: format!("{} ({n})", TITLES[(n as usize) % TITLES.len()]),
                labels,
                closed: n % 7 == 0,
                assignee: match n % 4 {
                    0 => None,
                    1 => Some("octocat"),
                    2 => Some("monalisa"),
                    _ => Some("hubot"),
                },
                author: if n % 2 == 0 { "octocat" } else { "monalisa" },
                milestone: match n % 3 {
                    0 => Some("v2.4"),
                    1 => Some("Sprint 12: polish"),
                    _ => None,
                },
                age: (n * 37) % 101,
            }
        })
        .collect()
}

fn node(issue: &FakeIssue, repo: &str) -> serde_json::Value {
    serde_json::json!({
        "number": issue.number,
        "title": issue.title,
        "updatedAt": format!("2026-10-{:02}T{:02}:{:02}:00Z", 26 - issue.age / 10, 23 - issue.age % 10, issue.number % 60),
        "labels": { "nodes": issue.labels.iter().map(|l| serde_json::json!({ "name": l })).collect::<Vec<_>>() },
        "author": { "login": issue.author },
        "bodyText": format!("The description of issue {}.", issue.number),
        "state": if issue.closed { "CLOSED" } else { "OPEN" },
        "repository": { "nameWithOwner": repo },
    })
}

/// The search query's terms, as the fake reads them.
#[derive(Default)]
struct Terms {
    repo: String,
    state: Option<bool>,
    include: Vec<String>,
    exclude: Vec<String>,
    /// `Some(None)`: `no:assignee`.
    assignee: Option<Option<String>>,
    author: Option<String>,
    /// `Some(None)`: `no:milestone`.
    milestone: Option<Option<String>>,
    words: Vec<String>,
    ascending: bool,
    by_number: bool,
}

/// Split `q` into tokens, a quoted run kept whole (its quotes removed); parentheses outside quotes
/// separate tokens and are dropped. Each token says whether any of it was quoted.
fn quoted_tokens(q: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut had_quote = false;
    for c in q.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                had_quote = true;
            }
            c if (c.is_whitespace() || c == '(' || c == ')') && !quoted => {
                if !current.is_empty() {
                    out.push((std::mem::take(&mut current), had_quote));
                }
                had_quote = false;
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push((current, had_quote));
    }
    out
}

fn tokens(q: &str) -> Vec<String> {
    quoted_tokens(q)
        .into_iter()
        .map(|(token, _)| token)
        .collect()
}

/// The qualifiers the fake reads; any other `name:` outside quotes is one GitHub would judge.
const KNOWN: &[&str] = &[
    "repo:",
    "is:",
    "state:",
    "label:",
    "-label:",
    "sort:",
    "no:",
    "assignee:",
    "author:",
    "milestone:",
];

/// A login as a search names it: `@me` is [`ME`].
fn person(login: &str) -> String {
    if login == "@me" { ME } else { login }.to_string()
}

/// A qualifier outside quotes the fake does not know, as `reject` mode refuses it.
fn unknown_qualifier(q: &str) -> Option<String> {
    quoted_tokens(q)
        .into_iter()
        .filter(|(token, quoted)| !quoted && token.contains(':'))
        .map(|(token, _)| token)
        .find(|token| !KNOWN.iter().any(|known| token.starts_with(known)))
}

fn terms(q: &str) -> Terms {
    let mut terms = Terms::default();
    for token in tokens(q) {
        if let Some(repo) = token.strip_prefix("repo:") {
            terms.repo = repo.to_string();
        } else if token == "is:issue" {
        } else if token == "is:open" || token == "state:open" {
            terms.state = Some(false);
        } else if token == "is:closed" || token == "state:closed" {
            terms.state = Some(true);
        } else if let Some(label) = token.strip_prefix("-label:") {
            terms.exclude.push(label.to_string());
        } else if let Some(label) = token.strip_prefix("label:") {
            terms.include.push(label.to_string());
        } else if token == "no:assignee" {
            terms.assignee = Some(None);
        } else if token == "no:milestone" {
            terms.milestone = Some(None);
        } else if let Some(login) = token.strip_prefix("assignee:") {
            terms.assignee = Some(Some(person(login)));
        } else if let Some(login) = token.strip_prefix("author:") {
            terms.author = Some(person(login));
        } else if let Some(title) = token.strip_prefix("milestone:") {
            terms.milestone = Some(Some(title.to_string()));
        } else if let Some(sort) = token.strip_prefix("sort:") {
            terms.ascending = sort.ends_with("-asc");
            terms.by_number = sort.starts_with("created");
        } else {
            terms.words.push(token.to_lowercase());
        }
    }
    terms
}

fn search(q: &str) -> serde_json::Value {
    eprintln!("fake_gh: q={q}");
    let terms = terms(q);
    let mut found: Vec<FakeIssue> = issues()
        .into_iter()
        .filter(|i| terms.state.is_none_or(|closed| i.closed == closed))
        .filter(|i| terms.include.iter().all(|l| i.labels.contains(&l.as_str())))
        .filter(|i| !terms.exclude.iter().any(|l| i.labels.contains(&l.as_str())))
        .filter(|i| {
            terms
                .assignee
                .as_ref()
                .is_none_or(|a| a.as_deref() == i.assignee)
        })
        .filter(|i| terms.author.as_deref().is_none_or(|a| a == i.author))
        .filter(|i| {
            terms
                .milestone
                .as_ref()
                .is_none_or(|m| m.as_deref() == i.milestone)
        })
        .filter(|i| {
            let title = i.title.to_lowercase();
            terms.words.iter().all(|w| title.contains(w.as_str()))
        })
        .collect();
    // Newest first, then reversed for `-asc`: a higher number was created later.
    if terms.by_number {
        found.sort_by_key(|i| std::cmp::Reverse(i.number));
    } else {
        found.sort_by_key(|i| i.age);
    }
    if terms.ascending {
        found.reverse();
    }
    let nodes: Vec<_> = found.iter().map(|i| node(i, &terms.repo)).collect();
    serde_json::json!({ "data": { "search": {
        "issueCount": nodes.len(),
        "pageInfo": { "hasNextPage": false, "endCursor": null },
        "nodes": nodes,
    } } })
}

/// The single-issue details read (837): the issue's labels and no relations, so picking an issue
/// shows its details rather than "Couldn't read this issue's details". `null` for a number the
/// repository lacks.
fn details(number: u64) -> serde_json::Value {
    let empty = serde_json::json!({ "totalCount": 0, "nodes": [] });
    let Some(issue) = issues().into_iter().find(|i| i.number == number) else {
        return serde_json::json!({ "data": { "repository": { "issue": null } } });
    };
    let labels: Vec<_> = issue
        .labels
        .iter()
        .map(|l| serde_json::json!({ "name": l }))
        .collect();
    serde_json::json!({ "data": { "repository": { "issue": {
        "number": number,
        "issueDependenciesSummary": { "blockedBy": 0, "totalBlockedBy": 0, "blocking": 0, "totalBlocking": 0 },
        "blockedBy": empty, "blocking": empty, "parent": null,
        "subIssuesSummary": { "total": 0, "completed": 0 }, "subIssues": empty,
        "labels": { "totalCount": labels.len(), "nodes": labels },
        "assignees": empty, "milestone": null, "projectItems": empty,
        "closedByPullRequestsReferences": empty,
        "timelineItems": { "nodes": [], "pageInfo": { "hasNextPage": false, "endCursor": null } },
    } } } })
}

fn listing(repo: &str) -> serde_json::Value {
    let mut open: Vec<FakeIssue> = issues().into_iter().filter(|i| !i.closed).collect();
    open.sort_by_key(|i| i.age);
    let nodes: Vec<_> = open.iter().map(|i| node(i, repo)).collect();
    serde_json::json!({ "data": { "repository": { "issues": {
        "totalCount": nodes.len(),
        "pageInfo": { "hasNextPage": false, "endCursor": null },
        "nodes": nodes,
    } } } })
}

fn labels(modes: &[&str], after: Option<&str>) -> serde_json::Value {
    let (names, next): (Vec<String>, Option<String>) = if modes.contains(&"no-labels") {
        (Vec::new(), None)
    } else if modes.contains(&"labels-cap") {
        // A full page and a next one, each page's names its own, for as long as it is asked.
        let page: u32 = after.and_then(|c| c.parse().ok()).unwrap_or(0);
        let names = (0..100)
            .map(|i| format!("label-{page:02}-{i:02}"))
            .collect();
        (names, Some((page + 1).to_string()))
    } else {
        (LABELS.iter().map(|l| (*l).to_string()).collect(), None)
    };
    let nodes: Vec<_> = names
        .iter()
        .map(|l| serde_json::json!({ "name": l }))
        .collect();
    serde_json::json!({ "data": { "repository": { "labels": {
        "pageInfo": { "hasNextPage": next.is_some(), "endCursor": next },
        "nodes": nodes,
    } } } })
}

/// The open milestones, in one page (US6).
fn milestones() -> serde_json::Value {
    let nodes: Vec<_> = MILESTONES
        .iter()
        .map(|t| serde_json::json!({ "title": t }))
        .collect();
    serde_json::json!({ "data": { "repository": { "milestones": {
        "pageInfo": { "hasNextPage": false, "endCursor": null },
        "nodes": nodes,
    } } } })
}

/// The assignable users whose login or name holds `q`, any case (US6).
fn people(q: &str) -> serde_json::Value {
    eprintln!("fake_gh: people q={q}");
    let q = q.to_lowercase();
    let nodes: Vec<_> = PEOPLE
        .iter()
        .filter(|(login, name)| {
            login.contains(q.as_str())
                || name.is_some_and(|n| n.to_lowercase().contains(q.as_str()))
        })
        .map(|(login, name)| serde_json::json!({ "login": login, "name": name }))
        .collect();
    serde_json::json!({ "data": { "repository": { "assignableUsers": { "nodes": nodes } } } })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--version") {
        println!("gh version 2.99.0 (fake)");
        return;
    }
    // `-f name=value` pairs.
    let fields: BTreeMap<String, String> = args
        .windows(2)
        .filter(|pair| pair[0] == "-f" || pair[0] == "-F")
        .filter_map(|pair| {
            let (name, value) = pair[1].split_once('=')?;
            Some((name.to_string(), value.to_string()))
        })
        .collect();
    let query = fields.get("query").cloned().unwrap_or_default();
    let repo = match (fields.get("owner"), fields.get("name")) {
        (Some(owner), Some(name)) => format!("{owner}/{name}"),
        _ => String::new(),
    };
    let mode = std::env::var("FAKE_GH_MODE").unwrap_or_default();
    let micold = std::env::var("MICOLD_FAKE_GH").unwrap_or_default();
    let modes: Vec<&str> = mode
        .split(',')
        .chain(micold.split(','))
        .map(str::trim)
        .collect();
    if modes.contains(&"slow") {
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    // The listing asks for each issue's `labels(first: 20)` too, so it is told apart first.
    let answer = if query.contains("issueDependenciesSummary") {
        details(
            fields
                .get("n")
                .and_then(|n| n.parse().ok())
                .unwrap_or_default(),
        )
    } else if query.contains("search(") {
        if modes.contains(&"search-refused") {
            eprintln!("fake_gh: q={}", fields.get("q").map_or("", String::as_str));
            serde_json::json!({ "data": { "search": null }, "errors": [{ "type": "INVALID",
                "path": ["search"],
                "message": "The search query is not valid (fake_gh search-refused)" }] })
        } else if let Some(unknown) = fields
            .get("q")
            .filter(|_| modes.contains(&"reject"))
            .and_then(|q| unknown_qualifier(q))
        {
            eprintln!("fake_gh: q={}", fields.get("q").map_or("", String::as_str));
            serde_json::json!({ "data": { "search": null }, "errors": [
                { "type": "INVALID", "path": ["search"],
                  "message": format!("Invalid search qualifier: {unknown} (fake_gh reject)") },
                { "type": "INVALID", "path": ["search"],
                  "message": "Check the qualifiers of github.com's issue search, then run it again." },
            ] })
        } else {
            search(fields.get("q").map(String::as_str).unwrap_or_default())
        }
    } else if query.contains("issues(states: OPEN") {
        listing(&repo)
    } else if query.contains("milestones(") {
        milestones()
    } else if query.contains("assignableUsers(") {
        people(fields.get("q").map(String::as_str).unwrap_or_default())
    } else if query.contains("labels(") {
        if modes.contains(&"labels-fail") {
            eprintln!("HTTP 502: Bad Gateway (https://api.github.com/graphql)");
            std::process::exit(1);
        }
        labels(&modes, fields.get("cursor").map(String::as_str))
    } else {
        serde_json::json!({ "errors": [{ "type": "NOT_SUPPORTED", "message": "fake_gh does not answer this query" }] })
    };
    println!("{answer}");
}
