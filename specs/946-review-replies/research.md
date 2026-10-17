# Research: Review Comments the Agent Answers (#946)

Every decision builds on feature 482 (`specs/482-worktree-changes-review/`); its R-numbers are
cited as 482-R*n*. No new dependency.

## R1 — Where statuses and replies live

**Decision**: extend `micold_core::review::comment::CommentState` from `Pending | Sent { at }` to
`Draft | Sent { at } | Reported { sent_at, report }`, where `Report { outcome: Outcome, reply:
String, first_at, last_at, by: Reporter }`, `Outcome = Applied | Answered`, `Reporter = Agent {
session } | User`. All transitions are methods of `EntryReview` (pure, core), so the daemon stays
a single writer (482-R3) and every rule is unit-tested without a window.

**Rationale**: one enum makes "status" exactly one of four values (FR-001) and keeps the reply
impossible on a Draft or Sent comment. `Reported` carries `sent_at` so a re-report or a manual mark
loses nothing. Answered's non-empty reply is checked by the one constructor (`EntryReview::report`),
the only path that builds a `Report`.

**Alternatives**: a separate `replies` map keyed by comment id (two stores to keep consistent on
clear, delete and worktree removal); an `Option<Report>` beside `state` (allows a Draft with a
reply).

## R2 — Re-anchoring algorithm

**Decision**: a pure `review::anchor::locate(quote: &[String], last: LineRange, lines: &SideLines)
-> Located`:

1. Compare lines with any trailing `\r` stripped on both sides (FR-025).
2. If the lines at `last` equal the quote → `Located::Same(last)`.
3. Otherwise collect every start line `p` where the `n = quote.len()` lines from `p` equal the
   quote. None → `Located::Gone`. One nearest `last.start()` by `|p − last.start()|` →
   `Located::Moved(p..=p+n−1)`. Two at the same smallest distance (one above, one below) →
   `Located::Ambiguous`.
4. `Located::shown(len)` gives the place to draw: the range for Same/Moved; `last` clamped into
   `1..=max(len, 1)` for Gone/Ambiguous (FR-021 "within the file's length").

Old-side comments are never relocated (FR-022): their outdated test stays 482's
`ReviewComment::is_outdated`.

Cost is O(lines × n) worst case with a first-line prefilter; the client runs it only for the shown
file's comments, the daemon only for drafts at send, so 200 comments stay well under SC-006's
100 ms.

**Rationale**: exact block match is predictable and testable byte for byte (SC-004: never placed
on lines that do not hold the quote without the outdated mark). "Nearest, tie → outdated" is the
spec's own rule (US4 s4).

**Alternatives**: diff-hunk line mapping (follows a comment through an edit to its own lines,
which the spec wants marked outdated); fuzzy matching (places comments on lines that do not hold
the quote, violating SC-004).

## R3 — Who re-anchors, and what is stored

**Decision**: the **range** a comment holds is its *last known place* and is stored; the
**outdated mark** stays derived and display-only (482-R14).

