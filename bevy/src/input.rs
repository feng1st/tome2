//! Keyboard input: ToME key bindings, both the original keyset
//! (digits / numpad) and the roguelike keyset (yuhjklbn).

use bevy::ecs::system::{Local, SystemParam};
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashSet;

use crate::data::{self, Dice, GameData};
use crate::game::{self, GridPos, MessageLog, Monster, Player, PlayerState, TurnState};
use crate::item::{self, FloorGold, FloorItem, Inventory};
use crate::map::{
    self, Map, T_FLOOR, T_SHAFT_DOWN, T_SHAFT_UP, T_STAIRS_DOWN, T_STAIRS_UP, T_WAY_LESS,
    T_WAY_MORE,
};
use crate::modal::Modal;
use crate::render::TileAssets;
use crate::town::{self, ShopStocks};
use crate::AppState;

/// Last sval of TV_POTION (defines.hpp SV_POTION_LAST): TV_POTION2
/// fountain flavours are encoded as `sval + SV_POTION_LAST`.
const SV_POTION_LAST: i32 = 63;

#[derive(SystemParam)]
pub struct InputCtx<'w, 's> {
    pub consumed: Res<'w, crate::modal::ModalInputConsumed>,
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    pub gd: Res<'w, GameData>,
    pub tiles: Res<'w, TileAssets>,
    pub map: ResMut<'w, Map>,
    pub player: Query<'w, 's, &'static mut GridPos, With<Player>>,
    pub monsters: Query<'w, 's, (Entity, &'static mut Monster, &'static GridPos), Without<Player>>,
    /// Monster experience/state added by the 19th-round port (save path).
    pub monster_extras: Query<'w, 's, &'static game::MonsterExtra>,
    pub stacks: Query<'w, 's, (Entity, &'static GridPos, &'static mut FloorItem), Without<Player>>,
    pub golds: Query<'w, 's, (Entity, &'static GridPos, &'static FloorGold), Without<Player>>,
    pub ps: ResMut<'w, PlayerState>,
    pub inv: ResMut<'w, Inventory>,

    pub plot: ResMut<'w, game::PlotQuest>,
    pub stocks: ResMut<'w, ShopStocks>,
    pub store: ResMut<'w, crate::save::LevelStore>,
    pub created: ResMut<'w, item::CreatedArtifacts>,
    pub modal: ResMut<'w, Modal>,
    pub log: ResMut<'w, MessageLog>,
    pub turn: ResMut<'w, TurnState>,
    pub repeat: ResMut<'w, MoveRepeat>,
    /// Global gameplay options (cmd4.cc / options.hpp).
    pub options: Res<'w, crate::options::Options>,
    /// Character notes (notes.cc).
    pub notes: ResMut<'w, crate::notes::Notes>,
    /// The automatizer rule set (squeltch.cc).
    pub automatizer: ResMut<'w, crate::squeltch::Automatizer>,
    /// Companion AI switches changed by the pet menu (cmd1.cc do_cmd_pet).
    pub pet_opts: ResMut<'w, PetOptions>,
    /// State of the companion command menu.
    pub pet: ResMut<'w, PetMenu>,
    /// State of the drop/destroy item selection (quantities).
    pub picker: ResMut<'w, ItemPicker>,
    pub time: Res<'w, Time>,
    pub commands: Commands<'w, 's>,
    pub exit: MessageWriter<'w, AppExit>,
    pub next: ResMut<'w, NextState<AppState>>,
    /// Corridor-following run state (cmd1.cc run_init/run_test).
    pub run: Local<'s, RunState>,
    /// Floor cells whose freshly spawned drop piles still need the
    /// automatizer sweep (spawned via deferred Commands, so they are only
    /// visible on the following frame).
    pub squeltch_pending: Local<'s, Vec<(i32, i32)>>,
}

const DIRS: &[(KeyCode, (i32, i32))] = &[
    // Roguelike keyset
    (KeyCode::KeyY, (-1, -1)),
    (KeyCode::KeyK, (0, -1)),
    (KeyCode::KeyU, (1, -1)),
    (KeyCode::KeyH, (-1, 0)),
    (KeyCode::KeyL, (1, 0)),
    (KeyCode::KeyB, (-1, 1)),
    (KeyCode::KeyJ, (0, 1)),
    (KeyCode::KeyN, (1, 1)),
    // Original keyset (digits + numpad)
    (KeyCode::Digit8, (0, -1)),
    (KeyCode::Digit2, (0, 1)),
    (KeyCode::Digit4, (-1, 0)),
    (KeyCode::Digit6, (1, 0)),
    (KeyCode::Digit7, (-1, -1)),
    (KeyCode::Digit9, (1, -1)),
    (KeyCode::Digit1, (-1, 1)),
    (KeyCode::Digit3, (1, 1)),
    (KeyCode::Numpad8, (0, -1)),
    (KeyCode::Numpad2, (0, 1)),
    (KeyCode::Numpad4, (-1, 0)),
    (KeyCode::Numpad6, (1, 0)),
    (KeyCode::Numpad7, (-1, -1)),
    (KeyCode::Numpad9, (1, -1)),
    (KeyCode::Numpad1, (-1, 1)),
    (KeyCode::Numpad3, (1, 1)),
    // Arrows (convenience)
    (KeyCode::ArrowUp, (0, -1)),
    (KeyCode::ArrowDown, (0, 1)),
    (KeyCode::ArrowLeft, (-1, 0)),
    (KeyCode::ArrowRight, (1, 0)),
];

pub fn direction_key(keys: &ButtonInput<KeyCode>) -> Option<(i32, i32)> {
    DIRS.iter()
        .find(|(k, _)| keys.just_pressed(*k))
        .map(|(_, d)| *d)
}

/// Direction keys currently held down (for movement repeating).
pub fn direction_held(keys: &ButtonInput<KeyCode>) -> Option<(i32, i32)> {
    DIRS.iter().find(|(k, _)| keys.pressed(*k)).map(|(_, d)| *d)
}

/// How a rest ends (cmd2.cc do_cmd_rest / dungeon.cc:3743).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RestMode {
    #[default]
    None,
    /// A fixed number of turns.
    Turns,
    /// Rest until HP and SP are full (`*`).
    Full,
    /// Rest as needed (`&`).
    AsNeeded,
}

/// Hold-to-move state: after an initial delay the step repeats quickly.
#[derive(Resource, Default)]
pub struct MoveRepeat {
    pub dir: Option<(i32, i32)>,
    /// Time (seconds) when the next repeated step fires.
    pub next: f32,
    /// How the current rest ends (cmd2.cc do_cmd_rest).
    pub rest_mode: RestMode,
    /// Text being typed into the rest prompt (None = closed).
    pub rest_prompt: Option<String>,
    /// `p_ptr->confusing` (cmd6.cc Scroll of Monster Confusion): the next
    /// landed melee blow confuses the victim with glowing hands
    /// (cmd1.cc:2332).  PlayerState (game.rs) owns the persistent field;
    /// this is its temporary home.
    pub confusing: bool,
    /// `p_ptr->immov_cntr` (cmd2.cc do_cmd_immovable_special): the
    /// mana/HP cost of the next immovable power.  C++ keeps it in
    /// player_type; it lives here until PlayerState gains the field.
    pub immov_cntr: i32,
    /// `ps.turn` at the last immov_cntr decrement.
    pub immov_turn: u64,
    /// Numeric command prefix typed after '0' (util.cc request_command
    /// `get_number`).
    pub command_arg: i32,
    /// Digits typed while the "Count:" prompt is open.
    pub count_prompt: Option<String>,
    /// `command_rep`: repeats left of the last repeatable command
    /// (cmd2.cc allow_repeat_command).
    pub command_rep: i32,
    /// The command being repeated and its chosen direction.
    pub repeat_cmd: Option<(String, i32, i32)>,
    /// `ps.turn` when the repeat last ran (one repeat per player turn).
    pub repeat_turn: u64,
}

// --- Companion command menu (cmd1.cc do_cmd_pet) ---

/// The ten `do_cmd_pet` commands (cmd1.cc:3896-3920).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PetCommand {
    /// 1: dismiss pets.
    DismissPets,
    /// 10: dismiss companions.
    DismissCompanions,
    /// 2: call pets (`pet_follow_distance = 1`).
    CallPets,
    /// 6: follow me (`pet_follow_distance = 6`).
    FollowMe,
    /// 3: seek and destroy (`pet_follow_distance = 255`).
    SeekAndDestroy,
    /// 4: toggle pets opening doors.
    ToggleDoors,
    /// 5: toggle pets picking items up (dropping what they carry).
    TogglePickup,
    /// 7: give target to a friend.
    GiveTargetFriend,
    /// 8: give target to all friends.
    GiveTargetAll,
    /// 9: friend forgets its target.
    ForgetTarget,
}

impl PetCommand {
    /// `power_desc[]` (cmd1.cc), with the two toggles reflecting state.
    pub fn desc(self, opts: &PetOptions) -> &'static str {
        match self {
            PetCommand::DismissPets => "dismiss pets",
            PetCommand::DismissCompanions => "dismiss companions",
            PetCommand::CallPets => "call pets",
            PetCommand::FollowMe => "follow me",
            PetCommand::SeekAndDestroy => "seek and destroy",
            PetCommand::ToggleDoors => {
                if opts.open_doors {
                    "disallow open doors"
                } else {
                    "allow open doors"
                }
            }
            PetCommand::TogglePickup => {
                if opts.pickup_items {
                    "disallow pickup items"
                } else {
                    "allow pickup items"
                }
            }
            PetCommand::GiveTargetFriend => "give target to a friend",
            PetCommand::GiveTargetAll => "give target to all friends",
            PetCommand::ForgetTarget => "friend forget target",
        }
    }
}

/// Pet AI switches (cmd1.cc p_ptr->pet_follow_distance / pet_open_doors /
/// pet_pickup_items; birth.cc:691 defaults).
///
/// NOTE: `game.rs` currently hardcodes `PET_FOLLOW_DISTANCE = 6`,
/// `PET_OPEN_DOORS = false` and `PET_PICKUP_ITEMS = false` (consts at
/// `game.rs:13256-13262`), so the follow-distance and the two flags are
/// stored here for the eventual consumer: `game.rs::monster_turns` should
/// read this resource (the pickup toggle's immediate drop of carried
/// objects is already applied here in `pet_command`).
#[derive(Resource, Clone, Debug)]
pub struct PetOptions {
    pub follow_distance: i32,
    pub open_doors: bool,
    pub pickup_items: bool,
}

impl Default for PetOptions {
    fn default() -> Self {
        // birth.cc:691 pet_follow_distance = 6, both flags false.
        PetOptions {
            follow_distance: 6,
            open_doors: false,
            pickup_items: false,
        }
    }
}

/// Open companion menu state and a pending target selection.
#[derive(Resource, Default)]
pub struct PetMenu {
    pub open: bool,
    pub commands: Vec<PetCommand>,
    /// A "give target"/"forget target" command waiting for a direction.
    pub pending_target: Option<PetCommand>,
    /// Upper-case menu letter awaiting a `[y/n]` verification.
    pub confirm: Option<usize>,
}

// --- Drop/destroy with quantity prompts (cmd3.cc do_cmd_drop/do_cmd_destroy) ---

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickerMode {
    /// cmd3.cc do_cmd_drop (USE_EQUIP | USE_INVEN).
    Drop,
    /// cmd3.cc do_cmd_destroy (USE_INVEN | USE_FLOOR | USE_AUTO).
    Destroy,
}

/// Which list the picker shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickerView {
    Pack,
    Equip,
    Floor,
}

/// One selected item; `count` is the stack size offered.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PickerSel {
    Pack(usize),
    Equip(usize),
    /// (floor stack entity, index within the stack).
    Floor(Entity, usize),
}

impl PickerSel {
    pub fn count(&self, ctx: &InputCtx) -> u32 {
        match self {
            PickerSel::Pack(i) => ctx.inv.pack.get(*i).map(|it| it.count).unwrap_or(1),
            PickerSel::Equip(_) => 1,
            PickerSel::Floor(e, i) => ctx
                .stacks
                .get(*e)
                .ok()
                .and_then(|(_, _, s)| s.stack.get(*i).map(|it| it.count))
                .unwrap_or(1),
        }
    }
}

/// The text-mode drop/destroy selection (the port has no full-screen item
/// selector; this mirrors `get_item` + `get_quantity` from cmd3.cc).
#[derive(Resource, Default)]
pub struct ItemPicker {
    pub mode: Option<PickerMode>,
    pub view: PickerView,
    pub sel: Option<PickerSel>,
    /// Digits typed for the quantity prompt; `Some("")` = prompt open.
    pub qty: Option<String>,
    /// The `$` toggle of the original item selector (`automatizer_create`):
    /// creating an automatizer rule for the selected object.
    pub create_rule: bool,
}

impl Default for PickerView {
    fn default() -> Self {
        PickerView::Pack
    }
}

// --- Corridor-following run (cmd1.cc run_init/run_test/run_step) ---

/// Original direction codes: 1=SW 2=S 3=SE 4=W 6=E 7=NW 8=N 9=NE.
const DDX: [i32; 10] = [0, -1, 0, 1, -1, 0, 1, -1, 0, 1];
const DDY: [i32; 10] = [0, 1, 1, 1, 0, 0, 0, -1, -1, -1];
/// Cycle through the legal directions (cmd1.cc).
const CYCLE: [i32; 17] = [1, 2, 3, 6, 9, 8, 7, 4, 1, 2, 3, 6, 9, 8, 7, 4, 1];
/// Map each direction into the middle of the cycle array (cmd1.cc).
const CHOME: [i32; 10] = [0, 8, 9, 10, 7, 0, 11, 6, 5, 4];
/// ddx_ddd/ddy_ddd (tables.cc), indexed 0..9 with 8 = the own grid.
const DDD: [(i32, i32); 9] = [
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
    (0, 0),
];

/// Run state saved between frames (the original file-scope find_* vars and
/// the `running` counter).
#[derive(Clone, Copy, Debug, Default)]
pub struct RunState {
    pub active: bool,
    /// Direction the run currently follows.
    pub current: i32,
    /// Direction the runner came from.
    pub prevdir: i32,
    /// Looking for an open area (rather than a break).
    pub openarea: bool,
    pub breakright: bool,
    pub breakleft: bool,
    /// Remaining steps (cap 1000).
    pub steps: i32,
    /// Time of the first Ctrl+Q press; a second press commits suicide.
    pub confirm_suicide: f32,
}

fn dir_from_delta(dx: i32, dy: i32) -> i32 {
    match (dx, dy) {
        (0, 1) => 2,
        (-1, 1) => 1,
        (1, 1) => 3,
        (-1, 0) => 4,
        (1, 0) => 6,
        (-1, -1) => 7,
        (0, -1) => 8,
        (1, -1) => 9,
        _ => 0,
    }
}

/// p_ptr->ffall || p_ptr->fly (cmd1.cc).
fn run_levitates(gd: &GameData, ps: &PlayerState, inv: &Inventory) -> bool {
    if ps.tim_ffall > 0 || ps.tim_fly > 0 {
        return true;
    }
    let tot = inv.totals_for(gd, ps);
    tot.feather || tot.fly || game::god_feather(ps) || game::god_fly(ps)
}

/// see_obstacle_grid (cmd1.cc:3092): a known wall or dangerous feature.
fn see_obstacle_grid(
    map: &Map,
    gd: &GameData,
    ps: &PlayerState,
    inv: &Inventory,
    x: i32,
    y: i32,
) -> bool {
    if !Map::in_bounds(x, y) {
        return false;
    }
    let t = map.terrain_at(x, y);
    // Levitation/flight clears the pit/water/ice cases; the original's
    // switch falls through from them into the lava immunity case
    // (cmd1.cc:3097-3113).
    if matches!(t, map::T_DARK_PIT | map::T_DEEP_WATER | map::T_ICE) {
        if run_levitates(gd, ps, inv) {
            return false;
        }
        if ps.invuln > 0 || inv.totals_for(gd, ps).immunities.contains("FIRE") {
            return false;
        }
    }
    if matches!(t, map::T_LAVA | map::T_SHAL_LAVA)
        && (ps.invuln > 0 || inv.totals_for(gd, ps).immunities.contains("FIRE"))
    {
        return false;
    }
    // "Safe" floor grids aren't obstacles (FF_CAN_RUN).
    if gd.terrain(t).can_run {
        return false;
    }
    // Must be known to the player.
    if !map.explored[Map::idx(x, y)] {
        return false;
    }
    true
}

/// see_obstacle (cmd1.cc:3130).
fn see_obstacle(
    map: &Map,
    gd: &GameData,
    ps: &PlayerState,
    inv: &Inventory,
    dir: i32,
    y: i32,
    x: i32,
) -> bool {
    let (ny, nx) = (y + DDY[dir as usize], x + DDX[dir as usize]);
    if !Map::in_bounds(nx, ny) {
        return false;
    }
    see_obstacle_grid(map, gd, ps, inv, nx, ny)
}

/// see_nothing (cmd1.cc:3147): an unknown corner.
#[allow(dead_code)]
fn see_nothing(
    map: &Map,
    gd: &GameData,
    _ps: &PlayerState,
    _inv: &Inventory,
    dir: i32,
    y: i32,
    x: i32,
) -> bool {
    let (ny, nx) = (y + DDY[dir as usize], x + DDX[dir as usize]);
    if !Map::in_bounds(nx, ny) {
        return true;
    }
    let idx = Map::idx(nx, ny);
    if map.explored[idx] {
        return false;
    }
    if !gd.terrain(map.terrain_at(nx, ny)).is_floor {
        return true;
    }
    if map.visible[idx] {
        return false;
    }
    true
}

/// run_init (cmd1.cc:3351): initialize the run algorithm for a direction.
fn run_init(
    map: &Map,
    gd: &GameData,
    ps: &PlayerState,
    inv: &Inventory,
    py: i32,
    px: i32,
    dir: i32,
) -> RunState {
    let mut st = RunState {
        active: true,
        current: dir,
        prevdir: dir,
        openarea: true,
        ..Default::default()
    };
    let (row, col) = (py + DDY[dir as usize], px + DDX[dir as usize]);
    let i = CHOME[dir as usize] as usize;
    let (mut deepleft, mut deepright) = (false, false);
    let (mut shortleft, mut shortright) = (false, false);
    if see_obstacle(map, gd, ps, inv, CYCLE[i + 1], py, px) {
        st.breakleft = true;
        shortleft = true;
    } else if see_obstacle(map, gd, ps, inv, CYCLE[i + 1], row, col) {
        st.breakleft = true;
        deepleft = true;
    }
    if see_obstacle(map, gd, ps, inv, CYCLE[i - 1], py, px) {
        st.breakright = true;
        shortright = true;
    } else if see_obstacle(map, gd, ps, inv, CYCLE[i - 1], row, col) {
        st.breakright = true;
        deepright = true;
    }
    if st.breakleft && st.breakright {
        st.openarea = false;
        if dir & 0x01 != 0 {
            if deepleft && !deepright {
                st.prevdir = CYCLE[i - 1];
            } else if deepright && !deepleft {
                st.prevdir = CYCLE[i + 1];
            }
        } else if see_obstacle(map, gd, ps, inv, CYCLE[i], row, col) {
            if shortleft && !shortright {
                st.prevdir = CYCLE[i - 2];
            } else if shortright && !shortleft {
                st.prevdir = CYCLE[i + 2];
            }
        }
    }
    st
}

/// run_test (cmd1.cc:3445): update the run path. Returns true when the run
/// should stop.
fn run_test(ctx: &mut InputCtx) -> bool {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return true;
    };
    let (py, px) = (pos.y, pos.x);
    let prev_dir = ctx.run.prevdir;
    let max = (prev_dir & 0x01) + 1;
    let mut option = 0i32;
    let mut option2 = 0i32;
    let map = &ctx.map;
    let gd = &ctx.gd;
    let ps = &ctx.ps;
    let inv = &ctx.inv;
    for i in -max..=max {
        let new_dir = CYCLE[(CHOME[prev_dir as usize] + i) as usize];
        let (row, col) = (py + DDY[new_dir as usize], px + DDX[new_dir as usize]);
        if !Map::in_bounds(col, row) {
            continue;
        }
        let idx = Map::idx(col, row);
        if ctx
            .monsters
            .iter()
            .any(|(_, m, p)| p.x == col && p.y == row && m.hp > 0 && map.visible[idx])
        {
            return true;
        }
        if ctx
            .stacks
            .iter()
            .any(|(_, p, s)| p.x == col && p.y == row && !s.stack.is_empty() && map.visible[idx])
        {
            return true;
        }
        let mut unknown = true;
        if map.explored[idx] {
            let t = map.terrain_at(col, row);
            let mut notice = true;
            match t {
                map::T_LAVA | map::T_SHAL_LAVA => {
                    if ps.invuln > 0 || inv.totals_for(gd, ps).immunities.contains("FIRE") {
                        notice = false;
                    }
                }
                map::T_DEEP_WATER | map::T_ICE => {
                    if run_levitates(gd, ps, inv) {
                        notice = false;
                    }
                }
                map::T_OPEN_DOOR | 5 => notice = false,
                _ => {}
            }
            if gd.terrain(t).dont_notice_running {
                notice = false;
            }
            if notice {
                return true;
            }
            unknown = false;
        }
        let (row_f, col_f) = (row, col);
        if unknown || gd.terrain(map.terrain_at(col_f, row_f)).is_floor {
            if !ctx.run.openarea {
                if option == 0 {
                    option = new_dir;
                } else if option2 != 0 {
                    return true;
                } else if option != CYCLE[(CHOME[prev_dir as usize] + i - 1) as usize] {
                    return true;
                } else if new_dir & 0x01 != 0 {
                    option2 = new_dir;
                } else {
                    option2 = option;
                    option = new_dir;
                }
            }
        } else if ctx.run.openarea {
            if i < 0 {
                ctx.run.breakright = true;
            } else if i > 0 {
                ctx.run.breakleft = true;
            }
        }
    }
    if ctx.run.openarea {
        for i in -max..0 {
            let new_dir = CYCLE[(CHOME[prev_dir as usize] + i) as usize];
            let (row, col) = (py + DDY[new_dir as usize], px + DDX[new_dir as usize]);
            if !Map::in_bounds(col, row) {
                continue;
            }
            if !see_obstacle_grid(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, col, row) {
                if ctx.run.breakright {
                    return true;
                }
            } else if ctx.run.breakleft {
                return true;
            }
        }
        for i in (1..=max).rev() {
            let new_dir = CYCLE[(CHOME[prev_dir as usize] + i) as usize];
            let (row, col) = (py + DDY[new_dir as usize], px + DDX[new_dir as usize]);
            if !Map::in_bounds(col, row) {
                continue;
            }
            if !see_obstacle_grid(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, col, row) {
                if ctx.run.breakleft {
                    return true;
                }
            } else if ctx.run.breakright {
                return true;
            }
        }
    } else if option == 0 {
        return true;
    } else if option2 == 0 {
        ctx.run.current = option;
        ctx.run.prevdir = option;
    } else {
        // find_examine = true, find_cut = false (default options):
        // take the primary option but allow curving.
        ctx.run.current = option;
        ctx.run.prevdir = option2;
    }
    if see_obstacle(
        &ctx.map,
        &ctx.gd,
        &ctx.ps,
        &ctx.inv,
        ctx.run.current,
        py,
        px,
    ) {
        return true;
    }
    false
}

/// run_step (cmd1.cc:3808) driven one frame at a time.
fn advance_run(ctx: &mut InputCtx) {
    if run_test(ctx) {
        ctx.run.active = false;
        ctx.turn.running = None;
        return;
    }
    ctx.run.steps -= 1;
    if ctx.run.steps <= 0 {
        ctx.run.active = false;
        return;
    }
    let dir = ctx.run.current;
    let (dx, dy) = (DDX[dir as usize], DDY[dir as usize]);
    let before = ctx.player.single().map(|p| (p.x, p.y)).ok();
    step_player(ctx, dx, dy, false);
    let after = ctx.player.single().map(|p| (p.x, p.y)).ok();
    // A blocked step (unknown wall, forced door) or a bump attack stops
    // the run, as disturb() does in the original. Stairs/quests stop it
    // too, before the level actually changes.
    if before == after || ctx.turn.pending.is_some() {
        ctx.run.active = false;
        ctx.turn.running = None;
    }
}

/// do_cmd_run_run (cmd2.cc:2044): start running in the given direction.
fn start_run(ctx: &mut InputCtx, dir: i32, steps: i32) {
    if !(1..=9).contains(&dir) {
        return;
    }
    if ctx.ps.confuse > 0 {
        ctx.log.add("You are too confused!");
        return;
    }
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return;
    };
    let (nx, ny) = (pos.x + DDX[dir as usize], pos.y + DDY[dir as usize]);
    let tree = ctx.map.terrain_at(nx, ny) == map::T_TREE;
    if see_obstacle(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, dir, pos.y, pos.x) && !tree {
        ctx.log.add("You cannot run in that direction.");
        return;
    }
    let st = run_init(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, pos.y, pos.x, dir);
    *ctx.run = st;
    ctx.run.steps = steps;
    advance_run(ctx);
}

/// Input while the player drives a controlled monster (dungeon.cc
/// process_command + monster3.cc do_control_*): movement sets the
/// creature's direction, and pick-up/drop/inventory/powers are free or
/// monster actions. Everything else is blocked.
fn control_input(ctx: &mut InputCtx) {
    let keys = ctx.keys.clone();
    // Locate the creature; if it is gone, control lapses.
    let Some((mx, my)) = ctx
        .monsters
        .iter()
        .find(|(_, m, _)| m.controlled)
        .map(|(_, _, p)| (p.x, p.y))
    else {
        ctx.ps.control = None;
        ctx.ps.control_power = None;
        ctx.log.add("You lose control of the creature.");
        return;
    };
    if let Some((dx, dy)) = direction_key(&keys) {
        ctx.ps.control_dir = (dx, dy);
        ctx.ps.control_pos = Some((mx, my));
        consume_turn(ctx);
        return;
    }
    // ',' or 'g': pick up everything under the creature (free).
    if keys.just_pressed(KeyCode::Comma) || keys.just_pressed(KeyCode::KeyG) {
        let mut taken: Vec<item::Item> = Vec::new();
        for (_, p, stack) in ctx.stacks.iter() {
            if p.x == mx && p.y == my {
                taken.extend(stack.stack.iter().cloned());
            }
        }
        if taken.is_empty() {
            ctx.log.add("There is nothing here to pick up.");
            return;
        }
        for (e, p, mut stack) in ctx.stacks.iter_mut() {
            if p.x == mx && p.y == my {
                stack.stack.clear();
                let _ = e;
            }
        }
        if let Some((_, mut m, _)) = ctx.monsters.iter_mut().find(|(_, m, _)| m.controlled) {
            m.items.extend(taken);
        }
        ctx.log.add("The creature picks up the items.");
        return;
    }
    // 'd': drop everything the creature carries (free).
    if keys.just_pressed(KeyCode::KeyD) {
        let items = match ctx.monsters.iter_mut().find(|(_, m, _)| m.controlled) {
            Some((_, mut m, _)) => std::mem::take(&mut m.items),
            None => Vec::new(),
        };
        if items.is_empty() {
            ctx.log.add("The creature carries nothing.");
            return;
        }
        for it in items {
            item::place_floor_item(
                &mut ctx.commands,
                &ctx.gd,
                &ctx.tiles,
                &mut ctx.stacks,
                mx,
                my,
                it,
            );
        }
        ctx.log.add("The creature drops its burden.");
        return;
    }
    // 'i': list what the creature carries (free).
    if keys.just_pressed(KeyCode::KeyI) {
        let mut lines: Vec<String> = Vec::new();
        if let Some((_, m, _)) = ctx.monsters.iter().find(|(_, m, _)| m.controlled) {
            for it in &m.items {
                lines.push(it.label(&ctx.gd, &ctx.inv.known));
            }
        }
        if lines.is_empty() {
            lines.push("(nothing)".to_string());
        }
        *ctx.modal = Modal::TextPage {
            title: "The creature's burden".to_string(),
            lines,
            page: 0,
        };
        return;
    }
    // 'm': use a power or abandon the creature (monster3.cc do_control_magic).
    if keys.just_pressed(KeyCode::KeyM) {
        *ctx.modal = Modal::ControlMagic { cursor: 0 };
    }
}

pub fn player_input(mut ctx: InputCtx) {
    // `immov_cntr` decays once per player turn (dungeon.cc process_player).
    if ctx.ps.turn != ctx.repeat.immov_turn {
        ctx.repeat.immov_turn = ctx.ps.turn;
        if ctx.repeat.immov_cntr > 0 {
            ctx.repeat.immov_cntr -= 1;
        }
    }
    let keys = ctx.keys.clone();
    // A modal window (inventory, shop, ...) grabs all input; this also
    // covers the frame in which modal_input closed one.
    if *ctx.modal != Modal::None || ctx.consumed.0 {
        if *ctx.modal != Modal::None {
            ctx.run.active = false;
        }
        return;
    }
    // files.cc autosave_checkpoint / dungeon.cc timed autosave: perform the
    // save here, where the full game context is available.
    if ctx.turn.autosave_due {
        ctx.turn.autosave_due = false;
        ctx.log.add("Autosaving the game...");
        let save = build_save(&mut ctx);
        crate::save::save_player(&save);
    }
    // `Modal::Possess` (modal.rs) integrated a corpse into a disembodied
    // spirit; clear the flag now that a body exists again.
    if ctx.ps.disembodied && ctx.ps.possessed.is_some() {
        ctx.ps.disembodied = false;
    }
    // Floor cells with deferred drop piles from last frame (kills): the
    // entities exist now, so the automatizer can see them.
    let pending = std::mem::take(&mut *ctx.squeltch_pending);
    for (x, y) in pending {
        squeltch_grid(&mut ctx, x, y);
    }
    // The notes prompt, the companion menu and the drop/destroy picker are
    // text-mode states owned by this module (they never open a Modal).
    if ctx.notes.input.is_some() {
        note_input(&mut ctx);
        return;
    }
    if ctx.pet.open {
        pet_menu_input(&mut ctx);
        return;
    }
    if let Some(cmd) = ctx.pet.pending_target {
        pet_target_input(&mut ctx, cmd);
        return;
    }
    if ctx.picker.mode.is_some() {
        item_picker_input(&mut ctx);
        return;
    }
    // The rest-duration prompt (cmd2.cc do_cmd_rest).
    if ctx.repeat.rest_prompt.is_some() {
        rest_prompt_input(&mut ctx);
        return;
    }
    // The numeric command count (util.cc request_command: '0' opens
    // "Count:" and digits follow; the count repeats the next command).
    if let Some(mut text) = ctx.repeat.count_prompt.clone() {
        let mut digit = None;
        for (k, c) in [
            (KeyCode::Digit0, '0'),
            (KeyCode::Digit1, '1'),
            (KeyCode::Digit2, '2'),
            (KeyCode::Digit3, '3'),
            (KeyCode::Digit4, '4'),
            (KeyCode::Digit5, '5'),
            (KeyCode::Digit6, '6'),
            (KeyCode::Digit7, '7'),
            (KeyCode::Digit8, '8'),
            (KeyCode::Digit9, '9'),
            (KeyCode::Numpad0, '0'),
            (KeyCode::Numpad1, '1'),
            (KeyCode::Numpad2, '2'),
            (KeyCode::Numpad3, '3'),
            (KeyCode::Numpad4, '4'),
            (KeyCode::Numpad5, '5'),
            (KeyCode::Numpad6, '6'),
            (KeyCode::Numpad7, '7'),
            (KeyCode::Numpad8, '8'),
            (KeyCode::Numpad9, '9'),
        ] {
            if keys.just_pressed(k) {
                digit = Some(c);
                break;
            }
        }
        if keys.just_pressed(KeyCode::Escape) {
            ctx.repeat.count_prompt = None;
            return;
        }
        if keys.just_pressed(KeyCode::Backspace) {
            text.pop();
            ctx.repeat.count_prompt = Some(text);
            return;
        }
        if let Some(c) = digit {
            if text.len() < 4 {
                text.push(c);
                ctx.repeat.count_prompt = Some(text);
            }
            return;
        }
        // Enter/Space (or any other command key) ends the count; the
        // command itself is processed below.
        ctx.repeat.count_prompt = None;
        let v = text.parse::<i32>().unwrap_or(0);
        ctx.repeat.command_arg = if v == 0 { 99 } else { v };
    }
    // '0' opens the count prompt (util.cc:2946).
    if keys.just_pressed(KeyCode::Digit0)
        && !ctx.keys.pressed(KeyCode::ShiftLeft)
        && !ctx.keys.pressed(KeyCode::ShiftRight)
    {
        ctx.repeat.count_prompt = Some(String::new());
        return;
    }
    // Repeats of the last repeatable command (cmd2.cc command_rep): one
    // execution per player turn; any key press disturbs them.
    if ctx.repeat.command_rep > 0 && ctx.ps.turn != ctx.repeat.repeat_turn {
        if keys.get_just_pressed().next().is_some()
            || ctx.turn.pending_cmd.is_some()
            || *ctx.modal != Modal::None
        {
            ctx.repeat.command_rep = 0;
            ctx.repeat.repeat_cmd = None;
        } else if ctx.ps.paralyze == 0 {
            if let Some((kind, dx, dy)) = ctx.repeat.repeat_cmd.clone() {
                ctx.repeat.command_rep -= 1;
                ctx.repeat.repeat_turn = ctx.ps.turn;
                let more = command_direction(&mut ctx, &kind, 0, dx, dy);
                if !more {
                    ctx.repeat.command_rep = 0;
                    ctx.repeat.repeat_cmd = None;
                }
            }
            return;
        }
    }
    // Level-up / winner notes (xtra2.cc check_experience, files.cc winner).
    notes_tick(&mut ctx);
    // Automatizer pack sweep (squeltch_inventory; dungeon.cc calls it after
    // most item commands).
    squeltch_inventory(&mut ctx);
    // Merge similar pack stacks and sort the pack (object2.cc
    // combine_pack/reorder_pack, called by dungeon.cc after every command).
    item::combine_pack(&ctx.gd, &mut ctx.inv, &mut ctx.log);
    item::reorder_pack(&ctx.gd, &mut ctx.inv, &mut ctx.log);
    // The random-teleport corruption asks its "Teleport?" question
    // (dungeon.cc process_world_corruptions).
    if ctx.turn.corrupt_teleport {
        ctx.turn.corrupt_teleport = false;
        *ctx.modal = Modal::CorruptTeleport;
        return;
    }
    // q_rand.cc princess_death: the princess is gone; wipe her glass room
    // and raise the way out where she stood.
    if let Some((ex, ey)) = ctx.plot.rand_exit.take() {
        let princesses: Vec<Entity> = ctx
            .monsters
            .iter()
            .filter(|(_, m, _)| ctx.gd.monsters[m.def].id == 969)
            .map(|(e, _, _)| e)
            .collect();
        for e in princesses {
            ctx.commands.entity(e).despawn();
        }
        for j in ey - 1..=ey + 1 {
            for i in ex - 1..=ex + 1 {
                if Map::in_bounds(i, j) {
                    ctx.map.set_terrain(i, j, map::T_FLOOR);
                }
            }
        }
        ctx.map.set_terrain(ex, ey, map::T_STAIRS_DOWN);
    }
    // q_betwen.cc death hook: the escape staircase appears under the
    // player once the thunderlord wing is broken.
    if ctx.plot.between_escape {
        ctx.plot.between_escape = false;
        if let Ok(mut pp) = ctx.player.single_mut() {
            ctx.map.set_terrain(pp.x, pp.y, map::T_STAIRS_UP);
            ctx.log.add("You can escape now.");
        }
    }
    // q_rand.cc hero_death: the rescued swordsman asks to join (or, once
    // the companion cap is reached, teaches a skill instead).
    if let Some(def) = ctx.plot.hero_join.take() {
        let companions = ctx.monsters.iter().filter(|(_, m, _)| m.companion).count() as i32;
        let cap = 1 + ctx.ps.skill_scale(crate::skill::SK_LORE, 6);
        if companions < cap {
            *ctx.modal = Modal::RandHero(def);
        } else {
            ctx.log.add("I must go on my own way now.");
            ctx.log.add("But before I go, I can help your skills.'");
            ctx.log.add("He touches your forehead.");
            let mut rng = crate::rng::current();
            let cands = crate::skill::random_gain_candidates(&ctx.ps, &ctx.gd, &mut rng);
            if cands.is_empty() {
                ctx.ps.skill_points += 6;
                ctx.log
                    .add("The grateful adventurer teaches you his craft. (+6 skill points)");
            } else {
                ctx.ps.pending_skill_gain = Some(cands);
            }
        }
        return;
    }
    // A princess-quest reward waits for a choice.
    if let Some(offers) = ctx.plot.offers.take() {
        *ctx.modal = Modal::RandReward(offers);
        return;
    }
    let keys = ctx.keys.clone();

    // Rest ends when its goal is met (dungeon.cc:3743): `*` waits for full
    // HP/SP, `&` for anything wrong to be cured.
    if ctx.turn.resting > 0 {
        let done = match ctx.repeat.rest_mode {
            RestMode::Full => ctx.ps.hp >= ctx.ps.max_hp && ctx.ps.mana >= ctx.ps.max_mana,
            RestMode::AsNeeded => rest_as_needed_done(&ctx),
            _ => false,
        };
        if done {
            ctx.turn.resting = 0;
            ctx.repeat.rest_mode = RestMode::None;
        }
    }

    // Any keypress disturbs resting / running.
    if keys.get_just_pressed().next().is_some() {
        ctx.turn.resting = 0;
        ctx.turn.running = None;
        ctx.run.active = false;
        ctx.repeat.rest_mode = RestMode::None;
    }

    // Autopilot / scripted runs park a straight-line run in the turn state;
    // take it over with the corridor algorithm.
    if let Some((dx, dy, steps)) = ctx.turn.running.take() {
        start_run(&mut ctx, dir_from_delta(dx, dy), steps);
        return;
    }

    // An active run takes one step per frame (cmd1.cc run_step). While
    // paralyzed the run waits (the original keeps the `running` counter).
    if ctx.run.active {
        if ctx.turn.pending.is_some() {
            ctx.run.active = false;
        } else if ctx.ps.paralyze > 0 {
            ctx.log.add("You are paralyzed!");
            consume_turn(&mut ctx);
            return;
        } else {
            advance_run(&mut ctx);
            return;
        }
    }

    // While controlling a monster, almost every command is redirected to
    // the creature (dungeon.cc process_command blocks the rest, including
    // quitting and the stairs).
    if ctx.ps.control.is_some() {
        control_input(&mut ctx);
        return;
    }

    // A level departure was confirmed: run the stairs again.
    if let Some(delta) = ctx.turn.leave_retry.take() {
        use_stairs(&mut ctx, delta, true);
        return;
    }

    // Quit confirmation dialog (saves the game before leaving).
    if ctx.turn.confirm_quit {
        if keys.just_pressed(KeyCode::KeyY) {
            // NOTES: session end (files.cc:5062 add_note_type).
            ctx.notes.add_save_game(&crate::notes::timestamp_now());
            let save = build_save(&mut ctx);
            crate::save::save_player(&save);
            ctx.log.add("Game saved.");
            ctx.exit.write(AppExit::Success);
        } else if keys.just_pressed(KeyCode::KeyN) || keys.just_pressed(KeyCode::Escape) {
            ctx.turn.confirm_quit = false;
        }
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    // Paralyzed characters cannot act at all (the turn still passes).
    if ctx.ps.paralyze > 0 {
        ctx.log.add("You are paralyzed!");
        consume_turn(&mut ctx);
        return;
    }

    // Heavily stunned characters sometimes fumble their action.
    if ctx.ps.stun > 50 && crate::rng::current().gen_bool(0.10) {
        ctx.log.add("You are too stunned to do anything!");
        consume_turn(&mut ctx);
        return;
    }

    // Ctrl+Q: the quest status screen (cmd4.cc:3791 do_cmd_checkquest
    // simply shows the knowledge Quests page).
    if ctrl && !shift && keys.just_pressed(KeyCode::KeyQ) {
        *ctx.modal = Modal::Knowledge { page: 7, scroll: 0 };
        return;
    }
    // Ctrl+Shift+Q: commit suicide, as in the original 'Q' key (files.cc
    // do_cmd_suicide); the confirmation is the port's re-press guard.
    if ctrl && shift && keys.just_pressed(KeyCode::KeyQ) {
        let now = ctx.time.elapsed_secs();
        if ctx.run.confirm_suicide > 0.0 && now - ctx.run.confirm_suicide < 5.0 {
            ctx.run.confirm_suicide = 0.0;
            ctx.ps.hp = 0;
            ctx.log.add("Goodbye, brave adventurer!");
            ctx.log.add("You commit suicide.");
            ctx.next.set(AppState::Dead);
        } else {
            ctx.run.confirm_suicide = now;
            ctx.log
                .add("Do you really want to quit? Press Ctrl+Shift+Q again to commit suicide.");
        }
        return;
    }
    // Shift+Q: save-and-quit confirmation (the original Ctrl+X path; the
    // autopilot save test drives Shift+Q).
    if shift && keys.just_pressed(KeyCode::KeyQ) {
        ctx.turn.confirm_quit = true;
        return;
    }
    // Ctrl+X: save and quit (dungeon.cc:3598 KTRL('X')).
    if ctrl && keys.just_pressed(KeyCode::KeyX) {
        ctx.turn.confirm_quit = true;
        return;
    }
    // Ctrl+W: toggle wizard mode (dungeon.cc:2811 KTRL('W')); leaving
    // wizard mode needs no confirmation.
    if ctrl && !shift && keys.just_pressed(KeyCode::KeyW) {
        if ctx.ps.wizard {
            game::toggle_wizard(&mut ctx.ps, true, &mut ctx.log);
        } else {
            *ctx.modal = Modal::ConfirmWizard { debug: false };
        }
        return;
    }
    // Ctrl+A: the debug-command gate (dungeon.cc:2837 KTRL('A')); the
    // original prompt is skipped once wizard mode or the 0x0008 noscore
    // bit is set.  The debug command menu itself is not ported.
    if ctrl && !shift && keys.just_pressed(KeyCode::KeyA) {
        if ctx.ps.wizard || ctx.ps.noscore & 0x0008 != 0 {
            game::enter_debug_mode_cmd(&mut ctx.ps, true, &mut ctx.log);
        } else {
            *ctx.modal = Modal::ConfirmWizard { debug: true };
        }
        return;
    }
    // R: rest (cmd2.cc do_cmd_rest).
    if keys.just_pressed(KeyCode::KeyR) && !shift {
        start_rest(&mut ctx);
        return;
    }
    // 5 / '.': stay still for one turn; do_cmd_stay runs carry()
    // (dungeon.cc:2964/2972), so always_pickup still collects the pile.
    if keys.just_pressed(KeyCode::Digit5)
        || keys.just_pressed(KeyCode::Numpad5)
        || (keys.just_pressed(KeyCode::Period) && !shift)
    {
        let (px, py) = ctx.player.single().map(|p| (p.x, p.y)).unwrap_or((0, 0));
        let pickup = ctx.options.always_pickup;
        carry_at(&mut ctx, px, py, pickup);
        consume_turn(&mut ctx);
        return;
    }
    // '<' / '>': staircases.
    if shift && keys.just_pressed(KeyCode::Comma) {
        use_stairs(&mut ctx, -1, false);
        return;
    }
    if shift && keys.just_pressed(KeyCode::Period) {
        use_stairs(&mut ctx, 1, false);
        return;
    }
    // Item and inventory commands.
    if keys.just_pressed(KeyCode::KeyG) && !shift {
        get_items(&mut ctx);
        return;
    }
    // Shift+G: give an item to a monster (cmd2.cc do_cmd_give).
    if keys.just_pressed(KeyCode::KeyG) && shift {
        *ctx.modal = Modal::Give { idx: 0 };
        return;
    }
    if keys.just_pressed(KeyCode::KeyI) && !shift {
        *ctx.modal = Modal::Inventory;
        return;
    }
    // Shift+I: examine an item in full (cmd3.cc do_cmd_observe).
    if keys.just_pressed(KeyCode::KeyI) && shift {
        *ctx.modal = Modal::Observe;
        return;
    }
    if keys.just_pressed(KeyCode::KeyE) && !shift {
        *ctx.modal = Modal::Equipment;
        return;
    }
    if keys.just_pressed(KeyCode::KeyE) && shift {
        let has = ctx
            .inv
            .pack
            .iter()
            .any(|it| matches!(ctx.gd.objects[it.def].tval, data::TV_FOOD | data::TV_CORPSE));
        if has {
            *ctx.modal = Modal::Eat;
        } else {
            ctx.log.add("You have nothing to eat.");
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyD) && !shift {
        if ctx.inv.pack.is_empty() && ctx.inv.equip.iter().all(|s| s.is_none()) {
            ctx.log.add("You are not carrying anything.");
        } else {
            start_item_picker(&mut ctx, PickerMode::Drop);
        }
        return;
    }
    // Shift+D: destroy an item (cmd3.cc do_cmd_destroy).
    if keys.just_pressed(KeyCode::KeyD) && shift {
        let floor_here = player_pos(&ctx)
            .map(|(x, y)| {
                ctx.stacks
                    .iter()
                    .any(|(_, p, s)| p.x == x && p.y == y && !s.stack.is_empty())
            })
            .unwrap_or(false);
        if ctx.inv.pack.is_empty() && ctx.inv.equip.iter().all(|s| s.is_none()) && !floor_here {
            ctx.log.add("You have nothing to destroy.");
        } else {
            start_item_picker(&mut ctx, PickerMode::Destroy);
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyW) {
        *ctx.modal = Modal::Wield;
        return;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        *ctx.modal = Modal::TakeOff;
        return;
    }
    // q: quaff a potion (as in ToME).
    if keys.just_pressed(KeyCode::KeyQ) && !shift {
        let has = ctx.inv.pack.iter().any(|it| {
            matches!(
                ctx.gd.objects[it.def].tval,
                data::TV_POTION | data::TV_POTION2
            )
        });
        if has {
            *ctx.modal = Modal::Quaff;
        } else {
            ctx.log.add("You have no potion to quaff.");
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        if ctx.ps.blind > 0 {
            ctx.log.add("You can't see to read!");
            return;
        }
        let has = ctx.inv.pack.iter().any(|it| {
            matches!(
                ctx.gd.objects[it.def].tval,
                data::TV_SCROLL | data::TV_PARCHMENT
            )
        });
        if has {
            *ctx.modal = Modal::Read;
        } else {
            ctx.log.add("You have no scroll to read.");
        }
        return;
    }
    // Magic devices: a aim wand, z use staff, Z activate rod.
    if keys.just_pressed(KeyCode::KeyA) && !shift {
        open_use_modal(&mut ctx, data::TV_WAND, Modal::Aim, "wand");
        return;
    }
    // A: activate a worn item's special power (artifact or base kind;
    // junkarts in the pack can also be activated, cmd6.cc).
    if keys.just_pressed(KeyCode::KeyA) && shift {
        let worn_ready = ctx.inv.equip.iter().flatten().any(|it| {
            it.timeout <= 0
                && ((it.artifact != 0
                    && ctx
                        .gd
                        .artifacts
                        .iter()
                        .any(|a| a.id == it.artifact && !a.activate.is_empty()))
                    || !ctx.gd.objects[it.def].activate.is_empty())
        });
        let pack_ready = ctx
            .inv
            .pack
            .iter()
            .any(|it| ctx.gd.objects[it.def].tval == data::TV_RANDART && it.timeout <= 0);
        if worn_ready || pack_ready {
            *ctx.modal = Modal::Activate;
        } else {
            ctx.log.add("You have no charged activatable item worn.");
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) && !shift {
        open_use_modal(&mut ctx, data::TV_STAFF, Modal::Staff, "staff");
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) && shift {
        // Rod tips assemble onto rod bodies; both count (cmd6.cc).
        let has = ctx
            .inv
            .pack
            .iter()
            .any(|it| matches!(ctx.gd.objects[it.def].tval, t if t == data::TV_ROD || t == data::TV_ROD_MAIN));
        if has {
            *ctx.modal = Modal::Rod;
        } else {
            ctx.log.add("You have no rod to use.");
        }
        return;
    }
    // Shift+U: use a racial/corruption/shape power (powers.cc).
    if keys.just_pressed(KeyCode::KeyU) && shift {
        *ctx.modal = Modal::Powers { cursor: 0 };
        return;
    }
    // U: possess a corpse / leave the current form (Possessor class).
    if keys.just_pressed(KeyCode::KeyU) && !shift {
        if ctx.ps.class_name != "Possessor" {
            ctx.log.add("Only a Possessor can bind a corpse.");
            return;
        }
        if ctx.ps.possessed.is_some() {
            // do_cmd_leave_body(drop_body = true): cursed gear blocks it,
            // otherwise the body may be kept as a corpse.
            let mut rng = crate::rng::current();
            match game::leave_body(
                &mut ctx.ps,
                &ctx.inv,
                &ctx.gd,
                true,
                &mut rng,
                &mut ctx.log,
            ) {
                Ok(Some(corpse)) => {
                    let Ok(pos) = ctx.player.single().map(|p| *p) else {
                        return;
                    };
                    item::place_floor_item(
                        &mut ctx.commands,
                        &ctx.gd,
                        &ctx.tiles,
                        &mut ctx.stacks,
                        pos.x,
                        pos.y,
                        corpse,
                    );
                }
                Ok(None) => {}
                Err(()) => return,
            }
            consume_turn(&mut ctx);
            return;
        }
        // Integrating picks a corpse; `Modal::Possess` walks the pack and
        // the next frame clears the disembodied flag (see the poll in
        // player_input).
        let has_corpse = ctx
            .inv
            .pack
            .iter()
            .any(|it| ctx.gd.objects[it.def].name == "Corpse");
        if has_corpse {
            *ctx.modal = Modal::Possess;
        } else if ctx.ps.disembodied {
            ctx.log.add("You have no corpse to incarnate in.");
        } else {
            ctx.log.add("You carry no corpse to possess.");
        }
        return;
    }
    // o: open a door or chest (cmd2.cc do_cmd_open).  A single known
    // closed door is targeted automatically (count_feats); otherwise the
    // direction prompt opens.  The original 'o' key.
    if keys.just_pressed(KeyCode::KeyO) && !shift && !ctrl {
        if let Ok(pos) = ctx.player.single().map(|p| *p) {
            let (n, last) = count_feats(&ctx, pos.x, pos.y, is_closed, false);
            if n == 0 {
                ctx.log.add("You see nothing there to open.");
                return;
            }
            if n == 1 {
                let (x, y) = last.unwrap();
                let dir = coords_to_dir(pos.x, pos.y, x, y);
                command_direction(
                    &mut ctx,
                    "open_door",
                    0,
                    DDX[dir as usize],
                    DDY[dir as usize],
                );
                return;
            }
        }
        ctx.turn.pending_cmd = Some(("open_door".to_string(), 0));
        return;
    }
    // Ctrl+P: pray / stop praying (gods.cc do_cmd_pray; the original 'p' —
    // the port's plain 'p' reads a scroll).
    if ctrl && keys.just_pressed(KeyCode::KeyP) {
        if ctx.ps.god == 0 {
            ctx.log
                .add("You worship no god. (Visit the temple altar in Bree.)");
            return;
        }
        ctx.ps.praying = !ctx.ps.praying;
        if ctx.ps.praying {
            ctx.log.add(format!(
                "You begin praying to {}.",
                crate::spell::god_name(ctx.ps.god)
            ));
        } else {
            ctx.log.add("You stop praying.");
        }
        consume_turn(&mut ctx);
        return;
    }
    // C: skill tree screen.
    if keys.just_pressed(KeyCode::KeyC) && !shift {
        // Snapshot the skill state; the screen only commits on confirm
        // (skills.cc do_cmd_skill saves the values at entry).
        crate::skill::begin_skill_session(&mut ctx.ps);
        *ctx.modal = Modal::Skills { cursor: 0 };
        return;
    }
    // Shift+C: close an adjacent door (cmd2.cc do_cmd_close). A single
    // known open door is targeted automatically (count_feats).
    if keys.just_pressed(KeyCode::KeyC) && shift {
        if let Ok(pos) = ctx.player.single().map(|p| *p) {
            let (n, last) = count_feats(&ctx, pos.x, pos.y, is_known_open_door, false);
            if n == 0 {
                ctx.log.add("You see nothing there to close.");
                return;
            }
            if n == 1 {
                let (x, y) = last.unwrap();
                let dir = coords_to_dir(pos.x, pos.y, x, y);
                command_direction(
                    &mut ctx,
                    "close_door",
                    0,
                    DDX[dir as usize],
                    DDY[dir as usize],
                );
                return;
            }
        }
        ctx.turn.pending_cmd = Some(("close_door".to_string(), 0));
        return;
    }
    // Ctrl+B: bash a door, altar or fountain (cmd2.cc do_cmd_bash).
    if ctrl && keys.just_pressed(KeyCode::KeyB) {
        ctx.turn.pending_cmd = Some(("bash".to_string(), 0));
        return;
    }
    // Ctrl+O: sacrifice at an altar (cmd2.cc do_cmd_sacrifice); the
    // port's original 'O' is the pet menu, so it moved to Ctrl+O.
    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        do_cmd_sacrifice(&mut ctx);
        return;
    }
    // Ctrl+T: show the game time (cmd4.cc do_cmd_time).
    if ctrl && keys.just_pressed(KeyCode::KeyT) {
        show_time(&mut ctx);
        return;
    }
    // B: abilities screen.
    if keys.just_pressed(KeyCode::KeyB) && !shift {
        *ctx.modal = Modal::Abilities { cursor: 0 };
        return;
    }
    // Shift+B: drink from / fill bottles at a fountain (cmd6.cc).
    if keys.just_pressed(KeyCode::KeyB) && shift {
        fountain_command(&mut ctx);
        return;
    }
    // X: use an activatable skill/ability action.
    if keys.just_pressed(KeyCode::KeyX) && !shift {
        *ctx.modal = Modal::Actions { cursor: 0 };
        return;
    }
    // Shift+X: cut up a corpse into meat (cmd6.cc do_cmd_cut_corpse).
    if keys.just_pressed(KeyCode::KeyX) && shift {
        let has = ctx
            .inv
            .pack
            .iter()
            .any(|it| ctx.gd.objects[it.def].tval == data::TV_CORPSE);
        if has {
            *ctx.modal = Modal::CutCorpse;
        } else {
            ctx.log.add("You have no corpses to cut up.");
        }
        return;
    }
    // f: fire a missile from the wielded bow (or throw a boomerang).
    if keys.just_pressed(KeyCode::KeyF) && !shift {
        fire_command(&mut ctx);
        return;
    }
    // F (shift+f): refuel the wielded light (cmd3.cc do_cmd_refill).
    if keys.just_pressed(KeyCode::KeyF) && shift {
        refill_light(&mut ctx);
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) && !shift {
        if ctx.inv.totals_for(&ctx.gd, &ctx.ps).no_magic {
            ctx.log.add("An antimagic field prevents spellcasting!");
        } else if crate::spell::known_spells(&ctx.gd, &ctx.ps).is_empty() {
            ctx.log.add("You cannot cast spells.");
        } else {
            *ctx.modal = Modal::Cast;
        }
        return;
    }
    // Shift+M: browse the spellbooks (cmd5.cc do_cmd_browse).
    if keys.just_pressed(KeyCode::KeyM) && shift {
        *ctx.modal = Modal::Browse { cursor: 0 };
        return;
    }
    // +: alter the adjacent grid (open/tunnel/attack; cmd2.cc
    // do_cmd_alter).
    if (keys.just_pressed(KeyCode::Equal) && shift) || keys.just_pressed(KeyCode::NumpadAdd) {
        ctx.turn.pending_cmd = Some(("alter".to_string(), 0));
        return;
    }
    // v: throw an item (cmd2.cc do_cmd_throw; not in the overworld map).
    if keys.just_pressed(KeyCode::KeyV) && !shift {
        if ctx.ps.wild_mode {
            return;
        }
        if ctx.inv.pack.is_empty() {
            ctx.log.add("You have nothing to throw.");
            return;
        }
        *ctx.modal = Modal::Throw { mult: 1 };
        return;
    }
    // Shift+V: jam a door with an iron spike (cmd2.cc do_cmd_spike).
    if keys.just_pressed(KeyCode::KeyV) && shift {
        if !ctx
            .inv
            .pack
            .iter()
            .any(|it| ctx.gd.objects[it.def].tval == data::TV_SPIKE)
        {
            ctx.log.add("You have no spikes!");
            return;
        }
        ctx.turn.pending_cmd = Some(("spike".to_string(), 0));
        return;
    }
    // Shift+K: knowledge menu.
    if keys.just_pressed(KeyCode::KeyK) && shift {
        *ctx.modal = Modal::Knowledge { page: 0, scroll: 0 };
        return;
    }
    // Shift+N: inscribe a pack item.
    if keys.just_pressed(KeyCode::KeyN) && shift {
        *ctx.modal = Modal::Inscribe { remove: false };
        return;
    }
    // Shift+P: message history.
    if keys.just_pressed(KeyCode::KeyP) && shift {
        *ctx.modal = Modal::Messages { scroll: 0 };
        return;
    }
    // Shift+O: command the companions (cmd1.cc do_cmd_pet).
    if keys.just_pressed(KeyCode::KeyO) && shift {
        start_pet_menu(&mut ctx);
        return;
    }
    // ':' (cmd4.cc do_cmd_note): append a note to the character notes file.
    if shift && keys.just_pressed(KeyCode::Semicolon) {
        ctx.notes.input = Some(String::new());
        ctx.log.add("Note: (type, Enter to save, Esc to cancel)");
        return;
    }
    // Shift+R: cure meat with a potion (cmd6.cc do_cmd_cure_meat).
    if keys.just_pressed(KeyCode::KeyR) && shift {
        let has = ctx.inv.pack.iter().any(|it| {
            ctx.gd.objects[it.def].tval == data::TV_CORPSE && ctx.gd.objects[it.def].sval == 5
        });
        if has {
            *ctx.modal = Modal::CureMeat { meat: usize::MAX };
        } else {
            ctx.log.add("You have no meat to cure.");
        }
        return;
    }
    // Shift+T: combat tactics and movement mode.
    if keys.just_pressed(KeyCode::KeyT) && shift {
        *ctx.modal = Modal::Tactic { cursor: 0 };
        return;
    }
    // }: sense the grid's mana, then engrave a learned rune
    // (cmd3.cc do_cmd_sense_grid_mana + cmd1.cc do_cmd_engrave; the
    // original 'x', taken here by the skill-actions menu).
    if keys.just_pressed(KeyCode::BracketRight) && shift {
        sense_grid_mana(&mut ctx);
        if (1..8).any(|i| game::rune_known(&ctx.ps, i)) {
            *ctx.modal = Modal::Engrave;
        }
        return;
    }
    // /: query a map symbol (cmd3.cc do_cmd_query_symbol).
    if keys.just_pressed(KeyCode::Slash) && !shift {
        *ctx.modal = Modal::QuerySymbol { result: None };
        return;
    }
    // *: set a persistent target (cmd3.cc do_cmd_target).
    if keys.just_pressed(KeyCode::Digit8) && shift {
        *ctx.modal = Modal::TargetLock { cursor: 0 };
        return;
    }
    // Shift+S: look around the map (cmd3.cc do_cmd_look).
    if keys.just_pressed(KeyCode::KeyS) && shift {
        let pos = ctx.player.single().map(|p| *p).ok();
        let (x, y) = pos.map(|p| (p.x, p.y)).unwrap_or((0, 0));
        *ctx.modal = Modal::Look { x, y };
        return;
    }
    // s: search for secret doors and traps.
    if keys.just_pressed(KeyCode::KeyS) && !shift {
        search(&mut ctx);
        return;
    }
    // A pending directional command (alter/close/spike/give) consumes the
    // next direction key or is cancelled with Esc.
    if let Some((kind, arg)) = ctx.turn.pending_cmd.clone() {
        if keys.just_pressed(KeyCode::Escape) {
            ctx.turn.pending_cmd = None;
            ctx.log.add("Cancelled.");
            return;
        }
        if let Some((dx, dy)) = direction_key(&keys) {
            ctx.turn.pending_cmd = None;
            let more = command_direction(&mut ctx, &kind, arg, dx, dy);
            // allow_repeat_command (cmd2.cc:768): the typed count becomes
            // that many repetitions; a command that cannot continue
            // cancels them (disturb).
            let count = ctx.repeat.command_arg;
            ctx.repeat.command_arg = 0;
            if count > 1 && more {
                ctx.repeat.command_rep = count - 1;
                ctx.repeat.repeat_cmd = Some((kind.clone(), dx, dy));
                ctx.repeat.repeat_turn = ctx.ps.turn;
            } else {
                ctx.repeat.command_rep = 0;
                ctx.repeat.repeat_cmd = None;
            }
        }
        return;
    }

    // Directional commands. Holding a direction key repeats the step
    // (initial delay 250 ms, then one step per 80 ms); running and
    // tunnelling only trigger on the initial press.
    let held = direction_held(&keys);
    let now = ctx.time.elapsed_secs();
    let mut step: Option<(i32, i32)> = direction_key(&keys);
    if step.is_some() {
        ctx.repeat.dir = held;
        ctx.repeat.next = now + 0.25;
    } else if !shift && !ctrl {
        if let Some(hd) = held {
            if ctx.repeat.dir == Some(hd) && now >= ctx.repeat.next {
                step = Some(hd);
                ctx.repeat.next = now + 0.08;
            }
        }
    }
    if held.is_none() {
        ctx.repeat.dir = None;
    }
    if let Some((dx, dy)) = step {
        // Immovable characters (DeathMold race, Siegecraft crossbow) do not
        // walk: they phase through the world (cmd2.cc do_cmd_unwalk, called
        // from do_cmd_walk). The special menu stays on the power key.
        if game::player_immovable(&ctx.gd, &ctx.ps, &ctx.inv) {
            unwalk(&mut ctx, dx, dy);
            return;
        }
        // Confusion scrambles most intended directions.
        let (dx, dy) = if ctx.ps.confuse > 0 && crate::rng::current().gen_bool(0.75) {
            let mut rng = crate::rng::current();
            let d = loop {
                let d = (rng.gen_range(-1..=1), rng.gen_range(-1..=1));
                if d != (0, 0) {
                    break d;
                }
            };
            ctx.log.add("You stumble in confusion!");
            d
        } else {
            (dx, dy)
        };
        if ctrl {
            tunnel(&mut ctx, dx, dy);
        } else if shift {
            start_run(&mut ctx, dir_from_delta(dx, dy), 1000);
        } else {
            // '-' walks without picking up (cmd2.cc do_cmd_walk(!pickup),
            // bound to dungeon.cc:2943); the default carries always_pickup.
            let pickup = if keys.pressed(KeyCode::Minus) {
                !ctx.options.always_pickup
            } else {
                ctx.options.always_pickup
            };
            step_player_pickup(&mut ctx, dx, dy, true, pickup);
            // allow_repeat_command (cmd2.cc:768/1822): a numeric prefix
            // typed with the count prompt repeats the walk.
            let count = ctx.repeat.command_arg;
            ctx.repeat.command_arg = 0;
            if count > 1 {
                ctx.repeat.command_rep = count - 1;
                let kind = if pickup { "walk" } else { "walk_nopick" };
                ctx.repeat.repeat_cmd = Some((kind.to_string(), dx, dy));
                ctx.repeat.repeat_turn = ctx.ps.turn;
            }
        }
    }
}

/// f: begin firing a missile (asks for a direction via the Target modal).
/// Without a bow, throws a boomerang instead (original do_cmd_fire).
fn fire_command(ctx: &mut InputCtx) {
    match &ctx.inv.equip[data::SLOT_BOW] {
        Some(bow) if ctx.gd.objects[bow.def].tval == data::TV_BOOMERANG => {}
        Some(bow) => {
            let Some((_mult, ammo_tval)) = data::bow_info(ctx.gd.objects[bow.def].sval) else {
                ctx.log.add("You cannot fire with that.");
                return;
            };
            // cmd2.cc:2404 takes the quiver first, then a pack stack or
            // the floor pile (USE_INVEN | USE_FLOOR).
            let (px, py) = player_pos(&ctx).unwrap_or((0, 0));
            let has_ammo = ctx.inv.pack.iter().any(|it| ctx.gd.objects[it.def].tval == ammo_tval)
                || ctx.inv.equip[data::SLOT_QUIVER]
                    .as_ref()
                    .is_some_and(|q| ctx.gd.objects[q.def].tval == ammo_tval && q.count > 0)
                || ctx.stacks.iter().any(|(_, p, s)| {
                    p.x == px
                        && p.y == py
                        && s.stack
                            .iter()
                            .any(|it| ctx.gd.objects[it.def].tval == ammo_tval)
                });
            if !has_ammo {
                ctx.log.add("You have no matching ammunition!");
                return;
            }
        }
        None => {
            let has_boomerang = ctx
                .inv
                .pack
                .iter()
                .any(|it| ctx.gd.objects[it.def].tval == data::TV_BOOMERANG);
            if !has_boomerang {
                ctx.log.add("You have no bow wielded.");
                return;
            }
        }
    }
    *ctx.modal = Modal::Target(crate::modal::PendingTarget {
        name: "Fire".to_string(),
        kind: "fire".to_string(),
        arg: String::new(),
        device: None,
    });
}

fn open_use_modal(ctx: &mut InputCtx, tval: i32, modal: Modal, what: &str) {
    let has = ctx
        .inv
        .pack
        .iter()
        .any(|it| ctx.gd.objects[it.def].tval == tval);
    if has {
        *ctx.modal = modal;
    } else {
        ctx.log.add(format!("You have no {} to use.", what));
    }
}

/// do_cmd_refill (cmd3.cc:798): refuel the wielded light; torches are
/// combined, lamps are filled with oil (cmd3.cc:861/780).
fn refill_light(ctx: &mut InputCtx) {
    let Some(lite) = ctx.inv.equip[data::SLOT_LITE].clone() else {
        ctx.log.add("You are not wielding a light.");
        return;
    };
    let o = &ctx.gd.objects[lite.def];
    let fueled = o.flags.iter().any(|f| f == "FUEL_LITE");
    if !fueled {
        ctx.log.add("Your light cannot be refilled.");
        return;
    }
    match o.sval {
        0 => refill_torch(ctx, lite),
        1 => refill_lamp(ctx, lite),
        _ => ctx.log.add("Your light cannot be refilled."),
    }
}

/// do_cmd_refill_torch (cmd3.cc:861): combine torches (timeout+timeout+5,
/// FUEL_TORCH cap).
fn refill_torch(ctx: &mut InputCtx, mut lite: item::Item) {
    let Some(i) = ctx.inv.pack.iter().position(|it| {
        ctx.gd.objects[it.def].tval == data::TV_LITE && ctx.gd.objects[it.def].sval == 0
    }) else {
        ctx.log.add("You have no extra torches.");
        return;
    };
    lite.fuel += ctx.inv.pack[i].fuel + 5;
    ctx.log.add("You combine the torches.");
    if lite.fuel >= 5000 {
        lite.fuel = 5000;
        ctx.log.add("Your torch is fully fueled.");
    } else {
        ctx.log.add("Your torch glows more brightly.");
    }
    ctx.inv.pack[i].count -= 1;
    if ctx.inv.pack[i].count == 0 {
        ctx.inv.pack.remove(i);
    }
    ctx.inv.equip[data::SLOT_LITE] = Some(lite);
    ctx.turn.fov_dirty = true;
    consume_turn(ctx);
}

/// do_cmd_refill_lamp (cmd3.cc:780): fill a lamp from a flask of oil (or
/// another lantern), FUEL_LAMP cap.
fn refill_lamp(ctx: &mut InputCtx, mut lite: item::Item) {
    let source = ctx.inv.pack.iter().position(|it| {
        ctx.gd.objects[it.def].tval == data::TV_FLASK
            || (ctx.gd.objects[it.def].tval == data::TV_LITE && ctx.gd.objects[it.def].sval == 1)
    });
    let Some(i) = source else {
        ctx.log.add("You have no flasks of oil.");
        return;
    };
    let def = ctx.inv.pack[i].def;
    if ctx.gd.objects[def].tval == data::TV_FLASK {
        lite.fuel += ctx.gd.objects[def].pval;
    } else {
        lite.fuel += ctx.inv.pack[i].fuel;
    }
    ctx.log.add("You fuel your lamp.");
    if lite.fuel >= 15000 {
        lite.fuel = 15000;
        ctx.log.add("Your lamp is full.");
    }
    ctx.inv.pack[i].count -= 1;
    if ctx.inv.pack[i].count == 0 {
        ctx.inv.pack.remove(i);
    }
    ctx.inv.equip[data::SLOT_LITE] = Some(lite);
    ctx.turn.fov_dirty = true;
    consume_turn(ctx);
}

/// Shift+B: drink from a fountain or fill an empty bottle (cmd6.cc
/// do_cmd_drink_fountain / do_cmd_fill_bottle). The fountain's potion kind
/// and remaining draughts live in Map::fountains.
fn fountain_command(ctx: &mut InputCtx) {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return;
    };
    let idx = Map::idx(pos.x, pos.y);
    let t = ctx.map.terrain_at(pos.x, pos.y);
    if t == 15 {
        ctx.log.add("The fountain is dried out.");
        return;
    }
    if t != 2 {
        ctx.log.add("You see no fountain here.");
        return;
    }
    let mut rng = crate::rng::current();
    // Known fountains keep their kind; unknown ones roll a potion kind
    // eligible at this depth (generate.cc place_fountain).
    let entry = ctx.map.fountains.entry(idx).or_insert_with(|| {
        let depth = ctx.ps.depth.max(1);
        // TV_POTION2 flavours are encoded as sval + SV_POTION_LAST
        // (generate.cc place_fountain).
        let potions: Vec<i32> = ctx
            .gd
            .objects
            .iter()
            .filter(|o| {
                (o.tval == data::TV_POTION || o.tval == data::TV_POTION2)
                    && o.depth <= depth
                    && o.flags.iter().any(|f| f == "FOUNTAIN")
            })
            .map(|o| {
                if o.tval == data::TV_POTION2 {
                    o.sval + SV_POTION_LAST
                } else {
                    o.sval
                }
            })
            .collect();
        let sval = if potions.is_empty() {
            0
        } else {
            potions[rng.gen_range(0..potions.len())]
        };
        (sval, (0..3).map(|_| rng.gen_range(1..=4)).sum())
    });
    let (sval, draughts) = *entry;
    if draughts <= 0 {
        ctx.map.set_terrain(pos.x, pos.y, 15);
        ctx.log.add("The fountain is dried out.");
        return;
    }
    // Decode the fountain's potion kind (cmd6.cc fountain_quaff).
    let (tval, psval) = if sval <= SV_POTION_LAST {
        (data::TV_POTION, sval)
    } else {
        (data::TV_POTION2, sval - SV_POTION_LAST)
    };
    // With a bottle in the pack, fill it; otherwise drink a draught.
    let bottle = ctx
        .inv
        .pack
        .iter()
        .position(|it| ctx.gd.objects[it.def].tval == data::TV_BOTTLE);
    if let Some(bi) = bottle {
        let Some(def) = ctx.gd.object_by_tval_sval(tval, psval) else {
            ctx.log.add("The fountain's water is strange and useless.");
            return;
        };
        let mut potion = item::Item::base(&ctx.gd, def);
        potion.identified = ctx.inv.known.contains(&def);
        ctx.inv.pack[bi].count -= 1;
        if ctx.inv.pack[bi].count == 0 {
            ctx.inv.pack.remove(bi);
        }
        ctx.inv.add_with(&ctx.gd, potion);
        ctx.map.fountains.insert(idx, (sval, draughts - 1));
        if draughts - 1 <= 0 {
            ctx.map.set_terrain(pos.x, pos.y, 15);
        }
        ctx.log.add("You fill a bottle from the fountain.");
        consume_turn(ctx);
        return;
    }
    let Some(def) = ctx.gd.object_by_tval_sval(tval, psval) else {
        ctx.log.add("The water tastes strange but does nothing.");
        return;
    };
    let occupied: std::collections::HashSet<(i32, i32)> =
        ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
    let def_name = ctx.gd.objects[def].name.clone();
    let mut p = pos;
    item::use_object(
        &ctx.gd,
        def,
        0,
        &mut ctx.ps,
        &mut ctx.inv,
        &mut ctx.map,
        &mut p,
        &occupied,
        &mut ctx.log,
        &mut ctx.turn,
        &mut ctx.next,
        &mut rng,
    );
    if (p.x, p.y) != (pos.x, pos.y) {
        if let Ok(mut gp) = ctx.player.single_mut() {
            gp.x = p.x;
            gp.y = p.y;
        }
    }
    ctx.map.fountains.insert(idx, (sval, draughts - 1));
    if draughts - 1 <= 0 {
        ctx.map.set_terrain(pos.x, pos.y, 15);
    }
    ctx.log
        .add(format!("You drink from the fountain of {}.", def_name));
    consume_turn(ctx);
}

/// Movement check: the Tree walking ability lets the player pass dense
/// forest (AB_TREE_WALK, dungeon.cc:1255).
fn player_can_walk(
    map: &Map,
    gd: &crate::data::GameData,
    ps: &game::PlayerState,
    inv: &Inventory,
    x: i32,
    y: i32,
) -> bool {
    player_can_walk_t(map, gd, ps, inv, x, y, map.terrain_at(x, y))
}

/// player_can_enter with an explicit feature: wilderness borders test the
/// *mimicked* neighbour terrain (cmd1.cc move_player_aux uses
/// c_ptr->mimic), everything else tests the real terrain.
#[allow(clippy::too_many_arguments)]
fn player_can_walk_t(
    map: &Map,
    gd: &crate::data::GameData,
    ps: &game::PlayerState,
    inv: &Inventory,
    x: i32,
    y: i32,
    t: u16,
) -> bool {
    let tot = inv.totals_for(gd, ps);
    // In the wilderness an overloaded swimmer must not step into deep
    // water (cmd1.cc:2456); in dungeons entering is allowed and the
    // drowning damage is applied on the world turn (dungeon.cc:1226).
    if t == map::T_DEEP_WATER && ps.wild_mode {
        let ffall = tot.feather || tot.fly || ps.tim_fly > 0 || ps.tim_ffall > 0;
        if !ffall && game::calc_total_weight(gd, inv) >= game::weight_limit(ps) / 2 {
            return false;
        }
        return true;
    }
    // The wilderness also forbids entering lava without protection
    // (cmd1.cc:2465-2475).
    if ps.wild_mode && matches!(t, map::T_LAVA | map::T_SHAL_LAVA) {
        let ffall = tot.feather || tot.fly || ps.tim_fly > 0 || ps.tim_ffall > 0;
        if !(tot.resists.contains("FIRE")
            || tot.immunities.contains("FIRE")
            || ps.oppose_fire > 0
            || ffall)
        {
            return false;
        }
    }
    // Wall mimicry (cmd1.cc player_can_enter): only walls are passable,
    // open floor is not, unless flying or climbing.
    let only_wall = ps.mimic_extra_turns > 0 && (ps.mimic_extra & crate::mimic::CLASS_WALL) != 0;
    if only_wall && !tot.fly && !tot.climb {
        let def = gd.terrain(t);
        return !map.walkable(gd, x, y) && def.tunnelable && !def.permanent;
    }
    if map.walkable(gd, x, y) {
        return true;
    }
    let def = gd.terrain(t);
    // Climbing, flight/levitation and pass-wall over flagged terrain
    // (cmd1.cc player_can_enter).
    let flying = tot.fly || ps.tim_fly > 0;
    let levitating = tot.feather || ps.tim_ffall > 0;
    if tot.climb && def.can_climb {
        return true;
    }
    if flying && (def.can_fly || def.can_levitate) {
        return true;
    }
    if levitating && def.can_levitate {
        return true;
    }
    let semi_wraith = game::player_has_flag(gd, ps, "SEMI_WRAITH");
    let pass_wall = tot.wraith || semi_wraith;
    if (pass_wall || only_wall) && def.can_pass {
        return true;
    }
    // Semi-wraiths (Spectre) seep through non-permanent rock, at a cost
    // (cmd1.cc player_can_enter pass_wall).
    if semi_wraith && def.tunnelable && !def.permanent {
        return true;
    }
    // Trees: flight, wraithform, Tree walking, an Ent shape or Yavanna's
    // favour (cmd1.cc player_can_enter).
    if def.name == "tree" {
        return flying
            || pass_wall
            || ps.has_ability(crate::skill::AB_TREE_WALKING)
            || ps.mimic_form == Some(crate::mimic::MIMIC_ENT)
            || (ps.god == 5 && ps.grace >= 9000);
    }
    // Webs stop everyone but spiders (cmd1.cc player_can_enter): the
    // possessed body's race flags or the Spider mimic shape gets through
    // (wraiths were already handled by the CAN_PASS branch above).
    if def.web {
        let spider_body = ps
            .possessed
            .and_then(|(pdef, _, _)| gd.monsters.get(pdef))
            .is_some_and(|m| m.has("SPIDER"));
        return spider_body || ps.mimic_form == Some(crate::mimic::MIMIC_SPIDER);
    }
    false
}

/// files.cc:3928 `autosave_checkpoint`: when `autosave_l` is on, save
/// before entering a new level.  The save itself runs next input frame.
fn autosave_request(opts: &crate::options::Options, turn: &mut game::TurnState) {
    if opts.autosave_l {
        turn.autosave_due = true;
    }
}

fn consume_turn(ctx: &mut InputCtx) {
    ctx.turn.world_turn = true;
    ctx.turn.fov_dirty = true;
    // dungeon.cc:1075 timed autosave (the world turn increments after this).
    if crate::files::timed_autosave_due(
        ctx.ps.turn + 1,
        ctx.options.autosave_freq,
        ctx.options.autosave_t,
    ) {
        ctx.turn.autosave_due = true;
    }
}

/// do_cmd_rest (cmd2.cc:2119): guard, then ask for a duration.
fn start_rest(ctx: &mut InputCtx) {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return;
    };
    // Can't rest on a Void Jumpgate -- too dangerous.
    if ctx.map.terrain_at(pos.x, pos.y) == map::T_BETWEEN {
        let name = ctx
            .gd
            .terrain(map::T_BETWEEN)
            .name
            .clone();
        ctx.log
            .add(format!("Resting on a {} is too dangerous!", name));
        return;
    }
    // Can't rest while undead, it would mean dying.
    if ctx.ps.undead_form.is_some() {
        ctx.log.add("Resting is impossible while undead!");
        return;
    }
    ctx.repeat.rest_prompt = Some(String::new());
}

/// Text-mode rest prompt ('R'): digits, `*` (full) or `&` (as needed).
fn rest_prompt_input(ctx: &mut InputCtx) {
    let keys = ctx.keys.clone();
    let text = ctx.repeat.rest_prompt.clone().unwrap_or_default();
    if keys.just_pressed(KeyCode::Escape) {
        ctx.repeat.rest_prompt = None;
        ctx.log.add("Cancelled.");
        return;
    }
    if keys.just_pressed(KeyCode::Backspace) {
        let mut t = text;
        t.pop();
        ctx.repeat.rest_prompt = Some(t);
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        let t = if text.is_empty() { "&".to_string() } else { text };
        ctx.repeat.rest_prompt = None;
        if t.starts_with('&') {
            ctx.turn.resting = 9999;
            ctx.repeat.rest_mode = RestMode::AsNeeded;
        } else if t.starts_with('*') {
            ctx.turn.resting = 9999;
            ctx.repeat.rest_mode = RestMode::Full;
        } else {
            let n = t.parse::<i32>().unwrap_or(0).clamp(1, 9999);
            ctx.turn.resting = n;
            ctx.repeat.rest_mode = RestMode::Turns;
        }
        return;
    }
    if let Some(c) = pressed_char_in(&keys) {
        if c.is_ascii_digit() || c == '*' || c == '&' {
            let mut t = text;
            if t.len() < 4 {
                t.push(c);
                ctx.repeat.rest_prompt = Some(t);
            }
        }
    }
}

/// A printable character from the raw key input (shift included).
fn pressed_char_in(keys: &ButtonInput<KeyCode>) -> Option<char> {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for k in keys.get_just_pressed() {
        match k {
            KeyCode::Digit0 | KeyCode::Numpad0 => return Some(if shift { ')' } else { '0' }),
            KeyCode::Digit1 | KeyCode::Numpad1 => return Some(if shift { '!' } else { '1' }),
            KeyCode::Digit2 | KeyCode::Numpad2 => return Some(if shift { '@' } else { '2' }),
            KeyCode::Digit3 | KeyCode::Numpad3 => return Some(if shift { '#' } else { '3' }),
            KeyCode::Digit4 | KeyCode::Numpad4 => return Some(if shift { '$' } else { '4' }),
            KeyCode::Digit5 | KeyCode::Numpad5 => return Some(if shift { '%' } else { '5' }),
            KeyCode::Digit6 | KeyCode::Numpad6 => return Some(if shift { '^' } else { '6' }),
            KeyCode::Digit7 | KeyCode::Numpad7 => return Some(if shift { '&' } else { '7' }),
            KeyCode::Digit8 | KeyCode::Numpad8 => return Some(if shift { '*' } else { '8' }),
            KeyCode::Digit9 | KeyCode::Numpad9 => return Some(if shift { '(' } else { '9' }),
            KeyCode::NumpadMultiply => return Some('*'),
            _ => {}
        }
    }
    None
}

/// do_cmd_time (cmd4.cc:3837): the day count, clock and the time flavour
/// line (timenorm/timefun, embedded from lib/file).
fn show_time(ctx: &mut InputCtx) {
    const DAY: u64 = 11520;
    const HOUR: u64 = DAY / 24;
    const MINUTE: u64 = HOUR / 60;
    const DAY_START: u64 = HOUR * 6;
    let turns = ctx.ps.turn + 10 * DAY_START;
    let minute = (turns / 10 / MINUTE) % 60;
    let hour = (turns / 10 / HOUR) % 24;
    let day = (turns / 10 / DAY) % 365 + 1;
    ctx.log.add(format!(
        "This is the {} day of your adventure.",
        ordinal(day)
    ));
    ctx.log.add(format!(
        "The time is {}:{:02} {}.",
        if hour % 12 == 0 { 12 } else { hour % 12 },
        minute,
        if hour < 12 { "AM" } else { "PM" }
    ));
    let mut rng = crate::rng::current();
    let fun = rng.gen_range(0..10) == 0 || ctx.ps.image > 0;
    let text = if fun {
        include_str!("../assets/data/timefun.txt")
    } else {
        include_str!("../assets/data/timenorm.txt")
    };
    let full = (hour * 100 + minute) as i32;
    let mut start = 9999i32;
    let mut end = -9999i32;
    let mut num = 0;
    let mut desc = "It is a strange time.".to_string();
    for line in text.lines() {
        let b = line.as_bytes();
        if b.is_empty() || b[0] == b'#' || b.len() < 2 || b[1] != b':' {
            continue;
        }
        match b[0] {
            b'S' => {
                start = line[2..].trim().parse().unwrap_or(0);
                end = start + 59;
            }
            b'E' => end = line[2..].trim().parse().unwrap_or(end),
            b'D' => {
                if start > full || full > end {
                    continue;
                }
                num += 1;
                if rng.gen_range(0..num) == 0 {
                    desc = line[2..].to_string();
                }
            }
            _ => {}
        }
    }
    ctx.log.add(desc);
}

/// get_day (util.cc:3391): 1st/2nd/3rd/th.
fn ordinal(n: u64) -> String {
    let suffix = if (n / 10) == 1 {
        "th"
    } else {
        match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

/// The `&` rest's stop test (dungeon.cc:3756): false while anything is
/// still wrong with the player.
fn rest_as_needed_done(ctx: &InputCtx) -> bool {
    let ps = &ctx.ps;
    if ps.hp != ps.max_hp || ps.mana != ps.max_mana {
        return false;
    }
    if let Some(sym) = &ctx.inv.equip[data::SLOT_SYMBIOTE] {
        if sym.pval2 < sym.pval3 {
            return false;
        }
    }
    !(ps.blind > 0
        || ps.confuse > 0
        || ps.poison > 0
        || ps.fear > 0
        || ps.stun > 0
        || ps.cut > 0
        || ps.slow > 0
        || ps.paralyze > 0
        || ps.image > 0
        || ps.word_recall > 0
        || ctx.repeat.immov_cntr != 0)
}

fn player_pos(ctx: &InputCtx) -> Option<(i32, i32)> {
    ctx.player.single().map(|p| (p.x, p.y)).ok()
}

/// A minimal key -> character mapping for the text prompts (notes,
/// quantities).  Mirrors the modal.rs text input for the ASCII range.
fn key_char(k: KeyCode, shift: bool) -> Option<char> {
    Some(match k {
        KeyCode::KeyA => {
            if shift {
                'A'
            } else {
                'a'
            }
        }
        KeyCode::KeyB => {
            if shift {
                'B'
            } else {
                'b'
            }
        }
        KeyCode::KeyC => {
            if shift {
                'C'
            } else {
                'c'
            }
        }
        KeyCode::KeyD => {
            if shift {
                'D'
            } else {
                'd'
            }
        }
        KeyCode::KeyE => {
            if shift {
                'E'
            } else {
                'e'
            }
        }
        KeyCode::KeyF => {
            if shift {
                'F'
            } else {
                'f'
            }
        }
        KeyCode::KeyG => {
            if shift {
                'G'
            } else {
                'g'
            }
        }
        KeyCode::KeyH => {
            if shift {
                'H'
            } else {
                'h'
            }
        }
        KeyCode::KeyI => {
            if shift {
                'I'
            } else {
                'i'
            }
        }
        KeyCode::KeyJ => {
            if shift {
                'J'
            } else {
                'j'
            }
        }
        KeyCode::KeyK => {
            if shift {
                'K'
            } else {
                'k'
            }
        }
        KeyCode::KeyL => {
            if shift {
                'L'
            } else {
                'l'
            }
        }
        KeyCode::KeyM => {
            if shift {
                'M'
            } else {
                'm'
            }
        }
        KeyCode::KeyN => {
            if shift {
                'N'
            } else {
                'n'
            }
        }
        KeyCode::KeyO => {
            if shift {
                'O'
            } else {
                'o'
            }
        }
        KeyCode::KeyP => {
            if shift {
                'P'
            } else {
                'p'
            }
        }
        KeyCode::KeyQ => {
            if shift {
                'Q'
            } else {
                'q'
            }
        }
        KeyCode::KeyR => {
            if shift {
                'R'
            } else {
                'r'
            }
        }
        KeyCode::KeyS => {
            if shift {
                'S'
            } else {
                's'
            }
        }
        KeyCode::KeyT => {
            if shift {
                'T'
            } else {
                't'
            }
        }
        KeyCode::KeyU => {
            if shift {
                'U'
            } else {
                'u'
            }
        }
        KeyCode::KeyV => {
            if shift {
                'V'
            } else {
                'v'
            }
        }
        KeyCode::KeyW => {
            if shift {
                'W'
            } else {
                'w'
            }
        }
        KeyCode::KeyX => {
            if shift {
                'X'
            } else {
                'x'
            }
        }
        KeyCode::KeyY => {
            if shift {
                'Y'
            } else {
                'y'
            }
        }
        KeyCode::KeyZ => {
            if shift {
                'Z'
            } else {
                'z'
            }
        }
        KeyCode::Digit0 | KeyCode::Numpad0 => '0',
        KeyCode::Digit1 | KeyCode::Numpad1 => '1',
        KeyCode::Digit2 | KeyCode::Numpad2 => '2',
        KeyCode::Digit3 | KeyCode::Numpad3 => '3',
        KeyCode::Digit4 | KeyCode::Numpad4 => '4',
        KeyCode::Digit5 | KeyCode::Numpad5 => '5',
        KeyCode::Digit6 | KeyCode::Numpad6 => '6',
        KeyCode::Digit7 | KeyCode::Numpad7 => '7',
        KeyCode::Digit8 | KeyCode::Numpad8 => '8',
        KeyCode::Digit9 | KeyCode::Numpad9 => '9',
        KeyCode::Space => ' ',
        KeyCode::Minus => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        KeyCode::Equal => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        KeyCode::Period => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        KeyCode::Comma => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        KeyCode::Slash => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        KeyCode::Backslash => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        KeyCode::Semicolon => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        KeyCode::Quote => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        KeyCode::BracketLeft => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        KeyCode::BracketRight => {
            if shift {
                '}'
            } else {
                ']'
            }
        }
        _ => return None,
    })
}

// --- Notes (src/notes.cc) ---

/// Poll the level-up and winner hooks once per input frame.  Level-ups can
/// happen in game.rs or modal.rs (which this task does not own), so a
/// level tracker is the safe central hook.
fn notes_tick(ctx: &mut InputCtx) {
    ctx.notes
        .check_level_up(ctx.ps.level, ctx.ps.turn, ctx.ps.depth);
    if (ctx.plot.won || ctx.plot.ultra_won) && !ctx.notes.winner_noted {
        ctx.notes.winner_noted = true;
        let player = ctx.ps.name.clone();
        let ts = crate::notes::timestamp_now();
        ctx.notes.add_winner(&player, &ts);
    }
}

/// `do_cmd_note` (cmd4.cc:2602): type a note, Enter saves, Esc cancels.
fn note_input(ctx: &mut InputCtx) {
    let keys = ctx.keys.clone();
    if keys.just_pressed(KeyCode::Escape) {
        ctx.notes.input = None;
        ctx.log.add("Note cancelled.");
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::Enter) {
        if let Some(text) = ctx.notes.input.take() {
            let text = text.trim().to_string();
            if !text.is_empty() {
                let (turn, depth) = (ctx.ps.turn, ctx.ps.depth);
                ctx.notes
                    .add_note(crate::notes::NOTE_USER, &text, turn, depth);
                ctx.log.add(format!("Noted: {}", text));
            }
        }
        return;
    }
    let mut buf = ctx.notes.input.take().unwrap_or_default();
    for k in keys.get_just_pressed() {
        match k {
            KeyCode::Backspace => {
                buf.pop();
            }
            _ => {
                if let Some(c) = key_char(*k, shift) {
                    if buf.chars().count() < 60 {
                        buf.push(c);
                    }
                }
            }
        }
    }
    ctx.notes.input = Some(buf);
}

// --- Automatizer application (squeltch.cc) ---

/// DestroyRule::do_apply_rule guards (rule.cc:225): aware, uninscribed,
/// non-artifact and not CURSE_NO_DROP.
fn destroy_allowed(ctx: &InputCtx, it: &item::Item) -> bool {
    crate::squeltch::object_aware_p(&ctx.gd, &ctx.inv, it)
        && it.inscription.is_empty()
        && !item::is_artifact(&ctx.gd, it)
        && !item::item_flags(&ctx.gd, it)
            .iter()
            .any(|f| *f == "CURSE_NO_DROP")
}

/// `squeltch_grid` (squeltch.cc:63): apply the automatizer to the pile at
/// (x, y).  Auto-pickup removes the object from the floor, auto-destroy
/// eliminates it, auto-inscribe writes the inscription.
fn squeltch_grid(ctx: &mut InputCtx, x: i32, y: i32) {
    if !ctx.automatizer.enabled || ctx.automatizer.rules.is_empty() {
        return;
    }
    let entities: Vec<Entity> = ctx
        .stacks
        .iter()
        .filter(|(_, p, s)| p.x == x && p.y == y && !s.stack.is_empty())
        .map(|(e, _, _)| e)
        .collect();
    for e in entities {
        let items: Vec<item::Item> = match ctx.stacks.get(e) {
            Ok((_, _, s)) => s.stack.clone(),
            Err(_) => continue,
        };
        let mut kept = Vec::new();
        for mut it in items {
            let action = {
                let m = crate::squeltch::MatchCtx::new(&ctx.gd, &ctx.inv, &ctx.ps, &it);
                ctx.automatizer.matching_action(&it, &m)
            };
            match action {
                Some(crate::squeltch::RuleAction::Destroy) => {
                    if destroy_allowed(ctx, &it) {
                        ctx.log.add("<Auto-destroy>");
                    } else {
                        kept.push(it);
                    }
                }
                Some(crate::squeltch::RuleAction::Pickup) => {
                    if item::inven_carry_okay(&ctx.gd, &ctx.inv, &it) {
                        ctx.log.add("<Auto-pickup>");
                        ctx.inv.add_with(&ctx.gd, it);
                    } else {
                        kept.push(it);
                    }
                }
                Some(crate::squeltch::RuleAction::Inscribe { inscription }) => {
                    if it.inscription.is_empty() {
                        ctx.log.add(format!("<Auto-Inscribe {{{}}}>", inscription));
                        it.inscription = inscription;
                    }
                    kept.push(it);
                }
                None => kept.push(it),
            }
        }
        let empty = kept.is_empty();
        if let Ok((_, _, mut s)) = ctx.stacks.get_mut(e) {
            s.stack = kept;
        }
        if empty {
            ctx.commands.entity(e).despawn();
        }
    }
}

/// `squeltch_inventory` (squeltch.cc:90): walk the pack until no rule
/// fires (100 iterations max, "changed" left over reports the original
/// "'apply_rules' ran too often." message).
fn squeltch_inventory(ctx: &mut InputCtx) {
    if !ctx.automatizer.enabled || ctx.automatizer.rules.is_empty() {
        return;
    }
    let mut changed = true;
    let mut iterations = 0;
    while changed && iterations < 100 {
        changed = false;
        iterations += 1;
        for i in 0..ctx.inv.pack.len() {
            let it = ctx.inv.pack[i].clone();
            let action = {
                let m = crate::squeltch::MatchCtx::new(&ctx.gd, &ctx.inv, &ctx.ps, &it);
                ctx.automatizer.matching_action(&it, &m)
            };
            match action {
                Some(crate::squeltch::RuleAction::Destroy) => {
                    if destroy_allowed(ctx, &it) {
                        ctx.log.add("<Auto-destroy>");
                        ctx.inv.pack.remove(i);
                        changed = true;
                        break;
                    }
                }
                Some(crate::squeltch::RuleAction::Inscribe { inscription }) => {
                    if it.inscription.is_empty() {
                        ctx.log.add(format!("<Auto-Inscribe {{{}}}>", inscription));
                        ctx.inv.pack[i].inscription = inscription;
                        changed = true;
                        break;
                    }
                }
                // PickupRule on an inventory slot is a no-op (rule.cc:270).
                _ => {}
            }
        }
    }
    if changed {
        ctx.log.add("'apply_rules' ran too often.");
    }
}

// --- Companion command menu (cmd1.cc:3863 do_cmd_pet) ---

/// Build the menu, mirroring the only-if-pets check and command list.
fn start_pet_menu(ctx: &mut InputCtx) {
    if ctx.ps.confuse > 0 {
        ctx.log.add("You are too confused to command your pets.");
        return;
    }
    let pets = ctx.monsters.iter().filter(|(_, m, _)| m.companion).count();
    if pets == 0 {
        ctx.log.add("You have no pets/companions.");
        return;
    }
    ctx.pet.commands = vec![
        PetCommand::DismissPets,
        PetCommand::DismissCompanions,
        PetCommand::CallPets,
        PetCommand::FollowMe,
        PetCommand::SeekAndDestroy,
        PetCommand::ToggleDoors,
        PetCommand::TogglePickup,
        PetCommand::GiveTargetFriend,
        PetCommand::GiveTargetAll,
        PetCommand::ForgetTarget,
    ];
    ctx.pet.open = true;
    ctx.pet.confirm = None;
    let opts = (*ctx.pet_opts).clone();
    ctx.log.add("(Command a-j, ESC=exit) Select a command:");
    for (i, c) in ctx.pet.commands.clone().iter().enumerate() {
        ctx.log
            .add(format!("{}) {}", (b'a' + i as u8) as char, c.desc(&opts)));
    }
}

/// Dismissible monsters without NO_DEATH: `pets` selects MSTATUS_PET
/// (monster2.cc `pet`), otherwise MSTATUS_COMPANION (cmd1.cc:4122/4198).
fn companion_entities(ctx: &InputCtx, pets: bool) -> Vec<(Entity, String)> {
    ctx.monsters
        .iter()
        .filter(|(_, m, _)| m.companion)
        .filter(|(_, m, _)| if pets { m.pet } else { !m.pet })
        .filter(|(_, m, _)| !ctx.gd.monsters[m.def].has("NO_DEATH"))
        .map(|(e, m, _)| (e, ctx.gd.monsters[m.def].name.clone()))
        .collect()
}

fn pet_menu_input(ctx: &mut InputCtx) {
    let keys = ctx.keys.clone();
    if keys.just_pressed(KeyCode::Escape) {
        ctx.pet.open = false;
        ctx.pet.confirm = None;
        ctx.log.add("Command cancelled.");
        return;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let Some(k) = keys.get_just_pressed().next().copied() else {
        return;
    };
    // Confirmation for an upper-case (verified) command.
    if let Some(i) = ctx.pet.confirm {
        let c = key_char(k, shift);
        if matches!(c, Some('y') | Some('Y')) {
            ctx.pet.confirm = None;
            ctx.pet.open = false;
            let cmd = ctx.pet.commands[i];
            pet_command(ctx, cmd);
        } else if matches!(c, Some('n') | Some('N')) {
            ctx.pet.confirm = None;
        }
        return;
    }
    if k == KeyCode::Enter && ctx.pet.commands.len() == 1 {
        let cmd = ctx.pet.commands[0];
        ctx.pet.open = false;
        pet_command(ctx, cmd);
        return;
    }
    let Some(c) = key_char(k, shift) else { return };
    let idx = if c.is_ascii_alphabetic() {
        (c.to_ascii_lowercase() as usize).saturating_sub('a' as usize)
    } else if c.is_ascii_digit() {
        26 + (c as usize - '0' as usize)
    } else {
        return;
    };
    if idx >= ctx.pet.commands.len() {
        return;
    }
    if c.is_ascii_uppercase() {
        let opts = (*ctx.pet_opts).clone();
        let desc = ctx.pet.commands[idx].desc(&opts);
        ctx.pet.confirm = Some(idx);
        ctx.log.add(format!("Use {}? [y/n]", desc));
        return;
    }
    ctx.pet.open = false;
    let cmd = ctx.pet.commands[idx];
    pet_command(ctx, cmd);
}

/// Execute one `do_cmd_pet` choice (cmd1.cc:4060-4270).
fn pet_command(ctx: &mut InputCtx, cmd: PetCommand) {
    match cmd {
        // Dismiss pets (MSTATUS_PET) or companions (MSTATUS_COMPANION);
        // the original processes the two buckets separately.
        PetCommand::DismissPets | PetCommand::DismissCompanions => {
            let pets = cmd == PetCommand::DismissPets;
            let targets = companion_entities(ctx, pets);
            let noun = if pets { "pet" } else { "companion" };
            if targets.is_empty() {
                ctx.log.add(format!(
                    "You have no dismissible {}s.",
                    noun
                ));
                return;
            }
            let mut dismissed = 0;
            for (e, name) in targets {
                ctx.log.add(format!("You dismiss the {}.", name));
                ctx.commands.entity(e).despawn();
                dismissed += 1;
            }
            ctx.log.add(format!(
                "You have dismissed {} {}{}.",
                dismissed,
                noun,
                if dismissed == 1 { "" } else { "s" }
            ));
        }
        // Call pets: pet_follow_distance = 1.
        PetCommand::CallPets => {
            ctx.pet_opts.follow_distance = 1;
            ctx.log.add("You call your pets back to your side.");
        }
        // Follow me: pet_follow_distance = 6 (the birth default).
        PetCommand::FollowMe => {
            ctx.pet_opts.follow_distance = 6;
            ctx.log.add("Your pets follow you closely.");
        }
        // Seek and destroy: pet_follow_distance = 255.
        PetCommand::SeekAndDestroy => {
            ctx.pet_opts.follow_distance = 255;
            ctx.log.add("Your pets seek and destroy the enemy!");
        }
        // Toggle pets opening doors.
        PetCommand::ToggleDoors => {
            ctx.pet_opts.open_doors = !ctx.pet_opts.open_doors;
            if ctx.pet_opts.open_doors {
                ctx.log.add("Your pets may open doors.");
            } else {
                ctx.log.add("Your pets will not open doors.");
            }
        }
        // Toggle pets picking items up; turning it off drops what they
        // carry (monster_drop_carried_objects, cmd1.cc:4258).
        PetCommand::TogglePickup => {
            ctx.pet_opts.pickup_items = !ctx.pet_opts.pickup_items;
            if ctx.pet_opts.pickup_items {
                ctx.log.add("Your pets may pick up items.");
            } else {
                ctx.log.add("Your pets will not pick up items.");
                let carriers: Vec<(Entity, i32, i32)> = ctx
                    .monsters
                    .iter()
                    .filter(|(_, m, _)| m.companion && !m.items.is_empty())
                    .map(|(e, _, p)| (e, p.x, p.y))
                    .collect();
                let mut dropped = 0;
                for (e, x, y) in carriers {
                    let items = match ctx.monsters.get_mut(e) {
                        Ok((_, mut m, _)) => std::mem::take(&mut m.items),
                        Err(_) => Vec::new(),
                    };
                    for it in items {
                        item::place_floor_item(
                            &mut ctx.commands,
                            &ctx.gd,
                            &ctx.tiles,
                            &mut ctx.stacks,
                            x,
                            y,
                            it,
                        );
                        dropped += 1;
                    }
                }
                if dropped > 0 {
                    ctx.log.add("Your pets drop what they were carrying.");
                }
            }
        }
        // Give target to a friend: `case 7` has no body in the original
        // (cmd1.cc switches on 1-6 and 8-10 only), so it does nothing.
        PetCommand::GiveTargetFriend => {
            ctx.log.add("Nothing happens.");
        }
        // Give target to all friends / forget target: pick a monster with
        // a direction (the original uses a full-screen targeting cursor).
        PetCommand::GiveTargetAll | PetCommand::ForgetTarget => {
            ctx.pet.pending_target = Some(cmd);
            if cmd == PetCommand::ForgetTarget {
                ctx.log.add("Select the friendly monster (direction).");
            } else {
                ctx.log.add("Select the target monster (direction).");
            }
        }
    }
}

/// Direction-based target selection for the pet commands (`tgt_pt`).
fn pet_target_input(ctx: &mut InputCtx, cmd: PetCommand) {
    let keys = ctx.keys.clone();
    if keys.just_pressed(KeyCode::Escape) {
        ctx.pet.pending_target = None;
        ctx.log.add("Cancelled.");
        return;
    }
    let Some((dx, dy)) = direction_key(&keys) else {
        return;
    };
    ctx.pet.pending_target = None;
    let Some((px, py)) = player_pos(ctx) else {
        return;
    };
    // The first monster along the ray, within sight (approximates the
    // cursor: tgt_pt picks any visible cell).
    let mut found: Option<(Entity, bool, usize, i32, i32)> = None;
    for step in 1..=12 {
        let (x, y) = (px + dx * step, py + dy * step);
        if !Map::in_bounds(x, y) || ctx.map.opaque(&ctx.gd, x, y) {
            break;
        }
        if let Some((e, m, _)) = ctx.monsters.iter().find(|(_, _, p)| p.x == x && p.y == y) {
            // Orders go to MSTATUS_PET/COMPANION only, never to friendly
            // NPCs (cmd1.cc:4094 `status < MSTATUS_PET`).
            found = Some((e, m.companion, m.def, x, y));
            break;
        }
    }
    let Some((e, is_friendly, _, _, _)) = found else {
        if cmd == PetCommand::ForgetTarget {
            ctx.log.add("This is not a correct friend.");
        } else {
            ctx.log.add("This is not a correct target.");
        }
        return;
    };
    match cmd {
        PetCommand::ForgetTarget => {
            if !is_friendly {
                ctx.log.add("You cannot give orders to this monster.");
                return;
            }
            if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                m.target = None;
            }
            ctx.log.add("Your friend forgets its target.");
        }
        PetCommand::GiveTargetAll => {
            // The selected monster is the target; every pet/friend hunts it.
            let mut given = 0;
            let targets: Vec<Entity> = ctx
                .monsters
                .iter()
                .filter(|(pe, m, _)| *pe != e && m.companion)
                .map(|(pe, _, _)| pe)
                .collect();
            for pe in targets {
                if let Ok((_, mut m, _)) = ctx.monsters.get_mut(pe) {
                    m.target = Some(e);
                }
                given += 1;
            }
            ctx.log.add("Target selected.");
            if given > 0 {
                ctx.log
                    .add(format!("{} friend(s) now hunt that target.", given));
            }
        }
        PetCommand::GiveTargetFriend => {}
        _ => {}
    }
}

// --- Drop / destroy with quantities (cmd3.cc do_cmd_drop/do_cmd_destroy) ---

fn start_item_picker(ctx: &mut InputCtx, mode: PickerMode) {
    ctx.picker.mode = Some(mode);
    ctx.picker.view = PickerView::Pack;
    ctx.picker.sel = None;
    ctx.picker.qty = None;
    ctx.picker.create_rule = false;
    picker_show(ctx);
}

/// List the current view for the text-mode selector.
fn picker_show(ctx: &mut InputCtx) {
    let Some(mode) = ctx.picker.mode else { return };
    let verb = if mode == PickerMode::Drop {
        "Drop"
    } else {
        "Destroy"
    };
    match ctx.picker.view {
        PickerView::Pack => {
            let other = if mode == PickerMode::Drop {
                "equip"
            } else {
                "floor"
            };
            ctx.log
                .add(format!("{} which item? (a-w, /={}, Esc)", verb, other));
            for (i, it) in ctx.inv.pack.iter().enumerate().take(23) {
                ctx.log.add(format!(
                    "  {}) {}",
                    (b'a' + i as u8) as char,
                    it.label(&ctx.gd, &ctx.inv.known)
                ));
            }
        }
        PickerView::Equip => {
            ctx.log
                .add(format!("{} which worn item? (a-z, /=back, Esc)", verb));
            let mut n = 0;
            for (slot, it) in ctx.inv.equip.iter().enumerate() {
                if let Some(it) = it {
                    ctx.log.add(format!(
                        "  {}) {:<8} {}",
                        (b'a' + n as u8) as char,
                        data::SLOT_NAMES[slot],
                        it.label(&ctx.gd, &ctx.inv.known)
                    ));
                    n += 1;
                }
            }
        }
        PickerView::Floor => {
            ctx.log.add("Destroy which floor item? (a-z, /=back, Esc)");
            if let Some((x, y)) = player_pos(ctx) {
                let items: Vec<item::Item> = ctx
                    .stacks
                    .iter()
                    .filter(|(_, p, _)| p.x == x && p.y == y)
                    .flat_map(|(_, _, s)| s.stack.clone())
                    .collect();
                for (i, it) in items.iter().enumerate() {
                    ctx.log.add(format!(
                        "  {}) {}",
                        (b'a' + i as u8) as char,
                        it.label(&ctx.gd, &ctx.inv.known)
                    ));
                }
            }
        }
    }
}

fn item_picker_input(ctx: &mut InputCtx) {
    let keys = ctx.keys.clone();
    let mode = ctx.picker.mode.unwrap();
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Quantity prompt open: digits, Backspace, Enter, Esc.
    if let Some(buf) = ctx.picker.qty.take() {
        let mut buf = buf;
        if keys.just_pressed(KeyCode::Escape) {
            cancel_picker(ctx);
            return;
        }
        if keys.just_pressed(KeyCode::Enter) {
            let amt = buf.parse::<u32>().unwrap_or(1).max(1);
            picker_execute(ctx, amt);
            return;
        }
        for k in keys.get_just_pressed() {
            match k {
                KeyCode::Backspace => {
                    buf.pop();
                }
                _ => {
                    if let Some(c) = key_char(*k, shift) {
                        if c.is_ascii_digit() && buf.len() < 3 {
                            buf.push(c);
                        }
                    }
                }
            }
        }
        ctx.picker.qty = Some(buf);
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        cancel_picker(ctx);
        return;
    }
    // '/' cycles the source list (get_item flags): drop uses
    // USE_INVEN/USE_EQUIP, destroy USE_INVEN/USE_FLOOR (cmd3.cc).
    if keys.just_pressed(KeyCode::Slash) {
        ctx.picker.view = match (mode, ctx.picker.view) {
            (PickerMode::Drop, PickerView::Pack) => PickerView::Equip,
            (PickerMode::Drop, _) => PickerView::Pack,
            (PickerMode::Destroy, PickerView::Pack) => PickerView::Floor,
            (PickerMode::Destroy, _) => PickerView::Pack,
        };
        ctx.picker.sel = None;
        picker_show(ctx);
        return;
    }
    // '$': toggle automatizer rule creation (item selection, object1.cc).
    if shift && keys.just_pressed(KeyCode::Digit4) {
        ctx.picker.create_rule = !ctx.picker.create_rule;
        ctx.log.add(if ctx.picker.create_rule {
            "New automatizer rule: ON"
        } else {
            "New automatizer rule: OFF"
        });
        return;
    }
    let Some(k) = keys.get_just_pressed().next().copied() else {
        return;
    };
    let Some(c) = key_char(k, shift) else { return };
    if !c.is_ascii_alphabetic() {
        return;
    }
    let idx = (c.to_ascii_lowercase() as usize).saturating_sub('a' as usize);
    let sel = match ctx.picker.view {
        PickerView::Pack => {
            if idx < ctx.inv.pack.len() {
                Some(PickerSel::Pack(idx))
            } else {
                None
            }
        }
        PickerView::Equip => {
            let mut n = 0;
            let mut found = None;
            for (slot, it) in ctx.inv.equip.iter().enumerate() {
                if it.is_some() {
                    if n == idx {
                        found = Some(PickerSel::Equip(slot));
                        break;
                    }
                    n += 1;
                }
            }
            found
        }
        PickerView::Floor => {
            let mut flattened: Vec<(Entity, usize)> = Vec::new();
            if let Some((x, y)) = player_pos(ctx) {
                for (e, p, s) in ctx.stacks.iter() {
                    if p.x == x && p.y == y {
                        for i in 0..s.stack.len() {
                            flattened.push((e, i));
                        }
                    }
                }
            }
            flattened
                .get(idx)
                .copied()
                .map(|(e, i)| PickerSel::Floor(e, i))
        }
    };
    let Some(sel) = sel else { return };
    let count = sel.count(ctx);
    ctx.picker.sel = Some(sel);
    if count > 1 {
        ctx.picker.qty = Some(String::new());
        ctx.log
            .add(format!("Quantity (1-{}, Enter = 1, Esc = cancel):", count));
    } else {
        picker_execute(ctx, 1);
    }
}

fn cancel_picker(ctx: &mut InputCtx) {
    ctx.picker.mode = None;
    ctx.picker.sel = None;
    ctx.picker.qty = None;
    ctx.log.add("Cancelled.");
}

fn picker_execute(ctx: &mut InputCtx, amt: u32) {
    let Some(mode) = ctx.picker.mode else { return };
    let Some(sel) = ctx.picker.sel.take() else {
        return;
    };
    ctx.picker.mode = None;
    ctx.picker.qty = None;
    match mode {
        PickerMode::Drop => picker_drop(ctx, sel, amt),
        PickerMode::Destroy => picker_destroy(ctx, sel, amt),
    }
}

/// `do_cmd_drop` (cmd3.cc:487): quantity, curse checks, One Ring and
/// q_poison hooks, then `inven_drop` (object2.cc:5761).
fn picker_drop(ctx: &mut InputCtx, sel: PickerSel, amt: u32) {
    let (it, restore): (item::Item, Option<PickerSel>) = match sel {
        PickerSel::Pack(i) => {
            if i >= ctx.inv.pack.len() {
                return;
            }
            (ctx.inv.pack[i].clone(), Some(PickerSel::Pack(i)))
        }
        PickerSel::Equip(slot) => {
            let Some(it) = ctx.inv.equip[slot].clone() else {
                return;
            };
            // A cursed worn item cannot be removed.
            if it.cursed {
                ctx.log.add("Hmmm, it seems to be cursed.");
                return;
            }
            (it, Some(PickerSel::Equip(slot)))
        }
        PickerSel::Floor(..) => {
            // Drop supports USE_EQUIP | USE_INVEN only.
            return;
        }
    };
    let flags = item::item_flags(&ctx.gd, &it)
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    // The One Ring can only be destroyed in the Great Fire of Mount Doom
    // (q_one.cc HOOK_DROP: cquest.status TAKEN and cave feat FEAT_GREAT_FIRE
    // exactly; the port keeps its depth-60 gate as the Great Fire lies at
    // Mount Doom's depth 99).
    if it.artifact == 13 {
        let (px, py) = player_pos(ctx).unwrap_or((0, 0));
        let t = ctx.map.terrain_at(px, py);
        if ctx.plot.one_taken && ctx.ps.depth >= 60 && t == 178 {
            if let Some(PickerSel::Pack(i)) = restore {
                ctx.inv.pack.remove(i);
            } else if let Some(PickerSel::Equip(slot)) = restore {
                ctx.inv.equip[slot] = None;
            }
            ctx.ps.ring_destroyed = true;
            ctx.plot.set(game::PLOT_ONE, game::PLOT_COMPLETED);
            ctx.plot.set(game::PLOT_SAURON, game::PLOT_TAKEN);
            ctx.log.add("You throw the One Ring into the Great Fire; it is rapidly consumed");
            ctx.log.add("by the searing flames.");
            ctx.log.add("You feel the powers of evil weakening.");
            ctx.log.add("Now you can go onto the hunt for Sauron!");
            if ctx.ps.god == 4 {
                ctx.log.add("Melkor abandons you in fury.");
                ctx.ps.god = 0;
                ctx.ps.grace = 0;
                ctx.ps.praying = false;
            }
        } else {
            ctx.log
                .add("You cannot bring yourself to drop the Ring here.");
            ctx.log
                .add("(It can only be destroyed in great fire, at depth 60 or below.)");
        }
        consume_turn(ctx);
        return;
    }
    if it.cursed && flags.iter().any(|f| f == "CURSE_NO_DROP") {
        ctx.log.add("Hmmm, you seem to be unable to drop it.");
        return;
    }
    // Purifying the tainted pond (q_poison.cc HOOK_DROP): counting every
    // monster with status <= MSTATUS_NEUTRAL (the port's non-friendly,
    // non-companion test).  The flask itself is then dropped normally; the
    // mayor's thanks and reward wait for the report (finish hook).
    if ctx.ps.depth == 0
        && ctx.plot.poison_cell == Some(ctx.map.wild)
        && ctx.gd.objects[it.def].name == "Water Curing"
        && ctx.plot.status(game::PLOT_POISON) == game::PLOT_TAKEN
    {
        let alive = ctx
            .monsters
            .iter()
            .filter(|(_, m, _)| !m.friendly && !m.companion)
            .count();
        if alive >= 10 {
            // The hook returns true: the drop is cancelled, no turn spent.
            ctx.log
                .add("There are too many monsters left to cure the water.");
            return;
        }
        ctx.plot.set(game::PLOT_POISON, game::PLOT_COMPLETED);
        for t in ctx.map.terrain.iter_mut() {
            if *t == map::T_TAINTED_WATER {
                *t = map::T_SHAL_WATER;
            }
        }
        ctx.log
            .add("Well done! The water seems to be clean now.");
        // Fall through to the normal drop so the empty flask lands on the
        // water (q_poison.cc success returns false).
    }
    // Take off worn equipment first (inven_takeoff); the item lands in the
    // pack and is then dropped like any pack object.
    let pack_idx = match restore {
        Some(PickerSel::Pack(i)) => i,
        Some(PickerSel::Equip(slot)) => {
            let Some(eq) = ctx.inv.equip[slot].take() else {
                return;
            };
            ctx.inv.add_with(&ctx.gd, eq.clone());
            ctx.inv
                .pack
                .iter()
                .position(|p| item::items_similar(&ctx.gd, p, &eq))
                .unwrap_or(ctx.inv.pack.len().saturating_sub(1))
        }
        Some(PickerSel::Floor(..)) | None => return,
    };
    let full = ctx.inv.pack[pack_idx].clone();
    let amt = amt.min(full.count).max(1);
    let mut dropped = full.clone();
    dropped.count = amt;
    // Wand charges are split proportionally (object2.cc:5761).
    if ctx.gd.objects[full.def].tval == data::TV_WAND && amt < full.count {
        dropped.charges = full.charges * amt as i32 / full.count as i32;
        ctx.inv.pack[pack_idx].charges -= dropped.charges;
    }
    let dname = dropped.label(&ctx.gd, &ctx.inv.known);
    let letter = (b'a' + pack_idx.min(25) as u8) as char;
    ctx.inv.pack[pack_idx].count -= amt;
    if ctx.inv.pack[pack_idx].count == 0 {
        ctx.inv.pack.remove(pack_idx);
    }
    let (px, py) = player_pos(ctx).unwrap_or((0, 0));
    item::place_floor_item(
        &mut ctx.commands,
        &ctx.gd,
        &ctx.tiles,
        &mut ctx.stacks,
        px,
        py,
        dropped,
    );
    // object2.cc:5761: "You drop %s (%c)."
    ctx.log.add(format!("You drop {} ({}).", dname, letter));
    consume_turn(ctx);
}

/// `do_cmd_destroy` (cmd3.cc:553): quantity, curse/artifact guards, wand
/// stack bookkeeping, blessed-piety, the automatizer rule hook and
/// `inc_stack_size(item, -amt)`.
fn picker_destroy(ctx: &mut InputCtx, sel: PickerSel, amt: u32) {
    let (mut it, restore): (item::Item, PickerSel) = match sel {
        PickerSel::Pack(i) => {
            if i >= ctx.inv.pack.len() {
                return;
            }
            (ctx.inv.pack[i].clone(), PickerSel::Pack(i))
        }
        PickerSel::Equip(slot) => {
            let Some(it) = ctx.inv.equip[slot].clone() else {
                return;
            };
            // Cursed worn gear cannot be taken off to destroy (cmd3.cc
            // uses USE_INVEN | USE_FLOOR, so equipment is only reachable
            // through the port's list; keep the curse guard).
            if it.cursed {
                ctx.log.add("Hmmm, it seems to be cursed.");
                return;
            }
            (it, PickerSel::Equip(slot))
        }
        PickerSel::Floor(e, i) => {
            let Some((_, _, s)) = ctx.stacks.get(e).ok() else {
                return;
            };
            let Some(it) = s.stack.get(i).cloned() else {
                return;
            };
            (it, PickerSel::Floor(e, i))
        }
    };
    let flags = item::item_flags(&ctx.gd, &it)
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if it.cursed && flags.iter().any(|f| f == "CURSE_NO_DROP") {
        ctx.log.add("Hmmm, you seem to be unable to destroy it.");
        return;
    }
    if it.artifact != 0 {
        ctx.log.add(format!(
            "You cannot destroy {}.",
            it.label(&ctx.gd, &ctx.inv.known)
        ));
        return;
    }
    let amt = amt.min(it.count).max(1);
    it.count = amt;
    let name = it.label(&ctx.gd, &ctx.inv.known);
    ctx.log.add(format!("You destroy {}.", name));
    // Create an automatizer rule (cmd3.cc:637).
    if ctx.automatizer.create_rule || ctx.picker.create_rule {
        let i = crate::squeltch::easy_add_rule(
            &mut ctx.automatizer,
            &ctx.gd,
            &ctx.inv,
            &it,
            crate::squeltch::AddRuleMode::TvalSval,
            it.identified,
        );
        ctx.log
            .add("Rule added. Please go to the Automatizer screen (press = then T)");
        ctx.log.add("to save the modified ruleset.");
        let _ = i;
    }
    // Eru frowns upon destroying blessed items (cmd3.cc).
    if flags.iter().any(|f| *f == "BLESSED") && ctx.ps.god == 1 {
        ctx.ps.grace -= 10 * ctx.gd.objects[it.def].depth as i32;
    }
    match restore {
        PickerSel::Pack(i) => {
            // Wand charge bookkeeping on a partial destruction.
            if ctx.gd.objects[ctx.inv.pack[i].def].tval == data::TV_WAND
                && amt < ctx.inv.pack[i].count
            {
                let total = ctx.inv.pack[i].count;
                let charges = ctx.inv.pack[i].charges;
                ctx.inv.pack[i].charges -= charges * amt as i32 / total as i32;
            }
            let full = ctx.inv.pack[i].count;
            if amt >= full {
                ctx.inv.pack.remove(i);
            } else {
                ctx.inv.pack[i].count -= amt;
            }
        }
        PickerSel::Equip(slot) => {
            ctx.inv.equip[slot] = None;
        }
        PickerSel::Floor(e, i) => {
            if let Ok((_, _, mut s)) = ctx.stacks.get_mut(e) {
                if i < s.stack.len() {
                    if amt >= s.stack[i].count {
                        s.stack.remove(i);
                    } else {
                        s.stack[i].count -= amt;
                    }
                }
            }
            let empty = ctx
                .stacks
                .get(e)
                .map(|(_, _, s)| s.stack.is_empty())
                .unwrap_or(false);
            if empty {
                ctx.commands.entity(e).despawn();
            }
        }
    }
    // Destroying takes no time (energy_use = 0), exactly like the original.
}

/// Probability Travel movement (spells2.cc passwall): slide through walls
/// until the first open cell, never landing on a monster.
fn passwall(ctx: &mut InputCtx, dx: i32, dy: i32) -> bool {
    if ctx.ps.wild_mode || ctx.ps.depth >= game::PLOT_DEPTH_BASE {
        return false;
    }
    if ctx.ps.depth > 0 && game::level_has_flag(&ctx.gd, &ctx.ps, "NO_TELEPORT") {
        return false;
    }
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return false;
    };
    let (mut x, mut y) = (pos.x, pos.y);
    for _ in 0..20 {
        x += dx;
        y += dy;
        if !Map::in_bounds(x, y) {
            return false;
        }
        if ctx.monsters.iter().any(|(_, _, mp)| mp.x == x && mp.y == y) {
            continue;
        }
        let def = ctx.gd.terrain(ctx.map.terrain_at(x, y));
        if def.permanent {
            return false;
        }
        if !def.is_floor || def.no_walk {
            continue;
        }
        {
            let Ok(mut p) = ctx.player.single_mut() else {
                return false;
            };
            p.x = x;
            p.y = y;
        }
        ctx.log.add("You pass through the walls...");
        return true;
    }
    false
}

/// Collect the full game state for saving (called on quit).
fn build_save(ctx: &mut InputCtx) -> crate::save::SaveGame {
    let player_pos = ctx.player.single().map(|p| (p.x, p.y)).unwrap_or((0, 0));
    let monsters = ctx
        .monsters
        .iter()
        .map(|(e, m, p)| {
            crate::save::MonsterSave::from_monster_with_extra(m, ctx.monster_extras.get(e).ok(), p)
        })
        .collect();
    let floor_items = ctx
        .stacks
        .iter()
        .map(|(_, p, stack)| (p.x, p.y, stack.stack.clone()))
        .collect();
    let floor_gold = ctx
        .golds
        .iter()
        .map(|(_, p, g)| (p.x, p.y, g.amount))
        .collect();
    crate::save::SaveGame {
        ps: ctx.ps.clone(),
        inv: ctx.inv.clone(),
        plot: ctx.plot.clone(),
        stocks: ctx.stocks.clone(),
        created: ctx.created.0.clone(),
        player_pos,
        map: ctx.map.clone(),
        monsters,
        floor_items,
        floor_gold,
        levels: ctx.store.clone(),
        rng_state: crate::rng::do_randomizer_state(),
        options: ctx.options.clone(),
    }
}

/// Pick up everything lying on the player's cell.
fn get_items(ctx: &mut InputCtx) {
    let Some((px, py)) = player_pos(ctx) else {
        return;
    };
    // py_pickup_floor order: the automatizer runs on the pile before the
    // manual pickup, so auto-destroyed objects never reach the pack and
    // auto-pickup rules grab theirs.
    squeltch_grid(ctx, px, py);
    if !pickup_floor(ctx, px, py) {
        ctx.log.add("There is nothing here to pick up.");
        return;
    }
    consume_turn(ctx);
}

/// `sense_floor` (object1.cc:5339): stepping on a grid makes the player
/// aware of the object kinds there and identifies the individual items
/// (`object_aware` + `object_known`), before the automatizer runs.
fn sense_floor(ctx: &mut InputCtx, x: i32, y: i32) {
    for (_, p, mut stack) in ctx.stacks.iter_mut() {
        if p.x != x || p.y != y {
            continue;
        }
        for it in stack.stack.iter_mut() {
            ctx.inv.learn(it.def);
            it.identified = true;
        }
    }
}

/// `py_pickup_floor(pickup=1)` (object1.cc:5388): take the whole pile.
/// Returns whether anything was picked up.  Quest pickup hooks (q_fireprof
/// scroll, q_god relic) fire here, exactly as in the original.
fn pickup_floor(ctx: &mut InputCtx, px: i32, py: i32) -> bool {
    // object1.cc py_pickup_floor: identify the pile before squelching and
    // picking (sense_floor + object_pickup's aware/known calls).
    sense_floor(ctx, px, py);
    let mut got = false;
    let mut picked: Vec<usize> = Vec::new();
    // The marked quest Scroll of Fire (q_fireprof get hook: pval2 == sval
    // and the "quest" inscription).
    let mut got_fire_scroll = false;
    for (e, p, mut stack) in ctx.stacks.iter_mut() {
        if p.x != px || p.y != py {
            continue;
        }
        let taken: Vec<item::Item> = stack.stack.drain(..).collect();
        let mut left = Vec::new();
        let mut full = false;
        for it in taken {
            // 23 pack slots (inventory.hpp INVEN_PACK); stacking items
            // still fit when a matching stack exists.
            if ctx.inv.pack.len() >= 23 && ctx.inv.merge_target(&ctx.gd, &it).is_none() {
                if !full {
                    ctx.log.add("You have no room for it.");
                    full = true;
                }
                left.push(it);
                continue;
            }
            // A symbiotic monster cannot be picked up without the skill
            // (object1.cc TV_HYPNOS).
            if ctx.gd.objects[it.def].tval == data::TV_HYPNOS
                && ctx.ps.skill(crate::skill::SK_SYMBIOTIC) == 0
            {
                ctx.log
                    .add("You have no idea how to handle a symbiotic monster; you leave it.");
                left.push(it);
                continue;
            }
            // `can_carry_heavy` would prompt in the original; without a
            // prompt the port skips the heavier-than-current burden.
            if !game::can_carry_heavy(&ctx.ps, &ctx.inv, &ctx.gd, &it) {
                ctx.log.add(format!(
                    "You are already carrying too much to pick {} up.",
                    it.label(&ctx.gd, &ctx.inv.known)
                ));
                left.push(it);
                continue;
            }
            let label = it.label(&ctx.gd, &ctx.inv.known);
            // The artifact note (spells2.cc:2074 note_found_object).
            if it.artifact != 0 || !it.artifact_name.is_empty() {
                let (turn, depth) = (ctx.ps.turn, ctx.ps.depth);
                ctx.notes.add_artifact(&label, turn, depth);
            }
            ctx.log.add(format!("You get {}.", label));
            picked.push(it.def);
            if it.pval2 == 48 && it.inscription == "quest" {
                got_fire_scroll = true;
            }
            ctx.inv.add_with(&ctx.gd, it);
        }
        if left.is_empty() {
            ctx.commands.entity(e).despawn();
        } else {
            stack.stack = left;
        }
        got = true;
    }
    // The marked Scroll of Fire completes the Old Mage's quest
    // (q_fireprof.cc get hook).
    if ctx.ps.depth == game::PLOT_DEPTH_BASE + game::PLOT_FIREPROOF
        && ctx.plot.status(game::PLOT_FIREPROOF) == game::PLOT_TAKEN
        && got_fire_scroll
    {
        ctx.plot.set(game::PLOT_FIREPROOF, game::PLOT_COMPLETED);
        // 24 fireproofing points (4/book, 3/staff, 1/scroll); the old
        // mage in Bree spends them on your items (q_fireprof.cc).
        ctx.plot.fireproof_points = 24;
        ctx.log.add("Fine! Looks like you've found it.");
        if let Some(qm) = ctx.gd.quest_map(game::PLOT_FIREPROOF) {
            let (ox, oy) = map::quest_map_offset(qm);
            ctx.map
                .set_terrain(qm.exit.0 + ox, qm.exit.1 + oy, map::T_STAIRS_UP);
        }
    }
    // The god's relic is returned the moment it is touched (q_god.cc).
    let is_relic = |d: usize| ctx.gd.objects[d].name.starts_with("Piece of the Relic");
    if picked.iter().any(|&d| is_relic(d)) {
        ctx.inv.pack.retain(|it| {
            !ctx.gd.objects[it.def]
                .name
                .starts_with("Piece of the Relic")
        });
        ctx.ps.relic_depth = 0;
        ctx.ps.relic_quest = false;
        ctx.ps.god_quests += 1;
        // +5*Prayer.mod per piece, +10*mod on the fifth (q_god.cc
        // quest_god_get_hook).
        game::god_relic_reward(&mut ctx.ps, &mut ctx.log);
    }
    got
}

/// Collect loose gold and report items when entering a cell.
/// cmd1.cc:453 carry / object1.cc:5365 py_pickup_floor: identify and
/// automatize the pile, absorb the gold, describe what is left and (when
/// `pickup`) collect the objects.  `pickup` comes from always_pickup for
/// the walk and stay keys.
fn carry_at(ctx: &mut InputCtx, x: i32, y: i32, pickup: bool) {
    // `py_pickup_floor` identifies the pile on every arrival
    // (object1.cc:5339 sense_floor), whether or not auto-pickup is on.
    sense_floor(ctx, x, y);
    // The automatizer sweeps the pile on every step (cmd1.cc carry ->
    // py_pickup_floor -> squeltch_grid).
    squeltch_grid(ctx, x, y);
    // always_pickup (cmd1.cc:453 carry(do_pickup)): with the option on the
    // whole pile is taken on arrival; the default is off.
    if pickup {
        pickup_floor(ctx, x, y);
    }
    for (e, p, gold) in ctx.golds.iter() {
        if p.x == x && p.y == y {
            ctx.ps.gold += gold.amount;
            // absorb_gold (object1.cc:5301).
            ctx.log.add(format!(
                "You have found {} gold pieces worth of {}.",
                gold.amount, gold.name
            ));
            ctx.commands.entity(e).despawn();
        }
    }
    for (_, p, stack) in ctx.stacks.iter() {
        if p.x == x && p.y == y && !stack.stack.is_empty() {
            if stack.stack.len() == 1 {
                ctx.log.add(format!(
                    "You see here {}.",
                    stack.stack[0].label(&ctx.gd, &ctx.inv.known)
                ));
            } else {
                ctx.log.add(format!(
                    "You see here a pile of {} items.",
                    stack.stack.len()
                ));
            }
        }
    }
}

fn cell_arrival(ctx: &mut InputCtx, x: i32, y: i32) {
    let pickup = ctx.options.always_pickup;
    cell_arrival_pickup(ctx, x, y, pickup);
}

/// cell_arrival with an explicit py_pickup_floor `pickup` flag.
fn cell_arrival_pickup(ctx: &mut InputCtx, x: i32, y: i32, pickup: bool) {
    carry_at(ctx, x, y, pickup);
    // Grid inscriptions fire when walked over (cmd1.cc:3055).
    let ci = Map::idx(x, y);
    if let Some(&rune) = ctx.map.inscriptions.get(&ci) {
        let (text, trig, _) = game::INSCRIPTIONS[rune as usize];
        ctx.log
            .add(format!("There is an inscription here: {}", text));
        if trig & game::INSCRIP_EXEC_WALK != 0 {
            let mut rng = crate::rng::current();
            let blast = game::execute_inscription(
                &mut ctx.commands,
                &ctx.gd,
                &ctx.tiles,
                &mut ctx.map,
                &mut ctx.stacks,
                x,
                y,
                rune,
                None,
                &mut ctx.log,
                &mut rng,
            );
            if let game::InscriptionBlast::Blast(gf, dam) = blast {
                game::rune_blast(
                    &mut ctx.commands,
                    &ctx.gd,
                    &ctx.tiles,
                    &mut ctx.map,
                    &mut ctx.ps,
                    &mut ctx.inv,
                    &mut ctx.plot,
                    &mut ctx.created.0,
                    &mut ctx.monsters,
                    &mut ctx.stacks,
                    x,
                    y,
                    gf,
                    dam,
                    (x, y),
                    &mut ctx.next,
                    &mut ctx.log,
                    &mut rng,
                );
            }
        }
    }
}

/// do_cmd_sense_grid_mana (cmd3.cc:1551): feel the magical energy of
/// the grid underfoot, with skill-blurred precision.
fn sense_grid_mana(ctx: &mut InputCtx) {
    let mut rng = crate::rng::current();
    let (px, py) = ctx.player.single().map(|p| (p.x, p.y)).unwrap_or((0, 0));
    let mana = ctx.map.mana.get(Map::idx(px, py)).copied().unwrap_or(0) as i32;
    let skill_dev = ctx.ps.skill_scale(crate::skill::SK_DEVICE, 150);
    let mut chance = skill_dev;
    if ctx.ps.confuse > 0 {
        chance /= 2;
    }
    chance -= mana / 10;
    if chance < 3 && rng.gen_range(0..(3 - chance + 1).max(1)) == 0 {
        chance = 3;
    }
    if chance < 3 || rng.gen_range(1..=chance) < 3 {
        ctx.log.add("You failed to sense the grid's mana.");
        return;
    }
    let blur = ((101 - skill_dev) / 2).clamp(1, 50);
    let approx = mana / blur * blur;
    ctx.log
        .add(format!("The grid holds about {} mana.", approx));
}

/// Trigger a trap the player just stepped on.
fn trigger_trap(ctx: &mut InputCtx, x: i32, y: i32) {
    ctx.map.known_traps.insert(Map::idx(x, y));
    let mut rng = crate::rng::current();
    let kind = ctx
        .map
        .trap_kinds
        .get(&Map::idx(x, y))
        .copied()
        .unwrap_or(map::TrapKind::Dart);
    match kind {
        map::TrapKind::Dart => {
            let dmg = Dice {
                count: 1,
                sides: 4,
                bonus: ctx.ps.depth as i32 / 2,
            }
            .roll(&mut rng);
            ctx.log.add(format!("A dart hits you! ({})", dmg));
            ctx.ps.hp -= dmg;
        }
        map::TrapKind::PoisonDart => {
            let dmg = Dice {
                count: 1,
                sides: 4,
                bonus: 0,
            }
            .roll(&mut rng);
            ctx.ps.hp -= dmg;
            ctx.ps.poison += rng.gen_range(5..=15);
            ctx.log.add(format!("A poisoned dart hits you! ({})", dmg));
        }
        map::TrapKind::Pit => {
            if has_ff(ctx) {
                ctx.log
                    .add("You float gently into a pit and climb out unharmed.");
            } else {
                let dmg = Dice {
                    count: 2,
                    sides: 6,
                    bonus: 0,
                }
                .roll(&mut rng);
                ctx.log.add(format!("You fall into a pit! ({})", dmg));
                ctx.ps.hp -= dmg;
            }
        }
        map::TrapKind::SpikedPit => {
            if has_ff(ctx) {
                ctx.log.add("You float gently over the spikes.");
            } else {
                let dmg = Dice {
                    count: 4,
                    sides: 6,
                    bonus: 0,
                }
                .roll(&mut rng);
                ctx.ps.hp -= dmg;
                let k = rng.gen_range(10..=30);
                let v = ctx.ps.cut + k;
                crate::game::set_cut(&ctx.gd, &mut ctx.ps, v);
                ctx.log
                    .add(format!("You fall onto poisoned spikes! ({})", dmg));
            }
        }
        map::TrapKind::Teleport => {
            ctx.log.add("You are enveloped in a blinding flash!");
            let occupied: HashSet<(i32, i32)> =
                ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
            if let Ok(mut pos) = ctx.player.single_mut() {
                teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pos, &occupied, 100, &mut rng);
                ctx.ps.last_teleport = Some((pos.x, pos.y));
            }
        }
        map::TrapKind::Fire => {
            let dmg = Dice {
                count: 4,
                sides: 6,
                bonus: 0,
            }
            .roll(&mut rng);
            ctx.log
                .add(format!("You are enveloped in flames! ({})", dmg));
            ctx.ps.hp -= dmg;
        }
        map::TrapKind::TrapDoor => {
            if has_ff(ctx) {
                ctx.log.add("You float gently down through a trap door!");
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth + 1));
            } else {
                let dmg = Dice {
                    count: 2,
                    sides: 8,
                    bonus: 0,
                }
                .roll(&mut rng);
                ctx.ps.hp -= dmg;
                ctx.log
                    .add(format!("You fall through a trap door! ({})", dmg));
                if ctx.ps.hp > 0 {
                    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth + 1));
                }
            }
        }
        map::TrapKind::Summon => {
            ctx.log.add("A magic rune summons monsters!");
            if let Ok(pos) = ctx.player.single() {
                game::summon_monsters_any(
                    &mut ctx.commands,
                    &ctx.gd,
                    &ctx.tiles,
                    &ctx.map,
                    ctx.ps.dungeon,
                    ctx.ps.depth,
                    &pos,
                    2,
                    &mut rng,
                );
            }
        }
    }
    if ctx.ps.hp <= 0 && game::player_death_check(&mut ctx.ps, &ctx.gd, &mut ctx.log) {
        ctx.log.add("You die.");
        ctx.next.set(AppState::Dead);
    }
}

/// Feather fall / flight / climbing protects from falls.
fn has_ff(ctx: &InputCtx) -> bool {
    let t = ctx.inv.totals_for(&ctx.gd, &ctx.ps);
    t.feather
        || t.fly
        || t.climb
        || game::god_feather(&ctx.ps)
        || game::god_fly(&ctx.ps)
        || ctx.ps.tim_fly > 0
        || ctx.ps.tim_ffall > 0
}

/// Search the eight surrounding cells for secret doors and traps.
fn search(ctx: &mut InputCtx) {
    let Ok(pos) = ctx.player.single() else {
        return;
    };
    let chance =
        (40 + 10 * ctx.ps.stat_bonus(crate::game::WIS) + 2 * ctx.ps.level as i32).clamp(5, 95);
    let mut rng = crate::rng::current();
    let mut found = false;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (x, y) = (pos.x + dx, pos.y + dy);
            let t = ctx.map.terrain_at(x, y);
            let i = Map::idx(x, y);
            if t == map::T_SECRET_DOOR && rng.gen_range(0..100) < chance {
                ctx.map.set_terrain(x, y, map::T_DOOR);
                ctx.log.add("You have found a secret door!");
                found = true;
            } else if t == map::T_TRAP
                && !ctx.map.known_traps.contains(&i)
                && rng.gen_range(0..100) < chance
            {
                ctx.map.known_traps.insert(i);
                ctx.log.add("You have found a trap!");
                found = true;
            }
        }
    }
    if !found {
        ctx.log.add("You find nothing.");
    }
    consume_turn(ctx);
}

/// Try to disarm a known trap (Ctrl+direction onto it).
fn disarm_trap(ctx: &mut InputCtx, x: i32, y: i32) {
    let chance =
        (30 + 5 * ctx.ps.stat_bonus(crate::game::DEX) + 2 * ctx.ps.level as i32).clamp(5, 95);
    let mut rng = crate::rng::current();
    if rng.gen_range(0..100) < chance {
        ctx.map.set_terrain(x, y, T_FLOOR);
        ctx.map.known_traps.remove(&Map::idx(x, y));
        ctx.log.add("You disarm the trap.");
    } else {
        ctx.log.add("You failed to disarm the trap!");
        trigger_trap(ctx, x, y);
    }
    consume_turn(ctx);
}

/// easy_open_door (cmd1.cc:2538) / do_cmd_open_aux (cmd2.cc:668): open a
/// closed door, picking locked ones. Consumes the turn. Returns true when
/// a failed lockpick may be retried (`more`, cmd2.cc:747).
fn force_door(ctx: &mut InputCtx, x: i32, y: i32, t: u16) -> bool {
    let mut rng = crate::rng::current();
    consume_turn(ctx);
    // Jammed door.
    if t >= map::T_DOOR + 8 {
        ctx.log.add("The door appears to be stuck.");
        return false;
    }
    // Closed door.
    if t == map::T_DOOR {
        ctx.map.set_terrain(x, y, map::T_OPEN_DOOR);
        ctx.log.add("You open the door.");
        return false;
    }
    // Locked door: try to pick it.
    let mut i = 100;
    let (px, py) = ctx.player.single().map(|p| (p.x, p.y)).unwrap_or((x, y));
    if ctx.ps.blind > 0 || !ctx.map.visible[Map::idx(px, py)] {
        i /= 10;
    }
    if ctx.ps.confuse > 0 || ctx.ps.image > 0 {
        i /= 10;
    }
    let j = (i - map::door_power(t) * 4).max(2);
    if rng.gen_range(0..100) < j {
        ctx.map.set_terrain(x, y, map::T_OPEN_DOOR);
        ctx.log.add("You have picked the lock.");
        game::gain_exp(&mut ctx.ps, 1, &mut ctx.log, &mut rng);
        false
    } else {
        ctx.log.add("You failed to pick the lock.");
        // "We may keep trying" (cmd2.cc:747).
        true
    }
}

/// do_cmd_bash_aux (cmd2.cc:1445): force a closed door, falling through it.
fn bash_door(ctx: &mut InputCtx, x: i32, y: i32, t: u16, dx: i32, dy: i32) {
    let mut rng = crate::rng::current();
    ctx.log.add("You smash into the door!");
    let bash = game::adj_str_blow(ctx.ps.stats[crate::game::STR]);
    let temp = (bash - (((t - map::T_DOOR) & 0x07) as i32) * 10).max(1);
    if rng.gen_range(0..100) < temp {
        ctx.log.add("The door crashes open!");
        if rng.gen_range(0..100) < 50 {
            ctx.map.set_terrain(x, y, 5);
        } else {
            ctx.map.set_terrain(x, y, map::T_OPEN_DOOR);
        }
        step_player(ctx, dx, dy, false);
    } else if rng.gen_range(0..100)
        < game::adj_dex_safe(ctx.ps.stats[crate::game::DEX]) + ctx.ps.level as i32
    {
        ctx.log.add("The door holds firm.");
    } else {
        ctx.log.add("You are off-balance.");
        ctx.ps.paralyze += 2 + rng.gen_range(0..2);
    }
}

/// do_cmd_bash_fountain (cmd2.cc:81): break a fountain in a water blast.
fn bash_fountain(ctx: &mut InputCtx, x: i32, y: i32) {
    let mut rng = crate::rng::current();
    ctx.log.add("You smash into the fountain!");
    let bash = game::adj_str_blow(ctx.ps.stats[crate::game::STR]);
    let temp = (bash - 50).max(1);
    if rng.gen_range(0..200) < temp {
        ctx.log.add("The fountain breaks!");
        let dam = Dice {
            count: 6,
            sides: 8,
            bonus: 0,
        }
        .roll(&mut rng);
        let targets: Vec<(Entity, usize, i32, i32)> = ctx
            .monsters
            .iter()
            .filter(|(_, m, p)| m.hp > 0 && map::chebyshev(p.x, p.y, x, y) <= 2)
            .map(|(e, m, p)| (e, m.def, p.x, p.y))
            .collect();
        let level = ctx.ps.level as i32;
        let mut dead = Vec::new();
        for (e, def_idx, mx, my) in targets {
            let eff = game::gf_monster_effect(
                &ctx.gd.monsters[def_idx],
                "WATER",
                dam,
                level,
                true,
                &mut rng,
            );
            let (_, mut m, _) = ctx.monsters.get_mut(e).expect("monster");
            m.hp -= eff.dam;
            game::clamp_no_death(&ctx.gd, &mut m);
            m.awake = true;
            if m.hp <= 0 {
                dead.push((e, def_idx, (mx, my)));
            }
        }
        for (e, def_idx, (mx, my)) in dead {
            // A POSSESSOR soul deincarnates instead of dying.
            if let Ok((_, mut dm, _)) = ctx.monsters.get_mut(e) {
                let mut rng = crate::rng::current();
                if game::deincarnate_monster(&ctx.gd, &mut dm, None, &mut ctx.log, &mut rng) {
                    continue;
                }
            }
            let held = ctx
                .monsters
                .get(e)
                .map(|(_, m, _)| {
                    (
                        m.held_artifact,
                        m.held_object,
                        m.items.clone(),
                        m.gold,
                        m.looted,
                    )
                })
                .unwrap_or((None, None, Vec::new(), 0, false));
            let quest_left = ctx
                .monsters
                .iter()
                .filter(|(qe, m, _)| *qe != e && m.quest)
                .count() as i32;
            let enemy_left = ctx
                .monsters
                .iter()
                .filter(|(qe, m, _)| *qe != e && !m.friendly && !m.companion && !m.neutral)
                .count() as i32;
            game::kill_monster(
                &mut ctx.commands,
                &ctx.gd,
                &ctx.tiles,
                &mut ctx.stacks,
                e,
                def_idx,
                (mx, my),
                held.0,
                held.1,
                held.2,
                held.3,
                held.4,
                &mut ctx.ps,
                &mut ctx.plot,
                &mut ctx.map,
                quest_left,
                enemy_left,
                &mut ctx.created.0,
                &mut ctx.inv,
                &mut ctx.log,
                &mut rng,
                false,
            );
        }
        ctx.map.set_terrain(x, y, map::T_DEEP_WATER);
    }
}

/// The spell-name list of a book item (its inscribed spells for random
/// books, otherwise the fixed sval list; spells4.cc init_school_books).
pub(crate) fn book_item_spells<'a>(
    gd: &'a GameData,
    it: &'a item::Item,
) -> Vec<&'a str> {
    if !it.spells.is_empty() {
        it.spells.iter().map(|s| s.as_str()).collect()
    } else {
        crate::spell::book_spells(gd.objects[it.def].sval)
            .map(|l| l.to_vec())
            .unwrap_or_default()
    }
}

/// udun_in_book (spells3.cc:2703): the number of Udun/Melkor spells.
pub(crate) fn udun_in_book(gd: &GameData, it: &item::Item) -> i32 {
    book_item_spells(gd, it)
        .iter()
        .filter(|n| {
            gd.spells
                .iter()
                .any(|s| s.name == **n && (s.school == 55 || s.god == "Melkor"))
        })
        .count() as i32
}

/// levels_in_book (spells3.cc:2728): total skill levels of the book's
/// spells (the port's RON `level` doubles as the school level).
fn levels_in_book(gd: &GameData, it: &item::Item) -> i32 {
    book_item_spells(gd, it)
        .iter()
        .filter_map(|n| gd.spells.iter().find(|s| s.name == *n))
        .map(|s| s.level as i32)
        .sum::<i32>()
        .max(1)
}

/// item_tester_hook_sacrificable (cmd2.cc:3698): Melkor takes corpses
/// (not skeletons/meat) and books without Udun spells.
pub(crate) fn sacrificable(gd: &GameData, god: u32, it: &item::Item) -> bool {
    if god != 4 {
        return false;
    }
    let o = &gd.objects[it.def];
    if o.tval == data::TV_CORPSE && o.sval == 1 {
        return true;
    }
    if o.tval == data::TV_BOOK && udun_in_book(gd, it) <= 0 {
        return true;
    }
    false
}

/// The piety a Melkor sacrifice yields (cmd2.cc:3835-3847): 2*monster
/// level for corpses, 2*levels_in_book for books.
pub(crate) fn sacrifice_grace(gd: &GameData, it: &item::Item) -> i32 {
    let o = &gd.objects[it.def];
    if o.tval == data::TV_CORPSE {
        let lvl = gd
            .monsters
            .get(it.note as usize)
            .map(|m| m.depth.max(1) as i32)
            .unwrap_or(1);
        2 * lvl
    } else {
        2 * levels_in_book(gd, it)
    }
}

/// do_cmd_sacrifice (cmd2.cc:3758): altar sacrifices. The port's original
/// 'O' key is the pet menu, so this lives on Ctrl+O.
fn do_cmd_sacrifice(ctx: &mut InputCtx) {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return;
    };
    let t = ctx.map.terrain_at(pos.x, pos.y);
    if !(161..=171).contains(&t) {
        if ctx.ps.god == 0 {
            ctx.log.add("You worship no god.");
        } else {
            ctx.log.add(format!(
                "You pray to {}. (Grace: {})",
                crate::spell::god_name(ctx.ps.god),
                ctx.ps.grace
            ));
        }
        return;
    }
    let agod = (t - 161 + 1) as u32;
    if ctx.ps.god == 0 {
        // The original prints the deity's description lines then asks
        // "Do you want to worship X?" (cmd2.cc:3775).
        *ctx.modal = Modal::AltarWorship { god: agod };
        return;
    }
    if ctx.ps.god != agod {
        ctx.log.add("You do not worship this god.");
        return;
    }
    if agod == 6 {
        // Aule: self-made items are sacrificed for value/10 piety
        // (cmd2.cc:3729 do_cmd_sacrifice_aule).
        if let Some(i) = ctx
            .inv
            .pack
            .iter()
            .position(item::item_tester_hook_sacrifice_aule)
        {
            let it = ctx.inv.pack[i].clone();
            let piety = it.cost(&ctx.gd) / 10;
            ctx.inv.pack.remove(i);
            ctx.ps.grace = (ctx.ps.grace + piety).clamp(-300000, 300000);
            ctx.log.add("Aule accepts your offering.");
            consume_turn(ctx);
            return;
        }
        ctx.log.add("You have nothing Aule would accept.");
        return;
    }
    if agod != 4 {
        ctx.log.add("Your god does not want a sacrifice right now.");
        return;
    }
    // Melkor: the healthy may give of themselves first (cmd2.cc:3789).
    if ctx.ps.hp > 10 && ctx.ps.max_hp > 10 {
        *ctx.modal = Modal::MelkorSacrifice { stage: 0 };
        return;
    }
    if let Some(i) = ctx
        .inv
        .pack
        .iter()
        .position(|it| sacrificable(&ctx.gd, ctx.ps.god, it))
    {
        let it = ctx.inv.pack[i].clone();
        let piety = sacrifice_grace(&ctx.gd, &it);
        ctx.inv.pack.remove(i);
        ctx.ps.grace = (ctx.ps.grace + piety).clamp(-300000, 300000);
        ctx.log.add("Melkor accepts your offering.");
        consume_turn(ctx);
        return;
    }
    ctx.log.add("You have nothing to sacrifice.");
}

/// do_cmd_bash (cmd2.cc:1552): bash a door, altar or fountain.
fn bash_at(ctx: &mut InputCtx, x: i32, y: i32, dx: i32, dy: i32) {
    let t = ctx.map.terrain_at(x, y);
    let bashable_door = map::is_closed_door(t) && t != map::T_SECRET_DOOR;
    let altar = (161..=171).contains(&t);
    if !bashable_door && !altar && t != 2 {
        ctx.log.add("You see nothing there to bash.");
        return;
    }
    if ctx
        .monsters
        .iter()
        .any(|(_, m, p)| p.x == x && p.y == y && m.hp > 0)
    {
        let e = ctx
            .monsters
            .iter()
            .find(|(_, _, p)| p.x == x && p.y == y)
            .map(|(e, _, _)| e);
        if let Some(e) = e {
            consume_turn(ctx);
            ctx.log.add("There is a monster in the way!");
            if !melee_attack(ctx, e) {
                consume_turn(ctx);
            }
        }
        return;
    }
    if altar {
        ctx.log.add("Are you mad? You want to anger the gods?");
        return;
    }
    if t == 2 {
        consume_turn(ctx);
        bash_fountain(ctx, x, y);
        return;
    }
    consume_turn(ctx);
    bash_door(ctx, x, y, t, dx, dy);
}

/// count_feats (cmd2.cc:589): count known matching adjacent grids and
/// remember the only match's position.
fn count_feats(
    ctx: &InputCtx,
    x: i32,
    y: i32,
    test: fn(&GameData, &Map, u16) -> bool,
    under: bool,
) -> (i32, Option<(i32, i32)>) {
    let mut count = 0;
    let mut last = None;
    for d in 0..9 {
        if d == 8 && !under {
            continue;
        }
        let (dx, dy) = DDD[d];
        let (nx, ny) = (x + dx, y + dy);
        if !Map::in_bounds(nx, ny) {
            continue;
        }
        let idx = Map::idx(nx, ny);
        if !ctx.map.explored[idx] {
            continue;
        }
        if !test(&ctx.gd, &ctx.map, ctx.map.terrain_at(nx, ny)) {
            continue;
        }
        count += 1;
        last = Some((nx, ny));
    }
    (count, last)
}

/// coords_to_dir (cmd2.cc:638).
fn coords_to_dir(px: i32, py: i32, x: i32, y: i32) -> i32 {
    let (dx, dy) = (x - px, y - py);
    match (dx, dy) {
        (-1, -1) => 7,
        (0, -1) => 8,
        (1, -1) => 9,
        (-1, 0) => 4,
        (1, 0) => 6,
        (-1, 1) => 1,
        (0, 1) => 2,
        (1, 1) => 3,
        _ => 0,
    }
}

fn is_known_open_door(_gd: &GameData, _map: &Map, t: u16) -> bool {
    t == map::T_OPEN_DOOR
}

/// is_closed (cmd2.cc:575): the closed/locked/jammed door range
/// FEAT_DOOR_HEAD..FEAT_DOOR_TAIL (secret doors are excluded).
fn is_closed(_gd: &GameData, _map: &Map, t: u16) -> bool {
    map::is_closed_door(t) && t != map::T_SECRET_DOOR
}

/// skill_dig (xtra1.cc calc_bonuses): TUNNEL gear, tool weight and STR.
fn skill_dig(gd: &GameData, ps: &PlayerState, inv: &Inventory) -> i32 {
    // TR_TUNNEL already contributes pval*20 in EquipTotals (xtra1.cc:2498).
    let mut s = inv.totals_for(gd, ps).tunnel;
    if let Some(tool) = &inv.equip[data::SLOT_TOOL] {
        if gd.objects[tool.def].tval == data::TV_DIGGING {
            s += gd.objects[tool.def].weight / 10;
        }
    }
    s += game::adj_str_dig(ps.stats[crate::game::STR]);
    s.max(1)
}

/// twall (cmd2.cc:1070): replace a wall, forgetting it first.
fn twall(ctx: &mut InputCtx, x: i32, y: i32, feat: u16) -> bool {
    if ctx.gd.terrain(ctx.map.terrain_at(x, y)).is_floor {
        return false;
    }
    let idx = Map::idx(x, y);
    ctx.map.explored[idx] = false;
    ctx.map.set_terrain(x, y, feat);
    ctx.turn.fov_dirty = true;
    true
}

/// do_cmd_tunnel_aux (cmd2.cc:1103): dig through the target grid.
/// Returns true when digging may continue.
fn tunnel_aux(ctx: &mut InputCtx, x: i32, y: i32) -> bool {
    let mut rng = crate::rng::current();
    let t = ctx.map.terrain_at(x, y);
    let (def_is_floor, def_tunnelable, def_permanent, def_tunnel) = {
        let def = ctx.gd.terrain(t);
        (
            def.is_floor,
            def.tunnelable,
            def.permanent,
            def.tunnel_desc.clone(),
        )
    };
    // f_info D:1 ("You tunnel into the granite wall."); the original falls
    // back to "You cannot tunnel through that." for features without one.
    let tunnel_msg = if def_tunnel.is_empty() {
        "You cannot tunnel through that.".to_string()
    } else {
        def_tunnel
    };
    // A digging tool in the tool slot is required, except for sandwalls.
    if !(98..=100).contains(&t) {
        let has_tool = ctx.inv.equip[data::SLOT_TOOL]
            .as_ref()
            .is_some_and(|it| ctx.gd.objects[it.def].tval == data::TV_DIGGING);
        if !has_tool {
            ctx.log
                .add("You need to have a shovel or pick in your tool slot.");
            return false;
        }
    }
    // do_cmd_tunnel_test.
    let idx = Map::idx(x, y);
    if !ctx.map.explored[idx] {
        ctx.log.add("You see nothing there.");
        return false;
    }
    if def_is_floor {
        ctx.log.add("You see nothing there to tunnel.");
        return false;
    }
    if !def_tunnelable {
        ctx.log.add("You cannot tunnel through that.");
        return false;
    }
    // Take a turn.
    consume_turn(ctx);
    let sd = skill_dig(&ctx.gd, &ctx.ps, &ctx.inv);
    let mut more = false;
    let mut skill_req = 0;
    let mut skill_req_1pct = 0;
    if def_permanent {
        ctx.log.add(tunnel_msg.clone());
    } else if t == map::T_TREE || t == 92 {
        skill_req = 10;
        skill_req_1pct = 14;
        if sd > 10 + rng.gen_range(0..400) && twall(ctx, x, y, map::T_GRASS) {
            ctx.log.add("You have cleared away the trees.");
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    } else if (56..=59).contains(&t) {
        skill_req = 40;
        skill_req_1pct = 56;
        if sd > 40 + rng.gen_range(0..1600) && twall(ctx, x, y, T_FLOOR) {
            ctx.log.add("You have finished the tunnel.");
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    } else if (50..=55).contains(&t) || (98..=100).contains(&t) {
        let gold = (52..=55).contains(&t) || t == 99 || t == 100;
        let soft = t == 99 || t == 100;
        let hard = !soft && (t - 50) & 0x01 == 1;
        let (req, req1, okay) = if hard {
            (20, 28, sd > 20 + rng.gen_range(0..800))
        } else if soft {
            (5, 8, sd > 5 + rng.gen_range(0..250))
        } else {
            (10, 14, sd > 10 + rng.gen_range(0..400))
        };
        skill_req = req;
        skill_req_1pct = req1;
        if okay && twall(ctx, x, y, T_FLOOR) {
            if gold {
                let amount = ctx.ps.depth as i32 * 20 + rng.gen_range(1..=100);
                item::place_gold(&mut ctx.commands, &ctx.tiles, x, y, amount);
                ctx.log.add("You have found something!");
            } else {
                ctx.log.add("You have finished the tunnel.");
            }
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    } else if t == map::T_RUBBLE {
        skill_req = 0;
        skill_req_1pct = 2;
        // Rubble leaves the dungeon's own floor behind (cmd2.cc:1271,
        // d_info[dungeon_type].floor1; q_god swaps it at runtime).
        let floor1 = if ctx.ps.depth == 0 || ctx.ps.dungeon == 0 {
            T_FLOOR
        } else {
            ctx.gd
                .dungeon(ctx.ps.dungeon)
                .floors
                .first()
                .map(|f| f.feat)
                .unwrap_or(T_FLOOR)
        };
        if sd > rng.gen_range(0..200) && twall(ctx, x, y, floor1) {
            ctx.log.add("You have removed the rubble.");
            if rng.gen_range(0..100) < 10 {
                let candidates: Vec<usize> = (0..ctx.gd.objects.len())
                    .filter(|&i| {
                        let o = &ctx.gd.objects[i];
                        o.depth <= ctx.ps.depth.max(1) && !data::is_ammo(o.tval)
                    })
                    .collect();
                if !candidates.is_empty() {
                    let def = candidates[rng.gen_range(0..candidates.len())];
                    let it = item::make_item(
                        &ctx.gd,
                        def,
                        ctx.ps.depth,
                        false,
                        &mut ctx.created.0,
                        &mut rng,
                    );
                    item::place_floor_item(
                        &mut ctx.commands,
                        &ctx.gd,
                        &ctx.tiles,
                        &mut ctx.stacks,
                        x,
                        y,
                        it,
                    );
                    if ctx.map.visible[Map::idx(x, y)] {
                        ctx.log.add("You have found something!");
                    }
                }
            }
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    } else if t == map::T_SECRET_DOOR {
        skill_req = 30;
        skill_req_1pct = 42;
        if sd > 30 + rng.gen_range(0..1200) && twall(ctx, x, y, T_FLOOR) {
            ctx.log.add("You have finished the tunnel.");
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    } else {
        // Doors and anything else tunnelable.
        skill_req = 30;
        skill_req_1pct = 42;
        if sd > 30 + rng.gen_range(0..1200) && twall(ctx, x, y, T_FLOOR) {
            ctx.log.add("You have finished the tunnel.");
        } else {
            ctx.log.add(tunnel_msg.clone());
            more = true;
        }
    }
    if more && rng.gen_range(0..50) == 0 {
        if sd < skill_req {
            ctx.log
                .add("You fail to make even the slightest of progress.");
            more = false;
        } else if sd < skill_req_1pct {
            ctx.log.add("This will take some time.");
        }
    }
    more
}

/// Resolve a pending directional command (cmd2.cc do_cmd_alter /
/// do_cmd_close / do_cmd_spike / do_cmd_give).
fn command_direction(ctx: &mut InputCtx, kind: &str, arg: usize, dx: i32, dy: i32) -> bool {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return false;
    };
    let (nx, ny) = (pos.x + dx, pos.y + dy);
    if !Map::in_bounds(nx, ny) {
        return false;
    }
    match kind {
        "alter" => {
            // Monsters are attacked (do_cmd_alter).
            let target = ctx
                .monsters
                .iter()
                .find(|(_, _, p)| p.x == nx && p.y == ny)
                .map(|(e, _, _)| e);
            if let Some(e) = target {
                if ctx.ps.fear > 0 {
                    ctx.log.add("You are too afraid to attack!");
                    return false;
                }
                if let Some((pdef, _, _)) = ctx.ps.possessed {
                    melee_possessed(ctx, e, pdef);
                    consume_turn(ctx);
                    return false;
                }
                if !melee_attack(ctx, e) {
                    consume_turn(ctx);
                }
                return false;
            }
            let t = ctx.map.terrain_at(nx, ny);
            if t == map::T_TRAP {
                if ctx.map.known_traps.contains(&Map::idx(nx, ny)) {
                    disarm_trap(ctx, nx, ny);
                    return false;
                }
            }
            if map::is_closed_door(t) && t != map::T_SECRET_DOOR {
                // do_cmd_open_aux: a failed lockpick may be retried.
                return force_door(ctx, nx, ny, t);
            }
            let def = ctx.gd.terrain(t);
            let diggable = def.tunnelable && !def.permanent;
            if diggable {
                // do_cmd_tunnel_aux returns `more` while digging.
                return tunnel_aux(ctx, nx, ny);
            } else {
                consume_turn(ctx);
                ctx.log.add("You attack the empty air.");
                return false;
            }
        }
        "open_door" => {
            // do_cmd_open: a monster in the way is attacked, otherwise
            // only a closed/locked/jammed door can be opened.
            if ctx.monsters.iter().any(|(_, _, p)| p.x == nx && p.y == ny) {
                consume_turn(ctx);
                ctx.log.add("There is a monster in the way!");
                let e = ctx
                    .monsters
                    .iter()
                    .find(|(_, _, p)| p.x == nx && p.y == ny)
                    .map(|(e, _, _)| e);
                if let Some(e) = e {
                    melee_attack(ctx, e);
                }
                return false;
            }
            let t = ctx.map.terrain_at(nx, ny);
            if !map::is_closed_door(t) || t == map::T_SECRET_DOOR {
                ctx.log.add("You see nothing there to open.");
                return false;
            }
            force_door(ctx, nx, ny, t)
        }
        "close_door" => {
            if ctx.monsters.iter().any(|(_, _, p)| p.x == nx && p.y == ny) {
                consume_turn(ctx);
                ctx.log.add("There is a monster in the way!");
                let e = ctx
                    .monsters
                    .iter()
                    .find(|(_, _, p)| p.x == nx && p.y == ny)
                    .map(|(e, _, _)| e);
                if let Some(e) = e {
                    melee_attack(ctx, e);
                }
                return false;
            }
            let t = ctx.map.terrain_at(nx, ny);
            if t == 5 {
                consume_turn(ctx);
                ctx.log.add("The door appears to be broken.");
            } else if t == map::T_OPEN_DOOR {
                ctx.map.set_terrain(nx, ny, map::T_DOOR);
                ctx.log.add("You close the door.");
                consume_turn(ctx);
            } else if map::is_door(t) {
                ctx.log.add("The door is already closed.");
            } else {
                ctx.log.add("You see nothing there to close.");
            }
            false
        }
        "bash" => {
            bash_at(ctx, nx, ny, dx, dy);
            false
        }
        "spike" => {
            let t = ctx.map.terrain_at(nx, ny);
            if !map::is_closed_door(t) || t == map::T_SECRET_DOOR {
                ctx.log.add("You see nothing there to spike.");
                return false;
            }
            if ctx.monsters.iter().any(|(_, _, p)| p.x == nx && p.y == ny) {
                consume_turn(ctx);
                ctx.log.add("There is a monster in the way!");
                let e = ctx
                    .monsters
                    .iter()
                    .find(|(_, _, p)| p.x == nx && p.y == ny)
                    .map(|(e, _, _)| e);
                if let Some(e) = e {
                    melee_attack(ctx, e);
                }
                return false;
            }
            let Some(si) = ctx
                .inv
                .pack
                .iter()
                .position(|it| ctx.gd.objects[it.def].tval == data::TV_SPIKE)
            else {
                ctx.log.add("You have no spikes!");
                return false;
            };
            let mut nt = t;
            if nt < 40 {
                nt += 8;
            }
            if nt < 47 {
                nt += 1;
            }
            ctx.map.set_terrain(nx, ny, nt);
            ctx.inv.pack[si].count -= 1;
            if ctx.inv.pack[si].count == 0 {
                ctx.inv.pack.remove(si);
            }
            ctx.log.add("You jam the door with a spike.");
            consume_turn(ctx);
            false
        }
        // Repeats of a numeric-prefixed walk (allow_repeat_command).
        "walk" => {
            step_player_pickup(ctx, dx, dy, true, true);
            true
        }
        "walk_nopick" => {
            step_player_pickup(ctx, dx, dy, true, false);
            true
        }
        "give" => {
            if arg >= ctx.inv.pack.len() {
                return false;
            }
            let target = ctx
                .monsters
                .iter()
                .find(|(_, _, p)| p.x == nx && p.y == ny)
                .map(|(e, _, _)| e);
            let Some(e) = target else {
                ctx.log.add("There is nothing there to give it to.");
                return false;
            };
            let it = ctx.inv.pack.remove(arg);
            let label = it.label(&ctx.gd, &ctx.inv.known);
            let name = {
                let (_, m, _) = ctx.monsters.get(e).expect("monster");
                ctx.gd.monsters[m.def].name.clone()
            };
            // Quest give hooks (q_hobbit.cc / q_shroom.cc HOOK_GIVE).
            if quest_give_hook(ctx, e, &it) {
                consume_turn(ctx);
                return false;
            }
            // drunk_takes_wine (modules.cc HOOK_GIVE): the happy drunk
            // quaffs Ale/Wine (TV_FOOD sval 38/39) and leaves a bottle.
            let o = &ctx.gd.objects[it.def];
            if name == "Singing, happy drunk"
                && o.tval == data::TV_FOOD
                && (o.sval == 38 || o.sval == 39)
            {
                ctx.log.add("'Hic!'");
                if let Some(bottle) = ctx.gd.object_by_tval_sval(data::TV_BOTTLE, 1) {
                    let empty = item::Item::base(&ctx.gd, bottle);
                    let (px, py) = player_pos(ctx).unwrap_or((0, 0));
                    item::place_floor_item(
                        &mut ctx.commands,
                        &ctx.gd,
                        &ctx.tiles,
                        &mut ctx.stacks,
                        px,
                        py,
                        empty,
                    );
                }
                consume_turn(ctx);
                return false;
            }
            // hobbit_food (modules.cc HOOK_GIVE): the scruffy hobbit eats
            // any food.
            if name == "Scruffy-looking hobbit" && o.tval == data::TV_FOOD {
                ctx.log.add("'Yum!'");
                consume_turn(ctx);
                return false;
            }
            // smeagol_ring (modules.cc HOOK_GIVE): Smeagol keeps rings.
            if name == "Smeagol" && o.tval == data::TV_RING {
                ctx.log.add("'MY... PRECIOUSSSSS!!!'");
                consume_turn(ctx);
                return false;
            }
            let (_, mut m, _) = ctx.monsters.get_mut(e).expect("monster");
            m.items.push(it);
            ctx.log.add(format!("You give {} to the {}.", label, name));
            consume_turn(ctx);
            false
        }
        "panic_hit" => {
            // PWR_PANIC_HIT: strike the adjacent foe then teleport away.
            // The power itself already spent the turn.
            let target = ctx
                .monsters
                .iter()
                .find(|(_, _, p)| p.x == nx && p.y == ny)
                .map(|(e, _, _)| e);
            let Some(e) = target else {
                ctx.log.add("You don't see any monster in this direction.");
                return false;
            };
            melee_attack(ctx, e);
            if ctx.inv.totals_for(&ctx.gd, &ctx.ps).no_tele {
                ctx.log.add("You cannot teleport!");
                return false;
            }
            let mut rng = crate::rng::current();
            let occupied: HashSet<(i32, i32)> =
                ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
            if let Ok(mut pp) = ctx.player.single_mut() {
                teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pp, &occupied, 30, &mut rng);
                ctx.ps.last_teleport = Some((pp.x, pp.y));
            }
            ctx.turn.fov_dirty = true;
            false
        }
        _ => false,
    }
}

/// Walk off the edge of a depth-0 wilderness area; true when the player
/// is moved to the adjacent world cell (cmd1.cc move_player_aux).
fn try_wild_edge(ctx: &mut InputCtx, nx: i32, ny: i32) -> bool {
    if ctx.ps.depth != 0 || ctx.ps.wild_mode {
        return false;
    }
    if nx != 0 && nx != map::MAP_W - 1 && ny != 0 && ny != map::MAP_H - 1 {
        return false;
    }
    if !player_can_walk(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, nx, ny) {
        return false;
    }
    let (mut wx, mut wy) = (ctx.ps.wild_x, ctx.ps.wild_y);
    let mut entry = (nx, ny);
    if nx == 0 {
        wx -= 1;
        entry.0 = map::MAP_W - 2;
    } else if nx == map::MAP_W - 1 {
        wx += 1;
        entry.0 = 1;
    }
    if ny == 0 {
        wy -= 1;
        entry.1 = map::MAP_H - 2;
    } else if ny == map::MAP_H - 1 {
        wy += 1;
        entry.1 = 1;
    }
    if !ctx.store.wilderness.in_bounds(wx, wy) {
        return false;
    }
    ctx.ps.wild_x = wx;
    ctx.ps.wild_y = wy;
    ctx.turn.ambush = false;
    ctx.turn.pending_pos = Some(entry);
    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Wild(wx, wy));
    true
}

/// item::teleport_player for InputCtx: the SF_TPORT drag cannot move the
/// read-only monster grid positions directly, so item::plan_tport_drags
/// computes the moves and they are applied through Commands.
#[allow(clippy::too_many_arguments)]
fn teleport_player_drag(
    gd: &crate::data::GameData,
    map: &crate::map::Map,
    monsters: &Query<(Entity, &mut Monster, &GridPos), Without<Player>>,
    commands: &mut Commands,
    pos: &mut GridPos,
    occupied: &HashSet<(i32, i32)>,
    range: i32,
    rng: &mut impl Rng,
) {
    let old = (pos.x, pos.y);
    item::teleport(gd, map, pos, occupied, range, rng);
    let new = (pos.x, pos.y);
    if old == new {
        return;
    }
    let snap: Vec<item::TportSnap> = monsters
        .iter()
        .map(|(e, m, p)| {
            let d = &gd.monsters[m.def];
            item::TportSnap {
                entity: e,
                x: p.x,
                y: p.y,
                level: game::monster_level(gd, m),
                tport: d.spells.iter().any(|s| s == "TPORT"),
                res_tele: d.has("RES_TELE"),
                awake: m.awake,
            }
        })
        .collect();
    let mut occ = occupied.clone();
    for s in &snap {
        occ.insert((s.x, s.y));
    }
    for (e, x, y) in item::plan_tport_drags(gd, map, old, new, &snap, &occ, rng) {
        commands.entity(e).insert(GridPos { x, y });
    }
}

/// The inner pick of `teleport_player_directed` (spells1.cc:184-208): on
/// the major axis the candidate is centred `dis/2` grids ahead with a
/// `dis/2` spread; the other axis uses a `dis/3` spread around the
/// player.  Re-picked until the Chebyshev distance is in [min, dis].
fn directed_candidate(
    px: i32,
    py: i32,
    dx: i32,
    dy: i32,
    min: i32,
    dis: i32,
    rng: &mut impl Rng,
) -> (i32, i32) {
    let x_major = dy == 0;
    let y_major = dx == 0;
    let x_neg = if dx < 0 { -1 } else { 1 };
    let y_neg = if dy < 0 { -1 } else { 1 };
    loop {
        let x = if x_major {
            crate::rng::rand_spread(px + x_neg * dis / 2, dis / 2, rng)
        } else {
            crate::rng::rand_spread(px, dis / 3, rng)
        };
        let y = if y_major {
            crate::rng::rand_spread(py + y_neg * dis / 2, dis / 2, rng)
        } else {
            crate::rng::rand_spread(py, dis / 3, rng)
        };
        let d = map::chebyshev(px, py, x, y);
        if d >= min && d <= dis {
            return (x, y);
        }
    }
}

/// teleport_player_directed (spells1.cc:139): phase up to `rad` cells in a
/// direction (do_cmd_unwalk for immovable characters).
fn teleport_directed(ctx: &mut InputCtx, dx: i32, dy: i32, rad: i32) {
    let mut rng = crate::rng::current();
    let Ok(mut pos) = ctx.player.single_mut() else {
        return;
    };
    let (px, py) = (pos.x, pos.y);
    // No direction: an ordinary teleport (spells1.cc:156).
    if dx == 0 && dy == 0 {
        let occupied: HashSet<(i32, i32)> =
            ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
        teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pos, &occupied, rad, &mut rng);
        ctx.ps.last_teleport = Some((pos.x, pos.y));
        ctx.turn.fov_dirty = true;
        return;
    }
    // Rooted means no move (spells1.cc:162).
    if ctx.ps.tim_roots > 0 {
        return;
    }
    let occupied: HashSet<(i32, i32)> = ctx
        .monsters
        .iter()
        .filter(|(_, m, _)| m.hp > 0)
        .map(|(_, _, p)| (p.x, p.y))
        .collect();
    let mut min = rad / 4;
    let mut dis = rad;
    let mut chosen: Option<(i32, i32)> = None;
    loop {
        // Verify max distance (spells1.cc:170): give up and teleport.
        if dis > 200 {
            let occupied: HashSet<(i32, i32)> =
                ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
            teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pos, &occupied, rad, &mut rng);
            ctx.ps.last_teleport = Some((pos.x, pos.y));
            ctx.turn.fov_dirty = true;
            return;
        }
        for _ in 0..500 {
            let (x, y) = directed_candidate(px, py, dx, dy, min, dis, &mut rng);
            if !Map::in_bounds(x, y) {
                continue;
            }
            // Require "naked" floor space (cave_empty_bold).
            if !ctx.map.walkable(&ctx.gd, x, y) || occupied.contains(&(x, y)) {
                continue;
            }
            chosen = Some((x, y));
            break;
        }
        if chosen.is_some() {
            break;
        }
        // Increase the maximum distance, decrease the minimum.
        dis *= 2;
        min /= 2;
    }
    let (x, y) = chosen.expect("the loop only exits after choosing");
    pos.x = x;
    pos.y = y;
    ctx.ps.last_teleport = Some((x, y));
    ctx.turn.fov_dirty = true;
    cell_arrival(ctx, x, y);
}

/// do_cmd_unwalk (cmd2.cc:1868): an immovable character's "step"; attacks
/// adjacent monsters, enters stairs/quests and phases through the rest.
fn unwalk(ctx: &mut InputCtx, dx: i32, dy: i32) {
    let Ok(pos) = ctx.player.single().map(|p| *p) else {
        return;
    };
    let (nx, ny) = (pos.x + dx, pos.y + dy);
    if !Map::in_bounds(nx, ny) {
        return;
    }
    consume_turn(ctx);
    // Attack monsters.
    if let Some((e, friendly)) = ctx
        .monsters
        .iter()
        .find(|(_, m, p)| p.x == nx && p.y == ny && m.hp > 0)
        .map(|(e, m, _)| (e, m.friendly))
    {
        if friendly {
            npc_chat(ctx, e);
            return;
        }
        if ctx.ps.fear > 0 {
            ctx.log.add("You are too afraid to attack!");
            return;
        }
        if let Some((pdef, _, _)) = ctx.ps.possessed {
            melee_possessed(ctx, e, pdef);
            return;
        }
        melee_attack(ctx, e);
        return;
    }
    // Exit the area at the map edge.
    if try_wild_edge(ctx, nx, ny) {
        return;
    }
    let t = ctx.map.terrain_at(nx, ny);
    if !ctx.gd.terrain(t).is_floor {
        teleport_directed(ctx, dx, dy, 10);
        return;
    }
    // Stairs and quest entrances are entered normally.
    if (6..=11).contains(&t) {
        step_player(ctx, dx, dy, false);
        return;
    }
    if ctx.ps.wild_mode {
        let mut rng = crate::rng::current();
        let (dx, dy) = if rng.gen_range(0..100) < 15 {
            let mut d = (dx, dy);
            while d == (dx, dy) || d == (0, 0) {
                d = (rng.gen_range(-1..=1), rng.gen_range(-1..=1));
            }
            d
        } else {
            (dx, dy)
        };
        step_player(ctx, dx, dy, false);
        return;
    }
    // Walking semantics: phase up to ten grids in the direction.
    teleport_directed(ctx, dx, dy, 10);
}

/// Move one step, or bump-attack a monster standing in the way.
fn step_player(ctx: &mut InputCtx, dx: i32, dy: i32, verbose: bool) {
    let pickup = ctx.options.always_pickup;
    step_player_pickup(ctx, dx, dy, verbose, pickup);
}

/// step_player with an explicit carry() flag: '-' walks with
/// `!always_pickup` (cmd2.cc do_cmd_walk / dungeon.cc:2943).
fn step_player_pickup(ctx: &mut InputCtx, mut dx: i32, mut dy: i32, verbose: bool, pickup: bool) {
    // The world overview steps between wilderness cells instead.
    if ctx.ps.wild_mode {
        wild_step(ctx, dx, dy);
        return;
    }
    let Ok(player) = ctx.player.single().map(|p| *p) else {
        return;
    };
    let (px, py) = (player.x, player.y);
    // Possessed bodies may refuse to go where the player wants
    // (cmd1.cc move_player_aux RF_RAND_25/RAND_50).
    let mut rng = crate::rng::current();
    if let Some((pdef, _, _)) = ctx.ps.possessed {
        let chance = {
            let m = &ctx.gd.monsters[pdef];
            match (m.has("RAND_25"), m.has("RAND_50")) {
                (true, true) => 75,
                (_, true) => 50,
                (true, _) => 25,
                _ => 0,
            }
        };
        if chance > 0 && rng.gen_range(0..100) < chance {
            let d = loop {
                let d = (rng.gen_range(-1..=1), rng.gen_range(-1..=1));
                if d != (0, 0) {
                    break d;
                }
            };
            dx = d.0;
            dy = d.1;
        }
    }
    // Ice is slippery (cmd1.cc move_player_aux).
    if ctx.map.terrain_at(px, py) == map::T_ICE
        && !run_levitates(&ctx.gd, &ctx.ps, &ctx.inv)
        && rng.gen_range(0..100) < (70 - ctx.ps.level as i32).max(0)
    {
        let d = loop {
            let d = (rng.gen_range(-1..=1), rng.gen_range(-1..=1));
            if d != (0, 0) {
                break d;
            }
        };
        dx = d.0;
        dy = d.1;
        ctx.log.add("You slip on the icy floor.");
    }
    let (nx, ny) = (px + dx, py + dy);

    // Attack a monster in the target cell; friendly NPCs chat or are
    // pushed past (cmd1.cc move_player_aux:2830-2872).
    let mut target = None;
    for (e, m, mp) in ctx.monsters.iter() {
        if mp.x == nx && mp.y == ny {
            target = Some((e, m.friendly));
            break;
        }
    }
    if let Some((e, friendly)) = target {
        if friendly {
            // Only visible, unwounded attention lets you sidestep a friend.
            let totals = ctx.inv.totals_for(&ctx.gd, &ctx.ps);
            let (def_idx, m_visible) = {
                let (_, m, _) = ctx.monsters.get(e).expect("monster");
                (
                    m.def,
                    ctx.map.visible[Map::idx(nx, ny)]
                        && (!ctx.gd.monsters[m.def].has("INVISIBLE")
                            || totals.see_invis
                            || ctx.ps.tim_invis > 0),
                )
            };
            let clear = m_visible
                && ctx.ps.confuse == 0
                && ctx.ps.stun == 0
                && ctx.ps.image == 0
                // The original also requires the player to be able to
                // enter the friend's grid (cmd1.cc:2834).
                && player_can_walk_t(
                    &ctx.map,
                    &ctx.gd,
                    &ctx.ps,
                    &ctx.inv,
                    nx,
                    ny,
                    ctx.map.terrain_at(nx, ny),
                );
            if clear {
                let name = ctx.gd.monsters[def_idx].name.clone();
                // Stormbringer hungers (33.4%); otherwise it is chat or a
                // shove past (floor under the player or a pass-wall ally).
                if weapon_is_stormbringer(&ctx.inv) && rng.gen_range(0..1000) > 666 {
                    ctx.log
                        .add(format!("Your black blade greedily attacks {}!", name));
                    if let Some((pdef, _, _)) = ctx.ps.possessed {
                        melee_possessed(ctx, e, pdef);
                    } else {
                        melee_attack(ctx, e);
                    }
                    // The original keeps moving after the attack.
                } else {
                    let player_floor = ctx.gd.terrain(ctx.map.terrain_at(px, py)).is_floor;
                    let pass_wall = ctx.gd.monsters[def_idx].has("PASS_WALL");
                    if player_floor || pass_wall {
                        ctx.log.add(format!("You push past {}.", name));
                        ctx.commands.entity(e).insert(GridPos { x: px, y: py });
                        if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                            m.awake = true;
                        }
                    } else {
                        ctx.log
                            .add(format!("{} is in your way!", capitalize(&name)));
                        return;
                    }
                }
            } else {
                // Confused, stunned or unaware: fall through to py_attack,
                // which stops before hitting a friend.
                if ctx.ps.fear > 0 {
                    ctx.log.add("You are too afraid to attack!");
                    consume_turn(ctx);
                    return;
                }
                if let Some((pdef, _, _)) = ctx.ps.possessed {
                    melee_possessed(ctx, e, pdef);
                    consume_turn(ctx);
                    return;
                }
                if !melee_attack(ctx, e) {
                    consume_turn(ctx);
                }
                return;
            }
        } else {
            // Fearful characters cannot bring themselves to strike.
            if ctx.ps.fear > 0 {
                ctx.log.add("You are too afraid to attack!");
                consume_turn(ctx);
                return;
            }
            // A Possessor fights with the form's own blows.
            if let Some((pdef, _, _)) = ctx.ps.possessed {
                melee_possessed(ctx, e, pdef);
                consume_turn(ctx);
                return;
            }
            if !melee_attack(ctx, e) {
                consume_turn(ctx);
            }
            return;
        }
    }

    // Doors block movement; bump into them to open/bash.
    let t = ctx.map.terrain_at(nx, ny);
    if map::is_closed_door(t) && t != map::T_SECRET_DOOR {
        force_door(ctx, nx, ny, t);
        return;
    }

    // Walk.
    let at_border = ctx.ps.depth == 0
        && !ctx.ps.wild_mode
        && (nx == 0 || nx == map::MAP_W - 1 || ny == 0 || ny == map::MAP_H - 1);
    // Wilderness borders are permanent rock whose mimic shows the
    // neighbouring area's terrain; entry is tested against that shown
    // feature (cmd1.cc move_player_aux checks c_ptr->mimic).
    let entry_t = if at_border {
        ctx.map.display_terrain(&ctx.gd, nx, ny)
    } else {
        t
    };
    if player_can_walk_t(&ctx.map, &ctx.gd, &ctx.ps, &ctx.inv, nx, ny, entry_t) {
        // Walking off the edge of a wilderness area moves to the adjacent
        // world cell (src/cmd1.cc; combat/ambush ends at the border).
        if at_border {
            let (mut wx, mut wy) = (ctx.ps.wild_x, ctx.ps.wild_y);
            let mut entry = (nx, ny);
            if nx == 0 {
                wx -= 1;
                entry.0 = map::MAP_W - 2;
            } else if nx == map::MAP_W - 1 {
                wx += 1;
                entry.0 = 1;
            }
            if ny == 0 {
                wy -= 1;
                entry.1 = map::MAP_H - 2;
            } else if ny == map::MAP_H - 1 {
                wy += 1;
                entry.1 = 1;
            }
            if ctx.store.wilderness.in_bounds(wx, wy) {
                ctx.ps.wild_x = wx;
                ctx.ps.wild_y = wy;
                ctx.turn.ambush = false;
                ctx.turn.pending_pos = Some(entry);
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Wild(wx, wy));
                consume_turn(ctx);
            }
            return;
        }
        if let Ok(mut p) = ctx.player.single_mut() {
            p.x = nx;
            p.y = ny;
        }
        // q_betwen.cc quest_between_move_hook: the thunderlord ambush
        // springs on any move made in the northern wilderness
        // (wilderness_y <= 19, or py <= 19 on the travel overview), once
        // per game.
        if ctx.plot.status(game::PLOT_BETWEEN) == game::PLOT_TAKEN
            && ctx.ps.depth == 0
            && !ctx.plot.between_ambush
            && if ctx.ps.wild_mode {
                ny <= 19
            } else {
                ctx.ps.wild_y <= 19
            }
        {
            ctx.plot.between_ambush = true;
            ctx.ps.wild_mode = false;
            ctx.log
                .add("Looks like a full wing of thunderlords ambushes you!");
            ctx.log
                .add("Trone steps forth and speaks: 'The secret of the Void Jumpgates");
            ctx.log
                .add("will not be used by any but the thunderlords!'");
            ctx.turn.pending =
                Some(game::Goto::Depth(game::PLOT_DEPTH_BASE + game::PLOT_BETWEEN));
            consume_turn(ctx);
            return;
        }
        // Dripping Tread: elemental terrain follows the caster
        // (cmd1.cc move_player).
        if ctx.ps.dripping_tread > 0 {
            let mut rng = crate::rng::current();
            crate::game::geomancy_random_floor(
                &mut ctx.map,
                &ctx.gd,
                &ctx.ps,
                nx,
                ny,
                false,
                &mut rng,
            );
            ctx.ps.dripping_tread -= 1;
            if ctx.ps.dripping_tread == 0 {
                ctx.log.add("You stop dripping raw elemental energies.");
            }
        }
        // Walking over matching ammunition feeds the quiver
        // (object1.cc pickup_ammo, from py_pickup_floor).
        {
            let tval = |def: usize| ctx.gd.objects[def].tval;
            let mut merged = 0u32;
            let stacks: Vec<(Entity, Vec<item::Item>)> = ctx
                .stacks
                .iter()
                .filter(|(_, p, _)| p.x == nx && p.y == ny)
                .map(|(e, _, s)| (e, s.stack.clone()))
                .collect();
            for (e, stack) in stacks {
                let mut left = Vec::new();
                for it in stack {
                    let keep = match &mut ctx.inv.equip[data::SLOT_QUIVER] {
                        Some(q)
                            if matches!(
                                tval(it.def),
                                data::TV_SHOT | data::TV_ARROW | data::TV_BOLT
                            ) && q.def == it.def
                                && q.to_h == it.to_h
                                && q.to_d == it.to_d
                                && q.ego == it.ego
                                && q.artifact == it.artifact
                                && q.count + it.count <= 100 =>
                        {
                            q.count += it.count;
                            merged += 1;
                            false
                        }
                        _ => true,
                    };
                    if keep {
                        left.push(it);
                    }
                }
                let empty = left.is_empty();
                if let Ok((_, _, mut s)) = ctx.stacks.get_mut(e) {
                    s.stack = left;
                }
                if empty {
                    ctx.commands.entity(e).despawn();
                }
            }
            if merged > 0 {
                ctx.log.add("You add the ammo to your quiver.");
            }
        }
        // A Void Jumpgate (FEAT_BETWEEN) hurls the player to its partner
        // (cmd2.cc between_effect: "You fall into the void.").
        if t == map::T_BETWEEN {
            if let Some(partner) = ctx.map.between.get(&Map::idx(nx, ny)).copied() {
                ctx.log.add("You fall into the void.");
                ctx.log.add("Brrrr! It's deadly cold.");
                let (px, py) = (
                    (partner % map::MAP_W as usize) as i32,
                    (partner / map::MAP_W as usize) as i32,
                );
                if let Ok(mut p) = ctx.player.single_mut() {
                    p.x = px;
                    p.y = py;
                }
                cell_arrival(ctx, px, py);
                consume_turn(ctx);
                return;
            }
        }
        // Stepping on a trap springs it.
        if t == map::T_TRAP {
            trigger_trap(ctx, nx, ny);
        }
        // Stepping into a shop entrance opens the shop window.
        if let Some(mark) = ctx.map.shops.get(&Map::idx(nx, ny)) {
            let store = mark.store;
            let town = ctx.map.town;
            // Hack -- Check the "locked doors" (store.cc:3184 do_cmd_store):
            // a shop whose owner caught you stealing stays shut.
            if town::store_banned(&ctx.stocks, town::skey(town, store), ctx.ps.turn) {
                ctx.log.add("The doors are locked.");
            } else {
                let mut rng = crate::rng::current();
                town::stock_for(
                    &ctx.gd,
                    &mut ctx.stocks,
                    town,
                    store,
                    &mut ctx.created.0,
                    ctx.ps.depth,
                    ctx.ps.level,
                    &mut rng,
                );
                // The temple doubles as the gods' altar (gods.cc). Shops
                // with extra building actions (Weaponsmith's compare, ...)
                // show the action menu first (store.cc show_building).
                if store == 3 {
                    *ctx.modal = Modal::Temple;
                } else {
                    let extra = ctx
                        .gd
                        .stores
                        .iter()
                        .find(|s| s.id == store)
                        .map(|s| s.actions.iter().any(|a| *a > 4))
                        .unwrap_or(false);
                    if extra {
                        *ctx.modal = Modal::Building(store);
                    } else {
                        *ctx.modal = Modal::Shop(store, town);
                    }
                }
            }
        }
        // Stepping onto a quest entrance enters the plot quest.
        if let Some(&qid) = ctx.map.quest_entrances.get(&Map::idx(nx, ny)) {
            enter_plot_quest(ctx, qid);
        }
        // Aragorn reforges Narsil when you enter the Castle of Minas
        // Anor (q_narsil.cc HOOK_MOVE).
        if ctx.map.buildings.get(&Map::idx(nx, ny)) == Some(&14) {
            let mut rng = crate::rng::current();
            if game::try_reforge_narsil(
                &ctx.gd,
                &mut ctx.plot,
                &mut ctx.inv,
                &mut ctx.created.0,
                &mut ctx.log,
            ) {
                consume_turn(ctx);
                return;
            }
            let _ = &mut rng;
        }
        // Galadriel's warning (q_one.cc quest_one_move_hook): stepping
        // onto the Mirror of Galadriel after the Necromancer's fall
        // starts the One Ring quest.
        if ctx.map.buildings.get(&Map::idx(nx, ny)) == Some(&23)
            && ctx.plot.status(game::PLOT_ONE) == game::PLOT_UNTAKEN
            && ctx.plot.status(game::PLOT_NECRO) >= game::PLOT_COMPLETED
        {
            // q_one.cc quest_one_move_hook: Galadriel warns once the
            // Necromancer (QUEST_NECRO) has fallen.
            {
                ctx.plot.set(game::PLOT_ONE, game::PLOT_TAKEN);
                ctx.plot.one_taken = true;
                ctx.log.add("You meet Galadriel; she seems worried.");
                ctx.log
                    .add("'The Necromancer of Dol Guldur was none other than");
                ctx.log
                    .add("Sauron himself, and the One Ring is his to command.");
                ctx.log
                    .add("Cast it into the fires of Mount Doom — and *NEVER* wear it!'");
                let line = match ctx.ps.god {
                    1 => "'Eru will abandon you if you wear it.'",
                    2 => "'Manwe will abandon you if you wear it.'",
                    3 => "'Tulkas will abandon you if you wear it.'",
                    5 => "'Yavanna will abandon you if you wear it.'",
                    4 => "'Melkor will abandon you if you destroy it.'",
                    _ => "",
                };
                if !line.is_empty() {
                    ctx.log.add(line);
                }
            }
        }
        // Flavor buildings open their service windows (bldg.cc).
        if let Some(&special) = ctx.map.buildings.get(&Map::idx(nx, ny)) {
            *ctx.modal = Modal::Building(special);
        }
        cell_arrival_pickup(ctx, nx, ny, pickup);
        consume_turn(ctx);
        return;
    }

    // Wraithform seeps through (non-permanent) rock.
    let def = ctx.gd.terrain(t);
    let seep = ctx.inv.totals_for(&ctx.gd, &ctx.ps).wraith
        || game::player_has_flag(&ctx.gd, &ctx.ps, "SEMI_WRAITH");
    if seep && def.tunnelable && !def.permanent {
        // Rooted means no move (cmd1.cc:2974).
        if ctx.ps.tim_roots > 0 {
            return;
        }
        if !ctx.inv.totals_for(&ctx.gd, &ctx.ps).wraith {
            // Semi-wraiths are hurt passing through walls (dungeon.cc:1264).
            let amt = (1 + ctx.ps.level as i32 / 5).min(ctx.ps.hp - 1).max(1);
            ctx.ps.hp -= amt;
            ctx.log.add("You pass through the walls ...");
        }
        if let Ok(mut p) = ctx.player.single_mut() {
            p.x = nx;
            p.y = ny;
        }
        consume_turn(ctx);
        return;
    }

    // Probability Travel: pass through walls and emerge on the other side
    // (spells2.cc passwall, p_ptr->prob_travel).
    if ctx.ps.prob_travel > 0 && passwall(ctx, dx, dy) {
        consume_turn(ctx);
        return;
    }

    // Blocked.
    let dark_pit = t == map::T_DARK_PIT
        && !(ctx.ps.tim_ffall > 0
            || ctx.inv.totals_for(&ctx.gd, &ctx.ps).feather
            || game::god_feather(&ctx.ps));
    if dark_pit {
        ctx.log.add("You can't cross the chasm.");
        return;
    }
    if verbose && Map::in_bounds(nx, ny) {
        let def = ctx.gd.terrain(t);
        let idx = Map::idx(nx, ny);
        if !ctx.map.explored[idx] && !ctx.map.visible[idx] {
            // "Notice things in the dark" (cmd1.cc:2896): unremembered
            // blockers are only felt, then memorized.
            if t == map::T_RUBBLE {
                ctx.log.add("You feel some rubble blocking your way.");
            } else if map::is_closed_door(t) && t < map::T_SECRET_DOOR {
                ctx.log.add("You feel a closed door blocking your way.");
            } else {
                let shown = ctx.map.display_terrain(&ctx.gd, nx, ny);
                let sdef = ctx.gd.terrain(shown);
                let desc = if !sdef.block_desc.is_empty() {
                    sdef.block_desc.clone()
                } else {
                    sdef.name.clone()
                };
                ctx.log.add(format!("You feel {}.", desc));
            }
            ctx.map.explored[idx] = true;
        } else if t == map::T_RUBBLE {
            ctx.log.add("There is rubble blocking your way.");
        } else if def.tunnelable {
            ctx.log
                .add("There is a wall in the way. (Ctrl+direction to tunnel)");
        } else if !def.block_desc.is_empty() {
            // f_info D:2 ("a web blocking your way", ...).
            ctx.log.add(format!("There is {}.", def.block_desc));
        } else {
            ctx.log.add(format!("There is a {} in the way.", def.name));
        }
    }
}

/// Step onto a plot-quest entrance: enter the quest level (unless it is
/// already finished). Quest-specific entry gifts apply (q_poison hands
/// out cure-water flasks).
fn enter_plot_quest(ctx: &mut InputCtx, qid: u32) {
    use game::{PLOT_COMPLETED, PLOT_FAILED, PLOT_POISON, PLOT_TAKEN, PLOT_UNTAKEN};
    match ctx.plot.status(qid) {
        PLOT_COMPLETED => {
            ctx.log.add("You have already completed this quest.");
        }
        PLOT_FAILED => {
            ctx.log.add("You failed this quest. You dare not return.");
        }
        _ => {
            // Quest chains gate their entrances (q_*.cc init/forbid hooks).
            if !game::quest_unlocked(&ctx.plot, qid) {
                ctx.log.add("You are not ready for this quest yet.");
                return;
            }
            // The Last Alliance needs a level-45 hero (q_betwen.cc).
            if qid == game::PLOT_BETWEEN && ctx.ps.level < 45 {
                ctx.log
                    .add("I fear you are not ready for the next quest, come back later.");
                return;
            }
            if qid != PLOT_POISON && ctx.plot.status(qid) == PLOT_UNTAKEN {
                ctx.plot.set(qid, PLOT_TAKEN);
            }
            if qid == PLOT_POISON && ctx.plot.status(PLOT_POISON) == PLOT_UNTAKEN {
                // 99 flasks of cure-water (q_poison.cc).
                if let Some(def) = ctx.gd.object_by_name("Water Curing") {
                    for _ in 0..99 {
                        ctx.inv.add_with(&ctx.gd, item::Item::base(&ctx.gd, def));
                    }
                    ctx.log.add("You are given 99 flasks of cure-water.");
                    ctx.log
                        .add("'Purify the pond once the beasts are driven off!'");
                    ctx.plot.set(PLOT_POISON, PLOT_TAKEN);
                }
            }
            ctx.ps.quest_origin = Some((ctx.ps.wild_x, ctx.ps.wild_y));
            autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(game::PLOT_DEPTH_BASE + qid));
        }
    }
}

/// Flavor text for unhandled Bree buildings (kept for the map legend).
#[allow(dead_code)]
fn building_text(ctx: &mut InputCtx, special: u32) {
    let text = match special {
        9 => "The Pet Shop is closed.",
        10 => "The Mayor's Office. The mayor is busy with paperwork.",
        12 => "The Soothsayer mutters cryptic prophecies.",
        57 => "The Mathom-house displays many curious relics.",
        58 => "The Prancing Pony is full of travellers and smoke.",
        _ => "An interesting building.",
    };
    ctx.log.add(text);
}

/// Talk to a friendly NPC (bump-interaction). Farmer Maggot handles the
/// mushroom fetch quest; Melinda and Merton run the lost-hobbit quest.
fn npc_chat(ctx: &mut InputCtx, e: Entity) {
    let (_, m, _) = ctx.monsters.get(e).expect("monster");
    let name = ctx.gd.monsters[m.def].name.clone();
    match name.as_str() {
        "Farmer Maggot" => {
            maggot_chat(ctx);
        }
        "Melinda Proudfoot" => {
            use game::{PLOT_COMPLETED, PLOT_HOBBIT, PLOT_TAKEN, PLOT_UNTAKEN};
            match ctx.plot.status(PLOT_HOBBIT) {
                PLOT_COMPLETED => {
                    // The Rod of Recall is handed over immediately
                    // (q_hobbit.cc chat hook).
                    if let Some(it) = game::named_item(
                        &ctx.gd,
                        "Recall",
                        None,
                        20,
                        &mut ctx.created.0,
                        &mut crate::rng::current(),
                    ) {
                        ctx.log
                            .add("'My Merton is back! You saved him, hero.'");
                        ctx.log.add("'Take this as a proof of my gratitude.'");
                        game::ps_reward_item(&mut ctx.inv, &ctx.gd, it, &mut ctx.log);
                    }
                    ctx.plot.set(PLOT_HOBBIT, 5); // FINISHED
                }
                PLOT_UNTAKEN => {
                    ctx.plot.set(PLOT_HOBBIT, PLOT_TAKEN);
                    // The depth was rolled at game start (birth.rs);
                    // legacy saves get a fallback roll.
                    if ctx.plot.hobbit_depth == 0 {
                        ctx.plot.hobbit_depth = crate::rng::current().gen_range(26..=34);
                    }
                    ctx.log.add("'Oh! Oh!'");
                    ctx.log.add("'My poor Merton, where is my poor Merton? He was playing");
                    ctx.log
                        .add("near that dreadful maze and never been seen again!'");
                }
                _ => {
                    ctx.log
                        .add("'Have you found my Merton yet? Give him a recall scroll!'");
                }
            }
        }
        "Merton Proudfoot, the lost hobbit" => {
            use game::{PLOT_COMPLETED, PLOT_HOBBIT};
            if ctx.plot.status(PLOT_HOBBIT) == PLOT_COMPLETED {
                ctx.log.add("'I'm on my way home now, thank you!'");
                return;
            }
            // Give him a Word of Recall scroll so he can go home.
            let idx = ctx
                .inv
                .pack
                .iter()
                .position(|it| ctx.gd.objects[it.def].name.starts_with("Word of Recall"));
            if let Some(i) = idx {
                ctx.inv.pack.remove(i);
                ctx.plot.set(PLOT_HOBBIT, PLOT_COMPLETED);
                ctx.plot.hobbit_turn = ctx.ps.turn;
                ctx.log.add("'A recall scroll! For me? Oh thank you!'");
                ctx.log
                    .add("Merton vanishes home in a flash of light. Tell Melinda!");
                ctx.log
                    .add("(She will want a few days to be sure he's safe.)");
            } else {
                ctx.log
                    .add("'I'm lost! If only I had a Word of Recall scroll...'");
            }
        }
        "Thrain, the King Under the Mountain" => {
            ctx.log
                .add("'Free me, and the treasure of the Mountain is yours.'");
        }
        _ => {
            ctx.log.add(format!("The {} does not want to talk.", name));
        }
    }
}

/// Farmer Maggot and the mushroom fetch quest: delivery happens
/// automatically once you carry enough mushrooms.
fn maggot_chat(ctx: &mut InputCtx) {
    use game::{PLOT_COMPLETED, PLOT_FAILED, PLOT_FINISHED, PLOT_SHROOM, PLOT_TAKEN, PLOT_UNTAKEN};
    match ctx.plot.status(PLOT_SHROOM) {
        PLOT_COMPLETED | PLOT_FINISHED => {
            ctx.log.add("Farmer Maggot waves at you cheerfully.");
        }
        PLOT_FAILED => {
            ctx.log.add("'My puppy! My poor little puppy!'");
            ctx.log.add("'YOU MURDERER! Out of my sight!'");
        }
        _ => {
            if ctx.plot.status(PLOT_SHROOM) == PLOT_UNTAKEN {
                ctx.plot.set(PLOT_SHROOM, PLOT_TAKEN);
                ctx.log.add("'My mushrooms, my mushrooms!'");
                ctx.log.add("'The rain... it ruins everything!'");
                ctx.log.add(
                    "'Could you please bring me back all the mushrooms growing \
                     to the west of Bree? Please try to not harm my dogs.'",
                );
            }
            let needed = ctx.plot.shrooms_needed.max(1);
            let have = count_mushrooms(ctx);
            if have >= needed as usize {
                deliver_mushrooms(ctx, needed as usize);
            } else {
                ctx.log.add(format!(
                    "Farmer Maggot sobs. (You carry {}/{} mushrooms.)",
                    have, needed
                ));
            }
        }
    }
}

/// Mushrooms in the pack (TV_FOOD sval 0-19).
fn count_mushrooms(ctx: &InputCtx) -> usize {
    ctx.inv
        .pack
        .iter()
        .filter(|it| {
            let o = &ctx.gd.objects[it.def];
            o.tval == data::TV_FOOD && (0..=19).contains(&o.sval)
        })
        .count()
}

/// Hand over the mushrooms (q_shroom.cc give hook): one Cure Serious
/// Wounds stack and the artifact sling "of Farmer Maggot" (a_info 149).
fn deliver_mushrooms(ctx: &mut InputCtx, needed: usize) {
    let mut left = needed;
    ctx.inv.pack.retain(|it| {
        let o = &ctx.gd.objects[it.def];
        let shroom = o.tval == data::TV_FOOD && (0..=19).contains(&o.sval);
        if shroom && left > 0 {
            left -= 1;
            false
        } else {
            true
        }
    });
    ctx.log.add("Oh thank you!");
    ctx.log.add("Take my sling and those mushrooms, may they help you!");
    ctx.log.add("Farmer Maggot heads back to his house.");
    let mut rng = crate::rng::current();
    // 15-20 mushrooms of Cure Serious Wounds (SV_FOOD_CURE_SERIOUS = 16).
    if let Some(def) = ctx.gd.object_by_tval_sval(data::TV_FOOD, 16) {
        let mut it = item::Item::base(&ctx.gd, def);
        it.count = rng.gen_range(15..=20);
        it.identified = true;
        ctx.inv.add_with(&ctx.gd, it);
    }
    // The specific artifact (a_info 149; apply_magic is part of the
    // artifact item construction).
    if let Some(sling) =
        item::make_specific_artifact(&ctx.gd, 149, &mut ctx.created.0, &mut rng)
    {
        ctx.log.add(format!(
            "He gives you {}!",
            sling.label(&ctx.gd, &ctx.inv.known)
        ));
        ctx.inv.add_with(&ctx.gd, sling);
    }
    ctx.plot.set(game::PLOT_SHROOM, game::PLOT_FINISHED);
}

/// The quest HOOK_GIVE behaviours (q_hobbit.cc:105, q_shroom.cc:150):
/// Merton takes a Word of Recall scroll, Maggot one mushroom at a time.
fn quest_give_hook(ctx: &mut InputCtx, e: Entity, it: &item::Item) -> bool {
    use game::{
        PLOT_COMPLETED, PLOT_FAILED_DONE, PLOT_HOBBIT, PLOT_SHROOM, PLOT_TAKEN, PLOT_UNTAKEN,
    };
    let name = {
        let (_, m, _) = ctx.monsters.get(e).expect("monster");
        ctx.gd.monsters[m.def].name.clone()
    };
    let o = &ctx.gd.objects[it.def];
    if name == "Merton Proudfoot, the lost hobbit" {
        if ctx.plot.status(PLOT_HOBBIT) == PLOT_COMPLETED {
            return false;
        }
        if o.tval == data::TV_SCROLL && o.name.starts_with("Word of Recall") {
            ctx.commands.entity(e).despawn();
            ctx.plot.set(PLOT_HOBBIT, PLOT_COMPLETED);
            ctx.plot.hobbit_turn = ctx.ps.turn;
            ctx.log.add("'Oh, thank you, noble one!'");
            ctx.log.add(
                "Merton Proudfoot reads the scroll and is recalled to the safety of his home.",
            );
            return true;
        }
        return false;
    }
    if name == "Farmer Maggot" {
        if !matches!(
            ctx.plot.status(PLOT_SHROOM),
            PLOT_TAKEN | PLOT_UNTAKEN
        ) {
            return false;
        }
        if o.tval != data::TV_FOOD || !(0..=19).contains(&o.sval) {
            return false;
        }
        // A dead quest dog fails the quest (q_shroom.cc:167).
        let dogs_alive = |name: &str| {
            ctx.monsters
                .iter()
                .any(|(_, m, _)| m.hp > 0 && ctx.gd.monsters[m.def].name == name)
        };
        if !dogs_alive("Grip, Farmer Maggot's dog")
            || !dogs_alive("Wolf, Farmer Maggot's dog")
            || !dogs_alive("Fang, Farmer Maggot's dog")
        {
            ctx.plot.set(game::PLOT_SHROOM, PLOT_FAILED_DONE);
            ctx.log.add("My puppy!  My poor, defenceless puppy...");
            ctx.log.add("YOU MURDERER!  Out of my sight!");
            ctx.commands.entity(e).despawn();
            return true;
        }
        let needed = ctx.plot.shrooms_needed.max(1) as usize;
        let have = count_mushrooms(ctx);
        if have >= needed {
            deliver_mushrooms(ctx, needed);
        } else {
            ctx.log.add(format!(
                "Oh thank you, but you still have {} mushrooms to bring back!",
                needed - have
            ));
        }
        return true;
    }
    false
}
/// Bump-attack while possessing a corpse: the form's blows replace the
/// wielded weapon entirely (Possessor class, cmd5.cc do_cmd_possess).
fn melee_possessed(ctx: &mut InputCtx, e: Entity, form_def: usize) {
    let mut rng = crate::rng::current();
    let form = ctx.gd.monsters[form_def].clone();
    // RF_NEVER_BLOW bodies cannot attack (cmd1.cc incarnate_monster_attack).
    if form.has("NEVER_BLOW") {
        return;
    }
    // The blows use the body's own level, not the possessor's.
    let rlev = form.depth.max(1) as i32;
    let form_blows: Vec<crate::data::BlowDef> = form.blows.clone();
    let form_name = form.name.clone();
    let (_, mut m, mpos) = ctx.monsters.get_mut(e).expect("monster");
    let held = (
        m.held_artifact,
        m.held_object,
        m.items.clone(),
        m.gold,
        m.looted,
    );
    let def_idx = m.def;
    let name = game::hallucinated_name(&ctx.gd, &ctx.ps, def_idx, &mut rng);
    let def = ctx.gd.monsters[def_idx].clone();
    let mac = def.ac + m.ac_mod;
    let to_d = ctx.inv.totals_for(&ctx.gd, &ctx.ps).to_d;
    let (mx, my) = (mpos.x, mpos.y);
    let vis = ctx.map.visible[Map::idx(mx, my)]
        && (!def.has("INVISIBLE")
            || ctx.ps.tim_invis > 0
            || ctx.inv.totals_for(&ctx.gd, &ctx.ps).see_invis);
    if !vis {
        ctx.log.add("You hear noise.");
    }
    m.awake = true;
    let mut killed = false;
    let mut blinked = false;
    let mut quake = false;
    for blow in form_blows.iter().take(4) {
        if m.hp <= 0 {
            break;
        }
        let effect = blow.effect.as_str();
        let method = blow.method.as_str();
        let touched = matches!(
            method,
            "HIT"
                | "TOUCH"
                | "PUNCH"
                | "KICK"
                | "CLAW"
                | "BITE"
                | "STING"
                | "BUTT"
                | "CRUSH"
                | "ENGULF"
                | "CHARGE"
                | "CRAWL"
        );
        if !game::monster_check_hit(game::blow_power(effect), rlev as u32, mac, &mut rng) {
            if touched && vis {
                ctx.log.add(format!("You miss the {}.", name));
            }
            continue;
        }
        let dice = Dice::parse(&blow.dice).unwrap_or(Dice {
            count: 1,
            sides: 2,
            bonus: 0,
        });
        let mut dmg = dice.roll(&mut rng) + to_d;
        // RBE_* effect conversion (cmd1.cc incarnate_monster_attack).
        let gf = match effect {
            "POISON" | "DISEASE" => "POIS",
            "UN_BONUS" | "UN_POWER" => "DISENCHANT",
            "ACID" => "ACID",
            "ELEC" => "ELEC",
            "FIRE" => "FIRE",
            "COLD" => "COLD",
            "HALLU" | "CONFUSE" => "CONFUSION",
            "TERRIFY" => "TURN_ALL",
            "PARALYZE" => "OLD_SLEEP",
            "EXP_10" | "EXP_20" | "EXP_40" | "EXP_80" => "NETHER",
            "TIME" => "TIME",
            "EAT_ITEM" | "EAT_GOLD" => {
                dmg = 0;
                if rng.gen_range(0..2) == 0 {
                    blinked = true;
                }
                ""
            }
            "EAT_FOOD" | "EAT_LITE" => {
                dmg = 0;
                ""
            }
            "" | "HURT" | "SANITY" => {
                dmg -= dmg * mac.clamp(0, 150) / 250;
                ""
            }
            _ => "",
        };
        if !gf.is_empty() {
            // GF_OLD_SLEEP uses the player's level, not the blow damage.
            let power = if gf == "OLD_SLEEP" {
                ctx.ps.level as i32 * 2
            } else {
                dmg
            };
            let eff = game::gf_monster_effect(&def, gf, power, rlev, true, &mut rng);
            m.poison += eff.poison;
            m.cut += eff.cut;
            m.stun += eff.stun;
            m.confused += eff.conf;
            if eff.slow {
                m.mspeed_mod -= 10;
            }
            dmg = eff.dam;
        }
        if effect == "SHATTER"
            && dmg > 23
            && ctx.ps.depth > 0
            && ctx.ps.depth < game::PLOT_DEPTH_BASE
        {
            quake = true;
        }
        let dealt = dmg.max(0);
        m.hp -= dealt;
        game::clamp_no_death(&ctx.gd, &mut m);
        if dealt > 0 && vis {
            ctx.log.add(format!(
                "Your {} form hits the {}. ({})",
                form_name, name, dealt
            ));
        }
        // Touched monsters burn or zap the body (cmd1.cc:1385).
        if touched {
            let aura_dice = Dice {
                count: 1 + (def.depth as i32) / 26,
                sides: 1 + (def.depth as i32) / 17,
                bonus: 0,
            };
            if def.has("AURA_FIRE") && !form.has("IM_FIRE") {
                let d = aura_dice.roll(&mut rng);
                if vis {
                    ctx.log.add("You are suddenly very hot!");
                }
                let d = game::symbiote_intercept(&ctx.ps, &mut ctx.inv, d, &mut rng, &mut ctx.log);
                let rem = game::absorb_damage(&mut ctx.ps, d, &mut ctx.log);
                ctx.ps.hp -= rem;
            }
            if def.has("AURA_ELEC") && !form.has("IM_ELEC") {
                let d = aura_dice.roll(&mut rng);
                if vis {
                    ctx.log.add("You get zapped!");
                }
                let d = game::symbiote_intercept(&ctx.ps, &mut ctx.inv, d, &mut rng, &mut ctx.log);
                let rem = game::absorb_damage(&mut ctx.ps, d, &mut ctx.log);
                ctx.ps.hp -= rem;
            }
        }
        if m.hp <= 0 {
            killed = true;
            break;
        }
    }
    drop(m);
    if quake {
        let (px, py) = ctx.player.single().map(|p| (p.x, p.y)).unwrap_or((mx, my));
        game::earthquake(&mut ctx.map, &ctx.gd, px, py, 8, &mut ctx.log, &mut rng);
    }
    if blinked && ctx.ps.hp > 0 {
        ctx.log.add("You flee laughing!");
        let occupied: HashSet<(i32, i32)> =
            ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
        let mut rng2 = crate::rng::current();
        if let Ok(mut pp) = ctx.player.single_mut() {
            teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pp, &occupied, map::MAX_SIGHT * 2 + 5, &mut rng2);
            ctx.ps.last_teleport = Some((pp.x, pp.y));
        }
        ctx.turn.fov_dirty = true;
    }
    if killed {
        // A POSSESSOR soul deincarnates instead of dying (monster3.cc
        // ai_deincarnate).
        if let Ok((_, mut dm, _)) = ctx.monsters.get_mut(e) {
            let mut rng = crate::rng::current();
            if game::deincarnate_monster(&ctx.gd, &mut dm, None, &mut ctx.log, &mut rng) {
                return;
            }
        }
        let quest_left = ctx
            .monsters
            .iter()
            .filter(|(qe, m, _)| *qe != e && m.quest)
            .count() as i32;
            let enemy_left = ctx
                .monsters
                .iter()
                .filter(|(qe, m, _)| *qe != e && !m.friendly && !m.companion && !m.neutral)
                .count() as i32;
        game::kill_monster(
            &mut ctx.commands,
            &ctx.gd,
            &ctx.tiles,
            &mut ctx.stacks,
            e,
            def_idx,
            (mx, my),
            held.0,
            held.1,
            held.2,
            held.3,
            held.4,
            &mut ctx.ps,
            &mut ctx.plot,
            &mut ctx.map,
            quest_left,
            enemy_left,
            &mut ctx.created.0,
            &mut ctx.inv,
            &mut ctx.log,
            &mut rng,
            false,
        );
    }
}

/// Bump-attack with the wielded weapon (original ToME formulas).
/// The player swings once per blow (num_blows) each round; slays,
/// brands, vorpal edges, vampiric drain and monster auras apply.
/// Returns true when a Spread Blows kill left unused attacks (the caller
/// then skips the world turn, cmd1.cc energy_use reduction).
/// The Demon Blade's per-blow projection (xtra2.cc set_project): while
/// `tim_project` lasts, a landed blow also projects the stored damage
/// type at the victim's cell (radius 0, or 1 at demonology skill 45).
/// Returns the monsters that died, for the caller to process.
fn demon_blade_project(
    ctx: &mut InputCtx,
    cx: i32,
    cy: i32,
    rng: &mut impl Rng,
) -> Vec<(Entity, usize, (i32, i32))> {
    let gf = ctx.ps.tim_project_gf.clone();
    if gf.is_empty() {
        return Vec::new();
    }
    let radius = ctx.ps.tim_project_rad.max(0);
    let dam = ctx.ps.tim_project_dam.max(1);
    let targets: Vec<(Entity, usize, i32, i32)> = ctx
        .monsters
        .iter()
        .filter(|(_, m, p)| m.hp > 0 && map::chebyshev(p.x, p.y, cx, cy) <= radius)
        .map(|(e, m, p)| (e, m.def, p.x, p.y))
        .collect();
    let level = ctx.ps.level as i32;
    let mut dead = Vec::new();
    for (e, def_idx, x, y) in targets {
        let eff = game::gf_monster_effect(&ctx.gd.monsters[def_idx], &gf, dam, level, true, rng);
        let (_, mut m, _) = ctx.monsters.get_mut(e).expect("monster");
        if eff.poison > 0 {
            m.poison += eff.poison;
        }
        if eff.cut > 0 {
            m.cut += eff.cut;
        }
        if eff.stun > 0 {
            m.stun += eff.stun;
        }
        if eff.conf > 0 {
            m.confused += eff.conf;
        }
        if eff.slow {
            m.mspeed_mod -= 10;
        }
        m.hp -= eff.dam;
        game::clamp_no_death(&ctx.gd, &mut m);
        m.awake = true;
        if m.hp <= 0 {
            dead.push((e, def_idx, (x, y)));
        }
    }
    dead
}

/// test_hit_fire / test_hit_norm (cmd1.cc:64/98): like game::test_hit but
/// with the invisible-target penalty and the luck roll.
fn test_hit_vis(chance: i32, ac: i32, vis: bool, luck: i32, rng: &mut impl Rng) -> bool {
    let k = rng.gen_range(0..100);
    if k < 10 {
        return k < 5;
    }
    if chance <= 0 {
        return false;
    }
    let c = if vis { chance } else { (chance + 1) / 2 };
    let roll = (c + game::luck_range(luck, -10, 10)).max(1);
    rng.gen_range(0..roll) >= ac * 3 / 4
}

/// critical_norm (cmd1.cc:172) with the skill tree's contributions and the
/// luck roll.
#[allow(clippy::too_many_arguments)]
fn critical_norm_melee(
    ps: &mut PlayerState,
    weight: i32,
    plus: i32,
    to_h: i32,
    xtra_crit: i32,
    is_light_sword: bool,
    dam: i32,
    luck: i32,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) -> i32 {
    // Tulkas' Divine Aim (set_tim_deadly) forces a *GREAT* hit and is
    // consumed by the blow (cmd1.cc:189-195).
    if ps.tim_deadly > 0 {
        ps.tim_deadly -= 1;
        if ps.tim_deadly == 0 {
            log.add("You are suddenly much less accurate.");
        }
        log.add("It was a *GREAT* hit!");
        return 3 * dam + 20;
    }
    let mut i = weight + (to_h + plus) * 5 + ps.skill_scale(ps.melee_style, 150) + 50 * xtra_crit;
    if is_light_sword {
        i += ps.skill_scale(crate::skill::SK_CRITS, 2000);
    }
    i += game::luck_range(luck, -100, 100);
    if rng.gen_range(1..=5000) > i {
        return dam;
    }
    let mut k = weight + rng.gen_range(1..=650);
    if is_light_sword {
        k += ps.skill_scale(crate::skill::SK_CRITS, 400);
    }
    let (msg, dam) = if k < 400 {
        ("good", 2 * dam + 5)
    } else if k < 700 {
        ("great", 2 * dam + 10)
    } else if k < 900 {
        ("superb", 3 * dam + 15)
    } else if k < 1300 {
        ("*GREAT*", 3 * dam + 20)
    } else {
        ("*SUPERB*", 7 * dam / 2 + 25)
    };
    log.add(format!("It was a {} hit!", msg));
    dam
}

/// test_hit_fire (cmd1.cc:64): fired/thrown missiles.  The maths are the
/// same as melee (`test_hit_vis`): 10% forced result, the invisible-target
/// half chance and `luck(-10, 10)`.  Used by `modal.rs::fire_missile` and
/// `modal.rs::power_throw`.
pub fn test_hit_fire(chance: i32, ac: i32, vis: bool, luck: i32, rng: &mut impl Rng) -> bool {
    test_hit_vis(chance, ac, vis, luck, rng)
}

/// `critical_shot` (cmd1.cc:131): criticals for thrown/fired objects.
/// `i = weight + (to_h + plus)*4 + get_skill_scale(skill, 100)
/// + 50*xtra_crit + luck(-100,100)`, then `randint(5000) <= i` with the
/// 500/1000 damage tiers.  (The melee `critical_norm` thresholds differ.)
/// Used by `modal.rs::fire_missile` and `modal.rs::power_throw`.
#[allow(clippy::too_many_arguments)]
pub fn critical_shot(
    weight: i32,
    plus: i32,
    dam: i32,
    skill_scale: i32,
    to_h: i32,
    xtra_crit: i32,
    luck: i32,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) -> i32 {
    let i = weight
        + (to_h + plus) * 4
        + skill_scale
        + 50 * xtra_crit
        + game::luck_range(luck, -100, 100);
    if rng.gen_range(1..=5000) > i {
        return dam;
    }
    let k = weight + rng.gen_range(1..=500);
    let (msg, dam) = if k < 500 {
        ("good", 2 * dam + 5)
    } else if k < 1000 {
        ("great", 2 * dam + 10)
    } else {
        ("superb", 3 * dam + 15)
    };
    log.add(format!("It was a {} hit!", msg));
    dam
}

const MA_KNEE: u32 = 0x0001;
const MA_SLOW: u32 = 0x0002;
const MA_WOUND: u32 = 0x0004;
const MA_STUN: u32 = 0x0008;
const MA_FULL_SLOW: u32 = 0x0010;

/// tables.cc ma_blows (MAX_MA = 17).
struct Ma {
    desc: &'static str,
    min_level: i32,
    chance: i32,
    dd: i32,
    ds: i32,
    effect: u32,
    power: i32,
}

const MA_BLOWS: [Ma; 17] = [
    Ma {
        desc: "You punch %s.",
        min_level: 1,
        chance: 0,
        dd: 2,
        ds: 4,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You kick %s.",
        min_level: 2,
        chance: 0,
        dd: 2,
        ds: 6,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You strike %s.",
        min_level: 3,
        chance: 0,
        dd: 2,
        ds: 7,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You hit %s with your knee.",
        min_level: 5,
        chance: 5,
        dd: 4,
        ds: 3,
        effect: MA_KNEE,
        power: 0,
    },
    Ma {
        desc: "You hit %s with your elbow.",
        min_level: 7,
        chance: 5,
        dd: 2,
        ds: 8,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You butt %s.",
        min_level: 9,
        chance: 10,
        dd: 4,
        ds: 5,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You kick %s.",
        min_level: 11,
        chance: 10,
        dd: 6,
        ds: 4,
        effect: MA_SLOW,
        power: 0,
    },
    Ma {
        desc: "You uppercut %s.",
        min_level: 13,
        chance: 12,
        dd: 8,
        ds: 4,
        effect: MA_STUN,
        power: 6,
    },
    Ma {
        desc: "You double-kick %s.",
        min_level: 16,
        chance: 15,
        dd: 10,
        ds: 4,
        effect: MA_STUN,
        power: 8,
    },
    Ma {
        desc: "You hit %s with a Cat's Claw.",
        min_level: 20,
        chance: 20,
        dd: 10,
        ds: 5,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You hit %s with a jump kick.",
        min_level: 25,
        chance: 25,
        dd: 10,
        ds: 6,
        effect: MA_STUN,
        power: 10,
    },
    Ma {
        desc: "You hit %s with an Eagle's Claw.",
        min_level: 29,
        chance: 25,
        dd: 12,
        ds: 6,
        effect: 0,
        power: 0,
    },
    Ma {
        desc: "You hit %s with a circle kick.",
        min_level: 33,
        chance: 30,
        dd: 12,
        ds: 8,
        effect: MA_STUN,
        power: 10,
    },
    Ma {
        desc: "You hit %s with an Iron Fist.",
        min_level: 37,
        chance: 35,
        dd: 16,
        ds: 8,
        effect: MA_STUN,
        power: 10,
    },
    Ma {
        desc: "You hit %s with a flying kick.",
        min_level: 41,
        chance: 35,
        dd: 16,
        ds: 10,
        effect: MA_STUN,
        power: 12,
    },
    Ma {
        desc: "You hit %s with a Dragon Fist.",
        min_level: 45,
        chance: 35,
        dd: 20,
        ds: 10,
        effect: MA_STUN,
        power: 16,
    },
    Ma {
        desc: "You hit %s with a Crushing Blow.",
        min_level: 48,
        chance: 35,
        dd: 20,
        ds: 12,
        effect: MA_STUN,
        power: 18,
    },
];

/// tables.cc bear_blows (MAX_BEAR = 8).
const BEAR_BLOWS: [Ma; 8] = [
    Ma {
        desc: "You claw %s.",
        min_level: 1,
        chance: 0,
        dd: 3,
        ds: 4,
        effect: MA_STUN,
        power: 4,
    },
    Ma {
        desc: "You swat %s.",
        min_level: 4,
        chance: 0,
        dd: 4,
        ds: 4,
        effect: MA_WOUND,
        power: 20,
    },
    Ma {
        desc: "You bite %s.",
        min_level: 9,
        chance: 2,
        dd: 4,
        ds: 4,
        effect: MA_WOUND,
        power: 30,
    },
    Ma {
        desc: "You hug %s.",
        min_level: 15,
        chance: 5,
        dd: 6,
        ds: 4,
        effect: MA_FULL_SLOW,
        power: 0,
    },
    Ma {
        desc: "You swat and rake %s.",
        min_level: 25,
        chance: 10,
        dd: 6,
        ds: 5,
        effect: MA_STUN | MA_WOUND,
        power: 10,
    },
    Ma {
        desc: "You hug and claw %s.",
        min_level: 30,
        chance: 15,
        dd: 6,
        ds: 6,
        effect: MA_FULL_SLOW | MA_WOUND,
        power: 60,
    },
    Ma {
        desc: "You double swat %s.",
        min_level: 35,
        chance: 20,
        dd: 9,
        ds: 7,
        effect: MA_STUN | MA_WOUND,
        power: 20,
    },
    Ma {
        desc: "You double swat and rake %s.",
        min_level: 40,
        chance: 25,
        dd: 10,
        ds: 10,
        effect: MA_STUN | MA_WOUND,
        power: 25,
    },
];

fn ma_desc(desc: &str, name: &str) -> String {
    desc.replace("%s", name)
}

/// Outcome of a martial-arts blow.
struct HandBlow {
    dam: i32,
    stun: i32,
    slow: bool,
    cut: bool,
}

/// py_attack_hand (cmd1.cc:1621): monk / bear-form bare-handed attack.
#[allow(clippy::too_many_arguments)]
fn py_attack_hand(
    gd: &GameData,
    ps: &mut PlayerState,
    def: &crate::data::MonsterDef,
    m_name: &str,
    m_hp: i32,
    m_level: i32,
    m_mspeed: i32,
    to_h: i32,
    to_d: i32,
    luck: i32,
    rng: &mut impl Rng,
    log: &mut MessageLog,
) -> HandBlow {
    let bear =
        ps.melee_style == crate::skill::SK_BEAR && ps.mimic_form == Some(crate::mimic::MIMIC_BEAR);
    let table: &[Ma] = if bear { &BEAR_BLOWS } else { &MA_BLOWS };
    let plev = if ps.melee_style == crate::skill::SK_HAND {
        ps.skill(crate::skill::SK_HAND)
    } else if bear {
        ps.skill(crate::skill::SK_BEAR)
    } else {
        ps.level as i32
    };
    let mut resist_stun = 0;
    if def.has("UNIQUE") {
        resist_stun += 88;
    }
    if def.has("NO_CONF") {
        resist_stun += 44;
    }
    if def.has("NO_SLEEP") {
        resist_stun += 44;
    }
    if def.has("UNDEAD") || def.has("NONLIVING") {
        resist_stun += 88;
    }
    let mut chosen = &table[0];
    if plev > 0 {
        let mut old = &table[0];
        let times = if plev < 7 { 1 } else { plev / 7 };
        for _ in 0..times {
            let mut ma = &table[rng.gen_range(0..table.len())];
            let pl = plev.max(1);
            while ma.min_level > plev || rng.gen_range(1..=pl) < ma.chance {
                ma = &table[rng.gen_range(0..table.len())];
            }
            if ma.min_level > old.min_level && ps.stun == 0 && ps.confuse == 0 {
                old = ma;
                ma = old;
            } else {
                ma = old;
            }
            chosen = ma;
        }
    }
    let mut k = Dice {
        count: chosen.dd,
        sides: chosen.ds,
        bonus: 0,
    }
    .roll(rng);
    let mut special = 0u32;
    let mut stun_effect = 0i32;
    let mut desc = false;
    if chosen.effect & MA_KNEE != 0 {
        if def.has("MALE") {
            if !desc {
                log.add(format!("You hit {} in the groin with your knee!", m_name));
            }
            special = MA_KNEE;
        } else if !desc {
            log.add(ma_desc(chosen.desc, m_name));
        }
        desc = true;
    }
    if chosen.effect & MA_FULL_SLOW != 0 {
        special = MA_SLOW;
        if !desc {
            log.add(ma_desc(chosen.desc, m_name));
        }
        desc = true;
    }
    if chosen.effect & MA_SLOW != 0 {
        let glyph = def.glyph();
        if !def.has("NEVER_MOVE") && !"UjmeEv$,DdsbBFIJQSXclnw!=?".contains(glyph) {
            if !desc {
                log.add(format!("You kick {} in the ankle.", m_name));
            }
            special = MA_SLOW;
        } else if !desc {
            log.add(ma_desc(chosen.desc, m_name));
        }
        desc = true;
    }
    if chosen.effect & MA_STUN != 0 {
        if chosen.power != 0 {
            stun_effect = chosen.power / 2 + rng.gen_range(1..=(chosen.power / 2).max(1));
        }
        if !desc {
            log.add(ma_desc(chosen.desc, m_name));
        }
        desc = true;
    }
    if chosen.effect & MA_WOUND != 0 {
        if rng.gen_range(0..100) < chosen.power {
            special |= MA_WOUND;
        }
        if !desc {
            log.add(ma_desc(chosen.desc, m_name));
        }
    }
    k = critical_norm_melee(
        ps,
        plev * rng.gen_range(1..=10),
        chosen.min_level,
        to_h,
        0,
        false,
        k,
        luck,
        rng,
        log,
    );
    let mut out = HandBlow {
        dam: k,
        stun: 0,
        slow: false,
        cut: special & MA_WOUND != 0,
    };
    if special & MA_KNEE != 0 && k + to_d < m_hp {
        log.add(format!("{} moans in agony!", capitalize(m_name)));
        out.stun = 7 + rng.gen_range(1..=13);
        resist_stun /= 3;
    }
    if special & (MA_FULL_SLOW | MA_SLOW) != 0 && k + to_d < m_hp {
        if !def.has("UNIQUE") && rng.gen_range(1..=plev.max(1)) > m_level && m_mspeed > 60 {
            log.add(format!("{} starts limping.", capitalize(m_name)));
            out.slow = true;
        }
    }
    if stun_effect != 0 && k + to_d < m_hp {
        let roll = m_level + resist_stun + 10;
        if plev > rng.gen_range(1..=roll.max(1)) {
            if out.stun == 0 {
                log.add(format!("{} is stunned.", capitalize(m_name)));
            }
            out.stun += stun_effect;
        }
    }
    let _ = gd;
    out
}

fn capitalize(s: &str) -> String {
    crate::zutil::capitalize(s)
}

/// do_nazgul (cmd1.cc:1779): hitting a Ringwraith shatters weapons and can
/// inflict the Black Breath. Returns true when the attack must stop.
fn do_nazgul(
    ctx: &mut InputCtx,
    weapon: &mut Option<item::Item>,
    def: &crate::data::MonsterDef,
    dmg: &mut i32,
    rng: &mut impl Rng,
) -> bool {
    if !def.has("NAZGUL") {
        return false;
    }
    let Some(w) = weapon.as_mut() else {
        return false;
    };
    let flags: HashSet<String> = item::item_flags(&ctx.gd, w)
        .iter()
        .map(|f| f.to_string())
        .collect();
    let ego = w.ego != 0;
    let artifact = w.artifact != 0;
    let mundane = !(flags.contains("SLAY_EVIL")
        || flags.contains("SLAY_UNDEAD")
        || flags.contains("KILL_UNDEAD"));
    let allow_shatter = !flags.contains("RES_MORGUL");
    if !ego && !artifact && allow_shatter {
        ctx.log.add("Your weapon *DISINTEGRATES*!");
        *dmg = 0;
        ctx.inv.equip[data::SLOT_WEAPON] = None;
        *weapon = None;
        return true;
    }
    if ego {
        if mundane {
            ctx.log
                .add("The Ringwraith is IMPERVIOUS to the mundane weapon.");
            *dmg = 0;
        }
        if rng.gen_range(0..100) < 25 && allow_shatter {
            ctx.log.add("Your weapon is destroyed!");
            ctx.inv.equip[data::SLOT_WEAPON] = None;
            *weapon = None;
            return true;
        }
    } else if artifact {
        if mundane {
            ctx.log
                .add("The Ringwraith is IMPERVIOUS to the mundane weapon.");
            *dmg = 0;
        }
        // apply_disenchant(INVEN_WIELD): artifacts resist 71% of the time.
        if w.to_h > 0 || w.to_d > 0 || w.to_a > 0 {
            if artifact && rng.gen_range(0..100) < 71 {
                ctx.log.add("Your weapon resists disenchantment!");
            } else {
                if w.to_h > 0 {
                    w.to_h -= 1;
                }
                if w.to_h > 5 && rng.gen_range(0..100) < 20 {
                    w.to_h -= 1;
                }
                if w.to_d > 0 {
                    w.to_d -= 1;
                }
                if w.to_d > 5 && rng.gen_range(0..100) < 20 {
                    w.to_d -= 1;
                }
                if w.to_a > 0 {
                    w.to_a -= 1;
                }
                if w.to_a > 5 && rng.gen_range(0..100) < 20 {
                    w.to_a -= 1;
                }
                ctx.log.add("Your weapon was disenchanted!");
                ctx.inv.equip[data::SLOT_WEAPON] = weapon.clone();
            }
        }
        if rng.gen_range(0..1000) == 0 && allow_shatter {
            ctx.log.add("Your weapon is destroyed!");
            ctx.inv.equip[data::SLOT_WEAPON] = None;
            *weapon = None;
            return true;
        }
    }
    if *dmg != 0 && rng.gen_range(0..100) < 25 {
        ctx.log.add("Your foe calls upon your soul!");
        ctx.log
            .add("You feel the Black Breath slowly draining you of life...");
        ctx.ps.black_breath = true;
    }
    false
}

/// One line of a `lib/file/dam_*.txt` table (files.cc get_rnd_line): the
/// first line is the count, `randint(count)` picks it and the buffer line
/// is skipped.
fn rnd_damage_line(file: &'static str, m_name: &str, rng: &mut impl Rng) -> String {
    let lines: Vec<&str> = file.lines().collect();
    let count: usize = lines.first().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
    let text = if count > 0 {
        lines
            .get(rng.gen_range(1..=count) + 1)
            .copied()
            .unwrap_or("You hit %s.")
    } else {
        "You hit %s."
    };
    let cap = crate::zutil::capitalize(m_name);
    text.replace("%^s", &cap).replace("%s", m_name)
}

/// flavored_attack (cmd1.cc:1503): percent-of-max-hp combat messages, with
/// the insanity dice swapping in a random `dam_*.txt` line.
fn flavored_attack(
    def: &crate::data::MonsterDef,
    m_name: &str,
    percent: i32,
    max_sanity: i32,
    sanity: i32,
    rng: &mut impl Rng,
) -> String {
    // These monsters never have flavoured combat msgs (cmd1.cc:2216).
    if "vwjmelX,.*".contains(def.glyph()) {
        return format!("You hit {}.", m_name);
    }
    let insanity = if max_sanity > 0 {
        (max_sanity - sanity).max(0) * 100 / max_sanity
    } else {
        0
    };
    let insane = rng.gen_range(0..100) < insanity;
    if !insane {
        return if percent < 5 {
            format!("You scratch {}.", m_name)
        } else if percent < 30 {
            format!("You hit {}.", m_name)
        } else if percent < 60 {
            format!("You wound {}.", m_name)
        } else if percent < 95 {
            format!("You cripple {}.", m_name)
        } else {
            format!("You demolish {}.", m_name)
        };
    }
    let file = if percent < 5 {
        DAM_NONE
    } else if percent < 30 {
        DAM_MED
    } else if percent < 60 {
        DAM_LOTS
    } else if percent < 95 {
        DAM_HUGE
    } else {
        DAM_XXX
    };
    rnd_damage_line(file, m_name, rng)
}

const DAM_NONE: &str = include_str!("../assets/data/dam_none.txt");
const DAM_MED: &str = include_str!("../assets/data/dam_med.txt");
const DAM_LOTS: &str = include_str!("../assets/data/dam_lots.txt");
const DAM_HUGE: &str = include_str!("../assets/data/dam_huge.txt");
const DAM_XXX: &str = include_str!("../assets/data/dam_xxx.txt");

/// teleport_away (spells1.cc): hurl a monster to a random empty floor cell.
fn teleport_monster_away(ctx: &mut InputCtx, e: Entity, rng: &mut impl Rng) {
    let occupied: HashSet<(i32, i32)> = ctx
        .monsters
        .iter()
        .filter(|(me, _, _)| *me != e)
        .map(|(_, _, p)| (p.x, p.y))
        .collect();
    for _ in 0..500 {
        let x = rng.gen_range(1..map::MAP_W - 1);
        let y = rng.gen_range(1..map::MAP_H - 1);
        if occupied.contains(&(x, y)) || !ctx.map.walkable(&ctx.gd, x, y) {
            continue;
        }
        ctx.commands.entity(e).insert(GridPos { x, y });
        return;
    }
}

/// The C++ tests the wielded artifact's name for `'Stormbringer'`
/// (cmd1.cc:1953/2842); the blade is forged specially when its unique dies.
fn weapon_is_stormbringer(inv: &Inventory) -> bool {
    inv.equip[data::SLOT_WEAPON]
        .as_ref()
        .is_some_and(|w| w.artifact_name == "'Stormbringer'")
}

fn melee_attack(ctx: &mut InputCtx, e: Entity) -> bool {
    let mut rng = crate::rng::current();
    // "Stop if friendly" (cmd1.cc:1950): only Stormbringer may strike an
    // ally, and only when the ally is clearly seen.
    {
        let (friendly, def_idx, mx, my) = {
            let (_, m, p) = ctx.monsters.get(e).expect("monster");
            (m.friendly, m.def, p.x, p.y)
        };
        if friendly {
            let totals = ctx.inv.totals_for(&ctx.gd, &ctx.ps);
            let visible = ctx.map.visible[Map::idx(mx, my)]
                && (!ctx.gd.monsters[def_idx].has("INVISIBLE")
                    || totals.see_invis
                    || ctx.ps.tim_invis > 0);
            if visible && ctx.ps.confuse == 0 && ctx.ps.stun == 0 && ctx.ps.image == 0 {
                let name = ctx.gd.monsters[def_idx].name.clone();
                if weapon_is_stormbringer(&ctx.inv) {
                    ctx.log
                        .add(format!("Your black blade greedily attacks {}!", name));
                } else {
                    ctx.log.add(format!("You stop to avoid hitting {}.", name));
                    return false;
                }
            }
        }
    }
    if ctx.inv.totals_for(&ctx.gd, &ctx.ps).never_blow {
        ctx.log.add("Your weapon refuses to strike!");
        return false;
    }
    // Attacking breaks the globe/disruption shield (cmd1.cc:1964).
    ctx.ps.invuln = 0;
    ctx.ps.disrupt_shield = 0;
    let weapon = ctx.inv.equip[data::SLOT_WEAPON].clone();
    let (dice, weight) = weapon
        .as_ref()
        .map(|w| (w.dice.clone(), ctx.gd.objects[w.def].weight))
        .unwrap_or_else(|| ("1d2".to_string(), 5));
    let wtval = weapon
        .as_ref()
        .map(|w| ctx.gd.objects[w.def].tval)
        .unwrap_or(0);
    let mastery = game::weaponmastery_skill(&ctx.ps, &ctx.inv, &ctx.gd);
    let slays = item::weapon_slays(&ctx.gd, &ctx.inv, None);
    let vorpal = slays.contains("VORPAL");
    let impact = slays.contains("IMPACT");
    let (_, m, mpos) = ctx.monsters.get_mut(e).expect("monster");
    let held = (
        m.held_artifact,
        m.held_object,
        m.items.clone(),
        m.gold,
        m.looted,
    );
    let def_idx = m.def;
    let name = game::hallucinated_name(&ctx.gd, &ctx.ps, def_idx, &mut rng);
    let def = ctx.gd.monsters[def_idx].clone();
    let mac = def.ac + m.ac_mod;
    let sleeping = !m.awake;
    let m_fear0 = m.fear > 0;
    let totals = ctx.inv.totals_for(&ctx.gd, &ctx.ps);
    let mut to_h = totals.to_h
        + game::adj_dex_th(ctx.ps.stats[crate::game::DEX])
        + game::adj_str_th(ctx.ps.stats[crate::game::STR])
        + game::tactic_info(ctx.ps.tactic).0;
    let chance = game::player_hit_chance(&ctx.ps, &ctx.inv, &ctx.gd);
    let blows = game::num_blows(&ctx.ps, &ctx.inv, &ctx.gd);
    let (mx, my) = (mpos.x, mpos.y);
    let mut killed = false;
    let mut extra_action = false;
    // Weapon specialization and Combat damage (xtra1.cc to_d_melee).
    // Mouse form divides p_ptr->to_d in mouse_calc (mimic.cc:67), but
    // calc_bonuses zeroes to_d at the top (xtra1.cc:2726) and the stat
    // bonus is only added later, so the original divides an already-zero
    // value: the stat-derived to-damage is NOT scaled down.
    let str_td = game::adj_str_td(ctx.ps.stats[crate::game::STR]);
    let mut damage_bonus = totals.to_d
        + ctx.ps.skill_scale(crate::skill::SK_COMBAT, 10)
        + str_td
        + game::tactic_info(ctx.ps.tactic).1
        // Eru/Sorcery forbidden-weapon malus also hits damage
        // (xtra1.cc:3727-3772 p_ptr->to_d; player_hit_chance takes to_h).
        - game::wield_malus(&ctx.ps, &ctx.inv, &ctx.gd);
    // A COULD2H weapon wielded together with a shield fights at a
    // penalty (xtra1.cc:3705-3723).
    if weapon
        .as_ref()
        .is_some_and(|w| item::item_flags(&ctx.gd, w).iter().any(|f| *f == "COULD2H"))
        && ctx.inv.equip[data::SLOT_SHIELD].is_some()
    {
        if let Some(w) = &weapon {
            let dd_ds = Dice::parse(&w.dice).map(|d| d.count * d.sides).unwrap_or(0);
            to_h -= w.to_h.abs() / 2;
            damage_bonus -= w.to_d.abs() / 2 + dd_ds / 2;
        }
    }
    // True Strike adds +15 to damage (xtra1.cc calc_bonuses).
    if ctx.ps.strike > 0 {
        damage_bonus += 15;
    }
    if let Some(sk) = mastery {
        damage_bonus += ctx.ps.skill(sk) / 2;
    }
    let vis = ctx.map.visible[Map::idx(mx, my)]
        && (!def.has("INVISIBLE") || totals.see_invis || ctx.ps.tim_invis > 0);
    // Backstab / stab-fleeing (cmd1.cc:1920-1931): only visible targets,
    // and only the first blow may be a backstab (cleared on hit or miss).
    let mut backstab = sleeping && vis && mastery.is_some();
    let mut stab_fleeing = !backstab && m_fear0 && vis && mastery.is_some();
    let hand_style = ctx.ps.melee_style != crate::skill::SK_MASTERY;
    let m_level = def.depth as i32;
    let brand_tval = matches!(
        wtval,
        data::TV_SHOT
            | data::TV_ARROW
            | data::TV_BOLT
            | data::TV_BOOMERANG
            | data::TV_HAFTED
            | data::TV_POLEARM
            | data::TV_SWORD
            | data::TV_AXE
            | data::TV_DIGGING
    );
    let mut weapon = weapon;
    let mut drain_left = 100i32;
    let mut drain_msg = true;
    let mut do_quake = false;
    // TR_CHAOTIC's effect is rolled on half the blows; when the roll
    // fails the previous effect stays (cmd1.cc declares it outside the
    // blow loop).
    let mut chaos_effect = 0;
    for blow_idx in 0..blows {
        if !test_hit_vis(chance, mac, vis, totals.luck, &mut rng) {
            ctx.log.add(format!("You miss {}.", name));
            backstab = false;
            continue;
        }
        let (m_hp, m_mspeed, m_max_hp, m_quest) = ctx
            .monsters
            .get(e)
            .map(|(_, m, _)| (m.hp, m.mspeed_mod, m.max_hp.max(1), m.quest))
            .unwrap_or((0, 0, 1, false));
        // TR_CHAOTIC (cmd1.cc:2041): a chaos effect on half the blows.
        if slays.contains("CHAOTIC") && rng.gen_range(0..2) == 0 {
            if rng.gen_range(0..5) < 3 {
                chaos_effect = 1;
            } else if rng.gen_range(0..250) == 0 {
                chaos_effect = 2;
            } else if rng.gen_range(0..10) != 0 {
                chaos_effect = 3;
            } else if rng.gen_range(0..2) == 0 {
                chaos_effect = 4;
            } else {
                chaos_effect = 5;
            }
        }
        // TR_VAMPIRIC (cmd1.cc:2020): remember the victim's hp for the drain.
        let drain_result = if (slays.contains("VAMPIRIC") || chaos_effect == 1)
            && !(def.has("UNDEAD") || def.has("NONLIVING"))
        {
            m_hp
        } else {
            0
        };
        let mut dmg;
        let special_cut;
        let mut special_pois = false;
        let mut hand_stun = 0i32;
        let mut hand_slow = false;
        let mut critted = false;
        let mut stop_attack = false;
        if hand_style {
            // Monk / bear-form martial arts (cmd1.cc py_attack_hand).
            let hb = py_attack_hand(
                &ctx.gd,
                &mut ctx.ps,
                &def,
                &name,
                m_hp,
                m_level,
                m_mspeed,
                to_h,
                damage_bonus,
                totals.luck,
                &mut rng,
                &mut ctx.log,
            );
            dmg = hb.dam + damage_bonus;
            hand_stun = hb.stun;
            hand_slow = hb.slow;
            special_cut = hb.cut;
        } else {
            let base = Dice::parse(&dice).map(|d| d.roll(&mut rng)).unwrap_or(1);
            // Slay/brand multipliers: only real weapons and ammo brand
            // (cmd1.cc tot_dam_aux).
            let (mult, m_pois, m_cut) = item::dam_aux(&slays, &def, &mut rng);
            let mut weapon_dmg = if brand_tval {
                (base * mult).max(1)
            } else {
                base
            };
            special_pois = m_pois;
            special_cut = m_cut;
            // Backstabbing (cmd1.cc:2098-2108): the striking bonus applies
            // on the blow that connects, not to the printed message.
            if backstab {
                weapon_dmg += weapon_dmg * ctx.ps.skill_scale(crate::skill::SK_BACKSTAB, 100) / 100;
            } else if stab_fleeing {
                weapon_dmg += weapon_dmg * ctx.ps.skill_scale(crate::skill::SK_BACKSTAB, 70) / 100;
            }
            // Impact weapons mark the spot for a quake (cmd1.cc).
            if impact && (weapon_dmg > 50 || rng.gen_range(0..7) == 0) {
                do_quake = true;
            }
            let is_light_sword = wtval == data::TV_SWORD && weight < 50;
            dmg = critical_norm_melee(
                &mut ctx.ps,
                weight,
                0,
                to_h,
                totals.crit,
                is_light_sword,
                weapon_dmg,
                totals.luck,
                &mut rng,
                &mut ctx.log,
            );
            critted = dmg != weapon_dmg;
            if vorpal && rng.gen_range(1..=6) == 1 {
                ctx.log.add(format!("Your weapon cuts deep into {}!", name));
                let step = dmg;
                while rng.gen_range(1..=4) == 1 {
                    dmg += step;
                }
            }
            // Tulkas grants mighty blows to the faithful (xtra1.cc):
            // the weapon's own plus and the melee skill are added again,
            // doubled when the favour roll succeeds (cmd1.cc:2155-2164).
            let weapon_to_d = weapon.as_ref().map(|w| w.to_d).unwrap_or(0);
            let to_d_melee = ctx.ps.skill_scale(crate::skill::SK_COMBAT, 10)
                + game::tactic_info(ctx.ps.tactic).1;
            if ctx.ps.god == 3 && ctx.ps.praying {
                let wis = game::stat_index(ctx.ps.stats[game::WIS]) as i32;
                let chance = (wis * 130 / 37 - m_level).max(0);
                let extra = (weapon_to_d + to_d_melee).max(0);
                if rng.gen_range(0..100) < chance && ctx.ps.grace > 1000 {
                    ctx.log
                        .add("You feel the hand of Tulkas helping your blow.");
                    dmg += 2 * extra;
                } else {
                    dmg += extra;
                }
            }
            // Player damage bonuses are added after the critical
            // (cmd1.cc: `k += p_ptr->to_d + p_ptr->to_d_melee`), and the
            // total is clamped to zero (cmd1.cc:2209-2210).
            dmg = (dmg + damage_bonus).max(0);
            // Ringwraiths shatter mundane weapons (cmd1.cc do_nazgul).
            if do_nazgul(ctx, &mut weapon, &def, &mut dmg, &mut rng) {
                stop_attack = true;
            }
        }
        // Melkor can curse the victim for his faithful (cmd1.cc:2182).
        if ctx.ps.god == 4 && ctx.ps.praying {
            if let Some(sp) = ctx.gd.spell_by_name("Curse") {
                let lv = crate::spell::get_level_s(&ctx.ps, &ctx.inv, &ctx.gd, sp, 100);
                if lv >= 10 {
                    let mlev = ctx
                        .monsters
                        .get(e)
                        .map(|(_, m, _)| game::monster_level(&ctx.gd, m))
                        .unwrap_or(1);
                    let chance =
                        (game::wisdom_scale(&ctx.ps, 30) * lv / mlev.max(1)).max(1);
                    if ctx.ps.grace > 5000 && rng.gen_range(0..100) < chance {
                        if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                            game::melkor_curse_monster(
                                &ctx.gd,
                                &ctx.ps,
                                &ctx.inv,
                                &mut m,
                                &mut ctx.log,
                            );
                        }
                    }
                }
            }
        }
        {
            let (_, mut m, _) = ctx.monsters.get_mut(e).expect("monster");
            if critted
                && wtval == data::TV_HAFTED
                && weight > 50
                && ctx.ps.skill(crate::skill::SK_STUN) > 0
                && rng.gen_range(0..100) < ctx.ps.skill(crate::skill::SK_STUN)
            {
                let add = if m.stun > 0 {
                    ctx.ps.skill_scale(crate::skill::SK_STUN, 30) + 10
                } else {
                    ctx.ps.skill_scale(crate::skill::SK_STUN, 60) + 20
                };
                m.stun = (m.stun + add).min(200);
                ctx.log.add(format!("You stun the {}!", name));
            }
            if hand_stun > 0 {
                m.stun = (m.stun + hand_stun).min(200);
            }
            if hand_slow && m.mspeed_mod > -60 {
                m.mspeed_mod -= 10;
            }
            // Attacking a neutral monster provokes it (cmd1.cc:2283).
            if m.neutral {
                m.neutral = false;
                m.awake = true;
                ctx.log.add(format!("The {} gets angry!", name));
            }
            // Attacking disturbs the monster (cmd1.cc `m_ptr->csleep = 0`).
            m.awake = true;
            m.hp -= dmg;
            game::clamp_no_death(&ctx.gd, &mut m);
            // A blow that brings the monster to 0 hp kills it right away
            // (cmd1.cc py_attack: the death is processed in the same turn).
            if m.hp <= 0 {
                killed = true;
            }
        }
        // attack_special (cmd1.cc:1551): bleeding and poison messages.
        if dmg > 0 && special_cut && !def.has("NO_CUT") && rng.gen_range(0..100) >= m_level {
            let already = ctx
                .monsters
                .get(e)
                .map(|(_, m, _)| m.cut > 0)
                .unwrap_or(false);
            ctx.log.add(format!(
                "{} is bleeding{}.",
                capitalize(&name),
                if already { " more strongly" } else { "" }
            ));
            if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                m.cut += dmg * 2;
            }
        }
        if dmg > 0 && special_pois && !def.has("IM_POIS") {
            let amount = if def.has("SUSCEP_POIS") {
                dmg * 2
            } else if rng.gen_range(0..100) >= m_level {
                dmg
            } else {
                0
            };
            if amount > 0 {
                let already = ctx
                    .monsters
                    .get(e)
                    .map(|(_, m, _)| m.poison > 0)
                    .unwrap_or(false);
                ctx.log.add(format!(
                    "{} is {}poisoned.",
                    capitalize(&name),
                    if already { "more " } else { "" }
                ));
                if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                    m.poison += amount;
                }
            }
        }
        // Vampiric drain (cmd1.cc:2255), capped at 100 hp per attack.
        if drain_result != 0 {
            let after = ctx
                .monsters
                .get(e)
                .map(|(_, m, _)| m.hp)
                .unwrap_or(drain_result);
            let delta = drain_result - after;
            let sides = delta / 6;
            if sides > 0 && drain_left > 0 {
                let mut heal = Dice {
                    count: 4,
                    sides,
                    bonus: 0,
                }
                .roll(&mut rng);
                if heal > drain_left {
                    heal = drain_left;
                }
                drain_left -= heal;
                if heal > 0 {
                    if drain_msg {
                        ctx.log
                            .add(format!("Your weapon drains life from {}!", name));
                        drain_msg = false;
                    }
                    ctx.ps.hp = (ctx.ps.hp + heal).min(ctx.ps.max_hp);
                }
            }
        }
        // Confusion attack / chaos effects (cmd1.cc:2332-2360).  A scroll
        // of Monster Confusion makes the hands glow for the next blow; the
        // glow is consumed even if the victim resists.
        if ctx.repeat.confusing || chaos_effect == 3 {
            if ctx.repeat.confusing {
                ctx.repeat.confusing = false;
                ctx.log.add("Your hands stop glowing.");
            }
            if def.has("NO_CONF") || rng.gen_range(0..100) < m_level {
                ctx.log.add(format!("{} is unaffected.", capitalize(&name)));
            } else {
                ctx.log
                    .add(format!("{} appears confused.", capitalize(&name)));
                let combat = ctx.ps.skill(crate::skill::SK_COMBAT).max(1);
                let add = 10 + rng.gen_range(0..combat) / 5;
                if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                    m.confused += add;
                }
            }
        } else if chaos_effect == 4 {
            ctx.log.add(format!("{} disappears!", capitalize(&name)));
            teleport_monster_away(ctx, e, &mut rng);
            stop_attack = true;
        } else if chaos_effect == 5
            && ctx.map.walkable(&ctx.gd, mx, my)
            && rng.gen_range(0..90) > m_level
        {
            if def.unique || m_quest {
                ctx.log.add(format!("{} is unaffected.", capitalize(&name)));
            } else if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
                if game::poly_monster(&ctx.gd, &mut m, &mut rng).is_some() {
                    ctx.log.add(format!("{} changes!", capitalize(&name)));
                } else {
                    ctx.log.add(format!("{} resists.", capitalize(&name)));
                }
            }
        }
        if chaos_effect == 2 {
            do_quake = true;
        }
        // Message (cmd1.cc:2212-2249): backstabs and fleeing backstabs
        // replace the flavoured hit message; the backstab flag is spent.
        let percent = 100 * dmg / m_max_hp;
        if backstab {
            ctx.log
                .add(format!("You cruelly stab the helpless, sleeping {}!", name));
            backstab = false;
        } else if stab_fleeing {
            ctx.log.add(format!("You backstab the fleeing {}!", name));
        } else {
            ctx.log.add(flavored_attack(
                &def,
                &name,
                percent,
                ctx.ps.max_sanity,
                ctx.ps.sanity,
                &mut rng,
            ));
        }
        if stop_attack {
            break;
        }
        // TR_CLONE weapons may clone the victim (cmd1.cc:2198; "of the
        // Dawn"/"of Angmar").
        let clone_weapon = weapon
            .as_ref()
            .map(|w| item::item_flags(&ctx.gd, w).iter().any(|f| *f == "CLONE"))
            .unwrap_or(false);
        if clone_weapon && rng.gen_bool(0.30) {
            let occupied: HashSet<(i32, i32)> =
                ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
            for _ in 0..18 {
                let (ox, oy) = (mx + rng.gen_range(-1..=1), my + rng.gen_range(-1..=1));
                if (ox, oy) == (mx, my)
                    || occupied.contains(&(ox, oy))
                    || !ctx.map.walkable(&ctx.gd, ox, oy)
                {
                    continue;
                }
                ctx.log
                    .add(format!("Oh no! Your weapon clones the {}!", name));
                // multiply_monster(clone=true): the copy is marked
                // SM_CLONED.
                game::spawn_clone(
                    &mut ctx.commands,
                    &ctx.gd,
                    &ctx.tiles,
                    def_idx,
                    ox,
                    oy,
                    &mut rng,
                );
                break;
            }
        }
        // Demon Blade (xtra2.cc set_project): each landed blow also
        // projects the stored damage type at the victim.
        if ctx.ps.tim_project > 0 {
            let proj_dead = demon_blade_project(ctx, mx, my, &mut rng);
            for (de, ddef, dcell) in proj_dead {
                if de == e {
                    killed = true;
                    continue;
                }
                // A POSSESSOR soul deincarnates instead of dying
                // (monster3.cc ai_deincarnate).
                if let Ok((_, mut dm, _)) = ctx.monsters.get_mut(de) {
                    let mut rng = crate::rng::current();
                    if game::deincarnate_monster(&ctx.gd, &mut dm, None, &mut ctx.log, &mut rng) {
                        continue;
                    }
                }
                let held2 = ctx
                    .monsters
                    .get(de)
                    .map(|(_, m, _)| {
                        (
                            m.held_artifact,
                            m.held_object,
                            m.items.clone(),
                            m.gold,
                            m.looted,
                        )
                    })
                    .unwrap_or((None, None, Vec::new(), 0, false));
                let quest_left = ctx
                    .monsters
                    .iter()
                    .filter(|(qe, m, _)| *qe != de && m.quest)
                    .count() as i32;
                    let enemy_left = ctx
                        .monsters
                        .iter()
                        .filter(|(qe, m, _)| *qe != de && !m.friendly && !m.companion && !m.neutral)
                        .count() as i32;
                game::kill_monster(
                    &mut ctx.commands,
                    &ctx.gd,
                    &ctx.tiles,
                    &mut ctx.stacks,
                    de,
                    ddef,
                    dcell,
                    held2.0,
                    held2.1,
                    held2.2,
                    held2.3,
                    held2.4,
                    &mut ctx.ps,
                    &mut ctx.plot,
                    &mut ctx.map,
                    quest_left,
                    enemy_left,
                    &mut ctx.created.0,
                    &mut ctx.inv,
                    &mut ctx.log,
                    &mut rng,
                    false,
                );
            }
            // The melee victim may have died to the projection.
            if ctx
                .monsters
                .get(e)
                .map(|(_, m, _)| m.hp <= 0)
                .unwrap_or(true)
            {
                killed = true;
            }
        }
        if killed {
            // Spread Blows: unused attacks carry over to another target
            // (AB_SPREAD_BLOWS; the caller skips the world turn).
            if ctx.ps.has_ability(crate::skill::AB_SPREAD_BLOWS) && blow_idx + 1 < blows {
                extra_action = true;
            }
            break;
        }
    }
    // A chaotic quake waits until the attack ends (cmd1.cc do_quake).
    if do_quake && ctx.ps.depth > 0 && ctx.ps.depth < game::PLOT_DEPTH_BASE {
        game::earthquake(&mut ctx.map, &ctx.gd, mx, my, 10, &mut ctx.log, &mut rng);
    }
    if let Ok((_, mut m, _)) = ctx.monsters.get_mut(e) {
        m.awake = true;
    }
    // Monster auras burn/jolt the attacker (cmd1.cc touch_zap_player).
    if !killed {
        for (aura, elem, msg) in [
            ("AURA_FIRE", "FIRE", "You are suddenly very hot!"),
            ("AURA_ELEC", "ELEC", "You get zapped!"),
        ] {
            if !def.has(aura) {
                continue;
            }
            let rlev = def.depth.max(1) as i32;
            let mut d = Dice {
                count: 1 + rlev / 26,
                sides: 1 + rlev / 17,
                bonus: 0,
            }
            .roll(&mut rng);
            if totals.immunities.contains(elem) {
                continue;
            }
            if (elem == "FIRE" && ctx.ps.oppose_fire > 0)
                || (elem == "ELEC" && ctx.ps.oppose_elec > 0)
            {
                d = (d + 2) / 3;
            }
            if totals.resists.contains(elem) {
                d = (d + 2) / 3;
            }
            if elem == "FIRE" && totals.sens_fire {
                d = (d + 2) * 2;
            }
            if d > 0 {
                let d = game::symbiote_intercept(&ctx.ps, &mut ctx.inv, d, &mut rng, &mut ctx.log);
                let rem = game::absorb_damage(&mut ctx.ps, d, &mut ctx.log);
                ctx.ps.hp -= rem;
                ctx.log.add(format!("{} ({})", msg, d));
                if ctx.ps.hp <= 0 && game::player_death_check(&mut ctx.ps, &ctx.gd, &mut ctx.log) {
                    ctx.log.add("You die.");
                    ctx.next.set(AppState::Dead);
                    return extra_action;
                }
            }
        }
    }
    if killed {
        // A POSSESSOR soul deincarnates instead of dying.
        if let Ok((_, mut dm, _)) = ctx.monsters.get_mut(e) {
            let mut rng = crate::rng::current();
            if game::deincarnate_monster(&ctx.gd, &mut dm, None, &mut ctx.log, &mut rng) {
                return extra_action;
            }
        }
        // Quest-flagged monsters still alive (for the thieves' victory).
        let quest_left = ctx
            .monsters
            .iter()
            .filter(|(qe, m, _)| *qe != e && m.quest)
            .count() as i32;
            let enemy_left = ctx
                .monsters
                .iter()
                .filter(|(qe, m, _)| *qe != e && !m.friendly && !m.companion && !m.neutral)
                .count() as i32;
        game::kill_monster(
            &mut ctx.commands,
            &ctx.gd,
            &ctx.tiles,
            &mut ctx.stacks,
            e,
            def_idx,
            (mx, my),
            held.0,
            held.1,
            held.2,
            held.3,
            held.4,
            &mut ctx.ps,
            &mut ctx.plot,
            &mut ctx.map,
            quest_left,
            enemy_left,
            &mut ctx.created.0,
            &mut ctx.inv,
            &mut ctx.log,
            &mut rng,
            false,
        );
        // The automatizer sweeps the death pile on the next frame (the
        // drop entities are spawned through deferred Commands), and the
        // unique kill is noted (xtra2.cc:3213).
        ctx.squeltch_pending.push((mx, my));
        if def.unique {
            let (turn, depth) = (ctx.ps.turn, ctx.ps.depth);
            ctx.notes.add_unique(&def.name, turn, depth);
        }
    } else {
        // The carried symbiote joins the melee with its own blows
        // (cmd1.cc carried_monster_attack).
        symbiote_attack(ctx, e, &mut rng);
    }
    extra_action
}

/// The carried symbiote strikes after the player's melee (cmd1.cc
/// carried_monster_attack): its own blows at the stored monster level.
fn symbiote_attack(ctx: &mut InputCtx, e: Entity, rng: &mut impl rand::Rng) {
    let Some(sym) = ctx.inv.equip[data::SLOT_SYMBIOTE].clone() else {
        return;
    };
    let form_def = sym.note as usize;
    if form_def >= ctx.gd.monsters.len() {
        return;
    }
    let form = ctx.gd.monsters[form_def].clone();
    let mon_level = sym.mon_level.max(1);
    let to_d = ctx.inv.totals_for(&ctx.gd, &ctx.ps).to_d;
    let (_, mut m, mpos) = ctx.monsters.get_mut(e).expect("monster");
    let held = (
        m.held_artifact,
        m.held_object,
        m.items.clone(),
        m.gold,
        m.looted,
    );
    let def_idx = m.def;
    let def = ctx.gd.monsters[def_idx].clone();
    let name = game::hallucinated_name(&ctx.gd, &ctx.ps, def_idx, rng);
    let mac = def.ac + m.ac_mod;
    let (mx, my) = (mpos.x, mpos.y);
    let vis = ctx.map.visible[Map::idx(mx, my)]
        && (!def.has("INVISIBLE")
            || ctx.ps.tim_invis > 0
            || ctx.inv.totals_for(&ctx.gd, &ctx.ps).see_invis);
    if !vis {
        ctx.log.add("You hear noise.");
    }
    let mut killed = false;
    let mut blinked = false;
    let mut quake = false;
    // At most four blows (cmd1.cc carried_monster_attack).
    for blow in form.blows.iter().take(4) {
        if m.hp <= 0 {
            break;
        }
        let effect = blow.effect.as_str();
        let method = blow.method.as_str();
        let touched = matches!(
            method,
            "HIT"
                | "TOUCH"
                | "PUNCH"
                | "KICK"
                | "CLAW"
                | "BITE"
                | "STING"
                | "BUTT"
                | "CRUSH"
                | "ENGULF"
                | "CHARGE"
                | "CRAWL"
        );
        if !game::monster_check_hit(game::blow_power(effect), mon_level, mac, rng) {
            if touched && vis {
                ctx.log
                    .add(format!("The {} misses the {}.", form.name, name));
            }
            continue;
        }
        let dice = Dice::parse(&blow.dice).unwrap_or(Dice {
            count: 1,
            sides: 2,
            bonus: 0,
        });
        let mut dmg = dice.roll(rng) + to_d;
        let gf = match effect {
            "POISON" | "DISEASE" => "POIS",
            "UN_BONUS" | "UN_POWER" | "ABOMINATION" => "DISENCHANT",
            "ACID" => "ACID",
            "ELEC" => "ELEC",
            "FIRE" => "FIRE",
            "COLD" => "COLD",
            "CONFUSE" | "HALLU" => "CONFUSION",
            "TERRIFY" => "TURN_ALL",
            "PARALYZE" => "OLD_SLEEP",
            "EXP_10" | "EXP_20" | "EXP_40" | "EXP_80" => "NETHER",
            "TIME" => "TIME",
            "EAT_ITEM" | "EAT_GOLD" | "EAT_FOOD" | "EAT_LITE" => {
                dmg = 0;
                if rng.gen_range(0..2) == 0 {
                    blinked = true;
                }
                ""
            }
            "" | "HURT" | "SANITY" => {
                dmg -= dmg * mac.clamp(0, 150) / 250;
                ""
            }
            _ => "",
        };
        if !gf.is_empty() {
            let power = if gf == "OLD_SLEEP" {
                mon_level as i32
            } else {
                dmg
            };
            let eff = game::gf_monster_effect(&def, gf, power, mon_level as i32, true, rng);
            m.poison += eff.poison;
            m.cut += eff.cut;
            m.stun += eff.stun;
            m.confused += eff.conf;
            if eff.slow {
                m.mspeed_mod -= 10;
            }
            dmg = eff.dam;
        }
        if effect == "SHATTER"
            && dmg > 23
            && ctx.ps.depth > 0
            && ctx.ps.depth < game::PLOT_DEPTH_BASE
        {
            quake = true;
        }
        let dealt = dmg.max(0);
        m.hp -= dealt;
        game::clamp_no_death(&ctx.gd, &mut m);
        if dealt > 0 && vis {
            ctx.log
                .add(format!("Your {} hits the {}. ({})", form.name, name, dealt));
        }
        // Touched monsters burn or zap the symbiote's host (and thus the
        // player, cmd1.cc:939).
        if touched {
            if def.has("AURA_FIRE") && !form.has("IM_FIRE") {
                let d = Dice {
                    count: 1 + (def.depth as i32) / 26,
                    sides: 1 + (def.depth as i32) / 17,
                    bonus: 0,
                }
                .roll(rng);
                if vis {
                    ctx.log.add("You are suddenly very hot!");
                }
                let d = game::symbiote_intercept(&ctx.ps, &mut ctx.inv, d, rng, &mut ctx.log);
                let rem = game::absorb_damage(&mut ctx.ps, d, &mut ctx.log);
                ctx.ps.hp -= rem;
            }
            if def.has("AURA_ELEC") && !form.has("IM_ELEC") {
                let d = Dice {
                    count: 1 + (def.depth as i32) / 26,
                    sides: 1 + (def.depth as i32) / 17,
                    bonus: 0,
                }
                .roll(rng);
                if vis {
                    ctx.log.add("You get zapped!");
                }
                let d = game::symbiote_intercept(&ctx.ps, &mut ctx.inv, d, rng, &mut ctx.log);
                let rem = game::absorb_damage(&mut ctx.ps, d, &mut ctx.log);
                ctx.ps.hp -= rem;
            }
        }
        if m.hp <= 0 {
            killed = true;
            break;
        }
    }
    drop(m);
    if quake {
        game::earthquake(&mut ctx.map, &ctx.gd, mx, my, 8, &mut ctx.log, rng);
    }
    if blinked && ctx.ps.hp > 0 {
        ctx.log
            .add(format!("You and the {} flee laughing!", form.name));
        let occupied: HashSet<(i32, i32)> =
            ctx.monsters.iter().map(|(_, _, p)| (p.x, p.y)).collect();
        let mut rng2 = crate::rng::current();
        if let Ok(mut pp) = ctx.player.single_mut() {
            teleport_player_drag(&ctx.gd, &ctx.map, &ctx.monsters, &mut ctx.commands, &mut pp, &occupied, map::MAX_SIGHT * 2 + 5, &mut rng2);
            ctx.ps.last_teleport = Some((pp.x, pp.y));
        }
        ctx.turn.fov_dirty = true;
    }
    if killed {
        let mut rng = crate::rng::current();
        // A POSSESSOR soul deincarnates instead of dying.
        if let Ok((_, mut dm, _)) = ctx.monsters.get_mut(e) {
            if game::deincarnate_monster(&ctx.gd, &mut dm, None, &mut ctx.log, &mut rng) {
                return;
            }
        }
        let quest_left = ctx
            .monsters
            .iter()
            .filter(|(qe, m, _)| *qe != e && m.quest)
            .count() as i32;
            let enemy_left = ctx
                .monsters
                .iter()
                .filter(|(qe, m, _)| *qe != e && !m.friendly && !m.companion && !m.neutral)
                .count() as i32;
        game::kill_monster(
            &mut ctx.commands,
            &ctx.gd,
            &ctx.tiles,
            &mut ctx.stacks,
            e,
            def_idx,
            (mx, my),
            held.0,
            held.1,
            held.2,
            held.3,
            held.4,
            &mut ctx.ps,
            &mut ctx.plot,
            &mut ctx.map,
            quest_left,
            enemy_left,
            &mut ctx.created.0,
            &mut ctx.inv,
            &mut ctx.log,
            &mut rng,
            false,
        );
    }
}

/// Tunnel into a wall (Ctrl+direction, as in the roguelike keyset),
/// or disarm a known trap in that direction.
fn tunnel(ctx: &mut InputCtx, dx: i32, dy: i32) {
    let Ok(pos) = ctx.player.single() else {
        return;
    };
    let (nx, ny) = (pos.x + dx, pos.y + dy);
    if !Map::in_bounds(nx, ny) {
        return;
    }
    if ctx.ps.wild_mode {
        return;
    }
    let t = ctx.map.terrain_at(nx, ny);
    if t == map::T_TRAP && ctx.map.known_traps.contains(&Map::idx(nx, ny)) {
        disarm_trap(ctx, nx, ny);
        return;
    }
    // No tunnelling through air (cmd2.cc do_cmd_tunnel).
    if ctx.gd.terrain(t).is_floor && !map::is_closed_door(t) {
        ctx.log.add("You cannot tunnel through air.");
        return;
    }
    // A monster is in the way.
    if ctx
        .monsters
        .iter()
        .any(|(_, m, p)| p.x == nx && p.y == ny && m.hp > 0)
    {
        consume_turn(ctx);
        ctx.log.add("There is a monster in the way!");
        let e = ctx
            .monsters
            .iter()
            .find(|(_, _, p)| p.x == nx && p.y == ny)
            .map(|(e, _, _)| e);
        if let Some(e) = e {
            melee_attack(ctx, e);
        }
        return;
    }
    tunnel_aux(ctx, nx, ny);
}

/// Use a staircase: delta +1 = down, -1 = up.  On a plot-quest level the
/// up staircase returns to the town the quest was entered from; in the
/// wilderness `<` starts travel and `>` enters the current world cell.
fn use_stairs(ctx: &mut InputCtx, delta: i32, confirmed: bool) {
    let Ok(pos) = ctx.player.single() else {
        return;
    };
    // World overview: `>` enters the current cell (dungeon or area).
    if ctx.ps.wild_mode {
        if delta > 0 {
            enter_wild_cell(ctx);
        }
        return;
    }
    let t = ctx.map.terrain_at(pos.x, pos.y);
    let idx = Map::idx(pos.x, pos.y);
    if ctx.ps.depth >= game::PLOT_DEPTH_BASE {
        if delta < 0 && t == T_STAIRS_UP {
            quest_exit(ctx);
        } else if delta > 0 && t != T_STAIRS_DOWN {
            ctx.log.add("I see no down staircase here.");
        }
        return;
    }
    // Dungeon levels.
    if ctx.ps.depth > 0 {
        let d_idx = ctx.ps.dungeon;
        let branch_parent = ctx.gd.dungeon(d_idx).branch_parent;
        let is_bottom = ctx.ps.depth == ctx.gd.dungeon(d_idx).maxdepth;
        let force_down = ctx.gd.dungeon(d_idx).has("FORCE_DOWN");
        let special = ctx.map.special.get(&idx).copied();
        if delta > 0 {
            // Unique levels ask y/n at once, before any stair handling
            // (cmd2.cc:372); other levels use the confirm_stairs option.
            if !confirmed && game::level_has_flag(&ctx.gd, &ctx.ps, "ASK_LEAVE") {
                *ctx.modal = Modal::ConfirmLeave {
                    delta: 1,
                    unique: true,
                };
                return;
            }
            if !matches!(t, T_STAIRS_DOWN | T_WAY_MORE | T_SHAFT_DOWN) {
                // Probability Travel lets you sink through the floor
                // (cmd2.cc go_down, p_ptr->prob_travel).
                let easy = !ctx.gd.dungeon(d_idx).has("NO_EASY_MOVE");
                if ctx.ps.prob_travel > 0 && easy && ctx.ps.depth < ctx.gd.dungeon(d_idx).maxdepth {
                    ctx.log.add("You sink through the floor.");
                    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth + 1));
                    return;
                }
                ctx.log.add("I see no down staircase here.");
                return;
            }
            // ask_leave(): the confirm_stairs option (cmd2.cc:365).
            if !confirmed && ctx.options.confirm_stairs {
                *ctx.modal = Modal::ConfirmLeave {
                    delta: 1,
                    unique: false,
                };
                return;
            }
            // The portal Melkor's death opened back to Arda
            // (q_ultrag.cc cave_set_feat(FEAT_MORE) at the player).
            if special == Some(map::SPECIAL_SURFACE) {
                ctx.log.add("You step through the portal to Arda.");
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Surface);
                return;
            }
            if let Some(branch) = special {
                if ctx.gd.dungeon(branch).min_plev > ctx.ps.level {
                    ctx.log
                        .add("You don't feel yourself experienced enough to go there...");
                    return;
                }
                let mind = ctx.gd.dungeon(branch).mindepth;
                let text = ctx.gd.dungeon(branch).text.clone();
                ctx.log.add(format!("You go into {}", text));
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Dungeon(branch, mind));
                return;
            }
            // The 149th level of the Void is sealed by a barrier of pure
            // magic; only an ULTIMATE artifact (a weapon imbued with the
            // Flame Imperishable) lets the player pass (q_ultrag.cc).
            if d_idx == 11 && ctx.ps.depth == 149 {
                let ultimate = ctx.inv.equip.iter().flatten().any(|it| {
                    item::item_flags(&ctx.gd, it)
                        .iter()
                        .any(|f| *f == "ULTIMATE")
                });
                if !ultimate {
                    ctx.log.add(
                        "It seems the level is protected by an impassable barrier of pure magic.",
                    );
                    ctx.log
                        .add("Only the most powerful magic could remove it. You will need to use");
                    ctx.log.add(
                        "the Flame Imperishable to pass. The source of Eru Iluvatar's own power.",
                    );
                    return;
                }
                ctx.log
                    .add("The power of the Flame Imperishable shatters the magical barrier.");
                ctx.log.add("The way before you is free.");
            }
            if is_bottom && !force_down {
                ctx.log.add("There are no more stairs leading down.");
                return;
            }
            let maxdepth = ctx.gd.dungeon(d_idx).maxdepth;
            match t {
                // Shaft down: 1-3 levels (cmd2.cc go_down_many).
                T_SHAFT_DOWN => {
                    let steps = crate::rng::current().gen_range(1..=3);
                    ctx.log.add("You enter a maze of down staircases.");
                    ctx.turn.pending_stair = Some(t);
                    ctx.turn.pending =
                        Some(game::Goto::Depth((ctx.ps.depth + steps).min(maxdepth)));
                }
                T_WAY_MORE => {
                    ctx.log.add("You enter the next area.");
                    ctx.turn.pending_stair = Some(t);
                    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth + 1));
                }
                _ => {
                    ctx.log.add("You enter a maze of down staircases.");
                    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth + 1));
                }
            }
        } else {
            // Unique levels ask before leaving forever (cmd2.cc ask_leave
            // on DF_ASK_LEAVE or the level's @:...:ASK_LEAVE flag).
            if !confirmed && game::level_has_flag(&ctx.gd, &ctx.ps, "ASK_LEAVE") {
                *ctx.modal = Modal::ConfirmLeave {
                    delta: -1,
                    unique: true,
                };
                return;
            }
            if !matches!(t, T_STAIRS_UP | T_WAY_LESS | T_SHAFT_UP) {
                // Probability Travel lets you rise through the ceiling
                // unless a powerful force prevents it (cmd2.cc go_up).
                let easy = !ctx.gd.dungeon(d_idx).has("NO_EASY_MOVE");
                if ctx.ps.prob_travel > 0 && easy && ctx.ps.depth > ctx.gd.dungeon(d_idx).mindepth {
                    ctx.log.add("You rise through the ceiling.");
                    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth - 1));
                    return;
                }
                ctx.log.add("I see no up staircase here.");
                return;
            }
            // ask_leave(): the confirm_stairs option (cmd2.cc:145).
            if !confirmed && ctx.options.confirm_stairs {
                *ctx.modal = Modal::ConfirmLeave {
                    delta: -1,
                    unique: false,
                };
                return;
            }
            // The Void can only be left by defeating Melkor
            // (q_ultrag.cc stair hooks).
            if d_idx == 11 && ctx.ps.depth == 128 {
                ctx.log.add("The portal to Arda is now closed.");
                return;
            }
            if d_idx == 11 && ctx.ps.depth == 150 {
                ctx.log
                    .add("The barrier seems to be impenetrable from this side.");
                ctx.log.add("You will have to move on.");
                return;
            }
            if let Some(special) = special {
                // Branch staircase back to the parent dungeon.
                if let Some((parent, pdepth)) = branch_parent {
                    if parent == special {
                        autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Dungeon(parent, pdepth));
                        return;
                    }
                }
                let maxd = ctx.gd.dungeon(special).maxdepth;
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Dungeon(special, maxd));
                return;
            }
            let mindepth = ctx.gd.dungeon(d_idx).mindepth;
            if ctx.ps.depth <= mindepth {
                ctx.log.add("You leave the dungeon.");
                autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Surface);
            } else {
                match t {
                    // Shaft up: 2-4 levels (cmd2.cc go_up_many), clamped.
                    T_SHAFT_UP => {
                        let steps = crate::rng::current().gen_range(2..=4);
                        ctx.log.add("You enter a maze of up staircases.");
                        ctx.turn.pending_stair = Some(t);
                        autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(
                            ctx.ps.depth.saturating_sub(steps).max(mindepth),
                        ));
                    }
                    T_WAY_LESS => {
                        ctx.log.add("You enter the previous area.");
                        ctx.turn.pending_stair = Some(t);
                        autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth - 1));
                    }
                    _ => {
                        ctx.log.add("You enter a maze of up staircases.");
                        autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Depth(ctx.ps.depth - 1));
                    }
                }
            }
        }
        return;
    }
    // Surface (depth 0, big-map mode).
    if delta > 0 {
        if t == T_STAIRS_DOWN {
            if let Some(&d_idx) = ctx.map.special.get(&idx) {
                enter_dungeon(ctx, d_idx);
            } else {
                ctx.log.add("I see no down staircase here.");
            }
        } else {
            ctx.log.add("I see no down staircase here.");
        }
        return;
    }
    // `<` starts the wilderness travel overview (dungeon.cc).
    if ctx.turn.ambush {
        ctx.log
            .add("To flee the ambush you have to reach the edge of the map.");
        return;
    }
    if ctx.ps.food < game::FOOD_HUNGRY {
        ctx.log.add("You are too hungry to travel.");
        return;
    }
    if ctx.ps.cut > 0 || ctx.ps.poison > 0 {
        ctx.log.add("You are too injured to travel.");
        return;
    }
    if ctx.ps.word_recall > 0 {
        ctx.log.add("You cannot travel while recalling.");
        return;
    }
    // Vampires (and other light-sensitive races) cannot travel by day or
    // with an unsafe light source (dungeon.cc travel checks).
    let hurt_lite = game::player_has_flag(&ctx.gd, &ctx.ps, "HURT_LITE");
    if hurt_lite && game::is_daytime(ctx.ps.turn) {
        ctx.log.add("You can't travel during the day!");
        return;
    }
    if hurt_lite && !game::light_is_safe(&ctx.gd, &ctx.inv) {
        ctx.log
            .add("Travel with your present light would be unsafe.");
        return;
    }
    if crate::mimic::forbid_travel(&ctx.ps) {
        ctx.log
            .add("You had best not travel with your extra limbs.");
        return;
    }
    let (wx, wy) = (ctx.ps.wild_x, ctx.ps.wild_y);
    ctx.store.wilderness.reveal(wx, wy, 3);
    ctx.log
        .add("You look out over the wilderness of Middle-earth.");
    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::WildMap(wx, wy));
}

/// `>` in the world overview: enter the cell's town/area or dungeon.
fn enter_wild_cell(ctx: &mut InputCtx) {
    let Ok(pos) = ctx.player.single() else {
        return;
    };
    ctx.ps.wild_x = pos.x;
    ctx.ps.wild_y = pos.y;
    let town = ctx
        .store
        .wilderness
        .town_at(&ctx.gd, pos.x, pos.y, &ctx.plot);
    if let Some(d) = ctx
        .store
        .wilderness
        .dungeon_at(&ctx.gd, pos.x, pos.y, &ctx.plot)
    {
        enter_dungeon(ctx, d);
        return;
    }
    if town != 0 {
        let name = ctx
            .gd
            .town(town, ctx.plot.town_destroyed(town))
            .map(|t| t.name.clone())
            .unwrap_or_default();
        ctx.log.add(format!("You enter {}.", name));
    } else {
        ctx.log.add("You enter the area.");
    }
    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Wild(pos.x, pos.y));
}

/// Enter a dungeon from a town/wilderness `>` stair (cmd2.cc): the depth
/// depends on whether the wilderness cell is the entrance or the exit.
fn enter_dungeon(ctx: &mut InputCtx, idx: u32) {
    let d = ctx.gd.dungeon(idx);
    if d.min_plev > ctx.ps.level {
        ctx.log
            .add("You don't feel yourself experienced enough to go there...");
        return;
    }
    let (ix, iy, ox, oy) = (d.ix, d.iy, d.ox, d.oy);
    let (mindepth, maxdepth, text) = (d.mindepth, d.maxdepth, d.text.clone());
    let depth = if ctx.ps.wild_x == ix && ctx.ps.wild_y == iy {
        mindepth
    } else if ctx.ps.wild_x == ox && ctx.ps.wild_y == oy {
        maxdepth
    } else {
        mindepth
    };
    ctx.turn.leaving_quest = 0;
    ctx.log.add(format!("You go into {}", text));
    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Dungeon(idx, depth));
}

/// Leaving a plot-quest level: return to the town it was entered from
/// (leaving_quest selects the pref P: entry point).
fn quest_exit(ctx: &mut InputCtx) {
    let qid = ctx.ps.depth - game::PLOT_DEPTH_BASE;
    // Escaping the Between through Turgon's tower wins the quest
    // (q_betwen.cc: reaching the tower completes it).
    if qid == game::PLOT_BETWEEN && ctx.plot.status(qid) == game::PLOT_TAKEN {
        ctx.plot.set(qid, game::PLOT_COMPLETED);
        ctx.log.add("Turgon is there.");
        ctx.log.add(
            "'Ah, thank you, noble hero! Now please return to Minas Anor to finish the link.'",
        );
    }
    // Nirnaeth: finding a way out completes the quest (q_nirna.cc
    // stair hook); the reward is paid at the report.
    if qid == game::PLOT_NIRNAETH && ctx.plot.status(qid) == game::PLOT_TAKEN {
        ctx.plot.set(qid, game::PLOT_COMPLETED);
        ctx.log.add("You found a way out!");
    }
    // Invasion: returning to Turgon after Maeglin's fall grants the
    // mages' life spell (q_invas.cc stair hook).
    if qid == game::PLOT_INVASION && ctx.plot.status(qid) == game::PLOT_COMPLETED {
        ctx.plot.set(qid, game::PLOT_FINISHED);
        ctx.ps.max_hp += 150;
        ctx.ps.hp += 150;
        ctx.log.add("Turgon appears before you and speaks:");
        ctx.log.add("'I will never be able to thank you enough.'");
        ctx.log.add(
            "'My most powerful mages will cast a powerful spell for you, giving you extra life.'",
        );
        ctx.log.add("(+150 max HP)");
    }
    // Quests that cannot be abandoned without failing them
    // (q_eol/q_fireprof/q_library/q_invas).
    let fail_on_exit = matches!(
        qid,
        game::PLOT_EOL | game::PLOT_FIREPROOF | game::PLOT_LIBRARY | game::PLOT_INVASION
    );
    if fail_on_exit && ctx.plot.status(qid) == game::PLOT_TAKEN {
        // Ask before abandoning (q_eol.cc/q_fireprof.cc/q_library.cc/
        // q_invas.cc get_check).
        *ctx.modal = Modal::AbandonQuest { qid };
        return;
    }
    leave_quest(ctx, qid);
}

/// Leave a plot-quest level back to the town it was entered from.
pub(crate) fn leave_quest(ctx: &mut InputCtx, qid: u32) {
    let (wx, wy) = ctx
        .ps
        .quest_origin
        .unwrap_or((ctx.ps.wild_x, ctx.ps.wild_y));
    ctx.ps.wild_x = wx;
    ctx.ps.wild_y = wy;
    ctx.ps.wild_mode = false;
    ctx.turn.leaving_quest = qid;
    autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Wild(wx, wy));
}

/// Move one world-overview step (do_cmd_walk_jump, src/cmd2.cc): one cell
/// per step, a whole 132-turn block of travel, ambushes in rough land.
fn wild_step(ctx: &mut InputCtx, dx: i32, dy: i32) {
    let Ok(mut pos) = ctx.player.single_mut() else {
        return;
    };
    let (nx, ny) = (pos.x + dx, pos.y + dy);
    if !ctx.store.wilderness.in_bounds(nx, ny) {
        return;
    }
    let (mut can_walk, wf_name, ambush_roll) = {
        let wild = &ctx.store.wilderness;
        let wf = wild.wf_at(&ctx.gd, nx, ny, &ctx.plot);
        (
            ctx.gd.terrain(wf.feat).is_floor && !ctx.gd.terrain(wf.feat).no_walk,
            wf.name.clone(),
            (wf.level as i32 - ctx.ps.level as i32 * 2).max(0),
        )
    };
    // Overhead travel is one keystroke: refuse deep water for heavy
    // swimmers and lava without fire protection/levitation, exactly like
    // player_can_enter's wild_mode branch (cmd1.cc:2461).
    {
        let feat = ctx.store.wilderness.wf_at(&ctx.gd, nx, ny, &ctx.plot).feat;
        let tot = ctx.inv.totals_for(&ctx.gd, &ctx.ps);
        if feat == map::T_DEEP_WATER {
            if game::calc_total_weight(&ctx.gd, &ctx.inv) >= game::weight_limit(&ctx.ps) / 2
                && !run_levitates(&ctx.gd, &ctx.ps, &ctx.inv)
            {
                can_walk = false;
            }
        } else if feat == map::T_SHAL_LAVA || feat == map::T_LAVA {
            let safe = tot.immunities.contains("FIRE")
                || tot.resists.contains("FIRE")
                || ctx.ps.oppose_fire > 0
                || run_levitates(&ctx.gd, &ctx.ps, &ctx.inv);
            if !safe {
                can_walk = false;
            }
        }
    }
    if !can_walk {
        ctx.log.add(format!(
            "There is a {} in the way.",
            ctx.gd.terrain(ctx.map.terrain_at(nx, ny)).name
        ));
        return;
    }
        // Rooted means no move (cmd1.cc:2974; Tree Roots).
        if ctx.ps.tim_roots > 0 {
            return;
        }
        pos.x = nx;
        pos.y = ny;
    // Remember the surroundings (WILDERNESS_SEE_RADIUS = 3).
    ctx.store.wilderness.reveal(nx, ny, 3);
    for j in ny - 3..=ny + 3 {
        for i in nx - 3..=nx + 3 {
            if Map::in_bounds(i, j) && ctx.store.wilderness.is_known(i, j) {
                let idx = Map::idx(i, j);
                ctx.map.lit[idx] = true;
                ctx.map.explored[idx] = true;
            }
        }
    }
    // One step equals (MAX_HGT+MAX_WID)/2 normal turns.
    ctx.ps.turn += 132;
    consume_turn(ctx);
    ctx.log.add(format!("You travel through {}.", wf_name));
    // Ambush (magik(level - player level * 2)).
    let mut rng = crate::rng::current();
    if ambush_roll > 0 && rng.gen_range(0..100) < ambush_roll {
        ctx.ps.wild_x = nx;
        ctx.ps.wild_y = ny;
        ctx.ps.wild_mode = false;
        ctx.turn.ambush = true;
        ctx.log.add("You are ambushed!");
        autosave_request(&ctx.options, &mut ctx.turn);
                ctx.turn.pending = Some(game::Goto::Wild(nx, ny));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MessageLog;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// `critical_shot` (cmd1.cc:131): a huge shot power always crits and
    /// reports one of the good/great/superb lines; a hopeless shot never
    /// changes the damage.
    #[test]
    fn critical_shot_tiers() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut log = MessageLog::default();
        let d = critical_shot(10_000, 0, 10, 0, 0, 0, 0, &mut rng, &mut log);
        assert_eq!(d, 3 * 10 + 15, "weight 10000 is a superb hit");
        assert!(log.lines.iter().any(|l| l == "It was a superb hit!"));
        // Negative power never crits (randint(5000) > i).
        let mut log = MessageLog::default();
        let d = critical_shot(-100_000, 0, 10, 0, 0, 0, 0, &mut rng, &mut log);
        assert_eq!(d, 10);
        assert!(log.lines.is_empty());
    }

    /// `test_hit_fire` honours the invisible half-chance: a chance that is
    /// zero after halving can never get past the armour roll.
    #[test]
    fn test_hit_fire_invisible_penalty() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut hits = 0;
        for _ in 0..200 {
            if test_hit_fire(1, 100, false, 0, &mut rng) {
                hits += 1;
            }
        }
        // (1+1)/2 = 1 vs AC 75: only the forced 5% hits connect.
        assert!(hits <= 20, "invisible chance halving ignored: {hits}");
    }

    /// Selects the seeded quick RNG but restores the complex stream on
    /// drop (including panics), so other tests keep their own randomness.
    struct QuickRngGuard;

    impl Drop for QuickRngGuard {
        fn drop(&mut self) {
            crate::rng::set_complex_rng();
        }
    }

    fn test_player(name: &str) -> (crate::data::GameData, crate::game::PlayerState) {
        let gd = crate::data::load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Warrior").unwrap_or(0);
        let ps = crate::birth::make_player(&gd, name.to_string(), race, class);
        (gd, ps)
    }

    /// A world with every resource `player_input` needs, an open floor map
    /// and the player at (10, 10).  Deferred commands are applied by
    /// `run_system_once`.
    fn populate_input_world(world: &mut World) {
        use bevy::ecs::message::Messages;
        let gd = crate::data::load_game_data();
        let race = gd.races.iter().position(|r| r.name == "Human").unwrap_or(0);
        let class = gd.classes.iter().position(|c| c.name == "Warrior").unwrap_or(0);
        let mut ps = crate::birth::make_player(&gd, "Input".to_string(), race, class);
        ps.turn = 1;
        ps.hp = ps.max_hp.max(100);
        ps.max_hp = ps.hp;
        ps.mana = ps.max_mana;
        world.insert_resource(gd);
        world.insert_resource(ps);
        world.insert_resource(TileAssets {
            font: Default::default(),
            bg_image: Default::default(),
            bg_layout: Default::default(),
            fg_image: Default::default(),
            fg_layout: Default::default(),
        });
        world.insert_resource(Inventory::default());
        let mut map = crate::map::blank_pub(map::T_FLOOR);
        map.visible.fill(true);
        map.explored.fill(true);
        world.insert_resource(map);
        world.insert_resource(MessageLog::default());
        world.insert_resource(TurnState::default());
        world.insert_resource(crate::game::PlotQuest::default());
        world.insert_resource(crate::item::CreatedArtifacts::default());
        world.insert_resource(ShopStocks::default());
        world.insert_resource(crate::save::LevelStore::default());
        world.insert_resource(Modal::default());
        world.insert_resource(crate::modal::ModalInputConsumed::default());
        world.insert_resource(MoveRepeat::default());
        world.insert_resource(crate::options::Options::default());
        world.insert_resource(crate::notes::Notes::default());
        world.insert_resource(crate::squeltch::Automatizer::default());
        world.insert_resource(PetOptions::default());
        world.insert_resource(PetMenu::default());
        world.insert_resource(ItemPicker::default());
        world.insert_resource(Time::<()>::default());
        world.insert_resource(NextState::<AppState>::default());
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(Messages::<AppExit>::default());
        world.spawn((GridPos { x: 10, y: 10 }, Player));
    }

    fn input_world() -> World {
        let mut world = World::new();
        populate_input_world(&mut world);
        world
    }

    /// An App whose Update schedule holds `player_input`, so its `Local`
    /// run state persists across frames (like the real game loop).
    fn input_app() -> App {
        let mut app = App::new();
        populate_input_world(app.world_mut());
        app.add_systems(Update, player_input);
        app
    }

    fn player_pos_of(world: &mut World) -> (i32, i32) {
        let mut q = world.query::<&GridPos>();
        let p = q.iter(world).next().expect("player");
        (p.x, p.y)
    }

    fn spawn_test_monster(world: &mut World, def: usize, hp: i32, x: i32, y: i32) {
        let m = Monster {
            def,
            hp,
            max_hp: hp,
            energy: 0,
            awake: true,
            friendly: false,
            quest: false,
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
            seen: false,
            detected: 0,
            controlled: false,
            mon_level: 0,
            possessor: None,
            ac_mod: 0,
            blow_penalty: 0,
            target: None,
        };
        world.spawn((m, GridPos { x, y }));
    }

    /// `move_player_aux` / `run_step` (cmd1.cc:2639/3808): one direction
    /// keypress steps the player east and consumes the turn.
    #[test]
    fn player_input_steps_and_consumes_the_turn() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        assert_eq!(player_pos_of(&mut world), (11, 10));
        assert!(world.resource::<TurnState>().world_turn);
    }

    /// `critical_norm` (cmd1.cc:172): Divine Aim forces a *GREAT* hit and
    /// is consumed blow by blow; a hopeless blow leaves the damage alone.
    #[test]
    fn critical_norm_forced_deadly_and_no_crit() {
        let (_gd, mut ps) = test_player("Crit");
        let mut rng = StdRng::seed_from_u64(11);
        let mut log = MessageLog::default();
        ps.tim_deadly = 2;
        ps.melee_style = crate::skill::SK_MASTERY;
        let d = critical_norm_melee(&mut ps, 30, 0, 0, 0, false, 10, 0, &mut rng, &mut log);
        assert_eq!(d, 3 * 10 + 20, "forced *GREAT* hit");
        assert_eq!(ps.tim_deadly, 1);
        assert!(log.lines.iter().any(|l| l == "It was a *GREAT* hit!"));
        let d = critical_norm_melee(&mut ps, 30, 0, 0, 0, false, 10, 0, &mut rng, &mut log);
        assert_eq!(d, 3 * 10 + 20);
        assert_eq!(ps.tim_deadly, 0, "the blow consumes the last point");
        assert!(log
            .lines
            .iter()
            .any(|l| l == "You are suddenly much less accurate."));
        let d = critical_norm_melee(&mut ps, -100_000, 0, 0, 0, false, 7, 0, &mut rng, &mut log);
        assert_eq!(d, 7, "a hopeless blow never changes the damage");
    }

    /// `flavored_attack` (cmd1.cc:1503): the percent bands pick the static
    /// verb, while full insanity replaces it with a `dam_*.txt` line.
    #[test]
    fn flavored_attack_bands_and_insanity() {
        let (gd, _ps) = test_player("Flavor");
        let def = gd
            .monsters
            .iter()
            .find(|m| !"vwjmelX,.*".contains(m.glyph()))
            .unwrap();
        let mut rng = StdRng::seed_from_u64(5);
        assert_eq!(
            flavored_attack(def, "the orc", 3, 100, 100, &mut rng),
            "You scratch the orc."
        );
        assert_eq!(
            flavored_attack(def, "the orc", 20, 100, 100, &mut rng),
            "You hit the orc."
        );
        assert_eq!(
            flavored_attack(def, "the orc", 70, 100, 100, &mut rng),
            "You cripple the orc."
        );
        assert_eq!(
            flavored_attack(def, "the orc", 99, 100, 100, &mut rng),
            "You demolish the orc."
        );
        // Sanity at zero = 100% insanity: the line comes from dam_none.txt.
        let line = flavored_attack(def, "the orc", 3, 100, 0, &mut rng);
        assert!(
            line.contains("orc") && line != "You scratch the orc.",
            "insane line: {line}"
        );
    }

    /// `py_attack_hand` (cmd1.cc:1621): a monk's blow rolls the martial
    /// table and returns a positive damage packet.
    #[test]
    fn py_attack_hand_rolls_a_martial_blow() {
        let (gd, mut ps) = test_player("Monk");
        ps.melee_style = crate::skill::SK_HAND;
        ps.skills
            .insert(crate::skill::SK_HAND, 40 * crate::skill::SKILL_STEP);
        let def = gd.monsters[0].clone();
        let mut rng = StdRng::seed_from_u64(13);
        let mut log = MessageLog::default();
        let hb = py_attack_hand(
            &gd,
            &mut ps,
            &def,
            "the orc",
            1000,
            5,
            100,
            10,
            0,
            0,
            &mut rng,
            &mut log,
        );
        assert!(hb.dam >= 1, "a landed martial blow does damage");
        assert!(!log.lines.is_empty(), "the blow is described");
    }

    /// `see_obstacle_grid` / `see_nothing` (cmd1.cc:3092/3147): a known
    /// wall blocks the run; an unexplored floor grid is an unknown corner.
    #[test]
    fn see_obstacle_and_unknown_corners() {
        let (gd, ps) = test_player("Runner");
        let inv = crate::item::Inventory::default();
        let mut map = crate::map::blank_pub(map::T_FLOOR);
        map.visible.fill(false);
        map.explored.fill(false);
        let (px, py) = (10, 10);
        // Unexplored, unseen floor: an unknown corner, not a known obstacle.
        assert!(see_nothing(&map, &gd, &ps, &inv, 8, py, px));
        assert!(!see_obstacle_grid(&map, &gd, &ps, &inv, px, py - 1));
        // Seeing the floor grid makes the corner known.
        map.visible[Map::idx(px, py - 1)] = true;
        assert!(!see_nothing(&map, &gd, &ps, &inv, 8, py, px));
        // A known wall blocks the runner.
        map.explored[Map::idx(px, py + 1)] = true;
        map.set_terrain(px, py + 1, map::T_GRANITE);
        assert!(see_obstacle(&map, &gd, &ps, &inv, 2, py, px));
        assert!(see_obstacle_grid(&map, &gd, &ps, &inv, px, py + 1));
    }

    /// `move_player_aux` (cmd1.cc:2639): bumping a closed door runs
    /// easy_open_door (force_door) instead of moving.
    #[test]
    fn bumping_a_closed_door_opens_it() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<Map>()
            .set_terrain(11, 10, map::T_DOOR);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        assert_eq!(player_pos_of(&mut world), (10, 10), "the door blocked");
        assert_eq!(
            world.resource::<Map>().terrain_at(11, 10),
            map::T_OPEN_DOOR
        );
    }

    /// `run_test` / `run_step` (cmd1.cc:3445/3808): a run keeps stepping
    /// along a corridor without further key presses.
    #[test]
    fn running_follows_a_corridor() {
        let mut app = input_app();
        {
            let mut map = app.world_mut().resource_mut::<Map>();
            for x in 11..20 {
                for y in [9, 11] {
                    map.set_terrain(x, y, map::T_GRANITE);
                }
            }
        }
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyL);
        }
        app.update();
        for _ in 0..8 {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
            app.update();
        }
        assert!(
            player_pos_of(app.world_mut()).0 > 13,
            "the run followed the corridor"
        );
    }

    /// `do_cmd_pet` (cmd1.cc:3863): the companion menu opens and dismissing
    /// pets despawns them.
    #[test]
    fn pet_menu_dismisses_pets() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        let def = {
            let gd = world.resource::<crate::data::GameData>();
            gd.monsters
                .iter()
                .position(|m| !m.has("NO_DEATH"))
                .unwrap()
        };
        spawn_test_monster(&mut world, def, 10, 12, 10);
        {
            let mut q = world.query::<&mut Monster>();
            for mut m in q.iter_mut(&mut world) {
                m.companion = true;
                m.pet = true;
            }
        }
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyO);
        }
        world.run_system_once(player_input).unwrap();
        assert!(world.resource::<PetMenu>().open, "the menu opened");
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyA);
        }
        world.run_system_once(player_input).unwrap();
        let mut q = world.query::<&Monster>();
        assert_eq!(q.iter(&world).count(), 0, "the pet was dismissed");
    }

    /// `do_nazgul` (cmd1.cc:1779): a mundane weapon disintegrates when it
    /// strikes a Ringwraith.
    #[test]
    fn nazgul_disintegrates_a_mundane_weapon() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        let mdef = {
            let gd = world.resource::<crate::data::GameData>();
            gd.monsters
                .iter()
                .position(|m| m.has("NAZGUL"))
                .expect("a nazgul race")
        };
        spawn_test_monster(&mut world, mdef, 500, 11, 10);
        let mut gone = false;
        for _ in 0..200 {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
            world.run_system_once(player_input).unwrap();
            if world.resource::<Inventory>().equip[data::SLOT_WEAPON].is_none() {
                gone = true;
                break;
            }
        }
        assert!(gone, "the mundane weapon disintegrated");
    }

    /// `player_can_enter` (cmd1.cc:2442): dense forest needs flight, Tree
    /// walking or an Ent shape; open floor is always walkable.
    #[test]
    fn player_can_walk_respects_trees() {
        let (gd, mut ps) = test_player("Walker");
        let inv = crate::item::Inventory::default();
        let mut map = crate::map::blank_pub(map::T_FLOOR);
        map.set_terrain(5, 5, map::T_TREE);
        assert!(!player_can_walk(&map, &gd, &ps, &inv, 5, 5));
        ps.tim_fly = 5;
        assert!(player_can_walk(&map, &gd, &ps, &inv, 5, 5));
        ps.tim_fly = 0;
        ps.mimic_form = Some(crate::mimic::MIMIC_ENT);
        assert!(player_can_walk(&map, &gd, &ps, &inv, 5, 5));
        assert!(player_can_walk(&map, &gd, &ps, &inv, 6, 5));
    }

    /// `do_cmd_leave_body` / `do_cmd_integrate_body` (cmd1.cc:4321/4278):
    /// the spirit leaves the body and can incarnate into a corpse.
    #[test]
    fn leave_and_integrate_body_round_trip() {
        let (gd, mut ps) = test_player("Spirit");
        let inv = crate::item::Inventory::default();
        let mut log = MessageLog::default();
        let mut rng = StdRng::seed_from_u64(21);
        let body = gd.monster_by_name("Filthy street urchin").unwrap();
        ps.possessed = Some((body, 30, 30));
        // Cursed gear blocks leaving.
        let mut cursed_inv = inv.clone();
        let mut cursed = {
            let def = gd.object_by_name("Dagger").unwrap();
            crate::item::Item::base(&gd, def)
        };
        cursed.cursed = true;
        cursed_inv.equip[data::SLOT_WEAPON] = Some(cursed);
        assert!(game::leave_body(&mut ps, &cursed_inv, &gd, true, &mut rng, &mut log).is_err());
        assert!(!ps.disembodied);
        // Leaving works; whether the corpse is kept depends on the skill
        // roll, so only the spirit state is asserted here.
        let _ = game::leave_body(&mut ps, &inv, &gd, false, &mut rng, &mut log).unwrap();
        assert!(ps.disembodied && ps.possessed.is_none());
        // A corpse object can be entered again.
        let cd = gd.object_by_tval_sval(data::TV_CORPSE, 1).unwrap();
        let mut corpse = crate::item::Item::base(&gd, cd);
        corpse.note = body as u32;
        corpse.pval3 = 30;
        assert!(game::integrate_body(&mut ps, &corpse, &gd, &mut log));
        assert!(!ps.disembodied);
        assert_eq!(ps.possessed.unwrap().0, body);
    }

    /// `execute_inscription` (cmd1.cc:4381) through grid entry: a known
    /// EXEC_WALK rune spends its mana and fires when stepped on.
    #[test]
    fn walking_over_an_engraved_rune_fires_it() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world.resource_mut::<PlayerState>().inscriptions = vec![false; 8];
        {
            let mut ps = world.resource_mut::<PlayerState>();
            game::rune_learn(&mut ps, game::INSCRIP_LIGHT);
        }
        {
            let mut map = world.resource_mut::<Map>();
            map.mana = vec![0u8; (map::MAP_W * map::MAP_H) as usize];
            map.mana[Map::idx(11, 10)] = 200;
            map.inscriptions
                .insert(Map::idx(11, 10), game::INSCRIP_LIGHT);
        }
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        assert_eq!(player_pos_of(&mut world), (11, 10));
        let cost = game::INSCRIPTIONS[game::INSCRIP_LIGHT as usize].2 as u8;
        assert_eq!(world.resource::<Map>().mana[Map::idx(11, 10)], 200 - cost);
        assert!(world
            .resource::<MessageLog>()
            .lines
            .iter()
            .any(|l| l == "The inscription shines in a bright light!"));
    }

    /// `run_init` (cmd1.cc:3351): an open area keeps the runner free, while
    /// obstacles on both diagonal fronts open a "break" corridor.
    #[test]
    fn run_init_breaks_follow_obstacles() {
        let (gd, ps) = test_player("Runner");
        let inv = crate::item::Inventory::default();
        let mut map = crate::map::blank_pub(map::T_FLOOR);
        map.explored.fill(true);
        let (px, py) = (10, 10);
        // Open floor all around: no break, open area.
        let st = run_init(&map, &gd, &ps, &inv, py, px, 8);
        assert!(st.openarea && !st.breakleft && !st.breakright);
        // Known walls on both diagonal fronts (NW and NE of the runner).
        map.set_terrain(px - 1, py - 1, map::T_GRANITE);
        map.set_terrain(px + 1, py - 1, map::T_GRANITE);
        let st = run_init(&map, &gd, &ps, &inv, py, px, 8);
        assert!(st.breakleft && st.breakright && !st.openarea);
    }

    /// `carry` (cmd1.cc:453): walking onto a pile with `always_pickup` on
    /// takes the objects into the pack.
    #[test]
    fn carry_picks_up_floor_items() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        let it = {
            let gd = world.resource::<crate::data::GameData>();
            let def = gd.object_by_name("Dagger").unwrap();
            crate::item::Item::base(gd, def)
        };
        world.spawn((
            crate::item::FloorItem { stack: vec![it] },
            GridPos { x: 11, y: 10 },
        ));
        world.resource_mut::<crate::options::Options>().always_pickup = true;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        assert_eq!(player_pos_of(&mut world), (11, 10));
        let gd = world.resource::<crate::data::GameData>();
        assert!(
            world
                .resource::<Inventory>()
                .pack
                .iter()
                .any(|i| gd.objects[i.def].name == "Dagger"),
            "the pile was picked up"
        );
    }

    /// `py_attack` (cmd1.cc:1875): walking into a weak monster attacks it
    /// and the blows can kill it.
    #[test]
    fn melee_attack_kills_a_weak_monster() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        let def = {
            let gd = world.resource::<crate::data::GameData>();
            gd.monsters
                .iter()
                .position(|m| m.name == "Scrawny cat")
                .unwrap()
        };
        spawn_test_monster(&mut world, def, 1, 11, 10);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        assert_eq!(player_pos_of(&mut world), (10, 10), "the monster blocked");
        let mut q = world.query::<&Monster>();
        assert!(
            q.iter(&world).all(|m| m.hp <= 0 || m.hp == 1),
            "the monster took damage but can no longer block the tile"
        );
    }

    /// `attack_special` (cmd1.cc:1551) via the melee path: a poison-brand
    /// weapon poisons a shallow monster in the same blow.
    #[test]
    fn melee_poison_brand_poisons_the_victim() {
        use bevy::ecs::system::RunSystemOnce;
        crate::rng::set_quick_rng(0xC0FF_EE11);
        let _rng_guard = QuickRngGuard;
        let mut world = input_world();
        let (wdef, mdef) = {
            let gd = world.resource::<crate::data::GameData>();
            let w = gd.object_by_name("Dagger").unwrap();
            let m = gd
                .monsters
                .iter()
                .position(|m| m.name == "Scrawny cat")
                .unwrap();
            (w, m)
        };
        let mut it = {
            let gd = world.resource::<crate::data::GameData>();
            crate::item::Item::base(gd, wdef)
        };
        it.artifact = 69; // "of Rilia" carries BRAND_POIS
        world.resource_mut::<Inventory>().equip[data::SLOT_WEAPON] = Some(it);
        spawn_test_monster(&mut world, mdef, 500, 11, 10);
        let mut poisoned = false;
        for _ in 0..30 {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
            world.run_system_once(player_input).unwrap();
            let mut q = world.query::<&Monster>();
            if q.iter(&world).any(|m| m.poison > 0) {
                poisoned = true;
                break;
            }
        }
        assert!(poisoned, "the poison brand poisoned the monster");
    }

    /// `do_cmd_close` / `do_cmd_spike` (cmd2.cc:939/1745): Shift+C closes
    /// the only adjacent open door and Shift+V jams the closed one.
    #[test]
    fn close_and_spike_a_door() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<Map>()
            .set_terrain(11, 10, map::T_OPEN_DOOR);
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyC);
        }
        world.run_system_once(player_input).unwrap();
        assert_eq!(
            world.resource::<Map>().terrain_at(11, 10),
            map::T_DOOR,
            "the open door was closed"
        );
        // Put an iron spike in the pack and jam the door.
        let spike = {
            let gd = world.resource::<crate::data::GameData>();
            let def = gd
                .objects
                .iter()
                .position(|o| o.tval == data::TV_SPIKE)
                .expect("a spike kind");
            crate::item::Item::base(gd, def)
        };
        world.resource_mut::<Inventory>().pack.push(spike);
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyV);
        }
        world.run_system_once(player_input).unwrap();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
        }
        world.run_system_once(player_input).unwrap();
        let t = world.resource::<Map>().terrain_at(11, 10);
        assert_ne!(t, map::T_DOOR, "the door was spiked ({t})");
        assert!(world.resource::<Inventory>().pack.is_empty(), "spike used");
    }

    /// `do_cmd_tunnel_aux` (cmd2.cc:1103): Ctrl+direction digs through an
    /// explored rubble grid with a digging tool in the tool slot.
    #[test]
    fn tunnelling_removes_rubble() {
        let mut app = input_app();
        {
            let gd = app.world_mut().resource::<crate::data::GameData>();
            let def = gd
                .objects
                .iter()
                .position(|o| o.tval == data::TV_DIGGING)
                .expect("a digger kind");
            let it = crate::item::Item::base(gd, def);
            app.world_mut().resource_mut::<Inventory>().equip[data::SLOT_TOOL] = Some(it);
        }
        {
            let mut map = app.world_mut().resource_mut::<Map>();
            let idx = Map::idx(11, 10);
            map.set_terrain(11, 10, map::T_RUBBLE);
            map.explored[idx] = true;
        }
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyL);
        }
        app.update();
        for _ in 0..200 {
            if app.world_mut().resource::<Map>().terrain_at(11, 10) != map::T_RUBBLE {
                break;
            }
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyL);
            app.update();
        }
        assert_ne!(
            app.world_mut().resource::<Map>().terrain_at(11, 10),
            map::T_RUBBLE,
            "the rubble was cleared"
        );
    }

    /// `do_cmd_rest` (cmd2.cc:2119): R + count + Enter arms a numbered rest
    /// (defaults to `&` when no count is typed).
    #[test]
    fn ctrl_w_confirms_then_toggles_wizard() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        // Ctrl-W asks first when the save is still scored.
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyW);
        }
        world.run_system_once(player_input).unwrap();
        assert!(matches!(
            *world.resource::<Modal>(),
            Modal::ConfirmWizard { debug: false }
        ));
        assert_eq!(world.resource::<PlayerState>().noscore, 0);
        // The original prompt's y/n is answered through the modal.
        world.insert_resource(crate::scores::HighScores::default());
        world.insert_resource(crate::modal::TargetLock::default());
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyY);
        }
        world.run_system_once(crate::modal::modal_input).unwrap();
        {
            let ps = world.resource::<PlayerState>();
            assert!(ps.wizard);
            assert_eq!(ps.noscore & 0x0002, 0x0002, "wizard marks the save");
        }
        assert!(matches!(*world.resource::<Modal>(), Modal::None));
        // Ctrl-W again leaves wizard mode without any prompt.
        world
            .resource_mut::<crate::modal::ModalInputConsumed>()
            .0 = false;
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyW);
        }
        world.run_system_once(player_input).unwrap();
        assert!(!world.resource::<PlayerState>().wizard);
        assert!(matches!(*world.resource::<Modal>(), Modal::None));
    }

    #[test]
    fn open_key_opens_an_adjacent_door() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<Map>()
            .set_terrain(11, 10, map::T_DOOR);
        // 'o' (dungeon.cc:3098 do_cmd_open) auto-targets the only
        // adjacent closed door.
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyO);
        world.run_system_once(player_input).unwrap();
        assert_eq!(
            world.resource::<Map>().terrain_at(11, 10),
            map::T_OPEN_DOOR,
            "the door did not open"
        );
        // Nothing to open anymore: the original message.
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyO);
        }
        world.run_system_once(player_input).unwrap();
        assert!(world
            .resource::<MessageLog>()
            .lines
            .iter()
            .any(|l| l == "You see nothing there to open."));
    }

    #[test]
    fn rest_prompt_arms_a_numbered_rest() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        world.run_system_once(player_input).unwrap();
        assert!(world.resource::<MoveRepeat>().rest_prompt.is_some());
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Digit5);
        }
        world.run_system_once(player_input).unwrap();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::Enter);
        }
        world.run_system_once(player_input).unwrap();
        assert_eq!(world.resource::<TurnState>().resting, 5);
        assert_eq!(world.resource::<MoveRepeat>().rest_mode, RestMode::Turns);
    }

    /// `do_cmd_bash` / `do_cmd_bash_fountain` (cmd2.cc:1552/81): Ctrl+B on
    /// a fountain smashes it and an altar only earns a warning.
    #[test]
    fn bashing_fountains_and_altars() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world.resource_mut::<Map>().set_terrain(11, 10, 2);
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyB);
        }
        world.run_system_once(player_input).unwrap();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
        }
        world.run_system_once(player_input).unwrap();
        assert!(world
            .resource::<MessageLog>()
            .lines
            .iter()
            .any(|l| l == "You smash into the fountain!"));
        // Altars refuse: "Are you mad?"
        {
            let mut map = world.resource_mut::<Map>();
            map.set_terrain(11, 10, 161);
        }
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyB);
        }
        world.run_system_once(player_input).unwrap();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
        }
        world.run_system_once(player_input).unwrap();
        assert!(world
            .resource::<MessageLog>()
            .lines
            .iter()
            .any(|l| l == "Are you mad? You want to anger the gods?"));
    }

    /// `item_tester_hook_sacrificable` / the grace value
    /// (cmd2.cc:3698/3835): Melkor takes corpses and non-Udun books for
    /// 2*level / 2*spell-levels of grace.
    #[test]
    fn melkor_sacrifice_helpers() {
        let (gd, _ps) = test_player("Melkor");
        // A corpse with a valid monster note is sacrificable.
        let cd = gd.object_by_tval_sval(data::TV_CORPSE, 1).unwrap();
        let mut corpse = crate::item::Item::base(&gd, cd);
        let body = gd
            .monsters
            .iter()
            .position(|m| m.depth >= 1)
            .unwrap();
        corpse.note = body as u32;
        assert!(sacrificable(&gd, 4, &corpse));
        assert!(!sacrificable(&gd, 1, &corpse));
        assert_eq!(
            sacrifice_grace(&gd, &corpse),
            2 * gd.monsters[body].depth.max(1) as i32
        );
        // A book with no Udun spells is sacrificable.
        if let Some(bd) = gd.objects.iter().position(|o| o.tval == data::TV_BOOK) {
            let book = crate::item::Item::base(&gd, bd);
            if udun_in_book(&gd, &book) <= 0 {
                assert!(sacrificable(&gd, 4, &book));
                assert_eq!(sacrifice_grace(&gd, &book), 2 * levels_in_book(&gd, &book));
            }
        }
    }

    /// `ask_leave` / `do_cmd_go_down` (cmd2.cc:145/365): with confirm_stairs
    /// on, a down staircase raises the confirmation instead of leaving.
    #[test]
    fn stairs_ask_leave_when_confirmed() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world.resource_mut::<PlayerState>().depth = 5;
        world.resource_mut::<PlayerState>().dungeon = 0;
        world.resource_mut::<crate::options::Options>().confirm_stairs = true;
        world
            .resource_mut::<Map>()
            .set_terrain(10, 10, T_STAIRS_DOWN);
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::Period);
        }
        world.run_system_once(player_input).unwrap();
        match &*world.resource::<Modal>() {
            Modal::ConfirmLeave { delta, unique } => {
                assert_eq!(*delta, 1);
                assert!(!*unique);
            }
            _ => panic!("expected ConfirmLeave"),
        }
        assert_eq!(world.resource::<PlayerState>().depth, 5, "did not leave");
    }

    /// `allow_repeat_command` (cmd2.cc:768): a numeric prefix typed with
    /// the count prompt repeats the next direction command.
    #[test]
    fn numeric_prefix_repeats_the_command() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Digit0);
        }
        world.run_system_once(player_input).unwrap();
        for d in [KeyCode::Digit3, KeyCode::Enter] {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(d);
            world.run_system_once(player_input).unwrap();
        }
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::KeyL);
        }
        world.run_system_once(player_input).unwrap();
        assert_eq!(
            world.resource::<MoveRepeat>().command_rep,
            2,
            "3 - 1 repeats are queued"
        );
    }

    /// `do_cmd_alter` (cmd2.cc:1641): Shift+= then a direction attacks an
    /// adjacent monster.
    #[test]
    fn alter_attacks_an_adjacent_monster() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        let def = {
            let gd = world.resource::<crate::data::GameData>();
            gd.monsters
                .iter()
                .position(|m| m.name == "Scrawny cat")
                .unwrap()
        };
        spawn_test_monster(&mut world, def, 50, 11, 10);
        crate::rng::set_quick_rng(0xA17E_1234);
        let _rng_guard = QuickRngGuard;
        let mut hit = false;
        for _ in 0..200 {
            {
                let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(KeyCode::ShiftLeft);
                keys.press(KeyCode::Equal);
            }
            world.run_system_once(player_input).unwrap();
            {
                let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(KeyCode::KeyL);
            }
            world.run_system_once(player_input).unwrap();
            let mut q = world.query::<&Monster>();
            if q.iter(&world).any(|m| m.hp < 50) {
                hit = true;
                break;
            }
        }
        assert_eq!(player_pos_of(&mut world), (10, 10));
        assert!(hit, "the alter attack landed");
    }

    /// teleport_player_directed (spells1.cc:139): the candidate geometry
    /// and the landing rules (Chebyshev min/max band, naked floor,
    /// rooted player refuses to move).
    #[test]
    fn directed_teleport_follows_spells1_cc() {
        use bevy::ecs::system::RunSystemOnce;
        use rand::SeedableRng;
        // Geometry: east picks x in [px, px+dis] centered ahead, y in
        // px±dis/3, and every candidate respects [min, dis].
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        for _ in 0..200 {
            let (x, y) = directed_candidate(50, 30, 1, 0, 2, 10, &mut rng);
            assert!((50..=60).contains(&x), "x {x}");
            assert!((27..=33).contains(&y), "y {y}");
            assert!((2..=10).contains(&map::chebyshev(50, 30, x, y)));
        }
        let (wx, _) = directed_candidate(50, 30, -1, 0, 2, 10, &mut rng);
        assert!((40..=50).contains(&wx), "west x {wx}");
        for _ in 0..200 {
            let (x, y) = directed_candidate(50, 30, 1, -1, 2, 10, &mut rng);
            assert!((47..=53).contains(&x), "diag x {x}");
            assert!((27..=33).contains(&y), "diag y {y}");
            assert!((2..=10).contains(&map::chebyshev(50, 30, x, y)));
        }

        // Landing: the player lands on walkable floor, at least `rad/4`
        // and at most `rad` away, and a rooted player does not move.
        crate::rng::set_quick_rng(0xD1_5E_C7);
        let _rng_guard = QuickRngGuard;
        let mut world = input_world();
        world
            .run_system_once(|mut ctx: InputCtx| {
                teleport_directed(&mut ctx, 1, 0, 10);
            })
            .unwrap();
        let (x, y) = player_pos_of(&mut world);
        assert!((10..=20).contains(&x), "eastward phase x={x}");
        assert!(map::chebyshev(10, 10, x, y) >= 2, "min clamp");
        let walkable = {
            let gd = world.resource::<crate::data::GameData>();
            world.resource::<Map>().walkable(gd, x, y)
        };
        assert!(walkable, "landed on a wall");
        world.resource_mut::<PlayerState>().tim_roots = 5;
        let before = player_pos_of(&mut world);
        world
            .run_system_once(|mut ctx: InputCtx| {
                teleport_directed(&mut ctx, 1, 0, 10);
            })
            .unwrap();
        assert_eq!(player_pos_of(&mut world), before, "rooted means no move");
    }

    /// `do_cmd_bash_aux` (cmd2.cc:1445): bashing a plain door eventually
    /// crashes it open.
    #[test]
    fn bashing_a_door_opens_it() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        // adj_str_blow(40) = 240 >= 100, so a plain door always gives way;
        // the seed only needs to cover the door/open-door variant roll.
        world.resource_mut::<PlayerState>().stats[crate::game::STR] = 40;
        crate::rng::set_quick_rng(0x0B_05E);
        let _rng_guard = QuickRngGuard;
        world.resource_mut::<Map>().set_terrain(11, 10, map::T_DOOR);
        let mut opened = false;
        for _ in 0..60 {
            {
                let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(KeyCode::ControlLeft);
                keys.press(KeyCode::KeyB);
            }
            world.run_system_once(player_input).unwrap();
            {
                let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(KeyCode::KeyL);
            }
            world.run_system_once(player_input).unwrap();
            if world.resource::<Map>().terrain_at(11, 10) != map::T_DOOR {
                opened = true;
                break;
            }
        }
        assert!(opened, "the door was bashed open");
    }

    /// `do_cmd_unwalk` (cmd2.cc:1868): an immovable body phases through
    /// walls instead of walking into them.
    #[test]
    fn immovable_bodies_phase_through_walls() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        // Fixed seed: the faithful spread (px..=px+dis) can also pick the
        // starting column, so pin the run that lands beyond the wall.
        crate::rng::set_quick_rng(0x9E37_79B9);
        let _rng_guard = QuickRngGuard;
        let body = {
            let gd = world.resource::<crate::data::GameData>();
            gd.monsters
                .iter()
                .position(|m| m.has("NEVER_MOVE"))
                .expect("an immovable body")
        };
        world.resource_mut::<PlayerState>().possessed = Some((body, 50, 50));
        {
            let mut map = world.resource_mut::<Map>();
            // A solid wall column: the only landing spots at distance
            // >= rad/4 are behind it.
            for y in 0..map.h {
                for x in 11..=16 {
                    map.set_terrain(x, y, map::T_GRANITE);
                }
            }
        }
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyL);
        world.run_system_once(player_input).unwrap();
        let (px, py) = player_pos_of(&mut world);
        assert!(
            px >= 17,
            "the immovable body phased through the wall: {px},{py}"
        );
    }

    /// `do_cmd_drop` / `do_cmd_destroy` (cmd3.cc:487/553): D drops the
    /// selected pack item and Shift+D destroys one.
    #[test]
    fn drop_and_destroy_selected_pack_items() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        for _ in 0..2 {
            let it = {
                let gd = world.resource::<crate::data::GameData>();
                let def = gd.object_by_name("Dagger").unwrap();
                crate::item::Item::base(gd, def)
            };
            world.resource_mut::<Inventory>().pack.push(it);
        }
        // Drop one copy: D, select 'a', Enter the quantity.
        for key in [KeyCode::KeyD, KeyCode::KeyA, KeyCode::Enter] {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            world.run_system_once(player_input).unwrap();
        }
        assert_eq!(world.resource::<Inventory>().pack.len(), 1, "one dropped");
        let mut q = world.query::<&crate::item::FloorItem>();
        assert_eq!(q.iter(&world).count(), 1, "the pile exists");
        // Destroy the other copy with Shift+D.
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyD);
        }
        world.run_system_once(player_input).unwrap();
        for key in [KeyCode::KeyA, KeyCode::Enter] {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            world.run_system_once(player_input).unwrap();
        }
        assert!(world.resource::<Inventory>().pack.is_empty(), "destroyed");
    }

    /// `compare_monster_experience` / `compare_monster_level` /
    /// `compare_player_kills` (cmd3.cc:1201-1255): the recall sort keys
    /// order by exp, then depth (Morgoth last), then player kills.
    #[test]
    fn knowledge_sort_comparators_follow_cmd3() {
        let gd = crate::data::load_game_data();
        let shallow = gd
            .monsters
            .iter()
            .position(|m| m.depth == 1 && m.exp < 10)
            .unwrap();
        let deep = gd
            .monsters
            .iter()
            .position(|m| m.depth == 60)
            .unwrap();
        assert!(game::compare_monster_level(&gd, shallow, deep));
        assert!(!game::compare_monster_level(&gd, deep, shallow));
        if let Some(morgoth) = gd.monsters.iter().position(|m| m.id == 862) {
            assert!(!game::compare_monster_level(&gd, morgoth, deep));
        }
        let mut kills = std::collections::HashMap::new();
        kills.insert(gd.monsters[deep].id, 1);
        assert!(game::compare_player_kills(&gd, &kills, shallow, deep));
        assert!(!game::compare_player_kills(&gd, &kills, deep, shallow));
        // plural_aux (cmd4.cc:3240) special-cases the Disembodied hand.
        assert!(game::plural_aux("Disembodied hand").contains("Disembodied hands"));
    }

    /// `do_cmd_sense_grid_mana` (cmd3.cc:1551): a skilled device user reads
    /// the grid's mana; low skill fails.
    #[test]
    fn sensing_grid_mana_checks_the_device_skill() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        world
            .resource_mut::<PlayerState>()
            .skills
            .insert(crate::skill::SK_DEVICE, 50 * crate::skill::SKILL_STEP);
        {
            let mut map = world.resource_mut::<Map>();
            map.mana = vec![0u8; (map::MAP_W * map::MAP_H) as usize];
            map.mana[Map::idx(10, 10)] = 100;
        }
        crate::rng::set_quick_rng(0x5EED_ACE);
        let _rng_guard = QuickRngGuard;
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::BracketRight);
        }
        world.run_system_once(player_input).unwrap();
        assert!(world
            .resource::<MessageLog>()
            .lines
            .iter()
            .any(|l| l.contains("The grid holds about")));
    }

    /// `do_cmd_power` (powers.cc:1190): Shift+U opens the power selector
    /// (the modal itself is exercised in modal.rs).
    #[test]
    fn shift_u_opens_the_power_selector() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = input_world();
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyU);
        }
        world.run_system_once(player_input).unwrap();
        assert!(*world.resource::<Modal>() == Modal::Powers { cursor: 0 });
        // Without shift, U is the Possessor body action instead.
        let mut world = input_world();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyU);
        world.run_system_once(player_input).unwrap();
        assert!(*world.resource::<Modal>() != Modal::Powers { cursor: 0 });
    }
}
