//! Spells: the school system (s_info). Spell rows come from spells.ron;
//! each casting class has access to a set of schools and unlocks their
//! spells by character level. Casting requires carrying a spellbook of
//! the matching school (TV_BOOK; the Book of Beginner Cantrips covers
//! low-level spells of every school). Wands/staffs/rods resolve their
//! effects by spell/object name through the same table (see modal.rs).

use rand::Rng;

use crate::data::{self, Dice, GameData, SpellRow};
use crate::game::{MessageLog, PlayerState};
use crate::item::Inventory;

/// School ids the character's class + specialisation develop: the union
/// of their skill modifiers whose skill is a magic school. The original
/// gates casting purely on the school skill value (spells6.cc
/// get_level_school), so whichever schools a class trains can be cast
/// from the matching books. Music (s_info skill 9) uses school id 100.
pub fn class_schools(gd: &GameData, ps: &PlayerState) -> Vec<u32> {
    let mut mods: Vec<&data::SkillMod> = Vec::new();
    if let Some(c) = ps.class(gd) {
        mods.extend(c.skills.iter());
    }
    if let Some(s) = ps.spec(gd) {
        mods.extend(s.skills.iter());
    }
    let mut out: Vec<u32> = Vec::new();
    for m in mods {
        if m.base == 0 && m.gain == 0 {
            continue;
        }
        let Some(sk) = gd.skills.iter().find(|s| s.name == m.skill) else {
            continue;
        };
        let id = if sk.id == crate::skill::SK_MUSIC {
            100
        } else {
            sk.id
        };
        if gd.schools.iter().any(|s| s.id == id) && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// God id -> deity name (tables.cc deity_info; ToME module gods).
pub fn god_name(god: u32) -> &'static str {
    match god {
        1 => "Eru",
        2 => "Manwe",
        3 => "Tulkas",
        4 => "Melkor",
        5 => "Yavanna",
        6 => "Aule",
        7 => "Varda",
        8 => "Ulmo",
        9 => "Mandos",
        _ => "",
    }
}

/// Deity name -> god id (short names used by the spell table).
pub fn god_id(name: &str) -> u32 {
    match name {
        "Eru" => 1,
        "Manwe" => 2,
        "Tulkas" => 3,
        "Melkor" => 4,
        "Yavanna" => 5,
        "Aule" => 6,
        "Varda" => 7,
        "Ulmo" => 8,
        "Mandos" => 9,
        _ => 0,
    }
}

/// Deity full name -> god id (p_info C:g:/C:a:g: lines, tables.cc
/// deity_info). "Nobody" is the atheist choice (GOD_NONE); unknown
/// names return None.
pub fn god_id_full(name: &str) -> Option<u32> {
    match name {
        "Nobody" => Some(0),
        "Eru Iluvatar" => Some(1),
        "Manwe Sulimo" => Some(2),
        "Tulkas" => Some(3),
        "Melkor Bauglir" => Some(4),
        "Yavanna Kementari" => Some(5),
        "Aule the Smith" => Some(6),
        "Varda Elentari" => Some(7),
        "Ulmo" => Some(8),
        "Mandos" => Some(9),
        _ => None,
    }
}

/// The effective school level needed to cast a spell (spells5.cc
/// `spell_type_skill_level`, checked by cmd5.cc `is_ok_spell` through
/// `get_level_school(spell, 50, 0)`): the caster's pooled school skill
/// (Sorcery/god-provided included) must reach the spell's own level.
/// There is no separate character-level gate in the original.
pub fn required_skill(spell: &SpellRow) -> i32 {
    spell.level as i32
}

/// `spell_type_name` (spell_type.cc:294): the registered name.
pub fn spell_name(spell: &SpellRow) -> &str {
    &spell.name
}

/// `spell_type_failure_rate` (spell_type.cc:400): the base failure rate
/// set by `spell_type_set_difficulty`.
pub fn failure_rate(spell: &SpellRow) -> i32 {
    spell.fail
}

/// `spell_type_mana_range` (spell_type.cc:414): the range set by
/// `spell_type_set_mana`, as (min, max).
pub fn mana_range(spell: &SpellRow) -> (i32, i32) {
    (spell.mana, spell.mana_max.max(spell.mana))
}

/// `spell_type_uses_piety_to_cast` (spell_type.cc:334): prayer spells
/// (school 53) consume piety rather than mana.
pub fn uses_piety_to_cast(spell: &SpellRow) -> bool {
    spell.school == 53
}

/// `get_power_name` (spells4.cc:62): which pool a spell drains.
pub fn power_name(spell: &SpellRow) -> &'static str {
    if uses_piety_to_cast(spell) {
        "piety"
    } else {
        "mana"
    }
}

/// `get_power` (spells4.cc:81): the caster's available power for this
/// spell: grace for prayers, mana for everything else.
pub fn power(ps: &PlayerState, spell: &SpellRow) -> i32 {
    if uses_piety_to_cast(spell) {
        ps.grace
    } else {
        ps.mana
    }
}

/// `spell_type_minimum_pval` (spell_type.cc:352): the minimum item pval
/// a music spell needs (the roman numeral of the song).
pub fn minimum_pval(spell: &SpellRow) -> i32 {
    music_song_info(&spell.name)
        .map(|(_, pval, _, _)| pval)
        .unwrap_or(0)
}

/// `spell_type_get_schools` (spell_type.cc:362): every school the spell
/// was registered with, in registration order (primary first, the
/// `spell_type_add_school` call second).
pub fn get_schools(spell: &SpellRow) -> Vec<u32> {
    let second = second_school(&spell.name);
    if second == 0 {
        vec![spell.school]
    } else {
        vec![spell.school, second]
    }
}

/// `spell_type_device_allocation` (spell_type.cc:321): the allocation row
/// for a device tval, if any (tval, rarity, base min/max, max min/max).
pub fn device_allocation(spell: &SpellRow, tval: i32) -> Option<(i32, i32, i32, i32, i32, i32)> {
    spell.alloc.iter().find(|a| a.0 == tval).copied()
}

/// `spell_type_roll_charges` (spell_type.cc:316): `dice_roll` over the
/// device charges string ("N+dM" = N + damroll(1, M), dice.cc).
pub fn roll_device_charges(spell: &SpellRow, rng: &mut impl Rng) -> i32 {
    let charges = spell.charges.as_str();
    if charges.is_empty() {
        return 0;
    }
    // dice_roll = base + damroll(num, sides); the charges strings use the
    // "N+dM" form, other forms go through the item dice parser.
    if let Some((base, die)) = charges.split_once("+d") {
        let base: i32 = base.parse().unwrap_or(0);
        let die: i32 = die.parse().unwrap_or(0);
        return (base + rng.gen_range(1..=die.max(1))).max(0);
    }
    Dice::parse(charges)
        .map(|d| d.roll(rng))
        .unwrap_or(0)
        .max(0)
}

/// God-provided school levels (spells6.cc schools_init + school_god):
/// while worshipping `god`, a school is used at `mul * Prayer / div` even
/// without training.  Only the five base-module gods exist here (Aule,
/// Varda, Ulmo and Mandos are Theme-only).
fn god_provider(school: u32, god: u32) -> Option<(i32, i32)> {
    Some(match (school, god) {
        (2, 1) => (1, 2),  // Mana <- Eru
        (4, 2) => (2, 3),  // Air <- Manwe
        (5, 5) => (1, 2),  // Water <- Yavanna
        (7, 3) => (4, 5),  // Earth <- Tulkas
        (7, 5) => (1, 2),  // Earth <- Yavanna
        (1, 2) => (1, 2),  // Conveyance <- Manwe
        (10, 1) => (2, 3), // Divination <- Eru
        (11, 5) => (1, 6), // Temporal <- Yavanna
        (6, 5) => (1, 2),  // Nature <- Yavanna
        (14, 2) => (1, 3), // Meta <- Manwe
        (51, 1) => (1, 3), // Mind <- Eru
        (51, 4) => (1, 3), // Mind <- Melkor
        _ => return None,
    })
}

/// Sorcery skill substitutes for these schools (spells6.cc
/// sorcery_school_new; Geomancy, Udun, Demon, Device and Music do not).
fn is_sorcery_school(school: u32) -> bool {
    !matches!(school, 13 | 52 | 53 | 55 | 100)
}

/// The school skill value (in 1000ths) after Sorcery substitution and
/// god-provided levels (spells6.cc get_level_school_callback).
pub fn effective_school_value(ps: &PlayerState, school: u32) -> i32 {
    let mut v = ps.skill_value(crate::skill::school_skill_id(school));
    if is_sorcery_school(school) {
        v = v.max(ps.skill_value(crate::skill::SK_SORCERY));
    }
    if let Some((mul, div)) = god_provider(school, ps.god) {
        v = v.max(ps.skill_value(crate::skill::SK_PRAY) * mul / div);
    }
    v
}

/// Udun bonus levels (spells6.cc udun_bonus_levels): (2*lev)/3 extra
/// effective levels, in 1/100 levels (100 = one level).
fn udun_bonus100(ps: &PlayerState) -> i32 {
    (ps.level as i32 * 2 / 3) * 100
}

/// The second school of a two-school spell (spells5.cc
/// spell_type_init_*/spell_type_add_school, in registration order: the
/// init school is primary, the `add_school` call secondary). The
/// original averages the levels of both schools and refuses to cast if
/// either is untrained. 0 = single.
pub fn second_school(name: &str) -> u32 {
    match name {
        "Grow Trees" => 11,    // Nature + Temporal [6, 11]
        "Tracker" => 1,        // Meta + Conveyance [14, 1]
        "Drain" => 2,          // Udun + Mana [55, 2]
        "Wraithform" => 1,     // Udun + Conveyance [55, 1]
        "Flame of Udun" => 3,  // Udun + Fire [55, 3]
        "Fire Golem" => 51,    // Fire + Mind [3, 51]
        "Wings of Winds" => 1, // Air + Conveyance [4, 1]
        "Thunderstorm" => 6,   // Air + Nature [4, 6]
        "Banishment" => 1,     // Temporal + Conveyance [11, 1]
        "Genocide" => 6,       // Udun + Nature [55, 6]
        _ => 0,
    }
}

/// Effective school skill of a spell: the average of both schools for a
/// two-school spell (spells6.cc get_level_school).
pub fn spell_school_skill(ps: &PlayerState, spell: &SpellRow) -> i32 {
    let s2 = second_school(&spell.name);
    if s2 == 0 {
        school_skill(ps, spell.school)
    } else {
        (school_skill(ps, spell.school) + school_skill(ps, s2)) / 2
    }
}

/// Primary school the tome of a spell belongs to (spells4.cc
/// init_school_books: multi-school spells sit in one specific book, not
/// necessarily the first school's). Kept for the tests and display use;
/// book access itself goes through `book_spells`.
#[allow(dead_code)]
pub fn spell_book_school(spell: &SpellRow) -> u32 {
    match spell.name.as_str() {
        "Grow Trees" => 6,                              // TOME_NATURE
        "Tracker" => 14,                                // TOME_META
        "Drain" | "Wraithform" | "Flame of Udun" => 55, // TOME_HELLFLAME
        "Fire Golem" => 3,                              // TOME_FIRE
        "Wings of Winds" => 4,                          // TOME_WINDS
        "Banishment" => 11,                             // TOME_TIME
        "Genocide" => 55,                               // TOME_HELLFLAME
        _ => spell.school,
    }
}

/// A Bard song's requirements (spells4.cc BOOK_DRUMS/HARPS/HORNS,
/// spells5.cc spell_type_init_music): (instrument sval, minimum pval,
/// castable while blind, lasting). The roman numeral of the song name is
/// the minimum instrument pval; lasting songs take effect every 10 turns
/// while sung (spells3.cc music_* and dungeon.cc check_music).
pub fn music_song_info(name: &str) -> Option<(i32, i32, bool, bool)> {
    Some(match name {
        "Stop singing(I)" => (0, 1, true, false), // any instrument
        "Holding Pattern(I)" => (58, 1, true, true),
        "Illusion Pattern(II)" => (58, 2, true, true),
        "Stun Pattern(IV)" => (58, 4, true, true),
        "Song of the Sun(I)" => (59, 1, true, true),
        "Flow of Life(II)" => (59, 2, false, true),
        "Heroic Ballad(II)" => (59, 2, false, true),
        "Hobbit Melodies(III)" => (59, 3, false, true),
        "Clairaudience(IV)" => (59, 4, false, true),
        "Blow(I)" => (60, 1, false, false),
        "Gush of Wind(II)" => (60, 2, false, false),
        "Horns of Ylmir(III)" => (60, 3, false, false),
        "Ambarkanta(IV)" => (60, 4, false, false),
        _ => return None,
    })
}

/// Spells castable while blind (spells5.cc
/// spell_type_set_castable_while_blind): Disperse Magic, the Geomancy
/// spells and Eru's See the Music; songs use their own flags.
pub fn castable_while_blind(spell: &SpellRow) -> bool {
    if spell.school == 100 {
        return music_song_info(&spell.name)
            .map(|(_, _, blind_ok, _)| blind_ok)
            .unwrap_or(false);
    }
    matches!(
        spell.name.as_str(),
        "Disperse Magic"
            | "See the Music"
            | "Call the Elements"
            | "Channel Elements"
            | "Elemental Wave"
            | "Vaporize"
            | "Geolysis"
            | "Dripping Tread"
            | "Grow Barrier"
    )
}

/// Spells castable while confused (spells5.cc
/// spell_type_set_castable_while_confused): only Disperse Magic and the
/// Geomancy spells (which have their own casting path here).
pub fn castable_while_confused(spell: &SpellRow) -> bool {
    matches!(
        spell.name.as_str(),
        "Disperse Magic"
            | "Call the Elements"
            | "Channel Elements"
            | "Elemental Wave"
            | "Vaporize"
            | "Geolysis"
            | "Dripping Tread"
            | "Grow Barrier"
    )
}

/// Inertia data of a spell that can be fed to Inertia Control
/// (spells5.cc spell_type_set_inertia; (difficulty, delay)). Spells
/// without it cannot be controlled.
pub fn inertia_info(name: &str) -> Option<(i32, i32)> {
    Some(match name {
        "Globe of Light" => (1, 40),
        "Fiery Shield" => (2, 15),
        "Remove Curses" => (1, 10),
        "Elemental Shield" => (2, 25),
        "Disruption Shield" => (9, 10),
        "Tidal Wave" => (4, 100),
        "Ice Storm" => (3, 40),
        "Ent's Potion" => (1, 30),
        "Vapor" => (1, 30),
        "Wings of Winds" => (1, 10),
        "Invisibility" => (1, 30),
        "Poison Blood" => (1, 35),
        "Thunderstorm" => (2, 15),
        "Stone Skin" => (2, 50),
        "Shake" => (2, 50),
        "Phase Door" => (1, 5),
        "Teleportation" => (1, 10),
        "Probability Travel" => (6, 40),
        "Grow Trees" => (5, 50),
        "Recovery" => (2, 100),
        "Regeneration" => (4, 40),
        "Vision" => (2, 200),
        "Sense Hidden" => (1, 10),
        "Reveal Ways" => (1, 10),
        "Sense Monsters" => (1, 10),
        "Essence of Speed" => (5, 20),
        "Banishment" => (5, 50),
        "Disperse Magic" => (1, 5),
        "Armor of Fear" => (2, 20),
        "Wraithform" => (4, 30),
        "Flame of Udun" => (7, 15),
        _ => return None,
    })
}

/// Spells the character can cast: the class school list (the port's book
/// gate) plus the original cmd5.cc `is_ok_spell` school-level test; god
/// spells also require worshipping that god. Two-school spells need both
/// schools trained (spells6.cc get_level_school "na").
pub fn known_spells<'a>(gd: &'a GameData, ps: &PlayerState) -> Vec<&'a SpellRow> {
    let schools = class_schools(gd, ps);
    gd.spells
        .iter()
        .filter(|s| {
            let s2 = second_school(&s.name);
            // The class list is the port's book gate; the original also
            // grants access through any live school value (god-provided
            // levels, Sorcery substitution, p_info skill modifiers).
            let schools_ok = if s2 == 0 {
                schools.contains(&s.school) || effective_school_value(ps, s.school) > 0
            } else {
                effective_school_value(ps, s.school) > 0 && effective_school_value(ps, s2) > 0
            };
            let castable = spell_usable_no_inv(ps, s);
            if !s.god.is_empty() {
                // God-granted Prayer spells: worship + school level.
                return god_id(&s.god) == ps.god && castable;
            }
            schools_ok && castable
        })
        .collect()
}

