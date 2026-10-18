# Cycle Log: Kubernetes session daemon runtime

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 6966 passed, 6 failed,
  12 ignored, 531 test binaries
- commit: `270a530f`
- recorded: cycle 0, before any change

## Notes and deviations

- The baseline ran in a cloud container as root (uid 0). Root ignores file permissions, so six
  tests that make a path unreadable or unremovable and expect a refusal fail here, and only here:
  `settings_refuses_save_over_failed_read::a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`,
  `settings_write_is_logged::a_refused_write_is_logged_as_a_failure_with_the_reason`,
  `worktree_leftovers::{a_directory_that_cannot_be_emptied_names_what_survived, the_leftover_report_is_capped}`,
  `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`,
  `settings_service_write_refuses_failed_read::a_service_write_over_an_unreadable_settings_file_is_refused`.
  None touches this feature's components. The loop treats them as pre-existing reds: run the
  loop as a non-root user, or exclude these six when judging "suite green".

## Cycle 1 (batch, M1 core types): U1–U12, U18–U21, U23–U25, U26 — RED, handed over

- deviation: one batch of behaviours per source file rather than one per cycle, to fit the milestone
  (38 tasks) into unit-sized contexts; each test still written and observed failing before its
  implementation. Stubs only (bodies return the wrong value), no implementation yet.
- tests: `crates/micold-core/src/kube/{settings,names,classify}.rs`, `daemon_image/mod.rs`,
  `protocol/auth.rs` (`listener_auth_tests`), all `#[cfg(test)]`.
- red: `scripts/build-lock.sh cargo test -p micold-core --lib -- kube:: daemon_image:: listener_auth_tests`
  -> `test result: FAILED. 4 passed; 18 failed` (assertion failures, e.g.
  `kube::names::tests::the_key_is_eight_hex_of_the_nul_joined_triple panicked at crates/micold-core/src/kube/names.rs:125:9`).
- passed on first run against the stubs (need the deliberate-mutant check after green):
  `a_toleration_with_another_effect_is_refused_naming_tolerations`, `each_of_the_four_can_be_set_alone`,
  `defaults_are_amd64_no_storage_class_and_nothing_allowed`, `an_empty_path_on_loopback_needs_no_token`.
- additions to the data model (recorded for review): `KubeFailure::Unreachable`/`Forbidden`/`Conflict`
  carry `context`/`namespace` for their contract text; new `VolumeTooSmall { claim }` (T021's
  smaller-volume refusal) and `Api { code, reason, message }` (unclassified answers until M6).
- commit: the handover commit (red; next unit makes it green).

## Cycle 1 (continued): GREEN; Cycle 2: U13–U16 (`DaemonRuntime::Kubernetes`)

- green: `cargo test -p micold-core --lib -- kube:: daemon_image:: listener_auth_tests` ->
  `test result: ok. 22 passed; 0 failed`.
- test fix: `the_key_is_eight_hex_of_the_nul_joined_triple` compared `("a\0b","c","d")` with
  `("a","b\0c","d")`, which join to the same bytes under any NUL separator (the test was wrong, not
  the rule); now `("ab","c","d")` vs `("a","bc","d")`.
- mutants (one run, all four killed): toleration `a:Maybe:E` accepted -> `a_toleration_with_another_effect_…`
  FAILED; request without limit refused -> `each_of_the_four_can_be_set_alone` FAILED; default volume 1 GiB
  -> `defaults_are_amd64_…` FAILED; `is_loopback` -> `is_unspecified` -> `an_empty_path_on_loopback_…` FAILED.
- object names: the slug inside names is cut to 31 so `micold-<slug>-<key>-git-credentials` stays a
  DNS-1123 label (63); `slug()` itself keeps its 40 cap.
- cycle 2 red: `crates/micold-core/tests/daemons_kube.rs` -> `error[E0599]: no variant named Kubernetes`
  (compile red); green after T013: `cargo test -p micold-core --all-targets --no-fail-fast` fails only
  the pre-existing root-user reds (`settings_refuses_save_over_failed_read`, `settings_write_is_logged`,
  `worktree_leftovers` ×2).
- fixture `settings_unsupported_kind.json` used `kubernetes` as the unknown kind; now `nomad`.
- U16 decision: `RegistryError::DuplicateName` stays field-less; the form already names the first
  ("Another daemon is already named <name>.", `features/daemons.rs` `form_error`), since names
  match case-insensitively the draft's name is the first's name.

### Unit 3 of M1 (2026-10-17)

- T001–T003 confirmed done in 52d8a1ab: `kube` 4.2 / `k8s-openapi` 0.28 in the client only,
  `cargo tree -p micold-client -i openssl-sys` matches no package, core and daemon gain no dependency.
