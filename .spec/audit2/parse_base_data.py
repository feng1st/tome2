#!/usr/bin/env python3
"""按原版解析代码的语法逐文件解析 lib/edit（audit2）。

语法来源：
  - process_dungeon_file (init1.cc:6844)：`?.`条件/`%:`包含/`F:`特性/D:`地图行
    —— 用于 *.map、t_*.txt、t_info.txt、misc.txt（原版对 misc.txt 就是走这个
    解析器，见 init2.cc:532）。
  - init1.cc init_*_txt：`N:` 起始、字母+`:` 字段的记录文件（r/k/f/e/a/...；
    q_info 若不存在则忽略）。
  - files.cc process_pref_file：`<key>:<fields>` 指令（lib/pref/*.prf）。

产出（.spec/audit2/checklist/data/<file>.md，逐条/逐行，带行号）：
  - 记录文件：每条记录一行，附紧凑字段串（D: 全文截断）。
  - 地图/城镇/杂项：每行一条，kind={cond,include,feat,row,raw}。
  - pref：每条指令一行，kind=指令首段。
index.json 中的 data 段同步更新为逐条明细索引。
"""
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
EDIT = os.path.join(ROOT, "lib", "edit")
PREF = os.path.join(ROOT, "lib", "pref")
OUT = os.path.join(ROOT, ".spec", "audit2", "checklist")
DATA_OUT = os.path.join(OUT, "data")

DUNGEON_FILES = {"misc.txt"}


def read(path):
    with open(path, encoding="latin-1") as f:
        return f.read()


def info_records(path):
    """N:-record parser (init1.cc init_*_txt record grammar)."""
    records = []
    cur = None
    for i, line in enumerate(read(path).splitlines(), 1):
        if not line or line[0] in ("#", " "):
            continue
        if len(line) > 1 and line[1] == ":":
            tag, val = line[0], line[2:]
            if tag == "N":
                cur = {"line": i, "id": val.split(":")[0], "name": val.split(":")[1]
                       if ":" in val else val, "fields": []}
                records.append(cur)
            elif cur is not None:
                cur["fields"].append((i, tag, val))
    return records


