//! Which section of a daemon form holds which field, per kind (feature 890, FR-002, FR-003): the
//! render-free table the form and the open-state rule are both built from.

use micold_client::features::daemons::{DraftRuntime, Section};
use micold_client::features::window::FieldId;
use micold_core::sandbox::CredentialShare;
use micold_core::session::AiCli;

use DraftRuntime::{Container, Host, Kubernetes, Ssh};
use Section::{Connection, Credentials, Image, Network, Placement, Resources, Sessions};

#[test]
fn each_kind_lists_its_sections_in_order() {
    assert_eq!(
        Section::of(Container),
        [
            Connection,
            Section::Container,
            Credentials,
            Network,
            Resources,
            Sessions
        ]
    );
    assert_eq!(
        Section::of(Kubernetes),
        [Connection, Image, Credentials, Resources, Placement]
    );
    assert_eq!(Section::of(Ssh), [Connection]);
    assert_eq!(Section::of(Host), [Connection, Sessions]);
}

#[test]
fn one_concept_has_one_title_and_limits_is_gone() {
    let titles: Vec<_> = [
        Connection,
        Section::Container,
        Image,
        Credentials,
        Network,
        Resources,
        Placement,
        Sessions,
    ]
    .iter()
    .map(|s| s.title())
    .collect();
    assert_eq!(
        titles,
        [
            "Connection",
            "Container",
            "Image",
            "Credentials",
            "Network",
            "Resources",
            "Placement and storage",
            "Sessions"
        ]
    );
}

#[test]
fn every_refusable_field_has_its_section() {
    let table: &[(DraftRuntime, &[&str], Section)] = &[
        (Container, &["container_name", "port"], Connection),
        (Container, &["image", "image_path"], Section::Container),
        (Container, &["cpus", "memory", "pids", "storage"], Resources),
        (Kubernetes, &["context", "namespace"], Connection),
        (Kubernetes, &["image"], Image),
        (Kubernetes, &["ssh_key_file", "pull_file"], Credentials),
        (
            Kubernetes,
            &["cpu_request", "cpu_limit", "memory_request", "memory_limit"],
            Resources,
        ),
        (
            Kubernetes,
            &[
                "node_selector",
                "tolerations",
                "volume_size",
                "storage_class",
            ],
            Placement,
        ),
        (Ssh, &["host", "user", "port", "key_file"], Connection),
    ];
    for (runtime, fields, section) in table {
        for field in *fields {
            assert_eq!(
                Section::holding_error(*runtime, field),
                Some(*section),
                "{runtime:?} {field}"
            );
        }
    }
    for runtime in [Container, Kubernetes, Ssh, Host] {
        for outside in ["name", "runtime"] {
            assert_eq!(Section::holding_error(runtime, outside), None);
        }
    }
    // A field the kind does not show is in none of its sections.
    assert_eq!(Section::holding_error(Ssh, "cpus"), None);
    assert_eq!(Section::holding_error(Host, "port"), None);
}

