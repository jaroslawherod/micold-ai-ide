# Feature Specification: Clickable links in the terminal

**Feature Branch**: `feat/links-in-terminal-should-be-clickable`

**Created**: 2026-09-14

**Status**: Draft

**Input**: User description: "the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application"

## Why this exists

An AI CLI session is full of links. The agent cites documentation, prints the URL of the pull
request it just opened, points at a CI run that failed, asks the user to sign in at a device-login
page, and — when it declares them as links — names the files it changed. A plain shell does the same: `gh pr create` prints the PR's
address, a dev server prints `http://localhost:5173`, a test runner prints a report path.

Today every one of those is inert text. To follow one the user has to drag-select it exactly — not a
character short, not with the trailing full stop — copy it, switch to a browser, and paste. When the
link wraps across two rows of a narrow pane, selecting it cleanly is harder still. A standalone
terminal emulator stopped asking that of its users long ago, and so did every IDE's embedded
terminal; this one still does.

Some programs go further and emit *declared* hyperlinks: text that reads "PR #312" or a file name,
behind which the program has attached the real address. The terminal already receives that address
alongside the text, then throws the information away at the last step — the user sees the words and
has no way to reach what they point to. Many programs, AI CLIs among them, only declare hyperlinks
when they believe the terminal can show them, and they judge that from how the terminal identifies
itself — which this one does not do in any way they recognise, unless the app happened to inherit
another terminal's identity from wherever it was started.

This feature makes both kinds of link behave as links: recognisable when the pointer is over them,
followable with a deliberate gesture, and opened in whatever the user's system uses for that kind of
address — a browser for a web page, the mail client for an address, the default application for a
document.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Follow a web address printed in the terminal (Priority: P1)

An AI CLI replies "I opened the pull request: https://github.com/acme/app/pull/312." The user moves
the pointer over the address; it is marked as a link and the pointer changes to show it can be
followed. The user activates it and the pull request opens in their default web browser. The full
stop after the address is not part of what opens. The same works in a Regular Terminal, and — for
programs that let the terminal do the wrapping — for an address the terminal wrapped onto a second
row because it did not fit the pane. (AI CLIs that lay out their own text break long lines
themselves; see Edge Cases.)

**Why this priority**: This is the reported problem and by far the most common case: plain web
addresses printed as text by AI CLIs and everyday command-line tools. Shipping only this story
already removes the select-copy-switch-paste routine for the links users meet most.

**Independent Test**: Print a line containing an `https://` address (followed by punctuation) in a
session's terminal, hover it, activate it, and confirm the default browser opens exactly that
address; repeat with the pane narrowed so the terminal wraps the address.

**Acceptance Scenarios**:

1. **Given** a terminal showing `See https://example.com/docs/page.html for details.`, **When** the
   user hovers any character of the address, **Then** exactly the characters of the address (and not
   "See", the space, or the final full stop) are shown as a link and the pointer indicates a link.
2. **Given** that hovered address, **When** the user performs the link gesture (FR-004) on it,
   **Then** the system's default web browser is asked to open `https://example.com/docs/page.html`,
   and no input is sent to the terminal's process.
3. **Given** an address longer than the pane is wide, so the terminal continued it on the next row as
   one soft-wrapped line, **When** the user hovers or activates either row's part, **Then** the whole
   address is treated as one link and the complete address opens.
4. **Given** an address enclosed in brackets or quotes, such as `(https://example.com/a_(b))` or
   `"https://example.com"`, **When** the user activates it, **Then** the enclosing punctuation is not
   part of the link, while brackets that belong to the address itself are kept.
5. **Given** the pointer is over ordinary text or a link, **When** the user presses and drags, or
   double- or triple-clicks, **Then** text selection behaves exactly as it does today and no link
   opens.
6. **Given** the address has scrolled into the terminal's scrollback and the user has scrolled up to
   it, **When** the user activates it, **Then** it opens the same way as on the live screen.

---

### User Story 2 - Follow a hyperlink a program declared behind its text (Priority: P2)