/// The Library quest's bookable spell list (initialize_bookable_spells,
/// src/q_library.cc:44-101). The original pushes 43 hardcoded spell ids;
/// this is the same order with this port's names.
pub const BOOKABLE_SPELLS: [&str; 43] = [
    "Manathrust",
    "Remove Curses",
    "Globe of Light",
    "Fire Golem",
    "Fireflash",
    "Firewall",
    "Geyser",
    "Vapor",
    "Ent's Potion",
    "Noxious Cloud",
    "Poison Blood",
    "Stone Skin",
    "Dig",
    "Recharge",
    "Disperse Magic",
    "Phase Door",
    "Teleport",
    "Sense Monsters",
    "Sense Hidden",
    "Reveal Ways",
    "Vision",
    "Magelock",
    "Slow Monster",
    "Essence of Speed",
    "Charm",
    "Confuse",
    "Armor of Fear",
    "Stun",
    "Grow Trees",
    "Healing",
    "Recovery",
    "See the Music",
    "Listen to the Music",
    "Manwe's Blessing",
    "Wind Shield",
    "Charm Animal",
    "Grow Grass",
    "Tree Roots",
    "Divine Aim",
    "Whirlwind",
    "Curse",
    "Corpse Explosion",
    "Drain Life",
];

/// The spells a Library reward book may teach, in list order.
pub fn bookable_spells<'a>(gd: &'a GameData) -> Vec<&'a SpellRow> {
    BOOKABLE_SPELLS
        .iter()
        .filter_map(|name| gd.spells.iter().find(|s| s.name == *name))
        .collect()
}

/// known_spells plus every spell contained in a carried custom tome
/// (q_library's Tome of <player>); these are castable regardless of
/// class or school.
pub fn known_spells_with_books<'a>(
    gd: &'a GameData,
    ps: &PlayerState,
    inv: &Inventory,
) -> Vec<&'a SpellRow> {
    let mut out = known_spells(gd, ps);
    let carried = inv.pack.iter().chain(inv.equip.iter().flatten());
    for book in carried {
        for name in &book.spells {
            if out.iter().any(|s| s.name == *name) {
                continue;
            }
            if let Some(s) = gd.spells.iter().find(|s| s.name == *name) {
                out.push(s);
            }
        }
    }
    out
}

/// The exact spell list of every base school book (spells4.cc
/// init_school_books; the lists are shown in that push order). Svals not
/// listed here (the Library's Tome of <player>, random books) resolve
/// through their inscribed `Item::spells` instead.
pub fn book_spells(sval: i32) -> Option<&'static [&'static str]> {
    Some(match sval {
        0 => &[
            "Manathrust",
            "Remove Curses",
            "Elemental Shield",
            "Disruption Shield",
        ],
        1 => &[
            "Globe of Light",
            "Fire Golem",
            "Fireflash",
            "Firewall",
            "Fiery Shield",
        ],
        2 => &[
            "Noxious Cloud",
            "Poison Blood",
            "Invisibility",
            "Sterilize",
            "Wings of Winds",
            "Thunderstorm",
        ],
        3 => &["Stone Skin", "Dig", "Stone Prison", "Shake", "Strike"],
        4 => &["Geyser", "Vapor", "Ent's Potion", "Tidal Wave", "Ice Storm"],
        5 => &[
            "Phase Door",
            "Teleport",
            "Teleport Away",
            "Word of Recall",
            "Probability Travel",
        ],
        6 => &[
            "Grow Trees",
            "Healing",
            "Recovery",
            "Regeneration",
            "Summon Animal",
        ],
        7 => &["Sense Monsters", "Sense Hidden", "Reveal Ways", "Vision"],
        8 => &["Magelock", "Slow Monster", "Essence of Speed", "Banishment"],
        9 => &[
            "Recharge",
            "Disperse Magic",
            "Spellbinder",
            "Tracker",
            "Inertia Control",
        ],
        10 => &["Charm", "Confuse", "Armor of Fear", "Stun"],
        11 => &["Drain", "Genocide", "Wraithform", "Flame of Udun"],
        20 => &["See the Music", "Listen to the Music", "Lay of Protection"],
        21 => &["Manwe's Blessing", "Wind Shield", "Manwe's Call", "Avatar"],
        22 => &["Divine Aim", "Whirlwind", "Wave of Power"],
        23 => &["Curse", "Corpse Explosion", "Mind Steal"],
        24 => &[
            "Charm Animal",
            "Grow Grass",
            "Tree Roots",
            "Water Bite",
            "Uproot",
        ],
        50 => &[
            "Manathrust",
            "Globe of Light",
            "Ent's Potion",
            "Phase Door",
            "Sense Monsters",
            "Sense Hidden",
        ],
        51 => &["Phase Door", "Teleport", "Teleport Away"],
        52 => &["Fire Golem", "Summon Animal"],
        _ => return None,
    })
}

/// The three daemon books (TV_DAEMON_BOOK svals 55/56/57) and the Demon
/// school spells each contains (src/spells4.cc BOOK_DEMON_SWORD/SHIELD/
/// HELM). They must be worn to cast from (TR_WIELD_CAST).
pub fn daemon_book_spells(sval: i32) -> Option<&'static [&'static str]> {
    Some(match sval {
        55 => &["Demon Blade", "Demon Madness", "Demon Field"],
        56 => &["Doom Shield", "Demon Cloak", "Unholy Word"],
        57 => &["Summon Demon", "Discharge Minion", "Control Demon"],
        _ => return None,
    })
}

/// Does the character hold a "book" covering this spell?
/// The Book of Beginner Cantrips (sval 50) covers level <= 2 spells;
/// Bard songs (Music) are sung through a wielded musical instrument
/// (instruments are school books in the original, src/cmd5.cc);
/// god-granted spells need no book either (granted directly).
pub fn has_book_for(inv: &Inventory, gd: &GameData, spell: &SpellRow) -> bool {
    if !spell.god.is_empty() {
        return true;
    }
    if spell.school == 100 {
        // The song must be in the wielded instrument's book (sval) and the
        // instrument's pval must reach the song's numeral
        // (cmd5.cc is_ok_spell).
        let Some((sval, min_pval, _, _)) = music_song_info(&spell.name) else {
            return false;
        };
        return inv.equip.iter().flatten().any(|w| {
            let o = &gd.objects[w.def];
            o.tval == data::TV_INSTRUMENT && (sval == 0 || o.sval == sval) && w.pval >= min_pval
        });
    }
    // Spell containers (SPELL_CONTAIN/WIELD_CAST artifacts) cast their
    // inscribed spell while wielded/equipped (cmd5.cc get_school_spell).
    if inv
        .equip
        .iter()
        .flatten()
        .any(|it| it.spells.iter().any(|n| n == &spell.name))
    {
        return true;
    }
    // A daemon book is a Demon school book, but only while worn
    // (cmd5.cc is_school_book + TR_WIELD_CAST: the three books occupy
    // the weapon, shield and head slots, object1.cc wield_slot).
    if inv.equip.iter().flatten().any(|it| {
        let o = &gd.objects[it.def];
        o.tval == data::TV_DAEMON_BOOK
            && daemon_book_spells(o.sval)
                .map(|names| names.contains(&spell.name.as_str()))
                .unwrap_or(false)
    }) {
        return true;
    }
    inv.pack.iter().any(|it| {
        // A custom tome or spell container inscribed with the spell. A
        // container with WIELD_CAST must be equipped (checked above).
        if it.spells.iter().any(|n| n == &spell.name) {
            return !crate::item::item_flags(gd, it)
                .iter()
                .any(|f| *f == "WIELD_CAST");
        }
        let o = &gd.objects[it.def];
        if o.tval != data::TV_BOOK {
            return false;
        }
        book_spells(o.sval)
            .map(|names| names.contains(&spell.name.as_str()))
            .unwrap_or(false)
    })
}

/// Spells the character can actually cast right now (book in pack).
/// (The cast modal lists known spells with a "(no book)" marker instead;
/// kept for tests and as the canonical castable filter.)
#[allow(dead_code)]
pub fn castable_spells<'a>(
    gd: &'a GameData,
    ps: &PlayerState,
    inv: &Inventory,
) -> Vec<&'a SpellRow> {
    known_spells(gd, ps)
        .into_iter()
        .filter(|s| has_book_for(inv, gd, s))
        .collect()
}

/// Effective skill level in a spell's school: the trained value, Sorcery
/// substitution and god-provided levels, plus the Udun bonus
/// (spells6.cc get_level_school).
pub fn school_skill(ps: &PlayerState, school: u32) -> i32 {
    let mut v = effective_school_value(ps, school) / crate::skill::SKILL_STEP;
    if school == 55 {
        v += ps.level as i32 * 2 / 3;
    }
    v
}

/// spell_type_random_type (spell_type.cc, set by the
/// `spell_type_init_*` functions): the skill whose random books a spell
/// may appear in. Arcane school spells use Magic, priest/god spells
/// Spirituality, songs Music; devices, geomancy, demonology and the
/// port-only helper rows are NO_RANDOM (0). The value is carried in the
/// spell row (`random_type`) straight from the original registration, so
/// the pool cannot drift from spells5.cc.
pub fn random_type(spell: &SpellRow) -> u32 {
    spell.random_type
}

/// get_random_spell (spells5.cc:58): a random spell of the given
/// `random_type` whose original skill level fits the dungeon level.
pub fn random_spell<'a>(
    gd: &'a GameData,
    random_type_wanted: u32,
    level: i32,
    rng: &mut impl Rng,
) -> Option<&'a SpellRow> {
    for _ in 0..1000 {
        let row = &gd.spells[rng.gen_range(0..gd.spells.len())];
        // NO_RANDOM rows never match: the original stores -1 there, which
        // no caller-supplied skill id can equal (the port's 0 stands in
        // for it, so it must be excluded explicitly).
        let rt = random_type(row);
        if rt != 0
            && rt == random_type_wanted
            && rng.gen_range(0..(row.level as i32 * 3).max(1)) < level
        {
            return Some(row);
        }
    }
    None
}

/// Scaled school skill: `scale * value / SKILL_MAX` (0..scale).
pub fn school_scale(ps: &PlayerState, school: u32, scale: i32) -> i32 {
    scale * (school_skill(ps, school) * crate::skill::SKILL_STEP) / crate::skill::SKILL_MAX
}

/// spell_type_casting_stat (spell_type.cc): music uses CHR, god/priest
/// spells WIS, everything else INT.
pub fn casting_stat(spell: &SpellRow) -> usize {
    if spell.school == 100 {
        crate::game::CHA
    } else if !spell.god.is_empty() {
        crate::game::WIS
    } else {
        crate::game::INT
    }
}

/// tables.cc adj_mag_stat (INT/WIS): effective-level bonus by stat index.
fn adj_mag_stat(stat: i32) -> i32 {
    const TABLE: [i32; 38] = [
        0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, // 3-17
        3, 3, 3, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];
    TABLE[crate::game::stat_index(stat)]
}

/// tables.cc adj_mag_fail (INT/WIS): the minimum failure percentage.
fn adj_mag_fail(stat: i32) -> i32 {
    const TABLE: [i32; 38] = [
        99, 99, 99, 99, 99, 50, 30, 20, 15, 12, 11, 10, 9, 8, 7, // 3-17
        6, 6, 5, 5, 5, 4, 4, 4, 4, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0, 0, 0,
    ];
    TABLE[crate::game::stat_index(stat)]
}

/// cmd7.cc `clamp_failure_chance`: apply the minimum failure rate, the
/// stun penalty (25 at stun > 50, else 15) and the 95% cap.
fn clamp_failure_chance(mut chance: i32, minfail: i32, stun: i32) -> i32 {
    let minfail = minfail.max(0);
    if chance < minfail {
        chance = minfail;
    }
    if stun > 50 {
        chance += 25;
    } else if stun > 0 {
        chance += 15;
    }
    chance.min(95)
}

/// Chance (percent) that casting fails (lua_bind.cc `spell_chance_school`):
/// the row's original failure rate reduced by the effective spell level
/// and the casting-stat adjustment, plus the not-enough-power penalty
/// (mana or, for prayers, grace: `get_power`), the minimum-failure floor
/// and the stun penalty. `fast_cast` is a port-only concept with no C++
/// counterpart and is ignored.
pub fn fail_chance(ps: &PlayerState, spell: &SpellRow, _fast_cast: bool) -> i32 {
    let stat = ps.stats[casting_stat(spell)];
    let Some(value) = spell_school_value(ps, spell) else {
        return 95;
    };
    let level = level_s_from(
        value / 10,
        spell.level as i32,
        50,
        1,
        spell_bonus100(ps, spell),
    );
    let mana = spell_mana_no_inv(ps, spell);
    let mut chance = spell.fail - 3 * (level - 1) - 3 * (adj_mag_stat(stat) - 1);
    if chance < 0 {
        chance = 0;
    }
    let have = power(ps, spell);
    if mana > have {
        chance += 15 * (mana - have);
    }
    // Extract the minimum failure rate (Perfect Casting skips the floor).
    let mut minfail = adj_mag_fail(stat);
    if !ps.has_ability(crate::skill::AB_PERFECT_CASTING) && minfail < 5 {
        minfail = 5;
    }
    // Priest prayer penalty for "edged" weapons: only Eru's priests shun
    // them (lua_bind.cc:158 `forbid_non_blessed() && icky_wield`).
    if crate::game::forbid_non_blessed(ps) && ps.icky_wield {
        chance += 25;
    }
    clamp_failure_chance(chance, minfail, ps.stun)
}

/// `spell_chance_school` (lua_bind.cc:124) with the worn +spell-power
/// (to_s) bonus folded into the effective level and the mana cost.
pub fn fail_chance_with_inv(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    spell: &SpellRow,
    _fast_cast: bool,
) -> i32 {
    let stat = ps.stats[casting_stat(spell)];
    let Some(value) = spell_school_value(ps, spell) else {
        return 95;
    };
    let to_s = inv.totals_for(gd, ps).spell_power;
    let level = level_s_from(
        value / 10,
        spell.level as i32,
        50,
        1,
        to_s * 100 + spell_bonus100(ps, spell),
    );
    let mana = get_level_school(
        ps,
        inv,
        gd,
        spell,
        spell.mana_max.max(spell.mana),
        spell.mana.min(spell.mana_max.max(spell.mana)),
    )
    .0;
    let mut chance = spell.fail - 3 * (level - 1) - 3 * (adj_mag_stat(stat) - 1);
    if chance < 0 {
        chance = 0;
    }
    let have = power(ps, spell);
    if mana > have {
        chance += 15 * (mana - have);
    }
    let mut minfail = adj_mag_fail(stat);
    if !ps.has_ability(crate::skill::AB_PERFECT_CASTING) && minfail < 5 {
        minfail = 5;
    }
    if crate::game::forbid_non_blessed(ps) && ps.icky_wield {
        chance += 25;
    }
    clamp_failure_chance(chance, minfail, ps.stun)
}

pub fn roll_dice(expr: &str, rng: &mut impl Rng) -> i32 {
    Dice::parse(expr)
        .map(|d| d.roll(rng))
        .unwrap_or_else(|| expr.trim().parse().unwrap_or(1))
}

/// lua_get_level (lua_bind.cc:36): the caster's effective level in a
/// spell from a school-skill value in 1/100 levels, the spell's own
/// level and a bonus (also in 1/100 levels). Clamped to `[min, ...]`.
fn level_s_from(lvl100: i32, spell_level: i32, max: i32, min: i32, bonus100: i32) -> i32 {
    let mut tmp = lvl100 - (spell_level - 1) * 100;
    if tmp >= 100 {
        tmp += bonus100;
    }
    ((tmp * max / 5000).max(min)).max(0)
}

/// The pooled effective school value of a spell (in 1/1000 skill units):
/// the single school, or the average of both for a two-school spell.
/// `None` when any of its schools is untrained (spells6.cc
/// get_level_school's "na": all schools must be non-zero).
fn spell_school_value(ps: &PlayerState, spell: &SpellRow) -> Option<i32> {
    let v1 = effective_school_value(ps, spell.school);
    let s2 = second_school(&spell.name);
    if v1 <= 0 {
        return None;
    }
    if s2 == 0 {
        return Some(v1);
    }
    let v2 = effective_school_value(ps, s2);
    if v2 <= 0 {
        return None;
    }
    Some((v1 + v2) / 2)
}

