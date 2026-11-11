//! `SelectTrigger` — the Material 3 trigger [`Select`](super::Select) and
//! [`MenuSelect`](super::MenuSelect) both draw (feature 1792, FR-001, FR-002, FR-009).
//!
//! A filled [`FormField`](super::FormField): the container shape and fill, the label that rests in
//! the middle and floats once there is a value, the value line, the trailing chevron, the state
//! layers and the ripple. It is one definition so the two controls cannot drift (Constitution
//! Principle VIII).
//!
//! It draws and owns nothing else. **Openness is an input** (`open`, and `focused`), because the two
//! owners keep it differently: `Select` reads it from its widget-tree state inside `Widget::layout`
//! (see its module docs), `MenuSelect` is handed it by its caller. What each does with a press is
//! theirs too; the trigger only paints. Colours, shapes and sizes are tokens and `Roles` (SC-003).

use iced::widget::{container, row};
use iced::{alignment, Element, Length};
use micold_core::tokens::{anatomy, Roles};

use super::form_field::{FieldText, Layer};
use super::style;
use crate::icons::Icon;

/// The Material 3 select trigger. Builder form (Principle VIII):
/// `SelectTrigger::new(value, roles).text(text).open(open).populated(has_value).into()`.
pub struct SelectTrigger<'a, M> {
    value: Element<'a, M>,
    roles: Roles,
    text: FieldText,
    open: bool,
    focused: bool,
    populated: bool,
    disabled: bool,
    pressable: bool,
}

impl<'a, M: 'a> SelectTrigger<'a, M> {
    /// A trigger showing `value` before the chevron, themed by `roles`. The value fills the width
    /// the chevron leaves, so it may clip or ellipsize itself without moving the chevron.
    pub fn new(value: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            value: value.into(),
            roles,
            text: FieldText::default(),
            open: false,
            focused: false,
            populated: false,
            disabled: false,
            pressable: true,
        }
    }

    /// The field's label, supporting text and the like.
    pub fn text(mut self, text: FieldText) -> Self {
        self.text = text;
        self
    }

    /// Whether the list is open: the indicator thickens and the container reads at the pressed
    /// layer.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Whether the trigger holds the keyboard: the indicator thickens and the container reads at
    /// the focus layer.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Whether there is a value to label: it decides whether the label rests or floats.
    pub fn populated(mut self, populated: bool) -> Self {
        self.populated = populated;
        self
    }

    /// Whether the container ripples a press itself (the default). Off when something around it
    /// already does, so a press is not shown twice.
    pub fn pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
        self
    }

    /// Draw it disabled: dimmed, no hover layer and no ripple. The caller dims the `value` it
    /// passes, which this cannot reach.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The state layer the trigger carries.
///
/// A free function so the ordering can be asserted without a renderer. An open list reads at the
/// pressed opacity (§7.7) and a select that merely holds the keyboard reads at the focus opacity;
/// which of the two wins when both are true is [`Layer`]'s to say, exactly as it is for the
/// checkbox — one control shows **one** layer.
pub(super) fn trigger_layer(open: bool, focused: bool) -> Layer {
    let open = if open { Layer::Pressed } else { Layer::None };
    open.max(if focused { Layer::Focused } else { Layer::None })
}

impl<'a, M: 'a> From<SelectTrigger<'a, M>> for Element<'a, M> {
    fn from(t: SelectTrigger<'a, M>) -> Self {
        let r = t.roles;
        let chevron: Element<'a, M> = if t.disabled {
            super::glyph::icon_colored(
                Icon::SelectChevron,
                anatomy::text_field::TRAILING_ICON,
                style::disabled_color(r.on_surface_variant),
            )
        } else {
            super::glyph::icon(
                Icon::SelectChevron,
                anatomy::text_field::TRAILING_ICON,
                r.on_surface_variant,
            )
        };
        let line = row![container(t.value).width(Length::Fill), chevron,]
            .align_y(alignment::Vertical::Center);

        // No padding, no background and no ripple of its own. All three belong to the container:
        // it is the thing the pointer is read against, so it is the thing that shades and ripples
        // (FR-034, BUG-002). A trigger that padded itself would also sit inset from every text
        // field beside it.
        let control: Element<'a, M> = container(line)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let field = super::FormField::new(control, r)
            // Thickened by the keyboard as well as by the list (FR-030): the same mark a focused
            // text field carries, so a settings surface tabbed through shows focus in one language.
            .active(t.open || t.focused)
            // An open list reads at the pressed opacity — §7.7 asks the select's open state to be
            // carried by the layer. Hover is not passed: the container sees the cursor itself.
            .layer(trigger_layer(t.open, t.focused))
            // Every pressable surface ripples (feature 019, FR-024c) and this is one.
            .pressable(t.pressable)
            .disabled(t.disabled)
            // A selection counts as content; an unset select rests its label like an empty input.
            .populated(t.populated);
        t.text.apply(field).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One control shows one layer, and which one is the enum's to say. A select that is both open
    /// and focused must not blend two opacities into a tone no token names — the same rule
    /// `style::checkbox` settles with the same `max`.
    #[test]
    fn the_trigger_shows_one_layer_and_open_outranks_focused() {
        assert_eq!(trigger_layer(false, false), Layer::None);
        assert_eq!(trigger_layer(false, true), Layer::Focused);
        assert_eq!(trigger_layer(true, false), Layer::Pressed);
        assert_eq!(
            trigger_layer(true, true),
            Layer::Pressed,
            "an open select is already reading at the pressed opacity; adding the focus layer to \
             it would say the same state twice",
        );
    }
}
