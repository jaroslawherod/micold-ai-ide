//! Permission prompts of a structured session (#942): what the CLI asks through the prompt tool,
//! what the user may answer, the answer the CLI is sent, and the session's always-allow rules.
//! Render-free and process-free; the daemon holds the requests and the client draws them.
//!
//! - [`request`]: a prompt-tool call parsed ([`PromptCall`]) and classified ([`RequestKind`]).
//! - [`decision`]: which choices fit a request ([`validate`]) and the answer JSON ([`answer`]).
//! - [`rules`]: the rule Always allow adds ([`AllowRule`], research R9).

pub mod decision;
pub mod request;
pub mod rules;

pub use decision::{answer, input_preview, validate, DecisionRefusal, PREVIEW_BYTES};
pub use request::{
    classify, parse_call, Malformed, PromptCall, Question, QuestionOption, RequestKind,
};
pub use rules::AllowRule;

use crate::session::PermissionMode;

/// The prompt tool as the CLI names it: `mcp__<server>__<tool>` (research R1).
pub const PROMPT_TOOL: &str = "mcp__micold__permission_prompt";

/// The permission arguments of a structured launch (contracts/claude-permission.md *Launch*):
/// `--allow-dangerously-skip-permissions` always, so a later switch to Bypass is reachable (R6);
/// `--permission-mode <mode>` only when `mode` is not Default (feature 938 FR-006a); the prompt
/// tool only when `prompt_tool` (the tool server bound the session and prompts are on, R14).
pub fn launch_args(mode: PermissionMode, prompt_tool: bool) -> Vec<String> {
    let mut args = vec!["--allow-dangerously-skip-permissions".to_string()];
    if !mode.is_default() {
        args.extend(["--permission-mode".to_string(), mode.cli_name().to_string()]);
    }
    if prompt_tool {
        args.extend([
            "--permission-prompt-tool".to_string(),
            PROMPT_TOOL.to_string(),
        ]);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_tool_is_named_after_the_tool_servers_binding() {
        assert_eq!(
            PROMPT_TOOL,
            format!(
                "mcp__{}__permission_prompt",
                crate::mcp::jsonrpc::SERVER_NAME
            )
        );
    }

    #[test]
    fn a_default_launch_adds_only_the_bypass_reachability_flag() {
        assert_eq!(
            launch_args(PermissionMode::Default, false),
            ["--allow-dangerously-skip-permissions"],
            "Default adds no mode override (FR-013)"
        );
    }

    #[test]
    fn a_chosen_mode_is_named_on_the_launch() {
        for mode in [
            PermissionMode::AcceptEdits,
            PermissionMode::Plan,
            PermissionMode::Bypass,
        ] {
            assert_eq!(
                launch_args(mode, false),
                [
                    "--allow-dangerously-skip-permissions",
                    "--permission-mode",
                    mode.cli_name()
                ],
                "{mode:?}"
            );
        }
    }

    #[test]
    fn the_prompt_tool_is_named_only_when_asked_for() {
        assert_eq!(
            launch_args(PermissionMode::Plan, true),
            [
                "--allow-dangerously-skip-permissions",
                "--permission-mode",
                "plan",
                "--permission-prompt-tool",
                "mcp__micold__permission_prompt"
            ]
        );
        assert_eq!(
            launch_args(PermissionMode::Default, true),
            [
                "--allow-dangerously-skip-permissions",
                "--permission-prompt-tool",
                "mcp__micold__permission_prompt"
            ]
        );
    }
}
