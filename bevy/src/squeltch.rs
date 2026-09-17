//! The Automatizer (src/squeltch.cc + src/squelch/*.cc, ToME 2.4).
//!
//! Ported rule model: a rule has a name, an action (destroy / pickup /
//! inscribe) and an optional condition tree.  Conditions can test the
//! item (`tval`, `sval`, `name`, `contain`, `symbol`, `inscription`,
//! `discount`, `status`), the player (`level`, `skill`, `ability`,
//! `race`, `subrace`, `class`) or the inventory/equipment (a subcondition
//! matched against any carried item).  `and`/`or`/`not` group them.
//!
//! Rules are matched in order and the first match wins
//! (`Automatizer::apply_rules`, automatizer.cc:52).  Application sites
//! live in `input.rs`: floor piles on walk/after a kill (`squeltch_grid`,
//! squeltch.cc:63) and the pack every turn (`squeltch_inventory`,
//! squeltch.cc:90).
//!
//! The original editor and `.atm` JSON files are replaced by a RON file
//! (`automat.ron` in the crate root, or `<player>.automat.ron`), loaded
//! on entering the game.  Rules default to empty, so the subsystem is
//! inert until the player adds rules through `do_cmd_destroy` ("$"
//! toggle in the original item selection).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::{self, GameData};
use crate::game::PlayerState;
use crate::item::{self, Inventory, Item};

/// `squelch::status_type` (object_status.hpp): item quality classes.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusType {
    Bad,
    VeryBad,
    Average,
    Good,
    VeryGood,
    Special,
    Terrible,
    None,
}

impl StatusType {
    /// status_mapping() (object_status.cc:13).
    #[allow(dead_code)]
    pub fn name(self) -> &'static str {
        match self {
            StatusType::Bad => "bad",
            StatusType::VeryBad => "very bad",
            StatusType::Average => "average",
            StatusType::Good => "good",
            StatusType::VeryGood => "very good",
            StatusType::Special => "special",
            StatusType::Terrible => "terrible",
            StatusType::None => "none",
        }
    }

    /// Parse the string form (`status_mapping().parse`).
    #[allow(dead_code)]
    pub fn parse(s: &str) -> Option<StatusType> {
        Some(match s.to_ascii_lowercase().as_str() {
            "bad" => StatusType::Bad,
            "very bad" => StatusType::VeryBad,
            "average" => StatusType::Average,
            "good" => StatusType::Good,
            "very good" => StatusType::VeryGood,
            "special" => StatusType::Special,
            "terrible" => StatusType::Terrible,
            "none" => StatusType::None,
            _ => return Option::None,
        })
    }
}

/// A condition node (squelch/condition.hpp).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// `TvalCondition`: item tval equals.
    Tval { tval: i32 },
    /// `SvalCondition`: aware item sval in `min..=max`.
    Sval { min: i32, max: i32 },
    /// `NameCondition`: full description equals (case-insensitive).
    Name { name: String },
    /// `ContainCondition`: description contains (case-insensitive).
    Contain { contain: String },
    /// `SymbolCondition`: the object's glyph.
    Symbol { symbol: String },
    /// `InscriptionCondition`: inscription contains.
    Inscription { inscription: String },
    /// `DiscountCondition`: aware item discount in `min..=max`.
    Discount { min: i32, max: i32 },
    /// `StatusCondition`: the item's quality class.
    Status { status: StatusType },
    /// `LevelCondition`: the player's level in `min..=max`.
    Level { min: i32, max: i32 },
    /// `SkillCondition`: `get_skill(name)` in `min..=max`.
    Skill { name: String, min: i32, max: i32 },
    /// `AbilityCondition`: the player has the ability.
    Ability { ability: String },
    /// `RaceCondition`: the player's race title.
    Race { race: String },
    /// `SubraceCondition`: the player's subrace title.
    Subrace { subrace: String },
    /// `ClassCondition`: the player's class/spec title.
    Class { class: String },
    /// `InventoryCondition`: any inventory item matches.
    Inventory { condition: Option<Box<Condition>> },
    /// `EquipmentCondition`: any worn item matches.
    Equipment { condition: Option<Box<Condition>> },
    /// `AndCondition`: all subconditions match.
    And { conditions: Vec<Condition> },
    /// `OrCondition`: at least one subcondition matches.
    Or { conditions: Vec<Condition> },
    /// `NotCondition`: the subcondition does not match.
    Not { condition: Option<Box<Condition>> },
}