/// The Spell-power / worn +spell-power (to_s) / Udun bonus in 1/100
/// level units (spells6.cc get_level_school_callback). The Spell-power
/// skill only applies when every school of the spell allows it.
fn spell_bonus100(ps: &PlayerState, spell: &SpellRow) -> i32 {
    let s2 = second_school(&spell.name);
    let allow_spell_power = (is_sorcery_school(spell.school) || !spell.god.is_empty())
        && (s2 == 0 || is_sorcery_school(s2));
    let mut bonus = 0;
    if allow_spell_power {
        bonus += ps.skill_scale(crate::skill::SK_SPELL, 20) * 100;
    }
    if spell.school == 55 || s2 == 55 {
        bonus += udun_bonus100(ps);
    }
    bonus
}

/// get_level_school (spells6.cc:244 → lua_bind.cc get_level_school_1):
/// the caster's effective level in `spell` with an explicit max/min
/// clamp. Returns (level, na); na means a school of the spell is
/// untrained and the level is the minimum.
pub fn get_level_school(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    spell: &SpellRow,
    max: i32,
    min: i32,
) -> (i32, bool) {
    let Some(value) = spell_school_value(ps, spell) else {
        return (min, true);
    };
    let to_s = inv.totals_for(gd, ps).spell_power;
    let bonus = to_s * 100 + spell_bonus100(ps, spell);
    (
        level_s_from(value / 10, spell.level as i32, max, min, bonus),
        false,
    )
}

/// get_level_s (lua_bind.cc:207): effective spell level, minimum 1.
pub fn get_level_s(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    spell: &SpellRow,
    max: i32,
) -> i32 {
    get_level_school(ps, inv, gd, spell, max, 1).0
}

/// The same without an inventory (the to_s equipment bonus is zero).
fn level_s_no_inv(ps: &PlayerState, spell: &SpellRow, max: i32, min: i32) -> i32 {
    match spell_school_value(ps, spell) {
        None => min,
        Some(value) => level_s_from(
            value / 10,
            spell.level as i32,
            max,
            min,
            spell_bonus100(ps, spell),
        ),
    }
}

/// cmd5.cc `is_ok_spell`: the spell is castable when the pooled school
/// level reaches its own level (get_level_school with min 0 is non-zero)
/// and both schools are trained.
pub fn spell_usable_no_inv(ps: &PlayerState, spell: &SpellRow) -> bool {
    spell_school_value(ps, spell).is_some()
        && level_s_no_inv(ps, spell, 50, 0) != 0
}

/// lua_bind.cc `get_mana`: the mana cost scales between the row's
/// original min and max with the caster's effective level in the spell.
pub fn spell_mana_no_inv(ps: &PlayerState, spell: &SpellRow) -> i32 {
    let max = spell.mana_max.max(spell.mana);
    level_s_no_inv(ps, spell, max, spell.mana.min(max))
}

/// `get_mana` with the worn +spell-power (to_s) bonus folded in.
pub fn spell_mana(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    spell: &SpellRow,
) -> i32 {
    let max = spell.mana_max.max(spell.mana);
    get_level_school(ps, inv, gd, spell, max, spell.mana.min(max)).0
}

/// get_level_device (lua_bind.cc:59-95): the effective level of a device
/// casting, from the Magic-Device skill plus the stick's pval3 bonus
/// level, capped by the stick's pval3 max level. No spell-power bonus.
pub fn device_level_s(ps: &PlayerState, spell: &SpellRow, pval3: i32, max: i32) -> i32 {
    let bonus = pval3 & 0xFFFF;
    let max_lvl = (pval3 >> 16) & 0xFFFF;
    let mut lvl = ps.skill_value(crate::skill::SK_DEVICE) + bonus * 1000;
    if max_lvl > 0 && lvl - (spell.level as i32 + 1) * 1000 >= max_lvl * 1000 {
        lvl = (max_lvl + spell.level as i32 - 1) * 1000;
    }
    level_s_from(lvl / 10, spell.level as i32, max, 1, 0)
}

/// Spell power bonus in percent. The original get_level_school folds the
/// caster's school skill, the Spell-power skill and the equipment SPELL
/// pval into the spell's effective level; the curated fixed-dice table
/// scales the effect by this percentage instead.
pub fn power_percent(ps: &PlayerState, spell: &SpellRow, item_power: i32) -> i32 {
    item_power + 5 * spell_school_skill(ps, spell) + 2 * ps.skill(crate::skill::SK_SPELL)
}

/// A dice expression rolled and scaled by `power_percent`.
pub fn powered_roll(
    ps: &PlayerState,
    spell: &SpellRow,
    item_power: i32,
    rng: &mut impl Rng,
) -> i32 {
    let base = roll_dice(&spell.arg, rng).max(1);
    base * (100 + power_percent(ps, spell, item_power)).max(100) / 100
}

/// Spend the mana for a casting attempt: the complete original `get_mana`
/// cost (scaled by the caster's school level) is paid whether the spell
/// succeeded or failed (spells4.cc lua_cast_school_spell calls
/// adjust_power(-get_mana(s)) in both branches).
pub fn spend_mana(ps: &mut PlayerState, spell: &SpellRow) {
    let cost = spell_mana_no_inv(ps, spell);
    ps.mana = (ps.mana - cost).max(0);
}

pub fn casting_failed(
    ps: &PlayerState,
    spell: &SpellRow,
    fast_cast: bool,
    rng: &mut impl Rng,
) -> bool {
    rng.gen_range(0..100) < fail_chance(ps, spell, fast_cast)
}

/// `casting_failed` with the worn +spell-power (to_s) bonus folded in:
/// the cast path's failure roll and mana cost must both see it
/// (lua_bind.cc spell_chance_school / get_mana).
pub fn casting_failed_with_inv(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    spell: &SpellRow,
    fast_cast: bool,
    rng: &mut impl Rng,
) -> bool {
    rng.gen_range(0..100) < fail_chance_with_inv(ps, inv, gd, spell, fast_cast)
}

/// Artifact activations (a_info a: lines) mapped onto the shared effect
/// kinds. Every activation name in the data resolves to a real effect
/// (original src/cmd6.cc activation_aux). Returns (effect row, recharge
/// spec): "N" fixed, "N+dM" rolled, "gorlim" = 3*(lev+10).
pub fn artifact_activation(name: &str) -> (SpellRow, String) {
    let (kind, arg, targeted, recharge): (&str, &str, bool, &str) = match name {
        "LIGHT" => ("light", "", false, "10+d10"),
        "MAP_LIGHT" => ("map", "", false, "50+d50"),
        "PALANTIR" => ("map", "", false, "100+d100"),
        "DETECT_ALL" => ("detect", "", false, "55+d55"),
        "DETECT_XTRA" => ("detect", "", false, "500"),
        "DRUEDAIN" => ("detect", "", false, "99"),
        "STONE_MUD" => ("dig", "", true, "5"),
        "EREBOR" => ("dig", "", true, "75"),
        "GROND" => ("alter_reality", "", false, "100"),
        "FUNDIN" => ("dispel_evil", "", false, "100+d100"),
        "DISP_EVIL" => ("dispel_evil", "", false, "300+d300"),
        "TELEPORT" => ("teleport", "100", false, "45"),
        "DIM_DOOR" => ("teleport", "10", false, "100"),
        "BELEGENNON" => ("teleport", "10", false, "300"),
        "MAGGOT" => ("maggot", "", true, "10+d50"),
        "TELE_AWAY" => ("teleport_away_beam", "", true, "200"),
        "TULKAS" => ("teleport_away", "", true, "150+d150"),
        "HURIN" => ("hurin", "", false, "100+d200"),
        "HARADRIM" => ("shero", "25+d25", false, "50+d50"),
        "RECALL" => ("recall", "", false, "200"),
        "CURE_1000" => ("heal", "1000", false, "888"),
        "REST_ALL" => ("heal", "1000", false, "750"),
        "ERU" => ("eru", "", false, "500"),
        "CURE_700" => ("heal", "700", false, "250"),
        "REST_LIFE" => ("heal", "700", false, "450"),
        "CURE_MW" => ("cure", "", false, "3+d3"),
        "CURE_POISON" => ("cure", "", false, "5"),
        "LEBOHAUM" => ("cure", "", false, "3"),
        "BLADETURNER" => ("cure", "", false, "800"),
        "ELESSAR" => ("elessar", "", false, "200"),
        "TURMIL" => ("drain_life", "90", true, "70"),
        "DRAIN_2" => ("drain_life", "120", true, "400"),
        "GANDALF" => ("mana", "", false, "666"),
        "CONFUSE" => ("confuse_monster", "", true, "15"),
        "SLEEP" => ("sleep_touch", "", false, "55"),
        "GILGALAD" => ("starlight", "", false, "75+d75"),
        "CELEBRIMBOR" => ("esp", "20+d20", false, "20+d50"),
        "BARAHIR" => ("dispel_small", "", false, "55+d55"),
        "NATUREBANE" => ("dispel_all", "300", false, "200+d200"),
        "MEDIATOR" => ("mediator", "", true, "400"),
        "GORLIM" => ("scare_all", "", false, "gorlim"),
        "COLLUIN" => ("resist", "", false, "111"),
        "ROHAN" => ("rohan", "", false, "250"),
        "CUBRAGOL" => ("brand_bolts", "", false, "999"),
        // ACT_ANGUIREL (cmd6.cc:4850): dormant like the original (no
        // a:ANGUIREL line exists in a_info.txt).
        "ANGUIREL" => ("anguirel", "", false, "35"),
        "BOROMIR" => ("summon_friendly", "15", false, "1000"),
        "DAWN" => ("summon_friendly", "dawn", false, "500+d500"),
        "DEST_DOOR" => ("dest_door", "", false, "10"),
        "GENOCIDE" => ("genocide", "", true, "500"),
        "MASS_GENO" => ("mass_genocide", "", false, "1000"),
        "SKULLCLEAVER" => ("destruction", "", false, "200+d200"),
        "PROT_EVIL" => ("protevil", "", false, "225+d225"),
        "RECHARGE" => ("recharge", "", false, "70"),
        "SPEED" => ("haste", "", false, "250"),
        "HELM" => ("aball", "SOUND:300:6", true, "300"),
        "BO_MISS_1" => ("abolt", "2d6", true, "2"),
        "BO_MISS_2" => ("abolt", "150", true, "90+d90"),
        "BO_ACID_1" => ("abolt", "5d8:ACID", true, "5+d5"),
        "BO_COLD_1" => ("abolt", "6d8:COLD", true, "7+d7"),
        "BO_ELEC_1" => ("abolt", "4d8:ELEC", true, "6+d6"),
        "BO_FIRE_1" => ("abolt", "9d8:FIRE", true, "8+d8"),
        "EOL" => ("abolt", "9d8", true, "7+d7"),
        "UMBAR" => ("abolt", "10d10", true, "20+d20"),
        "BELANGIL" => ("aball", "COLD:48:2", true, "5+d5"),
        "FIRESTAR" => ("aball", "FIRE:72:3", true, "100"),
        "BA_COLD_1" => ("aball", "COLD:48:2", true, "400"),
        "DRAIN_1" => ("aball", "COLD:100:2", true, "100+d100"),
        "BA_FIRE_1" => ("aball", "FIRE:72:2", true, "400"),
        "BA_POIS_1" => ("aball", "POIS:12:3", true, "4+d4"),
        "BA_COLD_2" => ("aball", "COLD:100:2", true, "300"),
        "BA_ELEC_2" => ("aball", "ELEC:100:3", true, "500"),
        "BA_MISS_3" => ("aball", "MISSILE:300:4", true, "500"),
        "BA_FIRE_2" => ("aball", "FIRE:120:3", true, "225+d225"),
        "BA_COLD_3" => ("aball", "COLD:200:3", true, "325+d325"),
        "BA_ELEC_3" => ("aball", "ELEC:250:3", true, "425+d425"),
        "ROCKET" => ("aball", "ROCKET:120+lev:2", true, "400"),
        "AXE_GOTHMOG" => ("aball", "FIRE:300:4", true, "200+d200"),
        "RAZORBACK" => ("razorback", "", false, "1000"),
        "POWER" => ("power_attack", "", false, "200"),
        // Base-kind activations (k_info, cmd6.cc activation_aux): the
        // elemental protection balls and the dragon scale breath attacks.
        "BA_ACID_4" => ("aball_oppose", "ACID:50:2", true, "50+d50"),
        "BA_ELEC_4" => ("aball_oppose", "ELEC:50:2", true, "50+d50"),
        "BA_COLD_4" => ("aball_oppose", "COLD:50:2", true, "50+d50"),
        "BA_FIRE_4" => ("aball_oppose", "FIRE:50:2", true, "50+d50"),
        "BA_POIS_4" => ("aball", "POIS:100:2", true, "40+d60"),
        "BR_ELEC" => ("aball", "ELEC:100:2", true, "90+d90"),
        "BR_COLD" => ("aball", "COLD:110:2", true, "90+d90"),
        "BR_FIRE" => ("aball", "FIRE:200:2", true, "90+d90"),
        "BR_ACID" => ("aball", "ACID:130:2", true, "90+d90"),
        "BR_POIS" => ("aball", "POIS:150:2", true, "90+d90"),
        "BR_CONF" => ("aball", "CONF:120:2", true, "90+d90"),
        "BR_MANY" => (
            "br_choice",
            "ELEC|COLD|ACID|POIS|FIRE:250:2",
            true,
            "60+d60",
        ),
        "BR_SOUND" => ("aball", "SOUND:130:2", true, "90+d90"),
        "BR_CHAOS" => ("br_choice", "CHAOS|DISENCHANT:220:2", true, "90+d60"),
        "BR_SHARD" => ("br_choice", "SOUND|SHARDS:230:2", true, "90+d60"),
        "BR_BALANCE" => (
            "br_choice",
            "CHAOS|DISENCHANT|SOUND|SHARDS:250:2",
            true,
            "90+d60",
        ),
        "BR_LIGHT" => ("br_choice", "LITE|DARK:200:2", true, "90+d60"),
        "BR_POWER" => ("aball", "MISSILE:300:3", true, "90+d60"),
        "DEST_TELE" => ("dest_tele", "", false, "0"),
        // e_info activations (cmd6.cc activation_aux).
        "JUMP" => ("teleport", "10", false, "10+d10"),
        "NOLDOR" => ("detect_treasure", "", false, "10+d20"),
        "SPIN" => ("spin", "", false, "50+d25"),
        "SPECTRAL" => ("spectral", "", false, "50+d50"),
        "BA_FIRE_H" => ("ball_self", "FIRE:300:7", false, "100"),
        "BA_COLD_H" => ("ball_self", "COLD:300:7", false, "100"),
        "BA_ELEC_H" => ("ball_self", "ELEC:300:7", false, "100"),
        "BA_ACID_H" => ("ball_self", "ACID:300:7", false, "100"),
        "ORCHAST" => ("detect", "orc", false, "10"),
        "THRAIN" => ("detect", "", false, "30+d30"),
        "NIGHT" => ("night_drain", "", true, "250"),
        "UNDEATH" => ("ruination", "", false, "10+d10"),
        "NARYA" => ("ring_heal", "500", false, "200+d100"),
        "NENYA" => ("ring_heal", "800", false, "100+d200"),
        "VILYA" => ("ring_heal", "900+", false, "200+d200"),
        "DURANDIL" => ("durandil", "", false, "3"),
        // Legacy random-artifact activations (tables.cc activation_info).
        "DEATH" => ("self_damage", "5000", false, "100"),
        "RUINATION" => ("ruination", "", false, "100"),
        "DESTRUC" => ("quake", "", false, "100"),
        "UNINT" => ("stat_dec", "1", false, "100"),
        "UNSTR" => ("stat_dec", "0", false, "100"),
        "UNCON" => ("stat_dec", "4", false, "100"),
        "UNCHR" => ("stat_dec", "5", false, "100"),
        "UNDEX" => ("stat_dec", "3", false, "100"),
        "UNWIS" => ("stat_dec", "2", false, "100"),
        "STATLOSS" => ("stat_loss", "", false, "100"),
        "HISTATLOSS" => ("stat_loss_hi", "", false, "100"),
        "EXPLOSS" => ("exp_loss", "20", false, "100"),
        "HIEXPLOSS" => ("exp_loss", "10", false, "100"),
        "SUMMON_MONST" => ("summon", "", false, "100"),
        "PARALYZE" => ("paralyze", "", false, "100"),
        "HALLU" => ("hallu", "", false, "100"),
        "POISON" => ("poison", "", false, "100"),
        "HUNGER" => ("hunger", "", false, "100"),
        "STUN" => ("stun", "", false, "100"),
        "CUTS" => ("cuts", "", false, "100"),
        "PARANO" => ("fear", "", false, "100"),
        "CONFUSION" => ("confuse", "", false, "100"),
        "BLIND" => ("blind", "", false, "100"),
        "PET_SUMMON" => ("summon_friendly", "2", false, "100"),
        "CURE_PARA" => ("cure_para", "", false, "100"),
        "CURE_HALLU" => ("cure_hallu", "", false, "100"),
        "CURE_POIS" | "CURE_STUN" | "CURE_CUTS" | "CURING" => ("cure", "", false, "100"),
        "CURE_HUNGER" => ("cure_hunger", "", false, "100"),
        "CURE_FEAR" => ("cure_fear", "", false, "100"),
        "CURE_CONF" => ("cure_conf", "", false, "100"),
        "CURE_BLIND" => ("cure_blind", "", false, "100"),
        "CURE_LW" => ("heal", "2d8", false, "10"),
        "DARKNESS" => ("darkness", "", false, "100"),
        "LEV_TELE" => ("lev_tele", "", false, "100"),
        "ACQUIREMENT" => ("acquirement", "", false, "100"),
        "WEIRD" => ("nothing", "", false, "100"),
        "AGGRAVATE" => ("aggravate", "", false, "100"),
        "MUT" => ("mutate", "", false, "100"),
        "CURE_INSANITY" => ("cure_insanity", "10d10", false, "100"),
        "LIGHT_ABSORBTION" => ("light_absorb", "", false, "100"),
        "ETERNAL_FLAME" => ("eternal_flame", "", false, "0"),
        _ => ("fizzle", "", false, "50"),
    };
    (
        SpellRow {
            name: name.to_string(),
            school: 0,
            level: 1,
            mana: 0,
            mana_max: 0,
            fail: 0,
            kind: kind.to_string(),
            arg: arg.to_string(),
            targeted,
            elem: String::new(),
            god: String::new(),
            random_type: 0,
            charges: String::new(),
            alloc: Vec::new(),
            desc: Vec::new(),
        },
        recharge.to_string(),
    )
}

