//! Corruption (src/corrupt.cc): permanent mutations of body and soul.
//! The ToME module offers the first fourteen corruptions (the remaining
//! entries are theme-module content); they are gained from the Potion of
//! Corruption, random artifact activations and polymorphing.

use rand::Rng;

use crate::game::{MessageLog, PlayerState, CHA, CON, DEX, INT, STR, WIS};
use crate::item::EquipTotals;

/// CORRUPTIONS_MAX (corrupt.hpp); entries 14+ are theme-only.
pub const CORRUPTIONS_MAX: usize = 34;

/// ToME corruption ids (corrupt.hpp).
pub const CORRUPT_BALROG_AURA: usize = 0;
pub const CORRUPT_BALROG_WINGS: usize = 1;
pub const CORRUPT_BALROG_STRENGTH: usize = 2;
pub const CORRUPT_BALROG_FORM: usize = 3;
pub const CORRUPT_DEMON_SPIRIT: usize = 4;
pub const CORRUPT_DEMON_HIDE: usize = 5;
pub const CORRUPT_DEMON_BREATH: usize = 6;
pub const CORRUPT_DEMON_REALM: usize = 7;
pub const CORRUPT_RANDOM_TELEPORT: usize = 8;
pub const CORRUPT_ANTI_TELEPORT: usize = 9;
pub const CORRUPT_TROLL_BLOOD: usize = 10;
pub const CORRUPT_VAMPIRE_TEETH: usize = 11;
pub const CORRUPT_VAMPIRE_STRENGTH: usize = 12;
pub const CORRUPT_VAMPIRE: usize = 13;

/// Corruption granted powers (powers.hpp PWR_*).
pub const PWR_SPIT_ACID: i32 = 0;
pub const PWR_BR_FIRE: i32 = 1;
pub const PWR_HYPN_GAZE: i32 = 2;
pub const PWR_VTELEPORT: i32 = 4;
pub const PWR_MIND_BLST: i32 = 5;
pub const PWR_VAMPIRISM: i32 = 7;
pub const PWR_BLINK: i32 = 10;
pub const PWR_BALROG: i32 = 61;
pub const POWER_COR_SPACE_TIME: i32 = 64;

pub struct CorruptionDef {
    pub name: &'static str,
    pub get_text: &'static str,
    /// `None` = cannot be removed by any means.
    pub lose_text: Option<&'static str>,
    pub desc: &'static str,
    pub depends: &'static [usize],
    pub opposes: &'static [usize],
    pub power: i32,
}

/// Corruption colors (corrupt.cc `byte color`, TERM_* attrs).
pub const CORRUPTION_COLORS: [u8; 14] = [3, 3, 3, 11, 4, 4, 4, 12, 5, 5, 5, 8, 8, 8];

/// init1.cc conv_color: terminal attr -> color character.
pub const CONV_COLOR: [char; 16] = [
    'd', 'w', 's', 'o', 'r', 'g', 'b', 'u', 'D', 'W', 'v', 'y', 'R', 'G', 'B', 'U',
];

