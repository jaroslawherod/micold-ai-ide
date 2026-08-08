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
//! [contracts/gallery-catalogue.md]: https://github.com/Cumulocity-IoT/micold-ai-ide/blob/main/specs/020-component-showcase-gallery/contracts/gallery-catalogue.md

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
        variants: &["Working", "Attention", "Ended"],
        density: &[],
        posed: &["Unknown", "Working", "AwaitingInput", "Ended"],
        live: &[],
        interactive: false,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::atoms::activity_badge,
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
        posed: &["inactive", "active", "accented"],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::controls::toggle_chip,
    },
    Entry {
        module: "material/form_field.rs",
        component: "FormField",
        variants: &[],
        density: &[],
        posed: &["label + supporting", "active", "error", "no label"],
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
        variants: &[],
        density: &[],
        posed: &[],
        live: &["hover over the scrollbar", "the scroll itself"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::surfaces::scrollable,
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
        // Not `variants`: `Info`/`Error` are `app::NoticeLevel`, which the library does not declare.
        // The completeness check is right to refuse them, and they are posed states all the same.
        variants: &[],
        density: &[],
        posed: &["Info", "Error", "with an action"],
        live: &["hover and press the action"],
        interactive: true,
        section: Section::Components,
        layout: Layout::FullWidth,
        render: sections::surfaces::connection_banner,
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
        variants: &[],
        density: &[],
        posed: &["selected row", "tags", "expandable parent"],
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
        posed: &["open", "closed (the rail)"],
        live: &["drag its handle"],
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
    // ---- floating surfaces and their triggers -----------------------------------------------
    // `MenuItem`, `ProjectRow` and `TreeItem` are deliberately absent: they are *records* the caller
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
        module: "material/menu.rs",
        component: "MenuOverlay",
        // `Anchor::TopEnd` — a trigger-attached popover hangs below the app bar.
        variants: &["TopEnd"],
        density: &[],
        posed: &[
            "open, anchored to the window's top-right — it hangs below the app bar in the \
application, so in this page it opens away from the trigger",
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
        // `Anchor::Point` — a context menu's corner sits at the cursor.
        variants: &["Point"],
        density: &[],
        posed: &["open at a fixed window point (a real one opens at the cursor)"],
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
        module: "material/project_switcher.rs",
        component: "ProjectSwitcherOverlay",
        variants: &[],
        density: &[],
        posed: &[
            "open, anchored to the window's top-right",
            "active row",
            "running count",
            "unavailable row",
        ],
        live: &["hover and press its rows", "Escape dismisses it"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::project_switcher_overlay,
    },
    Entry {
        module: "material/project_switcher.rs",
        component: "ProjectSwitcherTrigger",
        variants: &[],
        density: &[],
        posed: &[],
        live: &["hover", "pressed", "focus"],
        interactive: true,
        section: Section::Components,
        layout: Layout::Inline,
        render: sections::floating::project_switcher_trigger,
    },
    Entry {
        module: "material/mod.rs",
        component: "Tooltip",
        // Not `variants`: `Bottom`/`Left` are the rendering stack's `tooltip::Position`, not a library
        // enum. Posed states instead.
        variants: &[],
        density: &[],
        posed: &["below (the default)", "to the left"],
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
        module: "cdk/picker.rs",
        component: "Picker",
        reason: "a behaviour-layer wrapper, for the same reason as the two above: it anchors a \
                 list to a field's own bounds, applies the keyboard rule, decides when the list \
                 closes and how long it stays while leaving, and names no colour, size or spacing. \
                 Its field and its list arrive already drawn — what they look like is \
                 `material/picker.rs`, which the controls section poses through both of the \
                 controls built on it.",
    },
];
