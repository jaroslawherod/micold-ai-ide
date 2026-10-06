#!/bin/sh
# Repro for #566 on a private session bus.
# Usage: cargo build --manifest-path <this dir>/Cargo.toml --target-dir <scratch>; repro.sh <scratch>/debug/probe566
P=$1
eval $(dbus-daemon --session --fork --print-address=1 --print-pid=1 | { read a; read p; echo "export DBUS_SESSION_BUS_ADDRESS='$a' BUSPID=$p"; })
run() { # $1 = rule label, $2 = rule
  echo "== $1: $2"
  $P "$2" > $1.out & L=$!
  $P own & O=$!
  sleep 0.7
  U=$(sed -n 's/^UNIQUE //p' $1.out)
  # a peer that does not own org.freedesktop.Notifications: broadcast, then unicast to the listener
  dbus-send --session --type=signal /org/freedesktop/Notifications org.freedesktop.Notifications.ActionInvoked uint32:1 string:default
  dbus-send --session --type=signal --dest=$U /org/freedesktop/Notifications org.freedesktop.Notifications.ActionInvoked uint32:1 string:forged-unicast
  wait $L; wait $O; cat $1.out
}
cd "$(mktemp -d)"
run open "type='signal',interface='org.freedesktop.Notifications',path='/org/freedesktop/Notifications'"
run sender "type='signal',sender='org.freedesktop.Notifications',interface='org.freedesktop.Notifications',path='/org/freedesktop/Notifications'"
kill $BUSPID
