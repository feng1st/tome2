//! Mimicry (src/mimic.cc, src/cmd7.cc do_cmd_mimic): cloaks of mimicry
//! and Morphic Oil let the player take a temporary shape.  The shape's
//! stat changes are applied to PlayerState like equipment stat bonuses;
//! the remaining effects are folded into EquipTotals.

use rand::Rng;

use crate::data::GameData;
use crate::game::{MessageLog, PlayerState, CHA, CON, DEX, INT, STR, WIS};
use crate::item::EquipTotals;

/// MAX_MIMIC_POWERS / MIMIC_FORMS_MAX (defines.hpp).
pub const MIMIC_FORMS_MAX: usize = 14;

/// The five magic powers (tables.cc mimic_powers):
/// (minimum skill level, mana cost, base failure, name).
pub const MIMIC_POWERS: [(i32, i32, i32, &str); 5] = [
    (1, 2, 0, "Mimic"),
    (10, 6, 20, "Invisibility"),
    (25, 20, 25, "Legs Mimicry"),
    (30, 40, 30, "Wall Mimicry"),
    (35, 100, 40, "Arms Mimicry"),
];

/// CLASS_* bits of mimic_extra (defines.hpp).
pub const CLASS_LEGS: u32 = 0x0020;
pub const CLASS_ARMS: u32 = 0x0040;
pub const CLASS_WALL: u32 = 0x0080;

/// BODY_* indexes (defines.hpp) into the six p_info/r_info body-part slots.
pub const BODY_WEAPON: usize = 0;
pub const BODY_TORSO: usize = 1;
pub const BODY_ARMS: usize = 2;
pub const BODY_FINGER: usize = 3;
pub const BODY_HEAD: usize = 4;
pub const BODY_LEGS: usize = 5;
/// Per-part caps (tables.cc max_body_part).
pub const MAX_BODY_PART: [i32; 6] = [3, 1, 3, 6, 2, 2];

pub const MIMIC_ABOMINATION: u32 = 0;
pub const MIMIC_MOUSE: u32 = 1;
pub const MIMIC_WOLF: u32 = 4;
pub const MIMIC_SPIDER: u32 = 5;
pub const MIMIC_ENT: u32 = 6;
pub const MIMIC_BEAR: u32 = 10;
pub const MIMIC_BALROG: u32 = 11;

/// One mimicry shape (src/mimic.cc mimic_forms[]).
pub struct MimicFormDef {
    pub name: &'static str,
    pub obj_name: &'static str,
    /// Available in the ToME module (THEME-only entries are disabled).
    pub enabled: bool,
    /// Forms only reachable through special means (cloaks, powers).
    pub limit: bool,
    pub level: u32,
    pub rarity: u32,
    /// Random duration from Morphic Oil (closed interval).
    pub duration: (i32, i32),
}

pub const MIMIC_FORMS: [MimicFormDef; MIMIC_FORMS_MAX] = [
    MimicFormDef {
        name: "Abomination",
        obj_name: "Abominable Cloak",
        enabled: true,
        limit: false,
        level: 1,
        rarity: 101,
        duration: (20, 100),
    },
    MimicFormDef {
        name: "Mouse",
        obj_name: "Mouse Fur",
        enabled: true,
        limit: false,
        level: 1,
        rarity: 10,
        duration: (20, 40),
    },
    MimicFormDef {
        name: "Eagle",
        obj_name: "Feathers Cloak",
        enabled: true,
        limit: false,
        level: 10,
        rarity: 30,
        duration: (10, 50),
    },
    MimicFormDef {
        name: "Eagle",
        obj_name: "Feathered Cloak",
        enabled: false,
        limit: false,
        level: 10,
        rarity: 30,
        duration: (10, 50),
    },
    MimicFormDef {
        name: "Wolf",
        obj_name: "Wolf Pelt",
        enabled: true,
        limit: false,
        level: 20,
        rarity: 40,
        duration: (10, 50),
    },
    MimicFormDef {
        name: "Spider",
        obj_name: "Spider Web",
        enabled: true,
        limit: false,
        level: 25,
        rarity: 50,
        duration: (10, 50),
    },
    MimicFormDef {
        name: "Elder Ent",
        obj_name: "Entish Bark",
        enabled: true,
        limit: true,
        level: 40,
        rarity: 60,
        duration: (10, 30),
    },
    MimicFormDef {
        name: "Vapour",
        obj_name: "Cloak of Mist",
        enabled: true,
        limit: false,
        level: 15,
        rarity: 10,
        duration: (10, 40),
    },
    MimicFormDef {
        name: "Serpent",
        obj_name: "Snakeskin Cloak",
        enabled: true,
        limit: false,
        level: 30,
        rarity: 25,
        duration: (15, 20),
    },
    MimicFormDef {
        name: "Mumak",
        obj_name: "Mumak Hide",
        enabled: true,
        limit: false,
        level: 40,
        rarity: 40,
        duration: (15, 20),
    },
    MimicFormDef {
        name: "Bear",
        obj_name: "",
        enabled: true,
        limit: true,
        level: 1,
        rarity: 101,
        duration: (50, 200),
    },
    MimicFormDef {
        name: "Balrog",
        obj_name: "",
        enabled: true,
        limit: true,
        level: 1,
        rarity: 101,
        duration: (30, 70),
    },
    MimicFormDef {
        name: "Maia",
        obj_name: "",
        enabled: true,
        limit: true,
        level: 1,
        rarity: 101,
        duration: (30, 70),
    },
    MimicFormDef {
        name: "Fire Elem.",
        obj_name: "",
        enabled: true,
        limit: true,
        level: 1,
        rarity: 101,
        duration: (10, 10),
    },
];

