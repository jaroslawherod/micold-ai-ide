# The CI pipeline: what runs, what gets skipped, and why

Every pull request used to compile the whole workspace on Linux, macOS and Windows — including the
27% of merged pull requests that changed no code at all (measured over the last 100 merges). A
change that only adds a bug report under `specs/` or fixes a sentence in the user guide cannot
affect what the compiler or the test suite sees, so it no longer pays for three platforms' worth of
Rust compilation.

This page explains how that decision is made, what still runs, and what to do when the pipeline
surprises you.

## The two paths

Every run is classified as **documentation-only** or **code-affecting**, and takes one of two
shapes:

| Job | Code-affecting | Documentation-only |
|-----|----------------|--------------------|
| `classify change` | runs | runs |
| `fmt + clippy` | runs | **skipped** |
| `build + test (ubuntu-latest)` | runs | **skipped** |
| `build + test (macos-latest)` | runs | **skipped** |
| `build + test (windows-latest)` | runs | **skipped** |
| `assertion freeze (advisory)` | runs | **skipped** |
| `sandbox against a real runtime (linux)` | runs | **skipped** |
| `docs check` | runs | **runs** |
| `ci complete` | runs | runs |

Three jobs on a documentation-only run, all on Linux. No macOS or Windows runner is started.

`sandbox against a real runtime` is Linux-only because Docker Desktop is not available on GitHub's
macOS and Windows runners, so the alternative is no real-runtime job at all. It builds the `:dev`
sandbox image from the branch and then runs the `sandbox_real_*` tests in **two** steps, one per
crate — `micold-core` for the adapter layer, `micold-daemon` for what the isolation is for. The
second step is not optional tidiness: it was missing until feature 027's T147, and its eleven tests
carry the sandbox's headline claims. A `-p` that names one crate does not report the other as
skipped; it produces no output about it at all, which is why the job looked complete for the whole
of the feature it was written for.

`docs check` is deliberately unconditional: it is the one gate a documentation-only change can
actually break, so skipping the build must not skip it. Delete a required document and the run
fails, exactly as before.

For the same reason it is where the documentation site's pre-merge checks live — as steps in this
job, never as a job of their own, which a documentation-only change would skip. Besides the
required-document lists, `docs check` runs:

- `site/checks/page-set.sh` — every page under `docs/` is listed in `SUMMARY.md` and every entry
  names a file that exists, and the site stylesheet is still drawn from the design tokens rather
  than from literals;
- `site/checks/media-references.sh` — every media directive in a page names a manifest entry, and
  every manifest entry names a scene that exists and is referenced by some page;
- `site/checks/links.sh --sources` — every internal link and heading fragment in the sources
  resolves, without fetching a single external URL.

They fail on the pull request, where the page is still in front of its author, rather than at
render time on the release branch after the tag. The rest of a publication — the capture, the
render, and the checks that need a built site — is described in
[The documentation site](docs-site.md).

## What counts as documentation

The declaration lives in **`.gitattributes`**, as the attribute `micold-docs`:

```gitattributes
docs/**       micold-docs
specs/**      micold-docs
/*.md         micold-docs
/LICENSE      micold-docs
/*.png        micold-docs
.claude/skills/** micold-docs
/CHANGELOG.md -micold-docs
```

That file is the single source of truth. Two consumers read it, both through the same matcher
(`git check-attr micold-docs`), so they cannot drift apart:

- `scripts/classify-change.sh` — which paths did this change touch?
- `crates/micold-core/tests/documentation_is_not_read.rs` — which paths may no test read?

**Anything not listed is code.** Source, manifests, `Cargo.lock`, toolchain and tool
configuration, `scripts/`, `packaging/`, `assets/`, and the workflows themselves are all
code-affecting — even when only their comments change.

**Only `skills/` under `.claude/` is documentation.** Those files instruct the coding agent and
nothing in the build, the suite or the package opens them. The rest of that directory is the app's
own runtime state — `worktrees/`, local settings — which is untracked, so it stays code by default
rather than being declared prose it is not.

**`CHANGELOG.md` is a build input, not documentation.** `micold-core`'s `metadata.rs` does
`include_str!("../../../CHANGELOG.md")` so the app can show a "what's new" view offline, which
means changing the changelog changes the binary. It takes the full pipeline.

To check any path yourself:

```bash
git check-attr micold-docs -- path/to/file
```

`set` means documentation. `unset` and `unspecified` both mean code.

## How the classification is made

