# Data Model: Attach a Provider's Existing Worktrees and Sessions

All types are new, in `crates/micold-core/src/attach.rs`, serde-serializable, and held only in
memory: nothing here is stored. The one durable effect of attaching is the existing per-project
provenance record (`dir_name` set, `store.rs`).

## AttachableWorktree

| Field | Type | Notes |
|---|---|---|
| `dir_name` | `String` | key of the provenance record; also the MCP ref |
| `path` | `PathBuf` | absolute |
| `branch` | `Option<String>` | `None` for detached |
| `provider` | `Option<AiCli>` | the provider whose store holds sessions at this path, when known |
| `session_count` | `usize` | catalog sessions hosted at this worktree, revealed by attaching |
| `availability` | `Availability` enum: `Attachable`, `Unavailable(Missing \| Invalid)` | from `WorktreeStatus` (`Valid`, `Missing`, `Invalid`); prunable maps to `Missing` |

Built from `reconcile()` output filtered to `classify_owner == Agent`: a worktree already
recorded is not listed (FR-001, FR-004). Rules: only worktrees directly under `.claude/worktrees/`
can be `Agent`, so "outside a provider location" (Out of Scope) is excluded by construction.

## ResumableSession

| Field | Type | Notes |
|---|---|---|
| `id` | `Uuid` | the CLI's own id, same as the catalog `SessionId` after adoption |
| `provider` | `AiCli` | |
| `title` | `Option<String>` | `read_title`, else `read_label`; read for the returned page only |
| `last_activity` | `SystemTime` | transcript or index mtime |
| `target` | `AttachTarget` | where it ran |
| `status` | `ResumableStatus` | |

`AttachTarget` is `Default` or `Worktree { dir_name }`: a session never has "no location"
(Principle V). `ResumableStatus`:

- `Resumable`: its location is the root or a startable catalog worktree.
- `NeedsWorktreeAttach`: its worktree is attachable but not attached (FR-008 attaches it first).
- `Unresumable(UnresumableReason)`: `WorktreeMissing`, `WorktreeInvalid`, `NoLocation` (store
  directory names a worktree git does not list). Never resumed elsewhere (spec edge case).

A session already in the catalog (known id, whatever its worktree) is subtracted and never listed
(FR-007). Sessions the 026 adoption put under a still-hidden worktree are therefore not listed;
they show up when that worktree is attached, and the `AttachableWorktree` row carries
`session_count: usize` (catalog sessions at that worktree) so the user sees what attaching
reveals. `NeedsWorktreeAttach` only applies to a store session not yet in the catalog (recorded
after the last open) whose worktree is attachable.

## DiscoveryReport

`{ worktrees: Vec<AttachableWorktree>, sessions: Vec<ResumableSession>, notes: Vec<DiscoveryNote> }`

`DiscoveryNote { provider: Option<AiCli>, path: Option<PathBuf>, reason: SkipReason }`,
`SkipReason`: `StoreMissing` (informational, not shown as an error), `StoreUnreadable`,
`EntryCorrupt`, `SandboxStoreNotReadable`. The UI shows notes so the user can see why nothing was
found (Story 2 scenario 4); corrupt entries never abort the pass (FR-009, SC-005).

## OfferState (client, `features/attach.rs`)

`{ dismissed: BTreeSet<ProjectPath>, selection: BTreeSet<ItemKey> }`, in memory.
`offer_visible(project) = no_records && !report.is_empty() && !dismissed`. `no_records` = the project has
no provenance record (nothing the user has created, claimed or attached). It deliberately does not
count catalog sessions: feature 026 adopts root and startable-worktree sessions at every open
(before the snapshot), so a lost data directory already has adopted sessions when the client
evaluates the offer. Those adopted sessions are not part of the offer; the offer lists the
attachable worktrees and the not-yet-adopted resumable sessions of the report. A project with
records never gets the offer (FR-012, Story 4 scenario 2).

## AttachOutcome (per target)

`Attached | AlreadyAttached | Refused(reason)` where reasons are `NotAWorktreeOfProject`
(also what another project's checkout gets: it is absent from this repository's `git worktree list`),
`Unavailable`, `IoFailed`.
Attaching changes nothing on a `Refused` target.
