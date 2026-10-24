//! The one list (feature 020, T009 — [contracts/gallery-catalogue.md]).
//!
//! Two things read this module and they must not be able to disagree: [`super::gallery`] builds the
//! page by traversing it, and `tests/showcase_completeness.rs` holds it complete against the library.
//! So every entry carries **both** its names and the function that renders its own instances — an
//! entry cannot be declared without something to show, and an instance cannot appear without being
//! declared. A `match` on component names in the view would have allowed exactly that gap: an entry
//! with no arm renders nothing and passes a name-only check, which is the silent omission FR-012
//! exists to prevent, arriving through the back door.
//!
//! Three `const` slices, fully known at compile time. No registration at startup, no lazy
//! initialisation, no ordering that depends on anything — which is what makes FR-022's "the same
//! components, the same sample data and the same ordering on every launch" structural.
//!
//! # Adding a component
//!
//! 1. Add an [`Entry`] naming its module and type.
//! 2. List its named variants, and any other posed state (`disabled`, `selected`, …).
//! 3. List what has to be exercised live, and set [`Entry::interactive`] to match.
//! 4. Write its `render` in [`super::sections`], from the real component and [`super::samples`].
//! 5. If it has no appearance of its own, add an [`Exemption`] with the reason instead.
//!
//! The build tells you when you have not: the completeness check names what is missing, in either
//! direction.
//!
//! [contracts/gallery-catalogue.md]: https://github.com/jaroslawherod/micold-ai-ide/blob/main/specs/020-component-showcase-gallery/contracts/gallery-catalogue.md

use iced::Element;
use micold_core::tokens::Roles;

use super::sections;
use super::state::{Message, Showcase};

/// How an entry builds its instances.
///
/// Takes the whole [`Showcase`] so it can reach the replay counter it owns and the sample grid it
/// draws from; the active [`Roles`] so it resolves the current scheme's tokens — never a resolved
/// colour and never a style value (FR-010); and its own catalogue index, so a trigger it renders can
/// name itself (`Message::Replayed(index)`) without the index being duplicated into the entry as a
/// field that could drift from its position.
pub type Render = for<'a> fn(&'a Showcase, Roles, usize) -> Element<'a, Message>;

/// Which part of the page an entry renders in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// The component grid: instances posed side by side.
    Components,
    /// The motion section: a replayable demonstration, for a component whose appearance *is* an
    /// animation (FR-007a). Posing such a component as a still would be a picture of it.
    Motion,
}

/// How an entry shares horizontal space with its siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Chunked into rows alongside other inline entries.
    Inline,
    /// Its own full-width row, for a component whose natural size dwarfs its neighbours — a banner
    /// beside a chip. Without this, one oversized component pushes the rest off screen (spec, Edge
    /// Cases).
    FullWidth,
}

/// One component's place in the gallery.
pub struct Entry {
    /// The library module it is declared in, keyed as the shared inventory keys it — e.g.
    /// `material/button.rs`, `cdk/overlay.rs`. Half of the identity, because two modules each declare
    /// a `Surface`.
    pub module: &'static str,
    /// The component's type name — e.g. `Button`. Also its heading on the page (FR-001).
    pub component: &'static str,
    /// Named variants posed as separate instances (FR-003). Must match the library's enum variant
    /// names exactly; the completeness check holds both directions (FR-013).
    pub variants: &'static [&'static str],
    /// Density steps posed (FR-003a). **Empty on every entry at delivery**: no component honours a
    /// density step until feature 018 introduces the axis, at which point it adds the rows and the
    /// check starts holding them.
    pub density: &'static [&'static str],
    /// Other states posed as separate instances: `"disabled"`, `"selected"`, `"empty"`, … (FR-003).
    pub posed: &'static [&'static str],
    /// States that must be exercised with the pointer or keyboard rather than posed (FR-004). Shown
    /// as the section's caption, so a state absent from the page reads as live rather than missing
    /// (FR-005).
    pub live: &'static [&'static str],
    /// Whether the instances respond to a pointer or the keyboard.
    ///
    /// This is what makes FR-005 checkable. `interactive` and [`Self::live`] must agree — a non-empty
    /// `live` if and only if `interactive` — so the caption can neither go missing where a response
    /// is expected nor promise one that never comes. `tests/showcase_captions.rs` holds it.
    pub interactive: bool,
    /// Which part of the page this renders in.
    pub section: Section,
    /// How it shares its row.
    pub layout: Layout,
    /// Its instances. See [`Render`].
    pub render: Render,
}

/// One animation's place in the motion section.
pub struct MotionEntry {
    /// The helper's name as the library exposes it — `fade`, `expand`, `scale`, `scrim`. Matched
    /// against the `pub fn`s in `material/animation.rs`, both directions (FR-013a).
    pub animation: &'static str,
    /// What the entry is called on screen, so a developer comparing against a motion specification
    /// knows which one they are watching (FR-007c).
    pub label: &'static str,
    /// The demonstration, wired to this entry's replay trigger (FR-007b).
    pub render: Render,
}

