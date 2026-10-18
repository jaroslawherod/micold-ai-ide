//! The Kubernetes part of the Daemons form (feature 686, M4a: FR-001, FR-002, FR-004, FR-005,
//! FR-018): the draft's typed fields, the allowed-credentials list and the kubeconfig context list.
//!
//! The reducer in [`super::daemons`] owns the messages; this holds what the fields mean and how
//! they become [`KubeSettings`]. `base` keeps the settings the form does not edit yet (node
//! selector, tolerations, volume size, storage class) so a save never drops them.

use std::collections::BTreeSet;
use std::path::PathBuf;

use micold_core::daemon_resources::DaemonResources;
use micold_core::kube::credentials::KubeCredential;
use micold_core::kube::settings::{KubeContextName, KubeSettings, Namespace};
use micold_core::sandbox::image::ImageRef;
use micold_core::session::AiCli;

use super::daemons::{invalid_field, FormError};
use super::settings::{cores_text, parse_cores, parse_mib, LimitField, MAX_MIB};
use crate::kube::live::ContextEntry;

/// What the volume size field says while the daemon's volume may already exist (FR-005).
pub const VOLUME_NOTICE: &str = "Applies only to a new volume, after Remove";

/// The kubeconfig contexts the form offers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ContextList {
    /// Not asked for yet.
    #[default]
    NotLoaded,
    /// The merged kubeconfig's contexts.
    Found(Vec<ContextEntry>),
    /// Why there are none, as the form shows it (`no kubeconfig found; looked in <paths>`).
    Failed(String),
}

/// One line of the allowed-credentials list (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllowedKind {
    /// `~/.gitconfig`.
    GitConfig,
    /// `~/.git-credentials`.
    GitCredentials,
    /// One SSH private key; needs a file.
    GitSshKey,
    /// One AI CLI's sign-in.
    SignIn(AiCli),
    /// A registry `auths` file; needs a file.
    RegistryPull,
}

impl AllowedKind {
    /// Every line, in the order the form lists them.
    pub fn all() -> Vec<AllowedKind> {
        let mut all = vec![Self::GitConfig, Self::GitCredentials, Self::GitSshKey];
        all.extend(AiCli::ALL.iter().map(|cli| Self::SignIn(*cli)));
        all.push(Self::RegistryPull);
        all
    }

    /// The checkbox label.
    pub fn label(self) -> String {
        match self {
            Self::GitConfig => KubeCredential::GitConfig.label(),
            Self::GitCredentials => KubeCredential::GitCredentials.label(),
            Self::GitSshKey => "an SSH key".into(),
            Self::SignIn(cli) => KubeCredential::SignIn { cli }.label(),
            Self::RegistryPull => KubeCredential::RegistryPull {
                auths: PathBuf::new(),
            }
            .label(),
        }
    }
}

/// The Kubernetes fields of the form, as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KubeDraft {
    base: KubeSettings,
    /// The kube context.
    pub context: String,
    /// The namespace.
    pub namespace: String,
    /// The image reference; empty is the default SSH image.
    pub image: String,
    /// The AI CLIs to install.
    pub providers: BTreeSet<AiCli>,
    /// CPU request in cores; empty is unset.
    pub cpu_request: String,
    /// CPU limit in cores; empty is unset.
    pub cpu_limit: String,
    /// Memory request in MiB; empty is unset.
    pub memory_request: String,
    /// Memory limit in MiB; empty is unset.
    pub memory_limit: String,
    /// The credentials ticked.
    pub allowed: BTreeSet<AllowedKind>,
    /// The SSH key file, while the SSH key is ticked.
    pub ssh_key_file: String,
    /// The registry auth file, while registry pull is ticked.
    pub pull_file: String,
}

impl KubeDraft {
    /// A new daemon's fields: the defaults, installing the `enabled` AI CLIs.
    pub fn new(enabled: &[AiCli]) -> Self {
        Self::of(&KubeSettings::new("", "", enabled))
    }

