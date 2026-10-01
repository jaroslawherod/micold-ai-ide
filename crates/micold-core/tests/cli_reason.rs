//! Feature 037: the state of a session's environment, and the sentences said about it.
//!
//! `SpawnEnv::classify` follows research R2's table. `explain` and `name_list` follow
//! contracts/reason-wording.md W1 and W2, and the rules W2a to W2f are asserted over every state
//! and both places.

use std::path::Path;

use micold_core::cli_reason::{
    explain, name_list, AttemptDir, Explanation, Place, SpawnEnv, LABEL_ENABLED, LABEL_SCRIPT_PATH,
    LABEL_TIMEOUT,
};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::AiCli;

const SCRIPT: &str = "/home/u/.bashrc";
const IMAGE: &str = "img:tag";
const PROJECT: &str = "/work/project";

const EVERY_STATE: [SpawnEnv; 6] = [
    SpawnEnv::IncludeOff,
    SpawnEnv::NoScriptPath,
    SpawnEnv::ScriptNotFound,
    SpawnEnv::ScriptFailed,
    SpawnEnv::ScriptTimedOut,
    SpawnEnv::Applied,
];
const BOTH_PLACES: [Place<'static>; 2] = [Place::ThisComputer, Place::Image(IMAGE)];

fn non_zero_exit() -> EnvIncludeOutcome {
    EnvIncludeOutcome::NonZeroExit {
        code: 3,
        diagnostic: "boom".to_string(),
    }
}

fn timed_out() -> EnvIncludeOutcome {
    EnvIncludeOutcome::TimedOut {
        diagnostic: "still running".to_string(),
    }
}

fn every_attempt() -> Vec<Option<EnvIncludeOutcome>> {
    vec![
        None,
        Some(EnvIncludeOutcome::Disabled),
        Some(EnvIncludeOutcome::Success),
        Some(EnvIncludeOutcome::MissingScript),
        Some(non_zero_exit()),
        Some(timed_out()),
    ]
}

fn told(missing: &[AiCli], env: SpawnEnv, place: Place<'_>, dir: AttemptDir<'_>) -> Explanation {
    explain(missing, env, place, dir).expect("a missing CLI has a reason")
}

// --- SpawnEnv::classify (research R2) ---

#[test]
fn include_off_is_the_state_whatever_the_path_and_the_attempt() {
    for path in ["", "   ", SCRIPT] {
        for attempt in every_attempt() {
            assert_eq!(
                SpawnEnv::classify(false, path, attempt.as_ref()),
                Some(SpawnEnv::IncludeOff),
                "`enabled` is tested first: with the switch off no script is sourced, so the \
                 path and an earlier attempt say nothing (path {path:?}, attempt {attempt:?})"
            );
        }
    }
}

#[test]
fn a_blank_script_path_is_its_own_state_whatever_the_attempt() {
    for path in ["", "   "] {
        for attempt in every_attempt() {
            assert_eq!(
                SpawnEnv::classify(true, path, attempt.as_ref()),
                Some(SpawnEnv::NoScriptPath),
                "a blank path is tested before the attempt: nothing is sourced, and the action \
                 is to set the path, not to turn the switch on (path {path:?})"
            );
        }
    }
}

#[test]
fn a_missing_script_is_script_not_found() {
    assert_eq!(
        SpawnEnv::classify(true, SCRIPT, Some(&EnvIncludeOutcome::MissingScript)),
        Some(SpawnEnv::ScriptNotFound),
        "the resolver found no file at the script path"
    );
}

#[test]
fn a_non_zero_exit_is_script_failed() {
    assert_eq!(
        SpawnEnv::classify(true, SCRIPT, Some(&non_zero_exit())),
        Some(SpawnEnv::ScriptFailed),
        "the script was sourced and exited with an error"
    );
}

#[test]
fn a_timeout_is_script_timed_out() {
    assert_eq!(
        SpawnEnv::classify(true, SCRIPT, Some(&timed_out())),
        Some(SpawnEnv::ScriptTimedOut),
        "the script ran past the timeout"
    );
}

#[test]
fn a_successful_attempt_is_applied() {
    assert_eq!(
        SpawnEnv::classify(true, SCRIPT, Some(&EnvIncludeOutcome::Success)),
        Some(SpawnEnv::Applied),
        "the script's PATH additions reached the session"
    );
}

#[test]
fn settings_that_call_for_an_attempt_with_none_known_have_no_state() {
    for attempt in [None, Some(EnvIncludeOutcome::Disabled)] {
        assert_eq!(
            SpawnEnv::classify(true, SCRIPT, attempt.as_ref()),
            None,
            "on with a path set, the state is the attempt's outcome, and a guess would name a \
             cause that may not hold (attempt {attempt:?})"
        );
    }
}

#[test]
fn only_applied_counts_as_the_script_applied() {
    for env in EVERY_STATE {
        assert_eq!(
            env.script_applied(),
            env == SpawnEnv::Applied,
            "FR-002 lets a sentence say the CLI is not there only when the script's PATH \
             additions reached the session ({env:?})"
        );
    }
}

// --- name_list (W1) ---

#[test]
fn a_name_list_reads_as_prose() {
    assert_eq!(name_list(&[]), None, "nothing to name");
    assert_eq!(name_list(&[AiCli::Pi]).as_deref(), Some("Pi Coding Agent"));
    assert_eq!(
        name_list(&[AiCli::ClaudeCode, AiCli::Copilot]).as_deref(),
        Some("Claude Code and GitHub Copilot")
    );
    assert_eq!(
        name_list(&[AiCli::ClaudeCode, AiCli::Copilot, AiCli::Pi]).as_deref(),
        Some("Claude Code, GitHub Copilot and Pi Coding Agent"),
        "the last two are joined by \"and\", with no comma before it"
    );
}

// --- explain (W2) ---

#[test]
fn nothing_missing_has_no_explanation() {
    for env in EVERY_STATE {
        for place in BOTH_PLACES {
            assert_eq!(
                explain(&[], env, place, AttemptDir::Home),
                None,
                "with every CLI found there is nothing to say ({env:?}, {place:?})"
            );
        }
    }
}

#[test]
fn include_off_on_this_computer() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::IncludeOff,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session would not find Pi Coding Agent: sessions get only the login PATH, because \
         \"Source a script before each session\" is off."
    );
    assert_eq!(
        said.action,
        "Turn it on if your startup file puts it on the PATH, or install it on the login PATH."
    );
}

