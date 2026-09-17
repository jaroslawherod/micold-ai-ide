//! `SectionList` — the navigation rail a full-surface view is divided by (Principle VIII, feature
//! 027 FR-026/FR-026a).
//!
//! A column of named destinations, one of them current. Pressing one emits its message; the
//! caller changes what it shows and passes a new selection back. The component holds no application
//! state (only its slide lives in the widget tree, see below), which is what lets the same rail
//! serve a settings surface today and anything else later — the selection lives where the thing
//! being selected lives.
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
//! it answers "is the panel on screen?". This answers "which destination is current?".
//!
//! # It owns its slide
//!
//! Collapsing and expanding slide the rail between its two widths on the same curve and over the
//! same time as the worktree sidebar (feature 030, FR-001–FR-003). The component owns that motion,
//! and no caller opts in or out: `Rail` owns time and width, and each row owns its forms, laid out
//! at their rest widths and moved so its icon stays on its line (`RowSlide`). A caller passes
//! `collapsed` and nothing else; the flag flips on the press, and the rail catches up.
//!
//! Not a `Select` either, though both pick one of several. A select hides the alternatives behind
//! a trigger and is a *field* — it edits a value. This shows every destination at once and is
//! *navigation* — it changes what the surface displays and edits nothing.

use std::any::Any;
use std::marker::PhantomData;
use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{tree, Id, Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Renderer as _, Shell, Widget};
use iced::widget::{column, container, row, Space};
use iced::{touch, window, Alignment, Element, Event, Length, Rectangle, Size, Vector};

use super::keyboard_focus::Focus;
use super::navigation_drawer::parked;
use super::{Button, ButtonVariant, Glyph, Tag, Text, TypeRole};
use crate::icons::Icon;
use crate::ui::cdk::motion::Progress;
use micold_core::tokens::motion::{self, duration};
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

/// A row's width in the labelled rail: the rail less its padding on both sides.
const ROW_WIDTH: f32 = RAIL_WIDTH - 2.0 * spacing::SM;

/// A row's width in the icons-only rail. See [`ROW_WIDTH`].
const ROW_WIDTH_COLLAPSED: f32 = RAIL_WIDTH_COLLAPSED - 2.0 * spacing::SM;

/// At or below this fraction a row draws its icons-only form. Not zero, for the reason the
/// drawer's `CLOSED` is not: a track converges toward its target rather than reaching it. Below it
/// the rail is within 0.2dp of its collapsed width, so the swap moves nothing a pixel.
const ICONS_ONLY: f32 = 0.001;

/// At or above this fraction a badged row draws its labelled form again. See [`ICONS_ONLY`].
const FULL: f32 = 0.999;

/// Which of a row's renderings is drawn at a point of the slide (feature 030, research R3).
///
/// An enum so a row cannot be labelled and icons-only at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowForm {
    /// The row as the expanded rail draws it: glyph, name and chip.
    Labelled,
    /// [`RowForm::Labelled`] with its glyph tinted as the icons-only form tints it, so a badged
    /// row keeps its mark while its chip is cut off (FR-015).
    Marked,
    /// The row as the collapsed rail draws it: the glyph alone.
    IconsOnly,
}

/// How far a row given `width` is from its icons-only width to its labelled one, `0.0` to `1.0`.
///
/// The row's width is the rail's eased progress already, `ROW_WIDTH_COLLAPSED + 208 · p`, so the
/// row needs no second channel from the rail to know it (research R2).
fn fraction(width: f32) -> f32 {
    ((width - ROW_WIDTH_COLLAPSED) / (ROW_WIDTH - ROW_WIDTH_COLLAPSED)).clamp(0.0, 1.0)
}

/// The form a row draws at `fraction` (research R3).
fn form(fraction: f32, has_badge: bool) -> RowForm {
    if fraction <= ICONS_ONLY {
        RowForm::IconsOnly
    } else if fraction >= FULL || !has_badge {
        RowForm::Labelled
    } else {
        RowForm::Marked
    }
}

/// How far to move a row's drawn form sideways so its icon is on its line (research R4).
///
/// The line runs from the icon's x in the icons-only form (`x_icons`) to its x in the labelled
/// form (`x_labelled`) as `fraction` goes from 0 to 1. Whichever form is drawn, its icon, resting
/// at `x_drawn`, lands on that line, so the icon does not jump when the row changes form (FR-014).
fn offset(fraction: f32, x_labelled: f32, x_icons: f32, x_drawn: f32) -> f32 {
    x_icons + (x_labelled - x_icons) * fraction - x_drawn
}

/// The x of `node`'s first leaf in depth-first order, relative to `node` itself: a row's glyph,
/// however deep the form nests it. `None` for a node with no children.
///
/// Relative because a parked form's absolute x (about −8.5e37) has no precision left at dp scale.
fn first_leaf_x(node: &layout::Node) -> Option<f32> {
    let mut leaf = node.children().first()?;
    let mut x = leaf.bounds().x;
    while let Some(child) = leaf.children().first() {
        x += child.bounds().x;
        leaf = child;
    }
    Some(x)
}

