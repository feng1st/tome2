//! Game data loaded from RON assets converted from ToME's lib/edit/*.txt files.
// Some struct fields are carried for data completeness but not (yet) read.
#![allow(dead_code)]

use bevy::prelude::Resource;
use rand::Rng;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TerrainDef {
    pub id: u16,
    pub name: String,
    pub ch: String,
    pub color: u8,
    pub is_floor: bool,
    pub no_walk: bool,
    pub no_vision: bool,
    pub is_wall: bool,
    pub tunnelable: bool,
    pub permanent: bool,
    /// Stays lit through the night (f_info F:REMEMBER, src/wild.cc).
    #[serde(default)]
    pub remember: bool,
    /// Terrain a Grow Trees spell can turn into forest
    /// (f_info F:SUPPORT_GROWTH).
    #[serde(default)]
    pub support_growth: bool,
    /// Flight / levitation / climbing / pass-wall entry permission
    /// (f_info F:CAN_*; cmd1.cc player_can_enter).
    #[serde(default)]
    pub can_fly: bool,
    #[serde(default)]
    pub can_levitate: bool,
    #[serde(default)]
    pub can_climb: bool,
    #[serde(default)]
    pub can_pass: bool,
    /// Spider webs: only spiders may enter (f_info F:WEB;
    /// cmd1.cc player_can_enter).
    #[serde(default)]
    pub web: bool,
    /// Run/travel may cross this terrain (f_info F:CAN_RUN).
    #[serde(default)]
    pub can_run: bool,
    /// Noticed while walking (f_info F:NOTICE).
    #[serde(default)]
    pub notice: bool,
    /// Not noticed while running (f_info F:DONT_NOTICE_RUNNING).
    #[serde(default)]
    pub dont_notice_running: bool,
    /// A door feature (f_info F:DOOR).
    #[serde(default)]
    pub door: bool,
    /// Allows a lite source to provide permanent illumination
    /// (f_info F:SUPPORT_LIGHT, terrain glow handling).
    #[serde(default)]
    pub support_light: bool,
    /// Multi-hued, redrawn with a random colour (f_info F:ATTR_MULTI).
    #[serde(default)]
    pub attr_multi: bool,
    /// D:0 look description.
    #[serde(default)]
    pub desc: String,
    /// D:1 tunneling message.
    #[serde(default)]
    pub tunnel_desc: String,
    /// D:2 blocking message.
    #[serde(default)]
    pub block_desc: String,
    /// f_info M:<id>: the feature this one is displayed as (default:
    /// itself, init1.cc init_f_info_txt).
    #[serde(default)]
    pub mimic: u16,
    /// f_info E:<dd>d<ds>:<freq>:<type> damage effects (freq is x10).
    #[serde(default)]
    pub effects: Vec<TerrainEffectDef>,
}

/// One f_info `E:` terrain damage effect (init1.cc init_f_info_txt).
#[derive(Debug, Clone, Deserialize)]
pub struct TerrainEffectDef {
    #[serde(default)]
    pub dd: i32,
    #[serde(default)]
    pub ds: i32,
    #[serde(default)]
    pub freq: i32,
    /// GF_* name or numeric code, exactly as written in the file.
    #[serde(default)]
    pub typ: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlowDef {
    pub method: String,
    pub effect: String,
    pub dice: String,
}

fn default_weight() -> u32 {
    100
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonsterDef {
    pub id: u32,
    pub name: String,
    pub ch: String,
    pub color: u8,
    pub speed: i32,
    pub hp: String,
    /// Hit-dice count/sides (r_info I: field 2).
    #[serde(default)]
    pub hdice: String,
    #[serde(default)]
    pub hside: String,
    pub ac: i32,
    pub alert: i32,
    /// r_info I: field 3; hearing range in tens of feet (recall text).
    #[serde(default)]
    pub aaf: i32,
    pub depth: u32,
    pub rarity: u32,
    /// Corpse weight (r_info W: 3rd field; 0 treated as 100).
    #[serde(default = "default_weight")]
    pub weight: u32,
    pub exp: u32,
    pub blows: Vec<BlowDef>,
    /// Innate spell frequency: casts 1-in-N actions (0 = never).
    pub spell_freq: i32,
    /// Innate spell flags (r_info S: lines).
    pub spells: Vec<String>,
    pub open_door: bool,
    pub bash_door: bool,
    pub never_move: bool,
    pub unique: bool,
    /// Spawns with a few companions of the same kind.
    pub friends: bool,
    /// Spawns with a small escort of depth-appropriate monsters.
    pub escort: bool,
    /// r_info A: standard artifact drop (a_info id; 0 = none).
    #[serde(default)]
    pub artifact_idx: u32,
    /// Percent chance of the standard artifact drop.
    #[serde(default)]
    pub artifact_chance: u32,
    /// Monster drop theme O:<treasure>:<combat>:<magic>:<tools>
    /// (object2.cc kind_is_theme).
    #[serde(default)]
    pub objs: (u32, u32, u32, u32),
    /// Full r_info F: flag set (INVISIBLE, race flags, IM_*, auras, ...).
    pub flags: Vec<String>,
    /// r_info E: body parts (weapon/torso/arms/finger/head/legs;
    /// xtra1.cc calc_body for possessed bodies).
    #[serde(default)]
    pub body_parts: [i32; 6],
    /// r_info D: monster memory/recall text (joined exactly as the
    /// original strappend()es the lines).
    #[serde(default)]
    pub desc: String,
}

impl MonsterDef {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    /// The glyph used for race/summon pools (r_info G:).
    pub fn glyph(&self) -> char {
        self.ch.chars().next().unwrap_or('?')
    }
}

/// A race/class skill modifier (p_info G:k:/R:k:/C:k: lines).
/// `bop`/`mop` are the operator characters '+' '-' '=' '%'.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMod {
    pub skill: String,
    pub bop: String,
    pub base: i32,
    pub mop: String,
    pub gain: i32,
}

/// A level-granted ability (p_info R:b:/C:b: lines).
#[derive(Debug, Clone, Deserialize)]
pub struct SkillGrant {
    pub level: u32,
    pub ability: String,
}

/// A level-gated intrinsic flag from p_info R:F:/C:F: (xtra1.cc
/// apply_lflags): applied with `pval` once the character reaches `level`.
#[derive(Debug, Clone, Deserialize)]
pub struct LevelFlag {
    pub level: u32,
    pub pval: i32,
    pub flag: String,
}

/// A starting item proto (p_info R:O:/C:O:/C:a:O:): tval:sval:pval:xdy
/// (init1.cc read_proto_object); `number` is rolled as `dd`d`ds`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ObjectProto {
    pub tval: i32,
    pub sval: i32,
    #[serde(default)]
    pub pval: i32,
    pub dd: i32,
    pub ds: i32,
}

/// One class specialisation (p_info C:a: record). The player's final
/// class identity is the spec title (src/birth.cc spp_ptr->title).
#[derive(Debug, Clone, Deserialize)]
pub struct SpecDef {
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub objects: Vec<ObjectProto>,
    /// C:a:g: lines; "All Gods" as a single entry means every deity.
    #[serde(default)]
    pub gods: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SkillMod>,
    #[serde(default)]
    pub abilities: Vec<SkillGrant>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaceDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    pub stats: [i32; 6],
    pub mana: i32,
    /// p_info R:S: luck bonus (7th field).
    #[serde(default)]
    pub luck: i32,
    pub hitdie: u32,
    pub exp: u32,
    /// p_info R:P: infra-vision range.
    #[serde(default)]
    pub infra: i32,
    pub classes: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SkillMod>,
    #[serde(default)]
    pub abilities: Vec<SkillGrant>,
    /// p_info R:F: intrinsic flags (RES_*, SUST_*, ESP_*, IMMOVABLE...).
    #[serde(default)]
    pub flags: Vec<LevelFlag>,
    /// p_info R:O: starting item protos.
    #[serde(default)]
    pub objects: Vec<ObjectProto>,
    /// p_info R:Z: powers granted (calc_powers).
    #[serde(default)]
    pub powers: Vec<String>,
    /// p_info R:G: player flags (GOD_FRIEND, NO_GOD...).
    #[serde(default)]
    pub player_flags: Vec<String>,
    /// p_info R:E: body parts (weapon/torso/arms/finger/head/legs;
    /// xtra1.cc calc_body).
    #[serde(default)]
    pub body_parts: [i32; 6],
}

/// A base subrace / race modifier (p_info S: records, init1.cc
/// race_mod_info).  Base data ships nine records (Normal plus
/// Vampire/Spectre/Skeleton/Zombie/Barbarian/Hermit/LostSoul and one
/// placeholder); the Theme module's records are not ported.
#[derive(Debug, Clone, Deserialize)]
pub struct RaceModDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    /// S:D:'A' -> "Race Subrace", otherwise "Subrace Race".
    pub place: bool,
    pub stats: [i32; 6],
    pub luck: i32,
    /// Mana multiplier in percent (100 = unchanged).
    pub mana: i32,
    /// Added to the race/class hit die.
    pub hitdie: i32,
    /// Added to the race/class experience factor.
    pub exp: i32,
    /// p_info S:P: infra-vision range.
    #[serde(default)]
    pub infra: i32,
    /// S:E: extra body parts (weapons/torso/arms/fingers/head/legs).
    #[serde(default)]
    pub body_parts: [i32; 6],
    /// S:A: races this modifier can apply to.
    pub races: Vec<String>,
    /// S:C:A: classes this modifier opens up.
    #[serde(default)]
    pub classes: Vec<String>,
    /// S:C:F: classes this modifier forbids.
    #[serde(default)]
    pub forbidden_classes: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SkillMod>,
    #[serde(default)]
    pub abilities: Vec<SkillGrant>,
    #[serde(default)]
    pub flags: Vec<LevelFlag>,
    #[serde(default)]
    pub objects: Vec<ObjectProto>,
    /// p_info S:Z: powers granted (calc_powers).
    #[serde(default)]
    pub powers: Vec<String>,
    #[serde(default)]
    pub player_flags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    pub stats: [i32; 6],
    pub mana: i32,
    pub hitdie: u32,
    pub exp: u32,
    /// Bonus melee blows per round (p_info C:S).
    pub blows: i32,
    /// Melee blow parameters (p_info C:B:num:weight:mul).
    pub blow_num: i32,
    pub blow_wgt: i32,
    pub blow_mul: i32,
    #[serde(default)]
    pub skills: Vec<SkillMod>,
    #[serde(default)]
    pub abilities: Vec<SkillGrant>,
    /// p_info C:F: intrinsic flags (CRIT, RES_FEAR...).
    #[serde(default)]
    pub flags: Vec<LevelFlag>,
    /// p_info C:O: starting item protos.
    #[serde(default)]
    pub objects: Vec<ObjectProto>,
    /// p_info C:g: class god restriction (empty = none from the class).
    #[serde(default)]
    pub gods: Vec<String>,
    /// The C:a: specialisations (in file order).
    #[serde(default)]
    pub specs: Vec<SpecDef>,
    /// p_info C:Z: powers granted (calc_powers).
    #[serde(default)]
    pub powers: Vec<String>,
    /// p_info C:G: player flags (GOD_FRIEND, EASE_STEAL...).
    #[serde(default)]
    pub player_flags: Vec<String>,
    /// p_info C:D:1: level titles, one per five levels
    /// (files.cc class titles).
    #[serde(default)]
    pub titles: Vec<String>,
    /// p_info C:E: body parts (weapon/torso/arms/finger/head/legs).
    #[serde(default)]
    pub body_parts: [i32; 6],
}

/// One skill of the s_info tree (skills.ron).
#[derive(Debug, Clone, Deserialize)]
pub struct SkillDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    /// Parent skill id (-1 = root).
    pub father: i32,
    /// Order in the tree (T: lines; 0 = unplaced).
    pub order: u32,
    pub flags: Vec<String>,
    pub action_mkey: i32,
    pub action_desc: String,
    /// Lost Sword random-gain weight (G: line, default 100).
    pub chance: u32,
    pub increases: Vec<(u32, u32)>,
    pub excludes: Vec<u32>,
}

impl SkillDef {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

/// An ability (abilities.ron, ab_info.txt).
#[derive(Debug, Clone, Deserialize)]
pub struct AbilityDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    pub cost: i32,
    pub action_mkey: i32,
    pub action_desc: String,
    pub need_skills: Vec<(String, u32)>,
    /// Required stat indexes (index = 3-5 below means "stat_index-3").
    pub stats: [i32; 6],
    pub need_abilities: Vec<String>,
}

/// A building action (building_actions.ron, ba_info.txt).
#[derive(Debug, Clone, Deserialize)]
pub struct BuildingActionDef {
    pub id: u32,
    pub name: String,
    /// (hated, normal, liked) prices.
    pub costs: (i32, i32, i32),
    /// BACT_* action code.
    pub action: i32,
    /// 0 = any, 1 = normal+liked, 2 = liked only.
    pub restr: i32,
    pub letter: String,
    pub letter_aux: String,
}

// tvals from src/defines.hpp (only the ones we port)
pub const TV_SKELETON: i32 = 1;
pub const TV_BOTTLE: i32 = 2;
pub const TV_SPIKE: i32 = 5;
pub const TV_MSTAFF: i32 = 6;
pub const TV_PARCHMENT: i32 = 8;
pub const TV_CORPSE: i32 = 9;
pub const TV_EGG: i32 = 10;
pub const TV_JUNK: i32 = 11;
pub const TV_TOOL: i32 = 12;
pub const TV_INSTRUMENT: i32 = 14;
pub const TV_BOOMERANG: i32 = 15;
pub const TV_SHOT: i32 = 16;
pub const TV_ARROW: i32 = 17;
pub const TV_BOLT: i32 = 18;
pub const TV_BOW: i32 = 19;
pub const TV_DIGGING: i32 = 20;
pub const TV_HAFTED: i32 = 21;
pub const TV_POLEARM: i32 = 22;
pub const TV_SWORD: i32 = 23;
pub const TV_AXE: i32 = 24;
pub const TV_BOOTS: i32 = 30;
pub const TV_GLOVES: i32 = 31;
pub const TV_HELM: i32 = 32;
pub const TV_CROWN: i32 = 33;
pub const TV_SHIELD: i32 = 34;
pub const TV_CLOAK: i32 = 35;
pub const TV_SOFT_ARMOR: i32 = 36;
pub const TV_HARD_ARMOR: i32 = 37;
pub const TV_DRAG_ARMOR: i32 = 38;
pub const TV_LITE: i32 = 39;
pub const TV_AMULET: i32 = 40;
pub const TV_RING: i32 = 45;
pub const TV_TOTEM: i32 = 54;
pub const TV_STAFF: i32 = 55;
pub const TV_WAND: i32 = 65;
pub const TV_ROD: i32 = 66;
pub const TV_ROD_MAIN: i32 = 67;
pub const TV_SCROLL: i32 = 70;
pub const TV_POTION: i32 = 71;
pub const TV_POTION2: i32 = 72;
pub const TV_FLASK: i32 = 77;
pub const TV_FOOD: i32 = 80;
pub const TV_HYPNOS: i32 = 99;
pub const TV_GOLD: i32 = 100;
pub const TV_RANDART: i32 = 102;
pub const TV_BOOK: i32 = 111;
pub const TV_SYMBIOTIC_BOOK: i32 = 112;
pub const TV_MUSIC_BOOK: i32 = 113;
pub const TV_DRUID_BOOK: i32 = 114;
pub const TV_DAEMON_BOOK: i32 = 115;

/// Equipment slots.
pub const SLOT_WEAPON: usize = 0;
pub const SLOT_BODY: usize = 1;
pub const SLOT_HEAD: usize = 2;
pub const SLOT_HANDS: usize = 3;
pub const SLOT_FEET: usize = 4;
pub const SLOT_SHIELD: usize = 5;
pub const SLOT_CLOAK: usize = 6;
pub const SLOT_LITE: usize = 7;
pub const SLOT_BOW: usize = 8;
pub const SLOT_AMULET: usize = 9;
pub const SLOT_RING1: usize = 10;
pub const SLOT_RING2: usize = 11;
/// The symbiotic monster worn in symbiosis (INVEN_CARRY).
pub const SLOT_SYMBIOTE: usize = 12;
/// The quiver: ammunition worn for firing (INVEN_AMMO).
pub const SLOT_QUIVER: usize = 13;
pub const SLOT_TOOL: usize = 18;
/// Extra body parts from mimicry (xtra1.cc calc_body_bonus): extra arms
/// add a second weapon, shield and gloves slot; extra legs a second
/// boots slot.  Their items only count while the shape lasts.
pub const SLOT_WEAPON2: usize = 14;
pub const SLOT_SHIELD2: usize = 15;
pub const SLOT_HANDS2: usize = 16;
pub const SLOT_FEET2: usize = 17;
pub const NUM_SLOTS: usize = 19;

