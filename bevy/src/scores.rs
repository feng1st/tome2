//! Persistent high scores: recorded on death (files.cc top_twenty) and
//! shown by the town history, race legends and busts-of-kings building
//! actions. Stored in `scores.ron` next to the savegame.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::GameData;
use crate::game::{PlayerState, PlotQuest};

/// Keep the list bounded (original MAX_HISCORES).
const MAX_SCORES: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoreEntry {
    pub name: String,
    pub race: String,
    pub class: String,
    pub level: u32,
    pub max_depth: u32,
    pub exp: u64,
    pub turns: u64,
    pub gold: i32,
    pub score: u64,
    /// Cause of death (not tracked yet; "Death" for now).
    #[serde(default)]
    pub how: String,
    pub won: bool,
}

fn path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/scores.ron").to_string()
}

#[derive(Resource)]
pub struct HighScores {
    pub entries: Vec<ScoreEntry>,
}

impl Default for HighScores {
    fn default() -> Self {
        let entries = std::fs::read_to_string(path())
            .map(|s| Self::decode(&s))
            .unwrap_or_default();
        HighScores { entries }
    }
}

impl HighScores {
    fn save(&self) {
        if let Some(s) = Self::encode(&self.entries) {
            let _ = std::fs::write(path(), s);
        }
    }

    /// `highscore_write` (hiscore.cc:20): serialize the record list; the
    /// original writes one fixed 128-byte `high_score` record.
    pub fn encode(entries: &[ScoreEntry]) -> Option<String> {
        ron::to_string(entries).ok()
    }

    /// `highscore_read` (hiscore.cc:14): deserialize the record list.
    pub fn decode(text: &str) -> Vec<ScoreEntry> {
        ron::from_str(text).unwrap_or_default()
    }

    /// `highscore_seek` + `highscore_read` (hiscore.cc:8/14):
    /// positional record access; the port holds the list in memory.
    pub fn entry(&self, i: usize) -> Option<&ScoreEntry> {
        self.entries.get(i)
    }

    /// `highscore_add` (hiscore.cc:49): find the slot with
    /// `highscore_where` (`files::score_placement`), slide the lower
    /// scores down one and drop whatever falls past MAX_HISCORES.
    /// Returns the slot used.
    pub fn insert_entry(&mut self, entry: ScoreEntry) -> usize {
        let scores: Vec<u64> = self.entries.iter().map(|e| e.score).collect();
        let slot = crate::files::score_placement(&scores, entry.score);
        self.entries
            .insert(slot.min(self.entries.len()), entry);
        self.entries.truncate(MAX_SCORES);
        slot
    }

    /// The static `quest[]` danger levels (tables.cc:2405).  Quest 0 is
    /// the empty slot and quests 26/29 carry -1 (bounty, god).
    fn quest_level(id: u32) -> i64 {
        match id {
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
            26 => -1,
            27 => 20,
            28 => 35,
            29 => -1,
            _ => 0,
        }
    }

    /// total_points (files.cc:3942): the full score, including the option
    /// multiplier and the companion-death divisor.  `companion_killed` and
    /// `options` come from the caller because `record` is invoked from
    /// `game.rs::cleanup_level`, which only has the player state.
    pub fn compute_score_full(
        gd: &GameData,
        ps: &PlayerState,
        plot: &PlotQuest,
        companion_killed: i32,
        options: &crate::options::Options,
    ) -> u64 {
        let lev = ps.level as i64;
        let mut temp = lev * lev * lev * lev + 100 * ps.max_depth as i64;
        // p_ptr->max_exp / 5: the port only tracks the current exp.
        temp += ps.exp as i64 / 5;
        // mult starts at 20 and the original's maximize penalty is always
        // applied; preserve is a birth option not tracked by the port.
        let mut mult: i64 = 20;
        if options.preserve {
            mult -= 1;
        }
        mult -= 1;
        if options.auto_scum {
            mult -= 4;
        }
        if options.small_levels {
            mult += if options.always_small_level { 4 } else { 10 };
        }
        if options.empty_levels {
            mult += 2;
        }
        if options.smart_learn {
            mult += 4;
        }
        if mult < 2 {
            mult = 2;
        }
        temp = temp * mult / 20;
        temp += ps.gold.max(0) as i64 / 5;
        // Completing quests adds 2000 + level*100 each (negative levels
        // for bounty/god are kept as-is, like the original long math).
        for (id, status) in &plot.states {
            if *status >= 2 {
                temp += 2000 + Self::quest_level(*id) * 100;
            }
        }
        // Death of a companion is BAD: `temp /= (killed * 2 / 5)` with a
        // floor of 1.
        let comp_death = ((companion_killed * 2 / 5).max(1)) as i64;
        temp /= comp_death;
        let mut kill_pts: i64 = 0;
        for (def_idx, n) in &ps.kills {
            if gd
                .monsters
                .get(*def_idx as usize)
                .map(|m| m.unique)
                .unwrap_or(false)
            {
                kill_pts += 50;
            } else {
                kill_pts += *n as i64;
            }
        }
        temp += kill_pts * 50;
        if plot.won || plot.ultra_won {
            temp += 1_000_000;
        }
        temp.max(0) as u64
    }

