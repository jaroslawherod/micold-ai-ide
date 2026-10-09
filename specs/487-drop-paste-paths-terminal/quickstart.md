# Quickstart: 487 validation

## Part A — automated
`mise run test-core` (quoting, plan, layout), `mise run gate` (client, daemon, round-trip).
Expect: SC-003 table green for every `ShellKind`; no test sees `\n`/`\r` outside bracket (SC-004).

## Part B — visual pass (`visual-pass` skill, private Xvfb, `xdotool`)
1. Open a regular terminal; drag two files (one `my file's (1).png`) from a file manager window → both quoted paths at the prompt, nothing executed; press Enter and check `ls` lists them.
2. Split panes (484); drop on the unfocused pane → only it receives.
3. Put a screenshot on the clipboard (`xclip -selection clipboard -t image/png -i f.png`), paste into an AI session → `.micold-pasted/<id>/*.png` path appears; `git status` clean; file opens as the image.
4. Copy text and image together → text pastes, no file.
5. Sandboxed session: drop a project file → `/…` container path; drop `~/outside.txt` → notice naming it, nothing inserted.
6. Delete the session → `.micold-pasted/<id>` gone, dropped files untouched; kill the app, leave a stray `pasted/<dead-id>`, restart → removed.
