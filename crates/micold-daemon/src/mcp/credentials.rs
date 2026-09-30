//! The per-session bearer credentials of the tool server (feature 034, contracts/binding.md §2).
//!
//! Each session is issued one random credential, held only in memory: a service restart issues new
//! ones and no earlier credential is accepted again (FR-006). A credential names exactly one session,
//! which is how the server knows who is calling without trusting anything in the request body.

use std::collections::HashMap;
use std::sync::Mutex;

use micold_core::session::SessionId;
use uuid::Uuid;

/// The live credential registry. Credentials are never logged (SC-006).
#[derive(Default)]
pub struct Credentials {
    inner: Mutex<Maps>,
}

#[derive(Default)]
struct Maps {
    by_credential: HashMap<String, SessionId>,
    by_session: HashMap<SessionId, String>,
}

impl Credentials {
    /// The credential of `session`, issuing a fresh random one the first time. Stable for as long as
    /// the session keeps it.
    pub fn credential_for(&self, session: SessionId) -> String {
        let mut maps = self.lock();
        if let Some(existing) = maps.by_session.get(&session) {
            return existing.clone();
        }
        let credential = Uuid::new_v4().simple().to_string();
        maps.by_credential.insert(credential.clone(), session);
        maps.by_session.insert(session, credential.clone());
        credential
    }

    /// The session `credential` names, if it is live. A whole-string match: a prefix, a suffix or
    /// any other near miss names nothing.
    pub fn session_for(&self, credential: &str) -> Option<SessionId> {
        self.lock().by_credential.get(credential).copied()
    }

    /// Withdraw `session`'s credential. Returns whether it had one.
    pub fn revoke(&self, session: SessionId) -> bool {
        let mut maps = self.lock();
        match maps.by_session.remove(&session) {
            Some(credential) => {
                maps.by_credential.remove(&credential);
                true
            }
            None => false,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Maps> {
        self.inner.lock().expect("credential registry lock poisoned")
    }
}