/// Form record by index (mimic.cc get_mimic_form).  The original asserts
/// the bounds; the port falls back to Abomination (index 0).
pub fn form(mf_idx: u32) -> &'static MimicFormDef {
    MIMIC_FORMS.get(mf_idx as usize).unwrap_or(&MIMIC_FORMS[0])
}

/// Name of a form (mimic.cc get_mimic_name).
pub fn form_name(form: u32) -> &'static str {
    self::form(form).name
}

/// Object name of a form (mimic.cc get_mimic_object_name).
pub fn form_obj_name(form: u32) -> &'static str {
    self::form(form).obj_name
}

/// Level of a form (mimic.cc get_mimic_level).
pub fn form_level(mf_idx: u32) -> u32 {
    self::form(mf_idx).level
}

/// Find a form by exact name, first enabled entry wins (mimic.cc
/// resolve_mimic_name).  Returns -1 when no enabled form matches.
pub fn resolve_name(name: &str) -> i32 {
    for (i, f) in MIMIC_FORMS.iter().enumerate() {
        if f.enabled && f.name == name {
            return i as i32;
        }
    }
    -1
}

/// A random shape for a cloak (`limit=true`) or Morphic Oil (`false`);
/// falls back to Abomination (mimic.cc find_random_mimic_shape).
pub fn find_random_mimic_shape(level: i32, limit: bool, rng: &mut impl Rng) -> u32 {
    for _ in 0..1000 {
        let i = rng.gen_range(0..MIMIC_FORMS_MAX) as u32;
        let f = &MIMIC_FORMS[i as usize];
        if f.enabled && (limit || !f.limit) {
            if rng.gen_range(0..(f.level as i32 * 3)) < level
                && f.rarity < 100
                && rng.gen_range(0..100) < 100 - f.rarity as i32
            {
                return i;
            }
        }
    }
    0
}

/// Random duration for a Morphic Oil shape (mimic.cc
/// get_mimic_random_duration; rand_range is a closed interval).
pub fn random_duration(form: u32, rng: &mut impl Rng) -> i32 {
    let (min, max) = MIMIC_FORMS
        .get(form as usize)
        .map(|f| f.duration)
        .unwrap_or((20, 100));
    rng.gen_range(min..=max)
}

/// Stat changes of a shape (mimic.cc *_calc stat_add lines).
pub fn form_stat_deltas(form: u32, ml: i32) -> [i32; 6] {
    let mut d = [0i32; 6];
    match form {
        MIMIC_ABOMINATION => d = [-10, -10, -10, -10, -10, -10],
        MIMIC_MOUSE => {
            d[STR] += -5;
            d[DEX] += 3;
            d[CON] += 1;
        }
        2 | 3 => {
            d[STR] += -3;
            d[DEX] += 2 + ml / 15;
            d[CON] += 4 + ml / 20;
            d[INT] += -1;
            d[WIS] += 1;
            d[CHA] += -1;
        }
        4 => {
            d[STR] += 2 + ml / 20;
            d[DEX] += 3 + ml / 20;
            d[INT] += -3;
            d[CHA] += -2;
        }
        5 => {
            d[STR] += -4;
            d[DEX] += 1 + ml / 8;
            d[INT] += 1 + ml / 5;
            d[WIS] += 1 + ml / 5;
            d[CON] += -5;
            d[CHA] += -10;
        }
        6 => {
            d[STR] += ml / 5;
            d[INT] += -(ml / 7);
            d[WIS] += -(ml / 7);
            d[DEX] += -4;
            d[CON] += ml / 5;
            d[CHA] += -7;
        }
        7 => {
            d[STR] += -4;
            d[DEX] += 5;
            d[CON] += -4;
            d[CHA] += -10;
        }
        8 => {
            d[STR] += ml / 8;
            d[INT] += -6;
            d[WIS] += -6;
            d[DEX] += -4;
            d[CON] += ml / 7;
            d[CHA] += -6;
        }
        9 => {
            d[STR] += ml / 4;
            d[INT] += -8;
            d[WIS] += -4;
            d[DEX] += -5;
            d[CON] += ml / 3;
            d[CHA] += -10;
        }
        10 => {
            d[STR] += ml / 11;
            d[INT] += ml / 11;
            d[WIS] += ml / 11;
            d[DEX] += -1;
            d[CON] += ml / 11;
            d[CHA] += -10;
        }
        11 => {
            d[STR] += 5 + ml / 5;
            d[INT] += ml / 10;
            d[WIS] += -(5 + ml / 10);
            d[DEX] += ml / 10;
            d[CON] += 5 + ml / 5;
            d[CHA] += -(5 + ml / 10);
        }
        12 => {
            for x in d.iter_mut() {
                *x += 5 + ml / 5;
            }
        }
        13 => {
            d[STR] += 5 + ml / 5;
            d[DEX] += 5 + ml / 5;
            d[WIS] += -5 - ml / 5;
        }
        _ => {}
    }
    d
}

