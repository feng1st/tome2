//! Minimal global options (cmd4.cc:488 interact_with_options / options.hpp).
//!
//! The port has no user-preference file or options screens; this resource
//! carries the gameplay-relevant switches so the original semantics can be
//! enforced where they belong.  Defaults mirror `options.hpp` of the C++
//! reference implementation.
//!
//! CONSUMERS (the options module is owned by the input/notes task; the
//! gameplay sites live in files owned by others):
//! - `smart_learn` (`options.hpp:51`, default **false**): C++ gates all
//!   monster learning in `monster2.cc:3651 update_smart_learn` and
//!   `melee2.cc:343 remove_bad_spells`.  The port's `game.rs` functions of
//!   the same names do not check any option yet, so monsters always learn
//!   the player's resistances.  `game.rs::monster_turns` should return
//!   early from both when `!options.smart_learn`.
//! - `auto_scum` (`generate.cc:8433`, default **true**): the level
//!   generator re-rolls "boring" levels.  `game.rs::generate_dungeon_level`
//!   currently calls `map::generate_level` once.
//! - `small_levels` / `empty_levels` (`options.hpp:52-53`, default
//!   **true**): `generate.cc:8338`; `map.rs` only honours the SMALLEST /
//!   SMALL / BIG d_info flags.
//! - `wear_confirm` (`options.hpp:38`, default **true**): C++ prompts
//!   "Really use the X {cursed}?" for known cursed items (`cmd3.cc:249`).
//!   `modal.rs::wield_item` should call [`wear_confirm_needed`].
//! - `always_pickup` (`options.hpp`, default false): `cmd1.cc:453 carry`
//!   and `move_player_aux` call `py_pickup_floor` on every step.  The port
//!   keeps the false default (only ammo is quivered automatically).
//! - `confirm_stairs` (`options.hpp`, default true): `cmd2.cc:145
//!   ask_leave`; the port confirms only `DF_ASK_LEAVE` dungeons.
//! - `no_selling`: consumed by `town::ShopCtx` already (fed from callers,
//!   not from this resource yet).
//! - `find_ignore_doors` / `find_ignore_stairs` / `find_examine` /
//!   `find_cut` (cmd1.cc run algorithm): the port's run code hardcodes the
//!   C++ defaults.
//! - `autosave_freq` (`cmd4.cc:647`): the port saves only on quit.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::GameData;
use crate::item::Item;

/// Player preferences with gameplay impact (subset of `options.hpp`).
/// Some fields have no consumer yet (see the module docs); they are kept
/// so the consumer work is a one-line `options.field` read.
#[allow(dead_code)]
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// Monsters learn the player's resistances (default false).
    pub smart_learn: bool,
    /// Preserve artifacts across characters (options.hpp:86, default
    /// true): penalized in the score multiplier (files.cc total_points).
    pub preserve: bool,
    /// Re-roll boring levels (default true).
    pub auto_scum: bool,
    /// Randomly small levels (default true).
    pub small_levels: bool,
    /// Randomly empty levels (default true).
    pub empty_levels: bool,
    /// Always generate the smallest possible level.
    pub always_small_level: bool,
    /// Doors are always generated in room corners/crosses.
    pub ironman_rooms: bool,
    /// Confirm wielding a known cursed item (default true).
    pub wear_confirm: bool,
    /// Pick items up when walking over them (default false).
    pub always_pickup: bool,
    /// Ask before leaving a unique level (default true).
    pub confirm_stairs: bool,
    /// Shops refuse to buy from the player (default false).
    pub no_selling: bool,
    /// The run algorithm ignores doors (default true).
    pub find_ignore_doors: bool,
    /// The run algorithm stops at stairs (default false).
    pub find_ignore_stairs: bool,
    /// Stop running to examine interesting features (default true).
    pub find_examine: bool,
    /// Cut corners while running (default false).
    pub find_cut: bool,
    /// Monsters flow by sound toward the player's latest location
    /// (options.hpp:50, default **false**): melee2.cc find_safety uses the
    /// player's flow cost grid when set.
    pub flow_by_sound: bool,
    /// Autosave every N turns (0 = off).
    pub autosave_freq: u32,
    /// Reduce the light radius while running (options.hpp).
    pub view_reduce_lite: bool,
    /// Autosave before entering a new level (options.hpp autosave_l).
    pub autosave_l: bool,
    /// Timed autosave enabled (options.hpp autosave_t).
    pub autosave_t: bool,
    /// Cheat options (options.hpp:99-103), reset by `reset_cheat_options`.
    /// The port has no wizard command set yet; the fields are kept so the
    /// reset semantics are testable.
    pub cheat_peek: bool,
    pub cheat_hear: bool,
    pub cheat_room: bool,
    pub cheat_xtra: bool,
    pub cheat_live: bool,
}

