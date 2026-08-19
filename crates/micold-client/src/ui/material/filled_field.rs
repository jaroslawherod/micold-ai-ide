//! `FilledField` — Material's filled text-field *box*, drawn rather than composed (feature 018,
//! T045 — FR-031; contract §7.7).
//!
//! # Why this is a widget and not a stack of containers
//!
//! The first version of this built the field out of a `container` holding a `column` holding the
//! label and the control. Every individual piece was right — the tone, the roles, the padding — and
//! the result still did not read as Material, because a filled field is not a box with two rows of
//! text in it. It is a box with **fixed internal geometry**:
//!
//! ```text
//!   8dp  ┌──────────────────────────┐  ← 4dp rounded top corners
//!        │  Label            12/16  │  ← baseline fixed at 8dp from the top
//!        │  Value            16/24  │  ← sits directly beneath it, not centred in what is left
//!   8dp  └──────────────────────────┘
//!        ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔  ← the indicator is the box's own bottom edge
//! ```
//!
//! A column distributes leftover space; Material does not have leftover space here. 8 + 16 + 24 + 8
//! is 56 exactly, and every one of those numbers is load-bearing. Composing it left the pair
//! top-heavy with a gap above an indicator that then read as a rule *under* the field rather than
//! part of it — which is what "it doesn't look like Material" turned out to mean.
//!
//! So this owns its layout and its painting. iced's text input stays the *editing* engine — cursor,
//! selection, IME and clipboard are its business and reimplementing them would be a large
//! regression for no visual gain — but it no longer decides where anything sits.
//!
//! # The children are always four
//!
//! Leading adornment, control, trailing adornment, label — present whether or not they are filled,
//! an absent one as a `Shrink` space. A widget whose child count changes is rebuilt by the
//! renderer, and a text input's state includes its focus: a field that gained a child when a
//! validation error appeared would drop the keystroke that caused it (feature 021's lesson).

use iced::advanced::widget::{operation, tree, Id, Operation, Tree, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Renderer as _, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector};
use micold_core::tokens::{anatomy, density, Roles};

use super::style;

/// Where the label sits, and where the value sits beneath it (§7.7).
///
/// Named rather than inlined because they are the whole of the field's internal geometry, and they
/// sum to the container height: `PAD_Y + LABEL_LINE + VALUE_LINE + PAD_Y == 56`.
const PAD_Y: f32 = 8.0;
const LABEL_LINE: f32 = 16.0;
const VALUE_LINE: f32 = 24.0;

/// Which state layer the container carries (§5), strongest last.
///
/// Defined in [`style`] because the checkbox shares the ordering — see there. Hover is deliberately
/// *not* something a caller supplies for a field: see [`FilledField::draw`].
pub use style::Layer;

/// What the field's chrome responds to.
///
/// Grouped rather than passed as three loose booleans, because `(true, false, true)` at a call
/// site says nothing about which of focus, invalidity and the label's position it is describing —
/// and the three are read together everywhere they are read at all.
#[derive(Clone, Copy)]
pub struct State {
    /// Focused: the indicator thickens and takes the accent, and so does the label.
    pub active: bool,
    /// Invalid: the error role, which outranks `active`.
    pub error: bool,
    /// Whether the label has floated to the top, or is resting in the middle of an empty box.
    pub floating: bool,
    /// The strongest layer the *control* reports — focus for a text input, open for the select.
    /// Hover is not in here; the container reads that off the cursor itself.
    pub layer: Layer,
}

/// Whether the control holds the keyboard — asked of the control, not guessed from the pointer.
///
/// The field cannot infer this. A press in its 16dp padding lands on the container and not on the
/// input, and focus also moves for reasons that never reach this widget at all: Tab, a focus
/// operation, the window itself losing and regaining focus. Any rule written here would be a second
/// opinion about a fact the input already holds, and BUG-002 was exactly what two opinions look
/// like. So this asks, through the traversal the rendering stack provides for the purpose, and the
/// answer comes from the input's own state — the only copy of it there is.
///
/// A control that cannot be focused — the select's trigger is a plain container — never answers,
/// which leaves this `false` and its field's `active` entirely in its caller's hands (§7.7 wants a
/// picker's indicator to follow *open*, not focus).
#[derive(Default)]
struct AsksControlForFocus(bool);

impl Operation for AsksControlForFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        self.0 |= state.is_focused();
    }
}