pub const SLOT_NAMES: [&str; NUM_SLOTS] = [
    "Weapon", "Body", "Head", "Hands", "Feet", "Shield", "Cloak", "Light", "Bow", "Amulet",
    "Ring(L)", "Ring(R)", "Symbiote", "Quiver", "Weapon 2", "Shield 2", "Gloves 2", "Boots 2",
    "Tool",
];

/// Which equipment slot an object tval goes into (None = not wearable).
/// Rings report SLOT_RING1; wielding picks the free finger (see
/// modal::wield_item).
pub fn slot_of(tval: i32) -> Option<usize> {
    Some(match tval {
        TV_HAFTED | TV_POLEARM | TV_SWORD | TV_AXE | TV_MSTAFF => SLOT_WEAPON,
        // Original wield_slot: instruments and boomerangs go in the bow
        // slot, digging tools and climbing gear in the tool slot.
        TV_INSTRUMENT | TV_BOOMERANG | TV_BOW => SLOT_BOW,
        TV_DIGGING | TV_TOOL => SLOT_TOOL,
        TV_SOFT_ARMOR | TV_HARD_ARMOR | TV_DRAG_ARMOR => SLOT_BODY,
        TV_HELM | TV_CROWN => SLOT_HEAD,
        TV_GLOVES => SLOT_HANDS,
        TV_BOOTS => SLOT_FEET,
        TV_SHIELD => SLOT_SHIELD,
        TV_CLOAK => SLOT_CLOAK,
        TV_LITE => SLOT_LITE,
        TV_AMULET => SLOT_AMULET,
        TV_RING => SLOT_RING1,
        TV_HYPNOS => SLOT_SYMBIOTE,
        // Ammunition lives in the quiver (object1.cc wield_slot).
        TV_SHOT | TV_ARROW | TV_BOLT => SLOT_QUIVER,
        _ => return None,
    })
}

/// The extra body-part slot an item tval can use when mimicry grants
/// extra limbs (mimic.rs::slot_usable decides whether the player has
/// it; xtra1.cc calc_body_bonus).
pub fn second_slot_of(tval: i32) -> Option<usize> {
    Some(match tval {
        TV_HAFTED | TV_POLEARM | TV_SWORD | TV_AXE | TV_MSTAFF => SLOT_WEAPON2,
        TV_SHIELD => SLOT_SHIELD2,
        TV_GLOVES => SLOT_HANDS2,
        TV_BOOTS => SLOT_FEET2,
        _ => return None,
    })
}

/// The slot a daemon book occupies (object1.cc wield_slot): the
/// Demonblade is a weapon, the Demonshield a shield, the Demonhorn a
/// helm. Their sval picks the slot; all three must be worn to cast
/// from (WIELD_CAST).
pub fn daemon_book_slot(sval: i32) -> Option<usize> {
    Some(match sval {
        55 => SLOT_WEAPON,
        56 => SLOT_SHIELD,
        57 => SLOT_HEAD,
        _ => return None,
    })
}

/// Which slot an object kind occupies, including the sval-keyed daemon
/// books. Prefer this over `slot_of` whenever the def is at hand.
pub fn slot_of_item(o: &ObjectDef) -> Option<usize> {
    if o.tval == TV_DAEMON_BOOK {
        daemon_book_slot(o.sval)
    } else {
        slot_of(o.tval)
    }
}

/// Ammunition tvals (stackable, fired from a bow; boomerangs are thrown).
pub fn is_ammo(tval: i32) -> bool {
    matches!(tval, TV_SHOT | TV_ARROW | TV_BOLT | TV_BOOMERANG)
}

/// Bow sval -> (damage multiplier, ammo tval it fires).
pub fn bow_info(sval: i32) -> Option<(i32, i32)> {
    Some(match sval {
        2 => (2, TV_SHOT),
        12 => (2, TV_ARROW),
        13 => (3, TV_ARROW),
        23 => (3, TV_BOLT),
        24 => (4, TV_BOLT),
        _ => return None,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectDef {
    pub id: u32,
    pub name: String,
    pub ch: String,
    pub color: u8,
    pub tval: i32,
    pub sval: i32,
    pub pval: i32,
    pub fuel: i32,
    /// Spell cast by devices (wands/staffs), empty for non-devices.
    pub spell: String,
    pub depth: u32,
    pub rarity: u32,
    pub weight: i32,
    pub cost: i32,
    pub ac: i32,
    pub dice: String,
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    /// Light radius granted when wielded in the light slot (0/1/2).
    pub lite: u8,
    /// a: activation name (base-item powers like the Ring of Flames).
    #[serde(default)]
    pub activate: String,
    /// k_info T:<btval>:<bsval> (the specific-object NORM_ART marker).
    #[serde(default)]
    pub btval: i32,
    #[serde(default)]
    pub bsval: i32,
    /// k_info A: allocation rows: (locale level, chance); prob = 100/chance.
    #[serde(default)]
    pub alloc: Vec<(u32, u32)>,
    /// k_info F: flags (ring/amulet powers, EASY_KNOW, IGNORE_*, ...).
    #[serde(default)]
    pub flags: Vec<String>,
    /// k_info D: description lines (parchment texts, lore).
    #[serde(default)]
    pub desc: Vec<String>,
}

impl ObjectDef {
    pub fn glyph(&self) -> char {
        self.ch.chars().next().unwrap_or('?')
    }

    pub fn wearable(&self) -> bool {
        slot_of_item(self).is_some()
    }

    /// object1.cc object_easy_know: kinds known on sight (aware without
    /// identification).
    pub fn easy_know(&self) -> bool {
        match self.tval {
            // Three spellbook tvals (object1.cc) are known on sight.
            TV_DRUID_BOOK | TV_MUSIC_BOOK | TV_SYMBIOTIC_BOOK => true,
            TV_FOOD | TV_POTION | TV_POTION2 | TV_SCROLL | TV_ROD | TV_ROD_MAIN => {
                !self.flags.iter().any(|f| f == "NORM_ART")
            }
            TV_FLASK | TV_EGG | TV_BOTTLE | TV_SKELETON | TV_CORPSE | TV_HYPNOS | TV_SPIKE
            | TV_JUNK => true,
            TV_RING | TV_AMULET | TV_LITE => self.flags.iter().any(|f| f == "EASY_KNOW"),
            _ => false,
        }
    }

    /// Base description with static bonuses (used for base/known items).
    pub fn label(&self) -> String {
        let mut s = self.name.clone();
        if slot_of_item(self) == Some(SLOT_WEAPON) && self.dice != "0d0" {
            s.push_str(&format!(" ({})", self.dice));
        } else if is_ammo(self.tval) && self.dice != "0d0" {
            s.push_str(&format!(" ({})", self.dice));
        } else if self.ac > 0 {
            s.push_str(&format!(" [{}]", self.ac));
        }
        if self.to_h != 0 || self.to_d != 0 {
            s.push_str(&format!(" ({},{})", signed(self.to_h), signed(self.to_d)));
        }
        if self.to_a != 0 {
            s.push_str(&format!(" [{}]", signed(self.to_a)));
        }
        s
    }
}

fn signed(v: i32) -> String {
    if v >= 0 {
        format!("+{}", v)
    } else {
        v.to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoreEntry {
    pub proba: u32,
    /// Item name (k_info) for I: entries; empty for T: entries.
    pub name: String,
    pub tval: i32,
    pub sval: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoreDef {
    pub id: u32,
    pub name: String,
    pub ch: String,
    pub color: u8,
    pub max_items: u32,
    pub flags: Vec<String>,
    /// Candidate owner indices (st_info O: lines).
    #[serde(default)]
    pub owners: Vec<u32>,
    /// ba_info indices offered by this store (st_info A: line).
    #[serde(default)]
    pub actions: Vec<u32>,
    pub entries: Vec<StoreEntry>,
}

impl StoreDef {
    pub fn glyph(&self) -> char {
        self.ch.chars().next().unwrap_or('1')
    }

    pub fn is_home(&self) -> bool {
        self.name == "Home"
    }

    pub fn is_black_market(&self) -> bool {
        self.name == "Black Market"
    }
}

/// An ego-item type (e_info.txt), applied to matching base gear.
#[derive(Debug, Clone, Deserialize)]
pub struct EgoDef {
    pub id: u32,
    pub name: String,
    /// True: name goes before the base name ("Elven Plate Mail").
    pub prefix: bool,
    pub tvals: Vec<i32>,
    /// (tval, min_sval, max_sval) rows (e_info T:).
    #[serde(default)]
    pub svals: Vec<(i32, i32, i32)>,
    pub depth: u32,
    pub rarity: u32,
    /// e_info W: 2nd field; the `rand_int(mrarity) > rarity1` roll.
    #[serde(default = "default_rarity1")]
    pub rarity1: u32,
    pub cost: i32,
    /// Max random bonuses rolled at creation.
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    pub pval: i32,
    pub cursed: bool,
    /// e_info X: 3rd field: the generate.cc `rating` bonus of the ego.
    #[serde(default)]
    pub rating: i32,
    /// e_info a: activation.
    #[serde(default)]
    pub activate: String,
    /// e_info r:N: required base flags.
    #[serde(default)]
    pub need_flags: Vec<String>,
    /// e_info r:F: forbidden base flags.
    #[serde(default)]
    pub forbid_flags: Vec<String>,
    /// e_info R: rarity groups: (chance, real flags, generation flags).
    #[serde(default)]
    pub groups: Vec<EgoFlagGroup>,
    /// e_info F: flags of the always-applied groups (magik(100)) plus
    /// the generation flag names; the rolled rarity groups are in
    /// `groups` and are copied onto the item at creation.
    pub flags: Vec<String>,
    /// e_info Z: powers granted while worn (calc_powers).
    #[serde(default)]
    pub powers: Vec<String>,
}

pub fn default_rarity1() -> u32 {
    1
}

pub fn default_artifact_weight() -> i32 {
    -1
}

/// One e_info `R:` rarity group: applied with `magik(chance)`; the
/// generation flags drive `add_random_ego_flag`.
#[derive(Debug, Clone, Deserialize)]
pub struct EgoFlagGroup {
    pub chance: u32,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub fego: Vec<String>,
    /// e_info f: obvious flags of this rarity group (oflags).
    #[serde(default)]
    pub oflags: Vec<String>,
}

/// An artifact blueprint (a_info.txt) overlaid on a base object.
#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactDef {
    pub id: u32,
    /// Name suffix (e.g. "of Galadriel").
    pub name: String,
    pub tval: i32,
    pub sval: i32,
    /// Stat/speed bonus granted while worn (I: line).
    pub pval: i32,
    pub depth: u32,
    pub rarity: u32,
    /// a_info W: 3rd field: the artifact's fixed weight (-1 = base kind,
    /// only for saves/ron predating the field).
    #[serde(default = "default_artifact_weight")]
    pub weight: i32,
    pub cost: i32,
    pub ac: i32,
    pub dice: String,
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    pub cursed: bool,
    /// INSTA_ART: quest/script-only; never generated randomly.
    #[serde(default)]
    pub insta_art: bool,
    /// a: activation name (empty = not activatable).
    pub activate: String,
    /// a_info F: flags (stat/resist/speed/ignore abilities).
    pub flags: Vec<String>,
    /// a_info Z: powers granted while worn (calc_powers).
    #[serde(default)]
    pub powers: Vec<String>,
}

/// A magic school (s_info.txt).
#[derive(Debug, Clone, Deserialize)]
pub struct SchoolDef {
    pub id: u32,
    pub name: String,
    /// s_info A:17: the school can be used to cast spells.
    #[serde(default)]
    pub cast: bool,
}

/// A spell (curated table; see convert_data.py).
#[derive(Debug, Clone, Deserialize)]
pub struct SpellRow {
    pub name: String,
    pub school: u32,
    /// The original `spell_type_skill_level` (spells5.cc
    /// `spell_type_set_difficulty` first argument): both the spell's
    /// position on the school-level curve (`lua_get_level`) and the
    /// effective school level needed to cast it (`is_ok_spell`).
    pub level: u32,
    /// Minimum mana cost (`spell_type_set_mana` first argument).
    pub mana: i32,
    /// Maximum mana cost (second argument of `spell_type_set_mana`;
    /// the cost scales between `mana` and `mana_max` with the caster's
    /// effective level in the spell, lua_bind.cc `get_mana`). Older
    /// rows without the field use `mana` for both.
    #[serde(default)]
    pub mana_max: i32,
    /// Base failure rate (`spell_type_set_difficulty` second argument).
    pub fail: i32,
    /// bolt / heal / teleport / light / detect / identify /
    /// remove_curse / recall / dig / teleport_away
    pub kind: String,
    /// Dice expression or teleport range, depending on kind.
    pub arg: String,
    pub targeted: bool,
    /// Element of elemental rows (FIRE/COLD/ELEC/ACID/POIS; "" = untyped).
    pub elem: String,
    /// God a Prayer spell belongs to ("" = generic).
    pub god: String,
    /// `spell_type_random_type` (spells5.cc): the skill whose random books
    /// may roll this spell (SK_MAGIC/SK_SPIRITUALITY/SK_MUSIC);
    /// 0 = NO_RANDOM. Written by the converter from the `init_*` call, so
    /// port-only helper rows and the school-0 device aliases never enter
    /// `get_random_spell`'s pool (spells5.cc:58).
    #[serde(default)]
    pub random_type: u32,
    /// Device charges dice (spells5.cc set_device_charges; "" = not a
    /// device spell).
    #[serde(default)]
    pub charges: String,
    /// Device allocations (spells5.cc device_allocation):
    /// (tval, rarity, base_min, base_max, max_min, max_max).
    #[serde(default)]
    pub alloc: Vec<DeviceAllocation>,
    /// `spell_type_describe` registration lines (spells5.cc): the static
    /// help text shown by the spell browser / `print_spell_desc`.
    #[serde(default)]
    pub desc: Vec<String>,
}

/// One cell of a fixed map (Bree, quest levels): terrain id + lit flag.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FixedCell {
    pub t: u16,
    pub l: bool,
}

/// A shop entrance in the fixed Bree map.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BreeShop {
    pub x: i32,
    pub y: i32,
    pub store: u32,
}

/// A quest entrance in the fixed Bree map.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BreeEntrance {
    pub x: i32,
    pub y: i32,
    pub quest: u32,
}

/// A tree cell that becomes a quest entrance under a runtime condition
/// (troll glade: quest taken AND night; wights grave: quest taken).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BreeQuestCell {
    pub x: i32,
    pub y: i32,
    pub quest: u32,
    pub night_only: bool,
}

/// A flavor building in the fixed Bree map (hook plot buildings; entering
/// just shows a line of text).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BreeBuilding {
    pub x: i32,
    pub y: i32,
    pub special: u32,
}

/// The fixed Bree village map (t_info.txt via the pref parser), cropped
/// to MAP_W x MAP_H.
#[derive(Debug, Clone, Deserialize)]
pub struct BreeDef {
    pub cells: Vec<FixedCell>,
    pub shops: Vec<BreeShop>,
    pub entrances: Vec<BreeEntrance>,
    pub buildings: Vec<BreeBuilding>,
    #[serde(default)]
    pub quest_cells: Vec<BreeQuestCell>,
    pub start: (i32, i32),
}

/// A monster placement in a fixed quest level (r_info id).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct QuestMapMonster {
    pub def: u32,
    pub x: i32,
    pub y: i32,
    /// Counts toward the kill-everything victory (thieves).
    pub quest: bool,
}

/// An object placement in a fixed quest level (k_info id).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct QuestMapObject {
    pub def: u32,
    pub x: i32,
    pub y: i32,
}

/// A fixed-map `F:...:*N:...` random placement (init1.cc
/// process_dungeon_file_aux): a monster/object generated at
/// `quest level + level` (the offset is 0 for a bare `*`). Quest levels
/// use their own depth; d_info special levels load with quest level 0, so
/// there `level` is absolute.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct QuestMapRandom {
    #[serde(default)]
    pub level: i32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

/// A fixed-map `M:` mimic: `t` is the terrain the cell is displayed as
/// until noticed (init1.cc process_dungeon_file_aux).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct QuestMapMimic {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub t: u16,
}

/// A fixed quest level (thieves.map / trolls.map / wights.map / ...).
#[derive(Debug, Clone, Deserialize)]
pub struct QuestMapDef {
    pub quest: u32,
    pub w: i32,
    pub h: i32,
    pub cells: Vec<FixedCell>,
    pub monsters: Vec<QuestMapMonster>,
    pub objects: Vec<QuestMapObject>,
    pub start: (i32, i32),
    /// Where the exit staircase appears once the quest is completed.
    pub exit: (i32, i32),
    /// The 172-Marker grid (princess/Thrain/quest-monster spot).
    pub marker: (i32, i32),
    /// `F:...:*N` random monster placements (depth + level).
    #[serde(default)]
    pub random_monsters: Vec<QuestMapRandom>,
    /// `F:...:*N` random object placements (depth + level).
    #[serde(default)]
    pub random_objects: Vec<QuestMapRandom>,
    /// `M:` mimic display terrain per cell.
    #[serde(default)]
    pub mimics: Vec<QuestMapMimic>,
}

