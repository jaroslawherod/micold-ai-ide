#!/usr/bin/env bash
# Track the GitHub issue of an autopilot run, and keep its flow and effort labels true.
#
# Usage: issue.sh labels <n>                                  print the issue's labels, one line
#        issue.sh new <branch> <title> <body-file> <flow> [high|low]
#                                                            open the run's issue, labelled and claimed
#        issue.sh start <n> <branch> [<flow> [high|low]]     label it in-progress, assign @me, comment
#        issue.sh flow <n> <flow> [high|low]                 the run switched flow: move the labels
#        issue.sh done <n> <pr>...                           remove in-progress, close citing the PRs
#
# <flow> is bug, bugfix, feature or chore. The flow is written as the label flow:<flow> and the
# level as effort:<level>; an older flow:* or effort:* label that differs is removed, so the issue
# says what the run does. Without a level, effort:* labels stay as they are.
#
# start prints ISSUE_STARTED #<n>, or ISSUE_TAKEN #<n> (exit 1) when the issue is already labelled
# in-progress or assigned to someone else: that is another flow's work. new prints ISSUE_NEW #<n>.
set -uo pipefail
usage() { sed -n '4,9p' "$0" | cut -c3- >&2; exit 2; }
cmd=${1:-}
[ -n "$cmd" ] || usage
shift

check_flow() { # <flow> [level]
  case $1 in bug|bugfix|feature|chore) ;; *) echo "issue.sh: unknown flow '$1'" >&2; exit 2 ;; esac
  case ${2:-} in ""|high|low) ;; *) echo "issue.sh: unknown effort '$2'" >&2; exit 2 ;; esac
}
ensure() { # <label>...: create the labels the repo lacks
  local l
  for l in "$@"; do
    case $l in
      in-progress) gh label create "$l" --color FBCA04 --description "Being worked on" ;;
      flow:*) gh label create "$l" --color C5DEF5 --description "speckit-autopilot runs the ${l#flow:} flow" ;;
      effort:*) gh label create "$l" --color C5DEF5 --description "speckit-autopilot effort level" ;;
    esac >/dev/null 2>&1 || true
  done
}
wanted() { echo "flow:$1${2:+,effort:$2}"; } # <flow> [level]
stale() { # <n> <flow> [level]: the issue's flow:* and effort:* labels the choice replaces, comma-joined
  local keep="flow:$2" drop='^flow:'
  [ -n "${3:-}" ] && { keep="$keep|effort:$3"; drop='^(flow|effort):'; }
  local names; names=$(gh issue view "$1" --json labels -q '.labels[].name') || return 2
  echo "$names" | grep -E "$drop" | grep -vxE "$keep" | paste -sd, -; return 0
}

case $cmd in
  labels)
    [ $# -eq 1 ] || usage
    l=$(gh issue view "$1" --json labels -q '[.labels[].name] | join(", ")') || exit 2
    echo "LABELS #$1: ${l:-none}" ;;
  new)
    [ $# -ge 4 ] && [ $# -le 5 ] && [ -f "$3" ] || usage
    check_flow "$4" "${5:-}"
    ensure in-progress "flow:$4" ${5:+"effort:$5"}
    url=$(gh issue create --title "$2" --body-file "$3" --assignee @me \
      --label "in-progress,$(wanted "$4" "${5:-}")") || exit 2
    n=${url##*/}
    [[ $n =~ ^[0-9]+$ ]] || { echo "issue.sh: no issue number in '$url'" >&2; exit 2; }
    echo "ISSUE_NEW #$n" # before the comment: a retry after a failed comment would open a second issue
    gh issue comment "$n" --body "🤖 autopilot: started on branch \`$1\`." >/dev/null || true ;;
  start)
    [ $# -ge 2 ] && [ $# -le 4 ] || usage
    n=$1 branch=$2 flow=${3:-} level=${4:-}
    [ -z "$flow" ] || check_flow "$flow" "$level"
    me=$(gh api user -q .login) || exit 2
    taken=$(gh issue view "$n" --json labels,assignees \
      -q "([.labels[].name] | index(\"in-progress\")) != null or ([.assignees[].login] | map(select(. != \"$me\")) | length) > 0") || exit 2
    [ "$taken" = false ] || { echo "ISSUE_TAKEN #$n"; exit 1; }
    add=in-progress old=
    if [ -n "$flow" ]; then
      ensure "flow:$flow" ${level:+"effort:$level"}
      add="$add,$(wanted "$flow" "$level")"; old=$(stale "$n" "$flow" "$level") || exit 2
    fi
    ensure in-progress
    gh issue edit "$n" --add-label "$add" ${old:+--remove-label "$old"} --add-assignee @me >/dev/null || exit 2
    gh issue comment "$n" --body "🤖 autopilot: started on branch \`$branch\`." >/dev/null || exit 2
    echo "ISSUE_STARTED #$n" ;;
  flow)
    [ $# -ge 2 ] && [ $# -le 3 ] || usage
    check_flow "$2" "${3:-}"
    ensure "flow:$2" ${3:+"effort:$3"}
    old=$(stale "$1" "$2" "${3:-}") || exit 2
    gh issue edit "$1" --add-label "$(wanted "$2" "${3:-}")" ${old:+--remove-label "$old"} >/dev/null || exit 2
    echo "ISSUE_FLOW #$1 $2${3:+ $3}" ;;
  done)
    [ $# -ge 2 ] || usage
    n=$1; shift
    prs=$(printf '#%s, ' "$@"); prs=${prs%, }
    gh issue edit "$n" --remove-label in-progress >/dev/null || exit 2
    gh issue close "$n" --reason completed --comment "🤖 autopilot: merged to main in $prs." >/dev/null || exit 2
    echo "ISSUE_CLOSED #$n" ;;
  *) usage ;;
esac