A command-line tool prints "PR #312" (or a file name, or "View run") and attaches the real address to
that text as a declared hyperlink. The user hovers the text; it is marked as a link and the address it
will open is shown to them before they commit, so text that *says* one thing cannot silently open
another. Activating it opens the declared address, not the visible words.

**Why this priority**: Declared hyperlinks are already delivered to the terminal and simply discarded
at display time. They come second because few programs send them to a terminal they do not recognise
(FR-006), and because showing the real target on hover is what makes following them safe.

**Independent Test**: Emit text carrying a declared hyperlink whose visible text differs from its
address, hover it, confirm the true address is displayed, activate it, and confirm that address — not
the visible text — opens.

**Acceptance Scenarios**:

1. **Given** a terminal showing the text `docs` carrying the declared hyperlink
   `https://example.com/manual`, **When** the user hovers `docs`, **Then** it is shown as a link and
   the address `https://example.com/manual` is visible to the user while hovering.
2. **Given** that hovered text, **When** the user activates it, **Then** `https://example.com/manual`
   opens.
3. **Given** two adjacent runs of text with two different declared addresses, **When** the user
   hovers each, **Then** each run is its own link with its own address.
4. **Given** the same declared address on two runs of text separated by undeclared text, **When**
   the user hovers one run, **Then** only that run is marked.
5. **Given** declared-hyperlink text that itself looks like a different web address, **When** the
   user activates it, **Then** the declared address is the one opened, and the hover display showed
   that declared address beforehand.
6. **Given** `ls --hyperlink=always` (or an equivalent tool) run in a Regular Terminal on this
   machine, which declares each file name as a `file://` link carrying this machine's hostname,
   **When** the user activates a listed document's name, **Then** that document opens.

---

### User Story 3 - Open non-web links in their related application (Priority: P3)

The link is not a web page: an email address (`mailto:`) or a local file (`file://`, often a declared
hyperlink behind a file name the agent just wrote). Activating it hands the link to the application
the user's operating system associates with that kind of address — the mail client, the default
application for the document type — as the operating system would if the link were clicked anywhere
else. A file the operating system would *run* rather than open is shown in the file manager instead,
so a link can never start a program.

**Why this priority**: The request explicitly names "other related application", but these links are
rarer than web addresses in terminal output, and they carry the safety rules around what may be
opened.

**Independent Test**: Activate a `mailto:` link and a link to an existing local document, and confirm
each opens in its associated application; activate a link to an executable file and confirm it is
revealed in the file manager and not run.

**Acceptance Scenarios**:

1. **Given** a terminal showing `mailto:team@example.com`, **When** the user activates it, **Then**
   the system's default mail application is asked to open a message to that address.
2. **Given** a `file://` link (plain or declared) to a document that exists on this machine, **When**
   the user activates it, **Then** the operating system's default application for that document opens
   it.
3. **Given** a `file://` link to a folder, **When** the user activates it, **Then** the folder opens
   in the system's file manager.
4. **Given** a `file://` link to a file the operating system would run as a program (an executable, a
   script marked runnable, an application bundle, or a launcher/shortcut file), **When** the user
   activates it, **Then** the file is revealed in the system's file manager and is not run.
5. **Given** a `file://` link to a file that does not exist on this machine, **When** the user
   activates it, **Then** nothing is opened and the user is told the file could not be found.
6. **Given** a link that is not followable under FR-011 or FR-012 — including a `file://` link naming
   some other machine — **When** the user hovers or activates it, **Then** it is not treated as a
   link, nothing is opened, and no network access is made.
7. **Given** a sandboxed session (feature 027) on any platform whose agent prints a `file://` link to
   a document inside the project — with no host, or with the sandbox's own hostname — **When** the
   user activates it, **Then** the user is asked to confirm opening the named host path, and on
   confirming, the same file opens from its location on this machine.

---

### User Story 4 - Copy or open a link from the right-click menu (Priority: P3)

