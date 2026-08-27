//! Settings as a full-surface view (feature 027, US3 — FR-026).
//!
//! # Why it stopped being a dialog
//!
//! The modal was 420dp wide and held six controls. The sandbox brings a placement, a runtime, an
//! image source with two fields of its own, four credential opt-ins, resource limits and a network
//! posture — and a modal that scrolls is a modal that has outgrown being one. Worse, it was
//! *modal*: reading what a setting means while the thing it configures is behind a scrim is the
//! interaction that makes people close the dialog to check and then forget what they were doing.
//!
//! So Settings takes the main content area, with a rail down the left. The window's chrome — the
//! app bar and the connection banner — stays put, because leaving Settings is a thing the user has
//! to be able to see how to do.
//!
//! # The rail is a component, not a column of buttons
//!
//! [`SectionList`] lives in the component library (FR-026a), which is the difference between a
//! navigation pattern this application has and one this view has. `tests/settings_sections.rs`
//! holds that line: a rail rebuilt privately here would be invisible to every component gate in
//! the crate.

use crate::app::Message;
use crate::features::session::CliAvailability;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::material::{self, Button, Scrollable, Section, SectionList, SurfaceKind};
use crate::ui::settings::{appearance, daemon, environment, terminal};
use iced::widget::{column, row, Space};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The badge on the session-service row while any credential is shared (FR-004c).
///
/// The rail carries it as well as the section, so that "am I sharing anything?" is answerable from
/// wherever the user happens to be — the question is about the application, not about the page.
const SHARING: &str = "Sharing";

/// The whole Settings surface: the rail, the current section, and the two actions.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    env_include_outcome: &'a EnvIncludeOutcome,
    availability: Option<&'a CliAvailability>,
    focused: Option<FieldId>,
    rail_collapsed: bool,
    scheme: ColorScheme,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let sections: Vec<Section<Message>> = SettingsSection::ALL
        .iter()
        .map(|section| {
            let mut row = Section::new(
                section.label(),
                Message::Settings(SettingsMsg::SectionShown(*section)),
            );
            row.icon = Some(section.icon());
            if *section == SettingsSection::Daemon && draft.shares_credentials() {
                row.badge = Some(SHARING.to_string());
            }
            row
        })
        .collect();

    // The drawer, not a bare column: the rail slides in with the view rather than appearing whole,
    // which is the same transition the sidebar makes and the reason it is worth reaching for a
    // component that owns both children.
    //
    // The drawer is never *closed* here, and collapsing the rail (FR-026c) is not the same thing:
    // the drawer's closed child is empty, and an empty rail is a page with no way off it. What
    // collapsing does is narrow the rail while keeping every destination pressable, which is the
    // list's own question — so it is answered by `SectionList`, and the drawer stays open in both
    // states.
    let rail: Element<'a, Message> = material::NavigationDrawer::new(
        SectionList::new(sections, r)
            .selected(draft.section.index())
            .badge_accent(r.error, r.on_error)
            .collapsed(rail_collapsed)
            .toggle(Message::Settings(SettingsMsg::RailToggled)),
        Space::new().width(Length::Fixed(0.0)).height(Length::Fill),
    )
    .open(true)
    .into();

    let page: Element<'a, Message> = match draft.section {
        SettingsSection::Appearance => appearance::view(draft, r),
        SettingsSection::Terminal => terminal::view(draft, focused, r),
        SettingsSection::Environment => {
            environment::view(draft, env_include_outcome, availability, focused, r)
        }
        SettingsSection::Daemon => daemon::view(draft, availability, focused, r),
    };

    // Scrolled, and only the page is: the rail and the actions stay where the user left them while
    // the section under them moves, which is what makes a long section (the service's) usable
    // without losing the way out of it.
    let body = Scrollable::new(page, r)
        .width(Length::Fill)
        .height(Length::Fill);

    // The spacer carries no explicit height, and that is load-bearing: `Row::push` drops any child
    // whose size hint is *void* — `Length::Fixed(0.0)` in either axis — so a spacer declared
    // `.height(Length::Fixed(0.0))` is silently not added, and the two actions sit at the left edge
    // of a form whose fields are right-aligned to nothing.
    let actions = row![
        Space::new().width(Length::Fill),
        Button::outlined("Cancel", r).on_press(Message::Settings(SettingsMsg::Cancelled)),
        Button::filled("Save", r).on_press(Message::Settings(SettingsMsg::Saved)),
    ]
    .spacing(spacing::SM)
    .width(Length::Fill);

    let content = column![body, actions]
        .spacing(spacing::MD)
        .padding(spacing::LG)
        .width(Length::Fill)
        .height(Length::Fill);

    material::Surface::new(
        row![rail, content].width(Length::Fill).height(Length::Fill),
        SurfaceKind::Window,
        r,
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
