#1950: feat: `micold-tokens-ts` drift check in the gate (M2)

Part of #1950 (M4a of #1722, FR-007).

- `scripts/tests/tokens-ts-drift.test.sh` regenerates the module to a temp file and `cmp`s it with `apps/client/src/theme/tokens.generated.ts`. A drift or a missing file fails and prints the file path and the regeneration command. A self-test proves an edited copy and a missing file both fail.
- `docs/development/client-theme.md` (linked from `docs-site.md`): regeneration command, drift check, do-not-edit rule.

Verify: `bash scripts/tests/tokens-ts-drift.test.sh`. No user-guide change (`docs-not-needed`, developer tooling).

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvMp1yY3upPDheNSVgG
