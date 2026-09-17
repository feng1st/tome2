#!/usr/bin/env python3
"""Convert ToME plain-text data files (lib/edit/*.txt) into RON assets
consumed by the Bevy version.

Outputs (under bevy/assets/data/):
  - terrain.ron  : from f_info.txt
  - monsters.ron : from r_info.txt (blow methods/effects, innate spells, door flags)
  - races.ron    : from p_info.txt (R: records)
  - classes.ron  : from p_info.txt (C: records, incl. bonus blows)
  - items.ron    : from k_info.txt (weapons/armor/lites/ammo/bows/devices/food/
                   scrolls/potions/spellbooks)
  - egos.ron     : from e_info.txt (ego types applicable to ported tvals,
                   incl. ability flags)
  - artifacts.ron: from a_info.txt (artifacts on ported tvals, incl. ability
                   flags and a: activation names)
  - schools.ron  : from s_info.txt (magic schools with A:17 "Cast a spell")
  - stores.ron   : from st_info.txt (stores 0-8 + 11: the six classic shops,
                   Black Market, Home, Book Store, Inn)
  - spells.ron   : curated table below; ToME 2.x spells are hardcoded in
                    src/spells*.cc (not data files), so this is a faithful
                    simplification organised by s_info schools
  - bree.ron     : fixed Bree village map from t_info.txt (pref parser below,
                    cropped to 128x64); shops/quest entrances/flavor buildings
  - questmaps.ron: fixed quest levels from thieves.map/trolls.map/wights.map

Field formats (from the headers of the original files):
  f_info:  N:<id>:<name>  G:<char>:<color>  F:<flag>
  r_info:  N:<id>:<name>  G:<char>:<color>
           I:<speed>:<hp dice>:<vision>:<ac>:<alertness>
           W:<depth>:<rarity>:<corpse weight>:<exp>
           B:<method>:<effect>:<dice>
  p_info:  R:N:<idx>:<name>  R:D:<desc>  R:S:str:int:wis:dex:con:chr:mana
           R:P:<hitdie>:<exp%>  R:C:<class name>
           C:N:0:<idx>:<name>  C:D:0:<desc>  C:S:str:int:wis:dex:con:chr:mana:blows
           C:P:<hitdie>:<xp%>
  k_info:  N:<id>:<name>  G:<char>:<color>  I:<tval>:<sval>:<pval>[:<fuel>][:SPELL=name]
           W:<depth>:<rarity>:<weight>:<cost>  P:<ac>:<dice>:<to_h>:<to_d>:<to_a>
  st_info: N:<idx>:<name>  I:<proba>:<item name>  T:<proba>:<tval>:<sval>
           G:<char>:<color>  W:<max items>  F:<flag>
  e_info:  N:<id>:<name>  X:<pos>:<slot>:<rating>  T:<tval>:<min sval>:<max sval>
           W:<depth>:<r1>:<r2>:<cost>  C:<to_h>:<to_d>:<to_a>:<pval>  F:<flag>
  a_info:  N:<id>:<name>  I:<tval>:<sval>:<pval>  W:<depth>:<rarity>:<weight>:<cost>
           P:<ac>:<dice>:<to_h>:<to_d>:<to_a>  a:<activation>  F:<flag>
  r_info extra: S:1_IN_<n> (innate frequency) / S:<spell flag>
           F:OPEN_DOOR/BASH_DOOR/NEVER_MOVE/UNIQUE/FRIENDS/ESCORT
  s_info:  N:<id>:<name>  A:17:Cast a spell
"""
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
TOME_LIB = os.path.normpath(os.path.join(HERE, "..", "..", "lib", "edit"))
TOME_SRC = os.path.normpath(os.path.join(HERE, "..", "..", "src"))
OUT_DIR = os.path.normpath(os.path.join(HERE, "..", "assets", "data"))

COLOR_LETTER = {
    'd': 0, 'w': 1, 's': 2, 'o': 3, 'r': 4, 'g': 5, 'b': 6, 'u': 7,
    'D': 8, 'W': 9, 'v': 10, 'y': 11, 'R': 12, 'G': 13, 'B': 14, 'U': 15,
}


