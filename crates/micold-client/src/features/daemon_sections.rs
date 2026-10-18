//! The titled sections a daemon's settings are grouped in, per kind of daemon (feature 890,
//! FR-001 to FR-004). View state only: which are open is never saved. Render-free, so the table of
//! which section holds which field is tested without a window.

use super::daemons::DraftRuntime;
use super::window::FieldId;

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
            (Kubernetes, "allowed" | "ssh_key_file" | "pull_file") => Section::Credentials,
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
            (
                Kubernetes,
                F::DaemonKubeAllowed(_) | F::DaemonKubeSshKeyFile | F::DaemonKubePullFile,
            ) => Section::Credentials,
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
