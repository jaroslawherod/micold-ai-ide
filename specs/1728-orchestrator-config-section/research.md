# Research: `[orchestrator]` config section

## D1. All four file keys are `SecretPath`

- **Decision**: `ca_file`, `cert_file`, `key_file`, `enrollment_token_file` are `SecretPath`.
- **Rationale**: its `Debug`/`Display` already hide content and the resolved view already probes it
  as path + state (FR-007); the spec asks CA and cert to be shown the same way.
- **Rejected**: `PathBuf` for CA/cert (would need a second view path and could print content
  differently); a new wrapper type (duplicates `SecretPath`).

## D2. Hand-written URL check, no `url` crate

- **Decision**: a small function: scheme `https` (case-insensitive), non-empty host, no userinfo
  (`@` in authority), no `#` fragment; anything else is an invalid URL.
- **Rationale**: `url` is not a dependency of `micold-core`; the rule set is tiny and nothing is
  connected to here. Avoids a new dependency for a syntactic check.
- **Rejected**: `url::Url::parse` (new dependency in the render-free core); regex (opaque).

## D3. Which files are "secrets" for modes and watching

- **Decision**: `secret_entries` gains `key_file` and `enrollment_token_file` (V14 mode check,
  daemon watches and hashes them). `ca_file` and `cert_file` are public: shown with state, checked
  for existence under `--files`, not mode-checked.
- **Rationale**: spec US3 scenario 4 applies the mode rule to a "secret file"; a CA or certificate
  readable by others is normal.
- **Rejected**: mode-checking all four (false warnings on certificates).

## D4. Existence check is V15, not "secret unavailable"

- **Decision**: add all four to the `check_files` required list, so a missing one is an error
  (spec edge case on `--files`), a directory too (`is_file`).
- **Rejected**: leaving them to the secret "reported unavailable" path (spec requires an error).

## D5. Rule ids and checker placement

- **Decision**: new method `Checker::orchestrator` in `validate.rs` with rule ids O1 (url missing),
  O2 (url invalid or not https), O3 (cert/key pair), O4 (token and pair both), O5 (neither). Path
  absoluteness joins the existing V4 list. Errors are not short-circuited: an empty table reports
  O1 and O5.
- **Rejected**: a separate module (validate.rs is the single rule list; the file is large but the
  addition is about 60 lines).

## D6. Reload and restart

- **Decision**: no daemon code change. `config_load` already ignores a file whose `diagnose` has
  errors. Nothing consumes the table yet, so it is not listed under restart-needed keys; the
  consuming issue decides.

## D7. Header anchor

- **Decision**: a small helper in `Doc` returns the span of the `[orchestrator]` header (the table
  key span from the parsed document, falling back to a text search for the header line); tests
  assert the header line:column for an empty table and a table with keys.
- **Rejected**: `locate(.., false)` (value span may start at the first key or be empty).
