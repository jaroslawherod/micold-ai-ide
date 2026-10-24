# Quickstart: validating #1128

Part A is mechanical and runs in any checkout; Part B is the visual pass. Contracts:
[doc-edits](contracts/doc-edits.md), [covered-states](contracts/covered-states.md),
[visual-pass-record](contracts/visual-pass-record.md).

## Prerequisites

- A checkout of this branch rebased on main; `mise trust` once.
- Stage 2 parts (A2 stage 2 rows, A3, B): #1125 and #1127 merged on main.
- `lychee` for A1 (`cargo install lychee`), or CI's run of the same check.

## Part A

### A1 — Links and anchors (FR-019, SC-003)

```sh
site/checks/links.sh --sources
```

Expected: exit 0, no broken link or fragment in `docs/`.

### A2 — No stale mention (SC-001, SC-002, FR-003)

Run each pattern of [doc-edits §Patterns](contracts/doc-edits.md#patterns) for the stage over
`docs/user-guide/*.md site/media.toml`. Expected: every hit is an owning section, a mention that
links to its owning section, or a sentence that says the element is gone. Stage 1: no hit for the
bar under the terminal, and every "bar" in the four pages names which bar.

### A3 — Whole-window states (stage 2; FR-012, FR-013, SC-004, SC-005)

```sh
mise exec -- cargo test -p micold-client --test layout_snapshot
mise exec -- cargo test -p micold-client --test layout_text_overflow
```

Expected: both pass; `window-session-view` and `window-overview` are in
`tests/fixtures/layout_snapshot.txt`; gate W1–W5 pass in both schemes. Then a regeneration
(`UPDATE_LAYOUT_SNAPSHOT=1` on the first command) leaves `git diff --exit-code
crates/micold-client/tests/fixtures/layout_snapshot.txt` clean.

Mutation proof: apply each edit of [covered-states §Mutation proof](contracts/covered-states.md#mutation-proof-quickstart-a3)
in turn, run the first command, confirm the failure names the state (and the W assertion listed),
revert. Record the four outcomes in the PR description.

### A4 — Whole gate

```sh
mise run gate
```

Expected: green. M1 is docs only, so CI skips the suite for it (`.gitattributes` `micold-docs`);
A1 still runs in CI.

## Part B — Visual pass (stage 2; FR-015, FR-016, SC-006)

1. Draw `window-session-view` and `window-overview` at 1280×800, light and dark, through
   `ui::view` and the tiny-skia renderer (research R8), from a scratchpad harness; or run the
   `visual-pass` skill on Xvfb with the same states. Save the four PNGs under
   `specs/1128-layout-redesign-docs/visual-pass/`.
2. Open each beside its mockup board. Record five verdicts per frame in `visual-pass.md` per the
   record contract, with the commit.
3. Per region and theme, cite the contrast test or measure the ratio with
   `micold_core::tokens::contrast` (research R9). Under 4.5:1 for body text is a defect.
4. Each *differs by defect* → a ledger follow-up; no code change.

Expected: 4 screenshots, 20 verdicts, each non-*matches* verdict with its reason.
