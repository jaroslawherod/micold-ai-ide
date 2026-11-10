#!/usr/bin/env bash
# scripts/autopilot/eval.py: ledger metrics, gh-derived CI-red count, baseline delta, `changed`.
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
fail=0
ok() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: want [$3] got [$2]"; fail=1; fi; }

cat > "$tmp/gh" <<'STUB'
#!/usr/bin/env bash
case "$1 $2" in
  "pr view") echo '{"h":"feat/x"}' ;;
  "run list") echo "$GH_RUNS" ;;
esac
STUB
chmod +x "$tmp/gh"
cat > "$tmp/ledger.md" <<'L'
## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A | 1 | a:b | APPROVE |
| Review B M1 | 1 | c:d | CHANGES: 2 MAJOR |
| Review B M1 | 2 | e:f | APPROVE |

## Handover
L
export GH="$tmp/gh" GH_RUNS='[{"conclusion":"failure"},{"conclusion":"success"},{"conclusion":"failure"}]'
snap=$(mktemp -d); cp -r "$ROOT/evals" "$snap/" 2>/dev/null
trap 'rm -rf "$tmp" "$snap"' EXIT
cd "$ROOT"
cp evals/autopilot/corpus.json "$tmp/corpus.bak"; cp evals/autopilot/baseline.json "$tmp/base.bak" 2>/dev/null
restore() { cp "$tmp/corpus.bak" evals/autopilot/corpus.json; cp "$tmp/base.bak" evals/autopilot/baseline.json 2>/dev/null; rm -f evals/autopilot/ledgers/pr-9999.md; rmdir evals/autopilot/ledgers 2>/dev/null; }
trap 'restore; rm -rf "$tmp" "$snap"' EXIT

echo '[]' > evals/autopilot/corpus.json
S=scripts/autopilot/eval.py
python3 $S add 9999 --ledger "$tmp/ledger.md" >/dev/null
out=$(python3 $S baseline); ok "baseline totals" "$out" "baseline written: {'review_rounds': 3.0, 'first_pass': 0.0, 'ci_red': 2.0, 'tokens': None}"
out=$(python3 $S | grep '^pr-9999' | tr -s ' '); ok "per-run row" "$out" "pr-9999 3 0 2 -"
export GH_RUNS='[{"conclusion":"failure"},{"conclusion":"failure"},{"conclusion":"failure"}]'
out=$(python3 $S); rc=$?; ok "worse exits 1" "$rc" "1"
ok "delta printed" "$(echo "$out" | grep -c 'WORSE')" "1"
export GH_RUNS='[]'
python3 $S >/dev/null; ok "better exits 0" "$?" "0"
out=$(python3 $S changed HEAD); ok "changed: none" "$out" "no agent-shaping file differs from HEAD"
exit $fail
