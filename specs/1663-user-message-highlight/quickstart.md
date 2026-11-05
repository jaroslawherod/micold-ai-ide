# Quickstart: validating #1663

## Part A: automated
1. `mise run test-core` — token difference/contrast test passes.
2. `scripts/build-lock.sh cargo test -p micold-material user_turn` and `-p micold-client --test transcript_entry_view` — highlight present on user turns, absent elsewhere.
3. `scripts/build-lock.sh cargo test -p micold-client --test features_transcript_selection` — unchanged, passes (FR-008).
4. `mise run gate` before pushing.

## Part B: visual pass (FR-011, SC-001)
Run the `visual-pass` skill, light then dark, on a transcript holding: a typed prompt, a queued message delivered, a slash command (folded and unfolded), assistant text, thinking, a tool card, an empty-text prompt, a long wrapped prompt, a narrow pane, two user turns back to back, and a sub-agent scope. Expect: every user turn has the tinted surface and left accent; nothing else does; text legible; selection colour visible on the tint. Record screenshots and the verdict in the feature directory.
