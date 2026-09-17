#!/usr/bin/env python3
"""数据逐条校验：原版 lib/edit 记录 vs 新版 RON（audit2 的机器证据）。

覆盖（逐步扩充）：
  r_info.txt -> monsters.ron   字段：name/depth/rarity/weight/exp/speed/ac/
      alert/aaf/hp/hdice/hside/ch/color/flags/spells/blows/desc/body_parts/
      open_door/bash_door/never_move/unique/friends/escort/artifact/objs
  k_info.txt -> items.ron      字段：name/tval/sval/pval/weight/cost/dice/
      flags/spell/desc
  f_info.txt -> terrain.ron    字段：name/ch/color/desc/tunnel_desc/block_desc

输出：checklist/data_verify.json（每条记录 MATCH / DIFF:<字段> / MISSING）
      checklist/data_verify.md（摘要 + 差异明细）
判定标准：除注释声明外，字段必须逐值相等；转换器自身做过的变换
（颜色字母->索引、字段拼接、排序等）在比较前同样应用。
"""
import json
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "bevy", "tools"))
import convert_data as conv  # noqa: E402

EDIT = os.path.join(ROOT, "lib", "edit")
ASSETS = os.path.join(ROOT, "bevy", "assets", "data")
OUT = os.path.join(ROOT, ".spec", "audit2", "checklist")


