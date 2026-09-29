# Research: Create a Worktree from a GitHub Issue

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Each entry records the decision, why, and what was rejected. Paths are repository-relative.

---

## R1 — How GitHub is reached: the GitHub CLI (`gh`), as a subprocess

**Decision.** Issues are read by running the user's installed GitHub CLI, `gh api graphql …`, as a
child process, with `--hostname github.com`. The application never sees a token.

**Rationale.**

- FR-022 asks for "the user's existing GitHub sign-in on the machine" and forbids asking for,
  storing or displaying credentials. `gh` *is* the standard GitHub sign-in on a developer machine
  (`gh auth login`), and it already knows where its token lives on each OS: macOS Keychain,
  Windows Credential Manager, the Secret Service on Linux, or `hosts.yml` when no keyring is
  available, plus `GH_TOKEN`/`GITHUB_TOKEN` when the user set one. Reading any of those stores
  ourselves would mean three platform credential backends and a new way to leak a token (SC-006).
- The workspace has **no HTTP or TLS dependency** today (`crates/*/Cargo.toml`). Adding
  `reqwest`/`ureq` + rustls for one form would be the largest dependency addition in the project,
  against the constitution's "prefer minimal" rule, and still would not solve the credential
  question.
- The pattern is established: `micold_core::git::GitCli` shells out to `git` for every git side
  effect behind a trait with an in-memory fake (research R7 of feature 005). The issue source
  follows it exactly (R7 here).
- `gh` with no sign-in **fails** (exit status 4, "gh auth login") rather than falling back to
  anonymous access, and the GraphQL API has no anonymous tier at all — so FR-022's "no anonymous
  fallback, even for a public repository" holds by construction, not by a check we could forget.
- "GitHub tooling not installed" (spec Edge Cases) is then a precise, distinct condition: the
  executable cannot be found (R3).

**Alternatives rejected.**

| Alternative | Why not |
|---|---|
| HTTP client crate + token read from `gh auth token` | Two mechanisms instead of one; a token in our process memory and our logs' blast radius; a new TLS stack to vet and ship on three OSes. |
| HTTP client crate + OAuth device flow of our own | FR-022 forbids the app implementing its own sign-in; stores a credential. |
| `git credential fill` for `github.com` | Returns whatever the git credential helper holds — often a password or a token scoped for git only; prompts interactively when it has nothing, which a GUI child cannot answer. |
| libgit2 / gitoxide | Neither talks to the GitHub API. |

---

## R2 — GraphQL, not REST, and the search API only for FR-005a

**Decision.** The initial load is the GraphQL `repository.issues(states: OPEN, orderBy: {field:
UPDATED_AT, direction: DESC}, first: 100, after: $cursor)` connection, paged up to 10 times (1,000
issues, FR-004), each node carrying `number`, `title`, `updatedAt` and `labels(first: 20){ name }`,
and the connection's `totalCount`. The beyond-cap search (FR-005a) is one GraphQL request:
`search(type: ISSUE, query: "repo:<o>/<r> is:issue is:open <text>", first: 50)`, and — as a
**second query document**, used only when the typed text is a number (optionally `#`-prefixed) —
the same search plus `repository.issue(number: $n)`. Two documents, because GraphQL rejects a
declared variable that the query does not use ("All Variables Used"). The number lookup is
**partial-response tolerant**: `repository.issue` answers `NOT_FOUND` for a number that does not
exist or belongs to a pull request, `gh` then exits non-zero, and the response still carries
`data.search`. A response whose only error is `NOT_FOUND` at path `["repository","issue"]` is read
as "no such open issue" and its search hits are kept; stdout is parsed whenever it holds `data`,
whatever the exit status.

**Rationale.**

- GraphQL `issues` **excludes pull requests** by type, so FR-004's exclusion needs no filtering,
  and a page of 100 is 100 issues. The REST `/issues` endpoint mixes pull requests into every page,
  so a repository with many open PRs would need an unbounded number of pages to reach 1,000 issues.
