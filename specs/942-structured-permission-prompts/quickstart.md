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