- T008/T016 daemon half (U22–U25): `daemon/server.rs` `token_auth_path` decides through
  `listener_auth`; unset path = host process, empty + loopback = no token, empty + `0.0.0.0:7727` =
  fatal `InvalidInput` (`token_auth_tests`, 3 green). Written with the code, no separate red run.
- T010/T017 (U34): `Transport::Tunnel(Tunnel(Box<dyn AsyncReadWrite>))`; the duplex-half handshake
  test against a `DaemonCodec` stub is green. Compile red only (the variant did not exist).
- T015: `Cluster` port (`kube/mod.rs`: `ObjectKind`, `ApiFailure`, `Created`, `ObjectView`,
  `PodStatusView`, `EventView`, boxed futures) and `FakeCluster` (`kube/fake.rs`) with 3 self-tests;
  `cargo check -p micold-client -p micold-daemon --tests` green.

### M1a, unit 1 (revision 2, 2026-10-17)

Inner loop: `scripts/build-lock.sh cargo test -p micold-core --lib kube::` (single-module filter;
the core baseline still fails only the four root-user tests named above).

- U45–U48 (`kube/kubeconfig.rs`, tests from stash@{0}; stubs return empty values, not `todo!()`).
  red: 6 failed, e.g. `kubeconfig_splits_on_the_platform_separator_keeping_order` `left: [] right:
  ["/b/one", "/a/two", "/c/three"]`; `login_shell_output_yields_…` `left: Some("") right:
  Some("/usr/bin:/opt/bin")`. green: `home_var`, `list_separator`, `candidates`, `search_path`
  (first occurrence kept, `~/` = home), `parse_login_shell_output`; 6 passed. refactor:
  `split_list` returns a `Vec` instead of a collected iterator.
- U117 (`Quantity::{to_k8s, parse_k8s}`, tests from stash@{0}). red: 3 failed, `left: "" right:
  "10Gi"`, `left: None right: Some(Quantity(10737418240))`. green: `BINARY_UNITS` /
  `DECIMAL_UNITS` tables; 26 passed in `kube::`. The field-naming refusal of an unparseable
  quantity belongs to the form (M4a); `parse_k8s` answers `None`. refactor: none needed.
- U20 (access secret), U120 (`all_secrets(&allowed)`), `ObjectKind::noun`. red: 3 failed,
  `left: "" right: "micold-build-box-8384f9ba-ssh-access"`, `left: [] right: [""]`, `left: ""
  right: "pod"`. green: 29 passed in `kube::`. The stash's `all_secrets()` (every possible
  secret) was replaced by the revision-2 signature. refactor: none needed.
- U18, U19, U21: covered by revision 1's passing tests in `kube/names.rs` (red recorded in unit 2
  of M1 above); marked DONE with the tests named, not re-run red.

### M1a, unit 2 (revision 2, 2026-10-17)

- U26 (T109/T114, `kube/classify.rs`). red: did not compile, `no variant named ImageContract`,
  `HostKeyMismatch`, `StopTimeout`, `Unavailable`; `Api has no field named status`. green: the five
  variants with their contract §6 texts, `NoRuntime`/`Build`/`Push`/`VolumeTooSmall` removed,
  `From<ApiFailure>` (stash@{0}, re-shaped to `Api { status, message }`); 30 passed in `kube::`.
  refactor: none needed.
- U121 (T110/T115, `kube/fake.rs`, `kube/mod.rs`). red: did not compile, `no method named
  replace`, `hold_on_delete`, `release`; `no field resource_version on ObjectView`. green:
  `Cluster::replace(ns, kind, name, resource_version, manifest)` (stale version: `409`, nothing
  changed), `ObjectView::resource_version`, `FakeCluster::{hold_on_delete, release}` (stash@{0},
  plus `release`), `node_arch`/`set_node` removed; `port_forward` already logged the port;
  34 passed in `kube::`. refactor: none needed.
- U8, U11, U12, U13–U16, U118, U119 (T106/T107/T111/T112/T143; `kube/settings.rs`,
  `daemon_resources.rs`, `tests/daemons_kube.rs`). red: did not compile, `unresolved import
  crate::daemon_resources`, `no method named image_is_moving`, `no field providers`, 7 `mismatched
  types` (`KubeSettings::new` taking `&[AiCli]`). green: `DaemonResources` and its three tests
  moved to `daemon_resources.rs`, `daemon_image/` deleted; `KubeSettings` gets `image:
  Option<ImageRef>` (stored as the reference string), `providers: BTreeSet<AiCli>`,
  `image_is_moving`, `new(context, namespace, enabled)`; `RegistryRepo`, `NodeArch`, `registry`,
  `arch` and their two tests (U3, U4) removed; core lib 719 passed, `daemons_kube` 6 passed.
  U1, U2, U5–U7: revision 1's tests, unchanged and green. refactor: none needed.