The user wants the address but not to open it now — to paste it into a chat, or into a browser
profile other than the default. They right-click a link and choose to copy its address, which puts
the complete address (unwrapped, without surrounding punctuation) on the clipboard. The same menu
offers to open it.

**Why this priority**: It complements activation rather than enabling anything new on its own, and it
gives a deliberate, two-step way to act on a link whose activation gesture a user finds too easy to
trigger by accident.

**Independent Test**: Right-click a wrapped link, choose the copy action, paste elsewhere, and confirm
the complete address arrived; right-click again and choose open, and confirm it opens.

**Acceptance Scenarios**:

1. **Given** the pointer is over a link, **When** the user opens the terminal's right-click menu,
   **Then** the menu offers to open the link and to copy the link's address, in addition to anything
   it offers today.
2. **Given** that menu, **When** the user chooses to copy the address, **Then** the clipboard holds
   the link's complete address — for a declared hyperlink, the declared address, not its visible text.
3. **Given** the pointer is not over a link, **When** the user opens the right-click menu, **Then**
   the menu offers no link actions.

---

### Edge Cases

- **Accidental activation during selection.** A press that begins a drag selection on a link, or a
  double-/triple-click to select a word or line that contains a link, MUST select rather than open.
- **Click to focus.** A press on a link in an unfocused terminal focuses it as today; it opens the
  link only if the press was the link gesture (FR-004).
- **Mouse-reporting programs.** When the running program has turned on mouse reporting (a full-screen
  editor, a TUI), presses keep reaching the program as today; links are reached with Shift, the
  terminal's existing "the terminal, not the program" override (FR-016).
- **Output moving under the pointer.** New output scrolls the screen, or the program redraws the row,
  between hover and activation. The marking, the displayed address and the link that opens all follow
  what is under the pointer at that moment, never a stale earlier frame.
- **Trailing and enclosing punctuation.** `.`, `,`, `;`, `:`, `!`, `?`, closing quotes, and unbalanced
  closing brackets at the end of a detected plain address are not part of it; balanced brackets
  within the address are.
- **Markdown-style and angle-bracket forms** such as `[text](https://x.y)` and `<https://x.y>`: only
  the address itself is the link.
- **Addresses a program broke across lines itself.** AI CLIs that draw their own layout break every
  line at the pane width with real line breaks — Claude Code does — so any address longer than a row
  reaches the terminal in pieces. Only the first row's piece, and only if it is itself a well-formed
  address, is a link; continuation rows are not. Activating that piece opens a truncated address,
  which is why the address a link will open is always displayed on hover (FR-008) before the user
  commits. The full address is reachable only when the program declares it as a hyperlink (FR-006).
- **Malformed or unsupported addresses** (`https://`, `http://exa mple`, `javascript:…`, `data:…`): not
  detected as links, or — if declared — not followable; nothing opens.
- **Very long addresses** (hundreds of characters, wrapping across many rows): detected and opened in
  full, within the performance bound of SC-005.
- **No handler.** The operating system has no application registered for the address type, or
  launching it fails: the user sees a notification saying the link could not be opened and why, and
  the terminal is unaffected.
