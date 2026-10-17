//! Task progress read from a worktree's files (feature 813, research R9): the feature phase and
//! the ticked share of its task list, shown on the location's Overview card.

use crate::overview::CardState;
use std::path::Path;

/// Where a feature is in the spec-kit flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Spec,
    Plan,
    Tasks,
    Implement,
    Pr,
    Done,
}

impl Phase {
    /// The word the card shows.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Spec => "Spec",
            Phase::Plan => "Plan",
            Phase::Tasks => "Tasks",
            Phase::Implement => "Implement",
            Phase::Pr => "PR",
            Phase::Done => "Done",
        }
    }
}

/// Ticked boxes over all boxes; `total > 0` always.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub done: u32,
    pub total: u32,
}

/// What the worktree's files say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileProgress {
    pub feature_dir: Option<String>,
    pub artifact: Option<Phase>,
    pub ledger: Option<Phase>,
    pub tasks: Option<Ratio>,
}

/// Where a [`Progress`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    TaskList,
    Ledger,
    Hooks,
    None,
}

/// A card's progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub phase: Option<Phase>,
    pub ratio: Option<Ratio>,
    pub source: Source,
}

impl Default for Progress {
    /// No source: no phase and no ratio, which a card shows as *No task progress* (FR-017).
    fn default() -> Self {
        Self {
            phase: None,
            ratio: None,
            source: Source::None,
        }
    }
}

/// The feature folder, relative to the worktree root: `.specify/feature.json`'s
/// `feature_directory` as written, else `specs/<name>` for the one name in `spec_dirs` whose
/// leading number equals the numeric `ticket`; `None` when none or several match.
pub fn feature_dir(
    feature_json: Option<&str>,
    spec_dirs: &[String],
    ticket: Option<&str>,
) -> Option<String> {
    let from_json = feature_json
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|v| v.get("feature_directory")?.as_str().map(str::to_owned))
        .filter(|dir| !dir.trim().is_empty());
    if from_json.is_some() {
        return from_json;
    }
    let number: u64 = ticket?.parse().ok()?;
    let mut matching = spec_dirs.iter().filter(|name| {
        let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u64>() == Ok(number)
    });
    let found = matching.next()?;
    matching.next().is_none().then(|| format!("specs/{found}"))
}

/// Ticked over all `- [ ]` / `* [x]` lines; `None` when there is no checkbox (never 0/0).
pub fn checkbox_ratio(text: &str) -> Option<Ratio> {
    let (mut done, mut total) = (0u32, 0u32);
    for line in text.lines() {
        let rest = line.trim_start();
        let Some(rest) = rest.strip_prefix(['-', '*']) else {
            continue;
        };
        let ticked = match rest.strip_prefix(" [").and_then(|r| r.split_once("] ")) {
            Some((" ", _)) => false,
            Some(("x" | "X", _)) => true,
            _ => continue,
        };
        total += 1;
        done += u32::from(ticked);
    }
    (total > 0).then_some(Ratio { done, total })
}

/// The phase the autopilot ledger's `- **Phase**: <word>` line names; `None` for no line or an
/// unknown word. A numeric prefix (`4-milestones`) is ignored.
pub fn ledger_phase(text: &str) -> Option<Phase> {
    let word = text
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("- **Phase**:"))?
        .trim()
        .to_ascii_lowercase();
    let word = word.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-');
    Some(match word {
        "spec" | "clarify" | "bug" => Phase::Spec,
        "plan" => Phase::Plan,
        "tasks" => Phase::Tasks,
        "milestones" | "bugfix" | "chore" => Phase::Implement,
        "close" => Phase::Pr,
        "done" => Phase::Done,
        _ => return None,
    })
}

