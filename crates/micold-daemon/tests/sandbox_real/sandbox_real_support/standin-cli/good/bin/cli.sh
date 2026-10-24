#!/bin/sh
# The stand-in AI CLI: the same script `fake_ai_cli` (../mod.rs) writes as `claude`. Keep the two
# in step (FAKE_READY, FAKE_ECHO).
echo "FAKE-AI-READY $*"
while IFS= read -r line; do
  printf '%s%s\n' "FAKE-AI-ECHO:" "$line"
done
