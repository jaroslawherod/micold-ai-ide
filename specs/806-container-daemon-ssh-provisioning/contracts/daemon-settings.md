# Contract: container daemon settings and their migration

The `settings.json` shape of a container daemon (`DaemonRuntime::Container(ContainerSettings)`) and
the read-repair from the pre-upgrade shape. Types: [../data-model.md](../data-model.md).

## Written shape (after this feature)

```json
{ "kind": "container", "container_name": "micold-daemon-3", "port": 7730,
  "migration": "moved_off_released_image",
  "profile": {
    "runtime": "docker",
    "image_choice": { "kind": "basic" },
    "ai_clis": ["ClaudeCode"],
    "budget": { "cpus_milli": 2000, "memory_bytes": 4294967296 },
    "network": "no_outbound", "credentials": [], "survive_logout": false } }
```

`image_choice` is `{"kind":"basic"}`, `{"kind":"own","reference":"<ref>"}` or
`{"kind":"own","archive":"<path>"}`. `migration` is omitted when `None`. Other keys keep their
current names and encodings.

## Read rules (old → new), applied once at load and saved on the next write

| Stored | Becomes | `migration` |
|---|---|---|
| no `image` and no `image_choice` | `basic` | none |
| `image.kind = registry`, repository ∈ {`ghcr.io/jaroslawherod/micold-daemon`, retired namespace}, any tag or digest | `basic` | `moved_off_released_image` |
| `image.kind = registry`, any other reference | `own` + same reference | none |
| `image.kind = imported_file` | `own` + same archive path | none |
| `image.kind = local_build` | `own` + same reference (e.g. `micold-daemon:dev`) | none |
| `image_choice` present | as stored; `image` ignored | as stored |
| no `ai_clis` | every `AiCli` (D3) | — |
| `ai_clis` present | as stored (may be empty) | — |

Every other key (name, port, budget, network, credentials, survive_logout, runtime) is kept
unchanged (FR-016). The migrated daemon 1 (`migrated_entry`) follows the same rules.

## Validation on save (form)

- `own` with neither reference nor archive → refused, naming the field (US3 #4).
- `own` reference that `ImageRef::parse` rejects → refused with its message, as today.
- A reference naming the released image typed into "My own image" → refused: "The released daemon
  image is no longer used; choose the basic image." (FR-003).
- Budget: today's minimums and messages, unchanged (FR-014).
- `ai_clis` empty → accepted (edge case), the row and the session start say no AI CLI is selected.

## Form (spec 804 Container section) — FR-001, FR-004, FR-010, FR-014

- **Image**: the existing shared `Select` (`ui/material/select.rs`) with exactly two entries,
  "Basic image (recommended)" and "My own image" (replacing today's three `IMAGE_SOURCES`); the
  latter shows the existing reference `TextField`, an archive picker and the note "Your image must run an SSH server; see the user
  guide." No option names the released image (US4 #5).
- **AI CLIs**: one checkbox per `AiCli::ALL`, in the shared `Checkbox` (`ui/material/checkbox.rs`); a note that changes
  apply at the next start.
- **Limits**: unchanged fields and notices.
- Any change to image, AI CLIs or limits of a running daemon goes through the existing restart
  confirmation (spec 804 FR-010).
- Not shown for a remote SSH daemon (US6 #3).
