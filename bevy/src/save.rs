//! Save/load: the full game state (character, inventory, quest chain,
//! shop stocks, created artifacts and the current level with its
//! monsters and floor items) serialized to `savegame.ron` in the crate
//! root. Quitting with 'Q' saves; dying deletes the save (permadeath).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::game::{GridPos, Monster, MonsterExtra, PlayerState, PlotQuest};
use crate::item::{self, Inventory, Item};
use crate::map::Map;
use crate::town::ShopStocks;

/// One saved monster (plain data mirror of the Monster component).
#[derive(Clone, Serialize, Deserialize)]
pub struct MonsterSave {
    pub def: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub energy: i32,
    pub awake: bool,
    #[serde(default)]
    pub friendly: bool,
    #[serde(default)]
    pub quest: bool,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub cut: i32,
    #[serde(default)]
    pub poison: i32,
    #[serde(default)]
    pub fear: i32,
    #[serde(default)]
    pub companion: bool,
    /// MSTATUS_PET (see game::Monster::pet).
    #[serde(default)]
    pub pet: bool,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub mimic: u32,
    #[serde(default)]
    pub smart: u32,
    #[serde(default)]
    pub mspeed_mod: i32,
    #[serde(default)]
    pub stun: i32,
    #[serde(default)]
    pub confused: i32,
    #[serde(default)]
    pub held_artifact: Option<u32>,
    #[serde(default)]
    pub held_object: Option<u32>,
    #[serde(default)]
    pub items: Vec<item::Item>,
    #[serde(default)]
    pub looted: bool,
    #[serde(default)]
    pub controlled: bool,
    #[serde(default)]
    pub mon_level: i32,
    #[serde(default)]
    pub detected: i32,
    #[serde(default)]
    pub neutral: bool,
    /// POSSESSOR soul body's original race (ai_deincarnate).
    #[serde(default)]
    pub possessor: Option<usize>,
    /// do_melkor_curse instance armour modifier.
    #[serde(default)]
    pub ac_mod: i32,
    /// do_melkor_curse melee dice penalty.
    #[serde(default)]
    pub blow_penalty: i32,
    /// Per-monster state kept in the separate `MonsterExtra` component
    /// (monster experience / MFLAG_NICE / reproduction).
    #[serde(default)]
    pub extra: MonsterExtra,
}

impl MonsterSave {
    /// Snapshot without access to the monster's `MonsterExtra` component
    /// (legacy call sites; the extra state falls back to defaults).
    pub fn from_monster(m: &Monster, p: &GridPos) -> MonsterSave {
        MonsterSave::from_monster_with_extra(m, None, p)
    }

    /// Snapshot including the optional `MonsterExtra` component.
    pub fn from_monster_with_extra(
        m: &Monster,
        extra: Option<&MonsterExtra>,
        p: &GridPos,
    ) -> MonsterSave {
        MonsterSave {
            def: m.def,
            hp: m.hp,
            max_hp: m.max_hp,
            energy: m.energy,
            awake: m.awake,
            friendly: m.friendly,
            quest: m.quest,
            x: p.x,
            y: p.y,
            cut: m.cut,
            poison: m.poison,
            fear: m.fear,
            companion: m.companion,
            pet: m.pet,
            gold: m.gold,
            mimic: m.mimic,
            smart: m.smart,
            mspeed_mod: m.mspeed_mod,
            stun: m.stun,
            confused: m.confused,
            held_artifact: m.held_artifact,
            held_object: m.held_object,
            items: m.items.clone(),
            looted: m.looted,
            controlled: m.controlled,
            mon_level: m.mon_level,
            detected: m.detected,
            neutral: m.neutral,
            possessor: m.possessor,
            ac_mod: m.ac_mod,
            blow_penalty: m.blow_penalty,
            extra: extra.cloned().unwrap_or_default(),
        }
    }

    /// The `MonsterExtra` component to respawn alongside the monster.
    pub fn to_extra(&self) -> MonsterExtra {
        self.extra.clone()
    }

    pub fn to_monster(&self) -> Monster {
        Monster {
            def: self.def,
            hp: self.hp,
            max_hp: self.max_hp,
            energy: self.energy,
            awake: self.awake,
            friendly: self.friendly,
            quest: self.quest,
            cut: self.cut,
            poison: self.poison,
            fear: self.fear,
            companion: self.companion,
            pet: self.pet,
            neutral: self.neutral,
            gold: self.gold,
            mimic: self.mimic,
            smart: self.smart,
            mspeed_mod: self.mspeed_mod,
            stun: self.stun,
            confused: self.confused,
            held_artifact: self.held_artifact,
            held_object: self.held_object,
            items: self.items.clone(),
            looted: self.looted,
            seen: false,
            controlled: self.controlled,
            mon_level: self.mon_level,
            detected: self.detected,
            possessor: self.possessor,
            ac_mod: self.ac_mod,
            blow_penalty: self.blow_penalty,
            target: None,
        }
    }
}

/// A whole level's transient state (kept per depth for persistence).
#[derive(Clone, Serialize, Deserialize)]
pub struct LevelSnap {
    pub map: Map,
    pub monsters: Vec<MonsterSave>,
    pub floor_items: Vec<(i32, i32, Vec<Item>)>,
    pub floor_gold: Vec<(i32, i32, i32)>,
    pub player_pos: (i32, i32),
}

/// The identity of a persistent level: wilderness cells, dungeon depths
/// and plot-quest levels are tracked separately (with several dungeons,
/// an absolute depth alone is no longer unique).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LevelKey {
    Wild(i32, i32),
    /// The world overview map (always rebuilt).
    Overview,
    Dungeon {
        dungeon: u32,
        depth: u32,
    },
    Quest(u32),
}

/// `level_marker` (level_marker.hpp:8): the generation state of a level.
/// `do_level_marker` (loadsave.cc:589) serializes it through
/// `level_marker_values()`; the port keeps the same three names.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LevelMarker {
    Normal,
    Special,
    Regenerate,
}