- **Sandboxed sessions.** A sandboxed session (feature 027) names files as the sandbox sees them, and
  its programs see the sandbox's hostname, not this machine's. The sandbox shares some locations with
  this machine — for example the project, the sandbox's own home directory, its state directory and
  any shared credential files and folders — each mapped to a host location that is not always the same path (on
  Windows a project path appears under a different prefix; the sandbox's home appears where the
  host's home path maps to inside the sandbox but is really stored in the app's own state). Shared locations can nest — a shared
  credential folder sits inside the sandbox's home — and the most specific one wins. A file link is
  opened from the host location that holds the same file. A path the sandbox does not share with this
  machine (a temporary directory, the container's system folders) is reported as not reachable from
  this machine — never opened as a same-named host file. Because the sandboxed agent can write the
  files it links to, opening one on the host asks for confirmation first (FR-018a).
- **Encoded and platform-specific `file` paths.** A `file` link's path is percent-decoded before use
  (`ls --hyperlink` encodes spaces as `%20`), and on Windows `file:///C:/Users/…` names `C:\Users\…`.
  A network share written as `file://server/share/…` names another host and is not followable
  (FR-012).
- **Filesystems that mark every file executable** (some removable-drive and Windows-partition mounts
  on Linux and macOS): every file there counts as runnable under FR-013, so its links reveal the file
  rather than open it. This errs towards never running anything. A symbolic link is judged by the
  file it points to.
- **Pending opens.** When an open is waiting — for the double-click interval (FR-004) or for the
  sandbox confirmation (FR-018a) — it acts on the link captured at activation. If that link's session
  is closed or its sandbox has stopped before the open completes, nothing opens and the user is told
  why.
- **`localhost` addresses from a sandboxed session.** A dev server started inside the sandbox prints
  `http://localhost:5173`. Activating it opens the host's `localhost` in the host's browser, which
  reaches the sandboxed server only if that port is published to the host; this feature does not
  translate or publish ports.
- **Platforms whose terminal layer drops declared hyperlinks.** If a platform's pseudo-terminal
  removes declared hyperlinks before the terminal sees them (older Windows console hosts may), those
  programs' links degrade to plain text, where FR-001 still recognises any address the text contains.
  Whether each platform passes declared hyperlinks through is verified, not assumed.
- **Hover under a mouse-reporting program.** Without Shift held, the pointer over a link shows no link
  marking, because a click would go to the program; holding Shift shows it.
- **Multiple sessions and panes.** Hover state and activation belong to the pane under the pointer;
  following a link in one session's terminal changes nothing in any other session, and a background
  session's output never opens anything.
- **Repeated activation.** A single activation opens the link exactly once.
- **Cross-platform.** The link gesture, the pointer feedback, revealing files in the file manager and
  opening with the system's default application behave equivalently on Linux, macOS and Windows,
  using each platform's conventions for the gesture (for example its modifier key, if FR-004 chooses
  one), for what counts as a runnable file, and for opening an address.

## Requirements *(mandatory)*

### Functional Requirements

**Recognising links**

- **FR-001**: The terminal MUST recognise plain-text addresses that carry an explicit `http://`,
  `https://`, `mailto:` or `file://` prefix in its visible output and scrollback, in both AI CLI panes
  and Regular Terminal panes. Scheme-less text (`example.com`, `team@example.com`, `src/main.rs:42`)
  MUST NOT be recognised.
- **FR-002**: The terminal MUST recognise text that a program declared as a hyperlink as a link to the
  declared address, independently of what the visible text says.
- **FR-003**: A plain-text address that the terminal soft-wrapped across rows because it did not fit
  the pane's width MUST be recognised as a single link spanning those rows. Text on two rows separated
  by a real line break MUST NOT be joined.
- **FR-004**: The link gesture MUST be [NEEDS CLARIFICATION: which gesture opens a link? (a) a plain
  left click — because double-click word selection and triple-click line selection stay as they are
  (fixed below), a single click cannot be told from the start of a double-click, so opening waits
  until the double-click interval passes with no second press, pointer motion off the link before then
  cancels the pending open, and every link opens after that short delay; or (b) a modifier-click — Ctrl+click on Linux/Windows, Cmd+click on macOS —
  which opens immediately and leaves plain clicks exactly as they are today]. Fixed in either case: a
  link opens on release of a press on the link with no drag motion, and never from a drag selection, a
  double- or triple-click selection, or a middle-click paste. A gesture press on a link starts no
  selection unless the pointer then moves (FR-014). The moment of activation is that release.
- **FR-005**: Trailing sentence punctuation and enclosing quotes or unbalanced brackets MUST be
  excluded from a detected plain-text address; balanced brackets inside it MUST be kept.
