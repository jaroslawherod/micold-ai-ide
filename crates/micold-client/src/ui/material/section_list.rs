//! `SectionList` — the navigation rail a full-surface view is divided by (Principle VIII, feature
//! 027 FR-026/FR-026a).
//!
//! A column of named destinations, one of them current. Pressing one emits its message; the
//! caller changes what it shows and passes a new selection back. The component holds no state of
//! its own, which is what lets the same rail serve a settings surface today and anything else
//! later — the selection lives where the thing being selected lives.
//!
//! # Why this is in the library
//!
//! FR-026a asks for it explicitly, and the reason is worth stating: a rail built privately inside
//! the settings view is invisible to every gate this crate holds components to. It would not be
//! checked for the builder shape, would not appear in the showcase, would not be held to the type
//! scale or the state layers, and the second view that wanted one would grow its own. The
//! `NavigationDrawer` beside it is the same argument already won once.
//!
//! # What it is not
//!
//! Not a `NavigationDrawer`. That one animates a panel out of the way and leaves a rail behind —
//! it answers "is the panel on screen?". This answers "which destination is current?", and the two
//! compose: the settings surface puts a `SectionList` *inside* a drawer, so the rail can collapse
//! at a narrow window without either component knowing about the other's question.
//!
//! Not a `Select` either, though both pick one of several. A select hides the alternatives behind
//! a trigger and is a *field* — it edits a value. This shows every destination at once and is
//! *navigation* — it changes what the surface displays and edits nothing.

use std::marker::PhantomData;

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};

use super::{Button, ButtonVariant, Glyph, Tag, Text, TypeRole};
use crate::icons::Icon;
use micold_core::tokens::{spacing, Rgb, Roles};

/// The rail's width. Fixed rather than shrink-to-fit so that switching sections never moves the
/// content beside it — a rail that resized with its own selection would shift the whole form
/// sideways.
///
/// Wide enough for the widest row the application can produce, which is not the widest *label*: the
/// current row is drawn `Filled` and so is inset by `PADDING_FILLED` where every other row is inset
/// by `PADDING_TEXT`, and it may carry a badge as well. At 208 the longest name fit everywhere
/// except where it mattered — "Session service" wrapped onto two lines exactly when it was the
/// section you were on (found by the T075 visual pass; every layout gate was green, because a
/// wrapped label occupies the box it was given). The test
/// `the_current_row_fits_the_widest_label_and_a_badge` keeps that arithmetic honest.
const RAIL_WIDTH: f32 = 288.0;

/// The rail's width with the labels hidden (FR-026c) — Material 3's navigation-rail width.
///
/// A second fixed width rather than a shrink-to-fit, for the reason the first one is fixed: the
/// content beside the rail must not move when the selection changes. What *does* move is the
/// boundary between the two states, and that is the point — the width the labels gave up goes to
/// the section, which is the whole return on collapsing.
const RAIL_WIDTH_COLLAPSED: f32 = 80.0;

/// One destination in a [`SectionList`].
///
/// A record, not a component: it carries no appearance of its own and never becomes an element on
/// its own terms — the list decides how a destination is drawn, which is what keeps a selected row
/// in one place rather than one per call site. Public fields for the same reason [`MenuItem`]'s
/// are: a caller builds these in a `map`, and a constructor per optional field would be noise.
///
/// [`MenuItem`]: super::MenuItem
pub struct Section<M> {
    /// The destination's name, as shown.
    pub label: String,
    /// Emitted when this destination is pressed — including when it is already current, so that
    /// pressing the current row is inert rather than special.
    pub message: M,
    /// A short trailing marker, shown beside the label. For a destination whose *content* has
    /// something to say from outside it — "Sharing", on a section holding an opt-in that is on.
    pub badge: Option<String>,
    /// The glyph identifying this destination (FR-026b). It is what the row is reduced to when the
    /// rail is collapsed, so a rail whose destinations have none cannot usefully collapse — which
    /// is why [`row_parts`] keeps the label for a row without one rather than drawing nothing.
    pub icon: Option<Icon>,
}

impl<M> Section<M> {
    /// A destination with no badge.
    pub fn new(label: impl Into<String>, message: M) -> Self {
        Self {
            label: label.into(),
            message,
            badge: None,
            icon: None,
        }
    }
}

