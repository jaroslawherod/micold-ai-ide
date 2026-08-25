//! `Button` — the library's wrapper around the rendering stack's button (Principle VIII).
//!
//! Three variants are in use today, and every call site picked one by naming a style function
//! directly. That is the leak this feature closes: a call site that can name `style::filled` can
//! also name `style::outlined` for a confirm action, or forget the style entirely, and nothing
//! stops it. A call site now names the *variant* and cannot reach the style layer at all.
//!
//! Icon-only buttons keep their own component ([`IconButton`](super::IconButton)) — they carry a
//! glyph rather than a label, and a disabled glyph needs colouring the label path does not.
//!
//! Parity: each variant resolves to exactly the style its call sites use today (FR-005).

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::keyboard_focus::{Indicator, TakesTheKeyboard};
use crate::ui::material::style;
use crate::ui::material::text::{Text, TypeRole};
use iced::widget::{button, container, row};
use iced::{keyboard, Alignment, Element, Length, Padding};
use micold_core::tokens::{anatomy, density, shape, spacing, Rgb, Roles};

/// Each `impl Fn` returned by the style layer is a distinct opaque type, so the variants are boxed
/// behind one signature to be chosen at runtime.
type ButtonStyleFn = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

/// How much emphasis the button carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The primary action of a screen or dialog. One per group.
    Filled,
    /// A secondary action of equal standing — cancel beside confirm.
    Outlined,
    /// A low-emphasis action that sits inside other content: a menu entry, a list row, a tab.
    Text,
}

impl Variant {
    /// The content colour this variant draws its label in — and therefore the colour its ripple
    /// takes, since a state layer is the content colour over the container (contract §5).
    ///
    /// `pub(crate)` for controls *nested inside* a button, which have to match it: a `Button` sets
    /// `text_color` for its own label, but a nested `IconButton` sets an explicit glyph colour that
    /// overrides anything inherited, and its default is the roles' `on_surface` — wrong on any
    /// variant that paints its own fill. Asking the variant is what keeps the two in step; feature
    /// 012 BUG-001 is what happens when they are not.
    pub(crate) fn content(self, roles: Roles) -> micold_core::tokens::Rgb {
        match self {
            Variant::Filled => roles.on_primary,
            Variant::Outlined | Variant::Text => roles.primary,
        }
    }

    fn style(self, roles: Roles) -> ButtonStyleFn {
        match self {
            Variant::Filled => Box::new(style::filled(roles)),
            Variant::Outlined => Box::new(style::outlined(roles)),
            Variant::Text => Box::new(style::text_button(roles)),
        }
    }

    /// §7.3's horizontal padding for this variant — 24 for the two that draw a container, 12 for
    /// the text variant, "because a text button has no container to balance against".
    ///
    /// Read from the contract rather than left to the rendering stack, which insets a button by its
    /// own `DEFAULT_PADDING` of 10dp. That is what every labelled button in the application took
    /// while all three of these constants were referenced by nothing.
    fn padding(self) -> f32 {
        match self {
            Variant::Filled => anatomy::button::PADDING_FILLED,
            Variant::Outlined => anatomy::button::PADDING_OUTLINED,
            Variant::Text => anatomy::button::PADDING_TEXT,
        }
    }
}

/// A labelled button. Builder form (Principle VIII):
/// `Button::filled("Create", roles).on_press(Message::Create).into()`.
///
/// Without an `on_press` the button renders disabled, matching the rendering stack's own rule —
/// so "this action is unavailable" is expressed by having no message to send, not by a flag that
/// could disagree with one.
pub struct Button<'a, M> {
    content: Element<'a, M>,
    variant: Variant,
    roles: Roles,
    on_press: Option<M>,
    padding: Option<Padding>,
    width: Option<Length>,
    leading: Option<(Icon, Option<Rgb>)>,
}

