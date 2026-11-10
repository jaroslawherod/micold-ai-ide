# Contract: `scripts/autopilot/review-facts.py`

```
review-facts.py --out-dir <dir> [--base <ref>=origin/main] [--base-snapshot <tree>:<head>] [--lcov <file>]
```

- Prints the facts file path on stdout; exit 0 on success, including unknown coverage or a docs-only diff.
- Exit 1 with a one-line reason on stderr: no `<base>` ref, detached HEAD, unknown or stale snapshot, git failure. The unit then runs the single-agent review (FR-011).
- Reads the working tree as `review-snapshot.sh` does (never touches the index, HEAD or stash). With `--base-snapshot`, the diff is `review-snapshot.sh diff <snap>` (round 2+).
- Uses no model and no network. Only `git` and the Python stdlib.
- File name `review-facts-<tree>-<head>.md` in `--out-dir`; a rerun for the same snapshot overwrites the same file.
- Non-Rust diff: `## Symbols` and `## Callers` say `skipped: no Rust files`; files and ranges are still listed.
- Coverage: with an lcov file, lines are `covered` or `uncovered`; otherwise `unknown (no coverage data)`. Never `covered` without data (FR-003).