fn res(t: &mut EquipTotals, e: &str) {
    t.resists.insert(e.to_string());
}

fn imm(t: &mut EquipTotals, e: &str) {
    t.immunities.insert(e.to_string());
}

/// Non-stat effects of the active shape, folded into the equipment
/// totals (mimic.cc *_calc; `mimic_level` is the shape strength).
pub fn apply_mimic_totals(ps: &PlayerState, t: &mut EquipTotals) {
    let Some(form) = ps.mimic_form else { return };
    let ml = ps.mimic_level;
    match form {
        MIMIC_ABOMINATION => {
            t.speed -= 10;
            t.aggravate = true;
        }
        MIMIC_MOUSE => {
            t.speed += 5 + ml / 7;
            t.to_h += 10 + ml / 5;
            // The original divides p_ptr->to_d in mouse_calc (mimic.cc:67),
            // but calc_bonuses zeroes to_d first (xtra1.cc:2726) and folds
            // gear/stat bonuses in afterwards, so the division never hits
            // a nonzero value; the port matches that (no to_d penalty).
            t.stealth += 10 + ml / 5;
        }
        2 | 3 => {
            t.feather = true;
            t.speed += 2 + ml / 6;
            if ml >= 20 {
                t.fly = true;
                t.see_invis = true;
            }
            if ml >= 25 {
                t.free_act = true;
            }
            if ml >= 30 {
                res(t, "ELEC");
            }
            if ml >= 35 {
                t.sh_elec = true;
            }
        }
        4 => {
            t.speed += 10 + ml / 5;
            t.free_act = true;
            res(t, "FEAR");
            if ml >= 10 {
                res(t, "COLD");
            }
            if ml >= 15 {
                t.see_invis = true;
            }
            if ml >= 30 {
                res(t, "DARK");
            }
            if ml >= 35 {
                res(t, "CONF");
            }
        }
        5 => {
            t.speed += 5;
            res(t, "POIS");
            res(t, "FEAR");
            res(t, "DARK");
            if ml >= 40 {
                t.climb = true;
            }
        }
        6 => {
            t.speed -= 5 + ml / 10;
            t.ac += 10 + ml;
            res(t, "POIS");
            res(t, "COLD");
            t.free_act = true;
            t.regen = true;
            t.see_invis = true;
            t.sens_fire = true;
        }
        7 => {
            t.speed += 5;
            t.ac += 40 + ml;
            t.to_h -= 40;
            t.stealth += 10 + ml / 5;
            res(t, "POIS");
            res(t, "SHARDS");
            imm(t, "COLD");
            t.free_act = true;
            t.regen = true;
            t.see_invis = true;
            t.sens_fire = true;
            t.feather = true;
        }
        8 => {
            t.speed += 10 + ml / 6;
            t.ac += 3 + ml / 8;
            res(t, "POIS");
            if ml >= 25 {
                t.free_act = true;
            }
        }
        9 => {
            t.speed -= 5 + ml / 10;
            t.ac += 10 + ml / 6;
            t.to_d += 5 + (ml * 2) / 3;
            if ml >= 10 {
                res(t, "FEAR");
            }
            if ml >= 25 {
                res(t, "CONF");
            }
            if ml >= 30 {
                t.free_act = true;
            }
            if ml >= 35 {
                res(t, "NEXUS");
            }
        }
        10 => {
            t.speed += -5 + ml / 5;
            t.ac += 5 + (ml * 2) / 3;
            if ml >= 10 {
                t.free_act = true;
            }
            if ml >= 20 {
                t.regen = true;
            }
            if ml >= 30 {
                res(t, "CONF");
            }
            if ml >= 35 {
                res(t, "NEXUS");
            }
        }
        11 => {
            imm(t, "ACID");
            imm(t, "FIRE");
            imm(t, "ELEC");
            res(t, "DARK");
            res(t, "CHAOS");
            res(t, "POIS");
            t.hold_life = true;
            t.feather = true;
            t.regen = true;
            t.sh_fire = true;
            t.lite += 1;
            t.blows += 1;
        }
        12 => {
            imm(t, "FIRE");
            imm(t, "ELEC");
            imm(t, "ACID");
            imm(t, "COLD");
            res(t, "POIS");
            res(t, "LITE");
            res(t, "DARK");
            res(t, "CHAOS");
            t.hold_life = true;
            t.feather = true;
            t.regen = true;
            t.blows += 2;
        }
        13 => {
            imm(t, "FIRE");
            res(t, "POIS");
            t.sh_fire = true;
            t.lite += 1;
        }
        _ => {}
    }
}

