# sourced: defines the private environment and checks it. Aborts on any leak.
P=/tmp/claude-1000/-home-jaro-workspaces-micold-ai-ide--claude-worktrees-feat-notify-session-needs-attention/4922067d-38d0-4f20-a41f-9c0bae9ef34b/scratchpad/probe
RT=/run/user/1000/t091-probe
PHOME=$P/home
PENV=(env -i PATH=/usr/bin:/bin LANG=C.UTF-8 HOME=$PHOME XDG_RUNTIME_DIR=$RT
  XDG_CONFIG_HOME=$PHOME/.config XDG_DATA_HOME=$PHOME/.local/share XDG_CACHE_HOME=$PHOME/.cache
  XDG_STATE_HOME=$PHOME/.local/state
  DBUS_SESSION_BUS_ADDRESS=unix:path=$RT/bus DBUS_SYSTEM_BUS_ADDRESS=unix:path=$RT/sysbus
  GSETTINGS_BACKEND=memory XDG_SESSION_TYPE=wayland)
PWL=t091-wl
check_env() {
  local out; out=$("${PENV[@]}" "$@" env) || { echo "ABORT env"; return 1; }
  echo "--- private env ---"; echo "$out" | sort
  echo "$out" | grep -qx "DBUS_SESSION_BUS_ADDRESS=unix:path=$RT/bus" || { echo ABORT session bus; return 1; }
  echo "$out" | grep -qx "DBUS_SYSTEM_BUS_ADDRESS=unix:path=$RT/sysbus" || { echo ABORT system bus; return 1; }
  echo "$out" | grep -q "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus$" && { echo ABORT user bus; return 1; }
  echo "$out" | grep -q "^DISPLAY=" && { echo ABORT DISPLAY set; return 1; }
  echo "$out" | grep -qx "WAYLAND_DISPLAY=wayland-0" && { echo ABORT wayland-0; return 1; }
  echo "$out" | grep -qx "XDG_RUNTIME_DIR=$RT" || { echo ABORT runtime dir; return 1; }
  echo "$out" | grep -qx "HOME=$PHOME" || { echo ABORT home; return 1; }
  [ "$(stat -c %a "$RT")" = 700 ] || { echo ABORT rt mode; return 1; }
  echo "--- env OK ---"
}
