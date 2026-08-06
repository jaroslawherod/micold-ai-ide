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

use crate::ui::material::style;
use crate::ui::material::TypeRole;
use iced::widget::pick_list;
use iced::{Element, Length};
use micold_core::tokens::Roles;

/// A Material-styled select control. Builder form (Principle VIII):
/// `Select::new(options, selected, on_selected, roles).placeholder("...").into()`.
pub struct Select<'a, T, M> {
    options: &'a [T],
    selected: Option<T>,
    on_selected: Box<dyn Fn(T) -> M + 'a>,
    placeholder: String,
    roles: Roles,
    label: Option<String>,
    supporting: Option<String>,
    error: Option<String>,
    active: bool,
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
            label: None,
            supporting: None,
            error: None,
            active: false,
        }
    }

    /// The control's name, rendered inside its container above the value (FR-031a).
    ///
    /// §7.7 moves the free-standing label that sat above the select into the control's own
    /// container, which is what makes it look like every other field rather than like a button
    /// with a caption.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    /// The validation failure, if any — replaces the supporting text and recolours the chrome.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Whether the list is **open**, which thickens the active indicator (FR-043a).
    ///
    /// Open rather than focused, because `pick_list` has no focused status to report — that is
    /// accepted fidelity gap #3. It does not tell a parent when it opens either, so this is supplied
    /// by a caller that tracks it rather than observed. Left unset the indicator stays at rest,
    /// which is the honest answer for a caller that does not know.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
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
        // No padding of its own: `FormField` pads the container to §7.7's 16dp, and a select that
        // padded itself as well would sit inset from every text field beside it.
        let control = pick_list(s.options, s.selected, s.on_selected)
            .placeholder(s.placeholder)
            .width(Length::Fill)
            // Zero, for the same reason as the text field: the wrapper owns the padding, and the
            // stack's default would inset the value from the label above it.
            .padding(0)
            // A `pick_list` takes a size and a font separately rather than a role, so the role is
            // named here and taken apart — the call site still never states a number.
            .text_size(TypeRole::Body.size())
            .font(TypeRole::Body.font())
            .style(style::select_field(r))
            .menu_style(style::select_menu(r));

        let mut field = super::FormField::new(control, r).active(s.active);
        if let Some(label) = s.label {
            field = field.label(label);
        }
        if let Some(supporting) = s.supporting {
            field = field.supporting(supporting);
        }
        field.error(s.error).into()
    }
}
