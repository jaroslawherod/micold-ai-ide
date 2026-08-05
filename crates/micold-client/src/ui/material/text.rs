//! `Text` — the library's wrapper around the rendering stack's text widget (Principle VIII).
//!
//! Call sites used to pick a *number*: 135 of them named a raw size from the type scale, and each
//! one was free to pick a different number for the same kind of text. A label that should have been
//! `LABEL` was `BODY` in four places, and nothing could tell you which was intended.
//!
//! A call site now names a **role** — what this text *is* — and the role decides the size. That is
//! what makes feature 018's type scale a single edit: roles gain weight and line height in one
//! place, and every call site follows without being touched.
//!
//! Parity: each role resolves to exactly the size that role's call sites use today (FR-005).

use std::borrow::Cow;
use std::marker::PhantomData;

use crate::ui::material::style;
use iced::widget::text;
use iced::widget::text::LineHeight;
use iced::{Element, Font, Length};
use micold_core::tokens::{typography, Rgb, Roles};

/// What a piece of text *is*, rather than how big it is.
///
/// The sidebar roles are separate rather than a modifier because the sidebar runs at its own
/// density — an explicit, auditable 80% of the body/label roles — and that decision belongs in one
/// place rather than at every sidebar call site.
///
/// # Choosing between [`Body`](Self::Body), [`Caption`](Self::Caption) and [`Label`](Self::Label)
///
/// Material splits these on *what the text does*, not on how big it is, and the split is the whole
/// reason `Caption` and `Label` are both here at 12dp:
///
/// - **prose** — anything that reads as a sentence, whatever its size — is a `body` role, set at
///   weight 400. Sentences at weight 500 read as shouting, and this application had a lot of them:
///   error lines, diagnostics, "This repository has no other branches."
/// - **an interface label** — a terse word or fragment naming a thing, never a sentence — is a
///   `label` role, at weight 500. "git", "Type", "Branch:".
///
/// So `Caption` and `Label` are the same size and differ only in weight, deliberately. Picking
/// between them is picking whether the text is something you *read* or something you *scan*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRole {
    /// The largest role. Used once, for the empty-state headline.
    Display,
    /// A dialog or screen title.
    Headline,
    /// A screen-level title — the largest thing inside a surface rather than above it.
    Title,
    /// A heading introducing a group *within* a surface: a sidebar's header, a list's subheading.
    ///
    /// Distinct from [`Title`](Self::Title) because 22dp at weight 400 is a screen title, and a
    /// group heading nested inside a panel that already has one has to sit below it in the
    /// hierarchy — Material's `title_medium` is 16dp at weight 500, which reads as a heading
    /// through weight rather than through size.
    Section,
    /// Ordinary running text — the default for anything that is not one of the others.
    Body,
    /// Supporting prose: hints, error lines, diagnostics, explanatory sentences.
    ///
    /// The same size as [`Label`](Self::Label) and deliberately a *lighter* weight, because this is
    /// text to be read rather than scanned. See the type-level docs.
    Caption,
    /// Text on something you can press: a button, a menu item, a chip.
    ///
    /// Material's `label_large`, and one of the most recognisable things about the type system —
    /// button text is medium weight. Rendering it at the body weight is the same size and reads as
    /// a different design language, which is exactly the gap feature 018 exists to close.
    Action,
    /// A terse interface label naming a thing: a field's name, a badge, a count.
    Label,
    /// A worktree or session name in the sidebar.
    SidebarName,
    /// A tag chip's text in the sidebar.
    SidebarTag,
    /// A session line in the sidebar.
    SidebarSession,
}

/// The embedded Roboto faces. Registered once at startup in `main` so the application looks the
/// same on every platform rather than inheriting the OS UI font (FR-008); see
/// `assets/fonts/PROVENANCE.md`. Weight 400 and 500 are the only weights the Material 3 type scale
/// specifies, which is why exactly two static instances ship.
pub const ROBOTO_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../../../assets/fonts/Roboto-Regular.ttf");
pub const ROBOTO_MEDIUM_BYTES: &[u8] =
    include_bytes!("../../../../../assets/fonts/Roboto-Medium.ttf");

/// The family both shipped faces belong to (asserted by `tests/roboto_font.rs`). The Medium face
/// advertises `Roboto Medium` in its legacy family name and `Roboto` as its typographic family, so
/// both resolve here and are told apart by weight.
pub const ROBOTO: Font = Font::with_name("Roboto");

impl TypeRole {
    /// The Material 3 role this resolves to (contract §2.4, §2.5).
    ///
    /// The mapping is deliberately a *narrowing*: the scale has fifteen roles and this enum names
    /// the eleven the application actually distinguishes. The others exist in the scale and are
    /// reachable, but no call site is required to use one — Material's true display sizes (36–57)
    /// are larger than anything this app renders.
    ///
    /// This table is the whole of the app's typographic vocabulary, so it is pinned by
    /// `tests/type_role_mapping.rs` rather than left to be re-derived by reading call sites.
    pub fn resolved(self) -> typography::TypeRole {
        match self {
            // Feature 003's `DISPLAY` (32) was a headline in Material's vocabulary, not a display.
            TypeRole::Display => typography::HEADLINE_LARGE,
            TypeRole::Headline => typography::HEADLINE_SMALL,
            TypeRole::Title => typography::TITLE_LARGE,
            TypeRole::Section => typography::TITLE_MEDIUM,
            TypeRole::Body => typography::BODY_MEDIUM,
            TypeRole::Caption => typography::BODY_SMALL,
            TypeRole::Action => typography::LABEL_LARGE,
            TypeRole::Label => typography::LABEL_MEDIUM,
            TypeRole::SidebarName => typography::SIDEBAR_NAME,
            TypeRole::SidebarTag => typography::SIDEBAR_TAG,
            TypeRole::SidebarSession => typography::SIDEBAR_SESSION,
        }
    }

