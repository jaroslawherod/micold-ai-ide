# UI contract: daemon form sections

What the add/edit daemon form in Settings › Daemons shows, per kind, after each milestone. Texts in
`code` are exact. `·` below is `" · "` (space, U+00B7, space).

## Form layout (M2; FR-001, FR-002, FR-010)

Top to bottom, every kind:

1. The form title (`Add a daemon` / `Edit daemon`) and the required legend, as today.
2. `Name` text field, `Runs in` select — outside any section.
3. The runtime refusal caution (`FormError` field `runtime`), when one stands — outside any section,
   under `Runs in`.
4. The kind's sections, each an `AccordionSection`, in this order:

| Kind | Sections |
|---|---|
| Container | `Connection`, `Container`, `Credentials`, `Network`, `Resources`, `Sessions` |
| Kubernetes | `Connection`, `Image`, `Credentials`, `Resources`, `Placement and storage` |
| Remote over SSH | `Connection` |
| Host | `Connection`, `Sessions` |

5. `Save daemon` and `Cancel` — outside any section.

No `group(...)` label (`Kubernetes`, `Resources`, `Placement and storage`, `Allowed credentials`,
`Sessions`) is drawn as a heading in the form; the only headings are the section headers.

On open and on every `Runs in` change: `Connection` open, the rest closed (FR-004).

## Section bodies (M2 places the controls; M3 sets their look)

Order inside each body, top to bottom. "note:" is a section-wide first line (muted caption);
"group" is a `FieldGroup` (label, optional note, children); "supporting" is the control's own line.

### Container

- **Connection**: `Container name` (required) · `Port` (placeholder `A free port is chosen if empty`)
  · checkbox `Connect when the application starts`.
- **Container**: `Container runtime` select · `Image` select (supporting as today) · for
  `My own image`: `Image reference` (with the missing-CLI field note) and `Image file` · group
  `AI tools in this daemon`, note = `AI_CLI_NOTE`, one checkbox per AI CLI.
- **Credentials**: note `The container starts with none of your credentials. Share only what the
  agent needs.` · group `Shared with the container`, one checkbox per share, the unshared sign-in
  caution directly under `AI CLI sign-in`, then the sharing-summary caution last in the group.
- **Network**: `Network` select, supporting `Names still resolve either way — the runtime's resolver
  answers from the host side. Blocked means connections fail, not that nothing leaves the
  container.` · the no-outbound caution directly under the select.
- **Resources**: note `What the sandbox may consume. A limit the runtime cannot enforce is shown,
  and says so, rather than being quietly dropped.` · the four limit fields as today.
- **Sessions**: checkbox `KEEP_RUNNING_SETTING` · the survival note (`survival_note`) under it.

### Kubernetes

- **Connection**: when contexts were found, select `Context from your kubeconfig` listing them,
  placeholder `Choose a context`; choosing fills `Context` (and an empty `Namespace`, as
  `KubeDraft::choose_context` does today) · `Context` (required, supporting as today); the
  context-list failure caution directly under it when listing failed · `Namespace` (required) ·
  checkbox `Connect when the application starts`.
- **Image**: `Image` (supporting `Empty uses the default SSH image`) · group `AI CLIs installed in
  the pod`, note `Installed when the pod starts.`, one checkbox per AI CLI.
- **Credentials**: group `Allowed credentials`, note `Nothing reaches the pod unless it is ticked
  here.`, one checkbox per `AllowedKind`, each file field directly under its checkbox while ticked.
- **Resources**: the four request/limit fields as today.
- **Placement and storage**: group `Node selector`, note `One key=value per line`, the key-value
  editor, its error caution directly under it · group `Tolerations`, note `One per line`, the
  toleration editor, its error caution directly under it · `Volume size` · `Storage class`.

### Remote over SSH

- **Connection**: `HostPicker` (when hosts are configured) · `Host` (required) · `User` · `Port` ·
  `Key file` · checkbox `Connect when the application starts`.

### Host

- **Connection**: checkbox `Connect when the application starts`.
- **Sessions**: the host survival note (`survival_note(PlacementKind::HostProcess)`, a caution).

## Spacing (M3; FR-006, SC-003)

- Between consecutive controls in a section body, and between a group's children: `FIELD_GAP`
  (`spacing::SM`, 8 px).
