#!/usr/bin/env bash
# Track the GitHub issue an autopilot run started from.
#
# Usage: issue.sh start <n> <branch>   label it in-progress, assign @me, comment the branch
#        issue.sh done <n> <pr>...     remove the label, close it as completed citing the PRs
#
# start prints ISSUE_STARTED #<n>, or ISSUE_TAKEN #<n> (exit 1) when the issue is already labelled
# in-progress or assigned to someone else: that is another flow's work.
set -uo pipefail
cmd=${1:-} n=${2:-}
[ -n "$cmd" ] && [ -n "$n" ] || { echo "usage: $0 start <n> <branch> | done <n> <pr>..." >&2; exit 2; }
shift 2
case $cmd in
  start)
    [ $# -eq 1 ] || { echo "usage: $0 start <n> <branch>" >&2; exit 2; }
    me=$(gh api user -q .login) || exit 2
    taken=$(gh issue view "$n" --json labels,assignees \
      -q "([.labels[].name] | index(\"in-progress\")) != null or ([.assignees[].login] | map(select(. != \"$me\")) | length) > 0") || exit 2
    [ "$taken" = false ] || { echo "ISSUE_TAKEN #$n"; exit 1; }
    gh label create in-progress --color FBCA04 --description "Being worked on" >/dev/null 2>&1 || true
    gh issue edit "$n" --add-label in-progress --add-assignee @me >/dev/null || exit 2
    gh issue comment "$n" --body "🤖 autopilot: started on branch \`$1\`." >/dev/null || exit 2
    echo "ISSUE_STARTED #$n" ;;
  done)
    [ $# -ge 1 ] || { echo "usage: $0 done <n> <pr>..." >&2; exit 2; }
    prs=$(printf '#%s, ' "$@"); prs=${prs%, }
    gh issue edit "$n" --remove-label in-progress >/dev/null || exit 2
    gh issue close "$n" --reason completed --comment "🤖 autopilot: merged to main in $prs." >/dev/null || exit 2
    echo "ISSUE_CLOSED #$n" ;;
  *) echo "usage: $0 start <n> <branch> | done <n> <pr>..." >&2; exit 2 ;;
esac