- **FR-006**: Programs decide whether to declare hyperlinks from how the terminal identifies itself.
  The common detection rules (as implemented by the `supports-hyperlinks` libraries that Node and Rust
  command-line tools share) recognise only a fixed list of named terminals, or an explicit "force
  hyperlinks" setting. A session's terminal MUST [NEEDS CLARIFICATION: how should sessions advertise
  hyperlink support? (a) not at all — declared links appear only from programs that emit them
  unconditionally or when the user opts in themselves (for example through a program's own setting, or
  by setting the force-hyperlinks variable in the session environment script); plain-text
  addresses (FR-001) carry the rest, but an AI CLI that breaks its own lines (Claude Code does) shows
  any address longer than a row as a first-row piece that opens truncated; (b) set the explicit force-hyperlinks setting, so detecting programs emit declared
  links — including, for some programs, into output piped to a file, where the escape codes then
  appear as clutter; or (c) identify itself as a known hyperlink-capable terminal, which makes
  detecting programs emit links only to the terminal but may also make them assume that terminal's
  other capabilities]. Whatever is chosen, the terminal-identity variables such programs inspect
  (for example `TERM_PROGRAM`, `VTE_VERSION`, `WT_SESSION`) MUST have the same values in every
  session — sandboxed or not, AI CLI or Regular Terminal — whatever environment the app was started
  in, so a program's detection gives the same answer in each; the user guide MUST say what those
  values are.

**Showing links**

- **FR-007**: While the pointer is over a recognised, followable link, every character of that link —
  on every row it spans — MUST be visibly marked as a link (for example underlined), and the pointer
  MUST change to the platform's link pointer. The marking MUST disappear when the pointer leaves it.
  While the running program has mouse reporting on, the marking and pointer MUST appear only while
  Shift is held (FR-016). A declared link is a maximal run of adjacent cells carrying the same address,
  continuing across a soft wrap but not across a real line break; two runs separated by other cells
  are two links, even with the same address.
- **FR-008**: Whenever FR-007 marks a link — detected or declared — the address it will open MUST be
  displayed to the user, and that display MUST change in the same refresh as the link under the
  pointer does. For a `file` link from a sandboxed session, the displayed address is the host path
  after translation (FR-018), or a statement that the path is not reachable from this machine.
- **FR-009**: Link marking MUST remain legible in both the light and dark themes and MUST NOT hide the
  text's colours, the cursor, or a selection highlight.

**Opening links**

- **FR-010**: Activating an `http`, `https` or `mailto` link, a `file` link to a document, or a `file`
  link to a folder MUST ask the operating system to open it with the default application for that
  address type, on Linux, macOS and Windows.
- **FR-011**: Exactly these address types are followable: `http`, `https`, `mailto` and `file`, plus
  [NEEDS CLARIFICATION: may other address types that a program declares — an application's own scheme
  such as `vscode://`, `slack://` or `zoommtg://` — also be opened: (a) never, (b) after a
  confirmation naming the application, or (c) freely like web links?]. Every other address type,
  including `javascript`, `data`, `vbscript` and any type not listed, MUST NOT be followable.
- **FR-012**: A `file` link MUST be followable only when its host part is empty, `localhost`, this
  machine's own hostname, or — for a link printed in a sandboxed session — the sandbox's hostname. A
  `file` link naming any other host MUST NOT be followable, and deciding that MUST NOT touch the
  network.
