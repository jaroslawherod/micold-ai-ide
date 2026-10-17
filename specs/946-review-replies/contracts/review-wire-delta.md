# Contract: review wire and store delta (amends 482's contracts/review-wire.md)

482's W1–W12 stay unless amended here. `PROTOCOL_VERSION` 48 → 49 (one bump for everything
below).

## W13 — Comment state on the wire

`ReviewComment.state` is the new `CommentState` (`Draft | Sent { at } | Reported { sent_at,
report }`, data-model.md). Every `ReviewChanged` carries it.

## W14 — `ReviewEditOp::Mark { id, outcome, note }`

The user marks a Sent comment by hand (FR-006). `note` trimmed and `\r`-normalised; required for
`answered`; ≤ 20,000 characters. Refusals: `NotFound`; `InSend` → `Busy`; not Sent → `Refused`;
blank note for answered or over limit → `InvalidInput`. Stored then pushed (W5).

## W15 — `ReviewEditOp::Reanchor { moves }`

Moves `{ id, from: (start, end), to: (start, end) }`. Each move applies only while the comment's
range equals `from` and `to` covers as many lines as the quote; others are skipped, not refused.
`Ack` always (after the write when at least one applied); no write and no push when none applied.
Never changes `quote`, `text` or `state` (FR-023). Allowed during an open send (the snapshot's
batch is already built).

## W16 — `ReviewSend` re-anchors

Before opening the send, the daemon reads, off the state lock, the working-tree file of every Draft
current-side comment of the entry (`SideLines::from_bytes`; missing or non-text → gone), applies
`review::anchor::locate`, stores any moves (as W15), and judges those comments' outdated note
itself. The client's `outdated` ids are used only for removed-side comments.

## W17 — Agent reports

`report_review_comment` (contracts/agent-tool.md) changes the caller's entry exactly like an edit:
written first, then `ReviewChanged` pushed to every attached client (FR-007).

## S2 — Review file version 2

`reviews/<project id>.json` `version: 2`; state spelled as in data-model.md. Version 1 loads
(`pending` → Draft); the next write is version 2. Unknown versions quarantine as 482. On load, a
comment whose reference collides with an earlier one of the same entry gets a fresh id (research
R4).

## Tests

`crates/micold-core/tests/protocol_roundtrip.rs`: the new ops and states round-trip.
`review/store.rs` units: v1 file loads with Draft/Sent; v2 round-trips; collision on load re-ids.
`crates/micold-daemon/tests/review_edit.rs`: Mark (each refusal, success pushes), Reanchor
(compare-and-set, idempotent across two clients, no push when nothing applied).
`crates/micold-daemon/tests/review_send.rs`: a draft whose lines moved on disk is sent with its new
lines and stored there; a draft whose code is gone gets the outdated note.
