//! `FieldButton` — looks like a field, acts like a button (feature 1127, research R5, contract C5,
//! T2): the title bar's jump-to field.
//!
//! A pill [`anatomy::app_bar::FIELD_HEIGHT`] tall on `surface_container_highest` with a hairline
//! edge, holding a glyph, a label and an outlined key-cap hint. Pressing it — by pointer, or by
//! Enter or Space once Tab has reached it — sends one message. Nothing is typed into it: the
//! existing `FilledField` and `Typeahead` are real text inputs, which would take the keyboard and
//! duplicate the query of whatever the press opens.
//!
//! # It gives way in three steps
//!
//! The field is drawn in one of three [`Fit`]s, decided at layout from the room it is given, never
//! stored (data-model *Fit*): `Full` (glyph, label, hint) as wide as the room up to
//! [`FieldButton::max_width`]; `GlyphAndHint`; then `Glyph`, an [`anatomy::app_bar::ICON_TARGET`]
//! square that is still pressable. [`width`] reports the width each one needs, so a bar
//! narrowing it drops the label before the hint, each only once it no longer fits (FR-010).
//!
//! Without a press message the field is drawn disabled — its words and edges at the shared 38% —
//! and is not a Tab stop (FR-009, FR-017).
//!
//! Builder form (Principle VIII):
//! `FieldButton::new(Icon::Search, label, roles).hint(chord).max_width(w).on_press_maybe(m).id(id).into()`.

use crate::button::pressable;
use crate::give_way::{FirstThatFits, Need};
use crate::glyph::icon_colored;
use crate::icons::Icon;
use crate::shaped_text::width_of;
use crate::style;
use crate::text::{Text, TypeRole};
use iced::widget::{button, container, row};
use iced::{Alignment, Border, Element, Length, Padding};
use micold_core::tokens::{anatomy, shape, spacing, Rgb, Roles};

/// The label's role: running text, as a field's placeholder reads.
const LABEL_ROLE: TypeRole = TypeRole::Body;

/// The key-cap's role: a terse interface label.
const HINT_ROLE: TypeRole = TypeRole::Label;

/// The glyph's side: a leading icon's, as a button's (§7.3).
const GLYPH: f32 = anatomy::button::LEADING_ICON;

/// The pill's inset at each end.
const INSET: f32 = spacing::SM + spacing::XS;

/// Between the glyph, the label and the hint.
const GAP: f32 = spacing::SM;

/// The key-cap's inset around its words.
const KEY_INSET: Padding = Padding {
    top: 1.0,
    bottom: 1.0,
    left: spacing::XS,
    right: spacing::XS,
};

/// How much of the field is drawn (data-model *Fit*); decided at layout, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// The glyph, the label and the hint, as wide as the room up to the field's maximum.
    Full,
    /// The glyph and the hint.
    GlyphAndHint,
    /// The glyph alone, an `ICON_TARGET` square.
    Glyph,
}

/// The colour the field's glyph, label and hint are drawn in (contract T2): 4.5:1 on its fill.
/// `pub` for the contrast gate, which reads the composition from what the component draws.
pub fn content(roles: Roles) -> Rgb {
    roles.on_surface_variant
}

/// The colour of the pill's and the key-cap's edge (contract T2): 3:1 on the field and the bar.
pub fn outline(roles: Roles) -> Rgb {
    roles.outline
}

/// The least room `field` is drawn at `fit` in: for [`Fit::Full`] the width with nothing to spare,
/// which it grows from up to its maximum. A free function rather than a method, so the builder is
/// all chainable steps (Principle VIII): it is what a bar or a test asks of a field, not
/// configuration.
pub fn width<M: Clone>(field: &FieldButton<M>, fit: Fit) -> f32 {
    let hint = field.hint_width().map_or(0.0, |w| GAP + w);
    match fit {
        Fit::Full => 2.0 * INSET + GLYPH + GAP + width_of(&field.label, LABEL_ROLE).ceil() + hint,
        Fit::GlyphAndHint => 2.0 * INSET + GLYPH + hint,
        Fit::Glyph => anatomy::app_bar::ICON_TARGET,
    }
}

/// A pressable field. Built step by step, then `.into()`.
pub struct FieldButton<M> {
    glyph: Icon,
    label: String,
    roles: Roles,
    hint: Option<String>,
    max_width: f32,
    on_press: Option<M>,
    id: Option<iced::advanced::widget::Id>,
}

impl<M: Clone> FieldButton<M> {
    /// A field showing `glyph` and `label`, themed by `roles`, with no hint and no press yet
    /// (disabled until it has one). Its maximum width is the title bar's field's.
    pub fn new(glyph: Icon, label: impl Into<String>, roles: Roles) -> Self {
        Self {
            glyph,
            label: label.into(),
            roles,
            hint: None,
            max_width: anatomy::app_bar::FIELD_MAX_WIDTH,
            on_press: None,
            id: None,
        }
    }

