# Contract: container daemon form

- **Container name** field: no required marker. Empty, with a usable daemon name: placeholder is the default (e.g. `build-box` for `build box`), updated as the daemon name changes. Empty daemon name or no usable character: placeholder empty.
- Typed value: shown as typed, used trimmed, never replaced.
- Save with empty field:
  - usable default: saved with that name;
  - no usable default: refused, "Enter a container name." on the container name field;
  - default already used by another daemon: "That container is already used by daemon X." on the container name field.
- Edit of a saved daemon: field holds the stored name as a value.
- No other field, message or layout changes.
