# Autopilot ledger — #890 daemon-settings-accordion

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #890 "Group daemon settings sections in an accordion (new material component)"; user at run start: "do UI refinment and unification, split it for proper sections" (read as: unify the daemon form's look and split settings into proper sections, one PR per milestone). labels: enhancement, flow:feature (now also in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #890
- **Worktree branch**: claude/project-thread-rwlog8
- **Started**: 2026-10-18
- **Phase**: M2
- **Next step**: M2 handover: T015 visual pass, review A (scoped gate in its shadow), review B, full gate (scripts/gate.sh), PR files

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1063 | Design: spec, plan, tasks | merged (rebase) | ad4d1946127e6dc0c06e4add04416f2fc962db00 |
| #1068 | M1: AccordionSection summary and required marker | merged (rebase) | c0507555cf2611f462f7a55a590087b57730364f |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T005 | full | Showcase `AccordionSection` shows summary (closed), open, required marker | #1068 | merged |
| M2 | T006–T015 | full | Every daemon kind's form in FR-002 sections, Connection open, errors open their section; user guide | — | pending |
| M3 | T016–T028 | full | One field/section gap, FieldGroup labels, context select, section summaries and markers; user guide | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | The issue's accordion component and the local container's five sections already shipped in 804 M4 — what is left? | Extend the component (closed-state summary, required marker); split every kind's form (host, SSH, Kubernetes, container's non-container fields) into sections; unify look; one PR per milestone | agent-resolved | specs/804-session-service-in-daemons/spec.md FR-012/FR-013; crates/micold-client/src/ui/material/accordion_section.rs |
| D2 | spec | Same concept, different names per kind | "Connection", "Credentials", "Resources", "Sessions" everywhere; container "Limits" renamed "Resources" | agent-resolved | spec.md FR-003 |
| D3 | clarify | Does a required error shown on leaving an empty field (937 FR-006) force its section open? | Yes; any shown required error counts under FR-004 | agent-resolved | specs/937-required-field-markers/spec.md FR-006; crates/micold-core/src/required.rs FieldRequirement::Missing |
| D4 | clarify | Kubernetes "Image" section renamed "Container"? | No; container section also picks the runtime, keeps 804's name | agent-resolved | specs/804-session-service-in-daemons/spec.md FR-012 |
| D5 | clarify | Exact summary text per section | Fixed in the plan's UI contract | agent-resolved | spec.md FR-014 |
| D6 | tasks | Milestone order and size: milestones.md rule 1 (M1 = P1) and rule 3 (split >10 tasks) vs FR-016's three PRs | FR-016 order kept (M1 component, M2 US1, M3 US2 + summaries); M2 (10) and M3 (13 tasks) not split — user asked for three PRs, and a split of M3 has no separate observable deliverable | agent-resolved | spec.md FR-016; research.md R10 |
| D7 | tasks | speckit-analyze F1–F9 | Fixed F1 (FieldGroup uses spacing::SM, not ui/settings::FIELD_GAP; data-model), F2 (SC-002 + T009 pin form state), F3+F7 (T008 header test + Tab focus), F4 (FR-006 Runs-in gap), F5 (FR-016 wording), F6 (FR-009 namespace), F9 (summary under title); F8 no change (glue exception) | agent-resolved | speckit-analyze report |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 24cc8d1b:3b6f429d | CHANGES: 1 MAJOR (F1 FR-009 chooser), 3 MINOR — all fixed (F1 SSH select pattern; F2 required errors force sections open; F3 cautions/group notes placement; F4 SSH single section stated) |
| Spec | 2 (scoped, sonnet) | afc2d2d26e2837ad5c3c52dad8b0e4c936921f78:8e6bc4dbb04b88be60c3e57a63faaf2d358cc75b | CLEAN (F1–F4 fixed) |
| Clarify | 1 | spec.md Clarifications 2026-10-18 | 3 Q, all agent-resolved (D3–D5); FR-004 + edge case changed |
| Clarify | 2 | — | no critical ambiguities |
| Plan | 1 | 6a71819950f543013db8bf4fd875133e7b90b673:7d34cc6b80783b6daaee02c86a179f7f5f8d7174 | CLEAN, no findings |
| Tasks (analyze) | 1 | — | 0 CRITICAL/HIGH; 2 MEDIUM, 7 LOW — fixed per D7 |
| Tasks | 1 | ba794e2c48f587ac992b44fca4cfd51218aef620:939f376280476c9b308aca897e3933cf3dad7586 | CLEAN; 1 MINOR (M1 not the P1 story) — declined, see Declined |
| Review A M1 (code-review high) | 1 | — (run on f8a8cb27) | 7 unranked findings: 3 fixed (two-line header fit now asserted; empty summary drawn as none, U8; narrow test finds summary by text), 4 declined |
| Review B M1 (conformance, sonnet) | 1 | 0f60a67f0434ccd3637739a1679e21c9a10fb5a8:e79bfd617bd71c89b048059deb323c4dcef46cfe | CLEAN; 3 MINOR — F2, F3 fixed prose only (cycle-log note, research R4); F1 Verify not runnable in its sandbox (unit ran both: 13 + 41 passed) |
| Visual pass M1 (quickstart B1) | 1 | — | PASS, both themes, 150px narrow; visual-pass/m1/ |

