//! Component anatomy — the measurements that make a component the component it claims to be
//! (feature 018, T042 — FR-025 – FR-032; contract §7).
//!
//! A component "matching Material" turns out to be mostly a list of numbers. Individually each is
//! unremarkable: an app bar is 64dp, a dialog pads 24dp, a chip's ends are 12dp. Collectively they
//! are what separates a toolbar from a Material top app bar, and every one of them was a different
//! number before this feature.
//!
//! # Heights are not here
//!
//! Anything with a height asks [`density`](super::density), which owns the base heights and the one
//! piece of arithmetic that applies a density step. Restating a row height here would create two
//! constants for one measurement — exactly how the sidebar came to be 28dp while the contract said
//! 36dp — so this module deliberately has no `HEIGHT` for anything density already answers for.
//!
//! The exceptions are the components density has no step for: an app bar, a chip and a snackbar are
//! fixed by the specification rather than scaled by the density axis, so their heights live here.
//!
//! # Everything is dp
//!
//! Every value is a length in density-independent pixels, `f32` to match the rendering stack's
//! geometry. `tests/tokens_anatomy.rs` checks each against §7 and asserts the set as a whole is
//! positive and finite.

/// §7.1 — the small top app bar. The medium and large variants are not adopted.
pub mod app_bar {
    /// Total bar height.
    pub const HEIGHT: f32 = 64.0;
    /// Horizontal padding either side.
    pub const PADDING: f32 = 16.0;
    /// Padding before a *leading icon button*, which is smaller than [`PADDING`] because the
    /// button's own padding supplies the rest of the gap — 4 + 12 reads as 16 to the eye.
    pub const LEADING_ICON_PADDING: f32 = 4.0;
    /// The interactive target of each action, honoured regardless of the glyph's size.
    pub const ICON_TARGET: f32 = 48.0;
}

/// §7.2 — list and tree rows, at the two named densities.
///
/// The heights come from [`density`](super::density): `LIST_ROW_BASE` at `STANDARD` and `DENSE`.
/// What varies besides height is the horizontal padding and the gap after a leading icon, and those
/// are per-density rather than per-list — a list that picked its own would be the third density
/// FR-026b forbids.
pub mod list_row {
    /// Horizontal padding on a standard (48dp) row.
    pub const STANDARD_PADDING: f32 = 16.0;
    /// Horizontal padding on a dense (36dp) row — the worktree sidebar.
    pub const DENSE_PADDING: f32 = 8.0;
    /// Gap between a leading icon and the row's text, standard density.
    pub const STANDARD_ICON_GAP: f32 = 16.0;
    /// Gap between a leading icon and the row's text, dense density.
    pub const DENSE_ICON_GAP: f32 = 8.0;
}

/// §7.3 — buttons. The height is `density::BUTTON_BASE`; these are the rest.
pub mod button {
    /// The minimum interactive target, honoured **even where the visible container is smaller**.
    ///
    /// Every button in this application has a container smaller than this — 40dp, and an icon
    /// button's glyph is 24dp inside it — so this is never satisfied by accident. A target that
    /// merely matched the container would look identical and miss every one of them.
    pub const MIN_TOUCH_TARGET: f32 = 48.0;
    /// Horizontal padding, filled variant.
    pub const PADDING_FILLED: f32 = 24.0;
    /// Horizontal padding, outlined variant.
    pub const PADDING_OUTLINED: f32 = 24.0;
    /// Horizontal padding, text variant — tighter, because a text button has no container to
    /// balance against.
    pub const PADDING_TEXT: f32 = 12.0;
    /// Horizontal padding, icon-only variant.
    pub const PADDING_ICON: f32 = 8.0;
    /// A leading icon inside a labelled button.
    pub const LEADING_ICON: f32 = 18.0;
    /// The glyph of an icon-only button — larger than a leading icon, because it is the whole
    /// content rather than an accent to a label.
    pub const ICON_BUTTON_GLYPH: f32 = 24.0;
    /// The outline width of the outlined variant.
    pub const OUTLINE: f32 = 1.0;
}

/// §7.4 — dialogs.
pub mod dialog {
    /// Padding on all four sides.
    pub const PADDING: f32 = 24.0;
    /// Gap between the title and the body.
    pub const TITLE_TO_BODY: f32 = 16.0;
    /// Gap between the body and the action row — wider than [`TITLE_TO_BODY`], so the actions read
    /// as a separate region rather than as more body.
    pub const BODY_TO_ACTIONS: f32 = 24.0;
    /// Gap between adjacent actions.
    pub const ACTION_GAP: f32 = 8.0;
    /// The optional centred icon above the title.
    pub const ICON: f32 = 24.0;
    /// Narrowest a dialog may be.
    pub const MIN_WIDTH: f32 = 280.0;
    /// Widest a dialog may be, so a long message does not stretch to the window.
    pub const MAX_WIDTH: f32 = 560.0;
}

/// §7.5 — menus, context menus and popovers. Item height is `density::MENU_ITEM_BASE`.
pub mod menu {
    /// Padding above the first item and below the last.
    pub const VERTICAL_PADDING: f32 = 8.0;
    /// Horizontal padding inside an item.
    pub const ITEM_PADDING: f32 = 12.0;
    /// An item's leading icon.
    pub const ITEM_ICON: f32 = 24.0;
    /// A divider between item groups.
    pub const DIVIDER: f32 = 1.0;
}