/// Undo the active shape's stat changes (before changing or clearing it).
fn remove_stats(ps: &mut PlayerState) {
    if let Some(form) = ps.mimic_form {
        let d = form_stat_deltas(form, ps.mimic_level);
        for i in 0..6 {
            ps.stats[i] -= d[i];
        }
    }
}

/// Apply the active shape's stat changes.
fn apply_stats(ps: &mut PlayerState) {
    if let Some(form) = ps.mimic_form {
        let d = form_stat_deltas(form, ps.mimic_level);
        for i in 0..6 {
            ps.stats[i] += d[i];
        }
    }
}

/// Change shape (xtra2.cc set_mimic): a running shape is never replaced,
/// only refreshed and strengthened.  `turns <= 0` ends it.  Returns true
/// when the transformation state visibly changed.
pub fn set_mimic(
    gd: &GameData,
    ps: &mut PlayerState,
    turns: i32,
    form: u32,
    level: i32,
    log: &mut MessageLog,
) -> bool {
    let turns = turns.clamp(0, 10000);
    let active = ps.mimic_turns > 0;
    remove_stats(ps);
    let mut notice = false;
    if turns > 0 {
        if !active {
            log.add("You feel your body change.");
            ps.mimic_form = Some(form);
            notice = true;
        }
    } else if active {
        log.add("You are no longer transformed.");
        if ps.mimic_form == Some(MIMIC_BEAR) {
            ps.skill_hidden.insert(crate::skill::SK_BEAR);
            crate::skill::select_default_melee(ps, gd);
        }
        ps.mimic_form = None;
        notice = true;
    }
    ps.mimic_turns = turns;
    ps.mimic_level = level;
    if turns > 0 {
        apply_stats(ps);
    }
    if notice {
        crate::game::calc_sanity(ps);
    }
    notice
}

/// Do the two mimic checks when using a magic power (cmd7.cc fails/backfire
/// common code).  Returns the failure chance.
pub fn failure_chance(ps: &PlayerState, fail: i32, min_lev: i32, cost: i32, dex_stat: i32) -> i32 {
    let plev = ps.skill(crate::skill::SK_MIMICRY);
    let mut chance = fail - 3 * (plev - min_lev) - 3 * (adj_mag_stat(dex_stat) - 1);
    if cost > ps.mana {
        chance += 5 * (cost - ps.mana);
    }
    let minfail = adj_mag_fail(dex_stat);
    if chance < minfail {
        chance = minfail;
    }
    if ps.stun > 50 {
        chance += 25;
    } else if ps.stun > 0 {
        chance += 15;
    }
    chance.min(95)
}

/// INT/WIS/DEX casting stat table (tables.cc adj_mag_stat).
pub fn adj_mag_stat(stat: i32) -> i32 {
    const TABLE: [i32; 38] = [
        0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12,
        13, 14, 15, 16, 17, 18, 19, 20, 21,
    ];
    TABLE[crate::game::stat_index(stat)]
}

/// Minimum casting failure by stat (tables.cc adj_mag_fail).
pub fn adj_mag_fail(stat: i32) -> i32 {
    const TABLE: [i32; 38] = [
        99, 99, 99, 99, 99, 50, 30, 20, 15, 12, 11, 10, 9, 8, 7, 6, 6, 5, 5, 5, 4, 4, 4, 4, 3, 3,
        2, 2, 2, 2, 1, 1, 1, 1, 1, 0, 0, 0,
    ];
    TABLE[crate::game::stat_index(stat)]
}

/// The class used for body-part sums (C++ cp_ptr: the base class).
fn body_class<'a>(gd: &'a GameData, ps: &PlayerState) -> Option<&'a crate::data::ClassDef> {
    let name = if ps.base_class.is_empty() {
        &ps.class_name
    } else {
        &ps.base_class
    };
    gd.classes.iter().find(|c| &c.name == name)
}

/// calc_body (xtra1.cc:1977): the number of each body part of the current
/// shape, summing race + subrace (+ class) in the player's own body and the
/// possessed monster's parts otherwise, with the mimicry extra limbs and
/// the per-part caps of tables.cc max_body_part.
pub fn calc_body(gd: &GameData, ps: &PlayerState) -> [i32; 6] {
    let mut bp = [0i32; 6];
    if let Some((pdef, _, _)) = ps.possessed {
        if let Some(def) = gd.monsters.get(pdef) {
            bp.copy_from_slice(&def.body_parts);
        }
    } else {
        if let Some(race) = gd.races.iter().find(|r| r.name == ps.race_name) {
            for i in 0..6 {
                bp[i] += race.body_parts[i];
            }
        }
        if let Some(rm) = gd.racemods.iter().find(|r| r.id == ps.subrace) {
            for i in 0..6 {
                bp[i] += rm.body_parts[i];
            }
        }
    }
    if let Some(class) = body_class(gd, ps) {
        for i in 0..6 {
            bp[i] += class.body_parts[i];
        }
    }
    for i in 0..6 {
        bp[i] = bp[i].clamp(0, MAX_BODY_PART[i]);
    }
    if ps.mimic_extra_turns > 0 {
        if ps.mimic_extra & CLASS_ARMS != 0 {
            bp[BODY_WEAPON] = (bp[BODY_WEAPON] + 1).min(3);
            bp[BODY_ARMS] = (bp[BODY_ARMS] + 1).min(3);
        }
        if ps.mimic_extra & CLASS_LEGS != 0 {
            bp[BODY_LEGS] = (bp[BODY_LEGS] + 1).min(2);
        }
    }
    if ps.mimic_form == Some(MIMIC_BEAR) {
        bp[BODY_WEAPON] = 0;
        bp[BODY_ARMS] = 0;
        bp[BODY_LEGS] = 0;
    }
    bp
}

