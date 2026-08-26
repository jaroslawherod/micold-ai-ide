//! `SplitAction` — one action with a second way to reach it (Principle VIII, feature 026 T033).
//!
//! A primary icon button that performs the default, and an adjacent chevron that opens a list of
//! alternatives. Material calls this a split button; here it is the row action that starts a
//! session: press it and you get the default AI CLI in one interaction, press the chevron and you
//! choose which one.
//!
//! # It is a component rather than two buttons in a row
//!
//! The sidebar could have written `row![action_icon(...), action_icon(...)]` and been done. Three
//! things make that the wrong shape, and all three are properties of the *pair* rather than of
//! either half:
//!
//! 1. **The chevron is absent, not disabled, when there is nothing to choose between.** A control
//!    that opens a list of one is a worse single-CLI experience than the plain button it replaced
//!    (FR-006, SC-001), and "absent" is a decision the pair has to make once.
//! 2. **The two halves are one target group.** They share a tint and a size, and the gap between
//!    them is smaller than the gap to the next action — that is what makes them read as one
//!    control rather than two neighbours.
//! 3. **The height is the primary's.** The sidebar's scroll arithmetic hardcodes "a session row is
//!    always one line" (`features/sidebar.rs::row_heights`), and its own doc calls that the one
//!    place in the sidebar where a wrong answer is silent — the computed scroll target drifts from
//!    what is drawn and nothing complains. Both halves are `IconButton`s at the same role, so this
//!    composition cannot grow the row. `anatomy_size.rs` holds it.
//!
//! # It decides nothing
//!
//! Which half was pressed is all this reports. Whether that means "start the default", "offer the
//! list" or "nothing is installed" is `State::start_intent`'s answer (feature 026, T032a) —
//! Principle I's GUI exception covers drawing and does not cover branching.

use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::{IconButton, Tooltip, TypeRole};
use iced::widget::row;
use iced::{Alignment, Element};
use micold_core::tokens::{Rgb, Roles};

/// Reports where a chevron press landed, in window pixels — see [`SplitAction::on_secondary_anchor`].
///
/// Named rather than written inline for the reason `cdk::context_area` names its own: the borrow, the
/// box and the tuple argument together are past what `clippy::type_complexity` will read in a struct
/// field, and the alias says what the closure is *for* where the signature only says what it is.
type OnAnchor<'a, M> = Box<dyn Fn((u16, u16)) -> M + 'a>;

/// A primary action with an optional adjacent "…or choose" control.
///
/// Builder form: `SplitAction::new(icon, roles).on_press(m).on_secondary_press_maybe(m).into()`.
pub struct SplitAction<'a, M> {
    icon: Icon,
    roles: Roles,
    primary: Option<M>,
    primary_anchor: Option<OnAnchor<'a, M>>,
    secondary: Option<M>,
    secondary_anchor: Option<OnAnchor<'a, M>>,
    tooltip: Option<&'static str>,
    secondary_tooltip: Option<&'static str>,
    tint: Option<Rgb>,
    compact: bool,
    size: Option<TypeRole>,
}

impl<'a, M> SplitAction<'a, M> {
    /// A split action showing `icon`, themed by `roles`. Both halves start unpressable: a caller
    /// that supplies neither message gets an inert control rather than one that looks live.
    pub fn new(icon: Icon, roles: Roles) -> Self {
        Self {
            icon,
            roles,
            primary: None,
            primary_anchor: None,
            secondary: None,
            secondary_anchor: None,
            tooltip: None,
            secondary_tooltip: None,
            tint: None,
            compact: false,
            size: None,
        }
    }

    /// What the primary half emits.
    pub fn on_press(self, message: M) -> Self {
        self.on_press_maybe(Some(message))
    }

