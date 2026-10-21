//! The worktree session guard (feature 1210): decides whether a tool call of a launched session
//! targets a worktree that is not the session's own, so the daemon can deny it.
//!
//! Pure: no I/O beyond what [`PathFacts`] does, no daemon types. The daemon feeds it the session's
//! binding, the project's worktree list and the parsed `PreToolUse` payload, and turns a
//! [`GuardVerdict::Deny`] into the hook's deny body (`contracts/pretooluse-guard.md`).

pub mod paths;

use std::path::{Path, PathBuf};

use serde_json::Value;

use self::paths::{has_root, resolve, PathFacts, ResolvedPath};

/// The instruction every denial ends with (FR-003).
pub const DENY_MESSAGE: &str =
    "Do not work here; hand the work to a dedicated session with create_session.";

/// A worktree whose directory name starts with this is autopilot's prefetch worktree and is
/// reachable from any session (FR-007, research R5). Compared without regard to case.
pub const AUTOPILOT_PREFETCH_PREFIX: &str = "autopilot-prefetch";

/// The main checkout's shared build output directory, reachable from any session (FR-007).
pub const SHARED_BUILD_DIR: &str = "target-shared";

/// Where a session runs: its own worktree, the project root (a Default session), or unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionBinding {
    Worktree(PathBuf),
    Default(PathBuf),
    /// Cannot be determined: the guard fails open (FR-009).
    Unknown,
}

/// What a session's tool call does, as far as the guard judges it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    /// `Edit`, `Write`, `MultiEdit` (`file_path`) or `NotebookEdit` (`notebook_path`).
    Edit { tool: String, path: String },
    /// `Bash`, judged from M2.
    Bash { command: String },
    /// Reads and every tool the guard does not judge.
    Other,
}

impl ToolCall {
    /// Parse a `PreToolUse` payload's `tool_name` and `tool_input`. `Err` carries the reason the
    /// payload cannot be judged, for the fail-open log (FR-009).
    pub fn from_payload(payload: &Value) -> Result<Self, String> {
        let name = payload
            .get("tool_name")
            .and_then(Value::as_str)
            .ok_or("the payload has no tool_name")?;
        let field = |key: &str| {
            payload
                .get("tool_input")
                .and_then(|input| input.get(key))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("the {name} payload has no {key}"))
        };
        match name {
            "Edit" | "Write" | "MultiEdit" => Ok(Self::Edit {
                tool: name.to_owned(),
                path: field("file_path")?,
            }),
            "NotebookEdit" => Ok(Self::Edit {
                tool: name.to_owned(),
                path: field("notebook_path")?,
            }),
            "Bash" => Ok(Self::Bash {
                command: field("command")?,
            }),
            _ => Ok(Self::Other),
        }
    }
}

/// The payload's top-level `cwd`, if it has one.
pub fn payload_cwd(payload: &Value) -> Option<PathBuf> {
    payload
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|cwd| !cwd.is_empty())
        .map(PathBuf::from)
}

/// Everything one decision needs.
#[derive(Debug, Clone)]
pub struct GuardInput {
    pub binding: SessionBinding,
    pub project_root: PathBuf,
    /// Every location git lists for the project, the root included. Empty means unknown.
    pub worktrees: Vec<PathBuf>,
    /// The payload's `cwd`, which relative targets are resolved against.
    pub session_cwd: PathBuf,
    pub tool: ToolCall,
}

/// The guard's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardVerdict {
    Allow,
    /// Allowed because the call could not be judged; `reason` is for the log (FR-009).
    AllowUnknown {
        reason: String,
    },
    Deny {
        message: String,
        target: PathBuf,
    },
}