## Declined review findings

- Tasks r1 F1 (MINOR, M1 is not the P1 story): FR-016 and the user's ask fix the three-PR order (component, sections, look); M1 has an observable showcase deliverable. Ledger D6.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

- Review A M1: title position differs between headers with and without a summary — declined, the two-line header is the design (research R4).
- Review A M1: `Mounted::painted_text` duplicates `tests/support/layout.rs::painted_text` — declined, in-crate tests cannot reach `tests/support` (`material` is `pub(crate)`; same reason `test_support.rs` exists).
- Review A M1: required pose also carries the summary — declined, matches how M3 sections pose both; FR-015 asks for "closed with the marker".
- Review A M1: marker hidden while open — declined, FR-012 and research R5 (the open field shows its own marker).

## Handover

M2 (T006–T014 done and ticked; code, tests, docs written, uncommitted until this commit): branch
`claude/project-thread-rwlog8` reset to origin/main c0507555 (no branch-start.sh needed).
Done: `features/daemon_sections.rs` (Section::of/title/holding_error/holding), `State::section_open`
rewrite + `flipped_sections` cleared on `RuntimeChosen`, `container_fields::accordion` + six
container sections, `ui/settings/daemons.rs` `form` + `kube_sections` + `auto_start`, tests
(`daemon_sections.rs`, `daemons_feature.rs`, `daemons_section_geometry.rs`, `gates/daemon_form_fits.rs`),
user guide. Green: `cargo test -p micold-client --test daemon_sections --test daemons_feature
--test daemons_section_geometry --test layout_snapshot` (with CARGO_INCREMENTAL=0 etc).
Next: (1) T015 visual pass, both themes, into `visual-pass/m2/`; (2) tdd cycle-log entry for M2
(T009 red evidence not measured on main; say so); (3) scoped gate + review A (`code-review` high on
origin/main...HEAD) then review B (conformance, sonnet, via `claude -p`); (4) `scripts/gate.sh`;
(5) push `--force-with-lease=claude/project-thread-rwlog8:`, write pr-title.txt / pr-body.md in the
scratchpad (title "#890: Group daemon settings sections in an accordion (new material component) —
M2 every daemon kind's form in sections"). Other tests not yet run beyond the four binaries
(a full gate will show any other test that opened the Container section by default).

## Open escalation

None.

## Follow-ups not done

- M1 visual pass: on an inert header (no `on_toggle`, the showcase's static poses) the title dims as a disabled button but the summary keeps `on_surface_variant`. Every daemon-form section has a toggle, so no form shows it; dim the summary with the header if an inert use appears.
