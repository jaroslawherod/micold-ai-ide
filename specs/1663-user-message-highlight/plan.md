# Implementation Plan: Highlighted user messages in the structured view

**Branch**: `feat/1663_structured-view-messages-sent-by-the-user-are-not` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1663-user-message-highlight/spec.md`

## Summary

A user turn (typed prompt, delivered queued message, slash command card) is wrapped in a new shared
`UserTurn` primitive in `micold-material`: a `surface_container_high` fill with a 4dp `primary` left
edge, at the small corner. `ui/transcript/entries.rs` wraps the two `EntryKind::UserMessage`
branches in it. The wrap changes drawing only: selection paths (`vec![0]`, `vec![3]`), entry set,
order and scroll are untouched. Token choice and the contrast proof are in [research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain), iced

**Primary Dependencies**: `micold-material` (`style`, `Divider`/`DisclosureCard` patterns), `micold-core::tokens` (`Roles`, `contrast`, `AA_TEXT`)

**Storage**: N/A (no state, nothing persisted)

**Testing**: `cargo test` via `scripts/build-lock.sh`; `mise run test-core` for the token test; client view tests; quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows (theme tokens only)

**Project Type**: desktop app (iced), Cargo workspace

**Performance Goals**: no change; one extra container per user turn, no per-frame work

**Constraints**: no new fixed colours (FR-006); no change to selection units or copied text (FR-008)

**Scale/Scope**: one new primitive (~80 lines), two call-site edits, one style fn, one doc page edit

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | PASS: tests listed below precede each code unit; tasks order red before green |
| II. Multi-Session | PASS: stateless view code, nothing shared between sessions |
| III. Worktree Integration | PASS: not applicable, no file or VCS operation |
| IV. Local-First Storage | PASS: no storage, no network |
| V. Rust + iced | PASS: Rust and iced only |
| VI. Cross-Platform Parity | PASS: tokens only, no platform font or shadow (no shadow is drawn) |
| VII. Documentation | PASS: `docs/user-guide/structured-transcript.md` gains a short "Your messages" note in the same change |
| VIII. Reusable UI Foundation | PASS: `UserTurn` is a shared `micold-material` primitive with a builder (`UserTurn::new(content, roles)`, `.into()`), not a transcript-local one-off |

Post-design re-check: unchanged, all PASS. Complexity Tracking: none.

## Project Structure

### Documentation

```text
specs/1663-user-message-highlight/
  spec.md plan.md research.md data-model.md quickstart.md
  contracts/user-turn.md
```

### Source (new marked)

```text
crates/micold-material/src/user_turn.rs        # NEW: UserTurn builder -> Element
crates/micold-material/src/style.rs            # + user_turn_surface(r), user_turn_edge(r)
crates/micold-material/src/lib.rs              # + pub mod user_turn; pub use UserTurn
crates/micold-client/src/ui/transcript/entries.rs  # wrap both UserMessage branches
crates/micold-core/tests/tokens.rs             # + contrast/difference test for the pair
crates/micold-client/tests/transcript_entry_view.rs # + highlight present/absent tests
docs/user-guide/structured-transcript.md       # + note
```

## Requirement mapping

| Req | Where |
|---|---|
| FR-001, FR-001a | `style::user_turn_surface` = `surface_container_high`; token test asserts it differs from `background` and from `surface_container_low` (tool cards), both schemes |
| FR-002 | `UserTurn` left edge, `primary`, 4dp |
| FR-003 | wrap at the single `EntryKind::UserMessage` arm (both the command and plain branch); main and sub-agent scopes share that arm |
| FR-004 | wrap includes caption, body or card, attachments line (the whole `column`) |
| FR-005 | no other arm touched; view test asserts assistant/thinking/tool/command-output entries are not wrapped |
| FR-006 | only `Roles` fields; no `Color::from_rgb` literal; `Roles` follows scheme and accent live |
| FR-007, SC-002 | token test: `contrast(on_surface, surface_container_high) >= AA_TEXT` and for `on_surface_variant` (caption), both schemes |
| FR-008, SC-003 | selection paths unchanged; existing `features_transcript_selection.rs` runs unchanged; the wrap adds no text widget |
| FR-009 | the `DisclosureCard` is moved inside the wrapper unchanged, toggle message unchanged |
| FR-010 | no change to `Transcript` entry iteration or scroll code |
| FR-011 | quickstart §B visual pass, light and dark, via `visual-pass` |
| Edge: empty text, long text, narrow | the container is `Length::Fill` wide with the body wrapping as today; covered by a view test with empty text and by the visual pass |
| Edge: back-to-back turns | outer entry spacing already separates entries; checked in visual pass; add `spacing::XS` bottom padding only if the pass shows touching blocks |
| SC-005 | geometry gates run unchanged; any height delta is the padding of the wrap, updated in the gates' expected values only if they pin a user turn height |

## Test strategy

- Core unit (`micold-core/tests/tokens.rs`): difference and AA contrast of the chosen pairs, light and dark. FR-001a, FR-007.
- Material unit (`micold-material`): `UserTurn` builds with both schemes; style fns return the expected tokens and no shadow. FR-001, FR-002, FR-006.
- Client view (`transcript_entry_view.rs`): a typed, a queued-delivered (same `UserMessage` kind) and a command entry render inside the highlight; assistant text, thinking, tool and command output do not (FR-003/4/5, SC-004); empty text renders.
- Selection (`features_transcript_selection.rs`): run unchanged (FR-008, SC-003).
- Geometry gates: run unchanged (SC-005).
- Visual (quickstart §B): FR-011, SC-001.
