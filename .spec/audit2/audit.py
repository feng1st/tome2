#!/usr/bin/env python3
"""audit2 严格验收：只认机器可复核的证据，不认任何自述。

证据规则：
  methods:  - [x] 必须同时含
              `impl:bevy/src/<file>.rs:<line>`   （文件存在、行号有效）
              `test:<模块::测试名>`             （bevy/src 中真实存在的 #[test]）
            缺一不算完成；`logic:` 只是给人看的比对结论，不作为证据。
            hud 类（用户豁免）不参与判定。
  defines:  - [x] 要求常量名出现在 bevy/src；纯数字值时要求同一行出现该值。
  data:     - [x] 要求对应转换产物存在且记录数不少于原版
            （映射表见 DATA_MAP；未映射的保持未完成）。

退出码：0 = ACCEPTED（0 未决），1 = REJECTED。
"""
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
BEVY = os.path.join(ROOT, "bevy", "src")
ASSETS = os.path.join(ROOT, "bevy", "assets", "data")
OUT = os.path.join(ROOT, ".spec", "audit2", "checklist")
INDEX = os.path.join(OUT, "index.json")

DATA_MAP = {
    "r_info.txt": "monsters.ron",
    "k_info.txt": "items.ron",
    "f_info.txt": "terrain.ron",
    "e_info.txt": "egos.ron",
    "a_info.txt": "artifacts.ron",
    "ra_info.txt": "randarts.ron",
    "re_info.txt": "monster_egos.ron",
    "s_info.txt": "schools.ron",
    "d_info.txt": "dungeons.ron",
    "p_info.txt": None,  # races/classes/general_skills (multiple)
    "ow_info.txt": "owners.ron",
    "set_info.txt": "sets.ron",
    "ba_info.txt": "building_actions.ron",
    "q_info.txt": "questmaps.ron",
    "t_info.txt": None,  # bree.ron + world.ron
    "b_info.txt": "bree.ron",
    "misc.txt": None,
    "readme.txt": None,
    "world.txt": "world.ron",
}


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def bevy_text():
    out = []
    for f in sorted(os.listdir(BEVY)):
        if f.endswith(".rs"):
            out.append(read(os.path.join(BEVY, f)))
    return "\n".join(out)