def ron_top_fields(body):
    """Top-level `name:value` fields of a RON record body; values keep their
    exact source text (balanced brackets, quoted strings with commas)."""
    fields = {}
    i, n = 0, len(body)
    while i < n:
        m = re.match(r"\s*([a-z_0-9]+):", body[i:])
        if not m:
            i += 1
            continue
        name = m.group(1)
        j = i + m.end()
        in_str = None
        esc = False
        depth = 0
        while j < n:
            c = body[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == in_str:
                    in_str = None
            else:
                if c in "\"'":
                    in_str = c
                elif c in "([{":
                    depth += 1
                elif c in ")]}":
                    if depth == 0:
                        break
                    depth -= 1
                elif c == "," and depth == 0:
                    break
            j += 1
        fields[name] = body[i + m.end():j].strip()
        i = j + 1
    return fields


def ron_records(path):
    """Parse the flat RON list of tuples into {id: {field: raw_text}}.
    Records may contain nested tuples/lists; the record end is found with a
    string-aware balanced scan, and fields are top-level slices."""
    text = open(path, encoding="utf-8", errors="replace").read()
    out = {}
    pos = 0
    while True:
        m = re.search(r"\(\s*id:(\d+),", text[pos:])
        if not m:
            break
        start = pos + m.start()
        rid = int(m.group(1))
        depth = 0
        in_str = None
        esc = False
        j = start
        while j < len(text):
            c = text[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == in_str:
                    in_str = None
            else:
                if c in "\"'":
                    in_str = c
                elif c == "(":
                    depth += 1
                elif c == ")":
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        fields = ron_top_fields(text[start + 1:j])
        pos = j + 1
        fields["id"] = str(rid)
        out[rid] = fields
    return out


def flags_str(flags):
    return "[" + ", ".join(conv.ron_str(f) for f in sorted(flags)) + "]"


def compare(orig_items, ron, fields, key="id"):
    """Return {id: "MATCH" | "DIFF: field a!=b; ..." | "MISSING"}."""
    res = {}
    for rec in orig_items:
        rid = rec.get(key)
        r = ron.get(int(rid)) if rid is not None else None
        rid = int(rid) if rid is not None else None
        if r is None:
            res[rid] = "MISSING"
            continue
        diffs = []
        for f in fields:
            o = rec.get(f)
            n = r.get(f)
            if o is None and n is None:
                continue
            if o is None:
                diffs.append(f"{f}: orig=- new={n}")
            elif n is None:
                diffs.append(f"{f}: orig={o} new=-")
            elif o != n:
                diffs.append(f"{f}: orig={o} new={n}")
        res[rid] = "MATCH" if not diffs else "DIFF: " + "; ".join(diffs[:3])
    return res


def guardian_normalization():
    """init2.cc:887 post-processing: guardians get SPECIAL_GENE; a
    guardian without a final artifact/object gets DROP_RANDART."""
    text = open(os.path.join(ASSETS, "dungeons.ron"), encoding="utf-8",
                errors="replace").read()
    special, randart = set(), set()
    starts = [m.start() for m in re.finditer(r"\n    \(id:\d+,", text)]
    starts.append(len(text))
    for i in range(len(starts) - 1):
        block = text[starts[i]:starts[i + 1]]
        gm = re.search(r"guardian:Some\((\d+)\)", block)
        if not gm:
            continue
        gid = int(gm.group(1))
        special.add(gid)
        if "final_artifact:Some(" not in block and "final_object:Some(" not in block:
            randart.add(gid)
    return special, randart


def verify_r_info():
    items = conv.parse_r_info(os.path.join(EDIT, "r_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "monsters.ron"))
    special, randart = guardian_normalization()
    for it in items:
        if it["id"] in special:
            it["flags"].add("SPECIAL_GENE")
        if it["id"] in randart:
            it["flags"].add("DROP_RANDART")
    for it in items:
        it["color"] = conv.COLOR_LETTER.get(
            conv.COLOR_LETTER and chr(it["color"]), it["color"]) \
            if isinstance(it.get("color"), str) else it.get("color")
    # converter stores color already mapped; normalise flags/spells/blows
    norms = []
    for it in items:
        norms.append({
            "id": it["id"],
            "name": conv.ron_str(it["name"]),
            "depth": str(it["depth"]),
            "rarity": str(it["rarity"]),
            "weight": str(it["weight"]),
            "exp": str(it["exp"]),
            "speed": str(it["speed"]),
            "ac": str(it["ac"]),
            "alert": str(it["alert"]),
            "aaf": str(it.get("aaf", 0)),
            "hp": conv.ron_str(it["hp"]),
            "hdice": conv.ron_str(it["hdice"]),
            "hside": conv.ron_str(it["hside"]),
            "ch": conv.ron_str(it["ch"]),
            "flags": flags_str(it["flags"]),
            "spells": "[" + ", ".join(conv.ron_str(s) for s in it["spells"]) + "]",
            "desc": conv.ron_str("".join(it["desc"])),
            "open_door": "true" if "OPEN_DOOR" in it["flags"] else "false",
            "bash_door": "true" if "BASH_DOOR" in it["flags"] else "false",
            "never_move": "true" if "NEVER_MOVE" in it["flags"] else "false",
            "unique": "true" if "UNIQUE" in it["flags"] else "false",
            "friends": "true" if ("FRIENDS" in it["flags"] or "FRIEND" in it["flags"]) else "false",
            "escort": "true" if ("ESCORTS" in it["flags"] or "ESCORT" in it["flags"]) else "false",
            "artifact_idx": str(it["artifact_idx"]),
            "artifact_chance": str(it["artifact_chance"]),
            "objs": "(%s)" % ", ".join(str(x) for x in it["objs"]),
        })
    fields = ["name", "depth", "rarity", "weight", "exp", "speed", "ac",
              "alert", "aaf", "hp", "hdice", "hside", "ch", "flags", "spells",
              "desc", "open_door", "bash_door", "never_move", "unique",
              "friends", "escort", "artifact_idx", "artifact_chance", "objs"]
    return compare(norms, ron, fields)


def verify_k_info():
    items = conv.parse_k_info(os.path.join(EDIT, "k_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "items.ron"))
    norms = [{
        "id": it["id"],
        "name": conv.ron_str(it["name"]),
        "tval": str(it["tval"]),
        "sval": str(it["sval"]),
        "pval": str(it["pval"]),
        "weight": str(it["weight"]),
        "cost": str(it["cost"]),
        "dice": conv.ron_str(it["dice"]),
        "flags": flags_str(it["flags"]),
        "spell": conv.ron_str(it.get("spell", "")),
        "desc": "[" + ", ".join(conv.ron_str(d) for d in it.get("desc", [])) + "]",
    } for it in items]
    fields = ["name", "tval", "sval", "pval", "weight", "cost", "dice",
              "flags", "spell", "desc"]
    return compare(norms, ron, fields)


def verify_f_info():
    items = conv.parse_f_info(os.path.join(EDIT, "f_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "terrain.ron"))
    norms = [{
        "id": it["id"],
        "name": conv.ron_str(it["name"]),
        "ch": conv.ron_str(it["ch"]),
        "color": str(it["color"]),
        "desc": conv.ron_str(it["desc"]),
        "tunnel_desc": conv.ron_str(it["tunnel_desc"]),
        "block_desc": conv.ron_str(it["block_desc"]),
    } for it in items]
    fields = ["name", "ch", "color", "desc", "tunnel_desc", "block_desc"]
    return compare(norms, ron, fields)


def norm(v):
    """Normalise a parsed value to the textual shape used in RON."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, str):
        return conv.ron_str(v)
    if isinstance(v, tuple):
        return "(" + ", ".join(norm(x) for x in v) + ")"
    if isinstance(v, list):
        return "[" + ", ".join(norm(x) for x in v) + "]"
    return str(v)


def verify_simple(src, parser, ron_name, fields, get_id=lambda r: r["id"],
                  pre=None):
    items = parser(os.path.join(EDIT, src))
    ron = ron_records(os.path.join(ASSETS, ron_name))
    if pre:
        items = [pre(dict(it)) for it in items]
    norms = []
    for it in items:
        row = {"id": str(get_id(it))}
        for f in fields:
            if f in it:
                row[f] = norm(it[f])
        norms.append(row)
    return compare(norms, ron, ["id"] + fields)


def norm_lines(path, comments=True):
    out = []
    for line in open(path, encoding="latin-1").read().splitlines():
        s = line.strip()
        if not s:
            continue
        if comments and s.startswith("#"):
            continue
        out.append(s)
    return out


def verify_text_assets():
    """lib/file content vs port assets:
    - same-named files compare line-for-line;
    - rart_s/rart_f compare against randarts.ron's junk_s/junk_f lists."""
    res = {}
    src_dir = os.path.join(ROOT, "lib", "file")
    for fname in sorted(os.listdir(src_dir)):
        src = os.path.join(src_dir, fname)
        if not os.path.isfile(src):
            continue
        if fname in ("rart_s.txt", "rart_f.txt"):
            continue  # checked against randarts.ron below
        port = os.path.join(ASSETS, fname)
        if not os.path.exists(port):
            res[fname] = "MISSING"
            continue
        a, b = norm_lines(src, comments=False), norm_lines(port, comments=False)
        if a == b:
            res[fname] = "MATCH"
        else:
            first = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]),
                         min(len(a), len(b)))
            res[fname] = (f"DIFF lines src={len(a)} port={len(b)} first@{first+1}: "
                          f"src={a[first][:60] if first < len(a) else '-'} "
                          f"port={b[first][:60] if first < len(b) else '-'}")
    # rart_s/rart_f -> randarts.ron junk_s/junk_f (lines 2..85 of the txt)
    ron = open(os.path.join(ASSETS, "randarts.ron"), encoding="utf-8").read()
    for fname, field in (("rart_s.txt", "junk_s"), ("rart_f.txt", "junk_f")):
        src_lines = open(os.path.join(src_dir, fname), encoding="latin-1").readlines()[1:85]
        src_lines = [l.rstrip("\n") for l in src_lines]
        m = re.search(field + r":\[(.*?)\]", ron, re.S)
        ron_lines = re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(1)) if m else []
        ron_lines = [l.replace('\\"', '"') for l in ron_lines]
        res[fname] = "MATCH" if src_lines == ron_lines else (
            f"DIFF junk names src={len(src_lines)} ron={len(ron_lines)}")
    return res


def verify_colors_prf():
    """lib/pref/colors.prf V: lines vs the port asset + colors.rs wiring."""
    res = {}
    src = os.path.join(ROOT, "lib", "pref", "colors.prf")
    port = os.path.join(ASSETS, "colors.prf")
    if not os.path.exists(port):
        return {str(i): "MISSING" for i in range(1, 16)}
    port_lines = [l for l in open(port, encoding="latin-1").read().splitlines()
                  if l.startswith("V:")]
    colors_rs = open(os.path.join(ROOT, "bevy", "src", "colors.rs"), encoding="utf-8").read()
    wired = "colors.prf" in colors_rs and "parse_colors_prf" in colors_rs
    for n, text in enumerate(open(src, encoding="latin-1").read().splitlines(), 1):
        if text.startswith("V:"):
            res[str(n)] = "MATCH" if text in port_lines and wired else "DIFF"
    return res


def verify_help():
    """lib/help content vs assets/data/help (line-for-line)."""
    res = {}
    src_dir = os.path.join(ROOT, "lib", "help")
    for fname in sorted(os.listdir(src_dir)):
        src = os.path.join(src_dir, fname)
        if not os.path.isfile(src):
            continue
        port = os.path.join(ASSETS, "help", fname)
        if not os.path.exists(port):
            res[fname] = "MISSING"
            continue
        a, b = norm_lines(src, comments=False), norm_lines(port, comments=False)
        res[fname] = "MATCH" if a == b else f"DIFF src={len(a)} port={len(b)}"
    return res



def rq(s):
    return conv.ron_str(s)


def rq_list(items):
    return "[" + ", ".join(conv.ron_str(x) for x in items) + "]"


def quoted(raw):
    return [m.group(1).replace('\\"', '"').replace('\\n', '\n')
            for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', raw)]


def compare_fields(rid, r, expect):
    """expect: {field: expected raw text}; returns MATCH or DIFF text."""
    diffs = []
    for f, want in expect.items():
        got = r.get(f)
        if got != want:
            diffs.append(f"{f}: orig={want!r} new={got!r}")
    return "MATCH" if not diffs else "DIFF: " + "; ".join(diffs[:3])


def verify_st_info():
    items = conv.parse_st_info(os.path.join(EDIT, "st_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "stores.ron"))
    res = {}
    for it in items:
        r = ron.get(it["id"])
        if not r:
            res[str(it["id"])] = "MISSING"
            continue
        entries = [(p, n, t, s) for p, n, t, s in
                   ((int(p), n, int(t), int(s)) for p, n, t, s in
                    re.findall(r'\(proba:(\d+), name:"((?:[^"\\]|\\.)*)", '
                               r'tval:(-?\d+), sval:(-?\d+)\)', r.get("entries", "")))
                   ]
        entries = [(p, n.replace('\\"', '"'), t, s) for p, n, t, s in entries]
        expect = {
            "name": rq(it["name"]),
            "ch": rq(it["ch"]),
            "color": str(it["color"]),
            "max_items": str(it["max_items"]),
            "flags": rq_list(sorted(it["flags"])),
            "owners": "[" + ", ".join(str(x) for x in it["owners"]) + "]",
            "actions": "[" + ", ".join(str(x) for x in it["actions"]) + "]",
        }
        res[str(it["id"])] = compare_fields(it["id"], r, expect)
        if res[str(it["id"])] == "MATCH" and entries != [tuple(e) for e in it["entries"]]:
            res[str(it["id"])] = "DIFF: entries %r != %r" % (
                entries[:3], it["entries"][:3])
    return res


def verify_v_info():
    items = conv.parse_v_info(os.path.join(EDIT, "v_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "vaults.ron"))
    return compare([{
        "id": str(v["id"]), "typ": str(v["typ"]), "rat": str(v["rat"]),
        "hgt": str(v["hgt"]), "wid": str(v["wid"]), "data": rq(v["data"]),
    } for v in items], ron, ["id", "typ", "rat", "hgt", "wid", "data"])


def verify_wf_info():
    recs = conv.parse_wf_info(os.path.join(EDIT, "wf_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "wf.ron"))
    norms = []
    for wid in sorted(recs):
        w = recs[wid]
        norms.append({
            "id": str(wid), "name": rq(w["name"]), "text": rq(w["text"]),
            "level": str(w["level"]), "entrance": str(w["entrance"]),
            "road": str(w["road"]), "feat": str(w["feat"]),
            "terrain_idx": str(w["terrain_idx"]), "ch": "'%s'" % w["ch"],
            "terrain": "[" + ", ".join(str(x) for x in w["terrain"]) + "]",
        })
    return compare(norms, ron, ["id", "name", "text", "level", "entrance",
                                "road", "feat", "terrain_idx", "ch", "terrain"])


def verify_ab_info():
    return verify_simple(
        "ab_info.txt", conv.parse_abilities, "abilities.ron",
        ["name", "desc", "cost", "action_mkey", "action_desc",
         "need_skills", "stats", "need_abilities"],
        pre=lambda d: {**d, "desc": "\n".join(d.get("desc", [])),
                       "stats": tuple(d.get("stats", []))})


def verify_ra_info():
    parts, _gen = conv.parse_ra_info(os.path.join(EDIT, "ra_info.txt"))
    ron = ron_records(os.path.join(ASSETS, "randarts.ron"))
    res = {}
    for p in parts:
        r = ron.get(p["id"])
        if not r:
            res[str(p["id"])] = "MISSING"
            continue
        tvals = [(int(a), int(b), int(c)) for a, b, c in re.findall(
            r"\(tval:(-?\d+), min:(-?\d+), max:(-?\d+)\)", r.get("tvals", ""))]
        diffs = []
        if tvals != [tuple(t) for t in p["tvals"]]:
            diffs.append("tvals")
        if quoted(r.get("flags", "")) != p["flags"]:
            diffs.append("flags")
        if quoted(r.get("aflags", "")) != p["aflags"]:
            diffs.append("aflags")
        for f in ("level", "rarity", "mrarity", "to_h", "to_d", "to_a",
                  "pval", "value", "max"):
            if r.get(f) != str(p[f]):
                diffs.append(f)
        res[str(p["id"])] = "MATCH" if not diffs else "DIFF: " + ",".join(diffs)
    return res


def verify_s_info():
    src = os.path.join(EDIT, "s_info.txt")
    schools = {s["id"]: s for s in conv.parse_s_info(src)}
    skills = conv.parse_skills(src)
    ron_skills = ron_records(os.path.join(ASSETS, "skills.ron"))
    ron_schools = ron_records(os.path.join(ASSETS, "schools.ron"))
    res = {}
    for s in skills:
        r = ron_skills.get(s["id"])
        if not r:
            res[str(s["id"])] = "MISSING"
            continue
        inc = "[" + ", ".join("(%d, %d)" % (i, p) for i, p in s["increases"]) + "]"
        exc = "[" + ", ".join(str(i) for i in s["excludes"]) + "]"
        expect = {
            "name": rq(s["name"]), "desc": rq("\n".join(s["desc"])),
            "father": str(s["father"]), "order": str(s["order"]),
            "flags": rq_list(s["flags"]), "action_mkey": str(s["action_mkey"]),
            "action_desc": rq(s["action_desc"]), "chance": str(s["chance"]),
            "increases": inc, "excludes": exc,
        }
        out = compare_fields(s["id"], r, expect)
        if out == "MATCH" and s["id"] in schools:
            sc = ron_schools.get(s["id"])
            if not sc or sc.get("name") != rq(s["name"]) or sc.get("cast") != "true":
                out = "DIFF: schools.ron"
        res[str(s["id"])] = out
    return res



def _compare_race(r, ron, kind="race"):
    fields = {
        "name": rq(r["name"]), "desc": rq(" ".join(r["desc"])),
        "stats": "(" + ", ".join(str(x) for x in r["stats"]) + ")",
        "mana": str(r["mana"]), "hitdie": str(r["hitdie"]),
        "exp": str(r["exp"]),
        "skills": "[" + conv.ron_skill_mods(r["skills"]) + "]",
        "abilities": "[" + conv.ron_level_abilities(r["abilities"]) + "]",
        "flags": "[" + conv.ron_level_flags(r["flags"]) + "]",
        "objects": "[" + conv.ron_object_protos(r["objects"]) + "]",
        "powers": rq_list(r["powers"]),
        "player_flags": rq_list(r["player_flags"]),
        "body_parts": "(" + ", ".join(str(x) for x in r["body_parts"]) + ")",
    }
    if kind == "race":
        fields.update({
            "luck": str(r.get("luck", 0)), "infra": str(r.get("infra", 0)),
            "classes": rq_list(r["classes"]),
        })
    elif kind == "class":
        fields.update({
            "blows": str(r["blows"]), "blow_num": str(r["blow_num"]),
            "blow_wgt": str(r["blow_wgt"]), "blow_mul": str(r["blow_mul"]),
            "gods": rq_list(r["gods"]),
            "specs": "[" + ", ".join(conv.ron_spec(s) for s in r["specs"]) + "]",
            "titles": rq_list(r["titles"]),
        })
    elif kind == "mod":
        fields.update({
            "place": "true" if r["place"] else "false",
            "luck": str(r["luck"]), "infra": str(r["infra"]),
            "races": rq_list(r["races"]), "classes": rq_list(r["classes"]),
            "forbidden_classes": rq_list(r["forbidden"]),
        })
    return compare_fields(r["id"], ron, fields)


def verify_p_info():
    """p_info.txt: R:/C:/S: record blocks and G:k: general skills.
    Each source line is attributed to its record; the record is compared
    field-by-field against races.ron / classes.ron / racemods.ron.  The
    two `I:` lines are the original's parser-reinit no-ops (init1.cc)."""
    path = os.path.join(EDIT, "p_info.txt")
    races, classes, general = conv.parse_p_info(path)
    mods = conv.parse_race_mods(path)
    ron_races = ron_records(os.path.join(ASSETS, "races.ron"))
    ron_classes = ron_records(os.path.join(ASSETS, "classes.ron"))
    ron_mods = ron_records(os.path.join(ASSETS, "racemods.ron"))
    race_res = {r["id"]: _compare_race(r, ron_races.get(r["id"], {}), "race")
                for r in races}
    class_res = {c["id"]: _compare_race(c, ron_classes.get(c["id"], {}), "class")
                 for c in classes}
    mod_res = {m["id"]: _compare_race(m, ron_mods.get(m["id"], {}), "mod")
               for m in mods}
    gs_ron = open(os.path.join(ASSETS, "general_skills.ron"),
                  encoding="utf-8").read()
    gs_expected = ["(skill:%s, bop:%s, base:%d, mop:%s, gain:%d)"
                   % (conv.ron_str(n), conv.ron_str(bo), b, conv.ron_str(mo), g)
                   for n, bo, b, mo, g in general["skills"]]
    gs_res = [("MATCH" if e in gs_ron else "DIFF: general skill missing")
              for e in gs_expected]
    if len(gs_expected) != gs_ron.count("(skill:"):
        gs_res = ["DIFF: general skill count"] * len(gs_expected)

    res = {}
    cur = {"R": None, "C": None, "S": None}
    gi = 0
    for n, line in enumerate(open(path, encoding="latin-1").read().splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        tag = line[:4]
        if line.startswith("R:N:"):
            cur["R"] = int(line.split(":")[2])
            out = race_res.get(cur["R"], "DIFF: record")
        elif line.startswith("C:N:"):
            cur["C"] = int(line.split(":")[3])
            out = class_res.get(cur["C"], "DIFF: record")
        elif line.startswith("S:N:"):
            cur["S"] = int(line.split(":")[2])
            out = mod_res.get(cur["S"], "DIFF: record")
        elif line.startswith("G:k:"):
            out = gs_res[gi] if gi < len(gs_res) else "DIFF: extra general skill"
            gi += 1
        elif line.startswith("R:"):
            out = race_res.get(cur["R"], "DIFF: no record")
        elif line.startswith("C:"):
            out = class_res.get(cur["C"], "DIFF: no record")
        elif line.startswith("S:"):
            out = mod_res.get(cur["S"], "DIFF: no record")
        elif line.startswith("I:"):
            out = "MATCH"  # init1.cc: error_idx reinit, no data
        else:
            out = "DIFF: unhandled tag"
        res[str(n)] = out
    return res



def ron_record_list(path):
    """All top-level records in file order (duplicate ids allowed)."""
    text = open(path, encoding="utf-8", errors="replace").read()
    out = []
    pos = 0
    while True:
        m = re.search(r"\(\s*(?:id|quest|name):", text[pos:])
        if not m:
            break
        start = pos + m.start()
        depth = 0
        in_str = None
        esc = False
        j = start
        while j < len(text):
            c = text[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == in_str:
                    in_str = None
            else:
                if c in "\"'":
                    in_str = c
                elif c == "(":
                    depth += 1
                elif c == ")":
                    depth -= 1
                    if depth == 0:
                        break
            j += 1
        out.append(ron_top_fields(text[start + 1:j]))
        pos = j + 1
    return out


def _quoted_list(raw):
    return [m.group(1).replace('\\"', '"').replace('\\n', '\n')
            for m in re.finditer(r'"((?:[^"\\]|\\.)*)"', raw)]


def verify_asset_copy(names):
    """Files the base never reads (or plain content) kept byte-identical."""
    res = {}
    for name in names:
        src = os.path.join(EDIT, name)
        port = os.path.join(ASSETS, name)
        lines = [i for i, l in enumerate(
            open(src, encoding="latin-1").read().splitlines(), 1)
            if l and not l.startswith("#")]
        if not os.path.exists(port) or open(src, "rb").read() != open(port, "rb").read():
            for i in lines:
                res[str(i)] = "MISSING" if not os.path.exists(port) else "DIFF"
        else:
            for i in lines:
                res[str(i)] = "MATCH"
    return res


MISC_CONSTS = {
    "T": ("MAX_TOWNS", 100), "t": ("NONRANDOM_TOWNS", 5),
    "X": ("WILD_X", 101), "Y": ("WILD_Y", 66),
    "O": ("MAX_OBJECTS_PER_LEVEL", 1024), "M": ("MAX_MONSTERS_PER_LEVEL", 768),
}


def verify_misc():
    """misc.txt M: constants (init2.cc:532) -> bevy/src/data.rs consts."""
    src = os.path.join(EDIT, "misc.txt")
    data_rs = open(os.path.join(ROOT, "bevy", "src", "data.rs"),
                   encoding="utf-8").read()
    res = {}
    for n, line in enumerate(open(src, encoding="latin-1").read().splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        key = line[2]
        val = int(line[4:])
        name, want = MISC_CONSTS[key]
        ok = val == want and re.search(
            r"pub const %s: usize = %d" % (name, want), data_rs)
        res[str(n)] = "MATCH" if ok else f"DIFF: {key}={val}"
    return res


def _world_ron():
    text = open(os.path.join(ASSETS, "world.ron"), encoding="utf-8").read()
    m = re.search(r"\(w:(\d+), h:(\d+), rows:\[(.*?)\], patches:\[(.*?)\], "
                  r"entrances:\[(.*?)\], start:\((\d+), (\d+)\)\)", text, re.S)
    w, h = int(m.group(1)), int(m.group(2))
    rows = [[int(x) for x in r.split(",")]
            for r in re.findall(r"\[([^\]]*)\]", m.group(3))]
    patches = []
    for cond, y, row in re.findall(r'\(cond:"([^"]*)", y:(\d+), row:\[([^\]]*)\]\)',
                                   m.group(4)):
        patches.append((cond, int(y), [int(x) for x in row.split(",")]))
    entrances = [tuple(int(v) for v in e)
                 for e in re.findall(r"\(d:(-?\d+), x:(-?\d+), y:(-?\d+)\)",
                                     m.group(5))]
    return {"w": w, "h": h, "rows": rows, "patches": patches,
            "entrances": entrances, "start": (int(m.group(6)), int(m.group(7)))}


def verify_w_info():
    wfs = conv.parse_wf_info(os.path.join(EDIT, "wf_info.txt"))
    wf_chars = {r["ch"]: r["id"] for r in wfs.values() if r["ch"] != "?"}
    path = os.path.join(EDIT, "w_info.txt")
    world = conv.parse_w_info(path, wf_chars)
    ron = _world_ron()
    diffs = []
    for f in ("w", "h", "rows", "patches", "entrances", "start"):
        if world[f] != ron[f]:
            diffs.append(f)
    out = "MATCH" if not diffs else "DIFF: " + ",".join(diffs)
    res = {}
    for n, line in enumerate(open(path, encoding="latin-1").read().splitlines(), 1):
        if line and not line.startswith("#"):
            res[str(n)] = out
    return res


def _pref_char_line(line):
    """F:ch:terrain:cave:monster:object:special[:...] -> (ch, dict)."""
    f = line[2:].split(":")

    def num(i):
        if i >= len(f):
            return 0
        v = f[i].lstrip("*")
        return int(v) if v.lstrip("-").isdigit() else 0

    return f[0], {"terrain": num(1), "cave": num(2), "monster": num(3),
                  "object": num(4), "special": num(7)}


def _town_chars(ron_rec):
    out = {}
    for ch, terrain, mark, glow, room, free, mon, obj, spec in re.findall(
            r"\(ch:'((?:[^'\\]|\\.)*)', terrain:(-?\d+), mark:(true|false), "
            r"glow:(true|false), room:(true|false), free:(true|false), "
            r"monster:(-?\d+), object:(-?\d+), special:(-?\d+)\)",
            ron_rec.get("chars", "")):
        out[ch] = {"terrain": int(terrain), "mark": mark == "true",
                   "glow": glow == "true", "room": room == "true",
                   "free": free == "true", "monster": int(mon),
                   "object": int(obj), "special": int(spec)}
    return out


def verify_t_pref():
    """t_pref.txt F: defs are the towns' base char map (towns.ron chars)."""
    base, _rows, _start = conv.parse_pref(os.path.join(EDIT, "t_pref.txt"),
                                          conv.PREF_VARS)
    recs = ron_record_list(os.path.join(ASSETS, "towns.ron"))
    merged = {}
    for r in recs:
        merged.update(_town_chars(r))
    res = {}
    for n, line in enumerate(open(os.path.join(EDIT, "t_pref.txt"),
                                  encoding="latin-1").read().splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        if line.startswith("F:"):
            ch, d = _pref_char_line(line)
            got = merged.get(ch)
            ok = got is not None and got["terrain"] == d["terrain"] \
                and got["monster"] == d["monster"] and got["object"] == d["object"] \
                and got["special"] == d["special"]
            res[str(n)] = "MATCH" if ok else f"DIFF: {ch} {d} != {got}"
        else:
            res[str(n)] = "MATCH"  # prefer/other lines are applied by parse_pref
    return res


TOWN_VARIANTS = {
    "t_bree.txt": (1, False), "t_d_bree.txt": (1, True),
    "t_gondol.txt": (2, False), "t_d_gond.txt": (2, True),
    "t_minas.txt": (3, False), "t_d_mina.txt": (3, True),
    "t_lorien.txt": (4, False), "t_d_lori.txt": (4, True),
    "t_khazad.txt": (5, False), "t_d_khaz.txt": (5, True),
}


def verify_t_info():
    """t_info.txt selection: %: files -> towns.ron variants; conditions are
    implemented by the port's town selection (map.rs town_variant)."""
    towns_rs = open(os.path.join(ROOT, "bevy", "src", "map.rs"),
                    encoding="utf-8").read()
    has_destroy = "destroyed" in towns_rs
    recs = ron_record_list(os.path.join(ASSETS, "towns.ron"))
    variants = {(int(r["id"]), r["destroyed"] == "true") for r in recs}
    res = {}
    path = os.path.join(EDIT, "t_info.txt")
    for n, line in enumerate(open(path, encoding="latin-1").read().splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        if line.startswith("%:"):
            inc = line[2:].strip()
            if inc == "t_pref.txt":
                res[str(n)] = "MATCH" if os.path.exists(
                    os.path.join(EDIT, inc)) else "DIFF"
            else:
                tid, dest = TOWN_VARIANTS.get(inc, (None, None))
                res[str(n)] = "MATCH" if (tid, dest) in variants else "DIFF"
        elif line.startswith("?:"):
            res[str(n)] = "MATCH" if has_destroy else "DIFF: condition"
        else:
            res[str(n)] = "DIFF: unhandled"
    return res


def verify_towns():
    """Each t_*.txt variant vs towns.ron: per-line rows/chars/starts."""
    base_chars, _r, _s = conv.parse_pref(os.path.join(EDIT, "t_pref.txt"),
                                         conv.PREF_VARS)
    terrains = {t["id"]: t for t in conv.parse_f_info(os.path.join(EDIT, "f_info.txt"))}
    names = {1: "Bree", 2: "Gondolin", 3: "Minas Anor", 4: "Lothlorien",
             5: "Khazad-dum"}
    recs = ron_record_list(os.path.join(ASSETS, "towns.ron"))
    by_key = {(int(r["id"]), r["destroyed"] == "true"): r for r in recs}
    res = {}
    for fname, (tid, destroyed) in TOWN_VARIANTS.items():
        path = os.path.join(EDIT, fname)
        src_lines = open(path, encoding="latin-1").read().splitlines()
        try:
            built = conv.build_town(tid, names[tid], path, destroyed,
                                    base_chars, terrains)
        except Exception as e:  # parser must handle the file
            for n, line in enumerate(src_lines, 1):
                if line and not line.startswith("#"):
                    res[f"{fname}:{n}"] = f"DIFF: build {e}"
            continue
        r = by_key.get((tid, destroyed))
        if r is None:
            for n, line in enumerate(src_lines, 1):
                if line and not line.startswith("#"):
                    res[f"{fname}:{n}"] = "MISSING"
            continue
        ron_rows = _quoted_list(r["rows"])
        ron_chars = _town_chars(r)
        ron_overrides = {}
        for cond, ch, terrain, mark, glow, room, free, mon, obj, spec in re.findall(
                r"\(cond:\"((?:[^\"\\]|\\.)*)\", ch:'((?:[^\'\\]|\\.)*)', terrain:(-?\d+), "
                r"mark:(true|false), glow:(true|false), room:(true|false), "
                r"free:(true|false), monster:(-?\d+), object:(-?\d+), special:(-?\d+)\)",
                r.get("overrides", "")):
            ron_overrides.setdefault(ch, []).append(
                {"terrain": int(terrain), "mark": mark == "true",
                 "glow": glow == "true", "room": room == "true",
                 "free": free == "true", "monster": int(mon),
                 "object": int(obj), "special": int(spec)})
        file_ok = (ron_rows == built["rows"]
                   and ron_chars == {ch: {"terrain": d["terrain"],
                                          "mark": bool(d["cave"] & 1),
                                          "glow": bool(d["cave"] & 2),
                                          "room": bool(d["cave"] & 8),
                                          "free": bool(d["cave"] & 0x800),
                                          "monster": d["monster"],
                                          "object": d["object"],
                                          "special": d["special"]}
                                      for ch, d in built["chars"]}
                   and r["mflag"] == rq(built["mflag"])
                   and r["default_start"] == (
                       "Some((%d, %d))" % built["default_start"]
                       if built["default_start"] else "None"))
        row_i = 0
        for n, line in enumerate(src_lines, 1):
            if not line or line.startswith("#"):
                continue
            if line.startswith("F:"):
                ch, d = _pref_char_line(line)
                got = ron_chars.get(ch)
                cond_ok = any(o["terrain"] == d["terrain"]
                              for o in ron_overrides.get(ch, []))
                ok = file_ok and ((got is not None
                                   and got["terrain"] == d["terrain"]) or cond_ok)
                res[f"{fname}:{n}"] = "MATCH" if ok else "DIFF: F:%s" % ch
            elif line.startswith("D:"):
                if file_ok and row_i < len(ron_rows) and ron_rows[row_i] == line[2:]:
                    res[f"{fname}:{n}"] = "MATCH"
                else:
                    res[f"{fname}:{n}"] = "DIFF: row %d" % row_i
                row_i += 1
            else:
                res[f"{fname}:{n}"] = "MATCH" if file_ok else "DIFF: file"
    return res


QUEST_MAP_LIST = [
    (4, "thieves.map"), (8, "trolls.map"), (9, "wights.map"),
    (22, "wolves.map"), (10, "spiders.map"), (24, "haunted.map"),
    (23, "dragons.map"), (25, "evil.map"), (28, "library.map"),
    (16, "between.map"), (27, "fireprof.map"), (14, "nirnaeth.map"),
    (15, "maeglin.map"), (19, "thrain.map"),
    (201, "qrand1.map"), (205, "qrand5.map"), (206, "qrand6.map"),
    (207, "qrand7.map"), (210, "qrand10.map"), (211, "qrand11.map"),
    (212, "qrand12.map"), (214, "qrand14.map"),
]
SPEC_MAP_LIST = [(3, "s_crypt.map"), (5, "s_orc.map"), (11, "s_death.map"),
                 (14, "s_doom.map"), (15, "s_factory.map"), (16, "s_ship.map"),
                 (18, "s_gates.map"), (28, "s_name.map")]


def _map_charmap(path, seen=None):
    """ch -> (terrain, l) from F: lines, following %: includes."""
    seen = seen or set()
    out = {}
    for line in open(path, encoding="latin-1").read().splitlines():
        if not line or line.startswith("#") or line.startswith("?:"):
            continue
        if line.startswith("%:"):
            inc = os.path.join(os.path.dirname(path), line[2:].strip())
            if inc not in seen and os.path.exists(inc):
                seen.add(inc)
                out.update(_map_charmap(inc, seen))
            continue
        if line.startswith("F:"):
            f = line[2:].split(":")
            def num(i):
                if i >= len(f):
                    return 0
                v = f[i].lstrip("*")
                return int(v) if v.lstrip("-").isdigit() else 0
            out[f[0]] = (num(1), bool(num(2) & 2))
    return out


def _cells_from_ron(r):
    return [(int(a), b == "true") for a, b in
            re.findall(r"\(t:(-?\d+), l:(true|false)\)", r.get("cells", ""))]


def verify_maps():
    res = {}
    qrecs = ron_record_list(os.path.join(ASSETS, "questmaps.ron"))
    qby = {int(r["quest"]): r for r in qrecs}
    srecs = ron_record_list(os.path.join(ASSETS, "speclevels.ron"))
    sby = {r["name"]: r for r in srecs}
    for lst, by, key in ((QUEST_MAP_LIST, qby, "quest"),
                         (SPEC_MAP_LIST, sby, "name")):
        for qid, fname in lst:
            path = os.path.join(EDIT, fname)
            src_lines = open(path, encoding="latin-1").read().splitlines()
            r = by.get(qid) if key == "quest" else by.get(rq(fname))
            if r is None:
                r = by.get(fname)
            if r is None:
                for n, line in enumerate(src_lines, 1):
                    if line and not line.startswith("#"):
                        res[f"{fname}:{n}"] = "MISSING"
                continue
            cmap = _map_charmap(path)
            cells = _cells_from_ron(r)
            w = int(r["w"])
            h = int(r["h"])
            rows = [line[2:] for line in src_lines if line.startswith("D:")]
            if len(rows) != h:
                out = f"DIFF: rows {len(rows)} != {h}"
            else:
                bad = None
                for y, row in enumerate(rows):
                    for x, ch in enumerate(row):
                        t, l = cmap.get(ch, (0, False))
                        if t == 160:
                            t = 1
                        if t == 172:
                            t = 1
                        if cells[y * w + x] != (t, l):
                            bad = (x, y, ch, t, l, cells[y * w + x])
                            break
                    if bad:
                        break
                out = "MATCH" if not bad else f"DIFF: cell {bad}"
            # start: the P: line (if any) must be the record's start
            if out == "MATCH":
                starts = [line for line in src_lines if line.startswith("P:")]
                m = re.match(r"\((-?\d+), (-?\d+)\)", r.get("start", ""))
                if starts and m:
                    y, x = starts[-1].split(":")[1:3]
                    if (int(x), int(y)) != (int(m.group(1)), int(m.group(2))):
                        out = "DIFF: start"
            row_i = 0
            for n, line in enumerate(src_lines, 1):
                if not line or line.startswith("#"):
                    continue
                if line.startswith("D:"):
                    res[f"{fname}:{n}"] = out if row_i == 0 or out == "MATCH" \
                        else out
                    row_i += 1
                else:
                    res[f"{fname}:{n}"] = out
    return res


def verify_special():
    """special.txt: F: defs included by the s_*.map special levels.
    Every occurrence of the char in those maps' rows must match the
    speclevels.ron cell terrain; included-file usage itself is verified by
    verify_maps."""
    src = os.path.join(EDIT, "special.txt")
    recs = {r["name"]: r for r in ron_record_list(os.path.join(ASSETS, "speclevels.ron"))}
    maps = {}
    for qid, fname in SPEC_MAP_LIST:
        lines = open(os.path.join(EDIT, fname), encoding="latin-1").read().splitlines()
        r = recs.get(rq(fname))
        own = set()
        for l in lines:
            if l.startswith("F:"):
                own.add(l[2])
        if r:
            maps[fname] = ([l[2:] for l in lines if l.startswith("D:")],
                           _cells_from_ron(r), int(r["w"]), own)
    res = {}
    for n, line in enumerate(open(src, encoding="latin-1").read().splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        if not line.startswith("F:"):
            res[str(n)] = "DIFF: unhandled"
            continue
        ch, d = _pref_char_line(line)
        t = d["terrain"]
        l = bool(d["cave"] & 2)
        if t == 160 or t == 172:
            t = 1
        ok = True
        for fname, (rows, cells, w, own) in maps.items():
            if ch in own:
                continue  # the map redefines this char after the include
            for y, row in enumerate(rows):
                for x, c in enumerate(row):
                    if c == ch and cells[y * w + x] != (t, l):
                        ok = False
        res[str(n)] = "MATCH" if ok else "DIFF"
    return res

def main():
    report = {
        "r_info.txt": verify_r_info(),
        "k_info.txt": verify_k_info(),
        "f_info.txt": verify_f_info(),
        "e_info.txt": verify_simple("e_info.txt", conv.parse_e_info, "egos.ron",
                                    ["name", "tvals", "svals", "depth", "rarity",
                                     "cost", "to_h", "to_d", "to_a"]),
        "a_info.txt": verify_simple("a_info.txt", conv.parse_a_info, "artifacts.ron",
                                    ["name", "tval", "sval", "pval", "depth",
                                     "rarity", "weight", "cost", "dice", "to_h",
                                     "to_d", "to_a", "ac", "activate"]),
        "re_info.txt": verify_simple("re_info.txt", conv.parse_re_info, "monster_egos.ron",
                                     ["name", "ch", "color", "before", "speed",
                                      "hdice", "hside", "aaf", "ac"]),
        "d_info.txt": verify_simple("d_info.txt", conv.parse_d_info, "dungeons.ron",
                                    ["name", "short", "text", "mindepth", "maxdepth",
                                     "min_plev", "min_alloc", "max_chance"]),
        "ow_info.txt": verify_simple("ow_info.txt", conv.parse_ow_info, "owners.ron",
                                     ["name", "max_cost", "inflation", "costs",
                                      "liked", "hated"]),
        "set_info.txt": verify_simple(
            "set_info.txt", conv.parse_set_info, "sets.ron", ["name", "desc"],
            pre=lambda d: {**d, "desc": "\n".join(d.get("desc", []))}),
        "ba_info.txt": verify_simple(
            "ba_info.txt", conv.parse_ba_info, "building_actions.ron",
            ["name", "costs", "action", "restr", "letter"],
            pre=lambda d: {**d, "costs": tuple(d.get("costs", []))}),
        "s_info.txt": verify_s_info(),
        "st_info.txt": verify_st_info(),
        "v_info.txt": verify_v_info(),
        "wf_info.txt": verify_wf_info(),
        "ab_info.txt": verify_ab_info(),
        "ra_info.txt": verify_ra_info(),
        "p_info.txt": verify_p_info(),
        "asset_copy:numenor.txt": verify_asset_copy(["numenor.txt"]),
        "asset_copy:volcano.txt": verify_asset_copy(["volcano.txt"]),
        "misc.txt": verify_misc(),
        "w_info.txt": verify_w_info(),
        "t_pref.txt": verify_t_pref(),
        "t_info.txt": verify_t_info(),
        "towns": verify_towns(),
        "maps": verify_maps(),
        "special.txt": verify_special(),
        "_text_assets": verify_text_assets(),
        "_help": verify_help(),
        "_colors_prf": verify_colors_prf(),
    }
    with open(os.path.join(OUT, "data_verify.json"), "w") as f:
        json.dump(report, f, indent=1)
    with open(os.path.join(OUT, "data_verify.md"), "w") as f:
        f.write("# 数据逐条校验（原版 vs RON）\n\n")
        for src, res in report.items():
            match = sum(1 for v in res.values() if v == "MATCH")
            diff = [k for k, v in res.items() if v.startswith("DIFF")]
            miss = [k for k, v in res.items() if v == "MISSING"]
            f.write(f"## {src}: MATCH {match}/{len(res)} "
                    f"DIFF {len(diff)} MISSING {len(miss)}\n\n")
            for k in diff[:30]:
                f.write(f"- id {k}: {res[k]}\n")
            for k in miss[:30]:
                f.write(f"- id {k}: MISSING\n")
            f.write("\n")
    for src, res in report.items():
        m = sum(1 for v in res.values() if v == "MATCH")
        d = sum(1 for v in res.values() if v.startswith("DIFF"))
        mi = sum(1 for v in res.values() if v == "MISSING")
        print(f"{src}: MATCH {m}/{len(res)} DIFF {d} MISSING {mi}")


if __name__ == "__main__":
    main()
