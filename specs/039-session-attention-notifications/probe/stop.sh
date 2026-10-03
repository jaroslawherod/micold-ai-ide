#!/bin/bash
. "$(dirname "$0")/env.sh"
# kill only recorded pids (each is a setsid session leader we started) and their sessions
for p in $(tac $P/pids 2>/dev/null); do
  if [ -d /proc/$p ]; then kill -TERM -- -$p 2>/dev/null || kill -TERM $p; fi
done
sleep 2
for p in $(cat $P/pids 2>/dev/null); do [ -d /proc/$p ] && kill -KILL -- -$p 2>/dev/null; done
sleep 1
for p in $(cat $P/pids 2>/dev/null); do [ -d /proc/$p ] && echo "STILL ALIVE $p"; done
rm -rf $RT; : > $P/pids
echo "left with private runtime dir in env:"; grep -l "t091-probe" /proc/[0-9]*/environ 2>/dev/null | head
