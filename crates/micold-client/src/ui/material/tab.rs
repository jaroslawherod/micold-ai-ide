//! `Tab` — one member of a tab strip (feature 026 FR-013, Principle VIII).
//!
//! A tab was assembled inline in `ui/terminal.rs` until this feature: a `Button` wrapping a column
//! of an indicator rule over a row of three slots, built at the call site. That was tolerable while
//! one call site built it for one kind of member. It stopped being tolerable for two reasons at
//! once — one tab shape now serves two kinds of member (an open terminal instance, and the
//! session's AI CLI process), and the gallery discovers a component as a `pub struct` under
//! `src/ui/material/` or `src/ui/cdk/`, so a call-site assembly **cannot be posed at all**. FR-014
//! asks for it posed in both indicator orientations, which makes the promotion the route rather
//! than the tidying.
//!
//! # What a tab is
//!
//! Three things, and the middle one is the only one a caller always supplies:
//!
//! - an **indicator edge** — a 3dp accent rule on the marked tab, and a transparent rule of the
//!   same thickness on every other one, so activation moves colour and never geometry;
//! - a **label**, content-sized under a ceiling so a renamed instance ellipsises inside its tab
//!   rather than resizing the strip;
//! - two **reserved slots** flanking it, each the width of a §7.3 touch target. Reserved, not
//!   conditional: feature 023 FR-008a, at length in `ui/terminal.rs`, is that a child which comes
//!   and goes shifts every sibling after it, and iced's positional `Tree::diff_children` then hands
//!   a pressed control its neighbour's node and drops the press. A tab is a pressable control whose
//!   press is the whole feature, so nothing inside it may be pushed-or-not.
//!
//! Their sum is [`WIDTH`], and every tab measures it. That is what makes the indicator work: a rule
//! spans the width it is given, and `Length::Fill` inside a content-sized tab resolves against the
//! *button's* available space rather than the label's (feature 012 BUG-002).
//!
//! # The highlight is a tab's, not a button's
//!
//! A tab draws a shape in exactly one state — highlighted — and that shape is
//! [`state_layer_shape`]: rectangular, spanning the tab (FR-015, SC-010). Built as a
//! `ButtonVariant::Text` it inherited the fully rounded pill a text button draws, from two places
//! at once: `style::text_button`'s own `shape::FULL` border radius under the hover and press fill,
//! and the `shape::FULL` the ripple is clipped to. Both are overridden here through one
//! [`Button::shape`] step, so they cannot drift apart.
//!
//! Unhighlighted, a tab still draws nothing at all — no background, no outline, no pill (feature
//! 012 FR-004b).

use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::button::{Button, Variant as ButtonVariant};
use crate::ui::material::divider::Divider;
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{anatomy, shape, spacing, Rgb, Roles};

/// Which edge of the tab its active indicator is drawn on.
///
/// This application uses [`Self::Top`], which is the opposite of Material's default. Its tab strip
/// is anchored to the window's **bottom**, so the pane a tab selects is *above* it and a bottom
/// indicator would point away from what it marks (feature 012 FR-004b). A deliberate inversion that
/// is never shown next to the thing it inverts reads as a mistake to the next person, which is why
/// FR-014 asks the gallery to pose both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorEdge {
    /// The accent rule sits above the tab's content — this application's own.
    Top,
    /// The accent rule sits below it — Material's default.
    Bottom,
}

/// The trailing slot's layout footprint, and the leading slot's, which balances it.
///
/// §7.3's minimum touch target, **not** a glyph's visible size: a pressable, non-compact
/// `IconButton` wraps itself in a `MIN_TOUCH_TARGET` box so a small pill still gets a large target.
/// Measuring the visible pill instead — 24, in the first cut of feature 012's BUG-002 fix — leaves
/// the leading spacer narrower than the control it balances, and the label lands `(48 - 24) / 2 =
/// 12`dp left of centre. That is what that feature's visual pass caught, and it is why this reads
/// the anatomy constant rather than naming a number: the two must move together.
pub const SLOT_WIDTH: f32 = anatomy::button::MIN_TOUCH_TARGET;