- **Client** (display): after every diff read of the shown file, `features::changes` computes a
  `Located` per comment of that file on the new side, draws cards at `shown`, and marks outdated
  only Draft or Sent comments that are Gone or Ambiguous (FR-024). When some comments `Moved` and
  the shown new side is the working tree (Default entry, or the Uncommitted toggle on, and not a
  pull request's diff), it sends one `ReviewEditOp::Reanchor { moves }`.
- **Daemon** (send): `review_send` re-anchors every Draft current-side comment against the
  entry's working tree before building the batch, so a file the user never reopened still names
  its new lines (US4 s1, FR-023), and judges those comments' outdated note itself. Files are read
  on the blocking pool, outside the state lock; old-side comments keep the client's `outdated`
  ids, as in 482.
- `Reanchor` is compare-and-set: each move names `from` and `to`; the daemon applies a move only
  while the comment's range still equals `from` and `to` has the quote's length. Two windows that
  send the same moves apply them once; nothing applied means no write and no push.

**Rationale**: storing the last place makes restarts and other windows draw the comment where it
was last seen; deriving the mark keeps 482's property that every window reaches the same verdict
from the same files without agreeing on writes. Only the working tree is what the agent edits and
what the batch's line numbers refer to, so only it moves a stored range.

**Alternatives**: daemon-only re-anchoring with a file watcher (a second watcher beside the
client's 482-R9 one, for no gain); client-only (a file the user never opens keeps stale lines in
the batch); storing the outdated flag (needs every window to agree when to write it).

## R4 — Comment reference

**Decision**: `CommentRef` = `C-` + the first 8 lowercase hex digits of the comment's UUID
(`C-1a2b3c4d`, 10 characters, FR-014). Derived, never stored. `EntryReview::add` refuses an id
whose reference another comment of the entry already has; the daemon draws a new UUID until it is
free. On load, a stored comment whose reference collides with an earlier one in the same entry
gets a fresh id (only possible for comments written before this feature).

**Rationale**: derived from an id that never changes, so stable for the comment's life with no
counter to persist; random, so a reference of a cleared comment is not handed to a new one (Edge
Case "report for a cleared comment"); short enough to quote.

**Alternatives**: a per-entry counter (`R1`, `R2`): must be persisted even after the entry empties,
or a stale report names a reused number; the full UUID (36 characters, over FR-014).

## R5 — The report tool

**Decision**: a new shipped MCP tool `report_review_comment` on the daemon's agent tool server
(feature 034): arguments `comment` (reference, required), `outcome` (`applied` | `answered`,
required), `reply` (string, optional; required non-blank for `answered`). Not read-only, not
destructive, so it is audited (034 FR-018, target = the reference, never the reply) and policy
`Proceed`s it with no confirmation (FR-003). It acts only on the caller's own entry: the caller's
`SessionLocation` (worktree dir, or `""` for Default) picks the entry (FR-004, D5). A Default
caller is allowed: a report changes no worktree (Principle III untouched).

Order of checks: argument validation (`invalid_input`: unknown outcome, blank reply for answered,
reply over 20,000 characters) → lookup in the caller's entry; if absent, in the project's other
entries (`refused_by_policy`: "belongs to another entry") or nowhere (`not_found`) → state
(`conflict`: "still a draft", "being sent; not delivered yet"). Success writes the review file,
then pushes `ReviewChanged` to every window (FR-007's 2 s holds: the push is immediate).

**Rationale**: the tool server already knows the calling session and its worktree (`whoami`), so
FR-004's isolation is the server's own lookup, not trust in the agent's arguments.

**Alternatives**: parsing replies out of the terminal transcript (unreliable, CLI-specific); a
CLI hook (per-CLI, not every CLI has hooks).

## R6 — The batch format

**Decision**: `review::prompt::build` is replaced by a fixed preamble plus one self-contained
block per Draft comment (contracts/batch-format.md), each naming its reference, file, side and
lines, an outdated note line when applicable, the fenced quote (482's elision over 50 lines and
fence-longer-than-any-backtick-run rule) and the text. 482's `## <path>` group headings go: a block
must stand on its own (US2 s2). Order is 482's: path, start line, side, created, id. Output has no
`\r` and no trailing newline (the submission path presses Enter, 482).

The golden test in `review/prompt.rs` pins the exact bytes of contracts/batch-format.md's example;
the user guide carries the same example. FR-013's "change only together" is kept by that test's
doc comment and the review gate: a core test may not read `docs/` (constitution, TDD-gate
exemption check `documentation_is_not_read.rs`).

**Alternatives**: JSON blocks (harder for the user to read in the terminal; the agent reads
markdown fine); keeping 482's grouped layout (blocks not self-contained).

## R7 — Store and wire versions

**Decision**: the review file goes to `version: 2`; the reader accepts 1 and 2 (`pending` →
Draft, `sent` → Sent). An older build that meets version 2 quarantines the file (482's
`read_or_quarantine`), so nothing is overwritten silently. `PROTOCOL_VERSION` 48 → 49:
`ReviewComment`'s state shape on the postcard wire changes and `ReviewEditOp` gains `Reanchor`
and `Mark`.

## R8 — The review panel

**Decision**: a third pane of the Changes view, right of the diff, open by default and collapsible
from the toolbar ("Review (n)"); rows from a new shared component
`ui::material::ReviewPanelRow` (builder, Principle VIII), fixed height so the list uses the
existing `virtual_rows` for 200 comments (SC-006). Row order, grouping, status counts, what a row
can do and where selecting it jumps are decided in render-free `features::changes` (testable);
`ui/changes.rs` only renders. The card component `ReviewCommentCard` gains the Applied and
Answered states, `.reply(..)`, `.highlighted(..)` and the manual-mark actions. A full reply opens
in the shared dialog. Replies are rendered as plain text through one core helper that keeps `\n`
and replaces other control characters (Edge Case).

**Alternatives**: a modal review list (hides the diff it jumps to); a per-file list (482 already
shows comments per file).

## R9 — Marking by hand

**Decision**: `ReviewEditOp::Mark { id, outcome, note }` on a Sent comment (FR-006): Applied with
an optional note, Answered with a required note, `by: User`. A comment already reported can be
re-marked by the agent's later report (last wins, FR-005), not by the user (FR-006 names Sent).
The composer gains a target `Answer(CommentId)` for the note, so no new editor widget.
