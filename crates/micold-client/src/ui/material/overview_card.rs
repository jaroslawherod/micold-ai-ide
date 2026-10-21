//! `OverviewCard`: one location of the project Overview (feature 813, FR-005, FR-006, FR-021).
//!
//! A header row in the state's colour, with the state icon and word at its start (the reason is
//! their tooltip) and the location type's icon and word at its end (the word is their tooltip),
//! over one neutral body: the title (on up to three lines), the
//! short branch when it has one, the sessions line, and the activity (up to two lines), progress
//! and pull request rows when they have something to say (feature 1120, contracts K1). What each
//! line says is [`CardText`], tested in the core (FR-018). Every colour comes from the theme's roles through [`palette`]: the header
//! of Needs action is the `error` container, Needs attention `warning`, Working `primary`, Done
//! `success`, and Idle's is the grey `surface_variant`; the body is `surface_container_low` in
//! every state. With a press (`on_press`) the card is a control: a
//! click, or Enter or Space on the focused card, publishes the message, and it takes the keyboard
//! like a `Button` (research R13); without one it is informational and not focusable.

use crate::icons::Icon;
use crate::ui::cdk::context_area::ContextArea;
use crate::ui::cdk::control_overlay::ControlOverlay;
use crate::ui::material::keyboard_focus::{Indicator, TakesTheKeyboard};
use crate::ui::material::line_clamp::LineClamped;
use crate::ui::material::{
    style, Ellipsized, Glyph, ProgressBar, Ripple, Scrollable, Text, TextLink, Tooltip, TypeRole,
};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, stack, Space};
use iced::{keyboard, Alignment, Element, Length};
use micold_core::overview::{CardModel, CardState, LocationType};
use micold_core::overview_text::{CardText, ExpandedText};
use micold_core::tokens::{elevation, spacing, state, Rgb, Roles};
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

/// What the [`CardGrid`](super::CardGrid) reads of a card: [`OverviewCard::rest_messages`].
pub(crate) struct RestMessages<M> {
    /// The card's location.
    pub at: micold_core::session::SessionLocation,
    /// [`OverviewCard::on_rest`].
    pub on_rest: Option<M>,
    /// [`OverviewCard::on_leave`].
    pub on_leave: Option<M>,
    /// [`OverviewCard::on_link_rest`].
    pub on_link_rest: Option<M>,
    /// The id of the link the card shows, when it shows one.
    pub link_id: Option<iced::advanced::widget::Id>,
}

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
    expanded: bool,
    scroll_id: Option<iced::advanced::widget::Id>,
    on_body_scroll: Option<Rc<dyn Fn(bool, bool) -> M + 'a>>,
    on_rest: Option<M>,
    on_leave: Option<M>,
    on_keyboard_focus: Option<Rc<dyn Fn(bool) -> M + 'a>>,
    on_page: Option<Rc<dyn Fn(bool) -> Option<M> + 'a>>,
    on_issue_link: Option<M>,
    link_id: Option<iced::advanced::widget::Id>,
    on_link_rest: Option<M>,
    on_link_keyboard_focus: Option<Rc<dyn Fn(bool) -> M + 'a>>,
    on_link_menu_key: Option<M>,
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
            expanded: false,
            scroll_id: None,
            on_body_scroll: None,
            on_rest: None,
            on_leave: None,
            on_keyboard_focus: None,
            on_page: None,
            on_issue_link: None,
            link_id: None,
            on_link_rest: None,
            on_link_keyboard_focus: None,
            on_link_menu_key: None,
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

    /// Draw the card in its expanded form (feature 1121, research R3, contract U1): the header
    /// row and the title wrapped in full, then an inner scrollable with the display name (when it
    /// differs from the title), the full branch (when the title is not the branch), the worktree
    /// directory, the sessions line, the full activity, progress and pull requests, every line
    /// wrapped with no ellipsis; the card casts the menu elevation's shadow. The wrappers around
    /// the face (context area, ripple, keyboard, button) are those of the collapsed card, so focus
    /// and ripple carry over an expand and a collapse (FR-011).
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Name the expanded card's inner scrollable, so the application can page it (FR-008).
    pub fn scroll_id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.scroll_id = Some(id);
        self
    }

    /// Publish `f(up, down)` whenever the expanded card's inner scrollable moves or is laid out:
    /// whether content lies above and below its view (feeds `body_edges`, contract K4).
    pub fn on_body_scroll(mut self, f: impl Fn(bool, bool) -> M + 'a) -> Self {
        self.on_body_scroll = Some(Rc::new(f));
        self
    }

    /// Publish `message` when the pointer has rested on the card for
    /// [`micold_core::overview_expansion::EXPAND_REST`] (feature 1121, FR-002, research R5). The
    /// [`CardGrid`](super::CardGrid) holding the card keeps the timer; a card on its own publishes
    /// nothing.
    pub fn on_rest(mut self, message: M) -> Self {
        self.on_rest = Some(message);
        self
    }

    /// Publish `message` when the pointer leaves the card after resting on it (feature 1121,
    /// FR-005): published by the [`CardGrid`](super::CardGrid), as [`Self::on_rest`].
    pub fn on_leave(mut self, message: M) -> Self {
        self.on_leave = Some(message);
        self
    }

    /// The card's location, its rest messages and its link's id, for the grid's rest timer, which
    /// follows the card by its location, and for its press routing (feature 1122, R7, R8).
    pub(crate) fn rest_messages(&self) -> RestMessages<M>
    where
        M: Clone,
    {
        RestMessages {
            at: self.model.location.clone(),
            on_rest: self.on_rest.clone(),
            on_leave: self.on_leave.clone(),
            on_link_rest: self.on_link_rest.clone(),
            link_id: self
                .on_issue_link
                .as_ref()
                .and(self.model.issue_link.as_ref())
                .and(self.link_id.clone()),
        }
    }

    /// This card, as the collapsed twin of `expanded` in the [`CardGrid`](super::CardGrid): laid
    /// out with the link `expanded` shows, so the twin is as tall as the collapsed card it stands
    /// for (feature 1121 FR-009). The twin is only measured, so the link's message is never sent.
    pub(crate) fn twin_of(mut self, expanded: &Self) -> Self
    where
        M: Clone,
    {
        self.on_issue_link = expanded.on_issue_link.clone();
        self
    }

    /// Publish `f(true)` when the card gets keyboard focus (Tab, Shift+Tab, an arrow move, a
    /// `focus(id)` hand-back) and `f(false)` when it loses it; a pointer press that takes the
    /// keyboard publishes nothing (feature 1121, U1, FR-002).
    pub fn on_keyboard_focus(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_keyboard_focus = Some(Rc::new(f));
        self
    }

    /// Claim Page Down (`f(true)`) and Page Up (`f(false)`) while the card holds the keyboard,
    /// each only when `f` gives a message for it: the application answers with a page of the
    /// expanded card's body, and a direction it declines goes on as today (feature 1121, U1,
    /// FR-008, contract K4).
    pub fn on_page(mut self, f: impl Fn(bool) -> Option<M> + 'a) -> Self {
        self.on_page = Some(Rc::new(f));
        self
    }

    /// Show `model.issue_link` on the branch line as a link publishing `message` when clicked, or
    /// activated by key while it holds the keyboard (feature 1122, contract U3).
    pub fn on_issue_link(mut self, message: M) -> Self {
        self.on_issue_link = Some(message);
        self
    }

    /// Name the link, so the grid can find its bounds and the application can focus it.
    pub fn link_id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.link_id = Some(id);
        self
    }

    /// Publish `message` when the pointer has rested on the link for
    /// [`micold_core::overview_expansion::EXPAND_REST`]: kept by the [`CardGrid`](super::CardGrid),
    /// as [`Self::on_rest`] (research R8).
    pub fn on_link_rest(mut self, message: M) -> Self {
        self.on_link_rest = Some(message);
        self
    }

    /// Make the link a Tab stop right after the card, publishing `f(true)` when keyboard focus
    /// arrives on it and `f(false)` when it leaves (FR-008). Without it the link is no Tab stop,
    /// and a click still opens it (contract U3).
    pub fn on_link_keyboard_focus(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_link_keyboard_focus = Some(Rc::new(f));
        self
    }

    /// Publish `message` for the Menu key or Shift+F10 while the link holds the keyboard (feature
    /// 1122, FR-009): the card's menu, opened from its link. The link's Page Up / Page Down are the
    /// card's [`Self::on_page`] (R9).
    pub fn on_link_menu_key(mut self, message: M) -> Self {
        self.on_link_menu_key = Some(message);
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
        // A glyph and its word, in the header's colour, carrying `tooltip`. The word shortens with
        // an ellipsis when a narrow card leaves it too little room; the state's, laid out first,
        // keeps its width before the type's gives way.
        let cluster = |icon: Icon, word: &'static str, tooltip: &'static str| {
            Tooltip::new(
                row![
                    Glyph::<M>::new(icon, TypeRole::Label, roles).tint(p.on_header),
                    Ellipsized::<M>::at_role_spaced(word, TypeRole::Label, p.on_header).shrink()
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
        let content: Element<'a, M> = if card.expanded {
            expanded_body(&card, p)
        } else {
            collapsed_body(&card, p)
        };
        let targeted = card.targeted;
        let shadow = if card.expanded {
            style::elevation_shadow(roles, elevation::MENU)
        } else {
            iced::Shadow::default()
        };
        let fill = container(column![header, content])
            .width(Length::Fill)
            .style(move |_| container::Style {
                shadow,
                background: Some(iced::Background::Color(style::color(p.body))),
                border: iced::Border {
                    radius: RADIUS.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            });
        // The target mark is the top layer, so the header's fill cannot paint over the outline
        // (research R13); its *selected* layer covers the header and the body alike. Only a
        // targeted card stacks it, so no other card opens a renderer layer for nothing.
        let face: Element<'a, M> = if targeted {
            let mark =
                container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| {
                    let mut layer = style::color(p.on_body);
                    layer.a = state::SELECTED;
                    container::Style {
                        background: Some(iced::Background::Color(layer)),
                        border: iced::Border {
                            radius: RADIUS.into(),
                            color: style::color(roles.primary),
                            width: TARGET_WIDTH,
                        },
                        ..container::Style::default()
                    }
                });
            stack![fill, mark].width(Length::Fill).into()
        } else {
            fill.into()
        };
        let secondary = card.on_secondary_press;
        let menu = move |content: Element<'a, M>| match secondary {
            Some(f) => ContextArea::new(content)
                .on_secondary_press(move |at| f(at))
                .into(),
            None => content,
        };
        let Some(message) = card.on_press else {
            return menu(face);
        };
        // The card's face is drawn by the overlay below; the button over it draws nothing.
        let pressable = button(Space::new().width(Length::Fill).height(Length::Fill))
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fill)
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
            if let Some(page) = &card.on_page {
                for (key, down) in [
                    (keyboard::key::Named::PageDown, true),
                    (keyboard::key::Named::PageUp, false),
                ] {
                    // Unmodified only: Ctrl/Cmd+Shift+Page keys are the project chords (keymap.rs).
                    if let Some(message) = page(down) {
                        focusable = focusable.chord(key, keyboard::Modifiers::empty(), message);
                    }
                }
            }
        }
        if let Some(f) = card.on_keyboard_focus {
            focusable = focusable.on_shown_change(move |shown| f(shown));
        }
        let focusable = focusable.indicator(Indicator {
            outline: roles.secondary,
            layer: p.on_body,
            radius: RADIUS,
        });
        // The ripple is the card's only state layer while it runs (feature 971, D1); the context
        // area outside it takes only the secondary press, which the ripple ignores (feature 972).
        // The control chain is a layer over the face, which sees every event first: a press its
        // link captures never reaches the card, and the card neither hovers nor activates over the
        // link (feature 1122, R6). Focus order: the card, then its link.
        let control = Ripple::new(focusable, p.on_body, RADIUS).centred_on_keys();
        menu(ControlOverlay::new(face, control).into())
    }
}

