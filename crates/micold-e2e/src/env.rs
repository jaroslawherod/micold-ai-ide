//! Whether a suite runs (FR-010): only under the harness, which sets `MICOLD_E2E=1`.
//!
//! Unset, a suite test skips and prints the reason, so `cargo test --workspace` needs no cluster.
//! `MICOLD_E2E_REQUIRED=1` (the harness sets it too) turns that skip into a failure, so a run that
//! should have reached the cluster cannot pass by skipping.

use crate::cluster::Cluster;

/// The namespace and kube context the suites use when the harness does not name them.
pub const DEFAULT_NAMESPACE: &str = "micold-e2e";
pub const DEFAULT_CONTEXT: &str = "kind-micold-e2e";

/// What a suite test does, decided from the environment.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Run against this cluster.
    Run(Cluster),
    /// Skip, printing the reason.
    Skip(String),
    /// Fail with the reason: a skip under `MICOLD_E2E_REQUIRED=1`.
    Fail(String),
}

/// Decides from `var`, a lookup of environment variables (injected so the tests never touch the
/// process environment).
pub fn decide(var: impl Fn(&str) -> Option<String>) -> Decision {
    let required = var("MICOLD_E2E_REQUIRED").as_deref() == Some("1");
    match var("MICOLD_E2E").as_deref() {
        Some("1") => Decision::Run(Cluster {
            namespace: var("MICOLD_E2E_NAMESPACE").unwrap_or_else(|| DEFAULT_NAMESPACE.into()),
            context: var("MICOLD_E2E_CONTEXT").unwrap_or_else(|| DEFAULT_CONTEXT.into()),
        }),
        other => {
            let reason = match other {
                None => "MICOLD_E2E is not set: kind e2e suites run under scripts/e2e-kind.sh (mise run e2e-kind)".to_string(),
                Some(v) => format!("MICOLD_E2E is '{v}', not '1': kind e2e suites run under scripts/e2e-kind.sh (mise run e2e-kind)"),
            };
            if required {
                Decision::Fail(format!(
                    "MICOLD_E2E_REQUIRED=1 but the suite would skip: {reason}"
                ))
            } else {
                Decision::Skip(reason)
            }
        }
    }
}

/// The gate every suite test starts with: the cluster to test, or `None` after printing the skip
/// reason. Panics with the reason under `MICOLD_E2E_REQUIRED=1`.
pub fn cluster_or_skip(test: &str) -> Option<Cluster> {
    match decide(|k| std::env::var(k).ok()) {
        Decision::Run(cluster) => Some(cluster),
        Decision::Skip(reason) => {
            eprintln!("skipped {test}: {reason}");
            None
        }
        Decision::Fail(reason) => panic!("{test}: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn with(pairs: &[(&str, &str)]) -> Decision {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        decide(|k| map.get(k).cloned())
    }

    #[test]
    fn unset_skips_naming_the_variable_and_the_harness() {
        match with(&[]) {
            Decision::Skip(reason) => {
                assert!(reason.contains("MICOLD_E2E"), "{reason}");
                assert!(reason.contains("e2e-kind"), "{reason}");
            }
            other => panic!("want a skip, got {other:?}"),
        }
    }

    #[test]
    fn a_value_other_than_one_skips_and_names_it() {
        match with(&[("MICOLD_E2E", "yes")]) {
            Decision::Skip(reason) => assert!(reason.contains("'yes'"), "{reason}"),
            other => panic!("want a skip, got {other:?}"),
        }
    }

    #[test]
    fn required_turns_the_skip_into_a_failure() {
        match with(&[("MICOLD_E2E_REQUIRED", "1")]) {
            Decision::Fail(reason) => assert!(reason.contains("MICOLD_E2E_REQUIRED"), "{reason}"),
            other => panic!("want a failure, got {other:?}"),
        }
    }

    #[test]
    fn set_runs_with_the_default_namespace_and_context() {
        assert_eq!(
            with(&[("MICOLD_E2E", "1"), ("MICOLD_E2E_REQUIRED", "1")]),
            Decision::Run(Cluster {
                namespace: DEFAULT_NAMESPACE.into(),
                context: DEFAULT_CONTEXT.into()
            })
        );
    }

    #[test]
    fn set_runs_with_the_harness_namespace_and_context() {
        assert_eq!(
            with(&[
                ("MICOLD_E2E", "1"),
                ("MICOLD_E2E_NAMESPACE", "ns-x"),
                ("MICOLD_E2E_CONTEXT", "kind-x")
            ]),
            Decision::Run(Cluster {
                namespace: "ns-x".into(),
                context: "kind-x".into()
            })
        );
    }
}
