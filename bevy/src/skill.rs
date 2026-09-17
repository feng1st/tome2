#![allow(dead_code)] // many skill ids/abilities wait for later classes
//! The s_info skill tree and ab_info abilities (ninth round).
//!
//! Skill values are stored in 1000ths of a level (`SKILL_STEP`); a skill
//! point adds the skill's `mod` value. Values are computed once at birth
//! from the general/race/class modifiers in p_info.txt (compute_skills),
//! exactly like src/skills.cc. The screen ('C') lets the player allocate
//! unspent points up to `level + MAX_SKILL_OVERAGE`, with the percentage
//! "friendly" bonuses and the mutual exclusions from s_info.

use std::collections::HashMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::data::{GameData, SkillMod};
use crate::game::{MessageLog, PlayerState};
use crate::item::Inventory;

pub const SKILL_STEP: i32 = 1000;
pub const SKILL_MAX: i32 = 50000;
/// modules[].skills.max_skill_overage (tables.cc: ToME = 4).
pub const MAX_SKILL_OVERAGE: i32 = 4;
/// modules[].skills.skill_per_level (tables.cc: ToME = 6).
pub const SKILL_PER_LEVEL: i32 = 6;
/// Number of random-gain candidates of the Lost Sword quest.
pub const LOST_SWORD_NSKILLS: usize = 4;

// --- skill indices (src/skills_defs.hpp) ---
pub const SK_CONVEYANCE: u32 = 1;
pub const SK_MANA: u32 = 2;
pub const SK_FIRE: u32 = 3;
pub const SK_AIR: u32 = 4;
pub const SK_WATER: u32 = 5;
pub const SK_NATURE: u32 = 6;
pub const SK_EARTH: u32 = 7;
pub const SK_SYMBIOTIC: u32 = 8;
pub const SK_MUSIC: u32 = 9;
pub const SK_DIVINATION: u32 = 10;
pub const SK_TEMPORAL: u32 = 11;
pub const SK_DRUID: u32 = 12;
pub const SK_DAEMON: u32 = 13;
pub const SK_META: u32 = 14;
pub const SK_MAGIC: u32 = 15;
pub const SK_COMBAT: u32 = 16;
pub const SK_MASTERY: u32 = 17;
pub const SK_SWORD: u32 = 18;
pub const SK_AXE: u32 = 19;
pub const SK_POLEARM: u32 = 20;
pub const SK_HAFTED: u32 = 21;
pub const SK_BACKSTAB: u32 = 22;
pub const SK_ARCHERY: u32 = 23;
pub const SK_SLING: u32 = 24;
pub const SK_BOW: u32 = 25;
pub const SK_XBOW: u32 = 26;
pub const SK_BOOMERANG: u32 = 27;
pub const SK_SPIRITUALITY: u32 = 28;
pub const SK_MINDCRAFT: u32 = 29;
pub const SK_MISC: u32 = 30;
pub const SK_NECROMANCY: u32 = 31;
pub const SK_MIMICRY: u32 = 32;
pub const SK_ANTIMAGIC: u32 = 33;
pub const SK_STEALTH: u32 = 36;
pub const SK_STEALING: u32 = 40;
pub const SK_SORCERY: u32 = 41;
pub const SK_HAND: u32 = 42;
pub const SK_THAUMATURGY: u32 = 43;
pub const SK_SUMMON: u32 = 44;
pub const SK_SPELL: u32 = 45;
pub const SK_DODGE: u32 = 46;
pub const SK_BEAR: u32 = 47;
pub const SK_LORE: u32 = 48;
pub const SK_PRESERVATION: u32 = 49;
pub const SK_POSSESSION: u32 = 50;
pub const SK_MIND: u32 = 51;
pub const SK_CRITS: u32 = 52;
pub const SK_PRAY: u32 = 53;
pub const SK_LEARN: u32 = 54;
pub const SK_UDUN: u32 = 55;
pub const SK_DEVICE: u32 = 56;
pub const SK_STUN: u32 = 57;
pub const SK_BOULDER: u32 = 58;
pub const SK_GEOMANCY: u32 = 59;

// --- ability indices (ab_info.txt) ---
pub const AB_SPREAD_BLOWS: u32 = 0;
pub const AB_TREE_WALKING: u32 = 1;
pub const AB_PERFECT_CASTING: u32 = 2;
pub const AB_EXTRA_BLOW1: u32 = 3;
pub const AB_EXTRA_BLOW2: u32 = 4;
pub const AB_AMMO_CREATION: u32 = 5;
pub const AB_TOUCH_OF_DEATH: u32 = 6;
pub const AB_FAR_REACHING: u32 = 8;
pub const AB_UNDEAD_FORM: u32 = 10;

/// Monster ego modifier chars (monster_ego_modify): add/sub/set/percent.
pub fn modify_aux(a: i32, b: i32, op: &str) -> i32 {
    match op.chars().next().unwrap_or('+') {
        '+' => a + b,
        '-' => a - b,
        '=' => b,
        '%' => a * b / 100,
        _ => a + b,
    }
}

/// Given the name of a skill, returns the skill id or None if no such
/// skill is found (skills.cc find_skill; exact match, index 0 skipped in
/// the original because it is the unused root entry).
pub fn find_skill(gd: &GameData, needle: &str) -> Option<u32> {
    gd.skills
        .iter()
        .find(|s| !s.name.is_empty() && s.name == needle)
        .map(|s| s.id)
}

/// Case-insensitive find_skill (skills.cc find_skill_i).
pub fn find_skill_i(gd: &GameData, needle: &str) -> Option<u32> {
    gd.skills
        .iter()
        .find(|s| !s.name.is_empty() && s.name.eq_ignore_ascii_case(needle))
        .map(|s| s.id)
}

/// Given the name of an ability, returns its id or None (skills.cc
/// find_ability).
pub fn find_ability(gd: &GameData, name: &str) -> Option<u32> {
    gd.abilities
        .iter()
        .find(|a| !a.name.is_empty() && a.name == name)
        .map(|a| a.id)
}

/// Order two abilities by name (skills.cc compare_abilities).
pub fn compare_abilities(a: &crate::data::AbilityDef, b: &crate::data::AbilityDef) -> std::cmp::Ordering {
    a.name.cmp(&b.name)
}

