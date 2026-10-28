//! Tests of the daemon configuration schema: the four contract examples, defaults per kind, rules
//! V1–V19 (one test each, asserting the error names `file:line:col` and the key) and SC-005.

use std::path::PathBuf;

use super::*;

const ORIGIN: &str = "daemon.toml";

const HOST: &str = include_str!("../../tests/fixtures/daemon_config/host.toml");
const SSH: &str = include_str!("../../tests/fixtures/daemon_config/ssh.toml");
const CONTAINER: &str = include_str!("../../tests/fixtures/daemon_config/container.toml");
const KUBERNETES: &str = include_str!("../../tests/fixtures/daemon_config/kubernetes.toml");

const HOST_HEAD: &str =
    "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\nsocket = \"auto\"\n";
const CONTAINER_HEAD: &str = "schema = 1\n[daemon]\nname = \"t\"\nkind = \"container\"\n\
    [listen]\ntcp = \"127.0.0.1:7000\"\n[auth]\nclient_token_file = \"/run/t\"\n";

fn host(body: &str) -> String {
    format!("{HOST_HEAD}{body}")
}

fn container(body: &str) -> String {
    format!("{CONTAINER_HEAD}{body}")
}

fn ok(text: &str) -> DaemonConfig {
    parse(text, ORIGIN).unwrap_or_else(|errors| panic!("expected a valid file: {errors:#?}"))
}

fn errors(text: &str) -> Vec<ConfigError> {
    match parse(text, ORIGIN) {
        Err(errors) => errors,
        Ok(_) => panic!("expected errors for:\n{text}"),
    }
}

/// The one error of `rule`, whose message names `needle`.
fn expect(text: &str, rule: &str, needle: &str) -> ConfigError {
    let all = errors(text);
    all.iter()
        .find(|e| e.rule == rule && (e.message.contains(needle) || e.key.contains(needle)))
        .cloned()
        .unwrap_or_else(|| panic!("no {rule} error naming {needle:?} in {all:#?}"))
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|l| l.contains(needle)).unwrap() + 1
}

// --- T006: the four contract examples and defaults ---

#[test]
fn the_four_contract_examples_parse() {
    for (kind, text) in [
        (Kind::Host, HOST),
        (Kind::Ssh, SSH),
        (Kind::Container, CONTAINER),
        (Kind::Kubernetes, KUBERNETES),
    ] {
        assert_eq!(ok(text).daemon.kind, kind);
    }
}

#[test]
fn the_host_example_uses_the_machine_sign_in_and_keeps_its_fields() {
    let config = ok(HOST);
    assert_eq!(config.schema, 1);
    assert_eq!(config.github.tokens[0].name, "acme-org");
    assert_eq!(
        config.github.tokens[0].repos,
        Some(vec![RepoPattern::Owner("acme".into())])
    );
    assert_eq!(config.ai.default, Some(CliId::ClaudeCode));
    assert_eq!(config.ai.cli[0].use_machine_sign_in, Some(true));
    assert_eq!(config.git.isolate_host_helpers, Some(false));
}

#[test]
fn defaults_for_a_host_daemon() {
    let view = resolved_view(&ok(&host("")));
    assert_eq!(view.idle_stop, IdleStopSetting::Default);
    assert_eq!(view.log, "info");
    assert!(!view.git.isolate_host_helpers);
    assert!(view.git.ssh_to_https);
    assert_eq!(view.ssh_dir, None);
    assert_eq!(view.listen, Some(Listen::Socket(SocketSpec::Auto)));
    assert!(view.data_dir.is_some());
}

#[test]
fn defaults_for_an_ssh_daemon() {
    let text = "schema = 1\n[daemon]\nname = \"b\"\nkind = \"ssh\"\nhome = \"/home/jaro\"\n";
    let view = resolved_view(&ok(text));
    assert_eq!(
        view.data_dir,
        Some(PathBuf::from("/home/jaro/.local/share/micold-ai-ide"))
    );
    assert_eq!(view.listen, Some(Listen::Socket(SocketSpec::Auto)));
}

