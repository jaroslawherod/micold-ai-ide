# Research: Container name default

## R1: Where the sanitizer lives
- **Decision**: `micold_core::daemons::default_container_name`, pure.
- **Rejected**: client-only (cannot be tested by `test-core`, easy to diverge if the daemon ever needs it); in `DaemonRegistry::add` (the registry takes a resolved `ContainerSettings` and has no Draft; an implicit rewrite there would hide the value from the form and the placeholder).

## R2: Where the name is resolved
- **Decision**: at save in `runtime_of` (client), through `Draft::resolved_container_name`, shared with `form_error` and the placeholder helper.
- **Rejected**: copying the default into `draft.container_name` while typing the daemon name (the field would then be a typed value, FR-004 could not tell default from typed, and editing the daemon name would fight the user's edits); resolving inside the registry (see R1).

## R3: Required marker
- **Decision**: drop the entry from `Draft::required_fields`. The "required" state, section auto-open and the empty-attempt marker all derive from that list, so nothing else changes.
- **Rejected**: keep it required only when the daemon name is unusable (the marker would flicker while typing and the refusal is already a field error).

## R4: Unusable default
- **Decision**: reuse `invalid_field("container_name")` ("Enter a container name."); no new message.
- **Rejected**: a new "cannot derive" sentence (more copy and a new key for the same action: type a name).

## R5: Collision
- **Decision**: refuse (no suffix), per clarify D1. The `ContainerInUse` branch must compare against the resolved name (FR-010).
- **Rejected**: auto-suffix (`-2`): unpredictable names and a hidden second place where uniqueness is decided.

## R6: Length
- **Decision**: 63 (clarify D2), truncate before trimming trailing chars so the cut never leaves a trailing separator.
- **Rejected**: 255 / no limit (runtimes differ; 63 fits all).

## R7: Placeholder
- **Decision**: reuse `TextField::new(placeholder, ..)` (the port field uses it). Empty string when no default, so it is hidden.
- **Rejected**: helper text under the field (extra row; geometry gate and section height change).
