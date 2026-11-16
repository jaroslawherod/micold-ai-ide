# The CI pipeline: what runs, what gets skipped, and why

Every pull request used to compile the whole workspace — including the 27% of merged pull requests
that changed no code at all (measured over the last 100 merges). A change that only adds a bug
report under `specs/` or fixes a sentence in the user guide cannot affect what the compiler or the
test suite sees, so it no longer pays for a Rust compilation.

Linux is the only supported platform (#1897): every job runs on a Linux runner, and there is no
macOS or Windows leg, package or release job.

This page explains how that decision is made, what still runs, and what to do when the pipeline
surprises you.

## The two paths

Every run is classified as **documentation-only** or **code-affecting**, and takes one of two
shapes:

| Job | Code-affecting | Documentation-only |
|-----|----------------|--------------------|
| `classify change` | runs | runs |
| `fmt + clippy` | runs; skipped on a [proven main push](#main-pushes-that-repeat-the-pull-request-run) | **skipped** |
| `build + test (ubuntu-latest)` | runs | **skipped** |
| `assertion freeze (advisory)` | runs; skipped on a proven main push | **skipped** |
| `sandbox against a real runtime (linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `container daemon end to end (docker, linux)`, `(podman, linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `remote daemon over a real sshd (linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `web client` | runs on main pushes and nightly; on a pull request only with [web client paths](#path-gated-jobs) | **skipped** |
| `docs check` | runs | **runs** |
| `ci complete` | runs | runs |

Three jobs on a documentation-only run.

`sandbox against a real runtime` builds the `:dev`
sandbox image from the branch and then runs the `sandbox_real_*` tests in **two** steps, one per
crate — `micold-core` for the adapter layer, `micold-daemon` for what the isolation is for. The
second step is not optional tidiness: it was missing until feature 027's T147, and its eleven tests
carry the sandbox's headline claims. A `-p` that names one crate does not report the other as
skipped; it produces no output about it at all, which is why the job looked complete for the whole
of the feature it was written for.

The multi-daemon end-to-end tests (feature 491) are `sandbox_real_multi_daemon*` in
`crates/micold-daemon/tests/`, so the second step already runs them and needs no step of its own:
a container daemon, the host daemon and both together (one stopped, the other checked), each
driving a scripted stand-in for the AI CLI that sits first on `PATH`, so there is no sign-in,
network or cost. Run them on demand with `mise run image && mise run test-sandbox`; add
`MICOLD_RUNTIME=podman MICOLD_TEST_RUNTIME=podman` on a host with podman only. They are off by
default, like every `sandbox_real_*` test.

The provisioning end-to-end suite (feature 806, FR-025) is `sandbox_real_provision.rs` in the same
directory, so that second step runs it too: fresh start, reuse without copying, an image without an
SSH server, a failing install, a migrated daemon, a `NoOutbound` install through the relay, a stop
during a long install that leaves no container or daemon, and two concurrent starts that give one
container and one daemon. The 489 suite below runs on the shared provisioning. Locally:
`mise run image && mise run test-sandbox`; neither runs by default.

`container daemon end to end` (#1281) is a matrix of two legs, **docker** and **rootless podman**,
on the Linux runner. The job above starts its containers with the harness's own `create` argv, so a
mount option podman rejects in the application's argv (#1279) passed it and reached users. This job
creates the container the way Settings does — `DaemonContainers::bring_up`, so the credential
tmpfs, the worktree bind and the uid/gid mapping (`--user` on docker, `--userns=keep-id` on podman)
are the application's — from `micold-basic:dev` built from the branch, and drives the app's own
connection actor (`micold_client::daemon::actor`): the daemon is provisioned over SSH with the
stand-in AI CLI of the daemon suites (installed from a tarball in the shared project, no network),
a session starts, the stand-in answers a prompt, the session stops and the container is removed. A
refused option ends the start with the runtime's own output, which is the test's failure message.
The tests are the `sandbox_real_*` tests of `micold-client` (`src/shell/sandbox_real.rs`, the BUG-868
bring-up test included), run with `--bin micold-ai-ide` so no other test target is built.

A skip there is a failure: the tests skip on a host without the runtime or the image, and the job
sets `MICOLD_REAL_RUNTIME_REQUIRED=1`, which turns that skip into a panic. The last step also
checks that both tests passed and that the end-to-end test printed its closing timing line (`<runtime>:
connected after …s, done after …s`), so a filter that matches nothing cannot pass either. Locally:
`mise run image && mise run test-sandbox`, with `MICOLD_RUNTIME=podman MICOLD_TEST_RUNTIME=podman`
for podman.

`remote daemon over a real sshd` (feature 489) installs `openssh-server` and runs the `ssh_real_*`
tests (`mise run test-ssh` locally): a rootless `sshd` on loopback that logs in as the runner's own
user, the client's own build copied to a private `$HOME` and started through the bridge. They are
off by default (`ssh-real-runtime`) and skip with a printed reason on a machine with no `sshd`.

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

## Path-gated jobs

Beyond the docs-only verdict, `classify change` publishes two more booleans. The first, `packaging`, is true when
**any** touched path is declared `micold-packaging` in `.gitattributes` (`packaging/**`, the
workspace `Cargo.toml` and `Cargo.lock`, `release.yml`, `LICENSE`, `remote-daemon-bundle.sh`,
`build-lock.sh` and the toolchain pins). It is written to the run summary; since the Windows ARM64
package job was removed with Windows support (#1897), no job is gated on it.

The other, `daemon_runtime`, is true when **any** touched path is declared `micold-daemon-runtime`
(#954): `crates/micold-core/**`, `crates/micold-daemon/**`, the workspace `Cargo.toml` and
`Cargo.lock`, the toolchain pins (`rust-toolchain.toml`, `.cargo/config.toml`),
`packaging/sandbox/**` and `packaging/basic/**` (the two images the sandbox job builds),
`scripts/basic-image-hash.sh`, the icon `micold-core`'s `build.rs` reads, and **`ci.yml` itself**.
It gates `sandbox against a real runtime` (15–20 minutes), `container daemon end to end` (both
legs) and `remote daemon over a real sshd` (about 5 minutes) with the same rule as above: always on
`main` pushes and the nightly, on a pull request only when `daemon_runtime` is true, which `full-ci`
forces. One attribute serves all three jobs: they test the same crates, the sandbox jobs only add
the image directories, and the sshd job is never the critical path, so a second attribute would
save a few minutes on the rarest change (`packaging/**` alone) at the price of a second output to
keep in step. Of `micold-client` only the three files the end-to-end job drives are in the set
(`src/daemon.rs`, `src/shell/sandbox.rs`, `src/shell/sandbox_real.rs`); the rest of it is built by
the `test` job. A UI-only change skips the jobs; a daemon change runs them. Every failure path lands
on `daemon_runtime=true`.

A third, `client` (#1735), is true when **any** touched path is declared `micold-client`:
`apps/client/**`, `packages/protocol/**`, `pnpm-lock.yaml`, `pnpm-workspace.yaml` and the root
`package.json`. It gates the `web client` job (pnpm install, lint, typecheck, Jest with coverage,
diff coverage of the changed lines, the static web export; under ten minutes). `ci.yml` itself is
in the `micold-daemon-runtime` set, not this one, so an edit to the job rides on that verdict only
through a `full-ci` label or a client path. Every failure path lands on `client=true`.
`scripts/tests/client-pins.test.sh` keeps the job's node and pnpm pins equal to `mise.toml`.

To gate another job the same way: declare an attribute in `.gitattributes`, emit one more boolean
from `scripts/classify-change.sh` (the `packaging` block is the pattern), pin it in
`scripts/tests/classify-change.test.sh`, expose it as a `classify` output, and put it in the
job's `if:`.

## The Kubernetes real-cluster job

`kubernetes daemon on a real cluster` (`kube-cluster`, feature 686, FR-021) runs the `kube_real_*`
tests of `micold-client` against a throwaway `kind` cluster, through `scripts/test-kube.sh` (the
same script as `mise run test-kube`). It skips with the docs-only rule of the other jobs and runs on
every other change, whatever path: it is not gated by `daemon_runtime`, because the code under test
spans `micold-core`, `micold-client` and `packaging/`.

- **Linux only**, like the rest of the pipeline: kind needs a Linux container runtime.
- **Pinned.** `kind` and `kubectl` are downloaded at a fixed version and checked against a sha256
  (`KIND_SHA256`, `KUBECTL_SHA256` in the job's `env`); the node image is one stable Kubernetes
  version (`NODE_IMAGE` in `crates/micold-client/tests/kube_real_support/mod.rs`). Bump the three
  together, and take the checksums from the release's own `.sha256` files.
- **No registry, no sign-in, no provider network.** The pod's SSH image is built in the job from
  `packaging/basic` plus `packaging/kube/test.Containerfile` and loaded with `kind load
  docker-image`. The stand-in AI CLI comes from a package source the test process serves on the
  `kind` docker network's gateway (172.30.0.1), and the git remote is a `git daemon` there.
- **Skips, not failures.** Without docker, kind or kubectl each test prints `SKIP <name>: <reason>`
  and passes; the job installs them, so a skip in CI means the install step is broken.
- Part of `ci complete`; add `full-ci` to run it on a docs-only change.

## Red on main is reported

**Notification.** When `ci complete` fails on a push to `main` or on the nightly run, its last step
(`scripts/ci-notify-failure.sh`, cases in `scripts/tests/ci-notify-failure.test.sh`) says so:

- push: a comment on the merged pull request that carries the commit, mentioning its author (a bot
  author is named, not mentioned);
- no merged pull request (a direct push) or the nightly run: the tracking issue **CI is red on
  main** is opened, or commented on when one is already open.

The step has `pull-requests: write` and `issues: write` through `permissions:` on the `ci complete`
job alone; no other job holds them. It does not run on pull requests. Auto-revert is not done. A
merge queue (the ruleset would then run the full suite on the queued commit before it lands) is the
stronger alternative; it is a repository setting, outside this change.

## Main pushes that repeat the pull request run

A merged pull request already ran the pipeline on the tree that lands on `main`, so a push to
`main` re-running all of it (about 25 minutes, twice as many runner-minutes as needed) proves
nothing new. When the push is **PR-proven** (#959) it skips what the pull request run covered and
keeps what pull requests may skip:

| Job on a proven push | |
|---|---|
| `fmt + clippy`, `assertion freeze` | **skipped** (lint and the shell suites ran on the PR) |
| `build + test (ubuntu-latest)` | runs (`test_os` is the Linux leg on a proven push too) |
| `static Linux daemons` | runs (`test` needs its artifact) |
| sandbox, sshd | run |
| `docs check` | runs (cheap, and deliberately unconditional) |

**Detecting it.** `classify change` runs `scripts/pr-proven.sh` on a push (cases in
`scripts/tests/pr-proven.test.sh`) and exposes `pr_proven`. It is `true` only when exactly one
**merged** pull request into `main` carries the pushed commit (`GET commits/{sha}/pulls`) **and**
that pull request's head commit has the same git tree as the pushed commit. Everything else is
`false` and keeps the full pipeline: a direct push, several pull requests, a tree mismatch (a
rebase-merge of a branch that was behind `main` produces a tree no run tested), an API error. The
nightly schedule and pull requests are never proven, so the nightly still runs everything.

**The assumption.** The pull request's run was green on its head tree. That holds because the
branch ruleset requires `ci complete` and requires branches to be **up to date** before merging;
with it off, trees would differ more often and the script would just fall back to the full run, but
a PR run that was red and merged anyway (admin bypass) would be taken as proven. A merged pull
request that skipped jobs by path (docs-only, no daemon-runtime paths) skipped them on the push too:
`classify` still reads the push's own diff, so the path gates behave as before.

**How the skipped jobs keep `ci complete` honest.** `fmt + clippy` is skipped, not failed, so the
heavy jobs that `needs: lint` use an explicit condition (`!cancelled()`, `classify` succeeded,
`lint` is not `failure`, `static Linux daemons` succeeded, then the path gates) instead of the
implicit `success()`, which would skip them with lint. On a pull request a failed lint still skips
them. The `test` matrix takes its legs from the `test_os` output of `classify change`. `ci
complete` and its name are unchanged and treat skipped as satisfied.

**Notification.** The nightly run's tracking issue now mentions the author of the commit it tested,
as a direct push's does.

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

## Fail fast on lint

`build + test`, `sandbox against a real runtime`
and `remote daemon over a real sshd` list `fmt + clippy` in `needs:` (#957). A formatting or clippy
failure now shows red at about 2 minutes instead of the ~25 the heavy jobs took to report. The run
itself closes when `static Linux daemons` finishes (about 3 minutes), because that job does not wait
on lint; a green run starts those jobs about 95 s later. `static Linux daemons` and `assertion freeze` stay
parallel with lint: they are short and the daemons job feeds `test`, so gating it would serialise
the two.

- **Red lint still blocks.** A job that `needs: lint` is *skipped*, not failed, when lint fails.
  `ci complete` reads lint's own result (`failure` is `BLOCKED`), so the required check goes red.
- **The `if:` clauses keep their meaning.** Lint skipped on a docs-only run skips these jobs, as
  `docs_only` already did. Lint skipped on a [proven main push](#main-pushes-that-repeat-the-pull-request-run)
  must not, so `test`, sandbox and sshd spell their conditions out (lint is not
  `failure`) rather than rely on `success()`.
- Why not cancel siblings on first failure: it needs a token with `actions: write`, and a cancelled
  run reports `cancelled`, which is less legible than a red lint.

## The user-guide gate

Constitution Principle VII says a user-facing feature is not done until its documentation exists,
and the Documentation gate requires the guide to be updated in the same pull request. Half of that
was already mechanical — the docs build runs in CI — and half was not: whether the guide had in
fact been updated was left to review, which is the half a reviewer skips when the diff is long and
the feature works. `scripts/check-user-guide-updated.sh` is the other half.

It blocks a pull request when **all** of these hold:

1. Its title is a conventional-commit **feature** — `feat:`, `feat(scope):`, either with a `!`.
2. It touches a path declared `micold-user-facing` in `.gitattributes` — the client's screens
   (`crates/micold-client/src/ui/**`) and the behaviours they trigger
   (`crates/micold-client/src/features/**`).
3. It touches no page declared `micold-user-guide` — that is, nothing under `docs/user-guide/`.

A developer doc does not satisfy it. The reader Principle VII protects is the person using the
application, not the person building it.

### Why features and not every edit

The word in the principle is "feature", and holding the gate to it is what makes it worth having.
Replayed over the sixty merges before the gate existed, firing on any edit under the declared
paths would have blocked ten changes — eight of them bug fixes whose guide page says nothing
different. A label applied that often stops being read. Firing on `feat` alone would have blocked
two, and both were real omissions: naming the AI CLIs the service cannot find, and the Settings
icon rail.

Nothing lints the title prefix today, so writing `fix` gets past this. It is a guard against
forgetting, not against intent — and the convention is already load-bearing, since release-please
reads the same prefixes to decide version numbers.

The component library underneath the screens (`ui/material/**`, `ui/cdk/**`) is deliberately
excluded: Principle VII is satisfied for it by
[the component library guide](component-library.md), and a ripple timing or a token tweak changes
pixels without changing anything the user guide says.

### Getting past it

Either update the page in `docs/user-guide/` that the change makes wrong, or label the pull
request **`docs-not-needed`**. Like `full-ci`, applying a label starts a fresh run, so the waiver
takes effect without a push.

The label is a decision somebody signs, not a way to make the check quiet. If you find yourself
applying it to most features, the declared path set is wrong — widen or narrow `.gitattributes`
rather than training everyone to reach for the label.

The gate runs on pull requests only; a push to main has already been through it. Its own cases are
in `scripts/tests/check-user-guide-updated.test.sh` and run in the same job, before it.

## The PostgreSQL service container (#1948)

The `test` job starts a `postgres:17` service container (port 5432, a `pg_isready` health check;
user, password and database come from the job's `env`) and sets `MICOLD_TEST_POSTGRES_URL` to it.
The orchestrator's shared store suite (`crates/micold-daemon/tests/orchestrator/store_suite.rs`,
`postgres` module) and the PostgreSQL cases in `store_migrations.rs` and `store_scan.rs` run there;
each case creates and drops a database of its own.

Locally the cases print `skipped: MICOLD_TEST_POSTGRES_URL not set` and pass when the variable is
unset. CI also sets `MICOLD_REQUIRE_POSTGRES=1`, which turns that skip into a failure: a missing or
misnamed service fails the job instead of passing silently. To run them locally, point the variable
at a server whose user may create databases, for example
`MICOLD_TEST_POSTGRES_URL=postgres://postgres:pw@localhost:5432/postgres`.

## Linux only (#1897)

Linux is the only supported platform. #1877 first switched the macOS and Windows CI legs off;
#1897 removed them for good, together with the macOS bundle and Windows installer scripts, their
packaging inputs, the platform code paths and the `macos` and `windows` release jobs. Nothing is
kept for a later restore: bringing a platform back is a new feature, not a revert. The
`linux-daemons` job in `release.yml`, which used to hand its static daemons to the macOS and
Windows release jobs, is kept.

## The daemon's integration tests are grouped into five binaries (issue 1175)

Each file in `crates/micold-daemon/tests/` used to be its own test binary, compiled and linked on
every run. The files that touch no environment variable are now modules of five binaries, `tests/<group>/main.rs`
with one `mod <former file name>;` per file:

| Binary | Holds |
|---|---|
| `connection` | wire protocol, framing, viewers, terminal and scrollback tests |
| `sessions` | session start, supervision, activity and attention, mutation semantics |
| `settings` | the `*_setting` and `settings_*` files, the catalog and pane layout |
| `mcp` | the `mcp_*`, `worktree_*`, `env_include_*` and review files (not `hooks_receiver`, which installs a global `tracing` subscriber) |
| `ssh_real` | the `ssh_real_*` suite (`--features ssh-real-runtime`) |

- **Test names keep their file name**: `mutation_semantics::<test>` where it was `<test>` in the
  `mutation_semantics` binary, so the `sandbox_real_` and `ssh_real_` name filters of `mise.toml`
  and `ci.yml` still match, and `cargo test -p micold-daemon -- --list` lists the same tests.
- **Support files** (`tests/support/*.rs`) are declared once per binary in its `main.rs`; the
  modules take them with `use crate::conn;`. File-level `#![cfg(...)]` attributes stay at the top
  of the module file, where they gate that module alone.
- **What stays a binary of its own**: every file that sets or removes an environment variable (or
  uses a support module that does), because tests of one binary share a process and an
  environment; and the five files that install a global `tracing` subscriber or re-run the test
  binary by test name (`osc8_passthrough`). A new test that touches neither belongs in a group.

## Real-runtime daemon tests build only their own targets

The `sandbox against a real runtime (linux)` job's `Test (real runtime, daemon)` step used to run
`cargo test --release -p micold-daemon ... sandbox_real_`, which compiles every test binary of the
crate (149 in release) before the name filter selects 25 tests. A run measured 947 s for the step,
928 s of it the compile and 16 s the tests. Splitting the tests across a `--partition` matrix did
not help: each leg compiled everything again.

The step now passes `--test 'sandbox_real_*'` (cargo accepts globs), so only the `sandbox_real_*`
integration targets are built; the job is a single job again and its key, `if:` and `needs:` are
unchanged.

- **No test lost.** `cargo test --release -p micold-daemon --features sandbox-real-runtime
  sandbox_real_ -- --list` lists 25 tests with and without the glob. The daemon has no
  `sandbox_real_` doctests, so the old command ran none either.
- **`--release` stays.** `sandbox_real_session_start` compares the release daemon in the image with
  `CARGO_BIN_EXE_micold-daemon`; the profiles must match (see the comment in `ci.yml`).

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
