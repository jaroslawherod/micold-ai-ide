#!/usr/bin/env python3
"""Print only the part of a Spec Kit artifact a step needs, instead of reading the whole file.

Usage:
  brief.py milestone <feature-dir> <M-id>   the milestone's block, its tasks, the FR/SC items it
                                            satisfies, and its user stories' acceptance scenarios
  brief.py section <file> <heading-text>    one markdown section (first heading containing the text),
                                            through its subsections
  brief.py items <file> <ID>...             bullet items by ID (FR-001, SC-002, T014), with their
                                            continuation lines

Exit 1 with a message on stderr when something asked for is not found.
"""

import re
import sys
from pathlib import Path

HEADING = re.compile(r"^(#{1,6})\s+(.*)$")


def lines_of(path):
    return Path(path).read_text(encoding="utf-8").splitlines()


def section(lines, text):
    """Lines of the first heading containing `text`, up to the next heading of the same or a higher
    level."""
    for i, line in enumerate(lines):
        m = HEADING.match(line)
        if m and text.lower() in m.group(2).lower():
            level = len(m.group(1))
            out = [line]
            for rest in lines[i + 1:]:
                n = HEADING.match(rest)
                if n and len(n.group(1)) <= level:
                    break
                out.append(rest)
            while out and not out[-1].strip():
                out.pop()
            return out
    return None


def item_pattern(ident, task=False):
    # A task is only a checkbox line ("- [X] T004 …"): other bullets name tasks too ("- T004–T007
    # together"). A requirement is "- **FR-001**: …".
    if task:
        return re.compile(r"^\s*- \[[ xX]\] " + re.escape(ident) + r"(?![\w])")
    return re.compile(r"^\s*- (?:\[[ xX]\] )?(?:\*\*)?" + re.escape(ident) + r"(?![\w])")


def items(lines, idents, task=False):
    """Each bullet whose ID is in `idents`, with the more-indented or wrapped lines under it.
    Returns (lines, missing IDs, IDs matched more than once)."""
    pats = {i: item_pattern(i, task) for i in idents}
    found, out = [], []
    i = 0
    while i < len(lines):
        hit = next((k for k, p in pats.items() if p.match(lines[i])), None)
        if hit is None:
            i += 1
            continue
        found.append(hit)
        indent = len(lines[i]) - len(lines[i].lstrip())
        out.append(lines[i])
        i += 1
        while i < len(lines):
            nxt = lines[i]
            if not nxt.strip() or HEADING.match(nxt):
                break
            ind = len(nxt) - len(nxt.lstrip())
            if ind <= indent and nxt.lstrip().startswith("- "):
                break
            out.append(nxt)
            i += 1
    dup = sorted({k for k in found if found.count(k) > 1})
    return out, [k for k in idents if k not in found], dup


def expand_ranges(text, prefix):
    """expand_ranges('T001–T003, T090', 'T') -> [T001, T002, T003, T090];
    expand_ranges('FR-018a–FR-020', 'FR-') -> [FR-018a, FR-019, FR-020].
    The separator may be an en dash, an em dash or a hyphen; the end may drop the prefix
    ('SC-001–007'). A descending range keeps just its two ends."""
    ids = []
    p = re.escape(prefix)
    pat = (r"(?<![\w])" + p + r"(\d+)([a-z]?)(?![\w])"
           r"(?:\s*[-–—]\s*(?:" + p + r")?(\d+)([a-z]?)(?![\w]))?")
    for a, sa, b, sb in re.findall(pat, text):
        w = len(a)
        ids.append(f"{prefix}{a}{sa}")
        if b:
            lo, hi = int(a), int(b)
            ids += [f"{prefix}{n:0{w}d}" for n in range(lo + 1, hi)]
            ids.append(f"{prefix}{int(b):0{w}d}{sb}")
    return list(dict.fromkeys(ids))


def field(block, name):
    """A `- **Name**: …` field, with its wrapped continuation lines joined."""
    for i, line in enumerate(block):
        m = re.match(r"^- \*\*" + re.escape(name) + r"\*\*:\s*(.*)$", line)
        if m:
            parts = [m.group(1)]
            for rest in block[i + 1:]:
                if not rest.strip() or rest.lstrip().startswith("- **") or HEADING.match(rest):
                    break
                parts.append(rest.strip())
            return " ".join(parts)
    return ""


def milestone(feature, mid):
    feature = Path(feature)
    tasks = lines_of(feature / "tasks.md")
    ms = section(tasks, "Milestones")
    head = next((ln for ln in ms or [] if re.match(r"^#{2,6} " + re.escape(mid) + r"\b(?!\d)", ln)), None)
    block = section(ms, head.lstrip("# ")) if head else None
    if not block:
        sys.exit(f"brief: no milestone {mid} under '## Milestones' in {feature / 'tasks.md'}")
    print("\n".join(block))

    others = [ln for ln in ms if re.match(r"^#{2,6} M\d+", ln) and ln != head]
    if others:
        print("\nOther milestones (their tasks are out of scope here):")
        for head in others:
            sub = section(ms, head.lstrip("# "))
            print(f"- {head.lstrip('# ')}: {field(sub, 'Tasks') or '?'}")

    task_ids = expand_ranges(field(block, "Tasks"), "T")
    task_lines, missing, dup = items(tasks, task_ids, task=True)
    print(f"\n## Tasks of {mid} ({len(task_ids) - len(missing)} of {len(task_ids)})\n")
    print("\n".join(task_lines))
    if missing:
        print(f"\nNot found in tasks.md: {', '.join(missing)}")
    if dup:
        print(f"\nIn tasks.md more than once (check which belongs here): {', '.join(dup)}")

    spec = lines_of(feature / "spec.md")
    satisfies = field(block, "Satisfies")
    req_ids = [i for p in ("FR-", "SC-", "NFR-") for i in expand_ranges(satisfies, p)]
    if req_ids:
        req_lines, missing, _ = items(spec, req_ids)
        print(f"\n## Requirements {mid} satisfies\n")
        print("\n".join(req_lines))
        if missing:
            print(f"\nNot found in spec.md: {', '.join(missing)}")

    stories = sorted({int(n) for n in re.findall(r"\[US(\d+)\]", "\n".join(task_lines))}
                     | {int(n) for n in re.findall(r"\bUS ?(\d+)\b", satisfies)})
    for n in stories:
        story = section(spec, f"User Story {n} ")
        if story:
            print()
            print("\n".join(story))


def main(argv):
    if len(argv) < 3 or argv[0] not in ("milestone", "section", "items"):
        print(__doc__)
        return 2
    cmd, target, rest = argv[0], argv[1], argv[2:]
    if cmd == "milestone":
        milestone(target, rest[0])
    elif cmd == "section":
        out = section(lines_of(target), " ".join(rest))
        if out is None:
            sys.exit(f"brief: no heading containing {' '.join(rest)!r} in {target}")
        print("\n".join(out))
    else:
        out, missing, _ = items(lines_of(target), rest)
        print("\n".join(out))
        if missing:
            sys.exit(f"brief: not found in {target}: {', '.join(missing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
