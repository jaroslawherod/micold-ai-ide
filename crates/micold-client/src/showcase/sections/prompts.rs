//! A structured session's permission prompts (feature 942, T056): `ChoiceList` and `PromptCard`,
//! the card posed as each of the transcript's three request views, drawn by
//! `ui::transcript::permission` from the window's own prompt state.
//!
//! The samples are public so `tests/gates/prompt_card_fits.rs` lays out exactly what the gallery
//! poses.

use iced::widget::container;
use iced::{Element, Length};
use micold_core::protocol::messages::{PermissionRequest, PromptId};
use micold_core::session::SessionId;
use micold_core::structured::permission::request::{Question, QuestionOption, RequestKind};
use micold_core::theme::ColorScheme;
use micold_core::tokens::Roles;
use std::sync::LazyLock;

use crate::features::permission_prompts::State as Prompts;
use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::state::{Message, Showcase};
use crate::ui::material::ChoiceList;
use crate::ui::transcript::permission::request_view;
use crate::ui::transcript::{BlockCache, BlockRenderers, PromptsView, RenderCtx};

/// The width the prompts are posed at: about a transcript pane's in a laptop window.
const WIDTH: f32 = 560.0;

/// Which request view a [`prompt_card_sample`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptPose {
    /// A Bash call waiting for Allow, Deny or Always allow.
    Tool,
    /// A plan waiting for its approval.
    Plan,
    /// Two questions, the second multiple choice.
    Question,
}

/// The session the posed requests belong to.
static SESSION: LazyLock<SessionId> = LazyLock::new(SessionId::new);

impl PromptPose {
    fn id(self) -> PromptId {
        PromptId(match self {
            Self::Tool => 1,
            Self::Plan => 2,
            Self::Question => 3,
        })
    }

    fn request(self) -> PermissionRequest {
        let tool = PermissionRequest {
            id: self.id(),
            session: *SESSION,
            tool_use_id: None,
            tool_name: "Bash".into(),
            input_preview: r#"{"command":"cargo test --workspace","description":"Run the tests"}"#
                .into(),
            input_bytes: 66,
            kind: RequestKind::Tool,
            always_allow: Some(r#"Bash: "cargo test --workspace""#.into()),
        };
        match self {
            Self::Tool => tool,
            Self::Plan => PermissionRequest {
                tool_name: "ExitPlanMode".into(),
                kind: RequestKind::PlanApproval {
                    plan: "1. Read the parser and its tests.\n2. Add the missing case.\n\
                           3. Run the whole suite."
                        .into(),
                },
                always_allow: None,
                ..tool
            },
            Self::Question => PermissionRequest {
                tool_name: "AskUserQuestion".into(),
                kind: RequestKind::Question {
                    questions: vec![
                        question(
                            "Database",
                            "Which database should the migration target?",
                            false,
                        ),
                        question("Checks", "Which checks should run after it?", true),
                    ],
                },
                always_allow: None,
                ..tool
            },
        }
    }
}

fn question(header: &str, text: &str, multi_select: bool) -> Question {
    let options = if multi_select {
        [
            ("Unit tests", "Fast, no services"),
            ("Integration", "Needs Docker"),
        ]
    } else {
        [("Postgres", "The production store"), ("SQLite", "")]
    };
    Question {
        header: header.into(),
        text: text.into(),
        options: options
            .into_iter()
            .map(|(label, description)| QuestionOption {
                label: label.into(),
                description: description.into(),
            })
            .collect(),
        multi_select,
    }
}

/// The posed requests, the question's first answer chosen.
static PROMPTS: LazyLock<Prompts> = LazyLock::new(|| {
    let mut prompts = Prompts::default();
    for pose in [PromptPose::Tool, PromptPose::Plan, PromptPose::Question] {
        prompts.requested(pose.request());
    }
    prompts.toggle_option(*SESSION, PromptPose::Question.id(), 0, 0);
    prompts
});

/// The posed requests draw no block from the cache.
static CACHE: LazyLock<BlockCache> = LazyLock::new(BlockCache::default);

/// The request view `pose` in `scheme`, as the transcript draws a request whose card it does not
/// hold.
pub fn prompt_card_sample(pose: PromptPose, scheme: ColorScheme) -> Element<'static, Message> {
    request_view(
        PromptsView {
            state: &PROMPTS,
            on_prompt: |_| Message::NoOp,
            focused: None,
            on_focus: |_, _| Message::NoOp,
        },
        *SESSION,
        pose.id(),
        &BlockRenderers::PLAIN,
        RenderCtx {
            roles: micold_core::tokens::roles(scheme),
            scheme,
            cache: &CACHE,
        },
        |_| Message::NoOp,
    )
    .expect("the posed request is held")
}

/// `PromptCard` — a tool request, a plan approval and a question, as the transcript draws them.
pub fn prompt_card<'a>(s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let sized = |pose| container(prompt_card_sample(pose, s.scheme)).width(Length::Fixed(WIDTH));
    arrange(
        vec![
            posed("a tool request", sized(PromptPose::Tool), roles),
            posed("a plan approval", sized(PromptPose::Plan), roles),
            posed("a question", sized(PromptPose::Question), roles),
        ],
        Layout::FullWidth,
    )
}

/// `ChoiceList` — one choice with one chosen; several, with descriptions; and Other chosen, its
/// field under it.
pub fn choice_list<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let sized = |list: ChoiceList<'a, Message>| container(list).width(Length::Fixed(WIDTH));
    arrange(
        vec![
            posed(
                "single choice, one chosen",
                sized(
                    ChoiceList::new(roles)
                        .option("Postgres", "", true)
                        .option("SQLite", "", false)
                        .on_choose(|_| Message::NoOp),
                ),
                roles,
            ),
            posed(
                "multiple choice with descriptions",
                sized(
                    ChoiceList::new(roles)
                        .multiple(true)
                        .option("Unit tests", "Fast, no services", true)
                        .option("Integration", "Needs Docker", true)
                        .option("Benchmarks", "Slow; run before a release", false)
                        .on_choose(|_| Message::NoOp),
                ),
                roles,
            ),
            posed(
                "Other chosen, with its field",
                sized(
                    ChoiceList::new(roles)
                        .option("Postgres", "The production store", false)
                        .option("SQLite", "", false)
                        .on_choose(|_| Message::NoOp)
                        .other("CockroachDB", true)
                        .on_other(Message::NoOp)
                        .on_other_input(|_| Message::NoOp),
                ),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}