/// The label's share of the derived [`WIDTH`] — what a tab reserves for its own name before the two
/// slots and the gaps are counted (feature 012 BUG-005, FR-004c).
///
/// A floor, where [`LABEL_MAX_WIDTH`] is the ceiling at which a label ellipsises. It is not measured
/// text: a shaped width is not available in a `const`, and reserving one would make a tab's width
/// depend on its content, which is the thing FR-004c forbids. Sized instead by what has to remain
/// legible — comfortably more than the two digits an ordinal needs, and enough of a name to tell two
/// tabs apart once instances can be renamed.
pub const LABEL_MIN_WIDTH: f32 = 16.0;

/// The widest a tab's label may grow before it ellipsises (feature 012 BUG-002).
///
/// A *maximum*, not a fixed two-digit box. That box was sized for an ordinal, and an instance is to
/// become renameable — a tab will show a name, and a width chosen for `99` would have to be undone
/// that day. Content-sized under a ceiling serves both, and costs nothing now.
pub const LABEL_MAX_WIDTH: f32 = 120.0;

/// Every tab's width — uniform and fixed, which is what makes the indicator work at all.
///
/// **Derived, not chosen** (feature 012 BUG-005, FR-004c). It was written as a literal `128.0` — a
/// number that made three tab states look right and that no test could disagree with. There is a
/// fourth state, and 128 was not enough for it: a tab whose instance had stopped carried a restart
/// button too, and the row settled the 54.3dp shortfall by shrinking its trailing children until the
/// button was 0.0 wide and the close control was 45.2, under §7.3's target. Nothing overflowed, so
/// nothing failed.
///
/// Computing it from the things it has to hold means it moves when any of them does, and a further
/// child cannot be added without this sum being confronted.
pub const WIDTH: f32 = 2.0 * spacing::SM   // the tab's own padding, both edges
    + SLOT_WIDTH                            // the leading slot
    + spacing::XS
    + LABEL_MIN_WIDTH
    + spacing::XS
    + SLOT_WIDTH; // the trailing slot

/// The corner radius of a tab's hover and press state layer — `shape::NONE`, a rectangle spanning
/// the whole tab (FR-015, SC-010).
///
/// Its own function for the same reason [`indicator_colour`] is one: the rule is then a pure value
/// test rather than a claim about a `view()` no unit test can reach. What a test here cannot say is
/// that a rectangle of the right size is actually *drawn* — a state layer is drawn, not laid out, so
/// no geometry gate can see it and the visual pass is what judges it. What it does fix in place is
/// that nobody reintroduces the pill by reaching for the button's default again.
pub fn state_layer_shape() -> f32 {
    shape::NONE
}

/// The accent an active tab's indicator is drawn in — `None` for an inactive tab (feature 012
/// BUG-002, FR-004b).
///
/// A tab strip marks its selected member with an **indicator**, not with a container. Feature 012's
/// BUG-001 had this choosing `Filled` for the active tab and `Outlined` for the rest, and that was
/// the wrong idiom: it read the original defect (one filled pill among loose numbers) as "the
/// entries need containers", when the half nobody had written down is that a tab strip underlines
/// its active tab. No tab draws a container now.
pub fn indicator_colour(is_active: bool, r: Roles) -> Option<Rgb> {
    is_active.then_some(r.primary)
}

