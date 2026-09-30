#!/usr/bin/env bash
# PreToolUse(Bash) hook: enforces speckit-autopilot's hard rules in code, not just in prose.
#
# A no-op unless the command's directory is on a branch that an autopilot ledger names
# (specs/*/autopilot.md or specs/*/bugs/*.autopilot.md). There it blocks, with exit 2 and the
# reason on stderr (Claude Code shows it to the agent):
#   gh pr merge --delete-branch or --admin     the IDE owns cleanup; --admin bypasses the gate
#   git worktree remove                        the user removes the worktree
#   gh pr merge|close|review|comment|edit|ready|reopen <n>, wait-merge.sh <n>
#                                              when #<n> is not this flow's: not in the ledger and
#                                              not from this branch
#   gh api writes that merge a PR              use wait-merge.sh
#   git push                                   when the branch changes code (anything outside
#                                              docs/, specs/, .claude/ and *.md) and no green
#                                              `mise run gate` saw HEAD's code
# `mise run gate` records the tree it passed on in $(git rev-parse --git-path autopilot-gate-ok).
set -uo pipefail
in=$(cat)
cmd=$(jq -r '.tool_input.command // empty' <<<"$in")
[[ $cmd =~ (gh[[:space:]]+(pr|api)|wait-merge\.sh|git[[:space:]].*(push|worktree)) ]] || exit 0

cd "$(jq -r '.cwd // "."' <<<"$in")" 2>/dev/null || exit 0
re_gitdir='git[[:space:]]+-C[[:space:]]+([^[:space:]]+)'
if [[ $cmd =~ $re_gitdir ]]; then cd "${BASH_REMATCH[1]}" 2>/dev/null || exit 0; fi
top=$(git rev-parse --show-toplevel 2>/dev/null) && cd "$top" || exit 0
b=$(git branch --show-current 2>/dev/null)
[ -n "$b" ] || exit 0
ledger=$(grep -lxF -- "- **Worktree branch**: $b" specs/*/autopilot.md specs/*/bugs/*.autopilot.md 2>/dev/null | head -1)
[ -n "$ledger" ] || exit 0

deny() { printf 'autopilot gate: %s\n' "$*" >&2; exit 2; }
exempt='^(docs/|specs/|\.claude/)|\.md$'

if [[ $cmd =~ gh[[:space:]]+pr[[:space:]]+merge([^\;\&\|]*) ]]; then
  args=${BASH_REMATCH[1]}
  [[ $args =~ --delete-branch|(^|[[:space:]])-d([[:space:]]|$) ]] &&
    deny "never pass --delete-branch: the user removes the worktree in micold IDE, which cleans up the branch."
  [[ $args =~ --admin ]] && deny "never merge with --admin: merge on green through scripts/autopilot/wait-merge.sh."
fi
[[ $cmd =~ git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+worktree[[:space:]]+remove ]] &&
  deny "never remove a worktree: the user removes it in micold IDE."
if [[ $cmd =~ gh[[:space:]]+api ]]; then
  [[ $cmd =~ (mergePullRequest|/pulls/[0-9]+/merge) ]] && deny "do not merge through gh api; use scripts/autopilot/wait-merge.sh."
  exit 0
fi

re_pr='(gh[[:space:]]+pr[[:space:]]+(merge|close|review|comment|edit|ready|reopen)|wait-merge\.sh)[[:space:]]+#?([0-9]+)'
if [[ $cmd =~ $re_pr ]]; then
  n=${BASH_REMATCH[3]}
  grep -qE "#$n([^0-9]|$)" "$ledger" && exit 0
  head=$(gh pr view "$n" --json headRefName -q .headRefName 2>/dev/null) || deny "cannot read #$n to check it is this flow's PR."
  [ "$head" = "$b" ] && exit 0
  deny "#$n is not this flow's PR: not in $ledger and not from branch $b. Never merge, review, comment on or close another flow's PR."
fi

re_push='(^|[[:space:];&|(])git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+push([[:space:]]|$)'
if [[ $cmd =~ $re_push ]]; then
  [ -n "$(git diff --name-only origin/main...HEAD 2>/dev/null | grep -vE "$exempt")" ] || exit 0
  head_tree=$(git rev-parse 'HEAD^{tree}')
  stamps=$(git rev-parse --git-path autopilot-gate-ok)
  for t in $(tail -20 "$stamps" 2>/dev/null); do
    git cat-file -e "$t^{tree}" 2>/dev/null || continue
    [ -z "$(git diff --name-only "$t" "$head_tree" | grep -vE "$exempt")" ] && exit 0
  done
  deny "HEAD's code has not passed 'mise run gate' in this worktree (only docs may change after it). Run the gate (detached, per pr-and-merge.md §2), commit, then push."
fi
exit 0