/// The ToME-module corruption table (corrupt.cc corruptions[] 0..13).
pub const CORRUPTIONS: [CorruptionDef; 14] = [
    CorruptionDef {
        name: "Balrog Aura",
        get_text: "A corrupted wall of flames surrounds you.",
        lose_text: Some("The wall of corrupted flames abandons you."),
        desc: "  Surrounds you with a fiery aura\n  But it can burn scrolls when you read them",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Balrog Wings",
        get_text: "Wings of shadow grow in your back.",
        lose_text: Some("The wings in your back fall apart."),
        desc: "  Creates ugly, but working, wings allowing you to fly\n  But it reduces charisma by 4 and dexterity by 2",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Balrog Strength",
        get_text: "Your muscles get unnatural strength.",
        lose_text: Some("Your muscles get weaker again."),
        desc: "  Provides 3 strength and 1 constitution\n  But it reduces charisma by 1 and dexterity by 3",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Balrog Form",
        get_text: "You feel the might of a Balrog inside you.",
        lose_text: Some("The presence of the Balrog seems to abandon you."),
        desc: "  Allows you to turn into a Balrog at will\n  You need Balrog Wings, Balrog Aura and Balrog Strength to activate it",
        depends: &[CORRUPT_BALROG_AURA, CORRUPT_BALROG_WINGS, CORRUPT_BALROG_STRENGTH],
        opposes: &[],
        power: PWR_BALROG,
    },
    CorruptionDef {
        name: "Demon Spirit",
        get_text: "Your spirit opens to corrupted thoughts.",
        lose_text: Some("Your spirit closes again to the corrupted thoughts."),
        desc: "  Increases your intelligence by 1\n  But reduce your charisma by 2",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Demon Hide",
        get_text: "Your skin grows into a thick hide.",
        lose_text: Some("Your skin returns to a natural state."),
        desc: "  Increases your armour class by your level\n  Provides immunity to fire at level 40\n  But reduces speed by your level / 7",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Demon Breath",
        get_text: "Your breath becomes mephitic.",
        lose_text: Some("Your breath is once again normal."),
        desc: "  Provides fire breath\n  But gives a small chance to spoil potions when you quaff them",
        depends: &[],
        opposes: &[],
        power: PWR_BR_FIRE,
    },
    CorruptionDef {
        name: "Demon Realm",
        get_text: "You feel more attuned to the demon realm.",
        lose_text: Some("You lose your attunement to the demon realm."),
        desc: "  Provides access to the demon school skill and the use of demonic equipment\n  You need Demon Spirit, Demon Hide and Demon Breath to activate it",
        depends: &[CORRUPT_DEMON_SPIRIT, CORRUPT_DEMON_HIDE, CORRUPT_DEMON_BREATH],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Random teleportation",
        get_text: "Space seems to fizzle around you.",
        lose_text: Some("Space solidify again around you."),
        desc: "  Randomly teleports you around",
        depends: &[],
        opposes: &[CORRUPT_ANTI_TELEPORT],
        power: -1,
    },
    CorruptionDef {
        name: "Anti-teleportation",
        get_text: "Space continuum freezes around you.",
        lose_text: Some("Space continuum can once more be altered around you."),
        desc: "  Prevents all teleportations, be it of you or monsters",
        depends: &[],
        opposes: &[CORRUPT_RANDOM_TELEPORT],
        power: POWER_COR_SPACE_TIME,
    },
    CorruptionDef {
        name: "Troll Blood",
        get_text: "Your blood thickens, you sense corruption in it.",
        lose_text: Some("Your blood returns to a normal state."),
        desc: "  Troll blood flows in your veins, granting increased regeneration\n  It also enables you to feel the presence of other troll beings\n  But it will make your presence more noticeable and aggravating",
        depends: &[],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Vampiric Teeth",
        get_text: "You grow vampiric teeth!",
        lose_text: None,
        desc: "  Your teeth allow you to drain blood to feed yourself\n  However your stomach now only accepts blood.",
        depends: &[],
        opposes: &[],
        power: PWR_VAMPIRISM,
    },
    CorruptionDef {
        name: "Vampiric Strength",
        get_text: "Your body seems more dead than alive.",
        lose_text: None,
        desc: "  Your body seems somewhat dead\n  In this near undead state it has improved strength, constitution and intelligence\n  But reduced dexterity, wisdom and charisma.",
        depends: &[CORRUPT_VAMPIRE_TEETH],
        opposes: &[],
        power: -1,
    },
    CorruptionDef {
        name: "Vampire",
        get_text: "You die to be reborn in a Vampire form.",
        lose_text: None,
        desc: "  You are a Vampire. As such you resist cold, poison, darkness and nether.\n  Your life is sustained, but you cannot stand the light of the sun.",
        depends: &[CORRUPT_VAMPIRE_STRENGTH],
        opposes: &[],
        power: -1,
    },
];

/// Initialize corruptions (corrupt.cc init_corruptions: nothing needed,
/// the table is static).
pub fn init_corruptions() {}

/// Does the player have this corruption (corrupt.cc player_has_corruption).
pub fn has(ps: &PlayerState, id: usize) -> bool {
    ps.corruptions.get(id).copied().unwrap_or(false)
}