#[test]
fn defaults_for_a_container_daemon() {
    let view = resolved_view(&ok(&container("")));
    assert_eq!(view.data_dir, Some(PathBuf::from("/var/lib/micold-ai-ide")));
    assert_eq!(view.idle_stop, IdleStopSetting::Default);
    assert!(view.git.isolate_host_helpers);
    assert_eq!(view.ssh_dir, Some(PathBuf::from("/run/micold/ssh")));
}

#[test]
fn defaults_for_a_kubernetes_daemon() {
    let text = CONTAINER_HEAD.replace("container", "kubernetes");
    let view = resolved_view(&ok(&text));
    assert_eq!(view.idle_stop, IdleStopSetting::Off);
    assert!(view.git.isolate_host_helpers);
    assert_eq!(view.clone_root, Some(PathBuf::from("/data/repos")));
}

#[test]
fn defaults_for_tokens_clis_and_startup() {
    let body = "[[github.tokens]]\nname = \"a\"\ntoken_file = \"/s/a\"\n\
        [[ai.cli]]\nid = \"codex\"\n[startup]\nscript_file = \"/s.sh\"\n";
    let view = resolved_view(&ok(&host(body)));
    assert_eq!(view.tokens[0].host, "github.com");
    assert_eq!(view.tokens[0].repos, None);
    assert!(view.ai_cli[0].enabled);
    let startup = view.startup.unwrap();
    assert_eq!(startup.timeout_secs, 300);
    assert_eq!(startup.on_failure, OnFailure::Refuse);
    assert_eq!(startup.run, RunPolicy::EveryStart);
    assert!(startup.export_env);
    assert_eq!(startup.shell, PathBuf::from("/bin/sh"));
}

#[test]
fn a_token_host_can_be_a_base_url() {
    let body =
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"/s/a\"\nhost = \"http://127.0.0.1:9\"\n";
    let view = resolved_view(&ok(&host(body)));
    assert_eq!(view.tokens[0].host, "http://127.0.0.1:9");
}

#[test]
fn the_resolved_view_shows_a_missing_secret_as_missing() {
    let body = "[[github.tokens]]\nname = \"a\"\ntoken_file = \"/nonexistent/a\"\n";
    let text = resolved_view(&ok(&host(body))).to_string();
    assert!(text.contains("/nonexistent/a (missing)"), "{text}");
}

#[cfg(unix)]
#[test]
fn the_resolved_view_shows_a_readable_secret_with_its_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tok");
    std::fs::write(&file, "s3cr3t-value\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    let body = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        file.display()
    );
    let text = resolved_view(&ok(&host(&body))).to_string();
    assert!(
        text.contains(&format!("{} (readable, 0600)", file.display())),
        "{text}"
    );
    assert!(!text.contains("s3cr3t-value"));
}

// --- T007: V1-V19 ---

#[test]
fn v1_an_unknown_key_names_file_line_column_key_and_table() {
    let text = HOST_HEAD.replace("[daemon]\nname", "[daemon]\nlistne = 1\nname");
    let error = expect(&text, "V1", "listne");
    assert_eq!(error.line, line_of(&text, "listne"));
    assert_eq!(error.column, 1);
    assert!(
        error.message.contains("unknown key \"listne\" in [daemon]"),
        "{error}"
    );
    assert!(error.to_string().starts_with("daemon.toml:"), "{error}");
}

#[test]
fn v1_an_unknown_key_in_an_array_table_is_refused() {
    let text = host("[[projects]]\npath = \"/p\"\nbogus = true\n");
    let error = expect(&text, "V1", "bogus");
    assert_eq!(error.line, line_of(&text, "bogus"));
    assert!(error.message.contains("[[projects]]"), "{error}");
}

#[test]
fn v1_an_unknown_top_level_key_is_refused() {
    let text = host("extra = 1\n");
    expect(&text, "V1", "extra");
}

#[test]
fn v1_the_retired_git_base_key_is_refused() {
    expect(&host("[git]\nbase = \"https://x\"\n"), "V1", "base");
}

#[test]
fn v2_a_missing_schema_is_refused() {
    let text = HOST_HEAD.replace("schema = 1\n", "");
    expect(&text, "V2", "schema");
}