/// A rail of named destinations with one of them current. Builder form (Principle VIII):
/// `SectionList::new(sections, roles).selected(i).into()`.
pub struct SectionList<'a, M> {
    sections: Vec<Section<M>>,
    selected: usize,
    badge_accent: Option<(Rgb, Rgb)>,
    collapsed: bool,
    toggle: Option<M>,
    roles: Roles,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: Clone + 'a> SectionList<'a, M> {
    /// A rail showing `sections`, with the first current.
    pub fn new(sections: Vec<Section<M>>, roles: Roles) -> Self {
        Self {
            sections,
            selected: 0,
            badge_accent: None,
            collapsed: false,
            toggle: None,
            roles,
            _marker: PhantomData,
        }
    }

    /// Which destination is current, by index.
    ///
    /// An index out of range marks none of them rather than panicking or clamping to an end: the
    /// selection is the caller's state, and a caller mid-edit — a section removed, a list rebuilt
    /// — is better served by a rail that shows nothing current for a frame than by one that
    /// silently claims the wrong destination is.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// The colour pair a badge is drawn in — its fill, and the colour of the text on it. Defaults
    /// to the roles' primary accent; state it when the badge is a *warning* rather than a marker.
    ///
    /// A pair rather than one accent because the badge is drawn opaque, and it is drawn opaque
    /// because it sits on two different backgrounds: the surface behind an ordinary row, and the
    /// `primary` fill of the current one. A single accent at the chip's usual 20% tint disappeared
    /// into the second (T075).
    pub fn badge_accent(mut self, fill: Rgb, on_fill: Rgb) -> Self {
        self.badge_accent = Some((fill, on_fill));
        self
    }

    /// Draw the rail as icons alone (FR-026c). Every destination stays pressable and the current
    /// one stays marked; only the names go.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// The message the rail's own collapse control emits.
    ///
    /// The control belongs to the component, not to the caller: a rail that could be collapsed by
    /// a button the surface drew somewhere else would be a rail whose two states no other view
    /// could reuse — which is FR-026a's whole objection to a privately-built rail. The glyph
    /// follows the state, so the caller never picks one.
    pub fn toggle(mut self, message: M) -> Self {
        self.toggle = Some(message);
        self
    }
}

/// How a row at `index` is drawn given the current selection.
///
/// A free function so the rule can be asserted without a renderer: exactly one index is filled,
/// and an out-of-range selection fills none.
fn variant_at(index: usize, selected: usize) -> ButtonVariant {
    if index == selected {
        ButtonVariant::Filled
    } else {
        ButtonVariant::Text
    }
}

/// What one row draws, given the rail's state and what the destination carries.
///
/// A free function for the same reason [`variant_at`] is one: collapsing must be shown to cost no
/// *information*, and that is a claim about which parts a row is built from — not about pixels. A
/// renderer-level test could only say the rail got narrower.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowParts {
    /// The destination's glyph is drawn.
    pub icon: bool,
    /// Its name is drawn.
    pub label: bool,
    /// Its badge is drawn as a trailing chip beside the name.
    pub badge_chip: bool,
    /// Its badge is drawn by tinting the glyph instead, there being no room for a chip.
    pub badge_tint: bool,
}

/// See [`RowParts`].
///
/// The rule the two branches share: a badged destination is marked in **both** states. FR-004c asks
/// that an active credential opt-in be visible at a glance, and "at a glance" cannot mean "once you
/// reopen the rail" — a badge that disappeared when the labels did would make collapsing a way to
/// stop being told you are sharing something.
///
/// The second rule is that a destination with no glyph keeps its name even collapsed. Every section
/// this application has carries one (FR-026b, held by `tests/settings_rail.rs`), so that branch is
/// for a rail built elsewhere: drawing nothing at all would make a destination unreachable, which is
/// the one thing FR-026c forbids.
fn row_parts(collapsed: bool, has_icon: bool, has_badge: bool) -> RowParts {
    let iconic = collapsed && has_icon;
    RowParts {
        icon: has_icon,
        label: !iconic,
        badge_chip: has_badge && !iconic,
        badge_tint: has_badge && iconic,
    }
}

