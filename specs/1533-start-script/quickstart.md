# Quickstart: the start script (#1533)

Part A is automatic (`mise run gate`, plus `mise run test-sandbox` for container delivery). Part B
needs eyes at a display (`visual-pass`).

## Part A: behaviour

1. `micold-daemon config check daemon.toml` with a `[startup]` table prints the resolved view and
   exits 0; `timeout_secs = 0` exits 2 naming the key.
2. (In a container also check `echo $HOME` in the script equals `MICOLD_HOME`.) Script `export FOO=bar; mkdir -p ~/.local/bin; printf '#!/bin/sh\necho hi\n' > ~/.local/bin/hi; chmod +x ~/.local/bin/hi;
   export PATH=$HOME/.local/bin:$PATH`. Start a host daemon with it; a new session runs `echo $FOO; hi`.
   Expected `bar`, `hi`.
3. `env_file` with `TOKEN=x`; a script that only does `echo "$TOKEN" >/dev/null` leaves `TOKEN` out of a
   session; `export TOKEN` puts it in.
4. `run = "once_per_container"`: restart twice, the second start's log has no script output; edit the
   script, the next start runs it. A failing script is not recorded.
5. `echo oops; exit 1` with `refuse`: the daemon does not listen; `startup-result.json` says
   `Failed`, `Exit(1)`, tail `oops`. With `warn`: it listens and `ConfigStatus.startup` is `Warned`.
6. Change the script on a running daemon: it does not re-run; the status shows `restart_needed` with
   `startup`.
7. Repeat 2 and 5 on SSH, container (`mise run image`, `mise run test-sandbox`) and a pod once each
   delivery issue has merged.

## Part B: visual pass

1. Settings → Daemons → a daemon → Start script: pick a file; then switch to Inline and type a script.
   Look at spacing, labels, the validation message for timeout 0 and the "Restart to apply" notice.
2. With a refused daemon, open its error state: the exit status and the tail are readable and
   "Show output" opens the full tail. With `warn`, the warning row shows the same text.