def rust_tests(text):
    names = set()
    for m in re.finditer(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([A-Za-z_]\w*)", text):
        names.add(m.group(1))
    return names


def parse_bullets(path):
    return [l for l in read(path).splitlines() if l.startswith("- [")]


def check_methods(text, tests):
    lines = parse_bullets(os.path.join(OUT, "methods.md"))
    total = done = 0
    bad = []
    for l in lines:
        if l.startswith("- [~]"):
            continue  # hud exemption
        total += 1
        m = re.match(r"- \[( |x)\] `([^`]+):(\d+)` \*\*([^*]+)\*\*", l)
        if not m:
            bad.append(f"MALFORMED {l[:90]}")
            continue
        status, src, ln, name = m.group(1), m.group(2), int(m.group(3)), m.group(4)
        if status != "x":
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
        tname = test.group(1).split("::")[-1]
        if tname not in tests:
            bad.append(f"BAD-TEST {src}:{ln} {name} -> {test.group(1)}")
            continue
        done += 1
    return total, done, bad


def check_defines(text):
    lines = parse_bullets(os.path.join(OUT, "defines.md"))
    total = done = 0
    bad = []
    for l in lines:
        m = re.match(r"- \[( |x)\] `([^`]+):(\d+)` \*\*([A-Za-z_]\w*)\*\* = (.+)$", l)
        if not m:
            bad.append(f"MALFORMED {l[:90]}")
            continue
        status, src, ln, name, val = m.groups()
        total += 1
        if status != "x":
            bad.append(f"OPEN {src}:{ln} {name}")
            continue
        if not re.search(r"\b" + re.escape(name) + r"\b", text):
            bad.append(f"MISSING {src}:{ln} {name}")
            continue
        v = val.strip()
        if re.fullmatch(r"-?\d+", v):
            # numbers may be negative/large: word-boundary lookarounds that
            # do not require a word char adjacent to a leading '-'
            pat = re.compile(r"\b" + re.escape(name) + r"\b[^\n]*(?<![\w.])"
                             + re.escape(v) + r"(?![\w.])")
            if not pat.search(text):
                bad.append(f"VALUE-MISMATCH {src}:{ln} {name}={v}")
                continue
        done += 1
    return total, done, bad


def ron_count(path):
    if not os.path.exists(path):
        return -1
    txt = read(path)
    # entries are tuples "( ... )" at line starts
    return len(re.findall(r"^\s*\(", txt, re.M))


TOWN_FILES = {"t_bree.txt", "t_gondol.txt", "t_minas.txt", "t_lorien.txt",
              "t_khazad.txt"}


VERIFIED_SOURCES = {
    # source txt -> verifier name in verify_data.json (record-id based)
    "lib/edit/r_info.txt": "r_info.txt",
    "lib/edit/k_info.txt": "k_info.txt",
    "lib/edit/f_info.txt": "f_info.txt",
    "lib/edit/e_info.txt": "e_info.txt",
    "lib/edit/a_info.txt": "a_info.txt",
    "lib/edit/re_info.txt": "re_info.txt",
    "lib/edit/d_info.txt": "d_info.txt",
    "lib/edit/ow_info.txt": "ow_info.txt",
    "lib/edit/set_info.txt": "set_info.txt",
    "lib/edit/ba_info.txt": "ba_info.txt",
    "lib/edit/s_info.txt": "s_info.txt",
    "lib/edit/st_info.txt": "st_info.txt",
    "lib/edit/v_info.txt": "v_info.txt",
    "lib/edit/wf_info.txt": "wf_info.txt",
    "lib/edit/ab_info.txt": "ab_info.txt",
    "lib/edit/ra_info.txt": "ra_info.txt",
    "lib/edit/p_info.txt": "p_info.txt",
    "lib/edit/w_info.txt": "w_info.txt",
    "lib/edit/misc.txt": "misc.txt",
    "lib/edit/t_pref.txt": "t_pref.txt",
    "lib/edit/t_info.txt": "t_info.txt",
    "lib/edit/numenor.txt": "asset_copy:numenor.txt",
    "lib/edit/volcano.txt": "asset_copy:volcano.txt",
    "lib/edit/special.txt": "special.txt",
}


def check_data():
    """A data item is done only when a machine verifier reports its record
    MATCH.  Record ids come from the parse_base_data detail checklists."""
    try:
        import verify_data
        verify_data.main()
    except Exception as e:  # verifier must run cleanly
        return 0, 0, [f"VERIFY-ERROR {e}"]
    with open(os.path.join(OUT, "data_verify.json")) as f:
        verified = json.load(f)
    with open(os.path.join(OUT, "data_index.json")) as f:
        index = json.load(f)
    total = done = 0
    bad = []
    bad_by_file = {}
    for e in index:
        if e.get("category") in ("asset", "placeholder", "hud", "doc"):
            continue  # listed but exempt-pending (fonts/music/placeholders)
        rel = e["file"]
        detail = os.path.join(OUT, e["detail"])
        lines = [l for l in open(detail) if l.startswith("- [")]
        total += len(lines)
        ver = VERIFIED_SOURCES.get(rel)
        opened = 0
        # Same-named text assets (lib/file/*.txt) compare line-for-line.
        text_ok = False
        if rel.startswith("lib/file/"):
            name = os.path.basename(rel)
            text_ok = verified.get("_text_assets", {}).get(name) == "MATCH"
        elif rel.startswith("lib/help/"):
            name = os.path.basename(rel)
            text_ok = verified.get("_help", {}).get(name) == "MATCH"
        for l in lines:
            m = re.search(r"id=(\d+)", l)
            ok = False
            name = os.path.basename(rel)
            m3 = re.search(r":(\d+)`", l)
            line_key = f"{name}:{m3.group(1)}" if m3 else None
            if rel == "lib/pref/colors.prf":
                ok = (bool(m3) and verified.get("_colors_prf", {}).get(m3.group(1))
                      == "MATCH")
            elif rel.endswith(".map"):
                ok = (line_key is not None
                      and verified.get("maps", {}).get(line_key) == "MATCH")
            elif name in TOWN_FILES or name.startswith("t_d_"):
                ok = (line_key is not None
                      and verified.get("towns", {}).get(line_key) == "MATCH")
            elif ver and m:
                ok = verified.get(ver, {}).get(str(int(m.group(1)))) == "MATCH"
            elif ver:
                ok = (line_key is not None
                      and verified.get(ver, {}).get(m3.group(1)) == "MATCH")
            elif text_ok:
                ok = True
            if ok:
                done += 1
            else:
                opened += 1
        if opened:
            bad_by_file[rel] = opened
    for rel, n in sorted(bad_by_file.items()):
        bad.append(f"OPEN-DATA {rel}: {n} items")
    return total, done, bad


def main():
    if not os.path.exists(INDEX):
        print("run extract_base.py first")
        return 1
    text = bevy_text()
    tests = rust_tests(text)
    mt, md, mb = check_methods(text, tests)
    dt, dd, db = check_defines(text)
    at, ad, ab = check_data()
    bad = mb + db + ab
    print(f"methods: {md}/{mt}   defines: {dd}/{dt}   data: {ad}/{at}")
    print(f"violations: {len(bad)}")
    for v in bad[:40]:
        print("  -", v)
    if len(bad) > 40:
        print(f"  ... {len(bad) - 40} more")
    print("VERDICT:", "ACCEPTED" if not bad else "REJECTED")
    return 0 if not bad else 1


if __name__ == "__main__":
    sys.exit(main())