#[test]
fn v2_a_newer_schema_names_both_versions() {
    let text = HOST_HEAD.replace("schema = 1", "schema = 2");
    let error = expect(&text, "V2", "schema");
    assert!(
        error.message.contains('2') && error.message.contains('1'),
        "{error}"
    );
    assert_eq!(error.line, 1);
}

#[test]
fn v2_a_newer_schema_is_refused_even_with_keys_this_build_does_not_know() {
    let text = format!(
        "{}future_key = 1\n",
        HOST_HEAD.replace("schema = 1", "schema = 2")
    );
    let all = errors(&text);
    assert!(all.iter().any(|e| e.rule == "V2"), "{all:#?}");
    assert!(all.iter().all(|e| e.rule != "V1"), "{all:#?}");
}

#[test]
fn v3_a_listen_table_with_no_key_is_refused() {
    let text = "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\n";
    expect(text, "V3", "listen");
}

#[test]
fn v3_a_listen_table_with_two_keys_is_refused() {
    let text = format!("{CONTAINER_HEAD}").replace("[auth]", "socket = \"auto\"\n[auth]");
    let error = expect(&text, "V3", "listen");
    assert_eq!(error.line, line_of(&text, "[listen]"));
}

#[test]
fn v3_tcp_without_a_client_token_file_is_refused() {
    let text = "schema = 1\n[daemon]\nname = \"t\"\nkind = \"container\"\n[listen]\ntcp = \"127.0.0.1:7000\"\n";
    let error = expect(text, "V3", "client_token_file");
    assert_eq!(error.line, line_of(text, "tcp"));
}

#[test]
fn v3_a_container_with_no_listen_table_is_refused() {
    let text = "schema = 1\n[daemon]\nname = \"t\"\nkind = \"container\"\n";
    expect(text, "V3", "listen");
}

#[test]
fn v4_a_relative_path_is_refused_with_its_key() {
    let text = host("[[github.tokens]]\nname = \"a\"\ntoken_file = \"secrets/a\"\n");
    let error = expect(&text, "V4", "token_file");
    assert_eq!(error.line, line_of(&text, "secrets/a"));
}

#[test]
fn v4_a_tilde_is_not_expanded() {
    expect(
        &host("[storage]\nclone_root = \"~/src\"\n"),
        "V4",
        "clone_root",
    );
}

#[test]
fn v4_windows_absolute_paths_are_accepted() {
    ok(&host("[storage]\nclone_root = 'C:\\Users\\me\\src'\n"));
}

#[test]
fn v5_duplicate_token_names_name_both_entries() {
    let entry = "[[github.tokens]]\nname = \"a\"\ntoken_file = \"/s/a\"\n";
    let text = host(&format!("{entry}{entry}"));
    let error = expect(&text, "V5", "a");
    assert!(error.message.contains("line"), "{error}");
}

#[test]
fn v5_names_the_line_of_the_first_entry() {
    let entry =
        |name: &str| format!("[[github.tokens]]\nname = \"{name}\"\ntoken_file = \"/s/{name}\"\n");
    let text = host(&format!(
        "{}{}{}{}",
        entry("a"),
        entry("b"),
        entry("c"),
        entry("b")
    ));
    let error = expect(&text, "V5", "b");
    let first_b = text
        .lines()
        .position(|l| l.contains("name = \"b\""))
        .unwrap()
        + 1;
    assert!(
        error.message.contains(&format!("line {}", first_b - 1)),
        "{error}"
    );
}

#[test]
fn v5_duplicate_cli_ids_and_project_paths_are_refused() {
    expect(
        &host("[[ai.cli]]\nid = \"pi\"\n[[ai.cli]]\nid = \"pi\"\n"),
        "V5",
        "pi",
    );
    expect(
        &host("[[projects]]\npath = \"/p\"\n[[projects]]\npath = \"/p\"\n"),
        "V5",
        "/p",
    );
}

