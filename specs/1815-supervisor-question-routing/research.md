# Research: supervisor question routing (#1815)

## D1. Where the script/skill line falls
- **Decision**: scripts decide everything computable from fixtures (category gate, batch order and
  size, completeness, state transitions, upward target, overdue); the skill does the effects and the
  one judgment: does the subject match a *Never ask* item and does the repo answer it.
- **Rationale**: same split as #1813 (FR-010 there); the judgment needs repo reading, a script cannot.
- **Rejected**: a keyword classifier for subjects in bash (brittle, would answer a real question
  that merely mentions clippy); doing batching in prose only (not testable).

## D2. Where routing state lives
- **Decision**: a `## Routing` table in `.autopilot/supervisor.md`, keys `R<issue>.Q<n>`, written
  only by `supervisor.sh route`.
- **Rationale**: FR-011 and FR-010 need it in the supervisor ledger; a non-numeric key keeps
  `rows()` (matches `^ *[0-9]+ *$` in column 2) from reading Routing rows as issue rows, so the
  #1813 commands are untouched.
- **Rejected**: a new `Routing` column on the issue table (shifts `col()` indexes of every existing
  ledger and `ledger set`); a sidecar file (second thing to lose; `check-owner` covers one file);
  storing it in each worker's ledger (the supervisor must not edit worker ledgers).

## D3. Re-asking from the ledger
- **Decision**: a Routing row stores question, options, recommendation and evidence as single-line
  cells, so a restarted supervisor rebuilds the `AskUserQuestion` from the ledger, then drops rows
  whose run is no longer `asking` per `run-status.sh --json` (FR-009).
- **Rationale**: US3.2 forbids re-reading transcripts; run-status JSON is not a transcript but is
  only needed to drop answered rows.
- **Rejected**: re-collecting from run-status on every restart (loses `asked` vs `pending`, so it
  could ask twice, violating FR-011).

## D4. Evidence in the worker's Decisions row
- **Decision**: no new flag. The supervisor's answer text already names the evidence (FR-002); it
  lands in the Answer cell, and the Evidence cell reads `answered <date>` as today. Only `--by`
  is added (FR-005a).
- **Rejected**: `--evidence` flag (more surface, duplicates the answer text).

## D5. Mixed batch escalation
- **Decision**: when a batch escalation has some questions resolved by the supervisor and some by the
  user, the held resolved answers join the user's lines and the send is `resume <answer>` (By
  `user`); `--by supervisor` only when every question was resolved by the supervisor.
- **Rationale**: one `answer.sh` call writes one By for all rows; the user decided part of it.
- **Rejected**: two `resume` messages (the second would be `ignored: no open escalation`).

## D6. Cap of 4 and ordering
- **Decision**: `route next` emits up to 4 `pending` question rows in ascending issue then question
  number; a batch escalation never splits across calls where it fits (all its questions or none; a
  single issue with more than 4 questions goes alone, 4 at a time).
- **Rationale**: `AskUserQuestion` takes up to 4; splitting one run's questions across prompts risks
  a partial answer that FR-008a would hold anyway.
- **Rejected**: strict question-level packing (splits a run's batch needlessly).

## D7. Upward target source
- **Decision**: `route target --sessions <list_sessions JSON file> --me <id>` reads only the
  `parent` field of the row whose id is `--me`; absent, null or empty prints `user` (FR-012).
- **Rationale**: no dependency on #1873; works unchanged when the field arrives.
- **Rejected**: inferring the parent from the ledger Owner or env (FR-012 forbids).

## D8. Notification ownership
- **Decision**: `tasks/ask.md` step 0, supervised: banner to the transcript, no `PushNotification`;
  the supervisor sends the one notification per batch (FR-006a, SC-001).
- **Rejected**: suppressing in the supervisor (cannot, it does not control the worker).

## D9. Rows stay `asked` across an over-cap stop
- **Decision**: rows become `asked` when banners and notification go out; the OVER path ends the turn
  there. `route next --reask` returns them on restart, so the prompt is made once and the notification
  is never repeated (SC-001).
- **Rejected**: leaving rows `pending` (would resend banner and notification); a separate `prompted`
  flag (a second state dimension for no gain: re-asking an already-asked row is harmless).

## D10. Pipe handling in Routing cells
- **Decision**: store `|` as `/`; existing readers split on `|` and cannot honour `\|`.
- **Rejected**: `\|` as in `answer.sh` (that file's cells are only read by humans and `answer.sh`'s own awk, which does not re-split rows).