/// Roll an activation recharge spec ("N", "N+dM", or "gorlim" =
/// 3*(lev+10), cmd6.cc activation_aux).
pub fn roll_recharge(spec: &str, plev: i32, rng: &mut impl Rng) -> i32 {
    if spec == "gorlim" {
        return 3 * (plev + 10);
    }
    if let Some((base, die)) = spec.split_once("+d") {
        let base: i32 = base.parse().unwrap_or(0);
        let die: i32 = die.parse().unwrap_or(0);
        return base + rng.gen_range(0..die.max(1));
    }
    spec.parse().unwrap_or(100)
}

pub fn detect_message(count: usize, log: &mut MessageLog) {
    if count == 0 {
        log.add("You sense no monsters nearby.");
    } else if count == 1 {
        log.add("You sense a monster nearby.");
    } else {
        log.add(format!("You sense {} monsters nearby.", count));
    }
}

/// `spell_type_describe` (spell_type.cc:249): the registration-time
/// description lines (carried by `SpellRow.desc`).
pub fn spell_type_describe(sp: &SpellRow) -> &[String] {
    &sp.desc
}

/// `spell_type_description_foreach` (spell_type.cc:308): walk the lines.
pub fn spell_type_description_foreach(sp: &SpellRow) -> &[String] {
    &sp.desc
}

/// `no_info` (spells5.cc:30): the default info function.
pub fn no_info() -> String {
    String::new()
}

/// `spell_type_info` (spell_type.cc:393): the info dispatcher.
pub fn spell_type_info(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    sp: &SpellRow,
) -> String {
    spell_info(ps, inv, gd, sp)
}

/// `print_spell_desc` (spells4.cc:87): the description lines, the piety
/// note and the dynamic info.
pub fn print_spell_desc(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    sp: &SpellRow,
) -> Vec<String> {
    let mut out: Vec<String> = spell_type_description_foreach(sp).to_vec();
    if uses_piety_to_cast(sp) {
        out.push("It uses piety to cast.".to_string());
    }
    let info = spell_type_info(ps, inv, gd, sp);
    if !info.is_empty() {
        out.push(info);
    }
    out
}

