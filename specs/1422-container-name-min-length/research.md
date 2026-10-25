# Research: Container name minimum length

## D1: Where the minimum is enforced
- **Decision**: in the client form's `runtime_of` (Container arm), using a core predicate.
- **Rationale**: the form is the only entry point; load paths must keep legacy names (FR-006); the message belongs on the form field like other field errors.
- **Rejected**: `DaemonRegistry::check` (also runs where stored one-character names must still pass; would risk dropping legacy daemons, violating FR-006/SC-004). `default_container_name` returning `None` when short (the user would see "Enter a container name." instead of the minimum message, contradicting Scenario 1.1).

## D2: Typed names
- **Decision**: checked with the same predicate (spec FR-004).
- **Rejected**: defaulted names only; leaves the typed path broken.

## D3: Constant location
- **Decision**: `CONTAINER_NAME_MIN` beside `DEFAULT_CONTAINER_NAME_MAX` in `micold_core::daemons`; fixed value, not a setting (spec Assumptions).
- **Rejected**: literal `2` in the client; duplicates the rule and the message.

## D4: Counting
- **Decision**: `chars().count()` of the trimmed name.
- **Rationale**: spec Edge Cases say characters; typed names have no charset check, so multibyte input is possible.
- **Rejected**: `len()` bytes (a single multibyte character would pass).

## D5: Placeholder
- **Decision**: `container_name_hint` filters the default through the predicate.
- **Rejected**: leaving it and relying on the Save message; promises a name Save refuses (FR-005).

## D6: Short defaults
- **Decision**: no padding (spec Out of Scope).
