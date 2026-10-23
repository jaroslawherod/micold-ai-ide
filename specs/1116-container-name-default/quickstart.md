# Quickstart: Container name default

## Part A: automated

`mise run test-core` (function), `cargo test -p micold-client --test features_daemons`, then `mise run gate`.

## Part B: visual pass

1. Settings, Daemons, Add daemon, Runs in container. The Container name label has no required marker and the field is empty.
2. Type `build box` as the name: the container name field shows `build-box` as dim placeholder text, not as a value.
3. Type `my-ctr` in the field: the placeholder is gone, the value is `my-ctr`.
4. Clear it, Save: the daemon appears; reopen it with Edit: the field holds `build-box` as a value.
5. Add another container daemon named `build box`, leave the field empty, Save: the error sits under the Container name field.
6. Name `日本語`, empty field, Save: "Enter a container name." under the field.
