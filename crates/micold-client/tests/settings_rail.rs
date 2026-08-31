//! The settings rail: icons on every section, and a collapse that costs nothing (027, FR-026b–d).
//!
//! # What is being protected
//!
//! FR-026c's word is *navigable*. A rail that collapses to icons and stops being a way to reach a
//! section is not a smaller rail — it is a broken one, and it breaks quietly: the surface still
//! renders, the current section still shows, and the only symptom is that four of the application's
//! settings are now unreachable for whoever left the rail closed.
//!
//! So the two halves are held here. That every section can be *told apart* when only its icon is
//! drawn (FR-026b), and that collapsing is view state — remembered while the application is open,
//! and untouched by the form's own Save and Cancel (FR-026d). A collapsed flag drafted with the
//! rest of the form would be reverted by Cancel, which is the one thing a *view* preference must
//! never do: the user closed the rail to read the page, not to edit a setting.

use std::collections::BTreeSet;

use micold_client::app::{Message, State};
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::settings::{SettingsDraft, SettingsSection};

/// FR-026b: every section is identified by an icon, and no two sections share one.
///
/// Asserted against `SettingsSection::icon` rather than against the rendered rail, because the
/// section owns its identity: a second presentation of the same sections — a settings palette, a
/// jump list — must show the same glyph, and one that asked the rail would be free to invent its
/// own.
#[test]
fn every_section_has_an_icon_of_its_own() {
    let icons: BTreeSet<char> = SettingsSection::ALL
        .iter()
        .map(|s| s.icon().glyph())
        .collect();
    assert_eq!(
        icons.len(),
        SettingsSection::ALL.len(),
        "two sections share an icon, so the collapsed rail cannot tell them apart: {:?}",
        SettingsSection::ALL
            .iter()
            .map(|s| (s.label(), s.icon()))
            .collect::<Vec<_>>()
    );
}

/// The rail starts expanded (FR-026d). A first-run user meeting an icons-only rail has to discover
/// what four unlabelled glyphs mean before they can change a setting.
#[test]
fn the_rail_is_expanded_until_the_user_closes_it() {
    assert!(!State::default().settings.settings_rail_collapsed);
}

/// Toggling is a toggle: the same message closes and reopens it.
#[test]
fn the_toggle_closes_and_reopens_the_rail() {
    let mut state = State::default();
    state.update(Message::Settings(SettingsMsg::RailToggled));
    assert!(state.settings.settings_rail_collapsed);
    state.update(Message::Settings(SettingsMsg::RailToggled));
    assert!(!state.settings.settings_rail_collapsed);
}

/// FR-026d, the half that a drafted flag would fail. Save and Cancel both end the form; neither may
/// take the rail's state with them, and reopening Settings must find it as the user left it.
#[test]
fn neither_save_nor_cancel_reopens_a_rail_the_user_closed() {
    for ending in [
        Message::Settings(SettingsMsg::Saved),
        Message::Settings(SettingsMsg::Cancelled),
    ] {
        let mut state = State::default();
        state.update(Message::Settings(SettingsMsg::Opened));
        state.update(Message::Settings(SettingsMsg::RailToggled));
        assert!(state.settings.settings_rail_collapsed, "precondition");

        state.update(ending.clone());
        assert!(
            state.settings.settings_rail_collapsed,
            "{ending:?} reverted a view preference, which is what drafting it would do"
        );

        state.update(Message::Settings(SettingsMsg::Opened));
        assert!(
            state.settings.settings_rail_collapsed,
            "reopening Settings after {ending:?} lost the rail's state"
        );
    }
}

/// And the flag is not the form's: closing the rail must not make the form think it has an edit.
#[test]
fn closing_the_rail_is_not_an_edit_to_the_form() {
    let mut state = State::default();
    state.update(Message::Settings(SettingsMsg::Opened));
    let before = state.settings.settings_draft.clone();
    state.update(Message::Settings(SettingsMsg::RailToggled));
    assert_eq!(
        state.settings.settings_draft, before,
        "the rail's state reached the draft, so Cancel would revert it and Save would write it"
    );
}

/// FR-026c: a collapsed rail still navigates. The message a row emits does not depend on whether
/// the rail is showing labels, so choosing a section works identically in both states.
#[test]
fn every_section_is_still_selectable_while_the_rail_is_collapsed() {
    let mut state = State::default();
    state.update(Message::Settings(SettingsMsg::Opened));
    state.update(Message::Settings(SettingsMsg::RailToggled));

    for section in SettingsSection::ALL {
        state.update(Message::Settings(SettingsMsg::SectionShown(*section)));
        assert_eq!(
            state
                .settings
                .settings_draft
                .as_ref()
                .map(|d: &SettingsDraft| d.section),
            Some(*section),
            "{section:?} could not be reached from the collapsed rail"
        );
    }
}