#[test]
fn every_required_or_marked_control_has_its_section() {
    use FieldId as F;
    let share = CredentialShare::ALL[0];
    let cli = AiCli::ALL[0];
    let table: &[(DraftRuntime, FieldId, Section)] = &[
        (Container, F::DaemonContainerName, Connection),
        (Container, F::DaemonPort, Connection),
        (Container, F::DaemonAutoStart, Connection),
        (Container, F::DaemonImage, Section::Container),
        (Container, F::DaemonImagePath, Section::Container),
        (Container, F::DaemonAiCli(cli), Section::Container),
        (Container, F::DaemonCredential(share), Credentials),
        (Container, F::DaemonCpuLimit, Resources),
        (Container, F::DaemonMemoryLimit, Resources),
        (Container, F::DaemonPidLimit, Resources),
        (Container, F::DaemonStorageLimit, Resources),
        (Container, F::DaemonSurviveLogout, Sessions),
        (Kubernetes, F::DaemonKubeContext, Connection),
        (Kubernetes, F::DaemonKubeNamespace, Connection),
        (Kubernetes, F::DaemonAutoStart, Connection),
        (Kubernetes, F::DaemonKubeImage, Image),
        (Kubernetes, F::DaemonKubeProvider(cli), Image),
        (Kubernetes, F::DaemonKubeAllowed(0), Credentials),
        (Kubernetes, F::DaemonKubeSshKeyFile, Credentials),
        (Kubernetes, F::DaemonKubePullFile, Credentials),
        (Kubernetes, F::DaemonCpuRequest, Resources),
        (Kubernetes, F::DaemonCpuLimit, Resources),
        (Kubernetes, F::DaemonMemoryRequest, Resources),
        (Kubernetes, F::DaemonMemoryLimit, Resources),
        (Kubernetes, F::DaemonKubeVolumeSize, Placement),
        (Kubernetes, F::DaemonKubeStorageClass, Placement),
        (Ssh, F::DaemonSshHost, Connection),
        (Ssh, F::DaemonSshUser, Connection),
        (Ssh, F::DaemonSshPort, Connection),
        (Ssh, F::DaemonSshKeyFile, Connection),
        (Ssh, F::DaemonAutoStart, Connection),
        (Host, F::DaemonAutoStart, Connection),
    ];
    for (runtime, id, section) in table {
        assert_eq!(
            Section::holding(*runtime, *id),
            Some(*section),
            "{runtime:?} {id:?}"
        );
    }
    for runtime in [Container, Kubernetes, Ssh, Host] {
        assert_eq!(Section::holding(runtime, F::DaemonName), None);
    }
    // A section a kind lacks is never returned for it.
    assert_eq!(Section::holding(Ssh, F::DaemonCpuLimit), None);
    assert_eq!(
        Section::holding(Host, F::DaemonSurviveLogout),
        None,
        "a host daemon has no survive-logout control, though it has a Sessions section"
    );
}

// ---- Feature 890, M3: section summaries and the required marker (FR-014, contract §Section
// summaries). ----

mod summaries {
    use micold_client::features::daemon_sections::{marked, summary};
    use micold_client::features::daemons::{
        begin_add, AllowedKind, Draft, DraftRuntime, ImageKind, Section, State,
    };
    use micold_core::sandbox::{
        image::{ImageChoice, OwnImage},
        AiCliSelection, CredentialShare, NetworkPosture,
    };
    use micold_core::session::AiCli;

    /// A fresh add form of `runtime`, nothing typed.
    fn fresh(runtime: DraftRuntime) -> Draft {
        let mut state = State::default();
        begin_add(&mut state);
        let mut d = state.draft.expect("the form opens");
        d.runtime = runtime;
        d.auto_start = false;
        d
    }

    fn said(d: &Draft, s: Section) -> Option<String> {
        summary(d, s)
    }