#[test]
fn include_off_in_an_image() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::IncludeOff,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session in img:tag would not find Pi Coding Agent: sessions get only the image's \
         PATH, because \"Source a script before each session\" is off."
    );
    assert_eq!(
        said.action,
        "Turn it on if your startup file puts it on the PATH, or use an image that puts it on \
         its PATH."
    );
}

#[test]
fn no_script_path_on_this_computer() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::NoScriptPath,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session would not find Pi Coding Agent: no script is sourced, because \"Script \
         path\" is empty."
    );
    assert_eq!(
        said.action,
        "Set \"Script path\" if a startup file puts it on the PATH, or install it on the login \
         PATH."
    );
}

#[test]
fn no_script_path_in_an_image() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::NoScriptPath,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session in img:tag would not find Pi Coding Agent: no script is sourced, because \
         \"Script path\" is empty."
    );
    assert_eq!(
        said.action,
        "Set \"Script path\" if a startup file puts it on the PATH, or use an image that puts \
         it on its PATH."
    );
}

#[test]
fn script_not_found_names_the_directory_and_the_field() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptNotFound,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session would not find Pi Coding Agent: the startup script was not found for your \
         home directory, so its PATH additions are not applied."
    );
    assert_eq!(said.action, "Correct \"Script path\".");

    let in_image = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptNotFound,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        in_image.reason,
        "A session in img:tag would not find Pi Coding Agent: the startup script was not found \
         for your home directory, so its PATH additions are not applied."
    );
    assert_eq!(in_image.action, said.action);
}

#[test]
fn script_failed_names_the_directory_and_the_field() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptFailed,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session would not find Pi Coding Agent: the startup script exited with an error for \
         your home directory, so its PATH additions are not applied."
    );
    assert_eq!(said.action, "Fix the script named in \"Script path\".");

    let in_image = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptFailed,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        in_image.reason,
        "A session in img:tag would not find Pi Coding Agent: the startup script exited with \
         an error for your home directory, so its PATH additions are not applied."
    );
    assert_eq!(in_image.action, said.action);
}

