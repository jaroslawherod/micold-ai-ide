//! iced rendering layer for the main window. Bin-only; compiled with the `gui` feature.

mod about;
mod confirm_delete;
mod confirm_forget;
mod confirm_session_remove;
mod material;
pub use material::target_offset_delta;
mod project_selector;
mod rename;
mod settings_form;
mod shell;
mod sidebar;
pub mod style;
pub mod terminal;
mod toolbar;
mod worktree_form;
mod worktree_rename;

use crate::app::{Message, Overlay, State};
use crate::icons::Icon;
use crate::motion::Animator;
use crate::tokens::{self, spacing, type_scale, Rgb, Roles};
use iced::widget::{button, column, container, mouse_area, row, stack, text, Space};
use iced::{Element, Font, Length, Subscription};
use micold_core::session::SessionId;
use micold_core::theme::ColorScheme;

/// The embedded Material Symbols (Outlined) icon font. Registered once at startup in
/// `main` so every icon glyph resolves; see `assets/fonts/PROVENANCE.md`.
pub const MATERIAL_SYMBOLS_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/MaterialSymbolsOutlined.ttf");

/// The font family the embedded icon file advertises (asserted by `tests/icons_font.rs`).
pub const MATERIAL_SYMBOLS: Font = Font::with_name("Material Symbols Outlined");

/// Render an [`Icon`] as an element at a design-system size, tinted with a foreground color
/// role (FR-004). Reuses [`style::color`] so tint follows the active theme exactly like all
/// other text, giving light/dark and disabled states for free (FR-007).
pub fn icon<'a, M: 'a>(icon: Icon, size: u16, color: Rgb) -> Element<'a, M> {
    icon_colored(icon, size, style::color(color))
}

/// [`icon`] with an already-resolved color, so callers can apply alpha — notably
/// [`style::disabled_color`], since a glyph that colors itself does not inherit a disabled
/// button's `text_color`.
pub fn icon_colored<'a, M: 'a>(icon: Icon, size: u16, color: iced::Color) -> Element<'a, M> {
    text(icon.glyph().to_string())
        .font(MATERIAL_SYMBOLS)
        .size(size)
        .color(color)
        .into()
}

/// Identifies each animated element in the app. The generic [`Animator`] core
/// (`crate::motion`) is keyed by this; adding a new animated element is: add a variant,
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
    /// Sidebar tag-filter panel fade (feature 009).
    SidebarFilter,
}

/// Animation key for a worktree row's hover-revealed actions fade (feature 008). Each worktree
/// gets its own track (keyed by a hash of its `dir_name`) so rows fade in and out independently
/// — hovering B while A fades out animates both at once.
pub fn worktree_fx_key(dir_name: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    dir_name.hash(&mut hasher);
    hasher.finish()
}

/// Render the main window: the top app bar over the shell body (active project / empty
/// state), with any modal overlay stacked on top. Every surface is styled from the active
/// color scheme's design tokens. `motion` carries all material motion progress (menu fade,
/// sidebar slide, main-view fade, handle hover, and the overlay fade). `dismissing` is the
/// snapshot of a just-closed overlay still fading out (rendered instead of a live overlay when
/// The stack of dismissible global notification banners, newest last. Empty when there is
/// nothing to report, in which case it occupies no space.
fn notifications<'a>(state: &'a State, r: Roles) -> Element<'a, Message> {
    let mut stack = column![].spacing(spacing::SM);
    for (index, notification) in state.notifications.iter().enumerate() {
        let banner = row![
            text(notification.message.clone())
                .size(type_scale::BODY)
                .width(Length::Fill),
            button(text("Dismiss").size(type_scale::LABEL))
                .on_press(Message::NotificationDismissed(index))
                .style(style::outlined(r)),
        ]
        .spacing(spacing::SM)
        .align_y(iced::Alignment::Center);
        stack = stack.push(
            container(banner)
                .padding(spacing::MD)
                .width(Length::Fill)
                .style(style::notification(r, notification.level)),
        );
    }
    if state.notifications.is_empty() {
        stack.into()
    } else {
        container(stack)
            .padding([spacing::SM, spacing::MD])
            .width(Length::Fill)
            .into()
    }
}

