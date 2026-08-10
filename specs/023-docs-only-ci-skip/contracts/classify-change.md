# Contract: `scripts/classify-change.sh`

The one place that decides whether a change is documentation-only. The workflow consumes its
output and makes no such decision of its own.

## Invocation

```text
scripts/classify-change.sh <base-ref> <head-ref>
```

| Argument | Meaning |
|----------|---------|
| `base-ref` | What to compare against. `origin/<base branch>` for a pull request; the pushed-from SHA for a push. |
| `head-ref` | The tip being classified. `HEAD` in both cases. |

**Environment**

| Variable | Effect |
|----------|--------|
| `FORCE_FULL_CI` | When `1`/`true`, short-circuits to `docs_only=false` with reason `forced by full-ci label`. The workflow sets it from the pull request's labels (FR-021). |

**Preconditions**: run from inside the repository, with `base-ref` **already fetched**. On a
`pull_request` run, `actions/checkout` with `fetch-depth: 0` is not sufficient on its own — it does
not create `origin/<base>`, which is why the existing `assertions` job fetches explicitly even
though it also uses `fetch-depth: 0` (`ci.yml:100-104`). The caller does the fetch; the script does
not, deliberately — a script that silently repairs its own inputs cannot tell "base missing" from
"base empty", and the difference decides whether the run falls back to the full pipeline.

## Output

Key/value lines on stdout, in `GITHUB_OUTPUT` form so the workflow can append them directly:

```text
docs_only=true|false
reason=<one line of prose>
```

Followed, when `docs_only=false` because of specific paths, by the offending paths on stderr — one
per line, so they land in the job log where a reader looking at a surprising full run will find
them.

**Exit status**: `0` whenever a verdict was reached, including `docs_only=false`. Non-zero only
when the script cannot run at all (wrong arguments, not a repository) — and even then the workflow
treats the absence of `docs_only=true` as code-affecting, so a crash fails safe.

## Behaviour

| Input | `docs_only` | `reason` |
|-------|-------------|----------|
| `FORCE_FULL_CI` set | `false` | `forced by full-ci label` |
| No files changed | `true` | `no files changed` |
| Every changed path carries `micold-docs` | `true` | `N documentation paths` |
| Any changed path does not | `false` | `N non-documentation paths` |
| `base-ref` unresolvable, or all-zero (new branch) | `false` | `base ref unavailable` |
| No merge base with `base-ref` | `false` | `no merge base` |
| `git diff` fails for any other reason | `false` | `could not determine changed files` |

**Rules the implementation must honour**

- The changed set is the **whole** difference from the merge base — `git diff --name-only -z
  "<base>...<head>"`, three dots — never one commit's worth (FR-005).
- `-z` and `core.quotePath=false`, so a path containing a space, a quote or a non-ASCII byte
  survives intact.
- Deletions and renames count as changed paths. A rename produces both sides.
- Classification is `git check-attr micold-docs`, batched over stdin; only `set` is documentation
  (FR-025, [data-model](../data-model.md#entity-documentation-set)).
- Every failure path lands on `docs_only=false` (FR-006). There is no failure mode where the
  pipeline skips work because something went wrong.

## Test cases

The harness (`scripts/tests/classify-change.test.sh`) builds a throwaway repository per case and
asserts the verdict — written and observed failing before the script exists.

| Case | Expected |
|------|----------|
| Only files under `docs/` | `true` |
| Only files under `specs/` | `true` |
| `README.md` alone | `true` |
| `CHANGELOG.md` alone | `false` — it is compiled into the binary (research §R6) |
| One `.rs` file plus twenty documentation files | `false` |
| A documentation file in the first commit, a `.rs` file in the second | `false` (FR-005) |
| A `.rs` file in the first commit, a documentation file in the second | `false` (FR-005) |
| A comment-only change to `.github/workflows/ci.yml` | `false` (FR-004) |
| A deleted documentation file | `true` |
| A deleted source file | `false` |
| A documentation file whose path contains a space | `true`, path intact |
| Empty diff | `true` (FR-007) |
| Unresolvable base ref | `false`, reason `base ref unavailable` |
| `FORCE_FULL_CI=1` over a documentation-only diff | `false`, reason `forced by full-ci label` |
