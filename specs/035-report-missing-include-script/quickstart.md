# Quickstart: Report a Missing Environment-Include Script

## Part A: automated

```bash
mise run test-core     # core: classify, StdScriptPathProbe on temp files, check_bounded's 2 s bound
mise run gate          # fmt, clippy, the whole workspace (reducer, notice wording, shell triggers)
```

Expected: `crates/micold-core/tests/script_path_check.rs` covers contract rows C1–C8 and P1–P7.
`crates/micold-client/tests/features_settings.rs` covers S1–S8 and N1–N11.
`crates/micold-client/src/main_tests.rs` covers T1–T3, and shows that a session launch makes no
probe call.

## Part B: visual pass (run with the `visual-pass` skill)

Needs a private Xvfb display and a scratch settings store. Seed `settings.json` per step, open the
client and open Settings → Environment. Contract rows are in
[contracts/settings-indication.md](contracts/settings-indication.md).

| Step | Seed (`env_include_enabled`, `env_include_script_path`) | Pass when the Environment page shows |
|---|---|---|
| B1 | `false`, `/tmp/does-not-exist.sh` (absent) | caution `Script not found: /tmp/does-not-exist.sh`, then the OFF note (N2). US1 scenario 1 |
| B2 | `false`, an existing readable file | nothing below the timeout field (N11). US1 scenario 2 |
| B3 | `false`, `""` | nothing below the timeout field (N1/Idle). US1 scenario 3 |
| B4 | `true`, `/tmp/does-not-exist.sh` | one `Script not found: …` caution (not two), then the ON note (N3). US2 scenario 1 |
| B5 | from B4, untick, Save | a notification `The environment-include script was not found: /tmp/does-not-exist.sh`. Reopen: B1's page. US1 scenario 5, US2 scenario 2 |
| B6 | from B4, `touch /tmp/does-not-exist.sh`, reopen Settings | `The last attempt could not find the script`, then `… exists now. Save Settings or restart a session to source it.` (N10). US2 scenario 3 |
| B7 | `false`, `~/env.sh` | the not-found caution, then the TILDE note, then OFF (N5) |
| B8 | `false`, `env.sh` | the REL note only (N8) |
| B9 | from B1, type an existing path, Save, reopen | nothing, and no notification. US3 scenario 1 |
| B10 | from B1, clear the path, Save, reopen | nothing, and no notification. US3 scenario 2 |
| B11 | from B1 (feature off), Save without changes | a notification `The environment-include script was not found: /tmp/does-not-exist.sh`. Reopen: B1's page. US1 scenario 5 |
| B12 | M1 and M2 only (before M3 merges): `true`, `/tmp/does-not-exist.sh` | exactly today's page: 011's single `Script not found` caution (the interim of U63) |

Also check that the cautions and notes use the same style as 011's existing failure note (the
shared `caution`/`note` primitives), in both light and dark themes, and that no line overflows
the page column.

Save screenshots under `specs/035-report-missing-include-script/visual-pass/`.