impl<'a, M: Clone + 'a> From<SectionList<'a, M>> for Element<'a, M> {
    fn from(list: SectionList<'a, M>) -> Self {
        let roles = list.roles;
        let (badge_fill, badge_on_fill) = list
            .badge_accent
            .unwrap_or((roles.primary, roles.on_primary));
        let selected = list.selected;
        let collapsed = list.collapsed;

        let rows = list.sections.into_iter().enumerate().map(|(i, section)| {
            let variant = variant_at(i, selected);
            let parts = row_parts(collapsed, section.icon.is_some(), section.badge.is_some());
            // The label is drawn at the button's own content colour, so it is the *variant* that
            // decides whether the current row reads as filled — not a tint chosen here. Building
            // the row's text by hand would put a second answer to "what colour is a button's
            // label" in the library, which is the drift `Button::leading`'s history records.
            let content_tint = variant.content(roles, None);
            let mut content = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if let (true, Some(icon)) = (parts.icon, section.icon) {
                let tint = if parts.badge_tint {
                    badge_fill
                } else {
                    content_tint
                };
                content = content.push(Glyph::new(icon, TypeRole::Action, roles).tint(tint));
            }
            if parts.label {
                content = content.push(
                    Text::new(section.label, TypeRole::Action, roles)
                        .tint(content_tint)
                        .width(Length::Fill),
                );
            }
            if parts.badge_chip {
                if let Some(badge) = section.badge {
                    content = content.push(
                        Tag::<M>::new(badge, badge_fill)
                            .solid(badge_on_fill)
                            .role(TypeRole::Caption),
                    );
                }
            }

            // Centred when the row is nothing but its glyph, and only then. The current row is
            // `Filled` and inset by `PADDING_FILLED`; every other row is `Text` and inset by
            // `PADDING_TEXT` — a difference the labels hide and a column of bare icons does not.
            // Left-aligned, the current section's icon sat ~5dp right of the other three and the
            // rail stopped reading as a column (found by the §B.6 visual pass). Padding is
            // symmetric, so centring the content makes both variants land on the same axis.
            let content: Element<'a, M> = if parts.label {
                content.into()
            } else {
                container(content).center_x(Length::Fill).into()
            };

            Button::with_content(content, variant, roles)
                .width(Length::Fill)
                .on_press(section.message)
                .into()
        });

        let mut items = column(rows).spacing(spacing::XS);
        if let Some(message) = list.toggle {
            // Beneath the destinations, not above them: the rail's own control is not one of the
            // places the user navigates to, and putting it first would make the top-left glyph —
            // where the eye starts — the one that goes nowhere.
            items = items
                .push(Space::new().height(Length::Fill))
                .push(collapse_control(collapsed, message, roles));
        } else {
            items = items.push(Space::new().height(Length::Fill));
        }

        container(items)
            .width(Length::Fixed(if collapsed {
                RAIL_WIDTH_COLLAPSED
            } else {
                RAIL_WIDTH
            }))
            .height(Length::Fill)
            .padding(spacing::SM)
            .into()
    }
}

