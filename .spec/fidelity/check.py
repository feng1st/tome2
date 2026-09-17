#!/usr/bin/env python3
"""Watchdog-compatible checker for the fidelity review backlog."""
import re, sys
p = '/home/feng/Projects/game/tome2/.spec/fidelity/fidelity.md'
lines = open(p, encoding='utf-8').read().splitlines()
items = [l for l in lines if re.match(r'- \[( |x|~)\]', l)]
open_items = [l for l in items if l.startswith('- [ ]')]
done = sum(1 for l in items if l.startswith('- [x]'))
exempt = sum(1 for l in items if l.startswith('- [~]'))
print(f"fidelity: {done}/{done+len(open_items)} done (exempt {exempt})")
print(f"violations: {len(open_items)}")
for l in open_items[:25]:
    print("  -", l[6:150])
if len(open_items) > 25:
    print(f"  ... {len(open_items)-25} more")
sys.exit(1 if open_items else 0)
