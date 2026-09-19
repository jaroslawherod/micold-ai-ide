# Quickstart: validating untitled session labels

**Feature**: [spec.md](./spec.md) · **Contract**: [first-turn-label.md](./contracts/first-turn-label.md)

## Part A — automated (every milestone)

```bash
mise run test-core          # first_turn_label, copilot_provider, session_name_round_trip, schema_hash
mise run gate               # the whole CI gate, incl. the daemon and client suites below
```

| Test file | Proves |
|---|---|
| `crates/micold-core/tests/first_turn_label.rs` | C2–C5: `claude` and Copilot turn rules, the bound, shaping (FR-002, FR-003, FR-012, FR-014) |
| `crates/micold-core/tests/copilot_provider.rs` | C7: `summary:` read when `name:` is absent; `name:` wins; block scalar ignored (FR-016) |
| `crates/micold-core/tests/session_name_round_trip.rs` | C8.1–C8.3: `Derived` persists as `label`, loads back as `Derived`, a title wins (FR-007, FR-008) |
| `crates/micold-core/tests/schema_hash.rs` | C8.4: protocol 14 |
| `crates/micold-daemon/tests/untitled_session_labels.rs` | C6: discovery, recovery, title replaces label, label never replaces title, per-session isolation, live path within one tick, failed reads/writes change nothing (FR-001, FR-004–FR-006, FR-009–FR-011) |
| `crates/micold-client/tests/session_title_sync.rs` | C8.5: the client adopts `Derived`, then `Named` over it |

## Part B — on the development machine (the milestone that ships the behaviour)

Prerequisites: a build of the branch (`mise run build`), the reporter's project (BUG-001
*Reproduction*) with its five sessions, and Copilot sessions in `~/.copilot/session-state/`.

### B1 — Corpus probe (SC-001, SC-005, SC-008, SC-009)

```bash
MICOLD_LABEL_CORPUS=1 scripts/build-lock.sh cargo test -p micold-core \
  --test first_turn_label_corpus -- --ignored --nocapture
```

It walks the real stores read-only and prints, per provider, how many recorded conversations have a
title, a label, or neither. Pass:

- the four BUG-001 sessions (`9a536c7e`, `2dc1bd13`, `e0912fd9`, `f2e8ef75`) print four different
  labels, `9a536c7e` reading `/speckit-autopilot` (SC-001, SC-005);
- the probe also prints, as evidence only, the rule's result on the 44 unlisted old Copilot sessions
  (a `summary:` title each; a first-turn label each with titles ignored). SC-008 and SC-009 are
  judged on listed sessions by the fixture tests in Part A (`copilot_provider.rs`,
  `untitled_session_labels.rs`), because none of the 44 is listed (D10);
- no conversation with a typed prompt prints "neither" (SC-002).

### B2 — The sidebar after a restart (US1, US2, SC-001, SC-003)

1. Stop the background service and start the app (`mise run run`); open the reporter's project
   without opening any session.
2. Pass: 5 of 5 rows show a name or label; the four untitled ones read four different labels; the
   titled one reads its title (SC-001, SC-003, SC-005). Hover a row: the tooltip shows the same
   label.
3. Close the app, restart the background service, open the project again. Pass: the same labels
   appear with the list, with no "New session" flash (US1 #3).
4. SC-006: open a project with ~50 sessions, including transcripts over 1,000 records, on `main`
   before M1 and on the branch; the list appears no later by eye. The design bound is research R7
   (one ≤1 MiB read per untitled session, once); there is no automated timing test.

### B3 — A running session (US3, FR-010, SC-007)

1. Start a new `claude` session, type a prompt, and stop it from titling (interrupt before the first
   answer completes). Pass: the row shows the prompt's label within 60 s, without reopening.
2. Let the conversation continue until `claude` titles it. Pass: the row switches to the title, and
   after a service restart still shows the title (US2 #2, FR-006).
3. Repeat step 1 with a Copilot session before it writes `name:` (M3).
