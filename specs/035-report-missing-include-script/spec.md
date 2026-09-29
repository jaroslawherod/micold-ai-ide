# Feature Specification: Report a Missing Environment-Include Script

**Feature Branch**: `fix/github-issues`

**Created**: 2026-09-29

**Status**: Draft

**Input**: Bug report `BUG-006` in feature 011
(`specs/011-env-include-script/bugs/BUG-006.md`), from GitHub issue #435, "An
environment-include script path that does not exist is never reported". The settings file held
`"env_include_script_path": "/tmp/does-not-exist.sh"` with `"env_include_enabled": false`, and
the pair went unnoticed through many launches. Settings showed the stored string and nothing else.
Sessions got the service's own environment, so a CLI that `~/.bashrc` would have put on `PATH` was
missing. Nothing said the script could not be found. The issue's position: "A path that does not
exist is not a working configuration, whether the feature is switched on or off." It leaves three
points open, and this spec leaves them open for clarification (FR-004, FR-007, FR-008).

**Relation to feature 011**: Feature 011 (Closed) owns environment-include. Its FR-013 and SC-006
already require a Settings note when an *attempt* to resolve the script fails, and with the
feature on, a missing script shows "Script not found" today. With the feature off no attempt is
made, and 011's `contracts/settings-ui.md` (layout step 4) says nothing is shown. This spec adds
the behaviour 011 never intended: an opinion about the stored path while the feature is off, and
a consistent report of it in both states. It does not change how 011 sources the script.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See that the stored script path does not exist while the feature is off (Priority: P1)

A user has environment-include switched off, and the stored script path names a file that does
not exist. They open Settings. The environment-include group tells them that the script at that
path cannot be found, and that turning the feature on would not help until the path is fixed.
They can tell this state apart from "switched off, with a script that exists".

**Why this priority**: This is the reported case. Today it is completely silent, and it stood
between the user and a CLI that was installed and working.

**Independent Test**: Store the feature as off with the path `/tmp/does-not-exist.sh` (and
confirm that file is absent). Open Settings. The environment-include group shows a not-found
indication that names the path. Then store an existing file as the path, still off, and open
Settings again: no indication.

**Acceptance Scenarios**:

1. **Given** environment-include is off and the stored path names no existing file, **When** the
   user opens Settings, **Then** the environment-include group shows an indication that the
   script at that path cannot be found, and says the feature is currently off.
2. **Given** environment-include is off and the stored path names an existing file, **When** the
   user opens Settings, **Then** no not-found indication is shown.
3. **Given** environment-include is off and the stored path is blank, **When** the user opens
   Settings, **Then** no not-found indication is shown (a blank path is treated as disabled, as
   in feature 011).
4. **Given** the not-found indication is showing while the feature is off, **When** the user
   launches a session, **Then** the launch proceeds as it does today, with no script sourced and
   no delay.

---

### User Story 2 - The same report whether the feature is on or off (Priority: P2)

A user with the feature on and a missing script already sees "Script not found" in Settings
(feature 011). When they switch the feature off, or back on, the report about the missing path
does not vanish or change its meaning. Either way the page says the same thing about the path,
and whether the feature is on only changes what the page says about its effect on sessions.

**Why this priority**: Without it, switching the feature off hides the problem again, which is
the failure the issue describes. It builds on User Story 1.

**Independent Test**: Store the feature as on with a missing path, open Settings, note the
indication. Switch the feature off and save, reopen Settings: the path is still reported as not
found. Switch it on and save: still reported.

**Acceptance Scenarios**:

1. **Given** environment-include is on and the stored path names no existing file, **When** the
   user opens Settings, **Then** the page reports that the script at that path cannot be found
   (feature 011's failure indication, FR-013 there).
2. **Given** the missing path is reported with the feature on, **When** the user switches the
   feature off and saves, **Then** the page still reports that the script at that path cannot be
   found.
3. **Given** a missing path is reported, **When** the user creates the file at that path and
   opens Settings again, **Then** the not-found indication is gone, in either state. With the
   feature on, the page also says that the last attempt could not find the script, that the file
   exists now, and that saving the settings or restarting a session will source it (FR-014).

---

### User Story 3 - Fix a stored path that has gone missing (Priority: P3)

A user sees that the stored script path does not exist. They can get back to a working
configuration from within the application, without editing any file by hand.

**Why this priority**: Reporting the problem (User Stories 1 and 2) is the fix the issue asks for
first. A guided recovery is a convenience on top, and its shape is not yet decided (FR-008).

**Independent Test**: Store a missing path. From the not-found indication, reach a configuration
whose path exists (or that is deliberately blank), using only the application, and confirm the
indication is gone.

**Acceptance Scenarios**:

1. **Given** the not-found indication is showing, and FR-008's answer offers an in-app recovery,
   **When** the user follows it, **Then** the stored configuration no longer names a missing
   file and the indication is gone. (If FR-008's answer is "report only", this scenario is
   dropped and scenario 2 is the recovery.)
2. **Given** the not-found indication is showing, **When** the user types an existing path
   themselves and saves, **Then** the indication is gone.

---

### Edge Cases

- **Blank path**: no indication, in either state. Feature 011 treats a blank path as disabled.
- **Path names a directory, or a file the user cannot read**: it cannot be sourced, so it is
  reported as "not a readable file". With the feature on, a resolution attempt on such a path
  today ends as a non-zero exit (feature 011), so the page can show both notes: the path
  indication first, then 011's failure note with its own category and captured output. They state
  different facts and do not contradict each other.
- **Path that starts with `~`**: resolution does not expand `~` (feature 011 takes the path
  literally), so neither does the check. A path such as `~/env.sh` is reported as not found, and
  the indication says that `~` is not expanded and a full path is needed.
- **Relative path**: feature 011 sources the script in each session's own directory (FR-020
  there), so a relative path can exist for one project and not another, and no single answer is
  true for every window. A relative path is therefore not checked for existence. It gets its own
  indication: the path is relative, and whether it is found depends on the directory. Every
  window shows this same indication. With the feature on, 011's failure note for the last attempt
  (for example "Script not found" in that session's directory) is shown beside it; the two do not
  contradict each other.
- **The default path is missing**: on a fresh macOS or Windows install the platform default (011
  FR-004: `~/.bashrc`, or the PowerShell profile) often does not exist. It is reported the same
  way as a custom path. The indication stays non-blocking (FR-006), so a user who never needed the
  script is not stopped.
- **The file appears or disappears while the application runs**: the next time Settings is
  shown, the indication reflects the file as it is then (FR-009). A running session is never
  affected (011 FR-016).
- **Several sessions and several open windows** (Principle II): the indication is about the one
  app-level setting, so every window that shows Settings shows the same result for the same
  stored path at the same moment. Checking the path never waits on, or delays, any session.
- **Checking the path fails for a reason other than absence** (for example, a permissions error
  on a parent directory): an error that comes back at once is reported as "not a readable file",
  not silently treated as present. Only a check with no answer within FR-006's bound is reported
  as "could not be checked".
- **The file now exists, but the last attempt could not find it** (feature on): the cached
  resolution still says the script was missing, so new sessions still get no script until the
  next refresh (011 FR-007). The page must not simply look fine: FR-014 applies.
- **Checking the path hangs** (for example, a stalled network mount on Linux or macOS, or an
  unreachable UNC share on Windows): Settings still opens at once, and after the bound in FR-006
  the indication says the path could not be checked.
- **Cross-platform** (Principle VI): the indication behaves the same on Linux, macOS and Windows.
  On Windows, a missing default PowerShell profile is reported with its full path, the same way
  `~/.bashrc` is on Linux and macOS.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When the Settings interface shows the environment-include group and the stored
  script path is not blank, the group MUST show a not-found indication if that path does not name
  a readable file: a regular file (not a directory) that the user can read, as the operating
  system reports it for the current user. This holds whether the
  feature is on or off. The path is taken literally, as resolution takes it: `~` is not expanded.
  A relative path is not checked; it gets the relative-path indication in Edge Cases instead.
- **FR-002**: The not-found indication MUST name the path, MUST say whether the path does not
  exist or is not a readable file, and MUST say whether the feature is currently on or off. While
  the feature is off, it MUST make clear that no script is sourced now and that turning the
  feature on would not source one until the path is fixed.
- **FR-003**: While the feature is off, checking the path MUST NOT execute or source the script,
  and MUST NOT change what sessions receive: no script is sourced, exactly as in feature 011.
- **FR-004**: FR-001 and FR-009 already settle that the *stored* path is checked whenever
  Settings is shown and after every save. For a path the user is editing in Settings, before it
  is saved, the application MUST [NEEDS CLARIFICATION: check the draft path, when, and with what
  effect on saving? Options: do not check the draft (the saved path is reported after save);
  check as the user types; check on save and save a missing path with a warning; check on save
  and refuse to save a missing path until it is fixed or blank. This is what remains of issue
  #435's first open point ("validate when shown or saved") and BUG-006 ledger D3.]
- **FR-005**: With the feature on, the existing failure indication for a missing script (feature
  011, FR-013) MUST remain. Its wording MUST agree with FR-002, so the path is reported as the
  same problem in both states and the indication does not disappear when the feature is switched
  off (User Story 2).
- **FR-006**: The not-found indication MUST be non-blocking. It MUST NOT prevent opening
  Settings, saving the other settings, or launching a session. The check MUST NOT run as part of
  a session launch, and Settings MUST show its content without waiting for the check. If the
  check has no answer within 2 seconds, the indication MUST say the path could not be checked.
- **FR-007**: When a resolution cannot find the script, the application MUST report that
  [NEEDS CLARIFICATION: outside Settings too, and how often? Options: only in Settings (FR-001,
  FR-005); also once per application run in the main window, for example at the first session
  launch; also inside the notice about a missing AI CLI (feature 027 FR-023b), which today does
  not mention that environment-include is off or broken. This is issue #435's second open point
  ("report that once rather than silently falling back") and BUG-006 ledger D4. A report at
  session launch would come from the daemon's per-directory resolution, whose silent failures
  are issue #454; choosing that option overlaps #454's scope.]
- **FR-008**: The application's response to a stored path that has gone missing MUST be
  [NEEDS CLARIFICATION: which? Options: offer to switch to the platform default path (011 FR-004)
  when that file exists; offer to clear the path; offer to turn the feature off; or report only
  and leave the fix to the user's own edit in Settings (User Story 3, scenario 2). This is issue
  #435's third open point and BUG-006 ledger D5.]
- **FR-009**: The indication MUST reflect the path as it is each time Settings is shown, and
  after every save. A path that has become valid MUST no longer be reported. A path that has
  disappeared MUST be reported.
- **FR-010**: The not-found state MUST NOT be persisted. The settings file keeps only the enabled
  flag, the path and the timeout (feature 011 FR-008).
- **FR-011**: A blank stored path MUST NOT produce a not-found indication, in either state.
- **FR-012**: FR-001 to FR-011 and FR-014 MUST hold on Linux, macOS and Windows, for the platform default
  path as well as for a custom path.
- **FR-013**: The user guide's environment-include section MUST describe the not-found
  indication in both states, and what the user can do about it (Principle VII).
- **FR-014**: With the feature on, when the path check finds the file present but the last
  resolution attempt could not find the script (011 FR-013 still reports it), the page MUST say
  that the file exists now and that saving the settings or restarting a session will source it
  (011 FR-007). Showing Settings MUST NOT itself re-source the script or change the cache.

### Key Entities

- **Environment-Include Setting** (feature 011, unchanged): the enabled flag, the script path and
  the timeout. It is the only persisted part.
- **Script Path Check**: the in-memory result of asking whether the stored path (and, if FR-004 says
  so, the draft path) names a readable file: *present*, *not found*, *not a readable file*,
  *relative (not checked)* or *could not be checked*, plus the path it was checked for. It is never
  persisted, and it is independent of the enabled flag. It is not a resolution attempt: it never
  runs the script.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the feature off and a stored path that does not exist, a user who opens
  Settings learns that the script cannot be found in 100% of cases, without reading logs or files.
- **SC-002**: With a stored path that exists, or a blank path, the not-found indication appears
  in 0% of cases, whether the feature is on or off.
- **SC-003**: Switching the feature on or off never hides a not-found report: in both states the
  same missing path is reported.
- **SC-004**: With or without a missing path, a session launch does no path check at all,
  Settings shows its content without waiting for the check, and the indication appears within 2
  seconds of opening Settings.
- **SC-005**: Inspecting the settings file never reveals the not-found state, only the enabled
  flag, the path and the timeout.
- **SC-006**: From seeing the not-found indication, a user reaches a configuration with no
  missing path without editing any file outside the application (at the least by editing the
  path in Settings, plus whatever FR-008 adds).

## Assumptions

- The Settings interface's environment-include group (feature 011, FR-014/FR-015) is the surface
  for the indication. No new notification system is needed for FR-001 to FR-006. Whether one is
  needed for FR-007 is part of that marker.
- "Readable file" is defined in FR-001. It is stricter than feature 011's resolver, which only
  asks whether the path exists; a directory therefore passes the resolver's own check and fails
  later (see Edge Cases). The check runs on the machine where sessions run, which is the user's
  own machine (Principle IV).
- The existing "Script not found" note with the feature on already satisfies part of User Story
  2. This spec keeps it and aligns its wording; it does not replace it.
- How the reported pair got into the settings file (the client's unit tests wrote the real
  settings store) is fixed under feature 029's T069 (PR #410), and is not part of this spec.

## Out of Scope

- Checking the script's contents or syntax, or anything else that needs running it. Failures of
  a script that exists (non-zero exit, timeout) stay as feature 011 defines them.
- The daemon's per-directory resolution failures that are only logged and never reported to a
  client (`crates/micold-daemon/src/state.rs`). That is a defect against 011 as written (FR-013
  with FR-020), filed as GitHub issue #454. See `BUG-006.md`, "Observations".
- Changing when or how the script is sourced, the cache, or the timeout (011 FR-005 to FR-012,
  FR-019 to FR-021).
- Test isolation of the settings store (feature 029, T069).