/// The melee style list (skills.cc melee_skills[MAX_MELEE]).
pub const MELEE_SKILLS: [u32; 3] = [SK_MASTERY, SK_HAND, SK_BEAR];
/// The melee style names (skills.cc melee_names[MAX_MELEE]).
pub const MELEE_NAMES: [&str; 3] = ["Weapon combat", "Barehanded combat", "Bearform combat"];

/// Index of the current melee style in MELEE_SKILLS, 0 if unknown
/// (skills.cc get_melee_skill).
pub fn get_melee_skill(ps: &PlayerState) -> usize {
    MELEE_SKILLS
        .iter()
        .position(|s| ps.melee_style == *s)
        .unwrap_or(0)
}

/// Name of the current melee style (skills.cc get_melee_name).
pub fn get_melee_name(ps: &PlayerState) -> &'static str {
    MELEE_NAMES[get_melee_skill(ps)]
}

/// The developed, visible melee styles (skills.cc get_melee_skills).
/// Returns the count and sets the same parallel bool array semantics.
pub fn get_melee_skills(ps: &PlayerState) -> (usize, [bool; 3]) {
    let mut flags = [false; 3];
    let mut n = 0;
    for (i, sk) in MELEE_SKILLS.iter().enumerate() {
        if ps.skill_value(*sk) > 0 && !ps.skill_hidden.contains(sk) {
            flags[i] = true;
            n += 1;
        }
    }
    (n, flags)
}

/// The style the melee chooser cycles to: the next developed style after
/// the current one (skills.cc choose_melee).  None when the player has no
/// visible style; the current style when it is the only one.
pub fn next_melee_style(ps: &PlayerState) -> Option<(u32, &'static str)> {
    let (n, visible) = get_melee_skills(ps);
    if n == 0 {
        return None;
    }
    let cur = get_melee_skill(ps);
    for step in 1..=MELEE_SKILLS.len() {
        let i = (cur + step) % MELEE_SKILLS.len();
        if visible[i] {
            return Some((MELEE_SKILLS[i], MELEE_NAMES[i]));
        }
    }
    Some((MELEE_SKILLS[cur], MELEE_NAMES[cur]))
}

fn apply_mods(target: &mut HashMap<String, (i32, i32)>, mods: &[SkillMod]) {
    for m in mods {
        let e = target.entry(m.skill.clone()).or_insert((0, 0));
        e.0 = modify_aux(e.0, m.base, &m.bop);
        e.1 = modify_aux(e.1, m.gain, &m.mop);
    }
}

/// Augment a skill value/modifier with the i-th skill modifier
/// (skills.cc augment_skills).  Out-of-range i leaves both untouched.
pub fn augment_skills(v: &mut i32, m: &mut i32, modifiers: &[SkillMod], i: usize) {
    if let Some(s) = modifiers.get(i) {
        *v = modify_aux(*v, s.base, &s.bop);
        *m = modify_aux(*m, s.gain, &s.mop);
    }
}

/// Birth-time skill initialization: general (p_info G:k:), then race,
/// subrace, class, then the chosen specialisation (src/skills.cc
/// compute_skills/init_skill).
pub fn compute_skills(ps: &mut PlayerState, gd: &GameData) {
    let mut values: HashMap<String, (i32, i32)> = HashMap::new();
    apply_mods(&mut values, &gd.general_skills);
    if let Some(r) = gd.races.iter().find(|r| r.name == ps.race_name) {
        apply_mods(&mut values, &r.skills);
    }
    if let Some(m) = gd.racemods.get(ps.subrace as usize) {
        apply_mods(&mut values, &m.skills);
    }
    if let Some(c) = ps.class(gd) {
        apply_mods(&mut values, &c.skills);
    }
    if let Some(s) = ps.spec(gd) {
        apply_mods(&mut values, &s.skills);
    }
    ps.skills.clear();
    ps.skill_mods.clear();
    ps.skill_hidden.clear();
    ps.skill_dev.clear();
    for s in &gd.skills {
        let (v, m) = values.get(&s.name).copied().unwrap_or((0, 0));
        if v != 0 {
            ps.skills.insert(s.id, v.clamp(0, SKILL_MAX));
        }
        if m != 0 {
            ps.skill_mods.insert(s.id, m);
        }
        if s.has("HIDDEN") {
            ps.skill_hidden.insert(s.id);
        }
    }
    // Mark the ancestors of every developed branch (birth.cc:2571-2597:
    // `if (s_info[i].value || s_info[i].mod)`, so a zero mod counts as
    // untrained).
    for s in &gd.skills {
        if ps.skill_value(s.id) != 0
            || ps.skill_mods.get(&s.id).copied().unwrap_or(0) != 0
        {
            let mut z = s.father;
            while z > 0 {
                ps.skill_dev.insert(z as u32);
                z = gd
                    .skills
                    .iter()
                    .find(|x| x.id as i32 == z)
                    .map(|x| x.father)
                    .unwrap_or(-1);
            }
        }
    }
    recompute_hidden(ps, gd);
    select_default_melee(ps, gd);
}

/// HIDDEN is permanent; AUTO_HIDE skills are re-hidden every recalculation
/// and unlocked by state (Bearform when in bear form, Udun when following
/// Melkor -- src/xtra1.cc:2846, src/gods.cc:105).
pub fn recompute_hidden(ps: &mut PlayerState, gd: &GameData) {
    for s in &gd.skills {
        if s.has("HIDDEN") || s.has("AUTO_HIDE") {
            ps.skill_hidden.insert(s.id);
        }
    }
    if ps.god == 4 {
        ps.skill_hidden.remove(&SK_UDUN);
    }
    // The Bearform-combat skill is revealed while in Bear shape
    // (mimic.cc bear_calc).
    if ps.mimic_form == Some(crate::mimic::MIMIC_BEAR) {
        ps.skill_hidden.remove(&SK_BEAR);
    }
}

/// The first developed melee style (src/skills.cc select_default_melee).
pub fn select_default_melee(ps: &mut PlayerState, _gd: &GameData) {
    let mut style = SK_MASTERY;
    for sk in [SK_MASTERY, SK_HAND, SK_BEAR] {
        if ps.skill_value(sk) > 0 && !ps.skill_hidden.contains(&sk) {
            style = sk;
            break;
        }
    }
    ps.melee_style = style;
}

