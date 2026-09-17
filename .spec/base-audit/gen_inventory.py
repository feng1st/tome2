#!/usr/bin/env python3
"""Generate base-audit method inventories from the original ToME C++ source.

Output: .spec/base-audit/inventory/*.md, one file per subsystem group.
Each entry is a checkbox line ready for audit annotation:
    - [ ] `file.cc:LINE` name(args) — doc comment

Status legend (annotate by hand/agent after generation):
    [x] ported & verified in bevy
    [>] partially ported (note what is missing)
    [ ] NOT ported yet
    [~] n/a: data-dead, theme-only, frontend-only, or intentionally not ported
The generated files are audit snapshots; re-running overwrites annotations.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, "src")
OUT = os.path.join(ROOT, ".spec", "base-audit", "inventory")

GROUPS = {
    "00-core.md": [
        "game.cc", "files.cc", "init1.cc", "init2.cc", "modules.cc",
        "options.cc", "message.cc", "messages.cc", "notes.cc", "hiscore.cc",
        "loadsave.cc", "level_data.cc", "level_marker.cc", "levels.cc",
        "quest.cc", "hooks.cc", "dice.cc", "z-rand.cc",
    ],
    "01-commands.md": [
        "cmd1.cc", "cmd2.cc", "cmd3.cc", "cmd4.cc", "cmd5.cc", "cmd6.cc",
        "cmd7.cc", "bldg.cc", "store.cc", "help.cc", "squeltch.cc",
    ],
    "02-spells.md": [
        "spells1.cc", "spells2.cc", "spells3.cc", "spells4.cc", "spells5.cc",
        "spells6.cc", "spell_type.cc", "powers.cc", "randart.cc",
    ],
    "03-monsters.md": [
        "melee1.cc", "melee2.cc", "monster1.cc", "monster2.cc", "monster3.cc",
        "monster_spell.cc", "monster_type.cc",
    ],
    "04-objects.md": [
        "object1.cc", "object2.cc", "object_filter.cc", "object_flag_meta.cc",
    ],
    "05-map.md": [
        "cave.cc", "generate.cc", "gen_maze.cc", "gen_evol.cc", "dungeon.cc",
        "wild.cc",
    ],
    "06-quests.md": [
        "q_main.cc", "q_betwen.cc", "q_bounty.cc", "q_dragons.cc", "q_eol.cc",
        "q_evil.cc", "q_fireprof.cc", "q_god.cc", "q_haunted.cc", "q_hobbit.cc",
        "q_invas.cc", "q_library.cc", "q_narsil.cc", "q_nazgul.cc", "q_nirna.cc",
        "q_one.cc", "q_poison.cc", "q_rand.cc", "q_shroom.cc", "q_spider.cc",
        "q_thief.cc", "q_thrain.cc", "q_troll.cc", "q_ultrae.cc", "q_ultrag.cc",
        "q_wight.cc", "q_wolves.cc",
    ],
    "07-player.md": [
        "xtra1.cc", "xtra2.cc", "skills.cc", "birth.cc", "corrupt.cc",
        "mimic.cc", "gods.cc", "player_type.cc", "util.cc", "variable.cc",
    ],
    "09-tables.md": [
        "tables.cc", "z-form.cc", "z-util.cc",
    ],
}

# A line that starts at column 0 and looks like a definition.
FN_RE = re.compile(
    r"^(?:static\s+|inline\s+|extern\s+)*"
    r"(?:const\s+)?"
    r"(?:unsigned\s+|signed\s+)?"
    r"(?:void|bool|int|char|byte|s16b|s32b|u16b|u32b|long|short|float|double|"
    r"std::string|size_t|cptr|errr|term|object_type|monster_type|monster_race|"
    r"player_type|inventory|grid|auto|time_t|struct\s+\w+|\w+_type|[\w:]+_ptr|"
    r"[A-Za-z_]\w*(?:::[\w:]+)*)"
    r"[\s*&]+"
    r"([A-Za-z_]\w*)"
    r"\s*\((.*)$"
)


def strip_comment(lines, idx):
    """Return doc comment immediately preceding line idx (or '')."""
    j = idx - 1
    # skip blank lines
    while j >= 0 and lines[j].strip() == "":
        j -= 1
    if j < 0:
        return ""
    if lines[j].strip().endswith("*/"):
        end = j
        k = j
        while k >= 0 and "/*" not in lines[k]:
            k -= 1
        if k >= 0:
            raw = " ".join(lines[k:end + 1])
            raw = re.sub(r"/\*|\*/|\*", " ", raw)
            raw = re.sub(r"\s+", " ", raw).strip()
            if raw.startswith(" "):
                raw = raw[1:]
            return raw[:200]
    if lines[j].strip().startswith("//"):
        k = j
        while k >= 0 and lines[k].strip().startswith("//"):
            k -= 1
        raw = " ".join(l.strip().lstrip("/").strip() for l in lines[k + 1:j + 1])
        return re.sub(r"\s+", " ", raw).strip()[:200]
    return ""


def extract(path):
    with open(path, "r", errors="replace") as f:
        lines = f.readlines()
    out = []
    for i, line in enumerate(lines):
        if line.startswith((" ", "\t", "\n", "#", "//", "/*", "*", "}")):
            continue
        m = FN_RE.match(line.rstrip())
        if not m:
            continue
        groups = m.groups()
        name, rest = groups[-2], groups[-1]
        if name in ("if", "while", "for", "switch", "return", "else", "sizeof"):
            continue
        # require the signature to close with ) or ) { or similar on a following line
        sig = rest
        j = i
        while ")" not in sig and j < len(lines) - 1 and j - i < 6:
            j += 1
            sig += " " + lines[j].rstrip()
        if ")" not in sig:
            continue
        args = sig[:sig.index(")")]
        args = re.sub(r"\s+", " ", args).strip()
        if len(args) > 120:
            args = args[:120] + "..."
        doc = strip_comment(lines, i)
        out.append((i + 1, name, args, doc))
    return out


def main():
    os.makedirs(OUT, exist_ok=True)
    index = ["# Method inventory index", "",
             "Generated by `gen_inventory.py`. Each checkbox line is one C++ definition.",
             "Annotate: `[x]` ported+, `[>]` partial, `[ ]` missing, `[~]` n/a.", ""]
    for fname, files in GROUPS.items():
        body = [f"# Method inventory: {fname[:-3]}", ""]
        total = 0
        for base in files:
            path = os.path.join(SRC, base)
            if not os.path.exists(path):
                body.append(f"## {base} (MISSING)")
                continue
            fns = extract(path)
            total += len(fns)
            body.append(f"## {base} ({len(fns)} defs)")
            body.append("")
            for ln, name, args, doc in fns:
                desc = f" — {doc}" if doc else ""
                body.append(f"- [ ] `{base}:{ln}` **{name}**({args}){desc}")
            body.append("")
        index.append(f"- [{fname}](inventory/{fname}) — {total} defs")
        with open(os.path.join(OUT, fname), "w") as f:
            f.write("\n".join(body) + "\n")
        print(f"{fname}: {total} defs")
    with open(os.path.join(OUT, "README.md"), "w") as f:
        f.write("\n".join(index) + "\n")


if __name__ == "__main__":
    sys.exit(main())