impl LevelMarker {
    /// `level_marker_values()` (level_marker.cc:3): the original
    /// EnumStringMap's strings.
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            LevelMarker::Normal => "normal",
            LevelMarker::Special => "special",
            LevelMarker::Regenerate => "regenerate",
        }
    }

    /// The parse half of `EnumStringMap`; `None` where
    /// `do_level_marker` reports "Bad level marker" and aborts.
    #[allow(dead_code)]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(LevelMarker::Normal),
            "special" => Some(LevelMarker::Special),
            "regenerate" => Some(LevelMarker::Regenerate),
            _ => None,
        }
    }

    /// `is_normal_level` (level_marker.hpp:17).
    #[allow(dead_code)]
    pub fn is_normal(self) -> bool {
        self == LevelMarker::Normal
    }
}

/// Every persistent level (one carrying a `@:<depth>:S:` save extension)
/// the player has visited and left, plus the wilderness world-map state.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct LevelStore {
    pub levels: std::collections::HashMap<LevelKey, LevelSnap>,
    #[serde(default)]
    pub wilderness: crate::game::Wilderness,
}

#[derive(Serialize, Deserialize)]
pub struct SaveGame {
    pub ps: PlayerState,
    pub inv: Inventory,
    #[serde(default)]
    pub plot: PlotQuest,
    pub stocks: ShopStocks,
    pub created: HashSet<u32>,
    pub player_pos: (i32, i32),
    pub map: Map,
    pub monsters: Vec<MonsterSave>,
    pub floor_items: Vec<(i32, i32, Vec<Item>)>,
    pub floor_gold: Vec<(i32, i32, i32)>,
    #[serde(default)]
    pub levels: LevelStore,
    /// z-rand.cc current-RNG state at save time (`get_complex_rng_state`),
    /// restored by setup_level so a reloaded game continues the stream.
    #[serde(default)]
    pub rng_state: String,
    /// loadsave.cc `do_options`: the player preferences travel with the
    /// save (the port keeps them in one struct instead of the original's
    /// 256 bit flags plus named values).
    #[serde(default)]
    pub options: crate::options::Options,
}

/// A save game waiting to be restored by the next level setup.
#[derive(Resource)]
pub struct PendingLoad(pub SaveGame);

pub fn path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/savegame.ron").to_string()
}

pub fn exists() -> bool {
    std::path::Path::new(&path()).exists()
}

pub fn delete() {
    let _ = std::fs::remove_file(path());
}

/// Peek at the save file for the continue-menu line (name + level).
pub fn peek() -> Option<(String, u32, u32)> {
    let text = std::fs::read_to_string(path()).ok()?;
    let save: SaveGame = ron::from_str(&text).ok()?;
    Some((save.ps.name, save.ps.level, save.ps.depth))
}

pub fn load() -> Option<SaveGame> {
    let text = std::fs::read_to_string(path()).ok()?;
    load_text(&text)
}

/// `load_player`'s post-checks (loadsave.cc:2644): a save whose game turn
/// is still zero is rejected as a broken savefile (`if (!turn) err = -1`).
fn load_text(text: &str) -> Option<SaveGame> {
    let save = parse(text)?;
    if save.ps.turn == 0 {
        return None;
    }
    Some(save)
}

pub fn store(save: &SaveGame) {
    save_player(save);
}

/// Parse the RON text of a savefile (`rd_savefile` reads the file first).
pub fn parse(text: &str) -> Option<SaveGame> {
    ron::from_str(text).ok()
}

/// Serialize `save` and write it to `target` (`save_player_aux`).
#[allow(dead_code)] pub(crate) fn write_save(target: &str, save: &SaveGame) -> std::io::Result<()> {
    let text = ron::to_string(save)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(target, text)
}

/// `save_player_aux` (loadsave.cc:2709): the medium-level saver.  The
/// port has a single named slot (`savegame.ron`), so `name` is kept for
/// signature parity with the original.  A failed write removes the broken
/// file (`fd_kill`) exactly like the original.
#[allow(dead_code)] pub fn save_player_aux(name: &str, save: &SaveGame) -> std::io::Result<()> {
    let _ = name;
    let result = write_save(&path(), save);
    if result.is_err() {
        let _ = std::fs::remove_file(path());
    }
    result
}

/// `save_player` (loadsave.cc:2758): attempt to save the player in a
/// savefile.
pub fn save_player(save: &SaveGame) {
    match save_player_swap(&path(), save) {
        Ok(()) => info!("game saved"),
        Err(e) => warn!("save failed: {}", e),
    }
}

/// `load_player` (loadsave.cc:2566): attempt to load the savefile.
pub fn load_player() -> Option<SaveGame> {
    load()
}

/// The `.new`/`.old` dance of `save_player` (loadsave.cc:2758): write the
/// new save to `target.new`, preserve the current file as `target.old`,
/// rename the new file into place and delete the preserved one
/// (`fd_kill`/`fd_move`/`fd_kill`).  Any failure leaves the current save
/// untouched and removes the broken `.new` (the original kills it when
/// `do_savefile_aux` or `my_fclose` fail).
#[allow(dead_code)] pub(crate) fn save_player_swap(target: &str, save: &SaveGame) -> std::io::Result<()> {
    let safe = format!("{}.new", target);
    let temp = format!("{}.old", target);
    let _ = std::fs::remove_file(&safe);
    let result = write_save(&safe, save).and_then(|()| {
        let _ = std::fs::remove_file(&temp);
        let _ = std::fs::rename(target, &temp);
        std::fs::rename(&safe, target)?;
        let _ = std::fs::remove_file(&temp);
        Ok(())
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&safe);
    }
    result
}