/// Rule action (squelch/rule.hpp action_type).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RuleAction {
    /// `DestroyRule::do_apply_rule` (rule.cc:225).
    Destroy,
    /// `PickUpRule::do_apply_rule` (rule.cc:270).
    Pickup,
    /// `InscribeRule::do_apply_rule` (rule.cc:306).
    Inscribe { inscription: String },
}

/// `action_mapping()` string (rule.cc:20).
pub fn action_name(action: &RuleAction) -> &'static str {
    match action {
        RuleAction::Destroy => "destroy",
        RuleAction::Pickup => "pickup",
        RuleAction::Inscribe { .. } => "inscribe",
    }
}

/// One automatizer rule (squelch/rule.hpp).  The on-disk shape mirrors the
/// original JSON (`name`, `action`, optional `inscription`, optional
/// `condition`) but is stored as RON.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Rule {
    pub name: String,
    /// Ignored by the port (single module); kept for file compatibility.
    #[serde(default = "default_module")]
    pub module: String,
    /// `action_mapping().stringify` value: destroy / pickup / inscribe.
    pub action: String,
    /// `InscribeRule`'s inscription.
    #[serde(default)]
    pub inscription: Option<String>,
    #[serde(default)]
    pub condition: Option<Condition>,
}

pub fn default_module() -> String {
    "Tome".to_string()
}

impl Rule {
    /// Build a rule from its action form.
    pub fn new(name: &str, action: RuleAction, condition: Option<Condition>) -> Rule {
        let (action, inscription) = match &action {
            RuleAction::Destroy => ("destroy".to_string(), None),
            RuleAction::Pickup => ("pickup".to_string(), None),
            RuleAction::Inscribe { inscription } => {
                ("inscribe".to_string(), Some(inscription.clone()))
            }
        };
        Rule {
            name: name.to_string(),
            module: default_module(),
            action,
            inscription,
            condition,
        }
    }

    /// Parse the action (rule.cc Rule::parse_rule).  Invalid actions are
    /// rejected like the original.
    pub fn parsed_action(&self) -> Option<RuleAction> {
        match self.action.to_ascii_lowercase().as_str() {
            "destroy" => Some(RuleAction::Destroy),
            "pickup" => Some(RuleAction::Pickup),
            "inscribe" => self
                .inscription
                .clone()
                .map(|inscription| RuleAction::Inscribe { inscription }),
            _ => None,
        }
    }
}

/// Everything a condition can inspect about the player/inventory plus the
/// description and awareness flags of the item being tested.
pub struct MatchCtx<'a> {
    pub gd: &'a GameData,
    pub inv: &'a Inventory,
    pub ps: &'a PlayerState,
    /// `object_desc(o_ptr, -1, 0)` of the tested item.
    pub label: String,
    /// `object_aware_p(o_ptr)`.
    pub aware: bool,
}

impl<'a> MatchCtx<'a> {
    pub fn new(gd: &'a GameData, inv: &'a Inventory, ps: &'a PlayerState, it: &Item) -> Self {
        MatchCtx {
            gd,
            inv,
            ps,
            label: item_desc(gd, inv, it),
            aware: object_aware_p(gd, inv, it),
        }
    }
}

/// The item's description with its ego/artifact name, mirroring
/// `object_desc(buf, o_ptr, -1, 0)` (object1.cc).
pub fn item_desc(gd: &GameData, inv: &Inventory, it: &Item) -> String {
    it.label(gd, &inv.known)
}