def ron_str(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'


def ron_char(c):
    """A RON single-quoted character literal."""
    if c == "'":
        return '"\'"'
    if c == '\\':
        return "'\\\\'"
    return "'" + c + "'"


def ron_level_flags(fs):
    """(level, pval, flag) tuples -> RON struct list (p_info R:F:/C:F:)."""
    return ', '.join('(level:%d, pval:%d, flag:%s)' % (l, p, ron_str(n))
                     for l, p, n in fs)


def ron_skill_mods(mods):
    """(name, bop, base, mop, gain) tuples -> RON SkillMod list."""
    return ', '.join('(skill:%s, bop:%s, base:%d, mop:%s, gain:%d)'
                     % (ron_str(n), ron_str(bo), b, ron_str(mo), g)
                     for n, bo, b, mo, g in mods)


def ron_level_abilities(abs_):
    """(level, ability) tuples -> RON level-ability list."""
    return ', '.join('(level:%d, ability:%s)' % (l, ron_str(n))
                     for l, n in abs_)


def ron_object_protos(objs):
    """(tval, sval, pval, dd, ds) tuples -> RON ObjectProto list."""
    return ', '.join('(tval:%d, sval:%d, pval:%d, dd:%d, ds:%d)' % o
                     for o in objs)


def ron_spec(s):
    """One C:a: specialisation record -> RON SpecDef."""
    desc = ron_str(' '.join(s['desc']))
    return ('(name:%s, desc:%s, objects:[%s], gods:[%s], skills:[%s], '
            'abilities:[%s])' % (
                ron_str(s['name']), desc, ron_object_protos(s['objects']),
                ', '.join(ron_str(g) for g in s['gods']),
                ron_skill_mods(s['skills']),
                ron_level_abilities(s['abilities'])))


def parse_f_info(path):
    terrains = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                terrains.append(cur)
            _, fid, name = line.split(':', 2)
            cur = {'id': int(fid), 'name': name, 'ch': ' ', 'color': 0,
                   'flags': set(), 'desc': '', 'tunnel_desc': '',
                   'block_desc': '', 'mimic': int(fid), 'effects': []}
        elif cur is None:
            continue
        elif line.startswith('G:'):
            _, ch, color = line.split(':', 2)
            cur['ch'] = ch[0] if ch else ' '
            cur['color'] = COLOR_LETTER.get(color[0] if color else 'd', 0)
        elif line.startswith('D:'):
            # D:<0|1|2>:<text>: look / tunnel / block message
            # (init1.cc init_f_info_txt).
            kind = line[2] if len(line) > 2 else ''
            text = line[4:] if len(line) > 4 else ''
            if kind == '0':
                cur['desc'] = text
            elif kind == '1':
                cur['tunnel_desc'] = text
            elif kind == '2':
                cur['block_desc'] = text
        elif line.startswith('M:'):
            # M:<terrain id>: the feature this one is displayed as.
            cur['mimic'] = int(line[2:].strip())
        elif line.startswith('E:'):
            # E:<dd>d<ds>:<freq>:<type>; freq is stored x10.
            p = line[2:].split(':', 2)
            dd, ds = p[0].split('d', 1)
            freq = int(p[1]) * 10
            typ = p[2] if len(p) > 2 and p[2] else 'NONE'
            cur['effects'].append((int(dd), int(ds), freq, typ))
        elif line.startswith('F:'):
            cur['flags'].add(line[2:].strip())
    if cur:
        terrains.append(cur)
    return terrains


def parse_r_info(path):
    monsters = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur and cur.get('ch') is not None:
                monsters.append(cur)
            _, mid, name = line.split(':', 2)
            cur = {'id': int(mid), 'name': name, 'ch': None, 'color': 1,
                   'speed': 100, 'hp': '1d1', 'hdice': '1', 'hside': '1',
                   'ac': 0, 'alert': 0, 'aaf': 0,
                   'depth': 0, 'rarity': 1, 'weight': 100, 'exp': 0,
                   'blows': [], 'spell_freq': 0, 'spells': [],
                   'artifact_idx': 0, 'artifact_chance': 0, 'objs': [0, 0, 0, 0],
                   'body_parts': [0] * 6, 'desc': [], 'flags': set(),
                   'ok': False}
        elif cur is None:
            continue
        elif line.startswith('G:'):
            _, ch, color = line.split(':', 2)
            cur['ch'] = ch[0] if ch else '?'
            cur['color'] = COLOR_LETTER.get(color[0] if color else 'w', 1)
        elif line.startswith('I:'):
            p = line.split(':')
            cur['speed'] = int(p[1])
            cur['hp'] = p[2]
            if 'd' in p[2]:
                cur['hdice'], cur['hside'] = p[2].split('d', 1)
            cur['aaf'] = int(p[3])
            cur['ac'] = int(p[4])
            cur['alert'] = int(p[5])
        elif line.startswith('W:'):
            p = line.split(':')
            cur['depth'] = int(p[1])
            cur['rarity'] = max(1, int(p[2]))
            # Monsters: W:<depth>:<rarity>:<weight>:<exp> (r_info);
            # only used to size corpse breath effects (cmd6 corpse_effect).
            cur['weight'] = max(1, int(p[3]))
            cur['exp'] = int(p[4]) if len(p) > 4 else 0
        elif line.startswith('B:'):
            p = line[2:].split(':')
            method = p[0]
            effect = p[1] if len(p) > 1 else 'HURT'
            dice = p[2] if len(p) > 2 and re.fullmatch(r'\d+d\d+(\+\d+)?', p[2]) else '0d0'
            if len(cur['blows']) < 4:
                cur['blows'].append((method, effect, dice))
        elif line.startswith('S:'):
            s = line[2:].strip()
            m = re.fullmatch(r'1_IN_(\d+)', s)
            if m:
                cur['spell_freq'] = int(m.group(1))
            elif s:
                cur['spells'].append(s)
        elif line.startswith('E:'):
            # E:<weapon>:<torso>:<arms>:<finger>:<head>:<legs> body parts
            # (init1.cc init_r_info_txt; xtra1.cc calc_body).
            cur['body_parts'] = [int(x) for x in line[2:].split(':')[:6]]
        elif line.startswith('D:'):
            # D:<text>: monster memory/recall flavour text.
            cur['desc'].append(line[2:])
        elif line.startswith('A:'):
            # A:<artifact>:<chance%> -- the standard artifact drop
            # (init1.cc init_r_info; granted on death in xtra2.cc).
            p = line[2:].split(':')
            cur['artifact_idx'] = int(p[0])
            cur['artifact_chance'] = int(p[1]) if len(p) > 1 else 0
        elif line.startswith('O:'):
            # O:<treasure>:<combat>:<magic>:<tools> drop theme (init1.cc).
            p = [int(x) for x in line[2:].split(':')[:4]]
            cur['objs'] = p
        elif line.startswith('F:'):
            for f in line[2:].split('|'):
                cur['flags'].add(f.strip())
    if cur and cur.get('ch') is not None:
        monsters.append(cur)
    return monsters


def skill_modifier(tail):
    """'+1000:+800:Combat' -> (skill, base_op, base, mod_op, gain).

    The leading character is the operator (init1.cc read_skill_modifiers /
    monster_ego_modify): '+' add, '-' subtract, '=' set, '%' percent.
    """
    val, mod, name = tail.split(':', 2)
    return (name, val[0], int(val[1:]), mod[0], int(mod[1:]))


def parse_proto_object(tail):
    """'tval:sval:xdy' or 'tval:sval:pval:xdy' -> object proto tuple
    (init1.cc read_proto_object)."""
    p = tail.split(':')
    if len(p) == 3:
        tval, sval, dice = p
        pval = 0
    elif len(p) == 4:
        tval, sval, pval, dice = p
    else:
        raise ValueError('bad object proto %r' % tail)
    m = re.fullmatch(r'(\d+)d(\d+)', dice)
    if not m:
        raise ValueError('bad object dice %r' % tail)
    return (int(tval), int(sval), int(pval), int(m.group(1)), int(m.group(2)))


def new_spec(name):
    return {'name': name, 'desc': [], 'skills': [], 'abilities': [],
            'objects': [], 'gods': []}


def parse_p_info(path):
    """Returns (races, classes, general).

    races/classes carry their C:/R:S: stats rows, the skill modifier
    (R:k:/C:k:) and granted ability (R:b:/C:b:) lines, the starting
    object protos (R:O:/C:O:) and the class god restriction (C:g:).
    Every C: record carries its own C:a: specialisations, each with
    description, skill modifiers, abilities, objects and god restriction
    (init1.cc init_player_info_txt).
    """
    races, classes = [], []
    general = {'skills': [], 'abilities': []}
    cur = None
    kind = None
    spec = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('R:N:'):
            if kind == 'R' and cur:
                races.append(cur)
            elif kind == 'C' and cur:
                classes.append(cur)
            _, _, idx, name = line.split(':', 3)
            cur = {'id': int(idx), 'name': name, 'desc': [], 'stats': [0] * 6,
                   'mana': 0, 'luck': 0, 'hitdie': 10, 'exp': 100, 'infra': 0,
                   'classes': [], 'skills': [], 'abilities': [], 'flags': [],
                   'objects': [], 'gods': [], 'specs': [], 'powers': [],
                   'player_flags': [], 'lev': 1, 'pval': 0,
                   'body_parts': [0] * 6}
            kind = 'R'
            spec = None
        elif line.startswith('C:N:'):
            if kind == 'R' and cur:
                races.append(cur)
            elif kind == 'C' and cur:
                classes.append(cur)
            p = line.split(':')
            cur = {'id': int(p[3]), 'name': p[4], 'desc': [], 'stats': [0] * 6,
                   'mana': 0, 'hitdie': 9, 'exp': 0, 'blows': 0,
                   'blow_num': 4, 'blow_wgt': 30, 'blow_mul': 3,
                   'skills': [], 'abilities': [], 'flags': [], 'objects': [],
                   'gods': [], 'specs': [], 'powers': [],
                   'player_flags': [], 'titles': [], 'body_parts': [0] * 6,
                   'lev': 1, 'pval': 0}
            kind = 'C'
            spec = None
        elif cur is None and line.startswith('G:k:'):
            general['skills'].append(skill_modifier(line[4:]))
        elif line.startswith('C:a:N:'):
            spec = new_spec(line[6:].strip())
            cur['specs'].append(spec)
        elif spec is not None and line.startswith('C:a:D:'):
            spec['desc'].append(line[6:])
        elif spec is not None and line.startswith('C:a:k:'):
            spec['skills'].append(skill_modifier(line[6:]))
        elif spec is not None and line.startswith('C:a:b:'):
            p = line[6:].split(':', 1)
            spec['abilities'].append((int(p[0]), p[1]))
        elif spec is not None and line.startswith('C:a:O:'):
            spec['objects'].append(parse_proto_object(line[6:]))
        elif spec is not None and line.startswith('C:a:g:'):
            spec['gods'].append(line[6:])
        elif cur is None:
            continue
        elif kind == 'R' and line.startswith('R:D:'):
            cur['desc'].append(line[4:])
        elif kind == 'R' and line.startswith('R:E:'):
            # R:E:<weapon>:<torso>:<arms>:<finger>:<head>:<legs>
            # (init1.cc init_player_info_txt; xtra1.cc calc_body).
            cur['body_parts'] = [int(x) for x in line[4:].split(':')[:6]]
        elif kind == 'C' and line.startswith('C:D:'):
            p = line.split(':')
            if p[2] == '0':
                cur['desc'].append(p[3])
            elif p[2] == '1':
                # C:D:1:<title>: the level title for ranks of five
                # (files.cc class titles, shown in the character sheet).
                cur['titles'].append(line[6:])
        elif kind == 'C' and line.startswith('C:E:'):
            cur['body_parts'] = [int(x) for x in line[4:].split(':')[:6]]
        elif kind == 'R' and line.startswith('R:S:'):
            p = line.split(':')
            cur['stats'] = [int(x) for x in p[2:8]]
            # R:S is str:int:wis:dex:con:chr:luck (no mana; only S:/C: have it).
            cur['luck'] = int(p[8]) if len(p) > 8 else 0
        elif kind == 'C' and line.startswith('C:S:'):
            p = line.split(':')
            cur['stats'] = [int(x) for x in p[2:8]]
            cur['mana'] = int(p[8])
            cur['blows'] = int(p[9]) if len(p) > 9 else 0
        elif kind == 'R' and line.startswith('R:P:'):
            p = line.split(':')
            cur['hitdie'] = int(p[2])
            cur['exp'] = int(p[3])
            cur['infra'] = int(p[4]) if len(p) > 4 else 0
        elif kind == 'C' and line.startswith('C:P:'):
            p = line.split(':')
            cur['hitdie'] = int(p[2])
            cur['exp'] = int(p[3])
        elif kind == 'C' and line.startswith('C:B:'):
            p = line.split(':')
            cur['blow_num'] = int(p[2])
            cur['blow_wgt'] = int(p[3])
            cur['blow_mul'] = int(p[4])
        elif kind == 'R' and line.startswith('R:C:'):
            cur['classes'].append(line[4:])
        elif kind == 'R' and line.startswith('R:k:'):
            cur['skills'].append(skill_modifier(line[4:]))
        elif kind == 'C' and line.startswith('C:k:'):
            cur['skills'].append(skill_modifier(line[4:]))
        elif kind == 'R' and line.startswith('R:b:'):
            p = line[4:].split(':', 1)
            cur['abilities'].append((int(p[0]), p[1]))
        elif kind == 'C' and line.startswith('C:b:'):
            p = line[4:].split(':', 1)
            cur['abilities'].append((int(p[0]), p[1]))
        elif kind == 'C' and line.startswith('C:g:'):
            cur['gods'].append(line[4:])
        elif kind == 'R' and line.startswith('R:G:'):
            cur['player_flags'].extend(
                f.strip() for f in line[4:].replace('|', ' ').split())
        elif kind == 'C' and line.startswith('C:G:'):
            cur['player_flags'].extend(
                f.strip() for f in line[4:].replace('|', ' ').split())
        elif kind == 'R' and line.startswith('R:Z:'):
            cur['powers'].append(line[4:])
        elif kind == 'C' and line.startswith('C:Z:'):
            cur['powers'].append(line[4:])
        elif kind == 'R' and line.startswith('R:O:'):
            cur['objects'].append(parse_proto_object(line[4:]))
        elif kind == 'C' and line.startswith('C:O:'):
            cur['objects'].append(parse_proto_object(line[4:]))
        elif kind == 'R' and line.startswith('R:R:'):
            p = line.split(':')
            cur['lev'] = int(p[2])
            cur['pval'] = int(p[3])
        elif kind == 'C' and line.startswith('C:R:'):
            p = line.split(':')
            cur['lev'] = int(p[2])
            cur['pval'] = int(p[3])
        elif kind == 'R' and line.startswith('R:F:'):
            cur['flags'].append((cur['lev'], cur['pval'], line[4:]))
        elif kind == 'C' and line.startswith('C:F:'):
            cur['flags'].append((cur['lev'], cur['pval'], line[4:]))
    if kind == 'R' and cur:
        races.append(cur)
    elif kind == 'C' and cur:
        classes.append(cur)
    return races, classes, general


def parse_race_mods(path):
    """The base subraces (p_info.txt S: records, init1.cc race_mod_info).

    Each record carries its own stats/luck/mana, hit-die/exp/infra bonus,
    body parts, the allowed races (S:A:), allowed/forbidden classes
    (S:C:A:/S:C:F:), player flags (S:G:), level flags (S:R:/S:F:), skill
    modifiers (S:k:), level abilities (S:b:), granted powers (S:Z:) and
    starting objects (S:O:).
    """
    mods = []
    cur = None
    lev = 1
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('S:N:'):
            if cur:
                mods.append(cur)
            _, _, idx, name = line.split(':', 3)
            cur = {'id': int(idx), 'name': name, 'desc': [], 'place': False,
                   'stats': [0] * 6, 'luck': 0, 'mana': 100, 'hitdie': 0,
                   'exp': 0, 'infra': 0, 'body_parts': [0] * 6,
                   'races': [], 'classes': [], 'forbidden': [], 'skills': [],
                   'abilities': [], 'flags': [], 'objects': [], 'powers': [],
                   'player_flags': [], 'lev': 1, 'pval': 0}
            lev = 1
        elif cur is None:
            continue
        elif line.startswith('S:D:'):
            cur['place'] = line[4] == 'A'
            cur['desc'].append(line[6:])
        elif line.startswith('S:S:'):
            p = line.split(':')
            cur['stats'] = [int(x) for x in p[2:8]]
            cur['luck'] = int(p[8])
            cur['mana'] = int(p[9])
        elif line.startswith('S:P:'):
            p = line.split(':')
            cur['hitdie'] = int(p[2])
            cur['exp'] = int(p[3])
            cur['infra'] = int(p[4])
        elif line.startswith('S:E:'):
            cur['body_parts'] = [int(x) for x in line[4:].split(':')]
        elif line.startswith('S:A:'):
            cur['races'].append(line[4:])
        elif line.startswith('S:C:'):
            target = 'classes' if line[4] == 'A' else 'forbidden'
            cur[target].append(line[6:])
        elif line.startswith('S:G:'):
            cur['player_flags'].extend(
                f.strip() for f in line[4:].replace('|', ' ').split())
        elif line.startswith('S:R:'):
            p = line.split(':')
            lev = int(p[2])
            cur['pval'] = int(p[3])
        elif line.startswith('S:F:'):
            cur['flags'].append((lev, cur['pval'], line[4:]))
        elif line.startswith('S:k:'):
            cur['skills'].append(skill_modifier(line[4:]))
        elif line.startswith('S:b:'):
            p = line[4:].split(':', 1)
            cur['abilities'].append((int(p[0]), p[1]))
        elif line.startswith('S:Z:'):
            cur['powers'].append(line[4:])
        elif line.startswith('S:O:'):
            cur['objects'].append(parse_proto_object(line[4:]))
    if cur:
        mods.append(cur)
    return mods


def clean_name(name):
    """'& Short Sword~' -> 'Short Sword'"""
    return name.replace('&', '').replace('~', '').strip()


# tvals we port: ammo/bows, hafted/polearm/sword/axe, all armor, lites,
# staves/wands/rods, scrolls, potions (71+72), plain food (bread/meat/mold)
# and spellbooks (TV_BOOK 111; sval 0-8/20-24 school tomes, 50 cantrips,
# 255 = random spellbook, resolved at stock generation).  Seventh round adds:
# skeletons(1)/corpses(9)/eggs(10)/junk(11)/tools(12)/parchments(8),
# mage staffs(6)/instruments(14)/boomerangs(15)/digging tools(20),
# amulets(40)/rings(45), oil flasks(77) and gold pieces(100).
# Ninth round adds symbiotic monsters (99) and junkart artifacts (102).
KEEP_TVALS = {1, 2, 5, 6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
              24, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 45, 54, 55, 65, 66,
              67, 70, 71, 72, 77, 80, 99, 100, 102, 111, 114, 115}
KEEP_FOOD_SVALS = set(range(0, 20)) | {32, 33, 35, 36, 37, 38, 39, 40, 41, 42}
# sval 0-19 = mushrooms (quest 18 props, edible), 32/33/35/36 = Hard
# Biscuit / Venison / Ration / Slime Mold, 40 = Sprig of Athelas
# (the Nazgul quest reward).


def parse_k_info(path):
    items = []

    def keep(o):
        if o['id'] == 0:
            return o['name'] == 'something'  # k_info[0] TV_NOTHING dummy
        if o['id'] <= 0:
            return False
        if o['tval'] == 80:
            return o['sval'] in KEEP_FOOD_SVALS
        return o['tval'] in KEEP_TVALS

    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur and keep(cur):
                items.append(cur)
            _, kid, name = line.split(':', 2)
            cur = {'id': int(kid), 'name': clean_name(name), 'ch': '?', 'color': 1,
                   'tval': 0, 'sval': 0, 'pval': 0, 'fuel': 0, 'spell': '',
                   'depth': 0, 'rarity': 1, 'weight': 0, 'cost': 0,
                   'ac': 0, 'dice': '0d0', 'to_h': 0, 'to_d': 0, 'to_a': 0,
                   'flavored': '&' in name, 'activate': '', 'desc': [],
                   'flags': set(), 'alloc': [], 'btval': 0, 'bsval': 0,
                   'ok': False}
        elif cur is None:
            continue
        elif line.startswith('G:'):
            _, ch, color = line.split(':', 2)
            cur['ch'] = ch[0] if ch else '?'
            cur['color'] = COLOR_LETTER.get(color[0] if color else 'w', 1)
        elif line.startswith('I:'):
            p = line.split(':')
            cur['tval'] = int(p[1])
            cur['sval'] = int(p[2])
            cur['pval'] = int(p[3])
            cur['fuel'] = int(p[4]) if len(p) > 4 and p[4].lstrip('-').isdigit() else 0
            for extra in p[4:]:
                if extra.startswith('SPELL='):
                    cur['spell'] = extra[6:]
        elif line.startswith('W:'):
            p = line.split(':')
            cur['depth'] = int(p[1])
            cur['rarity'] = max(1, int(p[2]))
            cur['weight'] = int(p[3])
            cur['cost'] = int(p[4])
        elif line.startswith('A:'):
            # A:<level>[/<chance>]:... allocation pairs (init1.cc).
            for part in line[2:].split(':'):
                if not part:
                    continue
                if '/' in part:
                    lvl, ch = part.split('/', 1)
                    cur['alloc'].append((int(lvl), max(1, int(ch))))
                else:
                    cur['alloc'].append((int(part), 1))
        elif line.startswith('T:'):
            # T:<btval>:<bsval> (the NORM_ART "specific object" marker).
            p = line.split(':')
            if len(p) > 2:
                cur['btval'] = int(p[1])
                cur['bsval'] = int(p[2])
        elif line.startswith('P:'):
            p = line.split(':')
            cur['ac'] = int(p[1])
            cur['dice'] = p[2] if re.fullmatch(r'\d+d\d+', p[2]) else '0d0'
            cur['to_h'] = int(p[3])
            cur['to_d'] = int(p[4])
            cur['to_a'] = int(p[5]) if len(p) > 5 else 0
        elif line.startswith('F:'):
            for f in line[2:].split('|'):
                cur['flags'].add(f.strip())
        elif line.startswith('a:'):
            cur['activate'] = line[2:].strip()
        elif line.startswith('D:'):
            cur['desc'].append(line[2:].strip())
    if cur and keep(cur):
        items.append(cur)
    # Mushrooms display as "Mushroom of X" (k_info only stores the effect
    # name; the original composes it from the tval at description time).
    # Rings/amulets: plain kinds ("Strength", "Speed") display as
    # "Ring of X"; the flavored/full-name kinds (& Ring~ of Precognition,
    # Ring~ of Power) already carry their full name (object1.cc TV_RING /
    # TV_AMULET cases).
    for o in items:
        if o['tval'] == 99 and not o['name']:
            # k_info 653 is "& ~": the name is the carried monster race.
            o['name'] = 'Symbiote'
        if o['tval'] == 80 and 0 <= o['sval'] <= 19:
            o['name'] = 'Mushroom of ' + o['name']
        elif o['tval'] == 45:
            if not o['flavored'] and not o['name'].startswith('Ring'):
                o['name'] = 'Ring of ' + o['name']
        elif o['tval'] == 40:
            if o['flavored'] and not o['name'].startswith('Amulet'):
                o['name'] = o['name'] + ' Amulet'
            elif not o['flavored'] and not o['name'].startswith('Amulet'):
                o['name'] = 'Amulet of ' + o['name']
    return items


# I:<tval>:<sval>:<pval> lines may carry a fuel value and/or SPELL=name.
# a: lines hold artifact activation names (a_info).
# 0-7 classic + Book Store (8) + Pet Shop (9) + Inn (11) + Soothsayer (12)
# + Mathom-house (57) + The Prancing Pony (58, Bree's inn).
# Ninth round adds the professional/guild stores 13-37 (Library, Castle,
# Casino, guild halls, temples, smiths...); only their A: building-action
# list is consumed for now (stock entries are mostly empty).
STORE_IDS = set(range(0, 61)) - {56}


def parse_st_info(path):
    stores = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur is not None and cur['id'] in STORE_IDS:
                stores.append(cur)
            _, sid, name = line.split(':', 2)
            cur = {'id': int(sid), 'name': name, 'ch': '1', 'color': 7,
                   'max_items': 24, 'entries': [], 'flags': set(),
                   'owners': [], 'actions': []}
        elif cur is None:
            continue
        elif line.startswith('A:'):
            # Zero entries are empty slots (init1.cc erases them).
            cur['actions'] = [int(x) for x in line[2:].split(':') if int(x) != 0]
        elif line.startswith('I:'):
            _, proba, name = line.split(':', 2)
            cur['entries'].append((int(proba), clean_name(name), -1, -1))
        elif line.startswith('T:'):
            p = line.split(':')
            cur['entries'].append((int(p[1]), '', int(p[2]), int(p[3])))
        elif line.startswith('F:'):
            cur['flags'].add(line[2:].strip())
        elif line.startswith('G:'):
            _, ch, color = line.split(':', 2)
            cur['ch'] = ch[0] if ch else '1'
            cur['color'] = COLOR_LETTER.get(color[0] if color else 'u', 7)
        elif line.startswith('W:'):
            cur['max_items'] = int(line.split(':')[1])
        elif line.startswith('O:'):
            cur['owners'] = [int(x) for x in line[2:].split(':')]
    if cur is not None and cur['id'] in STORE_IDS:
        stores.append(cur)
    return [s for s in stores if s['id'] in STORE_IDS]


# tvals that ego/artifact generation can apply to (ammo/bow/weapon/armor/
# lite/digging/instrument/mage-staff/boomerang/ring/amulet)
GEAR_TVALS = {6, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 30, 31, 32, 33,
              34, 35, 36, 37, 38, 39, 40, 45, 67}
# make_ego_item (object2.cc:2192) matches any e_info T: tval, and a_m_aux_4
# rolls egos for the default kinds too: staves (55), wands (65), scrolls
# (70) and books (111) have real egos (Fireproof/of Plenty).
EGO_TVALS = GEAR_TVALS | {55, 65, 70, 111}
# Artifacts whose base kind is a daemon book (Demonblade/-shield/-horn,
# "of Gothmog") are wield-cast gear and must survive parsing.
ART_TVALS = GEAR_TVALS | {115}

# ETR_* generation-only ego flags (ego_flag_list.hpp): these are not
# object flags, they drive add_random_ego_flag at creation.
EGO_GENERATION_FLAGS = {
    'SUSTAIN', 'OLD_RESIST', 'ABILITY', 'R_ELEM', 'R_LOW', 'R_HIGH',
    'R_ANY', 'R_DRAGON', 'SLAY_WEAP', 'DAM_DIE', 'DAM_SIZE', 'PVAL_M1',
    'PVAL_M2', 'PVAL_M3', 'PVAL_M5', 'AC_M1', 'AC_M2', 'AC_M3', 'AC_M5',
    'TH_M1', 'TH_M2', 'TH_M3', 'TH_M5', 'TD_M1', 'TD_M2', 'TD_M3',
    'TD_M5', 'R_P_ABILITY', 'R_STAT', 'R_STAT_SUST', 'R_IMMUNITY',
    'LIMIT_BLOWS',
}


def parse_e_info(path):
    egos = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                egos.append(cur)
            _, eid, name = line.split(':', 2)
            cur = {'id': int(eid), 'name': name.strip(), 'pos': 'A',
                   'rating': 0,
                   'tvals': [], 'svals': [], 'depth': 0, 'rarity': 1,
                   'rarity1': 1, 'rarity2': 1, 'cost': 0,
                   'to_h': 0, 'to_d': 0, 'to_a': 0, 'pval': 0,
                   'cursed': False, 'flags': set(), 'powers': [],
                   'need_flags': [], 'forbid_flags': [], 'activate': '',
                   'groups': []}
        elif cur is None:
            continue
        elif line.startswith('X:'):
            p = line.split(':')
            cur['pos'] = p[1]
            if len(p) > 3:
                cur['rating'] = int(p[3])
        elif line.startswith('T:'):
            p = line.split(':')
            tv, minsv, maxsv = int(p[1]), int(p[2]), int(p[3])
            if tv in EGO_TVALS:
                cur['tvals'].append(tv)
                cur['svals'].append((tv, minsv, maxsv))
        elif line.startswith('W:'):
            p = line.split(':')
            cur['depth'] = int(p[1])
            cur['rarity1'] = max(1, int(p[2]))
            cur['rarity2'] = max(1, int(p[3]))
            # The generation weight uses mrarity (rarity2) and rarity1 is
            # the "rarity" in the rand_int(mrarity) > rarity check.
            cur['rarity'] = cur['rarity2']
            cur['cost'] = int(p[4])
        elif line.startswith('C:'):
            p = line.split(':')
            cur['to_h'], cur['to_d'], cur['to_a'], cur['pval'] = \
                (int(p[1]), int(p[2]), int(p[3]), int(p[4]))
        elif line.startswith('R:'):
            cur['groups'].append({'rarity': int(line[2:].split(':')[0].strip()),
                                  'flags': [], 'fego': [], 'oflags': []})
        elif line.startswith('F:'):
            for f in line[2:].split('|'):
                f = f.strip()
                if not f:
                    continue
                if f in ('CURSED', 'AUTO_CURSE'):
                    cur['cursed'] = True
                if cur['groups']:
                    g = cur['groups'][-1]
                    # EgoDef.flags is the always-on subset (magik(100)
                    # groups only); the per-rarity rolls live in `groups`.
                    if g['rarity'] >= 100:
                        cur['flags'].add(f)
                    (g['fego'] if f in EGO_GENERATION_FLAGS
                     else g['flags']).append(f)
                else:
                    # Defensive: flags before any R: are always-on.
                    cur['flags'].add(f)
        elif line.startswith('f:'):
            # Obvious flags: mirror of F: flags the player notices when
            # just looking at the item (e_info oflags).
            for f in line[2:].split('|'):
                f = f.strip()
                if not f:
                    continue
                if cur['groups']:
                    cur['groups'][-1]['oflags'].append(f)
                else:
                    cur['flags'].add(f)
        elif line.startswith('a:'):
            cur['activate'] = line[2:].strip()
        elif line.startswith('r:N:'):
            cur['need_flags'] = [f.strip() for f in line[4:].split('|') if f.strip()]
        elif line.startswith('r:F:'):
            cur['forbid_flags'] = [f.strip() for f in line[4:].split('|') if f.strip()]
        elif line.startswith('Z:'):
            cur['powers'].append(line[2:])
    if cur:
        egos.append(cur)
    return egos


def parse_a_info(path):
    arts = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur and (cur['tval'] in ART_TVALS or cur['id'] == 201):
                arts.append(cur)
            _, aid, name = line.split(':', 2)
            cur = {'id': int(aid), 'name': name.strip(), 'tval': 0, 'sval': 0,
                   'pval': 0, 'depth': 0, 'rarity': 1, 'weight': -1, 'cost': 0,
                   'ac': 0, 'dice': '0d0', 'to_h': 0, 'to_d': 0, 'to_a': 0,
                   'cursed': False, 'insta_art': False, 'activate': '',
                   'flags': set(), 'powers': []}
        elif cur is None:
            continue
        elif line.startswith('I:'):
            p = line.split(':')
            cur['tval'] = int(p[1])
            cur['sval'] = int(p[2])
            cur['pval'] = int(p[3]) if len(p) > 3 else 0
        elif line.startswith('a:'):
            cur['activate'] = line[2:].strip()
        elif line.startswith('W:'):
            p = line.split(':')
            cur['depth'] = int(p[1])
            cur['rarity'] = max(1, int(p[2]))
            cur['weight'] = int(p[3])
            cur['cost'] = int(p[4])
        elif line.startswith('P:'):
            p = line.split(':')
            cur['ac'] = int(p[1])
            cur['dice'] = p[2] if re.fullmatch(r'\d+d\d+(\+\d+)?', p[2]) else '0d0'
            cur['to_h'] = int(p[3])
            cur['to_d'] = int(p[4])
            cur['to_a'] = int(p[5]) if len(p) > 5 else 0
        elif line.startswith('F:'):
            for f in line[2:].split('|'):
                f = f.strip()
                cur['flags'].add(f)
                if f == 'CURSED':
                    cur['cursed'] = True
                elif f == 'INSTA_ART':
                    cur['insta_art'] = True
        elif line.startswith('Z:'):
            cur['powers'].append(line[2:])
    if cur and (cur['tval'] in ART_TVALS or cur['id'] == 201):
        arts.append(cur)
    # INSTA_ART artifacts are quest/script-only (never random drops); the
    # `insta_art` marker lets the game keep them out of random generation
    # while still building them for specific quests (the One Ring).
    return arts



def parse_re_info(path):
    """Monster ego races (re_info.txt). Modifier strings are kept as-is
    ("+5", "-3", "%150", "=10") and applied at spawn time."""
    egos = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                egos.append(cur)
            _, eid, name = line.split(':', 2)
            cur = {'id': int(eid), 'name': name.strip(), 'ch': '', 'color': -1,
                   'before': True, 'speed': '', 'hdice': '', 'hside': '',
                   'aaf': '', 'ac': '', 'sleep': '', 'level': '+0', 'rarity': 1,
                   'weight': '+0', 'mexp': '+0', 'spell_freq': 0,
                   'req_flags': [], 'req_chars': [],
                   'forbid_flags': [], 'forbid_chars': [], 'add_flags': [],
                   'remove_flags': [], 'add_spells': [], 'remove_spells': [],
                   'blows': []}
        elif cur is None:
            continue
        elif line.startswith('G:'):
            p = line.split(':')
            cur['ch'] = '' if p[1] == '*' else p[1]
            cur['color'] = -1 if p[2] == '*' else COLOR_LETTER.get(p[2][0], -1)
        elif line.startswith('I:'):
            # I:<m>spd:<m>hp1d<m>hp2:<m>aaf:<m>ac:<m>slp
            spd, hp, aaf, ac, slp = line[2:].split(':')
            hp1, hp2 = hp.split('d')
            cur['speed'], cur['aaf'], cur['ac'], cur['sleep'] = spd, aaf, ac, slp
            cur['hdice'], cur['hside'] = hp1, hp2
        elif line.startswith('W:'):
            # W:<m>lev:<rar>:<m>wt:<m>exp:<B|A>
            lev, rar, wt, exp, pos = line[2:].split(':')
            cur['level'], cur['rarity'] = lev, max(1, int(rar))
            cur['weight'], cur['mexp'] = wt, exp
            cur['before'] = (pos == 'B')
        elif line.startswith('B:'):
            method, effect, dice = line[2:].split(':')
            dd, ds = dice.split('d')
            cur['blows'].append({'method': method, 'effect': effect,
                                 'ddice': dd, 'dside': ds})
        elif line.startswith('F:'):
            v = line[2:]
            if v.startswith('R_CHAR_'):
                cur['req_chars'].append(v[7:])
            else:
                cur['req_flags'].append(v)
        elif line.startswith('H:'):
            v = line[2:]
            if v.startswith('R_CHAR_'):
                cur['forbid_chars'].append(v[7:])
            else:
                cur['forbid_flags'].append(v)
        elif line.startswith('M:'):
            cur['add_flags'].append(line[2:])
        elif line.startswith('O:'):
            cur['remove_flags'].append(line[2:])
        elif line.startswith('S:'):
            # S:1_IN_n sets the innate spell frequency; other S: lines
            # grant a spell (init1.cc init_re_info_txt).
            m = re.fullmatch(r'1_IN_(\d+)', line[2:].strip())
            if m:
                cur['spell_freq'] = 100 // int(m.group(1))
            else:
                cur['add_spells'].append(line[2:])
        elif line.startswith('T:'):
            # T:MF_ALL clears all inherited spells.
            cur['remove_spells'].append(line[2:])
    if cur:
        egos.append(cur)
    return egos


def parse_ow_info(path):
    """Store owners (ow_info.txt): cost percentages by race/class mood."""
    owners = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                owners.append(cur)
            _, oid, name = line.split(':', 2)
            cur = {'id': int(oid), 'name': name, 'max_cost': 10000,
                   'inflation': 100, 'costs': (100, 100, 100),
                   'liked': [], 'hated': []}
        elif cur is None:
            continue
        elif line.startswith('I:'):
            p = line.split(':')
            cur['max_cost'] = int(p[1])
            if len(p) > 2 and p[2].lstrip('-').isdigit():
                cur['inflation'] = int(p[2])
        elif line.startswith('C:'):
            p = line.split(':')
            cur['costs'] = (int(p[1]), int(p[2]), int(p[3]))
        elif line.startswith('L:'):
            cur['liked'].append(line[2:].strip())
        elif line.startswith('H:'):
            cur['hated'].append(line[2:].strip())
    if cur:
        owners.append(cur)
    return owners


def parse_v_info(path):
    """Vault templates (v_info.txt).  X:<typ>:<rat>:<hgt>:<wid>, D: map rows.

    Only typ 7 (lesser) and 8 (greater) are ever built (generate.cc
    build_type7/build_type8); the wilderness test record (typ 10) is dead
    data.  Holes in the N: numbering are ignored."""
    vaults = []
    cur = None

    def flush():
        # Every fully-specified record is ported (typ 7/8 are the ones
        # generate.cc builds; data.rs filters by typ at generation time).
        if cur and cur['typ'] and \
                len(cur['data']) == cur['hgt'] * cur['wid']:
            vaults.append(cur)

    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            flush()
            _, vid, name = line.split(':', 2)
            cur = {'id': int(vid), 'name': name, 'typ': 0, 'rat': 0,
                   'hgt': 0, 'wid': 0, 'data': ''}
        elif cur is None:
            continue
        elif line.startswith('X:'):
            typ, rat, hgt, wid = (int(x) for x in line[2:].split(':'))
            cur['typ'], cur['rat'], cur['hgt'], cur['wid'] = typ, rat, hgt, wid
        elif line.startswith('D:'):
            cur['data'] += line[2:]
    flush()
    return vaults


def parse_ra_info(path):
    """Randart parts (ra_info.txt) plus the power budget rows (G:).

    C: field order in the code is to-hit:to-dam:to-AC:pval (the file's
    comments are wrong; see src/randart.cc:300)."""
    parts, gen = [], []
    cur = None

    def flush():
        if cur and cur['tvals']:
            parts.append(cur)

    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            flush()
            cur = {'id': int(line[2:].split(':')[0]), 'tvals': [],
                   'level': 0, 'rarity': 0, 'mrarity': 0,
                   'to_h': 0, 'to_d': 0, 'to_a': 0, 'pval': 0,
                   'value': 0, 'max': 0, 'flags': [], 'aflags': []}
        elif cur is None:
            if line.startswith('G:'):
                m = re.fullmatch(r'G:(\d+):(\d+)d(\d+):(\d+)', line)
                if m:
                    gen.append(tuple(int(x) for x in m.groups()))
            continue
        elif line.startswith('T:'):
            tval, lo, hi = (int(x) for x in line[2:].split(':'))
            cur['tvals'].append((tval, lo, hi))
        elif line.startswith('X:'):
            value, mx = (int(x) for x in line[2:].split(':'))
            cur['value'], cur['max'] = value, mx
        elif line.startswith('W:'):
            lev, rar, mrar = (int(x) for x in line[2:].split(':'))
            cur['level'], cur['rarity'], cur['mrarity'] = lev, rar, max(1, mrar)
        elif line.startswith('C:'):
            th, td, ta, pv = (int(x) for x in line[2:].split(':'))
            cur['to_h'], cur['to_d'], cur['to_a'], cur['pval'] = th, td, ta, pv
        elif line.startswith('A:'):
            cur['aflags'].append(line[2:].strip())
        elif line.startswith('F:'):
            cur['flags'].append(line[2:].strip())
    flush()
    return parts, gen


def parse_set_info(path):
    """Artifact sets (set_info.txt): N: name; P: artifact:num:pval; the F:
    lines after a P: belong to that threshold (num is 1-based)."""
    sets = []
    cur = None
    member = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                sets.append(cur)
            _, sid, name = line.split(':', 2)
            cur = {'id': int(sid), 'name': name, 'desc': [],
                   'members': []}
            member = None
        elif cur is None:
            continue
        elif line.startswith('D:'):
            cur['desc'].append(line[2:])
        elif line.startswith('P:'):
            a_idx, num, pval = (int(x) for x in line[2:].split(':'))
            member = next((m for m in cur['members'] if m['artifact'] == a_idx),
                          None)
            if member is None:
                member = {'artifact': a_idx, 'tiers': []}
                cur['members'].append(member)
            while len(member['tiers']) < num:
                member['tiers'].append({'pval': 0, 'flags': []})
            member['tiers'][num - 1]['pval'] = pval
        elif line.startswith('F:') and member is not None:
            member['tiers'][-1]['flags'].append(line[2:].strip())
    if cur:
        sets.append(cur)
    return sets


def parse_artifact_names(path):
    """The randart name corpus (src/tables.cc artifact_names_list): one
    lowercase word per source string literal."""
    names = []
    inside = False
    for line in open(path, encoding='latin-1'):
        if not inside:
            if 'artifact_names_list' in line:
                inside = True
            continue
        for m in re.finditer(r'"([^"\\]*)\\n"', line):
            names.append(m.group(1))
        if ';' in line:
            break
    return names


def parse_junkarts(tome_lib, tome_src):
    """Legacy TV_RANDART random artifacts: the rart_s/rart_f name lists
    (init_randart, MAX_RANDARTS=84) and the activation table
    (src/tables.cc activation_info + ACT_* names from src/defines.hpp)."""
    names = lambda p: [l.rstrip('\n')
                       for l in open(p, encoding='latin-1').readlines()[1:85]]
    short = names(os.path.join(tome_lib, '..', 'file', 'rart_s.txt'))
    full = names(os.path.join(tome_lib, '..', 'file', 'rart_f.txt'))
    acts = []
    for line in open(os.path.join(tome_src, 'tables.cc'), encoding='latin-1'):
        m = re.search(r'\{\s*"([^"]+)",\s*(\d+),\s*ACT_([A-Z0-9_]+)\s*\}', line)
        if m and 'activation_info' not in line:
            acts.append((m.group(1), int(m.group(2)), m.group(3)))
    while len(acts) < 51:
        first = acts[0]
        acts.append(first)
    return short, full, acts[:51]


def parse_s_info(path):
    """Magic schools only (those granting the 'Cast a spell' action)."""
    schools = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur and cur['cast']:
                schools.append(cur)
            _, sid, name = line.split(':', 2)
            cur = {'id': int(sid), 'name': name, 'cast': False}
        elif cur is None:
            continue
        elif line.startswith('A:17:'):
            cur['cast'] = True
    if cur and cur['cast']:
        schools.append(cur)
    return schools


def parse_skills(path):
    """Full skill tree (s_info.txt): descriptors + f:/E:/T: relations."""
    by_name = {}
    order = []
    increases, excludes, fathers = [], [], {}
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if not line or line.startswith('#'):
            continue
        if line.startswith('N:'):
            _, sid, name = line.split(':', 2)
            cur = {'id': int(sid), 'name': name, 'desc': [], 'flags': [],
                   'action_mkey': 0, 'action_desc': '', 'chance': 100,
                   'father': -1, 'order': 0, 'increases': [], 'excludes': []}
            by_name[name] = cur
            order.append(name)
        elif cur is None:
            continue
        elif line.startswith('D:'):
            cur['desc'].append(line[2:])
        elif line.startswith('F:'):
            cur['flags'].append(line[2:].strip())
        elif line.startswith('G:'):
            cur['chance'] = int(line[2:])
        elif line.startswith('A:'):
            mkey, adesc = line[2:].split(':', 1)
            cur['action_mkey'] = int(mkey)
            cur['action_desc'] = adesc
        elif line.startswith('f:'):
            src, rest = line[2:].split(':', 1)
            dst, pct = rest.rsplit('%', 1)
            increases.append((src, dst, int(pct)))
        elif line.startswith('E:'):
            a, b = line[2:].split(':', 1)
            excludes.append((a, b))
        elif line.startswith('T:'):
            a, b = line[2:].split(':', 1)
            fathers[b] = a
    ids = {name: s['id'] for name, s in by_name.items()}
    for i, name in enumerate(order, start=1):
        by_name[name]['order'] = i
    for name, father in fathers.items():
        by_name[name]['father'] = ids.get(father, -1)
    for src, dst, pct in increases:
        if src in by_name and dst in by_name:
            by_name[src]['increases'].append((ids[dst], pct))
    for a, b in excludes:
        if a in by_name and b in by_name:
            by_name[a]['excludes'].append(ids[b])
            by_name[b]['excludes'].append(ids[a])
    return [by_name[n] for n in order]


def parse_abilities(path):
    """ab_info.txt: 9 abilities with cost/prerequisites/action."""
    abilities = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                abilities.append(cur)
            _, aid, name = line.split(':', 2)
            cur = {'id': int(aid), 'name': name, 'desc': [], 'cost': 0,
                   'action_mkey': 0, 'action_desc': '', 'need_skills': [],
                   'stats': [-1] * 6, 'need_abilities': []}
        elif cur is None:
            continue
        elif line.startswith('D:'):
            cur['desc'].append(line[2:])
        elif line.startswith('I:'):
            cur['cost'] = int(line[2:])
        elif line.startswith('A:'):
            mkey, adesc = line[2:].split(':', 1)
            cur['action_mkey'] = int(mkey)
            cur['action_desc'] = adesc
        elif line.startswith('k:'):
            lvl, skill = line[2:].split(':', 1)
            cur['need_skills'].append((skill, int(lvl)))
        elif line.startswith('S:'):
            lvl, stat = line[2:].split(':', 1)
            cur['stats']['STR INT WIS DEX CON CHR'.split().index(stat)] = int(lvl)
        elif line.startswith('a:'):
            cur['need_abilities'].append(line[2:])
    if cur:
        abilities.append(cur)
    return abilities


def parse_ba_info(path):
    """ba_info.txt: building actions (cost per owner mood, action code)."""
    actions = []
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            if cur:
                actions.append(cur)
            _, aid, name = line.split(':', 2)
            cur = {'id': int(aid), 'name': name, 'costs': [0, 0, 0],
                   'action': -1, 'restr': 0, 'letter': '.', 'letter_aux': ''}
        elif cur is None:
            continue
        elif line.startswith('C:'):
            p = line.split(':')
            cur['costs'] = [int(x) for x in p[1:4]]
        elif line.startswith('I:'):
            p = line[2:].split(':')
            cur['action'] = int(p[0])
            cur['restr'] = int(p[1])
            cur['letter'] = p[2] if len(p) > 2 and p[2] else '.'
            cur['letter_aux'] = p[3] if len(p) > 3 else ''
    if cur:
        actions.append(cur)
    return actions


# --- Wilderness (wf_info.txt / w_info.txt) -------------------------------
# wf_info records drive the per-cell wilderness terrain (src/init1.cc
# init_wf_info_txt): W:<level>:<entrance>:<road>:<feat>:<terrain_idx>:<char>
# and X: 18 height->f_info terrain mappings.  w_info.txt is the world map
# (101x66): 'W:D' rows of wf chars, 'W:M' line moves (used by the
# destroyed-Gondolin branch), 'W:E' dungeon entrances (W:E:<d>:<y>:<x>)
# and 'W:P:<x>:<y>' for the player's starting world cell.

MAX_WILD_TERRAIN = 18


def parse_wf_info(path):
    recs = {}
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if not line or line.startswith('#'):
            continue
        if line.startswith('N:'):
            _, idx, name = line.split(':', 2)
            cur = {'id': int(idx), 'name': name, 'text': '', 'level': 1,
                   'entrance': 0, 'road': 0, 'feat': 0, 'terrain_idx': 0,
                   'ch': '?', 'terrain': [0] * MAX_WILD_TERRAIN}
            recs[int(idx)] = cur
        elif cur is None:
            continue
        elif line.startswith('D:'):
            cur['text'] = line[2:]
        elif line.startswith('W:'):
            p = line[2:].split(':')
            cur['level'] = int(p[0])
            cur['entrance'] = int(p[1])
            cur['road'] = int(p[2])
            cur['feat'] = int(p[3])
            cur['terrain_idx'] = int(p[4])
            cur['ch'] = p[5][0] if len(p) > 5 and p[5] else '?'
        elif line.startswith('X:'):
            cur['terrain'] = [int(x) for x in line[2:].split(':')]
    return recs


def parse_w_info(path, wf_chars):
    """Parse the world map.  Returns base rows plus the conditional row
    replacements (the destroyed-Gondolin `$TOWN_DESTROY2` branch and its
    `W:M:0:1` line move)."""
    lines = [l.rstrip('\n') for l in open(path, encoding='latin-1')]

    def run(variables):
        y = 0
        active = True
        rows = {}
        entrances = []
        start = [None]
        for line in lines:
            if not line or line.startswith('#'):
                continue
            if line.startswith('?:'):
                e = line[2:].strip()
                active = True if e == '1' else eval_cond(e, variables)
                continue
            if not active:
                continue
            if line.startswith('W:D:'):
                rows[y] = line[4:]
                y += 1
            elif line.startswith('W:M:'):
                plus, n = line[4:].split(':')[:2]
                y += int(n) if int(plus) else -int(n)
            elif line.startswith('W:E:'):
                d, yy, xx = line[4:].split(':')[:3]
                entrances.append((int(d), int(xx), int(yy)))
            elif line.startswith('W:P:'):
                x, yy = line[4:].split(':')[:2]
                if start[0] is None:
                    start[0] = (int(x), int(yy))
        return rows, entrances, start[0]

    base_vars = {'TOWN_DESTROY1': 0, 'TOWN_DESTROY2': 0, 'TOWN_DESTROY3': 0,
                 'TOWN_DESTROY4': 0, 'TOWN_DESTROY5': 0}
    base_rows, entrances, start = run(base_vars)
    patches = []
    for var in ('TOWN_DESTROY1', 'TOWN_DESTROY2', 'TOWN_DESTROY3',
                'TOWN_DESTROY4', 'TOWN_DESTROY5'):
        v = dict(base_vars)
        v[var] = 1
        rows, _, _ = run(v)
        cond = 'town_destroy' + var[-1]
        for y, row in sorted(rows.items()):
            if base_rows.get(y) != row:
                patches.append((cond, y, row))
    w = max(len(r) for r in base_rows.values())
    h = max(base_rows) + 1
    grid = []
    for y in range(h):
        row = base_rows.get(y, ' ' * w)
        grid.append([wf_chars.get(c, 0) for c in row.ljust(w)])
    patch_out = []
    for cond, y, row in patches:
        patch_out.append((cond, y, [wf_chars.get(c, 0) for c in row.ljust(w)]))
    return {'w': w, 'h': h, 'rows': grid, 'patches': patch_out,
            'entrances': entrances, 'start': start}


# --- Dungeons (d_info.txt) -----------------------------------------------

def parse_d_info(path):
    recs = {}
    order = []
    cur = None
    rule = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if not line or line.startswith('#'):
            continue
        if line.startswith('N:'):
            _, idx, name = line.split(':', 2)
            cur = {'id': int(idx), 'name': name, 'short': '', 'text': '',
                   'mindepth': 0, 'maxdepth': 0, 'min_plev': 0,
                   'min_alloc': 0, 'max_chance': 0,
                   'floor_types': [1, 1, 1], 'floor_pcts': [[100, 100]] * 3,
                   'fill_types': [56, 56, 56], 'fill_pcts': [[100, 100]] * 3,
                   'outer_wall': 57, 'inner_wall': 58,
                   'theme': [20, 20, 20, 20], 'flags': [],
                   'effects': [], 'rules': [], 'rule_percents': [0] * 100,
                   'guardian': None, 'final_artifact': None,
                   'final_object': None, 'specials': [], 'generator': '',
                   'ix': -1, 'iy': -1, 'ox': -1, 'oy': -1, 'branch_parent': None}
            recs[int(idx)] = cur
            order.append(int(idx))
        elif cur is None:
            continue
        elif line.startswith('D:'):
            p = line[2:].split(':', 1)
            cur['short'] = p[0]
            cur['text'] = p[1] if len(p) > 1 else ''
        elif line.startswith('W:'):
            p = [int(x) for x in line[2:].split(':')[:5]]
            cur['mindepth'], cur['maxdepth'], cur['min_plev'], \
                cur['min_alloc'], cur['max_chance'] = p
        elif line.startswith('L:'):
            p = [int(x) for x in line[2:].split(':')]
            if len(p) >= 6:
                for i in range(3):
                    cur['floor_types'][i] = p[i * 2]
                    cur['floor_pcts'][i] = [p[i * 2 + 1], p[i * 2 + 1]]
            else:
                for i in range(3):
                    cur['floor_pcts'][i][1] = p[i]
        elif line.startswith('A:'):
            p = [int(x) for x in line[2:].split(':')]
            if len(p) >= 8:
                for i in range(3):
                    cur['fill_types'][i] = p[i * 2]
                    cur['fill_pcts'][i] = [p[i * 2 + 1], p[i * 2 + 1]]
                cur['outer_wall'] = p[6]
                cur['inner_wall'] = p[7]
            else:
                for i in range(3):
                    cur['fill_pcts'][i][1] = p[i]
        elif line.startswith('O:'):
            cur['theme'] = [int(x) for x in line[2:].split(':')[:4]]
        elif line.startswith('E:'):
            p = line[2:].split(':')[:3]
            dice = p[0].split('d')
            cur['effects'].append((int(dice[0]), int(dice[1]), int(p[1]),
                                   p[2] if len(p) > 2 else ''))
        elif line.startswith('F:'):
            s = line[2:]
            if s.startswith('WILD_'):
                a, b, c, d = [int(x) for x in
                              s[5:].split('__')[0].split('_') +
                              s[5:].split('__')[1].split('_')]
                cur['ix'], cur['iy'], cur['ox'], cur['oy'] = a, b, c, d
            elif s.startswith('FINAL_GUARDIAN_'):
                cur['guardian'] = int(s[len('FINAL_GUARDIAN_'):])
            elif s.startswith('FINAL_ARTIFACT_'):
                cur['final_artifact'] = int(s[len('FINAL_ARTIFACT_'):])
            elif s.startswith('FINAL_OBJECT_'):
                cur['final_object'] = int(s[len('FINAL_OBJECT_'):])
            elif s.startswith('SIZE_'):
                cur['flags'].append(s)
            elif s.startswith('FILL_METHOD_'):
                cur['flags'].append(s)
            elif s:
                cur['flags'].append(s)
        elif line.startswith('G:'):
            cur['generator'] = line[2:]
        elif line.startswith('R:'):
            pct, mode = [int(x) for x in line[2:].split(':')[:2]]
            rule = {'pct': pct, 'mode': mode, 'chars': [], 'mflags': [],
                    'mspells': []}
            cur['rules'].append(rule)
            lims = [cur['rules'][0]['pct']]
            for r in cur['rules'][1:]:
                lims.append(lims[-1] + r['pct'])
            for z in range(100):
                for y in range(len(cur['rules']) - 1, -1, -1):
                    if z < lims[y]:
                        cur['rule_percents'][z] = y
        elif line.startswith('M:') and rule is not None:
            s = line[2:]
            if s.startswith('R_CHAR_'):
                rule['chars'].append(s[len('R_CHAR_'):])
            elif s:
                rule['mflags'].append(s)
        elif line.startswith('S:') and rule is not None:
            rule['mspells'].append(line[2:])
        elif line.startswith('@:'):
            p = line[2:].split(':')
            depth = int(p[0]) + cur['mindepth']
            key, val = p[1], p[2] if len(p) > 2 else ''
            sp = next((s for s in cur['specials'] if s['depth'] == depth), None)
            if sp is None:
                sp = {'depth': depth, 'name': '', 'desc': '', 'map': '',
                      'flags': [], 'branch': None, 'save_extension': None}
                cur['specials'].append(sp)
            if key == 'N':
                sp['name'] = val
            elif key == 'D':
                sp['desc'] = val
            elif key == 'U':
                sp['map'] = val
            elif key == 'F':
                sp['flags'].append(val)
            elif key == 'B':
                sp['branch'] = int(val)
            elif key == 'S':
                # @:<depth>:S:<ext>: the level persists under this save-file
                # extension (init1.cc init_d_info_txt; loadsave.cc
                # save_dungeon/get_dungeon_save_extension).
                sp['save_extension'] = val
    # Branch links: a sub-dungeon remembers its parent + depth for the
    # upstairs return trip (post_d_info in src/init1.cc).
    for d in recs.values():
        for sp in d['specials']:
            if sp['branch'] is not None and sp['branch'] in recs:
                sub = recs[sp['branch']]
                if sub['branch_parent'] is None:
                    sub['branch_parent'] = (d['id'], sp['depth'])
    return [recs[i] for i in sorted(recs)]


# --- Towns (t_info.txt selection, t_*.txt pref maps) ---------------------

def parse_town_pref(path, base_chars, variables):
    """Parse a town pref file: unconditional F: defs land in the base char
    map, conditional F: defs become ordered overrides, D: rows form the
    layout and P: entries are keyed by their condition."""
    chars = dict(base_chars)
    overrides = []
    rows = []
    starts = []
    default_start = [None]
    mflag = ['']

    def rec(p):
        active = True
        cond = None
        for raw in open(p, encoding='latin-1'):
            line = raw.rstrip('\n')
            if not line or line.startswith('#'):
                continue
            if line.startswith('?:'):
                e = line[2:].strip()
                if e == '1':
                    active, cond = True, None
                else:
                    active = eval_cond(e, variables)
                    cond = None if active else e
                continue
            if not active and cond is None:
                continue
            if line.startswith('%:'):
                rec(os.path.join(os.path.dirname(p), line[2:].strip()))
                continue
            if line.startswith('f:'):
                mflag[0] = line[2:].strip()
                continue
            if line.startswith('F:'):
                f = line[2:].split(':')

                def num(i):
                    if i >= len(f):
                        return 0
                    v = f[i].lstrip('*')
                    return int(v) if v.lstrip('-').isdigit() else 0

                raise_ = 0
                if len(f) > 7 and not f[7].lstrip('*').lstrip('-').isdigit():
                    named = f[7].strip().strip('"')
                    raise_ = {'Library quest': 28,
                              'Old Mages quest': 27}.get(named, 0)
                    d = {'terrain': num(1), 'cave': num(2), 'monster': num(3),
                         'object': num(4), 'special': raise_, 'mflag': num(9)}
                else:
                    d = {'terrain': num(1), 'cave': num(2), 'monster': num(3),
                         'object': num(4), 'special': num(7), 'mflag': num(9)}
                if cond is None:
                    chars[f[0]] = d
                else:
                    overrides.append((cond, f[0], d))
            elif line.startswith('D:'):
                rows.append(line[2:])
            elif line.startswith('P:'):
                _, y, x = line.split(':')
                if cond is None:
                    default_start[0] = (int(x), int(y))
                else:
                    starts.append((cond, int(x), int(y)))

    rec(path)
    return {'rows': rows, 'chars': chars, 'overrides': overrides,
            'starts': starts, 'default_start': default_start[0],
            'mflag': mflag[0]}


def build_town(town_id, name, path, destroyed, base_chars, terrains):
    """One town variant (normal or destroyed map) from a pref file."""
    parsed = parse_town_pref(path, base_chars, PREF_VARS)
    rows = parsed['rows']
    assert len(rows) == MAP_H, f'{path}: {len(rows)} rows'
    chars = []
    for ch, d in sorted(parsed['chars'].items()):
        chars.append((ch, d))
    overrides = [(cond, ch, d) for cond, ch, d in parsed['overrides']]
    starts = []
    for cond, x, y in parsed['starts']:
        toks = cond.replace('[', ' ').replace(']', ' ').split()
        q = 0
        if 'LEAVING_QUEST' in toks:
            q = int(toks[toks.index('$LEAVING_QUEST') + 1])
        starts.append((q, x, y))
    return {'id': town_id, 'name': name, 'destroyed': destroyed,
            'rows': rows, 'chars': chars, 'overrides': overrides,
            'starts': starts, 'default_start': parsed['default_start'],
            'mflag': parsed['mflag']}


# --- Curated spell table -------------------------------------------------
# ToME 2.x spells are hardcoded in src/spells*.cc (they are not data), so
# this table is a faithful simplification organised by s_info school ids:
# 1 Conveyance, 2 Mana, 3 Fire, 4 Air, 5 Water, 6 Nature, 7 Earth,
# 10 Divination, 11 Temporal, 53 Prayer.
# School 0 = device-only rows: wands/staffs reference spells by name
# (k_info SPELL=...), rods are resolved by their object name.
# kind: bolt(dice) heal(dice) teleport(range) light detect map identify
#       remove_curse recall dig door teleport_away slow summon mana cure
#       fizzle nothing   (targeted = needs a direction)
SPELLS = [
    # name, school, level, mana, fail, kind, arg, targeted
    # --- class-castable ---
    ("Manathrust", 2, 1, 1, 10, 25, "bolt",          "2d6",    True),
    ("Remove Curses", 2, 10, 20, 30, 40, "remove_curse",  "",       False),
    ("Globe of Light", 3, 1, 2, 10, 15, "globe_of_light", "",       False),
    ("Fireflash", 3, 10, 5, 35, 70, "fireflash",     "6d8",    True),
    ("Noxious Cloud", 4, 3, 3, 20, 30, "noxious_cloud", "4d6",    True),
    ("Thunderstorm", 4, 25, 40, 60, 60, "thunderstorm",  "",       False),
    ("Tidal Wave", 5, 16, 16, 65, 40, "tidal_wave",    "",       False),
    ("Strike", 7, 30, 30, 60, 50, "strike_bolt",   "",       True),
    ("Dig", 7, 12, 14, 20, 14, "dig",           "",       True),
    ("Phase Door", 1, 1, 1, 10, 3, "teleport",      "10",     False),
    ("Teleport", 1, 10, 8, 30, 14, "teleport",      "100",    False),
    ("Teleport Away", 1, 23, 15, 60, 40, "teleport_away", "",       True),
    ("Word of Recall", 11, 30, 25, 60, 25, "convey_recall", "",       True),
    ("Detect Monsters", 10,  2,  1, 10, "detect",        "",       False),
    ("Identify",        10,  6,  5, 20, "identify",      "",       False),
    ("Cure Light Wounds", 6, 1,  2, 10, "heal",          "2d8",    False),
    ("Heal",             6,  9, 10, 30, "heal",          "4d8+20", False),
    ("Cure Light Wounds",53, 1,  2, 10, "heal",          "2d8",    False),
    ("Heal",            53,  9, 10, 30, "heal",          "4d8+20", False),
    ("Remove Curses",   53, 10, 20, 30, 40, "remove_curse",  "",       False),
    # --- missing base school spells (src/spells5.cc registrations).
    #     level/fail are the original difficulty (skill level, failure
    #     rate); mana is the original minimum (device casts scale with
    #     the device level instead). ---
    ("Firewall", 3, 15, 25, 40, 100, "firewall",         "",   True),
    ("Fiery Shield", 3, 20, 20, 50, 60, "fiery_shield",     "",   False),
    ("Wings of Winds", 4, 22, 30, 60, 40, "wings_of_winds",   "",   False),
    ("Probability Travel", 1, 35, 30, 90, 50, "probability_travel","",  False),
    ("Poison Blood", 4, 12, 10, 30, 20, "poison_blood",     "",   False),
    ("Sterilize", 4, 20, 10, 50, 100, "sterilize",        "",   False),
    ("Stone Prison", 7, 25, 30, 65, 50, "stone_prison",     "",   True),
    ("Shake", 7, 27, 25, 60, 30, "shake",            "",   True),
    ("Ice Storm", 5, 22, 30, 80, 60, "ice_storm",        "",   True),
    ("Healing", 6, 10, 15, 45, 50, "healing",          "",   False),
    ("Recovery", 6, 15, 10, 60, 25, "recovery",         "",   False),
    ("Genocide", 55, 25, 50, 90, 50, "genocide",         "",   True),
    ("Vision", 10, 15, 7, 45, 55, "vision",           "",   False),
    ("Sense Hidden", 10, 5, 2, 25, 10, "sense_hidden",     "",   False),
    ("Reveal Ways", 10, 9, 3, 20, 15, "reveal_ways",      "",   False),
    ("Sense Monsters", 10, 1, 1, 10, 20, "sense_monsters",   "",   False),
    ("Magelock", 11, 1, 1, 10, 35, "magelock",         "",   True),
    ("Slow Monster", 11, 10, 10, 35, 15, "slow_monster",     "",   True),
    ("Essence of Speed", 11, 15, 20, 50, 40, "essence_of_speed", "",   False),
    ("Banishment", 11, 30, 30, 95, 40, "banishment",       "",   True),
    ("Disperse Magic", 14, 15, 30, 40, 60, "disperse_magic",   "",   False),
    ("Charm", 51, 1, 1, 10, 20, "charm_monster",    "",   True),
    ("Confuse", 51, 5, 5, 20, 30, "confuse_monster",  "",   True),
    # --- device-only (wands, k_info SPELL=).  level/mana/fail are the
    #     original spell_type_set_difficulty + set_mana values
    #     (src/spells5.cc): the first entry of the mana range. ---
    ("Artifact Thunderlords", 0, 1, 1, 20, 1, "thunderlords", "",       False),
    ("Banishment", 0, 30, 30, 95, 40, "banishment",    "",          True),
    ("Charm", 0, 1, 1, 10, 20, "charm_monster", "",          True),
    ("Confuse", 0, 5, 5, 20, 30, "confuse_monster", "",        True),
    ("Demon Blade", 0, 1, 4, 10, 44, "demon_blade",  "",           False),
    ("Dig", 0, 12, 14, 20, 14, "dig",          "",           True),
    ("Disperse Magic", 0, 15, 30, 40, 60, "disperse_magic", "",         False),
    ("Essence of Speed", 0, 15, 20, 50, 40, "essence_of_speed", "",       False),
    ("Fiery Shield", 0, 20, 20, 50, 60, "fiery_shield",  "",           False),
    ("Fireflash", 0, 10, 5, 35, 70, "bolt",         "6d8",        True),
    ("Firewall", 0, 15, 25, 40, 100, "firewall",      "",           True),
    ("Genocide", 0, 25, 50, 90, 50, "genocide",     "",           True),
    ("Globe of Light", 0, 1, 2, 10, 15, "light",        "",           False),
    ("Haste Monster", 0, 10, 10, 30, 10, "haste_monster", "",           True),
    ("Healing", 0, 10, 15, 45, 50, "heal",         "20d10",      False),
    ("Heal Monster", 0, 3, 5, 15, 20, "heal_monster",  "",           True),
    ("Holy Fire of Mithrandir", 0, 30, 50, 75, 150, "holy_fire", "", False),
    ("Ice Storm", 0, 22, 30, 80, 60, "ice_storm",     "",           True),
    ("Magelock", 0, 1, 1, 10, 35, "magelock",      "",           True),
    ("Mana", 0, 30, 1, 80, 1, "mana",         "",           False),
    ("Manathrust", 0, 1, 1, 10, 25, "bolt",         "2d6",        True),
    ("Nothing", 0, 1, 0, 0, 0, "nothing",      "",           False),
    ("Noxious Cloud", 0, 3, 3, 20, 30, "bolt",         "4d6",        True),
    ("Poison Blood", 0, 12, 10, 30, 20, "poison_blood",  "",           False),
    ("Probability Travel", 0, 35, 30, 90, 50, "probability_travel", "",   False),
    ("Recovery", 0, 15, 10, 60, 25, "cure",         "",           False),
    ("Remove Curses", 0, 10, 20, 30, 40, "remove_curse", "",           False),
    ("Reveal Ways", 0, 9, 3, 20, 15, "reveal_ways",   "",           False),
    ("Sense Hidden", 0, 5, 2, 25, 10, "sense_hidden",  "",           False),
    ("Sense Monsters", 0, 1, 1, 10, 20, "sense_monsters","",           False),
    ("Shake", 0, 27, 25, 60, 30, "shake",        "",           True),
    ("Slow Monster", 0, 10, 10, 35, 15, "slow_monster",  "",           True),
    ("Sterilize", 0, 20, 10, 50, 100, "sterilize",     "",           False),
    ("Stone Prison", 0, 25, 30, 65, 50, "stone_prison", "",           True),
    ("Strike", 0, 30, 30, 60, 50, "bolt",         "4d8",        True),
    ("Summon", 0, 5, 5, 20, 25, "summon",       "",           False),
    ("Summon Animal", 0, 25, 25, 90, 50, "summon",       "",           False),
    ("Teleportation", 0, 10, 8, 30, 14, "teleport",     "100",        False),
    ("Teleport Away", 0, 23, 15, 60, 40, "teleport_away", "",          True),
    ("Thunderstorm", 0, 25, 40, 60, 60, "bolt",         "8d8",        True),
    ("Tidal Wave", 0, 16, 16, 65, 40, "bolt",         "6d6",        True),
    ("Vision", 0, 15, 7, 45, 55, "vision",        "",           False),
    ("Wings of Winds", 0, 22, 30, 60, 40, "wings_of_winds", "",          False),
    ("Wish", 0, 50, 400, 99, 400, "acquirement",  "",           False),
    # --- device-only (rods, resolved by object name) ---
    ("Acid Balls",       0, 1, 0, 0, "bolt",         "8d8",        True),
    ("Acid Bolts",       0, 1, 0, 0, "bolt",         "6d8",        True),
    ("Cold Balls",       0, 1, 0, 0, "bolt",         "8d8",        True),
    ("Curing",           0, 1, 0, 0, "cure",         "",           False),
    ("Detection",        0, 1, 0, 0, "detect",       "",           False),
    ("Door/Stair Location", 0, 1, 0, 0, "detect",    "",           False),
    ("Drain Life",       0, 1, 0, 0, "drain_life",    "",           True),
    ("Enlightenment",    0, 1, 0, 0, "map",          "",           False),
    ("Fire Balls",       0, 1, 0, 0, "bolt",         "8d8",        True),
    ("Fire Bolts",       0, 1, 0, 0, "bolt",         "6d8",        True),
    ("Frost Bolts",      0, 1, 0, 0, "bolt",         "5d8",        True),
    ("Havoc",            0, 1, 0, 0, "havoc",         "",           True),
    ("Home Summoning",   0, 1, 0, 0, "home",         "",           False),
    ("Illumination",     0, 1, 0, 0, "light",        "",           False),
    ("Light",            0, 1, 0, 0, "light",        "",           False),
    ("Lightning Balls",  0, 1, 0, 0, "bolt",         "8d8",        True),
    ("Lightning Bolts",  0, 1, 0, 0, "bolt",         "4d8",        True),
    ("Polymorph",        0, 1, 0, 0, "polymorph",     "",           True),
    ("Recall",           0, 1, 0, 0, "recall",       "",           False),
    ("Restoration",      0, 1, 0, 0, "restoration",   "",           False),
    ("Sleep Monster",    0, 1, 0, 0, "sleep_monster", "",           True),
    ("Speed",            0, 1, 0, 0, "haste",        "",           False),
    ("Teleport Other",   0, 1, 0, 0, "teleport_away", "",          True),
    # --- extended class-castable rows (buffs/summons; effect kinds live
    #     in modal::apply_effect) ---
    ("Haste",           11, 14, 12, 30, "haste",        "",        False),
    ("Heroism",         53,  3,  4, 15, "hero",         "",        False),
    ("Protection from Evil", 53, 5, 6, 20, "protevil",  "",        False),
    ("Resist Elements", 53,  7,  8, 20, "resist",       "",        False),
    ("Summon Animal", 6, 25, 25, 90, 50, "summon_animal","",        False),
    # --- remaining basic school spells (src/spells5.cc / spells3.cc;
    #     mana is the flat mid of the original min-max range) ---
    ("Geyser", 5, 1, 1, 5, 35, "geyser",          "",    True),
    ("Vapor", 5, 2, 2, 20, 12, "vapor",           "",    False),
    ("Ent's Potion", 5, 6, 7, 35, 15, "ents_potion",     "",    False),
    ("Invisibility", 4, 16, 10, 50, 20, "invisibility",    "",    False),
    ("Stone Skin", 7, 1, 1, 10, 50, "stone_skin",      "",    False),
    ("Grow Trees", 6, 6, 6, 35, 30, "grow_trees",      "",    False),
    ("Regeneration", 6, 20, 30, 70, 55, "regeneration",    "",    False),
    ("Recharge", 14, 5, 10, 20, 100, "recharge",        "",    False),
    ("Spellbinder", 14, 20, 100, 85, 300, "spellbinder",     "",    False),
    ("Tracker", 14, 30, 50, 95, 50, "tracker",         "",    False),
    ("Inertia Control", 14, 37, 300, 95, 700, "inertia_control", "",    False),
    ("Elemental Shield", 2, 20, 17, 40, 20, "elemental_shield","",    False),
    ("Disruption Shield", 2, 45, 50, 90, 50, "disruption_shield","",   False),
    ("Drain", 55, 1, 0, 20, 0, "drain_device",    "",    False),
    ("Stun", 51, 15, 10, 45, 90, "stun_bolt",       "",    True),
    ("Armor of Fear", 51, 10, 10, 35, 50, "armor_of_fear",   "",    False),
    ("Wraithform", 55, 30, 20, 95, 40, "wraithform",      "",    False),
    ("Flame of Udun", 55, 35, 70, 95, 100, "flame_of_udun",   "",    False),
    ("Fire Golem", 3, 7, 16, 40, 70, "fire_golem",      "",    False),
    # --- Demonologist demonology spells (school 13 Demon; only castable
    #     from a wielded TV_DAEMON_BOOK, src/spells3.cc demonology_*) ---
    ("Demon Blade", 13, 1, 4, 10, 44, "demon_blade",   "",        False),
    ("Demon Madness", 13, 10, 5, 25, 20, "demon_madness", "",        True),
    ("Demon Field", 13, 20, 20, 60, 60, "demon_field",   "",        True),
    ("Doom Shield", 13, 1, 2, 10, 30, "doom_shield",   "",        False),
    ("Unholy Word", 13, 25, 15, 55, 45, "unholy_word",   "",        True),
    ("Demon Cloak", 13, 20, 10, 70, 40, "demon_cloak",   "",        False),
    ("Summon Demon", 13, 5, 10, 30, 50, "summon_demon",  "",        False),
    ("Discharge Minion", 13, 10, 20, 30, 50, "discharge_minion","",      True),
    ("Control Demon", 13, 25, 30, 55, 70, "control_demon", "",        True),
    # --- Bard songs (school 100 Music; spells3.cc music_*, spells5.cc).
    #     Instruments are the school books: sval 58 Drums, 59 Harps,
    #     60 Horns; the roman numeral is the minimum instrument pval.
    #     mana/mana_max/fail are the original spell_type_set_mana /
    #     set_difficulty values. ---
    ("Stop singing(I)",      100,  1,  0, -400, "music_stop",      "",  False),
    ("Holding Pattern(I)", 100, 1, 1, 20, 10, "music_hold",      "",  False),
    ("Illusion Pattern(II)", 100, 5, 2, 30, 15, "music_conf",      "",  False),
    ("Stun Pattern(IV)", 100, 10, 3, 45, 25, "music_stun",      "",  False),
    ("Song of the Sun(I)", 100, 1, 1, 20, 1, "music_lite",      "",  False),
    ("Flow of Life(II)", 100, 7, 5, 35, 30, "music_heal",      "",  False),
    ("Heroic Ballad(II)", 100, 10, 4, 45, 14, "music_hero",      "",  False),
    ("Hobbit Melodies(III)", 100, 20, 10, 70, 30, "music_time",      "",  False),
    ("Clairaudience(IV)", 100, 25, 15, 75, 30, "music_mind",      "",  False),
    ("Blow(I)", 100, 4, 3, 20, 30, "music_blow",      "",  False),
    ("Gush of Wind(II)", 100, 14, 15, 30, 45, "music_wind",      "",  False),
    ("Horns of Ylmir(III)", 100, 20, 25, 20, 30, "music_ylmir",     "",  False),
    ("Ambarkanta(IV)", 100, 25, 70, 60, 70, "music_ambarkanta", "",  False),
    # --- god-specific spells (school 53 Prayer; god field gates them and
    #     they cost grace instead of mana) ---
    ("See the Music", 53, 1, 1, 20, 50, "eru_see_music","",       False),
    ("Listen to the Music", 53, 7, 15, 25, 200, "eru_listen",  "",        False),
    ("Lay of Protection", 53, 35, 400, 80, 400, "eru_lay",     "",        False),
    ("Manwe's Blessing", 53, 1, 10, 20, 100, "manwe_blessing","",      False),
    ("Wind Shield", 53, 10, 100, 30, 500, "manwe_wind_shield","",   False),
    ("Manwe's Call", 53, 20, 200, 40, 500, "manwe_call",  "",        False),
    ("Divine Aim", 53, 1, 30, 20, 500, "tulkas_divine_aim","",   False),
    ("Whirlwind", 53, 10, 100, 45, 100, "tulkas_whirlwind","",    False),
    ("Wave of Power", 53, 20, 200, 75, 200, "wave_of_power","",        True),
    ("Curse", 53, 1, 50, 20, 300, "melkor_curse", "",        True),
    ("Corpse Explosion", 53, 10, 100, 45, 500, "corpse_explosion","",     False),
    ("Mind Steal", 53, 20, 1000, 90, 3000, "mind_steal",   "",        True),
    ("Charm Animal", 53, 1, 10, 30, 100, "yavanna_charm_animal","", True),
    ("Grow Grass", 53, 10, 70, 65, 150, "yavanna_grow_grass","",  False),
    ("Water Bite", 53, 20, 150, 90, 300, "water_bite",  "",         False),
    # --- missing base god spells (spells5.cc registrations) ---
    ("Avatar", 53, 35, 1000, 80, 1000, "manwe_avatar",   "",     False),
    ("Tree Roots", 53, 15, 50, 70, 1000, "yavanna_roots",  "",     False),
    ("Uproot", 53, 35, 250, 95, 350, "yavanna_uproot", "",      True),
    # --- Aule/Varda/Ulmo/Mandos god spells (spells5.cc registrations) ---
    ("Firebrand", 53, 1, 10, 20, 100, "aule_firebrand", "", False),
    ("Enchant Weapon", 53, 10, 100, 20, 200, "aule_enchant_weapon", "", False),
    ("Enchant Armour", 53, 15, 100, 20, 200, "aule_enchant_armour", "", False),
    ("Child of Aule", 53, 20, 200, 40, 500, "aule_child", "", False),
    ("Light of Valinor", 53, 1, 1, 20, 100, "varda_light_of_valinor", "", False),
    ("Call of Almaren", 53, 10, 5, 20, 150, "varda_call_of_almaren", "", False),
    ("Evenstar", 53, 20, 20, 20, 200, "varda_evenstar", "", False),
    ("Star Kindler", 53, 30, 50, 20, 250, "varda_star_kindler", "", True),
    ("Song of Belegaer", 53, 1, 1, 25, 100, "ulmo_song_of_belegaer", "", True),
    ("Draught of Ulmonan", 53, 15, 25, 50, 200, "ulmo_draught_of_ulmonan", "", False),
    ("Call of the Ulumuri", 53, 20, 50, 75, 300, "ulmo_call_of_the_ulumuri", "", False),
    ("Wrath of Ulmo", 53, 30, 100, 95, 400, "ulmo_wrath_of_ulmo", "", True),
    ("Tears of Luthien", 53, 5, 10, 25, 100, "mandos_tears_of_luthien", "", False),
    ("Feanturi", 53, 10, 40, 50, 200, "mandos_feanturi", "", False),
    ("Tale of Doom", 53, 25, 60, 75, 300, "mandos_tale_of_doom", "", False),
    ("Call to the Halls", 53, 30, 80, 95, 400, "mandos_call_to_the_halls", "", False),
    ("Grow Athelas", 6, 30, 60, 95, 100, "grow_athelas", "", False),
]

# Element carried by elemental spell rows (empty = untyped); consulted
# against monster IM_*/SUSCEP_* flags and player resistances.
SPELL_ELEM = {
    "Fireflash": "FIRE", "Firewall": "FIRE", "Fire Balls": "FIRE",
    "Fire Bolts": "FIRE", "Holy Fire of Mithrandir": "FIRE",
    "Corpse Explosion": "FIRE",
    "Ice Storm": "COLD", "Frost Bolts": "COLD", "Cold Balls": "COLD",
    "Tidal Wave": "COLD", "Water Bite": "COLD",
    "Lightning Balls": "ELEC", "Lightning Bolts": "ELEC",
    "Thunderstorm": "ELEC",
    "Acid Balls": "ACID", "Acid Bolts": "ACID",
    "Noxious Cloud": "POIS", "Poison Blood": "POIS",
}

# Which god a Prayer spell belongs to ("" = generic Prayer spell).
SPELL_GOD = {
    "See the Music": "Eru", "Listen to the Music": "Eru",
    "Lay of Protection": "Eru",
    "Manwe's Blessing": "Manwe", "Wind Shield": "Manwe",
    "Manwe's Call": "Manwe",
    "Divine Aim": "Tulkas", "Whirlwind": "Tulkas", "Wave of Power": "Tulkas",
    "Curse": "Melkor", "Corpse Explosion": "Melkor", "Mind Steal": "Melkor",
    "Charm Animal": "Yavanna", "Grow Grass": "Yavanna", "Water Bite": "Yavanna",
    "Avatar": "Manwe", "Tree Roots": "Yavanna", "Uproot": "Yavanna",
    "Firebrand": "Aule", "Enchant Weapon": "Aule", "Enchant Armour": "Aule",
    "Child of Aule": "Aule",
    "Light of Valinor": "Varda", "Call of Almaren": "Varda",
    "Evenstar": "Varda", "Star Kindler": "Varda",
    "Song of Belegaer": "Ulmo", "Draught of Ulmonan": "Ulmo",
    "Call of the Ulumuri": "Ulmo", "Wrath of Ulmo": "Ulmo",
    "Tears of Luthien": "Mandos", "Feanturi": "Mandos",
    "Tale of Doom": "Mandos", "Call to the Halls": "Mandos",
}

# Device tables (spells5.cc spell_type_set_device_charges +
# device_allocation): name -> (charges dice, [(tval, rarity, base_min,
# base_max, max_min, max_max)]). TV_WAND=65, TV_STAFF=55. Used for stick
# generation charges, pval3 level ranges and get_random_stick.
WAND, STAFF = 65, 55
DEVICE_SPELLS = {
    "Artifact Thunderlords": ("3+d3", [(STAFF, 999, 1, 1, 1, 1)]),
    "Globe of Light": ("10+d5", [(STAFF, 7, 1, 15, 10, 45)]),
    "Fireflash": ("5+d5", [(WAND, 35, 1, 15, 15, 35)]),
    "Fiery Shield": ("3+d5", [(STAFF, 50, 1, 10, 5, 40)]),
    "Firewall": ("4+d5", [(WAND, 55, 1, 10, 5, 40)]),
    "Manathrust": ("7+d10", [(WAND, 5, 1, 20, 15, 33)]),
    "Remove Curses": ("3+d8", [(STAFF, 70, 1, 5, 15, 50)]),
    "Tidal Wave": ("6+d5", [(WAND, 54, 1, 10, 20, 50)]),
    "Ice Storm": ("3+d7", [(WAND, 65, 1, 5, 25, 45)]),
    "Noxious Cloud": ("5+d7", [(WAND, 15, 1, 15, 25, 50)]),
    "Wings of Winds": ("7+d5", [(STAFF, 27, 1, 10, 20, 50)]),
    "Poison Blood": ("10+d15", [(WAND, 45, 1, 25, 35, 50)]),
    "Thunderstorm": ("5+d5", [(WAND, 85, 1, 5, 25, 50)]),
    "Sterilize": ("7+d5", [(STAFF, 20, 1, 10, 20, 50)]),
    "Dig": ("15+d5", [(WAND, 25, 1, 1, 1, 1)]),
    "Stone Prison": ("5+d3", [(WAND, 57, 1, 3, 5, 20)]),
    "Strike": ("2+d6", [(WAND, 635, 1, 5, 10, 50)]),
    "Shake": ("5+d10", [(STAFF, 75, 1, 3, 9, 20)]),
    "Teleportation": ("7+d7", [(STAFF, 50, 1, 20, 20, 50)]),
    "Teleport Away": ("3+d5", [(WAND, 75, 1, 20, 20, 50)]),
    "Probability Travel": ("1+d2", [(STAFF, 97, 1, 5, 8, 25)]),
    "Healing": ("2+d3", [(STAFF, 90, 1, 5, 20, 40)]),
    "Recovery": ("5+d10", [(STAFF, 50, 1, 5, 10, 30)]),
    "Summon Animal": ("1+d3", [(WAND, 85, 1, 5, 15, 45)]),
    "Vision": ("4+d6", [(STAFF, 60, 1, 5, 10, 30)]),
    "Sense Hidden": ("1+d15", [(STAFF, 20, 1, 15, 10, 50)]),
    "Reveal Ways": ("6+d6", [(STAFF, 35, 1, 15, 25, 50)]),
    "Sense Monsters": ("5+d10", [(STAFF, 37, 1, 10, 15, 40)]),
    "Magelock": ("7+d5", [(WAND, 30, 1, 5, 15, 45)]),
    "Slow Monster": ("5+d5", [(WAND, 23, 1, 15, 20, 50)]),
    "Essence of Speed": ("3+d3", [(WAND, 80, 1, 1, 10, 39)]),
    "Banishment": ("1+d3", [(WAND, 98, 1, 15, 10, 36)]),
    "Disperse Magic": ("5+d5", [(WAND, 25, 1, 15, 5, 40)]),
    "Charm": ("7+d5", [(WAND, 35, 1, 15, 20, 40)]),
    "Confuse": ("3+d4", [(WAND, 45, 1, 5, 20, 40)]),
    "Genocide": ("2+d2", [(STAFF, 85, 1, 1, 5, 15)]),
    "Demon Blade": ("3+d7", [(WAND, 75, 1, 17, 20, 40)]),
    "Heal Monster": ("10+d10", [(WAND, 17, 1, 15, 20, 50)]),
    "Haste Monster": ("10+d5", [(WAND, 7, 1, 1, 20, 50)]),
    "Wish": ("1+d2", [(STAFF, 98, 1, 1, 1, 1)]),
    "Summon": ("1+d20", [(STAFF, 13, 1, 40, 25, 50)]),
    "Mana": ("2+d3", [(STAFF, 78, 1, 5, 20, 35)]),
    "Nothing": ("0+d0", [(STAFF, 3, 1, 1, 1, 1), (WAND, 3, 1, 1, 1, 1)]),
    "Holy Fire of Mithrandir": ("2+d5", [(STAFF, 999, 1, 1, 35, 35)]),
}

# Class -> school ids mapping lives in bevy/src/spell.rs (simplified from
# the ToME skill trees); school ids are the s_info N: indexes.

# --- Pref-file (fixed map) parser ----------------------------------------
# ToME fixed maps (towns, quest levels) are text files resolved by
# process_dungeon_file() (src/init1.cc): '%:' includes, '?:' conditions,
# 'F:' per-character feature definitions, 'D:' layout rows, 'P:' start.
# Conditions are evaluated with fixed values (Bree, daytime, every quest
# untaken); the only dynamic originals (thieves hideout becoming a house,
# troll/wight entrances appearing at night/after taking the quest) are
# simplified: the entrances are always visible.
PREF_VARS = {'TOWN': 1, 'DAYTIME': 1, 'LEAVING_QUEST': 0,
             'TOWN_DESTROY1': 0, 'TOWN_DESTROY2': 0, 'TOWN_DESTROY3': 0,
             'TOWN_DESTROY4': 0, 'TOWN_DESTROY5': 0,
             'QUEST4': 0, 'QUEST8': 0, 'QUEST9': 0, 'QUEST18': 0}


def eval_cond(expr, variables):
    """Evaluate a pref '?:' expression: [EQU $VAR n]/[NOT ..]/[AND .. ..]/[OR .. ..]."""
    # Keep quoted variable names (e.g. $QUEST"Library quest") as one token.
    expr = re.sub(r'"[^"]*"', lambda m: m.group(0).replace(' ', '\x00'), expr)
    toks = [t.replace('\x00', ' ') for t in
            expr.replace('[', ' [ ').replace(']', ' ] ').split()]
    pos = [0]

    def parse():
        tok = toks[pos[0]]
        pos[0] += 1
        if tok != '[':
            raise ValueError(f'bad condition: {expr}')
        op = toks[pos[0]]
        pos[0] += 1
        if op == 'EQU':
            var = toks[pos[0]]
            val = int(toks[pos[0] + 1])
            pos[0] += 3  # var, value, ']'
            return variables.get(var.lstrip('$'), 0) == val
        if op == 'NOT':
            v = not parse()
            pos[0] += 1  # ']'
            return v
        if op in ('AND', 'OR'):
            a = parse()
            b = parse()
            pos[0] += 1  # ']'
            return (a and b) if op == 'AND' else (a or b)
        raise ValueError(f'bad condition op {op}: {expr}')

    return parse()


def parse_pref(path, variables):
    """Parse a pref/fixed-map file into (char defs, layout rows, start).

    char defs: ch -> dict(terrain, cave, monster, object, special, mflag)
    start: (x, y) from P:<y>:<x> (or None).
    """
    chars, rows = {}, []
    start = [None]

    # The original keeps one flat "bypass" flag per file (?: sets it,
    # ?:1 clears it; includes start fresh).
    def rec(p):
        active = True
        for raw in open(p, encoding='latin-1'):
            line = raw.rstrip('\n')
            if not line or line.startswith('#'):
                continue
            if line.startswith('?:'):
                e = line[2:].strip()
                active = True if e == '1' else eval_cond(e, variables)
                continue
            if not active:
                continue
            if line.startswith('%:'):
                rec(os.path.join(os.path.dirname(p), line[2:].strip()))
                continue
            if line.startswith('F:'):
                f = line[2:].split(':')

                def num(i):
                    if i >= len(f):
                        return 0
                    v = f[i].lstrip('*')
                    return int(v) if v.lstrip('-').isdigit() else 0

                def rand(i):
                    return i < len(f) and f[i].startswith('*')

                chars[f[0]] = {'terrain': num(1), 'cave': num(2),
                               'monster': num(3), 'object': num(4),
                               'artifact': num(6), 'special': num(7),
                               'mimic': num(8), 'mflag': num(9),
                               'monster_random': rand(3),
                               'object_random': rand(4),
                               'artifact_random': rand(6)}
            elif line.startswith('D:'):
                rows.append(line[2:])
            elif line.startswith('P:'):
                _, y, x = line.split(':')
                start[0] = (int(x), int(y))

    rec(path)
    return chars, rows, start[0]


def pref_grid(chars, rows):
    """Yield (x, y, ch, celldef) for every layout character."""
    blank = {'terrain': 0, 'cave': 0, 'monster': 0, 'object': 0, 'special': 0,
             'mimic': 0, 'mflag': 0, 'monster_random': False,
             'object_random': False, 'artifact_random': False}
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            yield x, y, ch, chars.get(ch, blank)


# Map dimensions (the full Bree layout, no cropping: 198x66 cells).
MAP_W, MAP_H = 198, 66
# Quest ids (src/defines.hpp): thieves / troll glade / wights grave / shrooms.
QUEST_THIEVES, QUEST_TROLL, QUEST_WIGHT, QUEST_SHROOM = 4, 8, 9, 18
# Where the completion staircase appears (original quest scripts);
# quests not listed default to their entry point.
QUEST_EXITS = {QUEST_THIEVES: (23, 4), QUEST_TROLL: (3, 3), QUEST_WIGHT: (3, 3)}

# Additional fixed quest levels ported from lib/edit/*.map (see q_*.cc):
# (quest id, file).  22 wolves, 10 spiders, 24 haunted house, 23 dragon
# lair, 25 evil cave, 28 library, 16 the Between ambush, 27 fireproof
# scroll vault, 14 nirnaeth troll battle, 15 Maeglin's escape, 19 the
# prisoner of Dol Guldur (embedded as a dungeon vault, not an entrance),
# and the 8 princess-room layouts of the random quest (ids 200+type).
EXTRA_QUEST_MAPS = [
    (22, 'wolves.map'), (10, 'spiders.map'), (24, 'haunted.map'),
    (23, 'dragons.map'), (25, 'evil.map'), (28, 'library.map'),
    (16, 'between.map'), (27, 'fireprof.map'), (14, 'nirnaeth.map'),
    (15, 'maeglin.map'), (19, 'thrain.map'),
    (201, 'qrand1.map'), (205, 'qrand5.map'), (206, 'qrand6.map'),
    (207, 'qrand7.map'), (210, 'qrand10.map'), (211, 'qrand11.map'),
    (212, 'qrand12.map'), (214, 'qrand14.map'),
]

# d_info.txt '@:' special final levels of the original's side dungeons;
# our single dungeon uses them as fixed set-piece levels at these depths.
SPEC_LEVELS = [
    (3, 's_crypt.map'), (5, 's_orc.map'), (11, 's_death.map'),
    (14, 's_doom.map'), (15, 's_factory.map'), (16, 's_ship.map'),
    (18, 's_gates.map'), (28, 's_name.map'),
]

# Seed for the wilderness underlay plasma (the original draws one from the
# system RNG per game; we fix one so the layout is stable.  Seed 2 keeps
# every shop/entrance reachable on foot).
WILD_SEED = 2
# MAX_WILD_TERRAIN (src/terrain.hpp).
MAX_WILD_TERRAIN = 18


def parse_wf_bree_table(path):
    """The Bree sector's fractal-height -> terrain table (wf_info.txt)."""
    cur = None
    for line in open(path, encoding='latin-1'):
        line = line.rstrip('\n')
        if line.startswith('N:'):
            _, idx, _name = line.split(':', 2)
            cur = int(idx)
        elif line.startswith('X:') and cur == 1:
            table = [int(x) for x in line[2:].split(':')]
            assert len(table) == MAX_WILD_TERRAIN
            return table
    raise SystemExit('wf_info.txt: Bree record not found')


def plasma_underlay(w, h, table, seed):
    """The original wilderness underlay (src/wild.cc): every wilderness
    level is first filled with a plasma fractal heightfield (corners
    random, plasma_recursive with roughness 1), mapped through the
    sector's wf_info terrain table.  The town file is then overlaid,
    skipping ' ' cells (init1.cc:6279), which keeps this terrain."""
    rng = random.Random(seed)
    c = [[MAX_WILD_TERRAIN // 2] * w for _ in range(h)]
    c[1][1] = rng.randrange(MAX_WILD_TERRAIN)
    c[h - 2][1] = rng.randrange(MAX_WILD_TERRAIN)
    c[1][w - 2] = rng.randrange(MAX_WILD_TERRAIN)
    c[h - 2][w - 2] = rng.randrange(MAX_WILD_TERRAIN)

    def perturb_mid(x1, x2, x3, x4, xmid, ymid, rough, depth_max):
        # ToME randint(m) is 1..m, so the perturbation is +/- rough.
        tmp = rng.randrange(1, rough * 2 + 2) - (rough + 1)
        avg = ((x1 + x2 + x3 + x4) // 4) + tmp
        if ((x1 + x2 + x3 + x4) % 4) > 1:
            avg += 1
        c[ymid][xmid] = min(max(avg, 0), depth_max)

    def perturb_end(x1, x2, x3, xmid, ymid, rough, depth_max):
        tmp = rng.randrange(1, rough * 2 + 2) - (rough + 1)
        avg = ((x1 + x2 + x3) // 3) + tmp
        if (x1 + x2 + x3) % 3:
            avg += 1
        c[ymid][xmid] = min(max(avg, 0), depth_max)

    def plasma(x1, y1, x2, y2, depth_max, rough):
        xmid = (x2 - x1) // 2 + x1
        ymid = (y2 - y1) // 2 + y1
        if x1 + 1 == x2:
            return
        perturb_mid(c[y1][x1], c[y2][x1], c[y1][x2], c[y2][x2],
                    xmid, ymid, rough, depth_max)
        perturb_end(c[y1][x1], c[y1][x2], c[ymid][xmid], xmid, y1, rough, depth_max)
        perturb_end(c[y1][x2], c[y2][x2], c[ymid][xmid], x2, ymid, rough, depth_max)
        perturb_end(c[y2][x2], c[y2][x1], c[ymid][xmid], xmid, y2, rough, depth_max)
        perturb_end(c[y2][x1], c[y1][x1], c[ymid][xmid], x1, ymid, rough, depth_max)
        plasma(x1, y1, xmid, ymid, depth_max, rough)
        plasma(xmid, y1, x2, ymid, depth_max, rough)
        plasma(x1, ymid, xmid, y2, depth_max, rough)
        plasma(xmid, ymid, x2, y2, depth_max, rough)

    plasma(1, 1, w - 2, h - 2, MAX_WILD_TERRAIN - 1, 1)
    return [[table[c[y][x]] for x in range(w)] for y in range(h)]


def walkable_fn(terrains):
    flags = {t['id']: t['flags'] for t in terrains}

    def walkable(tid):
        fl = flags.get(tid, set())
        return 'FLOOR' in fl and 'NO_WALK' not in fl

    return walkable


def nearest_walkable(cells, walkable, want, exclude=set()):
    """Spiral out from `want` to the closest walkable cell in bounds."""
    for r in range(0, 40):
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if max(abs(dx), abs(dy)) != r:
                    continue
                x, y = want[0] + dx, want[1] + dy
                if not (0 <= x < MAP_W and 0 <= y < MAP_H):
                    continue
                if (x, y) in exclude:
                    continue
                if walkable(cells[y][x]):
                    return (x, y)
    raise SystemExit(f'no walkable cell near {want}')


def build_bree(terrains):
    chars, rows, start = parse_pref(os.path.join(TOME_LIB, 't_info.txt'),
                                    PREF_VARS)
    assert rows, 't_info.txt did not yield the Bree layout'
    # The troll/wight entrances are trees in the base map; the original
    # reveals them via conditionals (trolls: quest taken AND night,
    # wights: quest taken).  Their positions go to quest_cells and the
    # runtime swaps terrain 96 (tree) <-> 8 (entrance).
    # The plasma wilderness underlay: the town file is overlaid on top of
    # it and ' ' cells are skipped, keeping the wilderness (init1.cc:6279).
    # Bree's sector table (wf_info.txt N:1) maps heights 0-1 to dirt, 2-5
    # to grass and 6-17 to trees -- the big forests around the village.
    bree_table = parse_wf_bree_table(os.path.join(TOME_LIB, 'wf_info.txt'))
    cells = plasma_underlay(MAP_W, MAP_H, bree_table, WILD_SEED)
    lits = [[True] * MAP_W for _ in range(MAP_H)]
    shops, entrances, buildings, quest_cells = [], [], [], []
    for x, y, ch, c in pref_grid(chars, rows):
        if not (0 <= x < MAP_W and 0 <= y < MAP_H):
            continue
        if ch == ' ':
            continue  # keep the plasma wilderness underneath
        cells[y][x] = c['terrain']
        lits[y][x] = bool(c['cave'] & 2)
        if ch == 'y':
            quest_cells.append((x, y, QUEST_TROLL, True))
        elif ch == 'x':
            quest_cells.append((x, y, QUEST_WIGHT, False))
        if c['terrain'] == 8:
            entrances.append((x, y, c['special']))
        elif c['terrain'] == 74 and 0 <= c['special'] <= 8:
            shops.append((x, y, c['special']))
        elif c['terrain'] in (74, 75) and c['special'] > 8:
            buildings.append((x, y, c['special']))
    walkable = walkable_fn(terrains)
    # The mushroom quest has no .map entrance in the original (it lives in
    # the wilderness west of Bree); add one on the west edge.
    for x in range(1, 30):
        spot = None
        for y in sorted(range(1, MAP_H - 1), key=lambda yy: abs(yy - 32)):
            if walkable(cells[y][x]):
                spot = (x, y)
                break
        if spot:
            entrances.append((spot[0], spot[1], QUEST_SHROOM))
            cells[spot[1]][spot[0]] = 8
            lits[spot[1]][spot[0]] = True
            break
    # Foreign quests (originally other towns' plots) become wilderness
    # paths out of Bree, placed on the map edges.  Their availability is
    # gated at runtime (chain prerequisites, e.g. spiders before poison).
    edge_quests = [
        ('N', 22), ('N', 25),                    # wolves, evil
        ('E', 10), ('E', 28), ('E', 27),         # spiders, library, fireproof
        ('S', 24), ('S', 11), ('S', 16),         # haunted, poison, between
        ('W', 23), ('W', 13), ('W', 14), ('W', 15),  # dragons, eol,
                                                     # nirnaeth, invasion
    ]
    taken_xy = {(x, y) for x, y, _ in shops} | {(x, y) for x, y, _ in entrances}
    for edge, qid in edge_quests:
        inward = {'N': (0, 1), 'S': (0, -1), 'E': (-1, 0), 'W': (1, 0)}[edge]
        spot = None
        if edge == 'N':
            scan = ((x, y) for y in range(1, 6)
                    for x in range(20, MAP_W - 20))
        elif edge == 'S':
            scan = ((x, y) for y in range(MAP_H - 2, MAP_H - 7, -1)
                    for x in range(20, MAP_W - 20))
        elif edge == 'E':
            scan = ((x, y) for x in range(MAP_W - 2, MAP_W - 7, -1)
                    for y in range(4, MAP_H - 4))
        else:
            scan = ((x, y) for x in range(1, 6)
                    for y in sorted(range(2, MAP_H - 2),
                                    key=lambda yy: abs(yy - 20)))
        # Prefer a cell whose inward neighbour is walkable too (so the
        # entrance is not walled in by the plasma forest).
        for x, y in scan:
            if (x, y) not in taken_xy and walkable(cells[y][x]) \
                    and walkable(cells[y + inward[1]][x + inward[0]]):
                spot = (x, y)
                break
        if spot is None:
            for x, y in scan:
                if (x, y) not in taken_xy and walkable(cells[y][x]):
                    spot = (x, y)
                    break
        if spot is None:
            raise SystemExit(f'no edge spot for quest {qid}')
        x, y = spot
        entrances.append((x, y, qid))
        cells[y][x] = 8
        lits[y][x] = True
        taken_xy.add((x, y))
        # Carve a short path inland through the forest so the entrance is
        # reachable on foot (the plasma underlay can wall off edge cells).
        for step in range(1, 7):
            px, py = x + inward[0] * step, y + inward[1] * step
            if cells[py][px] == 96:  # tree -> grass
                cells[py][px] = 89
                lits[py][px] = True
    # Fall back to a walkable cell if the chosen spot is blocked.
    taken = {(x, y) for x, y, _ in shops} | {(x, y) for x, y, _ in entrances}
    # The original ignores the town's P: for a fresh character:
    # wilderness_gen pre-seeds oldpx/oldpy with the map centre
    # (MAX_WID/2, MAX_HGT/2) BEFORE t_info.txt is parsed, and P: only
    # applies when they are still 0 (src/wild.cc:179-180 vs
    # src/init1.cc:6561).  So the real spawn is the map centre, next to
    # the Soothsayer/Temple buildings.  (Quest maps' P: does apply.)
    start = (MAP_W // 2, MAP_H // 2)
    if not walkable(cells[start[1]][start[0]]) or start in taken:
        start = nearest_walkable(cells, walkable, start, taken)
    out_cells_before = None
    # Guarantee every shop/entrance is reachable from the start: flood
    # fill from the start, then carve grass paths through the forest
    # toward the connected region for anything left out.
    def connected():
        seen = set()
        stack = [start]
        seen.add(start)
        while stack:
            x, y = stack.pop()
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if 0 <= nx < MAP_W and 0 <= ny < MAP_H \
                        and (nx, ny) not in seen and walkable(cells[ny][nx]):
                    seen.add((nx, ny))
                    stack.append((nx, ny))
        return seen

    targets = [(x, y) for x, y, _ in shops] + [(x, y) for x, y, _ in entrances]
    for tx, ty in targets:
        # Walk from the target toward the map centre, turning trees into
        # grass, until the path joins the connected region.
        cx, cy = tx, ty
        for _ in range(MAP_W + MAP_H):
            if (cx, cy) in connected():
                break
            dx, dy = MAP_W // 2 - cx, MAP_H // 2 - cy
            options = []
            if dy:
                options.append((0, 1 if dy > 0 else -1))
            if dx:
                options.append((1 if dx > 0 else -1, 0))
            carved = False
            for sx, sy in options:
                nx, ny = cx + sx, cy + sy
                if not (0 <= nx < MAP_W and 0 <= ny < MAP_H):
                    continue
                if walkable(cells[ny][nx]) or cells[ny][nx] == 96:
                    if cells[ny][nx] == 96:
                        cells[ny][nx] = 89
                        lits[ny][nx] = True
                    cx, cy = nx, ny
                    carved = True
                    break
            if not carved:
                raise SystemExit(f'cannot carve path for {(tx, ty)}')
    assert all(t in connected() for t in targets), 'unreachable shop/entrance'
    out_cells = [(cells[y][x], lits[y][x]) for y in range(MAP_H) for x in range(MAP_W)]
    return {'cells': out_cells, 'shops': shops, 'entrances': entrances,
            'buildings': buildings, 'start': start, 'quest_cells': quest_cells}


def build_quest_map(path, quest_id, keep_ids):
    chars, rows, start = parse_pref(path, PREF_VARS)
    w = max(len(r) for r in rows)
    h = len(rows)
    cells, monsters, objects, artifacts = [], [], [], []
    random_monsters, random_objects, mimics = [], [], []
    marker = None
    for x, y, _ch, c in pref_grid(chars, rows):
        t = c['terrain']
        if t == 160:
            t = 1  # between-gates are plain floor in this port
        if t == 172:
            # The Marker grid: quest-specific spot (player entry in
            # maeglin.map, quest monster in qrand*.map, Thrain's cell in
            # thrain.map); the cell itself is plain floor.
            marker = (x, y)
            t = 1
        cells.append((t, bool(c['cave'] & 2)))
        # F: `*` markers place a random monster/object at
        # quest level + offset (init1.cc process_dungeon_file_aux
        # RANDOM_MONSTER / RANDOM_OBJECT).  RANDOM_FEATURE/_EGO/
        # _ARTIFACT are parsed by the original but never used when
        # placing the grid.
        if c.get('monster_random'):
            random_monsters.append((c['monster'], x, y))
        elif c['monster'] > 0:
            monsters.append((c['monster'], x, y, bool(c['mflag'] & 2)))
        if c.get('object_random'):
            random_objects.append((c['object'], x, y))
        elif c['object'] > 0 and c['object'] in keep_ids:
            objects.append((c['object'], x, y))
        # M:<terrain>: the grid is drawn as another feature (cave mimic).
        if c.get('mimic', 0):
            mimics.append((x, y, c['mimic']))
        # F:<ch>:...:<artifact>:... places a specific a_info artifact
        # (init1.cc: a_allow_special -> object_prep -> name1).  Only the
        # d_info special levels use it in the base data (Narya in
        # s_gates.map).
        if c.get('artifact', 0) > 0:
            artifacts.append((c['artifact'], x, y))
    if start is None:
        start = marker
    assert len(cells) == w * h and start, f'{path}: bad layout'
    return {'quest': quest_id, 'w': w, 'h': h, 'cells': cells,
            'monsters': monsters, 'objects': objects, 'artifacts': artifacts,
            'random_monsters': random_monsters, 'random_objects': random_objects,
            'mimics': mimics,
            'start': start, 'exit': QUEST_EXITS.get(quest_id, start),
            'marker': marker or start}




def main():

    os.makedirs(OUT_DIR, exist_ok=True)

    terrains = parse_f_info(os.path.join(TOME_LIB, 'f_info.txt'))
    with open(os.path.join(OUT_DIR, 'terrain.ron'), 'w') as f:
        f.write('[\n')
        for t in terrains:
            fl = t['flags']
            effects = ', '.join('(dd:%d, ds:%d, freq:%d, typ:%s)'
                                % (e[0], e[1], e[2], ron_str(e[3]))
                                for e in t['effects'])
            f.write('    (id:%d, name:%s, ch:%s, color:%d, '
                    'is_floor:%s, no_walk:%s, no_vision:%s, is_wall:%s, '
                    'tunnelable:%s, permanent:%s, remember:%s, '
                    'support_growth:%s, can_fly:%s, can_levitate:%s, '
                    'can_climb:%s, can_pass:%s, web:%s, can_run:%s, '
                    'notice:%s, dont_notice_running:%s, door:%s, '
                    'support_light:%s, attr_multi:%s, desc:%s, '
                    'tunnel_desc:%s, block_desc:%s, mimic:%d, '
                    'effects:[%s]),\n' % (
                        t['id'], ron_str(t['name']), ron_str(t['ch']), t['color'],
                        str('FLOOR' in fl).lower(), str('NO_WALK' in fl).lower(),
                        str('NO_VISION' in fl).lower(), str('WALL' in fl).lower(),
                        str('TUNNELABLE' in fl).lower(), str('PERMANENT' in fl).lower(),
                        str('REMEMBER' in fl).lower(),
                        str('SUPPORT_GROWTH' in fl).lower(),
                        str('CAN_FLY' in fl).lower(),
                        str('CAN_LEVITATE' in fl).lower(),
                        str('CAN_CLIMB' in fl).lower(),
                        str('CAN_PASS' in fl).lower(),
                        str('WEB' in fl).lower(),
                        str('CAN_RUN' in fl).lower(),
                        str('NOTICE' in fl).lower(),
                        str('DONT_NOTICE_RUNNING' in fl).lower(),
                        str('DOOR' in fl).lower(),
                        str('SUPPORT_LIGHT' in fl).lower(),
                        str('ATTR_MULTI' in fl).lower(),
                        ron_str(t['desc']), ron_str(t['tunnel_desc']),
                        ron_str(t['block_desc']), t['mimic'], effects))
        f.write(']\n')
    print(f'terrain.ron: {len(terrains)} entries')

    monsters = parse_r_info(os.path.join(TOME_LIB, 'r_info.txt'))
    # The d_info block is parsed here (rather than with the other dungeon
    # output) so guardian races can be marked exactly like init2.cc
    # init_guardians before monsters.ron is written.
    dungeons = parse_d_info(os.path.join(TOME_LIB, 'd_info.txt'))
    by_id = {m['id']: m for m in monsters}
    for d in dungeons:
        g = d['guardian']
        if not g or g not in by_id:
            continue
        # RF_SPECIAL_GENE: guardians never appear in the random pool.
        by_id[g]['flags'].add('SPECIAL_GENE')
        # RF_DROP_RANDART: dungeons with no final artifact/object give
        # their guardian a random artifact (init2.cc:922-926).
        if not d['final_artifact'] and not d['final_object']:
            by_id[g]['flags'].add('DROP_RANDART')
    with open(os.path.join(OUT_DIR, 'monsters.ron'), 'w') as f:
        f.write('[\n')
        for m in monsters:
            blows = ', '.join('(method:%s, effect:%s, dice:%s)'
                              % (ron_str(b[0]), ron_str(b[1]), ron_str(b[2]))
                              for b in m['blows'])
            spells = ', '.join(ron_str(s) for s in m['spells'])
            flags = ', '.join(ron_str(x) for x in sorted(m['flags']))
            body = ', '.join(str(x) for x in m['body_parts'])
            f.write('    (id:%d, name:%s, ch:%s, color:%d, speed:%d, hp:%s, '
                    'ac:%d, alert:%d, aaf:%d, depth:%d, rarity:%d, weight:%d, exp:%d, '
                    'blows:[%s], '
                    'spell_freq:%d, spells:[%s], open_door:%s, bash_door:%s, '
                    'never_move:%s, unique:%s, friends:%s, escort:%s, '
                    'artifact_idx:%d, artifact_chance:%d, objs:(%d, %d, %d, %d), '
                    'flags:[%s], hdice:%s, hside:%s, body_parts:(%s), '
                    'desc:%s),\n' % (
                        m['id'], ron_str(m['name']), ron_str(m['ch']), m['color'],
                        m['speed'], ron_str(m['hp']), m['ac'], m['alert'],
                        m['aaf'], m['depth'], m['rarity'], m['weight'], m['exp'], blows,
                        m['spell_freq'], spells,
                        str('OPEN_DOOR' in m['flags']).lower(),
                        str('BASH_DOOR' in m['flags']).lower(),
                        str('NEVER_MOVE' in m['flags']).lower(),
                        str('UNIQUE' in m['flags']).lower(),
                        str('FRIENDS' in m['flags']).lower(),
                        str('ESCORT' in m['flags'] or 'ESCORTS' in m['flags']).lower(),
                        m['artifact_idx'], m['artifact_chance'],
                        m['objs'][0], m['objs'][1], m['objs'][2], m['objs'][3],
                        flags, ron_str(m['hdice']), ron_str(m['hside']), body,
                        ron_str(''.join(m['desc']))))
        f.write(']\n')
    print(f'monsters.ron: {len(monsters)} entries')

    races, classes, general = parse_p_info(
        os.path.join(TOME_LIB, 'p_info.txt'))
    # Bard/Possessor/Summoner/Druid/... are p_info *specialisations*
    # (C:a: records) of the six base classes; every class carries its
    # own spec list (see classes.ron).

    # The skill system is data-wide: general skill modifiers apply to every
    # character, then race, class (p_info G:k:/R:k:/C:k: lines).
    skills = parse_skills(os.path.join(TOME_LIB, 's_info.txt'))
    abilities = parse_abilities(os.path.join(TOME_LIB, 'ab_info.txt'))
    with open(os.path.join(OUT_DIR, 'skills.ron'), 'w') as f:
        f.write('[\n')
        for s in skills:
            desc = ron_str('\n'.join(s['desc']))
            flags = ', '.join(ron_str(x) for x in s['flags'])
            inc = ', '.join('(%d, %d)' % (i, p) for i, p in s['increases'])
            exc = ', '.join(str(i) for i in s['excludes'])
            f.write('    (id:%d, name:%s, desc:%s, father:%d, order:%d, '
                    'flags:[%s], action_mkey:%d, action_desc:%s, chance:%d, '
                    'increases:[%s], excludes:[%s]),\n'
                    % (s['id'], ron_str(s['name']), desc, s['father'],
                       s['order'], flags, s['action_mkey'],
                       ron_str(s['action_desc']), s['chance'], inc, exc))
        f.write(']\n')
    print(f'skills.ron: {len(skills)} entries')

    with open(os.path.join(OUT_DIR, 'general_skills.ron'), 'w') as f:
        f.write('[\n')
        for n, bo, b, mo, g in general['skills']:
            f.write('    (skill:%s, bop:%s, base:%d, mop:%s, gain:%d),\n'
                    % (ron_str(n), ron_str(bo), b, ron_str(mo), g))
        f.write(']\n')
    print(f'general_skills.ron: {len(general["skills"])} entries')

    with open(os.path.join(OUT_DIR, 'abilities.ron'), 'w') as f:
        f.write('[\n')
        for a in abilities:
            desc = ron_str('\n'.join(a['desc']))
            need = ', '.join('(%s, %d)' % (ron_str(n), l)
                             for n, l in a['need_skills'])
            stats = ', '.join(str(x) for x in a['stats'])
            nabs = ', '.join(ron_str(x) for x in a['need_abilities'])
            f.write('    (id:%d, name:%s, desc:%s, cost:%d, action_mkey:%d, '
                    'action_desc:%s, need_skills:[%s], stats:(%s), '
                    'need_abilities:[%s]),\n'
                    % (a['id'], ron_str(a['name']), desc, a['cost'],
                       a['action_mkey'], ron_str(a['action_desc']), need,
                       stats, nabs))
        f.write(']\n')
    print(f'abilities.ron: {len(abilities)} entries')

    bactions = parse_ba_info(os.path.join(TOME_LIB, 'ba_info.txt'))
    with open(os.path.join(OUT_DIR, 'building_actions.ron'), 'w') as f:
        f.write('[\n')
        for a in bactions:
            f.write('    (id:%d, name:%s, costs:(%d, %d, %d), action:%d, '
                    'restr:%d, letter:%s, letter_aux:%s),\n'
                    % (a['id'], ron_str(a['name']), a['costs'][0],
                       a['costs'][1], a['costs'][2], a['action'], a['restr'],
                       ron_str(a['letter']), ron_str(a['letter_aux'])))
        f.write(']\n')
    print(f'building_actions.ron: {len(bactions)} entries')

    with open(os.path.join(OUT_DIR, 'races.ron'), 'w') as f:
        f.write('[\n')
        for r in races:
            stats = ', '.join(str(x) for x in r['stats'])
            cls = ', '.join(ron_str(c) for c in r['classes'])
            desc = ron_str(' '.join(r['desc']))
            sk = ron_skill_mods(r['skills'])
            ab = ron_level_abilities(r['abilities'])
            fl = ron_level_flags(r['flags'])
            objs = ron_object_protos(r['objects'])
            pfl = ', '.join(ron_str(f) for f in r['player_flags'])
            pw = ', '.join(ron_str(x) for x in r['powers'])
            body = ', '.join(str(x) for x in r['body_parts'])
            f.write('    (id:%d, name:%s, desc:%s, stats:(%s), mana:%d, '
                    'luck:%d, hitdie:%d, exp:%d, infra:%d, classes:[%s], '
                    'skills:[%s], abilities:[%s], flags:[%s], objects:[%s], '
                    'powers:[%s], player_flags:[%s], body_parts:(%s)),\n' % (
                        r['id'], ron_str(r['name']), desc, stats, r['mana'],
                        r.get('luck', 0), r['hitdie'], r['exp'],
                        r.get('infra', 0), cls, sk, ab, fl, objs, pw, pfl,
                        body))
        f.write(']\n')
    print(f'races.ron: {len(races)} entries')

    race_mods = parse_race_mods(os.path.join(TOME_LIB, 'p_info.txt'))
    with open(os.path.join(OUT_DIR, 'racemods.ron'), 'w') as f:
        f.write('[\n')
        for m in race_mods:
            stats = ', '.join(str(x) for x in m['stats'])
            body = ', '.join(str(x) for x in m['body_parts'])
            desc = ron_str(' '.join(m['desc']))
            races_a = ', '.join(ron_str(x) for x in m['races'])
            classes_a = ', '.join(ron_str(x) for x in m['classes'])
            classes_f = ', '.join(ron_str(x) for x in m['forbidden'])
            skm = ron_skill_mods(m['skills'])
            ab = ron_level_abilities(m['abilities'])
            fl = ron_level_flags(m['flags'])
            objs = ron_object_protos(m['objects'])
            pfl = ', '.join(ron_str(x) for x in m['player_flags'])
            pw = ', '.join(ron_str(x) for x in m['powers'])
            f.write('    (id:%d, name:%s, desc:%s, place:%s, stats:(%s), '
                    'luck:%d, mana:%d, hitdie:%d, exp:%d, infra:%d, '
                    'body_parts:(%s), races:[%s], classes:[%s], '
                    'forbidden_classes:[%s], skills:[%s], abilities:[%s], '
                    'flags:[%s], objects:[%s], powers:[%s], player_flags:[%s]),\n' % (
                        m['id'], ron_str(m['name']), desc,
                        str(m['place']).lower(), stats, m['luck'], m['mana'],
                        m['hitdie'], m['exp'], m['infra'], body, races_a,
                        classes_a, classes_f, skm, ab, fl, objs, pw, pfl))
        f.write(']\n')
    print(f'racemods.ron: {len(race_mods)} entries')

    with open(os.path.join(OUT_DIR, 'classes.ron'), 'w') as f:
        f.write('[\n')
        for c in classes:
            stats = ', '.join(str(x) for x in c['stats'])
            desc = ron_str(' '.join(c['desc']))
            sk = ron_skill_mods(c['skills'])
            ab = ron_level_abilities(c['abilities'])
            fl = ron_level_flags(c['flags'])
            objs = ron_object_protos(c['objects'])
            gods = ', '.join(ron_str(g) for g in c['gods'])
            specs = ', '.join(ron_spec(s) for s in c['specs'])
            pfl = ', '.join(ron_str(f) for f in c['player_flags'])
            pw = ', '.join(ron_str(x) for x in c['powers'])
            titles = ', '.join(ron_str(x) for x in c['titles'])
            body = ', '.join(str(x) for x in c['body_parts'])
            f.write('    (id:%d, name:%s, desc:%s, stats:(%s), mana:%d, '
                    'hitdie:%d, exp:%d, blows:%d, blow_num:%d, blow_wgt:%d, '
                    'blow_mul:%d, skills:[%s], abilities:[%s], '
                    'flags:[%s], objects:[%s], gods:[%s], specs:[%s], '
                    'powers:[%s], player_flags:[%s], titles:[%s], '
                    'body_parts:(%s)),\n' % (
                        c['id'], ron_str(c['name']), desc, stats, c['mana'],
                        c['hitdie'], c['exp'], c['blows'], c['blow_num'],
                        c['blow_wgt'], c['blow_mul'], sk, ab, fl, objs, gods,
                        specs, pw, pfl, titles, body))
        f.write(']\n')
    print(f'classes.ron: {len(classes)} entries, '
          f'{sum(len(c["specs"]) for c in classes)} specialisations, '
          f'general skills {len(general["skills"])}')

    items = parse_k_info(os.path.join(TOME_LIB, 'k_info.txt'))
    # The One Ring (k_info 664, TV_PARCHMENT), corpses (TV_CORPSE 641-645)
    # and the per-god relic pieces (TV_JUNK 814-818) are all real k_info
    # entries now; they are excluded from random generation in Rust
    # (gen_object_kind) and only appear through their plot hooks.
    with open(os.path.join(OUT_DIR, 'items.ron'), 'w') as f:
        f.write('[\n')
        for o in items:
            lite = 2 if 'LITE2' in o['flags'] else (1 if 'LITE1' in o['flags'] else 0)
            flags = ', '.join(ron_str(x) for x in sorted(o['flags']))
            alloc = ', '.join('(%d, %d)' % a for a in o['alloc'])
            f.write('    (id:%d, name:%s, ch:%s, color:%d, tval:%d, sval:%d, '
                    'pval:%d, fuel:%d, spell:%s, depth:%d, rarity:%d, weight:%d, '
                    'cost:%d, ac:%d, dice:%s, to_h:%d, to_d:%d, to_a:%d, '
                    'lite:%d, activate:%s, btval:%d, bsval:%d, alloc:[%s], '
                    'flags:[%s], desc:[%s]),\n' % (
                        o['id'], ron_str(o['name']), ron_str(o['ch']), o['color'],
                        o['tval'], o['sval'], o['pval'], o['fuel'],
                        ron_str(o['spell']), o['depth'],
                        o['rarity'], o['weight'], o['cost'], o['ac'],
                        ron_str(o['dice']), o['to_h'], o['to_d'], o['to_a'], lite,
                        ron_str(o['activate']), o['btval'], o['bsval'], alloc,
                        flags,
                        ', '.join(ron_str(d) for d in o['desc'])))
        f.write(']\n')
    print(f'items.ron: {len(items)} entries')

    stores = parse_st_info(os.path.join(TOME_LIB, 'st_info.txt'))
    with open(os.path.join(OUT_DIR, 'stores.ron'), 'w') as f:
        f.write('[\n')
        for s in stores:
            flags = ', '.join(ron_str(x) for x in sorted(s['flags']))
            owners = ', '.join(str(o) for o in s.get('owners', []))
            actions = ', '.join(str(a) for a in s.get('actions', []))
            f.write('    (id:%d, name:%s, ch:%s, color:%d, max_items:%d, '
                    'flags:[%s], owners:[%s], actions:[%s], entries:[\n'
                    % (s['id'], ron_str(s['name']), ron_str(s['ch']), s['color'],
                       s['max_items'], flags, owners, actions))
            for proba, name, tval, sval in s['entries']:
                f.write('        (proba:%d, name:%s, tval:%d, sval:%d),\n'
                        % (proba, ron_str(name), tval, sval))
            f.write('    ]),\n')
        f.write(']\n')
    print(f'stores.ron: {len(stores)} entries')

    # store owners: race/class price preferences (ow_info.txt)
    owners = parse_ow_info(os.path.join(TOME_LIB, 'ow_info.txt'))
    with open(os.path.join(OUT_DIR, 'owners.ron'), 'w') as f:
        f.write('[\n')
        for o in owners:
            liked = ', '.join(ron_str(x) for x in o['liked'])
            hated = ', '.join(ron_str(x) for x in o['hated'])
            f.write('    (id:%d, name:%s, max_cost:%d, inflation:%d, '
                    'costs:(%d, %d, %d), liked:[%s], hated:[%s]),\n'
                    % (o['id'], ron_str(o['name']), o['max_cost'], o['inflation'],
                       o['costs'][0], o['costs'][1], o['costs'][2], liked, hated))
        f.write(']\n')
    print(f'owners.ron: {len(owners)} entries')

    # monster egos: race variants applied at spawn (re_info.txt)
    megos = parse_re_info(os.path.join(TOME_LIB, 're_info.txt'))
    with open(os.path.join(OUT_DIR, 'monster_egos.ron'), 'w') as f:
        f.write('[\n')
        for e in megos:
            def lst(key):
                return ', '.join(ron_str(x) for x in e[key])
            blows = ', '.join(
                '(method:%s, effect:%s, ddice:%s, dside:%s)'
                % (ron_str(b['method']), ron_str(b['effect']),
                   ron_str(b['ddice']), ron_str(b['dside']))
                for b in e['blows'])
            f.write('    (id:%d, name:%s, ch:%s, color:%d, before:%s, '
                    'speed:%s, hdice:%s, hside:%s, aaf:%s, ac:%s, sleep:%s, '
                    'level:%s, rarity:%d, weight:%s, mexp:%s, spell_freq:%d, '
                    'req_flags:[%s], req_chars:[%s], forbid_flags:[%s], '
                    'forbid_chars:[%s], add_flags:[%s], remove_flags:[%s], '
                    'add_spells:[%s], remove_spells:[%s], blows:[%s]),\n'
                    % (e['id'], ron_str(e['name']), ron_str(e['ch']), e['color'],
                       str(e['before']).lower(), ron_str(e['speed']),
                       ron_str(e['hdice']), ron_str(e['hside']), ron_str(e['aaf']),
                       ron_str(e['ac']), ron_str(e['sleep']), ron_str(e['level']),
                       e['rarity'], ron_str(e['weight']), ron_str(e['mexp']),
                       e['spell_freq'],
                       lst('req_flags'), lst('req_chars'), lst('forbid_flags'),
                       lst('forbid_chars'), lst('add_flags'), lst('remove_flags'),
                       lst('add_spells'), lst('remove_spells'), blows))
        f.write(']\n')
    print(f'monster_egos.ron: {len(megos)} entries')

    egos = parse_e_info(os.path.join(TOME_LIB, 'e_info.txt'))
    with open(os.path.join(OUT_DIR, 'egos.ron'), 'w') as f:
        f.write('[\n')
        for e in egos:
            tvals = ', '.join(str(t) for t in e['tvals'])
            svals = ', '.join('(%d, %d, %d)' % s for s in e['svals'])
            flags = ', '.join(ron_str(x) for x in sorted(e['flags']))
            pw = ', '.join(ron_str(x) for x in e['powers'])
            need = ', '.join(ron_str(x) for x in e['need_flags'])
            forbid = ', '.join(ron_str(x) for x in e['forbid_flags'])
            groups = ', '.join(
                '(chance:%d, flags:[%s], fego:[%s], oflags:[%s])' % (
                    g['rarity'],
                    ', '.join(ron_str(x) for x in g['flags']),
                    ', '.join(ron_str(x) for x in g['fego']),
                    ', '.join(ron_str(x) for x in g['oflags']))
                for g in e['groups'])
            f.write('    (id:%d, name:%s, prefix:%s, tvals:[%s], svals:[%s], '
                    'depth:%d, rarity:%d, rarity1:%d, cost:%d, to_h:%d, '
                    'to_d:%d, to_a:%d, pval:%d, cursed:%s, rating:%d, '
                    'activate:%s, need_flags:[%s], forbid_flags:[%s], '
                    'groups:[%s], flags:[%s], powers:[%s]),\n' % (
                        e['id'], ron_str(e['name']),
                        str(e['pos'] == 'B').lower(), tvals, svals, e['depth'],
                        e['rarity'], e['rarity1'], e['cost'], e['to_h'], e['to_d'],
                        e['to_a'], e['pval'], str(e['cursed']).lower(),
                        e['rating'], ron_str(e['activate']), need, forbid,
                        groups, flags, pw))
        f.write(']\n')
    print(f'egos.ron: {len(egos)} entries')

    arts = parse_a_info(os.path.join(TOME_LIB, 'a_info.txt'))
    with open(os.path.join(OUT_DIR, 'artifacts.ron'), 'w') as f:
        f.write('[\n')
        for a in arts:
            flags = ', '.join(ron_str(x) for x in sorted(a['flags']))
            pw = ', '.join(ron_str(x) for x in a['powers'])
            f.write('    (id:%d, name:%s, tval:%d, sval:%d, pval:%d, depth:%d, '
                    'rarity:%d, weight:%d, cost:%d, ac:%d, dice:%s, to_h:%d, '
                    'to_d:%d, to_a:%d, cursed:%s, insta_art:%s, activate:%s, '
                    'powers:[%s], flags:[%s]),\n' % (
                        a['id'], ron_str(a['name']), a['tval'], a['sval'],
                        a['pval'], a['depth'], a['rarity'], a['weight'],
                        a['cost'], a['ac'], ron_str(a['dice']), a['to_h'],
                        a['to_d'], a['to_a'], str(a['cursed']).lower(),
                        str(a['insta_art']).lower(), ron_str(a['activate']),
                        pw, flags))
        f.write(']\n')
    print(f'artifacts.ron: {len(arts)} entries')

    # randarts: parts (ra_info.txt), the power budget rows (G:) and the
    # name corpus (src/tables.cc artifact_names_list).
    ra_parts, ra_gen = parse_ra_info(os.path.join(TOME_LIB, 'ra_info.txt'))
    ra_names = parse_artifact_names(os.path.join(TOME_SRC, 'tables.cc'))
    junk_s, junk_f, junk_acts = parse_junkarts(TOME_LIB, TOME_SRC)
    with open(os.path.join(OUT_DIR, 'randarts.ron'), 'w') as f:
        f.write('(parts:[\n')
        for p in ra_parts:
            tvals = ', '.join('(tval:%d, min:%d, max:%d)' % t for t in p['tvals'])
            flags = ', '.join(ron_str(x) for x in p['flags'])
            aflags = ', '.join(ron_str(x) for x in p['aflags'])
            f.write('    (id:%d, tvals:[%s], level:%d, rarity:%d, mrarity:%d, '
                    'to_h:%d, to_d:%d, to_a:%d, pval:%d, value:%d, max:%d, '
                    'flags:[%s], aflags:[%s]),\n' % (
                        p['id'], tvals, p['level'], p['rarity'], p['mrarity'],
                        p['to_h'], p['to_d'], p['to_a'], p['pval'], p['value'],
                        p['max'], flags, aflags))
        f.write('], gen:[')
        f.write(', '.join('(chance:%d, dd:%d, ds:%d, plus:%d)' % g
                          for g in ra_gen))
        f.write('], names:%s' % ron_str('\n'.join(ra_names)))
        f.write(', junk_s:[%s]' % ', '.join(ron_str(x) for x in junk_s))
        f.write(', junk_f:[%s]' % ', '.join(ron_str(x) for x in junk_f))
        f.write(', acts:[')
        f.write(', '.join('(desc:%s, cost:%d, act:%s)'
                          % (ron_str(a[0]), a[1], ron_str(a[2]))
                          for a in junk_acts))
        f.write('])\n')
    print(f'randarts.ron: {len(ra_parts)} parts, {len(ra_names)} names, '
          f'{len(junk_s)} junkarts, {len(junk_acts)} activations')

    # artifact sets (set_info.txt)
    sets = parse_set_info(os.path.join(TOME_LIB, 'set_info.txt'))
    with open(os.path.join(OUT_DIR, 'sets.ron'), 'w') as f:
        f.write('[\n')
        for s in sets:
            members = []
            for m in s['members']:
                tiers = ', '.join('(pval:%d, flags:[%s])' % (
                    t['pval'], ', '.join(ron_str(x) for x in t['flags']))
                    for t in m['tiers'])
                members.append('(artifact:%d, tiers:[%s])' % (m['artifact'], tiers))
            f.write('    (id:%d, name:%s, desc:%s, members:[%s]),\n' % (
                s['id'], ron_str(s['name']), ron_str('\n'.join(s['desc'])),
                ', '.join(members)))
        f.write(']\n')
    print(f'sets.ron: {len(sets)} entries')

    # vault templates (v_info.txt)
    vaults = parse_v_info(os.path.join(TOME_LIB, 'v_info.txt'))
    with open(os.path.join(OUT_DIR, 'vaults.ron'), 'w') as f:
        f.write('[\n')
        for v in vaults:
            f.write('    (id:%d, typ:%d, rat:%d, hgt:%d, wid:%d, data:%s),\n' % (
                v['id'], v['typ'], v['rat'], v['hgt'], v['wid'],
                ron_str(v['data'])))
        f.write(']\n')
    print(f'vaults.ron: {len(vaults)} entries')

    schools = parse_s_info(os.path.join(TOME_LIB, 's_info.txt'))
    # The Bard's Music school (s_info keeps it as a skill, not a school).
    schools.append({'id': 100, 'name': 'Music'})
    with open(os.path.join(OUT_DIR, 'schools.ron'), 'w') as f:
        f.write('[\n')
        for s in schools:
            f.write('    (id:%d, name:%s, cast:%s),\n'
                    % (s['id'], ron_str(s['name']),
                       str(bool(s.get('cast'))).lower()))
        f.write(']\n')
    print(f'schools.ron: {len(schools)} entries')

    # spell_type_describe lines (spells5.cc registrations): the original
    # description text, carried as the spell row's `desc` field.
    spell_descs = {}
    spells5_src = open(os.path.join(TOME_SRC, 'spells5.cc'),
                       encoding='latin-1').read()
    for m in re.finditer(r'spell_new\(&[A-Z_0-9]+, "([^"]+)"\)', spells5_src):
        name = m.group(1)
        j = spells5_src.find('spell_type_init', m.end())
        block = spells5_src[m.end():j if j > 0 else m.end() + 2000]
        lines = re.findall(r'spell_type_describe\(spell, "([^"]*)"\)', block)
        if lines:
            spell_descs[name] = lines

    # spell_type_init_* registrations: random_type + school. The port's
    # curated SPELLS table also carries helper rows (Cure Light Wounds,
    # Heal, Identify, ...) and cross-school duplicates that the original
    # never registered, so get_random_spell must not roll them: only rows
    # whose name/school match a registration get a non-zero random_type.
    school_by_name = {s['name'].upper(): s['id'] for s in schools}
    spell_reg = {}
    for m in re.finditer(r'spell_new\(&[A-Z_0-9]+, "([^"]+)"\)', spells5_src):
        name = m.group(1)
        j = spells5_src.find('spell_new', m.end())
        block = spells5_src[m.end():j if j > 0 else m.end() + 4000]
        init = re.search(
            r'spell_type_init_(mage|priest|music(?:_lasting)?|device|demonology'
            r'|geomancy)\(([^;]*?)\);', block, re.S)
        if not init:
            continue
        kind = init.group(1)
        args = [a.strip() for a in init.group(2).split(',')]
        if kind == 'mage':
            typ = 'NO_RANDOM' if any('NO_RANDOM' in a for a in args) else 'MAGIC'
            school = None
            if len(args) > 2:
                school = school_by_name.get(
                    args[2].replace('SCHOOL_', '').upper())
            spell_reg[name] = (typ, school)
        elif kind == 'priest':
            spell_reg[name] = ('SPIRIT', None)
        elif kind.startswith('music'):
            spell_reg[name] = ('MUSIC', None)
        else:
            spell_reg[name] = ('NO_RANDOM', None)

    # Port book names for registered C++ spells (the original names are
    # the keys of spell_reg).
    spell_alias = {
        'Teleport': 'Teleportation',
        'Word of Recall': 'Recall',
    }
    random_value = {
        'MAGIC': 15,       # SKILL_MAGIC
        'SPIRIT': 28,      # SKILL_SPIRITUALITY
        'MUSIC': 9,        # SKILL_MUSIC
    }

    def row_random_type(name, school):
        reg = spell_reg.get(name)
        if reg is not None:
            typ, reg_school = reg
            if typ == 'MAGIC' and reg_school is not None and school != reg_school:
                # A duplicate row of a registered name in another school
                # (e.g. the Prayer "Remove Curses" helper) is not random.
                return 0
            return random_value.get(typ, 0)
        alias = spell_alias.get(name)
        if alias is not None and spell_reg.get(alias, ('', None))[0] == 'MAGIC':
            return random_value['MAGIC']
        return 0

    school_ids = {s['id'] for s in schools} | {0}
    seen_spells = set()
    with open(os.path.join(OUT_DIR, 'spells.ron'), 'w') as f:
        f.write('[\n')
        for row in SPELLS:
            # 9-tuples carry the full C++ mana range (min, max); older
            # 8-tuples are flat-cost rows (max = min).
            if len(row) == 9:
                (name, school, level, mana, fail, mana_max,
                 kind, arg, targeted) = row
            else:
                (name, school, level, mana, fail, kind, arg, targeted) = row
                mana_max = mana
            if school not in school_ids:
                raise SystemExit(f'spell {name}: unknown school {school}')
            if (name, school) in seen_spells:
                continue  # staff/rod rows share names with castable rows
            seen_spells.add((name, school))
            elem = SPELL_ELEM.get(name, '')
            god = SPELL_GOD.get(name, '')
            charges, allocs = DEVICE_SPELLS.get(name, ('', []))
            alloc_s = '[' + ','.join(
                '(%d,%d,%d,%d,%d,%d)' % a for a in allocs) + ']'
            desc_s = '[' + ','.join(
                ron_str(x)
                for x in spell_descs.get(spell_alias.get(name, name), [])
            ) + ']'
            f.write('    (name:%s, school:%d, level:%d, mana:%d, mana_max:%d, '
                    'fail:%d, kind:%s, arg:%s, targeted:%s, elem:%s, god:%s, '
                    'random_type:%d, charges:%s, alloc:%s, desc:%s),\n' % (
                        ron_str(name), school, level, mana, mana_max, fail,
                        ron_str(kind), ron_str(arg),
                        str(targeted).lower(), ron_str(elem), ron_str(god),
                        row_random_type(name, school),
                        ron_str(charges), alloc_s, desc_s))
        f.write(']\n')
    print(f'spells.ron: {len(SPELLS)} entries')

    monster_ids = {m['id'] for m in monsters}
    bree = build_bree(terrains)
    known_qids = {QUEST_THIEVES, QUEST_TROLL, QUEST_WIGHT, QUEST_SHROOM,
                  22, 10, 24, 23, 25, 28, 16, 27, 14, 15, 11, 13}
    for _, _, qid in bree['entrances']:
        assert qid in known_qids, qid
    with open(os.path.join(OUT_DIR, 'bree.ron'), 'w') as f:
        f.write('(cells:[\n')
        for t, l in bree['cells']:
            f.write('    (t:%d, l:%s),\n' % (t, str(l).lower()))
        f.write('], shops:[')
        f.write(', '.join('(x:%d, y:%d, store:%d)' % s for s in bree['shops']))
        f.write('], entrances:[')
        f.write(', '.join('(x:%d, y:%d, quest:%d)' % e for e in bree['entrances']))
        f.write('], buildings:[')
        f.write(', '.join('(x:%d, y:%d, special:%d)' % b for b in bree['buildings']))
        f.write('], quest_cells:[')
        f.write(', '.join('(x:%d, y:%d, quest:%d, night_only:%s)'
                          % (c[0], c[1], c[2], str(c[3]).lower())
                          for c in bree['quest_cells']))
        f.write('], start:(%d, %d))\n' % bree['start'])
    print('bree.ron: %d shops, %d entrances, %d buildings, start %s'
          % (len(bree['shops']), len(bree['entrances']),
             len(bree['buildings']), bree['start']))

    keep_ids = {o['id'] for o in items}

    # --- wilderness: wf_info.txt + w_info.txt ---------------------------
    wfs = parse_wf_info(os.path.join(TOME_LIB, 'wf_info.txt'))
    wf_chars = {r['ch']: r['id'] for r in wfs.values() if r['ch'] != '?'}
    with open(os.path.join(OUT_DIR, 'wf.ron'), 'w') as f:
        f.write('[\n')
        for wid in sorted(wfs):
            w = wfs[wid]
            terrain = ', '.join(str(t) for t in w['terrain'])
            f.write('    (id:%d, name:%s, text:%s, level:%d, entrance:%d, '
                    'road:%d, feat:%d, terrain_idx:%d, ch:%s, terrain:[%s]),\n'
                    % (w['id'], ron_str(w['name']), ron_str(w['text']),
                       w['level'], w['entrance'], w['road'], w['feat'],
                       w['terrain_idx'], ron_char(w['ch']), terrain))
        f.write(']\n')
    print(f'wf.ron: {len(wfs)} entries')

    world = parse_w_info(os.path.join(TOME_LIB, 'w_info.txt'), wf_chars)
    with open(os.path.join(OUT_DIR, 'world.ron'), 'w') as f:
        f.write('(w:%d, h:%d, rows:[' % (world['w'], world['h']))
        for row in world['rows']:
            f.write('\n    [%s],' % ', '.join(str(v) for v in row))
        f.write('\n], patches:[')
        for cond, y, row in world['patches']:
            f.write('(cond:%s, y:%d, row:[%s]), '
                    % (ron_str(cond), y, ', '.join(str(v) for v in row)))
        f.write('], entrances:[')
        f.write(', '.join('(d:%d, x:%d, y:%d)' % e
                          for e in world['entrances']))
        f.write('], start:(%d, %d))\n' % world['start'])
    print('world.ron: %dx%d, %d entrances, %d patches, start %s'
          % (world['w'], world['h'], len(world['entrances']),
             len(world['patches']), world['start']))

    # --- dungeons: d_info.txt -------------------------------------------
    # (parsed earlier, before monsters.ron, for init_guardians marking)
    with open(os.path.join(OUT_DIR, 'dungeons.ron'), 'w') as f:
        f.write('[\n')
        for d in dungeons:
            floors = ', '.join('(feat:%d, top:%d, bottom:%d)'
                               % (d['floor_types'][i], d['floor_pcts'][i][0],
                                  d['floor_pcts'][i][1]) for i in range(3))
            fills = ', '.join('(feat:%d, top:%d, bottom:%d)'
                              % (d['fill_types'][i], d['fill_pcts'][i][0],
                                 d['fill_pcts'][i][1]) for i in range(3))
            flags = ', '.join(ron_str(x) for x in d['flags'])
            effects = ', '.join('(dd:%d, ds:%d, freq:%d, typ:%s)'
                                % (e[0], e[1], e[2], ron_str(e[3]))
                                for e in d['effects'])
            rules = []
            for r in d['rules']:
                rules.append('(pct:%d, mode:%d, chars:[%s], mflags:[%s], '
                             'mspells:[%s])'
                             % (r['pct'], r['mode'],
                                ', '.join(ron_str(c) for c in r['chars']),
                                ', '.join(ron_str(x) for x in r['mflags']),
                                ', '.join(ron_str(x) for x in r['mspells'])))
            specials = []
            for sp in d['specials']:
                specials.append('(depth:%d, name:%s, desc:%s, map:%s, '
                                'flags:[%s], branch:%s, save_extension:%s)'
                                % (sp['depth'], ron_str(sp['name']),
                                   ron_str(sp['desc']), ron_str(sp['map']),
                                   ', '.join(ron_str(x) for x in sp['flags']),
                                   'Some(%d)' % sp['branch']
                                   if sp['branch'] is not None else 'None',
                                   'Some(%s)' % ron_str(sp['save_extension'])
                                   if sp['save_extension'] else 'None'))
            f.write('    (id:%d, name:%s, short:%s, text:%s, mindepth:%d, '
                    'maxdepth:%d, min_plev:%d, min_alloc:%d, max_chance:%d, '
                    'floors:[%s], fills:[%s], outer_wall:%d, inner_wall:%d, '
                    'theme:(%d, %d, %d, %d), flags:[%s], effects:[%s], '
                    'rules:[%s], rule_percents:[%s], guardian:%s, '
                    'final_artifact:%s, final_object:%s, specials:[%s], '
                    'generator:%s, ix:%d, iy:%d, ox:%d, oy:%d, '
                    'branch_parent:%s),\n'
                    % (d['id'], ron_str(d['name']), ron_str(d['short']),
                       ron_str(d['text']), d['mindepth'], d['maxdepth'],
                       d['min_plev'], d['min_alloc'], d['max_chance'],
                       floors, fills, d['outer_wall'], d['inner_wall'],
                       d['theme'][0], d['theme'][1], d['theme'][2],
                       d['theme'][3], flags, effects, ', '.join(rules),
                       ', '.join(str(x) for x in d['rule_percents']),
                       'Some(%d)' % d['guardian'] if d['guardian'] else 'None',
                       'Some(%d)' % d['final_artifact']
                       if d['final_artifact'] else 'None',
                       'Some(%d)' % d['final_object']
                       if d['final_object'] else 'None',
                       ', '.join(specials), ron_str(d['generator']),
                       d['ix'], d['iy'], d['ox'], d['oy'],
                       'Some((%d, %d))' % d['branch_parent']
                       if d['branch_parent'] else 'None'))
        f.write(']\n')
    print(f'dungeons.ron: {len(dungeons)} entries')

    # --- towns: t_info.txt selection, t_*.txt pref maps ------------------
    tpref_path = os.path.join(TOME_LIB, 't_pref.txt')
    base_chars, _, _ = parse_pref(tpref_path, PREF_VARS)
    # t_pref.txt has F: lines only; parse_pref returns them as char defs.
    towns = []
    for tid, tname, fname, dfname in [
            (1, 'Bree', 't_bree.txt', 't_d_bree.txt'),
            (2, 'Gondolin', 't_gondol.txt', 't_d_gond.txt'),
            (3, 'Minas Anor', 't_minas.txt', 't_d_mina.txt'),
            (4, 'Lothlorien', 't_lorien.txt', 't_d_lori.txt'),
            (5, 'Khazad-dum', 't_khazad.txt', 't_d_khaz.txt')]:
        for destroyed, path in ((False, fname), (True, dfname)):
            towns.append(build_town(tid, tname, os.path.join(TOME_LIB, path),
                                    destroyed, base_chars, terrains))
    with open(os.path.join(OUT_DIR, 'towns.ron'), 'w') as f:
        f.write('[\n')
        for t in towns:
            f.write('    (id:%d, name:%s, destroyed:%s, mflag:%s, rows:[\n'
                    % (t['id'], ron_str(t['name']),
                       str(t['destroyed']).lower(), ron_str(t['mflag'])))
            for row in t['rows']:
                f.write('        %s,\n' % ron_str(row))
            f.write('    ], chars:[')
            f.write(', '.join('(ch:%s, terrain:%d, mark:%s, glow:%s, room:%s, '
                              'free:%s, monster:%d, object:%d, special:%d)'
                              % (ron_char(ch), d['terrain'],
                                 str(bool(d['cave'] & 1)).lower(),
                                 str(bool(d['cave'] & 2)).lower(),
                                 str(bool(d['cave'] & 8)).lower(),
                                 str(bool(d['cave'] & 0x800)).lower(),
                                 d['monster'], d['object'], d['special'])
                              for ch, d in t['chars']))
            f.write('], overrides:[')
            f.write(', '.join('(cond:%s, ch:%s, terrain:%d, mark:%s, glow:%s, '
                              'room:%s, free:%s, monster:%d, object:%d, '
                              'special:%d)'
                              % (ron_str(cond), ron_char(ch), d['terrain'],
                                 str(bool(d['cave'] & 1)).lower(),
                                 str(bool(d['cave'] & 2)).lower(),
                                 str(bool(d['cave'] & 8)).lower(),
                                 str(bool(d['cave'] & 0x800)).lower(),
                                 d['monster'], d['object'], d['special'])
                              for cond, ch, d in t['overrides']))
            f.write('], starts:[')
            f.write(', '.join('(quest:%d, x:%d, y:%d)' % s
                              for s in t['starts']))
            f.write('], default_start:%s),\n'
                    % ('Some((%d, %d))' % t['default_start']
                       if t['default_start'] else 'None'))
        f.write(']\n')
    print('towns.ron: %d variants' % len(towns))

    qmaps = []
    for qid, fname in (((QUEST_THIEVES, 'thieves.map'),
                        (QUEST_TROLL, 'trolls.map'),
                        (QUEST_WIGHT, 'wights.map')) + tuple(EXTRA_QUEST_MAPS)):
        qm = build_quest_map(os.path.join(TOME_LIB, fname), qid, keep_ids)
        before = len(qm['monsters'])
        # Some maps reference r_info ids that no longer exist in this
        # version of the data (e.g. maeglin.map's 251 "Snagga sapper");
        # drop those placements, keeping everything else.
        qm['monsters'] = [m for m in qm['monsters'] if m[0] in monster_ids]
        if len(qm['monsters']) != before:
            print(f'  {fname}: dropped {before - len(qm["monsters"])} '
                  f'unknown monster placements')
        qmaps.append(qm)
    with open(os.path.join(OUT_DIR, 'questmaps.ron'), 'w') as f:
        f.write('[\n')
        for qm in qmaps:
            f.write('    (quest:%d, w:%d, h:%d, cells:[\n'
                    % (qm['quest'], qm['w'], qm['h']))
            for t, l in qm['cells']:
                f.write('        (t:%d, l:%s),\n' % (t, str(l).lower()))
            f.write('    ], monsters:[')
            f.write(', '.join('(def:%d, x:%d, y:%d, quest:%s)'
                              % (d, x, y, str(q).lower())
                              for d, x, y, q in qm['monsters']))
            f.write('], objects:[')
            f.write(', '.join('(def:%d, x:%d, y:%d)' % o for o in qm['objects']))
            f.write('], random_monsters:[')
            f.write(', '.join('(level:%d, x:%d, y:%d)' % r
                              for r in qm['random_monsters']))
            f.write('], random_objects:[')
            f.write(', '.join('(level:%d, x:%d, y:%d)' % r
                              for r in qm['random_objects']))
            f.write('], mimics:[')
            f.write(', '.join('(x:%d, y:%d, t:%d)' % m for m in qm['mimics']))
            f.write('], start:(%d, %d), exit:(%d, %d), marker:(%d, %d)),\n'
                    % (qm['start'] + qm['exit'] + qm['marker']))
        f.write(']\n')
    print('questmaps.ron: %d quest maps' % len(qmaps))

    # The d_info '@:' special levels (fixed set-piece dungeon depths),
    # keyed by their map file name so dungeons.ron can reference them.
    map_names = []
    for d in dungeons:
        for sp in d['specials']:
            if sp['map'] and sp['map'] not in map_names:
                map_names.append(sp['map'])
    with open(os.path.join(OUT_DIR, 'speclevels.ron'), 'w') as f:
        f.write('[\n')
        for fname in map_names:
            qm = build_quest_map(os.path.join(TOME_LIB, fname), 0, keep_ids)
            qm['monsters'] = [m for m in qm['monsters'] if m[0] in monster_ids]
            f.write('    (name:%s, w:%d, h:%d, cells:[\n'
                    % (ron_str(fname), qm['w'], qm['h']))
            for t, l in qm['cells']:
                f.write('        (t:%d, l:%s),\n' % (t, str(l).lower()))
            f.write('    ], monsters:[')
            f.write(', '.join('(def:%d, x:%d, y:%d, quest:%s)'
                              % (d, x, y, str(q).lower())
                              for d, x, y, q in qm['monsters']))
            f.write('], objects:[')
            f.write(', '.join('(def:%d, x:%d, y:%d)' % o for o in qm['objects']))
            f.write('], artifacts:[')
            f.write(', '.join('(id:%d, x:%d, y:%d)' % a
                              for a in qm['artifacts']))
            f.write('], random_monsters:[')
            f.write(', '.join('(level:%d, x:%d, y:%d)' % r
                              for r in qm['random_monsters']))
            f.write('], random_objects:[')
            f.write(', '.join('(level:%d, x:%d, y:%d)' % r
                              for r in qm['random_objects']))
            f.write('], mimics:[')
            f.write(', '.join('(x:%d, y:%d, t:%d)' % m for m in qm['mimics']))
            f.write('], start:(%d, %d)),\n' % qm['start'])
        f.write(']\n')
    print('speclevels.ron: %d special levels' % len(map_names))


if __name__ == '__main__':
    main()
