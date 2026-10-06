# Contract: the review prompt text

Built by `micold_core::review::prompt::build` from the pending comments of one entry. Pure and
deterministic: the same comments give the same bytes on every platform (FR-015, SC-006).

## Shape

```text
Please address these review comments on the changes in <where>. Each one names a file (relative to
<root>), the lines it is about, and the code on those lines when the comment was written.

## <path>

### Lines <start>-<end> (current)
<fence>
<quoted line>
…
<fence>
<comment text>

### Line <n> (removed; line <n> of the base version)
<fence>
<quoted line>
<fence>
<comment text>

## <next path>
…
```

- `<where>` is `this worktree` for a worktree entry and `the project root` for the Default entry;
  `<root>` is `the worktree root` / `the project root`.
- One `## <path>` section per file, files in bytewise order of their `/`-separated relative path.
- Inside a file, comments ordered by start line, then `current` before `removed`, then creation
  time, then id.
- Heading: `Line <n>` for one line, `Lines <start>-<end>` (ASCII hyphen) for a range. Suffix
  `(current)` for new-side lines; `(removed; line <n> of the base version)` or
  `(removed; lines <start>-<end> of the base version)` for old-side lines (US2 s4).
- An outdated comment's heading gets ` - the file has changed since; quoted as written` (FR-013).
- `<fence>` is a run of backticks one longer than the longest backtick run in the quoted lines,
  at least three. No language tag.
- Quote over 50 lines (`MAX_QUOTED_LINES`): the first line, then
  `... <k> lines not shown ...`, then the last line, where `k = len - 2` (Edge "Very long comment
  ranges"). The heading keeps the full range.
- Comment text as stored (trimmed), with `\r\n` and `\r` normalised to `\n`.
- Line separator `\n` everywhere; one blank line between blocks; no trailing line break (the
  submission adds the single Enter, `encode_submission`).
- Nothing else from the comment list (no sent comments, no ids, no timestamps) (FR-014).

## Tests that pin it (core unit, `review/prompt.rs`)

P1 two files, three comments: exact expected string. P2 removed-line comment wording and base line
number. P3 range of 51 lines elided to first/last with `49 lines not shown`; range of 50 not elided.
P4 a quote containing ```` ``` ```` gets a four-backtick fence. P5 CRLF in comment text becomes LF; no
`\r` anywhere in the output. P6 ordering ties (same start line, both sides; same side, creation
time). P7 Default entry wording. P8 outdated suffix. P9 20 comments across 5 files: every id's text
appears exactly once (SC-002). P10 paths with spaces and non-ASCII are kept verbatim.
