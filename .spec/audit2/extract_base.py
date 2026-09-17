#!/usr/bin/env python3
"""从原版 C++ 源码与数据文件抽取审计清单（audit2）。

规则（用户给定）：
  - HUD 不走原版：终端绘制/状态栏/前端与消息显示类函数标记为 `hud`，
    列出但不要求与新版逐行等价。
  - lib/mods/theme 不移植：该目录不抽取。
  - 其余全部抽取，一项一项与新实现做逻辑比对。

产出（.spec/audit2/checklist/）：
  methods.md   每个函数/方法一行：文件:行、类、签名、注释首句
  defines.md   每个枚举/常量一行：文件:行、名称、值
  data.md      每个数据文件：行数、记录数、字段标签直方图、样例行号
  index.json   同上结构化版本，供 audit.py / 人工回查
"""
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "src")
LIB = os.path.join(ROOT, "lib")
EDIT = os.path.join(LIB, "edit")
OUT = os.path.join(ROOT, ".spec", "audit2", "checklist")

# HUD 豁免（用户规则）：终端/前端/绘制/消息显示相关文件与命名。
HUD_FILES = {
    "z-term.cc", "z-form.cc", "z-util.cc", "main.cc", "main-gcu.cc",
    "main-x11.cc", "main-gtk2.cc", "main-win.cc", "frontend.cc",
    "help.cc", "wizard2.cc", "key_queue.cc", "program_args.cc",
    "format_ext.cc", "joke.cc",
}
HUD_PREFIXES = (
    "prt_", "display_", "draw_", "show_", "put_str", "c_put_str", "c_prt",
    "text_out", "print_", "msg_", "cmsg_", "screen_save", "screen_load",
    "inkey", "askfor", "bell", "flush",
)

# 不抽取的目录（用户规则：主题不移植）。
SKIP_DIRS = ("lib/mods",)


def read(path):
    with open(path, encoding="latin-1") as f:
        return f.read()


def is_hud(fname, func):
    return fname in HUD_FILES or func.startswith(HUD_PREFIXES)


def doc_comment(lines, idx):
    """Contiguous // or /** */ comment block directly above lines[idx]."""
    out = []
    i = idx - 1
    # block comment
    if i >= 0 and lines[i].strip().endswith("*/"):
        while i >= 0:
            out.append(lines[i].strip())
            if lines[i].strip().startswith("/*"):
                break
            i -= 1
        return " ".join(reversed(out))
    while i >= 0 and lines[i].strip().startswith("//"):
        out.append(lines[i].strip().lstrip("/").strip())
        i -= 1
    return " ".join(reversed(out))


FUNC_RE = re.compile(
    r"^(?:template\s*<[^>]*>\s*)?"
    r"(?:static\s+|inline\s+|constexpr\s+|virtual\s+|extern\s+)*"
    r"(?:const\s+)?[A-Za-z_][\w:<>,\*&\s]*?"
    r"\b([A-Za-z_]\w*)\s*\("
)
CLASS_RE = re.compile(r"\b([A-Za-z_]\w*)::([A-Za-z_]\w*)\s*\(")


def extract_methods():
    items = []
    for fname in sorted(os.listdir(SRC)):
        if not fname.endswith((".cc", ".hpp")):
            continue
        path = os.path.join(SRC, fname)
        lines = read(path).splitlines()
        for i, line in enumerate(lines):
            if line[:1] in (" ", "\t", "#", "}", "/", "*", ""):
                continue
            m = FUNC_RE.match(line)
            if not m:
                continue
            func = m.group(1)
            if func in ("if", "for", "while", "switch", "return", "sizeof",
                        "catch", "do", "else"):
                continue
            cm = CLASS_RE.search(line)
            cls = cm.group(1) if cm else ""
            if cm:
                func = cm.group(2)
            # skip declarations (prototypes) only in .hpp? keep both, mark kind.
            kind = "decl" if line.rstrip().endswith(";") else "def"
            if kind == "decl" and not fname.endswith(".hpp"):
                continue
            if kind == "decl" and fname.endswith(".hpp"):
                # declarations duplicate definitions; keep only classes' decls
                if not cls:
                    continue
            items.append({
                "file": fname,
                "line": i + 1,
                "class": cls,
                "name": func,
                "kind": kind,
                "sig": line.strip().rstrip("{").strip(),
                "doc": doc_comment(lines, i)[:160],
                "category": "hud" if is_hud(fname, func) else "required",
            })
    # de-duplicate (definition wins over declaration)
    best = {}
    for it in items:
        key = (it["class"], it["name"], it["sig"].split("(")[0])
        cur = best.get(key)
        if cur is None or (cur["kind"] == "decl" and it["kind"] == "def"):
            best[key] = it
    return sorted(best.values(), key=lambda x: (x["file"], x["line"]))