/// The original per-spell `*_info` text (spells3.cc) and its level
/// helpers, shown next to the static data in the spell browser.
pub fn spell_info(
    ps: &PlayerState,
    inv: &crate::item::Inventory,
    gd: &GameData,
    sp: &SpellRow,
) -> String {
    let ls = |max: i32| get_level_s(ps, inv, gd, sp, max);
    let l0 = |max: i32| get_level_school(ps, inv, gd, sp, max, 0).0;
    let call_of_the_ulumuri_mlev = || 30 + l0(70);
    let call_to_the_halls_mlev = || 20 + l0(70);
    let charm_animal_power = || 10 + ls(170);
    let charm_animal_radius = || ls(2);
    let device_heal_monster_hp = || 20 + ls(380);
    let device_mana_pct = || 20 + ls(50);
    let draught_of_ulmonan_hp = || 5 * ls(50);
    let flow_of_life_hp = || 7 + ls(100);
    let get_spellbinder_max = || ls(4).min(4);
    let holding_pattern_power = || 10 + ls(100);
    let holy_fire_damage = || 50 + ls(300);
    let illusion_pattern_power = || 10 + ls(100);
    let mind_armor_of_fear_base_duration = || 10 + ls(100);
    let mind_armor_of_fear_power_dice = || 5 + ls(20);
    let mind_armor_of_fear_power_sides = || 1 + ls(7);
    let mind_charm_power = || 10 + ls(150);
    let mind_confuse_power = || 10 + ls(150);
    let mind_stun_power = || 10 + ls(150);
    let nature_grow_trees_radius = || 2 + ls(7);
    let nature_healing_percentage = || 15 + ls(35);
    let nature_healing_hp = || ps.max_hp as i32 * nature_healing_percentage() / 100;
    let regeneration_base_duration = || 5 + ls(50);
    let regeneration_power = || 300 + ls(700);
    let star_kindler_bursts = || ps.level as i32 / 5;
    let star_kindler_damage = || 20 + ls(100);
    let stun_pattern_power = || 10 + ls(90);
    let summon_animal_level = || 25 + ls(50);
    let tale_of_doom_duration = || 5 + ls(10);
    let tears_of_luthien_hp = || 10 * ls(30);
    let tempo_banishment_power = || 40 + ls(160);
    let tempo_essence_of_speed_base_duration = || 10 + ls(50);
    let tempo_essence_of_speed_bonus = || 5 + ls(20);
    let tidal_wave_damage = || 40 + ls(200);
    let tidal_wave_duration = || 6 + ls(10);
    let tree_roots_ac = || 10 + ls(60);
    let tree_roots_damage = || 10 + ls(20);
    let tree_roots_duration = || 10 + ls(30);
    let udun_flame_of_udun_base_duration = || 5 + ls(30);
    let udun_wraithform_base_duration = || 20 + ls(40);
    let uproot_mlevel = || 30 + ls(70);
    let water_bite_base_duration = || 30 + ls(150);
    let water_bite_damage = || 10 + ls(50);
    let water_ice_storm_damage = || 80 + ls(200);
    let water_ice_storm_duration = || 20 + ls(70);
    let water_ice_storm_radius = || 1 + l0(3);
    let water_vapor_damage = || 3 + ls(20);
    let water_vapor_duration = || 5 + 0;
    let water_vapor_radius = || 3 + l0(9);
    let wrath_of_ulmo_damage = || 40 + ls(150);
    let wrath_of_ulmo_duration = || 10 + ls(14);
    let yavanna_grow_grass_radius = || ls(4);
    let recall_get_d = || (21 - ls(15)).max(0);
    let recall_get_f = || (15 - ls(10)).max(1);
    let water_ent_potion_base_duration = || 25 + ls(40);
    let tempo_slow_monster_power = || 40 + ls(160);
    let light_of_valinor_damage = || 10 + ls(100);
    let light_of_valinor_radius = || 5 + ls(6);
    let get_manathrust_dam = || (3 + ls(50), 1 + ls(20));
    let get_geyser_damage = || (ls(10), 3 + ls(35));
    let get_belegaer_damage = || (ls(10), 3 + ls(35));
    match sp.name.as_str() {
        "Noxious Cloud" => format!("dam {} rad 3 dur {}", 7 + ls(150), 5 + ls(40)),
        "Wings of Winds" => format!("dur {}+d10", 5 + ls(25)),
        "Invisibility" => format!("dur {}+d20 power {}", 15 + ls(50), 20 + ls(50)),
        "Poison Blood" => format!("dur {}+d30", 25 + ls(25)),
        "Thunderstorm" => format!("dam {}d{} dur {}+d10", 5 + ls(10), 10 + ls(25), 10 + ls(25)),
        "Sterilize" => format!("dur {}+d30", 20 + ls(70)),
        "Phase Door" => format!("distance {}", 10 + ls(8)),
        "Teleport" | "Teleportation" => format!("distance {}", 100 + ls(100)),
        "Probability Travel" => format!("dur {}+d20", ls(60)),
        "Demon Blade" => format!("dur {}+d20 dam {}/blow", ls(80), 4 + ls(40)),
        "Demon Madness" => format!("dam {} rad {}", 20 + ls(200), 1 + l0(4)),
        "Demon Field" => format!("dam {} dur {}", 20 + ls(70), 30 + ls(100)),
        "Doom Shield" => format!("dur {}+d10 dam {}d{}", 20 + ls(100), 1 + ls(14), 10 + ls(15)),
        "Unholy Word" => format!("heal mhp% of {}%", 30 + l0(50)),
        "Demon Cloak" => format!("dur {}+d5", 5 + l0(15)),
        "Summon Demon" => format!("level {}", 5 + ls(100)),
        "Discharge Minion" => format!("dam {}% max {}", 20 + l0(60), 100 + l0(500)),
        "Control Demon" => format!("power {}", 50 + ls(250)),
        "Reveal Ways" => format!("rad {}", 10 + ls(40)),
        "Shake" => format!("rad {}", 4 + ls(10)),
        "See the Music" => format!("dur {}+d20", 10 + ls(100)),
        "Lay of Protection" => format!("rad {}", 1 + l0(2)),
        "Fireflash" => format!("dam {} rad {}", 20 + ls(500), 2 + ls(5)),
        "Fiery Shield" => format!("dam {}d{} dur {}+d20", 5 + ls(15), 5 + ls(7), 10 + ls(70)),
        "Firewall" => format!("dam {} dur {}", 40 + ls(150), 10 + ls(14)),
        "Fire Golem" => format!("golem level {}", 7 + ls(70)),
        "Call the Elements" => format!("rad {}", 1 + l0(5)),
        "Vaporize" => format!("rad {} dur {}", 1 + ls(4), 10 + ls(20)),
        "Geolysis" => format!("length {}", 5 + ls(12)),
        "Dripping Tread" => format!("dur {}+d15 movs", 10 + ls(50)),
        "Elemental Minion" => format!("min level {}", 10 + ls(120)),
        "Elemental Shield" => format!("dur {}+d10", 15 + ls(50)),
        "Disruption Shield" => format!("dur {}+d5", 3 + ls(10)),
        "Avatar" => format!("dur {}+d10", ls(20)),
        "Manwe's Blessing" => format!("dur {}+d40", ls(70) + 30),
        "Manwe's Call" => format!("level {}", ls(70) + 20),
        "Corpse Explosion" => format!("dam {}%", 20 + ls(70)),
        "Mind Steal" => format!("chance 1d(mlvl)<{}", ls(50)),
        "Recharge" => format!("power {}", 60 + ls(140)),
        "Spellbinder" => format!("number {} max level {}", get_spellbinder_max(), 7 + ls(35)),
        "Inertia Control" => format!("level {}", ls(10)),
        "Charm" => format!("power {}", mind_charm_power()),
        "Confuse" => format!("power {}", mind_confuse_power()),
        "Armor of Fear" => format!("dur {}+d10 power {}d{}", mind_armor_of_fear_base_duration(), mind_armor_of_fear_power_sides(), mind_armor_of_fear_power_dice()),
        "Stun" => format!("power {}", mind_stun_power()),
        "Essence of Speed" => format!("dur {}+d10 speed {}", tempo_essence_of_speed_base_duration(), tempo_essence_of_speed_bonus()),
        "Banishment" => format!("power {}", tempo_banishment_power()),
        "Divine Aim" => format!("dur {}+d10", ls(50)),
        "Wave of Power" => format!("blows {}", ls(crate::game::num_blows(ps, inv, gd))),
        "Wraithform" => format!("dur {}+d30", udun_wraithform_base_duration()),
        "Flame of Udun" => format!("dur {}+d15", udun_flame_of_udun_base_duration()),
        "Tidal Wave" => format!("dam {} dur {}", tidal_wave_damage(), tidal_wave_duration()),
        "Ice Storm" => format!("dam {} rad {} dur {}", water_ice_storm_damage(), water_ice_storm_radius(), water_ice_storm_duration()),
        "Vapor" => format!("dam {} rad {} dur {}", water_vapor_damage(), water_vapor_radius(), water_vapor_duration()),
        "Charm Animal" => format!("power {} rad {}", charm_animal_power(), charm_animal_radius()),
        "Grow Grass" => format!("rad {}", yavanna_grow_grass_radius()),
        "Tree Roots" => format!("dur {} AC {} dam {}", tree_roots_duration(), tree_roots_ac(), tree_roots_damage()),
        "Water Bite" => format!("dur {}+d30 dam {}/blow", water_bite_base_duration(), water_bite_damage()),
        "Uproot" => format!("lev {}", uproot_mlevel()),
        "Grow Trees" => format!("rad {}", nature_grow_trees_radius()),
        "Healing" => format!("heal {}% = {}hp", nature_healing_percentage(), nature_healing_hp()),
        "Regeneration" => format!("dur {}+d10 power {}", regeneration_base_duration(), regeneration_power()),
        "Summon Animal" => format!("level {}", summon_animal_level()),
        "Heal Monster" => format!("heal {}", device_heal_monster_hp()),
        "Mana" => format!("restore {}%", device_mana_pct()),
        "Holy Fire of Mithrandir" => format!("dam {}", holy_fire_damage()),
        "Holding Pattern(I)" => format!("power {}", holding_pattern_power()),
        "Illusion Pattern(II)" => format!("power {}", illusion_pattern_power()),
        "Stun Pattern(IV)" => format!("power {}", stun_pattern_power()),
        "Flow of Life(II)" => format!("heal {}/turn", flow_of_life_hp()),
        "Blow(I)" => format!("dam {}d{} rad {}", 2 + l0(10), 4 + l0(40), 1 + l0(12)),
        "Gush of Wind(II)" => format!("dist {} rad {}", 10 + l0(40), 1 + l0(12)),
        "Horns of Ylmir(III)" => format!("rad {}", 2 + ls(10)),
        "Enchant Weapon" => format!("tries {}", 1 + ls(50)/12),
        "Enchant Armour" => format!("tries {}", 1 + ls(50)/10),
        "Child of Aule" => format!("level {}", 20 + ls(70)),
        "Tears of Luthien" => format!("heals {}", tears_of_luthien_hp()),
        "Tale of Doom" => format!("dur {}", tale_of_doom_duration()),
        "Call to the Halls" => format!("level {}", call_to_the_halls_mlev()),
        "Draught of Ulmonan" => format!("cure {}", draught_of_ulmonan_hp()),
        "Call of the Ulumuri" => format!("level {}", call_of_the_ulumuri_mlev()),
        "Wrath of Ulmo" => format!("dam {} dur {}", wrath_of_ulmo_damage(), wrath_of_ulmo_duration()),
        "Star Kindler" => format!("dam {} bursts {} rad 10", star_kindler_damage(), star_kindler_bursts()),
        "Recall" | "Word of Recall" => format!("dur {}+d{} weight {}lb", recall_get_f(), recall_get_d(), 1 + ls(15)),
        "Sense Hidden" => if ls(50) >= 15 { format!("rad {} dur {}+d20", 15 + ls(40), 10 + ls(40)) } else { format!("rad {}", 15 + ls(40)) },
        "Sense Monsters" => if ls(50) >= 30 { format!("rad {} dur {}+d10", 10 + ls(40), 10 + ls(20)) } else { format!("rad {}", 10 + ls(40)) },
        "Stone Skin" => if ls(50) >= 25 { format!("dam {}d{} dur {}+d10 AC {}", 2 + ls(5), 3 + ls(5), 10 + ls(100), 10 + ls(50)) } else { format!("dur {}+d10 AC {}", 10 + ls(100), 10 + ls(50)) },
        "Strike" => if ls(50) >= 12 { format!("dam {} rad 1", 50 + ls(50)) } else { format!("dam {}", 50 + ls(50)) },
        "Globe of Light" => if ls(50) >= 15 { format!("dam {} rad {}", 10 + ls(100), 5 + ls(6)) } else { String::new() },
        "Manathrust" => { let (num, sides) = get_manathrust_dam(); format!("dam {}d{}", num, sides) },
        "Wind Shield" => { let mut buf = format!("dur {}+d20", ls(50) + 10); if ls(50) >= 10 { buf += &format!(" AC {}", ls(30)); } if ls(50) >= 20 { buf += &format!(" dam {}d{}", 1 + ls(2), 1 + ls(6)); } buf },
        "Slow Monster" => if ls(50) >= 20 { format!("power {} rad 1", tempo_slow_monster_power()) } else { format!("power {}", tempo_slow_monster_power()) },
        "Ent's Potion" => if ls(50) >= 12 { format!("dur {}+d25", water_ent_potion_base_duration()) } else { String::new() },
        "Geyser" => { let (dice, sides) = get_geyser_damage(); format!("dam {}d{}", dice, sides) },
        "Haste Monster" => "speed +10".to_string(),
        "Hobbit Melodies(III)" => if ls(50) >= 15 { format!("AC {} speed {}", 10 + ls(50), 7 + ls(10)) } else { format!("AC {}", 10 + ls(50)) },
        "Clairaudience(IV)" => if ls(50) >= 10 { format!("rad {}", 1 + l0(3)) } else { String::new() },
        "Firebrand" => { let level = ls(50); format!("dur {}+d20 dam {}/blow", level, 4 + level) },
        "Feanturi" => { let level = ls(50); if level >= 20 { format!("heals {}%", level) } else { String::new() } },
        "Song of Belegaer" => { let (dice, sides) = get_belegaer_damage(); format!("dam {}d{}", dice, sides) },
        "Light of Valinor" => if ls(50) >= 15 { format!("dam {} rad {}", light_of_valinor_damage(), light_of_valinor_radius()) } else { String::new() },
        _ => no_info(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::birth::make_player;
    use crate::data::load_game_data;

    fn class_idx(gd: &GameData, name: &str) -> usize {
        gd.classes.iter().position(|c| c.name == name).unwrap()
    }

    #[test]
    fn mage_knows_school_spells_by_level() {
        let gd = load_game_data();
        let mage = class_idx(&gd, "Mage");
        let ps = make_player(&gd, "T".into(), 0, mage);
        let names: Vec<&str> = known_spells(&gd, &ps)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"Manathrust"));
        // Phase Door needs one Conveyance school level (cmd5.cc
        // is_ok_spell); a level-1 Mage has none yet.
        assert!(!names.contains(&"Phase Door"));
        // Fireflash's original skill level is 10: both the position on
        // the school curve and the effective-level gate.
        assert!(!names.contains(&"Fireflash"));
        let mut ps5 = ps.clone();
        ps5.level = 6;
        let names5: Vec<&str> = known_spells(&gd, &ps5)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(!names5.contains(&"Fireflash"), "school skill gate");
        ps5.skills.insert(3, 10 * crate::skill::SKILL_STEP);
        let names6: Vec<&str> = known_spells(&gd, &ps5)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names6.contains(&"Fireflash"));
        // Identify belongs to Divination (Mage school 10), level 6.
        ps5.skills.insert(10, 6 * crate::skill::SKILL_STEP);
        let names7: Vec<&str> = known_spells(&gd, &ps5)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names7.contains(&"Identify"));
    }

    #[test]
    fn warrior_and_priest_spell_access() {
        let gd = load_game_data();
        let warrior = class_idx(&gd, "Warrior");
        let ps = make_player(&gd, "T".into(), 0, warrior);
        assert!(known_spells(&gd, &ps).is_empty());
        let priest = class_idx(&gd, "Priest");
        let pp = make_player(&gd, "T".into(), 0, priest);
        let names: Vec<&str> = known_spells(&gd, &pp)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"Cure Light Wounds"));
        // Priest has no arcane schools.
        assert!(!names.contains(&"Manathrust"));
    }

    #[test]
    fn casting_requires_a_spellbook() {
        let gd = load_game_data();
        let mage = class_idx(&gd, "Mage");
        let ps = make_player(&gd, "T".into(), 0, mage);
        let mut inv = crate::item::Inventory::default();
        // No book: nothing castable.
        assert!(castable_spells(&gd, &ps, &inv).is_empty());
        // Beginner Cantrips covers level <= 2 spells of any school.
        let cantrips = gd.object_by_tval_sval(crate::data::TV_BOOK, 50).unwrap();
        inv.pack.push(crate::item::Item::base(&gd, cantrips));
        let castable = castable_spells(&gd, &ps, &inv);
        assert!(castable.iter().any(|s| s.name == "Manathrust"));
        assert!(castable.iter().all(|s| s.level <= 2));
        // The Mana tome covers higher-level Mana spells too (with the
        // school skill trained up to the spell's original level: Remove
        // Curses is level 10).
        let tome = gd.object_by_tval_sval(crate::data::TV_BOOK, 0).unwrap();
        inv.pack.push(crate::item::Item::base(&gd, tome));
        let mut ps10 = ps.clone();
        ps10.level = 10;
        ps10.skills.insert(2, 10 * crate::skill::SKILL_STEP);
        let castable10 = castable_spells(&gd, &ps10, &inv);
        assert!(castable10.iter().any(|s| s.name == "Remove Curses"));
    }

    #[test]
    fn school_skill_reduces_fail_chance() {
        let gd = load_game_data();
        let mage = class_idx(&gd, "Mage");
        let mut ps = make_player(&gd, "T".into(), 0, mage);
        let sp = gd.spell_by_name("Manathrust").unwrap().clone();
        let stat = ps.stats[casting_stat(&sp)];
        let floor = adj_mag_fail(stat).max(5);
        ps.skills
            .insert(crate::skill::school_skill_id(sp.school), 5 * crate::skill::SKILL_STEP);
        // spell_chance_school: failure_rate - 3*(level-1)
        // - 3*(adj_mag_stat-1), clamped to the minimum.
        let level = 5;
        let expected = (sp.fail - 3 * (level - 1) - 3 * (adj_mag_stat(stat) - 1))
            .max(0)
            .max(floor)
            .min(95);
        assert_eq!(fail_chance(&ps, &sp, false), expected);
        // FAST_CAST is a port-only concept: the parameter is ignored.
        assert_eq!(fail_chance(&ps, &sp, true), expected);
        // Being trained below the spell's level cannot happen (the spell
        // is not known), but the untrained school reports the maximum.
        ps.skills.clear();
        assert_eq!(fail_chance(&ps, &sp, false), 95);
    }

    fn info_setup(school: u32) -> (crate::data::GameData, crate::game::PlayerState, crate::item::Inventory) {
        let gd = crate::data::load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Mage").unwrap_or(0);
        let mut ps = crate::birth::make_player(&gd, "I".into(), race, class);
        ps.skills.insert(
            crate::skill::school_skill_id(school),
            50 * crate::skill::SKILL_STEP,
        );
        let inv = crate::item::Inventory::default();
        (gd, ps, inv)
    }

    #[test]
    fn spell_info_matches_the_original_formulas() {
        let gd0 = crate::data::load_game_data();
        let nox = gd0.spell_by_name("Noxious Cloud").unwrap().clone();
        let (gd, ps, inv) = info_setup(nox.school);
        assert_eq!(
            spell_info(&ps, &inv, &gd, &nox),
            "dam 151 rad 3 dur 43"
        );
        let man = gd.spell_by_name("Manathrust").unwrap().clone();
        let (gd2, ps2, inv2) = info_setup(man.school);
        assert_eq!(spell_info(&ps2, &inv2, &gd2, &man), "dam 53d21");
        // A constant info string.
        let haste = gd.spell_by_name("Haste Monster").unwrap().clone();
        assert_eq!(spell_info(&ps, &inv, &gd, &haste), "speed +10");
    }

    #[test]
    fn spell_info_empty_without_school_training() {
        let gd = crate::data::load_game_data();
        let globe = gd.spell_by_name("Globe of Light").unwrap().clone();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Mage").unwrap_or(0);
        let ps = crate::birth::make_player(&gd, "N".into(), race, class);
        let inv = crate::item::Inventory::default();
        assert_eq!(spell_info(&ps, &inv, &gd, &globe), "");
    }

    #[test]
    fn spell_info_covers_every_spell_without_panicking() {
        let (gd, ps, inv) = info_setup(4);
        for sp in gd.spells.iter().filter(|s| s.school != 0) {
            let _ = spell_info(&ps, &inv, &gd, sp);
        }
    }

    #[test]
    fn god_spells_carry_their_school_and_deity() {
        let gd = crate::data::load_game_data();
        for (name, god, kind) in [
            ("Firebrand", "Aule", "aule_firebrand"),
            ("Enchant Weapon", "Aule", "aule_enchant_weapon"),
            ("Child of Aule", "Aule", "aule_child"),
            ("Light of Valinor", "Varda", "varda_light_of_valinor"),
            ("Evenstar", "Varda", "varda_evenstar"),
            ("Star Kindler", "Varda", "varda_star_kindler"),
            ("Song of Belegaer", "Ulmo", "ulmo_song_of_belegaer"),
            ("Wrath of Ulmo", "Ulmo", "ulmo_wrath_of_ulmo"),
            ("Tears of Luthien", "Mandos", "mandos_tears_of_luthien"),
            ("Call to the Halls", "Mandos", "mandos_call_to_the_halls"),
            ("Grow Athelas", "", "grow_athelas"),
        ] {
            let sp = gd.spell_by_name(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(sp.god, god, "{name} god");
            assert_eq!(sp.kind, kind, "{name} kind");
            if !god.is_empty() {
                assert_eq!(sp.school, 53, "{name} prayer school");
            }
        }
        assert_eq!(god_id("Aule"), 6);
        assert_eq!(god_id("Varda"), 7);
        assert_eq!(god_id("Ulmo"), 8);
        assert_eq!(god_id("Mandos"), 9);
        assert_eq!(god_id_full("Aule the Smith"), Some(6));
        assert_eq!(god_id_full("Varda Elentari"), Some(7));
    }

    #[test]
    fn spell_power_scales_with_skills() {
        let gd = load_game_data();
        let mage = class_idx(&gd, "Mage");
        let mut ps = make_player(&gd, "T".into(), 0, mage);
        // Untrained (birth skills cleared) casts at baseline power.
        ps.skills.clear();
        let sp = gd.spell_by_name("Manathrust").unwrap();
        assert_eq!(power_percent(&ps, sp, 0), 0);
        ps.skills.insert(
            crate::skill::school_skill_id(sp.school),
            5 * crate::skill::SKILL_STEP,
        );
        ps.skills
            .insert(crate::skill::SK_SPELL, 10 * crate::skill::SKILL_STEP);
        // 5 school levels * 5% + 10 Spell-power levels * 2% + item pval.
        assert_eq!(power_percent(&ps, sp, 7), 7 + 25 + 20);
    }

    #[test]
    fn god_spells_need_worship_and_songs_need_an_instrument() {
        let gd = load_game_data();
        let priest = class_idx(&gd, "Priest");
        let mut ps = make_player(&gd, "T".into(), 0, priest);
        ps.level = 10;
        ps.skills
            .insert(crate::skill::SK_PRAY, 10 * crate::skill::SKILL_STEP);
        // Without a god, god-specific spells are unknown.
        assert!(!known_spells(&gd, &ps).iter().any(|s| s.name == "Whirlwind"));
        ps.god = 3; // Tulkas
        assert!(known_spells(&gd, &ps).iter().any(|s| s.name == "Whirlwind"));
        // A different god grants different spells.
        assert!(!known_spells(&gd, &ps)
            .iter()
            .any(|s| s.name == "Water Bite"));
        let mut inv = crate::item::Inventory::default();
        let whirl = gd.spell_by_name("Whirlwind").unwrap();
        assert!(has_book_for(&inv, &gd, whirl), "god spells need no book");
        // Bard songs (Music school) are sung through a wielded instrument
        // whose pval must reach the song's roman numeral.
        let song = gd.spell_by_name("Heroic Ballad(II)").unwrap();
        assert_eq!(song.school, 100);
        assert!(!has_book_for(&inv, &gd, song), "no instrument wielded");
        // A base Harp has pval 1: enough for (I) songs, not for (II).
        let harp = gd.object_by_name("Harp").unwrap();
        let sun = gd.spell_by_name("Song of the Sun(I)").unwrap();
        inv.equip[crate::data::SLOT_WEAPON] = Some(crate::item::Item::base(&gd, harp));
        assert!(has_book_for(&inv, &gd, sun), "instrument wielded");
        assert!(!has_book_for(&inv, &gd, song), "instrument pval too low");
        // A powerful harp (pval 2) carries the ballad.
        if let Some(it) = inv.equip[crate::data::SLOT_WEAPON].as_mut() {
            it.pval = 2;
        }
        assert!(has_book_for(&inv, &gd, song), "instrument pval reached");
        // A Drum (sval 58) does not carry the harp song.
        let drum = gd.object_by_name("Drum").unwrap();
        let mut drum_item = crate::item::Item::base(&gd, drum);
        drum_item.pval = 2;
        inv.equip[crate::data::SLOT_WEAPON] = Some(drum_item);
        assert!(!has_book_for(&inv, &gd, song), "wrong instrument");
    }

    #[test]
    fn spell_containers_grant_their_spell() {
        let gd = load_game_data();
        // Shadow Cloak of Luthien (a_info 49, tval cloak sval 6) carries
        // SPELL_CONTAIN/WIELD_CAST; an inscribed spell is castable once worn.
        let base = gd.object_by_tval_sval(crate::data::TV_CLOAK, 6).unwrap();
        let mut cloak = crate::item::Item::base(&gd, base);
        cloak.artifact = 49;
        cloak.spells.push("Manathrust".to_string());
        let mut inv = crate::item::Inventory::default();
        let sp = gd.spell_by_name("Manathrust").unwrap();
        inv.pack.push(cloak.clone());
        assert!(!has_book_for(&inv, &gd, sp), "worn flag requires equipping");
        inv.pack.clear();
        inv.equip[crate::data::SLOT_CLOAK] = Some(cloak);
        assert!(has_book_for(&inv, &gd, sp));
        // The carried list includes the container's spell.
        let warrior = class_idx(&gd, "Warrior");
        let ps = make_player(&gd, "T".into(), 0, warrior);
        assert!(known_spells_with_books(&gd, &ps, &inv)
            .iter()
            .any(|s| s.name == "Manathrust"));
    }

    #[test]
    fn daemon_books_cover_the_demon_school_while_worn() {
        let gd = load_game_data();
        // Demonologist is a Warrior specialisation (p_info C:a:) whose
        // Demonology skill opens school 13 (Demon).
        let warrior = class_idx(&gd, "Warrior");
        let spec = gd.classes[warrior]
            .specs
            .iter()
            .position(|s| s.name == "Demonologist")
            .expect("Demonologist spec");
        let mut ps = crate::birth::make_player_spec(&gd, "T".into(), 0, 0, warrior, spec);
        ps.level = 25;
        ps.skills
            .insert(crate::skill::SK_DAEMON, 25 * crate::skill::SKILL_STEP);
        assert!(class_schools(&gd, &ps).contains(&13));
        let names: Vec<&str> = known_spells(&gd, &ps)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains(&"Demon Blade"));
        assert!(names.contains(&"Control Demon"));
        // Each daemon book holds three spells and only casts while worn.
        let blade = gd
            .object_by_tval_sval(crate::data::TV_DAEMON_BOOK, 55)
            .unwrap();
        let shield = gd
            .object_by_tval_sval(crate::data::TV_DAEMON_BOOK, 56)
            .unwrap();
        let horn = gd
            .object_by_tval_sval(crate::data::TV_DAEMON_BOOK, 57)
            .unwrap();
        assert_eq!(
            data::slot_of_item(&gd.objects[blade]),
            Some(data::SLOT_WEAPON)
        );
        assert_eq!(
            data::slot_of_item(&gd.objects[shield]),
            Some(data::SLOT_SHIELD)
        );
        assert_eq!(data::slot_of_item(&gd.objects[horn]), Some(data::SLOT_HEAD));
        let sp = gd
            .spells
            .iter()
            .find(|s| s.name == "Demon Blade" && s.school == 13)
            .unwrap();
        let mut inv = crate::item::Inventory::default();
        inv.pack.push(crate::item::Item::base(&gd, blade));
        assert!(!has_book_for(&inv, &gd, sp), "must be wielded");
        inv.pack.clear();
        inv.equip[crate::data::SLOT_WEAPON] = Some(crate::item::Item::base(&gd, blade));
        assert!(has_book_for(&inv, &gd, sp));
        // The shield book holds other spells; the horn book the summons.
        let cloak = gd
            .spells
            .iter()
            .find(|s| s.name == "Demon Cloak" && s.school == 13)
            .unwrap();
        assert!(!has_book_for(&inv, &gd, cloak));
        inv.equip[crate::data::SLOT_SHIELD] = Some(crate::item::Item::base(&gd, shield));
        assert!(has_book_for(&inv, &gd, cloak));
        let summon = gd
            .spells
            .iter()
            .find(|s| s.name == "Summon Demon" && s.school == 13)
            .unwrap();
        inv.equip[crate::data::SLOT_HEAD] = Some(crate::item::Item::base(&gd, horn));
        assert!(has_book_for(&inv, &gd, summon));
    }

    #[test]
    fn remaining_school_spells_are_in_the_data() {
        let gd = load_game_data();
        let expected: [(&str, &str); 19] = [
            ("Geyser", "geyser"),
            ("Vapor", "vapor"),
            ("Ent's Potion", "ents_potion"),
            ("Invisibility", "invisibility"),
            ("Stone Skin", "stone_skin"),
            ("Grow Trees", "grow_trees"),
            ("Regeneration", "regeneration"),
            ("Recharge", "recharge"),
            ("Spellbinder", "spellbinder"),
            ("Tracker", "tracker"),
            ("Inertia Control", "inertia_control"),
            ("Elemental Shield", "elemental_shield"),
            ("Disruption Shield", "disruption_shield"),
            ("Drain", "drain_device"),
            ("Stun", "stun_bolt"),
            ("Armor of Fear", "armor_of_fear"),
            ("Wraithform", "wraithform"),
            ("Flame of Udun", "flame_of_udun"),
            ("Fire Golem", "fire_golem"),
        ];
        for (name, kind) in expected {
            let sp = gd
                .spell_by_name(name)
                .unwrap_or_else(|| panic!("missing spell {name}"));
            assert_eq!(sp.kind, kind, "{name}");
        }
        // Multi-school spells (spells5.cc init + add_school order).
        assert_eq!(second_school("Grow Trees"), 11);
        assert_eq!(second_school("Tracker"), 1);
        assert_eq!(second_school("Drain"), 2);
        assert_eq!(second_school("Wraithform"), 1);
        assert_eq!(second_school("Flame of Udun"), 3);
        assert_eq!(second_school("Fire Golem"), 51);
        // Book placement (spells4.cc init_school_books).
        assert_eq!(
            spell_book_school(gd.spell_by_name("Grow Trees").unwrap()),
            6
        );
        assert_eq!(spell_book_school(gd.spell_by_name("Tracker").unwrap()), 14);
        assert_eq!(spell_book_school(gd.spell_by_name("Drain").unwrap()), 55);
        assert_eq!(
            spell_book_school(gd.spell_by_name("Fire Golem").unwrap()),
            3
        );
        // Inertia data only for controllable spells.
        assert_eq!(inertia_info("Stone Skin"), Some((2, 50)));
        assert_eq!(inertia_info("Fire Golem"), None);
        // Mind Steal is now a control effect (spells3.cc).
        assert_eq!(gd.spell_by_name("Mind Steal").unwrap().kind, "mind_steal");
        // Fire golem carries AI_PLAYER (r_info 1043).
        let golem = gd.monsters.iter().find(|m| m.name == "Fire golem");
        assert!(golem.map(|g| g.has("AI_PLAYER")).unwrap_or(false));
    }

    #[test]
    fn bard_song_list_matches_the_original_thirteen() {
        let gd = load_game_data();
        let songs: Vec<&str> = gd
            .spells
            .iter()
            .filter(|s| s.school == 100)
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(songs.len(), 13, "{songs:?}");
        for name in [
            "Stop singing(I)",
            "Holding Pattern(I)",
            "Illusion Pattern(II)",
            "Stun Pattern(IV)",
            "Song of the Sun(I)",
            "Flow of Life(II)",
            "Heroic Ballad(II)",
            "Hobbit Melodies(III)",
            "Clairaudience(IV)",
            "Blow(I)",
            "Gush of Wind(II)",
            "Horns of Ylmir(III)",
            "Ambarkanta(IV)",
        ] {
            assert!(songs.contains(&name), "{name}");
        }
        // Instrument books (spells4.cc BOOK_DRUMS/HARPS/HORNS) and the
        // minimum pval of each song.
        assert_eq!(
            music_song_info("Holding Pattern(I)"),
            Some((58, 1, true, true))
        );
        assert_eq!(
            music_song_info("Illusion Pattern(II)"),
            Some((58, 2, true, true))
        );
        assert_eq!(
            music_song_info("Stun Pattern(IV)"),
            Some((58, 4, true, true))
        );
        assert_eq!(
            music_song_info("Flow of Life(II)"),
            Some((59, 2, false, true))
        );
        assert_eq!(music_song_info("Blow(I)"), Some((60, 1, false, false)));
        assert_eq!(
            music_song_info("Ambarkanta(IV)"),
            Some((60, 4, false, false))
        );
        assert_eq!(
            music_song_info("Stop singing(I)"),
            Some((0, 1, true, false))
        );
    }

    #[test]
    fn library_tome_teaches_its_spells() {
        let gd = load_game_data();
        let warrior = class_idx(&gd, "Warrior");
        let ps = make_player(&gd, "T".into(), 0, warrior);
        let mut inv = crate::item::Inventory::default();
        let tome_def = gd.object_by_tval_sval(crate::data::TV_BOOK, 61).unwrap();
        let mut tome = crate::item::Item::base(&gd, tome_def);
        tome.spells = vec![
            "Manathrust".to_string(),
            "Heal".to_string(),
            "Whirlwind".to_string(),
        ];
        inv.pack.push(tome);
        let names: Vec<&str> = known_spells_with_books(&gd, &ps, &inv)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        for n in ["Manathrust", "Heal", "Whirlwind"] {
            assert!(names.contains(&n), "{}", n);
            let sp = gd.spell_by_name(n).unwrap();
            assert!(has_book_for(&inv, &gd, sp), "{}", n);
        }
        // The book does not grant unrelated spells.
        let other = gd.spell_by_name("Fireflash").unwrap();
        assert!(!has_book_for(&inv, &gd, other));
        // The bookable list is the q_library list restricted to ported rows.
        let bookable = bookable_spells(&gd);
        assert_eq!(bookable.len(), BOOKABLE_SPELLS.len());
        assert!(bookable.iter().any(|s| s.name == "Manathrust"));
    }

    #[test]
    fn every_spell_row_has_a_real_effect_and_schools_resolve() {
        let gd = load_game_data();
        // No placeholder rows are left: every spell (device or class) maps
        // onto a concrete effect kind.
        for sp in &gd.spells {
            assert_ne!(sp.kind, "fizzle", "{} still fizzles", sp.name);
        }
        // The multi-school spells of spells5.cc (primary + add_school).
        for (name, second) in [
            ("Wings of Winds", 1),
            ("Thunderstorm", 6),
            ("Banishment", 1),
            ("Genocide", 6),
        ] {
            assert_eq!(second_school(name), second, "{}", name);
        }
        // Every new class row is present with its book school.
        for name in [
            "Firewall",
            "Fiery Shield",
            "Ice Storm",
            "Healing",
            "Recovery",
            "Vision",
            "Sense Hidden",
            "Reveal Ways",
            "Sense Monsters",
            "Magelock",
            "Slow Monster",
            "Essence of Speed",
            "Disperse Magic",
            "Charm",
            "Confuse",
            "Probability Travel",
            "Poison Blood",
            "Stone Prison",
            "Shake",
            "Wings of Winds",
        ] {
            let sp = gd
                .spell_by_name(name)
                .unwrap_or_else(|| panic!("{} missing", name));
            assert_ne!(sp.school, 0, "{} is still device-only", name);
            assert!(!sp.kind.is_empty());
        }
        assert_eq!(
            spell_book_school(gd.spell_by_name("Wings of Winds").unwrap()),
            4
        );
        assert_eq!(
            spell_book_school(gd.spell_by_name("Banishment").unwrap()),
            11
        );
        assert_eq!(spell_book_school(gd.spell_by_name("Genocide").unwrap()), 55);
    }

    #[test]
    fn school_book_contents_match_init_school_books() {
        let gd = load_game_data();
        let has = |sval: i32, name: &str| {
            book_spells(sval)
                .map(|n| n.contains(&name))
                .unwrap_or(false)
        };
        // Beginner Cantrips: exactly the six original cantrips.
        assert!(has(50, "Manathrust"));
        assert!(has(50, "Ent's Potion"));
        assert!(has(50, "Sense Hidden"));
        assert!(!has(50, "Charm"));
        assert!(!has(50, "Magelock"));
        // Word of Recall lives in the Translocation book, not the Time.
        assert!(has(5, "Word of Recall"));
        assert!(!has(8, "Word of Recall"));
        // Knowledge holds only the four divinations.
        assert!(has(7, "Vision"));
        assert!(!has(7, "Detect Monsters"));
        assert!(!has(7, "Identify"));
        // Fire Golem also sits in the Book of Summoning.
        assert!(has(52, "Fire Golem"));
        assert!(has(52, "Summon Animal"));
        assert!(!has(52, "Regeneration"));
        // The god books list the base god spells.
        assert!(has(21, "Avatar"));
        assert!(has(24, "Tree Roots"));
        assert!(has(24, "Uproot"));
        // Every book entry resolves to a real spell row.
        for sval in [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 20, 21, 22, 23, 24, 50, 51, 52,
        ] {
            for name in book_spells(sval).unwrap() {
                assert!(
                    gd.spells.iter().any(|s| s.name == *name),
                    "book {sval}: {name} missing"
                );
            }
        }
    }

    #[test]
    fn spell_type_accessors_match_the_registration_calls() {
        let gd = load_game_data();
        // spell_new + spell_type_set_difficulty/set_mana/set_device_charges.
        let fireflash = gd.spell_by_name("Fireflash").unwrap();
        assert_eq!(spell_name(fireflash), "Fireflash");
        assert_eq!(required_skill(fireflash), 10);
        assert_eq!(failure_rate(fireflash), 35);
        assert_eq!(mana_range(fireflash), (5, 70));
        assert!(!uses_piety_to_cast(fireflash));
        // spell_type_init_music: CHR, Music random books, school 100, the
        // song numeral is the minimum instrument pval.
        let ballad = gd.spell_by_name("Heroic Ballad(II)").unwrap();
        assert_eq!(casting_stat(ballad), crate::game::CHA);
        assert_eq!(random_type(ballad), crate::skill::SK_MUSIC);
        assert_eq!(get_schools(ballad), vec![100]);
        assert_eq!(minimum_pval(ballad), 2);
        assert_eq!(spell_type_describe(ballad).len(), spell_type_description_foreach(ballad).len());
        // spell_type_init_priest: WIS + Spirituality + Prayer school.
        let manwe = gd.spell_by_name("Manwe's Blessing").unwrap();
        assert_eq!(casting_stat(manwe), crate::game::WIS);
        assert_eq!(random_type(manwe), crate::skill::SK_SPIRITUALITY);
        assert!(uses_piety_to_cast(manwe));
        assert_eq!(get_schools(manwe), vec![53]);
        // spell_type_init_mage with RANDOM: INT + Magic.
        assert_eq!(casting_stat(fireflash), crate::game::INT);
        assert_eq!(random_type(fireflash), crate::skill::SK_MAGIC);
        // spell_type_init_device (school 0) and spell_type_init_demonology
        // (school 13): NO_RANDOM.
        let heal_monster = gd
            .spells
            .iter()
            .find(|s| s.name == "Heal Monster")
            .unwrap();
        assert_eq!(random_type(heal_monster), 0);
        assert_eq!(get_schools(heal_monster), vec![0]);
        let demon_blade = gd
            .spells
            .iter()
            .find(|s| s.name == "Demon Blade" && s.school == 13)
            .unwrap();
        assert_eq!(random_type(demon_blade), 0);
        assert_eq!(casting_stat(demon_blade), crate::game::INT);
        // spell_type_add_school: registration order (primary first).
        let wraith = gd.spell_by_name("Wraithform").unwrap();
        assert_eq!(get_schools(wraith), vec![55, 1]);
        // spell_type_device_allocation + A: rows.
        assert_eq!(
            device_allocation(heal_monster, crate::data::TV_WAND),
            Some((65, 17, 1, 15, 20, 50))
        );
        assert_eq!(device_allocation(heal_monster, 999), None);
        // spell_type_roll_charges: "10+d10" = 10 + damroll(1, 10).
        assert_eq!(heal_monster.charges, "10+d10");
        let mut rng = crate::rng::new_seeded_rng(17);
        for _ in 0..50 {
            let c = roll_device_charges(heal_monster, &mut rng);
            assert!((11..=20).contains(&c), "{c}");
        }
        // A fixed charges value is returned as-is.
        let mut fixed = heal_monster.clone();
        fixed.charges = "7".to_string();
        assert_eq!(roll_device_charges(&fixed, &mut rng), 7);
        assert_eq!(roll_device_charges(ballad, &mut rng), 0, "songs have no charges");
    }

    #[test]
    fn prayer_power_and_failure_follow_spells4() {
        let gd = load_game_data();
        let mage = class_idx(&gd, "Mage");
        let mut ps = make_player(&gd, "P".into(), 0, mage);
        let cure = gd
            .spells
            .iter()
            .find(|s| s.name == "Cure Light Wounds" && s.school == 53)
            .unwrap()
            .clone();
        let man = gd.spell_by_name("Manathrust").unwrap().clone();
        // get_power_name / spell_type_uses_piety_to_cast.
        assert!(uses_piety_to_cast(&cure));
        assert_eq!(power_name(&cure), "piety");
        assert!(!uses_piety_to_cast(&man));
        assert_eq!(power_name(&man), "mana");
        // get_power: grace for prayers, mana otherwise.
        ps.grace = 37;
        ps.mana = 11;
        assert_eq!(power(&ps, &cure), 37);
        assert_eq!(power(&ps, &man), 11);
        // lua_cast_school_spell: the full get_mana cost is paid, not half.
        ps.skills.clear();
        ps.skills
            .insert(crate::skill::school_skill_id(53), 10 * crate::skill::SKILL_STEP);
        ps.mana = 500;
        let cost = spell_mana_no_inv(&ps, &cure);
        spend_mana(&mut ps, &cure);
        assert_eq!(ps.mana, 500 - cost);
        // The not-enough-power penalty measures the prayer against grace
        // (get_power), not against mana.
        let stat = ps.stats[casting_stat(&cure)];
        let minfail = adj_mag_fail(stat).max(5);
        let mut poor = ps.clone();
        poor.grace = 0;
        let mut rich = ps.clone();
        rich.grace = 100;
        assert_eq!(fail_chance(&poor, &cure, false), (15 * cost).max(minfail));
        assert_eq!(fail_chance(&rich, &cure, false), minfail);
        // The +25 edged-weapon penalty only applies to Eru's priests
        // (lua_bind.cc `forbid_non_blessed() && icky_wield`).
        rich.icky_wield = true;
        let no_god = fail_chance(&rich, &cure, false);
        rich.god = 1; // Eru: forbid_non_blessed
        assert_eq!(no_god, minfail);
        assert_eq!(fail_chance(&rich, &cure, false), 25.max(minfail).min(95));
    }
}