/// Hand the keyboard to the control, for a press that landed on the container around it.
///
/// The mirror of [`AsksControlForFocus`], and it goes through the same traversal for the same
/// reason: focus lives in the control's own state and this is the way in that does not require
/// knowing what the control is. A control with no focus to take never answers.
struct FocusesTheControl;

impl Operation for FocusesTheControl {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if !state.is_focused() {
            state.focus();
        }
    }
}

/// Take the keyboard back from the control, because the application says it no longer holds it.
///
/// The mirror of [`FocusesTheControl`], and the half BUG-004 was missing. A control that cannot be
/// focused never answers, so this is as safe to run over the select's plain container as the other
/// two are.
struct UnfocusesTheControl;

impl Operation for UnfocusesTheControl {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if state.is_focused() {
            state.unfocus();
        }
    }
}

/// What this last told its caller about the control's focus, and what the caller last said back.
///
/// `focused` is remembered so that only *changes* are published: the question is asked on every
/// event, and a field that re-announced "still focused" on every pointer move would put the
/// application into a message loop with itself.
///
/// `supplied` is remembered for the opposite direction (BUG-004). Focus is observed in the control
/// and *held* by the application, which is two copies of one fact, and until now only the control
/// could correct the application. A screen that takes the keyboard back — `State::focus_terminal()`
/// clears `focused_field` with no press landing anywhere near this field — left the field drawn at
/// rest while its input went on swallowing every keystroke. A **change** in the supplied flag is
/// the application changing its mind, and the control adopts it.
///
/// A change and not a disagreement, because a standing disagreement is also what an unreported
/// focus looks like: [`FocusesTheControl`] runs before the answer is asked for, and a focus
/// traversal can take focus without publishing at all.
#[derive(Default)]
struct Reported {
    focused: bool,
    supplied: bool,
    /// A change seen in `diff` and not yet carried to the control — see [`FilledField::diff`].
    pending: Option<bool>,
}

