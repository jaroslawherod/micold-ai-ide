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


def item_pattern(ident):
    # "- **FR-001**: …", "- [X] T004 …", "- [ ] T004 …"
    return re.compile(r"^\s*- (?:\[[ xX]\] )?(?:\*\*)?" + re.escape(ident) + r"\b")


def items(lines, idents):
    """Each bullet whose ID is in `idents`, with the more-indented or wrapped lines under it."""
    pats = {i: item_pattern(i) for i in idents}
    found, out = set(), []
    i = 0
    while i < len(lines):
        hit = next((k for k, p in pats.items() if p.match(lines[i])), None)
        if hit is None:
            i += 1
            continue
        found.add(hit)
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
    return out, [k for k in idents if k not in found]


def expand_ranges(text):
    """'T001–T012, T082–T086, T090' -> ['T001', …]."""
    ids = []
    for a, b in re.findall(r"\b(T\d+)(?:\s*[–—-]\s*(T\d+))?", text):
        if b:
            width = len(a) - 1
            ids += [f"T{n:0{width}d}" for n in range(int(a[1:]), int(b[1:]) + 1)]
        else:
            ids.append(a)
    return list(dict.fromkeys(ids))


def field(block, name):
    for line in block:
        m = re.match(r"^- \*\*" + re.escape(name) + r"\*\*:\s*(.*)$", line)
        if m:
            return m.group(1)
    return ""


def milestone(feature, mid):
    feature = Path(feature)
    tasks = lines_of(feature / "tasks.md")
    ms = section(tasks, "Milestones")
    block = section(ms, mid + " ") if ms else None
    if not block:
        sys.exit(f"brief: no milestone {mid} under '## Milestones' in {feature / 'tasks.md'}")
    print("\n".join(block))

    task_ids = expand_ranges(field(block, "Tasks"))
    task_lines, missing = items(tasks, task_ids)
    print(f"\n## Tasks of {mid} ({len(task_ids) - len(missing)} of {len(task_ids)})\n")
    print("\n".join(task_lines))
    if missing:
        print(f"\nNot found in tasks.md: {', '.join(missing)}")

    spec = lines_of(feature / "spec.md")
    req_ids = list(dict.fromkeys(re.findall(r"\b(?:FR|SC|NFR)-\d+\b", field(block, "Satisfies"))))
    if req_ids:
        req_lines, missing = items(spec, req_ids)
        print(f"\n## Requirements {mid} satisfies\n")
        print("\n".join(req_lines))
        if missing:
            print(f"\nNot found in spec.md: {', '.join(missing)}")

    stories = sorted({int(n) for n in re.findall(r"\[US(\d+)\]", "\n".join(task_lines))})
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
        out, missing = items(lines_of(target), rest)
        print("\n".join(out))
        if missing:
            sys.exit(f"brief: not found in {target}: {', '.join(missing)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