    /// What `settings` shows.
    pub fn of(settings: &KubeSettings) -> Self {
        let r = &settings.resources;
        let mib = |b: Option<micold_core::sandbox::Bytes>| {
            b.map(|b| b.as_mib().to_string()).unwrap_or_default()
        };
        let mut draft = Self {
            base: settings.clone(),
            context: settings.context.0.clone(),
            namespace: settings.namespace.0.clone(),
            image: settings
                .image
                .as_ref()
                .map(ImageRef::to_reference)
                .unwrap_or_default(),
            providers: settings.providers.clone(),
            cpu_request: r.cpu_request.map(cores_text).unwrap_or_default(),
            cpu_limit: r.cpu_limit.map(cores_text).unwrap_or_default(),
            memory_request: mib(r.memory_request),
            memory_limit: mib(r.memory_limit),
            allowed: BTreeSet::new(),
            ssh_key_file: String::new(),
            pull_file: String::new(),
        };
        for credential in &settings.allowed {
            match credential {
                KubeCredential::GitConfig => draft.allowed.insert(AllowedKind::GitConfig),
                KubeCredential::GitCredentials => draft.allowed.insert(AllowedKind::GitCredentials),
                KubeCredential::GitSshKey { key } => {
                    draft.ssh_key_file = key.display().to_string();
                    draft.allowed.insert(AllowedKind::GitSshKey)
                }
                KubeCredential::SignIn { cli } => draft.allowed.insert(AllowedKind::SignIn(*cli)),
                KubeCredential::RegistryPull { auths } => {
                    draft.pull_file = auths.display().to_string();
                    draft.allowed.insert(AllowedKind::RegistryPull)
                }
            };
        }
        draft
    }

    /// The stored volume size as the form's note shows it.
    pub fn volume_size_text(&self) -> String {
        self.base.volume_size.to_k8s()
    }

    /// The kubeconfig context `name` was chosen: it fills an empty namespace from the context's
    /// own default.
    pub fn choose_context(&mut self, name: &str, contexts: &ContextList) {
        self.context = name.to_string();
        if self.namespace.trim().is_empty() {
            if let ContextList::Found(list) = contexts {
                if let Some(ns) = list
                    .iter()
                    .find(|c| c.name == name)
                    .and_then(|c| c.namespace.clone())
                {
                    self.namespace = ns;
                }
            }
        }
    }

    /// The four limits as typed, in the order the form lists them (request, limit; CPU, memory).
    pub fn resource_texts(&self) -> [&str; 4] {
        [
            &self.cpu_request,
            &self.cpu_limit,
            &self.memory_request,
            &self.memory_limit,
        ]
    }

    /// The settings the fields describe, or the first refused field.
    pub fn settings(&self) -> Result<KubeSettings, FormError> {
        let mut s = self.base.clone();
        s.context = KubeContextName(self.context.trim().to_string());
        s.namespace = Namespace(self.namespace.trim().to_string());
        s.image = match self.image.trim() {
            "" => None,
            text => Some(ImageRef::parse(text).map_err(|_| invalid_field("image"))?),
        };
        s.providers = self.providers.clone();
        s.resources = self.resources()?;
        s.allowed = self.credentials()?;
        s.validate().map_err(|e| match e {
            micold_core::daemons::RegistryError::InvalidField { field } => invalid_field(field),
            _ => invalid_field("context"),
        })?;
        Ok(s)
    }

    fn resources(&self) -> Result<DaemonResources, FormError> {
        let cores = |text: &str, field: &'static str| {
            parse_cores(text).map_err(|(_, message)| FormError { field, message })
        };
        let memory = |text: &str, field: &'static str| {
            parse_mib(
                text,
                LimitField::Memory,
                micold_core::sandbox::MIN_MEMORY,
                MAX_MIB,
            )
            .map_err(|(_, message)| FormError { field, message })
        };
        Ok(DaemonResources {
            cpu_request: cores(&self.cpu_request, "cpu_request")?,
            cpu_limit: cores(&self.cpu_limit, "cpu_limit")?,
            memory_request: memory(&self.memory_request, "memory_request")?,
            memory_limit: memory(&self.memory_limit, "memory_limit")?,
        })
    }

    fn credentials(&self) -> Result<BTreeSet<KubeCredential>, FormError> {
        let file = |ticked: AllowedKind, text: &str, field: &'static str| {
            if !self.allowed.contains(&ticked) {
                return Ok(None);
            }
            match text.trim() {
                "" => Err(invalid_field(field)),
                path => Ok(Some(PathBuf::from(path))),
            }
        };
        let mut out = BTreeSet::new();
        for kind in &self.allowed {
            match kind {
                AllowedKind::GitConfig => out.insert(KubeCredential::GitConfig),
                AllowedKind::GitCredentials => out.insert(KubeCredential::GitCredentials),
                AllowedKind::SignIn(cli) => out.insert(KubeCredential::SignIn { cli: *cli }),
                AllowedKind::GitSshKey | AllowedKind::RegistryPull => true,
            };
        }
        if let Some(key) = file(AllowedKind::GitSshKey, &self.ssh_key_file, "ssh_key_file")? {
            out.insert(KubeCredential::GitSshKey { key });
        }
        if let Some(auths) = file(AllowedKind::RegistryPull, &self.pull_file, "pull_file")? {
            out.insert(KubeCredential::RegistryPull { auths });
        }
        Ok(out)
    }
}
