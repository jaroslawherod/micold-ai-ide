# Research: Explain Why an AI CLI Is Not Offered

Every Technical Context item was known from the code, so nothing is marked NEEDS CLARIFICATION.
Each decision records what was chosen and what was rejected.

## R1. Where the environment state comes from

**Decision**: The session service keeps the outcome of each directory's environment-include
attempt beside the variables it already caches. `EnvIncludeCell` in
`crates/micold-daemon/src/state.rs` changes from `Arc<OnceLock<Vec<(String, String)>>>` to
`Arc<OnceLock<ResolvedEnv>>`, where the new `ResolvedEnv { vars, env: SpawnEnv }` holds both. A new
`DaemonState::spawn_env_for(cwd) -> ResolvedEnv` does what `env_include_vars_for` does today, and
`env_include_vars_for` becomes `spawn_env_for(cwd).vars`. The two settings states (off, blank
path) are decided in the same short lock that reads the settings today, before any cell is taken.

**Rationale**: Today `env_include_vars_for` logs the outcome and drops it (state.rs:727). The
availability answer and every spawn read that one cell, so storing the outcome there gives the
reason exactly the attempt that decided what is offered (FR-012), and adds no script run (FR-014):
a reason is read from a cell that the availability walk fills anyway.

**Alternatives rejected**:

- A second map `PathBuf → EnvIncludeOutcome`. Two maps can be invalidated apart, and a reason could
  then describe an attempt other than the one whose `PATH` was walked.
- Deriving the state in the client from `App::env_include_last_outcome` (feature 011). That value
  is the outcome for the directory most recently resolved, not for the directory asked about
  (spec Edge Cases, "The environment-include group shows another directory").
- Resolving again when a reason is needed. FR-014 forbids the extra run.

## R2. How the state is classified

**Decision**: A new closed enum `micold_core::cli_reason::SpawnEnv` with six variants, one per row
of FR-001, built by `SpawnEnv::classify(enabled, script_path, attempt)`:

| Input | `SpawnEnv` |
|---|---|
| `enabled == false` | `IncludeOff` |
| enabled, `script_path.trim().is_empty()` | `NoScriptPath` |
| attempt `MissingScript` | `ScriptNotFound` |
| attempt `NonZeroExit { .. }` | `ScriptFailed` |
| attempt `TimedOut { .. }` | `ScriptTimedOut` |
| attempt `Success` | `Applied` |
| enabled, path set, no attempt known (or `Disabled`) | `None` |

The order is the daemon's own: `enabled` is tested first, then the blank path, as
`env_include_vars_for` does today.

