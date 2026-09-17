#!/usr/bin/env python3
"""Consistency checker for the base audit (the loop's referee).

Exit 0 iff every report bullet is [x] or [~] (no [ ] open, no [>] partial).
Prints the offending bullets so they can be fed back as work items.

Usage:
  python3 .spec/base-audit/loop/check.py            # marker check only
  python3 .spec/base-audit/loop/check.py --test     # also run cargo test
  python3 .spec/base-audit/loop/check.py --full     # also run autoplay smoke
"""
import glob
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
AUDIT = os.path.join(ROOT, ".spec", "base-audit")
SKIP = {
    os.path.join(AUDIT, "inventory", "README.md"),
    os.path.join(AUDIT, "enums", "04-data-counts.md"),
    os.path.join(AUDIT, "enums", "05-coverage.md"),
    os.path.join(AUDIT, "MASTER.md"),
    os.path.join(AUDIT, "README.md"),
    os.path.join(AUDIT, "AUDIT_PROTOCOL.md"),
}
CHECKLISTS = []
for d in ("reports", "inventory", "enums"):
    CHECKLISTS += sorted(glob.glob(os.path.join(AUDIT, d, "*.md")))
CHECKLISTS = [p for p in CHECKLISTS if p not in SKIP]

MARK = re.compile(r"^- \[( |x|~|>)\]")
BAD = (" ", ">")


def collect():
    open_items, partial_items = [], []
    counts = {"x": 0, "~": 0, " ": 0, ">": 0}
    for path in CHECKLISTS:
        lines = open(path).read().splitlines()
        for i, l in enumerate(lines):
            m = MARK.match(l)
            if not m:
                continue
            counts[m.group(1)] += 1
            rel = os.path.relpath(path, AUDIT)
            if m.group(1) == " ":
                open_items.append((rel, i + 1, l))
            elif m.group(1) == ">":
                partial_items.append((rel, i + 1, l))
    return counts, open_items, partial_items


def run(cmd, cwd, timeout=1200):
    p = subprocess.run(cmd, cwd=cwd, shell=True, capture_output=True, text=True, timeout=timeout)
    return p.returncode, (p.stdout + p.stderr)


def main():
    args = sys.argv[1:]
    counts, open_items, partial_items = collect()
    print("=== base audit marker check ===")
    print(f"[x]={counts['x']}  [>]={counts['>']}  [ ]={counts[' ']}  [~]={counts['~']}")
    for title, items in (("OPEN", open_items), ("PARTIAL", partial_items)):
        if items:
            print(f"\n-- {title} ({len(items)}) --")
            for f, ln, text in items:
                print(f"  {f}:{ln}: {text[:200]}")
    ok = not open_items and not partial_items

    if "--test" in args or "--full" in args:
        print("\n=== cargo test ===")
        code, out = run("cargo test 2>&1 | tail -3", os.path.join(ROOT, "bevy"))
        print(out.strip())
        ok = ok and code == 0 and "test result: ok" in out

    if "--full" in args:
        print("\n=== autoplay smoke ===")
        env = "TOME_AUTOPLAY=/tmp/opencode/loop_smoke.png"
        code, out = run(f"{env} timeout 240 cargo run 2>&1 | grep -icE 'panic|Encountered'", os.path.join(ROOT, "bevy"))
        panics = out.strip()
        print(f"panic lines: {panics}")
        ok = ok and panics == "0"

    print("\n=== VERDICT:", "CONSISTENT" if ok else "NOT CONSISTENT", "===")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