- T116 (revert, no new behaviour): `protocol/auth.rs`, `daemon/server.rs` and `connect.rs` restored
  to `origin/main`; `git diff origin/main -- <the three>` is empty; feature 027's token tests are
  covered by the gate.
- U35–U41, U122, U123, U124, U125 (T019/T117/T118/T025/T119/T120; `kube/manifest.rs`,
  `kube/access.rs`, `FakeKeyGen`). red: tests over `todo!()` stubs, 17 failed with `not yet
  implemented`. green: `pod`, `pvc`, `secret`, `PodStamp::of`, `drift`, `AccessSecret::{parse,
  to_data, with_client, without_client}`; the `ResourceNames` keeps the daemon's name,
  `ObjectView` gains `annotations` and `data` (secrets travel as `stringData`, which the live
  adapter decodes on read). The access-dir variable is `MICOLD_SSH_DIR` (assumed; the shared
  image contract names only the directory). kube:: 73 passed after both entries (access 5, manifest 11, start 17). refactor: none needed.
- U49–U57, U126–U128 (T021/T027/T121; `kube/start.rs`, `start_tests.rs`). Tests and sequence were
  written together, no separate red run: the sequence has no meaningful stub (the first run of
  the suite was the first compile). Covers plan and bounds, exact timeout, create/adopt/Conflict,
  deleting pod waited out, key race and idempotence, drift Ask/Recreate/Keep, rollback in reverse
  with the added key, adopted objects kept; preflight reviews, classifiers and `Tunnel` are
  M2/M1c. green: see above. refactor: none needed.
- U129–U131, U86–U89 (T122/T023/T123/T124/T125; `Tunnel` in `kube/start.rs`, `client/kube/{live,relay,keys}.rs`).
  Tests and code were written together in commit 1dda9203, no separate red run (no meaningful stub
  for adapter helpers over a real client); review B flagged this and it is recorded here as
  such. After review A (retry until the bound), a real red: `a_port_that_closes_before_sshd_listens_is_retried_until_it_answers`
  failed (start failed `ImageContract` on the first closed stream), green after the retry loop.
  Covers: banner/non-SSH/silent/exited container, relay forwarding and refused forward, key
  files created once, stable id, owner-only modes, `known_hosts` pinning. refactor: none needed.

## M3 — Only allowed credentials reach the pod (T063–T069, T102)

- A10–A13, U43, U44, U58, U59, U66–U70, U136: tests written with the code in the same cycles
  (`kube::credentials::`, `kube::manifest::`, `kube::start::`); each asserts on the secrets,
  volumes, mounts and security contexts of the manifests the `FakeCluster` recorded. No red run
  was captured separately in this cloud unit (the tests and their code were added together);
  recorded honestly here. Waiver (review B F1): the behaviours are pure manifest/credential builders over the fake cluster; a late red would re-test reverted code, so the strength check is the milestone's mutation pass in close (T098). Review A's SSH-key mode fix: `GIT_SSH_COMMAND` copies the key to a 0600 temp file (the old assertion `-i /run/micold/creds/ssh/id` fails against the new command, the new one passes). refactor: none needed.
- Decision: with nothing allowed the `Secrets` phase makes no create, replace or delete of a
  credential secret and reads no source file, but it does `get` each possible credential secret
  name to find one a withdrawal left behind (there is no list call), so A12 holds when the last
  credential is withdrawn.

## M4a — The Kubernetes form, Add entry hidden (T024, T130, T066, T031, T131, T070, T032)

- U95, U96, U97, U99, U104, U134, U135: the reducer tests (`tests/features_daemons.rs`, module
  `kubernetes`) were written first, against an API that did not exist: red was the compile failure
  (`runtime_choices`, `Msg::ContextsListed`, `KubeDraft`, … unresolved). Green came with
  `features/daemons_kube.rs` and the reducer arms; one test then failed for a real reason (the row's
  endpoint was empty for Kubernetes) and passed after `views` used `KubeSettings::endpoint`.
- T130 characterization: `the_container_limits_are_edited_as_before` passed before and after the
  `limit_field` extraction (T131), which moved the drawing loop only.
- T032: `tests/daemons_section_geometry.rs` paints the Kubernetes form and row (taller window; the
  default one cut the form off, which the first run showed).
- Decision: "providers enabled in Settings" has no per-provider switch yet; the default is the
  Settings draft's `default_ai_cli` (Claude Code when no draft is open).