/// A specific artifact placed by a fixed map (r_info `F:` artifact
/// field; init1.cc places it with a_allow_special set).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct QuestMapArtifact {
    pub id: u32,
    pub x: i32,
    pub y: i32,
}

/// A d_info '@:' special level: a fixed set-piece layout used when the
/// dungeon reaches `depth`.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecLevelDef {
    /// The map file name from d_info (`s_crypt.map` etc.).
    pub name: String,
    pub w: i32,
    pub h: i32,
    pub cells: Vec<FixedCell>,
    pub monsters: Vec<QuestMapMonster>,
    pub objects: Vec<QuestMapObject>,
    #[serde(default)]
    pub artifacts: Vec<QuestMapArtifact>,
    pub start: (i32, i32),
    /// `F:...:*N` random monster placements (depth + level).
    #[serde(default)]
    pub random_monsters: Vec<QuestMapRandom>,
    /// `F:...:*N` random object placements (depth + level).
    #[serde(default)]
    pub random_objects: Vec<QuestMapRandom>,
    /// `M:` mimic display terrain per cell.
    #[serde(default)]
    pub mimics: Vec<QuestMapMimic>,
}

// --- Wilderness (wf_info.txt / w_info.txt) ------------------------------

/// A wilderness terrain record (wf_info.txt): the world-map glyph and the
/// per-cell plasma terrain mapping.
#[derive(Debug, Clone, Deserialize)]
pub struct WfDef {
    pub id: u32,
    pub name: String,
    pub text: String,
    /// Monster/object level in this kind of area.
    pub level: u32,
    /// <1000: town number; >=1000: dungeon (1000 + d_info index).
    pub entrance: u32,
    /// ROAD_* bits (the shipped map uses none).
    pub road: u32,
    /// The f_info feature shown on the world overview map.
    pub feat: u16,
    /// Terrain kind index (WILD_* monster filtering in src/monster1.cc).
    pub terrain_idx: u32,
    pub ch: char,
    /// 18 height values -> f_info terrain ids.
    pub terrain: Vec<u16>,
}

impl WfDef {
    /// The town number of this record (0 = none).
    pub fn town(&self) -> u32 {
        if self.entrance > 0 && self.entrance < 1000 {
            self.entrance
        } else {
            0
        }
    }

