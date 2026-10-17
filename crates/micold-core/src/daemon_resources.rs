//! A daemon's CPU and memory requests and limits, shared by the Kubernetes runtime (686) and the
//! container daemon (#806). Data model "DaemonResources".

use serde::{Deserialize, Serialize};

use crate::daemons::RegistryError;
use crate::sandbox::{Bytes, MilliCpus, DEFAULT_MEMORY, DEFAULT_MILLI_CPUS};

/// CPU and memory requests and limits, each set or unset on its own. A field missing from the
/// stored settings takes its [`Default`] value; a stored `null` is unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonResources {
    /// `resources.requests.cpu`.
    pub cpu_request: Option<MilliCpus>,
    /// `resources.limits.cpu`.
    pub cpu_limit: Option<MilliCpus>,
    /// `resources.requests.memory`.
    pub memory_request: Option<Bytes>,
    /// `resources.limits.memory`.
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
    fn resources_default_to_no_requests_and_the_sandbox_limits() {
        let r = DaemonResources::default();
        assert_eq!(r.cpu_request, None);
        assert_eq!(r.memory_request, None);
        assert_eq!(r.cpu_limit, Some(DEFAULT_MILLI_CPUS));
        assert_eq!(r.memory_limit, Some(DEFAULT_MEMORY));
    }

    #[test]
    fn a_missing_field_takes_its_default_and_a_null_one_is_unset() {
        let empty: DaemonResources = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, DaemonResources::default());
        let no_cpu_limit: DaemonResources = serde_json::from_str(r#"{"cpu_limit":null}"#).unwrap();
        assert_eq!(no_cpu_limit.cpu_limit, None);
        assert_eq!(no_cpu_limit.memory_limit, Some(DEFAULT_MEMORY));
        let round_trip: DaemonResources =
            serde_json::from_str(&serde_json::to_string(&no_cpu_limit).unwrap()).unwrap();
        assert_eq!(round_trip, no_cpu_limit);
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
