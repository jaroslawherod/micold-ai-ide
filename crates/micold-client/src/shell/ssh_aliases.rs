//! SSH host aliases of github.com (#1475, 034 FR-002, research R5).
//!
//! A remote such as `git@github-work:o/r.git` names github.com only through a `Host github-work`
//! block in the user's `~/.ssh/config`, and only the user's machine has that file: a container
//! daemon has no `~/.ssh` at all. So the client resolves each such host with `ssh -G`, off the
//! update thread, and tells the daemon which ones are github.com's
//! ([`ClientMsg::SshHostAliases`]); the daemon's issue lookup, `create_worktree` and credential
//! targets then give the same answer whether or not sessions are sandboxed (FR-026).
//!
//! The daemon's `RemoteList` answers with the URLs as configured. [`prepare`] runs before the
//! answer is handled: a host `ssh -G` has not been asked about yet holds the answer back until it
//! has ([`resolve_then`]), and an alias of github.com is then rewritten to its
//! `ssh://git@github.com/…` form, so every reader of the remotes sees GitHub. Each host is asked
//! once per process.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};

use iced::Task;
use micold_client::app::Message;
use micold_client::daemon::Outbox;
use micold_core::daemons::DaemonId;
use micold_core::github::{known_github_aliases, resolve_ssh_aliases_with, ssh_alias_candidate};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult};

/// What resolves an SSH host to its real host name: `ssh -G` on this machine.
pub type SshResolver = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// `ssh -G` on this machine, and what it answered for each host asked about.
#[derive(Clone)]
pub struct SshHosts {
    resolve: Option<SshResolver>,
    answers: Arc<Mutex<BTreeMap<String, Option<String>>>>,
}

impl SshHosts {
    /// With `resolve` as `ssh -G`; `None` resolves nothing (a test's capabilities).
    pub fn new(resolve: Option<SshResolver>) -> Self {
        Self {
            resolve,
            answers: Arc::default(),
        }
    }

    fn answers(&self) -> MutexGuard<'_, BTreeMap<String, Option<String>>> {
        self.answers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The alias candidates among `urls` that `ssh -G` has not been asked about, each once.
    fn unasked<'a>(&self, urls: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        if self.resolve.is_none() {
            return Vec::new();
        }
        let answers = self.answers();
        let hosts: BTreeSet<String> = urls
            .into_iter()
            .filter_map(ssh_alias_candidate)
            .filter(|h| !answers.contains_key(h))
            .collect();
        hosts.into_iter().collect()
    }

    /// Ask `ssh -G` about `hosts` and remember every answer, a failure too. Blocking.
    pub fn ask(&self, hosts: &[String]) {
        let Some(resolve) = &self.resolve else {
            return;
        };
        for host in hosts {
            // Recorded first, so a resolver that panics cannot leave the host unasked and hold
            // the replayed event again.
            self.answers().insert(host.clone(), None);
            let real = resolve(host);
            self.answers().insert(host.clone(), real);
        }
    }

    /// Every host `ssh -G` resolved to github.com or ssh.github.com.
    fn github_aliases(&self) -> BTreeSet<String> {
        self.answers()
            .iter()
            .filter(|(_, real)| matches!(real.as_deref(), Some("github.com" | "ssh.github.com")))
            .map(|(host, _)| host.clone())
            .collect()
    }

    /// Whether one of `urls` is on an alias of github.com.
    fn names_a_github_alias<'a>(&self, urls: impl IntoIterator<Item = &'a str>) -> bool {
        let aliases = self.github_aliases();
        urls.into_iter()
            .filter_map(ssh_alias_candidate)
            .any(|h| aliases.contains(&h))
    }

    /// Tell the daemon behind `outbox` every alias of github.com, when `urls` name one: the
    /// daemon holds what it was told, so a reply naming none needs no message.
    fn tell<'a>(&self, outbox: Option<&Outbox>, urls: impl IntoIterator<Item = &'a str>) {
        if let Some(outbox) = outbox {
            if self.names_a_github_alias(urls) {
                outbox.send(ClientMsg::SshHostAliases {
                    github_hosts: self.github_aliases().into_iter().collect(),
                });
            }
        }
    }
}

/// What [`prepare`] made of an event.
pub enum Prepared {
    /// Handle it now.
    Ready(DaemonMsg),
    /// Ask `ssh -G` about `hosts` first ([`resolve_then`]), then handle it.
    Resolve {
        /// The hosts not asked about yet.
        hosts: Vec<String>,
        /// The event, unchanged.
        event: DaemonMsg,
    },
}

