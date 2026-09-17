//! Character creation ("birth"): name, race, class selection.
//! Text appears instantly -- no typing/fade-in effects.

use bevy::prelude::*;
use rand::Rng;

use crate::data::{self, GameData};
use crate::game::{PlayerState};
use crate::item::Inventory;
use crate::render::TileAssets;
use crate::town::ShopStocks;
use crate::AppState;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BirthStep {
    /// Offered when a save file exists: continue or start over.
    #[default]
    Menu,
    /// The module introduction (modules.cc tome_intro), two pages.
    Intro,
    Name,
    Race,
    /// Base subrace / race modifier (p_info S:, src/birth.cc dump_rmods).
    Subrace,
    Class,
    /// Class specialisation (p_info C:a:, src/birth.cc dump_specs).
    Spec,
    /// Deity choice, limited to the class/spec god list.
    God,
    /// Number of random quests (q_rand.cc: 0-98, default 20).
    Quests,
    /// Stat generation: autoroll minimums or point-buy base stats
    /// (birth.cc player_birth_aux_auto / player_birth_aux_point).
    Stats,
}

#[derive(Resource, Default)]
pub struct BirthFlow {
    pub step: BirthStep,
    pub name: String,
    pub race: Option<usize>,
    /// Chosen subrace (p_info S: record id).
    pub subrace: Option<usize>,
    pub class: Option<usize>,
    pub spec: Option<usize>,
    /// Chosen deity id (0 = Nobody) once the god step is done.
    pub god: Option<u32>,
    /// Quest-count input buffer.
    pub quests: String,
    /// Introduction page (0/1).
    pub intro_page: u8,
    /// Stats screen: the folded result of the autoroller.
    pub stats: [i32; 6],
    /// Autoroller minimums per stat (folded values).
    pub min_stats: [i32; 6],
    /// Point-buy base stats (10..=18 before race/class bonuses).
    pub point_stats: [i32; 6],
    /// Selected stat row on the stats screen.
    pub stat_cursor: usize,
    /// true = point buy, false = autoroll (birth.cc options->point_based).
    pub stat_mode: bool,
    /// Autoroller rounds spent on the current roll.
    pub stat_rounds: u64,
    /// Previous autoroll result: (folded stats, rounds). Filled by
    /// `save_prev_data` when rerolling; `load_prev_data` swaps it back in
    /// (birth.cc game->previous_char).
    pub previous: Option<([i32; 6], u64)>,
}

#[derive(Component)]
pub struct BirthRoot;
#[derive(Component)]
pub struct BirthText;

/// Runs every frame in the Birth state; spawns the UI once the tile assets
/// (font) are available. (The initial state's OnEnter fires *before* the
/// Startup schedule, so we cannot rely on OnEnter for this.)
pub fn ensure_birth_ui(
    mut commands: Commands,
    tiles: Option<Res<TileAssets>>,
    q: Query<(), With<BirthRoot>>,
) {
    if !q.is_empty() {
        return;
    }
    let Some(tiles) = tiles else { return };
    // With an existing save the flow starts at the continue menu.
    let step = if crate::save::exists() {
        BirthStep::Menu
    } else {
        BirthStep::Name
    };
    commands.insert_resource(BirthFlow {
        step,
        ..BirthFlow::default()
    });
    commands
        .spawn((
            BirthRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(48.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.04)),
        ))
        .with_children(|root| {
            root.spawn((
                BirthText,
                Text::new(""),
                TextFont {
                    font: tiles.font.clone().into(),
                    font_size: 16.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.85)),
            ));
        });
}

pub fn cleanup_birth(mut commands: Commands, q: Query<Entity, With<BirthRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn letter_index(k: KeyCode) -> Option<usize> {
    use KeyCode::*;
    Some(match k {
        KeyA => 0,
        KeyB => 1,
        KeyC => 2,
        KeyD => 3,
        KeyE => 4,
        KeyF => 5,
        KeyG => 6,
        KeyH => 7,
        KeyI => 8,
        KeyJ => 9,
        KeyK => 10,
        KeyL => 11,
        KeyM => 12,
        KeyN => 13,
        KeyO => 14,
        KeyP => 15,
        KeyQ => 16,
        KeyR => 17,
        KeyS => 18,
        KeyT => 19,
        KeyU => 20,
        KeyV => 21,
        KeyW => 22,
        KeyX => 23,
        KeyY => 24,
        KeyZ => 25,
        _ => return None,
    })
}

/// Subraces legal for a race (p_info S:A: choice masks; every race's
/// list contains the record-0 Normal entry).
pub fn allowed_subraces(gd: &GameData, race_idx: usize) -> Vec<usize> {
    let race = &gd.races[race_idx];
    (0..gd.racemods.len())
        .filter(|&mi| gd.racemods[mi].races.iter().any(|n| n == &race.name))
        .collect()
}

/// Classes available to a race + subrace.  The subrace can open extra
/// classes (S:C:A:) and forbid some (S:C:F:); birth.cc:1627
/// `(race.choice | mod.pclass) & ~mod.mclass`.
fn allowed_classes(gd: &GameData, race_idx: usize, subrace_idx: usize) -> Vec<usize> {
    let race = &gd.races[race_idx];
    let mod_ = gd.racemods.get(subrace_idx);
    (0..gd.classes.len())
        .filter(|&ci| {
            let name = &gd.classes[ci].name;
            let base = race.classes.iter().any(|n| n == name);
            let opens = mod_.is_some_and(|m| m.classes.iter().any(|n| n == name));
            let forbidden = mod_.is_some_and(|m| m.forbidden_classes.iter().any(|n| n == name));
            (base || opens) && !forbidden
        })
        .collect()
}

/// (class.gods | spec.gods) bit set, as in src/birth.cc:1828.
fn god_mask(gd: &GameData, class_idx: usize, spec_idx: usize) -> u32 {
    let class = &gd.classes[class_idx];
    let spec = &class.specs[spec_idx];
    let mut mask = 0u32;
    for name in class.gods.iter().chain(spec.gods.iter()) {
        if name == "All Gods" {
            mask |= 0xFFFF_FFFF;
        } else if let Some(id) = crate::spell::god_id_full(name) {
            mask |= 1 << id;
        }
    }
    mask
}

/// Gods (0 = Nobody) allowed by the chosen race + class + specialisation.
/// A race with R:G:NO_GOD (Maia) can never have a god; an empty mask also
/// means no god choice at all (atheist).
pub fn allowed_gods(
    gd: &GameData,
    race_idx: usize,
    subrace_idx: usize,
    class_idx: usize,
    spec_idx: usize,
) -> Vec<u32> {
    let race = &gd.races[race_idx];
    let subrace = gd.racemods.get(subrace_idx);
    let atheist = race.player_flags.iter().any(|f| f == "NO_GOD")
        || subrace.is_some_and(|m| m.player_flags.iter().any(|f| f == "NO_GOD"));
    if atheist {
        return Vec::new();
    }
    gods_from_mask(god_mask(gd, class_idx, spec_idx))
}

/// Gods allowed for a live character (birth/temple, gods.cc follow_god).
/// Unknown legacy race/class/spec data is treated as unrestricted.
pub fn allowed_gods_for(gd: &GameData, ps: &PlayerState) -> Vec<u32> {
    if crate::game::player_has_flag(gd, ps, "NO_GOD") {
        return Vec::new();
    }
    let Some(ci) = gd
        .classes
        .iter()
        .position(|c| c.name == ps.base_class_name())
    else {
        return gods_from_mask(0xFFFF_FFFF);
    };
    let Some(si) = gd.classes[ci]
        .specs
        .iter()
        .position(|s| s.name == ps.class_name)
    else {
        return gods_from_mask(0xFFFF_FFFF);
    };
    gods_from_mask(god_mask(gd, ci, si))
}

fn gods_from_mask(mask: u32) -> Vec<u32> {
    (0..=9).filter(|g| mask & (1 << g) != 0).collect()
}

/// The full name of a god id for the birth screen.
pub fn god_name_full(god: u32) -> &'static str {
    match god {
        0 => "Nobody",
        1 => "Eru Iluvatar",
        2 => "Manwe Sulimo",
        3 => "Tulkas",
        4 => "Melkor Bauglir",
        5 => "Yavanna Kementari",
        6 => "Aule the Smith",
        7 => "Varda Elentari",
        8 => "Ulmo",
        9 => "Mandos",
        _ => "Nobody",
    }
}

/// Build a character of the given base class without a specialisation
/// choice: the class' first C:a: record (the file order puts the class'
/// own name first, e.g. Warrior/Mage/...). Convenience for callers that
/// do not care about the spec (mostly tests).
#[allow(dead_code)]
/// `create_random_name` (birth.cc:115): the three-syllable name generator.
pub fn create_random_name(rng: &mut impl rand::Rng) -> String {
    const SYL1: &[&str] = &[
        "Ab", "Ac", "Ad", "Af", "Agr", "Ast", "As", "Al", "Adw", "Adr", "Ar",
        "B", "Br", "C", "Cr", "Ch", "Cad", "D", "Dr", "Dw", "Ed", "Eth", "Et",
        "Er", "El", "Eow", "F", "Fr", "G", "Gr", "Gw", "Gal", "Gl", "H", "Ha",
        "Ib", "Jer", "K", "Ka", "Ked", "L", "Loth", "Lar", "Leg", "M", "Mir",
        "N", "Nyd", "Ol", "Oc", "On", "P", "Pr", "R", "Rh", "S", "Sev", "T",
        "Tr", "Th", "V", "Y", "Z", "W", "Wic",
    ];
    const SYL2: &[&str] = &[
        "a", "ae", "au", "ao", "are", "ale", "ali", "ay", "ardo", "e", "ei",
        "ea", "eri", "era", "ela", "eli", "enda", "erra", "i", "ia", "ie",
        "ire", "ira", "ila", "ili", "ira", "igo", "o", "oa", "oi", "oe",
        "ore", "u", "y",
    ];
    const SYL3: &[&str] = &[
        "a", "and", "b", "bwyn", "baen", "bard", "c", "ctred", "cred", "ch",
        "can", "d", "dan", "don", "der", "dric", "dfrid", "dus", "f", "g",
        "gord", "gan", "l", "li", "lgrin", "lin", "lith", "lath", "loth",
        "ld", "ldric", "ldan", "m", "mas", "mos", "mar", "mond", "n",
        "nydd", "nidd", "nnon", "nwan", "nyth", "nad", "nn", "nnor", "nd",
        "p", "r", "ron", "rd", "s", "sh", "seth", "sean", "t", "th", "tha",
        "tlan", "trem", "tram", "v", "vudd", "w", "wan", "win", "wyn", "wyr",
        "wyr", "wyth",
    ];
    format!(
        "{}{}{}",
        SYL1[rng.gen_range(0..SYL1.len())],
        SYL2[rng.gen_range(0..SYL2.len())],
        SYL3[rng.gen_range(0..SYL3.len())]
    )
}