    /// total_points with the port's defaults (no options resource at the
    /// `record` call site; see `compute_score_full`).
    pub fn compute_score(gd: &GameData, ps: &PlayerState, plot: &PlotQuest) -> u64 {
        Self::compute_score_full(gd, ps, plot, 0, &crate::options::Options::default())
    }

    /// Record the final score of the current character (death only; the
    /// original records winners when they eventually die too).
    pub fn record(&mut self, gd: &GameData, ps: &PlayerState, plot: &PlotQuest) {
        let score = Self::compute_score(gd, ps, plot);
        // files.cc:4712 stores `game->died_from` truncated to 31 chars.
        let how = if ps.died_from.is_empty() {
            "Death".to_string()
        } else {
            ps.died_from.chars().take(31).collect()
        };
        let entry = ScoreEntry {
            name: if ps.name.is_empty() {
                "Anonymous".to_string()
            } else {
                ps.name.clone()
            },
            race: ps.race_name.clone(),
            class: ps.class_name.clone(),
            level: ps.level,
            max_depth: ps.max_depth,
            exp: ps.exp,
            turns: ps.turn,
            gold: ps.gold,
            score,
            how,
            won: plot.won || plot.ultra_won,
        };
        // Keep the file invariant the original relies on (the stored
        // records are sorted); then insert at the highscore_where slot.
        self.entries.sort_by(|a, b| b.score.cmp(&a.score));
        self.insert_entry(entry);
        self.save();
    }

