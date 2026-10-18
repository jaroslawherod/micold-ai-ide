# Quickstart §B and §C results (813, close)

Date: 2026-10-18. The cloud container has no display tooling (no xdotool, no sway, no Vulkan ICD),
so the pass was not re-run at close; this file collects what each milestone recorded.

| Check | Result | Evidence |
|---|---|---|
| B1, B15 (states, grid, greyscale, narrow) | PASS (type word only in tooltip was FR-002 FAIL, fixed test-first) | m1/notes.md |
| B4, B5 (PR counts, conflict) | PASS; showcase lacks a PR-off card | m2/notes.md |
| B6, B7 (badge, shortcut) | not run; geometry pinned by layout snapshot, toolbar mapping unit-tested | m3/notes.md |
| B2, B3 (task summary, progress) | not run | ../visual-pass-m5.md |
| B8, B9 (traversal, activation) | not run; logic covered by client tests | ledger, M4 |
| B10 (hook-fed activity) | not run; daemon hook-activity tests and card-fit test cover it | ledger, M6 |
| B11–B14, B16 | not run (need real client, daemon, fixture project) | n/a |

**Open for a human at a display**: B2–B3, B6–B14, B16.

## §C — macOS and Windows

No macOS or Windows machine here. CI covers them only in part: job `test` runs on
`ubuntu-latest, macos-latest, windows-latest`, but on macOS and Windows it runs `cargo test -p
micold-core --all-targets` and the enumerated gate `--test` list; the full workspace run (the
Overview client tests, keymap, layout snapshots) is Linux only (`ci.yml`, "Test (full workspace)",
`runner.os == 'Linux'`). The macOS bundle job and the Windows installer smoke jobs (x64, ARM64)
build and launch the packaged app. The Cmd+Shift+O mapping on macOS and the Overview on a real
macOS and Windows desktop (B1, B7, B15) are not exercised by CI: a human pass (T071b).
