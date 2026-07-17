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
use micold_ai_ide::motion::Animator;
use micold_ai_ide::theme::ColorScheme;
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

/// Identifies each animated element in the app. The generic [`Animator`] core
/// (`micold_ai_ide::motion`) is keyed by this; adding a new animated element is: add a variant,
/// set its target, and read its progress — no per-animation fields anywhere (FR-007/FR-008).
/// This enum is the app-specific consumer side; the reusable core carries no key scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionKey {
    /// Overflow-menu fade.
    Menu,
    /// Sidebar slide (0 = collapsed, 1 = expanded).
    Sidebar,
    /// Main-view fade (dips to 0 and back to 1 when the content changes).
    Main,
    /// Sidebar resize-handle hover-highlight (0 = idle, 1 = fully highlighted).
    HandleHover,
    /// The currently open/closing modal overlay's fade (0 = hidden, 1 = shown).
    Overlay,
}

/// Render the main window: the top app bar over the shell body (active project / empty
/// state), with any modal overlay stacked on top. Every surface is styled from the active
/// color scheme's design tokens. `motion` carries all material motion progress (menu fade,
/// sidebar slide, main-view fade, handle hover, and the overlay fade). `dismissing` is the
/// snapshot of a just-closed overlay still fading out (rendered instead of a live overlay when
/// `state.overlay` is already `None` — see [`crate::ClosingOverlay`]).
pub fn view<'a>(
    state: &'a State,
    terminal: Option<&'a terminal::RuntimeTerminal>,
    motion: &Animator<MotionKey>,
    dismissing: Option<&'a crate::ClosingOverlay>,
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
        let main = material::fade(main_inner, motion.get(MotionKey::Main), bg);
        let left: Element<'a, Message> =
            if state.sidebar_hidden && motion.get(MotionKey::Sidebar) <= 0.001 {
                sidebar::collapsed_strip(scheme)
            } else {
                row![
                    material::slide(sidebar::view(state, scheme), motion.get(MotionKey::Sidebar)),
                    sidebar::handle(scheme, motion.get(MotionKey::HandleHover))
                ]
                .into()
            };
        row![left, main]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        material::fade(shell::view(state, scheme), motion.get(MotionKey::Main), bg)
    };

    let mut base: Element<'a, Message> = container(column![toolbar::view(state, scheme), body])
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
    let base: Element<'a, Message> = material::MenuOverlay::new(
        base,
        toolbar::overflow_items(state),
        Message::HelpMenuToggled,
        roles,
    )
    .progress(motion.get(MotionKey::Menu))
    .into();

    // Float the project switcher panel (feature 008, FR-004/005/006/007/008/009). Rows are
    // built purely from the workspace: active marker, running-background-session count, and
    // unavailable badge. Mutually exclusive with the overflow menu (handled in the reducer).
    let switcher_rows: Vec<material::ProjectRow<Message>> = state
        .switcher_entries()
        .into_iter()
        .map(|e| material::ProjectRow {
            label: e.label,
            is_active: e.is_active,
            running_count: e.running_count,
            available: e.available,
            on_select: Message::KnownProjectReopened(e.path),
        })
        .collect();
    let base = material::ProjectSwitcherOverlay::new(
        base,
        switcher_rows,
        Message::ProjectSelectorOpened,
        Message::ProjectSwitcherToggled,
        roles,
    )
    .open(state.project_switcher_open)
    .into();

    // The overlay fade progress (0 = hidden, 1 = fully shown). Drives both the enter (a live
    // overlay fading in as this rises 0→1) and the exit (a dismissing snapshot fading out as it
    // falls 1→0). At <= 0.001 the modal renders `base` unchanged.
    let overlay_progress = motion.get(MotionKey::Overlay);
    match state.overlay {
        // No overlay open. If one is still fading out, render its snapshot (captured before the
        // core cleared its live state) so the exit animation has something to draw (FR-002).
        Overlay::None => match dismissing {
            Some(closing) => dismissing_modal(base, closing, scheme, overlay_progress),
            None => base,
        },
        Overlay::About => about::modal(base, scheme, overlay_progress),
        Overlay::ProjectSelector => match &state.selector {
            Some(selector) => project_selector::modal(base, selector, scheme, overlay_progress),
            // Overlay flagged but no selector state — render the base defensively.
            None => base,
        },
        Overlay::RenameProject => match &state.rename_draft {
            Some(draft) => rename::modal(base, draft, scheme, overlay_progress),
            None => base,
        },
        Overlay::AddWorktree => match &state.worktree_form {
            Some(form) => worktree_form::modal(
                base,
                form,
                state.worktree_error.as_deref(),
                scheme,
                overlay_progress,
            ),
            None => base,
        },
        Overlay::Settings => match &state.settings_draft {
            Some(draft) => settings_form::modal(base, draft, scheme, overlay_progress),
            None => base,
        },
    }
}

/// Render the snapshot of a just-closed overlay so it can keep fading out after the pure core
/// has already cleared its live state (see [`crate::ClosingOverlay`]). Delegates to the same
/// per-overlay `modal` render functions as the live path, so the exit is the enter in reverse.
fn dismissing_modal<'a>(
    base: Element<'a, Message>,
    closing: &'a crate::ClosingOverlay,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    use crate::ClosingOverlay;
    match closing {
        ClosingOverlay::About => about::modal(base, scheme, progress),
        ClosingOverlay::Selector(selector) => {
            project_selector::modal(base, selector, scheme, progress)
        }
        ClosingOverlay::Rename(draft) => rename::modal(base, draft, scheme, progress),
        ClosingOverlay::Worktree(form, error) => {
            worktree_form::modal(base, form, error.as_deref(), scheme, progress)
        }
        ClosingOverlay::Settings(draft) => settings_form::modal(base, draft, scheme, progress),
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