    /// Every role a call site can name, so a test or the showcase can enumerate them without
    /// restating the list (and drifting from it).
    pub const ALL: [TypeRole; 11] = [
        TypeRole::Display,
        TypeRole::Headline,
        TypeRole::Title,
        TypeRole::Section,
        TypeRole::Body,
        TypeRole::Caption,
        TypeRole::Action,
        TypeRole::Label,
        TypeRole::SidebarName,
        TypeRole::SidebarTag,
        TypeRole::SidebarSession,
    ];

    /// The role's name as the enum spells it — for the showcase's per-role labels and for test
    /// failures that need to name the offender.
    pub fn name(self) -> &'static str {
        match self {
            TypeRole::Display => "Display",
            TypeRole::Headline => "Headline",
            TypeRole::Title => "Title",
            TypeRole::Section => "Section",
            TypeRole::Body => "Body",
            TypeRole::Caption => "Caption",
            TypeRole::Action => "Action",
            TypeRole::Label => "Label",
            TypeRole::SidebarName => "SidebarName",
            TypeRole::SidebarTag => "SidebarTag",
            TypeRole::SidebarSession => "SidebarSession",
        }
    }

    /// The role's size in dp.
    pub fn size(self) -> f32 {
        self.resolved().size
    }

    /// The font this role is set in — Roboto at the role's weight.
    ///
    /// Only 400 and 500 occur, and they are the two faces that ship. Anything else would silently
    /// resolve to whichever face the matcher considered closest.
    pub fn font(self) -> Font {
        Font {
            weight: match self.resolved().weight {
                w if w >= 500 => iced::font::Weight::Medium,
                _ => iced::font::Weight::Normal,
            },
            ..ROBOTO
        }
    }

    /// The role's line height in dp, as a plain number.
    ///
    /// For the caller that needs the *number* rather than the renderer's type — sizing a panel
    /// whose height depends on how tall its lines are. Everything that renders text wants
    /// [`Self::line_height`] instead.
    pub fn line_height_dp(self) -> f32 {
        self.resolved().line_height
    }

    /// The role's line height, as an absolute dp value rather than a multiple of the size.
    ///
    /// Absolute because that is how Material states it, and because a ratio drifts: `20/14` is not
    /// a number anyone would write down, and rounding it per role reintroduces exactly the
    /// per-call-site variation the scale removes (FR-007).
    pub fn line_height(self) -> LineHeight {
        LineHeight::Absolute(self.resolved().line_height.into())
    }
}

/// How strongly the text reads against its surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Emphasis {
    /// Inherits the surrounding foreground colour.
    Inherited,
    /// The muted foreground, for supporting text.
    Muted,
    /// An explicit role colour.
    Tinted(Rgb),
}

/// A piece of text at a type role. Builder form (Principle VIII):
/// `Text::new("Cancel", TypeRole::Body, roles).muted().into()`.
pub struct Text<'a, M> {
    content: Cow<'a, str>,
    role: TypeRole,
    roles: Roles,
    emphasis: Emphasis,
    width: Option<Length>,
    height: Option<Length>,
    align_y: Option<iced::alignment::Vertical>,
    marker: PhantomData<M>,
}

impl<'a, M: 'a> Text<'a, M> {
    /// Text reading `content` at `role`, themed by `roles`. Inherits the surrounding foreground
    /// colour; call [`Self::muted`] or [`Self::tint`] to change that.
    pub fn new(content: impl Into<Cow<'a, str>>, role: TypeRole, roles: Roles) -> Self {
        Self {
            content: content.into(),
            role,
            roles,
            emphasis: Emphasis::Inherited,
            width: None,
            height: None,
            align_y: None,
            marker: PhantomData,
        }
    }

    /// Supporting text: the muted foreground rather than the full-strength one.
    pub fn muted(mut self) -> Self {
        self.emphasis = Emphasis::Muted;
        self
    }

    /// Colour the text with an explicit role — for text that carries a meaning of its own, such as
    /// an error line or a tag's own accent.
    pub fn tint(mut self, color: Rgb) -> Self {
        self.emphasis = Emphasis::Tinted(color);
        self
    }

    /// Lay the text out at a given width — most often `Length::Fill`, so a following element is
    /// pushed to the far edge of its row.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Lay the text out at a given height, and optionally align it within that height. Used where
    /// a line has to match the height of something beside it.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Align the text vertically within its height.
    pub fn align_y(mut self, align: iced::alignment::Vertical) -> Self {
        self.align_y = Some(align);
        self
    }
}

impl<'a, M: 'a> From<Text<'a, M>> for Element<'a, M> {
    fn from(t: Text<'a, M>) -> Self {
        let mut widget = text(t.content)
            .size(t.role.size())
            .font(t.role.font())
            .line_height(t.role.line_height());
        match t.emphasis {
            // No style call at all, so the text inherits — which is what an unstyled call site
            // does today. Setting an explicit colour here would silently override a container's
            // foreground and would not be parity.
            Emphasis::Inherited => {}
            Emphasis::Muted => widget = widget.style(style::muted(t.roles)),
            Emphasis::Tinted(color) => widget = widget.color(style::color(color)),
        }
        if let Some(width) = t.width {
            widget = widget.width(width);
        }
        if let Some(height) = t.height {
            widget = widget.height(height);
        }
        if let Some(align) = t.align_y {
            widget = widget.align_y(align);
        }
        widget.into()
    }
}
