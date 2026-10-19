//! The titled sections a daemon's settings are grouped in, per kind of daemon (feature 890,
//! FR-001 to FR-004). View state only: which are open is never saved. Render-free, so the table of
//! which section holds which field is tested without a window.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::daemons::{AllowedKind, Draft, DraftRuntime, ImageKind};
use super::settings::github_tokens::utc_date;
use super::window::FieldId;
use micold_core::daemons::CredentialLine;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::{CredentialShare, NetworkPosture};
use micold_core::scoped_credential::{CredentialStatus, UnixTime, EXPIRY_WARNING_SECS};
use micold_core::session::AiCli;

/// What joins the parts of a closed section's summary (U+00B7 between spaces).
const SEPARATOR: &str = " \u{b7} ";

/// A titled, collapsible group of a daemon form's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    /// How to reach the daemon: names, ports, context, host, connect at start.
    Connection,
    /// A container's runtime and image.
    Container,
    /// A pod's image and the AI CLIs installed in it.
    Image,
    /// What the daemon may use of the user's credentials.
    Credentials,
    /// The container's network posture.
    Network,
    /// Processor, memory, process and storage limits, requests and limits.
    Resources,
    /// A pod's node selector, tolerations and storage.
    Placement,
    /// Survive-logout.
    Sessions,
}

const CONTAINER: &[Section] = &[
    Section::Connection,
    Section::Container,
    Section::Credentials,
    Section::Network,
    Section::Resources,
    Section::Sessions,
];
const KUBERNETES: &[Section] = &[
    Section::Connection,
    Section::Image,
    Section::Credentials,
    Section::Resources,
    Section::Placement,
];
const SSH: &[Section] = &[Section::Connection];
const HOST: &[Section] = &[Section::Connection, Section::Sessions];

impl Section {
    /// The sections of a daemon of `runtime`, in the order the form lists them (FR-002).
    pub fn of(runtime: DraftRuntime) -> &'static [Section] {
        match runtime {
            DraftRuntime::Container => CONTAINER,
            DraftRuntime::Kubernetes => KUBERNETES,
            DraftRuntime::Ssh => SSH,
            DraftRuntime::Host => HOST,
        }
    }

    /// The header text; one name for one concept in every kind (FR-003).
    pub fn title(self) -> &'static str {
        match self {
            Section::Connection => "Connection",
            Section::Container => "Container",
            Section::Image => "Image",
            Section::Credentials => "Credentials",
            Section::Network => "Network",
            Section::Resources => "Resources",
            Section::Placement => "Placement and storage",
            Section::Sessions => "Sessions",
        }
    }

    /// The section of a daemon of `runtime` that holds the field a refused save names (a
    /// `FormError::field`). `None` for `name` and `runtime`, which sit outside every section.
    pub fn holding_error(runtime: DraftRuntime, field: &str) -> Option<Section> {
        use DraftRuntime::{Container, Host, Kubernetes, Ssh};
        let section = match (runtime, field) {
            (Container, "container_name" | "port") => Section::Connection,
            (Container, "image" | "image_path") => Section::Container,
            (Container | Kubernetes, "cpus" | "memory" | "pids" | "storage") => Section::Resources,
            (Kubernetes, "context" | "namespace") => Section::Connection,
            (Kubernetes, "image") => Section::Image,
            (Kubernetes, "allowed" | "pull_file") => Section::Credentials,
            (Kubernetes, "cpu_request" | "cpu_limit" | "memory_request" | "memory_limit") => {
                Section::Resources
            }
            (Kubernetes, "node_selector" | "tolerations" | "volume_size" | "storage_class") => {
                Section::Placement
            }
            (Ssh, "host" | "user" | "port" | "key_file") => Section::Connection,
            (Host, _) | (_, _) => return None,
        };
        Self::of(runtime).contains(&section).then_some(section)
    }

    /// The section of a daemon of `runtime` that holds the control `id`. `None` for the name and
    /// for a control the kind does not show.
    pub fn holding(runtime: DraftRuntime, id: FieldId) -> Option<Section> {
        use DraftRuntime::{Container, Host, Kubernetes, Ssh};
        use FieldId as F;
        let section = match (runtime, id) {
            (_, F::DaemonAutoStart) => Section::Connection,
            (Container, F::DaemonContainerName | F::DaemonPort) => Section::Connection,
            (Container, F::DaemonImage | F::DaemonImagePath | F::DaemonAiCli(_)) => {
                Section::Container
            }
            (Container, F::DaemonCredential(_)) => Section::Credentials,
            (
                Container,
                F::DaemonCpuLimit
                | F::DaemonMemoryLimit
                | F::DaemonPidLimit
                | F::DaemonStorageLimit,
            ) => Section::Resources,
            (Container, F::DaemonSurviveLogout) => Section::Sessions,
            (Kubernetes, F::DaemonKubeContext | F::DaemonKubeNamespace) => Section::Connection,
            (Kubernetes, F::DaemonKubeImage | F::DaemonKubeProvider(_)) => Section::Image,
            (Kubernetes, F::DaemonKubeAllowed(_) | F::DaemonKubePullFile) => Section::Credentials,
            (
                Kubernetes,
                F::DaemonCpuRequest
                | F::DaemonCpuLimit
                | F::DaemonMemoryRequest
                | F::DaemonMemoryLimit,
            ) => Section::Resources,
            (Kubernetes, F::DaemonKubeVolumeSize | F::DaemonKubeStorageClass) => Section::Placement,
            (Ssh, F::DaemonSshHost | F::DaemonSshUser | F::DaemonSshPort | F::DaemonSshKeyFile) => {
                Section::Connection
            }
            (Host, _) | (_, _) => return None,
        };
        Self::of(runtime).contains(&section).then_some(section)
    }
}