#[test]
fn v6_two_tokens_with_the_same_pattern_are_accepted() {
    let body = "[[github.tokens]]\nname = \"a\"\ntoken_file = \"/s/a\"\nrepos = [\"acme/*\"]\n\
        [[github.tokens]]\nname = \"b\"\ntoken_file = \"/s/b\"\nrepos = [\"acme/*\"]\n";
    assert_eq!(ok(&host(body)).github.tokens.len(), 2);
}

#[test]
fn v7_model_is_refused_for_a_cli_without_a_model_argument() {
    let text = host("[[ai.cli]]\nid = \"codex\"\nmodel = \"x\"\n");
    let error = expect(&text, "V7", "model");
    assert_eq!(error.line, line_of(&text, "model"));
    assert!(error.message.contains("codex"), "{error}");
}

#[test]
fn v7_an_api_base_url_is_refused_for_a_cli_without_one() {
    expect(
        &host("[[ai.cli]]\nid = \"pi\"\napi_base_url = \"https://x\"\n"),
        "V7",
        "api_base_url",
    );
    ok(&host(
        "[[ai.cli]]\nid = \"codex\"\napi_base_url = \"https://x\"\n",
    ));
}

#[test]
fn v7_a_sign_in_file_is_refused_for_copilot_and_pi() {
    for id in ["copilot", "pi"] {
        let body = format!("[[ai.cli]]\nid = \"{id}\"\nsign_in_file = \"/s/in\"\n");
        expect(&host(&body), "V7", "sign_in_file");
    }
}

#[test]
fn v7_an_api_key_file_is_refused_for_a_cli_with_no_key_variable() {
    for id in ["copilot", "pi"] {
        let body = format!("[[ai.cli]]\nid = \"{id}\"\napi_key_file = \"/s/k\"\n");
        expect(&host(&body), "V7", "api_key_file");
    }
    ok(&host(
        "[[ai.cli]]\nid = \"codex\"\napi_key_file = \"/s/k\"\n",
    ));
}

#[test]
fn v7_a_version_must_look_like_a_version() {
    expect(
        &container("[[ai.cli]]\nid = \"pi\"\nversion = \"1.0; rm -rf\"\n"),
        "V7",
        "version",
    );
    ok(&container(
        "[[ai.cli]]\nid = \"pi\"\nversion = \"1.2.3-beta.1\"\n",
    ));
}

#[test]
fn v7_a_version_on_a_host_daemon_is_a_warning() {
    let text = host("[[ai.cli]]\nid = \"pi\"\nversion = \"1.2.3\"\n");
    let (config, findings) = diagnose(&text, ORIGIN);
    assert!(config.is_some());
    assert!(findings
        .iter()
        .any(|f| f.severity == Severity::Warning && f.key.contains("version")));
}

#[test]
fn v8_daemon_owned_arguments_are_refused() {
    for arg in [
        "--resume",
        "--session-id=1",
        "--settings",
        "--mcp-config",
        "--input-format",
    ] {
        let body = format!("[[ai.cli]]\nid = \"claude_code\"\nargs = [\"{arg}\"]\n");
        let error = expect(&host(&body), "V8", arg.split('=').next().unwrap());
        assert!(error.message.contains(arg.split('=').next().unwrap()));
    }
    ok(&host(
        "[[ai.cli]]\nid = \"claude_code\"\nargs = [\"--permission-mode\", \"acceptEdits\"]\n",
    ));
}

#[test]
fn v9_more_than_one_credential_is_refused() {
    let body =
        "[[ai.cli]]\nid = \"claude_code\"\nsign_in_file = \"/s/a\"\napi_key_file = \"/s/b\"\n";
    expect(&host(body), "V9", "claude_code");
}

#[test]
fn v9_a_token_file_is_for_claude_code_and_copilot_only() {
    expect(
        &host("[[ai.cli]]\nid = \"codex\"\ntoken_file = \"/s/t\"\n"),
        "V9",
        "codex",
    );
    ok(&host(
        "[[ai.cli]]\nid = \"copilot\"\ntoken_file = \"/s/t\"\n",
    ));
}

