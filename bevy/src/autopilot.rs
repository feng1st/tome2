//! Autopilot for automated smoke-testing: enabled via the TOME_AUTOPLAY
//! environment variable (value = path of the screenshot to write).
//! Creates a character, wanders randomly, screenshots, exits.
//!
//! Extra hooks:
//! - TOME_STAY_TOWN=1     : do not auto-descend into the dungeon
//! - TOME_TEST_CLASS=name : create this class instead of Warrior
//! - TOME_TEST_MODAL=m    : open modal m (inventory/equipment/cast/shop/
//!                          drop/wield/takeoff/quaff/read/library/fireproof/
//!                          artifact/skills/abilities/actions/guild/ringpower)
//!                          and screenshot it
//! - TOME_TEST_POLY=1     : racial polymorph (do_poly_self) before play

use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use rand::Rng;

use crate::birth::{self, BirthFlow};
use crate::data::GameData;
use crate::game::{GridPos, Player, TurnState};
use crate::map::Map;
use crate::modal::Modal;
use crate::town::{self, ShopStocks};
use crate::AppState;

#[derive(Resource)]
pub struct Autopilot {
    pub frame: u32,
    pub shot_path: String,
    pub shot_taken: bool,
    /// Frames spent on the death screen (TOME_TEST_DEATH).
    pub dead_frames: u32,
}

pub fn maybe_insert(app: &mut App) {
    if let Ok(path) = std::env::var("TOME_AUTOPLAY") {
        app.insert_resource(Autopilot {
            frame: 0,
            shot_path: path,
            shot_taken: false,
            dead_frames: 0,
        });
    }
}

/// On the RIP screen (TOME_TEST_DEATH): press Enter to restart, so the
/// death -> Birth -> Playing resource path is exercised.
pub fn autopilot_dead(mut auto: ResMut<Autopilot>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    auto.dead_frames += 1;
    if auto.dead_frames == 20 {
        keys.press(KeyCode::Enter);
    } else if auto.dead_frames == 22 {
        keys.release(KeyCode::Enter);
    }
}