/// `state.overlay` is already `None` — see [`crate::app::ClosingOverlay`]).
pub fn view<'a>(
    state: &'a State,
    terminal: Option<&'a terminal::RuntimeTerminal>,
    motion: &Animator<MotionKey>,
    dismissing: Option<&'a crate::app::ClosingOverlay>,
    row_fx: &crate::motion::Animator<u64>,
    env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
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
                    material::slide(
                        sidebar::view(state, scheme, row_fx, motion.get(MotionKey::SidebarFilter)),
                        motion.get(MotionKey::Sidebar)
                    ),
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

    // The global notification surface, rendered here — between the toolbar and the body, above
    // every branch that decides what the body is. Deliberately unconditional: the failures this
    // replaces were all cases where state was set correctly but the only render site sat inside
    // a branch that could not be taken. Nothing may nest this inside an `if`.
    let mut base: Element<'a, Message> = container(column![
        toolbar::view(state, scheme),
        notifications(state, roles),
        body
    ])
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

    // Float the project switcher panel. Rows are built purely from the workspace: active
    // marker, running-background-session count, and unavailable badge. Mutually exclusive with
    // the overflow menu (handled in the reducer).
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

    // Float the worktree right-click context menu over everything, anchored near the sidebar
    // (feature 008, FR-013). Only present while a worktree's menu is open.
    let base = match &state.worktree_menu_open {
        Some(dir) => material::MenuOverlay::new(
            base,
            worktree_menu_items(dir, &state.worktree_display_name(dir)),
            Message::WorktreeMenuDismissed,
            roles,
        )
        .anchor(iced::Point::new(24.0, 96.0))
        .into(),
        None => base,
    };

    // Float the session right-click context menu over everything (bugfix BUG-003). Only present
    // while a session's menu is open.
    let base = match state.session_menu_open {
        Some(id) => material::MenuOverlay::new(
            base,
            session_menu_items(id),
            Message::SessionMenuDismissed,
            roles,
        )
        .anchor(iced::Point::new(24.0, 96.0))
        .into(),
        None => base,
    };

    // The overlay fade progress (0 = hidden, 1 = fully shown). Drives both the enter (a live
    // overlay fading in as this rises 0→1) and the exit (a dismissing snapshot fading out as it
    // falls 1→0). At <= 0.001 the modal renders `base` unchanged.
    let overlay_progress = motion.get(MotionKey::Overlay);
    match state.overlay {
        // No overlay open. If one is still fading out, render its snapshot (captured before the
        // core cleared its live state) so the exit animation has something to draw (FR-002).
        Overlay::None => match dismissing {
            Some(closing) => {
                dismissing_modal(base, closing, scheme, overlay_progress, env_include_outcome)
            }
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
            Some(draft) => {
                settings_form::modal(base, draft, scheme, overlay_progress, env_include_outcome)
            }
            None => base,
        },
        Overlay::ConfirmWorktreeDelete => match &state.worktree_delete_target {
            Some(dir) => {
                let branch = state
                    .worktrees
                    .iter()
                    .find(|w| &w.dir_name == dir)
                    .and_then(|w| w.branch.as_deref());
                confirm_delete::modal(
                    base,
                    dir,
                    &state.worktree_display_name(dir),
                    branch,
                    state.worktree_delete_keep_branch,
                    scheme,
                    overlay_progress,
                )
            }
            None => base,
        },
        Overlay::RenameWorktree => match &state.worktree_rename_draft {
            Some(draft) => worktree_rename::modal(base, draft, scheme, overlay_progress),
            None => base,
        },
        Overlay::ConfirmSessionRemove => match state
            .session_remove_target
            .and_then(|id| state.workspace.find_session(id))
        {
            Some((_, session)) => confirm_session_remove::modal(
                base,
                session.label.display(),
                scheme,
                overlay_progress,
            ),
            None => base,
        },
        Overlay::ConfirmForgetProject => match &state.forget_target {
            Some(path) => {
                // The display name and running-session count are read from the catalog/sessions
                // at render time; the count (FR-002a) is exactly the set the binary will stop.
                let display_name = state
                    .workspace
                    .projects
                    .iter()
                    .find(|p| &p.path == path)
                    .map(|p| p.display_name.clone())
                    .unwrap_or_else(|| micold_core::project::default_display_name(path));
                let running = state.workspace.running_session_count(path);
                confirm_forget::modal(base, &display_name, running, scheme, overlay_progress)
            }
            None => base,
        },
    }
}

