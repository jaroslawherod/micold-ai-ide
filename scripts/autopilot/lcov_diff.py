"""Diff and lcov parsing shared by review-facts.py and scripts/diff-coverage.py (#1931).

Git and the stdlib only. Importing this module pins git's path-printing config for every git call.
"""
import os
import re
import subprocess


class Fail(Exception):
    pass


# paths printed as they are, whatever the user's git config says about quoting or prefixes
os.environ.update(GIT_CONFIG_COUNT="3", GIT_CONFIG_KEY_0="core.quotepath", GIT_CONFIG_VALUE_0="false",
                  GIT_CONFIG_KEY_1="diff.noprefix", GIT_CONFIG_VALUE_1="false",
                  GIT_CONFIG_KEY_2="diff.mnemonicPrefix", GIT_CONFIG_VALUE_2="false")


def run(*cmd, ok=(0,)):
    p = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    if p.returncode not in ok:
        err = (p.stderr.strip().splitlines() or [f"exit {p.returncode}"])[0]
        raise Fail(f"{' '.join(cmd[:3])} failed: {err}")
    return p


def git(*args, ok=(0,)):
    return run("git", *args, ok=ok).stdout


def changed_ranges(diff):
    """{path: [(first, last)]} of added or changed lines on the new side; deleted files are left out."""
    files, path, header = {}, None, False
    for line in diff.splitlines():
        if line.startswith("diff --git"):
            header = True
        elif header and line.startswith("+++ "):
            path = None if line == "+++ /dev/null" else line[6:].split("\t")[0]
            if path:
                files.setdefault(path, [])
        elif line.startswith("@@") and path:
            header = False
            m = re.match(r"@@ -\S+ \+(\d+)(?:,(\d+))? @@", line)
            n = int(m.group(2)) if m.group(2) is not None else 1
            first = int(m.group(1))
            # a pure deletion (n == 0) is pinned to the line before it, so its enclosing item is found
            files[path].append((max(first, 1), first + n - 1) if n else (max(first, 1), max(first, 1)))
    return files


def fmt_ranges(ranges):
    return ", ".join(str(a) if a == b else f"{a}-{b}" for a, b in ranges) or "none"


def compress(nums):
    out, nums = [], sorted(set(nums))
    for n in nums:
        if out and n == out[-1][1] + 1:
            out[-1] = (out[-1][0], n)
        else:
            out.append((n, n))
    return out


def read_lcov(path, top):
    """{repo-relative path: {line: hits}} from an lcov file; absolute paths are made repo-relative."""
    cov, cur = {}, None
    top = os.path.realpath(top)
    with open(path, errors="replace") as f:
        for line in f:
            line = line.strip()
            if line.startswith("SF:"):
                p = line[3:]
                if os.path.isabs(p):
                    p = os.path.relpath(os.path.realpath(p), top)
                cur = cov.setdefault(os.path.normpath(p), {})
            elif line.startswith("DA:") and cur is not None:
                try:
                    n, hits = line[3:].split(",")[:2]
                    cur[int(n)] = cur.get(int(n), 0) + int(hits)
                except ValueError:
                    continue  # a malformed DA line carries no data
    return cov