pub fn make_player(gd: &GameData, name: String, race_idx: usize, class_idx: usize) -> PlayerState {
    make_player_spec(gd, name, race_idx, 0, class_idx, 0)
}

/// birth.cc get_stats: 5 + 1d3 + 1d4 + 1d5 per stat with the 42 < sum < 57
/// re-roll, then the race/subrace/class adjustments. The port folds the
/// original 18/xx steps into single integers (19 = 18/10, ... 40 = 18/220).
fn roll_stats(
    race: &data::RaceDef,
    subrace: Option<&data::RaceModDef>,
    class: &data::ClassDef,
    rng: &mut impl Rng,
) -> [i32; 6] {
    let mut dice = [0i32; 18];
    loop {
        let mut sum = 0;
        for (i, d) in dice.iter_mut().enumerate() {
            *d = rng.gen_range(1..=3 + (i % 3) as i32);
            sum += *d;
        }
        if sum > 42 && sum < 57 {
            break;
        }
    }
    let mut stats = [0; 6];
    for i in 0..6 {
        let roll = 5 + dice[3 * i] + dice[3 * i + 1] + dice[3 * i + 2];
        let bonus = race.stats[i] + subrace.map(|m| m.stats[i]).unwrap_or(0) + class.stats[i];
        // birth.cc get_stats uses modify_stat_value (adjust_stat's twin).
        stats[i] = crate::game::modify_stat_value(roll, bonus);
    }
    stats
}

/// birth.cc birth_stat_costs: the point-buy cost of base stat values
/// 10..18 (player_birth_aux_point, birth.cc:2082).
pub const BIRTH_STAT_COSTS: [i32; 9] = [0, 1, 2, 4, 7, 11, 16, 22, 30];

/// player_birth_aux_point's gold rule (birth.cc:2168): unused points pay
/// 100 gp each on top of 100, capped at 600.
pub fn point_buy_gold(cost: i32) -> i32 {
    (100 * (48 - cost) + 100).min(600)
}

/// The point-buy total of a base stat array (values 10..=18).
pub fn point_buy_cost(stats: &[i32; 6]) -> i32 {
    stats
        .iter()
        .map(|s| BIRTH_STAT_COSTS[(*s - 10).clamp(0, 8) as usize])
        .sum()
}

/// player_birth_aux_auto (birth.cc:2250): re-roll character stats until
/// every stat (after race/subrace/class bonuses, folded values) reaches
/// its requested minimum or the original 1,000,000-round cap is hit.
/// Returns (stats, rounds, per-stat success counts).
pub fn auto_roll(
    race: &data::RaceDef,
    subrace: Option<&data::RaceModDef>,
    class: &data::ClassDef,
    min_stats: &[i32; 6],
    rng: &mut impl Rng,
) -> ([i32; 6], u64, [u64; 6]) {
    let mut matches = [0u64; 6];
    let mut rounds = 0u64;
    loop {
        let stats = roll_stats(race, subrace, class, rng);
        rounds += 1;
        let mut accept = true;
        for i in 0..6 {
            if stats[i] >= min_stats[i] {
                matches[i] += 1;
            } else {
                accept = false;
            }
        }
        if accept || rounds >= 1_000_000 {
            return (stats, rounds, matches);
        }
    }
}

/// Display order of the six stats (birth.cc stat_names).
pub const STAT_NAMES: [&str; 6] = ["STR", "INT", "WIS", "DEX", "CON", "CHR"];

/// The race/subrace/class folded stat bonuses of the current selection.
fn birth_stat_bonuses(flow: &BirthFlow, gd: &GameData) -> Option<[i32; 6]> {
    let race = gd.races.get(flow.race?)?;
    let subrace = gd.racemods.get(flow.subrace.unwrap_or(0));
    let class = gd.classes.get(flow.class?)?;
    let mut b = [0; 6];
    for i in 0..6 {
        b[i] = race.stats[i] + subrace.map(|m| m.stats[i]).unwrap_or(0) + class.stats[i];
    }
    Some(b)
}

/// Fold point-buy base stats (10..18) through the race/subrace/class
/// bonuses the way birth.cc update_stuff does.
pub fn fold_point_stats(flow: &BirthFlow, gd: &GameData) -> [i32; 6] {
    let mut out = flow.point_stats;
    if let Some(b) = birth_stat_bonuses(flow, gd) {
        for i in 0..6 {
            out[i] = crate::game::modify_stat_value(out[i], b[i]);
        }
    }
    out
}

/// birth.cc save_prev_data: stash the current roll for the 'v' (previous)
/// key. The original copies race/rmod/class/spec/quests/god/grace/au,
/// stat_max[6] and luck into game->previous_char; here the values that can
/// change between rolls are the folded stats and the round count.
pub fn save_prev_data(flow: &mut BirthFlow) {
    flow.previous = Some((flow.stats, flow.stat_rounds));
}

/// birth.cc load_prev_data: bring back the stashed roll. With `save` the
/// current roll is swapped into the slot, so the key toggles between the
/// last two rolls (the original stores/restores au, the six stats and
/// luck the same way).
pub fn load_prev_data(flow: &mut BirthFlow, save: bool) {
    let Some(prev) = flow.previous else {
        return;
    };
    let temp = (flow.stats, flow.stat_rounds);
    flow.stats = prev.0;
    flow.stat_rounds = prev.1;
    if save {
        flow.previous = Some(temp);
    }
}

/// Run the autoroller with the current minimums (player_birth_aux_auto).
pub fn roll_auto_stats(flow: &mut BirthFlow, gd: &GameData) {
    let Some(race) = gd.races.get(flow.race.unwrap_or(0)) else {
        return;
    };
    let subrace = gd.racemods.get(flow.subrace.unwrap_or(0));
    let Some(class) = gd.classes.get(flow.class.unwrap_or(0)) else {
        return;
    };
    let mut rng = crate::rng::current();
    let (stats, rounds, _) = auto_roll(race, subrace, class, &flow.min_stats, &mut rng);
    flow.stats = stats;
    flow.stat_rounds = rounds;
}

/// birth.cc roll_player_hp: pre-roll the whole 50-level hit-die array so
/// the cumulative total at level 50 sits inside the 3/8..5/8 band. Entry 0
/// is always the maximal level-1 die.
fn roll_player_hp(hitdie: i32, rng: &mut impl Rng) -> Vec<i32> {
    let hd = hitdie.max(1);
    let min_value = (50 * (hd - 1) * 3) / 8 + 50;
    let max_value = (50 * (hd - 1) * 5) / 8 + 50;
    let mut rolls = vec![hd];
    loop {
        rolls.truncate(1);
        while rolls.len() < 50 {
            rolls.push(rng.gen_range(1..=hd));
        }
        let total: i32 = rolls.iter().sum();
        if total >= min_value && total <= max_value {
            break;
        }
    }
    rolls
}

/// birth.cc get_money: randint(100) + 300 minus the stats' cost, minimum
/// 100. Stat values above 18 are the folded 18/xx steps (23 = 18/50).
fn roll_gold(stats: &[i32; 6], rng: &mut impl Rng) -> i32 {
    let mut gold = rng.gen_range(1..=100) + 300;
    for &s in stats {
        if s >= 23 {
            gold -= 300;
        } else if s >= 20 {
            gold -= 200;
        } else if s > 18 {
            gold -= 150;
        } else {
            gold -= (s - 8) * 10;
        }
    }
    gold.max(100)
}

pub fn make_player_spec(
    gd: &GameData,
    name: String,
    race_idx: usize,
    subrace_idx: usize,
    class_idx: usize,
    spec_idx: usize,
) -> PlayerState {
    make_player_spec_with_stats(gd, name, race_idx, subrace_idx, class_idx, spec_idx, None, None)
}

