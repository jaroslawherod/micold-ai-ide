//! `HostPicker` — choose a host from the user's SSH configuration (feature 489, FR-002).
//!
//! A [`Select`](super::Select) over the `Host` aliases of `~/.ssh/config`, worded for that one job:
//! the label says where the names come from and the placeholder says what choosing does (user,
//! port and key then come from the same file). It adds no behaviour of its own, so the open list,
//! the keyboard handling and the state layers are the select's.
//!
//! Builder form: `HostPicker::new(&hosts, &current, |host| Msg::Chosen(host), roles).into()`.

use iced::Element;
use micold_core::tokens::Roles;

use super::Select;

/// A select over the configured SSH hosts.
pub struct HostPicker<'a, M> {
    select: Select<'a, String, M>,
}

impl<'a, M> HostPicker<'a, M> {
    /// A picker over `hosts`, at `current` when that is one of them, emitting `on_choose(host)`.
    pub fn new(
        hosts: &'a [String],
        current: &str,
        on_choose: impl Fn(String) -> M + 'a,
        roles: Roles,
    ) -> Self {
        let selected = hosts.iter().find(|h| h.as_str() == current).cloned();
        let placeholder = if hosts.is_empty() {
            "No hosts in your SSH configuration"
        } else {
            "User, port and key come from your SSH configuration"
        };
        Self {
            select: Select::new(hosts, selected, on_choose, roles)
                .placeholder(placeholder)
                .label("Host from your SSH configuration"),
        }
    }
}

impl<'a, M: Clone + 'a> From<HostPicker<'a, M>> for Element<'a, M> {
    fn from(picker: HostPicker<'a, M>) -> Self {
        picker.select.into()
    }
}