    #[test]
    fn container_connection_joins_name_port_and_start() {
        let mut d = fresh(DraftRuntime::Container);
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("Any free port · Manual start")
        );
        d.container_name = "  micold-dev ".into();
        d.port = "7050".into();
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("micold-dev · Port 7050 · Manual start")
        );
        d.auto_start = true;
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("micold-dev · Port 7050 · Connects at launch")
        );
    }

    #[test]
    fn container_container_names_runtime_image_and_tools() {
        let mut d = fresh(DraftRuntime::Container);
        d.profile.ai_clis = AiCliSelection(Default::default());
        assert_eq!(
            said(&d, Section::Container).as_deref(),
            Some("Docker · Basic image · No AI tools")
        );
        d.profile.ai_clis = AiCliSelection([AiCli::ClaudeCode].into_iter().collect());
        d.profile.runtime = micold_core::sandbox::runtime::RuntimeKind::Podman;
        d.profile.image = ImageChoice::Own(OwnImage::Reference("x".into()));
        let text = said(&d, Section::Container).unwrap();
        assert_eq!(
            text,
            format!(
                "Podman · Own image · {}",
                AiCli::ClaudeCode.provider().display_name()
            )
        );
        assert_eq!(ImageKind::of(&d.profile.image), ImageKind::Own);
    }

    #[test]
    fn container_credentials_network_resources_sessions() {
        let mut d = fresh(DraftRuntime::Container);
        d.profile.credentials.clear();
        assert_eq!(
            said(&d, Section::Credentials).as_deref(),
            Some("Nothing shared")
        );
        d.profile.credentials.insert(CredentialShare::SshAgent);
        d.profile.credentials.insert(CredentialShare::GitConfig);
        assert_eq!(
            said(&d, Section::Credentials).as_deref(),
            Some("Shared: Git configuration, SSH agent")
        );
        d.profile.network = NetworkPosture::NoOutbound;
        assert_eq!(
            said(&d, Section::Network).as_deref(),
            Some("No outbound connections")
        );
        d.profile.network = NetworkPosture::Outbound;
        assert_eq!(
            said(&d, Section::Network).as_deref(),
            Some("Outbound connections allowed")
        );
        d.cpus = String::new();
        d.memory_mib = String::new();
        d.pids = String::new();
        d.storage_mib = String::new();
        assert_eq!(
            said(&d, Section::Resources).as_deref(),
            Some("Runtime defaults")
        );
        d.cpus = " 2 ".into();
        d.memory_mib = "4096".into();
        assert_eq!(
            said(&d, Section::Resources).as_deref(),
            Some("CPU 2 · Memory 4096 MiB")
        );
        d.pids = "100".into();
        d.storage_mib = "512".into();
        assert_eq!(
            said(&d, Section::Resources).as_deref(),
            Some("CPU 2 · Memory 4096 MiB · Processes 100 · Storage 512 MiB")
        );
        d.profile.survive_logout = false;
        assert_eq!(
            said(&d, Section::Sessions).as_deref(),
            Some("Stops when signed out or idle")
        );
        d.profile.survive_logout = true;
        assert_eq!(
            said(&d, Section::Sessions).as_deref(),
            Some("Keeps running when signed out")
        );
    }

    #[test]
    fn kubernetes_sections() {
        let mut d = fresh(DraftRuntime::Kubernetes);
        d.kube.context = String::new();
        d.kube.namespace = String::new();
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("Manual start")
        );
        d.kube.context = "dev-cluster".into();
        d.kube.namespace = " team-a".into();
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("dev-cluster · Namespace team-a · Manual start")
        );
        d.kube.image = String::new();
        d.kube.providers = [AiCli::ClaudeCode].into_iter().collect();
        assert_eq!(
            said(&d, Section::Image).as_deref(),
            Some(&*format!(
                "Default SSH image · {}",
                AiCli::ClaudeCode.provider().display_name()
            ))
        );
        d.kube.allowed.clear();
        assert_eq!(
            said(&d, Section::Credentials).as_deref(),
            Some("Nothing allowed")
        );
        d.kube.allowed.insert(AllowedKind::GitSshKey);
        d.kube.allowed.insert(AllowedKind::GitConfig);
        assert_eq!(
            said(&d, Section::Credentials).as_deref(),
            Some("Allowed: Git config, SSH key")
        );
        d.kube.cpu_request = String::new();
        d.kube.cpu_limit = String::new();
        d.kube.memory_request = String::new();
        d.kube.memory_limit = String::new();
        assert_eq!(
            said(&d, Section::Resources).as_deref(),
            Some("No requests or limits")
        );
        d.kube.cpu_request = "0.5".into();
        d.kube.cpu_limit = "2".into();
        d.kube.memory_request = "512".into();
        d.kube.memory_limit = "1024".into();
        assert_eq!(
            said(&d, Section::Resources).as_deref(),
            Some("CPU request 0.5 · CPU limit 2 · Memory request 512 MiB · Memory limit 1024 MiB")
        );
        d.kube.volume_size = "20Gi".into();
        d.kube.storage_class = String::new();
        d.kube.node_selector = "  \n".into();
        d.kube.tolerations = String::new();
        assert_eq!(
            said(&d, Section::Placement).as_deref(),
            Some("Volume 20Gi · Default storage class")
        );
        d.kube.storage_class = "fast".into();
        d.kube.node_selector = "disk=ssd".into();
        d.kube.tolerations = "\ngpu".into();
        assert_eq!(
            said(&d, Section::Placement).as_deref(),
            Some("Volume 20Gi · Storage class fast · Node selector · Tolerations")
        );
        d.kube.volume_size = String::new();
        d.kube.storage_class = String::new();
        d.kube.node_selector = String::new();
        d.kube.tolerations = String::new();
        assert_eq!(
            said(&d, Section::Placement).as_deref(),
            Some("Default storage class")
        );
    }

    #[test]
    fn ssh_and_host() {
        let mut d = fresh(DraftRuntime::Ssh);
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("Manual start")
        );
        d.ssh_user = "dev".into();
        d.ssh_port = "2222".into();
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("Manual start"),
            "user and port show only with a host"
        );
        d.ssh_host = "build-box".into();
        d.auto_start = true;
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("dev@build-box:2222 · Connects at launch")
        );
        d.ssh_user = String::new();
        d.ssh_port = String::new();
        d.auto_start = false;
        assert_eq!(
            said(&d, Section::Connection).as_deref(),
            Some("build-box · Manual start")
        );
        let h = fresh(DraftRuntime::Host);
        assert_eq!(
            said(&h, Section::Connection).as_deref(),
            Some("Manual start")
        );
        assert_eq!(
            said(&h, Section::Sessions).as_deref(),
            Some("Stops when signed out or idle")
        );
    }

    #[test]
    fn a_section_the_kind_lacks_has_no_summary() {
        assert_eq!(said(&fresh(DraftRuntime::Ssh), Section::Network), None);
        assert_eq!(said(&fresh(DraftRuntime::Host), Section::Resources), None);
    }

    #[test]
    fn the_marker_follows_the_required_fields() {
        let mut c = fresh(DraftRuntime::Container);
        c.container_name = String::new();
        assert!(marked(&c, Section::Connection));
        c.container_name = "box".into();
        assert!(!marked(&c, Section::Connection));
        c.profile.image = ImageChoice::Own(OwnImage::Reference(String::new()));
        c.image_reference = String::new();
        c.image_path = String::new();
        assert!(marked(&c, Section::Container));
        c.image_reference = "img:1".into();
        assert!(!marked(&c, Section::Container));
        assert!(!marked(&c, Section::Network));

        let mut k = fresh(DraftRuntime::Kubernetes);
        k.kube.context = String::new();
        k.kube.namespace = "ns".into();
        assert!(marked(&k, Section::Connection));
        k.kube.context = "ctx".into();
        assert!(!marked(&k, Section::Connection));
        k.kube.namespace = " ".into();
        assert!(marked(&k, Section::Connection));

        let mut s = fresh(DraftRuntime::Ssh);
        assert!(marked(&s, Section::Connection));
        s.ssh_host = "h".into();
        assert!(!marked(&s, Section::Connection));
        assert!(!marked(&fresh(DraftRuntime::Host), Section::Connection));
    }

    #[test]
    fn allowed_kinds_have_short_labels() {
        assert_eq!(AllowedKind::GitConfig.short_label(), "Git config");
        assert_eq!(AllowedKind::GitCredentials.short_label(), "Git credentials");
        assert_eq!(AllowedKind::GitSshKey.short_label(), "SSH key");
        assert_eq!(
            AllowedKind::SignIn(AiCli::ClaudeCode).short_label(),
            format!("{} sign-in", AiCli::ClaudeCode.provider().display_name())
        );
        assert_eq!(
            AllowedKind::RegistryPull.short_label(),
            "Registry credentials"
        );
    }
}