`scripts/classify-change.sh` diffs against the **merge base** (`base...head`, three dots), so the
verdict comes from everything the pull request changes, not from whichever commit happened to be
last. A pull request that changes a Rust file in its first commit and only prose afterwards is
still code-affecting.

Two rules govern it:

1. A change is documentation-only only when **every** touched path is declared documentation. One
   path outside the set makes the whole change code-affecting.
2. **Every failure path lands on code-affecting.** A base ref that cannot be resolved, an unrelated
   history, a diff that fails — all fall back to running everything. There is no input for which
   "something went wrong" means "skip the build".

## The merge gate

The default branch requires exactly one status check: **`ci complete`**. It summarises the run
rather than enumerating the pipeline, and that is what makes the skipping possible at all —
requiring individual job names welds the merge gate to the pipeline's internal shape, so no job can
be skipped without leaving a required check unreported and the pull request unmergeable forever.

`ci complete` fails if any job it covers failed or was cancelled. A **skipped** job satisfies it: a
job that did not need to run is not a failure.

Two properties are load-bearing, and both are asserted by
`crates/micold-core/tests/ci_gate_covers_every_job.rs` rather than left to review:

- **`if: always()`.** Without it the implicit `success()` applies, so the gate would be *skipped*
  whenever a covered job failed — and a skipped check reports success. The gate would go green
  exactly when the run went red.
- **Every job appears in its `needs:`.** A job added later and forgotten is a job whose failures
  stop blocking merges, silently. Nothing in GitHub Actions notices; the workflow stays valid.

Renaming the `ci complete` job breaks the merge gate for every pull request, and no pull request
can fix it — the ruleset lives in repository settings, not in the repo.

## Reading a run

The `classify change` job writes its verdict to the run summary, so the top of any run says which
path it took and why. On a code-affecting run it also lists the paths that decided it — the first
place to look when a change you expected to be cheap ran the full build.

`ci complete` writes a table of every covered job's result, including whether each one was gated.

A green `ci complete` on a documentation-only run means **nothing needed building** — not that the
build passed. The skipped jobs are shown as skipped precisely so the two are never confused.

## Forcing the full pipeline

Label the pull request **`full-ci`**. Applying it starts a fresh run that ignores the
classification and runs everything.

It has to be a fresh run: re-running an existing one replays the original event payload, which
would not contain a label added afterwards. Removing the label is deliberately not a trigger — the
next push reclassifies anyway.

Use it when you are changing the pipeline itself, when you suspect the classification is wrong, or
when something moved underneath the branch.

## Why skipping is safe

The whole scheme rests on one condition: **no test or build step reads the contents of the
documentation set.** If one did, the change that broke it would be exactly the change CI declined
to run.

That condition is not a promise in a document. `documentation_is_not_read` scans every Rust source
under `crates/` for a string literal that resolves to a real path carrying `micold-docs`, and fails
the build if it finds one. A handful of literals resolve to documentation paths while reading
nothing — a fixture branch name that happens to look like a directory, a filename created inside a
temporary repository — and those are allowlisted individually with a written reason. A stale
allowlist entry fails too.

The constitution's Development Workflow & Quality Gates section names this exemption and points at
that check (v1.6.0).

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| A change you expected to be cheap ran the full build | Something it touched is not declared documentation | Read the offending paths in the `classify change` summary. This is usually correct behaviour — widen `.gitattributes` only if the path genuinely cannot affect a build |
| The classifier reports `base ref unavailable` | The base ref was not fetched. `actions/checkout` with `fetch-depth: 0` does **not** create `origin/<base>` on a pull request | The `classify` job fetches it explicitly; if that step was changed, restore it. Symptom: everything looks fine and the full pipeline always runs |
| `ci complete` is pending forever | The ruleset requires a context nothing produced — usually the gate job was renamed | Restore the job name, or update the ruleset. Only the latter is possible from repository settings |
| `ci complete` is green but a job failed | The job is missing from the gate's `needs:`, or `if: always()` was dropped | Fix both, and check why `ci_gate_covers_every_job` did not catch it |
| `documentation_is_not_read` fails on a literal that reads nothing | A fixture string that happens to name a real documentation path | Add it to the test's `ALLOWED` list with a reason |
| A job fails in `Install … system dependencies` with `apt-get update` exit 100 and a 403 or timeout from a host this project does not use | A vendor repository the runner image preconfigures (Microsoft, Google Chrome) had an outage. `apt-get update` fails if *any* configured repository fails | Already handled: the step empties `/etc/apt/sources.list.d` except `ubuntu.sources` before updating, so only the Ubuntu archive is consulted. If this reappears, the archive itself is down — re-run |