/// Decide one tool call (FR-002, FR-007, FR-009, FR-011).
pub fn decide(input: &GuardInput, facts: &dyn PathFacts) -> GuardVerdict {
    let own = match &input.binding {
        SessionBinding::Worktree(path) | SessionBinding::Default(path) => path,
        SessionBinding::Unknown => return unknown("the session's own worktree is not known"),
    };
    if input.worktrees.is_empty() {
        return unknown("the project's worktree list is not known");
    }
    match &input.tool {
        ToolCall::Edit { tool, path } => {
            let path = Path::new(path);
            let target = if has_root(path) {
                path.to_path_buf()
            } else {
                input.session_cwd.join(path)
            };
            judge(input, own, facts, tool, &target)
        }
        ToolCall::Bash { .. } | ToolCall::Other => GuardVerdict::Allow,
    }
}

fn unknown(reason: &str) -> GuardVerdict {
    GuardVerdict::AllowUnknown {
        reason: reason.to_owned(),
    }
}

/// Judge one absolute `target` of `tool`: the most specific worktree containing it must be the
/// session's own, an allowlisted one, or none.
fn judge(
    input: &GuardInput,
    own: &Path,
    facts: &dyn PathFacts,
    tool: &str,
    target: &Path,
) -> GuardVerdict {
    let resolved = resolve(facts, target);
    let root = resolve(facts, &input.project_root);
    let own = resolve(facts, own);
    if root.join(SHARED_BUILD_DIR).contains(&resolved) {
        return GuardVerdict::Allow;
    }
    let mut candidates: Vec<ResolvedPath> = input
        .worktrees
        .iter()
        .map(|worktree| resolve(facts, worktree))
        .collect();
    candidates.push(root.clone());
    candidates.push(own.clone());
    let Some(best) = candidates
        .into_iter()
        .filter(|candidate| candidate.contains(&resolved))
        .max_by_key(ResolvedPath::depth)
    else {
        return GuardVerdict::Allow;
    };
    // A directory under the root's worktrees folder that git no longer lists is a removed
    // worktree: nothing to protect.
    let unlisted = best == root && root.join(".claude/worktrees").contains(&resolved);
    // Autopilot's prefetch worktree is a worktree, never the checkout itself, whatever it is named.
    let prefetch = best != root && is_prefetch(&best);
    if best == own || prefetch || unlisted {
        return GuardVerdict::Allow;
    }
    GuardVerdict::Deny {
        message: format!(
            "Blocked: {tool} targets {}, which is in another worktree. {DENY_MESSAGE}",
            target.display()
        ),
        target: target.to_path_buf(),
    }
}