/// The body part and minimum count an equipment slot needs (xtra1.cc
/// calc_body's INVEN_* mapping).
pub fn slot_body_part(slot: usize) -> (usize, i32) {
    use crate::data::*;
    match slot {
        SLOT_WEAPON | SLOT_BOW => (BODY_WEAPON, 1),
        SLOT_WEAPON2 => (BODY_WEAPON, 2),
        SLOT_BODY | SLOT_CLOAK | SLOT_LITE | SLOT_QUIVER | SLOT_SYMBIOTE => (BODY_TORSO, 1),
        SLOT_RING1 => (BODY_FINGER, 1),
        SLOT_RING2 => (BODY_FINGER, 2),
        SLOT_HEAD | SLOT_AMULET => (BODY_HEAD, 1),
        SLOT_HANDS | SLOT_SHIELD | SLOT_TOOL => (BODY_ARMS, 1),
        SLOT_HANDS2 | SLOT_SHIELD2 => (BODY_ARMS, 2),
        SLOT_FEET => (BODY_LEGS, 1),
        SLOT_FEET2 => (BODY_LEGS, 2),
        _ => (BODY_TORSO, 1),
    }
}

/// Whether the current body has this equipment slot.  The extra
/// weapon/shield/gloves slots need the extra arms, the second boots slot
/// the extra legs (xtra1.cc calc_body).
pub fn slot_usable(ps: &PlayerState, slot: usize) -> bool {
    use crate::data::{SLOT_FEET2, SLOT_HANDS2, SLOT_SHIELD2, SLOT_WEAPON2};
    let live = ps.mimic_extra_turns > 0;
    let arms = live && (ps.mimic_extra & CLASS_ARMS) != 0;
    let legs = live && (ps.mimic_extra & CLASS_LEGS) != 0;
    match slot {
        SLOT_WEAPON2 | SLOT_SHIELD2 | SLOT_HANDS2 => arms,
        SLOT_FEET2 => legs,
        _ => true,
    }
}

/// Whether the current body has this slot, based on the actual body-part
/// counts (calc_body / xtra1.cc) rather than the mimicry extras alone.
pub fn slot_usable_body(gd: &GameData, ps: &PlayerState, slot: usize) -> bool {
    let (part, needed) = slot_body_part(slot);
    calc_body(gd, ps)[part] >= needed
}

/// Slots the current body no longer has release their gear into the
/// pack (xtra1.cc calc_body_bonus calls inven_takeoff on a lost body
/// part); the stat deltas are undone and the AC refreshed.
pub fn drop_unusable_slots(
    gd: &GameData,
    ps: &mut PlayerState,
    inv: &mut crate::item::Inventory,
    log: &mut MessageLog,
) {
    let mut lost = false;
    for slot in 0..crate::data::NUM_SLOTS {
        if slot_usable_body(gd, ps, slot) {
            continue;
        }
        if let Some(it) = inv.equip[slot].take() {
            let d = crate::item::stat_deltas(gd, &it);
            for i in 0..6 {
                ps.stats[i] -= d[i];
            }
            log.add(format!(
                "{} slips free as your body shifts.",
                it.label(gd, &inv.known)
            ));
            inv.pack.push(it);
            lost = true;
        }
    }
    if lost {
        ps.ac = crate::game::player_ac(ps, inv, gd);
    }
}

/// A body without arms/legs/head/fingers forces the gear on the missing
/// parts off (xtra1.cc calc_body's inven_takeoff loop).  This covers the
/// Bear shape and possessed bodies (e.g. DeathMold: no head, no legs).
pub fn enforce_body(gd: &GameData, ps: &mut PlayerState, inv: &mut crate::item::Inventory) {
    let mut removed = false;
    for slot in 0..crate::data::NUM_SLOTS {
        if slot_usable_body(gd, ps, slot) {
            continue;
        }
        if let Some(item) = inv.equip[slot].take() {
            let d = crate::item::stat_deltas(gd, &item);
            for i in 0..6 {
                ps.stats[i] -= d[i];
            }
            inv.pack.push(item);
            removed = true;
        }
    }
    if removed {
        ps.ac = crate::game::player_ac(ps, inv, gd);
    }
}

