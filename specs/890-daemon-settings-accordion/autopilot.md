# Autopilot ledger — #890 daemon-settings-accordion

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #890 "Group daemon settings sections in an accordion (new material component)"; user at run start: "do UI refinment and unification, split it for proper sections" (read as: unify the daemon form's look and split settings into proper sections, one PR per milestone). labels: enhancement, flow:feature (now also in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #890
- **Worktree branch**: claude/project-thread-rwlog8
- **Started**: 2026-10-18
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1063 | Design: spec, plan, tasks | merged (rebase) | ad4d1946127e6dc0c06e4add04416f2fc962db00 |
| #1068 | M1: AccordionSection summary and required marker | merged (rebase) | c0507555cf2611f462f7a55a590087b57730364f |
| #1075 | M2: daemon forms in sections | merged (rebase) | b33e26ed297d0b02dc5f0864c640da7d9768ecb6 |
| #1082 | M3: one look, summaries, marker | merged (rebase) | f3d8a05ad7b45eedc27ff7b5330ca32712324012 |
| (orchestrator opens) | Close: tdd-verify, T029, spec closed | in review | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T005 | full | Showcase `AccordionSection` shows summary (closed), open, required marker | #1068 | merged |
| M2 | T006–T015 | full | Every daemon kind's form in FR-002 sections, Connection open, errors open their section; user guide | #1075 | merged |
| M3 | T016–T028 | full | One field/section gap, FieldGroup labels, context select, section summaries and markers; user guide | #1082 | merged |

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
| Review A M2 (code-review high, headless) | 1 | be7183037bd8e32b5f7cf0ba45918bbf1f2b3f23:d53128e282e961dc1276cca8a18b84ebcca69cc8 | 2 MAJOR + 5 MINOR: both MAJOR fixed with tests first (a click on an error-held section is not recorded; `allowed` refusal maps to Credentials and is shown there); MINOR same-kind re-choice no longer clears the flips (fixed); 4 MINOR declined |
| Review B M2 (conformance, sonnet) | 1 | be7183037bd8e32b5f7cf0ba45918bbf1f2b3f23:d53128e282e961dc1276cca8a18b84ebcca69cc8 | CLEAN; 2 MINOR: F1 fixed (T009 red measured on origin/main: Cancel bottom 1851 > 800; cycle-log), F2 its scratch worktree, outside the repo |
| Visual pass M2 (quickstart B2–B3) | 1 | — | PASS light; dark Container + Resources error; Tab walk and dark Host/SSH/Kubernetes not judged; visual-pass/m2/ |
| Review A M3 (headless reviewer, high-level correctness) | 1 | ff248a7331fa644dfa71963d23b13b695d01150a:b33e26ed297d0b02dc5f0864c640da7d9768ecb6 | CLEAN |
| Review B M3 (conformance, sonnet) | 1 | ff248a7331fa644dfa71963d23b13b695d01150a:b33e26ed297d0b02dc5f0864c640da7d9768ecb6 | CHANGES: F1 BLOCKER lib test did not compile (fixed, 809 lib tests pass); F2 MAJOR no M3 cycle-log (written, says honestly no separate red run); F3 MINOR T028 open until visual pass; F1 fix verified by the full gate rather than a second round (the gate runs `--lib`) |
| Visual pass M3 (quickstart B4–B6, headless `claude -p`; the skill's fork got no args) | 1 | — | PASS all 9 checks, light + dark, 640 px Kubernetes; D1 (select long value wraps and clips at 640 px) left as a follow-up; visual-pass/m3/ |
| Review B M3 round 2 (scoped, sonnet) | 2 | — | CLEAN (F1 lib test builds, F2 cycle-log has M3) |
| TDD verify (close, inline; smell pass by a fresh reviewer) | 1 | f3d8a05a | PASS_WITH_GAPS: F1 HIGH (lost discard-then-reopen assertion) fixed, F4 F7 F9 fixed, rest open as test-style follow-ups (tdd/verification.md); 3 deliberate mutants all killed; M2/M3 rows added to the test list |

## Declined review findings

- Tasks r1 F1 (MINOR, M1 is not the P1 story): FR-016 and the user's ask fix the three-PR order (component, sections, look); M1 has an observable showcase deliverable. Ledger D6.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

- Review A M1: title position differs between headers with and without a summary — declined, the two-line header is the design (research R4).
- Review A M1: `Mounted::painted_text` duplicates `tests/support/layout.rs::painted_text` — declined, in-crate tests cannot reach `tests/support` (`material` is `pub(crate)`; same reason `test_support.rs` exists).
- Review A M2 (MINOR): `section_open` calls `required_fields()` once per section per view — declined, six small calls per render, no measured cost.
- Review A M2 (MINOR): hand-written field→section table beside `FormError` strings — declined for this PR; `daemon_sections` tests pin every field name the form reads (T006).
- Review A M2 (MINOR): duplicated accordion push blocks in `form`/`kube_sections` — declined, each kind's controls differ; the list order comes from `Section::of`.
- Review A M2 (MINOR): context-list failure caution sits below *Context* — declined, B found it conforming (contract §Section bodies); doc-comment wording left as is.
- Review A M1: required pose also carries the summary — declined, matches how M3 sections pose both; FR-015 asks for "closed with the marker".
- Review A M1: marker hidden while open — declined, FR-012 and research R5 (the open field shows its own marker).

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Shared `Select` wraps and clips a long selected value at a narrow width (visual-pass/m3 D1, seen with a 48-character kube context); a daemon row's action row clips at 640 px for an Unreachable daemon (D2, untouched by 890).
- M1 visual pass: on an inert header (no `on_toggle`, the showcase's static poses) the title dims as a disabled button but the summary keeps `on_surface_variant`. Every daemon-form section has a toggle, so no form shows it; dim the summary with the header if an inert use appears.