#[cfg(test)]
mod level_s_tests {
    use super::*;
    use crate::birth::make_player;
    use crate::data::load_game_data;

    fn mage(gd: &GameData) -> PlayerState {
        let idx = gd.classes.iter().position(|c| c.name == "Mage").unwrap();
        make_player(gd, "T".into(), 0, idx)
    }

    #[test]
    fn get_level_s_follows_the_original_curve() {
        let gd = load_game_data();
        let ps = mage(&gd);
        let sp = gd.spell_by_name("Manathrust").unwrap().clone();
        let inv = crate::item::Inventory::default();
        // Untrained school: level floors at 1 (lua_get_level min).
        let mut p0 = ps.clone();
        p0.skills.clear();
        assert_eq!(get_level_s(&p0, &inv, &gd, &sp, 50), 1);
        // Mana school 20: (2000 - 0)*50/5000 = 20.
        let mut p20 = ps.clone();
        p20.skills.insert(2, 20 * crate::skill::SKILL_STEP);
        assert_eq!(get_level_s(&p20, &inv, &gd, &sp, 50), 20);
        // Mana school 50 reaches the curve's reference point: 50.
        let mut p50 = ps.clone();
        p50.skills.insert(2, 50 * crate::skill::SKILL_STEP);
        assert_eq!(get_level_s(&p50, &inv, &gd, &sp, 50), 50);
        p50.skills
            .insert(crate::skill::SK_SPELL, 50 * crate::skill::SKILL_STEP);
        // +20 levels of Spell-power bonus push past it (no clamp): 70.
        assert_eq!(get_level_s(&p50, &inv, &gd, &sp, 50), 70);
    }

