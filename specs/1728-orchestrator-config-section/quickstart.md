# Quickstart

1. `mise run test-core` : unit tests for every rule in [contracts/config-check.md](contracts/config-check.md), the leak test and the no-table equivalence test pass.
2. Write a `daemon.toml` with a valid `[orchestrator]` table (url + ca_file + cert_file + key_file), run `micold-daemon config check --config <file>`: exit 0; add `enrollment_token_file`: exit 2 at that key.
3. Add `--files` with a missing `ca_file`: error at the value; `key_file` mode 0644: warning on a host daemon.
4. Open `docs/user-guide/daemon-configuration.md`: the `[orchestrator]` section lists each rule (SC-004).
