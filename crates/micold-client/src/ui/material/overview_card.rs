//! `OverviewCard`: one location of the project Overview (feature 813, FR-005, FR-006, FR-021).
//!
//! A header row in the state's colour, with the state icon and word at its start (the reason is
//! their tooltip) and the location type's icon and word at its end (the word is their tooltip),
//! over one neutral body: the title (ellipsized, the display name in a tooltip), the short branch
//! when it has one, the sessions line, and the activity, progress and pull request rows when they
//! have something to say (feature 1120, contracts K1). What each line says is [`CardText`], tested
//! in the core (FR-018). Every colour comes from the theme's roles through [`palette`]: the header
//! of Needs action is the `error` container, Needs attention `warning`, Working `primary`, Done
//! `success`, and Idle's is the grey `surface_variant`; the body is `surface_container_low` in
//! every state. With a press (`on_press`) the card is a control: a
//! click, or Enter or Space on the focused card, publishes the message, and it takes the keyboard
//! like a `Button` (research R13); without one it is informational and not focusable.

use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::material::keyboard_focus::{Indicator, TakesTheKeyboard};
use crate::ui::material::{style, Ellipsized, Glyph, ProgressBar, Ripple, Text, Tooltip, TypeRole};
use iced::widget::{button, column, container, row, stack, Space};
use iced::{keyboard, Alignment, Element, Length};
use micold_core::overview::{CardModel, CardState, LocationType};
use micold_core::overview_text::CardText;
use micold_core::tokens::{spacing, state, Rgb, Roles};
use std::marker::PhantomData;
use std::rc::Rc;

/// The card's minimum height: feature 813's card, which every card was until feature 1120. A card is
/// as tall as its rows need and never shorter than this when its limits ask it to be: the grid lays
/// every card of a row at the row's tallest (FR-014).
pub const MIN_HEIGHT: f32 = 172.0;

/// The colours one card state is drawn in (feature 1120, data-model *Palette*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The header row's fill: the state's colour.
    pub header: Rgb,
    /// The header's icons and words.
    pub on_header: Rgb,
    /// The body's fill, the same in every state.
    pub body: Rgb,
    /// The title, and the card's ripple, focus and *selected* layers.
    pub on_body: Rgb,
    /// Every body line but the title.
    pub on_body_variant: Rgb,
}

/// The roles `state`'s card is drawn in (research R9): the header in the state's container pair,
/// Idle's in the grey `surface_variant` pair; the body in the surface roles.
pub fn palette(state: CardState, r: &Roles) -> Palette {
    let (header, on_header) = match state {
        CardState::NeedsAction => (r.error_container, r.on_error_container),
        CardState::NeedsAttention => (r.warning_container, r.on_warning_container),
        CardState::Working => (r.primary_container, r.on_primary_container),
        CardState::Done => (r.success_container, r.on_success_container),
        CardState::Idle => (r.surface_variant, r.on_surface_variant),
    };
    Palette {
        header,
        on_header,
        body: r.surface_container_low,
        on_body: r.on_surface,
        on_body_variant: r.on_surface_variant,
    }
}

/// The icon of a card state.
pub fn state_icon(state: CardState) -> Icon {
    match state {
        CardState::NeedsAction => Icon::CardNeedsAction,
        CardState::NeedsAttention => Icon::CardNeedsAttention,
        CardState::Working => Icon::StateRunning,
        CardState::Done => Icon::CardDone,
        CardState::Idle => Icon::StateIdle,
    }
}

/// The icon of a location type.
pub fn type_icon(kind: LocationType) -> Icon {
    match kind {
        LocationType::Feature => Icon::TypeFeature,
        LocationType::Bugfix => Icon::TypeBugfix,
        LocationType::Chore => Icon::TypeChore,
        LocationType::Other => Icon::TypeOther,
        LocationType::Included => Icon::TypeIncluded,
        LocationType::OrphanInvalid => Icon::TypeOrphan,
        LocationType::ProjectRoot => Icon::ProjectRoot,
    }
}

/// The icons and words of the type and state, apart from the layout that places them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The type icon.
    pub type_icon: Icon,
    /// The type's word: the end of the header row, and its tooltip.
    pub type_label: &'static str,
    /// The state icon.
    pub state_icon: Icon,
    /// The state's word.
    pub state_label: &'static str,
    /// The reason, the state word's tooltip.
    pub reason_tooltip: &'static str,
}

/// The words of `model`.
pub fn words(model: &CardModel) -> Words {
    Words {
        type_icon: type_icon(model.kind),
        type_label: model.kind.label(),
        state_icon: state_icon(model.state),
        state_label: model.state.label(),
        reason_tooltip: model.reason.label(),
    }
}

/// What a secondary press becomes, built from the press point in window pixels.
type PressAt<'a, M> = Rc<dyn Fn((u16, u16)) -> M + 'a>;

/// The card. See the module documentation.
#[derive(Clone)]
pub struct OverviewCard<'a, M> {
    model: CardModel,
    roles: Roles,
    on_press: Option<M>,
    on_secondary_press: Option<PressAt<'a, M>>,
    on_menu_key: Option<M>,
    targeted: bool,
    menu_open: bool,
    id: Option<iced::advanced::widget::Id>,
    _marker: PhantomData<&'a M>,
}

/// The card's corner radius, shared by its fill and its focus ring.
const RADIUS: f32 = 12.0;

/// The target outline's width (feature 972, R7).
pub const TARGET_WIDTH: f32 = 2.0;