    #[test]
    fn worn_spell_power_lowers_cast_cost_and_failure() {
        let gd = load_game_data();
        let mut ps = mage(&gd);
        ps.skills.insert(2, 20 * crate::skill::SKILL_STEP);
        // Plenty of power: the failure comparison must not be distorted by
        // the "not enough mana" penalty (fail_chance adds 15 per point).
        ps.max_mana = 1000;
        ps.mana = 1000;
        let sp = gd.spell_by_name("Manathrust").unwrap().clone();
        let mut inv = crate::item::Inventory::default();
        let plain_cost = spell_mana(&ps, &inv, &gd, &sp);
        let plain_fail = fail_chance_with_inv(&ps, &inv, &gd, &sp, false);
        // A worn +SPELL item (e.g. an ego/artifact power) raises to_s.
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut wand = crate::item::Item::base(&gd, sword);
        wand.flags.push("SPELL".to_string());
        wand.pval = 10;
        inv.equip[crate::data::SLOT_WEAPON] = Some(wand);
        let boosted_cost = spell_mana(&ps, &inv, &gd, &sp);
        let boosted_fail = fail_chance_with_inv(&ps, &inv, &gd, &sp, false);
        // get_mana scales the cost up with the effective level and caps at
        // mana_max (25 for Manathrust), so worn +spell-power raises it; the
        // failure roll is the part that improves.
        assert!(
            boosted_cost != plain_cost,
            "to_s must reach get_mana: {boosted_cost} == {plain_cost}"
        );
        assert!(
            boosted_fail <= plain_fail,
            "to_s must not raise the failure rate"
        );
        // The cast-path helper rolls against the inv-aware chance.
        let mut rng = crate::rng::new_seeded_rng(3);
        let _ = casting_failed_with_inv(&ps, &inv, &gd, &sp, false, &mut rng);
    }

    #[test]
    fn device_level_uses_magic_device_and_pval3() {
        let gd = load_game_data();
        let ps = mage(&gd);
        let sp = gd.spell_by_name("Manathrust").unwrap().clone();
        // pval3 = (max 33 << 16) | bonus 20, Magic-Device 0:
        // lvl = 20*1000/10 = 2000(1/100) -> (2000-0)*50/5000 = 20? capped
        // by stick max: (33+1-1)=33 no cap -> 20.
        let mut p = ps.clone();
        p.skills.clear();
        assert_eq!(device_level_s(&p, &sp, (33 << 16) | 20, 50), 20);
        // Device skill 10 adds ten more.
        p.skills
            .insert(crate::skill::SK_DEVICE, 10 * crate::skill::SKILL_STEP);
        assert_eq!(device_level_s(&p, &sp, (33 << 16) | 20, 50), 30);
    }

    #[test]
    fn activation_recharge_specs_roll() {
        let mut rng = crate::rng::current();
        assert_eq!(roll_recharge("300", 1, &mut rng), 300);
        for _ in 0..50 {
            let v = roll_recharge("75+d75", 1, &mut rng);
            assert!((75..150).contains(&v), "{}", v);
        }
        assert_eq!(roll_recharge("gorlim", 10, &mut rng), 60);
        // Every activation name in the data maps to a real effect
        // (no fizzle): both a_info artifacts and base k_info items.
        let gd = crate::data::load_game_data();
        let mut names: Vec<&str> = gd
            .artifacts
            .iter()
            .map(|a| a.activate.as_str())
            .filter(|a| !a.is_empty())
            .collect();
        names.extend(
            gd.objects
                .iter()
                .map(|o| o.activate.as_str())
                .filter(|a| !a.is_empty()),
        );
        for name in names {
            let (row, _) = artifact_activation(name);
            assert_ne!(row.kind, "fizzle", "{} still fizzles", name);
        }
        for name in [
            "FUNDIN", "HARADRIM", "NARYA", "VILYA", "THRAIN", "DAWN", "NIGHT",
        ] {
            let (row, _) = artifact_activation(name);
            assert_ne!(row.kind, "fizzle", "{}", name);
        }
        assert_eq!(artifact_activation("FUNDIN").0.kind, "dispel_evil");
        assert_eq!(artifact_activation("HARADRIM").0.kind, "shero");
        assert_eq!(artifact_activation("GROND").0.kind, "alter_reality");
        // The One Ring's ACT_POWER opens the ring_of_power menu.
        let (power, _) = artifact_activation("POWER");
        assert_eq!(power.kind, "power_attack");
        assert!(!power.targeted);
        // e_info a: ego activations (Noldor/Spinning/Spectral/dragon
        // scale breaths) all resolve to real effects.
        let ego_names: Vec<&str> = gd
            .egos
            .iter()
            .map(|e| e.activate.as_str())
            .filter(|a| !a.is_empty())
            .collect();
        assert!(ego_names.len() >= 9, "{ego_names:?}");
        for name in &ego_names {
            let (row, _) = artifact_activation(name);
            assert_ne!(row.kind, "fizzle", "{} still fizzles", name);
        }
        assert_eq!(artifact_activation("NOLDOR").0.kind, "detect_treasure");
        assert_eq!(artifact_activation("SPECTRAL").0.kind, "spectral");
        assert_eq!(artifact_activation("SPIN").0.kind, "spin");
        assert_eq!(artifact_activation("BA_FIRE_H").0.kind, "ball_self");
    }

    #[test]
    fn gating_and_school_providers_follow_spells6() {
        let gd = load_game_data();
        // Castable while blind / confused (spells5.cc flags).
        assert!(castable_while_blind(
            gd.spell_by_name("See the Music").unwrap()
        ));
        assert!(castable_while_blind(
            gd.spell_by_name("Disperse Magic").unwrap()
        ));
        assert!(!castable_while_blind(
            gd.spell_by_name("Manathrust").unwrap()
        ));
        assert!(castable_while_confused(
            gd.spell_by_name("Disperse Magic").unwrap()
        ));
        assert!(!castable_while_confused(
            gd.spell_by_name("Whirlwind").unwrap()
        ));
        // The full 31-spell inertia table.
        assert_eq!(inertia_info("Vision"), Some((2, 200)));
        assert_eq!(inertia_info("Globe of Light"), Some((1, 40)));
        // God-provided school levels (spells6.cc school_god).
        let mage = gd.classes.iter().position(|c| c.name == "Mage").unwrap();
        let mut ps = make_player(&gd, "T".into(), 0, mage);
        ps.skills.clear();
        ps.god = 1; // Eru: Mana = Pray/2, Divination = 2*Pray/3.
        ps.skills
            .insert(crate::skill::SK_PRAY, 30 * crate::skill::SKILL_STEP);
        assert_eq!(school_skill(&ps, 2), 15);
        assert_eq!(school_skill(&ps, 10), 20);
        // Udun bonus levels: (2*lev)/3 (spells6.cc udun_bonus_levels).
        ps.level = 30;
        assert_eq!(school_skill(&ps, 55), 20);
        // Sorcery substitutes for trained schools.
        ps.god = 0;
        ps.skills
            .insert(crate::skill::SK_SORCERY, 25 * crate::skill::SKILL_STEP);
        assert_eq!(school_skill(&ps, 3), 25);
    }

    #[test]
    fn device_only_rows_use_the_original_difficulty() {
        let gd = load_game_data();
        for (name, level, fail) in [
            ("Heal Monster", 3, 15),
            ("Haste Monster", 10, 30),
            ("Wish", 50, 99),
            ("Summon", 5, 20),
            ("Mana", 30, 80),
            ("Holy Fire of Mithrandir", 30, 75),
        ] {
            let sp = gd
                .spells
                .iter()
                .find(|s| s.name == name && s.school == 0)
                .unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(sp.level, level, "{name}");
            assert_eq!(sp.fail, fail, "{name}");
        }
        // The Golden Horn recalls to the surface (spells3.cc).
        let tl = gd
            .spells
            .iter()
            .find(|s| s.name == "Artifact Thunderlords")
            .unwrap();
        assert_eq!(tl.kind, "thunderlords");
        assert!(!tl.targeted);
        assert_eq!(tl.charges, "3+d3");
        // The three restored base god spells.
        for (name, god, kind) in [
            ("Avatar", "Manwe", "manwe_avatar"),
            ("Tree Roots", "Yavanna", "yavanna_roots"),
            ("Uproot", "Yavanna", "yavanna_uproot"),
        ] {
            let sp = gd.spell_by_name(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(sp.god, god, "{name}");
            assert_eq!(sp.kind, kind, "{name}");
        }
    }

    #[test]
    fn spell_rows_carry_the_original_difficulty_and_mana_range() {
        let gd = load_game_data();
        // (name, school, skill_level, mana_min, mana_max, failure) from the
        // spells5.cc spell_type_set_difficulty/set_mana calls.
        for (name, school, level, mana, mana_max, fail) in [
            ("Fireflash", 3, 10, 5, 70, 35),
            ("Thunderstorm", 4, 25, 40, 60, 60),
            ("Tidal Wave", 5, 16, 16, 40, 65),
            ("Teleport", 1, 10, 8, 14, 30),
            ("Word of Recall", 11, 30, 25, 25, 60),
            ("Wraithform", 55, 30, 20, 40, 95),
            ("Ambarkanta(IV)", 100, 25, 70, 70, 60),
            ("Lay of Protection", 53, 35, 400, 400, 80),
            ("Uproot", 53, 35, 250, 350, 95),
            ("Demon Blade", 13, 1, 4, 44, 10),
        ] {
            let sp = gd
                .spells
                .iter()
                .find(|s| s.name == name && s.school == school)
                .unwrap_or_else(|| panic!("missing {name}/{school}"));
            assert_eq!(
                (sp.level, sp.mana, sp.mana_max, sp.fail),
                (level, mana, mana_max, fail),
                "{name}"
            );
        }
    }

    #[test]
    fn spell_mana_scales_between_the_original_min_and_max() {
        let gd = load_game_data();
        let mut ps = mage(&gd);
        let sp = gd.spell_by_name("Manathrust").unwrap().clone();
        // Mana 1 school level: the minimum (1).
        assert_eq!(spell_mana_no_inv(&ps, &sp), 1);
        // Mana 25 school levels: lua_get_level(2500, 1, 25, 1) = 12.
        ps.skills.insert(2, 25 * crate::skill::SKILL_STEP);
        assert_eq!(spell_mana_no_inv(&ps, &sp), 12);
    }

    #[test]
    fn random_spell_matches_get_random_spell() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        for _ in 0..50 {
            let sp = random_spell(&gd, crate::skill::SK_MAGIC, 50, &mut rng)
                .expect("a Magic spell fits level 50");
            assert_eq!(random_type(sp), crate::skill::SK_MAGIC);
            assert!(!sp.kind.is_empty());
        }
        // The 25% sacred branch of random books has real rows now
        // (the old school==SKILL_SPIRITUALITY comparison never matched).
        assert!(random_spell(&gd, crate::skill::SK_SPIRITUALITY, 25, &mut rng).is_some());
        // Every returned row carries the requested random type, and the
        // level gate uses rand_int(skill_level*3) < level (spells5.cc:58):
        // a level-0 request can never pass for a level >= 1 spell.
        for seed in 0..64u64 {
            let mut r = crate::rng::new_seeded_rng(seed);
            let sp = random_spell(&gd, crate::skill::SK_MAGIC, 1, &mut r).unwrap();
            assert_eq!(random_type(sp), crate::skill::SK_MAGIC);
        }
        let mut r = crate::rng::new_seeded_rng(7);
        assert!(random_spell(&gd, crate::skill::SK_MAGIC, 0, &mut r).is_none());
        // NO_RANDOM rows (device/demonology/geomancy) map to -1 in the
        // original, so even a wanted type of 0 must not match them.
        let mut r = crate::rng::new_seeded_rng(7);
        assert!(random_spell(&gd, 0, 50, &mut r).is_none());
    }

    /// random_type comes from the C++ registration, so port-only helper
    /// rows and cross-school duplicates never enter the pool
    /// (spells5.cc:58 / spell_type_init_*).
    #[test]
    fn random_pool_excludes_unregistered_and_duplicate_rows() {
        let gd = load_game_data();
        let by_name = |n: &str| gd.spells.iter().find(|s| s.name == n).unwrap();
        // Port-only helper rows are NO_RANDOM whichever school they use.
        for name in [
            "Cure Light Wounds",
            "Heal",
            "Detect Monsters",
            "Identify",
            "Haste",
            "Heroism",
            "Protection from Evil",
            "Resist Elements",
        ] {
            for sp in gd.spells.iter().filter(|s| s.name == name) {
                assert_eq!(random_type(sp), 0, "{name} (school {})", sp.school);
            }
        }
        // The registered mana-school row stays random; the Prayer duplicate
        // added for the helper lists does not.
        assert_eq!(random_type(by_name("Remove Curses")), crate::skill::SK_MAGIC);
        assert_eq!(
            random_type(
                gd.spells
                    .iter()
                    .find(|s| s.name == "Remove Curses" && s.school == 53)
                    .unwrap()
            ),
            0
        );
        // Device aliases (school 0) never enter even when the name is a
        // registered spell; the castable book names alias onto the
        // registration instead.
        assert_eq!(
            random_type(
                gd.spells
                    .iter()
                    .find(|s| s.name == "Teleportation")
                    .unwrap()
            ),
            0
        );
        assert_eq!(random_type(by_name("Teleport")), crate::skill::SK_MAGIC);
        assert_eq!(
            random_type(by_name("Word of Recall")),
            crate::skill::SK_MAGIC
        );
        // The whole registered corpus is exactly 57/34/13 random rows.
        let counts = gd.spells.iter().map(random_type).fold(
            (0, 0, 0, 0),
            |(m, s, mu, z), rt| match rt {
                crate::skill::SK_MAGIC => (m + 1, s, mu, z),
                crate::skill::SK_SPIRITUALITY => (m, s + 1, mu, z),
                crate::skill::SK_MUSIC => (m, s, mu + 1, z),
                _ => (m, s, mu, z + 1),
            },
        );
        assert_eq!(counts, (57, 34, 13, 87));
    }

    /// The per-spell `*_info` texts (spells3.cc) recomputed from the
    /// original formulas with the same `get_level_s` / `get_level_school`
    /// helpers the game uses.
    fn check_spell_info(
        name: &str,
        school: u32,
        skill: i32,
        want: &dyn Fn(
            &crate::game::PlayerState,
            &crate::item::Inventory,
            &GameData,
            &dyn Fn(i32) -> i32,
            &dyn Fn(i32) -> i32,
        ) -> String,
    ) {
        let gd = load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Mage").unwrap_or(0);
        let sp = gd
            .spell_by_name(name)
            .unwrap_or_else(|| panic!("missing spell {name}"))
            .clone();
        let mut ps = crate::birth::make_player(&gd, "I".into(), race, class);
        ps.skills.clear();
        ps.skills.insert(
            crate::skill::school_skill_id(school),
            skill * crate::skill::SKILL_STEP,
        );
        let inv = crate::item::Inventory::default();
        let ls = |max: i32| get_level_s(&ps, &inv, &gd, &sp, max);
        let l0 = |max: i32| get_level_school(&ps, &inv, &gd, &sp, max, 0).0;
        let want = want(&ps, &inv, &gd, &ls, &l0);
        assert_eq!(spell_info(&ps, &inv, &gd, &sp), want, "{name}");
    }

