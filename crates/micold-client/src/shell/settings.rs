//! The effectful half of the settings feature: shape B (feature 028, contract M2).
//!
//! `features/settings.rs` holds every settings transition, and all of them are pure. Four
//! additionally need something done at the I/O boundary — `settings.json` written, or the values
//! the shell owns read back into the freshly opened draft — and an `iced::Task` is what says so.
//! That is the line M2 draws: an arm belongs here when it must return a `Task`, and in the feature
//! otherwise.
//!
//! The bodies themselves did not move. They are still [`crate::shell::persist`]'s, called from
//! here rather than from a per-variant arm in `main.rs`; what changed is that `main.rs` now has
//! one settings arm instead of three, and the routing decision — which of them need an effect —
//! is stated once, in this file, next to the effects.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::settings::Msg;

use micold_core::scoped_credential::pat;
use micold_core::scoped_credential::pat_store::PatStore;

use crate::shell::persist;
use crate::shell::scoped_credential::{check_and_store, UreqTransport};
use crate::App;

/// Read the stored GitHub tokens into the section (Settings opened).
fn load_github_tokens(app: &mut App) {
    let stored = PatStore::on_device()
        .map(|store| store.list())
        .unwrap_or_default()
        .into_iter()
        .map(|(repo, pat)| (repo, pat.expires_at))
        .collect();
    app.core
        .update(Message::Settings(Msg::GithubPatsLoaded(stored)));
}

/// Tell the credential wiring about the tokens the section replaced or removed.
fn refresh_github_tokens(app: &mut App) {
    for repo in app.core.settings.github_tokens.take_refresh() {
        app.git_credentials.refresh(repo);
    }
}

/// Check the token the form raised and, if GitHub accepts it, store it: on a blocking thread, the
/// answer comes back as [`Msg::GithubPatChecked`].
fn check_github_token(app: &mut App) -> Task<Message> {
    let Some(request) = app.core.settings.github_tokens.take_request() else {
        return Task::none();
    };
    let answered = request.repo.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                check_and_store(
                    &UreqTransport::default(),
                    PatStore::on_device().as_ref(),
                    &request.repo,
                    pat::normalize(request.token.as_str()),
                    micold_core::unix_time::unix_secs(),
                )
            })
            .await
            .unwrap_or_else(|_| Err("The check did not finish.".to_string()))
        },
        move |result| {
            Message::Settings(Msg::GithubPatChecked {
                repo: answered.clone(),
                result,
            })
        },
    )
}

/// This feature's effectful entry point: one arm in `main.rs` routes here (contract M2).
///
/// The `pure` arm is not a fallback that might be wrong — it is every transition other than the
/// four named above, each of which finishes in the reducer. They reach `State::update` through the same wrapper variant they arrived
/// under, so the pure path is identical whether or not the shell was in the way.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    match msg {
        Msg::Opened => {
            let task = persist::on_settings_opened(app);
            load_github_tokens(app);
            task
        }
        // #1002: a pasted token is checked off the UI thread and stored only if GitHub accepts it.
        Msg::GithubPatAddRequested => {
            app.core.update(Message::Settings(msg));
            check_github_token(app)
        }
        Msg::GithubPatChecked { .. } => {
            app.core.update(Message::Settings(msg));
            refresh_github_tokens(app);
            Task::none()
        }
        Msg::GithubPatRemoveRequested(repo) => {
            let removed = PatStore::on_device().map_or(Ok(()), |store| store.remove(&repo));
            match removed {
                Ok(()) => {
                    app.core
                        .update(Message::Settings(Msg::GithubPatRemoveRequested(repo)));
                    refresh_github_tokens(app);
                }
                Err(e) => app
                    .core
                    .notify_error(format!("Couldn't remove the GitHub token: {e}")),
            }
            Task::none()
        }
        Msg::Saved => persist::on_settings_saved(app),
        // Feature 533: the answer to the Keep-or-Remove question is applied by the save it held.
        Msg::JiraKeepOrRemoveChosen(_) => {
            app.core.update(Message::Settings(msg));
            persist::on_settings_saved(app)
        }
        // The theme variant does not bind, so `msg` is still whole to re-wrap:
        // `on_theme_changed` applies it through the reducer and then persists.
        Msg::ThemePreferenceChanged(_) => persist::on_theme_changed(app, Message::Settings(msg)),
        pure => {
            app.core.update(Message::Settings(pure));
            Task::none()
        }
    }
}
