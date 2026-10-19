#!/bin/bash
# seed.sh <root>: a git repo with branches a,b,c created in it, a second worktree, projects.json, fixture
set -e
R=$1; rm -rf $R/proj $R/other; mkdir -p $R/data/micold-ai-ide; cd $R
git init -q -b main proj; cd proj
git config user.email v@p.x; git config user.name vp
git remote add origin https://github.com/acme/widgets.git
echo 0 > f.txt; git add .; git commit -qm init
git update-ref refs/remotes/origin/main HEAD
git worktree add -q .claude/worktrees/wt1 -b a main
cd .claude/worktrees/wt1
for b in a b c; do [ $b = a ] || git switch -qc $b; echo $b > $b.txt; git add .; git commit -qm "work on $b"; done
cd ../../..
git worktree add -q .claude/worktrees/wt2 -b d main
P=$(pwd -P)
cat > $R/data/projects.json <<J
{"schema_version":1,"last_active":"$P","projects":[{"path":"$P","display_name":"widgets","is_git_repo":true}]}
J
pr() { # number title state created branch checks
cat <<J
{"number":$1,"title":"$2","url":"https://github.com/acme/widgets/pull/$1","state":"$3","isDraft":false,"createdAt":"$4","isCrossRepository":false,"baseRefName":"main","headRefName":"$5","headRefOid":"$(git rev-parse $5)","commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"checkRunCountsByState":[{"state":"$6","count":1}],"statusContextCountsByState":[]}}}}]}}
J
}
cat > $R/fixture.json <<J
{"a":[$(pr 41 "Add widget A" MERGED 2026-10-10T10:00:00Z a SUCCESS)],
 "b":[$(pr 42 "Rework widget B" CLOSED 2026-10-11T10:00:00Z b FAILURE)],
 "c":[$(pr 43 "Widget C polish" OPEN 2026-10-12T10:00:00Z c IN_PROGRESS)]}
J
echo seeded $P
