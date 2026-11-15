//! `GroupHeader` and `DropMarker` — a user group's header row in the sidebar and the line that
//! marks where a dragged row would land (feature 1786, Constitution Principle VIII, research R7).
//!
//! The header is a [`TreeItem`] so it sits in the sidebar's one [`TreeView`] like every other row,
//! at the one-line row height `features::sidebar::row_heights` gives it: the twisty, the name in
//! [`TypeRole::UserGroupHeader`] (shortened with an ellipsis when long, the full name on the
//! row's tooltip) and how many rows the group holds, or "empty". It names no state (FR-013).

#[cfg(doc)]
use super::TreeView;
use super::{TreeItem, TypeRole};
use iced::widget::container;
use iced::{Element, Length};
use micold_core::tokens::Roles;

/// The words a group with no rows shows in place of its count.
const EMPTY_GROUP: &str = "empty";

/// What a right-click turns into: the press point, in window coordinates, to a message.
type OnRightPress<M> = Box<dyn Fn((u16, u16)) -> M>;

/// A user group's header row. Build with [`GroupHeader::new`], then turn it into a [`TreeItem`]
/// (`.into()`) and put it in a [`TreeView`].
pub struct GroupHeader<M> {
    name: String,
    on_toggle: Option<M>,
    collapsed: bool,
    count: usize,
    roles: Roles,
    key: Option<u64>,
    on_right_press: Option<OnRightPress<M>>,
}

impl<M> GroupHeader<M> {
    /// The header of the group named `name`. It draws a twisty only once it has an
    /// [`on_toggle`](Self::on_toggle): collapsing arrives with feature 1786's US6.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            on_toggle: None,
            collapsed: false,
            count: 0,
            roles: micold_core::tokens::DARK,
            key: None,
            on_right_press: None,
        }
    }

    /// Send `message` when the twisty is pressed (FR-011).
    pub fn on_toggle(mut self, message: M) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Whether the group's rows are hidden under it (FR-011).
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// How many rows the group holds, shown whether or not it is collapsed.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// The colour roles it is drawn in.
    pub fn roles(mut self, roles: Roles) -> Self {
        self.roles = roles;
        self
    }

    /// Emit `f(point)` when the header is right-clicked, `point` being the press in window
    /// coordinates: where the group menu hangs.
    pub fn on_right_press(mut self, f: impl Fn((u16, u16)) -> M + 'static) -> Self {
        self.on_right_press = Some(Box::new(f));
        self
    }

    /// What the row is, for its widget state to follow it when the list moves it.
    pub fn key(mut self, key: u64) -> Self {
        self.key = Some(key);
        self
    }
}

/// The words after a group's name: its count, or [`EMPTY_GROUP`].
fn count_words(count: usize) -> String {
    match count {
        0 => EMPTY_GROUP.to_string(),
        n => n.to_string(),
    }
}

impl<'a, M: 'a> From<GroupHeader<M>> for TreeItem<'a, M> {
    fn from(header: GroupHeader<M>) -> Self {
        let r = header.roles;
        let words = count_words(header.count);
        let mut item = TreeItem::new(0, header.name.clone(), r.on_surface)
            .heading(TypeRole::UserGroupHeader)
            .annotation(words, r.on_surface_variant)
            .row_tooltip(header.name);
        if let Some(on_toggle) = header.on_toggle {
            item = item.expandable(!header.collapsed, on_toggle);
        }
        if let Some(key) = header.key {
            item = item.key(key);
        }
        if let Some(on_right_press) = header.on_right_press {
            item = item.on_right_press(on_right_press);
        }
        item
    }
}

/// The insertion line a drag draws where the row would land (feature 1786, FR-006).
pub struct DropMarker {
    roles: Roles,
}

/// The marker's thickness, in dp.
const DROP_MARKER_HEIGHT: f32 = 2.0;

impl DropMarker {
    /// A marker in `roles`' primary colour.
    pub fn new(roles: Roles) -> Self {
        Self { roles }
    }
}

impl<'a, M: 'a> From<DropMarker> for Element<'a, M> {
    /// The marker, the full width of its row.
    fn from(marker: DropMarker) -> Self {
        let fill = marker.roles.primary;
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(DROP_MARKER_HEIGHT))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(super::style::color(fill))),
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_with_rows_shows_its_count_and_an_empty_one_says_so() {
        assert_eq!(count_words(3), "3");
        assert_eq!(count_words(0), EMPTY_GROUP);
    }

    #[test]
    fn the_header_is_a_one_line_heading_row_with_its_full_name_on_the_tooltip() {
        let long = "A very long group name that cannot fit the sidebar's width at all";
        let item: TreeItem<'static, ()> =
            GroupHeader::new(long).on_toggle(()).collapsed(true).into();
        assert_eq!(item.row_tooltip.as_deref(), Some(long));
        assert_eq!(item.expandable, Some(false));
        assert!(item.tags.is_empty() && item.supporting_text.is_none());
    }
}