/// The phase the feature's files imply: ticked tasks.md, tasks.md, plan.md, spec.md.
pub fn artifact_phase(spec: bool, plan: bool, tasks: bool, ratio: Option<Ratio>) -> Option<Phase> {
    if tasks {
        return Some(if ratio.is_some_and(|r| r.done > 0) {
            Phase::Implement
        } else {
            Phase::Tasks
        });
    }
    if plan {
        return Some(Phase::Plan);
    }
    spec.then_some(Phase::Spec)
}

/// The card's phase: the ledger, else Done for a Done card, else PR for an open pull request,
/// else what the files imply.
pub fn phase(
    ledger: Option<Phase>,
    card_state: CardState,
    pr_open: bool,
    artifact: Option<Phase>,
) -> Option<Phase> {
    ledger
        .or((card_state == CardState::Done).then_some(Phase::Done))
        .or(pr_open.then_some(Phase::Pr))
        .or(artifact)
}

/// The most a progress file may weigh; a larger one counts as absent.
const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// The file's text; `None` when missing, unreadable, over [`MAX_FILE_BYTES`] or not UTF-8.
fn read_text(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// What the worktree at `root` says about the feature of `ticket`. Blocking I/O: call it off
/// the UI thread. Anything missing, huge or unreadable is absent.
pub fn read(root: &Path, ticket: Option<&str>) -> FileProgress {
    let feature_json = read_text(&root.join(".specify/feature.json"));
    let spec_dirs: Vec<String> = std::fs::read_dir(root.join("specs"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    let Some(feature_dir) = feature_dir(feature_json.as_deref(), &spec_dirs, ticket) else {
        return FileProgress::default();
    };
    let dir = root.join(&feature_dir);
    let tasks_text = read_text(&dir.join("tasks.md"));
    let tasks = tasks_text.as_deref().and_then(checkbox_ratio);
    let exists = |name: &str| dir.join(name).is_file();
    FileProgress {
        artifact: artifact_phase(
            exists("spec.md"),
            exists("plan.md"),
            tasks_text.is_some() || exists("tasks.md"),
            tasks,
        ),
        ledger: read_text(&dir.join("autopilot.md"))
            .as_deref()
            .and_then(ledger_phase),
        tasks,
        feature_dir: Some(feature_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(done: u32, total: u32) -> Option<Ratio> {
        Some(Ratio { done, total })
    }

    #[test]
    fn checkbox_variants_count() {
        let text = "- [ ] a\n- [x] b\n* [X] c\n  - [ ] d\n";
        assert_eq!(checkbox_ratio(text), r(2, 4));
    }

    #[test]
    fn checkbox_ignores_other_lines() {
        let text = "# Title\n- plain\n-[x] nospace\n- [y] odd\n- [x]no trailing space\n1. [x] numbered\n- [x] ok\n";
        assert_eq!(checkbox_ratio(text), r(1, 1));
    }

    #[test]
    fn checkbox_none_is_none_never_zero_of_zero() {
        assert_eq!(checkbox_ratio(""), None);
        assert_eq!(checkbox_ratio("no boxes\n"), None);
    }

    #[test]
    fn ledger_vocabulary() {
        let p = |w: &str| ledger_phase(&format!("# Ledger\n- **Phase**: {w}\n- other\n"));
        for w in ["spec", "clarify", "bug"] {
            assert_eq!(p(w), Some(Phase::Spec), "{w}");
        }
        assert_eq!(p("plan"), Some(Phase::Plan));
        assert_eq!(p("tasks"), Some(Phase::Tasks));
        for w in ["milestones", "bugfix", "chore"] {
            assert_eq!(p(w), Some(Phase::Implement), "{w}");
        }
        assert_eq!(p("close"), Some(Phase::Pr));
        assert_eq!(p("done"), Some(Phase::Done));
    }

    #[test]
    fn ledger_numeric_prefix_stripped_and_unknown_is_none() {
        assert_eq!(
            ledger_phase("- **Phase**: 4-milestones\n"),
            Some(Phase::Implement)
        );
        assert_eq!(ledger_phase("- **Phase**: 2-plan\n"), Some(Phase::Plan));
        assert_eq!(ledger_phase("- **Phase**: wibble\n"), None);
        assert_eq!(ledger_phase("nothing here\n"), None);
    }

    #[test]
    fn feature_dir_prefers_feature_json() {
        let dirs = vec!["813-a".to_string()];
        assert_eq!(
            feature_dir(
                Some(r#"{"feature_directory":"specs/900-x"}"#),
                &dirs,
                Some("813")
            ),
            Some("specs/900-x".into())
        );
    }

    #[test]
    fn feature_dir_falls_back_to_the_one_matching_folder() {
        let dirs = vec!["812-a".to_string(), "813-b".to_string()];
        assert_eq!(
            feature_dir(None, &dirs, Some("813")),
            Some("specs/813-b".into())
        );
        assert_eq!(
            feature_dir(Some("not json"), &dirs, Some("813")),
            Some("specs/813-b".into())
        );
    }

    #[test]
    fn feature_dir_none_when_ambiguous_or_unmatched() {
        let dirs = vec!["813-a".to_string(), "813-b".to_string()];
        assert_eq!(feature_dir(None, &dirs, Some("813")), None);
        assert_eq!(feature_dir(None, &dirs, Some("7")), None);
        assert_eq!(feature_dir(None, &dirs, Some("ABC-1")), None);
        assert_eq!(feature_dir(None, &dirs, None), None);
    }

    #[test]
    fn artifact_phase_ladder() {
        assert_eq!(
            artifact_phase(true, true, true, r(1, 3)),
            Some(Phase::Implement)
        );
        assert_eq!(
            artifact_phase(true, true, true, r(0, 3)),
            Some(Phase::Tasks)
        );
        assert_eq!(artifact_phase(true, true, true, None), Some(Phase::Tasks));
        assert_eq!(artifact_phase(true, true, false, None), Some(Phase::Plan));
        assert_eq!(artifact_phase(true, false, false, None), Some(Phase::Spec));
        assert_eq!(artifact_phase(false, false, false, None), None);
    }

    #[test]
    fn phase_precedence() {
        let art = Some(Phase::Plan);
        assert_eq!(
            phase(Some(Phase::Spec), CardState::Done, true, art),
            Some(Phase::Spec)
        );
        assert_eq!(phase(None, CardState::Done, true, art), Some(Phase::Done));
        assert_eq!(phase(None, CardState::Idle, true, art), Some(Phase::Pr));
        assert_eq!(phase(None, CardState::Idle, false, art), art);
        assert_eq!(phase(None, CardState::Idle, false, None), None);
    }

    fn write(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[test]
    fn read_a_missing_worktree_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(dir.path(), Some("813")), FileProgress::default());
    }

    #[test]
    fn read_finds_the_feature_and_its_files() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "specs/813-x/spec.md", b"s");
        write(dir.path(), "specs/813-x/plan.md", b"p");
        write(dir.path(), "specs/813-x/tasks.md", b"- [x] a\n- [ ] b\n");
        write(
            dir.path(),
            "specs/813-x/autopilot.md",
            b"- **Phase**: close\n",
        );
        let got = read(dir.path(), Some("813"));
        assert_eq!(got.feature_dir.as_deref(), Some("specs/813-x"));
        assert_eq!(got.artifact, Some(Phase::Implement));
        assert_eq!(got.ledger, Some(Phase::Pr));
        assert_eq!(got.tasks, r(1, 2));
    }

    #[test]
    fn read_huge_and_non_utf8_files_count_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let mut huge = b"- [x] a\n".to_vec();
        huge.resize(1024 * 1024 + 1, b' ');
        write(dir.path(), "specs/813-x/tasks.md", &huge);
        write(dir.path(), "specs/813-x/autopilot.md", &[0xff, 0xfe, 0xfd]);
        let got = read(dir.path(), Some("813"));
        assert_eq!(got.tasks, None);
        assert_eq!(got.ledger, None);
    }
}
