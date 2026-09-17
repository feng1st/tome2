//! Items: object instances (plusses, ego/artifact, curses, charges, fuel),
//! floor stacks, gold, inventory/equipment, object generation and
//! potion/scroll/food effects.

use bevy::prelude::*;
use rand::distributions::{Distribution, WeightedIndex};
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::data::{self, Dice, GameData};
use crate::game::{GridPos, MessageLog, PlayerState, TurnState};
use crate::map::{self, Map};
use crate::render::{self, TileAssets};
use crate::AppState;

/// One concrete object: a base kind plus instance state (identification,
/// ego/artifact, curses, charges, fuel, stack count).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Index into GameData::objects.
    pub def: usize,
    /// e_info id (0 = none).
    pub ego: u32,
    /// Second ego (object_type.name2b; the 7% double-ego roll).
    #[serde(default)]
    pub ego2: u32,
    /// a_info id (0 = none).
    pub artifact: u32,
    /// Final combat stats (base + ego/artifact modifiers).
    pub dice: String,
    pub ac: i32,
    pub to_h: i32,
    pub to_d: i32,
    pub to_a: i32,
    /// Instance weight (object_type.weight; a_info artifacts may override
    /// the base kind and decayed corpses re-roll it). 0 = base kind.
    #[serde(default)]
    pub weight: i32,
    /// Stack size (ammunition; 1 for everything else).
    pub count: u32,
    /// Wand/staff charges (-1 = not a charged device).
    pub charges: i32,
    /// Light fuel remaining (lites) / recharge timer (rods).
    pub fuel: i32,
    /// Stat/speed bonus from ego/artifact (0 for plain items).
    pub pval: i32,
    /// Secondary value (symbiote current hp; object pval2).
    #[serde(default)]
    pub pval2: i32,
    /// Tertiary value (symbiote max hp; object pval3).
    #[serde(default)]
    pub pval3: i32,
    /// Artifact activation recharge countdown (0 = ready).
    pub timeout: i32,
    pub cursed: bool,
    /// Fully identified (bonuses and ego/artifact known).
    pub identified: bool,
    /// The wielder felt the curse.
    pub known_cursed: bool,
    /// Fireproofed by the Old Mage (fireproof quest reward).
    #[serde(default)]
    pub fireproof: bool,
    /// Sub-id for special items: the monster kind of a corpse, egg or
    /// symbiotic monster.
    #[serde(default)]
    pub note: u32,
    /// Stored experience of a symbiotic monster (object exp field).
    #[serde(default)]
    pub mon_exp: u32,
    /// Stored monster level of a symbiotic monster (object elevel).
    #[serde(default)]
    pub mon_level: u32,
    /// Per-instance flags (random_resistance rolls on Lordly rings and
    /// Amulets of Resistance/the Magi; randart powers later).
    #[serde(default)]
    pub flags: Vec<String>,
    /// Custom spell list: the Library quest's "Tome of <player>" teaches
    /// exactly these spells (q_library.cc book slots).
    #[serde(default)]
    pub spells: Vec<String>,
    /// Randart proper name (create_artifact; "" for a_info artifacts).
    #[serde(default)]
    pub artifact_name: String,
    /// Conjured item (TR_TEMPORARY): vanishes when `timeout` runs out
    /// (cmd7.cc Necromantic Teeth).
    #[serde(default)]
    pub temporary: bool,
    /// Player-written inscription (cmd3.cc do_cmd_inscribe). Shown after
    /// the label in braces; part of the stacking key.
    #[serde(default)]
    pub inscription: String,
    /// Shop discount percentage (object2.cc object_value).
    #[serde(default)]
    pub discount: i32,
    /// Game-unique value of a legacy TV_RANDART "junkart" (init_randart
    /// rolls randnor(0, 250); object2.cc object_value_real).
    #[serde(default)]
    pub junk_cost: i32,
    /// Object level of a LEVELS artifact (object1.cc object_gain_level).
    #[serde(default = "default_elevel")]
    pub elevel: i32,
    /// Randomly-rolled device spell of a "Spell" wand/staff
    /// (spells5.cc get_random_stick; "" = use the kind's SPELL= name).
    #[serde(default)]
    pub stick_spell: String,
    /// How the object was found (object_type.found; OBJ_FOUND_*).
    #[serde(default)]
    pub found: u8,
    /// Where-found bookkeeping (object_type.found_aux1..4); meaning depends
    /// on `found` (monster r_idx/ego/dungeon/level, store index, ...).
    #[serde(default)]
    pub found_aux1: i32,
    #[serde(default)]
    pub found_aux2: i32,
    #[serde(default)]
    pub found_aux3: i32,
    #[serde(default)]
    pub found_aux4: i32,
}

/// object_type.found values (defines.hpp:283-291).
pub const OBJ_FOUND_MONSTER: u8 = 1;
pub const OBJ_FOUND_FLOOR: u8 = 2;
pub const OBJ_FOUND_VAULT: u8 = 3;
pub const OBJ_FOUND_SPECIAL: u8 = 4;
pub const OBJ_FOUND_RUBBLE: u8 = 5;
pub const OBJ_FOUND_REWARD: u8 = 6;
pub const OBJ_FOUND_STORE: u8 = 7;
pub const OBJ_FOUND_STOLEN: u8 = 8;
pub const OBJ_FOUND_SELFMADE: u8 = 9;

pub fn default_elevel() -> i32 {
    1
}

impl Item {
    /// A plain instance of a base object kind.
    pub fn base(gd: &GameData, def: usize) -> Item {
        let o = &gd.objects[def];
        let fuel = if o.tval == data::TV_LITE { o.fuel } else { 0 };
        // Base pval matters for gear whose power is in the base flags
        // (digging tools' TUNNEL, instruments).
        let pval = if matches!(o.tval, data::TV_DIGGING | data::TV_INSTRUMENT) {
            o.pval
        } else {
            0
        };
        // Rod mains carry a mana capacity in pval2 and start empty of a
        // tip and fully charged (object2.cc object_prep TV_ROD_MAIN);
        // rod tips carry their mana cost in pval2.
        let (pval, pval2, timeout) = if o.tval == data::TV_ROD_MAIN {
            (0, o.sval.max(1), o.sval.max(1))
        } else if o.tval == data::TV_ROD {
            (0, o.pval, 0)
        } else if o.flags.iter().any(|f| f == "LEVELS") {
            // object_prep: sentient objects start with one realm point.
            (pval, 1, 0)
        } else {
            (pval, 0, 0)
        };
        Item {
            def,
            ego: 0,
            ego2: 0,
            artifact: 0,
            dice: o.dice.clone(),
            ac: o.ac,
            to_h: o.to_h,
            to_d: o.to_d,
            to_a: o.to_a,
            weight: o.weight,
            count: 1,
            charges: -1,
            fuel,
            pval,
            pval2,
            pval3: 0,
            timeout,
            // object_prep ORs the base kind F:CURSED into art_flags.
            cursed: o.flags.iter().any(|f| f == "CURSED"),
            // Gear hides its plusses; flavoured kinds (potions, scrolls,
            // devices, jewelry, mushrooms) are unknown until the type is
            // identified; EASY_KNOW kinds are known on sight (object1.cc
            // object_easy_know + object_known_p).
            identified: if needs_identify(o.tval) {
                o.flags.iter().any(|f| f == "EASY_KNOW")
            } else {
                !gd.flavor_pos.contains_key(&def) || o.flags.iter().any(|f| f == "EASY_KNOW")
            },
            known_cursed: false,
            fireproof: false,
            note: 0,
            mon_exp: 0,
            mon_level: 0,
            flags: Vec::new(),
            spells: Vec::new(),
            artifact_name: String::new(),
            temporary: false,
            inscription: String::new(),
            discount: 0,
            junk_cost: 0,
            // Sentient objects start at k_ptr->level/10 + 1 (object_prep).
            elevel: if o.flags.iter().any(|f| f == "LEVELS") {
                o.depth as i32 / 10 + 1
            } else {
                1
            },
            stick_spell: String::new(),
            found: 0,
            found_aux1: 0,
            found_aux2: 0,
            found_aux3: 0,
            found_aux4: 0,
        }
    }

    /// object_value (object2.cc:1300): real value less the discount.
    pub fn cost(&self, gd: &GameData) -> i32 {
        object_value(gd, self)
    }

    /// One-line description. `known` holds the def indices whose *type* is
    /// identified (potions/scrolls/devices); gear uses `self.identified`.
    pub fn label(&self, gd: &GameData, known: &HashSet<usize>) -> String {
        let o = &gd.objects[self.def];
        // A symbiotic monster shows its race and remaining life
        // (object1.cc TV_HYPNOS description).
        if o.tval == data::TV_HYPNOS {
            let mname = gd
                .monsters
                .get(self.note as usize)
                .map(|m| m.name.as_str())
                .unwrap_or("unknown");
            return format!("{} ({} hp)", mname, self.pval2.max(0));
        }
        // A totem is named after the monster it holds (object1.cc
        // TV_TOTEM: "& #~ of <monster>").
        if o.tval == data::TV_TOTEM {
            let mname = gd
                .monsters
                .get(self.note as usize)
                .map(|m| m.name.as_str())
                .unwrap_or("unknown");
            return format!("{} of {}", o.name, mname);
        }
        // Potions/scrolls/devices: type identification.
        if needs_type_id(o.tval) && !self.identified && !known.contains(&self.def) {
            let kind = match o.tval {
                data::TV_POTION | data::TV_POTION2 => "Potion",
                data::TV_SCROLL => "Scroll",
                data::TV_WAND => "Wand",
                data::TV_STAFF => "Staff",
                data::TV_RING => "Ring",
                data::TV_AMULET => "Amulet",
                _ => "Rod",
            };
            return if self.count > 1 {
                format!("{} {}s", self.count, kind)
            } else {
                kind.to_string()
            };
        }
        let mut s = o.name.clone();
        // corpse/egg names carry the monster (object1.cc:1005/1021).
        if (o.tval == data::TV_CORPSE || o.tval == data::TV_EGG) && self.note != 0 {
            if let Some(m) = gd.monsters.get(self.note as usize) {
                let base = if o.tval == data::TV_EGG { "egg" } else { "corpse" };
                // Only unique *corpses* use the possessive (object1.cc
                // TV_CORPSE; eggs are plain "<monster> egg").
                s = if m.unique && o.tval == data::TV_CORPSE {
                    format!("{}'s {}", m.name, base)
                } else {
                    format!("{} {}", m.name, base)
                };
            }
        }
        // A legacy random artifact shows its generated full name.
        if o.tval == data::TV_RANDART && !self.artifact_name.is_empty() {
            s = self.artifact_name.clone();
        } else if o.tval == data::TV_ROD_MAIN {
            let base = s.replace(" of#", "");
            s = if self.pval != 0 {
                match gd.object_by_tval_sval(data::TV_ROD, self.pval) {
                    Some(d) if self.identified => {
                        format!("{} of {}", base, gd.objects[d].name)
                    }
                    _ => base,
                }
            } else {
                base
            };
        } else if s.contains('#') {
            // "Tome of#" -> "Tome of <player>" (object1.cc modstr), or the
            // mimic shape of Morphic Oil / a Cloak of Mimicry, or the
            // single spell of a random spellbook (sval 255).
            let sub = if o.tval == data::TV_POTION2 && o.sval == 1 && self.identified {
                crate::mimic::form_name(self.pval2.max(0) as u32).to_string()
            } else if o.tval == data::TV_CLOAK && o.sval == 100 {
                crate::mimic::form_obj_name(self.pval2.max(0) as u32).to_string()
            } else if o.tval == data::TV_BOOK && o.sval == 255 {
                self.spells
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "a random spell".to_string())
            } else if self.identified {
                self.artifact_name.clone()
            } else {
                String::new()
            };
            s = s.replace('#', &sub);
        }
        if self.artifact != 0 && self.identified {
            if let Some(a) = gd.artifacts.iter().find(|a| a.id == self.artifact) {
                s = format!("The {} {}", o.name, a.name);
            }
        } else if (self.ego != 0 || self.ego2 != 0) && self.identified {
            // Double egos combine prefix and suffix (object1.cc object_desc
            // name2b handling): "Elven Plate Mail of Resistance".
            let mut base = s.clone();
            for id in [self.ego2, self.ego] {
                if id == 0 {
                    continue;
                }
                if let Some(e) = gd.egos.iter().find(|e| e.id == id) {
                    base = if e.prefix {
                        format!("{} {}", e.name, base)
                    } else {
                        format!("{} {}", base, e.name)
                    };
                }
            }
            s = base;
        } else if !self.artifact_name.is_empty()
            && self.identified
            && !o.name.contains('#')
            && o.tval != data::TV_RANDART
        {
            // Randarts append their generated name (object1.cc).
            s = format!("{} {}", o.name, self.artifact_name);
        }
        if self.count > 1 {
            // The original pluralizer adds "es" after s/h (object1.cc:1288).
            let plural = if s.ends_with('s') || s.ends_with('h') {
                "es"
            } else {
                "s"
            };
            s = format!("{} {}{}", self.count, s, plural);
        }
        // Combat stats: shown for identified gear (lites have no dice).
        let flags = item_flags(gd, self);
        let has = |f: &str| flags.iter().any(|x| *x == f);
        let weapon = data::slot_of_item(o) == Some(data::SLOT_WEAPON);
        let gear = (o.wearable() && o.tval != data::TV_LITE) || data::is_ammo(o.tval);
        if gear && self.identified {
            if (weapon || data::is_ammo(o.tval)) && self.dice != "0d0" {
                s.push_str(&format!(" ({})", self.dice));
            } else if self.ac > 0 {
                s.push_str(&format!(" [{}]", self.ac));
            }
            // SHOW_MODS forces the (+h,+d) display even at (+0,+0).
            if self.to_h != 0 || self.to_d != 0 || has("SHOW_MODS") {
                s.push_str(&format!(" ({:+},{:+})", self.to_h, self.to_d));
            }
            if self.to_a != 0 {
                s.push_str(&format!(" [{:+}]", self.to_a));
            }
        }
        // Exploding missiles (object1.cc:1448).
        if data::is_ammo(o.tval) && self.pval2 != 0 {
            s.push_str(" (exploding)");
        }
        // A spell container names the spell it holds (object1.cc:1404).
        if self.identified && has("SPELL_CONTAIN") && self.pval2 != -1 {
            let spell = self
                .spells
                .first()
                .cloned()
                .unwrap_or_else(|| "a random spell".to_string());
            s.push_str(&format!(" [{}]", spell));
        }
        // MANA/LIFE show the stored pval as a percentage (object1.cc:1541).
        if self.identified && self.pval > 0 && (has("MANA") || has("LIFE")) {
            s.push_str(&format!(" ({}%)", 100 * self.pval / 5));
        }
        if self.identified && matches!(o.tval, data::TV_WAND | data::TV_STAFF) {
            s.push_str(&format!(
                " ({} charge{})",
                self.charges.max(0),
                if self.charges != 1 { "s" } else { "" }
            ));
        } else if self.identified && o.tval == data::TV_ROD_MAIN {
            // Rod mains show stored/capacity mana (object1.cc:1570).
            s.push_str(&format!(" ({}/{})", self.timeout, self.pval2.max(0)));
        } else if self.identified && o.tval == data::TV_ROD {
            // Rod tips show their activation cost (object1.cc:1580).
            s.push_str(&format!(" ({} Mana to cast)", self.pval2));
        } else if o.tval == data::TV_LITE && has("FUEL_LITE") {
            s.push_str(&format!(" (with {} turns of light)", self.fuel.max(0)));
        }
        // pval prose (object1.cc:1591-1643).
        if self.identified && self.pval != 0 && o.tval != data::TV_LITE {
            s.push_str(&format!(" ({:+}", self.pval));
            if !has("HIDE_TYPE") {
                if has("SPEED") {
                    s.push_str(" to speed");
                } else if has("BLOWS") {
                    s.push_str(" attack");
                    if self.pval.abs() != 1 {
                        s.push('s');
                    }
                } else if has("CRIT") {
                    s.push_str("% of critical hits");
                } else if has("STEALTH") {
                    s.push_str(" to stealth");
                } else if has("INFRA") {
                    s.push_str(" to infravision");
                }
            }
            s.push(')');
        }
        // Charging activatable items (object1.cc:1647).
        if self.identified && has("ACTIVATE") && self.timeout != 0 {
            if o.tval == data::TV_EGG {
                s.push_str(" (stopped)");
            } else {
                s.push_str(" (charging)");
            }
        }
        if self.elevel > 1 && self.identified {
            // LEVELS artifacts show the exp still needed and the level
            // (object1.cc:1423).
            let need = if self.elevel < 50 {
                let idx = (self.elevel as usize).clamp(1, 50) - 1;
                let need = crate::game::PLAYER_EXP[idx] * 5 / 2;
                need.saturating_sub(self.mon_exp as u64).to_string()
            } else {
                "*****".to_string()
            };
            s.push_str(&format!(" (E:{}, L:{})", need, self.elevel));
        }
        // The mode-3 inscription assembly (object1.cc:1668-1710).
        let mut inscrip: Vec<String> = Vec::new();
        if self.known_cursed && self.cursed {
            inscrip.push("cursed".to_string());
        }
        if let Some(pos) = self.inscription.find(['%', '#']) {
            inscrip.push(self.inscription[..pos].to_string());
        }
        if self.discount != 0 && self.inscription.is_empty() {
            inscrip.push(format!("{}% off", self.discount));
        }
        if !self.inscription.is_empty() {
            inscrip.push(self.inscription.clone());
        }
        if !inscrip.is_empty() {
            let mut cut = 75.min(s.len());
            while cut > 0 && !s.is_char_boundary(cut) {
                cut -= 1;
            }
            s.truncate(cut);
            s.push_str(&format!(" {{{}}}", inscrip.join(", ")));
        }
        s
    }
}

/// object_type.weight: the instance weight when set, else the base kind's
/// (a_info artifacts copy `a.weight`, decayed corpses re-roll it).
pub fn item_weight(gd: &GameData, item: &Item) -> i32 {
    if item.weight != 0 || gd.objects[item.def].weight == 0 {
        item.weight
    } else {
        gd.objects[item.def].weight
    }
}

/// The base/ego/artifact/instance flags of an item (k_info F:, e_info/a_info
/// F: lines, per-instance random_resistance rolls). For real artifacts the
/// a_info flags *replace* the base kind flags (object1.cc object_flags).
pub fn item_flags<'a>(gd: &'a GameData, item: &'a Item) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    if let Some(a) = gd.artifacts.iter().find(|a| a.id == item.artifact) {
        out.extend(a.flags.iter().map(|f| f.as_str()));
    } else {
        out.extend(gd.objects[item.def].flags.iter().map(|f| f.as_str()));
    }
    out.extend(item.flags.iter().map(|f| f.as_str()));
    if let Some(e) = gd.egos.iter().find(|e| e.id == item.ego) {
        out.extend(e.flags.iter().map(|f| f.as_str()));
    }
    if let Some(e) = gd.egos.iter().find(|e| e.id == item.ego2) {
        out.extend(e.flags.iter().map(|f| f.as_str()));
    }
    out
}

/// Stat deltas granted by an item's ego/artifact flags (STR..CHA).
pub fn stat_deltas(gd: &GameData, item: &Item) -> [i32; 6] {
    let mut d = [0; 6];
    for f in item_flags(gd, item) {
        let idx = match f {
            "STR" => Some(0),
            "INT" => Some(1),
            "WIS" => Some(2),
            "DEX" => Some(3),
            "CON" => Some(4),
            "CHR" => Some(5),
            _ => None,
        };
        if let Some(i) = idx {
            d[i] += item.pval;
        }
    }
    d
}

/// object1.cc artifact_p: a real artifact, a NORM_ART base kind or a
/// legacy random artifact.
pub fn is_artifact(gd: &GameData, item: &Item) -> bool {
    if item.artifact != 0 || !item.artifact_name.is_empty() {
        return true;
    }
    let o = &gd.objects[item.def];
    o.tval == data::TV_RANDART || o.flags.iter().any(|f| f == "NORM_ART")
}

/// `tval_descs` (tables.cc:3237): the category blurb printed by
/// `grab_tval_desc` in the object-info screen (object1.cc:1793).
pub fn tval_desc(tval: i32) -> Option<&'static str> {
    Some(match tval {
        data::TV_MSTAFF => "Mage Staves are the spellcaster's weapons of choice.  They all reduce spellcasting time to 80% of normal time and some will yield even greater powers.",
        data::TV_PARCHMENT => "Parchments can contain useful information ... or useless junk.",
        data::TV_EGG => "Eggs are laid by some monsters.  If they hatch in your inventory the monster will be your friend.",
        data::TV_TOOL => "Tools can be digging implements, climbing equipment and such. They have their own slot in your inventory.",
        data::TV_INSTRUMENT => "Musical instruments can be used with the Music skill to play magical songs. Some of them can also be activated.",
        data::TV_BOOMERANG => "Boomerangs can be used instead of bows or slings.  They are more like melee weapons than bows.",
        data::TV_SHOT => "Shots are small, hard balls.  They are the standard ammunition for slings.  You can carry them in your quiver if you have a sling equipped.",
        data::TV_ARROW => "Arrows are the standard ammunition for bows.  You can carry them in your quiver if you have a bow equipped.",
        data::TV_BOLT => "Bolts are the standard ammunition for crossbows.  You can carry them in your quiver if you have a crossbow equipped.",
        data::TV_BOW => "Slings, bows and crossbows are used to attack monsters from a distance.",
        data::TV_DIGGING => "Tools can be digging implements, climbing equipment and such.  They have their own slot in your inventory.",
        data::TV_HAFTED => "Hafted weapons are melee weapons.  Eru followers can use them without penalties.",
        data::TV_SWORD => "Swords are melee weapons.",
        data::TV_AXE => "Axes are melee weapons.",
        data::TV_POLEARM => "Polearms are melee weapons.",
        data::TV_DRAG_ARMOR => "Dragon armour is made from the scales of dead dragons. These mighty sets of armour usually yield great power to their wearer.",
        data::TV_LITE => "Lights allow you to read things and see from afar. Some of them need to be fueled but some do not.",
        data::TV_AMULET => "Amulets are fine pieces of jewelry, usually imbued with arcane magics.",
        data::TV_RING => "Rings are fine pieces of jewelry, usually imbued with arcane magics.",
        data::TV_STAFF => "Staves are objects imbued with mystical powers.",
        data::TV_WAND => "Wands are like small staves and usually have a targeted effect.",
        data::TV_ROD => "Rod tips are the physical bindings of powerful spells.  Zap (attach) them to a rod to get a fully functional rod. Each spell takes some mana from the rod it is attached to to work.",
        data::TV_ROD_MAIN => "Rods contain mana reserves used to cast spells in rod tips.  Zap (attach) a rod tip to them to get a fully functional rod. Each spell takes some mana from the rod it is attached to to work.",
        data::TV_SCROLL => "Scrolls are magical parchments imbued with magic spells. Some are good, some...are not.  When a scroll is read, its magic is released and the scroll is destroyed.",
        data::TV_POTION => "Potions are magical liquids.  Some of them are beneficial...some not.",
        data::TV_POTION2 => "Potions are magical liquids.  Some of them are beneficial...some not.",
        data::TV_FLASK => "Flasks of oil can be used to refill lanterns.",
        data::TV_FOOD => "Everybody needs to eat, even you.",
        data::TV_HYPNOS => "This monster seems to be hypnotised and friendly.",
        data::TV_RANDART => "Those objects are only known of by rumours.  It is said that they can be activated for great or strange effects...",
        data::TV_JUNK => "Junk is usually worthless, though experienced archers can create ammo with them.",
        data::TV_SKELETON => "It looks dead...",
        data::TV_BOTTLE => "An empty bottle.",
        data::TV_SPIKE => "Spikes can be used to jam doors.",
        data::TV_CORPSE => "It looks dead...",
        data::TV_BOOTS => "Boots can help your armour rating.  Some of these are magical.",
        data::TV_GLOVES => "Handgear is used to protect hands, but nonmagical ones can sometimes hinder spellcasting.",
        data::TV_HELM => "Headgear will protect your head.",
        data::TV_CROWN => "Headgear will protect your head.",
        data::TV_SHIELD => "Shields will help improve your defence rating, but you cannot use them with two handed weapons.",
        data::TV_CLOAK => "Cloaks can shield you from damage.  Sometimes they also provide magical powers.",
        data::TV_SOFT_ARMOR => "Soft armour is light, and will not hinder your combat much.",
        data::TV_HARD_ARMOR => "Hard armour provides much more protection than soft armour but also hinders combat much more.",
        data::TV_SYMBIOTIC_BOOK => "This mystical book is used by symbiants to extend their symbiosis.",
        data::TV_MUSIC_BOOK => "This song book is used by bards to play songs.",
        data::TV_DRUID_BOOK => "This mystical book is used by druids to call upon the powers of nature.",
        data::TV_DAEMON_BOOK => "This unholy demon equipment is used with the Demonology skill to control the school of demon power.",
        _ => return None,
    })
}

/// `item_tester_hook_sacrifice_aule` (cmd2.cc:3720): only self-made items
/// may be offered to Aule.
pub fn item_tester_hook_sacrifice_aule(it: &Item) -> bool {
    it.found == OBJ_FOUND_SELFMADE
}

/// `label_to_inven` (object1.cc:3076): a pack label to its index.
pub fn label_to_inven(c: char, pack_len: usize) -> Option<usize> {
    if !c.is_ascii_lowercase() {
        return None;
    }
    let i = (c as u8 - b'a') as usize;
    if i >= pack_len || i > 25 {
        None
    } else {
        Some(i)
    }
}

/// `label_to_equip` (object1.cc:3098): an equipment label to its slot.
/// Equipment labels start at 'a' (the original maps them onto the
/// INVEN_WIELD.. slots independently of the pack).
pub fn label_to_equip(c: char, equip_len: usize) -> Option<usize> {
    if !(c.is_ascii_lowercase() || c > 'z') {
        return None;
    }
    let i = (c as u8).wrapping_sub(b'a') as usize;
    if i >= equip_len {
        None
    } else {
        Some(i)
    }
}

/// `grab_tval_desc` (object1.cc:1793): the category blurb for the object
/// info screen (same table as `tval_desc`).
pub fn grab_tval_desc(tval: i32) -> Option<&'static str> {
    tval_desc(tval)
}

/// `index_to_label` (object1.cc:3062): a combined pack/equipment index to
/// its display letter. Pack indexes are 0..23; equipment labels repeat
/// from 'a' (INVEN_WIELD = 23 in the original).
pub fn index_to_label(i: i32) -> char {
    let j = if i < 23 { i } else { i - 23 };
    (b'a' as i32 + j) as u8 as char
}

/// `ego_item_p` (object1.cc:5877).
pub fn ego_item_p(item: &Item) -> bool {
    item.ego != 0 || item.ego2 != 0
}

/// `is_ego_p` (object1.cc:5885).
pub fn is_ego_p(item: &Item, ego: u32) -> bool {
    item.ego == ego || item.ego2 == ego
}

/// `object_flags_known` (object1.cc:625): the flags visible on an
/// identified item. Unknown items show nothing; real artifacts replace the
/// base kind flags; RES_CHAOS implies RES_CONF (the original "Hack").
pub fn item_flags_known<'a>(gd: &'a GameData, item: &'a Item) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    if !item.identified {
        return out;
    }
    if let Some(a) = gd.artifacts.iter().find(|a| a.id == item.artifact) {
        out.extend(a.flags.iter().map(|f| f.as_str()));
    } else {
        out.extend(gd.objects[item.def].flags.iter().map(|f| f.as_str()));
    }
    out.extend(item.flags.iter().map(|f| f.as_str()));
    if let Some(e) = gd.egos.iter().find(|e| e.id == item.ego) {
        out.extend(e.flags.iter().map(|f| f.as_str()));
    }
    if let Some(e) = gd.egos.iter().find(|e| e.id == item.ego2) {
        out.extend(e.flags.iter().map(|f| f.as_str()));
    }
    if out.contains(&"RES_CHAOS") {
        out.push("RES_CONF");
    }
    out
}

/// `get_item_letter_color` (object1.cc:3595): the terminal colour of the
/// inventory label.
pub fn get_item_letter_color(gd: &GameData, item: &Item, known: &HashSet<usize>) -> i64 {
    use crate::base_defs::{
        TERM_GREEN, TERM_L_BLUE, TERM_SLATE, TERM_VIOLET, TERM_WHITE, TERM_YELLOW,
    };
    if !(item.identified || known.contains(&item.def)) {
        return TERM_SLATE;
    }
    let mut color = TERM_WHITE;
    if ego_item_p(item) {
        color = TERM_L_BLUE;
    }
    if is_artifact(gd, item) {
        color = TERM_YELLOW;
    }
    if item.artifact != 0 {
        if gd.set_of_artifact(item.artifact).is_some() {
            color = TERM_GREEN;
        }
        if item_flags(gd, item).iter().any(|f| *f == "ULTIMATE") {
            color = TERM_VIOLET;
        }
    }
    color
}

/// `item_tester_okay` (object1.cc:3544): list eligibility for one slot.
/// `full` lists empty slots; gold is never listed.
pub fn item_tester_okay(
    gd: &GameData,
    full: bool,
    item: Option<&Item>,
    filter: impl Fn(&Item) -> bool,
) -> bool {
    if full {
        return true;
    }
    let Some(it) = item else { return false };
    if gd.objects[it.def].tval == data::TV_GOLD {
        return false;
    }
    filter(it)
}

/// `get_item_okay` (object1.cc:4144): `item_tester_okay` for a combined
/// pack/equipment index.
pub fn get_item_okay(
    gd: &GameData,
    i: usize,
    pack: &[Item],
    equip: &[Option<Item>],
    filter: impl Fn(&Item) -> bool,
) -> bool {
    if i >= pack.len() + equip.len() {
        return false;
    }
    let item = if i < pack.len() {
        Some(&pack[i])
    } else {
        equip.get(i - pack.len()).and_then(|s| s.as_ref())
    };
    item_tester_okay(gd, false, item, filter)
}

/// `get_tag` (object1.cc:4167): the first inventory index whose inscription
/// carries "@tag" (or "@<cmd>tag").
pub fn get_tag(pack: &[Item], equip: &[Option<Item>], cmd: char, tag: char) -> Option<usize> {
    let carries = |inscrip: &str| {
        let chars: Vec<char> = inscrip.chars().collect();
        (0..chars.len()).any(|k| {
            chars[k] == '@'
                && (chars.get(k + 1) == Some(&tag)
                    || (chars.get(k + 1) == Some(&cmd) && chars.get(k + 2) == Some(&tag)))
        })
    };
    for (i, it) in pack.iter().enumerate() {
        if carries(&it.inscription) {
            return Some(i);
        }
    }
    for (i, slot) in equip.iter().enumerate() {
        if slot.as_ref().is_some_and(|it| carries(&it.inscription)) {
            return Some(pack.len() + i);
        }
    }
    None
}

/// `scan_floor` (object1.cc:4227): the indexes of up to 23 items accepted
/// by the filter.
pub fn scan_floor(items: &[Item], filter: impl Fn(&Item) -> bool) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        if !filter(it) {
            continue;
        }
        out.push(i);
        if out.len() == 23 {
            break;
        }
    }
    out
}

/// `item_tester_hook_getable` (object1.cc:5089): the pack has room and the
/// player can handle a symbiotic monster.
pub fn item_tester_hook_getable(
    gd: &GameData,
    inv: &Inventory,
    item: &Item,
    symbiotic_skill: bool,
) -> bool {
    inven_carry_okay(gd, inv, item)
        && !(gd.objects[item.def].tval == data::TV_HYPNOS && !symbiotic_skill)
}

/// `get_item_allow` (object1.cc:4107): an inscription "!<cmd>" or "!*"
/// forces a confirmation; true when the choice must be confirmed.
pub fn inscription_prevents(inscription: &str, cmd: char) -> bool {
    let chars: Vec<char> = inscription.chars().collect();
    (0..chars.len()).any(|i| {
        chars[i] == '!'
            && match chars.get(i + 1) {
                Some('*') => true,
                Some(c) => *c == cmd,
                None => false,
            }
    })
}

/// `verify` (object1.cc:4080): the confirmation prompt text.
pub fn verify_prompt(prompt: &str, name: &str) -> String {
    format!("{} {}? ", prompt, name)
}

/// `check_first` (object1.cc:1810): the separator before a damage entry.
pub fn check_first(first: &mut bool) -> &'static str {
    if *first {
        *first = false;
        ""
    } else {
        ", "
    }
}

/// The "%d.%d" / "%d" damage rendering of output_dam/output_ammo_dam.
fn fmt_tenths(dam: i32) -> String {
    if dam > 0 {
        if dam % 10 != 0 {
            format!("{}.{}", dam / 10, dam % 10)
        } else {
            format!("{}", dam / 10)
        }
    } else {
        "0".to_string()
    }
}

/// `output_dam` (object1.cc:1824): one "N against X" melee damage entry of
/// the weapon comparison. `mult2 == 0` adds a single entry.
#[allow(clippy::too_many_arguments)]
pub fn output_dam(
    first: &mut bool,
    dd: i32,
    ds: i32,
    to_d: i32,
    p_to_d: i32,
    p_to_d_melee: i32,
    num_blow: i32,
    mult: i32,
    mult2: i32,
    against: &str,
    against2: &str,
) -> String {
    let roll = |mult: i32| {
        let mut dam = (dd + dd * ds) * 5 * mult;
        dam += (to_d + p_to_d + p_to_d_melee) * 10;
        dam * num_blow
    };
    let mut s = String::from(check_first(first));
    s.push_str(&fmt_tenths(roll(mult)));
    s.push_str(&format!(" against {}", against));
    if mult2 != 0 {
        s.push_str(check_first(first));
        s.push_str(&fmt_tenths(roll(mult2)));
        s.push_str(&format!(" against {}", against2));
    }
    s
}

/// `output_ammo_dam` (object1.cc:1915): the ranged damage entry. The
/// shooter's to_d and the player's to_d_ranged only apply to fired ammo,
/// not to thrown boomerangs.
#[allow(clippy::too_many_arguments)]
pub fn output_ammo_dam(
    first: &mut bool,
    boomerang: bool,
    dd: i32,
    ds: i32,
    to_d: i32,
    bow_to_d: i32,
    p_to_d_ranged: i32,
    tmul: i32,
    mult: i32,
    mult2: i32,
    against: &str,
    against2: &str,
) -> String {
    let roll = |m: i32| {
        let mut dam = (dd + dd * ds) * 5;
        dam += to_d * 10;
        if !boomerang {
            dam += bow_to_d * 10;
        }
        dam *= tmul;
        if !boomerang {
            dam += p_to_d_ranged * 10;
        }
        dam * m
    };
    let mut s = String::from(check_first(first));
    s.push_str(&fmt_tenths(roll(mult)));
    s.push_str(&format!(" against {}", against));
    if mult2 != 0 {
        s.push_str(check_first(first));
        s.push_str(&fmt_tenths(roll(mult2)));
        s.push_str(&format!(" against {}", against2));
    }
    s
}

/// `mention_use` (object1.cc:3341): how an equipment slot carries an item.
/// `weight` is the item's weight and `str_hold` adj_str_hold[stat_ind[A_STR]].
pub fn mention_use(slot: usize, tval: i32, weight: i32, str_hold: i32) -> &'static str {
    let mut p = match slot {
        data::SLOT_WEAPON | data::SLOT_WEAPON2 => "Wielding",
        data::SLOT_BOW => "Shooting",
        data::SLOT_RING1 | data::SLOT_RING2 => "On finger",
        data::SLOT_AMULET => "Around neck",
        data::SLOT_LITE => "Light source",
        data::SLOT_BODY => "On body",
        data::SLOT_CLOAK => "About body",
        data::SLOT_SHIELD | data::SLOT_SHIELD2 => "On arm",
        data::SLOT_HEAD => "On head",
        data::SLOT_HANDS | data::SLOT_HANDS2 => "On hands",
        data::SLOT_FEET | data::SLOT_FEET2 => "On feet",
        data::SLOT_SYMBIOTE => "Symbiote",
        data::SLOT_QUIVER => "Quiver",
        data::SLOT_TOOL => "Using",
        _ => "In pack",
    };
    if matches!(slot, data::SLOT_WEAPON | data::SLOT_WEAPON2) && weight / 10 > str_hold {
        p = "Just lifting";
    }
    if slot == data::SLOT_BOW {
        if tval == data::TV_INSTRUMENT {
            p = "Playing";
        } else if weight / 10 > str_hold {
            p = "Just holding";
        }
    }
    p
}

/// `describe_use` (object1.cc:3444): the prose form for worn gear.
pub fn describe_use(slot: usize, tval: i32, weight: i32, str_hold: i32) -> &'static str {
    let mut p = match slot {
        data::SLOT_WEAPON | data::SLOT_WEAPON2 => "attacking monsters with",
        data::SLOT_BOW => "shooting missiles with",
        data::SLOT_RING1 | data::SLOT_RING2 => "wearing on your finger",
        data::SLOT_AMULET => "wearing around your neck",
        data::SLOT_LITE => "using to light the way",
        data::SLOT_BODY => "wearing on your body",
        data::SLOT_CLOAK => "wearing on your back",
        data::SLOT_SHIELD | data::SLOT_SHIELD2 => "wearing on your arm",
        data::SLOT_HEAD => "wearing on your head",
        data::SLOT_HANDS | data::SLOT_HANDS2 => "wearing on your hands",
        data::SLOT_FEET | data::SLOT_FEET2 => "wearing on your feet",
        data::SLOT_SYMBIOTE => "in symbiosis with",
        data::SLOT_QUIVER => "carrying in your quiver",
        data::SLOT_TOOL => "using as a tool",
        _ => "carrying in your pack",
    };
    if matches!(slot, data::SLOT_WEAPON | data::SLOT_WEAPON2) && weight / 10 > str_hold {
        p = "just lifting";
    }
    if slot == data::SLOT_BOW {
        if tval == data::TV_INSTRUMENT {
            p = "playing music with";
        } else if weight / 10 > str_hold {
            p = "just holding";
        }
    }
    p
}

/// `toggle_inven_equip` (object1.cc:4039): flip the inventory/equipment
/// window flags. The Bevy build draws both in modal windows, so only the
/// pure flag swap is kept.
pub fn toggle_inven_equip(flags: i64) -> i64 {
    use crate::base_defs::{PW_EQUIP, PW_INVEN};
    if flags & PW_INVEN != 0 {
        (flags & !PW_INVEN) | PW_EQUIP
    } else if flags & PW_EQUIP != 0 {
        (flags & !PW_EQUIP) | PW_INVEN
    } else {
        flags
    }
}

/// `wearable_p` (loadsave.cc:1045): the tval switch deciding whether an
/// object can be taken into a slot (the port's `ObjectDef::wearable`).
pub fn wearable_p(gd: &GameData, item: &Item) -> bool {
    gd.objects[item.def].wearable()
}

/// `object_filter::IsArtifact` (object_filter.cc:27): a standard artifact
/// (`name1 > 0`), unlike `is_artifact`/`artifact_p` which also accept
/// randarts and NORM_ART kinds.  `bldg.cc:1016` refuses smith work on
/// identified standard artifacts, so the shop checks use this predicate.
pub fn is_standard_artifact(item: &Item) -> bool {
    item.artifact != 0
}

/// `object_mention` (object2.cc:1890): the wizard "describe created
/// object" line.
pub fn object_mention_text(gd: &GameData, item: &Item, known: &HashSet<usize>) -> String {
    let name = item.label(gd, known);
    if is_artifact(gd, item) {
        format!("Artifact ({})", name)
    } else if !item.artifact_name.is_empty() {
        "Random artifact".to_string()
    } else if ego_item_p(item) {
        format!("Ego-item ({})", name)
    } else {
        format!("Object ({})", name)
    }
}

/// Artifacts are indestructible; ego/artifact IGNORE_<element> flags
/// protect the item from that element's inventory damage; fireproofed
/// items ignore fire.
pub fn item_ignores(gd: &GameData, item: &Item, element: &str) -> bool {
    if is_artifact(gd, item) {
        return true;
    }
    if element == "FIRE" && item.fireproof {
        return true;
    }
    item_flags(gd, item)
        .iter()
        .any(|f| *f == format!("IGNORE_{}", element))
}

/// Gear that hides its plusses until identified.
fn needs_identify(tval: i32) -> bool {
    data::slot_of(tval).is_some()
}

/// Consumables identified as a type (potion/scroll/wand/staff/rod) plus
/// jewelry (rings/amulets keep their flavor hidden until known).
fn needs_type_id(tval: i32) -> bool {
    matches!(
        tval,
        data::TV_POTION
            | data::TV_POTION2
            | data::TV_SCROLL
            | data::TV_WAND
            | data::TV_STAFF
            | data::TV_ROD
            | data::TV_ROD_MAIN
            | data::TV_RING
            | data::TV_AMULET
    )
}

/// The player's pack, worn equipment and item lore.
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub pack: Vec<Item>,
    pub equip: [Option<Item>; data::NUM_SLOTS],
    /// Def indices whose type is identified (potions/scrolls/devices).
    pub known: HashSet<usize>,
}

impl Default for Inventory {
    fn default() -> Self {
        Inventory {
            pack: Vec::new(),
            equip: core::array::from_fn(|_| None),
            known: HashSet::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct EquipTotals {
    pub to_h: i32,
    pub to_d: i32,
    pub ac: i32,
    pub lite: i32,
    /// Stat bonuses from ego/artifact flags (added to base stats).
    pub stats: [i32; 6],
    /// Bonus speed (affects how often the player gets free actions).
    pub speed: i32,
    /// Element/fear resistances (RES_FIRE/RES_COLD/.../RES_FEAR), stored
    /// by their short GF-style name ("FIRE", "POIS", "CONF", ...).
    pub resists: HashSet<String>,
    /// Paralysis immunity.
    pub free_act: bool,
    /// Makes monsters slower to notice the player.
    pub stealth: i32,
    /// Melee/ranged damage flags: SLAY_*, KILL_*, BRAND_*, WOUNDING,
    /// VORPAL, CHAOTIC, VAMPIRIC, IMPACT, BLESSED.
    pub slays: HashSet<String>,
    /// Sustained stats ("STR".."CHR" from SUST_*).
    pub sustains: HashSet<String>,
    /// Immunities ("ACID"/"ELEC"/"FIRE"/"COLD"/"NETHER").
    pub immunities: HashSet<String>,
    /// Takes double fire damage (SENS_FIRE).
    pub sens_fire: bool,
    /// Telepathy: race names from ESP_* ("ALL", "EVIL", "ORC", ...).
    pub esp: HashSet<String>,
    /// Sees invisible monsters.
    pub see_invis: bool,
    /// The player is invisible (monsters only notice up close).
    pub invis: bool,
    /// Invisibility power (TR_INVIS pval * 10, xtra1.cc:2514); a
    /// positive value grants `invis`, negative pvals offset it.
    pub invis_power: i32,
    /// Infravision radius (warm-blooded monsters show without LOS).
    pub infra: i32,
    /// Reflects bolts back at the caster.
    pub reflect: bool,
    /// Resists experience drain (HOLD_LIFE).
    pub hold_life: bool,
    /// Feather fall / flight / climbing (trap and gravity protection).
    pub feather: bool,
    pub fly: bool,
    pub climb: bool,
    /// Faster regeneration.
    pub regen: bool,
    /// Food drains five times slower.
    pub slow_digest: bool,
    /// Monsters always know where you are.
    pub aggravate: bool,
    /// Fiery / shocking melee aura (hurts attackers).
    pub sh_fire: bool,
    pub sh_elec: bool,
    /// Digging speed bonus (TUNNEL pval * 20, xtra1.cc apply_flags).
    pub tunnel: i32,
    /// Extra melee blows (BLOWS pval).
    pub blows: i32,
    /// +1 bow multiplier / +1 shot per round.
    pub xtra_might: bool,
    pub xtra_shots: bool,
    /// Percent modifiers to max mana / max hp (MANA/LIFE pval).
    pub mana_pct: i32,
    pub life_pct: i32,
    /// Spell power bonus (SPELL pval, scales spell damage).
    pub spell_power: i32,
    /// Cannot teleport / cannot cast spells (cursed flags).
    pub no_tele: bool,
    pub no_magic: bool,
    /// 50% antimagic field (disrupts nearby monster casting).
    pub antimagic50: bool,
    /// Cursed item drains experience over time.
    pub drain_exp: bool,
    /// Cursed item teleports the wearer randomly.
    pub teleport_itis: bool,
    /// Cursed item occasionally withers a stat (Black Breath).
    pub black_breath: bool,
    /// Ancient foul curse: aggravates and drains (TY_CURSE/DG_CURSE).
    pub ty_curse: bool,
    /// Luck bonus (LUCK pval): better hit and crit rolls.
    pub luck: i32,
    /// Wraithform: walk through (non-permanent) walls.
    pub wraith: bool,
    /// Casting is more reliable (FAST_CAST).
    pub fast_cast: bool,
    /// Weapon cannot make melee attacks (NEVER_BLOW).
    pub never_blow: bool,
    /// Level-entry senses (PRECOGNITION).
    pub precognition: bool,
    /// Needs no air (MAGIC_BREATH; also grants water breathing).
    pub magic_breath: bool,
    /// Breathes water (WATER_BREATH).
    pub water_breath: bool,
    /// Critical-hit chance bonus in percent (CRIT pval, xtra1.cc).
    pub crit: i32,
    /// Cursed gear draining life/mana every turn (DRAIN_HP/DRAIN_MANA).
    pub drain_hp: i32,
    pub drain_mana: i32,
    /// Cannot move normally (IMMOVABLE; the special action menu is used).
    pub immovable: bool,
}

/// p_ptr->free_act after calc_bonuses: TR_FREE_ACT, Manwe's prayer
/// (grace >= 7000) and the unencumbered barehand monk past skill 24
/// (xtra1.cc:2294/2533/2888).  Consumers must use this, not the equipment
/// totals alone.
pub fn has_free_act(
    ps: &PlayerState,
    inv: &Inventory,
    gd: &GameData,
    totals: &EquipTotals,
) -> bool {
    totals.free_act || crate::game::god_free_act(ps) || crate::game::monk_free_act(ps, inv, gd)
}

/// A slay/brand/damage flag (shared by equipment totals and weapon slays).
/// TR_BLESSED is not a damage flag here: it only marks the blade for
/// `forbid_non_blessed` (xtra1.cc:2525 bless_blade), so it is excluded.
pub fn is_slay_flag(f: &str) -> bool {
    f.starts_with("SLAY_")
        || f.starts_with("KILL_")
        || f.starts_with("BRAND_")
        || matches!(f, "WOUNDING" | "VORPAL" | "CHAOTIC" | "VAMPIRIC" | "IMPACT")
}

/// Fold one equipment flag into the totals (original apply_flags).
fn apply_equip_flag(t: &mut EquipTotals, f: &str, pval: i32) {
    match f {
        "STR" => t.stats[0] += pval,
        "INT" => t.stats[1] += pval,
        "WIS" => t.stats[2] += pval,
        "DEX" => t.stats[3] += pval,
        "CON" => t.stats[4] += pval,
        "CHR" => t.stats[5] += pval,
        "SPEED" => t.speed += pval,
        "FREE_ACT" => t.free_act = true,
        "STEALTH" => t.stealth += pval,
        "LITE1" => t.lite += 1,
        "LITE2" => t.lite += 2,
        "LITE3" => t.lite += 3,
        "INFRA" => t.infra += pval,
        "TUNNEL" => t.tunnel += pval * 20,
        "BLOWS" => t.blows += pval,
        "MANA" => t.mana_pct += pval,
        "LIFE" => t.life_pct += pval,
        "SPELL" => t.spell_power += pval,
        "LUCK" => t.luck += pval,
        "SENS_FIRE" => t.sens_fire = true,
        "SEE_INVIS" => t.see_invis = true,
        "INVIS" => {
            // TR_INVIS adds pval * 10 invisibility power (xtra1.cc:2514);
            // only a net positive power hides the player.
            t.invis_power += pval * 10;
            if pval > 0 {
                t.invis = true;
            }
        }
        "REFLECT" => t.reflect = true,
        "HOLD_LIFE" => t.hold_life = true,
        "FEATHER" => t.feather = true,
        "FLY" => t.fly = true,
        "CLIMB" => t.climb = true,
        "REGEN" => t.regen = true,
        "SLOW_DIGEST" => t.slow_digest = true,
        "AGGRAVATE" => t.aggravate = true,
        "SH_FIRE" => t.sh_fire = true,
        "SH_ELEC" => t.sh_elec = true,
        "XTRA_MIGHT" => t.xtra_might = true,
        "XTRA_SHOTS" => t.xtra_shots = true,
        "NO_TELE" => t.no_tele = true,
        "NO_MAGIC" => t.no_magic = true,
        "ANTIMAGIC_50" => t.antimagic50 = true,
        "DRAIN_EXP" => t.drain_exp = true,
        "TELEPORT" => t.teleport_itis = true,
        "IMMOVABLE" => t.immovable = true,
        "BLACK_BREATH" => t.black_breath = true,
        "TY_CURSE" | "DG_CURSE" => {
            t.ty_curse = true;
            t.aggravate = true;
        }
        "WRAITH" => t.wraith = true,
        "FAST_CAST" => t.fast_cast = true,
        "NEVER_BLOW" => t.never_blow = true,
        "PRECOGNITION" => t.precognition = true,
        "MAGIC_BREATH" => {
            t.magic_breath = true;
            t.water_breath = true;
        }
        "WATER_BREATH" => t.water_breath = true,
        "CRIT" => t.crit += pval,
        "DRAIN_HP" => t.drain_hp += 1,
        "DRAIN_MANA" => t.drain_mana += 1,
        _ => {
            if let Some(elem) = f.strip_prefix("RES_") {
                // RES_MORGUL counts as nether resistance.
                let elem = if elem == "MORGUL" { "NETHER" } else { elem };
                t.resists.insert(elem.to_string());
            } else if let Some(stat) = f.strip_prefix("SUST_") {
                t.sustains.insert(stat.to_string());
            } else if let Some(elem) = f.strip_prefix("IM_") {
                t.immunities.insert(elem.to_string());
            } else if let Some(race) = f.strip_prefix("ESP_") {
                t.esp.insert(race.to_string());
            } else if is_slay_flag(f) {
                t.slays.insert(f.to_string());
            }
        }
    }
}

/// Apply every intrinsic level flag the character has reached
/// (xtra1.cc apply_lflags: `for i in 1..=lev`).
fn apply_level_flags(t: &mut EquipTotals, flags: &[crate::data::LevelFlag], level: u32) {
    for f in flags {
        if f.level <= level {
            apply_equip_flag(t, &f.flag, f.pval);
        }
    }
}

impl Inventory {
    /// Sum of bonuses from all worn equipment, including artifact set
    /// bonuses (set_info.txt; apply_set in src/object1.cc): wearing N
    /// distinct members applies every member's tiers 1..=N.
    pub fn totals(&self, gd: &GameData) -> EquipTotals {
        let mut t = EquipTotals::default();
        for (slot, item) in self.equip.iter().enumerate() {
            let Some(item) = item else { continue };
            // Ammunition in the quiver lends no melee bonuses or flags
            // (object1.cc calc_bonuses skips INVEN_AMMO); its slays are
            // applied to shots in weapon_slays.
            if slot == data::SLOT_QUIVER {
                continue;
            }
            // A second weapon lends its flags and pval but not its
            // plusses: only the first weapon is swung (xtra1.cc
            // calc_bonuses skips weapon to_h/to_d).
            if slot != data::SLOT_WEAPON2 {
                t.to_h += item.to_h;
                t.to_d += item.to_d;
            }
            t.ac += item.ac + item.to_a;
            for f in item_flags(gd, item) {
                apply_equip_flag(&mut t, f, item.pval);
            }
        }
        for set in &gd.sets {
            let worn: Vec<u32> = set
                .members
                .iter()
                .map(|m| m.artifact)
                .filter(|a| self.equip.iter().flatten().any(|it| it.artifact == *a))
                .collect();
            if worn.is_empty() {
                continue;
            }
            for m in &set.members {
                if !worn.contains(&m.artifact) {
                    continue;
                }
                for j in 0..worn.len().min(m.tiers.len()) {
                    for f in &m.tiers[j].flags {
                        apply_equip_flag(&mut t, f, m.tiers[j].pval);
                    }
                }
            }
        }
        // A climbing set works from the pack (original cmd3 climbing).
        if self
            .pack
            .iter()
            .any(|it| gd.objects[it.def].tval == data::TV_TOOL)
        {
            t.climb = true;
        }
        // A carried symbiotic monster lends its four flags (xtra1.cc
        // calc_wield_monster): invisibility power, reflection and, per the
        // original, ffall for flight and water breathing for aquatic.
        if let Some(sym) = &self.equip[data::SLOT_SYMBIOTE] {
            if let Some(m) = gd.monsters.get(sym.note as usize) {
                if m.has("INVISIBLE") {
                    t.invis_power += 20;
                    t.invis = true;
                }
                if m.has("REFLECTING") {
                    t.reflect = true;
                }
                if m.has("CAN_FLY") {
                    t.feather = true;
                }
                if m.has("AQUATIC") {
                    t.water_breath = true;
                }
            }
        }
        t
    }

    /// Equipment totals plus the character's intrinsic race/class level
    /// flags (p_info R:F:/C:F:; xtra1.cc apply_lflags). While a Possessor
    /// wears a borrowed body the race flags are suppressed, matching the
    /// original calc_bonuses (`!mimic_form && !body_monster`).
    pub fn totals_for(&self, gd: &GameData, ps: &PlayerState) -> EquipTotals {
        let mut t = self.totals(gd);
        // Dungeons with DF_NO_TELEPORT forbid all teleportation
        // (spells1.cc project_p / project_m).
        if ps.depth > 0
            && ps.depth < crate::game::PLOT_DEPTH_BASE
            && gd.dungeon(ps.dungeon).has("NO_TELEPORT")
        {
            t.no_tele = true;
        }
        if ps.possessed.is_none() && ps.mimic_form.is_none() && !ps.disembodied {
            if let Some(race) = gd.races.iter().find(|r| r.name == ps.race_name) {
                apply_level_flags(&mut t, &race.flags, ps.level);
                // Base infravision is racial (xtra1.cc
                // `see_infra = rp_ptr->infra + rmp_ptr->infra`).
                t.infra += race.infra;
            }
            if let Some(subrace) = gd.racemods.get(ps.subrace as usize) {
                apply_level_flags(&mut t, &subrace.flags, ps.level);
                t.infra += subrace.infra;
            }
        }
        // A Possessor's borrowed body overrides the player's body
        // (xtra1.cc calc_body_bonus): its AC, speed and intrinsic flags.
        // A disembodied spirit only takes the Lost Soul wraith form.
        if ps.disembodied {
            t.wraith = true;
        } else if let Some((body, _, _)) = ps.possessed {
            if let Some(m) = gd.monsters.get(body) {
                t.ac += m.ac;
                // calc_body_bonus assigns the body's absolute speed.
                t.speed += m.speed - 110;
                if m.has("NEVER_MOVE") {
                    t.immovable = true;
                }
                if m.has("STUPID") {
                    t.stats[crate::game::INT] -= 1;
                }
                if m.has("SMART") {
                    t.stats[crate::game::INT] += 1;
                }
                if m.has("REFLECTING") {
                    t.reflect = true;
                }
                if m.has("INVISIBLE") {
                    t.invis_power += 20;
                    t.invis = true;
                }
                if m.has("REGENERATE") {
                    t.regen = true;
                }
                if m.has("AURA_FIRE") {
                    t.sh_fire = true;
                }
                if m.has("AURA_ELEC") {
                    t.sh_elec = true;
                }
                if m.has("PASS_WALL") {
                    t.wraith = true;
                }
                if m.has("SUSCEP_FIRE") {
                    t.sens_fire = true;
                }
                if m.has("IM_ACID") {
                    t.resists.insert("ACID".to_string());
                }
                if m.has("IM_ELEC") {
                    t.resists.insert("ELEC".to_string());
                }
                if m.has("IM_FIRE") {
                    t.resists.insert("FIRE".to_string());
                }
                if m.has("IM_POIS") {
                    t.resists.insert("POIS".to_string());
                }
                if m.has("IM_COLD") {
                    t.resists.insert("COLD".to_string());
                }
                if m.has("RES_NETH") {
                    t.resists.insert("NETHER".to_string());
                }
                if m.has("RES_NEXU") {
                    t.resists.insert("NEXUS".to_string());
                }
                if m.has("RES_DISE") {
                    t.resists.insert("DISEN".to_string());
                }
                if m.has("NO_FEAR") {
                    t.resists.insert("FEAR".to_string());
                }
                if m.has("NO_SLEEP") {
                    t.free_act = true;
                }
                if m.has("NO_CONF") {
                    t.resists.insert("CONF".to_string());
                }
                if m.has("CAN_FLY") {
                    t.feather = true;
                }
                if m.has("AQUATIC") {
                    t.water_breath = true;
                }
            }
        }
        // Base luck is fixed at birth: race + subrace + rand_range(-5,5)
        // (birth.cc:391, calc_bonuses `luck_cur = luck_base`).
        t.luck += ps.luck_base;
        if let Some(class) = ps.class(gd) {
            apply_level_flags(&mut t, &class.flags, ps.level);
        }
        crate::corrupt::apply_corruptions(ps, &mut t);
        crate::mimic::apply_mimic_totals(ps, &mut t);
        // A timed Demon Cloak reflects bolts and arrows (set_tim_reflect),
        // and so does Wraithform (xtra1.cc calc_bonuses).
        if ps.tim_reflect > 0 {
            t.reflect = true;
        }
        // Wraithform passes through walls; when the player is embodied it
        // also gives +50 AC and reflection, but a disembodied spirit only
        // gets +10 AC (xtra1.cc:3200-3214 tim_wraith).
        if ps.tim_wraith > 0 {
            t.wraith = true;
            if ps.disembodied {
                t.ac += 10;
            } else {
                t.ac += 50;
                t.reflect = true;
            }
        }
        // A temporary holy aura holds life and brings luck (xtra1.cc:3218);
        // its +1 light is folded into game::player_lite.
        if ps.holy > 0 {
            t.hold_life = true;
            t.luck += 5;
        }
        // The Unbeliever's continuum disruption (xtra1.cc calc_bonuses):
        // an active field freezes space and guards against nexus forces.
        if ps.antimagic_field && ps.skill(crate::skill::SK_ANTIMAGIC) > 0 {
            t.no_tele = true;
            t.resists.insert("NEXUS".to_string());
        }
        t
    }

    /// The slay/brand flags of one worn item (weapon, bow or ammo),
    /// including the set tiers it currently benefits from.
    pub fn item_slays_with_set(&self, gd: &GameData, item: &Item) -> HashSet<String> {
        let mut out = Inventory::item_slays(gd, item);
        if item.artifact != 0 {
            if let Some(set) = gd.set_of_artifact(item.artifact) {
                let n = set
                    .members
                    .iter()
                    .filter(|m| {
                        self.equip
                            .iter()
                            .flatten()
                            .any(|it| it.artifact == m.artifact)
                    })
                    .count();
                if let Some(m) = set.members.iter().find(|m| m.artifact == item.artifact) {
                    for j in 0..n.min(m.tiers.len()) {
                        for f in &m.tiers[j].flags {
                            if is_slay_flag(f) {
                                out.insert(f.clone());
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// The slay/brand flags of one worn item (weapon, bow or ammo).
    pub fn item_slays(gd: &GameData, item: &Item) -> HashSet<String> {
        let mut out = HashSet::new();
        for f in item_flags(gd, item) {
            if is_slay_flag(f) {
                out.insert(f.to_string());
            }
        }
        out
    }

    /// Add an item to the pack, merging similar stacks (object2.cc
    /// inven_carry/object_absorb).
    pub fn add_with(&mut self, gd: &GameData, item: Item) {
        if let Some(i) = self.merge_target(gd, &item) {
            self.absorb(gd, i, item);
            return;
        }
        self.pack.push(item);
    }

    /// object_absorb: blend two similar items into the pack slot.
    pub fn absorb(&mut self, gd: &GameData, i: usize, j: Item) {
        absorb_item_tval(gd, &mut self.pack[i], j);
    }

    /// The pack slot `item` would merge into, if any (object2.cc
    /// object_similar).
    pub fn merge_target(&self, gd: &GameData, item: &Item) -> Option<usize> {
        self.pack.iter().position(|p| items_similar(gd, p, item))
    }

    /// Mark a consumable type as known (after successful use).
    pub fn learn(&mut self, def: usize) {
        self.known.insert(def);
    }
}

/// object2.cc object_absorb: blend `b` into `a` (count, known status,
/// inscription, wand charges). `wand` selects the TV_WAND charge merge;
/// `None` keeps the legacy merge-any-charges behavior (town.rs callers).
pub fn absorb_item_for(a: &mut Item, b: Item, wand: Option<bool>) {
    let total = a.count + b.count;
    a.count = total.min(99);
    if b.identified {
        a.identified = true;
    }
    if !b.inscription.is_empty() {
        a.inscription = b.inscription;
    }
    // Keep the largest discount (object_absorb).
    if b.discount > a.discount {
        a.discount = b.discount;
    }
    // Only wands combine their charges (-LM-); staffs keep the shared
    // per-item charge count required for stacking (object_similar).
    let merge = match wand {
        Some(w) => w,
        None => a.charges >= 0,
    };
    if merge && a.charges >= 0 && b.charges >= 0 {
        a.charges += b.charges;
    }
}

/// object2.cc object_absorb for a known tval.
pub fn absorb_item_tval(gd: &GameData, a: &mut Item, b: Item) {
    let wand = gd.objects[a.def].tval == data::TV_WAND;
    absorb_item_for(a, b, Some(wand));
}

/// Legacy object_absorb entry point (town.rs home_carry).
pub fn absorb_item(a: &mut Item, b: Item) {
    absorb_item_for(a, b, None);
}

/// find_empty_slot (object2.cc:5461): the first free pack slot, or None
/// when the pack holds all 23 slots.
pub fn find_empty_slot(inv: &Inventory) -> Option<usize> {
    // The port's pack is kept compact, so the first free slot is the
    // length; index 23 is the overflow slot.
    if inv.pack.len() < 23 {
        Some(inv.pack.len())
    } else {
        None
    }
}

/// inven_carry_okay (object2.cc:5435): gold is never carried; the pack
/// has room, or a similar stack counts as room.
pub fn inven_carry_okay(gd: &GameData, inv: &Inventory, item: &Item) -> bool {
    if gd.objects[item.def].tval == data::TV_GOLD {
        return false;
    }
    find_empty_slot(inv).is_some() || inv.merge_target(gd, item).is_some()
}

/// inven_item_increase / floor_item_increase (object2.cc:5196/5353):
/// apply a signed stack-size delta, clamped to 0..=255.
pub fn item_increase(item: &mut Item, num: i32) {
    item.count = (item.count as i32 + num).clamp(0, 255) as u32;
}

/// excise_object_idx/delete_object_idx (object2.cc:75/116): remove one
/// object from a floor stack. The ECS caller despawns the entity when the
/// stack empties (o_list slot wipe + c_ptr->o_idxs removal).
pub fn delete_floor_object(stack: &mut Vec<Item>, idx: usize) -> Option<Item> {
    if idx >= stack.len() {
        None
    } else {
        Some(stack.remove(idx))
    }
}

/// delete_object (object2.cc:150): clear every object on a grid; the ECS
/// caller despawns the floor entity.
pub fn delete_floor_stack(stack: &mut Vec<Item>) {
    stack.clear();
}

/// compact_objects_aux (object2.cc:182): move the object at i1 into the
/// hole at i2. The port's object list is positional (Vec/ECS), so the
/// remove+insert replaces the structure copy and index repair.
pub fn compact_objects_aux(items: &mut Vec<Item>, i1: usize, i2: usize) {
    if i1 == i2 || i1 >= items.len() {
        return;
    }
    let it = items.remove(i1);
    let at = i2.min(items.len());
    items.insert(at, it);
}

/// compact_objects (object2.cc:242): the saving throw that spares an object
/// from compaction. High-level objects are immune; objects near the player
/// are immune (cur_dis shrinks with cur_lev); artifacts are immune until
/// 300 + level (400 + level for NORM_ART); otherwise monsters protect at
/// chance 100 and the floor at 90, reduced by cur_lev / 2.
pub fn compaction_spares(
    level: i32,
    art: bool,
    norm_art: bool,
    held: bool,
    distance: i32,
    cur_lev: i32,
    rng: &mut impl Rng,
) -> bool {
    if level > cur_lev {
        return true;
    }
    let cur_dis = 12 * (101 - cur_lev) / 100;
    if cur_dis > 0 && distance < cur_dis {
        return true;
    }
    if art {
        if cur_lev < 300 + level {
            return true;
        }
        if norm_art && cur_lev < 400 + level {
            return true;
        }
    }
    let mut chance = if held { 100 } else { 90 };
    chance -= cur_lev / 2;
    rng.gen_range(0..100) < chance
}

/// get_object (object2.cc:6265): the item addressed by a combined
/// pack/equipment index (negative floor indexes have no ECS equivalent).
pub fn get_object<'a>(
    pack: &'a [Item],
    equip: &'a [Option<Item>],
    item: i32,
) -> Option<&'a Item> {
    if item < 0 {
        return None;
    }
    let i = item as usize;
    if i < pack.len() {
        Some(&pack[i])
    } else {
        equip.get(i - pack.len()).and_then(|s| s.as_ref())
    }
}

/// object2.cc inven_item_optimize: drop empty stacks and split a
/// partially-charged wand/staff stack from its fuller siblings (used
/// after partial destruction).
pub fn inven_item_optimize(inv: &mut Inventory) {
    inv.pack.retain(|it| it.count > 0);
}

/// object2.cc object_similar: can `b` absorb into `a`?
pub fn items_similar(gd: &GameData, a: &Item, b: &Item) -> bool {
    const MAX_STACK_SIZE: u32 = 100;
    if a.def != b.def || a.artifact != b.artifact || a.ego != b.ego || a.ego2 != b.ego2 {
        return false;
    }
    if a.count + b.count >= MAX_STACK_SIZE {
        return false;
    }
    let o = &gd.objects[a.def];
    let fa = item_flags(gd, a);
    let fb = item_flags(gd, b);
    if fa.iter().any(|f| *f == "SPELL_CONTAIN") || fb.iter().any(|f| *f == "SPELL_CONTAIN") {
        return false;
    }
    match o.tval {
        data::TV_BOOK => {
            if !a.identified || !b.identified {
                return false;
            }
            if (a.artifact != 0) != (b.artifact != 0) || (a.ego != 0) != (b.ego != 0) {
                return false;
            }
            if o.sval == 255 && (a.pval != b.pval || a.spells != b.spells) {
                return false;
            }
        }
        data::TV_RANDART
        | data::TV_INSTRUMENT
        | data::TV_HYPNOS
        | data::TV_EGG
        | data::TV_CORPSE
        | data::TV_ROD_MAIN => return false,
        data::TV_TOTEM => {
            if a.pval != b.pval || a.pval2 != b.pval2 {
                return false;
            }
        }
        data::TV_POTION | data::TV_POTION2 => {
            if a.pval2 != b.pval2 {
                return false;
            }
        }
        data::TV_SCROLL => {
            if a.pval != b.pval || a.pval2 != b.pval2 {
                return false;
            }
        }
        data::TV_STAFF => {
            if !a.identified || !b.identified {
                return false;
            }
            if a.charges != b.charges || a.stick_spell != b.stick_spell || a.pval3 != b.pval3 {
                return false;
            }
            if fa.iter().any(|f| *f == "RECHARGED") != fb.iter().any(|f| *f == "RECHARGED") {
                return false;
            }
        }
        data::TV_WAND => {
            if !a.identified || !b.identified {
                return false;
            }
            if a.stick_spell != b.stick_spell || a.pval3 != b.pval3 {
                return false;
            }
            if fa.iter().any(|f| *f == "RECHARGED") != fb.iter().any(|f| *f == "RECHARGED") {
                return false;
            }
        }
        data::TV_ROD => {}
        data::TV_BOW
        | data::TV_BOOMERANG
        | data::TV_DIGGING
        | data::TV_HAFTED
        | data::TV_POLEARM
        | data::TV_MSTAFF
        | data::TV_SWORD
        | data::TV_AXE
        | data::TV_BOOTS
        | data::TV_GLOVES
        | data::TV_HELM
        | data::TV_CROWN
        | data::TV_SHIELD
        | data::TV_CLOAK
        | data::TV_SOFT_ARMOR
        | data::TV_HARD_ARMOR
        | data::TV_DRAG_ARMOR
        | data::TV_DAEMON_BOOK
        | data::TV_RING
        | data::TV_AMULET
        | data::TV_LITE
        | data::TV_BOLT
        | data::TV_ARROW
        | data::TV_SHOT => {
            // Wearables need full knowledge; missiles only matching
            // knowledge; identical plusses/pval/dice and timeout.
            if matches!(o.tval, data::TV_BOLT | data::TV_ARROW | data::TV_SHOT) {
                if a.identified != b.identified {
                    return false;
                }
            } else if !a.identified || !b.identified {
                return false;
            }
            if a.to_h != b.to_h
                || a.to_d != b.to_d
                || a.to_a != b.to_a
                || a.pval != b.pval
                || a.pval2 != b.pval2
                || a.ac != b.ac
                || a.dice != b.dice
                || a.timeout != b.timeout
                || a.cursed != b.cursed
            {
                return false;
            }
        }
        data::TV_FOOD => {
            if a.pval2 != b.pval2 {
                return false;
            }
        }
        _ => {
            if !a.identified || !b.identified {
                return false;
            }
        }
    }
    // Identical instance flags (art_flags in the original).
    let mut sa = a.flags.clone();
    let mut sb = b.flags.clone();
    sa.sort();
    sb.sort();
    if sa != sb {
        return false;
    }
    // Require semi-matching inscriptions (one empty is fine).
    if !a.inscription.is_empty() && !b.inscription.is_empty() && a.inscription != b.inscription {
        return false;
    }
    true
}

/// flag_cost (object2.cc:756): the price of an item's flags in gold.
/// `plusses` is the item's pval. Activations are priced for randarts
/// (artifact_name) from the activation name.
pub fn flag_cost(gd: &GameData, item: &Item, plusses: i32) -> i32 {
    let flags = item_flags(gd, item);
    let has = |f: &str| flags.iter().any(|x| *x == f);
    if has("TEMPORARY") || item.temporary || has("CURSE_NO_DROP") {
        return 0;
    }
    let mut total = 0i32;
    for f in ["STR", "INT", "WIS", "DEX", "CON"] {
        if has(f) {
            total += 1000 * plusses;
        }
    }
    if has("CHR") {
        total += 250 * plusses;
    }
    if has("CHAOTIC") {
        total += 10000;
    }
    if has("VAMPIRIC") {
        total += 13000;
    }
    if has("STEALTH") {
        total += 250 * plusses;
    }
    if has("INFRA") {
        total += 150 * plusses;
    }
    if has("TUNNEL") {
        total += 175 * plusses;
    }
    if has("SPEED") && plusses > 0 {
        total += 10000 + 2500 * plusses;
    }
    if has("BLOWS") && plusses > 0 {
        total += 10000 + 2500 * plusses;
    }
    if has("MANA") {
        total += 1000 * plusses;
    }
    if has("SPELL") {
        total += 2000 * plusses;
    }
    for (f, v) in [
        ("SLAY_ANIMAL", 3500),
        ("SLAY_EVIL", 4500),
        ("SLAY_UNDEAD", 3500),
        ("SLAY_DEMON", 3500),
        ("SLAY_ORC", 3000),
        ("SLAY_TROLL", 3500),
        ("SLAY_GIANT", 3500),
        ("SLAY_DRAGON", 3500),
        ("KILL_DEMON", 5500),
        ("KILL_UNDEAD", 5500),
        ("KILL_DRAGON", 5500),
        ("VORPAL", 5000),
        ("IMPACT", 5000),
        ("BRAND_POIS", 7500),
        ("BRAND_ACID", 7500),
        ("BRAND_ELEC", 7500),
        ("BRAND_FIRE", 5000),
        ("BRAND_COLD", 5000),
        ("SUST_STR", 850),
        ("SUST_INT", 850),
        ("SUST_WIS", 850),
        ("SUST_DEX", 850),
        ("SUST_CON", 850),
        ("SUST_CHR", 250),
        ("INVIS", 3000),
        ("IM_ACID", 10000),
        ("IM_ELEC", 10000),
        ("IM_FIRE", 10000),
        ("IM_COLD", 10000),
        ("REFLECT", 10000),
        ("FREE_ACT", 4500),
        ("HOLD_LIFE", 8500),
        ("RES_ACID", 1250),
        ("RES_ELEC", 1250),
        ("RES_FIRE", 1250),
        ("RES_COLD", 1250),
        ("RES_POIS", 2500),
        ("RES_FEAR", 2500),
        ("RES_LITE", 1750),
        ("RES_DARK", 1750),
        ("RES_BLIND", 2000),
        ("RES_CONF", 2000),
        ("RES_SOUND", 2000),
        ("RES_SHARDS", 2000),
        ("RES_NETHER", 2000),
        ("RES_NEXUS", 2000),
        ("RES_CHAOS", 2000),
        ("RES_DISEN", 10000),
        ("RES_MORGUL", 0),
        ("SH_FIRE", 5000),
        ("SH_ELEC", 5000),
        ("NO_TELE", 2500),
        ("NO_MAGIC", 2500),
        ("WRAITH", 250000),
        ("LITE1", 750),
        ("LITE2", 1250),
        ("LITE3", 2750),
        ("SEE_INVIS", 2000),
        ("SLOW_DIGEST", 750),
        ("REGEN", 2500),
        ("XTRA_MIGHT", 2250),
        ("XTRA_SHOTS", 10000),
        ("IGNORE_ACID", 100),
        ("IGNORE_ELEC", 100),
        ("IGNORE_FIRE", 100),
        ("IGNORE_COLD", 100),
        ("ACTIVATE", 100),
        ("BLESSED", 750),
        ("FEATHER", 1250),
        ("FLY", 10000),
        ("PRECOGNITION", 250000),
        ("DECAY", 0),
        ("EASY_KNOW", 0),
        ("HIDE_TYPE", 0),
        ("SHOW_MODS", 0),
        ("INSTA_ART", 0),
    ] {
        if has(f) {
            total += v;
        }
    }
    if has("LIFE") {
        total += 5000 * plusses;
    }
    if has("SENS_FIRE") {
        total -= 100;
    }
    if has("DRAIN_EXP") {
        total -= 12500;
    }
    let esp = flags.iter().filter(|f| f.starts_with("ESP_")).count() as i32;
    total += 12500 * esp;
    if has("TELEPORT") {
        if item.cursed {
            total -= 7500;
        } else {
            total += 250;
        }
    }
    for (f, v) in [
        ("AGGRAVATE", -10000),
        ("CURSED", -5000),
        ("HEAVY_CURSE", -12500),
        ("PERMA_CURSE", -15000),
        ("NEVER_BLOW", -15000),
        ("BLACK_BREATH", -12500),
        ("DG_CURSE", -25000),
        ("CLONE", -10000),
        ("TY_CURSE", -15000),
    ] {
        if has(f) {
            total += v;
        }
    }
    if has("LEVELS") {
        total += item.elevel * 2000;
    }
    // Extra for a randart's activation.
    if !item.artifact_name.is_empty() && has("ACTIVATE") {
        let act = if item.pval2 >= 0 {
            gd.randarts
                .acts
                .get(item.pval2 as usize)
                .map(|a| a.act.as_str())
                .unwrap_or("")
        } else {
            ""
        };
        total += activation_cost(act);
    }
    total
}

/// The ACT_* price table of flag_cost (object2.cc).
fn activation_cost(act: &str) -> i32 {
    match act {
        "SUNLIGHT" => 250,
        "BO_MISS_1" => 250,
        "BA_POIS_1" => 300,
        "BO_ELEC_1" => 250,
        "BO_ACID_1" => 250,
        "BO_COLD_1" => 250,
        "BO_FIRE_1" => 250,
        "BA_COLD_1" => 750,
        "BA_FIRE_1" => 1000,
        "DRAIN_1" => 500,
        "BA_COLD_2" => 1250,
        "BA_ELEC_2" => 1500,
        "DRAIN_2" => 750,
        "VAMPIRE_1" => 1000,
        "BO_MISS_2" => 1000,
        "BA_FIRE_2" => 1750,
        "BA_COLD_3" => 2500,
        "BA_ELEC_3" => 2500,
        "WHIRLWIND" => 7500,
        "VAMPIRE_2" => 2500,
        "CALL_CHAOS" => 5000,
        "ROCKET" => 5000,
        "DISP_EVIL" => 4000,
        "DISP_GOOD" => 3500,
        "BA_MISS_3" => 5000,
        "CONFUSE" => 500,
        "SLEEP" => 750,
        "QUAKE" => 600,
        "TERROR" => 2500,
        "TELE_AWAY" => 2000,
        "GENOCIDE" => 10000,
        "MASS_GENO" => 10000,
        "CHARM_ANIMAL" => 7500,
        "CHARM_UNDEAD" => 10000,
        "CHARM_OTHER" => 10000,
        "CHARM_ANIMALS" => 12500,
        "CHARM_OTHERS" => 17500,
        "SUMMON_ANIMAL" => 10000,
        "SUMMON_PHANTOM" => 12000,
        "SUMMON_ELEMENTAL" => 15000,
        "SUMMON_DEMON" => 20000,
        "SUMMON_UNDEAD" => 20000,
        "CURE_LW" => 500,
        "CURE_MW" => 750,
        "REST_LIFE" => 7500,
        "REST_ALL" => 15000,
        "CURE_700" => 10000,
        "CURE_1000" => 15000,
        "ESP" => 1500,
        "BERSERK" => 800,
        "PROT_EVIL" => 5000,
        "RESIST_ALL" => 5000,
        "SPEED" => 15000,
        "XTRA_SPEED" => 25000,
        "WRAITH" => 25000,
        "INVULN" => 25000,
        "LIGHT" => 150,
        "MAP_LIGHT" => 500,
        "DETECT_ALL" => 1000,
        "DETECT_XTRA" => 12500,
        "RUNE_EXPLO" => 4000,
        "RUNE_PROT" => 10000,
        "SATIATE" => 2000,
        "DEST_DOOR" => 100,
        "STONE_MUD" => 1000,
        "RECHARGE" => 1000,
        "ALCHEMY" => 10000,
        "DIM_DOOR" => 10000,
        "TELEPORT" => 2000,
        "RECALL" => 7500,
        _ => 0,
    }
}

/// object_value_real (object2.cc:990) without the discount.
pub fn object_value_real(gd: &GameData, item: &Item) -> i32 {
    let o = &gd.objects[item.def];
    // Legacy random artifacts carry their own game-unique price.
    if o.tval == data::TV_RANDART {
        return item.junk_cost.max(0);
    }
    if o.cost == 0 {
        return 0;
    }
    if item.temporary || item_flags(gd, item).iter().any(|f| *f == "TEMPORARY") {
        return 0;
    }
    let mut value = o.cost;
    let flags = item_flags(gd, item);
    let has = |f: &str| flags.iter().any(|x| *x == f);

    if item.ego != 0 || !item.artifact_name.is_empty() || !item.flags.is_empty() {
        value += flag_cost(gd, item, item.pval);
    } else if item.artifact != 0 {
        let Some(a) = gd.artifacts.iter().find(|a| a.id == item.artifact) else {
            return 0;
        };
        if a.cost == 0 {
            return 0;
        }
        value = a.cost;
    }

    // Pay the inscribed spell of containers.
    if has("SPELL_CONTAIN") {
        let spell_level = item
            .spells
            .first()
            .and_then(|n| gd.spell_by_name(n))
            .map(|s| s.level as i32)
            .unwrap_or(0);
        value += if item.pval2 != -1 {
            5000 + 500 * spell_level
        } else {
            5000
        };
    }

    // pval credits for wearable kinds.
    if item.pval != 0
        && matches!(
            o.tval,
            data::TV_BOW
                | data::TV_BOOMERANG
                | data::TV_DIGGING
                | data::TV_HAFTED
                | data::TV_POLEARM
                | data::TV_SWORD
                | data::TV_AXE
                | data::TV_BOOTS
                | data::TV_GLOVES
                | data::TV_HELM
                | data::TV_CROWN
                | data::TV_SHIELD
                | data::TV_CLOAK
                | data::TV_SOFT_ARMOR
                | data::TV_HARD_ARMOR
                | data::TV_DRAG_ARMOR
                | data::TV_LITE
                | data::TV_AMULET
                | data::TV_RING
                | data::TV_MSTAFF
                | data::TV_INSTRUMENT
        )
    {
        for f in ["STR", "INT", "WIS", "DEX", "CON", "CHR"] {
            if has(f) {
                value += item.pval * 200;
            }
        }
        if has("CRIT") {
            value += item.pval * 500;
        }
        if has("STEALTH") {
            value += item.pval * 100;
        }
        if has("INFRA") {
            value += item.pval * 50;
        }
        if has("TUNNEL") {
            value += item.pval * 50;
        }
        if has("BLOWS") {
            value += item.pval * 2000;
        }
        if has("SPEED") {
            value += item.pval * 30000;
        }
    }

    let def_dice = Dice::parse(&o.dice);
    let item_dice = Dice::parse(&item.dice);
    let spell_level = |item: &Item| -> i32 {
        let name = if !item.stick_spell.is_empty() {
            item.stick_spell.clone()
        } else {
            o.spell.clone()
        };
        gd.spell_by_name(&name).map(|s| s.level as i32).unwrap_or(0)
    };
    match o.tval {
        data::TV_EGG => {
            if let Some(m) = gd.monsters.get(item.pval2.max(0) as usize) {
                value += m.depth as i32 * 100;
            }
        }
        data::TV_WAND | data::TV_STAFF => {
            value *= spell_level(item);
            value *= ((item.pval3 >> 16) + (item.pval3 & 0xFFFF)) / 2;
            value /= 6;
            value += (value / 20) * item.charges.max(0) / item.count.max(1) as i32;
        }
        data::TV_BOOK => {
            if o.sval == 255 {
                value *= item
                    .spells
                    .first()
                    .and_then(|n| gd.spell_by_name(n))
                    .map(|s| s.level as i32)
                    .unwrap_or(1);
            }
        }
        data::TV_ROD_MAIN => {
            if item.pval2 > 0 {
                if let Some(tip) = gd.object_by_tval_sval(data::TV_ROD, item.pval2) {
                    value += gd.objects[tip].cost;
                }
            }
        }
        data::TV_RING | data::TV_AMULET => {
            if (item.to_a < 0 || item.to_h < 0 || item.to_d < 0) && value == 0 {
                return 0;
            }
            value += (item.to_h + item.to_d + item.to_a) * 100;
        }
        data::TV_BOOTS
        | data::TV_GLOVES
        | data::TV_CLOAK
        | data::TV_CROWN
        | data::TV_HELM
        | data::TV_SHIELD
        | data::TV_SOFT_ARMOR
        | data::TV_HARD_ARMOR
        | data::TV_DRAG_ARMOR => {
            if item.to_a < 0 && value == 0 {
                return 0;
            }
            value += (item.to_h + item.to_d + item.to_a) * 100;
        }
        data::TV_BOW
        | data::TV_BOOMERANG
        | data::TV_DIGGING
        | data::TV_HAFTED
        | data::TV_SWORD
        | data::TV_DAEMON_BOOK
        | data::TV_AXE
        | data::TV_POLEARM => {
            if item.to_h + item.to_d < 0 && value == 0 {
                return 0;
            }
            value += (item.to_h + item.to_d + item.to_a) * 100;
            if let (Some(ib), Some(db)) = (item_dice, def_dice) {
                if ib.count > db.count && ib.sides == db.sides {
                    value += (ib.count - db.count) * ib.sides * 100;
                }
            }
        }
        data::TV_SHOT | data::TV_ARROW | data::TV_BOLT => {
            if item.to_h + item.to_d < 0 && value == 0 {
                return 0;
            }
            value += (item.to_h + item.to_d) * 5;
            if let (Some(ib), Some(db)) = (item_dice, def_dice) {
                if ib.count > db.count && ib.sides == db.sides {
                    value += (ib.count - db.count) * ib.sides * 5;
                }
            }
            if item.pval2 != 0 {
                value *= 14;
            }
        }
        _ => {}
    }
    value
}

/// object_value (object2.cc:1300): real value with the shop discount.
pub fn object_value(gd: &GameData, item: &Item) -> i32 {
    if item.cursed {
        return 0;
    }
    let mut value = object_value_real(gd, item);
    if item.discount != 0 {
        value -= value * item.discount / 100;
    }
    value.max(0)
}

/// A stack of objects lying on one map cell.
#[derive(Component)]
pub struct FloorItem {
    pub stack: Vec<Item>,
}

/// A pile of gold lying on the floor.
#[derive(Component)]
pub struct FloorGold {
    pub amount: i32,
    /// Coin/gem kind name ("copper", "silver", "rubies", ...); the
    /// original TV_GOLD kind (object2.cc make_gold).
    pub name: String,
}

/// make_gold (object2.cc:4714): pick a treasure kind and a "worth".
/// Returns (k_info id, amount); OBJ_GOLD_LIST 480, MAX_GOLD 18.
pub fn make_gold(gd: &GameData, depth: u32, rng: &mut impl Rng) -> (u32, i32) {
    make_gold_coin(gd, depth, None, rng)
}

/// get_coin_type (xtra2.cc:2070): creeping coin monsters force every gold
/// drop to one treasure kind, matched from the monster's name.
pub fn get_coin_type(def: &crate::data::MonsterDef) -> Option<usize> {
    if def.ch != "$" {
        return None;
    }
    for (needle, idx) in [
        (" copper ", 2usize),
        (" silver ", 5),
        (" gold ", 10),
        (" mithril ", 16),
        (" adamantite ", 17),
        ("Copper ", 2),
        ("Silver ", 5),
        ("Gold ", 10),
        ("Mithril ", 16),
        ("Adamantite ", 17),
    ] {
        if def.name.contains(needle) {
            return Some(idx);
        }
    }
    None
}

/// make_gold with the global `coin_type` override (object2.cc:4728).
pub fn make_gold_coin(
    gd: &GameData,
    depth: u32,
    coin: Option<usize>,
    rng: &mut impl Rng,
) -> (u32, i32) {
    const OBJ_GOLD_LIST: u32 = 480;
    const MAX_GOLD: i32 = 18;
    let level = depth as i32;
    let mut i = ((rng.gen_range(1..=level + 2) + 2) / 2) - 1;
    // Apply "extra" magic: rand_int(GREAT_OBJ) == 0, an occasional deeper
    // treasure kind (object2.cc:4721).
    if rng.gen_range(0..GREAT_OBJ) == 0 {
        i += randint(level + 1, rng);
    }
    // Creeping coins only generate "themselves".
    if let Some(c) = coin {
        i = c as i32;
    }
    if i >= MAX_GOLD {
        i = MAX_GOLD - 1;
    }
    let id = OBJ_GOLD_LIST + i.max(0) as u32;
    let base = gd
        .objects
        .iter()
        .find(|o| o.id == id)
        .map(|o| o.cost)
        .unwrap_or(3)
        .max(1);
    let amount = base + 8 * randint(base, rng) + randint(8, rng);
    (id, amount)
}

/// The coin type of a gold pile ("" when the id is unknown).
pub fn gold_name(gd: &GameData, id: u32) -> String {
    gd.objects
        .iter()
        .find(|o| o.id == id)
        .map(|o| o.name.clone())
        .unwrap_or_default()
}

/// Turn a bare TV_RANDART into one of the 84 legacy random artifacts
/// (object2.cc finalize_randart): each exists once per game, carries a
/// random full name and one of the 51 random activations.
pub fn finalize_junkart(
    gd: &GameData,
    item: &mut Item,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) {
    let n = gd.randarts.junk_f.len().min(gd.randarts.junk_s.len());
    if n == 0 {
        return;
    }
    for _ in 0..2001 {
        let r = rng.gen_range(0..n) as u32;
        // Junkart uniqueness keys live above the a_info ids.
        if created.contains(&(1000 + r)) {
            continue;
        }
        created.insert(1000 + r);
        item.note = r;
        item.artifact_name = gd.randarts.junk_f[r as usize].clone();
        let acts = gd.randarts.acts.len().max(1);
        item.pval2 = rng.gen_range(0..acts) as i32;
        // init_randart: a per-game cost from randnor(0, 250), min 0.
        item.junk_cost = randnor(0, 250, rng).max(0);
        item.identified = true;
        return;
    }
}

/// defines.hpp GREAT_OBJ / MAX_DEPTH (object2.cc get_obj_num level boost).
const GREAT_OBJ: i32 = 20;
const MAX_DEPTH: i32 = 128;

/// object2.cc kind_is_theme: the theme gives each tval a percentage; the
/// junk types get the leftover. TV_GOLD and unhandled tvals never match.
fn kind_is_theme(theme: (u32, u32, u32, u32), o: &data::ObjectDef, rng: &mut impl Rng) -> bool {
    let sum = theme.0 + theme.1 + theme.2 + theme.3;
    if sum == 0 {
        return true;
    }
    if !kind_theme_tval(o.tval) {
        return false;
    }
    let prob = if matches!(
        o.tval,
        data::TV_SKELETON | data::TV_BOTTLE | data::TV_JUNK | data::TV_CORPSE | data::TV_EGG
    ) {
        (100i32 - sum as i32).max(0) as u32
    } else {
        theme_prob(theme, o.tval)
    };
    (rng.gen_range(0..100) as u32) < prob
}

/// The tval -> theme slot mapping of object2.cc kind_is_theme.
fn theme_prob(theme: (u32, u32, u32, u32), tval: i32) -> u32 {
    match tval {
        data::TV_CROWN | data::TV_DRAG_ARMOR | data::TV_AMULET | data::TV_RING => theme.0,
        data::TV_SHOT
        | data::TV_ARROW
        | data::TV_BOLT
        | data::TV_BOOMERANG
        | data::TV_BOW
        | data::TV_HAFTED
        | data::TV_POLEARM
        | data::TV_SWORD
        | data::TV_AXE
        | data::TV_GLOVES
        | data::TV_HELM
        | data::TV_SHIELD
        | data::TV_SOFT_ARMOR
        | data::TV_HARD_ARMOR => theme.1,
        data::TV_MSTAFF
        | data::TV_STAFF
        | data::TV_WAND
        | data::TV_ROD
        | data::TV_ROD_MAIN
        | data::TV_SCROLL
        | data::TV_PARCHMENT
        | data::TV_POTION
        | data::TV_POTION2
        | data::TV_RANDART
        | data::TV_BOOK
        | data::TV_SYMBIOTIC_BOOK
        | data::TV_MUSIC_BOOK
        | data::TV_DRUID_BOOK
        | data::TV_DAEMON_BOOK => theme.2,
        data::TV_LITE
        | data::TV_CLOAK
        | data::TV_BOOTS
        | data::TV_SPIKE
        | data::TV_DIGGING
        | data::TV_FLASK
        | data::TV_FOOD
        | data::TV_TOOL
        | data::TV_INSTRUMENT => theme.3,
        _ => 0,
    }
}

fn kind_theme_tval(tval: i32) -> bool {
    matches!(
        tval,
        data::TV_SKELETON
            | data::TV_BOTTLE
            | data::TV_JUNK
            | data::TV_CORPSE
            | data::TV_EGG
            | data::TV_CROWN
            | data::TV_DRAG_ARMOR
            | data::TV_AMULET
            | data::TV_RING
            | data::TV_SHOT
            | data::TV_ARROW
            | data::TV_BOLT
            | data::TV_BOOMERANG
            | data::TV_BOW
            | data::TV_HAFTED
            | data::TV_POLEARM
            | data::TV_SWORD
            | data::TV_AXE
            | data::TV_GLOVES
            | data::TV_HELM
            | data::TV_SHIELD
            | data::TV_SOFT_ARMOR
            | data::TV_HARD_ARMOR
            | data::TV_MSTAFF
            | data::TV_STAFF
            | data::TV_WAND
            | data::TV_ROD
            | data::TV_ROD_MAIN
            | data::TV_SCROLL
            | data::TV_PARCHMENT
            | data::TV_POTION
            | data::TV_POTION2
            | data::TV_RANDART
            | data::TV_BOOK
            | data::TV_SYMBIOTIC_BOOK
            | data::TV_MUSIC_BOOK
            | data::TV_DRUID_BOOK
            | data::TV_DAEMON_BOOK
            | data::TV_LITE
            | data::TV_CLOAK
            | data::TV_BOOTS
            | data::TV_SPIKE
            | data::TV_DIGGING
            | data::TV_FLASK
            | data::TV_FOOD
            | data::TV_TOOL
            | data::TV_INSTRUMENT
    )
}

/// The created-key of a NORM_ART kind (the original's k_ptr->artifact).
pub fn norm_art_key(def: usize) -> u32 {
    100_000 + def as u32
}

/// object2.cc kind_is_legal: theme + special/uniqueness restrictions.
pub fn kind_is_legal(
    gd: &GameData,
    theme: (u32, u32, u32, u32),
    def: usize,
    created: &HashSet<u32>,
    rng: &mut impl Rng,
) -> bool {
    let o = &gd.objects[def];
    if !kind_is_theme(theme, o, rng) {
        return false;
    }
    // SPECIAL_GENE only via a_allow_special (never in the normal pool).
    if o.flags.iter().any(|f| f == "SPECIAL_GENE") {
        return false;
    }
    // No two base kinds created twice (TR_NORM_ART).
    if o.flags.iter().any(|f| f == "NORM_ART") && created.contains(&norm_art_key(def)) {
        return false;
    }
    if o.tval == data::TV_CORPSE {
        return !matches!(o.sval, 1 | 2 | 3 | 4);
    }
    if o.tval == data::TV_HYPNOS {
        return false;
    }
    // The One Ring parchment is quest-only in this port (the real Ring is
    // a_info 13, spawned by q_one).
    if o.name == "The One Ring" {
        return false;
    }
    // Used only for the Nazgul rings (SV_RING_SPECIAL).
    if o.tval == data::TV_RING && o.sval == 5 {
        return false;
    }
    true
}

/// kind_is_legal without a dungeon theme or created-kind set (store.cc
/// kind_is_storeok's per-kind form).
pub fn kind_is_legal_plain(gd: &GameData, def: usize, rng: &mut impl Rng) -> bool {
    kind_is_legal(gd, (0, 0, 0, 0), def, &HashSet::new(), rng)
}

/// object2.cc kind_is_good: the "good object" allocation hook.
pub fn kind_is_good(gd: &GameData, def: usize) -> bool {
    let o = &gd.objects[def];
    match o.tval {
        data::TV_HARD_ARMOR
        | data::TV_SOFT_ARMOR
        | data::TV_DRAG_ARMOR
        | data::TV_SHIELD
        | data::TV_CLOAK
        | data::TV_BOOTS
        | data::TV_GLOVES
        | data::TV_HELM
        | data::TV_CROWN => o.to_a >= 0,
        data::TV_BOW
        | data::TV_SWORD
        | data::TV_AXE
        | data::TV_HAFTED
        | data::TV_POLEARM
        | data::TV_DIGGING
        | data::TV_MSTAFF
        | data::TV_BOOMERANG => o.to_h >= 0 && o.to_d >= 0,
        data::TV_BOLT | data::TV_ARROW => true,
        // Rods: silver and better (SV_ROD_SILVER = 100).
        data::TV_ROD_MAIN => o.sval >= 100,
        // Expensive rod tips.
        data::TV_ROD => o.cost >= 4500,
        // The good tomes (SV_BOOK_MAX_GOOD = 49).
        data::TV_BOOK => o.sval <= 49,
        // Rings: Speed only (SV_RING_SPEED = 31).
        data::TV_RING => o.sval == 31,
        data::TV_AMULET => matches!(o.sval, 8 | 9 | 15 | 22 | 23 | 24 | 25),
        _ => false,
    }
}

/// object2.cc kind_is_artifactable: a good kind that can carry at least
/// one ra_info randart power.
pub fn kind_is_artifactable(gd: &GameData, def: usize) -> bool {
    if !kind_is_good(gd, def) {
        return false;
    }
    let o = &gd.objects[def];
    gd.randarts.parts.iter().any(|p| {
        p.tvals
            .iter()
            .any(|f| f.tval == o.tval && f.min <= o.sval && f.max >= o.sval)
    })
}

/// One allocation row of init2.cc init_alloc.
struct AllocRow {
    def: usize,
    level: u32,
    prob: u32,
}

/// get_obj_num (object2.cc:568): pick from the A: allocation table.
fn get_obj_num(
    gd: &GameData,
    level: u32,
    theme: (u32, u32, u32, u32),
    good: bool,
    created: &HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<usize> {
    // Boost level occasionally (bizarre calculation): rand_int(GREAT_OBJ)
    // is 0..=GREAT_OBJ-1 and rand_int(MAX_DEPTH) 0..=MAX_DEPTH-1.
    let mut level = level;
    if level > 0 && rng.gen_range(0..GREAT_OBJ) == 0 {
        level = 1 + (level as i32 * MAX_DEPTH / rng.gen_range(0..MAX_DEPTH).max(1)) as u32;
    }
    let mut table: Vec<AllocRow> = Vec::new();
    for (def, o) in gd.objects.iter().enumerate() {
        for &(locale, chance) in &o.alloc {
            if chance == 0 || locale > level {
                continue;
            }
            if good && !kind_is_good(gd, def) {
                continue;
            }
            if !kind_is_legal(gd, theme, def, created, rng) {
                continue;
            }
            table.push(AllocRow {
                def,
                level: locale,
                prob: (100 / chance).max(1),
            });
        }
    }
    if table.is_empty() {
        return None;
    }
    let weights: Vec<u32> = table.iter().map(|e| e.prob).collect();
    let dist = WeightedIndex::new(&weights).ok()?;
    let mut i = dist.sample(rng);
    let p = rng.gen_range(0..100);
    // Try for a "better" object once (60%) or twice (10%).
    if p < 60 {
        let j = i;
        let k = dist.sample(rng);
        i = if table[k].level < table[j].level {
            j
        } else {
            k
        };
    }
    if p < 10 {
        let j = i;
        let k = dist.sample(rng);
        i = if table[k].level < table[j].level {
            j
        } else {
            k
        };
    }
    Some(table[i].def)
}

/// Roll an object kind appropriate for the dungeon depth (normal
/// allocation, no theme restriction; see `gen_object_kind_themed`).
/// tval 0 items (plot) never spawn randomly.
pub fn gen_object_kind(gd: &GameData, depth: u32, rng: &mut impl Rng) -> Option<usize> {
    gen_object_kind_themed(gd, depth, None, false, None, rng)
}

/// The full make_object kind roll: dungeon object theme, good-object
/// restriction and NORM_ART uniqueness (object2.cc make_object).
pub fn gen_object_kind_themed(
    gd: &GameData,
    depth: u32,
    theme: Option<(u32, u32, u32, u32)>,
    good: bool,
    created: Option<&HashSet<u32>>,
    rng: &mut impl Rng,
) -> Option<usize> {
    let empty = HashSet::new();
    let created = created.unwrap_or(&empty);
    let theme = theme.unwrap_or((0, 0, 0, 0));
    get_obj_num(gd, depth, theme, good, created, rng)
}

/// Artifacts already generated this game (each artifact is unique).
#[derive(Resource, Default, Serialize, Deserialize)]
pub struct CreatedArtifacts(pub HashSet<u32>);

/// Maybe turn a piece of gear into an ego item.
/// make_ego_item (object2.cc:2192): pick an ego fitting the base kind,
/// then apply its C: plusses/pval and its R: rarity groups.
fn apply_ego(gd: &GameData, item: &mut Item, depth: u32, good: bool, rng: &mut impl Rng) {
    let o = &gd.objects[item.def];
    let (tval, sval) = (o.tval, o.sval);
    let base_flags = &o.flags;
    let ok: Vec<&data::EgoDef> = gd
        .egos
        .iter()
        .filter(|e| {
            if e.name.is_empty() {
                return false;
            }
            if !e
                .svals
                .iter()
                .any(|&(tv, min, max)| tv == tval && min <= sval && max >= sval)
            {
                return false;
            }
            // Good egos are worth something, bad ones cost nothing.
            if good && e.cost == 0 {
                return false;
            }
            if !good && e.cost != 0 {
                return false;
            }
            // r:N:/r:F: base flag requirements.
            if e.need_flags.iter().any(|f| !base_flags.contains(f)) {
                return false;
            }
            if e.forbid_flags.iter().any(|f| base_flags.contains(f)) {
                return false;
            }
            true
        })
        .collect();
    if ok.is_empty() {
        return;
    }
    let mut chosen: Option<&data::EgoDef> = None;
    for _ in 0..ok.len() * 10 {
        let e = ok[rng.gen_range(0..ok.len())];
        // Loose out-of-depth check.
        if e.depth > depth {
            let d = (e.depth - depth) as i32;
            if rng.gen_range(0..d.max(1)) != 0 {
                continue;
            }
        }
        // The mrarity/rarity rarity roll.
        let mr = e.rarity.max(1) as i32;
        let luck = rng.gen_range(-(mr / 2)..=(mr / 2));
        if rng.gen_range(0..(mr - luck).max(1)) > e.rarity1 as i32 {
            continue;
        }
        chosen = Some(e);
        break;
    }
    let Some(e) = chosen else {
        return;
    };
    item.ego = e.id;
    apply_ego_effects(gd, item, e, tval, depth, rng);
    // Rarely a second, complementary ego ("of Might of the Magi";
    // object2.cc:2286-2327).  Prefix and suffix must not repeat.
    let first_prefix = e.prefix;
    let chance = 7 + rng.gen_range(-7..=7);
    if chance > 0 && rng.gen_range(0..100) < chance && item.ego2 == 0 {
        let mut chosen2: Option<&data::EgoDef> = None;
        for _ in 0..ok.len() * 10 {
            let e2 = ok[rng.gen_range(0..ok.len())];
            if e2.id == item.ego || e2.prefix == first_prefix {
                continue;
            }
            if e2.depth > depth {
                let d = (e2.depth - depth) as i32;
                if rng.gen_range(0..d.max(1)) != 0 {
                    continue;
                }
            }
            let mr = e2.rarity.max(1) as i32;
            let luck = rng.gen_range(-(mr / 2)..=(mr / 2));
            if rng.gen_range(0..(mr - luck).max(1)) > e2.rarity1 as i32 {
                continue;
            }
            chosen2 = Some(e2);
            break;
        }
        if let Some(e2) = chosen2 {
            item.ego2 = e2.id;
            apply_ego_effects(gd, item, e2, tval, depth, rng);
        }
    }
}

/// Apply an ego's R: rarity groups and C: plusses to the instance
/// (object2.cc make_ego_item 2324-2340).  Also used by the
/// brand_bolts/brand_ammo activations (cmd6.cc/cmd7.cc).
pub(crate) fn apply_ego_effects(
    gd: &GameData,
    item: &mut Item,
    e: &data::EgoDef,
    tval: i32,
    depth: u32,
    rng: &mut impl Rng,
) {
    if e.cursed {
        item.cursed = true;
    }
    // R: rarity groups: each gets its own magik(chance) roll; real flags
    // are OR'd in, generation flags call add_random_ego_flag.
    let mut limit_blows = false;
    for g in &e.groups {
        if g.chance < 100 && rng.gen_range(1..=100) > g.chance {
            continue;
        }
        for f in &g.flags {
            if !item.flags.iter().any(|x| x == f) {
                item.flags.push(f.clone());
            }
        }
        for f in &g.fego {
            add_random_ego_flag(item, tval, depth, f, &mut limit_blows, rng);
        }
    }
    // No insane number of blows.
    if limit_blows && item.flags.iter().any(|f| f == "BLOWS") && item.pval > 2 {
        item.pval = rng.gen_range(1..=2);
    }
    // C: bonuses (randint(max), not 0..=max).
    for (v, dst) in [(e.to_h, 0usize), (e.to_d, 1), (e.to_a, 2)] {
        if v > 0 {
            let roll = rng.gen_range(1..=v);
            match dst {
                0 => item.to_h += roll,
                1 => item.to_d += roll,
                _ => item.to_a += roll,
            }
        } else if v < 0 {
            let roll = rng.gen_range(1..=-v);
            match dst {
                0 => item.to_h -= roll,
                1 => item.to_d -= roll,
                _ => item.to_a -= roll,
            }
        }
    }
    if e.pval > 0 {
        item.pval += rng.gen_range(1..=e.pval);
    } else if e.pval < 0 {
        item.pval -= rng.gen_range(1..=-e.pval);
    }
    // Spell containers hold no spell of their own.
    if item_flags(gd, item).iter().any(|f| *f == "SPELL_CONTAIN") {
        item.pval2 = -1;
    }
    item.identified = false;
}

/// `object_prep` + a forced `name2` ego, then `apply_magic`'s ego analysis
/// (q_poison.cc:171 reward): the ego is applied even when its `T:` list does
/// not name the base kind (the Elvenkind quest armour).
pub fn force_ego(
    gd: &GameData,
    def: usize,
    ego_id: u32,
    depth: u32,
    rng: &mut impl Rng,
) -> Item {
    let mut item = Item::base(gd, def);
    if let Some(e) = gd.egos.iter().find(|e| e.id == ego_id) {
        let tval = gd.objects[def].tval;
        apply_ego_effects(gd, &mut item, e, tval, depth, rng);
        item.ego = e.id;
    }
    item
}

/// dragon_resist (object2.cc:2447): one random resist, sometimes several.
fn dragon_resist(item: &mut Item, tval: i32, rng: &mut impl Rng) {
    loop {
        let specific = if randint(4, rng) == 1 {
            rng.gen_range(5..=18)
        } else {
            rng.gen_range(17..=38)
        };
        random_resistance_specific(item, tval, specific, rng);
        if randint(2, rng) != 1 {
            break;
        }
    }
}

/// a_m_aux_2 (object2.cc:2468): armour special cases.
fn armour_magic(item: &mut Item, tval: i32, sval: i32, level: i32, rng: &mut impl Rng) {
    match (tval, sval) {
        // Elven cloaks carry 1..=4 stealth pval.
        (data::TV_CLOAK, 2) => item.pval = rng.gen_range(1..=4),
        // A Cloak of Mimicry stores its shape in pval2.
        (data::TV_CLOAK, 100) => {
            item.pval2 = crate::mimic::find_random_mimic_shape(level, true, rng) as i32
        }
        // Dragon scale shields/helms roll their dragon resistance.
        (data::TV_SHIELD, 6) | (data::TV_HELM, 7) => dragon_resist(item, tval, rng),
        _ => {}
    }
}

/// add_random_ego_flag (object2.cc:3292): the ETR_* creation powers.
fn add_random_ego_flag(
    item: &mut Item,
    tval: i32,
    depth: u32,
    flag: &str,
    limit_blows: &mut bool,
    rng: &mut impl Rng,
) {
    let push = |item: &mut Item, f: &str| {
        if !item.flags.iter().any(|x| x == f) {
            item.flags.push(f.to_string());
        }
    };
    match flag {
        "SUSTAIN" => {
            const S: [&str; 6] = [
                "SUST_STR", "SUST_INT", "SUST_WIS", "SUST_DEX", "SUST_CON", "SUST_CHR",
            ];
            push(item, S[rng.gen_range(0..6)]);
        }
        "OLD_RESIST" => {
            const HI: [&str; 11] = [
                "RES_BLIND",
                "RES_CONF",
                "RES_SOUND",
                "RES_SHARDS",
                "RES_NETHER",
                "RES_NEXUS",
                "RES_CHAOS",
                "RES_DISEN",
                "RES_POIS",
                "RES_DARK",
                "RES_LITE",
            ];
            push(item, HI[rng.gen_range(0..HI.len())]);
        }
        "ABILITY" => {
            const AB: [&str; 8] = [
                "FEATHER",
                "LITE1",
                "SEE_INVIS",
                "ESP_ALL",
                "SLOW_DIGEST",
                "REGEN",
                "FREE_ACT",
                "HOLD_LIFE",
            ];
            push(item, AB[rng.gen_range(0..AB.len())]);
        }
        "R_ELEM" => random_resistance_specific(item, tval, rng.gen_range(5..=18), rng),
        "R_LOW" => random_resistance_specific(item, tval, rng.gen_range(5..=16), rng),
        "R_HIGH" => random_resistance_specific(item, tval, rng.gen_range(17..=38), rng),
        "R_ANY" => random_resistance_specific(item, tval, rng.gen_range(5..=38), rng),
        "R_DRAGON" => dragon_resist(item, tval, rng),
        "SLAY_WEAP" => {
            if let Some(mut d) = Dice::parse(&item.dice) {
                if rng.gen_range(1..=3) == 1 {
                    d.count *= 2;
                } else {
                    d.count += 1;
                    d.sides += 1;
                }
                item.dice = fmt_dice(d);
            }
            if rng.gen_range(1..=5) == 1 {
                push(item, "BRAND_POIS");
            }
            if tval == data::TV_SWORD && rng.gen_range(1..=3) == 1 {
                push(item, "VORPAL");
            }
        }
        "DAM_DIE" => {
            if let Some(mut d) = Dice::parse(&item.dice) {
                d.count += 1;
                item.dice = fmt_dice(d);
            }
        }
        "DAM_SIZE" => {
            if let Some(mut d) = Dice::parse(&item.dice) {
                d.sides += 1;
                item.dice = fmt_dice(d);
            }
        }
        "PVAL_M1" => item.pval += 1,
        "PVAL_M2" => item.pval += m_bonus(2, depth as i32, rng),
        "PVAL_M3" => item.pval += m_bonus(3, depth as i32, rng),
        "PVAL_M5" => item.pval += m_bonus(5, depth as i32, rng),
        "AC_M1" => item.to_a += 1,
        "AC_M2" => item.to_a += m_bonus(2, depth as i32, rng),
        "AC_M3" => item.to_a += m_bonus(3, depth as i32, rng),
        "AC_M5" => item.to_a += m_bonus(5, depth as i32, rng),
        "TH_M1" => item.to_h += 1,
        "TH_M2" => item.to_h += m_bonus(2, depth as i32, rng),
        "TH_M3" => item.to_h += m_bonus(3, depth as i32, rng),
        "TH_M5" => item.to_h += m_bonus(5, depth as i32, rng),
        "TD_M1" => item.to_d += 1,
        "TD_M2" => item.to_d += m_bonus(2, depth as i32, rng),
        "TD_M3" => item.to_d += m_bonus(3, depth as i32, rng),
        "TD_M5" => item.to_d += m_bonus(5, depth as i32, rng),
        "R_P_ABILITY" => {
            let f = ["STEALTH", "INFRA", "TUNNEL", "SPEED", "BLOWS"][rng.gen_range(0..5)];
            push(item, f);
        }
        "R_STAT" => {
            const S: [&str; 6] = ["STR", "INT", "WIS", "DEX", "CON", "CHR"];
            push(item, S[rng.gen_range(0..6)]);
        }
        "R_STAT_SUST" => {
            const S: [&str; 6] = ["STR", "INT", "WIS", "DEX", "CON", "CHR"];
            let s = S[rng.gen_range(0..6)];
            push(item, s);
            push(item, &format!("SUST_{}", s));
        }
        "R_IMMUNITY" => {
            let i = rng.gen_range(0..4);
            let (im, ig) = [
                ("IM_FIRE", "IGNORE_FIRE"),
                ("IM_ACID", "IGNORE_ACID"),
                ("IM_ELEC", "IGNORE_ELEC"),
                ("IM_COLD", "IGNORE_COLD"),
            ][i];
            push(item, im);
            push(item, ig);
        }
        "LIMIT_BLOWS" => *limit_blows = !*limit_blows,
        _ => {}
    }
}

/// Format a dice expression back ("2d6", "1d8+2").
fn fmt_dice(d: Dice) -> String {
    if d.bonus > 0 {
        format!("{}d{}+{}", d.count, d.sides, d.bonus)
    } else {
        format!("{}d{}", d.count, d.sides)
    }
}

// --- Randarts (create_artifact, src/randart.cc) --------------------------

/// z-rand.cc `randint` (1..=n, 1 for n < 2).  This helper used to be
/// misnamed `rand_int`; every call site corresponds to an original
/// `randint` call (the original `rand_int` is 0..=n-1).
fn randint(n: i32, rng: &mut impl Rng) -> i32 {
    crate::rng::randint(n, rng)
}

/// roll <dd>d<ds> (z-rand.cc damroll).
fn damroll(dd: i32, ds: i32, rng: &mut impl Rng) -> i32 {
    crate::rng::damroll(dd, ds, rng)
}

/// The randart name corpus turns into a trigram model (build_prob).
type NameProbs = std::collections::HashMap<(u8, u8, u8), i64>;
type NameTotals = std::collections::HashMap<(u8, u8), i64>;

fn build_name_probs(names: &str) -> (NameProbs, NameTotals) {
    const S_WORD: u8 = 26;
    let mut probs: NameProbs = Default::default();
    let mut totals: NameTotals = Default::default();
    for name in names.split('\n') {
        let mut c_prev = S_WORD;
        let mut c_cur = S_WORD;
        for ch in name.chars() {
            if !ch.is_ascii_alphabetic() {
                continue;
            }
            let c_next = ch.to_ascii_lowercase() as u8 - b'a';
            *probs.entry((c_prev, c_cur, c_next)).or_insert(0) += 1;
            *totals.entry((c_prev, c_cur)).or_insert(0) += 1;
            c_prev = c_cur;
            c_cur = c_next;
        }
        *probs.entry((c_prev, c_cur, S_WORD)).or_insert(0) += 1;
        *totals.entry((c_prev, c_cur)).or_insert(0) += 1;
    }
    (probs, totals)
}

/// make_word: a random 5-9 letter word with at least one vowel.
fn make_word(names: &str, rng: &mut impl Rng) -> String {
    const S_WORD: u8 = 26;
    let (probs, totals) = build_name_probs(names);
    'startover: loop {
        let mut vow = 0;
        let mut lnum = 0;
        let mut tries = 0;
        let mut word: Vec<u8> = Vec::new();
        let (mut c_prev, mut c_cur) = (S_WORD, S_WORD);
        loop {
            let total = *totals.get(&(c_prev, c_cur)).unwrap_or(&0);
            if total <= 0 {
                continue 'startover;
            }
            let r = rng.gen_range(0..total);
            let mut c_next = 0u8;
            let mut cum = *probs.get(&(c_prev, c_cur, 0)).unwrap_or(&0);
            while cum <= r {
                if c_next >= S_WORD {
                    break;
                }
                c_next += 1;
                cum += *probs.get(&(c_prev, c_cur, c_next)).unwrap_or(&0);
            }
            if c_next == S_WORD {
                if lnum < 5 || vow == 0 {
                    tries += 1;
                    if tries < 10 {
                        continue;
                    }
                    continue 'startover;
                }
                break;
            }
            if lnum >= 9 {
                continue 'startover;
            }
            word.push(b'a' + c_next);
            if matches!(c_next, 0 | 4 | 8 | 14 | 20) {
                vow += 1;
            }
            lnum += 1;
            c_prev = c_cur;
            c_cur = c_next;
        }
        let s: String = word.iter().map(|&b| b as char).collect();
        let mut chars = s.chars();
        let first = chars.next().unwrap_or('a').to_ascii_uppercase();
        return format!("{}{}", first, chars.as_str());
    }
}

/// get_random_name: 1/3 "'Word'", else "of Word".
fn random_artifact_name(names: &str, rng: &mut impl Rng) -> String {
    let word = make_word(names, rng);
    if rng.gen_range(0..3) == 0 {
        format!("'{}'", word)
    } else {
        format!("of {}", word)
    }
}

/// Cursed randarts (curse_artifact, src/spells2.cc): flip and penalize
/// every bonus, then pile on the curse flags.
fn curse_artifact(item: &mut Item, rng: &mut impl Rng) {
    if item.pval != 0 {
        item.pval = -(item.pval + randint(4, rng));
    }
    if item.to_a != 0 {
        item.to_a = -(item.to_a + randint(4, rng));
    }
    if item.to_h != 0 {
        item.to_h = -(item.to_h + randint(4, rng));
    }
    if item.to_d != 0 {
        item.to_d = -(item.to_d + randint(4, rng));
    }
    add_item_flag(item, "HEAVY_CURSE");
    add_item_flag(item, "CURSED");
    if randint(3, rng) == 1 {
        add_item_flag(item, "TY_CURSE");
    }
    if randint(2, rng) == 1 {
        add_item_flag(item, "AGGRAVATE");
    }
    if randint(3, rng) == 1 {
        add_item_flag(item, "DRAIN_EXP");
    }
    if randint(3, rng) == 1 {
        add_item_flag(item, "BLACK_BREATH");
    }
    if randint(2, rng) == 1 {
        add_item_flag(item, "TELEPORT");
    } else if randint(3, rng) == 1 {
        add_item_flag(item, "NO_TELE");
    }
    item.cursed = true;
}

/// grab_one_power (src/randart.cc): eligibility filters, then up to
/// `ok * 10` random draws through the level and rarity gates.
fn grab_one_power(
    gd: &GameData,
    item: &Item,
    ps_level: u32,
    max_times: &mut std::collections::HashMap<u32, i32>,
    rng: &mut impl Rng,
) -> Option<usize> {
    let o = &gd.objects[item.def];
    let flags = item_flags(gd, item);
    let has_flag = |f: &str| flags.iter().any(|x| *x == f);
    let ok: Vec<usize> = gd
        .randarts
        .parts
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let tval_ok = p
                .tvals
                .iter()
                .any(|t| t.tval == o.tval && t.min <= o.sval && o.sval <= t.max);
            if !tval_ok || p.value <= 0 {
                return false;
            }
            if p.pval > 0 && p.pval < item.pval {
                return false;
            }
            if max_times.get(&p.id).copied().unwrap_or(0) >= p.max {
                return false;
            }
            if p.aflags.iter().any(|f| has_flag(f)) {
                return false;
            }
            true
        })
        .map(|(i, _)| i)
        .collect();
    if ok.is_empty() {
        return None;
    }
    for _ in 0..ok.len() * 10 {
        let i = ok[rng.gen_range(0..ok.len())];
        let p = &gd.randarts.parts[i];
        if p.level > ps_level {
            let d = p.level - ps_level;
            if rng.gen_range(0..d) != 0 {
                continue;
            }
        }
        if rng.gen_range(0..p.mrarity.max(1)) < p.rarity {
            continue;
        }
        *max_times.entry(p.id).or_insert(0) += 1;
        return Some(i);
    }
    None
}

/// create_artifact (src/randart.cc): imbue an item with random powers.
/// Returns true (the original always names the item once called).
pub fn create_artifact(
    gd: &GameData,
    item: &mut Item,
    ps_level: u32,
    a_scroll: bool,
    player_name: &str,
    rng: &mut impl Rng,
) -> bool {
    let mut a_cursed = false;
    if !a_scroll && randint(13, rng) == 1 {
        a_cursed = true;
    }
    let mut powers: i32 = gd
        .randarts
        .gen
        .iter()
        .map(|g| damroll(g.dd, g.ds, rng) + g.plus)
        .sum();
    if !a_cursed && randint(30, rng) == 1 {
        powers *= 2;
    }
    if a_cursed {
        powers /= 2;
    }
    let mut max_times: std::collections::HashMap<u32, i32> = Default::default();
    let mut pval = 0i32;
    while powers > 0 {
        powers -= 1;
        let Some(i) = grab_one_power(gd, item, ps_level, &mut max_times, rng) else {
            continue;
        };
        let p = gd.randarts.parts[i].clone();
        for f in &p.flags {
            if !item.flags.iter().any(|x| x == f) {
                item.flags.push(f.clone());
            }
        }
        if p.to_h > 0 {
            item.to_h += randint(p.to_h, rng);
        } else if p.to_h < 0 {
            item.to_h -= randint(-p.to_h, rng);
        }
        if p.to_d > 0 {
            item.to_d += randint(p.to_d, rng);
        } else if p.to_d < 0 {
            item.to_d -= randint(-p.to_d, rng);
        }
        if p.to_a > 0 {
            item.to_a += randint(p.to_a, rng);
        } else if p.to_a < 0 {
            item.to_a -= randint(-p.to_a, rng);
        }
        if ((pval > p.pval) && p.pval != 0) || pval == 0 {
            pval = p.pval;
        }
    }
    if pval > 0 {
        item.pval = randint(pval, rng);
    } else if pval < 0 {
        item.pval = randint(-pval, rng);
    }
    // Every randart shrugs off the four elements (randart.cc).
    for f in ["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"] {
        if !item.flags.iter().any(|x| x == f) {
            item.flags.push(f.to_string());
        }
    }
    if a_cursed {
        curse_artifact(item, rng);
    }
    item.ego = 0;
    item.ego2 = 0;
    item.artifact = 0;
    item.artifact_name = if a_scroll {
        // Default name "of '<player>'" (randart.cc:343). A custom name
        // would need a text-input modal.
        format!("of '{}'", player_name)
    } else {
        random_artifact_name(&gd.randarts.names, rng)
    };
    // ToME hack: spell containers know no spell until one is inscribed.
    if item_flags(gd, item).iter().any(|f| *f == "SPELL_CONTAIN") {
        item.pval2 = -1;
    }
    item.identified = false;
    true
}

/// random_artifact_power (src/object2.cc): one of the eight minor
/// artifact abilities, avoiding duplicates.
fn random_artifact_power(item: &mut Item, rng: &mut impl Rng) {
    const POWERS: [&str; 8] = [
        "FEATHER",
        "LITE1",
        "SEE_INVIS",
        "ESP_ALL",
        "SLOW_DIGEST",
        "REGEN",
        "FREE_ACT",
        "HOLD_LIFE",
    ];
    for _ in 0..1000 {
        let p = POWERS[rng.gen_range(0..POWERS.len())];
        if !item.flags.iter().any(|x| x == p) {
            item.flags.push(p.to_string());
            return;
        }
    }
}

/// random_artifact_resistance (src/object2.cc): a_info artifacts with
/// RANDOM_RESIST / RANDOM_POWER / RANDOM_RES_OR_POWER gain a random
/// resistance and/or one of the minor artifact powers when created.
pub fn artifact_random_flags(gd: &GameData, item: &mut Item, rng: &mut impl Rng) {
    let Some(a) = gd.artifacts.iter().find(|a| a.id == item.artifact) else {
        return;
    };
    let mut give_res = a.flags.iter().any(|f| f == "RANDOM_RESIST");
    let mut give_power = a.flags.iter().any(|f| f == "RANDOM_POWER");
    if a.flags.iter().any(|f| f == "RANDOM_RES_OR_POWER") {
        if randint(2, rng) == 1 {
            give_res = true;
        } else {
            give_power = true;
        }
    }
    if give_power {
        random_artifact_power(item, rng);
    }
    if give_res {
        // object2.cc:2018: artifact_bias = 0, then roll in 17..=38.
        let tval = gd.objects[item.def].tval;
        let mut bias = ArtifactBias::None;
        for _ in 0..1000 {
            let before = item.flags.len();
            random_resistance(item, tval, rng.gen_range(17..=38), &mut bias, rng);
            if item.flags.len() != before {
                break;
            }
        }
    }
}

/// Maybe turn a piece of gear into an artifact (rare). Each artifact
/// exists only once per game; an already-created artifact fails the roll.
fn apply_artifact(
    gd: &GameData,
    item: &mut Item,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
    depth: u32,
) -> bool {
    // No artifacts in the town (object2.cc make_artifact).
    if depth == 0 {
        return false;
    }
    let o = &gd.objects[item.def];
    // make_artifact (object2.cc:2120): list order, out-of-depth and
    // rarity rolls per candidate.
    for a in &gd.artifacts {
        if a.tval != o.tval || a.sval != o.sval || a.insta_art || created.contains(&a.id) {
            continue;
        }
        // Guardians' and ultimate artifacts never spawn at random
        // (TR_SPECIAL_GENE unless a_allow_special).
        if a.flags.iter().any(|f| f == "SPECIAL_GENE") {
            continue;
        }
        if a.depth > depth {
            let d = (a.depth - depth) as i32 * 2;
            if rng.gen_range(0..d.max(1)) != 0 {
                continue;
            }
        }
        let rar = a.rarity.max(1) as i32;
        let luck = rng.gen_range(-(rar / 2)..=(rar / 2));
        if rng.gen_range(0..(rar - luck).max(1)) != 0 {
            continue;
        }
        item.artifact = a.id;
        created.insert(a.id);
        item.dice = a.dice.clone();
        item.ac = a.ac;
        item.to_h = a.to_h;
        item.to_d = a.to_d;
        item.to_a = a.to_a;
        item.pval = a.pval;
        if a.weight >= 0 {
            item.weight = a.weight;
        }
        item.cursed = a.cursed;
        item.identified = false;
        // RANDOM_RESIST / RANDOM_POWER / RANDOM_RES_OR_POWER artifacts
        // roll their extra powers at creation (src/object2.cc).
        artifact_random_flags(gd, item, rng);
        init_levels_artifact(gd, o, item);
        return true;
    }
    false
}

/// Shared artifact init for TR_LEVELS artifacts (object2.cc:3907
/// `init_obj_exp`): elevel from the base kind's level and one realm point.
pub(crate) fn init_levels_artifact(gd: &GameData, o: &crate::data::ObjectDef, item: &mut Item) {
    let levels = o.flags.iter().any(|f| f == "LEVELS")
        || gd
            .artifacts
            .iter()
            .find(|a| a.id == item.artifact)
            .is_some_and(|a| a.flags.iter().any(|f| f == "LEVELS"));
    if levels {
        if item.elevel <= 1 {
            item.elevel = (o.depth as i32 / 10 + 1).max(1);
        }
        item.mon_exp = 0;
        if item.pval2 <= 0 {
            item.pval2 = 1;
        }
    }
}

/// Turn an existing base item into the specific artifact (cmd6.cc
/// activate_eternal_flame: o_ptr->name1 = artifact_idx + apply_magic).
pub fn imbue_artifact(gd: &GameData, item: &mut Item, id: u32) -> bool {
    let Some(a) = gd.artifacts.iter().find(|a| a.id == id) else {
        return false;
    };
    let o = gd.objects[item.def].clone();
    item.artifact = a.id;
    item.artifact_name = String::new();
    item.dice = a.dice.clone();
    item.ac = a.ac;
    item.to_h = a.to_h;
    item.to_d = a.to_d;
    item.to_a = a.to_a;
    item.pval = a.pval;
    if a.weight >= 0 {
        item.weight = a.weight;
    }
    item.cursed = a.cursed;
    item.identified = true;
    init_levels_artifact(gd, &o, item);
    true
}

/// Create a specific a_info artifact by id (dungeon FINAL_ARTIFACT and
/// FINAL_GUARDIAN loot), registering it in `created`.
pub fn make_specific_artifact(
    gd: &GameData,
    artifact_id: u32,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<Item> {
    if created.contains(&artifact_id) {
        return None;
    }
    let item = specific_artifact_item(gd, artifact_id, rng)?;
    created.insert(artifact_id);
    Some(item)
}

/// Build a specific a_info artifact instance (no uniqueness bookkeeping;
/// used when a guardian's held loot is dropped).
pub fn specific_artifact_item(gd: &GameData, artifact_id: u32, rng: &mut impl Rng) -> Option<Item> {
    let a = gd.artifacts.iter().find(|a| a.id == artifact_id)?;
    let def = gd.object_by_tval_sval(a.tval, a.sval)?;
    let mut item = Item::base(gd, def);
    item.artifact = a.id;
    item.dice = a.dice.clone();
    item.ac = a.ac;
    item.to_h = a.to_h;
    item.to_d = a.to_d;
    item.to_a = a.to_a;
    item.pval = a.pval;
    if a.weight >= 0 {
        item.weight = a.weight;
    }
    item.cursed = a.cursed;
    item.identified = false;
    artifact_random_flags(gd, &mut item, rng);
    let o = gd.objects[item.def].clone();
    init_levels_artifact(gd, &o, &mut item);
    Some(item)
}

/// Create a fully-rolled object instance. `good` forces the ego/artifact
/// rolls (Black Market, quest rewards). `created` tracks artifact
/// uniqueness across the whole game.
/// Ammo creation (cmd7.cc do_cmd_archer): a stack of 15-30 pieces of the
/// best ammo kind the current depth offers.
pub fn forge_ammo(gd: &GameData, tval: i32, depth: u32, rng: &mut impl Rng) -> Item {
    let mut cands: Vec<usize> = gd
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.tval == tval && o.depth > 0 && o.depth <= depth)
        .map(|(i, _)| i)
        .collect();
    cands.sort_by_key(|i| gd.objects[*i].depth);
    let def = cands
        .last()
        .copied()
        .or_else(|| gd.object_by_tval_sval(tval, 1))
        .unwrap_or(0);
    let mut item = Item::base(gd, def);
    item.count = rng.gen_range(15..=30);
    item.identified = true;
    item
}

/// Create a fully-rolled object instance (apply_magic with `okay=true`,
/// `great=false`, no forced power).
pub fn make_item(
    gd: &GameData,
    def: usize,
    depth: u32,
    good: bool,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Item {
    apply_magic(gd, def, depth, true, good, false, None, created, rng)
}

/// The randart chances of the ToME module (tables.cc modules[]:
/// weapon 30, armor 20, jewelry 20).
const RANDART_WEAPON: i32 = 30;
const RANDART_ARMOR: i32 = 20;
const RANDART_JEWEL: i32 = 20;

/// apply_magic (object2.cc:3724): the full object-generation pipeline.
/// `okay` allows the artifact rolls (store stock and some quest paths pass
/// false), `good`/`great` force the power roll (make_object), and
/// `force_power` overrides it outright (wizard2.cc and the store "cursed"
/// hack).
#[allow(clippy::too_many_arguments)]
pub fn apply_magic(
    gd: &GameData,
    def: usize,
    depth: u32,
    okay: bool,
    good: bool,
    great: bool,
    force_power: Option<i32>,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Item {
    let mut item = Item::base(gd, def);
    let o = &gd.objects[def];
    // TR_NORM_ART base kinds are unique pseudo-artifacts: the first
    // creation claims the kind, later ones fall back to the T: kind
    // (object2.cc apply_magic 3745-3788).
    if o.flags.iter().any(|f| f == "NORM_ART") {
        if created.contains(&norm_art_key(def)) {
            if let Some(fb) = gd.object_by_tval_sval(o.btval, o.bsval) {
                return Item::base(gd, fb);
            }
        } else {
            created.insert(norm_art_key(def));
            if matches!(o.tval, data::TV_WAND | data::TV_STAFF) && !o.spell.is_empty() {
                roll_stick(gd, &mut item, def, depth, rng);
            }
            if o.flags.iter().any(|f| f == "LEVELS") {
                item.elevel = o.depth as i32 / 10 + 1;
            }
            return item;
        }
    }

    // Apply luck, then cap the level (apply_magic:3730/3793).
    let mut lev = depth as i32 + rng.gen_range(-7..=7);
    if lev > MAX_DEPTH - 1 {
        lev = MAX_DEPTH - 1;
    }
    // A spell container holds no spell of its own yet.
    if o.flags.iter().any(|f| f == "SPELL_CONTAIN") {
        item.pval2 = -1;
    }

    // Roll the power (apply_magic:3795-3825): f1 = lev + 10 + luck(-15,15)
    // capped at 75, f2 = f1/2 capped at 20; good/great force good/great.
    let mut power = 0;
    {
        let mut f1 = lev + 10 + rng.gen_range(-15..=15);
        if f1 > 75 {
            f1 = 75;
        }
        let mut f2 = f1 / 2;
        if f2 > 20 {
            f2 = 20;
        }
        if good || rng.gen_range(1..=100) <= f1 {
            power = 1;
            if great || rng.gen_range(1..=100) <= f2 {
                power = 2;
            }
        } else if rng.gen_range(1..=100) <= f1 {
            power = -1;
            if rng.gen_range(1..=100) <= f2 {
                power = -2;
            }
        }
        if let Some(p) = force_power {
            power = p;
        }
    }

    // Artifact rolls: one at power 2, four when `great` forces it
    // (apply_magic:3827-3842).
    let mut rolls = 0;
    if power >= 2 {
        rolls = 1;
    }
    if great {
        rolls = 4;
    }
    if !okay || item.artifact != 0 {
        rolls = 0;
    }
    for _ in 0..rolls {
        if apply_artifact(gd, &mut item, created, rng, depth) {
            break;
        }
    }
    if item.artifact != 0 {
        // Analyze artifacts (apply_magic:3851-3895): the stats were
        // copied by apply_artifact; plural artifacts collapse to one.
        item.count = 1;
        let spell_contain = item_flags(gd, &item).iter().any(|f| *f == "SPELL_CONTAIN");
        let levels = item_flags(gd, &item).iter().any(|f| *f == "LEVELS");
        if spell_contain {
            item.pval2 = -1;
        }
        if levels {
            init_levels_artifact(gd, o, &mut item);
        }
        finish_rod_main(gd, o, &mut item);
        item
    } else {
        // The per-tval magic (apply_magic:3897-3991).
        match o.tval {
            data::TV_RANDART => {
                finalize_junkart(gd, &mut item, created, rng);
            }
            data::TV_HAFTED
            | data::TV_POLEARM
            | data::TV_MSTAFF
            | data::TV_SWORD
            | data::TV_AXE
            | data::TV_BOOMERANG
            | data::TV_BOW
            | data::TV_SHOT
            | data::TV_ARROW
            | data::TV_BOLT => a_m_aux_1(gd, &mut item, lev, power, depth, rng),
            // Demon books are either weapons (Demonblade, SV 55) or armour.
            data::TV_DAEMON_BOOK if o.sval == 55 => {
                a_m_aux_1(gd, &mut item, lev, power, depth, rng)
            }
            data::TV_DAEMON_BOOK
            | data::TV_DRAG_ARMOR
            | data::TV_HARD_ARMOR
            | data::TV_SOFT_ARMOR
            | data::TV_SHIELD
            | data::TV_HELM
            | data::TV_CROWN
            | data::TV_CLOAK
            | data::TV_GLOVES
            | data::TV_BOOTS => a_m_aux_2(gd, &mut item, lev, power, depth, rng),
            data::TV_RING | data::TV_AMULET => {
                if power == 0 && rng.gen_range(0..100) < 50 {
                    power = -1;
                }
                a_m_aux_3(gd, &mut item, lev, power, depth, rng);
            }
            _ => a_m_aux_4(gd, &mut item, lev, power, depth, rng),
        }

        // The tail (apply_magic:4025-4098): base-kind curses, LEVELS
        // init, spell holders, mage staffs and rod mains.
        if o.flags.iter().any(|f| f == "CURSED") {
            item.cursed = true;
        }
        let levels = item_flags(gd, &item).iter().any(|f| *f == "LEVELS");
        let spell_contain = item_flags(gd, &item).iter().any(|f| *f == "SPELL_CONTAIN");
        if levels {
            init_levels_artifact(gd, o, &mut item);
        }
        if spell_contain {
            item.pval2 = -1;
        }
        if o.tval == data::TV_MSTAFF && o.sval == 1 && item.pval < 0 {
            item.pval = 0;
        }
        finish_rod_main(gd, o, &mut item);
        item
    }
}

/// The apply_magic tail for rod mains: the Capacity ego doubles the mana
/// store, then the rod starts full (object2.cc:4087-4096).
fn finish_rod_main(gd: &GameData, o: &crate::data::ObjectDef, item: &mut Item) {
    if o.tval != data::TV_ROD_MAIN {
        return;
    }
    let cap = o.sval.max(1);
    let capacity = item_flags(gd, item).iter().any(|f| *f == "CAPACITY");
    item.pval2 = if capacity { cap * 2 } else { cap };
    item.timeout = item.pval2;
}

/// a_m_aux_1 (object2.cc:2350): weapons, bows, ammo and mage staffs.
#[allow(clippy::too_many_arguments)]
fn a_m_aux_1(
    gd: &GameData,
    item: &mut Item,
    lev: i32,
    power: i32,
    depth: u32,
    rng: &mut impl Rng,
) {
    let tohit1 = rng.gen_range(1..=5) + m_bonus(5, lev, rng);
    let todam1 = rng.gen_range(1..=5) + m_bonus(5, lev, rng);
    let tohit2 = m_bonus(10, lev, rng);
    let todam2 = m_bonus(10, lev, rng);

    // Very good / very cursed: an ego (or rarely a randart).
    if power > 1 {
        if rng.gen_range(0..RANDART_WEAPON) == 1 {
            create_artifact(gd, item, depth, false, "", rng);
        } else {
            apply_ego(gd, item, depth, true, rng);
        }
    } else if power < -1 {
        apply_ego(gd, item, depth, false, rng);
    }

    if power > 0 {
        item.to_h += tohit1;
        item.to_d += todam1;
        if power > 1 {
            item.to_h += tohit2;
            item.to_d += todam2;
        }
    } else if power < 0 {
        item.to_h -= tohit1;
        item.to_d -= todam1;
        if power < -1 {
            item.to_h -= tohit2;
            item.to_d -= todam2;
        }
        if item.to_h + item.to_d < 0 {
            item.cursed = true;
            add_item_flag(item, "CURSED");
        }
    }

    match gd.objects[item.def].tval {
        // Mage staffs are wielded spell containers (object2.cc:2414).
        data::TV_MSTAFF => {
            for f in ["SPELL_CONTAIN", "WIELD_CAST"] {
                if !item.flags.iter().any(|x| x == f) {
                    item.flags.push(f.to_string());
                }
            }
        }
        // Exploding missiles: pval2 indexes the 27 GF_* list.
        data::TV_BOLT | data::TV_ARROW | data::TV_SHOT => {
            if power == 1 && item.ego == 0 && item.artifact == 0 && rng.gen_range(1..=100) < 30 {
                item.pval2 = rng.gen_range(1..=MISSILE_GFS.len() as i32);
            }
        }
        _ => {}
    }
}

/// a_m_aux_2 (object2.cc:2468): body armour, shields and the like.
#[allow(clippy::too_many_arguments)]
fn a_m_aux_2(
    gd: &GameData,
    item: &mut Item,
    lev: i32,
    power: i32,
    depth: u32,
    rng: &mut impl Rng,
) {
    let toac1 = rng.gen_range(1..=5) + m_bonus(5, lev, rng);
    let toac2 = m_bonus(10, lev, rng);

    if power > 1 {
        if rng.gen_range(0..RANDART_ARMOR) == 1 {
            create_artifact(gd, item, depth, false, "", rng);
        } else {
            apply_ego(gd, item, depth, true, rng);
        }
    } else if power < -1 {
        apply_ego(gd, item, depth, false, rng);
    }

    if power > 0 {
        item.to_a += toac1;
        if power > 1 {
            item.to_a += toac2;
        }
    } else if power < 0 {
        item.to_a -= toac1;
        if power < -1 {
            item.to_a -= toac2;
        }
        if item.to_a < 0 {
            item.cursed = true;
            add_item_flag(item, "CURSED");
        }
    }

    let o = gd.objects[item.def].clone();
    armour_magic(item, o.tval, o.sval, lev, rng);
}

/// a_m_aux_3 (object2.cc:2597): rings and amulets.
#[allow(clippy::too_many_arguments)]
fn a_m_aux_3(
    gd: &GameData,
    item: &mut Item,
    lev: i32,
    power: i32,
    depth: u32,
    rng: &mut impl Rng,
) {
    if power > 1 {
        if rng.gen_range(0..RANDART_JEWEL) == 1 {
            create_artifact(gd, item, depth, false, "", rng);
        } else {
            apply_ego(gd, item, depth, true, rng);
        }
    } else if power < -1 {
        apply_ego(gd, item, depth, false, rng);
    }
    jewelry_magic(gd, item, power, lev, rng);
}

/// a_m_aux_4 (object2.cc:3028): everything else.  The generation powers
/// call for an ego/randart first, then the tval special cases.
#[allow(clippy::too_many_arguments)]
fn a_m_aux_4(
    gd: &GameData,
    item: &mut Item,
    lev: i32,
    power: i32,
    depth: u32,
    rng: &mut impl Rng,
) {
    if power > 1 {
        // Only lites use the randart path; every other kind rolls the
        // die first and then makes an ego anyway (object2.cc:3036).
        let randart = rng.gen_range(0..RANDART_JEWEL) == 1;
        if randart && gd.objects[item.def].tval == data::TV_LITE {
            create_artifact(gd, item, depth, false, "", rng);
        } else {
            apply_ego(gd, item, depth, true, rng);
        }
    } else if power < -1 {
        apply_ego(gd, item, depth, false, rng);
    }

    let o = gd.objects[item.def].clone();
    match o.tval {
        // Random spellbooks hold one Magic (75%) / Spirituality spell.
        data::TV_BOOK => {
            if o.sval == 255 {
                if let Some(name) = random_book_spell(gd, lev.max(0) as u32, rng) {
                    if let Some(sp) = gd.spell_by_name(&name) {
                        item.pval = gd
                            .spells
                            .iter()
                            .position(|s| s.name == sp.name)
                            .unwrap_or(0) as i32;
                    }
                    item.spells.push(name);
                }
            }
        }
        // FUEL_LITE lites start with a random amount of fuel.
        data::TV_LITE => {
            if o.flags.iter().any(|f| f == "FUEL_LITE") && o.fuel > 0 {
                item.fuel = rng.gen_range(1..=o.fuel);
            }
        }
        data::TV_CORPSE => {
            let cands: Vec<usize> = (0..gd.monster_base_count).collect();
            if let Some(idx) = crate::game::get_mon_num_from(gd, depth, &cands, rng) {
                item.note = if gd.monsters[idx].unique {
                    2
                } else {
                    idx as u32
                };
                item.pval3 = 0;
            }
        }
        data::TV_EGG => {
            let cands: Vec<usize> = (0..gd.monster_base_count)
                .filter(|&i| gd.monsters[i].has("HAS_EGG"))
                .collect();
            // C++ retries get_mon_num 1000 times and falls back to r_info
            // 940 (Blue fire-lizard) when no egg layer fits the depth.
            let idx = crate::game::get_mon_num_from(gd, depth, &cands, rng)
                .or_else(|| gd.monster_by_id(940));
            if let Some(idx) = idx {
                item.note = idx as u32;
                let w = gd.monsters[idx].weight.max(1) as i32;
                item.pval = w * 3 + rng.gen_range(0..w) + 1;
                item.pval2 = w + rng.gen_range(0..w) / 10 + 1;
                item.weight = item.pval2;
            }
        }
        data::TV_HYPNOS => {
            let cands: Vec<usize> = (0..gd.monster_base_count).collect();
            if let Some(mut idx) = crate::game::get_mon_num_from(gd, depth, &cands, rng) {
                // NEVER_MOVE monsters cannot be symbiotes; the original
                // falls back to r_info 20 (object2.cc:3151).
                if gd.monsters[idx].has("NEVER_MOVE") {
                    if let Some(fallback) = gd.monster_by_id(20) {
                        idx = fallback;
                    }
                }
                item.note = idx as u32;
                let hd = gd.monsters[idx].hdice.parse::<i32>().unwrap_or(1);
                let hs = gd.monsters[idx].hside.parse::<i32>().unwrap_or(1);
                item.pval3 = hd * hs;
                item.mon_exp = 0;
                item.elevel = gd.monsters[idx].depth as i32;
            }
        }
        data::TV_WAND | data::TV_STAFF => roll_stick(gd, item, item.def, depth, rng),
        data::TV_POTION2 => {
            if o.sval == 1 {
                item.pval2 = crate::mimic::find_random_mimic_shape(lev, false, rng) as i32;
            }
        }
        // Horns of Dragonkind get their breath element in pval2.
        data::TV_INSTRUMENT => {
            if o.sval == 60 && (item.ego == 130 || item.ego2 == 130) {
                // GF_ELEC, GF_FIRE, GF_COLD, GF_ACID (defines.hpp).
                item.pval2 = [1, 5, 4, 3][rng.gen_range(0..4)];
            }
        }
        _ => {}
    }
}

/// The 27 explosive missile GF types (a_m_aux_1); pval2 = index + 1.
pub const MISSILE_GFS: [&str; 27] = [
    "ELEC",
    "POIS",
    "ACID",
    "COLD",
    "FIRE",
    "PLASMA",
    "LITE",
    "DARK",
    "SHARDS",
    "SOUND",
    "CONFUSION",
    "FORCE",
    "INERTIA",
    "MANA",
    "METEOR",
    "ICE",
    "CHAOS",
    "NETHER",
    "NEXUS",
    "TIME",
    "GRAVITY",
    "KILL_WALL",
    "AWAY_ALL",
    "TURN_ALL",
    "NUKE",
    "STUN",
    "DISINTEGRATE",
];

/// make_object (object2.cc): the "special artifact" roll comes first, then
/// a normal object of the depth's allocation.  Only dungeon generation
/// (floor, treasure, q_rand rewards) uses this; towns and shop stock roll
/// ordinary objects, and specific placements (FINAL_ARTIFACT, map
/// artifacts) go through `make_specific_artifact`.
pub fn make_object(
    gd: &GameData,
    depth: u32,
    good: bool,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<Item> {
    make_object_ex(gd, depth, good, false, created, rng)
}

/// make_object with `great` (object2.cc apply_magic power 2): the good
/// allocation and artifact rolls are boosted and an ego is forced.
pub fn make_object_ex(
    gd: &GameData,
    depth: u32,
    good: bool,
    great: bool,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<Item> {
    make_object_themed(gd, depth, good, great, (0, 0, 0, 0), created, rng)
}

/// make_object with a drop theme (monster r_info O: / dungeon d_info O:).
pub fn make_object_themed(
    gd: &GameData,
    depth: u32,
    good: bool,
    great: bool,
    theme: (u32, u32, u32, u32),
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<Item> {
    // object2.cc: invprob = good ? 10 - luck(-9,9) : 1000; no town rolls.
    let invprob = if good { 10 } else { 1000 };
    // Base level for the object (good items are rolled 10 levels deeper).
    let base = if good { depth + 10 } else { depth };
    let mut item = None;
    if depth > 0 && rng.gen_range(0..invprob) == 0 {
        item = make_artifact_special(gd, base, created, rng);
    }
    if item.is_none() {
        let def = gen_object_kind_themed(gd, base, Some(theme), good, Some(created), rng)?;
        // apply_magic uses the real object level, not the good +10 base;
        // `great` forces power 2 (artifact/ego rolls).
        item = Some(apply_magic(
            gd, def, depth, true, good, great, None, created, rng,
        ));
    }
    let mut item = item.expect("item");
    // Hack -- generate multiple spikes/missiles (object2.cc:4560);
    // artifacts never stack.
    if matches!(
        gd.objects[item.def].tval,
        data::TV_SPIKE | data::TV_SHOT | data::TV_ARROW | data::TV_BOLT
    ) {
        item.count = damroll(6, 7, rng) as u32;
    }
    if is_artifact(gd, &item) {
        item.count = 1;
    }
    Some(item)
}

/// The generate.cc `rating`/`good_item_flag` contribution of a freshly
/// made object (object2.cc apply_magic): real artifacts +10 (+10 when
/// cost > 50000) and the good-item flag, named randarts +40, egos their
/// X: rating, dragon armours, dragon shields/helms and Blood of Life
/// fixed bonuses, and the out-of-depth bonus for uncursed objects.
pub fn rating_of(gd: &GameData, item: &Item, depth: u32) -> (i32, bool) {
    let mut rating = 0;
    let mut good_item = false;
    if item.artifact != 0 {
        rating += 10;
        if gd
            .artifacts
            .iter()
            .find(|a| a.id == item.artifact)
            .is_some_and(|a| a.cost > 50000)
        {
            rating += 10;
        }
        good_item = true;
    } else if !item.artifact_name.is_empty() {
        rating += 40;
    }
    for id in [item.ego, item.ego2] {
        if let Some(e) = gd.egos.iter().find(|e| e.id == id) {
            rating += e.rating;
        }
    }
    let o = &gd.objects[item.def];
    match (o.tval, o.sval) {
        (data::TV_DRAG_ARMOR, _) => rating += 30,
        (data::TV_SHIELD, 6) | (data::TV_HELM, 7) => rating += 5,
        (data::TV_POTION, 3) => rating += 25, // SV_POTION_BLOOD
        (data::TV_RING, 31) => rating += 25,  // SV_RING_SPEED
        (data::TV_RING, 48) => rating += 5,   // SV_RING_LORDLY
        (data::TV_AMULET, 8) => rating += 25, // SV_AMULET_THE_MAGI
        _ => {}
    }
    if !item.cursed && o.depth > depth {
        rating += (o.depth - depth) as i32;
    }
    (rating, good_item)
}

/// make_artifact_special (object2.cc:2037): roll through the a_info list
/// for an uncreated INSTA_ART artifact.  SPECIAL_GENE artifacts are only
/// reachable through a_allow_special (final guardians, quests, fixed map
/// placements) and never appear here; The One Ring is quest-only.
fn make_artifact_special(
    gd: &GameData,
    depth: u32,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> Option<Item> {
    for a in &gd.artifacts {
        if !a.insta_art || created.contains(&a.id) {
            continue;
        }
        // The One Ring only exists once the quest has spawned it
        // (object2.cc ART_POWER hack).
        if a.id == 13 || a.flags.iter().any(|f| f == "SPECIAL_GENE") {
            continue;
        }
        // Out-of-depth roll and rarity roll, in list order.
        if a.depth > depth {
            let d = (a.depth - depth) * 2;
            if rng.gen_range(0..d.max(1)) != 0 {
                continue;
            }
        }
        let rar = a.rarity.max(1) as i32;
        let luck = rng.gen_range(-(rar / 2)..=(rar / 2));
        if rng.gen_range(0..(rar - luck).max(1)) != 0 {
            continue;
        }
        // The base kind must be within reach as well.
        let Some(def) = gd.object_by_tval_sval(a.tval, a.sval) else {
            continue;
        };
        let klevel = gd.objects[def].depth;
        if klevel > depth {
            let d = (klevel - depth) * 5;
            if rng.gen_range(0..d.max(1)) != 0 {
                continue;
            }
        }
        let item = specific_artifact_item(gd, a.id, rng)?;
        created.insert(a.id);
        return Some(item);
    }
    None
}

/// The Artifact Creation scroll's item filter (item_tester_hook_artifactable,
/// src/spells2.cc): weapons, armour, digging tools, rings and amulets.
pub fn artifactable(tval: i32) -> bool {
    matches!(
        tval,
        data::TV_DIGGING
            | data::TV_HAFTED
            | data::TV_POLEARM
            | data::TV_SWORD
            | data::TV_AXE
            | data::TV_RING
            | data::TV_AMULET
    ) || (data::TV_BOOTS..=data::TV_DRAG_ARMOR).contains(&tval)
}

/// randnor (z-rand.cc:154) now lives in `crate::rng`; keep the local
/// name for the many existing call sites.
fn randnor(mean: i32, stand: i32, rng: &mut impl Rng) -> i32 {
    crate::rng::randnor(mean, stand, rng)
}

/// Original m_bonus (src/object2.cc): a normally-distributed bonus scaled
/// by the dungeon level, clamped to 0..=max.
pub fn m_bonus(max: i32, level: i32, rng: &mut impl Rng) -> i32 {
    const MAX_DEPTH: i32 = 128;
    let level = level.clamp(0, MAX_DEPTH - 1);
    let mut bonus = (max * level) / MAX_DEPTH;
    if rng.gen_range(0..MAX_DEPTH) < (max * level) % MAX_DEPTH {
        bonus += 1;
    }
    let mut stand = max / 4;
    if rng.gen_range(0..4) < max % 4 {
        stand += 1;
    }
    randnor(bonus, stand, rng).clamp(0, max)
}

/// randomized_level_in_range (object2.cc:2978): a stick level rolled in
/// [min, max], biased by the dungeon level.
fn randomized_level(min: i32, max: i32, level: i32, rng: &mut impl Rng) -> i32 {
    let mut r = max - min;
    if r * 2 > level {
        r = level / 2;
    }
    min + m_bonus(r.max(0), level, rng)
}

/// get_random_stick (spells5.cc:80): a random device spell for a
/// "Spell" wand/staff, gated by spell level and allocation rarity.
fn random_stick_spell(gd: &GameData, tval: i32, level: u32, rng: &mut impl Rng) -> Option<String> {
    for _ in 0..1000 {
        let row = &gd.spells[rng.gen_range(0..gd.spells.len())];
        let Some(&(_, rarity, _, _, _, _)) = row.alloc.iter().find(|a| a.0 == tval) else {
            continue;
        };
        if rng.gen_range(0..(row.level as i32 * 3).max(1)) < level as i32
            && rng.gen_range(0..100) < 100 - rarity
        {
            return Some(row.name.clone());
        }
    }
    None
}

/// get_random_spell (spells5.cc): a random spell of the Magic (75%) or
/// Spirituality school whose level fits the dungeon depth.
fn random_book_spell(gd: &GameData, level: u32, rng: &mut impl Rng) -> Option<String> {
    let wanted = if rng.gen_range(0..100) >= 75 {
        crate::skill::SK_SPIRITUALITY
    } else {
        crate::skill::SK_MAGIC
    };
    crate::spell::random_spell(gd, wanted, level as i32, rng).map(|r| r.name.clone())
}

/// a_m_aux_4 (object2.cc:3168-3228): a wand/staff's spell (random for
/// the "Spell" kinds), its stick level range in pval3 and its charges
/// rolled from the spell's device table.
fn roll_stick(gd: &GameData, item: &mut Item, def: usize, depth: u32, rng: &mut impl Rng) {
    let o = &gd.objects[def];
    let spell_name = if o.spell.is_empty() {
        let rolled = random_stick_spell(gd, o.tval, depth, rng);
        item.stick_spell = rolled.clone().unwrap_or_default();
        rolled.unwrap_or_else(|| o.name.clone())
    } else {
        o.spell.clone()
    };
    let row = gd.spell_by_name(&spell_name);
    let alloc = row.and_then(|r| r.alloc.iter().find(|a| a.0 == o.tval).copied());
    if let Some((_, _, bmin, bmax, mmin, mmax)) = alloc {
        let bonus = randomized_level(bmin, bmax, depth as i32, rng);
        let maxl = randomized_level(mmin, mmax, depth as i32, rng);
        item.pval3 = ((maxl & 0xFFFF) << 16) | (bonus & 0xFFFF);
    }
    let charges = row.map(|r| r.charges.as_str()).unwrap_or("");
    if !charges.is_empty() {
        // Device charges are "N+dM" = base + damroll(1, M) = base +
        // randint(M) (dice.cc dice_roll), not the XdY item-dice format.
        item.charges = if let Some((base, die)) = charges.split_once("+d") {
            let base: i32 = base.parse().unwrap_or(0);
            let die: i32 = die.parse().unwrap_or(0);
            base + rng.gen_range(1..=die.max(1))
        } else {
            Dice::parse(charges).map(|d| d.roll(rng)).unwrap_or(0)
        }
        .max(0);
    } else {
        // Fallback for kinds without a device table entry.
        item.charges = if o.tval == data::TV_WAND {
            Dice {
                count: 2,
                sides: 6,
                bonus: 2,
            }
            .roll(rng)
        } else {
            Dice {
                count: 2,
                sides: 8,
                bonus: 3,
            }
            .roll(rng)
        };
    }
}

/// artifact_bias (variable.cc): the resistance family the last roll
/// showed, which biases later free random_resistance rolls within one
/// artifact's creation.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ArtifactBias {
    #[default]
    None,
    Acid,
    Elec,
    Fire,
    Cold,
    Pois,
    Warrior,
    Necromantic,
    Chaos,
    Rogue,
}

pub(crate) fn add_item_flag(item: &mut Item, f: &str) {
    if !item.flags.iter().any(|x| x == f) {
        item.flags.push(f.to_string());
    }
}

/// One random_resistance roll (src/spells2.cc:1768): the full 41-case
/// resist table, plus the artifact bias pre-roll. `specific` 0 = free
/// roll; ids 1..4 are immunities (re-rolled 11/12 of the time), 5..38
/// the resist table, 39/40 the armour elemental shields and 41
/// reflection. Returns true when a new flag was added.
fn random_resistance(
    item: &mut Item,
    tval: i32,
    specific: i32,
    bias: &mut ArtifactBias,
    rng: &mut impl Rng,
) -> bool {
    let before = item.flags.len();
    // Bias chain: only free rolls get the pre-roll (C++ `if (!specific)`).
    if specific == 0 && *bias != ArtifactBias::None {
        let armor = (data::TV_CLOAK..=data::TV_HARD_ARMOR).contains(&tval);
        match *bias {
            ArtifactBias::Acid => {
                if !item.flags.iter().any(|x| x == "RES_ACID") {
                    add_item_flag(item, "RES_ACID");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if rng.gen_range(0..20) == 0 && !item.flags.iter().any(|x| x == "IM_ACID") {
                    add_item_flag(item, "IM_ACID");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
            }
            ArtifactBias::Elec => {
                if !item.flags.iter().any(|x| x == "RES_ELEC") {
                    add_item_flag(item, "RES_ELEC");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if armor && !item.flags.iter().any(|x| x == "SH_ELEC") {
                    add_item_flag(item, "SH_ELEC");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if rng.gen_range(0..20) == 0 && !item.flags.iter().any(|x| x == "IM_ELEC") {
                    add_item_flag(item, "IM_ELEC");
                    if rng.gen_range(0..2) == 1 {
                        return true;
                    }
                }
            }
            ArtifactBias::Fire => {
                if !item.flags.iter().any(|x| x == "RES_FIRE") {
                    add_item_flag(item, "RES_FIRE");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if armor && !item.flags.iter().any(|x| x == "SH_FIRE") {
                    add_item_flag(item, "SH_FIRE");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if rng.gen_range(0..20) == 0 && !item.flags.iter().any(|x| x == "IM_FIRE") {
                    add_item_flag(item, "IM_FIRE");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
            }
            ArtifactBias::Cold => {
                if !item.flags.iter().any(|x| x == "RES_COLD") {
                    add_item_flag(item, "RES_COLD");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if rng.gen_range(0..20) == 0 && !item.flags.iter().any(|x| x == "IM_COLD") {
                    add_item_flag(item, "IM_COLD");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
            }
            ArtifactBias::Pois => {
                if !item.flags.iter().any(|x| x == "RES_POIS") {
                    add_item_flag(item, "RES_POIS");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
            }
            ArtifactBias::Warrior => {
                if rng.gen_range(0..3) != 0 && !item.flags.iter().any(|x| x == "RES_FEAR") {
                    add_item_flag(item, "RES_FEAR");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
                if rng.gen_range(0..3) == 0 && !item.flags.iter().any(|x| x == "NO_MAGIC") {
                    add_item_flag(item, "NO_MAGIC");
                    if rng.gen_range(0..2) == 0 {
                        return true;
                    }
                }
            }
            ArtifactBias::Necromantic => {
                for f in ["RES_NETHER", "RES_POIS", "RES_DARK"] {
                    if !item.flags.iter().any(|x| x == f) {
                        add_item_flag(item, f);
                        if rng.gen_range(0..2) == 0 {
                            return true;
                        }
                    }
                }
            }
            ArtifactBias::Chaos => {
                for f in ["RES_CHAOS", "RES_CONF", "RES_DISEN"] {
                    if !item.flags.iter().any(|x| x == f) {
                        add_item_flag(item, f);
                        if rng.gen_range(0..2) == 0 {
                            return true;
                        }
                    }
                }
            }
            ArtifactBias::Rogue => {}
            ArtifactBias::None => {}
        }
    }
    let roll = if specific > 0 {
        specific
    } else {
        rng.gen_range(1..=41)
    };
    let armor = (data::TV_CLOAK..=data::TV_HARD_ARMOR).contains(&tval);
    let flag: &'static str = match roll {
        1..=4 => {
            // Immunities are barely ever granted; otherwise roll again.
            if rng.gen_range(1..=12) != 1 {
                return random_resistance(item, tval, specific, bias, rng);
            }
            let im = ["IM_ACID", "IM_ELEC", "IM_COLD", "IM_FIRE"][(roll - 1) as usize];
            if *bias == ArtifactBias::None {
                *bias = [
                    ArtifactBias::Acid,
                    ArtifactBias::Elec,
                    ArtifactBias::Cold,
                    ArtifactBias::Fire,
                ][(roll - 1) as usize];
            }
            im
        }
        5 | 6 | 13 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Acid;
            }
            "RES_ACID"
        }
        7 | 8 | 14 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Elec;
            }
            "RES_ELEC"
        }
        9 | 10 | 15 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Fire;
            }
            "RES_FIRE"
        }
        11 | 12 | 16 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Cold;
            }
            "RES_COLD"
        }
        17 | 18 => {
            // Poison biases towards poison, necromancy or roguery.
            if *bias == ArtifactBias::None {
                if rng.gen_range(1..=4) != 1 {
                    *bias = ArtifactBias::Pois;
                } else if rng.gen_range(1..=2) == 1 {
                    *bias = ArtifactBias::Necromantic;
                } else if rng.gen_range(1..=2) == 1 {
                    *bias = ArtifactBias::Rogue;
                }
            }
            "RES_POIS"
        }
        19 | 20 => {
            if *bias == ArtifactBias::None && rng.gen_range(1..=3) == 1 {
                *bias = ArtifactBias::Warrior;
            }
            "RES_FEAR"
        }
        21 => "RES_LITE",
        22 => "RES_DARK",
        23 | 24 => "RES_BLIND",
        25 | 26 => {
            if *bias == ArtifactBias::None && rng.gen_range(1..=6) == 1 {
                *bias = ArtifactBias::Chaos;
            }
            "RES_CONF"
        }
        27 | 28 => "RES_SOUND",
        29 | 30 => "RES_SHARDS",
        31 | 32 => {
            if *bias == ArtifactBias::None && rng.gen_range(1..=3) == 1 {
                *bias = ArtifactBias::Necromantic;
            }
            "RES_NETHER"
        }
        33 | 34 => "RES_NEXUS",
        35 | 36 => {
            if *bias == ArtifactBias::None && rng.gen_range(0..2) == 1 {
                *bias = ArtifactBias::Chaos;
            }
            "RES_CHAOS"
        }
        37 | 38 => "RES_DISEN",
        39 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Elec;
            }
            if armor {
                "SH_ELEC"
            } else if specific > 0 {
                // Guard the original's self-recursion (only a caller
                // passing 39 directly could hit it).
                return random_resistance(item, tval, rng.gen_range(5..=38), bias, rng);
            } else {
                return random_resistance(item, tval, 0, bias, rng);
            }
        }
        40 => {
            if *bias == ArtifactBias::None {
                *bias = ArtifactBias::Fire;
            }
            if armor {
                "SH_FIRE"
            } else if specific > 0 {
                return random_resistance(item, tval, rng.gen_range(5..=38), bias, rng);
            } else {
                return random_resistance(item, tval, 0, bias, rng);
            }
        }
        41 => {
            if matches!(
                tval,
                data::TV_SHIELD | data::TV_CLOAK | data::TV_HELM | data::TV_HARD_ARMOR
            ) {
                "REFLECT"
            } else if specific > 0 {
                return random_resistance(item, tval, rng.gen_range(5..=38), bias, rng);
            } else {
                return random_resistance(item, tval, 0, bias, rng);
            }
        }
        _ => return false,
    };
    add_item_flag(item, flag);
    item.flags.len() != before
}

/// random_resistance(o_ptr, specific) (src/spells2.cc) without a tracked
/// bias: used by the R_* ego powers and dragon resistances.
fn random_resistance_specific(item: &mut Item, tval: i32, specific: i32, rng: &mut impl Rng) {
    let mut bias = ArtifactBias::None;
    random_resistance(item, tval, specific, &mut bias, rng);
}

/// Jewelry magic (a_m_aux_3, src/object2.cc): rolled plusses/pvals and
/// curses by (tval, sval). `power < 0` reverses the bonuses; plain
/// jewelry is given the -1 power half of the time by apply_magic.
fn jewelry_magic(gd: &GameData, item: &mut Item, power: i32, level: i32, rng: &mut impl Rng) {
    let o = &gd.objects[item.def];
    let (tval, sval) = (o.tval, o.sval);
    let neg = power < 0;
    let mut touched = false;
    macro_rules! mb {
        ($max:expr) => {
            m_bonus($max, level, rng)
        };
    }
    match (tval, sval) {
        // --- Rings ---
        (45, 0) => {
            // Woe (always cursed)
            item.cursed = true;
            item.to_a = -(5 + mb!(10));
            item.pval = -(1 + mb!(5));
            touched = true;
        }
        (45, 2) | (45, 3) => {
            // Weakness / Stupidity (always cursed)
            item.cursed = true;
            item.pval = -(1 + mb!(5));
            touched = true;
        }
        (45, 16) => {
            item.to_a = 5 + rng.gen_range(1..=8) + mb!(10);
            if neg {
                item.cursed = true;
                item.to_a = -item.to_a;
            }
            touched = true;
        }
        (45, 17) | (45, 18) | (45, 19) => {
            // Acid / Flames / Ice
            item.to_a = 5 + rng.gen_range(1..=5) + mb!(10);
            touched = true;
        }
        (45, 24..=27) => {
            // STR / INT / DEX / CON
            item.pval = 1 + mb!(5);
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        (45, 28) => {
            item.to_h = 5 + rng.gen_range(1..=8) + mb!(10);
            if neg {
                item.cursed = true;
                item.to_h = -item.to_h;
            }
            touched = true;
        }
        (45, 29) => {
            item.to_d = 5 + rng.gen_range(1..=8) + mb!(10);
            if neg {
                item.cursed = true;
                item.to_d = -item.to_d;
            }
            touched = true;
        }
        (45, 30) => {
            item.to_h = rng.gen_range(1..=7) + mb!(10);
            item.to_d = rng.gen_range(1..=7) + mb!(10);
            if neg {
                item.cursed = true;
                item.to_h = -item.to_h;
                item.to_d = -item.to_d;
            }
            touched = true;
        }
        (45, 31) => {
            // Speed: super-charged 50% of the time, recursively.
            item.pval = rng.gen_range(1..=5) + mb!(5);
            while rng.gen_bool(0.5) {
                item.pval += 1;
            }
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        (45, 48) => {
            // Lordly: high random resistances plus a big AC bonus.
            loop {
                random_resistance_specific(item, tval, rng.gen_range(19..=38), rng);
                if rng.gen_range(1..=4) != 1 {
                    break;
                }
            }
            item.to_a = 10 + rng.gen_range(1..=5) + mb!(10);
            touched = true;
        }
        (45, 49) => {
            // Extra attacks
            item.pval = mb!(3).max(1);
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        (45, 59) => {
            // Critical hits
            item.pval = mb!(10).max(1);
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        // --- Amulets ---
        (40, 0) => {
            // Doom (always cursed)
            item.cursed = true;
            item.to_a = -(5 + mb!(10));
            item.pval = -(1 + mb!(5));
            touched = true;
        }
        (40, 6) | (40, 7) | (40, 26) | (40, 28) => {
            // Brilliance / Charisma / Infra-vision / Wisdom
            item.pval = 1 + mb!(5);
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        (40, 8) => {
            // The Magi (never cursed; 1-in-3 slow digestion)
            item.pval = 1 + mb!(3);
            if rng.gen_range(1..=3) == 1 {
                item.flags.push("SLOW_DIGEST".to_string());
            }
            touched = true;
        }
        (40, 13) | (40, 14) => {
            // Anti-Magic / Anti-Teleportation
            if neg {
                item.cursed = true;
                touched = true;
            }
        }
        (40, 15) => {
            // Resistance: 1-in-3 a random resist (ids 5..38), 1-in-5
            // poison resist.
            if rng.gen_range(1..=3) == 1 {
                random_resistance_specific(item, tval, rng.gen_range(5..=38), rng);
            }
            if rng.gen_range(1..=5) == 1 && !item.flags.iter().any(|f| f == "RES_POIS") {
                item.flags.push("RES_POIS".to_string());
            }
        }
        (40, 17) => {
            // Serpents: a cursed one reverses only its pval (object2.cc).
            item.pval = 1 + mb!(5);
            item.to_a = 1 + mb!(6);
            if neg {
                item.cursed = true;
                item.pval = -item.pval;
            }
            touched = true;
        }
        (40, 23) | (40, 25) => {
            // Trickery / Devotion (never cursed)
            item.pval = 1 + mb!(3);
            touched = true;
        }
        (40, 24) => {
            // Weaponmastery (never cursed)
            item.pval = 1 + mb!(2);
            item.to_a = 1 + mb!(4);
            item.to_h = 1 + mb!(5);
            item.to_d = 1 + mb!(5);
            touched = true;
        }
        _ => {}
    }
    if touched {
        item.identified = false;
    }
}

/// Spawn (or extend) the floor stack at (x, y). Follows floor_carry
/// (object2.cc:6022): similar stacks absorb the item, and a grid never
/// holds more than 23 object entries (extra drops are lost).
pub fn place_floor_item(
    commands: &mut Commands,
    gd: &GameData,
    tiles: &TileAssets,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    x: i32,
    y: i32,
    item: Item,
) {
    for (_e, pos, mut stack) in stacks.iter_mut() {
        if pos.x == x && pos.y == y {
            // Scan for a combination target first.
            if let Some(i) = stack.stack.iter().position(|p| items_similar(gd, p, &item)) {
                absorb_item_tval(gd, &mut stack.stack[i], item);
                return;
            }
            // The 23-object cap; artifacts may always fall (the original
            // drop_near teleports them to a free grid instead).
            if stack.stack.len() > 23 && !is_artifact(gd, &item) {
                return;
            }
            stack.stack.push(item);
            return;
        }
    }
    let o = &gd.objects[item.def];
    commands.spawn((
        FloorItem { stack: vec![item] },
        crate::game::GameEntity,
        GridPos { x, y },
        render::glyph_sprite(tiles, o.glyph(), o.color),
        Transform::from_translation(render::grid_to_world(x, y, 1.5)),
        Visibility::Hidden,
    ));
}

/// Spawn a gold pile at (x, y).
pub fn place_gold(commands: &mut Commands, tiles: &TileAssets, x: i32, y: i32, amount: i32) {
    place_gold_kind(commands, tiles, x, y, amount, String::new());
}

/// Spawn a gold pile with its coin-type name (make_gold).
pub fn place_gold_kind(
    commands: &mut Commands,
    tiles: &TileAssets,
    x: i32,
    y: i32,
    amount: i32,
    name: String,
) {
    commands.spawn((
        FloorGold { amount, name },
        crate::game::GameEntity,
        GridPos { x, y },
        render::glyph_sprite(tiles, '$', 11),
        Transform::from_translation(render::grid_to_world(x, y, 1.5)),
        Visibility::Hidden,
    ));
}

/// Scatter loose objects and gold over a freshly generated dungeon level.
/// Returns the generate.cc `rating` and `good_item_flag` contributions of
/// the placed objects (place_object -> make_object: artifacts, egos and
/// out-of-depth items raise the level feeling).
#[allow(clippy::too_many_arguments)]
pub fn scatter_objects(
    commands: &mut Commands,
    gd: &GameData,
    tiles: &TileAssets,
    map: &Map,
    depth: u32,
    dungeon: u32,
    player: (i32, i32),
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> (i32, bool) {
    // generate.cc:7701-7705: randnor(DUN_AMT_ROOM=9,3) room objects +
    // randnor(DUN_AMT_ITEM=3,3) objects, plus randnor(DUN_AMT_GOLD=3,3)
    // gold.  The port's Map carries no CAVE_ROOM bit, so the room/corridor
    // split cannot be honoured; the budget and legal-grid search remain.
    let objects = randnor(9, 3, rng).max(0) + randnor(3, 3, rng).max(0);
    let gold = randnor(3, 3, rng).max(0);
    let mut used: HashSet<(i32, i32)> = HashSet::new();
    let mut placed = 0;
    let mut rating = 0i32;
    let mut good_item = false;
    while placed < objects + gold {
        // Pick a legal spot (alloc_object's SAFE_MAX_ATTEMPTS = 5000); when
        // the level is full the original gives up on the whole budget.
        let mut spot = None;
        for _ in 0..5000 {
            let x = rng.gen_range(1..map::MAP_W - 1);
            let y = rng.gen_range(1..map::MAP_H - 1);
            if !map.walkable(gd, x, y)
                || map::is_door(map.terrain_at(x, y))
                || map::chebyshev(x, y, player.0, player.1) < 3
                || !used.insert((x, y))
            {
                continue;
            }
            spot = Some((x, y));
            break;
        }
        let Some((x, y)) = spot else { break };
        // Gold first, then the object budget; level generation always
        // creates fresh stacks (cells are unique).
        if placed < gold {
            let (gid, amount) = make_gold(gd, depth, rng);
            place_gold_kind(commands, tiles, x, y, amount, gold_name(gd, gid));
        } else if let Some(mut item) = make_object_themed(
            gd,
            depth,
            false,
            false,
            gd.dungeon(dungeon).theme,
            created,
            rng,
        ) {
            // place_object(OBJ_FOUND_FLOOR): dungeon type and level
            // (object2.cc:4631-4649).
            let (r, g) = rating_of(gd, &item, depth);
            rating += r;
            good_item |= g;
            item.found = OBJ_FOUND_FLOOR;
            item.found_aux1 = dungeon as i32;
            item.found_aux2 = depth as i32;
            let o = &gd.objects[item.def];
            commands.spawn((
                FloorItem { stack: vec![item] },
                crate::game::GameEntity,
                GridPos { x, y },
                render::glyph_sprite(tiles, o.glyph(), o.color),
                Transform::from_translation(render::grid_to_world(x, y, 1.5)),
                Visibility::Hidden,
            ));
        }
        placed += 1;
    }
    (rating, good_item)
}

/// Monster death loot: sometimes gold, rarely an object.
#[allow(clippy::too_many_arguments)]
/// The treasure a monster carries (monster2.cc place_monster_one: gold or
/// one object, rolled against the dungeon level). Returned as a satchel so
/// the Stealing action can rob it before the kill; the same roll lands on
/// the floor when the monster dies.
/// The monster's carried treasure (monster2.cc place_monster_one): the
/// number of entries comes from the DROP_* flags, each entry is gold or an
/// object (ONLY_GOLD / ONLY_ITEM), DROP_GOOD / DROP_GREAT set quality and
/// DROP_RANDART carries an extra random artifact.
pub fn monster_carried_treasure(
    gd: &GameData,
    def: &crate::data::MonsterDef,
    depth: u32,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) -> (i32, Vec<Item>) {
    let good = def.has("DROP_GOOD");
    let great = def.has("DROP_GREAT");
    let only_item = def.has("ONLY_ITEM");
    let only_gold = def.has("ONLY_GOLD");
    let mimic = def.has("MIMIC");
    let mut number = 0;
    if def.has("DROP_60") && rng.gen_range(0..100) < 60 {
        number += 1;
    }
    if def.has("DROP_90") && rng.gen_range(0..100) < 90 {
        number += 1;
    }
    if def.has("DROP_1D2") {
        number += rng.gen_range(1..=2);
    }
    if def.has("DROP_2D2") {
        number += rng.gen_range(2..=4);
    }
    if def.has("DROP_3D2") {
        number += rng.gen_range(3..=6);
    }
    if def.has("DROP_4D2") {
        number += rng.gen_range(4..=8);
    }
    if mimic {
        number = 1;
    }
    let mut gold = 0;
    let mut items = Vec::new();
    // Creeping coins force their own treasure kind (object2.cc:4728).
    let coin = get_coin_type(def);
    for _ in 0..number {
        // monster2.cc: gold unless ONLY_ITEM, or when ONLY_GOLD forces it;
        // otherwise a 50/50 split.
        let do_gold = !mimic && !only_item && (only_gold || rng.gen_range(0..100) < 50);
        if do_gold {
            let (_, amount) = make_gold_coin(gd, depth, coin, rng);
            gold += amount;
        } else if let Some(item) =
            make_object_themed(gd, depth, good, great, def.objs, created, rng)
        {
            items.push(item);
        }
    }
    // DROP_RANDART carries one extra random artifact (monster2.cc:2306).
    if def.has("DROP_RANDART") {
        const RANDART_TVALS: [i32; 18] = [
            data::TV_MSTAFF,
            data::TV_BOOMERANG,
            data::TV_DIGGING,
            data::TV_HAFTED,
            data::TV_POLEARM,
            data::TV_AXE,
            data::TV_SWORD,
            data::TV_BOOTS,
            data::TV_GLOVES,
            data::TV_HELM,
            data::TV_CROWN,
            data::TV_SHIELD,
            data::TV_CLOAK,
            data::TV_SOFT_ARMOR,
            data::TV_HARD_ARMOR,
            data::TV_LITE,
            data::TV_AMULET,
            data::TV_RING,
        ];
        let candidates: Vec<usize> = gd
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| RANDART_TVALS.contains(&o.tval) && o.depth >= 1 && o.depth <= depth)
            .map(|(i, _)| i)
            .collect();
        if !candidates.is_empty() {
            let d = candidates[rng.gen_range(0..candidates.len())];
            let mut it = make_item(gd, d, depth, good, created, rng);
            create_artifact(gd, &mut it, depth, false, "", rng);
            items.push(it);
        }
    }
    (gold, items)
}

/// drop_near (object2.cc:4834): place a dropped/thrown object near
/// (x, y). It breaks with `chance` percent (unless an artifact), and the
/// 7x7 line-of-sight floor cells are scored by distance and crowding;
/// similar stacks combine. `occupied` holds the monster cells (artifact
/// bounce) and `player` the player's cell (the "roll beneath your feet"
/// message). Returns where it landed.
#[allow(clippy::too_many_arguments)]
pub fn drop_near(
    commands: &mut Commands,
    gd: &GameData,
    tiles: &TileAssets,
    map: &Map,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    log: &mut MessageLog,
    known: &HashSet<usize>,
    occupied: &HashSet<(i32, i32)>,
    player: Option<(i32, i32)>,
    obj: Item,
    chance: i32,
    x: i32,
    y: i32,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    let plural = obj.count != 1;
    let label = obj.label(gd, known);
    if !is_artifact(gd, &obj) && rng.gen_range(0..100) < chance {
        log.add(format!(
            "The {} disappear{}.",
            label,
            if plural { "" } else { "s" }
        ));
        return None;
    }
    let mut bs = -1i32;
    let mut bn = 0;
    let mut best = (x, y);
    let mut found = false;
    for dy in -3..=3 {
        for dx in -3..=3 {
            let d = dy * dy + dx * dx;
            if d > 10 {
                continue;
            }
            let (ty, tx) = (x + dx, y + dy);
            if !Map::in_bounds(ty, tx) {
                continue;
            }
            if !map::line_of_sight(map, gd, x, y, tx, ty) {
                continue;
            }
            if !gd.terrain(map.terrain_at(tx, ty)).is_floor {
                continue;
            }
            let mut comb = false;
            let mut k = 0usize;
            for (_, p, s) in stacks.iter() {
                if p.x == tx && p.y == ty {
                    k += s.stack.len();
                    for o in &s.stack {
                        if items_similar(gd, o, &obj) {
                            comb = true;
                        }
                    }
                }
            }
            if !comb {
                k += 1;
            }
            if k > 23 {
                continue;
            }
            let s = 1000 - (d + k as i32 * 5);
            if s < bs {
                continue;
            }
            if s > bs {
                bn = 0;
            }
            bn += 1;
            if bn >= 2 && rng.gen_range(0..bn) != 0 {
                continue;
            }
            bs = s;
            best = (ty, tx);
            found = true;
        }
    }
    if !found {
        if !is_artifact(gd, &obj) {
            log.add(format!(
                "The {} disappear{}.",
                label,
                if plural { "" } else { "s" }
            ));
            return None;
        }
        // Preserve artifacts: bounce them to any free floor grid.
        for i in 0..1000 {
            let (tx, ty) = if i < 900 {
                (
                    best.0 + rng.gen_range(-1..=1),
                    best.1 + rng.gen_range(-1..=1),
                )
            } else {
                (
                    rng.gen_range(1..map::MAP_W - 1),
                    rng.gen_range(1..map::MAP_H - 1),
                )
            };
            if Map::in_bounds(tx, ty)
                && gd.terrain(map.terrain_at(tx, ty)).is_floor
                && !occupied.contains(&(tx, ty))
            {
                best = (tx, ty);
                found = true;
                break;
            }
        }
        if !found {
            log.add(format!(
                "The {} disappear{}.",
                label,
                if plural { "" } else { "s" }
            ));
            return None;
        }
    }
    place_floor_item(commands, gd, tiles, stacks, best.0, best.1, obj);
    if chance != 0 {
        if let Some(pp) = player {
            if pp == best {
                log.add("You feel something roll beneath your feet.");
            }
        }
    }
    Some(best)
}

/// The context-free monster-death loot path: the same `monster_carried_treasure`
/// roll as `drop_loot`, but every drop lands through `drop_near`
/// (object2.cc/xtra2.cc:2125-2176).  The game.rs death caller should use
/// this once it can pass the map and monster cells.
#[allow(clippy::too_many_arguments)]
pub fn drop_loot_ex(
    commands: &mut Commands,
    gd: &GameData,
    tiles: &TileAssets,
    map: &Map,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    log: &mut MessageLog,
    known: &HashSet<usize>,
    occupied: &HashSet<(i32, i32)>,
    player: Option<(i32, i32)>,
    x: i32,
    y: i32,
    depth: u32,
    def: &crate::data::MonsterDef,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) {
    let (gold, items) = monster_carried_treasure(gd, def, depth, created, rng);
    if gold > 0 {
        place_gold(commands, tiles, x, y, gold);
    }
    for item in items {
        drop_near(
            commands, gd, tiles, map, stacks, log, known, occupied, player, item, -1, x, y, rng,
        );
    }
}

pub fn drop_loot(
    commands: &mut Commands,
    gd: &GameData,
    tiles: &TileAssets,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    x: i32,
    y: i32,
    depth: u32,
    def: &crate::data::MonsterDef,
    created: &mut HashSet<u32>,
    rng: &mut impl Rng,
) {
    let (gold, items) = monster_carried_treasure(gd, def, depth, created, rng);
    if gold > 0 {
        place_gold(commands, tiles, x, y, gold);
    }
    for item in items {
        place_floor_item(commands, gd, tiles, stacks, x, y, item);
    }
}

/// Teleport the entity at `pos` to a random walkable cell within `range`.
/// Mirrors teleport_player's landing test (spells1.cc:478): a naked,
/// non-permanent floor grid outside vaults (`CAVE_ICKY`) and unoccupied.
pub fn teleport(
    gd: &GameData,
    map: &Map,
    pos: &mut GridPos,
    occupied: &HashSet<(i32, i32)>,
    range: i32,
    rng: &mut impl Rng,
) {
    for _ in 0..300 {
        let nx = pos.x + rng.gen_range(-range..=range);
        let ny = pos.y + rng.gen_range(-range..=range);
        if !Map::in_bounds(nx, ny) || occupied.contains(&(nx, ny)) {
            continue;
        }
        if map.icky.contains(&Map::idx(nx, ny)) {
            continue;
        }
        if gd.terrain(map.terrain_at(nx, ny)).permanent {
            continue;
        }
        if map.walkable(gd, nx, ny) {
            pos.x = nx;
            pos.y = ny;
            return;
        }
    }
}

/// One monster's teleport-drag snapshot entry (teleport_to_player).
#[derive(Clone, Copy)]
pub struct TportSnap {
    pub entity: Entity,
    pub x: i32,
    pub y: i32,
    pub level: i32,
    pub tport: bool,
    pub res_tele: bool,
    pub awake: bool,
}

/// teleport_to_player (spells1.cc:364): pick the landing cells for awake
/// `SF_TPORT` monsters that do not resist teleportation.  The C++ scans
/// the eight neighbours of the player's *previous* cell after a teleport
/// (spells1.cc:576-613); the 100-sided skill test against the monster
/// level lives here.  Returns `(entity, x, y)` moves for the caller to
/// apply (a mutable query or Commands::insert).
pub fn plan_tport_drags(
    gd: &GameData,
    map: &Map,
    old: (i32, i32),
    player: (i32, i32),
    snap: &[TportSnap],
    occupied: &HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) -> Vec<(Entity, i32, i32)> {
    let mut candidates: Vec<(Entity, i32)> = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (ox, oy) = (old.0 + dx, old.1 + dy);
            for s in snap {
                if (s.x, s.y) == (ox, oy) && s.tport && !s.res_tele && s.awake {
                    candidates.push((s.entity, s.level));
                }
            }
        }
    }
    let mut out = Vec::new();
    for (e, level) in candidates {
        // "Skill" test (spells1.cc:382).
        if rng.gen_range(1..=100) > level {
            continue;
        }
        let mut dis = 2;
        let mut min = dis / 2;
        let mut attempts = 500;
        let mut spot: Option<(i32, i32)> = None;
        while attempts > 0 && spot.is_none() {
            attempts -= 1;
            if dis > 200 {
                dis = 200;
            }
            for _ in 0..500 {
                let ny = player.1 + rng.gen_range(-dis..=dis);
                let nx = player.0 + rng.gen_range(-dis..=dis);
                let d = map::pref_distance(player.1, player.0, ny, nx);
                if d < min || d > dis {
                    continue;
                }
                if !Map::in_bounds(nx, ny) {
                    continue;
                }
                if !map.walkable(gd, nx, ny)
                    || occupied.contains(&(nx, ny))
                    || (nx, ny) == player
                {
                    continue;
                }
                let t = map.terrain_at(nx, ny);
                if t == map::T_GLYPH || t == map::T_MINOR_GLYPH {
                    continue;
                }
                spot = Some((nx, ny));
                break;
            }
            dis *= 2;
            min /= 2;
        }
        if let Some((nx, ny)) = spot {
            out.push((e, nx, ny));
        }
    }
    out
}

/// The Query-based `teleport_to_player` drag used by ModalCtx/game.rs
/// (their monster queries expose a mutable GridPos).
#[allow(clippy::too_many_arguments)]
pub fn drag_tport_monsters<F: bevy::ecs::query::QueryFilter>(
    gd: &GameData,
    map: &Map,
    old: (i32, i32),
    player: (i32, i32),
    monsters: &mut Query<(Entity, &mut crate::game::Monster, &mut GridPos), F>,
    rng: &mut impl Rng,
) {
    let snap: Vec<TportSnap> = monsters
        .iter()
        .map(|(e, m, p)| {
            let d = &gd.monsters[m.def];
            TportSnap {
                entity: e,
                x: p.x,
                y: p.y,
                level: crate::game::monster_level(gd, m),
                tport: d.spells.iter().any(|s| s == "TPORT"),
                res_tele: d.has("RES_TELE"),
                awake: m.awake,
            }
        })
        .collect();
    let occupied: HashSet<(i32, i32)> = snap.iter().map(|s| (s.x, s.y)).collect();
    for (e, nx, ny) in plan_tport_drags(gd, map, old, player, &snap, &occupied, rng) {
        if let Ok((_, _, mut mp)) = monsters.get_mut(e) {
            mp.x = nx;
            mp.y = ny;
        }
    }
}

/// The C++ `teleport_player` (spells1.cc:470): move the player to a random
/// cell up to `range` away and then drag awake SF_TPORT monsters from the
/// old position's neighbours.  Player teleport call sites should use this
/// instead of `teleport` so the drag is not skipped.
#[allow(clippy::too_many_arguments)]
pub fn teleport_player<F: bevy::ecs::query::QueryFilter>(
    gd: &GameData,
    map: &Map,
    monsters: &mut Query<(Entity, &mut crate::game::Monster, &mut GridPos), F>,
    pos: &mut GridPos,
    occupied: &HashSet<(i32, i32)>,
    range: i32,
    rng: &mut impl Rng,
) {
    let old = (pos.x, pos.y);
    teleport(gd, map, pos, occupied, range, rng);
    let player = (pos.x, pos.y);
    if old != player {
        drag_tport_monsters(gd, map, old, player, monsters, rng);
    }
}

/// teleport_player_level (spells1.cc:857): a random shift up or down with
/// the quest/no-teleport/rooted guards.  Returns true when a transition is
/// scheduled.
pub fn teleport_player_level(
    gd: &GameData,
    ps: &mut PlayerState,
    turn: &mut TurnState,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> bool {
    // No effect in a plot quest.
    if ps.depth >= crate::game::PLOT_DEPTH_BASE {
        log.add("There is no effect.");
        return false;
    }
    if crate::game::level_has_flag(gd, ps, "NO_TELEPORT") {
        log.add("No teleport on special levels...");
        return false;
    }
    if crate::game::level_has_flag(gd, ps, "NO_EASY_MOVE") {
        log.add("Some powerful force prevents your from teleporting.");
        return false;
    }
    if crate::corrupt::resist_continuum(ps) {
        log.add("The space-time continuum can't be disrupted.");
        return false;
    }
    if ps.tim_roots > 0 {
        return false;
    }
    let d = gd.dungeon(ps.dungeon);
    let (msg, depth) = if ps.depth == 0 {
        ("You sink through the floor.", ps.depth + 1)
    } else if ps.depth >= d.maxdepth.saturating_sub(1) {
        ("You rise up through the ceiling.", ps.depth - 1)
    } else if rng.gen_range(0..100) < 50 {
        ("You rise up through the ceiling.", ps.depth - 1)
    } else {
        ("You sink through the floor.", ps.depth + 1)
    };
    log.add(msg);
    turn.pending = Some(crate::game::Goto::Depth(depth));
    true
}

/// combine_pack (object2.cc:5838): merge similar pack stacks, keeping the
/// lower slot and running the full object_absorb merge.
pub fn combine_pack(gd: &GameData, inv: &mut Inventory, log: &mut MessageLog) {
    let mut flag = false;
    let mut i = inv.pack.len();
    while i > 0 {
        i -= 1;
        let mut j = 0;
        while j < i {
            if items_similar(gd, &inv.pack[j], &inv.pack[i]) {
                let it = inv.pack.remove(i);
                absorb_item_tval(gd, &mut inv.pack[j], it);
                flag = true;
                break;
            }
            j += 1;
        }
    }
    if flag {
        log.add("You combine some items in your pack.");
    }
}

/// reorder_pack (object2.cc:5911): stable insertion sort of the pack by
/// decreasing tval, awareness, increasing sval, identification, rod
/// recharge time and finally decreasing value.
pub fn reorder_pack(gd: &GameData, inv: &mut Inventory, log: &mut MessageLog) {
    let aware = |gd: &GameData, inv: &Inventory, it: &Item| {
        !gd.flavor_pos.contains_key(&it.def) || inv.known.contains(&it.def) || it.identified
    };
    let mut flag = false;
    let mut i = 1usize;
    while i < inv.pack.len() {
        let o = inv.pack[i].clone();
        let o_val = object_value(gd, &o);
        let o_tval = gd.objects[o.def].tval;
        let o_sval = gd.objects[o.def].sval;
        let o_aware = aware(gd, inv, &o);
        let mut j = 0usize;
        while j < i {
            let p = &inv.pack[j];
            let j_tval = gd.objects[p.def].tval;
            if o_tval > j_tval {
                break;
            }
            if o_tval < j_tval {
                j += 1;
                continue;
            }
            if !o_aware {
                j += 1;
                continue;
            }
            if !aware(gd, inv, p) {
                break;
            }
            let j_sval = gd.objects[p.def].sval;
            if o_sval < j_sval {
                break;
            }
            if o_sval > j_sval {
                j += 1;
                continue;
            }
            if !o.identified {
                j += 1;
                continue;
            }
            if !p.identified {
                break;
            }
            if o_tval == data::TV_ROD_MAIN {
                if o.timeout > p.timeout {
                    break;
                }
                if o.timeout < p.timeout {
                    j += 1;
                    continue;
                }
            }
            let j_val = object_value(gd, p);
            if o_val > j_val {
                break;
            }
            if o_val < j_val {
                j += 1;
                continue;
            }
            j += 1;
        }
        if j < i {
            let it = inv.pack.remove(i);
            inv.pack.insert(j, it);
            flag = true;
        }
        i += 1;
    }
    if flag {
        log.add("You reorder some items in your pack.");
    }
}

fn heal(ps: &mut PlayerState, amount: i32, cut: i32, log: &mut MessageLog) {
    ps.hp = (ps.hp + amount).min(ps.max_hp);
    ps.cut = (ps.cut - cut).max(0);
    log.add("You feel better.");
}

fn hurt(
    gd: &GameData,
    ps: &mut PlayerState,
    amount: i32,
    log: &mut MessageLog,
    next: &mut NextState<AppState>,
) {
    ps.hp -= amount;
    log.add(format!("It hurts! ({})", amount));
    if ps.hp <= 0 && crate::game::player_death_check(ps, gd, log) {
        log.add("You die.");
        next.set(AppState::Dead);
    }
}

/// tot_dam_aux (src/cmd1.cc): slay/brand multipliers of a set of item
/// flags against a monster's race flags. Returns the damage multiplier
/// plus monster-side poison/cut inflictions (SPEC_POIS/SPEC_CUT).
pub fn dam_aux(
    slays: &HashSet<String>,
    def: &crate::data::MonsterDef,
    rng: &mut impl Rng,
) -> (i32, bool, bool) {
    let has = |f: &str| slays.contains(f);
    let m = |f: &str| def.has(f);
    let mut mult = 1;
    let (mut pois, mut cut) = (false, false);
    if (has("SLAY_ANIMAL") && m("ANIMAL")) || (has("SLAY_EVIL") && m("EVIL")) {
        mult = mult.max(2);
    }
    for (slay, race) in [
        ("SLAY_UNDEAD", "UNDEAD"),
        ("SLAY_DEMON", "DEMON"),
        ("SLAY_ORC", "ORC"),
        ("SLAY_TROLL", "TROLL"),
        ("SLAY_GIANT", "GIANT"),
        ("SLAY_DRAGON", "DRAGON"),
    ] {
        if has(slay) && m(race) {
            mult = mult.max(3);
        }
    }
    for (kill, race) in [
        ("KILL_DRAGON", "DRAGON"),
        ("KILL_UNDEAD", "UNDEAD"),
        ("KILL_DEMON", "DEMON"),
    ] {
        if has(kill) && m(race) {
            mult = mult.max(5);
        }
    }
    // TR_BLESSED itself only sets bless_blade (xtra1.cc:2525); the port's
    // old extra x2 vs EVIL was an invention, so it is gone.
    let brand = |flag: &str, im: &str, sus: &str, mult: &mut i32| -> bool {
        if !has(flag) {
            return false;
        }
        if m(im) {
            return true; // immune: no brand effect
        }
        if m(sus) {
            *mult = (*mult).max(6);
        } else {
            *mult = (*mult).max(3);
        }
        false
    };
    brand("BRAND_ACID", "IM_ACID", "SUSCEP_ACID", &mut mult);
    brand("BRAND_ELEC", "IM_ELEC", "SUSCEP_ELEC", &mut mult);
    brand("BRAND_FIRE", "IM_FIRE", "SUSCEP_FIRE", &mut mult);
    brand("BRAND_COLD", "IM_COLD", "SUSCEP_COLD", &mut mult);
    // Chaotic weapons carry a random brand on every hit.
    if has("CHAOTIC") {
        let flags = ["BRAND_ACID", "BRAND_ELEC", "BRAND_FIRE", "BRAND_COLD"];
        let f = flags[rng.gen_range(0..flags.len())];
        let (im, sus) = match &f[6..] {
            "ACID" => ("IM_ACID", "SUSCEP_ACID"),
            "ELEC" => ("IM_ELEC", "SUSCEP_ELEC"),
            "FIRE" => ("IM_FIRE", "SUSCEP_FIRE"),
            _ => ("IM_COLD", "SUSCEP_COLD"),
        };
        if !m(im) {
            mult = mult.max(if m(sus) { 6 } else { 3 });
        }
    }
    if has("BRAND_POIS") && !m("IM_POIS") {
        if m("SUSCEP_POIS") {
            mult = mult.max(6);
            pois = rng.gen_bool(0.95);
        } else {
            mult = mult.max(3);
            pois = rng.gen_bool(0.50);
        }
    }
    if has("WOUNDING") && !m("NO_CUT") {
        cut = rng.gen_bool(0.5);
    }
    (mult.max(1), pois, cut)
}

/// Vampiric weapons heal the wielder for a quarter of the damage dealt
/// to living targets.
pub fn vampiric_heal(slays: &HashSet<String>, def: &crate::data::MonsterDef, dam: i32) -> i32 {
    if !slays.contains("VAMPIRIC") {
        return 0;
    }
    if def.has("UNDEAD") || def.has("DEMON") || def.has("NONLIVING") {
        return 0;
    }
    (dam / 4).max(1)
}

/// Slay flags of the wielded weapon (melee) or bow+ammo (ranged).
pub fn weapon_slays(gd: &GameData, inv: &Inventory, ranged_ammo: Option<&Item>) -> HashSet<String> {
    match ranged_ammo {
        None => inv.equip[data::SLOT_WEAPON]
            .as_ref()
            .map(|w| inv.item_slays_with_set(gd, w))
            .unwrap_or_default(),
        Some(ammo) => {
            let mut s = inv.item_slays_with_set(gd, ammo);
            if let Some(bow) = &inv.equip[data::SLOT_BOW] {
                s.extend(inv.item_slays_with_set(gd, bow));
            }
            s
        }
    }
}

/// How many distinct members of an artifact's set are worn.
pub fn set_members_worn(gd: &GameData, inv: &Inventory, artifact: u32) -> usize {
    let Some(set) = gd.set_of_artifact(artifact) else {
        return 0;
    };
    set.members
        .iter()
        .filter(|m| {
            inv.equip
                .iter()
                .flatten()
                .any(|it| it.artifact == m.artifact)
        })
        .count()
}

/// Drain a stat (monster LOSE_* blows, curses). Sustains block it;
/// stats never drop below 3. Restored by restore_stat.
pub fn drain_stat(ps: &mut PlayerState, i: usize, totals: &EquipTotals, log: &mut MessageLog) {
    let names = ["STR", "INT", "WIS", "DEX", "CON", "CHR"];
    if totals.sustains.contains(names[i]) {
        log.add(format!("Your {} is unaffected!", names[i]));
        return;
    }
    if ps.stats[i] > 3 {
        ps.stats[i] -= 1;
        log.add(format!("You feel your {} draining away!", names[i]));
    }
}

/// Restore a stat to its (birth + equipment) maximum (Restore potions).
pub fn restore_stat(ps: &mut PlayerState, i: usize, totals: &EquipTotals, log: &mut MessageLog) {
    let names = ["STR", "INT", "WIS", "DEX", "CON", "CHR"];
    let max = ps.stat_base[i] + totals.stats[i];
    if ps.stats[i] < max {
        ps.stats[i] = max;
        log.add(format!("Your {} is restored!", names[i]));
    }
}

/// Lose experience (nether/time attacks). HOLD_LIFE gives a chance
/// (by attack strength) to shrug it off entirely, else blunts it.
pub fn lose_exp(
    ps: &mut PlayerState,
    amount: u64,
    block_pct: u32,
    totals: &EquipTotals,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) {
    if totals.hold_life && rng.gen_range(0..100) < block_pct {
        log.add("You resist the draining effect!");
        return;
    }
    let amount = if totals.hold_life {
        amount / 10
    } else {
        amount
    };
    let lost = amount.min(ps.exp);
    ps.exp -= lost;
    ps.exp_drained += lost;
    log.add(format!("You feel your life draining away! (-{} exp)", lost));
}

/// apply_disenchant (spells1.cc:2252): pick one of the eight worn gear
/// slots (or the explicit `mode` slot, C++ `INVEN_WIELD + weap`) and
/// strip its plusses.  An empty slot or an item with nothing to lose is
/// a no-op; artifacts resist 71% of the time.  The caller checks the
/// DISEN resistance before calling (melee1.cc:540) - the totals check in
/// `disenchant_item` covers the port's resistance-equipped callers.
pub fn apply_disenchant(
    gd: &GameData,
    inv: &mut Inventory,
    totals: &EquipTotals,
    mode: Option<usize>,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    if totals.resists.contains("DISEN") {
        log.add("You resist the disenchantment!");
        return;
    }
    // randint(8): WIELD, BOW, BODY, OUTER, ARM(SHIELD), HEAD, HANDS, FEET.
    const TABLE: [usize; 8] = [
        data::SLOT_WEAPON,
        data::SLOT_BOW,
        data::SLOT_BODY,
        data::SLOT_CLOAK,
        data::SLOT_SHIELD,
        data::SLOT_HEAD,
        data::SLOT_HANDS,
        data::SLOT_FEET,
    ];
    let (slot, table_idx) = match mode {
        Some(s) => (s, TABLE.iter().position(|&t| t == s).unwrap_or(0)),
        None => {
            let n = rng.gen_range(1..=8) - 1;
            (TABLE[n], n)
        }
    };
    let label = index_to_label(23 + table_idx as i32);
    let known = inv.known.clone();
    let Some(item) = inv.equip.get_mut(slot).and_then(|s| s.as_mut()) else {
        return;
    };
    // Nothing to disenchant.
    if item.to_h <= 0 && item.to_d <= 0 && item.to_a <= 0 {
        return;
    }
    let name = item.label(gd, &known);
    if item.artifact != 0 && rng.gen_range(0..100) < 71 {
        log.add(format!(
            "Your {} ({}) resist{} disenchantment!",
            name,
            label,
            if item.count != 1 { "" } else { "s" }
        ));
        return;
    }
    // Disenchant tohit/todam/toac, with the extra 20% roll above +5.
    if item.to_h > 0 {
        item.to_h -= 1;
    }
    if item.to_h > 5 && rng.gen_range(0..100) < 20 {
        item.to_h -= 1;
    }
    if item.to_d > 0 {
        item.to_d -= 1;
    }
    if item.to_d > 5 && rng.gen_range(0..100) < 20 {
        item.to_d -= 1;
    }
    if item.to_a > 0 {
        item.to_a -= 1;
    }
    if item.to_a > 5 && rng.gen_range(0..100) < 20 {
        item.to_a -= 1;
    }
    log.add(format!(
        "Your {} ({}) {} disenchanted!",
        name,
        label,
        if item.count != 1 { "were" } else { "was" }
    ));
}

/// Disenchant a random worn item (UN_BONUS / disenchantment breath):
/// plusses are stripped; artifacts usually resist.
pub fn disenchant_item(
    gd: &GameData,
    inv: &mut Inventory,
    totals: &EquipTotals,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    apply_disenchant(gd, inv, totals, None, log, rng);
}

/// Curse a random worn item (CAUSE_1-3 / Hand of Doom; melee2.cc:2322).
/// One random equipment slot is picked; blessed items get a biased save.
pub fn curse_equipment(
    gd: &GameData,
    inv: &mut Inventory,
    chance: i32,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    curse_equipment_ex(gd, inv, chance, 0, false, log, rng);
}

/// curse_equipment with the heavy-curse roll and the DG_CURSE variant
/// (melee2.cc:2322/2377). `heavy_chance` upgrades artifacts to
/// HEAVY_CURSE; `dg` additionally brands the item with DG_CURSE.
pub fn curse_equipment_ex(
    gd: &GameData,
    inv: &mut Inventory,
    chance: i32,
    heavy_chance: i32,
    dg: bool,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    if rng.gen_range(1..=100) > chance {
        return;
    }
    let slot = rng.gen_range(0..data::NUM_SLOTS);
    let Some(item) = inv.equip[slot].as_mut() else {
        return;
    };
    let flags = item_flags(gd, item);
    let has = |f: &str| flags.iter().any(|x| *x == f);
    // Extra, biased saving throw for blessed items.
    if has("BLESSED") && rng.gen_range(1..=888) > chance {
        let name = item.label(gd, &inv.known);
        log.add(format!(
            "Your {} resist{} cursing!",
            name,
            if item.count > 1 { "" } else { "s" }
        ));
        return;
    }
    // name1/name2/randart-name: artifacts can be heavily cursed.
    let is_art = item.artifact != 0 || item.ego != 0 || !item.artifact_name.is_empty();
    let mut changed = false;
    if rng.gen_range(1..=100) <= heavy_chance && is_art {
        if !has("HEAVY_CURSE") {
            changed = true;
        }
        add_item_flag(item, "HEAVY_CURSE");
        add_item_flag(item, "CURSED");
        if dg {
            add_item_flag(item, "DG_CURSE");
        }
    } else {
        if !has("CURSED") {
            changed = true;
        }
        add_item_flag(item, "CURSED");
        if dg {
            add_item_flag(item, "DG_CURSE");
        }
    }
    item.cursed = true;
    item.known_cursed = true;
    if changed {
        log.add("There is a malignant black aura surrounding you...");
        if item.inscription == "uncursed" {
            item.inscription.clear();
        }
    }
}

/// Remove the curse from every worn item. HEAVY_CURSE resists half the
/// time; PERMA_CURSE can never be lifted (original behavior).
/// Outcome of a Recharge spell / scroll / activation (spells2.cc
/// recharge).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RechargeOutcome {
    /// The device gained `charges` (wand/staff) or fuel (rod).
    Recharged(i32),
    /// The magic backfired and drained the device.
    Drained,
    /// The magic consumed the device (one of a stack, or the whole wand
    /// stack on a catastrophic failure).
    Destroyed,
}

/// Recharge a wand, staff or assembled rod (spells2.cc recharge).
/// `power` is 130 for the scroll, 60 for activation, 80 for the shop
/// service and 60 + scale(RECHARGE, 140) for the spell. `perfect` is
/// AB_PERFECT_CASTING (Mages recharge more safely).
pub fn recharge_device(
    gd: &GameData,
    item: &mut Item,
    power: i32,
    perfect: bool,
    rng: &mut impl Rng,
) -> RechargeOutcome {
    let def = &gd.objects[item.def];
    let lev = def.depth.max(1) as i32;
    let flags = item_flags(gd, item);
    let recharged = flags.contains(&"RECHARGE");
    let no_recharge = flags.contains(&"NO_RECHARGE");
    let tval = def.tval;
    let mut fail = no_recharge;
    if tval == data::TV_ROD_MAIN {
        let strength = ((power - lev).max(0)) / 5;
        if strength > 0 && !recharged && rng.gen_range(0..strength) == 0 {
            fail = true;
        }
        if fail {
            item.timeout = 0;
            return RechargeOutcome::Drained;
        }
        let amount = power
            * Dice {
                count: 3,
                sides: 2,
                bonus: 0,
            }
            .roll(rng);
        item.timeout = (item.timeout + amount).min(item.pval2.max(1));
        // Mark as recharged (spells2.cc:2334).
        add_item_flag(item, "RECHARGED");
        return RechargeOutcome::Recharged(amount);
    }
    // Wands and staves: a stack of wands splits the charge penalty.
    let strength = if tval == data::TV_WAND && item.count > 1 {
        (100 + power - lev - 8 * item.charges / item.count.max(1) as i32) / 15
    } else {
        (100 + power - lev - 8 * item.charges) / 15
    };
    if strength > 0 && !recharged && rng.gen_range(0..strength) == 0 {
        fail = true;
    }
    if fail {
        // Failure severity (spells2.cc): rods drain, wands may lose the
        // whole stack, staves are destroyed; Perfect Casting is kinder.
        let fail_type = if perfect {
            match tval {
                data::TV_ROD_MAIN => {
                    if rng.gen_range(0..10) == 0 {
                        2
                    } else {
                        1
                    }
                }
                data::TV_WAND => {
                    if rng.gen_range(0..3) != 0 {
                        2
                    } else {
                        1
                    }
                }
                data::TV_STAFF => {
                    if rng.gen_range(0..2) == 0 {
                        2
                    } else {
                        0
                    }
                }
                _ => 1,
            }
        } else {
            match tval {
                data::TV_ROD_MAIN => {
                    if rng.gen_range(0..3) == 0 {
                        2
                    } else {
                        1
                    }
                }
                data::TV_WAND => {
                    if rng.gen_range(0..5) == 0 {
                        3
                    } else {
                        2
                    }
                }
                data::TV_STAFF => 2,
                _ => 1,
            }
        };
        add_item_flag(item, "RECHARGED");
        return match fail_type {
            0 => RechargeOutcome::Recharged(0),
            1 => {
                item.charges = 0;
                RechargeOutcome::Drained
            }
            _ => RechargeOutcome::Destroyed,
        };
    }
    let mut amount = rng.gen_range(1..=(power / (lev + 2) + 1).max(1));
    if tval == data::TV_WAND && item.count > 1 {
        amount += rng.gen_range(1..=(amount * (item.count as i32 - 1)).max(1)) / 2;
        amount = amount.clamp(1, 12);
    }
    if tval == data::TV_STAFF && item.count > 1 {
        amount = (amount / item.count as i32).max(1);
    }
    item.charges += amount;
    // Mark as recharged (spells2.cc:2334).
    add_item_flag(item, "RECHARGED");
    RechargeOutcome::Recharged(amount)
}

/// remove_curse_object / remove_curse_aux (spells2.cc:528/591): the worn
/// equipment walk.  `all` is the `*Remove Curse*` variant that also
/// lifts heavy curses; perma-curses never lift.  Non-artifact cursed
/// items get one jk roll to have their penalties reversed.
pub fn remove_curses_ex(
    gd: &GameData,
    inv: &mut Inventory,
    log: &mut MessageLog,
    level: u32,
    all: bool,
    rng: &mut impl Rng,
) -> bool {
    let mut any = false;
    for item in inv.equip.iter_mut().flatten() {
        if !item.cursed {
            continue;
        }
        let flags = item_flags(gd, item);
        if flags.iter().any(|f| *f == "PERMA_CURSE") {
            continue;
        }
        if !all && flags.iter().any(|f| *f == "HEAVY_CURSE") {
            continue;
        }
        // Reverse the curse effect: 1 in (55 - level) for non-artifacts
        // (spells2.cc:557).
        if !is_artifact(gd, item) && rng.gen_range(1..=(55i64 - level as i64).max(1)) == 1 {
            if item.to_a < 0 {
                item.to_a = -item.to_a;
            }
            if item.to_h < 0 {
                item.to_h = -item.to_h;
            }
            if item.to_d < 0 {
                item.to_d = -item.to_d;
            }
            if item.pval < 0 {
                item.pval = -item.pval;
            }
        }
        item.cursed = false;
        item.known_cursed = false;
        item.flags.retain(|f| f != "CURSED" && f != "HEAVY_CURSE");
        any = true;
    }
    if any {
        log.add("You feel as if a heavy burden has been lifted.");
    } else {
        log.add("You feel no change.");
    }
    any
}

/// project_o (spells1.cc:3730) hates_* tables: which objects an element
/// destroys when it hits them. PLASMA/METEOR combine two of these and are
/// handled directly in `floor_damage_events`.
pub fn hates_element(tval: i32, element: &str) -> bool {
    match element {
        "ACID" => matches!(
            tval,
            data::TV_ARROW
                | data::TV_BOLT
                | data::TV_BOW
                | data::TV_SWORD
                | data::TV_AXE
                | data::TV_HAFTED
                | data::TV_POLEARM
                | data::TV_HELM
                | data::TV_CROWN
                | data::TV_SHIELD
                | data::TV_BOOTS
                | data::TV_GLOVES
                | data::TV_CLOAK
                | data::TV_SOFT_ARMOR
                | data::TV_HARD_ARMOR
                | data::TV_DRAG_ARMOR
                | data::TV_STAFF
                | data::TV_SCROLL
                | data::TV_SKELETON
                | data::TV_BOTTLE
                | data::TV_EGG
        ),
        "ELEC" => matches!(tval, data::TV_RING | data::TV_WAND | data::TV_EGG),
        "FIRE" => matches!(
            tval,
            data::TV_ARROW
                | data::TV_LITE
                | data::TV_BOW
                | data::TV_HAFTED
                | data::TV_POLEARM
                | data::TV_BOOTS
                | data::TV_GLOVES
                | data::TV_CLOAK
                | data::TV_SOFT_ARMOR
                | data::TV_BOOK
                | data::TV_SYMBIOTIC_BOOK
                | data::TV_MUSIC_BOOK
                | data::TV_STAFF
                | data::TV_SCROLL
                | data::TV_EGG
        ),
        "COLD" | "ICE" | "SHARDS" | "FORCE" | "SOUND" => matches!(
            tval,
            data::TV_POTION | data::TV_POTION2 | data::TV_FLASK | data::TV_BOTTLE | data::TV_EGG
        ),
        _ => false,
    }
}

/// A secondary explosion caused by destroying an object on the floor
/// (spells1.cc project_o). The caller projects these once the stack scan
/// is over, because item.rs has no monster/terrain access.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloorBoom {
    /// A shattered potion: project `gf` at (x, y).
    Potion {
        x: i32,
        y: i32,
        gf: &'static str,
        dam: i32,
        radius: i32,
    },
    /// An exploding corpse: project shards at (x, y).
    Corpse {
        x: i32,
        y: i32,
        dam: i32,
        radius: i32,
    },
}

/// potion_smash_effect (spells1.cc:8420): what the contents of a shattered
/// potion project. `None` = the potion merely splashes (useless contents,
/// personal buffs, food-like fare). Returns (gf, damage, radius).
pub fn potion_smash_effect(sval: i32, rng: &mut impl Rng) -> Option<(&'static str, i32, i32)> {
    let (gf, dam, radius): (&'static str, i32, i32) = match sval {
        // Salt water, slime mold, stat-loss, water, apple juice.
        0 | 1 | 2 | 5 | 13 | 16..=21 => return None,
        // Useful only on the inside: detection, cures, resists, buffs,
        // augments, enlightenment, resistance, invulnerability, new life.
        24..=28 | 30..=33 | 41..=53 | 55..=57 | 59..=60 | 62..=63 => return None,
        4 => ("OLD_SLOW", 5, 2),
        6 => ("POIS", 3, 2),
        7 => ("DARK", 0, 2),
        9 => ("OLD_CONF", 0, 2),
        11 => ("OLD_SLEEP", 0, 2),
        15 | 22 => ("SHARDS", damroll(25, 25, rng), 2),
        23 => ("MANA", damroll(10, 10, rng), 1),
        29 => ("OLD_SPEED", 0, 2),
        34 => ("OLD_HEAL", damroll(2, 3, rng), 2),
        35 => ("OLD_HEAL", damroll(4, 3, rng), 2),
        36 | 61 => ("OLD_HEAL", damroll(6, 3, rng), 2),
        37 => ("OLD_HEAL", damroll(10, 10, rng), 2),
        38 | 39 => ("OLD_HEAL", damroll(50, 50, rng), 1),
        40 => ("MANA", damroll(10, 10, rng), 1),
        _ => return None,
    };
    Some((gf, dam, radius))
}

/// Elemental destruction of floor item stacks (project_o, spells1.cc):
/// hated objects are destroyed on contact with the element's exact
/// message; artifacts and IGNORE_<element> items are unaffected. Damaging
/// projection that leaves a mark (corpse explosion, potion smash) is
/// returned in `events` for the caller to apply.
pub fn floor_damage(
    gd: &GameData,
    commands: &mut Commands,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    cells: &[(i32, i32)],
    element: &str,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    floor_damage_events(
        gd,
        commands,
        stacks,
        cells,
        element,
        100,
        log,
        rng,
        &mut Vec::new(),
    );
}

/// project_o with the full per-object logic. `dam` is the projection's
/// reduced damage (used by corpse explosions).
#[allow(clippy::too_many_arguments)]
pub fn floor_damage_events(
    gd: &GameData,
    commands: &mut Commands,
    stacks: &mut Query<(Entity, &GridPos, &mut FloorItem), impl bevy::ecs::query::QueryFilter>,
    cells: &[(i32, i32)],
    element: &str,
    dam: i32,
    log: &mut MessageLog,
    rng: &mut impl Rng,
    events: &mut Vec<FloorBoom>,
) {
    for (e, pos, mut stack) in stacks.iter_mut() {
        if !cells.contains(&(pos.x, pos.y)) {
            continue;
        }
        let (x, y) = (pos.x, pos.y);
        let mut keep: Vec<Item> = Vec::with_capacity(stack.stack.len());
        for it in stack.stack.drain(..) {
            let tval = gd.objects[it.def].tval;
            let plural = it.count > 1;
            let is_art = is_artifact(gd, &it);
            let flags = item_flags(gd, &it);
            let ignores = |el: &str| flags.iter().any(|f| *f == format!("IGNORE_{}", el));
            let mut do_kill = false;
            let mut ignore = false;
            let mut note: Option<&'static str> = None;
            match element {
                "CORPSE_EXPL" => {
                    // GF_CORPSE_EXPL: corpses detonate in a shard burst
                    // scaled by the projection damage.
                    if tval == data::TV_CORPSE {
                        do_kill = true;
                        note = Some(if plural { " explode!" } else { " explodes!" });
                        if let Some(m) = gd.monsters.get(it.note as usize) {
                            let hd = m.hdice.parse::<i32>().unwrap_or(1);
                            let hs = m.hside.parse::<i32>().unwrap_or(1);
                            let base = if m.has("FORCE_MAXHP") {
                                hd * hs
                            } else {
                                damroll(hd, hs, rng)
                            };
                            let bdam = (base * dam / 100).max(1);
                            let radius = (7 * dam / 100).max(1);
                            events.push(FloorBoom::Corpse {
                                x,
                                y,
                                dam: bdam,
                                radius,
                            });
                        }
                    }
                }
                "ACID" => {
                    if hates_element(tval, "ACID") {
                        do_kill = true;
                        note = Some(if plural { " melt!" } else { " melts!" });
                        if ignores("ACID") {
                            ignore = true;
                        }
                    }
                }
                "ELEC" => {
                    if hates_element(tval, "ELEC") {
                        do_kill = true;
                        note = Some(if plural {
                            " are destroyed!"
                        } else {
                            " is destroyed!"
                        });
                        if ignores("ELEC") {
                            ignore = true;
                        }
                    }
                }
                "FIRE" => {
                    if hates_element(tval, "FIRE") {
                        do_kill = true;
                        note = Some(if plural { " burn up!" } else { " burns up!" });
                        if ignores("FIRE") {
                            ignore = true;
                        }
                    }
                }
                "COLD" => {
                    if hates_element(tval, "COLD") {
                        do_kill = true;
                        note = Some(if plural { " shatter!" } else { " shatters!" });
                        if ignores("COLD") {
                            ignore = true;
                        }
                    }
                }
                "ICE" | "SHARDS" | "FORCE" | "SOUND" => {
                    // The "break" table does *not* honour IGNORE_COLD.
                    if hates_element(tval, "COLD") {
                        do_kill = true;
                        note = Some(if plural { " shatter!" } else { " shatters!" });
                    }
                }
                "PLASMA" => {
                    if hates_element(tval, "FIRE") {
                        do_kill = true;
                        note = Some(if plural { " burn up!" } else { " burns up!" });
                        ignore = ignores("FIRE");
                    }
                    if hates_element(tval, "ELEC") {
                        ignore = false;
                        do_kill = true;
                        note = Some(if plural {
                            " are destroyed!"
                        } else {
                            " is destroyed!"
                        });
                        if ignores("ELEC") {
                            ignore = true;
                        }
                    }
                }
                "METEOR" => {
                    if hates_element(tval, "FIRE") {
                        do_kill = true;
                        note = Some(if plural { " burn up!" } else { " burns up!" });
                        ignore = ignores("FIRE");
                    }
                    if hates_element(tval, "COLD") {
                        ignore = false;
                        do_kill = true;
                        note = Some(if plural { " shatter!" } else { " shatters!" });
                        if ignores("COLD") {
                            ignore = true;
                        }
                    }
                }
                // Mana, chaos and disintegration destroy everything.
                "MANA" | "DISINTEGRATE" => {
                    do_kill = true;
                    note = Some(if plural {
                        " are destroyed!"
                    } else {
                        " is destroyed!"
                    });
                }
                "CHAOS" => {
                    do_kill = true;
                    note = Some(if plural {
                        " are destroyed!"
                    } else {
                        " is destroyed!"
                    });
                    if flags.iter().any(|f| *f == "RES_CHAOS") {
                        ignore = true;
                    }
                }
                // Holy and hell fire consume cursed non-artifacts.
                "HOLY_FIRE" | "HELL_FIRE" => {
                    if it.cursed {
                        do_kill = true;
                        note = Some(if plural {
                            " are destroyed!"
                        } else {
                            " is destroyed!"
                        });
                    }
                }
                _ => {}
            }
            if !do_kill {
                keep.push(it);
                continue;
            }
            let name = it.label(gd, &HashSet::new());
            if is_art || ignore {
                log.add(format!(
                    "The {} {} unaffected!",
                    name,
                    if plural { "are" } else { "is" }
                ));
                keep.push(it);
                continue;
            }
            if let Some(n) = note {
                log.add(format!("The {}{}", name, n));
            }
            // Potions shatter into their contents.
            if tval == data::TV_POTION || tval == data::TV_POTION2 {
                if let Some((gf, sdam, radius)) = potion_smash_effect(gd.objects[it.def].sval, rng)
                {
                    events.push(FloorBoom::Potion {
                        x,
                        y,
                        gf,
                        dam: sdam,
                        radius,
                    });
                }
            }
        }
        stack.stack = keep;
        if stack.stack.is_empty() {
            commands.entity(e).despawn();
        }
    }
}



/// Try to raise one plus of an item (Enchant scrolls). Plusses above +10
/// resist further enchantment; +15 is the absolute cap (ToME m_bonus).
/// The enchant failure table (spells2.cc:520 enchant_table).
const ENCHANT_TABLE: [i32; 16] = [
    0, 10, 50, 100, 200, 300, 400, 500, 650, 800, 950, 987, 993, 995, 998, 1000,
];

/// remove_curse (spells2.cc:607): the `all = false` worn-equipment walk.
pub fn remove_curses(
    gd: &GameData,
    inv: &mut Inventory,
    log: &mut MessageLog,
    level: u32,
) -> bool {
    let mut rng = crate::rng::current();
    remove_curses_ex(gd, inv, log, level, false, &mut rng)
}

/// remove_all_curse (spells2.cc:612): the scroll that also lifts heavy
/// curses.
pub fn remove_all_curses(
    gd: &GameData,
    inv: &mut Inventory,
    log: &mut MessageLog,
    level: u32,
) -> bool {
    let mut rng = crate::rng::current();
    remove_curses_ex(gd, inv, log, level, true, &mut rng)
}

/// A single enchantment roll on one value (spells2.cc enchant):
/// `randint(1000) > chance` succeeds; artifacts save 50% of the time.
fn enchant_roll(value: i32, art: bool, rng: &mut impl Rng) -> bool {
    let chance = if value < 0 {
        0
    } else if value > 15 {
        1000
    } else {
        ENCHANT_TABLE[value as usize]
    };
    rng.gen_range(1..=1000) > chance && (!art || rng.gen_range(0..100) < 50)
}

/// Which fields `enchant` may improve.
#[derive(Clone, Copy, Default)]
pub struct EnchantFlags {
    pub to_h: bool,
    pub to_d: bool,
    pub to_a: bool,
    pub pval: bool,
}

/// enchant (spells2.cc:1556): up to `n` enchantment rolls with the pile
/// resistance, the artifact 50% save and the 25% curse-breaking roll.
pub fn enchant(
    gd: &GameData,
    item: &mut Item,
    n: i32,
    f: EnchantFlags,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> bool {
    let art = is_artifact(gd, item);
    let mut prob = item.count.max(1) as i32 * 100;
    if data::is_ammo(gd.objects[item.def].tval) {
        prob /= 20;
    }
    let mut res = false;
    // Break the curse unless it is permanent (spells2.cc:1566).
    let maybe_break_curse = |item: &mut Item, log: &mut MessageLog| {
        if item.cursed && !item_flags(gd, item).iter().any(|f| *f == "PERMA_CURSE") {
            log.add("The curse is broken!");
            item.cursed = false;
            item.known_cursed = false;
            item.flags.retain(|f| f != "CURSED" && f != "HEAVY_CURSE");
        }
    };
    for _ in 0..n {
        if prob > 0 && rng.gen_range(0..prob) >= 100 {
            continue;
        }
        if f.to_h && enchant_roll(item.to_h, art, rng) {
            item.to_h += 1;
            res = true;
            if item.to_h >= 0 && rng.gen_range(0..100) < 25 {
                maybe_break_curse(item, log);
            }
        }
        if f.to_d && enchant_roll(item.to_d, art, rng) {
            item.to_d += 1;
            res = true;
            if item.to_d >= 0 && rng.gen_range(0..100) < 25 {
                maybe_break_curse(item, log);
            }
        }
        if f.pval && enchant_roll(item.pval * 2, art, rng) {
            item.pval += 1;
            res = true;
            if item.pval >= 0 && rng.gen_range(0..100) < 25 {
                maybe_break_curse(item, log);
            }
        }
        if f.to_a && enchant_roll(item.to_a, art, rng) {
            item.to_a += 1;
            res = true;
            if item.to_a >= 0 && rng.gen_range(0..100) < 25 {
                maybe_break_curse(item, log);
            }
        }
    }
    res
}

pub fn enchant_plus(value: i32, rng: &mut impl Rng) -> Option<i32> {
    if enchant_roll(value, false, rng) {
        Some(value + 1)
    } else {
        None
    }
}

/// Elemental inventory damage from monster breaths (spells1.cc
/// inven_damage + acid_dam/fire_dam/...): the element destroys the
/// hated tvals unit by unit; armor loses a point to acid (minus_ac).
/// Artifacts are immune; IGNORE_<element> protects the item. The caller
/// supplies the exact chance (1 for <30 damage, 2 for <60, else 3).
pub fn inven_damage(
    gd: &GameData,
    inv: &mut Inventory,
    element: &str,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    // The port's callers pass no damage: use the middle tier.
    inven_damage_ex(gd, inv, element, 2, log, rng);
}

/// inven_damage (spells1.cc:1774) with the damage-derived chance.
/// Returns the svals of potions shattered inside the pack, so the caller
/// can apply `potion_smash_effect` (item.rs has no projection access).
pub fn inven_damage_ex(
    gd: &GameData,
    inv: &mut Inventory,
    element: &str,
    perc: i32,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> Vec<i32> {
    let mut smashed = Vec::new();
    // Acid first etches one worn armour piece (minus_ac): pick one of the
    // six body slots, and only a real armour piece with AC left is hit.
    if element == "ACID" {
        const BODY: [usize; 6] = [
            data::SLOT_BODY,
            data::SLOT_SHIELD,
            data::SLOT_CLOAK,
            data::SLOT_HANDS,
            data::SLOT_HEAD,
            data::SLOT_FEET,
        ];
        let slot = BODY[rng.gen_range(0..BODY.len())];
        if let Some(item) = inv.equip[slot].as_mut() {
            if gd.objects[item.def].ac + item.to_a > 0 {
                let name = item.label(gd, &inv.known);
                if item_ignores(gd, item, "ACID") {
                    log.add(format!("Your {} is unaffected!", name));
                } else {
                    item.to_a -= 1;
                    log.add(format!("Your {} is damaged!", name));
                }
            }
        }
    }
    let mut i = 0;
    while i < inv.pack.len() {
        // C++ skips artifacts before the destroy-type check.
        if is_artifact(gd, &inv.pack[i]) {
            i += 1;
            continue;
        }
        let tval = gd.objects[inv.pack[i].def].tval;
        if !hates_element(tval, element) || item_ignores(gd, &inv.pack[i], element) {
            i += 1;
            continue;
        }
        let count = inv.pack[i].count;
        let mut amt = 0;
        for _ in 0..count {
            if rng.gen_range(0..100) < perc {
                amt += 1;
            }
        }
        if amt == 0 {
            i += 1;
            continue;
        }
        // "All of your X (a) were destroyed!" etc. (spells1.cc:1810).
        let it = &inv.pack[i];
        let name = it.label(gd, &inv.known);
        let letter = (b'a' + i as u8) as char;
        let prefix = if count > 1 {
            if amt == count {
                "All of y"
            } else if amt > 1 {
                "Some of y"
            } else {
                "One of y"
            }
        } else {
            "Y"
        };
        log.add(format!(
            "{}our {} ({}) {} destroyed!",
            prefix,
            name,
            letter,
            if amt > 1 { "were" } else { "was" }
        ));
        // Potions smash open at the player's position.
        if tval == data::TV_POTION {
            smashed.push(gd.objects[it.def].sval);
        }
        // Wand charges scale with the surviving wands (-LM-).
        if tval == data::TV_WAND && amt < count {
            let it = &mut inv.pack[i];
            it.charges -= it.charges * amt as i32 / count.max(1) as i32;
        }
        if amt >= count {
            inv.pack.remove(i);
        } else {
            inv.pack[i].count -= amt as u32;
            i += 1;
        }
    }
    smashed
}

/// breakage_chance (cmd2.cc:2218): percent chance a thrown/fired object
/// breaks. `archery_scale` is get_skill_scale(SKILL_ARCHERY, 10).
pub fn breakage_chance(tval: i32, archery_scale: i32) -> i32 {
    let reducer = 1 + archery_scale;
    match tval {
        data::TV_FLASK | data::TV_POTION | data::TV_POTION2 | data::TV_BOTTLE | data::TV_FOOD => {
            100
        }
        data::TV_LITE | data::TV_SCROLL | data::TV_SKELETON => 50,
        data::TV_ARROW => 50 / reducer,
        data::TV_WAND | data::TV_SPIKE => 25,
        data::TV_SHOT | data::TV_BOLT => 25 / reducer,
        data::TV_BOOMERANG => 1,
        _ => 10,
    }
}

/// Enchant the wielded weapon (to-hit and/or to-dam), n attempts each.
pub fn enchant_weapon(
    gd: &GameData,
    inv: &mut Inventory,
    to_h: bool,
    to_d: bool,
    n: i32,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) {
    let Some(w) = &mut inv.equip[data::SLOT_WEAPON] else {
        log.add("You have no weapon wielded.");
        return;
    };
    let any = enchant(
        gd,
        w,
        n,
        EnchantFlags {
            to_h,
            to_d,
            ..Default::default()
        },
        log,
        rng,
    );
    log.add(if any {
        "Your weapon glows brightly!"
    } else {
        "The enchantment fails."
    });
}

/// Enchant a random worn armour piece, n attempts.
pub fn enchant_armour(
    gd: &GameData,
    inv: &mut Inventory,
    n: i32,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) {
    let slots: Vec<usize> = (0..data::NUM_SLOTS)
        .filter(|&s| {
            s != data::SLOT_WEAPON
                && s != data::SLOT_BOW
                && s != data::SLOT_LITE
                && s != data::SLOT_AMULET
                && s != data::SLOT_RING1
                && s != data::SLOT_RING2
                && inv.equip[s].is_some()
        })
        .collect();
    let Some(&slot) = slots.get(rng.gen_range(0..slots.len().max(1))) else {
        log.add("You are not wearing any armour.");
        return;
    };
    let item = inv.equip[slot].as_mut().expect("worn");
    let any = enchant(
        gd,
        item,
        n,
        EnchantFlags {
            to_a: true,
            ..Default::default()
        },
        log,
        rng,
    );
    log.add(if any {
        "Your armour glows brightly!"
    } else {
        "The enchantment fails."
    });
}

/// Eat a food item: restores nutrition by its pval, then applies the
/// food's special effect (mushrooms, Athelas; original do_cmd_eat_food).
pub fn eat_food(
    gd: &GameData,
    def: usize,
    ps: &mut PlayerState,
    inv: &mut Inventory,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) {
    let o = &gd.objects[def];
    // Food feeds the player in different ways (cmd6.cc do_cmd_eat_food):
    // vampires gain nothing, NO_FOOD races a fortieth, everyone else all.
    let vampire = crate::game::player_has_flag(gd, ps, "VAMPIRE");
    let undead = crate::game::player_has_flag(gd, ps, "UNDEAD");
    let no_food = crate::game::player_has_flag(gd, ps, "NO_FOOD");
    let gained = if vampire {
        log.add("Mere victuals hold scant sustenance for a being such as yourself.");
        if ps.food < crate::game::FOOD_HUNGRY {
            log.add("Your hunger can only be satisfied with fresh blood!");
        }
        0
    } else if no_food {
        log.add(if undead {
            "The food of mortals is poor sustenance for you."
        } else {
            "Food is poor sustenance for you."
        });
        o.pval / 40
    } else {
        o.pval
    };
    ps.food = (ps.food + gained).min(crate::game::FOOD_MAX);
    log.add(format!("You eat {}.", o.name));
    if ps.food >= crate::game::FOOD_FULL {
        log.add("You are full!");
    }
    let sval = o.sval;
    let totals = inv.totals_for(gd, ps);
    let res = |e: &str| totals.resists.contains(e);
    // Full sval effect table (cmd6.cc do_cmd_eat_food).
    match sval {
        0 => {
            // Mushroom of Poison (cmd6.cc:1026).
            if !res("POIS") && ps.oppose_pois <= 0 {
                ps.poison += 10 + rng.gen_range(1..=10);
                log.add("You are poisoned!");
            }
        }
        1 => {
            if !res("BLIND") {
                ps.blind += 200 + rng.gen_range(1..=200);
                log.add("You are blinded!");
            }
        }
        2 => {
            if !res("FEAR") {
                ps.fear += 10 + rng.gen_range(1..=10);
                log.add("You feel paranoid!");
            }
        }
        3 => {
            if !res("CONF") {
                ps.confuse += 10 + rng.gen_range(1..=10);
                log.add("You feel confused!");
            }
        }
        4 => {
            // Hallucination (cmd6.cc:1066).
            if !res("CHAOS") {
                ps.image += 250 + rng.gen_range(1..=250);
                log.add("Oh, wow! Everything looks so cosmic now!");
            }
        }
        5 => {
            if !has_free_act(ps, inv, gd, &totals) {
                ps.paralyze = 10 + rng.gen_range(1..=10);
                log.add("You are paralyzed!");
            }
        }
        6 | 7 | 8 | 9 | 10 | 11 => {
            // Bad mushrooms: damage plus a stat hit (cmd6.cc:1086-1132).
            let (dd, ds, stat) = match sval {
                6 => (6, 6, 0),    // Weakness
                7 => (6, 6, 4),    // Sickness
                8 => (8, 8, 1),    // Stupidity
                9 => (8, 8, 2),    // Naivety
                10 => (10, 10, 4), // Unhealth
                _ => (10, 10, 0),  // Disease
            };
            let dam = Dice {
                count: dd,
                sides: ds,
                bonus: 0,
            }
            .roll(rng);
            ps.hp -= dam;
            log.add(format!("You are wracked with pain! ({dam})"));
            drain_stat(ps, stat, &totals, log);
        }
        12 => {
            ps.poison = 0;
            log.add("You are no longer poisoned.");
        }
        13 => {
            ps.blind = 0;
            log.add("You can see again.");
        }
        14 => {
            ps.fear = 0;
            log.add("You feel bold again.");
        }
        15 => {
            ps.confuse = 0;
            log.add("Your head clears.");
        }
        16 => {
            ps.hp = (ps.hp
                + Dice {
                    count: 4,
                    sides: 8,
                    bonus: 0,
                }
                .roll(rng))
            .min(ps.max_hp);
            log.add("You feel better.");
        }
        17 => restore_stat(ps, 0, &totals, log),
        18 => restore_stat(ps, 4, &totals, log),
        19 => {
            for i in 0..6 {
                restore_stat(ps, i, &totals, log);
            }
        }
        36 => {
            // Slime Mold: 2% chance of the grow-mold power (cmd6.cc:1246).
            if rng.gen_range(0..100) < 2 && !ps.power_grow_mold {
                ps.power_grow_mold = true;
                log.add("You feel a sudden affinity for mold.");
            }
        }
        37 => {
            // Lembas / Elvish Waybread (cmd6.cc:1260).
            ps.poison = 0;
            ps.hp = (ps.hp
                + Dice {
                    count: 4,
                    sides: 8,
                    bonus: 0,
                }
                .roll(rng))
            .min(ps.max_hp);
            ps.food = crate::game::FOOD_MAX - 1;
            log.add("That tastes very good.");
        }
        38 | 39 => {
            // Pint of Ale/Wine: the empty bottle is returned (cmd6.cc:1270).
            if let Some(b) = gd
                .objects
                .iter()
                .position(|o| o.tval == crate::data::TV_BOTTLE && o.sval == 1)
            {
                inv.add_with(gd, Item::base(gd, b));
                log.add("You keep the empty bottle.");
            }
        }
        40 => {
            // Sprig of Athelas (cmd6.cc:1283): cures poison/stun/cuts and
            // breaks the Black Breath's hold.
            ps.poison = 0;
            ps.stun = 0;
            ps.cut = 0;
            if ps.black_breath {
                ps.black_breath = false;
                log.add("The hold of the Black Breath on you is broken!");
            }
            log.add("A wholesome taste spreads through your body.");
        }
        41 => {
            // Greater Ration of Health: permanent +70 hp (cmd6.cc:1017).
            ps.hp_mod += 70;
            ps.max_hp += 70;
            ps.hp += 70;
            log.add("You feel your health improve!");
        }
        42 => {
            // Fortune cookie: a random rumour (cmd6.cc:1195).
            let roll = rng.gen_range(0..20);
            let text = if roll == 0 {
                CHAINSWD_TXT
            } else if roll == 1 {
                ERROR_TXT
            } else if roll < 5 {
                DEATH_TXT
            } else {
                RUMORS_TXT
            };
            log.add(random_file_line(text, rng));
        }
        _ => {
            // Plain food (biscuits, jerky, rations): just "tastes good".
            if sval >= 32 {
                log.add("That tastes good.");
            }
        }
    }
}

const RUMORS_TXT: &str = include_str!("../assets/data/rumors.txt");
const DEATH_TXT: &str = include_str!("../assets/data/death.txt");
const ERROR_TXT: &str = include_str!("../assets/data/error.txt");
const CHAINSWD_TXT: &str = include_str!("../assets/data/chainswd.txt");

/// `object_attr` (object1.cc:5780): the colour an object is drawn with.
/// A legacy TV_RANDART junkart uses the per-game `random_artifacts[sval].attr`
/// rolled once by init_randart (birth.cc:487) and saved with the player;
/// everything else keeps its kind's colour.
pub fn object_attr(gd: &GameData, it: &Item, ps: &PlayerState) -> u8 {
    let o = &gd.objects[it.def];
    if o.tval == data::TV_RANDART {
        if let Some(&c) = ps.junkart_colors.get(it.note as usize) {
            return c;
        }
    }
    o.color
}

/// The per-game shuffled flavour colour of an unidentified kind
/// (object1.cc object_flavor + flavor_init). EASY_KNOW kinds are aware
/// on sight and keep their true colour.
pub fn flavor_color(gd: &GameData, ps: &PlayerState, def_idx: usize) -> Option<u8> {
    let (g, pos) = *gd.flavor_pos.get(&def_idx)?;
    let o = &gd.objects[def_idx];
    // EASY_KNOW kinds are known on sight and keep their true colour; the
    // caller already checks the per-pack known set for the rest.
    if o.flags.iter().any(|f| f == "EASY_KNOW") {
        return None;
    }
    let group = &gd.flavor_groups[g];
    let n = group.len();
    if n < 2 {
        return Some(o.color);
    }
    let mut order: Vec<usize> = (0..n).collect();
    let mut rng = rand::rngs::StdRng::seed_from_u64(ps.flavor_seed ^ (g as u64 + 1));
    rand::seq::SliceRandom::shuffle(order.as_mut_slice(), &mut rng);
    Some(gd.objects[group[order[pos]]].color)
}

/// A random line from a lib/file text (the first line is the count, the
/// second the buffer marker; both are skipped).
fn random_file_line(text: &str, rng: &mut impl Rng) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("********") && l.parse::<u32>().is_err())
        .collect();
    lines
        .get(rng.gen_range(0..lines.len().max(1)))
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Corpse sub-values (src/defines.hpp SV_CORPSE_*).
const SV_CORPSE_CORPSE: i32 = 1;
const SV_CORPSE_SKELETON: i32 = 2;
const SV_CORPSE_HEAD: i32 = 3;
const SV_CORPSE_SKULL: i32 = 4;
const SV_CORPSE_MEAT: i32 = 5;

/// Prepare a fresh corpse/skeleton for monster `def_idx` (xtra2.cc
/// place_corpse): the meat amount and corpse weight feed corpse_effect.
pub fn init_corpse(gd: &GameData, it: &mut Item, def_idx: usize, decay: i32, rng: &mut impl Rng) {
    let w = gd
        .monsters
        .get(def_idx)
        .map(|m| m.weight.max(1) as i32)
        .unwrap_or(100);
    it.note = def_idx as u32;
    it.pval = w + rng.gen_range(0..w);
    // object2.cc place_corpse: pval2/pval3 carry the monster and the
    // corpse weight; the instance weight mirrors it (C++ weight).
    it.pval2 = w + rng.gen_range(0..w) / 10 + 1;
    it.weight = it.pval2;
    it.fuel = decay;
    // Where-found bookkeeping (xtra2.cc place_corpse).
    it.found = OBJ_FOUND_MONSTER;
    it.found_aux1 = def_idx as i32;
}

/// The head/skeleton a decayed corpse turns into (object2.cc pack_decay
/// 6124-6163 / floor_decay 6215-6252), or None when it simply rots away.
/// `found_aux1` (the monster) rides along so the remainder keeps its name.
pub fn decayed_corpse(gd: &GameData, it: &Item, rng: &mut impl Rng) -> Option<Item> {
    if gd.objects[it.def].tval != crate::data::TV_CORPSE {
        return None;
    }
    let sval = gd.objects[it.def].sval as i32;
    let mon = it.note as usize;
    let def = gd.monsters.get(mon);
    // C++ re-rolls the remainder's weight from the monster race weight
    // (wt/60 + rand_int(wt)/600 for a skull, wt/4 + rand_int(wt)/40 for a
    // skeleton).
    let wt = def.map(|m| m.weight.max(1) as i32).unwrap_or(100);
    let (new_sval, new_weight) = if sval == SV_CORPSE_HEAD {
        // A head becomes a skull.
        (SV_CORPSE_SKULL, wt / 60 + rng.gen_range(0..wt) / 600)
    } else if sval == SV_CORPSE_CORPSE
        && def.map(|m| m.has("DROP_SKELETON")).unwrap_or(false)
    {
        // A corpse of a skeletal monster becomes a skeleton.
        (SV_CORPSE_SKELETON, wt / 4 + rng.gen_range(0..wt) / 40)
    } else {
        return None;
    };
    let kind = gd.object_by_tval_sval(crate::data::TV_CORPSE, new_sval)?;
    let mut out = Item::base(gd, kind);
    out.count = it.count;
    out.note = it.note;
    out.weight = new_weight;
    // Named remains are artifacts ("The skull of Farmer Maggot").
    if !it.artifact_name.is_empty() {
        out.identified = true;
        out.artifact_name = "named remains".to_string();
    }
    Some(out)
}

/// object_out_desc_where_found (object1.cc:2066) + the found switch
/// (object1.cc:2977-3019): the "You found it..." inspection line, or None
/// when the item has no where-found record.
pub fn where_found_text(gd: &GameData, it: &Item) -> Option<String> {
    let place = |dungeon: i32, level: i32| -> String {
        if dungeon <= 0 {
            "in the wilderness or in a town".to_string()
        } else {
            let name = gd
                .dungeons
                .iter()
                .find(|d| d.id as i32 == dungeon)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| "the dungeon".to_string());
            format!("on level {} of {}", level, name)
        }
    };
    match it.found {
        OBJ_FOUND_MONSTER => {
            let mon = gd
                .monsters
                .get(it.found_aux1.max(0) as usize)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "something".to_string());
            Some(format!(
                "You found it in the remains of {} {}.",
                mon,
                place(it.found_aux3, it.found_aux4)
            ))
        }
        OBJ_FOUND_FLOOR => Some(format!(
            "You found it lying on the ground {}.",
            place(it.found_aux1, it.found_aux2)
        )),
        OBJ_FOUND_VAULT => Some(format!(
            "You found it lying in a vault {}.",
            place(it.found_aux1, it.found_aux2)
        )),
        OBJ_FOUND_SPECIAL => {
            Some("You found it lying on the floor of a special level.".to_string())
        }
        OBJ_FOUND_RUBBLE => Some("You found it while digging a rubble.".to_string()),
        OBJ_FOUND_REWARD => Some("It was given to you as a reward.".to_string()),
        OBJ_FOUND_STORE => Some(format!(
            "You bought it from the {}.",
            gd.stores
                .get(it.found_aux1.max(0) as usize)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "a shop".to_string())
        )),
        OBJ_FOUND_STOLEN => Some(format!(
            "You stole it from the {}.",
            gd.stores
                .get(it.found_aux1.max(0) as usize)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "a shop".to_string())
        )),
        OBJ_FOUND_SELFMADE => Some("You made it yourself.".to_string()),
        _ => None,
    }
}

/// Lose experience without the HOLD_LIFE bookkeeping (corpse_effect
/// applies its own hold checks before calling this).
fn lose_exp_raw(ps: &mut PlayerState, amount: u64, log: &mut MessageLog) {
    let lost = amount.min(ps.exp);
    ps.exp -= lost;
    ps.exp_drained += lost;
    log.add(format!("You feel your life draining away! (-{} exp)", lost));
}

/// The effects of eating (or slicing) a corpse: its blows discharge
/// immediately and its breath organ still holds a charge (cmd6.cc
/// corpse_effect). The monster kind is `it.note`, the meat `it.pval` and
/// the corpse weight `it.pval2` (set by `init_corpse`). Returns the
/// friendly-summon flags (S_*) that fired; the caller spawns them since
/// it owns map/commands access.
#[allow(clippy::too_many_arguments)]
pub fn corpse_effect(
    gd: &GameData,
    it: &mut Item,
    ps: &mut PlayerState,
    inv: &mut Inventory,
    totals: &EquipTotals,
    cutting: bool,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> Vec<String> {
    let Some(def) = gd.monsters.get(it.note as usize).cloned() else {
        return Vec::new();
    };
    let sval = gd.objects[it.def].sval as i32;
    let meat = it.pval.max(1);
    let weight = it.pval2.max(1);
    let mweight = def.weight.max(1) as i32;

    // How much of the monster's breath attack remains.
    let mut brpow = if meat <= mweight {
        0
    } else {
        ((meat - mweight) / 5).min(mweight / 5)
    };
    // Breath is only discharged by accident or by slicing off pieces.
    if sval != SV_CORPSE_CORPSE || (!cutting && rng.gen_range(0..(weight / 5).max(1)) != 0) {
        brpow = 0;
    }

    let res = |e: &str| totals.resists.contains(e);
    let immune = |e: &str| totals.immunities.contains(e);
    let has_spell = |s: &str| def.spells.iter().any(|x| x == s);
    let mut harmful = false;
    let mut idam = 0;

    // Immediate effects from the monster's blows (never when cutting).
    if !cutting {
        for b in def.blows.iter().take(4) {
            if b.method.is_empty() {
                continue;
            }
            let Some(d) = Dice::parse(&b.dice) else {
                continue;
            };
            let dam = d.roll(rng) * meat / weight / 2;
            idam = d.roll(rng) * if weight / meat > 2 { weight / meat } else { 2 };
            let mdam = d.count * d.sides * 2;
            // Methods that mean nothing once the creature is dead.
            if matches!(
                b.method.as_str(),
                "BITE"
                    | "STING"
                    | "ENGULF"
                    | "DROOL"
                    | "SPIT"
                    | "GAZE"
                    | "WAIL"
                    | "BEG"
                    | "INSULT"
                    | "MOAN"
            ) {
                continue;
            }
            match b.effect.as_str() {
                "POISON" => {
                    if !res("POIS") {
                        ps.poison += dam + idam + 10;
                        log.add("The corpse is poisonous!");
                        harmful = true;
                    }
                }
                "ACID" => {
                    if !immune("ACID") && dam > 0 {
                        let mut dmg = dam;
                        if res("ACID") {
                            dmg = (dmg + 2) / 3;
                        }
                        ps.hp -= dmg;
                        log.add("The acidic flesh burns you!");
                        harmful = true;
                    } else {
                        ps.resist_timer += idam;
                    }
                }
                "FIRE" => {
                    // Original quirk: the immunity test is inverted here.
                    if immune("FIRE") || dam <= 0 {
                        let mut dmg = dam;
                        if res("FIRE") {
                            dmg = (dmg + 2) / 3;
                        }
                        ps.hp -= dmg;
                        log.add("The fiery flesh burns you!");
                        harmful = true;
                    } else {
                        ps.resist_timer += idam;
                    }
                }
                "BLIND" => {
                    if !res("BLIND") {
                        ps.blind += dam * 2 + idam * 2 + 20;
                        log.add("You are blinded by the juices!");
                    }
                }
                "CONFUSE" => {
                    if !res("CONF") {
                        ps.confuse += dam + idam + 10;
                        log.add("You feel confused!");
                    }
                    if !res("CHAOS") && mdam > dam && rng.gen_range(0..(mdam - dam)) != 0 {
                        ps.confuse += dam * 10 + idam * 10 + 100;
                    }
                }
                "HALLU" => {
                    if !res("CHAOS") && mdam > dam && rng.gen_range(0..(mdam - dam)) != 0 {
                        ps.image += dam * 10 + idam * 10 + 50;
                    }
                }
                "TERRIFY" => {
                    if !res("FEAR") {
                        ps.fear += dam + idam + 10;
                        log.add("You feel terrified!");
                    }
                }
                "PARALYZE" => {
                    if !has_free_act(ps, inv, gd, totals) {
                        ps.paralyze += dam + idam + 10;
                        log.add("You are paralyzed!");
                    }
                }
                "LOSE_STR" => drain_stat(ps, 0, totals, log),
                "LOSE_INT" => drain_stat(ps, 1, totals, log),
                "LOSE_WIS" => drain_stat(ps, 2, totals, log),
                "LOSE_DEX" => drain_stat(ps, 3, totals, log),
                "LOSE_CON" => drain_stat(ps, 4, totals, log),
                "LOSE_CHR" => drain_stat(ps, 5, totals, log),
                "LOSE_ALL" => {
                    for i in 0..6 {
                        drain_stat(ps, i, totals, log);
                    }
                    it.pval = 1;
                }
                "SANITY" => {
                    log.add("You feel your sanity slipping away!");
                    crate::game::take_sanity_hit(ps, dam, "eating an insane monster", log);
                }
                "EXP_10" | "EXP_20" | "EXP_40" | "EXP_80" => {
                    log.add("A black aura surrounds the corpse!");
                    let n = match b.effect.as_str() {
                        "EXP_10" => 10,
                        "EXP_20" => 20,
                        "EXP_40" => 40,
                        _ => 80,
                    };
                    if totals.hold_life && rng.gen_range(0..100) < 50 {
                        log.add("You keep hold of your life force!");
                    } else {
                        let amount = Dice {
                            count: n,
                            sides: 6,
                            bonus: 0,
                        }
                        .roll(rng) as u64
                            + (ps.exp / 100) * 2;
                        if totals.hold_life {
                            log.add("You feel your life slipping away!");
                            lose_exp_raw(ps, amount / 10, log);
                        } else {
                            lose_exp_raw(ps, amount, log);
                        }
                    }
                    it.pval = 1;
                }
                // Meaningless after death (HURT/UN_*/EAT_*/ELEC/COLD/...).
                _ => {}
            }
        }
    }

    // The organ that supplies breath attacks is not emptied on death.
    // Branches run in order so later ones see the leftovers, as original.
    let mut brdam = 0;
    if has_spell("BR_ACID") && brpow > 0 {
        brdam = (brpow / 3).min(1600);
        log.add("You are hit by a gush of acid!");
        if !immune("ACID") && brdam > 0 {
            ps.hp -= crate::game::apply_gf(gd, ps, inv, totals, "ACID", brdam, log, rng);
            harmful = true;
        }
        it.pval = 1;
    } else if has_spell("BR_ACID") {
        ps.resist_timer += rng.gen_range(0..10) + 10;
    }

    if has_spell("BR_ELEC") && brpow > 0 {
        brdam = (brpow / 3).min(1600);
        log.add("You receive a heavy shock!");
        if !immune("ELEC") && brdam > 0 {
            ps.hp -= crate::game::apply_gf(gd, ps, inv, totals, "ELEC", brdam, log, rng);
            harmful = true;
        }
        it.pval2 -= brpow;
        it.pval = it.pval2.max(1);
    } else if has_spell("BR_ELEC") {
        ps.resist_timer += rng.gen_range(0..10) + 10;
    }

    if has_spell("BR_FIRE") && brpow > 0 {
        brdam = (brpow / 3).min(1600);
        log.add("Roaring flames engulf you!");
        if !immune("FIRE") && brdam > 0 {
            ps.hp -= crate::game::apply_gf(gd, ps, inv, totals, "FIRE", brdam, log, rng);
            harmful = true;
        }
        it.pval = 1;
    } else if has_spell("BR_FIRE") {
        ps.resist_timer += rng.gen_range(0..10) + 10;
    }

    if has_spell("BR_COLD") && brpow > 0 {
        brdam = (brpow / 3).min(1600);
        log.add("You are caught in a freezing liquid!");
        if !immune("COLD") && brdam > 0 {
            ps.hp -= crate::game::apply_gf(gd, ps, inv, totals, "COLD", brdam, log, rng);
            harmful = true;
        }
        it.pval2 -= brpow;
        it.pval = it.pval2.max(1);
    } else if has_spell("BR_COLD") {
        ps.resist_timer += rng.gen_range(0..10) + 10;
    }

    if has_spell("BR_POIS") && brpow > 0 {
        brdam = (brpow / 3).min(800);
        log.add("You are surrounded by toxic gases!");
        if res("POIS") {
            brdam = (brdam + 2) / 3;
        } else {
            ps.poison += rng.gen_range(0..brdam.max(1)) + 10;
        }
        ps.hp -= brdam;
        it.pval2 -= brpow;
        it.pval = it.pval2.max(1);
        harmful = true;
    }

    if has_spell("BR_NETH") && brpow > 0 {
        brdam = (brpow / 6).min(550);
        log.add("A black aura surrounds the corpse!");
        if res("NETHER") {
            brdam = brdam * 6 / (rng.gen_range(1..=6) + 6);
        } else if totals.hold_life && rng.gen_range(0..100) < 75 {
            log.add("You keep hold of your life force!");
        } else {
            let amount = if totals.hold_life {
                200 + (ps.exp / 1000) * 2
            } else {
                200 + (ps.exp / 100) * 2
            };
            lose_exp_raw(ps, amount, log);
        }
        ps.hp -= brdam;
        it.pval2 -= brpow;
        it.pval = it.pval2.max(1);
        harmful = true;
    }

    if has_spell("BR_CONF") && brpow > 0 {
        log.add("A strange liquid splashes on you!");
        if !res("CONF") {
            ps.confuse += brdam + idam + 10;
        }
        it.pval2 -= brpow;
        it.pval = it.pval2.max(1);
    }

    if has_spell("BR_CHAO") && brpow > 0 {
        brdam = (brpow / 6).min(600);
        log.add("A swirling cloud surrounds you!");
        if res("CHAOS") {
            brdam = brdam * 6 / (rng.gen_range(1..=6) + 6);
        }
        if !res("CONF") {
            ps.confuse += rng.gen_range(0..20) + 10;
        }
        if !res("CHAOS") {
            ps.confuse += rng.gen_range(1..=10);
        }
        if !res("NETHER") && !res("CHAOS") {
            if totals.hold_life && rng.gen_range(0..100) < 75 {
                log.add("You keep hold of your life force!");
            } else {
                let amount = if totals.hold_life {
                    500 + (ps.exp / 1000) * 2
                } else {
                    5000 + (ps.exp / 100) * 2
                };
                lose_exp_raw(ps, amount, log);
            }
        }
        ps.hp -= brdam;
        it.pval = 1;
    }

    if has_spell("BR_DISE") && brpow > 0 {
        brdam = (brpow / 6).min(500);
        log.add("You are blasted by raw mana!");
        if res("DISEN") {
            brdam = brdam * 6 / (rng.gen_range(1..=6) + 6);
        } else {
            disenchant_item(gd, inv, totals, log, rng);
        }
        ps.hp -= brdam;
        it.pval = 1;
    }

    if has_spell("BR_PLAS") && brpow > 0 {
        brdam = (brpow / 6).min(150);
        log.add("Searing flames engulf the corpse!");
        if res("FIRE") {
            brdam = (brdam + 2) / 3;
        }
        if !res("SOUND") {
            let cap = if brdam > 40 { 35 } else { brdam * 3 / 4 + 5 };
            let k = rng.gen_range(1..=cap.max(1));
            crate::game::set_stun(gd, ps, ps.stun + k);
        }
        ps.hp -= brdam;
        harmful = true;
        it.pval = 1;
    }

    // Jellies are immune to their own acid.
    if def.ch.starts_with('j') && def.has("IM_ACID") {
        let dam = Dice {
            count: 8,
            sides: 8,
            bonus: 0,
        }
        .roll(rng);
        if !immune("ACID") && dam > 0 {
            let mut dmg = dam;
            if res("ACID") {
                dmg = (dmg + 2) / 3;
            }
            ps.hp -= dmg;
            log.add("The acidic remains burn you!");
        }
        harmful = true;
    }
    // Icky things are poisonous through and through.
    if def.ch.chars().next().is_some_and(|c| "ijkmS,".contains(c)) && def.has("IM_POIS") {
        if !res("POIS") {
            ps.poison += rng.gen_range(0..15) + 10;
        }
        harmful = true;
    }

    // Good effects only when nothing bad happened and the corpse is whole.
    let mut summons = Vec::new();
    if !harmful && !cutting && sval != SV_CORPSE_MEAT {
        for flag in ["IM_ACID", "IM_ELEC", "IM_FIRE", "IM_COLD", "IM_POIS"] {
            if def.has(flag) {
                ps.resist_timer += rng.gen_range(0..10) + 10;
            }
        }
        if def.has("RES_NETH") {
            ps.protevil += rng.gen_range(0..25) + 3 * def.depth as i32;
        }
        if def.has("RES_PLAS") {
            ps.resist_timer += rng.gen_range(0..20) + 20;
        }
        if def.has("NO_FEAR") {
            ps.fear = 0;
        }
        if def.has("NO_STUN") {
            ps.stun = 0;
        }
        if def.has("NO_CONF") {
            ps.confuse = 0;
        }
        for sp in &def.spells {
            if sp.starts_with("S_") {
                summons.push(sp.clone());
            }
        }
    }
    summons
}

/// Eat one bite of a corpse (cmd6.cc do_cmd_eat_food corpse branch).
/// Returns the friendly summons when a bite was taken; `None` when there
/// is not enough meat left. The caller keeps the item for `sval` 1
/// (corpse) and 3 (head), destroys it otherwise.
#[allow(clippy::too_many_arguments)]
pub fn eat_corpse(
    gd: &GameData,
    it: &mut Item,
    ps: &mut PlayerState,
    inv: &mut Inventory,
    totals: &EquipTotals,
    log: &mut MessageLog,
    rng: &mut impl Rng,
) -> Option<Vec<String>> {
    let sval = gd.objects[it.def].sval as i32;
    let def = gd.monsters.get(it.note as usize).cloned()?;
    // Freshness is the C++ `timeout`: 0 = a raw, fresh corpse, positive =
    // cured/preserved meat (do_cmd_cure_meat raises it).
    let fresh = it.timeout == 0;
    let res_pois = totals.resists.contains("POIS");
    if sval == SV_CORPSE_CORPSE {
        let mw = def.weight.max(1) as i32;
        let no_meat = if def.has("DROP_SKELETON") {
            it.pval2 <= mw * 3 / 5
        } else {
            it.pval2 <= mw * 7 / 20
        };
        if no_meat {
            log.add("There is not enough meat.");
            return None;
        }
        log.add(if fresh {
            "Ugh! Raw meat!"
        } else {
            "That tastes good."
        });
        it.pval -= 10;
        it.pval2 -= 10;
    } else if sval == SV_CORPSE_HEAD {
        log.add("You feel rather sick.");
        it.pval -= 10;
        it.pval2 -= 10;
    } else if sval == SV_CORPSE_MEAT {
        log.add(if fresh {
            "You quickly swallow the meat."
        } else {
            "That tastes good."
        });
        if fresh && it.pval2 > it.pval && !res_pois {
            let k = it.pval2 - it.pval;
            ps.poison += rng.gen_range(0..k.max(1)) + k;
        }
    }
    let summons = corpse_effect(gd, it, ps, inv, totals, false, log, rng);
    ps.food = (ps.food + if fresh { 2500 } else { 2000 }).min(crate::game::FOOD_MAX);
    if fresh && it.pval2 - it.pval > 10 && !res_pois {
        let k = it.pval2 - it.pval;
        ps.poison += rng.gen_range(0..k.max(1)) + k;
    }
    // Partially cured: the timeout shrinks without overflowing
    // (cmd6.cc:1403-1407).
    if it.pval2 > it.timeout && it.pval2 > 0 {
        it.timeout = (it.timeout * ((100 * it.timeout) / it.pval2)) / 100;
    }
    Some(summons)
}

/// Drink a potion / read a scroll. Returns true if the item is used up.
/// `pos` is the player position (mutated by teleportation effects).
/// Successful use identifies the item type (`inv.known`).
#[allow(clippy::too_many_arguments)]
pub fn use_object(
    gd: &GameData,
    def: usize,
    // Object pval2 (mimic shape of Morphic Oil; 0 when not applicable).
    pval2: i32,
    ps: &mut PlayerState,
    inv: &mut Inventory,
    map: &mut Map,
    pos: &mut GridPos,
    occupied: &HashSet<(i32, i32)>,
    log: &mut MessageLog,
    turn: &mut TurnState,
    next: &mut NextState<AppState>,
    rng: &mut impl Rng,
) -> bool {
    let o = &gd.objects[def];
    let name = o.name.as_str();
    match o.tval {
        data::TV_POTION | data::TV_POTION2 => {
            let totals = inv.totals_for(gd, ps);
            // Sanity cures (cmd6.cc SV_POTION2_CURE_*).
            if name.starts_with("Cure") && name.ends_with("Insanity") {
                let dice = if name.starts_with("Cure Critical") {
                    Dice {
                        count: 12,
                        sides: 8,
                        bonus: 0,
                    }
                } else if name.starts_with("Cure Serious") {
                    Dice {
                        count: 8,
                        sides: 8,
                        bonus: 0,
                    }
                } else if name.starts_with("Cure Light") {
                    Dice {
                        count: 4,
                        sides: 8,
                        bonus: 0,
                    }
                } else {
                    Dice {
                        count: 10,
                        sides: 100,
                        bonus: 0,
                    }
                };
                crate::game::heal_insanity(ps, dice.roll(rng), log);
                inv.learn(def);
                return true;
            }
            if name == "Curing" {
                // SV_POTION_CURING: heals 50 hp and clears all ills.
                heal(ps, 50, 0, log);
                ps.blind = 0;
                ps.poison = 0;
                ps.confuse = 0;
                ps.stun = 0;
                ps.cut = 0;
                ps.image = 0;
                crate::game::heal_insanity(ps, 50, log);
                inv.learn(def);
                return true;
            }
            if name.starts_with("Morphic Oil") {
                // Morphic Oil of <shape> (cmd6.cc SV_POTION2_MIMIC): no
                // skill check, no failure; a running shape is refreshed.
                if ps.mimic_turns == 0 {
                    let form = if pval2 > 0 { pval2 as u32 } else { 0 };
                    let form = if crate::mimic::MIMIC_FORMS
                        .get(form as usize)
                        .map(|f| f.enabled)
                        .unwrap_or(false)
                    {
                        form
                    } else {
                        0
                    };
                    let turns = crate::mimic::random_duration(form, rng);
                    crate::mimic::set_mimic(gd, ps, turns, form, (ps.level as i32 * 2) / 3, log);
                    crate::mimic::enforce_body(gd, ps, inv);
                }
                inv.learn(def);
                return true;
            }
            if name == "Water Curing" {
                // Quest item (q_poison): used by pouring it into the pond.
                log.add("Nothing happens.");
                inv.learn(def);
                return true;
            }
            if name.contains("*Healing*") {
                // *Healing*: 1200 hp and a full cure (cmd6.cc).
                heal(ps, 1200, 0, log);
                ps.cut = 0;
                ps.stun = 0;
                ps.poison = 0;
                ps.blind = 0;
                ps.confuse = 0;
                ps.fear = 0;
            } else if name == "Healing" {
                heal(ps, 300, 0, log);
                ps.cut = 0;
                ps.stun = 0;
                ps.poison = 0;
                ps.blind = 0;
                ps.confuse = 0;
                ps.fear = 0;
            } else if name == "Life" || name == "Blood of Life" || name == "New Life" {
                ps.hp = ps.max_hp;
                ps.cut = 0;
                ps.stun = 0;
                ps.poison = 0;
                ps.fear = 0;
                ps.blind = 0;
                ps.confuse = 0;
                for i in 0..6 {
                    restore_stat(ps, i, &totals, log);
                }
                if name == "Life" {
                    // Life cures the Black Breath (cmd6.cc:2009).
                    for it in inv.equip.iter_mut().flatten() {
                        it.flags.retain(|f| f != "BLACK_BREATH");
                    }
                    ps.black_breath = false;
                    log.add("The Black Breath is lifted from you.");
                }
                if name == "Blood of Life" {
                    ps.allow_one_death += 1;
                    log.add("You feel a hidden reserve of life within you.");
                }
                if name == "New Life" {
                    ps.exp_drained = 0;
                }
                log.add("You feel life flowing through you.");
            } else if name.starts_with("Cure Critical") {
                heal(
                    ps,
                    Dice {
                        count: 6,
                        sides: 8,
                        bonus: 10,
                    }
                    .roll(rng),
                    0,
                    log,
                );
                ps.cut = 0;
                ps.stun = 0;
                ps.confuse = 0;
                ps.blind = 0;
                ps.poison = 0;
            } else if name.starts_with("Cure Serious") {
                let cut = ps.cut / 2;
                heal(
                    ps,
                    Dice {
                        count: 4,
                        sides: 8,
                        bonus: 0,
                    }
                    .roll(rng),
                    cut,
                    log,
                );
                ps.confuse = 0;
            } else if name.starts_with("Cure Light") {
                heal(
                    ps,
                    Dice {
                        count: 2,
                        sides: 8,
                        bonus: 0,
                    }
                    .roll(rng),
                    10,
                    log,
                );
                ps.blind = 0;
            } else if name == "Curing" {
                ps.cut = 0;
                ps.stun = 0;
                ps.poison = 0;
                ps.blind = 0;
                ps.confuse = 0;
                heal(ps, 50, 0, log);
            } else if name.starts_with("Neutralise Poison") {
                ps.poison = 0;
                log.add("You are no longer poisoned.");
            } else if name.starts_with("Slow Poison") {
                ps.poison /= 2;
                log.add("The poison weakens.");
            } else if name.starts_with("Restore Mana") {
                ps.mana = ps.max_mana;
                log.add("You feel your head clear.");
            } else if name.starts_with("Restore Life Levels") {
                ps.exp += ps.exp_drained;
                if ps.exp_drained > 0 {
                    log.add("You feel your life force returning.");
                }
                ps.exp_drained = 0;
            } else if name.starts_with("Restore ") {
                let idx = match name {
                    n if n.contains("Strength") => 0,
                    n if n.contains("Intelligence") => 1,
                    n if n.contains("Wisdom") => 2,
                    n if n.contains("Dexterity") => 3,
                    n if n.contains("Constitution") => 4,
                    _ => 5,
                };
                restore_stat(ps, idx, &totals, log);
            } else if name == "Augmentation" {
                for i in 0..6 {
                    ps.stat_base[i] = crate::game::modify_stat_value(ps.stat_base[i], 1);
                    ps.stats[i] = crate::game::modify_stat_value(ps.stats[i], 1);
                }
                log.add("You feel yourself improving in every way!");
            } else if [
                "Strength",
                "Intelligence",
                "Wisdom",
                "Dexterity",
                "Constitution",
                "Charisma",
            ]
            .contains(&name)
            {
                let idx = match name {
                    "Strength" => 0,
                    "Intelligence" => 1,
                    "Wisdom" => 2,
                    "Dexterity" => 3,
                    "Constitution" => 4,
                    _ => 5,
                };
                ps.stat_base[idx] = crate::game::modify_stat_value(ps.stat_base[idx], 1);
                ps.stats[idx] = crate::game::modify_stat_value(ps.stats[idx], 1);
                log.add("You feel stronger in body and mind!");
            } else if name == "Boldness" {
                ps.fear = 0;
                log.add("You feel bold!");
            } else if name == "Heroism" {
                ps.fear = 0;
                ps.hero += rng.gen_range(25..=50);
                log.add("You feel like a hero!");
            } else if name == "Berserk Strength" {
                ps.fear = 0;
                ps.shero += rng.gen_range(25..=50);
                log.add("You feel like a slayer!");
            } else if name == "Speed" {
                // Speed only adds a little when already hasted (cmd6.cc).
                if ps.fast == 0 {
                    ps.fast = 16 + rng.gen_range(1..=24);
                } else {
                    ps.fast += 5;
                }
                log.add("You feel yourself moving faster!");
            } else if name == "Resistance" {
                ps.resist_timer += 20 + rng.gen_range(1..=20);
                log.add("You feel resistant to the elements!");
            } else if name == "Resist Heat" {
                ps.oppose_fire = ps.oppose_fire.max(10 + rng.gen_range(1..=10));
                log.add("You feel resistant to fire!");
            } else if name == "Resist Cold" {
                ps.oppose_cold = ps.oppose_cold.max(10 + rng.gen_range(1..=10));
                log.add("You feel resistant to cold!");
            } else if name == "Invulnerability" {
                // True invulnerability (set_invuln).
                ps.invuln = 7 + rng.gen_range(1..=6);
                log.add("You feel invincible!");
            } else if name.contains("Enlightenment") {
                for v in map.explored.iter_mut() {
                    *v = true;
                }
                log.add("An image of your surroundings forms in your mind.");
            } else if name == "Detect Invisible" {
                ps.tim_invis += 12 + rng.gen_range(1..=12);
                log.add("Your eyes tingle.");
            } else if name == "Infra-vision" {
                ps.tim_infra += 100 + rng.gen_range(1..=100);
                log.add("Your eyes tingle.");
            } else if name == "Invisibility" {
                ps.invis_turns += 20 + rng.gen_range(1..=30);
                ps.invis_power = ps.invis_power.max(20);
                log.add("You fade from sight.");
            } else if name == "Experience" {
                crate::game::gain_exp(ps, 100000, log, rng);
            } else if name == "Potion of Learning" {
                // Learning grants skill points (cmd6.cc).
                let gain = 4 + rng.gen_range(1..=6);
                ps.skill_points += gain;
                log.add(format!("You feel more learned! (+{} skill points)", gain));
            } else if name.starts_with("Detonations") {
                ps.stun += 75;
                crate::game::set_cut(gd, ps, ps.cut + 5000);
                hurt(
                    gd,
                    ps,
                    Dice {
                        count: 50,
                        sides: 20,
                        bonus: 0,
                    }
                    .roll(rng),
                    log,
                    next,
                );
            } else if name == "Death" {
                hurt(gd, ps, 5000, log, next);
            } else if name.starts_with("Poison") || name == "Sickliness" {
                ps.poison += Dice {
                    count: 2,
                    sides: 6,
                    bonus: 10,
                }
                .roll(rng);
                log.add("You feel sick!");
            } else if name == "Sleep" {
                // SV_POTION_SLEEP: rand_int(4)+4 unless free action.
                if !has_free_act(ps, inv, gd, &totals) {
                    ps.paralyze = 4 + rng.gen_range(0..4);
                    log.add("You fall asleep!");
                } else {
                    log.add("You resist the sleep!");
                }
            } else if name == "Slowness" {
                // SV_POTION_SLOWNESS: slow + randint(25) + 15.
                ps.slow += 15 + rng.gen_range(1..=25);
                log.add("You feel slow!");
            } else if name == "Blindness" {
                if totals.resists.contains("BLIND") {
                    log.add("You resist the blindness!");
                } else {
                    // SV_POTION_BLINDNESS: blind + rand_int(100) + 100.
                    ps.blind += 100 + rng.gen_range(0..100);
                    log.add("You are blinded!");
                }
            } else if name == "Weakness" {
                drain_stat(ps, 0, &totals, log);
            } else if name == "Stupidity" {
                drain_stat(ps, 1, &totals, log);
            } else if name == "Naivety" {
                drain_stat(ps, 2, &totals, log);
            } else if name == "Clumsiness" {
                drain_stat(ps, 3, &totals, log);
            } else if name == "Ugliness" {
                drain_stat(ps, 5, &totals, log);
            } else if name == "Ruination" {
                for i in 0..6 {
                    drain_stat(ps, i, &totals, log);
                }
            } else if name == "Corruption" {
                if crate::corrupt::gain_random(gd, ps, log, rng).is_none() {
                    log.add("Your spirit resists the corruption.");
                }
            } else if name == "Lose Memories" {
                // A quarter of recent experience is lost (cmd6.cc).
                let amount = ps.exp / 4;
                lose_exp(ps, amount, 75, &totals, rng, log);
                log.add("Your memories fade away!");
            } else if name == "Salt Water" {
                ps.food = -1;
                ps.poison = 0;
                ps.paralyze = ps.paralyze.max(4);
                log.add("You vomit!");
            } else if name == "Booze" {
                ps.food = (ps.food + 200).min(crate::game::FOOD_MAX);
                // Confusion is resisted like any confusion source
                // (cmd6.cc SV_POTION_CONFUSION).
                if !(totals.resists.contains("CONF") || totals.resists.contains("CHAOS")) {
                    ps.confuse += 15 + rng.gen_range(1..=20);
                    if rng.gen_bool(0.5) {
                        ps.image += rng.gen_range(5..=20);
                    }
                    if rng.gen_range(0..13) == 0 {
                        let mut rng2 = crate::rng::current();
                        crate::item::teleport(gd, map, pos, occupied, 100, &mut rng2);
                        ps.last_teleport = Some((pos.x, pos.y));
                        // wake up with no memory of the map (lose_all_info
                        // / wiz_dark; cave.cc:3769).
                        map.wiz_dark();
                        log.add("You wake up elsewhere with a sore head...");
                        log.add("You can't remember a thing, or how you got here!");
                    }
                }
                log.add("You feel tipsy.");
            } else if name == "Apple Juice" || name == "Water" {
                ps.food = (ps.food + 500).min(crate::game::FOOD_MAX);
                log.add("That was refreshing.");
            } else if name == "Slime Mold Juice" {
                ps.food = (ps.food + 1500).min(crate::game::FOOD_MAX);
                log.add("That was... nutritious?");
            } else {
                log.add("Nothing happens.");
            }
            inv.learn(def);
            true
        }
        data::TV_SCROLL => {
            if name.starts_with("Phase Door") {
                teleport(gd, map, pos, occupied, 10, rng);
                ps.last_teleport = Some((pos.x, pos.y));
                log.add("You phase through space.");
            } else if name.starts_with("Teleportation") {
                teleport(gd, map, pos, occupied, 100, rng);
                ps.last_teleport = Some((pos.x, pos.y));
                log.add("You teleport away.");
            } else if name.starts_with("Teleport Level") {
                teleport_player_level(gd, ps, turn, log, rng);
            } else if name.starts_with("Word of Recall") {
                if crate::game::recall_blocked(gd, ps, log) {
                    return true;
                }
                crate::game::start_recall(ps, log, rng);
            } else if name.starts_with("Light") {
                map::light_area(map, pos.x, pos.y, 2);
                log.add("You are surrounded by a white light.");
            } else if name.starts_with("Magic Mapping") {
                for v in map.explored.iter_mut() {
                    *v = true;
                }
                log.add("An image of your surroundings forms in your mind.");
            } else if name.starts_with("*Remove Curse*") {
                remove_all_curses(gd, inv, log, ps.level);
            } else if name.starts_with("Remove Curse") {
                remove_curses(gd, inv, log, ps.level);
            } else if name.starts_with("Enchant Weapon To-Hit") {
                enchant_weapon(gd, inv, true, false, 1, rng, log);
            } else if name.starts_with("Enchant Weapon To-Dam") {
                enchant_weapon(gd, inv, false, true, 1, rng, log);
            } else if name.starts_with("*Enchant Weapon*") {
                enchant_weapon(gd, inv, true, true, 2, rng, log);
            } else if name.starts_with("*Enchant Armour*") {
                enchant_armour(gd, inv, 2, rng, log);
            } else if name.starts_with("Enchant Armour") {
                enchant_armour(gd, inv, 1, rng, log);
            } else if name.starts_with("Identify") {
                // Handled by the modal (picks an item); never consumed here.
                return false;
            } else {
                log.add("Nothing happens.");
            }
            inv.learn(def);
            true
        }
        _ => {
            log.add("You cannot use that.");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::birth::make_player;
    use crate::data::load_game_data;

    #[test]
    fn monster_treasure_follows_the_drop_flags() {
        let gd = load_game_data();
        let mut created = std::collections::HashSet::new();
        let mut rng = crate::rng::current();
        // A monster with DROP_1D2 carries 1-2 entries (monster2.cc).
        let def = gd
            .monsters
            .iter()
            .find(|m| m.has("DROP_1D2") && !m.has("ONLY_ITEM") && !m.has("ONLY_GOLD"))
            .expect("DROP_1D2 monster")
            .clone();
        let mut gold = 0;
        let mut items = 0;
        for _ in 0..2000 {
            let (g, its) = monster_carried_treasure(&gd, &def, 20, &mut created, &mut rng);
            assert!(its.len() + (g > 0) as usize <= 2);
            assert!(its.len() + (g > 0) as usize >= 1);
            if g > 0 {
                assert!(g >= 7, "gold roll {} below the original minimum", g);
                gold += 1;
            }
            if !its.is_empty() {
                items += 1;
            }
        }
        assert!(gold > 0 && items > 0, "gold {gold}, items {items}");
        // A monster without any DROP_* flag carries nothing.
        let plain = gd
            .monsters
            .iter()
            .find(|m| {
                [
                    "DROP_60",
                    "DROP_90",
                    "DROP_1D2",
                    "DROP_2D2",
                    "DROP_3D2",
                    "DROP_4D2",
                    "DROP_RANDART",
                    "MIMIC",
                ]
                .iter()
                .all(|f| !m.has(f))
            })
            .expect("plain monster")
            .clone();
        for _ in 0..20 {
            let (g, its) = monster_carried_treasure(&gd, &plain, 20, &mut created, &mut rng);
            assert_eq!(g, 0);
            assert!(its.is_empty());
        }
        // ONLY_ITEM never yields gold; ONLY_GOLD always does.
        if let Some(d) = gd.monsters.iter().find(|m| m.has("ONLY_ITEM")).cloned() {
            for _ in 0..50 {
                let (g, _) = monster_carried_treasure(&gd, &d, 20, &mut created, &mut rng);
                assert_eq!(g, 0);
            }
        }
        if let Some(d) = gd
            .monsters
            .iter()
            .find(|m| m.has("ONLY_GOLD") && m.has("DROP_1D2"))
            .cloned()
        {
            let mut seen_gold = false;
            for _ in 0..50 {
                let (g, its) = monster_carried_treasure(&gd, &d, 20, &mut created, &mut rng);
                assert!(its.is_empty());
                seen_gold |= g > 0;
            }
            assert!(seen_gold);
        }
    }

    #[test]
    fn drop_randart_monsters_carry_an_artifact() {
        let gd = load_game_data();
        let def = gd
            .monsters
            .iter()
            .find(|m| m.has("DROP_RANDART"))
            .expect("DROP_RANDART monster")
            .clone();
        let mut created = std::collections::HashSet::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        use rand::SeedableRng;
        let mut artifacts = 0;
        for _ in 0..50 {
            let (_, its) = monster_carried_treasure(&gd, &def, 40, &mut created, &mut rng);
            artifacts += its.iter().filter(|it| it.artifact != 0).count();
        }
        assert!(artifacts > 0, "no randart carried: {artifacts}");
    }

    #[test]
    fn special_artifacts_roll_from_the_insta_pool() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(7);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..300 {
            let mut created = std::collections::HashSet::new();
            if let Some(it) = make_artifact_special(&gd, 90, &mut created, &mut rng) {
                let a = gd.artifacts.iter().find(|a| a.id == it.artifact).unwrap();
                assert!(a.insta_art);
                assert_ne!(a.id, 13, "The One Ring is quest-only");
                assert!(!a.flags.iter().any(|f| *f == "SPECIAL_GENE"));
                seen.insert(a.id);
            }
        }
        assert!(seen.len() >= 2, "no special artifacts rolled: {seen:?}");
    }

    #[test]
    fn artifact_drops_and_map_placements_come_from_the_data() {
        let gd = load_game_data();
        // Saruman's r_info A: line (Orthanc).
        let saruman = gd
            .monsters
            .iter()
            .find(|m| m.name.starts_with("Saruman"))
            .expect("Saruman");
        assert_eq!(saruman.artifact_idx, 202);
        assert_eq!(saruman.artifact_chance, 30);
        // The fixed-map artifact field of s_gates.map (Narya).
        let gates = gd.spec_level("s_gates.map").expect("s_gates");
        assert!(gates.artifacts.iter().any(|a| a.id == 10));
        // Every map-placed artifact must resolve to a ported kind.
        for sl in &gd.spec_levels {
            for a in &sl.artifacts {
                let def = gd
                    .artifacts
                    .iter()
                    .find(|d| d.id == a.id)
                    .unwrap_or_else(|| panic!("artifact {} missing ({})", a.id, sl.name));
                assert!(
                    gd.object_by_tval_sval(def.tval, def.sval).is_some(),
                    "artifact {} has no ported base kind",
                    a.id
                );
            }
        }
    }

    fn town_map() -> Map {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        {
            let wild = crate::game::Wilderness::new(&gd, &mut rng);
            let (wx, wy) = gd.town_cell(1);
            map::generate_town(
                &gd,
                &crate::game::PlotQuest::default(),
                &wild,
                wx,
                wy,
                1,
                0,
                true,
                &mut rng,
            )
            .map
        }
    }

    #[test]
    fn gen_object_respects_depth() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // The allocator picks A: rows, whose locale may sit far above the
        // requested level after the 1/20 GREATE_OBJ level boost; every
        // result must at least be a legal allocation-table kind.
        for _ in 0..200 {
            let d = gen_object_kind(&gd, 1, &mut rng).expect("some object");
            let o = &gd.objects[d];
            assert!(!o.alloc.is_empty(), "{} has no A: rows", o.name);
            assert_ne!(o.tval, 0);
            assert!(!o.flags.iter().any(|f| f == "SPECIAL_GENE"));
            assert_ne!(o.tval, data::TV_HYPNOS);
        }
    }

    #[test]
    fn equipment_totals_work() {
        let gd = load_game_data();
        let mut inv = Inventory::default();
        assert_eq!(inv.totals(&gd).lite, 0);
        let torch = gd.object_by_name("Wooden Torch").unwrap();
        inv.equip[data::SLOT_LITE] = Some(Item::base(&gd, torch));
        assert_eq!(inv.totals(&gd).lite, 1);
        let sword = gd.object_by_name("Long Sword").unwrap();
        inv.equip[data::SLOT_WEAPON] = Some(Item::base(&gd, sword));
        assert!(gd.objects[sword].dice != "0d0");
        let armor = gd.object_by_name("Soft Leather Armour").unwrap();
        inv.equip[data::SLOT_BODY] = Some(Item::base(&gd, armor));
        assert!(inv.totals(&gd).ac > 0);
    }

    #[test]
    fn wraith_form_ac_and_reflect_depend_on_embodiment() {
        let gd = load_game_data();
        let warrior = gd.classes.iter().position(|c| c.name == "Warrior").unwrap();
        let mut ps = make_player(&gd, "Wraith".to_string(), 0, warrior);
        let inv = Inventory::default();
        let base = inv.totals_for(&gd, &ps).ac;
        ps.tim_wraith = 5;
        // Embodied: +50 AC and reflection (xtra1.cc:3208).
        let embodied = inv.totals_for(&gd, &ps);
        assert_eq!(embodied.ac, base + 50);
        assert!(embodied.wraith && embodied.reflect);
        // Disembodied: +10 AC, no reflection (xtra1.cc:3202).
        ps.tim_wraith = 0;
        ps.disembodied = true;
        let spirit_base = inv.totals_for(&gd, &ps).ac;
        ps.tim_wraith = 5;
        let spirit = inv.totals_for(&gd, &ps);
        assert_eq!(spirit.ac, spirit_base + 10);
        assert!(spirit.wraith && !spirit.reflect);
    }

    #[test]
    fn race_and_class_level_flags_apply() {
        let gd = load_game_data();
        let ent = gd.races.iter().position(|r| r.name == "Ent").unwrap();
        let warrior = gd.classes.iter().position(|c| c.name == "Warrior").unwrap();
        let mut ps = make_player(&gd, "Flag Test".to_string(), ent, warrior);
        let inv = Inventory::default();
        // Ent level 1: SPEED -5 plus SENS_FIRE/SLOW_DIGEST.
        let t = inv.totals_for(&gd, &ps);
        assert!(t.sens_fire);
        assert!(t.slow_digest);
        assert_eq!(t.speed, -5);
        // Ent level 20 gains ESP_EVIL/ORC/TROLL; Warrior's RES_FEAR at 30.
        ps.level = 30;
        ps.class_name = "Warrior".to_string();
        let t = inv.totals_for(&gd, &ps);
        assert!(t.esp.contains("EVIL") && t.esp.contains("ORC"));
        assert!(t.resists.contains("FEAR"));
        // RohanKnight stacks SPEED at each milestone (3 + 1 + 1 at 10).
        ps.race_name = "RohanKnight".to_string();
        ps.level = 10;
        assert_eq!(inv.totals_for(&gd, &ps).speed, 5);
    }

    #[test]
    fn corpse_effect_discharges_breath_and_summons() {
        use rand::SeedableRng;
        let mut gd = load_game_data();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let cd = gd.object_by_tval_sval(crate::data::TV_CORPSE, 1).unwrap();
        let warrior = gd.classes.iter().position(|c| c.name == "Warrior").unwrap();
        let mut ps = make_player(&gd, "Corpse".to_string(), 0, warrior);
        let mut inv = Inventory::default();
        let totals = inv.totals_for(&gd, &ps);
        let mut log = MessageLog::default();

        // Fire hound: a whole-corpse bite discharges its breath.
        let fire = gd
            .monsters
            .iter()
            .position(|m| m.name == "Fire hound")
            .unwrap();
        let mut corpse = Item::base(&gd, cd);
        init_corpse(&gd, &mut corpse, fire, 1000, &mut rng);
        assert!(corpse.pval > 0 && corpse.pval2 > 0 && corpse.note as usize == fire);
        let hp = ps.hp;
        corpse_effect(
            &gd,
            &mut corpse,
            &mut ps,
            &mut inv,
            &totals,
            true,
            &mut log,
            &mut rng,
        );
        assert!(ps.hp < hp, "fire breath should hurt a level-1 human");

        // A harmless summoner's corpse grants friendly summons.
        let base = gd
            .monsters
            .iter()
            .find(|m| m.name == "Fire hound")
            .unwrap()
            .clone();
        let idx = gd.monsters.len();
        let mut summoner = base;
        summoner.blows.clear();
        summoner.flags.clear();
        summoner.spells = vec!["S_KIN".to_string()];
        gd.monsters.push(summoner);
        let mut corpse2 = Item::base(&gd, cd);
        init_corpse(&gd, &mut corpse2, idx, 1000, &mut rng);
        let summons = corpse_effect(
            &gd,
            &mut corpse2,
            &mut ps,
            &mut inv,
            &totals,
            false,
            &mut log,
            &mut rng,
        );
        assert_eq!(summons, vec!["S_KIN".to_string()]);

        // Too little meat left: eating is refused.
        corpse2.pval = 1;
        corpse2.pval2 = 1;
        assert!(eat_corpse(
            &gd,
            &mut corpse2,
            &mut ps,
            &mut inv,
            &totals,
            &mut log,
            &mut rng
        )
        .is_none());
    }

    #[test]
    fn quiver_holds_ammo_without_melee_bonus() {
        let gd = load_game_data();
        assert_eq!(data::slot_of(data::TV_ARROW), Some(data::SLOT_QUIVER));
        let arrow = gd.object_by_name("Arrow").expect("Arrow");
        let mut inv = Inventory::default();
        let mut a = Item::base(&gd, arrow);
        a.to_h = 5;
        a.to_d = 7;
        inv.equip[data::SLOT_QUIVER] = Some(a);
        let t = inv.totals(&gd);
        assert_eq!(t.to_h, 0);
        assert_eq!(t.to_d, 0);
    }

    #[test]
    fn rod_mains_and_tips_are_craftable() {
        let gd = load_game_data();
        let main = gd
            .object_by_tval_sval(crate::data::TV_ROD_MAIN, 10)
            .expect("Wooden Rod");
        let it = Item::base(&gd, main);
        assert_eq!(it.pval, 0);
        assert_eq!(it.pval2, 10);
        assert_eq!(it.timeout, 10);
        let tip = gd
            .object_by_tval_sval(crate::data::TV_ROD, 22)
            .expect("Fire Bolts tip");
        let t = Item::base(&gd, tip);
        assert!(t.pval2 > 0);
        assert!(gd.spell_by_name(&gd.objects[tip].name).is_some());
    }

    #[test]
    fn junkarts_finalize_unique_names_and_activations() {
        let gd = load_game_data();
        assert_eq!(gd.randarts.junk_f.len(), 84);
        assert_eq!(gd.randarts.acts.len(), 51);
        let def = gd
            .object_by_tval_sval(crate::data::TV_RANDART, 0)
            .expect("Random Artifact");
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        let a = make_item(&gd, def, 10, false, &mut created, &mut rng);
        let b = make_item(&gd, def, 10, false, &mut created, &mut rng);
        assert_ne!(a.note, b.note);
        assert!(!a.artifact_name.is_empty());
        assert_eq!(a.label(&gd, &HashSet::new()), a.artifact_name);
        let act = &gd.randarts.acts[a.pval2.max(0) as usize];
        let (row, _) = crate::spell::artifact_activation(&act.act);
        assert_ne!(row.kind, "fizzle", "{} is unmapped", act.act);
    }

    #[test]
    fn junkart_colours_follow_the_per_game_table() {
        let gd = load_game_data();
        let ps = crate::birth::make_player(&gd, "Colour".into(), 0, 0);
        // init_randart rolls one colour per junkart (birth.cc:475).
        assert_eq!(ps.junkart_colors.len(), gd.randarts.junk_s.len());
        assert!(ps
            .junkart_colors
            .iter()
            .all(|&c| (1..=15).contains(&c)));
        let def = gd
            .object_by_tval_sval(crate::data::TV_RANDART, 0)
            .expect("Random Artifact");
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        let a = make_item(&gd, def, 10, false, &mut created, &mut rng);
        // object_attr returns the table entry, not the kind's colour.
        assert_eq!(object_attr(&gd, &a, &ps), ps.junkart_colors[a.note as usize]);
        // The colour survives a save/load round trip with the player.
        let text = ron::ser::to_string(&ps).unwrap();
        let back: crate::game::PlayerState = ron::from_str(&text).unwrap();
        assert_eq!(back.junkart_colors, ps.junkart_colors);
    }

    #[test]
    fn ego_items_get_named_and_curses_stick() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let sword = gd.object_by_name("Long Sword").unwrap();
        // Force ego rolls until one applies.
        let mut saw_ego = false;
        let mut saw_curse = false;
        for _ in 0..200 {
            let mut created = HashSet::new();
            let item = make_item(&gd, sword, 10, true, &mut created, &mut rng);
            if item.ego != 0 {
                saw_ego = true;
                let known = HashSet::new();
                let mut labeled = item.clone();
                labeled.identified = true;
                let s = labeled.label(&gd, &known);
                assert!(s.len() > "Long Sword".len(), "{}", s);
            }
            if item.cursed {
                saw_curse = true;
            }
            if saw_ego && saw_curse {
                break;
            }
        }
        assert!(saw_ego, "no ego generated in 200 good rolls");
    }

    #[test]
    fn devices_get_charges_and_lites_get_fuel() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut created = HashSet::new();
        let wand = gd.object_by_name("Manathrust").unwrap();
        let w = make_item(&gd, wand, 5, false, &mut created, &mut rng);
        assert!(w.charges > 0);
        let torch = gd.object_by_name("Wooden Torch").unwrap();
        let t = Item::base(&gd, torch);
        assert_eq!(t.fuel, 4000);
    }

    #[test]
    fn cure_light_wounds_heals() {
        let gd = load_game_data();
        let mut map = town_map();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        ps.hp = 1;
        let mut inv = Inventory::default();
        let mut pos = GridPos { x: 5, y: 5 };
        let mut log = MessageLog::default();
        let mut turn = TurnState::default();
        let mut next = NextState::<AppState>::default();
        let mut rng = crate::rng::current();
        let potion = gd.object_by_name("Cure Light Wounds").unwrap();
        let used = use_object(
            &gd,
            potion,
            0,
            &mut ps,
            &mut inv,
            &mut map,
            &mut pos,
            &HashSet::new(),
            &mut log,
            &mut turn,
            &mut next,
            &mut rng,
        );
        assert!(used);
        assert!(ps.hp > 1, "hp: {}", ps.hp);
        assert!(inv.known.contains(&potion), "potion type should be known");
    }

    #[test]
    fn phaseDoor_teleports_and_recall_changes_level() {
        let gd = load_game_data();
        let mut map = town_map();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        ps.depth = 3;
        let mut inv = Inventory::default();
        let start = GridPos { x: 40, y: 30 };
        let mut pos = start;
        let mut log = MessageLog::default();
        let mut turn = TurnState::default();
        let mut next = NextState::<AppState>::default();
        let mut rng = crate::rng::current();
        let scroll = gd.object_by_name("Phase Door").unwrap();
        use_object(
            &gd,
            scroll,
            0,
            &mut ps,
            &mut inv,
            &mut map,
            &mut pos,
            &HashSet::new(),
            &mut log,
            &mut turn,
            &mut next,
            &mut rng,
        );
        assert!(map.walkable(&gd, pos.x, pos.y));
        assert!(
            map::chebyshev(start.x, start.y, pos.x, pos.y) <= 10,
            "teleported too far: {:?}",
            pos
        );
        let recall = gd.object_by_name("Word of Recall").unwrap();
        use_object(
            &gd,
            recall,
            0,
            &mut ps,
            &mut inv,
            &mut map,
            &mut pos,
            &HashSet::new(),
            &mut log,
            &mut turn,
            &mut next,
            &mut rng,
        );
        // The recall charges for a while before it fires.
        assert!(ps.word_recall > 0);
        ps.word_recall = 1;
        crate::game::tick_word_recall(&gd, &mut ps, &mut turn, &mut log);
        assert!(matches!(turn.pending, Some(crate::game::Goto::Surface)));
        assert_eq!(ps.recall_depth, 3);
    }

    #[test]
    fn new_item_kinds_and_inscriptions_load() {
        let gd = load_game_data();
        for name in [
            "Empty Bottle",
            "Iron Spike",
            "Lembas",
            "Pint of Fine Ale",
            "Pint of Fine Wine",
            "Greater Ration of Health",
            "Fortune cookie",
        ] {
            assert!(gd.object_by_name(name).is_some(), "{} missing", name);
        }
        let def = gd.object_by_name("Iron Spike").unwrap();
        let mut it = Item::base(&gd, def);
        assert_eq!(gd.objects[def].tval, data::TV_SPIKE);
        it.inscription = "!d".to_string();
        assert!(it.label(&gd, &Default::default()).contains("{!d}"));
    }

    #[test]
    fn eating_food_restores_nutrition() {
        let gd = load_game_data();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        ps.food = 100;
        let mut inv = Inventory::default();
        let mut log = MessageLog::default();
        let mut rng = crate::rng::current();
        let ration = gd.object_by_name("Ration of Food").unwrap();
        eat_food(&gd, ration, &mut ps, &mut inv, &mut log, &mut rng);
        assert_eq!(ps.food, 100 + 5000);
    }

    #[test]
    fn tval_desc_covers_the_original_table() {
        let staff = tval_desc(data::TV_MSTAFF).expect("Mage Staves have a blurb");
        assert!(staff.contains("spellcasting time to 80%"));
        assert!(tval_desc(data::TV_SWORD).is_some_and(|s| s.contains("melee weapons")));
        assert!(tval_desc(9999).is_none());
    }

    #[test]
    fn aule_accepts_only_selfmade_items() {
        let gd = load_game_data();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut it = Item::base(&gd, sword);
        assert!(!item_tester_hook_sacrifice_aule(&it));
        it.found = OBJ_FOUND_SELFMADE;
        assert!(item_tester_hook_sacrifice_aule(&it));
    }

    #[test]
    fn labels_map_to_inventory_and_equipment() {
        assert_eq!(label_to_inven('a', 3), Some(0));
        assert_eq!(label_to_inven('c', 3), Some(2));
        assert_eq!(label_to_inven('d', 3), None);
        assert_eq!(label_to_inven('A', 3), None);
        assert_eq!(label_to_equip('a', 12), Some(0));
        assert_eq!(label_to_equip('l', 12), Some(11));
        assert_eq!(label_to_equip('m', 12), None);
        assert_eq!(label_to_equip('A', 12), None);
    }

    #[test]
    fn wearable_p_follows_the_original_tval_switch() {
        let gd = load_game_data();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let potion = gd.object_by_name("Cure Light Wounds").unwrap();
        assert!(wearable_p(&gd, &Item::base(&gd, sword)));
        assert!(!wearable_p(&gd, &Item::base(&gd, potion)));
    }

    #[test]
    fn standard_artifact_predicate_excludes_randarts() {
        let gd = load_game_data();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut it = Item::base(&gd, sword);
        assert!(!is_standard_artifact(&it));
        it.artifact = 5;
        assert!(is_standard_artifact(&it));
        it.artifact = 0;
        it.artifact_name = "of Testing".to_string();
        assert!(!is_standard_artifact(&it), "a randart is not IsArtifact");
        assert!(is_artifact(&gd, &it), "but it is artifact_p");
    }

    #[test]
    fn artifacts_are_unique_per_game() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut created = HashSet::new();
        // Generate artifacts until every Long Sword artifact is created;
        // afterwards no more can appear. The original roll is per-candidate
        // with out-of-depth and rarity gates, so deep/rare ones take many
        // tries.
        let mut ids: HashSet<u32> = HashSet::new();
        for _ in 0..200_000 {
            let item = make_item(&gd, sword, 50, true, &mut created, &mut rng);
            if item.artifact != 0 {
                assert!(ids.insert(item.artifact), "duplicate artifact!");
            }
        }
        let possible = gd
            .artifacts
            .iter()
            .filter(|a| {
                !a.insta_art
                    && !a.flags.iter().any(|f| f == "SPECIAL_GENE")
                    && gd.object_by_tval_sval(a.tval, a.sval) == Some(sword)
            })
            .count();
        if possible > 0 {
            assert_eq!(created.len(), possible);
            for _ in 0..100 {
                let item = make_item(&gd, sword, 50, true, &mut created, &mut rng);
                assert_eq!(item.artifact, 0, "artifact after all created");
            }
        }
    }

    #[test]
    fn insta_art_artifacts_are_quest_only() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut created = HashSet::new();
        for def in 0..gd.objects.len() {
            let o = &gd.objects[def];
            if !o.wearable() {
                continue;
            }
            for _ in 0..30 {
                let it = make_item(&gd, def, 90, true, &mut created, &mut rng);
                if it.artifact != 0 {
                    let a = gd.artifacts.iter().find(|a| a.id == it.artifact).unwrap();
                    assert!(!a.insta_art, "random insta artifact {}", a.id);
                    assert!(
                        !a.flags.iter().any(|f| f == "SPECIAL_GENE"),
                        "random special-gene artifact {}",
                        a.id
                    );
                }
            }
        }
        // The One Ring in particular only comes from the Sauron quest.
        let ring = gd.artifacts.iter().find(|a| a.id == 13).expect("One Ring");
        assert!(ring.insta_art);
        assert!(gd
            .objects
            .iter()
            .any(|o| o.tval == ring.tval && o.sval == ring.sval));
    }

    #[test]
    fn enchant_plus_follows_the_enchant_table() {
        let mut rng = crate::rng::current();
        // +0 has a 0% failure chance; +15 and beyond are impossible.
        assert_eq!(enchant_plus(0, &mut rng), Some(1));
        assert_eq!(enchant_plus(15, &mut rng), None);
        assert_eq!(enchant_plus(16, &mut rng), None);
        // +5 succeeds some of the time (chance 300/1000).
        let mut succeeded = false;
        for _ in 0..200 {
            if enchant_plus(5, &mut rng).is_some() {
                succeeded = true;
                break;
            }
        }
        assert!(succeeded, "a +5 item must be enchantable sometimes");
    }

    #[test]
    fn fire_breath_burns_scrolls_but_not_artifacts() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut inv = Inventory::default();
        let scroll = gd.object_by_name("Phase Door").unwrap();
        for _ in 0..4 {
            inv.pack.push(Item::base(&gd, scroll));
        }
        let mut art = Item::base(&gd, scroll);
        art.artifact = 1; // artifacts are indestructible
        inv.pack.push(art);
        let mut log = MessageLog::default();
        // Run until every plain scroll burns (the original per-unit 2-3%
        // chance replaced the old flat 25%).
        let mut calls = 0;
        while inv.pack.len() > 1 && calls < 100_000 {
            inven_damage(&gd, &mut inv, "FIRE", &mut log, &mut rng);
            calls += 1;
        }
        assert_eq!(inv.pack.len(), 1, "only the artifact survives");
        assert_eq!(inv.pack[0].artifact, 1);
    }

    #[test]
    fn slays_and_brands_follow_tot_dam_aux() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let orc = (0..gd.monsters.len())
            .find(|&i| gd.monsters[i].has("ORC"))
            .unwrap();
        let dragon = (0..gd.monsters.len())
            .find(|&i| gd.monsters[i].has("DRAGON"))
            .unwrap();
        // SLAY_ORC is x2-3 vs orcs, useless vs dragons.
        let slays: HashSet<String> = ["SLAY_ORC".to_string()].into_iter().collect();
        let (m1, _, _) = dam_aux(&slays, &gd.monsters[orc], &mut rng);
        assert_eq!(m1, 3);
        let (m2, _, _) = dam_aux(&slays, &gd.monsters[dragon], &mut rng);
        assert_eq!(m2, 1);
        // KILL_DRAGON is x5 vs dragons.
        let slays: HashSet<String> = ["KILL_DRAGON".to_string()].into_iter().collect();
        let (m3, _, _) = dam_aux(&slays, &gd.monsters[dragon], &mut rng);
        assert_eq!(m3, 5);
        // Fire brand: x3 normally, nothing against fire-immune monsters.
        let immune = (0..gd.monsters.len())
            .find(|&i| gd.monsters[i].has("IM_FIRE"))
            .unwrap();
        let slays: HashSet<String> = ["BRAND_FIRE".to_string()].into_iter().collect();
        let (m4, _, _) = dam_aux(&slays, &gd.monsters[orc], &mut rng);
        assert_eq!(m4, 3);
        let (m5, _, _) = dam_aux(&slays, &gd.monsters[immune], &mut rng);
        assert_eq!(m5, 1);
        // Susceptible monsters take x6.
        let suscep = (0..gd.monsters.len())
            .find(|&i| gd.monsters[i].has("SUSCEP_FIRE"))
            .unwrap();
        let (m6, _, _) = dam_aux(&slays, &gd.monsters[suscep], &mut rng);
        assert_eq!(m6, 6);
    }

    #[test]
    fn gf_respects_immunity_resist_and_sustains() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        let mut inv = Inventory::default();
        let mut log = MessageLog::default();
        let mut totals = crate::item::EquipTotals::default();
        // Immunity nullifies the damage entirely.
        totals.immunities.insert("FIRE".to_string());
        let d = crate::game::apply_gf(
            &gd, &mut ps, &mut inv, &totals, "FIRE", 90, &mut log, &mut rng,
        );
        assert_eq!(d, 0);
        // Resistance cuts it to a third.
        let mut totals2 = crate::item::EquipTotals::default();
        totals2.resists.insert("FIRE".to_string());
        let d = crate::game::apply_gf(
            &gd, &mut ps, &mut inv, &totals2, "FIRE", 90, &mut log, &mut rng,
        );
        assert!(d <= 31, "resisted damage: {}", d);
        // Sustains block stat drains.
        totals2.sustains.insert("STR".to_string());
        let str_before = ps.stats[0];
        crate::item::drain_stat(&mut ps, 0, &totals2, &mut log);
        assert_eq!(ps.stats[0], str_before);
        // Unsustained stats drain.
        crate::item::drain_stat(&mut ps, 0, &crate::item::EquipTotals::default(), &mut log);
        assert_eq!(ps.stats[0], str_before - 1);
        // HOLD_LIFE blocks most experience drains.
        ps.exp = 10000;
        let mut totals3 = crate::item::EquipTotals::default();
        totals3.hold_life = true;
        let mut blocked = 0;
        for _ in 0..20 {
            let before = ps.exp;
            crate::item::lose_exp(&mut ps, 500, 95, &totals3, &mut rng, &mut log);
            if ps.exp == before {
                blocked += 1;
            }
        }
        assert!(blocked > 10, "hold life blocked {} of 20", blocked);
    }

    #[test]
    fn heavy_curses_resist_removal_and_perma_never_lifts() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let heavy_ego = gd
            .egos
            .iter()
            .find(|e| e.flags.iter().any(|f| f == "HEAVY_CURSE"));
        let perma_ego = gd
            .egos
            .iter()
            .find(|e| e.flags.iter().any(|f| f == "PERMA_CURSE"));
        let mut inv = Inventory::default();
        let mut log = MessageLog::default();
        if let Some(e) = heavy_ego {
            let mut it = Item::base(&gd, sword);
            it.ego = e.id;
            it.cursed = true;
            inv.equip[crate::data::SLOT_WEAPON] = Some(it);
        }
        if let Some(e) = perma_ego {
            let mut it = Item::base(&gd, sword);
            it.ego = e.id;
            it.cursed = true;
            inv.equip[crate::data::SLOT_BODY] = Some(it);
        }
        for _ in 0..10 {
            remove_curses_ex(&gd, &mut inv, &mut log, 1, false, &mut rng);
        }
        if perma_ego.is_some() {
            assert!(inv.equip[crate::data::SLOT_BODY].as_ref().unwrap().cursed);
        }
    }

    #[test]
    fn jewelry_generation_follows_a_m_aux_3() {
        use rand::SeedableRng;
        let gd = load_game_data();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut created = HashSet::new();
        // Ring of Speed: non-zero pval, positive when not cursed.
        let speed = gd.object_by_name("Ring of Speed").expect("speed ring");
        let mut saw_positive = false;
        for _ in 0..100 {
            let it = make_item(&gd, speed, 50, false, &mut created, &mut rng);
            if it.ego == 0 && it.artifact == 0 && it.artifact_name.is_empty() {
                assert!(it.pval != 0, "speed pval {}", it.pval);
                if !it.cursed {
                    assert!(it.pval > 0);
                    saw_positive = true;
                }
            }
        }
        assert!(saw_positive, "no plain uncursed speed ring rolled");
        // Ring of Weakness is always cursed with a negative pval.
        let weak = gd
            .object_by_name("Ring of Weakness")
            .expect("weakness ring");
        for _ in 0..50 {
            let it = make_item(&gd, weak, 1, false, &mut created, &mut rng);
            if it.ego == 0 && it.artifact == 0 && it.artifact_name.is_empty() {
                assert!(it.cursed && it.pval < 0, "{:?}", it);
                break;
            }
        }
        // Ring of Flames: AC bonus and a fire activation.
        let flames = gd.object_by_name("Ring of Flames").expect("flames ring");
        let it = make_item(&gd, flames, 50, true, &mut created, &mut rng);
        if it.ego == 0 && it.artifact == 0 && it.artifact_name.is_empty() {
            assert!(it.to_a > 0, "flames to_a {}", it.to_a);
        }
        assert_eq!(gd.objects[flames].activate, "BA_FIRE_4");
        // Amulet of Doom is always cursed.
        let doom = gd.object_by_name("Amulet of Doom").expect("doom amulet");
        for _ in 0..50 {
            let it = make_item(&gd, doom, 50, false, &mut created, &mut rng);
            if it.ego == 0 && it.artifact == 0 && it.artifact_name.is_empty() {
                assert!(it.cursed, "plain amulet of doom not cursed");
                break;
            }
        }
    }

    #[test]
    fn jewelry_base_flags_flow_into_totals() {
        let gd = load_game_data();
        let mut inv = Inventory::default();
        // EASY_KNOW kinds are identified on sight.
        let fa = gd.object_by_name("Ring of Free Action").expect("fa ring");
        let it = Item::base(&gd, fa);
        assert!(it.identified, "EASY_KNOW ring not identified");
        inv.equip[crate::data::SLOT_RING1] = Some(it);
        assert!(inv.totals(&gd).free_act);
        // Sustain rings sustain via their base flags.
        let sus = gd
            .object_by_name("Ring of Sustain Strength")
            .expect("sustain ring");
        inv.equip[crate::data::SLOT_RING2] = Some(Item::base(&gd, sus));
        assert!(inv.totals(&gd).sustains.contains("STR"));
        // A climbing set works from the pack.
        let climb = gd.object_by_name("Climbing Set").expect("climbing set");
        inv.pack.push(Item::base(&gd, climb));
        assert!(inv.totals(&gd).climb);
        // Digging tools carry their TUNNEL pval * 20 (xtra1.cc:2498).
        let mattock = gd.object_by_name("Mattock").expect("mattock");
        inv.equip[crate::data::SLOT_WEAPON] = Some(Item::base(&gd, mattock));
        assert_eq!(inv.totals(&gd).tunnel, 60);
    }

    #[test]
    fn ego_generation_powers_and_crit_totals() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // SUSTAIN egos sustain one random stat; OLD_RESIST adds a high
        // resist (instance flags, ego_flag_list powers).
        let sus = gd
            .egos
            .iter()
            .find(|e| e.flags.iter().any(|f| f == "SUSTAIN"))
            .cloned();
        let old = gd
            .egos
            .iter()
            .find(|e| e.flags.iter().any(|f| f == "OLD_RESIST"))
            .cloned();
        assert!(sus.is_some() && old.is_some(), "SUSTAIN/OLD_RESIST egos");
        let base_of = |e: &crate::data::EgoDef| {
            gd.objects
                .iter()
                .position(|o| o.tval == e.tvals[0])
                .expect("base item for ego tval")
        };
        if let Some(e) = sus {
            let base = base_of(&e);
            let mut saw = false;
            for _ in 0..500 {
                let mut it = Item::base(&gd, base);
                apply_ego(&gd, &mut it, 50, true, &mut rng);
                if it.ego == e.id {
                    assert!(
                        it.flags.iter().any(|f| f.starts_with("SUST_")),
                        "SUSTAIN flags {:?}",
                        it.flags
                    );
                    saw = true;
                    break;
                }
            }
            assert!(saw, "SUSTAIN ego never rolled");
        }
        if let Some(e) = old {
            let base = base_of(&e);
            let mut saw = false;
            for _ in 0..500 {
                let mut it = Item::base(&gd, base);
                apply_ego(&gd, &mut it, 50, true, &mut rng);
                if it.ego == e.id {
                    assert!(
                        it.flags.iter().any(|f| f.starts_with("RES_")),
                        "OLD_RESIST flags {:?}",
                        it.flags
                    );
                    saw = true;
                    break;
                }
            }
            assert!(saw, "OLD_RESIST ego never rolled");
        }
        // ETR_ABILITY grants one random minor ability (object2.cc).
        let abil = gd
            .egos
            .iter()
            .find(|e| e.flags.iter().any(|f| f == "ABILITY"))
            .cloned();
        assert!(abil.is_some(), "ABILITY egos");
        if let Some(e) = abil {
            const ABILITIES: [&str; 8] = [
                "FEATHER",
                "LITE1",
                "SEE_INVIS",
                "ESP_ALL",
                "SLOW_DIGEST",
                "REGEN",
                "FREE_ACT",
                "HOLD_LIFE",
            ];
            let base = base_of(&e);
            let mut saw = false;
            for _ in 0..500 {
                let mut it = Item::base(&gd, base);
                apply_ego(&gd, &mut it, 50, true, &mut rng);
                if it.ego == e.id {
                    assert!(
                        it.flags.iter().any(|f| ABILITIES.contains(&f.as_str())),
                        "ABILITY flags {:?}",
                        it.flags
                    );
                    saw = true;
                    break;
                }
            }
            assert!(saw, "ABILITY ego never rolled");
        }
        // CRIT pval feeds the critical-hit roll via EquipTotals.
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut inv = Inventory::default();
        let mut ring = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        ring.flags.push("CRIT".to_string());
        ring.pval = 7;
        inv.equip[crate::data::SLOT_RING1] = Some(ring);
        assert_eq!(inv.totals(&gd).crit, 7);
        // DRAIN_HP/DRAIN_MANA count pieces.
        let mut cursed = Item::base(&gd, sword);
        cursed.flags.push("DRAIN_HP".to_string());
        cursed.flags.push("DRAIN_MANA".to_string());
        inv.equip[crate::data::SLOT_BODY] = Some(cursed);
        let t = inv.totals(&gd);
        assert!(t.drain_hp >= 1 && t.drain_mana >= 1);
    }

    #[test]
    fn extra_limb_slots_exist_and_hold_gear() {
        let gd = load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let class = gd.classes.iter().position(|c| c.name == "Warrior").unwrap();
        let mut ps = make_player(&gd, "Limb Test".to_string(), race, class);
        let sword = gd.object_by_name("Long Sword").expect("sword");
        let mut inv = Inventory::default();
        // The extra slots do not exist without extra limbs (xtra1.cc
        // calc_body_bonus).
        assert!(!crate::mimic::slot_usable(&ps, data::SLOT_WEAPON2));
        assert!(!crate::mimic::slot_usable(&ps, data::SLOT_FEET2));
        assert_eq!(
            data::second_slot_of(crate::data::TV_SWORD),
            Some(data::SLOT_WEAPON2)
        );
        assert_eq!(
            data::second_slot_of(crate::data::TV_BOOTS),
            Some(data::SLOT_FEET2)
        );
        ps.mimic_extra = crate::mimic::CLASS_ARMS | crate::mimic::CLASS_LEGS;
        ps.mimic_extra_turns = 10;
        for slot in [
            data::SLOT_WEAPON2,
            data::SLOT_SHIELD2,
            data::SLOT_HANDS2,
            data::SLOT_FEET2,
        ] {
            assert!(crate::mimic::slot_usable(&ps, slot), "slot {slot}");
        }
        // The first weapon lends its plusses; the second only its flags.
        let mut w1 = Item::base(&gd, sword);
        w1.to_h = 5;
        w1.to_d = 7;
        let mut w2 = Item::base(&gd, sword);
        w2.to_h = 11;
        w2.to_d = 13;
        w2.flags.push("RES_FIRE".to_string());
        inv.equip[data::SLOT_WEAPON] = Some(w1);
        inv.equip[data::SLOT_WEAPON2] = Some(w2);
        let t = inv.totals(&gd);
        assert_eq!(t.to_h, 5);
        assert_eq!(t.to_d, 7);
        assert!(t.resists.contains("FIRE"));
        // Two weapons of one kind keep the mastery skill; a mixed pair
        // disables it (xtra1.cc get_weaponmastery_skill -> -1).
        assert!(crate::game::weaponmastery_skill(&ps, &inv, &gd).is_some());
        let axe = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_AXE)
            .expect("axe");
        inv.equip[data::SLOT_WEAPON2] = Some(Item::base(&gd, axe));
        assert!(crate::game::weaponmastery_skill(&ps, &inv, &gd).is_none());
        // Bear shape forces every weapon/shield/boots slot off, extras
        // included.
        let shield = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_SHIELD)
            .expect("shield");
        inv.equip[data::SLOT_SHIELD2] = Some(Item::base(&gd, shield));
        ps.mimic_form = Some(crate::mimic::MIMIC_BEAR);
        crate::mimic::enforce_body(&gd, &mut ps, &mut inv);
        for slot in [
            data::SLOT_WEAPON,
            data::SLOT_WEAPON2,
            data::SLOT_SHIELD,
            data::SLOT_SHIELD2,
            data::SLOT_FEET,
            data::SLOT_FEET2,
        ] {
            assert!(inv.equip[slot].is_none(), "slot {slot} not removed");
        }
        // When the extra limbs fade the worn gear drops into the pack.
        ps.mimic_form = None;
        ps.mimic_extra = crate::mimic::CLASS_ARMS;
        ps.mimic_extra_turns = 1;
        inv.equip[data::SLOT_WEAPON2] = Some(Item::base(&gd, sword));
        ps.mimic_extra = 0;
        ps.mimic_extra_turns = 0;
        let before = inv.pack.len();
        let mut log = crate::game::MessageLog::default();
        crate::mimic::drop_unusable_slots(&gd, &mut ps, &mut inv, &mut log);
        assert!(inv.equip[data::SLOT_WEAPON2].is_none());
        assert_eq!(inv.pack.len(), before + 1);
    }

    #[test]
    fn new_item_kinds_are_ported() {
        let gd = load_game_data();
        // Jewelry, digging tools, instruments, boomerangs, flasks, eggs,
        // corpses, parchments and the plot items all come from k_info now.
        assert!(gd.object_by_name("The One Ring").is_some());
        assert!(gd.object_by_name("Piece of the Relic of Eru").is_some());
        assert!(gd.object_by_name("Flask of oil").is_some());
        assert!(gd.object_by_name("Harp").is_some());
        assert!(gd.object_by_name("Metal Boomerang").is_some());
        assert!(gd.object_by_tval_sval(crate::data::TV_CORPSE, 1).is_some());
        assert!(gd.object_by_tval_sval(crate::data::TV_EGG, 1).is_some());
        assert!(gd
            .object_by_name("Adventurer's Guide to Middle-earth")
            .is_some());
        // The One Ring and relic pieces never spawn randomly.
        let mut rng = crate::rng::current();
        for _ in 0..200 {
            let d = gen_object_kind(&gd, 99, &mut rng).expect("some object");
            let o = &gd.objects[d];
            assert!(o.name != "The One Ring");
            assert!(!o.name.starts_with("Piece of the Relic"));
        }
    }

    #[test]
    fn randart_name_model_follows_randart_cc() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let names = gd.randarts.names.as_str();
        let (probs, totals) = build_name_probs(names);
        // build_prob counts every alphabetic character plus one E_WORD
        // close per name line.
        let letters = names.chars().filter(|c| c.is_ascii_alphabetic()).count() as i64;
        let lines = names.split('\n').count() as i64;
        assert_eq!(totals.values().sum::<i64>(), letters + lines);
        assert_eq!(probs.values().sum::<i64>(), letters + lines);
        // make_word: 5-9 letters, capitalized first letter, at least one
        // vowel (MIN/MAX_NAME_LEN and the vow counter).
        for seed in 0..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let w = make_word(names, &mut rng);
            assert!((5..=9).contains(&w.len()), "{w}");
            assert!(w.chars().next().unwrap().is_ascii_uppercase(), "{w}");
            assert!(w.chars().skip(1).all(|c| c.is_ascii_lowercase()), "{w}");
            assert!(
                w.chars().any(|c| "aeiou".contains(c.to_ascii_lowercase())),
                "{w}"
            );
        }
        // get_random_name: "'Word'" in 1/3, otherwise "of Word".
        let (mut quote, mut of) = (0, 0);
        for seed in 0..60 {
            let mut rng = StdRng::seed_from_u64(seed);
            let n = random_artifact_name(names, &mut rng);
            if n.starts_with('\'') {
                quote += 1;
            } else if n.starts_with("of ") {
                of += 1;
            } else {
                panic!("unexpected randart name {n}");
            }
        }
        assert!(quote > 0 && of > 0, "both name forms appear");
    }

    #[test]
    fn randarts_are_imbued() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let def = gd.object_by_name("Long Sword").unwrap();
        // The scroll path defaults to "of '<player>'" (randart.cc:343; a
        // custom name would need the text-input prompt).
        let mut rng = StdRng::seed_from_u64(7);
        let mut it = Item::base(&gd, def);
        create_artifact(&gd, &mut it, 30, true, "Frodo", &mut rng);
        assert_eq!(it.artifact_name, "of 'Frodo'");
        assert!(it.identified == false);
        // Every randart ignores the four base elements.
        for f in ["IGNORE_ACID", "IGNORE_ELEC", "IGNORE_FIRE", "IGNORE_COLD"] {
            assert!(it.flags.iter().any(|x| x == f), "{}", f);
        }
        // Natural randarts get generated names and powers.
        let mut got_power = false;
        for seed in 0..40 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut it = Item::base(&gd, def);
            create_artifact(&gd, &mut it, 40, false, "", &mut rng);
            assert!(!it.artifact_name.is_empty());
            if it.to_h != 0 || it.to_d != 0 || it.flags.len() > 4 {
                got_power = true;
                break;
            }
        }
        assert!(got_power, "no randart rolled any power");
        // The name corpus generated a real word ("of X" or "'X'").
        assert!(
            it.artifact_name.starts_with("of ") || it.artifact_name.starts_with('\''),
            "{}",
            it.artifact_name
        );
    }

    #[test]
    fn random_artifact_flags_apply() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        // An a_info artifact with RANDOM_*; its base kind must exist.
        let art = gd
            .artifacts
            .iter()
            .find(|a| {
                a.flags.iter().any(|f| {
                    matches!(
                        f.as_str(),
                        "RANDOM_RESIST" | "RANDOM_POWER" | "RANDOM_RES_OR_POWER"
                    )
                }) && gd.object_by_tval_sval(a.tval, a.sval).is_some()
            })
            .expect("a RANDOM_* artifact");
        let def = gd.object_by_tval_sval(art.tval, art.sval).unwrap();
        let mut rng = StdRng::seed_from_u64(3);
        let mut it = Item::base(&gd, def);
        it.artifact = art.id;
        artifact_random_flags(&gd, &mut it, &mut rng);
        assert!(!it.flags.is_empty(), "no random artifact flags rolled");
    }

    #[test]
    fn artifact_sets_grant_tiers() {
        let gd = load_game_data();
        // The Dragon Slayer set (Bow of Bard + Arrow of Bard) is fully
        // present: wearing two distinct members applies the num=2 tier.
        let bow_art = gd
            .artifacts
            .iter()
            .find(|a| a.id == 125)
            .expect("Bow of Bard");
        let arrow_art = gd
            .artifacts
            .iter()
            .find(|a| a.id == 63)
            .expect("Arrow of Bard");
        assert!(gd.set_of_artifact(125).is_some());
        assert_eq!(
            gd.set_of_artifact(125).unwrap().id,
            gd.set_of_artifact(63).unwrap().id
        );
        let bow_def = gd.object_by_tval_sval(bow_art.tval, bow_art.sval).unwrap();
        let arrow_def = gd
            .object_by_tval_sval(arrow_art.tval, arrow_art.sval)
            .unwrap();
        let mut inv = Inventory::default();
        let mut bow = Item::base(&gd, bow_def);
        bow.artifact = 125;
        let mut arrow = Item::base(&gd, arrow_def);
        arrow.artifact = 63;
        // One member: no tier-2 bonuses yet.
        inv.equip[crate::data::SLOT_BOW] = Some(bow.clone());
        assert_eq!(inv.totals(&gd).stats[4], 0); // CON
                                                 // Two members: CON/DEX/RES_FIRE from the bow, SPEED from the arrow.
        inv.equip[crate::data::SLOT_RING1] = Some(arrow);
        let t = inv.totals(&gd);
        assert_eq!(t.stats[4], 3, "CON from the set");
        assert_eq!(t.stats[3], 3, "DEX from the set");
        assert!(t.resists.contains("FIRE"));
        assert_eq!(t.speed, 5);
        // Taking one off drops the tier.
        inv.equip[crate::data::SLOT_RING1] = None;
        assert_eq!(inv.totals(&gd).stats[4], 0);
    }

    #[test]
    fn recharge_devices_follow_the_original_formula() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(11);
        // A wand gains charges (or backfires, which empties it).
        let wand_def = gd.object_by_tval_sval(crate::data::TV_WAND, 3).unwrap();
        let mut wand = Item::base(&gd, wand_def);
        wand.charges = 0;
        match recharge_device(&gd, &mut wand, 130, false, &mut rng) {
            RechargeOutcome::Recharged(n) => assert!(n >= 1 && wand.charges >= n),
            RechargeOutcome::Drained => assert_eq!(wand.charges, 0),
            RechargeOutcome::Destroyed => {}
        }
        // An assembled rod refills toward its capacity.
        let rod_def = gd
            .object_by_tval_sval(crate::data::TV_ROD_MAIN, 10)
            .unwrap();
        let mut rod = Item::base(&gd, rod_def);
        rod.timeout = 0;
        match recharge_device(&gd, &mut rod, 130, true, &mut rng) {
            RechargeOutcome::Recharged(_) => assert!(rod.timeout > 0),
            _ => assert_eq!(rod.timeout, 0),
        }
        assert!(rod.timeout <= rod.pval2);
    }

    #[test]
    fn eternal_flame_imbues_ultimate_artifacts() {
        let gd = load_game_data();
        // The four base objects the Eternal Flame accepts (cmd6.cc).
        let cases = [
            (crate::data::TV_SWORD, 17, 147u32),
            (crate::data::TV_MSTAFF, 1, 127),
            (crate::data::TV_BOW, 24, 152),
            (crate::data::TV_DRAG_ARMOR, 30, 17),
        ];
        for (tval, sval, artifact_id) in cases {
            let def = gd.object_by_tval_sval(tval, sval).expect("base object");
            let mut it = Item::base(&gd, def);
            assert!(imbue_artifact(&gd, &mut it, artifact_id));
            assert_eq!(it.artifact, artifact_id);
            let flags = item_flags(&gd, &it);
            assert!(flags.contains(&"ULTIMATE"), "artifact {artifact_id}");
        }
        // The Flame Imperishable is the activator (k_info 296).
        let flame = gd.object_by_id(296).expect("Flame Imperishable");
        assert_eq!(gd.objects[flame].activate, "ETERNAL_FLAME");
        let (row, _) = crate::spell::artifact_activation("ETERNAL_FLAME");
        assert_eq!(row.kind, "eternal_flame");
    }

    #[test]
    fn fireproof_items_ignore_fire() {
        let gd = load_game_data();
        let scroll = gd.object_by_tval_sval(crate::data::TV_SCROLL, 4).unwrap();
        let mut it = Item::base(&gd, scroll);
        assert!(!item_ignores(&gd, &it, "FIRE"));
        it.fireproof = true;
        assert!(item_ignores(&gd, &it, "FIRE"));
        assert!(!item_ignores(&gd, &it, "COLD"));
    }

    fn use_potion(name: &str, pval2: i32) -> PlayerState {
        let gd = load_game_data();
        let mut map = town_map();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        let mut inv = Inventory::default();
        let mut pos = GridPos { x: 5, y: 5 };
        let mut log = MessageLog::default();
        let mut turn = TurnState::default();
        let mut next = NextState::<AppState>::default();
        let mut rng = crate::rng::current();
        let def = gd.object_by_name(name).expect(name);
        let used = use_object(
            &gd,
            def,
            pval2,
            &mut ps,
            &mut inv,
            &mut map,
            &mut pos,
            &HashSet::new(),
            &mut log,
            &mut turn,
            &mut next,
            &mut rng,
        );
        assert!(used, "{} should be usable", name);
        ps
    }

    #[test]
    fn morphic_oil_takes_the_shape_of_its_form() {
        let ps = use_potion("Morphic Oil of #", 4); // Wolf
        assert_eq!(ps.mimic_form, Some(crate::mimic::MIMIC_WOLF));
        assert!(ps.mimic_turns > 0);
        assert_eq!(
            ps.stats[0],
            ps.stat_base[0]
                + crate::mimic::form_stat_deltas(crate::mimic::MIMIC_WOLF, ps.mimic_level)[0],
            "wolf strength applies while transformed"
        );
    }

    #[test]
    fn corruption_potion_grants_a_corruption() {
        let ps = use_potion("Corruption", 0);
        assert!(ps.corruptions.iter().any(|c| *c), "a corruption is gained");
    }

    #[test]
    fn cure_insanity_potion_heals_the_mind() {
        let gd = load_game_data();
        let def = gd.object_by_name("Cure Critical Insanity").unwrap();
        let mut map = town_map();
        let mut ps = make_player(&gd, "T".into(), 0, 0);
        ps.stats[crate::game::WIS] += 8;
        crate::game::calc_sanity(&mut ps);
        ps.sanity = 0;
        let mut inv = Inventory::default();
        let mut pos = GridPos { x: 5, y: 5 };
        let mut log = MessageLog::default();
        let mut turn = TurnState::default();
        let mut next = NextState::<AppState>::default();
        let mut rng = crate::rng::current();
        let used = use_object(
            &gd,
            def,
            0,
            &mut ps,
            &mut inv,
            &mut map,
            &mut pos,
            &HashSet::new(),
            &mut log,
            &mut turn,
            &mut next,
            &mut rng,
        );
        assert!(used);
        assert!(ps.sanity > 0, "sanity restored");
    }
}

#[cfg(test)]
mod food_device_weight_tests {
    use super::*;
    use crate::birth::make_player;
    use crate::data::load_game_data;

    fn warrior(gd: &GameData) -> PlayerState {
        let idx = gd.classes.iter().position(|c| c.name == "Warrior").unwrap();
        make_player(gd, "T".into(), 0, idx)
    }

    #[test]
    fn lembas_fills_and_bad_mushrooms_hurt() {
        let gd = load_game_data();
        let mut log = crate::game::MessageLog::default();
        let mut rng = crate::rng::current();
        let mut ps = warrior(&gd);
        ps.food = 100;
        let mut inv = Inventory::default();
        let lembas = gd.object_by_name("Lembas").expect("lembas");
        eat_food(&gd, lembas, &mut ps, &mut inv, &mut log, &mut rng);
        assert_eq!(ps.food, crate::game::FOOD_MAX - 1);
        assert!(ps.hp > 0);
        // Greater Ration of Health: permanent +70 max hp.
        let ration = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_FOOD && o.sval == 41)
            .expect("great ration");
        let max_before = ps.max_hp;
        eat_food(&gd, ration, &mut ps, &mut inv, &mut log, &mut rng);
        assert_eq!(ps.max_hp, max_before + 70);
        assert_eq!(ps.hp_mod, 70);
        // Mushroom of Poison: 10+d10 poison without resistance.
        let poison = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_FOOD && o.sval == 0)
            .expect("poison shroom");
        eat_food(&gd, poison, &mut ps, &mut inv, &mut log, &mut rng);
        assert!(ps.poison >= 10, "poisoned {}", ps.poison);
        // Mushroom of Weakness: 6d6 damage and a stat drain.
        let weak = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_FOOD && o.sval == 6)
            .expect("weakness shroom");
        let hp_before = ps.hp;
        eat_food(&gd, weak, &mut ps, &mut inv, &mut log, &mut rng);
        assert!(ps.hp < hp_before, "weakness deals damage");
    }

    #[test]
    fn wands_roll_device_charges_and_stick_levels() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut created = HashSet::new();
        let wand = gd.object_by_name("Manathrust").expect("wand");
        let w = make_item(&gd, wand, 20, false, &mut created, &mut rng);
        // Manathrust charges are 7+d10 = 8..=17 (dice.cc dice_roll adds
        // damroll(1, 10) = randint(10)).
        assert!((8..=17).contains(&w.charges), "charges {}", w.charges);
        // pval3 packs (max << 16) | bonus from the allocation ranges.
        let bonus = w.pval3 & 0xFFFF;
        let maxl = (w.pval3 >> 16) & 0xFFFF;
        assert!((1..=20).contains(&bonus), "bonus {}", bonus);
        assert!((15..=33).contains(&maxl), "max {}", maxl);
        // Staff of Teleportation: 7+d7.
        let staff = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_STAFF && o.spell == "Teleportation")
            .expect("staff");
        let s = make_item(&gd, staff, 20, false, &mut created, &mut rng);
        // 7+d7 = 8..=14.
        assert!((8..=14).contains(&s.charges), "staff charges {}", s.charges);
    }

    #[test]
    fn random_stick_spell_matches_get_random_stick() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let def = gd.object_by_name("Spell").expect("random wand kind");
        let mut created = HashSet::new();
        let mut rolled = 0;
        for _ in 0..50 {
            let it = make_item(&gd, def, 90, false, &mut created, &mut rng);
            if it.stick_spell.is_empty() {
                continue;
            }
            // The rolled name resolves to a spell with a wand allocation
            // (spells5.cc get_random_stick gates on the device rarity).
            let sp = gd
                .spell_by_name(&it.stick_spell)
                .unwrap_or_else(|| panic!("rolled spell {} missing", it.stick_spell));
            assert!(sp.alloc.iter().any(|a| a.0 == crate::data::TV_WAND));
            rolled += 1;
        }
        assert!(rolled > 0, "no random stick spell rolled");
    }

    #[test]
    fn weight_limit_and_encumbrance_slow() {
        let gd = load_game_data();
        let mut ps = warrior(&gd);
        ps.stats[crate::game::STR] = 10;
        // STR 10 -> adj_str_wgt 12 -> 1200 deca-pounds.
        assert_eq!(crate::game::weight_limit(&ps), 1200);
        let mut inv = Inventory::default();
        let sword = gd.object_by_name("Long Sword").expect("sword");
        let mut it = Item::base(&gd, sword);
        it.count = 1;
        inv.pack.push(it);
        let w = crate::game::calc_total_weight(&gd, &inv);
        assert!(w > 0);
        // Ridiculous load: stack many swords to exceed half capacity.
        for _ in 0..200 {
            inv.pack.push(Item::base(&gd, sword));
        }
        let loaded = crate::game::calc_total_weight(&gd, &inv);
        assert!(loaded > crate::game::weight_limit(&ps) / 2);
        let light_speed = crate::game::player_speed(
            &{
                let mut p = ps.clone();
                p.food = 100;
                p
            },
            &Inventory::default(),
            &gd,
        );
        let heavy_speed = crate::game::player_speed(&ps, &inv, &gd);
        assert!(heavy_speed < light_speed, "encumbrance slows");
        // A full belly costs 10 speed (xtra1.cc:3389).
        ps.food = crate::game::FOOD_MAX;
        let bloated = crate::game::player_speed(&ps, &Inventory::default(), &gd);
        assert!(bloated <= light_speed - 10);
    }

    #[test]
    fn flavor_colors_shuffle_per_seed() {
        let gd = load_game_data();
        let ps1 = warrior(&gd);
        let mut ps2 = ps1.clone();
        ps2.flavor_seed = 999;
        let potion = gd
            .objects
            .iter()
            .position(|o| {
                o.tval == crate::data::TV_POTION && !o.flags.iter().any(|f| f == "EASY_KNOW")
            })
            .expect("potion");
        let c1 = flavor_color(&gd, &ps1, potion);
        let c2 = flavor_color(&gd, &ps2, potion);
        assert!(c1.is_some() && c2.is_some());
        // Same seed always agrees with itself.
        assert_eq!(c1, flavor_color(&gd, &ps1, potion));
    }
}

#[cfg(test)]
mod object_gen_value_tests {
    use super::*;
    use crate::data::load_game_data;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn allocation_table_supports_good_and_norm_art() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(42);
        // kind_is_good drops consumables and keeps plain weapons/armour.
        let potion = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_POTION)
            .unwrap();
        assert!(!kind_is_good(&gd, potion));
        let sword = gd.object_by_name("Long Sword").unwrap();
        assert!(kind_is_good(&gd, sword));
        assert!(kind_is_artifactable(&gd, sword));
        // A NORM_ART kind is only created once (object2.cc k_ptr->artifact).
        let blood = gd.object_by_name("Blood of Life").unwrap();
        assert!(gd.objects[blood].flags.iter().any(|f| f == "NORM_ART"));
        let mut created = HashSet::new();
        let _ = make_item(&gd, blood, 30, false, &mut created, &mut rng);
        assert!(created.contains(&norm_art_key(blood)));
        // The second creation falls back to the T: kind.
        let again = make_item(&gd, blood, 30, false, &mut created, &mut rng);
        assert_ne!(again.def, blood);
    }

    #[test]
    fn apply_magic_power_curve_and_forced_power() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(7);
        let sword = gd.object_by_name("Long Sword").unwrap();
        // force_power = 0: the plain object kind, no magic at all.
        let mut created = HashSet::new();
        let plain = apply_magic(&gd, sword, 50, true, false, false, Some(0), &mut created, &mut rng);
        assert_eq!((plain.to_h, plain.to_d, plain.ego, plain.artifact), (0, 0, 0, 0));
        // force_power = 2: always enchanted and at least some kind of ego.
        let mut created = HashSet::new();
        let mut saw_ego = false;
        for _ in 0..30 {
            let it = apply_magic(&gd, sword, 20, true, false, false, Some(2), &mut created, &mut rng);
            assert!(it.to_h != 0 || it.to_d != 0, "no enchantment");
            if it.ego != 0 || it.artifact != 0 || !it.artifact_name.is_empty() {
                saw_ego = true;
            }
        }
        assert!(saw_ego, "power 2 never produced an ego/artifact");
        // force_power = -2: penalized plusses and (almost always) cursed.
        let mut created = HashSet::new();
        for _ in 0..30 {
            let it = apply_magic(&gd, sword, 1, true, false, false, Some(-2), &mut created, &mut rng);
            assert!(it.to_h < 0 && it.to_d < 0, "{} {}", it.to_h, it.to_d);
            // A bad ego may legally add its own C: plusses, so the curse
            // flag is only guaranteed on the plain path.
            if it.ego == 0 && it.artifact == 0 {
                assert!(it.cursed, "very cursed item not flagged");
            }
        }
    }

    #[test]
    fn label_details_follow_object_desc_aux() {
        let gd = load_game_data();
        let known = HashSet::new();
        // pval prose (HIDE_TYPE kinds like the Ring of Speed suppress it).
        let boots = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_BOOTS)
            .expect("boots");
        let mut it = Item::base(&gd, boots);
        it.identified = true;
        it.pval = 7;
        it.flags.push("SPEED".to_string());
        let s = it.label(&gd, &known);
        assert!(s.contains("(+7 to speed)"), "{s}");
        // Rod tips show their Mana cost.
        let rod = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_ROD && o.pval > 0)
            .expect("rod tip");
        let mut tip = Item::base(&gd, rod);
        tip.identified = true;
        let s = tip.label(&gd, &known);
        assert!(s.contains("Mana to cast"), "{s}");
        // The mode-3 inscription assembly joins cursed / discount /
        // inscription with ", ".
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut sc = Item::base(&gd, sword);
        sc.identified = true;
        sc.cursed = true;
        sc.known_cursed = true;
        sc.inscription = "!d".to_string();
        sc.discount = 25;
        let s = sc.label(&gd, &known);
        assert!(s.contains("{cursed, !d}"), "{s}");
        sc.inscription.clear();
        let s = sc.label(&gd, &known);
        assert!(s.contains("{cursed, 25% off}"), "{s}");
    }

    #[test]
    fn rating_of_counts_egos_artifacts_and_depth() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(3);
        // An out-of-depth plain object raises the rating by its level.
        let star = gd
            .objects
            .iter()
            .position(|o| o.depth > 40 && o.tval == data::TV_SWORD)
            .expect("deep sword");
        let it = Item::base(&gd, star);
        let (r, g) = rating_of(&gd, &it, 1);
        assert!(r >= 40 && !g, "{r}");
        // A real artifact adds 10 and sets the good-item flag.
        let art = specific_artifact_item(&gd, 1, &mut rng).expect("artifact");
        let (r, g) = rating_of(&gd, &art, 0);
        assert!(r >= 10 && g, "{r} {g}");
        // Artifacts copy their a_info W: weight over the base kind.
        let armor = specific_artifact_item(&gd, 16, &mut rng).expect("Razorback");
        assert_eq!(armor.weight, 500);
        assert_ne!(gd.objects[armor.def].weight, 500);
    }

    #[test]
    fn corpse_decay_turns_heads_into_skulls() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(11);
        let mon = gd.monster_by_id(2).expect("monster");
        let head = gd.object_by_tval_sval(data::TV_CORPSE, 3).expect("head");
        let mut it = Item::base(&gd, head);
        init_corpse(&gd, &mut it, mon, 1000, &mut rng);
        let out = decayed_corpse(&gd, &it, &mut rng).expect("skull");
        assert_eq!(gd.objects[out.def].sval, 4);
        assert_eq!(out.note, mon as u32);
    }

    #[test]
    fn theme_restricts_the_object_pool() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(5);
        // A 100% magic theme never yields combat/treasure/tools tvals.
        for _ in 0..50 {
            let def = gen_object_kind_themed(&gd, 30, Some((0, 0, 100, 0)), false, None, &mut rng)
                .expect("themed kind");
            assert!(
                theme_prob((0, 0, 100, 0), gd.objects[def].tval) == 100,
                "{} {}",
                gd.objects[def].name,
                gd.objects[def].tval
            );
        }
    }

    #[test]
    fn make_gold_uses_the_coin_table() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let (id, amount) = make_gold(&gd, 20, &mut rng);
            let base = gd.objects.iter().find(|o| o.id == id).unwrap().cost;
            assert!((480..=497).contains(&id));
            assert!(amount >= base + 8 + 1, "{amount} vs {base}");
            assert!(amount <= base + 8 * base + 8);
        }
    }

    #[test]
    fn flag_cost_prices_speed_and_curses() {
        let gd = load_game_data();
        let boots = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_BOOTS && o.cost > 0)
            .unwrap();
        let mut it = Item::base(&gd, boots);
        it.flags.push("SPEED".into());
        it.pval = 5;
        assert!(flag_cost(&gd, &it, 5) >= 10000 + 12500);
        let mut cursed = it.clone();
        cursed.cursed = true;
        cursed.flags.push("CURSED".into());
        assert_eq!(object_value(&gd, &cursed), 0);
        // A plain item is worth its base cost.
        let plain = Item::base(&gd, boots);
        assert_eq!(object_value(&gd, &plain), gd.objects[boots].cost);
    }

    #[test]
    fn items_similar_stacks_ammo_and_merges_wands() {
        let gd = load_game_data();
        let arrow = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_ARROW)
            .unwrap();
        let mut a = Item::base(&gd, arrow);
        a.identified = true;
        let mut b = a.clone();
        b.count = 5;
        assert!(items_similar(&gd, &a, &b));
        // Different plusses do not stack, and the count is absorbed.
        b.to_h = 1;
        assert!(!items_similar(&gd, &a, &b));
        let mut inv = Inventory::default();
        inv.add_with(&gd, a.clone());
        inv.add_with(&gd, b.clone());
        assert_eq!(inv.pack.len(), 2);
    }

    #[test]
    fn potion_smash_table_matches_spells1() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(5);
        // Useless contents and personal buffs just splash.
        for sval in [0, 1, 2, 5, 13, 16, 21, 24, 30, 45, 56, 60, 63] {
            assert!(potion_smash_effect(sval, &mut rng).is_none(), "sval {sval}");
        }
        // Explosive potions project their element.
        assert_eq!(
            potion_smash_effect(22, &mut rng).map(|s| s.0),
            Some("SHARDS")
        );
        let death = potion_smash_effect(23, &mut rng).unwrap();
        assert_eq!(death.0, "MANA");
        assert_eq!(death.2, 1);
        assert!((1..=100).contains(&death.1));
        let slowness = potion_smash_effect(4, &mut rng).unwrap();
        assert_eq!(slowness, ("OLD_SLOW", 5, 2));
        let heal = potion_smash_effect(37, &mut rng).unwrap();
        assert_eq!(heal.0, "OLD_HEAL");
        assert!(heal.1 >= 10);
        // SV_POTION_CURING (61) is a heal, not a no-op potion.
        assert_eq!(
            potion_smash_effect(61, &mut rng).map(|s| s.0),
            Some("OLD_HEAL")
        );
    }

    #[test]
    fn breakage_chances_match_cmd2() {
        assert_eq!(breakage_chance(data::TV_POTION, 0), 100);
        assert_eq!(breakage_chance(data::TV_SCROLL, 0), 50);
        assert_eq!(breakage_chance(data::TV_ARROW, 0), 50);
        assert_eq!(breakage_chance(data::TV_ARROW, 4), 10);
        assert_eq!(breakage_chance(data::TV_SHOT, 0), 25);
        assert_eq!(breakage_chance(data::TV_BOOMERANG, 0), 1);
        assert_eq!(breakage_chance(data::TV_SWORD, 0), 10);
    }

    #[test]
    fn inven_damage_destroys_and_reports_smashed_potions() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let potion = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_POTION && o.sval == 23)
            .expect("Death potion");
        let scroll = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_SCROLL)
            .expect("scroll");
        let mut inv = Inventory::default();
        // Fire destroys scrolls; cold shatters potions.
        let mut it = Item::base(&gd, potion);
        it.count = 3;
        inv.pack.push(it);
        inv.pack.push(Item::base(&gd, scroll));
        let mut rng = StdRng::seed_from_u64(9);
        let mut log = crate::game::MessageLog::default();
        let smashed = inven_damage_ex(&gd, &mut inv, "COLD", 100, &mut log, &mut rng);
        assert!(smashed.contains(&23), "death potion did not smash");
        assert!(inv
            .pack
            .iter()
            .all(|i| gd.objects[i.def].tval != data::TV_POTION));
        // A guaranteed destruction reports each item.
        assert!(log.lines.iter().any(|l| l.contains("destroyed!")));
        // Artifacts are immune even at 100%.
        let mut inv = Inventory::default();
        let mut art = Item::base(&gd, scroll);
        art.artifact_name = "of Testing".to_string();
        inv.pack.push(art);
        let before = inv.pack.len();
        inven_damage_ex(&gd, &mut inv, "FIRE", 100, &mut log, &mut rng);
        assert_eq!(inv.pack.len(), before);
    }

    #[test]
    fn curse_equipment_heavy_and_dg_marks_artifacts() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let sword = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_SWORD)
            .expect("sword");
        let mut inv = Inventory::default();
        let mut art = Item::base(&gd, sword);
        art.ego = 1;
        for slot in 0..data::NUM_SLOTS {
            inv.equip[slot] = Some(art.clone());
        }
        let mut rng = StdRng::seed_from_u64(11);
        let mut log = crate::game::MessageLog::default();
        // One random slot is picked per call; fill every slot and call
        // until all have been hit.
        for _ in 0..500 {
            curse_equipment_ex(&gd, &mut inv, 100, 100, true, &mut log, &mut rng);
            if inv.equip.iter().all(|s| {
                s.as_ref()
                    .is_some_and(|i| i.flags.iter().any(|f| f == "HEAVY_CURSE"))
            }) {
                break;
            }
        }
        for slot in 0..data::NUM_SLOTS {
            let it = inv.equip[slot].as_ref().unwrap();
            for f in ["HEAVY_CURSE", "CURSED", "DG_CURSE"] {
                assert!(it.flags.iter().any(|x| x == f), "slot {slot} missing {f}");
            }
            assert!(it.cursed);
        }
    }

    #[test]
    fn apply_flags_follow_xtra1_numerics() {
        let gd = load_game_data();
        let sword = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_SWORD)
            .expect("sword");
        let mut it = Item::base(&gd, sword);
        it.pval = 3;
        it.flags = vec![
            "STEALTH".into(),
            "TUNNEL".into(),
            "INVIS".into(),
            "LUCK".into(),
            "BLESSED".into(),
        ];
        let mut inv = Inventory::default();
        inv.equip[data::SLOT_WEAPON] = Some(it);
        let warrior = gd
            .classes
            .iter()
            .position(|c| c.name == "Warrior")
            .expect("warrior");
        let mut ps = crate::birth::make_player(&gd, "T".into(), 0, warrior);
        ps.luck_base = 0;
        let t = inv.totals_for(&gd, &ps);
        assert_eq!(t.stealth, 3, "TR_STEALTH adds pval");
        assert_eq!(t.tunnel, 60, "TR_TUNNEL adds pval * 20");
        assert_eq!(t.invis_power, 30, "TR_INVIS adds pval * 10");
        assert!(t.invis);
        assert_eq!(t.luck, 3, "TR_LUCK adds pval");
        // TR_BLESSED is bless_blade only, never a damage slay.
        assert!(!t.slays.contains("BLESSED"));
        assert!(!is_slay_flag("BLESSED"));
        // A holy aura holds life and adds +5 luck (xtra1.cc:3218).
        ps.holy = 100;
        let t = inv.totals_for(&gd, &ps);
        assert!(t.hold_life);
        assert_eq!(t.luck, 8);
    }

    #[test]
    fn randnor_degenerate_and_range() {
        let mut rng = crate::rng::current();
        // z-rand.cc: stand < 1 returns 0, not the mean.
        assert_eq!(randnor(1234, 0, &mut rng), 0);
        assert_eq!(randnor(1234, -5, &mut rng), 0);
        // Samples stay near the mean for reasonable deviations.
        let mut sum = 0i64;
        let n = 4000;
        for _ in 0..n {
            let v = randnor(100, 20, &mut rng);
            assert!((-200..=400).contains(&v));
            sum += v as i64;
        }
        let mean = sum / n;
        assert!((60..=140).contains(&mean), "mean {mean}");
    }

    #[test]
    fn creeping_coins_force_their_coin_type() {
        let gd = load_game_data();
        let copper = gd
            .monsters
            .iter()
            .find(|m| m.ch == "$" && m.name.to_lowercase().contains("copper"))
            .expect("copper coin monster");
        assert_eq!(get_coin_type(copper), Some(2));
        let add = gd
            .monsters
            .iter()
            .find(|m| m.ch == "$" && m.name.to_lowercase().contains("adamantite"))
            .expect("adamantite coin monster");
        assert_eq!(get_coin_type(add), Some(17));
        // A forced kind returns that exact treasure id (OBJ_GOLD_LIST + 2).
        let mut rng = crate::rng::current();
        let (id, amount) = make_gold_coin(&gd, 30, Some(2), &mut rng);
        assert_eq!(id, 482);
        assert!(amount > 0);
    }
}

#[cfg(test)]
mod object12_audit_tests {
    use super::*;
    use crate::data::load_game_data;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn sword(gd: &GameData) -> usize {
        gd.object_by_name("Long Sword").expect("Long Sword")
    }

    #[test]
    fn easy_know_follows_object1_switch() {
        let gd = load_game_data();
        let potion = gd.objects.iter().find(|o| o.tval == data::TV_POTION).unwrap();
        assert!(potion.easy_know());
        let food = gd.objects.iter().find(|o| o.tval == data::TV_FOOD).unwrap();
        assert!(food.easy_know());
        let corpse = gd.objects.iter().find(|o| o.tval == data::TV_CORPSE).unwrap();
        assert!(corpse.easy_know());
        let ring = gd.object_by_name("Ring of Free Action").unwrap();
        assert!(gd.objects[ring].easy_know());
        let plain_ring = gd
            .objects
            .iter()
            .find(|o| o.tval == data::TV_RING && !o.flags.iter().any(|f| f == "EASY_KNOW"))
            .unwrap();
        assert!(!plain_ring.easy_know());
        let norm = gd.objects.iter().find(|o| {
            matches!(o.tval, data::TV_POTION | data::TV_POTION2)
                && o.flags.iter().any(|f| f == "NORM_ART")
        });
        if let Some(o) = norm {
            assert!(!o.easy_know());
        }
    }

    #[test]
    fn object_flags_extra_and_known_follow_object1() {
        let gd = load_game_data();
        let def = sword(&gd);
        let mut rng = StdRng::seed_from_u64(3);
        // object_flags_xtra: SUSTAIN picks one of six sustains, OLD_RESIST
        // one of the eleven high resists.
        let mut it = Item::base(&gd, def);
        let mut limit = false;
        add_random_ego_flag(&mut it, data::TV_SWORD, 30, "SUSTAIN", &mut limit, &mut rng);
        assert!(
            it.flags.iter().any(|f| f.starts_with("SUST_")),
            "{:?}",
            it.flags
        );
        add_random_ego_flag(&mut it, data::TV_SWORD, 30, "OLD_RESIST", &mut limit, &mut rng);
        assert!(
            it.flags.iter().any(|f| f.starts_with("RES_")),
            "{:?}",
            it.flags
        );
        // object_flags_known: unknown items hide everything; RES_CHAOS
        // implies RES_CONF once known.
        let mut unknown = Item::base(&gd, def);
        unknown.identified = false;
        assert!(item_flags_known(&gd, &unknown).is_empty());
        unknown.identified = true;
        unknown.flags.push("RES_CHAOS".to_string());
        let known = item_flags_known(&gd, &unknown);
        assert!(known.contains(&"RES_CHAOS") && known.contains(&"RES_CONF"));
        // Base kind flags flow into item_flags (object_flags).
        let fa = gd.object_by_name("Ring of Free Action").unwrap();
        let ring = Item::base(&gd, fa);
        assert!(item_flags(&gd, &ring).contains(&"FREE_ACT"));
    }

    #[test]
    fn item_letter_colours_follow_object1() {
        use crate::base_defs::{
            TERM_GREEN, TERM_L_BLUE, TERM_SLATE, TERM_VIOLET, TERM_WHITE, TERM_YELLOW,
        };
        let gd = load_game_data();
        let known = HashSet::new();
        let def = sword(&gd);
        let plain = Item::base(&gd, def);
        assert_eq!(get_item_letter_color(&gd, &plain, &known), TERM_SLATE);
        let mut id = plain.clone();
        id.identified = true;
        assert_eq!(get_item_letter_color(&gd, &id, &known), TERM_WHITE);
        let mut ego = id.clone();
        ego.ego = 1;
        assert_eq!(get_item_letter_color(&gd, &ego, &known), TERM_L_BLUE);
        let plain_art = gd
            .artifacts
            .iter()
            .find(|a| gd.set_of_artifact(a.id).is_none())
            .expect("set-less artifact");
        let mut art = id.clone();
        art.artifact = plain_art.id;
        assert_eq!(get_item_letter_color(&gd, &art, &known), TERM_YELLOW);
        art.artifact = 125;
        assert_eq!(get_item_letter_color(&gd, &art, &known), TERM_GREEN);
        art.flags.push("ULTIMATE".to_string());
        assert_eq!(get_item_letter_color(&gd, &art, &known), TERM_VIOLET);
    }

    #[test]
    fn labels_tags_and_tester_helpers_follow_object1() {
        let gd = load_game_data();
        assert_eq!(index_to_label(0), 'a');
        assert_eq!(index_to_label(23), 'a');
        assert_eq!(index_to_label(25), 'c');
        assert_eq!(
            mention_use(data::SLOT_WEAPON, data::TV_SWORD, 10, 100),
            "Wielding"
        );
        assert_eq!(
            mention_use(data::SLOT_WEAPON, data::TV_SWORD, 2000, 100),
            "Just lifting"
        );
        assert_eq!(
            mention_use(data::SLOT_BOW, data::TV_INSTRUMENT, 50, 100),
            "Playing"
        );
        assert_eq!(
            mention_use(data::SLOT_BOW, data::TV_BOW, 2000, 100),
            "Just holding"
        );
        assert_eq!(
            mention_use(data::SLOT_RING1, data::TV_RING, 1, 100),
            "On finger"
        );
        assert_eq!(
            mention_use(data::SLOT_QUIVER, data::TV_ARROW, 1, 100),
            "Quiver"
        );
        assert_eq!(
            describe_use(data::SLOT_BODY, data::TV_SOFT_ARMOR, 1, 100),
            "wearing on your body"
        );
        assert_eq!(
            describe_use(data::SLOT_SYMBIOTE, data::TV_HYPNOS, 1, 100),
            "in symbiosis with"
        );
        assert_eq!(
            describe_use(data::SLOT_QUIVER, data::TV_ARROW, 1, 100),
            "carrying in your quiver"
        );
        assert_eq!(
            describe_use(data::SLOT_WEAPON, data::TV_SWORD, 2000, 100),
            "just lifting"
        );
        // item_tester_okay: empty slots only when full, gold never.
        let gold = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_GOLD)
            .expect("gold kind");
        assert!(!item_tester_okay(&gd, false, None, |_| true));
        assert!(item_tester_okay(&gd, true, None, |_| true));
        assert!(!item_tester_okay(
            &gd,
            false,
            Some(&Item::base(&gd, gold)),
            |_| true
        ));
        let sword_def = sword(&gd);
        // object_char/object_attr: the drawn glyph and colour come from the
        // kind's default char/attr (the port has no x_char/x_attr state).
        assert_eq!(
            gd.objects[sword_def].glyph(),
            gd.objects[sword_def].ch.chars().next().unwrap()
        );
        assert!(gd.objects[sword_def].color > 0);
        assert!(item_tester_okay(
            &gd,
            false,
            Some(&Item::base(&gd, sword_def)),
            |it| gd.objects[it.def].tval == data::TV_SWORD
        ));
        // get_item_okay indexes the combined pack/equip array.
        let mut pack = vec![Item::base(&gd, sword_def)];
        let mut equip: [Option<Item>; data::NUM_SLOTS] = std::array::from_fn(|_| None);
        assert!(get_item_okay(&gd, 0, &pack, &equip, |_| true));
        assert!(!get_item_okay(&gd, 1, &pack, &equip, |_| true));
        equip[0] = Some(Item::base(&gd, sword_def));
        assert!(get_item_okay(&gd, pack.len(), &pack, &equip, |_| true));
        // get_tag finds @n and @<cmd>n.
        pack[0].inscription = "hello @x".to_string();
        assert_eq!(get_tag(&pack, &equip, 'q', 'x'), Some(0));
        pack[0].inscription = "@qx".to_string();
        assert_eq!(get_tag(&pack, &equip, 'q', 'x'), Some(0));
        assert_eq!(get_tag(&pack, &equip, 'z', 'x'), None);
        // scan_floor caps at 23 matches.
        let many: Vec<Item> = (0..30).map(|_| Item::base(&gd, sword_def)).collect();
        assert_eq!(
            scan_floor(&many, |it| gd.objects[it.def].tval == data::TV_SWORD).len(),
            23
        );
        assert!(inscription_prevents("!d", 'd'));
        assert!(inscription_prevents("junk !*", 'q'));
        assert!(!inscription_prevents("!d", 'q'));
        assert_eq!(
            verify_prompt("Really try", "a Long Sword"),
            "Really try a Long Sword? "
        );
        // item_tester_hook_getable: a full pack blocks, symbiotes need skill.
        let mut full = Inventory::default();
        for d in 0..23 {
            full.pack.push(Item::base(&gd, d));
        }
        assert!(!inven_carry_okay(&gd, &full, &Item::base(&gd, 23)));
        if let Some(h) = gd.objects.iter().position(|o| o.tval == data::TV_HYPNOS) {
            assert!(!item_tester_hook_getable(
                &gd,
                &Inventory::default(),
                &Item::base(&gd, h),
                false
            ));
            assert!(item_tester_hook_getable(
                &gd,
                &Inventory::default(),
                &Item::base(&gd, h),
                true
            ));
        }
    }

    #[test]
    fn damage_display_follows_object1_output_dam() {
        let mut first = true;
        let s = output_dam(&mut first, 1, 6, 0, 0, 0, 1, 2, 0, "animals", "");
        assert_eq!(s, "7 against animals");
        assert!(!first);
        let s = output_dam(&mut first, 1, 6, 2, 1, 1, 2, 3, 6, "x", "y");
        assert_eq!(s, ", 29 against x, 50 against y");
        let mut first = true;
        let s = output_ammo_dam(&mut first, false, 1, 4, 2, 3, 1, 2, 1, 0, "animals", "");
        assert_eq!(s, "16 against animals");
        let mut first = true;
        let s = output_ammo_dam(&mut first, true, 1, 4, 2, 3, 1, 2, 1, 0, "animals", "");
        assert_eq!(s, "9 against animals");
    }

    #[test]
    fn toggle_inven_equip_flips_object1_window_flags() {
        use crate::base_defs::{PW_EQUIP, PW_INVEN};
        assert_eq!(toggle_inven_equip(PW_INVEN), PW_EQUIP);
        assert_eq!(toggle_inven_equip(PW_EQUIP), PW_INVEN);
        assert_eq!(toggle_inven_equip(PW_INVEN | PW_EQUIP), PW_EQUIP);
        assert_eq!(toggle_inven_equip(0), 0);
    }

    #[test]
    fn absorb_and_combine_follow_object_absorb() {
        let gd = load_game_data();
        // Wands combine charges on stacking; staffs keep their shared
        // per-item charges (object_absorb only sums TV_WAND pval).
        let wand_def = gd.object_by_tval_sval(data::TV_WAND, 3).expect("wand");
        let mut a = Item::base(&gd, wand_def);
        a.identified = true;
        a.charges = 5;
        let mut b = a.clone();
        b.charges = 7;
        let mut inv = Inventory::default();
        inv.pack.push(a);
        inv.add_with(&gd, b);
        assert_eq!(inv.pack.len(), 1);
        assert_eq!(inv.pack[0].count, 2);
        assert_eq!(inv.pack[0].charges, 12);
        let staff_def = gd.object_by_tval_sval(data::TV_STAFF, 1).expect("staff");
        let mut sa = Item::base(&gd, staff_def);
        sa.identified = true;
        sa.charges = 5;
        let sb = sa.clone();
        let mut inv = Inventory::default();
        inv.pack.push(sa);
        inv.add_with(&gd, sb);
        assert_eq!(inv.pack.len(), 1, "identical staff stacks");
        assert_eq!(inv.pack[0].count, 2);
        assert_eq!(inv.pack[0].charges, 5, "staff charges must not double");
        // combine_pack runs the full absorb (inscription blending).
        let mut p = Item::base(&gd, wand_def);
        p.identified = true;
        p.charges = 2;
        let mut q = p.clone();
        q.charges = 3;
        q.inscription = "!d".to_string();
        let mut inv = Inventory::default();
        inv.pack.push(p);
        inv.pack.push(q);
        let mut log = MessageLog::default();
        combine_pack(&gd, &mut inv, &mut log);
        assert_eq!(inv.pack.len(), 1);
        assert_eq!(inv.pack[0].inscription, "!d");
        assert!(log.lines.iter().any(|l| l.contains("combine")));
    }

    #[test]
    fn stack_size_slots_and_floor_removal_follow_object2() {
        let gd = load_game_data();
        let def = sword(&gd);
        let mut it = Item::base(&gd, def);
        it.count = 5;
        item_increase(&mut it, 3);
        assert_eq!(it.count, 8);
        item_increase(&mut it, -100);
        assert_eq!(it.count, 0);
        item_increase(&mut it, 300);
        assert_eq!(it.count, 255);
        let mut inv = Inventory::default();
        for d in 0..23 {
            inv.pack.push(Item::base(&gd, d));
        }
        assert_eq!(find_empty_slot(&inv), None);
        assert!(!inven_carry_okay(&gd, &inv, &Item::base(&gd, 23)));
        let gold = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_GOLD)
            .unwrap();
        assert!(!inven_carry_okay(
            &gd,
            &Inventory::default(),
            &Item::base(&gd, gold)
        ));
        // delete_floor_object is the excise/delete_object_idx core.
        let mut stack = vec![Item::base(&gd, def), Item::base(&gd, def)];
        assert!(delete_floor_object(&mut stack, 0).is_some());
        assert_eq!(stack.len(), 1);
        assert!(delete_floor_object(&mut stack, 5).is_none());
        delete_floor_stack(&mut stack);
        assert!(stack.is_empty());
        // compact_objects_aux moves the tail object into the hole.
        let mut items = vec![
            Item::base(&gd, sword(&gd)),
            Item::base(&gd, def),
            Item::base(&gd, def),
        ];
        compact_objects_aux(&mut items, 2, 0);
        assert_eq!(items.len(), 3);
        // compact_objects spares artifacts and nearby objects.
        let mut rng = StdRng::seed_from_u64(5);
        for _ in 0..20 {
            assert!(compaction_spares(10, true, false, false, 50, 10, &mut rng));
            assert!(compaction_spares(10, false, false, false, 0, 10, &mut rng));
            assert!(compaction_spares(300, false, false, false, 50, 10, &mut rng));
        }
        // get_object resolves combined indexes.
        let pack = vec![Item::base(&gd, def)];
        let mut equip: [Option<Item>; data::NUM_SLOTS] = std::array::from_fn(|_| None);
        assert!(get_object(&pack, &equip, 0).is_some());
        assert!(get_object(&pack, &equip, -3).is_none());
        equip[2] = Some(Item::base(&gd, def));
        assert!(get_object(&pack, &equip, 1 + 2).is_some());
    }

    #[test]
    fn where_found_text_follows_object_out_desc() {
        let gd = load_game_data();
        let def = sword(&gd);
        let mut it = Item::base(&gd, def);
        assert!(where_found_text(&gd, &it).is_none());
        it.found = OBJ_FOUND_FLOOR;
        it.found_aux1 = gd.dungeons.iter().find(|d| d.id > 0).unwrap().id as i32;
        it.found_aux2 = 5;
        let s = where_found_text(&gd, &it).unwrap();
        assert!(
            s.starts_with("You found it lying on the ground on level 5 of "),
            "{s}"
        );
        it.found = OBJ_FOUND_SELFMADE;
        assert_eq!(
            where_found_text(&gd, &it).unwrap(),
            "You made it yourself."
        );
        it.found = OBJ_FOUND_MONSTER;
        it.found_aux1 = gd.monsters.iter().position(|m| !m.unique).unwrap() as i32;
        it.found_aux3 = 0;
        it.found_aux4 = 0;
        let s = where_found_text(&gd, &it).unwrap();
        assert!(s.starts_with("You found it in the remains of "), "{s}");
    }

    #[test]
    fn object_prep_and_awareness_follow_object2() {
        let gd = load_game_data();
        let lite = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_LITE && o.fuel > 0)
            .expect("lite");
        assert!(Item::base(&gd, lite).fuel > 0);
        let rod = gd.object_by_tval_sval(data::TV_ROD_MAIN, 10).expect("rod");
        let it = Item::base(&gd, rod);
        assert_eq!(it.pval, 0);
        assert_eq!(it.pval2, 10);
        assert_eq!(it.timeout, 10);
        let cursed = gd
            .objects
            .iter()
            .position(|o| o.flags.iter().any(|f| f == "CURSED"))
            .expect("cursed kind");
        assert!(Item::base(&gd, cursed).cursed);
        // object_aware_p/object_known_p: EASY_KNOW kinds and learned types.
        let potion_def = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_POTION)
            .unwrap();
        let mut flavoured = Item::base(&gd, potion_def);
        assert!(!flavoured.identified, "flavoured potion starts unknown");
        flavoured.identified = true;
        let eak = gd.object_by_name("Ring of Free Action").unwrap();
        assert!(Item::base(&gd, eak).identified, "EASY_KNOW known on sight");
        let mut inv = Inventory::default();
        inv.learn(potion_def);
        assert!(inv.known.contains(&potion_def));
    }

    #[test]
    fn a_m_aux_one_two_and_dragon_resist_follow_object2() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(17);
        let def = sword(&gd);
        let mut it = Item::base(&gd, def);
        a_m_aux_1(&gd, &mut it, 20, 1, 20, &mut rng);
        assert!(it.to_h > 0 && it.to_d > 0);
        let mut bad = Item::base(&gd, def);
        a_m_aux_1(&gd, &mut bad, 20, -2, 20, &mut rng);
        assert!(bad.to_h < 0 && bad.to_d < 0 && bad.cursed);
        let boots = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_BOOTS)
            .unwrap();
        let mut armour = Item::base(&gd, boots);
        a_m_aux_2(&gd, &mut armour, 20, 1, 20, &mut rng);
        assert!(armour.to_a > 0);
        let shield = gd
            .object_by_tval_sval(data::TV_SHIELD, 6)
            .expect("dragon shield");
        let mut ds = Item::base(&gd, shield);
        dragon_resist(&mut ds, data::TV_SHIELD, &mut rng);
        assert!(!ds.flags.is_empty(), "dragon shield got no resistance");
        // m_bonus stays inside [0, max].
        for _ in 0..200 {
            let v = m_bonus(5, 50, &mut rng);
            assert!((0..=5).contains(&v), "m_bonus out of range: {v}");
        }
    }

    #[test]
    fn random_artifact_power_and_mention_follow_object2() {
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(9);
        let mut it = Item::base(&gd, sword(&gd));
        random_artifact_power(&mut it, &mut rng);
        const ABILITIES: [&str; 8] = [
            "FEATHER",
            "LITE1",
            "SEE_INVIS",
            "ESP_ALL",
            "SLOW_DIGEST",
            "REGEN",
            "FREE_ACT",
            "HOLD_LIFE",
        ];
        assert!(
            it.flags
                .iter()
                .any(|f| ABILITIES.contains(&f.as_str()))
                || it.flags.iter().any(|f| f == "ESP_ALL"),
            "{:?}",
            it.flags
        );
        // object_mention: plain, ego and artifact forms.
        let known = HashSet::new();
        let mut plain = Item::base(&gd, sword(&gd));
        plain.identified = true;
        assert!(object_mention_text(&gd, &plain, &known).starts_with("Object ("));
        let mut ego = plain.clone();
        ego.ego = 1;
        assert!(object_mention_text(&gd, &ego, &known).starts_with("Ego-item ("));
        assert!(ego_item_p(&ego) && is_ego_p(&ego, 1) && !is_ego_p(&ego, 2));
        let mut art = plain.clone();
        art.artifact = 1;
        assert!(object_mention_text(&gd, &art, &known).starts_with("Artifact ("));
    }

    #[test]
    fn reorder_pack_sorts_like_object2() {
        let gd = load_game_data();
        // Put a low-tval item before a high-tval one; reorder flips them
        // (objects sort by decreasing tval).
        let low = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_LITE)
            .unwrap();
        let high = sword(&gd);
        let mut inv = Inventory::default();
        let mut a = Item::base(&gd, low);
        a.identified = true;
        let mut b = Item::base(&gd, high);
        b.identified = true;
        inv.pack.push(a);
        inv.pack.push(b);
        let mut log = MessageLog::default();
        reorder_pack(&gd, &mut inv, &mut log);
        assert_eq!(
            gd.objects[inv.pack[0].def].tval,
            data::TV_LITE,
            "higher tval sorts first"
        );
    }

    #[test]
    fn floor_carry_spawns_and_merges_stacks() {
        use bevy::ecs::system::RunSystemOnce;
        let gd: &'static GameData = Box::leak(Box::new(load_game_data()));
        let def = sword(gd);
        let tiles: &'static crate::render::TileAssets =
            Box::leak(Box::new(crate::render::TileAssets {
                font: Default::default(),
                bg_image: Default::default(),
                bg_layout: Default::default(),
                fg_image: Default::default(),
                fg_layout: Default::default(),
            }));
        let mut world = World::new();
        for _ in 0..2 {
            world
                .run_system_once(
                    move |mut commands: Commands,
                          mut q: Query<(Entity, &GridPos, &mut FloorItem)>| {
                        let it = Item::base(gd, def);
                        place_floor_item(&mut commands, gd, tiles, &mut q, 3, 4, it);
                    },
                )
                .unwrap();
        }
        let mut q = world.query::<(Entity, &GridPos, &FloorItem)>();
        let entries: Vec<_> = q.iter(&world).collect();
        assert_eq!(entries.len(), 1, "similar objects share one floor stack");
        assert_eq!((entries[0].1.x, entries[0].1.y), (3, 4));
        assert_eq!(entries[0].2.stack.len(), 2);
    }

    #[test]
    fn drop_near_places_an_object_on_the_floor() {
        use bevy::ecs::system::RunSystemOnce;
        let gd: &'static GameData = Box::leak(Box::new(load_game_data()));
        let tiles: &'static crate::render::TileAssets =
            Box::leak(Box::new(crate::render::TileAssets {
                font: Default::default(),
                bg_image: Default::default(),
                bg_layout: Default::default(),
                fg_image: Default::default(),
                fg_layout: Default::default(),
            }));
        let map: &'static Map = Box::leak(Box::new(crate::map::blank_pub(crate::map::T_FLOOR)));
        let known: &'static HashSet<usize> = Box::leak(Box::new(HashSet::new()));
        let occupied: &'static HashSet<(i32, i32)> = Box::leak(Box::new(HashSet::new()));
        let def = sword(gd);
        let mut world = World::new();
        world.insert_resource(MessageLog::default());
        world
            .run_system_once(
                move |mut commands: Commands,
                      mut q: Query<(Entity, &GridPos, &mut FloorItem)>,
                      mut log: ResMut<MessageLog>| {
                    let mut rng = crate::rng::current();
                    let it = Item::base(gd, def);
                    let spot = drop_near(
                        &mut commands, gd, tiles, map, &mut q, &mut log, known, occupied, None, it,
                        0, 5, 5, &mut rng,
                    );
                    assert!(spot.is_some(), "drop_near failed on a clear grid");
                },
            )
            .unwrap();
        let mut q = world.query::<&FloorItem>();
        assert_eq!(q.iter(&world).count(), 1);
    }

    #[test]
    fn scatter_objects_places_objects_and_gold() {
        use bevy::ecs::system::RunSystemOnce;
        let gd: &'static GameData = Box::leak(Box::new(load_game_data()));
        let tiles: &'static crate::render::TileAssets =
            Box::leak(Box::new(crate::render::TileAssets {
                font: Default::default(),
                bg_image: Default::default(),
                bg_layout: Default::default(),
                fg_image: Default::default(),
                fg_layout: Default::default(),
            }));
        let map: &'static Map = Box::leak(Box::new(crate::map::blank_pub(crate::map::T_FLOOR)));
        let mut world = World::new();
        for _ in 0..10 {
            world
                .run_system_once(
                    move |mut commands: Commands, mut q: Query<(Entity, &GridPos, &mut FloorItem)>| {
                        let mut rng = crate::rng::current();
                        let mut created = HashSet::new();
                        let (rating, good) = scatter_objects(
                            &mut commands, gd, tiles, map, 20, 2, (20, 20), &mut created, &mut rng,
                        );
                        assert!(rating >= 0 && !good || good);
                    },
                )
                .unwrap();
        }
        let mut stacks = world.query::<&FloorItem>();
        let mut golds = world.query::<&FloorGold>();
        assert!(
            stacks.iter(&world).count() + golds.iter(&world).count() > 0,
            "scatter_objects placed nothing in 10 attempts"
        );
    }
}

