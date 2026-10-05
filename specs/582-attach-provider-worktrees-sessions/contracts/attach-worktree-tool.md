# Contract: `attach_worktree` (MCP)

Mutating, not destructive, audited (`is_mutating_tool`), no confirmation prompt.

**Input**: `{ "worktree": string }`: a `dir_name` from `list_worktrees` (with `include_hidden`), an absolute path, or a branch name. Resolution order: `dir_name`, then path (canonicalized), then branch; the first unique match wins, an ambiguous branch is `invalid_input`.

**Result** (one JSON object): `{ "worktree": ref, "path": string, "branch": string|null, "outcome": "attached" | "already_attached" }`.

**Errors** (`OpError`, nothing changed):
- `refused_by_policy`: caller is a Default session. Text: "a session running in the project root (Default) may not attach worktrees (Constitution Principle III); ask from a session that runs in a worktree".
- `not_found`: matches no worktree of the caller's project (a worktree outside the managed directory that the user has not included is not one), a path outside the project or a worktree outside the managed directory the user has not included, or a worktree of another project (the spec's branch-conflict edge case: attach checks out nothing, so no branch conflict can arise; the other project's checkout is simply not this repository's worktree).
- `invalid_input`: `default`, empty, or an ambiguous branch; an included worktree (one the user listed from outside the managed directory), which is not under a provider location; an unavailable (missing or invalid) worktree, naming why.

**Effects**: writes the provenance record only; `list_worktrees` then returns it with `hidden: false` and every window gets a catalog broadcast. Idempotent and race-safe (FR-016).
