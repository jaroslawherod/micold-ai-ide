//! `CommandPalette` — the list of commands a `/` opens over a composer (feature 943, T056, FR-007,
//! FR-008, FR-010).
//!
//! It sits in the composer's completion slot, above the text area, in the flow of the composer:
//! the menu surface with a note on top (what the list is, when it is not the session's own), each
//! group's header, and the commands as the pickers' result rows ([`picker`](super::picker)'s row,
//! with the typed name emphasised and a second line). The highlighted row is the one Tab or Enter
//! takes; a click takes any. Nothing matching, it says so in place of the rows.
//!
//! It decides nothing: which commands, in which order, under which headers, and what each row's
//! second line says (a terminal-only command's "opens in terminal" first, so a cut line never
//! takes it) are the caller's (`features::command_palette`).
//!
//! Builder form: `CommandPalette::new(roles).note(…).group("Skills")
//!     .command(TypeaheadRow::new("/speckit-plan", spans).details(…, vec![]))
//!     .highlighted(Some(0)).on_pick(Msg::Picked).into()`.

use iced::widget::{column, container, Column};
use iced::{Element, Length};
use micold_core::tokens::{density, spacing, Roles};

use super::picker::{row_element, Row, MAX_ROWS_BEFORE_SCROLL, PICKER_HIGHLIGHT};
use super::{Scrollable, Text, TypeRole};

/// One line of the list.
enum Line {
    Group(String),
    Command(Row),
}

/// The command palette.
pub struct CommandPalette<'a, M> {
    roles: Roles,
    lines: Vec<Line>,
    highlighted: Option<usize>,
    note: Option<String>,
    empty: Option<String>,
    on_pick: Option<Box<dyn Fn(usize) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> CommandPalette<'a, M> {
    /// An empty palette themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            lines: Vec::new(),
            highlighted: None,
            note: None,
            empty: None,
            on_pick: None,
        }
    }

    /// A group's header, over the commands added after it.
    pub fn group(mut self, title: impl Into<String>) -> Self {
        self.lines.push(Line::Group(title.into()));
        self
    }

    /// One more command row. Rows are counted from 0 in the order added, headers aside.
    pub fn command(mut self, row: Row) -> Self {
        self.lines.push(Line::Command(row.single_line()));
        self
    }

    /// The row Tab or Enter takes.
    pub fn highlighted(mut self, row: Option<usize>) -> Self {
        self.highlighted = row;
        self
    }

    /// A muted line over the list: what it is, when that needs saying.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// What stands in place of the rows when there are none.
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty = Some(message.into());
        self
    }

    /// A click on row `n` sends `f(n)`.
    pub fn on_pick(mut self, f: impl Fn(usize) -> M + 'a) -> Self {
        self.on_pick = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<CommandPalette<'a, M>> for Element<'a, M> {
    fn from(p: CommandPalette<'a, M>) -> Self {
        let r = p.roles;
        let caption = |text: String| {
            Text::new(text, TypeRole::Caption, r)
                .muted()
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .width(Length::Fill)
        };
        let mut parts: Column<'a, M> = column![].spacing(spacing::XS).width(Length::Fill);
        if let Some(note) = p.note {
            parts = parts.push(container(caption(note)).padding([spacing::XS, spacing::SM]));
        }
        let mut list: Column<'a, M> = column![].width(Length::Fill);
        let mut index = 0;
        for line in p.lines {
            match line {
                Line::Group(title) => {
                    list = list.push(
                        container(
                            Text::new(title, TypeRole::Label, r)
                                .muted()
                                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                                .width(Length::Fill),
                        )
                        .padding([spacing::XS, spacing::SM]),
                    );
                }
                Line::Command(row) => {
                    let highlighted = p.highlighted == Some(index);
                    let press = p.on_pick.as_ref().map(|f| f(index));
                    let drawn = row_element(row, highlighted, false, press, r);
                    list = list.push(if highlighted {
                        // The keyboard's row carries the pickers' highlight `Id`, so the shell
                        // scrolls it into view as the arrows move it (`picker_highlight_into_view`).
                        container(drawn).id(PICKER_HIGHLIGHT.clone()).into()
                    } else {
                        drawn
                    });
                    index += 1;
                }
            }
        }
        if index == 0 {
            if let Some(empty) = p.empty {
                parts = parts.push(container(caption(empty)).padding([spacing::XS, spacing::SM]));
            }
        } else {
            parts = parts.push(
                container(Scrollable::new(list, r).height(Length::Shrink))
                    .max_height(density::MENU_ITEM_BASE * MAX_ROWS_BEFORE_SCROLL),
            );
        }
        super::menu_panel(parts, Length::Fill, r, true, spacing::XS)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, roles, Mounted};
    use super::*;

    fn palette(r: Roles) -> CommandPalette<'static, String> {
        CommandPalette::new(r)
            .note("The commands Claude Code always has")
            .group("Skills")
            .command(
                Row::new("/speckit-plan", std::iter::once(1..3).collect())
                    .details("Plan a feature", vec![]),
            )
            .group("Built-in commands")
            .command(
                Row::new("/mcp", vec![]).details("opens in terminal · Manage MCP servers", vec![]),
            )
            .highlighted(Some(1))
            .on_pick(|n| format!("pick {n}"))
    }

    fn words(m: &mut Mounted<'_>) -> Vec<String> {
        m.painted_text().into_iter().map(|p| p.content).collect()
    }

    /// The note, every header, every row and its second line are drawn, in both schemes.
    #[test]
    fn the_note_headers_rows_and_details_are_drawn() {
        for r in [roles(), dark_roles()] {
            let drawn = words(&mut Mounted::new(palette(r)));
            let all = drawn.join("|");
            for part in [
                "The commands Claude Code always has",
                "Skills",
                "Built-in commands",
                "Plan a feature",
                "opens in terminal · Manage MCP servers",
            ] {
                assert!(all.contains(part), "`{part}`: {drawn:?}");
            }
            assert!(all.contains("/mcp"), "{drawn:?}");
        }
    }

    /// With no rows, the empty message stands in their place.
    #[test]
    fn no_rows_draws_the_empty_message() {
        let p: CommandPalette<'static, String> = CommandPalette::new(roles())
            .empty_message("No command matches")
            .on_pick(|n| n.to_string());
        let drawn = words(&mut Mounted::new(p));
        assert!(drawn.iter().any(|d| d == "No command matches"), "{drawn:?}");
    }
}