/// Finds the focused control in one form's subtree and takes the keyboard from it (research R5).
///
/// What it found is `taken`: the control's index among the subtree's focusable controls, and
/// whether its focus was shown.
///
/// It reads what `TakesTheKeyboard` offers through `custom` rather than through `focusable`, because
/// `Focusable` cannot say whether the focus is shown. Anything else offered there, such as a
/// ripple's state, is neither counted nor touched.
#[derive(Debug, Default)]
struct TakeFocus {
    /// Focusable controls visited so far.
    seen: usize,
    taken: Option<(usize, bool)>,
}

impl Operation for TakeFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        let Some(focus) = state.downcast_mut::<Focus>() else {
            return;
        };
        let (focused, visible) = focus.held();
        if focused {
            self.taken.get_or_insert((self.seen, visible));
            focus.hold(false, false);
        }
        self.seen += 1;
    }
}

/// Gives the keyboard to the `index`-th focusable control in one form's subtree, shown or not as
/// `visible` says, and takes it from every other (research R5). See [`TakeFocus`].
#[derive(Debug)]
struct GiveFocus {
    index: usize,
    visible: bool,
    /// Focusable controls visited so far.
    seen: usize,
}

impl GiveFocus {
    fn new(index: usize, visible: bool) -> Self {
        Self {
            index,
            visible,
            seen: 0,
        }
    }
}

impl Operation for GiveFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        let Some(focus) = state.downcast_mut::<Focus>() else {
            return;
        };
        let this = self.seen == self.index;
        focus.hold(this, this && self.visible);
        self.seen += 1;
    }
}

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
        let paint = Paint {
            roles,
            badge_fill,
            badge_on_fill,
        };
        let rows = list.sections.into_iter().enumerate().map(|(i, section)| {
            let variant = variant_at(i, selected);
            let has_badge = section.badge.is_some();
            let build = |parts: RowParts| destination_form(&section, variant, parts, paint);
            let forms = if section.icon.is_some() {
                // FR-006: each form is the row exactly as that state of the rail builds it.
                let labelled = row_parts(false, true, has_badge);
                Forms::Sliding([
                    build(labelled),
                    build(RowParts {
                        badge_tint: has_badge,
                        ..labelled
                    }),
                    build(row_parts(true, true, has_badge)),
                ])
            } else {
                Forms::Single(build(row_parts(collapsed, false, has_badge)))
            };
            Element::new(RowSlide { forms, has_badge })
        });

        let mut items = column(rows).spacing(spacing::XS);
        if let Some(message) = list.toggle {
            // Beneath the destinations, not above them: the rail's own control is not one of the
            // places the user navigates to, and putting it first would make the top-left glyph —
            // where the eye starts — the one that goes nowhere.
            items = items
                .push(Space::new().height(Length::Fill))
                .push(Element::new(RowSlide {
                    forms: Forms::Sliding([
                        collapse_control(false, message.clone(), roles),
                        collapse_control(false, message.clone(), roles),
                        collapse_control(true, message, roles),
                    ]),
                    has_badge: false,
                }));
        } else {
            items = items.push(Space::new().height(Length::Fill));
        }

        // `Fill`: the width is the rail's to give, and it gives the one the slide has reached.
        let rendering = container(items)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(spacing::SM)
            .into();
        Element::new(Rail {
            rendering,
            collapsed,
        })
    }
}

/// The colours a destination's forms are drawn in.
#[derive(Debug, Clone, Copy)]
struct Paint {
    roles: Roles,
    badge_fill: Rgb,
    badge_on_fill: Rgb,
}

/// One form of a destination's row, drawn from `parts`.
fn destination_form<'a, M: Clone + 'a>(
    section: &Section<M>,
    variant: ButtonVariant,
    parts: RowParts,
    paint: Paint,
) -> Element<'a, M> {
    let Paint {
        roles,
        badge_fill,
        badge_on_fill,
    } = paint;
    // The label is drawn at the button's own content colour, so it is the *variant* that decides
    // whether the current row reads as filled — not a tint chosen here. Building the row's text by
    // hand would put a second answer to "what colour is a button's label" in the library, which is
    // the drift `Button::leading`'s history records.
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
            Text::new(section.label.clone(), TypeRole::Action, roles)
                .tint(content_tint)
                .width(Length::Fill),
        );
    }
    if parts.badge_chip {
        if let Some(badge) = &section.badge {
            content = content.push(
                Tag::<M>::new(badge.clone(), badge_fill)
                    .solid(badge_on_fill)
                    .role(TypeRole::Caption),
            );
        }
    }

    // Centred when the row is nothing but its glyph, and only then. The current row is `Filled`
    // and inset by `PADDING_FILLED`; every other row is `Text` and inset by `PADDING_TEXT` — a
    // difference the labels hide and a column of bare icons does not. Left-aligned, the current
    // section's icon sat ~5dp right of the other three and the rail stopped reading as a column
    // (found by the §B.6 visual pass). Padding is symmetric, so centring the content makes both
    // variants land on the same axis.
    let content: Element<'a, M> = if parts.label {
        content.into()
    } else {
        container(content).center_x(Length::Fill).into()
    };

    Button::with_content(content, variant, roles)
        .width(Length::Fill)
        .on_press(section.message.clone())
        .into()
}

