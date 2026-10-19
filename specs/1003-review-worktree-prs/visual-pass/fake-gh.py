#!/usr/bin/env python3
# Fake gh for the 1003 visual pass: canned PRs per head branch, read from $VP_GH_FIXTURE.
import sys, json, os
a = sys.argv[1:]
if a[:2] == ["auth", "status"]:
    sys.exit(0 if os.environ.get("VP_GH_SIGNED_OUT") != "1" else 1)
if a[:1] == ["api"] and a[-1].startswith("repos/") and "/pulls/" in a[-1]:
    import subprocess
    num = int(a[-1].rsplit("/", 1)[1])
    fx = json.load(open(os.environ["VP_GH_FIXTURE"]))
    for br, prs in fx.items():
        for pr in prs:
            if pr["number"] == num:
                sys.stdout.write(subprocess.run(["git", "-C", os.path.dirname(os.environ["VP_GH_FIXTURE"]) + "/proj", "diff", "main..." + pr["headRefOid"]], capture_output=True, text=True).stdout)
                sys.exit(0)
    sys.exit(1)
if a[:2] != ["api", "graphql"]:
    sys.exit(1)
if os.path.exists(os.environ.get("VP_GH_SIGNED_OUT_FILE", "/nonexistent")):
    sys.stderr.write("To get started with GitHub CLI, please run:  gh auth login\n"); sys.exit(4)
f = {}
for i, x in enumerate(a):
    if x == "-f":
        k, _, v = a[i + 1].partition("=")
        f[k] = v
fx = json.load(open(os.environ["VP_GH_FIXTURE"]))
repo = {"defaultBranchRef": {"name": "main"}}
n = 0
while f"b{n}" in f:
    n += 1
listing = "p0:" in f["query"]
for i in range(n):
    prs = fx.get(f[f"b{i}"], [])
    if listing:
        repo[f"p{i}"] = {"nodes": prs}
    else:
        for al in ("o", "r", "oc", "mc"):
            repo[f"{al}{i}"] = {"nodes": [], "totalCount": 0}
body = {"data": {"repository": repo, "rateLimit": {"remaining": 4000, "resetAt": "2030-01-01T00:00:00Z"}}}
sys.stdout.write("HTTP/2.0 200 OK\r\nContent-Type: application/json\r\n\r\n" + json.dumps(body))