**Rationale**: The classification follows the outcome feature 011's resolver already reports. A
path that exists but cannot be read (a directory, a mode-000 file) is sourced and fails, so it is
`ScriptFailed`, the same category the environment-include group shows for it ("Exited with an
error"). That is Story 1 scenario 4's "in the same terms the environment-include group uses", and
it needs no probe of the path (FR-014). The diagnostic text of an outcome is dropped: FR-007
forbids repeating the script's output.

**Alternatives rejected**:

- Putting `EnvIncludeOutcome` itself on the wire. It carries the script's diagnostic output, which
  FR-007 keeps out of the note, and it has no variant for "on, blank path".
- A stat of the script path to tell "unreadable" from "failed". It is a second source of truth
  beside the attempt, and can disagree with it.
- A `bool applied` plus a string. Six states with six actions are an enum (Principle V).

## R3. The wire change

**Decision**: `DaemonMsg::AiCliAvailability` gains `env: Option<SpawnEnv>`. `PROTOCOL_VERSION`
goes 17 → 18 in one edit, with its paragraph in `protocol/version.rs`. `None` means the service
could not say which state holds: the user has no home directory to resolve for, or the blocking
task that resolves it failed. The request is unchanged.

**Rationale**: The answer already blocks on the directory's resolution, so the state is known when
the set is. One message keeps the set and its reason from one attempt (FR-012). A new field makes
an older peer fail to decode, which the version bump turns into the existing "versions differ"
recovery (027).

**Alternatives rejected**:

- A second request for the reason. Two answers can come from two attempts.
- Sending the sentence. The Settings note, the row list and the missing-default message each need
  their own subject and form, and the client holds the image reference (R5).
- `#[serde(default)]` without a bump. `tests/schema_hash.rs` pins the pair, and a peer that
  silently drops the field would show a note with no reason, which FR-001 forbids.
- A seventh `Unknown` variant instead of `Option`. Wording would then need a sentence for a state
  FR-001 does not have. `None` is "the answer could not be obtained" for the reason only, and
  FR-011 already says that shows nothing.

## R4. Where the sentences are written

**Decision**: One pure module, `micold_core::cli_reason`, owns every sentence:
`explain(missing, env, place, dir) -> Option<Explanation>` for the standing surfaces and the reply
to an AI session, and `start_refusal(cli, env, place, dir, launch) -> String` for the failed start
and the missing-default message. The service and the client both call it. `name_list` moves there
from `features/settings.rs`. The three setting labels become `pub const`s there, and
`ui/settings/environment.rs` reads them for its checkbox and field labels.

**Rationale**: The service writes two of the five surfaces (the start failure, the reply to an AI
session) and the client the other three. SC-004 allows zero disagreements, and one function is the
only arrangement in which a disagreement cannot be written. `micold-core` is the one crate both
depend on. A label held as a constant that the Settings page itself renders cannot drift from the
sentence that names it (FR-003).

**Alternatives rejected**:

- Sentences in each crate, held together by a test that compares them. A test can be weakened, and
  the daemon cannot depend on the client to compare.
- The client rewriting the service's failure text. The failure must state the reason that held at
  its event (FR-012), which only the service knows.

## R5. Place and directory

**Decision**: `Place::{ThisComputer, Image(&str)}` and `AttemptDir::{Home, Dir(&Path)}` are
arguments of the wording functions, not wire fields. The client takes the place from
`CliAvailability::source`, stamped at receipt as today (`shell/daemon_sync.rs`
`availability_source`). The service takes it from `MICOLD_IMAGE_REFERENCE`, as
`missing_cli_reason` does today. The client takes the directory from the key the answer was filed
under: `AvailabilityAnswers::answered` stamps a new `CliAvailability::asked_for: AvailabilityKey`.
A row that reads the home answer through `for_dir`'s fallback therefore says "your home directory"
(FR-004a, 033 FR-005). The service passes `Dir(cwd)` for a start and for the reply to an AI
session.

**Rationale**: 027 decided the service does not report the image, because the client already owns
that fact. The same holds for the directory: the client named it in the request.

**Alternatives rejected**:

- Echoing `cwd` in the answer. The home request sends `None`, so the echo could not tell a row
  from Settings.
- Working out the directory at the read site. `for_dir` returns the home answer for a row without
  its own, and the read site cannot tell which it got. Stamping at the filing site can.

## R6. The row's list

**Decision**: `material::MenuOverlay` gains a builder method `note(text)`: a non-interactive block
under the items, after a `Divider`, with wrapped `TypeRole::Label` text in the muted role and the
item padding at both sides. `material::menu_panel_size_with_note(items, note)` gives the size for
anchor clamping, measuring the wrapped note with the paragraph API `material/ellipsized.rs` uses.
`features::session::State::start_menu_note(dir) -> Option<String>` decides the text: `Some` only
when the answer in use for the row offers two or more CLIs, misses at least one and carries an
`env`. The text is `explain`'s reason and action.

**Rationale**: FR-010 needs the CLI named, with a reason and an action, in a list that already
opens, and it must not be choosable. A note has no `on_press`, so it cannot start anything
(Story 3 scenario 4). One note serves several missing CLIs (FR-004). A builder method on the shared
menu is Principle VIII's route. The rule lives in the render-free feature, so it is unit-tested.
`start_menu_note` returns `None` for a row with fewer than two available CLIs, which keeps the list
033 FR-010 opens for a missing default unchanged (D6).

**Alternatives rejected**:

- An inert `MenuItem` (`message: None`) per missing CLI. It names the CLI but has no room for a
  reason: an item is one line of 48dp, and `menu_panel_size` depends on that.
- A tooltip on an inert item. The reason would need a hover, which SC-007's "one interaction"
  does not count, and a keyboard user would not get it.
- A notification when the list opens. It is said at an event and would not follow a newer answer,
  but FR-012 makes an open row list a standing surface.

## R7. FR-009 against FR-001's table

**Decision**: FR-009's sentence "In the first five states of FR-001 it MUST NOT tell the user to
install the CLI" is corrected in spec.md to the rule the rest of the spec states: installing is
never the only action in those states (SC-003), and in the three failed-attempt states it is not
named at all. In the two settings states the failure names it as the alternative FR-001's table
gives.

**Rationale**: FR-001 gives "or install the CLI on the login `PATH`" as part of the action for the
first two states, and FR-012 requires every surface to give the same reason and action. Read
literally, FR-009 made the start failure the one surface that drops half of it. Story 2 scenario 2
("does not say to install the CLI") is a failed-attempt state and stays true. SC-003 already words
the limit as "the only action".

**Alternatives rejected**:

- A second action table for failures. It breaks FR-012 and SC-004.
- Dropping "install" from FR-001's first two rows. A CLI that no startup file provides has no other
  remedy when environment-include is off.

## R8. The missing-default message when the reason is unknown

**Decision**: When the answer in use has `env: None`, the missing-default message says
"*<name>* would not be found by a session here. Start this session on another AI CLI." The
Settings note and the row note show nothing in that case.

**Rationale**: FR-008's message must still explain why a list opened in place of a session (026
BUG-001), so it cannot be silent. It must not claim "isn't installed" without knowing (FR-002).
The sentence states only what the answer says.

**Alternatives rejected**: Keeping today's "isn't installed. Install it…" for that case (FR-002).
Silence (026 BUG-001 is the defect of opening a list without saying why).

## R9. What is not changed

- Which CLIs are offered, when the client asks, and what invalidates the cache (033 contract C1,
  011 FR-007). The reason rides the answers those occasions already produce, which is FR-013.
- The pane text and the banner. They show the service's failure text as it is
  (`ui/terminal.rs` `empty_terminal_message`, `catalog_sync.rs` `announce_start_failures`), so
  FR-009 is met by the service's sentence alone.
- The environment-include group's lines (011, 035).
- `settings.json`. Nothing is persisted (Key Entities: "It is not stored").
