//! `Select` — a Material-styled wrapper around iced's built-in `pick_list` widget
//! (Constitution Principle VIII, feature 013).
//!
//! Supersedes this module's original hand-rolled `SelectTrigger`/`SelectOverlay`, which revealed
//! its item list *inline* (pushing later fields down) because the trigger lives inside `Modal`'s
//! fixed-width, content-sized dialog box, where the hand-rolled panel — built from the same
//! `stack!`-based mechanism `MenuOverlay`/`ProjectSwitcherOverlay` use — had no `Length::Fill`
//! window to float against. `pick_list` doesn't have that problem: it implements `Widget::
//! overlay()` directly, so its dropdown is positioned from the trigger's on-screen bounds by
//! iced's own overlay system, independent of the parent layout's size constraints — the same
//! mechanism every `pick_list`/`combo_box`/tooltip in any iced app already relies on. It also
//! seeds the open menu's highlighted row from the current value, so reopening the list visibly
//! marks the current selection (FR-003) with no bespoke state of this app's own.

use micold_core::tokens::{spacing, type_scale, Roles};
use crate::ui::style;
use iced::widget::pick_list;
use iced::{Element, Length};

/// A Material-styled select control. Builder form (Principle VIII):
/// `Select::new(options, selected, on_selected, roles).placeholder("...").into()`.
pub struct Select<'a, T, M> {
    options: &'a [T],
    selected: Option<T>,
    on_selected: Box<dyn Fn(T) -> M + 'a>,
    placeholder: String,
    roles: Roles,
}

impl<'a, T, M> Select<'a, T, M>
where
    T: Clone + ToString + PartialEq + 'a,
{
    /// A select over `options`, currently at `selected` (or unset), emitting `on_selected(t)`
    /// when `t` is picked, themed by `roles`.
    pub fn new(
        options: &'a [T],
        selected: Option<T>,
        on_selected: impl Fn(T) -> M + 'a,
        roles: Roles,
    ) -> Self {
        Self {
            options,
            selected,
            on_selected: Box::new(on_selected),
            placeholder: "Select…".to_string(),
            roles,
        }
    }

    /// Text shown when nothing is selected (default: "Select…").
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

impl<'a, T, M> From<Select<'a, T, M>> for Element<'a, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{
    fn from(s: Select<'a, T, M>) -> Self {
        let r = s.roles;
        pick_list(s.options, s.selected, s.on_selected)
            .placeholder(s.placeholder)
            .width(Length::Fill)
            .padding(spacing::SM)
            .text_size(type_scale::BODY)
            .style(style::select_field(r))
            .menu_style(style::select_menu(r))
            .into()
    }
}
