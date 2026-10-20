//! Default prompt template (feature 1161): one pure renderer shared by the create-worktree dialog
//! and the daemon's MCP tools (FR-011).
//!
//! Syntax: `{name}` substitutes one of [`PromptTemplate::PLACEHOLDERS`]; `{{` and `}}` are literal
//! braces; any other brace is a [`TemplateError`]. One left-to-right scan, and substituted values
//! are appended to the output and never scanned again (research R2).

use std::fmt;

/// A value a template can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placeholder {
    Issue,
    IssueTitle,
    IssueBody,
    Branch,
    Type,
    Name,
}

impl Placeholder {
    const ALL: [Self; 6] = [
        Self::Issue,
        Self::IssueTitle,
        Self::IssueBody,
        Self::Branch,
        Self::Type,
        Self::Name,
    ];

    /// The spelling between the braces.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::IssueTitle => "issue_title",
            Self::IssueBody => "issue_body",
            Self::Branch => "branch",
            Self::Type => "type",
            Self::Name => "name",
        }
    }

    fn from_spelling(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.spelling() == name)
    }
}

/// Why a template text is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateError {
    /// A `{name}` that is not a placeholder; holds the name as written.
    Unknown(String),
    /// A lone `{` or `}`.
    Unbalanced,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => {
                write!(f, "Unknown placeholder {{{name}}}; valid: ")?;
                for (i, p) in Placeholder::ALL.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{{{}}}", p.spelling())?;
                }
                Ok(())
            }
            Self::Unbalanced => {
                f.write_str("Unbalanced brace in the template; use {{ and }} for literal braces")
            }
        }
    }
}

impl std::error::Error for TemplateError {}

/// What a template is rendered against. Empty means absent (FR-004). Never persisted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorktreeFacts {
    pub issue: String,
    pub issue_title: String,
    pub issue_body: String,
    pub branch: String,
    pub type_: String,
    pub name: String,
}

impl WorktreeFacts {
    fn value(&self, p: Placeholder) -> &str {
        match p {
            Placeholder::Issue => &self.issue,
            Placeholder::IssueTitle => &self.issue_title,
            Placeholder::IssueBody => &self.issue_body,
            Placeholder::Branch => &self.branch,
            Placeholder::Type => &self.type_,
            Placeholder::Name => &self.name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Placeholder(Placeholder),
}

/// A template text that parsed: the only thing [`PromptTemplate::render`] accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTemplate {
    segments: Vec<Segment>,
}

impl PromptTemplate {
    /// The placeholder spellings, in the order of the error message.
    pub const PLACEHOLDERS: [&'static str; 6] = [
        Placeholder::ALL[0].spelling(),
        Placeholder::ALL[1].spelling(),
        Placeholder::ALL[2].spelling(),
        Placeholder::ALL[3].spelling(),
        Placeholder::ALL[4].spelling(),
        Placeholder::ALL[5].spelling(),
    ];

    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '}' => return Err(TemplateError::Unbalanced),
                '{' => {
                    let mut name = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some('{') | None => return Err(TemplateError::Unbalanced),
                            Some(ch) => name.push(ch),
                        }
                    }
                    let p =
                        Placeholder::from_spelling(&name).ok_or(TemplateError::Unknown(name))?;
                    if !literal.is_empty() {
                        segments.push(Segment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(Segment::Placeholder(p));
                }
                other => literal.push(other),
            }
        }
        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }
        Ok(Self { segments })
    }

    /// The prompt for `facts`; `None` when the result is blank after trimming (FR-006).
    pub fn render(&self, facts: &WorktreeFacts) -> Option<String> {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(text) => out.push_str(text),
                Segment::Placeholder(p) => out.push_str(facts.value(*p)),
            }
        }
        (!out.trim().is_empty()).then_some(out)
    }
}

/// Render stored template text, treating empty, invalid or blank-rendering text as no prompt
/// (FR-006, FR-012, FR-013). The one call both the dialog and the MCP tools make (FR-011).
pub fn render_text(text: &str, facts: &WorktreeFacts) -> Option<String> {
    PromptTemplate::parse(text).ok()?.render(facts)
}

/// One template, facts and expected output, so the dialog and daemon tests assert against the
/// same rendering (FR-011).
#[doc(hidden)]
pub mod shared_fixture {
    use super::WorktreeFacts;

    pub const TEMPLATE: &str =
        "Fix #{issue} {issue_title} on {branch} ({type}/{name}) {{ok}}\n{issue_body}";
    pub const OUTPUT: &str =
        "Fix #42 Crash on save on fix/42_crash (fix/crash) {ok}\nIt crashes. {issue}";

    pub fn facts() -> WorktreeFacts {
        WorktreeFacts {
            issue: "42".into(),
            issue_title: "Crash on save".into(),
            issue_body: "It crashes. {issue}".into(),
            branch: "fix/42_crash".into(),
            type_: "fix".into(),
            name: "crash".into(),
        }
    }
}
