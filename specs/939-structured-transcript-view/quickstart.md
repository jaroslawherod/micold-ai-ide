# Quickstart: validating the Structured Session Transcript View (#939)

## Part A — automated

| Check | Command | Expect |
|---|---|---|
| Transcript model against the fixtures | `cargo test -p micold-core --test structured_transcript` (and `scripts/gate.sh`) | G1–G5 of [contracts/transcript-model.md](contracts/transcript-model.md): every fixture event accounted for, one card per tool use with its result, streamed == unstreamed, stable keys, bound, sub-agent counts (SC-001, SC-002) |
| Summaries and footer figures | `cargo test -p micold-core --lib structured::transcript` | FR-009 rules per tool, generic summary, one line; FR-005 formats, omitted figures left out |
| View state | `cargo test -p micold-client --test features_structured_transcript` | fold toggling and persistence across events, rebuild and session switch; follow-tail at end / away / back / jump; per-session isolation (FR-010, FR-013, FR-017–FR-019, FR-021, SC-003) |
| Shared components | `cargo test -p micold-client --test showcase_completeness --test icons` and the builder gate | `DisclosureCard` has a showcase entry and a builder; new icons pinned |
| Open and stream budget | `cargo test --release -p micold-client --test transcript_open_budget -- --ignored` | 5,000 events, every card open: first layout + draw < 1 s; each streamed update < 100 ms (SC-004) |
| Terminal sessions unchanged | `scripts/gate.sh` | every existing test green (SC-006) |

Entities: [data-model.md](data-model.md). Seam: [contracts/block-renderers.md](contracts/block-renderers.md).

## Part B — manual / visual pass (GUI exception)

Prerequisite: a logged-in `claude` on `PATH`; the dev client against a project with a structured
session (start-session chevron → **Claude Code (structured)**).

1. Start a structured session with a prompt that reads a file and answers: the pane shows a
   session-start line (model, folder), the reply text growing as it streams, a folded `Read` card
   with the file path as its summary turning from running to done, and a footer with duration,
   the session's running cost ("session total $…") and turns. No plain-log line.
2. Unfold and fold the card with the pointer, then with Tab and Space/Enter.
3. A turn that fails (`--max-turns 1` through a tool-heavy prompt, or a tool that errors): the card
   and footer show "Failed" with the error icon; check a greyscale screenshot (SC-005).
4. While a reply streams, scroll up: the visible text does not move and "Jump to latest" appears;
   press it: the view follows again and the control hides. Scroll back to the end by wheel,
   trackpad and scrollbar drag: following resumes.
5. Switch to another session and back, and view the session in a second window: unfolded cards
   stay unfolded in the first window; the second window shows the same entries with its own fold
   and scroll state. Tile two structured sessions side by side: no mixing.
6. A terminal session's pane is unchanged.
7. Light and dark theme: every entry kind, the failed card and footer, raw rows and the control.

Record the pass with the `visual-pass` skill into `visual-pass/`.
