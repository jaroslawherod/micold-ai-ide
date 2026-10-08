//! The Compare view of a run group (feature 483, contracts/parallel-surfaces.md C1–C5): glue only.
//!
//! What a row says — the status, the reason, the counts, whether it can open a diff — is decided
//! by the render-free `features::runs::compare_rows`; this module lays it out from library
//! components. It stands in the terminal pane's place while open, under the Changes view (C1).

use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

use crate::app::{Message, State};
use crate::features::runs::{compare_rows, CompareRow, Msg, RunCounts};
use crate::icons::Icon;
use crate::ui::material::{Button, Tag, Text, Tooltip, TypeRole};

/// Characters of the prompt the header shows before it is cut; the tooltip holds all of it (C1).
pub const PROMPT_CLAMP: usize = 160;

/// The counts of a run as a row reads them: `<files> files +<added> −<removed>` (C2, U+2212).
pub fn counts_text(files: u32, added: u32, removed: u32) -> String {
    format!("{files} files +{added} \u{2212}{removed}")
}

/// `prompt` cut at [`PROMPT_CLAMP`] characters with an ellipsis, on one line.
pub fn clamped_prompt(prompt: &str) -> String {
    let line = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= PROMPT_CLAMP {
        return line;
    }
    let cut: String = line.chars().take(PROMPT_CLAMP).collect();
    format!("{}…", cut.trim_end())
}

/// The Compare view, or nothing when none is open or its group is gone.
pub fn view<'a>(state: &'a State, scheme: ColorScheme) -> Option<Element<'a, Message>> {
    let r = tokens::roles(scheme);
    let (group, rows) = compare_rows(&state.runs, state.active_sessions())?;

    let prompt = Tooltip::new(
        Text::new(clamped_prompt(&group.prompt), TypeRole::Body, r).muted(),
        group.prompt.clone(),
        r,
    )
    .max_lines(12);
    let heading = column![
        Text::new(group.name.clone(), TypeRole::Title, r),
        Text::new(
            format!("Base branch: {}", group.base_branch),
            TypeRole::Body,
            r
        )
        .muted(),
        prompt,
    ]
    .spacing(spacing::XS);
    let header = row![
        container(heading).width(Length::Fill),
        Button::text("Close", r)
            .leading(Icon::Close)
            .on_press(Message::Runs(Msg::CompareClosed)),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);

    let mut list = column![].spacing(spacing::SM);
    for row in &rows {
        list = list.push(run_row(row, r, |run| {
            Message::Runs(Msg::DiffOpened { run })
        }));
    }
    Some(
        container(
            column![header, list]
                .spacing(spacing::LG)
                .height(Length::Fill),
        )
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    )
}

/// One run: `#n`, provider, status, then the counts or the reason, the `uncommitted` tag and
/// **Open diff** (C2–C5).
pub fn run_row<'a, M: Clone + 'a>(
    row_: &CompareRow,
    r: Roles,
    open_diff: impl Fn(u8) -> M,
) -> Element<'a, M> {
    let detail: Element<'a, M> = match (&row_.reason, &row_.counts) {
        (Some(reason), _) => Text::new(reason.clone(), TypeRole::Body, r)
            .tint(r.error)
            .into(),
        (None, RunCounts::Ready(s)) => {
            Text::new(counts_text(s.files, s.added, s.removed), TypeRole::Body, r).into()
        }
        (None, RunCounts::Loading) => Text::new("Reading changes…", TypeRole::Body, r)
            .muted()
            .into(),
        (None, RunCounts::Failed(message)) => Text::new(message.clone(), TypeRole::Body, r)
            .tint(r.error)
            .into(),
        (None, RunCounts::None) => Space::new().into(),
    };
    let uncommitted = matches!(&row_.counts, RunCounts::Ready(s) if s.uncommitted);
    let mut line = row![
        Text::new(format!("#{}", row_.number), TypeRole::Body, r).width(Length::Fixed(40.0)),
        Text::new(
            row_.provider.provider().command().to_string(),
            TypeRole::Body,
            r
        )
        .width(Length::Fixed(120.0)),
        Text::new(row_.status, TypeRole::Body, r).width(Length::Fixed(160.0)),
        container(detail).width(Length::Fill),
    ]
    .spacing(spacing::MD)
    .align_y(Alignment::Center);
    if uncommitted {
        line = line.push(Tag::new("uncommitted", r.tertiary));
    }
    if row_.can_open_diff {
        line = line.push(Button::text("Open diff", r).on_press(open_diff(row_.number)));
    }
    line.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_read_files_then_added_then_removed_with_a_true_minus() {
        assert_eq!(counts_text(4, 120, 30), "4 files +120 \u{2212}30");
    }

    #[test]
    fn a_long_prompt_is_cut_on_one_line_and_a_short_one_is_kept() {
        assert_eq!(clamped_prompt("fix\nthe  bug"), "fix the bug");
        let long = "word ".repeat(100);
        let cut = clamped_prompt(&long);
        assert!(cut.ends_with('…') && cut.chars().count() <= PROMPT_CLAMP + 1);
    }
}
