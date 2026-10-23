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
| `fmt + clippy` | runs; skipped on a [proven main push](#main-pushes-that-repeat-the-pull-request-run) | **skipped** |
| `build + test (ubuntu-latest)` | runs; no leg on a proven main push | **skipped** |
| `build + test (macos-latest)` | runs; the full daemon suite only on main pushes, nightly and `full-ci` ([why](#platform-daemon-suites-run-after-the-merge)) | **skipped** |
| `build + test (windows-latest)` | runs; the full daemon suite only on main pushes, nightly and `full-ci` | **skipped** |
| `assertion freeze (advisory)` | runs; skipped on a proven main push | **skipped** |
| `sandbox against a real runtime (linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `container daemon end to end (docker, linux)`, `(podman, linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `remote daemon over a real sshd (linux)` | runs on main pushes and nightly; on a pull request only with [daemon-runtime paths](#path-gated-jobs) or `full-ci` | **skipped** |
| `package + smoke (windows-11-arm)` | runs on main pushes and nightly; on a pull request only with [packaging paths](#path-gated-jobs) or `full-ci` | **skipped** |
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

The multi-daemon end-to-end tests (feature 491) are `sandbox_real_multi_daemon*` in
`crates/micold-daemon/tests/`, so the second step already runs them and needs no step of its own:
a container daemon, the host daemon and both together (one stopped, the other checked), each
driving a scripted stand-in for the AI CLI that sits first on `PATH`, so there is no sign-in,
network or cost. Run them on demand with `mise run image && mise run test-sandbox`; add
`MICOLD_RUNTIME=podman MICOLD_TEST_RUNTIME=podman` on a host with podman only. They are off by
default, like every `sandbox_real_*` test. The end-to-end runs on the Linux runner only; macOS and
Windows rest on the render-free tests (`daemon_rows_uniform`, `per_daemon_actors`, the registry
and state-machine tests), which run on every platform.

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
**any** touched path is declared `micold-packaging` in `.gitattributes` (`packaging/**`, the two
`scripts/windows-install*.sh`, the workspace `Cargo.toml` and `Cargo.lock`, `release.yml`, and the
installer's other inputs: its icon, `LICENSE`, `remote-daemon-bundle.sh`, `build-lock.sh` and the
toolchain pins). The
`package + smoke (windows-11-arm)` job — about 21 minutes, the pipeline's critical path — runs when:

- the event is a push to `main` or the nightly schedule (03:17 UTC), always; or
- the event is a pull request and `packaging` is true. The `full-ci` label forces it true.

A pull request that touches only Rust code does not wait on it. A skipped job satisfies
`ci complete`, so the required check and its name are unchanged. A nightly failure is a red
scheduled run on `main` (the scheduled run executes the whole pipeline, as it has no base ref to
classify against); GitHub notifies the author of the schedule's last edit.

Like `docs_only`, every failure path lands on `packaging=true`. `ci.yml` itself is not a packaging
path: a step edit to the job is proven on the next `main` push or night. Add `full-ci` when you
want it earlier.

The second, `daemon_runtime`, is true when **any** touched path is declared `micold-daemon-runtime`
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

- **Linux only.** kind needs a Linux container runtime; GitHub's macOS runners have none and its
  Windows runners cannot run Linux containers. The client-side rules that differ per platform
  (kubeconfig candidates, path separators, exec-plugin resolution, home directory) are pure core
  tests that the `test` job runs on all three platforms. The reason is also a comment in `ci.yml`.
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

## Platform daemon suites run after the merge

On macOS and Windows a pull request builds, runs the render-free core tests and the component and
architecture gates, packages and smoke-tests, but **skips the full daemon suite** (#956): the
`Test (daemon)` step on macOS and `Test (daemon, Windows)` on Windows (about 7–11 minutes, the
largest part of those legs). They run in full when:

- the event is a push to `main` or the nightly schedule; or
- the pull request carries the `full-ci` label (`classify change` exposes it as the `full_ci`
  output, and the two steps read that one output).

Ubuntu is unchanged: `cargo test --workspace` runs the whole suite, daemon included, on every
code-affecting pull request, so the platform-independent daemon logic is still gated before merge.
The targeted `Test (daemon Windows process supervision)` step also stays on every pull request.

**The accepted cost:** a break that only macOS or Windows sees (a `cfg(unix)` arm, a path
separator, ConPTY behaviour) is found after the merge. Add `full-ci` to a pull request that edits
platform-specific code in `micold-daemon`.

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
| `build + test (ubuntu-latest)` | **skipped** (its `cargo test --workspace` ran on the PR) |
| `build + test (macos-latest)`, `(windows-latest)` | run, with the full daemon suites |
| `static Linux daemons` | runs (the macOS, Windows and ARM64 packaging steps need its artifact) |
| `package + smoke (windows-11-arm)`, sandbox, sshd | run |
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
request that skipped jobs by path (docs-only, no packaging paths) skipped them on the push too:
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

`build + test` (all three legs), `package + smoke (windows-11-arm)`, `sandbox against a real runtime`
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
  must not, so `test`, the ARM64 package, sandbox and sshd spell their conditions out (lint is not
  `failure`) rather than rely on `success()`.
- **The matrix is gated whole.** `needs:` is per job, not per leg, so the Linux leg waits too. It
  is not the critical path (macOS and Windows are), so green wall-clock is unchanged in practice.
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

## The macOS packaging step

The macOS `.app` bundle is composed and launched inside the **`test` job's macOS leg**, as a step —
not as a job of its own. Look for *"Package and launch the macOS bundle"* in a run; it is guarded by
`if: runner.os == 'macOS'`, so the Linux and Windows legs skip it.

That placement is deliberate and it is about the section above. A new job would have to be added to
`ci complete`'s `needs:`, and `ci_gate_covers_every_job.rs` would fail until it was — which is the
gate working, but it means the packaging check and the merge gate have to be changed together
forever after. As a step it rides a job that is already covered, already has the checkout, and has
already built the binaries the bundle is composed from. No second compile, and no edit to the merge
gate (research R12).

What the step actually proves is worth being precise about, because "the bundle was created" is the
weaker half:

1. `scripts/macos-bundle.sh` composes the bundle from `target/debug` and ad-hoc signs it — the same
   script `mise run app` runs, so a contributor reproduces the CI artifact exactly.
2. The bundled executable is **launched**, and the step waits for the session daemon's endpoint
   (`~/.micold/run/d.sock`) to appear.

Step 2 is the one that catches the failure that matters. The client resolves the daemon as a sibling
of its own `current_exe()`, so a bundle that puts the two binaries anywhere but together looks
perfectly healthy — correct plist, valid signature, right layout by eye — and can never start a
session. Waiting for the endpoint is that constraint asserting itself.

### The drawing-surface escape hatch

A GitHub runner may not be able to give iced a drawing surface at all. That failure says nothing
about packaging — the bundle loaded, dyld resolved every library, the plist parsed — so the step
reports it and passes.

The hatch is matched against the launch log, not the exit status, because iced reports a missing
surface and a missing library through the same code path. It is deliberately narrow: every other way
of dying still fails the step, and a missing library, a rejected signature and a bundle whose daemon
is not beside its client all look nothing like a surface failure. If you see that message in a run,
the packaging half was still checked.

`specs/028-macos-package/quickstart.md` §B states what this step does, and
`crates/micold-core/tests/macos_package_gate.rs` holds that statement and the workflow together, so
the description cannot drift from the step.

## The Windows ARM64 smoke step

*"Install and launch it"* in the `windows-arm64-package` job ran 1064-1075 s (run 39107555048:
07:33:28 to 07:51:23), against 28 s for the same script on the x64 leg of the `test` job. The build
before it is 173 s; the installer itself is a few seconds.

The cause was neither a fixed sleep nor a launch timeout. `scripts/windows-install-smoke.sh` asked
Windows about 25 questions (is this pid alive, does the pipe exist, what is the uninstall key's
version, and so on), each by starting `powershell.exe`, and on `windows-11-arm` every start costs
about 40 s. The log timestamps between consecutive steps of the script are 42-45 s apart whatever
the step does (a registry read, a `Start-Process`, one pipe poll), then 60-130 s where a step asks
two or three.

The script now starts one PowerShell and sends it every question over a pipe (`win` in the script),
so the cost is paid once. The checks, their order and their timeouts are unchanged. Expect the step
to take well under 5 minutes; if a run shows it creeping up, compare the timestamps between the
`==` lines in the log with the numbers above.

## The Windows build cache

Issue 958. The Windows leg of `build + test` is the slowest and the least predictable job of a
code-affecting run, so it sets the run's wall clock.

**Before** (the Windows `build + test` job over the last 10 successful code-affecting CI runs on
`main`, 2026-10-17 to 2026-10-18, from the Actions jobs API): median 1600 s, min 1093 s, max
1839 s, mean 1503 s, standard deviation 241 s. The rust-cache restore step takes 36-77 s. The
daemon test step is the largest part (median 614 s, 406-629 s), then core tests (~160 s) and
`cargo build --workspace` (~110 s). Runs whose post-cache step took 120-250 s saved a new cache
(a miss on the exact key); the rest saved nothing.

**What changed**: a Windows-only step in the `test` job creates a 30 GB dynamic VHDX, formats it as
a Dev Drive (ReFS, Defender in performance mode) and mounts it at the workspace's `target` folder,
before rust-cache runs so the restore lands on it. The step then writes Cargo's `CACHEDIR.TAG` into
the volume root: a fresh volume root has no entries at all (no `.` or `..`), libuv's `scandir` reports
that as ENOENT, and rust-cache's key globbing over the workspace (`**/.cargo/config.toml`) then
failed, so the first run with the drive neither restored nor saved a cache and rebuilt from scratch
(2062 s). A drive letter would not help (an empty `E:\` fails the same way), and a junction from
`target` to a lettered drive was rejected: Git's GNU tar, which `@actions/cache` prefers on Windows,
archives a junction as a symlink, while it walks a volume mount point as a directory.
Because the path is unchanged, `CARGO_TARGET_DIR`,
rust-cache, `target/remote-daemon` and the installer scripts need no change (an absolute
`CARGO_TARGET_DIR` such as `E:/target` was rejected: rust-cache joins it onto the workspace path and
`windows-installer.sh` treats only `/`-prefixed paths as absolute). The rust-cache step also reports
`cache-hit` in the run summary. If the image cannot make a Dev Drive the step warns and the leg
builds in a plain `target`; the run summary says which happened. The ARM64 job is untouched.
sccache was not adopted (a second cache beside rust-cache that does not touch the test steps).

**After the Dev Drive** (16 successful `main` runs after the merge, 2026-10-18 11:20 to 21:00 UTC,
same query): median 1660 s, min 1143 s, max 1816 s, mean 1563 s, standard deviation 242 s. No
change from before. In each run's log the Dev Drive step lists `CACHEDIR.TAG` in `target`,
rust-cache prints `Restored from cache key` (or `No cache found`) with no `ENOENT` annotation, and
its post step saves after a miss; if the hit rate regresses, the mount is the first suspect.

**Defender exclusions** (maintainer's decision, 2026-10-19): a second Windows-only step, before the
Dev Drive, adds Defender exclusion paths for the workspace, `~/.cargo`, `~/.rustup`, `RUNNER_TEMP`
and the tool cache, and exclusion processes for `cargo`, `rustc`, `rustdoc` and `link`. The runner is
discarded after the job, so nothing outside it is affected. The step writes to the run summary
whether real-time protection was on; if it reads `False`, Defender cannot be the cause and the
exclusions do nothing. **After**: 11 `main` runs, median 1809 s, min 1292 s, max 2145 s, standard
deviation 251 s. No gain.

**What the two modes are** (2026-10-20): the daemon test step's ~440 s / ~670 s split is test-profile
compilation, not test time (`Finished test profile ... in 4m 55s` against `8m 27s`). The rust-cache
step now emits a `Windows build cache` notice annotation with `cache-hit`, the restored `target` size,
the `deps` file count, the CPU model and the core count. Seven runs after it restored the identical
cache (exact hit, 9769 MB, 2920 deps files); the one on an Intel Xeon Platinum 8573C took 587 s
for the daemon step, the six on AMD EPYC 7763 took 670-714 s. Later runs ruled the CPU model out
(below). Even on a full hit the daemon's ~150 integration-test binaries recompile on
every run, because rust-cache drops the workspace's own crates before it saves.

**Reverted** (2026-10-20): the median kept rising (1600 s before, 1660 s with the Dev Drive, 1809 s
with Defender exclusions, ~1950 s over the seven runs above), so both steps were removed to see
whether they caused it; the annotation stays. **After**: 14 `main` runs, median 1684 s, min 1254 s,
max 2003 s. The two steps were most likely costing time, and the job is back near its baseline.
All 14 restored the identical cache (exact hit, 9776 MB, 2920 deps files), and AMD EPYC 7763
runners gave both the fastest (434 s) and the slowest (740 s) daemon step, so neither the cache nor
the CPU model explains the spread: it is run-to-run variation of the shared hosted runners.

**Conclusion** (issue 958 closed): no step-level change to the Windows leg reduced its time or
spread. The remaining lever is compile work: the daemon's ~150 integration-test files each build
their own test binary, recompiled on every run. Combining them into a few binaries is tracked in
issue 1175.

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
- **Not split:** the `Test (daemon)` (macOS) and `Test (daemon, Windows)` steps live in the `test`
  job and are untouched.

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