/// §7.6 — chips and tags.
///
/// Fixed height: a chip is not on the density axis, because it is sized by its label rather than by
/// how much of a list fits on screen.
pub mod chip {
    /// Chip height.
    pub const HEIGHT: f32 = 32.0;
    /// Horizontal padding.
    pub const PADDING: f32 = 12.0;
    /// Horizontal padding when a leading icon is present — the icon takes some of the padding's
    /// job of keeping the label off the chip's edge.
    pub const PADDING_WITH_ICON: f32 = 8.0;
    /// A leading icon inside a chip.
    pub const ICON: f32 = 18.0;
    /// The outline width of an unselected chip.
    pub const OUTLINE: f32 = 1.0;
}

/// §7.7 — the filled text field and the select control. Height is `density::TEXT_FIELD_BASE`.
pub mod text_field {
    /// Horizontal padding inside the container.
    pub const PADDING: f32 = 16.0;
    /// The bottom active indicator at rest.
    pub const INDICATOR: f32 = 1.0;
    /// The bottom active indicator when the field is focused — or, for the select, when it is open,
    /// since a `pick_list` cannot report focus (FR-043a).
    ///
    /// Thicker than [`INDICATOR`] on purpose: a filled field has no border to recolour, so this
    /// difference *is* the focus affordance. Equal values would leave focus invisible.
    pub const INDICATOR_ACTIVE: f32 = 2.0;
    /// The select's trailing chevron.
    pub const TRAILING_ICON: f32 = 24.0;
}

/// §7.8 — the snackbar. Fixed height: it is sized by its message, not by the density axis.
pub mod snackbar {
    /// The shortest a snackbar can be; a longer message grows it.
    pub const MIN_HEIGHT: f32 = 48.0;
    /// Horizontal padding.
    pub const PADDING_H: f32 = 16.0;
    /// Vertical padding — deliberately not a spacing step. 14dp is what puts a `body_medium` line
    /// in a 48dp container, and rounding it to 12 or 16 changes the height it is derived from.
    pub const PADDING_V: f32 = 14.0;
    /// Widest a snackbar may be, so a long message wraps rather than spanning the window.
    pub const MAX_WIDTH: f32 = 600.0;
}

/// §7.9 — the linear progress indicator.
pub mod progress {
    /// Bar thickness, track and indicator alike.
    pub const THICKNESS: f32 = 4.0;
    /// Gap between the bar and its stage label.
    pub const LABEL_GAP: f32 = 4.0;
}

/// Every anatomy value, for the tests that hold properties across the whole set rather than
/// checking one number at a time.
///
/// Listed rather than derived because Rust has no reflection over modules — so the one risk is this
/// falling behind, which `tests/tokens_anatomy.rs` guards with a count.
pub const ALL: [(&str, f32); 42] = [
    ("app_bar::HEIGHT", app_bar::HEIGHT),
    ("app_bar::PADDING", app_bar::PADDING),
    (
        "app_bar::LEADING_ICON_PADDING",
        app_bar::LEADING_ICON_PADDING,
    ),
    ("app_bar::ICON_TARGET", app_bar::ICON_TARGET),
    ("list_row::STANDARD_PADDING", list_row::STANDARD_PADDING),
    ("list_row::DENSE_PADDING", list_row::DENSE_PADDING),
    ("list_row::STANDARD_ICON_GAP", list_row::STANDARD_ICON_GAP),
    ("list_row::DENSE_ICON_GAP", list_row::DENSE_ICON_GAP),
    ("button::MIN_TOUCH_TARGET", button::MIN_TOUCH_TARGET),
    ("button::PADDING_FILLED", button::PADDING_FILLED),
    ("button::PADDING_OUTLINED", button::PADDING_OUTLINED),
    ("button::PADDING_TEXT", button::PADDING_TEXT),
    ("button::PADDING_ICON", button::PADDING_ICON),
    ("button::LEADING_ICON", button::LEADING_ICON),
    ("button::ICON_BUTTON_GLYPH", button::ICON_BUTTON_GLYPH),
    ("button::OUTLINE", button::OUTLINE),
    ("dialog::PADDING", dialog::PADDING),
    ("dialog::TITLE_TO_BODY", dialog::TITLE_TO_BODY),
    ("dialog::BODY_TO_ACTIONS", dialog::BODY_TO_ACTIONS),
    ("dialog::ACTION_GAP", dialog::ACTION_GAP),
    ("dialog::ICON", dialog::ICON),
    ("dialog::MIN_WIDTH", dialog::MIN_WIDTH),
    ("dialog::MAX_WIDTH", dialog::MAX_WIDTH),
    ("menu::VERTICAL_PADDING", menu::VERTICAL_PADDING),
    ("menu::ITEM_PADDING", menu::ITEM_PADDING),
    ("menu::ITEM_ICON", menu::ITEM_ICON),
    ("menu::DIVIDER", menu::DIVIDER),
    ("chip::HEIGHT", chip::HEIGHT),
    ("chip::PADDING", chip::PADDING),
    ("chip::PADDING_WITH_ICON", chip::PADDING_WITH_ICON),
    ("chip::ICON", chip::ICON),
    ("chip::OUTLINE", chip::OUTLINE),
    ("text_field::PADDING", text_field::PADDING),
    ("text_field::INDICATOR", text_field::INDICATOR),
    ("text_field::INDICATOR_ACTIVE", text_field::INDICATOR_ACTIVE),
    ("text_field::TRAILING_ICON", text_field::TRAILING_ICON),
    ("snackbar::MIN_HEIGHT", snackbar::MIN_HEIGHT),
    ("snackbar::PADDING_H", snackbar::PADDING_H),
    ("snackbar::PADDING_V", snackbar::PADDING_V),
    ("snackbar::MAX_WIDTH", snackbar::MAX_WIDTH),
    ("progress::THICKNESS", progress::THICKNESS),
    ("progress::LABEL_GAP", progress::LABEL_GAP),
];