/// The items in a worktree's right-click context menu (feature 008, FR-013; "Copy name" added
/// for cross-application clipboard access to labels the app doesn't render as selectable text).
fn worktree_menu_items(dir: &str, display_name: &str) -> Vec<material::MenuItem<Message>> {
    vec![
        material::MenuItem::new(
            Icon::Copy,
            "Copy name",
            Message::TextCopyRequested(display_name.to_string()),
        ),
        material::MenuItem::new(
            Icon::Rename,
            "Rename",
            Message::WorktreeRenameStarted(dir.to_string()),
        ),
        material::MenuItem::new(
            Icon::Unavailable,
            "Delete",
            Message::WorktreeDeleteRequested(dir.to_string()),
        ),
    ]
}

/// The items in a session's right-click context menu (bugfix BUG-003): "Close" archives (kept,
/// hidden, never resurrected by reconciliation — FR-015a/FR-020c); "Remove" permanently deletes,
/// behind a confirm dialog (FR-015c).
fn session_menu_items(id: SessionId) -> Vec<material::MenuItem<Message>> {
    vec![
        material::MenuItem::new(Icon::Close, "Close", Message::SessionCloseRequested(id)),
        material::MenuItem::new(
            Icon::Unavailable,
            "Remove",
            Message::SessionRemoveRequested(id),
        ),
    ]
}

/// Render the snapshot of a just-closed overlay so it can keep fading out after the pure core
/// has already cleared its live state (see [`crate::app::ClosingOverlay`]). Delegates to the same
/// per-overlay `modal` render functions as the live path, so the exit is the enter in reverse.
fn dismissing_modal<'a>(
    base: Element<'a, Message>,
    closing: &'a crate::app::ClosingOverlay,
    scheme: ColorScheme,
    progress: f32,
    env_include_outcome: &'a micold_core::env_include::EnvIncludeOutcome,
) -> Element<'a, Message> {
    use crate::app::ClosingOverlay;
    match closing {
        ClosingOverlay::About => about::modal(base, scheme, progress),
        ClosingOverlay::Selector(selector) => {
            project_selector::modal(base, selector, scheme, progress)
        }
        ClosingOverlay::Rename(draft) => rename::modal(base, draft, scheme, progress),
        ClosingOverlay::Worktree(form, error) => {
            worktree_form::modal(base, form, error.as_deref(), scheme, progress)
        }
        ClosingOverlay::Settings(draft) => {
            settings_form::modal(base, draft, scheme, progress, env_include_outcome)
        }
        ClosingOverlay::ConfirmDelete(dir) => {
            // Fading-out snapshot: the override may already be gone, so fall back to the derived
            // name for the exit animation. The live state (branch, checkbox choice) is gone by
            // now too — this non-interactive snapshot fades the dialog without the branch
            // checkbox rather than reconstructing it.
            let friendly = micold_core::naming::display_name(dir);
            confirm_delete::modal(base, dir, &friendly, None, false, scheme, progress)
        }
        ClosingOverlay::WorktreeRename(draft) => {
            worktree_rename::modal(base, draft, scheme, progress)
        }
        ClosingOverlay::ConfirmSessionRemove(label) => {
            confirm_session_remove::modal(base, label, scheme, progress)
        }
        ClosingOverlay::ConfirmForget(display_name, running) => {
            confirm_forget::modal(base, display_name, *running, scheme, progress)
        }
    }
}

/// Keyboard subscription. While a modal overlay is open, Esc dismisses it — the About
/// dialog (FR-011) or the project selector. Mirrors [`crate::app::on_escape`].
///
/// Feature 006 (FR-009): while the embedded terminal holds focus, the app binds NO global
/// keyboard shortcuts — every key is owned by the focused terminal widget (so Esc and any app
/// chord reach the `claude` process instead of driving the app).
pub fn subscription(state: &State) -> Subscription<Message> {
    if state.terminal_focused {
        return Subscription::none();
    }
    // The sidebar filter panel (feature 009) is a lightweight popover, not a modal `Overlay`,
    // so it's checked ahead of the `Overlay` match below (mirrors `on_escape`'s priority).
    if state.overlay == Overlay::None && state.sidebar_filter_open {
        return iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::SidebarFilterMenuToggled)
        });
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
        Overlay::ConfirmWorktreeDelete => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::WorktreeDeleteCancelled)
        }),
        Overlay::RenameWorktree => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::WorktreeRenameCancelled)
        }),
        Overlay::ConfirmSessionRemove => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::SessionRemoveCancelled)
        }),
        Overlay::ConfirmForgetProject => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::ProjectForgetCancelled)
        }),
    }
}