impl<'a, M: Clone + 'a> Button<'a, M> {
    /// The primary action, carrying `label` at the body role.
    pub fn filled(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Filled, roles)
    }

    /// A secondary action of equal standing, carrying `label` at the body role.
    pub fn outlined(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Outlined, roles)
    }

    /// A low-emphasis action, carrying `label` at the body role.
    pub fn text(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Text, roles)
    }

    /// A button wrapping arbitrary `content` — a row of icon plus label, a tag chip, a tree row.
    /// The variant still decides the appearance; only what sits inside differs.
    pub fn with_content(
        content: impl Into<Element<'a, M>>,
        variant: Variant,
        roles: Roles,
    ) -> Self {
        Self {
            content: content.into(),
            variant,
            roles,
            on_press: None,
            padding: None,
            leading: None,
            width: None,
        }
    }

    fn labelled(label: impl Into<String>, variant: Variant, roles: Roles) -> Self {
        let label: Element<'a, M> = Text::new(label.into(), TypeRole::Action, roles).into();
        Self::with_content(label, variant, roles)
    }

    /// The message emitted on press. Omit it and the button is disabled.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The press message from an `Option` — disabled when `None`. For an action that is available
    /// only in some states, so the call site expresses the condition once.
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }

    /// Override the button's padding.
    ///
    /// **A parity affordance, not a design decision.** Today's call sites use four different
    /// paddings for the same variant, and reproducing that exactly is what makes this feature
    /// reviewable. Feature 018 assigns each variant a height from the density scale and this step
    /// goes away with the last caller.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Lay the button out at a given width — `Length::Fill` for a full-width list row.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// A leading icon before the label, drawn at §7.3's [`anatomy::button::LEADING_ICON`].
    ///
    /// The slot belongs to the component because the figure does. Two call sites built
    /// `row![Glyph::new(icon, TypeRole::Action, r), Text::new(..)]` by hand, which sized the glyph
    /// to the *label's* 14dp — the same shape as the icon button's own glyph, and as the menu item
    /// BUG-003's T103 found. §7.3 gives a leading icon 18dp: smaller than an icon button's 24,
    /// because it is an accent to a label rather than the whole content.
    ///
    /// The tint comes from the **variant**, like the label's does — `on_primary` on a filled
    /// button, the accent on an outlined or text one. A leading icon is part of a button's content,
    /// and content is one colour.
    ///
    /// It used to be a required argument, on the reasoning that "inferring it here would change what
    /// they draw today". It would have, and that was the defect: three outlined buttons passed
    /// `on_surface` beside a label the variant draws in `primary`, so each was one control in two
    /// colours. No geometry gate could see it — the glyph was 18dp, in the leading slot, in the
    /// wrong tone — and it was found by looking at the running application (018's BUG-007, T139).
    ///
    /// For the rare glyph whose colour *means* something on its own, use
    /// [`leading_tinted`](Self::leading_tinted).
    pub fn leading(mut self, glyph: Icon) -> Self {
        self.leading = Some((glyph, None));
        self
    }

    /// A leading icon in a stated tint, for a glyph carrying its own meaning — the destructive
    /// `error` red on "Forget", which is saying something the label's accent does not.
    ///
    /// Deliberately the longer name: an override should read as one at the call site, so that the
    /// ordinary case cannot be written by accident.
    pub fn leading_tinted(mut self, glyph: Icon, tint: Rgb) -> Self {
        self.leading = Some((glyph, Some(tint)));
        self
    }
}

