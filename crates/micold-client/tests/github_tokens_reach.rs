//! One GitHub token for many repositories (#1529, US2, FR-007, FR-009, SC-002): the rendered
//! `[[github.tokens]]` block carries a token's name, token file and repositories, and Settings
//! lists the tokens in file order and says, per repository, which token it uses and which are
//! shadowed. Every token is fake.

use std::path::PathBuf;

use micold_client::app::State;
use micold_client::daemon_config_render::{prepare, DaemonSettings, GithubTokenSetting};
use micold_client::features::settings::github_tokens::{use_text, Checked};
use micold_client::features::settings::{update, Msg, TokenInput};
use micold_core::daemon_config::{parse, Kind, Listen, RepoPattern, SocketSpec};
use micold_core::scoped_credential::pat_store::{PatStore, StoredPat};
use micold_core::scoped_credential::SecretToken;
use micold_core::typeahead::Direction;

const FAKE: &str = "github_pat_FAKE_0123456789";
const T0: u64 = 1_800_000_000;

fn pattern(text: &str) -> RepoPattern {
    RepoPattern::parse(text).expect("a pattern")
}

fn rendered(tokens: Vec<GithubTokenSetting>) -> String {
    let mut settings = DaemonSettings::new("t", Listen::Socket(SocketSpec::Auto));
    settings.github_tokens = tokens;
    prepare(&settings, Kind::Host).expect("the file is valid")
}

#[test]
fn the_rendered_block_carries_name_token_file_and_repos_and_parses_back() {
    let text = rendered(vec![
        GithubTokenSetting {
            name: "org".into(),
            token_file: PathBuf::from("/secrets/github-pat/org"),
            repos: vec![pattern("acme/*"), pattern("acme/app")],
        },
        GithubTokenSetting {
            name: "any".into(),
            token_file: PathBuf::from("/secrets/github-pat/any"),
            repos: vec![],
        },
    ]);
    assert!(
        text.contains(
            "[[github.tokens]]\nname = \"org\"\ntoken_file = \"/secrets/github-pat/org\"\nrepos = [\"acme/*\", \"acme/app\"]\n"
        ),
        "{text}"
    );
    let config = parse(&text, "daemon.toml").expect("parses");
    let names: Vec<_> = config
        .github
        .tokens
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["org", "any"], "file order is kept");
    assert_eq!(config.github.tokens[1].repos, None, "no filter, no key");
}

#[test]
fn the_tokens_of_a_store_render_in_the_order_settings_shows_them() {
    let dir = tempfile::tempdir().unwrap();
    let store = PatStore::new(dir.path());
    let pat = |repos: &[&str]| StoredPat {
        token: SecretToken::new(FAKE),
        expires_at: T0,
        repos: repos.iter().map(|r| pattern(r)).collect(),
    };
    store.put("b", &pat(&["acme/*"])).unwrap();
    store.put("a", &pat(&[])).unwrap();
    let text = rendered(GithubTokenSetting::from_store(&store));
    let (b, a) = (
        text.find("name = \"b\"").unwrap(),
        text.find("name = \"a\"").unwrap(),
    );
    assert!(b < a, "{text}");
    assert!(text.contains(&format!(
        "token_file = \"{}\"",
        dir.path().join("github-pat/b").display()
    )));
}

fn load(state: &mut State, tokens: &[(&str, &[&str])]) {
    update(
        state,
        Msg::GithubPatsLoaded(
            tokens
                .iter()
                .map(|(n, r)| (n.to_string(), r.iter().map(|p| pattern(p)).collect(), T0))
                .collect(),
        ),
    );
}

fn lines(state: &State) -> Vec<String> {
    state
        .settings
        .github_tokens
        .uses()
        .iter()
        .map(use_text)
        .collect()
}

#[test]
fn two_tokens_for_the_same_organisation_the_first_wins_and_reordering_changes_the_winner() {
    let mut state = State::default();
    load(
        &mut state,
        &[("first", &["acme/*"]), ("second", &["acme/*"])],
    );
    let names: Vec<_> = state
        .settings
        .github_tokens
        .rows
        .iter()
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(names, ["first", "second"], "file order");
    assert_eq!(lines(&state), ["acme/*: uses first; shadowed: second"]);
    update(&mut state, Msg::GithubPatMoved(1, Direction::Prev));
    assert_eq!(lines(&state), ["acme/*: uses second; shadowed: first"]);
    assert!(
        state.settings.github_tokens.take_refresh(),
        "the wiring checks again"
    );
}

#[test]
fn an_exact_filter_beats_the_organisation_and_each_token_shows_its_reach() {
    let mut state = State::default();
    load(
        &mut state,
        &[("org", &["acme/*"]), ("app", &["acme/app"]), ("all", &[])],
    );
    assert_eq!(
        lines(&state),
        [
            "acme/*: uses org; shadowed: all",
            "acme/app: uses app; shadowed: org, all"
        ]
    );
    let rows = &state.settings.github_tokens.rows;
    assert_eq!(rows[0].reach_text(), "Reaches acme/*");
    assert_eq!(
        rows[2].reach_text(),
        "Reaches every repository the token can access"
    );
}

#[test]
fn one_pasted_token_reaching_many_repositories_is_one_row_with_its_reach() {
    let mut state = State::default();
    update(&mut state, Msg::GithubPatNameChanged("work".into()));
    update(
        &mut state,
        Msg::GithubPatTokenChanged(TokenInput::new(FAKE.into())),
    );
    update(&mut state, Msg::GithubPatAddRequested);
    let request = state
        .settings
        .github_tokens
        .take_request()
        .expect("one request");
    assert!(request.repos.is_empty());
    let reach: Vec<String> = (1..=5).map(|n| format!("acme/r{n}")).collect();
    update(
        &mut state,
        Msg::GithubPatChecked {
            name: request.name,
            repos: request.repos,
            result: Ok(Checked {
                expires_at: T0 + 86_400 * 30,
                reach: reach.clone(),
            }),
        },
    );
    let rows = &state.settings.github_tokens.rows;
    assert_eq!(rows.len(), 1, "SC-002: 1 token, not 5");
    assert!(
        rows[0].reach_text().contains("acme/r5"),
        "{}",
        rows[0].reach_text()
    );
}