/// The one-line summary of `section`'s current values for a closed header (FR-014; contract
/// §Section summaries), or `None` when the section is not the kind's, or every part is empty and
/// the row gives no fallback. Recomputed from the draft on every view, so it follows each keystroke.
pub fn summary(draft: &Draft, section: Section) -> Option<String> {
    if !Section::of(draft.runtime).contains(&section) {
        return None;
    }
    let parts: Vec<String> = match draft.runtime {
        DraftRuntime::Container => container_parts(draft, section),
        DraftRuntime::Kubernetes => kube_parts(draft, section),
        DraftRuntime::Ssh => vec![ssh_target(draft), auto_start(draft)],
        DraftRuntime::Host => match section {
            Section::Sessions => vec!["Stops when signed out or idle".into()],
            _ => vec![auto_start(draft)],
        },
    };
    let parts: Vec<String> = parts.into_iter().filter(|p| !p.is_empty()).collect();
    (!parts.is_empty()).then(|| parts.join(SEPARATOR))
}

/// Whether `section` holds a required field that is still empty (FR-014, US3 AS3).
pub fn marked(draft: &Draft, section: Section) -> bool {
    draft.required_fields().into_iter().any(|(id, req)| {
        req.required && req.empty && Section::holding(draft.runtime, id) == Some(section)
    })
}

fn auto_start(draft: &Draft) -> String {
    if draft.auto_start {
        "Connects at launch".into()
    } else {
        "Manual start".into()
    }
}

/// `label value` when `value` has text, else nothing.
fn labelled(label: &str, value: &str, unit: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        String::new()
    } else {
        format!("{label} {value}{unit}")
    }
}

/// The parts that are set, or `fallback` when none is.
fn set_or(parts: Vec<String>, fallback: &str) -> Vec<String> {
    let set: Vec<String> = parts.into_iter().filter(|p| !p.is_empty()).collect();
    if set.is_empty() {
        vec![fallback.into()]
    } else {
        set
    }
}

fn cli_names<'a>(chosen: impl Fn(AiCli) -> bool + 'a) -> String {
    let names: Vec<&str> = AiCli::ALL
        .into_iter()
        .filter(|c| chosen(*c))
        .map(|c| c.provider().display_name())
        .collect();
    if names.is_empty() {
        "No AI tools".into()
    } else {
        names.join(", ")
    }
}