/// Stat changes of a corruption (xtra1.cc calc_corruptions stat_add).
fn stat_deltas(id: usize) -> [i32; 6] {
    let mut d = [0i32; 6];
    match id {
        CORRUPT_BALROG_WINGS => {
            d[CHA] += -4;
            d[DEX] += -2;
        }
        CORRUPT_BALROG_STRENGTH => {
            d[STR] += 3;
            d[CON] += 1;
            d[DEX] += -3;
            d[CHA] += -1;
        }
        CORRUPT_DEMON_SPIRIT => {
            d[INT] += 1;
            d[CHA] += -2;
        }
        CORRUPT_VAMPIRE_STRENGTH => {
            d[STR] += 3;
            d[INT] += 2;
            d[WIS] += -3;
            d[DEX] += -2;
            d[CON] += 1;
            d[CHA] += -4;
        }
        _ => {}
    }
    d
}

/// Gain a corruption (corrupt.cc player_gain_corruption): sets the flag and
/// invokes the gain callback.  The original does *not* print `get_text`
/// here; that announcement belongs to gain_random_corruption, so birth's
/// direct vampire-set calls are silent as in the C++.
pub fn gain(ps: &mut PlayerState, id: usize, log: &mut MessageLog) {
    if has(ps, id) {
        return;
    }
    if ps.corruptions.len() <= id {
        ps.corruptions.resize(id + 1, false);
    }
    ps.corruptions[id] = true;
    let d = stat_deltas(id);
    for i in 0..6 {
        ps.stats[i] += d[i];
    }
    match id {
        CORRUPT_VAMPIRE_STRENGTH => {
            ps.max_hp += 1;
            ps.hp += 1;
            ps.exp += 100;
        }
        CORRUPT_DEMON_REALM => sync_demon_realm(ps),
        _ => {}
    }
    crate::game::calc_sanity(ps);
}

/// The "#####<color>" style announcement of a gained corruption
/// (corrupt.cc gain_random_corruption).
pub fn announce(id: usize, log: &mut MessageLog) {
    if let Some(def) = CORRUPTIONS.get(id) {
        log.add(def.get_text);
    }
}

/// Gain a corruption and run its full acquisition effects: the vampire
/// strength chain re-rolls the character's life (corrupt.cc
/// player_gain_vampire_strength -> do_rebirth).  Callers with the game
/// data and rng handy should use this over `gain`.
pub fn gain_full(
    gd: &crate::data::GameData,
    ps: &mut PlayerState,
    id: usize,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    let had = has(ps, id);
    gain(ps, id, log);
    if !had && id == CORRUPT_VAMPIRE_STRENGTH {
        // The callback rerolls the character first, then announces
        // (corrupt.cc player_gain_vampire_strength).
        crate::game::do_rebirth(ps, gd, log, rng);
        log.add("You feel death slipping inside.");
    }
}

/// Lose one corruption (corrupt.cc player_lose_corruption).
pub fn lose(ps: &mut PlayerState, id: usize, log: &mut MessageLog) {
    if !has(ps, id) {
        return;
    }
    let d = stat_deltas(id);
    for i in 0..6 {
        ps.stats[i] -= d[i];
    }
    ps.corruptions[id] = false;
    if id == CORRUPT_VAMPIRE_STRENGTH {
        ps.max_hp = (ps.max_hp - 1).max(1);
        ps.hp = ps.hp.min(ps.max_hp);
    }
    crate::game::calc_sanity(ps);
    if let Some(def) = CORRUPTIONS.get(id) {
        if let Some(text) = def.lose_text {
            log.add(text);
        }
    }
}

/// Is this corruption allowed at all (corrupt.cc
/// player_allow_corruption): it must belong to the running module, and
/// Vampire Teeth additionally needs a race that cannot change subrace.
pub fn allow_corruption(gd: &crate::data::GameData, ps: &PlayerState, id: usize) -> bool {
    // Only the ToME-module entries exist in this port; the rest of
    // CORRUPTIONS_MAX is theme content.
    if id >= CORRUPTIONS.len() {
        return false;
    }
    // Vampire teeth is special.
    if id == CORRUPT_VAMPIRE_TEETH {
        return crate::game::player_has_flag(gd, ps, "NO_SUBRACE_CHANGE");
    }
    true
}