/// Failure chance of a shape change with a cloak (cmd7.cc
/// get_mimic_chance + clamp_failure_chance(chance, 2)).
pub fn get_mimic_chance(ps: &PlayerState, form: u32) -> i32 {
    let level = MIMIC_FORMS
        .get(form as usize)
        .map(|f| f.level as i32)
        .unwrap_or(1);
    let mut chance = level * 3;
    chance -= ps.skill_scale(crate::skill::SK_MIMICRY, 150);
    chance -= 3 * adj_mag_stat(ps.stats[DEX]);
    // clamp_failure_chance(chance, 2): the floor is applied *before* the
    // stun penalty, so a stunned caster cannot dip below 2+15/25.
    if chance < 2 {
        chance = 2;
    }
    if ps.stun > 50 {
        chance += 25;
    } else if ps.stun > 0 {
        chance += 15;
    }
    chance.min(95)
}

/// Check if the player may travel with extra limbs (cmd7.cc
/// mimic_forbid_travel).
pub fn forbid_travel(ps: &PlayerState) -> bool {
    let att = ps.mimic_extra;
    let value = ps.mimic_extra_turns;
    value > 0 && (att & (CLASS_ARMS | CLASS_LEGS)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;

    fn totals_for_form(
        gd: &GameData,
        ps: &mut PlayerState,
        form: u32,
        ml: i32,
    ) -> EquipTotals {
        ps.mimic_form = Some(form);
        ps.mimic_level = ml;
        crate::item::Inventory::default().totals_for(gd, ps)
    }

    #[test]
    fn mimic_table_names_levels_and_durations_match_the_original() {
        // mimic.cc mimic_forms[] order: names, object names, level/rarity,
        // duration ranges and the ToME/Theme module filter.
        let names: Vec<&str> = MIMIC_FORMS.iter().map(|f| f.name).collect();
        assert_eq!(
            names,
            [
                "Abomination",
                "Mouse",
                "Eagle",
                "Eagle",
                "Wolf",
                "Spider",
                "Elder Ent",
                "Vapour",
                "Serpent",
                "Mumak",
                "Bear",
                "Balrog",
                "Maia",
                "Fire Elem."
            ]
        );
        let obj: Vec<&str> = MIMIC_FORMS.iter().map(|f| f.obj_name).collect();
        assert_eq!(
            &obj[..10],
            [
                "Abominable Cloak",
                "Mouse Fur",
                "Feathers Cloak",
                "Feathered Cloak",
                "Wolf Pelt",
                "Spider Web",
                "Entish Bark",
                "Cloak of Mist",
                "Snakeskin Cloak",
                "Mumak Hide"
            ]
        );
        assert!(MIMIC_FORMS[10..].iter().all(|f| f.obj_name.is_empty()));
        // The second Eagle is Theme-only (modules { MODULE_THEME, -1 }).
        assert!(!MIMIC_FORMS[3].enabled);
        assert!(MIMIC_FORMS
            .iter()
            .enumerate()
            .all(|(i, f)| f.enabled == (i != 3)));
        // limit flags: Elder Ent and the four extra shapes.
        assert!(MIMIC_FORMS[6].limit && MIMIC_FORMS[10].limit);
        assert!(!MIMIC_FORMS[1].limit);
        let levels: Vec<u32> = MIMIC_FORMS.iter().map(|f| f.level).collect();
        assert_eq!(levels, [1, 1, 10, 10, 20, 25, 40, 15, 30, 40, 1, 1, 1, 1]);
        // get_mimic_form / get_mimic_name / get_mimic_object_name /
        // get_mimic_level.
        assert_eq!(form(4).name, "Wolf");
        assert_eq!(form_name(5), "Spider");
        assert_eq!(form_obj_name(0), "Abominable Cloak");
        assert_eq!(form_level(6), 40);
        assert_eq!(form(99).name, "Abomination", "unknown index falls back");
        // resolve_mimic_name skips the disabled Theme copy.
        assert_eq!(resolve_name("Eagle"), 2);
        assert_eq!(resolve_name("Mouse"), 1);
        assert_eq!(resolve_name("Maia"), 12);
        assert_eq!(resolve_name("Vampire"), -1);
        // get_mimic_random_duration uses the per-form closed interval.
        let mut rng = rand::thread_rng();
        for _ in 0..200 {
            let d = random_duration(6, &mut rng);
            assert!((10..=30).contains(&d));
        }
        for _ in 0..200 {
            let d = random_duration(0, &mut rng);
            assert!((20..=100).contains(&d));
        }
    }

    #[test]
    fn mimic_calc_stat_deltas_match_the_original_formulas() {
        // [STR, INT, WIS, DEX, CON, CHA] picked at level break points.
        assert_eq!(form_stat_deltas(MIMIC_ABOMINATION, 50), [-10; 6]);
        assert_eq!(form_stat_deltas(MIMIC_MOUSE, 10), [-5, 0, 0, 3, 1, 0]);
        assert_eq!(form_stat_deltas(2, 10), [-3, -1, 1, 2, 4, -1]);
        assert_eq!(form_stat_deltas(4, 20), [3, -3, 0, 4, 0, -2]);
        assert_eq!(form_stat_deltas(5, 25), [-4, 6, 6, 4, -5, -10]);
        assert_eq!(form_stat_deltas(6, 40), [8, -5, -5, -4, 8, -7]);
        assert_eq!(form_stat_deltas(7, 15), [-4, 0, 0, 5, -4, -10]);
        assert_eq!(form_stat_deltas(8, 30), [3, -6, -6, -4, 4, -6]);
        assert_eq!(form_stat_deltas(9, 40), [10, -8, -4, -5, 13, -10]);
        assert_eq!(form_stat_deltas(10, 40), [3, 3, 3, -1, 3, -10]);
        assert_eq!(form_stat_deltas(11, 20), [9, 2, -7, 2, 9, -7]);
        assert_eq!(form_stat_deltas(12, 20), [9; 6]);
        assert_eq!(form_stat_deltas(13, 20), [9, 0, -9, 9, 0, 0]);
        // set_mimic applies the deltas to the base stats and undoes them.
        let gd = load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Mimic".into(), 0, 0);
        let base = ps.stats;
        let mut log = MessageLog::default();
        set_mimic(&gd, &mut ps, 10, MIMIC_WOLF, 20, &mut log);
        for i in 0..6 {
            assert_eq!(ps.stats[i], base[i] + form_stat_deltas(MIMIC_WOLF, 20)[i]);
        }
        set_mimic(&gd, &mut ps, 0, MIMIC_WOLF, 20, &mut log);
        assert_eq!(ps.stats, base);
    }

    #[test]
    fn mimic_calc_effects_match_the_original_flags() {
        let gd = load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Mimic".into(), 0, 0);

        // Abomination: -10 speed, aggravate.
        let t = totals_for_form(&gd, &mut ps, MIMIC_ABOMINATION, 50);
        assert_eq!(t.speed, -10);
        assert!(t.aggravate);

        // Mouse: speed 5+ml/7, to_h 10+ml/5, stealth 10+ml/5.
        let t = totals_for_form(&gd, &mut ps, MIMIC_MOUSE, 10);
        assert_eq!((t.speed, t.to_h, t.stealth), (6, 12, 12));

        // Eagle at 35: free action, resist elec, shield elec, fly, see invis.
        let t = totals_for_form(&gd, &mut ps, 2, 35);
        assert!(t.fly && t.see_invis && t.free_act && t.sh_elec && t.feather);
        assert!(t.resists.contains("ELEC"));

        // Wolf at 35: resists cold/dark/conf/fear, free action, see invis.
        let t = totals_for_form(&gd, &mut ps, 4, 35);
        assert!(t.free_act && t.see_invis);
        for r in ["COLD", "DARK", "CONF", "FEAR"] {
            assert!(t.resists.contains(r), "wolf misses {r}");
        }

        // Spider at 40: +5 speed, poisons/fear/dark, climb.
        let t = totals_for_form(&gd, &mut ps, 5, 40);
        assert_eq!(t.speed, 5);
        assert!(t.climb && t.resists.contains("POIS"));

        // Ent at 40: speed -(5+4), ac 10+ml, statuses and resists.
        let t = totals_for_form(&gd, &mut ps, 6, 40);
        assert_eq!((t.speed, t.ac), (-9, 50));
        assert!(t.free_act && t.regen && t.see_invis && t.sens_fire);
        assert!(t.resists.contains("POIS") && t.resists.contains("COLD"));

        // Vapour at 15: ac 55, to_h -40, stealth 13, imm cold, feather.
        let t = totals_for_form(&gd, &mut ps, 7, 15);
        assert_eq!((t.ac, t.to_h, t.stealth), (55, -40, 13));
        assert!(t.immunities.contains("COLD") && t.feather && t.sens_fire);

        // Serpent at 25: speed 10+4, ac 3+3, free action from 25.
        let t = totals_for_form(&gd, &mut ps, 8, 25);
        assert_eq!((t.speed, t.ac), (14, 6));
        assert!(t.free_act && t.resists.contains("POIS"));
        // Free action only from level 25.
        let t = totals_for_form(&gd, &mut ps, 8, 24);
        assert!(!t.free_act);

        // Mumak at 40: speed -9, ac 16, to_d 5+2*40/3 = 31, resist nexus.
        let t = totals_for_form(&gd, &mut ps, 9, 40);
        assert_eq!((t.speed, t.ac, t.to_d), (-9, 16, 31));
        assert!(t.resists.contains("NEXUS") && t.free_act);

        // Bear at 40: speed 3, ac 5+80/3=31, resists conf/nexus, regen.
        let t = totals_for_form(&gd, &mut ps, 10, 40);
        assert_eq!((t.speed, t.ac), (3, 31));
        assert!(t.free_act && t.regen);
        assert!(t.resists.contains("CONF") && t.resists.contains("NEXUS"));

        // Balrog adds one blow and its immunities; Maia adds two.
        let t = totals_for_form(&gd, &mut ps, 11, 50);
        assert_eq!(t.blows, 1);
        assert!(t.immunities.contains("ACID")
            && t.immunities.contains("FIRE")
            && t.immunities.contains("ELEC"));
        assert!(t.sh_fire && t.hold_life && t.feather && t.regen && t.lite >= 1);
        let t = totals_for_form(&gd, &mut ps, 12, 50);
        assert_eq!(t.blows, 2);
        assert!(t.immunities.contains("COLD") && t.immunities.contains("ACID"));
        // Fire elemental has no blow bonus.
        let t = totals_for_form(&gd, &mut ps, 13, 50);
        assert_eq!(t.blows, 0);
        assert!(t.immunities.contains("FIRE") && t.sh_fire && t.lite >= 1);

        // No form: no mimic contribution.
        ps.mimic_form = None;
        let t = crate::item::Inventory::default().totals_for(&gd, &ps);
        assert_eq!((t.speed, t.blows, t.aggravate), (0, 0, false));
    }

    #[test]
    fn mimic_shape_powers_follow_mimic_levels() {
        let gd = load_game_data();
        let inv = crate::item::Inventory::default();
        let mut ps = crate::birth::make_player(&gd, "Mimic".into(), 0, 0);
        // POWER_INVISIBILITY (62) from Mouse at skill 30.
        ps.mimic_form = Some(MIMIC_MOUSE);
        ps.mimic_level = 29;
        assert!(!crate::game::available_powers(&gd, &inv, &ps).contains(&62));
        ps.mimic_level = 30;
        assert!(crate::game::available_powers(&gd, &inv, &ps).contains(&62));
        // POWER_WEB (63) from Spider at skill 25.
        ps.mimic_form = Some(MIMIC_SPIDER);
        ps.mimic_level = 24;
        assert!(!crate::game::available_powers(&gd, &inv, &ps).contains(&63));
        ps.mimic_level = 25;
        assert!(crate::game::available_powers(&gd, &inv, &ps).contains(&63));
        // PWR_GROW_TREE (40) from the Ent unconditionally.
        ps.mimic_form = Some(MIMIC_ENT);
        ps.mimic_level = 1;
        assert!(crate::game::available_powers(&gd, &inv, &ps).contains(&40));
    }

    #[test]
    fn mimic_change_chance_and_travel_forbid_follow_cmd7() {
        let gd = load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Mimic".into(), 0, 0);
        // get_mimic_chance: level*3 - mimicry skill scale - 3*adj_mag_stat
        // (DEX), then clamp_failure_chance(chance, 2).
        ps.skills
            .insert(crate::skill::SK_MIMICRY, 10 * crate::skill::SKILL_STEP);
        ps.stats[DEX] = 18;
        let expected = {
            let chance = MIMIC_FORMS[MIMIC_WOLF as usize].level as i32 * 3
                - ps.skill_scale(crate::skill::SK_MIMICRY, 150)
                - 3 * adj_mag_stat(ps.stats[DEX]);
            chance.clamp(2, 95)
        };
        assert_eq!(get_mimic_chance(&ps, MIMIC_WOLF), expected);
        // Stun is applied after the 2% floor: a stunned caster is at
        // least 2+15 (cmd7.cc clamp_failure_chance).
        ps.stun = 10;
        assert_eq!(get_mimic_chance(&ps, MIMIC_WOLF), expected + 15);
        ps.stun = 51;
        assert_eq!(get_mimic_chance(&ps, MIMIC_WOLF), expected + 25);
        // A weak caster hits the 2% floor before the stun penalty.
        let mut weak = crate::birth::make_player(&gd, "Weak".into(), 0, 0);
        weak.skills.clear();
        weak.skill_mods.clear();
        weak.stats[DEX] = 40;
        weak.stun = 10;
        assert_eq!(get_mimic_chance(&weak, MIMIC_MOUSE), 2 + 15);
        // mimic_forbid_travel: only while the extra limbs last.
        let mut ps = crate::birth::make_player(&gd, "Limbs".into(), 0, 0);
        assert!(!forbid_travel(&ps));
        ps.mimic_extra = CLASS_ARMS;
        ps.mimic_extra_turns = 0;
        assert!(!forbid_travel(&ps));
        ps.mimic_extra_turns = 5;
        assert!(forbid_travel(&ps));
        ps.mimic_extra = CLASS_WALL;
        assert!(!forbid_travel(&ps), "wall mimicry is not a travel block");
    }

    #[test]
    fn find_random_mimic_shape_respects_limit_level_and_rarity() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(3);
        // Cloak shapes (limit=true) can roll the limited Ent; oil cannot.
        let mut saw_ent = false;
        for _ in 0..500 {
            let f = find_random_mimic_shape(127, true, &mut rng);
            assert!(MIMIC_FORMS[f as usize].enabled);
            saw_ent |= f == MIMIC_ENT;
            // Rarity 101 forms are never returned (mimic.cc rarity < 100).
            assert!(f != MIMIC_ABOMINATION && f != MIMIC_BEAR && f != MIMIC_BALROG);
        }
        assert!(saw_ent, "cloak rolls can include the Ent");
        for _ in 0..500 {
            let f = find_random_mimic_shape(127, false, &mut rng);
            assert_ne!(f, MIMIC_ENT, "oil never rolls a limited form");
        }
        // Level 0 never passes the level check, so the fallback is index 0.
        for _ in 0..50 {
            assert_eq!(find_random_mimic_shape(0, false, &mut rng), 0);
        }
    }
}
