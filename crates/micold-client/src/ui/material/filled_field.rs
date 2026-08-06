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

use iced::advanced::widget::{tree, Operation, Tree, Widget};
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
}

/// The filled box: container, label, control and active indicator, laid out to §7.7's metrics.
pub struct FilledField<'a, M> {
    /// `[leading, control, trailing, label]`, always four.
    children: Vec<Element<'a, M>>,
    roles: Roles,
    state: State,
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
        }
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

        // The adornments take their natural width at either end; what is left is the control's.
        let mut nodes = Vec::with_capacity(4);
        let leading = self.children[0].as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(inner, VALUE_LINE)),
        );
        let trailing = self.children[2].as_widget_mut().layout(
            &mut tree.children[2],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(inner, VALUE_LINE)),
        );
        let taken = leading.size().width + trailing.size().width;
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
                pad + leading.size().width,
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
        let label = label.move_to((pad, label_y));

        let trailing_w = trailing.size().width;
        nodes.push(leading.move_to((pad, PAD_Y + LABEL_LINE)));
        nodes.push(control);
        nodes.push(trailing.move_to((width - pad - trailing_w, PAD_Y + LABEL_LINE)));
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }
}

impl<'a, M: 'a> From<FilledField<'a, M>> for Element<'a, M> {
    fn from(f: FilledField<'a, M>) -> Self {
        Element::new(f)
    }
}
