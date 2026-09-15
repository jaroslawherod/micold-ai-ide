# Quickstart Part B: the manual pass (T082, T085)

**Date**: 2026-09-15 · **Site**: <https://jaroslawherod.github.io/micold-ai-ide/>, naming
`micold-ai-ide-v0.15.0` (released 2026-09-14) · **Browser**: Playwright Chromium 1234, headless, on
Linux · **Script**: [quickstart-b-pass.mjs](./quickstart-b-pass.mjs)

## Who read it

An agent read this, not a stranger with a stopwatch. It drove a headless browser against the
published site and judged from screenshots and computed styles. Two things follow from that:

- **The timings are the browser's.** They show how many steps each answer takes and that nothing on
  the way is slow. They do not show how long a person takes to read. B1 and B2 record the steps
  beside the seconds, so a reader can judge the human time.
- **The reader was not cold.** It had read this feature's spec before opening the site. B1's "can a
  stranger say what this is" is therefore judged from the text on the first screen, not from a
  stranger's reaction.

To re-run: copy `site/checks/package*.json` into an empty directory, `npm ci`, create `shots/`, and
run `node quickstart-b-pass.mjs`. It writes `result.json` and its screenshots there.

## Summary

| Step | Criterion | Result |
|---|---|---|
| B1 | SC-001: say what it does and reach Install in under 60 s, without scrolling | **Pass** |
| B2 | SC-006: find a topic in under 30 s | **Pass**, with a search defect (finding 1) |
| B3 | FR-029: one design language | **Partial**: the header does not read as the app bar (finding 2) |
| B4 | FR-015a, FR-030b: motion | **Pass**; FR-030a observed failing on the side (finding 3) |
| B5 | FR-025: on a phone | **Pass on emulation**. No real phone was used, so this step as written is **not run** |

## B1. A stranger can tell what this is

Viewport 1366×768, fresh context, no scrolling (`scrollY` 0).
[Screenshot](./quickstart-b1-home.png).

The first screen holds, in order:

- the heading **Micold AI IDE**;
- the sentence *"Run AI coding sessions on several branches at once. Micold AI IDE opens a git
  project, gives each piece of work its own worktree, and runs an AI CLI session in a terminal
  beside the code — in a background service that keeps every session alive when you close the
  window."*;
- the link row **Install · Read the user guide · Source on GitHub** (Install at y = 243 px);
- the application's window, with the demonstration project open.

- **What it does**: yes. The one sentence answers it, and the screenshot beneath it shows it.
- **Install reachable**: yes, in one click. The link goes to `install.html` (*Installing Micold AI
  IDE*).
- **Time**: the page loaded in 2.0 s, and Install was reached 0.6 s after the click. A person needs
  one sentence of reading and one click, well inside 60 s.

## B2. A reader can find a topic

Each search starts from a page that is not the answer. The toggle is clicked, the query is typed
key by key, and the matching result is clicked. The time runs from the toggle to the answer's
heading being visible.

| Topic | From | Query | Answer's rank | Landed on | Time |
|---|---|---|---|---|---|
| Revealing hidden agent worktrees | Settings | `agent worktrees` | 1 | Worktrees & sessions › Agent worktrees | 1.7 s |
| The scrollback limit | Opening a project | `scrollback` | 1 | Session daemon › Scrollback is bounded (2: Worktrees & sessions › Sizing, resize & scrollback) | 1.3 s |
| Running the service in a container | Help & About | `container` | 2 | Running the session service in a container › What the container can see (1: Settings › Container) | 1.2 s |
| Choosing your theme *(random)* | Installing | `theme` | 1 | Appearance & theming › Choosing your theme | 1.2 s |
| When a branch can't be used *(random)* | Session daemon | `branch` | 3 | Worktrees & sessions › When a branch can't be used | 1.1 s |
| Opening About *(random)* | Icons | `about` | — | **No search results for 'about'.** | — |

The random topics came from a seeded shuffle: three user-guide pages, then one heading from each.

**Judgement: pass.** Each search answer is a single query and at most three results to scan. The
About topic is also one click away in the sidebar (**Help & About**), and the sidebar is open on
every page at this width. Every topic can be found in far less than 30 s. The failed query is a
defect all the same.

### Finding 1: "about" returns nothing

A reader looking for the About dialog types the dialog's own name and gets *No search results for
'about'.* ([screenshot](./quickstart-b2-about-no-results.png)). The same result comes from the home
page. `about dialog` works (*Help & About › What the About dialog shows* is second), and so does
`help`. The likely cause is that mdBook's search index drops English stop words, and "about" is one
of them. The automated search check (page-checks.mjs) does not catch this, because it builds each
query from the page's heading, *Help & About*, and "help" survives.

## B3. One design language

The comparison is the home page in the light scheme, where a light screenshot sits on a light page.
The dark scheme was checked on *Appearance & theming*, where the dark screenshots sit on a dark
page.

Where the site and the application agree:

- **Colours and type**: the same roles in both schemes. The page is `rgb(253 248 253)` / `rgb(20 19
  22)`, the navigation drawer is `rgb(248 242 247)` / `rgb(28 27 30)`, and Roboto is used
  throughout. In dark, the screenshot's app bar and the page behind it are the same shade.
- **Surfaces**: separated by shade, with no outlines. The sidebar and the app bar have no border.
  Screenshots carry a 16 px corner and a level-1 shadow (`0 1px 4px`) instead of a frame. Inside
  the screenshots, the application does the same.

Judgement: the page content and its screenshots read as the same product, in both schemes.

### Finding 2: the header does not read as the application's app bar

Compare [the site's header](./quickstart-b3-site-header.png) with [the app bar in the
screenshot](./quickstart-b3-app-bar.png):

- **The title is pinned to the top of the bar.** `#mdbook-menu-bar .menu-title` is 64 px tall, but
  `site/theme/css/site.css` sets its `line-height` to the title-large token (28 px). The text
  therefore sits in the top 28 px, while the icons beside it are centred at 32 px. mdBook centres
  the title by giving it the bar's height as its line-height, and the override removed that. The
  same happens on a phone.
- **The title is centred horizontally.** The application's app bar starts its title at the leading
  edge, next to the actions.
- **The icons are not Material Symbols.** The bar's five icons (menu, theme brush, search, GitHub,
  edit) are mdBook's inline Font Awesome SVGs. FR-031 names the application's icon set.

Both schemes show all three. Everything else about the header matches: the surface-container shade,
no outline, the type, the height. But the header is the one piece of chrome FR-029a asks to *be*
the app bar, and it is the part that least resembles it.

## B4. Motion

Page: *Worktrees & sessions*, which has two clips.

- **Idle**: both clips are paused at 0:00, with `autoplay` off and `preload="none"`. Two frames of
  the first clip, taken 3 s apart, are byte-identical, and no clip file was requested before play.
  Each poster is the clip's first state, and the first one shows the window before *New worktree*
  is opened ([screenshot](./quickstart-b4-clip-poster.png)). The posters show the starting screen,
  not the action, so it is each clip's `aria-label` and the prose around it that say what the
  section is about.
- **Started**: only then was `create-worktree-light.webm` fetched. It is playing, `loop` is on, it
  is `muted`, and it decoded 0 audio bytes. Its duration is 11.4 s, under the 15 s limit.
- **Reduced motion** (`prefers-reduced-motion: reduce`, then reload): no element keeps a non-zero
  `transition-duration`, and both clips are paused again.

**Judgement: pass**, for FR-015a and FR-030b.

### Finding 3: the site's own transitions are mdBook's, not the application's (FR-030a)

Without reduced motion, the transitions are:

| Element | Property | Transition |
|---|---|---|
| `#mdbook-sidebar` | `transform` | `0.3s ease` |
| `#mdbook-page-wrapper` | `margin-left`, `transform` | `0.3s ease` |
| `.nav-chapters` (previous/next) | `color`, `background-color` | `0.5s ease` |
| header icons (`.fa-svg`) | `color` | `0.5s ease` |
| `#mdbook-searchbar` | `box-shadow` | `0.3s ease-in-out` |

Only some elements use the application's standard curve (`0.1s cubic-bezier(0.2, 0, 0, 1)`). The
sidebar slide is exactly the kind of transition FR-030a names ("an expanding navigation section"),
and it still uses mdBook's 300 ms `ease`. B4 does not check FR-030a; this was seen while probing
FR-030b.

## B5. On a phone

Chromium device emulation of iPhone 13 (390 px) and Pixel 7 (412 px), on *Worktrees & sessions* and
*Appearance & theming*, scrolled top to bottom so lazy images load.

| Device | Page | Viewport | Page scroll width | Media width | Body text |
|---|---|---|---|---|---|
| iPhone 13 | Worktrees & sessions | 390 | 390 | 342 ×4 | 16 px |
| iPhone 13 | Appearance & theming | 390 | 390 | 342 ×5 | 16 px |
| Pixel 7 | Worktrees & sessions | 412 | 412 | 364 ×4 | 16 px |
| Pixel 7 | Appearance & theming | 412 | 412 | 364 ×5 | 16 px |

There is no horizontal scrolling. Every image and clip fits inside the viewport with its margins.
Body text is 16 px, with `width=device-width, initial-scale=1`, so no zooming is needed. Outside
code blocks and tables, which scroll inside their own box, nothing overflows.

**Judgement: pass on emulation.** The quickstart asks for a real phone, and emulation does not show
how a real browser renders fonts, handles safe areas, or plays video. **B5 as written is not run.**

## Not covered

- A real phone (B5).
- A person's reading time (B1, B2): see *Who read it*.
- Safari and Firefox. Every step ran in Chromium.
- As the quickstart says, the clips are step-captured, so this pass shows nothing about the
  application's own motion.