- **FR-013**: A `file` link to a *runnable* file MUST NOT be opened or run; it MUST instead be revealed
  in the system's file manager — the file selected in its folder where the platform's file manager
  supports that, otherwise (on Linux desktops without that support) its containing folder opened.
  Runnability is decided by a denylist, so that every other document still opens in its associated
  application. A file is runnable when:
  - **Linux**: it has any execute permission bit, or it is a desktop launcher (`.desktop`), an
    AppImage, a `.jar`, or an installer or package (`.deb`, `.rpm`, `.snap`, `.flatpak`,
    `.flatpakref`, `.run`).
  - **macOS**: it has any execute permission bit, or it is an application or other bundle, or its
    type is one that launches or runs something when opened: `.app`, `.command`, `.terminal`, `.tool`,
    `.pkg`, `.mpkg`, `.jar`, `.workflow`, `.action`, `.scpt`, `.applescript`, `.webloc`, `.fileloc`,
    `.inetloc`, `.url`, `.shortcut`.
  - **Windows**: its extension appears in the system's executable-extension list, or it is one of the
    types that run or act through a file association: `.exe`, `.com`, `.bat`, `.cmd`, `.ps1`, `.psm1`,
    `.vbs`, `.vbe`, `.js`, `.jse`, `.wsf`, `.wsh`, `.hta`, `.scr`, `.pif`, `.cpl`, `.msc`, `.msi`,
    `.msp`, `.reg`, `.lnk`, `.url`, `.jar`, `.appref-ms`, `.application`, `.appx`, `.msix`, `.chm`,
    `.inf`, `.scf`, `.settingcontent-ms`, `.library-ms`, `.search-ms`.
  - **Folders**: on macOS a folder that is an application or other bundle counts as runnable; every
    other folder, on every platform, opens in the file manager (FR-010).
- **FR-014**: Activating a link MUST NOT send any input to the terminal's process and MUST NOT change
  the terminal's selection or scroll position. Apart from the confirmation FR-018a requires, it MUST
  NOT change keyboard focus beyond what a press on the terminal already does today.
- **FR-015**: When an address cannot be opened — no registered application, the launch fails, a file
  link names a file that does not exist on this machine, or it names a path a sandboxed session does
  not share with this machine — the user MUST see a notification that names the address and the
  reason, and nothing else MUST happen.
- **FR-016**: When the running program has turned on mouse reporting, a press without Shift MUST
  continue to reach the program exactly as today — including a modifier-click. With Shift held, the
  link gesture (FR-004) on a link MUST open it, and a Shift-drag MUST select as today.
- **FR-017**: For the link gesture, the link that opens MUST be the one under the pointer at the moment
  of activation, as the terminal shows it at that moment. For the right-click menu, it is the link
  that was under the pointer when the menu opened (FR-020).
- **FR-018**: For a sandboxed session, a `file` link's path MUST be translated through the locations
  the sandbox shares with this machine to the host location holding the same file, on every platform,
  using the most specific shared location when they nest. A path in no shared location MUST be
  reported under FR-015 and MUST NOT be opened as a host path of the same name. For a session that is
  not sandboxed the path is used as it is.
- **FR-018a**: Before a `file` link from a sandboxed session is opened or revealed, the user MUST be
  asked to confirm, in a prompt naming the host path that will be opened. Declining opens nothing.
  Links of every other address type from a sandboxed session follow the same rules as any session.
- **FR-019**: Opening a link MUST NOT transmit anything off the device other than what the opened
  application itself does with the address; the terminal MUST NOT fetch, preview or validate an
  address over the network.

**Link actions menu**

- **FR-020**: When the terminal's right-click menu is opened over a link, it MUST offer "Open Link"
  and "Copy Link Address"; over anything else it MUST NOT offer them. Both act on the link captured
  when the menu opened, even if output has since moved it. "Open Link" otherwise follows the same
  rules as the link gesture (FR-010 – FR-016, FR-018, FR-018a, FR-019).
- **FR-021**: "Copy Link Address" MUST put the link's complete address on the clipboard — the declared
  address for a declared hyperlink, and the full unwrapped address without excluded punctuation for a
  plain-text one.

**Isolation and documentation**

- **FR-022**: Hover state and link activation MUST be scoped to the single terminal pane under the
  pointer; no session's output may open a link without the user's gesture in that session's pane.
- **FR-023**: The user guide's terminal section MUST describe how to recognise, open and copy links,
  the gesture on each platform and under mouse-driven programs, what happens to runnable files and to
  file links from sandboxed sessions, how sessions advertise hyperlink support (FR-006), and what
  happens when a link cannot be opened.

### Key Entities

