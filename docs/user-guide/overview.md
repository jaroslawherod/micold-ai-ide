# The worktree Overview

The **Overview** shows every location of the open project at a glance: one card for the project root
(**Default**) and one for each worktree, so you can see which sessions need you without opening
each worktree in turn.

## Opening and closing it

Press **Overview** in the top bar, next to the project switcher. Press it again to close the
Overview. The button is greyed out while [Settings](settings.md) is open; an Overview that was open
before Settings shows again when Settings closes. Selecting a session in the sidebar also closes
the Overview and shows that session.

The shortcut **Ctrl+Shift+O** (**Cmd+Shift+O** on macOS) does the same, also while a terminal has the
keyboard; the app takes it and the terminal never receives it. The button's tooltip names it. The
button looks pressed while the Overview is open.

### The badge

When some location needs you, the button carries a badge: an icon and a count, beside the label. A
red exclamation mark with *N* means *N* locations **need action**; if none does, an amber eye with
*N* means *N* **need attention**. Hover the button for the words (*2 need action*). With neither,
there is no badge. The badge shows whether or not the Overview is open.

The Overview replaces the main area; the sidebar and the top bar stay. Switching project keeps it
open and shows the new project's cards.

## One card per location

Each card shows:

- the location's **type**, as an icon and a word (the word leads the footer line, and is also the icon's tooltip);
- its name, and the branch below it;
- a one-line summary (see [The summary line](#the-summary-line));
- its session counts: how many sessions there are and how many are running;
- its **state**, as an icon and a word. Hover for the reason;
- when the agent reports it, an **activity line** (see [What the agent is doing now](#what-the-agent-is-doing-now));
- a **progress row** (see [Feature progress](#feature-progress));
- for a card with a branch, its **pull request counts**, such as *1 open · 2 merged* (drafts count as open; a count of 100 or more reads *100+*).

## The summary line

The summary says what the location is for. For a worktree named after a GitHub issue number (such as `feat-813_worktree-overview`) it is the **issue's title**, once the app has it. Otherwise it is the **branch** (or the display name when there is no branch). Default reads *Project root*.

Titles come from the issues the New-worktree form already loaded, or from one read of each issue when the Overview opens, when the project is on GitHub and `gh` is available. A Jira ticket, an offline machine, a closed issue or a missing `gh` leave the branch as the summary. Each title is read once while the app runs, and is never written to a log.

## Feature progress

The progress row reads the worktree's own files, so it needs no session. It shows the feature's **phase** and, when there is a task list, the **ratio of ticked tasks** with a bar.

- **Ratio**: the ticked checkboxes over all checkboxes in the feature's `tasks.md`, such as *Implement · 6/10*. The feature folder is the one named in `.specify/feature.json`, else the one `specs/<number>-*` folder matching the issue number.
- **Phase**: *Spec*, *Plan*, *Tasks*, *Implement*, *PR* or *Done*. The first of these that applies wins:
  1. the autopilot ledger's `Phase` line (`autopilot.md` in the feature folder) wins over everything else;
  2. *Done* when the card is **Done**;
  3. *PR* when the branch has an open pull request;
  4. otherwise what the files imply: `tasks.md` with a ticked task is *Implement*, `tasks.md` alone is *Tasks*, then `plan.md` is *Plan*, then `spec.md` is *Spec*.
- **No bar without a ratio**: a phase without a task list shows the phase alone.
- **Empty state**: with nothing to read, the row says *No task progress* and shows no bar.

The files are watched only while the Overview is open: each worktree is read once on opening and again shortly after a change, so a card follows the files within 5 seconds.

## What the agent is doing now

When the location's main session runs an agent CLI with **hooks configured** (Claude Code with the app's hooks installed), the card shows an **activity line** above the progress row: the agent's in-progress todo, else the latest prompt you submitted shortened to one line, with the todo list's done / total after it (*Writing tests · 2/5*). The task-list ratio of the progress row stays below it as the feature's progress.

Without hooks, with a session that has reported nothing yet, or while the window reconnects to the service, there is no activity line and the card shows the summary and progress described above, without an error.

The todo text and prompt stay on your machine: the hooks report them to the app's own service, which hands them to the window over the same local connection. They are held in memory only, never written to a log or to the saved session list, and sent nowhere else.

## Pull request counts

The counts come from the pull request status the app already reads; the Overview starts no reading of its own. When they cannot be read, the card shows a dash (*—*) and its tooltip says why:

- *pull request status is off*: the pull request status setting is off;
- *GitHub is not reachable*: the last reading could not reach GitHub;
- *rate-limited until HH:MM*: GitHub asked the app to wait, and readings resume then;
- *not read yet*: no reading has covered the branch yet.

Default, a detached HEAD and an orphan have no branch, so they show no counts. An open pull request that cannot merge because of a conflict makes its card **Needs attention** with the reason *merge conflict*.

| Icon | Type | Meaning |
|------|------|---------|
| Folder | Project root | The Default entry |
| Light bulb | Feature | A worktree whose branch is a `feat` branch |
| Bug | Bugfix | A `fix` branch |
| Wrench | Chore | Any other conventional type, such as `chore` or `docs` |
| Folder | Other | A branch with no conventional type |
| Link | Included | A worktree included from elsewhere |
| Crossed-out folder | Orphan | A worktree that is missing or invalid |

## Activating a card

Click a card, or move to it and press **Enter** or **Space**. The Overview closes, and one of three things follows:

- **The location has a session**: its main session is selected and shown, as if you had clicked it in the sidebar.
- **The location has no session**: its row is revealed in the sidebar (scrolled into view and marked as selected), and the menu of AI CLIs opens at the row's start button. Nothing starts until you pick one.
- **An orphan or invalid location** (a missing or broken worktree, where no session can start): its row is only revealed in the sidebar.

The revealed mark stays until you select something else in the sidebar. A card in any state, **Idle** included, activates the same way.

## Keyboard

Opening the Overview puts the keyboard on its first card. **Tab** and **Shift+Tab** move to the next and previous card; the arrow keys do the same (**Right** and **Down** forward, **Left** and **Up** back). The focused card shows a ring. **Escape** closes the Overview, after any dialog or unsaved Settings draft that Escape would close first. While a terminal has the keyboard, the arrow keys and Escape go to the terminal.

## The five states, worst first

Cards are ordered by state, worst first, and in the sidebar's order within a state. The first row
below that matches decides the state.

| State | Icon | What puts a location here (the tooltip reason) |
|-------|------|------------------------------------------------|
| **Needs action** | Exclamation mark | A session needs permission (*needs permission*); an agent request is waiting for your confirmation (*agent request waiting*) |
| **Needs attention** | Eye | A session failed (*session failed*) or can be resumed (*resumable*); the branch's open pull request has failing checks (*checks failing*), requested changes (*changes requested*) or a merge conflict (*merge conflict*) |
| **Working** | Play | A session is running (*working*) |
| **Done** | Check circle | The branch's pull request is merged and the worktree can be removed (*merged*) |
| **Idle** | Pause | None of the above (*idle*), or the activity is not known (*activity unknown*) |

Pull-request reasons need the pull-request status reading switched on in [Settings](settings.md);
without it a card never shows them.

## Live updates

The cards follow the sidebar: a session changing state, a worktree added or removed, or a new
pull request reading updates the Overview as it happens. Nothing needs refreshing.

## When the connection is lost

If the connection to the session service drops, the banner at the top says so. The cards keep
showing the last session states they knew, which may be out of date, and they make no claim about
pull requests until the connection is back.
