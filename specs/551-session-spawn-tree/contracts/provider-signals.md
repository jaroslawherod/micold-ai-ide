# Contract: provider origin readers

```text
fn reported_origin(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<Uuid>
```

- Pure read of the provider's own store; no write, no network.
- Returns `Some` only when the provider's own record names the session that started this one.
  Never derived from a title, a path, a prefix or a timestamp (FR-002).
- Default implementation returns `None`.
- A reader is merged only with a recorded fixture of the provider's real output in
  `crates/micold-core/tests/fixtures/<provider>/`, and a test that the same fixture without the
  field yields `None`.
- The signal table in research.md R2 is mirrored by the user guide table (FR-014); a test fails
  when an `AiCli` variant has no row.