#[test]
fn v10_settings_file_and_place_are_refused_on_host_and_ssh() {
    expect(
        &host("[[ai.cli]]\nid = \"claude_code\"\nsettings_file = \"/c/s.json\"\n"),
        "V10",
        "settings_file",
    );
    let place = "[[ai.cli]]\nid = \"codex\"\nplace = [{ file = \"/c/a.md\", as = \"A.md\" }]\n";
    expect(&host(place), "V10", "place");
    ok(&container(
        "[[ai.cli]]\nid = \"claude_code\"\nsettings_file = \"/c/s.json\"\n",
    ));
}

#[test]
fn v10_a_placed_file_may_not_escape_the_config_dir() {
    for escaping in ["../x", "/abs", "a/../../x"] {
        let body = format!(
            "[[ai.cli]]\nid = \"codex\"\nplace = [{{ file = \"/c/a.md\", as = \"{escaping}\" }}]\n"
        );
        expect(&container(&body), "V10", "place");
    }
}

#[test]
fn v12_a_remote_needs_a_clone_root_and_a_path_under_it() {
    expect(
        &host("[[projects]]\npath = \"/p\"\nremote = \"https://x/y\"\n"),
        "V12",
        "/p",
    );
    let outside = "[storage]\nclone_root = \"/src\"\n[[projects]]\npath = \"/other/p\"\nremote = \"https://x/y\"\n";
    expect(&host(outside), "V12", "/other/p");
    ok(&host("[storage]\nclone_root = \"/src\"\n[[projects]]\npath = \"/src/p\"\nremote = \"https://x/y\"\n"));
}

#[test]
fn v13_the_startup_timeout_is_one_to_3600() {
    for bad in ["0", "3601", "-5"] {
        let text = host(&format!(
            "[startup]\nscript_file = \"/s.sh\"\ntimeout_secs = {bad}\n"
        ));
        let error = expect(&text, "V13", "timeout_secs");
        assert_eq!(error.line, line_of(&text, "timeout_secs"));
    }
    ok(&host(
        "[startup]\nscript_file = \"/s.sh\"\ntimeout_secs = 3600\n",
    ));
}

#[test]
fn v16_reserved_variable_names_are_refused_in_all_three_tables() {
    expect(&host("[session.env]\nPATH = \"/x\"\n"), "V16", "PATH");
    expect(
        &host("[session.env]\nGIT_CONFIG_COUNT = \"1\"\n"),
        "V16",
        "GIT_CONFIG_COUNT",
    );
    expect(
        &host("[session.secret_env]\nMICOLD_X = \"/s/x\"\n"),
        "V16",
        "MICOLD_X",
    );
    expect(
        &host("[[ai.cli]]\nid = \"codex\"\nenv = { CODEX_HOME = \"/x\" }\n"),
        "V16",
        "CODEX_HOME",
    );
    ok(&host("[session.env]\nRUST_LOG = \"info\"\n"));
}

#[test]
fn v16_a_cli_key_variable_is_reserved_when_the_cli_has_a_key_file() {
    let body = "[[ai.cli]]\nid = \"claude_code\"\napi_key_file = \"/s/k\"\nenv = { ANTHROPIC_API_KEY = \"x\" }\n";
    expect(&host(body), "V16", "ANTHROPIC_API_KEY");
    let session = "[[ai.cli]]\nid = \"copilot\"\ntoken_file = \"/s/k\"\n[session.env]\nCOPILOT_GITHUB_TOKEN = \"x\"\n";
    expect(&host(session), "V16", "COPILOT_GITHUB_TOKEN");
    ok(&host("[session.env]\nANTHROPIC_API_KEY = \"x\"\n"));
}

#[test]
fn v18_a_session_file_target_must_stay_inside_the_home() {
    for bad in ["/abs/.npmrc", "../x", "a/../../x"] {
        let body = format!("[[session.files]]\nfile = \"/s/n\"\nas = \"{bad}\"\n");
        expect(&host(&body), "V18", "as");
    }
    ok(&host(
        "[[session.files]]\nfile = \"/s/n\"\nas = \".npmrc\"\nmode = \"0600\"\n",
    ));
}

#[test]
fn v18_a_session_file_may_not_target_a_cli_config_dir() {
    let body = "[[session.files]]\nfile = \"/s/n\"\nas = \".claude/settings.json\"\n";
    expect(&host(body), "V18", "as");
}

