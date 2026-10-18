//! `OverviewCard`: one location of the project Overview (feature 813, FR-005, FR-006, FR-021).
//!
//! The type icon and the display name (ellipsized, the full name in a tooltip), the state icon and
//! word (the reason is the word's tooltip), the branch when it has one, the summary line and the
//! activity line (when the agent's hooks report), the sessions footer. Every colour comes from the
//! theme's roles through [`palette`]: Needs action is the `error` family, Needs attention
//! `warning`, Working `primary`, Done `success`, and Idle sits on the de-emphasised
//! `surface_container_low`. With a press (`on_press`) the card is a control: a
//! click, or Enter or Space on the focused card, publishes the message, and it takes the keyboard
//! like a `Button` (research R13); without one it is informational and not focusable.

use crate::icons::Icon;
use crate::ui::material::keyboard_focus::{Indicator, TakesTheKeyboard};
use crate::ui::material::{style, Ellipsized, Glyph, ProgressBar, Ripple, Text, Tooltip, TypeRole};
use iced::widget::{button, column, container, row};
use iced::{keyboard, Alignment, Element, Length};
use micold_core::hook_activity::CurrentActivity;
use micold_core::overview::{CardModel, CardState, LocationType};
use micold_core::task_progress::{Progress, Ratio};
use micold_core::tokens::{spacing, Rgb, Roles};
use std::marker::PhantomData;

/// The card's one height: the same for every state and every content, so a grid row never ragged.
pub const HEIGHT: f32 = 172.0;

/// The colours one card state is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The card's fill.
    pub fill: Rgb,
    /// The words on the fill.
    pub text: Rgb,
    /// The icons on the fill.
    pub icon: Rgb,
}

/// The fill, text and icon roles of `state` (research R11).
pub fn palette(state: CardState, r: &Roles) -> Palette {
    let (fill, on) = match state {
        CardState::NeedsAction => (r.error_container, r.on_error_container),
        CardState::NeedsAttention => (r.warning_container, r.on_warning_container),
        CardState::Working => (r.primary_container, r.on_primary_container),
        CardState::Done => (r.success_container, r.on_success_container),
        CardState::Idle => (r.surface_container_low, r.on_surface_variant),
    };
    Palette {
        fill,
        text: on,
        icon: on,
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

/// What the card says in words, apart from the layout that places it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The type icon.
    pub type_icon: Icon,
    /// The type's word: the footer's first words and the type icon's tooltip.
    pub type_label: &'static str,
    /// The name's tooltip: the whole display name, however much of it shows.
    pub name_tooltip: String,
    /// The state icon.
    pub state_icon: Icon,
    /// The state's word.
    pub state_label: &'static str,
    /// The reason, the state word's tooltip.
    pub reason_tooltip: &'static str,
    /// The footer line.
    pub footer: String,
}

/// What the card's progress row says (FR-016, FR-017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressWords {
    /// *Implement \u{b7} 6/10*, *6/10*, the phase alone, or *No task progress*.
    pub text: String,
    /// The ratio a bar is drawn for; `None` draws no bar.
    pub bar: Option<Ratio>,
}

/// The words of a card's progress row.
pub fn progress_words(progress: &Progress) -> ProgressWords {
    let text = match (progress.ratio, progress.phase) {
        (Some(ratio), Some(phase)) => {
            format!("{} \u{b7} {}/{}", phase.label(), ratio.done, ratio.total)
        }
        (Some(ratio), None) => format!("{}/{}", ratio.done, ratio.total),
        (None, Some(phase)) => phase.label().to_string(),
        (None, None) => "No task progress".to_string(),
    };
    ProgressWords {
        text,
        bar: progress.ratio,
    }
}

/// The activity line: what the agent is doing now, with its todo ratio when it has a list
/// (*Writing tests \u{b7} 2/5*). The task-list progress row stays below it (Story 5).
pub fn activity_words(activity: &CurrentActivity) -> String {
    match activity.ratio {
        Some(ratio) => format!("{} \u{b7} {}/{}", activity.line, ratio.done, ratio.total),
        None => activity.line.clone(),
    }
}

/// The words of `model`.
pub fn words(model: &CardModel) -> Words {
    Words {
        type_icon: type_icon(model.kind),
        type_label: model.kind.label(),
        name_tooltip: model.display_name.clone(),
        state_icon: state_icon(model.state),
        state_label: model.state.label(),
        reason_tooltip: model.reason.label(),
        footer: format!(
            "{} · {} sessions, {} running",
            model.kind.label(),
            model.sessions.0,
            model.sessions.1
        ),
    }
}