/// The rail's own collapse control, drawn like a destination that is never current.
///
/// Labelled when there is room for a label, because "what does this glyph do?" is a question a user
/// should have to ask at most once — and once the rail is collapsed the answer is on screen in the
/// rail's own shape.
fn collapse_control<'a, M: Clone + 'a>(
    collapsed: bool,
    message: M,
    roles: Roles,
) -> Element<'a, M> {
    let icon = if collapsed {
        Icon::ShowSidebar
    } else {
        Icon::HideSidebar
    };
    let tint = ButtonVariant::Text.content(roles, None);
    let mut content = row![Glyph::new(icon, TypeRole::Action, roles).tint(tint)]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    if !collapsed {
        content = content.push(
            Text::new("Collapse", TypeRole::Action, roles)
                .tint(tint)
                .width(Length::Fill),
        );
    }
    // Centred once it is a bare glyph, on the same axis as the destinations above it — it is a
    // `Text` button like the unselected rows, but the current row is `Filled` and inset further,
    // so "match the rows" only holds if all of them are centred.
    let content: Element<'a, M> = if collapsed {
        container(content).center_x(Length::Fill).into()
    } else {
        content.into()
    };
    Button::with_content(content, ButtonVariant::Text, roles)
        .width(Length::Fill)
        .on_press(message)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::roles;

    /// How wide "Session service" is at `TypeRole::Action`, and "Sharing" is inside its chip at
    /// `TypeRole::Caption` — both approximate, read off the rendered rail during the T075 visual
    /// pass rather than measured by a shaper.
    ///
    /// Approximate is enough for what they are used for: a *floor* under the rail's content width,
    /// with the slack that follows from rounding both up. They are here so that a longer section
    /// name added later fails a test instead of quietly wrapping.
    const WIDEST_LABEL: f32 = 160.0;
    /// See [`WIDEST_LABEL`].
    const WIDEST_BADGE: f32 = 56.0;

    fn sections() -> Vec<Section<()>> {
        vec![
            Section::new("Appearance", ()),
            Section::new("Terminal", ()),
            Section::new("Session service", ()),
        ]
    }

    /// The property a rail exists to have. Two filled rows say two destinations are current, which
    /// is not a state the surface behind it can be in.
    #[test]
    fn exactly_one_row_is_marked_current() {
        for selected in 0..3 {
            let filled = (0..3)
                .filter(|i| variant_at(*i, selected) == ButtonVariant::Filled)
                .count();
            assert_eq!(
                filled, 1,
                "with section {selected} current, {filled} rows were drawn as current"
            );
        }
    }

    /// A selection past the end marks nothing rather than the last row. The rail is a view of the
    /// caller's state; inventing a current destination it did not ask for would have the surface
    /// and its rail disagreeing about what is on screen.
    #[test]
    fn a_selection_out_of_range_marks_nothing() {
        let filled = (0..3)
            .filter(|i| variant_at(*i, 99) == ButtonVariant::Filled)
            .count();
        assert_eq!(filled, 0);
    }

    /// The rail's width does not depend on which row is current, so choosing a section never moves
    /// the form beside it.
    #[test]
    fn the_rail_is_the_same_width_whichever_section_is_current() {
        let r = roles(ColorScheme::Dark);
        for selected in [0usize, 1, 2, 99] {
            let element: Element<'_, ()> =
                SectionList::new(sections(), r).selected(selected).into();
            assert_eq!(
                element.as_widget().size().width,
                Length::Fixed(RAIL_WIDTH),
                "the rail changed width with section {selected} current"
            );
        }
    }

    /// A badge is content, not a second layout: adding one must not change the rail's footprint,
    /// or turning a credential opt-in on would shift the form.
    #[test]
    fn a_badge_does_not_change_the_rails_width() {
        let r = roles(ColorScheme::Dark);
        let mut badged = sections();
        badged[2].badge = Some("Sharing".into());
        let element: Element<'_, ()> = SectionList::new(badged, r).into();
        assert_eq!(element.as_widget().size().width, Length::Fixed(RAIL_WIDTH));
    }

    /// The row that has the least room for its label is the one the user is on, and it is also the
    /// only one that can be both filled *and* badged. Sizing the rail by the longest name alone put
    /// "Session service" on two lines whenever it was current.
    ///
    /// Arithmetic rather than a rendered measurement, because what broke was arithmetic: the rail
    /// was sized against `PADDING_TEXT` and the row that mattered was drawn with `PADDING_FILLED`.
    #[test]
    fn the_current_row_fits_the_widest_label_and_a_badge() {
        let padding = micold_core::tokens::anatomy::button::PADDING_FILLED;
        let content = RAIL_WIDTH - 2.0 * spacing::SM - 2.0 * padding;
        let needed = WIDEST_LABEL + spacing::SM + WIDEST_BADGE;
        assert!(
            content >= needed,
            "the current row has {content}dp for its content and needs {needed}dp; the widest \
             section name wraps when it is the one selected"
        );
    }

    /// An empty rail is representable. It is not a state this application reaches, but a component
    /// that panics on one is a component that cannot be composed with a computed list.
    #[test]
    fn an_empty_rail_is_representable() {
        let r = roles(ColorScheme::Dark);
        let element: Element<'_, ()> = SectionList::new(Vec::<Section<()>>::new(), r).into();
        assert_eq!(element.as_widget().size().width, Length::Fixed(RAIL_WIDTH));
    }

    fn iconic() -> Vec<Section<()>> {
        sections()
            .into_iter()
            .map(|mut s| {
                s.icon = Some(Icon::Settings);
                s
            })
            .collect()
    }

    fn iconic_badged() -> Vec<Section<()>> {
        let mut sections = iconic();
        sections[2].badge = Some("Sharing".into());
        sections
    }

    /// FR-026c: the width the labels occupied goes to the section beside the rail. A collapse that
    /// left the rail its old width would satisfy every other assertion here and return nothing.
    #[test]
    fn collapsing_gives_the_width_back() {
        let r = roles(ColorScheme::Dark);
        let element: Element<'_, ()> = SectionList::new(iconic(), r).collapsed(true).into();
        assert_eq!(
            element.as_widget().size().width,
            Length::Fixed(RAIL_WIDTH_COLLAPSED)
        );
        const { assert!(RAIL_WIDTH_COLLAPSED < RAIL_WIDTH) };
    }

    /// The collapsed rail is as fixed as the expanded one, and for the same reason: a rail that
    /// widened for the current row — or for a badge — would move the form sideways as you used it.
    #[test]
    fn the_collapsed_rail_is_the_same_width_whatever_it_carries() {
        let r = roles(ColorScheme::Dark);
        for selected in [0usize, 1, 2, 99] {
            for sections in [iconic(), iconic_badged()] {
                let element: Element<'_, ()> = SectionList::new(sections, r)
                    .collapsed(true)
                    .selected(selected)
                    .toggle(())
                    .into();
                assert_eq!(
                    element.as_widget().size().width,
                    Length::Fixed(RAIL_WIDTH_COLLAPSED),
                    "the collapsed rail changed width with section {selected} current"
                );
            }
        }
    }

    /// The claim FR-026c actually makes, stated as parts rather than pixels: collapsing drops the
    /// name and nothing else. The glyph stays — it is now the only way to tell the destination
    /// apart — and so does the badge, in the one form there is room for.
    #[test]
    fn collapsing_drops_the_name_and_keeps_everything_else() {
        let open = row_parts(false, true, true);
        let shut = row_parts(true, true, true);
        assert!(open.icon && shut.icon, "the glyph is what identifies a row");
        assert!(open.label && !shut.label, "only the name is given up");
        assert!(
            shut.badge_tint && !shut.badge_chip,
            "a badged section stays marked when the rail is collapsed (FR-004c)"
        );
        assert!(
            open.badge_chip && !open.badge_tint,
            "with room for the chip there is no reason to tint the glyph instead"
        );
    }

    /// A badge is never silently dropped, in either state. FR-004c's "at a glance" cannot mean
    /// "after you reopen the rail".
    #[test]
    fn a_badge_is_shown_in_both_states() {
        for collapsed in [false, true] {
            for has_icon in [false, true] {
                let parts = row_parts(collapsed, has_icon, true);
                assert!(
                    parts.badge_chip || parts.badge_tint,
                    "collapsed={collapsed} has_icon={has_icon} showed no badge at all"
                );
                assert!(
                    !(parts.badge_chip && parts.badge_tint),
                    "collapsed={collapsed} has_icon={has_icon} drew the badge twice"
                );
            }
        }
    }

    /// The escape hatch: a destination with no glyph keeps its name, or a rail built elsewhere
    /// could collapse into a column of blank buttons — unreachable, which is what FR-026c forbids.
    #[test]
    fn a_row_with_no_icon_keeps_its_name_even_collapsed() {
        let parts = row_parts(true, false, false);
        assert!(parts.label);
        assert!(!parts.icon);
    }

    /// Every row is drawn from something in both states. Stated over the whole input space rather
    /// than case by case, because "draws nothing" is the failure that makes a section unreachable.
    #[test]
    fn no_row_is_ever_drawn_empty() {
        for collapsed in [false, true] {
            for has_icon in [false, true] {
                for has_badge in [false, true] {
                    let p = row_parts(collapsed, has_icon, has_badge);
                    assert!(
                        p.icon || p.label,
                        "collapsed={collapsed} has_icon={has_icon} has_badge={has_badge} \
                         produced a row with nothing in it"
                    );
                }
            }
        }
    }
}