/// Before `event` from the daemon behind `outbox` is handled. A `RemoteList` answer naming a
/// host not asked about yet waits for `ssh -G`; otherwise its aliases of github.com are rewritten
/// and the daemon is told them. The credential targets are handled at once: `ssh -G` runs on a
/// thread of its own and the daemon, told the aliases, announces the targets again.
pub fn prepare(hosts: &SshHosts, outbox: Option<&Outbox>, event: DaemonMsg) -> Prepared {
    match event {
        DaemonMsg::OperationOk {
            req,
            result: OperationResult::RemoteList { remotes },
        } => {
            let urls = || remotes.iter().map(|r| r.url.as_str());
            let unasked = hosts.unasked(urls());
            if !unasked.is_empty() {
                return Prepared::Resolve {
                    hosts: unasked,
                    event: DaemonMsg::OperationOk {
                        req,
                        result: OperationResult::RemoteList { remotes },
                    },
                };
            }
            hosts.tell(outbox, urls());
            let aliases = hosts.github_aliases();
            let remotes = resolve_ssh_aliases_with(remotes, &known_github_aliases(&aliases, None));
            Prepared::Ready(DaemonMsg::OperationOk {
                req,
                result: OperationResult::RemoteList { remotes },
            })
        }
        DaemonMsg::GitCredentialTargets { ref targets, .. } => {
            let urls: Vec<String> = targets.iter().filter_map(|t| t.push_url.clone()).collect();
            let unasked = hosts.unasked(urls.iter().map(String::as_str));
            if unasked.is_empty() {
                hosts.tell(outbox, urls.iter().map(String::as_str));
            } else {
                let (hosts, outbox) = (hosts.clone(), outbox.cloned());
                std::thread::spawn(move || {
                    hosts.ask(&unasked);
                    hosts.tell(outbox.as_ref(), urls.iter().map(String::as_str));
                });
            }
            Prepared::Ready(event)
        }
        event => Prepared::Ready(event),
    }
}

/// Ask `ssh -G` about `hosts` on a blocking thread, then hand `event` from `daemon` back to the
/// update loop, where [`prepare`] finds every host answered.
pub fn resolve_then(
    ssh: SshHosts,
    hosts: Vec<String>,
    daemon: DaemonId,
    event: DaemonMsg,
) -> Task<Message> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || ssh.ask(&hosts)).await;
        },
        move |()| {
            Message::daemon(
                daemon,
                micold_client::features::connection::Msg::Event(event),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::futures::channel::mpsc;
    use micold_core::git::GitRemote;

    fn stub() -> SshHosts {
        SshHosts::new(Some(Arc::new(|h: &str| match h {
            "github-micold" => Some("github.com".to_string()),
            "gitlab-alias" => Some("gitlab.com".to_string()),
            _ => None,
        })))
    }

    fn listed(urls: &[&str]) -> DaemonMsg {
        DaemonMsg::OperationOk {
            req: 7,
            result: OperationResult::RemoteList {
                remotes: urls
                    .iter()
                    .map(|u| GitRemote {
                        name: "origin".into(),
                        url: (*u).into(),
                    })
                    .collect(),
            },
        }
    }

    fn urls_of(prepared: Prepared) -> Vec<String> {
        match prepared {
            Prepared::Ready(DaemonMsg::OperationOk {
                result: OperationResult::RemoteList { remotes },
                ..
            }) => remotes.into_iter().map(|r| r.url).collect(),
            _ => panic!("expected a ready RemoteList"),
        }
    }

    #[test]
    fn an_alias_waits_for_ssh_then_reads_as_github_and_the_daemon_is_told() {
        let hosts = stub();
        let (tx, mut rx) = mpsc::unbounded();
        let outbox = Outbox::new(tx);
        let event = listed(&["git@github-micold:o/r.git", "git@gitlab-alias:x/y.git"]);
        let Prepared::Resolve {
            hosts: asked,
            event,
        } = prepare(&hosts, Some(&outbox), event)
        else {
            panic!("an unasked host waits for ssh -G");
        };
        assert_eq!(asked, vec!["github-micold", "gitlab-alias"]);
        assert!(
            rx.try_recv().is_err(),
            "nothing is sent before ssh -G answered"
        );

        hosts.ask(&asked);
        let urls = urls_of(prepare(&hosts, Some(&outbox), event));
        assert_eq!(
            urls,
            vec!["ssh://git@github.com/o/r.git", "git@gitlab-alias:x/y.git"]
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            ClientMsg::SshHostAliases {
                github_hosts: vec!["github-micold".into()]
            }
        );
    }

    #[test]
    fn no_alias_and_no_ssh_change_nothing_and_send_nothing() {
        let (tx, mut rx) = mpsc::unbounded();
        let outbox = Outbox::new(tx);
        let urls = ["git@github.com:o/r.git", "https://example.com/o/r"];
        assert_eq!(
            urls_of(prepare(&stub(), Some(&outbox), listed(&urls))),
            urls
        );
        // A test's capabilities: no `ssh` at all, so an alias is never held back.
        let none = SshHosts::new(None);
        assert_eq!(
            urls_of(prepare(
                &none,
                Some(&outbox),
                listed(&["git@github-micold:o/r"])
            )),
            vec!["git@github-micold:o/r"]
        );
        assert!(rx.try_recv().is_err());
    }
}
