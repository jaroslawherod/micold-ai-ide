# BUG-006: sandboxed `claude` sessions keep their conversations (T213, T215)

**Date**: 2026-09-19 · **Runtime**: Docker, x86_64 Linux · **Image**: `micold-daemon:dev`, built from
this branch · **Client**: release build of this branch · **Covers**: SC-012a, US2 scenario 10, FR-004e,
FR-009a

## T213: the real-runtime probe

`crates/micold-daemon/tests/sandbox_real_ai_cli_sessions.rs`:

```
test sandbox_real_ai_cli_sessions_survive_the_sandbox_with_the_sign_in_shared ... claude in the sandbox printed:
$ cd '/tmp/.tmpAXlID8/project' && claude -p 'Reply with the single word hi.' --session-id 244790c0-5
hi
rc=0
ok
test sandbox_real_ai_cli_sessions_survive_the_sandbox_without_the_sign_in ... ok
```

Mutant: restoring the old share (the whole `~/.claude`, read-only) fails the shared probe at the
check that the transcript is under `<state>/sandbox-home`. The host's `.credentials.json` kept its
modification time across the run, so the token was not refreshed.

## T215: quickstart §B with the sign-in shared

The app ran on a private display with private data and runtime directories and the real `HOME`, so
only the host's sign-in token reached the sandbox. `docker ps` showed `micold-sandbox` running, and
the daemon log said `listening (sandboxed)`.

1. **No read-only warning.** `claude` opened on its usual trust prompt for `/tmp/micold-t215/project`
   and nothing else (`bug-006-startup.png`).
2. **A real conversation.** `Reply with the single word hi.` was answered `hi`.
3. **It survives a restart.** The app was closed, its process confirmed gone, and relaunched. The
   session was listed and resumed with the exchange on screen (`bug-006-resumed-session.png`). The
   daemon log for the relaunch has no `pruned empty sessions` line.
4. **The transcript is in the sandbox's home**:
   `<state>/sandbox-home/.claude/projects/-tmp-micold-t215-project/dd737e59-ee7e-4c31-bc54-7b8cbe269e1d.jsonl`
   (23,555 bytes).
5. **The host's `~/.claude` has no record of it.** `~/.claude/projects/-tmp-micold-t215-project`
   does not exist.

## Found on the way: first run asks to sign in

A fresh sandbox home has no `~/.claude.json`, so the first interactive `claude` walks through its
own onboarding. That includes a "Select login method" step with an OAuth link, although
`claude auth status` in the same container reports `"loggedIn": true` from the shared token. With
`hasCompletedOnboarding` set in the sandbox's own `~/.claude.json`, `claude` starts signed in
("Claude Max") and shows only the trust prompt. That is the state step 1 above ran from.
`claude -p` skips onboarding, which is why T213 never saw it. This is outside BUG-006's scope
and is reported rather than fixed here.

## Harness notes

- The seeded `settings.json` needs `settings_version`. Without it the store treats the file as
  corrupt and falls back to defaults, which means the host placement. The first attempt ran
  unsandboxed for this reason and is not evidence.
- A project under `/tmp/claude-1000` makes the runtime create that directory root-owned inside
  the container, and `claude` then refuses to use it as its temp directory. The project was moved
  to `/tmp/micold-t215/project`.
- The app was launched with the calling agent's `CLAUDE_CODE_*` variables unset, so the host
  `claude` would not inherit them.