/// `object_aware_p` (object1.cc): the *kind* is known.  Base kinds are
/// always aware; flavoured consumables/jewelry need identification.
pub fn object_aware_p(gd: &GameData, inv: &Inventory, it: &Item) -> bool {
    let o = &gd.objects[it.def];
    if it.identified || inv.known.contains(&it.def) {
        return true;
    }
    !matches!(
        o.tval,
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

/// `object_status` (object_status.cc:29): artifact/ego/slot based quality.
pub fn object_status(gd: &GameData, it: &Item) -> StatusType {
    // object_known_p: the instance is fully identified.
    if !it.identified {
        return StatusType::None;
    }
    let slot = data::slot_of_item(&gd.objects[it.def]);
    if item::is_artifact(gd, it) {
        return if it.cursed {
            StatusType::Terrible
        } else {
            StatusType::Special
        };
    }
    if it.ego != 0 {
        return if it.cursed {
            StatusType::VeryBad
        } else {
            StatusType::VeryGood
        };
    }
    match slot {
        Some(data::SLOT_WEAPON)
        | Some(data::SLOT_BOW)
        | Some(data::SLOT_QUIVER)
        | Some(data::SLOT_TOOL) => {
            if it.to_h + it.to_d < 0 {
                StatusType::Bad
            } else if it.to_h + it.to_d > 0 {
                StatusType::Good
            } else {
                StatusType::Average
            }
        }
        Some(s) if (data::SLOT_BODY..=data::SLOT_FEET).contains(&s) => {
            if it.to_a < 0 {
                StatusType::Bad
            } else if it.to_a > 0 {
                StatusType::Good
            } else {
                StatusType::Average
            }
        }
        Some(data::SLOT_RING1) => {
            if it.to_d + it.to_h < 0 || it.to_a < 0 || it.pval < 0 {
                StatusType::Bad
            } else {
                StatusType::Average
            }
        }
        Some(data::SLOT_AMULET) => {
            if it.pval < 0 {
                StatusType::Bad
            } else {
                StatusType::Average
            }
        }
        _ => StatusType::Average,
    }
}

/// object_status is only reached with `object_known_p`; the status action
/// from `easy_add_rule` may be built from an unidentified item's instance
/// data, which the original still accepts (it calls object_status which
/// returns NONE).
impl Condition {
    /// `Condition::is_match` (condition.cc).
    pub fn is_match(&self, it: &Item, ctx: &MatchCtx) -> bool {
        match self {
            Condition::Tval { tval } => ctx.gd.objects[it.def].tval == *tval,
            Condition::Sval { min, max } => {
                ctx.aware
                    && (ctx.gd.objects[it.def].sval >= *min && ctx.gd.objects[it.def].sval <= *max)
            }
            Condition::Name { name } => ctx.label.eq_ignore_ascii_case(name),
            Condition::Contain { contain } => ctx
                .label
                .to_ascii_lowercase()
                .contains(&contain.to_ascii_lowercase()),
            Condition::Symbol { symbol } => {
                let ch = ctx.gd.objects[it.def].ch.chars().next();
                symbol
                    .chars()
                    .next()
                    .map(|c| Some(c) == ch)
                    .unwrap_or(false)
            }
            Condition::Inscription { inscription } => it
                .inscription
                .to_ascii_lowercase()
                .contains(&inscription.to_ascii_lowercase()),
            Condition::Discount { min, max } => {
                ctx.aware && it.discount >= *min && it.discount <= *max
            }
            Condition::Status { status } => *status == object_status(ctx.gd, it),
            Condition::Level { min, max } => {
                let l = ctx.ps.level as i32;
                l >= *min && l <= *max
            }
            Condition::Skill { name, min, max } => {
                let Some(def) = ctx
                    .gd
                    .skills
                    .iter()
                    .find(|s| s.name.eq_ignore_ascii_case(name))
                else {
                    return false;
                };
                let v = ctx.ps.skill(def.id);
                v >= *min && v <= *max
            }
            Condition::Ability { ability } => ctx
                .gd
                .abilities
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case(ability))
                .is_some_and(|a| ctx.ps.has_ability(a.id)),
            Condition::Race { race } => ctx.ps.race_name.eq_ignore_ascii_case(race),
            Condition::Subrace { subrace } => ctx
                .gd
                .racemods
                .get(ctx.ps.subrace as usize)
                .map(|r| r.name.eq_ignore_ascii_case(subrace))
                .unwrap_or(false),
            Condition::Class { class } => ctx.ps.class_name.eq_ignore_ascii_case(class),
            Condition::Inventory { condition } => condition.as_ref().is_some_and(|c| {
                ctx.inv
                    .pack
                    .iter()
                    .any(|i| c.is_match(i, &MatchCtx::new(ctx.gd, ctx.inv, ctx.ps, i)))
            }),
            Condition::Equipment { condition } => condition.as_ref().is_some_and(|c| {
                ctx.inv
                    .equip
                    .iter()
                    .flatten()
                    .any(|i| c.is_match(i, &MatchCtx::new(ctx.gd, ctx.inv, ctx.ps, i)))
            }),
            Condition::And { conditions } => conditions.iter().all(|c| c.is_match(it, ctx)),
            Condition::Or { conditions } => conditions.iter().any(|c| c.is_match(it, ctx)),
            Condition::Not { condition } => match condition {
                Some(c) => !c.is_match(it, ctx),
                None => true, // NotCondition::is_match returns true with no child.
            },
        }
    }
}

impl Rule {
    /// `Rule::apply_rule` (rule.cc:117): the condition matches -> act.
    pub fn matches(&self, it: &Item, ctx: &MatchCtx) -> bool {
        self.condition.as_ref().is_some_and(|c| c.is_match(it, ctx))
    }
}

/// The automatizer singleton (`automatizer` + `automatizer_enabled`).
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Automatizer {
    /// `automatizer_enabled` (variable.cc:571, saved in loadsave.cc:2404).
    /// The original defaults to **false** and is switched on from the
    /// `= T` editor screen; the port defaults to true so loaded rules are
    /// active (the editor is replaced by the file + destroy-rule flow).
    pub enabled: bool,
    pub rules: Vec<Rule>,
    /// `automatizer_create`: when set, `do_cmd_destroy` adds a rule for
    /// the destroyed item (item selection "$" toggle).
    #[serde(skip)]
    pub create_rule: bool,
}

impl Default for Automatizer {
    fn default() -> Self {
        Automatizer {
            enabled: true,
            rules: Vec::new(),
            create_rule: false,
        }
    }
}

fn default_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/automat.ron"))
}

