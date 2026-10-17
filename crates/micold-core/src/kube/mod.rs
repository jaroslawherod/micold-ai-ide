//! The Kubernetes daemon runtime (feature 686): pure rules and the `Cluster` port. No `kube` crate
//! here; the live adapter is `micold-client::kube::live`.

pub mod classify;
pub mod credentials;
pub mod names;
pub mod settings;
pub mod start;