/// Can the player gain this corruption (corrupt.cc
/// player_can_gain_corruption + test_depend_corrupt).
pub fn can_gain(gd: &crate::data::GameData, ps: &PlayerState, id: usize) -> bool {
    if has(ps, id) {
        return false;
    }
    let Some(def) = CORRUPTIONS.get(id) else {
        return false;
    };
    // The original refuses Troll Blood for Trolls.
    if id == CORRUPT_TROLL_BLOOD && ps.race_name == "Troll" {
        return false;
    }
    if !allow_corruption(gd, ps, id) {
        return false;
    }
    if def.depends.iter().any(|d| !test_depend_held(ps, *d)) {
        return false;
    }
    if def.opposes.iter().any(|o| has(ps, *o)) {
        return false;
    }
    true
}

/// The player holds this corruption and its dependencies are intact
/// (corrupt.cc test_depend_corrupt with can_gain=false).
fn test_depend_held(ps: &PlayerState, id: usize) -> bool {
    if !has(ps, id) {
        return false;
    }
    let Some(def) = CORRUPTIONS.get(id) else {
        return false;
    };
    if id == CORRUPT_TROLL_BLOOD && ps.race_name == "Troll" {
        return false;
    }
    if def.depends.iter().any(|d| !test_depend_held(ps, *d)) {
        return false;
    }
    if def.opposes.iter().any(|o| has(ps, *o)) {
        return false;
    }
    true
}