- `totalCount` states exactly whether the loaded list is complete, which is what decides whether
  search may contact GitHub (FR-005a: "When the loaded list is complete, search MUST NOT contact
  GitHub"). Inferring it from a short last page cannot tell "exactly 1,000" from "more".
- The search API is rate-limited separately (30 requests/minute) and its index lags behind writes
  by seconds to minutes. Using it for the main list would make a just-opened issue missing and
  retries rate-limited; using it only for the rare beyond-cap search, debounced (R9), keeps both
  risks off the common path.
- Search results that matched only in body or comments are dropped client-side by FR-005's rule
  (spec Clarifications: a beyond-cap hit is held to FR-005's rule); the query text is sent as typed and nothing else about the
  project is (FR-025).
- The numeric lookup is needed because the search API does not match an issue by its number; a
  developer typing `4312` expects issue #4312 even when it is beyond the cap (FR-005's "by number").
- 20 labels per issue: the GitHub UI itself truncates label display well before that; an issue with
  more than 20 labels is matched and typed on its first 20. Recorded as a known bound in
  [contracts/github-issue-source.md](./contracts/github-issue-source.md) §3.

**Alternatives rejected.** REST `/repos/{o}/{r}/issues` (mixes PRs, no total); the search API for
the main list (rate limit, index lag, and a hard 1,000-result ceiling that would make the cap
indistinguishable from "complete"); `gh issue list --json …` (a porcelain command whose flags and
defaults change between `gh` releases, and which caps `--limit` differently across versions —
`gh api graphql` is the stable plumbing, as `git`'s porcelain/plumbing split is for `GitCli`).

---

## R3 — Finding `gh` when the app is launched from the desktop (Principle VI)

**Decision.** The executable is resolved to an absolute path before spawning, by walking, in order
(a `PATH` entry is unquoted on Windows, and a relative entry is never a candidate, since `gh` runs
in the user's home):

1. the `PATH` the user's **environment-include** snapshot contributes for the project root, when it
   contributes one (feature 011: the client sources `~/.bashrc` / the PowerShell profile per
   directory and caches the result in `App::env_include_cache`, keyed by directory). The cache
   usually holds no entry for the project root — it is seeded for `default_resolution_cwd` at boot
   and for restarted sessions' directories only — so the lookup is: the cache entry for the project
   root if present; otherwise resolve it with `env_include::snapshot_for(caps.env_include(), …)`
   using the saved enabled/path/timeout settings, **inside the load's `spawn_blocking`** (bounded by
   the env-include timeout, never on the update loop); the snapshot is returned with the load result
   and the shell inserts it into the cache, so the next load and every search reuse it. The `PATH`
   key is matched case-insensitively (Windows spells it `Path`);
2. this process's own `PATH` (`micold_core::provider::process_path`);
3. a fixed, per-OS list of **well-known install directories**:
   - macOS: `/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`, `~/.local/bin`
   - Linux: `/usr/local/bin`, `/usr/bin`, `/snap/bin`, `/home/linuxbrew/.linuxbrew/bin`,
     `~/.linuxbrew/bin`, `~/.local/bin`, `~/bin`
   - Windows: `%ProgramFiles%\GitHub CLI`, `%ProgramFiles(x86)%\GitHub CLI`,
     `%LOCALAPPDATA%\Microsoft\WinGet\Links`, `%USERPROFILE%\scoop\shims`,
     `%ProgramData%\chocolatey\bin`

The first hit wins. None → `IssueLoadError::ToolMissing`, the "GitHub tooling not installed"
outcome, distinct from "not signed in".

**Rationale.** A Dock-launched macOS app gets `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, so Homebrew's
`gh` is invisible to it even though the user's terminal finds it; a `.desktop`-launched Linux app
often lacks `~/.local/bin`; on Windows the installer puts `gh` on the machine `PATH`, which Explorer
does pass on, but winget "portable" installs and scoop land in per-user shim directories that a
running Explorer may not have picked up. Step 1 reuses the mechanism the app already has for exactly
this problem (the reason feature 011 exists); step 3 covers users who have environment-include off
or whose profile does not set `PATH`.

The walk is a **pure function** over its inputs — the two `PATH` values, a home directory, the
relevant environment variables, an explicit `HostOs` value **and an injected existence probe**
(`&dyn Fn(&Path) -> bool`). The executable name (`gh` / `gh.exe`) and the `PATH` separator (`:` /
`;`) are derived from `HostOs`, not from the host, so every OS's table, separator and file name is
tested on every CI host against a fake probe (Principle VI: "core logic MUST NOT branch on the host
OS directly"; the caller passes `HostOs::current()`, the one `cfg`, and `Path::is_file` as the
probe). `provider::resolves_on_path` is **not** reused for this: it splits with the host's
`split_paths` and reads the host's `PATHEXT`, which would make the Windows table untestable off
Windows.

Once located, the absolute path is returned with the load result and kept by the form for that
load's searches, so a search does not locate again.

**Alternatives rejected.** Spawning `gh` by bare name (fails from the Dock — the edge case the spec
names); spawning a login shell to run `gh` (`bash -lc gh …`: slow, runs the user's profile on every
keystroke search, and does not exist on Windows); asking the user for the path in Settings (a
configuration burden for a problem the app can solve, and not what "found the same way" means).

---

## R4 — Where the fetch runs: in the client, on the host — never in the daemon

**Decision.** The `gh` subprocess runs in `micold-client`, from a `Task::perform`, against the host
machine. The daemon is asked only for the repository's **remote URLs** (R5), which is local git
metadata.

**Rationale.**

- Feature 027 can place the daemon in a container (`PlacementKind::Sandbox`). There, `gh` is not in
  the image, the user's keychain is not reachable, and the sandbox's network and credential-share
  settings (FR-004c of 027) decide what the container may reach. FR-026 requires the issue list to
  work identically whether or not sessions run in a sandbox, and the spec's Edge Cases say "the
  sandbox's network and credential settings neither enable nor block the issue list". Running in the
  client — which always runs on the host, in the user's desktop session, next to the user's sign-in —
  makes that true by placement rather than by configuration.
- The client already runs host subprocesses (`SubprocessResolver` for environment-include,
  `SystemLinkOpener`), behind capabilities assembled in `shell/capabilities.rs`.
- Feature 027's `Capabilities::without_local_git` rule (the client must not run *git* when the
  daemon's filesystem differs) does not apply to `gh api`: it reads nothing from the filesystem, and
  its only input is `owner/name`, which the daemon supplies.

**Alternatives rejected.** Fetch in the daemon, like `BranchList` (breaks under the sandbox, as
above; would also require forwarding the user's GitHub credential into the container — a new
credential share that 027 deliberately makes opt-in); fetch in the client *and* read remotes in the
client (the client may not run git under a Windows-host sandbox — `without_local_git`).

---

## R5 — Finding the repository: a new read-only `RemoteList` RPC

**Decision.** A new correlated request `ClientMsg::RemoteList { req, project }` → the daemon runs
`git -C <repo> config --local --get-regexp ^remote\..+\.url$` through a new `Git::remote_list`
trait method and replies
`OperationResult::RemoteList { remotes: Vec<GitRemote> }` (name + fetch URL). The **choice** of
remote and the parse of the URL into `owner/name` are pure `micold_core::github` functions run in the
client.

**Rationale.**

- FR-002 needs the answer *before* the user chooses the source (the chip is shown disabled with a
  reason), so the lookup happens when the form opens. It is local git metadata, no network, so
  FR-003's "never in the background" (which is about requesting *issues*) is not engaged, and
  Principle IV is intact.
- It mirrors `BranchList` exactly: read-only, correlated, `project`-scoped, and `reject_non_repo`
  for a non-repository — so the client code path, the pending-op bookkeeping and the daemon's
  `spawn_blocking` shape are copies of existing ones, not inventions.
- Remote selection (spec Edge Cases: `origin` when it is a GitHub remote, otherwise the first GitHub
  remote in git's listing order) and URL parsing are decision logic, so they are in `micold-core`
  under test (Principle I), and the daemon ships raw facts only.
- **Repository-local config only, raw URLs.** The answer must not depend on where the daemon runs
  (FR-026). `git remote -v` applies `url.<base>.insteadOf` rewrites, and those usually live in
  `~/.gitconfig`, which reaches a sandboxed daemon only when the user opts into
  `CredentialShare::GitConfig` (027). A remote aliased through `insteadOf` would then be a GitHub
  remote on the host placement and "no GitHub remote" in the sandbox — the sandbox's credential
  settings deciding the issue list, which the spec's Edge Cases forbid. `git config --local` reads
  only the repository's own `.git/config`, which is in the mounted project on either placement, so
  the answer is identical. **Consequence, documented in the user guide:** a remote whose URL is an
  alias (`gh:owner/repo` rewritten by a global `insteadOf`) is not recognised as GitHub.

**Protocol cost.** One `ClientMsg` variant and one `OperationResult` variant are a wire-visible
change: `PROTOCOL_VERSION` goes **15 → 16** in `micold_core::protocol::version` with its doc line,
and the pin in `crates/micold-core/tests/schema_hash.rs` (`FEATURE_026_PROTOCOL_VERSION`'s
successor for this feature) moves with it — the repository's rule is one bump per feature's wire
change (`version.rs`: "MUST be bumped on any wire-visible change"). The changed `SCHEMA_HASH` makes
mismatched client/daemon builds refuse each other at the handshake, as intended.

**URL forms accepted** (host compared case-insensitively; `github.com`, or `ssh.github.com` for
SSH-over-443 — FR-002 rejects Enterprise hosts): `https://[userinfo@]github.com/o/r(.git)(/)`,
`http://…`, `git@github.com:o/r(.git)`, `ssh://git@github.com(:22)/o/r(.git)`,
`ssh://git@ssh.github.com:443/o/r(.git)`, `git://github.com/o/r(.git)`. Userinfo (a user name or a
token-in-URL) is accepted and discarded — never displayed, logged or sent. Anything else —
`www.github.com`, a GHE host, a local path, an unexpanded alias — is "not a GitHub remote".

**Alternatives rejected.** `git remote -v` (applies global `insteadOf` — placement-dependent, above);
reading `.git/config` with our own parser (misses `include` directives git itself resolves);
`gh repo view` (needs network and sign-in just to decide whether to *enable* the chip, and would
contact GitHub before the user opted in — Principle IV).

---

## R6 — Timeouts and killing a hung `gh`

**Decision.** Each `gh` invocation runs under a hard 10-second bound (FR-007: "no answer to any
single request within 10 seconds"). The bounded runner is feature 011's `run_bounded`, promoted from
a private function in `micold_core::env_include` to `pub fn run_bounded` in
`micold_core::process` (where `no_window` already lives), together with its `RunOutcome`,
`kill_process_group` and `JobHandle`. The initial load is up to 10 sequential requests, each
bounded separately.

**One change on the way: drain the pipes while waiting.** Today `run_bounded` polls `try_wait`
until the child exits and only then reads stdout/stderr. A child that writes more than the OS pipe
buffer (64 KiB on Linux, less on macOS/Windows) blocks on its write, never exits, and is killed as a
timeout. Env-include's `env -0` output is a few KiB, so it never mattered; a 100-issue GraphQL page
with labels can be well over that — and would fail on some OSes and not others (Principle VI). The
promoted runner reads stdout and stderr on two reader threads while it polls, joining them after
exit or kill. A test in `process_run_bounded.rs` has a child write 1 MiB to stdout and requires
`Exited` well inside the bound; env-include's existing tests keep holding the old behaviour.

**Rationale.** `run_bounded` already kills the whole process group on Unix and uses a Job Object on
Windows (BUG-003 of feature 011) — `gh` spawns helpers (credential helpers, a pager when misused),
and killing only the direct child leaves them running. Rewriting that for a second caller would
fork a cross-platform subtlety that was already debugged once. The run happens inside
`Task::perform`'s async block via `tokio::task::spawn_blocking`, so the UI never waits.

`gh` is also run with `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `NO_COLOR=1`,
`GH_PAGER=` and `CLICOLOR=0`, so it never blocks on a prompt, never writes an update notice into
stderr (which the classifier reads) and never pages.

**Alternatives rejected.** `tokio::process` + `tokio::time::timeout` (the workspace's tokio has no
`process` feature, and it kills the direct child only); a single 10-second budget for the whole
paged load (FR-007 is per request, and a slow-but-alive 1,000-issue load legitimately takes longer).

---

## R7 — The seam: an `IssueSource` trait with `GhCli` and `FakeIssueSource`

**Decision.** `micold_core::github::IssueSource` with two methods — `list_open(repo, cursor)` → one
page, and `search_open(repo, text)` → matches — and a production `GhCli` (holding the resolved
`gh` path) plus a `FakeIssueSource` for tests. The paging loop, the cap, the PR exclusion guard, the
error classification and the result merge are pure functions *around* the trait, tested without a
subprocess. `GhCli` is chosen once, in `Capabilities::real()`
(`tests/no_concrete_implementations.rs` enforces that).

**Rationale.** The same shape as `Git`/`GitCli`/`FakeGit` and `EnvIncludeResolver`/
`SubprocessResolver`/`FakeEnvIncludeResolver`, which the TDD profile lists as the helpers to reuse.

**Alternatives rejected.** A concrete `GhCli` with no trait (the paging, cap and staleness rules
could then only be tested by running `gh` against GitHub — not in CI, not offline); a closure seam
(`Fn(&[&str]) -> RunOutcome`) at the argv level (tests would assert on GraphQL text rather than on
behaviour, and every test would re-encode JSON fixtures for paging); a trait over raw `RunOutcome`
bytes (moves parsing out of the tested pure layer into the fake).

---

## R8 — Classifying `gh` failures into FR-007's plain-language reasons

**Decision.** A pure `classify(outcome) -> IssueLoadError` over `(spawn error kind, exit status,
stderr, stdout)`:

| Evidence | Error |
|---|---|
| executable not found by R3, or spawn `NotFound` | `ToolMissing` |
| exit 4, or stderr mentions `gh auth login` / "not logged in" / HTTP 401 / "Bad credentials" | `NotSignedIn` |
| GraphQL `NOT_FOUND` / "Could not resolve to a Repository", HTTP 404, HTTP 403 without "rate limit", SAML "Resource protected by organization SAML enforcement", "Resource not accessible by …" (a token without Issues access) | `NoAccess` |
| "API rate limit exceeded", "secondary rate limit", GraphQL `RATE_LIMITED`, HTTP 429 | `RateLimited` |
| "error connecting to", "dial tcp", "no such host", "could not resolve host", "connection refused", "network is unreachable", "TLS handshake timeout", "i/o timeout" | `Offline` |
| `run_bounded` had to kill it | `TimedOut` |
| stdout not parseable as the expected JSON, or anything else | `Other(first stderr line)` |

The patterns are **fixtures** — stderr captured from `gh` 2.x on Linux, macOS and Windows —
committed under `crates/micold-core/tests/fixtures/gh/` and asserted in `github_classify.rs`, so a
wording change in a future `gh` shows up as a failing fixture refresh rather than a silent `Other`.
Unknown text falls to `Other`, which still offers a retry (FR-007), never a crash.

**Alternatives rejected.** `gh auth status` before every load (an extra process and round trip,
and it reports on *all* hosts rather than the one request; the exit status of the real request says
the same thing); exit-status-only classification (`gh api` exits 1 for everything but auth).

---

## R9 — Staleness (FR-007a) and the search debounce

**Decision.** A monotonically increasing `issue_request_seq: u64` lives in the worktree-form
feature's `State`, **outside** `WorktreeForm`, and is never reset. Every load, retry and search is
stamped with the next value; the form records the seq it is waiting for; a result whose seq is not
the awaited one — or that arrives with no form open — is dropped by the reducer. The beyond-cap
search is debounced 300 ms: a keystroke schedules `SearchDue(seq)` after a timer, and the reducer
starts the `gh` request only if `seq` is still the latest.

**Rationale.** A counter inside the form would restart at 0 when the form is reopened, so a result
from the closed form's request 1 could land on the new form's request 1 — the exact collision the
spec's "Form closed while issues load" edge case forbids. The debounce protects the 30/min search
rate limit (R2) without delaying the local results: FR-005's local filtering runs on every keystroke
with no debounce, as feature 021 R11 requires; only the network leg waits.

**Alternatives rejected.** Cancelling the in-flight task (iced `Task` abort handles exist but the
blocking `gh` child would still run to completion; discarding by seq is what 016/021 already do for
branch lists); no debounce (a 10-character query is 10 search requests — a third of the minute's
budget).

---

## R10 — The label-to-type mapping lives in `settings.json`

**Decision.** A new field `issue_label_types: Vec<LabelTypeEntry>` on `micold_core::settings::Settings`,
`#[serde(default = "default_issue_label_types")]`, where each entry is `{ "label": "bug", "type":
"fix" }`. Client-owned: written only by the client's Settings save, through
`SettingsStore::update` (the read-merge-write rule of BUG-025). `SETTINGS_VERSION` stays 4 — an
additive, defaulted field, the precedent set by `default_ai_cli` and `pi_activity_component`.

**Rationale.** FR-016 (application-wide), FR-020 (local filesystem, survives restart) and FR-021
(default until edited) are exactly what `settings.json` already provides for every other global
preference. A file that omits the field reads the default (FR-021); a user who deletes every entry
keeps an empty mapping (their choice), and "Restore defaults" writes the default back (FR-018).
The type serializes as `ConventionalType::as_str()` tokens; an unknown token on read (hand edit)
drops that entry rather than the whole file (the settings-schema contract's forward-compatibility
rule). A per-project mapping later (spec Assumptions) would be an optional override beside this
field, which this shape does not preclude.

**Read at the moment of a pick (FR-014a, SC-005).** The shell reads the stored mapping from the
settings store when it handles a pick and hands it to the reducer inside the message. Nothing
caches it, so a save in Settings takes effect in every open project on the very next pick, with no
restart and no cross-feature state write. The read is synchronous on the update loop — one small
JSON file, the same thing `on_settings_opened` already does — and has one side effect worth
knowing: a corrupt `settings.json` is moved to `.bak` by `load`'s recovery, exactly as it would be
at the next boot or Settings open. A pick on a corrupt file therefore uses `default_mapping()` and
triggers that recovery; this is recorded rather than engineered around.

**`ValidSettings` carries it.** The client's save builds the whole client-owned half from
`ValidSettings::into_settings()` (`features/settings.rs`) and merges it through `update`, so
`ValidSettings` gains `issue_label_types` and `into_settings` copies it. A `features_settings.rs`
case asserts that a save changing only the theme keeps the stored mapping.

**Alternatives rejected.** A separate `issue-mapping.json` (a second settings file with its own
lock, corruption recovery and schema — everything `JsonFileSettingsStore` already does); holding the
mapping in client `State` (a second copy to keep coherent with the file; `feature_write_isolation`
forbids the settings feature writing the worktree-form feature's state).

---

## R11 — Naming from the title: a pure `name_from_title` (FR-010)

**Decision.** `micold_core::naming::name_from_title(title) -> String` returns the text the form's
Name field is set to. It walks the title's words (split on whitespace), keeping the longest prefix
of whole words whose `slugify` is ≤ 50 characters; if even the first word's slug exceeds 50, it
returns the first 50 characters of the full title's slug; if the title slugs to nothing, it returns
`""` (the "name required" validation then applies — spec Edge Cases). The ticket is
`issue.number.to_string()` (FR-009).

**Rationale.** The Name field shows human text ("Crash when opening empty project"), which is what
the user edits (FR-011); `derive` then slugs it exactly as if typed (FR-010, AS4). Cutting on the
*slug's* length is what FR-010 specifies, because the slug is what reaches the directory. It sits in
`naming.rs` beside `slugify` and `derive`, the single source of truth for naming (feature 005,
FR-006a).

**Alternatives rejected.** Filling the Name with the slug (the field would show `crash-when-opening…`,
which the user did not type and which reads as the form editing their input); cutting the title at
50 characters (the slug can be shorter or — after `-wt` Windows-reserved suffixing — longer).

---

## R12 — Matching issues: reuse `micold_core::typeahead` over one row string

**Decision.** Each loaded issue carries a precomputed `row_text` — `#<number> <title>` followed by
`  ·  <label>, <label>` when it has labels — and the picker ranks with
`typeahead::rank(&issues, |i| i.row_text.as_str(), &query)`, the same function and tiers as the
branch picker (FR-005's "literal matches first, then approximate ones"). The row the picker shows is
that same string, so the emphasis spans land on the characters that matched. The keyboard rule is
`typeahead::intent_for`/`move_highlight`, unchanged.

**Rationale.** FR-005 asks for "the existing-branch picker's behaviour (feature 021)", and the
component, `material::Typeahead`, already takes rows of text + spans. Matching the row text matches
number, title and label names in one pass, with no second ranking rule to drift from 021's. The
beyond-cap merge (FR-005a) re-ranks the union with the same call, then drops any searched issue that
has no match — the "same rule as loaded issues" of FR-005a.

**Budget.** 021's `typeahead_budget.rs` holds 500 branch names inside 16 ms in release; issue rows
are longer. A new case **in the existing `crates/micold-core/tests/typeahead_budget.rs`** ranks
1,000 synthetic issue rows and asserts < 50 ms, far inside SC-003's 1 second. Adding it to that file
rather than a new one keeps it in CI's existing release-build step
(`cargo test --release -p micold-core --test typeahead_budget`, 021 BUG-003) with no workflow edit.

**Alternatives rejected.** A separate issue matcher over three fields (duplicates 021's tiers — the
Component-reuse gate's concern applied to logic); matching title only (FR-005 names number and
labels).

---

## R13 — The source switch's disabled state: extend `ToggleChip`

**Decision.** `material::ToggleChip` gains `.disabled(bool)` (no press message, the Material
disabled treatment), and the form places the reason as a muted caption under the switch when the
GitHub chip is disabled (FR-002). The shared component's gallery entry poses the disabled state
(`showcase_completeness.rs` requires every public state to be shown).

**Rationale.** Principle VIII: the switch is already built from `ToggleChip`; a third chip that
cannot be disabled would be forked or faked by swallowing its message. A caption rather than a
tooltip, because the reason must be readable without hovering — and tooltips do not exist on touch
or keyboard-only paths.

**The same caption line carries the opt-in notice (FR-025, Principle IV).** When the chip is
enabled and not yet chosen, the line under the switch reads "GitHub issue reads open issues of
**owner/name** from GitHub." — so the user is told what choosing it contacts *before* the choice
that starts the load. After choosing, the source body repeats it while loading and after.

**Alternatives rejected.** Hiding the chip when unavailable (FR-002 requires it shown, with the
reason); a tooltip-only reason (unreadable from the keyboard, above); a separate `DisabledChip`
widget (a fork of `ToggleChip` — the Component-reuse gate's exact case); an extra "Load issues"
confirmation after choosing the source (a second click for every use, when the notice before the
choice already makes the choice informed).

---

## R14 — The Settings editor: composed from existing primitives

**Decision.** A fifth Settings section, **GitHub issues** (`SettingsSection::GithubIssues`), whose
page is an ordered list of rows, each `TextField` (label) + `Select<ConventionalType>` (type) +
`IconButton`s for move up, move down and remove, followed by **Add entry** and **Restore defaults**
buttons. Validation (FR-019) lives on `SettingsDraft::validate` beside every other field's, reporting
a `FieldError` whose `FieldId::IssueMappingLabel(index)` identifies the offending row.

Icons: three new `Icon` variants — `IssueMapping` (Material Symbols `label`), `MoveUp`
(`arrow_upward`), `MoveDown` (`arrow_downward`) — mapped to glyphs already in the shipped
full-coverage Material Symbols font (`assets/fonts/PROVENANCE.md`); there is no GitHub brand glyph
in that font and none is added.

**Rationale.** Every control already exists in `ui/material/`; reordering by buttons works from the
keyboard and needs no drag-and-drop primitive. The save-together rule of feature 027 (one Save for
all sections) is unchanged.

**Alternatives rejected.** Drag-to-reorder (no shared primitive; would be a new component with its
own gallery, accessibility and three-OS pointer work for a list typically three entries long); a
free-text `label=type` textarea (parsing and error reporting in a text blob; FR-019 wants the
offending *entry* identified).
