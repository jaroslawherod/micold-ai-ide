# Data model: Daemon settings in titled sections

View state only. No stored type changes: `DaemonRegistry`, `DaemonRuntime`, `SandboxProfile` and
`KubeSettings` are untouched, so every stored daemon reads, edits and saves as before (SC-004).

## Section (render-free, `crates/micold-client/src/features/daemon_sections.rs`, new)

Moved from `features/daemons.rs` (`Section`, `Section::ALL`, `Section::title`, `Section::holds`)
and widened; re-exported as `features::daemons::Section`.

| Variant | Title (header text) | Kinds |
|---|---|---|
| `Connection` | Connection | Container, Kubernetes, SSH, Host |
| `Container` | Container | Container |
| `Image` | Image | Kubernetes |
| `Credentials` | Credentials | Container, Kubernetes |
| `Network` | Network | Container |
| `Resources` (was `Limits`) | Resources | Container, Kubernetes |
| `Placement` | Placement and storage | Kubernetes |
| `Sessions` | Sessions | Container, Host |

- `Section::of(runtime: DraftRuntime) -> &'static [Section]` — FR-002, in order:
  - Container: `Connection, Container, Credentials, Network, Resources, Sessions`
  - Kubernetes: `Connection, Image, Credentials, Resources, Placement`
  - SSH: `Connection`
  - Host: `Connection, Sessions`
- `Section::title(self) -> &'static str` — the table above.
- `Section::ALL` is dropped (it listed the container's sections); callers use `Section::of`.

### Field-to-section map

`Section::holding_error(runtime, field: &str) -> Option<Section>` (for `FormError::field`) and
`Section::holding(runtime, id: FieldId) -> Option<Section>` (for required fields) answer from one
table. A field not listed (`name`, `runtime`, `DaemonName`) is in no section.

| Runtime | Section | `FormError::field` names | `FieldId`s |
|---|---|---|---|
| Container | Connection | `container_name`, `port` | `DaemonContainerName`, `DaemonPort`, `DaemonAutoStart` |
| Container | Container | `image`, `image_path` | `DaemonImage`, `DaemonImagePath`, `DaemonAiCli(_)` |
| Container | Credentials | — | `DaemonCredential(_)` |
| Container | Network | — | — |
| Container | Resources | `cpus`, `memory`, `pids`, `storage` | `DaemonCpuLimit`, `DaemonMemoryLimit`, `DaemonPidLimit`, `DaemonStorageLimit` |
| Container | Sessions | — | `DaemonSurviveLogout` |
| Kubernetes | Connection | `context`, `namespace` | `DaemonKubeContext`, `DaemonKubeNamespace`, `DaemonAutoStart` |
| Kubernetes | Image | `image` | `DaemonKubeImage`, `DaemonKubeProvider(_)` |
| Kubernetes | Credentials | `ssh_key_file`, `pull_file` | `DaemonKubeAllowed(_)`, `DaemonKubeSshKeyFile`, `DaemonKubePullFile` |
| Kubernetes | Resources | `cpu_request`, `cpu_limit`, `memory_request`, `memory_limit` | `DaemonCpuRequest`, `DaemonCpuLimit`, `DaemonMemoryRequest`, `DaemonMemoryLimit` |
| Kubernetes | Placement | `node_selector`, `tolerations`, `volume_size`, `storage_class` | `DaemonKubeVolumeSize`, `DaemonKubeStorageClass` |
| SSH | Connection | `host`, `user`, `port`, `key_file` | `DaemonSshHost`, `DaemonSshUser`, `DaemonSshPort`, `DaemonSshKeyFile`, `DaemonAutoStart` |
| Host | Connection | — | `DaemonAutoStart` |

The `FormError::field` names are the ones `ui/settings/daemons.rs` reads with `error_of` today; the
tasks unit checks the list against every `error_of(state, "…")` call so that no refusal lands in a
closed section unopened.

## Section open state (`features::daemons::State`)

- `flipped_sections: BTreeSet<Section>` — unchanged type; cleared by `begin_add`, `begin_edit`,
  `discard_draft` (as today) **and by `Msg::RuntimeChosen`** (FR-004).
- `State::section_open(section) -> bool` =
  `(section == first of Section::of(runtime)) XOR flipped`
  `|| holding_error(runtime, form_error.field) == Some(section)`
  `|| any (id, req) in draft.required_fields() with holding(runtime, id) == Some(section) and draft.required.state(id, req) == Missing`.
  With no draft, `false`.
- Transitions: `SectionToggled(s)` flips `s` (FR-005: no draft or registry change);
  `RuntimeChosen(_)` → default state of the new kind; a refused save or a left-empty required field
  → its section forced open while the error stands; closing the form → forgotten.

## Section summary and marker (render-free, `daemon_sections.rs`)

- `summary(draft: &Draft, section: Section) -> Option<String>` — the exact texts are the
  [UI contract](contracts/ui.md#section-summaries). `None` when the contract gives no fallback and
  every part is empty.
- `marked(draft: &Draft, section: Section) -> bool` — some `(id, req)` of
  `draft.required_fields()` held by `section` has `req.required && req.empty` (FR-014, US3 AS3).
- `AllowedKind::short_label(self) -> String` (new, `features/daemons_kube.rs`): the noun-only name
  the Kubernetes Credentials summary lists (contract table).

## AccordionSection (shared component, `ui/material/accordion_section.rs`)

New optional builder inputs (FR-011 to FR-013); defaults keep today's look:

| Field | Type | Default | Drawn |
|---|---|---|---|
| `summary` | `Option<String>` | `None` | closed only; one line under the title, `Caption`, `on_surface_variant`, ellipsized |
| `required` | `bool` | `false` | closed only; `micold_core::required::MARKER` appended to the title in its role and tint |

Builders: `.summary(impl Into<String>)`, `.summary_maybe(Option<String>)`, `.required(bool)`.

## FieldGroup (shared component, `ui/material/field_group.rs`, new)

| Field | Type | Default | Drawn |
|---|---|---|---|
| `label` | `String` | required | `TypeRole::Label`, muted |
| `note` | `Option<String>` | `None` | `TypeRole::Caption`, muted, directly under the label |
| `children` | `Vec<Element>` | empty | one column at the label's left edge, `FIELD_GAP` apart |

Builder: `FieldGroup::new(label, roles).note(text).push(child).extend(children).into()`.

## ContextEntry (`crates/micold-client/src/kube/live.rs`)

Gains `impl Display` (writes `name`) so a `Select<ContextEntry>` lists contexts (R6). No field change.
