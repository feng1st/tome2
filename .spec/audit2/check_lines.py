#!/usr/bin/env python3
"""Validate method evidence for one original source file (agent helper).

Usage: python3 .spec/audit2/check_lines.py util.cc
Applies the exact rules of audit.py::check_methods to the lines of that file
and prints the failures (and counts).  Exits 1 if any line is incomplete.
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BEVY = os.path.join(ROOT, "bevy", "src")
OUT = os.path.join(ROOT, ".spec", "audit2", "checklist")


def bevy_text():
    return "\n".join(open(os.path.join(BEVY, f), encoding="utf-8").read()
                     for f in sorted(os.listdir(BEVY)) if f.endswith(".rs"))


def rust_tests(text):
    names = set()
    for m in re.finditer(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([A-Za-z_]\w*)", text):
        names.add(m.group(1))
    return names


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    want = sys.argv[1]
    text = bevy_text()
    tests = rust_tests(text)
    total = done = 0
    bad = []
    for l in open(os.path.join(OUT, "methods.md"), encoding="utf-8").read().splitlines():
        if not l.startswith("- ["):
            continue
        m = re.match(r"- \[( |x)\] `([^`]+):(\d+)` \*\*([^*]+)\*\*", l)
        if not m:
            continue
        src, ln, name = m.group(2), m.group(3), m.group(4)
        if src != want:
            continue
        total += 1
        if m.group(1) != "x":
            bad.append(f"OPEN {src}:{ln} {name}")
            continue
        impl = re.search(r"impl:(bevy/src/([A-Za-z_]+\.rs)):(\d+)", l)
        test = re.search(r"test:([A-Za-z_][\w:]*::[A-Za-z_]\w*)", l)
        if not impl or not test:
            bad.append(f"NO-EVIDENCE {src}:{ln} {name}")
            continue
        p = os.path.join(ROOT, impl.group(1))
        if not os.path.exists(p) or int(impl.group(3)) > len(open(p).read().splitlines()):
            bad.append(f"BAD-IMPL {src}:{ln} {name} -> {impl.group(1)}:{impl.group(3)}")
            continue
        if test.group(1).split("::")[-1] not in tests:
            bad.append(f"BAD-TEST {src}:{ln} {name} -> {test.group(1)}")
            continue
        done += 1
    print(f"{want}: {done}/{total} complete")
    for b in bad:
        print("  -", b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
