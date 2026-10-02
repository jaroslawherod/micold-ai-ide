# Evidence: 037 visual pass, T017 (B1-B8, B14) and the M4 full pass (T036, T037, last section)

Date 2026-10-01. Ran on Xvfb :87 + lavapipe (software Vulkan), not a real display; private HOME, XDG dirs and PATH (stub claude/copilot, pi only via the include script). Binaries built from HEAD b205f81d and pinned in ~/vp037/bin. Window 1200x900; crops show Settings > Environment from the Default AI CLI field down.
Geometry (all rows): note's left edge aligns with the select's text inset, wraps inside the column, ends before the select's right edge, no overlap with the next control. Legible in both themes.
B9-B13 were run with their milestones (rows below). Mid-flight animation not covered.

| Step | Seed | Sentence seen | Result | Screenshot |
|---|---|---|---|---|
| B1 light | false, env.sh (Pi only via script) | A session would not find Pi Coding Agent: sessions get only the login PATH, because "Source a script before each session" is off. Turn it on if your startup file puts it on the PATH, or install it on the login PATH. (selector lists Claude Code, GitHub Copilot only) | PASS | b1-light.png |
| B1 dark | false, env.sh (Pi only via script) | A session would not find Pi Coding Agent: sessions get only the login PATH, because "Source a script before each session" is off. Turn it on if your startup file puts it on the PATH, or install it on the login PATH. (selector lists Claude Code, GitHub Copilot only) | PASS | b1-dark.png |
| B2 light | true, env.sh (light: ticked in UI, saved, Settings reopened; dark: seeded) | no note; selector lists Pi Coding Agent | PASS | b2-light.png |
| B2 dark | true, env.sh (light: ticked in UI, saved, Settings reopened; dark: seeded) | no note; selector lists Pi Coding Agent | PASS | b2-dark.png |
| B3 light | true, empty path | A session would not find Pi Coding Agent: no script is sourced, because "Script path" is empty. Set "Script path" if a startup file puts it on the PATH, or install it on the login PATH. | PASS | b3-light.png |
| B3 dark | true, empty path | A session would not find Pi Coding Agent: no script is sourced, because "Script path" is empty. Set "Script path" if a startup file puts it on the PATH, or install it on the login PATH. | PASS | b3-dark.png |
| B4 light | true, /tmp/does-not-exist.sh | A session would not find Pi Coding Agent: the startup script was not found for your home directory, so its PATH additions are not applied. Correct "Script path". | PASS | b4-light.png |
| B4 dark | true, /tmp/does-not-exist.sh | A session would not find Pi Coding Agent: the startup script was not found for your home directory, so its PATH additions are not applied. Correct "Script path". | PASS | b4-dark.png |
| B5 light | true, exit 3 | A session would not find Pi Coding Agent: the startup script exited with an error for your home directory, so its PATH additions are not applied. Fix the script named in "Script path". | PASS | b5-light.png |
| B5 dark | true, exit 3 | A session would not find Pi Coding Agent: the startup script exited with an error for your home directory, so its PATH additions are not applied. Fix the script named in "Script path". | PASS | b5-dark.png |
| B6 light | true, sleep 30, timeout 1 | A session would not find Pi Coding Agent: the startup script timed out for your home directory, so its PATH additions are not applied. Fix the script named in "Script path", or raise "Timeout". | PASS | b6-light.png |
| B6 dark | true, sleep 30, timeout 1 | A session would not find Pi Coding Agent: the startup script timed out for your home directory, so its PATH additions are not applied. Fix the script named in "Script path", or raise "Timeout". | PASS | b6-dark.png |
| B7 light | true, script adds nothing | Pi Coding Agent was not found on the PATH sessions get for your home directory: the login PATH plus what the startup script adds. Install it, or make the script add its directory. | PASS | b7-light.png |
| B7 dark | true, script adds nothing | Pi Coding Agent was not found on the PATH sessions get for your home directory: the login PATH plus what the startup script adds. Install it, or make the script add its directory. | PASS | b7-dark.png |
| B8 light | claude, copilot, pi all on login PATH | no note | PASS | b8-light.png |
| B8 dark | claude, copilot, pi all on login PATH | no note | PASS | b8-dark.png |
| B14 | container image | not run | covered by Part A | `missing_cli_is_reported_where_it_is_chosen.rs` asserts both image rows. `mise run image` would replace the shared `micold-daemon:dev` tag other worktrees use, so no image was built (quickstart lines 57-60). |
| B9 light | false, env.sh (pi only via script), default Pi, project open; pressed + on the Default row (2026-10-02, Xvfb :137 + lavapipe, HEAD 1e58735e build) | A session would not find Pi Coding Agent: sessions get only the login PATH, because "Source a script before each session" is off. Turn it on if your startup file puts it on the PATH, or install it on the login PATH. Or start this session on another AI CLI. (menu listed Claude Code only, no session started, no "isn't installed") | PASS | b9-light.png |
| B10 banner light | true, env.sh, Pi session running; Settings saved with sleep.sh, timeout 1; stand-in killed (3 auto-restarts, crash loop); pressed restart | A session would not find Pi Coding Agent: the startup script timed out for /home/jaro/vp037b/s/proj, so its PATH additions are not applied. Fix the script named in "Script path", or raise "Timeout". Then restart this session: its conversation can only continue in Pi Coding Agent. (one banner; Resume ending; no "install", no "another AI CLI") | PASS | b10-banner-light.png |
| B10 pane light | same as above | the pane kept the session's terminal (an empty grid with a cursor) and the bar read "failed restart"; the sentence was in the banner only. The pane says a failure in words only when it has no terminal to keep (`ui/terminal.rs`, `empty_terminal_message`), which predates 037 and is where the same service sentence is shown verbatim. Reproduced twice. | NOT SHOWN (ledger D20) | b10-pane-light.png |
| B11 light | real client: include on, script `exit3.sh` (`exit 3`), default Claude Code, stub `claude` and `copilot` on PATH, Pi on neither; one project at a long path; pressed the Default row's chevron (2026-10-02, Xvfb :151 + lavapipe, HEAD 5662df9b build, binaries pinned in ~/vp037c/bin) | Claude Code, GitHub Copilot, a divider, then: A session would not find Pi Coding Agent: the startup script exited with an error for /home/jaro/vp037c/s/proj-with-a-rather-long-directory-name-to-force-wrapping/subdir-number-one/subdir-number-two, so its PATH additions are not applied. Fix the script named in "Script path". The note wraps inside the panel and the path breaks at its slashes and hyphens. Clicking the note left the list open and started nothing; clicking outside dismissed it | PASS | b11-client-light.png |
| B11 dark | same as above | the same list, sentence and wrapping | PASS | b11-client-dark.png |
| B11 lowest row, dark | same seed, window shrunk to 1200x300 so the panel does not fit under the row (one row, not several seeded projects) | the panel moved up and ends at the window's bottom edge with the note's last line visible; its border is flush on the edge, with no margin | PASS | b11-client-lowest-row.png |
| B11 showcase light | component showcase, Floating, `MenuOverlay`, "Open a start list with a note"; sample directory `/home/dev/projects/atlas/.worktrees/feat-availability-notes` (Xvfb :83) | two items, a divider, the note in 6 lines; the path breaks after `/home/dev/projects/`; nothing crosses the panel's edge; no hover state on the note, a click on it leaves the menu open | PASS | b11-light.png |
| B11 showcase dark | same as above | same | PASS | b11-dark.png |
| B11 showcase short window, dark | showcase window shrunk to 1600x230 (taken with the earlier, short sample path) | the panel stays inside the window and scrolls; the note is reached by scrolling | PASS | b11-clamp-dark.png |
| B12 dark | real client, only stub `claude` on PATH | the Default row's start action is the "+" alone: no chevron, nothing new | PASS | b12-client.png |
| B13 light | showcase, "Open the menu panel" (a menu without a note) | Copy name, Rename, Delete; no divider, no note | PASS | b13-light.png |
| B13 dark | same as above | same | PASS | b13-dark.png |