/// Abilities granted by race/class/spec when reaching a level
/// (src/skills.cc apply_level_abilities).
pub fn apply_level_abilities(
    ps: &mut PlayerState,
    gd: &GameData,
    level: u32,
    mut log: Option<&mut MessageLog>,
) {
    let mut grants: Vec<String> = Vec::new();
    if let Some(r) = gd.races.iter().find(|r| r.name == ps.race_name) {
        grants.extend(
            r.abilities
                .iter()
                .filter(|a| a.level == level)
                .map(|a| a.ability.clone()),
        );
    }
    if let Some(m) = gd.racemods.get(ps.subrace as usize) {
        grants.extend(
            m.abilities
                .iter()
                .filter(|a| a.level == level)
                .map(|a| a.ability.clone()),
        );
    }
    if let Some(c) = ps.class(gd) {
        grants.extend(
            c.abilities
                .iter()
                .filter(|a| a.level == level)
                .map(|a| a.ability.clone()),
        );
    }
    if let Some(s) = ps.spec(gd) {
        grants.extend(
            s.abilities
                .iter()
                .filter(|a| a.level == level)
                .map(|a| a.ability.clone()),
        );
    }
    for name in grants {
        if let Some(ab) = gd.abilities.iter().find(|a| a.name == name) {
            if !ps.has_ability(ab.id) {
                ps.abilities.push(ab.id);
                if level > 1 {
                    if let Some(log) = log.as_deref_mut() {
                        log.add(format!("You have learned the ability '{}'.", ab.name));
                    }
                }
            }
        }
    }
}

/// Learn an ability through the abilities screen (cost in skill points).
pub fn learn_ability(
    ps: &mut PlayerState,
    gd: &GameData,
    ab: &crate::data::AbilityDef,
) -> Result<(), String> {
    ability_status(ps, gd, ab)?;
    ps.skill_points -= ab.cost;
    ps.abilities.push(ab.id);
    Ok(())
}

/// Why an ability cannot be learned right now (skills.cc can_learn_ability).
pub fn ability_status(
    ps: &PlayerState,
    gd: &GameData,
    ab: &crate::data::AbilityDef,
) -> Result<(), String> {
    if ps.has_ability(ab.id) {
        return Err("known".into());
    }
    if ps.skill_points < ab.cost {
        return Err("not enough points".into());
    }
    for (skill_name, level) in &ab.need_skills {
        let Some(sk) = gd.skills.iter().find(|s| &s.name == skill_name) else {
            continue;
        };
        if ps.skill(sk.id) < *level as i32 {
            return Err(format!("{} skill {} needed", skill_name, level));
        }
    }
    for need in &ab.need_abilities {
        let Some(na) = gd.abilities.iter().find(|a| &a.name == need) else {
            continue;
        };
        if !ps.has_ability(na.id) {
            return Err(format!("'{}' needed", need));
        }
    }
    for i in 0..6 {
        if ab.stats[i] > -1 && ps.stats[i] < ab.stats[i] {
            return Err(format!(
                "{} {} needed",
                ["STR", "INT", "WIS", "DEX", "CON", "CHR"][i],
                ab.stats[i]
            ));
        }
    }
    Ok(())
}

/// Grant any race/class abilities the character's level has reached.
/// Called every turn (cheap idempotent scan), so multi-level exp gains
/// still pick up every grant.
pub fn sync_level_abilities(ps: &mut PlayerState, gd: &GameData, log: &mut MessageLog) {
    for level in 2..=ps.level {
        apply_level_abilities(ps, gd, level, Some(&mut *log));
    }
}

/// Keep the maximum mana in sync with the original calc_mana formula
/// (xtra1.cc): the whole computation in original order (base, race/class,
/// Eru, gloves, TR_MANA, armour encumbrance, Mana skill, Inertia
/// Control).  `magic_mana_bonus` stores the currently applied value.
pub fn sync_magic_mana(ps: &mut PlayerState, gd: &GameData, inv: &Inventory) {
    let want = crate::game::calc_mana(ps, gd, inv);
    let diff = want - ps.magic_mana_bonus;
    if diff != 0 {
        ps.magic_mana_bonus = want;
        ps.max_mana = (ps.max_mana + diff).max(0);
        if ps.mana > ps.max_mana {
            ps.mana = ps.max_mana;
        }
    }
}

/// Spell school id -> s_info skill id. Music is school 100 in this port
/// (the original s_info index is 9); everything else matches.
pub fn school_skill_id(school: u32) -> u32 {
    if school == 100 {
        SK_MUSIC
    } else {
        school
    }
}

/// Skill level in a spell's school (0 when never trained).
pub fn school_skill(ps: &PlayerState, school: u32) -> i32 {
    ps.skill(school_skill_id(school))
}

/// A skill cannot be raised above `level + MAX_SKILL_OVERAGE`.
pub fn allocation_cap(ps: &PlayerState) -> i32 {
    (ps.level as i32 + MAX_SKILL_OVERAGE + 1) * SKILL_STEP
}

/// Spend one point on a skill, applying the friendly percentage bonuses
/// and wiping mutually exclusive skills (skills.cc increase_skill +
/// recalc_skills_theory).
pub fn increase_skill(ps: &mut PlayerState, gd: &GameData, id: u32) -> Result<(), String> {
    if ps.skill_points <= 0 {
        return Err("You have no skill points to spend.".into());
    }
    let Some(def) = gd.skills.iter().find(|s| s.id == id) else {
        return Err("Unknown skill.".into());
    };
    let Some(&modv) = ps.skill_mods.get(&id) else {
        return Err("You cannot train this skill.".into());
    };
    if modv == 0 {
        return Err("You cannot train this skill.".into());
    }
    let value = ps.skill_value(id);
    if value >= SKILL_MAX {
        return Err("This skill is already mastered.".into());
    }
    if value + modv >= allocation_cap(ps) {
        return Err(format!(
            "Cannot raise a skill value above {} + player level.",
            MAX_SKILL_OVERAGE
        ));
    }
    ps.skill_points -= 1;
    ps.skills.insert(id, value + modv);
    *ps.skill_invest.entry(id).or_insert(0) += 1;
    let def = def.clone();
    // Friendly percentage bonuses to related skills: each invested point
    // moves the target by target.mod * pct / 100 (skills.cc
    // recalc_skills_theory).
    let mut deltas: Vec<(u32, i32)> = Vec::new();
    for (target, pct) in &def.increases {
        let tmod = ps.skill_mods.get(target).copied().unwrap_or(0);
        deltas.push((*target, tmod * *pct as i32 / 100));
    }
    for (target, delta) in deltas {
        let v = (ps.skill_value(target) + delta).clamp(0, SKILL_MAX);
        ps.skills.insert(target, v);
    }
    // Exclusions nullify the opposing skill and refund the points
    // invested in it during this session (skills.cc recalc_skills_theory:
    // skill_points += invest[exclude_si]).
    for ex in &def.excludes {
        let invested = ps.skill_invest.remove(ex).unwrap_or(0);
        ps.skill_points += invested;
        if ps.skill_value(*ex) != 0 {
            ps.skills.insert(*ex, 0);
        }
    }
    Ok(())
}

