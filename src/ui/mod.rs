//! iced rendering layer for the main window. Bin-only; compiled with the `gui` feature.

mod about;
mod material;
mod project_selector;
mod rename;
mod settings_form;
mod shell;
mod sidebar;
pub mod style;
pub mod terminal;
mod toolbar;
mod worktree_form;

use iced::widget::{column, container, mouse_area, row, stack, text, Space};
use iced::{Element, Font, Length, Subscription};
use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::icons::Icon;
use micold_ai_ide::tokens::{self, Rgb};

/// The embedded Material Symbols (Outlined) icon font. Registered once at startup in
/// `main` so every icon glyph resolves; see `assets/fonts/PROVENANCE.md`.
pub const MATERIAL_SYMBOLS_BYTES: &[u8] =
    include_bytes!("../../assets/fonts/MaterialSymbolsOutlined.ttf");

/// The font family the embedded icon file advertises (asserted by `tests/icons_font.rs`).
pub const MATERIAL_SYMBOLS: Font = Font::with_name("Material Symbols Outlined");

/// Render an [`Icon`] as an element at a design-system size, tinted with a foreground color
/// role (FR-004). Reuses [`style::color`] so tint follows the active theme exactly like all
/// other text, giving light/dark and disabled states for free (FR-007).
pub fn icon<'a, M: 'a>(icon: Icon, size: u16, color: Rgb) -> Element<'a, M> {
    text(icon.glyph().to_string())
        .font(MATERIAL_SYMBOLS)
        .size(size)
        .color(style::color(color))
        .into()
}

/// Animation progress (0..1) for the material motion wrappers, driven by the binary's clock.
#[derive(Debug, Clone, Copy)]
pub struct Anim {
    /// Overflow-menu fade progress.
    pub menu: f32,
    /// Sidebar slide progress (0 = collapsed, 1 = expanded).
    pub sidebar: f32,
    /// Main-view fade progress (dips to 0 and back to 1 when the content changes).
    pub main: f32,
    /// Sidebar resize-handle hover-highlight progress (0 = idle, 1 = fully highlighted).
    pub handle_hover: f32,
}

impl Default for Anim {
    fn default() -> Self {
        Self {
            menu: 0.0,
            sidebar: 1.0,
            main: 1.0,
            handle_hover: 0.0,
        }
    }
}

/// Render the main window: the top app bar over the shell body (active project / empty
/// state), with any modal overlay (About or the project selector) stacked on top. Every
/// surface is styled from the active color scheme's design tokens. `anim` carries the
/// material motion progress (menu fade, sidebar slide, main-view fade).
pub fn view<'a>(
    state: &'a State,
    terminal: Option<&'a terminal::RuntimeTerminal>,
    anim: Anim,
) -> Element<'a, Message> {
    let scheme = state.color_scheme();
    let roles = tokens::roles(scheme);
    let bg = style::color(roles.background);

    // With a project open, show the worktree sidebar beside the main area; the main area is
    // the embedded terminal when a session is active (FR-012), else the project surface. The
    // sidebar slides in/out and is resizable; the main content fades when it changes.
    let body: Element<'a, Message> = if state.workspace.active_project().is_some() {
        let main_inner: Element<'a, Message> = if state.active_session.is_some() {
            terminal::pane(state, terminal, scheme)
        } else {
            shell::view(state, scheme)
        };
        let main = material::fade(main_inner, anim.main, bg);
        let left: Element<'a, Message> = if state.sidebar_hidden && anim.sidebar <= 0.001 {
            sidebar::collapsed_strip(scheme)
        } else {
            row![
                material::slide(sidebar::view(state, scheme), anim.sidebar),
                sidebar::handle(scheme, anim.handle_hover)
            ]
            .into()
        };
        row![left, main]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        material::fade(shell::view(state, scheme), anim.main, bg)
    };

    let mut base: Element<'a, Message> = container(column![toolbar::view(scheme), body])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::window_bg(roles))
        .into();

    // While resizing, a full-window capture layer tracks the cursor and ends the drag on
    // release, so the drag continues even when the pointer leaves the thin handle.
    if state.sidebar_dragging {
        let capture = mouse_area(
            container(Space::new(Length::Fill, Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_move(|p| Message::SidebarDragMoved(p.x.max(0.0) as u16))
        .on_release(Message::SidebarDragEnded);
        base = stack![base, capture].into();
    }

    // Float the toolbar's overflow menu over everything (no toolbar reflow), fading in/out.
    let base = material::MenuOverlay::new(
        base,
        toolbar::overflow_items(state),
        Message::HelpMenuToggled,
        roles,
    )
    .progress(anim.menu)
    .into();

    match state.overlay {
        Overlay::None => base,
        Overlay::About => about::modal(base, scheme),
        Overlay::ProjectSelector => match &state.selector {
            Some(selector) => project_selector::modal(base, selector, scheme),
            // Overlay flagged but no selector state — render the base defensively.
            None => base,
        },
        Overlay::RenameProject => match &state.rename_draft {
            Some(draft) => rename::modal(base, draft, scheme),
            None => base,
        },
        Overlay::AddWorktree => match &state.worktree_form {
            Some(form) => worktree_form::modal(base, form, state.worktree_error.as_deref(), scheme),
            None => base,
        },
        Overlay::Settings => match &state.settings_draft {
            Some(draft) => settings_form::modal(base, draft, scheme),
            None => base,
        },
    }
}

/// Keyboard subscription. While a modal overlay is open, Esc dismisses it — the About
/// dialog (FR-011) or the project selector. Mirrors [`micold_ai_ide::app::on_escape`].
///
/// Feature 006 (FR-009): while the embedded terminal holds focus, the app binds NO global
/// keyboard shortcuts — every key is owned by the focused terminal widget (so Esc and any app
/// chord reach the `claude` process instead of driving the app).
pub fn subscription(state: &State) -> Subscription<Message> {
    if state.terminal_focused {
        return Subscription::none();
    }
    // `on_key_press` takes a non-capturing `fn`, so each overlay supplies its own.
    match state.overlay {
        Overlay::None => Subscription::none(),
        Overlay::About => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::AboutClosed)
        }),
        Overlay::ProjectSelector => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::ProjectSelectorClosed)
        }),
        Overlay::RenameProject => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::RenameCancelled)
        }),
        Overlay::AddWorktree => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::AddWorktreeCancelled)
        }),
        Overlay::Settings => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::SettingsCancelled)
        }),
    }
}