/// What a row can draw (feature 030, research R3).
enum Forms<'a, M> {
    /// A row with a glyph: its labelled form, the same form with its glyph tinted, and its
    /// icons-only form, in that order. Every one is the same widget, so a slot's tree state is
    /// never handed to a different kind of widget.
    Sliding([Element<'a, M>; 3]),
    /// A row with no glyph, which keeps its name collapsed and so follows the rail's width
    /// (FR-013's exemption, research R3a).
    Single(Element<'a, M>),
}

impl<'a, M> Forms<'a, M> {
    fn elements(&self) -> &[Element<'a, M>] {
        match self {
            Forms::Sliding(forms) => forms,
            Forms::Single(form) => std::slice::from_ref(form),
        }
    }

    fn elements_mut(&mut self) -> &mut [Element<'a, M>] {
        match self {
            Forms::Sliding(forms) => forms,
            Forms::Single(form) => std::slice::from_mut(form),
        }
    }
}

/// The slot of [`Forms::Sliding`] each [`RowForm`] is drawn from.
fn slot(form: RowForm) -> usize {
    match form {
        RowForm::Labelled => 0,
        RowForm::Marked => 1,
        RowForm::IconsOnly => 2,
    }
}

/// `cursor` as a row passes it on: unavailable unless it is over the row's own `bounds`.
fn within(cursor: mouse::Cursor, bounds: Rectangle) -> mouse::Cursor {
    if cursor.is_over(bounds) {
        cursor
    } else {
        mouse::Cursor::Unavailable
    }
}

/// One row of the rail as the rail moves (FR-013, FR-014, FR-015).
///
/// Its forms are laid out at their rest widths, so no label wraps and no row changes height while
/// the rail is between them; one is drawn, moved so its glyph is on the line between its two rest
/// positions, and the others are parked. The row owns no time: how far through the slide it is
/// comes from the width the rail gives it.
struct RowSlide<'a, M> {
    forms: Forms<'a, M>,
    has_badge: bool,
}

/// A row's own state: the form a pointer press is held on, if any.
///
/// A press is a click only if its release reaches the form it started on. The slide changes a row's
/// form near each end, so the form a press lands on stays drawn until the press is let go, and the
/// row changes form then. The icon stays on its line either way: the held form is moved like any
/// drawn form.
#[derive(Debug, Default)]
struct Held(Option<usize>);

impl<M> RowSlide<'_, M> {
    /// The child drawn when the row is `width` wide and `held` is the form a press is held on.
    fn drawn(&self, held: &Held, width: f32) -> usize {
        match (&self.forms, held.0) {
            (Forms::Sliding(_), Some(child)) if !self.is_unused(child) => child,
            (Forms::Sliding(_), _) => slot(form(fraction(width), self.has_badge)),
            (Forms::Single(_), _) => 0,
        }
    }

    /// Whether `child` is an unbadged row's marked form, which is never drawn: it gets a zero-size
    /// node and no events, and is laid out only to hand back focus it holds.
    fn is_unused(&self, child: usize) -> bool {
        matches!(self.forms, Forms::Sliding(_)) && child == slot(RowForm::Marked) && !self.has_badge
    }

    /// The child drawn in `layout`, with its layout.
    fn drawn_in<'l>(&self, tree: &Tree, layout: Layout<'l>) -> Option<(usize, Layout<'l>)> {
        let drawn = self.drawn(tree.state.downcast_ref(), layout.bounds().width);
        layout
            .children()
            .nth(drawn)
            .map(|form_layout| (drawn, form_layout))
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for RowSlide<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Held>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Held::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.forms
            .elements()
            .iter()
            .map(|form| Tree::new(form.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(self.forms.elements());
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width;
        let drawn = self.drawn(tree.state.downcast_ref(), width);
        let unused: [bool; 3] = std::array::from_fn(|child| self.is_unused(child));
        let forms = match &mut self.forms {
            Forms::Single(form) => {
                let node = form
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits);
                return layout::Node::with_children(node.size(), vec![node]);
            }
            Forms::Sliding(forms) => forms,
        };
        let at = |rest: f32| {
            layout::Limits::new(Size::new(rest, 0.0), Size::new(rest, limits.max().height))
        };
        let rest = [ROW_WIDTH, ROW_WIDTH, ROW_WIDTH_COLLAPSED];
        let nodes: Vec<layout::Node> = forms
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
            .map(|(child, (form, tree))| {
                // An unbadged row's marked form is never drawn, so it gets a zero-size node here.
                if unused[child] {
                    layout::Node::new(Size::ZERO)
                } else {
                    form.as_widget_mut()
                        .layout(tree, renderer, &at(rest[child]))
                }
            })
            .collect();

        // The keyboard follows the row to the form it draws (FR-007, research R5). A parked form
        // holding focus gives it up, and the drawn form's matching control takes it, shown or not
        // as it was. When no parked form holds focus nothing changes, so this is idempotent.
        for child in (0..forms.len()).filter(|child| *child != drawn) {
            // A marked form laid out only for this step: a badge that clears while the marked form
            // is drawn leaves focus in a form no longer laid out, and `operate` needs its node.
            let node = if unused[child] {
                forms[child].as_widget_mut().layout(
                    &mut tree.children[child],
                    renderer,
                    &at(ROW_WIDTH),
                )
            } else {
                nodes[child].clone()
            };
            let mut take = TakeFocus::default();
            forms[child].as_widget_mut().operate(
                &mut tree.children[child],
                Layout::new(&node),
                renderer,
                &mut take,
            );
            if let Some((index, visible)) = take.taken {
                forms[drawn].as_widget_mut().operate(
                    &mut tree.children[drawn],
                    Layout::new(&nodes[drawn]),
                    renderer,
                    &mut GiveFocus::new(index, visible),
                );
            }
        }

        let [labelled, _, icons_only] = &nodes[..] else {
            unreachable!("a sliding row has three children");
        };
        let x_labelled = first_leaf_x(labelled).unwrap_or(0.0);
        let x_icons = first_leaf_x(icons_only).unwrap_or(0.0);
        let x_drawn = first_leaf_x(&nodes[drawn]).unwrap_or(0.0);
        let shift = offset(fraction(width), x_labelled, x_icons, x_drawn);
        let height = nodes[drawn].size().height;
        let nodes = nodes
            .into_iter()
            .enumerate()
            .map(|(child, node)| {
                if child == drawn {
                    node.translate(Vector::new(shift, 0.0))
                } else {
                    parked(node)
                }
            })
            .collect();
        layout::Node::with_children(Size::new(width, height), nodes)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let width = layout.bounds().width;
        let drawn = self.drawn(tree.state.downcast_ref(), width);
        let unused: [bool; 3] = std::array::from_fn(|child| self.is_unused(child));
        let captured_before = shell.is_event_captured();
        // The drawn form is wider than the row mid-slide, and the part past the row's edge is cut
        // off; the pointer there reaches nothing (FR-009).
        let cursor = within(cursor, layout.bounds());
        // What lets a parked form finish what it started: a press let go, a ripple run out
        // (contract §5 *Pointer*). Nothing that could start something new.
        let finishing = matches!(
            event,
            Event::Window(window::Event::RedrawRequested(_))
                | Event::Mouse(
                    mouse::Event::ButtonReleased(mouse::Button::Left) | mouse::Event::CursorLeft
                )
                | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
        );
        for ((child, form), form_layout) in self
            .forms
            .elements_mut()
            .iter_mut()
            .enumerate()
            .zip(layout.children())
        {
            if child == drawn {
                form.as_widget_mut().update(
                    &mut tree.children[child],
                    event,
                    form_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            } else if finishing && !unused[child] {
                // Its messages and captures go nowhere: a form nobody can see does not act. What
                // it asks of the runtime is kept, so its motion settles rather than freezes.
                let mut dropped = Vec::new();
                let mut parked_shell = Shell::new(&mut dropped);
                form.as_widget_mut().update(
                    &mut tree.children[child],
                    event,
                    form_layout,
                    mouse::Cursor::Unavailable,
                    renderer,
                    clipboard,
                    &mut parked_shell,
                    viewport,
                );
                if parked_shell.is_layout_invalid() {
                    shell.invalidate_layout();
                }
                if parked_shell.are_widgets_invalid() {
                    shell.invalidate_widgets();
                }
                let next = shell.redraw_request().min(parked_shell.redraw_request());
                Shell::replace_redraw_request(shell, next);
            }
        }

        // A press the drawn form took holds that form until it is let go; once it is, the row
        // takes the form its width asks for, which may be another one.
        let pressed = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. })
        );
        let let_go = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
        );
        let held = tree.state.downcast_mut::<Held>();
        if pressed && !captured_before && shell.is_event_captured() {
            held.0 = Some(drawn);
        } else if let_go && held.0.take().is_some() && self.drawn(&Held(None), width) != drawn {
            shell.invalidate_layout();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let cursor = within(cursor, layout.bounds());
        self.drawn_in(tree, layout)
            .map(|(drawn, form_layout)| {
                self.forms.elements()[drawn].as_widget().mouse_interaction(
                    &tree.children[drawn],
                    form_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some((drawn, form_layout)) = self.drawn_in(tree, layout) else {
            return;
        };
        let bounds = layout.bounds();
        let draw = |renderer: &mut iced::Renderer, viewport: &Rectangle| {
            self.forms.elements()[drawn].as_widget().draw(
                &tree.children[drawn],
                renderer,
                theme,
                style,
                form_layout,
                cursor,
                viewport,
            );
        };
        // A form laid out at its rest width is wider than the row mid-slide; what does not fit is
        // cut off at the row's edge rather than drawn over the section beside the rail (research
        // R6). A form that fits is drawn directly, so a rail at rest opens no layer.
        //
        // The viewport is cut to the row too. A clip pushed inside the form (a ripple's bands)
        // replaces this one rather than intersecting with it, and is cut to the viewport it is
        // given, so a whole viewport would let a press's ripple paint past the rail.
        if form_layout.bounds().width > bounds.width {
            let Some(visible) = bounds.intersection(viewport) else {
                return;
            };
            renderer.with_layer(visible, |renderer| draw(renderer, &visible));
        } else {
            draw(renderer, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some((drawn, form_layout)) = self.drawn_in(tree, layout) {
            self.forms.elements_mut()[drawn].as_widget_mut().operate(
                &mut tree.children[drawn],
                form_layout,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let (drawn, form_layout) = self.drawn_in(tree, layout)?;
        self.forms.elements_mut()[drawn].as_widget_mut().overlay(
            &mut tree.children[drawn],
            form_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// The width the rail comes to rest at in each state.
fn width_of(collapsed: bool) -> f32 {
    if collapsed {
        RAIL_WIDTH_COLLAPSED
    } else {
        RAIL_WIDTH
    }
}

/// How long the rail takes to move between its widths (FR-003).
///
/// The *sidebar slide* row of feature 018's motion contract (§6.3), duration and curve both, which
/// the worktree sidebar's `NavigationDrawer` runs on too. Owned here for the reason the drawer owns
/// its own: how long a thing takes is part of how it looks, so no caller picks it.
const SLIDE: Duration = Duration::from_millis(duration::MEDIUM_4);

/// See [`SLIDE`].
const SLIDE_CURVE: motion::Easing = motion::EMPHASIZED;

/// The rail as it moves between its two widths (feature 030, FR-001).
///
/// A widget rather than a width chosen when the view is built, because a width chosen then has no
/// "a frame ago" to move from: that is how the rail used to jump. Not a `NavigationDrawer`: a drawer
/// closes to nothing rather than to a narrower rail, and it reaches its parked child with every
/// operation, so a Tab could land on a copy of the rail nobody can see.
struct Rail<'a, M> {
    rendering: Element<'a, M>,
    collapsed: bool,
}

/// The slide, in widget-tree state (Principle VIII: the application holds no animation state).
struct Slide {
    /// `1.0` expanded, `0.0` collapsed.
    progress: Progress,
}

impl<M> Rail<'_, M> {
    fn target(&self) -> f32 {
        if self.collapsed {
            0.0
        } else {
            1.0
        }
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Rail<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Slide>()
    }

    fn state(&self) -> tree::State {
        // At its destination: opening Settings with the rail collapsed is not a collapse (FR-005).
        tree::State::new(Slide {
            progress: Progress::new(self.target()).easing(
                SLIDE_CURVE.x1,
                SLIDE_CURVE.y1,
                SLIDE_CURVE.x2,
                SLIDE_CURVE.y2,
            ),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.rendering.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.rendering));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(width_of(self.collapsed)), Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let progress = tree.state.downcast_ref::<Slide>().progress.value();
        let width = RAIL_WIDTH_COLLAPSED + (RAIL_WIDTH - RAIL_WIDTH_COLLAPSED) * progress;
        let limits = layout::Limits::new(
            Size::new(width, limits.min().height),
            Size::new(width, limits.max().height),
        );
        let child = self
            .rendering
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        layout::Node::with_children(Size::new(width, child.size().height), vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        // Advance first, so the rows hear the event against the arrangement about to be drawn.
        // `on_layout_frame`: `layout` reads the progress to size the rail (BUG-001).
        let target = self.target();
        tree.state
            .downcast_mut::<Slide>()
            .progress
            .on_layout_frame(event, target, SLIDE, shell);
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        self.rendering.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map(|child_layout| {
                self.rendering.as_widget().mouse_interaction(
                    &tree.children[0],
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(child_layout) = layout.children().next() {
            self.rendering.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child_layout) = layout.children().next() {
            self.rendering.as_widget_mut().operate(
                &mut tree.children[0],
                child_layout,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let child_layout = layout.children().next()?;
        self.rendering.as_widget_mut().overlay(
            &mut tree.children[0],
            child_layout,
            renderer,
            viewport,
            translation,
        )
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
    use iced::advanced::widget::Tree;
    use iced::advanced::Layout;
    use iced::{Point, Size};
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

    /// A row's width inside the rail at rest, restated from the rail's so a slip in
    /// `ROW_WIDTH` fails here rather than agreeing with itself.
    const ROW_LABELLED: f32 = 272.0;
    const ROW_ICONS_ONLY: f32 = 64.0;

    #[test]
    fn a_rows_fraction_runs_from_its_icons_only_width_to_its_labelled_width() {
        assert_eq!(fraction(ROW_ICONS_ONLY), 0.0);
        assert_eq!(fraction((ROW_ICONS_ONLY + ROW_LABELLED) / 2.0), 0.5);
        assert_eq!(fraction(ROW_LABELLED), 1.0);
    }

    #[test]
    fn a_rows_fraction_is_clamped_outside_its_rest_widths() {
        assert_eq!(fraction(ROW_ICONS_ONLY - 24.0), 0.0);
        assert_eq!(fraction(ROW_LABELLED + 28.0), 1.0);
    }

    #[test]
    fn a_row_draws_its_icons_only_form_at_the_collapsed_threshold_and_no_later() {
        for has_badge in [false, true] {
            assert_eq!(form(0.0, has_badge), RowForm::IconsOnly);
            assert_eq!(form(0.001, has_badge), RowForm::IconsOnly);
            assert_ne!(form(0.002, has_badge), RowForm::IconsOnly);
        }
    }

    #[test]
    fn a_badged_row_draws_its_labelled_form_only_from_the_expanded_threshold() {
        assert_eq!(form(1.0, true), RowForm::Labelled);
        assert_eq!(form(0.999, true), RowForm::Labelled);
        assert_eq!(form(0.998, true), RowForm::Marked);
        assert_eq!(form(0.5, true), RowForm::Marked);
        assert_eq!(form(0.002, true), RowForm::Marked);
    }

    /// An unselected row's glyph x inside the rail at rest, and the icons-only one's (contract §3).
    const X_LABELLED: f32 = 20.0;
    const X_ICONS: f32 = 33.0;
    /// The current row's: `Filled` is inset 12dp further than `Text`.
    const X_LABELLED_CURRENT: f32 = 32.0;

    #[test]
    fn the_drawn_icon_is_on_its_line_whichever_form_is_drawn() {
        for f in [0.0, 0.5, 1.0] {
            let line = X_ICONS + (X_LABELLED - X_ICONS) * f;
            for x_drawn in [X_LABELLED, X_ICONS] {
                assert_eq!(
                    x_drawn + offset(f, X_LABELLED, X_ICONS, x_drawn),
                    line,
                    "at f {f}, a form whose icon rests at {x_drawn}"
                );
            }
        }
    }

    #[test]
    fn the_current_rows_line_is_twelve_times_the_fraction_right_of_another_rows() {
        for f in [0.0, 0.5, 1.0] {
            let current = X_ICONS + offset(f, X_LABELLED_CURRENT, X_ICONS, X_ICONS);
            let other = X_ICONS + offset(f, X_LABELLED, X_ICONS, X_ICONS);
            assert_eq!(current - other, 12.0 * f, "at f {f}");
        }
    }

    /// The glyph is the first leaf of both of the control's forms, though the icons-only one
    /// wraps it in a centring container, and it is found relative to the node it is asked about.
    #[test]
    fn the_first_leaf_is_the_glyph_in_both_forms_relative_to_the_node() {
        let renderer = crate::ui::material::test_support::renderer();
        let r = roles(ColorScheme::Dark);
        let padding = micold_core::tokens::anatomy::button::PADDING_TEXT;
        // (collapsed, laid out at, the glyph's x inside the row)
        for (collapsed, width, expected) in [
            (false, ROW_LABELLED, padding),
            (true, ROW_ICONS_ONLY, X_ICONS - spacing::SM),
        ] {
            let mut control = collapse_control(collapsed, (), r);
            let mut tree = Tree::new(control.as_widget());
            let limits = layout::Limits::new(Size::ZERO, Size::new(width, 100.0));
            let node = control
                .as_widget_mut()
                .layout(&mut tree, &renderer, &limits)
                .move_to(Point::new(100.0, 50.0));
            assert_eq!(
                first_leaf_x(&node),
                Some(expected),
                "collapsed {collapsed}: the glyph's x inside its row"
            );
        }
    }

    #[test]
    fn an_unbadged_row_draws_its_labelled_form_between_the_thresholds() {
        assert_eq!(form(0.002, false), RowForm::Labelled);
        assert_eq!(form(0.5, false), RowForm::Labelled);
        assert_eq!(form(0.998, false), RowForm::Labelled);
        assert_eq!(form(0.999, false), RowForm::Labelled);
    }

    /// Two buttons in a column, as a row's forms hold them; the first enabled or not.
    fn pair(first_enabled: bool) -> Element<'static, ()> {
        let r = roles(ColorScheme::Dark);
        iced::widget::Column::new()
            .push(Button::text("First", r).on_press_maybe(first_enabled.then_some(())))
            .push(Button::text("Second", r).on_press(()))
            .into()
    }

    /// Lay `element` out and run `operation` over it.
    fn operate(element: &mut Element<'_, ()>, tree: &mut Tree, operation: &mut dyn Operation) {
        let renderer = crate::ui::material::test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(ROW_LABELLED, 400.0));
        let node = element.as_widget_mut().layout(tree, &renderer, &limits);
        element
            .as_widget_mut()
            .operate(tree, Layout::new(&node), &renderer, operation);
    }

    /// The `i`-th button's focus in a [`pair`]'s tree.
    fn focus(tree: &mut Tree, i: usize) -> &mut Focus {
        tree.children[i].state.downcast_mut::<Focus>()
    }

    #[test]
    fn taking_focus_reports_the_focused_controls_index_and_whether_it_was_shown_and_clears_it() {
        // (the control focused, whether its focus is shown)
        for (index, visible) in [(1, false), (0, true)] {
            let mut element = pair(true);
            let mut tree = Tree::new(element.as_widget());
            focus(&mut tree, index).hold(true, visible);
            let mut take = TakeFocus::default();
            operate(&mut element, &mut tree, &mut take);
            assert_eq!(take.taken, Some((index, visible)));
            for i in 0..2 {
                assert_eq!(
                    focus(&mut tree, i).held(),
                    (false, false),
                    "control {i} after taking focus from control {index}"
                );
            }
        }
    }

    #[test]
    fn giving_focus_focuses_the_nth_control_as_shown_as_it_was_and_clears_the_rest() {
        // (the control given focus, whether it is shown, the control focused beforehand)
        for (index, visible, before) in [(1, false, 0), (0, true, 1)] {
            let mut element = pair(true);
            let mut tree = Tree::new(element.as_widget());
            focus(&mut tree, before).hold(true, true);
            operate(&mut element, &mut tree, &mut GiveFocus::new(index, visible));
            for i in 0..2 {
                let expected = if i == index {
                    (true, visible)
                } else {
                    (false, false)
                };
                assert_eq!(
                    focus(&mut tree, i).held(),
                    expected,
                    "control {i} after giving focus to control {index}"
                );
            }
        }
    }

    /// A pressable button offers its ripple's state through the same `custom` call, and the
    /// operations must neither count it nor panic on it.
    #[test]
    fn the_focus_operations_skip_state_that_is_not_focus() {
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(ROW_LABELLED, 40.0));
        let mut ripple = crate::ui::cdk::ripple::Ripple::new();
        let mut held = Focus::default();
        held.hold(true, true);

        let mut take = TakeFocus::default();
        take.custom(None, bounds, &mut ripple);
        take.custom(None, bounds, &mut held);
        assert_eq!(take.taken, Some((0, true)));

        let mut given = Focus::default();
        let mut give = GiveFocus::new(0, false);
        give.custom(None, bounds, &mut ripple);
        give.custom(None, bounds, &mut given);
        assert_eq!(given.held(), (true, false));
    }

    /// A disabled control is no tab stop, so it is not counted: the first *focusable* control is
    /// the enabled second button.
    #[test]
    fn a_disabled_control_is_not_counted() {
        let mut element = pair(false);
        let mut tree = Tree::new(element.as_widget());
        operate(&mut element, &mut tree, &mut GiveFocus::new(0, true));
        assert_eq!(
            focus(&mut tree, 0).held(),
            (false, false),
            "the disabled one"
        );
        assert_eq!(focus(&mut tree, 1).held(), (true, true), "the enabled one");

        let mut take = TakeFocus::default();
        operate(&mut element, &mut tree, &mut take);
        assert_eq!(take.taken, Some((0, true)));
    }

    /// An element mounted with its tree, laid out at a fixed size and advanced a frame at a time
    /// as iced's loop advances it: `update(RedrawRequested(at))`, then a relayout. Built from `flag`
    /// and rebuilt through `Tree::diff` when it flips, as a view is when its state changes.
    struct Mount<F: Fn(bool) -> Element<'static, ()>> {
        build: F,
        flag: bool,
        element: Element<'static, ()>,
        tree: Tree,
        node: layout::Node,
        renderer: iced::Renderer,
    }

    impl<F: Fn(bool) -> Element<'static, ()>> Mount<F> {
        const SIZE: Size = Size::new(1000.0, 400.0);

        fn new(build: F, flag: bool) -> Self {
            let element = build(flag);
            let tree = Tree::new(element.as_widget());
            let renderer = crate::ui::material::test_support::renderer();
            let mut mount = Self {
                build,
                flag,
                node: layout::Node::new(Size::ZERO),
                element,
                tree,
                renderer,
            };
            mount.layout();
            mount
        }

        fn layout(&mut self) {
            let limits = layout::Limits::new(Size::ZERO, Self::SIZE);
            self.node =
                self.element
                    .as_widget_mut()
                    .layout(&mut self.tree, &self.renderer, &limits);
        }

        fn flip(&mut self) {
            self.flag = !self.flag;
            self.element = (self.build)(self.flag);
            self.tree.diff(self.element.as_widget());
        }

        fn frame(&mut self, at: std::time::Instant) {
            let event = iced::Event::Window(iced::window::Event::RedrawRequested(at));
            let mut messages = Vec::new();
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                iced::mouse::Cursor::Unavailable,
                &self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut iced::advanced::Shell::new(&mut messages),
                &Rectangle::with_size(Self::SIZE),
            );
            self.layout();
        }

        fn width(&self) -> f32 {
            self.node.size().width
        }
    }

    /// The rail slides as the worktree sidebar does, on the same curve over the same time
    /// (FR-003, SC-002). Driven by the same flip and the same frame instants, both have covered the
    /// same share of their width change at every frame. The instants begin 0, +64 ms, +84 ms, a
    /// quarter of `MEDIUM_4` in linear time (the drawer's curve test says why), where the share must
    /// not be linear's; and both are at rest by `MEDIUM_4` plus two gaps.
    #[test]
    fn the_rail_and_the_sidebar_move_alike() {
        use super::super::NavigationDrawer;
        use micold_core::tokens::motion::duration;

        const PANEL: f32 = 300.0;
        const GAP: u64 = 64;
        let r = roles(ColorScheme::Dark);
        let mut drawer = Mount::new(
            |open| {
                NavigationDrawer::new(Space::new().width(PANEL).height(100.0), Space::new())
                    .open(open)
                    .into()
            },
            true,
        );
        let mut rail = Mount::new(
            move |expanded| {
                SectionList::new(iconic_badged(), r)
                    .collapsed(!expanded)
                    .toggle(())
                    .into()
            },
            true,
        );
        assert_eq!(
            (drawer.width(), rail.width()),
            (PANEL, RAIL_WIDTH),
            "precondition: both mounted open"
        );

        drawer.flip();
        rail.flip();
        let settled = duration::MEDIUM_4 + 2 * GAP;
        let mut instants = vec![0, 64, 84];
        while *instants.last().unwrap() < settled {
            instants.push(instants.last().unwrap() + GAP);
        }
        let start = std::time::Instant::now();
        for (sample, at) in instants.iter().enumerate() {
            let at_instant = start + std::time::Duration::from_millis(*at);
            drawer.frame(at_instant);
            rail.frame(at_instant);
            let sidebar = (PANEL - drawer.width()) / PANEL;
            let ours = (RAIL_WIDTH - rail.width()) / (RAIL_WIDTH - RAIL_WIDTH_COLLAPSED);
            assert!(
                (sidebar - ours).abs() <= 0.01,
                "at {at} ms the sidebar had covered {sidebar} of its slide and the rail {ours} \
                 (widths {} and {})",
                drawer.width(),
                rail.width()
            );
            if sample == 2 {
                assert!(
                    (ours - 0.25).abs() > 0.05,
                    "the rail moved linearly: {ours} at a quarter of the slide"
                );
            }
        }
        assert_eq!(
            (drawer.width(), rail.width()),
            (0.0, RAIL_WIDTH_COLLAPSED),
            "not both at rest {settled} ms in"
        );
    }

    /// The rail's rows under a mounted rail's node: the children of the column inside its padded
    /// container, the spacer and the collapse control included.
    fn mounted_rows(node: &layout::Node) -> Vec<Layout<'_>> {
        Layout::new(node)
            .children()
            .next()
            .and_then(|container| container.children().next())
            .map(|column| column.children().collect())
            .unwrap_or_default()
    }

    /// Every drawn leaf under `layout`, in tree order. A parked node keeps its size but is moved
    /// about −8.5e37 away (`navigation_drawer::parked`), so a leaf that far off is not drawn.
    fn drawn_leaves(layout: Layout<'_>) -> Vec<Rectangle> {
        let mut children = layout.children().peekable();
        if children.peek().is_none() {
            let bounds = layout.bounds();
            return if bounds.width > 0.0 && bounds.height > 0.0 && bounds.x > -1.0e6 {
                vec![bounds]
            } else {
                Vec::new()
            };
        }
        children.flat_map(drawn_leaves).collect()
    }

    /// A destination with no glyph keeps its name when collapsed, so it cannot slide as the others
    /// do; it follows the rail's width instead (FR-013's exemption). On every frame of both slides
    /// its badge chip stays inside its row, the row inside the rail, and the row draws something, and the destinations never
    /// reach further down than the taller rest state puts them (FR-015, spec *Edge Cases*).
    #[test]
    fn a_row_with_no_icon_keeps_its_chip() {
        use micold_core::tokens::motion::duration;

        const GAP: u64 = 16;
        const NO_ICON: usize = 2;
        const EDGE: f32 = 0.01;
        let r = roles(ColorScheme::Dark);
        let build = move |expanded: bool| -> Element<'static, ()> {
            let mut sections = iconic();
            sections[NO_ICON].icon = None;
            sections[NO_ICON].badge = Some("Sharing".into());
            SectionList::new(sections, r)
                .collapsed(!expanded)
                .toggle(())
                .into()
        };
        let bottom = |node: &layout::Node| {
            let rows = mounted_rows(node);
            let last = rows[sections().len() - 1].bounds();
            last.y + last.height
        };
        let tallest =
            bottom(&Mount::new(build, true).node).max(bottom(&Mount::new(build, false).node));

        let mut mount = Mount::new(build, true);
        let start = std::time::Instant::now();
        let mut at = 0;
        for _ in 0..2 {
            mount.flip();
            let settled = at + duration::MEDIUM_4 + 2 * GAP;
            let mut widths = Vec::new();
            while at <= settled {
                mount.frame(start + std::time::Duration::from_millis(at));
                widths.push(mount.width());
                let rows = mounted_rows(&mount.node);
                let row = rows[NO_ICON].bounds();
                let leaves = drawn_leaves(rows[NO_ICON]);
                let chip = leaves
                    .last()
                    .unwrap_or_else(|| panic!("at {at} ms the row with no icon was drawn empty"));
                assert!(
                    row.x + row.width <= mount.width() - spacing::SM + EDGE,
                    "at {at} ms the row with no icon {row:?} reached past the rail's padding at {}",
                    mount.width() - spacing::SM
                );
                assert!(
                    chip.x >= row.x - EDGE && chip.x + chip.width <= row.x + row.width + EDGE,
                    "at {at} ms (rail {}) the chip {chip:?} left its row {row:?}",
                    mount.width()
                );
                assert!(
                    bottom(&mount.node) <= tallest + EDGE,
                    "at {at} ms (rail {}) the destinations reached {} below the taller rest state's \
                     {tallest}",
                    mount.width(),
                    bottom(&mount.node)
                );
                at += GAP;
            }
            assert!(
                widths
                    .iter()
                    .any(|w| *w > RAIL_WIDTH_COLLAPSED && *w < RAIL_WIDTH),
                "precondition: no frame had the rail between its widths: {widths:?}"
            );
        }
    }
}