- Between consecutive sections, and between `Runs in` (or its caution) and the first section:
  `SECTION_GAP` (`spacing::SM`, 8 px).
- A `FieldGroup`'s label, note and children start at the same left edge as the body's text fields.

## Section summaries (M3; FR-014, ledger D5)

Shown on a closed header only. A summary is its parts joined with `·`, empty parts left out; when
every part is empty and the row gives no fallback, no summary is shown. Values are the typed text,
trimmed. `<auto>` is `Connects at launch` when *Connect when the application starts* is ticked, else
`Manual start`. `<clis>` is the ticked AI CLIs' display names (`AiCli::ALL` order) joined with
`, `, or `No AI tools` when none is ticked.

| Kind | Section | Parts, in order | Example |
|---|---|---|---|
| Container | Connection | container name · `Port <port>` or, when empty, `Any free port` · `<auto>` | `micold-dev · Port 7050 · Manual start` |
| Container | Container | `Docker` / `Podman` · `Basic image` / `Own image` · `<clis>` | `Docker · Basic image · Claude Code, Codex` |
| Container | Credentials | `Shared: <labels>` (`CredentialShare::label`, `CredentialShare::ALL` order, `, `-joined), or `Nothing shared` | `Shared: Git configuration, SSH agent` |
| Container | Network | `No outbound connections` / `Outbound connections allowed` | `No outbound connections` |
| Container | Resources | `CPU <cpus>` · `Memory <mib> MiB` · `Processes <pids>` · `Storage <mib> MiB`, set ones only; none set: `Runtime defaults` | `CPU 2 · Memory 4096 MiB` |
| Container | Sessions | `Keeps running when signed out` (opt-in ticked) / `Stops when signed out or idle` | `Stops when signed out or idle` |
| Kubernetes | Connection | context · `Namespace <namespace>` · `<auto>` | `dev-cluster · Namespace team-a · Manual start` |
| Kubernetes | Image | image, or `Default SSH image` when empty · `<clis>` | `Default SSH image · Claude Code` |
| Kubernetes | Credentials | `Allowed: <short labels>` (`AllowedKind::all()` order, `, `-joined), or `Nothing allowed` | `Allowed: Git config, SSH key` |
| Kubernetes | Resources | `CPU request <v>` · `CPU limit <v>` · `Memory request <v> MiB` · `Memory limit <v> MiB`, set ones only; none set: `No requests or limits` | `CPU request 0.5 · CPU limit 2` |
| Kubernetes | Placement and storage | `Volume <size>` · `Storage class <class>` or, when empty, `Default storage class` · `Node selector` (any non-blank line) · `Tolerations` (any non-blank line) | `Volume 20Gi · Default storage class` |
| SSH | Connection | `[<user>@]<host>[:<port>]` (user and port only when typed; the whole part only when host is typed) · `<auto>` | `dev@build-box:2222 · Connects at launch` |
| Host | Connection | `<auto>` | `Manual start` |
| Host | Sessions | `Stops when signed out or idle` | `Stops when signed out or idle` |

`AllowedKind::short_label`: `Git config`, `Git credentials`, `SSH key`, `<CLI display name>
sign-in`, `Registry credentials`.

A summary updates on every keystroke (it is recomputed from the draft each view).

## Required marker (M3; FR-012, FR-014)

A closed section header shows `*` after its title when `daemon_sections::marked` is true: a
required field it holds is empty. In practice: Container › Connection (container name), Container
› Container (`My own image` with neither reference nor file), Kubernetes › Connection (context,
namespace), SSH › Connection (host). An open section shows no header marker (its fields carry
their own).

## Shared component (M1; FR-011 to FR-013, FR-015)

`AccordionSection::new(title, body, roles).open(b).on_toggle(m).summary(text).required(b).into()`

- Closed with a summary: title, then the summary on one line under it in `Caption`,
  `on_surface_variant`, ellipsized to the header's width minus the chevron.
- Open: no summary, no marker.
- Without `.summary`/`.required`: identical layout to today (same header height).
- Keyboard, pointer, focus ring, focus stops: unchanged (the header is still one `Button`).
- Showcase entry `AccordionSection` poses: `closed, with a summary`, `open`,
  `closed, a required field empty`, and the live toggle.
