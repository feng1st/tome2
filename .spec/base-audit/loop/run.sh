#!/bin/sh
# The convergence loop's entry point: sync mechanical checklists, then run the
# anti-cheat acceptance.  Exit 0 only when everything is consistent.
set -e
cd "$(dirname "$0")/../../.."
python3 .spec/base-audit/loop/sync_checklists.py
python3 .spec/base-audit/loop/accept.py "$@"
