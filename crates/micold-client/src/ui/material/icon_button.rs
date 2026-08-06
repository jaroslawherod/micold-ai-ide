//! `IconButton` — a reusable icon-only button primitive (Constitution Principle VIII).
//!
//! Theme-aware (tinted via a design-system color role) with a disabled state. Exposed as a
//! chainable builder terminating in `.into()` (Principle VIII builder-API rule): construct with
//! the required glyph + roles, then set optional size/tint/press before converting to an
//! `Element`. Reused across the sidebar and any icon action rather than each feature forking one.

use crate::icons::Icon;
use crate::ui::material::glyph::{icon, icon_colored};
use crate::ui::material::style;
use crate::ui::material::text::TypeRole;
use iced::widget::{button, container};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, shape, spacing, Rgb, Roles};
use std::marker::PhantomData;

/// A boxed button style function — lets [`From::from`] pick between [`style::text_button`] and
/// [`style::circular_icon_button`] at runtime despite each being a distinct `impl Fn` opaque type.
type ButtonStyleFn = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

/// A compact icon-only button. Without an `on_press` it renders disabled (greyed, inert).
/// Styled as a low-emphasis text button so it sits unobtrusively in dense rows.
pub struct IconButton<'a, M> {
    glyph: Icon,
    roles: Roles,
    size: f32,
    tint: Option<Rgb>,
    padding: f32,
    compact: bool,
    circular: bool,
    on_press: Option<M>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> IconButton<'a, M> {
    /// Build an icon button for `glyph` themed by `roles`. Defaults: the body role's size,
    /// `on_surface` tint, `spacing::XS` padding, no press action (disabled).
    pub fn new(glyph: Icon, roles: Roles) -> Self {
        Self {
            glyph,
            roles,
            size: TypeRole::Body.size(),
            tint: None,
            padding: spacing::XS,
            compact: false,
            circular: false,
            on_press: None,
            _marker: PhantomData,
        }
    }

    /// Size the glyph to a type role instead of the default body role — so a call site names what
    /// the icon should match rather than a number (FR-004).
    pub fn size(mut self, role: TypeRole) -> Self {
        self.size = role.size();
        self
    }

    /// Keep the button's natural size instead of enlarging it to §7.3's 48dp target.
    ///
    /// **A recorded deviation, not a preference.** FR-027 asks for a 48×48 minimum interactive
    /// target on every control. FR-026a/FR-026c protect the sidebar's `dense` density, whose rows
    /// are 36dp. The sidebar header carries four controls beside its title in a ~260dp panel: at
    /// 48dp each they leave the title 60dp, and at the dense 36dp they still leave 60dp — neither
    /// fits the word "Worktrees", which wants 73.5px. `tests/layout_text_overflow.rs` is what
    /// showed that, by reporting the title painting past its clip.
    ///
    /// So inside the sidebar the target stays as it was. That is FR-027 unmet in one region, and it
    /// is written down here and in `tasks.md` rather than resolved by quietly shrinking the title
    /// or dropping a control — both of which would be a product decision made by a styling pass.
    /// Every icon button outside the sidebar takes the full 48dp.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Override the button's padding (defaults to `spacing::XS`) — widen the click target for a
    /// button that otherwise sits in a dense row (e.g. `spacing::SM`).
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Render a fully-rounded (circular) hit area around the glyph instead of the default
    /// squarish-rounded one — reaches a true circle only when width and height end up equal, so
    /// pair it with a small, uniform `.padding(...)` (e.g. `spacing::XS`).
    pub fn circular(mut self) -> Self {
        self.circular = true;
        self
    }

    /// Override the icon tint (defaults to the roles' `on_surface`).
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Set the message emitted on press (enables the button).
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Set the press message from an `Option` (disabled when `None`).
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, M: Clone + 'a> From<IconButton<'a, M>> for Element<'a, M> {
    fn from(b: IconButton<'a, M>) -> Self {
        let tint = b.tint.unwrap_or(b.roles.on_surface);
        // Grey the glyph here rather than relying on `text_button`'s `Status::Disabled` branch:
        // `icon` sets an explicit `.color()`, which overrides the button's inherited
        // `text_color`, so the style fn could never reach the glyph and a disabled icon button
        // rendered at full strength — contradicting this type's own doc comment.
        let content = if b.on_press.is_none() {
            icon_colored(b.glyph, b.size, style::disabled_color(tint))
        } else {
            icon(b.glyph, b.size, tint)
        };
        // `text_button`/`circular_icon_button` are each a distinct `impl Fn` opaque type, so an
        // `if`/`else` can't bind them to one local — box both branches to a common `dyn Fn`.
        let style_fn: ButtonStyleFn = if b.circular {
            Box::new(style::circular_icon_button(b.roles))
        } else {
            Box::new(style::text_button(b.roles))
        };
        let pressable = b.on_press.is_some();
        let mut btn = button(content).padding(b.padding).style(style_fn);
        if let Some(message) = b.on_press {
            btn = btn.on_press(message);
        }
        // Ripples in the glyph's own tint, and only when it can be pressed — a disabled icon
        // button already grades its glyph down, and a ripple would contradict that.
        let element: Element<'a, M> = if pressable {
            super::Ripple::new(btn, tint, shape::FULL).into()
        } else {
            btn.into()
        };
        if !pressable {
            return element;
        }
        // §7.3's 48dp minimum interactive target, honoured **even though the visible container is
        // smaller** — a 24dp glyph in a 40dp box is the ordinary case here, so a target that merely
        // matched the container would be wrong on every icon button in the application (FR-027).
        //
        // A centring container rather than padding on the button: padding would grow the *visible*
        // pill along with the target, and §7.3 asks for a large target around a small container,
        // not a large container.
        if b.compact {
            return element;
        }
        container(element)
            .width(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .height(Length::Fixed(anatomy::button::MIN_TOUCH_TARGET))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
