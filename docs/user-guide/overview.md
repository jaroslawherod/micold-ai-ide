# The worktree Overview

The **Overview** shows every location of the open project at a glance: one card for the project root
(**Default**) and one for each worktree, so you can see which sessions need you without opening
each worktree in turn.

## Layout

The cards sit in a grid of up to five cards per row. The cards share the row's width equally, so a
wider window makes each card wider rather than adding more cards to a row. A narrower window shows
fewer cards per row, down to one. With more cards than fit on screen, the grid scrolls down; it
never scrolls sideways.

A card is never shorter than it used to be, and grows taller when it has more to show, such as a
branch, an activity line, task progress and pull requests. Every card in a row is as tall as the
tallest card in that row, so rows stay even and nothing is cut off.

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

## Full screen

To give the cards the whole window, press **Full screen** at the right end of the Overview's
header, or press **Ctrl+Shift+F** (**Cmd+Shift+F** on macOS). The sidebar, its collapsed strip
included, slides away and the cards fill the window's width; the top bar stays, with the Overview
button, its badge and the project switcher. The button then reads **Exit full screen**; press it or
the shortcut again and the sidebar slides back. The button's tooltip names the shortcut, and Tab
reaches it. While a card's menu is open, the shortcut does nothing.

Entering and leaving full screen is one motion: the cards widen (or narrow) and slide to their new
places as the sidebar goes or returns, never overlapping. A card that moves to another row slides
along the row ends instead of vanishing and reappearing. Press the button or the shortcut again
part-way and the motion reverses from where it is. The card at the top of your view stays in view
through the motion, even when the cards move to another row. To have no motion at all, turn on
[Reduce motion](settings.md#reduce-motion) in Settings: full screen then switches in one step.

Leaving the Overview leaves full screen in the same step: activating a card, **Escape**, the
top-bar **Overview** button or **Ctrl+Shift+O** close the Overview and bring the sidebar back. The
sidebar always returns exactly as you left it: collapsed or expanded, at the same width. Full
screen never changes the sidebar itself.

Opening and closing [Settings](settings.md) keeps full screen, and so does switching project. The
Overview always opens in the normal layout: full screen is not remembered once the Overview closes,
nor across restarts.

## One card per location

Each card has a **header row** across its top, tinted in its state's colour (see [The five states, worst first](#the-five-states-worst-first)), over a body that is the same neutral colour on every card.

The header row shows:

- at its start, the card's **state**, as an icon and a word. Hover it for the reason;
- at its end, the location's **type**, as an icon and a word (see the table below). Hover it for the type's word.

The body below it shows:

- its **title**, which says what the location is for (see [Title](#title)), on up to three lines;
- below it, the **branch** in short: a conventional branch such as `fix/1115_container-daemon-fails-to-start` reads `fix/1115` (and `feat-813_worktree-overview` reads `feat/813`), any other branch reads as it is. A card whose title already is the branch does not repeat it;
- its **sessions line**: *No sessions*, *1 session*, *3 sessions*, followed by how many run when some do (*1 session · 1 running*);
- when the agent reports it, an **activity line** (see [What the agent is doing now](#what-the-agent-is-doing-now));
- a **progress row**, when there is progress to show (see [Feature progress](#feature-progress));
- for a card with a branch, its **pull request counts** (see [Pull request counts](#pull-request-counts)).

A row with nothing to say is not shown.

### Title

For a worktree named after a GitHub issue number (such as `feat-813_worktree-overview`) the title is the **issue's title**, once the app has it. Otherwise it is the **branch**, or the location's name when there is no branch: Default's title is *Default*. A long title wraps onto up to **three lines**, and a title longer than that ends in an ellipsis (…) on the third. To read it in full, see [Reading a card in full](#reading-a-card-in-full).

Titles come from the issues the New-worktree form already loaded, or from one read of each issue when the Overview opens, when the project is on GitHub and `gh` is available. A Jira ticket, an offline machine, a closed issue or a missing `gh` leave the branch as the title. Each title is read once while the app runs, and is never written to a log.

## Reading a card in full

Rest the pointer on a card for a moment and it **expands in place**: its top-left corner and width stay where they are, and it grows downward, drawn over the cards below it with a shadow that sets it apart. No other card moves. The expanded card shows, wrapped and with no ellipsis:

- the full **title**, and the location's name when it differs from the title;
- the full **branch** name, when the title is not the branch;
- the **worktree directory**, with your home folder written as `~` (`~/code/repo/.claude/worktrees/feat-813_worktree-overview`);
- the full **activity line**;
- every other row of the card, as it reads collapsed.

Move the pointer off the card and it **collapses**; rest it on another card and that one expands instead, so at most one card is expanded at a time. A card near the bottom of the Overview is moved up just enough to be seen whole. A card taller than the Overview is as tall as the Overview, keeps its header row and title in view, and scrolls the rest inside the card with the mouse wheel; once its content is at its end, the next wheel gesture scrolls the Overview.

While a card menu is open (see [Acting on a card](#acting-on-a-card)), the card it is for keeps its form, expanded or not, and no other card expands until the menu closes. Switching to another window, or closing the Overview, collapses the card; reopening the Overview shows every card collapsed.

The title, branch and activity have no tooltip of their own: the expanded card shows them in full. The header row's state and type words keep theirs.

## Feature progress

The progress row reads the worktree's own files, so it needs no session. It shows the feature's **phase** and, when there is a task list, the **ratio of ticked tasks** with a bar.

- **Ratio**: the ticked checkboxes over all checkboxes in the feature's `tasks.md`, such as *Implement · 6/10*. The feature folder is the one named in `.specify/feature.json`, else the one `specs/<number>-*` folder matching the issue number.
- **Phase**: *Spec*, *Plan*, *Tasks*, *Implement*, *PR* or *Done*. The first of these that applies wins:
  1. the autopilot ledger's `Phase` line (`autopilot.md` in the feature folder) wins over everything else;
  2. *Done* when the card is **Done**;
  3. *PR* when the branch has an open pull request;
  4. otherwise what the files imply: `tasks.md` with a ticked task is *Implement*, `tasks.md` alone is *Tasks*, then `plan.md` is *Plan*, then `spec.md` is *Spec*.
- **No bar without a ratio**: a phase without a task list shows the phase alone.
- **Nothing to show**: with neither a phase nor a task list, the card has no progress row.

The files are watched only while the Overview is open: each worktree is read once on opening and again shortly after a change, so a card follows the files within 5 seconds.

## What the agent is doing now

When the location's main session runs an agent CLI with **hooks configured** (Claude Code with the app's hooks installed), the card shows an **activity line** above the progress row: the agent's in-progress todo, else the latest prompt you submitted, with the todo list's done / total after it (*Writing tests · 2/5*). The task-list ratio of the progress row stays below it as the feature's progress.

The line shows the words only, wrapped onto up to **two lines**, and a longer one ends in an ellipsis (…) on the second. A prompt over several lines is joined, and agent markup is removed, so a prompt another agent sent as `<agent-message from="orchestrator">Continue M2</agent-message>` reads *Continue M2*. A slash command reads as a sentence: `/speckit-autopilot #1117` reads *Running autopilot on #1117* and `/clear` reads *Running clear*; every command reads the same way, built-in ones included. When nothing readable is left, such as a prompt that is only markup, the card shows no activity line, and no done / total on its own. The line only ever shows the main session's activity, never another session's.

Without hooks, with a session that has reported nothing yet, or while the window reconnects to the service, there is no activity line and the card shows the title and progress described above, without an error.

The todo text and prompt stay on your machine: the hooks report them to the app's own service, which hands them to the window over the same local connection. They are held in memory only, never written to a log or to the saved session list, and sent nowhere else.

## Pull request counts

The row shows the branch's open and merged pull requests, such as *1 open · 2 merged*; drafts count as open, and a count of 100 or more reads *100+*. A count of 0 is left out (*1 open*, *2 merged*), and with both at 0 the card has no pull request row.

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

Click a card, or move to it and press **Enter** or **Space**. A click shows a ripple that spreads from the point you pressed across the card; **Enter** or **Space** shows it spreading from the card's centre. The Overview closes about 0.3 seconds later, so you see the card respond. While it waits, further clicks, **Enter**, **Space** and **Escape** change nothing: the first card you activated stands. Then one of three things follows:

- **The location has a session**: its main session is selected and shown, as if you had clicked it in the sidebar.
- **The location has no session**: its row is revealed in the sidebar (scrolled into view and marked as selected), and the menu of AI CLIs opens at the row's start button. Nothing starts until you pick one.
- **An orphan or invalid location** (a missing or broken worktree, where no session can start): its row is only revealed in the sidebar.

The revealed mark stays until you select something else in the sidebar. A card in any state, **Idle** included, activates the same way.

## Acting on a card

Right-click a card (on macOS, Ctrl+click or a two-finger click) to open its menu at the pointer. It
lists **Open** first, which does what clicking the card does, followed by exactly the entries of
that location's menu in the sidebar, in the same order (for example *Review changes*, *Copy name*,
*Rename*, *Delete*). An entry does what it does from the sidebar, including any confirmation it
asks for.

- **Entries that navigate** (*Open*, *Review changes*, an entry that starts or selects a session)
  close the Overview and show their result.
- **Every other entry** (rename, delete, copy, bind to a daemon) acts in place: the Overview stays
  open, any dialog the entry opens shows over it, and the card reflects the result.

While the menu is open, its card is marked as the target: a bold outline in the accent colour and a
tinted fill, unlike the thin ring of keyboard focus or the light tint of hovering. The mark stays
on that card wherever the pointer goes and goes when the menu closes.

**Escape**, a click outside the menu, or scrolling the grid closes the menu without running
anything; a click that closes it does not also activate what it landed on. Right-clicking empty
space in the Overview closes it too, and right-clicking another card closes the menu and opens
that card's. The menu also closes when its location goes away or the
Overview closes.

From the keyboard, press the **Menu** key or **Shift+F10** on a focused card to open the same menu
inside the card, with **Open** highlighted. **Down** and **Up** move through the entries (from the
last back to the first, and the reverse), and **Enter** or **Space** runs the highlighted one. The
same keys work on a menu opened with the pointer. **Escape** closes the menu and puts the keyboard
back on its card; after an entry that leaves the Overview open, the keyboard returns to the card
too, or to the first card if that one is gone. With no card focused the keys do nothing.

## Keyboard

Opening the Overview puts the keyboard on its first card. **Tab** and **Shift+Tab** move to the next and previous card; the arrow keys do the same (**Right** and **Down** forward, **Left** and **Up** back). The focused card shows a ring. **Menu** or **Shift+F10** opens its menu (see *Acting on a card*). **Escape** closes the Overview, after any dialog or unsaved Settings draft that Escape would close first. **Ctrl+Shift+F** (**Cmd+Shift+F** on macOS) enters and leaves [full screen](#full-screen) while the Overview is open. While a terminal has the keyboard, the arrow keys, Escape and the full-screen shortcut go to the terminal.

## The five states, worst first

Cards are ordered by state, worst first, and in the sidebar's order within a state. The first row
below that matches decides the state.

The state's colour is the card's header row: red for **Needs action**, amber for **Needs
attention**, the accent colour for **Working**, green for **Done**, and grey for **Idle**, so an
Idle card stands back beside the others. The body of every card keeps one neutral colour, and its
text reads the same in every state, in the light and the dark theme.

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