/// One member of a tab strip. Builder form (Principle VIII):
/// `Tab::new(label, roles).active(true).trailing(close).on_press(msg).into()`.
pub struct Tab<'a, M> {
    label: Element<'a, M>,
    roles: Roles,
    leading: Option<Element<'a, M>>,
    trailing: Option<Element<'a, M>>,
    pub(crate) edge: IndicatorEdge,
    active: bool,
    on_press: Option<M>,
    on_secondary_press: Option<Box<dyn Fn((u16, u16)) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Tab<'a, M> {
    /// A tab showing `label`, themed by `roles`. Inactive, top-edged, both slots reserved and
    /// empty, and not pressable until [`Self::on_press`] says so.
    pub fn new(label: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            roles,
            leading: None,
            trailing: None,
            edge: IndicatorEdge::Top,
            active: false,
            on_press: None,
            on_secondary_press: None,
        }
    }

    /// Fill the leading slot — feature 026's stopped mark. The slot exists either way; this is what
    /// goes in it.
    pub fn leading(mut self, content: impl Into<Element<'a, M>>) -> Self {
        self.leading = Some(content.into());
        self
    }

    /// Fill the trailing slot — a terminal tab's close control. Left empty on a tab that offers no
    /// close (FR-004), which is why the slot is reserved rather than reclaimed: a strip whose tabs
    /// are not all one size reads as a control among controls rather than as a strip.
    pub fn trailing(mut self, content: impl Into<Element<'a, M>>) -> Self {
        self.trailing = Some(content.into());
        self
    }

    /// Draw the indicator on `edge` instead of the default [`IndicatorEdge::Top`].
    pub fn edge(mut self, edge: IndicatorEdge) -> Self {
        self.edge = edge;
        self
    }

    /// Mark this tab as the one whose content is displayed. Exactly one tab in a strip should be.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// What a primary press asks for — selecting this tab.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// What a secondary (right) press asks for, at the pressed point.
    ///
    /// The wrapper lets the child answer first and intercepts only the right button, so
    /// [`Self::on_press`] keeps working through it.
    pub fn on_secondary_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_secondary_press = Some(Box::new(f));
        self
    }

}

/// The colour a tab draws its own content in — the accent when it is the marked one, the muted tint
/// otherwise.
///
/// A free function rather than an associated one so a call site can reach it without naming the
/// tab's message type. Every control **nested inside** a tab has to take this too: without a
/// container the accent is the only thing separating the active tab from its neighbours, so a close
/// glyph left on `IconButton`'s `on_surface` default would read as belonging to a different tab
/// than the label beside it (feature 012 BUG-001, FR-011a).
pub fn content_colour(is_active: bool, r: Roles) -> Rgb {
    indicator_colour(is_active, r).unwrap_or(r.on_surface_variant)
}