DEFINE_RE = re.compile(r"^#define\s+([A-Za-z_]\w*)\s+(.+)$")
ENUM_RE = re.compile(r"^enum(?:\s+class)?\s+([A-Za-z_]\w*)?")
ENUM_ITEM_RE = re.compile(r"^\s*([A-Za-z_]\w*)\s*(?:=\s*([^,]+))?,")


def extract_defines():
    items = []
    for fname in sorted(os.listdir(SRC)):
        if not fname.endswith((".hpp",)):
            continue
        path = os.path.join(SRC, fname)
        lines = read(path).splitlines()
        in_enum = None
        for i, line in enumerate(lines):
            dm = DEFINE_RE.match(line)
            if dm:
                name, val = dm.group(1), dm.group(2).strip()
                if "(" in name:  # function-like macro
                    continue
                items.append({
                    "file": fname, "line": i + 1, "kind": "define",
                    "name": name, "value": val[:80],
                })
                continue
            em = ENUM_RE.match(line)
            if em:
                in_enum = em.group(1) or "anonymous"
                continue
            if in_enum:
                if line.strip().startswith("}"):
                    in_enum = None
                    continue
                im = ENUM_ITEM_RE.match(line)
                if im:
                    items.append({
                        "file": fname, "line": i + 1, "kind": "enum",
                        "name": im.group(1),
                        "value": (im.group(2) or "").strip()[:80],
                    })
    return items


def extract_data_legacy():
    items = []
    for fname in sorted(os.listdir(EDIT)):
        path = os.path.join(EDIT, fname)
        lines = read(path).splitlines()
        tags = {}
        records = 0
        samples = []
        for i, line in enumerate(lines):
            if not line or line[0] in ("#", " "):
                continue
            if len(line) > 1 and line[1] == ":":
                tag = line[0]
                tags[tag] = tags.get(tag, 0) + 1
                if tag == "N":
                    records += 1
                    if len(samples) < 3:
                        samples.append((i + 1, line[:110]))
        items.append({
            "file": fname,
            "lines": len(lines),
            "records": records,
            "tags": tags,
            "samples": samples,
        })
    return items


def main():
    os.makedirs(OUT, exist_ok=True)
    import parse_base_data
    methods = extract_methods()
    defines = extract_defines()
    data = parse_base_data.build()

    with open(os.path.join(OUT, "index.json"), "w") as f:
        json.dump({"methods": methods, "defines": defines, "data": data},
                  f, indent=1)

    with open(os.path.join(OUT, "methods.md"), "w") as f:
        f.write("# 原版方法/函数清单（audit2）\n\n")
        f.write("> 证据规则：`impl:` 指向新版实现（文件:行），`test:` 必须是\n")
        f.write("> bevy/src 中真实存在的 #[test]；两者缺一不算完成。`logic:` 为\n")
        f.write("> 逻辑比对结论（供人工复核）。hud 类由用户豁免。\n\n")
        for m in methods:
            if m["category"] == "hud":
                continue
            args = re.search(r"\(.*", m["sig"])
            arg_s = args.group(0)[:80] if args else "()"
            f.write(f"- [ ] `{m['file']}:{m['line']}` **{m['name']}**"
                    f"{arg_s} — {m['doc']}\n")
        f.write("\n## HUD 豁免（列出备查，不要求逐行等价）\n\n")
        for m in methods:
            if m["category"] != "hud":
                continue
            args = re.search(r"\(.*", m["sig"])
            arg_s = args.group(0)[:80] if args else "()"
            f.write(f"- [~] `{m['file']}:{m['line']}` **{m['name']}**"
                    f"{arg_s} — {m['doc']}\n")

    with open(os.path.join(OUT, "defines.md"), "w") as f:
        f.write("# 原版枚举/常量清单（audit2）\n\n")
        for d in defines:
            f.write(f"- [ ] `{d['file']}:{d['line']}` **{d['name']}** = "
                    f"{d['value']}\n")


    req = [m for m in methods if m["category"] == "required"]
    print(f"methods: {len(methods)} (required {len(req)}, hud {len(methods) - len(req)})")
    print(f"defines/enums: {len(defines)}")
    print(f"data files: {len(data)}, items: {sum(d['items'] for d in data)}")
    print("written to", OUT)


if __name__ == "__main__":
    main()
