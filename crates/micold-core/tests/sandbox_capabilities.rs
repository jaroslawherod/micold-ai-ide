//! What the runtime can enforce, asked once — T079, rules RC-1 … RC-3.
//!
//! Two properties, cheap to state and easy to lose.
//!
//! The probe costs a `docker info` on every call it is not cached for, and it is consulted by the
//! settings view, the argv builder and the reconciliation report — three call sites, one of them a
//! view function that runs on every frame. So it is cached, and cached against the thing that can
//! invalidate it: the runtime's own version. The user who upgrades Docker under a running
//! application must not have to restart it to be told the new answer (RC-1).
//!
//! And [`reconcile`] is the single fact both the view and the argv builder read (RC-2), so it must
//! not *edit* anything. A reconciliation that cleared the limits it could not honour would silently
//! erase the user's stored intent the first time they opened the app on a machine that could not
//! enforce it — and moving back would not bring it back (RC-3).

use micold_core::sandbox::image::ImageSource;
use micold_core::sandbox::parse::{ContainerFacts, ImageFacts};
use micold_core::sandbox::runtime::{
    reconcile, CapabilityCache, ContainerId, ContainerRuntime, IdentityMapping, LimitSupport,
    Progress, RuntimeCapabilities, RuntimeError, RuntimeKind, RuntimeVersion,
};
use micold_core::sandbox::{Bytes, MilliCpus, ResourceBudget, SandboxProfile, SandboxSpec};

use std::cell::Cell;

/// A runtime that answers `detect` from whatever version it currently reports, counts what it was
/// asked, and refuses everything else.
///
/// Counting is the whole point: "cached" is not observable in the value returned — the same
/// capabilities come back either way — only in whether the runtime was consulted to produce them.
/// The other methods panic rather than returning a plausible default, so a test that reaches one by
/// accident fails loudly instead of passing for the wrong reason.
struct Counting {
    version: Cell<&'static str>,
    detects: Cell<usize>,
    probes: Cell<usize>,
    /// The next `probe` fails with this instead of answering.
    probe_fails: Cell<bool>,
}

impl Counting {
    fn at(version: &'static str) -> Self {
        Self {
            version: Cell::new(version),
            detects: Cell::new(0),
            probes: Cell::new(0),
            probe_fails: Cell::new(false),
        }
    }
}

impl ContainerRuntime for Counting {
    fn detect(&self) -> Result<RuntimeVersion, RuntimeError> {
        self.detects.set(self.detects.get() + 1);
        Ok(RuntimeVersion {
            kind: RuntimeKind::Docker,
            version: self.version.get().to_string(),
        })
    }

