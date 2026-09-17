#!/usr/bin/env python3
"""Cross-check all C++ flag / GF / blow-effect / summon names for:
  - DATA: appears anywhere in lib/edit/*.txt (else data-dead => n/a)
  - RUST: appears as a string literal or enum name anywhere in bevy/src/*.rs
Output: .spec/base-audit/enums/05-coverage.md
"""
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, "src")
EDIT = os.path.join(ROOT, "lib", "edit")
BEVY = os.path.join(ROOT, "bevy", "src")
OUT = os.path.join(ROOT, ".spec", "base-audit", "enums", "05-coverage.md")

FLAG_FILES = [
    ("FF", "feature_flag_list.hpp", "feature flags"),
    ("RF", "monster_race_flag_list.hpp", "monster race flags"),
    ("RSF", "monster_spell_flag_list.hpp", "monster spell flags"),
    ("OF", "object_flag_list.hpp", "object flags"),
    ("EF", "ego_flag_list.hpp", "ego flags"),
    ("PRF", "player_race_flag_list.hpp", "player flags"),
    ("SKF", "skill_flag_list.hpp", "skill flags"),
    ("STF", "store_flag_list.hpp", "store flags"),
    ("DF", "dungeon_flag_list.hpp", "dungeon flags"),
]


def read(p):
    with open(p, "r", errors="replace") as f:
        return f.read()


def xmacro(fname, macro):
    txt = read(os.path.join(SRC, fname))
    pat = re.compile(rf"^{macro}\(\s*(?:\d+)\s*,\s*(?:\d+)\s*,\s*([A-Za-z_0-9]+)\s*\)", re.M)
    return pat.findall(txt)


def main():
    data_txt = "\n".join(read(os.path.join(EDIT, f)) for f in os.listdir(EDIT) if f.endswith(".txt"))
    rust_txt = "\n".join(read(os.path.join(BEVY, f)) for f in os.listdir(BEVY) if f.endswith(".rs"))

    # GF and RBE from defines.hpp
    defines = read(os.path.join(SRC, "defines.hpp"))
    gfs = re.findall(r"^#define\s+(GF_[A-Z_0-9]+)\s+\d+", defines, re.M)
    rbes = re.findall(r"^#define\s+(RBE_[A-Z_0-9]+)\s+\d+", defines, re.M)
    summons = re.findall(r"^#define\s+(SUMMON_[A-Z_0-9]+)\s+\d+", defines, re.M)

    out = ["# Mechanical coverage report", "",
           "`DATA`=在 lib/edit 数据中出现过；`RUST`=在 bevy/src 代码中作为标识串出现。",
           "RUST=0 且 DATA>0 的条目是**高优先级缺口候选**；DATA=0 为数据死条目（n/a）。", ""]

    def section(title, names, prefix=""):
        out.append(f"## {title}")
        out.append("")
        out.append("| name | DATA | RUST | note |")
        out.append("|---|---|---|---|")
        rows = []
        for n in names:
            bare = n[len(prefix):] if prefix else n
            d = len(re.findall(r"\b" + re.escape(bare) + r"\b", data_txt))
            r = len(re.findall(r'"' + re.escape(bare) + r'"', rust_txt)) + rust_txt.count(bare + "_") // 999
            note = ""
            if d == 0:
                note = "data-dead"
            elif r == 0:
                note = "**GAP?**"
            rows.append((n, d, r, note))
        for n, d, r, note in rows:
            out.append(f"| `{n}` | {d} | {r} | {note} |")
        out.append("")

    for macro, fname, desc in FLAG_FILES:
        section(f"{macro}: {desc} ({fname})", xmacro(fname, macro), macro + "_")
    section("GF_* spell types", gfs, "GF_")
    section("RBE_* blow effects", rbes, "RBE_")
    section("SUMMON_* types", summons, "SUMMON_")

    with open(OUT, "w") as f:
        f.write("\n".join(out) + "\n")

    # console summary of likely gaps
    print("== likely gaps (DATA>0, RUST==0) ==")
    total = 0
    for macro, fname, desc in FLAG_FILES:
        for n in xmacro(fname, macro):
            bare = n[len(macro) + 1:]
            d = len(re.findall(r"\b" + re.escape(bare) + r"\b", data_txt))
            r = len(re.findall(r'"' + re.escape(bare) + r'"', rust_txt))
            if d > 0 and r == 0:
                print(f"  {macro}_{bare} (data {d})")
                total += 1
    for n in gfs + rbes + summons:
        bare = n.split("_", 1)[1]
        d = len(re.findall(r"\b" + re.escape(bare) + r"\b", data_txt))
        r = len(re.findall(r'"' + re.escape(bare) + r'"', rust_txt))
        if d > 0 and r == 0:
            print(f"  {n} (data {d})")
            total += 1
    print("total suspects:", total)


if __name__ == "__main__":
    main()
