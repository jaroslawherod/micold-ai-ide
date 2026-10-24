# Quickstart: validating Structured Session Permission Prompts (#942)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Request parsing, kinds, answers against the recorded fixtures | `cargo test -p micold-core --test structured_permission` | every fixture's call classified as its kind; `answer()` for the fixture's choice equals the recorded answer; malformed calls refused (FR-002, FR-004, FR-010, FR-020, SC-004) |
| Mode, rules, decisions, control lines | `cargo test -p micold-core --lib -- structured::permission structured::control` | `launch_args` per mode; `AllowRule` per tool; `validate` per kind; `encode_set_mode`; `Status`, `ControlResponse`, `Init.permission_mode` parsed from `prompt-*.jsonl` |
| Persisted and summarised mode | `cargo test -p micold-core --test session_store --test protocol_roundtrip` | legacy record → `Default`; round trip; `skip_serializing_if` (FR-013, FR-015) |
| Daemon against the stand-in CLI | `cargo test -p micold-daemon --test structured_permission --test structured_permission_mode` | hold until decided; each choice; two clients, first wins; late attach replays; withdrawal on result, process end, stop, remove, hang-up within 1 s; refused late decisions; activity *needs permission* and back; always-allow skips the second request; mode at launch and after restart; mode change while running, refused change (FR-019, SC-001–SC-003, SC-005) |
| Terminal sessions and #938 unchanged | `scripts/gate.sh` | every existing test green; the one #938 assertion this feature supersedes (FR-006a's argv list) is changed in M2, nothing else (SC-007) |
| Window state | `cargo test -p micold-client --test features_permission_prompts` | drafts, `can_submit`, choice building, placement under the card or at the end, resolution labels, per-session isolation, "waiting" flag (FR-006–FR-009) |
| Shared components | `cargo test -p micold-client --test showcase_completeness` and the builder gate | `ChoiceList` and `PromptCard` have a showcase entry and a builder |
| Geometry | `cargo test -p micold-client --test layout_snapshot` | `gates/prompt_card_fits.rs`: the action row stays inside the card at the narrowest pane width |

Entities: [data-model.md](data-model.md). Contracts: [contracts/](contracts/).

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; the dev client against a scratch project; #939's
transcript view and #941's composer on `main`.

1. Start a structured session (chevron → **Claude Code (structured)**) in **Default**. Ask it to
   run `echo hi > out.txt`. The `Bash` card shows the command with **Allow**, **Deny**, **Always
   allow**; the composer shows *Waiting for you*; the sidebar row shows *Blocked: needs
   permission*. Allow: the card shows *Allowed*, the turn finishes. Clicks beyond the answer: 1
   (SC-006).
2. Ask for the same command twice with **Always allow** on the first: the second runs without a
   prompt. Deny another with a message: Claude's reply quotes it.
3. Open a second window on the same session with a request pending: both show it; answer in one,
   it closes in both.
4. A Write of a large file (> 64 KiB): a bounded preview with **Show all**; allowing writes the
   whole file.
5. Change the mode to **Accept edits** from the mode control while the session idles; ask for a file
   edit: no prompt. Choose **Bypass**: a confirmation naming its effect; confirm; the view says
   *Bypass: Claude runs every tool without asking*.