fn container_parts(d: &Draft, section: Section) -> Vec<String> {
    let p = &d.profile;
    match section {
        Section::Connection => vec![
            d.container_name.trim().into(),
            if d.port.trim().is_empty() {
                "Any free port".into()
            } else {
                labelled("Port", &d.port, "")
            },
            auto_start(d),
        ],
        Section::Container => vec![
            match p.runtime {
                RuntimeKind::Docker => "Docker".into(),
                RuntimeKind::Podman => "Podman".into(),
            },
            match ImageKind::of(&p.image) {
                ImageKind::Basic => "Basic image".into(),
                ImageKind::Own => "Own image".into(),
            },
            cli_names(|c| p.ai_clis.0.contains(&c)),
        ],
        Section::Credentials => {
            let shared: Vec<&str> = CredentialShare::ALL
                .into_iter()
                .filter(|s| p.credentials.contains(s))
                .map(|s| s.label())
                .collect();
            vec![if shared.is_empty() {
                "Nothing shared".into()
            } else {
                format!("Shared: {}", shared.join(", "))
            }]
        }
        Section::Network => vec![match p.network {
            NetworkPosture::NoOutbound => "No outbound connections".into(),
            NetworkPosture::Outbound => "Outbound connections allowed".into(),
        }],
        Section::Resources => set_or(
            vec![
                labelled("CPU", &d.cpus, ""),
                labelled("Memory", &d.memory_mib, " MiB"),
                labelled("Processes", &d.pids, ""),
                labelled("Storage", &d.storage_mib, " MiB"),
            ],
            "Runtime defaults",
        ),
        Section::Sessions => vec![if p.survive_logout {
            "Keeps running when signed out".into()
        } else {
            "Stops when signed out or idle".into()
        }],
        Section::Image | Section::Placement => Vec::new(),
    }
}

fn kube_parts(d: &Draft, section: Section) -> Vec<String> {
    let k = &d.kube;
    let any_line = |text: &str| text.lines().any(|l| !l.trim().is_empty());
    match section {
        Section::Connection => vec![
            k.context.trim().into(),
            labelled("Namespace", &k.namespace, ""),
            auto_start(d),
        ],
        Section::Image => vec![
            if k.image.trim().is_empty() {
                "Default SSH image".into()
            } else {
                k.image.trim().into()
            },
            cli_names(|c| k.providers.contains(&c)),
        ],
        Section::Credentials => {
            let allowed: Vec<String> = AllowedKind::all()
                .into_iter()
                .filter(|a| k.allowed.contains(a))
                .map(AllowedKind::short_label)
                .collect();
            vec![if allowed.is_empty() {
                "Nothing allowed".into()
            } else {
                format!("Allowed: {}", allowed.join(", "))
            }]
        }
        Section::Resources => set_or(
            vec![
                labelled("CPU request", &k.cpu_request, ""),
                labelled("CPU limit", &k.cpu_limit, ""),
                labelled("Memory request", &k.memory_request, " MiB"),
                labelled("Memory limit", &k.memory_limit, " MiB"),
            ],
            "No requests or limits",
        ),
        Section::Placement => vec![
            labelled("Volume", &k.volume_size, ""),
            if k.storage_class.trim().is_empty() {
                "Default storage class".into()
            } else {
                labelled("Storage class", &k.storage_class, "")
            },
            if any_line(&k.node_selector) {
                "Node selector".into()
            } else {
                String::new()
            },
            if any_line(&k.tolerations) {
                "Tolerations".into()
            } else {
                String::new()
            },
        ],
        Section::Container | Section::Network | Section::Sessions => Vec::new(),
    }
}

/// `[<user>@]<host>[:<port>]`, empty without a host.
fn ssh_target(d: &Draft) -> String {
    let host = d.ssh_host.trim();
    if host.is_empty() {
        return String::new();
    }
    let (user, port) = (d.ssh_user.trim(), d.ssh_port.trim());
    let user = if user.is_empty() {
        String::new()
    } else {
        format!("{user}@")
    };
    let port = if port.is_empty() {
        String::new()
    } else {
        format!(":{port}")
    };
    format!("{user}{host}{port}")
}