/// The card. See the module documentation.
#[derive(Clone)]
pub struct OverviewCard<'a, M> {
    model: CardModel,
    roles: Roles,
    on_press: Option<M>,
    id: Option<iced::advanced::widget::Id>,
    _marker: PhantomData<&'a M>,
}

/// The card's corner radius, shared by its fill and its focus ring.
const RADIUS: f32 = 12.0;

impl<'a, M: 'a> OverviewCard<'a, M> {
    /// The card of `model`, themed by `roles`.
    pub fn new(model: &CardModel, roles: Roles) -> Self {
        Self {
            model: model.clone(),
            roles,
            on_press: None,
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
        let glyph =
            |icon: Icon| Element::from(Glyph::<M>::new(icon, TypeRole::Title, roles).tint(p.icon));
        let header = row![
            Tooltip::new(glyph(w.type_icon), w.type_label, roles),
            Tooltip::new(
                Ellipsized::<M>::at_role(card.model.display_name.clone(), TypeRole::Title, p.text),
                w.name_tooltip,
                roles
            ),
            Tooltip::new(
                row![
                    glyph(w.state_icon),
                    Text::new(w.state_label, TypeRole::Label, roles).tint(p.text)
                ]
                .spacing(spacing::XS)
                .align_y(Alignment::Center),
                w.reason_tooltip,
                roles
            ),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
        let mut body = column![header].spacing(spacing::XS);
        if let Some(branch) = &card.model.branch {
            body = body.push(Ellipsized::<M>::at_role(
                branch.clone(),
                TypeRole::Caption,
                p.text,
            ));
        }
        body = body
            .push(Ellipsized::<M>::at_role(
                card.model.summary.clone(),
                TypeRole::Body,
                p.text,
            ))
            .push(Text::new(w.footer, TypeRole::Caption, roles).tint(p.text));
        if let Some(activity) = &card.model.activity {
            body = body.push(Ellipsized::<M>::at_role(
                activity_words(activity),
                TypeRole::Caption,
                p.text,
            ));
        }
        let row = progress_words(&card.model.progress);
        body = body.push(Text::new(row.text, TypeRole::Caption, roles).tint(p.text));
        if let Some(ratio) = row.bar {
            body = body.push(ProgressBar::new(ratio, roles));
        }
        if let Some((text, tooltip)) = card.model.prs.footer() {
            let line = Text::new(text, TypeRole::Caption, roles).tint(p.text);
            body = match tooltip {
                Some(reason) => body.push(Tooltip::new(line, reason, roles)),
                None => body.push(line),
            };
        }
        let face = container(body)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .padding(spacing::MD)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(style::color(p.fill))),
                border: iced::Border {
                    radius: RADIUS.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            });
        let Some(message) = card.on_press else {
            return face.into();
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
        let focusable = focusable
            .key(keyboard::key::Named::Enter, message.clone())
            .key(keyboard::key::Named::Space, message)
            .indicator(Indicator {
                outline: roles.secondary,
                layer: p.text,
                radius: RADIUS,
            });
        // Outermost, so the ripple is the card's only state layer while it runs (feature 971, D1).
        Ripple::new(focusable, p.text, RADIUS)
            .centred_on_keys()
            .into()
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
    use micold_core::task_progress::Phase;
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
            summary: "Adds the worktree overview".into(),
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

    /// FR-016, FR-017: the row has a bar only with a ratio; without one it says so in words.
    #[test]
    fn the_progress_row_has_a_bar_only_with_a_ratio() {
        let with = with_progress(Some((6, 10)), Some(Phase::Implement));
        assert_eq!(bars(&with, 280.0), 1);
        assert_eq!(progress_words(&with.progress).text, "Implement \u{b7} 6/10");
        let phase_only = with_progress(None, Some(Phase::Pr));
        assert_eq!(bars(&phase_only, 280.0), 0);
        assert_eq!(progress_words(&phase_only.progress).text, "PR");
        let none = with_progress(None, None);
        assert_eq!(bars(&none, 280.0), 0);
        let words = progress_words(&none.progress);
        assert_eq!((words.text.as_str(), words.bar), ("No task progress", None));
    }

    /// FR-016: the bar fits the card at 280 px with every other row present, and the card keeps
    /// its one height.
    #[test]
    fn the_card_with_every_row_fits_its_height() {
        let mut full = with_progress(Some((6, 10)), Some(Phase::Implement));
        full.prs = PrDisplay::Unknown(micold_core::overview::PrUnknown::Unreachable);
        full.activity = Some(micold_core::hook_activity::CurrentActivity {
            line: "A long activity line that has to ellipsize rather than wrap in the card"
                .repeat(2),
            ratio: Some(micold_core::task_progress::Ratio { done: 3, total: 7 }),
        });
        let node = super::super::test_support::laid_out(card(&full), Size::new(280.0, 400.0));
        assert_eq!(node.size().height, HEIGHT);
        for part in leaves(&node) {
            assert!(
                part.y + part.height <= HEIGHT - spacing::MD + 0.01,
                "{part:?} runs into the card's padding"
            );
        }
    }

    fn card(model: &CardModel) -> Element<'static, ()> {
        OverviewCard::<()>::new(model, tokens::roles(ColorScheme::Light)).into()
    }

    /// U4: the card is one height whatever its state.
    #[test]
    fn the_card_has_one_height_in_all_five_states() {
        let heights: Vec<f32> = CardState::ALL
            .into_iter()
            .map(|s| {
                super::super::test_support::size_of(
                    card(&model(s, "feat-a")),
                    Size::new(400.0, 400.0),
                )
                .height
            })
            .collect();
        assert!(heights.iter().all(|h| *h == HEIGHT), "{heights:?}");
    }

    /// U4: at 280 px no two pieces of the card overlap and none leaves it, even with a long name.
    #[test]
    fn no_text_overlaps_at_280_and_a_long_name_ellipsizes() {
        let long = "a-display-name-that-is-far-too-long-to-fit-in-the-card ".repeat(4);
        for state in CardState::ALL {
            for name in ["feat-a", long.as_str()] {
                let node = super::super::test_support::laid_out(
                    card(&model(state, name)),
                    Size::new(280.0, 400.0),
                );
                let parts = leaves(&node);
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
                assert_eq!(node.size().width, 280.0);
            }
        }
    }

    /// U4: the full name is the tooltip of the ellipsized name.
    #[test]
    fn the_full_display_name_is_in_the_tooltip() {
        let long = "x".repeat(200);
        assert_eq!(words(&model(CardState::Idle, &long)).name_tooltip, long);
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
        assert_eq!(
            words(&model(CardState::Idle, "n")).footer,
            "Feature · 3 sessions, 2 running",
            "FR-002: the type's word is visible text on the card, not only a tooltip"
        );
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
        // Mid-top edge, inside the stroke (centred half a ring width in), which antialiasing
        // blends with the fill: so closeness, not equality.
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

    /// T003 (US1-4, FR-003): mid-ripple the pixel at the press point has moved from the fill
    /// toward the state's content colour, and no further than it.
    #[test]
    fn the_ripple_is_drawn_in_the_states_content_colour() {
        let roles = tokens::roles(ColorScheme::Light);
        for state in CardState::ALL {
            let p = palette(state, &roles);
            let mut m = activatable(&model(state, "feat-a"));
            let at = Point::new(20.0, 100.0);
            m.press(at);
            m.tree
                .state
                .downcast_mut::<RippleState>()
                .advance(EXPAND, FADE);
            let (pixels, width) = m.drawn(iced::Color::BLACK);
            let i = ((at.y as u32 * width + at.x as u32) * 4) as usize;
            let got = [pixels[i], pixels[i + 1], pixels[i + 2]];
            let (fill, text) = (
                [p.fill.r, p.fill.g, p.fill.b],
                [p.text.r, p.text.g, p.text.b],
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
        let at = Point::new(20.0, 100.0);
        m.press(at);
        assert!(!ripple(&m).is_idle());
        settle(&mut m);
        assert!(ripple(&m).is_idle());
        let (pixels, width) = m.drawn(iced::Color::BLACK);
        let i = ((at.y as u32 * width + at.x as u32) * 4) as usize;
        let got = [pixels[i], pixels[i + 1], pixels[i + 2]];
        for (c, want) in [p.fill.r, p.fill.g, p.fill.b].into_iter().enumerate() {
            assert!(got[c].abs_diff(want) <= 1, "channel {c}: {got:?} vs fill");
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
