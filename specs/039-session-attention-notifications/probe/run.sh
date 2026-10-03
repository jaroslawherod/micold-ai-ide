#!/bin/bash
# usage: run.sh <name> <mode real|fake|none> <drive steps after both windows are up...>
set -u
. "$(dirname "$0")/env.sh"
name=$1; mode=$2; shift 2
BIN=/home/jaro/workspaces/micold-ai-ide/target-shared/debug/t091-probe
L=$P/logs/$name; mkdir -p $L
$P/start.sh > $L/start.log 2>&1 || { cat $L/start.log; exit 1; }
"${PENV[@]}" setsid gjs -m /usr/share/gnome-shell/org.gnome.Shell.Notifications > $L/notif-helper.log 2>&1 &
echo $! >> $P/pids
sleep 5
CENV=("${PENV[@]}" WAYLAND_DISPLAY=$PWL ICED_BACKEND=tiny-skia WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1)
check_env WAYLAND_DISPLAY=$PWL > $L/env.log || { cat $L/env.log; $P/stop.sh; exit 1; }
"${PENV[@]}" python3 $P/drive.py move:640,700 sleep:1.5 esc sleep:1.5 > $L/drive.log 2>&1
"${CENV[@]}" T091_DELAY=9 setsid $BIN A $mode > $L/A.log 2>&1 &
echo $! >> $P/pids
sleep 3
"${CENV[@]}" setsid $BIN B > $L/B.log 2>&1 &
echo $! >> $P/pids
"${PENV[@]}" python3 $P/drive.py "$@" >> $L/drive.log 2>&1
cp $P/logs/shell.log $L/shell.log
$P/stop.sh > $L/stop.log 2>&1
sort -s -k1,1 $L/A.log $L/B.log $L/drive.log | grep -a "^[0-9]\{10\}\." > $L/merged.log
cat $L/merged.log; echo "--- stop:"; cat $L/stop.log