    /// Entries of one race, best first.
    pub fn by_race<'a>(&'a self, race: &str) -> impl Iterator<Item = &'a ScoreEntry> {
        let race = race.to_string();
        self.entries.iter().filter(move |e| e.race == race)
    }

    /// Entries of one class, best first.
    pub fn by_class<'a>(&'a self, class: &str) -> impl Iterator<Item = &'a ScoreEntry> {
        let class = class.to_string();
        self.entries.iter().filter(move |e| e.class == class)
    }

    /// "Busts of Greatest Kings": only legendary level-50 characters.
    pub fn kings(&self) -> impl Iterator<Item = &ScoreEntry> {
        self.entries.iter().filter(|e| e.level >= 50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with_score(score: u64) -> ScoreEntry {
        ScoreEntry {
            name: format!("Hero{score}"),
            race: "Human".to_string(),
            class: "Warrior".to_string(),
            level: 1,
            max_depth: 1,
            exp: 0,
            turns: 0,
            gold: 0,
            score,
            how: "Death".to_string(),
            won: false,
        }
    }

    #[test]
    fn highscore_slots_follow_hiscore_cc() {
        let mut hs = HighScores {
            entries: Vec::new(),
        };
        // highscore_add slides the list down from the highscore_where slot.
        assert_eq!(hs.insert_entry(entry_with_score(100)), 0);
        assert_eq!(hs.insert_entry(entry_with_score(50)), 1);
        assert_eq!(hs.insert_entry(entry_with_score(10)), 2);
        assert_eq!(hs.insert_entry(entry_with_score(75)), 1);
        assert_eq!(
            hs.entries.iter().map(|e| e.score).collect::<Vec<_>>(),
            vec![100, 75, 50, 10]
        );
        // highscore_seek/read: positional record access.
        assert_eq!(hs.entry(0).unwrap().score, 100);
        assert_eq!(hs.entry(3).unwrap().score, 10);
        assert!(hs.entry(4).is_none());
        // A full list caps at the last slot (MAX_HISCORES - 1).
        let mut full = HighScores {
            entries: vec![entry_with_score(10); MAX_SCORES],
        };
        assert_eq!(full.insert_entry(entry_with_score(5)), MAX_SCORES - 1);
        assert_eq!(full.entries.len(), MAX_SCORES);
        assert_eq!(full.entries[MAX_SCORES - 1].score, 5);
        // highscore_write/read round-trip through the record text.
        let text = HighScores::encode(&hs.entries).expect("encode");
        let back = HighScores::decode(&text);
        assert_eq!(back, hs.entries);
        assert!(HighScores::decode("not ron").is_empty());
    }

    #[test]
    fn score_prefers_stronger_characters() {
        let gd = crate::data::load_game_data();
        let mut weak = crate::birth::make_player(&gd, "Weak".into(), 0, 0);
        weak.level = 5;
        weak.max_depth = 3;
        let mut strong = weak.clone();
        strong.level = 40;
        strong.max_depth = 80;
        let plot = PlotQuest::default();
        assert!(
            HighScores::compute_score(&gd, &strong, &plot)
                > HighScores::compute_score(&gd, &weak, &plot)
        );
        let winner = strong.clone();
        let mut won_plot = PlotQuest::default();
        won_plot.won = true;
        assert!(
            HighScores::compute_score(&gd, &winner, &won_plot)
                > HighScores::compute_score(&gd, &winner, &plot)
        );
    }

    #[test]
    fn compute_score_full_follows_total_points() {
        use crate::options::Options;
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Scorer".into(), 0, 0);
        ps.level = 20;
        ps.max_depth = 30;
        ps.exp = 100_000;
        ps.gold = 5_000;
        let mut plot = PlotQuest::default();
        plot.set(4, 2); // Thieves!: 2000 + 5*100
        let opts = Options::default();
        let base = HighScores::compute_score_full(&gd, &ps, &plot, 0, &opts);
        // A second completed quest adds its own 2000 + level*100: quest
        // 22 (Wolves!) is level 15.
        let mut plot2 = plot.clone();
        plot2.set(22, 2);
        assert_eq!(
            HighScores::compute_score_full(&gd, &ps, &plot2, 0, &opts) - base,
            2000 + 15 * 100
        );
        // quest levels are the static tables.cc values.
        assert_eq!(HighScores::quest_level(17), 99);
        assert_eq!(HighScores::quest_level(26), -1);
        // Companion deaths divide the total: 5 killed = /2.
        assert_eq!(
            HighScores::compute_score_full(&gd, &ps, &plot, 5, &opts),
            base / 2
        );
        assert_eq!(
            HighScores::compute_score_full(&gd, &ps, &plot, 1, &opts),
            base
        );
        // Options move the multiplier: auto_scum costs 4 points of mult,
        // small_levels adds 10 (not always-small).
        let mut scum_off = opts.clone();
        scum_off.auto_scum = false;
        assert!(HighScores::compute_score_full(&gd, &ps, &plot, 0, &scum_off) > base);
        let mut small_off = opts.clone();
        small_off.small_levels = false;
        assert!(HighScores::compute_score_full(&gd, &ps, &plot, 0, &small_off) < base);
    }

    #[test]
    fn record_uses_the_real_death_cause() {
        let gd = crate::data::load_game_data();
        let mut ps = crate::birth::make_player(&gd, "Fallen".into(), 0, 0);
        let plot = PlotQuest::default();
        let mut hs = HighScores {
            entries: Vec::new(),
        };
        ps.died_from = "a Cave Spider".to_string();
        hs.record(&gd, &ps, &plot);
        assert_eq!(hs.entries[0].how, "a Cave Spider");
        // The record keeps the original's 31-character truncation.
        ps.died_from = "x".repeat(80);
        hs.record(&gd, &ps, &plot);
        assert!(
            hs.entries.iter().any(|e| e.how.len() == 31),
            "truncated record missing: {:?}",
            hs.entries.iter().map(|e| &e.how).collect::<Vec<_>>()
        );
        // No recorded cause falls back to the plain label.
        ps.died_from.clear();
        hs.record(&gd, &ps, &plot);
        assert!(hs.entries.iter().any(|e| e.how == "Death"));
    }
}
