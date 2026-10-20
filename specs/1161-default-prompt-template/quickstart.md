# Quickstart: validation

## Part A: automated
1. `mise run test-core` (prompt_template, settings round trip).
2. `mise run gate`.

## Part B: visual pass (needs a display; run with the `visual-pass` skill)
1. Settings: the template field is multi-line, lists the six placeholders; type `Do {issu}`, save: the error names `{issu}` and the stored value is unchanged. Type `/speckit-autopilot #{issue}`, save: saved.
2. New worktree dialog: pick issue #42 and a CLI: the first-prompt field shows `/speckit-autopilot #42`. Edit it, change the issue: the edit stays. Clear it, create: the session starts with no prompt.
3. Empty template: the field is empty, as before.
4. MCP: with the template set, `create_session` on an issue worktree without `prompt`: `read_session_output` shows the rendered text; with `prompt: "hi"` it shows `hi`. With `gh` unavailable, title/body render empty within 5 s.