## T036 and T037: the full pass on the merged result (M4, 2026-10-02, HEAD bc569992)

### Part A

| Command | Result |
|---|---|
| `mise run gate` (fmt, clippy core and workspace, `cargo test --workspace`, `scripts/tests/*.test.sh`) on bc569992 | PASS, exit 0. `cargo test --workspace` covers the core crate; `mise run test-core` was not run on its own. |

The Linux, macOS and Windows jobs on CI hold "each platform" for the core and service tests.

### Part B, every step

| Step | Recorded in | Result |
|---|---|---|
| B1-B8, both themes | the table above (M1, T017) | PASS |
| B9 | B9 light row (M2) | PASS |
| B10 | B10 banner and pane rows (M2, D20), and the row below | PASS |
| B11 | B11 rows (M3): real client both themes, lowest row, showcase, short window | PASS |
| B12 | B12 row (M3) | PASS |
| B13 | B13 showcase rows (M3), and the real-client rows below | PASS |
| B14 | B14 row: container image not built, covered by Part A | covered by Part A |

Run at M4 (Xvfb :241 + lavapipe, private HOME, XDG dirs and PATH, binaries built from bc569992):

| Step | Seed | Sentence or result seen | Result | Screenshot |
|---|---|---|---|---|
| B10 pane without a terminal, light (D20) | include on, `env.sh` puts a stub `pi` on the PATH, a Pi session with a recorded conversation; client and service quit, `env_include_enabled` set to false, client started again so the session fails to start | the pane has no terminal grid and says: "A session would not find Pi Coding Agent: sessions get only the login PATH, because "Source a script before each session" is off. Turn it on if your startup file puts it on the PATH, or install it on the login PATH. Then restart this session: its conversation can only continue in Pi Coding Agent." The bar reads "failed restart". No banner showed: the sentence is in the pane only. | PASS (W3 Resume, IncludeOff) | b10-noterm-light.png (a text crop: the missing grid, the bar and the absent banner were seen on the full window and are not in the crop) |
| B13 real client, light | stub `claude`, `copilot` and `pi` all on the login PATH, one project, default Claude Code; pressed the Default row's chevron | three CLIs, no divider, no note | PASS | b13-client-light.png |
| B13 real client, dark | same | same | PASS | b13-client-dark.png |

With B10 banner (restart in place: the banner carries the sentence) and B10 pane (no terminal: the pane carries it), each place the service's failure text is shown has been seen once.

### Wording cross-check (T037)

Compared by a fresh reader, word for word with W1's placeholders expanded:

- `contracts/reason-wording.md` W2 (all seven rows: `IncludeOff`, `NoScriptPath`, `ScriptNotFound`, `ScriptFailed`, `ScriptTimedOut`, `Applied` host, `Applied` image, with the "were", "aren't" and "their directories" plurals) equals the strings in `crates/micold-core/src/cli_reason.rs`.
- W3 (`Fresh` and `Resume`, the generic rows and the `Applied`/image rows) equals them, and so does `start_refusal_unknown` against W5's R8 sentence.
- The user guide pages: `settings.md` (lines 74-82) paraphrases each state in a table (bold labels, "the CLI" for the name, "Fix the script") and contradicts no sentence; `agent-tools.md` (106-108), `worktrees-and-sessions.md` (688-690, 787-803) and `sandboxed-daemon.md` (104-107) quote only setting labels and causes that match. None says "not installed", "the image lacks it" or install as the only action for a state where the script was not applied (SC-003).
- No difference found; the contract and the guide are unchanged.