#[test]
fn v18_two_session_files_may_not_collide() {
    let entry = "[[session.files]]\nfile = \"/s/n\"\nas = \".npmrc\"\n";
    expect(&host(&format!("{entry}{entry}")), "V18", ".npmrc");
}

#[test]
fn v18_equivalent_spellings_collide_and_claude_json_is_protected() {
    let a = "[[session.files]]\nfile = \"/s/n\"\nas = \"./.npmrc\"\n";
    let b = "[[session.files]]\nfile = \"/s/n\"\nas = \".NPMRC\"\n";
    expect(&host(&format!("{a}{b}")), "V18", "collides");
    expect(
        &host("[[session.files]]\nfile = \"/s/n\"\nas = \".claude.json\"\n"),
        "V18",
        "as",
    );
}

#[test]
fn v8_the_finding_points_at_the_offending_element() {
    let text =
        host("[[ai.cli]]\nid = \"claude_code\"\nargs = [\n  \"--ok\",\n  \"--settings\",\n]\n");
    let error = expect(&text, "V8", "--settings");
    assert_eq!(error.line, line_of(&text, "--settings"));
}

#[test]
fn idle_stop_accepts_default_off_and_seconds_and_names_a_bad_value() {
    let kind = |value: &str| {
        format!("{HOST_HEAD}").replace(
            "name = \"t\"",
            &format!("name = \"t\"\nidle_stop = {value}"),
        )
    };
    assert_eq!(
        ok(&kind("\"off\"")).daemon.idle_stop,
        Some(IdleStopSetting::Off)
    );
    assert_eq!(
        ok(&kind("90")).daemon.idle_stop,
        Some(IdleStopSetting::Secs(90))
    );
    for bad in ["-5", "0", "\"soon\"", "true"] {
        let error = errors(&kind(bad)).remove(0);
        assert!(
            error.message.contains("idle_stop") || error.message.contains("default"),
            "{bad}: {error}"
        );
    }
}

#[test]
fn v18_a_session_file_mode_is_octal() {
    let body = "[[session.files]]\nfile = \"/s/n\"\nas = \".npmrc\"\nmode = \"rw\"\n";
    expect(&host(body), "V18", "mode");
}

#[test]
fn v18_an_ssh_key_needs_known_hosts() {
    let text = container("[git]\nssh_key_file = \"/s/k\"\n");
    let error = expect(&text, "V18", "known_hosts_file");
    assert_eq!(error.line, line_of(&text, "ssh_key_file"));
    ok(&container(
        "[git]\nssh_key_file = \"/s/k\"\nknown_hosts_file = \"/c/kh\"\n",
    ));
}

#[test]
fn v19_machine_sign_in_is_for_host_and_ssh_only() {
    let body = "[[ai.cli]]\nid = \"claude_code\"\nuse_machine_sign_in = true\n";
    expect(&container(body), "V19", "claude_code");
    ok(&host(body));
}

#[test]
fn v19_machine_sign_in_excludes_every_other_credential() {
    for other in ["sign_in_file", "token_file", "api_key_file"] {
        let body = format!(
            "[[ai.cli]]\nid = \"claude_code\"\nuse_machine_sign_in = true\n{other} = \"/s/x\"\n"
        );
        expect(&host(&body), "V19", "claude_code");
    }
}

#[test]
fn several_semantic_errors_are_all_reported() {
    let body = "[session.env]\nPATH = \"/x\"\n[[projects]]\npath = \"rel\"\n";
    let all = errors(&host(body));
    assert!(all.len() >= 2, "{all:#?}");
}

#[test]
fn a_syntax_error_is_reported_with_its_position() {
    let all = errors("schema = 1\n[daemon\n");
    assert_eq!(all[0].line, 2);
}

// --- check_files: V14, V15, V17 ---