/// The skill state saved when the skill screen opens (skills.cc
/// do_cmd_skill's skill_values_save/skill_mods_save/skill_points_save).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillSession {
    pub skill_points: i32,
    pub values: HashMap<u32, i32>,
    pub mods: HashMap<u32, i32>,
}

/// Snapshot the skill state and start a fresh investment session
/// (skills.cc do_cmd_skill entry: save values, clear invest[]).
pub fn begin_skill_session(ps: &mut PlayerState) {
    ps.skill_invest.clear();
    ps.skill_session = Some(SkillSession {
        skill_points: ps.skill_points,
        values: ps.skills.clone(),
        mods: ps.skill_mods.clone(),
    });
}

/// Close the skill screen: the session's investments become permanent
/// (skills.cc commits the screen, the 'y' answer).  The UI calls this.
pub fn clear_skill_session(ps: &mut PlayerState) {
    ps.skill_invest.clear();
    ps.skill_session = None;
}

/// Restore the values saved at screen entry (skills.cc do_cmd_skill's
/// 'n' answer).  Returns false when no session is open.
pub fn restore_skill_session(ps: &mut PlayerState) -> bool {
    let Some(s) = ps.skill_session.take() else {
        return false;
    };
    ps.skill_points = s.skill_points;
    ps.skills = s.values;
    ps.skill_mods = s.mods;
    ps.skill_invest.clear();
    true
}

/// Sell back one point (skills.cc decrease_skill); only points invested
/// during the current session can be taken back, and the friendly bonuses
/// are not reversed until the session is committed.
pub fn decrease_skill(ps: &mut PlayerState, gd: &GameData, id: u32) -> Result<(), String> {
    let Some(&modv) = ps.skill_mods.get(&id) else {
        return Err("You cannot train this skill.".into());
    };
    if modv == 0 || ps.skill_value(id) <= 0 {
        return Err("Nothing to take back.".into());
    }
    if ps.skill_invest.get(&id).copied().unwrap_or(0) <= 0 {
        return Err("You can only take back points invested this session.".into());
    }
    ps.skill_points += 1;
    *ps.skill_invest.entry(id).or_insert(0) -= 1;
    let v = (ps.skill_value(id) - modv).max(0);
    ps.skills.insert(id, v);
    let _ = gd;
    Ok(())
}

/// is_known (skills.cc): value/mod != 0, or any descendant is known.
pub fn is_known(ps: &PlayerState, gd: &GameData, id: u32) -> bool {
    if ps.skill_value(id) != 0 || ps.skill_mods.get(&id).copied().unwrap_or(0) != 0 {
        return true;
    }
    gd.skills
        .iter()
        .any(|s| s.father as u32 == id && is_known(ps, gd, s.id))
}

/// Does the skill have a known child (skills.cc has_child)?
pub fn has_child(ps: &PlayerState, gd: &GameData, sel: u32) -> bool {
    gd.skills
        .iter()
        .any(|s| s.father as u32 == sel && is_known(ps, gd, s.id))
}

/// Flatten the skill tree: children in their descriptor `order`
/// (skills.cc get_idx).  `full` recurses into every known branch
/// (init_table with full=true, used by dump_skills); otherwise only
/// expanded (dev) branches are walked (the screen).
fn skill_tree(ps: &PlayerState, gd: &GameData, full: bool) -> Vec<(u32, u32)> {
    fn rec(
        ps: &PlayerState,
        gd: &GameData,
        father: i32,
        depth: u32,
        full: bool,
        out: &mut Vec<(u32, u32)>,
    ) {
        let mut kids: Vec<&crate::data::SkillDef> =
            gd.skills.iter().filter(|s| s.father == father).collect();
        kids.sort_by_key(|s| s.order);
        for s in kids {
            if ps.skill_hidden.contains(&s.id) || !is_known(ps, gd, s.id) {
                continue;
            }
            out.push((s.id, depth));
            if full || ps.skill_dev.contains(&s.id) {
                rec(ps, gd, s.id as i32, depth + 1, full, out);
            }
        }
    }
    let mut out = Vec::new();
    rec(ps, gd, -1, 0, full, &mut out);
    out
}

/// Flatten the skill tree for the screen: children in their descriptor
/// `order` (skills.cc get_idx) and only when their branch is expanded
/// (dev), exactly like skills.cc init_table_aux.
pub fn visible_tree(ps: &PlayerState, gd: &GameData) -> Vec<(u32, u32)> {
    skill_tree(ps, gd, false)
}

/// The whole known tree regardless of dev (skills.cc init_table(full)).
pub fn full_tree(ps: &PlayerState, gd: &GameData) -> Vec<(u32, u32)> {
    skill_tree(ps, gd, true)
}

/// Dump the skill tree (skills.cc dump_skills).  `full` shows every
/// known skill, `selected` indents it like the original's screen.
pub fn dump_skills(ps: &PlayerState, gd: &GameData) -> String {
    let mut out = format!("\nSkills (points left: {})", ps.skill_points);
    for (id, depth) in full_tree(ps, gd) {
        let value = ps.skill_value(id);
        let modv = ps.skill_mods.get(&id).copied().unwrap_or(0);
        if value == 0 && id != SK_MISC && modv == 0 {
            continue;
        }
        let name = gd
            .skills
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.name.as_str())
            .unwrap_or("?");
        let mut buf = String::from("\n");
        for _ in 0..depth {
            buf.push_str("         ");
        }
        if has_child(ps, gd, id) {
            buf.push_str(&format!(" - {}", name));
        } else {
            buf.push_str(&format!(" . {}", name));
        }
        out.push_str(&format!(
            "{:<49}{}{:06.3} [{:05.3}]",
            buf,
            if value < 0 { "-" } else { " " },
            (value.abs() as f64) / SKILL_STEP as f64,
            modv as f64 / 1000.0
        ));
    }
    out.push('\n');
    out
}

