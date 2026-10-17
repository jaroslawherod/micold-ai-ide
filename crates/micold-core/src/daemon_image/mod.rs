//! The daemon image source and resources, shared by the Kubernetes runtime (686) and the
//! container daemon (#806). Data model "DaemonImageSource" and "DaemonResources".

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::daemons::RegistryError;
use crate::sandbox::image::ImageRef;
use crate::sandbox::{Bytes, MilliCpus, DEFAULT_MEMORY, DEFAULT_MILLI_CPUS};
use crate::session::AiCli;

/// Where a daemon's image comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonImageSource {
    /// Built on this machine from the app's template or a given Containerfile, then pushed.
    Build {
        /// `None` is the app's template.
        dockerfile: Option<PathBuf>,
        /// The AI CLIs installed into the image.
        providers: BTreeSet<AiCli>,
    },
    /// A reference the cluster pulls as given.
    Pull {
        /// The image reference.
        reference: ImageRef,
    },
}

impl DaemonImageSource {
    /// The default source: build from the template with the CLIs the provider list enables.
    pub fn default_for(enabled: &[AiCli]) -> Self {
        DaemonImageSource::Build {
            dockerfile: None,
            providers: enabled.iter().copied().collect(),
        }
    }

    /// Whether a pulled reference can change under a running daemon, as the sandbox flags it.
    pub fn is_moving(&self) -> bool {
        match self {
            DaemonImageSource::Pull { reference } => reference.is_moving(),
            DaemonImageSource::Build { .. } => false,
        }
    }
}

impl Default for DaemonImageSource {
    /// Every CLI the app supports; the form narrows it to the enabled ones.
    fn default() -> Self {
        Self::default_for(&AiCli::ALL)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StoredImageSource {
    Build {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dockerfile: Option<PathBuf>,
        #[serde(default)]
        providers: BTreeSet<AiCli>,
    },
    Pull {
        reference: String,
    },
}

impl Serialize for DaemonImageSource {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            DaemonImageSource::Build {
                dockerfile,
                providers,
            } => StoredImageSource::Build {
                dockerfile: dockerfile.clone(),
                providers: providers.clone(),
            },
            DaemonImageSource::Pull { reference } => StoredImageSource::Pull {
                reference: reference.to_reference(),
            },
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for DaemonImageSource {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match StoredImageSource::deserialize(d)? {
            StoredImageSource::Build {
                dockerfile,
                providers,
            } => DaemonImageSource::Build {
                dockerfile,
                providers,
            },
            StoredImageSource::Pull { reference } => DaemonImageSource::Pull {
                reference: ImageRef::parse(&reference)
                    .map_err(|e| serde::de::Error::custom(e.message()))?,
            },
        })
    }
}

/// CPU and memory requests and limits, each set or unset on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonResources {
    /// `resources.requests.cpu`.
    #[serde(default)]
    pub cpu_request: Option<MilliCpus>,
    /// `resources.limits.cpu`.
    #[serde(default)]
    pub cpu_limit: Option<MilliCpus>,
    /// `resources.requests.memory`.
    #[serde(default)]
    pub memory_request: Option<Bytes>,
    /// `resources.limits.memory`.
    #[serde(default)]
    pub memory_limit: Option<Bytes>,
}

impl Default for DaemonResources {
    fn default() -> Self {
        Self {
            cpu_request: None,
            cpu_limit: Some(DEFAULT_MILLI_CPUS),
            memory_request: None,
            memory_limit: Some(DEFAULT_MEMORY),
        }
    }
}

impl DaemonResources {
    /// Refuse a request above its limit, naming the request field.
    pub fn validate(&self) -> Result<(), RegistryError> {
        fn above<T: PartialOrd>(request: Option<T>, limit: Option<T>) -> bool {
            matches!((request, limit), (Some(r), Some(l)) if r > l)
        }
        if above(self.cpu_request.map(|c| c.0), self.cpu_limit.map(|c| c.0)) {
            return Err(RegistryError::InvalidField {
                field: "cpu_request",
            });
        }
        if above(
            self.memory_request.map(|m| m.0),
            self.memory_limit.map(|m| m.0),
        ) {
            return Err(RegistryError::InvalidField {
                field: "memory_request",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_source_builds_the_template_with_the_enabled_clis() {
        let enabled = [AiCli::ClaudeCode, AiCli::Pi];
        assert_eq!(
            DaemonImageSource::default_for(&enabled),
            DaemonImageSource::Build {
                dockerfile: None,
                providers: enabled.into_iter().collect(),
            },
            "Build from the app's template with the Settings provider list"
        );
    }

    #[test]
    fn a_pulled_moving_tag_is_flagged_as_the_sandbox_flags_it() {
        for (raw, moving) in [
            ("ghcr.io/acme/micold-daemon:latest", true),
            ("ghcr.io/acme/micold-daemon", true),
            ("ghcr.io/acme/micold-daemon:0.30.0", false),
        ] {
            let reference = ImageRef::parse(raw).unwrap();
            let expected = reference.is_moving();
            let source = DaemonImageSource::Pull { reference };
            assert_eq!(
                source.is_moving(),
                expected,
                "{raw}: same rule as sandbox::image"
            );
            assert_eq!(source.is_moving(), moving, "{raw}");
        }
    }

    #[test]
    fn resources_default_to_no_requests_and_the_sandbox_limits() {
        let r = DaemonResources::default();
        assert_eq!(r.cpu_request, None);
        assert_eq!(r.memory_request, None);
        assert_eq!(r.cpu_limit, Some(DEFAULT_MILLI_CPUS));
        assert_eq!(r.memory_limit, Some(DEFAULT_MEMORY));
    }

    #[test]
    fn each_of_the_four_can_be_set_alone() {
        let none = DaemonResources {
            cpu_request: None,
            cpu_limit: None,
            memory_request: None,
            memory_limit: None,
        };
        let alone = [
            DaemonResources {
                cpu_request: Some(MilliCpus(500)),
                ..none
            },
            DaemonResources {
                cpu_limit: Some(MilliCpus(500)),
                ..none
            },
            DaemonResources {
                memory_request: Some(Bytes::from_mib(256)),
                ..none
            },
            DaemonResources {
                memory_limit: Some(Bytes::from_mib(256)),
                ..none
            },
            none,
        ];
        for r in alone {
            assert_eq!(r.validate(), Ok(()), "{r:?} is valid on its own");
        }
    }

    #[test]
    fn a_request_above_its_limit_is_refused_naming_the_request() {
        let base = DaemonResources::default();
        let at = DaemonResources {
            cpu_request: Some(MilliCpus(2000)),
            cpu_limit: Some(MilliCpus(2000)),
            ..base
        };
        assert_eq!(
            at.validate(),
            Ok(()),
            "a request equal to its limit is accepted"
        );
        let cpu = DaemonResources {
            cpu_request: Some(MilliCpus(2001)),
            cpu_limit: Some(MilliCpus(2000)),
            ..base
        };
        assert_eq!(
            cpu.validate(),
            Err(RegistryError::InvalidField {
                field: "cpu_request"
            })
        );
        let mem = DaemonResources {
            memory_request: Some(Bytes::from_mib(5000)),
            memory_limit: Some(Bytes::from_mib(4096)),
            ..base
        };
        assert_eq!(
            mem.validate(),
            Err(RegistryError::InvalidField {
                field: "memory_request"
            })
        );
    }
}