/// The card's issue link (feature 1122, contract U3): `model.issue_link` drawn as a [`TextLink`] at
/// the branch line's role, only when [`OverviewCard::on_issue_link`] is set. A click on it leaves the
/// keyboard where it was (R7), and it is a Tab stop only once its focus reports are wired.
fn issue_link<'a, M: Clone + 'a>(card: &OverviewCard<'a, M>) -> Option<Element<'a, M>> {
    let (Some(link), Some(message)) = (&card.model.issue_link, &card.on_issue_link) else {
        return None;
    };
    let mut link = TextLink::new(link.label.clone(), message.clone(), card.roles)
        .role(TypeRole::Body)
        .inline(true)
        .focus_on_press(false)
        .focusable(card.on_link_keyboard_focus.is_some());
    if let Some(id) = card.link_id.clone() {
        link = link.id(id);
    }
    if let Some(f) = card.on_link_keyboard_focus.clone() {
        link = link.on_keyboard_focus(move |shown| f(shown));
    }
    // The card's own keys stand down while it is targeted or its menu is open; so do the link's.
    if !card.targeted && !card.menu_open {
        if let Some(menu) = card.on_link_menu_key.clone() {
            link = link.on_menu_key(menu);
        }
        if let Some(page) = card.on_page.clone() {
            link = link.on_page(move |down| page(down));
        }
    }
    Some(link.into())
}

/// The collapsed card's body, under its header (feature 1120, feature 1121 FR-001).
fn collapsed_body<'a, M: Clone + 'a>(card: &OverviewCard<'a, M>, p: Palette) -> Element<'a, M> {
    let roles = card.roles;
    let text = CardText::of(&card.model);
    // The title wraps onto three lines and the activity onto two before an ellipsis (feature
    // 1121, FR-001); the branch stays one line. None of them has a tooltip: the expanded card
    // shows their full text (feature 1121, FR-004).
    let mut body = column![LineClamped::<M>::new(text.title, TypeRole::CardTitle, 3)
        .tint(p.on_body)
        .width(Length::Fill)]
    .spacing(spacing::XS);
    // The branch line (R11): the link at its natural width at the line's end, the short branch
    // ellipsized into what it leaves; alone in the line's place when there is no branch line.
    let link = issue_link(card);
    match (text.branch, link) {
        (Some(branch), link) => {
            let branch = Ellipsized::<M>::at_role_spaced(branch, TypeRole::Body, p.on_body_variant);
            body = match link {
                Some(link) => body.push(
                    row![branch, link]
                        .spacing(spacing::SM)
                        .align_y(Alignment::Center),
                ),
                None => body.push(branch),
            };
        }
        (None, Some(link)) => body = body.push(link),
        (None, None) => {}
    }
    body = body.push(Text::new(text.sessions, TypeRole::Body, roles).tint(p.on_body_variant));
    if let Some(activity) = text.activity {
        body = body.push(
            LineClamped::<M>::new(activity, TypeRole::Body, 2)
                .tint(p.on_body_variant)
                .width(Length::Fill),
        );
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
    container(body).padding(spacing::MD).into()
}

/// A body line of the expanded card: wrapped in full, at `role`, in `tint`.
fn wrapped<'a, M: 'a>(words: String, role: TypeRole, tint: Rgb, roles: Roles) -> Element<'a, M> {
    Text::new(words, role, roles)
        .tint(tint)
        .width(Length::Fill)
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

/// Whether content lies above and below the expanded card's inner view (contract K4), from the
/// scroll metrics, each rounded to a whole pixel on its own: a body at its true end can read a
/// pixel short of it, so a pixel's slack is the end (review A F4, M2).
fn body_edges(offset: u32, view: u32, content: u32) -> (bool, bool) {
    (
        offset > 0,
        offset.saturating_add(view).saturating_add(1) < content,
    )
}

/// The user's home directory, looked up once: the expanded body shortens the worktree directory
/// against it on every view build (review B F3, M2).
fn home() -> Option<&'static std::path::Path> {
    static HOME: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    HOME.get_or_init(|| directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()))
        .as_deref()
}

/// The expanded card's body, under its header (feature 1121, research R3): the title outside the
/// inner scrollable, so it stays in view (FR-008), and every other line inside it.
fn expanded_body<'a, M: Clone + 'a>(card: &OverviewCard<'a, M>, p: Palette) -> Element<'a, M> {
    let roles = card.roles;
    let full = ExpandedText::of(&card.model, home());
    let text = CardText::of(&card.model);
    let body_line = |words: String| wrapped::<M>(words, TypeRole::Body, p.on_body_variant, roles);
    let mut rest = column![].spacing(spacing::XS);
    let link = issue_link(card);
    // A link that is the body's first row has the room its focus gutter spills into above it, so
    // the inner scrollable does not clip its ring (feature 1122, review A M2 F4).
    let link_first = link.is_some() && full.display_name.is_none() && full.branch.is_none();
    if let Some(name) = full.display_name {
        rest = rest.push(body_line(name));
    }
    // The branch line wraps in full with the link at its end, on its first line (R11); alone in
    // the line's place when there is no branch line.
    match (full.branch, link) {
        (Some(branch), Some(link)) => {
            rest = rest.push(row![body_line(branch), link].spacing(spacing::SM));
        }
        (Some(branch), None) => rest = rest.push(body_line(branch)),
        (None, Some(link)) => rest = rest.push(link),
        (None, None) => {}
    }
    rest = rest.push(body_line(full.dir));
    rest = rest.push(body_line(text.sessions));
    if let Some(activity) = full.activity {
        rest = rest.push(body_line(activity));
    }
    if let Some(row) = text.progress {
        rest = rest.push(body_line(row.text));
        if let Some(ratio) = row.bar {
            rest = rest.push(ProgressBar::new(ratio, roles));
        }
    }
    if let Some((words, _)) = text.prs {
        rest = rest.push(body_line(words));
    }
    // The card's right edge is the grid's, where the grid's own bar floats when it scrolls (one
    // column, or the last): the inner bar keeps clear of it, taking that strip from the text's
    // right padding, so the text keeps its room (M4 visual re-look r2).
    let clear = super::scrollable::BAR_FOOTPRINT;
    let mut scroll = Scrollable::new(
        container(rest).padding(iced::Padding {
            top: if link_first {
                spacing::XS.max(super::text_link::GUTTER)
            } else {
                spacing::XS
            },
            right: spacing::MD - clear,
            bottom: spacing::MD,
            left: spacing::MD,
        }),
        roles,
    );
    if let Some(id) = card.scroll_id.clone() {
        scroll = scroll.id(id);
    }
    if let Some(f) = card.on_body_scroll.clone() {
        scroll = scroll.on_scroll_metrics(move |offset, view, content| {
            let (up, down) = body_edges(offset, view, content);
            f(up, down)
        });
    }
    column![
        container(wrapped::<M>(
            full.title,
            TypeRole::CardTitle,
            p.on_body,
            roles
        ))
        .padding(iced::Padding {
            top: spacing::MD,
            right: spacing::MD,
            bottom: 0.0,
            left: spacing::MD,
        }),
        container(Element::from(scroll)).padding(iced::Padding {
            right: clear,
            ..iced::Padding::ZERO
        }),
    ]
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{leaves, overlap};
    use super::super::test_support::{named_key, roles, Focusables, Mounted};
    use super::*;
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
            dir: std::path::PathBuf::from("/repo/.worktrees/x"),
            issue_link: None,
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

    /// FR-006, FR-007, FR-008, SC-003: the issue title, the short branch and the sessions line;
    /// the display name and the full branch only on the expanded card (feature 1121, FR-004).
    #[test]
    fn the_card_shows_the_title_the_short_branch_and_the_sessions_line() {
        let m = titled();
        let text = painted(&m);
        assert!(
            text.contains(&"Overview cards: readable layout".to_string()),
            "the issue title is shown whole when it fits: {text:?}"
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
        Mounted::new_in(
            container(OverviewCard::<String>::new(model, roles())).width(Length::Fixed(width)),
            Size::new(width, 1000.0),
        )
        .painted_text()
    }

    /// FR-014, FR-015, US4.1, SC-002: with every optional row, short or long, at 280, 380 and
    /// 480 px, no two pieces of the card overlap, every piece sits inside the card, and the card
    /// is at least `MIN_HEIGHT` tall.
    #[test]
    fn a_card_with_every_row_fits_at_every_card_width() {
        for m in [full(false), full(true)] {
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
            }
        }
    }

    /// FR-015: at every card width a long title, branch and activity end in an ellipsis.
    #[test]
    fn a_long_title_branch_and_activity_end_in_an_ellipsis_at_every_width() {
        let m = full(true);
        for width in WIDTHS {
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

    /// How many lines `text` needs at `role` laid out `room` wide, with nothing limiting it.
    fn lines_needed(text: &str, role: TypeRole, room: f32) -> usize {
        let (_, size) = super::super::line_clamp::tests::laid_out_at(text, role, 50, room);
        (size.height / role.line_height_dp()).round() as usize
    }

    /// Words that need exactly `lines` lines at `role` in a card `width` wide: the shortest run of
    /// them that wraps onto that many.
    fn needing(lines: usize, role: TypeRole, width: f32) -> String {
        let room = width - 2.0 * spacing::MD;
        let mut text = String::new();
        for word in "Overview cards wrap the title onto three lines and the activity onto two"
            .split(' ')
            .cycle()
            .take(400)
        {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(word);
            if lines_needed(&text, role, room) == lines {
                return text;
            }
        }
        panic!("no run of words needs {lines} lines at {width}");
    }

    /// The card `width` wide with `title` and `activity`, and what it paints: each paragraph and
    /// the card's own size.
    fn card_with(
        title: &str,
        activity: &str,
        width: f32,
    ) -> (Vec<super::super::test_support::Painted>, Size) {
        let mut m = model(CardState::Working, "feat-a");
        m.title = title.into();
        m.activity = Some(micold_core::hook_activity::CurrentActivity {
            line: activity.into(),
            ratio: None,
        });
        let mut mounted = Mounted::new_in(
            container(OverviewCard::<String>::new(&m, roles())).width(Length::Fixed(width)),
            Size::new(width, 1000.0),
        );
        let painted = mounted.painted_text();
        (painted, mounted.node.size())
    }

    /// The painted paragraph that starts `text`'s first word run, and how many `role` lines it
    /// takes.
    fn lines_of(
        painted: &[super::super::test_support::Painted],
        start: &str,
        role: TypeRole,
    ) -> (String, usize) {
        let p = painted
            .iter()
            .find(|p| p.content.starts_with(start))
            .unwrap_or_else(|| panic!("no {start:?} paragraph: {painted:?}"));
        (
            p.content.clone(),
            (p.bounds.height / role.line_height_dp()).round() as usize,
        )
    }

    /// Feature 1121 US1 sc.1, sc.2, sc.4, FR-001: at 280, 380 and 480 px a title needing 1, 2 or
    /// 3 lines takes that many in full; one needing 5 takes 3, the third ending in an ellipsis.
    #[test]
    fn the_title_wraps_onto_up_to_three_lines_then_ends_in_an_ellipsis() {
        for width in WIDTHS {
            for (needs, takes) in [(1, 1), (2, 2), (3, 3), (5, 3)] {
                let title = needing(needs, TypeRole::CardTitle, width);
                let (painted, _) = card_with(&title, "Editing card_grid.rs", width);
                let (shown, lines) = lines_of(&painted, "Overview", TypeRole::CardTitle);
                assert_eq!(lines, takes, "{width}: a title needing {needs}: {shown:?}");
                if needs <= 3 {
                    assert_eq!(shown, title, "{width}: shown in full");
                } else {
                    assert!(shown.ends_with('\u{2026}'), "{width}: {shown:?}");
                }
            }
        }
    }

    /// Feature 1121 US1 sc.3, sc.4, FR-001: an activity needing 1 or 2 lines takes that many in
    /// full; one needing 4 takes 2, the second ending in an ellipsis.
    #[test]
    fn the_activity_wraps_onto_up_to_two_lines_then_ends_in_an_ellipsis() {
        for width in WIDTHS {
            for (needs, takes) in [(1, 1), (2, 2), (4, 2)] {
                let activity = needing(needs, TypeRole::Body, width);
                let (painted, _) = card_with("Adds a card", &activity, width);
                let (shown, lines) = lines_of(&painted, "Overview", TypeRole::Body);
                assert_eq!(
                    lines, takes,
                    "{width}: an activity needing {needs}: {shown:?}"
                );
                if needs <= 2 {
                    assert_eq!(shown, activity, "{width}: shown in full");
                } else {
                    assert!(shown.ends_with('\u{2026}'), "{width}: {shown:?}");
                }
            }
        }
    }

    /// Feature 1121 FR-001: the short branch stays one line, ellipsized, however long it is.
    #[test]
    fn the_branch_stays_one_line_ellipsized() {
        let m = full(true);
        for width in WIDTHS {
            let (shown, lines) = lines_of(&painted_at(&m, width), "topic-", TypeRole::Body);
            assert_eq!(lines, 1, "{width}: {shown:?}");
            assert!(shown.ends_with('\u{2026}'), "{width}: {shown:?}");
        }
    }

    /// Feature 1121 FR-007, SC-002: with a five-line title and a four-line activity, no painted
    /// text leaves the card at 280, 380 or 480 px.
    #[test]
    fn no_wrapped_text_leaves_the_card() {
        for width in WIDTHS {
            let title = needing(5, TypeRole::CardTitle, width);
            let activity = needing(4, TypeRole::Body, width);
            let (painted, card) = card_with(&title, &activity, width);
            for p in &painted {
                let b = p.bounds;
                assert!(
                    b.x >= -0.01
                        && b.y >= -0.01
                        && b.x + b.width <= card.width + 0.01
                        && b.y + b.height <= card.height + 0.01,
                    "{width}: {p:?} leaves the card {card:?}"
                );
            }
        }
    }

    /// A card whose every text is longer than any card is wide: a long title, a display name
    /// apart from it, and an unbroken 120-character branch and directory.
    fn overlong() -> CardModel {
        let mut m = full(true);
        m.display_name = "feat-1121_overview-card-full-text".into();
        m.branch = Some(format!("feat/{}", "b".repeat(115)));
        m.dir = std::path::PathBuf::from(format!("/{}", "d".repeat(119)));
        m
    }

    /// What `model`'s expanded card paints `width` wide, and the card's size.
    fn expanded_at(
        model: &CardModel,
        width: f32,
        roles: Roles,
    ) -> (Vec<super::super::test_support::Painted>, Size) {
        let mut mounted = Mounted::new_in(
            container(
                OverviewCard::<String>::new(model, roles)
                    .on_press("open".to_string())
                    .expanded(true),
            )
            .width(Length::Fixed(width)),
            Size::new(width, 4000.0),
        );
        let painted = mounted.painted_text();
        (painted, mounted.node.size())
    }

    /// Feature 1121 U1, FR-004, FR-007, US2 sc.1, sc.7: at 280, 380 and 480 px the expanded card
    /// shows the full title, the display name, the full branch, the worktree directory, the
    /// sessions line, the full activity, progress and pull requests, none ellipsized, every line
    /// inside the card.
    #[test]
    fn an_expanded_card_shows_every_text_in_full_inside_it() {
        let m = overlong();
        let words = micold_core::overview_text::ExpandedText::of(&m, None);
        for width in WIDTHS {
            let (painted, card) = expanded_at(&m, width, roles());
            let shown: Vec<&str> = painted.iter().map(|p| p.content.as_str()).collect();
            for want in [
                words.title.as_str(),
                words.display_name.as_deref().expect("a display name"),
                words.branch.as_deref().expect("a branch"),
                words.dir.as_str(),
                words.activity.as_deref().expect("an activity"),
                "3 sessions \u{b7} 2 running",
                "Implement \u{b7} 6/10",
            ] {
                assert!(shown.contains(&want), "{width}: {want:?} not in {shown:?}");
            }
            for p in &painted {
                assert!(!p.content.ends_with('\u{2026}'), "{width}: {p:?}");
                let b = p.bounds;
                assert!(
                    b.x >= -0.01
                        && b.x + b.width <= card.width + 0.01
                        && b.y + b.height <= card.height + 0.01,
                    "{width}: {p:?} leaves the card {card:?}"
                );
            }
        }
    }

    /// Feature 1121 FR-015: in every state and both schemes each text of the expanded card is
    /// in the colour the same text has on the collapsed card.
    #[test]
    fn an_expanded_cards_texts_keep_the_collapsed_cards_colours() {
        for r in [roles(), super::super::test_support::dark_roles()] {
            for state in CardState::ALL {
                let m = model(state, "feat-a");
                let collapsed = mounted_at(&m, 380.0, r).painted_text();
                let (expanded, _) = expanded_at(&m, 380.0, r);
                // The short branch is the one text the expanded card replaces (by the full one).
                let mut same = 0;
                for p in collapsed.iter().filter(|p| p.content != "feat/813") {
                    let twin = expanded
                        .iter()
                        .find(|e| e.content == p.content)
                        .unwrap_or_else(|| panic!("{state:?}: no {:?} expanded", p.content));
                    assert_eq!(twin.color, p.color, "{state:?}: {:?}", p.content);
                    same += 1;
                }
                assert_eq!(same, collapsed.len() - 1, "{state:?}: {collapsed:?}");
                let full = expanded
                    .iter()
                    .find(|e| e.content == "feat/813-overview")
                    .expect("the full branch");
                let short = collapsed.iter().find(|p| p.content == "feat/813");
                assert_eq!(Some(full.color), short.map(|p| p.color), "{state:?}");
            }
        }
    }

    /// Feature 1121 FR-011, US4 sc.1: the expanded card takes the keyboard, and a click or Enter
    /// on it publishes its message as on the collapsed card.
    #[test]
    fn an_expanded_card_activates_as_the_collapsed_one() {
        use iced::advanced::mouse::{Button, Cursor, Event as Mouse};
        let mut card = Mounted::new(
            OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                .on_press("open".to_string())
                .expanded(true),
        );
        assert_eq!(card.drive(Focusables(0)), Some(1));
        let at = card.node.bounds().center();
        card.press(at);
        let released = card.send(
            iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            Cursor::Available(at),
        );
        assert_eq!(released, ["open"]);
        assert_eq!(
            card.send(named_key(Named::Enter), Cursor::Unavailable),
            ["open"],
            "the press gave the expanded card the keyboard"
        );
    }

    /// `model`'s expanded card, `width` wide and `height` tall, publishing `"<up> <down>"` on a
    /// scroll of its inner body.
    fn expanded_in(model: &CardModel, width: f32, height: f32) -> Mounted<'static> {
        Mounted::new_in(
            container(
                OverviewCard::<String>::new(model, roles())
                    .on_press("open".to_string())
                    .on_body_scroll(|up, down| format!("{up} {down}"))
                    .expanded(true),
            )
            .width(Length::Fixed(width))
            .height(Length::Fixed(height)),
            Size::new(width, height),
        )
    }

    /// A wheel turn of `dy` pixels (negative: toward the end) with the pointer at `at`.
    fn wheel(m: &mut Mounted<'_>, at: Point, dy: f32) -> Vec<String> {
        use iced::advanced::mouse::{Cursor, Event as Mouse, ScrollDelta};
        m.send(
            iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Pixels { x: 0.0, y: dy },
            }),
            Cursor::Available(at),
        )
    }

    /// Where `m` paints the paragraph that says `content`.
    fn top_of(m: &mut Mounted<'_>, content: &str) -> f32 {
        m.painted_text()
            .into_iter()
            .find(|p| p.content == content)
            .unwrap_or_else(|| panic!("{content:?} is not painted"))
            .bounds
            .y
    }

    /// Feature 1121 FR-008, R3: the header row and the title lie outside the inner scrollable:
    /// with the body scrolled to its end, its first line is scrolled out of view and no longer
    /// painted, while the header and the title are painted where they were.
    #[test]
    fn an_expanded_cards_header_and_title_stay_while_its_body_scrolls() {
        let m = overlong();
        let full = micold_core::overview_text::ExpandedText::of(&m, None);
        let mut card = expanded_in(&m, 280.0, 260.0);
        let stays = [full.title.as_str(), words(&m).state_label];
        let first = full.display_name.expect("a display name");
        let before = stays.map(|c| top_of(&mut card, c));
        assert!(card.painted_text().iter().any(|p| p.content == first));
        wheel(&mut card, Point::new(140.0, 250.0), -10_000.0);
        assert_eq!(
            stays.map(|c| top_of(&mut card, c)),
            before,
            "the header or title moved"
        );
        assert!(
            !card.painted_text().iter().any(|p| p.content == first),
            "the body's first line did not scroll out of view"
        );
    }

    /// Feature 1121 M4 visual re-look r2 (2x, one column): the card's right edge is the grid's,
    /// where the grid's own scrollbar floats, so the inner body's scrollbar sits clear of that
    /// bar's footprint and stays reachable; the body's text stays within its margins.
    #[test]
    fn an_expanded_cards_inner_scrollbar_clears_the_grids_scrollbar() {
        let (width, height) = (280.0, 260.0);
        let mut card = expanded_in(&overlong(), width, height);
        let [(viewport, _)] = card.scrollables()[..] else {
            panic!("the expanded card holds one inner scrollable");
        };
        assert!(
            viewport.x + viewport.width <= width - super::super::scrollable::BAR_FOOTPRINT + 0.01,
            "the inner bar lies under the grid's bar at the card's right edge: {viewport:?}"
        );
        // The text stays within the card's margins.
        for line in card.painted_text() {
            let b = line.bounds;
            assert!(b.x >= spacing::MD - 0.01, "{line:?}");
            assert!(b.x + b.width <= width - spacing::MD + 0.01, "{line:?}");
        }
        // The bar took its strip from the body's padding, not from its text: the activity wraps
        // exactly as body text does in the full room between the card's margins. An unbroken run
        // of a glyph narrower than the strip breaks at the last glyph that fits, so any lost
        // width shows.
        let mut m = overlong();
        let line = "i".repeat(300);
        let activity = m.activity.as_mut().expect("an activity");
        (activity.line, activity.ratio) = (line.clone(), None);
        let room = width - 2.0 * spacing::MD;
        let mut free = Mounted::new_in(
            container(wrapped::<String>(
                line.clone(),
                TypeRole::Body,
                roles().on_surface_variant,
                roles(),
            ))
            .width(Length::Fixed(room)),
            Size::new(room, 4000.0),
        );
        let wraps = |painted: Vec<super::super::test_support::Painted>| {
            painted
                .into_iter()
                .find(|p| p.content == line)
                .map(|p| p.bounds.width)
                .unwrap_or_else(|| panic!("{line:?} is not painted"))
        };
        // Precondition of the comparison: one `i` is narrower than the strip, so a strip lost
        // from the text's room always costs the run at least one glyph.
        let glyph = Mounted::new_in(
            container(wrapped::<String>(
                "i".to_string(),
                TypeRole::Body,
                roles().on_surface_variant,
                roles(),
            ))
            .width(Length::Fixed(room)),
            Size::new(room, 4000.0),
        )
        .painted_text()
        .into_iter()
        .find(|p| p.content == "i")
        .map(|p| p.bounds.width)
        .expect("one `i` is painted");
        assert!(
            glyph > 0.0 && glyph < super::super::scrollable::BAR_FOOTPRINT,
            "an `i` ({glyph} wide) is not narrower than the strip, so a lost strip could go unseen"
        );
        let (full_room, (in_body, _)) =
            (wraps(free.painted_text()), expanded_at(&m, width, roles()));
        let in_body = wraps(in_body);
        assert!(
            (full_room - in_body).abs() < 0.01,
            "the body's text lost width to the inner bar: it wraps {in_body} wide, not {full_room}"
        );
    }

    /// Feature 1121 K4: the inner body publishes `(up, down)` = `(true, false)` at its end and
    /// `(false, true)` back at its top; with nothing to scroll it publishes nothing, so the
    /// application's `body_edges` keep the `(false, false)` an expand starts them at.
    #[test]
    fn an_expanded_cards_body_publishes_its_scroll_edges() {
        let at = Point::new(140.0, 250.0);
        let mut card = expanded_in(&overlong(), 280.0, 260.0);
        assert_eq!(wheel(&mut card, at, -10_000.0), ["true false"]);
        assert_eq!(wheel(&mut card, at, 10_000.0), ["false true"]);
        let mut short = expanded_in(&model(CardState::Idle, "feat-a"), 380.0, 600.0);
        assert!(wheel(&mut short, at, -10_000.0).is_empty());
    }

    /// Review A F4 (M2): the metrics come rounded to whole pixels, each on its own, so a body at its
    /// true end can read up to a pixel short of it; it is at its end then, and Page Down falls
    /// through to the Overview (FR-008). Two pixels short it is not.
    #[test]
    fn a_body_a_rounding_short_of_its_end_is_at_its_end() {
        // 600.1 + 400.4 = 1000.5, rounded 600 + 400 and 1001.
        assert_eq!(body_edges(600, 400, 1001), (true, false));
        assert_eq!(body_edges(600, 400, 1000), (true, false));
        assert_eq!(body_edges(600, 400, 1002), (true, true));
        assert_eq!(body_edges(0, 400, 1000), (false, true));
    }

    /// The card of `model` expanded, `on_press` set, in a `width` column as tall as it needs.
    fn expanded_card(model: &CardModel, targeted: bool, width: f32) -> Mounted<'static> {
        Mounted::new_in(
            OverviewCard::<String>::new(model, tokens::roles(ColorScheme::Light))
                .on_press("pressed".to_string())
                .targeted(targeted)
                .expanded(true),
            Size::new(width, 4000.0),
        )
    }

    /// Feature 1121 FR-003: the expanded card casts the menu elevation's shadow below it; the
    /// collapsed card casts none.
    #[test]
    fn an_expanded_card_casts_the_menu_shadow() {
        let r = roles();
        let backdrop = style::color(r.surface);
        let m = model(CardState::Idle, "feat-a");
        let below = |expanded: bool| {
            let mut card = Mounted::new_in(
                container(
                    OverviewCard::<String>::new(&m, r)
                        .on_press("open".to_string())
                        .expanded(expanded),
                )
                .padding(24)
                .width(Length::Fixed(380.0)),
                Size::new(380.0, 4000.0),
            );
            let (pixels, width) = card.drawn(backdrop);
            let bottom = card.node.size().height as u32 - 24;
            let i = (((bottom + 4) * width + width / 2) * 4) as usize;
            pixels[i..i + 3].to_vec()
        };
        let plain = [backdrop.r, backdrop.g, backdrop.b].map(|c| (c * 255.0).round() as u8);
        assert_eq!(below(false), plain, "the collapsed card casts a shadow");
        assert_ne!(below(true), plain, "the expanded card casts no shadow");
    }

    /// Feature 1121 FR-011: the focus ring, the target mark and the ripple of an expanded card
    /// cover its whole drawn bounds: the ring and the mark reach its bottom edge, and a press in
    /// its bottom-right corner ripples from there.
    #[test]
    fn an_expanded_cards_ring_mark_and_ripple_cover_its_bounds() {
        let r = tokens::roles(ColorScheme::Light);
        let backdrop = style::color(r.surface);
        let m = overlong();
        let near = |want: Rgb, p: &[u8]| -> f32 {
            let want = style::color(want);
            [want.r, want.g, want.b]
                .iter()
                .zip(p)
                .map(|(w, got)| (w * 255.0 - f32::from(*got)).abs())
                .sum()
        };
        let mut card = expanded_card(&m, false, 280.0);
        let size = card.node.size();
        let (rest, width) = card.drawn(backdrop);
        let bottom = |row_from_end: u32| {
            (((size.height.ceil() as u32 - row_from_end) * width + width / 2) * 4) as usize
        };
        card.focus_next();
        let (focused, _) = card.drawn(backdrop);
        let i = bottom(2);
        assert!(
            near(r.secondary, &focused[i..i + 3]) + 60.0 < near(r.secondary, &rest[i..i + 3]),
            "no focus ring at the expanded card's bottom edge: {:?}",
            &focused[i..i + 3]
        );
        let (targeted, _) = expanded_card(&m, true, 280.0).drawn(backdrop);
        let i = bottom(1);
        assert!(
            near(r.primary, &targeted[i..i + 3]) < 12.0,
            "no target mark at the expanded card's bottom edge: {:?}",
            &targeted[i..i + 3]
        );
        let corner = Point::new(size.width - 0.5, size.height - 0.5);
        let mut pressed = expanded_card(&m, false, 280.0);
        pressed.press(corner);
        let origin = ripple(&pressed)
            .origin()
            .expect("the press started a ripple");
        assert!(
            (origin.x - corner.x).abs() <= 1.0 && (origin.y - corner.y).abs() <= 1.0,
            "pressed {corner:?}, rippled from {origin:?}"
        );
    }

    /// Feature 1121 FR-004: hovering the collapsed card's title, branch or activity shows no
    /// tooltip, since the expanded card shows their full text; the state word keeps its reason.
    #[test]
    fn the_title_branch_and_activity_have_no_tooltip() {
        let m = full(false);
        let text = mounted_at(&m, 380.0, roles()).painted_text();
        for hovered in [
            m.title.as_str(),
            "feat/1120",
            "Editing card_grid.rs \u{b7} 3/7",
        ] {
            let at = word(&text, hovered).bounds.center();
            let floated = mounted_at(&m, 380.0, roles()).floated_text(at);
            assert!(
                floated.is_empty(),
                "hovering {hovered:?} floats {floated:?}"
            );
        }
        let at = word(&text, m.state.label()).bounds.center();
        let floated = mounted_at(&m, 380.0, roles()).floated_text(at);
        assert_eq!(floated, [m.reason.label()], "the header tooltips stay");
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

    /// SC-005: each card carries its type icon with the type's word, and its state icon with the
    /// state's word and the reason; the five states and the types are told apart by icon.
    #[test]
    fn a_card_shows_its_type_and_state_with_their_words() {
        let mut state_icons = Vec::new();
        for state in CardState::ALL {
            let w = words(&model(state, "n"));
            assert_eq!(w.reason_tooltip, CardReason::Working.label());
            assert_eq!(w.type_icon, Icon::TypeFeature);
            let text = painted(&model(state, "n"));
            for word in [state.label(), LocationType::Feature.label()] {
                assert!(
                    text.iter().any(|t| t == word),
                    "{state:?}: the card paints {word:?}: {text:?}"
                );
            }
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
    /// words and their icons are in `on_header`; the body below is the one `body` fill with the title in `on_body`
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
                    // The header's icons are glyphs painted as text (T042, F2: tinted `on_header`).
                    let state_glyph = word(&text, &state_icon(state).glyph().to_string());
                    let type_glyph = word(&text, &type_icon(m.kind).glyph().to_string());
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
                        ("state icon", state_glyph, p.on_header),
                        ("type icon", type_glyph, p.on_header),
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

    /// K4 (FR-015): a pane under one minimum card stacks cards narrower than 280 px (a 200 px pane
    /// leaves 168 px). The longest state word and the longest type words still keep the header on
    /// one line inside the card: the state word stays whole, the type word ends where the card does,
    /// shortened with an ellipsis if it must be, and no two header words overlap.
    #[test]
    fn a_narrow_card_keeps_its_header_words_on_one_line_inside_it() {
        for kind in [LocationType::ProjectRoot, LocationType::Included] {
            for state in CardState::ALL {
                let mut m = full(false);
                m.state = state;
                m.kind = kind;
                for width in [168.0, 200.0] {
                    let text = mounted_at(&m, width, roles()).painted_text();
                    let at = format!("{state:?}, {kind:?} at {width}");
                    let state_word = word(&text, state.label());
                    let type_word = text
                        .iter()
                        .find(|t| {
                            t.content == kind.label()
                                || (t.content.ends_with('\u{2026}')
                                    && kind
                                        .label()
                                        .starts_with(t.content.trim_end_matches('\u{2026}')))
                        })
                        .unwrap_or_else(|| panic!("{at}: no type word: {text:?}"));
                    for w in [state_word, type_word] {
                        assert!(
                            w.bounds.x >= 0.0 && w.bounds.x + w.bounds.width <= width,
                            "{at}: {w:?} leaves the card"
                        );
                        assert_eq!(
                            w.bounds.y, state_word.bounds.y,
                            "{at}: {w:?} is not on the header's line"
                        );
                    }
                    assert!(
                        state_word.bounds.x + state_word.bounds.width <= type_word.bounds.x,
                        "{at}: {state_word:?} overlaps {type_word:?}"
                    );
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

    /// Feature 1121 T042, FR-008: with `.on_page` and the keyboard, Page Down / Page Up publish
    /// the card's page message for the directions it claims, captured; a direction it declines,
    /// a card without `.on_page`, or an unfocused card claims nothing, and the key goes on as today.
    #[test]
    fn page_keys_on_the_focused_card_publish_its_page_message() {
        let cursor = iced::advanced::mouse::Cursor::Unavailable;
        let card = |paged: bool| {
            let card = OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                .on_press("open".to_string())
                .expanded(true);
            Mounted::new(if paged {
                // Down only: the body is at its top (K4).
                card.on_page(|down| down.then(|| "page down".to_string()))
            } else {
                card
            })
        };
        let mut focused = card(true);
        focused.focus_next();
        assert_eq!(
            focused.send_captured(named_key(Named::PageDown), cursor),
            (vec!["page down".to_string()], true)
        );
        assert_eq!(
            focused.send_captured(named_key(Named::PageUp), cursor),
            (vec![], false),
            "a declined direction is not claimed"
        );
        // Review A F2 (M4): a modified Page key is a project chord (keymap NAV_CHORDS), not a page.
        for modifiers in [
            iced::keyboard::Modifiers::CTRL | iced::keyboard::Modifiers::SHIFT,
            iced::keyboard::Modifiers::LOGO | iced::keyboard::Modifiers::SHIFT,
        ] {
            assert_eq!(
                focused.send_captured(
                    super::super::test_support::key_with(Named::PageDown, modifiers),
                    cursor
                ),
                (vec![], false),
                "{modifiers:?}+PageDown passes through"
            );
        }
        let mut unfocused = card(true);
        assert_eq!(
            unfocused.send_captured(named_key(Named::PageDown), cursor),
            (vec![], false)
        );
        let mut plain = card(false);
        plain.focus_next();
        assert_eq!(
            plain.send_captured(named_key(Named::PageDown), cursor),
            (vec![], false)
        );
    }

    /// Feature 1121 T042 (U1): `.on_keyboard_focus` publishes when Tab gives the card the
    /// keyboard, and nothing when a press does (FR-002).
    #[test]
    fn keyboard_focus_publishes_on_tab_and_not_on_a_press() {
        let card = || {
            Mounted::new(
                OverviewCard::<String>::new(&model(CardState::Idle, "feat-a"), roles())
                    .on_press("open".to_string())
                    .on_keyboard_focus(|shown| format!("keyboard {shown}")),
            )
        };
        let mut tabbed = card();
        tabbed.focus_next();
        assert_eq!(tabbed.redraw(), ["keyboard true"]);
        let mut pressed = card();
        let at = pressed.container().center();
        assert!(
            !pressed.press(at).iter().any(|m| m.starts_with("keyboard")),
            "a press shows no keyboard focus"
        );
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
    use iced::advanced::widget::tree::{Tag, Tree};
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

    /// The tree of the card's ripple, wherever the card's wrappers put it.
    fn ripple_tree(tree: &Tree) -> Option<&Tree> {
        if tree.tag == Tag::of::<RippleState>() {
            return Some(tree);
        }
        tree.children.iter().find_map(ripple_tree)
    }

    fn ripple_tree_mut(tree: &mut Tree) -> Option<&mut Tree> {
        if tree.tag == Tag::of::<RippleState>() {
            return Some(tree);
        }
        tree.children.iter_mut().find_map(ripple_tree_mut)
    }

    fn ripple<'m>(m: &'m Mounted<'_>) -> &'m RippleState {
        ripple_tree(&m.tree)
            .expect("the card ripples")
            .state
            .downcast_ref::<RippleState>()
    }

    fn ripple_mut<'m>(m: &'m mut Mounted<'_>) -> &'m mut RippleState {
        ripple_tree_mut(&mut m.tree)
            .expect("the card ripples")
            .state
            .downcast_mut::<RippleState>()
    }

    fn settle(m: &mut Mounted<'_>) {
        let state = ripple_mut(m);
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

    /// T003 (US1-4, US1.5, FR-003; feature 1120, R9): mid-ripple the pixel at the press point,
    /// in the body's padding and in the header's, has moved from that row's fill toward the
    /// content colour `on_body`, and no further than it.
    #[test]
    fn the_ripple_is_drawn_in_the_states_content_colour() {
        let roles = tokens::roles(ColorScheme::Light);
        for state in CardState::ALL {
            let p = palette(state, &roles);
            // In the left padding, clear of any line: below the header, and inside it.
            for (what, at, base) in [
                ("body", Point::new(6.0, 100.0), p.body),
                ("header", Point::new(5.0, 20.0), p.header),
            ] {
                let mut m = activatable(&model(state, "feat-a"));
                m.press(at);
                ripple_mut(&mut m).advance(EXPAND, FADE);
                let (pixels, width) = m.drawn(iced::Color::BLACK);
                let i = ((at.y as u32 * width + at.x as u32) * 4) as usize;
                let got = [pixels[i], pixels[i + 1], pixels[i + 2]];
                let (fill, text) = (
                    [base.r, base.g, base.b],
                    [p.on_body.r, p.on_body.g, p.on_body.b],
                );
                let mut moved = false;
                for c in 0..3 {
                    let (lo, hi) = (fill[c].min(text[c]), fill[c].max(text[c]));
                    assert!(
                        u16::from(got[c]) + 2 >= u16::from(lo)
                            && u16::from(got[c]) <= u16::from(hi) + 2,
                        "{state:?} {what}: channel {c} is {} outside {lo}..{hi}",
                        got[c]
                    );
                    moved |= got[c].abs_diff(fill[c]) > 2;
                }
                assert!(
                    moved,
                    "{state:?} {what}: the ripple left the fill unchanged: {got:?}"
                );
            }
        }
    }

    /// T004 (FR-006): a plain card holds no ripple state and publishes nothing.
    #[test]
    fn a_plain_card_does_not_ripple() {
        let mut plain = Mounted::new(OverviewCard::<String>::new(
            &model(CardState::Idle, "feat-a"),
            tokens::roles(ColorScheme::Light),
        ));
        assert!(ripple_tree(&plain.tree).is_none());
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
        ripple_mut(&mut a).advance(EXPAND, FADE);
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

    // ---- Feature 1122: the issue link on the card (contract U3; FR-003, FR-005, FR-006, FR-015) ----

    use micold_core::issue_link::IssueLink;

    const LINK: &str = "Issue #1122";

    /// `titled()` with an issue link and a branch of `branch`.
    fn with_link(branch: Option<&str>) -> CardModel {
        let mut m = titled();
        m.branch = branch.map(str::to_string);
        m.issue_link = Some(IssueLink {
            label: LINK.into(),
            address: "https://github.com/o/r/issues/1122".into(),
        });
        m
    }

    /// The card of `model` with every message a link test reads, collapsed or `expanded`.
    fn linked(model: &CardModel, expanded: bool) -> OverviewCard<'static, String> {
        OverviewCard::<String>::new(model, tokens::roles(ColorScheme::Light))
            .on_press("open".to_string())
            .on_secondary_press(|(x, y)| format!("menu at {x},{y}"))
            .on_issue_link("link".to_string())
            .link_id(iced::advanced::widget::Id::new("link"))
            .expanded(expanded)
    }

    /// `card` in a 380 x 400 room (an expanded card fills its height, as in the grid).
    fn in_room(card: OverviewCard<'static, String>, width: f32) -> Mounted<'static> {
        Mounted::new_in(
            container(card).width(Length::Fixed(width)),
            Size::new(width, 400.0),
        )
    }

    /// Where the paragraph that says `content` is painted, if anywhere.
    fn painted_where(m: &mut Mounted<'_>, content: &str) -> Option<Rectangle> {
        m.painted_text()
            .into_iter()
            .find(|p| p.content == content)
            .map(|p| p.bounds)
    }

    fn link_point(m: &mut Mounted<'_>) -> Point {
        painted_where(m, LINK)
            .unwrap_or_else(|| panic!("{LINK:?} is not painted"))
            .center()
    }

    /// A point in the body's left padding, clear of every line and of the link.
    fn off_link(m: &Mounted<'_>) -> Point {
        Point::new(6.0, m.node.bounds().height * 0.6)
    }

    fn release(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
        m.send(
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// U3, FR-005, SC-003: a click on the link publishes only the link's message, starts no
    /// ripple and leaves the card without the keyboard, collapsed and expanded.
    #[test]
    fn a_click_on_the_link_publishes_only_the_links_message() {
        for expanded in [false, true] {
            let mut m = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            let at = link_point(&mut m);
            assert!(m.press(at).is_empty(), "expanded {expanded}");
            assert!(
                ripple(&m).is_idle(),
                "expanded {expanded}: the press rippled"
            );
            assert_eq!(release(&mut m, at), ["link"], "expanded {expanded}");
            assert!(
                m.send(named_key(Named::Enter), mouse::Cursor::Unavailable)
                    .is_empty(),
                "expanded {expanded}: the click gave the card the keyboard"
            );
        }
    }

    /// FR-005: a press on the card released on the link activates nothing, and its ripple fades.
    #[test]
    fn a_press_on_the_card_released_on_the_link_activates_nothing() {
        for expanded in [false, true] {
            let mut m = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            let (on, off) = (link_point(&mut m), off_link(&m));
            m.press(off);
            assert!(!ripple(&m).is_idle(), "expanded {expanded}");
            m.hover(on);
            assert!(release(&mut m, on).is_empty(), "expanded {expanded}");
            settle(&mut m);
            assert!(
                ripple(&m).is_idle(),
                "expanded {expanded}: the ripple did not fade"
            );
        }
    }

    /// Story 4 sc. 5: a press on the link released elsewhere on the card activates nothing.
    #[test]
    fn a_press_on_the_link_released_on_the_card_activates_nothing() {
        for expanded in [false, true] {
            let mut m = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            let (on, off) = (link_point(&mut m), off_link(&m));
            m.press(on);
            m.hover(off);
            assert!(release(&mut m, off).is_empty(), "expanded {expanded}");
        }
    }

    /// Story 4 sc. 1: a click on the card outside the link activates it with its ripple.
    #[test]
    fn a_click_on_the_card_outside_the_link_activates_it_as_before() {
        for expanded in [false, true] {
            let mut m = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            let off = off_link(&m);
            m.press(off);
            assert!(!ripple(&m).is_idle(), "expanded {expanded}");
            assert_eq!(release(&mut m, off), ["open"], "expanded {expanded}");
        }
    }

    /// FR-006, Story 4 sc. 4: a secondary press on the link opens the card's menu there.
    #[test]
    fn a_secondary_press_on_the_link_opens_the_cards_menu() {
        for expanded in [false, true] {
            let mut m = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            let at = link_point(&mut m);
            m.hover(at);
            let got = m.send(
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                mouse::Cursor::Available(at),
            );
            assert_eq!(
                got,
                [format!("menu at {},{}", at.x as u16, at.y as u16)],
                "expanded {expanded}"
            );
        }
    }

    /// FR-008, contract U3: with `.on_link_keyboard_focus`, Tab goes card, its link, the next
    /// card; Shift+Tab goes back; Enter on the link opens it without the card's ripple, and Enter
    /// on the card activates the card.
    #[test]
    fn tab_reaches_the_link_right_after_its_card() {
        use iced::advanced::widget::operation::focusable;
        for expanded in [false, true] {
            let mut m = Mounted::new_in(
                iced::widget::column![
                    container(
                        linked(&with_link(Some("feat/1122_x")), expanded)
                            .on_link_keyboard_focus(|f| format!("link focus {f}"))
                    )
                    .height(Length::Fixed(300.0)),
                    OverviewCard::<String>::new(&titled(), roles()).on_press("next".to_string()),
                ],
                Size::new(380.0, 2000.0),
            );
            let none = mouse::Cursor::Unavailable;
            m.focus_next();
            assert!(m.redraw().is_empty(), "expanded {expanded}");
            m.focus_next();
            assert_eq!(m.redraw(), ["link focus true"], "expanded {expanded}");
            assert_eq!(m.send(named_key(Named::Enter), none), ["link"]);
            assert!(
                ripple(&m).is_idle(),
                "expanded {expanded}: Enter on the link rippled"
            );
            m.focus_next();
            assert_eq!(m.redraw(), ["link focus false"], "expanded {expanded}");
            assert_eq!(m.send(named_key(Named::Enter), none), ["next"]);
            m.drive(focusable::focus_previous::<()>());
            assert_eq!(m.redraw(), ["link focus true"], "expanded {expanded}");
            m.drive(focusable::focus_previous::<()>());
            assert_eq!(m.redraw(), ["link focus false"], "expanded {expanded}");
            assert_eq!(m.send(named_key(Named::Enter), none), ["open"]);
            assert!(
                !ripple(&m).is_idle(),
                "expanded {expanded}: Enter on the card"
            );
        }
    }

    /// `card` with its link wired for the keyboard, in a room, with the keyboard on the link (Tab
    /// past the card), the focus reports drained.
    fn on_the_link(card: OverviewCard<'static, String>, size: Size) -> Mounted<'static> {
        let mut m = Mounted::new_in(
            container(card.on_link_keyboard_focus(|f| format!("link focus {f}")))
                .width(Length::Fixed(size.width))
                .height(Length::Fixed(size.height)),
            size,
        );
        m.focus_next();
        m.focus_next();
        assert_eq!(m.redraw(), ["link focus true"]);
        m
    }

    /// FR-009, R9: the Menu key and Shift+F10 on the focused link publish the link's menu
    /// message, captured, and not the card's; collapsed and expanded.
    #[test]
    fn the_menu_keys_on_the_focused_link_publish_the_links_menu_message() {
        use super::super::test_support::key_with;
        use iced::keyboard::Modifiers;
        let none = mouse::Cursor::Unavailable;
        for expanded in [false, true] {
            let card = linked(&with_link(Some("feat/1122_x")), expanded)
                .on_menu_key("card menu".to_string())
                .on_link_menu_key("link menu".to_string());
            let mut m = on_the_link(card, Size::new(380.0, 400.0));
            for key in [
                key_with(Named::ContextMenu, Modifiers::empty()),
                key_with(Named::F10, Modifiers::SHIFT),
            ] {
                assert_eq!(
                    m.send_captured(key.clone(), none),
                    (vec!["link menu".to_string()], true),
                    "expanded {expanded}: {key:?}"
                );
            }
            assert!(
                ripple(&m).is_idle(),
                "expanded {expanded}: a menu key rippled"
            );
        }
    }

    /// Story 3 sc.6, R9: Page Up / Page Down on the focused link publish the card's page message.
    #[test]
    fn page_keys_on_the_focused_link_publish_the_cards_page_message() {
        let none = mouse::Cursor::Unavailable;
        for expanded in [false, true] {
            let card = linked(&with_link(Some("feat/1122_x")), expanded)
                .on_page(|down| Some(format!("page {}", if down { "down" } else { "up" })));
            let mut m = on_the_link(card, Size::new(380.0, 400.0));
            assert_eq!(
                m.send_captured(named_key(Named::PageDown), none),
                (vec!["page down".to_string()], true),
                "expanded {expanded}"
            );
            assert_eq!(
                m.send_captured(named_key(Named::PageUp), none),
                (vec!["page up".to_string()], true),
                "expanded {expanded}"
            );
            // A modified Page key is a project chord (keymap NAV_CHORDS), not a page.
            let chord = super::super::test_support::key_with(
                Named::PageDown,
                iced::keyboard::Modifiers::CTRL | iced::keyboard::Modifiers::SHIFT,
            );
            assert_eq!(m.send_captured(chord, none), (vec![], false));
        }
    }

    /// FR-008, Story 3 sc.6: a link in the scrolled part of an expanded card's body is scrolled
    /// into the inner body's viewport when it takes the keyboard.
    #[test]
    fn a_focused_link_below_the_fold_scrolls_into_the_body() {
        let mut model = with_link(Some("feat/1122_x"));
        model.display_name = "a long display name ".repeat(40);
        let mut m = on_the_link(linked(&model, true), Size::new(380.0, 220.0));
        let viewport = m.scrollables()[0].0;
        // Where the focused link is on screen, the inner body's scroll taken off.
        let on_screen = |m: &mut Mounted<'_>| {
            m.drive(crate::ui::focus::focused_bounds())
                .flatten()
                .expect("the link holds the keyboard")
        };
        let before = on_screen(&mut m);
        assert!(
            before.y > viewport.y + viewport.height,
            "the link starts below the fold: {before:?} in {viewport:?}"
        );
        m.drive(crate::ui::focus::into_view::<()>());
        let after = on_screen(&mut m);
        assert!(
            after.y >= viewport.y && after.y + after.height <= viewport.y + viewport.height,
            "the link is in view: {after:?} in {viewport:?}"
        );
    }

    /// Contract U3: without `.on_link_keyboard_focus` the link is no Tab stop, so Tab goes from the
    /// card straight to the next one.
    #[test]
    fn without_link_focus_reports_the_link_is_no_tab_stop() {
        for expanded in [false, true] {
            let mut bare = in_room(linked(&with_link(Some("feat/1122_x")), expanded), 380.0);
            assert_eq!(bare.drive(Focusables(0)), Some(1), "expanded {expanded}");
            let mut wired = in_room(
                linked(&with_link(Some("feat/1122_x")), expanded)
                    .on_link_keyboard_focus(|f| format!("{f}")),
                380.0,
            );
            assert_eq!(wired.drive(Focusables(0)), Some(2), "expanded {expanded}");
        }
    }

    /// The width `LINK` is painted at with nothing in its way.
    fn link_alone_width() -> f32 {
        let mut alone = Mounted::new(super::super::TextLink::new(LINK, String::new(), roles()));
        painted_where(&mut alone, LINK).expect("painted").width
    }

    /// FR-003, SC-004: at 280, 380 and 480 px, with a 120-character branch, the link is painted
    /// whole inside the card, at the branch line's role (FR-015), and the collapsed card's short
    /// branch ends in an ellipsis.
    #[test]
    fn the_link_is_whole_inside_the_card_at_every_width() {
        // Not a conventional branch, which shows as `type/number` (FR-007): 120 characters.
        let branch = format!("topic-{}", "x".repeat(114));
        let whole = link_alone_width();
        for expanded in [false, true] {
            for width in WIDTHS {
                let mut m = in_room(linked(&with_link(Some(&branch)), expanded), width);
                let card = m.node.bounds();
                let text = m.painted_text();
                let link = text
                    .iter()
                    .find(|p| p.content == LINK)
                    .unwrap_or_else(|| panic!("{width} expanded {expanded}: no link {text:?}"));
                assert!(
                    (link.bounds.width - whole).abs() < 0.5,
                    "{width} expanded {expanded}: {link:?} is cut from {whole}"
                );
                assert!(
                    card.contains(link.bounds.position())
                        && link.bounds.x + link.bounds.width <= card.x + card.width
                        && link.bounds.y + link.bounds.height <= card.y + card.height,
                    "{width} expanded {expanded}: {link:?} leaves {card:?}"
                );
                assert_eq!(link.size, TypeRole::Body.size(), "FR-015");
                if !expanded {
                    let short = text
                        .iter()
                        .find(|p| p.content.contains("xxxx"))
                        .unwrap_or_else(|| panic!("{width}: no branch {text:?}"));
                    assert!(short.content.ends_with('\u{2026}'), "{width}: {short:?}");
                }
            }
        }
    }

    /// FR-003: the link sits on the branch line, centred on it, and keeps its height: the card is
    /// as tall with the link as without it (the link's focus gutter spills over its neighbours).
    #[test]
    fn the_link_sits_on_the_branch_line_and_keeps_its_height() {
        let linked_model = with_link(Some("feat/1122_x"));
        let mut plain_model = linked_model.clone();
        plain_model.issue_link = None;
        let mut m = in_room(linked(&linked_model, false), 380.0);
        let plain = in_room(linked(&plain_model, false), 380.0);
        assert_eq!(m.node.bounds().height, plain.node.bounds().height);
        let branch = painted_where(&mut m, "feat/1122").expect("the short branch is painted");
        let link = painted_where(&mut m, LINK).expect("the link is painted");
        assert!(
            (branch.center_y() - link.center_y()).abs() <= 0.5,
            "{branch:?} vs {link:?}"
        );
        assert!(
            link.x > branch.x + branch.width,
            "the link follows the branch"
        );
    }

    /// Review A M2 F4 (deferred to M4): when the link is the expanded body's first row (no display
    /// name, no branch line), its whole focus gutter lies inside the body's viewport, so no part of
    /// the ring is clipped.
    #[test]
    fn a_link_first_in_the_expanded_body_keeps_its_whole_gutter() {
        let mut model = with_link(None);
        model.display_name = model.title.clone();
        let mut m = on_the_link(linked(&model, true), Size::new(380.0, 400.0));
        let viewport = m.scrollables()[0].0;
        let gutter = m
            .drive(crate::ui::focus::focused_bounds())
            .flatten()
            .expect("the link holds the keyboard");
        assert!(
            gutter.y >= viewport.y,
            "the gutter's top {gutter:?} is above the body's viewport {viewport:?}"
        );
    }

    /// FR-003: with no branch line (the title is the branch, or there is no branch) the link sits
    /// alone in its place, flush with the lines around it.
    #[test]
    fn with_no_branch_line_the_link_sits_alone_in_its_place() {
        let mut titled_by_branch = with_link(Some("feat/1122_x"));
        titled_by_branch.title = "feat/1122_x".into();
        for model in [titled_by_branch, with_link(None)] {
            for expanded in [false, true] {
                let mut m = in_room(linked(&model, expanded), 380.0);
                let text = m.painted_text();
                let link = text.iter().find(|p| p.content == LINK).expect("link");
                let sessions = text
                    .iter()
                    .find(|p| p.content.contains("session"))
                    .expect("sessions line");
                assert!(
                    (link.bounds.x - sessions.bounds.x).abs() <= 0.5,
                    "expanded {expanded}: {link:?} vs {sessions:?}"
                );
                assert!(
                    link.bounds.y < sessions.bounds.y,
                    "expanded {expanded}: the link sits above the sessions line"
                );
            }
        }
    }

    /// Contract U3: without `.on_issue_link`, or with no link in the model, the card lays out
    /// exactly as before, with no extra node.
    #[test]
    fn a_card_without_a_link_lays_out_as_before() {
        use super::super::test_support::all_bounds;
        let mut none = with_link(Some("feat/1122_x"));
        none.issue_link = None;
        for expanded in [false, true] {
            let before = Mounted::new(
                OverviewCard::<String>::new(&none, roles())
                    .on_press("open".to_string())
                    .expanded(expanded),
            );
            for card in [
                OverviewCard::<String>::new(&with_link(Some("feat/1122_x")), roles())
                    .on_press("open".to_string())
                    .expanded(expanded),
                OverviewCard::<String>::new(&none, roles())
                    .on_press("open".to_string())
                    .on_issue_link("link".to_string())
                    .expanded(expanded),
            ] {
                let after = Mounted::new(card);
                assert_eq!(all_bounds(&after.node), all_bounds(&before.node));
            }
        }
    }
}
