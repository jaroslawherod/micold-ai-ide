# The worktree Overview

The **Overview** shows every location of the open project at a glance: one card for the project root
(**Default**) and one for each worktree, so you can see which sessions need you without opening
each worktree in turn.

## Opening and closing it

Press **Overview** in the top bar, next to the project switcher. Press it again to close the
Overview. The button is greyed out while [Settings](settings.md) is open; an Overview that was open
before Settings shows again when Settings closes. Selecting a session in the sidebar also closes
the Overview and shows that session.

The Overview replaces the main area; the sidebar and the top bar stay. Switching project keeps it
open and shows the new project's cards.

## One card per location

Each card shows:

- the location's **type**, as an icon (hover for the word);
- its name, and the branch below it;
- a one-line summary (the branch, or *Project root* for Default);
- its session counts: how many sessions there are and how many are running;
- its **state**, as an icon and a word. Hover for the reason.

| Icon | Type | Meaning |
|------|------|---------|
| Folder | Project root | The Default entry |
| Light bulb | Feature | A worktree whose branch is a `feat` branch |
| Bug | Bugfix | A `fix` branch |
| Wrench | Chore | Any other conventional type, such as `chore` or `docs` |
| Folder | Other | A branch with no conventional type |
| Link | Included | A worktree included from elsewhere |
| Crossed-out folder | Orphan | A worktree that is missing or invalid |

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
