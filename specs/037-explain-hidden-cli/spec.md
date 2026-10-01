# Feature Specification: Explain Why an AI CLI Is Not Offered

**Feature Branch**: `fix/github-issues`

**Created**: 2026-10-01

**Status**: Draft

**Input**: GitHub issue #434, "A CLI that is not installed where sessions run is hidden with no
explanation". An AI CLI that cannot be found in the environment sessions are spawned with is absent
from every place a CLI is chosen: the Settings default, the per-session override, and the list
offered at start. The reporter of `specs/029-pi-cli-provider/bugs/BUG-001.md` concluded from the
empty list that the application had no Pi support at all. Their `pi` was installed and working, but
only on the `PATH` that the environment-include script adds. PR #397 fixed *which* CLIs are offered
and deliberately added no explanation. Afterwards the same reporter still had to be told out of
band that their own `env_include_enabled: false` was why Pi stayed hidden. The issue's position: "A
CLI missing for a reason the user can act on — environment-include switched off, a script that
failed or timed out, an install in a directory the login PATH does not carry — should be
distinguishable from one that does not exist on this machine at all."

**Relation to features 026, 027, 029, 011 and 035** (all Closed):

- 026 FR-006 and 029 FR-003/FR-003b decide *whether* a CLI is offered: only when a session started
  in that directory would find it. This spec does not change that rule.
- 027 FR-023b put a note under **Default AI CLI** that names each missing CLI. Under host placement
  it reads "*<names>* isn't installed on this computer, which is where sessions run." Under
  container placement it says the CLI is not in the image. Each sentence states one cause as fact.
  For the BUG-001 reporter it was false: Pi was installed. No closed spec requires the note to tell
  causes apart or to say what would make the CLI appear.
- 011 and 035 report the state of environment-include (off, script not found, exited with an
  error, timed out) in its own Settings group. Nothing links that state to a missing CLI.

This spec adds the behaviour none of them intended: the reason a CLI is not offered, and the action
that would change it, said wherever the application names a missing CLI.

## Clarifications

### Session 2026-10-01

- Q: On a sidebar row where fewer than two CLIs are available, so the row has no chevron (026
  FR-006), where is a CLI that is not offered there explained? → A: Not at the row. The chevron
  rule stays as it is. The reason appears only in a row list that already opens: two or more CLIs
  are available there and another supported one is missing. A row with fewer than two available
  CLIs is unchanged, and that user gets the reason from the Settings note (Story 1) and the start
  and restart messages (Story 2). No closed spec is amended. _(decided by user)_
- Q: On a row with fewer than two available CLIs whose stored default is missing, pressing start
  already opens the row's list although the row has no chevron (033 FR-010). Does that list name
  the CLIs that are not offered? → A: No. FR-010 applies only where two or more CLIs are available.
  On this row the message of FR-008, shown as the list opens, carries the reason for the default,
  and the list itself is as before this feature. When nothing is available there is no list at
  all: the press goes to the session service and the failure of FR-009 carries the reason.
  _(agent-resolved: specs/037-explain-hidden-cli/spec.md#Clarifications (first answer: a row with
  fewer than two available CLIs is unchanged and is served by Story 2);
  specs/033-directory-aware-start-affordance/spec.md#FR-010;
  crates/micold-client/src/ui/sidebar.rs#start_press)_
