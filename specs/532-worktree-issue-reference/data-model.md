# Data model

No new or changed data. The entities in the spec already exist:

- **Worktree name pair**: `micold_core::naming::DerivedNames` (branch, directory), produced by
  `naming::derive(&WorktreeNaming)`. Unchanged.
- **Issue tag**: `micold_core::naming::Tag::Issue`, read from the directory name by
  `naming::parse_tags`. Unchanged.

The only artifacts that change are three compiled-in strings (contracts/agent-guidance.md).
