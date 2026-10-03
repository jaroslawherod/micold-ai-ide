#!/bin/bash
# starts private buses + headless gnome-shell; records pids in $P/pids
set -u
. "$(dirname "$0")/env.sh"
[ -e $RT ] && { echo "ABORT: $RT exists (previous run not cleaned)"; exit 1; }
mkdir -m 700 $RT; mkdir -p $PHOME/.config $PHOME/.local/share $PHOME/.cache $PHOME/.local/state $P/logs
: > $P/pids
for b in bus sysbus; do
cat > $P/$b.conf <<XML
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig><type>session</type><listen>unix:path=$RT/$b</listen><auth>EXTERNAL</auth>
<policy context="default"><allow send_destination="*" eavesdrop="true"/><allow eavesdrop="true"/><allow own="*"/></policy></busconfig>
XML
  "${PENV[@]}" setsid dbus-daemon --config-file=$P/$b.conf --nofork --nopidfile >$P/logs/$b.log 2>&1 &
  echo $! >> $P/pids
done
sleep 1
[ -S $RT/bus ] && [ -S $RT/sysbus ] || { echo ABORT buses not up; exit 1; }
check_env || exit 1
"${PENV[@]}" setsid gnome-shell --headless --no-x11 --wayland-display=$PWL --virtual-monitor 1280x800 --unsafe-mode >$P/logs/shell.log 2>&1 &
echo $! >> $P/pids
for i in $(seq 30); do [ -S $RT/$PWL ] && break; sleep 1; done
[ -S $RT/$PWL ] && echo "shell up: $RT/$PWL" || { echo "shell socket missing"; tail -30 $P/logs/shell.log; }
cat $P/pids
