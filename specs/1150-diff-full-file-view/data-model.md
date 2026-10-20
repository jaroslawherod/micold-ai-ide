# Data model: #1150

All types are in-memory only (nothing persisted, nothing on the wire).

## 1. Hidden range (`review::expand::Gap`)
`Gap { before_hunk: usize, new_start: u32, len: u32, old_start: u32 }`: a run of unchanged lines. Positions: before the first hunk (`before_hunk == 0`), between hunks, after the last (`before_hunk == hunks.len()`; its `len` needs the new side's line count). `gaps(hunks, new_total) -> Vec<Gap>`; zero-length runs are omitted. Hunk-header conventions (`diff.rs`): for `new_len == 0` (pure deletion) `new_start` is the line before the deletion, and `new_start == 0` when the new side is empty; so a hunk's covered new range is `new_start + (new_len == 0 ? 1 : 0) ..= new_start + new_len` (empty when `new_len == 0`), and gap boundaries use that range, never `new_start+new_len-1`. A hunk at line 1 has no leading gap; one ending at the last line has no trailing gap (so empty and one-line files, added files and files fully covered offer no marker).

## 2. Expansion state (`Expansion`)
`Expansion { whole: bool, open: BTreeSet<GapKey> }`, `GapKey { position: Position, new_start: u32, len: u32 }`, `Position::{Leading, Between(u32 /* new_end of previous hunk */), Trailing}`. Methods: `toggle(&Gap)`, `set_whole(bool)`, `is_open(&Gap)`, `reconcile(&self, gaps: &[Gap]) -> Expansion` (keep open keys equal to a current gap's key; drop others; keep `whole`). Per file: `BTreeMap<(SourceKey, RelPath), Expansion>` in the open view; `SourceKey::{Worktree, Pr(u64)}`.

## 3. Shaped diff (`Shaped`)
`shape(hunks: &[Hunk], new_side: &SideLines, e: &Expansion) -> Shaped { hunks: Vec<Hunk>, gaps: Vec<Gap> }`. Invariants (property-tested): every line of `new_side` appears exactly once in new-number order when `whole`; changed lines and removed lines are preserved in place; line numbers are continuous across a revealed run and its neighbours; touching hunks are merged; `shape(h, s, closed) == h` with `gaps` listing every hidden run. Collapsing returns the input hunks. Merging touching hunks recomputes `old_len`/`new_len`, keeps the first hunk's `section` and `old_start`/`new_start`, and takes `old_no_newline`/`new_no_newline` from the last hunk. Property tests cover pure-deletion hunks, a hunk at line 1, a hunk at EOF, an empty new side, and a missing trailing newline.

## 4. Browse stack (`BrowseStack`)
`Frame::{Diff { file: Option<RelPath>, offset: u32 }, File { path: RelPath, offset: u32 }}` (expansion lives in the per-file map, so a frame stores only the selection and scroll); `Vec<Frame>`; `push(frame)`, `back() -> Option<Frame>`. Opening a file from a diff pushes the diff frame; opening from a browsed file pushes that file's frame. Back with an empty stack is a no-op. On source change the stack is cleared and the current browsed path is re-read (spec edge case).

## 5. File source (`review::browse::FileSource`)
`FileSource::{Worktree { dir: PathBuf }, PrHead { dir: PathBuf, repo: GithubRepo, commit: String }}`. `Confined` newtype = a `RelPath` already checked against the root (R5). `list_tree(&FileSource) -> Result<Vec<RelPath>, FileReadFailure>`; `read_file(&FileSource, &Confined, force_large) -> Result<FileText, FileReadFailure>`.

## 6. File text and reasons
`FileText { lines: SideLines, bytes: u64 }`. `FileReadFailure::{Binary, NotUtf8, Missing, TooLarge { bytes }, OutsideRepository, Unreadable, HeadUnavailable}` each with one user message (spec edge cases; FR-009). `Revealable` is `Some` only for `FileDiff::Text` of a file that is not added and not deleted.

## 7. Reference (`review::reference::Reference`)
`Reference { range: Range<usize>, path: RelPath, line: Option<u32> }` (byte range in the line text). Derived per visible line from the source's file list; never stored.

## 8. Browsed file view (`features::changes::BrowsedFile`)
`BrowsedFile { path, source: SourceKey, load: Load<FileText>, spans: Option<SchemeSpans>, diff_available: bool }`; its rows come from `diff::plain_file(&SideLines)` (a `FileDiff::Text` of context lines), rendered by `DiffView` with no old side. `diff_available` is true when the change list contains the path (FR-015).

## State transitions
Diff -> (BrowseOpen | ReferenceActivated) -> File -> (Back | OpenFileDiff) -> Diff. Source switch (worktree/PR/PR): expansion map entries of other sources stay but are not applied; browse stack cleared.
