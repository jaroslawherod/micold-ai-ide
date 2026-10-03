#!/bin/bash
P=$(cd "$(dirname "$0")" && pwd); m=$1; name=${2:-$1}
$P/run.sh $name $m key:42 sleep:2.5 "wait:$P/logs/$name/A.log,Notify -> id" sleep:1.5 "shot:$P/logs/$name/banner.png" move:640,90 sleep:0.4 "shot:$P/logs/$name/hover.png" press sleep:3 "shot:$P/logs/$name/after.png" sleep:0.5 2>&1 | grep -v "drive\] \(sleep\|wait\)\|start mode\|session bus\|screenshot"