/// Print the abilities list (skills.cc dump_abilities), sorted by name.
pub fn dump_abilities(ps: &PlayerState, gd: &GameData) -> String {
    let mut table: Vec<&crate::data::AbilityDef> = gd
        .abilities
        .iter()
        .filter(|a| !a.name.is_empty() && ps.has_ability(a.id))
        .collect();
    table.sort_by(|a, b| compare_abilities(a, b));
    if table.is_empty() {
        return String::new();
    }
    let mut out = String::from("\nAbilities");
    for a in table {
        out.push_str(&format!("\n * {}", a.name));
    }
    out.push('\n');
    out
}

/// Display a skill value the way the original does: "12.800".
pub fn value_str(value: i32) -> String {
    format!("{}.{:03}", value / SKILL_STEP, value % SKILL_STEP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::birth::make_player;
    use crate::data::load_game_data;

    fn human(gd: &GameData, class: usize) -> PlayerState {
        make_player(gd, "Tester".into(), 0, class)
    }

    #[test]
    fn skill_data_loads_and_birth_computes() {
        let gd = load_game_data();
        assert_eq!(gd.skills.len(), 54);
        assert_eq!(gd.abilities.len(), 9);
        assert_eq!(gd.building_actions.len(), 47);
        assert!(gd.building_actions.iter().any(|b| b.action == 23));
        // Warrior: Combat base 2.000, Weaponmastery base 1.000.
        let ps = human(&gd, 0);
        assert_eq!(ps.skill(SK_COMBAT), 2);
        assert_eq!(ps.skill(SK_MASTERY), 1);
        assert!(ps.skill_value(SK_COMBAT) > 0);
        // Level-1 class abilities (p_info C:b: lines).
        assert!(ps.has_ability(AB_EXTRA_BLOW1));
        assert!(ps.has_ability(AB_EXTRA_BLOW2));
        assert!(!ps.has_ability(AB_SPREAD_BLOWS));
        // HIDDEN skills stay hidden; dev marks the ancestors.
        assert!(ps.skill_hidden.contains(&SK_UDUN));
        assert!(ps.skill_dev.contains(&SK_COMBAT));
        // Mage: Magic and Mana trained.
        let mage = human(
            &gd,
            gd.classes.iter().position(|c| c.name == "Mage").unwrap(),
        );
        assert!(mage.skill(SK_MAGIC) >= 1);
        assert!(mage.skill(SK_MANA) >= 1);
        // Tree walking is a racial grant (Ent / Wood-Elf).
        let ent = gd.races.iter().position(|r| r.name == "Ent").unwrap();
        let ent_ps = make_player(&gd, "E".into(), ent, 0);
        assert!(ent_ps.has_ability(AB_TREE_WALKING));
    }

    #[test]
    fn allocation_follows_mod_cap_and_exclusions() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skill_points = 100;
        ps.level = 2;
        let v0 = ps.skill_value(SK_MASTERY);
        let m = ps.skill_mods.get(&SK_MASTERY).copied().unwrap();
        increase_skill(&mut ps, &gd, SK_MASTERY).unwrap();
        assert_eq!(ps.skill_value(SK_MASTERY), v0 + m);
        assert_eq!(ps.skill_points, 99);
        // Friendly bonus: Weaponmastery improves Combat by 50% of
        // Combat's own mod.
        let c0 = ps.skill_value(SK_COMBAT)
            - gd.skills
                .iter()
                .find(|s| s.id == SK_MASTERY)
                .unwrap()
                .increases
                .iter()
                .find(|(t, _)| *t == SK_COMBAT)
                .map(|(_, p)| ps.skill_mods[&SK_COMBAT] * *p as i32 / 100)
                .unwrap();
        assert!(ps.skill_value(SK_COMBAT) > c0);
        // Raise Weaponmastery to the level cap (level+4+1 overage).
        while increase_skill(&mut ps, &gd, SK_MASTERY).is_ok() {}
        assert!(increase_skill(&mut ps, &gd, SK_MASTERY).is_err());
        let cap = allocation_cap(&ps);
        assert!(ps.skill_value(SK_MASTERY) + m >= cap);
        // Exclusions: an Antimagic point nullifies the excluded schools.
        ps.skills.insert(SK_MANA, 2 * SKILL_STEP);
        ps.skill_mods.insert(SK_MANA, 500);
        assert!(ps.skill_value(SK_MANA) != 0);
        increase_skill(&mut ps, &gd, SK_ANTIMAGIC).unwrap();
        assert_eq!(ps.skill_value(SK_MANA), 0);
    }

    #[test]
    fn skill_session_refunds_exclusions_and_gates_decrease() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skill_points = 10;
        ps.skill_mods.insert(SK_MANA, 500);
        ps.skill_mods.insert(SK_ANTIMAGIC, 500);
        increase_skill(&mut ps, &gd, SK_MANA).unwrap();
        increase_skill(&mut ps, &gd, SK_MANA).unwrap();
        assert_eq!(ps.skill_points, 8);
        assert_eq!(ps.skill_value(SK_MANA), 1 * SKILL_STEP);
        // Antimagic excludes Mana: the two session points come back.
        increase_skill(&mut ps, &gd, SK_ANTIMAGIC).unwrap();
        assert_eq!(ps.skill_points, 9);
        assert_eq!(ps.skill_value(SK_MANA), 0);
        // Only this session's points can be taken back.
        assert!(decrease_skill(&mut ps, &gd, SK_MANA).is_err());
        decrease_skill(&mut ps, &gd, SK_ANTIMAGIC).unwrap();
        assert_eq!(ps.skill_points, 10);
        assert!(decrease_skill(&mut ps, &gd, SK_ANTIMAGIC).is_err());
        clear_skill_session(&mut ps);
        let mut ps2 = human(&gd, 0);
        ps2.skill_points = 5;
        ps2.skill_mods.insert(SK_ANTIMAGIC, 500);
        ps2.skills.insert(SK_ANTIMAGIC, 2 * SKILL_STEP);
        assert!(decrease_skill(&mut ps2, &gd, SK_ANTIMAGIC).is_err());
    }

    #[test]
    fn abilities_check_prerequisites() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skill_points = 30;
        let tree = gd
            .abilities
            .iter()
            .find(|a| a.id == AB_TREE_WALKING)
            .unwrap();
        assert!(ability_status(&ps, &gd, tree).is_err());
        assert!(learn_ability(&mut ps, &gd, tree).is_err());
        ps.skills.insert(SK_NATURE, 20 * SKILL_STEP);
        assert!(learn_ability(&mut ps, &gd, tree).is_ok());
        assert!(ps.has_ability(AB_TREE_WALKING));
        // Perfect casting needs Magic 35 (and points).
        let perfect = gd
            .abilities
            .iter()
            .find(|a| a.id == AB_PERFECT_CASTING)
            .unwrap();
        ps.skills.insert(SK_MAGIC, 35 * SKILL_STEP);
        assert!(ability_status(&ps, &gd, perfect).is_ok());
    }

    #[test]
    fn random_gain_offers_four_skills() {
        let gd = load_game_data();
        let ps = human(&gd, 0);
        let mut rng = crate::rng::current();
        let cands = random_gain_candidates(&ps, &gd, &mut rng);
        assert_eq!(cands.len(), LOST_SWORD_NSKILLS);
        let (id, val, mod_inc) = cands[0];
        let mut ps2 = ps.clone();
        apply_random_gain(&mut ps2, &gd, id, val, mod_inc);
        assert!(ps2.skill_value(id) > 0 || val == 0);
        assert!(ps2.skill_mods.get(&id).copied().unwrap_or(0) >= 300);
    }

    #[test]
    fn school_ids_map_music_and_stealth_changes() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skills.insert(SK_STEALTH, 20 * SKILL_STEP);
        assert_eq!(school_skill(&ps, 100), ps.skill(SK_MUSIC));
        assert_eq!(school_skill(&ps, 3), ps.skill(SK_FIRE));
        assert_eq!(ps.skill_scale(SK_STEALTH, 25), 10);
    }

    #[test]
    fn skill_and_ability_names_resolve() {
        let gd = load_game_data();
        // find_skill is exact, find_skill_i is case-insensitive.
        assert_eq!(find_skill(&gd, "Combat"), Some(SK_COMBAT));
        assert_eq!(find_skill(&gd, "combat"), None);
        assert_eq!(find_skill_i(&gd, "cOmBaT"), Some(SK_COMBAT));
        assert_eq!(find_skill_i(&gd, "No Such Skill"), None);
        // find_ability matches the ab_info name exactly.
        let tree = gd
            .abilities
            .iter()
            .find(|a| a.name == "Tree walking")
            .unwrap();
        assert_eq!(find_ability(&gd, "Tree walking"), Some(tree.id));
        assert_eq!(find_ability(&gd, "tree walking"), None);
        // compare_abilities sorts by name.
        let mut list: Vec<&crate::data::AbilityDef> = gd.abilities.iter().collect();
        list.sort_by(|a, b| compare_abilities(a, b));
        assert!(list.windows(2).all(|w| w[0].name <= w[1].name));
    }

    #[test]
    fn melee_style_helpers_follow_the_original_list() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        // Warrior starts with Weaponmastery: one visible style.
        assert_eq!(ps.melee_style, SK_MASTERY);
        assert_eq!(get_melee_skill(&ps), 0);
        assert_eq!(get_melee_name(&ps), "Weapon combat");
        let (n, flags) = get_melee_skills(&ps);
        assert_eq!(n, 1);
        assert_eq!(flags, [true, false, false]);
        // Give Barehand and Bearform, hide Bearform: only two count.
        ps.skills.insert(SK_HAND, 1000);
        ps.skills.insert(SK_BEAR, 1000);
        ps.skill_hidden.insert(SK_BEAR);
        let (n, flags) = get_melee_skills(&ps);
        assert_eq!(n, 2);
        assert_eq!(flags, [true, true, false]);
        // A non-list style falls back to index 0.
        ps.melee_style = SK_SWORD;
        assert_eq!(get_melee_skill(&ps), 0);
        // The chooser cycles to the next visible style (choose_melee).
        ps.melee_style = SK_MASTERY;
        assert_eq!(
            next_melee_style(&ps),
            Some((SK_HAND, "Barehanded combat")),
            "Bearform is hidden, so Barehand is next"
        );
        let mut none = human(&gd, 0);
        none.skills.clear();
        none.skill_mods.clear();
        none.skill_hidden.clear();
        assert_eq!(next_melee_style(&none), None);
        // select_default_melee picks the first developed style.
        ps.melee_style = SK_MASTERY;
        select_default_melee(&mut ps, &gd);
        assert_eq!(ps.melee_style, SK_MASTERY);
        let mut bare = human(&gd, 0);
        bare.skills.clear();
        bare.skill_mods.clear();
        bare.skills.insert(SK_HAND, 1000);
        select_default_melee(&mut bare, &gd);
        assert_eq!(bare.melee_style, SK_HAND);
    }

    #[test]
    fn tree_uses_descriptor_order_and_hides_unknown() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        // Make every root developed and visible; the roots must come out
        // in descriptor order (54 is hidden), not in id order.
        for id in [15u32, 16, 28, 12, 30, 36, 48] {
            ps.skills.insert(id, SKILL_STEP);
        }
        let roots: Vec<u32> = visible_tree(&ps, &gd)
            .into_iter()
            .filter(|(_, d)| *d == 0)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(roots, vec![15, 16, 28, 12, 30, 36, 48]);
        // A child is known through its own value...
        let mut child_only = human(&gd, 0);
        child_only.skills.clear();
        child_only.skill_mods.clear();
        child_only.skills.insert(SK_MASTERY, SKILL_STEP);
        assert!(is_known(&child_only, &gd, SK_COMBAT));
        assert!(has_child(&child_only, &gd, SK_COMBAT));
        // ...or through a known descendant.
        child_only.skills.clear();
        child_only.skill_dev.clear();
        child_only.skills.insert(SK_SWORD, SKILL_STEP);
        assert!(is_known(&child_only, &gd, SK_MASTERY));
        assert!(is_known(&child_only, &gd, SK_COMBAT));
        // The screen only walks expanded branches, the dump walks all.
        let vis: Vec<u32> = visible_tree(&child_only, &gd)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(vis, vec![SK_COMBAT]);
        let full: Vec<u32> = full_tree(&child_only, &gd)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert!(full.contains(&SK_MASTERY) && full.contains(&SK_SWORD));
        // Hidden children are skipped by the tree but still count as
        // children for the dump markers.
        let mut hidden = human(&gd, 0);
        hidden.skills.insert(SK_UDUN, SKILL_STEP);
        assert!(has_child(&hidden, &gd, SK_MAGIC));
        // is_known tests value/mod truthiness (skills.cc:221 `value ||
        // mod`): a zero mod does not make a skill known.
        let mut zero_mod = human(&gd, 0);
        zero_mod.skills.clear();
        zero_mod.skill_mods.clear();
        zero_mod.skill_mods.insert(SK_COMBAT, 0);
        assert!(!is_known(&zero_mod, &gd, SK_COMBAT));
        zero_mod.skill_mods.insert(SK_COMBAT, 500);
        assert!(is_known(&zero_mod, &gd, SK_COMBAT));
    }

    #[test]
    fn dump_skills_and_abilities_match_the_original_format() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skill_points = 3;
        let dump = dump_skills(&ps, &gd);
        assert!(dump.starts_with("\nSkills (points left: 3)"));
        assert!(dump.contains("Combat"));
        assert!(dump.contains("1.000"), "value shown in 1000ths: {}", dump);
        // Abilities dump is sorted and only lists known abilities.
        let ent = gd.races.iter().position(|r| r.name == "Ent").unwrap();
        let ent_ps = make_player(&gd, "E".into(), ent, 0);
        let dump = dump_abilities(&ent_ps, &gd);
        assert!(dump.starts_with("\nAbilities"));
        assert!(dump.contains(" * Tree walking"));
        assert!(dump.ends_with('\n'));
        // An ability-less player gets an empty dump.
        let mut no_abs = human(&gd, 0);
        no_abs.abilities.clear();
        assert!(dump_abilities(&no_abs, &gd).is_empty());
        // Warrior's level-1 grants show up sorted.
        let warrior = dump_abilities(&human(&gd, 0), &gd);
        let extra1 = warrior.find("Extra Max Blow(1)").unwrap();
        let extra2 = warrior.find("Extra Max Blow(2)").unwrap();
        assert!(extra1 < extra2);
    }

    #[test]
    fn augment_skills_applies_value_and_modifier_ops() {
        let m = SkillMod {
            skill: "Combat".into(),
            bop: "+".into(),
            base: 2000,
            mop: "+".into(),
            gain: 800,
        };
        let mut v = 0;
        let mut md = 0;
        augment_skills(&mut v, &mut md, std::slice::from_ref(&m), 0);
        assert_eq!((v, md), (2000, 800));
        // Out of range leaves the values alone.
        augment_skills(&mut v, &mut md, std::slice::from_ref(&m), 1);
        assert_eq!((v, md), (2000, 800));
        let pct = SkillMod {
            skill: "Combat".into(),
            bop: "%".into(),
            base: 150,
            mop: "=".into(),
            gain: 50,
        };
        augment_skills(&mut v, &mut md, std::slice::from_ref(&pct), 0);
        assert_eq!((v, md), (3000, 50));
    }

    #[test]
    fn wrs_returns_a_weighted_permutation() {
        let mut rng = crate::rng::new_seeded_rng(3);
        let out = wrs(&[1, 2, 3, 4], &mut rng);
        let mut sorted = out.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3], "a permutation of the indices");
        assert_eq!(wrs(&[], &mut rng), Vec::<usize>::new());
        assert_eq!(wrs(&[7], &mut rng), vec![0]);
        // The heaviest weight must come first most of the time.
        let mut firsts = [0usize; 4];
        for seed in 0..200 {
            let mut rng = crate::rng::new_seeded_rng(seed);
            firsts[wrs(&[1, 1, 1, 100], &mut rng)[0]] += 1;
        }
        assert!(firsts[3] > 150, "weight 100 dominates: {:?}", firsts);
    }

    #[test]
    fn random_gain_follows_the_mod_formulas() {
        let gd = load_game_data();
        // All random-gain skills get the same mod so every candidate uses
        // the branch under test.
        for modv in [0, 100, 300, 450, 500, 900] {
            let mut ps = human(&gd, 0);
            for s in gd.skills.iter().filter(|s| s.has("RANDOM_GAIN")) {
                ps.skills.insert(s.id, 0);
                ps.skill_mods.insert(s.id, modv);
            }
            let mut rng = crate::rng::new_seeded_rng(9);
            let cands = random_gain_candidates(&ps, &gd, &mut rng);
            assert_eq!(cands.len(), LOST_SWORD_NSKILLS);
            let expected = match modv {
                0 => (SKILL_STEP, 300),
                100 => (SKILL_STEP, 200),
                300 => (300, 100),
                450 => (450, 50),
                500 => (1500, 0),
                _ => (2700, 0),
            };
            for (_, val, mod_inc) in cands {
                assert_eq!((val, mod_inc), expected, "mod {modv}");
            }
        }
        // The value never passes SKILL_MAX.
        let mut ps = human(&gd, 0);
        let id = gd.skills.iter().find(|s| s.has("RANDOM_GAIN")).unwrap().id;
        ps.skills.insert(id, SKILL_MAX);
        ps.skill_mods.insert(id, 600);
        let (val, _) = random_gain_candidates(&ps, &gd, &mut crate::rng::new_seeded_rng(1))
            .into_iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, v, m)| (v, m))
            .unwrap_or((0, 0));
        assert!(val == 0 || ps.skill_value(id) + val <= SKILL_MAX);
        // The oppose scan finds an owned skill excluding the candidate.
        let mut opp = human(&gd, 0);
        opp.skills.insert(SK_ANTIMAGIC, SKILL_STEP);
        assert_eq!(opposing_skill(&opp, &gd, SK_MANA), Some(SK_ANTIMAGIC));
        assert_eq!(opposing_skill(&human(&gd, 0), &gd, SK_MANA), None);
    }

    #[test]
    fn forbid_gloves_and_non_blessed_follow_the_skill_and_god() {
        let gd = load_game_data();
        // A warrior has no arcane skill: no glove restriction.
        let warrior = human(&gd, 0);
        assert!(!crate::game::forbid_gloves(&warrior));
        assert!(!crate::game::forbid_non_blessed(&warrior));
        // A mage with Magic/Mana forbids non-FA gloves.
        let mage = human(
            &gd,
            gd.classes.iter().position(|c| c.name == "Mage").unwrap(),
        );
        assert!(crate::game::forbid_gloves(&mage));
        // Any of the seven listed schools is enough.
        for sk in [SK_SORCERY, SK_MANA, SK_FIRE, SK_AIR, SK_WATER, SK_EARTH, SK_THAUMATURGY] {
            let mut ps = human(&gd, 0);
            ps.skill_mods.clear();
            ps.skills.clear();
            ps.skills.insert(sk, SKILL_STEP);
            assert!(crate::game::forbid_gloves(&ps), "skill {sk}");
        }
        // Only Eru (god 1) forbids edged weapon use.
        let mut eru = human(&gd, 0);
        eru.god = 1;
        assert!(crate::game::forbid_non_blessed(&eru));
        // get_skill/get_skill_scale read the raw 1000ths.
        let mut ps = human(&gd, 0);
        ps.skills.insert(SK_LORE, 12_500);
        assert_eq!(ps.skill(SK_LORE), 12);
        assert_eq!(ps.skill_scale(SK_LORE, 100), 25);
    }

    #[test]
    fn skill_screen_session_commits_or_restores() {
        let gd = load_game_data();
        let mut ps = human(&gd, 0);
        ps.skill_points = 10;
        let base_value = ps.skill_value(SK_MASTERY);
        begin_skill_session(&mut ps);
        increase_skill(&mut ps, &gd, SK_MASTERY).unwrap();
        assert_eq!(ps.skill_points, 9);
        assert_ne!(ps.skill_value(SK_MASTERY), base_value);
        // Declining restores the saved points and values.
        assert!(restore_skill_session(&mut ps));
        assert_eq!(ps.skill_points, 10);
        assert_eq!(ps.skill_value(SK_MASTERY), base_value);
        assert!(ps.skill_invest.is_empty());
        assert!(!restore_skill_session(&mut ps));
        // Accepting keeps them.
        begin_skill_session(&mut ps);
        increase_skill(&mut ps, &gd, SK_MASTERY).unwrap();
        clear_skill_session(&mut ps);
        assert_eq!(ps.skill_points, 9);
        assert_ne!(ps.skill_value(SK_MASTERY), base_value);
        assert!(ps.skill_invest.is_empty());
        assert!(ps.skill_session.is_none());
    }

    #[test]
    fn level_abilities_apply_on_level_up() {
        let gd = load_game_data();
        // Archer gains Ammo creation at level 2 (p_info C:b:2).
        let archer = gd.classes.iter().position(|c| c.name == "Archer").unwrap();
        let mut ps = make_player(&gd, "A".into(), 0, archer);
        assert!(!ps.has_ability(AB_AMMO_CREATION));
        let mut log = MessageLog::default();
        apply_level_abilities(&mut ps, &gd, 2, Some(&mut log));
        assert!(ps.has_ability(AB_AMMO_CREATION));
        assert!(log.lines.iter().any(|l| l.contains("Ammo creation")));
        // Applying the same level twice is idempotent.
        let n = ps.abilities.len();
        apply_level_abilities(&mut ps, &gd, 2, Some(&mut log));
        assert_eq!(ps.abilities.len(), n);
        // Warrior's level-1 abilities were already granted at birth.
        let warrior = human(&gd, 0);
        assert!(warrior.has_ability(AB_EXTRA_BLOW1));
        assert!(warrior.has_ability(AB_EXTRA_BLOW2));
    }
}