fn is_prefetch(worktree: &ResolvedPath) -> bool {
    worktree
        .last()
        .is_some_and(|name| name.to_lowercase().starts_with(AUTOPILOT_PREFETCH_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::paths::fake::FakeFacts;
    use super::*;
    use serde_json::json;

    const ROOT: &str = "/p/proj";
    const A: &str = "/p/proj/.claude/worktrees/a";
    const B: &str = "/p/proj/.claude/worktrees/b";

    fn input(binding: SessionBinding, path: &str) -> GuardInput {
        GuardInput {
            binding,
            project_root: ROOT.into(),
            worktrees: vec![ROOT.into(), A.into(), B.into()],
            session_cwd: A.into(),
            tool: ToolCall::Edit {
                tool: "Write".into(),
                path: path.into(),
            },
        }
    }

    fn in_a() -> SessionBinding {
        SessionBinding::Worktree(A.into())
    }

    fn default_session() -> SessionBinding {
        SessionBinding::Default(ROOT.into())
    }

    fn verdict(input: &GuardInput) -> GuardVerdict {
        decide(input, &FakeFacts::default())
    }

    fn denied(v: &GuardVerdict) -> bool {
        matches!(v, GuardVerdict::Deny { .. })
    }

    #[test]
    fn a_session_is_denied_in_another_worktree_naming_create_session() {
        let v = verdict(&input(in_a(), &format!("{B}/src/x.rs")));
        let GuardVerdict::Deny { message, target } = v else {
            panic!("expected a deny, got {v:?}");
        };
        assert!(message.contains("create_session"), "{message}");
        assert!(message.contains(&format!("{B}/src/x.rs")), "{message}");
        assert!(message.contains("Write"), "{message}");
        assert_eq!(target, PathBuf::from(format!("{B}/src/x.rs")));
    }

    #[test]
    fn a_session_is_allowed_in_its_own_worktree() {
        assert_eq!(
            verdict(&input(in_a(), &format!("{A}/src/x.rs"))),
            GuardVerdict::Allow
        );
        assert_eq!(
            verdict(&input(default_session(), &format!("{ROOT}/src/x.rs"))),
            GuardVerdict::Allow
        );
    }

    #[test]
    fn a_worktree_session_is_denied_in_the_project_root_outside_the_allowlist() {
        assert!(denied(&verdict(&input(
            in_a(),
            &format!("{ROOT}/src/x.rs")
        ))));
    }

    #[test]
    fn a_default_session_is_denied_in_any_worktree() {
        assert!(denied(&verdict(&input(
            default_session(),
            &format!("{A}/src/x.rs")
        ))));
    }

    #[test]
    fn a_removed_worktree_is_allowed() {
        let gone = format!("{ROOT}/.claude/worktrees/gone/x.rs");
        assert_eq!(verdict(&input(in_a(), &gone)), GuardVerdict::Allow);
        assert_eq!(
            verdict(&input(default_session(), &gone)),
            GuardVerdict::Allow
        );
    }

    #[test]
    fn a_path_outside_every_worktree_is_allowed() {
        assert_eq!(
            verdict(&input(in_a(), "/tmp/scratch.txt")),
            GuardVerdict::Allow
        );
    }

    #[test]
    fn an_unknown_binding_or_an_empty_list_fails_open_with_a_reason() {
        let unknown_binding = input(SessionBinding::Unknown, &format!("{B}/x"));
        assert!(matches!(
            verdict(&unknown_binding),
            GuardVerdict::AllowUnknown { .. }
        ));
        let mut no_list = input(in_a(), &format!("{B}/x"));
        no_list.worktrees.clear();
        assert!(matches!(
            verdict(&no_list),
            GuardVerdict::AllowUnknown { .. }
        ));
    }

    #[test]
    fn the_shared_build_dir_and_the_prefetch_worktree_are_allowed_from_any_session() {
        let prefetch = format!("{ROOT}/.claude/worktrees/autopilot-prefetch-1210");
        for binding in [in_a(), default_session()] {
            for path in [
                format!("{ROOT}/target-shared/debug/x"),
                format!("{prefetch}/src/x.rs"),
            ] {
                let mut i = input(binding.clone(), &path);
                i.worktrees.push(prefetch.clone().into());
                assert_eq!(verdict(&i), GuardVerdict::Allow, "{binding:?} {path}");
            }
        }
        // The carve-out is exactly the build directory: a sibling stays denied.
        assert!(denied(&verdict(&input(
            in_a(),
            &format!("{ROOT}/target-shared2/x")
        ))));
    }

    #[test]
    fn a_checkout_named_like_the_prefetch_worktree_is_not_exempt() {
        let root = "/p/autopilot-prefetch-tools";
        let own = format!("{root}/.claude/worktrees/a");
        let i = GuardInput {
            binding: SessionBinding::Worktree(own.clone().into()),
            project_root: root.into(),
            worktrees: vec![root.into(), own.into()],
            session_cwd: root.into(),
            tool: ToolCall::Edit {
                tool: "Write".into(),
                path: format!("{root}/src/x.rs"),
            },
        };
        assert!(denied(&verdict(&i)));
    }

    #[test]
    fn a_relative_target_is_resolved_against_the_session_cwd() {
        assert_eq!(verdict(&input(in_a(), "src/x.rs")), GuardVerdict::Allow);
        assert!(denied(&verdict(&input(in_a(), "../b/src/x.rs"))));
    }

    #[test]
    fn the_longest_matching_worktree_wins() {
        let mut i = input(in_a(), &format!("{A}/nested/x.rs"));
        i.worktrees.push(format!("{A}/nested").into());
        assert!(denied(&verdict(&i)));
    }

    #[test]
    fn reads_and_unknown_tools_are_never_judged() {
        let mut i = input(in_a(), "");
        i.tool = ToolCall::Other;
        assert_eq!(verdict(&i), GuardVerdict::Allow);
    }

    #[test]
    fn fifty_worktrees_are_decided_well_under_100_ms() {
        let mut i = input(in_a(), &format!("{ROOT}/.claude/worktrees/w49/x.rs"));
        i.worktrees = (0..50)
            .map(|n| PathBuf::from(format!("{ROOT}/.claude/worktrees/w{n}")))
            .collect();
        i.worktrees.push(ROOT.into());
        let start = std::time::Instant::now();
        assert!(denied(&verdict(&i)));
        assert!(start.elapsed() < std::time::Duration::from_millis(100));
    }

    /// One logical layout (own worktree `a`, other worktree `b`) spelled the way each platform
    /// spells it: the verdicts must agree (FR-014, SC-006).
    #[test]
    fn the_same_logical_layout_gives_the_same_verdict_on_every_platform_spelling() {
        // (root, own, other worktree as listed, target spelling of the other, case-insensitive)
        let layouts = [
            (
                "/home/me/p",
                "/home/me/p/.claude/worktrees/a",
                "/home/me/p/.claude/worktrees/b",
                "/home/me/p/.claude/worktrees/b/x.rs",
                false,
            ),
            (
                "/Users/me/P",
                "/Users/me/P/.claude/worktrees/a",
                "/Users/me/P/.claude/worktrees/b",
                "/users/ME/p/.claude/worktrees/B/x.rs",
                true,
            ),
            (
                "C:\\Me\\P",
                "C:\\Me\\P\\.claude\\worktrees\\a",
                "C:\\Me\\P\\.claude\\worktrees\\b",
                "c:/me/p/.claude/worktrees/b/x.rs",
                true,
            ),
        ];
        for (root, own, other, other_target, insensitive) in layouts {
            let facts = FakeFacts {
                insensitive,
                ..Default::default()
            };
            let at = |path: &str| GuardInput {
                binding: SessionBinding::Worktree(own.into()),
                project_root: root.into(),
                worktrees: vec![root.into(), own.into(), other.into()],
                session_cwd: own.into(),
                tool: ToolCall::Edit {
                    tool: "Edit".into(),
                    path: path.into(),
                },
            };
            let deny = decide(&at(other_target), &facts);
            let allow = decide(&at(&format!("{own}/x.rs")), &facts);
            assert!(denied(&deny), "{root}: {deny:?}");
            assert_eq!(allow, GuardVerdict::Allow, "{root}");
        }
    }

    #[test]
    fn from_payload_reads_the_edit_tools_the_bash_command_and_ignores_the_rest() {
        let edit = |tool: &str, key: &str| {
            ToolCall::from_payload(&json!({"tool_name": tool, "tool_input": {key: "/x/y"}}))
        };
        for tool in ["Edit", "Write", "MultiEdit"] {
            assert_eq!(
                edit(tool, "file_path"),
                Ok(ToolCall::Edit {
                    tool: tool.into(),
                    path: "/x/y".into()
                })
            );
        }
        assert_eq!(
            edit("NotebookEdit", "notebook_path"),
            Ok(ToolCall::Edit {
                tool: "NotebookEdit".into(),
                path: "/x/y".into()
            })
        );
        assert_eq!(
            ToolCall::from_payload(&json!({"tool_name": "Bash", "tool_input": {"command": "ls"}})),
            Ok(ToolCall::Bash {
                command: "ls".into()
            })
        );
        assert_eq!(
            ToolCall::from_payload(
                &json!({"tool_name": "Read", "tool_input": {"file_path": "/x"}})
            ),
            Ok(ToolCall::Other)
        );
    }

    #[test]
    fn an_unparsable_payload_yields_a_reason() {
        assert!(ToolCall::from_payload(&json!({})).is_err());
        let err =
            ToolCall::from_payload(&json!({"tool_name": "Write", "tool_input": {}})).unwrap_err();
        assert!(err.contains("file_path"), "{err}");
        assert_eq!(payload_cwd(&json!({"cwd": "/a"})), Some("/a".into()));
        assert_eq!(payload_cwd(&json!({})), None);
    }
}