/// The leading slot, at [`SLOT_WIDTH`] whether it is filled or not.
///
/// The width is the slot's, never its content's, and that is the whole point of the function. A
/// slot nobody filled is still a slot — see the module doc, and feature 023 FR-008a: a child that
/// comes and goes inside a pressable control is the positional-`diff_children` trap.
///
/// **A slot that resized to fit is the same defect wearing the other hat**, and it is the one this
/// function was extracted to fix. Feature 026's stopped mark is an `ActivityBadge`, which reserves
/// the sidebar tag's ~11dp rather than a touch target's 48 — so a tab whose process had stopped
/// built a row 37dp narrower than its neighbours', and the centring column pulled its label 20dp
/// toward the leading edge. Every geometry gate was green: `tab_children_fit` asks about tabs in
/// the *application*, where no tab carries a mark until T049, and each node was exactly where its
/// own layout said it was. It was found by measuring glyph ink across the gallery's own row (T013)
/// — the labels fell at 94.5, 255.5 and 396.5 on a 161dp pitch, so the third was 20 short of the
/// 416.5 its own midline was at.
///
/// Wrapping here rather than at each call site is what makes that unrepeatable: a caller cannot
/// hand a tab something the wrong size, because the size is not the caller's to give.
///
/// # Why only the leading one
///
/// The trailing slot passes its content through at whatever width that content measures, and it has
/// to for now: the close control a terminal tab puts there is a **compact** `IconButton`
/// (`.circular().padding(XS)`), which lays out at 20dp rather than the 48 [`SLOT_WIDTH`]'s own
/// documentation assumes. Boxing it to 48 moves the committed layout fixture, which feature 026's
/// promotion is required not to do.
///
/// So the two slots are **not** equal in the application today — 48 against 20 — and a tab's label
/// therefore sits about 14dp right of its own midline. That is feature 012's, not this feature's:
/// its `tab_children_fit` gate measures the *content row's* centre against the tab's, and a row
/// whose two ends are unequal is still centred as a row, so the gate cannot see where the label
/// inside it landed. Recorded here because measuring it is how it was found, and because T022 —
/// which requires the AI tab's two slots to be equal — is where it has to be confronted rather than
/// carried forward.
fn leading_slot<'a, M: 'a>(content: Option<Element<'a, M>>) -> Element<'a, M> {
    match content {
        Some(content) => container(content)
            .center_x(Length::Fixed(SLOT_WIDTH))
            .center_y(Length::Shrink)
            .into(),
        None => Space::new().width(Length::Fixed(SLOT_WIDTH)).into(),
    }
}

impl<'a, M: Clone + 'a> From<Tab<'a, M>> for Element<'a, M> {
    fn from(t: Tab<'a, M>) -> Self {
        let r = t.roles;
        let content = row![
            leading_slot(t.leading),
            container(t.label)
                .max_width(LABEL_MAX_WIDTH)
                .center_x(Length::Shrink),
            t.trailing
                .unwrap_or_else(|| Space::new().width(Length::Fixed(SLOT_WIDTH)).into()),
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center);

        // Every tab reserves the indicator's height whether or not it draws one. An indicator that
        // appeared only on activation would grow its tab by 3dp and push the row — under the
        // pointer, between a press and its release.
        let bar: Element<'a, M> = match indicator_colour(t.active, r) {
            Some(accent) => Divider::horizontal(r)
                .thickness(anatomy::tab::INDICATOR)
                .tint(accent)
                .into(),
            None => Space::new()
                .height(Length::Fixed(anatomy::tab::INDICATOR))
                .into(),
        };
        // `Fill` on the column, not `Shrink`, and it is the *width* half of the same rule. The
        // active tab's rule fills, so its column measures the tab's whole content box and the row
        // below centres in it; an inactive tab's transparent spacer has no width, so a shrinking
        // column would measure only the row and pin it to the leading edge. The label would then
        // sit off the midline on every inactive tab and slide across on activation by half the
        // slack — 4.6dp at this width, and found only by measuring glyph ink at a fixed crop.
        let marked = match t.edge {
            IndicatorEdge::Top => column![bar, content],
            IndicatorEdge::Bottom => column![content, bar],
        }
        .width(Length::Fill)
        .align_x(Alignment::Center);

        let mut button = Button::with_content(marked, ButtonVariant::Text, r)
            // `Text` on every tab: no background, no outline (FR-004b). One fixed width for all of
            // them, so the indicator's `Fill` resolves to the tab rather than to whatever space the
            // bar happens to offer.
            .width(Length::Fixed(WIDTH))
            .padding(spacing::SM)
            // FR-015: a tab's highlight is a tab's. See the module doc.
            .shape(state_layer_shape());
        if let Some(message) = t.on_press {
            button = button.on_press(message);
        }
        match t.on_secondary_press {
            Some(f) => ContextArea::new(button).on_secondary_press(f).into(),
            None => button.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::text::{Text, TypeRole};
    use iced::widget::Space;
    use iced::Element;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::{self, Roles};

    fn roles() -> Roles {
        tokens::roles(ColorScheme::Dark)
    }

    fn label<'a>(r: Roles) -> Text<'a, ()> {
        Text::new("1", TypeRole::Label, r)
    }

    /// Principle VIII's shape, which `tests/material_builder_api.rs` holds the library to: the
    /// required inputs go to the constructor, everything optional is a chainable step, and the
    /// whole thing terminates in `.into()`.
    ///
    /// A tab's required input is what it shows and what it is themed by. Everything else is a
    /// property of *this* tab among its neighbours — whether it is the marked one, which edge its
    /// indicator sits on, what occupies its two reserved slots — and a call site that wants none of
    /// them should not have to say so four times.
    ///
    /// The parts it reports, not its pixels. Where they land is `tests/gates/tab_children_fit.rs`'s
    /// question and the layout snapshot's; whether the builder carried them at all is this one's,
    /// and it is the half that can be asked without a renderer.
    #[test]
    fn a_tab_is_built_from_its_label_and_takes_its_parts_through_steps() {
        let r = roles();

        let bare: Tab<'_, ()> = Tab::new(label(r), r);
        assert!(
            bare.leading.is_none(),
            "a tab with nothing in its leading slot must not have one filled in for it"
        );
        assert!(bare.trailing.is_none(), "likewise the trailing slot");
        assert!(
            !bare.active,
            "a tab is not the marked one until something says it is"
        );
        assert_eq!(
            bare.edge,
            IndicatorEdge::Top,
            "the default edge is this application's own (feature 012 FR-004b): the strip is \
             anchored to the window's bottom, so the pane a tab selects is above it and a bottom \
             indicator would point away from what it marks"
        );

        let dressed: Tab<'_, ()> = Tab::new(label(r), r)
            .leading(Space::new())
            .trailing(Space::new())
            .edge(IndicatorEdge::Bottom)
            .active(true);
        assert!(dressed.leading.is_some(), "the leading slot was not carried");
        assert!(
            dressed.trailing.is_some(),
            "the trailing slot was not carried"
        );
        assert!(dressed.active, "the active flag was not carried");
        assert_eq!(dressed.edge, IndicatorEdge::Bottom, "the edge was not carried");

        let _: Element<'_, ()> = dressed.into();
    }

    /// FR-015 / SC-010: a tab's state layer is a **rectangle**, not the pill a text button draws.
    ///
    /// The one part of the highlight that is a *value* rather than a composited pixel, and
    /// therefore the one part Principle I can hold. What it cannot say is that a rectangle of the
    /// right size is actually drawn on hover — a state layer is drawn, not laid out, so no geometry
    /// gate can see it and the visual pass is what judges it. What it does fix in place is that
    /// nobody reintroduces the pill by reaching for the button's default again, which is exactly
    /// how a tab came to wear one: it was built as a `ButtonVariant::Text` and inherited it.
    #[test]
    fn a_tabs_state_layer_is_rectangular_and_not_a_buttons_pill() {
        assert_eq!(
            state_layer_shape(),
            shape::NONE,
            "a tab's highlight spans the tab and has its corners (FR-015): a rounded pill inside a \
             strip reads as separate buttons lighting up rather than as one strip with a moving \
             highlight"
        );
        assert_ne!(
            state_layer_shape(),
            shape::FULL,
            "`shape::FULL` is the fully rounded pill every `Button` wraps itself in by default — \
             the shape a tab inherits by being built as one, and the shape FR-015 exists to \
             replace"
        );
    }

    /// FR-010a / feature 012 FR-004a: a slot measures the slot, not what is in it.
    ///
    /// The leading spacer balances the trailing close control so the label sits on the tab's
    /// midline. That holds while the slot is empty and stops holding the moment something narrower
    /// than a touch target goes in it — which is exactly what feature 026's stopped mark is. Found
    /// by measuring glyph ink in the gallery (T013): the marked tab's label was 20dp left of where
    /// its neighbours' sat, with every geometry gate green, because no tab in the application
    /// carries a mark yet.
    #[test]
    fn a_slot_is_the_slots_width_whatever_is_in_it() {
        let r = roles();
        let empty: Element<'_, ()> = leading_slot(None);
        assert_eq!(
            empty.as_widget().size().width,
            Length::Fixed(SLOT_WIDTH),
            "an empty slot must still reserve a touch target's width"
        );
        let filled: Element<'_, ()> = leading_slot(Some(
            crate::ui::material::ActivityBadge::for_emphasis(
                Some(crate::ui::material::BadgeEmphasis::Stopped),
                r,
            )
            .into(),
        ));
        assert_eq!(
            filled.as_widget().size().width,
            Length::Fixed(SLOT_WIDTH),
            "a filled slot must measure the slot, not its content — a mark narrower than the \
             control it balances pulls the label off the tab's midline, and no geometry gate can \
             see it until a tab in a covered state carries one"
        );
    }

    // The four tests below came from `ui/terminal.rs`'s `mod tests` with feature 026's promotion
    // (T010). They describe the tab's own anatomy and they follow the constants they are about;
    // leaving them behind would have left `ui/terminal.rs` testing a component it no longer builds.
    // Their assertions are unchanged.

    /// BUG-002, FR-004b: exactly the active tab carries an indicator.
    ///
    /// Replaces `tab_variant_always_draws_a_container`, which asserted neither arm was
    /// `ButtonVariant::Text`. That test was right for BUG-001 and is wrong now — every tab is
    /// `Text`, because no tab draws a container. It is replaced rather than deleted: a test that
    /// pins a decision *should* fail when the decision changes, and what would be wrong is leaving
    /// the new rule unpinned afterwards.
    #[test]
    fn only_the_active_tab_carries_an_indicator() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            assert_eq!(
                indicator_colour(true, r),
                Some(r.primary),
                "{scheme:?}: the active tab must be marked by an accent indicator (FR-004b)"
            );
            assert_eq!(
                indicator_colour(false, r),
                None,
                "{scheme:?}: an inactive tab draws no indicator — the mark is what distinguishes \
                 the active one, so a second bar would say two tabs are selected"
            );
        }
    }

    /// BUG-005, FR-004c: the tab's width is the sum of what it has to hold, not a number.
    ///
    /// The test that would have failed the day `WIDTH` was written as `128.0`. It cannot catch
    /// a *missing* child on its own — a sum is only as complete as its terms, and the term this bug
    /// was about (the restart affordance) is no longer one of them — so it is the pair to
    /// `tests/gates/tab_children_fit.rs`, which reads what the children were actually given. This
    /// end says the budget is the sum of its parts; that end says nobody was squeezed.
    ///
    /// Restated rather than referenced, deliberately. Writing `assert_eq!(WIDTH, WIDTH)`
    /// through the same expression would pass on any value; spelling the arithmetic out means a
    /// term silently dropped from the definition fails here.
    #[test]
    fn the_tab_width_is_the_sum_of_what_a_tab_holds() {
        let padding = 2.0 * spacing::SM;
        let targets = 2.0 * anatomy::button::MIN_TOUCH_TARGET; // leading spacer + close control
        let gaps = 2.0 * spacing::XS;
        assert_eq!(
            WIDTH,
            padding + targets + gaps + LABEL_MIN_WIDTH,
            "WIDTH must be derived from the constants a tab's widest arrangement requires \
             (FR-004c), not chosen against an observed one. A chosen figure is silently wrong the \
             first time a tab gains a child, and wrong in the one way layout does not report: iced \
             settles a shortfall by shrinking the trailing children, so the control disappears \
             instead of the row overflowing."
        );
    }

    /// The leading spacer balances the whole trailing edge, which is what puts the label on the
    /// tab's midline (FR-004a).
    ///
    /// One control on that edge today. It was briefly two — the close and a restart button — and
    /// the label was then off centre by 30dp with nothing to say so, because the spacer balanced
    /// only the close. FR-010b took the restart out; this fails if anything is put back.
    #[test]
    fn the_leading_slot_balances_the_trailing_one() {
        assert_eq!(
            SLOT_WIDTH,
            anatomy::button::MIN_TOUCH_TARGET,
            "the spacer must balance the control it faces at that control's laid-out footprint, \
             not at its visible pill — a pressable non-compact `IconButton` wraps itself in a \
             MIN_TOUCH_TARGET box, and measuring the pill put the label (48 - 24) / 2 = 12dp left \
             of centre (BUG-002's visual pass)"
        );
    }

    /// The indicator is the *only* difference between the two states, and it must be an accent —
    /// not the surrounding bar's foreground, which would read as a border artefact rather than a
    /// selection (SC-009).
    #[test]
    fn the_indicator_is_an_accent_not_a_surface_colour() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);
            let accent = indicator_colour(true, r).expect("active tab has an indicator");
            assert_ne!(
                accent, r.on_surface,
                "{scheme:?}: the indicator must be an accent, not the bar's own foreground"
            );
            assert_ne!(
                accent, r.surface,
                "{scheme:?}: an indicator painted in the surface colour is invisible"
            );
        }
    }
}