    /// The dungeon index of this record (None = no dungeon).
    pub fn dungeon(&self) -> Option<u32> {
        if self.entrance >= 1000 {
            Some(self.entrance - 1000)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct WorldEntrance {
    pub d: u32,
    pub x: i32,
    pub y: i32,
}

/// A conditional world-map row replacement (the destroyed-Gondolin branch).
#[derive(Debug, Clone, Deserialize)]
pub struct WorldPatch {
    pub cond: String,
    pub y: i32,
    pub row: Vec<u32>,
}

/// The 101x66 Middle-earth world map (w_info.txt).
#[derive(Debug, Clone, Deserialize)]
pub struct WorldDef {
    pub w: i32,
    pub h: i32,
    pub rows: Vec<Vec<u32>>,
    pub patches: Vec<WorldPatch>,
    pub entrances: Vec<WorldEntrance>,
    pub start: (i32, i32),
}

impl WorldDef {
    /// The wilderness record at a world cell, applying the destroyed-town
    /// symbol patches (the `town_destroyN` rows of w_info.txt).
    pub fn feat(&self, x: i32, y: i32, destroyed: &[bool; 6]) -> u32 {
        if !(0..self.w).contains(&x) || !(0..self.h).contains(&y) {
            return 0;
        }
        let mut v = self.rows[y as usize][x as usize];
        for p in &self.patches {
            if p.y != y {
                continue;
            }
            let n: Option<usize> = p
                .cond
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok();
            let Some(n) = n else { continue };
            if n < 6 && destroyed[n] {
                v = p.row.get(x as usize).copied().unwrap_or(v);
            }
        }
        v
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DungeonFloorDef {
    pub feat: u16,
    /// Percentages at the top and bottom of the dungeon (linear between).
    pub top: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DungeonEffectDef {
    pub dd: i32,
    pub ds: i32,
    pub freq: i32,
    pub typ: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DungeonRuleDef {
    pub pct: i32,
    pub mode: u32,
    pub chars: Vec<String>,
    pub mflags: Vec<String>,
    pub mspells: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DungeonSpecialDef {
    /// Absolute dungeon depth.
    pub depth: u32,
    pub name: String,
    pub desc: String,
    /// The speclevels.ron map name.
    pub map: String,
    pub flags: Vec<String>,
    /// A branch staircase down to another dungeon.
    pub branch: Option<u32>,
    /// `@:<depth>:S:` save-file extension override (levels.cc
    /// level_data::save_extension).  A level carrying one persists across
    /// visits (`save_dungeon`); the shipped data sets it only on Mount Doom
    /// depth 99 (`mdm`).
    #[serde(default)]
    pub save_extension: Option<String>,
}

/// A dungeon type (d_info.txt).
#[derive(Debug, Clone, Deserialize)]
pub struct DungeonDef {
    pub id: u32,
    pub name: String,
    pub short: String,
    pub text: String,
    pub mindepth: u32,
    pub maxdepth: u32,
    pub min_plev: u32,
    pub min_alloc: u32,
    pub max_chance: u32,
    pub floors: Vec<DungeonFloorDef>,
    pub fills: Vec<DungeonFloorDef>,
    pub outer_wall: u16,
    pub inner_wall: u16,
    pub theme: (u32, u32, u32, u32),
    pub flags: Vec<String>,
    pub effects: Vec<DungeonEffectDef>,
    pub rules: Vec<DungeonRuleDef>,
    pub rule_percents: Vec<u32>,
    pub guardian: Option<u32>,
    pub final_artifact: Option<u32>,
    pub final_object: Option<u32>,
    pub specials: Vec<DungeonSpecialDef>,
    pub generator: String,
    /// Wilderness coords of the entrance (F:WILD_ix_iy__ox_oy).
    pub ix: i32,
    pub iy: i32,
    pub ox: i32,
    pub oy: i32,
    pub branch_parent: Option<(u32, u32)>,
}

impl DungeonDef {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    /// The special level on this absolute depth, if any.
    pub fn special_at(&self, depth: u32) -> Option<&DungeonSpecialDef> {
        self.specials.iter().find(|s| s.depth == depth)
    }

    /// A branch staircase on this depth.
    pub fn branch_at(&self, depth: u32) -> Option<u32> {
        self.specials
            .iter()
            .find(|s| s.depth == depth && s.branch.is_some())
            .and_then(|s| s.branch)
    }
}

// --- Towns (t_info.txt pref maps) ---------------------------------------

/// One town feature letter (F: line).  The cave_* flags mirror the
/// original CAVE_MARK/GLOW/ROOM/FREE bits.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct TownCharDef {
    pub ch: char,
    pub terrain: u16,
    #[serde(default)]
    pub mark: bool,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub room: bool,
    #[serde(default)]
    pub free: bool,
    #[serde(default)]
    pub monster: u32,
    #[serde(default)]
    pub object: u32,
    #[serde(default)]
    pub special: u32,
}

/// A conditional F: override (quest status, daytime, leaving quest, ...).
#[derive(Debug, Clone, Deserialize)]
pub struct TownOverride {
    pub cond: String,
    #[serde(default)]
    pub mark: bool,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub room: bool,
    #[serde(default)]
    pub free: bool,
    pub ch: char,
    pub terrain: u16,
    #[serde(default)]
    pub monster: u32,
    #[serde(default)]
    pub object: u32,
    #[serde(default)]
    pub special: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct TownStart {
    pub quest: u32,
    pub x: i32,
    pub y: i32,
}

/// A whole town variant (normal / destroyed) from a t_*.txt pref map.
#[derive(Debug, Clone, Deserialize)]
pub struct TownDef {
    pub id: u32,
    pub name: String,
    pub destroyed: bool,
    /// Lowercase `f:` monster theme (ELVEN/DWARVEN for the town spawns).
    #[serde(default)]
    pub mflag: String,
    pub rows: Vec<String>,
    pub chars: Vec<TownCharDef>,
    pub overrides: Vec<TownOverride>,
    pub starts: Vec<TownStart>,
    pub default_start: Option<(i32, i32)>,
}

/// A store owner (ow_info.txt): race/class price preferences.
#[derive(Debug, Clone, Deserialize)]
pub struct OwnerDef {
    pub id: u32,
    pub name: String,
    /// Won't buy items worth more than this.
    pub max_cost: i32,
    /// Greed factor (original "inflation"; 100 = neutral).
    #[serde(default = "hundred")]
    pub inflation: i32,
    /// (hated, normal, liked) price percentage factors.
    pub costs: (i32, i32, i32),
    pub liked: Vec<String>,
    pub hated: Vec<String>,
}

fn hundred() -> i32 {
    100
}

/// A vault template (v_info.txt): typ 7 = lesser, 8 = greater. `data`
/// holds `hgt * wid` map characters (see build_vault in src/generate.cc).
#[derive(Debug, Clone, Deserialize)]
pub struct VaultDef {
    pub id: u32,
    pub typ: u32,
    pub rat: u32,
    pub hgt: u32,
    pub wid: u32,
    pub data: String,
}

/// One (tval, sval range) a randart part applies to.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RandartTval {
    pub tval: i32,
    pub min: i32,
    pub max: i32,
}

/// One randart power/part (ra_info.txt; create_artifact in src/randart.cc).
#[derive(Debug, Clone, Deserialize)]
pub struct RandartPartDef {
    pub id: u32,
    pub tvals: Vec<RandartTval>,
    /// Minimum player level (out-of-depth penalty below it).
    pub level: u32,
    pub rarity: u32,
    pub mrarity: u32,
    /// C: field order is to-hit : to-dam : to-AC : pval.
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    pub pval: i32,
    pub value: i32,
    /// Max times this part may appear on one item.
    pub max: i32,
    pub flags: Vec<String>,
    /// Antagonistic flags: the part is rejected if the item has any.
    pub aflags: Vec<String>,
}

/// One G: row of the randart power budget (damroll(dd,ds) + plus).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RandartGenDef {
    pub chance: i32,
    pub dd: i32,
    pub ds: i32,
    pub plus: i32,
}

/// The whole randart data file (parts + budget + name corpus).
#[derive(Debug, Deserialize)]
pub struct RandartFile {
    pub parts: Vec<RandartPartDef>,
    pub gen: Vec<RandartGenDef>,
    pub names: String,
    /// Legacy TV_RANDART "junkart" short/full names (rart_s/rart_f.txt).
    #[serde(default)]
    pub junk_s: Vec<String>,
    #[serde(default)]
    pub junk_f: Vec<String>,
    #[serde(default)]
    pub acts: Vec<JunkActDef>,
}

/// One random-artifact activation (tables.cc activation_info).
#[derive(Debug, Clone, Deserialize)]
pub struct JunkActDef {
    pub desc: String,
    /// Recharge cost (timeout = cost / 10).
    pub cost: u32,
    /// Activation name (ACT_* suffix, matching artifact_activation).
    pub act: String,
}

/// One threshold tier of a set member (P:/F: lines).
#[derive(Debug, Clone, Deserialize)]
pub struct SetTierDef {
    pub pval: i32,
    pub flags: Vec<String>,
}

/// One artifact of a set and its per-count tiers.
#[derive(Debug, Clone, Deserialize)]
pub struct SetMemberDef {
    pub artifact: u32,
    pub tiers: Vec<SetTierDef>,
}

/// An artifact set (set_info.txt; apply_set in src/object1.cc).
#[derive(Debug, Clone, Deserialize)]
pub struct SetDef {
    pub id: u32,
    pub name: String,
    pub desc: String,
    pub members: Vec<SetMemberDef>,
}

/// One blow override of a monster ego (re_info B: lines).
#[derive(Debug, Clone, Deserialize)]
pub struct EgoBlowDef {
    pub method: String,
    pub effect: String,
    pub ddice: String,
    pub dside: String,
}

/// A monster ego race (re_info.txt): stat modifiers plus flags/spells.
/// Modifier strings: "+N" add, "-N" subtract, "=N" set, "%N" percentage.
#[derive(Debug, Clone, Deserialize)]
pub struct EgoMonDef {
    pub id: u32,
    pub name: String,
    pub ch: String,
    pub color: i32,
    /// Name goes before the base name ("Skeleton Orc").
    pub before: bool,
    pub speed: String,
    pub hdice: String,
    pub hside: String,
    pub aaf: String,
    pub ac: String,
    pub sleep: String,
    pub level: String,
    pub rarity: u32,
    pub weight: String,
    pub mexp: String,
    /// re_info S:1_IN_n -> 100/n (matches the base spell_freq scale).
    #[serde(default)]
    pub spell_freq: i32,
    pub req_flags: Vec<String>,
    pub req_chars: Vec<String>,
    pub forbid_flags: Vec<String>,
    pub forbid_chars: Vec<String>,
    pub add_flags: Vec<String>,
    pub remove_flags: Vec<String>,
    pub add_spells: Vec<String>,
    /// re_info T: nspells; "MF_ALL" clears every inherited spell.
    #[serde(default)]
    pub remove_spells: Vec<String>,
    pub blows: Vec<EgoBlowDef>,
}

/// Apply a re_info modifier string to a value.
pub fn apply_mod(v: i32, m: &str) -> i32 {
    if m.is_empty() {
        return v;
    }
    let (kind, num) = m.split_at(1);
    let n: i32 = num.parse().unwrap_or(0);
    match kind {
        "+" => v + n,
        "-" => v - n,
        "=" => n,
        "%" => v * n / 100,
        _ => v,
    }
}

/// Does the base monster qualify for this ego? (mego_ok, monster2.cc)
pub fn ego_mon_ok(base: &MonsterDef, e: &EgoMonDef) -> bool {
    if e.req_flags.iter().any(|f| !base.has(f)) {
        return false;
    }
    if e.forbid_flags.iter().any(|f| base.has(f)) {
        return false;
    }
    let ch = base.glyph();
    if !e.req_chars.is_empty() && !e.req_chars.iter().any(|c| c.starts_with(ch)) {
        return false;
    }
    if e.forbid_chars.iter().any(|c| c.starts_with(ch)) {
        return false;
    }
    true
}

/// Build the modified monster race of an ego variant (race_info_idx).
pub fn apply_monster_ego(base: &MonsterDef, e: &EgoMonDef) -> MonsterDef {
    let mut v = base.clone();
    v.name = if e.before {
        format!("{} {}", e.name, base.name)
    } else {
        format!("{} {}", base.name, e.name)
    };
    if !e.ch.is_empty() {
        v.ch = e.ch.clone();
    }
    if e.color >= 0 {
        v.color = e.color as u8;
    }
    // HP dice.
    if let Some(d) = Dice::parse(&base.hp) {
        let count = apply_mod(d.count, &e.hdice).max(1);
        let sides = apply_mod(d.sides, &e.hside).max(1);
        v.hp = format!("{}d{}", count, sides);
    }
    // MODIFY() floors: speed 50, AC 0, sleep 0, level 1, exp 0,
    // weight 10 (monster2.cc race_info_idx).
    v.speed = apply_mod(base.speed, &e.speed).max(50);
    v.ac = apply_mod(base.ac, &e.ac).max(0);
    v.alert = apply_mod(base.alert, &e.sleep).max(0);
    v.aaf = apply_mod(base.aaf, &e.aaf).max(1);
    v.depth = apply_mod(base.depth as i32, &e.level).max(1) as u32;
    v.exp = apply_mod(base.exp as i32, &e.mexp).max(0) as u32;
    // Corpse weight scales with the ego (monster2.cc MODIFY weight).
    v.weight = apply_mod(base.weight as i32, &e.weight).max(10) as u32;
    // Ego spellcasters keep the better innate spell frequency
    // (monster2.cc race_info_idx: max of the original 100/n values).
    // Base r_info rows store n (1-in-n), egos the 100/n scale, so a
    // larger ego frequency means a *smaller* n.
    if e.spell_freq > 0 {
        let ego_n = 100 / e.spell_freq;
        v.spell_freq = if v.spell_freq == 0 {
            ego_n
        } else {
            v.spell_freq.min(ego_n)
        };
    }
    // Flags: remove then add; recompute the derived booleans.
    v.flags.retain(|f| !e.remove_flags.iter().any(|r| r == f));
    for f in &e.add_flags {
        if !v.flags.contains(f) {
            v.flags.push(f.clone());
        }
    }
    let has = |f: &str| v.flags.iter().any(|x| x == f);
    v.open_door = has("OPEN_DOOR");
    v.bash_door = has("BASH_DOOR");
    v.never_move = has("NEVER_MOVE");
    v.unique = has("UNIQUE");
    v.friends = has("FRIENDS");
    v.escort = has("ESCORT");
    // Blows: pad to the ego's slots, then apply dice/method/effect.
    while v.blows.len() < e.blows.len() {
        v.blows.push(BlowDef {
            method: String::new(),
            effect: String::new(),
            dice: "0d0".to_string(),
        });
    }
    for (i, eb) in e.blows.iter().enumerate() {
        let b = &mut v.blows[i];
        if let Some(d) = Dice::parse(&b.dice) {
            let count = apply_mod(d.count, &eb.ddice).max(0);
            let sides = apply_mod(d.sides, &eb.dside).max(0);
            b.dice = if d.bonus > 0 {
                format!("{}d{}+{}", count, sides, d.bonus)
            } else {
                format!("{}d{}", count, sides)
            };
        }
        if !eb.method.is_empty() {
            b.method = eb.method.clone();
        }
        if !eb.effect.is_empty() {
            b.effect = eb.effect.clone();
        }
    }
    // re_info T:MF_ALL clears all inherited spells; T: lines remove them.
    if e.remove_spells.iter().any(|s| s == "MF_ALL") {
        v.spells.clear();
    } else {
        v.spells.retain(|s| !e.remove_spells.contains(s));
    }
    for s in &e.add_spells {
        if !v.spells.contains(s) {
            v.spells.push(s.clone());
        }
    }
    v
}

/// misc.txt (init2.cc:532 process_dungeon_file): engine table sizes.
pub const MAX_TOWNS: usize = 100;
pub const NONRANDOM_TOWNS: usize = 5;
pub const WILD_X: usize = 101;
pub const WILD_Y: usize = 66;
pub const MAX_OBJECTS_PER_LEVEL: usize = 1024;
pub const MAX_MONSTERS_PER_LEVEL: usize = 768;

#[derive(Resource)]
pub struct GameData {
    /// Indexed by terrain id (holes filled with terrain 0).
    pub terrain: Vec<TerrainDef>,
    /// Base monsters plus, appended after `monster_base_count`, the
    /// ego variants built at load time (re_info).
    pub monsters: Vec<MonsterDef>,
    /// Number of "real" r_info monsters (variants start here).
    pub monster_base_count: usize,
    /// base def index -> variant def indices.
    pub variants_by_base: std::collections::HashMap<usize, Vec<usize>>,
    /// variant def index -> ego id (parallel to the appended range).
    pub variant_egos: Vec<u32>,
    pub monster_egos: Vec<EgoMonDef>,
    pub owners: Vec<OwnerDef>,
    pub races: Vec<RaceDef>,
    /// Base subraces / race modifiers (p_info S: records).
    pub racemods: Vec<RaceModDef>,
    pub classes: Vec<ClassDef>,
    /// General skill modifiers applied to every character (p_info G:k:).
    pub general_skills: Vec<SkillMod>,
    /// The full s_info skill tree.
    pub skills: Vec<SkillDef>,
    /// ab_info abilities.
    pub abilities: Vec<AbilityDef>,
    /// ba_info building actions.
    pub building_actions: Vec<BuildingActionDef>,
    pub objects: Vec<ObjectDef>,
    pub stores: Vec<StoreDef>,
    pub egos: Vec<EgoDef>,
    pub artifacts: Vec<ArtifactDef>,
    pub schools: Vec<SchoolDef>,
    pub spells: Vec<SpellRow>,
    pub quest_maps: Vec<QuestMapDef>,
    /// Wilderness terrain records (wf_info.txt).
    pub wf: Vec<WfDef>,
    /// The world map (w_info.txt).
    pub world: WorldDef,
    /// Dungeon types (d_info.txt).
    pub dungeons: Vec<DungeonDef>,
    /// Town layouts, normal and destroyed (t_info.txt pref maps).
    pub towns: Vec<TownDef>,
    /// Fixed set-piece dungeon levels (d_info '@:' maps), by name.
    pub spec_levels: Vec<SpecLevelDef>,
    /// Vault templates (v_info.txt).
    pub vaults: Vec<VaultDef>,
    /// Randart parts and tables (ra_info.txt + tables.cc names).
    pub randarts: RandartFile,
    /// Artifact sets (set_info.txt).
    pub sets: Vec<SetDef>,
    /// Flavour groups for the per-game colour shuffle (object1.cc
    /// object_flavor): amulets/rings/staves/wands/rods/scrolls/potions/
    /// mushrooms, ordered by sval.
    pub flavor_groups: Vec<Vec<usize>>,
    /// def index -> (flavor group, position in the group).
    pub flavor_pos: std::collections::HashMap<usize, (usize, usize)>,
}

impl GameData {
    pub fn terrain(&self, id: u16) -> &TerrainDef {
        &self.terrain[id as usize]
    }

    /// First object whose name matches (names are pre-cleaned).
    pub fn object_by_name(&self, name: &str) -> Option<usize> {
        self.objects.iter().position(|o| o.name == name)
    }

    /// First object with this tval/sval pair (T: store entries).
    pub fn object_by_tval_sval(&self, tval: i32, sval: i32) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.tval == tval && o.sval == sval)
    }

    /// First monster with this exact name (quest uniques).
    pub fn monster_by_name(&self, name: &str) -> Option<usize> {
        self.monsters.iter().position(|m| m.name == name)
    }

    /// First monster with this r_info id (fixed quest level placements).
    pub fn monster_by_id(&self, id: u32) -> Option<usize> {
        self.monsters.iter().position(|m| m.id == id)
    }

    /// First object with this k_info id (fixed quest level placements).
    pub fn object_by_id(&self, id: u32) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    /// `GameEditData::k_info_keys` (game_edit_data.cc:5): the k_info map
    /// keys in ascending order.  The port keeps the kinds in a `Vec`, so
    /// the keys are collected and sorted the same way.
    #[allow(dead_code)]
    pub fn k_info_keys(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self.objects.iter().map(|o| o.id).collect();
        keys.sort_unstable();
        keys
    }

    /// The fixed map of a plot quest (thieves/trolls/wights/...).
    pub fn quest_map(&self, quest: u32) -> Option<&QuestMapDef> {
        self.quest_maps.iter().find(|q| q.quest == quest)
    }

    /// The set-piece layout for a dungeon depth, if any (by map name).
    pub fn spec_level(&self, name: &str) -> Option<&SpecLevelDef> {
        self.spec_levels.iter().find(|s| s.name == name)
    }

    /// A wilderness terrain record (wf_info ids have a hole at 11).
    pub fn wf(&self, id: u32) -> &WfDef {
        self.wf.iter().find(|w| w.id == id).unwrap_or(&self.wf[0])
    }

    /// A dungeon type.
    pub fn dungeon(&self, id: u32) -> &DungeonDef {
        self.dungeons
            .iter()
            .find(|d| d.id == id)
            .unwrap_or(&self.dungeons[0])
    }

    /// `current_level_data()` (levels.cc:6): the `@:` per-depth record of
    /// a dungeon level, or None for the all-zero default record.
    pub fn level_data(&self, dungeon: u32, depth: u32) -> Option<&DungeonSpecialDef> {
        self.dungeon(dungeon).special_at(depth)
    }

    /// `get_branch()` (levels.cc:24): branch staircase target, 0 = none.
    pub fn level_branch(&self, dungeon: u32, depth: u32) -> u32 {
        self.dungeon(dungeon).branch_at(depth).unwrap_or(0)
    }

    /// `get_fbranch()` (levels.cc:29): parent dungeon back-reference; the
    /// original fills it only on the child's first floor (post_d_info).
    pub fn level_fbranch(&self, dungeon: u32, depth: u32) -> u32 {
        let d = self.dungeon(dungeon);
        if depth == d.mindepth {
            d.branch_parent.map(|(parent, _)| parent).unwrap_or(0)
        } else {
            0
        }
    }

    /// `get_flevel()` (levels.cc:34): parent depth relative to the
    /// parent's first floor (post_d_info: parent_depth - parent mindepth).
    pub fn level_flevel(&self, dungeon: u32, depth: u32) -> i32 {
        let d = self.dungeon(dungeon);
        if depth == d.mindepth {
            d.branch_parent
                .map(|(parent, parent_depth)| {
                    parent_depth as i32 - self.dungeon(parent).mindepth as i32
                })
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// `get_dungeon_save_extension()` (levels.cc:39): the level's own
    /// save-file extension (`@:<depth>:S:`, parsed mindepth-relative as
    /// `depth + mindepth`), None when unset.  Only these levels persist
    /// (loadsave.cc save_dungeon); everything else is rebuilt on entry.
    pub fn level_save_extension(&self, dungeon: u32, depth: u32) -> Option<&str> {
        self.level_data(dungeon, depth)
            .and_then(|s| s.save_extension.as_deref())
    }

    /// `get_dungeon_map_name()` (levels.cc:44): the `@:<depth>:U:` map.
    pub fn level_map_name(&self, dungeon: u32, depth: u32) -> Option<&str> {
        self.level_data(dungeon, depth)
            .filter(|s| !s.map.is_empty())
            .map(|s| s.map.as_str())
    }

    /// `get_dungeon_name()` (levels.cc:49): the `@:<depth>:N:` short name
    /// shown instead of the depth (xtra1.cc prt_depth).
    pub fn level_name(&self, dungeon: u32, depth: u32) -> Option<&str> {
        self.level_data(dungeon, depth)
            .filter(|s| !s.name.is_empty())
            .map(|s| s.name.as_str())
    }

    /// `get_level_flags()` (levels.cc:54): the `@:<depth>:F:` flags of
    /// this level alone (callers OR them into the dungeon flags).
    pub fn level_flags(&self, dungeon: u32, depth: u32) -> Vec<&str> {
        self.level_data(dungeon, depth)
            .map(|s| s.flags.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// `get_level_description()` (levels.cc:59): the `@:<depth>:D:`
    /// feeling override (cmd4.cc do_cmd_feeling).
    pub fn level_description(&self, dungeon: u32, depth: u32) -> Option<&str> {
        self.level_data(dungeon, depth)
            .filter(|s| !s.desc.is_empty())
            .map(|s| s.desc.as_str())
    }

    /// A town layout variant (the original t_info.txt town selection).
    pub fn town(&self, id: u32, destroyed: bool) -> Option<&TownDef> {
        self.towns
            .iter()
            .find(|t| t.id == id && t.destroyed == destroyed)
    }

    /// The world-map cell of a fixed town (scans w_info.txt).
    pub fn town_cell(&self, id: u32) -> (i32, i32) {
        for y in 0..self.world.h {
            for x in 0..self.world.w {
                if self.wf(self.world.feat(x, y, &[false; 6])).town() == id {
                    return (x, y);
                }
            }
        }
        self.world.start
    }

    /// A vault template of a type (7 lesser / 8 greater).
    pub fn vault_of_type(&self, typ: u32, rng: &mut impl Rng) -> Option<&VaultDef> {
        let cands: Vec<&VaultDef> = self.vaults.iter().filter(|v| v.typ == typ).collect();
        if cands.is_empty() {
            return None;
        }
        Some(cands[rng.gen_range(0..cands.len())])
    }

    /// The set an artifact belongs to, if any.
    pub fn set_of_artifact(&self, artifact: u32) -> Option<&SetDef> {
        self.sets
            .iter()
            .find(|s| s.members.iter().any(|m| m.artifact == artifact))
    }

    /// A spell by name (device effects, spellbook listings).
    pub fn spell_by_name(&self, name: &str) -> Option<&SpellRow> {
        self.spells.iter().find(|s| s.name == name)
    }
}

pub fn load_game_data() -> GameData {
    let raw: Vec<TerrainDef> = ron::from_str(include_str!("../assets/data/terrain.ron"))
        .unwrap_or_else(|_| broken_lib_panic("cannot open 'data/terrain.ron'"));
    let max_id = raw.iter().map(|t| t.id).max().unwrap_or(0) as usize;
    let fallback = raw
        .iter()
        .find(|t| t.id == 0)
        .cloned()
        .expect("terrain 0 missing");
    let mut terrain = vec![fallback; max_id + 1];
    for t in raw {
        terrain[t.id as usize] = t.clone();
    }
    // Base monsters plus the ego variants (re_info) appended at load.
    let mut monsters: Vec<MonsterDef> =
        ron::from_str(include_str!("../assets/data/monsters.ron")).expect("invalid monsters.ron");
    let monster_egos: Vec<EgoMonDef> =
        ron::from_str(include_str!("../assets/data/monster_egos.ron"))
            .expect("invalid monster_egos.ron");
    let owners: Vec<OwnerDef> =
        ron::from_str(include_str!("../assets/data/owners.ron")).expect("invalid owners.ron");
    let monster_base_count = monsters.len();
    let mut variants_by_base: std::collections::HashMap<usize, Vec<usize>> = Default::default();
    let mut variant_egos: Vec<u32> = Vec::new();
    let bases = monsters.clone();
    for (bi, base) in bases.iter().enumerate() {
        if base.id == 0 || base.unique {
            continue;
        }
        for e in &monster_egos {
            if ego_mon_ok(base, e) {
                variants_by_base.entry(bi).or_default().push(monsters.len());
                variant_egos.push(e.id);
                monsters.push(apply_monster_ego(base, e));
            }
        }
    }
    let objects: Vec<ObjectDef> =
        ron::from_str(include_str!("../assets/data/items.ron")).expect("invalid items.ron");
    // Flavour groups (object1.cc object_flavor): the shuffled colours of
    // unidentified jewellery/devices/scrolls/potions/mushrooms.
    let mut flavor_groups: Vec<Vec<usize>> = Vec::new();
    let mut flavor_pos: std::collections::HashMap<usize, (usize, usize)> = Default::default();
    for tval in [40, 45, 55, 65, 66, 70, 71, 80] {
        let mut group: Vec<usize> = objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                if tval == 71 {
                    o.tval == 71 || o.tval == 72
                } else if tval == 80 {
                    o.tval == 80 && o.sval < 32
                } else {
                    o.tval == tval
                }
            })
            .map(|(i, _)| i)
            .collect();
        group.sort_by_key(|&i| objects[i].sval);
        let g = flavor_groups.len();
        for (pos, &def) in group.iter().enumerate() {
            flavor_pos.insert(def, (g, pos));
        }
        flavor_groups.push(group);
    }
    let mut gd = GameData {
        terrain,
        monsters,
        monster_base_count,
        variants_by_base,
        variant_egos,
        monster_egos,
        owners,
        races: ron::from_str(include_str!("../assets/data/races.ron")).expect("invalid races.ron"),
        racemods: ron::from_str(include_str!("../assets/data/racemods.ron"))
            .expect("invalid racemods.ron"),
        classes: ron::from_str(include_str!("../assets/data/classes.ron"))
            .expect("invalid classes.ron"),
        general_skills: ron::from_str(include_str!("../assets/data/general_skills.ron"))
            .expect("invalid general_skills.ron"),
        skills: ron::from_str(include_str!("../assets/data/skills.ron"))
            .expect("invalid skills.ron"),
        abilities: ron::from_str(include_str!("../assets/data/abilities.ron"))
            .expect("invalid abilities.ron"),
        building_actions: ron::from_str(include_str!("../assets/data/building_actions.ron"))
            .expect("invalid building_actions.ron"),
        objects,
        stores: ron::from_str(include_str!("../assets/data/stores.ron"))
            .expect("invalid stores.ron"),
        egos: ron::from_str(include_str!("../assets/data/egos.ron")).expect("invalid egos.ron"),
        artifacts: ron::from_str(include_str!("../assets/data/artifacts.ron"))
            .expect("invalid artifacts.ron"),
        schools: ron::from_str(include_str!("../assets/data/schools.ron"))
            .expect("invalid schools.ron"),
        spells: ron::from_str(include_str!("../assets/data/spells.ron"))
            .expect("invalid spells.ron"),
        quest_maps: ron::from_str(include_str!("../assets/data/questmaps.ron"))
            .expect("invalid questmaps.ron"),
        wf: ron::from_str(include_str!("../assets/data/wf.ron")).expect("invalid wf.ron"),
        world: ron::from_str(include_str!("../assets/data/world.ron")).expect("invalid world.ron"),
        dungeons: ron::from_str(include_str!("../assets/data/dungeons.ron"))
            .expect("invalid dungeons.ron"),
        towns: ron::from_str(include_str!("../assets/data/towns.ron")).expect("invalid towns.ron"),
        spec_levels: ron::from_str(include_str!("../assets/data/speclevels.ron"))
            .expect("invalid speclevels.ron"),
        vaults: ron::from_str(include_str!("../assets/data/vaults.ron"))
            .expect("invalid vaults.ron"),
        randarts: ron::from_str(include_str!("../assets/data/randarts.ron"))
            .expect("invalid randarts.ron"),
        sets: ron::from_str(include_str!("../assets/data/sets.ron")).expect("invalid sets.ron"),
        flavor_groups,
        flavor_pos,
    };
    apply_guardian_flags(&mut gd);
    gd
}

/// `init_guardians` (init2.cc:889): mark each dungeon's final guardian,
/// final artifact and final object with SPECIAL_GENE, and give a guardian
/// with no final treasure DROP_RANDART.  The generated data already
/// carries these marks; applying them at load keeps the invariant.
fn apply_guardian_flags(gd: &mut GameData) {
    let guardians: Vec<(u32, Option<u32>, Option<u32>)> = gd
        .dungeons
        .iter()
        .filter_map(|d| d.guardian.map(|g| (g, d.final_artifact, d.final_object)))
        .collect();
    for (guardian, final_artifact, final_object) in guardians {
        let Some(mi) = gd.monster_by_id(guardian) else { continue };
        if !gd.monsters[mi].flags.iter().any(|f| f == "SPECIAL_GENE") {
            gd.monsters[mi].flags.push("SPECIAL_GENE".to_string());
        }
        if let Some(aid) = final_artifact {
            if let Some(ai) = gd.artifacts.iter().position(|a| a.id == aid) {
                if !gd.artifacts[ai].flags.iter().any(|f| f == "SPECIAL_GENE") {
                    gd.artifacts[ai].flags.push("SPECIAL_GENE".to_string());
                }
            }
        }
        if let Some(oid) = final_object {
            if let Some(oi) = gd.object_by_id(oid) {
                if !gd.objects[oi].flags.iter().any(|f| f == "SPECIAL_GENE") {
                    gd.objects[oi].flags.push("SPECIAL_GENE".to_string());
                }
            }
        }
        if final_artifact.is_none()
            && final_object.is_none()
            && !gd.monsters[mi].flags.iter().any(|f| f == "DROP_RANDART")
        {
            gd.monsters[mi].flags.push("DROP_RANDART".to_string());
        }
    }
}

/// `init_angband_aux` (init2.cc:938): the fatal "the lib directory is
/// probably missing or broken" diagnosis.  A panic is the port's quit;
/// the data parser reports unreadable shipped assets this way.
pub fn broken_lib_panic(why: &str) -> ! {
    panic!("Fatal Error: {why}. The data directory is probably missing or broken.")
}

/// `range_type` (range.hpp) + `range_init` (range.cc:5): an inclusive
/// integer level band.  `device_allocation` stores these as inline
/// `(min, max)` pairs; the struct mirrors the original one-for-one.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub min: i32,
    pub max: i32,
}

impl Range {
    /// `range_init` (range.cc:5): set both bounds.
    #[allow(dead_code)]
    pub fn new(min: i32, max: i32) -> Self {
        Range { min, max }
    }
}

/// `device_allocation` (device_allocation.hpp:9): the device allocation
/// row of a spell.  The original struct is `{ tval, rarity, base_level,
/// max_level }`; the port's `SpellRow.alloc` keeps the exact same six
/// fields in the same order (tval, rarity, base.min, base.max, max.min,
/// max.max).
pub type DeviceAllocation = (i32, i32, i32, i32, i32, i32);

/// `device_allocation_init` (device_allocation.cc:5): set the tval and
/// zero the rarity and both level ranges.
#[allow(dead_code)]
pub fn device_allocation_init(d: &mut DeviceAllocation, tval: i32) {
    *d = (tval, 0, 0, 0, 0, 0);
}

/// `device_allocation_new` (device_allocation.cc:15): allocate and
/// initialize a device allocation for `tval`.
#[allow(dead_code)]
pub fn device_allocation_new(tval: i32) -> DeviceAllocation {
    (tval, 0, 0, 0, 0, 0)
}

/// A dice expression like "1d6", "2d4+2".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dice {
    pub count: i32,
    pub sides: i32,
    pub bonus: i32,
}

impl Dice {
    /// `dice_print` (dice.cc:68): render the dice back as `B+NdM`.
    pub fn print(&self) -> String {
        let mut s = String::new();
        if self.bonus > 0 {
            s.push_str(&self.bonus.to_string());
        }
        if self.count > 0 || self.sides > 0 {
            if self.bonus > 0 {
                s.push('+');
            }
            if self.count > 1 {
                s.push_str(&self.count.to_string());
            }
            s.push('d');
            s.push_str(&self.sides.to_string());
        }
        s
    }

    /// Parse the original `dice.cc dice_parse` grammar: `B+NdM`, `B+dM`,
    /// `dM`, `NdM` or `N` (plus the port's `NdM+B` form used in its own
    /// data).  `dice_roll` is then `base + damroll(num, sides)`.
    pub fn parse(s: &str) -> Option<Dice> {
        let s = s.trim();
        if let Some(i) = s.find('+') {
            let (left, right) = (s[..i].trim(), s[i + 1..].trim());
            if left.contains('d') {
                // Port extension: "NdM+B" keeps its flat bonus.
                let (n, d) = left.split_once('d')?;
                return Some(Dice {
                    count: if n.is_empty() { 1 } else { n.parse().ok()? },
                    sides: d.parse().ok()?,
                    bonus: right.parse().ok()?,
                });
            }
            // "B+NdM" / "B+dM".
            let base: i32 = left.parse().ok()?;
            let (n, d) = right.split_once('d')?;
            return Some(Dice {
                count: if n.is_empty() { 1 } else { n.parse().ok()? },
                sides: d.parse().ok()?,
                bonus: base,
            });
        }
        if let Some((n, d)) = s.split_once('d') {
            return Some(Dice {
                count: if n.is_empty() { 1 } else { n.parse().ok()? },
                sides: d.parse().ok()?,
                bonus: 0,
            });
        }
        Some(Dice {
            count: 0,
            sides: 0,
            bonus: s.parse().ok()?,
        })
    }

    /// `dice_roll` (dice.cc:62): `base + damroll(num, sides)`.
    /// `damroll` (z-rand.cc:194) sums `randint(sides)`, and `randint`
    /// (z-rand.cc:227) returns 1 whenever the die has fewer than two
    /// sides.  The C++ result is *not* clamped, so neither is this.
    pub fn roll(&self, rng: &mut impl Rng) -> i32 {
        let mut total = self.bonus;
        for _ in 0..self.count {
            total += if self.sides < 2 {
                1
            } else {
                rng.gen_range(1..=self.sides)
            };
        }
        total
    }
}

/// Port of `src/object_filter.cc`: the original builds `object_filter_t`
/// values (i.e. `std::function<bool(object_type const *)>`) from small
/// named predicates.  The commands pass plain `impl Fn(&Item) -> bool`
/// closures in the port, so these factories are the shared, testable
/// counterparts of the original helper namespace.
pub mod object_filter {
    use std::collections::HashSet;

    use crate::item::{ego_item_p, is_artifact, item_flags, Item};

    /// `object_filter::TVal`: the item kind has this tval.
    pub fn tval(gd: &super::GameData, tval: i32) -> impl Fn(&Item) -> bool + '_ {
        move |it| gd.objects[it.def].tval == tval
    }

    /// `object_filter::SVal`: the item kind has this sval.
    pub fn sval(gd: &super::GameData, sval: i32) -> impl Fn(&Item) -> bool + '_ {
        move |it| gd.objects[it.def].sval == sval
    }

    /// `object_filter::HasFlags`: `bool(flags & mask)`, i.e. the item's
    /// (known or raw) flags share at least one entry with the mask.
    pub fn has_flags<'a>(
        gd: &'a super::GameData,
        mask: &'a [&'a str],
    ) -> impl Fn(&Item) -> bool + 'a {
        move |it| item_flags(gd, it).iter().any(|f| mask.contains(f))
    }

    /// `object_filter::IsArtifact`: a standard artifact (`name1 > 0`).
    pub fn is_standard_artifact() -> impl Fn(&Item) -> bool {
        |it| it.artifact != 0
    }

    /// `object_filter::IsArtifactP`: `artifact_p` (includes randarts).
    pub fn is_artifact_p(gd: &super::GameData) -> impl Fn(&Item) -> bool + '_ {
        move |it| is_artifact(gd, it)
    }

    /// `object_filter::IsEgo`: `ego_item_p`.
    pub fn is_ego() -> impl Fn(&Item) -> bool {
        ego_item_p
    }

    /// `object_filter::IsKnown`: `object_known_p` = fully identified or
    /// the kind's type is known in the player's inventory.
    pub fn is_known(known: &HashSet<usize>) -> impl Fn(&Item) -> bool + '_ {
        move |it| it.identified || known.contains(&it.def)
    }

    /// `object_filter::True`: accepts everything.
    pub fn all() -> impl Fn(&Item) -> bool {
        |_| true
    }

    /// `object_filter::Not`: negates a filter.
    pub fn not<F: Fn(&Item) -> bool>(p: F) -> impl Fn(&Item) -> bool {
        move |it| !p(it)
    }

    /// `object_filter::And()`: the zero-argument base of the variadic
    /// conjunction (object_filter.hpp), which is vacuously true.
    pub fn and() -> impl Fn(&Item) -> bool {
        |_| true
    }

    /// `object_filter::Or()`: the zero-argument base of the variadic
    /// disjunction (object_filter.hpp), which is vacuously false.
    pub fn or() -> impl Fn(&Item) -> bool {
        |_| false
    }

    /// `object_filter::And(Arg0&&, Args&&...)` (object_filter.hpp:67):
    /// logical conjunction of every filter in the pack.  The C++ variadic
    /// expands to `arg0(o) && args(o)...`; the port takes a slice of
    /// trait objects.
    #[allow(dead_code)]
    pub fn all_of<'a>(filters: &'a [&'a dyn Fn(&Item) -> bool]) -> impl Fn(&Item) -> bool + 'a {
        move |it| filters.iter().all(|f| f(it))
    }

    /// `object_filter::Or(Arg0&&, Args&&...)` (object_filter.hpp:82):
    /// logical disjunction of every filter in the pack.
    #[allow(dead_code)]
    pub fn any_of<'a>(filters: &'a [&'a dyn Fn(&Item) -> bool]) -> impl Fn(&Item) -> bool + 'a {
        move |it| filters.iter().any(|f| f(it))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::data::load_game_data;
        use crate::item::Item;

        #[test]
        fn object_filter_predicates_match_the_original() {
            let gd = load_game_data();
            let sword = gd
                .objects
                .iter()
                .position(|o| o.tval == crate::data::TV_SWORD)
                .expect("sword kind");
            let mut it = Item::base(&gd, sword);
            let mut known = HashSet::new();

            // TVal / SVal.
            assert!(tval(&gd, crate::data::TV_SWORD)(&it));
            assert!(!tval(&gd, crate::data::TV_BOW)(&it));
            let sword_sval = gd.objects[sword].sval;
            assert!(sval(&gd, sword_sval)(&it));
            assert!(!sval(&gd, sword_sval + 1)(&it));

            // HasFlags: any mask bit present (bool(flags & mask)).
            it.flags.push("SUST_STR".to_string());
            assert!(has_flags(&gd, &["SUST_STR"])(&it));
            assert!(has_flags(&gd, &["SUST_STR", "SUST_DEX"])(&it));
            assert!(!has_flags(&gd, &["SUST_DEX"])(&it));

            // IsArtifact is name1 > 0; IsArtifactP also accepts randarts.
            assert!(!is_standard_artifact()(&it));
            assert!(!is_artifact_p(&gd)(&it));
            it.artifact_name = "of Testing".to_string();
            assert!(!is_standard_artifact()(&it));
            assert!(is_artifact_p(&gd)(&it));
            it.artifact = 1;
            assert!(is_standard_artifact()(&it));

            // IsEgo.
            assert!(!is_ego()(&it));
            it.ego = 1;
            assert!(is_ego()(&it));

            // IsKnown: identified or a known kind.
            assert!(!is_known(&known)(&it));
            known.insert(it.def);
            assert!(is_known(&known)(&it));
            let mut identified = it.clone();
            identified.identified = true;
            assert!(is_known(&HashSet::new())(&identified));

            // True / Not / And() / Or().
            assert!(all()(&it));
            assert!(!not(all())(&it));
            assert!(and()(&it));
            assert!(!or()(&it));
        }

        #[test]
        fn variadic_and_or_compose_like_object_filter_hpp() {
            let gd = load_game_data();
            let sword_def = gd
                .objects
                .iter()
                .position(|o| o.tval == crate::data::TV_SWORD)
                .expect("sword kind");
            let mut sword = Item::base(&gd, sword_def);
            sword.flags.push("SUST_STR".to_string());
            let bow = gd
                .objects
                .iter()
                .position(|o| o.tval == crate::data::TV_BOW)
                .expect("bow kind")
                ;
            let bow = Item::base(&gd, bow);

            let tval_sword = tval(&gd, crate::data::TV_SWORD);
            let tval_bow = tval(&gd, crate::data::TV_BOW);
            let has_sust = has_flags(&gd, &["SUST_STR"]);
            let never = has_flags(&gd, &["DEFINITELY_NOT_A_FLAG"]);

            // And(Arg0&&, Args&&...) = arg0(o) && args(o)...
            assert!(all_of(&[&tval_sword, &has_sust])(&sword));
            assert!(!all_of(&[&tval_sword, &has_sust])(&bow));
            assert!(!all_of(&[&tval_sword, &never])(&sword));
            // Or(Arg0&&, Args&&...) = arg0(o) || args(o)...
            assert!(any_of(&[&tval_bow, &has_sust])(&sword));
            assert!(any_of(&[&tval_bow, &tval_sword])(&sword));
            assert!(!any_of(&[&tval_sword, &never])(&bow));
            // The empty packs mirror And()/Or().
            let none: [&dyn Fn(&Item) -> bool; 0] = [];
            assert!(all_of(&none)(&sword));
            assert!(!any_of(&none)(&sword));
        }
    }
}

/// Port of the wizard/debug confirmation gates (dungeon.cc:2720
/// `enter_wizard_mode`, dungeon.cc:2748 `enter_debug_mode`).  The Bevy
/// port has no wizard or debug command set (autopilot is an env-gated
/// harness), so only the gates and their noscore marking are kept; the
/// UI confirmation is passed in as `confirmed`.
pub mod debug_mode {
    /// `enter_wizard_mode`: ask before the first entry (unless the save
    /// is already unscored or the character is dead) and flag the save
    /// with noscore bit 0x0002.
    pub fn enter_wizard_mode(noscore: &mut u32, hp: i32, confirmed: bool) -> bool {
        if *noscore == 0 && hp >= 0 {
            if !confirmed {
                return false;
            }
            *noscore |= 0x0002;
        }
        true
    }

    /// `enter_debug_mode`: noscore bit 0x0008; never prompts once wizard
    /// mode is on.
    pub fn enter_debug_mode(noscore: &mut u32, wizard: bool, confirmed: bool) -> bool {
        if *noscore == 0 && !wizard {
            if !confirmed {
                return false;
            }
            *noscore |= 0x0008;
        }
        true
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn wizard_and_debug_gates_follow_dungeon_cc() {
            let mut noscore = 0u32;
            assert!(!enter_wizard_mode(&mut noscore, 1, false));
            assert_eq!(noscore, 0, "refusal leaves the save scored");
            assert!(enter_wizard_mode(&mut noscore, 1, true));
            assert_eq!(noscore, 0x0002);
            // Already unscored: no confirmation needed.
            assert!(enter_wizard_mode(&mut noscore, 1, false));
            // A dead character (loaded with -w) skips the prompt.
            let mut fresh = 0u32;
            assert!(enter_wizard_mode(&mut fresh, -1, false));
            assert_eq!(fresh, 0);
            // Debug commands: prompt once, mark 0x0008, silent when wizard.
            let mut noscore = 0u32;
            assert!(!enter_debug_mode(&mut noscore, false, false));
            assert!(enter_debug_mode(&mut noscore, false, true));
            assert_eq!(noscore, 0x0008);
            let mut wizard_save = 0u32;
            assert!(enter_debug_mode(&mut wizard_save, true, false));
            assert_eq!(wizard_save, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn misc_txt_table_sizes_match() {
        // lib/edit/misc.txt M: lines.
        assert_eq!(super::MAX_TOWNS, 100);
        assert_eq!(super::NONRANDOM_TOWNS, 5);
        assert_eq!(super::WILD_X, 101);
        assert_eq!(super::WILD_Y, 66);
        assert_eq!(super::MAX_OBJECTS_PER_LEVEL, 1024);
        assert_eq!(super::MAX_MONSTERS_PER_LEVEL, 768);
        let gd = super::load_game_data();
        assert_eq!(gd.world.w, 101);
        assert_eq!(gd.world.h, 66);
        assert_eq!(gd.towns.iter().filter(|t| t.destroyed).count() + super::NONRANDOM_TOWNS, super::NONRANDOM_TOWNS * 2);
    }

    use super::*;

    #[test]
    fn dice_parse() {
        assert_eq!(
            Dice::parse("1d6"),
            Some(Dice {
                count: 1,
                sides: 6,
                bonus: 0
            })
        );
        assert_eq!(
            Dice::parse("2d4+3"),
            Some(Dice {
                count: 2,
                sides: 4,
                bonus: 3
            })
        );
        // dice.cc dice_parse forms: "B+NdM", "B+dM", "dM", "N".
        assert_eq!(
            Dice::parse("5+2d6"),
            Some(Dice {
                count: 2,
                sides: 6,
                bonus: 5
            })
        );
        assert_eq!(
            Dice::parse("7+d10"),
            Some(Dice {
                count: 1,
                sides: 10,
                bonus: 7
            })
        );
        assert_eq!(
            Dice::parse("d12"),
            Some(Dice {
                count: 1,
                sides: 12,
                bonus: 0
            })
        );
        assert_eq!(
            Dice::parse("9"),
            Some(Dice {
                count: 0,
                sides: 0,
                bonus: 9
            })
        );
        assert_eq!(Dice::parse("junk"), None);
        assert_eq!(Dice::parse("*"), None);
    }

    #[test]
    fn dice_roll_in_range() {
        let mut rng = crate::rng::current();
        let d = Dice::parse("2d6+1").unwrap();
        for _ in 0..100 {
            let v = d.roll(&mut rng);
            assert!((3..=13).contains(&v));
        }
    }

    #[test]
    fn dice_roll_keeps_negative_bonuses_and_zero_sided_dice() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(3);
        // dice.cc has no clamp: a 1d1-5 roll is negative.
        let d = Dice {
            count: 1,
            sides: 1,
            bonus: -5,
        };
        assert_eq!(d.roll(&mut rng), -4);
        let mut low = Dice {
            count: 0,
            sides: 0,
            bonus: -12,
        };
        assert_eq!(low.roll(&mut rng), -12);
        // randint(sides<2) is 1 per die (z-rand.cc:229), never a panic.
        low.count = 3;
        assert_eq!(low.roll(&mut rng), -9);
        let one = Dice {
            count: 2,
            sides: 0,
            bonus: 0,
        };
        assert_eq!(one.roll(&mut rng), 2);
    }

    #[test]
    fn range_init_and_device_allocations_follow_the_originals() {
        // range.cc:5 sets both bounds verbatim.
        let r = Range::new(3, 9);
        assert_eq!((r.min, r.max), (3, 9));
        let zero = Range::new(0, 0);
        assert_eq!((zero.min, zero.max), (0, 0));

        // device_allocation.cc:5/15: new(tval) zeroes the rarity and both
        // ranges; init sets the tval in place.
        let mut d = device_allocation_new(65);
        assert_eq!(d, (65, 0, 0, 0, 0, 0));
        device_allocation_init(&mut d, 55);
        assert_eq!(d, (55, 0, 0, 0, 0, 0));

        // The shipped rows use the same six-field layout: (tval, rarity,
        // base.min, base.max, max.min, max.max).
        let gd = load_game_data();
        let heal = gd
            .spells
            .iter()
            .find(|s| s.name == "Heal Monster")
            .expect("Heal Monster");
        assert_eq!(heal.alloc, vec![(65, 17, 1, 15, 20, 50)]);
        assert!(gd
            .spells
            .iter()
            .flat_map(|s| &s.alloc)
            .all(|a| a.0 > 0 && a.1 > 0 && a.2 <= a.3 && a.4 <= a.5));
    }

    #[test]
    fn k_info_keys_are_the_sorted_object_ids() {
        // game_edit_data.cc: keys() collects the k_info map keys and
        // sorts them.
        let gd = load_game_data();
        let keys = gd.k_info_keys();
        assert_eq!(keys.len(), gd.objects.len());
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
        assert!(!keys.is_empty());
        // Every id resolves back to a kind.
        for id in keys {
            assert!(gd.object_by_id(id).is_some(), "no kind for id {id}");
        }
    }

    #[test]
    fn game_data_loads() {
        let gd = load_game_data();
        assert!(gd.terrain(56).is_wall);
        assert!(gd.terrain(1).is_floor);
        assert!(gd.monsters.len() > 800);
        assert_eq!(gd.races.len(), 22);
        assert_eq!(gd.classes.len(), 6);
        // Every monster glyph must map into the foreground atlas.
        for m in &gd.monsters {
            let c = m.ch.chars().next().unwrap_or('?') as usize;
            assert!((32..=126).contains(&c), "bad glyph for {}", m.name);
        }
        // Objects and stores load; shop lookups resolve.
        assert!(gd.objects.len() > 300);
        assert_eq!(gd.stores.len(), 60);
        assert!(gd.object_by_name("Wooden Torch").is_some());
        assert!(gd.object_by_name("Short Sword").is_some());
        assert!(gd.object_by_tval_sval(71, 34).is_some()); // Cure Light Wounds
        assert!(gd.object_by_name("Arrow").is_some());
        assert!(gd.object_by_name("Short Bow").is_some());
        assert!(gd.object_by_name("Ration of Food").is_some());
        assert!(!gd.egos.is_empty());
        assert!(!gd.artifacts.is_empty());
        assert!(!gd.schools.is_empty());
        assert!(!gd.spells.is_empty());
        // Spellbooks ported (school tome sval 0 = Mana, cantrips sval 50).
        assert!(gd.object_by_tval_sval(TV_BOOK, 0).is_some());
        assert!(gd.object_by_tval_sval(TV_BOOK, 50).is_some());
        // Ego/artifact ability flags and activations parsed.
        assert!(gd.egos.iter().any(|e| !e.flags.is_empty()));
        assert!(gd.artifacts.iter().any(|a| !a.activate.is_empty()));
        // The One Ring: a_info 13 (the wearable Ring of Power) is kept as
        // an INSTA_ART quest artifact; k_info 664 is the lore parchment.
        assert!(gd.object_by_name("The One Ring").is_some());
        let one = gd
            .artifacts
            .iter()
            .find(|a| a.id == 13)
            .expect("One Ring artifact");
        assert!(one.insta_art);
        assert_eq!(one.tval, TV_RING);
        // Device spells resolve to spell rows.
        for o in &gd.objects {
            if !o.spell.is_empty() {
                assert!(gd.spell_by_name(&o.spell).is_some(), "{}", o.spell);
            }
        }
        for o in &gd.objects {
            let c = o.glyph() as usize;
            assert!((32..=126).contains(&c), "bad glyph for {}", o.name);
        }
        // Mushrooms (TV_FOOD sval 0-19) are ported for the shroom quest.
        let shrooms = gd
            .objects
            .iter()
            .filter(|o| o.tval == TV_FOOD && (0..=19).contains(&o.sval))
            .count();
        assert_eq!(shrooms, 20);
        assert!(gd.object_by_name("Mushroom of Cure Poison").is_some());
        // The world map: the destroyed-Gondolin branch swaps its symbol.
        let gond = gd.town_cell(2);
        let normal = gd.world.feat(gond.0, gond.1, &[false; 6]);
        let mut destroyed = [false; 6];
        destroyed[2] = true;
        let ruined = gd.world.feat(gond.0, gond.1, &destroyed);
        assert_ne!(normal, ruined);
        assert_eq!(gd.wf(ruined).town(), 2);
        // Every dungeon's depths, guardians and special maps resolve.
        for d in &gd.dungeons {
            assert!(d.mindepth <= d.maxdepth, "dungeon {}", d.name);
            assert!(!d.floors.is_empty());
            if let Some(g) = d.guardian {
                assert!(gd.monster_by_id(g).is_some(), "guardian {}", g);
            }
            for sp in &d.specials {
                if !sp.map.is_empty() {
                    assert!(gd.spec_level(&sp.map).is_some(), "map {}", sp.map);
                }
            }
        }
        // Moria's branch staircase leads to the Small Water Cave.
        let moria = gd.dungeon(22);
        assert_eq!(moria.branch_at(40), Some(24));
        assert_eq!(gd.dungeon(24).branch_parent, Some((22, 40)));
        // The fixed Bree map and the fixed quest levels load.
        assert_eq!(gd.wf.len(), 27);
        assert_eq!(gd.world.w, 101);
        assert_eq!(gd.world.h, 66);
        assert_eq!(gd.world.entrances.len(), 13);
        assert_eq!(gd.world.patches.len(), 1);
        assert_eq!(gd.dungeons.len(), 28);
        assert_eq!(gd.towns.len(), 10);
        for t in &gd.towns {
            assert_eq!(t.rows.len(), 66);
        }
        let bree = gd.town(1, false).expect("normal Bree");
        // Bree's map: thieves/troll/wight quest entrances + the Barrow-Downs
        // dungeon stair from t_pref.txt.
        assert!(bree.chars.iter().any(|c| c.ch == 'z' && c.terrain == 8));
        assert!(bree.chars.iter().any(|c| c.ch == '{' && c.terrain == 7));
        assert_eq!(gd.quest_maps.len(), 22);
        assert_eq!(gd.spec_levels.len(), 8);
        for qm in &gd.quest_maps {
            assert_eq!(qm.cells.len(), (qm.w * qm.h) as usize);
        }
        for sl in &gd.spec_levels {
            assert_eq!(sl.cells.len(), (sl.w * sl.h) as usize);
        }
        // Bards/Possessors/... are p_info specialisations of the six base
        // classes, each carrying its own skills, gear and god restriction.
        let loremaster = gd.classes.iter().find(|c| c.name == "Loremaster").unwrap();
        assert!(loremaster.specs.iter().any(|s| s.name == "Bard"));
        assert!(loremaster.specs.iter().any(|s| s.name == "Possessor"));
        assert!(loremaster.specs.iter().any(|s| s.name == "Summoner"));
        let priest = gd.classes.iter().find(|c| c.name == "Priest").unwrap();
        assert!(priest.specs.iter().any(|s| s.name == "Druid"));
        let warrior = gd.classes.iter().find(|c| c.name == "Warrior").unwrap();
        assert!(warrior.specs.iter().any(|s| s.name == "Demonologist"));
        assert_eq!(gd.classes.iter().map(|c| c.specs.len()).sum::<usize>(), 30);
        // Specs carry their starting gear and god restriction.
        let bard = loremaster.specs.iter().find(|s| s.name == "Bard").unwrap();
        assert!(bard.objects.iter().any(|o| o.tval == TV_INSTRUMENT));
        assert!(bard.skills.iter().any(|s| s.skill == "Music"));
        let druid = priest.specs.iter().find(|s| s.name == "Druid").unwrap();
        assert_eq!(druid.gods, vec!["Yavanna Kementari".to_string()]);
        let warrior_spec = warrior.specs.iter().find(|s| s.name == "Warrior").unwrap();
        assert_eq!(warrior_spec.gods, vec!["All Gods".to_string()]);
        // Music school and god-specific spells exist.
        assert!(gd.schools.iter().any(|s| s.id == 100));
        assert!(gd.spells.iter().any(|s| s.god == "Melkor"));
        assert!(gd.spells.iter().any(|s| s.school == 100));
    }

    #[test]
    fn summoner_totems_and_remaining_class_items_are_ported() {
        let gd = load_game_data();
        let partial = gd.object_by_tval_sval(TV_TOTEM, 1).expect("partial totem");
        let true_totem = gd.object_by_tval_sval(TV_TOTEM, 2).expect("true totem");
        assert_eq!(gd.objects[partial].name, "Partial Totem");
        assert_eq!(gd.objects[true_totem].name, "True Totem");
        // Druid Elemental Stones and Daemonologist books came along with
        // the Summoner totems.
        assert!(gd.object_by_tval_sval(TV_DRUID_BOOK, 0).is_some());
        assert!(gd.object_by_tval_sval(TV_DAEMON_BOOK, 55).is_some());
        // A totem is labelled after the monster it holds.
        let maggot = gd.monster_by_name("Farmer Maggot").expect("maggot");
        let mut it = crate::item::Item::base(&gd, partial);
        it.note = maggot as u32;
        assert!(it.label(&gd, &Default::default()).contains("Maggot"));
    }

    #[test]
    fn levels_cc_level_data_accessors_match_the_original() {
        let gd = load_game_data();
        // Moria's `@:` rows: Orc Town at depth 35, the SWC branch at 40.
        let town = gd.level_data(22, 35).expect("Orc Town record");
        assert_eq!(town.name, "Orc Town");
        assert_eq!(town.map, "s_orc.map");
        assert_eq!(gd.level_name(22, 35), Some("Orc Town"));
        assert_eq!(gd.level_map_name(22, 35), Some("s_orc.map"));
        assert_eq!(
            gd.level_description(22, 35),
            Some("You hear orc warcries.")
        );
        assert_eq!(
            gd.level_flags(22, 35),
            ["NO_GENO", "NO_NEW_MONSTER", "SPECIAL", "NO_STAIR", "ASK_LEAVE", "NO_TELEPORT"]
        );
        // The default (no level_data) record yields zero/empty values.
        assert!(gd.level_data(22, 36).is_none());
        assert_eq!(gd.level_branch(22, 36), 0);
        assert_eq!(gd.level_flags(22, 36), Vec::<&str>::new());
        assert_eq!(gd.level_name(22, 36), None);
        assert_eq!(gd.level_map_name(22, 36), None);
        assert_eq!(gd.level_description(22, 36), None);

        // get_branch: the branch staircase at Moria 40 -> SWC (24).
        assert_eq!(gd.level_branch(22, 40), 24);
        assert_eq!(gd.level_branch(22, 35), 0);

        // get_fbranch/get_flevel: only the child's first floor carries the
        // back-reference; flevel is parent_depth - parent.mindepth
        // (40 - 30 = 10), as post_d_info computes it.
        assert_eq!(gd.level_fbranch(24, 32), 22);
        assert_eq!(gd.level_flevel(24, 32), 10);
        assert_eq!(gd.level_fbranch(24, 33), 0);
        assert_eq!(gd.level_flevel(24, 33), 0);
        assert_eq!(gd.level_fbranch(22, 30), 0);
        assert_eq!(gd.level_flevel(22, 30), 0);

        // get_dungeon_save_extension: the shipped d_info.txt has exactly one
        // `@:S:` row, Mount Doom's `@:14:S:mdm` (mindepth 85, so it
        // resolves to absolute depth 99).  Every other level is unset.
        let extensions: Vec<(u32, u32, &str)> = gd
            .dungeons
            .iter()
            .flat_map(|d| {
                d.specials.iter().filter_map(move |sp| {
                    sp.save_extension
                        .as_deref()
                        .map(|ext| (d.id, sp.depth, ext))
                })
            })
            .collect();
        assert_eq!(extensions, [(5, 99, "mdm")]);
        assert_eq!(gd.level_save_extension(5, 99), Some("mdm"));
        // The mindepth-relative key (14) is not an absolute depth.
        assert_eq!(gd.level_save_extension(5, 14), None);
        assert_eq!(gd.level_save_extension(5, 98), None);
        assert_eq!(gd.level_save_extension(22, 35), None);
    }

    #[test]
    fn init_guardians_marks_final_treasures_and_randart_guardians() {
        let mut gd = load_game_data();
        // Strip the generated marks, then let the runtime pass re-apply
        // them exactly as init2.cc init_guardians does.
        for m in &mut gd.monsters {
            m.flags
                .retain(|f| f != "SPECIAL_GENE" && f != "DROP_RANDART");
        }
        for a in &mut gd.artifacts {
            a.flags.retain(|f| f != "SPECIAL_GENE");
        }
        for o in &mut gd.objects {
            o.flags.retain(|f| f != "SPECIAL_GENE");
        }
        super::apply_guardian_flags(&mut gd);
        let mut guarded = 0;
        for d in &gd.dungeons {
            let Some(g) = d.guardian else { continue };
            guarded += 1;
            let mi = gd.monster_by_id(g).expect("guardian monster");
            assert!(
                gd.monsters[mi].flags.iter().any(|f| f == "SPECIAL_GENE"),
                "{} is not SPECIAL_GENE",
                gd.monsters[mi].name
            );
            if let Some(aid) = d.final_artifact {
                let ai = gd
                    .artifacts
                    .iter()
                    .position(|a| a.id == aid)
                    .expect("final artifact");
                assert!(gd.artifacts[ai].flags.iter().any(|f| f == "SPECIAL_GENE"));
            }
            if let Some(oid) = d.final_object {
                let oi = gd.object_by_id(oid).expect("final object");
                assert!(gd.objects[oi].flags.iter().any(|f| f == "SPECIAL_GENE"));
            }
            if d.final_artifact.is_none() && d.final_object.is_none() {
                assert!(
                    gd.monsters[mi].flags.iter().any(|f| f == "DROP_RANDART"),
                    "{} does not drop a randart",
                    gd.monsters[mi].name
                );
            }
        }
        assert!(guarded >= 7, "dungeons with guardians: {guarded}");
    }

    #[test]
    #[should_panic(expected = "missing or broken")]
    fn init_angband_aux_reports_a_broken_data_directory() {
        super::broken_lib_panic("cannot open 'data/terrain.ron'");
    }
}

#[cfg(test)]
mod dice_print_tests {
    use super::Dice;

    #[test]
    fn dice_print_matches_the_original() {
        let d = |count, sides, bonus| Dice { count, sides, bonus };
        assert_eq!(d(2, 6, 1).print(), "1+2d6");
        assert_eq!(d(1, 6, 0).print(), "d6");
        assert_eq!(d(0, 0, 5).print(), "5");
        assert_eq!(d(2, 6, 0).print(), "2d6");
        assert_eq!(d(0, 8, 3).print(), "3+d8");
    }
}

/// Audit tests for init1.cc: every loaded record family is checked against
/// the original template-file grammar and the shipped data.
#[cfg(test)]
mod init1_audit_tests {
    use super::*;
    use std::sync::OnceLock;

    fn gd() -> &'static GameData {
        static GD: OnceLock<GameData> = OnceLock::new();
        GD.get_or_init(load_game_data)
    }

    #[test]
    fn init1_expand_fit_and_dup_append() {
        let gd = gd();
        // expand_to_fit_index: the terrain table is indexed by f_info id and
        // holes up to the maximum id are filled with the "nothing" record.
        assert!(gd.terrain.len() > 216);
        assert_eq!(gd.terrain[0].name, "nothing");
        assert_eq!(gd.terrain[56].name, "granite wall");
        // my_strdup/strappend: r_info D: lines are appended with no
        // separator (all 768 monster texts concatenate).
        for m in &gd.monsters {
            assert!(!m.desc.is_empty() || m.id == 0, "empty monster text {}", m.id);
        }
        let m = gd.monster_by_name("Farmer Maggot").unwrap();
        assert_eq!(
            gd.monsters[m].desc,
            "He's lost his dogs. He's had his mushrooms stolen. He's not a happy hobbit! "
        );
    }

    #[test]
    fn init1_flag_ties_and_color_letters() {
        let gd = gd();
        // flag_tie/lookup_flags: the C++ mask + name arrays become the
        // Vec<String> flag lists checked through has().
        let wall = gd.terrain(56);
        assert!(wall.is_wall && wall.no_walk && wall.no_vision && wall.tunnelable && wall.can_pass);
        assert!(gd.terrain(1).can_run && gd.terrain(1).support_light);
        let maggot = gd.monsters[gd.monster_by_name("Farmer Maggot").unwrap()].clone();
        assert!(maggot.has("NEVER_MOVE") && maggot.has("UNIQUE") && maggot.open_door);
        // color_char_to_attr: G:<ch>:<letter> through the 16 letter table.
        assert_eq!(gd.terrain(85).color, 12); // G:.:R -> TERM_L_RED
        assert_eq!(gd.terrain(2).color, 1); // G:_:w
        assert_eq!(maggot.color, 1); // G:h:w
    }

    #[test]
    fn init1_skill_mod_proto_ability_grammar() {
        let gd = gd();
        // read_skill_modifiers: '<op><n>:<op><n>:<skill>' keeps both
        // operators and values.
        let human = &gd.races[0];
        assert_eq!(
            (
                human.skills.len(),
                human.stats,
                human.hitdie,
                human.exp,
                human.infra,
                human.luck
            ),
            (0, [0; 6], 10, 100, 0, 0)
        );
        let war = gd.classes.iter().find(|c| c.name == "Warrior").unwrap();
        let combat = war.skills.iter().find(|s| s.skill == "Combat").unwrap();
        assert_eq!(
            (
                combat.bop.as_str(),
                combat.base,
                combat.mop.as_str(),
                combat.gain
            ),
            ("+", 2000, "+", 800)
        );
        // read_proto_object: tval:sval:xdy with the optional pval defaulting
        // to 0 (the 4-field form parses the same way).
        let p = war.objects[0];
        assert_eq!((p.tval, p.sval, p.pval, p.dd, p.ds), (45, 38, 0, 1, 1));
        // read_ability: '<level>:<ability name>' resolved by name.
        assert_eq!(
            (war.abilities[0].level, war.abilities[0].ability.as_str()),
            (25, "Spread blows")
        );
    }

    #[test]
    fn init1_class_race_skill_player_and_object_flags() {
        let gd = gd();
        // grab_one_player_race_flag.
        let rogue = gd.classes.iter().find(|c| c.name == "Rogue").unwrap();
        assert!(rogue.player_flags.iter().any(|f| f == "EASE_STEAL"));
        // grab_one_race_allow_flag + grab_one_class_flag (S:A:/S:C:).
        let vampire = gd.racemods.iter().find(|r| r.name == "Vampire").unwrap();
        assert!(vampire.races.iter().any(|r| r == "Human"));
        assert_eq!(vampire.classes, vec!["Mage".to_string()]);
        assert!(vampire.forbidden_classes.is_empty());
        // grab_one_skill_flag.
        assert!(gd.skills.iter().find(|s| s.name == "Combat").unwrap().has("RANDOM_GAIN"));
        assert!(gd.skills.iter().find(|s| s.id == 54).unwrap().has("HIDDEN"));
        // object_flag_set_from_string / grab_object_flag (k_info F:).
        let blood = &gd.objects[gd.object_by_id(573).unwrap()];
        assert!(blood.flags.iter().any(|f| f == "FULL_NAME"));
        assert!(blood.flags.iter().any(|f| f == "NORM_ART"));
    }

    #[test]
    fn init1_activation_and_power_lookup() {
        let gd = gd();
        // get_activation: the a: name is kept verbatim and resolved at use
        // time instead of the stored table index.
        assert_eq!(gd.objects[gd.object_by_id(138).unwrap()].activate, "DEST_TELE");
        assert_eq!(
            gd.objects[gd.object_by_id(296).unwrap()].activate,
            "ETERNAL_FLAME"
        );
        let phial = gd.artifacts.iter().find(|a| a.id == 1).unwrap();
        assert_eq!(phial.activate, "LIGHT");
        let one = gd.artifacts.iter().find(|a| a.id == 13).unwrap();
        assert_eq!(one.activate, "POWER");
        // find_power_idx: Z: names resolve to the known power list.
        assert_eq!(one.powers, vec!["change the world".to_string()]);
        assert_eq!(
            gd.races.iter().find(|r| r.name == "Beorning").unwrap().powers,
            vec!["turn into a bear".to_string()]
        );
        assert!(gd.egos.iter().any(|e| e.powers.iter().any(|p| p == "blink")));
    }

    #[test]
    fn init1_player_info_records() {
        let gd = gd();
        assert_eq!(gd.races.len(), 22);
        assert_eq!(gd.classes.len(), 6);
        let half_elf = gd.races.iter().find(|r| r.name == "Half-Elf").unwrap();
        assert_eq!(half_elf.stats, [0, 1, 1, 1, -1, 1]);
        assert_eq!(half_elf.hitdie, 9);
        assert_eq!(half_elf.infra, 2);
        let md = half_elf
            .skills
            .iter()
            .find(|s| s.skill == "Magic-Device")
            .unwrap();
        assert_eq!((md.base, md.gain), (300, 0));
        assert!(half_elf.player_flags.iter().any(|f| f == "ELF"));
        let elf = gd.races.iter().find(|r| r.name == "Elf").unwrap();
        let lite = elf.flags.iter().find(|f| f.flag == "RES_LITE").unwrap();
        assert_eq!((lite.level, lite.pval), (1, 0));
        // R:O starting gear and R:b abilities.
        let beorning = gd.races.iter().find(|r| r.name == "Beorning").unwrap();
        assert_eq!(
            (beorning.objects.len(), beorning.body_parts),
            (0, [1, 1, 1, 2, 1, 1])
        );
        let war = gd.classes.iter().find(|c| c.name == "Warrior").unwrap();
        assert_eq!(war.stats, [5, -2, -2, 2, 2, -1]);
        assert_eq!((war.hitdie, war.exp, war.blow_num, war.blow_wgt, war.blow_mul), (9, 0, 4, 30, 5));
        let fear = &war.flags[0];
        assert_eq!((fear.level, fear.pval, fear.flag.as_str()), (30, 0, "RES_FEAR"));
        assert_eq!(war.titles[0], "Rookie");
        // C:g gods and the C:a: specialisations.
        let swordmaster = war.specs.iter().find(|s| s.name == "Swordmaster").unwrap();
        assert_eq!(swordmaster.gods[0], "Nobody");
        assert!(swordmaster
            .skills
            .iter()
            .any(|s| s.skill == "Sword-mastery" && s.base == 1000 && s.gain == 300));
        assert_eq!(gd.classes.iter().map(|c| c.specs.len()).sum::<usize>(), 30);
        // G:k: general skills.
        assert_eq!(gd.general_skills.len(), 5);
        assert_eq!(
            (
                gd.general_skills[0].skill.as_str(),
                gd.general_skills[0].base,
                gd.general_skills[0].gain
            ),
            ("Monster-lore", 0, 500)
        );
    }

    #[test]
    fn init1_vault_records() {
        let gd = gd();
        assert_eq!(gd.vaults.len(), 103);
        let v = gd.vaults.iter().find(|v| v.id == 1).unwrap();
        assert_eq!((v.typ, v.rat, v.hgt, v.wid), (7, 5, 14, 20));
        assert_eq!(v.data.len(), (v.hgt * v.wid) as usize);
        assert!(v.data.starts_with("   %%%"));
        // Holes in the N: numbering are skipped; the wilderness test record
        // (typ 10) is dead data kept for completeness.
        assert!(gd.vaults.iter().any(|v| v.typ == 10));
        assert!(gd.vaults.iter().all(|v| v.id < 256));
    }

    #[test]
    fn init1_terrain_records() {
        let gd = gd();
        // grab_one_feature_flag + init_f_info_txt.
        let wall = gd.terrain(56);
        assert!(wall.is_wall && wall.tunnelable && wall.can_pass && wall.support_light);
        assert_eq!(wall.tunnel_desc, "You tunnel into the granite wall.");
        assert_eq!(gd.terrain(57).mimic, 56);
        let lava = gd.terrain(85);
        assert_eq!(
            (
                lava.effects[0].dd,
                lava.effects[0].ds,
                lava.effects[0].freq,
                lava.effects[0].typ.as_str()
            ),
            (-1, 2, 10, "FIRE")
        );
        assert!(lava.can_fly && lava.can_levitate && lava.remember && lava.support_light);
        assert_eq!(lava.desc, "You move across the deep lava.");
        let ice = gd.terrain(90);
        assert_eq!(
            (ice.effects[0].ds, ice.effects[0].freq, ice.effects[0].typ.as_str()),
            (1, 500, "ICE")
        );
        let great_fire = gd.terrain(178);
        assert!(great_fire.permanent && great_fire.attr_multi);
        assert_eq!(great_fire.effects[0].typ, "HELL_FIRE");
        let fountain = gd.terrain(2);
        assert!(fountain.remember && fountain.notice && fountain.is_floor);
        assert_eq!(fountain.mimic, 2);
    }

    #[test]
    fn init1_object_records() {
        let gd = gd();
        // G: glyph/colour, I: tval/sval/pval/fuel, W: weight/cost.
        let torch = &gd.objects[gd.object_by_name("Wooden Torch").unwrap()];
        assert_eq!(torch.name, "Wooden Torch"); // &/~ stripped by test_item_name
        assert_eq!(
            (torch.ch.as_str(), torch.color, torch.tval, torch.sval),
            ("~", 7, 39, 0)
        );
        assert_eq!((torch.fuel, torch.lite, torch.weight, torch.cost), (4000, 1, 30, 2));
        assert!(torch.flags.iter().any(|f| f == "EASY_KNOW"));
        assert_eq!(torch.desc.len(), 2);
        // I: ...:SPELL=name resolves the device spell.
        let mana = &gd.objects[gd.object_by_name("Manathrust").unwrap()];
        assert_eq!((mana.spell.as_str(), mana.pval, mana.depth), ("Manathrust", -1, 3));
        // A: allocation rows (`level/chance` pairs).
        let spell_stick = &gd.objects[gd.object_by_id(269).unwrap()];
        assert_eq!(
            spell_stick.alloc,
            vec![(3, 1), (13, 1), (23, 1), (43, 1), (63, 1), (83, 1)]
        );
        // T: btval/bsval NORM_ART marker and a: activation.
        let flame = &gd.objects[gd.object_by_id(296).unwrap()];
        assert_eq!((flame.btval, flame.bsval), (39, 2));
        assert!(flame.flags.iter().any(|f| f == "ACTIVATE_NO_WIELD"));
        let precog = &gd.objects[gd.object_by_id(700).unwrap()];
        assert_eq!((precog.btval, precog.bsval), (45, 23));
    }

    #[test]
    fn init1_artifact_records() {
        let gd = gd();
        assert!(gd.artifacts.len() > 150);
        let phial = gd.artifacts.iter().find(|a| a.id == 1).unwrap();
        assert_eq!(
            (
                phial.name.as_str(),
                phial.tval,
                phial.sval,
                phial.pval,
                phial.depth,
                phial.rarity,
                phial.weight,
                phial.cost,
                phial.dice.as_str()
            ),
            ("of Galadriel", 39, 100, 4, 20, 10, 10, 10000, "1d1")
        );
        assert!(phial.insta_art);
        assert!(phial.flags.iter().any(|f| f == "LITE3"));
        let one = gd.artifacts.iter().find(|a| a.id == 13).unwrap();
        assert_eq!((one.to_h, one.to_d, one.pval, one.cursed), (15, 15, 5, true));
        // a_info 201 is the special corpse artifact (kept by id).
        let corpse = gd.artifacts.iter().find(|a| a.id == 201).unwrap();
        assert_eq!((corpse.tval, corpse.sval, corpse.name.as_str()), (9, 1, ""));
        assert!(gd.artifacts.iter().any(|a| !a.insta_art));
    }

    #[test]
    fn init1_set_records() {
        let gd = gd();
        assert_eq!(gd.sets.len(), 4);
        let elven = gd.sets.iter().find(|s| s.id == 0).unwrap();
        assert_eq!(elven.name, "Elven Gifts");
        assert_eq!(elven.members.len(), 2);
        let art1 = elven.members.iter().find(|m| m.artifact == 1).unwrap();
        assert_eq!(art1.tiers.len(), 2);
        assert_eq!(art1.tiers[0].pval, 0);
        assert!(art1.tiers[0].flags.is_empty());
        assert_eq!(art1.tiers[1].pval, 1);
        assert!(art1.tiers[1].flags.iter().any(|f| f == "RES_DARK"));
        let art88 = elven.members.iter().find(|m| m.artifact == 88).unwrap();
        assert_eq!(art88.tiers[1].pval, 2);
        assert!(art88.tiers[1].flags.iter().any(|f| f == "REGEN"));
    }

    #[test]
    fn init1_school_and_skill_records() {
        let gd = gd();
        // init_s_info_txt: only A:17: schools cast; the Theme school 100 is
        // registered but not castable.
        assert!(gd.schools.iter().any(|s| s.id == 2 && s.name == "Mana" && s.cast));
        assert!(gd.schools.iter().any(|s| s.id == 100 && s.name == "Music" && !s.cast));
        // Full skill tree: N: descriptors plus f:/E:/T: relations.
        assert!(gd.skills.len() > 40);
        let combat = gd.skills.iter().find(|s| s.name == "Combat").unwrap();
        assert_eq!(combat.id, 16);
        assert!(combat.has("RANDOM_GAIN"));
        let sword = gd.skills.iter().find(|s| s.name == "Sword-mastery").unwrap();
        assert_eq!(sword.father, 17); // Weaponmastery
        let music = gd.skills.iter().find(|s| s.name == "Music").unwrap();
        assert_eq!((music.father, music.action_mkey), (28, 17));
        assert_eq!(music.action_desc, "Cast a spell");
        assert!(music.excludes.contains(&33)); // E:Music:Antimagic
        assert!(music.increases.contains(&(15, 10))); // f:Music:Magic%10
        let geomancy = gd.skills.iter().find(|s| s.name == "Geomancy").unwrap();
        assert_eq!(geomancy.father, 15);
        assert!(geomancy.increases.contains(&(3, 45))); // f:Geomancy:Fire%45
        assert!(geomancy.excludes.contains(&33));
        // G: random-gain chance and unplaced/0 defaults.
        let spell_learning = gd.skills.iter().find(|s| s.id == 54).unwrap();
        assert_eq!(spell_learning.chance, 100);
        assert!(spell_learning.has("HIDDEN"));
    }

    #[test]
    fn init1_ability_records() {
        let gd = gd();
        assert!(gd.abilities.len() >= 9);
        let spread = &gd.abilities[0];
        assert_eq!((spread.name.as_str(), spread.cost), ("Spread blows", 5));
        assert_eq!(spread.need_skills, vec![("Combat".to_string(), 30)]);
        assert_eq!(spread.stats[3], 17); // Dex
        assert_eq!(spread.stats[0], -1);
        let perfect = gd
            .abilities
            .iter()
            .find(|a| a.name == "Perfect casting")
            .unwrap();
        assert_eq!(perfect.need_skills, vec![("Magic".to_string(), 35)]);
        assert!(perfect.desc.contains("0% failure rate"));
    }

    #[test]
    fn init1_ego_flag_lookup_and_records() {
        let gd = gd();
        assert!(gd.egos.len() > 100);
        let fire = gd.egos.iter().find(|e| e.id == 7).unwrap();
        assert_eq!(fire.name, "of Resist Fire");
        assert_eq!(fire.tvals, vec![36, 36, 37]);
        let tv = fire.svals[1];
        assert_eq!((tv.0, tv.1, tv.2), (36, 17, 255));
        assert_eq!(
            (fire.depth, fire.rarity, fire.rarity1, fire.cost, fire.rating),
            (0, 20, 4, 800, 14)
        );
        assert_eq!(fire.groups.len(), 1);
        assert_eq!(fire.groups[0].chance, 100);
        assert!(fire.groups[0].flags.iter().any(|f| f == "RES_FIRE"));
        assert!(fire.groups[0].oflags.iter().any(|f| f == "IGNORE_FIRE"));
        // ETR generation flags stay out of the applied object flags
        // (lookup_ego_flag) and live in the rarity group.
        let mana = gd.egos.iter().find(|e| e.id == 1).unwrap();
        assert!(mana.groups[0].flags.iter().any(|f| f == "MANA"));
        assert!(mana.groups[1].fego.iter().any(|f| f == "PVAL_M2"));
        assert!(!mana.flags.iter().any(|f| f == "PVAL_M2"));
        assert!(mana.groups[1].chance == 70);
        assert_eq!(mana.activate, "");
    }

    #[test]
    fn init1_randart_records() {
        let gd = gd();
        assert_eq!(gd.randarts.parts.len(), 190);
        let p = gd.randarts.parts.iter().find(|p| p.id == 1).unwrap();
        assert_eq!((p.tvals[0].tval, p.tvals[0].min, p.tvals[0].max), (6, 0, 255));
        assert_eq!(
            (p.level, p.rarity, p.mrarity, p.to_h, p.to_d, p.pval, p.value, p.max),
            (5, 1, 4, -5, -5, 5, 10, 1)
        );
        assert!(p.flags.iter().any(|f| f == "MANA"));
        // Power budget rows and the name corpus.
        assert_eq!(gd.randarts.gen.len(), 4);
        let g0 = gd.randarts.gen[0];
        assert_eq!((g0.chance, g0.dd, g0.ds, g0.plus), (100, 1, 5, 1));
        assert_eq!(gd.randarts.names.len(), 4731);
        // Legacy TV_RANDART junkarts.
        assert_eq!(gd.randarts.junk_s.len(), 84);
        assert_eq!(gd.randarts.junk_f.len(), 84);
        assert_eq!(gd.randarts.acts.len(), 51);
        assert_eq!(gd.randarts.acts[0].act, "DEATH");
    }

    #[test]
    fn init1_monster_records() {
        let gd = gd();
        assert_eq!(gd.monster_base_count, 893);
        let maggot = &gd.monsters[gd.monster_by_name("Farmer Maggot").unwrap()];
        assert_eq!((maggot.speed, maggot.hp.as_str(), maggot.aaf, maggot.ac, maggot.alert), (110, "35d10", 40, 10, 3));
        assert_eq!((maggot.depth, maggot.rarity, maggot.weight, maggot.exp), (0, 4, 730, 0));
        assert_eq!(maggot.objs, (0, 100, 0, 0));
        assert_eq!(maggot.body_parts, [0, 1, 1, 2, 1, 1]);
        assert!(maggot.unique && maggot.never_move && maggot.has("WILD_TOWN"));
        // S:1_IN_n is stored as n (1-in-n) for base monsters.
        let hell = &gd.monsters[gd.monster_by_id(39).unwrap()];
        assert_eq!((hell.hdice.as_str(), hell.hside.as_str()), ("15", "100"));
        assert_eq!(hell.spell_freq, 9);
        assert_eq!(hell.spells, vec!["BLINK", "TELE_AWAY", "TPORT"]);
        assert_eq!(hell.blows.len(), 3);
        assert_eq!(
            (
                hell.blows[0].method.as_str(),
                hell.blows[0].effect.as_str(),
                hell.blows[0].dice.as_str()
            ),
            ("GAZE", "*", "0d0")
        );
        // A: standard artifact drop.
        let eol = &gd.monsters[gd.monster_by_id(660).unwrap()];
        assert_eq!((eol.artifact_idx, eol.artifact_chance), (84, 50));
        assert_eq!(eol.blows[0].dice, "3d8");
    }

    #[test]
    fn init1_monster_ego_records() {
        let gd = gd();
        // monster_ego_modify via apply_mod: + - = % operators, unknown is
        // an add.
        assert_eq!(apply_mod(10, "+5"), 15);
        assert_eq!(apply_mod(10, "-5"), 5);
        assert_eq!(apply_mod(10, "=5"), 5);
        assert_eq!(apply_mod(10, "%50"), 5);
        assert_eq!(apply_mod(10, ""), 10);
        assert_eq!(gd.monster_egos.len(), 13);
        let zombie = gd.monster_egos.iter().find(|e| e.id == 2).unwrap();
        assert_eq!(
            (
                zombie.speed.as_str(),
                zombie.hdice.as_str(),
                zombie.hside.as_str(),
                zombie.aaf.as_str(),
                zombie.ac.as_str(),
                zombie.sleep.as_str()
            ),
            ("%95", "%110", "%100", "%90", "+10", "-5")
        );
        assert!(zombie.before);
        assert_eq!(zombie.rarity, 14);
        assert!(zombie.req_flags.iter().any(|f| f == "DROP_CORPSE"));
        assert!(zombie.forbid_chars.iter().any(|c| c == "A"));
        assert!(zombie.remove_spells.iter().any(|s| s == "MF_ALL"));
        assert!(zombie.add_flags.iter().any(|f| f == "UNDEAD"));
        // race_info_idx: modifiers apply with the original floors, keeping
        // the aaf modification.
        let maggot = &gd.monsters[gd.monster_by_name("Farmer Maggot").unwrap()];
        let v = apply_monster_ego(maggot, zombie);
        assert_eq!((v.aaf, v.speed, v.ac, v.alert), (36, 104, 20, 0));
        assert_eq!(v.hp, "38d10");
        assert!(v.name.starts_with("Zombie "));
        assert!(v.has("UNDEAD") && !v.has("MORTAL"));
        // S:1_IN_4 egos take the better frequency in the 1-in-n scale.
        let lich = gd.monster_egos.iter().find(|e| e.id == 3).unwrap();
        assert_eq!(lich.spell_freq, 25);
        let hell = &gd.monsters[gd.monster_by_id(39).unwrap()];
        assert_eq!(apply_monster_ego(hell, lich).spell_freq, 4);
    }

    #[test]
    fn init1_dungeon_records() {
        let gd = gd();
        assert_eq!(gd.dungeons.len(), 28);
        let moria = gd.dungeon(22);
        assert_eq!((moria.mindepth, moria.maxdepth, moria.min_plev, moria.min_alloc, moria.max_chance), (30, 50, 20, 40, 40));
        assert_eq!(
            (moria.floors[0].feat, moria.floors[0].top, moria.floors[0].bottom),
            (88, 100, 100)
        );
        assert_eq!(moria.fills[0].feat, 97);
        assert_eq!((moria.outer_wall, moria.inner_wall), (57, 97));
        assert_eq!(moria.theme, (30, 50, 10, 5));
        assert!(moria.has("FORCE_DOWN") && moria.has("RANDOM_TOWNS"));
        // R: rules with their monster chars and the flat percent remap.
        assert_eq!(moria.rules.len(), 4);
        assert!(moria.rules[0].chars.is_empty());
        assert_eq!(moria.rules[0].mflags, vec!["ORC"]);
        assert_eq!(moria.rules[0].pct, 40);
        assert_eq!(moria.rules[1].mflags, vec!["GIANT", "TROLL"]);
        assert_eq!(moria.rules[2].mflags, vec!["DEMON"]);
        assert_eq!(
            (
                moria.rule_percents[0],
                moria.rule_percents[39],
                moria.rule_percents[40],
                moria.rule_percents[69],
                moria.rule_percents[70],
                moria.rule_percents[90],
                moria.rule_percents[99]
            ),
            (0, 0, 1, 1, 2, 3, 3)
        );
        // @: level data and post_d_info back-references.
        assert_eq!(moria.guardian, Some(872));
        assert_eq!((moria.ix, moria.iy, moria.ox, moria.oy), (45, 30, 44, 37));
        assert_eq!(moria.specials[0].depth, 35);
        assert_eq!(moria.specials[0].name, "Orc Town");
        assert_eq!(moria.specials[0].map, "s_orc.map");
        assert!(moria.specials[0].flags.iter().any(|f| f == "NO_GENO"));
        assert_eq!(moria.branch_at(40), Some(24));
        assert_eq!(gd.dungeon(24).branch_parent, Some((22, 40)));
        // E: effects and their x10 frequency.
        let doom = gd.dungeon(5);
        assert_eq!(
            (
                doom.effects[0].dd,
                doom.effects[0].ds,
                doom.effects[0].freq,
                doom.effects[0].typ.as_str()
            ),
            (2, 10, 10, "FIRE")
        );
        assert_eq!(doom.specials[0].map, "s_doom.map");
        assert!(doom.has("LAVA_RIVER"));
    }

    #[test]
    fn init1_store_records() {
        let gd = gd();
        assert_eq!(gd.stores.len(), 60);
        let gs = &gd.stores[0];
        assert_eq!((gs.id, gs.name.as_str(), gs.max_items), (0, "General Store", 24));
        assert_eq!(gs.glyph(), '1');
        assert_eq!(gs.owners, vec![0, 5, 6, 7]);
        assert_eq!(gs.actions, vec![1, 2, 3, 4]);
        assert_eq!(gs.entries.len(), 16);
        assert_eq!((gs.entries[0].proba, gs.entries[0].name.as_str()), (100, "Wooden Torch"));
        // T: rows carry the tval/sval directly (sval 256 = any).
        let magic = gd.stores.iter().find(|s| s.id == 5).unwrap();
        assert!(magic
            .entries
            .iter()
            .any(|e| e.tval == 65 && e.sval == 3 && e.proba == 100));
        assert!(magic
            .entries
            .iter()
            .any(|e| e.tval == 55 && e.sval == 15 && e.proba == 60));
        // grab_one_store_flag.
        let black = gd.stores.iter().find(|s| s.id == 6).unwrap();
        assert!(black.flags.iter().any(|f| f == "ALL_ITEM"));
        assert!(black.flags.iter().any(|f| f == "MEDIUM_LEVEL"));
    }

    #[test]
    fn init1_building_action_records() {
        let gd = gd();
        assert_eq!(gd.building_actions.len(), 47);
        let nothing = &gd.building_actions[0];
        assert_eq!((nothing.name.as_str(), nothing.action, nothing.letter.as_str()), ("Nothing", 0, "."));
        let sell = &gd.building_actions[1];
        assert_eq!(sell.name, "Sell an item");
        assert_eq!((sell.action, sell.restr), (43, 0));
        assert_eq!((sell.letter.as_str(), sell.letter_aux.as_str()), ("s", "d"));
        // C: costs are kept per owner mood.
        let expensive = gd
            .building_actions
            .iter()
            .find(|a| a.costs.0 > 0 && a.costs.1 > 0 && a.costs.2 > 0)
            .unwrap();
        assert!(expensive.costs.0 >= expensive.costs.1);
        assert!(expensive.costs.1 >= expensive.costs.2);
    }

    #[test]
    fn init1_owner_records() {
        let gd = gd();
        assert_eq!(gd.owners.len(), 70);
        let bilbo = &gd.owners[0];
        assert_eq!(bilbo.name, "Bilbo the Friendly(Hobbit)");
        assert_eq!((bilbo.max_cost, bilbo.inflation), (20000, 120));
        assert_eq!(bilbo.costs, (120, 100, 80));
        assert!(bilbo.liked.iter().any(|r| r == "Hobbit"));
        assert!(bilbo.hated.iter().any(|r| r == "Troll"));
        // grab_one_race_flag resolves both race and class names; every
        // shipped entry resolves to one of the p_info titles.
        for o in &gd.owners {
            for r in o.liked.iter().chain(o.hated.iter()) {
                assert!(
                    gd.races.iter().any(|x| x.name == *r)
                        || gd.classes.iter().any(|c| c.name == *r),
                    "{}: unknown race/class {}",
                    o.name,
                    r
                );
            }
        }
    }

    #[test]
    fn init1_wilderness_records() {
        let gd = gd();
        assert_eq!(gd.wf.len(), 27);
        let bree = gd.wf(1);
        assert_eq!(
            (
                bree.name.as_str(),
                bree.text.as_str(),
                bree.level,
                bree.entrance,
                bree.road,
                bree.feat,
                bree.terrain_idx
            ),
            ("Bree ", "a small village", 1, 1, 0, 203, 1)
        );
        assert_eq!(bree.ch, '1');
        assert_eq!(
            bree.terrain,
            vec![88, 88, 89, 89, 89, 89, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96, 96]
        );
        // W:<level>:<entrance> decoding.
        assert_eq!(bree.town(), 1);
        assert_eq!(bree.dungeon(), None);
        let nether = gd.wf.iter().find(|w| w.dungeon() == Some(6)).unwrap();
        assert_eq!(nether.entrance, 1006);
        assert_eq!(nether.town(), 0);
    }

    #[test]
    fn init1_process_dungeon_file_pref() {
        let gd = gd();
        // process_dungeon_file/aux: the Bree pref map resolves its
        // conditional F: overrides and P: starts (expr evaluation).
        let bree = gd.town(1, false).unwrap();
        assert_eq!(bree.rows.len(), 66);
        assert_eq!(bree.chars.len(), 45);
        assert_eq!(bree.default_start, Some((131, 33)));
        assert_eq!(bree.overrides.len(), 4);
        let house = bree.overrides.iter().find(|o| o.cond.contains("QUEST4")).unwrap();
        assert_eq!((house.ch, house.terrain, house.special), ('z', 74, 7));
        let glade = bree.overrides.iter().find(|o| o.cond.contains("QUEST8")).unwrap();
        assert_eq!((glade.ch, glade.terrain, glade.special), ('y', 8, 8));
        assert!(bree.overrides.iter().any(|o| o.cond.contains("DAYTIME")));
        // Fixed quest maps: F: placements, F:* random markers, M: mimics.
        let thieves = gd.quest_map(4).unwrap();
        assert_eq!((thieves.w, thieves.h), (33, 23));
        assert_eq!(thieves.cells.len(), (33 * 23) as usize);
        assert_eq!((thieves.start, thieves.exit, thieves.marker), ((4, 4), (23, 4), (4, 4)));
        assert_eq!(thieves.monsters.len(), 10);
        let mm = &thieves.monsters[0];
        assert_eq!((mm.def, mm.x, mm.y, mm.quest), (46, 5, 9, true));
        assert_eq!(thieves.objects.len(), 2);
        assert_eq!((thieves.objects[0].def, thieves.objects[0].x, thieves.objects[0].y), (395, 11, 1));
        let fireproof = gd.quest_map(27).unwrap();
        assert_eq!(fireproof.random_objects[0].level, 25);
        assert_eq!((fireproof.random_objects[0].x, fireproof.random_objects[0].y), (44, 3));
        let haunted = gd.quest_map(19).unwrap();
        assert_eq!((haunted.mimics[0].x, haunted.mimics[0].y, haunted.mimics[0].t), (2, 2, 61));
        // d_info special levels ('@:' maps).
        assert_eq!(gd.spec_levels.len(), 8);
        let gates = gd.spec_level("s_gates.map").unwrap();
        assert_eq!((gates.w, gates.h), (70, 57));
        assert_eq!(gates.cells.len(), (70 * 57) as usize);
        assert_eq!(gates.start, (37, 11));
        assert_eq!(
            (gates.artifacts[0].id, gates.artifacts[0].x, gates.artifacts[0].y),
            (10, 28, 42)
        );
        // The destroyed-Gondolin world row replacement (w_info W:M branch).
        assert_eq!(gd.world.patches.len(), 1);
        assert_eq!(gd.world.patches[0].cond, "town_destroy2");
    }
}
