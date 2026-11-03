//! The Start script group of a daemon's form (#1533, FR-010): the script (a picked file or text
//! written inline) and the other `[startup]` keys, as typed, and what Save makes of them.
//!
//! Render-free. The group is offered only for a kind whose script the application delivers
//! ([`micold_core::daemons::start_script_delivered`]), so none is savable without delivery.

use std::path::PathBuf;

use micold_core::daemon_config::{Kind, OnFailure, RunPolicy};
use micold_core::daemons::{StartupScript, StartupSettings};

use super::daemons::{DraftRuntime, FormError};
use crate::app::Editor;

/// The shortest and longest `startup.timeout_secs` the daemon accepts.
pub const TIMEOUT_RANGE: std::ops::RangeInclusive<i64> = 1..=3600;

/// Where the script comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartSource {
    /// No start script.
    #[default]
    None,
    /// A script file on this machine.
    File,
    /// Text written in the form, saved as a file in the application's data.
    Inline,
}

/// The Start script fields as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartDraft {
    pub source: StartSource,
    /// The picked file's path; empty means none.
    pub file: String,
    /// The inline script's editor, beside its text.
    pub editor: Editor,
    pub text: String,
    /// `startup.shell`; empty means the daemon's default.
    pub shell: String,
    /// `startup.timeout_secs` as typed; empty means no timeout.
    pub timeout: String,
    pub on_failure: OnFailure,
    pub run: RunPolicy,
    /// `startup.env_file`; empty means none.
    pub env_file: String,
    pub export_env: bool,
}

impl Default for StartDraft {
    fn default() -> Self {
        Self::new(None, String::new())
    }
}

impl StartDraft {
    /// The fields of a stored script, with the text of an inline one.
    pub fn new(stored: Option<&StartupSettings>, inline_text: String) -> Self {
        let text = |p: &Option<PathBuf>| {
            p.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let Some(s) = stored else {
            return Self {
                source: StartSource::None,
                file: String::new(),
                editor: Editor::new(""),
                text: String::new(),
                shell: String::new(),
                timeout: String::new(),
                on_failure: OnFailure::default(),
                run: RunPolicy::default(),
                env_file: String::new(),
                export_env: true,
            };
        };
        let (source, file, text_in) = match &s.script {
            StartupScript::File(path) => (
                StartSource::File,
                path.to_string_lossy().into_owned(),
                String::new(),
            ),
            StartupScript::Inline => (StartSource::Inline, String::new(), inline_text),
        };
        Self {
            source,
            file,
            editor: Editor::new(&text_in),
            text: text_in,
            shell: text(&s.shell),
            timeout: s.timeout_secs.map(|t| t.to_string()).unwrap_or_default(),
            on_failure: s.on_failure,
            run: s.run,
            env_file: text(&s.env_file),
            export_env: s.export_env,
        }
    }

    /// The settings Save stores, or the field at fault: `None` when there is no script.
    pub fn settings(&self) -> Result<Option<StartupSettings>, FormError> {
        let path = |text: &str| {
            Some(text.trim())
                .filter(|t| !t.is_empty())
                .map(PathBuf::from)
        };
        let script = match self.source {
            StartSource::None => return Ok(None),
            StartSource::File => StartupScript::File(path(&self.file).ok_or(FormError {
                field: "startup_script",
                message: "Pick the script file, or write the script inline".to_string(),
            })?),
            StartSource::Inline => {
                if self.text.trim().is_empty() {
                    return Err(FormError {
                        field: "startup_script",
                        message: "Write the script, or pick a file".to_string(),
                    });
                }
                StartupScript::Inline
            }
        };
        Ok(Some(StartupSettings {
            script,
            shell: path(&self.shell),
            timeout_secs: self.timeout_secs()?,
            on_failure: self.on_failure,
            run: self.run,
            env_file: path(&self.env_file),
            export_env: self.export_env,
        }))
    }

    fn timeout_secs(&self) -> Result<Option<i64>, FormError> {
        let typed = self.timeout.trim();
        if typed.is_empty() {
            return Ok(None);
        }
        typed
            .parse::<i64>()
            .ok()
            .filter(|secs| TIMEOUT_RANGE.contains(secs))
            .map(Some)
            .ok_or_else(|| FormError {
                field: "startup_timeout",
                message: format!(
                    "The timeout is a whole number of seconds from {} to {}",
                    TIMEOUT_RANGE.start(),
                    TIMEOUT_RANGE.end()
                ),
            })
    }

    /// The inline script's text to save, when the script is inline.
    pub fn inline_text(&self) -> Option<&str> {
        (self.source == StartSource::Inline).then_some(self.text.as_str())
    }

    /// The inline text was replaced.
    pub fn set_text(&mut self, text: String) {
        self.editor = Editor::new(&text);
        self.text = text;
    }
}

/// The daemon kind a form's runtime makes.
pub fn kind_of(runtime: DraftRuntime) -> Kind {
    match runtime {
        DraftRuntime::Host => Kind::Host,
        DraftRuntime::Ssh => Kind::Ssh,
        DraftRuntime::Container => Kind::Container,
        DraftRuntime::Kubernetes => Kind::Kubernetes,
    }
}

/// The note under a daemon's row while a start script change waits for a restart (US5.2).
pub const RESTART_TO_APPLY: &str =
    "Restart to apply: the start script or its settings changed and run at the next start.";

/// Whether the daemon's changed restart-only keys (`ConfigStatus.changed_restart_keys`) include
/// the `[startup]` table or a key in it.
pub fn startup_restart_pending(changed_restart_keys: &[String]) -> bool {
    changed_restart_keys
        .iter()
        .any(|k| k == "startup" || k.starts_with("startup."))
}