- **Link**: A followable span of terminal text. Has the address it opens, the cells (possibly across
  several soft-wrapped rows) it covers, and its origin — *detected* from plain text or *declared* by
  the program. Exists only as long as the text it covers is on screen or in scrollback; nothing about
  it is persisted.
- **Address type**: The scheme of a link's address (`https`, `mailto`, `file`, …). Decides whether the
  link is followable (FR-011, FR-012) and how it is opened (FR-010, FR-013).
- **Shared location**: For a sandboxed session, a pair of a path as the sandbox sees it and the host
  location that holds the same files. Decides where a `file` link opens from (FR-018).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Following a web address printed in a terminal takes one gesture, down from the current
  select → copy → switch window → paste → Enter routine.
- **SC-002**: On a reference corpus of at least 50 real lines of AI CLI and command-line tool output
  containing addresses (with punctuation, brackets, quotes and soft wrapping), 100% of the addresses
  FR-001 covers — on one row or soft-wrapped — are recognised with exactly the right start and end.
  The corpus also contains scheme-less addresses and addresses a program broke with real line breaks;
  it passes only if no scheme-less text is recognised, a hard-broken address is recognised on its
  first row alone (when that piece is well-formed) and on no continuation row, and the hover display
  for that piece shows exactly the piece that would open.
- **SC-003**: With a modifier-click gesture the operating system receives the open request within
  1 second of release; with a plain-click gesture, within 1 second after the double-click interval
  ends. For a link that needs confirmation (FR-018a) the bound runs from the confirmation. This holds
  on all three platforms.
- **SC-004**: In a scripted run of 100 drag, double-click and triple-click selections that start on
  links, zero links are opened.
- **SC-005**: Hover marking appears or clears in the next rendered frame after the pointer moves.
  While a pane streams 10,000 lines where every line holds an address — including lines of 1,000
  characters wrapping across many rows — the time to render each frame stays within 10% of the same
  output with the addresses replaced by plain words.
- **SC-006**: For every link, the address displayed on hover and the address opened (for a sandboxed
  `file` link, the host path after translation, as also named in its confirmation) are identical in
  100% of tested cases, including when the output under the pointer changes.
- **SC-007**: On each platform, a test set holding one file of every runnable kind FR-013 lists for
  that platform yields 100% revealed and 0% run, and a set of ordinary documents (text, image, PDF,
  HTML) yields 100% opened in their associated applications.

## Assumptions

- The client application runs on the user's own machine, so "the system's default application" means
  the one on the machine showing the terminal — also when the session service runs sandboxed.
- Declared hyperlinks already reach the terminal display intact with their addresses on Linux and
  macOS. On Windows this depends on the pseudo-terminal layer and is verified during planning (see
  Edge Cases).
- Addresses that a program breaks across rows with real line breaks — as AI CLIs that draw their own
  layout do — are not joined: the terminal cannot tell a broken address from two lines of text. When
  such a program emits declared hyperlinks (FR-006), the whole address is still reachable through the
  declared link.
- Scheme-less paths such as `src/main.rs:42` and bare domains or email addresses are not links in this
  feature; they are too easily confused with ordinary text, and a path would need a base directory
  and an editor to open it in.
- No confirmation is asked before opening `http`, `https`, `mailto` or document `file` links from a
  session that is not sandboxed: they open in applications the user chose as their defaults, which is
  what mainstream terminals do. Runnable files are never opened (FR-013) because a declared hyperlink
  can hide one behind harmless text. A sandboxed session's `file` links do ask (FR-018a): feature
  027's promise is that the sandbox cannot reach beyond what it shares, and opening a file the
  sandboxed agent wrote — a page with scripts, a document with macros — in a host application would
  reach past it on nothing more than a click on harmless-looking text.
- Links are recognised in what is displayed; there is no list or history of links, and nothing about
  links is saved.
- Keyboard-only link navigation (moving between links with keys) is out of scope.
- Opening a link inside the application itself (an in-app browser, preview or editor) is out of scope;
  links always open in external applications.
