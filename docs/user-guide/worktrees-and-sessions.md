# Worktrees & Sessions

Micold AI IDE organizes your work into **worktrees** (isolated git branches checked out under
your project) and **sessions** (interactive AI CLI runs inside a worktree). The left sidebar
shows worktrees at the top level; selecting a worktree opens its main session, and a worktree
with several sessions expands to list them. The right side hosts the embedded terminal for the
active session. The sidebar also always shows one **Default** entry —
a session location that isn't a worktree at all, for work you don't want to isolate onto its own
branch (see [The "Default" entry](#the-default-entry-sessions-without-a-worktree) below).

## Opening a project (git repositories only)

Open a project with **Open a project** (empty state) or **Open another project** (app bar), then
choose a folder.

- Only **git repositories** can be opened as projects. If you choose a folder that is not a git
  repository, opening is refused with a message and nothing is opened.
- Once opened, the project becomes the active context and the sidebar lists its worktrees.

## Browsing worktrees

- Worktrees discovered under `.claude/worktrees/` appear as top-level items in the sidebar.
- Selecting a worktree opens its **main session** in the terminal, whichever session you looked at
  last. A worktree with one session has no toggle and no sub-rows; one with several has a toggle
  (collapsed by default) that lists the main session first and marks each helper "helper". Rename
  or close the only session from the worktree row's context menu. A worktree with no session shows
  its start action instead.
- A worktree whose directory was deleted outside the app, or that is not a valid git worktree,
  shows its name in the error color with a **missing** or **invalid** status tag — you cannot
  start new sessions on it until it is resolved.
- A worktree (and the Default entry) holds **at most one session** by default. The limit is soft:
  asking for a second one without saying so on purpose — from the app or from an assistant's
  `create_session` tool — is refused with a message naming the session that is already there, how
  to bring it back (`start_session` on its id) and how to ask for another, and nothing is created.
  An assistant asks for another on purpose with `allow_additional_session: true` on
  `create_session`; the existing session is left as it is. A session you closed (archived) no
  longer counts, so the worktree can take a new one.
  In the app you are asked rather than refused: starting a session where one already exists (even
  from a start button that was drawn before the session appeared) opens a confirmation that names
  the worktree and its session. **Start** creates the second session with the CLI you chose and
  runs it beside the first; **Cancel** creates nothing and shows no error.
- **Main and helper sessions.** The first session of a worktree is its **main session**, chosen
  when the worktree starts; any session added beside it on purpose is a **helper**. Which one is
  main is stored, so it stays the same across restarts until you change it: you can make a helper
  the main session, and the former main becomes a helper. When you delete the main session, the
  first remaining session of that worktree becomes main in the same step; deleting a helper never
  changes the main. Closing the only session leaves the worktree free, and its next session is the
  main one. Worktrees recorded before this existed treat their first open session as the main one
  until you choose another; nothing is rewritten.

- **Worktrees that already hold several sessions** (from before this limit, or because you asked
  for helpers) open untouched: no session is removed or changed. A new session there is still
  refused, and starting one session while another of the same worktree is starting or running is
  refused too: in the sidebar the row opens without starting and its tooltip names the running
  session, and the confirmation that appears lets you run both on purpose. Stopping the running one
  first needs no confirmation. To get back to one session, delete the extras.

<!-- media: worktree-sidebar-light -->

### Worktree state icon

Every worktree row, and the **Default** entry, shows an icon for its current state, just after
the toggle and before the name. You see it without selecting, opening or expanding the row, and
it changes by itself as soon as the session service reports a change; it never raises a
notification, plays a sound or moves the selection. Each state has its own shape, so you can tell
them apart without relying on colour:

| Icon | State | What puts a worktree in it |
|------|-------|----------------------------|
| Triangle | **Running** | A session is working on a prompt, or is starting or restarting |
| Octagon | **Blocked** | A session stopped mid-turn to ask for a permission or an answer, failed (its start failed or its restarts ran out), or is waiting to be resumed after the session service restarted |
| Two bars | **Idle / waiting** | The session finished its turn and waits for your next prompt, has no process running, or its process ended — or the worktree has no session at all |

- **Unknown.** When the app cannot read what a session is doing (for example, a session on a CLI
  that does not report its activity), the icon's place stays empty, and the row does not claim
  to be idle or blocked. The name stays where it is.
- **Asking for a permission.** A session that stops mid-turn to ask for a permission or an
  answer makes its worktree **Blocked** (*Blocked: needs permission*) until it carries on: the
  icon turns to the triangle as soon as the session works again. A turn that simply ended stays
  **Idle / waiting**.
- **Several sessions.** A worktree with a main session and helpers shows one icon for all of
  them: **Blocked** if any is blocked, else **Running** if any is running, else unknown if any is
  unknown, else **Idle / waiting**. Closed (archived) sessions do not count.
- **The tooltip.** Hover a row to read its state in words on a line of its own — *Running*,
  *Blocked: needs permission*, *Blocked: failed*, *Blocked: resumable*, *Idle* or *State unknown* — above the unread line,
  if the row has one.
- **The Default entry** shows the state of the project root's sessions by the same rules; its
  house icon and everything else about it stay the same.

### Which session service a worktree runs on

Each worktree row names the session service (daemon) its sessions run on. After an upgrade there is
exactly one — the host process or the container you already used — and every worktree, and the
"Default" entry, runs on it; nothing is asked and nothing is shared that was not shared before. A
worktree whose daemon was removed stays listed and says **no daemon** until it is moved to another.

A worktree on a remote daemon ([Remote daemons](remote-daemons.md)) shows its path on the host,
not a local one, and its tooltip names the daemon it runs on. Its sessions and terminals run on that
host; you create, rename and delete such a worktree the same way as a local one.

### Refreshing the list

The sidebar's worktree list is not live. It is rebuilt when you open or switch to a project, and
after this app itself creates, deletes, renames, includes or excludes a worktree — but nothing is
watching the repository. So a worktree that appears by any other route is invisible until one of
those moments comes round again: one you created with `git worktree add` in a terminal, one a
coding agent made for its own work, or one you removed by hand.

Click the **refresh** button in the sidebar header — between the "Worktrees" title and **add
worktree** — to re-read the list right now. Whatever git and the filesystem currently report is
what you get: worktrees that appeared are added, ones that are gone are removed, and a branch that
changed is shown as it now is.

Switching away from the project and back used to be the only way to force this, and it no longer
is. Nothing else about your view changes: the rows you had expanded stay expanded, your tag
filters stay applied, and the session you are working in keeps running.

While the re-read is running the button dims and stops responding, and its hover label reads
"Refreshing worktrees…" — pressing it again does nothing, because one refresh is already on its
way. When the answer arrives you get a short "Worktree list refreshed." notice. That notice is the
point of the whole exchange on a project where nothing has changed: most refreshes find nothing,
and without it a button that correctly did its job would be indistinguishable from one that ignored
you.

If the re-read cannot be done — the project folder has been moved or deleted, git cannot read the
repository, or the session service is not running — you get a notice saying why, and **the list you
were already looking at stays on screen**. A failed refresh never empties the sidebar. The button
returns to normal either way, so you can fix the problem and press it again. In the rare case where
no answer comes back at all, it gives up after about half a minute rather than staying dimmed.

The button re-reads the list once, and that is all it does: it starts no timer, installs no watcher,
and changes nothing about the list's behaviour afterwards. The list is still not live, and it never
re-reads itself in the background — on demand means on demand. If you want to know whether something
changed, press it.

### Reading a worktree: name & tags

Each worktree is shown as a clean, human-friendly **name** on the first line, with small
color-coded **tags** beneath it:

- A **type tag** — the Conventional-Commits type from the worktree's branch (`feat`, `fix`,
  `chore`, `docs`, `refactor`, `test`, `build`, `ci`, `perf`, `style`). Each type has its own
  fixed color, so you can recognize what a worktree is for at a glance.
- An **issue tag** — the ticket you entered when you created the worktree. A Jira-style key is
  shown upper-cased (`ABC-123`); a GitHub or GitLab issue number is shown as `#123`.
- A **status tag** — `missing` or `invalid` for a worktree that is not usable (see above).

The name is derived from the descriptive part of the worktree's folder: `feat-abc-123_login-page`
shows as **Login page** with `feat` and `ABC-123` tags. The tags are display-only — the underlying
branch and directory names are unchanged, and a worktree that does not follow the naming convention
simply shows no type tag.

Worktrees that an AI agent creates through the [agent tools](agent-tools.md) follow the same
convention: given a `ticket` (or a `github_issue`), the agent's worktree is named
`${type}/${ticket}_${name}` and gets the same issue tag and issue-filter match as one made in the
dialog.

The `_` is what separates the ticket from the description, so the app never has to guess where one
ends. A name without one has no ticket, which is exactly right for something like
`feat-reporting-2` — the trailing `2` is part of the name, not an issue number. Two consequences
worth knowing:

- Worktrees created before this rule existed have no `_`, so their issue tag is gone. Their names
  read correctly, and you can always [rename](#managing-a-worktree-right-click) one.
- A branch from elsewhere that uses `snake_case` is read as having a ticket: `fix/some_bug` shows
  as **Bug** with a `SOME` tag. The separator means one thing everywhere, and nothing can tell a
  stray underscore from a deliberate one. Rename the worktree if it bothers you.

The sidebar is intentionally compact — tight left/right padding and a slightly smaller font — so
long names and their tags get as much width as possible. It stays legible in both light and dark
themes.

### Pull request status

With **Show pull request status on worktrees** checked in [Settings →
GitHub](settings.md#show-pull-request-status-on-worktrees), a worktree whose branch has a pull
request shows a small indicator at the right of its row, before the row's buttons. It is off until
you turn it on.

The first glyph is the pull request's state:

| Glyph | State |
| --- | --- |
| Branching arrow (`call_split`) | Open |
| Pencil on lines (`edit_note`) | Draft |
| Merging arrow (`call_merge`) | Merged |
| Circle with a bar (`block`) | Closed without merging |

An open or draft pull request that has checks shows a second glyph, the combined result of its
checks:

| Glyph | Checks |
| --- | --- |
| Circled cross (`cancel`) | Failing: at least one check failed |
| Clock (`schedule`) | Pending: none failed, and at least one is still queued or running |
| Tick (`check`) | Passing: every check finished without a failure |

A merged or closed pull request, and one with no checks, shows the state alone. Every state and
status has its own shape, so none depends on colour. The indicator keeps its size however wide the
sidebar is.

The same switch also lists every pull request of a worktree's branch above the file list of its
[Changes view](reviewing-changes.md#pull-requests).

#### When the status is read

With the switch on, the status of a project is read:

- when you open the project in a window;
- when you turn the switch on;
- every 5 minutes while the project stays open, without you doing anything;
- whenever you press the sidebar's refresh button: the indicators catch up within about 10
  seconds, and the button and its "Worktree list refreshed." notice do not wait for the reading.

Only one reading of a project runs at a time. Refreshes pressed while one is under way cause one
further reading when it ends. A project no window shows is never read, and with the switch off
nothing is sent to GitHub.

Each reading keeps the last status you saw until a newer one replaces it. When the last reading is
more than 10 minutes old, the indicator is drawn dimmed, and the tooltip adds
`Read: <n> min ago` (`Read: <h> h ago` from 120 minutes) after the review line.

If GitHub answers that its request limit was reached, nothing is read until the time GitHub gives
for the limit to reset, and the indicators keep showing what was last read. The same pause applies
to a refresh pressed meanwhile.

Failures are silent: with no network, no answer within 10 seconds, or an answer the application
cannot understand, the rows keep their last status and no error, notice or dialog appears; the next
reading is tried at the next interval. When pull requests cannot be read at all (`gh` is no longer
installed, you are no longer signed in, the sign-in cannot see the repository, or the repository no
longer has a GitHub remote), every indicator, tooltip line and removal suggestion of that project
is removed, again without a message.

#### The tooltip and opening the pull request

Hover a row with an indicator and its tooltip adds these lines after the ones it already has:

- `Pull request: #<number> <title>`: a title longer than 72 characters is cut and ends in `…`.
- `PR state:` `open`, `draft`, `merged` or `closed`.
- `Checks:` `passing`, `pending` or `failing`, for an open or draft pull request that has checks.
- `Review:` `approved`, `changes requested` or `review required`, when GitHub reports a decision;
  without one there is no line.

Hovering reads nothing from GitHub or the disk: the tooltip shows what the last reading found, and
a row without an indicator has the tooltip it always had.

To see the pull request on GitHub, right-click the row and choose **Open pull request**, directly
above **Delete**. It opens in your default browser; nothing else changes (selection, sessions and
the sidebar stay as they were), and only a `https://github.com/` address is ever opened. A row
without an indicator has no such entry.

#### A merged pull request

When the pull request shown for a worktree was merged, and the worktree's branch has no commits
beyond the ones the pull request merged, the row also carries the chip **can be removed**, and its
tooltip ends with `Cleanup: merged — this worktree can be removed (right-click, Delete)`. The
application works this out from your local repository at every reading; it fetches nothing.

The mark is only a suggestion. Nothing is removed on its own: remove the worktree as you always
have, with **Delete** in the row's right-click menu, and its confirmation appears as it does for any
worktree.

No mark appears when the branch has commits made after the merged ones, when the pull request was
closed without merging, when the branch has a newer open pull request (the indicator then shows
that one), or when the pull request's last commits were never fetched into your repository, since
the application cannot then tell that the branch holds nothing newer.

#### Which pull request, and which worktrees

- **Which pull request:** the one for the worktree's branch, in the project's own GitHub
  repository. When a branch has several, an open or draft one is shown if there is any, otherwise the most
  recently created; among several open ones, the most recently created.
- **Which worktrees:** a worktree with no branch (detached) and the **Default** entry never show
  one, and a row without a pull request looks as it always did.
- **Only github.com, only the project's own repository:** a repository on another host, or a pull
  request that exists only in the repository a fork was made from, shows nothing.
- **A window that does not hold the project** shows none: the status is read by the window that
  holds the project's sessions.
- **Without `gh`, or without a sign-in,** no row shows an indicator and no error appears.

### Filtering worktrees by tag

Tap the **filter** button in the sidebar header to reveal the tag-filter panel — it's hidden
by default so the list has the full sidebar to itself until you need it:

- Tap a **type chip** (e.g. `fix`) to show only worktrees of that type.
- Tap **issue** to show only worktrees that have a Jira key.
- Tap **untyped** to show worktrees that do not follow the naming convention.
- Multiple filters combine with **OR** — tapping `feat` and `fix` shows both.
- **Clear filters** restores the full list in one tap. If a filter matches nothing, an empty
  message with a clear action is shown.

Only chips for tags actually present in your worktrees are offered. Close the panel by
clicking outside it, pressing `Esc`, or tapping the filter button again — any active filter
stays applied either way. Whenever a filter is active, the filter button itself stays tinted
so you can tell filtering is on even with the panel closed.

**One exception, and only one.** The worktree holding your current session stays listed even when
your filters exclude it — including when it's an [agent worktree](#agent-worktrees). It sits where it would sit unfiltered, and carries a **current session** chip saying why
it's there, so a row that survived a filter it doesn't match is never unexplained. Every other
excluded worktree stays out of the list, and the exception disappears as soon as you move to a session
somewhere your filters do allow. Adding it changes nothing about the filter chips on offer.

### Agent worktrees

AI coding assistants create worktrees of their own inside your project, in the same
`.claude/worktrees/` folder the app manages. Some are throwaway scratch worktrees for a background
sub-task, with machine-generated names like `agent-a885b42dc521fbda1`. Others are ordinary session
worktrees the assistant made because you asked it to work on something.

**The app lists every worktree git reports for the project**, including one you added from outside the
`.claude/worktrees/` folder. It does not guess who made a worktree
from its name, and there is no `agent` chip. A worktree
called `agent-something` is listed like `fix-the-parser`, and renaming a folder or branch never
changes whether it shows or where it sits. Tag filters apply to every row alike.

Every row is **fully live**: you can start a session in one, rename it, or delete it, with the
same confirmation as any other worktree. Deleting a worktree an agent is still using will disrupt
whatever it was doing, exactly as it would from a terminal.

### Sub-agent runs

When Claude Code starts a sub-agent inside a session, Claude Code itself reports it (its
`SubagentStart` and `SubagentStop` events), and the app records the run against that session. The
record is kept across restarts of the app's background service; a run that was still going when the
session ended or the service restarted is shown as stopped. Recording a run changes nothing in the
sidebar yet: no worktree or session is hidden, moved or marked busy because of it. Claude Code's
`claude --bg` background sessions send no such report and are not recognised.

Worktrees kept anywhere else on disk are listed only when you asked for them; see
[Including a worktree that already exists](#including-a-worktree-that-already-exists).

### Resuming stored sessions

Every worktree of the project is listed in the sidebar, whatever its name and whoever made it; the
app keeps no separate record of which ones are "yours". What the app may not know about are
sessions: to see the conversations an assistant stored for this project, press the folder button
in the sidebar header (its tooltip reads **Attach existing…**). It needs a project to be open.

#### Resuming sessions the app has no record of

The dialog lists **stored sessions**: conversations that Claude Code,
Copilot or Pi recorded for this project (in its folder or one of its worktrees) and that the app
does not show yet, newest first with their title. Press **Resume** on one to add it to the sidebar
and continue it where you left off; the assistant is started with its own resume option, so the
conversation carries on from what it stored.
Codex and OpenCode sessions are not listed here: the app does not read their conversation stores.

- A session that cannot be resumed is listed with the reason and has no **Resume** button: its
  worktree's folder is gone, its folder is not a worktree git knows, or it ran somewhere that is not
  a worktree of this project. It is never resumed in another folder instead.
- A session that is already starting or running in the app is refused with "that session is already
  running"; it is resumed in one place at a time.
- Under the list, a short note says what could not be read: an assistant's session folder that is
  unreadable or has a damaged entry, and, when the project runs in a sandbox, that sessions stored
  on this computer cannot be listed. A missing session folder is mentioned only when it is why the
  list is empty.
- The app does not detect a session that is still running in a terminal outside Micold. Resuming it
  here would start a second copy of the conversation, so close the other one first.

**Resume** adds the session to the sidebar and then starts it, so the conversation opens at once.

A worktree has one main session. When the worktree you resume into already has one, **Resume** asks
first, naming both sessions: **Replace** stops the worktree's session, removes it from the sidebar and
makes the resumed one the main session; **Keep both** adds the resumed session beside it as a
helper and stops nothing; **Cancel** changes nothing. Restarting or starting a session you already
have never adds another. **Resume all** adds only the most recent stored session of each worktree
that has none, and leaves the others listed so you can resume them one at a time.

#### The offer when you open a project

When you open a project and the app finds stored sessions it does not list, a banner at the top of
the sidebar says how many. Press **Resume all** to resume every one of them that can be, or, when
none can, **Review** to open the **Attach existing…** list. Nothing is resumed until you press a
button.

Press **Dismiss** to hide the banner for that project until the app is restarted. Dismissing loses
nothing: **Attach existing…** still lists everything.

### Resizing and hiding the sidebar

- **Resize**: drag the thin handle on the sidebar's right edge to make it wider or narrower.
- **Hide**: click the **hide** button (panel-collapse icon) in the sidebar header, next to
  **add worktree**. The sidebar collapses to a thin strip.
- Hiding takes the header with it, so the **refresh** button is unavailable until you show the
  sidebar again; the collapsed strip hosts only the **show** button.
- **Show**: click the **show** button (panel-open icon) on the collapsed strip to bring it back.

## The "Default" entry: sessions without a worktree

Above your worktrees, the sidebar always shows one **Default** entry — even before you've
created any worktree at all. Starting a session from it runs directly in your project's own
root directory (whatever branch you currently have checked out there), instead of inside an
isolated worktree.

Use it for work that doesn't need its own branch: quick one-off commands, inspecting or running
the project exactly as it's currently checked out, or anything you deliberately don't want
isolated into a throwaway worktree. Creating a worktree first, just to run a quick command,
is unnecessary overhead this avoids.

- The Default entry is **not a worktree** — it has its own icon (a house) rather than the
  worktree iconography, so it's never mistaken for one, and it has no type/issue/status tags
  and no right-click menu (no rename, delete, or copy-name — those are worktree-only actions).
- Starting a session from it **never creates, modifies, or removes a worktree or branch**.
- It supports the same session actions as a worktree — start, switch, close, and it persists
  and restores across restarts exactly like a worktree-bound session (see
  [Starting, switching, and closing sessions](#starting-switching-and-closing-sessions) below,
  which applies equally here).
- You can run **multiple concurrent sessions** from the Default entry, the same as a worktree.
- Because every Default session shares your project's single checkout, actions that change the
  working tree (like switching branches from within one Default session's terminal) are visible
  to every other Default session too — this is expected, not a bug, since there's no worktree
  isolating them from each other.
- The Default entry is **never hidden by the sidebar's tag filters** — since it isn't a
  worktree, filtering by branch-derived tags doesn't apply to it; it always stays visible.
- Hover any sidebar entry to see a tooltip describing it. For the **Default** entry that is its
  location — the project root itself. For a **worktree** the tooltip leads with the worktree's
  **full name**, then gives its location relative to the project. The sidebar is narrow, so a long
  name is shortened with an ellipsis in the row itself; hovering is how you read the whole of it,
  and how you confirm exactly where a session is about to run before you start it. The tooltip
  opens **below** the entry, or **above** it for an entry at the bottom of the window, so it never
  hides the entry you are pointing at or the actions on it.
- A worktree's tooltip also names the two things its row label cannot show, each on its own line:
  the **branch** it is bound to, and — when it differs from the displayed name — the **folder** on
  disk. Both are absent from the row because the displayed name is derived by stripping the type
  token and the ticket out of the folder name, which is what makes it readable and also what makes
  it unusable as a path or a branch to type.
- A row that is **flagged** explains itself on hover. A worktree whose directory is gone, or whose
  directory git does not recognise, adds a **Status** line reading `missing` or `invalid` — the
  same word as the chip on the row. A worktree you added from outside this app's own
  `.claude/worktrees/` folder marks its location `(outside this app)`, which is why that one path
  is absolute where the others are relative. A healthy worktree says nothing about its status, so
  the rows that do have something to say are the ones that stand out.

## Creating a worktree

Click **add** in the sidebar header to open the New worktree form. At the top, three chips choose
where the worktree's branch comes from:

- **New branch** (the default) — describe the work and the app derives a fresh branch name.
- **Existing branch** — pick a branch that already exists. See
  [Working from an existing branch](#working-from-an-existing-branch) below.
- **GitHub issue** — pick one of the repository's open GitHub issues, and its number and title
  fill in the ticket and the name. See [From a GitHub issue](#from-a-github-issue) below.

When the app knows more than one daemon, a **Daemon** select in the form chooses where the new
worktree runs; it starts on the daemon your existing setup used. The choice is remembered with the
project, so it is the same after a restart. With no daemon registered the select is left out.

**Required fields.** The dialog shows a `* required` line above its first field. Fields marked `*` must be filled before the worktree can be created: **Type**
and **Name** for a new branch or a GitHub issue, **Branch** when you work from an existing one.
Ticket and the session's fields are optional and carry no mark. **Create** is always available: press it
with a required field empty and every empty required field shows "This field is required" at once, and nothing is created.
Typing into a field clears its message.

### Creating a new branch

With **New branch** selected:

1. **Type** — a select control: click it to open a list of every Conventional-Commits type
   (`feat`, `fix`, `chore`, `docs`, …), then click one to choose it. The control always shows the
   currently chosen type when closed, and marks it in the list when reopened.
2. **Ticket** — optional reference (e.g. `ABC-123`). Leave blank to omit it.
3. **Name** — a short description (e.g. `login page`).

Beside **Type**, an **AI CLI** select chooses the session the worktree starts with: your
default CLI is preselected and **None** (last in the list) creates the worktree without a session.
With a CLI chosen, an optional **First prompt** field holds the opening prompt sent to it. The
session is defined here, when the worktree is created, and opens at once; there is no separate
add action on a worktree that already has one.

The form shows the derived names before you create:

- **Directory**: `.claude/worktrees/${type}-${ticket}_${name}`
- **Branch**: `${type}/${ticket}_${name}`

For example, `feat` + `ABC-123` + `Login page` creates the branch `feat/abc-123_login-page` and a
worktree at `.claude/worktrees/feat-abc-123_login-page`. With no ticket, `chore` + `cleanup` gives
`chore/cleanup` — no `_` anywhere. Illegal characters in the ticket or name are automatically
simplified (slugified), so `#123` is a perfectly good ticket; it shows up as a `#123` tag in the
sidebar.

<!-- media: create-worktree-light -->

The `_` separates the ticket from the description, and it is on the branch as well as the folder.
That is what lets the app read the ticket back later: delete a worktree and re-create it from the
branch and the tag comes back, instead of the app having to guess where the ticket ended.

Creating a worktree makes the new git branch and worktree for you — no manual git commands. If the
derived name collides with a branch that already exists, the app asks what you want to do rather
than refusing — see [Working from an existing branch](#working-from-an-existing-branch). If the
worktree *folder* already exists, creation is blocked with a message that names the folder and tells
you to pick a different name or remove that folder — no branch choice can resolve it, so there is
only the one answer. You get the same sentence whether the app catches the clash while you are
filling the form in or only when it tries to create. If anything fails partway, the app rolls back so no half-created branch or directory
is left behind.

If the project uses git submodules, they're fetched automatically as part of creating the
worktree — including submodules nested inside other submodules — so the new worktree is ready to
use immediately, with no extra `git submodule` commands to run yourself. Projects without
submodules are unaffected. While a worktree is being created, the form shows a progress bar
alongside a short description of what's currently happening. That description names the step for
what you actually chose — "Checking out existing branch" when you reused one, "Replacing branch and
creating worktree" when you overwrote one, "Creating tracking branch and worktree" when you
continued from a remote, and "Creating branch and worktree" for an ordinary new branch — followed by
"Setting up submodules" where they apply — the description only
ever names a step that's actually part of this creation, so a repository without submodules never
shows a submodule-related step. This can take a little longer than usual for a repository with
submodules to fetch, so seeing the description change (rather than a single static message) is
expected, not a sign the app is stuck. If creation fails, the progress bar stops and the
description stays on the step where it failed, next to the error message.

If fetching a submodule fails (for example, a network problem or a private submodule remote you
aren't authenticated against), the worktree is not created — the branch and directory are rolled
back the same way any other creation failure is, and the error names the submodule that failed
and why, so you can fix the problem and try again.

While a worktree is being created, clicking outside the form or pressing Escape does nothing: the
form stays open with its progress showing, so a stray click can't hide how the creation went. To
leave the form before it finishes, press **Cancel**. Cancel closes the form but does not stop the
creation, which carries on in the background. When it finishes, a notification tells you the
outcome: the name of the new worktree, or the error and the step where it failed.

> Naming formats are fixed in this version and are intended to become configurable later.

### From a GitHub issue

With **GitHub issue** selected, the form lists the repository's open issues. Pick one and its
number becomes the **Ticket** and its title becomes the **Name**. Everything else works as for a
new branch: the same preview, the same create, the same prompt when the branch name is already
taken. The issue's labels choose the **Type** (see below); you can change it before you create.

**What you need.** The app reads issues through the GitHub CLI, `gh`, so install it from
[cli.github.com](https://cli.github.com) and sign in once with `gh auth login` in a terminal. The app
looks for `gh` on the `PATH` your environment-include script sets up (Settings → Environment), then
on the app's own `PATH`, then in the usual install folders. Issues are read as you: a private
repository works when your `gh` sign-in can see it, and an anonymous read is never attempted.

**Which repository.** The chip reads the repository behind the `origin` remote when it is on
github.com, otherwise the first github.com remote in the repository's configuration. Only
github.com is supported. A remote whose address is rewritten by git's `url.<base>.insteadOf` setting
is read as written in the configuration, so an alias that only becomes a github.com address after
rewriting is not recognised. When no github.com remote is found, the chip is disabled and the line
under the chips says why: "This repository has no GitHub remote." or "Couldn't read this
repository's remotes: …".

**What is sent to GitHub, and when.** Opening the form contacts nothing but the app's own session
service. Before you choose the chip, the line under the chips names the repository it would read:
"GitHub issue reads open issues of *owner/name* from GitHub." Only choosing the chip, or pressing
**Retry** after a failure, asks GitHub for the repository's open issues. The request carries the
repository's owner and name and nothing else from your project. When more than 1,000 issues are
open, typing in **Issue** also searches GitHub (see below); that request carries the text you typed
as well. Picking an issue asks GitHub once more, for that issue's relationships (see below); that
request carries the repository's owner and name and the issue's number. The same notice stays at
the top of the issue list while the source is chosen.

**The list.** Issues are listed most recently updated first. Each takes two lines: its number and
title, then who reported it and its labels. Long text wraps onto further lines instead of being cut
off, and `ghost` stands for a reporter whose account has been deleted.
Type in **Issue** to narrow the list by number, by a word of the title, by a label, or by the
reporter's GitHub login: typing `octo` keeps the issues `octocat` reported, whatever the letter
case, and highlights the matched part of the login in each row. Use the arrow keys and Enter to
pick without leaving the field. The list follows the highlight as you move with Up and Down, so the
highlighted issue is always fully visible, however many lines it takes; an issue taller than the
whole list shows from its top.
Rest the pointer on an issue for 3 seconds, without moving it, to see the start of the issue's
description in a tooltip: plain text, without Markdown marks or link addresses, at most three lines,
ending in "…" when the description is longer. It closes when you move to another issue, leave the
list or pick the issue, and nothing appears while the pointer keeps moving. It needs a pointer: the
arrow keys do not open it. An issue without a description shows nothing. The descriptions arrive
shortly after the list, from the top of the list down, so that reading them does not hold the list
back: for a moment after the list appears, and for longer on a repository with many open issues, an
issue further down may have no tooltip yet. If the descriptions cannot be read, the list stays as it
is and the tooltips are simply missing until the list is loaded again. Resting on an issue asks
GitHub for nothing, and a description is not searched: typing a word that appears only in a
description does not find the issue.
The search runs over the loaded issues, on
your machine. A pick replaces whatever was in the ticket and name, including an earlier pick, and both
fields stay editable afterwards. At most 1,000 issues are loaded. When the repository has more, a
line under the list says "Showing the 1,000 most recently updated of *N* open issues — search also
looks on GitHub." While the
list loads, you can switch back to another chip or fill the fields in yourself. A repository with no
open issues says "*owner/name* has no open issues."

**When the issues can't be read.** The list shows what went wrong and a **Retry** button:

| Message | What to do |
|---|---|
| "Couldn't read issues: the GitHub CLI (`gh`) isn't installed. …" | Install `gh`, sign in with `gh auth login`, then retry. |
| "Couldn't read issues: you're not signed in to GitHub. …" | Run `gh auth login` in a terminal, then retry. |
| "Couldn't read issues: your GitHub sign-in can't access *owner/name*." | Sign in with an account that can see the repository, or ask for access. |
| "Couldn't reach GitHub. Check your connection, then retry." | Check the network connection, then retry. |
| "GitHub's rate limit was reached. Wait a minute, then retry." | Wait a minute, then retry. |
| "GitHub didn't answer within 10 seconds." | Retry. If it keeps happening, check `gh` works from a terminal. |
| "Couldn't read issues: …" | Read the detail `gh` reported, then retry. |

**The issue's relationships.** Right after you pick an issue, the form reads what GitHub knows
about it and shows it below the **Issue** field, one row per kind, in this order, and only the
kinds the issue has:

| Row | What it shows |
|---|---|
| **Blocked by** | The issues blocking it, each with its number, title and whether it is open or closed. When at least one is open the row is a warning: its label is red and its text starts "Blocked:" and says how many are open, as in "Blocked: 1 open of 2: …". |
| **Blocking** | The issues it blocks. |
| **Parent** | Its parent issue, with the repository when it lives elsewhere (*owner/name*#3). |
| **Sub-issues** | "1 of 2 closed:" and then the sub-issues. |
| **Closing pull requests** | The pull requests that will close it, each marked open, draft, merged or closed. |
| **Linked pull requests** | The other pull requests that mention it. When GitHub has more than the app read, the row says "at least *N*". |
| **Assignees** | Their GitHub logins. |
| **Milestone** | Its title. |
| **Projects** | Each project it is in, with its filled-in fields: "Roadmap: Status In progress, Estimate 3; Backlog". |

An issue with none of these shows nothing extra. Its labels have no row, because the issue's row in
the list already shows them. A row names at most 20 items and then says "and *N* more"; items you
have no permission to see are not named and are counted: "; *N* you can't see". With more than five
rows, the block scrolls. When your GitHub sign-in cannot read one kind, for example projects, that
row says "Not readable:" and why, so a kind that cannot be read is never mistaken for one that is
empty. When the whole read fails, the form shows one grey line, "Couldn't read this issue's details." followed by what went
wrong, and nothing else changes.

The relationships are only information. **Create** never waits for them, and is never refused or
changed by them: a blocked issue gets its worktree like any other, and the same goes while the read
is still running, after it fails, and when it takes longer than 10 seconds. Picking another issue,
or leaving the **GitHub issue** chip, drops the relationships of the earlier pick. The worktree's
row in the sidebar looks as it always did; the relationships are shown in the form only.

### The issue's labels choose the type

Picking an issue also sets the **Type** from its labels, through one label-to-type mapping that
applies to every project. Until you change it, the mapping is:

| Issue label | Type |
|---|---|
| `bug` | fix |
| `enhancement` | feat |
| `documentation` | docs |

GitHub's other stock labels, such as `question` or `good first issue`, choose no type. Labels match
whatever their letter case, so `Bug` counts as `bug`. When an issue carries several mapped labels,
the entry listed first in the mapping wins: an issue labelled both `enhancement` and `bug` becomes
a fix. Each row in the list shows the issue's labels, so you can tell which type a pick will give.

A pick replaces the type you had selected. When none of the issue's labels is in the mapping, the
pick clears the type, and the form asks you to choose one before it creates the worktree. Either
way, the **Type** stays yours to change afterwards. The mapping is read at the moment you pick, so
changing it later does not change the type of an issue you already picked. It is kept in the app's
`settings.json`, together with your other settings, and holds only labels and types, never anything
from an issue.

To add your own labels, change a type, reorder entries or restore the defaults, use **Settings →
GitHub** (see [GitHub issues](settings.md#github-issues)).

### Searching beyond the 1,000 loaded issues

When the repository has more open issues than the 1,000 the form loads, what you type in **Issue**
is also sent to GitHub's search, so an older issue can still be found — by its number (`4312` or
`#4312`), a word of its title, or a label. The loaded matches show at once, as always. Once you stop
typing for a moment (0.3 seconds), "Searching GitHub…" appears under the list, and the open issues
GitHub finds join the loaded ones. Each issue is listed once, and pull requests and closed issues
never appear. An issue GitHub matched only in its description or comments is left out, because
nothing in its row shows why it matched. Pick a found issue like any other.

A reporter's login is searched the same way as a label: what you type is sent as ordinary search
text, never as a filter by author. So an older issue is found by its reporter only when GitHub's
search for that text returns it; a found issue is then listed if its number, title, labels or
reporter match what you typed.

Each keystroke replaces the previous search, so only the answer for what is typed now is used. When
every open issue is already loaded, nothing is sent: the search stays on your machine.

If the search fails, the loaded matches stay, and the line under the list says "Search beyond the
loaded issues failed —" followed by one of the reasons above, with a **Retry** button that runs the
same search again. GitHub allows about 30 searches a minute; if you hit that, wait a minute, then
retry.

### From a Jira issue

A project connected to Jira Cloud can start a worktree from one of its open issues, the way a GitHub
issue does. Pick one and its key becomes the **Ticket**, its summary (shortened to 50 characters)
becomes the **Name**, and the **Type** is chosen from the issue's type name and labels. Everything
else works as for a new branch.

**What you need.** A Jira Cloud site (Jira Data Center and Server are not supported), an API token
from your Atlassian account, the Jira integration switched on, and a connection for the project, all
in **Settings → Jira** (see [Jira](settings.md#jira)).

**What is sent, and where.** Only the connection's own site is contacted. The request carries the
project key, the search text you typed and your credentials (account email and API token), and
nothing else from your project.

**GitHub or Jira.** A project with a Jira connection and no GitHub remote lists Jira and shows no
choice. A project with both shows a **GitHub / Jira** choice under the branch chips; GitHub is the
default. Choosing the other source replaces the list and clears the search text, and the dialog
opens on that source the next time you use it in the same project. Each project remembers its own
choice. With the Jira integration off, or no connection for the project, the dialog has no choice
and lists GitHub as before.

**The list.** Open issues are listed most recently updated first, up to 1,000. When the project has
more, typing in **Issue** also searches on the Jira site, beyond the loaded issues, and the line
under the list says so. Typing narrows the list at once to issues whose key (`ABC-123`), summary or
any label contains the text, ignoring letter case; the Jira site is asked only when the list is
capped and you have paused typing for a moment. A search that fails leaves the matches already
loaded and offers **Retry**. Pick an issue
with the pointer or with the arrow keys and Enter; a pick replaces the ticket and name, which stay
editable.

**The type.** The issue's type name and labels are matched against the label-to-type list in
**Settings → GitHub** (see [GitHub issues](settings.md#github-issues)); the entry nearest the top
wins. With the defaults, a `Bug` issue selects fix. When nothing matches, the type is left for you
to choose.

**The branch and the tag.** The branch is `${type}/${KEY-123}_${name}`, for example
`fix/ABC-123_login-fails`, and the sidebar shows the key as the worktree's issue tag.

**When the issues can't be read.** The list shows what went wrong and a **Retry** button. Creating a
worktree by hand keeps working whatever the notice says.

| Message | What to do |
|---|---|
| "Couldn't reach *host*: *reason*." | Check the network connection, proxy and the site address, then retry. |
| "*host* did not answer within 10 seconds." | Retry. If it keeps happening, check the site opens in a browser. |
| "*host* refused the credentials. Check the email and API token in Settings > Jira." | Correct the email or create a new API token, save, then retry. |
| "*host* does not show project *KEY* to this account. Check the project key and your access in Settings > Jira." | Correct the project key, or ask for access to the project, then retry. |
| "*host* is limiting requests. Try again in a minute." | Wait a minute, then retry. |
| "*host* did not answer like Jira Cloud. Jira Data Center and Server are not supported." | Check the site address is a Jira Cloud site; Data Center and Server cannot be used. |

To send one prompt to several AI CLIs at once, each in its own new worktree, use **Run in
parallel** beside **Add a worktree**; see [Running a prompt in parallel](parallel-runs.md).

### On a Kubernetes daemon

A worktree bound to a Kubernetes daemon comes from the project's git remote: the daemon clones it
onto its volume first, so commits you have not pushed stay local. A project with no remote is
refused, and a remote that needs a credential needs it allowed for the daemon. See
[Kubernetes daemons](kubernetes-daemons.md#worktrees-on-a-kubernetes-daemon).

## Working from an existing branch

The **Daemon** select (see [Creating a worktree](#creating-a-worktree)) is on this form too. When a
branch or a folder name is refused because another worktree holds it, the message also says which
daemon that worktree runs on.

Work doesn't always start in this app. You might have begun a branch in a terminal, pushed one
from another machine, or been handed one by a colleague. Either route below brings it into a
worktree without leaving the app.

### Picking the branch from a list

Choose the **Existing branch** chip in the New worktree form and search for the branch you want.
The field lists every branch until you type; from the first character on, it narrows to the
branches that match, and the characters you matched are picked out in colour inside each row — so
you can see *why* a branch is in the list, not just that it is.

You don't have to type the name exactly. Three kinds of search work, in this order of confidence:

| What you type | What it finds | Example |
|---|---|---|
| Text that is really in the name | The branches containing it | `report` → `feat/reporting-dashboard` |
| The letters in order, with gaps | Branches you're abbreviating | `frep` → `feat/reporting` |
| The name with one letter wrong, missing or extra | Branches you mistyped | `reportng` → `feat/reporting` |

Exact matches are always listed above approximate ones, so the branch you meant stays at the top.
The emphasis shows which kind of match you got: a solid run of colour means the text was really
there, and scattered letters mean you abbreviated.

Approximate matching needs something to go on, so it starts at **three characters** — one or two
letters find only what literally contains them. Typo tolerance starts at **five**, because below
that one wrong letter is too large a share of what you typed to tell a mistake from a different
branch.

Use **↑** and **↓** to move through the results and **Enter** to take the one you're on;
**Escape** closes the list and leaves your search text alone. Everything else you type goes into
the field, so searching never breaks stride. The **✕** at the end of the field clears the search
in one action.

Each row shows the branch name and, for a branch that only exists on a remote, which remote it
came from:

| Row | Meaning |
|-----|---------|
| `feat/login` | A local branch, ready to use. |
| `feat/reporting · origin` | Exists on `origin`, not yet on this machine. |
| `feat/login · in use by feat-login` | Already checked out in that worktree — not available. |
| `feat/login · in use by an agent worktree` | Held by an agent worktree, which has its own sidebar row. |
| `fix/olx · in use outside this app` | Held by a worktree the app doesn't manage — see below. |
| `main · in use by the project checkout` | The project's own current branch — not available. |

A branch that is in use elsewhere stays in the list — dimmed, and not selectable. It is shown
rather than hidden so you can read *where* it is in use instead of wondering why it is missing.

If nothing matches what you typed, the list says so. Clear the field or shorten your search to see
everything again.

The worktree folder is derived from the branch name — `feat/abc-123_login` becomes
`.claude/worktrees/feat-abc-123_login` — and the form shows it before you create. A branch this app
created keeps its ticket through the trip, so the worktree comes back with the same `ABC-123` tag it
had before. A branch without a `_` gets no ticket tag; the app does not guess one.

> **Remote branches reflect your last fetch.** The app never contacts a remote here; it reads
> only what's already in your repository. Run `git fetch` yourself first if you want the list to
> be current.

### When the name you typed is already taken

If you're creating a new branch and the derived name already exists, the form asks what to do
instead of refusing:

- **Reuse branch** — create the worktree on the existing branch, with all of its commits intact.
  This is what you want to continue work started elsewhere.
- **Overwrite…** — discard that branch and start again from the current checkout. Asks for a
  second confirmation first, because it destroys commits.
- **Cancel** — change nothing. Your form entries are kept, so you can adjust and try again.

If the branch exists only on a remote, the choices are **Continue from `<remote>`** (creates a
local branch at the remote branch's tip and tracks it, so your next push goes back to the right
place) or **Start fresh** (an ordinary new branch at the current checkout, which will diverge
from the remote branch of the same name).

> **Overwrite cannot be undone from the app.** The old commits are no longer reachable from that
> branch. Git's reflog may still hold them for a while, but recovering from it is a manual git
> operation the app does not offer — treat overwrite as permanent.

### When a branch can't be used

A branch that is already checked out somewhere can't back a second worktree — git allows a branch
in only one place at a time. The app says so and identifies where it is. Neither reuse nor
overwrite is offered: open that location to continue there, or pick a different branch.

Where it is can be one of four places, and the message tells you which:

- **Another of your worktrees** — named by its folder, which is its row in the sidebar.
- **The project's own checkout** — the branch the project directory itself is on.
- **An agent worktree** — one an assistant made inside the project's worktree folder; it has its
  own row in the sidebar.
- **A worktree outside this app** — one git knows about that this app doesn't manage: another
  tool's worktree directory (`.git-paw/worktrees/…` and the like), or a checkout in some unrelated
  folder. These never appear in the sidebar however you filter, so the message gives the **full
  path** instead of a folder name. `git worktree list` in the project directory shows all of them.

That last case is worth knowing about if a branch looks perfectly ordinary and is refused anyway.
It usually means a worktree you created outside the app — or a tool you used before this one —
still holds it. Removing that worktree (`git worktree remove <path>`) releases the branch.

### Including a worktree that already exists

For that last case there is a better answer than deleting anything: **Include that worktree**, the
button the message offers. The work is already there, in a worktree you or another tool created —
including it simply tells the app to show it too, exactly where it is.

What it does **not** do matters as much as what it does:

- Nothing is moved, copied, renamed, or re-registered. The worktree stays where it is, and the tool
  that created it goes on finding it there.
- No git command runs at all. The branch, its commits, and the repository are untouched.
- The branch is still checked out in that worktree, so it still can't back a *second* one. What
  changes is that you can now work in the one that holds it, from here.

An included worktree behaves like any other: it appears in the sidebar, hosts sessions, and can be
renamed or deleted. Two things mark it out. Its row carries an **outside this app** tag, and hovering
it shows the full path — a folder name alone wouldn't tell you where a worktree the app didn't create
actually lives. And if its folder name happens to match one of the app's own worktrees, the app shows
it under a qualified name rather than renaming anything on disk.

Inclusion is remembered per project, across restarts, and is reversible: right-click the row and
choose **Stop showing**. That removes it from the sidebar and leaves it completely untouched on
disk — it is the opposite of Delete, not a milder version of it. Deleting an included worktree is
still possible, and its confirmation names the full path it is about to remove, precisely because
that path is somewhere the app didn't put it.

One case inclusion does not cover: a folder under `.claude/worktrees/` that git has *forgotten*
about. That is already listed, marked invalid, and no branch is held for it — so it never produces
the refusal above. Repairing one is a `git worktree repair` job, outside the app.

## Managing a worktree (right-click)

Right-click a worktree in the sidebar to open its context menu:

- **Copy name** — copies the worktree's displayed name to the system clipboard, so it can be
  pasted into any other application (browser, chat, terminal, etc.). Useful because the sidebar
  label itself isn't a text field you can select from directly.
- **Rename** — changes only the name shown for the worktree in the sidebar. It does **not**
  rename the folder on disk or the git branch, and the type/issue tags are unaffected (they
  keep deriving from the folder name). The custom name is remembered across app restarts. Clearing
  it is not needed — just rename again.
- **Stop showing** — only on a worktree you *included* (see
  [Including a worktree that already exists](#including-a-worktree-that-already-exists)). Removes
  the row from the sidebar and changes nothing on disk: the worktree, its branch, and its files are
  exactly as they were. Include it again at any time.
- **Delete** — removes the worktree completely. A confirmation dialog first spells out exactly
  what will be removed: the worktree directory under `.claude/worktrees/` and **all of its
  sessions** — this part is unconditional. (For an *included* worktree the dialog gives its full
  path instead, and says it is one outside the app, because that is where the deletion lands.) If the worktree has an associated git branch, the
  dialog also offers an **"Also delete the branch"** checkbox, **checked by default** so
  confirming without changing anything behaves exactly as before (the branch is deleted along
  with everything else). Uncheck it to keep the branch — the directory and sessions are still
  removed, but the branch remains an ordinary branch in the repository, usable later (for
  example, to create a new worktree from it). Confirming terminates any running sessions in
  that worktree first, then removes the directory, sessions, and (unless unchecked) the branch —
  this cannot be undone. Cancelling removes nothing, including any change you made to the
  checkbox. (A worktree that is already missing/invalid can still be cleaned up this way.)
- **Start another session...** — only on a worktree that already has a session (the **Default**
  entry offers it too). A confirmation names the worktree and its existing session. Confirming
  starts a **helper** session beside it, selected and shown; the existing session stays the main
  one. Cancelling changes nothing. The confirmation still starts the session if another window or
  an agent added one while it was open.
- **Review changes** — opens the Changes view: every file the worktree changed against its base
  branch, committed and uncommitted. The **Default** entry offers this item too. See
  [Reviewing a worktree's changes](reviewing-changes.md).

## Starting, switching, and closing sessions

- A worktree created with an AI CLI already has its session. Only a worktree with no session shows
  a **start session** action; use it to launch one. Its terminal opens on the right. Which AI CLI it runs is your default, or whatever
  you pick from the chevron beside that action — see
  [Choosing which AI CLI a session runs](#choosing-which-ai-cli-a-session-runs).
- A worktree can host **multiple concurrent sessions** — start as many as you need for parallel,
  non-interfering tasks.
- **Switch** between sessions by selecting them in the sidebar. Background sessions keep running;
  only the displayed terminal changes.
- Right-click a helper session for **Make main session**: it becomes the worktree's main session
  and the former main becomes a helper, in the same step. Selecting the worktree then opens it.
  The main session's menu does not offer it.
- Right-click a session for **Close** and **Remove**:
  - **Close** stops its AI CLI process and hides it from the sidebar. It does not reappear —
    including on a later restart, even though the underlying conversation itself still exists on
    disk in the CLI's own storage. There is no way to bring a closed session back through the UI.
  - **Remove** permanently deletes the session's record, after a confirmation step. Unlike Close,
    there is no possible recovery path back into the sidebar either. Remove is only offered on a
    still-visible session — a closed session can't be removed separately, since it's already
    hidden. When you remove a worktree's main session and others remain, the confirmation names
    the first remaining session, which becomes the main session; removing a helper never changes
    the main.
  - Close, Remove, deleting a worktree and forgetting a project also delete the session's
    [saved terminal history](settings.md#terminal) from the disk, before the action is done. **Stopping** a
    session keeps it: the earlier output is shown again when the session is started.

Session labels come from the AI CLI itself (its own session title); until a title is available a
placeholder is shown.

### The name on a session row

You never type a session's name. It is the AI CLI's own name for the conversation, and the row
picks it up as soon as the CLI has one — usually a few exchanges in, once there is enough of a
conversation to name.

**The name stays.** Once a session has been named, that name is the row's, whether or not anything
is running: after you close the app, after the background service restarts, after a reboot. You do
not have to open a session to find out which one it is — the list you come back to reads the same
as the list you left, so you can pick the session you want by its name alone.

**A conversation the CLI never named reads what you first typed in it.** Claude Code does not name
every conversation. When it has not, the row shows the first thing you typed instead: your first
prompt, or for a skill or slash command the text you gave it (just the command's name, such as
`/speckit-autopilot`, when you gave it none). It is shown on one line and cut at 80 characters with
"…". Commands the CLI answers itself, like `/model`, are skipped, and so is anything the CLI or this
app inserted into the conversation. If the CLI names the conversation later, that name replaces
this label, and the label never comes back.

**GitHub Copilot sessions follow the same rule.** A Copilot session the CLI has not named yet shows
the first thing you typed in it, and switches to Copilot's own name for the conversation once it
writes one. Text Copilot puts into the conversation itself — the context it loads for a skill, the
instructions it discovers in the project, the prompts it writes to keep itself going — is not
counted as something you typed.

**A session from an older Copilot shows the summary it wrote.** Copilot 1.0.36 and earlier recorded
its name for a conversation as a *summary* rather than a name. That summary is Copilot's own name
for the session, so the row shows it, exactly as it shows a newer Copilot's name — and in preference
to the first thing you typed.

**"New session" means nothing has been typed in the conversation yet**, not that the app hasn't
finished loading. A session you created and never talked to reads "New session" for as long as that
is true. It does not wait around unnamed, though: with no conversation in it there is nothing to
come back to, so the next time the project is opened while nothing is running it, the row is tidied
away. A session that was named, or that shows what you typed, keeps its row, even if the CLI later
clears out that conversation's records.

**The newest name wins.** If the conversation moves on and the CLI re-titles it, the row follows,
and that newer name is the one that comes back next time. Names are per session: re-titling one
never touches another.

**A name you chose in the CLI yourself is the one that sticks.** Rename a conversation in Claude
Code — `/rename` — and that is the name the row reads whenever the app looks that conversation up in
the CLI's records: the first time it meets it, after the background service restarts, or on another
machine. The name the CLI had generated before your rename does not come back, and the CLI's own
later titles do not replace it either — your name is the conversation's name until you change it
again.

**A row that was already showing the pre-rename name is corrected the next time you open that
session.** A name the app has already recorded is never looked up again (that is what keeps names
stable), so a row that picked up the CLI-generated name before this was fixed keeps showing it. Open
that session once and the row takes the name the CLI is displaying, which is yours.

**Sessions from before this was true get their names back.** If you have sessions that were showing
"New session" even though you had talked in them, opening the project is enough — each one is
looked up in its own CLI's records, once, and keeps the name it finds, or failing a name, the first
thing you typed. A session whose conversation the CLI no longer has keeps what it was already
showing; nothing takes a name or a label away.

**A session you are working in right now is no different.** Type your first prompt into a session
the CLI has not named, and within a minute its row reads that prompt — you do not have to reopen
the project, restart anything or switch away and back. It makes no difference whether the CLI
starts working before or after it tells the app that you typed; either way the row catches up on
its own. And when the CLI names the conversation later, the row switches to that name while you
watch, the same as it would after a restart.

## Choosing which AI CLI a session runs

A session runs one AI coding CLI — Claude Code, GitHub Copilot, Pi Coding Agent, Codex or OpenCode — and which one is
decided when the session is created.

- **Press the start-session action** and you get the CLI set as your
  [Default AI CLI](./settings.md#default-ai-cli), in one press, exactly as before.
- **Press the small chevron beside it** to pick a different CLI for this session only. Your default
  is not changed.
- **If only one CLI is installed, the chevron is not there at all.** There is nothing to choose
  between, so the affordance is the plain button it always was.
- Only CLIs you actually have installed are ever offered.
- **Each row offers what its own project or worktree provides.** A row is answered for the
  directory a session started from it would run in, including what
  [the environment a session starts in](./settings.md#the-environment-a-session-starts-in) adds
  there. So if one project's environment-include script puts `pi` on the `PATH` and another's does
  not, the chevron is on the first project's rows and not on the second's. Opening Settings,
  reconnecting to the session service, or opening another row's list never changes what a row
  offers. Until a row's own answer arrives — for a moment after a project opens — it offers what
  your home directory has.
- **A list shows every CLI and says why one is disabled.** A row's list has an entry for every
  supported AI CLI. One that is not available for that row's directory is a disabled entry with a
  red icon, and it cannot be chosen. Hovering the icon shows why a session started there would
  not find it and what to change — for example that the environment-include script failed for that
  directory, and to fix the script named in "Script path". A row that offers every CLI has no
  disabled entry, and a row with one CLI has no chevron and is unchanged.
- **Installed means installed where sessions run.** With the session service directly on this
  computer, that is your `PATH`. With it [in a container](./sandboxed-daemon.md), it is the
  container's image, and what is on your own `PATH` makes no difference. The image this app
  publishes, and one built from a checkout, ships all three.
- **If your default CLI is one a session would not find there, pressing start says so and offers
  the ones it would find** instead of trying to run something that isn't there. The message names
  the CLI, gives the reason for that row's directory and what to change (the same reasons as the
  note under [Default AI CLI](./settings.md#default-ai-cli)), and the list opens. If the reason is
  not known yet, the message says only that a session would not find it there. Nothing is created
  until you pick — your default stays as you set it, and the list is checked at that moment, so a
  CLI you installed since opening the app is in it.

**The choice is fixed for the session's lifetime.** There is no way to switch a running session to
another CLI, and nothing switches it for you — not changing your default, not restarting the app,
not restarting your machine. A session is a conversation with one tool, and each tool keeps its
conversations in its own place.

**Two sessions in the same worktree can run different CLIs at once.** They do not interfere: each
has its own process, its own terminal, its own conversation record, and its own title.

### Sessions on Codex and OpenCode

Pick **Codex** or **OpenCode** from the chevron beside the start-session action, or set one as your
[Default AI CLI](./settings.md#default-ai-cli). Each is offered once its command (`codex`,
`opencode`) is installed where sessions run: on your `PATH`, or in the container's image when the
session service runs in one. A tool that starts sessions for you, through the app's tool server, can
ask for them with `ai_cli` set to `codex` or `opencode`.

- **It starts in the session's worktree**, running the bare command, and the sidebar and terminal
  bar label it `codex` or `opencode`. Sign in to the CLI yourself first: the app does not carry your
  credentials.
- **The session remembers its provider** across restarts of the app and the session service. The
  activity badge of a Codex or OpenCode session reads unknown, whether the CLI is printing or quiet:
  neither gives the app a reliable busy or idle signal, so it does not guess.
- **A restarted Codex session resumes its own conversation.** A little after Codex starts, the app
  finds the conversation it recorded in `~/.codex` (or `$CODEX_HOME`) for that folder and links it
  to the session; restarting the session then runs `codex resume` on that conversation, never on
  another session's. The link is made only when the match is certain. If two Codex sessions run in
  the same folder, or Codex has not recorded a conversation (you have not sent a first message, or
  it records nothing), no link is made and a restart opens a fresh conversation without an error.
  The app only reads Codex's records; it never changes or deletes them. Closing a session
  does not remove its conversation from Codex.
- **A Codex session is named from your first message** once Codex has recorded one, as sessions on
  the other CLIs are. Until then it keeps the usual placeholder name. Codex's activity badge reads
  unknown.
- **A restarted OpenCode session resumes its own conversation.** A little after OpenCode starts, the
  app asks the `opencode` command which conversation it recorded for that folder and links it to the
  session; restarting the session then runs `opencode --session` on that conversation, never
  `--continue` and never another session's. As for Codex, the link is made only when the match is
  certain: with two OpenCode sessions in the same folder, with no recorded conversation, or when the
  `opencode` command does not answer within two seconds, no link is made and a restart opens a fresh
  conversation without an error. The app only reads; it never changes or deletes OpenCode's
  conversations, and closing a session does not remove its conversation from OpenCode.
- **An OpenCode session is named from your first message** once OpenCode has recorded one. Until
  then it keeps the usual placeholder name.
- **Codex asks whether to trust a folder** the first time it runs there. When a tool asks for a
  first prompt in a folder Codex has not been told to trust, the prompt is not typed, so it cannot
  answer that question for you; run `codex` once in the project folder and accept it. OpenCode asks no such question.
- **A provider whose command is missing says so.** If `codex` or `opencode` is not on the `PATH`
  sessions get, it is not offered as an entry; the note under the list names it and says why a
  session would not find it and what to change. It cannot be started any other way either: a tool
  asking for it through the tool server is refused with the same reason before anything is
  created. Install the CLI and it is available the next time the list is opened or a
  session is started, with no restart of the app.
- **Neither is connected to the app's tool server** for now: the sessions start without it.
- **OpenCode does not update itself** inside a session the app starts.

### What the sidebar shows

Each session row carries a short text label naming its CLI — `claude`, `copilot`, `pi`, `codex` or `opencode`. It is text, not
a colour or an icon alone, so it reads the same way for everyone and survives a narrow sidebar: if
the row runs out of room the *title* is what shortens, never the CLI label.

Open a session and its terminal bar names the CLI too — on the AI tab at the bottom-right, beside
its sparkle, reading `claude`, `copilot`, `pi`, `codex` or `opencode` — so you can tell what you are talking to without going
back to the sidebar. It is there whichever pane the session is showing.

The busy/idle indicator works the same way for both CLIs — same shape, same states, no "less
certain" variant for one of them.

### Sessions you started outside this app

If you run `claude`, `copilot` or `pi` yourself in a worktree, this app finds that conversation the next
time you open the project and lists it as a session of that CLI. This happens on **every** open, not
just the first, so a conversation you start while the project is open shows up when you come back
to it.

Two things worth knowing about discovered sessions:

- **A session you closed here stays closed.** Closing writes a durable marker in the CLI's own
  storage, so it is not re-listed later even if this app's own records are lost.
- **A discovered session shows no busy/idle indicator until you start it here.** The app is not
  supervising it, so it makes no claim about what it is doing — it reads as unknown rather than
  guessing at idle. Select it and start it and it becomes an ordinary session, indicator included.
  (For Pi, the indicator also needs
  [Show activity for Pi sessions](./settings.md#show-activity-for-pi-sessions) on; with it off, a Pi
  session reads unknown by design.)

### Sessions on Pi

Pick **Pi Coding Agent** from the chevron beside the start-session action, or set it as your
[Default AI CLI](./settings.md#default-ai-cli). It is offered once `pi` is installed where sessions
run: on your `PATH`, or in the container's image when the session service runs in one. The sidebar
and the terminal bar label its sessions `pi`.

A few things about a Pi session are worth knowing:

- **Its conversation lives in Pi's own store**, not in this app — under `~/.pi/agent/sessions/`, or
  under `$PI_CODING_AGENT_DIR` if you set it. The app never moves or copies it. That means it is
  yours outside the app too: run `pi --resume` in the same worktree and pick it, or pass
  `pi --session-id <id>`, and you are in the same conversation the app shows.
- **It starts offline.** The app launches `pi` with its update check, version request and telemetry
  turned off (`PI_OFFLINE=1`, `PI_SKIP_VERSION_CHECK=1`, `PI_TELEMETRY=0`). If you set any of these
  yourself, your value is used.
- **Nothing in your Pi configuration is changed** — not your settings, not your models, not your
  extensions.
- **If the conversation was deleted from Pi's store**, resuming the session does not quietly start a
  new one: the terminal says Pi no longer has this conversation, and you can close the session or
  start a new one.

#### When a conversation is already in use

The app never runs two of its own sessions on one conversation: a session *is* its conversation, so
opening one that is already running shows you the running one and starts nothing.

What it cannot know is whether **you** have the same conversation open in a `pi` you started
yourself. Where a CLI leaves a sign that a conversation is live, the app warns you before starting
and lets you go ahead — the warning is advice, not a lock, and it can be wrong in both directions.
Pi leaves no such sign, so **on Pi that warning never appears**. The app says nothing rather than
guess; if you do run the same conversation in two places at once, both write to it.

### When a CLI isn't installed

- It is never offered — not in Settings, not in the per-session list.
- Sessions that already run it are **still listed and still labelled with it**. They do not
  disappear and they are not relabelled as something else.
- **Settings tells you before you start anything.** Under
  [Default AI CLI](./settings.md#default-ai-cli), and under *Image reference* when sessions run in a
  container, a note names each CLI missing from where sessions run.
- **Starting one tells you why, and starts nothing.** You get a clear failure rather than a terminal
  that never comes to life. The session's pane and an error banner name the CLI, give the reason for
  the session's directory and say what to change. The reasons are the six described under
  [Default AI CLI](./settings.md#default-ai-cli): for example "Source a script before each session"
  is off, "Script path" is empty, or the startup script was not found, failed or timed out. What it
  says about installing depends on which:
  - When the startup script was not found, failed or timed out, it does not mention installing: the
    fix is the script ("Script path", or "Timeout").
  - When no script is sourced ("Source a script before each session" is off, or "Script path" is
    empty), it says to turn that on or set it, and offers installing the CLI on the login `PATH` as
    the alternative (in a container, using an image that puts the CLI on its `PATH`).
  - When the script was applied, it says the CLI was not found on the `PATH` sessions get. Only
    then is installing the first advice, with making the script add the CLI's directory as the
    alternative.

  In a container whose image lacks the CLI, it names the image and says to choose one that provides
  it.
  - **A new session** is also offered another AI CLI.
  - **A restarted session** is never pointed at another CLI: its conversation lives in that CLI's
    own store, so it can only continue there. The message says to fix the cause and then restart
    the session.
- **Restart fails the same way until that is fixed.** So the pane does not suggest *restart* for
  this failure, as it does after a crash loop. Once you have fixed the cause, or the service runs
  from an image that has the CLI, press **restart** in the bar. The service has to restart before a
  newly chosen image is used, as described under [Container daemons](./settings.md#container-daemons).
- The banner appears once per failure. Pressing **restart** again with nothing changed fails again
  without a second banner, and the bar's `failed` is what remains.

### Reopening where you left off

The app remembers, per project, which session you had in front of you — and it remembers across
restarts. Quit with a session open and reopen later, and that session is in front of you again,
ready to type in, with no clicks.

- It comes back **whether or not it was still running**, and it **comes back up**. Sessions do not
  keep running while the app is closed, so the usual case is returning to a stopped one — and
  reopening resumes it, exactly as if you had clicked it in the sidebar. Arriving by reopening is
  not treated differently from arriving by clicking.
- **One session is resumed, not several**: the one you were looking at, in the project that opens.
  Other sessions in that project stay as they were, and a project you have not opened is untouched —
  its own last session waits until you switch to it.
- If the resume cannot happen — the project is open in another window, or its folder is unavailable
  — the terminal says what is actually true of the session rather than looking as though it were
  starting, and the `restart` control in the bar is there when you want it.
- Each project remembers its own, so switching projects takes you to that project's last session.
- If the session you were on has been **closed**, or its record has gone, the app opens the project
  as it otherwise would and leaves everything else alone. Closing a session does not wipe the
  memory — it just means there is nothing to return to.
- If its **worktree** was deleted outside the app, you still land on it, shown the way any session
  with a missing worktree is shown. You can see and select that session yourself, so the app returns
  you to it rather than pretending it is not there.
- **Forgetting a project** forgets which session it was on, along with everything else the app kept
  about it.

### Sessions an agent started

When an agent starts another session, the new session is listed one level under the session that
started it, not beside it as if you had started it. A session started by a session that was itself
started by an agent nests one level further down. The same tree is shown under a worktree and under
**Default**.

- **The marker.** Every session an agent started carries the word `agent` before the name of its AI
  CLI, for example `agent, claude`. A session you started has no such word.
- **It behaves like any other session.** Select it to open it, type into it, stop it, resume it or
  close it exactly as you would a session you started.
- **Origin pending.** If the session that started it is not in the list yet, the session is shown at
  the top level of its worktree and reads `agent, origin pending, ...`. It moves under its origin as
  soon as the origin appears.
- **Origin removed.** If the session that started it was deleted, the session stays at the top level
  and reads `agent, origin removed, ...`. It is never dropped.
- **No manual reparenting.** The nesting follows what the agent reported; you cannot drag a session
  under another.
- **Folding.** A session with children has an arrow before its name. Press it to fold the children
  away; every session below it is hidden, however deep. A folded session shows how many sessions it
  started directly, for example `claude, 3 agents`, and a dot at the end of the row when any hidden
  session below it needs your attention (the unread rule of the location rows). Press the arrow
  again to expand them. A session you never folded is expanded.
- **Folds are remembered.** Which sessions you folded is saved in the settings file
  (`folded_origins`) and is still folded after you restart the app. A fold is kept even while the
  session is not listed, for example before the first list arrives after a start, so it is never
  lost by a slow start.
- **Which providers.** Only a provider that reports which session started a session shows the tree.
  The table below says which do; a session of any other provider is listed as an ordinary session.

### Which providers report which session started them

The app never guesses the relationship from a name, a folder or a start time. It uses only what the
provider itself records, so a provider that records nothing is a gap, and its sessions stay at the
top level of their worktree exactly as before.

| Provider | Reports | Signal | When it does not report |
| --- | --- | --- | --- |
| Claude Code | No | None found: its sub-agents run inside one session and its transcript links are between messages, not sessions | Its sessions are listed at the top level |
| GitHub Copilot | No | Its sub-agent events are inside the same session's record, not separate sessions | Its sessions are listed at the top level |
| Pi Coding Agent | No | Its `parentSession` marks a fork of a session, not one an agent started | Its sessions are listed at the top level |
| Codex | Reports | The spawning thread named in the session's own record | A session whose record names none is listed at the top level |
| OpenCode | Reports | The parent session named in the session's own record | A session whose record names none is listed at the top level |

The Codex and OpenCode readers follow those providers' documented record formats and have not yet
been checked against a recorded real session. A parent the app does not know yet shows as
`origin pending` until it appears.

### Finding the session you are on

The session the terminal is showing is the **current** session, and the sidebar always says which
one that is:

- Its row is highlighted, and its name is set slightly heavier than the rows around it. The weight
  is there so the current session is still identifiable in a screenshot converted to greyscale, or
  by anyone who cannot separate the highlight from the hover shading beside it.
- The location holding it — a worktree, or **Default** — is **opened for you**, so the row is
  actually on screen rather than hidden inside a collapsed entry. This is what tells you where you
  are after switching projects, when every row would otherwise be collapsed.

You keep control of the panel:

- **Collapsing that row closes it for good**, for as long as you stay on the same session. It does
  not spring back open when a worktree is created or re-discovered in the background.
- **Nothing else is opened or closed on your behalf.** Other rows are left exactly as you had them.
- A row opened for you **stays** open when you move on to another session. Ceasing to be current
  takes away the highlight, never the open row.
- **Selecting a session yourself** highlights it and moves nothing — you were already looking at it.
- If the row would be **below the fold** in a project with many worktrees, the list scrolls just far
  enough to bring it into view. If it was already visible, the list does not move at all, and once
  you scroll the panel yourself nothing scrolls it back until you move to another session.
- After a fresh start, no session is current until you pick one or start one, and the sidebar says
  so by highlighting nothing.

## The embedded terminal, resume & restart

- The terminal runs the session's AI CLI with its working directory set to the session's worktree,
  so each session is scoped to its own branch.
- Type in the input line and press **Enter** to send input to the CLI; its output streams above.
- If a session's process exits unexpectedly, it is **automatically restarted**, resuming the prior
  conversation — the app asks the CLI to resume the session id it owns, so you come back to the
  same conversation rather than a fresh one. Repeated rapid failures stop the auto-restart and mark
  the session **failed** so you can retry manually. The pane then says why it gave up and what the
  last exit was — *"Gave up after 3 restart attempts — last exit: exit status 1."* — beside the
  **restart** control that resumes it, and the status bar under it reads `failed after 3 attempts`.
  A window you open afterwards is shown both, so a loop that ran while you were away is not reduced
  to the word *failed*.
- **Closing** the active project (or quitting the app) stops that project's session processes but
  keeps the sessions; reopening the project restores them and resumes the same conversations.
  **Switching** to another project does not stop them — see below.

> Requires the session's CLI — `claude`, `copilot`, `pi`, `codex` or `opencode` — where sessions run: on your `PATH`, or in
> the container's image. If it is missing, starting the session reports which one could not be found
> — see [When a CLI isn't installed](#when-a-cli-isnt-installed).

## Switching to a regular terminal

Each session's terminal can also run a plain shell instead of the AI CLI — useful for running git
commands, scripts, or anything else scoped to that session's worktree without leaving the app.

- The **tab strip** in the terminal's bottom bar is how you move between the AI CLI (`claude`,
  `copilot`, `pi`, `codex` or `opencode`) and a plain shell: press the AI tab at the right-hand end to talk to the CLI, press a
  numbered tab to get a shell. The marked tab is the one the pane is showing, so the strip is the
  single place to check which process your keystrokes are going to — it names where each press
  takes you rather than just saying "the other one".
- The shell starts with its working directory set to the session's worktree, same as the AI CLI —
  so `git status`, build scripts, and so on all run against the right branch.
- **Both processes keep running** while you switch — leaving the AI tab never stops or restarts
  the CLI, and leaving a shell's tab leaves that shell running in the background. Switching back
  reattaches to whichever process was already there, exactly as you left it.
- If the shell exits (you typed `exit`, or it crashed), a **restart** control appears in the same
  bar so you can start a fresh one; unlike the AI CLI, the shell never restarts on its own.
- Switching to a shell never stops, restarts, or otherwise touches your AI conversation —
  even mid-turn. It keeps running in the background exactly as it was, including its own
  crash-auto-restart if it happens to exit while you're looking at the shell, and switching back
  reattaches to that same conversation with nothing lost.
- **If a shell can't be started**, the pane stays on the AI tab, no new tab is left behind, and a
  notice says why:
  - **Worktree directory is missing** — the session's worktree folder was deleted or moved outside
    the app, so there is nowhere to start the shell. The app never starts it somewhere else
    instead. Recreate the worktree at the same path, or close the session.
  - **Couldn't open a terminal: …** — any other reason, followed by the service's own description.

<!-- media: switch-session-light -->

### Running more than one Regular Terminal instance

A session isn't limited to a single Regular Terminal — you can open as many independent shell
instances as you need, side by side.

- An **open a new instance** button — the "+" — sits in the bottom bar at the end of the numbered
  tabs. Press it (or use **Ctrl+Shift+T** / **Cmd+Shift+T** on macOS while the terminal has focus)
  to start another independent shell, scoped to the same session working directory as the first.
  The button is there even when only one instance is open, so you can always go from one to two.
- The keyboard shortcut only opens a new instance while the session is already showing a Regular
  Terminal — pressing it while the AI tab is marked does nothing and does not change tabs.
- Each instance is a fully separate shell process: running a long command in one never affects
  the others, and closing or restarting one instance never touches its siblings or your AI
  conversation.
- Each instance tracks its own running/exited state independently, including ones you're not
  currently looking at — see **the tab strip** below, which is where that state is reported.
- Closing a background instance leaves everything else exactly as it was. Closing the instance
  you're currently looking at automatically brings up the next one in the list (or the previous one,
  if you closed the last) — the pane is never left showing a closed instance. Closing your very last
  instance marks the AI tab.
- Coming back from the AI tab always returns you to whichever instance was last active — not an
  arbitrary one.

### The tab strip

The bottom bar carries a **tab strip** of everything the session can show you: one numbered tab per
open Regular Terminal instance, in the order you opened them, and then the AI conversation's own tab
at the right-hand end.

**Exactly one tab is always marked**, and it is the one whose content the pane is displaying — so
the strip tells you where you are without your having to press anything. Click a tab to bring that
pane to the front; whatever you switch away from keeps running untouched in the background.

The strip is **always there**, even in a session with one Regular Terminal or none at all. A
brand-new session shows a single tab — the AI conversation's — marked, because that is what you are
looking at.

#### The AI conversation's tab

It sits at the right-hand end and stays there as you open and close instances, so it is always one
press away. Three things make it different from its neighbours, and all three are deliberate:

- **It has no close button.** A session has exactly one AI CLI process, and ending it is not
  something this control offers — by any press. Every instance tab has one; the AI tab keeps the
  space and leaves it empty, so all the tabs stay the same size and the strip still reads as a strip.
- **Clicking it only switches the view.** It never starts, stops or restarts anything — not
  the AI CLI, and not any terminal instance — and clicking it while you are already looking at the AI
  conversation does nothing at all. Switching away and back returns you to the same terminal
  instance you left.
- **Its right-click menu is a terminal tab's minus Close** (see below).

#### What a right-click offers

**Right-click any tab** for what you can do to that process. On an instance tab that is **Restart**
(offered only while that instance's own shell is stopped) and **Close**; on the AI tab it is the
same menu without Close. The menu acts on the tab you clicked, not on whichever pane you happen to
be looking at.

If a right-click **does nothing**, that is the answer rather than a fault: the only thing the menu
could have offered is a restart, the process is running, and an empty panel would say there is
something to do here and then withhold it.

#### Which processes aren't running

**A tab whose process isn't running wears a small red ring** at its leading edge. It means "there is
something you can do here" — right-click that tab and **Restart** will be waiting. It appears on the
AI conversation's tab in exactly the same place and for the same reason, so one glance along the
strip tells you what is and isn't running.

- It shows for a process that has **stopped** — exited, crashed, or never started — and **not** for
  one that is still starting up. A starting process is on its way and there is nothing to do to it;
  the mark would only send you to a right-click that does nothing.
- It is independent of which tab is selected, so a tab can be both the one you're looking at and the
  one that isn't running, and it says both.
- It appears on a **background** instance without your having to select it. If one exits or crashes
  while you're viewing a different one, its tab gains the ring where you can see it — right-click
  and choose **Restart** to start a fresh shell for just that instance, without switching to it
  first, and without touching any sibling or your AI conversation.

#### When there are more tabs than fit

Past about five open instances the tabs need more width than the bar can give them. They **scroll**
rather than shrink: turn the mouse wheel over the strip to move along it. No tab is ever made
narrower, ellipsised or dropped, and the "+" and the AI tab keep their full size and position
however many instances are open — the AI tab in particular stays one press away rather
than being something you have to scroll to.

- **A faded edge means there is more that way.** When the tab you are *looking at* is the one out of
  sight, that edge takes the marked tab's own accent colour instead — so the fade tells you not just
  that there is more, but which way the pane you are in has gone.
- **Selecting a tab scrolls it into view**, so you never end up looking at a pane whose tab you
  cannot see. If you then scroll away by hand, it stays where you put it.

## Sessions in the background

Switching to a different project **does not stop your sessions**. When you change the active
project — from the top-bar project switcher, the **Known projects** list, or the folder browser —
the project you leave keeps all its sessions running in the background: their AI CLI processes
stay alive and their output keeps accumulating while you are away.

When you switch back, the project's sessions are still running and the session that was in the
foreground is shown again, exactly as you left it (other sessions stay in the background). Any
number of projects can hold running sessions at once.

If one of a background project's sessions exits unexpectedly while you are away, it is
auto-restarted under the same crash-loop guard as a foreground session. When you return to that
project a short **notice** tells you a background session was restarted — the state never changes
silently. Dismiss it with its **Dismiss** button.

### Closing the window does not stop your work

Sessions run in a background service, not inside the window, so **closing the last window leaves them
running.** An agent mid-task keeps working, and reopening the application re-attaches to it with the
output that accumulated while you were gone.

That service does not run forever either. **After 30 continuous minutes with nothing connected it
stops itself**, and reopening the application starts a fresh one. A session that was running when it
stopped comes back *resumable* rather than lost: selecting it resumes the same conversation where it
left off, and nothing is ever auto-resumed with nobody watching.

Nothing about this is a setting you have to find, and nothing needs installing to make it work — see
[the session service](../daemon.md) for what it does and does not promise, including what a reboot
costs you.

### Unread sessions

A session that finishes its turn, or stops to ask for a permission, while you are not looking at it
becomes **unread**. Its sidebar row then carries the **unread mark**: a small filled dot at the
right-hand end of the row, with the session's name set in a heavier weight.

- **It is not the activity dot.** The activity dot at the left of the name says what the session is
  doing *now* — working, awaiting input, ended — and changes as the session does. The unread mark
  says that the session finished a turn *since you last looked at it*, and it stays when the
  session starts working again. A row can carry both, either, or neither.
- **Looking at the session clears it.** The mark goes, within a second, when the session comes
  into view: you select it in a window that has the keyboard focus, you switch to its project and
  it is the session shown, its window regains the focus, or you close Settings and the session is
  shown again. Nothing else clears it, and there is no command to mark a session read or unread.
- **Every window agrees.** Unread state belongs to the session, not to a window: a session you
  read in one window loses its mark in all of them.
- **It does not depend on notifications.** The mark appears whether or not a desktop notification
  was shown: also with the **Desktop notifications** switch off
  ([Settings](settings.md#desktop-notifications)), and on a desktop that has no notification
  service.
- **It is there when you come back.** Sessions that finished a turn while the application was
  closed are unread when you open it, and a session that was unread when you closed the last
  window is still unread, also after a restart of the computer. A session you had read, and that
  has done nothing since, stays read.

#### Which worktrees have unread sessions

Each worktree row, and the **Default** row, that holds unread sessions carries the unread mark
followed by their number — `● 2` — at the right-hand end of the row, so you can see which worktrees
need you without expanding them.

- **What it counts.** The sessions of that worktree (or of the project root, on the **Default**
  row) that are unread, not closed, and not the session you are looking at. A row with none shows
  nothing.
- **Collapsed or expanded.** The number is there whether the row is collapsed or expanded; when it
  is expanded, each unread session row below it still carries its own mark. Expanding, collapsing
  or hovering a row reads nothing.
- **Looking at a session lowers it.** When an unread session comes into view, its row's number
  drops by one within a second, and the mark goes with the last one. A session that becomes unread
  raises it the same way.
- **In words.** Hover the row: its tooltip ends with the count in words, for example *2 unread
  sessions*.
- **Filtered-out worktrees.** A worktree the sidebar hides with a tag filter has no row to carry the number. Its unread sessions are
  counted only on the project switcher. The one exception is the worktree of the session you are
  in: its row stays listed, and carries its number like any other.

Unread state is kept on your computer with the rest of the session list and is sent nowhere.

### Being told when a session needs you

When a session you are not looking at stops to ask for a permission, or finishes a turn of a
minute or more — its activity dot changes to *awaiting input* — the application shows **one desktop
notification** for it. Its title is the session's name as its sidebar row shows it, followed by
what happened: *needs permission* or *finished a long task*. A shorter turn notifies nothing but
still marks the session unread ([Desktop notifications](settings.md#desktop-notifications)). The
text names the project and the worktree (or the **Default** entry, for a session without a
worktree). Nothing from the conversation itself is shown.

- **Never for the session in view.** A session is in view when it is the selected session of a
  window that has the keyboard focus, and that window shows the session (its AI conversation or one
  of its terminal tabs). Select another session, switch to another project, put another
  application in front, minimise the window, or open Settings, and the session is no longer in
  view: a change then notifies you.
- **Once per wait.** A session that is already waiting does not notify again because its CLI
  reminds you it is idle; it notifies again only after it has worked and stopped once more.
- **One, however many windows are open.** Several windows of the application share one
  notification per event, and several sessions that stop together each get their own.
- **Background projects too.** A session in a project you are not in notifies you with its own
  project's name.
- **Only while a window is open.** With no window open, nothing is shown — not at that moment and
  not when you open the application again. Sessions it already finds waiting when it starts raise
  nothing either; after a lost connection to the session service, a session that changed to
  waiting while it was away notifies you once.

The notification is shown through each system's own facility, on **Linux**, **macOS** and
**Windows**:

- **Linux** needs a desktop notification service (every major desktop has one).
- **macOS** asks for permission the first time the application shows a notification. Only the
  installed application bundle can show them, not a bare binary
  ([Installing on macOS](install-macos.md#notifications)).
- **Windows** shows them only for an installed build, one set up with the installer, not for an exe
  run from a folder ([Installing on Windows](install-windows.md#notifications)).

The system may ask you for permission, or withhold it: you may have refused, turned notifications off
for the application, or have Do Not Disturb on. The two install pages say where to allow or turn them
off. When nothing can be shown, because the permission is withheld, no service is running, or the
system refuses it, nothing else changes: sessions and their activity dots work as before, so the dot
still shows the state, no error appears in the application's window, and the failure is written to
its log once.

To turn them off inside the application, use the **Desktop notifications** switch in **Settings →
Environment** ([Desktop notifications](settings.md#desktop-notifications)). The unread marks stay.

#### Clicking a notification

**Clicking the notification opens the session.** The application's window comes to the front —
restored first when it is minimised — and shows the session: when the session belongs to a project
other than the one the window is in, the window switches to that project, exactly as the project
switcher does, and then selects the session. The session is then in view, so its unread mark goes.
Nothing else happens to it: a click does not stop, interrupt or restart the session, and types
nothing into it.

- **Which window comes forward.** With several windows open, the one that already has the
  session's project open comes forward and shows it; the click does not move the project to
  another window. When no window has that project open, the window you used last opens it.
- **A session that is gone.** When the session was closed or removed in the meantime, or its
  project was forgotten or its folder is no longer there, the window still comes forward, your
  selection stays as it was, and a notice says `That session is no longer available.`
- **A notification older than its window.** A click is reported to the window that raised the
  notification. When you have closed that window, clicking the notification does not open the
  session, also when other windows of the application are open, and with no window open it does
  nothing or starts the application as usual. The session's unread mark is still there to find it
  by.
- **macOS: one hour.** On macOS a notification stays in Notification Centre for one hour. After
  that the application removes it, and it can no longer be clicked. The session's unread mark
  stays.
- **A text-only notification service.** Some Linux notification services only show text and report
  no click. The notification is shown all the same; clicking it does nothing.
- **Wayland.** On a Wayland desktop an application cannot put its own window in front; the
  desktop decides. When the notification service passes an activation token with the click and
  the desktop accepts it — GNOME does — the window comes forward and takes the keyboard.
  Otherwise the window switches to the session and **asks for your attention** instead — most
  desktops highlight it in the taskbar or the overview — and takes the keyboard when you go to it.
  This was seen on GNOME Shell 50; other Wayland desktops may do either. On X11, macOS and
  Windows the window takes the keyboard focus itself.

### Claude plan usage

On a Claude Pro or Max plan, the app keeps track of how much of your plan's usage limits you have
used and when the next limit resets. The values come from Claude Code's own status-line data in
the Claude sessions the app runs: after each response, Claude Code hands its status line the
plan's `rate_limits`, and the app passes them to its background service. Nothing appears until a
Claude session started by the app has had a response.

- **No credential is read.** The app does not open your Claude login or call any web service.
- **Nothing is sent anywhere.** The values go from the session to the app's service over the local
  machine only.
- **Your own status line keeps working.** To keep its own copy of the values, the app sets the
  status line of the Claude sessions it starts. If you have a status line of your own, the app
  finds it and runs it with the same input, so you see exactly what you saw before. It looks only
  at the `statusLine` key of `.claude/settings.local.json` and `.claude/settings.json` in the
  session's folder, then of `~/.claude/settings.json` (or `settings.json` in `CLAUDE_CONFIG_DIR`
  when that is set), and reads nothing else from those files. The first one that has a status line
  is used, the same order Claude Code uses. A change to that key takes effect in sessions started
  afterwards; a change to your script itself takes effect at once.

Sessions on other AI CLIs, and Claude sessions you started outside the app, are not changed.

Once a Claude session has had a response, the app bar shows the usage at its right-hand end,
before the project switcher: the highest percentage among your plan's limits and when that limit
resets, for example **42% · 15:30**. The reset is in your local time, with the day added when it is
more than a day away. Hover the indicator to see every limit with its percentage and reset time,
and when the values were last updated.

When a limit's reset time passes, it drops out and the indicator shows the highest limit still
running. When none is left, or no Claude session has had a response since the app's background
service started, the indicator is hidden. There is no error or notice when the values are not
available, for example offline, signed out, or with an API key instead of a plan.

## Colored, real-terminal output

The embedded terminal renders the AI CLI's output like a real terminal, not as flat text:

- **Colors and styles** — ANSI foreground/background colors (the standard 16, bright, 256-color,
  and 24-bit truecolor) and text styles (bold, dim, italic, underline, strikethrough, and
  reverse/inverse) appear the same as in a standalone terminal.
- **Theme-aware defaults** — when output specifies no explicit color, the terminal's default
  text and background follow the app's light/dark theme and update when you switch themes. The 16
  ANSI colors use a fixed conventional palette so programs look as their authors intended. A
  program that asks the terminal whether its background is light or dark — `claude` does — is told
  the current theme, but it asks when it starts: after switching themes, restart it to pick up
  colours that suit the new background.
- **Full-screen interfaces** — the CLI's interactive UI and other full-screen (alternate-screen)
  programs redraw cleanly, with the cursor shown at its current position.
- **Focus** — the terminal you are looking at is where the keyboard goes, unless you have handed
  it away or something that types has taken it (a colored border marks the focused terminal).

<!-- media: session-terminal-light -->

## Interacting with the terminal

Start a session, or select one in the sidebar, and its terminal is focused right away — just type,
no click needed. (You can also click the terminal to focus it, e.g. after releasing focus.)
Keystrokes stream straight to the CLI as you press them, exactly like a standalone terminal:

- **Everything reaches the CLI**: printable characters, Enter, Backspace, Tab, arrow keys,
  Home/End/PageUp/PageDown, Insert/Delete, function keys, and control chords (Ctrl+C to
  interrupt, Ctrl+D, Ctrl+R, Ctrl+U, …). There is no "type a line and press Enter" box any more.
- **Paste** with the platform paste shortcut (Ctrl+Shift+V, or Cmd+V on macOS); the text is
  inserted into the CLI as input.
- **Select** text by dragging with the mouse (double-click selects a word, triple-click a line);
  the selection is copied to the clipboard automatically on release. **Copy** the current
  selection with Ctrl+Shift+C (Cmd+C on macOS); **middle-click** pastes. A plain click, without
  dragging, clears the selection and selects nothing, so it never replaces what is on the
  clipboard.
- **Mouse-driven programs**: when the running program turns on mouse reporting, mouse clicks are
  forwarded to it; hold **Shift** while dragging to select text instead.
- **Keys route to the terminal's process only while the terminal is focused.** When focused, every
  key — including Escape and shortcuts the app would otherwise use — goes to it; when not focused,
  those keys drive the application instead. Input is only delivered while the session's process
  is running (otherwise keystrokes are ignored and the header shows the session status).
- **Leaving focus**: press **Ctrl+Shift+E** (Cmd+Shift+E on macOS). Releasing focus never
  interrupts the running session. Clicking on empty app chrome no longer does it — see below.
  You rarely need this: going anywhere else in the application hands the keyboard over on its
  own, so the chord is for the times you want the app's shortcuts back without leaving the
  terminal you are looking at.
- **Pane shortcuts**: **Ctrl+Shift+D**, **H** and the arrow keys (Cmd+Shift on macOS) split and
  move between terminal panes; the app takes them and the terminal never sees them. See
  [Terminal panes](terminal-panes.md#keyboard-shortcuts).
- **Overview**: **Ctrl+Shift+O** (Cmd+Shift+O on macOS) opens and closes the
  [worktree Overview](overview.md), also from a focused terminal; the app takes it and the
  terminal never sees it.
- **Navigation**: **Ctrl+Shift+.** and **Ctrl+Shift+,** (Cmd+Shift+. and Cmd+Shift+, on macOS)
  step to the next and previous location of the project, in the order the sidebar draws them:
  Default, then the worktrees. They work whether or not a terminal is focused, and the terminal
  never sees them. Past the last location the next press wraps to the first, and back. Worktrees
  hidden by a tag filter or inside a collapsed run group are skipped. A location with a session
  shows its main session, as a click on its row does; an empty location is highlighted in the
  sidebar instead, and no session is started. The keyboard lands where it was: in the new
  session's terminal when a terminal had focus, otherwise on the sidebar row, which shows the focus
  ring (also on an empty location). The next press then steps on from that row; click the terminal
  to type in it again.
  **Ctrl+Shift+[** and **Ctrl+Shift+]** (Cmd+Shift+[ and Cmd+Shift+] on macOS) do the same for the
  sessions of the current location: the main session, then its helpers, wrapping at both ends. A
  collapsed row opens to show the session. Plain Ctrl+[ and Ctrl+] still reach the terminal.
  **Ctrl+Shift+PageUp** and **Ctrl+Shift+PageDown** (Cmd+Shift+PageUp and Cmd+Shift+PageDown on
  macOS; on a laptop keyboard Cmd+Shift+Fn+Up and Cmd+Shift+Fn+Down) switch to the previous and next
  project, in the order of the project switcher, exactly as picking it there does. Projects that
  are unavailable are skipped, and with only one available project nothing changes. All three
  levels wrap: past the last entry the next press goes to the first, and back.
- **Quick-switcher**: **Ctrl+Shift+P** (Cmd+Shift+P on macOS) opens a search box over the list of
  the active project's locations, each followed by its open sessions, also from a focused terminal;
  the app takes it and the terminal never sees it. The tag filter applies, as it does to the
  sidebar, and a session closed in the sidebar is not listed. Type part of a name: the letters
  must appear in that order, in any case, and names that start with what you typed come first.
  A session is found by its name; typing a location's name keeps its sessions listed right after
  it, so Down reaches them. Up and Down move the highlight, Enter or a click chooses; Escape, the
  same chord or a click outside closes it. Choosing is a row click and never starts a session: a
  location with a session shows it, an empty one is highlighted in the sidebar, and a session is
  shown with its row opened and selected in the sidebar.

### Links

Web and mail addresses in a terminal are links, and so is text a program marked as a link.

- **Hover** over an address such as `https://example.com/docs` or `mailto:team@example.com` and
  it is underlined, exactly the address and not the punctuation around it. The full address it
  opens shows in a small label at the bottom-left of the terminal, or at the top-left while the
  pointer is near the bottom. An address the terminal wrapped onto the next row is one link.
  An address needs its scheme to be recognised: `https://example.com` is a link, a bare
  `example.com` or `team@example.com` is ordinary text. Addresses that would hand the click to
  another application — `vscode://`, `slack://`, `javascript:`, `data:` — are never links either.
- **Open** it with **Ctrl+click** on Linux and Windows, or **Cmd+click** on macOS. The pointer
  turns into a hand while you hold Ctrl (Cmd) over a link, to show a click will open it. A web
  address opens in your default browser, a `mailto:` address in your default mail client.
- **Selecting still works as before.** A plain click, a drag, and a double or triple click select
  text even when they start on a link, and never open it.
- **Right-click a link** for **Open Link** and **Copy Link Address**, above the terminal's own Copy
  and Paste. Both act on the link you right-clicked, even if the output has moved since. Copy Link
  Address copies the complete address — an address the terminal wrapped onto the next row as one
  line, and, for a link a program declared, the address it declared rather than the word you see.
  Over anything that is not a link the menu offers Copy and Paste alone.
- **Programs that use the mouse** (`vim` with `:set mouse=a`, `htop`, …) get Ctrl+clicks as they
  always did. The pointer still turns into a hand over a link while Ctrl (Cmd on macOS) is held; the
  program may open it itself. Hold **Shift** as well, **Shift+Ctrl+click** (Shift+Cmd+click on
  macOS), to have the terminal open it; the underline shows only while Shift is held.
- **Links a program declares.** Some programs print a word such as `docs` that stands for an
  address. The whole word is marked as one link, and the label shows, and a click opens, the
  address the program declared, even when the visible text reads like another address.
- **Whether a program declares links is its own choice.** micold does not announce that its
  terminal shows declared links, and a session no longer inherits the identity of the terminal you
  started micold from (`TERM_PROGRAM`, `FORCE_HYPERLINK` and similar variables are removed), so a
  program cannot mistake micold for that terminal. To have programs that honour it, such as the AI
  CLIs, declare their links, add `export FORCE_HYPERLINK=1` to the script micold sources before each
  session (see [The environment a session starts in](settings.md#the-environment-a-session-starts-in))
  and start a new session.
- **Long addresses from an AI CLI.** An AI CLI that breaks its own lines to fit the terminal prints
  a long address as several rows, so only the first row's piece is a link and it opens a truncated
  address. Check the label before you click; with `FORCE_HYPERLINK=1` the CLI declares the whole
  address instead.
- **Files and folders on this computer.** A `file://` link — the kind `ls --hyperlink=always` prints
  — opens a document in the application set up for it, and a folder in your file manager.
- **A file your computer would run is shown, never run.** A program, a script with its execute bit
  set, an installer or a launcher (`.desktop` and `.AppImage` on Linux, an app bundle, `.command` or
  `.pkg` on macOS, `.exe`, `.bat`, `.ps1`, `.msi` and anything in `%PATHEXT%` on Windows) is revealed
  in your file manager with the file selected, so clicking a link can never start something. A folder
  is not a program, so it opens — except a macOS app or other bundle, which is shown.
- **A file on another machine is not a link.** `file://otherhost/share/report.docx` names a file this
  computer does not have, so it is not underlined and a click does nothing.
- **A file that is not there.** If the file has been moved or deleted since the program printed it,
  nothing opens and a notification says
  `Couldn't open file:///home/you/notes.txt: the file doesn't exist on this machine`.
- **Sessions in the sandbox.** A session running in the sandbox prints the paths it sees inside the
  container, which are not this computer's paths. A file in a folder the sandbox shares with your
  computer opens from the place on your computer that holds the same file: the label reads that
  path, and a click asks **Open a file from the sandbox?** naming it, because the sandboxed session
  could have written the file. Nothing opens until you answer.
- **Paths the sandbox does not share** are not reachable from your computer. Their labels read
  `/work/project/readme.txt — not reachable from this machine`, and a click opens nothing rather than
  the wrong file: a notification says
  `Couldn't open file:///work/project/readme.txt: the sandbox doesn't share that location with this machine`.
- **If the sandbox stops** while the question is on screen, answering **Open** opens nothing and a
  notification says `Couldn't open /home/you/project/readme.txt: the sandbox has stopped`.
- **When it can't open.** If nothing on your computer is set up to open the address, or the
  browser fails to start, a notification says so, for example
  `Couldn't open https://example.com/docs: no application is set up to open it`.

### One press does what you pressed

Every control in the window acts on the **first** press, whatever the terminal was holding. Press
a tab and the pane switches to it; press a session in the sidebar and it opens; press a toolbar
button and it fires. You never press something twice — once to get out of the terminal, once to
actually use it.

What a press does to the keyboard depends only on what you pressed:

- **Something that types** — a text field, or a menu or dialog that opens on the press — takes the
  keyboard, and hands it back when it closes.
- **Something that types nothing** — an icon button, a toggle, a menu item that performs an action
  — leaves the keyboard exactly where it was. Press it while typing in the terminal and you carry
  straight on typing.
- **Empty space, or a disabled control** — changes nothing at all. Inert space is not a way out of
  the terminal; the release chord and the release control are.

The release control is always in the bottom bar, and greys out when the terminal does not hold the
keyboard — it does not appear and disappear as you work.

### While you are typing somewhere else

The terminal never takes the keyboard out from under you. A dialog, a menu, the project switcher,
the sidebar's filter panel or a text field holds it for as long as it is open, and hands it back
when it closes — unless you had released the terminal first, in which case the keyboard stays with
the application.

Nothing that happens on its own moves the keyboard: not terminal output, not a background session
finishing its start-up, not a session changing state. Only you move it.

The terminal's own right-click menu is the exception that proves the rule — it belongs to the pane,
so opening it leaves the terminal holding the keyboard and you can carry on typing.

### Landing on a session ready to type

Anything that puts a different terminal in front of you leaves that terminal holding the keyboard,
so you can type straight away:

- selecting a session in the sidebar, or starting a new one;
- switching between AI CLI and Regular Terminal mode;
- opening, closing or switching a Regular Terminal instance;
- switching to a project whose session is restored;
- launching the app with a session restored from last time.

Going to a terminal on purpose also ends an earlier release — you asked for that terminal, so it
gets the keyboard. (Releasing is about the moment, not something a session remembers.)

### Leaving the app and coming back

Switching to another window and back changes nothing about where the keyboard is. If you were
typing in a terminal, keep typing — no click. If you had released the terminal, it stays released
and your app shortcuts keep working. If you were half-way through a dialog field, the caret is
still in it.

Nothing is saved and restored here; there is simply nothing for leaving the window to change.

### Pressing into the terminal

Pressing a terminal that does not hold the keyboard both gives it the keyboard **and** does what the
press would have done anyway — placing the cursor, starting a selection, or reaching a mouse-driven
program at the cell you pressed. No press is spent purely on focusing.

## Sizing, resize & scrollback

- The terminal tells the CLI how many rows and columns are actually visible, so its interface
  lays out to fit. **Resizing** the window or dragging the sidebar reflows the terminal and the
  running interface to the new size.
- **Scroll** with the mouse wheel *or* a touchpad to move back through earlier output (up to the
  scrollback limit). Two-finger touchpad scrolling works the same as a wheel, including the fine,
  slow gestures that move less than a line at a time. When a full-screen program has taken over the
  mouse, the scroll is forwarded to it instead.
- A **scrollbar** appears on the right edge of the pane while you are scrolled back, showing where
  you are in the history. Drag it to move, or click the track to page. It hides itself once you
  return to the live bottom, so no scrollbar simply means there is nothing scrolled back.

> The scrollback limit is configurable — see [Settings](./settings.md).

### Earlier output after a stop and start

When you stop a session and start it again, or its process exits and is restarted, the terminal
keeps what the session printed before. The earlier output stays in the scrollback, with its colours
and styles, and one dim line marks where the new run begins:

```text
── session restarted at 2026-10-02 14:31 +02:00 ──
```

The date and time are your local time when the session started again. Everything above the line is
from the earlier run; everything below it is new. A session that printed nothing before gets no
line. After several restarts you see each run's output with a line between them. History beyond
the scrollback limit is dropped, oldest lines first.

- The history is also kept across a restart of the background service, or of the computer, when the
  session was stopped or its process had exited before the restart. The service saves it to disk at
  that moment.
- After **Restart service**, after an update, after a logout or a reboot, and after the service
  stopped itself because no window was open for 30 minutes, no output is missing: the service saves
  every running terminal before it stops, and waits at most 3 seconds for that. On Windows the
  installer and **Restart service** ask the service to stop and give it 5 seconds before they end it.
- A session that is still running is saved at most every 30 seconds, and only when it printed
  something new. So after a crash or a power loss, up to the last minute of its output can be
  missing from the restored history. An orderly stop is not affected: see above.
- The saved histories are on your own computer only, one file per session in a `terminal-history`
  folder: `~/.local/share/micold-ai-ide` on Linux, `~/Library/Application Support/micold-ai-ide`
  on macOS, and `%LOCALAPPDATA%\micold-ai-ide\data` on Windows (the local profile, never the
  roaming one, so Windows does not copy it to other computers). Only you can read them.
- **Settings → Terminal → Save terminal history** turns the saving off (see
  [Settings](./settings.md#terminal)). With it off, a stop and start still shows the earlier
  output above the line; a restart of the background service or of the computer does not, and the
  files already saved are deleted.
- If a saved history cannot be read (the file is damaged, was written by another version of the
  app, or the service may not read it), the session starts and runs as usual and its terminal shows
  one dim line instead of the earlier output:

  ```text
  ── earlier output could not be restored ──
  ```

  Nothing of the unreadable file is shown, other sessions are not affected, and the session's new
  output is saved again as for any other. The reason is in the session service's log and among the
  recent errors in **Session service diagnostics**.
- **Regular Terminal** instances are not covered: a stopped and started shell starts with an empty
  terminal.
- An AI CLI that draws full-screen (Claude Code and Copilot CLI do by default) repaints its whole
  window, so only its last screen is kept above the line. The conversation itself comes back
  through the CLI's own resume, which the restart uses.
