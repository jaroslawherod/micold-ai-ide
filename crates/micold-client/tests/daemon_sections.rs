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
        Some(Sessions).filter(|_| false)
    );
}
