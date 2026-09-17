#!/usr/bin/env python3
"""Generate per-record checklists for every lib/edit data file.

Output: .spec/base-audit/enums/10..20-*.md
Each record: id, name and all non-description fields, ready for audit annotation.
"""
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
EDIT = os.path.join(ROOT, "lib", "edit")
OUT = os.path.join(ROOT, ".spec", "base-audit", "enums")
os.makedirs(OUT, exist_ok=True)


def read(p):
    with open(p, "r", errors="replace") as f:
        return f.read()


def split_records(path, start_re, skip_prefixes=("D",)):
    """Yield (id, name, fields) where fields maps prefix -> list of values."""
    txt = read(path)
    records = []
    cur = None
    for line in txt.splitlines():
        if not line or line.startswith("#"):
            continue
        m = re.match(start_re, line)
        if m:
            if cur:
                records.append(cur)
            cur = {"id": m.group("id"), "name": m.group("name"), "fields": {}}
            continue
        if cur is None:
            continue
        fm = re.match(r"^([A-Za-z@$?%#]):(.*)$", line)
        if fm:
            pfx, val = fm.group(1), fm.group(2)
            if pfx in skip_prefixes:
                continue
            cur["fields"].setdefault(pfx, []).append(val)
        else:
            # raw map body line (v_info) or continuation
            cur["fields"].setdefault("_raw", []).append(line.strip())
    if cur:
        records.append(cur)
    return records


def fmt(fields, exclude=(), maxlen=220):
    parts = []
    for pfx in sorted(fields):
        if pfx in exclude or pfx == "_raw":
            continue
        vals = fields[pfx]
        joined = ";".join(v for v in vals)
        parts.append(f"{pfx}:{joined}")
    s = " ".join(parts)
    if len(s) > maxlen:
        s = s[:maxlen] + "…"
    return s


def write(name, content):
    with open(os.path.join(OUT, name), "w") as f:
        f.write(content + "\n")
    print(name, len(content.splitlines()), "lines")


HEADER = ("自动生成的数据枚举清单。逐条对照 bevy 的 RON/代码标注：\n"
          "`[x]`=该记录已进 RON 且其特殊行为已消费；`[>]`=进 RON 但行为部分缺失；"
          "`[ ]`=缺失；`[~]`=数据死条目/Theme/前端。\n\n")

PREFIX = re.compile(r"^PREFIX:(?P<id>[^:]*):(?P<name>.*)$")
N_PREFIX = re.compile(r"^N:(?P<id>[^:]*):(?P<name>.*)$")

# ---- f_info ----
recs = split_records(os.path.join(EDIT, "f_info.txt"), N_PREFIX)
out = [HEADER + f"# f_info.txt — terrain features ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `f:{r['id']}` **{r['name']}** — {fmt(r['fields'])}")
write("10-f_info.md", "\n".join(out))

# ---- r_info ----
recs = split_records(os.path.join(EDIT, "r_info.txt"), N_PREFIX)
out = [HEADER + f"# r_info.txt — monsters ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `r:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=400)}")
write("11-r_info.md", "\n".join(out))

# ---- k_info ----
recs = split_records(os.path.join(EDIT, "k_info.txt"), N_PREFIX)
out = [HEADER + f"# k_info.txt — object kinds ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `k:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=300)}")
write("12-k_info.md", "\n".join(out))

# ---- a_info ----
recs = split_records(os.path.join(EDIT, "a_info.txt"), N_PREFIX)
out = [HEADER + f"# a_info.txt — artifacts ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `a:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=300)}")
write("13-a_info.md", "\n".join(out))

# ---- e_info ----
recs = split_records(os.path.join(EDIT, "e_info.txt"), N_PREFIX)
out = [HEADER + f"# e_info.txt — ego items ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `e:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=300)}")
write("14-e_info.md", "\n".join(out))

# ---- p_info (races / subraces / classes / specs) ----
txt = read(os.path.join(EDIT, "p_info.txt"))
blocks = {"races": [], "subraces": [], "classes": [], "specs": [], "general": []}
cur = None
for line in txt.splitlines():
    if not line or line.startswith("#"):
        continue
    m = re.match(r"^R:N:(?P<id>[^:]*):(?P<name>.*)$", line)
    if m:
        cur = ("races", m)
        blocks["races"].append({"id": m.group("id"), "name": m.group("name"), "fields": {}})
        continue
    m = re.match(r"^S:N:(?P<id>[^:]*):(?P<name>.*)$", line)
    if m:
        cur = ("subraces", m)
        blocks["subraces"].append({"id": m.group("id"), "name": m.group("name"), "fields": {}})
        continue
    m = re.match(r"^C:a:N:(?P<id>[^:]*):(?P<name>.*)$", line)
    if m:
        cur = ("specs", m)
        blocks["specs"].append({"id": m.group("id"), "name": m.group("name"), "fields": {}})
        continue
    m = re.match(r"^C:N:(?P<id>[^:]*):(?P<name>.*)$", line)
    if m:
        cur = ("classes", m)
        blocks["classes"].append({"id": m.group("id"), "name": m.group("name"), "fields": {}})
        continue
    m = re.match(r"^G:k:(.*)$", line)
    if m:
        blocks["general"].append({"id": m.group(1).split(":")[2] if len(m.group(1).split(":")) > 2 else "?", "name": m.group(1), "fields": {}})
        continue
    if cur and re.match(r"^[A-Za-z]", line):
        pfx, _, val = line.partition(":")
        blocks[cur[0]][-1]["fields"].setdefault(pfx, []).append(val)