/// A recorded statement that a component has no gallery instance.
pub struct Exemption {
    /// The exempted component's module.
    pub module: &'static str,
    /// Its type name.
    pub component: &'static str,
    /// Why it cannot be shown (FR-015). Mandatory: a blank reason fails the build, because an
    /// exemption without one is indistinguishable from an oversight.
    pub reason: &'static str,
}

/// Every component in the gallery, in the order the page shows them.
///
/// One list, deliberately. The render functions live in [`super::sections`], grouped so section work
/// can be split across commits — but the list itself stays here, because "which components does the
/// gallery contain" should be one file to read rather than six to reconcile. Grouped by kind for
/// reading, not by rule: the completeness check cares only that every component in the library appears
/// here or on [`EXEMPTIONS`].
pub const COMPONENTS: &[Entry] = &[
    // ---- atoms: text, a glyph, a rule, a chip, a badge -------------------------------------
    Entry {
        module: "material/text.rs",
        component: "Text",
        variants: &[
            "Display",
            "Headline",
            "Title",
            "Section",
            "Body",
            "Caption",
            "Action",
            "Label",
            "SidebarName",
            "SidebarTag",
            "SidebarSession",
            // Feature 024: the same size as `SidebarSession` at the other weight. Posed beside it
            // deliberately — the difference is the whole point, and a weight difference is one of
            // the few things you genuinely cannot judge from memory.
            "SidebarSessionCurrent",
            // Feature 1125: a sidebar state group's header, 12 px at weight 500 (R8).
            "SidebarGroup",
            // Feature 1120: the Overview card's 17 px title, scoped to the card (R10).
            "CardTitle",
            // BUG-1298: the card-scoped 13/12 px roles of the Overview Card Redesign (D6).
            "CardStrong",
            "CardType",
            "CardSmall",
            "CardTag",
        ],
        density: &[],
        posed: &["muted"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::text,
    },
    Entry {
        module: "material/ellipsized.rs",
        component: "Ellipsized",
        variants: &[],
        density: &[],
        posed: &["fits", "truncated"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::ellipsized,
    },
    Entry {
        module: "material/glyph.rs",
        component: "Glyph",
        variants: &[],
        density: &[],
        posed: &["tinted", "disabled"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::glyph,
    },
    Entry {
        module: "material/divider.rs",
        component: "Divider",
        variants: &[],
        density: &[],
        posed: &["horizontal", "vertical"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::atoms::divider,
    },
    Entry {
        // Feature 937 (C3, FR-011): the line that explains the required marker, once per form.
        module: "material/required_legend.rs",
        component: "RequiredLegend",
        variants: &[],
        density: &[],
        posed: &["legend"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::required_legend,
    },
    Entry {
        module: "material/tag.rs",
        component: "Tag",
        variants: &[],
        density: &[],
        posed: &["feat accent", "fix accent", "at the label size"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::tag,
    },
    Entry {
        // Feature 1101: a Jira ticket's status, tinted by its category.
        module: "material/status_chip.rs",
        component: "StatusChip",
        variants: &[],
        density: &[],
        posed: &["To Do", "In Progress", "Done", "no category", "stale", "long name"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::status_chip,
    },
    Entry {
        module: "material/icon_label.rs",
        component: "IconLabel",
        variants: &[],
        density: &[],
        posed: &["Label role", "Body role", "untinted"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::icon_label,
    },
    Entry {
        module: "material/activity_badge.rs",
        component: "ActivityBadge",
        variants: &["Working", "Attention", "Ended", "Stopped"],
        density: &[],
        posed: &["Unknown", "Working", "AwaitingInput", "Ended", "Stopped"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::activity_badge,
    },
    Entry {
        module: "material/unread_mark.rs",
        component: "UnreadMark",
        variants: &[],
        density: &[],
        posed: &[
            "an unread session row",
            "a read session row",
            "a collapsed worktree row with an unread count",
            "the same worktree row expanded",
            "a worktree row with none",
            "the switcher's button with an unread count",
            "the switcher's button with none",
            "the Overview button toggled on, with a needs-action badge",
            "the Overview button with a needs-attention badge",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::atoms::unread_mark,
    },
    Entry {
        module: "material/state_icon.rs",
        component: "StateIcon",
        variants: &[],
        density: &[],
        posed: &[
            "Running",
            "Blocked",
            "Idle",
            "Unknown",
            "a worktree row with tags carrying it",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::atoms::state_icon,
    },
    Entry {
        module: "material/pull_request_indicator.rs",
        component: "PullRequestIndicator",
        variants: &[
            "Open", "Draft", "Merged", "Closed", "Pending", "Passing", "Failing",
        ],
        density: &[],
        posed: &[
            "open",
            "draft",
            "merged",
            "closed",
            "open, checks passing",
            "open, checks pending",
            "open, checks failing",
            "draft, checks passing",
            "draft, checks pending",
            "draft, checks failing",
            "open, checks passing, stale",
            "merged, stale",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::pull_request_indicator,
    },
    Entry {
        module: "material/pull_request_row.rs",
        component: "PullRequestRow",
        variants: &[],
        density: &[],
        posed: &[
            "open",
            "draft",
            "merged",
            "closed",
            "selected",
            "stale",
            "a long title",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::pull_request_row,
    },
    Entry {
        module: "material/overview_card.rs",
        component: "OverviewCard",
        // BUG-1298 FR-026: the status words' tones, posed by the "pull request rows" pose.
        variants: &["Bad", "Waiting", "Good", "Plain"],
        density: &[],
        posed: &[
            "five states",
            "Default",
            "orphan",
            "with progress",
            "without progress",
            "targeted",
            "wrapped titles",
        ],
        live: &["expand and collapse"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::overview_card,
    },
    Entry {
        module: "material/progress_bar.rs",
        component: "ProgressBar",
        variants: &[],
        density: &[],
        posed: &[
            "0/10",
            "6/10",
            "10/10",
            "6/10, marker 8/10",
            "9/10, marker 8/10, warning",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::progress_bar,
    },
    Entry {
        module: "material/card_grid.rs",
        component: "CardGrid",
        variants: &[],
        density: &[],
        posed: &["1 card", "3 cards", "30 cards", "narrow", "zoom"],
        live: &["zooming"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::card_grid,
    },
    Entry {
        module: "material/usage_indicator.rs",
        component: "UsageIndicator",
        variants: &[],
        density: &[],
        posed: &["normal", "warning", "long label"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::usage_indicator,
    },
    // ---- controls: buttons, a checkbox, a chip, a field, a dropdown ------------------------
    Entry {
        module: "material/button.rs",
        component: "Button",
        variants: &["Filled", "Outlined", "Text"],
        density: &[],
        posed: &["enabled", "disabled"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::button,
    },
    Entry {
        module: "material/icon_button.rs",
        component: "IconButton",
        variants: &[],
        density: &[],
        posed: &[
            "default",
            "circular",
            "at the title size",
            "tinted",
            "disabled",
        ],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::icon_button,
    },
    Entry {
        module: "material/split_action.rs",
        component: "SplitAction",
        variants: &[],
        density: &[],
        posed: &[
            "with a choice",
            "nothing to choose between",
            "compact, in a dense row",
            "disabled",
        ],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::split_action,
    },
    Entry {
        module: "material/host_picker.rs",
        component: "HostPicker",
        variants: &[],
        density: &[],
        posed: &["no configured hosts"],
        live: &["press the trigger to open the list of configured hosts"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::host_picker,
    },
    Entry {
        module: "material/labelled_toggle.rs",
        component: "LabelledToggle",
        variants: &[],
        density: &[],
        posed: &["default", "a longer word", "tinted", "disabled"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::labelled_toggle,
    },
    Entry {
        module: "material/checkbox.rs",
        component: "Checkbox",
        variants: &[],
        density: &[],
        posed: &["unchecked", "checked", "disabled"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::checkbox,
    },
    Entry {
        module: "material/toggle_chip.rs",
        component: "ToggleChip",
        variants: &[],
        density: &[],
        posed: &["inactive", "active", "accented", "disabled"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::toggle_chip,
    },
    Entry {
        module: "material/form_field.rs",
        component: "FormField",
        // The four state layers (BUG-002, FR-035). Posed rather than driven because the question
        // they answer is about the *differences* — is focus distinguishable from hover, is pressed
        // stronger than both — and hover and focus are transient, so a live instance can only ever
        // show one of them at a time.
        variants: &["None", "Hovered", "Focused", "Pressed"],
        density: &[],
        posed: &[
            "label + supporting",
            "active",
            "error",
            "no label",
            // Feature 937 (C6): the required marker, resting and floated, on a text field and a
            // select.
            "required",
            "required, filled",
            "required select",
            "required select, filled",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::controls::form_field,
    },
    Entry {
        module: "material/text_field.rs",
        component: "TextField",
        variants: &[],
        density: &[],
        posed: &["empty", "filled", "read-only"],
        live: &["hover", "focus", "text entry"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::text_field,
    },
    Entry {
        module: "material/text_link.rs",
        component: "TextLink",
        variants: &[],
        density: &[],
        // Feature 1122 (U1): at the body role, which a card's branch line uses, and at a smaller one.
        posed: &["body", "caption"],
        live: &["hover", "focus", "press"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::text_link,
    },
    Entry {
        module: "material/select.rs",
        component: "Select",
        variants: &[],
        density: &[],
        // Nothing is posed, and that is a change made by feature 022. Two frozen instances used to
        // stand for "unset" and "selected", because a `pick_list`-backed select could not be driven
        // — picking went to `NoOp` and nothing moved. It can be now: the value is the gallery's and
        // the openness is the widget's, so unset *is* the resting state and selected is one press
        // away. A pose of a state a live instance passes through on its own is a second, frozen
        // answer to a question the live one already answers (feature 021's FR-020a, BUG-001).
        posed: &[],
        live: &[
            "press the trigger to open the list; pressing it again closes it",
            "press a row to choose it — the value and the marker both follow",
            "Escape or a press outside closes it, taking nothing",
            "↑ / ↓ move the highlight from the current choice, Enter takes the row it is on",
            "Tab closes the list and still moves focus on",
            "hover and the open state are the same layer at two opacities",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::select,
    },
    Entry {
        module: "material/typeahead.rs",
        component: "Typeahead",
        variants: &[],
        density: &[],
        // Nothing is posed. Every state this component has — which characters are emphasised, where
        // the keyboard is, which row is chosen, what an unavailable row looks like — is a function of
        // what has just been typed, so all of it is exercised rather than staged.
        posed: &[],
        live: &[
            "press the field to open the list; picking, Escape or a press outside closes it",
            "type to narrow the list and see the matched characters picked out",
            "clear the search with the ✕",
            "↑ / ↓ move the highlight, Enter takes the row it is on",
            "press a row to choose it; the dimmed one cannot be chosen",
            "a row with a second line wraps it, and a long title, instead of cutting either",
            "the list opens with the first row chosen and the second highlighted",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::controls::typeahead,
    },
    Entry {
        module: "material/filter_panel.rs",
        component: "FilterTrigger",
        variants: &[],
        density: &[],
        posed: &["inactive", "active"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::filter_trigger,
    },
    Entry {
        module: "material/resize_handle.rs",
        component: "ResizeHandle",
        variants: &[],
        density: &[],
        posed: &[],
        live: &["hover", "the drag itself"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::resize_handle,
    },
    Entry {
        module: "material/split_view.rs",
        component: "SplitView",
        variants: &[],
        density: &[],
        posed: &["two panes", "four panes"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::split_view,
    },
    // ---- surfaces, containers and lists ----------------------------------------------------
    Entry {
        module: "material/surface.rs",
        component: "Surface",
        variants: &[
            "Window",
            "Plain",
            "Dialog",
            "Sidebar",
            "Toolbar",
            "Menu",
            "ListItem",
            "Notification",
            "Chip",
        ],
        density: &[],
        posed: &[],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::surfaces::surface,
    },
    Entry {
        module: "material/scrollable.rs",
        component: "Scrollable",
        variants: &["Vertical", "Horizontal"],
        density: &[],
        posed: &[],
        live: &["hover over the scrollbar", "the scroll itself"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::surfaces::scrollable,
    },
    Entry {
        module: "material/section_list.rs",
        component: "SectionList",
        variants: &[],
        density: &[],
        // `Section` is deliberately absent, for the reason `MenuItem` and `TreeItem` are: it is a
        // record the caller fills in, not a component. It is on the page all the same — a rail is
        // nothing but the sections it was given.
        posed: &["with a badge on the last row"],
        live: &[
            "press a row to make it current",
            "collapse it to its icons, and back",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::surfaces::section_list,
    },
    Entry {
        module: "material/accordion.rs",
        component: "Accordion",
        variants: &[],
        density: &[],
        // It is the panel half only — the trigger is a separate component, paired by the call site.
        posed: &[
            "closed",
            "open",
            "the panel half only: its trigger is a separate component",
        ],
        live: &["press the trigger to reveal it"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::accordion,
    },
    Entry {
        module: "material/accordion_section.rs",
        component: "AccordionSection",
        variants: &[],
        density: &[],
        posed: &[
            "closed, with a summary",
            "open",
            "closed, a required field empty",
            "a group: opening one closes the others",
            "independent sections",
        ],
        live: &[
            "press the header, or focus it and press Space or Enter",
            "press a header in the group, then in the independent sections",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::accordion_section,
    },
    Entry {
        module: "material/disclosure_card.rs",
        component: "DisclosureCard",
        variants: &[],
        density: &[],
        posed: &[
            "folded",
            "open",
            "running",
            "failed, with its label and icon",
            "a thinking card",
            "an Edit card with its line counts",
        ],
        live: &[
            "press the header, or focus it and press Space or Enter",
            "a transcript of a recorded tool turn: press a card's header",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::disclosure_card,
    },
    Entry {
        module: "material/markdown_view.rs",
        component: "MarkdownView",
        variants: &[],
        density: &[],
        posed: &["the markdown subset", "muted, as a thinking card"],
        live: &["Tab to a link or a copy control and press Enter (goes nowhere here)"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::markdown_view,
    },
    Entry {
        module: "material/code_block.rs",
        component: "CodeBlock",
        variants: &[],
        density: &[],
        posed: &[
            "idle, a line wider than the block",
            "copied",
            "copy failed",
            "large",
        ],
        live: &[
            "scroll the long line sideways",
            "press the copy control, or focus it and press Enter (goes nowhere here)",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::code_block,
    },
    Entry {
        module: "material/link_text.rs",
        component: "LinkText",
        variants: &[],
        density: &[],
        posed: &["web, mail and a relative link"],
        live: &["press a link, or Tab to it and press Enter (goes nowhere here)"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::link_text,
    },
    Entry {
        module: "material/field_group.rs",
        component: "FieldGroup",
        variants: &[],
        density: &[],
        posed: &["with a note", "without a note"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::field_group,
    },
    Entry {
        module: "material/toolbar.rs",
        component: "Toolbar",
        variants: &[],
        density: &[],
        posed: &["title only", "with actions"],
        live: &["hover and press its actions"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::toolbar,
    },
    Entry {
        module: "material/connection_banner.rs",
        component: "ConnectionBanner",
        // `Info`/`Error` are `style::NoticeLevel`, which the Material crate declares (feature 1262).
        variants: &["Info", "Error"],
        density: &[],
        posed: &["with an action"],
        live: &["hover and press the action"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::connection_banner,
    },
    Entry {
        module: "material/status_bar.rs",
        component: "StatusBar",
        variants: &[
            "Neutral", "Positive", "Warning", "Error", "Keep", "Drop", "Shorten",
        ],
        density: &[],
        posed: &["steady", "busy", "warning", "too narrow"],
        live: &["hover an item for its tooltip"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::status_bar,
    },
    Entry {
        module: "material/progress.rs",
        component: "StageProgress",
        variants: &[],
        density: &[],
        posed: &["in progress", "with a live line"],
        // Empty, and not an oversight (T085). `live` means "a state you have to *exercise* with the
        // pointer or keyboard", and there is none here — but this component does animate, on its
        // own, continuously, for as long as it is mounted. It is the one entry in the catalogue for
        // which "not interactive" and "not moving" come apart. `sections::surfaces::stage_progress`
        // records why it gets no run control and why the cost is accepted.
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::stage_progress,
    },
    Entry {
        module: "material/tree_view.rs",
        component: "TreeView",
        // `Selection` (feature 1125, R7): "selected row" poses Secondary, "two-line row, selected"
        // poses Primary.
        variants: &["Primary", "Secondary"],
        density: &[],
        posed: &[
            "selected row",
            "Overview row",
            "Overview row, with a badge",
            "Overview row, selected",
            "group header, open",
            "group header, closed",
            "two-line row",
            "two-line row, selected",
            "keyboard-focused worktree row",
            "tags",
            "expandable parent",
            "run group, expanded",
            "run group, collapsed",
            "Run in parallel dialog",
            "Run in parallel dialog, invalid",
            "Quick-switcher dialog",
            "Quick-switcher dialog, nothing matches",
            "Dismiss group dialog",
            "Cleanup offer",
            "Cleanup offer, second confirmation",
        ],
        live: &["hover", "pressed", "right-press"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::tree_view,
    },
    Entry {
        module: "material/navigation_drawer.rs",
        component: "NavigationDrawer",
        variants: &[],
        density: &[],
        posed: &["open", "closed (the rail)", "retracted"],
        live: &["drag its handle", "retracting"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::navigation_drawer,
    },
    // ---- the terminal ----------------------------------------------------------------------
    Entry {
        module: "material/terminal_pane.rs",
        component: "TerminalPane",
        variants: &[],
        density: &[],
        posed: &["unfocused", "focused"],
        live: &["scroll", "select text", "keyboard input"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::terminal::terminal_pane,
    },
    Entry {
        module: "material/edge_fade.rs",
        component: "EdgeFade",
        variants: &[],
        density: &[],
        posed: &[
            "nothing beyond either edge",
            "more that way",
            "both edges",
            "the marked tab is out there",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::terminal::edge_fade,
    },
    Entry {
        module: "material/tab.rs",
        component: "Tab",
        variants: &["Top", "Bottom"],
        density: &[],
        posed: &["marked", "unmarked", "stopped"],
        live: &["hover — a rectangular state layer, never a pill", "press"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::terminal::tab,
    },
    Entry {
        module: "material/tab_strip.rs",
        component: "TabStrip",
        variants: &["Top", "Bottom"],
        density: &[],
        posed: &["indicator on the top edge", "indicator on the bottom edge"],
        live: &["hover along the strip"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::terminal::tab_strip,
    },
    // ---- the review surfaces (feature 482) ---------------------------------------------------
    Entry {
        module: "material/virtual_rows.rs",
        component: "VirtualRows",
        variants: &[],
        density: &[],
        posed: &["2,000 rows", "Compare rows"],
        live: &["scroll — rows are built as they come into view"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::virtual_rows,
    },
    Entry {
        module: "material/diff_view.rs",
        component: "DiffView",
        variants: &[],
        density: &[],
        posed: &[
            "unified",
            "side by side",
            "unified, syntax-coloured",
            "side by side, syntax-coloured",
            "binary message",
            "large-file gate",
            "picked range with a comment",
        ],
        live: &["scroll the unified diff", "Show diff (goes nowhere here)"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::diff_view,
    },
    Entry {
        module: "material/inline_diff.rs",
        component: "InlineDiff",
        variants: &[],
        density: &[],
        posed: &["an Edit", "a MultiEdit", "a Write", "a large diff"],
        live: &["scroll a long line sideways"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::inline_diff,
    },
    Entry {
        module: "material/text_area.rs",
        component: "TextArea",
        variants: &[],
        density: &[],
        posed: &[
            "empty (click it: focused)",
            "multi-line",
            "at its height limit, scrolling inside",
        ],
        // The limited area is a still: the page performs no edits, so its lines do not scroll
        // here; `text_area.rs`'s tests drive the scroll.
        live: &["click the empty area: its indicator thickens"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::text_area,
    },
    Entry {
        module: "material/attachment_chip.rs",
        component: "AttachmentChip",
        variants: &[],
        density: &[],
        posed: &["a picture with its preview", "a long file name"],
        live: &["hover the remove control: its label", "Tab to it: its focus ring"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::composer::attachment_chip,
    },
    Entry {
        module: "material/queue_row.rs",
        component: "QueueRow",
        variants: &[],
        density: &[],
        posed: &[
            "first, with its actions",
            "a long text with pictures",
            "without Move to composer",
        ],
        live: &["hover and Tab through the actions"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::composer::queue_row,
    },
    Entry {
        module: "material/command_palette.rs",
        component: "CommandPalette",
        variants: &[],
        density: &[],
        posed: &[
            "the session's commands, grouped",
            "the fallback list, with its note",
            "nothing matches",
        ],
        live: &["hover a row, and click one"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::composer::command_palette,
    },
    Entry {
        module: "material/busy_indicator.rs",
        component: "BusyIndicator",
        variants: &[],
        density: &[],
        posed: &["busy", "idle"],
        // Empty for `StageProgress`'s reason: busy, the ring turns on its own for as long as it is
        // mounted, and there is nothing to exercise with the pointer or keyboard.
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::composer::busy_indicator,
    },
    Entry {
        module: "material/choice_list.rs",
        component: "ChoiceList",
        variants: &[],
        density: &[],
        posed: &[
            "single choice, one chosen",
            "multiple choice with descriptions",
            "Other chosen, with its field",
        ],
        live: &[
            "hover a row: its state layer",
            "Tab to a row: its focus ring; Space or Enter presses it",
        ],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::prompts::choice_list,
    },
    Entry {
        module: "material/prompt_card.rs",
        component: "PromptCard",
        variants: &[],
        density: &[],
        posed: &["a tool request", "a plan approval", "a question"],
        live: &["hover and Tab through the actions and the choices"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::prompts::prompt_card,
    },
    Entry {
        module: "material/review_bar.rs",
        component: "ReviewBar",
        variants: &[],
        density: &[],
        posed: &[
            "no comments",
            "drafts",
            "drafts, drawer open",
            "sending",
            "error shown",
            "narrow, drafts only",
            "narrow, counts hidden",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::review_bar,
    },
    Entry {
        module: "material/side_sheet.rs",
        component: "SideSheet",
        variants: &[],
        density: &[],
        posed: &["over a diff", "over a narrow area: half of it"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::side_sheet,
    },
    Entry {
        module: "material/review_comment.rs",
        component: "ReviewCommentCard",
        variants: &["Draft", "Sent", "InSend", "Applied", "Answered"],
        density: &[],
        posed: &[
            "outdated",
            "short reply",
            "shortened reply",
            "reply cut at 4 lines",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::review_comment,
    },
    Entry {
        module: "material/review_panel_row.rs",
        component: "ReviewPanelRow",
        variants: &["Draft", "Sent", "Applied", "Answered"],
        density: &[],
        posed: &[
            "with reply",
            "selected",
            "not in the diff",
            "whole reply",
            "long reply, shortened",
            "sent, with actions",
            "highlighted card",
        ],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::review_panel_row,
    },
    Entry {
        module: "material/browse_bar.rs",
        component: "BrowseBar",
        variants: &[],
        density: &[],
        posed: &["Back only", "with Open diff", "long path", "Back disabled"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::review::browse_bar,
    },
    // ---- floating surfaces and their triggers -----------------------------------------------
    // `MenuItem` and `TreeItem` are deliberately absent: they are *records* the caller
    // fills in, not components — no element conversion, public fields — and the builder-API gate
    // already partitions them out of the library's component set. They are visible on the page all the
    // same, inside the menus, the switcher and the tree that consume them.
    Entry {
        module: "material/snackbar.rs",
        component: "Snackbar",
        // `Anchor::BottomCenter` — above a dialog and its scrim, but out of the action row's way.
        variants: &["BottomCenter"],
        density: &[],
        posed: &["info, dismissible", "error, dismissible", "no action"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::floating::snackbar,
    },
    Entry {
        module: "material/modal.rs",
        component: "Modal",
        // `Anchor::Center` — a dialog is centred, which is the anchor this surface poses.
        variants: &["Center"],
        density: &[],
        posed: &["open, centred in the window rather than near the trigger"],
        live: &["Escape and the scrim dismiss it"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::modal,
    },
    Entry {
        module: "material/scrolling_dialog.rs",
        component: "ScrollingDialog",
        variants: &[],
        density: &[],
        posed: &["a body taller than its room, scrolling above pinned actions"],
        live: &["the body scrolls, the actions stay put"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::scrolling_dialog,
    },
    Entry {
        module: "material/menu.rs",
        component: "MenuOverlay",
        // `Anchor::TopEnd` — a trigger-attached popover hangs below the app bar.
        variants: &["TopEnd"],
        density: &[],
        posed: &[
            "open, anchored to the window's top-right — it hangs below the app bar in the \
application, so in this page it opens away from the trigger",
            "the project switcher's list: the same panel, with an active row, a running count, \
unread counts and an unavailable row",
            "an item keyboard-highlighted (the card menu's Up and Down): the focus state layer and \
ring on one entry, the panel the same size",
        ],
        live: &["hover and press its items", "Escape dismisses it"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::menu_overlay,
    },
    Entry {
        module: "material/menu.rs",
        component: "ContextMenu",
        // `Anchor::Point` — a context menu's corner sits at the cursor. `Anchor::BottomStart` —
        // the same panel opened from a control in a bar pinned to the window's bottom edge, which
        // has no room to hang a panel downward.
        variants: &["Point", "BottomStart"],
        density: &[],
        posed: &[
            "open at a point (a fixed window point here; a real one opens at the cursor)",
            "rising from the bottom edge, for a menu opened out of a bottom bar",
        ],
        live: &["hover and press its items", "Escape dismisses it"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::context_menu,
    },
    Entry {
        module: "material/menu.rs",
        component: "MenuTrigger",
        variants: &[],
        density: &[],
        posed: &[],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::menu_trigger,
    },
    Entry {
        module: "material/lib.rs",
        component: "Tooltip",
        // `TooltipPosition` is `cdk::tooltip::Position`, a library enum since 029 BUG-001, so each
        // side is posed as a variant.
        variants: &["Bottom", "Left", "Top", "Right", "FollowCursor"],
        density: &[],
        posed: &[
            "multi-line, wrapped at the ceiling",
            "after 3 s at rest: hold the cursor still",
            "follows the pointer: move over the area",
        ],
        live: &["hover and wait"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::tooltip,
    },
    // ---- components whose appearance IS an animation (rendered in the motion section) -------
    Entry {
        module: "material/ripple.rs",
        component: "Ripple",
        variants: &[],
        density: &[],
        posed: &["at rest"],
        live: &["press anywhere on the surface"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::controls::ripple_component,
    },
    Entry {
        module: "material/animation.rs",
        component: "Fade",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::fade_component,
    },
    Entry {
        module: "material/animation.rs",
        component: "Expand",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::expand_component,
    },
    Entry {
        module: "material/animation.rs",
        component: "Scale",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::scale_component,
    },
    Entry {
        module: "material/animation.rs",
        component: "Scrim",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::scrim_component,
    },
    Entry {
        module: "material/animation.rs",
        component: "ViewFade",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::view_fade,
    },
    Entry {
        module: "material/animation.rs",
        component: "HoverReveal",
        variants: &[],
        density: &[],
        posed: &["replayable"],
        live: &["press Replay, then Reverse"],
        interactive: true,
        section: Section::Motion,
        layout: Layout::FullWidth,
        render: sections::motion::hover_reveal,
    },
];

/// Every animation the library provides, each replayable on demand (FR-007a, FR-007b).
pub const MOTION: &[MotionEntry] = sections::motion::MOTION;

/// Components with no visible appearance of their own (FR-015).
///
/// The behaviour layer's hosts: they decide *where* a floating surface sits and what closes it, and
/// have nothing to look at. Every floating component in the gallery goes through both of them, so
/// they are exercised on the page without being posed on it.
pub const EXEMPTIONS: &[Exemption] = &[
    Exemption {
        module: "material/keyboard_focus.rs",
        component: "TakesTheKeyboard",
        reason: "the focus ring every focusable component wraps itself in: it is public only so the \
                 client's own components (overview card, tree view, code block) can compose it across \
                 the Material crate's boundary (feature 1262), and every ring it draws is posed by \
                 their entries.",
    },
    Exemption {
        module: "material/line_clamp.rs",
        component: "LineClamped",
        reason: "the line-limited label inside the tooltip and the overview card's title: it is \
                 public only so the client's card and review row can compose it across the Material \
                 crate's boundary (feature 1262), and every line it draws is posed by their entries.",
    },
    Exemption {
        module: "material/key_value_list.rs",
        component: "KeyValueList",
        reason: "a `TextArea` with the placeholder `key=value` and a row parser: it draws nothing \
                 of its own, so the gallery's `TextArea` entry already poses every pixel it \
                 has. A second entry would pose the same editor twice under two names.",
    },
    Exemption {
        module: "material/toleration_list.rs",
        component: "TolerationList",
        reason: "a `TextArea` with the placeholder `its row format` and a row parser: it draws nothing \
                 of its own, so the gallery's `TextArea` entry already poses every pixel it \
                 has. A second entry would pose the same editor twice under two names.",
    },
    Exemption {
        module: "material/filled_field.rs",
        component: "FilledField",
        reason: "the filled box `FormField` renders — the container, the in-container label, the \
                 adornment slots and the active indicator, laid out to §7.7's fixed internal \
                 geometry. It has an appearance, but not one of its own: `FormField` is the only \
                 thing that builds it, and the gallery already poses it in four states there. A \
                 second entry would pose the same pixels twice under two names, which is what makes \
                 a catalogue stop being readable.",
    },
    Exemption {
        module: "cdk/overlay.rs",
        component: "Overlay",
        reason: "the overlay host: it positions and stacks floating surfaces and draws nothing of \
                 its own. Every floating entry in the gallery is pushed onto it, so it is exercised \
                 by the page rather than posed on it.",
    },
    Exemption {
        module: "cdk/overlay.rs",
        component: "Surface",
        reason: "a behaviour-layer wrapper: it carries a panel's layer, anchor and dismissal, and \
                 has no appearance. What it wraps — a menu panel, a dialog — is what the floating \
                 section poses.",
    },
    Exemption {
        module: "cdk/control_overlay.rs",
        component: "ControlOverlay",
        reason: "a behaviour-layer wrapper with no appearance: it lays a control over content \
                 that sees every event first, so a press on a link inside a card reaches the link \
                 alone (feature 1122, R6). It draws its two children and nothing else; the \
                 `OverviewCard` entry's issue-link poses show it at work, and its rules are held \
                 by its own unit tests.",
    },
    Exemption {
        module: "cdk/keyed.rs",
        component: "Keyed",
        reason: "a behaviour-layer row or column with no appearance: it lays its children out as \
                 iced's own row or column does and only keeps each child's widget state with its \
                 key when the list changes (feature 1126, R10). The transcript's entries are drawn \
                 through it, and its rules are held by its own unit tests.",
    },
    Exemption {
        module: "cdk/reflow.rs",
        component: "Reflow",
        reason: "a layout-only wrapper with no appearance: it measures a trailing cluster and \
                 either sets it beside a leading block or moves it to the line below (003 \
                 BUG-002). It draws its two children and nothing else; posing it would show \
                 whatever it was handed, and its breakpoint is held by its own unit tests and \
                 `tests/known_projects_reflow.rs`.",
    },
    Exemption {
        module: "material/terminal_pane.rs",
        component: "GridSizeReporter",
        reason: "a measuring wrapper with no appearance: it lays out to its child, draws the child \
                 and nothing else, and exists only to report the terminal area's size in cells so a \
                 session can be *started* at it (BUG-003, FR-014b). What the area looks like is \
                 `TerminalPane`, which the gallery already poses; posing an invisible wrapper would \
                 show its child a second time under a name that draws nothing.",
    },
    Exemption {
        module: "cdk/picker.rs",
        component: "Picker",
        reason: "a behaviour-layer wrapper, for the same reason as the two above: it anchors a \
                 list to a field's own bounds, applies the keyboard rule, decides when the list \
                 closes and how long it stays while leaving, and names no colour, size or spacing. \
                 Its field and its list arrive already drawn — what they look like is \
                 `material/picker.rs`, which the controls section poses through both of the \
                 controls built on it.",
    },
    Exemption {
        module: "cdk/context_area.rs",
        component: "ContextArea",
        reason: "a behaviour-layer wrapper, for the same reason as the three above: it delegates \
                 layout, draw, operate and overlay to its single child and intercepts one event — a \
                 secondary press over its own bounds — publishing the point so a caller can open a \
                 menu there (`012` BUG-005, FR-010b). It names no colour, size or spacing and adds \
                 no pixel to what it wraps. What a secondary press *produces* is the `ContextMenu` \
                 the floating section poses, on both of its anchors.",
    },
    Exemption {
        module: "cdk/tooltip.rs",
        component: "Tooltip",
        reason: "a behaviour-layer wrapper, for the same reason as `Picker`: it decides which side \
                 of its trigger a hover label floats on — the other side when the asked-for one has \
                 no room (029 BUG-001) — and names no colour, size or spacing. Its panel arrives \
                 already drawn; what it looks like is `material/lib.rs`'s `Tooltip`, which the \
                 gallery poses on each of its four sides.",
    },
    Exemption {
        module: "cdk/keyboard_elsewhere.rs",
        component: "KeyboardElsewhere",
        reason: "a behaviour-layer wrapper with no appearance: it delegates layout, draw, operate \
                 and overlay to its single child and, while told the terminal holds the keyboard, \
                 clears every widget focus before a key reaches the child (018 BUG-016). What it \
                 changes is a focus ring that is *not* drawn, which the gallery cannot pose; its \
                 rule is held by `material/field_focus.rs`'s tests.",
    },
    Exemption {
        module: "cdk/focus_within.rs",
        component: "FocusWithin",
        reason: "a behaviour-layer wrapper with no appearance: it delegates layout, draw, operate \
                 and overlay to its single child and reports which of the child's focusables holds \
                 the keyboard (feature 941, T061 F5). What it changes is which control keeps its \
                 focus ring after Tab, which the gallery cannot pose; its rule is held by its own \
                 unit test and `tests/composer_tab.rs`.",
    },
];