/// The git credential lines of a sandboxed daemon's row, one per project it reported, by the
/// daemon's path for it (#1002, FR-013, FR-008). A project still waiting for its first check has
/// none, so a line appears only once there is something to say, and a delivered credential
/// replaces the line that said there was none.
pub fn credential_lines(
    statuses: &BTreeMap<PathBuf, CredentialStatus>,
    now: UnixTime,
) -> Vec<CredentialLine> {
    statuses
        .iter()
        .filter_map(|(project, status)| {
            let name = project
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| project.display().to_string());
            let (text, warning) = match status {
                CredentialStatus::Pending => return None,
                CredentialStatus::Held { repo, expires_at } => {
                    let date = utc_date(*expires_at);
                    if status.expires_within(now, EXPIRY_WARNING_SECS) {
                        (
                            format!(
                                "{name}: Git credential for {repo} expires on {date}: paste a \
                                 new token in Settings"
                            ),
                            true,
                        )
                    } else {
                        (
                            format!("{name}: Git credential for {repo}, expires on {date}"),
                            false,
                        )
                    }
                }
                CredentialStatus::None(reason) => (
                    format!("{name}: No git credential: {}", reason.message()),
                    true,
                ),
            };
            Some(CredentialLine { text, warning })
        })
        .collect()
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    use micold_core::github::GithubRepo;
    use micold_core::scoped_credential::NoCredentialReason;

    /// 2026-10-19T00:00:00Z.
    const NOW: UnixTime = 1_792_368_000;
    const DAY: UnixTime = 24 * 60 * 60;

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("git@github.com:you/scratch.git").expect("a GitHub remote")
    }

    fn one(status: CredentialStatus) -> Vec<CredentialLine> {
        let statuses = BTreeMap::from([(PathBuf::from("/work/scratch"), status)]);
        credential_lines(&statuses, NOW)
    }

    #[test]
    fn a_held_credential_names_the_project_the_repository_and_the_expiry_date() {
        let lines = one(CredentialStatus::Held {
            repo: repo(),
            expires_at: NOW + 30 * DAY,
        });
        assert_eq!(
            lines,
            vec![CredentialLine {
                text: "scratch: Git credential for you/scratch, expires on 2026-11-18".into(),
                warning: false,
            }]
        );
    }

    #[test]
    fn a_credential_expiring_within_a_day_warns_and_says_what_to_do() {
        let lines = one(CredentialStatus::Held {
            repo: repo(),
            expires_at: NOW + DAY,
        });
        assert_eq!(
            lines,
            vec![CredentialLine {
                text: "scratch: Git credential for you/scratch expires on 2026-10-20: paste a new \
                       token in Settings"
                    .into(),
                warning: true,
            }]
        );
    }

    #[test]
    fn no_credential_gives_the_reason_with_its_remedy() {
        for reason in [
            NoCredentialReason::SourceNotSetUp,
            NoCredentialReason::PatExpired,
            NoCredentialReason::NoRemote,
            NoCredentialReason::SourceUnreachable("timed out".into()),
        ] {
            let lines = one(CredentialStatus::None(reason.clone()));
            assert_eq!(
                lines,
                vec![CredentialLine {
                    text: format!("scratch: No git credential: {}", reason.message()),
                    warning: true,
                }]
            );
        }
    }

    #[test]
    fn a_project_waiting_for_its_first_check_has_no_line() {
        assert!(one(CredentialStatus::Pending).is_empty());
    }

    #[test]
    fn a_delivered_credential_replaces_the_line_that_said_there_was_none() {
        let before = one(CredentialStatus::None(NoCredentialReason::SourceNotSetUp));
        let after = one(CredentialStatus::Held {
            repo: repo(),
            expires_at: NOW + 30 * DAY,
        });
        assert!(before[0].text.contains("No git credential"));
        assert_eq!(after.len(), 1);
        assert!(!after[0].text.contains("No git credential"));
        assert!(!after[0].warning);
    }

    #[test]
    fn one_line_per_project_in_path_order() {
        let statuses = BTreeMap::from([
            (
                PathBuf::from("/work/b"),
                CredentialStatus::None(NoCredentialReason::NoRemote),
            ),
            (PathBuf::from("/work/a"), CredentialStatus::Pending),
            (
                PathBuf::from("/work/c"),
                CredentialStatus::None(NoCredentialReason::PatExpired),
            ),
        ]);
        let texts: Vec<String> = credential_lines(&statuses, NOW)
            .into_iter()
            .map(|l| l.text)
            .collect();
        assert_eq!(texts.len(), 2);
        assert!(texts[0].starts_with("b: "));
        assert!(texts[1].starts_with("c: "));
    }
}
