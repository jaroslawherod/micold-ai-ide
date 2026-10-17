# Contract: the review batch (replaces 482's contracts/review-prompt.md)

Built by `micold_core::review::prompt::build` from the Draft comments of one entry. Pure and
deterministic: the same comments give the same bytes on every platform and every build (FR-012,
SC-003). Independent of delivery (FR-026): the daemon types it into a terminal session as one
submission; a structured session (#945) can send the same text as a user message.

The user guide's "The review message" section (docs/user-guide/reviewing-changes.md) carries the
example below verbatim. Changing this layout means changing that section in the same pull request
(FR-013); the golden test P1 below pins the bytes.

## Shape

```text
<preamble paragraph 1>

<preamble paragraph 2>

### Comment <ref>: <path>, <lines> (current)
<fence>
<quoted line>
…
<fence>
<comment text>

### Comment <ref>: <path>, <lines> (removed, base version)
Note: the file has changed since this comment was written; the code is quoted as written.
<fence>
<quoted line>
<fence>
<comment text>
```

### Preamble

Paragraph 1 (`<where>`: `this worktree` / `the project root`; `<root>`: `the worktree root` /
`the project root`):

> Please work through these review comments on the changes in `<where>`. Each block below is one
> comment: its reference, the file (relative to `<root>`), the lines it is about, the code on those
> lines when the comment was written, and the comment itself.

Paragraph 2:

> For each comment, either make the change it asks for, or explain why you are not making it. Then
> report each outcome by the comment's reference with the report_review_comment tool of the micold
> tool server: outcome "applied" when you made the change (a reply is optional), or "answered" with
> your explanation as the reply. If you cannot use that tool, answer each comment by its reference
> in your reply instead.

Each paragraph is one line in the output (no hard wraps); the tool name comes from
`prompt::REPORT_TOOL`, the same constant the MCP catalog lists.

### Block

- Heading: `### Comment <ref>: <path>, <lines> <side>`.
  - `<ref>`: the comment's reference, `C-` + 8 lowercase hex digits (FR-014).
  - `<path>`: relative to the entry root, `/`-separated, verbatim (spaces and non-ASCII kept).
  - `<lines>`: `line <n>` for one line, `lines <start>-<end>` (ASCII hyphen) for a range; the
    comment's range after re-anchoring (FR-023).
  - `<side>`: `(current)` for the new side, `(removed, base version)` for the old side.
- Outdated note line, only for an outdated comment: `Note: the file has changed since this comment
  was written; the code is quoted as written.`
- `<fence>`: backticks, one more than the longest backtick run in the shown quote lines, at least
  three. No language tag.
- Quote: one line per quoted line, trailing `\r` dropped; over 50 lines the first line,
  `... <k> lines not shown ...` (`k = len − 2`), the last line. The heading keeps the full range.
- Comment text as stored (trimmed), `\r\n` and `\r` → `\n`.

### Whole message

- Blocks ordered by path (bytewise), start line, side (current first), creation time, id (FR-010).
- One blank line between paragraphs and blocks; `\n` only, no `\r` anywhere; no trailing line break
  (the submission presses Enter once, 482).
- Only Draft comments; nothing about Sent, Applied or Answered ones; no ids other than references,
  no timestamps.

## Example (worktree entry; P1's expected bytes)

````text
Please work through these review comments on the changes in this worktree. Each block below is one comment: its reference, the file (relative to the worktree root), the lines it is about, the code on those lines when the comment was written, and the comment itself.

For each comment, either make the change it asks for, or explain why you are not making it. Then report each outcome by the comment's reference with the report_review_comment tool of the micold tool server: outcome "applied" when you made the change (a reply is optional), or "answered" with your explanation as the reply. If you cannot use that tool, answer each comment by its reference in your reply instead.

### Comment C-00000002: src/a.rs, line 3 (removed, base version)
```
use std::io;
```
Keep this import.

### Comment C-00000001: src/a.rs, lines 12-14 (current)
```
fn a() {
    b.clone()
}
```
Why clone here?

### Comment C-00000003: src/b.rs, line 7 (current)
Note: the file has changed since this comment was written; the code is quoted as written.
```
let x = 1;
```
Name this.
````

## Tests that pin it (core unit, `review/prompt.rs`)

P1 the example above, exact bytes (comments with UUIDs `00000001-…` etc., C-00000003 outdated).
P2 Default entry wording. P3 51-line quote elided with `49 lines not shown`, 50 not elided, heading
keeps the full range. P4 a quote containing ```` ``` ```` gets a four-backtick fence. P5 CRLF in
quote and text: no `\r` in the output. P6 ordering ties (same start, both sides; same side, by
creation). P7 20 comments across 5 files: 20 blocks, 20 distinct references, each text exactly once
(SC-002). P8 built twice → equal bytes. P9 the preamble names `REPORT_TOOL` and the catalog lists a
tool of that name.
