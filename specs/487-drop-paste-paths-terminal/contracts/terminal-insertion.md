# Contract: terminal insertion

Internal contracts between core, client and daemon (no wire or protocol change; `PROTOCOL_VERSION` untouched).

1. `path_insert::plan_insertion(paths: &[PathBuf], shell: ShellKind, target: InsertTarget) -> InsertionPlan` — pure except path canonicalisation; order of `accepted` equals input order.
2. `path_insert::quote(shell, &OsStr) -> Result<String, Unrepresentable>` — total for UTF-8 except where the shell cannot represent the name (`Err(Unrepresentable)`); the result, read by `shell`, yields exactly one argument equal to the input.
3. Client `Outcome::Insert { terminal, text }` → `keymap::paste_bytes(text, bracketed)`; never contains a line break outside bracketed-paste markers when the input has none; client never appends `\r`.
4. Notices: `Refusal::message()` → `notify_error`; partial drops also notify (AC US3.3).
5. Daemon: `delete_session` post-condition: `<worktree>/.micold-pasted/<id>` and `<data|state>/pasted/<id>` do not exist; start sweep post-condition: no `pasted/<id>` for a non-live id.