/// Gain a random corruption (corrupt.cc gain_random_corruption).
/// Returns the id gained, if any.
pub fn gain_random(
    gd: &crate::data::GameData,
    ps: &mut PlayerState,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> Option<usize> {
    let pool: Vec<usize> = (0..CORRUPTIONS.len())
        .filter(|i| can_gain(gd, ps, *i))
        .collect();
    if pool.is_empty() {
        return None;
    }
    let id = pool[rng.gen_range(0..pool.len())];
    gain_full(gd, ps, id, log, rng);
    announce(id, log);
    Some(id)
}

/// Lose a random removable corruption, cascading broken dependencies
/// (corrupt.cc lose_corruption).
pub fn lose_random(
    ps: &mut PlayerState,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> Option<usize> {
    let pool: Vec<usize> = (0..CORRUPTIONS.len())
        .filter(|i| CORRUPTIONS[*i].lose_text.is_some() && test_depend_held(ps, *i))
        .collect();
    let max = pool.len();
    if max == 0 {
        return None;
    }
    let pick = rng.gen_range(0..max);
    let id = pool[pick];
    lose(ps, id, log);
    // Cascade over broken dependents.  The original loop is
    // `for (i = 0; i < max - 1; i++)`, so the last pool entry is never
    // re-checked; that off-by-one is kept bug-for-bug.
    for i in 0..max.saturating_sub(1) {
        let c = pool[i];
        if has(ps, c) != test_depend_held(ps, c) {
            lose(ps, c, log);
        }
    }
    Some(id)
}

/// `corrupt_corrupted` (xtra2.cc:5327): the "corrupted" race acts up --
/// 45% chance to lose a corruption, otherwise gain one.
pub fn corrupt_corrupted(
    gd: &crate::data::GameData,
    ps: &mut PlayerState,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    if crate::rng::magik(45, rng) {
        lose_random(ps, log, rng);
    } else {
        gain_random(gd, ps, log, rng);
    }
}

/// The demon realm corruption reveals the Demonology skill
/// (xtra1.cc calc_corruptions + s_info mod).
pub fn sync_demon_realm(ps: &mut PlayerState) {
    if has(ps, CORRUPT_DEMON_REALM) {
        if ps
            .skill_mods
            .get(&crate::skill::SK_DAEMON)
            .copied()
            .unwrap_or(0)
            == 0
        {
            ps.skill_mods.insert(crate::skill::SK_DAEMON, 1500);
        }
        ps.skill_hidden.remove(&crate::skill::SK_DAEMON);
    }
}

/// The power granted by a corruption, None when it grants none
/// (corrupt.cc get_corruption_power).
pub fn corruption_power(id: usize) -> Option<i32> {
    let def = CORRUPTIONS.get(id)?;
    if def.power >= 0 {
        Some(def.power)
    } else {
        None
    }
}

/// The player's active corruption powers (powers the 'U' menu lists).
pub fn powers(ps: &PlayerState) -> Vec<i32> {
    let mut out = Vec::new();
    for i in 0..CORRUPTIONS.len() {
        if let Some(power) = corruption_power(i) {
            if has(ps, i) && !out.contains(&power) {
                out.push(power);
            }
        }
    }
    out
}

/// Dump the corruption list (corrupt.cc dump_corruptions).  `color`
/// prefixes each name with its `#####` color code, `header` adds the
/// leading "Corruption list:" line.
pub fn dump_corruptions(ps: &PlayerState, color: bool, header: bool) -> String {
    let mut out = String::new();
    let mut header = header;
    for i in 0..CORRUPTIONS_MAX {
        // The original prints the header on the first loop iteration,
        // whether or not any corruption is held.
        if header {
            out.push_str("\nCorruption list:\n\n");
            header = false;
        }
        let Some(def) = CORRUPTIONS.get(i) else {
            continue;
        };
        if !has(ps, i) {
            continue;
        }
        if color {
            let c = CONV_COLOR[(CORRUPTION_COLORS[i] & 0xf) as usize];
            out.push_str(&format!("#####{}{}:\n", c, def.name));
        } else {
            out.push_str(&format!("{}:\n", def.name));
        }
        out.push_str(&format!("{}\n\n", def.desc));
    }
    out
}

/// Does the anti-teleportation corruption currently freeze space
/// (xtra1.cc calc_corruptions resist_continuum)?
pub fn resist_continuum(ps: &PlayerState) -> bool {
    has(ps, CORRUPT_ANTI_TELEPORT) && !ps.corrupt_anti_teleport_stopped
}

/// Fold the continuous corruption effects into the equipment totals
/// (xtra1.cc calc_corruptions).
pub fn apply_corruptions(ps: &PlayerState, t: &mut EquipTotals) {
    let lev = ps.level as i32;
    if has(ps, CORRUPT_BALROG_AURA) {
        t.sh_fire = true;
        t.lite += 1;
    }
    if has(ps, CORRUPT_BALROG_WINGS) {
        t.fly = true;
    }
    if has(ps, CORRUPT_DEMON_HIDE) {
        t.ac += lev;
        t.speed -= lev / 7;
        if ps.level >= 40 {
            t.immunities.insert("FIRE".to_string());
        }
    }
    if has(ps, CORRUPT_ANTI_TELEPORT) && !ps.corrupt_anti_teleport_stopped {
        // The continuum freezes: no teleportation, of the player or of
        // monsters (spells1.cc checks resist_continuum in every teleport
        // function).  Reusing `no_tele` hooks the existing guards.
        t.no_tele = true;
    }
    if has(ps, CORRUPT_TROLL_BLOOD) {
        t.regen = true;
        t.aggravate = true;
        t.esp.insert("TROLL".to_string());
    }
    if has(ps, CORRUPT_VAMPIRE) {
        for e in ["POIS", "NETHER", "COLD", "DARK"] {
            t.resists.insert(e.to_string());
        }
        t.hold_life = true;
        t.lite += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corruption_table_grants_the_original_powers() {
        init_corruptions();
        assert_eq!(CORRUPTIONS_MAX, 34);
        assert_eq!(CORRUPTIONS.len(), 14);
        // get_corruption_power: -1 entries give nothing.
        assert_eq!(corruption_power(CORRUPT_BALROG_FORM), Some(PWR_BALROG));
        assert_eq!(corruption_power(CORRUPT_DEMON_BREATH), Some(PWR_BR_FIRE));
        assert_eq!(corruption_power(CORRUPT_ANTI_TELEPORT), Some(POWER_COR_SPACE_TIME));
        assert_eq!(corruption_power(CORRUPT_BALROG_AURA), None);
        assert_eq!(corruption_power(14), None);
        // The vampire chain cannot be removed by any means.
        assert_eq!(CORRUPTIONS[CORRUPT_VAMPIRE_TEETH].lose_text, None);
        assert_eq!(CORRUPTIONS[CORRUPT_VAMPIRE_STRENGTH].lose_text, None);
        assert_eq!(CORRUPTIONS[CORRUPT_VAMPIRE].lose_text, None);
        // Balrog Form depends on its three components.
        assert_eq!(
            CORRUPTIONS[CORRUPT_BALROG_FORM].depends,
            [CORRUPT_BALROG_AURA, CORRUPT_BALROG_WINGS, CORRUPT_BALROG_STRENGTH].as_slice()
        );
        // Random/anti teleportation oppose each other.
        assert_eq!(
            CORRUPTIONS[CORRUPT_RANDOM_TELEPORT].opposes,
            [CORRUPT_ANTI_TELEPORT].as_slice()
        );
        assert_eq!(
            CORRUPTIONS[CORRUPT_ANTI_TELEPORT].opposes,
            [CORRUPT_RANDOM_TELEPORT].as_slice()
        );
    }

    #[test]
    fn corruption_gain_and_lose_apply_stats_texts_and_powers() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "C".into(), 0, 0);
        let mut log = crate::game::MessageLog::default();
        let str0 = ps.stats[STR];
        let con0 = ps.stats[CON];
        // player_gain_corruption + player_set_corruption + the stat deltas
        // of calc_corruptions (Balrog Strength: +3 STR, +1 CON, -3 DEX,
        // -1 CHA).
        gain(&mut ps, CORRUPT_BALROG_STRENGTH, &mut log);
        assert!(has(&ps, CORRUPT_BALROG_STRENGTH));
        assert_eq!(ps.stats[STR], str0 + 3);
        assert_eq!(ps.stats[CON], con0 + 1);
        // player_gain_corruption is silent (corrupt.cc:769); the
        // get_text announcement lives in gain_random_corruption.
        assert!(!log.lines.iter().any(|l| l == "Your muscles get unnatural strength."));
        announce(CORRUPT_BALROG_STRENGTH, &mut log);
        assert!(log.lines.iter().any(|l| l == "Your muscles get unnatural strength."));
        // Player_lose_corruption reverses the stat deltas.
        lose(&mut ps, CORRUPT_BALROG_STRENGTH, &mut log);
        assert!(!has(&ps, CORRUPT_BALROG_STRENGTH));
        assert_eq!(ps.stats[STR], str0);
        assert_eq!(ps.stats[CON], con0);
        // Gaining twice is a no-op.
        gain(&mut ps, CORRUPT_BALROG_FORM, &mut log);
        let n = log.lines.len();
        gain(&mut ps, CORRUPT_BALROG_FORM, &mut log);
        assert_eq!(log.lines.len(), n);
        assert!(powers(&ps).contains(&PWR_BALROG));
        // Vampire Strength adds 1 max hp and 100 exp, then re-rolls the
        // character (corrupt.cc player_gain_vampire_strength).
        ps.corruptions.clear();
        ps.max_plv = 0;
        let exp0 = ps.exp;
        let mut rng = crate::rng::new_seeded_rng(5);
        gain_full(&gd, &mut ps, CORRUPT_VAMPIRE_STRENGTH, &mut log, &mut rng);
        assert!(has(&ps, CORRUPT_VAMPIRE_STRENGTH));
        assert!(ps.exp >= exp0 + 100);
        assert_eq!(ps.max_plv, ps.level, "do_rebirth reran the life rating");
        assert!(log.lines.iter().any(|l| l == "You feel death slipping inside."));
    }

    #[test]
    fn corruption_allow_and_dependency_rules() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "C".into(), 0, 0);
        let mut log = crate::game::MessageLog::default();
        // player_allow_corruption: theme entries 14+ are not in ToME and
        // vampire teeth need a NO_SUBRACE_CHANGE race.
        assert!(!allow_corruption(&gd, &ps, 14));
        assert!(!allow_corruption(&gd, &ps, CORRUPT_VAMPIRE_TEETH));
        assert!(allow_corruption(&gd, &ps, CORRUPT_BALROG_AURA));
        // test_depend_corrupt: Form needs all three components.
        assert!(!can_gain(&gd, &ps, CORRUPT_BALROG_FORM));
        gain(&mut ps, CORRUPT_BALROG_AURA, &mut log);
        gain(&mut ps, CORRUPT_BALROG_WINGS, &mut log);
        gain(&mut ps, CORRUPT_BALROG_STRENGTH, &mut log);
        assert!(can_gain(&gd, &ps, CORRUPT_BALROG_FORM));
        // ...and cannot be gained twice.
        gain(&mut ps, CORRUPT_BALROG_FORM, &mut log);
        assert!(!can_gain(&gd, &ps, CORRUPT_BALROG_FORM));
        // Opposing corruptions block each other.
        assert!(can_gain(&gd, &ps, CORRUPT_RANDOM_TELEPORT));
        gain(&mut ps, CORRUPT_RANDOM_TELEPORT, &mut log);
        assert!(!can_gain(&gd, &ps, CORRUPT_ANTI_TELEPORT));
        // Detached dependencies fail the held test.
        lose(&mut ps, CORRUPT_BALROG_AURA, &mut log);
        assert!(!test_depend_held(&ps, CORRUPT_BALROG_FORM));
        // Trolls never get Troll Blood.
        let troll = gd.races.iter().position(|r| r.name == "Troll").unwrap();
        let troll_ps = crate::birth::make_player(&gd, "T".into(), troll, 0);
        assert!(!can_gain(&gd, &troll_ps, CORRUPT_TROLL_BLOOD));
        // A NO_SUBRACE_CHANGE race may gain vampire teeth.
        let spectre = gd.racemods.iter().position(|m| m.name.trim() == "Spectre").unwrap();
        let spectre_ps =
            crate::birth::make_player_spec(&gd, "S".into(), 0, spectre, 0, 0);
        assert!(allow_corruption(&gd, &spectre_ps, CORRUPT_VAMPIRE_TEETH));
        assert!(can_gain(&gd, &spectre_ps, CORRUPT_VAMPIRE_TEETH));
    }

    #[test]
    fn random_gain_and_lose_pick_from_valid_pools() {
        let gd = crate::data::load_game_data();
        let mut log = crate::game::MessageLog::default();
        let mut rng = crate::rng::new_seeded_rng(21);
        let mut ps = crate::birth::make_player(&gd, "C".into(), 0, 0);
        // gain_random never offers the vampire chain to a plain human.
        for _ in 0..50 {
            if let Some(id) = gain_random(&gd, &mut ps, &mut log, &mut rng) {
                assert!(id < CORRUPT_VAMPIRE_TEETH);
            }
        }
        // The C++ cascade loop is `for (i = 0; i < max - 1; i++)`, so the
        // last pool entry is never re-checked.  With the whole removable
        // Balrog chain held the pool is [Aura, Wings, Strength, Form]
        // (ascending ids): removing an earlier component orphans Form,
        // which the original happily leaves held.
        let mut saw_orphan_form = false;
        let mut saw_form_removed = false;
        for seed in 0..200u64 {
            let mut ps = crate::birth::make_player(&gd, "C".into(), 0, 0);
            for id in [
                CORRUPT_BALROG_AURA,
                CORRUPT_BALROG_WINGS,
                CORRUPT_BALROG_STRENGTH,
                CORRUPT_BALROG_FORM,
            ] {
                gain(&mut ps, id, &mut log);
            }
            let mut rng = crate::rng::new_seeded_rng(seed);
            let lost = lose_random(&mut ps, &mut log, &mut rng).expect("pool is not empty");
            if lost == CORRUPT_BALROG_FORM {
                saw_form_removed = true;
                for id in [
                    CORRUPT_BALROG_AURA,
                    CORRUPT_BALROG_WINGS,
                    CORRUPT_BALROG_STRENGTH,
                ] {
                    assert!(has(&ps, id), "components survive losing Form");
                }
            } else {
                // Form is the last pool entry and the loop stops at max-1.
                assert!(has(&ps, CORRUPT_BALROG_FORM), "seed {seed}");
                assert!(!test_depend_held(&ps, CORRUPT_BALROG_FORM), "seed {seed}");
                saw_orphan_form = true;
            }
        }
        assert!(saw_orphan_form && saw_form_removed);
        // With nothing left to lose, the pool is empty.
        ps.corruptions.clear();
        assert!(lose_random(&mut ps, &mut log, &mut rng).is_none());
    }

    #[test]
    fn dump_corruptions_lists_held_entries_with_colors() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "C".into(), 0, 0);
        let mut log = crate::game::MessageLog::default();
        // The header is printed even with no corruption held.
        let empty = dump_corruptions(&ps, false, true);
        assert_eq!(empty, "\nCorruption list:\n\n");
        // Without header remains empty.
        assert_eq!(dump_corruptions(&ps, false, false), "");
        gain(&mut ps, CORRUPT_BALROG_AURA, &mut log);
        let plain = dump_corruptions(&ps, false, true);
        assert!(plain.contains("Balrog Aura:\n"));
        assert!(plain.contains("a fiery aura"));
        assert!(!plain.contains("#####"));
        // Color prefix uses conv_color[TERM_ORANGE] = 'o'.
        let colored = dump_corruptions(&ps, true, false);
        assert!(colored.starts_with("#####oBalrog Aura:\n"), "{}", colored);
        // Entrance entries unheld are skipped.
        assert!(!colored.contains("Demon"));
    }

    #[test]
    fn vampire_corruption_chain_sets_flags_name_and_power() {
        let gd = crate::data::load_game_data();
        // A race that can no longer change subrace is the only one that
        // may gain the vampire teeth mid-game.
        let spectre = gd.racemods.iter().position(|m| m.name.trim() == "Spectre").unwrap();
        let mut ps = crate::birth::make_player_spec(&gd, "C".into(), 0, spectre, 0, 0);
        let mut log = crate::game::MessageLog::default();
        // player_gain_vampire_teeth: VAMPIRE/UNDEAD/NO_SUBRACE_CHANGE and
        // the PWR_VAMPIRISM power (corrupt.cc subrace_add_power).
        gain(&mut ps, CORRUPT_VAMPIRE_TEETH, &mut log);
        assert!(crate::game::player_has_flag(&gd, &ps, "VAMPIRE"));
        assert!(crate::game::player_has_flag(&gd, &ps, "UNDEAD"));
        assert!(crate::game::player_has_flag(&gd, &ps, "NO_SUBRACE_CHANGE"));
        assert!(powers(&ps).contains(&PWR_VAMPIRISM));
        // player_gain_vampire: HURT_LITE and the race title.
        gain(&mut ps, CORRUPT_VAMPIRE, &mut log);
        assert!(crate::game::player_has_flag(&gd, &ps, "HURT_LITE"));
        assert!(crate::game::display_race_name(&gd, &ps).contains("Vampire"));
    }

    #[test]
    fn corrupt_corrupted_gains_or_loses_a_corruption() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(
            &gd,
            "C".into(),
            gd.races.iter().position(|r| r.name == "Human").unwrap_or(0),
            gd.classes
                .iter()
                .position(|c| c.name == "Warrior")
                .unwrap_or(0),
        );
        let mut log = crate::game::MessageLog::default();
        let mut rng = crate::rng::new_seeded_rng(7);
        let before: Vec<bool> = (0..CORRUPTIONS.len()).map(|i| has(&ps, i)).collect();
        for _ in 0..200 {
            corrupt_corrupted(&gd, &mut ps, &mut log, &mut rng);
        }
        let after: Vec<bool> = (0..CORRUPTIONS.len()).map(|i| has(&ps, i)).collect();
        assert_ne!(before, after, "the roll must change the corruption set");
    }
}
