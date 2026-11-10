# Data model

- **Facts file** (markdown, one per `<tree>:<head>`): header (snapshot, base, generated-at, tool status), `## Files` (path, changed ranges, kind), `## Symbols` (name, kind, file:line, language note), `## Callers` (symbol -> file:line list, count, truncated flag), `## Coverage` (per file: covered/uncovered/unknown line ranges, source). Sections for skipped parts say `skipped: <reason>`. Size target under 400 lines; larger lists are truncated with a count.
- **Review plan**: at most 6 probes. Probe = {id, question, targets (files or symbols), evidence-to-return}.
- **Evidence**: per probe, at most 40 lines, or `UNANSWERED: <reason>`.
- **Verdict**: unchanged `VERDICT:` and `F<n>` format.
- **Shadow record** (`shadow.md` row): PR, review, current findings (BLOCKER/MAJOR/MINOR counts), staged findings, parity result with adjudication, cache-read tokens current, cache-read tokens staged. Followed by a Decision section (FR-013).
