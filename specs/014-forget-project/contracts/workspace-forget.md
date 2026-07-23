# Contract: `Workspace::forget` (pure core)

Internal Rust API on the `Workspace` aggregate in `src/workspace.rs`. This is the render-free,
unit-tested core operation behind the Forget feature. No I/O.

## Signature

```rust
impl Workspace {
    pub fn forget(&mut self, path: &Path);
}
```

## Preconditions

- `self` is a valid `Workspace` (its invariant holds: `active`, when `Some`, references a `path`
  present in `projects`).
- `path` may be any path spelling; it is canonicalized internally. It need not be a known project.

## Postconditions

Let `key = canonicalize_best_effort(path)`.

1. **Record removed**: no element of `self.projects` has `path == key`. All other elements are
   unchanged and retain their order. *(FR-003)*
2. **Session records dropped**: `self.sessions` contains no `key` entry. Other keys unchanged.
   *(FR-005)*
3. **Worktree-name overrides dropped**: `self.worktree_names` contains no `key` entry. Other keys
   unchanged. *(FR-005)*
4. **Active pointer**: if `self.active == Some(key)` before the call, `self.active == None` after;
   otherwise `self.active` is unchanged. *(FR-008)*
5. **No-op on unknown path**: if no project had `path == key`, `self` is unchanged in all fields.
6. **Availability-independent**: the result is identical whether the removed project's
   `availability` was `Available` or `Unavailable`. *(FR-011)*
7. **Disk untouched**: the function performs no filesystem or process I/O. *(FR-006 — property of
   the pure core; process/FS effects live only in the binary handler.)*

## Invariant after call

`active`, when `Some`, still references a `path` present in `projects` — preserved because the
only way `active` could dangle (removing the active project) is covered by postcondition 4.

## Test obligations (write first — Red)

| # | Scenario | Assertion |
|---|----------|-----------|
| T1 | Forget a non-active known project among several | Its record gone; `active` and all other projects/sessions unchanged. |
| T2 | Forget the active project | Record gone; `active == None`. |
| T3 | Forget the only project | `projects.is_empty()`, `active == None`. |
| T4 | Project had sessions + worktree-name overrides | `sessions[key]` and `worktree_names[key]` both absent afterward. |
| T5 | Forget an `Unavailable` project | Removed identically to an available one. |
| T6 | Forget an unknown / already-forgotten path | `Workspace` unchanged (no panic, no partial mutation). |
| T7 | Path spelled non-canonically (e.g. trailing separator / uncanonicalized) | Still matches and removes the canonical entry. |
