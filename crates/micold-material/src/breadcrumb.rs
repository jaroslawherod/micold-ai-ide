//! `Breadcrumb` — a `project / place` path at the leading end of a bar (feature 1127, research R4,
//! contract C2, C3).
//!
//! Two segments and the separator between them. The **project** segment is a pressable text
//! [`Button`] with a trailing drop-down glyph, saying it opens a list, and the unread mark of
//! feature 039 after it. The **place** segment is a plain label, never a control (FR-006): it says
//! where the user is, and pressing it would only move them somewhere they already are.
//!
//! # It gives way place first
//!
//! The breadcrumb is a hugging give-way [`Lane`]: when its bar narrows it, the place shortens with
//! an ellipsis first (`Shorten(0)`), then the project (`Shorten(1)`), each to a floor of one
//! character and `…`; the separator, the glyph and the mark never shorten (FR-007, data-model
//! *Give-way order*). A segment that is shortened shows its full text in its tooltip, which is
//! decided where the shortening is: at layout, by [`FirstThatFits`].
//!
//! Builder form (Principle VIII):
//! `Breadcrumb::new(project, roles).tooltip(t).on_press(m).trailing_mark(n, t).place(p, full).open(b).into()`.

use crate::give_way::{FirstThatFits, Floor, GiveWay, Group, Lane, Need, Slot};
use crate::icons::Icon;
use crate::shaped_text::width_of;
use crate::style::Host;
use crate::text::{Text, TypeRole};
use crate::{Button, ButtonVariant, Ellipsized, Tooltip};
use iced::widget::container;
use iced::{Element, Padding};
use micold_core::tokens::{anatomy, spacing, Roles};

/// The role both segments and the separator are set at: the label role a button's text takes, so
/// the place reads at the project's size and weight (contract T3).
const ROLE: TypeRole = TypeRole::Action;

/// What stands between the segments.
const SEPARATOR: &str = "/";

/// A `project / place` path. Built step by step, then `.into()`.
pub struct Breadcrumb<M> {
    project: String,
    roles: Roles,
    tooltip: Option<String>,
    on_press: Option<M>,
    mark: Option<(usize, String)>,
    place: Option<(String, Option<String>)>,
    open: bool,
}

impl<M: Clone> Breadcrumb<M> {
    /// A breadcrumb whose project segment reads `project`, themed by `roles`, with no place yet.
    pub fn new(project: impl Into<String>, roles: Roles) -> Self {
        Self {
            project: project.into(),
            roles,
            tooltip: None,
            on_press: None,
            mark: None,
            place: None,
            open: false,
        }
    }

    /// The project segment's tooltip: what pressing it does (`Switch project`, contract C2). When
    /// the segment is shortened the project's full name follows on a second line.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// What pressing the project segment sends. Without it the segment is drawn disabled.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The unread mark after the project segment's glyph, as [`Button::trailing_mark`] draws it:
    /// none for a count of zero.
    pub fn trailing_mark(mut self, unread: usize, tooltip: impl Into<String>) -> Self {
        self.mark = (unread > 0).then(|| (unread, tooltip.into()));
        self
    }

    /// The place segment after a `/`: `text`, and `full` when the full text differs from it (a
    /// short branch's full branch). Its tooltip is `full`, or `text` once the segment is shortened.
    /// Without a place neither the separator nor the segment is drawn.
    pub fn place(mut self, text: impl Into<String>, full: Option<String>) -> Self {
        self.place = Some((text.into(), full));
        self
    }

