#!/usr/bin/env python3
"""Check that the lines a change touched are run by the tests (#1931).

  diff-coverage.py --lcov <file> [--base <ref>=origin/main] [--head <ref>=HEAD] [--lang rust]
                   [--config scripts/guardrails-scope.conf] [--threshold N] [--summary <file>]
  diff-coverage.py --print-globs <lang> [--config <file>]

Takes the lines added or changed between the merge base of --base and --head, keeps the files that
match the --lang glob list of the config, and compares the executable ones (lines the lcov file
has a DA record for) with their hit counts. Any lcov file works, whatever produced it. Exit 0 when
the covered share reaches the threshold (or nothing executable changed), 1 when it does not, 2 on
bad input. The markdown report goes to stdout and is appended to --summary, and to
$GITHUB_STEP_SUMMARY when that is set.
"""
import argparse
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "autopilot"))
from lcov_diff import Fail, changed_ranges, compress, fmt_ranges, git, read_lcov  # noqa: E402

DEFAULT_CONFIG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "guardrails-scope.conf")


def read_config(path):
    """(settings, {lang: [glob]}) from the config file; `#` starts a comment."""
    settings, langs, section = {}, {}, None
    with open(path) as f:
        for raw in f:
            line = raw.split("#", 1)[0].strip()
            if not line:
                continue
            if line.startswith("[") and line.endswith("]"):
                section = line[1:-1].strip()
                if section != "settings":
                    langs.setdefault(section, [])
            elif section == "settings":
                key, _, value = line.partition("=")
                settings[key.strip()] = value.strip()
            elif section:
                langs[section].append(line)
            else:
                raise Fail(f"{path}: line outside any [section]: {line}")
    return settings, langs


def glob_re(glob):
    out, i = [], 0
    while i < len(glob):
        if glob.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif glob[i] == "*":
            out.append("[^/]*")
            i += 1
        elif glob[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(glob[i]))
            i += 1
    return re.compile("".join(out) + r"\Z")


def report(rows, covered, total, threshold, ok):
    pct = 100.0 * covered / total if total else 100.0
    out = ["### diff coverage", ""]
    if not total:
        out.append("No changed executable line in scope: nothing to check.")
        return "\n".join(out) + "\n"
    out.append(f"{'PASS' if ok else 'FAIL'}: {covered}/{total} changed executable lines covered "
               f"({pct:.1f}%, threshold {threshold:g}%).")
    uncovered = [(p, u) for p, _c, _t, u in rows if u]
    if uncovered:
        out += ["", "| file | uncovered changed lines |", "| --- | --- |"]
        out += [f"| {p} | {fmt_ranges(compress(u))} |" for p, u in uncovered]
    return "\n".join(out) + "\n"


def check(args):
    settings, langs = read_config(args.config)
    if args.print_globs:
        if args.print_globs not in langs:
            raise Fail(f"no [{args.print_globs}] section in {args.config}")
        print("\n".join(langs[args.print_globs]))
        return 0
    if not args.lcov:
        raise Fail("--lcov is required")
    if args.lang not in langs:
        raise Fail(f"no [{args.lang}] section in {args.config}")
    threshold = args.threshold if args.threshold is not None else float(settings.get("threshold", 80))
    scope = [glob_re(g) for g in langs[args.lang]]

    top = git("rev-parse", "--show-toplevel").strip()
    os.chdir(top)
    lcov = read_lcov(args.lcov, top)
    mb = git("merge-base", args.base, args.head).strip()
    changed = changed_ranges(git("diff", "-U0", "--no-renames", "--no-color", mb, args.head))

    rows, covered, total = [], 0, 0
    for path, ranges in sorted(changed.items()):
        if not any(r.match(path) for r in scope) or path not in lcov:
            continue
        lines = sorted({n for a, b in ranges for n in range(a, b + 1) if n in lcov[path]})
        hit = [n for n in lines if lcov[path][n] > 0]
        rows.append((path, len(hit), len(lines), [n for n in lines if lcov[path][n] == 0]))
        covered, total = covered + len(hit), total + len(lines)
    ok = total == 0 or 100.0 * covered / total >= threshold
    text = report(rows, covered, total, threshold, ok)
    sys.stdout.write(text)
    for dest in (args.summary, os.environ.get("GITHUB_STEP_SUMMARY")):
        if dest:
            with open(dest, "a") as f:
                f.write(text)
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--lcov")
    ap.add_argument("--base", default="origin/main")
    ap.add_argument("--head", default="HEAD")
    ap.add_argument("--lang", default="rust")
    ap.add_argument("--config", default=DEFAULT_CONFIG)
    ap.add_argument("--threshold", type=float)
    ap.add_argument("--summary")
    ap.add_argument("--print-globs", metavar="LANG")
    args = ap.parse_args()
    try:
        return check(args)
    except (Fail, OSError, ValueError) as e:
        print(f"diff-coverage: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