    fn probe(&self) -> Result<RuntimeCapabilities, RuntimeError> {
        self.probes.set(self.probes.get() + 1);
        if self.probe_fails.get() {
            return Err(RuntimeError::NotRunning {
                kind: RuntimeKind::Docker,
            });
        }
        Ok(RuntimeCapabilities {
            kind: RuntimeKind::Docker,
            version: self.version.get().to_string(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            // Varied with the version, so a stale answer is visible in the value and not only in
            // the call count.
            storage: match self.version.get() {
                "29.5.1" => LimitSupport::unsupported("the old driver could not"),
                _ => LimitSupport::Supported,
            },
            identity_mapping: IdentityMapping::ExplicitUidGid,
        })
    }

    fn inspect_image(&self, _reference: &str) -> Result<Option<ImageFacts>, RuntimeError> {
        unreachable!("the capability cache never touches an image")
    }
    fn acquire_image(
        &self,
        _source: &ImageSource,
        _progress: &mut dyn FnMut(Progress),
    ) -> Result<ImageFacts, RuntimeError> {
        unreachable!("the capability cache never touches an image")
    }
    fn create(&self, _spec: &SandboxSpec) -> Result<ContainerId, RuntimeError> {
        unreachable!("the capability cache never creates a container")
    }
    fn start(&self, _id: &ContainerId) -> Result<(), RuntimeError> {
        unreachable!("the capability cache never starts a container")
    }
    fn stop(&self, _id: &ContainerId) -> Result<(), RuntimeError> {
        unreachable!("the capability cache never stops a container")
    }
    fn remove(&self, _id: &ContainerId) -> Result<(), RuntimeError> {
        unreachable!("the capability cache never removes a container")
    }
    fn inspect(&self, _id: &ContainerId) -> Result<ContainerFacts, RuntimeError> {
        unreachable!("the capability cache never inspects a container")
    }
    fn find(&self, _name: &str) -> Result<Option<ContainerFacts>, RuntimeError> {
        unreachable!("the capability cache never looks for a container")
    }
    fn logs(&self, _id: &ContainerId, _lines: usize) -> Result<Vec<String>, RuntimeError> {
        unreachable!("the capability cache never reads logs")
    }
}

/// Asked ten times while nothing changes, the expensive half runs once.
///
/// The settings view reads capabilities to decide which limits are editable, and a view function
/// runs on every frame. Without this, opening the daemon settings page spawns a `docker info` per
/// frame for as long as it is open.
#[test]
fn the_probe_runs_once_while_the_runtime_is_unchanged() {
    let runtime = Counting::at("30.0.0");
    let mut cache = CapabilityCache::new();

    for _ in 0..10 {
        cache.capabilities(&runtime).expect("the probe answered");
    }

    assert_eq!(
        runtime.probes.get(),
        1,
        "the probe ran once per call, not once per version"
    );
}

/// And it runs again the moment the runtime underneath changes.
///
/// A cache keyed on nothing would be worse than no cache: the user upgrades Docker, the limit that
/// was unavailable becomes available, and the application goes on telling them it cannot be set
/// until they restart it.
#[test]
fn a_new_runtime_version_re_probes() {
    let runtime = Counting::at("29.5.1");
    let mut cache = CapabilityCache::new();

    let before = cache.capabilities(&runtime).expect("the probe answered");
    assert!(
        !before.storage.is_supported(),
        "the fixture's old version cannot enforce storage; the test proves nothing otherwise"
    );

    runtime.version.set("30.0.0");
    let after = cache.capabilities(&runtime).expect("the probe answered");

    assert_eq!(runtime.probes.get(), 2, "the upgrade did not re-probe");
    assert!(
        after.storage.is_supported(),
        "the answer is still the one from before the upgrade"
    );
    assert_eq!(after.version, "30.0.0");
}

/// Every call asks *which* version, which is the cheap half.
///
/// Stated so the design is deliberate rather than incidental: the cheap command runs every time
/// precisely so the expensive one does not have to.
#[test]
fn the_cheap_half_is_what_decides() {
    let runtime = Counting::at("30.0.0");
    let mut cache = CapabilityCache::new();
    for _ in 0..3 {
        cache.capabilities(&runtime).expect("the probe answered");
    }
    assert_eq!(runtime.detects.get(), 3);
    assert_eq!(runtime.probes.get(), 1);
}

/// A runtime that stops responding does not erase what was already known.
///
/// The daemon going down mid-session is ordinary (US6), and the settings view asking for
/// capabilities at that moment should surface the failure — not lose the cached answer, so that the
/// next successful call has to pay for the probe again.
#[test]
fn a_failed_probe_leaves_the_cached_answer_alone() {
    let runtime = Counting::at("30.0.0");
    let mut cache = CapabilityCache::new();
    cache.capabilities(&runtime).expect("the probe answered");

    runtime.version.set("31.0.0");
    runtime.probe_fails.set(true);
    assert!(
        cache.capabilities(&runtime).is_err(),
        "a runtime that is down must be reported, not papered over with a stale answer"
    );

    runtime.probe_fails.set(false);
    let after = cache.capabilities(&runtime).expect("the probe answered");
    assert_eq!(after.version, "31.0.0");
}

fn caps(storage: LimitSupport) -> RuntimeCapabilities {
    RuntimeCapabilities {
        kind: RuntimeKind::Docker,
        version: "30.0.0".into(),
        cpus: LimitSupport::Supported,
        memory: LimitSupport::Supported,
        pids: LimitSupport::Supported,
        storage,
        identity_mapping: IdentityMapping::ExplicitUidGid,
    }
}

fn every_limit_set() -> SandboxProfile {
    SandboxProfile {
        budget: ResourceBudget {
            cpus_milli: Some(MilliCpus(2500)),
            memory_bytes: Some(Bytes::from_mib(4096)),
            pids: Some(512),
            storage_bytes: Some(Bytes::from_mib(8192)),
        },
        ..SandboxProfile::default()
    }
}

/// The property RC-3 exists for: the user's stored intent survives a runtime that cannot honour it.
///
/// Reconciliation is a *report*, not an edit. A version that cleared the unsatisfiable limits would
/// look correct — the sandbox starts, nothing is silently unenforced — and would quietly destroy a
/// setting the user chose, on a machine they may be borrowing for an afternoon.
#[test]
fn reconcile_never_mutates_the_profile() {
    let profile = every_limit_set();
    let before = profile.clone();

    let report = reconcile(
        &profile,
        &caps(LimitSupport::unsupported("this driver cannot")),
    );

    assert_eq!(
        report.len(),
        1,
        "one limit is unsupported, so exactly one is reported: {report:?}"
    );
    assert_eq!(report[0].field, "storage");
    assert_eq!(
        profile, before,
        "the profile was edited; the user's stored intent is gone"
    );
}

/// And it is total: every combination of set/unset against supported/unsupported has an answer, and
/// none of them panics.
///
/// Stated as an exhaustive walk rather than a handful of cases because the failure mode is an
/// unexpected combination — a limit the user set on a runtime that never reports on it — arriving
/// in a view that then cannot draw.
#[test]
fn reconcile_is_total() {
    for set in [false, true] {
        for supported in [false, true] {
            let profile = SandboxProfile {
                budget: ResourceBudget {
                    cpus_milli: set.then_some(MilliCpus(2500)),
                    memory_bytes: set.then(|| Bytes::from_mib(4096)),
                    pids: set.then_some(512),
                    storage_bytes: set.then(|| Bytes::from_mib(8192)),
                },
                ..SandboxProfile::default()
            };
            let support = if supported {
                LimitSupport::Supported
            } else {
                LimitSupport::unsupported("not here")
            };
            let mut all = caps(support.clone());
            all.cpus = support.clone();
            all.memory = support.clone();
            all.pids = support;

            let report = reconcile(&profile, &all);
            let expected = if set && !supported { 4 } else { 0 };
            assert_eq!(
                report.len(),
                expected,
                "set={set} supported={supported} reported {report:?}"
            );
            for limit in &report {
                assert!(
                    !limit.reason.is_empty(),
                    "an unsatisfiable limit with no reason is one the view cannot explain"
                );
            }
        }
    }
}

/// An unset limit is never reported, however incapable the runtime is.
///
/// The distinction `Option` carries (RB-2): unset means "leave the runtime's default", which no
/// runtime can fail to honour. Reporting it would fill the settings view with warnings about
/// limits the user never asked for.
#[test]
fn an_unset_limit_is_not_a_complaint() {
    let profile = SandboxProfile {
        budget: ResourceBudget {
            cpus_milli: None,
            memory_bytes: None,
            pids: None,
            storage_bytes: None,
        },
        ..SandboxProfile::default()
    };
    let mut none_of_it = caps(LimitSupport::unsupported("not here"));
    none_of_it.cpus = LimitSupport::unsupported("not here");
    none_of_it.memory = LimitSupport::unsupported("not here");
    none_of_it.pids = LimitSupport::unsupported("not here");

    assert!(reconcile(&profile, &none_of_it).is_empty());
}
