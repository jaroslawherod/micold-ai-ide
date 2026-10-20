# Research: full-file view and repo browsing in the Changes view (#1150)

## R1: Reveal by shaping the diff, not by a second renderer
- **Decision**: `expand::shape(hunks, new_side, &Expansion) -> Shaped { hunks, gaps }` inserts the revealed lines as `LineKind::Context` `DiffLine`s (old/new numbers from the neighbouring hunk's offset) and merges touching hunks; `gaps` lists the still-hidden runs with the index of the hunk each precedes (`hunks.len()` for the trailing one). `unified_rows`/`side_by_side_rows` and their `*Index` types take `&Shaped`, emitting a gap row where a gap is.
- **Rationale**: FR-002/FR-007/SC-002 (numbers, colour and layouts identical to context lines) and FR-006 (picking) come for free; the logic is pure and testable in core.
- **Rejected**: a separate "expanded view" widget (duplicates layouts, Principle VIII); inserting lines inside `DiffView` at draw time (decision logic in untestable glue, Principle I).

## R2: Where the new side's lines come from
- **Worktree changes**: the field `LoadedDiff.new` already holds the full new version of the diff's own range (`GitCli::file_diff` reads it for outdated checks): the working tree normally, `HEAD` in the committed-only view. The revealed side is always the diff's own new side, which is FR-005's "worktree's current content" in the default view and the committed content when the user chose committed-only (stated in the user guide). Browsed files follow the same toggle: committed-only reads `HEAD`, otherwise the working tree. The field is read at the same moment as the diff, so numbering always matches the diff (concurrent edits cannot desync, spec edge case). No extra read.
- **Pull request**: `LoadedDiff` is parsed from the patch with `new: None`; the file is read at the PR head (R3) **when the file is selected** (eager, one read, cancelled by a newer selection), so the trailing marker's hidden-line count is known before the user acts (FR-001). Until the read answers, the file shows its hunks with no markers and a muted "Reading file…" caption; a failed read shows the FR-009 message and no markers. For worktree changes the new side is read with the diff, so there is no such state.
- **Rejected**: re-reading the worktree file at expansion time (races with the agent's edits, shows lines against stale numbering).

## R3: Reading a file at the PR head
- **Decision**: `FileSource::PrHead { commit, repo }` where `commit` is `PullRequestStatus::head` (`headRefOid`, already held by the list). Order: (1) local `git cat-file blob <commit>:<path>` in the entry directory when `git cat-file -e <commit>` succeeds (offline, fast, also covers deleted head branches whose commits are still local); (2) otherwise `PullRequestSource::file_at(repo, commit, path)`, one `gh api repos/{o}/{r}/contents/{path}?ref={commit}` request with `Accept: application/vnd.github.raw`, no retry; (3) failure maps to the FR-009 message. Tree listing the same way: `git ls-tree -r -z --name-only <commit>`, else `gh api repos/{o}/{r}/git/trees/{commit}?recursive=1`.
- **Rationale**: spec Assumptions: "obtainable the way the PR diff itself is; no new credentials" (775 uses `gh` through `PullRequestSource`). The local path avoids a network round trip when possible.
- **Rejected**: `git fetch` of the PR ref (mutates the repository: FR-010 says expanding modifies nothing); reading the worktree file and assuming it equals head (SC-003 forbids).
- **Size**: the raw media type serves files up to 100 MB; files over `MAX_VERSION_BYTES` are refused before reading when the size is known (tree entry/`cat-file -s`), else cut at the limit and reported large. The recursive trees API can answer `truncated: true`: the list is then partial, a note says so, and the picker still accepts a typed path (read on demand). The args builder percent-encodes each path segment.

## R4: File tree and path picker
- **Decision**: the tree is the set of repository files of the shown source: worktree changes `git ls-files -z --cached --others --exclude-standard`; PR head per R3. A path picker (typeahead over the same list, existing `Typeahead`/`picker`) and a collapsible folder tree (existing `TreeView`) both emit `Msg::BrowseOpen(RelPath)`. Untracked-and-ignored files (FR-011) are reachable only by typing a path in the picker, which checks existence on disk (worktree changes only).
- **Rationale**: one list serves tree, picker and reference resolution (R8) with no per-line I/O.
- **Rejected**: walking the filesystem (ignores .gitignore, slow in large repos; `fs_scan.rs` serves another purpose).

## R5: Path confinement (FR-014, symlinks)
- **Decision**: `browse::Confined::new(root, rel)` rejects absolute paths, `..` components and Windows prefixes lexically, then for disk reads canonicalises the parent and requires it to start with the canonical root; a final symlink is read as a link target string only if it stays inside, else refused ("not followed outside the repository"). PR-head reads use git object paths, which cannot escape.
- **Rejected**: lexical check only (symlink escape).

## R6: Shared components
- `DiffView` gains builder methods for gap rows and reference spans (Principle VIII: extend, not fork). New `BrowseBar` (path, Back, "Open diff") is a builder component with a showcase entry. Tree and picker reuse existing `TreeView` and `Typeahead`.

## R7: Picking
- **Decision**: one function `pickable(view, file) -> bool`, NEW in `features/changes.rs` and called by the existing pick handling in `update`, is the only gate; revealed and browsed-in-diff lines follow whatever the diff's pick path decides; untouched browsed files return `false` until #1149 specifies an anchor (spec Story 4 s3: no anchor of our own). A test pins `false` so #1149's change is a visible one-line edit.
- **Rejected**: inventing an anchor for untouched files (spec forbids).

## R8: References in diff lines
- **Decision**: `reference::find(text, resolve) -> Vec<Reference { range, path, line }>`. Candidates: quoted strings and bare path-like tokens (contain `/` or end in a known extension), with optional `:LINE`, `#LLINE`, `(LINE)` suffix. Resolution against the source's file list (R4): relative to the current file's directory, then repository root; for extensionless relative imports (`./x`, `../x`) probe `.rs .ts .tsx .js .jsx .py .go .md` and `/index.*`, `/mod.rs`. A candidate resolving to no listed file is not returned (not activatable). Language module paths (`crate::a::b`, `import a.b`) are out: only strings that name a file.
- **Rationale**: spec FR-012 "path or import string that names an existing repository file"; symbol lookup out of scope.
- **Rejected**: using rust-analyzer/LSP or ctags (semantic intelligence is out of scope).

## R9: Large files and responsiveness
- Over `MAX_VERSION_BYTES`: expansion/browse shows the large note with "Show anyway", remembered in `shown_large` (per file, while the view is open; extends the existing 482 set, keyed by source). Rows are virtualised already. Whole-file syntax highlighting (`ui/syntax::highlight_file`) runs on the blocking thread under the existing time budget (#771); on budget the lines show uncoloured with the diff otherwise intact. A single progressive path is enough for 5,000 lines; beyond the 2 MB limit nothing is read without consent.

## R10: Refresh and state keys
- `Expansion` is keyed by `(SourceKey, RelPath)` where `SourceKey = Worktree | Pr(number)`; so state of one source never applies to another. On a diff refresh, `reconcile` keeps an open gap whose position and new-side span are unchanged, else drops it; whole-file stays on. Back-to-diff stores the selection and scroll offset only; restore re-shapes from the per-file `Expansion` map against the refreshed diff.

## R11: No protocol/daemon change
- Reads are local to the client like the 482 file list and diff (`app.caps.shared_git()`); a daemon that is not local yields the existing `NOT_LOCAL` path, where expansion and browsing show the FR-009 message. Rejected: daemon-side file reads (a protocol bump for no benefit).