impl<'a, M: 'a> OverviewCard<'a, M> {
    /// The card of `model`, themed by `roles`.
    pub fn new(model: &CardModel, roles: Roles) -> Self {
        Self {
            model: model.clone(),
            roles,
            on_press: None,
            on_secondary_press: None,
            on_menu_key: None,
            targeted: false,
            menu_open: false,
            id: None,
            _marker: PhantomData,
        }
    }

    /// Make the card a control: `message` is published by a click, and by Enter or Space while the
    /// card holds the keyboard.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Publish `f(point)` on a secondary press over the card, `point` being the press in window
    /// pixels: the card's context menu opens there (feature 972, FR-001). The primary press keeps
    /// publishing [`Self::on_press`] only.
    pub fn on_secondary_press(mut self, f: impl Fn((u16, u16)) -> M + 'a) -> Self {
        self.on_secondary_press = Some(Rc::new(f));
        self
    }

    /// Publish `message` on the Menu key and on Shift+F10 while the card holds the keyboard: the
    /// card's context menu opens from the keyboard (feature 972, FR-009). The event is captured, so
    /// no handler beneath the card, a terminal included, sees it. Dropped while the card is
    /// targeted, like Enter and Space.
    pub fn on_menu_key(mut self, message: M) -> Self {
        self.on_menu_key = Some(message);
        self
    }

    /// Mark the card as the target of its open context menu (feature 972, FR-006, research R7): a
    /// 2dp `primary` outline and the *selected* state layer over the header and the body, which neither hover (a
    /// layer alone) nor the focus ring (`secondary`, inset) draws. While targeted, Enter and Space
    /// belong to the menu, so the card drops its claims on them (FR-013).
    pub fn targeted(mut self, targeted: bool) -> Self {
        self.targeted = targeted;
        self
    }

    /// Say a card menu is open, on this card or another: no card claims Enter, Space or the menu
    /// keys then, so they reach the menu's own listener (feature 972, FR-013). Without it a focused
    /// card other than the target would activate on the Enter meant for the menu.
    pub fn menu_open(mut self, open: bool) -> Self {
        self.menu_open = open;
        self
    }

    /// Name the card so the application can move the keyboard to it.
    pub fn id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl<'a, M: Clone + 'a> From<OverviewCard<'a, M>> for Element<'a, M> {
    fn from(card: OverviewCard<'a, M>) -> Self {
        let roles = card.roles;
        let p = palette(card.model.state, &roles);
        let w = words(&card.model);
        let text = CardText::of(&card.model);
        // A glyph and its word, in the header's colour, carrying `tooltip`.
        let cluster = |icon: Icon, word: &'static str, tooltip: &'static str| {
            Tooltip::new(
                row![
                    Glyph::<M>::new(icon, TypeRole::Label, roles).tint(p.on_header),
                    Text::new(word, TypeRole::Label, roles).tint(p.on_header)
                ]
                .spacing(spacing::XS)
                .align_y(Alignment::Center),
                tooltip,
                roles,
            )
        };
        let header = container(
            row![
                cluster(w.state_icon, w.state_label, w.reason_tooltip),
                Space::new().width(Length::Fill),
                cluster(w.type_icon, w.type_label, w.type_label),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([spacing::SM, spacing::MD])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(style::color(p.header))),
            border: iced::Border {
                radius: iced::border::Radius {
                    top_left: RADIUS,
                    top_right: RADIUS,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                ..iced::Border::default()
            },
            ..container::Style::default()
        });
        let line = |words: String, tooltip: Option<String>| -> Element<'a, M> {
            let shown = Ellipsized::<M>::at_role_spaced(words, TypeRole::Body, p.on_body_variant);
            match tooltip {
                Some(tip) => Tooltip::new(shown, tip, roles).into(),
                None => shown.into(),
            }
        };
        let mut body = column![Tooltip::new(
            Ellipsized::<M>::at_role_spaced(text.title, TypeRole::CardTitle, p.on_body),
            text.title_tooltip,
            roles
        )]
        .spacing(spacing::XS);
        if let Some(branch) = text.branch {
            body = body.push(line(branch.text, Some(branch.tooltip)));
        }
        body = body.push(Text::new(text.sessions, TypeRole::Body, roles).tint(p.on_body_variant));
        if let Some(activity) = text.activity {
            body = body.push(line(activity.text, Some(activity.tooltip)));
        }
        if let Some(row) = text.progress {
            body = body.push(Text::new(row.text, TypeRole::Body, roles).tint(p.on_body_variant));
            if let Some(ratio) = row.bar {
                body = body.push(ProgressBar::new(ratio, roles));
            }
        }
        if let Some((text, tooltip)) = text.prs {
            let line = Text::new(text, TypeRole::Body, roles).tint(p.on_body_variant);
            body = match tooltip {
                Some(reason) => body.push(Tooltip::new(line, reason, roles)),
                None => body.push(line),
            };
        }
        let targeted = card.targeted;
        let fill = container(column![header, container(body).padding(spacing::MD)])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(style::color(p.body))),
                border: iced::Border {
                    radius: RADIUS.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            });
        // The target mark is the top layer, so the header's fill cannot paint over the outline
        // (research R13); its *selected* layer covers the header and the body alike.
        let mark =
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| {
                let mut layer = style::color(p.on_body);
                layer.a = state::SELECTED;
                container::Style {
                    background: targeted.then_some(iced::Background::Color(layer)),
                    border: iced::Border {
                        radius: RADIUS.into(),
                        color: style::color(roles.primary),
                        width: if targeted { TARGET_WIDTH } else { 0.0 },
                    },
                    ..container::Style::default()
                }
            });
        let face = stack![fill, mark].width(Length::Fill);
        let secondary = card.on_secondary_press;
        let menu = move |content: Element<'a, M>| match secondary {
            Some(f) => ContextArea::new(content)
                .on_secondary_press(move |at| f(at))
                .into(),
            None => content,
        };
        let Some(message) = card.on_press else {
            return menu(face.into());
        };
        // The card is its own fill, so the button around it draws nothing.
        let pressable = button(face)
            .padding(0)
            .width(Length::Fill)
            .style(|_, _| button::Style::default())
            .on_press(message.clone());
        let mut focusable = TakesTheKeyboard::new(pressable, true);
        if let Some(id) = card.id {
            focusable = focusable.id(id);
        }
        if !targeted && !card.menu_open {
            focusable = focusable
                .key(keyboard::key::Named::Enter, message.clone())
                .key(keyboard::key::Named::Space, message);
            if let Some(menu) = card.on_menu_key {
                focusable = focusable
                    .key(keyboard::key::Named::ContextMenu, menu.clone())
                    .chord(keyboard::key::Named::F10, keyboard::Modifiers::SHIFT, menu);
            }
        }
        let focusable = focusable.indicator(Indicator {
            outline: roles.secondary,
            layer: p.on_body,
            radius: RADIUS,
        });
        // The ripple is the card's only state layer while it runs (feature 971, D1); the context
        // area outside it takes only the secondary press, which the ripple ignores (feature 972).
        menu(
            Ripple::new(focusable, p.on_body, RADIUS)
                .centred_on_keys()
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{leaves, overlap};
    use super::super::test_support::{named_key, roles, Mounted};
    use super::*;
    use iced::advanced::widget::operation::Outcome;
    use iced::advanced::widget::Operation;
    use iced::keyboard::key::Named;
    use iced::{Rectangle, Size};
    use micold_core::overview::{Activation, CardReason, PrDisplay};
    use micold_core::session::{SessionId, SessionLocation};
    use micold_core::task_progress::{Phase, Progress, Ratio};
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn model(state: CardState, name: &str) -> CardModel {
        CardModel {
            location: SessionLocation::Worktree("w".into()),
            display_name: name.into(),
            branch: Some("feat/813-overview".into()),
            kind: LocationType::Feature,
            sessions: (3, 2),
            state,
            reason: CardReason::Working,
            title: "Adds the worktree overview".into(),
            prs: PrDisplay::Hidden,
            progress: Default::default(),
            activity: None,
            activation: Activation::OpenSession(SessionId::new()),
        }
    }

    fn with_progress(ratio: Option<(u32, u32)>, phase: Option<Phase>) -> CardModel {
        let mut m = model(CardState::Working, "feat-a");
        m.progress = Progress {
            phase,
            ratio: ratio.map(|(done, total)| Ratio { done, total }),
            source: micold_core::task_progress::Source::TaskList,
        };
        m
    }

    /// How many bars (4 px leaves) the card draws.
    fn bars(model: &CardModel, width: f32) -> usize {
        let node = super::super::test_support::laid_out(card(model), Size::new(width, 400.0));
        leaves(&node)
            .iter()
            .filter(|l| (l.height - micold_core::tokens::anatomy::progress::THICKNESS).abs() < 0.01)
            .count()
    }

    /// FR-016, FR-009: the row has a bar only with a ratio; with neither a ratio nor a phase
    /// there is no row at all.
    #[test]
    fn the_progress_row_has_a_bar_only_with_a_ratio() {
        let with = with_progress(Some((6, 10)), Some(Phase::Implement));
        assert_eq!(bars(&with, 280.0), 1);
        assert!(painted(&with).contains(&"Implement \u{b7} 6/10".to_string()));
        let phase_only = with_progress(None, Some(Phase::Pr));
        assert_eq!(bars(&phase_only, 280.0), 0);
        assert!(painted(&phase_only).contains(&"PR".to_string()));
        let none = with_progress(None, None);
        assert_eq!(bars(&none, 280.0), 0);
        assert_eq!(
            painted(&none).len(),
            painted(&phase_only).len() - 1,
            "no progress row: one line fewer than with a phase"
        );
    }

    /// Every paragraph `model`'s card paints, in draw order.
    fn painted(model: &CardModel) -> Vec<String> {
        Mounted::new(OverviewCard::<String>::new(model, roles()))
            .painted_text()
            .into_iter()
            .map(|p| p.content)
            .collect()
    }

    /// The card of US2's example: an issue title, a conventional branch, one running session.
    fn titled() -> CardModel {
        let mut m = model(CardState::Working, "feat-1120_x");
        m.title = "Overview cards: readable layout".into();
        m.branch = Some("feat/1120_x".into());
        m.sessions = (1, 1);
        m
    }

    /// FR-009, FR-010, SC-003: a row with nothing to say is not shown.
    #[test]
    fn no_empty_progress_or_pull_request_row_is_shown() {
        let mut m = with_progress(None, None);
        m.prs = PrDisplay::Counts(micold_core::overview::PrCounts {
            open: 0,
            merged: 0,
            open_capped: false,
            merged_capped: false,
        });
        for text in painted(&m) {
            assert_ne!(text, "No task progress", "FR-009: no empty progress row");
            assert!(
                !text.contains("0 open"),
                "FR-010: no 0 \u{b7} 0 row: {text}"
            );
        }
    }

    /// FR-006, FR-007, FR-008, SC-003: the issue title, the short branch and the sessions line,
    /// with the display name and the full branch on hover.
    #[test]
    fn the_card_shows_the_title_the_short_branch_and_the_sessions_line() {
        let m = titled();
        let text = painted(&m);
        let title = "Overview cards: readable layout";
        assert!(
            text.iter().any(|t| {
                let shown = t.trim_end_matches('\u{2026}');
                shown.starts_with("Overview cards") && title.starts_with(shown)
            }),
            "the issue title is the title, ellipsized at most: {text:?}"
        );
        assert!(text.contains(&"feat/1120".to_string()), "{text:?}");
        assert!(
            !text
                .iter()
                .any(|t| t.contains("feat/1120_x") || t == "feat-1120_x"),
            "neither the full branch nor the display name is shown: {text:?}"
        );
        assert!(
            text.contains(&"1 session \u{b7} 1 running".to_string()),
            "{text:?}"
        );
        let words = micold_core::overview_text::CardText::of(&m);
        assert_eq!(
            words.title_tooltip,
            "Overview cards: readable layout\nfeat-1120_x"
        );
        assert_eq!(
            words.branch.map(|b| b.tooltip).as_deref(),
            Some("feat/1120_x")
        );
    }

    /// US2.8: a card whose title is its branch shows the branch once.
    #[test]
    fn a_card_titled_by_its_branch_shows_it_once() {
        let mut m = titled();
        m.title = "feat/1120_x".into();
        let text = painted(&m);
        assert_eq!(
            text.iter().filter(|t| t.contains("feat/1120")).count(),
            1,
            "{text:?}"
        );
    }

    /// A card with every optional row: a branch, an activity, progress with a bar, and pull
    /// requests; `long` makes the title, branch and activity too long for any card width.
    fn full(long: bool) -> CardModel {
        let mut m = with_progress(Some((6, 10)), Some(Phase::Implement));
        m.prs = PrDisplay::Unknown(micold_core::overview::PrUnknown::Unreachable);
        let tail = if long {
            " that is far too long to fit".repeat(8)
        } else {
            String::new()
        };
        m.title = format!("Adds the worktree overview{tail}");
        // A conventional branch shows as `type/number` (FR-007), so the long one is not.
        m.branch = Some(if long {
            format!("topic{}", tail.replace(' ', "-"))
        } else {
            "feat/1120-overview".into()
        });
        m.activity = Some(micold_core::hook_activity::CurrentActivity {
            line: format!("Editing card_grid.rs{tail}"),
            ratio: Some(micold_core::task_progress::Ratio { done: 3, total: 7 }),
        });
        m
    }

    /// The card widths FR-015 and SC-002 name.
    const WIDTHS: [f32; 3] = [280.0, 380.0, 480.0];

    /// What `model`'s card paints laid out `width` wide.
    fn painted_at(model: &CardModel, width: f32) -> Vec<super::super::test_support::Painted> {
        Mounted::new(
            container(OverviewCard::<String>::new(model, roles())).width(Length::Fixed(width)),
        )
        .painted_text()
    }

    /// FR-014, FR-015, US4.1, SC-002: with every optional row, at 280, 380 and 480 px, no two
    /// pieces of the card overlap, every piece sits inside the card, and the card is at least
    /// `MIN_HEIGHT` tall; a long title, branch and activity end in an ellipsis and the tooltips
    /// hold the full text.
    #[test]
    fn a_card_with_every_row_fits_at_every_card_width() {
        for long in [false, true] {
            let m = full(long);
            for width in WIDTHS {
                let node = super::super::test_support::laid_out(card(&m), Size::new(width, 1000.0));
                assert!(
                    node.size().height >= MIN_HEIGHT,
                    "{width}: {:?}",
                    node.size()
                );
                assert_eq!(node.size().width, width);
                let parts = pieces(&node);
                for (i, a) in parts.iter().enumerate() {
                    assert!(
                        a.x >= -0.01
                            && a.y >= -0.01
                            && a.x + a.width <= width + 0.01
                            && a.y + a.height <= node.size().height - spacing::MD + 0.01,
                        "{width}: {a:?} leaves the card's padded box {:?}",
                        node.size()
                    );
                    for b in &parts[i + 1..] {
                        assert!(!overlap(a, b), "{width}: {a:?} overlaps {b:?}");
                    }
                }
                if long {
                    let text = painted_at(&m, width);
                    for start in ["Adds", "topic-", "Editing"] {
                        let line = text
                            .iter()
                            .find(|p| p.content.starts_with(start))
                            .unwrap_or_else(|| panic!("{width}: no {start:?} line: {text:?}"));
                        assert!(line.content.ends_with('\u{2026}'), "{width}: {line:?}");
                    }
                }
            }
            if long {
                let words = CardText::of(&m);
                assert!(words.title_tooltip.starts_with(&m.title));
                assert_eq!(words.branch.map(|b| b.tooltip), m.branch.clone());
                let activity = words.activity.expect("an activity row");
                assert!(activity
                    .tooltip
                    .contains(&m.activity.as_ref().unwrap().line));
            }
        }
    }

    /// FR-004, SC-004: the title is 17 px, the body lines 14 px, and nothing on the card under
    /// 12 px.
    #[test]
    fn the_title_is_17_px_the_body_14_and_nothing_under_12() {
        let m = full(false);
        let text = painted_at(&m, 380.0);
        for p in &text {
            assert!(p.size >= 12.0, "{p:?} is under 12 px");
        }
        let size_of = |start: &str| {
            text.iter()
                .find(|p| p.content.starts_with(start))
                .unwrap_or_else(|| panic!("no {start:?} line: {text:?}"))
                .size
        };
        assert_eq!(size_of("Adds"), 17.0);
        assert_eq!(size_of("Feature"), 12.0, "the header's words are Label");
        for body in ["feat/1120", "3 sessions", "Editing card_grid", "Implement"] {
            assert_eq!(size_of(body), 14.0, "{body}");
        }
    }

    /// FR-014, US4.3: the card is a minimum, not a fixed height: a sparse card given a 172 px
    /// minimum by its limits is 172 tall, and alone it is no taller than it needs.
    #[test]
    fn a_sparse_card_takes_the_minimum_its_limits_give() {
        let m = model(CardState::Idle, "feat-a");
        let mut element = card(&m);
        let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &super::super::test_support::renderer(),
            &iced::advanced::layout::Limits::new(
                Size::new(280.0, MIN_HEIGHT),
                Size::new(280.0, 1000.0),
            ),
        );
        assert_eq!(node.size(), Size::new(280.0, MIN_HEIGHT));
        let natural = super::super::test_support::size_of(card(&m), Size::new(280.0, 1000.0));
        assert!(natural.height < MIN_HEIGHT, "{natural:?}");
    }

    /// The card's leaves but the target-mark layer, which is stacked over the whole card (K1).
    fn pieces(node: &iced::advanced::layout::Node) -> Vec<Rectangle> {
        leaves(node)
            .into_iter()
            .filter(|l| l.size() != node.size())
            .collect()
    }

    fn card(model: &CardModel) -> Element<'static, ()> {
        OverviewCard::<()>::new(model, tokens::roles(ColorScheme::Light)).into()
    }

    /// U4, FR-015: at 280, 380 and 480 px no two pieces of the card overlap and none leaves it,
    /// even with a long title and a long branch.
    #[test]
    fn no_text_overlaps_at_280_and_a_long_title_ellipsizes() {
        let long = "a-title-that-is-far-too-long-to-fit-in-the-card ".repeat(4);
        for state in CardState::ALL {
            for (text, width) in ["feat-a", long.as_str()]
                .into_iter()
                .flat_map(|t| WIDTHS.map(|w| (t, w)))
            {
                let mut m = model(state, text);
                m.title = text.into();
                m.branch = Some(format!("feat/{text}"));
                let node = super::super::test_support::laid_out(card(&m), Size::new(width, 400.0));
                let parts = pieces(&node);
                let whole = Rectangle::new(iced::Point::ORIGIN, node.size());
                for (i, a) in parts.iter().enumerate() {
                    assert!(
                        a.x >= -0.01
                            && a.x + a.width <= whole.width + 0.01
                            && a.y + a.height <= whole.height + 0.01,
                        "{state:?}: {a:?} leaves the card {whole:?}"
                    );
                    for b in &parts[i + 1..] {
                        assert!(!overlap(a, b), "{state:?}: {a:?} overlaps {b:?}");
                    }
                }
                assert_eq!(node.size().width, width);
            }
        }
    }

    /// U4, FR-015: the full title and the display name are the tooltip of the ellipsized title.
    #[test]
    fn the_full_title_is_in_the_tooltip() {
        let long = "x".repeat(200);
        let mut m = model(CardState::Idle, "feat-a");
        m.title = long.clone();
        assert_eq!(
            micold_core::overview_text::CardText::of(&m).title_tooltip,
            format!("{long}\nfeat-a")
        );
    }

    /// SC-005: each card carries its type icon with the type's word, and its state icon with the
    /// state's word and the reason; the five states and the types are told apart by icon.
    #[test]
    fn a_card_shows_its_type_and_state_with_their_words() {
        let mut state_icons = Vec::new();
        for state in CardState::ALL {
            let w = words(&model(state, "n"));
            assert_eq!(w.state_label, state.label());
            assert_eq!(w.reason_tooltip, CardReason::Working.label());
            assert_eq!(w.type_label, LocationType::Feature.label());
            assert_eq!(w.type_icon, Icon::TypeFeature);
            state_icons.push(w.state_icon);
        }
        let distinct: std::collections::BTreeSet<String> =
            state_icons.iter().map(|i| format!("{i:?}")).collect();
        assert_eq!(distinct.len(), 5, "{state_icons:?}");
        let types = [
            LocationType::Feature,
            LocationType::Bugfix,
            LocationType::Chore,
            LocationType::Other,
            LocationType::Included,
            LocationType::OrphanInvalid,
            LocationType::ProjectRoot,
        ];
        let icons: Vec<Icon> = types.iter().map(|t| type_icon(*t)).collect();
        for (i, a) in icons.iter().enumerate() {
            assert!(!icons[i + 1..].contains(a), "{a:?} stands for two types");
        }
        let text = painted(&model(CardState::Idle, "n"));
        assert!(
            text.contains(&"Feature".to_string()) && text.contains(&"Idle".to_string()),
            "FR-001: the type's and the state's words are visible text on the card: {text:?}"
        );
        assert!(
            text.contains(&"3 sessions \u{b7} 2 running".to_string()),
            "the type word left the sessions line: {text:?}"
        );
    }

    /// The paragraph that says exactly `word`.
    fn word<'p>(
        text: &'p [super::super::test_support::Painted],
        word: &str,
    ) -> &'p super::super::test_support::Painted {
        text.iter()
            .find(|p| p.content == word)
            .unwrap_or_else(|| panic!("no {word:?}: {text:?}"))
    }

    /// `model`'s card mounted `width` wide in a room that holds it.
    fn mounted_at(model: &CardModel, width: f32, roles: Roles) -> Mounted<'static> {
        Mounted::new_in(
            container(OverviewCard::<String>::new(model, roles)).width(Length::Fixed(width)),
            Size::new(500.0, 1000.0),
        )
    }

    /// The pixel at `(x, y)` of a drawn buffer `width` wide is `want`, give or take rounding.
    fn is_colour(pixels: &[u8], width: u32, (x, y): (f32, f32), want: Rgb) -> bool {
        let i = ((y as u32 * width + x as u32) * 4) as usize;
        [want.r, want.g, want.b]
            .iter()
            .zip(&pixels[i..i + 3])
            .all(|(w, g)| w.abs_diff(*g) <= 2)
    }

    /// T032 (K1, K4 — FR-001, FR-002, FR-003, US1.1, SC-005): at 280, 380 and 480 px, in both
    /// schemes and the five states, the header row spans the card's width at its top in the
    /// state's `header` fill, the state's glyph and word end left of where the type's start, the
    /// words are in `on_header`; the body below is the one `body` fill with the title in `on_body`
    /// and the other lines in `on_body_variant`; the type word is no longer in the sessions line.
    #[test]
    fn the_header_row_carries_the_state_at_the_start_and_the_type_at_the_end() {
        for r in [roles(), super::super::test_support::dark_roles()] {
            for state in CardState::ALL {
                let mut m = full(false);
                m.state = state;
                let p = palette(state, &r);
                for width in WIDTHS {
                    let mut card = mounted_at(&m, width, r);
                    let text = card.painted_text();
                    let state_word = word(&text, state.label());
                    let type_word = word(&text, "Feature");
                    let title = text
                        .iter()
                        .find(|t| t.content.starts_with("Adds"))
                        .expect("a title");
                    let sessions = word(&text, "3 sessions \u{b7} 2 running");
                    let at = format!("{state:?} at {width}");
                    assert!(
                        state_word.bounds.x + state_word.bounds.width < type_word.bounds.x,
                        "{at}: the state {:?} does not end left of the type {:?}",
                        state_word.bounds,
                        type_word.bounds
                    );
                    assert!(
                        state_word.bounds.x < width / 2.0 && type_word.bounds.x > width / 2.0,
                        "{at}: the state starts the header and the type ends it"
                    );
                    let header_bottom = state_word.bounds.y + state_word.bounds.height;
                    assert!(
                        header_bottom <= title.bounds.y,
                        "{at}: the header is on top"
                    );
                    for (what, painted, want) in [
                        ("state word", state_word, p.on_header),
                        ("type word", type_word, p.on_header),
                        ("title", title, p.on_body),
                        ("sessions line", sessions, p.on_body_variant),
                    ] {
                        assert_eq!(painted.color, style::color(want), "{at}: the {what}");
                    }
                    let (pixels, w) = card.drawn(style::color(r.surface));
                    let h = card.node.size().height;
                    let band = header_bottom + 3.0;
                    for x in [1.0, width / 2.0, width - 2.0] {
                        assert!(
                            is_colour(&pixels, w, (x, band), p.header),
                            "{at}: ({x}, {band}) is not the header fill"
                        );
                    }
                    for spot in [(4.0, title.bounds.center_y()), (width / 2.0, h - 4.0)] {
                        assert!(
                            is_colour(&pixels, w, spot, p.body),
                            "{at}: {spot:?} is not the body fill"
                        );
                    }
                }
            }
        }
    }

    /// T032 (US1.4): hovering the state word shows the reason, hovering the type word its word.
    #[test]
    fn the_header_words_carry_the_reason_and_the_type_as_tooltips() {
        let m = full(false);
        for width in [280.0, 380.0] {
            let text = mounted_at(&m, width, roles()).painted_text();
            for (hovered, want) in [(m.state.label(), m.reason.label()), ("Feature", "Feature")] {
                let at = word(&text, hovered).bounds.center();
                let floated = mounted_at(&m, width, roles()).floated_text(at);
                assert_eq!(floated, [want], "{width}: hovering {hovered:?}");
            }
        }
    }

    /// How many controls take the keyboard in the card.
    struct Focusables(usize);

    impl Operation<usize> for Focusables {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<usize>)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&iced::advanced::widget::Id>,
            _bounds: Rectangle,
            _state: &mut dyn iced::advanced::widget::operation::Focusable,
        ) {
            self.0 += 1;
        }

        fn finish(&self) -> Outcome<usize> {
            Outcome::Some(self.0)
        }
    }

    fn mounted_targeted(targeted: bool) -> Mounted<'static> {
        Mounted::new(
            OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                .on_press("open".to_string())
                .targeted(targeted),
        )
    }

    fn mounted(pressable: bool) -> Mounted<'static> {
        let card = OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles());
        Mounted::new(if pressable {
            card.on_press("open".to_string())
        } else {
            card
        })
    }

    /// US3.4, FR-014: a card with a press takes the keyboard, and one without does not.
    #[test]
    fn a_card_takes_the_keyboard_only_with_a_press() {
        assert_eq!(mounted(true).drive(Focusables(0)), Some(1));
        assert_eq!(mounted(false).drive(Focusables(0)), Some(0));
    }

    /// US3.4: Enter and Space on the focused card publish its message; unfocused, they do not; a
    /// card without a press publishes nothing.
    #[test]
    fn enter_and_space_on_the_focused_card_publish_its_message() {
        let mut card = mounted(true);
        let cursor = iced::advanced::mouse::Cursor::Unavailable;
        assert!(card.send(named_key(Named::Enter), cursor).is_empty());
        card.focus_next();
        for key in [Named::Enter, Named::Space] {
            assert_eq!(card.send(named_key(key), cursor), ["open"], "{key:?}");
        }
        let mut plain = mounted(false);
        plain.focus_next();
        assert!(plain.send(named_key(Named::Enter), cursor).is_empty());
    }

    /// FR-014: a click publishes it too.
    #[test]
    fn a_click_on_the_card_publishes_its_message() {
        use iced::advanced::mouse::{Button, Cursor, Event as Mouse};
        let mut card = mounted(true);
        let at = card.node.bounds().center();
        card.press(at);
        let released = card.send(
            iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            Cursor::Available(at),
        );
        assert_eq!(released, ["open"]);
    }

    /// Feature 972, FR-001: a secondary press over a card with `on_secondary_press` publishes the
    /// menu message at the window-pixel press point; FR-013: a primary press still publishes only
    /// `on_press`; without the builder a secondary press publishes nothing.
    #[test]
    fn a_secondary_press_publishes_the_menu_message_at_the_press_point() {
        use iced::advanced::mouse::{Button, Cursor, Event as Mouse};
        use iced::Point;
        let right = |card: &mut Mounted<'_>, at: Point| {
            card.hover(at);
            card.send(
                iced::Event::Mouse(Mouse::ButtonPressed(Button::Right)),
                Cursor::Available(at),
            )
        };
        let menu = || {
            OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                .on_press("open".to_string())
                .on_secondary_press(|(x, y)| format!("menu at {x},{y}"))
        };
        let at = Point::new(40.0, 30.0);

        let mut card = Mounted::new(menu());
        assert_eq!(right(&mut card, at), ["menu at 40,30"]);

        let mut card = Mounted::new(menu());
        card.press(at);
        let released = card.send(
            iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            Cursor::Available(at),
        );
        assert_eq!(
            released,
            ["open"],
            "a primary click keeps its 813 behaviour"
        );

        let mut plain = mounted(true);
        assert!(right(&mut plain, at).is_empty(), "no builder, no menu");
    }

    /// FR-022: keyboard focus draws the ring in `secondary` along the card's edge; at rest there is
    /// none.
    #[test]
    fn the_focused_card_draws_the_focus_ring() {
        let r = roles();
        let backdrop = style::color(r.surface);
        let ring = style::color(r.secondary);
        // How far a pixel is from the ring's colour, summed over the channels.
        let apart = |p: &[u8]| -> f32 {
            [ring.r, ring.g, ring.b]
                .iter()
                .zip(p)
                .map(|(want, got)| (want * 255.0 - f32::from(*got)).abs())
                .sum()
        };
        let mut card = mounted(true);
        let (rest, width) = card.drawn(backdrop);
        // Mid-top edge, over the header row (feature 1120, FR-016), inside the stroke (centred
        // half a ring width in), which antialiasing blends with the fill: so closeness, not
        // equality.
        let i = ((width + width / 2) * 4) as usize;
        card.focus_next();
        let (focused, _) = card.drawn(backdrop);
        assert!(
            apart(&focused[i..i + 3]) + 60.0 < apart(&rest[i..i + 3]),
            "the focused card's edge {:?} is no nearer the ring colour than at rest {:?}",
            &focused[i..i + 3],
            &rest[i..i + 3]
        );
    }

    /// Feature 972, FR-006 (R7): a targeted card draws a 2dp `primary` outline at its edge and the
    /// selected layer over its fill; an untargeted one draws neither; and it differs from the
    /// focused card (a `secondary` ring) and from the card at rest by more than a layer.
    #[test]
    fn the_targeted_card_draws_a_primary_outline_and_the_selected_layer() {
        let r = roles();
        let backdrop = style::color(r.surface);
        let outline = style::color(r.primary);
        let apart = |p: &[u8]| -> f32 {
            [outline.r, outline.g, outline.b]
                .iter()
                .zip(p)
                .map(|(want, got)| (want * 255.0 - f32::from(*got)).abs())
                .sum()
        };
        let (rest, width) = mounted_targeted(false).drawn(backdrop);
        let (targeted, _) = mounted_targeted(true).drawn(backdrop);
        let mut focused_card = mounted_targeted(false);
        focused_card.focus_next();
        let (focused, _) = focused_card.drawn(backdrop);
        let px = |x: u32, y: u32| ((y * width + x) * 4) as usize;
        // Mid-top edge, over the header row (feature 1120), both rows of the 2dp outline.
        for row in [0, 1] {
            let i = px(width / 2, row);
            assert!(
                apart(&targeted[i..i + 3]) < 12.0,
                "row {row} of the edge {:?} is not the primary outline",
                &targeted[i..i + 3]
            );
            assert!(
                apart(&rest[i..i + 3]) > 60.0,
                "an untargeted card draws an outline"
            );
        }
        // Inside the padding, left of the text, in the body and in the header (feature 1120,
        // FR-016): only the selected layer changes the fill.
        let inside = px(5, 80);
        for (what, at) in [("body", inside), ("header", px(5, 20))] {
            assert_ne!(
                targeted[at..at + 3],
                rest[at..at + 3],
                "no selected layer over the {what}"
            );
        }
        assert_ne!(
            targeted, focused,
            "the target mark must differ from the focus ring"
        );
        assert_ne!(focused[inside..inside + 3], targeted[inside..inside + 3]);
    }

    /// Feature 972, FR-013 (R6): while targeted the card drops its Enter and Space claims, so the
    /// menu owns them; the click still reaches `on_press`.
    #[test]
    fn a_targeted_card_does_not_publish_on_enter_or_space() {
        let mut card = mounted_targeted(true);
        card.focus_next();
        let cursor = iced::advanced::mouse::Cursor::Unavailable;
        for key in [Named::Enter, Named::Space] {
            assert!(card.send(named_key(key), cursor).is_empty(), "{key:?}");
        }
        let mut open = mounted_targeted(false);
        open.focus_next();
        assert_eq!(open.send(named_key(Named::Enter), cursor), ["open"]);
    }

    /// Feature 972, FR-009 (T021): the Menu key and Shift+F10 on the focused card publish the menu
    /// message and are captured; F10 alone does nothing; unfocused, nothing; a targeted card drops
    /// the claim. Neither key starts a ripple (feature 971).
    #[test]
    fn the_menu_key_and_shift_f10_on_the_focused_card_publish_the_menu_message() {
        use super::super::test_support::key_with;
        use iced::keyboard::Modifiers;
        let cursor = iced::advanced::mouse::Cursor::Unavailable;
        let card = |targeted| {
            Mounted::new(
                OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                    .on_press("open".to_string())
                    .on_menu_key("menu".to_string())
                    .targeted(targeted),
            )
        };
        let menu_keys = [
            key_with(Named::ContextMenu, Modifiers::empty()),
            key_with(Named::F10, Modifiers::SHIFT),
        ];
        let mut unfocused = card(false);
        for key in &menu_keys {
            assert_eq!(
                unfocused.send_captured(key.clone(), cursor),
                (vec![], false)
            );
        }
        let mut focused = card(false);
        focused.focus_next();
        for key in &menu_keys {
            assert_eq!(
                focused.send_captured(key.clone(), cursor),
                (vec!["menu".to_string()], true),
                "{key:?}"
            );
        }
        assert_eq!(
            focused.send_captured(named_key(Named::F10), cursor),
            (vec![], false)
        );
        assert!(ripple(&focused).is_idle(), "a menu key must not ripple");
        let mut targeted = card(true);
        targeted.focus_next();
        for key in &menu_keys {
            assert_eq!(targeted.send_captured(key.clone(), cursor), (vec![], false));
        }
    }

    /// Feature 972, FR-013: while a menu is open on another card, this one claims no key either.
    #[test]
    fn a_card_drops_its_keys_while_any_menu_is_open() {
        use super::super::test_support::key_with;
        let cursor = iced::advanced::mouse::Cursor::Unavailable;
        let mut card = Mounted::new(
            OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                .on_press("open".to_string())
                .on_menu_key("menu".to_string())
                .menu_open(true),
        );
        card.focus_next();
        for key in [
            named_key(Named::Enter),
            named_key(Named::Space),
            key_with(Named::ContextMenu, iced::keyboard::Modifiers::empty()),
        ] {
            assert_eq!(card.send_captured(key, cursor), (vec![], false));
        }
    }

    /// Feature 972, FR-008 (R3): an area around the cards publishes on a right-press in empty space
    /// and stays silent over a card, whose own area has answered and captured the press.
    #[test]
    fn a_right_press_over_a_card_does_not_also_reach_the_pane_around_it() {
        use iced::advanced::mouse::{Button, Cursor, Event as Mouse};
        let pane = || {
            ContextArea::new(
                iced::widget::container(
                    OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                        .on_press("open".to_string())
                        .on_secondary_press(|(x, y)| format!("menu at {x},{y}")),
                )
                .height(Length::Fixed(MIN_HEIGHT * 2.0)),
            )
            .on_secondary_press(|_| "dismiss".to_string())
        };
        let right = |at: iced::Point| {
            let mut pane = Mounted::new(pane());
            pane.hover(at);
            pane.send(
                iced::Event::Mouse(Mouse::ButtonPressed(Button::Right)),
                Cursor::Available(at),
            )
        };
        assert_eq!(right(iced::Point::new(40.0, 30.0)), ["menu at 40,30"]);
        assert_eq!(
            right(iced::Point::new(40.0, MIN_HEIGHT * 2.0 - 10.0)),
            ["dismiss"],
            "empty space dismisses"
        );
    }

    // ---- Feature 971: the pointer ripple (FR-001, FR-003, FR-004, FR-005, FR-006) ----

    use crate::ui::cdk::ripple::Ripple as RippleState;
    use iced::{mouse, Point};
    use micold_core::tokens::motion::duration;
    use std::time::Duration;

    /// The ripple's own timings: one advance is one frame at them.
    const EXPAND: Duration = Duration::from_millis(duration::MEDIUM_2);
    const FADE: Duration = Duration::from_millis(duration::SHORT_4);

    fn activatable(model: &CardModel) -> Mounted<'static> {
        Mounted::new(
            OverviewCard::<String>::new(model, tokens::roles(ColorScheme::Light))
                .on_press("pressed".to_string()),
        )
    }

    fn ripple<'m>(m: &'m Mounted<'_>) -> &'m RippleState {
        m.tree.state.downcast_ref::<RippleState>()
    }

    fn settle(m: &mut Mounted<'_>) {
        let state = m.tree.state.downcast_mut::<RippleState>();
        for _ in 0..120 {
            state.advance(EXPAND, FADE);
        }
    }

    /// T003 (FR-001, SC-001): a press at each corner and the centre starts a ripple at that
    /// point, in every state.
    #[test]
    fn a_press_ripples_from_the_press_point_in_every_state() {
        for state in CardState::ALL {
            let b = activatable(&model(state, "feat-a")).node.bounds();
            let spots = [
                (0.5, 0.5),
                (b.width - 0.5, 0.5),
                (0.5, b.height - 0.5),
                (b.width - 0.5, b.height - 0.5),
                (b.width / 2.0, b.height / 2.0),
            ];
            for (x, y) in spots {
                let mut m = activatable(&model(state, "feat-a"));
                m.press(Point::new(x, y));
                let origin = ripple(&m).origin().expect("the press started a ripple");
                assert!(
                    (origin.x - x).abs() <= 1.0 && (origin.y - y).abs() <= 1.0,
                    "{state:?}: pressed ({x}, {y}), rippled from {origin:?}"
                );
            }
        }
    }

    /// T003 (US1-4, FR-003; feature 1120, R9): mid-ripple the pixel at the press point has moved
    /// from the body's fill toward its content colour `on_body`, and no further than it.
    #[test]
    fn the_ripple_is_drawn_in_the_states_content_colour() {
        let roles = tokens::roles(ColorScheme::Light);
        for state in CardState::ALL {
            let p = palette(state, &roles);
            let mut m = activatable(&model(state, "feat-a"));
            // In the body's left padding, clear of any line.
            let at = Point::new(6.0, 100.0);
            m.press(at);
            m.tree
                .state
                .downcast_mut::<RippleState>()
                .advance(EXPAND, FADE);
            let (pixels, width) = m.drawn(iced::Color::BLACK);
            let i = ((at.y as u32 * width + at.x as u32) * 4) as usize;
            let got = [pixels[i], pixels[i + 1], pixels[i + 2]];
            let (fill, text) = (
                [p.body.r, p.body.g, p.body.b],
                [p.on_body.r, p.on_body.g, p.on_body.b],
            );
            let mut moved = false;
            for c in 0..3 {
                let (lo, hi) = (fill[c].min(text[c]), fill[c].max(text[c]));
                assert!(
                    u16::from(got[c]) + 2 >= u16::from(lo)
                        && u16::from(got[c]) <= u16::from(hi) + 2,
                    "{state:?}: channel {c} is {} outside {lo}..{hi}",
                    got[c]
                );
                moved |= got[c].abs_diff(fill[c]) > 2;
            }
            assert!(
                moved,
                "{state:?}: the ripple left the fill unchanged: {got:?}"
            );
        }
    }

    /// T004 (FR-006): a plain card holds no ripple state and publishes nothing.
    #[test]
    fn a_plain_card_does_not_ripple() {
        let mut plain = Mounted::new(OverviewCard::<String>::new(
            &model(CardState::Idle, "feat-a"),
            tokens::roles(ColorScheme::Light),
        ));
        assert_ne!(
            plain.tree.tag,
            iced::advanced::widget::tree::Tag::of::<RippleState>()
        );
        assert!(plain.press(Point::new(20.0, 20.0)).is_empty());
    }

    /// T004: a secondary-button press starts no ripple.
    #[test]
    fn a_secondary_press_starts_no_ripple() {
        let mut m = activatable(&model(CardState::Idle, "feat-a"));
        m.send(
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
        );
        assert!(ripple(&m).is_idle());
    }

    /// T004 (FR-004, US1-6, US1-5, SC-004): once the ripple fades the pointer-pressed card shows
    /// its plain fill (no focus layer) and the ripple is idle, so nothing animates.
    #[test]
    fn after_the_fade_a_pointer_press_leaves_no_layer() {
        let roles = tokens::roles(ColorScheme::Light);
        let p = palette(CardState::Working, &roles);
        let mut m = activatable(&model(CardState::Working, "feat-a"));
        // In the body's left padding, clear of any line.
        let at = Point::new(6.0, 100.0);
        m.press(at);
        assert!(!ripple(&m).is_idle());
        settle(&mut m);
        assert!(ripple(&m).is_idle());
        let (pixels, width) = m.drawn(iced::Color::BLACK);
        let i = ((at.y as u32 * width + at.x as u32) * 4) as usize;
        let got = [pixels[i], pixels[i + 1], pixels[i + 2]];
        for (c, want) in [p.body.r, p.body.g, p.body.b].into_iter().enumerate() {
            assert!(
                got[c].abs_diff(want) <= 1,
                "channel {c}: {got:?} vs the body"
            );
        }
    }

    /// T006 (FR-005): two cards ripple independently; a second press restarts from the new point.
    #[test]
    fn cards_ripple_independently_and_a_second_press_restarts() {
        let mut a = activatable(&model(CardState::Working, "a"));
        let b = activatable(&model(CardState::Working, "b"));
        a.press(Point::new(10.0, 10.0));
        a.tree
            .state
            .downcast_mut::<RippleState>()
            .advance(EXPAND, FADE);
        assert!(ripple(&b).is_idle(), "pressing A rippled B");
        let mid = ripple(&a).expansion();
        a.press(Point::new(100.0, 100.0));
        assert_eq!(ripple(&a).origin(), Some(Point::new(100.0, 100.0)));
        assert!(ripple(&a).expansion() < mid);
    }

    /// T006 (edge): press, drag off the card, release: the card is not activated while its
    /// ripple fades.
    #[test]
    fn a_press_dragged_off_does_not_activate() {
        let mut m = activatable(&model(CardState::Working, "a"));
        m.press(Point::new(20.0, 20.0));
        let outside = Point::new(20.0, 700.0);
        m.hover(outside);
        let released = m.send(
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(outside),
        );
        assert!(released.is_empty());
        assert!(!ripple(&m).is_idle(), "the ripple still fades");
    }

    /// T006 (edge): a press in a rounded corner, outside the rounded shape, ripples from the press
    /// point.
    #[test]
    fn a_press_in_a_rounded_corner_ripples_from_the_press_point() {
        let mut m = activatable(&model(CardState::Working, "a"));
        m.press(Point::new(0.5, 0.5));
        assert_eq!(ripple(&m).origin(), Some(Point::new(0.5, 0.5)));
    }

    // ---- Feature 971: the keyboard ripple (FR-002, FR-004, FR-005, SC-002) ----

    /// T016 (FR-002, SC-002, US2-1, US2-2): Enter and Space on the focused card start a ripple at
    /// the card's centre, in every state.
    #[test]
    fn enter_and_space_on_the_focused_card_ripple_from_the_centre() {
        let cursor = mouse::Cursor::Unavailable;
        for state in CardState::ALL {
            for key in [Named::Enter, Named::Space] {
                let mut m = activatable(&model(state, "feat-a"));
                let b = m.node.bounds();
                m.focus_next();
                assert!(ripple(&m).is_idle(), "{state:?}: focusing rippled");
                assert_eq!(m.send(named_key(key), cursor), ["pressed"]);
                let origin = ripple(&m).origin().expect("the key started a ripple");
                assert!(
                    (origin.x - b.width / 2.0).abs() <= 1.0
                        && (origin.y - b.height / 2.0).abs() <= 1.0,
                    "{state:?} {key:?}: rippled from {origin:?}"
                );
            }
        }
    }

    /// T016 (US2-3): a card that does not hold the focus shows no ripple on Enter or Space.
    #[test]
    fn enter_and_space_on_an_unfocused_card_do_not_ripple() {
        let cursor = mouse::Cursor::Unavailable;
        for key in [Named::Enter, Named::Space] {
            let mut m = activatable(&model(CardState::Idle, "feat-a"));
            assert!(m.send(named_key(key), cursor).is_empty());
            assert!(ripple(&m).is_idle(), "{key:?}");
        }
    }

    /// T016 (FR-004, FR-005): the keyboard ripple fades to idle, leaving nothing animating, and a
    /// second card is untouched.
    #[test]
    fn the_keyboard_ripple_fades_to_idle() {
        let mut m = activatable(&model(CardState::Idle, "feat-a"));
        m.focus_next();
        m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
        assert!(!ripple(&m).is_idle());
        settle(&mut m);
        assert!(ripple(&m).is_idle());
        let other = activatable(&model(CardState::Idle, "feat-b"));
        assert!(ripple(&other).is_idle());
    }
}