/// make_player_spec with the birth stats screen's chosen values: the
/// folded stat array and, for point-bought characters, the gold from
/// unused points (birth.cc player_birth_aux_point).
#[allow(clippy::too_many_arguments)]
pub fn make_player_spec_with_stats(
    gd: &GameData,
    name: String,
    race_idx: usize,
    subrace_idx: usize,
    class_idx: usize,
    spec_idx: usize,
    stats_override: Option<[i32; 6]>,
    gold_override: Option<i32>,
) -> PlayerState {
    let race = &gd.races[race_idx];
    let subrace = gd.racemods.get(subrace_idx);
    let class = &gd.classes[class_idx];
    let spec = &class.specs[spec_idx];
    let mut rng = crate::rng::current();
    let stats = stats_override.unwrap_or_else(|| roll_stats(race, subrace, class, &mut rng));
    let hitdie =
        (race.hitdie as i32 + subrace.map(|m| m.hitdie).unwrap_or(0) + class.hitdie as i32).max(1);
    // get_extra sets mhp = hitdie; the CON bonus is folded in by the
    // update_stuff PU_HP pass (calc_hitpoints) right below.
    let max_hp = hitdie.max(1);
    let mana_mult = subrace.map(|m| m.mana).unwrap_or(100);
    let mana_user = class.mana > 0
        || class
            .skills
            .iter()
            .chain(spec.skills.iter())
            .chain(subrace.map(|m| m.skills.as_slice()).unwrap_or(&[]))
            .any(|m| m.skill == "Magic" && (m.base != 0 || m.gain != 0));
    let spell_bonus = ((stats[1].max(stats[2]) - 10) / 2).max(0);
    let max_mana = if mana_user {
        (1 + spell_bonus) * mana_mult / 100
    } else {
        0
    };
    let mut ps = PlayerState {
        name,
        race_name: race.name.clone(),
        class_name: spec.name.clone(),
        base_class: class.name.clone(),
        subrace: subrace_idx as u32,
        mana_mult,
        astral: false,
        fates: Vec::new(),
        no_mortal: false,
        black_breath: false,
        hp_mod: 0,
        power_grow_mold: false,
        extra_powers: Vec::new(),
        known_powers: Vec::new(),
        known_dungeon_towns: Vec::new(),
        inscriptions: Vec::new(),
        flavor_seed: rand::random(),
        // init_randart (birth.cc:471): one colour per legacy junkart roll
        // (randint(15)), fixed for the whole game and saved with it.
        junkart_colors: (0..gd.randarts.junk_s.len())
            .map(|_| rng.gen_range(1..=15) as u8)
            .collect(),
        level: 1,
        exp: 0,
        exp_factor: (race.exp as i32 + subrace.map(|m| m.exp).unwrap_or(0) + class.exp as i32)
            .max(1) as u64,
        hp: max_hp,
        max_hp,
        hp_rolls: vec![hitdie],
        hp_planned: roll_player_hp(hitdie, &mut rng),
        mana: max_mana,
        max_mana,
        mana_user,
        hitdie,
        stats,
        ac: 0,
        gold: gold_override.unwrap_or_else(|| roll_gold(&stats, &mut rng)),
        depth: 0,
        dungeon: 0,
        wild_x: gd.world.start.0,
        wild_y: gd.world.start.1,
        wild_mode: false,
        recall_dungeon: 4,
        quest_origin: None,
        recall_depth: 1,
        word_recall: 0,
        allow_one_death: 0,
        turn: 0,
        max_depth: 0,
        kills: std::collections::HashMap::new(),
        unique_seen: std::collections::HashSet::new(),
        // player_wipe's "Hack -- Well fed player": PY_FOOD_FULL - 1.
        food: crate::game::FOOD_FULL - 1,
        cut: 0,
        stun: 0,
        poison: 0,
        fear: 0,
        blind: 0,
        confuse: 0,
        skill_points: 0,
        skills: std::collections::HashMap::new(),
        skill_mods: std::collections::HashMap::new(),
        skill_dev: std::collections::HashSet::new(),
        skill_invest: std::collections::HashMap::new(),
        skill_session: None,
        skill_hidden: std::collections::HashSet::new(),
        abilities: Vec::new(),
        melee_style: crate::skill::SK_MASTERY,
        icky_wield: false,
        cumber_glove: false,
        piercing: false,
        undead_form: None,
        stat_base: stats,
        exp_drained: 0,
        exp_frac: 0,
        max_plv: 1,
        slow: 0,
        paralyze: 0,
        hero: 0,
        shero: 0,
        blessed: 0,
        holy: 0,
        tim_deadly: 0,
        tim_roots: 0,
        tim_roots_ac: 0,
        tim_roots_dam: 0,
        melkor_sacrifice: 0,
        luck_base: race.luck
            + subrace.map(|m| m.luck).unwrap_or(0)
            + crate::rng::current().gen_range(-5..=5),
        fast: 0,
        speed_factor: 10,
        lightspeed: 0,
        protevil: 0,
        esp_timer: 0,
        resist_timer: 0,
        brand_shots: 0,
        parasite: None,
        ring_destroyed: false,
        ring_worn: false,
        ring_lives: 0,
        died_from: String::new(),
        maintain_sum: 0,
        god: 0,
        grace: 0,
        praying: false,
        god_quests: 0,
        god_quests_given: 0,
        // q_god.cc quest_god_birth_objects_hook: mindepth 1, maxdepth 4
        // (five levels) before the first quest is granted.
        god_temple_mindepth: 1,
        god_min_plev: 0,
        god_failed: false,
        relic_depth: 0,
        relic_quest: false,
        relic_tries: 0,
        possessed: None,
        disembodied: false,
        god_boon: [0; 6],
        god_mana_boon: 0,
        // The exact calc_mana base is applied on the first upkeep
        // (skill::sync_magic_mana); this tracks the currently applied
        // base so the sync only ever applies the difference.
        magic_mana_bonus: max_mana,
        pending_skill_gain: None,
        wizard: false,
        noscore: 0,
        life_boon: 0,
        random_towns: Vec::new(),
        sanity: 0,
        max_sanity: 0,
        image: 0,
        sanity_dead: false,
        corruptions: Vec::new(),
        corrupt_anti_teleport_stopped: false,
        mimic_form: None,
        mimic_level: 0,
        mimic_turns: 0,
        mimic_extra: 0,
        mimic_extra_turns: 0,
        invis_turns: 0,
        invis_power: 0,
        antimagic_field: false,
        shield: 0,
        shield_power: 0,
        absorb_soul: 0,
        random_spells: Vec::new(),
        thaum_level: 0,
        dripping_tread: 0,
        precog_timer: 0,
        tim_project: 0,
        tim_project_gf: String::new(),
        tim_project_dam: 0,
        tim_project_rad: 0,
        tim_reflect: 0,
        shield_counter: (0, 0),
        shield_fire: (0, 0),
        shield_great_fire: false,
        shield_fear: false,
        no_breeders: 0,
        tim_invis: 0,
        tim_infra: 0,
        tim_poison: 0,
        tim_thunder: 0,
        thunder_dd: 0,
        thunder_ds: 0,
        tim_fly: 0,
        tim_ffall: 0,
        prob_travel: 0,
        tim_regen: 0,
        tim_regen_pow: 0,
        tim_wraith: 0,
        oppose_fire: 0,
        oppose_cold: 0,
        oppose_elec: 0,
        oppose_acid: 0,
        oppose_pois: 0,
        oppose_cc: 0,
        strike: 0,
        tim_lite: 0,
        disrupt_shield: 0,
        invuln: 0,
        music_extra: None,
        spellbinder: None,
        inertia_spell: None,
        inertia_timer: 0,
        inertia_cost: 0,
        inertia_penalty_applied: 0,
        last_teleport: None,
        control: None,
        control_pos: None,
        control_dir: (0, 0),
        control_power: None,
        spellbinder_pending: false,
        inertia_pending: false,
        tactic: 4,
        movement: 4,
        visited_dungeons: std::collections::HashMap::new(),
    };
    // Skill tree values from race/class/general modifiers, level-1
    // abilities (p_info R:b:/C:b:), and the default melee style.
    crate::skill::compute_skills(&mut ps, gd);
    crate::skill::apply_level_abilities(&mut ps, gd, 1, None);
    // calc_hitpoints (xtra1.cc): level-1 mhp = hitdie + adj_con_mhp/2
    // with the level+1 floor, exactly the update_stuff PU_HP pass
    // player_birth runs after get_extra (and after the skill pass, so the
    // Sorcery penalty sees the trained skills).
    crate::game::recalc_max_hp(&mut ps, &Inventory::default(), gd);
    ps.hp = ps.max_hp;
    crate::game::calc_sanity(&mut ps);
    ps.sanity = ps.max_sanity;
    ps
}

/// Give one p_info object proto (src/birth.cc outfit_obj): the number is
/// rolled as `dd`d`ds`, the pval is set when nonzero, and the item is
/// auto-identified (own gear).
fn give_proto(
    inv: &mut Inventory,
    gd: &GameData,
    proto: &data::ObjectProto,
    rng: &mut impl rand::Rng,
) {
    let Some(def) = gd.object_by_tval_sval(proto.tval, proto.sval) else {
        return;
    };
    let mut item = crate::item::Item::base(gd, def);
    item.identified = true;
    item.count = (0..proto.dd.max(1))
        .map(|_| rng.gen_range(1..=proto.ds.max(1)) as u32)
        .sum();
    if proto.pval != 0 {
        item.pval = proto.pval;
    }
    inv.learn(def);
    inv.pack.push(item);
}

/// Give a pre-inscribed spellbook (TV_BOOK 255, src/birth.cc
/// player_outfit_spellbook). The book teaches its spell through the
/// custom-tome spell list.
fn give_spellbook(inv: &mut Inventory, gd: &GameData, spell_name: &str) {
    let Some(def) = gd.object_by_tval_sval(data::TV_BOOK, 255) else {
        return;
    };
    let mut item = crate::item::Item::base(gd, def);
    item.identified = true;
    item.spells.push(spell_name.to_string());
    inv.learn(def);
    inv.pack.push(item);
}

/// Starting equipment: race/class/spec object protos plus the fixed kit
/// from src/birth.cc player_outfit (adventurer guide, rations, torches),
/// and the per-class starting spellbook / mimic cloak. A weapon, light
/// and (for Bards) instrument are auto-equipped for convenience.
pub fn make_inventory(
    gd: &GameData,
    race_idx: usize,
    subrace_idx: usize,
    class_idx: usize,
    spec_idx: usize,
) -> Inventory {
    let mut inv = Inventory::default();
    let mut rng = crate::rng::current();
    let race = &gd.races[race_idx];
    let subrace = gd.racemods.get(subrace_idx);
    let class = &gd.classes[class_idx];
    let spec = &class.specs[spec_idx];

    // src/birth.cc player_outfit: everyone starts with these.
    if let Some(d) = gd.object_by_tval_sval(data::TV_PARCHMENT, 20) {
        let mut item = crate::item::Item::base(gd, d);
        item.identified = true;
        inv.learn(d);
        inv.pack.push(item);
    }
    // Class-specific starting spellbooks (src/birth.cc).
    match spec.name.as_str() {
        "Ranger" => give_spellbook(&mut inv, gd, "Phase Door"),
        "Geomancer" => give_spellbook(&mut inv, gd, "Geyser"),
        "Priest(Eru)" => give_spellbook(&mut inv, gd, "See the Music"),
        "Priest(Manwe)" => give_spellbook(&mut inv, gd, "Manwe's Blessing"),
        "Druid" => give_spellbook(&mut inv, gd, "Charm Animal"),
        "Dark-Priest" => give_spellbook(&mut inv, gd, "Curse"),
        "Paladin" => give_spellbook(&mut inv, gd, "Divine Aim"),
        _ => {}
    }
    // Mimics start with a Cloak of Mimicry locked to the Mouse shape
    // (src/birth.cc resolve_mimic_name("Mouse")).
    if spec.name == "Mimic" {
        if let Some(d) = gd.object_by_tval_sval(data::TV_CLOAK, 100) {
            let mut item = crate::item::Item::base(gd, d);
            item.identified = true;
            item.pval2 = crate::mimic::MIMIC_MOUSE as i32;
            inv.learn(d);
            inv.pack.push(item);
        }
    }
    // Fixed kit given before the race/class protos in the original
    // (birth.cc player_outfit gives the guide, books, food and torches,
    // then outfit_objs() walks the four proto lists last).
    let qty = rng.gen_range(3..=7);
    for name in ["Ration of Food"] {
        if let Some(d) = gd.object_by_name(name) {
            let mut item = crate::item::Item::base(gd, d);
            item.identified = true;
            item.count = qty;
            inv.learn(d);
            inv.pack.push(item);
        }
    }
    if let Some(d) = gd.object_by_name("Wooden Torch") {
        let mut item = crate::item::Item::base(gd, d);
        item.identified = true;
        // The torch count is an independent roll (birth.cc:967).
        item.count = rng.gen_range(3..=7);
        item.timeout = rng.gen_range(3..=7) * 500;
        inv.learn(d);
        inv.pack.push(item);
    }
    for proto in race
        .objects
        .iter()
        .chain(subrace.map(|m| m.objects.as_slice()).unwrap_or(&[]))
        .chain(class.objects.iter())
        .chain(spec.objects.iter())
    {
        give_proto(&mut inv, gd, proto, &mut rng);
    }
    // Convenience auto-equip: a weapon (a Bard's instrument, a
    // Demonologist's demon blade), a light.
    let bard = spec.name == "Bard";
    if let Some(i) = inv.pack.iter().position(|it| {
        let o = &gd.objects[it.def];
        if bard {
            o.tval == data::TV_INSTRUMENT
        } else {
            matches!(o.tval, 19..=24)
                || o.tval == data::TV_MSTAFF
                || data::slot_of_item(o) == Some(data::SLOT_WEAPON)
        }
    }) {
        inv.equip[data::SLOT_WEAPON] = Some(inv.pack.remove(i));
    }
    if let Some(i) = inv
        .pack
        .iter()
        .position(|it| gd.objects[it.def].tval == data::TV_LITE)
    {
        inv.equip[data::SLOT_LITE] = Some(inv.pack.remove(i));
    }
    inv
}

