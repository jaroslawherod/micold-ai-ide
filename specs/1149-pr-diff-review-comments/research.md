# Research: Review comments on a pull request's diff (#1149)

## R1. Where the pull request lives on a comment

**Decision**: `ReviewComment` gains `pr: Option<PrAnchor>`; `PrAnchor { number: u64, head_branch:
String, head_commit: String }`. `Scope::{WorkingTree, PullRequest(u64)}` is derived by
`ReviewComment::scope()`. Comments stay in the entry's one list, keyed by worktree directory.

**Rationale**: the entry key is what 482 FR-020/021, the MCP report and forget-on-delete already
use; a second key level would touch all of them. An `Option` field cannot hold a comment that is
both scopes (Principle V). `head_commit` is part of the comment because FR-002 anchors it, and it
never changes after creation, like `quote` and `text`.

**Rejected**: a separate PR-comment type and list (duplicates `EntryReview` rules, reports and
the panel); a `scope` stored apart from the anchor (two sources of truth); keying by (entry, PR)
in the store (breaks `forget_worktree` and the one-open-send-per-entry rule of FR-010).

## R2. Operations that act on "the drafts"

**Decision**: `begin_send`, `clear_sent` and `discard_pending` take a `Scope` and touch only
comments of it. The wire carries the scope: `ClearSent { scope }`, `DiscardPending { scope }`,
and `ReviewSend { pr: Option<PrSend> }` (`None` is the working tree). One send is open per entry
whatever its scope (FR-010 says in every window; a second send of another scope is `Busy`).

**Rationale**: FR-006 and FR-015 need per-scope acts; one open send per entry keeps the 482
`sending` marker and its tests valid. A user who presses Send on a second pull request while the
first is in flight sees the existing "already being sent" refusal.

**Rejected**: one open send per scope (`sending` becomes a map; two terminals typed to at once
interleave in one session; no gain).

## R3. The head commit of the diff shown

**Decision**: `PullRequestView.head` is the listed `headRefOid` at the moment the diff read
starts, and is the `head_commit` of comments written on it. When a list read shows a new `head`
for the shown number, the client re-reads the diff keeping the selected file, and sets `head` when
the new diff is ready. Comments keep their own `head_commit`.

**Rationale**: the list already carries `head` (775/1003); no new GitHub call. FR-012 needs the
diff to follow the head. Known limit, recorded: `gh pr diff` and the list are two reads, so a push
between them can make the recorded commit one behind the lines shown; the next list read corrects
it and the batch then states both commits (US4 s2), which is the specified handling.

**Rejected**: asking `gh` for the diff at a pinned commit (the compare API caps files and
differs from the PR diff 1003 shows); reading the commit with local git (the branch may be
unfetched, FR edge case).

## R4. Re-anchoring on a diff that has only hunks

**Decision**: `anchor::locate` becomes generic over a `Lines` trait (`line(n) -> Option<&str>`,
`len()`), implemented by `SideLines` and by new `HunkLines` (the numbered lines of one side that
the file's hunks show). A PR comment is located in `HunkLines` of its file; code outside the hunks
counts as gone. PR comments are judged for every file with a comment, not only the selected one,
since all patches are in memory. Moves are persisted with the existing `Reanchor` op (a PR move no
longer suppressed by `reanchor_effect`).

**Rationale**: the PR diff carries no full file (775: "no full versions to show"). A line the user
could pick was in a hunk; if after a push it is in no hunk, it left the pull request's change, and
"outdated" is the honest mark (946 FR-021). Reuses the 946 rules and their tests.

**Recorded limitation**: a comment on an unchanged context line that a later push moves outside every hunk is
marked outdated although its code is unchanged. The batch still carries the recorded quote and
lines, so nothing is lost; SC-004 and FR-012 are read as "unchanged code still shown in the diff" (spec amended).
A test pins this.

**Rejected**: reading the file at the head commit with local git (not always local, R3); never
re-anchoring PR comments (violates FR-012/SC-004).

## R5. Where the batch is built and what travels

**Decision**: the daemon builds the batch (as 482: `EntryReview::begin_send` builds the snapshot
under the lock). `ReviewSend` carries `PrSend { number, title, head_branch, state, current_head }`
from the client's list. The daemon validates it (number > 0, hex commit ids of 40 or 64 digits,
non-empty branch without control characters, state one of four), flattens the title to one line
(control characters to spaces, at most 200 characters), never logs or stores it, and reads two
facts itself: the entry's checked-out branch and whether each distinct recorded commit exists
locally. `outdated` ids from the client are trusted for both sides of PR comments (the daemon has
no PR diff), and for the removed side only of working-tree comments as before.

**Rationale**: the daemon owns delivery and the open send; the client alone knows the list. Facts
about the checkout belong to the process that sits beside the checkout (the daemon may be remote).
The flattened title keeps terminal control bytes out of the typed batch.

**Rejected**: the client building the batch (breaks the single place the format lives and the
daemon's byte-identical test); the daemon asking GitHub (offline sending is required, FR edge
case); trusting the title unfiltered.

## R6. Wrong branch, merged or closed

**Decision** (clarified by the user): deliver anyway. When the checked-out branch is not
`head_branch` (or HEAD is detached, or unreadable), the preamble says to switch to it first. A
merged or closed pull request is stated in the preamble; sending is allowed.

## R7. Persistence

**Decision**: the file is written as `version: 3` only while it holds a PR comment, else as 2. Each comment
carries `pr` (`null` for the working tree) in version 3. Versions 1, 2 and 3 load. An older build meets version 3 as an unknown version and
quarantines the file (482 rule; downgrading after using PR comments loses that file's comments, accepted), rather than reading PR comments as working-tree ones.

**Rejected**: writing 3 always (needless downgrade loss); keeping version 2 with an optional field (an older build would drop the field and
treat PR comments as working-tree comments, then send them with the wrong code).

## R8. Protocol

**Decision**: one bump, 56 to 57, for `Add.pr`, the scoped `ClearSent`/`DiscardPending`,
`ReviewSend.pr` and the comment shape. Struct-variant fields change postcard layout, so old and
new peers must not mix; the version handshake already refuses that.

## R9. The `Git` additions

**Decision**: two inherent `GitCli` methods in `review/git.rs`, beside `review_base` (the review
reads are inherent, not `Git` trait methods, and the daemon has no injectable git): `checked_out_branch(dir) -> Option<String>`
(`git symbolic-ref --short -q HEAD`; `None` when detached) and `has_commit(dir, sha) -> bool` (`git
cat-file -e <sha>^{commit}`), both through `read` with its deadline. No `Git` trait change, no `FakeGit`.

**Rejected**: the client sending its own `view.branch` (the client's local git may differ from
a remote daemon's checkout).