    /// The outlined key-cap after the label: the shortcut that does what pressing the field does.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The widest the field grows at [`Fit::Full`] when its room is wider.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// What pressing the field sends; `None` draws it disabled and leaves it out of Tab.
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }

    /// Name the field so the application can move the keyboard to it with `focus(id)`.
    pub fn id(mut self, id: impl Into<iced::advanced::widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The width of the key-cap, its words and its inset.
    fn hint_width(&self) -> Option<f32> {
        self.hint
            .as_deref()
            .map(|h| width_of(h, HINT_ROLE).ceil() + KEY_INSET.left + KEY_INSET.right)
    }

    /// The field drawn at `fit`, pressable when it has a message.
    fn rendering<'a>(&self, fit: Fit) -> Element<'a, M>
    where
        M: 'a,
    {
        let r = self.roles;
        let enabled = self.on_press.is_some();
        let tint = content(r);
        let words = |text: String, role: TypeRole| -> Text<'a, M> {
            let t = Text::new(text, role, r);
            if enabled {
                t.tint(tint)
            } else {
                t.disabled_tint(tint)
            }
        };
        let glyph_colour = if enabled {
            style::color(tint)
        } else {
            style::disabled_color(tint)
        };
        let glyph = container(icon_colored(self.glyph, GLYPH, glyph_colour))
            .width(Length::Fixed(GLYPH))
            .center_x(Length::Fixed(GLYPH));
        let edge = if enabled {
            style::color(outline(r))
        } else {
            style::disabled_color(outline(r))
        };
        let hint = |hint: &str, width: f32| {
            container(words(hint.to_string(), HINT_ROLE))
                .padding(KEY_INSET)
                .width(Length::Fixed(width))
                .center_x(Length::Fixed(width))
                .style(move |_theme: &iced::Theme| container::Style {
                    border: Border {
                        color: edge,
                        width: anatomy::button::OUTLINE,
                        radius: shape::EXTRA_SMALL.into(),
                    },
                    ..container::Style::default()
                })
        };

        let mut line = row![glyph].spacing(GAP).align_y(Alignment::Center);
        if fit == Fit::Full {
            line = line.push(words(self.label.clone(), LABEL_ROLE).width(Length::Fill));
        }
        if fit != Fit::Glyph {
            if let (Some(text), Some(width)) = (self.hint.as_deref(), self.hint_width()) {
                line = line.push(hint(text, width));
            }
        }

        let (pill_width, height, inset) = match fit {
            Fit::Full => (Length::Fill, anatomy::app_bar::FIELD_HEIGHT, INSET),
            Fit::GlyphAndHint => (
                Length::Fixed(width(self, fit)),
                anatomy::app_bar::FIELD_HEIGHT,
                INSET,
            ),
            // A square, the glyph in its middle.
            Fit::Glyph => (
                Length::Fixed(anatomy::app_bar::ICON_TARGET),
                anatomy::app_bar::ICON_TARGET,
                (anatomy::app_bar::ICON_TARGET - GLYPH) / 2.0,
            ),
        };
        let pill = button(
            container(line)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .width(pill_width)
        .height(Length::Fixed(height))
        .padding(Padding {
            left: inset,
            right: inset,
            ..Padding::ZERO
        })
        .style(style::field_pill(r));
        let pressed = pressable(
            pill,
            self.on_press.clone(),
            tint,
            shape::FULL,
            r.secondary,
            self.id.clone(),
        );
        if fit == Fit::Full {
            // Grows with its room, but never past its maximum, nor below what it needs.
            container(pressed)
                .max_width(self.max_width.max(width(self, Fit::Full)))
                .into()
        } else {
            pressed
        }
    }
}

