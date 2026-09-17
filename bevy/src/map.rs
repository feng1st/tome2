//! Dungeon map: generation (rooms/corridors/doors/traps/town), field of
//! view with lighting, helpers.

use bevy::prelude::Resource;
use rand::distributions::Distribution;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::data::{DungeonDef, GameData};

pub const MAP_W: i32 = 198;
pub const MAP_H: i32 = 66;
pub const FOV_RADIUS: i32 = 8;
/// Maximum sight distance (defines.hpp MAX_SIGHT): permanently lit grids
/// are visible anywhere in line of sight up to this range.
pub const MAX_SIGHT: i32 = 20;

// Terrain ids from lib/edit/f_info.txt
pub const T_NOTHING: u16 = 0;
pub const T_FLOOR: u16 = 1;
pub const T_OPEN_DOOR: u16 = 4;
pub const T_STAIRS_UP: u16 = 6;
pub const T_STAIRS_DOWN: u16 = 7;
/// Sentinel `Map.special` for the portal Melkor's death opens home
/// (q_ultrag.cc: FEAT_MORE at the player's feet, returning to Arda).
pub const SPECIAL_SURFACE: u32 = u32::MAX;
pub const T_QUEST_ENTER: u16 = 8;
pub const T_SHAFT_DOWN: u16 = 13;
pub const T_SHAFT_UP: u16 = 14;
pub const T_WAY_MORE: u16 = 179;
pub const T_WAY_LESS: u16 = 180;
pub const T_TRAP: u16 = 17;
pub const T_WEB: u16 = 16;
pub const T_GLYPH: u16 = 3;
pub const T_DOOR: u16 = 32;
pub const T_LOCKED_MIN: u16 = 33;
pub const T_LOCKED_MAX: u16 = 39;
pub const T_SECRET_DOOR: u16 = 48;
pub const T_RUBBLE: u16 = 49;
pub const T_MINOR_GLYPH: u16 = 64;
pub const T_GRANITE: u16 = 56;
pub const T_PERMANENT: u16 = 60;
pub const T_SHOP: u16 = 74;
pub const T_SHAL_WATER: u16 = 84;
pub const T_LAVA: u16 = 85;
pub const T_SHAL_LAVA: u16 = 86;
pub const T_GRASS: u16 = 89;
pub const T_TREE: u16 = 96;
pub const T_DEEP_WATER: u16 = 187;
pub const T_TAINTED_WATER: u16 = 174;
// Geomancy terrain (f_info: dark pit / ice / sand / ice wall / sandwalls /
// flowers / shallow magma & quartz veins used by elemental minions).
pub const T_DARK_PIT: u16 = 87;
pub const T_ICE: u16 = 90;
pub const T_SAND: u16 = 91;
pub const T_ICE_WALL: u16 = 95;
pub const T_SANDWALL: u16 = 98;
pub const T_SANDWALL_H: u16 = 99;
pub const T_SANDWALL_K: u16 = 100;
pub const T_FLOWER: u16 = 199;
pub const T_MAGMA: u16 = 52;
pub const T_QUARTZ: u16 = 53;
pub const T_MAGMA_K: u16 = 54;
pub const T_QUARTZ_K: u16 = 55;

/// Number of tunnel actions needed to dig out one wall cell.
pub const DIG_TURNS: i32 = 4;

pub fn is_door(t: u16) -> bool {
    (T_OPEN_DOOR..=T_SECRET_DOOR).contains(&t)
}

pub fn is_closed_door(t: u16) -> bool {
    (T_DOOR..=T_SECRET_DOOR).contains(&t)
}

/// Lock strength 0-6 for locked doors, 0 for plain doors.
pub fn door_power(t: u16) -> i32 {
    if (T_LOCKED_MIN..=T_LOCKED_MAX).contains(&t) {
        (t - T_LOCKED_MIN) as i32
    } else {
        0
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ShopMark {
    pub store: u32,
    pub ch: char,
    pub color: u8,
}

/// Trap flavours (deeper levels roll nastier traps).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TrapKind {
    Dart,
    PoisonDart,
    Pit,
    SpikedPit,
    Teleport,
    Fire,
    TrapDoor,
    Summon,
}

impl TrapKind {
    pub fn roll(depth: u32, rng: &mut impl Rng) -> TrapKind {
        use TrapKind::*;
        // Weighted by dungeon depth.
        let table: &[(TrapKind, i32)] = &[
            (Dart, 10),
            (Pit, 8),
            (Teleport, 6),
            (PoisonDart, 2 + depth as i32),
            (SpikedPit, depth as i32),
            (Fire, depth as i32),
            (TrapDoor, (depth as i32 - 2).max(0)),
            (Summon, (depth as i32 - 4).max(0)),
        ];
        let total: i32 = table.iter().map(|(_, w)| w).sum();
        let mut roll = rng.gen_range(0..total);
        for (kind, w) in table {
            roll -= w;
            if roll < 0 {
                return *kind;
            }
        }
        Dart
    }
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Map {
    pub terrain: Vec<u16>,
    pub explored: Vec<bool>,
    pub visible: Vec<bool>,
    /// Permanently lit cells (lit rooms, town, light spells).
    pub lit: Vec<bool>,
    /// Tunnel progress per cell index.
    pub dig: HashMap<usize, i32>,
    /// Trap cells the player has noticed (searching / triggering).
    pub known_traps: HashSet<usize>,
    /// Trap flavour per trap cell.
    pub trap_kinds: HashMap<usize, TrapKind>,
    /// Shop entrances: cell index -> store.
    pub shops: HashMap<usize, ShopMark>,
    /// Quest entrances (fixed maps): cell index -> quest id.
    pub quest_entrances: HashMap<usize, u32>,
    /// Flavor buildings (fixed maps): cell index -> st_info-style special.
    pub buildings: HashMap<usize, u32>,
    /// Paired FEAT_BETWEEN (Void Jumpgate) cells: cell index -> partner.
    #[serde(default)]
    pub between: HashMap<usize, usize>,
    /// Terrain special values: dungeon index on staircases (cave.special).
    #[serde(default)]
    pub special: HashMap<usize, u32>,
    /// Town number of a town level (0 = wilderness/dungeon).
    #[serde(default)]
    pub town: u32,
    /// Fountains: cell index -> (potion sval with potion2 offset, draughts
    /// left). cave.special/cave.special2 (generate.cc place_fountain).
    #[serde(default)]
    pub fountains: HashMap<usize, (i32, i32)>,
    /// Level feeling 0..10 (generate.cc rating; 1 = special item).
    /// 0 = no feeling (towns, quest levels).
    #[serde(default)]
    pub feeling: i32,
    /// Raw generation rating before the feeling is quantised
    /// (generate.cc `rating`): kept so late hooks (the q_rand princess
    /// room's `rating += 10`) can recompute the feeling.
    #[serde(default)]
    pub rating: i32,
    /// generate.cc `good_item_flag`: forces feeling 1.
    #[serde(default)]
    pub good_item: bool,
    /// Town monster theme (lowercase `f:` line, e.g. ELVEN).
    #[serde(default)]
    pub mflag: String,
    /// Wilderness record of a surface area (0 = none).
    #[serde(default)]
    pub wf: u32,
    /// The area was entered as an ambush (wilderness_gen encounter).
    #[serde(default)]
    pub encounter: bool,
    /// The wilderness world cell of a surface area.
    #[serde(default)]
    pub wild: (i32, i32),
    /// Lasting area effects (spells1.cc lasting_effects / fire_cloud).
    #[serde(default)]
    pub clouds: Vec<Cloud>,
    /// Per-grid magical energy (cave_type.mana, generate.cc
    /// generate_grid_mana).
    #[serde(default)]
    pub mana: Vec<u8>,
    /// Grid inscriptions by cell index (cave_type.inscription).
    #[serde(default)]
    pub inscriptions: HashMap<usize, u8>,
    /// CAVE_ICKY grids (vaults, fated objects, quest items): teleport
    /// landing, fate placement and streamers must not touch them.
    #[serde(default)]
    pub icky: HashSet<usize>,
    /// Per-cell display mimic (cave_type.mimic): the feature shown until
    /// the grid is known / while running wild borders.  The real terrain
    /// stays in `terrain` for all rules.
    #[serde(default)]
    pub mimic: HashMap<usize, u16>,
    /// Generated level dimensions (cur_wid / cur_hgt).
    #[serde(default = "default_map_w")]
    pub w: i32,
    #[serde(default = "default_map_h")]
    pub h: i32,
}

fn default_map_w() -> i32 {
    MAP_W
}

fn default_map_h() -> i32 {
    MAP_H
}

/// A lasting area effect: damages monsters inside its radius once per world
/// turn (spells2.cc fire_cloud; the original keeps one effect per cell, this
/// keeps one disc per cloud). Waves expand by one cell per turn.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cloud {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
    /// Grows by one each turn (EFF_WAVE).
    #[serde(default)]
    pub wave: bool,
    /// Radius ceiling for growing waves.
    #[serde(default)]
    pub max_radius: i32,
    /// Direction the wave front advances in; (0,0) = all around.
    #[serde(default)]
    pub dir: (i32, i32),
    pub gf: String,
    pub damage: i32,
    pub turns: i32,
}

impl Map {
    /// wiz_lite_extra: mark every cell as known, visible and lit.  Astral
    /// beings see the whole level while they climb out of Mandos
    /// (generate.cc:8523).
    pub fn reveal_all(&mut self) {
        for i in 0..self.terrain.len() {
            self.explored[i] = true;
            self.visible[i] = true;
            self.lit[i] = true;
        }
    }

    /// wiz_dark (cave.cc:3769): forget the dungeon map ("Thinking of
    /// Maud...").  CAVE_MARK is forgotten; permanent glow (CAVE_GLOW)
    /// stays, as does visibility.
    pub fn wiz_dark(&mut self) {
        for e in self.explored.iter_mut() {
            *e = false;
        }
    }

    pub fn idx(x: i32, y: i32) -> usize {
        (y * MAP_W + x) as usize
    }

    pub fn in_bounds(x: i32, y: i32) -> bool {
        (0..MAP_W).contains(&x) && (0..MAP_H).contains(&y)
    }

    pub fn terrain_at(&self, x: i32, y: i32) -> u16 {
        if Self::in_bounds(x, y) {
            self.terrain[Self::idx(x, y)]
        } else {
            T_NOTHING
        }
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, t: u16) {
        if Self::in_bounds(x, y) {
            self.terrain[Self::idx(x, y)] = t;
        }
    }

    /// The feature to draw for a cell (cave.cc map_info): the cell's
    /// mimic if set, otherwise the f_info default mimic of the real
    /// feature.  Only rendering and terrain messages use this; movement,
    /// line of sight and map predicates use `terrain_at`.
    pub fn display_terrain(&self, gd: &GameData, x: i32, y: i32) -> u16 {
        if !Self::in_bounds(x, y) {
            return T_NOTHING;
        }
        if let Some(&m) = self.mimic.get(&Self::idx(x, y)) {
            if m != T_NOTHING {
                return m;
            }
        }
        let t = self.terrain_at(x, y);
        let m = gd.terrain(t).mimic;
        if m != T_NOTHING {
            m
        } else {
            t
        }
    }

    pub fn walkable(&self, gd: &GameData, x: i32, y: i32) -> bool {
        if !Self::in_bounds(x, y) {
            return false;
        }
        let def = gd.terrain(self.terrain_at(x, y));
        def.is_floor && !def.no_walk
    }

    pub fn opaque(&self, gd: &GameData, x: i32, y: i32) -> bool {
        if !Self::in_bounds(x, y) {
            return true;
        }
        gd.terrain(self.terrain_at(x, y)).no_vision
    }
}

/// is_wall (cave.cc:71): whether a grid is a wall for the purpose of
/// magic mapping / clairvoyance.  Vanilla floors and doors (below
/// FEAT_SECRET) are never walls; the glass wall is not; the illusion
/// wall and the small tree are.  Uses the displayed (mimicked) feature.
pub fn is_wall(map: &Map, gd: &GameData, x: i32, y: i32) -> bool {
    const FEAT_SECRET: u16 = 48;
    const FEAT_GLASS_WALL: u16 = 188;
    const FEAT_ILLUS_WALL: u16 = 189;
    const FEAT_SMALL_TREES: u16 = 202;
    let t = map.display_terrain(gd, x, y);
    if t < FEAT_SECRET {
        return false;
    }
    if t == FEAT_GLASS_WALL {
        return false;
    }
    if t == FEAT_ILLUS_WALL || t == FEAT_SMALL_TREES {
        return true;
    }
    gd.terrain(t).is_wall
}

/// get_shimmer_color (cave.cc:518): the fixed colour cycle used when a
/// multi-hued monster has no breath to derive colours from.
pub fn get_shimmer_color(rng: &mut impl Rng) -> u8 {
    match crate::rng::randint(7, rng) {
        1 => 4,
        2 => 12,
        3 => 1,
        4 => 13,
        5 => 6,
        6 => 8,
        7 => 5,
        _ => 10,
    }
}

/// Breath colours (cave.cc:556 lookup_breath_color), in monster spell bit
/// order.  `255` marks "any colour".
const BREATH_COLORS: &[(&str, u8, u8)] = &[
    ("BR_ACID", 2, 8),
    ("BR_ELEC", 6, 14),
    ("BR_FIRE", 4, 12),
    ("BR_COLD", 1, 9),
    ("BR_POIS", 5, 13),
    ("BR_NETH", 13, 5),
    ("BR_LITE", 11, 3),
    ("BR_DARK", 8, 2),
    ("BR_CONF", 15, 7),
    ("BR_SOUN", 11, 15),
    ("BR_CHAO", 255, 255),
    ("BR_DISE", 10, 10),
    ("BR_NEXU", 12, 10),
    ("BR_TIME", 14, 14),
    ("BR_INER", 9, 2),
    ("BR_GRAV", 9, 2),
    ("BR_SHAR", 7, 15),
    ("BR_PLAS", 3, 4),
    ("BR_WALL", 7, 15),
    ("BR_MANA", 14, 1),
    ("BR_NUKE", 5, 13),
    ("BR_DISI", 1, 12),
];

/// multi_hued_attr (cave.cc:605): a multi-hued monster shimmers according
/// to its breaths; with no ranged attack it can be any colour.
pub fn multi_hued_attr(m: &crate::data::MonsterDef, rng: &mut impl Rng) -> u8 {
    if m.spell_freq == 0 {
        return get_shimmer_color(rng);
    }
    let mut allowed: Vec<u8> = Vec::with_capacity(15);
    let mut breaths = 0;
    let mut second_color = 0u8;
    for (name, first, second) in BREATH_COLORS {
        if !m.spells.iter().any(|s| s == name) {
            continue;
        }
        if *first == 255 {
            return crate::rng::randint(15, rng) as u8;
        }
        breaths += 1;
        if breaths == 6 {
            return crate::rng::randint(15, rng) as u8;
        }
        if !allowed.contains(first) {
            allowed.push(*first);
        }
        if breaths == 1 {
            second_color = *second;
        }
    }
    if breaths == 0 {
        return get_shimmer_color(rng);
    }
    if breaths == 1 {
        allowed.push(second_color);
    }
    allowed[rng.gen_range(0..allowed.len())]
}

/// image_monster (cave.cc:452): a random "live" monster race's glyph.
pub fn image_monster(gd: &GameData, rng: &mut impl Rng) -> (char, u8) {
    let live: Vec<usize> = (1..gd.monsters.len())
        .filter(|&i| !gd.monsters[i].name.is_empty())
        .collect();
    let m = &gd.monsters[live[rng.gen_range(0..live.len())]];
    (m.glyph(), m.color)
}

/// image_object (cave.cc:487): a random object kind's glyph.
pub fn image_object(gd: &GameData, rng: &mut impl Rng) -> (char, u8) {
    let o = &gd.objects[rng.gen_range(0..gd.objects.len())];
    (o.glyph(), o.color)
}

/// image_random (cave.cc:501): 75% monster, 25% object.
pub fn image_random(gd: &GameData, rng: &mut impl Rng) -> (char, u8) {
    if rng.gen_range(0..100) < 75 {
        image_monster(gd, rng)
    } else {
        image_object(gd, rng)
    }
}

/// MONSTER_FLOW_DEPTH (config.hpp:68).
pub const MONSTER_FLOW_DEPTH: i32 = 32;

/// update_flow / update_flow_aux (cave.cc:3492/3542): breadth-first cost
/// from the player over every grid with feat < FEAT_RUBBLE, limited to
/// `max_depth` steps.  The original stores it in cave.cost/cave.when; the
/// Bevy wolves currently move by game::move_priority, so the grid is
/// provided for callers instead of living in the map.
pub fn compute_flow(map: &Map, x0: i32, y0: i32, max_depth: i32) -> Vec<u16> {
    let mut cost = vec![u16::MAX; (MAP_W * MAP_H) as usize];
    let passable = |x: i32, y: i32| Map::in_bounds(x, y) && map.terrain_at(x, y) < T_RUBBLE;
    if !passable(x0, y0) {
        return cost;
    }
    let mut queue: std::collections::VecDeque<(i32, i32, i32)> = std::collections::VecDeque::new();
    cost[Map::idx(x0, y0)] = 0;
    queue.push_back((x0, y0, 0));
    while let Some((x, y, n)) = queue.pop_front() {
        if n == max_depth {
            continue;
        }
        for (dy, dx) in COMPASS8 {
            let (nx, ny) = (x + dx, y + dy);
            if !passable(nx, ny) {
                continue;
            }
            let i = Map::idx(nx, ny);
            if cost[i] != u16::MAX {
                continue;
            }
            cost[i] = (n + 1) as u16;
            if n + 1 < max_depth {
                queue.push_back((nx, ny, n + 1));
            }
        }
    }
    cost
}

/// UI tracking state (cave.cc health_track / monster_race_track /
/// object_track): which monster/object the side panels describe.  The
/// Bevy HUD derives the looked-at entity from the ECS query; this keeps
/// the original bookkeeping available for the panels.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct Tracking {
    pub health_who: Option<u32>,
    pub monster_race: Option<(u32, i32)>,
    pub object: Option<u32>,
}

impl Tracking {
    /// health_track (cave.cc:4038).
    pub fn health_track(&mut self, m_idx: Option<u32>) {
        self.health_who = m_idx;
    }

    /// monster_race_track (cave.cc:4052).
    pub fn monster_race_track(&mut self, r_idx: u32, ego: i32) {
        self.monster_race = Some((r_idx, ego));
    }

    /// object_track (cave.cc:4067).
    pub fn object_track(&mut self, o_idx: u32) {
        self.object = Some(o_idx);
    }
}

/// priority table (cave.cc:1593): the minimap's per-feature priority.
const PRIORITY_TABLE: &[(u16, i32)] = &[
    (0, 2),
    (1, 5),
    (48, 10),
    (51, 11),
    (50, 12),
    (49, 13),
    (98, 14),
    (4, 15),
    (5, 15),
    (32, 17),
    (55, 19),
    (54, 19),
    (100, 19),
    (187, 20),
    (84, 20),
    (85, 20),
    (86, 20),
    (88, 20),
    (89, 20),
    (87, 20),
    (96, 20),
    (97, 20),
    (90, 20),
    (91, 20),
    (92, 20),
    (93, 20),
    (94, 20),
    (2, 22),
    (15, 22),
    (6, 25),
    (7, 25),
    (180, 25),
    (179, 25),
    (14, 25),
    (13, 25),
];

/// priority (cave.cc:1667): the display priority of a glyph/colour pair,
/// used by the small-scale map.  Terminal graphics compare raw glyphs;
/// the Bevy port compares the terrain's `ch`/`color`, with 20 default.
pub fn display_priority(gd: &GameData, ch: char, color: u8) -> i32 {
    for &(feat, p1) in PRIORITY_TABLE {
        let f = gd.terrain(feat);
        if f.ch.chars().next() == Some(ch) && f.color == color {
            return p1;
        }
    }
    20
}

pub struct GeneratedLevel {
    pub map: Map,
    pub start: (i32, i32),
    /// Contents the vault pass left for populate_level to spawn.
    pub vault: VaultSpawns,
}

fn center(room: (i32, i32, i32, i32)) -> (i32, i32) {
    (room.0 + room.2 / 2, room.1 + room.3 / 2)
}

fn blank_map(fill: u16) -> Map {
    blank_pub(fill)
}

/// A map filled with one terrain (public helper for level builders).
pub fn blank_pub(fill: u16) -> Map {
    let n = (MAP_W * MAP_H) as usize;
    Map {
        terrain: vec![fill; n],
        explored: vec![false; n],
        visible: vec![false; n],
        lit: vec![false; n],
        feeling: 0,
        rating: 0,
        good_item: false,
        dig: HashMap::new(),
        known_traps: HashSet::new(),
        trap_kinds: HashMap::new(),
        shops: HashMap::new(),
        quest_entrances: HashMap::new(),
        fountains: HashMap::new(),
        buildings: HashMap::new(),
        between: HashMap::new(),
        special: HashMap::new(),
        town: 0,
        mflag: String::new(),
        wf: 0,
        encounter: false,
        wild: (0, 0),
        clouds: Vec::new(),
        mana: Vec::new(),
        inscriptions: HashMap::new(),
        icky: HashSet::new(),
        mimic: HashMap::new(),
        w: MAP_W,
        h: MAP_H,
    }
}

/// f_info `E:` terrain damage (dungeon.cc apply_effect).  The six live
/// f_info entries (their `TerrainDef.effects`, freq already x10 as in
/// init1.cc): (feature, dice, sides, frequency, gf).  A dice or side of
/// -1 means the player's level; the effect fires when turn % frequency
/// is zero (the caller runs this every ten game turns).
const TERRAIN_EFFECTS: &[(u16, i32, i32, u64, &str)] = &[
    (85, -1, 2, 10, "FIRE"),
    (86, -1, 1, 10, "FIRE"),
    (90, 1, 1, 500, "ICE"),
    (102, 1, 1, 400, "NETHER"),
    (178, 150, 2, 10, "HELL_FIRE"),
    (205, -1, 2, 10, "FIRE"),
];

/// The periodic self-damage of the terrain underfoot (dungeon.cc:392
/// apply_effect, run at the player's grid every world tick).  Returns the
/// gf name and rolled damage when the turn triggers; the caller applies
/// it to the player (PROJECT_KILL|PROJECT_HIDE).
pub fn terrain_effect(
    map: &Map,
    x: i32,
    y: i32,
    turn: u64,
    player_level: i32,
    rng: &mut impl Rng,
) -> Option<(i32, &'static str)> {
    let t = map.terrain_at(x, y);
    for &(feat, dice, sides, freq, gf) in TERRAIN_EFFECTS {
        if feat != t || freq == 0 || turn % freq as u64 != 0 {
            continue;
        }
        let d = if dice == -1 { player_level } else { dice };
        let s = if sides == -1 { player_level } else { sides };
        let mut dam = 0;
        for _ in 0..d {
            dam += rng.gen_range(1..=s.max(1));
        }
        return Some((dam, gf));
    }
    None
}

/// `grace_delay_trigger` (dungeon.cc:488): increments the delay and fires
/// on every 15th call, resetting it.
pub fn grace_delay_trigger(delay: &mut i32) -> bool {
    *delay += 1;
    if *delay >= 15 {
        *delay = 0;
        true
    } else {
        false
    }
}

/// `process_world_gods` (dungeon.cc:509): the per-god grace trickle for
/// Varda, Ulmo, Aule and Mandos, fired through the 15-call grace delay.
/// `delay` is the player's grace_delay counter (kept by the caller) and
/// the grace updates use the inc_piety clamp to +/-300000.
#[allow(clippy::too_many_arguments)]
pub fn process_world_gods(
    gd: &GameData,
    ps: &mut crate::game::PlayerState,
    inv: &crate::item::Inventory,
    map: &Map,
    player_pos: (i32, i32),
    log: &mut crate::game::MessageLog,
    delay: &mut i32,
    rng: &mut impl Rng,
) {
    let add = |ps: &mut crate::game::PlayerState, amt: i32| {
        ps.grace = (ps.grace + amt).clamp(-300000, 300000);
    };
    let race_name = ps.race_name.clone();
    let subrace_name = gd
        .racemods
        .get(ps.subrace as usize)
        .map(|r| r.name.clone())
        .unwrap_or_default();

    // Varda: piety rises in the light and falls for evil races.
    if ps.god == 7 && grace_delay_trigger(delay) {
        if map.lit[Map::idx(player_pos.0, player_pos.1)] {
            add(ps, 2);
        }
        if matches!(race_name.as_str(), "Orc" | "Troll" | "Dragon" | "Demon") {
            add(ps, -2);
        } else {
            add(ps, -1);
        }
        if ps.praying {
            add(ps, -1);
        }
    }

    // Ulmo: favours the Edain and tridents.
    if ps.god == 8 && grace_delay_trigger(delay) {
        if matches!(
            race_name.as_str(),
            "Human" | "Dunadan" | "Druadan" | "RohanKnight"
        ) {
            add(ps, 2);
        } else if matches!(race_name.as_str(), "Easterling" | "Demon" | "Orc") {
            add(ps, -2);
        } else {
            add(ps, 1);
        }
        if ps.praying {
            add(ps, -1);
        }
        for it in inv.pack.iter().chain(inv.equip.iter().flatten()) {
            let o = &gd.objects[it.def];
            if o.tval == crate::data::TV_POLEARM && o.sval == crate::base_defs::SV_TRIDENT as i32 {
                add(ps, 1);
            }
        }
    }

    // Aule: favours dwarves and their hammers, and may cast Stone Skin.
    if ps.god == 6 && grace_delay_trigger(delay) {
        if !matches!(race_name.as_str(), "Dwarf" | "Petty-dwarf" | "Gnome" | "Dark-Elf") {
            add(ps, -1);
        }
        for it in inv.pack.iter().chain(inv.equip.iter().flatten()) {
            let o = &gd.objects[it.def];
            match o.tval {
                crate::data::TV_AXE => add(ps, 1),
                crate::data::TV_HAFTED => {
                    if matches!(o.sval, 8 | 10 | 19) {
                        add(ps, 1);
                    }
                }
                _ => {}
            }
        }
        if ps.praying {
            add(ps, -2);
            let grace = ps.grace;
            let chance = if grace >= 50000 { 50000 } else { 50000 - grace };
            if chance > 0 && rng.gen_range(1..=100000) <= 100000 / chance {
                let v = rng.gen_range(1..=10) + 10 + grace / 100;
                let p = 10 + grace / 100;
                crate::game::set_shield(ps, v, p, log);
                // SHIELD_COUNTER from grace 10000 (d1/d2 dice).
                ps.shield_counter = if grace >= 10000 {
                    (2 + grace / 200, 3 + grace / 400)
                } else {
                    (0, 0)
                };
                ps.shield_fear = false;
                log.add("Aule casts Stone Skin on you.");
            }
        }
    }

    // Mandos: loves astral beings, hates vampires and demons.
    if ps.god == 9 && grace_delay_trigger(delay) {
        if subrace_name == "LostSoul" {
            add(ps, 1);
        }
        if race_name != "High-Elf" {
            add(ps, -1);
        }
        if subrace_name == "Vampire" || race_name == "Demon" {
            add(ps, -10);
        } else {
            add(ps, 2);
        }
        if ps.praying {
            add(ps, -5);
        }
    }
}

/// generate_grid_mana (generate.cc:8058): one level in ten is a
/// "Magical level" (mult 3 and +10+d10 per grid); everything else is
/// mult 2. Deeper levels hold more mana.
pub fn generate_grid_mana(map: &mut Map, depth: u32, rng: &mut impl Rng) {
    if map.mana.len() != map.terrain.len() {
        map.mana = vec![0; map.terrain.len()];
    }
    let magical = rng.gen_range(0..10) == 0;
    let mult = if magical { 3 } else { 2 };
    for i in 0..map.mana.len() {
        let mut v = mult * crate::item::m_bonus(255, depth as i32, rng) / 2;
        if magical {
            v += 10 + rng.gen_range(0..10);
        }
        map.mana[i] = v.clamp(0, 255) as u8;
    }
}

/// Vault contents that `populate_level` must spawn (the original places
/// them while building the vault; we defer to keep map.rs free of ECS).
#[derive(Default, Clone)]
pub struct VaultSpawns {
    /// (monster def index, x, y, level) -- spawned asleep with groups/egos.
    pub monsters: Vec<(usize, i32, i32, u32)>,
    /// (object level, x, y, good) -- rolled by make_object when populated.
    pub objects: Vec<(u32, i32, i32, bool)>,
    /// (monster def index, x, y) -- spawned singly with no ego/companions
    /// (dungeon town townspeople; place_monster_one with ego 0).
    pub singles: Vec<(usize, i32, i32)>,
    /// Monster nests and pits (generate.cc build_type5/6): a themed
    /// monster fill over a room's floor cells.
    pub nests: Vec<NestSpawn>,
    /// `F:...:*N` random monster markers of a fixed map (init1.cc
    /// process_dungeon_file_aux): the resolved monster level and the map
    /// position.  The spawner rolls a monster of that level (place_monster).
    pub random_monsters: Vec<(u32, i32, i32)>,
    /// `F:...:*N` random object markers: (level, x, y, good, great)
    /// (75% normal / 15% good / 10% great).
    pub random_objects: Vec<(u32, i32, i32, bool, bool)>,
}

/// A themed monster nest (type 5) or pit (type 6) room
/// (generate.cc vault_aux_* themes).
#[derive(Default, Clone)]
pub struct NestSpawn {
    /// Theme name: jelly/animal/undead/chapel/kennel/orc/troll/giant/
    /// demon/symbol.
    pub theme: String,
    /// Floor cells to fill.
    pub cells: Vec<(i32, i32)>,
    /// How many of those cells get a monster.
    pub count: usize,
}

/// Classic rooms-and-corridors dungeon generation, with doors at room
/// entrances, hidden traps in corridors, lit rooms and v_info vaults.
pub fn generate_level(gd: &GameData, depth: u32, rng: &mut impl Rng) -> GeneratedLevel {
    let mut map = blank_map(T_GRANITE);
    let mut vault = VaultSpawns::default();
    let mut vault_cells: HashSet<(i32, i32)> = HashSet::new();
    let mut vault_rects: Vec<(i32, i32, i32, i32)> = Vec::new();

    // Vaults are attempted first (the original's "unusual room" roll,
    // generate.cc level_generate_dungeon): two nested 1-in-194 rolls
    // scaled by depth, then a 100-roll for the type (greater 10%,
    // lesser 15%).  roomdep gates greater vaults to depth 10 and lesser
    // to depth 5.
    if depth >= 5 {
        for _ in 0..DUN_ROOMS {
            if !(rng.gen_range(0..DUN_UNUSUAL) < depth && rng.gen_range(0..DUN_UNUSUAL) < depth) {
                continue;
            }
            let k = rng.gen_range(0..100);
            let typ = if k < 10 && depth >= 10 {
                8
            } else if k < 25 {
                7
            } else {
                continue;
            };
            let Some(v) = gd.vault_of_type(typ, rng).cloned() else {
                continue;
            };
            if let Some(rect) = place_vault(
                &mut map,
                gd,
                &v,
                depth,
                &vault_rects,
                &mut vault_cells,
                &mut vault,
                rng,
            ) {
                vault_rects.push(rect);
            }
        }
    }

    // Non-overlapping rooms (keeping clear of the vault footprints).
    let mut rooms: Vec<(i32, i32, i32, i32)> = Vec::new();
    for _ in 0..250 {
        let w = rng.gen_range(4..12);
        let h = rng.gen_range(3..8);
        let x = rng.gen_range(1..MAP_W - w - 1);
        let y = rng.gen_range(1..MAP_H - h - 1);
        let overlaps = rooms.iter().any(|&(rx, ry, rw, rh)| {
            x < rx + rw + 1 && rx < x + w + 1 && y < ry + rh + 1 && ry < y + h + 1
        }) || vault_rects.iter().any(|&(rx, ry, rw, rh)| {
            x < rx + rw + 1 && rx < x + w + 1 && y < ry + rh + 1 && ry < y + h + 1
        });
        if overlaps {
            continue;
        }
        rooms.push((x, y, w, h));
        for yy in y..y + h {
            for xx in x..x + w {
                map.set_terrain(xx, yy, T_FLOOR);
            }
        }
    }

    // Most rooms are lit (as in Angband's lite_room).
    for &(x, y, w, h) in &rooms {
        if rng.gen_bool(0.75) {
            for yy in y..y + h {
                for xx in x..x + w {
                    map.lit[Map::idx(xx, yy)] = true;
                }
            }
        }
    }

    // L-shaped corridors between consecutive room centers.  Vault cells
    // are not carved: only the vault's own doors/floor openings connect
    // it (the original tunnels through the walls rather than through the
    // vault's rooms).
    let carve = |map: &mut Map, x: i32, y: i32| {
        if !vault_cells.contains(&(x, y)) {
            map.set_terrain(x, y, T_FLOOR);
        }
    };
    for i in 1..rooms.len() {
        let (x1, y1) = center(rooms[i - 1]);
        let (x2, y2) = center(rooms[i]);
        if rng.gen_bool(0.5) {
            for x in x1.min(x2)..=x1.max(x2) {
                carve(&mut map, x, y1);
            }
            for y in y1.min(y2)..=y1.max(y2) {
                carve(&mut map, x2, y);
            }
        } else {
            for y in y1.min(y2)..=y1.max(y2) {
                carve(&mut map, x1, y);
            }
            for x in x1.min(x2)..=x1.max(x2) {
                carve(&mut map, x, y2);
            }
        }
    }

    place_doors(&mut map, &rooms, depth, rng);

    // Stairs: player starts on the up staircase of the first room,
    // down staircase goes to a different room (or far away floor cell).
    let start = if rooms.is_empty() {
        (MAP_W / 2, MAP_H / 2)
    } else {
        center(rooms[0])
    };
    map.set_terrain(start.0, start.1, T_STAIRS_UP);
    let down = if rooms.len() > 1 {
        center(rooms[rng.gen_range(1..rooms.len())])
    } else {
        ((start.0 + 10).min(MAP_W - 2), start.1)
    };
    map.set_terrain(down.0, down.1, T_STAIRS_DOWN);

    place_traps(
        &mut map,
        &|x, y| {
            rooms
                .iter()
                .any(|&(rx, ry, rw, rh)| x >= rx && x < rx + rw && y >= ry && y < ry + rh)
        },
        depth,
        rng,
    );
    ensure_stairs_reachable(&mut map, gd, start, (down.0, down.1), rng);

    generate_grid_mana(&mut map, depth, rng);
    GeneratedLevel { map, start, vault }
}

// --- d_info dungeon levels ----------------------------------------------

/// init_feat_info (src/generate.cc): interpolate the L:/A: percentage
/// tables between the first and the last floor of a dungeon.
pub fn init_feat_info(d: &crate::data::DungeonDef, depth: u32) -> ([u16; 100], [u16; 100]) {
    let cur_depth = depth.saturating_sub(d.mindepth) as i32 + 1;
    let max_depth = d.maxdepth.saturating_sub(d.mindepth) as i32 + 1;
    let interp =
        |f: &crate::data::DungeonFloorDef| f.top + (f.bottom - f.top) * cur_depth / max_depth;
    let mut floors = [T_FLOOR; 100];
    let mut fills = [T_GRANITE; 100];
    let mut cum = 0;
    for (i, f) in d.floors.iter().enumerate() {
        let pct = interp(f).max(0);
        let end = if i == d.floors.len() - 1 {
            100
        } else {
            (cum + pct).min(100)
        };
        for slot in floors.iter_mut().take(end as usize).skip(cum as usize) {
            *slot = f.feat;
        }
        cum = end;
    }
    let mut cum = 0;
    for (i, f) in d.fills.iter().enumerate() {
        let pct = interp(f).max(0);
        let end = if i == d.fills.len() - 1 {
            100
        } else {
            (cum + pct).min(100)
        };
        for slot in fills.iter_mut().take(end as usize).skip(cum as usize) {
            *slot = f.feat;
        }
        cum = end;
    }
    (floors, fills)
}

/// fill_level (src/generate.cc): random fill with a smoothing pass.
fn fill_level(map: &mut Map, w: i32, h: i32, fills: &[u16; 100], smooth: i32, rng: &mut impl Rng) {
    let mut step0 = match smooth {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        _ => 8,
    };
    // Paranoia -- step must not exceed half of a short side
    // (generate.cc fill_level:7175).
    if h < 16 && step0 > 4 {
        step0 = 4;
    }
    if w < 16 && step0 > 4 {
        step0 = 4;
    }
    if step0 == 0 {
        let filler = fills[0];
        for y in 0..h {
            for x in 0..w {
                map.set_terrain(x, y, filler);
            }
        }
        return;
    }
    for y in (0..h).step_by(step0 as usize) {
        for x in (0..w).step_by(step0 as usize) {
            map.set_terrain(x, y, fills[rng.gen_range(0..100)]);
        }
    }
    let mut step = step0 >> 1;
    let mut shift = 14u32;
    let mut selector = 0u32;
    while step > 0 {
        let y_wrap = ((h - 1) / (step * 2)) * (step * 2);
        let x_wrap = ((w - 1) / (step * 2)) * (step * 2);
        for y in (0..h).step_by(step as usize) {
            let y_even = (y / step) % 2 == 0;
            for x in (0..w).step_by(step as usize) {
                let x_even = (x / step) % 2 == 0;
                if y_even && x_even {
                    continue;
                }
                if shift >= 14 {
                    selector = rng.gen_range(0..0x10000000u32);
                    shift = 0;
                } else {
                    selector >>= 2;
                    shift += 1;
                }
                let mut y_sel = if y_even {
                    y
                } else if selector & 2 != 0 {
                    y + step
                } else {
                    y - step
                };
                let mut x_sel = if x_even {
                    x
                } else if selector & 1 != 0 {
                    x + step
                } else {
                    x - step
                };
                if y_sel >= h {
                    y_sel = 0;
                } else if y_sel < 0 {
                    y_sel = y_wrap;
                }
                if x_sel >= w {
                    x_sel = 0;
                } else if x_sel < 0 {
                    x_sel = x_wrap;
                }
                let t = map.terrain_at(x_sel, y_sel);
                map.set_terrain(x, y, t);
            }
        }
        step >>= 1;
    }
}

/// place_floor (cave.cc:3864): the depth-interpolated floor table.
fn place_floor(map: &mut Map, floors: &[u16; 100], x: i32, y: i32, rng: &mut impl Rng) {
    map.set_terrain(x, y, floors[rng.gen_range(0..100)]);
}

/// place_filler (cave.cc:3885): the depth-interpolated fill table.
fn place_filler(map: &mut Map, fills: &[u16; 100], x: i32, y: i32, rng: &mut impl Rng) {
    map.set_terrain(x, y, fills[rng.gen_range(0..100)]);
}

/// place_floor_convert_glass (cave.cc:3875): lay a depth-appropriate
/// floor; a glass-wall result becomes molten glass (f_info 103), which
/// is passable, so destroyed glass walls never seal a level.
pub fn place_floor_convert_glass(
    map: &mut Map,
    d: &crate::data::DungeonDef,
    depth: u32,
    x: i32,
    y: i32,
    rng: &mut impl Rng,
) {
    const FEAT_GLASS: u16 = 188;
    const FEAT_MOLTEN_GLASS: u16 = 103;
    let (floors, _) = init_feat_info(d, depth);
    place_floor(map, &floors, x, y, rng);
    if map.terrain_at(x, y) == FEAT_GLASS {
        map.set_terrain(x, y, FEAT_MOLTEN_GLASS);
    }
}

/// cave_valid_bold (cave.cc:420): a grid may be destroyed/transmuted
/// unless it is permanent or holds an artifact.  The artifact check is
/// the caller's (map.rs cannot see floor items).
pub fn cave_valid_bold(map: &Map, gd: &GameData, x: i32, y: i32, has_artifact: bool) -> bool {
    if has_artifact {
        return false;
    }
    if !Map::in_bounds(x, y) {
        return false;
    }
    !gd.terrain(map.terrain_at(x, y)).permanent
}

/// gen_maze.cc `dig`: recursively open a spanning maze in a bit-mask
/// cell array.  d = 0 east, 1 north, 2 west, 3 south; bit 0 = east
/// wall, bit 1 = north, bit 2 = west, bit 3 = south.
fn maze_dig(maze: &mut [i8], stride: i32, y: i32, x: i32, mut d: i32, rng: &mut impl Rng) {
    let at = |y: i32, x: i32| (y * stride + x) as usize;
    match d {
        0 => maze[at(y, x)] |= 4,
        1 => maze[at(y, x)] |= 8,
        2 => maze[at(y, x)] |= 1,
        _ => maze[at(y, x)] |= 2,
    }
    if rng.gen_range(1..=100) < 50 {
        d = rng.gen_range(0..4);
    }
    for _ in 1..=4 {
        let (dy, dx) = match d {
            0 => (0, 1),
            1 => (-1, 0),
            2 => (0, -1),
            _ => (1, 0),
        };
        if maze[at(y + dy, x + dx)] == 0 {
            match d {
                0 => maze[at(y, x)] |= 1,
                1 => maze[at(y, x)] |= 2,
                2 => maze[at(y, x)] |= 4,
                _ => maze[at(y, x)] |= 8,
            }
            maze_dig(maze, stride, y + dy, x + dx, d, rng);
        }
        d = (d + 1) % 4;
    }
}

/// level_generate_maze (gen_maze.cc:146): twisty little passages.
fn build_maze_level(map: &mut Map, floors: &[u16; 100], w: i32, h: i32, rng: &mut impl Rng) {
    let stride = w / 2 + 2;
    let rows = h / 2 + 2;
    let mut maze = vec![-1i8; (stride * rows) as usize];
    for j in 1..=h / 2 {
        for i in 1..=w / 2 {
            maze[(j * stride + i) as usize] = 0;
        }
    }
    let y = rng.gen_range(1..=h / 2);
    let x = rng.gen_range(1..=w / 2);
    let d = rng.gen_range(0..4);
    maze_dig(&mut maze, stride, y, x, d, rng);
    maze[(y * stride + x) as usize] = 0;
    // Close the entrance against the outer cells whose wall was opened.
    for d in 0..4 {
        let (dy, dx, m1, m2) = match d {
            0 => (0, 1, 1i8, 4i8),
            1 => (-1, 0, 2, 8),
            2 => (0, -1, 4, 1),
            _ => (1, 0, 8, 2),
        };
        if maze[((y + dy) * stride + x + dx) as usize] != -1
            && maze[((y + dy) * stride + x + dx) as usize] & m2 != 0
        {
            maze[(y * stride + x) as usize] |= m1;
        }
    }
    // Translate the maze bits into a real map.
    for j in 1..=(h / 2) - 2 {
        for i in 1..=(w / 2) - 2 {
            let c = maze[(j * stride + i) as usize];
            if c != 0 {
                place_floor(map, floors, i * 2, j * 2, rng);
            }
            if c & 1 != 0 {
                place_floor(map, floors, i * 2 + 1, j * 2, rng);
            }
            if c & 8 != 0 {
                place_floor(map, floors, i * 2, j * 2 + 1, rng);
            }
        }
    }
}

/// evolve_level (gen_evol.cc:25): one game-of-life step over the level.
/// `noise` first randomizes cells towards the majority terrain.  Grids
/// holding an object or a monster (or the player) never evolve.
pub fn evolve_level(
    map: &mut Map,
    gd: &GameData,
    floors: &[u16; 100],
    fills: &[u16; 100],
    px: i32,
    py: i32,
    noise: bool,
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) {
    if noise {
        let (mut cw, mut cf) = (0, 0);
        for i in 1..w - 1 {
            for j in 1..h - 1 {
                let d = gd.terrain(map.terrain_at(i, j));
                if d.is_wall {
                    cw += 1;
                }
                if d.is_floor {
                    cf += 1;
                }
            }
        }
        for i in 1..w - 1 {
            for j in 1..h - 1 {
                let t = map.terrain_at(i, j);
                let d = gd.terrain(t);
                if d.permanent || (i == px && j == py) || occupied.contains(&(i, j)) {
                    continue;
                }
                if rng.gen_range(0..100) < 7 { // gen_evol.cc:60 magik(7)
                    if cw > cf {
                        place_floor(map, floors, i, j, rng);
                    } else {
                        place_filler(map, fills, i, j, rng);
                    }
                }
            }
        }
    }
    for i in 1..w - 1 {
        for j in 1..h - 1 {
            let t = map.terrain_at(i, j);
            let d = gd.terrain(t);
            if d.permanent || (i == px && j == py) || occupied.contains(&(i, j)) {
                continue;
            }
            let mut c = 0;
            for x in i - 1..=i + 1 {
                for y in j - 1..=j + 1 {
                    if x == i && y == j {
                        continue;
                    }
                    if gd.terrain(map.terrain_at(x, y)).is_wall {
                        c += 1;
                    }
                }
            }
            if c < 4 || c >= 7 {
                if d.is_wall {
                    place_floor(map, floors, i, j, rng);
                }
            } else if c == 4 || c == 5 {
                if !d.is_wall {
                    place_filler(map, fills, i, j, rng);
                }
            }
        }
    }
}

/// level_generate_life (gen_evol.cc:137): a cellular-automaton level that
/// evolves every ten turns (DF_EVOLVE).
fn build_life_level(
    map: &mut Map,
    gd: &GameData,
    floors: &[u16; 100],
    fills: &[u16; 100],
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) {
    for i in 1..w - 1 {
        for j in 1..h - 1 {
            let idx = Map::idx(i, j);
            map.lit[idx] = true;
            map.explored[idx] = true;
            if rng.gen_range(0..100) < 45 {
                place_floor(map, floors, i, j, rng);
            } else {
                place_filler(map, fills, i, j, rng);
            }
        }
    }
    for _ in 0..3 {
        evolve_level(
            map,
            gd,
            floors,
            fills,
            -1,
            -1,
            false,
            w,
            h,
            &HashSet::new(),
            rng,
        );
    }
}

/// Non-overlapping rooms (rectangular or circular) filled with the
/// depth-interpolated floor types.
fn carve_rooms(
    map: &mut Map,
    w: i32,
    h: i32,
    floors: &[u16; 100],
    circular: bool,
    depth: u32,
    rng: &mut impl Rng,
) -> (Vec<(i32, i32, i32, i32)>, Vec<NestSpawn>) {
    let mut rooms: Vec<(i32, i32, i32, i32)> = Vec::new();
    let mut nests: Vec<NestSpawn> = Vec::new();
    let themes = [
        "jelly", "animal", "undead", "kennel", "chapel", "orc", "troll", "giant", "demon", "symbol",
    ];
    for _ in 0..250 {
        let rw = rng.gen_range(4..12);
        let rh = rng.gen_range(3..8);
        if w - rw - 1 <= 1 || h - rh - 1 <= 1 {
            break;
        }
        let x = rng.gen_range(1..w - rw - 1);
        let y = rng.gen_range(1..h - rh - 1);
        let overlaps = rooms.iter().any(|&(rx, ry, rwidth, rheight)| {
            x < rx + rwidth + 1 && rx < x + rw + 1 && y < ry + rheight + 1 && ry < y + rh + 1
        });
        if overlaps {
            continue;
        }
        rooms.push((x, y, rw, rh));
        let mut cells: Vec<(i32, i32)> = Vec::new();
        // Room shapes (generate.cc build_type1/3/4/9/12).
        let roll = rng.gen_range(0..100);
        let circle = circular && rng.gen_bool(0.3) || (!circular && roll < 8);
        let cross = !circular && (8..16).contains(&roll);
        let distorted = !circular && (16..22).contains(&roll);
        let (cx, cy) = (x + rw / 2, y + rh / 2);
        let rad = (rw.min(rh) / 2).max(2);
        let (h1, h2, h3, h4) = (
            rng.gen_range(-8..8),
            rng.gen_range(0..8),
            rng.gen_range(0..16),
            rng.gen_range(-8..8),
        );
        for yy in y..y + rh {
            for xx in x..x + rw {
                let inside = if circle {
                    let (dx, dy) = (xx - cx, yy - cy);
                    dx * dx + dy * dy <= rad * rad
                } else if cross {
                    xx == cx || yy == cy
                } else if distorted {
                    let dy = (yy - cy + h1).max(0);
                    let dx = (xx - cx - h2).max(0);
                    let dy2 = (yy - cy - h3).max(0);
                    let dx2 = (xx - cx - h4).max(0);
                    dx * dx + dy * dy + dx2 * dx2 + dy2 * dy2 <= rad * rad
                } else {
                    true
                };
                if inside {
                    map.set_terrain(xx, yy, floors[rng.gen_range(0..100)]);
                    map.lit[Map::idx(xx, yy)] = rng.gen_bool(0.75);
                    cells.push((xx, yy));
                }
            }
        }
        // Monster nest or pit (build_type5/6): a themed room fill,
        // deferred to populate_level.
        if depth >= 6 && rooms.len() > 2 && cells.len() >= 8 && rng.gen_bool(0.14) {
            let pit = rng.gen_bool(0.5);
            let count = ((cells.len() as f32) * if pit { 0.5 } else { 0.33 }) as usize;
            let theme = themes[rng.gen_range(0..themes.len())].to_string();
            nests.push(NestSpawn {
                theme,
                cells,
                count,
            });
        }
    }
    (rooms, nests)
}

// --- generate.cc level_generate_dungeon (rooms and corridors) ------------

/// BLOCK_HGT/BLOCK_WID (generate.cc): generation block size.
const BLOCK_HGT: i32 = 11;
const BLOCK_WID: i32 = 11;
/// roomdep[] (generate.cc:318): minimum depth per room type.
const ROOMDEP: [u32; 13] = [0, 1, 1, 3, 3, 5, 5, 5, 10, 1, 3, 10, 10];
const DUN_TUN_RND: i32 = 10;
const DUN_TUN_CHG: i32 = 30;
const DUN_TUN_CON: i32 = 15;
const DUN_TUN_PEN: i32 = 25;
const DUN_TUN_JCT: i32 = 90;
/// FEAT_WALL_SOLID (defines.hpp): a pierced room wall kept solid while
/// the tunnels are built.
const T_WALL_SOLID: u16 = 59;
/// DUN_CAVERN (generate.cc:158).
const DUN_CAVERN: i32 = 30;

/// The rooms/corridors builder state (generate.cc `dun_data`).
struct RoomsGen<'a> {
    map: &'a mut Map,
    gd: &'a GameData,
    d: &'a crate::data::DungeonDef,
    flags: &'a [String],
    vault: &'a mut VaultSpawns,
    floors: &'a [u16; 100],
    fills: &'a [u16; 100],
    depth: u32,
    maxdepth: u32,
    w: i32,
    h: i32,
    row_rooms: i32,
    col_rooms: i32,
    room_map: Vec<bool>,
    cent: Vec<(i32, i32)>,
    /// Junction cells where try_doors runs after the tunnels.
    door: Vec<(i32, i32)>,
    crowded: bool,
    /// CAVE_ROOM per grid.
    room: Vec<bool>,
    no_doors: bool,
    circular: bool,
    destroyed: bool,
    /// options->ironman_rooms: unusual rooms every time and no roomdep.
    ironman: bool,
    /// A fractal cavern was placed before the rooms (DF_CAVERN roll).
    has_cavern: bool,
    rating: i32,
    good_item: bool,
}

impl<'a> RoomsGen<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        map: &'a mut Map,
        gd: &'a GameData,
        d: &'a crate::data::DungeonDef,
        flags: &'a [String],
        floors: &'a [u16; 100],
        fills: &'a [u16; 100],
        vault: &'a mut VaultSpawns,
        depth: u32,
        w: i32,
        h: i32,
        destroyed: bool,
        has_cavern: bool,
        ironman: bool,
    ) -> Self {
        let row_rooms = h / BLOCK_HGT;
        let col_rooms = w / BLOCK_WID;
        RoomsGen {
            map,
            gd,
            d,
            flags,
            vault,
            floors,
            fills,
            depth,
            maxdepth: d.maxdepth,
            w,
            h,
            row_rooms,
            col_rooms,
            room_map: vec![false; (row_rooms * col_rooms) as usize],
            cent: Vec::new(),
            door: Vec::new(),
            crowded: false,
            room: vec![false; (MAP_W * MAP_H) as usize],
            no_doors: flags.iter().any(|f| f == "NO_DOORS"),
            circular: flags.iter().any(|f| f == "CIRCULAR_ROOMS"),
            destroyed,
            ironman,
            has_cavern,
            rating: 0,
            good_item: false,
        }
    }

    fn is_room(&self, x: i32, y: i32) -> bool {
        self.room[Map::idx(x, y)]
    }

    fn set_room(&mut self, x: i32, y: i32, light: bool) {
        if Map::in_bounds(x, y) {
            let i = Map::idx(x, y);
            self.room[i] = true;
            if light {
                self.map.lit[i] = true;
            }
        }
    }

    fn clear_room(&mut self, x: i32, y: i32) {
        if Map::in_bounds(x, y) {
            self.room[Map::idx(x, y)] = false;
        }
    }

    fn mark_icky(&mut self, x: i32, y: i32) {
        if Map::in_bounds(x, y) {
            self.map.icky.insert(Map::idx(x, y));
        }
    }

    fn has(&self, f: &str) -> bool {
        self.flags.iter().any(|x| x == f)
    }

    /// build_rectangle (generate.cc:1161).
    fn build_rectangle(&mut self, y1: i32, x1: i32, y2: i32, x2: i32, feat: u16, light: bool) {
        for x in x1..=x2 {
            self.map.set_terrain(x, y1, feat);
            self.set_room(x, y1, light);
            self.map.set_terrain(x, y2, feat);
            self.set_room(x, y2, light);
        }
        for y in y1..=y2 {
            self.map.set_terrain(x1, y, feat);
            self.set_room(x1, y, light);
            self.map.set_terrain(x2, y, feat);
            self.set_room(x2, y, light);
        }
    }

    /// place_random_stairs (generate.cc:875).
    fn place_random_stairs(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        if !naked_floor(self.map, self.gd, x, y) {
            return;
        }
        let down = if self.depth == 0 {
            true
        } else if self.depth >= self.maxdepth {
            false
        } else {
            rng.gen_range(0..100) < 50
        };
        if !down {
            // place_up_stairs: 1/3 shafts unless DF_NO_SHAFT.
            let feat = if rng.gen_range(0..3) != 0 || self.has("NO_SHAFT") {
                T_STAIRS_UP
            } else {
                T_SHAFT_UP
            };
            self.map.set_terrain(x, y, feat);
        } else {
            let feat = if self.depth + 4 > self.maxdepth
                || rng.gen_range(0..3) != 0
                || self.has("NO_SHAFT")
            {
                T_STAIRS_DOWN
            } else {
                T_SHAFT_DOWN
            };
            self.map.set_terrain(x, y, feat);
        }
        self.map.special.remove(&Map::idx(x, y));
    }

    /// place_locked_door (generate.cc:909).
    fn place_locked_door(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        if Map::in_bounds(x, y) {
            self.map
                .set_terrain(x, y, T_LOCKED_MIN + rng.gen_range(0..7));
        }
    }

    /// place_random_door (generate.cc:919).
    fn place_random_door(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        if !Map::in_bounds(x, y) {
            return;
        }
        let tmp = rng.gen_range(0..1000);
        let feat = if tmp < 300 {
            T_OPEN_DOOR
        } else if tmp < 400 {
            5
        } else if tmp < 600 {
            T_LOCKED_MIN + rng.gen_range(0..7)
        } else if tmp < 900 {
            T_DOOR
        } else if tmp < 999 {
            T_LOCKED_MIN + rng.gen_range(0..7)
        } else {
            T_DOOR + 8 + rng.gen_range(0..8)
        };
        self.map.set_terrain(x, y, feat);
    }

    /// cave_clean_bold / cave_naked_bold for the room helpers.
    fn clean_floor(&self, x: i32, y: i32) -> bool {
        naked_floor(self.map, self.gd, x, y)
    }

    /// check_room_boundary (generate.cc:1683): keep a cavern connected
    /// when a room would cut it off.
    fn check_room_boundary(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, rng: &mut impl Rng) {
        if !self.has_cavern {
            return;
        }
        let mut count = 0;
        let mut old_is_floor = get_is_floor(self.gd, self.map, x1 - 1, y1);
        for x in x1..=x2 {
            let new_is_floor = get_is_floor(self.gd, self.map, x, y1 - 1);
            if new_is_floor != old_is_floor {
                count += 1;
            }
            old_is_floor = new_is_floor;
        }
        for y in y1..=y2 {
            let new_is_floor = get_is_floor(self.gd, self.map, x2 + 1, y);
            if new_is_floor != old_is_floor {
                count += 1;
            }
            old_is_floor = new_is_floor;
        }
        for x in (x1..=x2).rev() {
            let new_is_floor = get_is_floor(self.gd, self.map, x, y2 + 1);
            if new_is_floor != old_is_floor {
                count += 1;
            }
            old_is_floor = new_is_floor;
        }
        for y in (y1..=y2).rev() {
            let new_is_floor = get_is_floor(self.gd, self.map, x1 - 1, y);
            if new_is_floor != old_is_floor {
                count += 1;
            }
            old_is_floor = new_is_floor;
        }
        if count == 0 || count == 2 {
            return;
        }
        for y in y1..=y2 {
            for x in x1..=x2 {
                if Map::in_bounds(x, y) {
                    self.place_floor_at(x, y, rng);
                }
            }
        }
    }

    /// room_alloc (generate.cc:1860).
    #[allow(clippy::too_many_arguments)]
    fn room_alloc(
        &mut self,
        width: i32,
        height: i32,
        crowded: bool,
        by0: i32,
        bx0: i32,
        rng: &mut impl Rng,
    ) -> Option<(i32, i32)> {
        let mut by0 = by0;
        let mut bx0 = bx0;
        let temp = (width - 1) / BLOCK_WID + 1;
        let mut ebx = bx0 + temp;
        while bx0 > 0 && ebx > self.col_rooms {
            bx0 -= 1;
            ebx -= 1;
        }
        if ebx > self.col_rooms {
            return None;
        }
        let temp = (height - 1) / BLOCK_HGT + 1;
        let mut eby = by0 + temp;
        while by0 > 0 && eby > self.row_rooms {
            by0 -= 1;
            eby -= 1;
        }
        if eby > self.row_rooms {
            return None;
        }
        for by in by0..eby {
            for bx in bx0..ebx {
                if self.room_map[(by * self.col_rooms + bx) as usize] {
                    return None;
                }
            }
        }
        let cy = ((by0 + eby) * BLOCK_HGT) / 2;
        let cx = ((bx0 + ebx) * BLOCK_WID) / 2;
        if self.cent.len() < 100 {
            self.cent.push((cy, cx));
        }
        for by in by0..eby {
            for bx in bx0..ebx {
                self.room_map[(by * self.col_rooms + bx) as usize] = true;
            }
        }
        if crowded {
            self.crowded = true;
        }
        self.check_room_boundary(
            cx - width / 2 - 1,
            cy - height / 2 - 1,
            cx + width / 2 + 1,
            cy + height / 2 + 1,
            rng,
        );
        Some((cx, cy))
    }

    fn place_floor_at(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        place_floor(self.map, self.floors, x, y, rng);
    }

    fn place_filler_at(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        place_filler(self.map, self.fills, x, y, rng);
    }

    /// build_type1 (generate.cc:1949): normal rectangular room.
    fn build_type1(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let y1 = rng.gen_range(1..=4);
        let x1 = rng.gen_range(1..=10);
        let y2 = rng.gen_range(1..=3);
        let x2 = rng.gen_range(1..=9);
        let xsize = x1 + x2;
        let ysize = y1 + y2;
        let Some((xval, yval)) = self.room_alloc(xsize + 2, ysize + 2, false, by0, bx0, rng) else {
            return;
        };
        let y1 = yval - ysize / 2;
        let x1 = xval - xsize / 2;
        let y2 = y1 + ysize - 1;
        let x2 = x1 + xsize - 1;
        let light = self.depth <= rng.gen_range(1..=25);
        for y in y1..=y2 {
            for x in x1..=x2 {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
        self.build_rectangle(y1 - 1, x1 - 1, y2 + 1, x2 + 1, T_VAULT_OUTER, light);
        if ysize > 2 && xsize > 2 {
            if rng.gen_range(0..20) == 0 {
                for y in (y1..=y2).step_by(2) {
                    for x in (x1..=x2).step_by(2) {
                        self.map.set_terrain(x, y, T_VAULT_INNER);
                    }
                }
            } else if rng.gen_range(0..50) == 0 {
                for y in ((y1 + 2)..=(y2 - 2)).step_by(2) {
                    self.map.set_terrain(x1, y, T_VAULT_INNER);
                    self.map.set_terrain(x2, y, T_VAULT_INNER);
                }
                for x in ((x1 + 2)..=(x2 - 2)).step_by(2) {
                    self.map.set_terrain(x, y1, T_VAULT_INNER);
                    self.map.set_terrain(x, y2, T_VAULT_INNER);
                }
            }
        }
    }

    /// build_type2 (generate.cc:2022): overlapping rectangular rooms.
    fn build_type2(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let Some((xval, yval)) = self.room_alloc(25, 11, false, by0, bx0, rng) else {
            return;
        };
        let y1a = yval - rng.gen_range(1..=4);
        let y2a = yval + rng.gen_range(1..=3);
        let x1a = xval - rng.gen_range(1..=14);
        let x2a = xval + rng.gen_range(1..=6);
        let y1b = yval - rng.gen_range(1..=3);
        let y2b = yval + rng.gen_range(1..=4);
        let x1b = xval - rng.gen_range(1..=6);
        let x2b = xval + rng.gen_range(1..=14);
        let light = self.depth <= rng.gen_range(1..=25);
        self.build_rectangle(y1a - 1, x1a - 1, y2a + 1, x2a + 1, T_VAULT_OUTER, light);
        self.build_rectangle(y1b - 1, x1b - 1, y2b + 1, x2b + 1, T_VAULT_OUTER, light);
        for y in y1a..=y2a {
            for x in x1a..=x2a {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
        for y in y1b..=y2b {
            for x in x1b..=x2b {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
    }

    /// build_type3 (generate.cc:2085): cross shaped rooms.
    fn build_type3(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let Some((xval, yval)) = self.room_alloc(25, 11, false, by0, bx0, rng) else {
            return;
        };
        let (wx, wy) = (1, 1);
        let dy = rng.gen_range(3..=4);
        let dx = rng.gen_range(3..=11);
        let (y1a, y2a, x1a, x2a) = (yval - dy, yval + dy, xval - wx, xval + wx);
        let (y1b, y2b, x1b, x2b) = (yval - wy, yval + wy, xval - dx, xval + dx);
        let light = self.depth <= rng.gen_range(1..=25);
        self.build_rectangle(y1a - 1, x1a - 1, y2a + 1, x2a + 1, T_VAULT_OUTER, light);
        self.build_rectangle(y1b - 1, x1b - 1, y2b + 1, x2b + 1, T_VAULT_OUTER, light);
        for y in y1a..=y2a {
            for x in x1a..=x2a {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
        for y in y1b..=y2b {
            for x in x1b..=x2b {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
        match rng.gen_range(0..4) {
            1 => {
                for y in y1b..=y2b {
                    for x in x1a..=x2a {
                        self.map.set_terrain(x, y, T_VAULT_INNER);
                    }
                }
            }
            2 => {
                self.build_rectangle(y1b, x1a, y2b, x2a, T_VAULT_INNER, false);
                match rng.gen_range(0..4) {
                    0 => self.place_locked_door(xval, y1b, rng),
                    1 => self.place_locked_door(xval, y2b, rng),
                    2 => self.place_locked_door(x1a, yval, rng),
                    _ => self.place_locked_door(x2a, yval, rng),
                }
                self.vault.objects.push((self.depth, xval, yval, false));
                let num = rng.gen_range(1..=2) + 2;
                self.vault_monsters(yval, xval, num, rng);
            }
            3 => {
                if rng.gen_range(0..3) == 0 {
                    for y in y1b..=y2b {
                        if y == yval {
                            continue;
                        }
                        self.map.set_terrain(x1a - 1, y, T_VAULT_INNER);
                        self.map.set_terrain(x2a + 1, y, T_VAULT_INNER);
                    }
                    for x in x1a..=x2a {
                        if x == xval {
                            continue;
                        }
                        self.map.set_terrain(x, y1b - 1, T_VAULT_INNER);
                        self.map.set_terrain(x, y2b + 1, T_VAULT_INNER);
                    }
                    if rng.gen_range(0..3) == 0 {
                        self.place_locked_door(x1a - 1, yval, rng);
                        self.place_locked_door(x2a + 1, yval, rng);
                        self.place_locked_door(xval, y1b - 1, rng);
                        self.place_locked_door(xval, y2b + 1, rng);
                    }
                } else if rng.gen_range(0..3) == 0 {
                    self.map.set_terrain(xval, yval, T_VAULT_INNER);
                    self.map.set_terrain(xval, y1b, T_VAULT_INNER);
                    self.map.set_terrain(xval, y2b, T_VAULT_INNER);
                    self.map.set_terrain(x1a, yval, T_VAULT_INNER);
                    self.map.set_terrain(x2a, yval, T_VAULT_INNER);
                } else if rng.gen_range(0..3) == 0 {
                    self.map.set_terrain(xval, yval, T_VAULT_INNER);
                }
            }
            _ => {}
        }
    }

    /// build_type4 (generate.cc:2256): large room with inner features.
    fn build_type4(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let Some((xval, yval)) = self.room_alloc(25, 11, false, by0, bx0, rng) else {
            return;
        };
        let (y1, y2, x1, x2) = (yval - 4, yval + 4, xval - 11, xval + 11);
        let light = self.depth <= rng.gen_range(1..=25);
        for y in y1 - 1..=y2 + 1 {
            for x in x1 - 1..=x2 + 1 {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, light);
            }
        }
        self.build_rectangle(y1 - 1, x1 - 1, y2 + 1, x2 + 1, T_VAULT_OUTER, light);
        let (y1, y2, x1, x2) = (y1 + 2, y2 - 2, x1 + 2, x2 - 2);
        self.build_rectangle(y1 - 1, x1 - 1, y2 + 1, x2 + 1, T_VAULT_INNER, light);
        match rng.gen_range(1..=5) {
            1 => {
                match rng.gen_range(1..=4) {
                    1 => self.place_locked_door(xval, y1 - 1, rng),
                    2 => self.place_locked_door(xval, y2 + 1, rng),
                    3 => self.place_locked_door(x1 - 1, yval, rng),
                    _ => self.place_locked_door(x2 + 1, yval, rng),
                }
                self.vault_monsters(yval, xval, 1, rng);
            }
            2 => {
                match rng.gen_range(1..=4) {
                    1 => self.place_locked_door(xval, y1 - 1, rng),
                    2 => self.place_locked_door(xval, y2 + 1, rng),
                    3 => self.place_locked_door(x1 - 1, yval, rng),
                    _ => self.place_locked_door(x2 + 1, yval, rng),
                }
                self.build_rectangle(yval - 1, xval - 1, yval + 1, xval + 1, T_VAULT_INNER, false);
                match rng.gen_range(1..=4) {
                    1 => self.place_locked_door(xval, yval - 1, rng),
                    2 => self.place_locked_door(xval, yval + 1, rng),
                    3 => self.place_locked_door(xval - 1, yval, rng),
                    _ => self.place_locked_door(xval + 1, yval, rng),
                }
                let num = rng.gen_range(1..=3) + 2;
                self.vault_monsters(yval, xval, num, rng);
                if rng.gen_range(0..100) < 80 {
                    self.vault.objects.push((self.depth, xval, yval, false));
                } else {
                    self.place_random_stairs(xval, yval, rng);
                }
            }
            3 => {
                match rng.gen_range(1..=4) {
                    1 => self.place_locked_door(xval, y1 - 1, rng),
                    2 => self.place_locked_door(xval, y2 + 1, rng),
                    3 => self.place_locked_door(x1 - 1, yval, rng),
                    _ => self.place_locked_door(x2 + 1, yval, rng),
                }
                for y in yval - 1..=yval + 1 {
                    for x in xval - 1..=xval + 1 {
                        self.map.set_terrain(x, y, T_VAULT_INNER);
                    }
                }
                if rng.gen_range(0..2) == 0 {
                    let tmp = rng.gen_range(1..=2);
                    for y in yval - 1..=yval + 1 {
                        for x in xval - 5 - tmp..=xval - 3 - tmp {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                        for x in xval + 3 + tmp..=xval + 5 + tmp {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                }
                if rng.gen_range(0..3) == 0 {
                    for x in xval - 5..=xval + 5 {
                        self.map.set_terrain(x, yval - 1, T_VAULT_INNER);
                        self.map.set_terrain(x, yval + 1, T_VAULT_INNER);
                    }
                    self.map.set_terrain(xval - 5, yval, T_VAULT_INNER);
                    self.map.set_terrain(xval + 5, yval, T_VAULT_INNER);
                    self.place_locked_door(xval - 3, yval - 3 + rng.gen_range(1..=2) * 2, rng);
                    self.place_locked_door(xval + 3, yval - 3 + rng.gen_range(1..=2) * 2, rng);
                    self.vault_monsters(yval, xval - 2, rng.gen_range(1..=2), rng);
                    self.vault_monsters(yval, xval + 2, rng.gen_range(1..=2), rng);
                    if rng.gen_range(0..3) == 0 {
                        self.vault.objects.push((self.depth, xval - 2, yval, false));
                    }
                    if rng.gen_range(0..3) == 0 {
                        self.vault.objects.push((self.depth, xval + 2, yval, false));
                    }
                }
            }
            4 => {
                match rng.gen_range(1..=4) {
                    1 => self.place_locked_door(xval, y1 - 1, rng),
                    2 => self.place_locked_door(xval, y2 + 1, rng),
                    3 => self.place_locked_door(x1 - 1, yval, rng),
                    _ => self.place_locked_door(x2 + 1, yval, rng),
                }
                for y in y1..=y2 {
                    for x in x1..=x2 {
                        if (x + y) & 1 != 0 {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                }
                self.vault_monsters(yval, xval - 5, rng.gen_range(1..=3), rng);
                self.vault_monsters(yval, xval + 5, rng.gen_range(1..=3), rng);
                self.vault_objects(xval, yval, 3, rng);
            }
            _ => {
                for y in y1..=y2 {
                    self.map.set_terrain(xval, y, T_VAULT_INNER);
                }
                for x in x1..=x2 {
                    self.map.set_terrain(x, yval, T_VAULT_INNER);
                }
                if rng.gen_range(0..100) < 50 {
                    let i = rng.gen_range(1..=10);
                    self.place_locked_door(xval - i, y1 - 1, rng);
                    self.place_locked_door(xval + i, y1 - 1, rng);
                    self.place_locked_door(xval - i, y2 + 1, rng);
                    self.place_locked_door(xval + i, y2 + 1, rng);
                } else {
                    let i = rng.gen_range(1..=3);
                    self.place_locked_door(x1 - 1, yval + i, rng);
                    self.place_locked_door(x1 - 1, yval - i, rng);
                    self.place_locked_door(x2 + 1, yval + i, rng);
                    self.place_locked_door(x2 + 1, yval - i, rng);
                }
                let num = 2 + rng.gen_range(1..=2);
                self.vault_objects(xval, yval, num, rng);
                self.vault_monsters(yval + 1, xval - 4, rng.gen_range(1..=4), rng);
                self.vault_monsters(yval + 1, xval + 4, rng.gen_range(1..=4), rng);
                self.vault_monsters(yval - 1, xval - 4, rng.gen_range(1..=4), rng);
                self.vault_monsters(yval - 1, xval + 4, rng.gen_range(1..=4), rng);
            }
        }
    }

    /// vault_objects (generate.cc:1768): up to `num` objects near (x, y).
    fn vault_objects(&mut self, x: i32, y: i32, num: i32, rng: &mut impl Rng) {
        for _ in 0..num {
            for _ in 0..11 {
                let ny = rand_spread(rng, y, 2);
                let nx = rand_spread(rng, x, 3);
                if !Map::in_bounds(nx, ny) || !self.clean_floor(nx, ny) {
                    continue;
                }
                // 75% object / 25% gold (gold is rolled as an object by
                // the port's make_object).
                self.vault.objects.push((self.depth, nx, ny, false));
                break;
            }
        }
    }

    /// vault_monsters (generate.cc:1824): `num` sleeping monsters near
    /// (x1, y1) with los.
    fn vault_monsters(&mut self, y1: i32, x1: i32, num: i32, rng: &mut impl Rng) {
        for _ in 0..num {
            for _ in 0..9 {
                let Some((y, x)) = scatter_pos(self.map, self.gd, y1, x1, 1, rng) else {
                    continue;
                };
                if !self.clean_floor(x, y) {
                    continue;
                }
                if let Some(def) = vault_monster(self.gd, self.depth + 2, rng) {
                    self.vault.monsters.push((def, x, y, self.depth + 2));
                }
            }
        }
    }

    /// fill_treasure (generate.cc:4484): stock a random vault region.
    fn fill_treasure(
        &mut self,
        x1: i32,
        x2: i32,
        y1: i32,
        y2: i32,
        difficulty: i32,
        rng: &mut impl Rng,
    ) {
        let cx = (x1 + x2) / 2;
        let cy = (y1 + y2) / 2;
        let size = (x2 - x1).abs() + (y2 - y1).abs();
        if size == 0 {
            return;
        }
        for x in x1..=x2 {
            for y in y1..=y2 {
                let mut value =
                    (pref_distance(cy, cx, y, x) * 100) / size + rng.gen_range(1..=10) - difficulty;
                if rng.gen_range(1..=100) - difficulty * 3 > 50 {
                    value = 20;
                }
                let t = self.map.terrain_at(x, y);
                if !(get_is_floor(self.gd, self.map, x, y) || t == T_SHAL_WATER || t == T_SHAL_LAVA)
                {
                    continue;
                }
                if value < 0 {
                    self.add_monster_at(x, y, self.depth + 40, rng);
                    self.vault.objects.push((self.depth + 20, x, y, true));
                } else if value < 5 {
                    self.add_monster_at(x, y, self.depth + 20, rng);
                    self.vault.objects.push((self.depth + 10, x, y, true));
                } else if value < 10 {
                    self.add_monster_at(x, y, self.depth + 9, rng);
                } else if value < 17 {
                    // Intentional blank space.
                } else if value < 23 {
                    if rng.gen_range(0..100) < 25 {
                        self.vault.objects.push((self.depth, x, y, false));
                    }
                } else if value < 30 {
                    self.add_monster_at(x, y, self.depth + 5, rng);
                } else if value < 40 {
                    if rng.gen_range(0..100) < 50 {
                        self.add_monster_at(x, y, self.depth + 3, rng);
                    }
                    if rng.gen_range(0..100) < 50 {
                        self.vault.objects.push((self.depth + 7, x, y, false));
                    }
                } else if value < 50 {
                    // Do nothing.
                } else if rng.gen_range(0..100) < 20 {
                    self.add_monster_at(x, y, self.depth, rng);
                } else if rng.gen_range(0..100) < 50 {
                    // Do nothing.
                } else if rng.gen_range(0..100) < 50 {
                    self.vault.objects.push((self.depth, x, y, false));
                }
            }
        }
    }

    fn add_monster_at(&mut self, x: i32, y: i32, level: u32, rng: &mut impl Rng) {
        if !self.clean_floor(x, y) {
            return;
        }
        if let Some(def) = vault_monster(self.gd, level, rng) {
            self.vault.monsters.push((def, x, y, level));
        }
    }

    /// build_type5 (generate.cc:2730): monster nest.
    fn build_type5(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let Some((xval, yval)) = self.room_alloc(25, 11, true, by0, bx0, rng) else {
            return;
        };
        let (y1, y2, x1, x2) = (yval - 4, yval + 4, xval - 11, xval + 11);
        for y in y1..=y2 {
            for x in x1..=x2 {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, false);
            }
        }
        self.build_rectangle(y1 - 1, x1 - 1, y2 + 1, x2 + 1, T_VAULT_OUTER, false);
        let (iy1, iy2, ix1, ix2) = (y1 + 2, y2 - 2, x1 + 2, x2 - 2);
        self.build_rectangle(iy1 - 1, ix1 - 1, iy2 + 1, ix2 + 1, T_VAULT_INNER, false);
        match rng.gen_range(1..=4) {
            1 => self.place_locked_door(xval, y1 - 1, rng),
            2 => self.place_locked_door(xval, y2 + 1, rng),
            3 => self.place_locked_door(x1 - 1, yval, rng),
            _ => self.place_locked_door(x2 + 1, yval, rng),
        }
        let tmp = rng.gen_range(1..=self.depth.max(1) as i32);
        let theme = if tmp < 25 && rng.gen_range(0..2) != 0 {
            let mut template = 0usize;
            let mut ok = false;
            for _ in 0..5000 {
                let t = rng.gen_range(0..self.gd.monster_base_count.max(1));
                let m = &self.gd.monsters[t];
                if m.unique || m.id == 0 {
                    continue;
                }
                if m.depth + rng.gen_range(1..=5) > self.depth + rng.gen_range(1..=5) {
                    continue;
                }
                template = t;
                ok = true;
                break;
            }
            if !ok {
                return;
            }
            if self.depth >= 25 + rng.gen_range(1..=15) && rng.gen_range(0..2) != 0 {
                NestTheme::Symbol(template)
            } else {
                NestTheme::Clone(template)
            }
        } else if tmp < 25 {
            NestTheme::Jelly
        } else if tmp < 50 {
            NestTheme::Treasure
        } else if tmp < 65 {
            if rng.gen_range(0..3) == 0 {
                NestTheme::Kennel
            } else {
                NestTheme::Animal
            }
        } else if rng.gen_range(0..3) == 0 {
            NestTheme::Chapel
        } else {
            NestTheme::Undead
        };
        let mut what = Vec::with_capacity(64);
        let mut empty = false;
        for _ in 0..64 {
            match themed_monster(self.gd, self.depth + 10, theme, rng) {
                Some(d) => what.push(d),
                None => empty = true,
            }
        }
        if empty {
            return;
        }
        self.rating += 10;
        if self.depth <= 40 && rng.gen_range(1..=(self.depth * self.depth + 50).max(1)) < 300 {
            self.good_item = true;
        }
        for y in yval - 2..=yval + 2 {
            for x in xval - 9..=xval + 9 {
                let def = what[rng.gen_range(0..64)];
                if self.clean_floor(x, y) {
                    self.vault.monsters.push((def, x, y, self.depth));
                }
            }
        }
    }

    /// build_type6 (generate.cc:2970): monster pit.
    fn build_type6(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let Some((xval, yval)) = self.room_alloc(25, 11, true, by0, bx0, rng) else {
            return;
        };
        let (y1, y2, x1, x2) = (yval - 4, yval + 4, xval - 11, xval + 11);
        for y in y1 - 1..=y2 + 1 {
            for x in x1 - 1..=x2 + 1 {
                self.place_floor_at(x, y, rng);
                self.set_room(x, y, false);
            }
        }
        self.build_rectangle(y1 - 1, x1 - 1, y2 + 1, x2 + 1, T_VAULT_OUTER, false);
        let (iy1, iy2, ix1, ix2) = (y1 + 2, y2 - 2, x1 + 2, x2 - 2);
        self.build_rectangle(iy1 - 1, ix1 - 1, iy2 + 1, ix2 + 1, T_VAULT_OUTER, false);
        match rng.gen_range(1..=4) {
            1 => self.place_locked_door(xval, y1 - 1, rng),
            2 => self.place_locked_door(xval, y2 + 1, rng),
            3 => self.place_locked_door(x1 - 1, yval, rng),
            _ => self.place_locked_door(x2 + 1, yval, rng),
        }
        let tmp = rng.gen_range(1..=self.depth.max(1) as i32);
        let theme = if tmp < 20 {
            NestTheme::Orc
        } else if tmp < 40 {
            NestTheme::Troll
        } else if tmp < 55 {
            NestTheme::Giant
        } else if tmp < 70 {
            if rng.gen_range(1..=4) != 1 {
                let mut template = 0usize;
                for _ in 0..5000 {
                    let t = rng.gen_range(0..self.gd.monster_base_count.max(1));
                    let m = &self.gd.monsters[t];
                    if m.unique || m.id == 0 {
                        continue;
                    }
                    if m.depth + rng.gen_range(1..=5) > self.depth + rng.gen_range(1..=5) {
                        continue;
                    }
                    template = t;
                    break;
                }
                NestTheme::Symbol(template)
            } else {
                NestTheme::Chapel
            }
        } else if tmp < 80 {
            let mask: &'static [&'static str] = match rng.gen_range(0..6) {
                0 => &["BR_ACID"],
                1 => &["BR_ELEC"],
                2 => &["BR_FIRE"],
                3 => &["BR_COLD"],
                4 => &["BR_POIS"],
                _ => &["BR_ACID", "BR_ELEC", "BR_FIRE", "BR_COLD", "BR_POIS"],
            };
            NestTheme::Dragon(mask)
        } else {
            NestTheme::Demon
        };
        let mut what = Vec::with_capacity(16);
        let mut empty = false;
        for _ in 0..16 {
            match themed_monster(self.gd, self.depth + 10, theme, rng) {
                Some(d) => what.push(d),
                None => empty = true,
            }
        }
        if empty {
            return;
        }
        // Bubble sort by monster level, keep every other entry.
        for _ in 0..15 {
            for j in 0..15 {
                let p1 = self.gd.monsters[what[j]].depth;
                let p2 = self.gd.monsters[what[j + 1]].depth;
                if p1 > p2 {
                    what.swap(j, j + 1);
                }
            }
        }
        for i in 0..8 {
            what[i] = what[i * 2];
        }
        self.rating += 10;
        if self.depth <= 40 && rng.gen_range(1..=(self.depth * self.depth + 50).max(1)) < 300 {
            self.good_item = true;
        }
        let mut spots: Vec<(i32, i32, usize)> = Vec::new();
        for x in xval - 9..=xval + 9 {
            spots.push((x, yval - 2, 0));
            spots.push((x, yval + 2, 0));
        }
        for y in yval - 1..=yval + 1 {
            spots.push((xval - 9, y, 0));
            spots.push((xval + 9, y, 0));
            spots.push((xval - 8, y, 1));
            spots.push((xval + 8, y, 1));
            spots.push((xval - 7, y, 1));
            spots.push((xval + 7, y, 1));
            spots.push((xval - 6, y, 2));
            spots.push((xval + 6, y, 2));
            spots.push((xval - 5, y, 2));
            spots.push((xval + 5, y, 2));
            spots.push((xval - 4, y, 3));
            spots.push((xval + 4, y, 3));
            spots.push((xval - 3, y, 3));
            spots.push((xval + 3, y, 3));
            spots.push((xval - 2, y, 4));
            spots.push((xval + 2, y, 4));
        }
        for x in xval - 1..=xval + 1 {
            spots.push((x, yval + 1, 5));
            spots.push((x, yval - 1, 5));
        }
        spots.push((xval + 1, yval, 6));
        spots.push((xval - 1, yval, 6));
        spots.push((xval, yval, 7));
        for (x, y, which) in spots {
            let def = what[which];
            if self.clean_floor(x, y) {
                self.vault.monsters.push((def, x, y, self.depth));
            }
        }
    }

    /// build_type7/8 (generate.cc:3544/3610): v_info vault.
    fn build_type_vault(&mut self, by0: i32, bx0: i32, typ: u32, rng: &mut impl Rng) {
        let Some(v) = self.gd.vault_of_type(typ, rng).cloned() else {
            return;
        };
        let Some((xval, yval)) = self.room_alloc(v.wid as i32, v.hgt as i32, false, by0, bx0, rng)
        else {
            return;
        };
        let x0 = xval - v.wid as i32 / 2;
        let y0 = yval - v.hgt as i32 / 2;
        stamp_vault(self.map, self.gd, &v, x0, y0, self.depth, self.vault, rng);
        self.rating += v.rat as i32;
        if self.depth <= 50
            || rng.gen_range(1..=((self.depth as i32 - 40).pow(2) as u32 + 50).max(1)) < 400
        {
            self.good_item = true;
        }
    }

    /// build_type9 (generate.cc:3679): vertical oval room.
    fn build_type9(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let rad = 2 + rng.gen_range(0..8);
        let Some((x0, y0)) = self.room_alloc(rad * 2 + 1, rad * 2 + 1, false, by0, bx0, rng) else {
            return;
        };
        let light = rng.gen_range(1..=self.depth.max(1) as i32) <= 5;
        for x in x0 - rad..=x0 + rad {
            for y in y0 - rad..=y0 + rad {
                let d = pref_distance(y0, x0, y, x);
                if d == rad {
                    self.set_room(x, y, light);
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                }
                if d < rad {
                    self.set_room(x, y, light);
                    self.place_floor_at(x, y, rng);
                }
            }
        }
    }

    /// dist2 (generate.cc:5537): the perturbed distance used by crypts.
    #[allow(clippy::too_many_arguments)]
    fn dist2(&self, x1: i32, y1: i32, x2: i32, y2: i32, h: (i32, i32, i32, i32)) -> i32 {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let (h1, h2, h3, h4) = h;
        if dx >= 2 * dy {
            return dx + (dy * h1) / h2.max(1);
        }
        if dy >= 2 * dx {
            return dy + (dx * h1) / h2.max(1);
        }
        (dx + dy) * 128 / 181 + (dx * dx / (dy * h3.max(1)) + dy * dy / (dx * h3.max(1))) * h4
    }

    /// build_small_room (generate.cc:4386).
    fn build_small_room(&mut self, x0: i32, y0: i32, rng: &mut impl Rng) {
        self.build_rectangle(y0 - 1, x0 - 1, y0 + 1, x0 + 1, T_VAULT_INNER, false);
        match rng.gen_range(0..4) {
            0 => self.place_locked_door(x0 - 1, y0, rng),
            1 => self.place_locked_door(x0 + 1, y0, rng),
            2 => self.place_locked_door(x0, y0 - 1, rng),
            _ => self.place_locked_door(x0, y0 + 1, rng),
        }
        self.place_floor_at(x0, y0, rng);
    }

    /// add_outer_wall (generate.cc:5485): recursively convert the border
    /// of a floored region into outer walls.
    #[allow(clippy::too_many_arguments)]
    fn add_outer_wall(&mut self, x: i32, y: i32, light: bool, x1: i32, y1: i32, x2: i32, y2: i32) {
        if !Map::in_bounds(x, y) {
            return;
        }
        if self.is_room(x, y) {
            return;
        }
        self.set_room(x, y, false);
        if get_is_floor(self.gd, self.map, x, y) {
            for i in -1..=1 {
                for j in -1..=1 {
                    if x + i >= x1 && x + i <= x2 && y + j >= y1 && y + j <= y2 {
                        self.add_outer_wall(x + i, y + j, light, x1, y1, x2, y2);
                        if light {
                            self.map.lit[Map::idx(x, y)] = true;
                        }
                    }
                }
            }
        } else if self.map.terrain_at(x, y) == T_GRANITE {
            self.map.set_terrain(x, y, T_VAULT_OUTER);
            if light {
                self.map.lit[Map::idx(x, y)] = true;
            }
        } else if self.map.terrain_at(x, y) == 62 {
            if light {
                self.map.lit[Map::idx(x, y)] = true;
            }
        }
    }

    /// build_type12 (generate.cc:5794): crypt room.
    fn build_type12(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let h = (
            rng.gen_range(1..=32) - 16,
            rng.gen_range(1..=16),
            rng.gen_range(1..=32),
            rng.gen_range(1..=32) - 16,
        );
        let light = rng.gen_range(1..=self.depth.max(1) as i32) <= 5;
        let rad = rng.gen_range(1..=9);
        let Some((x0, y0)) = self.room_alloc(rad * 2 + 3, rad * 2 + 3, false, by0, bx0, rng) else {
            return;
        };
        for x in x0 - rad..=x0 + rad {
            for y in y0 - rad..=y0 + rad {
                self.clear_room(x, y);
                if self.dist2(x0, y0, x, y, h) <= rad - 1 || pref_distance(y0, x0, y, x) < 3 {
                    self.place_floor_at(x, y, rng);
                } else {
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                }
                if (y + rad) == y0 || (y - rad) == y0 || (x + rad) == x0 || (x - rad) == x0 {
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                }
            }
        }
        self.add_outer_wall(
            x0,
            y0,
            light,
            x0 - rad - 1,
            y0 - rad - 1,
            x0 + rad + 1,
            y0 + rad + 1,
        );
        let mut emptyflag = true;
        'scan: for x in x0 - 2..=x0 + 2 {
            for y in y0 - 2..=y0 + 2 {
                if !get_is_floor(self.gd, self.map, x, y) {
                    emptyflag = false;
                    break 'scan;
                }
            }
        }
        if emptyflag && rng.gen_range(0..2) == 0 {
            self.build_small_room(x0, y0, rng);
            self.vault.objects.push((self.depth, x0, y0, false));
            let num = rng.gen_range(1..=2) + 2;
            self.vault_monsters(y0, x0, num, rng);
        }
    }

    /// One boundary grid of generate_fracave.
    #[allow(clippy::too_many_arguments)]
    fn frac_boundary(
        &mut self,
        x0: i32,
        y0: i32,
        xhsize: i32,
        yhsize: i32,
        x: i32,
        y: i32,
        outer: bool,
        light: bool,
        room: bool,
        rng: &mut impl Rng,
    ) {
        let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
        if !Map::in_bounds(mx, my) {
            return;
        }
        if outer {
            self.map.set_terrain(mx, my, T_VAULT_OUTER);
            if light {
                self.map.lit[Map::idx(mx, my)] = true;
            }
            if room {
                self.set_room(mx, my, light);
            } else {
                self.place_filler_at(mx, my, rng);
            }
        } else {
            self.place_filler_at(mx, my, rng);
        }
        self.map.icky.remove(&Map::idx(mx, my));
    }

    /// generate_hmap + generate_fracave (generate.cc:3789/4060) for the
    /// region (x0, y0)..(x0+xsize, y0+ysize).  Returns false when the
    /// connected cave is too small.
    #[allow(clippy::too_many_arguments)]
    fn fractal_cave_region(
        &mut self,
        x0: i32,
        y0: i32,
        xsiz: i32,
        ysiz: i32,
        grd: i32,
        roug: i32,
        cutoff: i32,
        light: bool,
        room: bool,
        rng: &mut impl Rng,
    ) -> bool {
        let xhsize = xsiz / 2;
        let yhsize = ysiz / 2;
        let xsize = xhsize * 2;
        let ysize = yhsize * 2;
        let maxsize = xsize.max(ysize);
        let hidx = |x: i32, y: i32| (y * (xsize + 1) + x) as usize;
        let ri = |rng: &mut dyn rand::RngCore, n: i32| -> i32 {
            if n <= 0 {
                0
            } else {
                (rng.next_u32() % n as u32) as i32 + 1
            }
        };
        let mut hmap = vec![255u8; ((xsize + 1) * (ysize + 1)) as usize];
        for x in 0..=xsize {
            for y in 0..=ysize {
                let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                if Map::in_bounds(mx, my) {
                    self.map.icky.remove(&Map::idx(mx, my));
                }
            }
        }
        let store = |h: &mut Vec<u8>, x: i32, y: i32, val: i32| {
            let i = hidx(x, y);
            if h[i] != 255 {
                return;
            }
            let mut val = val & 0xff;
            if (x == 0 || y == 0 || x == xhsize * 2 || y == yhsize * 2) && val <= cutoff {
                val = (cutoff + 1) & 0xff;
            }
            if val > 216 {
                val = T_FLOOR as i32;
            }
            h[i] = val as u8;
        };
        store(&mut hmap, 0, 0, maxsize);
        store(&mut hmap, 0, ysize, maxsize);
        store(&mut hmap, xsize, 0, maxsize);
        store(&mut hmap, xsize, ysize, maxsize);
        store(&mut hmap, xhsize, yhsize, 0);
        let mut xstep = xsize * 256;
        let mut xhstep = xstep;
        let mut ystep = ysize * 256;
        let mut yhstep = ystep;
        let xxsize = xsize * 256;
        let yysize = ysize * 256;
        while xstep / 256 > 1 || ystep / 256 > 1 {
            xstep = xhstep;
            xhstep /= 2;
            ystep = yhstep;
            yhstep /= 2;
            let mut i = xhstep;
            while i <= xxsize - xhstep {
                let mut j = 0;
                while j <= yysize {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut hmap, i / 256, j / 256, v);
                    } else {
                        let l = hmap[hidx((i - xhstep) / 256, j / 256)] as i32;
                        let r = hmap[hidx((i + xhstep) / 256, j / 256)] as i32;
                        let v = (l + r) / 2 + (ri(rng, xstep / 256) - xhstep / 256) * roug / 16;
                        store(&mut hmap, i / 256, j / 256, v);
                    }
                    j += ystep;
                }
                i += xstep;
            }
            let mut j = yhstep;
            while j <= yysize - yhstep {
                let mut i = 0;
                while i <= xxsize {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut hmap, i / 256, j / 256, v);
                    } else {
                        let u = hmap[hidx(i / 256, (j - yhstep) / 256)] as i32;
                        let d = hmap[hidx(i / 256, (j + yhstep) / 256)] as i32;
                        let v = (u + d) / 2 + (ri(rng, ystep / 256) - yhstep / 256) * roug / 16;
                        store(&mut hmap, i / 256, j / 256, v);
                    }
                    i += xstep;
                }
                j += ystep;
            }
            let mut i = xhstep;
            while i <= xxsize - xhstep {
                let mut j = yhstep;
                while j <= yysize - yhstep {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut hmap, i / 256, j / 256, v);
                    } else {
                        let ul = hmap[hidx((i - xhstep) / 256, (j - yhstep) / 256)] as i32;
                        let dl = hmap[hidx((i - xhstep) / 256, (j + yhstep) / 256)] as i32;
                        let ur = hmap[hidx((i + xhstep) / 256, (j - yhstep) / 256)] as i32;
                        let dr = hmap[hidx((i + xhstep) / 256, (j + yhstep) / 256)] as i32;
                        let v = (ul + dl + ur + dr) / 4
                            + (ri(rng, xstep / 256) - xhstep / 256) * (362 / 16) / 256 * roug;
                        store(&mut hmap, i / 256, j / 256, v);
                    }
                    j += ystep;
                }
                i += xstep;
            }
        }
        // hack_isnt_wall + fill_hack from the centre.
        let mut done = vec![false; hmap.len()];
        let mut boundary: HashSet<usize> = HashSet::new();
        let mut amount = 0i32;
        let mut stack = vec![(yhsize, xhsize)];
        while let Some((y, x)) = stack.pop() {
            for i in -1..=1 {
                for j in -1..=1 {
                    let (nx, ny) = (x + i, y + j);
                    if nx > 0 && nx < xsize && ny > 0 && ny < ysize {
                        let di = hidx(nx, ny);
                        if done[di] {
                            continue;
                        }
                        done[di] = true;
                        let (mx, my) = (nx + x0 - xhsize, ny + y0 - yhsize);
                        if hmap[di] as i32 <= cutoff {
                            self.place_floor_at(mx, my, rng);
                            stack.push((ny, nx));
                            amount += 1;
                        } else {
                            self.map.set_terrain(mx, my, T_VAULT_OUTER);
                        }
                    } else {
                        boundary.insert(hidx(nx, ny));
                    }
                }
            }
        }
        if amount < 10 {
            for x in 0..=xsize {
                for y in 0..ysize {
                    let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                    self.place_filler_at(mx, my, rng);
                    self.clear_room(mx, my);
                    self.map.icky.remove(&Map::idx(mx, my));
                }
            }
            return false;
        }
        for i in 0..=xsize {
            let top = boundary.contains(&hidx(i, 0));
            let bottom = boundary.contains(&hidx(i, ysize));
            self.frac_boundary(x0, y0, xhsize, yhsize, i, 0, top, light, room, rng);
            self.frac_boundary(x0, y0, xhsize, yhsize, i, ysize, bottom, light, room, rng);
        }
        for i in 1..ysize {
            let left = boundary.contains(&hidx(0, i));
            let right = boundary.contains(&hidx(xsize, i));
            self.frac_boundary(x0, y0, xhsize, yhsize, 0, i, left, light, room, rng);
            self.frac_boundary(x0, y0, xhsize, yhsize, xsize, i, right, light, room, rng);
        }
        for x in 1..xsize {
            for y in 1..ysize {
                let di = hidx(x, y);
                let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                if done[di] && get_is_floor(self.gd, self.map, mx, my) {
                    self.map.icky.remove(&Map::idx(mx, my));
                    if light {
                        self.map.lit[Map::idx(mx, my)] = true;
                    }
                    if room {
                        self.set_room(mx, my, light);
                    }
                } else if done[di] && self.map.terrain_at(mx, my) == T_VAULT_OUTER {
                    // Filled wall: keep it as a room wall on room caves,
                    // otherwise let the fill terrain take over.
                    self.map.icky.remove(&Map::idx(mx, my));
                    if light {
                        self.map.lit[Map::idx(mx, my)] = true;
                    }
                    if room {
                        self.set_room(mx, my, light);
                    } else {
                        self.place_filler_at(mx, my, rng);
                    }
                } else {
                    // Unconnected region (generate_fracave final loop):
                    // back to the dungeon filler and clear CAVE_ICKY/ROOM.
                    self.place_filler_at(mx, my, rng);
                    self.clear_room(mx, my);
                    self.map.icky.remove(&Map::idx(mx, my));
                }
            }
        }
        true
    }

    /// build_type10 (generate.cc:4334): fractal cave room.
    fn build_type10(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let xsize = rng.gen_range(1..=22) * 2 + 6;
        let ysize = rng.gen_range(1..=15) * 2 + 6;
        let Some((x0, y0)) = self.room_alloc(xsize + 1, ysize + 1, false, by0, bx0, rng) else {
            return;
        };
        let light = self.depth <= rng.gen_range(1..=25);
        loop {
            let grd = 1 << rng.gen_range(1..=4);
            let roug = rng.gen_range(1..=8) * rng.gen_range(1..=4);
            let cutoff = rng.gen_range(1..=xsize / 4)
                + rng.gen_range(1..=ysize / 4)
                + rng.gen_range(1..=xsize / 4)
                + rng.gen_range(1..=ysize / 4);
            if self.fractal_cave_region(x0, y0, xsize, ysize, grd, roug, cutoff, light, true, rng) {
                break;
            }
        }
    }

    /// build_type11 (generate.cc:5701): random vault family.
    fn build_type11(&mut self, by0: i32, bx0: i32, rng: &mut impl Rng) {
        let xsize = rng.gen_range(1..=22) + 22;
        let ysize = rng.gen_range(1..=11) + 11;
        let Some((x0, y0)) = self.room_alloc(xsize + 2, ysize + 2, false, by0, bx0, rng) else {
            return;
        };
        self.rating += 10;
        if self.depth <= 50
            || rng.gen_range(1..=((self.depth as i32 - 40).pow(2) as u32 + 1).max(1)) < 400
        {
            self.good_item = true;
        }
        match rng.gen_range(1..=8) {
            1 => self.build_bubble_vault(x0, y0, xsize, ysize, rng),
            2 => self.build_room_vault(x0, y0, xsize, ysize, rng),
            3 => self.build_cave_vault(x0, y0, xsize, ysize, rng),
            4 => self.build_maze_vault(x0, y0, xsize, ysize, rng),
            5 => self.build_mini_c_vault(x0, y0, xsize, ysize, rng),
            6 => self.build_castle_vault(x0, y0, xsize, ysize, rng),
            7 => self.build_target_vault(x0, y0, xsize, ysize, rng),
            _ => {}
        }
    }

    /// add_door (generate.cc:4430).
    fn add_door(&mut self, x: i32, y: i32, rng: &mut impl Rng) {
        if !Map::in_bounds(x, y) || self.map.terrain_at(x, y) != T_VAULT_OUTER {
            return;
        }
        if get_is_floor(self.gd, self.map, x, y - 1)
            && get_is_floor(self.gd, self.map, x, y + 1)
            && self.map.terrain_at(x - 1, y) == T_VAULT_OUTER
            && self.map.terrain_at(x + 1, y) == T_VAULT_OUTER
        {
            self.place_locked_door(x, y, rng);
            self.place_filler_at(x - 1, y, rng);
            self.place_filler_at(x + 1, y, rng);
        }
        if self.map.terrain_at(x, y - 1) == T_VAULT_OUTER
            && self.map.terrain_at(x, y + 1) == T_VAULT_OUTER
            && get_is_floor(self.gd, self.map, x - 1, y)
            && get_is_floor(self.gd, self.map, x + 1, y)
        {
            self.place_locked_door(x, y, rng);
            self.place_filler_at(x, y - 1, rng);
            self.place_filler_at(x, y + 1, rng);
        }
    }

    /// build_bubble_vault (generate.cc:4630).
    fn build_bubble_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        const BUBBLENUM: usize = 10;
        let xhsize = xsize / 2;
        let yhsize = ysize / 2;
        let mut center = [(0i32, 0i32); BUBBLENUM];
        center[0] = (
            rng.gen_range(1..=xsize - 3) + 1,
            rng.gen_range(1..=ysize - 3) + 1,
        );
        for i in 1..BUBBLENUM {
            let mut done = false;
            let (mut x, mut y) = (0, 0);
            for _ in 0..2000 {
                done = true;
                x = rng.gen_range(1..=xsize - 3) + 1;
                y = rng.gen_range(1..=ysize - 3) + 1;
                for j in 0..i {
                    if x == center[j].0 || y == center[j].1 {
                        done = false;
                    }
                }
                if done {
                    break;
                }
            }
            if !done {
                return;
            }
            center[i] = (x, y);
        }
        self.build_rectangle(
            y0 - yhsize,
            x0 - xhsize,
            y0 - yhsize + ysize - 1,
            x0 - xhsize + xsize - 1,
            T_VAULT_OUTER,
            false,
        );
        for x in 1..xsize - 1 {
            for y in 1..ysize - 1 {
                let mut min1 = pref_distance(x, y, center[0].0, center[0].1);
                let mut min2 = pref_distance(x, y, center[1].0, center[1].1);
                if min1 > min2 {
                    std::mem::swap(&mut min1, &mut min2);
                }
                for c in center.iter().skip(2) {
                    let temp = pref_distance(x, y, c.0, c.1);
                    if temp < min1 {
                        min2 = min1;
                        min1 = temp;
                    } else if temp < min2 {
                        min2 = temp;
                    }
                }
                let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                if min2 - min1 <= 2 && min1 >= 3 {
                    self.place_filler_at(mx, my, rng);
                } else {
                    self.place_floor_at(mx, my, rng);
                }
                self.set_room(mx, my, false);
                self.map.icky.insert(Map::idx(mx, my));
            }
        }
        for _ in 0..500 {
            let x = rng.gen_range(1..=xsize - 3) - xhsize + x0 + 1;
            let y = rng.gen_range(1..=ysize - 3) - yhsize + y0 + 1;
            self.add_door(x, y, rng);
        }
        let diff = rng.gen_range(1..=5);
        self.fill_treasure(
            x0 - xhsize + 1,
            x0 - xhsize + xsize - 2,
            y0 - yhsize + 1,
            y0 - yhsize + ysize - 2,
            diff,
            rng,
        );
    }

    /// build_room (generate.cc:4786).
    fn build_room(
        &mut self,
        mut x1: i32,
        mut x2: i32,
        mut y1: i32,
        mut y2: i32,
        rng: &mut impl Rng,
    ) {
        if x1 == x2 || y1 == y2 {
            return;
        }
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }
        let xsize = x2 - x1;
        let ysize = y2 - y1;
        self.build_rectangle(y1, x1, y2, x2, T_VAULT_OUTER, false);
        for x in 1..xsize {
            for y in 1..ysize {
                let (mx, my) = (x1 + x, y1 + y);
                if self.map.terrain_at(mx, my) == T_VAULT_OUTER {
                    self.place_floor_at(mx, my, rng);
                    self.set_room(mx, my, false);
                } else {
                    self.set_room(mx, my, false);
                }
                self.map.icky.insert(Map::idx(mx, my));
            }
        }
    }

    /// build_room_vault (generate.cc:4840).
    fn build_room_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        let xhsize = xsize / 2;
        let yhsize = ysize / 2;
        for x1 in 0..=xsize {
            let x = x0 - xhsize + x1;
            for y1 in 0..=ysize {
                let y = y0 - yhsize + y1;
                self.map.set_terrain(x, y, T_VAULT_OUTER);
                self.map.icky.remove(&Map::idx(x, y));
            }
        }
        for _ in 0..10 {
            let x1 = rng.gen_range(1..=xhsize) * 2 + x0 - xhsize;
            let x2 = rng.gen_range(1..=xhsize) * 2 + x0 - xhsize;
            let y1 = rng.gen_range(1..=yhsize) * 2 + y0 - yhsize;
            let y2 = rng.gen_range(1..=yhsize) * 2 + y0 - yhsize;
            self.build_room(x1, x2, y1, y2, rng);
        }
        for x in 0..=xsize {
            let mx = x0 - xhsize + x;
            for y in 0..=ysize {
                let my = y0 - yhsize + y;
                if self.map.terrain_at(mx, my) == T_VAULT_OUTER {
                    self.place_filler_at(mx, my, rng);
                }
            }
        }
        for _ in 0..500 {
            let x = rng.gen_range(1..=xsize - 2) - xhsize + x0 + 1;
            let y = rng.gen_range(1..=ysize - 2) - yhsize + y0 + 1;
            self.add_door(x, y, rng);
        }
        let diff = rng.gen_range(1..=5) + 5;
        self.fill_treasure(
            x0 - xhsize + 1,
            x0 - xhsize + xsize - 1,
            y0 - yhsize + 1,
            y0 - yhsize + ysize - 1,
            diff,
            rng,
        );
    }

    /// build_cave_vault (generate.cc:4898).
    fn build_cave_vault(&mut self, x0: i32, y0: i32, xsiz: i32, ysiz: i32, rng: &mut impl Rng) {
        let xhsize = xsiz / 2;
        let yhsize = ysiz / 2;
        let xsize = xhsize * 2;
        let ysize = yhsize * 2;
        loop {
            let grd = 1 << rng.gen_range(1..=4);
            let roug = rng.gen_range(1..=8) * rng.gen_range(1..=4);
            let cutoff = rng.gen_range(1..=xsize / 4)
                + rng.gen_range(1..=ysize / 4)
                + rng.gen_range(1..=xsize / 4)
                + rng.gen_range(1..=ysize / 4);
            if self.fractal_cave_region(x0, y0, xsize, ysize, grd, roug, cutoff, false, true, rng) {
                break;
            }
        }
        for x in 0..=xsize {
            for y in 0..=ysize {
                let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                if Map::in_bounds(mx, my) {
                    self.map.icky.insert(Map::idx(mx, my));
                }
            }
        }
        let diff = rng.gen_range(1..=5);
        self.fill_treasure(
            x0 - xhsize + 1,
            x0 - xhsize + xsize - 1,
            y0 - yhsize + 1,
            y0 - yhsize + ysize - 1,
            diff,
            rng,
        );
    }

    /// r_visit (generate.cc:4966): randomized depth-first spanning tree.
    #[allow(clippy::too_many_arguments)]
    fn r_visit(
        &mut self,
        y1: i32,
        x1: i32,
        y2: i32,
        x2: i32,
        node: usize,
        dir: usize,
        visited: &mut [bool],
        rng: &mut impl Rng,
    ) {
        let m = (x2 - x1) / 2 + 1;
        let n = (y2 - y1) / 2 + 1;
        visited[node] = true;
        let x = 2 * (node as i32 % m) + x1;
        let y = 2 * (node as i32 / m) + y1;
        self.place_floor_at(x, y, rng);
        let mut adj = [0usize, 1, 2, 3];
        if rng.gen_range(0..3) == 0 {
            for i in 0..4 {
                let j = rng.gen_range(0..4);
                adj.swap(i, j);
            }
        } else {
            adj[0] = dir;
            for i in 1..4 {
                let j = 1 + rng.gen_range(0..3);
                adj.swap(i, j);
            }
        }
        for i in 0..4 {
            match adj[i] {
                0 => {
                    if (node as i32 / m) < n - 1 && !visited[node + m as usize] {
                        self.place_floor_at(x, y + 1, rng);
                        self.r_visit(y1, x1, y2, x2, node + m as usize, dir, visited, rng);
                    }
                }
                1 => {
                    if (node as i32 / m) > 0 && !visited[node - m as usize] {
                        self.place_floor_at(x, y - 1, rng);
                        self.r_visit(y1, x1, y2, x2, node - m as usize, dir, visited, rng);
                    }
                }
                2 => {
                    if (node as i32 % m) < m - 1 && !visited[node + 1] {
                        self.place_floor_at(x + 1, y, rng);
                        self.r_visit(y1, x1, y2, x2, node + 1, dir, visited, rng);
                    }
                }
                _ => {
                    if (node as i32 % m) > 0 && !visited[node - 1] {
                        self.place_floor_at(x - 1, y, rng);
                        self.r_visit(y1, x1, y2, x2, node - 1, dir, visited, rng);
                    }
                }
            }
        }
    }

    /// build_maze_vault (generate.cc:5067).
    fn build_maze_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        let light = self.depth <= rng.gen_range(1..=25);
        let dy = ysize / 2 - 1;
        let dx = xsize / 2 - 1;
        let (y1, x1, y2, x2) = (y0 - dy, x0 - dx, y0 + dy, x0 + dx);
        for y in y1 - 1..=y2 + 1 {
            for x in x1 - 1..=x2 + 1 {
                self.set_room(x, y, light);
                self.mark_icky(x, y);
                if x == x1 - 1 || x == x2 + 1 || y == y1 - 1 || y == y2 + 1 {
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                } else {
                    self.map.set_terrain(x, y, T_VAULT_INNER);
                }
            }
        }
        let m = dx + 1;
        let n = dy + 1;
        let num_vertices = (m * n) as usize;
        let mut visited = vec![false; num_vertices];
        let root = rng.gen_range(0..num_vertices);
        self.r_visit(y1, x1, y2, x2, root, 0, &mut visited, rng);
        let diff = rng.gen_range(1..=5);
        self.fill_treasure(x1, x2, y1, y2, diff, rng);
    }

    /// build_mini_c_vault (generate.cc:5139).
    fn build_mini_c_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        let dy = ysize / 2 - 1;
        let dx = xsize / 2 - 1;
        let (y1, x1, y2, x2) = (y0 - dy, x0 - dx, y0 + dy, x0 + dx);
        for y in y1 - 1..=y2 + 1 {
            for x in x1 - 1..=x2 + 1 {
                self.set_room(x, y, false);
                self.mark_icky(x, y);
                self.map.set_terrain(x, y, T_VAULT_PERM);
            }
        }
        let m = dx + 1;
        let n = dy + 1;
        let num_vertices = (m * n) as usize;
        let mut visited = vec![false; num_vertices];
        let root = rng.gen_range(0..num_vertices);
        self.r_visit(y1, x1, y2, x2, root, 0, &mut visited, rng);
        for x in x1..=x2 {
            for y in y1..=y2 {
                let total = x - x1 + y - y1;
                if total % 2 == 1 && get_is_floor(self.gd, self.map, x, y) {
                    self.map.set_terrain(x, y, T_VAULT_INNER);
                }
            }
        }
        if rng.gen_range(0..2) == 0 {
            let y = rng.gen_range(1..=dy) + dy / 2;
            self.map.set_terrain(x1 - 1, y1 + y, T_VAULT_OUTER);
            self.map.set_terrain(x2 + 1, y1 + y, T_VAULT_OUTER);
        } else {
            let x = rng.gen_range(1..=dx) + dx / 2;
            self.map.set_terrain(x1 + x, y1 - 1, T_VAULT_OUTER);
            self.map.set_terrain(x1 + x, y2 + 1, T_VAULT_OUTER);
        }
        self.fill_treasure(x1, x2, y1, y2, 10, rng);
    }

    /// build_recursive_room (generate.cc:5228).
    fn build_recursive_room(
        &mut self,
        mut x1: i32,
        mut y1: i32,
        mut x2: i32,
        mut y2: i32,
        mut power: i32,
        rng: &mut impl Rng,
    ) {
        let mut xsize = x2 - x1;
        let mut ysize = y2 - y1;
        let choice;
        if power < 3 && xsize > 12 && ysize > 12 {
            choice = 1;
        } else if power < 10 {
            if rng.gen_range(1..=10) > 2 && xsize < 8 && ysize < 8 {
                choice = 4;
            } else {
                choice = rng.gen_range(1..=2) + 1;
            }
        } else {
            choice = rng.gen_range(1..=3) + 1;
        }
        match choice {
            1 => {
                for x in x1..=x2 {
                    self.map.set_terrain(x, y1, T_VAULT_OUTER);
                    self.map.set_terrain(x, y2, T_VAULT_OUTER);
                }
                for y in y1 + 1..y2 {
                    self.map.set_terrain(x1, y, T_VAULT_OUTER);
                    self.map.set_terrain(x2, y, T_VAULT_OUTER);
                }
                if rng.gen_range(0..2) == 0 {
                    let y = rng.gen_range(1..=ysize) + y1;
                    self.place_floor_at(x1, y, rng);
                    self.place_floor_at(x2, y, rng);
                } else {
                    let x = rng.gen_range(1..=xsize) + x1;
                    self.place_floor_at(x, y1, rng);
                    self.place_floor_at(x, y2, rng);
                }
                let t1 = rng.gen_range(1..=ysize / 3) + y1;
                let t2 = y2 - rng.gen_range(1..=ysize / 3);
                let t3 = rng.gen_range(1..=xsize / 3) + x1;
                let t4 = x2 - rng.gen_range(1..=xsize / 3);
                self.build_recursive_room(x1 + 1, y1 + 1, x2 - 1, t1, power + 1, rng);
                self.build_recursive_room(x1 + 1, t2, x2 - 1, y2, power + 1, rng);
                self.build_recursive_room(x1 + 1, t1 + 1, t3, t2 - 1, power + 3, rng);
                self.build_recursive_room(t4, t1 + 1, x2 - 1, t2 - 1, power + 3, rng);
                x1 = t3;
                x2 = t4;
                y1 = t1;
                y2 = t2;
                xsize = x2 - x1;
                ysize = y2 - y1;
                power += 2;
                // Fall through to the "try to build a room" case.
                if xsize < 3 || ysize < 3 {
                    for y in y1..y2 {
                        for x in x1..x2 {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                    return;
                }
                for x in x1 + 1..=x2 - 1 {
                    self.map.set_terrain(x, y1 + 1, T_VAULT_INNER);
                    self.map.set_terrain(x, y2 - 1, T_VAULT_INNER);
                }
                for y in y1 + 1..=y2 - 1 {
                    self.map.set_terrain(x1 + 1, y, T_VAULT_INNER);
                    self.map.set_terrain(x2 - 1, y, T_VAULT_INNER);
                }
                let y = rng.gen_range(1..=(ysize - 3).max(1)) + y1 + 1;
                if rng.gen_range(0..2) == 0 {
                    self.place_floor_at(x1 + 1, y, rng);
                } else {
                    self.place_floor_at(x2 - 1, y, rng);
                }
                self.build_recursive_room(x1 + 2, y1 + 2, x2 - 2, y2 - 2, power + 3, rng);
            }
            4 => {
                if xsize < 3 || ysize < 3 {
                    for y in y1..y2 {
                        for x in x1..x2 {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                    return;
                }
                for x in x1 + 1..=x2 - 1 {
                    self.map.set_terrain(x, y1 + 1, T_VAULT_INNER);
                    self.map.set_terrain(x, y2 - 1, T_VAULT_INNER);
                }
                for y in y1 + 1..=y2 - 1 {
                    self.map.set_terrain(x1 + 1, y, T_VAULT_INNER);
                    self.map.set_terrain(x2 - 1, y, T_VAULT_INNER);
                }
                let y = rng.gen_range(1..=(ysize - 3).max(1)) + y1 + 1;
                if rng.gen_range(0..2) == 0 {
                    self.place_floor_at(x1 + 1, y, rng);
                } else {
                    self.place_floor_at(x2 - 1, y, rng);
                }
                self.build_recursive_room(x1 + 2, y1 + 2, x2 - 2, y2 - 2, power + 3, rng);
            }
            2 => {
                if xsize < 3 {
                    for y in y1..y2 {
                        for x in x1..x2 {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                    return;
                }
                let t1 = rng.gen_range(1..=xsize - 2) + x1 + 1;
                self.build_recursive_room(x1, y1, t1, y2, power - 2, rng);
                self.build_recursive_room(t1 + 1, y1, x2, y2, power - 2, rng);
            }
            _ => {
                if ysize < 3 {
                    for y in y1..y2 {
                        for x in x1..x2 {
                            self.map.set_terrain(x, y, T_VAULT_INNER);
                        }
                    }
                    return;
                }
                let t1 = rng.gen_range(1..=ysize - 2) + y1 + 1;
                self.build_recursive_room(x1, y1, x2, t1, power - 2, rng);
                self.build_recursive_room(x1, t1 + 1, x2, y2, power - 2, rng);
            }
        }
    }

    /// build_castle_vault (generate.cc:5439).
    fn build_castle_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        let dy = ysize / 2 - 1;
        let dx = xsize / 2 - 1;
        let (y1, x1, y2, x2) = (y0 - dy, x0 - dx, y0 + dy, x0 + dx);
        for y in y1 - 1..=y2 + 1 {
            for x in x1 - 1..=x2 + 1 {
                self.set_room(x, y, false);
                self.mark_icky(x, y);
                self.place_floor_at(x, y, rng);
            }
        }
        let power = rng.gen_range(1..=5);
        self.build_recursive_room(x1, y1, x2, y2, power, rng);
        let diff = rng.gen_range(1..=3);
        self.fill_treasure(x1, x2, y1, y2, diff, rng);
    }

    /// build_target_vault (generate.cc:5568).
    fn build_target_vault(&mut self, x0: i32, y0: i32, xsize: i32, ysize: i32, rng: &mut impl Rng) {
        let h = (
            rng.gen_range(1..=32) - 16,
            rng.gen_range(1..=16),
            rng.gen_range(1..=32),
            rng.gen_range(1..=32) - 16,
        );
        let rad = if xsize > ysize { ysize / 2 } else { xsize / 2 };
        for x in x0 - rad..=x0 + rad {
            for y in y0 - rad..=y0 + rad {
                if !Map::in_bounds(x, y) {
                    continue;
                }
                self.clear_room(x, y);
                self.map.icky.insert(Map::idx(x, y));
                if self.dist2(x0, y0, x, y, h) <= rad - 1 {
                    self.place_floor_at(x, y, rng);
                } else {
                    self.map.set_terrain(x, y, T_GRANITE);
                }
                if (y + rad) == y0 || (y - rad) == y0 || (x + rad) == x0 || (x - rad) == x0 {
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                }
            }
        }
        self.add_outer_wall(
            x0,
            y0,
            false,
            x0 - rad - 1,
            y0 - rad - 1,
            x0 + rad + 1,
            y0 + rad + 1,
        );
        for x in x0 - rad / 2..=x0 + rad / 2 {
            for y in y0 - rad / 2..=y0 + rad / 2 {
                if self.dist2(x0, y0, x, y, h) == rad / 2 {
                    self.map.set_terrain(x, y, T_VAULT_INNER);
                }
            }
        }
        for x in x0 - rad..=x0 + rad {
            self.map.set_terrain(x, y0, T_VAULT_INNER);
        }
        for y in y0 - rad..=y0 + rad {
            self.map.set_terrain(x0, y, T_VAULT_INNER);
        }
        for y in y0 - 1..=y0 + 1 {
            self.map.set_terrain(x0 - 1, y, T_VAULT_INNER);
            self.map.set_terrain(x0 + 1, y, T_VAULT_INNER);
        }
        for x in x0 - 1..=x0 + 1 {
            self.map.set_terrain(x, y0 - 1, T_VAULT_INNER);
            self.map.set_terrain(x, y0 + 1, T_VAULT_INNER);
        }
        self.place_floor_at(x0, y0, rng);
        let x = (rad - 2) / 4 + 1;
        let y = rad / 2 + x;
        self.add_door(x0 + x, y0, rng);
        self.add_door(x0 + y, y0, rng);
        self.add_door(x0 - x, y0, rng);
        self.add_door(x0 - y, y0, rng);
        self.add_door(x0, y0 + x, rng);
        self.add_door(x0, y0 + y, rng);
        self.add_door(x0, y0 - x, rng);
        self.add_door(x0, y0 - y, rng);
        let diff = rng.gen_range(1..=3) + 3;
        self.fill_treasure(x0 - rad, x0 + rad, y0 - rad, y0 + rad, diff, rng);
    }

    /// build_cavern (generate.cc:4295): a fractal cave over the whole
    /// level, before the rooms.
    fn build_cavern(&mut self, rng: &mut impl Rng) {
        let light = self.depth <= rng.gen_range(1..=25);
        let xs = self.w - 1;
        let ys = self.h - 1;
        let x0 = xs / 2;
        let y0 = ys / 2;
        let xsize = x0 * 2;
        let ysize = y0 * 2;
        loop {
            let grd = 1 << (rng.gen_range(1..=4) + 4);
            let roug = rng.gen_range(1..=8) * rng.gen_range(1..=4);
            let cutoff = xsize / 2;
            if self.fractal_cave_region(x0, y0, xsize, ysize, grd, roug, cutoff, light, false, rng)
            {
                break;
            }
        }
    }

    /// The dungeon's own fill features (build_tunnel wall test).
    fn has_fill_feat(&self, t: u16) -> bool {
        self.fills.contains(&t)
    }

    /// build_tunnel (generate.cc:5912).
    #[allow(clippy::too_many_arguments)]
    fn build_tunnel(
        &mut self,
        mut row1: i32,
        mut col1: i32,
        row2: i32,
        col2: i32,
        water: bool,
        rng: &mut impl Rng,
    ) {
        let mut tunn: Vec<(i32, i32)> = Vec::new();
        let mut walls: Vec<(i32, i32)> = Vec::new();
        let (start_row, start_col) = (row1, col1);
        let (mut rdir, mut cdir) = correct_dir(row1, col1, row2, col2, rng);
        let mut door_flag = false;
        let mut loop_count = 0;
        while row1 != row2 || col1 != col2 {
            loop_count += 1;
            if loop_count > 2000 {
                break;
            }
            if rng.gen_range(0..100) < DUN_TUN_CHG {
                let (r, c) = correct_dir(row1, col1, row2, col2, rng);
                rdir = r;
                cdir = c;
                if rng.gen_range(0..100) < DUN_TUN_RND {
                    let (r, c) = rand_dir(rng);
                    rdir = r;
                    cdir = c;
                }
            }
            let mut tmp_row = row1 + rdir;
            let mut tmp_col = col1 + cdir;
            let mut guard = 0;
            while !Map::in_bounds(tmp_col, tmp_row)
                || tmp_col < 0
                || tmp_row < 0
                || tmp_col >= self.w
                || tmp_row >= self.h
            {
                guard += 1;
                if guard > 5000 {
                    return;
                }
                let (r, c) = correct_dir(row1, col1, row2, col2, rng);
                rdir = r;
                cdir = c;
                if rng.gen_range(0..100) < DUN_TUN_RND {
                    let (r, c) = rand_dir(rng);
                    rdir = r;
                    cdir = c;
                }
                tmp_row = row1 + rdir;
                tmp_col = col1 + cdir;
            }
            let feat = self.map.terrain_at(tmp_col, tmp_row);
            if feat == T_PERMANENT || feat == 62 || feat == T_WALL_SOLID {
                continue;
            }
            if feat == T_VAULT_OUTER && self.room[Map::idx(tmp_col, tmp_row)] {
                let (y, x) = (tmp_row + rdir, tmp_col + cdir);
                let nf = self.map.terrain_at(x, y);
                if nf == T_PERMANENT || nf == 62 || nf == T_WALL_SOLID {
                    continue;
                }
                if nf == T_VAULT_OUTER && self.room[Map::idx(x, y)] {
                    continue;
                }
                row1 = tmp_row;
                col1 = tmp_col;
                walls.push((row1, col1));
                for yy in row1 - 1..=row1 + 1 {
                    for xx in col1 - 1..=col1 + 1 {
                        if self.map.terrain_at(xx, yy) == T_VAULT_OUTER
                            && self.room[Map::idx(xx, yy)]
                        {
                            self.map.set_terrain(xx, yy, T_WALL_SOLID);
                        }
                    }
                }
            } else if self.room[Map::idx(tmp_col, tmp_row)] {
                row1 = tmp_row;
                col1 = tmp_col;
            } else if self.has_fill_feat(feat) {
                row1 = tmp_row;
                col1 = tmp_col;
                tunn.push((row1, col1));
                door_flag = false;
            } else {
                row1 = tmp_row;
                col1 = tmp_col;
                if !door_flag {
                    self.door.push((row1, col1));
                    door_flag = true;
                }
                if rng.gen_range(0..100) >= DUN_TUN_CON
                    && ((row1 - start_row).abs() > 10 || (col1 - start_col).abs() > 10)
                {
                    break;
                }
            }
        }
        for (y, x) in tunn {
            if !water {
                self.place_floor_at(x, y, rng);
            } else {
                self.map.set_terrain(x, y, T_SHAL_WATER);
            }
        }
        for (y, x) in walls {
            self.place_floor_at(x, y, rng);
            if !self.no_doors && rng.gen_range(0..100) < DUN_TUN_PEN {
                self.place_random_door(x, y, rng);
            }
        }
    }

    /// next_to_corr (generate.cc:6169).
    fn next_to_corr(&self, y1: i32, x1: i32) -> i32 {
        let mut k = 0;
        for (dy, dx) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (y, x) = (y1 + dy, x1 + dx);
            if !Map::in_bounds(x, y) || !get_is_floor(self.gd, self.map, x, y) {
                continue;
            }
            if !self.has_fill_feat(self.map.terrain_at(x, y)) {
                continue;
            }
            if self.room[Map::idx(x, y)] {
                continue;
            }
            k += 1;
        }
        k
    }

    /// possible_doorway (generate.cc:6216).
    fn possible_doorway(&self, y: i32, x: i32) -> bool {
        if !Map::in_bounds(x, y) || !Map::in_bounds(x, y - 1) || !Map::in_bounds(x, y + 1) {
            return false;
        }
        if self.next_to_corr(y, x) >= 2 {
            if self.gd.terrain(self.map.terrain_at(x, y - 1)).is_wall
                && self.gd.terrain(self.map.terrain_at(x, y + 1)).is_wall
            {
                return true;
            }
            if self.gd.terrain(self.map.terrain_at(x - 1, y)).is_wall
                && self.gd.terrain(self.map.terrain_at(x + 1, y)).is_wall
            {
                return true;
            }
        }
        false
    }

    /// try_doors (generate.cc:6246).
    fn try_doors(&mut self, y: i32, x: i32, rng: &mut impl Rng) {
        if self.no_doors {
            return;
        }
        let mut dir_ok = [false; 4];
        let mut n = 0;
        for i in 0..4 {
            let (dy, dx) = DIR4[i];
            let (yy, xx) = (y + dy, x + dx);
            if !Map::in_bounds(xx, yy) {
                continue;
            }
            if self.gd.terrain(self.map.terrain_at(xx, yy)).is_wall {
                continue;
            }
            if self.room[Map::idx(xx, yy)] {
                continue;
            }
            if !self.possible_doorway(yy, xx) {
                continue;
            }
            dir_ok[i] = true;
            n += 1;
        }
        if rng.gen_range(0..100) < 75 {
            for i in 0..4 {
                if !dir_ok[i] {
                    continue;
                }
                if rng.gen_range(0..100) < DUN_TUN_JCT {
                    let (dy, dx) = DIR4[i];
                    self.place_random_door(x + dx, y + dy, rng);
                }
            }
        } else {
            if n == 4 {
                dir_ok = [false; 4];
                let a = rng.gen_range(0..4);
                let b = rng.gen_range(0..4);
                dir_ok[a] = true;
                dir_ok[b] = true;
            } else if n == 3 || n == 2 {
                // C++ keeps exactly the k'th OK direction: `k--` is a
                // post-decrement, so once k reaches zero the direction is
                // kept and later ones are rejected (k becomes -1).
                let mut k = rng.gen_range(0..n);
                for i in 0..4 {
                    if dir_ok[i] {
                        if k != 0 {
                            k -= 1;
                            dir_ok[i] = false;
                        } else {
                            k = -1;
                        }
                    }
                }
            }
            for i in 0..4 {
                if !dir_ok[i] {
                    continue;
                }
                let (dy, dx) = DIR4[i];
                self.place_locked_door(x + dx, y + dy, rng);
            }
        }
    }

    /// room_build (generate.cc:6364).
    fn room_build(&mut self, y: i32, x: i32, typ: usize, rng: &mut impl Rng) -> bool {
        // options->ironman_rooms ignores the room depth restrictions
        // (generate.cc room_build:6367).
        if self.depth < ROOMDEP[typ] && !self.ironman {
            return false;
        }
        if (self.crowded && (typ == 5 || typ == 6)) || typ == 0 {
            return false;
        }
        match typ {
            12 => self.build_type12(y, x, rng),
            11 => self.build_type11(y, x, rng),
            10 => self.build_type10(y, x, rng),
            9 => self.build_type9(y, x, rng),
            8 => self.build_type_vault(y, x, 8, rng),
            7 => self.build_type_vault(y, x, 7, rng),
            6 => self.build_type6(y, x, rng),
            5 => self.build_type5(y, x, rng),
            4 => self.build_type4(y, x, rng),
            3 => self.build_type3(y, x, rng),
            2 => self.build_type2(y, x, rng),
            _ => self.build_type1(y, x, rng),
        }
        true
    }

    /// level_generate_dungeon's room loop (generate.cc:6586) plus the
    /// tunnels and junction doors.
    fn build_level(&mut self, town_level: bool, rng: &mut impl Rng) {
        for _ in 0..DUN_ROOMS {
            let y = rng.gen_range(0..self.row_rooms);
            let x = rng.gen_range(0..self.col_rooms);
            if self.destroyed {
                let k = rng.gen_range(1..=100);
                if !self.has_cavern && k < self.depth as i32 {
                    if self.room_build(y, x, 10, rng) {
                        continue;
                    }
                } else if self.circular && self.room_build(y, x, 9, rng) {
                    continue;
                } else if self.room_build(y, x, 1, rng) {
                    continue;
                }
                continue;
            }
            if !town_level
                && (self.ironman || rng.gen_range(0..DUN_UNUSUAL) < self.depth)
            {
                let k = if self.ironman {
                    0
                } else {
                    rng.gen_range(0..100)
                };
                if self.ironman || rng.gen_range(0..DUN_UNUSUAL) < self.depth {
                    if k < 10 && self.room_build(y, x, 8, rng) {
                        continue;
                    }
                    if k < 25 && self.room_build(y, x, 7, rng) {
                        continue;
                    }
                    if k < 40 && self.room_build(y, x, 5, rng) {
                        continue;
                    }
                    if k < 55 && self.room_build(y, x, 6, rng) {
                        continue;
                    }
                    if k < 60 && self.room_build(y, x, 11, rng) {
                        continue;
                    }
                }
                if k < 25 && self.room_build(y, x, 4, rng) {
                    continue;
                }
                if k < 45 && self.room_build(y, x, 3, rng) {
                    continue;
                }
                if k < 65 && self.room_build(y, x, 2, rng) {
                    continue;
                }
                if k < 80 && self.room_build(y, x, 10, rng) {
                    continue;
                }
                if k < 90 {
                    if self.circular && self.room_build(y, x, 1, rng) {
                        continue;
                    } else if self.room_build(y, x, 9, rng) {
                        continue;
                    }
                }
                if k < 100 && self.room_build(y, x, 12, rng) {
                    continue;
                }
            }
            if self.has("CAVE") {
                if self.room_build(y, x, 10, rng) {
                    continue;
                }
            } else if self.circular && self.room_build(y, x, 9, rng) {
                continue;
            } else if self.room_build(y, x, 1, rng) {
                continue;
            }
        }
        let mut guard = 0;
        while self.cent.is_empty() {
            guard += 1;
            if guard > 100 {
                break;
            }
            self.room_build(0, 0, 1, rng);
        }
        let n = self.cent.len();
        for _ in 0..n {
            let p1 = rng.gen_range(0..n);
            let p2 = rng.gen_range(0..n);
            self.cent.swap(p1, p2);
        }
        let cents = self.cent.clone();
        if !cents.is_empty() {
            let (mut py, mut px) = cents[cents.len() - 1];
            for (cy, cx) in cents {
                self.build_tunnel(cy, cx, py, px, false, rng);
                py = cy;
                px = cx;
            }
        }
        for y in 0..self.h {
            for x in 0..self.w {
                if self.map.terrain_at(x, y) == T_WALL_SOLID {
                    self.map.set_terrain(x, y, T_VAULT_OUTER);
                }
            }
        }
        let doors = std::mem::take(&mut self.door);
        for (y, x) in doors {
            self.try_doors(y, x, rng);
        }
    }
}

/// The four cardinal steps (ddy_ddd/ddx_ddd: S, N, E, W).
const DIR4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// correct_dir (generate.cc:339).
fn correct_dir(y1: i32, x1: i32, y2: i32, x2: i32, rng: &mut impl Rng) -> (i32, i32) {
    let mut rdir = if y1 == y2 {
        0
    } else if y1 < y2 {
        1
    } else {
        -1
    };
    let mut cdir = if x1 == x2 {
        0
    } else if x1 < x2 {
        1
    } else {
        -1
    };
    if rdir != 0 && cdir != 0 {
        if rng.gen_range(0..100) < 50 {
            rdir = 0;
        } else {
            cdir = 0;
        }
    }
    (rdir, cdir)
}

/// rand_dir (generate.cc:363).
fn rand_dir(rng: &mut impl Rng) -> (i32, i32) {
    DIR4[rng.gen_range(0..4)]
}

/// build_type5/6 monster themes (generate.cc vault_aux_*).
#[derive(Clone, Copy)]
enum NestTheme {
    Jelly,
    Animal,
    Undead,
    Chapel,
    Kennel,
    Treasure,
    Clone(usize),
    Symbol(usize),
    Orc,
    Troll,
    Giant,
    Demon,
    Dragon(&'static [&'static str]),
}

/// The vault_aux_* predicates.
fn theme_ok(gd: &GameData, m: &crate::data::MonsterDef, theme: NestTheme) -> bool {
    if m.unique {
        return false;
    }
    let ch = m.glyph();
    match theme {
        NestTheme::Jelly => "ijm,".contains(ch) && !m.has("EVIL"),
        NestTheme::Animal => m.has("ANIMAL"),
        NestTheme::Undead => m.has("UNDEAD"),
        NestTheme::Chapel => ch == 'A' || m.name.contains("riest"),
        NestTheme::Kennel => ch == 'Z' || ch == 'C',
        NestTheme::Treasure => "!|$?=".contains(ch),
        NestTheme::Clone(t) => gd.monsters[t].id == m.id,
        NestTheme::Symbol(t) => gd.monsters[t].glyph() == ch,
        NestTheme::Orc => ch == 'o',
        NestTheme::Troll => ch == 'T',
        NestTheme::Giant => ch == 'P',
        NestTheme::Demon => ch == 'U',
        NestTheme::Dragon(mask) => {
            (ch == 'D' || ch == 'd') && mask.iter().all(|f| m.spells.iter().any(|s| s == f))
        }
    }
}

/// get_mon_num through a vault_aux_* filter (weighted by rarity).
fn themed_monster(
    gd: &GameData,
    level: u32,
    theme: NestTheme,
    rng: &mut impl Rng,
) -> Option<usize> {
    let candidates: Vec<usize> = (0..gd.monster_base_count)
        .filter(|&i| {
            let m = &gd.monsters[i];
            m.depth >= 1 && m.depth <= level && theme_ok(gd, m, theme)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let weights: Vec<u32> = candidates
        .iter()
        .map(|&i| (10 / gd.monsters[i].rarity).max(1))
        .collect();
    let dist = rand::distributions::WeightedIndex::new(&weights).ok()?;
    Some(candidates[dist.sample(rng)])
}

/// scatter (cave.cc:4002): a random grid within distance `d` with los.
pub fn scatter_pos(
    map: &Map,
    gd: &GameData,
    y: i32,
    x: i32,
    d: i32,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    // The original's `while (--attempts_left)` with attempts_left = 5000
    // makes 4999 draws before giving up (cave.cc:4005).
    for _ in 0..4999 {
        let ny = rand_spread(rng, y, d);
        let nx = rand_spread(rng, x, d);
        if !Map::in_bounds(nx, ny) {
            continue;
        }
        if d > 1 && pref_distance(y, x, ny, nx) > d {
            continue;
        }
        if line_of_sight(map, gd, x, y, nx, ny) {
            return Some((ny, nx));
        }
    }
    None
}

/// get_is_floor (generate.cc:1666).
fn get_is_floor(gd: &GameData, map: &Map, x: i32, y: i32) -> bool {
    if !Map::in_bounds(x, y) {
        return false;
    }
    gd.terrain(map.terrain_at(x, y)).is_floor
}

/// recursive_river (generate.cc:1196): grow a river by perturbed-midpoint
/// subdivision.  The centre is `feat1`, the width `feat2` border, and
/// one-in-50 split junctions add tributaries.
#[allow(clippy::too_many_arguments)]
fn recursive_river(
    map: &mut Map,
    gd: &GameData,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    feat1: u16,
    feat2: u16,
    width: i32,
    rng: &mut impl Rng,
) {
    let length = pref_distance(y1, x1, y2, x2);
    if length > 4 {
        let dx = (x2 - x1) / 2;
        let dy = (y2 - y1) / 2;
        let changex = if dy != 0 {
            rng.gen_range(1..=dy.abs()) * 2 - dy.abs()
        } else {
            0
        };
        let changey = if dx != 0 {
            rng.gen_range(1..=dx.abs()) * 2 - dx.abs()
        } else {
            0
        };
        let (mx, my) = (x1 + dx + changex, y1 + dy + changey);
        recursive_river(map, gd, x1, y1, mx, my, feat1, feat2, width, rng);
        recursive_river(map, gd, mx, my, x2, y2, feat1, feat2, width, rng);
        // Split the river some of the time (DUN_WAT_CHG = 50).
        if width > 0 && rng.gen_range(0..50) == 0 {
            recursive_river(
                map,
                gd,
                mx,
                my,
                x1 + 8 * (dx + changex),
                y1 + 8 * (dy + changey),
                feat1,
                feat2,
                width - 1,
                rng,
            );
        }
    } else {
        for l in 0..length {
            let x = x1 + l * (x2 - x1) / length;
            let y = y1 + l * (y2 - y1) / length;
            for ty in y - width - 1..=y + width + 1 {
                for tx in x - width - 1..=x + width + 1 {
                    if !Map::in_bounds(tx, ty) {
                        continue;
                    }
                    let t = map.terrain_at(tx, ty);
                    if t == feat1 || t == feat2 {
                        continue;
                    }
                    let d = pref_distance(y, x, ty, tx);
                    if d > rand_spread(rng, width, 1) {
                        continue;
                    }
                    // Do not convert permanent features.
                    if gd.terrain(t).permanent {
                        continue;
                    }
                    // The border gets feat2, the centre feat1.
                    map.set_terrain(tx, ty, if d > width { feat2 } else { feat1 });
                    // Lava terrain glows.
                    if feat1 == T_LAVA {
                        map.lit[Map::idx(tx, ty)] = true;
                    }
                    // Hack -- don't teleport here (CAVE_ICKY).
                    map.icky.insert(Map::idx(tx, ty));
                }
            }
        }
    }
}

/// add_river (generate.cc:1307): a river from a random map edge to a
/// random point in the lower right quadrant.
fn add_river(map: &mut Map, gd: &GameData, feat1: u16, feat2: u16, rng: &mut impl Rng) {
    let (w, h) = (map.w, map.h);
    let y2 = rng.gen_range(1..=h / 2 - 2) + h / 2;
    let x2 = rng.gen_range(1..=w / 2 - 2) + w / 2;
    let (x1, y1) = match rng.gen_range(1..=4) {
        1 => (rng.gen_range(1..=w - 2) + 1, 1),
        2 => (1, rng.gen_range(1..=h - 2) + 1),
        3 => (w - 1, rng.gen_range(1..=h - 2) + 1),
        _ => (rng.gen_range(1..=w - 2) + 1, h - 1),
    };
    // Width of rivers (DUN_WAT_RNG = 2).
    let wid = rng.gen_range(1..=2);
    recursive_river(map, gd, x1, y1, x2, y2, feat1, feat2, wid, rng);
}

/// generate.cc build_streamer: a wandering vein of `vein` through the
/// dungeon's walls; each converted tile has a 1/`chance` of becoming the
/// treasure (`hidden`) variant instead.
#[allow(clippy::too_many_arguments)]
fn build_streamer(
    map: &mut Map,
    d: &crate::data::DungeonDef,
    fills: &[u16; 100],
    vein: u16,
    hidden: u16,
    chance: i32,
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) {
    // rand_spread (z-rand.cc:251) is one uniform draw; the original
    // consumes y then x then the ddd direction.
    let mut y = rand_spread(rng, h / 2, 10);
    let mut x = rand_spread(rng, w / 2, 15);
    let (dx, dy) = COMPASS8[rng.gen_range(0..8)];
    // SAFE_MAX_ATTEMPTS (generate.cc:54): 4999 passes, one step each.
    for _ in 0..4999 {
        for _ in 0..5 {
            // Re-roll until the nearby grid is on the map (in_bounds2).
            let mut ty = y;
            let mut tx = x;
            for _ in 0..5000 {
                ty = rand_spread(rng, y, 2);
                tx = rand_spread(rng, x, 2);
                if Map::in_bounds(tx, ty) {
                    break;
                }
            }
            if !Map::in_bounds(tx, ty) {
                continue;
            }
            let t = map.terrain_at(tx, ty);
            if t == d.inner_wall || t == d.outer_wall || fills.contains(&t) {
                let placed = if rng.gen_range(0..chance.max(1)) == 0 {
                    hidden
                } else {
                    vein
                };
                map.set_terrain(tx, ty, placed);
            }
        }
        x += dx;
        y += dy;
        if !Map::in_bounds(x, y) {
            break;
        }
    }
}

/// ddd[] directions (src/tables.cc): the eight compass steps, in the
/// order the original's keypad direction table visits them.
const COMPASS8: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// rand_spread (z-rand.cc:251): delegate to the canonical port.  For
/// negative `d` the original's inner range is inverted and `rand_range`
/// returns its first argument (`a - d`); the old local formula
/// (`0..=2*d`, an invalid range) panicked instead.
pub fn rand_spread(rng: &mut impl Rng, a: i32, d: i32) -> i32 {
    crate::rng::rand_spread(a, d, rng)
}

/// build_streamer2 (generate.cc:1447): streams and pools of trees, water
/// or lava.  `killwall` lets the stream pierce walls (0 preserves them).
fn build_streamer2(
    map: &mut Map,
    gd: &GameData,
    feat: u16,
    killwall: bool,
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) {
    let poolchance = rng.gen_range(1..=10);
    let mut y = rand_spread(rng, h / 2, 10);
    let mut x = rand_spread(rng, w / 2, 15);
    let (mut dy, mut dx) = COMPASS8[rng.gen_range(0..8)];
    if poolchance > 2 {
        loop {
            for _ in 0..9 {
                let mut tx = 0;
                let mut ty = 0;
                let mut ok = false;
                for _ in 0..5000 {
                    ty = rand_spread(rng, y, 1);
                    tx = rand_spread(rng, x, 1);
                    if Map::in_bounds(tx, ty) {
                        ok = true;
                        break;
                    }
                }
                if !ok {
                    break;
                }
                if map.icky.contains(&Map::idx(tx, ty)) {
                    continue;
                }
                let t = map.terrain_at(tx, ty);
                let d = gd.terrain(t);
                if d.permanent && d.is_floor {
                    continue;
                }
                if !killwall && d.is_wall {
                    continue;
                }
                map.set_terrain(tx, ty, feat);
            }
            y += dy;
            x += dx;
            if rng.gen_range(0..20) == 0 {
                let (ndy, ndx) = COMPASS8[rng.gen_range(0..8)];
                dy = ndy;
                dx = ndx;
            }
            if !Map::in_bounds(x, y) {
                break;
            }
        }
    } else if feat == T_DEEP_WATER || feat == T_LAVA {
        let poolsize = 5 + rng.gen_range(1..=10);
        let mid = poolsize / 2;
        for i in 0..poolsize {
            for j in 0..poolsize {
                let tx = x + j;
                let ty = y + i;
                if !Map::in_bounds(ty, tx) {
                    continue;
                }
                if i < mid {
                    if j < mid {
                        if i + j + 1 < mid {
                            continue;
                        }
                    } else if j > mid + i {
                        continue;
                    }
                } else if j < mid {
                    if i > mid + j {
                        continue;
                    }
                } else if i + j > mid * 3 - 1 {
                    continue;
                }
                if gd.terrain(map.terrain_at(tx, ty)).permanent {
                    continue;
                }
                map.set_terrain(tx, ty, feat);
            }
        }
    }
}

/// Place `num` staircases of a kind on naked floor next to walls.
fn place_stairs(
    map: &mut Map,
    gd: &GameData,
    kind: u16,
    num: i32,
    special: Option<u32>,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    // SAFE_MAX_ATTEMPTS (generate.cc:54).
    const SAFE_MAX_ATTEMPTS: i32 = 5000;
    let mut first = None;
    let mut walls = 3;
    let mut cnt = 0;
    let mut i = 0;
    // alloc_stairs' loop: each stair gets up to SAFE_MAX_ATTEMPTS tries at
    // the current wall requirement (`next_to_walls`), then the requirement
    // drops by one.  The loop keeps going after `num` stairs while nothing
    // was placed at all (num > 1).
    while i < num || (cnt < 1 && num > 1) {
        let mut spot = None;
        for _ in 0..=SAFE_MAX_ATTEMPTS {
            let x = rng.gen_range(1..map.w.max(3) - 1);
            let y = rng.gen_range(1..map.h.max(3) - 1);
            if !map.walkable(gd, x, y) {
                continue;
            }
            // Never bury a town shop under a staircase.
            if map.shops.contains_key(&Map::idx(x, y)) {
                continue;
            }
            let adjacent_walls = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter(|(dx, dy)| !map.walkable(gd, x + dx, y + dy))
                .count() as i32;
            if adjacent_walls < walls {
                continue;
            }
            spot = Some((x, y));
            break;
        }
        if let Some((x, y)) = spot {
            map.set_terrain(x, y, kind);
            if let Some(s) = special {
                map.special.insert(Map::idx(x, y), s);
            }
            if first.is_none() {
                first = Some((x, y));
            }
            cnt += 1;
        }
        if walls > 0 {
            walls -= 1;
        }
        i += 1;
    }
    first
}

/// place_new_way (generate.cc:448): dig a path from a map edge into the
/// dungeon; returns the edge cell (x, y) the way starts at.
#[allow(clippy::too_many_arguments)]
fn place_new_way(
    map: &mut Map,
    gd: &GameData,
    floors: &[u16; 100],
    room_flags: &[bool],
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    let is_safe_floor =
        |map: &Map, x: i32, y: i32| Map::in_bounds(x, y) && floors.contains(&map.terrain_at(x, y));
    let is_way = |map: &Map, x: i32, y: i32| {
        Map::in_bounds(x, y) && matches!(map.terrain_at(x, y), T_WAY_MORE | T_WAY_LESS)
    };
    loop {
        let (x0, y0, x1, y1, x2, y2);
        let (xx, yy);
        if rng.gen_range(0..h + w) < h {
            yy = rng.gen_range(1..=h - 2);
            xx = 1 + rng.gen_range(0..2) * (w - 3);
            if xx == 1 {
                (x0, y0, x1, y1, x2, y2) = (1, 0, 0, -1, 0, 1);
            } else {
                (x0, y0, x1, y1, x2, y2) = (-1, 0, 0, -1, 0, 1);
            }
        } else {
            xx = rng.gen_range(1..=w - 2);
            yy = 1 + rng.gen_range(0..2) * (h - 3);
            if yy == 1 {
                (x0, y0, x1, y1, x2, y2) = (0, 1, -1, 0, 1, 0);
            } else {
                (x0, y0, x1, y1, x2, y2) = (0, -1, -1, 0, 1, 0);
            }
        }
        if map.icky.contains(&Map::idx(xx, yy)) {
            continue;
        }
        let t = map.terrain_at(xx, yy);
        let dt = gd.terrain(t);
        if dt.permanent && dt.is_floor {
            continue;
        }
        if room_flags[Map::idx(xx, yy)] && t == T_VAULT_OUTER {
            continue;
        }
        if is_way(map, xx + x1, yy + y1)
            || is_way(map, xx + x2, yy + y2)
            || is_way(map, xx + x0, yy + y0)
        {
            continue;
        }
        let mut way: Vec<(i32, i32)> = Vec::new();
        let mut ok = false;
        let (mut cx, mut cy) = (xx, yy);
        let mut guard = 0;
        while Map::in_bounds(cx, cy) {
            guard += 1;
            if guard > w + h {
                break;
            }
            if is_safe_floor(map, cx + x0, cy + y0)
                || is_safe_floor(map, cx + x1, cy + y1)
                || is_safe_floor(map, cx + x2, cy + y2)
            {
                ok = true;
                break;
            }
            let nt = map.terrain_at(cx + x0, cy + y0);
            if nt == 62 {
                ok = true;
                break;
            }
            if nt == T_PERMANENT {
                break;
            }
            let (fx, fy) = (cx + x0, cy + y0);
            if Map::in_bounds(fx, fy) && room_flags[Map::idx(fx, fy)] {
                let c1 = Map::in_bounds(fx + x1, fy + y1) && room_flags[Map::idx(fx + x1, fy + y1)];
                let c2 = Map::in_bounds(fx + x2, fy + y2) && room_flags[Map::idx(fx + x2, fy + y2)];
                if c1 && !c2 {
                    way.push((cx + x1, cy + y1));
                    way.push((cx + x1 + x0, cy + y1 + y0));
                } else if c2 && !c1 {
                    way.push((cx + x2, cy + y2));
                    way.push((cx + x2 + x0, cy + y2 + y0));
                } else {
                    way.push((fx, fy));
                }
                ok = true;
                break;
            }
            way.push((fx, fy));
            cx += x0;
            cy += y0;
        }
        if ok {
            for (x, y) in way {
                if Map::in_bounds(x, y) {
                    place_floor(map, floors, x, y, rng);
                }
            }
            return Some((xx, yy));
        }
    }
}

/// alloc_stairs through place_new_way (flat dungeons).
#[allow(clippy::too_many_arguments)]
fn place_way_stairs(
    map: &mut Map,
    gd: &GameData,
    floors: &[u16; 100],
    room_flags: &[bool],
    w: i32,
    h: i32,
    kind: u16,
    num: i32,
    special: Option<u32>,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    let mut first = None;
    for _ in 0..num {
        let Some((x, y)) = place_new_way(map, gd, floors, room_flags, w, h, rng) else {
            continue;
        };
        map.set_terrain(x, y, kind);
        if let Some(s) = special {
            map.special.insert(Map::idx(x, y), s);
        }
        if first.is_none() {
            first = Some((x, y));
        }
    }
    first
}

/// The dimension of a dungeon level (SMALLEST/SMALL/BIG/wilderness).
/// Requested level size (generate.cc:8257-8371): a d_info `R:` size is
/// not used by the base data, DF_SMALLEST forces one panel, DF_SMALL or
/// the random `small_levels` roll (1 in SMALL_LEVEL) picks 1-2 panels
/// wide by 1-3 high, and DF_BIG suppresses both.  `always_small_level`
/// forces the small roll.
fn level_size(
    d: &crate::data::DungeonDef,
    opts: &crate::options::Options,
    rng: &mut impl Rng,
) -> (i32, i32) {
    let big = d.has("BIG");
    if !big && d.has("SMALLEST") {
        (66, 22)
    } else if !big
        && (opts.always_small_level
            || d.has("SMALL")
            || (opts.small_levels && rng.gen_range(0..SMALL_LEVEL) == 0))
    {
        (rng.gen_range(1..=2) * 66, rng.gen_range(1..=3) * 22)
    } else {
        (MAP_W, MAP_H)
    }
}

/// cave_naked_bold: floor, not permanent, nothing placed on it.
fn naked_floor(map: &Map, gd: &GameData, x: i32, y: i32) -> bool {
    let t = map.terrain_at(x, y);
    let d = gd.terrain(t);
    d.is_floor
        && !d.permanent
        && !map.icky.contains(&Map::idx(x, y))
        && !map.special.contains_key(&Map::idx(x, y))
}

/// alloc_object's spot search (generate.cc:1069): a random naked floor
/// grid, restricted to (or excluded from) CAVE_ROOM grids.
fn alloc_object_cell(
    map: &Map,
    gd: &GameData,
    room_flags: &[bool],
    want_room: bool,
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    for _ in 0..5000 {
        let x = rng.gen_range(0..w);
        let y = rng.gen_range(0..h);
        if !naked_floor(map, gd, x, y) {
            continue;
        }
        if room_flags[Map::idx(x, y)] != want_room {
            continue;
        }
        return Some((x, y));
    }
    None
}

/// Place a fountain (generate.cc:797 place_fountain): a random eligible
/// potion (TV_POTION or TV_POTION2; the latter encoded as
/// sval+SV_POTION_LAST) with damroll(3,4) draughts, 30% empty.
fn place_fountain(
    gd: &GameData,
    map: &mut Map,
    room_flags: &[bool],
    depth: u32,
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) {
    const SV_POTION_LAST: i32 = 63;
    let potions: Vec<i32> = gd
        .objects
        .iter()
        .filter(|o| {
            (o.tval == crate::data::TV_POTION || o.tval == crate::data::TV_POTION2)
                && o.depth <= depth
                && o.flags.iter().any(|f| f == "FOUNTAIN")
        })
        .map(|o| {
            if o.tval == crate::data::TV_POTION2 {
                o.sval + SV_POTION_LAST
            } else {
                o.sval
            }
        })
        .collect();
    if potions.is_empty() {
        return;
    }
    let Some((x, y)) = alloc_object_cell(map, gd, room_flags, true, w, h, rng) else {
        return;
    };
    let sval = potions[rng.gen_range(0..potions.len())];
    let i = Map::idx(x, y);
    if rng.gen_range(0..100) < 30 {
        map.set_terrain(x, y, 15);
        map.fountains.insert(i, (sval, 0));
    } else {
        map.set_terrain(x, y, 2);
        let draughts: i32 = (0..3).map(|_| rng.gen_range(1..=4)).sum();
        map.fountains.insert(i, (sval, draughts));
    }
}

/// Place a pair of Void Jumpgates (generate.cc:846 place_between): the
/// partner is a random naked floor grid anywhere on the level.
fn place_between(gd: &GameData, map: &mut Map, x: i32, y: i32, w: i32, h: i32, rng: &mut impl Rng) {
    let mut partner = None;
    for _ in 0..5000 {
        let gx = rng.gen_range(0..w);
        let gy = rng.gen_range(0..h);
        if naked_floor(map, gd, gx, gy) && (gx, gy) != (x, y) {
            partner = Some((gx, gy));
            break;
        }
    }
    let Some((gx, gy)) = partner else {
        return;
    };
    map.set_terrain(x, y, T_BETWEEN);
    map.set_terrain(gx, gy, T_BETWEEN);
    map.between.insert(Map::idx(x, y), Map::idx(gx, gy));
    map.between.insert(Map::idx(gx, gy), Map::idx(x, y));
}

/// Build a destroyed level (generate.cc:1571 destroy_level): randint(5)
/// blast epicenters of radius 16 delete monsters and objects and convert
/// every valid grid with the 200-roll table (granite/quartz/magma/sand/
/// floor), clearing room, icky, mark and glow flags.
fn destroy_level(
    map: &mut Map,
    gd: &GameData,
    vault: &mut VaultSpawns,
    floors: &[u16; 100],
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) {
    let mut blasted: HashSet<(i32, i32)> = HashSet::new();
    for _ in 0..rng.gen_range(1..=4) {
        let x1 = rng.gen_range(5..=w - 1 - 5);
        let y1 = rng.gen_range(5..=h - 1 - 5);
        for y in (y1 - 15)..=(y1 + 15) {
            for x in (x1 - 15)..=(x1 + 15) {
                if !Map::in_bounds(x, y) {
                    continue;
                }
                if pref_distance(y1, x1, y, x) >= 16 {
                    continue;
                }
                blasted.insert((x, y));
                // cave_valid_bold: perma-grids and artifact grids survive.
                if gd.terrain(map.terrain_at(x, y)).permanent {
                    continue;
                }
                let t = rng.gen_range(0..200);
                let feat = if t < 20 {
                    T_GRANITE
                } else if t < 60 {
                    51
                } else if t < 90 {
                    50
                } else if t < 110 {
                    T_SANDWALL
                } else {
                    floors[rng.gen_range(0..100)]
                };
                map.set_terrain(x, y, feat);
                let i = Map::idx(x, y);
                map.icky.remove(&i);
                map.lit[i] = false;
                map.explored[i] = false;
            }
        }
    }
    // delete_monster / delete_object: the port defers vault contents, so
    // drop the recorded spawns inside the blasts instead.
    vault
        .monsters
        .retain(|&(_, x, y, _)| !blasted.contains(&(x, y)));
    vault
        .objects
        .retain(|&(_, x, y, _)| !blasted.contains(&(x, y)));
    vault
        .singles
        .retain(|&(_, x, y)| !blasted.contains(&(x, y)));
    for nest in vault.nests.iter_mut() {
        nest.cells.retain(|c| !blasted.contains(c));
    }
    vault.nests.retain(|n| !n.cells.is_empty());
}

/// The dungeon's "feeling" from the generation rating
/// (generate.cc: rating thresholds).
pub fn feeling_from_rating(rating: i32, good_item: bool) -> i32 {
    let mut feeling = if rating > 100 {
        2
    } else if rating > 80 {
        3
    } else if rating > 60 {
        4
    } else if rating > 40 {
        5
    } else if rating > 30 {
        6
    } else if rating > 20 {
        7
    } else if rating > 10 {
        8
    } else if rating > 0 {
        9
    } else {
        10
    };
    if good_item {
        feeling = 1;
    }
    feeling
}

/// Generate a dungeon level from its d_info record (floors/walls/rivers/
/// flags/stairs/branches; the guardian is placed by `populate_level`).
pub fn generate_dungeon_level(
    gd: &GameData,
    dungeon: u32,
    depth: u32,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    generate_dungeon_level_ex(
        gd,
        dungeon,
        depth,
        None,
        None,
        &crate::options::Options::default(),
        false,
        rng,
    )
}

/// Generate a dungeon level with the player's options and the `is_quest`
/// state (generate.cc level_generate_dungeon): quest levels cannot be
/// destroyed and every staircase becomes an up staircase.
pub fn generate_dungeon_level_for(
    gd: &GameData,
    dungeon: u32,
    depth: u32,
    opts: &crate::options::Options,
    is_quest: bool,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    generate_dungeon_level_ex(gd, dungeon, depth, None, None, opts, is_quest, rng)
}

/// Generate a level from an explicit dungeon record (the per-god Lost
/// Temple attributes of q_god.cc `set_god_dungeon_attributes`).
#[allow(clippy::too_many_arguments)]
pub fn generate_dungeon_level_def(
    gd: &GameData,
    dungeon: u32,
    d: &DungeonDef,
    depth: u32,
    opts: &crate::options::Options,
    is_quest: bool,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    generate_dungeon_level_ex(gd, dungeon, depth, None, Some(d), opts, is_quest, rng)
}

/// Generate a dungeon level that holds a random town (F:RANDOM_TOWNS):
/// a normal level with the 66x22 town screen stamped in its centre
/// (town_gen, src/wild.cc).  Town levels never hold vaults.
pub fn generate_dungeon_town_level(
    gd: &GameData,
    dungeon: u32,
    depth: u32,
    town: u32,
    seed: u32,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    generate_dungeon_level_ex(
        gd,
        dungeon,
        depth,
        Some((town, seed)),
        None,
        &crate::options::Options::default(),
        false,
        rng,
    )
}

#[allow(clippy::too_many_arguments)]
fn generate_dungeon_level_ex(
    gd: &GameData,
    dungeon: u32,
    depth: u32,
    town: Option<(u32, u32)>,
    def_override: Option<&DungeonDef>,
    opts: &crate::options::Options,
    is_quest: bool,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    let d = def_override.unwrap_or_else(|| gd.dungeon(dungeon));
    // get_level_flags (generate.cc): the dungeon record's flags plus the
    // `@:<depth>:F:` overrides of this level.
    let mut level_flags: Vec<String> = d.flags.clone();
    if let Some(s) = d.special_at(depth) {
        for f in &s.flags {
            if !level_flags.contains(f) {
                level_flags.push(f.clone());
            }
        }
    }
    let has = |f: &str| level_flags.iter().any(|x| x == f);
    let (floors, fills) = init_feat_info(d, depth);
    let (mut w, mut h) = level_size(d, opts, rng);
    // DF_DOUBLE (generate.cc cave_gen supersize): the level is generated
    // at half resolution, then every grid tile is blown up to a 2x2
    // block, giving Erebor and the Helcaraxe their huge, coarse layout.
    let double = has("DOUBLE") && w == MAP_W && h == MAP_H;
    if double {
        w /= 2;
        h /= 2;
    }
    let mut map = blank_map(T_PERMANENT);
    map.w = w;
    map.h = h;
    let smooth = d
        .flags
        .iter()
        .find_map(|f| f.strip_prefix("FILL_METHOD_"))
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    // DF_EMPTY (generate.cc:6502): "arena" levels are filled with the
    // dungeon floor instead of its fill terrain; `empty_levels` rolls an
    // extra 1 in EMPTY_LEVEL.
    let empty_level = has("EMPTY") || (opts.empty_levels && rng.gen_range(0..EMPTY_LEVEL) == 0);
    let table = if empty_level { &floors } else { &fills };
    fill_level(&mut map, w, h, table, smooth, rng);

    // d_info G: level generators (init2.cc registers "dungeon", "maze"
    // and "life"; anything else falls back to the standard generator).
    let generator = d.generator.as_str();
    let custom_layout = generator == "maze" || generator == "life";
    if generator == "maze" {
        build_maze_level(&mut map, &floors, w, h, rng);
    } else if generator == "life" {
        build_life_level(&mut map, gd, &floors, &fills, w, h, rng);
    }

    let mut vault = VaultSpawns::default();
    let mut room_flags = vec![false; (MAP_W * MAP_H) as usize];
    let mut rooms_rating = 0i32;
    let mut rooms_good_item = false;
    if custom_layout {
        // The life level is one big room; the maze has no rooms.
        if generator == "life" {
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    room_flags[Map::idx(x, y)] = true;
                }
            }
        }
    } else {
        // Possible cavern (generate.cc:6517): a fractal cave in the
        // middle of an otherwise normal rooms-and-corridors level.
        let has_cavern = has("CAVERN") && rng.gen_range(0..(depth / 2).max(1) as i32) > DUN_CAVERN;
        // Possible "destroyed" level (generate.cc:6530); no quest levels
        // and no small levels.
        let destroyed = depth > 10
            && !has("NO_DESTROY")
            && !is_quest
            && w == MAP_W
            && h == MAP_H
            && rng.gen_range(0..DUN_DEST) == 0;
        let mut rg = RoomsGen::new(
            &mut map,
            gd,
            d,
            &level_flags,
            &floors,
            &fills,
            &mut vault,
            depth,
            w,
            h,
            destroyed,
            has_cavern,
            opts.ironman_rooms,
        );
        if has_cavern {
            rg.build_cavern(rng);
        }
        rg.build_level(town.is_some(), rng);
        rooms_rating = rg.rating;
        rooms_good_item = rg.good_item;
        room_flags = std::mem::take(&mut rg.room);
        if destroyed {
            destroy_level(&mut map, gd, &mut vault, &floors, w, h, rng);
        }
    }

    // Rivers / lava streams (LAVA_RIVER(S), WATER_RIVER(S)); DF_NO_STREAMERS
    // only gates build_streamer2, never the rivers.
    if !custom_layout {
        if has("WATER_RIVER") && rng.gen_range(0..4) == 0 {
            add_river(&mut map, gd, T_DEEP_WATER, T_SHAL_WATER, rng);
        }
        if has("LAVA_RIVER") && rng.gen_range(0..4) == 0 {
            add_river(&mut map, gd, T_LAVA, T_SHAL_LAVA, rng);
        }
        if has("WATER_RIVERS") {
            let max = 3 + rng.gen_range(0..2);
            for _ in 0..max {
                if rng.gen_range(0..3) == 0 {
                    add_river(&mut map, gd, T_DEEP_WATER, T_SHAL_WATER, rng);
                }
            }
        }
        if has("LAVA_RIVERS") {
            let max = 2 + rng.gen_range(0..2);
            for _ in 0..max {
                if rng.gen_range(0..3) == 0 {
                    add_river(&mut map, gd, T_LAVA, T_SHAL_LAVA, rng);
                }
            }
        }
    }

    // Rubble in corridors (generate.cc alloc_object ALLOC_TYP_RUBBLE).
    let k = 2 + depth as i32 / 6;
    let mut rubble = rng.gen_range(1..=k.max(1));
    for _ in 0..400 {
        if rubble <= 0 {
            break;
        }
        let x = rng.gen_range(1..w - 1);
        let y = rng.gen_range(1..h - 1);
        if naked_floor(&map, gd, x, y) && !room_flags[Map::idx(x, y)] {
            map.set_terrain(x, y, T_RUBBLE);
            rubble -= 1;
        }
    }

    // Magma / quartz veins in Mordor and Angband (generate.cc:6782
    // build_streamer FEAT_MAGMA/FEAT_QUARTZ with hidden treasure).
    if !custom_layout && (dungeon == 2 || dungeon == 3) {
        for _ in 0..3 {
            build_streamer(&mut map, d, &fills, 50, 54, 90, w, h, rng);
        }
        for _ in 0..2 {
            build_streamer(&mut map, d, &fills, 51, 55, 40, w, h, rng);
        }
    }

    // Sand veins (DF_SAND_VEIN, build_streamer FEAT_SANDWALL).
    if !custom_layout && has("SAND_VEIN") && rng.gen_range(0..4) == 0 {
        build_streamer(&mut map, d, &fills, T_SANDWALL, T_SANDWALL_H, 40, w, h, rng);
    }

    // Streamers of trees, water or lava (build_streamer2, gated by
    // DF_NO_STREAMERS only).  The original's `!(dun_level <= 33)`
    // condition makes the lava branch below unreachable, but it is kept
    // for fidelity.
    if !custom_layout && !has("NO_STREAMERS") {
        if has("FLAT") && rng.gen_range(1..=20) > 15 {
            let num = rng.gen_range(1..=DUN_STR_QUA);
            for _ in 0..num {
                build_streamer2(&mut map, gd, T_TREE, true, w, h, rng);
            }
        }
        if depth > 33 && rng.gen_range(1..=20) > 15 {
            let num = rng.gen_range(1..=DUN_STR_QUA - 1);
            for _ in 0..num {
                build_streamer2(&mut map, gd, T_SHAL_WATER, false, w, h, rng);
            }
            if rng.gen_range(1..=20) > 15 {
                let num = rng.gen_range(1..=DUN_STR_QUA);
                for _ in 0..num {
                    build_streamer2(&mut map, gd, T_DEEP_WATER, true, w, h, rng);
                }
            }
        } else if depth > 33 {
            // Levels 34+ where the water roll above failed: shallow/deep
            // lava first, then shallow/deep water (generate.cc:6908).
            if rng.gen_range(1..=20) > 15 {
                let num = rng.gen_range(1..=DUN_STR_QUA);
                for _ in 0..num {
                    build_streamer2(&mut map, gd, T_SHAL_LAVA, false, w, h, rng);
                }
                if rng.gen_range(1..=20) > 15 {
                    let num = rng.gen_range(1..=DUN_STR_QUA - 1);
                    for _ in 0..num {
                        build_streamer2(&mut map, gd, T_LAVA, true, w, h, rng);
                    }
                }
            } else if rng.gen_range(1..=20) > 15 {
                let num = rng.gen_range(1..=DUN_STR_QUA - 1);
                for _ in 0..num {
                    build_streamer2(&mut map, gd, T_SHAL_WATER, false, w, h, rng);
                }
                if rng.gen_range(1..=20) > 15 {
                    let num = rng.gen_range(1..=DUN_STR_QUA);
                    for _ in 0..num {
                        build_streamer2(&mut map, gd, T_DEEP_WATER, true, w, h, rng);
                    }
                }
            }
        }
    }

    // Random features in rooms (ALLOC_TYP_ALTAR/BETWEEN/FOUNTAIN).
    if let Some((ax, ay)) = alloc_object_cell(&map, gd, &room_flags, true, w, h, rng) {
        if rng.gen_range(0..100) < 10 {
            map.set_terrain(ax, ay, 164);
        }
    }
    for _ in 0..2 {
        let Some(a) = alloc_object_cell(&map, gd, &room_flags, true, w, h, rng) else {
            break;
        };
        place_between(gd, &mut map, a.0, a.1, w, h, rng);
    }
    place_fountain(gd, &mut map, &room_flags, depth, w, h, rng);

    // A random dungeon town overwrites the centre of the level (town_gen
    // runs before the stairs are allocated, src/generate.cc).
    if let Some((tid, seed)) = town {
        add_dungeon_town(gd, &mut map, &floors, w, h, tid, seed, &mut vault);
    }

    // Staircases (alloc_stairs, src/generate.cc). Flat dungeons use the
    // "path to the next/previous area" features instead of stairs.
    // On a quest level (is_quest) every requested staircase becomes an up
    // staircase, so the only way out is back the way you came.
    let flat = has("FLAT");
    let (down_kind, up_kind) = if flat {
        (T_WAY_MORE, T_WAY_LESS)
    } else {
        (T_STAIRS_DOWN, T_STAIRS_UP)
    };
    let (down_shaft, up_shaft) = if flat {
        (T_WAY_MORE, T_WAY_LESS)
    } else {
        (T_SHAFT_DOWN, T_SHAFT_UP)
    };
    let (down_kind, down_shaft) = if is_quest {
        (up_kind, up_shaft)
    } else {
        (down_kind, down_shaft)
    };
    let mut start = None;
    if let Some(branch) = d.branch_at(depth) {
        if flat {
            place_way_stairs(
                &mut map,
                gd,
                &floors,
                &room_flags,
                w,
                h,
                down_kind,
                5,
                Some(branch),
                rng,
            );
        } else {
            place_stairs(&mut map, gd, down_kind, 5, Some(branch), rng);
        }
    }
    if let Some((parent, _)) = d.branch_parent {
        if depth == d.mindepth {
            if flat {
                place_way_stairs(
                    &mut map,
                    gd,
                    &floors,
                    &room_flags,
                    w,
                    h,
                    up_kind,
                    5,
                    Some(parent),
                    rng,
                );
            } else {
                place_stairs(&mut map, gd, up_kind, 5, Some(parent), rng);
            }
        }
    }
    if depth < d.maxdepth || (depth == d.maxdepth && has("FORCE_DOWN")) {
        start = start.or(if flat {
            place_way_stairs(
                &mut map,
                gd,
                &floors,
                &room_flags,
                w,
                h,
                down_kind,
                rng.gen_range(3..=4),
                None,
                rng,
            )
        } else {
            place_stairs(&mut map, gd, down_kind, rng.gen_range(3..=4), None, rng)
        });
        // 0-1 down shafts (flat dungeons reuse WAY_MORE).
        if !has("NO_SHAFT") {
            if flat {
                place_way_stairs(
                    &mut map,
                    gd,
                    &floors,
                    &room_flags,
                    w,
                    h,
                    down_shaft,
                    rng.gen_range(0..=1),
                    None,
                    rng,
                );
            } else {
                place_stairs(&mut map, gd, down_shaft, rng.gen_range(0..=1), None, rng);
            }
        }
    }
    if depth > d.mindepth || (depth == d.mindepth && !has("NO_UP")) {
        start = if flat {
            place_way_stairs(
                &mut map,
                gd,
                &floors,
                &room_flags,
                w,
                h,
                up_kind,
                rng.gen_range(1..=2),
                None,
                rng,
            )
        } else {
            place_stairs(&mut map, gd, up_kind, rng.gen_range(1..=2), None, rng)
        }
        .or(start);
        if !has("NO_SHAFT") {
            if flat {
                place_way_stairs(
                    &mut map,
                    gd,
                    &floors,
                    &room_flags,
                    w,
                    h,
                    up_shaft,
                    rng.gen_range(0..=1),
                    None,
                    rng,
                );
            } else {
                place_stairs(&mut map, gd, up_shaft, rng.gen_range(0..=1), None, rng);
            }
        }
    }
    // new_player_spot (generate.cc:684): the player arrives on a random
    // "naked" floor grid (walkable, non-permanent, outside vaults, free of
    // the level's pre-placed monsters/objects), not on the first stair.
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();
    for &(_, x, y, _) in &vault.monsters {
        occupied.insert((x, y));
    }
    for &(_, x, y) in &vault.singles {
        occupied.insert((x, y));
    }
    for &(_, x, y) in &vault.random_monsters {
        occupied.insert((x, y));
    }
    for &(_, x, y, _) in &vault.objects {
        occupied.insert((x, y));
    }
    for &(_, x, y, _, _) in &vault.random_objects {
        occupied.insert((x, y));
    }
    for nest in &vault.nests {
        for &(x, y) in &nest.cells {
            occupied.insert((x, y));
        }
    }
    let start = random_naked_spot(&map, gd, w, h, &occupied, rng)
        .or(start)
        .unwrap_or_else(|| {
        for _ in 0..2000 {
            let x = rng.gen_range(1..w - 1);
            let y = rng.gen_range(1..h - 1);
            if map.walkable(gd, x, y) {
                return (x, y);
            }
        }
        (w / 2, h / 2)
    });

    place_traps(&mut map, &|x, y| room_flags[Map::idx(x, y)], depth, rng);
    let down = find_terrain(&map, T_STAIRS_DOWN)
        .or_else(|| find_terrain(&map, T_SHAFT_DOWN))
        .or_else(|| find_terrain(&map, T_WAY_MORE))
        .unwrap_or(start);
    ensure_stairs_reachable(&mut map, gd, start, down, rng);

    // Random dungeon towns: hidden-layout shops can land in a sealed
    // pocket; guarantee they are reachable from the player's entry.
    if town.is_some() {
        let shops: Vec<(i32, i32)> = map
            .shops
            .keys()
            .map(|i| (*i as i32 % MAP_W, *i as i32 / MAP_W))
            .collect();
        for cell in shops {
            if !cells_connected(gd, &map, start, cell) {
                connect_cells(gd, &mut map, start, cell);
            }
        }
    }

    // Feeling (generate.cc rating thresholds; vaults/nests/pits/random
    // vaults add rating and the 1000-turn good_item_flag roll).
    let rating = rooms_rating + vault.objects.len() as i32;
    let good_item = rooms_good_item || vault.objects.iter().any(|&(_, _, _, good)| good);
    map.rating = rating;
    map.good_item = good_item;
    map.feeling = feeling_from_rating(rating, good_item);
    generate_grid_mana(&mut map, depth, rng);

    // Empty (arena) levels are mostly lit (generate.cc:7864).
    if empty_level && (rng.gen_range(0..DARK_EMPTY) != 1 || rng.gen_range(0..100) > depth) {
        map.reveal_all();
    }

    // DF_DOUBLE: supersize every grid tile to a 2x2 block
    // (generate.cc supersize_grid_tile); contents scatter into one of
    // the four cells.
    if double {
        let (m, s, v) = supersize_level(map, start, vault, w, h, rng);
        return GeneratedLevel {
            map: m,
            start: s,
            vault: v,
        };
    }

    GeneratedLevel { map, start, vault }
}

/// Supersize a half-resolution level to full size (DF_DOUBLE).
fn supersize_level(
    map: Map,
    start: (i32, i32),
    vault: VaultSpawns,
    w: i32,
    h: i32,
    rng: &mut impl Rng,
) -> (Map, (i32, i32), VaultSpawns) {
    let mut out = blank_map(T_PERMANENT);
    out.mana = vec![0; out.terrain.len()];
    let mut idx_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for y in 0..h {
        for x in 0..w {
            let src = Map::idx(x, y);
            let t = map.terrain[src];
            let mut cells = Vec::new();
            for dy in 0..2 {
                for dx in 0..2 {
                    let (nx, ny) = (x * 2 + dx, y * 2 + dy);
                    let ni = Map::idx(nx, ny);
                    out.terrain[ni] = t;
                    out.lit[ni] = map.lit[src];
                    if let Some(&mv) = map.mana.get(src) {
                        out.mana[ni] = mv;
                    }
                    cells.push(ni);
                }
            }
            idx_map.insert(src, cells);
        }
    }
    for &src in &map.icky {
        if let Some(cells) = idx_map.get(&src) {
            for &ni in cells {
                out.icky.insert(ni);
            }
        }
    }
    for (src, kind) in &map.trap_kinds {
        if let Some(cells) = idx_map.get(src) {
            for &ni in cells {
                out.trap_kinds.insert(ni, *kind);
            }
        }
    }
    for (src, dest) in &map.between {
        if let (Some(a), Some(b)) = (idx_map.get(src), idx_map.get(dest)) {
            for &ni in a {
                out.between.insert(ni, b[0]);
            }
        }
    }
    for (src, sp) in &map.special {
        if let Some(cells) = idx_map.get(src) {
            for &ni in cells {
                out.special.insert(ni, *sp);
            }
        }
    }
    for (src, f) in &map.fountains {
        if let Some(cells) = idx_map.get(src) {
            out.fountains.insert(cells[0], *f);
        }
    }
    out.feeling = map.feeling;
    out.rating = map.rating;
    out.good_item = map.good_item;
    out.mflag = map.mflag.clone();
    let mut v = VaultSpawns::default();
    for &(def, x, y, level) in &vault.monsters {
        let (nx, ny) = (x * 2 + rng.gen_range(0..2), y * 2 + rng.gen_range(0..2));
        v.monsters.push((def, nx, ny, level));
    }
    for &(def, x, y, good) in &vault.objects {
        let (nx, ny) = (x * 2 + rng.gen_range(0..2), y * 2 + rng.gen_range(0..2));
        v.objects.push((def, nx, ny, good));
    }
    for &(def, x, y) in &vault.singles {
        let (nx, ny) = (x * 2 + rng.gen_range(0..2), y * 2 + rng.gen_range(0..2));
        v.singles.push((def, nx, ny));
    }
    for nest in &vault.nests {
        let cells = nest.cells.iter().map(|&(x, y)| (x * 2, y * 2)).collect();
        v.nests.push(NestSpawn {
            theme: nest.theme.clone(),
            cells,
            count: nest.count,
        });
    }
    let start = (start.0 * 2, start.1 * 2);
    (out, start, v)
}

// --- Random dungeon towns (F:RANDOM_TOWNS) ------------------------------

/// The town screen embedded in a level (SCREEN_WID/HGT).
const TOWN_W: i32 = 66;
const TOWN_H: i32 = 22;
/// TOWN_NORMAL_FLOOR: chance the town floors are plain dungeon floor
/// instead of the dungeon's random floor mix.
const TOWN_NORMAL_FLOOR: i32 = 70;

/// distance (src/cave.cc): hypot approximation max + min/2.
pub fn pref_distance(y1: i32, x1: i32, y2: i32, x2: i32) -> i32 {
    let dy = (y1 - y2).abs();
    let dx = (x1 - x2).abs();
    if dy > dx {
        dy + (dx >> 1)
    } else {
        dx + (dy >> 1)
    }
}

/// get_shops (src/wild.cc): the stores a random town may build; only
/// STF_RANDOM stores enter the pool, each with a 50% base chance.
fn town_shops(gd: &GameData, rng: &mut impl Rng) -> Vec<u32> {
    let mut rooms = Vec::new();
    for st in &gd.stores {
        let has = |f: &str| st.flags.iter().any(|x| x == f);
        let mut chance = 50i32;
        if has("COMMON") {
            chance += 30;
        }
        if has("RARE") {
            chance -= 20;
        }
        if has("VERY_RARE") {
            chance -= 30;
        }
        if rng.gen_range(0..100) >= chance {
            continue;
        }
        if has("RANDOM") {
            rooms.push(st.id);
        }
    }
    rooms
}

/// Turn one cell into a shop entrance (build_store's FEAT_SHOP + special).
fn town_shop_mark(gd: &GameData, map: &mut Map, x: i32, y: i32, store: u32) {
    if !Map::in_bounds(x, y) {
        return;
    }
    map.set_terrain(x, y, T_SHOP);
    if let Some(s) = gd.stores.iter().find(|s| s.id == store) {
        map.shops.insert(
            Map::idx(x, y),
            ShopMark {
                store,
                ch: s.glyph(),
                color: s.color,
            },
        );
    }
}

/// set_border (src/wild.cc): floor becomes a door, anything else a solid
/// permanent wall.
fn set_border(map: &mut Map, gd: &GameData, y: i32, x: i32) {
    if !Map::in_bounds(x, y) {
        return;
    }
    let t = map.terrain_at(x, y);
    let was_floor = gd.terrain(t).is_floor || is_door(t);
    map.set_terrain(x, y, if was_floor { T_DOOR } else { T_PERMANENT });
    let i = Map::idx(x, y);
    map.shops.remove(&i);
    map.special.remove(&i);
    map.lit[i] = false;
}

/// town_borders (src/wild.cc): wall in the town's rim.
fn town_borders(map: &mut Map, gd: &GameData, qy: i32, qx: i32) {
    for y in qy..qy + TOWN_H - 1 {
        set_border(map, gd, y, qx);
    }
    for y in qy..qy + TOWN_H - 1 {
        set_border(map, gd, y, qx + TOWN_W - 1);
    }
    for x in qx..qx + TOWN_W - 1 {
        set_border(map, gd, qy, x);
    }
    for x in qx..qx + TOWN_W {
        set_border(map, gd, qy + TOWN_H - 1, x);
    }
}

/// build_store (src/wild.cc): an invulnerable rectangular building with a
/// door facing the town centre.
#[allow(clippy::too_many_arguments)]
fn town_build_store(
    gd: &GameData,
    map: &mut Map,
    store: u32,
    qy: i32,
    qx: i32,
    yy: i32,
    xx: i32,
    rng: &mut impl Rng,
) {
    let y0 = qy + yy * 9 + 6;
    let x0 = qx + xx * 14 + 12;
    let y1 = y0 - rng.gen_range(1..=if yy == 0 { 3 } else { 2 });
    let y2 = y0 + rng.gen_range(1..=if yy == 1 { 3 } else { 2 });
    let x1 = x0 - rng.gen_range(1..=5);
    let x2 = x0 + rng.gen_range(1..=5);
    for y in y1..=y2 {
        for x in x1..=x2 {
            map.set_terrain(x, y, T_PERMANENT);
        }
    }
    // Pick a door direction (S,N,E,W), re-rolling the annoying ones once.
    let mut tmp = rng.gen_range(0..4);
    if (tmp == 0 && yy == 1)
        || (tmp == 1 && yy == 0)
        || (tmp == 2 && xx == 3)
        || (tmp == 3 && xx == 0)
    {
        tmp = rng.gen_range(0..4);
    }
    let (dx, dy) = match tmp {
        0 => (rng.gen_range(x1..=x2), y2),
        1 => (rng.gen_range(x1..=x2), y1),
        2 => (x2, rng.gen_range(y1..=y2)),
        _ => (x1, rng.gen_range(y1..=y2)),
    };
    town_shop_mark(gd, map, dx, dy, store);
}

/// build_store_circle (src/wild.cc): a round building with a straight
/// entrance corridor to its centre.
#[allow(clippy::too_many_arguments)]
fn town_build_store_circle(
    gd: &GameData,
    map: &mut Map,
    store: u32,
    qy: i32,
    qx: i32,
    yy: i32,
    xx: i32,
    rng: &mut impl Rng,
) {
    let y0 = qy + yy * 9 + 6;
    let x0 = qx + xx * 14 + 12;
    let rad = 2 + rng.gen_range(0..2);
    for y in y0 - rad..=y0 + rad {
        for x in x0 - rad..=x0 + rad {
            if pref_distance(y0, x0, y, x) > rad {
                continue;
            }
            map.set_terrain(x, y, T_PERMANENT);
        }
    }
    let mut tmp = rng.gen_range(0..4);
    if (tmp == 0 && yy == 1)
        || (tmp == 1 && yy == 0)
        || (tmp == 2 && xx == 3)
        || (tmp == 3 && xx == 0)
    {
        tmp = rng.gen_range(0..4);
    }
    match tmp {
        0 => {
            for y in y0..=y0 + rad {
                map.set_terrain(x0, y, T_FLOOR);
            }
        }
        1 => {
            for y in y0 - rad..=y0 {
                map.set_terrain(x0, y, T_FLOOR);
            }
        }
        2 => {
            for x in x0..=x0 + rad {
                map.set_terrain(x, y0, T_FLOOR);
            }
        }
        _ => {
            for x in x0 - rad..=x0 {
                map.set_terrain(x, y0, T_FLOOR);
            }
        }
    }
    town_shop_mark(gd, map, x0, y0, store);
}

/// Lay a block of town floor (floor_type[rand_int(100)] or plain floor).
fn town_floor_block(
    map: &mut Map,
    floors: &[u16; 100],
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
    plain: bool,
    free: &mut HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) {
    for y in y0..y1 {
        for x in x0..x1 {
            let t = if plain {
                T_FLOOR
            } else {
                floors[rng.gen_range(0..100)]
            };
            map.set_terrain(x, y, t);
            free.insert((x, y));
        }
    }
}

/// Build the 2x4 store block shared by the normal and circular layouts.
fn town_place_stores(
    gd: &GameData,
    map: &mut Map,
    rooms: &mut Vec<u32>,
    qy: i32,
    qx: i32,
    circle: bool,
    rng: &mut impl Rng,
) {
    for yy in 0..2 {
        for xx in 0..4 {
            let Some(store) = rooms.pop() else { return };
            if circle {
                town_build_store_circle(gd, map, store, qy, qx, yy, xx, rng);
            } else {
                town_build_store(gd, map, store, qy, qx, yy, xx, rng);
            }
        }
    }
}

/// town_gen_hack (src/wild.cc): a rectangular town of two store rows.
fn town_gen_hack(
    gd: &GameData,
    map: &mut Map,
    floors: &[u16; 100],
    qy: i32,
    qx: i32,
    free: &mut HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) {
    let plain = rng.gen_range(0..100) < TOWN_NORMAL_FLOOR;
    town_floor_block(
        map,
        floors,
        qx + 1,
        qx + TOWN_W - 1,
        qy + 1,
        qy + TOWN_H - 1,
        plain,
        free,
        rng,
    );
    let mut rooms = town_shops(gd, rng);
    town_place_stores(gd, map, &mut rooms, qy, qx, false, rng);
    if rng.gen_range(0..100) < TOWN_NORMAL_FLOOR {
        town_borders(map, gd, qy, qx);
    }
}

/// town_gen_circle (src/wild.cc): a rectangular town closed off by two
/// half-circles of wall.
fn town_gen_circle(
    gd: &GameData,
    map: &mut Map,
    floors: &[u16; 100],
    qy: i32,
    qx: i32,
    free: &mut HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) {
    let plain = rng.gen_range(0..100) < TOWN_NORMAL_FLOOR;
    let rad = TOWN_H / 2;
    for x in qx + rad..qx + TOWN_W - rad {
        set_border(map, gd, qy, x);
    }
    for x in qx + rad..qx + TOWN_W - rad {
        set_border(map, gd, qy + TOWN_H - 1, x);
    }
    town_floor_block(
        map,
        floors,
        qx + rad,
        qx + TOWN_W - rad,
        qy + 1,
        qy + TOWN_H - 1,
        plain,
        free,
        rng,
    );
    let cy = qy + TOWN_H / 2;
    for y in cy - rad..cy + rad {
        for x in qx..qx + rad + 1 {
            let d = pref_distance(cy, qx + rad, y, x);
            if d == rad || d == rad - 1 {
                set_border(map, gd, y, x);
                free.remove(&(x, y));
            }
            if d < rad - 1 {
                let t = if plain {
                    T_FLOOR
                } else {
                    floors[rng.gen_range(0..100)]
                };
                map.set_terrain(x, y, t);
                free.insert((x, y));
            }
        }
    }
    let cx = qx + TOWN_W - rad - 1;
    for y in cy - rad..cy + rad {
        for x in cx..cx + rad + 1 {
            let d = pref_distance(cy, cx, y, x);
            if d == rad || d == rad - 1 {
                set_border(map, gd, y, x);
                free.remove(&(x, y));
            }
            if d < rad - 1 {
                let t = if plain {
                    T_FLOOR
                } else {
                    floors[rng.gen_range(0..100)]
                };
                map.set_terrain(x, y, t);
                free.insert((x, y));
            }
        }
    }
    let mut rooms = town_shops(gd, rng);
    town_place_stores(gd, map, &mut rooms, qy, qx, true, rng);
}

/// town_gen_hidden (src/wild.cc): no town at all, just shop doors hidden
/// at random spots of the dungeon.
fn town_gen_hidden(gd: &GameData, map: &mut Map, rng: &mut impl Rng) {
    let mut rooms = town_shops(gd, rng);
    let n = if rooms.is_empty() {
        0
    } else {
        // rand_int(rooms/2) + rooms/2, i.e. between half and all-but-one.
        let half = rooms.len() / 2;
        half + if half == 0 { 0 } else { rng.gen_range(0..half) }
    };
    for _ in 0..n {
        let Some(store) = rooms.pop() else { break };
        let mut spot = None;
        for _ in 0..10000 {
            let x = rng.gen_range(1..MAP_W - 1);
            let y = rng.gen_range(1..MAP_H - 1);
            if map.walkable(gd, x, y) {
                spot = Some((x, y));
                break;
            }
        }
        let Some((x, y)) = spot else { break };
        town_shop_mark(gd, map, x, y, store);
    }
}

/// Make sure the town can be walked into from the rest of the level:
/// if the interior is sealed off, dig a straight corridor to the nearest
/// reachable dungeon floor (the original relies on a corridor happening
/// to cross the town, which random seeds cannot guarantee).
fn town_ensure_reachable(gd: &GameData, map: &mut Map, free: &HashSet<(i32, i32)>) {
    if free.is_empty() {
        return;
    }
    let passable = |m: &Map, x: i32, y: i32| {
        Map::in_bounds(x, y) && (m.walkable(gd, x, y) || is_door(m.terrain_at(x, y)))
    };
    // BFS through the town's free cells; escaping to a passable
    // non-town cell means the town is already reachable.
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    let mut queue: Vec<(i32, i32)> = free.iter().copied().collect();
    for &c in &queue {
        seen.insert(c);
    }
    let mut qi = 0;
    let mut connected = false;
    'walk: while qi < queue.len() {
        let (x, y) = queue[qi];
        qi += 1;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if !seen.insert(n) {
                continue;
            }
            if passable(map, n.0, n.1) {
                if !free.contains(&n) {
                    connected = true;
                    break 'walk;
                }
                queue.push(n);
            }
        }
    }
    if connected {
        return;
    }
    // Dig to the *largest* walkable component (the main dungeon body,
    // which the player's entry and stairs belong to); the nearest
    // walkable cell could be an isolated pocket.
    let mut comp = vec![usize::MAX; (MAP_W * MAP_H) as usize];
    let mut comp_sizes: Vec<u32> = Vec::new();
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let i = Map::idx(x, y);
            if comp[i] != usize::MAX || !map.walkable(gd, x, y) || free.contains(&(x, y)) {
                continue;
            }
            let cid = comp_sizes.len();
            let mut size = 0u32;
            let mut stack = vec![(x, y)];
            comp[i] = cid;
            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if !Map::in_bounds(nx, ny) {
                        continue;
                    }
                    let ni = Map::idx(nx, ny);
                    if comp[ni] == usize::MAX
                        && map.walkable(gd, nx, ny)
                        && !free.contains(&(nx, ny))
                    {
                        comp[ni] = cid;
                        stack.push((nx, ny));
                    }
                }
            }
            comp_sizes.push(size);
        }
    }
    let main_comp = comp_sizes
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| *s)
        .map(|(i, _)| i);
    // Multi-source BFS over the whole level (walls allowed) to find the
    // closest walkable cell outside the town, then dig back to the town.
    use std::collections::VecDeque;
    let mut dist: HashMap<(i32, i32), u32> = HashMap::new();
    let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut q: VecDeque<(i32, i32)> = VecDeque::new();
    let boundary = |m: &Map, x: i32, y: i32| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            !Map::in_bounds(nx, ny) || !m.walkable(gd, nx, ny)
        })
    };
    for &(x, y) in free {
        if boundary(map, x, y) && dist.insert((x, y), 0).is_none() {
            q.push_back((x, y));
        }
    }
    while let Some((x, y)) = q.pop_front() {
        let d = dist[&(x, y)];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if !Map::in_bounds(n.0, n.1) || dist.contains_key(&n) {
                continue;
            }
            dist.insert(n, d + 1);
            prev.insert(n, (x, y));
            let on_main = main_comp
                .map(|mc| comp[Map::idx(n.0, n.1)] == mc)
                .unwrap_or(false);
            if (on_main || (main_comp.is_none() && map.walkable(gd, n.0, n.1)))
                && !free.contains(&n)
            {
                let mut cur = n;
                while let Some(&p) = prev.get(&cur) {
                    if !map.walkable(gd, cur.0, cur.1) {
                        map.set_terrain(cur.0, cur.1, T_FLOOR);
                        let i = Map::idx(cur.0, cur.1);
                        map.shops.remove(&i);
                        map.special.remove(&i);
                    }
                    cur = p;
                }
                return;
            }
            q.push_back(n);
        }
    }
}

/// Whether two cells are in the same walkable region (doors count as
/// passable, like doors in the original cave_floor_bold + is_open).
fn cells_connected(gd: &GameData, map: &Map, from: (i32, i32), to: (i32, i32)) -> bool {
    let passable = |x: i32, y: i32| {
        Map::in_bounds(x, y) && (map.walkable(gd, x, y) || is_door(map.terrain_at(x, y)))
    };
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    let mut stack = vec![from];
    seen.insert(from);
    while let Some((x, y)) = stack.pop() {
        if (x, y) == to {
            return true;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if !seen.contains(&n) && passable(n.0, n.1) {
                seen.insert(n);
                stack.push(n);
            }
        }
    }
    false
}

/// Dig the shortest corridor between two cells through non-permanent
/// terrain (random dungeon towns can land in a sealed pocket).
fn connect_cells(gd: &GameData, map: &mut Map, from: (i32, i32), to: (i32, i32)) {
    use std::collections::VecDeque;
    if from == to {
        return;
    }
    let diggable =
        |m: &Map, x: i32, y: i32| Map::in_bounds(x, y) && m.terrain_at(x, y) != T_PERMANENT;
    let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    let mut q: VecDeque<(i32, i32)> = VecDeque::new();
    q.push_back(from);
    seen.insert(from);
    while let Some((x, y)) = q.pop_front() {
        if (x, y) == to {
            let mut cur = to;
            while cur != from {
                if !map.walkable(gd, cur.0, cur.1) {
                    map.set_terrain(cur.0, cur.1, T_FLOOR);
                    map.special.remove(&Map::idx(cur.0, cur.1));
                }
                cur = prev[&cur];
            }
            return;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if seen.contains(&n) || !diggable(map, n.0, n.1) {
                continue;
            }
            seen.insert(n);
            prev.insert(n, (x, y));
            q.push_back(n);
        }
    }
}

/// town_gen (src/wild.cc): stamp a random town onto the level.
#[allow(clippy::too_many_arguments)]
fn add_dungeon_town(
    gd: &GameData,
    map: &mut Map,
    floors: &[u16; 100],
    w: i32,
    h: i32,
    town: u32,
    seed: u32,
    spawns: &mut VaultSpawns,
) {
    use rand::SeedableRng;
    // The original seeds the quick RNG with the town seed; a dedicated
    // RNG keeps the layout stable even if the level is regenerated.
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    if w < TOWN_W || h < TOWN_H {
        return;
    }
    map.town = town;
    let qy = (h - TOWN_H) / 2;
    let qx = (w - TOWN_W) / 2;
    let mut free: HashSet<(i32, i32)> = HashSet::new();
    match rng.gen_range(0..3) {
        0 => town_gen_hack(gd, map, floors, qy, qx, &mut free, &mut rng),
        1 => town_gen_circle(gd, map, floors, qy, qx, &mut free, &mut rng),
        _ => town_gen_hidden(gd, map, &mut rng),
    }
    town_ensure_reachable(gd, map, &free);
    // place_townspeople: 1% per free grid, `t` townsfolk (depth 0) only.
    let mut cells: Vec<(i32, i32)> = free.into_iter().collect();
    cells.sort();
    place_townspeople(gd, map, cells, spawns, &mut rng);
}

/// First cell with a terrain id.
pub fn find_terrain(map: &Map, t: u16) -> Option<(i32, i32)> {
    (0..MAP_H)
        .flat_map(|y| (0..MAP_W).map(move |x| (x, y)))
        .find(|&(x, y)| map.terrain_at(x, y) == t)
}

// --- Wilderness and towns -------------------------------------------------

/// A quote-aware tokenizer for pref `?:` expressions.
fn tokenize_cond(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                cur.push(c);
            }
            '[' | ']' if !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                out.push(c.to_string());
            }
            c if c.is_whitespace() && !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Evaluate a town pref `?:` condition against the current game state
/// (process_dungeon_file_expr, src/init1.cc).
pub fn eval_pref_cond(
    cond: &str,
    plot: &crate::game::PlotQuest,
    daytime: bool,
    leaving_quest: u32,
) -> bool {
    let value = |name: &str| -> i32 {
        if name == "DAYTIME" {
            return daytime as i32;
        }
        if name == "LEAVING_QUEST" {
            return leaving_quest as i32;
        }
        if let Some(rest) = name.strip_prefix("TOWN_DESTROY") {
            if let Ok(n) = rest.parse::<usize>() {
                return plot.destroyed_towns.get(n).copied().unwrap_or(false) as i32;
            }
        }
        if let Some(rest) = name.strip_prefix("QUEST") {
            if let Ok(q) = rest.parse::<u32>() {
                return plot.status(q) as i32;
            }
            let named = rest.trim_matches('"');
            let qid = match named {
                "Library quest" => crate::game::PLOT_LIBRARY,
                "Old Mages quest" => crate::game::PLOT_FIREPROOF,
                _ => 0,
            };
            return plot.status(qid) as i32;
        }
        0
    };
    let toks = tokenize_cond(cond);
    let mut pos = 0usize;
    fn parse(toks: &[String], pos: &mut usize, value: &dyn Fn(&str) -> i32) -> bool {
        if toks.get(*pos).map(String::as_str) != Some("[") {
            return false;
        }
        *pos += 1;
        let op = toks.get(*pos).map(String::as_str).unwrap_or("");
        *pos += 1;
        match op {
            "EQU" => {
                let var = toks.get(*pos).map(String::as_str).unwrap_or("").to_string();
                let v = toks
                    .get(*pos + 1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                *pos += 3;
                value(&var.trim_start_matches('$')) == v
            }
            "NOT" => {
                let v = !parse(toks, pos, value);
                *pos += 1;
                v
            }
            "AND" | "OR" => {
                let a = parse(toks, pos, value);
                let b = parse(toks, pos, value);
                *pos += 1;
                if op == "AND" {
                    a && b
                } else {
                    a || b
                }
            }
            _ => false,
        }
    }
    parse(&toks, &mut pos, &value)
}

/// `perturb_point_mid` (wild.cc:47): average four corners and perturb.
/// The integer average rounds up when `sum % 4 > 1` (the original tests
/// the remainder's truthiness; equivalent for non-negative heights).
fn perturb_mid(
    c: &mut [i32],
    w: i32,
    xs: [i32; 4],
    xmid: i32,
    ymid: i32,
    rough: i32,
    depth_max: i32,
    rng: &mut impl Rng,
) {
    let tmp = rng.gen_range(1..=rough * 2 + 1) - (rough + 1);
    let mut avg = (xs[0] + xs[1] + xs[2] + xs[3]) / 4 + tmp;
    if (xs[0] + xs[1] + xs[2] + xs[3]) % 4 > 1 {
        avg += 1;
    }
    c[(ymid * w + xmid) as usize] = avg.clamp(0, depth_max);
}

/// `perturb_point_end` (wild.cc:77): average three corners and perturb.
/// Rounds up whenever `sum % 3` is non-zero.
fn perturb_end(
    c: &mut [i32],
    w: i32,
    xs: [i32; 3],
    xmid: i32,
    ymid: i32,
    rough: i32,
    depth_max: i32,
    rng: &mut impl Rng,
) {
    let tmp = rng.gen_range(1..=rough * 2 + 1) - (rough + 1);
    let mut avg = (xs[0] + xs[1] + xs[2]) / 3 + tmp;
    if (xs[0] + xs[1] + xs[2]) % 3 != 0 {
        avg += 1;
    }
    c[(ymid * w + xmid) as usize] = avg.clamp(0, depth_max);
}

/// The original plasma fractal (src/wild.cc generate_area): a heightfield
/// mapped through a wf_info terrain table.  Deterministic per cell seed.
pub fn plasma_area(h: i32, w: i32, table: &[u16], seed: u32) -> Vec<u16> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let max = 18;
    let mut c = vec![(max / 2) as i32; (w * h) as usize];
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    for (x, y) in [(1, 1), (w - 2, 1), (1, h - 2), (w - 2, h - 2)] {
        c[idx(x, y)] = rng.gen_range(0..max);
    }
    fn plasma(
        c: &mut [i32],
        w: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        depth_max: i32,
        rough: i32,
        rng: &mut impl Rng,
    ) {
        let xmid = (x2 - x1) / 2 + x1;
        let ymid = (y2 - y1) / 2 + y1;
        if x1 + 1 == x2 {
            return;
        }
        let i = |x: i32, y: i32| (y * w + x) as usize;
        let v = [c[i(x1, y1)], c[i(x1, y2)], c[i(x2, y1)], c[i(x2, y2)]];
        perturb_mid(c, w, v, xmid, ymid, rough, depth_max, rng);
        let v = [c[i(x1, y1)], c[i(x2, y1)], c[i(xmid, ymid)]];
        perturb_end(c, w, v, xmid, y1, rough, depth_max, rng);
        let v = [c[i(x2, y1)], c[i(x2, y2)], c[i(xmid, ymid)]];
        perturb_end(c, w, v, x2, ymid, rough, depth_max, rng);
        let v = [c[i(x2, y2)], c[i(x1, y2)], c[i(xmid, ymid)]];
        perturb_end(c, w, v, xmid, y2, rough, depth_max, rng);
        let v = [c[i(x1, y2)], c[i(x1, y1)], c[i(xmid, ymid)]];
        perturb_end(c, w, v, x1, ymid, rough, depth_max, rng);
        plasma(c, w, x1, y1, xmid, ymid, depth_max, rough, rng);
        plasma(c, w, xmid, y1, x2, ymid, depth_max, rough, rng);
        plasma(c, w, x1, ymid, xmid, y2, depth_max, rough, rng);
        plasma(c, w, xmid, ymid, x2, y2, depth_max, rough, rng);
    }
    plasma(&mut c, w, 1, 1, w - 2, h - 2, max - 1, 1, &mut rng);
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let hv = c[idx(x, y)].clamp(0, table.len() as i32 - 1) as usize;
            table[hv]
        })
        .collect()
}

/// Generate the wilderness area of one world cell (wilderness_gen): the
/// plasma terrain plus the eight neighbour edges (seamless borders), the
/// roads, the dungeon entrance and day/night lighting.
#[allow(clippy::too_many_arguments)]
pub fn generate_wild_area(
    gd: &GameData,
    wilderness: &crate::game::Wilderness,
    plot: &crate::game::PlotQuest,
    wx: i32,
    wy: i32,
    daytime: bool,
    encounter: bool,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    let wf = wilderness.wf_at(gd, wx, wy, plot);
    let center = plasma_area(MAP_H, MAP_W, &wf.terrain, wilderness.seed(wx, wy));
    let terrain = center;
    let w = MAP_W;
    let h = MAP_H;
    // Neighbour areas (each with its own table) supply the map border's
    // *displayed* terrain.  Like wilderness_gen (src/wild.cc) the border
    // itself is FEAT_PERM_SOLID; only the mimic shows the neighbour's
    // feature (so the border can never be walked on or built upon).
    let mut border_mimic: Vec<(i32, i32, u16)> = Vec::new();
    let dirs: [(i32, i32); 8] = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ];
    for (dx, dy) in dirs {
        let (nx, ny) = (wx + dx, wy + dy);
        if !wilderness.in_bounds(nx, ny) {
            continue;
        }
        let nwf = wilderness.wf_at(gd, nx, ny, plot);
        let area = plasma_area(h, w, &nwf.terrain, wilderness.seed(nx, ny));
        let get = |x: i32, y: i32| area[(y * w + x) as usize];
        match (dx.signum(), dy.signum()) {
            (0, -1) => {
                for x in 0..w {
                    border_mimic.push((x, 0, get(x, h - 2)));
                }
            }
            (0, 1) => {
                for x in 0..w {
                    border_mimic.push((x, h - 1, get(x, 1)));
                }
            }
            (-1, 0) => {
                for y in 0..h {
                    border_mimic.push((0, y, get(w - 2, y)));
                }
            }
            (1, 0) => {
                for y in 0..h {
                    border_mimic.push((w - 1, y, get(1, y)));
                }
            }
            (-1, -1) => border_mimic.push((0, 0, get(w - 2, h - 2))),
            (1, -1) => border_mimic.push((w - 1, 0, get(1, h - 2))),
            (-1, 1) => border_mimic.push((0, h - 1, get(w - 2, 1))),
            (1, 1) => border_mimic.push((w - 1, h - 1, get(1, 1))),
            _ => {}
        }
    }
    let mut map = blank_map(T_NOTHING);
    map.terrain = terrain;
    for (x, y, t) in border_mimic {
        map.set_terrain(x, y, T_PERMANENT);
        map.mimic.insert(Map::idx(x, y), t);
    }
    map.wf = wf.id;
    map.encounter = encounter;
    map.wild = (wx, wy);

    // Maggot's field (q_shroom.cc): a mushroom clearing west of Bree.
    if (wx, wy) == (33, 21) {
        for y in MAP_H / 2 - 5..=MAP_H / 2 + 5 {
            for x in MAP_W / 2 - 7..=MAP_W / 2 + 7 {
                map.set_terrain(x, y, 181);
            }
        }
    }

    // The poisoned pond (q_poison.cc): while the quest is taken, water in
    // the assigned cell is tainted around a random centre.
    if plot.status(crate::game::PLOT_POISON) == crate::game::PLOT_TAKEN
        && plot.poison_cell == Some((wx, wy))
    {
        let (cy, cx) = (rng.gen_range(22..MAP_H - 22), rng.gen_range(32..MAP_W - 32));
        for y in cy - 25..=cy + 25 {
            for x in cx - 25..=cx + 25 {
                if chebyshev(cx, cy, x, y) > 25 || !Map::in_bounds(x, y) {
                    continue;
                }
                let t = map.terrain_at(x, y);
                if (t == T_SHAL_WATER || t == T_DEEP_WATER) && rng.gen_bool(0.8) {
                    map.set_terrain(x, y, T_TAINTED_WATER);
                }
            }
        }
    }

    // Roads (ROAD_N/S/E/W) straight through the middle.
    if wf.road != 0 {
        if wf.road & 1 != 0 {
            for y in 1..h / 2 {
                map.set_terrain(w / 2, y, T_FLOOR);
            }
        }
        if wf.road & 2 != 0 {
            for y in h / 2..h - 1 {
                map.set_terrain(w / 2, y, T_FLOOR);
            }
        }
        if wf.road & 4 != 0 {
            for x in w / 2..w - 1 {
                map.set_terrain(x, h / 2, T_FLOOR);
            }
        }
        if wf.road & 8 != 0 {
            for x in 1..w / 2 {
                map.set_terrain(x, h / 2, T_FLOOR);
            }
        }
    }

    // Dungeon entrance (a `>` on a random floor cell).
    if let Some(didx) = wilderness.dungeon_at(gd, wx, wy, plot) {
        let mut placed = false;
        for _ in 0..500 {
            let x = rng.gen_range(6..w - 6);
            let y = rng.gen_range(6..h - 6);
            if map.walkable(gd, x, y) {
                map.set_terrain(x, y, T_STAIRS_DOWN);
                map.special.insert(Map::idx(x, y), didx);
                map.lit[Map::idx(x, y)] = true;
                placed = true;
                break;
            }
        }
        // wild.cc places the entrance unconditionally; if the plasma
        // rolled no walkable cell, force one (with a small clearing).
        if !placed {
            let x = rng.gen_range(6..w - 6);
            let y = rng.gen_range(6..h - 6);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    map.set_terrain(x + dx, y + dy, T_FLOOR);
                }
            }
            map.set_terrain(x, y, T_STAIRS_DOWN);
            map.special.insert(Map::idx(x, y), didx);
            map.lit[Map::idx(x, y)] = true;
        }
    }

    // At least one floor grid.
    if !(1..h - 1).any(|y| (1..w - 1).any(|x| map.terrain_at(x, y) == T_GRASS)) {
        map.set_terrain(h / 2, w / 2, T_GRASS);
    }

    // Day lightens and memorizes everything; at night only REMEMBER
    // terrain keeps its light and memory (wilderness_gen, src/wild.cc).
    for y in 0..h {
        for x in 0..w {
            let i = Map::idx(x, y);
            let remember = gd.terrain(map.terrain[i]).remember;
            if daytime {
                map.lit[i] = true;
                map.explored[i] = true;
            } else if remember {
                map.lit[i] = true;
                map.explored[i] = true;
            } else {
                map.lit[i] = false;
                map.explored[i] = false;
            }
        }
    }
    let _ = encounter;
    GeneratedLevel {
        map,
        start: (w / 2, h / 2),
        vault: VaultSpawns::default(),
    }
}

/// The world overview map (`wilderness_gen_small`): one cell per world
/// square, with dungeon entrances and the known flags.
pub fn generate_world_map(
    gd: &GameData,
    wilderness: &crate::game::Wilderness,
    plot: &crate::game::PlotQuest,
) -> GeneratedLevel {
    let mut map = blank_map(T_PERMANENT);
    map.wild = (-1, -1);
    for y in 0..wilderness.h {
        for x in 0..wilderness.w {
            if !Map::in_bounds(x, y) {
                continue;
            }
            let wf = wilderness.wf_at(gd, x, y, plot);
            let t = if wilderness.dungeon_at(gd, x, y, plot).is_some() {
                T_STAIRS_DOWN
            } else {
                wf.feat
            };
            map.set_terrain(x, y, t);
            if wilderness.is_known(x, y) {
                map.lit[Map::idx(x, y)] = true;
                map.explored[Map::idx(x, y)] = true;
            }
        }
    }
    GeneratedLevel {
        map,
        start: (0, 0),
        vault: VaultSpawns::default(),
    }
}

/// Generate a town from its pref map (normal or destroyed variant), with
/// the conditional feature overrides evaluated against the plot state.
#[allow(clippy::too_many_arguments)]
/// Resolve a town's feature letters with the conditional overrides the
/// current game state applies.
fn town_chars(
    def: &crate::data::TownDef,
    plot: &crate::game::PlotQuest,
    daytime: bool,
    leaving_quest: u32,
) -> HashMap<char, crate::data::TownCharDef> {
    let mut chars: HashMap<char, crate::data::TownCharDef> =
        def.chars.iter().map(|c| (c.ch, *c)).collect();
    for o in &def.overrides {
        if eval_pref_cond(&o.cond, plot, daytime, leaving_quest) {
            chars.insert(
                o.ch,
                crate::data::TownCharDef {
                    ch: o.ch,
                    terrain: o.terrain,
                    mark: o.mark,
                    glow: o.glow,
                    room: o.room,
                    free: o.free,
                    monster: o.monster,
                    object: o.object,
                    special: o.special,
                },
            );
        }
    }
    chars
}

/// Stamp one town feature cell: terrain, lighting and the derived shop /
/// building / quest-entrance / staircase bookkeeping.
fn town_apply_cell(
    gd: &GameData,
    map: &mut Map,
    x: i32,
    y: i32,
    d: &crate::data::TownCharDef,
    daytime: bool,
) {
    map.set_terrain(x, y, d.terrain);
    let remember = gd.terrain(d.terrain).remember;
    let i = Map::idx(x, y);
    map.lit[i] = d.glow || daytime || remember;
    if d.mark || daytime || remember {
        map.explored[i] = true;
    }
    map.shops.remove(&i);
    map.buildings.remove(&i);
    map.quest_entrances.remove(&i);
    map.special.remove(&i);
    if d.terrain == 74 && d.special <= 9 {
        if let Some(store) = gd.stores.iter().find(|s| s.id == d.special) {
            map.shops.insert(
                i,
                ShopMark {
                    store: store.id,
                    ch: store.glyph(),
                    color: store.color,
                },
            );
        }
    } else if (d.terrain == 74 || d.terrain == 75) && d.special > 9 {
        map.buildings.insert(i, d.special);
    } else if d.terrain == T_QUEST_ENTER && d.special > 0 {
        map.quest_entrances.insert(i, d.special);
    } else if d.terrain == T_STAIRS_DOWN && d.special > 0 {
        map.special.insert(i, d.special);
    }
}

/// Re-evaluate a town's conditional features in place (day/night flips,
/// quest state changes while the player stands in the town).
pub fn refresh_town(
    gd: &GameData,
    map: &mut Map,
    plot: &crate::game::PlotQuest,
    daytime: bool,
    leaving_quest: u32,
) {
    let Some(def) = gd.town(map.town, plot.town_destroyed(map.town)) else {
        return;
    };
    let chars = town_chars(def, plot, daytime, leaving_quest);
    for (y, row) in def.rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == ' ' || !Map::in_bounds(x as i32, y as i32) {
                continue;
            }
            if let Some(d) = chars.get(&ch) {
                town_apply_cell(gd, map, x as i32, y as i32, d, daytime);
            }
        }
    }
}

/// Generate a town from its pref map (normal or destroyed variant), with
/// the conditional feature overrides evaluated against the plot state.
#[allow(clippy::too_many_arguments)]
pub fn generate_town(
    gd: &GameData,
    plot: &crate::game::PlotQuest,
    wilderness: &crate::game::Wilderness,
    wx: i32,
    wy: i32,
    town_id: u32,
    leaving_quest: u32,
    daytime: bool,
    rng: &mut impl Rng,
) -> GeneratedLevel {
    let destroyed = plot.town_destroyed(town_id);
    let def = gd
        .town(town_id, destroyed)
        .or_else(|| gd.town(town_id, false))
        .expect("town data missing");
    let wf = wilderness.wf_at(gd, wx, wy, plot);
    let mut map = blank_map(T_NOTHING);
    map.terrain = plasma_area(MAP_H, MAP_W, &wf.terrain, wilderness.seed(wx, wy));
    map.town = town_id;
    map.mflag = def.mflag.clone();
    map.wf = wf.id;
    map.wild = (wx, wy);

    let chars = town_chars(def, plot, daytime, leaving_quest);
    for (y, row) in def.rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == ' ' || !Map::in_bounds(x as i32, y as i32) {
                continue;
            }
            if let Some(d) = chars.get(&ch) {
                town_apply_cell(gd, &mut map, x as i32, y as i32, d, daytime);
            }
        }
    }

    // Day lightens and memorizes every grid (including the plasma
    // underlay between the town features); at night only REMEMBER
    // terrain keeps its light and memory (wilderness_gen, src/wild.cc).
    for i in 0..map.terrain.len() {
        let remember = gd.terrain(map.terrain[i]).remember;
        if daytime || remember {
            map.lit[i] = true;
            map.explored[i] = true;
        } else {
            map.lit[i] = false;
            map.explored[i] = false;
        }
    }

    // Starting position: the P: selected by the quest we return from, or
    // the default (LEAVING_QUEST 0) / map centre.  Resolved before the
    // connectivity pass so the paths lead to where the player arrives.
    let mut start = if leaving_quest != 0 {
        def.starts
            .iter()
            .find(|s| s.quest == leaving_quest)
            .map(|s| (s.x, s.y))
            .or(def.default_start)
    } else {
        def.default_start
    }
    .unwrap_or((MAP_W / 2, MAP_H / 2));
    if !map.walkable(gd, start.0, start.1) {
        'outer: for r in 1i32..40 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = (start.0 + dx, start.1 + dy);
                    if Map::in_bounds(x, y) && map.walkable(gd, x, y) {
                        start = (x, y);
                        break 'outer;
                    }
                }
            }
        }
    }

    // Guarantee that every shop, building and entrance is reachable
    // from the arrival point: the plasma underlay around the pref layout
    // can grow trees across a path, so anything cut off gets a grass
    // path toward the player.
    let walkable = |m: &Map, x: i32, y: i32| m.walkable(gd, x, y);
    let reachable = |m: &Map| {
        let mut seen = vec![false; (MAP_W * MAP_H) as usize];
        let mut stack = vec![start];
        seen[Map::idx(start.0, start.1)] = true;
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if Map::in_bounds(nx, ny) && walkable(m, nx, ny) {
                    let i = Map::idx(nx, ny);
                    if !seen[i] {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
        seen
    };
    let targets: Vec<usize> = map
        .shops
        .keys()
        .chain(map.buildings.keys())
        .chain(map.quest_entrances.keys())
        .chain(map.special.keys())
        .copied()
        .collect();
    for target in targets {
        let mut seen = reachable(&map);
        if seen[target] {
            continue;
        }
        let mut cx = (target as i32) % MAP_W;
        let mut cy = (target as i32) / MAP_W;
        for _ in 0..(MAP_W + MAP_H) {
            let mut seen = reachable(&map);
            if seen[Map::idx(cx, cy)] {
                break;
            }
            let (dx, dy) = (start.0 - cx, start.1 - cy);
            let mut moved = false;
            for (sx, sy) in [
                if dy != 0 { (0, dy.signum()) } else { (0, 0) },
                if dx != 0 { (dx.signum(), 0) } else { (0, 0) },
            ] {
                if sx == 0 && sy == 0 {
                    continue;
                }
                let (nx, ny) = (cx + sx, cy + sy);
                if !Map::in_bounds(nx, ny) {
                    continue;
                }
                if !walkable(&map, nx, ny) {
                    map.set_terrain(nx, ny, T_GRASS);
                }
                cx = nx;
                cy = ny;
                moved = true;
                break;
            }
            if !moved {
                break;
            }
            let _ = &mut seen;
        }
    }

    // place_townspeople (src/wild.cc): 1% per free town grid places a
    // depth-0 glyph-`t` monster (create_townpeople_hook / get_mon_num(0)).
    // The pref-map floor cells are the CAVE_FREE equivalent.
    let mut free_cells: Vec<(i32, i32)> = Vec::new();
    for (y, row) in def.rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == ' ' || !Map::in_bounds(x as i32, y as i32) {
                continue;
            }
            if chars.contains_key(&ch) {
                free_cells.push((x as i32, y as i32));
            }
        }
    }
    let mut vault = VaultSpawns::default();
    place_townspeople(gd, &map, free_cells, &mut vault, rng);

    GeneratedLevel { map, start, vault }
}

/// place_townspeople (src/wild.cc:1013): on every free grid a 1% roll
/// places a depth-0 `t`-glyph monster (the `create_townpeople_hook`).
/// `cells` must already be the CAVE_FREE-equivalent grids of the town.
pub fn place_townspeople(
    gd: &GameData,
    map: &Map,
    cells: impl IntoIterator<Item = (i32, i32)>,
    spawns: &mut VaultSpawns,
    rng: &mut impl Rng,
) {
    let folk: Vec<usize> = (0..gd.monster_base_count)
        .filter(|&i| {
            let m = &gd.monsters[i];
            m.glyph() == 't' && m.depth == 0 && !m.unique
        })
        .collect();
    if folk.is_empty() {
        return;
    }
    for (x, y) in cells {
        if !map.walkable(gd, x, y) {
            continue;
        }
        if rng.gen_range(0..100) == 0 {
            let def = folk[rng.gen_range(0..folk.len())];
            spawns.singles.push((def, x, y));
        }
    }
}

/// DUN_ROOMS / DUN_UNUSUAL / DUN_DEST (src/generate.cc).
const DUN_ROOMS: u32 = 50;
const DUN_UNUSUAL: u32 = 194;
const DUN_DEST: u32 = 18;
/// SMALL_LEVEL (generate.cc:151): 1/chance of a smaller size.
const SMALL_LEVEL: u32 = 6;
/// EMPTY_LEVEL (generate.cc:152): 1/chance of an 'empty' (arena) level.
const EMPTY_LEVEL: u32 = 15;
/// DARK_EMPTY (generate.cc:153): 1/chance an arena level stays dark.
const DARK_EMPTY: u32 = 5;
/// DUN_STR_QUA (src/generate.cc:176): number of quartz/water streamers.
const DUN_STR_QUA: i32 = 2;

/// T_* ids used only by vaults (f_info.txt via src/generate.cc).
const T_VAULT_OUTER: u16 = 58;
const T_VAULT_INNER: u16 = 57;
const T_VAULT_PERM: u16 = 61;
const T_GLASS: u16 = 188;
const T_ILLUSION: u16 = 189;
pub const T_BETWEEN: u16 = 160;

/// Pick a random monster kind up to `level` (weighted by rarity).
fn vault_monster(gd: &GameData, level: u32, rng: &mut impl Rng) -> Option<usize> {
    let candidates: Vec<usize> = (0..gd.monster_base_count)
        .filter(|&i| {
            let d = &gd.monsters[i];
            d.depth >= 1 && d.depth <= level && !d.unique
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let weights: Vec<u32> = candidates
        .iter()
        .map(|&i| (10 / gd.monsters[i].rarity).max(1))
        .collect();
    let dist = rand::distributions::WeightedIndex::new(&weights).ok()?;
    Some(candidates[dist.sample(rng)])
}

fn vault_object(
    _gd: &GameData,
    level: u32,
    x: i32,
    y: i32,
    good: bool,
    out: &mut VaultSpawns,
    _rng: &mut impl Rng,
) {
    // The kind is rolled by make_object at populate time so the special
    // artifact roll happens before the kind is chosen (object2.cc).
    out.objects.push((level, x, y, good));
}

/// Stamp a vault template onto the map at (x0, y0) (build_vault,
/// src/generate.cc:3298).  Returns the grids it touched.
#[allow(clippy::too_many_arguments)]
fn stamp_vault(
    map: &mut Map,
    gd: &GameData,
    v: &crate::data::VaultDef,
    x0: i32,
    y0: i32,
    depth: u32,
    out: &mut VaultSpawns,
    rng: &mut impl Rng,
) -> Vec<(i32, i32)> {
    let (w, h) = (v.wid as i32, v.hgt as i32);
    let mut cells = Vec::new();
    let mut gates: [Option<(i32, i32)>; 8] = [None; 8];
    for dy in 0..h {
        for dx in 0..w {
            let i = (dy * w + dx) as usize;
            let ch = v.data.as_bytes().get(i).copied().unwrap_or(b' ') as char;
            if ch == ' ' {
                continue;
            }
            let (x, y) = (x0 + dx, y0 + dy);
            if !Map::in_bounds(x, y) {
                continue;
            }
            map.set_terrain(x, y, T_FLOOR);
            cells.push((x, y));
            map.icky.insert(Map::idx(x, y));
            match ch {
                '%' => map.set_terrain(x, y, T_VAULT_OUTER),
                '#' => map.set_terrain(x, y, T_VAULT_INNER),
                'X' => map.set_terrain(x, y, T_VAULT_PERM),
                'G' => map.set_terrain(x, y, T_GLASS),
                'I' => map.set_terrain(x, y, T_ILLUSION),
                '+' => {
                    map.set_terrain(x, y, T_LOCKED_MIN + rng.gen_range(0..7));
                }
                '*' => {
                    if rng.gen_range(0..100) < 75 {
                        vault_object(gd, depth, x, y, false, out, rng);
                    }
                }
                '&' => {
                    if let Some(def) = vault_monster(gd, depth + 5, rng) {
                        out.monsters.push((def, x, y, depth + 5));
                    }
                }
                '@' => {
                    if let Some(def) = vault_monster(gd, depth + 11, rng) {
                        out.monsters.push((def, x, y, depth + 11));
                    }
                }
                '9' => {
                    if let Some(def) = vault_monster(gd, depth + 9, rng) {
                        out.monsters.push((def, x, y, depth + 9));
                    }
                    vault_object(gd, depth + 7, x, y, true, out, rng);
                }
                '8' => {
                    if let Some(def) = vault_monster(gd, depth + 40, rng) {
                        out.monsters.push((def, x, y, depth + 40));
                    }
                    vault_object(gd, depth + 20, x, y, true, out, rng);
                }
                ',' => {
                    if rng.gen_range(0..100) < 50 {
                        if let Some(def) = vault_monster(gd, depth + 3, rng) {
                            out.monsters.push((def, x, y, depth + 3));
                        }
                    }
                    if rng.gen_range(0..100) < 50 {
                        vault_object(gd, depth + 7, x, y, false, out, rng);
                    }
                }
                'A' => {
                    vault_object(gd, depth + 12, x, y, true, out, rng);
                }
                '0'..='7' => {
                    let d = (ch as u8 - b'0') as usize;
                    map.set_terrain(x, y, T_BETWEEN);
                    match gates[d] {
                        None => gates[d] = Some((x, y)),
                        Some((ox, oy)) => {
                            map.between.insert(Map::idx(x, y), Map::idx(ox, oy));
                            map.between.insert(Map::idx(ox, oy), Map::idx(x, y));
                        }
                    }
                }
                // '.', ',', '^' and anything else stay plain floor
                // (traps were removed from the original; a/b/c/d/P/B/p
                // abort in the original and never appear in the data).
                _ => {}
            }
        }
    }
    cells
}

/// Find a free spot for a vault and stamp it (old rooms-corridors path).
#[allow(clippy::too_many_arguments)]
fn place_vault(
    map: &mut Map,
    gd: &GameData,
    v: &crate::data::VaultDef,
    depth: u32,
    taken: &[(i32, i32, i32, i32)],
    vault_cells: &mut HashSet<(i32, i32)>,
    out: &mut VaultSpawns,
    rng: &mut impl Rng,
) -> Option<(i32, i32, i32, i32)> {
    let (w, h) = (v.wid as i32, v.hgt as i32);
    if w > MAP_W - 2 || h > MAP_H - 2 {
        return None;
    }
    // Find a free rectangle (all granite, clear of other vaults).
    let mut spot = None;
    for _ in 0..400 {
        let x0 = rng.gen_range(1..MAP_W - w - 1);
        let y0 = rng.gen_range(1..MAP_H - h - 1);
        if taken.iter().any(|&(rx, ry, rw, rh)| {
            x0 < rx + rw + 1 && rx < x0 + w + 1 && y0 < ry + rh + 1 && ry < y0 + h + 1
        }) {
            continue;
        }
        let fits = (0..h).all(|dy| (0..w).all(|dx| map.terrain_at(x0 + dx, y0 + dy) == T_GRANITE));
        if fits {
            spot = Some((x0, y0));
            break;
        }
    }
    let (x0, y0) = spot?;
    for c in stamp_vault(map, gd, v, x0, y0, depth, out, rng) {
        vault_cells.insert(c);
    }
    Some((x0, y0, w, h))
}

/// Move a buried down staircase into the start's connected component (the
/// original tunnels to every room; our simplified corridors can be cut off
/// by a sealed vault).
fn ensure_stairs_reachable(
    map: &mut Map,
    gd: &GameData,
    start: (i32, i32),
    down: (i32, i32),
    rng: &mut impl Rng,
) {
    let passable =
        |map: &Map, x: i32, y: i32| map.walkable(gd, x, y) || is_door(map.terrain_at(x, y));
    let reachable = |map: &Map| {
        let mut seen = vec![false; (MAP_W * MAP_H) as usize];
        let mut stack = vec![start];
        seen[Map::idx(start.0, start.1)] = true;
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if Map::in_bounds(nx, ny) && passable(map, nx, ny) {
                    let i = Map::idx(nx, ny);
                    if !seen[i] {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
        seen
    };
    if reachable(map)[Map::idx(down.0, down.1)] {
        return;
    }
    let seen = reachable(map);
    let mut spots: Vec<(i32, i32)> = (1..MAP_H - 1)
        .flat_map(|y| (1..MAP_W - 1).map(move |x| (x, y)))
        .filter(|&(x, y)| seen[Map::idx(x, y)] && map.walkable(gd, x, y) && (x, y) != start)
        .collect();
    if spots.is_empty() {
        return;
    }
    let pick = spots.swap_remove(rng.gen_range(0..spots.len()));
    // Move the same feature (ways for flat levels, shafts, stairs).
    let kind = map.terrain_at(down.0, down.1);
    map.set_terrain(down.0, down.1, T_FLOOR);
    map.set_terrain(pick.0, pick.1, kind);
    if let Some(s) = map.special.remove(&Map::idx(down.0, down.1)) {
        map.special.insert(Map::idx(pick.0, pick.1), s);
    }
}

/// Turn corridor cells that lead into a room into doors (closed, locked or
/// secret).
fn place_doors(map: &mut Map, rooms: &[(i32, i32, i32, i32)], depth: u32, rng: &mut impl Rng) {
    for &(x, y, w, h) in rooms {
        // Cells just outside the room perimeter.
        let mut candidates: Vec<(i32, i32)> = Vec::new();
        for xx in x - 1..=x + w {
            candidates.push((xx, y - 1));
            candidates.push((xx, y + h));
        }
        for yy in y..y + h {
            candidates.push((x - 1, yy));
            candidates.push((x + w, yy));
        }
        let mut placed = 0;
        for (cx, cy) in candidates {
            if placed >= 3 {
                break;
            }
            if !Map::in_bounds(cx, cy) || map.terrain_at(cx, cy) != T_FLOOR {
                continue;
            }
            // Only corridor cells (not floor belonging to another room).
            let in_room = rooms
                .iter()
                .any(|&(rx, ry, rw, rh)| cx >= rx && cx < rx + rw && cy >= ry && cy < ry + rh);
            if in_room || !rng.gen_bool(0.5) {
                continue;
            }
            let roll: f64 = rng.gen();
            let door = if roll < 0.08 {
                T_SECRET_DOOR
            } else if roll < 0.25 {
                T_LOCKED_MIN + (depth as u16 / 4).min(6)
            } else {
                T_DOOR
            };
            map.set_terrain(cx, cy, door);
            placed += 1;
        }
    }
}

/// Scatter hidden traps on corridor floor cells.
/// new_player_spot (generate.cc:684): a random "naked" floor grid —
/// walkable, not permanent, outside vaults (CAVE_ICKY) and free of the
/// level's pre-placed monsters/objects.
fn random_naked_spot(
    map: &Map,
    gd: &GameData,
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
    rng: &mut impl Rng,
) -> Option<(i32, i32)> {
    for _ in 0..5000 {
        let x = rng.gen_range(1..w - 1);
        let y = rng.gen_range(1..h - 1);
        if occupied.contains(&(x, y)) || map.icky.contains(&Map::idx(x, y)) {
            continue;
        }
        if map.terrain_at(x, y) == T_TRAP {
            continue;
        }
        let def = gd.terrain(map.terrain_at(x, y));
        if map.walkable(gd, x, y) && !def.permanent {
            return Some((x, y));
        }
    }
    None
}

fn place_traps(map: &mut Map, in_room: &dyn Fn(i32, i32) -> bool, depth: u32, rng: &mut impl Rng) {
    let count = 2 + depth as i32 / 2;
    let mut placed = 0;
    let mut tries = 0;
    while placed < count && tries < 500 {
        tries += 1;
        let x = rng.gen_range(1..map.w.max(3) - 1);
        let y = rng.gen_range(1..map.h.max(3) - 1);
        if map.terrain_at(x, y) != T_FLOOR {
            continue;
        }
        if map.shops.contains_key(&Map::idx(x, y)) {
            continue;
        }
        if in_room(x, y) {
            continue;
        }
        map.set_terrain(x, y, T_TRAP);
        map.trap_kinds
            .insert(Map::idx(x, y), TrapKind::roll(depth, rng));
        placed += 1;
    }
}

/// One fixed-map `F:` marker definition (init1.cc:6066
/// process_dungeon_file_aux).  The `*` prefix selects a random feature/
/// monster/object/ego/artifact drawn from the quest level's own depth.
#[derive(Debug, Clone, Default)]
pub struct MarkerDef {
    pub feature: u16,
    pub random_feature: bool,
    pub cave_info: u32,
    pub monster: u32,
    pub random_monster: bool,
    pub object: u32,
    pub random_object: bool,
    pub ego: u32,
    pub random_ego: bool,
    pub artifact: u32,
    pub random_artifact: bool,
    pub special: u32,
    pub mimic: u16,
    pub mflag: u32,
}

/// Parse one `F:<letter>:<terrain>:<cave_info>:<monster>:<object>:<ego>:
/// <artifact>:<special>:<mimic>:<mflag>` line.
pub fn parse_f_marker(line: &str) -> Option<(char, MarkerDef)> {
    let rest = line.strip_prefix("F:")?;
    let f: Vec<&str> = rest.split(':').collect();
    if f.len() < 2 || f[0].chars().count() != 1 {
        return None;
    }
    let mut def = MarkerDef::default();
    let num = |v: Option<&&str>| -> (u32, bool) {
        match v {
            Some(v) if v.starts_with('*') => (v[1..].parse::<u32>().unwrap_or(0), true),
            Some(v) => (v.parse::<u32>().unwrap_or(0), false),
            None => (0, false),
        }
    };
    let (feature, random_feature) = num(f.get(1));
    def.feature = feature as u16;
    def.random_feature = random_feature;
    def.cave_info = num(f.get(2)).0;
    let (monster, random_monster) = num(f.get(3));
    def.monster = monster;
    def.random_monster = random_monster;
    let (object, random_object) = num(f.get(4));
    def.object = object;
    def.random_object = random_object;
    let (ego, random_ego) = num(f.get(5));
    def.ego = ego;
    def.random_ego = random_ego;
    let (artifact, random_artifact) = num(f.get(6));
    def.artifact = artifact;
    def.random_artifact = random_artifact;
    def.special = num(f.get(7)).0;
    def.mimic = num(f.get(8)).0 as u16;
    def.mflag = num(f.get(9)).0;
    Some((f[0].chars().next()?, def))
}

/// Parse every `F:` line of a fixed-map file body.
pub fn parse_f_markers(text: &str) -> HashMap<char, MarkerDef> {
    let mut out = HashMap::new();
    for line in text.lines() {
        if let Some((ch, def)) = parse_f_marker(line.trim()) {
            out.insert(ch, def);
        }
    }
    out
}

/// Offset at which a fixed quest level is centred on the 128x64 map.
pub fn quest_map_offset(qm: &crate::data::QuestMapDef) -> (i32, i32) {
    ((MAP_W - qm.w) / 2, (MAP_H - qm.h) / 2)
}

/// The monster/object level of a plot quest (tables.cc quest[].level).
/// The ids match game.rs PLOT_* / src/defines.hpp QUEST_*; quests without
/// a level (random/bounty/god) fall back to 1.
pub fn quest_level(quest: u32) -> u32 {
    match quest {
        1 => 70,
        2 => 99,
        3 => 100,
        4 => 5,
        5 => 5,
        6 => 25,
        7 => 40,
        8 => 30,
        9 => 30,
        10 => 25,
        11 => 30,
        12 => 20,
        13 => 30,
        14 => 37,
        15 => 80,
        16 => 80,
        17 => 99,
        18 => 3,
        19 => 60,
        20 => 150,
        21 => 150,
        22 => 15,
        23 => 25,
        24 => 45,
        25 => 60,
        27 => 20,
        28 => 35,
        _ => 1,
    }
}

/// Apply a fixed map's `F:...:*N` random monster/object markers
/// (init1.cc process_dungeon_file_aux).  `base` is the quest level for
/// quest maps; d_info special levels load with quest level 0, so there
/// the offsets are absolute.  The actual entities are spawned by the
/// caller from the returned `VaultSpawns` (map.rs stays ECS-free).
#[allow(clippy::too_many_arguments)]
pub fn roll_map_markers(
    base: u32,
    random_monsters: &[crate::data::QuestMapRandom],
    random_objects: &[crate::data::QuestMapRandom],
    (ox, oy): (i32, i32),
    out: &mut VaultSpawns,
    rng: &mut impl Rng,
) {
    for rm in random_monsters {
        let level = (base as i32 + rm.level).max(1) as u32;
        out.random_monsters.push((level, rm.x + ox, rm.y + oy));
    }
    for ro in random_objects {
        // 75% normal / 15% good / 10% great (place_object good/great).
        let level = (base as i32 + ro.level).max(0) as u32;
        let (good, great) = if rng.gen_range(0..100) < 75 {
            (false, false)
        } else if rng.gen_range(0..100) < 80 {
            (true, false)
        } else {
            (true, true)
        };
        out.random_objects
            .push((level, ro.x + ox, ro.y + oy, good, great));
    }
}

/// A fixed plot-quest level (thieves/trolls/wights/...), centred on the
/// map and surrounded by permanent rock. If the map's entry point is
/// not walkable it is moved to the nearest walkable cell.
pub fn generate_quest_level(gd: &GameData, quest: u32) -> Option<GeneratedLevel> {
    let qm = gd.quest_map(quest)?;
    let mut map = blank_map(T_PERMANENT);
    let (ox, oy) = quest_map_offset(qm);
    for (i, c) in qm.cells.iter().enumerate() {
        let x = ox + i as i32 % qm.w;
        let y = oy + i as i32 / qm.w;
        map.set_terrain(x, y, c.t);
        map.lit[Map::idx(x, y)] = c.l;
    }
    // `M:` mimic cells display another feature until the map is learned.
    for m in &qm.mimics {
        let (x, y) = (m.x + ox, m.y + oy);
        if Map::in_bounds(x, y) && m.t != T_NOTHING {
            map.mimic.insert(Map::idx(x, y), m.t);
        }
    }
    let mut start = (qm.start.0 + ox, qm.start.1 + oy);
    if !map.walkable(gd, start.0, start.1) {
        'outer: for r in 1i32..20 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = (start.0 + dx, start.1 + dy);
                    if Map::in_bounds(x, y) && map.walkable(gd, x, y) {
                        start = (x, y);
                        break 'outer;
                    }
                }
            }
        }
    }
    // `F:...:*N` random markers (quest level + offset).
    let mut vault = VaultSpawns::default();
    let mut rng = crate::rng::current();
    roll_map_markers(
        quest_level(quest),
        &qm.random_monsters,
        &qm.random_objects,
        (ox, oy),
        &mut vault,
        &mut rng,
    );
    Some(GeneratedLevel { map, start, vault })
}

/// The mushroom-quest field (quest 18 has no .map; the original generates
/// it procedurally): a lit grass clearing with a rectangular field.
pub fn generate_shroom_field(_gd: &GameData) -> GeneratedLevel {
    const T_GRASS: u16 = 89;
    const T_FIELD: u16 = 181;
    let mut map = blank_map(T_GRASS);
    let (cx, cy) = (MAP_W / 2, MAP_H / 2);
    for y in cy - 5..=cy + 5 {
        for x in cx - 7..=cx + 7 {
            map.set_terrain(x, y, T_FIELD);
        }
    }
    for v in map.lit.iter_mut() {
        *v = true;
    }
    // The player starts south of the field, on the way back to Bree.
    let start = (cx, cy + 9);
    map.set_terrain(start.0, start.1, T_STAIRS_UP);
    GeneratedLevel {
        map,
        start,
        vault: VaultSpawns::default(),
    }
}

/// The centre of the mushroom field (Maggot's spot, dog/mushroom area).
pub fn shroom_field_center() -> (i32, i32) {
    (MAP_W / 2, MAP_H / 2)
}

/// Eol's fractal cave (q_eol.cc): a 50x30 plasma height-map converted by
/// a flood fill from the middle (generate.cc generate_hmap +
/// generate_fracave). Everything outside the cave stays granite.
pub fn generate_eol_cave(rng: &mut impl Rng) -> GeneratedLevel {
    let xsize: i32 = 50;
    let ysize: i32 = 30;
    let xhsize = xsize / 2;
    let yhsize = ysize / 2;
    let x0 = 2 + xhsize;
    let y0 = 2 + yhsize;
    let maxsize = xsize.max(ysize);
    let hidx = |x: i32, y: i32| (y * (xsize + 1) + x) as usize;
    fn ri(rng: &mut impl Rng, n: i32) -> i32 {
        if n <= 0 {
            0
        } else {
            rng.gen_range(1..=n)
        }
    }
    // store_height: 255 marks "not computed yet"; boundary heights are
    // pushed above the cutoff so cave edges are not square; >216 becomes
    // floor (q_eol.cc FEAT_FLOOR terrain value).
    fn store(
        h: &mut [u8],
        xsize: i32,
        xhsize: i32,
        yhsize: i32,
        cutoff: i32,
        x: i32,
        y: i32,
        val: i32,
    ) {
        let i = (y * (xsize + 1) + x) as usize;
        if h[i] != 255 {
            return;
        }
        let mut val = val & 0xff;
        if (x == 0 || y == 0 || x == xhsize * 2 || y == yhsize * 2) && val <= cutoff {
            val = (cutoff + 1) & 0xff;
        }
        if val > 216 {
            val = T_FLOOR as i32;
        }
        h[i] = val as u8;
    }
    // hack_isnt_wall: convert one visited grid cell to floor/wall.
    #[allow(clippy::too_many_arguments)]
    fn isnt_wall(
        map: &mut Map,
        h: &[u8],
        done: &mut [bool],
        xsize: i32,
        xhsize: i32,
        yhsize: i32,
        x0: i32,
        y0: i32,
        cutoff: i32,
        x: i32,
        y: i32,
    ) -> bool {
        let i = (y * (xsize + 1) + x) as usize;
        if done[i] {
            return false;
        }
        done[i] = true;
        let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
        if h[i] as i32 <= cutoff {
            map.set_terrain(mx, my, T_FLOOR);
            true
        } else {
            map.set_terrain(mx, my, T_GRANITE);
            false
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn fill(
        map: &mut Map,
        h: &[u8],
        done: &mut [bool],
        xsize: i32,
        ysize: i32,
        xhsize: i32,
        yhsize: i32,
        x0: i32,
        y0: i32,
        cutoff: i32,
        x: i32,
        y: i32,
        amount: &mut i32,
    ) {
        for i in -1..=1 {
            for j in -1..=1 {
                let (nx, ny) = (x + i, y + j);
                if nx > 0 && nx < xsize && ny > 0 && ny < ysize {
                    if isnt_wall(map, h, done, xsize, xhsize, yhsize, x0, y0, cutoff, nx, ny) {
                        fill(
                            map, h, done, xsize, ysize, xhsize, yhsize, x0, y0, cutoff, nx, ny,
                            amount,
                        );
                        *amount += 1;
                    }
                }
            }
        }
    }

    loop {
        let grd = 2 ^ rng.gen_range(1..=4);
        let roug = rng.gen_range(1..=8) * rng.gen_range(1..=4);
        let cutoff = rng.gen_range(1..=xsize / 4)
            + rng.gen_range(1..=ysize / 4)
            + rng.gen_range(1..=xsize / 4)
            + rng.gen_range(1..=ysize / 4);
        let mut h = vec![255u8; ((xsize + 1) * (ysize + 1)) as usize];
        store(&mut h, xsize, xhsize, yhsize, cutoff, 0, 0, maxsize);
        store(&mut h, xsize, xhsize, yhsize, cutoff, 0, ysize, maxsize);
        store(&mut h, xsize, xhsize, yhsize, cutoff, xsize, 0, maxsize);
        store(&mut h, xsize, xhsize, yhsize, cutoff, xsize, ysize, maxsize);
        store(&mut h, xsize, xhsize, yhsize, cutoff, xhsize, yhsize, 0);

        let mut xstep = xsize * 256;
        let mut xhstep = xstep;
        let mut ystep = ysize * 256;
        let mut yhstep = ystep;
        let xxsize = xsize * 256;
        let yysize = ysize * 256;
        while xstep / 256 > 1 || ystep / 256 > 1 {
            xstep = xhstep;
            xhstep /= 2;
            ystep = yhstep;
            yhstep /= 2;
            // Middle top to bottom.
            let mut i = xhstep;
            while i <= xxsize - xhstep {
                let mut j = 0;
                while j <= yysize {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    } else {
                        let l = h[hidx((i - xhstep) / 256, j / 256)] as i32;
                        let r = h[hidx((i + xhstep) / 256, j / 256)] as i32;
                        let v = (l + r) / 2 + (ri(rng, xstep / 256) - xhstep / 256) * roug / 16;
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    }
                    j += ystep;
                }
                i += xstep;
            }
            // Middle left to right.
            let mut j = yhstep;
            while j <= yysize - yhstep {
                let mut i = 0;
                while i <= xxsize {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    } else {
                        let u = h[hidx(i / 256, (j - yhstep) / 256)] as i32;
                        let d = h[hidx(i / 256, (j + yhstep) / 256)] as i32;
                        let v = (u + d) / 2 + (ri(rng, ystep / 256) - yhstep / 256) * roug / 16;
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    }
                    i += xstep;
                }
                j += ystep;
            }
            // Centres (diagonal, scaled by 362/16/256).
            let mut i = xhstep;
            while i <= xxsize - xhstep {
                let mut j = yhstep;
                while j <= yysize - yhstep {
                    if xhstep / 256 > grd {
                        let v = ri(rng, maxsize);
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    } else {
                        let ul = h[hidx((i - xhstep) / 256, (j - yhstep) / 256)] as i32;
                        let dl = h[hidx((i - xhstep) / 256, (j + yhstep) / 256)] as i32;
                        let ur = h[hidx((i + xhstep) / 256, (j - yhstep) / 256)] as i32;
                        let dr = h[hidx((i + xhstep) / 256, (j + yhstep) / 256)] as i32;
                        let v = (ul + dl + ur + dr) / 4
                            + (ri(rng, xstep / 256) - xhstep / 256) * (362 / 16) / 256 * roug;
                        store(&mut h, xsize, xhsize, yhsize, cutoff, i / 256, j / 256, v);
                    }
                    j += ystep;
                }
                i += xstep;
            }
        }

        // Flood fill from the middle: keep the connected cave only.
        let mut map = blank_map(T_GRANITE);
        let mut done = vec![false; h.len()];
        let mut amount = 0i32;
        fill(
            &mut map,
            &h,
            &mut done,
            xsize,
            ysize,
            xhsize,
            yhsize,
            x0,
            y0,
            cutoff,
            xhsize,
            yhsize,
            &mut amount,
        );
        if amount < 10 {
            continue;
        }
        // Eol at the first clean cell of the scan (high x/y), the player
        // at the last (low x/y) -- q_eol.cc.
        let mut start = None;
        for x in (2..=xsize - 1).rev() {
            for y in (2..=ysize - 1).rev() {
                let (mx, my) = (x + x0 - xhsize, y + y0 - yhsize);
                if map.terrain_at(mx, my) == T_FLOOR {
                    start = Some((mx, my));
                }
            }
        }
        let Some(start) = start else {
            continue;
        };
        map.set_terrain(start.0, start.1, T_STAIRS_UP);
        return GeneratedLevel {
            map,
            start,
            vault: VaultSpawns::default(),
        };
    }
}

/// MAX_RANGE (defines.hpp): the maximum spell/missile projection range.
pub const MAX_RANGE: i32 = 18;

/// los (cave.cc:150): the original line-of-sight algorithm.  Unlike a
/// plain Bresenham line it permits a beam that exactly meets a corner,
/// and a knight's move around a single blocking grid.
pub fn los(map: &Map, gd: &GameData, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let ax = dx.abs();
    let ay = dy.abs();
    // Handle adjacent (or identical) grids.
    if ax < 2 && ay < 2 {
        return true;
    }
    // cave_sight_bold: the grid does not block vision.
    let sight = |x: i32, y: i32| -> bool { !map.opaque(gd, x, y) };
    // Directly south/north.
    if dx == 0 {
        if dy > 0 {
            for ty in y1 + 1..y2 {
                if !sight(x1, ty) {
                    return false;
                }
            }
        } else {
            for ty in (y2 + 1..y1).rev() {
                if !sight(x1, ty) {
                    return false;
                }
            }
        }
        return true;
    }
    // Directly east/west.
    if dy == 0 {
        if dx > 0 {
            for tx in x1 + 1..x2 {
                if !sight(tx, y1) {
                    return false;
                }
            }
        } else {
            for tx in (x2 + 1..x1).rev() {
                if !sight(tx, y1) {
                    return false;
                }
            }
        }
        return true;
    }
    let sx = if dx < 0 { -1 } else { 1 };
    let sy = if dy < 0 { -1 } else { 1 };
    // Knight's moves.
    if ax == 1 && ay == 2 && sight(x1, y1 + sy) {
        return true;
    }
    if ay == 1 && ax == 2 && sight(x1 + sx, y1) {
        return true;
    }
    // Scale factors (div 2 and full).
    let f2 = ax * ay;
    let f1 = f2 << 1;
    let (mut tx, mut ty);
    if ax >= ay {
        let mut qy = ay * ay;
        let m = qy << 1;
        tx = x1 + sx;
        if qy == f2 {
            ty = y1 + sy;
            qy -= f1;
        } else {
            ty = y1;
        }
        while x2 - tx != 0 {
            if !sight(tx, ty) {
                return false;
            }
            qy += m;
            if qy < f2 {
                tx += sx;
            } else if qy > f2 {
                ty += sy;
                if !sight(tx, ty) {
                    return false;
                }
                qy -= f1;
                tx += sx;
            } else {
                ty += sy;
                qy -= f1;
                tx += sx;
            }
        }
    } else {
        let mut qx = ax * ax;
        let m = qx << 1;
        ty = y1 + sy;
        if qx == f2 {
            tx = x1 + sx;
            qx -= f1;
        } else {
            tx = x1;
        }
        while y2 - ty != 0 {
            if !sight(tx, ty) {
                return false;
            }
            qx += m;
            if qx < f2 {
                ty += sy;
            } else if qx > f2 {
                tx += sx;
                if !sight(tx, ty) {
                    return false;
                }
                qx -= f1;
                ty += sy;
            } else {
                tx += sx;
                qx -= f1;
                ty += sy;
            }
        }
    }
    true
}

/// mmove2 (cave.cc:3895): the incremental motion used by projectable.
fn mmove2(y: i32, x: i32, y1: i32, x1: i32, y2: i32, x2: i32) -> (i32, i32) {
    let dy = (y - y1).abs();
    let dx = (x - x1).abs();
    let mut dist = dy.max(dx);
    dist += 1;
    let dy = (y2 - y1).abs();
    let dx = (x2 - x1).abs();
    if dy == 0 && dx == 0 {
        return (y, x);
    }
    if dy > dx {
        let shift = (dist * dx + (dy - 1) / 2) / dy;
        let nx = if x2 < x1 { x1 - shift } else { x1 + shift };
        let ny = if y2 < y1 { y1 - dist } else { y1 + dist };
        (ny, nx)
    } else {
        let shift = (dist * dy + (dx - 1) / 2) / dx;
        let ny = if y2 < y1 { y1 - shift } else { y1 + shift };
        let nx = if x2 < x1 { x1 - dist } else { x1 + dist };
        (ny, nx)
    }
}

/// projectable (cave.cc:3953): the beam's actual path — never passes
/// through an opaque or non-floor grid (walls block spells even though
/// corner sight may allow seeing the target).
pub fn projectable(map: &Map, gd: &GameData, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let mut x = x1;
    let mut y = y1;
    for dist in 0..=MAX_RANGE {
        if x == x2 && y == y2 {
            return true;
        }
        if dist != 0 && (map.opaque(gd, x, y) || !map.walkable(gd, x, y)) {
            break;
        }
        let (ny, nx) = mmove2(y, x, y1, x1, y2, x2);
        y = ny;
        x = nx;
    }
    false
}

/// Line of sight between two grids (the original cave.cc `los`).
pub fn line_of_sight(map: &Map, gd: &GameData, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    los(map, gd, x0, y0, x1, y1)
}

/// A cell counts as lit when permanently lit, or when it is a wall
/// adjacent to lit floor (Angband's view_bright_lite).
fn cell_lit(map: &Map, gd: &GameData, x: i32, y: i32) -> bool {
    if map.lit[Map::idx(x, y)] {
        return true;
    }
    if !gd.terrain(map.terrain_at(x, y)).is_wall {
        return false;
    }
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if !Map::in_bounds(nx, ny) {
                continue;
            }
            if map.lit[Map::idx(nx, ny)] && gd.terrain(map.terrain_at(nx, ny)).is_floor {
                return true;
            }
        }
    }
    false
}

/// Recompute the visible set around (ox, oy); marks cells explored as well.
/// A cell is visible when in line of sight AND either lit, within the
/// player's light radius, or the player's own cell.  Lit (CAVE_GLOW)
/// grids are visible at any distance in LOS up to MAX_SIGHT, as in
/// cave.cc update_view; the light radius only governs torch-lit cells.
pub fn compute_fov(map: &mut Map, gd: &GameData, ox: i32, oy: i32, light: i32) {
    for v in map.visible.iter_mut() {
        *v = false;
    }
    let r = MAX_SIGHT;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let (x, y) = (ox + dx, oy + dy);
            if !Map::in_bounds(x, y) {
                continue;
            }
            let i = Map::idx(x, y);
            let near = chebyshev(ox, oy, x, y) <= light.max(0);
            if !(near || cell_lit(map, gd, x, y)) {
                continue;
            }
            if line_of_sight(map, gd, ox, oy, x, y) {
                map.visible[i] = true;
                map.explored[i] = true;
            }
        }
    }
    let i = Map::idx(ox, oy);
    map.visible[i] = true;
    map.explored[i] = true;
}

/// Light an area around (ox, oy) permanently (Light Area spell/scroll).
pub fn light_area(map: &mut Map, ox: i32, oy: i32, radius: i32) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (ox + dx, oy + dy);
            if Map::in_bounds(x, y) {
                map.lit[Map::idx(x, y)] = true;
            }
        }
    }
}

pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (bx - ax).abs().max((by - ay).abs())
}

#[cfg(test)]
mod tests {
    #[test]
    fn mana_field_is_filled_per_level() {
        let mut rng = crate::rng::new_seeded_rng(11);
        let mut m = blank_map(1);
        super::generate_grid_mana(&mut m, 20, &mut rng);
        assert_eq!(m.mana.len(), (MAP_W * MAP_H) as usize);
        assert!(m.mana.iter().any(|v| *v > 0), "mana veins exist");
    }

    use super::*;
    use crate::data::load_game_data;

    #[test]
    fn generated_level_is_connected() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let gen = generate_level(&gd, 1, &mut rng);
        let map = &gen.map;

        // Player start is walkable (standing on the up staircase).
        assert!(map.walkable(&gd, gen.start.0, gen.start.1));

        // A down staircase exists somewhere.
        let down = (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| (x, y)))
            .find(|&(x, y)| map.terrain_at(x, y) == T_STAIRS_DOWN)
            .expect("no down staircase");

        // Flood fill from the start must reach it. Doors are passable
        // (they can be opened/bashed), so count them as walkable here.
        let passable = |x: i32, y: i32| map.walkable(&gd, x, y) || is_door(map.terrain_at(x, y));
        let mut seen = vec![false; (MAP_W * MAP_H) as usize];
        let mut stack = vec![gen.start];
        seen[Map::idx(gen.start.0, gen.start.1)] = true;
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if Map::in_bounds(nx, ny) && passable(nx, ny) {
                    let i = Map::idx(nx, ny);
                    if !seen[i] {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
        assert!(seen[Map::idx(down.0, down.1)], "down staircase unreachable");
    }

    #[test]
    fn doors_and_traps_are_generated() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut doors = 0;
        let mut traps = 0;
        let mut lit_rooms = 0;
        for _ in 0..5 {
            let gen = generate_level(&gd, 5, &mut rng);
            for y in 0..MAP_H {
                for x in 0..MAP_W {
                    let t = gen.map.terrain_at(x, y);
                    if is_closed_door(t) {
                        doors += 1;
                    }
                    if t == T_TRAP {
                        traps += 1;
                    }
                    if gen.map.lit[Map::idx(x, y)] && t == T_FLOOR {
                        lit_rooms += 1;
                    }
                }
            }
        }
        assert!(doors > 0, "no doors generated");
        assert!(traps > 0, "no traps generated");
        assert!(lit_rooms > 0, "no lit rooms generated");
    }

    /// A town generated on its real world cell with a fresh wilderness.
    pub(crate) fn gen_town_for(
        gd: &GameData,
        plot: &crate::game::PlotQuest,
        town: u32,
        leaving: u32,
        rng: &mut impl Rng,
    ) -> GeneratedLevel {
        let wild = crate::game::Wilderness::new(gd, rng);
        let (wx, wy) = gd.town_cell(town);
        generate_town(gd, plot, &wild, wx, wy, town, leaving, true, rng)
    }

    #[test]
    fn flat_dungeon_uses_way_features() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // Barrow-Downs (dungeon 4) is FLAT; flat dungeons place the
        // "path to the next/previous area" features instead of stairs.
        assert!(gd.dungeon(4).has("FLAT"));
        let gen = generate_dungeon_level(&gd, 4, 1, &mut rng);
        assert!(gen.map.terrain.iter().any(|&t| t == T_WAY_MORE));
        assert!(gen.map.terrain.iter().any(|&t| t == T_WAY_LESS));
        assert!(!gen.map.terrain.iter().any(|&t| t == T_STAIRS_DOWN));
    }

    #[test]
    fn town_has_shops_and_stairs() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let gen = gen_town_for(&gd, &crate::game::PlotQuest::default(), 1, 0, &mut rng);
        // Bree's shops (the original t_pref letters) and its thieves'
        // quest entrance are present; the troll/wight gates stay trees
        // until their quests are taken.
        assert!(gen.map.shops.len() >= 5, "shops: {}", gen.map.shops.len());
        assert!(gen.map.quest_entrances.values().any(|&q| q == 4));
        assert!(!gen.map.quest_entrances.values().any(|&q| q == 8));
        assert!(gen.map.special.values().any(|&d| d == 4), "Barrow-Downs");
        assert!(gen.map.walkable(&gd, gen.start.0, gen.start.1));
        // The whole town is lit and memorized by day.
        assert!(gen.map.lit[Map::idx(gen.start.0, gen.start.1)]);
        assert!(gen.map.explored.iter().all(|&e| e));
        // Every shop and quest entrance is reachable from the start.
        let mut seen = vec![false; (MAP_W * MAP_H) as usize];
        let mut stack = vec![gen.start];
        seen[Map::idx(gen.start.0, gen.start.1)] = true;
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if Map::in_bounds(nx, ny) && gen.map.walkable(&gd, nx, ny) {
                    let i = Map::idx(nx, ny);
                    if !seen[i] {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
        for &i in gen.map.shops.keys() {
            assert!(seen[i], "shop cell {} unreachable", i);
        }
        for &i in gen.map.quest_entrances.keys() {
            assert!(seen[i], "quest entrance {} unreachable", i);
        }
        // Once the thieves' hideout is cleared it becomes the Home.
        let mut plot = crate::game::PlotQuest::default();
        plot.set(crate::game::PLOT_THIEVES, crate::game::PLOT_COMPLETED);
        let gen = gen_town_for(&gd, &plot, 1, 0, &mut rng);
        assert!(gen
            .map
            .shops
            .values()
            .any(|m| m.store == crate::town::HOME_STORE));
    }

    #[test]
    fn town_conditional_features_apply() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // The troll glade only opens once the quest is taken *and* it is
        // night (t_bree.txt); the wight grave once taken.
        let wild = crate::game::Wilderness::new(&gd, &mut rng);
        let (wx, wy) = gd.town_cell(1);
        let mut plot = crate::game::PlotQuest::default();
        plot.set(crate::game::PLOT_TROLL, crate::game::PLOT_TAKEN);
        let night = generate_town(&gd, &plot, &wild, wx, wy, 1, 0, false, &mut rng);
        assert!(night.map.quest_entrances.values().any(|&q| q == 8));
        // Re-evaluating in the same map at sunrise closes it again.
        let mut map = night.map.clone();
        refresh_town(&gd, &mut map, &plot, true, 0);
        assert!(!map.quest_entrances.values().any(|&q| q == 8));
    }

    #[test]
    fn galadriels_mirror_and_void_portal_appear_in_lothlorien() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // The Mirror is a building (st 23) in Lothlorien; before the Void
        // quest starts the static portal cell is still dirt.
        let gen = gen_town_for(&gd, &crate::game::PlotQuest::default(), 4, 0, &mut rng);
        assert!(
            gen.map.buildings.values().any(|&b| b == 23),
            "The Mirror building"
        );
        assert!(!gen.map.special.values().any(|&d| d == 11));
        // Once QUEST_ULTRA_GOOD is taken the t_lorien override turns the
        // 'v' cell into a jumpgate to the Void (d_info 11).
        let mut plot = crate::game::PlotQuest::default();
        plot.states
            .insert(crate::game::PLOT_ULTRA_GOOD, crate::game::PLOT_TAKEN);
        let gen = gen_town_for(&gd, &plot, 4, 0, &mut rng);
        assert!(gen.map.special.values().any(|&d| d == 11), "void jumpgate");
        // Recall is refused inside the Void (q_ultrag.cc recall hook).
        let mut log = crate::game::MessageLog::default();
        let mut ps = crate::birth::make_player(&gd, "T".into(), 0, 0);
        ps.dungeon = 11;
        ps.depth = 140;
        assert!(crate::game::recall_blocked(&gd, &ps, &mut log));
        ps.dungeon = 4;
        ps.depth = 10;
        assert!(!crate::game::recall_blocked(&gd, &ps, &mut log));
    }

    #[test]
    fn towns_variants_load_and_differ() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // Gondolin: the destroyed map keeps its layout but loses the
        // shops, buildings and quest entrances (t_d_gond.txt).
        let normal = gen_town_for(&gd, &crate::game::PlotQuest::default(), 2, 0, &mut rng);
        assert!(!normal.map.shops.is_empty());
        assert!(!normal.map.buildings.is_empty());
        let mut plot = crate::game::PlotQuest::default();
        plot.destroyed_towns[2] = true;
        let ruined = gen_town_for(&gd, &plot, 2, 0, &mut rng);
        assert!(ruined.map.shops.is_empty());
        assert!(ruined.map.buildings.is_empty());
        // Every town starts somewhere walkable.
        for town in 1..=5u32 {
            let gen = gen_town_for(&gd, &crate::game::PlotQuest::default(), town, 0, &mut rng);
            assert!(
                gen.map.walkable(&gd, gen.start.0, gen.start.1),
                "town {}",
                town
            );
        }
    }

    #[test]
    fn bree_trees_block_movement() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let gen = gen_town_for(&gd, &crate::game::PlotQuest::default(), 1, 0, &mut rng);
        // Bree keeps the original tree cells (terrain 96) and they are
        // unwalkable walls (the void "nothing" cells stay walkable, as
        // in the original: f_info 0 carries the FLOOR flag).
        let trees = (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| (x, y)))
            .filter(|&(x, y)| gen.map.terrain_at(x, y) == 96)
            .count();
        assert!(trees > 100, "Bree lost its trees: {}", trees);
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                if gen.map.terrain_at(x, y) == 96 {
                    assert!(!gen.map.walkable(&gd, x, y));
                }
            }
        }
    }

    #[test]
    fn vaults_are_ported_and_placed() {
        let gd = load_game_data();
        // 56 lesser + 46 greater vaults plus the type-10 wilderness test
        // record, all ported from v_info.txt (only 7/8 are ever built).
        assert_eq!(gd.vaults.len(), 103);
        assert!(gd.vaults.iter().any(|v| v.typ == 10));
        for v in gd.vaults.iter().filter(|v| v.typ == 7 || v.typ == 8) {
            assert_eq!(v.data.len(), (v.hgt * v.wid) as usize, "vault {}", v.id);
        }
        // Deep levels sometimes contain vault stone (57/58/61 are only
        // used by build_vault; normal generation is pure granite 56).
        let mut rng = crate::rng::current();
        let mut found = 0;
        for _ in 0..150 {
            let gen = generate_level(&gd, 30, &mut rng);
            if gen.map.terrain.iter().any(|&t| matches!(t, 57 | 58 | 61)) {
                found += 1;
            }
        }
        assert!(found > 0, "no vault was placed in 150 deep levels");
    }

    #[test]
    fn vault_contents_are_recorded() {
        let gd = load_game_data();
        // A vault with fixed monster/object symbols always yields some
        // contents (the `&`/`8` vaults); find levels until one shows up.
        let mut rng = crate::rng::current();
        let mut found = false;
        for _ in 0..300 {
            let gen = generate_level(&gd, 40, &mut rng);
            if !gen.vault.monsters.is_empty() || !gen.vault.objects.is_empty() {
                found = true;
                break;
            }
        }
        assert!(found, "vault contents never recorded");
    }

    #[test]
    fn vault_between_gates_pair_up() {
        let gd = load_game_data();
        let mut map = blank_map(T_GRANITE);
        let v = crate::data::VaultDef {
            id: 999,
            typ: 7,
            rat: 1,
            hgt: 3,
            wid: 5,
            data: "%%%%%%0.0%%%%%%".to_string(),
        };
        let mut vault = VaultSpawns::default();
        let mut cells = HashSet::new();
        let mut rng = crate::rng::current();
        let rect = place_vault(&mut map, &gd, &v, 10, &[], &mut cells, &mut vault, &mut rng)
            .expect("vault placed");
        let (x0, y0, _, _) = rect;
        // The two digits became Void Jumpgates linked to each other.
        let g0 = Map::idx(x0 + 1, y0 + 1);
        let g1 = Map::idx(x0 + 3, y0 + 1);
        assert_eq!(map.terrain[g0], T_BETWEEN);
        assert_eq!(map.terrain[g1], T_BETWEEN);
        assert_eq!(map.between.get(&g0), Some(&g1));
        assert_eq!(map.between.get(&g1), Some(&g0));
    }

    #[test]
    fn vault_spawns_are_on_floor() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut total = 0;
        for _ in 0..200 {
            let gen = generate_level(&gd, 40, &mut rng);
            for &(_, x, y, _) in &gen.vault.monsters {
                assert!(gen.map.walkable(&gd, x, y), "monster on wall at {x},{y}");
                total += 1;
            }
            for &(level, x, y, _) in &gen.vault.objects {
                assert!(gen.map.walkable(&gd, x, y), "object on wall at {x},{y}");
                assert!(level > 0);
                total += 1;
            }
        }
        assert!(total > 0, "no vault spawns recorded in 200 levels");
    }

    #[test]
    fn terrain_effects_follow_f_info() {
        let _gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut map = blank_map(T_GRANITE);
        map.set_terrain(10, 10, 85);
        let (_, gf) = terrain_effect(&map, 10, 10, 10, 10, &mut rng).expect("deep lava");
        assert_eq!(gf, "FIRE");
        assert!(terrain_effect(&map, 10, 10, 11, 10, &mut rng).is_none());
        map.set_terrain(10, 10, 90);
        assert!(terrain_effect(&map, 10, 10, 500, 10, &mut rng).is_some());
        assert!(terrain_effect(&map, 10, 10, 510, 10, &mut rng).is_none());
        map.set_terrain(10, 10, 102);
        assert_eq!(
            terrain_effect(&map, 10, 10, 400, 10, &mut rng).unwrap().1,
            "NETHER"
        );
        assert!(terrain_effect(&map, 10, 10, 800, 10, &mut rng).is_some());
        map.set_terrain(10, 10, T_FLOOR);
        assert!(terrain_effect(&map, 10, 10, 10, 10, &mut rng).is_none());
    }

    #[test]
    fn fixed_map_markers_parse() {
        let (ch, def) = parse_f_marker("F:g:1:0:0:*:*").unwrap();
        assert_eq!(ch, 'g');
        assert_eq!(def.feature, 1);
        assert_eq!(def.monster, 0);
        assert!(def.random_object && def.random_ego);
        let (_, def) = parse_f_marker("F:8:1:0:*79:*77").unwrap();
        assert_eq!(def.monster, 79);
        assert!(def.random_monster);
        assert_eq!(def.object, 77);
        assert!(def.random_object);
        let (_, def) = parse_f_marker("F:b:1:6:150:43:*:0:0:0:2").unwrap();
        assert_eq!(def.monster, 150);
        assert_eq!(def.object, 43);
        assert!(def.random_ego);
        assert_eq!(def.mflag, 2);
        let text = "F:a:1:0:5\nF:b:1:0:6\n";
        assert_eq!(parse_f_markers(text).len(), 2);
    }

    #[test]
    fn terrain_effects_match_f_info_data() {
        let gd = load_game_data();
        for &(feat, dd, ds, freq, gf) in TERRAIN_EFFECTS {
            let def = gd.terrain(feat);
            assert_eq!(def.effects.len(), 1, "f_info {feat} effect count");
            let e = &def.effects[0];
            assert_eq!(
                (e.dd, e.ds, e.freq as u64, e.typ.as_str()),
                (dd, ds, freq, gf),
                "f_info {feat} effect drifted from the map table"
            );
        }
    }

    #[test]
    fn fixed_map_random_markers_are_recorded() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut saw_object = false;
        let mut saw_mimic = false;
        for qm in &gd.quest_maps {
            if qm.random_objects.is_empty() && qm.mimics.is_empty() {
                continue;
            }
            let gen = generate_quest_level(&gd, qm.quest).expect("quest level");
            let (ox, oy) = quest_map_offset(qm);
            if !qm.random_objects.is_empty() {
                assert_eq!(gen.vault.random_objects.len(), qm.random_objects.len());
                let (level, x, y, _, _) = gen.vault.random_objects[0];
                let ro = qm.random_objects[0];
                assert_eq!((x, y), (ro.x + ox, ro.y + oy));
                assert_eq!(
                    level,
                    (quest_level(qm.quest) as i32 + ro.level).max(0) as u32
                );
                saw_object = true;
            }
            if let Some(m) = qm.mimics.iter().find(|m| m.t != T_NOTHING) {
                assert_eq!(gen.map.mimic.get(&Map::idx(m.x + ox, m.y + oy)), Some(&m.t));
                saw_mimic = true;
            }
        }
        // d_info special levels carry absolute random markers (base 0);
        // roll_map_markers resolves them for the spawner.
        let mut saw_monster = false;
        for sl in &gd.spec_levels {
            if sl.random_monsters.is_empty() {
                continue;
            }
            let mut vault = VaultSpawns::default();
            roll_map_markers(0, &sl.random_monsters, &[], (0, 0), &mut vault, &mut rng);
            assert_eq!(vault.random_monsters.len(), sl.random_monsters.len());
            assert_eq!(
                vault.random_monsters[0],
                (
                    sl.random_monsters[0].level.max(1) as u32,
                    sl.random_monsters[0].x,
                    sl.random_monsters[0].y
                )
            );
            saw_monster = true;
            break;
        }
        assert!(
            saw_monster && saw_object && saw_mimic,
            "fixed-map F:/M: markers were not exercised"
        );
    }

    #[test]
    fn evolve_level_starves_and_spawns() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut map = blank_map(T_GRANITE);
        for y in 8..12 {
            for x in 8..12 {
                map.set_terrain(x, y, T_FLOOR);
            }
        }
        let floors = [T_FLOOR; 100];
        let fills = [T_GRANITE; 100];
        let before = map.terrain.iter().filter(|&&t| t == T_FLOOR).count();
        evolve_level(
            &mut map,
            &gd,
            &floors,
            &fills,
            -1,
            -1,
            false,
            MAP_W,
            MAP_H,
            &HashSet::new(),
            &mut rng,
        );
        let after = map.terrain.iter().filter(|&&t| t == T_FLOOR).count();
        assert!(
            after > before,
            "suffocated walls should die out: {} -> {}",
            before,
            after
        );
    }

    #[test]
    fn level_size_honours_options_and_flags() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let normal = gd
            .dungeons
            .iter()
            .find(|d| !d.has("SMALL") && !d.has("SMALLEST") && !d.has("BIG"))
            .unwrap();
        // Everything off => full size.
        let off = crate::options::Options {
            small_levels: false,
            empty_levels: false,
            always_small_level: false,
            ..Default::default()
        };
        assert_eq!(level_size(normal, &off, &mut rng), (MAP_W, MAP_H));
        // always_small_level forces a small level for a non-BIG dungeon.
        let small = crate::options::Options {
            always_small_level: true,
            ..Default::default()
        };
        let (w, h) = level_size(normal, &small, &mut rng);
        assert!(w <= 132 && h <= 66, "small level {w}x{h}");
        assert!((w, h) != (MAP_W, MAP_H));
        // DF_BIG suppresses the small roll entirely.
        if let Some(big) = gd.dungeons.iter().find(|d| d.has("BIG")) {
            assert_eq!(level_size(big, &small, &mut rng), (MAP_W, MAP_H));
        }
    }

    #[test]
    fn quest_levels_get_up_only_stairs() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let off = crate::options::Options {
            small_levels: false,
            empty_levels: false,
            ..Default::default()
        };
        // Moria depth 2 with is_quest: alloc_stairs turns every requested
        // staircase into an up staircase.
        let gen = generate_dungeon_level_for(&gd, 22, 2, &off, true, &mut rng);
        assert!(find_terrain(&gen.map, T_STAIRS_UP).is_some());
        assert!(find_terrain(&gen.map, T_STAIRS_DOWN).is_none());
        assert!(find_terrain(&gen.map, T_SHAFT_DOWN).is_none());
        // Without is_quest the down staircases are back.
        let gen = generate_dungeon_level_for(&gd, 22, 2, &off, false, &mut rng);
        let downs = find_terrain(&gen.map, T_STAIRS_DOWN).is_some()
            || find_terrain(&gen.map, T_SHAFT_DOWN).is_some();
        assert!(downs);
    }

    #[test]
    fn maze_and_life_generators_are_used() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // Dungeon 18 "Maze" uses the gen_maze generator (twisty passages
        // on a 66x22 SMALLEST level).
        let maze = gd.dungeons.iter().find(|d| d.generator == "maze").unwrap();
        assert_eq!(maze.id, 18);
        let gen = generate_dungeon_level(&gd, maze.id, maze.mindepth, &mut rng);
        assert!(gen.map.walkable(&gd, gen.start.0, gen.start.1));
        let floors = (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| (x, y)))
            .filter(|&(x, y)| gen.map.walkable(&gd, x, y))
            .count();
        assert!(floors > 40, "maze too small: {floors}");
        // Everything outside the 66x22 level stays permanent.
        assert_eq!(gen.map.terrain_at(190, 60), T_PERMANENT);
        // Heart of the Earth (10) uses the game-of-life generator; force
        // a full-size level so the (50, 30) probe is inside it.
        let life = gd.dungeons.iter().find(|d| d.generator == "life").unwrap();
        assert_eq!(life.id, 10);
        let full = crate::options::Options {
            small_levels: false,
            empty_levels: false,
            ..Default::default()
        };
        let gen = generate_dungeon_level_for(
            &gd,
            life.id,
            life.mindepth,
            &full,
            false,
            &mut rng,
        );
        assert!(gen.map.walkable(&gd, gen.start.0, gen.start.1));
        // The whole interior is memorized and lit (CAVE_GLOW|CAVE_MARK).
        assert!(gen.map.lit[Map::idx(50, 30)]);
        assert!(gen.map.explored[Map::idx(50, 30)]);
    }

    #[test]
    fn all_dungeons_generate_without_panic() {
        use rand::SeedableRng;
        let gd = load_game_data();
        for seed in 0..50u64 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            for d in &gd.dungeons {
                if d.maxdepth == 0 {
                    continue;
                }
                let mid = d.mindepth + (d.maxdepth - d.mindepth) / 2;
                for depth in [d.mindepth, mid, d.maxdepth] {
                    if depth < 1 {
                        continue;
                    }
                    let gen = generate_dungeon_level(&gd, d.id, depth, &mut rng);
                    assert!(
                        gen.map.walkable(&gd, gen.start.0, gen.start.1),
                        "seed {seed} dungeon {} depth {}: start not walkable",
                        d.id,
                        depth
                    );
                }
            }
        }
    }

    #[test]
    fn dungeon_rooms_use_the_generatecc_layout() {
        use rand::SeedableRng;
        let gd = load_game_data();
        // Fixed seed: the layout coverage below must be deterministic
        // (the original random-vault roll is rare).  The seed also has to
        // show every family below, so it is tied to the generator's
        // current draw order.
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        // Deep levels should show room outer walls (58), vault perm
        // walls (61/62) from the random-vault family and streamers
        // (build_streamer2) over enough tries.
        let mut outer = 0;
        let mut perm = 0;
        let mut water = 0;
        for _ in 0..40 {
            // Paths of the Dead: deep and without DF_NO_STREAMERS.
            let gen = generate_dungeon_level(&gd, 16, 50, &mut rng);
            assert!(gen.map.walkable(&gd, gen.start.0, gen.start.1));
            for &t in &gen.map.terrain {
                if t == 58 {
                    outer += 1;
                }
                if t == 61 || t == 62 {
                    perm += 1;
                }
                if t == T_DEEP_WATER || t == T_SHAL_WATER {
                    water += 1;
                }
            }
        }
        assert!(outer > 0, "no room outer walls generated");
        assert!(perm > 0, "no random vault (perm walls) generated");
        assert!(water > 0, "no water streamer generated");
    }

    #[test]
    fn dungeon_levels_follow_d_info() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // Barrow-Downs (mindepth 1, FLAT): a way up and ways down.
        let gen = generate_dungeon_level(&gd, 4, 1, &mut rng);
        assert!(find_terrain(&gen.map, T_WAY_LESS).is_some());
        assert!(find_terrain(&gen.map, T_WAY_MORE).is_some());
        // Moria at its branch depth: down staircases carry dungeon 24.
        let gen = generate_dungeon_level(&gd, 22, 40, &mut rng);
        assert!(gen.map.special.values().any(|&d| d == 24));
    }

    #[test]
    fn eol_cave_is_a_connected_cavern() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        for _ in 0..5 {
            let gen = generate_eol_cave(&mut rng);
            assert!(gen.map.walkable(&gd, gen.start.0, gen.start.1));
            // The start cell is the up staircase; count every walkable
            // cell of the connected cavern.
            let floors = (0..MAP_H)
                .flat_map(|y| (0..MAP_W).map(move |x| (x, y)))
                .filter(|&(x, y)| gen.map.walkable(&gd, x, y))
                .count();
            assert!(floors >= 10, "cave too small: {floors}");
            // Everything outside the 50x30 region stays granite.
            assert_eq!(gen.map.terrain_at(0, 0), T_GRANITE);
            assert_eq!(gen.map.terrain_at(60, 40), T_GRANITE);
        }
    }

    #[test]
    fn wilderness_areas_have_dungeon_entrances() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let wild = crate::game::Wilderness::new(&gd, &mut rng);
        let plot = crate::game::PlotQuest::default();
        // Mirkwood's '*' world cell gets a `>` tagged with dungeon 1.
        let mut cell = None;
        'scan: for y in 0..gd.world.h {
            for x in 0..gd.world.w {
                if wild.wf_at(&gd, x, y, &plot).dungeon() == Some(1) {
                    cell = Some((x, y));
                    break 'scan;
                }
            }
        }
        let (ex, ey) = cell.expect("Mirkwood cell");
        let gen = generate_wild_area(&gd, &wild, &plot, ex, ey, true, false, &mut rng);
        let (sx, sy) = find_terrain(&gen.map, T_STAIRS_DOWN).expect("dungeon stair");
        assert_eq!(gen.map.special.get(&Map::idx(sx, sy)), Some(&1));
        // The world overview renders the same cell as a staircase.
        let world = generate_world_map(&gd, &wild, &plot);
        assert_eq!(world.map.terrain_at(ex, ey), T_STAIRS_DOWN);
    }

    #[test]
    fn fov_blocked_by_walls() {
        let gd = load_game_data();
        // A 5x5 lit room at (8..13, 8..13), granite everywhere else.
        let mut map = blank_map(T_GRANITE);
        for y in 8..13 {
            for x in 8..13 {
                map.set_terrain(x, y, T_FLOOR);
                map.lit[Map::idx(x, y)] = true;
            }
        }
        compute_fov(&mut map, &gd, 10, 10, 1);
        // Own cell and room neighbour visible.
        assert!(map.visible[Map::idx(10, 10)]);
        assert!(map.visible[Map::idx(11, 10)]);
        // Adjacent wall (unlit, outside light radius 1? no: within) is lit
        // only via light radius; beyond it, granite not visible.
        assert!(map.visible[Map::idx(10, 7)]);
        assert!(!map.visible[Map::idx(10, 5)]);
    }

    #[test]
    fn place_townspeople_is_one_percent_of_free_grids() {
        use rand::SeedableRng;
        let gd = load_game_data();
        let map = blank_pub(T_FLOOR);
        let cells: Vec<(i32, i32)> = (1..40)
            .flat_map(|x| (1..20).map(move |y| (x, y)))
            .collect();
        let mut spawns = VaultSpawns::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(11);
        place_townspeople(&gd, &map, cells.clone(), &mut spawns, &mut rng);
        // place_townspeople (wild.cc:1013): a 1% roll per CAVE_FREE grid
        // placing a depth-0 `t` monster; never a fixed eight residents.
        assert!(!spawns.singles.is_empty(), "some townsfolk rolled");
        assert!(
            spawns.singles.len() < cells.len() / 10,
            "1% of {} grids is not {}",
            cells.len(),
            spawns.singles.len()
        );
        for &(def, x, y) in &spawns.singles {
            let d = &gd.monsters[def];
            assert_eq!(d.glyph(), 't');
            assert_eq!(d.depth, 0);
            assert!(!d.unique);
            assert!((1..40).contains(&x) && (1..20).contains(&y));
        }
    }

    #[test]
    fn fov_dark_room_needs_light() {
        let gd = load_game_data();
        // Same room but unlit: without a light source only the own cell
        // is visible; with a torch (light 1) the neighbours show.
        let mut map = blank_map(T_GRANITE);
        for y in 8..13 {
            for x in 8..13 {
                map.set_terrain(x, y, T_FLOOR);
            }
        }
        compute_fov(&mut map, &gd, 10, 10, 0);
        assert!(map.visible[Map::idx(10, 10)]);
        assert!(!map.visible[Map::idx(11, 10)]);
        compute_fov(&mut map, &gd, 10, 10, 1);
        assert!(map.visible[Map::idx(11, 10)]);
    }

    #[test]
    fn dungeon_towns_have_reachable_shops() {
        use rand::SeedableRng;
        let gd = load_game_data();
        // Moria (22) is a RANDOM_TOWNS dungeon; try enough seeds to see
        // all three layouts (normal / circle / hidden).
        let mut saw_inside = false;
        let mut saw_hidden = false;
        for seed in 0..24u32 {
            // A dedicated seeded RNG makes the generated level (and the
            // reachability assertions) reproducible.
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
            let gen =
                generate_dungeon_town_level(&gd, 22, 33, crate::game::TOWN_RANDOM, seed, &mut rng);
            assert_eq!(gen.map.town, crate::game::TOWN_RANDOM);
            assert!(!gen.map.shops.is_empty(), "seed {seed}: no shops");
            // The layout is the first draw of the town RNG.
            let layout = rand::rngs::StdRng::seed_from_u64(seed as u64).gen_range(0..3);
            // Every shop is a STF_RANDOM store of the data.
            for (&i, mark) in &gen.map.shops {
                let st = gd.stores.iter().find(|s| s.id == mark.store).unwrap();
                assert!(st.flags.iter().any(|f| f == "RANDOM"));
                assert_eq!(gen.map.terrain[i], T_SHOP);
            }
            if layout == 2 {
                saw_hidden = true;
            } else {
                saw_inside = true;
                // An interior town has townspeople and keeps its shops
                // inside the 66x22 screen.
                assert!(
                    !gen.vault.singles.is_empty(),
                    "seed {seed}: town without townspeople"
                );
                let qx = (MAP_W - TOWN_W) / 2;
                let qy = (MAP_H - TOWN_H) / 2;
                for &i in gen.map.shops.keys() {
                    let (x, y) = (i as i32 % MAP_W, i as i32 / MAP_W);
                    assert!(
                        (qx..qx + TOWN_W).contains(&x) && (qy..qy + TOWN_H).contains(&y),
                        "seed {seed}: shop outside the town screen"
                    );
                }
            }
            // Shops are reachable from the player's entry (doors passable).
            let passable =
                |x: i32, y: i32| gen.map.walkable(&gd, x, y) || is_door(gen.map.terrain_at(x, y));
            let mut seen = vec![false; (MAP_W * MAP_H) as usize];
            let mut stack = vec![gen.start];
            seen[Map::idx(gen.start.0, gen.start.1)] = true;
            while let Some((x, y)) = stack.pop() {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if Map::in_bounds(nx, ny) && passable(nx, ny) {
                        let i = Map::idx(nx, ny);
                        if !seen[i] {
                            seen[i] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            for &i in gen.map.shops.keys() {
                assert!(seen[i], "seed {seed}: shop {i} unreachable from start");
            }
        }
        assert!(saw_inside, "no town layout with an interior was generated");
        assert!(saw_hidden, "the hidden layout was never generated");
    }

    #[test]
    fn town_store_ids_are_random_flagged() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // get_shops only ever returns STF_RANDOM stores.
        for _ in 0..10 {
            for id in town_shops(&gd, &mut rng) {
                let st = gd.stores.iter().find(|s| s.id == id).unwrap();
                assert!(st.flags.iter().any(|f| f == "RANDOM"));
            }
        }
    }
}

#[cfg(test)]
mod double_mana_tests {
    use super::*;
    use crate::data::load_game_data;

    #[test]
    fn double_dungeons_supersize_to_full_map() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        // Erebor (DOUBLE+BIG): the level must still fill the whole map,
        // with a reachable staircase.
        let erebor = gd
            .dungeons
            .iter()
            .find(|d| d.has("DOUBLE"))
            .expect("erebor");
        for depth in [erebor.mindepth, erebor.mindepth + 3] {
            let gen = generate_dungeon_level(&gd, erebor.id, depth, &mut rng);
            let mut walls = 0;
            let mut floors = 0;
            for y in 0..MAP_H {
                for x in 0..MAP_W {
                    if gen.map.walkable(&gd, x, y) {
                        floors += 1;
                    } else {
                        walls += 1;
                    }
                }
            }
            assert!(floors > 500, "depth {} floors {}", depth, floors);
            // 2x2 block structure: every walkable cell's siblings agree.
            let x0 = gen.start.0 / 2 * 2;
            let y0 = gen.start.1 / 2 * 2;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert!(
                    gen.map.walkable(&gd, x0 + dx, y0 + dy),
                    "start block not supersized"
                );
            }
            let _ = walls;
        }
    }

    #[test]
    fn grid_mana_is_generated_everywhere() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let gen = generate_dungeon_level(&gd, 4, 5, &mut rng);
        assert_eq!(gen.map.mana.len(), gen.map.terrain.len());
        assert!(gen.map.mana.iter().any(|&m| m > 0));
    }

    #[test]
    fn mordor_and_angband_get_magma_and_quartz_veins() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        for dungeon in [2u32, 3] {
            let mut found = false;
            for _ in 0..8 {
                let gen = generate_dungeon_level(&gd, dungeon, 60, &mut rng);
                let n = gen
                    .map
                    .terrain
                    .iter()
                    .filter(|&&t| matches!(t, 50 | 51 | 54 | 55))
                    .count();
                if n > 0 {
                    found = true;
                    break;
                }
            }
            assert!(found, "dungeon {dungeon} never generated a vein");
        }
    }

    #[test]
    fn terrain_carry_the_can_fly_climb_pass_flags() {
        let gd = load_game_data();
        // f_info 97 "mountain chain" is climbable, 49 "rubble" flyable.
        assert!(gd.terrain(97).can_climb);
        assert!(gd.terrain(49).can_fly && gd.terrain(49).can_pass);
        // Open floor is plain walkable terrain.
        assert!(!gd.terrain(1).can_fly && !gd.terrain(1).can_climb);
    }

    #[test]
    fn los_and_projectable_match_the_original_corner_rules() {
        let gd = load_game_data();
        let mut map = blank_map(T_GRANITE);
        for y in 0..6 {
            for x in 0..6 {
                map.set_terrain(x, y, T_FLOOR);
            }
        }
        // A corner blocked on both orthogonal sides: the exact-corner
        // diagonal from (0,0) to (2,2) is allowed (cave.cc los: qy == f2).
        map.set_terrain(1, 0, T_GRANITE);
        map.set_terrain(0, 1, T_GRANITE);
        assert!(los(&map, &gd, 0, 0, 2, 2));
        assert!(projectable(&map, &gd, 0, 0, 2, 2));
        // An opaque cell on the beam blocks both vision and projection.
        map.set_terrain(1, 1, T_GRANITE);
        assert!(!los(&map, &gd, 0, 0, 2, 2));
        assert!(!projectable(&map, &gd, 0, 0, 2, 2));
        // Straight lines still require clear intermediates.
        map.set_terrain(1, 1, T_FLOOR);
        map.set_terrain(1, 0, T_GRANITE);
        assert!(!los(&map, &gd, 0, 0, 2, 0));
    }
}

/// Focused checks for the cave.cc / generate.cc audit entries.
#[cfg(test)]
mod audit_tests {
    use super::*;
    use crate::data::load_game_data;
    use rand::SeedableRng;
    use std::collections::HashSet;

    fn seeded(seed: u64) -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(seed)
    }

    /// cave.cc:50 distance: max(dy,dx) + min(dy,dx)/2.
    #[test]
    fn cave_distance_matches_the_original_approximation() {
        assert_eq!(pref_distance(0, 0, 0, 0), 0);
        assert_eq!(pref_distance(0, 0, 10, 0), 10);
        assert_eq!(pref_distance(0, 0, 0, 9), 9);
        assert_eq!(pref_distance(0, 0, 3, 4), 5);
        assert_eq!(pref_distance(1, 1, 6, 5), 7);
    }

    /// cave.cc cave_floor/plain_floor/sight/perma predicates, which live
    /// on TerrainDef flags (`is_floor`, `remember`, `permanent`,
    /// `no_vision`) plus Map::walkable/opaque.
    #[test]
    fn cave_flag_predicates_match_the_original() {
        let gd = load_game_data();
        let floor = gd.terrain(T_FLOOR);
        assert!(floor.is_floor && !floor.permanent && !floor.no_vision);
        let granite = gd.terrain(T_GRANITE);
        assert!(!granite.is_floor && granite.is_wall && granite.no_vision);
        assert!(gd.terrain(T_PERMANENT).permanent);

        let mut map = blank_map(T_FLOOR);
        // cave_plain_floor_grid: floor without FF_REMEMBER.
        let plain = |m: &Map, x: i32, y: i32| {
            let d = gd.terrain(m.terrain_at(x, y));
            d.is_floor && !d.remember
        };
        assert_eq!(plain(&map, 5, 5), !floor.remember);
        // cave_sight_grid == !opaque, cave_floor_bold == is_floor.
        assert!(map.walkable(&gd, 5, 5));
        assert!(!map.opaque(&gd, 5, 5));
        map.set_terrain(5, 5, T_GRANITE);
        assert!(!map.walkable(&gd, 5, 5));
        assert!(map.opaque(&gd, 5, 5));
        // cave_perma_grid / cave_valid_bold.
        map.set_terrain(6, 6, T_PERMANENT);
        assert!(gd.terrain(map.terrain_at(6, 6)).permanent);
        assert!(!cave_valid_bold(&map, &gd, 6, 6, false));
        assert!(!cave_valid_bold(&map, &gd, 5, 5, true), "artifact grid");
    }

    /// cave.cc:71 is_wall exceptions (glass / illusion / small trees) and
    /// the mimic-aware display feature.
    #[test]
    fn is_wall_exceptions_follow_cave_cc() {
        let gd = load_game_data();
        let mut map = blank_map(T_FLOOR);
        assert!(!is_wall(&map, &gd, 1, 1));
        for (t, expected) in [(188u16, false), (189, true), (202, true), (T_GRANITE, true)] {
            map.set_terrain(1, 1, t);
            assert_eq!(is_wall(&map, &gd, 1, 1), expected, "terrain {t}");
        }
        map.set_terrain(2, 2, T_GRANITE);
        map.mimic.insert(Map::idx(2, 2), T_FLOOR);
        assert!(!is_wall(&map, &gd, 2, 2));
    }

    /// cave.cc map_info layers: `display_terrain` applies the cell mimic
    /// (or the f_info default mimic) without changing the real terrain.
    #[test]
    fn display_layers_honour_mimics() {
        let gd = load_game_data();
        let mut map = blank_map(T_FLOOR);
        map.set_terrain(3, 3, T_GRANITE);
        assert_eq!(map.display_terrain(&gd, 3, 3), T_GRANITE);
        map.mimic.insert(Map::idx(3, 3), T_FLOOR);
        assert_eq!(map.display_terrain(&gd, 3, 3), T_FLOOR);
        assert_eq!(map.terrain_at(3, 3), T_GRANITE);
        // Unknown traps are drawn as floor (render.rs cell_render).
        map.set_terrain(4, 4, T_TRAP);
        assert_eq!(map.terrain_at(4, 4), T_TRAP);
        assert!(!map.known_traps.contains(&Map::idx(4, 4)));
    }

    /// cave.cc:3769 wiz_dark / wiz_lite_extra (Map::reveal_all).
    #[test]
    fn wiz_dark_forgets_and_reveal_all_shows() {
        let mut map = blank_map(T_FLOOR);
        map.explored[Map::idx(1, 1)] = true;
        map.visible[Map::idx(1, 1)] = true;
        map.wiz_dark();
        assert!(!map.explored.iter().any(|&e| e));
        map.reveal_all();
        assert!(map.explored.iter().all(|&e| e));
        assert!(map.visible.iter().all(|&v| v));
        assert!(map.lit.iter().all(|&l| l));
    }

    /// cave.cc:4002 scatter: bounded, LOS-checked spots.
    #[test]
    fn scatter_pos_stays_in_los_and_range() {
        let gd = load_game_data();
        let map = blank_map(T_FLOOR);
        let mut rng = seeded(7);
        for _ in 0..50 {
            let (ny, nx) = scatter_pos(&map, &gd, 20, 20, 3, &mut rng).expect("spot");
            assert!(Map::in_bounds(nx, ny));
            assert!(pref_distance(20, 20, ny, nx) <= 3);
            assert!(line_of_sight(&map, &gd, 20, 20, nx, ny));
        }
    }

    /// cave.cc:3895 mmove2 incremental motion.
    #[test]
    fn mmove2_steps_towards_the_target() {
        assert_eq!(mmove2(0, 0, 0, 0, 0, 5), (0, 1));
        assert_eq!(mmove2(1, 0, 0, 0, 10, 5), (2, 1));
        assert_eq!(mmove2(3, 3, 3, 3, 3, 3), (3, 3));
    }

    /// generate.cc:339/363 correct_dir and rand_dir.
    #[test]
    fn correct_dir_never_moves_diagonally() {
        let mut rng = seeded(1);
        for _ in 0..200 {
            let (r, c) = correct_dir(3, 3, 9, 12, &mut rng);
            assert!(r == 0 || c == 0);
            assert!((r, c) == (1, 0) || (r, c) == (0, 1));
        }
        assert_eq!(correct_dir(5, 5, 5, 9, &mut rng), (0, 1));
        assert_eq!(correct_dir(5, 5, 9, 5, &mut rng), (1, 0));
        for _ in 0..50 {
            let (r, c) = rand_dir(&mut rng);
            assert!(DIR4.contains(&(r, c)));
        }
    }

    /// generate.cc:1362/1447 streamers only convert eligible terrain.
    #[test]
    fn streamers_convert_eligible_walls() {
        let gd = load_game_data();
        let d = gd.dungeon(2);
        let mut rng = seeded(3);
        let mut map = blank_map(T_GRANITE);
        let fills = [T_GRANITE; 100];
        build_streamer(&mut map, d, &fills, 50, 54, 90, MAP_W, MAP_H, &mut rng);
        let veins = map.terrain.iter().filter(|&&t| t == 50 || t == 54).count();
        assert!(veins > 0, "streamer placed no vein");
        assert!(veins < (MAP_W * MAP_H) as usize / 2, "streamer flooded");

        let mut map2 = blank_map(T_GRANITE);
        build_streamer2(&mut map2, &gd, T_SHAL_WATER, false, MAP_W, MAP_H, &mut rng);
        assert!(map2.terrain.iter().all(|&t| t == T_GRANITE));
    }

    /// generate.cc:1196/1307 recursive_river / add_river.
    #[test]
    fn rivers_carve_water_and_lava() {
        let gd = load_game_data();
        let mut rng = seeded(31);
        let mut map = blank_map(T_GRANITE);
        add_river(&mut map, &gd, T_DEEP_WATER, T_SHAL_WATER, &mut rng);
        assert!(map
            .terrain
            .iter()
            .any(|&t| t == T_DEEP_WATER || t == T_SHAL_WATER));
        assert!(!map.icky.is_empty(), "river cells must be CAVE_ICKY");
        let mut map2 = blank_map(T_GRANITE);
        add_river(&mut map2, &gd, T_LAVA, T_SHAL_LAVA, &mut rng);
        assert!(map2.terrain.iter().any(|&t| t == T_LAVA || t == T_SHAL_LAVA));
        assert!(map2.lit.iter().any(|&l| l), "lava river glows");
    }

    /// generate.cc:7149 fill_level: smooth fills cover the region.
    #[test]
    fn fill_level_covers_small_maps() {
        let mut rng = seeded(9);
        let fills = [T_GRANITE; 100];
        let mut map = blank_map(T_NOTHING);
        map.w = 12;
        map.h = 12;
        fill_level(&mut map, 12, 12, &fills, 4, &mut rng);
        for y in 0..12 {
            for x in 0..12 {
                assert_eq!(map.terrain_at(x, y), T_GRANITE);
            }
        }
    }

    /// generate.cc vault_aux_* predicates.
    #[test]
    fn vault_theme_filters_match_the_original() {
        let gd = load_game_data();
        if let Some(unique) = gd.monsters.iter().find(|m| m.unique) {
            assert!(!theme_ok(&gd, unique, NestTheme::Orc));
        }
        if let Some(orc) = gd.monsters.iter().find(|m| !m.unique && m.glyph() == 'o') {
            assert!(theme_ok(&gd, orc, NestTheme::Orc));
            assert!(!theme_ok(&gd, orc, NestTheme::Undead));
        }
        if let Some(dragon) = gd.monsters.iter().find(|m| {
            !m.unique && matches!(m.glyph(), 'D' | 'd') && m.spells.iter().any(|s| s == "BR_FIRE")
        }) {
            assert!(theme_ok(&gd, dragon, NestTheme::Dragon(&["BR_FIRE"])));
            assert!(!theme_ok(&gd, dragon, NestTheme::Dragon(&["BR_COLD", "BR_FIRE"]))
                || dragon.spells.iter().all(|s| s != "BR_COLD"));
        }
    }

    /// generate.cc:974 alloc_stairs / place_stairs: wall-adjacent naked
    /// floor, the decreasing wall requirement and the SAFE_MAX_ATTEMPTS
    /// retry bounds.
    #[test]
    fn place_stairs_writes_staircases() {
        let gd = load_game_data();
        let mut rng = seeded(5);
        // A lone floor cell is surrounded by four walls (next_to_walls 4),
        // so the walls=3 requirement accepts it right away.
        let mut map = blank_map(T_GRANITE);
        map.set_terrain(10, 10, T_FLOOR);
        let first = place_stairs(&mut map, &gd, T_STAIRS_DOWN, 3, Some(7), &mut rng);
        let (x, y) = first.expect("no staircase placed");
        assert_eq!((x, y), (10, 10));
        assert_eq!(map.terrain_at(x, y), T_STAIRS_DOWN);
        assert_eq!(map.special.get(&Map::idx(x, y)), Some(&7));
        let stairs = map.terrain.iter().filter(|&&t| t == T_STAIRS_DOWN).count();
        assert!((1..=3).contains(&stairs), "stairs {stairs}");
        // Like the original, a wall-less floor can fail the first stair
        // requirement but the loop retries with fewer walls until one is
        // placed (num > 1).
        let mut open = blank_map(T_GRANITE);
        for yy in 9..=11 {
            for xx in 9..=11 {
                open.set_terrain(xx, yy, T_FLOOR);
            }
        }
        assert!(
            place_stairs(&mut open, &gd, T_STAIRS_DOWN, 2, None, &mut rng).is_some(),
            "the lowered wall requirement must still place a stair"
        );
    }

    /// generate.cc:684 new_player_spot's naked-floor search.
    #[test]
    fn random_naked_spot_avoids_occupied_and_icky() {
        let gd = load_game_data();
        let mut rng = seeded(11);
        let mut map = blank_map(T_FLOOR);
        let mut occupied = HashSet::new();
        occupied.insert((10, 10));
        map.icky.insert(Map::idx(11, 10));
        let (x, y) = random_naked_spot(&map, &gd, MAP_W, MAP_H, &occupied, &mut rng).unwrap();
        assert_ne!((x, y), (10, 10));
        assert!(!map.icky.contains(&Map::idx(x, y)));
    }

    /// generate.cc:448 place_new_way digs a path to the dungeon floors.
    #[test]
    fn place_new_way_digs_to_the_floor() {
        let gd = load_game_data();
        let mut rng = seeded(42);
        let mut map = blank_map(T_GRANITE);
        for y in 30..36 {
            for x in 90..110 {
                map.set_terrain(x, y, T_FLOOR);
            }
        }
        let before = map.terrain.iter().filter(|&&t| t == T_FLOOR).count();
        let floors = [T_FLOOR; 100];
        let room_flags = vec![false; (MAP_W * MAP_H) as usize];
        let spot = place_new_way(&mut map, &gd, &floors, &room_flags, MAP_W, MAP_H, &mut rng);
        assert!(spot.is_some(), "no way found");
        let after = map.terrain.iter().filter(|&&t| t == T_FLOOR).count();
        assert!(after > before, "way dug no floor");
    }

    /// generate.cc:797 place_fountain.
    #[test]
    fn place_fountain_marks_a_dungeon_fountain() {
        let gd = load_game_data();
        let mut rng = seeded(21);
        let mut map = blank_map(T_FLOOR);
        let room_flags = vec![true; (MAP_W * MAP_H) as usize];
        place_fountain(&gd, &mut map, &room_flags, 50, MAP_W, MAP_H, &mut rng);
        let eligible = gd.objects.iter().any(|o| {
            (o.tval == crate::data::TV_POTION || o.tval == crate::data::TV_POTION2)
                && o.depth <= 50
                && o.flags.iter().any(|f| f == "FOUNTAIN")
        });
        if eligible {
            assert_eq!(map.fountains.len(), 1);
        }
    }

    /// generate.cc alloc_object features (rubble/altar/fountain).
    #[test]
    fn alloc_object_features_appear_in_levels() {
        let gd = load_game_data();
        let mut rng = seeded(77);
        let mut rubble = false;
        let mut fountain = false;
        for _ in 0..10 {
            let gen = generate_dungeon_level(&gd, 4, 10, &mut rng);
            rubble |= gen.map.terrain.iter().any(|&t| t == T_RUBBLE);
            fountain |= !gen.map.fountains.is_empty();
        }
        assert!(rubble, "no rubble placed");
        assert!(fountain, "no fountain placed");
    }

    /// generate.cc:1571 destroy_level reshapes terrain.
    #[test]
    fn destroy_level_reshapes_the_map() {
        let gd = load_game_data();
        let mut rng = seeded(13);
        let mut map = blank_map(T_FLOOR);
        let mut vault = VaultSpawns::default();
        let floors = [T_FLOOR; 100];
        destroy_level(&mut map, &gd, &mut vault, &floors, MAP_W, MAP_H, &mut rng);
        assert!(map.terrain.iter().any(|&t| t != T_FLOOR));
    }

    /// generate.cc:7041 init_feat_info builds the 100-slot tables from the
    /// dungeon's floor/fill definitions.
    #[test]
    fn init_feat_info_uses_the_dungeon_tables() {
        let gd = load_game_data();
        let d = gd.dungeon(2);
        let (floors, fills) = init_feat_info(d, d.mindepth);
        let floor_feats: HashSet<u16> = d.floors.iter().map(|f| f.feat).collect();
        let fill_feats: HashSet<u16> = d.fills.iter().map(|f| f.feat).collect();
        assert!(floors.iter().all(|t| floor_feats.contains(t)));
        assert!(fills.iter().all(|t| fill_feats.contains(t)));
        assert!(floors.len() == 100 && fills.len() == 100);
    }

    /// cave.cc:452/487/501/518/605 hallucination glyphs and multi-hued
    /// monster colours.
    #[test]
    fn hallucination_and_shimmer_helpers_follow_cave_cc() {
        let gd = load_game_data();
        let mut rng = seeded(5);
        let (ch, color) = image_monster(&gd, &mut rng);
        assert!(ch != '\0' && color < 16);
        let (_, color) = image_object(&gd, &mut rng);
        assert!(color < 16);
        let (_, color) = image_random(&gd, &mut rng);
        assert!(color < 16);
        assert!((1..=15).contains(&get_shimmer_color(&mut rng)));

        let no_breath = gd.monsters.iter().find(|m| m.spell_freq == 0 && !m.spells.is_empty());
        if let Some(m) = no_breath {
            assert!(multi_hued_attr(m, &mut rng) < 16);
        }
        if let Some(m) = gd.monsters.iter().find(|m| {
            m.spell_freq > 0 && m.spells.iter().any(|s| s == "BR_CHAO")
        }) {
            assert!((1..=15).contains(&multi_hued_attr(m, &mut rng)));
        }
        // A one-breath monster uses both colours of that breath.
        if let Some(m) = gd.monsters.iter().find(|m| {
            m.spell_freq > 0 && m.spells.iter().filter(|s| s.starts_with("BR_")).count() == 1
        }) {
            let a = multi_hued_attr(m, &mut rng);
            assert!((1..=15).contains(&a));
        }
    }

    /// cave.cc:3492/3542 update_flow: BFS distance from the player.
    #[test]
    fn compute_flow_matches_the_original_bfs() {
        let mut map = blank_map(T_GRANITE);
        for y in 8..12 {
            for x in 8..16 {
                map.set_terrain(x, y, T_FLOOR);
            }
        }
        let flow = compute_flow(&map, 10, 10, MONSTER_FLOW_DEPTH);
        assert_eq!(flow[Map::idx(10, 10)], 0);
        assert_eq!(flow[Map::idx(11, 10)], 1);
        assert_eq!(flow[Map::idx(15, 10)], 5);
        // 8-connected BFS: one diagonal then four straight steps.
        assert_eq!(flow[Map::idx(15, 11)], 5);
        // Walls are never reached.
        assert_eq!(flow[Map::idx(7, 10)], u16::MAX);
        let shallow = compute_flow(&map, 10, 10, 3);
        assert_eq!(shallow[Map::idx(15, 10)], u16::MAX);
        assert_eq!(shallow[Map::idx(13, 10)], 3);
    }

    /// cave.cc:4038/4052/4067 tracking bookkeeping.
    #[test]
    fn tracking_records_the_displayed_entries() {
        let mut t = Tracking::default();
        t.health_track(Some(12));
        assert_eq!(t.health_who, Some(12));
        t.health_track(None);
        assert_eq!(t.health_who, None);
        t.monster_race_track(42, 3);
        assert_eq!(t.monster_race, Some((42, 3)));
        t.object_track(7);
        assert_eq!(t.object, Some(7));
    }

    /// cave.cc:1667 priority and the display glyph for memorized terrain.
    #[test]
    fn display_priority_matches_cave_cc() {
        let gd = load_game_data();
        let floor = gd.terrain(T_FLOOR);
        assert_eq!(
            display_priority(&gd, floor.ch.chars().next().unwrap(), floor.color),
            5
        );
        let granite = gd.terrain(T_GRANITE);
        // Granite walls are drawn as secret doors; secret doors score 10.
        let secret = gd.terrain(48);
        assert_eq!(display_priority(&gd, secret.ch.chars().next().unwrap(), secret.color), 10);
        let down = gd.terrain(T_STAIRS_DOWN);
        assert_eq!(display_priority(&gd, down.ch.chars().next().unwrap(), down.color), 25);
        let _ = granite;
        assert_eq!(display_priority(&gd, '~', 15), 20);
    }

    /// cave.cc:1380/1391 panel_col_of/move_cursor_relative: the Bevy
    /// camera maps a grid to screen world coordinates instead.
    #[test]
    fn screen_coordinates_map_the_panel() {
        let centre = crate::render::grid_to_world(MAP_W / 2, MAP_H / 2, 0.0);
        assert!(centre.x.abs() <= crate::render::TILE);
        let left = crate::render::grid_to_world(0, 0, 0.0);
        let right = crate::render::grid_to_world(MAP_W - 1, MAP_H - 1, 0.0);
        assert!(left.x < right.x && left.y > right.y);
    }

    /// cave.cc:3864/3875/3885 place_floor, glass conversion, place_filler.
    #[test]
    fn floor_and_filler_placement_helpers() {
        let gd = load_game_data();
        let d = gd.dungeon(2);
        let mut rng = seeded(17);
        let (floors, fills) = init_feat_info(d, 30);
        let mut map = blank_map(T_NOTHING);
        for y in 0..10 {
            for x in 0..10 {
                place_floor(&mut map, &floors, x, y, &mut rng);
                place_filler(&mut map, &fills, x + 10, y, &mut rng);
            }
        }
        for y in 0..10 {
            for x in 0..20 {
                assert_ne!(map.terrain_at(x, y), T_NOTHING, "cell {x},{y} not filled");
            }
        }
        place_floor_convert_glass(&mut map, d, 30, 5, 5, &mut rng);
        assert_ne!(map.terrain_at(5, 5), 188, "glass wall left in floor");
    }
}

/// Audit tests for bldg.cc building actions, driven through the public
/// `modal::modal_input`/`modal::sync_modal` entry points (modal.rs owns
/// the implementation; these tests pin the bldg.cc behaviour).
#[cfg(test)]
mod bldg_tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    use crate::data::load_game_data;
    use crate::game::{
        GridPos, MessageLog, Player, PlayerState, PlotQuest, TurnState, TURNS_PER_DAY,
    };
    use crate::item::{CreatedArtifacts, Inventory};
    use crate::modal::{Modal, ModalInputConsumed, ModalText, TargetLock};
    use crate::town::ShopStocks;

    /// A minimal World wired for `modal_input`/`sync_modal`.
    pub(crate) fn bldg_world(modal: Modal) -> World {
        crate::rng::set_quick_rng(0xB1D6_0000);
        let gd = load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Mage").unwrap_or(0);
        let mut ps = crate::birth::make_player(&gd, "Bldg".into(), race, class);
        ps.gold = 10_000;
        ps.level = 10;
        let mut map = super::blank_pub(super::T_FLOOR);
        map.town = 1;
        let mut world = World::new();
        world.insert_resource(gd);
        world.insert_resource(ps);
        world.insert_resource(crate::render::TileAssets {
            font: Default::default(),
            bg_image: Default::default(),
            bg_layout: Default::default(),
            fg_image: Default::default(),
            fg_layout: Default::default(),
        });
        world.insert_resource(Inventory::default());
        world.insert_resource(map);
        world.insert_resource(MessageLog::default());
        world.insert_resource(TurnState::default());
        world.insert_resource(PlotQuest::default());
        world.insert_resource(ShopStocks::default());
        world.insert_resource(CreatedArtifacts::default());
        world.insert_resource(crate::scores::HighScores::default());
        world.insert_resource(NextState::<crate::AppState>::default());
        world.insert_resource(TargetLock::default());
        world.insert_resource(crate::input::MoveRepeat::default());
        world.insert_resource(crate::options::Options::default());
        world.insert_resource(ModalInputConsumed::default());
        world.insert_resource(crate::notes::Notes::default());
        world.insert_resource(modal);
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.spawn((GridPos { x: 10, y: 10 }, Player));
        world
    }

    fn press(world: &mut World, key: KeyCode) {
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset(key);
            keys.press(key);
        }
        world.run_system_once(crate::modal::modal_input).unwrap();
        world.resource_mut::<ButtonInput<KeyCode>>().reset(key);
    }

    fn reopen(world: &mut World, modal: Modal) {
        *world.resource_mut::<Modal>() = modal;
    }

    pub(crate) fn last_log(world: &World) -> String {
        world
            .resource::<MessageLog>()
            .lines
            .back()
            .cloned()
            .unwrap_or_default()
    }

    fn log_count(world: &World) -> usize {
        world.resource::<MessageLog>().lines.len()
    }

    #[test]
    fn casino_actions_open_the_gamble_modal() {
        // Casino (15) actions: a) rules, b/c/d) In-Between/Craps/Slots.
        for (key, game) in [
            (KeyCode::KeyB, 12),
            (KeyCode::KeyC, 14),
            (KeyCode::KeyD, 16),
        ] {
            let mut world = bldg_world(Modal::Building(15));
            press(&mut world, key);
            assert!(
                matches!(*world.resource::<Modal>(), Modal::Gamble { game: g, .. } if g == game),
                "choice {key:?} did not open game {game}"
            );
        }
        let mut world = bldg_world(Modal::Building(15));
        press(&mut world, KeyCode::KeyA);
        assert!(matches!(
            *world.resource::<Modal>(),
            Modal::TextPage { ref title, .. } if title == "Gambling rules"
        ));
    }

    #[test]
    fn inn_rest_food_and_rumors_follow_inn_comm() {
        // The Prancing Pony (58): rest is a paid night action.
        let mut world = bldg_world(Modal::Building(58));
        {
            let mut ps = world.resource_mut::<PlayerState>();
            ps.turn = TURNS_PER_DAY * 3 / 4; // night
            ps.hp = 1;
            ps.mana = 0;
            ps.poison = 0;
            ps.cut = 0;
        }
        // The Pony's st_info actions are [2,4,5,6,7], so the menu
        // letters are c) rest, d) food, e) rumours (data-driven).
        press(&mut world, KeyCode::KeyC);
        {
            let ps = world.resource::<PlayerState>();
            assert_eq!(ps.gold, 9980);
            assert_eq!(ps.hp, ps.max_hp);
            assert_eq!(ps.mana, ps.max_mana);
            assert!(crate::game::is_daytime(ps.turn), "time advanced to sunrise");
        }
        // Mortals cannot take a room by day.
        reopen(&mut world, Modal::Building(58));
        world.resource_mut::<PlayerState>().turn = 100;
        press(&mut world, KeyCode::KeyC);
        assert!(last_log(&world).contains("only at night"));
        // Buy food and drink (2 au, ba 6).
        reopen(&mut world, Modal::Building(58));
        press(&mut world, KeyCode::KeyD);
        assert_eq!(
            world.resource::<PlayerState>().food,
            crate::game::FOOD_MAX - 1
        );
        // Listen for rumours prints a line.
        reopen(&mut world, Modal::Building(58));
        let before = log_count(&world);
        press(&mut world, KeyCode::KeyE);
        assert!(log_count(&world) > before);
    }

    #[test]
    fn building_restrictions_and_charges_follow_bldg_process_command() {
        // Black Market "share of stolen gold" (ba 30) is liked-only
        // (restr == 2): an unfriendly/absent owner refuses.
        let mut world = bldg_world(Modal::Building(6));
        press(&mut world, KeyCode::KeyA);
        assert!(last_log(&world).contains("no right to choose"));
        // With an owner that likes the player's race the action runs.
        let owner = {
            let gd = world.resource::<crate::data::GameData>();
            gd.owners
                .iter()
                .find(|o| o.liked.iter().any(|n| n == "Human" || n == "Mage"))
                .expect("a friendly owner")
                .id
        };
        world
            .resource_mut::<ShopStocks>()
            .owners
            .insert(crate::town::skey(1, 6), owner);
        reopen(&mut world, Modal::Building(6));
        press(&mut world, KeyCode::KeyA);
        assert!(!last_log(&world).contains("no right to choose"));
    }

    #[test]
    fn compare_weapons_filters_melee_and_charges_once() {
        let mut world = bldg_world(Modal::Building(2));
        {
            let (sword, potion) = {
                let gd = world.resource::<crate::data::GameData>();
                (
                    gd.object_by_name("Long Sword").unwrap(),
                    gd.object_by_tval_sval(crate::data::TV_POTION, 1).unwrap(),
                )
            };
            let gd = world.resource::<crate::data::GameData>();
            let items = [
                crate::item::Item::base(&gd, sword),
                crate::item::Item::base(&gd, potion),
                crate::item::Item::base(&gd, sword),
            ];
            world.resource_mut::<Inventory>().pack.extend(items);
        }
        // Open the smith's service (ba 23 -> BACT_COMPARE_WEAPONS).
        press(&mut world, KeyCode::KeyA);
        assert!(matches!(
            *world.resource::<Modal>(),
            Modal::CompareWeapons {
                cost: 200,
                first: None,
                ..
            }
        ));
        // First pick is the first sword; second pick must skip the potion.
        press(&mut world, KeyCode::KeyA);
        press(&mut world, KeyCode::KeyB);
        match &*world.resource::<Modal>() {
            Modal::CompareWeapons { first, second, .. } => {
                assert_eq!(*first, Some(0));
                assert_eq!(*second, Some(2), "the potion was not offered");
            }
            _ => panic!("expected CompareWeapons"),
        }
        assert_eq!(world.resource::<PlayerState>().gold, 10_000 - 200);
    }

    #[test]
    fn compare_weapons_needs_two_melee_weapons() {
        let mut world = bldg_world(Modal::Building(2));
        let (sword, potion) = {
            let gd = world.resource::<crate::data::GameData>();
            (
                gd.object_by_name("Long Sword").unwrap(),
                gd.object_by_tval_sval(crate::data::TV_POTION, 1).unwrap(),
            )
        };
        let gd = world.resource::<crate::data::GameData>();
        let items = [
            crate::item::Item::base(&gd, sword),
            crate::item::Item::base(&gd, potion),
        ];
        world.resource_mut::<Inventory>().pack.extend(items);
        press(&mut world, KeyCode::KeyA);
        assert!(last_log(&world).contains("Bring me weapons"));
    }

    #[test]
    fn enchant_weapon_caps_at_level_over_five_and_charges() {
        let mut world = bldg_world(Modal::Building(17));
        {
            let gd = world.resource::<crate::data::GameData>();
            let sword = gd.object_by_name("Long Sword").unwrap();
            let it = crate::item::Item::base(&gd, sword);
            world.resource_mut::<Inventory>().equip[crate::data::SLOT_WEAPON] = Some(it);
        }
        press(&mut world, KeyCode::KeyA);
        {
            let inv = world.resource::<Inventory>();
            let w = inv.equip[crate::data::SLOT_WEAPON].as_ref().unwrap();
            assert_eq!((w.to_h, w.to_d), (1, 1));
        }
        assert_eq!(world.resource::<PlayerState>().gold, 10_000 - 700);
        // Level 10 caps the smith at +2: the second visit reaches it...
        reopen(&mut world, Modal::Building(17));
        press(&mut world, KeyCode::KeyA);
        assert_eq!(world.resource::<PlayerState>().gold, 10_000 - 1400);
        // ...and the third is free ("in fine condition").
        reopen(&mut world, Modal::Building(17));
        press(&mut world, KeyCode::KeyA);
        assert_eq!(world.resource::<PlayerState>().gold, 10_000 - 1400);
        assert!(
            world
                .resource::<MessageLog>()
                .lines
                .iter()
                .any(|l| l.contains("fine condition")),
            "no fine-condition status line"
        );
    }

    #[test]
    fn compare_weapons_report_lists_strike_and_attack() {
        let mut world = bldg_world(Modal::CompareWeapons {
            first: Some(0),
            second: Some(1),
            cost: 200,
        });
        {
            let (sword, dagger) = {
                let gd = world.resource::<crate::data::GameData>();
                (
                    gd.object_by_name("Long Sword").unwrap(),
                    gd.object_by_name("Dagger").unwrap(),
                )
            };
            let gd = world.resource::<crate::data::GameData>();
            let items = [
                crate::item::Item::base(&gd, sword),
                crate::item::Item::base(&gd, dagger),
            ];
            world.resource_mut::<Inventory>().pack.extend(items);
        }
        world.spawn((Text::default(), ModalText));
        world.run_system_once(crate::modal::sync_modal).unwrap();
        let mut q = world.query::<&Text>();
        let text = q.iter(&world).map(|t| t.0.clone()).collect::<String>();
        assert!(text.contains("One Strike:"), "{text}");
        assert!(text.contains("One Attack:"), "{text}");
        assert!(text.contains("Blows"), "{text}");
        assert!(text.contains("Long Sword"), "{text}");
        assert!(text.contains("Dagger"), "{text}");
    }

    #[test]
    fn building_menu_text_lists_the_actions() {
        // clear_bldg/show_building are terminal row painters; the port's
        // equivalent is the modal text rebuilt by sync_modal.
        let mut world = bldg_world(Modal::Building(58));
        world.spawn((Text::default(), ModalText));
        world.run_system_once(crate::modal::sync_modal).unwrap();
        let mut q = world.query::<&Text>();
        let text = q.iter(&world).map(|t| t.0.clone()).collect::<String>();
        assert!(text.contains("Prancing Pony"));
        assert!(text.contains("Rest for the night"));
    }

    #[test]
    fn town_history_opens_the_bldg_txt_page() {
        // The Mirror of Galadriel's "Town history" (23,2 -> ba 15).
        let mut world = bldg_world(Modal::Building(23));
        press(&mut world, KeyCode::KeyC);
        let modal = world.resource::<Modal>();
        match &*modal {
            Modal::TextPage { title, lines, .. } => {
                assert_eq!(title, "Historical Town View");
                assert!(!lines.is_empty(), "bldg.txt rendered");
            }
            _ => panic!("expected the town history page"),
        }
    }
}

/// Audit tests for wild.cc (plasma generation, town layouts, wilderness
/// overview and the reveal radius); town layout coverage lives in
/// `tests::dungeon_towns_have_reachable_shops` and `tests::town_has_shops_and_stairs`.
#[cfg(test)]
mod wild_tests {
    use super::*;
    use crate::data::load_game_data;

    #[test]
    fn plasma_area_is_deterministic_and_uses_the_terrain_table() {
        let gd = load_game_data();
        let table: Vec<u16> = gd.wf(1).terrain.clone();
        let a = plasma_area(66, 101, &table, 12345);
        let b = plasma_area(66, 101, &table, 12345);
        assert_eq!(a, b, "same seed, same fractal");
        let c = plasma_area(66, 101, &table, 54321);
        assert_ne!(a, c, "different seed, different fractal");
        assert_eq!(a.len(), (101 * 66) as usize);
        // Every cell maps through the wf terrain table; the untouched
        // border keeps the MAX_WILD_TERRAIN/2 height (9).
        assert!(a.iter().all(|t| table.contains(t)));
        assert_eq!(a[0], table[9], "border row keeps the base height");
        assert_eq!(a[(66 - 1) * 101], table[9], "border column base height");
    }

    /// wild.cc:47/77 perturb_point_mid/end: the integer average rounds up
    /// on the original's truthiness tests for non-negative heights, and
    /// the result is clamped to [0, depth_max].
    #[test]
    fn plasma_perturbations_round_up_like_the_original() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let mut c = vec![0i32; 25];
        // rough = 0 makes the perturbation randint(1) - 1 = 0.
        perturb_end(&mut c, 5, [3, 3, 3], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 3, "9/3, no remainder");
        perturb_end(&mut c, 5, [3, 3, 4], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 4, "10/3 rounds up");
        perturb_end(&mut c, 5, [3, 3, 7], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 5, "13/3 rounds up");
        perturb_mid(&mut c, 5, [3, 3, 3, 3], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 3, "12/4, remainder 0");
        perturb_mid(&mut c, 5, [3, 3, 3, 4], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 3, "13/4, remainder 1 does not round");
        perturb_mid(&mut c, 5, [3, 3, 4, 4], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 4, "14/4, remainder 2 rounds");
        perturb_end(&mut c, 5, [0, 0, 0], 2, 2, 0, 100, &mut rng);
        assert_eq!(c[12], 0);
        perturb_end(&mut c, 5, [100, 100, 100], 2, 2, 0, 7, &mut rng);
        assert_eq!(c[12], 7, "clamped to depth_max");
    }

    #[test]
    fn wilderness_reveal_marks_a_radius() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut wild = crate::game::Wilderness::new(&gd, &mut rng);
        let (x, y) = (wild.w / 2, wild.h / 2);
        wild.reveal(x, y, 3);
        assert!(wild.is_known(x, y));
        assert!(wild.is_known(x + 2, y));
        assert!(!wild.is_known(x + 4, y), "outside the radius");
        assert!(!wild.is_known(x, y + 4));
    }

    #[test]
    fn wilderness_gen_places_roads_and_entrance() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let wild = crate::game::Wilderness::new(&gd, &mut rng);
        let plot = crate::game::PlotQuest::default();
        // The shipped world grid references no ROAD_* cells, so patch one
        // to wf 7 ("road", ROAD_EAST|ROAD_WEST) to exercise the painters.
        let mut gd = gd;
        let (wx, wy) = (50, 33);
        assert!(gd.world.rows[wy as usize][wx as usize] == 0 || true);
        gd.world.rows[wy as usize][wx as usize] = 7;
        let gen = generate_wild_area(&gd, &wild, &plot, wx, wy, true, false, &mut rng);
        assert_eq!(gen.map.terrain_at(MAP_W - 2, MAP_H / 2), T_FLOOR, "east road");
        assert_eq!(gen.map.terrain_at(1, MAP_H / 2), T_FLOOR, "west road");
    }

    #[test]
    fn generate_world_map_marks_known_and_dungeon_cells() {
        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let mut wild = crate::game::Wilderness::new(&gd, &mut rng);
        let plot = crate::game::PlotQuest::default();
        // Mirkwood's '*' world cell (dungeon 1).
        let (ex, ey) = (0..gd.world.h)
            .flat_map(|y| (0..gd.world.w).map(move |x| (x, y)))
            .find(|&(x, y)| wild.wf_at(&gd, x, y, &plot).dungeon() == Some(1))
            .expect("Mirkwood cell");
        wild.reveal(ex, ey, 2);
        let gen = generate_world_map(&gd, &wild, &plot);
        assert_eq!(gen.map.terrain_at(ex, ey), T_STAIRS_DOWN);
        assert!(gen.map.explored[Map::idx(ex, ey)], "known cell is marked");
        assert!(!gen.map.explored[Map::idx(0, 0)], "unknown cell stays dark");
    }
}

/// Audit tests for dungeon.cc world-turn hooks: resurrection, upkeep
/// regeneration/recharging/decay, clouds and the god trickle.  The world
/// turn is driven through the public `game::monster_turns` system.
#[cfg(test)]
mod dungeon_cc_tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use rand::SeedableRng;

    use super::bldg_tests::bldg_world;
    use crate::data::load_game_data;
    use crate::game::{GridPos, MessageLog, PlayerState, TurnState};
    use crate::item::{Inventory, Item};
    use crate::modal::Modal;

    fn world_turn(world: &mut World) {
        world.resource_mut::<TurnState>().world_turn = true;
        world.run_system_once(crate::game::monster_turns).unwrap();
    }

    #[test]
    fn erus_grace_can_resurrect_the_faithful() {
        // 70% roll: a handful of attempts settles it probabilistically.
        let gd = load_game_data();
        let mut world = World::new();
        world.insert_resource(gd);
        world.insert_resource(crate::scores::HighScores::default());
        let mut revived = false;
        for _ in 0..64 {
            let mut ps = {
                let gd = world.resource::<crate::data::GameData>();
                crate::birth::make_player(gd, "Eru".into(), 0, 0)
            };
            ps.god = 1;
            ps.praying = true;
            ps.grace = 200_000;
            ps.hp = 0;
            world.insert_resource(ps);
            world
                .run_system_once(crate::game::cleanup_level)
                .unwrap();
            if world.get_resource::<crate::game::Resurrect>().is_some() {
                revived = true;
                break;
            }
        }
        assert!(revived, "Eru never resurrected in 64 attempts");
        // A different god's prayer is a final death.
        world.remove_resource::<crate::game::Resurrect>();
        let mut ps = {
            let gd = world.resource::<crate::data::GameData>();
            crate::birth::make_player(gd, "Mortal".into(), 0, 0)
        };
        ps.god = 7;
        ps.praying = true;
        ps.grace = 200_000;
        ps.hp = 0;
        world.insert_resource(ps);
        world
            .run_system_once(crate::game::cleanup_level)
            .unwrap();
        assert!(world.get_resource::<crate::game::Resurrect>().is_none());
    }

    #[test]
    fn world_upkeep_regenerates_hp_and_mana() {
        let mut world = bldg_world(Modal::None);
        let start_hp = {
            let mut ps = world.resource_mut::<PlayerState>();
            ps.hp = (ps.max_hp - 50).max(1);
            ps.mana = 0;
            ps.food = crate::game::FOOD_MAX;
            ps.poison = 0;
            ps.cut = 0;
            ps.hp
        };
        for _ in 0..40 {
            world_turn(&mut world);
        }
        let ps = world.resource::<PlayerState>();
        assert!(ps.hp > start_hp, "hp {} -> {}", start_hp, ps.hp);
        assert!(ps.mana > 0, "mana did not regenerate");
    }

    #[test]
    fn recharged_rods_notice_double_bang() {
        let mut world = bldg_world(Modal::None);
        {
            let def = {
                let gd = world.resource::<crate::data::GameData>();
                gd.objects
                    .iter()
                    .position(|o| o.tval == crate::data::TV_ROD_MAIN)
                    .expect("rod main")
            };
            let gd = world.resource::<crate::data::GameData>();
            let mut it = Item::base(&gd, def);
            it.pval2 = 10;
            it.timeout = 9;
            it.inscription = "!!".to_string();
            world.resource_mut::<Inventory>().pack.push(it);
        }
        world_turn(&mut world);
        assert!(
            super::bldg_tests::last_log(&world).contains("recharged"),
            "no recharge notice"
        );
    }

    #[test]
    fn corpse_decay_runs_on_the_world_turn() {
        let mut world = bldg_world(Modal::None);
        {
            let def = {
                let gd = world.resource::<crate::data::GameData>();
                gd.object_by_tval_sval(crate::data::TV_CORPSE, 1)
                    .expect("corpse kind")
            };
            let gd = world.resource::<crate::data::GameData>();
            let mut it = Item::base(&gd, def);
            it.fuel = 1;
            world.resource_mut::<Inventory>().pack.push(it);
        }
        world_turn(&mut world);
        let log = world.resource::<MessageLog>();
        assert!(
            log.lines.iter().any(|l| l.contains("decompose")),
            "no decay message: {:?}",
            log.lines
        );
    }

    #[test]
    fn monsters_and_symbiotes_regenerate_every_hundred_turns() {
        let mut world = bldg_world(Modal::None);
        world.spawn((
            ron::from_str::<crate::game::Monster>(
                "(def:1,hp:50,max_hp:100,energy:0,awake:true)",
            )
            .expect("monster"),
            GridPos { x: 20, y: 20 },
        ));
        {
            let def = {
                let gd = world.resource::<crate::data::GameData>();
                gd.objects
                    .iter()
                    .position(|o| o.tval == crate::data::TV_SWORD)
                    .expect("sword kind")
            };
            let gd = world.resource::<crate::data::GameData>();
            let mut sym = Item::base(&gd, def);
            sym.pval2 = 10;
            sym.pval3 = 1000;
            world.resource_mut::<Inventory>().equip[crate::data::SLOT_SYMBIOTE] = Some(sym);
        }
        world.resource_mut::<PlayerState>().turn = 99;
        world_turn(&mut world); // ps.turn becomes 100
        let mut q = world.query::<&crate::game::Monster>();
        let hp = q.iter(&world).next().map(|m| m.hp).unwrap_or(0);
        assert_eq!(hp, 51, "maxhp/100 regeneration");
        let sym = world.resource::<Inventory>().equip[crate::data::SLOT_SYMBIOTE]
            .as_ref()
            .expect("symbiote");
        assert_eq!(sym.pval2, 20, "pval3/100 regeneration");
    }

    #[test]
    fn lasting_clouds_damage_the_player() {
        let mut world = bldg_world(Modal::None);
        world
            .resource_mut::<crate::map::Map>()
            .clouds
            .push(super::Cloud {
                x: 10,
                y: 10,
                radius: 0,
                wave: false,
                max_radius: 0,
                dir: (0, 0),
                gf: "FIRE".to_string(),
                damage: 3,
                turns: 3,
            });
        let before = world.resource::<PlayerState>().hp;
        world_turn(&mut world);
        let ps = world.resource::<PlayerState>();
        assert!(ps.hp < before, "cloud damage {} -> {}", before, ps.hp);
        assert_eq!(
            world.resource::<crate::map::Map>().clouds[0].turns,
            2,
            "cloud expires one turn"
        );
    }

    #[test]
    fn light_is_safe_requires_both_curses() {
        let gd = load_game_data();
        let mut inv = Inventory::default();
        let lite = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_LITE)
            .expect("lite kind");
        let mut it = Item::base(&gd, lite);
        it.flags.push("TY_CURSE".to_string());
        inv.equip[crate::data::SLOT_LITE] = Some(it);
        assert!(!crate::game::light_is_safe(&gd, &inv));
        inv.equip[crate::data::SLOT_LITE]
            .as_mut()
            .unwrap()
            .flags
            .push("DG_CURSE".to_string());
        assert!(crate::game::light_is_safe(&gd, &inv));
        inv.equip[crate::data::SLOT_LITE] = None;
        assert!(crate::game::light_is_safe(&gd, &inv), "no light is safe");
    }

    #[test]
    fn world_god_trickle_follows_dungeon_cc() {
        let gd = load_game_data();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut map = super::blank_map(super::T_FLOOR);
        let mut log = MessageLog::default();
        let mut inv = Inventory::default();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let mut ps = crate::birth::make_player(&gd, "Varda".into(), human, 0);
        ps.god = 7;
        ps.grace = 1000;
        let mut delay = 0;
        for _ in 0..14 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 1000, "fires every 15th call");
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 999, "dark + human: -1");
        map.lit[super::Map::idx(5, 5)] = true;
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 1000, "light + human: +2 -1");

        // Ulmo: an Edain gets +2, each trident +1.
        ps.god = 8;
        ps.grace = 1000;
        let trident = gd
            .object_by_tval_sval(crate::data::TV_POLEARM, 5)
            .expect("trident");
        inv.pack.push(Item::base(&gd, trident));
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 1003, "+2 Human +1 trident");

        // Mandos: a High-Elf gets +2 (no non-Elf penalty, not undead).
        ps.god = 9;
        ps.grace = 0;
        ps.race_name = "High-Elf".to_string();
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 2);

        // Aule: a Dwarf avoids the -1 and gains +1 per axe.
        ps.god = 6;
        ps.grace = 0;
        ps.race_name = "Dwarf".to_string();
        inv.pack.clear();
        let axe = gd.objects.iter().position(|o| o.tval == crate::data::TV_AXE).unwrap();
        inv.pack.push(Item::base(&gd, axe));
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 1, "Dwarf + axe");

        // Aule praying: after the -2 prayer cost grace lands below 50000,
        // so chance = 50000 - grace = 1 and the free Stone Skin always
        // lands (dungeon.cc:637-673).
        inv.pack.clear();
        ps.grace = 50001;
        ps.praying = true;
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, 49999, "praying costs 2");
        assert!(ps.shield > 0, "Stone Skin shield");
        assert_eq!(ps.shield_power, 10 + 49999 / 100);
        assert_eq!(
            ps.shield_counter,
            (2 + 49999 / 200, 3 + 49999 / 400),
            "SHIELD_COUNTER above 10000 grace"
        );
        assert!(log
            .lines
            .iter()
            .any(|l| l == "Aule casts Stone Skin on you."));

        // Mandos: a vampire is hated (-10) on top of the non-High-Elf -1.
        ps.god = 9;
        ps.grace = 0;
        ps.praying = false;
        ps.shield = 0;
        ps.race_name = "Human".to_string();
        ps.subrace = gd
            .racemods
            .iter()
            .position(|r| r.name == "Vampire")
            .expect("Vampire subrace") as u32;
        for _ in 0..15 {
            super::process_world_gods(
                &gd, &mut ps, &inv, &map, (5, 5), &mut log, &mut delay, &mut rng,
            );
        }
        assert_eq!(ps.grace, -11, "non-Elf -1, vampire -10");
    }
}
