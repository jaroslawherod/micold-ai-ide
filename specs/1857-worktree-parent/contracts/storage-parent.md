# Contract: project state additions

```json
{
  "worktree_ids":     { "feat-1857-x": "<uuid>" },
  "worktree_parents": { "<child-uuid>": { "parent": "default" | { "worktree": "<uuid>" }, "removed": false, "last_name": "feat-1812-y" } }
}
```

Both fields are optional and omitted when empty. Older builds ignore them. A newer build reading an old file mints ids lazily on first catalog refresh and records no parents (FR-003). A project that writes through the legacy catalog fallback keeps the fields empty (see the split-state comments in `store.rs`).