def dungeon_lines(path):
    items = []
    for i, line in enumerate(read(path).splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        if line.startswith("?:"):
            kind = "cond"
        elif line.startswith("%:"):
            kind = "include"
        elif line.startswith("F:"):
            kind = "feat"
        elif line.startswith("D:"):
            kind = "row"
        else:
            kind = "raw"
        items.append({"line": i, "kind": kind, "text": line[:160]})
    return items


def pref_lines(path):
    items = []
    for i, line in enumerate(read(path).splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        items.append({"line": i, "kind": line.split(":", 1)[0],
                      "text": line[:160]})
    return items


def file_items(path):
    name = os.path.basename(path)
    if name.endswith("_info.txt"):
        recs = info_records(path)
        if recs:
            items = []
            for r in recs:
                fields = "; ".join(
                    f"{tag}@{ln}={val if len(val) <= 90 else val[:90] + '…'}"
                    for ln, tag, val in r["fields"])
                items.append({
                    "line": r["line"], "kind": "record",
                    "id": r["id"], "name": r["name"],
                    "text": f"id={r['id']} {r['name']} | {fields}",
                })
            return items
    if name in DUNGEON_FILES or name.endswith(".map") or name.startswith("t_"):
        return dungeon_lines(path)
    if name.endswith(".txt") or name.endswith(".prf"):
        return pref_lines(path)
    return []


def text_lines(path, rel):
    """Plain text data (lib/file, lib/help): every line is an item."""
    items = []
    for i, line in enumerate(read(path).splitlines(), 1):
        if line.strip() == "":
            continue
        items.append({"line": i, "kind": "line", "text": line[:160]})
    return items


def asset_entry(path, rel):
    """Binary/UI assets (lib/xtra fonts/music): one item per file."""
    size = os.path.getsize(path)
    return [{"line": 1, "kind": "asset", "text": f"{rel} ({size} bytes)"}]


def placeholder_entry(path, rel):
    return [{"line": 1, "kind": "placeholder", "text": rel}]


def walk_lib():
    """All files under lib/ except lib/mods (theme excluded by user)."""
    out = []
    for root, dirs, files in os.walk(os.path.join(ROOT, "lib")):
        dirs[:] = [d for d in dirs if d != "mods"]
        for fname in sorted(files):
            if fname in ("CMakeLists.txt", ".gitignore"):
                continue  # build metadata, not game data
            path = os.path.join(root, fname)
            rel = os.path.relpath(path, os.path.join(ROOT, "lib"))
            top = rel.split(os.sep)[0]
            if top == "edit":
                items = file_items(path)
                cat = "edit"
            elif top == "pref":
                items = file_items(path)
                # Terminal/window/font preferences are HUD configuration
                # (user ruling: not portable to the Bevy HUD).  colors.prf
                # stays required: it redefines the palette and is applied by
                # bevy/src/colors.rs.
                cat = "pref" if fname == "colors.prf" else "hud"
            elif top in ("file", "help"):
                items = text_lines(path, rel)
                cat = "text"
            elif top == "xtra":
                items = asset_entry(path, rel)
                cat = "asset"
            else:
                if fname == "delete.me":
                    items = placeholder_entry(path, rel)
                    cat = "placeholder"
                else:
                    items = text_lines(path, rel)
                    cat = "text"
            if not items:
                # Comment-only documentation files (readme.txt, font-mac.prf):
                # list the comment lines as `doc` (index value, exempt).
                docs = []
                for i, line in enumerate(read(path).splitlines(), 1):
                    if line.startswith("#"):
                        docs.append({"line": i, "kind": "doc",
                                     "text": line.lstrip("# ").strip()[:160]})
                if docs:
                    items, cat = docs, "doc"
                else:
                    items = [{"line": 1, "kind": "empty",
                              "text": f"{rel} (no records)"}]
                    cat = "placeholder"
            out.append((rel, cat, items))
    return out


def build():
    os.makedirs(DATA_OUT, exist_ok=True)
    index = []
    for rel, cat, items in walk_lib():
        detail = os.path.join(DATA_OUT, rel.replace(os.sep, "__") + ".md")
        with open(detail, "w") as f:
            f.write(f"# 数据清单：lib/{rel}（{cat}，逐条/逐行）\n\n")
            for it in items:
                marker = " " if cat in ("edit", "pref", "text") else "~"
                f.write(f"- [{marker}] `lib/{rel}:{it['line']}` "
                        f"[{it['kind']}] {it['text']}\n")
        index.append({
            "file": "lib/" + rel.replace(os.sep, "/"),
            "category": cat,
            "items": len(items),
            "detail": os.path.relpath(detail, OUT),
            "kind": items[0]["kind"],
        })
    with open(os.path.join(OUT, "data.md"), "w") as f:
        f.write("# 原版数据文件清单（audit2，逐条/逐行）\n\n")
        f.write("| 文件 | 类别 | 条目数 | 类型 | 明细 |\n|---|---|---|---|---|\n")
        for e in index:
            f.write(f"| `{e['file']}` | {e['category']} | {e['items']} | "
                    f"{e['kind']} | [{e['detail']}]({e['detail']}) |\n")
    with open(os.path.join(OUT, "data_index.json"), "w") as f:
        json.dump(index, f, indent=1)
    return index


def main():
    index = build()
    total = sum(e["items"] for e in index)
    print(f"data files: {len(index)}, items: {total}")
    for e in index[:8]:
        print(f"  {e['file']}: {e['items']} ({e['kind']})")
    # persist index fragment for extract_base/audit
    with open(os.path.join(OUT, "data_index.json"), "w") as f:
        json.dump(index, f, indent=1)


if __name__ == "__main__":
    main()