#[test]
fn script_timed_out_names_the_directory_and_both_fields() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptTimedOut,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "A session would not find Pi Coding Agent: the startup script timed out for your home \
         directory, so its PATH additions are not applied."
    );
    assert_eq!(
        said.action,
        "Fix the script named in \"Script path\", or raise \"Timeout\"."
    );

    let in_image = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptTimedOut,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        in_image.reason,
        "A session in img:tag would not find Pi Coding Agent: the startup script timed out for \
         your home directory, so its PATH additions are not applied."
    );
    assert_eq!(in_image.action, said.action);
}

#[test]
fn applied_on_this_computer_names_both_ways_out() {
    let said = told(
        &[AiCli::Pi],
        SpawnEnv::Applied,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        said.reason,
        "Pi Coding Agent was not found on the PATH sessions get for your home directory: the \
         login PATH plus what the startup script adds."
    );
    assert_eq!(
        said.action,
        "Install it, or make the script add its directory."
    );
}

#[test]
fn applied_in_an_image_keeps_the_sentence_of_027() {
    // W2d: reason and action joined by a space are the sentence `missing_cli_notice` wrote before
    // this feature, byte for byte.
    let one = told(
        &[AiCli::Copilot],
        SpawnEnv::Applied,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        format!("{} {}", one.reason, one.action),
        "GitHub Copilot isn't in img:tag. Sessions run in that image, so it has to provide any \
         AI CLI you want to use."
    );
    let all = told(
        &AiCli::ALL,
        SpawnEnv::Applied,
        Place::Image(IMAGE),
        AttemptDir::Dir(Path::new(PROJECT)),
    );
    assert_eq!(
        format!("{} {}", all.reason, all.action),
        "Claude Code, GitHub Copilot and Pi Coding Agent aren't in img:tag. Sessions run in \
         that image, so it has to provide any AI CLI you want to use."
    );
}

