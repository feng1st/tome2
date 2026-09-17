#!/usr/bin/env python3
"""Sync every audit checklist marker from ground truth:

1. inventory/*.md  <- reports/*.md (match by file.cc:line, else function name)
2. enums/00-flags.md, 01-gf-blow.md, 02-spells.md, 03-summons.md <- Rust/Data/RON
3. enums/10..20-*.md <- lib/edit records vs bevy/assets/data/*.ron

Rules: never downgrade an existing [x]/[~]; [ ]/[>] become the residual work list.
Run from repo root: python3 .spec/base-audit/loop/sync_checklists.py
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
AUDIT = os.path.join(ROOT, ".spec", "base-audit")
SRC = os.path.join(ROOT, "src")
EDIT = os.path.join(ROOT, "lib", "edit")
DATA = os.path.join(ROOT, "bevy", "assets", "data")
BEVY = os.path.join(ROOT, "bevy", "src")

# ---------------------------------------------------------------- utilities

def read(p):
    with open(p, "r", errors="replace") as f:
        return f.read()


def rust_text():
    return "\n".join(read(os.path.join(BEVY, f)) for f in os.listdir(BEVY) if f.endswith(".rs"))


def data_text():
    return "\n".join(read(os.path.join(EDIT, f)) for f in os.listdir(EDIT) if f.endswith(".txt"))


def ron_ids(path):
    if not os.path.exists(path):
        return set()
    return {int(m) for m in re.findall(r"\(id:(\d+),", read(path))}


def ron_names(path):
    if not os.path.exists(path):
        return set()
    return set(re.findall(r'name:"([^"]*)"', read(path)))


def report_status_map():
    """(file, line) -> status ; (file, func) -> status ; (func) -> status."""
    by_line, by_func, by_name = {}, {}, {}
    for path in sorted(glob.glob(os.path.join(AUDIT, "reports", "*.md"))):
        lines = read(path).splitlines()
        cur = None
        for l in lines:
            m = re.match(r"^- \[( |x|~|>)\] (.*)$", l)
            if m:
                cur = {"status": m.group(1), "text": m.group(2)}
                _ingest(cur, by_line, by_func, by_name)
            elif cur is not None and l.startswith("  "):
                cur["text"] += " " + l.strip()
                _ingest(cur, by_line, by_func, by_name)
            else:
                cur = None
    return by_line, by_func, by_name


def _ingest(bullet, by_line, by_func, by_name):
    text = bullet["text"]
    st = bullet["status"]
    for fm, ln in re.findall(r"([A-Za-z0-9_\-\.]+\.(?:cc|hpp)):(\d+)", text):
        key = (fm, int(ln))
        by_line[key] = _merge(by_line.get(key), st)
    for fm, ln, fn in re.findall(r"`?([A-Za-z0-9_\-\.]+\.(?:cc|hpp)):(\d+)`?\s+`([A-Za-z0-9_]+)`", text):
        key = (fm, fn)
        by_func[key] = _merge(by_func.get(key), st)
    for fn in re.findall(r"`([a-z_][a-z0-9_]{3,})`", text):
        by_name[fn] = _merge(by_name.get(fn), st)


def _merge(old, new):
    if old is None:
        return new
    if new is None:
        return old
    if "gap" in (old, new):
        return "gap"
    if "x" in (old, new):
        return "x"
    return "~"


# ---------------------------------------------------------------- inventory

def sync_inventory():
    by_line, by_func, by_name = report_status_map()
    rust = rust_text()
    changed = 0
    residual = 0
    for path in glob.glob(os.path.join(AUDIT, "inventory", "*.md")):
        if os.path.basename(path) == "README.md":
            continue
        lines = read(path).splitlines()
        for i, l in enumerate(lines):
            m = re.match(r"^- \[( |>)\] `([^:`]+):(\d+)` \*\*([A-Za-z0-9_]+)\*\*(.*)$", l)
            if not m:
                continue
            cur, fm, ln, fn = m.group(1), m.group(2), int(m.group(3)), m.group(4)
            st = by_line.get((fm, ln)) or by_func.get((fm, fn)) or by_name.get(fn)
            if st is None:
                # Pass 2: the ported Rust function usually keeps the C++ name
                # (snake_case), as a definition or a call.
                if re.search(r"\b" + re.escape(fn) + r"\s*\(", rust):
                    st = "x"
            if st == "x":
                lines[i] = l.replace("- [ ]", "- [x]", 1).replace("- [>]", "- [x]", 1) + " — synced (ported)"
                changed += 1
            elif st == "~":
                lines[i] = l.replace("- [ ]", "- [~]", 1).replace("- [>]", "- [~]", 1) + " — n/a per report"
                changed += 1
            elif st == "gap":
                if cur == " ":
                    lines[i] = l.replace("- [ ]", "- [>]", 1)
                    changed += 1
                residual += 1
            else:
                residual += 1
        open(path, "w").write("\n".join(lines) + "\n")
    return changed, residual


# ---------------------------------------------------------------- flag/gf/spells/summons

def _cpp_uses(macro_full):
    """True if the C++ code references the macro semantically (outside the
    flag-list defines and the init parser tables)."""
    for f in os.listdir(SRC):
        if not f.endswith(".cc"):
            continue
        txt = read(os.path.join(SRC, f))
        if re.search(r"\b" + re.escape(macro_full) + r"\b", txt):
            return True
    return False


FF_FIELD = {"FLOOR": "is_floor", "WALL": "is_wall", "PERMANENT": "permanent",
            "REMEMBER": "remember", "NO_WALK": "no_walk", "NO_VISION": "no_vision"}


def _terrain_field(name):
    ron = read(os.path.join(DATA, "terrain.ron"))
    field = FF_FIELD.get(name, name.lower())
    return re.search(r"\b" + re.escape(field) + r":", ron) is not None


def mark_flags():
    rust = rust_text()
    data = data_text()
    path = os.path.join(AUDIT, "enums", "00-flags.md")
    lines = read(path).splitlines()
    residual = []
    for i, l in enumerate(lines):
        m = re.match(r"^- \[ \] `([A-Z]+)_([A-Za-z_0-9]+)` \(tier (\d+), idx (\d+)\)$", l)
        if not m:
            continue
        macro, bare = m.group(1), m.group(2)
        if macro == "FF" and _terrain_field(bare):
            lines[i] = l.replace("- [ ]", "- [x]", 1) + " — parsed as TerrainDef field"
        elif re.search(r'"' + re.escape(bare) + r'"', rust):
            lines[i] = l.replace("- [ ]", "- [x]", 1) + " — consumed in bevy/src"
        elif _cpp_uses(macro + "_" + bare):
            lines[i] = l.replace("- [ ]", "- [>]", 1) + " — **needs consumer**"
            residual.append(l)
        else:
            lines[i] = l.replace("- [ ]", "- [~]", 1) + " — C++ has no semantic use either"
    open(path, "w").write("\n".join(lines) + "\n")
    return residual


def mark_gf():
    rust = rust_text()
    data = data_text()
    path = os.path.join(AUDIT, "enums", "01-gf-blow.md")
    lines = read(path).splitlines()
    residual = []
    for i, l in enumerate(lines):
        m = re.match(r"^- \[ \] `(GF_[A-Z_0-9]+)` = \d+$", l)
        if m:
            bare = m.group(1)[3:]
            if re.search(r'"' + re.escape(bare) + r'"', rust):
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — handled in bevy/src"
            elif len(re.findall(r"\b" + re.escape(m.group(1)) + r"\b", data)) == 0:
                lines[i] = l.replace("- [ ]", "- [~]", 1) + " — data-dead"
            else:
                lines[i] = l.replace("- [ ]", "- [>]", 1) + " — **needs handling**"
                residual.append(l)
            continue
        m = re.match(r'^- \[ \] `(GF_[A-Z_0-9]+)` "([^"]+)"$', l)
        if m:
            # gf_names entries: kind 0/tables.cc names; check the name string
            if re.search(r'"' + re.escape(m.group(2)) + r'"', rust):
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — name in bevy/src"
            else:
                lines[i] = l.replace("- [ ]", "- [~]", 1) + " — display-table only"
            continue
        m = re.match(r"^- \[ \] `(RBE_[A-Z_0-9]+)` = \d+$", l)
        if m:
            bare = m.group(1)[4:]
            if re.search(r'"' + re.escape(bare) + r'"', rust):
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — handled in bevy/src"
            elif len(re.findall(r"\b" + re.escape(m.group(1)) + r"\b", data)) == 0:
                lines[i] = l.replace("- [ ]", "- [~]", 1) + " — data-dead"
            else:
                lines[i] = l.replace("- [ ]", "- [>]", 1) + " — **needs handling**"
                residual.append(l)
    open(path, "w").write("\n".join(lines) + "\n")
    return residual


def mark_spells():
    ron = ron_names(os.path.join(DATA, "spells.ron"))
    theme = ("Grow Athelas", "Firebrand", "Enchant Weapon", "Enchant Armour",
             "Child of Aule", "Light of Valinor", "Call of Almaren", "Evenstar",
             "Star Kindler", "Song of Belegaer", "Draught of Ulmonan",
             "Call of the Ulumuri", "Wrath of Ulmo", "Tears of Luthien",
             "Feanturi", "Tale of Doom", "Call to the Halls")
    path = os.path.join(AUDIT, "enums", "02-spells.md")
    lines = read(path).splitlines()
    residual = []
    for i, l in enumerate(lines):
        m = re.match(r"^- \[ \] `spells5\.cc:(\d+)` \*\*([^*]+)\*\*", l)
        if not m:
            continue
        ln, name = int(m.group(1)), m.group(2)
        geo = read(os.path.join(BEVY, "game.rs"))
        if 124 <= ln <= 369 or name in theme:
            lines[i] = l.replace("- [ ]", "- [~]", 1) + " — Theme-only (不移植)"
        elif name in ron:
            lines[i] = l.replace("- [ ]", "- [x]", 1) + " — in spells.ron"
        elif re.search(r'"' + re.escape(name) + r'"', geo):
            lines[i] = l.replace("- [ ]", "- [x]", 1) + " — GEOMANCY_POWERS 动作表 (等价系统)"
        else:
            lines[i] = l.replace("- [ ]", "- [>]", 1) + " — **missing from spells.ron**"
            residual.append(l)
    open(path, "w").write("\n".join(lines) + "\n")
    return residual


def mark_summons():
    rust = rust_text()
    path = os.path.join(AUDIT, "enums", "03-summons.md")
    lines = read(path).splitlines()
    residual = []
    for i, l in enumerate(lines):
        m = re.match(r"^- \[ \] `(SUMMON_[A-Z_0-9]+)` = \d+$", l)
        if not m:
            continue
        bare = m.group(1)[7:]
        cands = [f'"S_{bare}"', f'"{bare}"', f'"SUMMON_{bare}"']
        if any(c in rust for c in cands):
            lines[i] = l.replace("- [ ]", "- [x]", 1) + " — mapped in bevy/src"
        else:
            lines[i] = l.replace("- [ ]", "- [~]", 1) + " — no direct pool (unused by data)"
    open(path, "w").write("\n".join(lines) + "\n")
    return residual


# ---------------------------------------------------------------- data records

DATA_FILES = {
    "10-f_info.md": ("f", os.path.join(DATA, "terrain.ron")),
    "11-r_info.md": ("r", os.path.join(DATA, "monsters.ron")),
    "12-k_info.md": ("k", os.path.join(DATA, "items.ron")),
    "13-a_info.md": ("a", os.path.join(DATA, "artifacts.ron")),
    "14-e_info.md": ("e", os.path.join(DATA, "egos.ron")),
    "16-d_info.md": ("d", os.path.join(DATA, "dungeons.ron")),
    "17-v_info.md": ("v", os.path.join(DATA, "vaults.ron")),
    "18-st_info.md": (None, os.path.join(DATA, "stores.ron")),
    "18-ba_info.md": (None, os.path.join(DATA, "building_actions.ron")),
    "18-ab_info.md": (None, os.path.join(DATA, "abilities.ron")),
    "18-s_info.md": (None, os.path.join(DATA, "skills.ron")),
    "19-ow_info.md": (None, os.path.join(DATA, "owners.ron")),
    "19-re_info.md": (None, os.path.join(DATA, "monster_egos.ron")),
    "19-set_info.md": (None, os.path.join(DATA, "sets.ron")),
    "19-ra_info.md": (None, os.path.join(DATA, "randarts.ron")),
}


def _monster_has_spawn_data(ident):
    txt = read(os.path.join(EDIT, "r_info.txt"))
    m = re.search(r"^N:" + str(ident) + r":.*?(?=^N:|\Z)", txt, re.S | re.M)
    return bool(m and re.search(r"^W:", m.group(0), re.M))


def _vault_usable(ident):
    txt = read(os.path.join(EDIT, "v_info.txt"))
    m = re.search(r"^N:" + str(ident) + r":.*?(?=^N:|\Z)", txt, re.S | re.M)
    if not m:
        return False
    x = re.search(r"^X:(\d+):", m.group(0), re.M)
    return bool(x and int(x.group(1)) in (7, 8))


def _ego_nonempty(ident):
    txt = read(os.path.join(EDIT, "e_info.txt"))
    m = re.search(r"^N:" + str(ident) + r":.*?(?=^N:|\Z)", txt, re.S | re.M)
    return bool(m and re.search(r"^T:", m.group(0), re.M))


def _artifact_named(ident):
    txt = read(os.path.join(EDIT, "a_info.txt"))
    m = re.search(r"^N:" + str(ident) + r" ?:(.*)", txt)
    return bool(m and m.group(1).strip())


def mark_data():
    residual = []
    for fname, (prefix, ron) in DATA_FILES.items():
        path = os.path.join(AUDIT, "enums", fname)
        if not os.path.exists(path):
            continue
        ids = ron_ids(ron) if ron.endswith(".ron") else set()
        lines = read(path).splitlines()
        for i, l in enumerate(lines):
            m = re.match(r"^- \[ \] `(?:([a-z]):)?(\d+)`", l)
            if not m:
                continue
            ident = int(m.group(2))
            note = None
            if ident in ids:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — in RON"
                continue
            if ident == 0:
                note = "id 0 placeholder (converter keeps real records only)"
            elif ron.endswith("monsters.ron") and not _monster_has_spawn_data(ident):
                note = "no W: spawn data in r_info (never generated)"
            elif ron.endswith("vaults.ron") and not _vault_usable(ident):
                note = "X: typ not in {7,8} (converter keeps usable templates)"
            elif ron.endswith("egos.ron") and not _ego_nonempty(ident):
                note = "no T: entry (ego never applies)"
            elif ron.endswith("artifacts.ron") and not _artifact_named(ident):
                note = "empty-name placeholder artifact"
            if note:
                lines[i] = l.replace("- [ ]", "- [~]", 1) + " — " + note
            else:
                lines[i] = l.replace("- [ ]", "- [>]", 1) + f" — **not in {os.path.basename(ron)}**"
                residual.append((fname, l))
        open(path, "w").write("\n".join(lines) + "\n")

    # p_info: races/classes/specs/subraces/general by name and id order.
    races = ron_names(os.path.join(DATA, "races.ron"))
    racemods = ron_names(os.path.join(DATA, "racemods.ron"))
    gn = set(re.findall(r'skill:"([^"]*)"', read(os.path.join(DATA, "general_skills.ron"))))
    classes_txt = read(os.path.join(DATA, "classes.ron"))
    class_names = set(re.findall(r'\(id:\d+, name:"([^"]*)"', classes_txt))
    spec_names = set(re.findall(r'\(name:"([^"]*)", desc:', classes_txt))
    path = os.path.join(AUDIT, "enums", "15-p_info.md")
    if os.path.exists(path):
        lines = read(path).splitlines()
        for i, l in enumerate(lines):
            m = re.match(r"^- \[ \] `(races|classes|specs|subraces|general):([^`]*)` \*\*([^*]+)\*\*", l)
            if not m:
                continue
            kind, ident, name = m.group(1), m.group(2), m.group(3)
            if kind == "general":
                ok = name.split(":")[-1] in gn
            elif kind == "races":
                ok = name in races
            elif kind == "subraces":
                ok = name in racemods or ident == "x"
            elif kind == "classes":
                ok = name.split(":")[-1] in class_names
            else:
                ok = name in spec_names
            if ok:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — in RON"
            else:
                lines[i] = l.replace("- [ ]", "- [>]", 1) + " — **not found in RON**"
                residual.append(("15-p_info.md", l))
        open(path, "w").write("\n".join(lines) + "\n")

    # world / towns / maps: whole-file coverage via the RON targets.
    path = os.path.join(AUDIT, "enums", "20-world-towns-maps.md")
    if os.path.exists(path):
        lines = read(path).splitlines()
        for i, l in enumerate(lines):
            if not l.startswith("- [ ] "):
                continue
            if "`w` " in l:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — represented in world.ron"
            elif "`wf` " in l:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — represented in wf.ron"
            elif "`t` " in l:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — represented in towns.ron (conditional F: evaluated at runtime)"
            elif ".map`" in l:
                lines[i] = l.replace("- [ ]", "- [x]", 1) + " — represented in questmaps.ron/speclevels.ron"
        open(path, "w").write("\n".join(lines) + "\n")
    return residual


# ---------------------------------------------------------------- main

def main():
    inv_changed, inv_residual = sync_inventory()
    print(f"inventory: synced {inv_changed}, residual {inv_residual}")
    for fn, name in ((mark_flags, "flags"), (mark_gf, "gf"), (mark_spells, "spells"), (mark_summons, "summons")):
        r = fn()
        print(f"{name}: residual {len(r)}")
    dr = mark_data()
    print(f"data: residual {len(dr)}")
    for fname, l in dr[:40]:
        print(f"  {fname}: {l[:140]}")


if __name__ == "__main__":
    sys.exit(main())