out = [HEADER + "# p_info.txt — races / subraces / classes / specs", ""]
for key in ("general", "races", "subraces", "classes", "specs"):
    out.append(f"## {key} ({len(blocks[key])})")
    out.append("")
    for r in blocks[key]:
        out.append(f"- [ ] `{key}:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=250)}")
    out.append("")
write("15-p_info.md", "\n".join(out))

# ---- d_info ----
recs = split_records(os.path.join(EDIT, "d_info.txt"), N_PREFIX)
out = [HEADER + f"# d_info.txt — dungeons ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `d:{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=300)}")
write("16-d_info.md", "\n".join(out))

# ---- v_info ----
recs = split_records(os.path.join(EDIT, "v_info.txt"), N_PREFIX, skip_prefixes=("D",))
out = [HEADER + f"# v_info.txt — vaults ({len(recs)})", ""]
for r in recs:
    out.append(f"- [ ] `v:{r['id']}` **{r['name']}** — {fmt(r['fields'])}")
write("17-v_info.md", "\n".join(out))

# ---- st_info / ba_info / ab_info / s_info ----
for fname, label, start in (
    ("st_info.txt", "stores", N_PREFIX),
    ("ba_info.txt", "building actions", N_PREFIX),
    ("ab_info.txt", "abilities", N_PREFIX),
    ("s_info.txt", "skills", N_PREFIX),
):
    recs = split_records(os.path.join(EDIT, fname), start)
    out = [HEADER + f"# {fname} — {label} ({len(recs)})", ""]
    for r in recs:
        out.append(f"- [ ] `{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=250)}")
    write(f"18-{fname.replace('.txt','')}.md", "\n".join(out))

# ---- ow_info / re_info / set_info / ra_info ----
for fname, label in (
    ("ow_info.txt", "store owners"),
    ("re_info.txt", "ego monsters"),
    ("set_info.txt", "artifact sets"),
    ("ra_info.txt", "randart parts"),
):
    frec = N_PREFIX if fname != "ra_info.txt" else re.compile(r"^N:(?P<id>[^:]*)(?::(?P<name>.*))?$")
    recs = split_records(os.path.join(EDIT, fname), frec)
    out = [HEADER + f"# {fname} — {label} ({len(recs)})", ""]
    for r in recs:
        out.append(f"- [ ] `{r['id']}` **{r['name']}** — {fmt(r['fields'], maxlen=250)}")
    write(f"19-{fname.replace('.txt','')}.md", "\n".join(out))

# ---- w_info / wf_info / t_info + maps ----
out = [HEADER + "# world / towns / quest maps", ""]
w = read(os.path.join(EDIT, "w_info.txt"))
out.append(f"## w_info.txt — world map")
out.append("")
for line in w.splitlines():
    if line.startswith(("W:D", "W:E", "W:P", "W:M")):
        out.append(f"- [ ] `w` {line[:160]}")
out.append("")
wf = read(os.path.join(EDIT, "wf_info.txt"))
out.append(f"## wf_info.txt — wilderness terrain ({len(re.findall('^W:', wf, re.M))})")
out.append("")
for line in wf.splitlines():
    if line.startswith(("W:", "X:")):
        out.append(f"- [ ] `wf` {line[:160]}")
out.append("")
ti = read(os.path.join(EDIT, "t_info.txt"))
out.append("## t_info.txt — town pref actions")
out.append("")
for line in ti.splitlines():
    if line.startswith(("%:", "?:", "F:", "D:", "P:", "L:")):
        out.append(f"- [ ] `t` {line[:160]}")
out.append("")
out.append("## map files")
out.append("")
for fn in sorted(os.listdir(EDIT)):
    if fn.endswith(".map"):
        body = read(os.path.join(EDIT, fn))
        out.append(f"- [ ] `{fn}` — {len(body.splitlines())} lines")
write("20-world-towns-maps.md", "\n".join(out))

print("done")