    /// Draw the project segment selected, while the list it opens is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// What the project segment stands on: the bar's surface, which asks for `on_surface` content
/// rather than a text button's accent (contract T3). The label, the glyph, the mark's number, the
/// state layer and the ripple all take it together, as they do on any host that imposes its
/// foreground. `pub` for the contrast gate, which reads the composition from what the view calls.
pub fn host(roles: Roles) -> Host {
    Host::accent(roles.surface, roles.on_surface)
}

/// What a segment shortened as far as it goes keeps: its first character and `…`.
fn floor_text(text: &str) -> String {
    let mut out: String = text.chars().take(1).collect();
    out.push('…');
    out
}

/// A segment's slot in the breadcrumb's lane: it shortens at `rank`, down to [`floor_text`].
fn shortens(rank: u8, text: &str) -> Slot {
    Slot {
        give_way: GiveWay::Shorten(rank),
        group: Group::Leading,
        floor: Floor::Saving(width_of(text, ROLE) - width_of(&floor_text(text), ROLE)),
    }
}

impl<'a, M: Clone + 'a> From<Breadcrumb<M>> for Element<'a, M> {
    fn from(b: Breadcrumb<M>) -> Self {
        let r = b.roles;
        let host = host(r);
        let project = |tooltip: Option<String>| -> Element<'a, M> {
            let label = Ellipsized::at_role(
                b.project.clone(),
                ROLE,
                ButtonVariant::Text.content(r, Some(host)),
            )
            .shrink();
            let mut button = Button::with_content(label, ButtonVariant::Text, r)
                .on_host(host)
                .trailing(Icon::SelectChevron)
                .selected(b.open)
                .on_press_maybe(b.on_press.clone());
            if let Some(tooltip) = tooltip {
                button = button.tooltip(tooltip);
            }
            if let Some((unread, words)) = &b.mark {
                button = button.trailing_mark(*unread, words.clone());
            }
            button.into()
        };
        let shortened_tooltip = Some(match &b.tooltip {
            Some(own) => format!("{own}\n{}", b.project),
            None => b.project.clone(),
        });
        let mut lane = Lane::hugging(spacing::XS).push(
            FirstThatFits::new()
                .or(project(b.tooltip.clone()), Need::Natural)
                .or(project(shortened_tooltip), Need::Natural)
                .into(),
            shortens(1, &b.project),
        );