#[test]
fn several_clis_are_them_and_were() {
    let two = [AiCli::Copilot, AiCli::Pi];
    let off = told(
        &two,
        SpawnEnv::IncludeOff,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        off.reason,
        "A session would not find GitHub Copilot and Pi Coding Agent: sessions get only the \
         login PATH, because \"Source a script before each session\" is off."
    );
    assert_eq!(
        off.action,
        "Turn it on if your startup file puts them on the PATH, or install them on the login \
         PATH.",
        "\"Turn it on\" is the switch. \"them\" is the CLIs"
    );
    let in_image = told(
        &two,
        SpawnEnv::NoScriptPath,
        Place::Image(IMAGE),
        AttemptDir::Home,
    );
    assert_eq!(
        in_image.action,
        "Set \"Script path\" if a startup file puts them on the PATH, or use an image that \
         puts them on its PATH."
    );

    let applied = told(
        &two,
        SpawnEnv::Applied,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert_eq!(
        applied.reason,
        "GitHub Copilot and Pi Coding Agent were not found on the PATH sessions get for your \
         home directory: the login PATH plus what the startup script adds."
    );
    assert_eq!(
        applied.action,
        "Install them, or make the script add their directories."
    );

    let three = told(
        &AiCli::ALL,
        SpawnEnv::Applied,
        Place::ThisComputer,
        AttemptDir::Home,
    );
    assert!(
        three
            .reason
            .starts_with("Claude Code, GitHub Copilot and Pi Coding Agent were not found"),
        "three names, plural verb: {}",
        three.reason
    );
}

#[test]
fn a_row_s_own_directory_is_named_as_displayed() {
    let dir = AttemptDir::Dir(Path::new(PROJECT));
    let timed_out = told(
        &[AiCli::Pi],
        SpawnEnv::ScriptTimedOut,
        Place::ThisComputer,
        dir,
    );
    assert_eq!(
        timed_out.reason,
        "A session would not find Pi Coding Agent: the startup script timed out for \
         /work/project, so its PATH additions are not applied."
    );
    let applied = told(&[AiCli::Pi], SpawnEnv::Applied, Place::ThisComputer, dir);
    assert_eq!(
        applied.reason,
        "Pi Coding Agent was not found on the PATH sessions get for /work/project: the login \
         PATH plus what the startup script adds."
    );
}

// --- The rules of W2, over every state and both places ---

/// Every `(state, place, explanation)` for one, two and three missing CLIs, with the attempt made
/// for `dir`.
fn every_explanation(dir: AttemptDir<'_>) -> Vec<(SpawnEnv, Place<'static>, Explanation)> {
    let mut all = Vec::new();
    for env in EVERY_STATE {
        for place in BOTH_PLACES {
            for missing in [&AiCli::ALL[..1], &AiCli::ALL[..2], &AiCli::ALL[..]] {
                all.push((env, place, told(missing, env, place, dir)));
            }
        }
    }
    all
}

#[test]
fn w2a_no_sentence_says_not_installed_unless_the_script_was_applied() {
    for (env, place, said) in every_explanation(AttemptDir::Home) {
        if env.script_applied() {
            continue;
        }
        for sentence in [&said.reason, &said.action] {
            for claim in ["isn't installed", "not installed", "isn't in", "aren't in"] {
                assert!(
                    !sentence.contains(claim),
                    "FR-002: without the script applied nobody knows the CLI is absent, so \
                     \"{claim}\" is a guess ({env:?}, {place:?}): {sentence}"
                );
            }
        }
        let lowered = said.action.to_lowercase();
        assert!(
            !(lowered.starts_with("install") || lowered.starts_with("use an image")),
            "SC-003: installing is never the only action while the script is not applied \
             ({env:?}, {place:?}): {}",
            said.action
        );
        assert!(
            said.action.contains(LABEL_SCRIPT_PATH) || said.action.starts_with("Turn it on"),
            "the action names an environment-include setting ({env:?}, {place:?}): {}",
            said.action
        );
    }
}

#[test]
fn w2b_every_quoted_label_is_one_the_settings_page_shows() {
    let labels = [LABEL_ENABLED, LABEL_SCRIPT_PATH, LABEL_TIMEOUT];
    assert_eq!(
        labels,
        [
            "Source a script before each session",
            "Script path",
            "Timeout"
        ],
        "the labels the Settings page renders"
    );
    for (env, place, said) in every_explanation(AttemptDir::Home) {
        for sentence in [&said.reason, &said.action] {
            // Odd-numbered pieces of a split on `"` are what sits between a pair of quotes.
            let quoted: Vec<&str> = sentence.split('"').skip(1).step_by(2).collect();
            for label in quoted {
                assert!(
                    labels.contains(&label),
                    "FR-003: \"{label}\" is not a label on the page ({env:?}, {place:?}): \
                     {sentence}"
                );
            }
        }
    }
}

#[test]
fn w2c_the_directory_is_named_exactly_where_an_attempt_is_reported() {
    for (dir, shown) in [
        (AttemptDir::Home, "your home directory"),
        (AttemptDir::Dir(Path::new(PROJECT)), PROJECT),
    ] {
        for (env, place, said) in every_explanation(dir) {
            let reports_an_attempt = !matches!(
                (env, place),
                (SpawnEnv::IncludeOff | SpawnEnv::NoScriptPath, _)
                    | (SpawnEnv::Applied, Place::Image(_))
            );
            let both = format!("{} {}", said.reason, said.action);
            assert_eq!(
                both.contains(shown),
                reports_an_attempt,
                "FR-004a: a reason that reports an attempt says which directory it was for, \
                 and a settings state names none ({env:?}, {place:?}): {both}"
            );
        }
    }
}

#[test]
fn w2e_no_sentence_names_a_platform_s_startup_file() {
    for (env, place, said) in every_explanation(AttemptDir::Home) {
        for sentence in [&said.reason, &said.action] {
            for file in [".bashrc", ".zshrc", ".profile", "$PROFILE"] {
                assert!(
                    !sentence.contains(file),
                    "FR-016: \"the startup script\" is the stored script path on every platform \
                     ({env:?}, {place:?}): {sentence}"
                );
            }
        }
    }
}

#[test]
fn w2f_no_sentence_sends_the_user_to_another_line() {
    for (env, place, said) in every_explanation(AttemptDir::Home) {
        for sentence in [&said.reason, &said.action] {
            assert!(
                !sentence.to_lowercase().contains("see below"),
                "FR-007: the note is complete by itself ({env:?}, {place:?}): {sentence}"
            );
        }
    }
}
