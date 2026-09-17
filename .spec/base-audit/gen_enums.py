#!/usr/bin/env python3
"""Generate enumeration checklists for the base audit.

Sources:
  - src/*_flag_list.hpp  (X-macro flag tables)
  - src/defines.hpp      (GF_*, RBE_*, SUMMON_*, TV_*, etc.)
  - src/tables.cc        (gf_names[])
  - src/spells5.cc       (school_spells_init + spells_init_tome = base ToME spell set)
  - lib/edit/*.txt       (record counts per data file)
Output: .spec/base-audit/enums/*.md
"""
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, "src")
EDIT = os.path.join(ROOT, "lib", "edit")
OUT = os.path.join(ROOT, ".spec", "base-audit", "enums")
os.makedirs(OUT, exist_ok=True)


def read(p):
    with open(p, "r", errors="replace") as f:
        return f.read()


def extract_xmacro(src_file, macro):
    """Return list of (tier, index, name) from an X-macro list file."""
    txt = read(os.path.join(SRC, src_file))
    pat = re.compile(rf"^{macro}\(\s*(\d+)\s*,\s*(\d+)\s*,\s*([A-Za-z_0-9]+)\s*\)", re.M)
    return [(int(a), int(b), c) for a, b, c in pat.findall(txt)]


def extract_defines(txt, prefix):
    pat = re.compile(rf"^#define\s+({prefix}[A-Z_0-9]+)\s+(-?\d+)", re.M)
    return [(n, int(v)) for n, v in pat.findall(txt)]


def extract_spells():
    """Parse spells5.cc school_spells_init (base) + spells_init_tome."""
    lines = read(os.path.join(SRC, "spells5.cc")).splitlines()
    spells = []
    cur = None
    for i, line in enumerate(lines, 1):
        m = re.search(r'spell_new\(&([A-Z_0-9]+),\s*"([^"]+)"\)', line)
        if m:
            if cur:
                spells.append(cur)
            cur = {"line": i, "idx": m.group(1), "name": m.group(2),
                   "mana": "", "diff": "", "school": "?", "desc": [], "init": ""}
            continue
        if not cur:
            continue
        m = re.search(r'spell_type_set_mana\(spell,\s*([^,]+),\s*([^)]+)\)', line)
        if m:
            cur["mana"] = f"{m.group(1).strip()}/{m.group(2).strip()}"
        m = re.search(r'spell_type_set_difficulty\(spell,\s*([^,]+),\s*([^)]+)\)', line)
        if m:
            cur["diff"] = f"{m.group(1).strip()}/{m.group(2).strip()}"
        m = re.search(r'spell_type_describe\(spell,\s*"([^"]*)"', line)
        if m:
            cur["desc"].append(m.group(1))
        m = re.search(r'spell_type_init_(\w+)\(spell,\s*', line)
        if m:
            cur["init"] = m.group(1)
            if m.group(1) in ("mage", "priest"):
                # next arg may be RANDOM then school on this or next line
                tail = line[m.end():]
                sm = re.search(r'(?:RANDOM|SCHOOL_\w+)\s*,\s*(SCHOOL_\w+)', tail)
                if not sm:
                    sm = re.search(r'SCHOOL_\w+', " ".join(lines[i:i + 2]))
                cur["school"] = sm.group(0) if sm else "?"
        if cur["init"] in ("mage", "priest") and cur["school"] == "?":
            sm = re.search(r'SCHOOL_\w+', line)
            if sm:
                cur["school"] = sm.group(0)
    if cur:
        spells.append(cur)
    return spells


def main():
    defines = read(os.path.join(SRC, "defines.hpp"))
    flag_files = [
        ("feature_flag_list.hpp", "FF", "地形旗标 feature flags"),
        ("monster_race_flag_list.hpp", "RF", "怪物种族旗标"),
        ("monster_spell_flag_list.hpp", "RSF", "怪物法术旗标"),
        ("object_flag_list.hpp", "OF", "物品旗标"),
        ("ego_flag_list.hpp", "EF", "ego/artifact 旗标"),
        ("player_race_flag_list.hpp", "PRF", "种族/职业玩家旗标"),
        ("skill_flag_list.hpp", "SKF", "技能旗标"),
        ("store_flag_list.hpp", "STF", "商店旗标"),
        ("dungeon_flag_list.hpp", "DF", "地城旗标"),
    ]
    out = ["# Flag enumeration checklists", "",
           "自动生成：`src/*_flag_list.hpp` 的全部 X-macro 旗标。", ""]
    for fname, macro, desc in flag_files:
        items = extract_xmacro(fname, macro)
        out.append(f"## {fname} — {desc}（{len(items)}）")
        out.append("")
        for tier, idx, name in items:
            out.append(f"- [ ] `{macro}_{name}` (tier {tier}, idx {idx})")
        out.append("")
    with open(os.path.join(OUT, "00-flags.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    # GF + RBE
    gf = extract_defines(defines, "GF_")
    rbe = extract_defines(defines, "RBE_")
    gfn = re.findall(r'\{\s*(GF_\w+)\s*,\s*"([^"]+)"\s*\}', read(os.path.join(SRC, "tables.cc")))
    out = ["# GF / blow-effect enumerations", "",
           "## GF_* 定义（defines.hpp）", ""]
    for n, v in gf:
        out.append(f"- [ ] `{n}` = {v}")
    out += ["", "## gf_names[] 表（tables.cc，元素名）", ""]
    for n, desc in gfn:
        out.append(f"- [ ] `{n}` \"{desc}\"")
    out += ["", "## RBE_* 怪物近战特效（defines.hpp）", ""]
    for n, v in rbe:
        out.append(f"- [ ] `{n}` = {v}")
    with open(os.path.join(OUT, "01-gf-blow.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    # spells
    spells = extract_spells()
    out = [f"# Base spell enumeration（spells5.cc，{len(spells)} 条）", "",
           "Tome 模块的全部注册法术（school_spells_init + spells_init_tome）。",
           "Theme 专属（GROW_ATHELAS/AULE_*/VARDA_*/ULMO_*/MANDOS_*）不在此表。",
           "RANDOM = 多学派随机，school 显示已解析值。", ""]
    for s in spells:
        desc = " / ".join(s["desc"])
        out.append(f"- [ ] `spells5.cc:{s['line']}` **{s['name']}** "
                   f"[{s['idx']}] init={s['init']} school={s['school']} "
                   f"mana={s['mana']} diff={s['diff']} — {desc}")
    with open(os.path.join(OUT, "02-spells.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    # monster spells / summon types
    summ = extract_defines(defines, "SUMMON_")
    out = ["# Summon types (defines.hpp)", ""]
    for n, v in summ:
        out.append(f"- [ ] `{n}` = {v}")
    with open(os.path.join(OUT, "03-summons.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    # data record counts
    out = ["# Data file record counts", ""]
    for fname in sorted(os.listdir(EDIT)):
        if not fname.endswith(".txt"):
            continue
        txt = read(os.path.join(EDIT, fname))
        recs = len(re.findall(r"^N:", txt, re.M))
        out.append(f"- `{fname}`: N: records = {recs}")
    with open(os.path.join(OUT, "04-data-counts.md"), "w") as f:
        f.write("\n".join(out) + "\n")

    print("spells:", len(spells))
    print("flag files:", len(flag_files))


if __name__ == "__main__":
    main()