    #[test]
    fn spells3_info_strings_match_the_original_formulas() {
        check_spell_info("Noxious Cloud", 4, 50, &|_, _, _, ls, _| {
            format!("dam {} rad 3 dur {}", 7 + ls(150), 5 + ls(40))
        });
        check_spell_info("Wings of Winds", 4, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10", 5 + ls(25))
        });
        check_spell_info("Invisibility", 4, 50, &|_, _, _, ls, _| {
            format!("dur {}+d20 power {}", 15 + ls(50), 20 + ls(50))
        });
        check_spell_info("Poison Blood", 4, 50, &|_, _, _, ls, _| {
            format!("dur {}+d30", 25 + ls(25))
        });
        check_spell_info("Thunderstorm", 4, 50, &|_, _, _, ls, _| {
            format!(
                "dam {}d{} dur {}+d10",
                5 + ls(10),
                10 + ls(25),
                10 + ls(25)
            )
        });
        check_spell_info("Sterilize", 4, 50, &|_, _, _, ls, _| {
            format!("dur {}+d30", 20 + ls(70))
        });
        check_spell_info("Phase Door", 1, 50, &|_, _, _, ls, _| {
            format!("distance {}", 10 + ls(8))
        });
        check_spell_info("Teleport", 1, 50, &|_, _, _, ls, _| {
            format!("distance {}", 100 + ls(100))
        });
        check_spell_info("Probability Travel", 1, 50, &|_, _, _, ls, _| {
            format!("dur {}+d20", ls(60))
        });
        check_spell_info("Demon Blade", 13, 50, &|_, _, _, ls, _| {
            format!("dur {}+d20 dam {}/blow", ls(80), 4 + ls(40))
        });
        check_spell_info("Demon Madness", 13, 50, &|_, _, _, ls, l0| {
            format!("dam {} rad {}", 20 + ls(200), 1 + l0(4))
        });
        check_spell_info("Demon Field", 13, 50, &|_, _, _, ls, _| {
            format!("dam {} dur {}", 20 + ls(70), 30 + ls(100))
        });
        check_spell_info("Doom Shield", 13, 50, &|_, _, _, ls, _| {
            format!(
                "dur {}+d10 dam {}d{}",
                20 + ls(100),
                1 + ls(14),
                10 + ls(15)
            )
        });
        check_spell_info("Unholy Word", 13, 50, &|_, _, _, _, l0| {
            format!("heal mhp% of {}%", 30 + l0(50))
        });
        check_spell_info("Demon Cloak", 13, 50, &|_, _, _, _, l0| {
            format!("dur {}+d5", 5 + l0(15))
        });
        check_spell_info("Summon Demon", 13, 50, &|_, _, _, ls, _| {
            format!("level {}", 5 + ls(100))
        });
        check_spell_info("Discharge Minion", 13, 50, &|_, _, _, _, l0| {
            format!("dam {}% max {}", 20 + l0(60), 100 + l0(500))
        });
        check_spell_info("Control Demon", 13, 50, &|_, _, _, ls, _| {
            format!("power {}", 50 + ls(250))
        });
        check_spell_info("Reveal Ways", 10, 50, &|_, _, _, ls, _| {
            format!("rad {}", 10 + ls(40))
        });
        check_spell_info("Shake", 7, 50, &|_, _, _, ls, _| {
            format!("rad {}", 4 + ls(10))
        });
        check_spell_info("See the Music", 20, 50, &|_, _, _, ls, _| {
            format!("dur {}+d20", 10 + ls(100))
        });
        check_spell_info("Lay of Protection", 20, 50, &|_, _, _, _, l0| {
            format!("rad {}", 1 + l0(2))
        });
        check_spell_info("Fireflash", 3, 50, &|_, _, _, ls, _| {
            format!("dam {} rad {}", 20 + ls(500), 2 + ls(5))
        });
        check_spell_info("Fiery Shield", 3, 50, &|_, _, _, ls, _| {
            format!(
                "dam {}d{} dur {}+d20",
                5 + ls(15),
                5 + ls(7),
                10 + ls(70)
            )
        });
        check_spell_info("Firewall", 3, 50, &|_, _, _, ls, _| {
            format!("dam {} dur {}", 40 + ls(150), 10 + ls(14))
        });
        check_spell_info("Fire Golem", 52, 50, &|_, _, _, ls, _| {
            format!("golem level {}", 7 + ls(70))
        });
        check_spell_info("Elemental Shield", 2, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10", 15 + ls(50))
        });
        check_spell_info("Disruption Shield", 2, 50, &|_, _, _, ls, _| {
            format!("dur {}+d5", 3 + ls(10))
        });
        check_spell_info("Avatar", 21, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10", ls(20))
        });
        check_spell_info("Manwe's Blessing", 21, 50, &|_, _, _, ls, _| {
            format!("dur {}+d40", ls(70) + 30)
        });
        check_spell_info("Manwe's Call", 21, 50, &|_, _, _, ls, _| {
            format!("level {}", ls(70) + 20)
        });
        check_spell_info("Corpse Explosion", 53, 50, &|_, _, _, ls, _| {
            format!("dam {}%", 20 + ls(70))
        });
        check_spell_info("Mind Steal", 53, 50, &|_, _, _, ls, _| {
            format!("chance 1d(mlvl)<{}", ls(50))
        });
        check_spell_info("Recharge", 2, 50, &|_, _, _, ls, _| {
            format!("power {}", 60 + ls(140))
        });
        check_spell_info("Spellbinder", 2, 50, &|_, _, _, ls, _| {
            format!("number {} max level {}", ls(4).min(4), 7 + ls(35))
        });
        check_spell_info("Inertia Control", 2, 50, &|_, _, _, ls, _| {
            format!("level {}", ls(10))
        });
        check_spell_info("Charm", 51, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(150))
        });
        check_spell_info("Confuse", 51, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(150))
        });
        check_spell_info("Armor of Fear", 51, 50, &|_, _, _, ls, _| {
            format!(
                "dur {}+d10 power {}d{}",
                10 + ls(100),
                1 + ls(7),
                5 + ls(20)
            )
        });
        check_spell_info("Stun", 51, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(150))
        });
        check_spell_info("Essence of Speed", 11, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10 speed {}", 10 + ls(50), 5 + ls(20))
        });
        check_spell_info("Banishment", 1, 50, &|_, _, _, ls, _| {
            format!("power {}", 40 + ls(160))
        });
        check_spell_info("Divine Aim", 22, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10", ls(50))
        });
        check_spell_info("Wave of Power", 22, 50, &|ps, inv, gd, ls, _| {
            format!(
                "blows {}",
                ls(crate::game::num_blows(ps, inv, gd).max(1))
            )
        });
        check_spell_info("Wraithform", 55, 50, &|_, _, _, ls, _| {
            format!("dur {}+d30", 20 + ls(40))
        });
        check_spell_info("Flame of Udun", 55, 50, &|_, _, _, ls, _| {
            format!("dur {}+d15", 5 + ls(30))
        });
        check_spell_info("Tidal Wave", 5, 50, &|_, _, _, ls, _| {
            format!("dam {} dur {}", 40 + ls(200), 6 + ls(10))
        });
        check_spell_info("Ice Storm", 5, 50, &|_, _, _, ls, l0| {
            format!(
                "dam {} rad {} dur {}",
                80 + ls(200),
                1 + l0(3),
                20 + ls(70)
            )
        });
        check_spell_info("Vapor", 5, 50, &|_, _, _, ls, l0| {
            format!("dam {} rad {} dur 5", 3 + ls(20), 3 + l0(9))
        });
        check_spell_info("Charm Animal", 11, 50, &|_, _, _, ls, _| {
            format!("power {} rad {}", 10 + ls(170), ls(2))
        });
        check_spell_info("Grow Grass", 6, 50, &|_, _, _, ls, _| {
            format!("rad {}", ls(4))
        });
        check_spell_info("Tree Roots", 6, 50, &|_, _, _, ls, _| {
            format!(
                "dur {} AC {} dam {}",
                10 + ls(30),
                10 + ls(60),
                10 + ls(20)
            )
        });
        check_spell_info("Water Bite", 6, 50, &|_, _, _, ls, _| {
            format!("dur {}+d30 dam {}/blow", 30 + ls(150), 10 + ls(50))
        });
        check_spell_info("Uproot", 53, 50, &|_, _, _, ls, _| {
            format!("lev {}", 30 + ls(70))
        });
        check_spell_info("Grow Trees", 6, 50, &|_, _, _, ls, _| {
            format!("rad {}", 2 + ls(7))
        });
        check_spell_info("Healing", 6, 50, &|ps, _, _, ls, _| {
            let pct = 15 + ls(35);
            format!("heal {}% = {}hp", pct, ps.max_hp * pct / 100)
        });
        check_spell_info("Regeneration", 6, 50, &|_, _, _, ls, _| {
            format!("dur {}+d10 power {}", 5 + ls(50), 300 + ls(700))
        });
        check_spell_info("Summon Animal", 6, 50, &|_, _, _, ls, _| {
            format!("level {}", 25 + ls(50))
        });
        check_spell_info("Heal Monster", 0, 50, &|_, _, _, ls, _| {
            format!("heal {}", 20 + ls(380))
        });
        check_spell_info("Mana", 0, 50, &|_, _, _, ls, _| {
            format!("restore {}%", 20 + ls(50))
        });
        check_spell_info("Holy Fire of Mithrandir", 0, 50, &|_, _, _, ls, _| {
            format!("dam {}", 50 + ls(300))
        });
        check_spell_info("Holding Pattern(I)", 100, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(100))
        });
        check_spell_info("Illusion Pattern(II)", 100, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(100))
        });
        check_spell_info("Stun Pattern(IV)", 100, 50, &|_, _, _, ls, _| {
            format!("power {}", 10 + ls(90))
        });
        check_spell_info("Flow of Life(II)", 100, 50, &|_, _, _, ls, _| {
            format!("heal {}/turn", 7 + ls(100))
        });
        check_spell_info("Blow(I)", 100, 50, &|_, _, _, _, l0| {
            format!("dam {}d{} rad {}", 2 + l0(10), 4 + l0(40), 1 + l0(12))
        });
        check_spell_info("Gush of Wind(II)", 100, 50, &|_, _, _, _, l0| {
            format!("dist {} rad {}", 10 + l0(40), 1 + l0(12))
        });
        check_spell_info("Horns of Ylmir(III)", 100, 50, &|_, _, _, ls, _| {
            format!("rad {}", 2 + ls(10))
        });
        check_spell_info("Enchant Weapon", 53, 50, &|_, _, _, ls, _| {
            format!("tries {}", 1 + ls(50) / 12)
        });
        check_spell_info("Enchant Armour", 53, 50, &|_, _, _, ls, _| {
            format!("tries {}", 1 + ls(50) / 10)
        });
        check_spell_info("Child of Aule", 53, 50, &|_, _, _, ls, _| {
            format!("level {}", 20 + ls(70))
        });
        check_spell_info("Tears of Luthien", 53, 50, &|_, _, _, ls, _| {
            format!("heals {}", 10 * ls(30))
        });
        check_spell_info("Tale of Doom", 53, 50, &|_, _, _, ls, _| {
            format!("dur {}", 5 + ls(10))
        });
        check_spell_info("Call to the Halls", 53, 50, &|_, _, _, _, l0| {
            format!("level {}", 20 + l0(70))
        });
        check_spell_info("Draught of Ulmonan", 53, 50, &|_, _, _, ls, _| {
            format!("cure {}", 5 * ls(50))
        });
        check_spell_info("Call of the Ulumuri", 53, 50, &|_, _, _, _, l0| {
            format!("level {}", 30 + l0(70))
        });
        check_spell_info("Wrath of Ulmo", 53, 50, &|_, _, _, ls, _| {
            format!("dam {} dur {}", 40 + ls(150), 10 + ls(14))
        });
        check_spell_info("Star Kindler", 53, 50, &|ps, _, _, ls, _| {
            format!(
                "dam {} bursts {} rad 10",
                20 + ls(100),
                ps.level as i32 / 5
            )
        });
        check_spell_info("Recall", 11, 50, &|_, _, _, ls, _| {
            format!(
                "dur {}+d{} weight {}lb",
                (15 - ls(10)).max(1),
                (21 - ls(15)).max(0),
                1 + ls(15)
            )
        });
        check_spell_info("Manathrust", 2, 50, &|_, _, _, ls, _| {
            format!("dam {}d{}", 3 + ls(50), 1 + ls(20))
        });
        check_spell_info("Geyser", 5, 50, &|_, _, _, ls, _| {
            format!("dam {}d{}", ls(10), 3 + ls(35))
        });
        check_spell_info("Haste Monster", 0, 50, &|_, _, _, _, _| {
            "speed +10".to_string()
        });
        check_spell_info("Firebrand", 53, 50, &|_, _, _, ls, _| {
            format!("dur {}+d20 dam {}/blow", ls(50), 4 + ls(50))
        });
        check_spell_info("Song of Belegaer", 53, 50, &|_, _, _, ls, _| {
            format!("dam {}d{}", ls(10), 3 + ls(35))
        });
        check_spell_info("Wind Shield", 21, 50, &|_, _, _, ls, _| {
            let mut buf = format!("dur {}+d20", ls(50) + 10);
            if ls(50) >= 10 {
                buf += &format!(" AC {}", ls(30));
            }
            if ls(50) >= 20 {
                buf += &format!(" dam {}d{}", 1 + ls(2), 1 + ls(6));
            }
            buf
        });
    }

    #[test]
    fn spells3_info_gates_follow_the_original_thresholds() {
        // Conditional info strings (spells3.cc) at a skill below the
        // gate: the port must return the short form.
        check_spell_info("Sense Hidden", 10, 5, &|_, _, _, ls, _| {
            if ls(50) >= 15 {
                format!("rad {} dur {}+d20", 15 + ls(40), 10 + ls(40))
            } else {
                format!("rad {}", 15 + ls(40))
            }
        });
        check_spell_info("Sense Monsters", 10, 5, &|_, _, _, ls, _| {
            if ls(50) >= 30 {
                format!("rad {} dur {}+d10", 10 + ls(40), 10 + ls(20))
            } else {
                format!("rad {}", 10 + ls(40))
            }
        });
        check_spell_info("Stone Skin", 7, 5, &|_, _, _, ls, _| {
            if ls(50) >= 25 {
                format!(
                    "dam {}d{} dur {}+d10 AC {}",
                    2 + ls(5),
                    3 + ls(5),
                    10 + ls(100),
                    10 + ls(50)
                )
            } else {
                format!("dur {}+d10 AC {}", 10 + ls(100), 10 + ls(50))
            }
        });
        check_spell_info("Strike", 7, 5, &|_, _, _, ls, _| {
            if ls(50) >= 12 {
                format!("dam {} rad 1", 50 + ls(50))
            } else {
                format!("dam {}", 50 + ls(50))
            }
        });
        check_spell_info("Globe of Light", 3, 5, &|_, _, _, ls, _| {
            if ls(50) >= 15 {
                format!("dam {} rad {}", 10 + ls(100), 5 + ls(6))
            } else {
                String::new()
            }
        });
        check_spell_info("Ent's Potion", 5, 5, &|_, _, _, ls, _| {
            if ls(50) >= 12 {
                format!("dur {}+d25", 25 + ls(40))
            } else {
                String::new()
            }
        });
        check_spell_info("Hobbit Melodies(III)", 100, 5, &|_, _, _, ls, _| {
            if ls(50) >= 15 {
                format!("AC {} speed {}", 10 + ls(50), 7 + ls(10))
            } else {
                format!("AC {}", 10 + ls(50))
            }
        });
        check_spell_info("Clairaudience(IV)", 100, 5, &|_, _, _, ls, l0| {
            if ls(50) >= 10 {
                format!("rad {}", 1 + l0(3))
            } else {
                String::new()
            }
        });
        check_spell_info("Feanturi", 53, 5, &|_, _, _, ls, _| {
            let level = ls(50);
            if level >= 20 {
                format!("heals {}%", level)
            } else {
                String::new()
            }
        });
        check_spell_info("Light of Valinor", 53, 5, &|_, _, _, ls, _| {
            if ls(50) >= 15 {
                format!("dam {} rad {}", 10 + ls(100), 5 + ls(6))
            } else {
                String::new()
            }
        });
        check_spell_info("Slow Monster", 11, 50, &|_, _, _, ls, _| {
            if ls(50) >= 20 {
                format!("power {} rad 1", 40 + ls(160))
            } else {
                format!("power {}", 40 + ls(160))
            }
        });
    }

}
