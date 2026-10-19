//! `FieldGroup` — a labelled group of controls inside a form section (feature 890, FR-008).
//!
//! A run of checkboxes, or a multi-line editor, has no label of its own the way a text field does;
//! this gives it one in a single style, with an optional note directly under the label, and lines
//! the children up at the label's left edge. Builder form (Principle VIII):
//! `FieldGroup::new(label, roles).note(text).push(child).into()`.

use iced::widget::column;
use iced::{Element, Length};
use micold_core::tokens::{spacing, Roles};

use super::{Text, TypeRole};

/// A group label, an optional note and the children, one column at one left edge.
pub struct FieldGroup<'a, M> {
    label: String,
    note: Option<String>,
    children: Vec<Element<'a, M>>,
    roles: Roles,
}

impl<'a, M: 'a> FieldGroup<'a, M> {
    /// An empty group named `label`, themed by `roles`.
    pub fn new(label: impl Into<String>, roles: Roles) -> Self {
        Self {
            label: label.into(),
            note: None,
            children: Vec::new(),
            roles,
        }
    }

    /// An explanation of the whole group, directly under the label.
    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    /// Add one child below the others.
    pub fn push(mut self, child: impl Into<Element<'a, M>>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Add several children below the others.
    pub fn extend(mut self, children: impl IntoIterator<Item = Element<'a, M>>) -> Self {
        self.children.extend(children);
        self
    }
}

impl<'a, M: 'a> From<FieldGroup<'a, M>> for Element<'a, M> {
    fn from(group: FieldGroup<'a, M>) -> Self {
        let roles = group.roles;
        let mut head = column![Text::new(group.label, TypeRole::Label, roles).muted()]
            .spacing(spacing::XS)
            .width(Length::Fill);
        if let Some(note) = group.note {
            head = head.push(Text::new(note, TypeRole::Caption, roles).muted());
        }
        let children = column(group.children)
            .spacing(spacing::SM)
            .width(Length::Fill);
        column![head, children]
            .spacing(spacing::SM)
            .width(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::style;
    use super::super::test_support::{dark_roles, roles, Mounted};
    use super::*;

    fn group<'a>(note: Option<&str>, r: Roles) -> FieldGroup<'a, String> {
        let mut g = FieldGroup::new("AI tools", r)
            .push(Text::new("first", TypeRole::Body, r))
            .push(Text::new("second", TypeRole::Body, r))
            .push(Text::new("third", TypeRole::Body, r));
        if let Some(n) = note {
            g = g.note(n);
        }
        g
    }

    fn at(painted: &[super::super::test_support::Painted], text: &str) -> iced::Rectangle {
        painted
            .iter()
            .find(|p| p.content == text)
            .unwrap_or_else(|| panic!("{text:?} not painted: {painted:?}"))
            .bounds
    }

    /// The label is a muted `Label`, the note a muted `Caption` directly under it, the children
    /// follow in order — in both themes (data-model §FieldGroup).
    #[test]
    fn it_paints_label_then_note_then_children_in_their_roles() {
        for r in [roles(), dark_roles()] {
            let mut m = Mounted::new(group(Some("Installed at start."), r));
            let painted = m.painted_text();
            let label = painted.iter().find(|p| p.content == "AI tools").unwrap();
            assert_eq!(label.size, TypeRole::Label.size());
            assert_eq!(label.font, TypeRole::Label.font());
            assert_eq!(label.color, style::color(r.on_surface_variant));
            let note = painted
                .iter()
                .find(|p| p.content == "Installed at start.")
                .unwrap();
            assert_eq!(note.size, TypeRole::Caption.size());
            assert_eq!(note.color, style::color(r.on_surface_variant));
            let (l, n) = (label.bounds, note.bounds);
            let first = at(&painted, "first");
            assert!(n.y >= l.y + l.height - 0.5, "note under label");
            assert!(first.y >= n.y + n.height - 0.5, "children after the note");
            assert!(at(&painted, "second").y > first.y);
        }
    }

    /// Label, note and children share one left edge.
    #[test]
    fn label_note_and_children_share_a_left_edge() {
        let r = roles();
        let mut m = Mounted::new(group(Some("Installed at start."), r));
        let painted = m.painted_text();
        let edge = at(&painted, "AI tools").x;
        for t in ["Installed at start.", "first", "second", "third"] {
            assert!((at(&painted, t).x - edge).abs() < 0.5, "{t} off the edge");
        }
    }

    /// Children are `spacing::SM` apart; without a note the first follows the label.
    #[test]
    fn children_are_one_gap_apart_and_a_missing_note_leaves_no_hole() {
        let r = roles();
        let mut with = Mounted::new(group(Some("Installed at start."), r));
        let mut without = Mounted::new(group(None, r));
        let (w, wo) = (with.painted_text(), without.painted_text());
        let step = |p: &[_], a, b| at(p, b).y - at(p, a).y;
        assert!(
            (step(&w, "first", "second") - step(&wo, "first", "second")).abs() < 0.5,
            "gap between children is the same"
        );
        let body = TypeRole::Body.line_height_dp();
        assert!((step(&wo, "first", "second") - (body + spacing::SM)).abs() < 1.0);
        assert!(
            at(&wo, "first").y - at(&wo, "AI tools").y < at(&w, "first").y - at(&w, "AI tools").y,
            "no note, no hole"
        );
        assert!(wo.iter().all(|p| p.content != "Installed at start."));
    }
}