6. Create a session in **Plan** (the creation flow's mode choice). Ask for a change: the plan view
   shows the whole plan with **Approve and accept edits**, **Approve**, **Keep planning**. Keep
   planning with a note, then Approve and accept edits: the shown mode becomes *Accept edits*.
7. Ask Claude to ask two questions, one multi-select (`AskUserQuestion`): options with
   descriptions, single and multiple choice, **Other**; Submit is disabled until both are
   answered; the reply names the answers. Two questions answered in at most three presses beyond
   the choices (SC-006).
8. With a request pending, interrupt the turn (#941) and stop the session: the request disappears.
9. Keep a request pending for more than 10 minutes, then Allow: the tool runs (R3).
10. A terminal session asks in its own terminal screen as before.
11. Light and dark theme: request card, plan view, question view, mode control, Bypass label,
    keyboard focus order through the actions (Tab, Space/Enter).

Record the pass with the `visual-pass` skill into `visual-pass/`.

## Results (close unit, 2026-10-24)

### Part A — run at `ae23a467` (main after M9, PR #1312)

The full gate (`/mnt/project-files/scripts/split-gate.sh`, `scripts/gate.sh`'s steps in CI's order,
split to fit the cloud container's disk) passed: fmt, clippy (core, then workspace, `-D warnings`),
every test binary of the workspace (10,129 passed, 0 failed, 18 ignored, 639 binaries), the
as-a-normal-user tests and `scripts/tests/*.test.sh`. The rows above, read from that run:

| Check | Result |
|---|---|
| Request parsing, kinds, answers against the fixtures | core `structured_permission` 9 passed |
| Mode, rules, decisions, control lines | core lib `structured::permission` 34 and `structured::control` 6 passed |
| Persisted and summarised mode | `session_store` 12, `protocol_roundtrip` 45 passed |
| Daemon against the stand-in CLI | daemon `structured_permission` 30, `structured_permission_mode` 13 passed |
| Terminal sessions and #938 unchanged | the whole gate green (SC-007) |
| Window state | `features_permission_prompts` 26 passed |
| Shared components | `showcase_completeness` 46, `material_builder_api` 13 passed |
| Geometry | `layout_snapshot` 103 passed (`gates/prompt_card_fits.rs`, `gates/composer_fits.rs`) |

### Part B — offscreen only; the live steps are not run

This cloud container has no logged-in `claude`, so no step that needs a real request was run.
What each step checks was covered offscreen instead (tiny-skia captures after `layout::settle`,
light and dark, in [visual-pass/](visual-pass/)) or by a test:

| Step | Covered by | Live run |
|---|---|---|
| 1 request card, *Waiting for you*, sidebar *Blocked* | `visual-pass/m7-prompt-cards-*`, `m9-waiting-*`; `transcript_permission_view`, `composer_fits` | not run |
| 2 Always allow, deny with a message | daemon `each_decision_reaches_the_cli_and_always_allow_skips_an_identical_call` | not run |
| 3 two windows, first answer wins | daemon `a_call_is_shown_to_every_client_and_held_until_one_decides`, `of_two_concurrent_decisions_exactly_one_wins` | not run |
| 4 large Write, **Show all** | `a_cut_preview_offers_show_all_which_asks_for_the_whole_input` | not run |
| 5 mode control, Bypass confirmation and label | `visual-pass/m9-accept-edits-*`, `m9-bypass-confirm-*`, `m9-bypass-*`, `m8-bypass-bar-*`; `pressing_the_mode_control_opens_the_four_modes_and_picking_one_asks_for_it` | not run |
| 6 create in Plan, plan view, shown mode after approval | `visual-pass/m8-start-list-*`, `m7-prompt-cards-*`; daemon `approving_a_plan_with_accept_edits_sets_the_mode_before_claude_is_answered` | not run |
| 7 two questions, Other, Submit disabled | `visual-pass/m7-choice-list-*`; `a_question_is_submitted_only_once_every_question_is_answered` | not run |
| 8 interrupt and stop withdraw the request | `visual-pass/m9-withdrawn-*`; daemon `a_result_withdraws_the_request_and_the_cli_is_denied`, `a_request_whose_agent_hangs_up_is_withdrawn`, `a_request_is_withdrawn_when_its_session_stops` | not run |
| 9 a request pending over 10 minutes | `mcp_binding_plan` `a_structured_binding_waits_for_the_prompt_tools_answer_and_a_terminal_one_is_unchanged` (the binding's tool timeout) | not run |
| 10 a terminal session asks in its terminal | `a_terminal_sessions_launch_and_binding_carry_no_permission_setting` | not run |
| 11 light and dark, focus order | every capture above in both schemes; focus order not checked | not run |

The click counts of SC-006 (steps 1 and 7) were never measured live. Follow-up for jaro: run Part
B 1–11 by hand with a logged-in `claude` and the dev client.