        if let Some((text, full)) = b.place {
            // The separator keeps the button's own inset on its far side, so it stands as far from
            // the place as from the project's glyph.
            let separator = container(Text::new(SEPARATOR, ROLE, r).tint(r.on_surface_variant))
                .padding(Padding {
                    right: anatomy::button::PADDING_TEXT,
                    ..Padding::ZERO
                });
            let kept = Slot {
                give_way: GiveWay::Keep,
                group: Group::Leading,
                floor: Floor::Natural,
            };
            let label = |tooltip: Option<String>| -> Element<'a, M> {
                let label = Ellipsized::at_role(text.clone(), ROLE, r.on_surface_variant).shrink();
                match tooltip {
                    Some(tooltip) => Tooltip::new(label, tooltip, r).into(),
                    None => label.into(),
                }
            };
            let slot = shortens(0, &text);
            let whole = label(full.clone());
            let cut = label(Some(full.unwrap_or_else(|| text.clone())));
            lane = lane.push(separator.into(), kept).push(
                FirstThatFits::new()
                    .or(whole, Need::Natural)
                    .or(cut, Need::Natural)
                    .into(),
                slot,
            );
        }
        lane.into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, leaves, roles, Mounted};
    use super::*;
    use iced::advanced::mouse;
    use iced::{Event, Point, Rectangle, Size};

    const ROOM: Size = Size::new(800.0, 100.0);
    const PROJECT: &str = "micold-ai-ide";
    const LONG_PLACE: &str = "feat/1127-a-branch-name-sixty-characters-long-for-the-bar-x";

    fn crumb() -> Breadcrumb<String> {
        Breadcrumb::new(PROJECT, roles())
            .tooltip("Switch project")
            .on_press("toggle".to_string())
    }

    /// The painted words, in drawing order.
    fn words(m: &mut Mounted<'_>) -> Vec<String> {
        m.painted_text().into_iter().map(|p| p.content).collect()
    }

    /// The breadcrumb's lane children: project, then the separator and the place when there is one.
    fn segments(m: &Mounted<'_>) -> Vec<Rectangle> {
        m.node.children().iter().map(|c| c.bounds()).collect()
    }

    /// The tooltip floating over `at`, joined.
    fn tooltip_at(m: &mut Mounted<'_>, at: Point) -> String {
        m.floated_text(at).join("\n")
    }

    /// T024 (FR-004): the project segment is a pressable button reading the project, with the
    /// drop-down glyph after it; without a place there is no separator and no place.
    #[test]
    fn the_project_segment_alone_is_a_pressable_button_with_its_glyph() {
        let mut m = Mounted::new_in(crumb(), ROOM);
        assert_eq!(segments(&m).len(), 1, "no separator, no place");
        let painted = words(&mut m);
        let glyph = Icon::SelectChevron.glyph().to_string();
        assert!(painted.contains(&PROJECT.to_string()), "{painted:?}");
        assert!(painted.contains(&glyph), "the drop-down glyph: {painted:?}");
        assert!(!painted.iter().any(|w| w == SEPARATOR), "{painted:?}");
        let centre = segments(&m)[0].center();
        let sent = m.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            mouse::Cursor::Available(centre),
        );
        let sent = [
            sent,
            m.send(
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                mouse::Cursor::Available(centre),
            ),
        ]
        .concat();
        assert!(sent.contains(&"toggle".to_string()), "{sent:?}");
    }

    /// T024 (FR-006, C3): a place adds `/` and a label after the project, and pressing the place
    /// sends nothing.
    #[test]
    fn a_place_adds_the_separator_and_a_label_that_is_not_a_control() {
        let mut m = Mounted::new_in(crumb().place("fix/1117", None), ROOM);
        let s = segments(&m);
        assert_eq!(s.len(), 3, "project, separator, place");
        assert!(s[0].x + s[0].width <= s[1].x && s[1].x + s[1].width <= s[2].x);
        let painted = words(&mut m);
        assert!(painted.contains(&SEPARATOR.to_string()), "{painted:?}");
        assert!(painted.contains(&"fix/1117".to_string()), "{painted:?}");
        let at = mouse::Cursor::Available(s[2].center());
        let sent = [
            m.send(
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                at,
            ),
            m.send(
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                at,
            ),
        ]
        .concat();
        assert!(sent.is_empty(), "the place is not a control: {sent:?}");
    }

    /// T024 (FR-004): the unread mark is drawn on the project segment; none for zero.
    #[test]
    fn the_unread_mark_is_drawn_and_none_for_zero() {
        let mut marked = Mounted::new_in(crumb().trailing_mark(3, "3 unread elsewhere"), ROOM);
        assert!(words(&mut marked).contains(&"3".to_string()));
        let mut none = Mounted::new_in(crumb().trailing_mark(0, "none"), ROOM);
        assert!(!words(&mut none).contains(&"0".to_string()));
        assert_eq!(
            segments(&none)[0].width,
            segments(&Mounted::new_in(crumb(), ROOM))[0].width
        );
    }

    /// T024 (contract C2): the project segment always carries its tooltip, and the unread mark's
    /// words follow it.
    #[test]
    fn the_project_segment_carries_its_tooltip_at_full_width() {
        let mut m = Mounted::new_in(crumb().trailing_mark(3, "3 unread elsewhere"), ROOM);
        let at = segments(&m)[0].center();
        let tip = tooltip_at(&mut m, at);
        assert!(tip.starts_with("Switch project"), "{tip:?}");
        assert!(tip.contains("3 unread elsewhere"), "{tip:?}");
        assert!(
            !tip.contains(PROJECT),
            "not shortened, so no name line: {tip:?}"
        );
    }

    /// T024 (FR-007, C2, C3): narrowed, the place shortens first and the project keeps its name;
    /// narrower still, the project shortens too. The glyph and the mark never shorten, nothing
    /// leaves the breadcrumb's room, and each shortened segment's tooltip holds its full text.
    #[test]
    fn narrowed_the_place_shortens_before_the_project_and_tooltips_hold_the_full_text() {
        let long_project = "a-project-with-a-name-sixty-characters-long-for-the-title-ba";
        let build = |project: &str| {
            Breadcrumb::new(project, roles())
                .tooltip("Switch project")
                .on_press("toggle".to_string())
                .trailing_mark(3, "3 unread elsewhere")
                .place(LONG_PLACE, None)
        };

        // Room for the whole project and part of the place.
        let mut m = Mounted::new_in(build(PROJECT), Size::new(300.0, 100.0));
        let painted = words(&mut m);
        assert!(
            painted.contains(&PROJECT.to_string()),
            "project kept: {painted:?}"
        );
        assert!(
            painted
                .iter()
                .any(|w| w.ends_with('…') && LONG_PLACE.starts_with(w.trim_end_matches('…'))),
            "the place shortens: {painted:?}"
        );
        let place = segments(&m)[2];
        assert_eq!(tooltip_at(&mut m, place.center()), LONG_PLACE);
        let project = segments(&m)[0];
        assert!(!tooltip_at(&mut m, project.center()).contains(PROJECT));

        // Too narrow for both: each keeps its floor, one character and `…`.
        let room = Size::new(220.0, 100.0);
        let mut m = Mounted::new_in(build(long_project), room);
        let painted = words(&mut m);
        let glyph = Icon::SelectChevron.glyph().to_string();
        assert!(
            painted.contains(&glyph) && painted.contains(&"3".to_string()),
            "{painted:?}"
        );
        let project_drawn = painted
            .iter()
            .find(|w| long_project.starts_with(w.trim_end_matches('…')) && w.ends_with('…'))
            .unwrap_or_else(|| panic!("the project shortens: {painted:?}"));
        assert!(
            project_drawn.chars().count() >= 2,
            "at least one character and …"
        );
        assert!(
            painted.contains(&floor_text(LONG_PLACE)),
            "the place is at its floor before the project shortens: {painted:?}"
        );
        for leaf in leaves(&m.node) {
            assert!(
                leaf.x + leaf.width <= room.width + 0.5,
                "{leaf:?} leaves the room"
            );
        }
        let project = segments(&m)[0];
        let tip = tooltip_at(&mut m, project.center());
        assert!(
            tip.starts_with("Switch project") && tip.contains(long_project),
            "the shortened project's tooltip holds its full name: {tip:?}"
        );
    }

    /// T024 (FR-006): a place whose full text differs (a short branch) shows the full text on
    /// hover even when it is not shortened.
    #[test]
    fn a_short_branch_shows_its_full_branch_on_hover() {
        let mut m = Mounted::new_in(
            crumb().place("fix/1117", Some("fix/1117-title-bar".to_string())),
            ROOM,
        );
        let place = segments(&m)[2];
        assert_eq!(tooltip_at(&mut m, place.center()), "fix/1117-title-bar");
    }

    /// T024: `.open(true)` draws the project segment selected (the layer is `Button::selected`'s,
    /// pinned in `button.rs`) and moves nothing.
    #[test]
    fn an_open_breadcrumb_moves_nothing() {
        let closed = Mounted::new_in(crumb().place("fix/1117", None), ROOM);
        let open = Mounted::new_in(crumb().place("fix/1117", None).open(true), ROOM);
        assert_eq!(segments(&closed), segments(&open));
        assert_eq!(leaves(&closed.node), leaves(&open.node));
    }

    /// Contract T3: the host the breadcrumb stands its project segment on gives a text button's
    /// content `on_surface`, in both schemes.
    #[test]
    fn the_project_segment_takes_on_surface() {
        for r in [roles(), dark_roles()] {
            assert_eq!(ButtonVariant::Text.content(r, Some(host(r))), r.on_surface);
        }
    }
}