#[cfg(unix)]
fn secret_file(dir: &std::path::Path, name: &str, mode: u32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let file = dir.join(name);
    std::fs::write(&file, "s3cr3t-value\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
    file
}

#[cfg(unix)]
#[test]
fn v14_a_wide_secret_is_a_warning_on_host_and_an_error_in_a_container() {
    let dir = tempfile::tempdir().unwrap();
    let file = secret_file(dir.path(), "tok", 0o644);
    let body = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        file.display()
    );
    for (text, severity) in [
        (host(&body), Severity::Warning),
        (container(&body), Severity::Error),
    ] {
        let found = check_files(&text, ORIGIN, &ok(&text));
        let finding = found
            .iter()
            .find(|f| f.rule == "V14" && f.key.contains("token_file"))
            .unwrap();
        assert_eq!(finding.severity, severity);
        assert_eq!(finding.line, line_of(&text, &file.display().to_string()));
    }
}

#[cfg(unix)]
#[test]
fn v14_an_owner_only_secret_passes() {
    let dir = tempfile::tempdir().unwrap();
    let file = secret_file(dir.path(), "tok", 0o600);
    let body = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        file.display()
    );
    let text = container(&body);
    assert!(check_files(&text, ORIGIN, &ok(&text))
        .iter()
        .all(|f| f.rule != "V14"));
}

#[test]
fn v15_a_missing_required_file_is_an_error_naming_the_path() {
    let body = "[startup]\nscript_file = \"/nonexistent/start.sh\"\n";
    let text = host(body);
    let found = check_files(&text, ORIGIN, &ok(&text));
    let finding = found.iter().find(|f| f.rule == "V15").unwrap();
    assert!(finding.message.contains("/nonexistent/start.sh"));
    assert_eq!(finding.line, line_of(&text, "script_file"));
}

#[test]
fn v15_a_missing_secret_is_not_a_start_refusal() {
    let text = host("[[github.tokens]]\nname = \"a\"\ntoken_file = \"/nonexistent/a\"\n");
    assert!(check_files(&text, ORIGIN, &ok(&text))
        .iter()
        .all(|f| f.rule != "V15"));
}

#[test]
fn v17_a_placed_mcp_config_that_defines_micold_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("mcp.json");
    std::fs::write(&file, r#"{"mcpServers":{"micold":{"command":"x"}}}"#).unwrap();
    let body = format!(
        "[[ai.cli]]\nid = \"claude_code\"\nplace = [{{ file = \"{}\", as = \".mcp.json\" }}]\n",
        file.display()
    );
    let text = container(&body);
    let found = check_files(&text, ORIGIN, &ok(&text));
    let finding = found.iter().find(|f| f.rule == "V17").unwrap();
    assert!(finding.message.contains("micold"));
}

#[test]
fn v17_a_placed_file_without_that_server_passes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("mcp.json");
    std::fs::write(&file, r#"{"mcpServers":{"other":{}}}"#).unwrap();
    let body = format!(
        "[[ai.cli]]\nid = \"claude_code\"\nplace = [{{ file = \"{}\", as = \".mcp.json\" }}]\n",
        file.display()
    );
    let text = container(&body);
    assert!(check_files(&text, ORIGIN, &ok(&text))
        .iter()
        .all(|f| f.rule != "V17"));
}

// --- T008: SC-005 ---

#[test]
fn debug_and_display_never_contain_a_secrets_content() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tok");
    std::fs::write(&file, "s3cr3t-value\n").unwrap();
    let body = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{0}\"\n[session.secret_env]\nNPM = \"{0}\"\n",
        file.display()
    );
    let config = ok(&host(&body));
    let secret = &config.github.tokens[0].token_file;
    let value = secret.read().unwrap();
    assert_eq!(value.expose(), "s3cr3t-value");
    let shown = format!(
        "{config:?}{config:#?}{secret:?}{secret}{value:?}{:?}{}",
        resolved_view(&config),
        resolved_view(&config)
    );
    assert!(!shown.contains("s3cr3t-value"), "{shown}");
    assert!(shown.contains(&file.display().to_string()));
}

#[test]
fn a_secret_path_prints_its_path() {
    let secret = SecretPath::new("/run/secrets/x");
    assert!(format!("{secret:?}").contains("/run/secrets/x"));
    assert_eq!(secret.to_string(), "/run/secrets/x");
}