    /// What the primary half emits, or `None` to leave it unpressable.
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.primary = message;
        self
    }

    /// What the secondary half emits — and whether it exists at all.
    ///
    /// `None` removes the chevron entirely rather than disabling it. That is the whole reason this
    /// takes an `Option` instead of a message: a disabled chevron still occupies the row and still
    /// reads as an affordance that is temporarily unavailable, which is the wrong story when the
    /// user has one CLI installed and there is nothing to choose (FR-006).
    pub fn on_secondary_press_maybe(mut self, message: Option<M>) -> Self {
        self.secondary = message;
        self
    }

    /// Also report *where* a **primary** press landed, as `f(point)` in window pixels.
    ///
    /// The primary half does not always start something. When the stored default is not installed
    /// it offers the list instead (feature 026, FR-004 scenario 4), and a list has to hang from the
    /// control that opened it wherever it was opened from — so the half that can open one has to be
    /// able to say where it is, exactly as [`Self::on_secondary_anchor`] does for the chevron.
    ///
    /// Reported on every primary press, not only the ones that open something: what a press means
    /// is the state's to decide, and this control does not know which answer it just published. A
    /// point that arrives with no list open is a no-op at the reducer, which is the same contract
    /// the chevron's anchor already relies on for the press that *closes* the list.
    pub fn on_primary_anchor(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.primary_anchor = Some(Box::new(f));
        self
    }

    /// Also report *where* a chevron press landed, as `f(point)` in window pixels.
    ///
    /// The chevron opens a floating list, and a list has to hang from the control that opened it
    /// rather than from a figure written into the view — that was 018's BUG-008, and
    /// `tests/context_menu_anchor_call_sites.rs` is what keeps it fixed. The press point is the
    /// only thing that knows where a sidebar row is: the view lays the rows out and does not hold
    /// their positions, and a row scrolls.
    ///
    /// Two messages leave one press, in order: the chevron's own says the list was asked for, this
    /// says where. They are separate because the chevron is an ordinary enabled button and stays
    /// one — a wrapper that swallowed the press to build a single message would leave iced drawing
    /// it as disabled, without its hover and press state layers.
    pub fn on_secondary_anchor(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.secondary_anchor = Some(Box::new(f));
        self
    }

    /// Hover text for the primary half.
    ///
    /// Named `primary_tooltip` rather than the bare word, for the reason `TreeItem::row_tooltip`
    /// is: the rendering stack has a widget of that name which carries its own overlay, and two
    /// guards — `material_boundary.rs` and `one_overlay_implementation.rs` — read an unprefixed
    /// call to it in a source file as a boundary crossing. They scan text, so a builder step that
    /// shadows a guarded widget name makes every call site look like one. The prefix is what keeps
    /// those guards able to tell the difference.
    pub fn primary_tooltip(mut self, text: &'static str) -> Self {
        self.tooltip = Some(text);
        self
    }

    /// Hover text for the chevron.
    pub fn secondary_tooltip(mut self, text: &'static str) -> Self {
        self.secondary_tooltip = Some(text);
        self
    }

    /// Tint both halves. They are one control, so they take one tint.
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Size both halves for a dense row — see [`IconButton::compact`].
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// The type role both halves are sized at.
    pub fn size(mut self, role: TypeRole) -> Self {
        self.size = Some(role);
        self
    }
}

/// The gap *within* the control. Deliberately tighter than the `spacing::XS` the sidebar puts
/// between separate actions: the two halves have to read as one control, and equal gaps would make
/// the chevron look like the next icon along.
const WITHIN: f32 = 0.0;

impl<'a, M: Clone + 'a> From<SplitAction<'a, M>> for Element<'a, M> {
    fn from(split: SplitAction<'a, M>) -> Self {
        let half = |glyph: Icon, message: Option<M>, tooltip: Option<&'static str>| {
            let pressable = message.is_some();
            let mut button = IconButton::new(glyph, split.roles).on_press_maybe(message);
            if split.compact {
                button = button.compact();
            }
            if let Some(role) = split.size {
                button = button.size(role);
            }
            if let Some(tint) = split.tint {
                button = button.tint(tint);
            }
            match tooltip.filter(|_| pressable) {
                Some(text) => Tooltip::new(button, text, split.roles).into(),
                None => Element::from(button),
            }
        };

        let primary = half(split.icon, split.primary.clone(), split.tooltip);
        let primary = match split.primary_anchor {
            Some(anchor) => ContextArea::new(primary).on_primary_press(anchor).into(),
            None => primary,
        };
        match split.secondary.clone() {
            None => primary,
            Some(message) => {
                let chevron = half(Icon::SelectChevron, Some(message), split.secondary_tooltip);
                let chevron = match split.secondary_anchor {
                    Some(anchor) => ContextArea::new(chevron).on_primary_press(anchor).into(),
                    None => chevron,
                };
                row![primary, chevron]
                    .spacing(WITHIN)
                    .align_y(Alignment::Center)
                    .into()
            }
        }
    }
}