impl Default for Options {
    fn default() -> Self {
        // options.hpp defaults; the gameplay ones with real impact are
        // documented per-field above.
        Options {
            smart_learn: false,
            preserve: true,
            auto_scum: true,
            small_levels: true,
            empty_levels: true,
            always_small_level: false,
            ironman_rooms: false,
            wear_confirm: true,
            always_pickup: false,
            confirm_stairs: true,
            no_selling: false,
            find_ignore_doors: true,
            find_ignore_stairs: false,
            find_examine: true,
            find_cut: false,
            flow_by_sound: false,
            autosave_freq: 0,
            view_reduce_lite: false,
            autosave_l: false,
            autosave_t: false,
            cheat_peek: false,
            cheat_hear: false,
            cheat_room: false,
            cheat_xtra: false,
            cheat_live: false,
        }
    }
}

impl Options {
    /// `options::reset_cheat_options` (options.cc:3): the five cheats the
    /// birth flow clears before character creation (birth.cc:670).
    pub fn reset_cheat_options(&mut self) {
        self.cheat_peek = false;
        self.cheat_hear = false;
        self.cheat_room = false;
        self.cheat_xtra = false;
        self.cheat_live = false;
    }
}

/// The `wear_confirm` gate of `cmd3.cc:249`: a known cursed item asks
/// "Really use the X {cursed}?".  `known` is `Inventory::known`; `gd` is
/// kept for API symmetry with the other option helpers.
#[allow(dead_code)]
pub fn wear_confirm_needed(
    _gd: &GameData,
    it: &Item,
    known: &std::collections::HashSet<usize>,
) -> bool {
    it.cursed && (it.identified || known.contains(&it.def))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;

    #[test]
    fn defaults_match_options_hpp() {
        let o = Options::default();
        assert!(!o.smart_learn, "smart_learn defaults off");
        assert!(o.auto_scum, "auto_scum defaults on");
        assert!(o.small_levels && o.empty_levels);
        assert!(o.wear_confirm);
        assert!(!o.always_pickup);
    }

    #[test]
    fn wear_confirm_only_when_known_cursed() {
        let gd = load_game_data();
        let def = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_SWORD)
            .unwrap();
        let known = std::collections::HashSet::new();
        let mut it = Item::base(&gd, def);
        assert!(!wear_confirm_needed(&gd, &it, &known));
        it.cursed = true;
        // Unknown curse: no prompt (object_known_p is false).
        assert!(!wear_confirm_needed(&gd, &it, &known));
        it.identified = true;
        assert!(wear_confirm_needed(&gd, &it, &known));
    }

    #[test]
    fn reset_cheat_options_clears_the_five_cheats() {
        // options.cc:3 clears the five cheat flags; everything else is
        // left alone.
        let mut o = Options {
            cheat_peek: true,
            cheat_hear: true,
            cheat_room: true,
            cheat_xtra: true,
            cheat_live: true,
            smart_learn: true,
            ..Default::default()
        };
        o.reset_cheat_options();
        assert!(!o.cheat_peek);
        assert!(!o.cheat_hear);
        assert!(!o.cheat_room);
        assert!(!o.cheat_xtra);
        assert!(!o.cheat_live);
        assert!(o.smart_learn, "unrelated options survive the reset");
        // The defaults are already clean.
        let mut d = Options::default();
        d.reset_cheat_options();
        assert!(!d.cheat_peek && !d.cheat_live);
    }
}