/// The filled box: container, label, control and active indicator, laid out to §7.7's metrics.
pub struct FilledField<'a, M> {
    /// `[leading, control, trailing, label]`, always four.
    children: Vec<Element<'a, M>>,
    roles: Roles,
    state: State,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: 'a> FilledField<'a, M> {
    pub fn new(
        leading: Element<'a, M>,
        control: Element<'a, M>,
        trailing: Element<'a, M>,
        label: Element<'a, M>,
        roles: Roles,
        state: State,
    ) -> Self {
        Self {
            children: vec![leading, control, trailing, label],
            roles,
            state,
            on_focus_change: None,
        }
    }

    /// Report the control gaining or losing the keyboard (BUG-003).
    ///
    /// Focus decides three things at once — where the label sits, whether the indicator thickens,
    /// and which state layer the container carries — and the first of those is settled when the
    /// field is *built*, before this widget exists. So the fact has to travel up: this notices it,
    /// the application holds it, and the next view passes it back down as
    /// [`active`](super::FormField::active). Without a caller here, `active` is the only source and
    /// nothing ever sets it, which is the whole of BUG-003.
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    /// The field's total height: §7.7's 56dp, from the density scale so it follows the axis.
    fn height() -> f32 {
        density::height(density::TEXT_FIELD_BASE, density::STANDARD)
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for FilledField<'a, M> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        // A rebuild is the only place the application's answer can be *seen* changing, so it is the
        // only honest place to notice (BUG-004). An event is not: frames are driven by messages,
        // and several can pass with no input at all, so a flag that went true and back to false
        // between two keystrokes would never be observed by `update` alone.
        //
        // Noticed here and acted on there, because taking the keyboard away from the control means
        // running an operation over it, and that needs a layout and a renderer — neither of which
        // a diff has. What is recorded is the intent; `update` spends it before the input sees its
        // next event, which is the moment before it could matter.
        if self.on_focus_change.is_some() {
            let reported = tree.state.downcast_mut::<Reported>();
            if reported.supplied != self.state.active {
                reported.supplied = self.state.active;
                reported.pending = Some(self.state.active);
            }
        }
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(Self::height()))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let height = Self::height();
        let width = limits.max().width;
        let pad = anatomy::text_field::PADDING;
        let inner = (width - pad * 2.0).max(0.0);

        // The leading adornment takes a **fixed** slot, the trailing one its natural width at the
        // far end; what is left is the control's.
        let mut nodes = Vec::with_capacity(4);
        let leading = self.children[0].as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(
                Size::ZERO,
                Size::new(anatomy::text_field::LEADING_ICON.min(inner), VALUE_LINE),
            ),
        );
        let trailing = self.children[2].as_widget_mut().layout(
            &mut tree.children[2],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(inner, VALUE_LINE)),
        );
        // Everything after the leading slot starts on **one** x — the control and the label alike.
        //
        // This is BUG-003 item 1. The control was inset past the adornment and the label was pinned
        // at `pad`, so a field with both a leading icon and a resting label drew the label
        // underneath the icon — which is the state every search picker opens in. Two rules for one
        // column is the defect; the fix is that there is now one.
        //
        // The slot is a fixed width rather than the glyph's advance (§7.2's rule, BUG-006's
        // lesson): a column that followed the advance would land somewhere different for every
        // icon, and no arithmetic after it could be right for all of them.
        let leading_slot = if leading.size().width > 0.0 {
            anatomy::text_field::LEADING_ICON
        } else {
            0.0
        };
        let indent = if leading_slot > 0.0 {
            leading_slot + anatomy::text_field::LEADING_GAP
        } else {
            0.0
        };
        let content_x = pad + indent;
        let taken = indent + trailing.size().width;
        let control_width = (inner - taken).max(0.0);

        let control = self.children[1]
            .as_widget_mut()
            .layout(
                &mut tree.children[1],
                renderer,
                &layout::Limits::new(
                    Size::new(control_width, 0.0),
                    Size::new(control_width, VALUE_LINE),
                ),
            )
            .move_to((
                content_x,
                if self.state.floating {
                    PAD_Y + LABEL_LINE
                } else {
                    // Resting, the control sits where the label is: an empty input's caret belongs
                    // at the label it is about to replace, not below it.
                    (height - VALUE_LINE) / 2.0
                },
            ));
        // Floating: the small label at the top, with the value beneath it. Resting: the full-size
        // label centred in the box, where the value will appear — it *is* where the value goes,
        // which is what makes it read as the field's content rather than as a caption above it.
        let label_line = if self.state.floating {
            LABEL_LINE
        } else {
            VALUE_LINE
        };
        // Centred *within* its band rather than pinned to the band's top. A line of text measures
        // shorter than the line box it is given — 20dp of glyphs in the 24dp value line — so
        // pinning would leave the resting label sitting a couple of dp above the caret it stands
        // in for, which is visible as a wobble the moment the label lifts.
        let band_center = if self.state.floating {
            PAD_Y + LABEL_LINE / 2.0
        } else {
            height / 2.0
        };
        let label = self.children[3].as_widget_mut().layout(
            &mut tree.children[3],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(inner, label_line)),
        );
        let label_y = band_center - label.size().height / 2.0;
        let label = label.move_to((content_x, label_y));

        // The adornments are centred in the container, not pinned to the floating value's line.
        // Material centres them, and the two positions differ by 8dp — which is the whole gap
        // between a resting label and an icon that is supposed to sit on the same line as it.
        let leading_size = leading.size();
        let trailing_size = trailing.size();
        nodes.push(leading.move_to((
            pad + (leading_slot - leading_size.width).max(0.0) / 2.0,
            (height - leading_size.height) / 2.0,
        )));
        nodes.push(control);
        nodes.push(trailing.move_to((
            width - pad - trailing_size.width,
            (height - trailing_size.height) / 2.0,
        )));
        nodes.push(label);

        layout::Node::with_children(Size::new(width, height), nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let r = self.roles;

        // The container's tone and shape are *asked for*, not restated here: `field_container` is
        // what `text_field_anatomy.rs` asserts against §7.7, and a widget that painted its own
        // colour would put the appearance in one place and the check in another. Rounded on top and
        // square beneath, which is what lets the indicator read as the box's own edge rather than a
        // line drawn under it.
        let container = style::field_container(r)(theme);
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: container.border,
                ..Default::default()
            },
            container
                .background
                .unwrap_or(iced::Background::Color(iced::Color::TRANSPARENT)),
        );

        // The state layer, over the container's **own bounds** (FR-034, BUG-002).
        //
        // This is the whole of that bug's fix and it is worth saying why it lives here rather than
        // on the control. `layout` puts the control in one 24dp value line inside 16dp of padding,
        // so a layer painted by the control covered 440×24 of a 472×56 field — while hover and
        // press were read off the *field*. The rectangle that responds and the rectangle that
        // shades are now the same rectangle because only one thing owns both, and it is this one.
        //
        // Hover is read off the cursor here rather than supplied, for the same reason: a caller
        // that tracked its own hover would be a second opinion about a fact this widget can see,
        // and the two disagreeing is exactly what the bug was. `Select` used to hold a `hovered`
        // flag for this; it does not need to any more.
        let hovered = cursor.is_over(bounds);
        let layer = self
            .state
            .layer
            .max(if hovered { Layer::Hovered } else { Layer::None });
        if layer != Layer::None {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: container.border,
                    ..Default::default()
                },
                iced::Background::Color(style::state_fill(
                    style::color(r.on_surface),
                    layer.opacity(),
                )),
            );
        }

        // The active indicator, drawn *inside* the container's own footprint so the two are one
        // shape. Thickening it is the whole of a filled field's focus affordance — there is no
        // border left to recolour.
        let (colour, thickness) = style::field_indicator(r, self.state.active, self.state.error);
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y + bounds.height - thickness,
                    width: bounds.width,
                    height: thickness,
                },
                ..Default::default()
            },
            iced::Background::Color(colour),
        );

        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style_, layout, cursor, viewport);
        }
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
        // The application changed its mind, and this is the moment before that could matter: the
        // input has not seen this event yet, so a keystroke is handled by whichever control the
        // application currently says holds the keyboard (BUG-004). Noticed in `diff`, spent here.
        if let Some(target) = tree.state.downcast_mut::<Reported>().pending.take() {
            let control = layout
                .children()
                .nth(1)
                .expect("the control is the second of the container's four slots");
            if target {
                self.children[1].as_widget_mut().operate(
                    &mut tree.children[1],
                    control,
                    renderer,
                    &mut FocusesTheControl,
                );
            } else {
                self.children[1].as_widget_mut().operate(
                    &mut tree.children[1],
                    control,
                    renderer,
                    &mut UnfocusesTheControl,
                );
            }
            tree.state.downcast_mut::<Reported>().focused = target;
        }

        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        // A press anywhere in the container reaches the control (FR-034). The control occupies one
        // 24dp value line inside 16dp of padding, so most of a 56dp field is not the input — and a
        // press there used to land on a box that shades, hovers and looks entirely pressable, and
        // do nothing. The layer already covers the whole container; this is the other half of
        // "wherever a press is accepted" being one rectangle rather than two.
        //
        // Not over the control itself — the input handles its own presses, and re-focusing after it
        // has just placed a caret would drag the caret to the end of the text. Not over the
        // adornments either: a trailing icon button is an action of its own, and a press on it is
        // that action rather than an attempt to type.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let mut slots = layout.children();
            let leading = slots.next().expect("leading");
            let control = slots.next().expect("control");
            let trailing = slots.next().expect("trailing");
            let elsewhere = cursor.is_over(layout.bounds())
                && !cursor.is_over(control.bounds())
                && !cursor.is_over(leading.bounds())
                && !cursor.is_over(trailing.bounds());
            if elsewhere {
                // A control that cannot be focused simply never answers, which is what makes this
                // safe to run for the select's plain container as well as for a text input.
                self.children[1].as_widget_mut().operate(
                    &mut tree.children[1],
                    control,
                    renderer,
                    &mut FocusesTheControl,
                );
            }
        }

        // *After* the children, always: the event that changes focus is the one the input has just
        // handled, and asking before it does would report the previous frame's answer.
        if self.on_focus_change.is_some() {
            let mut asks = AsksControlForFocus::default();
            self.children[1].as_widget_mut().operate(
                &mut tree.children[1],
                layout
                    .children()
                    .nth(1)
                    .expect("the control is the second of the container's four slots"),
                renderer,
                &mut asks,
            );
            let reported = tree.state.downcast_mut::<Reported>();
            if reported.focused != asks.0 {
                reported.focused = asks.0;
                if let Some(on_focus_change) = &self.on_focus_change {
                    shell.publish(on_focus_change(asks.0));
                }
            }
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child
                .as_widget_mut()
                .operate(state, layout, renderer, operation);
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
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    /// Stateful only because of what it has already told its caller — see [`Reported`]. Nothing
    /// about the field's *appearance* is kept here; that is all supplied or read off the cursor.
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Reported>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Reported::default())
    }
}

impl<'a, M: 'a> From<FilledField<'a, M>> for Element<'a, M> {
    fn from(f: FilledField<'a, M>) -> Self {
        Element::new(f)
    }
}
