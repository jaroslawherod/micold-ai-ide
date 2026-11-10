#!/usr/bin/env python3
"""Write a facts file for the current diff: files and changed ranges, changed Rust symbols, their
callers and coverage of the changed lines. No model, no network: git and the stdlib only.

  review-facts.py --out-dir <dir> [--base <ref>=origin/main] [--base-snapshot <tree>:<head>] [--lcov <file>]

Prints the facts file path. Exit 1 with a one-line reason on stderr when the facts cannot be made
(the unit then runs the single-agent review). Contract: specs/1804-staged-review/contracts/facts-file.md
"""
import argparse
import os
import re
import subprocess
import sys

SNAPSHOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "review-snapshot.sh")
CALLERS_SHOWN = 8  # callers listed per symbol
CALLERS_LINES = 300  # callers lines in the whole file, so the facts stay under 400 lines


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


ITEM = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:default\s+)?(?:(?:const|async|unsafe|extern(?:\s+\"[^\"]*\")?)\s+)*"
    r"(fn|struct|enum|trait|const|static|type|impl|union)\b(.*)$"
)


def strip_code(line, in_block):
    """Drop comments and string contents from a line; returns (code, still in a block comment)."""
    out, i = [], 0
    while i < len(line):
        two = line[i : i + 2]
        if in_block:
            if two == "*/":
                in_block, i = False, i + 2
            else:
                i += 1
        elif two == "//":
            break
        elif two == "/*":
            in_block, i = True, i + 2
        elif line[i] == '"':
            i += 1
            while i < len(line) and line[i] != '"':
                i += 2 if line[i] == "\\" else 1
            i += 1
            out.append('""')
        elif line[i] == "'" and (m := re.match(r"'(?:\\.[^']*|[^\\'])'", line[i:])):
            i += m.end()  # a char literal such as '{'; a lifetime ('a) does not match
        else:
            out.append(line[i])
            i += 1
    return "".join(out), in_block


def item_name(kind, rest):
    if kind == "impl":
        rest = rest.strip()
        if rest.startswith("<"):  # drop the impl's own generics
            d = 0
            for i, c in enumerate(rest):
                d += (c == "<") - (c == ">")
                if d == 0:
                    rest = rest[i + 1 :]
                    break
        rest = re.split(r"\bwhere\b|\{", rest)[0]
        if " for " in rest:
            rest = rest.split(" for ", 1)[1]
        rest = re.sub(r"<.*", "", rest.replace("&", " ").replace("dyn ", " "))
        m = re.findall(r"\w+", rest.split("::")[-1])
        return m[-1] if m else "?"
    m = re.match(r"\s*(\w+)", rest)
    return m.group(1) if m else "?"


def rust_items(text):
    """Items of one Rust file as (qualified name, kind, first line, last line), by a textual scan."""
    items, stack, pending = [], [], None
    depth = paren = 0
    in_block = False
    for no, raw in enumerate(text.splitlines(), 1):
        code, in_block = strip_code(raw, in_block)
        if pending is None:
            m = ITEM.match(code)
            if m and not (m.group(1) in ("const", "static") and re.match(r"\s*(\{|\|)", m.group(2))):
                owner = next((s for s in reversed(stack) if s["kind"] in ("impl", "trait")), None)
                name = item_name(m.group(1), m.group(2))
                if owner and m.group(1) == "fn":
                    name = f"{owner['name']}::{name}"
                pending = {"name": name, "kind": m.group(1), "start": no}
        for c in code:
            if c in "([":
                paren += 1
            elif c in ")]":
                paren -= 1
            elif c == "{":
                depth += 1
                if pending:
                    pending["depth"] = depth
                    stack.append(pending)
                    pending = None
            elif c == "}":
                if stack and stack[-1].get("depth") == depth:
                    done = stack.pop()
                    items.append((done["name"], done["kind"], done["start"], no))
                depth -= 1
            elif c == ";" and pending and paren <= 0:
                items.append((pending["name"], pending["kind"], pending["start"], no))
                pending = None
    return items


def changed_symbols(text, ranges):
    """The innermost item of every changed line, in file order."""
    items = rust_items(text)
    found = {}
    for first, last in ranges:
        for n in range(first, last + 1):
            inside = [i for i in items if i[2] <= n <= i[3]]
            if inside:
                best = min(inside, key=lambda i: i[3] - i[2])
                found[best[:3]] = best
    return sorted(found.values(), key=lambda i: i[2])


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


def callers_of(name, own_file, own_line):
    out = run("git", "grep", "-nw", "--untracked", "-e", name, "--", "*.rs", ok=(0, 1)).stdout
    hits = []
    for l in out.splitlines():
        f, n, _ = (l.split(":", 2) + ["", ""])[:3]
        if n.isdigit() and not (f == own_file and int(n) == own_line):
            hits.append(f"{f}:{n}")
    return hits


def build(args):
    top = git("rev-parse", "--show-toplevel").strip()
    os.chdir(top)
    if run("git", "symbolic-ref", "-q", "HEAD", ok=(0, 1)).returncode:
        raise Fail("detached HEAD")
    if run("git", "rev-parse", "--verify", "-q", f"{args.base}^{{commit}}", ok=(0, 1)).returncode:
        raise Fail(f"no {args.base} ref")
    snap = run(SNAPSHOT).stdout.strip()
    tree, head = snap.split(":")
    if args.base_snapshot:
        p = run(SNAPSHOT, "diff", args.base_snapshot, "-U0", "--no-renames", "--no-color", ok=(0, 2))
        if p.returncode:
            raise Fail(f"{(p.stderr.strip() or 'bad snapshot').splitlines()[0]}")
        diff, basis = p.stdout, f"snapshot {args.base_snapshot}"
    else:
        mb = git("merge-base", args.base, "HEAD").strip()
        diff = git("diff", "-U0", "--no-renames", "--no-color", mb, tree)
        basis = f"{args.base} ({mb[:12]})"
    files = changed_ranges(diff)
    rust = {p: r for p, r in files.items() if p.endswith(".rs")}
    lcov = read_lcov(args.lcov, top) if args.lcov else None

    out = [f"# Review facts {tree}:{head}", "",
           f"Base: {basis}. Made by code, no model. Symbols come from a textual scan of the Rust files "
           "and callers are approximate textual matches (`git grep -nw`); coverage is `unknown` "
           "unless lcov data names the file.", "", "## Files", ""]
    out += [f"- {p}: {fmt_ranges(r)}" for p, r in sorted(files.items())] or ["none"]

    out += ["", "## Symbols", ""]
    symbols = {}
    if not rust:
        out.append("skipped: no Rust files")
    for p in sorted(rust):
        symbols[p] = changed_symbols(git("show", f"{tree}:{p}"), rust[p])
        out += [f"- {p}: {name} ({kind}, lines {a}-{b})" for name, kind, a, b in symbols[p]] or [
            f"- {p}: none (changes outside any fn, type or const)"]

    out += ["", "## Callers", ""]
    if not rust:
        out.append("skipped: no Rust files")
    budget, seen, omitted = CALLERS_LINES, set(), 0
    for p, syms in symbols.items():
        for name, _kind, a, _b in syms:
            bare = name.split("::")[-1]
            if (p, name) in seen or bare == "?":
                continue
            seen.add((p, name))
            if budget <= CALLERS_SHOWN:
                omitted += 1
                continue
            hits = callers_of(bare, p, a)
            shown = hits[:CALLERS_SHOWN]
            trunc = len(shown) < len(hits)
            out.append(f"- {name}: {len(hits)} callers{' (truncated)' if trunc else ''}")
            out += [f"  - {h}" for h in shown]
            budget -= len(shown) + 1
    if omitted:
        out.append(f"- {omitted} more symbols: callers not looked up (line budget spent)")

    out += ["", "## Coverage", ""]
    for p, ranges in sorted(files.items()):
        if lcov is None:
            out.append(f"- {p}: unknown (no coverage data)")
        elif p not in lcov:
            out.append(f"- {p}: unknown (not in coverage data)")
        else:
            lines = [n for a, b in ranges for n in range(a, b + 1) if n in lcov[p]]
            cov = compress(n for n in lines if lcov[p][n] > 0)
            unc = compress(n for n in lines if lcov[p][n] == 0)
            out.append(f"- {p}: covered {fmt_ranges(cov)}; uncovered {fmt_ranges(unc)}")

    os.makedirs(args.out_dir, exist_ok=True)
    dest = os.path.join(args.out_dir, f"review-facts-{tree}-{head}.md")
    tmp = dest + ".tmp"
    with open(tmp, "w") as f:
        f.write("\n".join(out) + "\n")
    os.replace(tmp, dest)
    return dest


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--base", default="origin/main")
    ap.add_argument("--base-snapshot")
    ap.add_argument("--lcov")
    args = ap.parse_args()
    try:
        print(build(args))
    except (Fail, OSError) as e:
        print(f"review-facts: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
