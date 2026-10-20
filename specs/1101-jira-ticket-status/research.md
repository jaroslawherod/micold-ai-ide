# Research: Jira Ticket Status

## R1: Status for the picker rides the existing requests
- **Decision**: add `status` to `FIELDS` in `jira_http.rs`; `parse_page` reads `fields.status.name`
  and `fields.status.statusCategory.key` (`new` | `indeterminate` | `done`).
- **Rationale**: FR-001; zero extra requests; the list already runs.
- **Rejected**: a separate status request per picker load (extra traffic, same data).

## R2: Linked tickets are read by key, in one batched search
- **Decision**: `POST /rest/api/3/search/jql` with `key in ("A-1","A-2",...) AND project = "KEY"`,
  `fields: ["status"]`, `maxResults` = key count, chunks of 100. Keys missing from the answer are
  "not found or not visible" and retire.
- **Rationale**: FR-007, SC-003; reuses the endpoint, auth and parsing already proven (feature 533
  R6); the `project =` clause enforces "only tickets of the connected project" and keeps other
  projects' keys out of the request.
- **Unknown keys**: Jira may answer `400` ("issue ... does not exist") to a `key in` list holding an
  unknown key instead of omitting it. Decided now: on `400` the source bisects the key list (depth
  capped at 7, so at most about 2*log2(n) extra requests, only when a key is bad); a single key
  isolated by `400` is reported as absent (not found). Any other error aborts the read as a failure.
  Both forms (omitted, or `400`) therefore end as "absent" and FR-008 / SC-005 hold. The common case
  (all keys valid) stays one request. `FakeJiraSource` scripts "400 on an unknown key".
- **Rejected**: `GET /issue/{key}` per ticket (N requests, breaks SC-003); `bulkfetch` endpoint
  (POST, 100 keys, but silently drops missing and needs a second field syntax with no gain over the
  search we already parse); reading the picker list (excludes Done, FR-007).

## R3: One pure `StatusBoard` in core state
- **Decision**: a per-project map key -> `StatusEntry`, plus `last_read: Option<Instant-like>` and an
  `in_flight` flag, as a reducer in `micold-core` (`jira_status.rs`). Time is passed in, not read.
- **Rationale**: FR-010 (single source for sidebar, Overview, every window), testable without a GUI
  (Principle I), coalescing (edge case) is the in-flight flag.
- **Rejected**: per-surface caches (can disagree); persisting to disk (stale across restarts, adds
  storage for nothing, Principle IV).

## R4: Triggers
- **Decision**: project open, and `WindowFocusChanged(true)` when `now - last_read > 5 min` (or never
  read). No timer subscription. A trigger while a refresh is in flight is dropped.
- **Rationale**: the clarified FR-005; the existing focus message is already handled in `main.rs`.
- **Rejected**: periodic timer (user chose none); refresh on every focus (needless traffic).

## R5: Failure and retirement
- **Decision**: `Ok` replaces entries; a key absent from an `Ok` answer retires. `Err` marks every
  held entry stale and increments `failures`; the third consecutive `Err` retires all. Switch off or
  connection removed clears the board.
- **Rationale**: FR-008, SC-005; maps 1:1 to the spec's rule; the failure kind is not distinguished
  for display (no dialog), only for retire (not-found is the absent-key case).

## R6: Gate and 533 amendment
- **Decision**: reuse `jira_request(app)` (switch + connection + token + source) as the one gate;
  widen it to `pub(crate)`. Extend `jira_is_requested_only_on_named_events.rs` with the FR-005
  events. Update the Settings/picker disclosure to say keys of linked tickets are sent with the
  project key and credentials with the dialog closed (FR-006b).
- **Rejected**: a second gate (two places to forget the switch).

## R7: Chip as a shared builder over `Tag`
- **Decision**: `ui/material/status_chip.rs`, `StatusChip::new(&TicketStatus).stale(bool)`
  terminating in `.into()`; tint from theme `Roles` per category (To Do: neutral/secondary, In
  Progress: primary, Done: tertiary/success role); stale = reduced opacity.
- **Rationale**: Principle VIII; `Tag` already gives accent chips with theme roles.
- **Accessible text**: iced exposes no accessibility tree, so FR-013 is the chip's `accessible_text()`
  model value (name, plus "may be out of date"), also used as the tooltip; tests assert it there.
- **Rejected**: a chip per surface; colour-only encoding.

## R8: Overview card
- **Decision**: add an optional chip slot to the card builder in `overview_card.rs`, filled from the
  board by the key from `micold_core::naming::ticket` when it is a Jira key.
- **Rejected**: leaving the card out (clarified: included).