pub fn birth_input(
    keys: Res<ButtonInput<KeyCode>>,
    gd: Res<GameData>,
    mut flow: ResMut<BirthFlow>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    match flow.step {
        BirthStep::Menu => {
            // Enter: restore the saved game. N: start a new character.
            if keys.just_pressed(KeyCode::Enter) {
                if let Some(save) = crate::save::load() {
                    commands.insert_resource(save.ps.clone());
                    commands.insert_resource(save.inv.clone());
                    commands.insert_resource(save.plot.clone());
                    commands.insert_resource(save.stocks.clone());
                    commands.insert_resource(save.levels.clone());
                    commands.insert_resource(crate::item::CreatedArtifacts(save.created.clone()));
                    commands.insert_resource(crate::save::PendingLoad(save));
                    next.set(AppState::Playing);
                    return;
                }
                // Save unreadable: fall through to the introduction.
                flow.step = BirthStep::Intro;
                flow.intro_page = 0;
            } else if keys.just_pressed(KeyCode::KeyN) {
                flow.step = BirthStep::Intro;
                flow.intro_page = 0;
            }
        }
        BirthStep::Intro => {
            if keys.get_just_pressed().next().is_some() {
                if flow.intro_page == 0 {
                    flow.intro_page = 1;
                } else {
                    flow.step = BirthStep::Name;
                }
            }
        }
        BirthStep::Name => {
            let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
            for k in keys.get_just_pressed() {
                match k {
                    KeyCode::Backspace => {
                        flow.name.pop();
                    }
                    KeyCode::Enter => {
                        if flow.name.trim().is_empty() {
                            flow.name = "Player".to_string();
                        }
                        flow.step = BirthStep::Race;
                    }
                    KeyCode::Space => flow.name.push(' '),
                    KeyCode::Minus => flow.name.push('-'),
                    k => {
                        if let Some(i) = letter_index(*k) {
                            if flow.name.len() < 20 {
                                let c = (b'a' + i as u8) as char;
                                flow.name
                                    .push(if shift { c.to_ascii_uppercase() } else { c });
                            }
                        }
                    }
                }
            }
        }
        BirthStep::Race => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Name;
                return;
            }
            for k in keys.get_just_pressed() {
                if let Some(i) = letter_index(*k) {
                    if i < gd.races.len() {
                        flow.race = Some(i);
                        flow.subrace = None;
                        let subs = allowed_subraces(&gd, i);
                        // A single legal subrace is chosen instantly
                        // (birth.cc: race_mod_info.size() == 1 branch).
                        if subs.len() == 1 {
                            flow.subrace = Some(subs[0]);
                            flow.step = BirthStep::Class;
                        } else {
                            flow.step = BirthStep::Subrace;
                        }
                    }
                }
            }
        }
        BirthStep::Subrace => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Race;
                return;
            }
            let Some(race_idx) = flow.race else {
                flow.step = BirthStep::Race;
                return;
            };
            let allowed = allowed_subraces(&gd, race_idx);
            for k in keys.get_just_pressed() {
                if let Some(i) = letter_index(*k) {
                    if i < allowed.len() {
                        flow.subrace = Some(allowed[i]);
                        flow.class = None;
                        flow.spec = None;
                        flow.god = None;
                        flow.step = BirthStep::Class;
                        return;
                    }
                }
            }
        }
        BirthStep::Class => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Subrace;
                return;
            }
            let Some(race_idx) = flow.race else {
                flow.step = BirthStep::Race;
                return;
            };
            let allowed = allowed_classes(&gd, race_idx, flow.subrace.unwrap_or(0));
            for k in keys.get_just_pressed() {
                if let Some(i) = letter_index(*k) {
                    if i < allowed.len() {
                        flow.class = Some(allowed[i]);
                        flow.spec = None;
                        flow.god = None;
                        flow.quests = "20".to_string();
                        flow.step = BirthStep::Spec;
                        return;
                    }
                }
            }
        }
        BirthStep::Spec => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Class;
                return;
            }
            let Some(class_idx) = flow.class else {
                flow.step = BirthStep::Class;
                return;
            };
            let n = gd.classes[class_idx].specs.len();
            for k in keys.get_just_pressed() {
                if let Some(i) = letter_index(*k) {
                    if i < n {
                        flow.spec = Some(i);
                        let gods = allowed_gods(
                            &gd,
                            flow.race.unwrap_or(0),
                            flow.subrace.unwrap_or(0),
                            class_idx,
                            i,
                        );
                        if gods.len() == 1 {
                            flow.god = Some(gods[0]);
                            flow.step = BirthStep::Quests;
                        } else if gods.is_empty() {
                            flow.god = Some(0);
                            flow.step = BirthStep::Quests;
                        } else {
                            flow.god = None;
                            flow.step = BirthStep::God;
                        }
                        return;
                    }
                }
            }
        }
        BirthStep::God => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Spec;
                return;
            }
            let (Some(class_idx), Some(spec_idx)) = (flow.class, flow.spec) else {
                flow.step = BirthStep::Spec;
                return;
            };
            let gods = allowed_gods(
                &gd,
                flow.race.unwrap_or(0),
                flow.subrace.unwrap_or(0),
                class_idx,
                spec_idx,
            );
            for k in keys.get_just_pressed() {
                if let Some(i) = letter_index(*k) {
                    if i < gods.len() {
                        flow.god = Some(gods[i]);
                        flow.step = BirthStep::Quests;
                        return;
                    }
                }
            }
        }
        BirthStep::Quests => {
            if keys.just_pressed(KeyCode::Escape) {
                let multi = match (flow.class, flow.spec) {
                    (Some(ci), Some(si)) => {
                        allowed_gods(
                            &gd,
                            flow.race.unwrap_or(0),
                            flow.subrace.unwrap_or(0),
                            ci,
                            si,
                        )
                        .len()
                            > 1
                    }
                    _ => false,
                };
                flow.step = if multi {
                    BirthStep::God
                } else {
                    BirthStep::Spec
                };
                return;
            }
            for k in keys.get_just_pressed() {
                match k {
                    KeyCode::Backspace => {
                        flow.quests.pop();
                    }
                    KeyCode::Enter => {
                        let raw = flow.quests.trim();
                        let n = if raw.is_empty() {
                            20
                        } else {
                            raw.parse::<i32>().unwrap_or(20).clamp(0, 98)
                        };
                        // The stats screen follows (birth.cc
                        // player_birth_aux: point buy or autoroll).
                        flow.stats = [0; 6];
                        flow.min_stats = [10; 6];
                        flow.point_stats = [10; 6];
                        flow.stat_cursor = 0;
                        flow.stat_rounds = 0;
                        flow.stat_mode = false;
                        roll_auto_stats(&mut flow, &gd);
                        flow.step = BirthStep::Stats;
                        let _ = n;
                        return;
                    }
                    k @ (KeyCode::Digit0
                    | KeyCode::Digit1
                    | KeyCode::Digit2
                    | KeyCode::Digit3
                    | KeyCode::Digit4
                    | KeyCode::Digit5
                    | KeyCode::Digit6
                    | KeyCode::Digit7
                    | KeyCode::Digit8
                    | KeyCode::Digit9) => {
                        if flow.quests.len() < 2 {
                            let d = match k {
                                KeyCode::Digit0 => '0',
                                KeyCode::Digit1 => '1',
                                KeyCode::Digit2 => '2',
                                KeyCode::Digit3 => '3',
                                KeyCode::Digit4 => '4',
                                KeyCode::Digit5 => '5',
                                KeyCode::Digit6 => '6',
                                KeyCode::Digit7 => '7',
                                KeyCode::Digit8 => '8',
                                _ => '9',
                            };
                            flow.quests.push(d);
                        }
                    }
                    _ => {}
                }
            }
        }
        BirthStep::Stats => {
            if keys.just_pressed(KeyCode::Escape) {
                flow.step = BirthStep::Quests;
                return;
            }
            for k in keys.get_just_pressed() {
                match k {
                    KeyCode::KeyA => flow.stat_mode = false,
                    KeyCode::KeyP => flow.stat_mode = true,
                    KeyCode::KeyR => {
                        if !flow.stat_mode {
                            // Keep the current roll for 'v' (save_prev_data
                            // runs before the next roll in the original).
                            save_prev_data(&mut flow);
                            roll_auto_stats(&mut flow, &gd);
                        }
                    }
                    KeyCode::KeyV => {
                        // Previous character (birth.cc 'p', port key 'v'):
                        // only offered once a roll has been stashed.
                        if flow.previous.is_some() {
                            load_prev_data(&mut flow, true);
                        }
                    }
                    KeyCode::ArrowUp | KeyCode::Digit8 => {
                        flow.stat_cursor = (flow.stat_cursor + 5) % 6;
                    }
                    KeyCode::ArrowDown | KeyCode::Digit2 => {
                        flow.stat_cursor = (flow.stat_cursor + 1) % 6;
                    }
                    KeyCode::ArrowLeft | KeyCode::Digit4 => {
                        let i = flow.stat_cursor;
                        if flow.stat_mode {
                            if flow.point_stats[i] > 10 {
                                flow.point_stats[i] -= 1;
                            }
                        } else if flow.min_stats[i] > 0 {
                            flow.min_stats[i] -= 1;
                        }
                    }
                    KeyCode::ArrowRight | KeyCode::Digit6 => {
                        let i = flow.stat_cursor;
                        if flow.stat_mode {
                            if flow.point_stats[i] < 18 {
                                let mut trial = flow.point_stats;
                                trial[i] += 1;
                                if point_buy_cost(&trial) <= 48 {
                                    flow.point_stats = trial;
                                }
                            }
                        } else if flow.min_stats[i] < 40 {
                            flow.min_stats[i] += 1;
                        }
                    }
                    KeyCode::Enter => {
                        let raw = flow.quests.trim();
                        let n = if raw.is_empty() {
                            20
                        } else {
                            raw.parse::<i32>().unwrap_or(20).clamp(0, 98)
                        };
                        let (stats, gold) = if flow.stat_mode {
                            (
                                fold_point_stats(&flow, &gd),
                                Some(point_buy_gold(point_buy_cost(&flow.point_stats))),
                            )
                        } else {
                            (flow.stats, None)
                        };
                        start_game(&mut commands, &gd, &flow, n, Some(stats), gold);
                        next.set(AppState::Playing);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Post-creation subrace effects shared by the birth flow and the test
/// harness: the placeholder "Vampire" subrace gains the corruption chain
/// (birth.cc player_outfit) and an astral being starts in the Halls of
/// Mandos (birth.cc: `p_ptr->astral`).
pub fn apply_subrace_birth(gd: &GameData, ps: &mut PlayerState) {
    if gd
        .racemods
        .get(ps.subrace as usize)
        .is_some_and(|m| m.name.trim() == "Vampire")
    {
        let mut rng = crate::rng::current();
        for id in [
            crate::corrupt::CORRUPT_VAMPIRE_TEETH,
            crate::corrupt::CORRUPT_VAMPIRE_STRENGTH,
            crate::corrupt::CORRUPT_VAMPIRE,
        ] {
            crate::corrupt::gain_full(gd, ps, id, &mut crate::game::MessageLog::default(), &mut rng);
        }
    }
    // Undead start at nightfall (dungeon.cc:4660).
    if crate::game::player_has_flag(gd, ps, "UNDEAD") {
        ps.turn = crate::game::TURNS_PER_DAY / 2;
    }
    if crate::game::player_has_flag(gd, ps, "ASTRAL") {
        ps.astral = true;
        ps.dungeon = 8;
        ps.depth = 98;
        ps.wild_x = 45;
        ps.wild_y = 19;
        ps.recall_dungeon = 4;
    }
}

/// Everything a finished birth produces (birth.cc player_birth): the
/// character, the starting kit, the plot/quest state and the wilderness.
pub struct NewGame {
    pub ps: PlayerState,
    pub inv: Inventory,
    pub plot: crate::game::PlotQuest,
    pub levels: crate::save::LevelStore,
}

/// Create the character and the world state once the birth flow is
/// complete (race/class chosen, stats rolled or bought, quest count).
#[allow(clippy::too_many_arguments)]
pub fn build_new_game(
    gd: &GameData,
    flow: &BirthFlow,
    quests: i32,
    stats: Option<[i32; 6]>,
    gold: Option<i32>,
) -> Option<NewGame> {
    let (Some(race_idx), Some(class_idx), Some(spec_idx)) = (flow.race, flow.class, flow.spec)
    else {
        return None;
    };
    let subrace_idx = flow.subrace.unwrap_or(0);
    let name = flow.name.trim().to_string();
    let mut ps = make_player_spec_with_stats(
        gd, name, race_idx, subrace_idx, class_idx, spec_idx, stats, gold,
    );
    ps.god = flow.god.unwrap_or(0);
    // Birth grace: 200 for a god-friendly class/subrace
    // (C:G:/R:G:GOD_FRIEND), 100 otherwise, 0 for atheists
    // (src/birth.cc:1921).
    if ps.god != 0 {
        let friendly = crate::game::player_race_flags(gd, &ps)
            .iter()
            .any(|f| *f == "GOD_FRIEND");
        ps.grace = if friendly { 200 } else { 100 };
    }
    // Subrace/race-modifier birth effects (vampire chain, astral start).
    apply_subrace_birth(gd, &mut ps);
    // Following Melkor reveals the Udun skill (gods.cc).
    crate::skill::recompute_hidden(&mut ps, gd);
    // Roll the random dungeon towns (player_birth, src/birth.cc).
    ps.random_towns = crate::game::roll_random_towns(gd, &mut crate::rng::current());
    let inv = make_inventory(gd, race_idx, subrace_idx, class_idx, spec_idx);
    ps.ac = crate::game::player_ac(&ps, &inv, gd);
    // Stat-granting starting gear applies immediately.
    for it in inv.equip.iter().flatten() {
        let d = crate::item::stat_deltas(gd, it);
        for i in 0..6 {
            ps.stats[i] += d[i];
        }
    }
    // Seed the power notification set (xtra1.cc calc_powers_silent at
    // birth) so the first upkeep does not re-announce every racial power.
    ps.known_powers = crate::game::available_powers(gd, &inv, &ps);
    // Random quests are rolled once, at birth (q_rand.cc).
    let mut plot = crate::game::PlotQuest::default();
    let mut rng = crate::rng::current();
    plot.init_random_quests(gd, quests, &mut rng);
    // Quest init hooks also run at game start (q_shroom.cc:318-347,
    // q_hobbit.cc quest_hobbit_init_hook): the mushroom count and the
    // hobbit's level are fixed before the player ever meets the NPCs.
    plot.shrooms_needed = rng.gen_range(7..=14);
    plot.hobbit_depth = rng.gen_range(26..=34);
    let levels = crate::save::LevelStore {
        levels: Default::default(),
        wilderness: crate::game::Wilderness::new(gd, &mut crate::rng::current()),
    };
    Some(NewGame {
        ps,
        inv,
        plot,
        levels,
    })
}

/// Insert the finished birth's resources once the flow is complete.
#[allow(clippy::too_many_arguments)]
fn start_game(
    commands: &mut Commands,
    gd: &GameData,
    flow: &BirthFlow,
    quests: i32,
    stats: Option<[i32; 6]>,
    gold: Option<i32>,
) {
    let Some(ng) = build_new_game(gd, flow, quests, stats, gold) else {
        return;
    };
    commands.insert_resource(ng.ps);
    commands.insert_resource(ng.inv);
    commands.insert_resource(ng.plot);
    commands.insert_resource(ShopStocks::default());
    commands.insert_resource(ng.levels);
    commands.insert_resource(crate::item::CreatedArtifacts::default());
}

pub fn update_birth_ui(
    flow: Res<BirthFlow>,
    gd: Res<GameData>,
    mut q: Query<&mut Text, With<BirthText>>,
) {
    let Ok(mut text) = q.single_mut() else {
        return;
    };
    *text = Text::new(birth_screen_text(&flow, &gd));
}

/// The text of the current birth screen. Kept pure so every step's
/// contents can be compared with the original `dump_*` / `birth_put_stats`
/// / `begin_screen` output in tests.
pub fn birth_screen_text(flow: &BirthFlow, gd: &GameData) -> String {
    let content = match flow.step {
        BirthStep::Menu => {
            let line = crate::save::peek()
                .map(|(name, level, depth)| format!("{} (level {}, depth {})", name, level, depth))
                .unwrap_or_else(|| "unknown hero".to_string());
            format!(
                "T o M E\nTales of Middle-earth — Bevy edition\n\n\n\
                 A saved game exists:\n\n  {}\n\n\
                 [Enter] continue  [N] new character",
                line
            )
        }
        BirthStep::Intro => {
            if flow.intro_page == 0 {
                "\n\nArt thou an adventurer,\n\
                 One who passes through the waterfalls we call danger\n\
                 to find the true nature of the legends beyond them?\n\
                 If this is so, then seeketh me.\n\n\n\
                 [Press any key to continue]"
                    .to_string()
            } else {
                "\n\nDarkGod\n\
                 in collaboration with\n\
                 Eru Iluvatar,\n\
                 Manwe\n\
                 and\n\
                 All the T.o.M.E. contributors (see credits.txt)\n\
                 present\n\n\
                 T.o.M.E.\n\n\
                 [Press any key to continue]"
                    .to_string()
            }
        }
        BirthStep::Name => format!(
            "T o M E\nTales of Middle-earth — Bevy edition\n\n\n\
             Enter your character's name:\n\n  > {}_\n\n\
             [Enter] continue",
            flow.name
        ),
        BirthStep::Race => {
            let mut s = String::from("Choose a race:\n\n");
            for (i, r) in gd.races.iter().enumerate() {
                let letter = (b'a' + i as u8) as char;
                s.push_str(&format!(
                    "  {}) {:<14} {}  HD:{:2} XP:{}%\n",
                    letter,
                    r.name,
                    stats_str(&r.stats),
                    r.hitdie,
                    r.exp
                ));
            }
            if let Some(sel) = flow.race {
                s.push_str(&format!("\n{}", gd.races[sel].desc));
            }
            s.push_str("\n\n[a-v] select  [Esc] back");
            s
        }
        BirthStep::Subrace => {
            let race_idx = flow.race.unwrap_or(0);
            let race = &gd.races[race_idx];
            let allowed = allowed_subraces(&gd, race_idx);
            let mut s = format!("Choose a subrace of the {} (race modifier):\n\n", race.name);
            for (i, mi) in allowed.iter().enumerate() {
                let m = &gd.racemods[*mi];
                let letter = (b'a' + i as u8) as char;
                let title = m.name.trim();
                let title = if title.is_empty() { "Normal" } else { title };
                s.push_str(&format!(
                    "  {}) {:<12} {}  HD:{:+} XP:{:+}% Mana:{}%\n",
                    letter,
                    title,
                    stats_str(&m.stats),
                    m.hitdie,
                    m.exp,
                    m.mana
                ));
            }
            if let Some(si) = flow.subrace {
                if let Some(m) = gd.racemods.get(si) {
                    s.push_str(&format!("\n{}", m.desc));
                }
            }
            s.push_str("\n\n[a-?] select  [Esc] back");
            s
        }
        BirthStep::Class => {
            let mut s = String::from("Choose a class:\n\n");
            let race_idx = flow.race.unwrap_or(0);
            for (i, ci) in allowed_classes(&gd, race_idx, flow.subrace.unwrap_or(0))
                .iter()
                .enumerate()
            {
                let c = &gd.classes[*ci];
                let letter = (b'a' + i as u8) as char;
                s.push_str(&format!(
                    "  {}) {:<12} {}  HD:{:2} XP:+{}%\n      {}\n",
                    letter,
                    c.name,
                    stats_str(&c.stats),
                    c.hitdie,
                    c.exp,
                    c.desc
                ));
            }
            s.push_str("\n[a-f] select  [Esc] back");
            s
        }
        BirthStep::Spec => {
            let class_idx = flow.class.unwrap_or(0);
            let class = &gd.classes[class_idx];
            let mut s = format!("Choose a specialisation of the {}:\n\n", class.name);
            for (i, sp) in class.specs.iter().enumerate() {
                let letter = (b'a' + i as u8) as char;
                s.push_str(&format!("  {}) {:<14} {}\n", letter, sp.name, sp.desc));
            }
            if let Some(si) = flow.spec {
                if let Some(sp) = class.specs.get(si) {
                    s.push_str(&format!("\n{}", sp.desc));
                }
            }
            s.push_str(&format!(
                "\n\n[a-{}] select  [Esc] back",
                (b'a' + class.specs.len().saturating_sub(1) as u8) as char
            ));
            s
        }
        BirthStep::God => {
            let (Some(ci), Some(si)) = (flow.class, flow.spec) else {
                return "Choose a god to worship:\n\n".to_string();
            };
            let gods = allowed_gods(
                &gd,
                flow.race.unwrap_or(0),
                flow.subrace.unwrap_or(0),
                ci,
                si,
            );
            let mut s = String::from("Choose a god to worship:\n\n");
            for (i, g) in gods.iter().enumerate() {
                let letter = (b'a' + i as u8) as char;
                s.push_str(&format!("  {}) {}\n", letter, god_name_full(*g)));
            }
            s.push_str("\n\n[a-?] select  [Esc] back");
            s
        }
        BirthStep::Quests => format!(
            "Number of random quests? (0-98)\n\n\
             Princesses and lost swordsmen will wait on many dungeon\n\
             levels over your career.\n\n  > {}_\n\n\
             [0-9] type  [Backspace] erase  [Enter] continue (default 20)",
            flow.quests
        ),
        BirthStep::Stats => {
            let folded = fold_point_stats(&flow, &gd);
            let mut s = format!(
                "Generate your attributes ({})\n\n",
                if flow.stat_mode {
                    "point buy"
                } else {
                    "autoroll"
                }
            );
            for i in 0..6 {
                let marker = if i == flow.stat_cursor { ">" } else { " " };
                if flow.stat_mode {
                    let cost = BIRTH_STAT_COSTS[(flow.point_stats[i] - 10).clamp(0, 8) as usize];
                    s.push_str(&format!(
                        " {} {:<4} {:<3} (bonus {:+})  cost {}\n",
                        marker, STAT_NAMES[i], folded[i], folded[i] - flow.point_stats[i], cost
                    ));
                } else {
                    s.push_str(&format!(
                        " {} {:<4} {:<3} (min {})\n",
                        marker, STAT_NAMES[i], flow.stats[i], flow.min_stats[i]
                    ));
                }
            }
            if flow.stat_mode {
                s.push_str(&format!(
                    "\nTotal cost {}/48.  [2/8] row  [4/6] adjust  [a] autoroll  [Enter] accept",
                    point_buy_cost(&flow.point_stats)
                ));
            } else {
                s.push_str(&format!(
                    "\n{} rounds.  [2/8] row  [4/6] minimum  [r] reroll  [p] point buy{}  [Enter] accept",
                    flow.stat_rounds,
                    // The original only offers 'p' once a previous roll
                    // exists (birth.cc player_birth_aux_auto).
                    if flow.previous.is_some() { "  [v] prev" } else { "" }
                ));
            }
            s.push_str("\n[Esc] back");
            s
        }
    };
    content
}

fn stats_str(stats: &[i32; 6]) -> String {
    format!(
        "S{:+} I{:+} W{:+} D{:+} C{:+} Ch{:+}",
        stats[0], stats[1], stats[2], stats[3], stats[4], stats[5]
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_random_name_uses_the_syllable_tables() {
        let mut rng = crate::rng::new_seeded_rng(42);
        let a = crate::birth::create_random_name(&mut rng);
        let b = crate::birth::create_random_name(&mut rng);
        assert!(!a.is_empty() && a.chars().all(|c| c.is_ascii_alphabetic()), "{a}");
        assert!(!b.is_empty());
        let mut rng2 = crate::rng::new_seeded_rng(42);
        assert_eq!(crate::birth::create_random_name(&mut rng2), a);
    }

    use super::*;
    use crate::data::load_game_data;

    fn class_spec(gd: &GameData, class: &str, spec: &str) -> (usize, usize) {
        let ci = gd.classes.iter().position(|c| c.name == class).unwrap();
        let si = gd.classes[ci]
            .specs
            .iter()
            .position(|s| s.name == spec)
            .unwrap();
        (ci, si)
    }

    #[test]
    fn every_class_spec_is_selectable_and_sets_the_identity() {
        let gd = load_game_data();
        assert_eq!(gd.classes.iter().map(|c| c.specs.len()).sum::<usize>(), 30);
        let (ci, si) = class_spec(&gd, "Loremaster", "Bard");
        let ps = make_player_spec(&gd, "T".into(), 0, 0, ci, si);
        assert_eq!(ps.class_name, "Bard");
        assert_eq!(ps.base_class, "Loremaster");
        // The spec's Music modifier applies on top of the class.
        assert!(ps.skill(crate::skill::SK_MUSIC) >= 1);
        // Base-class stats/blow params come from Loremaster (blow_mul 3).
        let class = ps.class(&gd).unwrap();
        assert_eq!(class.name, "Loremaster");
        assert_eq!(class.blow_mul, 3);
        // The Summoner is a Loremaster spec with maxed Monster-lore.
        let (ci, si) = class_spec(&gd, "Loremaster", "Summoner");
        let ps = make_player_spec(&gd, "T".into(), 0, 0, ci, si);
        assert!(ps.skill(crate::skill::SK_LORE) >= 15);
        assert!(ps.skill(crate::skill::SK_SUMMON) >= 1);
        // The Daemonologist's Demonology skill comes from the spec.
        let (ci, si) = class_spec(&gd, "Warrior", "Demonologist");
        let ps = make_player_spec(&gd, "T".into(), 0, 0, ci, si);
        assert_eq!(ps.skill(crate::skill::SK_DAEMON), 1);
        // The Druid is a Yavanna priest: Prayer from the class, spec adds
        // Monster-lore/Summoning.
        let (ci, si) = class_spec(&gd, "Priest", "Druid");
        let ps = make_player_spec(&gd, "T".into(), 0, 0, ci, si);
        assert_eq!(ps.skill(crate::skill::SK_PRAY), 1);
        assert_eq!(ps.skill(crate::skill::SK_LORE), 1);
    }

    #[test]
    fn spec_god_restrictions_follow_the_p_info_lists() {
        let gd = load_game_data();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        assert_eq!(allowed_gods(&gd, 0, 0, ci, si), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let (ci, si) = class_spec(&gd, "Warrior", "Swordmaster");
        assert_eq!(allowed_gods(&gd, 0, 0, ci, si), vec![0, 2, 3, 4, 5]);
        let (ci, si) = class_spec(&gd, "Priest", "Druid");
        assert_eq!(allowed_gods(&gd, 0, 0, ci, si), vec![5]);
        let (ci, si) = class_spec(&gd, "Priest", "Dark-Priest");
        assert_eq!(allowed_gods(&gd, 0, 0, ci, si), vec![4]);
        // Loremaster has C:g:All Gods, so its specs are unrestricted
        // (deity_info has nine gods: Eru..Mandos).
        let (ci, si) = class_spec(&gd, "Loremaster", "Bard");
        assert_eq!(allowed_gods(&gd, 0, 0, ci, si), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        // A race with R:G:NO_GOD (Maia, id 21) never gets a god choice.
        let maia = gd.races.iter().position(|r| r.name == "Maia").unwrap();
        assert!(allowed_gods(&gd, maia, 0, ci, si).is_empty());
    }

    #[test]
    fn starting_kits_come_from_the_proto_lists() {
        let gd = load_game_data();
        // Warrior spec: class C:O: ring + chain, spec C:O: broad sword.
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let inv = make_inventory(&gd, 0, 0, ci, si);
        let all: Vec<_> = inv.pack.iter().chain(inv.equip.iter().flatten()).collect();
        assert!(all.iter().any(|i| gd.objects[i.def].name == "Broad Sword"));
        assert!(all.iter().any(|i| gd.objects[i.def].name == "Chain Mail"));
        assert!(all
            .iter()
            .any(|i| gd.objects[i.def].name == "Ring of Fear Resistance"));
        // Every character gets the guide, rations and torches.
        assert!(inv
            .pack
            .iter()
            .any(|i| gd.objects[i.def].tval == data::TV_PARCHMENT));
        assert!(inv
            .pack
            .iter()
            .any(|i| gd.objects[i.def].name == "Ration of Food"));
        // A weapon is auto-equipped for convenience.
        assert!(inv.equip[data::SLOT_WEAPON].is_some());
        assert!(inv.equip[data::SLOT_LITE].is_some());
        // Bards carry (and wield) their Harp from C:a:O.
        let (ci, si) = class_spec(&gd, "Loremaster", "Bard");
        let inv = make_inventory(&gd, 0, 0, ci, si);
        let wielded = &inv.equip[data::SLOT_WEAPON].as_ref().unwrap();
        assert_eq!(gd.objects[wielded.def].tval, data::TV_INSTRUMENT);
        // The Druid gets the Charm Animal spellbook (birth.cc).
        let (ci, si) = class_spec(&gd, "Priest", "Druid");
        let inv = make_inventory(&gd, 0, 0, ci, si);
        assert!(inv
            .pack
            .iter()
            .any(|i| gd.objects[i.def].tval == data::TV_BOOK
                && i.spells.iter().any(|s| s == "Charm Animal")));
        // Mimics start with a Mouse-shaped Cloak of Mimicry.
        let (ci, si) = class_spec(&gd, "Loremaster", "Mimic");
        let inv = make_inventory(&gd, 0, 0, ci, si);
        assert!(inv
            .pack
            .iter()
            .any(|i| gd.objects[i.def].tval == data::TV_CLOAK
                && i.pval2 == crate::mimic::MIMIC_MOUSE as i32));
    }

    fn subrace(gd: &GameData, name: &str) -> usize {
        gd.racemods
            .iter()
            .position(|m| m.name.trim() == name)
            .unwrap()
    }

    #[test]
    fn base_subraces_carry_their_birth_effects() {
        let gd = load_game_data();
        assert_eq!(gd.racemods.len(), 9);
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        // Every race can pick the record-0 Normal entry; Humans also get
        // the undead/human-flavoured modifiers.
        let subs = allowed_subraces(&gd, human);
        let names: Vec<&str> = subs.iter().map(|i| gd.racemods[*i].name.trim()).collect();
        assert!(names.contains(&"Normal") || names.contains(&""));
        assert!(names.contains(&"Vampire"));
        assert!(names.contains(&"Spectre"));
        assert!(names.contains(&"Barbarian"));
        // The S:A list keeps Vampires out of the Elven races.
        let elf = gd.races.iter().position(|r| r.name == "Elf").unwrap();
        assert!(!allowed_subraces(&gd, elf)
            .iter()
            .any(|i| gd.racemods[*i].name == "Vampire"));

        // Spectre: undead semi-wraith, extra stealth, infra and luck.
        let (ci, si) = class_spec(&gd, "Mage", "Mage");
        let sp = subrace(&gd, "Spectre");
        let ps = make_player_spec(&gd, "S".into(), human, sp, ci, si);
        assert_eq!(ps.subrace as usize, sp);
        assert!(crate::game::player_has_flag(&gd, &ps, "UNDEAD"));
        assert!(crate::game::player_has_flag(&gd, &ps, "SEMI_WRAITH"));
        assert!(crate::game::player_has_flag(&gd, &ps, "NO_CUT"));
        // Stats are rolled (5 + 1d3 + 1d4 + 1d5, 8..=17) plus the
        // race/subrace/class adjustments; Spectre STR is -5.
        let mage = gd.classes[ci].stats[0];
        let spectre = gd.racemods[sp].stats[0];
        assert!(
            (8 + spectre + mage..=17 + spectre + mage).contains(&ps.stats[0]),
            "STR {} out of range (bonus {})",
            ps.stats[0],
            spectre + mage
        );
        assert_eq!(ps.mana_mult, 105);
        assert_eq!(ps.skill(crate::skill::SK_STEALTH), 2); // +2000
        let t = crate::item::Inventory::default().totals_for(&gd, &ps);
        assert_eq!(t.infra, 3);
        assert_eq!(t.luck, ps.luck_base);

        // Zombie forbids the Mage class, Vampire opens it for any race.
        let (ci, _) = class_spec(&gd, "Mage", "Mage");
        let zombie = subrace(&gd, "Zombie");
        let zclasses = allowed_classes(&gd, human, zombie);
        assert!(!zclasses.iter().any(|c| gd.classes[*c].name == "Mage"));
        let troll = gd.races.iter().position(|r| r.name == "Troll").unwrap();
        let vamp = subrace(&gd, "Vampire");
        let vclasses = allowed_classes(&gd, troll, vamp);
        assert!(vclasses.iter().any(|c| gd.classes[*c].name == "Mage"));
        assert!(vclasses.iter().any(|c| gd.classes[*c].name == "Warrior"));
        let _ = ci;
    }

    #[test]
    fn vampire_and_astral_birth_effects() {
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Rogue", "Rogue");
        // Plain Vampire subrace: gains the three vampire corruptions and
        // becomes undead/vampiric (birth.cc player_outfit).
        let vamp = subrace(&gd, "Vampire");
        let mut ps = make_player_spec(&gd, "V".into(), human, vamp, ci, si);
        apply_subrace_birth(&gd, &mut ps);
        assert!(crate::corrupt::has(
            &ps,
            crate::corrupt::CORRUPT_VAMPIRE_TEETH
        ));
        assert!(crate::corrupt::has(&ps, crate::corrupt::CORRUPT_VAMPIRE));
        assert!(crate::game::player_has_flag(&gd, &ps, "VAMPIRE"));
        assert!(crate::game::player_has_flag(&gd, &ps, "HURT_LITE"));
        assert!(crate::game::player_has_flag(&gd, &ps, "NO_SUBRACE_CHANGE"));
        assert!(crate::corrupt::powers(&ps).contains(&crate::corrupt::PWR_VAMPIRISM));
        // Vampire teeth cannot be rolled for a plain character, but can
        // for a NO_SUBRACE_CHANGE subrace (Spectre).
        let mut plain = make_player_spec(&gd, "P".into(), human, 0, ci, si);
        assert!(!crate::corrupt::can_gain(
            &gd,
            &plain,
            crate::corrupt::CORRUPT_VAMPIRE_TEETH
        ));
        let sp = subrace(&gd, "Spectre");
        let spectre = make_player_spec(&gd, "S".into(), human, sp, ci, si);
        assert!(crate::corrupt::can_gain(
            &gd,
            &spectre,
            crate::corrupt::CORRUPT_VAMPIRE_TEETH
        ));
        // LostSoul: astral start in the Halls of Mandos (dungeon 8, 98).
        let lost = subrace(&gd, "LostSoul");
        let mut astral = make_player_spec(&gd, "L".into(), human, lost, ci, si);
        apply_subrace_birth(&gd, &mut astral);
        assert!(astral.astral);
        assert_eq!(astral.dungeon, 8);
        assert_eq!(astral.depth, 98);
        assert_eq!(astral.recall_dungeon, 4);
        let mut log = crate::game::MessageLog::default();
        assert!(crate::game::astral_recall_blocked(&astral, &mut log));
        // Undead start at nightfall (dungeon.cc).
        let mut ghost = make_player_spec(&gd, "G".into(), human, sp, ci, si);
        apply_subrace_birth(&gd, &mut ghost);
        assert!(!crate::game::is_daytime(ghost.turn));
    }

    #[test]
    fn point_buy_and_autoroll_apis() {
        // birth_stat_costs + gold rule (birth.cc:2082, 2168).
        assert_eq!(BIRTH_STAT_COSTS, [0, 1, 2, 4, 7, 11, 16, 22, 30]);
        assert_eq!(point_buy_cost(&[10; 6]), 0);
        assert_eq!(point_buy_gold(0), 600);
        assert_eq!(point_buy_gold(30), 600);
        assert_eq!(point_buy_gold(48), 100);
        let stats = [18, 18, 10, 10, 10, 10];
        assert_eq!(point_buy_cost(&stats), 30 + 30);

        // Autoroll accepts immediately when the minimums are low and
        // reports the per-stat match counts (birth.cc:2250).
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let race = gd.races[human].clone();
        let class = gd.classes[ci].clone();
        let mut rng = crate::rng::current();
        let (rolled, rounds, matches) =
            auto_roll(&race, None, &class, &[3; 6], &mut rng);
        assert!(rounds >= 1);
        assert!(matches.iter().all(|m| *m >= 1));
        assert!(rolled.iter().all(|s| (3..=40).contains(s)));
        assert_eq!(rolled.len(), 6);
        let _ = si;
    }

    #[test]
    fn stats_screen_folds_point_buy_and_reaches_creation() {
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let mut flow = BirthFlow {
            race: Some(human),
            subrace: Some(0),
            class: Some(ci),
            spec: Some(si),
            ..Default::default()
        };
        flow.point_stats = [18, 16, 10, 10, 10, 10];
        let cost = point_buy_cost(&flow.point_stats);
        assert!(cost <= 48, "cost {cost}");
        let folded = fold_point_stats(&flow, &gd);
        let gold = point_buy_gold(cost);
        let ps = make_player_spec_with_stats(
            &gd,
            "Point Buyer".into(),
            human,
            0,
            ci,
            si,
            Some(folded),
            Some(gold),
        );
        assert_eq!(ps.stats, folded);
        assert_eq!(ps.stat_base, folded);
        assert_eq!(ps.gold, gold);
        // Autoroll honours the minimums and is reachable from the flow.
        flow.min_stats = [3; 6];
        roll_auto_stats(&mut flow, &gd);
        assert!(flow.stat_rounds >= 1);
        assert!(flow.stats.iter().all(|s| *s >= 3));
    }

    #[test]
    fn adjust_stat_folds_like_modify_stat_value() {
        // birth.cc adjust_stat is byte-for-byte xtra1.cc modify_stat_value
        // (used by get_stats); the port's folded scale makes each original
        // point one integer step.
        assert_eq!(crate::game::modify_stat_value(17, 1), 18);
        assert_eq!(crate::game::modify_stat_value(18, 2), 20);
        assert_eq!(crate::game::modify_stat_value(20, -4), 16);
        assert_eq!(crate::game::modify_stat_value(23, 99), 40);
        assert_eq!(crate::game::modify_stat_value(3, -5), 3);
        // get_stats applies it to the rolled value with the race/class sum.
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, _si) = class_spec(&gd, "Warrior", "Warrior");
        let race = gd.races[human].clone();
        let class = gd.classes[ci].clone();
        let mut rng = crate::rng::new_seeded_rng(7);
        for _ in 0..20 {
            let stats = roll_stats(&race, None, &class, &mut rng);
            let mut raw_sum = 0;
            for i in 0..6 {
                // Recover the raw 5 + 1d3 + 1d4 + 1d5 roll (the Human race
                // has no adjustment, so only the class shift is removed).
                let raw = stats[i] - class.stats[i];
                assert!((8..=17).contains(&raw), "stat {i}: {raw}");
                raw_sum += raw;
            }
            // The 42 < sum(18 dice) < 57 re-roll check becomes 73..=86.
            assert!((73..=86).contains(&raw_sum), "{raw_sum}");
        }
    }

    #[test]
    fn roll_player_hp_stays_in_the_birth_band() {
        // birth.cc roll_player_hp: entry 0 is the maximal level-1 die, the
        // remaining 49 are 1d(hitdie) and the cumulative total must land in
        // the 3/8..5/8 band (PY_MAX_LEVEL = 50).
        let mut rng = crate::rng::new_seeded_rng(11);
        for hitdie in [1, 4, 6, 10] {
            let rolls = roll_player_hp(hitdie, &mut rng);
            assert_eq!(rolls.len(), 50);
            assert_eq!(rolls[0], hitdie);
            assert!(rolls[1..].iter().all(|r| (1..=hitdie).contains(r)));
            let total: i32 = rolls.iter().sum();
            let min_value = (50 * (hitdie - 1) * 3) / 8 + 50;
            let max_value = (50 * (hitdie - 1) * 5) / 8 + 50;
            assert!(
                (min_value..=max_value).contains(&total),
                "hd {hitdie}: {total} not in {min_value}..={max_value}"
            );
        }
    }

    #[test]
    fn roll_gold_follows_the_stat_penalties() {
        // birth.cc get_money: randint(100)+300, then -300 for 18/50+,
        // -200 for 18/20+, -150 above 18 and (stat-8)*10 otherwise,
        // floored at 100.
        let mut rng = crate::rng::new_seeded_rng(13);
        // Stat 10 costs (10-8)*10 = 20 each, 120 in total.
        for _ in 0..20 {
            let gold = roll_gold(&[10; 6], &mut rng);
            assert!((181..=280).contains(&gold), "{gold}");
        }
        // Very low stats refund 50 each on top of the base.
        let low = roll_gold(&[3; 6], &mut rng);
        assert!((601..=700).contains(&low), "{low}");
        // Six 18/200+ stats exhaust the 300..400 base gold: floor 100.
        assert_eq!(roll_gold(&[40; 6], &mut rng), 100);
        // Stats at 8 cost nothing.
        let base = roll_gold(&[8; 6], &mut rng);
        assert!((301..=400).contains(&base), "{base}");
    }

    #[test]
    fn fresh_character_wipes_state_and_rolls_extra_info() {
        // birth.cc player_wipe zeroes the struct ("well fed", no fate, no
        // cheat flags, level 1) and get_extra sets level/exp/hitdie/tactics.
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let mut ps = make_player_spec(&gd, "Fresh".into(), human, 0, ci, si);
        assert_eq!(ps.level, 1);
        assert_eq!(ps.max_plv, 1);
        assert_eq!(ps.exp, 0);
        assert_eq!(ps.food, crate::game::FOOD_FULL - 1);
        assert_eq!(ps.hp_mod, 0);
        assert!(!ps.black_breath);
        assert!(!ps.no_mortal);
        assert!(!ps.disembodied);
        assert!(!ps.wild_mode);
        assert_eq!(ps.allow_one_death, 0);
        assert!(ps.extra_powers.is_empty());
        assert!(ps.inertia_spell.is_none());
        // get_extra: hitdice sum and exp factor from race+subrace+class.
        let class = &gd.classes[ci];
        let race = &gd.races[human];
        let hitdie =
            race.hitdie as i32 + gd.racemods[0].hitdie + class.hitdie as i32;
        assert_eq!(ps.hitdie, hitdie.max(1));
        // calc_hitpoints at level 1: hitdie + adj_con_mhp[CON]/2, floor 2.
        assert!(ps.max_hp >= 2);
        for (con, adj_con_mhp) in [(3, -5), (10, 0), (18, 3), (28, 10), (40, 27)] {
            let mut stats = [12; 6];
            stats[4] = con;
            let built = make_player_spec_with_stats(
                &gd,
                "H".into(),
                human,
                0,
                ci,
                si,
                Some(stats),
                Some(0),
            );
            let want = (hitdie + adj_con_mhp / 2).max(2);
            assert_eq!(built.max_hp, want, "CON {con}");
            assert_eq!(built.hp, want);
        }
        assert_eq!(ps.hp_planned.len(), 50);
        assert_eq!(ps.hp_planned[0], ps.hitdie);
        let exp = race.exp as i32 + gd.racemods[0].exp + class.exp as i32;
        assert_eq!(ps.exp_factor, exp.max(1) as u64);
        // get_extra tactics/movement and inside_quest default.
        assert_eq!(ps.tactic, 4);
        assert_eq!(ps.movement, 4);
        assert!(ps.quest_origin.is_none());
        // The wipe leaves the player "fully healed" (chp = mhp equivalent).
        assert_eq!(ps.hp, ps.max_hp);
        // Inscriptions are cleared.
        assert!(ps.inscriptions.iter().all(|k| !*k));
        let _ = si;
    }

    #[test]
    fn save_and_load_prev_data_swap_the_rolls() {
        // birth.cc save_prev_data / load_prev_data(true): stash the
        // current roll, load it back and keep the old one for the next
        // toggle; a plain load without save keeps the slot.
        let mut flow = BirthFlow::default();
        flow.stats = [17, 16, 15, 14, 13, 12];
        flow.stat_rounds = 5;
        save_prev_data(&mut flow);
        flow.stats = [3; 6];
        flow.stat_rounds = 9;
        load_prev_data(&mut flow, true);
        assert_eq!(flow.stats, [17, 16, 15, 14, 13, 12]);
        assert_eq!(flow.stat_rounds, 5);
        assert_eq!(flow.previous, Some(([3; 6], 9)));
        // Toggling again brings the other roll back.
        load_prev_data(&mut flow, true);
        assert_eq!(flow.stats, [3; 6]);
        assert_eq!(flow.stat_rounds, 9);
        // save=false simply restores without touching the slot.
        flow.previous = Some(([20; 6], 1));
        load_prev_data(&mut flow, false);
        assert_eq!(flow.stats, [20; 6]);
        assert_eq!(flow.previous, Some(([20; 6], 1)));
        // No stored roll: nothing happens.
        let mut empty = BirthFlow::default();
        empty.stats = [1; 6];
        load_prev_data(&mut empty, true);
        assert_eq!(empty.stats, [1; 6]);
    }

    #[test]
    fn player_birth_builds_the_game_state() {
        // birth.cc player_birth: wipe -> aux -> skills/abilities -> god ->
        // melee -> outfit -> random towns -> town stores -> wilderness.
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let flow = BirthFlow {
            name: "Born".into(),
            race: Some(human),
            subrace: Some(0),
            class: Some(ci),
            spec: Some(si),
            god: Some(1),
            ..Default::default()
        };
        let ng = build_new_game(&gd, &flow, 7, None, None).expect("complete flow");
        assert_eq!(ng.ps.name, "Born");
        assert_eq!(ng.ps.level, 1);
        assert_eq!(ng.ps.god, 1);
        // Warrior is not GOD_FRIEND: the base 100 grace applies.
        assert_eq!(ng.ps.grace, 100);
        // Level-1 abilities are applied (Extra Max Blow 1/2).
        assert!(!ng.ps.abilities.is_empty());
        // Skills were computed with the points reset.
        assert_eq!(ng.ps.skill_points, 0);
        assert!(ng.ps.skill(crate::skill::SK_COMBAT) > 0);
        // Random dungeon towns were rolled for the flagged dungeons.
        assert!(!ng.ps.random_towns.is_empty());
        // Starting kit plus the auto-equipped weapon/light.
        assert!(!ng.inv.pack.is_empty());
        assert!(ng.inv.equip[data::SLOT_WEAPON].is_some());
        // Quest state: main plot taken, the two quest init hooks rolled.
        assert_eq!(
            ng.plot.states.get(&crate::game::PLOT_NECRO),
            Some(&crate::game::PLOT_TAKEN)
        );
        assert!((7..=14).contains(&ng.plot.shrooms_needed));
        assert!((26..=34).contains(&ng.plot.hobbit_depth));
        // Wilderness seeds exist for every cell.
        assert_eq!(
            ng.levels.wilderness.seeds.len(),
            (gd.world.w * gd.world.h) as usize
        );
        assert!(ng.levels.wilderness.seeds.iter().any(|s| *s != 0));
        // Fully healed by creation.
        assert_eq!(ng.ps.hp, ng.ps.max_hp);
        // An incomplete flow builds nothing.
        let incomplete = BirthFlow::default();
        assert!(build_new_game(&gd, &incomplete, 0, None, None).is_none());
    }

    #[test]
    fn savefile_slot_matches_the_original_index_roles() {
        // The original keeps a global.svg index of many savefiles
        // (load_savefile_names / save_savefile_names); the port keeps one
        // named slot, so existence/name peeking and storing map onto
        // save::delete/exists/peek/store. Back up any real save around
        // the test.
        let path = crate::save::path();
        let backup = std::fs::read(&path).ok();
        crate::save::delete();
        assert!(!crate::save::exists());
        assert!(crate::save::peek().is_none());

        let gd = load_game_data();
        let mut rng = crate::rng::current();
        let map = crate::map::generate_level(&gd, 3, &mut rng).map;
        let mut ps = make_player(&gd, "Slot Hero".into(), 0, 0);
        // load_player rejects a zero turn as a broken savefile.
        ps.turn = 1;
        let save = crate::save::SaveGame {
            ps,
            inv: crate::item::Inventory::default(),
            plot: crate::game::PlotQuest::default(),
            stocks: ShopStocks::default(),
            created: Default::default(),
            player_pos: (1, 1),
            map,
            monsters: Vec::new(),
            floor_items: Vec::new(),
            floor_gold: Vec::new(),
            levels: crate::save::LevelStore::default(),
            rng_state: String::new(),
            options: Default::default(),
        };
        crate::save::store(&save);
        assert!(crate::save::exists());
        assert_eq!(
            crate::save::peek().map(|(n, _, _)| n),
            Some("Slot Hero".into())
        );
        let loaded = crate::save::load().expect("reload");
        assert_eq!(loaded.ps.name, "Slot Hero");

        // Restore the previous state of the slot.
        match backup {
            Some(bytes) => std::fs::write(&path, bytes).expect("restore"),
            None => crate::save::delete(),
        }
    }

    #[test]
    fn starting_kit_rolls_follow_the_original_ranges() {
        // player_outfit: rand_range(3,7) rations, rand_range(3,7) torches
        // with timeout rand_range(3,7)*500.
        let gd = load_game_data();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        for _ in 0..2 {
            let inv = make_inventory(&gd, 0, 0, ci, si);
            let rations = inv
                .pack
                .iter()
                .find(|i| gd.objects[i.def].name == "Ration of Food")
                .expect("rations");
            assert!((3..=7).contains(&rations.count), "{}", rations.count);
            // The light is auto-equipped by the port.
            let torch = inv
                .pack
                .iter()
                .chain(inv.equip.iter().flatten())
                .find(|i| gd.objects[i.def].name == "Wooden Torch")
                .expect("torch");
            assert!((3..=7).contains(&torch.count), "{}", torch.count);
            assert!(torch.timeout > 0 && torch.timeout % 500 == 0);
            assert!((1500..=3500).contains(&torch.timeout), "{}", torch.timeout);
        }
    }

    #[test]
    fn birth_screens_render_the_selection_lists() {
        // dump_races / dump_rmods / dump_classes / dump_specs / dump_gods
        // and the quest prompt, as rendered by the port's text screens.
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");

        let mut flow = BirthFlow {
            step: BirthStep::Intro,
            ..Default::default()
        };
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Press any key to continue"), "{text}");
        flow.step = BirthStep::Name;
        flow.name = "Hero".into();
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Hero"), "{text}");

        flow.step = BirthStep::Race;
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Human"), "{text}");
        assert!(text.contains("Choose a race"));

        flow.step = BirthStep::Subrace;
        flow.race = Some(human);
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Choose a subrace"));
        assert!(text.contains("Normal"));

        flow.step = BirthStep::Class;
        flow.subrace = Some(0);
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Choose a class"));
        assert!(text.contains("Warrior"));

        flow.step = BirthStep::Spec;
        flow.class = Some(ci);
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Choose a specialisation"));
        assert!(text.contains("Warrior"));

        flow.step = BirthStep::God;
        flow.spec = Some(si);
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Choose a god"));
        assert!(text.contains("Nobody")); // the atheist entry
        for name in ["Eru Iluvatar", "Manwe Sulimo", "Tulkas", "Melkor Bauglir"] {
            assert!(text.contains(name), "{name}: {text}");
        }

        flow.step = BirthStep::Quests;
        flow.quests = "20".into();
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("Number of random quests"));
        assert!(text.contains("20_"));

        // The menu (begin_screen) offers continue / new character.
        flow.step = BirthStep::Menu;
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("[Enter] continue"));
        assert!(text.contains("[N] new character"));
    }

    #[test]
    fn birth_stats_screen_renders_minimums_and_point_costs() {
        // birth_put_stats shows the folded stats; the point-buy screen
        // shows the birth_stat_costs table.
        let gd = load_game_data();
        let human = gd.races.iter().position(|r| r.name == "Human").unwrap();
        let (ci, si) = class_spec(&gd, "Warrior", "Warrior");
        let mut flow = BirthFlow {
            step: BirthStep::Stats,
            race: Some(human),
            subrace: Some(0),
            class: Some(ci),
            spec: Some(si),
            stat_mode: false,
            ..Default::default()
        };
        flow.stats = [16, 15, 14, 13, 12, 11];
        flow.min_stats = [17, 10, 10, 10, 10, 10];
        flow.stat_rounds = 1234;
        let text = birth_screen_text(&flow, &gd);
        for name in STAT_NAMES {
            assert!(text.contains(name), "{name}: {text}");
        }
        assert!(text.contains("1234 rounds"));
        assert!(text.contains("(min 17)"));
        assert!(!text.contains("[v] prev"), "no roll stashed yet");
        flow.previous = Some(([3; 6], 1));
        let text = birth_screen_text(&flow, &gd);
        assert!(text.contains("[v] prev"), "{text}");

        flow.stat_mode = true;
        flow.point_stats = [18, 16, 10, 10, 10, 10];
        let text = birth_screen_text(&flow, &gd);
        // birth_stat_costs: 18 -> 30, 16 -> 16, 10 -> 0.
        assert!(text.contains("Total cost 46/48"), "{text}");
        assert!(text.contains("cost 30"), "{text}");
        assert!(text.contains("cost 16"));
    }
}