/// In Birth: instantly create a character (no typing animation).
/// If TOME_BIRTH_SHOT is set, screenshot the birth UI instead and exit.
pub fn autopilot_birth(
    mut auto: ResMut<Autopilot>,
    gd: Res<GameData>,
    mut flow: Option<ResMut<BirthFlow>>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    auto.frame += 1;
    // TOME_BIRTH_SHOT=path: screenshot the birth UI. TOME_BIRTH_STEP=
    // spec|god drives the flow to that step first (class from
    // TOME_BIRTH_CLASS, default Loremaster).
    if let Ok(step) = std::env::var("TOME_BIRTH_STEP") {
        if auto.frame == 5 {
            if let Some(f) = flow.as_deref_mut() {
                f.name = "Test".to_string();
                f.race = Some(0);
                let class_name =
                    std::env::var("TOME_BIRTH_CLASS").unwrap_or_else(|_| "Loremaster".into());
                f.class = gd.classes.iter().position(|c| c.name == class_name);
                match step.as_str() {
                    "subrace" => {
                        // Human choice list: the first record is Normal,
                        // the second the first real modifier.
                        f.subrace = Some(1);
                        f.step = birth::BirthStep::Subrace;
                    }
                    "spec" => f.step = birth::BirthStep::Spec,
                    "god" => {
                        let ci = f.class.unwrap_or(0);
                        f.spec = Some(gd.classes[ci].specs.len().saturating_sub(1));
                        f.step = birth::BirthStep::God;
                    }
                    _ => {}
                }
            }
        }
    }
    if let Ok(path) = std::env::var("TOME_BIRTH_SHOT") {
        if auto.frame == 30 {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if auto.frame >= 50 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if auto.frame < 5 {
        return;
    }
    if let Some(ref mut f) = flow {
        f.name = "Test".to_string();
    }
    // TOME_TEST_LOAD=1: restore the save file instead of a new character
    // (verifies the continue/load path end-to-end).
    if std::env::var("TOME_TEST_LOAD").is_ok() {
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
    }
    // Class from TOME_TEST_CLASS (default: Warrior), specialisation from
    // TOME_TEST_SPEC (default: the class' first C:a: record). A name that
    // matches a spec (Bard, Summoner, Druid, ...) selects its class.
    let want_class = std::env::var("TOME_TEST_CLASS").ok();
    let mut class_idx = 0usize;
    let mut spec_idx = 0usize;
    if let Some(name) = want_class.as_deref() {
        if let Some(ci) = gd.classes.iter().position(|c| c.name == name) {
            class_idx = ci;
        } else if let Some((ci, si)) = gd.classes.iter().enumerate().find_map(|(ci, c)| {
            c.specs
                .iter()
                .position(|s| s.name == name)
                .map(|si| (ci, si))
        }) {
            class_idx = ci;
            spec_idx = si;
        }
    }
    if let Some(name) = std::env::var("TOME_TEST_SPEC").ok() {
        if let Some(si) = gd.classes[class_idx]
            .specs
            .iter()
            .position(|s| s.name == name)
        {
            spec_idx = si;
        }
    }
    // Subrace from TOME_TEST_SUBRACE (Vampire/Spectre/Skeleton/Zombie/
    // Barbarian/Hermit/LostSoul; default the record-0 Normal entry).
    let mut subrace_idx = 0usize;
    if let Some(name) = std::env::var("TOME_TEST_SUBRACE").ok() {
        if let Some(mi) = gd.racemods.iter().position(|m| m.name.trim() == name) {
            subrace_idx = mi;
        }
    }
    let mut ps =
        birth::make_player_spec(&gd, "Test".to_string(), 0, subrace_idx, class_idx, spec_idx);
    // Autopick the first allowed god (Druid -> Yavanna, Dark-Priest ->
    // Melkor, ...); "All Gods" starts with Nobody.
    ps.god = birth::allowed_gods(&gd, 0, subrace_idx, class_idx, spec_idx)
        .first()
        .copied()
        .unwrap_or(0);
    if ps.god != 0 {
        let friendly = crate::game::player_race_flags(&gd, &ps)
            .iter()
            .any(|f| *f == "GOD_FRIEND");
        ps.grace = if friendly { 200 } else { 100 };
    }
    birth::apply_subrace_birth(&gd, &mut ps);
    crate::skill::recompute_hidden(&mut ps, &gd);
    // TOME_TEST_FATE=1: plant a certain find-object fate on the depth
    // the autopilot descends to, so the spawn message can be checked.
    if std::env::var("TOME_TEST_FATE").is_ok() {
        let depth = std::env::var("TOME_START_DEPTH")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        let mut rng = crate::rng::current();
        if let Some(k) = crate::item::gen_object_kind(&gd, depth + 5, &mut rng) {
            ps.fates.push(crate::game::Fate {
                fate: crate::game::FATE_FIND_O,
                level: depth,
                serious: true,
                object: k as u32,
                ..Default::default()
            });
        }
    }
    // TOME_TEST_DEMONBLADE=1: keep the Demon Blade projection active so
    // the wanderer's melee blows exercise the projection path.
    if std::env::var("TOME_TEST_DEMONBLADE").is_ok() {
        ps.tim_project = 500;
        ps.tim_project_gf = "HELL_FIRE".to_string();
        ps.tim_project_dam = 40;
        ps.tim_project_rad = 1;
    }
    // Roll the random dungeon towns, then optionally jump straight into one
    // (TOME_TEST_TOWN[=dungeon:depth], default Moria depth 33).
    ps.random_towns = crate::game::roll_random_towns(&gd, &mut crate::rng::current());
    // Screenshots of town services start inside Bree (the world start is
    // the wilderness cell just north of it).
    if !ps.astral
        && (std::env::var("TOME_STAY_TOWN").is_ok()
            || std::env::var("TOME_SHOT_SHOPS").is_ok()
            || std::env::var("TOME_TEST_MODAL").is_ok())
    {
        let (wx, wy) = gd.town_cell(1);
        ps.wild_x = wx;
        ps.wild_y = wy;
    }
    if std::env::var("TOME_TEST_TOWN").is_ok() {
        let spec = std::env::var("TOME_TEST_TOWN").unwrap_or_default();
        let parts: Vec<&str> = spec.split(':').collect();
        let num = |s: Option<&&str>, d: u32| s.and_then(|x| x.parse::<u32>().ok()).unwrap_or(d);
        let dungeon = num(parts.first(), 22);
        let depth = num(parts.get(1), 33);
        let seed = num(parts.get(2), 0x5EED_1234);
        if !ps
            .random_towns
            .iter()
            .any(|t| t.dungeon == dungeon && t.depth == depth)
        {
            ps.random_towns.push(crate::game::RandomTown {
                dungeon,
                depth,
                town: crate::game::TOWN_RANDOM,
                seed,
            });
        }
        ps.dungeon = dungeon;
        ps.depth = depth;
        ps.recall_dungeon = dungeon;
        ps.recall_depth = depth;
    }
    let inv = birth::make_inventory(&gd, 0, subrace_idx, class_idx, spec_idx);
    // TOME_TEST_EVIL=1: carry the One Ring so its confirmation can be
    // driven (q_one.cc evil path).
    let mut inv = inv;
    // TOME_TEST_EXTRA=1: grant mimicry extra limbs and fill the second
    // weapon/shield/gloves/boots slots (equipment screenshot).
    if std::env::var("TOME_TEST_EXTRA").is_ok() {
        ps.mimic_extra = crate::mimic::CLASS_ARMS | crate::mimic::CLASS_LEGS;
        ps.mimic_extra_turns = 1000;
        let find = |tval: i32| gd.objects.iter().position(|o| o.tval == tval);
        for (slot, tval) in [
            (crate::data::SLOT_WEAPON2, crate::data::TV_SWORD),
            (crate::data::SLOT_SHIELD2, crate::data::TV_SHIELD),
            (crate::data::SLOT_HANDS2, crate::data::TV_GLOVES),
            (crate::data::SLOT_FEET2, crate::data::TV_BOOTS),
        ] {
            if let Some(d) = find(tval) {
                inv.equip[slot] = Some(crate::item::Item::base(&gd, d));
            }
        }
    }
    if std::env::var("TOME_TEST_EVIL").is_ok() {
        if let Some(ring) = crate::item::specific_artifact_item(&gd, 13, &mut crate::rng::current()) {
            inv.pack.push(ring);
        }
    }
    // TOME_TEST_POLY=1: force a racial polymorph (do_poly_self) before
    // play starts, so the race-change path can be exercised.
    if std::env::var("TOME_TEST_POLY").is_ok() {
        let mut rng = crate::rng::current();
        let mut log = crate::game::MessageLog::default();
        ps.level = 30;
        ps.exp = ps.exp_needed();
        ps.max_hp += 400;
        ps.hp = ps.max_hp;
        // One call has only a 1/3 chance of a racial change; keep going
        // until the HUD shows a new race (bounded for safety).
        for _ in 0..40 {
            crate::game::do_poly_self(&gd, &mut ps, &inv, &mut log, &mut rng);
            if ps.race_name != "Human" || ps.hp <= 0 {
                break;
            }
        }
        ps.hp = ps.hp.max(1);
    }
    ps.ac = crate::game::player_ac(&ps, &inv, &gd);
    commands.insert_resource(ps);
    commands.insert_resource(inv);
    commands.insert_resource(crate::item::CreatedArtifacts::default());
    let mut plot = crate::game::PlotQuest::default();
    plot.init_random_quests(&gd, 20, &mut crate::rng::current());
    if std::env::var("TOME_TEST_EVIL").is_ok() {
        plot.ring_spawned = true;
    }
    commands.insert_resource(plot);
    commands.insert_resource(ShopStocks::default());
    commands.insert_resource(crate::save::LevelStore {
        levels: Default::default(),
        wilderness: crate::game::Wilderness::new(&gd, &mut crate::rng::current()),
    });
    next.set(AppState::Playing);
}

/// In Playing: descend into the dungeon, wander randomly; then screenshot
/// and exit. With TOME_TEST_MODAL, open the requested modal instead and
/// screenshot it.
/// Floor item access for the autopilot test hooks (keeps the system under
/// Bevy's 16-parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
pub struct AutopilotFloor<'w, 's> {
    pub tiles: Res<'w, crate::render::TileAssets>,
    pub stacks: Query<
        'w,
        's,
        (
            Entity,
            &'static GridPos,
            &'static mut crate::item::FloorItem,
        ),
        (Without<Player>, Without<crate::game::Monster>),
    >,
}

pub fn autopilot_play(
    mut auto: ResMut<Autopilot>,
    gd: Res<GameData>,
    mut map: ResMut<Map>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut turn: ResMut<TurnState>,
    mut modal: ResMut<Modal>,
    mut stocks: ResMut<ShopStocks>,
    mut plot: ResMut<crate::game::PlotQuest>,
    mut ps: ResMut<crate::game::PlayerState>,
    mut inv: ResMut<crate::item::Inventory>,
    mut player: Query<&mut GridPos, With<Player>>,
    mut monsters: Query<(Entity, &mut crate::game::Monster, &GridPos), Without<Player>>,
    mut floor: AutopilotFloor,
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    auto.frame += 1;
    let f = auto.frame;
    if f < 10 {
        return;
    }
    // TOME_TEST_DEATH=1: die once and let autopilot_dead press Enter on
    // the RIP screen (exercises the death -> restart resource path).
    if std::env::var("TOME_TEST_DEATH").is_ok() && f == 20 && auto.dead_frames == 0 {
        ps.hp = 0;
        next.set(AppState::Dead);
        return;
    }
    // TOME_TEST_SAVE=1: drive the real Q -> y quit path so the game saves.
    if std::env::var("TOME_TEST_SAVE").is_ok() {
        if f == 100 {
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyQ);
        } else if f == 102 {
            keys.release(KeyCode::KeyQ);
            keys.release(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyY);
        } else if f == 104 {
            keys.release(KeyCode::KeyY);
        } else if f >= 110 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // Shop-street screenshot: stand next to the first shop and shoot.
    if std::env::var("TOME_SHOT_SHOPS").is_ok() {
        if f == 15 {
            if let (Some(&i), Ok(mut p)) = (map.shops.keys().next(), player.single_mut()) {
                let (sx, sy) = (i as i32 % crate::map::MAP_W, i as i32 / crate::map::MAP_W);
                p.x = sx;
                p.y = sy + 2;
                turn.fov_dirty = true;
            }
        }
        if f >= 40 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 60 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // Modal UI test: open the window, screenshot, exit.
    if let Ok(m) = std::env::var("TOME_TEST_MODAL") {
        if f == 30 {
            let mut rng = crate::rng::current();
            *modal = match m.as_str() {
                "inventory" => Modal::Inventory,
                "equipment" => Modal::Equipment,
                "cast" => Modal::Cast,
                "drop" => Modal::Drop,
                "wield" => Modal::Wield,
                "takeoff" => Modal::TakeOff,
                "quaff" => Modal::Quaff,
                "read" => Modal::Read,
                "shop" => {
                    town::stock_for(&gd, &mut stocks, 1, 0, &mut Default::default(), ps.depth, ps.level, &mut rng);
                    Modal::Shop(0, 1)
                }
                "library" => {
                    plot.set(crate::game::PLOT_LIBRARY, crate::game::PLOT_COMPLETED);
                    Modal::Library {
                        current: 0,
                        slots: Vec::new(),
                    }
                }
                "skills" => {
                    ps.skill_points = 12;
                    Modal::Skills { cursor: 0 }
                }
                "abilities" => {
                    ps.skill_points = 12;
                    Modal::Abilities { cursor: 0 }
                }
                "actions" => Modal::Actions { cursor: 0 },
                "golem" => {
                    // A controlled-monster power menu (Fire Golem / Mind
                    // Steal; monster3.cc do_control_magic).
                    if let Some((_, mut m, _)) = monsters.iter_mut().next() {
                        m.controlled = true;
                        m.companion = true;
                        ps.control = Some(m.def as u32);
                    }
                    Modal::ControlMagic { cursor: 0 }
                }
                "mirror" => {
                    // Galadriel's Mirror with the Void path unlocked
                    // (q_ultrag.cc).
                    plot.ultra_good_unlocked = true;
                    Modal::UltraMirror
                }
                "knowledge" => Modal::Knowledge { page: 0, scroll: 0 },
                "messages" => Modal::Messages { scroll: 0 },
                "browse" => Modal::Browse { cursor: 0 },
                "observe" => Modal::Observe,
                "inscribe" => Modal::Inscribe { remove: false },
                "tactic" => Modal::Tactic { cursor: 0 },
                "steal" => {
                    town::stock_for(&gd, &mut stocks, 1, 0, &mut Default::default(), ps.depth, ps.level, &mut rng);
                    Modal::StealShop(0, 1)
                }
                "gamble" => {
                    ps.gold = 10000;
                    Modal::Gamble {
                        game: 12,
                        bet: 100,
                        maxbet: 1000,
                        result: "Black die: 3   Black die: 9   Red die: 5\nYOU WON (payoff x3)!"
                            .to_string(),
                        point: 0,
                        start_gold: 10000,
                    }
                }
                "compare" => {
                    // Two melee weapons to compare (bldg.cc).
                    inv.pack.clear();
                    for sval in [17, 24] {
                        if let Some(d) = gd.object_by_tval_sval(crate::data::TV_SWORD, sval) {
                            let mut it = crate::item::Item::base(&gd, d);
                            it.identified = true;
                            inv.pack.push(it);
                        }
                    }
                    Modal::CompareWeapons {
                        first: Some(0),
                        second: Some(1),
                        cost: 200,
                    }
                }
                "engrave" => {
                    for id in 1..8u8 {
                        crate::game::rune_learn(&mut ps, id);
                    }
                    Modal::Engrave
                }
                "querysymbol" => Modal::QuerySymbol {
                    result: Some("'#' = granite wall".to_string()),
                },
                "targetlock" => Modal::TargetLock { cursor: 0 },
                "blinkpower" => {
                    // A Gnome's racial blink power (R:Z:blink).
                    ps.race_name = "Gnome".to_string();
                    ps.level = 10;
                    ps.exp = ps.exp_needed();
                    ps.stats[crate::game::WIS] = 40;
                    ps.mana = 20;
                    ps.max_mana = 20;
                    Modal::Powers { cursor: 0 }
                }
                "midas" => Modal::MidasTouch { idx: None },
                "rohan" => Modal::RohanPower { step: 0 },
                "thunder" => Modal::ThunderPower { step: 0 },
                "straightroad" => {
                    let (x, y) = player.single().map(|p| (p.x, p.y)).unwrap_or((0, 0));
                    Modal::StraightRoad { x, y }
                }
                "ringpower" => {
                    // The One Ring's power menu (cmd6.cc ACT_POWER).
                    if let Some(it) = crate::item::specific_artifact_item(&gd, 13, &mut rng) {
                        inv.pack.push(it);
                    }
                    Modal::RingPower {
                        equip: false,
                        idx: 0,
                    }
                }
                "flame" => {
                    // The Eternal Flame and a Long Sword to imbue
                    // (cmd6.cc activate_eternal_flame).
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_JUNK, 255) {
                        let mut flame = crate::item::Item::base(&gd, d);
                        flame.identified = true;
                        inv.pack.push(flame);
                    }
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_SWORD, 17) {
                        inv.pack.push(crate::item::Item::base(&gd, d));
                    }
                    Modal::EternalFlame { cursor: 0 }
                }
                "mimic" => {
                    // Mimicry powers: develop the skill and wear a cloak.
                    ps.skills.insert(crate::skill::SK_MIMICRY, 20000);
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_CLOAK, 100) {
                        let mut cloak = crate::item::Item::base(&gd, d);
                        cloak.pval2 = 4; // Wolf
                        cloak.identified = true;
                        inv.equip[crate::data::SLOT_CLOAK] = Some(cloak);
                    }
                    Modal::MimicPower { cursor: 0 }
                }
                "powers" => {
                    // Corruption-granted powers (Balrog chain + anti-teleport)
                    // plus a racial power (Hobbit "create food", R:Z:).
                    let mut log = crate::game::MessageLog::default();
                    for id in [0usize, 1, 2, 3, 9] {
                        crate::corrupt::gain(&mut ps, id, &mut log);
                    }
                    ps.race_name = "Hobbit".to_string();
                    ps.level = 20;
                    ps.exp = ps.exp_needed();
                    Modal::Powers { cursor: 0 }
                }
                "power" => {
                    // TOME_TEST_POWER=<id|name>: grant one power (the
                    // extra_powers hook) and highlight it in the menu.
                    let spec = std::env::var("TOME_TEST_POWER")
                        .unwrap_or_else(|_| "spit acid".to_string());
                    let pid = spec
                        .parse::<i32>()
                        .ok()
                        .or_else(|| crate::game::power_id_by_name(&spec))
                        .unwrap_or(0);
                    if !ps.extra_powers.contains(&pid) {
                        ps.extra_powers.push(pid);
                    }
                    // Levelling properly keeps hp/mana/spells in step.
                    let mut level_log = crate::game::MessageLog::default();
                    ps.exp += 1_000_000;
                    crate::game::check_experience(&mut ps, &mut level_log, &mut rng);
                    for s in ps.stats.iter_mut() {
                        *s = (*s).max(30);
                    }
                    ps.mana = ps.max_mana;
                    ps.hp = ps.max_hp;
                    let avail = crate::game::available_powers(&gd, &inv, &ps);
                    let cursor = avail.iter().position(|id| *id == pid).unwrap_or(0);
                    Modal::Powers { cursor }
                }
                "activate" => {
                    // Ring of Teleportation's base-kind activation
                    // (cmd6.cc ACT_DEST_TELE); base activations need the
                    // item to be worn.
                    if let Some(d) = gd.object_by_name("Ring of Teleportation") {
                        let mut it = crate::item::Item::base(&gd, d);
                        it.identified = true;
                        inv.equip[crate::data::SLOT_RING1] = Some(it);
                    }
                    Modal::Activate
                }
                "throw" => {
                    // An item to throw (cmd2.cc do_cmd_throw).
                    if let Some(d) = gd.object_by_name("Dagger") {
                        inv.pack.push(crate::item::Item::base(&gd, d));
                    } else if let Some(d) = gd.object_by_name("Long Sword") {
                        inv.pack.push(crate::item::Item::base(&gd, d));
                    }
                    Modal::Throw { mult: 1 }
                }
                "eatmagic" => {
                    // A charged wand for PWR_EAT_MAGIC.
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_WAND, 0) {
                        let mut it = crate::item::Item::base(&gd, d);
                        it.charges = 7;
                        it.identified = true;
                        inv.pack.push(it);
                    }
                    Modal::EatMagic
                }
                "awaken" => {
                    // A floor symbiote to awaken (PWR_UNHYPNO).
                    let mut items = Vec::new();
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_HYPNOS, 1) {
                        let mut it = crate::item::Item::base(&gd, d);
                        it.note = 1;
                        it.pval2 = 5;
                        let (x, y) = player.single().map(|p| (p.x, p.y)).unwrap_or((0, 0));
                        items.push((x, y, 1u32));
                        crate::item::place_floor_item(
                            &mut commands,
                            &gd,
                            &floor.tiles,
                            &mut floor.stacks,
                            x,
                            y,
                            it,
                        );
                    }
                    Modal::Awaken { items }
                }
                "powerweak" => {
                    // Below both mana and safe HP: the weakened-state
                    // confirmation (powers.cc power_chance get_check).
                    ps.mana = 0;
                    ps.hp = 5;
                    Modal::PowerWeak { id: 40 }
                }
                "antimagic" => {
                    ps.skills.insert(crate::skill::SK_ANTIMAGIC, 30000);
                    Modal::Antimagic
                }
                "mindcraft" => {
                    ps.skills.insert(crate::skill::SK_MINDCRAFT, 30000);
                    ps.mana = 80;
                    ps.max_mana = 80;
                    Modal::Mindcraft { cursor: 0 }
                }
                "necromancy" => {
                    ps.skills.insert(crate::skill::SK_NECROMANCY, 40000);
                    ps.mana = 200;
                    ps.max_mana = 200;
                    Modal::Necromancy { cursor: 0 }
                }
                "geomancy" => {
                    // Develop the school and wield a magestaff.
                    for id in [
                        crate::skill::SK_FIRE,
                        crate::skill::SK_AIR,
                        crate::skill::SK_WATER,
                        crate::skill::SK_EARTH,
                        crate::skill::SK_GEOMANCY,
                    ] {
                        ps.skills.insert(id, 30000);
                    }
                    ps.mana = 200;
                    ps.max_mana = 200;
                    ps.stats[1] = 30;
                    ps.stats[2] = 30;
                    if let Some(d) = gd.object_by_tval_sval(crate::data::TV_MSTAFF, 1) {
                        let mut staff = crate::item::Item::base(&gd, d);
                        staff.identified = true;
                        inv.equip[crate::data::SLOT_WEAPON] = Some(staff);
                    }
                    Modal::Geomancy { cursor: 0 }
                }
                "demon" => {
                    // Daemonologist: develop Demonology and wear all three
                    // daemon books so every school spell is castable.
                    ps.level = 25;
                    ps.skills.insert(crate::skill::SK_DAEMON, 25000);
                    ps.mana = 200;
                    ps.max_mana = 200;
                    for (sval, slot) in [
                        (55, crate::data::SLOT_WEAPON),
                        (56, crate::data::SLOT_SHIELD),
                        (57, crate::data::SLOT_HEAD),
                    ] {
                        if let Some(d) = gd.object_by_tval_sval(crate::data::TV_DAEMON_BOOK, sval) {
                            let mut book = crate::item::Item::base(&gd, d);
                            book.identified = true;
                            inv.equip[slot] = Some(book);
                        }
                    }
                    Modal::Cast
                }
                "powermage" => {
                    ps.skills.insert(crate::skill::SK_THAUMATURGY, 30000);
                    ps.mana = 200;
                    ps.max_mana = 200;
                    let mut log = crate::game::MessageLog::default();
                    crate::game::gain_thaum_spells(&mut ps, &mut log, &mut rng);
                    Modal::Thaumaturgy { cursor: 0 }
                }
                "summon" => {
                    ps.skills.insert(crate::skill::SK_SUMMON, 30000);
                    Modal::Summon
                }
                "possessor" => {
                    if let Some(d) = gd.monster_by_name("Wolf") {
                        ps.possessed = Some((d, 30, 30));
                    }
                    Modal::Possessor
                }
                "guild" => Modal::Building(17),
                "fireproof" => {
                    plot.set(crate::game::PLOT_FIREPROOF, crate::game::PLOT_COMPLETED);
                    plot.fireproof_points = 24;
                    for name in ["Fire", "Identify", "Light"] {
                        if let Some(d) = gd.object_by_name(name) {
                            inv.pack.push(crate::item::Item::base(&gd, d));
                        }
                    }
                    Modal::Fireproof
                }
                "artifact" => {
                    if let Some(d) = gd.object_by_name("Long Sword") {
                        inv.pack.push(crate::item::Item::base(&gd, d));
                    }
                    let scroll = gd.object_by_name("Artifact Creation");
                    let idx = if let Some(d) = scroll {
                        inv.pack.push(crate::item::Item::base(&gd, d));
                        inv.pack.len() - 1
                    } else {
                        0
                    };
                    Modal::ArtifactCreate(idx)
                }
                "steal" => {
                    // Put a purse and an item on the nearest monster so
                    // the stealing window has entries.
                    let mut target = None;
                    for (_, mut m, p) in monsters.iter_mut() {
                        if m.gold == 0 && m.items.is_empty() {
                            m.gold = 250;
                            if let Some(d) = gd.object_by_name("Long Sword") {
                                m.items.push(crate::item::Item::base(&gd, d));
                            }
                            let items: Vec<String> =
                                m.items.iter().map(|it| it.label(&gd, &inv.known)).collect();
                            target = Some((p.x, p.y, m.gold, items));
                            break;
                        }
                    }
                    if let Some((x, y, gold, mut labels)) = target {
                        labels.insert(0, format!("{} gold pieces", gold));
                        Modal::Steal {
                            x,
                            y,
                            cursor: 0,
                            labels,
                            gold,
                        }
                    } else {
                        Modal::None
                    }
                }
                _ => Modal::None,
            };
        }
        // TOME_TEST_CAST=1 drives a geomancy cast: Enter selects the first
        // spell and the direction key fires it (cloud/terrain paths).
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "geomancy" {
            if f == 40 {
                keys.press(KeyCode::Enter);
            } else if f == 42 {
                keys.release(KeyCode::Enter);
            } else if f == 45 {
                keys.press(KeyCode::ArrowRight);
            } else if f == 47 {
                keys.release(KeyCode::ArrowRight);
            }
        }
        // TOME_TEST_CAST=1 with the demon modal casts a chosen spell
        // (TOME_TEST_CAST_SPELL, default 'a' = Demon Blade) at the left.
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "demon" {
            let name = std::env::var("TOME_TEST_CAST_SPELL").unwrap_or_else(|_| "a".into());
            let key = spell_key(&name).unwrap_or(KeyCode::KeyA);
            if f == 40 {
                keys.press(key);
            } else if f == 42 {
                keys.release(key);
            } else if f == 45 {
                keys.press(KeyCode::ArrowLeft);
            } else if f == 47 {
                keys.release(KeyCode::ArrowLeft);
            }
        }
        // TOME_TEST_CAST=1 with the blinkpower modal uses the first power
        // (a Gnome's blink) from the Powers menu.
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "blinkpower" {
            if f == 40 {
                keys.press(KeyCode::Enter);
            } else if f == 42 {
                keys.release(KeyCode::Enter);
            }
        }
        // TOME_TEST_CAST=1 with the powers modal uses its first entry
        // (Hobbit "create food").
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "powers" {
            if f == 40 {
                keys.press(KeyCode::Enter);
            } else if f == 42 {
                keys.release(KeyCode::Enter);
            }
        }
        // TOME_TEST_CAST=1 with the "power" modal uses the TOME_TEST_POWER
        // entry; TOME_TEST_POWER_DIR pushes an optional direction
        // (left/right/up/down) for targeted powers.
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "power" {
            let dir = std::env::var("TOME_TEST_POWER_DIR").unwrap_or_default();
            if f == 40 {
                keys.press(KeyCode::Enter);
            } else if f == 42 {
                keys.release(KeyCode::Enter);
            } else if f == 45 {
                match dir.as_str() {
                    "left" => keys.press(KeyCode::ArrowLeft),
                    "right" => keys.press(KeyCode::ArrowRight),
                    "up" => keys.press(KeyCode::ArrowUp),
                    "down" => keys.press(KeyCode::ArrowDown),
                    _ => {}
                }
            } else if f == 47 {
                match dir.as_str() {
                    "left" => keys.release(KeyCode::ArrowLeft),
                    "right" => keys.release(KeyCode::ArrowRight),
                    "up" => keys.release(KeyCode::ArrowUp),
                    "down" => keys.release(KeyCode::ArrowDown),
                    _ => {}
                }
            }
        }
        // TOME_TEST_CAST=1 with the throw/eatmagic/awaken modals picks
        // the first entry; throw then asks for a direction.
        // TOME_TEST_CAST=1 with the activate modal picks the first item
        // and confirms the ring's destruction (ACT_DEST_TELE).
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "activate" {
            if f == 40 {
                keys.press(KeyCode::KeyA);
            } else if f == 42 {
                keys.release(KeyCode::KeyA);
            } else if f == 45 {
                keys.press(KeyCode::KeyY);
            } else if f == 47 {
                keys.release(KeyCode::KeyY);
            }
        }
        if std::env::var("TOME_TEST_CAST").is_ok()
            && matches!(m.as_str(), "throw" | "eatmagic" | "awaken")
        {
            if f == 40 {
                keys.press(KeyCode::KeyA);
            } else if f == 42 {
                keys.release(KeyCode::KeyA);
            } else if f == 45 && m == "throw" {
                keys.press(KeyCode::ArrowRight);
            } else if f == 47 && m == "throw" {
                keys.release(KeyCode::ArrowRight);
            }
        }
        // TOME_TEST_CAST=1 with the ringpower modal drives one of the
        // ring_of_power choices (TOME_TEST_CAST_SPELL = s/r/c).
        if std::env::var("TOME_TEST_CAST").is_ok() && m == "ringpower" {
            let choice = std::env::var("TOME_TEST_CAST_SPELL").unwrap_or_else(|_| "c".into());
            let key = match choice.as_str() {
                "s" => KeyCode::KeyS,
                "r" => KeyCode::KeyR,
                _ => KeyCode::KeyC,
            };
            if f == 40 {
                keys.press(key);
            } else if f == 42 {
                keys.release(key);
            } else if f == 45 && choice == "c" {
                keys.press(KeyCode::ArrowLeft);
            } else if f == 47 && choice == "c" {
                keys.release(KeyCode::ArrowLeft);
            }
        }
        if f >= 60 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 80 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // TOME_TEST_FIREGOLEM=1: summon the Fire Golem, take control and walk
    // it east (spells3.cc fire_golem + monster3.cc control inputs).
    if std::env::var("TOME_TEST_FIREGOLEM").is_ok() {
        if f == 10 {
            ps.level = 25;
            ps.mana = 120;
            ps.max_mana = 120;
            ps.skills.insert(3, 25 * crate::skill::SKILL_STEP);
            ps.skills.insert(51, 25 * crate::skill::SKILL_STEP);
            if let Some(d) = gd.object_by_tval_sval(crate::data::TV_LITE, 0) {
                inv.pack.push(crate::item::Item::base(&gd, d));
            }
            *modal = Modal::FireGolem { cursor: 0 };
        }
        for (press, release, key) in [
            (20, 22, KeyCode::Enter),
            (30, 32, KeyCode::ArrowRight),
            (36, 38, KeyCode::ArrowRight),
            (42, 44, KeyCode::ArrowDown),
        ] {
            if f == press {
                keys.press(key);
            }
            if f == release {
                keys.release(key);
            }
        }
        if f >= 60 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 80 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // TOME_TEST_EVIL=1: drive the One Ring's three warnings and wear it.
    if std::env::var("TOME_TEST_EVIL").is_ok() {
        if f == 15 {
            if let Some(i) = inv.pack.iter().position(|it| it.artifact == 13) {
                *modal = Modal::RingWear { idx: i, step: 0 };
            }
        }
        for (press, release) in [(20, 21), (24, 25), (28, 29)] {
            if f == press {
                keys.press(KeyCode::KeyY);
            }
            if f == release {
                keys.release(KeyCode::KeyY);
            }
        }
        if f >= 40 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 60 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // TOME_TEST_VOID=1: stand on the Void's 149th level, hit the barrier
    // without an ULTIMATE artifact, then imbue one and descend to Melkor
    // (q_ultrag.cc / input.rs stair hooks).
    if std::env::var("TOME_TEST_VOID").is_ok() {
        let mut rng = crate::rng::current();
        if f == 12 {
            ps.level = 50;
            ps.max_hp = 600;
            ps.hp = 600;
            ps.max_mana = 600;
            ps.mana = 600;
            ps.dungeon = 11;
            ps.depth = 149;
            ps.recall_dungeon = 11;
            ps.recall_depth = 149;
            turn.pending = Some(crate::game::Goto::Dungeon(11, 149));
        }
        if f == 25 {
            if let Some(i) = map
                .terrain
                .iter()
                .position(|&t| t == crate::map::T_STAIRS_DOWN)
            {
                if let Ok(mut p) = player.single_mut() {
                    p.x = (i as i32) % crate::map::MAP_W;
                    p.y = (i as i32) / crate::map::MAP_W;
                    turn.fov_dirty = true;
                }
            }
        }
        for (press, release, key, shift) in [
            (30, 33, KeyCode::Period, true),
            (55, 58, KeyCode::Period, true),
        ] {
            if f == press {
                if shift {
                    keys.press(KeyCode::ShiftLeft);
                }
                keys.press(key);
            }
            if f == release {
                keys.release(key);
                if shift {
                    keys.release(KeyCode::ShiftLeft);
                }
            }
        }
        if f == 45 {
            // Imbue a long sword with the Eternal Flame (ULTIMATE).
            if let Some(a) = gd.artifacts.iter().find(|a| a.id == 147).map(|_| 147u32) {
                if let Some(mut it) = crate::item::specific_artifact_item(&gd, a, &mut rng) {
                    it.identified = true;
                    inv.equip[crate::data::SLOT_WEAPON] = Some(it);
                    ps.ac = crate::game::player_ac(&ps, &inv, &gd);
                }
            }
        }
        if f >= 90 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 110 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // Never get stuck in a shop window.
    if *modal != Modal::None {
        *modal = Modal::None;
    }
    let mut rng = crate::rng::current();
    // TOME_TEST_ENTER=1: start in Bree, stand on the Barrow-Downs stair
    // and press '>' (validates the dungeon entrance key path).
    if std::env::var("TOME_TEST_ENTER").is_ok() {
        if f == 12 {
            let (wx, wy) = gd.town_cell(1);
            ps.wild_x = wx;
            ps.wild_y = wy;
            turn.pending = Some(crate::game::Goto::Wild(wx, wy));
        }
        if f == 20 {
            if let Some(i) = map.special.keys().next().copied() {
                if let Ok(mut p) = player.single_mut() {
                    p.x = (i as i32) % crate::map::MAP_W;
                    p.y = (i as i32) / crate::map::MAP_W;
                    turn.fov_dirty = true;
                }
            }
        }
        if f == 24 {
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::Period);
        }
        if f == 26 {
            keys.release(KeyCode::ShiftLeft);
            keys.release(KeyCode::Period);
        }
        if f >= 60 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 80 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // TOME_TEST_WALK=1: walk south from the world start across area
    // borders into Bree (validates edge transitions and town entry).
    if std::env::var("TOME_TEST_WALK").is_ok() {
        if f >= 12 && f < 250 && f % 2 == 0 && turn.running.is_none() {
            turn.running = Some((0, 1, 3));
        }
        if f >= 260 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 290 {
            exit.write(AppExit::Success);
        }
        return;
    }
    // TOME_TEST_QUEST=thieves|trolls|wights|shroom|<quest id>: enter the
    // plot quest.
    if f == 12 {
        if let Ok(q) = std::env::var("TOME_TEST_QUEST") {
            let qid = match q.as_str() {
                "thieves" => crate::game::PLOT_THIEVES,
                "trolls" => crate::game::PLOT_TROLL,
                "wights" => crate::game::PLOT_WIGHT,
                "shroom" => crate::game::PLOT_SHROOM,
                other => other.parse().unwrap_or(0),
            };
            if qid > 0 {
                turn.pending = Some(crate::game::Goto::Depth(crate::game::PLOT_DEPTH_BASE + qid));
            }
        } else if std::env::var("TOME_TEST_TOWN").is_ok() {
            // Enter the random dungeon town picked at birth.
            turn.pending = Some(crate::game::Goto::Dungeon(ps.dungeon, ps.depth));
        } else if std::env::var("TOME_TEST_WILD").is_ok() {
            // Open the world overview (travel mode) and stay there.
            turn.pending = Some(crate::game::Goto::WildMap(ps.wild_x, ps.wild_y));
        } else if !ps.astral && std::env::var("TOME_STAY_TOWN").is_err() {
            // Leave for a dungeon right away (unless asked to stay, or
            // the character is an astral being already in Mandos).
            // TOME_START_DEPTH=n picks the dungeon depth (default 1); the
            // dungeon follows the depth (Barrow-Downs/Mirkwood/Mordor/
            // Angband) as in the original world map.
            let depth = std::env::var("TOME_START_DEPTH")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            let (dun, d) = match depth {
                1..=10 => (4, depth),
                11..=33 => (1, depth),
                34..=66 => (2, depth),
                _ => (3, depth.clamp(67, 127)),
            };
            turn.pending = Some(crate::game::Goto::Dungeon(dun, d));
        }
    }
    if f < 250 && f % 2 == 0 && turn.running.is_none() && std::env::var("TOME_TEST_FATE").is_err() {
        let (dx, dy) = (rng.gen_range(-1..=1), rng.gen_range(-1..=1));
        if dx != 0 || dy != 0 {
            turn.running = Some((dx, dy, 3));
        }
    }
    // TOME_TEST_DEMONBLADE=1: repeatedly stand next to a weak monster and
    // attack it, so the melee projection is actually exercised. The
    // character is kept alive on purpose (screenshot test, not a fight).
    if std::env::var("TOME_TEST_DEMONBLADE").is_ok() {
        if f % 25 == 5 {
            ps.max_hp = 9999;
            ps.hp = ps.max_hp;
            ps.stats[0] = 40; // STR: guaranteed melee hits for the test
            ps.hero = 100;
            let pp = player.single().map(|p| *p).ok();
            let nearest = pp.and_then(|pp| {
                monsters
                    .iter()
                    .min_by_key(|(_, _, mp)| (mp.x - pp.x).abs() + (mp.y - pp.y).abs())
                    .map(|(e, _, mp)| (e, mp.x, mp.y))
            });
            if let Some((e, mx, my)) = nearest {
                if let Ok((_, mut m, _)) = monsters.get_mut(e) {
                    m.hp = 1;
                }
                if let Ok(mut p) = player.single_mut() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (mx + dx, my + dy);
                        if map.walkable(&gd, nx, ny) {
                            p.x = nx;
                            p.y = ny;
                            break;
                        }
                    }
                }
            }
            turn.fov_dirty = true;
        }
        if f % 25 == 6 {
            if let Ok(p) = player.single() {
                let adjacent = monsters
                    .iter()
                    .find(|(_, _, mp)| (mp.x - p.x).abs() + (mp.y - p.y).abs() == 1)
                    .map(|(_, _, mp)| ((mp.x - p.x), (mp.y - p.y)));
                if let Some(key) = adjacent.and_then(|(dx, dy)| arrow_key(dx, dy)) {
                    keys.press(key);
                }
            }
        }
        if f % 25 == 8 {
            for key in [
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
            ] {
                keys.release(key);
            }
        }
    }
    // TOME_TEST_TOWN: stand in the random dungeon town's shop street.
    if std::env::var("TOME_TEST_TOWN").is_ok() {
        if f >= 15 && !auto.shot_taken {
            if let Some(&i) = map.shops.keys().next() {
                let (sx, sy) = (i as i32 % crate::map::MAP_W, i as i32 / crate::map::MAP_W);
                if let Ok(mut p) = player.single_mut() {
                    p.x = sx;
                    p.y = (sy + 3).min(crate::map::MAP_H - 2);
                }
                // Reveal the town for the screenshot (memory rendering).
                for y in sy - 30..=sy + 30 {
                    for x in sx - 40..=sx + 40 {
                        if crate::map::Map::in_bounds(x, y) {
                            let idx = crate::map::Map::idx(x, y);
                            if map.walkable(&gd, x, y) {
                                map.explored[idx] = true;
                            }
                        }
                    }
                }
                turn.fov_dirty = true;
            }
        }
        if f >= 40 && !auto.shot_taken {
            auto.shot_taken = true;
            let path = auto.shot_path.clone();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        if f >= 60 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if f >= 260 && !auto.shot_taken {
        auto.shot_taken = true;
        let path = auto.shot_path.clone();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if f >= 290 {
        exit.write(AppExit::Success);
    }
}

/// a-z -> KeyCode (bevy's KeyCode cannot be converted from an index).
fn spell_key(name: &str) -> Option<KeyCode> {
    Some(match name.chars().next()? {
        'a' => KeyCode::KeyA,
        'b' => KeyCode::KeyB,
        'c' => KeyCode::KeyC,
        'd' => KeyCode::KeyD,
        'e' => KeyCode::KeyE,
        'f' => KeyCode::KeyF,
        'g' => KeyCode::KeyG,
        'h' => KeyCode::KeyH,
        'i' => KeyCode::KeyI,
        'j' => KeyCode::KeyJ,
        'k' => KeyCode::KeyK,
        'l' => KeyCode::KeyL,
        'm' => KeyCode::KeyM,
        'n' => KeyCode::KeyN,
        'o' => KeyCode::KeyO,
        'p' => KeyCode::KeyP,
        'q' => KeyCode::KeyQ,
        'r' => KeyCode::KeyR,
        's' => KeyCode::KeyS,
        't' => KeyCode::KeyT,
        'u' => KeyCode::KeyU,
        'v' => KeyCode::KeyV,
        'w' => KeyCode::KeyW,
        'x' => KeyCode::KeyX,
        'y' => KeyCode::KeyY,
        'z' => KeyCode::KeyZ,
        _ => return None,
    })
}

/// Cardinal direction -> arrow key.
fn arrow_key(dx: i32, dy: i32) -> Option<KeyCode> {
    Some(match (dx, dy) {
        (0, -1) => KeyCode::ArrowUp,
        (0, 1) => KeyCode::ArrowDown,
        (-1, 0) => KeyCode::ArrowLeft,
        (1, 0) => KeyCode::ArrowRight,
        _ => return None,
    })
}
