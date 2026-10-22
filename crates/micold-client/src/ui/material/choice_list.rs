//! `ChoiceList` — a list of choices, one of them or several, each a label with an optional
//! description, and an optional **Other** row with a field for a free answer (feature 942, T058,
//! research R11, FR-008).
//!
//! The library had single toggles (`Checkbox`, `ToggleChip`) and no group of choices: a question
//! Claude asks needs one, with a radio mark when one option is the answer and a checkbox mark when
//! several may be. Each row is one press target and one keyboard stop that answers Space and
//! Enter, with the menu rows' hover and press layers; the chosen state is the mark, never colour
//! alone. The list holds no state: the caller supplies what is chosen and is told what was pressed.
//!
//! Builder form:
//! `ChoiceList::new(roles).multiple(true).option("Postgres", "The main store", chosen)
//!     .on_choose(Msg::Chose).other(&draft.other, other_chosen).on_other(Msg::Other)
//!     .on_other_input(Msg::OtherEdited).into()`.

use iced::keyboard::key::Named;
use iced::widget::{button, column, container, row, Space};
use iced::{Alignment, Element, Length, Padding};
use micold_core::tokens::{shape, spacing, Roles};

use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::{style, Glyph, Text, TextField, TypeRole};
use crate::icons::Icon;
use iced::widget::text::Wrapping;

/// What the free-answer row is labelled.
pub const OTHER: &str = "Other";

/// What the free-answer field prompts with.
pub const OTHER_PLACEHOLDER: &str = "Your answer";

/// A single-choice ring's side.
const RING: f32 = 18.0;

/// The dot inside a chosen ring.
const DOT: f32 = 8.0;

/// One option.
struct Choice {
    label: String,
    description: String,
    chosen: bool,
}

/// The free-answer row.
struct Other<'a> {
    value: &'a str,
    chosen: bool,
}

/// A list of choices.
pub struct ChoiceList<'a, M> {
    roles: Roles,
    multiple: bool,
    options: Vec<Choice>,
    on_choose: Option<Box<dyn Fn(usize) -> M + 'a>>,
    other: Option<Other<'a>>,
    on_other: Option<M>,
    on_other_input: Option<Box<dyn Fn(String) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> ChoiceList<'a, M> {
    /// An empty single-choice list themed by `roles`; without [`Self::on_choose`] its rows are
    /// drawn but cannot be pressed.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            multiple: false,
            options: Vec::new(),
            on_choose: None,
            other: None,
            on_other: None,
            on_other_input: None,
        }
    }

    /// Whether several options may be chosen: checkbox marks instead of radio marks.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// One more option: its `label`, a `description` under it (none when empty), and whether it
    /// is `chosen`.
    pub fn option(
        mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        chosen: bool,
    ) -> Self {
        self.options.push(Choice {
            label: label.into(),
            description: description.into(),
            chosen,
        });
        self
    }

    /// The message a press of option `i` (by position) sends.
    pub fn on_choose(mut self, f: impl Fn(usize) -> M + 'a) -> Self {
        self.on_choose = Some(Box::new(f));
        self
    }

    /// The **Other** row after the options, `chosen` or not; once chosen, a field under it shows
    /// `value`.
    pub fn other(mut self, value: &'a str, chosen: bool) -> Self {
        self.other = Some(Other { value, chosen });
        self
    }

    /// The message a press of the Other row sends.
    pub fn on_other(mut self, message: M) -> Self {
        self.on_other = Some(message);
        self
    }

    /// The message the Other field sends as it is typed in; without it the field is disabled.
    pub fn on_other_input(mut self, f: impl Fn(String) -> M + 'a) -> Self {
        self.on_other_input = Some(Box::new(f));
        self
    }
}

/// The mark of a row: a ring with a dot when chosen (one choice), a checkbox glyph (several).
fn mark<'a, M: 'a>(multiple: bool, chosen: bool, r: Roles) -> Element<'a, M> {
    if multiple {
        let icon = if chosen {
            Icon::TaskDone
        } else {
            Icon::TaskOpen
        };
        let tint = if chosen {
            r.primary
        } else {
            r.on_surface_variant
        };
        return Glyph::new(icon, TypeRole::Body, r).tint(tint).into();
    }
    let dot: Element<'a, M> = if chosen {
        container(Space::new())
            .width(DOT)
            .height(DOT)
            .style(style::choice_dot(r))
            .into()
    } else {
        Space::new().into()
    };
    container(dot)
        .width(RING)
        .height(RING)
        .center_x(RING)
        .center_y(RING)
        .style(style::choice_ring(r, chosen))
        .into()
}