fn player_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/"))
        .join(format!("{}.automat.ron", name))
}

impl Automatizer {
    /// `Automatizer::apply_rules` (automatizer.cc:52): the first matching
    /// rule's action, or None.
    pub fn matching_action(&self, it: &Item, ctx: &MatchCtx) -> Option<RuleAction> {
        self.rules
            .iter()
            .find(|r| r.matches(it, ctx))
            .and_then(|r| r.parsed_action())
    }

    /// `Automatizer::append_rule` (automatizer.cc:41); returns its index.
    pub fn append_rule(&mut self, rule: Rule) -> usize {
        self.rules.push(rule);
        self.rules.len() - 1
    }

    /// Load `<player>.automat.ron` then `automat.ron`, like
    /// `dungeon.cc:4558 automatizer_load`.  Returns true when a file was
    /// read.
    pub fn load_for(&mut self, player: &str) -> bool {
        for path in [player_path(player), default_path()] {
            if let Some(rules) = load_rules_from(&path) {
                self.rules = rules;
                return true;
            }
        }
        false
    }

    /// `automatizer_save_rules` (squeltch.cc:166), RON instead of JSON.
    #[allow(dead_code)]
    pub fn save_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(&self.rules, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

/// `easy_add_rule` modes (squeltch.cc:450 add_rule_mode).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddRuleMode {
    /// Match the tval alone ("Destroy all of the same [F]amily").
    Tval,
    /// Match tval + exact sval ("Destroy all of the same [T]ype").
    TvalSval,
    /// Match the full description ("[N]ame").
    Name,
}

/// `easy_add_rule` (squeltch.cc:452): append a destroy rule for the
/// object, optionally ANDed with its status, and return its index.
pub fn easy_add_rule(
    aut: &mut Automatizer,
    gd: &GameData,
    inv: &Inventory,
    it: &Item,
    mode: AddRuleMode,
    do_status: bool,
) -> usize {
    let desc = item_desc(gd, inv, it);
    let o = &gd.objects[it.def];
    let mut condition = match mode {
        AddRuleMode::Tval => Condition::Tval { tval: o.tval },
        AddRuleMode::TvalSval => Condition::And {
            conditions: vec![
                Condition::Tval { tval: o.tval },
                Condition::Sval {
                    min: o.sval,
                    max: o.sval,
                },
            ],
        },
        AddRuleMode::Name => Condition::Name { name: desc },
    };
    if do_status {
        condition = Condition::And {
            conditions: vec![
                condition,
                Condition::Status {
                    status: object_status(gd, it),
                },
            ],
        };
    }
    aut.append_rule(Rule::new(
        action_name(&RuleAction::Destroy),
        RuleAction::Destroy,
        Some(condition),
    ))
}

/// `automatizer_load` (squeltch.cc:574) for one path: a missing file is
/// skipped (None), a parse failure warns and yields None, and success
/// replaces the rule list.
pub fn load_rules_from(path: &std::path::Path) -> Option<Vec<Rule>> {
    let text = std::fs::read_to_string(path).ok()?;
    match ron::from_str::<Vec<Rule>>(&text) {
        Ok(rules) => Some(rules),
        Err(e) => {
            warn!("automatizer: failed to parse {}: {}", path.display(), e);
            None
        }
    }
}

/// `automatizer_load` (dungeon.cc:4558): try `<player>.automat.ron`
/// first, then `automat.ron`, on entering a game.
pub fn load_rules_on_enter(ps: Res<PlayerState>, mut aut: ResMut<Automatizer>) {
    if aut.load_for(&ps.name) {
        info!(
            "automatizer: loaded {} rule(s) for {}",
            aut.rules.len(),
            ps.name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{load_game_data, TV_POTION, TV_SWORD};
    use crate::game::PlayerState;

    fn test_player(gd: &GameData) -> PlayerState {
        crate::birth::make_player(gd, "Autotest".into(), 0, 0)
    }

    fn ctx<'a>(
        gd: &'a GameData,
        inv: &'a Inventory,
        ps: &'a PlayerState,
        it: &Item,
    ) -> MatchCtx<'a> {
        MatchCtx::new(gd, inv, ps, it)
    }

    #[test]
    fn tval_and_sval_conditions_match() {
        let gd = load_game_data();
        let inv = Inventory::default();
        let ps = test_player(&gd);
        let sword_def = gd.objects.iter().position(|o| o.tval == TV_SWORD).unwrap();
        let sword = Item::base(&gd, sword_def);
        let c = ctx(&gd, &inv, &ps, &sword);
        assert!(Condition::Tval { tval: TV_SWORD }.is_match(&sword, &c));
        assert!(!Condition::Tval { tval: TV_POTION }.is_match(&sword, &c));
        assert!(Condition::Sval {
            min: gd.objects[sword_def].sval,
            max: gd.objects[sword_def].sval
        }
        .is_match(&sword, &c));
        // Sval on an unaware flavoured potion never matches.
        let mut pot = Item::base(
            &gd,
            gd.objects.iter().position(|o| o.tval == TV_POTION).unwrap(),
        );
        pot.identified = false;
        let pc = ctx(&gd, &inv, &ps, &pot);
        assert!(!pc.aware);
        assert!(!Condition::Sval { min: 0, max: 999 }.is_match(&pot, &pc));
    }

    fn sword_with(gd: &GameData, plus: i32) -> Item {
        let def = gd.objects.iter().position(|o| o.tval == TV_SWORD).unwrap();
        let mut it = Item::base(&gd, def);
        it.to_h = plus;
        it.to_d = 0;
        it.identified = true;
        it
    }

    #[test]
    fn status_condition_uses_object_status() {
        let gd = load_game_data();
        let inv = Inventory::default();
        let ps = test_player(&gd);
        let plain = sword_with(&gd, 0);
        assert_eq!(object_status(&gd, &plain), StatusType::Average);
        assert!(Condition::Status {
            status: StatusType::Average
        }
        .is_match(&plain, &ctx(&gd, &inv, &ps, &plain)));
        let good = sword_with(&gd, 2);
        assert_eq!(object_status(&gd, &good), StatusType::Good);
        let cursed = sword_with(&gd, -2);
        assert_eq!(object_status(&gd, &cursed), StatusType::Bad);
        // Artifacts are special (terrible when cursed).
        let mut art = sword_with(&gd, 5);
        art.artifact = 1;
        art.identified = true;
        assert_eq!(object_status(&gd, &art), StatusType::Special);
        art.cursed = true;
        assert_eq!(object_status(&gd, &art), StatusType::Terrible);
    }

    #[test]
    fn grouping_and_not_conditions() {
        let gd = load_game_data();
        let inv = Inventory::default();
        let ps = test_player(&gd);
        let sword = sword_with(&gd, 3);
        let c = ctx(&gd, &inv, &ps, &sword);
        let both = Condition::And {
            conditions: vec![
                Condition::Tval { tval: TV_SWORD },
                Condition::Name {
                    name: c.label.clone(),
                },
            ],
        };
        assert!(both.is_match(&sword, &c));
        let nope = Condition::And {
            conditions: vec![
                Condition::Tval { tval: TV_SWORD },
                Condition::Contain {
                    contain: "zzz".into(),
                },
            ],
        };
        assert!(!nope.is_match(&sword, &c));
        assert!(Condition::Or {
            conditions: vec![nope, Condition::Tval { tval: TV_SWORD }],
        }
        .is_match(&sword, &c));
        assert!(Condition::Not {
            condition: Some(Box::new(Condition::Tval { tval: TV_POTION })),
        }
        .is_match(&sword, &c));
    }

    #[test]
    fn inventory_and_equipment_conditions_recurse() {
        let gd = load_game_data();
        let mut inv = Inventory::default();
        let ps = test_player(&gd);
        let sword = sword_with(&gd, 4);
        let potion_def = gd.objects.iter().position(|o| o.tval == TV_POTION).unwrap();
        inv.pack.push(Item::base(&gd, potion_def));
        let mut ring = sword_with(&gd, 1);
        ring.def = gd
            .objects
            .iter()
            .position(|o| o.tval == data::TV_RING)
            .unwrap();
        inv.equip[data::SLOT_RING1] = Some(ring);
        let c = ctx(&gd, &inv, &ps, &sword);
        let any_potion = Condition::Inventory {
            condition: Some(Box::new(Condition::Tval { tval: TV_POTION })),
        };
        assert!(any_potion.is_match(&sword, &c));
        let any_ring = Condition::Equipment {
            condition: Some(Box::new(Condition::Tval {
                tval: data::TV_RING,
            })),
        };
        assert!(any_ring.is_match(&sword, &c));
        let any_bow = Condition::Inventory {
            condition: Some(Box::new(Condition::Tval { tval: data::TV_BOW })),
        };
        assert!(!any_bow.is_match(&sword, &c));
    }

    #[test]
    fn rule_order_first_match_wins() {
        let gd = load_game_data();
        let mut inv = Inventory::default();
        let ps = test_player(&gd);
        let sword = sword_with(&gd, 1);
        let mut aut = Automatizer::default();
        aut.append_rule(Rule::new(
            "first",
            RuleAction::Destroy,
            Some(Condition::Tval { tval: TV_SWORD }),
        ));
        aut.append_rule(Rule::new(
            "second",
            RuleAction::Inscribe {
                inscription: "!k".into(),
            },
            Some(Condition::Tval { tval: TV_SWORD }),
        ));
        inv.pack.push(sword.clone());
        let c = ctx(&gd, &inv, &ps, &sword);
        assert_eq!(aut.matching_action(&sword, &c), Some(RuleAction::Destroy));
    }

    #[test]
    fn easy_add_rule_builds_status_anded_rule() {
        let gd = load_game_data();
        let inv = Inventory::default();
        let sword = sword_with(&gd, 3);
        let mut aut = Automatizer::default();
        let i = easy_add_rule(&mut aut, &gd, &inv, &sword, AddRuleMode::TvalSval, true);
        assert_eq!(i, 0);
        assert_eq!(aut.rules.len(), 1);
        // The condition matches the exact kind + status, but not another.
        let ps = test_player(&gd);
        let c = ctx(&gd, &inv, &ps, &sword);
        assert!(aut.rules[0].matches(&sword, &c));
        let other = sword_with(&gd, -3);
        let co = ctx(&gd, &inv, &ps, &other);
        assert!(!aut.rules[0].matches(&other, &co));
    }

    #[test]
    fn ron_roundtrip_and_file_io() {
        let gd = load_game_data();
        let mut aut = Automatizer::default();
        aut.append_rule(Rule::new(
            "squelch junk",
            RuleAction::Destroy,
            Some(Condition::And {
                conditions: vec![
                    Condition::Tval { tval: TV_SWORD },
                    Condition::Not {
                        condition: Some(Box::new(Condition::Inscription {
                            inscription: "keep".into(),
                        })),
                    },
                ],
            }),
        ));
        aut.append_rule(Rule::new(
            "pickup potions",
            RuleAction::Pickup,
            Some(Condition::Tval { tval: TV_POTION }),
        ));
        let text =
            ron::ser::to_string_pretty(&aut.rules, ron::ser::PrettyConfig::default()).unwrap();
        let back: Vec<Rule> = ron::from_str(&text).expect("rules parse");
        assert_eq!(back, aut.rules);

        let path = std::env::temp_dir().join(format!("tome-automat-{}.ron", std::process::id()));
        aut.save_file(&path).unwrap();
        let mut loaded = Automatizer::default();
        assert!(loaded.load_file_for_test(&path));
        assert_eq!(loaded.rules.len(), 2);
        assert_eq!(loaded.rules[0].action, "destroy");
        assert_eq!(loaded.rules[0].name, "squelch junk");
        let _ = std::fs::remove_file(&path);
        let _ = gd;
    }

    /// automatizer_load (squeltch.cc:574): missing path skips, parse
    /// failure warns and skips, success replaces the rule list.
    #[test]
    fn automatizer_load_contract() {
        let dir = std::env::temp_dir();
        let missing = dir.join("tome-automat-does-not-exist.ron");
        let _ = std::fs::remove_file(&missing);
        assert!(load_rules_from(&missing).is_none(), "missing path skips");
        let bad = dir.join(format!("tome-automat-bad-{}.ron", std::process::id()));
        std::fs::write(&bad, "this is not ron [").unwrap();
        assert!(load_rules_from(&bad).is_none(), "parse failure skips");
        let good = dir.join(format!("tome-automat-good-{}.ron", std::process::id()));
        let rules = vec![Rule::new(
            "loaded",
            RuleAction::Destroy,
            Some(Condition::Tval { tval: TV_SWORD }),
        )];
        std::fs::write(&good, ron::ser::to_string(&rules).unwrap()).unwrap();
        let back = load_rules_from(&good).expect("good rules");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "loaded");
        let _ = std::fs::remove_file(&bad);
        let _ = std::fs::remove_file(&good);
    }

    #[test]
    fn item_desc_and_aware_flag() {
        let gd = load_game_data();
        let inv = Inventory::default();
        let def = gd.objects.iter().position(|o| o.tval == TV_SWORD).unwrap();
        let it = Item::base(&gd, def);
        assert!(!item_desc(&gd, &inv, &it).is_empty());
        assert!(object_aware_p(&gd, &inv, &it), "base items are aware");
    }
}

#[cfg(test)]
impl Automatizer {
    /// Test helper: load from an explicit path.
    pub(crate) fn load_file_for_test(&mut self, path: &std::path::Path) -> bool {
        match std::fs::read_to_string(path) {
            Ok(text) => match ron::from_str::<Vec<Rule>>(&text) {
                Ok(rules) => {
                    self.rules = rules;
                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}