/// Weighted random shuffle (skills.cc wrs), the Efraimidis-Spirakis
/// algorithm: for each weight w, draw u in [0,1) and sort descending by
/// u^(1/w).  Returns the indices in selection order.
pub fn wrs(unscaled_weights: &[i32], rng: &mut impl Rng) -> Vec<usize> {
    let n = unscaled_weights.len();
    let scale: i64 = unscaled_weights.iter().map(|w| *w as i64).sum();
    let mut keys_and_indexes: Vec<(f64, usize)> = Vec::with_capacity(n);
    for (i, w) in unscaled_weights.iter().enumerate() {
        let u = rng.gen_range(0..100000) as f64 / 100000.0;
        let weight = if scale != 0 {
            *w as f64 / scale as f64
        } else {
            0.0
        };
        let k = u.powf(1.0 / weight);
        // Negated so an ascending sort is descending by key.
        keys_and_indexes.push((-k, i));
    }
    keys_and_indexes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    keys_and_indexes.into_iter().map(|(_, i)| i).collect()
}

/// The four candidates of a Lost Sword random skill gain (skills.cc
/// do_get_new_skill): RANDOM_GAIN skills weighted by their G: chance,
/// with the value/mod increase computed from the current mod.
pub fn random_gain_candidates(
    ps: &PlayerState,
    gd: &GameData,
    rng: &mut impl Rng,
) -> Vec<(u32, i32, i32)> {
    let available: Vec<u32> = gd
        .skills
        .iter()
        .filter(|s| s.has("RANDOM_GAIN"))
        .map(|s| s.id)
        .collect();
    let weights: Vec<i32> = available
        .iter()
        .map(|id| {
            gd.skills
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.chance as i32)
                .unwrap_or(100)
        })
        .collect();
    let indexes = wrs(&weights, rng);
    let mut picks = Vec::new();
    for &idx in indexes.iter().take(LOST_SWORD_NSKILLS) {
        let id = available[idx];
        let value = ps.skill_value(id);
        let modv = ps.skill_mods.get(&id).copied().unwrap_or(0);
        let (mut val, mod_inc) = if modv == 0 {
            (SKILL_STEP, 300)
        } else if modv < 300 {
            (SKILL_STEP, 300 - modv)
        } else if modv < 500 {
            (modv, (500 - modv).min(100))
        } else {
            (3 * modv, 0)
        };
        if value + val > SKILL_MAX {
            val = SKILL_MAX - value;
        }
        picks.push((id, val, mod_inc));
    }
    picks
}

/// The first owned skill that mutually excludes `id` (skills.cc
/// do_get_new_skill's oppose scan).  The original only asks for a
/// confirmation in that case and never wipes the opponent.
pub fn opposing_skill(ps: &PlayerState, gd: &GameData, id: u32) -> Option<u32> {
    gd.skills
        .iter()
        .filter(|s| ps.skill_value(s.id) != 0)
        .find(|s| s.excludes.contains(&id))
        .map(|s| s.id)
}

/// Apply a chosen random skill gain (skills.cc do_get_new_skill).  The
/// original does not touch the opposing skills; its skill-screen theory
/// only zeroes them when the session invests in the opponent.  The
/// "mutually exclusive ... continue?" prompt lives in the modal caller.
pub fn apply_random_gain(
    ps: &mut PlayerState,
    _gd: &GameData,
    id: u32,
    val: i32,
    mod_inc: i32,
) -> Vec<String> {
    let v = (ps.skill_value(id) + val).clamp(0, SKILL_MAX);
    ps.skills.insert(id, v);
    *ps.skill_mods.entry(id).or_insert(0) += mod_inc;
    Vec::new()
}
