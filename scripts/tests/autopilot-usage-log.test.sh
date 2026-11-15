#!/usr/bin/env bash
# scripts/autopilot/usage-log.py: header once, one row per call, CSV quoting, missing fields empty.
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
fail=0
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: want [$3] got [$2]"; fail=1; fi; }
export USAGE_LOG_CSV="$tmp/sub/threads.csv"
S="$ROOT/scripts/autopilot/usage-log.py"

echo '{"ccr":{"id":"session_A","title":"Fix, \"quoted\" thing","created_at":"c1","updated_at":"u1","configured_model":"claude-opus-5-5[1m]","parent_session_id":"session_P","session_context":{"effort_level":"medium"},"external_metadata":{"last_served_model":"claude-opus-5-5","context_usage":{"used_tokens":120},"usage":{"cost_usd":1.5,"cache_read_tokens":10,"cache_write_tokens":20,"input_tokens":3,"output_tokens":4}}}}' | "$S"
echo '{"id":"session_B","title":"bare"}' | "$S"
ok "exit" "$?" "0"

ok "lines" "$(wc -l <"$USAGE_LOG_CSV")" "3"
ok "header" "$(head -1 "$USAGE_LOG_CSV")" "id,title,created_at,updated_at,model,last_served_model,effort_level,cost_usd,cache_read_tokens,cache_write_tokens,input_tokens,output_tokens,context_used_tokens,claude_reply_count,parent_session_id,recent_action"
got=$(python3 -I -c 'import csv,sys; r=list(csv.reader(open(sys.argv[1]))); print("|".join(r[1])); print("|".join(r[2]))' "$USAGE_LOG_CSV")
ok "full row" "$(sed -n 1p <<<"$got")" 'session_A|Fix, "quoted" thing|c1|u1|claude-opus-5-5[1m]|claude-opus-5-5|medium|1.5|10|20|3|4|120||session_P|'
ok "sparse row" "$(sed -n 2p <<<"$got")" 'session_B|bare||||||||||||||'
echo 'nope' | "$S" 2>/dev/null; ok "bad json fails" "$?" "1"
ok "no extra row" "$(wc -l <"$USAGE_LOG_CSV")" "3"
exit $fail