/// One row: its mark, its label and description, pressed with the pointer, Space or Enter.
fn choice_row<'a, M: Clone + 'a>(
    marked: Element<'a, M>,
    label: String,
    description: &str,
    message: Option<M>,
    r: Roles,
) -> Element<'a, M> {
    let mut words = column![Text::new(label, TypeRole::Body, r)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill)]
    .spacing(spacing::XS)
    .width(Length::Fill);
    if !description.is_empty() {
        words = words.push(
            Text::new(description.to_owned(), TypeRole::Caption, r)
                .muted()
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        );
    }
    let content = row![container(marked).padding(Padding::ZERO.top(2.0)), words]
        .spacing(spacing::SM)
        .align_y(Alignment::Start);
    let pressable = button(content)
        .padding([spacing::XS, spacing::SM])
        .width(Length::Fill)
        .style(style::menu_row(r, false, false))
        .on_press_maybe(message.clone());
    let mut focus = TakesTheKeyboard::new(pressable, message.is_some()).indicator(Indicator {
        outline: r.secondary,
        layer: r.on_surface_variant,
        radius: shape::SMALL,
    });
    if let Some(message) = message {
        focus = focus
            .key(Named::Space, message.clone())
            .key(Named::Enter, message);
    }
    focus.into()
}

impl<'a, M: Clone + 'a> From<ChoiceList<'a, M>> for Element<'a, M> {
    fn from(list: ChoiceList<'a, M>) -> Self {
        let r = list.roles;
        let multiple = list.multiple;
        let mut rows = column![].spacing(spacing::XS).width(Length::Fill);
        for (i, choice) in list.options.into_iter().enumerate() {
            let message = list.on_choose.as_ref().map(|f| f(i));
            rows = rows.push(choice_row(
                mark(multiple, choice.chosen, r),
                choice.label,
                &choice.description,
                message,
                r,
            ));
        }
        if let Some(other) = list.other {
            rows = rows.push(choice_row(
                mark(multiple, other.chosen, r),
                OTHER.to_owned(),
                "",
                list.on_other,
                r,
            ));
            if other.chosen {
                let mut field = TextField::new(OTHER_PLACEHOLDER, other.value, r);
                if let Some(on_input) = list.on_other_input {
                    field = field.on_input(on_input);
                }
                rows = rows.push(container(field).padding(Padding {
                    top: 0.0,
                    bottom: spacing::XS,
                    left: spacing::SM + RING + spacing::SM,
                    right: spacing::SM,
                }));
            }
        }
        rows.into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, Focusables, Mounted};
    use super::*;
    use iced::advanced::mouse;

    fn list(r: Roles, multiple: bool) -> ChoiceList<'static, String> {
        ChoiceList::new(r)
            .multiple(multiple)
            .option("Postgres", "The main store", true)
            .option("SQLite", "", false)
            .on_choose(|i| format!("choose {i}"))
            .other("", false)
            .on_other("other".to_string())
    }

    /// Every label, description and the Other row are drawn, in both schemes.
    #[test]
    fn every_label_and_description_is_drawn() {
        for r in [roles(), dark_roles()] {
            let painted = Mounted::new(list(r, false)).painted_text();
            for text in ["Postgres", "The main store", "SQLite", OTHER] {
                assert!(
                    painted.iter().any(|p| p.content == text),
                    "`{text}`: {painted:?}"
                );
            }
        }
    }

    /// A several-choice list marks its rows with checkbox glyphs, chosen and not.
    #[test]
    fn a_multiple_choice_list_marks_rows_as_checkboxes() {
        let painted = Mounted::new(list(roles(), true)).painted_text();
        for icon in [Icon::TaskDone, Icon::TaskOpen] {
            assert!(
                painted
                    .iter()
                    .any(|p| p.content == icon.glyph().to_string()),
                "{icon:?}: {painted:?}"
            );
        }
    }

    /// Each row is one keyboard stop, and Space or Enter presses it.
    #[test]
    fn each_row_is_a_keyboard_stop_that_space_and_enter_press() {
        let mut m = Mounted::new(list(roles(), false));
        assert_eq!(m.drive(Focusables(0)), Some(3));
        m.focus_next();
        assert_eq!(
            m.send(named_key(Named::Space), mouse::Cursor::Unavailable),
            vec!["choose 0".to_string()]
        );
        m.focus_next();
        m.focus_next();
        assert_eq!(
            m.send(named_key(Named::Enter), mouse::Cursor::Unavailable),
            vec!["other".to_string()]
        );
    }

    /// Without `on_choose` the rows cannot be pressed and are no keyboard stops.
    #[test]
    fn without_on_choose_no_row_is_a_stop() {
        let mut m = Mounted::new(ChoiceList::<String>::new(roles()).option("A", "", false));
        assert_eq!(m.drive(Focusables(0)), Some(0));
    }

    /// Other chosen shows its field with what was typed; not chosen, no field.
    #[test]
    fn the_other_field_shows_once_other_is_chosen() {
        let chosen = ChoiceList::<String>::new(roles())
            .option("A", "", false)
            .other("in the cloud", true)
            .on_other_input(|t| t);
        let painted = Mounted::new(chosen).painted_text();
        assert!(
            painted.iter().any(|p| p.content == "in the cloud"),
            "{painted:?}"
        );
        let unchosen = ChoiceList::<String>::new(roles()).other("in the cloud", false);
        let painted = Mounted::new(unchosen).painted_text();
        assert!(
            painted.iter().all(|p| p.content != "in the cloud"),
            "{painted:?}"
        );
    }
}