impl<'a, M: Clone + 'a> From<Button<'a, M>> for Element<'a, M> {
    fn from(b: Button<'a, M>) -> Self {
        // §7.3's 40dp, the first row of the variant table and the same for all three. It was stated
        // in this file's own documentation ("Feature 018 assigns each variant a height from the
        // density scale") and applied nowhere: `density::BUTTON_BASE` was referenced by no call
        // site, and a filled button laid out at 30dp — its label plus the rendering stack's default
        // padding. Found by `anatomy_size.rs`, which is the check BUG-002 added for exactly this
        // shape: a figure that is right in the token module and never reaches the component.
        //
        // The centring wrapper is not optional, and FR-030a is why. Fixing a height above the
        // content's creates slack, and `button` lays its content out under `limits.height(Fixed)`,
        // which sets the minimum with the maximum — so the content node is stretched to 40dp and
        // draws at the top of it unless something says otherwise. That was BUG-001 exactly, one
        // component over. A wrapper rather than the content's own `align_y` because the content is
        // an arbitrary `Element` here, not a `Text` this type can reach into.
        //
        // A leading icon joins the label here rather than at the call site, so §7.3's 18dp is the
        // component's business — see [`Button::leading`].
        let inner: Element<'a, M> = match b.leading {
            Some((glyph, tint)) => {
                // The variant's own content colour unless the call site meant something by the
                // glyph's tone — one control, one colour, by default.
                let tint = tint.unwrap_or_else(|| b.variant.content(b.roles));
                row![icon(glyph, anatomy::button::LEADING_ICON, tint), b.content]
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center)
                    .into()
            }
            None => b.content,
        };
        let content = container(inner)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        let mut widget = button(content)
            .height(Length::Fixed(density::BUTTON_BASE))
            .style(b.variant.style(b.roles));
        // §7.3's horizontal padding, from the variant table. Vertical padding is zero because the
        // height above is what makes the button 40dp — padding on this axis would add to a figure
        // the contract fixes, and the centring wrapper is what places the content inside it.
        //
        // The caller's override still wins: today's list rows and terminal controls are `Button`s
        // whose inset belongs to the row they sit in, not to §7.3.
        widget = widget.padding(b.padding.unwrap_or(Padding {
            top: 0.0,
            bottom: 0.0,
            left: b.variant.padding(),
            right: b.variant.padding(),
        }));
        if let Some(width) = b.width {
            widget = widget.width(width);
        }
        let on_key = b.on_press.clone();
        let pressable = b.on_press.is_some();
        if let Some(message) = b.on_press {
            widget = widget.on_press(message);
        }
        // Wrapping is the opt-in: every `Button` ripples without any call site asking (FR-024c).
        //
        // Except a disabled one. A button with no `on_press` cannot be pressed, and a ripple on it
        // would report a press that will never happen — worse than no feedback, because it says the
        // opposite of what the disabled styling says.
        let pressed: Element<'a, M> = if pressable {
            super::Ripple::new(widget, b.variant.content(b.roles), shape::FULL).into()
        } else {
            widget.into()
        };

        // And the keyboard, which the rendering stack's button does not have either (FR-030). Every
        // button in the application joins the traversal by being built, exactly as every one of
        // them ripples by being built — a capability a call site has to remember to ask for is a
        // capability most call sites will not have.
        //
        // Outside the ripple rather than inside it, so the focus indicator is drawn over the state
        // layer and not under it.
        let mut focusable = TakesTheKeyboard::new(pressed, pressable);
        if let Some(message) = on_key {
            // Both keys, unlike the checkbox beside it. Enter and Space are what a button answers
            // everywhere it exists — WAI-ARIA names both — and a *focused* button answering Enter
            // takes nothing from the dialog: the field whose `on_submit` saves the form does not
            // hold the keyboard while this does.
            focusable = focusable
                .key(keyboard::key::Named::Enter, message.clone())
                .key(keyboard::key::Named::Space, message);
        }
        focusable
            .indicator(Indicator {
                // §5's ring is `secondary` against every variant, so a focused button is the same
                // mark whether it is filled, outlined or text. The state layer under it is the
                // variant's own content colour, because that is what a state layer is made of.
                outline: b.roles.secondary,
                layer: b.variant.content(b.roles),
                radius: shape::FULL,
            })
            .into()
    }
}