- Q: Until a row's own availability answer arrives, the row is drawn from the home directory's
  answer (033 FR-005). Which reason does a row surface give in that interval? → A: The reason that
  belongs to the answer in use, which is the home directory's. The offer and the reason always come
  from one answer (FR-012), so a surface never names a CLI as missing without a reason (FR-001) and
  never pairs one answer's offer with another's reason. Both switch to the row's own when its
  answer arrives, with no user action. Opening the row's list asks for that answer (033 FR-004).
  Silence (FR-011) is for a row with no answer in use at all. _(agent-resolved:
  specs/033-directory-aware-start-affordance/spec.md#FR-005;
  crates/micold-client/src/features/session.rs#AvailabilityAnswers::for_dir)_
- Q: Do the limits of FR-002 and the agreement of FR-012 cover the row's CLI list as well as the
  messages? → A: Yes. FR-001 covers every surface of FR-006 to FR-010. SC-003 and SC-004 now name
  the row's CLI list among the surfaces they count. _(agent-resolved:
  specs/037-explain-hidden-cli/spec.md#FR-001)_
- Q: Does a row's CLI list give the action as well as the reason for a CLI it does not offer? →
  A: Yes. FR-001 pairs every reason with its action, FR-003 has every reason name the setting to
  change, and SC-003 already counts the list among the surfaces that must not give "install it" as
  the only action. FR-010, Story 3 and SC-007 now say "reason and action". _(agent-resolved:
  specs/037-explain-hidden-cli/spec.md#FR-001; specs/037-explain-hidden-cli/spec.md#FR-003)_
- Q: The missing-default message, the start failure and the reply to an AI session are each said
  once, at an event. Does one that was already shown change when a newer availability answer
  arrives? → A: No. Each states the reason that held at its event and is not rewritten. The
  missing-default message uses the answer in use for the row at the press. The start failure and
  the reply to an AI session use the environment that start resolved, which is the resolution the
  availability answer shares (029 FR-003b). The next such message uses the answer then in use. Only
  the standing surfaces, the Settings note and an open row list, follow a newer answer.
  _(agent-resolved:
  crates/micold-client/src/features/session.rs#start_menu_toggled (the notice is said only as the
  list opens); specs/029-pi-cli-provider/spec.md#FR-003b;
  crates/micold-daemon/src/state.rs#ai_clis_available_in)_
- Q: A reason can report an attempt made for another directory than the row's: the home
  directory's, while the row is drawn from the home answer. How does the reason stay true? → A: A
  reason that reports the outcome of an attempt (the last four states of FR-001) says which
  directory the attempt was for, as Story 1 scenario 3 ("for my home directory") and Story 2
  scenario 2 ("for its directory") already do. A row drawn from the home answer therefore says the
  home directory's attempt failed, never its own. The first two states are settings, the same for
  every directory, and name none. _(agent-resolved: specs/037-explain-hidden-cli/spec.md#User
  Story 1 (scenario 3); specs/037-explain-hidden-cli/spec.md#Edge Cases (per-directory outcomes
  differ); crates/micold-client/src/features/session.rs#AvailabilityAnswers::for_dir)_
- Q: FR-004a has the "last attempt succeeded" reason name a directory, but FR-005 and Story 2
  scenario 4 keep the container sentence of 027 FR-023b unchanged, and that sentence names no
  directory. Which holds under container placement? → A: FR-005. In the last state under container
  placement the sentence stays as it is today in Settings and at a failed start: it names the CLI
  and the image and no directory. It is a statement about the image, not a report of one
  directory's attempt. FR-004a covers the three failed-attempt states under both placements and
  the last state under host placement only. Story 1 scenario 5 now names the home directory.
  _(agent-resolved: specs/037-explain-hidden-cli/spec.md#FR-005;
  specs/037-explain-hidden-cli/spec.md#User Story 2 (scenario 4);
  specs/027-sandboxed-daemon-runtime/spec.md#FR-023b;
  crates/micold-daemon/src/state.rs#missing_cli_reason;
  crates/micold-client/src/features/settings.rs#missing_cli_notice)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Settings says why a CLI is missing and what would make it appear (Priority: P1)

I installed an AI CLI through a version manager, so it is only on the `PATH` my shell startup file
builds. The application does not offer it. I open Settings to choose it as my default and it is not
in the list. I want the note under the field to tell me the reason that applies to my setup and the
one thing to change, so I do not conclude that the application cannot run this CLI.

**Why this priority**: This is the reported failure. The Settings default is the one place every
user has, whatever number of CLIs they have installed, and it already names the missing CLI. Making
that note truthful and actionable removes the misreading on its own.

**Independent Test**: On a computer where one CLI resolves only through the environment-include
script, open Settings in each environment state (off; on with no script path; script not found;
exited with an error; timed out; on and succeeded). In each state the note under **Default AI CLI**
names the missing CLI, gives the reason for that state and names the action. Turning
environment-include on, saving, and opening Settings again shows the CLI in the selector with no
restart of the application.

**Acceptance Scenarios**:

1. **Given** sessions run on this computer, environment-include is off, and `pi` is not on the
   login `PATH`, **When** I open Settings, **Then** the note under **Default AI CLI** names Pi
   Coding Agent, says sessions get only the login `PATH` because environment-include is off, and
   says that turning **Source a script before each session** on would let a session find a CLI
   that my startup file puts on the `PATH`.
2. **Given** scenario 1, **When** I turn environment-include on with a script that puts `pi` on
   the `PATH`, save (which closes Settings) and open Settings again, **Then** Pi Coding Agent is in
   the selector and the note no longer names it, without restarting the application.
3. **Given** environment-include is on and the last attempt for my home directory timed out,
   **When** I open Settings, **Then** the note names the missing CLI, says the startup script timed
   out for my home directory so its `PATH` additions were not applied, and names the script path
   and timeout fields as what to change. The note is complete without the environment-include
   group's own outcome line, which may be showing another directory.
4. **Given** environment-include is on and the last attempt for my home directory exited with an
   error, or found no readable file at the script path, **When** I open Settings, **Then** the note
   gives that outcome as the reason, in the same terms the environment-include group uses for it.
4a. **Given** environment-include is on and the script path is blank, **When** I open Settings,
   **Then** the note says no script is sourced because no script path is set, and names setting
   one as the action. It does not say to turn environment-include on.
5. **Given** sessions run on this computer, environment-include is on and the last attempt for my
   home directory succeeded, and a CLI is still not found, **When** I open Settings, **Then** the
   note says the CLI was not found on the `PATH` sessions get for my home directory (the login
   `PATH` plus what the script adds) and names both ways out: install it, or make the script add
   its directory.
6. **Given** every CLI the application supports is found, **When** I open Settings, **Then** no
   note is shown.
7. **Given** the session service has not yet answered which CLIs exist, **When** I open Settings,
   **Then** no note is shown, and one appears when the answer arrives if a CLI is missing.

---

### User Story 2 - A failed or refused start gives the same reason (Priority: P2)

My default names a CLI that a session cannot find, or I restart a session whose CLI has gone
missing. The application already tells me which CLI is missing. I want that message to carry the
reason and the action too, so it does not tell me to install something I already installed.

**Why this priority**: These messages appear at the moment the user is blocked. Today they say
"isn't installed. Install it…", which repeats the misleading claim. They are fewer users than
Story 1 because the CLI must have been chosen before it went missing.

**Independent Test**: With a stored default that resolves only through the script, switch
environment-include off and press start on a row. The message that names the missing CLI gives the
environment-include reason and its action, and still offers the available CLIs. Restart a session
of that CLI: the failure in the pane and the banner give the same reason.

**Acceptance Scenarios**:

1. **Given** my default CLI is not found for a row's directory because environment-include is off,
   **When** I press start on that row, **Then** the message names the CLI, gives that reason and
   its action, offers the CLIs that are available, and starts nothing until I choose.
2. **Given** an existing session whose CLI is not found because the startup script timed out for
   its directory, **When** I restart it, **Then** the failure names the CLI and that reason, says
   to fix the script and restart the session, and does not say to install the CLI.
3. **Given** a CLI is not found although the environment resolved in full, **When** a start fails
   for it, **Then** the message keeps today's remedy (install it, or use another CLI for a new
   session) and adds where it was looked for.
4. **Given** sessions run in a container, the environment resolved in full, and the image lacks a
   CLI, **When** a start fails for it, **Then** the message is unchanged from today: it names the
   image and says the image must provide the CLI.
5. **Given** sessions run in a container and environment-include is off or its script was not
   applied, **When** a start fails for a CLI a session would not find, **Then** the message gives
   that environment state as the reason and does not say the image lacks the CLI.
6. **Given** an AI session asks the application to start a session on a CLI that would not be
   found in the target directory, **When** the request is refused, **Then** the reply it receives
   names the CLI and gives the same reason and action a person would be shown for that directory.

---

### User Story 3 - A row's CLI list tells me a CLI is hidden (Priority: P3)

I start sessions from the sidebar and rarely open Settings. A row offers what its own directory
provides, so a CLI can be offered on one project's rows and not on another's. When I open a row's
list of CLIs, I want it to tell me that a CLI is not offered there and why, without first guessing
that Settings holds the answer.

**Why this priority**: It shows per-directory differences that the Settings note, which answers
for the home directory, cannot show. It ranks below Stories 1 and 2 because it reaches only rows
that already have a list to open: a row with fewer than two available CLIs has no chevron (026
FR-006) and stays as it is. That user, the BUG-001 reporter among them, is served by Stories 1
and 2.

**Independent Test**: With three supported CLIs and two projects whose environments differ, so
that one row finds all three and the other finds two, open the per-session CLI choice on each row.
The row that lacks a CLI says so and gives the reason for that row's directory. The other row says
nothing. A third row whose directory provides one CLI has no chevron, as today.

**Acceptance Scenarios**:

1. **Given** a row whose directory provides two or more supported CLIs and does not provide
   another, **When** I open that row's CLI choice, **Then** the list names the CLI that is not
   offered there and gives the reason and its action for that directory.
1a. **Given** a row whose directory provides fewer than two supported CLIs, **When** I look at the
   row, **Then** it has no chevron and shows nothing about missing CLIs, exactly as before this
   feature (026 FR-006).
1b. **Given** a row whose directory provides one supported CLI and my stored default is not that
   one, **When** I press start on the row, **Then** its list opens as before this feature (033
   FR-010) with the message of Story 2 scenario 1, and the list itself says nothing about CLIs
   that are not offered.
2. **Given** a row whose directory provides every supported CLI, **When** I open its CLI choice,
   **Then** nothing about missing CLIs is shown.
3. **Given** two rows whose directories differ in what they provide, **When** I open each row's
   CLI choice, **Then** each gives the answer for its own directory, and opening one never changes
   what the other shows.
4. **Given** a CLI is named as not offered in a row's list, **When** I press it, **Then** nothing
   starts and the list offers no way to choose it: an unavailable CLI can never be chosen by
   accident.

---

### Edge Cases

- **Nothing is available at all.** Every supported CLI is missing. The note names all of them
  with the one reason that applies. No surface is left empty without a sentence. A row in this
  state has no list to open: pressing start asks the session service for the stored default, and
  the failure of FR-009 gives the reason.
- **A row is still drawn from the home directory's answer.** Until a row's own answer arrives it
  uses the home directory's (033 FR-005). Its list and its missing-default message then give the
  home answer's reason, and a reason that reports an attempt says it was the home directory's
  (FR-004a). The list follows the row's own answer as soon as it arrives. A message already shown
  is not rewritten, and the next one uses the row's own answer. This is one answer used in two
  places, not one project's outcome given to another.
- **Not answered yet.** Until an availability answer is in use for a directory, no reason is
  shown. A reason is a finding, and silence before the answer is not one (027 FR-023c).
- **The answer cannot be obtained.** The session service is unreachable or the request fails. The
  application shows no reason about a CLI. It never presents a failure to ask as a missing CLI.
- **Several CLIs are missing.** They share one environment, so they share one reason. The note
  names them together and gives the reason once.
- **Environment-include is off and the stored script path names no file.** Feature 035 reports the
  path. The CLI note gives "off" as the reason and does not repeat the path report.
- **Environment-include is on and the script path is blank.** Features 011 and 035 treat a blank
  path as disabled: no script is sourced although the switch reads on. It has its own row in FR-001
  only because the switch reads on, so "turn it on" would be wrong advice. The reason is "no script
  path is set", never "turn it on" and never "not found".
- **The script path is relative.** Whether it is found depends on each directory. The reason for
  a directory is the outcome of that directory's own attempt, whichever row of FR-001 that is.
- **The script succeeded but changed nothing.** A startup file that adds nothing to the `PATH` is a
  successful resolution. The reason is the "resolved in full, not found" one.
- **Per-directory outcomes differ.** Resolution runs per project directory. The Settings note
  reasons from the home directory's outcome. A row reasons from its own directory's outcome. A
  timed-out script in one project is never given as the reason for another.
- **The environment-include group shows another directory.** Its outcome line reflects the
  directory most recently resolved (feature 011), which need not be the home directory. The CLI
  note therefore states the home directory's outcome itself and never says "see the outcome below".
- **The state changes while Settings is open.** A session restart re-sources the script, or a save
  changes environment-include. The note follows the new state the next time availability is
  answered. It never shows a reason for a state that no longer holds after that answer.
- **A message was shown and the state then changed.** The missing-default message, a start failure
  and a reply to an AI session each state the reason that held when they were said. They are not
  rewritten when a newer answer arrives. The next press, start or request gives the new reason.
- **Several sessions and rows at once (Principle II).** Showing a reason starts nothing, stops
  nothing and changes no running session. Two rows opened in quick succession each keep their own
  answer. Sessions already running a CLI that has gone missing stay listed and labelled.
- **Container placement.** A session in a container gets the container's `PATH` plus what
  environment-include adds there (029 FR-003b applies to both placements). A CLI the image
  installs through a version manager is hidden by the same states as on the host. The reasons of
  FR-001 apply, naming the image as the place. Only when the environment resolved in full does the
  note say the image lacks the CLI, in 027's wording.
- **Cross-platform (Principle VI).** On Windows the default startup script is the PowerShell
  profile and on Linux and macOS it is the bash startup file. The reasons, the actions and the
  setting labels are the same on all three. Only the startup file a reason refers to differs.
- **A CLI that is installed but too old or not signed in.** It is found, so it is offered. This
  feature says nothing about it (029 FR-003a).

## Requirements *(mandatory)*

### Functional Requirements

**The reason**

- **FR-001**: On each surface listed in FR-006 to FR-010, when the application names an AI CLI as
  missing it MUST give exactly one reason, chosen from the state of the environment a session in
  that directory would get. "Login `PATH`" below means the `PATH` the session service itself has:
  the user's login session under host placement, the container's under container placement.

  | State of that environment | Reason given | Action named |
  |---|---|---|
  | Environment-include is off | Sessions get only the login `PATH` | Turn environment-include on, if the startup file puts the CLI on the `PATH`; or install the CLI on the login `PATH` |
  | On, script path is blank | No script is sourced, because no script path is set | Set a script path, if a startup file puts the CLI on the `PATH`; or install the CLI on the login `PATH` |
  | On, script path names no readable file | The startup script could not be read, so its `PATH` additions are not applied | Correct the script path |
  | On, last attempt exited with an error | The startup script failed, so its `PATH` additions are not applied | Fix the script |
  | On, last attempt timed out | The startup script timed out, so its `PATH` additions are not applied | Fix the script or raise the timeout |
  | On, last attempt succeeded | The CLI was not found on the login `PATH` or among what the script adds | Install the CLI, or make the script add its directory |

- **FR-002**: The application MUST NOT state that a CLI "is not installed", or that the image lacks
  it, in the first five states of FR-001 (the states in which no script was applied). In those
  states it does not know, and the sentence MUST say only that a session would not find the CLI and
  why. Only the last state may say the CLI was not found.
- **FR-003**: Every reason MUST name the CLI by its human-readable name (026 FR-006, 029 FR-001)
  and MUST name the setting to change by the label Settings shows for it.
- **FR-004**: When several CLIs are missing for the same directory, the application MUST name them
  together and give the reason once.
- **FR-004a**: A reason that reports the outcome of an attempt (the last four states of FR-001)
  MUST say which directory the attempt was for: the home directory in the Settings note and on a
  row drawn from the home directory's answer (033 FR-005), the row's or session's own directory
  otherwise. The first two states are settings that hold for every directory and name none.
  Under container placement the last state is the exception: the sentence FR-005 keeps names the
  image and no directory.
- **FR-005**: Under container placement FR-001 to FR-004 MUST apply as they do on the host, with
  the image named as the place sessions run. In the last state of FR-001 the wording of 027 FR-023b
  MUST stay as it is: the note names the CLI and the image and says the image must provide it, and
  names no directory (the exception in FR-004a). In
  the first five states the note under *Image reference* and the note under **Default AI CLI** MUST
  give the environment reason instead. This refines 027 FR-023b, which named the image as the cause
  in every state.

**Where it is said**

- **FR-006**: The note under **Default AI CLI** in Settings MUST carry the reason and action of
  FR-001, reasoned for the user's home directory (the directory 029 FR-003b uses for this field).
- **FR-007**: The Settings note MUST be complete by itself: it states the home directory's outcome
  and names the fields to change (the environment-include switch, the script path, the timeout). It
  MUST NOT refer the user to the environment-include group's outcome line as the evidence, because
  that line reflects the directory most recently resolved (feature 011), which may differ. It MUST
  NOT repeat the script's output or the path report of feature 035.
- **FR-008**: The message shown when a start is refused because the stored default is missing
  (026 FR-002) MUST carry the reason and action for the directory of the row being started. It MUST
  still offer the available CLIs and start nothing until the user chooses.
- **FR-009**: The failure shown when a session cannot start or restart because its CLI is not found
  (the pane text and the banner) MUST carry the reason and action for that session's directory,
  taken from the environment that start resolved. In
  the first five states of FR-001 it MUST NOT give installing the CLI as the only action (SC-003),
  and in the three failed-attempt states it MUST NOT name installing at all, as FR-001's table has
  it. A resumed session MUST still never be pointed at another CLI.
- **FR-009a**: The reply an AI session receives when it asks the application to start a session on
  a CLI that would not be found (feature 034) MUST carry the reason and action for the target
  directory, under FR-002's limits.
- **FR-010**: When a sidebar row's per-session CLI list opens and a supported CLI is not offered
  for that row's directory, the list MUST name that CLI and give the reason and action of FR-001
  for that directory. An unavailable CLI MUST NOT be selectable. The rule for when the row has a
  chevron is unchanged (026 FR-006, 033 FR-001): a row with fewer than two available CLIs MUST stay
  without one, and this feature MUST add nothing to that row. That includes the list such a row opens when the stored
  default is missing (033 FR-010): it MUST NOT name the CLIs that are not offered, and the message
  of FR-008 is what carries the reason there.

**When it is said**

- **FR-011**: No reason MUST be shown when every supported CLI is found, when no availability
  answer is in use for that directory yet, or when the answer could not be obtained. A row drawn
  from the home directory's answer while its own is awaited (033 FR-005) has an answer in use.
- **FR-012**: A reason MUST reflect the same availability answer that decides what is offered. For
  one directory and one answer, every surface (FR-006, FR-008, FR-009, FR-009a, FR-010) MUST give the
  same reason. While a row is drawn from the home directory's answer (033 FR-005), the reason it
  gives MUST be that answer's, and MUST change to the row's own together with what is offered.
  The Settings note and an open row list are standing surfaces and MUST follow a newer answer. A
  message said at an event (FR-008, FR-009, FR-009a) MUST state the reason that held at that event
  and MUST NOT be rewritten afterwards. The environment a start resolves is the resolution the
  availability answer shares (029 FR-003b), so a failure and the answer cannot disagree about one
  attempt.
- **FR-013**: When environment-include settings are saved, or a directory's environment is
  resolved again, the reason shown for that directory MUST follow the new state with no restart of
  the application, on the same occasions on which the offered CLIs are refreshed today.
- **FR-014**: Giving a reason MUST NOT cause any run of the startup script that would not happen
  without it, and MUST NOT look for the CLI anywhere other than the environment a session gets.
  The reason is derived only from what the application already knows about that environment when
  it answers availability (SC-006 counts this).
- **FR-015**: Giving a reason MUST NOT change what is offered, the stored default, any setting, or
  any session. The application MUST NOT switch environment-include on, or change the script path,
  on the user's behalf (035 FR-008).

**Everywhere**

- **FR-016**: FR-001 to FR-015 MUST hold on Linux, macOS and Windows. The setting labels are the
  same on all three. Where a reason refers to the startup file, it MUST refer to the one that
  platform sources (the stored script path), not to a file of another platform.
- **FR-017**: The user guide MUST describe the reasons and their actions where it describes the
  **Default AI CLI** note and where it describes a CLI that is not installed, and MUST replace the
  statements that a missing CLI is reported as not installed.

### Key Entities

- **Supported CLI**: An AI CLI the application can run (Claude Code, GitHub Copilot, Pi Coding
  Agent). The full set is known to the application whether or not any is found.
- **Availability answer**: For one directory, which supported CLIs a session started there would
  find. It already exists (027 FR-023c, 029 FR-003b). This feature adds the reason below to it.
- **Unavailability reason**: For one directory, the single state from FR-001 that explains why the
  missing CLIs are missing, with its action. It is not stored. It lasts only as long as the answer
  it belongs to.
- **Environment state**: Whether environment-include is on, and the outcome of the last attempt to
  resolve it for that directory (features 011 and 035).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In each of the six states of FR-001, the Settings note names the missing CLI, the
  reason for that state and its action: six of six states, checked on each platform and under both
  placements.
- **SC-002**: A user in the BUG-001 situation (CLI installed through a version manager,
  environment-include off) gets from "the CLI is not in the list" to "the CLI is saved as the
  default" in at most five interactions after reading the note (turn the switch on, save, open
  Settings again, choose the CLI, save), with no restart of the application and no documentation.
- **SC-003**: In no state where the startup script was not applied does any surface say the CLI is
  not installed, say the image lacks it, or tell the user to install it as the only action: zero
  occurrences across the Settings notes, the missing-default message, the start failure, the
  reply to an AI session and the row's CLI list.
- **SC-004**: For one directory at one moment, all surfaces that name a missing CLI agree on the
  reason: zero disagreements across the surfaces of FR-012, the row's CLI list included.
- **SC-005**: With every supported CLI found, and before the first availability answer, no surface
  shows a reason: zero notes in both cases.
- **SC-006**: Showing a reason causes no additional run of the startup script and no additional
  search of the computer: the number of script runs with the note shown equals the number without.
- **SC-007**: On a row whose directory provides two or more supported CLIs and lacks another, the
  reason and its action for that directory are reachable in one interaction from that row (opening
  its CLI list), without opening Settings. On a row with fewer than two available CLIs the start
  control is the same as before this feature: zero added controls.

## Assumptions

- **No search beyond the session environment.** The application does not look in version-manager
  directories or anywhere else to prove a CLI exists on the computer. It can therefore tell "the
  environment was not fully applied" from "the environment was applied and the CLI is not in it",
  and no more. That is the distinction the issue asks for, drawn from facts the application has.
- **Wording is text only.** The note names the setting to change. It adds no button that changes
  environment-include, because Default AI CLI and environment-include are on the same Settings
  page and 035 FR-008 rejected automatic recovery.
- **The availability rule is unchanged.** What is offered, the launch gate, and the absence of an
  unavailable CLI from the Settings selector stay as 026, 027 and 029 define them. The
  per-session control's "fewer than two" rule (026 FR-006) also stays: this feature never adds a
  chevron to a row.
- **Container placement follows the same rule.** Environment-include applies where sessions run
  under both placements (029 FR-003b), so the same states can hide a CLI the image has. The
  image-naming sentence of 027 FR-023b is kept for the one state in which it is true.
- **The existing reports stay where they are.** Features 011 and 035 keep ownership of the
  environment-include lines. This feature refers to them and does not move or reword them.
- **Home directory for Settings.** The Settings note reasons from the home directory's outcome, as
  the selector does. A difference in one project is shown in that project's start messages, and in
  its rows' CLI lists where a row has one.

## Out of Scope

- Changing which CLIs are offered, or offering a CLI a session would not find.
- Showing a chevron, or any other new control, on a row with fewer than two available CLIs.
- Detecting where a CLI is installed, or which version manager installed it.
- Installing a CLI, or changing environment-include settings for the user.
- Reporting a CLI that is found but too old, not signed in, or otherwise unusable.
- Rewording the environment-include reports of features 011 and 035, or changing which
  directory's outcome the environment-include group shows.
- Making the Settings selector follow an unsaved environment-include change. Availability follows
  saved settings, as today.