/// Faithful port of the original binary savefile layer (`loadsave.cc`:
/// `sf_get`/`sf_put` plus the size-aware `do_*` routines).  The port's own
/// save format is the RON text produced by serde, so this codec is kept as
/// the reference implementation of the original byte layout: little-endian
/// integers, `u32` length-prefixed strings, counted containers and 64-byte
/// seeds.  `Sf::notes` collects the warnings the original printed through
/// `note()`.
#[allow(dead_code)] pub mod legacy {
    use std::collections::{BTreeMap, HashSet};
    use std::hash::Hash;

    /// `ls_flag_t` (loadsave.cc:72).
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum LsFlag {
        Load,
        Save,
    }

    /// The `FILE *fff` handle plus the `note()` output.  Reading past the
    /// end yields 0xFF exactly like `getc()` returning EOF (`& 0xFF`).
    #[derive(Default)]
    pub struct Sf {
        pub buf: Vec<u8>,
        pub pos: usize,
        pub notes: Vec<String>,
    }

    impl Sf {
        pub fn writing() -> Self {
            Self::default()
        }

        pub fn reading(buf: Vec<u8>) -> Self {
            Sf {
                buf,
                pos: 0,
                notes: Vec::new(),
            }
        }

        /// `sf_get` (loadsave.cc:93): `getc(fff) & 0xFF`.
        pub fn sf_get(&mut self) -> u8 {
            let c = self.buf.get(self.pos).copied().unwrap_or(0xFF);
            self.pos += 1;
            c
        }

        /// `sf_put` (loadsave.cc:105).
        pub fn sf_put(&mut self, v: u8) {
            self.buf.push(v);
        }

        /// `note` (loadsave.cc:55): collected instead of drawn.
        pub fn note(&mut self, msg: impl Into<String>) {
            self.notes.push(msg.into());
        }
    }

    /// `do_byte` (loadsave.cc:114).
    pub fn do_byte(sf: &mut Sf, v: &mut u8, flag: LsFlag) {
        match flag {
            LsFlag::Load => *v = sf.sf_get(),
            LsFlag::Save => sf.sf_put(*v),
        }
    }

    /// `do_char` (loadsave.cc:132): the byte image of a `char`.
    pub fn do_char(sf: &mut Sf, c: &mut i8, flag: LsFlag) {
        let mut b = *c as u8;
        do_byte(sf, &mut b, flag);
        if flag == LsFlag::Load {
            *c = b as i8;
        }
    }

    /// `do_std_bool` (loadsave.cc:137): any nonzero byte is true.
    pub fn do_std_bool(sf: &mut Sf, x: &mut bool, flag: LsFlag) {
        match flag {
            LsFlag::Load => *x = sf.sf_get() != 0,
            LsFlag::Save => sf.sf_put(if *x { 1 } else { 0 }),
        }
    }

    /// `do_u16b` (loadsave.cc:155): little-endian.
    pub fn do_u16b(sf: &mut Sf, v: &mut u16, flag: LsFlag) {
        match flag {
            LsFlag::Load => {
                *v = sf.sf_get() as u16;
                *v |= (sf.sf_get() as u16) << 8;
            }
            LsFlag::Save => {
                sf.sf_put((*v & 0xFF) as u8);
                sf.sf_put(((*v >> 8) & 0xFF) as u8);
            }
        }
    }

    /// `do_s16b` (loadsave.cc:176): the same bytes as `do_u16b`.
    pub fn do_s16b(sf: &mut Sf, v: &mut i16, flag: LsFlag) {
        let mut u = *v as u16;
        do_u16b(sf, &mut u, flag);
        if flag == LsFlag::Load {
            *v = u as i16;
        }
    }

    /// `do_u32b` (loadsave.cc:181): little-endian.
    pub fn do_u32b(sf: &mut Sf, v: &mut u32, flag: LsFlag) {
        match flag {
            LsFlag::Load => {
                *v = sf.sf_get() as u32;
                *v |= (sf.sf_get() as u32) << 8;
                *v |= (sf.sf_get() as u32) << 16;
                *v |= (sf.sf_get() as u32) << 24;
            }
            LsFlag::Save => {
                sf.sf_put((*v & 0xFF) as u8);
                sf.sf_put(((*v >> 8) & 0xFF) as u8);
                sf.sf_put(((*v >> 16) & 0xFF) as u8);
                sf.sf_put(((*v >> 24) & 0xFF) as u8);
            }
        }
    }

    /// `do_s32b` (loadsave.cc:205): the same bytes as `do_u32b`.
    pub fn do_s32b(sf: &mut Sf, v: &mut i32, flag: LsFlag) {
        let mut u = *v as u32;
        do_u32b(sf, &mut u, flag);
        if flag == LsFlag::Load {
            *v = u as i32;
        }
    }

    /// `do_int` (loadsave.cc:210): `int` travels as a `u32b`.
    pub fn do_int(sf: &mut Sf, v: &mut i32, flag: LsFlag) {
        do_s32b(sf, v, flag);
    }

    /// `save_std_string` (loadsave.cc:227): `u32` length + raw bytes.
    pub fn save_std_string(sf: &mut Sf, s: &str) {
        let mut n = s.len() as u32;
        do_u32b(sf, &mut n, LsFlag::Save);
        for b in s.as_bytes() {
            sf.sf_put(*b);
        }
    }

    /// `load_std_string` (loadsave.cc:239).
    pub fn load_std_string(sf: &mut Sf) -> String {
        let mut n = 0u32;
        do_u32b(sf, &mut n, LsFlag::Load);
        let mut bytes = Vec::with_capacity(n as usize);
        for _ in 0..n {
            bytes.push(sf.sf_get());
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// `do_std_string` (loadsave.cc:259).
    pub fn do_std_string(sf: &mut Sf, s: &mut String, flag: LsFlag) {
        match flag {
            LsFlag::Load => *s = load_std_string(sf),
            LsFlag::Save => save_std_string(sf, s),
        }
    }

    /// The `option_value` record (loadsave.cc:82).
    #[derive(Clone, Default, PartialEq, Eq, Debug)]
    pub struct OptionValue {
        pub name: String,
        pub value: bool,
    }

    /// `do_option_value` (loadsave.cc:272).
    pub fn do_option_value(sf: &mut Sf, ov: &mut OptionValue, flag: LsFlag) {
        do_std_string(sf, &mut ov.name, flag);
        do_std_bool(sf, &mut ov.value, flag);
    }

    /// `do_flag_set` (loadsave.cc:284): each 32-bit tier as a `u32b`.
    pub fn do_flag_set(sf: &mut Sf, flags: &mut [u32], flag: LsFlag) {
        for f in flags.iter_mut() {
            do_u32b(sf, f, flag);
        }
    }

    /// `do_vector` (loadsave.cc:292): `u32b` count then each element.
    /// Loading clears the vector, reserves the count and default-fills it.
    pub fn do_vector<T: Default + Clone>(
        sf: &mut Sf,
        flag: LsFlag,
        v: &mut Vec<T>,
        mut f: impl FnMut(&mut T, &mut Sf, LsFlag),
    ) {
        let mut n = v.len() as u32;
        do_u32b(sf, &mut n, flag);
        if flag == LsFlag::Load {
            v.clear();
            v.resize_with(n as usize, T::default);
        }
        for i in 0..n as usize {
            f(&mut v[i], sf, flag);
        }
    }

    /// `do_array` (loadsave.cc:311): `what` names the array in the
    /// "Too many ... !" warning.  The original then indexes the overflowed
    /// `n` out of bounds; the port clamps to the array length.
    pub fn do_array<T>(
        sf: &mut Sf,
        what: &str,
        flag: LsFlag,
        array: &mut [T],
        size: usize,
        mut f: impl FnMut(&mut T, &mut Sf, LsFlag),
    ) {
        let mut n = size as u32;
        do_u32b(sf, &mut n, flag);
        if flag == LsFlag::Load && n as usize > size {
            sf.note(format!(
                "Too many {}: {} > {}! Game may act strangely or crash.",
                what, n, size
            ));
        }
        for i in 0..(n as usize).min(array.len()) {
            f(&mut array[i], sf, flag);
        }
    }

    /// `do_fixed_map` (loadsave.cc:333): count then key/value pairs; on
    /// load keys no longer present are read into a dummy value and dropped.
    pub fn do_fixed_map<K, V>(
        sf: &mut Sf,
        flag: LsFlag,
        map: &mut BTreeMap<K, V>,
        mut fk: impl FnMut(&mut K, &mut Sf, LsFlag),
        mut fv: impl FnMut(&mut V, &mut Sf, LsFlag),
    ) where
        K: Ord + Default + Clone,
        V: Default + Clone,
    {
        let mut n = map.len() as u32;
        do_u32b(sf, &mut n, flag);
        if flag == LsFlag::Load {
            for _ in 0..n {
                let mut key = K::default();
                fk(&mut key, sf, flag);
                match map.get_mut(&key) {
                    Some(v) => fv(v, sf, flag),
                    None => {
                        let mut v = V::default();
                        fv(&mut v, sf, flag);
                    }
                }
            }
        } else {
            let mut entries: Vec<(K, V)> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            for (mut key, mut value) in entries.drain(..) {
                fk(&mut key, sf, flag);
                fv(&mut value, sf, flag);
            }
        }
    }

    /// `do_unordered_set` (loadsave.cc:379): the save side copies the keys
    /// out because the field function takes a mutable reference.
    pub fn do_unordered_set<K>(
        sf: &mut Sf,
        flag: LsFlag,
        set: &mut HashSet<K>,
        mut f: impl FnMut(&mut K, &mut Sf, LsFlag),
    ) where
        K: Eq + Hash + Default + Clone,
    {
        let mut n = set.len() as u32;
        do_u32b(sf, &mut n, flag);
        if flag == LsFlag::Load {
            for _ in 0..n {
                let mut key = K::default();
                f(&mut key, sf, flag);
                set.insert(key);
            }
        } else {
            let mut keys: Vec<K> = set.iter().cloned().collect();
            for key in keys.iter_mut() {
                f(key, sf, flag);
            }
        }
    }

    /// `do_bytes` (loadsave.cc:423).
    pub fn do_bytes(sf: &mut Sf, flag: LsFlag, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            do_byte(sf, b, flag);
        }
    }

    /// `seed_t::n_bytes` (seed.hpp:10).
    pub const SEED_N_BYTES: usize = 64;

    /// `seed_t` as its 64-byte image (`do_seed`, loadsave.cc:431).  The
    /// port itself stores the game seeds as `u64`/`u32` values instead
    /// (flavor_seed, wilderness and town seeds).
    pub type SeedBytes = [u8; SEED_N_BYTES];

    /// `do_seed` (loadsave.cc:431).
    pub fn do_seed(sf: &mut Sf, seed: &mut SeedBytes, flag: LsFlag) {
        do_bytes(sf, flag, seed);
    }

    /// `do_boost_optional` (loadsave.cc:449): a `u32` count (0 or 1)
    /// followed by the value when present.
    pub fn do_boost_optional<T: Default>(
        sf: &mut Sf,
        maybe: &mut Option<T>,
        flag: LsFlag,
        mut f: impl FnMut(&mut T, &mut Sf, LsFlag),
    ) {
        match flag {
            LsFlag::Save => {
                let mut n: u32 = if maybe.is_some() { 1 } else { 0 };
                do_u32b(sf, &mut n, LsFlag::Save);
                if let Some(v) = maybe.as_mut() {
                    f(v, sf, flag);
                }
            }
            LsFlag::Load => {
                let mut n = 0u32;
                do_u32b(sf, &mut n, LsFlag::Load);
                while n > 0 {
                    n -= 1;
                    let mut v = maybe.take().unwrap_or_default();
                    f(&mut v, sf, flag);
                    *maybe = Some(v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;
    use crate::map;

    /// A save round-trips through RON with all state intact.
    #[test]
    fn save_roundtrip() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut level_map = map::generate_level(&gd, 5, &mut rng).map;
        level_map.town = crate::game::TOWN_RANDOM;
        level_map.clouds.push(crate::map::Cloud {
            x: 4,
            y: 5,
            radius: 2,
            wave: true,
            max_radius: 6,
            dir: (1, 0),
            gf: "FIRE".to_string(),
            damage: 30,
            turns: 5,
        });
        level_map.shops.insert(
            map::Map::idx(3, 3),
            map::ShopMark {
                store: 40,
                ch: '1',
                color: 7,
            },
        );
        let mut ps = crate::birth::make_player(&gd, "Saver".into(), 0, 0);
        ps.level = 7;
        ps.gold = 1234;
        ps.random_towns.push(crate::game::RandomTown {
            dungeon: 22,
            depth: 33,
            town: crate::game::TOWN_RANDOM,
            seed: 42,
        });
        ps.skills.insert(2, 3 * crate::skill::SKILL_STEP);
        let mut inv = Inventory::default();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut it = Item::base(&gd, sword);
        it.to_h = 3;
        it.ego = 1;
        inv.pack.push(it);
        let save = SaveGame {
            ps,
            inv,
            plot: PlotQuest::default(),
            stocks: ShopStocks::default(),
            created: [42u32].into_iter().collect(),
            player_pos: (10, 20),
            map: level_map,
            monsters: vec![MonsterSave {
                def: 1,
                hp: 9,
                max_hp: 10,
                energy: 50,
                awake: true,
                friendly: false,
                quest: false,
                x: 3,
                y: 4,
                cut: 0,
                poison: 0,
                fear: 0,
                companion: false,
                pet: false,
                neutral: false,
                gold: 0,
                mimic: 0,
                smart: 0,
                mspeed_mod: 0,
                stun: 0,
                confused: 0,
                held_artifact: None,
                held_object: None,
                items: Vec::new(),
                looted: false,
                controlled: false,
                mon_level: 0,
                detected: 0,
                possessor: None,
                ac_mod: 0,
                blow_penalty: 0,
                extra: MonsterExtra {
                    exp: 12345,
                    nice: true,
                    repro: true,
                    partial: false,
                    blow_bonus: [0; 4],
                },
            }],
            floor_items: vec![(7, 8, vec![])],
            floor_gold: vec![(1, 2, 99)],
            levels: LevelStore::default(),
            rng_state: "00000000000000000000000000000001:00000000000000000000000000000003"
                .to_string(),
            options: crate::options::Options {
                smart_learn: true,
                ..Default::default()
            },
        };
        let text = ron::to_string(&save).expect("serialize");
        let back: SaveGame = ron::from_str(&text).expect("deserialize");
        assert_eq!(back.ps.name, "Saver");
        assert_eq!(back.ps.level, 7);
        assert_eq!(
            back.ps.skills.get(&2),
            Some(&(3 * crate::skill::SKILL_STEP))
        );
        assert_eq!(back.inv.pack[0].to_h, 3);
        assert!(back.created.contains(&42));
        assert_eq!(back.player_pos, (10, 20));
        assert_eq!(back.monsters.len(), 1);
        // MonsterExtra (exp/nice/repro) survives the round trip and is
        // split back into its component on restore.
        assert_eq!(back.monsters[0].extra.exp, 12345);
        assert!(back.monsters[0].extra.nice);
        assert!(back.monsters[0].extra.repro);
        let extra = back.monsters[0].to_extra();
        assert_eq!(extra.exp, 12345);
        assert!(back.map.terrain.len() == (map::MAP_W * map::MAP_H) as usize);
        assert_eq!(back.map.clouds.len(), 1);
        assert_eq!(back.map.clouds[0].gf, "FIRE");
        assert_eq!(back.map.clouds[0].turns, 5);
        assert!(back.map.clouds[0].wave);
        assert_eq!(back.floor_gold[0].2, 99);
        // Random dungeon towns survive the save.
        assert_eq!(back.ps.random_towns.len(), 1);
        assert_eq!(back.ps.random_towns[0].town, crate::game::TOWN_RANDOM);
        assert_eq!(back.map.town, crate::game::TOWN_RANDOM);
        assert!(back.map.shops.contains_key(&map::Map::idx(3, 3)));
        // do_options: preferences travel with the save.
        assert!(back.options.smart_learn);
        assert!(back.options.auto_scum);
    }

    #[test]
    fn rng_state_round_trips_through_the_save_file() {
        use rand::RngCore;
        // Advance the complex RNG, capture the state that a save would
        // store, then read the next draws.
        crate::rng::set_complex_rng();
        let _ = crate::rng::current().next_u64();
        let state = crate::rng::get_complex_rng_state();
        let expected: Vec<u64> = (0..4).map(|_| crate::rng::current().next_u64()).collect();
        // Move to some unrelated stream, then restore from the save.
        crate::rng::set_quick_rng(1234);
        let _ = crate::rng::current().next_u64();
        crate::rng::set_complex_rng_state(&state);
        crate::rng::set_complex_rng();
        let actual: Vec<u64> = (0..4).map(|_| crate::rng::current().next_u64()).collect();
        assert_eq!(expected, actual);
    }

    /// A minimal save with a generated level (`do_savefile_aux` writes the
    /// whole `SaveGame`).
    fn base_save(name: &str) -> SaveGame {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let map = map::generate_level(&gd, 5, &mut rng).map;
        let ps = crate::birth::make_player(&gd, name.into(), 0, 0);
        SaveGame {
            ps,
            inv: Inventory::default(),
            plot: PlotQuest::default(),
            stocks: ShopStocks::default(),
            created: HashSet::new(),
            player_pos: (10, 20),
            map,
            monsters: Vec::new(),
            floor_items: Vec::new(),
            floor_gold: Vec::new(),
            levels: LevelStore::default(),
            rng_state: String::new(),
            options: Default::default(),
        }
    }

    #[test]
    fn save_roundtrip_preserves_player_quests_levels_and_wilderness() {
        let gd = load_game_data();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut save = base_save("Deep Hero");
        // do_extra: fates, inscriptions, hit-die rolls, monster lore,
        // random spells and the named timers.
        save.ps.fates.push(crate::game::Fate {
            fate: crate::game::FATE_FIND_O,
            level: 12,
            serious: true,
            object: sword as u32,
            artifact: 0,
            monster: 0,
            know: true,
        });
        save.ps.inscriptions = vec![true, false, true];
        save.ps.hp_rolls = vec![9, 8, 7];
        save.ps.hp_planned = vec![5, 6, 7];
        save.ps.kills.insert(1, 4);
        save.ps.unique_seen.insert(2);
        save.ps.inertia_timer = 7;
        save.ps.random_spells.push(crate::game::RandomSpell {
            level: 3,
            mana: 4,
            shape: 2,
            gf: "FIRE".into(),
            radius: 2,
            dam_dice: 3,
            dam_sides: 4,
            untried: true,
        });
        // do_quests / do_towns.
        save.plot.destroyed_towns[3] = true;
        save.plot.states.insert(20, crate::game::PLOT_COMPLETED);
        // save_dungeon serialization: persistent levels live in the
        // LevelStore (map, monsters, floor items/gold, player position).
        let snap_map = save.map.clone();
        save.levels.levels.insert(
            LevelKey::Dungeon { dungeon: 1, depth: 5 },
            LevelSnap {
                map: snap_map,
                monsters: Vec::new(),
                floor_items: vec![(2, 3, Vec::new())],
                floor_gold: vec![(4, 5, 66)],
                player_pos: (2, 3),
            },
        );
        let quest_map = save.map.clone();
        save.levels.levels.insert(
            LevelKey::Quest(4),
            LevelSnap {
                map: quest_map,
                monsters: Vec::new(),
                floor_items: Vec::new(),
                floor_gold: Vec::new(),
                player_pos: (0, 0),
            },
        );
        // do_wilderness: per-cell seeds and known flags.
        save.levels.wilderness.w = 2;
        save.levels.wilderness.h = 2;
        save.levels.wilderness.seeds = vec![1, 2, 3, 4];
        save.levels.wilderness.known = vec![true, false, false, true];

        let back: SaveGame = ron::from_str(&ron::to_string(&save).unwrap()).unwrap();
        assert_eq!(back.ps.fates.len(), 1);
        assert_eq!(back.ps.fates[0].level, 12);
        assert!(back.ps.fates[0].know);
        assert_eq!(back.ps.inscriptions, vec![true, false, true]);
        assert_eq!(back.ps.hp_rolls, vec![9, 8, 7]);
        assert_eq!(back.ps.hp_planned, vec![5, 6, 7]);
        assert_eq!(back.ps.kills.get(&1), Some(&4));
        assert!(back.ps.unique_seen.contains(&2));
        assert_eq!(back.ps.inertia_timer, 7);
        assert_eq!(back.ps.random_spells[0], save.ps.random_spells[0]);
        assert_eq!(back.plot.states.get(&20), Some(&crate::game::PLOT_COMPLETED));
        assert!(back.plot.destroyed_towns[3]);
        let snap = back
            .levels
            .levels
            .get(&LevelKey::Dungeon { dungeon: 1, depth: 5 })
            .expect("snapshot");
        assert_eq!(snap.floor_gold[0].2, 66);
        assert_eq!(snap.player_pos, (2, 3));
        assert!(back.levels.levels.contains_key(&LevelKey::Quest(4)));
        assert_eq!(back.levels.wilderness.seeds, vec![1, 2, 3, 4]);
        assert!(back.levels.wilderness.known[0]);
        assert!(!back.levels.wilderness.known[1]);
    }

    #[test]
    fn inventory_pack_equipment_and_awareness_round_trip() {
        let gd = load_game_data();
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut save = base_save("Packrat");
        // do_inventory: pack, worn slots and the identified-kind set.
        let mut it = Item::base(&gd, sword);
        it.to_h = 3;
        it.ego = 1;
        save.inv.pack.push(it.clone());
        save.inv.equip[crate::data::SLOT_WEAPON] = Some(it);
        save.inv.known.insert(sword);
        // do_artifacts: the per-game created-artifact ids.
        save.created.insert(42);
        let back: SaveGame = ron::from_str(&ron::to_string(&save).unwrap()).unwrap();
        assert_eq!(back.inv.pack.len(), 1);
        assert_eq!(back.inv.pack[0].to_h, 3);
        assert_eq!(back.inv.pack[0].ego, 1);
        let w = back.inv.equip[crate::data::SLOT_WEAPON].as_ref().unwrap();
        assert_eq!(w.def, sword);
        assert!(back.inv.known.contains(&sword));
        assert!(back.created.contains(&42));
    }

    #[test]
    fn shop_stocks_round_trip_through_the_save_file() {
        let gd = load_game_data();
        let mut save = base_save("Trader");
        // do_store / do_stores: stock items, owner, store_open countdown
        // and the daily-turnover marker.
        let key = crate::town::skey(1, crate::town::HOME_STORE);
        let sword = gd.object_by_name("Long Sword").unwrap();
        let mut it = Item::base(&gd, sword);
        it.count = 2;
        save.stocks.stocks.insert(key, vec![it]);
        save.stocks.owners.insert(key, 4);
        save.stocks.open_until.insert(key, 123_456);
        save.stocks.last_day.insert(key, 77);
        let back: SaveGame = ron::from_str(&ron::to_string(&save).unwrap()).unwrap();
        assert_eq!(back.stocks.stocks[&key].len(), 1);
        assert_eq!(back.stocks.stocks[&key][0].count, 2);
        assert_eq!(back.stocks.owners.get(&key), Some(&4));
        assert_eq!(back.stocks.open_until.get(&key), Some(&123_456));
        assert_eq!(back.stocks.last_day.get(&key), Some(&77));
    }

    #[test]
    fn level_keys_serialize_like_the_original_level_markers() {
        let keys = vec![
            LevelKey::Wild(1, 2),
            LevelKey::Overview,
            LevelKey::Dungeon { dungeon: 3, depth: 9 },
            LevelKey::Quest(4),
        ];
        let text = ron::to_string(&keys).unwrap();
        let back: Vec<LevelKey> = ron::from_str(&text).unwrap();
        assert_eq!(back, keys);
        // do_level_marker aborts on an unknown marker; the enum simply
        // fails to deserialize.
        assert!(ron::from_str::<LevelKey>("\"NoSuchMarker\"").is_err());
    }

    #[test]
    fn level_marker_values_round_trip_the_original_strings() {
        // level_marker.cc:3 maps NORMAL/SPECIAL/REGENERATE to the
        // serialized names; level_marker.hpp:17 tests NORMAL.
        assert_eq!(LevelMarker::Normal.as_str(), "normal");
        assert_eq!(LevelMarker::Special.as_str(), "special");
        assert_eq!(LevelMarker::Regenerate.as_str(), "regenerate");
        for m in [
            LevelMarker::Normal,
            LevelMarker::Special,
            LevelMarker::Regenerate,
        ] {
            assert_eq!(LevelMarker::parse(m.as_str()), Some(m));
        }
        assert!(LevelMarker::Normal.is_normal());
        assert!(!LevelMarker::Special.is_normal());
        assert!(!LevelMarker::Regenerate.is_normal());
        // do_level_marker's "Bad level marker" path.
        assert_eq!(LevelMarker::parse("Regenerate"), None);
        assert_eq!(LevelMarker::parse(""), None);
    }

    #[test]
    fn message_log_caps_at_one_hundred_like_the_original() {
        // do_message/do_messages serialized each (text, count, color); the
        // port keeps text + count (no colour) in MessageLog and the save
        // file does not carry the log, so only the bounded log is verified.
        let mut log = crate::game::MessageLog::default();
        for i in 0..150 {
            log.add(format!("msg {}", i));
        }
        assert_eq!(log.lines.len(), 100);
        assert_eq!(log.lines.front().unwrap(), "msg 50");
        assert_eq!(log.lines.back().unwrap(), "msg 149");
    }

    #[test]
    fn save_player_swap_writes_atomically_and_keeps_the_old_file() {
        let dir = std::env::temp_dir().join(format!("tome-save-swap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("slot.ron");
        let target = target.to_str().unwrap().to_string();
        let safe = format!("{}.new", target);

        let save_a = base_save("Alice");
        save_player_swap(&target, &save_a).unwrap();
        assert_eq!(
            parse(&std::fs::read_to_string(&target).unwrap())
                .unwrap()
                .ps
                .name,
            "Alice"
        );
        assert!(!std::path::Path::new(&safe).exists());

        let save_b = base_save("Bob");
        save_player_swap(&target, &save_b).unwrap();
        assert_eq!(
            parse(&std::fs::read_to_string(&target).unwrap())
                .unwrap()
                .ps
                .name,
            "Bob"
        );

        // A failed write leaves the current save untouched.
        std::fs::create_dir(&safe).unwrap();
        let save_c = base_save("Carol");
        assert!(save_player_swap(&target, &save_c).is_err());
        assert_eq!(
            parse(&std::fs::read_to_string(&target).unwrap())
                .unwrap()
                .ps
                .name,
            "Bob"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_player_rejects_broken_and_missing_savefiles() {
        let mut save = base_save("Parser");
        save.ps.turn = 42;
        let text = ron::to_string(&save).unwrap();
        assert_eq!(load_text(&text).unwrap().ps.name, "Parser");
        // rd_savefile: a file that does not parse is rejected.
        assert!(parse("This is not a savefile").is_none());
        assert!(parse("").is_none());
        assert!(load_text("This is not a savefile").is_none());
        // load_player's paranoia: a zero turn is a broken savefile even
        // when the RON parses.
        save.ps.turn = 0;
        let zero = ron::to_string(&save).unwrap();
        assert!(parse(&zero).is_some());
        assert!(load_text(&zero).is_none());
    }

    #[test]
    fn legacy_codec_numbers_are_little_endian_and_size_aware() {
        use super::legacy::{self, LsFlag};
        let mut sf = legacy::Sf::writing();
        let mut b = 0x12u8;
        legacy::do_byte(&mut sf, &mut b, LsFlag::Save);
        let mut c = -5i8;
        legacy::do_char(&mut sf, &mut c, LsFlag::Save);
        let mut t = true;
        legacy::do_std_bool(&mut sf, &mut t, LsFlag::Save);
        let mut f = false;
        legacy::do_std_bool(&mut sf, &mut f, LsFlag::Save);
        let mut u16v = 0x1234u16;
        legacy::do_u16b(&mut sf, &mut u16v, LsFlag::Save);
        let mut s16v = -2i16;
        legacy::do_s16b(&mut sf, &mut s16v, LsFlag::Save);
        let mut u32v = 0xDEAD_BEEFu32;
        legacy::do_u32b(&mut sf, &mut u32v, LsFlag::Save);
        let mut s32v = -123_456i32;
        legacy::do_s32b(&mut sf, &mut s32v, LsFlag::Save);
        let mut iv = -7i32;
        legacy::do_int(&mut sf, &mut iv, LsFlag::Save);
        let mut bytes = [1u8, 2, 3];
        legacy::do_bytes(&mut sf, LsFlag::Save, &mut bytes);
        let mut seed = [0u8; legacy::SEED_N_BYTES];
        for (i, x) in seed.iter_mut().enumerate() {
            *x = i as u8;
        }
        legacy::do_seed(&mut sf, &mut seed, LsFlag::Save);
        assert_eq!(
            &sf.buf[..20],
            &[
                0x12, 0xFB, 1, 0, 0x34, 0x12, 0xFE, 0xFF, 0xEF, 0xBE, 0xAD, 0xDE, 0xC0, 0x1D, 0xFE,
                0xFF, 0xF9, 0xFF, 0xFF, 0xFF
            ]
        );
        assert_eq!(&sf.buf[20..23], &[1, 2, 3]);
        assert_eq!(&sf.buf[23..], &seed[..]);

        // Read everything back from the byte stream.
        let mut rd = legacy::Sf::reading(sf.buf.clone());
        let mut b2 = 0u8;
        legacy::do_byte(&mut rd, &mut b2, LsFlag::Load);
        assert_eq!(b2, 0x12);
        let mut c2 = 0i8;
        legacy::do_char(&mut rd, &mut c2, LsFlag::Load);
        assert_eq!(c2, -5);
        let mut t2 = false;
        legacy::do_std_bool(&mut rd, &mut t2, LsFlag::Load);
        assert!(t2);
        let mut f2 = true;
        legacy::do_std_bool(&mut rd, &mut f2, LsFlag::Load);
        assert!(!f2);
        let mut u16r = 0u16;
        legacy::do_u16b(&mut rd, &mut u16r, LsFlag::Load);
        assert_eq!(u16r, 0x1234);
        let mut s16r = 0i16;
        legacy::do_s16b(&mut rd, &mut s16r, LsFlag::Load);
        assert_eq!(s16r, -2);
        let mut u32r = 0u32;
        legacy::do_u32b(&mut rd, &mut u32r, LsFlag::Load);
        assert_eq!(u32r, 0xDEAD_BEEF);
        let mut s32r = 0i32;
        legacy::do_s32b(&mut rd, &mut s32r, LsFlag::Load);
        assert_eq!(s32r, -123_456);
        let mut ir = 0i32;
        legacy::do_int(&mut rd, &mut ir, LsFlag::Load);
        assert_eq!(ir, -7);
        let mut bytes2 = [0u8; 3];
        legacy::do_bytes(&mut rd, LsFlag::Load, &mut bytes2);
        assert_eq!(bytes2, [1, 2, 3]);
        let mut seed2 = [0u8; legacy::SEED_N_BYTES];
        legacy::do_seed(&mut rd, &mut seed2, LsFlag::Load);
        assert_eq!(seed2, seed);
        assert_eq!(rd.pos, sf.buf.len());

        // EOF reads as 0xFF (`getc() & 0xFF`).
        let mut eof = legacy::Sf::reading(Vec::new());
        assert_eq!(eof.sf_get(), 0xFF);
    }

    #[test]
    fn legacy_codec_strings_and_containers_follow_the_original_format() {
        use super::legacy::{self, LsFlag};
        let mut sf = legacy::Sf::writing();
        let mut s = "Túrin".to_string();
        legacy::do_std_string(&mut sf, &mut s, LsFlag::Save);
        assert_eq!(&sf.buf[..4], &(6u32).to_le_bytes());
        assert_eq!(&sf.buf[4..], "Túrin".as_bytes());

        let mut ov = legacy::OptionValue {
            name: "auto_scum".into(),
            value: true,
        };
        legacy::do_option_value(&mut sf, &mut ov, LsFlag::Save);

        let mut flags = [0x0102_0304u32, 0xFFFF_FFFF];
        legacy::do_flag_set(&mut sf, &mut flags, LsFlag::Save);

        let mut v = vec![10u16, 20, 30];
        legacy::do_vector(&mut sf, LsFlag::Save, &mut v, |x, sf, fl| {
            legacy::do_u16b(sf, x, fl)
        });
        let mut arr = [7i16, -8, 9];
        legacy::do_array(&mut sf, "attacks", LsFlag::Save, &mut arr, 3, |x, sf, fl| {
            legacy::do_s16b(sf, x, fl)
        });

        let mut map = std::collections::BTreeMap::new();
        map.insert(1i32, 100i32);
        map.insert(2i32, 200i32);
        legacy::do_fixed_map(
            &mut sf,
            LsFlag::Save,
            &mut map,
            |k, sf, fl| legacy::do_int(sf, k, fl),
            |val, sf, fl| legacy::do_int(sf, val, fl),
        );
        let mut set: std::collections::HashSet<i32> = [5i32].into_iter().collect();
        legacy::do_unordered_set(&mut sf, LsFlag::Save, &mut set, |k, sf, fl| {
            legacy::do_int(sf, k, fl)
        });

        // Read it all back.
        let mut rd = legacy::Sf::reading(sf.buf);
        let mut s2 = String::new();
        legacy::do_std_string(&mut rd, &mut s2, LsFlag::Load);
        assert_eq!(s2, "Túrin");
        let mut ov2 = legacy::OptionValue::default();
        legacy::do_option_value(&mut rd, &mut ov2, LsFlag::Load);
        assert_eq!(ov2, ov);
        let mut flags2 = [0u32; 2];
        legacy::do_flag_set(&mut rd, &mut flags2, LsFlag::Load);
        assert_eq!(flags2, flags);
        let mut v2: Vec<u16> = vec![999];
        legacy::do_vector(&mut rd, LsFlag::Load, &mut v2, |x, sf, fl| {
            legacy::do_u16b(sf, x, fl)
        });
        assert_eq!(v2, vec![10, 20, 30]);
        let mut arr2 = [0i16; 3];
        legacy::do_array(&mut rd, "attacks", LsFlag::Load, &mut arr2, 3, |x, sf, fl| {
            legacy::do_s16b(sf, x, fl)
        });
        assert_eq!(arr2, [7, -8, 9]);
        let mut map2 = std::collections::BTreeMap::new();
        map2.insert(2i32, 0i32);
        legacy::do_fixed_map(
            &mut rd,
            LsFlag::Load,
            &mut map2,
            |k, sf, fl| legacy::do_int(sf, k, fl),
            |val, sf, fl| legacy::do_int(sf, val, fl),
        );
        // The removed key 1 is read into a dummy and dropped; key 2 is
        // updated in place.
        assert_eq!(map2.len(), 1);
        assert_eq!(map2.get(&2), Some(&200));
        let mut set2 = std::collections::HashSet::new();
        legacy::do_unordered_set(&mut rd, LsFlag::Load, &mut set2, |k, sf, fl| {
            legacy::do_int(sf, k, fl)
        });
        assert!(set2.contains(&5));
    }

    #[test]
    fn legacy_codec_optional_bounds_and_notes_match_the_original() {
        use super::legacy::{self, LsFlag};
        // do_boost_optional: count 0/1 then the value when present.
        let mut sf = legacy::Sf::writing();
        let mut some: Option<i32> = Some(42);
        legacy::do_boost_optional(&mut sf, &mut some, LsFlag::Save, |v, sf, fl| {
            legacy::do_int(sf, v, fl)
        });
        let mut none: Option<i32> = None;
        legacy::do_boost_optional(&mut sf, &mut none, LsFlag::Save, |v, sf, fl| {
            legacy::do_int(sf, v, fl)
        });
        assert_eq!(&sf.buf, &[1, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0]);

        let mut rd = legacy::Sf::reading(sf.buf);
        let mut back_none: Option<i32> = None;
        legacy::do_boost_optional(&mut rd, &mut back_none, LsFlag::Load, |v, sf, fl| {
            legacy::do_int(sf, v, fl)
        });
        assert_eq!(back_none, Some(42));
        // A count of 0 leaves a pre-existing value untouched (the original
        // only emplaces inside the while loop).
        let mut back_some: Option<i32> = Some(7);
        legacy::do_boost_optional(&mut rd, &mut back_some, LsFlag::Load, |v, sf, fl| {
            legacy::do_int(sf, v, fl)
        });
        assert_eq!(back_some, Some(7));

        // do_array warns ("Too many ...") and does not over-run the array.
        let mut over = legacy::Sf::writing();
        let mut n = 5u32;
        legacy::do_u32b(&mut over, &mut n, LsFlag::Save);
        for i in 0..5 {
            let mut x = i as i32;
            legacy::do_int(&mut over, &mut x, LsFlag::Save);
        }
        let mut rd = legacy::Sf::reading(over.buf);
        let mut arr = [0i32; 3];
        legacy::do_array(&mut rd, "attacks", LsFlag::Load, &mut arr, 3, |v, sf, fl| {
            legacy::do_int(sf, v, fl)
        });
        assert_eq!(arr, [0, 1, 2]);
        assert_eq!(rd.notes.len(), 1);
        assert!(rd.notes[0].contains("Too many attacks: 5 > 3"));
    }
}
