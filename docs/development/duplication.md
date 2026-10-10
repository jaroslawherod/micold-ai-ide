# Duplication budget

`mise run duplication` fails when duplicated code grows past a budget (issue #645). It needs `node`
(pinned in `mise.toml`; jscpd runs through `npx`) and `cargo install similarity-rs`; it does not build the workspace and takes about 30 s.

| Tool | Finds | Budget | Measured 2026-10-10 |
|---|---|---|---|
| `jscpd` (`--min-lines 8 --min-tokens 80`) | exact token clones | 2.4 % duplicated lines | 2.36 % (325 clones, after #698) |
| `similarity-rs` (`-t 0.85 -m 8 --skip-test`) | near-duplicate functions | 750 pairs | 747 pairs |

The budgets are in `mise.toml`, just above the measurement. Lower them when a refactor lowers the
number; never raise one to land a change; extract the shared helper instead.

## Not in `mise run gate`

Left out on purpose: both tools are downloads outside the pinned toolchain, so the gate would stop
being offline and reproducible, and a percentage over 230k lines moves little per PR. Run the task
before a PR that adds a lot of similar code, or from a scheduled CI job. Joining the gate is one line
in `[tasks.gate]` once the tools are pinned in CI.

## Triage

[duplication-triage.md](duplication-triage.md) sorts every known cluster into the five fix types of
issue #645, with lines removable, risk and a rank, one row per future child issue.