impl<'a, M: Clone + 'a> From<FieldButton<M>> for Element<'a, M> {
    fn from(f: FieldButton<M>) -> Self {
        FirstThatFits::new()
            .or(f.rendering(Fit::Full), Need::Width(width(&f, Fit::Full)))
            .or(
                f.rendering(Fit::GlyphAndHint),
                Need::Width(width(&f, Fit::GlyphAndHint)),
            )
            .or(f.rendering(Fit::Glyph), Need::Natural)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{named_key, renderer, roles, Focusables, Mounted};
    use super::*;
    use iced::keyboard::key::Named;
    use iced::{Point, Size};

    const LABEL: &str = "Jump to location, session or project";
    const HINT: &str = "Ctrl+Shift+P";
    const HEIGHT: f32 = 100.0;

    fn field() -> FieldButton<String> {
        // The bundled fonts first: the field measures its words as it is built.
        let _ = renderer();
        FieldButton::new(Icon::Search, LABEL, roles())
            .hint(HINT)
            .on_press_maybe(Some("jump".to_string()))
    }

    fn mounted(f: FieldButton<String>, room: f32) -> Mounted<'static> {
        Mounted::new_in(f, Size::new(room, HEIGHT))
    }

    /// The painted words, in drawing order.
    fn words(m: &mut Mounted<'_>) -> Vec<String> {
        m.painted_text().into_iter().map(|p| p.content).collect()
    }

    fn glyph() -> String {
        Icon::Search.glyph().to_string()
    }

    /// T034 (FR-008, data-model *Fit*): with room to spare the field is a `FIELD_HEIGHT` pill as
    /// wide as its maximum and no wider, drawing its glyph, its label and its hint.
    #[test]
    fn at_full_it_draws_everything_and_never_grows_past_its_maximum() {
        let f = field();
        assert!(
            width(&f, Fit::Full) <= anatomy::app_bar::FIELD_MAX_WIDTH,
            "the title bar's field fits its own maximum: {}",
            width(&f, Fit::Full)
        );
        let mut m = mounted(field(), 1000.0);
        let b = m.node.bounds();
        assert_eq!(b.width, anatomy::app_bar::FIELD_MAX_WIDTH);
        assert_eq!(b.height, anatomy::app_bar::FIELD_HEIGHT);
        let painted = words(&mut m);
        for wanted in [glyph().as_str(), LABEL, HINT] {
            assert!(
                painted.iter().any(|w| w == wanted),
                "{wanted:?}: {painted:?}"
            );
        }
        let narrower = mounted(field().max_width(300.0), 1000.0).node.bounds();
        assert_eq!(narrower.width, 300.0f32.max(width(&f, Fit::Full)));
    }

    /// T034 (FR-010, data-model *Fit*): each fit is drawn in exactly the width it reports, and a
    /// room a pixel narrower takes the next — the label goes before the hint, and the last is a
    /// pressable `ICON_TARGET` square.
    #[test]
    fn its_three_widths_are_reported_and_each_is_drawn_in_its_own() {
        let f = field();
        let (full, both, alone) = (
            width(&f, Fit::Full),
            width(&f, Fit::GlyphAndHint),
            width(&f, Fit::Glyph),
        );
        assert!(full > both && both > alone, "{full} > {both} > {alone}");
        assert_eq!(alone, anatomy::app_bar::ICON_TARGET);

        let mut m = mounted(field(), full);
        assert_eq!(m.node.bounds().width, full);
        assert!(words(&mut m).iter().any(|w| w == LABEL));

        let mut m = mounted(field(), full - 1.0);
        assert_eq!(m.node.bounds().width, both);
        assert_eq!(m.node.bounds().height, anatomy::app_bar::FIELD_HEIGHT);
        let painted = words(&mut m);
        assert!(!painted.iter().any(|w| w == LABEL), "{painted:?}");
        assert!(painted.iter().any(|w| w == HINT), "{painted:?}");

        let mut m = mounted(field(), both - 1.0);
        let b = m.node.bounds();
        assert_eq!((b.width, b.height), (alone, alone), "an ICON_TARGET square");
        assert_eq!(words(&mut m), vec![glyph()]);
        let centre = Point::new(b.width / 2.0, b.height / 2.0);
        let mut sent = m.press(centre);
        sent.extend(m.send(
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
            iced::mouse::Cursor::Available(centre),
        ));
        assert_eq!(
            sent,
            vec!["jump".to_string()],
            "the glyph alone still presses"
        );
    }

    /// T034 (FR-013): a pressable field is one Tab stop, and Enter and Space each publish it.
    #[test]
    fn enter_and_space_publish_the_press() {
        for key in [Named::Enter, Named::Space] {
            let mut m = mounted(field(), 1000.0);
            assert_eq!(m.drive(Focusables(0)), Some(1));
            m.focus_next();
            let sent = m.send(named_key(key), iced::mouse::Cursor::Unavailable);
            assert_eq!(sent, vec!["jump".to_string()], "{key:?}");
        }
    }

    /// T034 (FR-009, FR-017): without a press the field draws its words at the shared 38%, is no
    /// Tab stop, and a click publishes nothing.
    #[test]
    fn without_a_press_it_is_disabled_and_no_tab_stop() {
        let _ = renderer();
        let f = FieldButton::<String>::new(Icon::Search, LABEL, roles()).hint(HINT);
        let mut m = mounted(f, 1000.0);
        assert_eq!(m.drive(Focusables(0)), Some(0));
        for p in m.painted_text() {
            assert!(
                (p.color.a - style::DISABLED_OPACITY).abs() < 0.01,
                "{:?} is not at the disabled opacity: {:?}",
                p.content,
                p.color
            );
        }
        let at = m.node.bounds().center();
        let mut sent = m.press(at);
        sent.extend(m.send(
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
            iced::mouse::Cursor::Available(at),
        ));
        assert!(sent.is_empty(), "{sent:?}");
    }
}
