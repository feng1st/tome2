//! Port of the monster recall screen (`src/monster1.cc`): `roff_aux`,
//! `roff_name`, `roff_top`, `screen_roff`, `monster_description_out` and
//! `display_roff`.  The original draws to a terminal with colours; the
//! port produces the same text for the text-page modal (the Bevy UI
//! carries colours where it wants them).

use crate::data::{GameData, MonsterDef};
use crate::game::PlayerState;

/// The environment `roff_aux` reads from globals in the original.
pub struct RecallEnv {
    /// `dun_level`: the depth the player is on (for the out-of-depth
    /// colouring of the native level).
    pub dun_level: i32,
    /// `p_ptr->lev`: the player's level (experience line).
    pub player_level: i32,
    /// `r_ptr->r_pkills`: kills of this race this life.
    pub kills: u32,
    /// `r_ptr->max_num == 0`: a unique that has been slain
    /// (the port keeps `ps.kills[def] > 0` for that).
    pub unique_slain: bool,
}

fn he(msex: i32) -> &'static str {
    match msex {
        1 => "he",
        2 => "she",
        _ => "it",
    }
}

fn his(msex: i32) -> &'static str {
    match msex {
        1 => "his",
        2 => "her",
        _ => "its",
    }
}

fn cap(s: &str) -> String {
    crate::zutil::capitalize(s)
}

/// `roff_name` (monster1.cc:1262): "The <name> ('c')/('c'):".  The port's
/// data keeps a single glyph (r_info G:), so the optional x_char slot
/// repeats it.
pub fn roff_name(r: &MonsterDef) -> String {
    let mut s = String::new();
    if !r.unique {
        s.push_str("The ");
    }
    s.push_str(&r.name);
    let ch = r.ch.chars().next().unwrap_or('?');
    s.push_str(&format!(" ('{}')/('{}'):", ch, ch));
    s
}

/// `roff_top` (monster1.cc:1313).
pub fn roff_top(r: &MonsterDef) -> String {
    roff_name(r)
}

fn has(r: &MonsterDef, flag: &str) -> bool {
    r.has(flag)
}

fn spell(r: &MonsterDef, flag: &str) -> bool {
    r.spells.iter().any(|s| s == flag)
}

/// `roff_aux` (monster1.cc:53): the recall body.
pub fn roff_aux(r: &MonsterDef, env: &RecallEnv) -> String {
    let mut s = String::new();
    let msex = if has(r, "FEMALE") {
        2
    } else if has(r, "MALE") {
        1
    } else {
        0
    };
    let mut old;

    // Treat uniques differently.
    if has(r, "UNIQUE") {
        if env.unique_slain {
            s.push_str("You have slain this foe.  ");
        }
    } else if env.kills > 0 {
        s.push_str("You have killed at least ");
        s.push_str(&env.kills.to_string());
        s.push_str(" of these creatures.  ");
    } else {
        s.push_str("No battles to the death are recalled.  ");
    }

    // Descriptions.
    s.push_str(&r.desc);
    s.push_str("  ");

    // Describe location.
    old = false;
    if has(r, "PET") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is friendly to you");
        old = true;
    }
    if r.depth == 0 {
        if old {
            s.push_str(", ");
        } else {
            s.push_str(&cap(he(msex)));
            s.push(' ');
        }
        s.push_str("lives in the town or the wilderness");
        old = true;
    } else {
        if old {
            s.push_str(", ");
        } else {
            s.push_str(&cap(he(msex)));
            s.push(' ');
        }
        s.push_str("is normally found on level ");
        s.push_str(&r.depth.to_string());
        old = true;
    }

    // Describe movement.
    {
        if old {
            s.push_str(", and ");
        } else {
            s.push_str(&cap(he(msex)));
            s.push(' ');
            old = true;
        }
        s.push_str("moves");
        if has(r, "RAND_50") || has(r, "RAND_25") {
            if has(r, "RAND_50") && has(r, "RAND_25") {
                s.push_str(" extremely");
            } else if has(r, "RAND_50") {
                s.push_str(" somewhat");
            } else if has(r, "RAND_25") {
                s.push_str(" a bit");
            }
            s.push_str(" erratically");
            if r.speed != 110 {
                s.push_str(", and");
            }
        }
        if r.speed > 110 {
            if r.speed > 130 {
                s.push_str(" incredibly");
            } else if r.speed > 120 {
                s.push_str(" very");
            }
            s.push_str(" quickly");
        } else if r.speed < 110 {
            if r.speed < 90 {
                s.push_str(" incredibly");
            } else if r.speed < 100 {
                s.push_str(" very");
            }
            s.push_str(" slowly");
        } else {
            s.push_str(" at normal speed");
        }
    }

    if has(r, "NEVER_MOVE") {
        if old {
            s.push_str(", but ");
        } else {
            s.push_str(&cap(he(msex)));
            s.push(' ');
            old = true;
        }
        s.push_str("does not deign to chase intruders");
    }
    if old {
        s.push_str(".  ");
    }

    // Experience.
    {
        if has(r, "UNIQUE") {
            s.push_str("Killing this");
        } else {
            s.push_str("A kill of this");
        }
        if has(r, "ELDRITCH_HORROR") {
            s.push_str(" sanity-blasting");
        }
        if has(r, "ANIMAL") {
            s.push_str(" natural");
        }
        if has(r, "EVIL") {
            s.push_str(" evil");
        }
        if has(r, "GOOD") {
            s.push_str(" good");
        }
        if has(r, "UNDEAD") {
            s.push_str(" undead");
        }
        if has(r, "DRAGON") {
            s.push_str(" dragon");
        } else if has(r, "DEMON") {
            s.push_str(" demon");
        } else if has(r, "GIANT") {
            s.push_str(" giant");
        } else if has(r, "TROLL") {
            s.push_str(" troll");
        } else if has(r, "ORC") {
            s.push_str(" orc");
        } else if has(r, "THUNDERLORD") {
            s.push_str(" Thunderlord");
        } else if has(r, "SPIDER") {
            s.push_str(" spider");
        } else if has(r, "NAZGUL") {
            s.push_str(" Nazgul");
        } else {
            s.push_str(" creature");
        }

        let plev = env.player_level.max(1) as i64;
        let mexp = r.exp as i64;
        let level = r.depth as i64;
        let i = mexp * level / plev;
        let j = ((mexp * level % plev) * 1000 / plev + 5) / 10;
        s.push_str(" is worth ");
        s.push_str(&format!("{}.{:02}", i, j));
        s.push_str(" point");
        if !(i == 1 && j == 0) {
            s.push('s');
        }
        let mut p = "th";
        let d = env.player_level % 10;
        if env.player_level / 10 != 1 {
            if d == 1 {
                p = "st";
            } else if d == 2 {
                p = "nd";
            } else if d == 3 {
                p = "rd";
            }
        }
        let q = if matches!(env.player_level, 8 | 11 | 18) {
            "n"
        } else {
            ""
        };
        s.push_str(&format!(
            " for a{} {}{} level character.  ",
            q, env.player_level, p
        ));
    }

    if has(r, "AURA_FIRE") && has(r, "AURA_ELEC") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is surrounded by flames and electricity.  ");
    } else if has(r, "AURA_FIRE") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is surrounded by flames.  ");
    } else if has(r, "AURA_ELEC") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is surrounded by electricity.  ");
    }

    if has(r, "REFLECTING") {
        s.push_str(&cap(he(msex)));
        s.push_str(" reflects bolt spells.  ");
    }

    if has(r, "ESCORT") || has(r, "ESCORTS") {
        s.push_str(&cap(he(msex)));
        s.push_str(" usually appears with escorts.  ");
    } else if has(r, "FRIEND") || has(r, "FRIENDS") {
        s.push_str(&cap(he(msex)));
        s.push_str(" usually appears in groups.  ");
    }

    // Collect innate attacks.
    let mut vp: Vec<&str> = Vec::new();
    if spell(r, "SHRIEK") {
        vp.push("shriek for help");
    }
    if spell(r, "ROCKET") {
        vp.push("shoot a rocket");
    }
    if spell(r, "ARROW_1") {
        vp.push("fire an arrow");
    }
    if spell(r, "ARROW_2") {
        vp.push("fire arrows");
    }
    if spell(r, "ARROW_3") {
        vp.push("fire a missile");
    }
    if spell(r, "ARROW_4") {
        vp.push("fire missiles");
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" may ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" or ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    // Collect breaths.
    let mut breath = false;
    let mut magic = false;
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("BR_ACID", "acid"),
        ("BR_ELEC", "lightning"),
        ("BR_FIRE", "fire"),
        ("BR_COLD", "frost"),
        ("BR_POIS", "poison"),
        ("BR_NETH", "nether"),
        ("BR_LITE", "light"),
        ("BR_DARK", "darkness"),
        ("BR_CONF", "confusion"),
        ("BR_SOUN", "sound"),
        ("BR_CHAO", "chaos"),
        ("BR_DISE", "disenchantment"),
        ("BR_NEXU", "nexus"),
        ("BR_TIME", "time"),
        ("BR_INER", "inertia"),
        ("BR_GRAV", "gravity"),
        ("BR_SHAR", "shards"),
        ("BR_PLAS", "plasma"),
        ("BR_WALL", "force"),
        ("BR_MANA", "mana"),
        ("BR_NUKE", "toxic waste"),
        ("BR_DISI", "disintegration"),
    ] {
        if spell(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        breath = true;
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" may breathe ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" or ");
            }
            s.push_str(item);
        }
    }

    // Collect spells.
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("BA_ACID", "produce acid balls"),
        ("BA_ELEC", "produce lightning balls"),
        ("BA_FIRE", "produce fire balls"),
        ("BA_COLD", "produce frost balls"),
        ("BA_POIS", "produce poison balls"),
        ("BA_NETH", "produce nether balls"),
        ("BA_WATE", "produce water balls"),
        ("BA_NUKE", "produce balls of radiation"),
        ("BA_MANA", "invoke mana storms"),
        ("BA_DARK", "invoke darkness storms"),
        ("BA_CHAO", "invoke raw chaos"),
        ("HAND_DOOM", "invoke the Hand of Doom"),
        ("DRAIN_MANA", "drain mana"),
        ("MIND_BLAST", "cause mind blasting"),
        ("BRAIN_SMASH", "cause brain smashing"),
        ("CAUSE_1", "cause light wounds and cursing"),
        ("CAUSE_2", "cause serious wounds and cursing"),
        ("CAUSE_3", "cause critical wounds and cursing"),
        ("CAUSE_4", "cause mortal wounds"),
        ("BO_ACID", "produce acid bolts"),
        ("BO_ELEC", "produce lightning bolts"),
        ("BO_FIRE", "produce fire bolts"),
        ("BO_COLD", "produce frost bolts"),
        ("BO_POIS", "produce poison bolts"),
        ("BO_NETH", "produce nether bolts"),
        ("BO_WATE", "produce water bolts"),
        ("BO_MANA", "produce mana bolts"),
        ("BO_PLAS", "produce plasma bolts"),
        ("BO_ICEE", "produce ice bolts"),
        ("MISSILE", "produce magic missiles"),
        ("SCARE", "terrify"),
        ("BLIND", "blind"),
        ("CONF", "confuse"),
        ("SLOW", "slow"),
        ("HOLD", "paralyze"),
        ("HASTE", "haste-self"),
        ("HEAL", "heal-self"),
        ("BLINK", "blink-self"),
        ("TPORT", "teleport-self"),
        ("S_BUG", "summon software bugs"),
        ("S_RNG", "summon RNG"),
        ("TELE_TO", "teleport to"),
        ("TELE_AWAY", "teleport away"),
        ("TELE_LEVEL", "teleport level"),
        ("S_THUNDERLORD", "summon a Thunderlord"),
        ("DARKNESS", "create darkness"),
        ("FORGET", "cause amnesia"),
        ("RAISE_DEAD", "raise dead"),
        ("S_MONSTER", "summon a monster"),
        ("S_MONSTERS", "summon monsters"),
        ("S_KIN", "summon aid"),
        ("S_ANT", "summon ants"),
        ("S_SPIDER", "summon spiders"),
        ("S_HOUND", "summon hounds"),
        ("S_HYDRA", "summon hydras"),
        ("S_ANGEL", "summon an angel"),
        ("S_DEMON", "summon a demon"),
        ("S_UNDEAD", "summon an undead"),
        ("S_DRAGON", "summon a dragon"),
        ("S_ANIMAL", "summon animal"),
        ("S_ANIMALS", "summon animals"),
        ("S_HI_UNDEAD", "summon Greater Undead"),
        ("S_HI_DRAGON", "summon Ancient Dragons"),
        ("S_HI_DEMON", "summon Greater Demons"),
        ("S_WRAITH", "summon Ringwraith"),
        ("S_UNIQUE", "summon Unique Monsters"),
    ] {
        if spell(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        magic = true;
        if breath {
            s.push_str(", and is also");
        } else {
            s.push_str(&cap(he(msex)));
            s.push_str(" is");
        }
        s.push_str(" magical, casting spells");
        if has(r, "SMART") {
            s.push_str(" intelligently");
        }
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" which ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" or ");
            }
            s.push_str(item);
        }
    }

    if breath || magic {
        let freq = if r.spell_freq > 0 {
            100 / r.spell_freq
        } else {
            0
        };
        let n = (freq + freq) / 2;
        s.push_str("; 1 time in ");
        s.push_str(&(100 / n.max(1)).to_string());
        s.push_str(".  ");
    }

    // Toughness.
    {
        s.push_str(&cap(he(msex)));
        s.push_str(" has an armor rating of ");
        s.push_str(&r.ac.to_string());
        let hd = r.hdice.parse::<i32>().unwrap_or(0);
        let hs = r.hside.parse::<i32>().unwrap_or(0);
        if has(r, "FORCE_MAXHP") {
            s.push_str(" and a life rating of ");
            s.push_str(&(hd * hs).to_string());
            s.push_str(".  ");
        } else {
            s.push_str(" and a life rating of ");
            s.push_str(&format!("{}d{}", hd, hs));
            s.push_str(".  ");
        }
    }

    // Special abilities.
    let mut vp: Vec<&str> = Vec::new();
    if has(r, "OPEN_DOOR") {
        vp.push("open doors");
    }
    if has(r, "BASH_DOOR") {
        vp.push("bash down doors");
    }
    if has(r, "PASS_WALL") {
        vp.push("pass through walls");
    }
    if has(r, "KILL_WALL") {
        vp.push("bore through walls");
    }
    if has(r, "MOVE_BODY") {
        vp.push("push past weaker monsters");
    }
    if has(r, "KILL_BODY") {
        vp.push("destroy weaker monsters");
    }
    if has(r, "TAKE_ITEM") {
        vp.push("pick up objects");
    }
    if has(r, "KILL_ITEM") {
        vp.push("destroy objects");
    }
    if has(r, "HAS_LITE") {
        vp.push("illuminate the dungeon");
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" can ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" and ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    if has(r, "INVISIBLE") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is invisible.  ");
    }
    if has(r, "COLD_BLOOD") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is cold blooded.  ");
    }
    if has(r, "EMPTY_MIND") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is not detected by telepathy.  ");
    }
    if has(r, "WEIRD_MIND") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is rarely detected by telepathy.  ");
    }
    if spell(r, "MULTIPLY") {
        s.push_str(&cap(he(msex)));
        s.push_str(" breeds explosively.  ");
    }
    if has(r, "REGENERATE") {
        s.push_str(&cap(he(msex)));
        s.push_str(" regenerates quickly.  ");
    }
    if has(r, "MORTAL") {
        s.push_str(&cap(he(msex)));
        s.push_str(" is a mortal being.  ");
    } else {
        s.push_str(&cap(he(msex)));
        s.push_str(" is an immortal being.  ");
    }

    // Susceptibilities.
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("HURT_ROCK", "rock remover"),
        ("HURT_LITE", "bright light"),
        ("SUSCEP_FIRE", "fire"),
        ("SUSCEP_COLD", "cold"),
        ("SUSCEP_ACID", "acid"),
        ("SUSCEP_ELEC", "lightning"),
        ("SUSCEP_POIS", "poison"),
    ] {
        if has(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" is hurt by ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" and ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    // Immunities.
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("IM_ACID", "acid"),
        ("IM_ELEC", "lightning"),
        ("IM_FIRE", "fire"),
        ("IM_COLD", "cold"),
        ("IM_POIS", "poison"),
    ] {
        if has(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" resists ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" and ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    // Resistances.
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("RES_NETH", "nether"),
        ("RES_WATE", "water"),
        ("RES_PLAS", "plasma"),
        ("RES_NEXU", "nexus"),
        ("RES_DISE", "disenchantment"),
        ("RES_TELE", "teleportation"),
    ] {
        if has(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" resists ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" and ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    // Non-effects.
    let mut vp: Vec<&str> = Vec::new();
    for (flag, text) in [
        ("NO_STUN", "stunned"),
        ("NO_FEAR", "frightened"),
        ("NO_CONF", "confused"),
        ("NO_SLEEP", "slept"),
    ] {
        if has(r, flag) {
            vp.push(text);
        }
    }
    if !vp.is_empty() {
        s.push_str(&cap(he(msex)));
        for (n, item) in vp.iter().enumerate() {
            if n == 0 {
                s.push_str(" cannot be ");
            } else if n < vp.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(" or ");
            }
            s.push_str(item);
        }
        s.push_str(".  ");
    }

    // How aware is it?
    {
        let act = if r.alert > 200 {
            "prefers to ignore"
        } else if r.alert > 95 {
            "pays very little attention to"
        } else if r.alert > 75 {
            "pays little attention to"
        } else if r.alert > 45 {
            "tends to overlook"
        } else if r.alert > 25 {
            "takes quite a while to see"
        } else if r.alert > 10 {
            "takes a while to see"
        } else if r.alert > 5 {
            "is fairly observant of"
        } else if r.alert > 3 {
            "is observant of"
        } else if r.alert > 1 {
            "is very observant of"
        } else if r.alert > 0 {
            "is vigilant for"
        } else {
            "is ever vigilant for"
        };
        s.push_str(&format!(
            "{} {} intruders, which {} may notice from {} feet.  ",
            cap(he(msex)),
            act,
            he(msex),
            10 * r.aaf
        ));
    }

    // Drops gold and/or items.
    {
        let mut drop_gold = 0;
        let mut drop_item = 0;
        for (flag, amount) in [
            ("DROP_4D2", 8),
            ("DROP_3D2", 6),
            ("DROP_2D2", 4),
            ("DROP_1D2", 2),
            ("DROP_90", 1),
            ("DROP_60", 1),
        ] {
            if has(r, flag) {
                drop_gold += amount;
                drop_item += amount;
            }
        }
        if has(r, "ONLY_GOLD") {
            drop_item = 0;
        }
        if has(r, "ONLY_ITEM") {
            drop_gold = 0;
        }

        let mut sin = false;
        let n = drop_gold.max(drop_item);
        if n == 0 {
            s.push_str(&cap(he(msex)));
            s.push_str(" carries no items");
        } else if n == 1 {
            s.push_str(&cap(he(msex)));
            s.push_str(" may carry a");
            sin = true;
        } else if n == 2 {
            s.push_str(&cap(he(msex)));
            s.push_str(" may carry one or two");
        } else {
            s.push_str(&cap(he(msex)));
            s.push_str(&format!(" may carry up to {}", n));
        }

        let mut p: Option<&str> = if has(r, "DROP_GREAT") {
            Some(" exceptional")
        } else if has(r, "DROP_GOOD") {
            sin = false;
            Some(" good")
        } else {
            None
        };

        if drop_item > 0 {
            if sin {
                s.push('n');
            }
            sin = false;
            if let Some(p) = p {
                s.push_str(p);
            }
            s.push_str(" object");
            if n != 1 {
                s.push('s');
            }
            p = Some(" or");
        }

        if drop_gold > 0 {
            if p.is_none() {
                sin = false;
            }
            if sin {
                s.push('n');
            }
            sin = false;
            if let Some(p) = p {
                s.push_str(p);
            }
            s.push_str(" treasure");
            if n != 1 {
                s.push('s');
            }
        }
        s.push_str(".  ");
    }

    // Attacks.
    let known: Vec<&crate::data::BlowDef> = r.blows.iter().collect();
    if !known.is_empty() {
        s.push_str(&cap(he(msex)));
        for (idx, blow) in known.iter().enumerate() {
            if idx == 0 {
                s.push_str(" can ");
            } else if idx < known.len() - 1 {
                s.push_str(", ");
            } else {
                s.push_str(", and ");
            }
            let method = blow_method(&blow.method);
            s.push_str(method);
            if let Some(q) = blow_effect(&blow.effect) {
                s.push_str(" to ");
                s.push_str(q);
                let (d1, d2) = parse_dice(&blow.dice);
                if d1 != 0 && d2 != 0 {
                    s.push_str(" with damage");
                    s.push_str(&format!(" {}d{}", d1, d2));
                }
            }
        }
        s.push_str(".  ");
    } else if has(r, "NEVER_BLOW") {
        s.push_str(&cap(he(msex)));
        s.push_str(" has no physical attacks.  ");
    } else {
        s.push_str(&format!("Nothing is known about {} attack.  ", his(msex)));
    }

    s.push('\n');
    s
}

fn blow_method(method: &str) -> &'static str {
    match method {
        "HIT" => "hit",
        "TOUCH" => "touch",
        "PUNCH" => "punch",
        "KICK" => "kick",
        "CLAW" => "claw",
        "BITE" => "bite",
        "STING" => "sting",
        "BUTT" => "butt",
        "CRUSH" => "crush",
        "ENGULF" => "engulf",
        "CHARGE" => "charge",
        "CRAWL" => "crawl on you",
        "DROOL" => "drool on you",
        "SPIT" => "spit",
        "EXPLODE" => "explode",
        "GAZE" => "gaze",
        "WAIL" => "wail",
        "SPORE" => "release spores",
        "BEG" => "beg",
        "INSULT" => "insult",
        "MOAN" => "moan",
        "SHOW" => "sing",
        _ => "do something weird",
    }
}

fn blow_effect(effect: &str) -> Option<&'static str> {
    Some(match effect {
        "HURT" => "attack",
        "POISON" => "poison",
        "UN_BONUS" => "disenchant",
        "UN_POWER" => "drain charges",
        "EAT_GOLD" => "steal gold",
        "EAT_ITEM" => "steal items",
        "EAT_FOOD" => "eat your food",
        "EAT_LITE" => "absorb light",
        "ACID" => "shoot acid",
        "ELEC" => "electrocute",
        "FIRE" => "burn",
        "COLD" => "freeze",
        "BLIND" => "blind",
        "CONFUSE" => "confuse",
        "TERRIFY" => "terrify",
        "PARALYZE" => "paralyze",
        "LOSE_STR" => "reduce strength",
        "LOSE_INT" => "reduce intelligence",
        "LOSE_WIS" => "reduce wisdom",
        "LOSE_DEX" => "reduce dexterity",
        "LOSE_CON" => "reduce constitution",
        "LOSE_CHR" => "reduce charisma",
        "LOSE_ALL" => "reduce all stats",
        "SHATTER" => "shatter",
        "EXP_10" => "lower experience (by 10d6+)",
        "EXP_20" => "lower experience (by 20d6+)",
        "EXP_40" => "lower experience (by 40d6+)",
        "EXP_80" => "lower experience (by 80d6+)",
        "DISEASE" => "disease",
        "TIME" => "time",
        "SANITY" => "blast sanity",
        "HALLU" => "cause hallucinations",
        "PARASITE" => "parasite",
        _ => return None, // "*" (RBE_ANY) leaves the effect unmentioned
    })
}

fn parse_dice(dice: &str) -> (i32, i32) {
    let mut parts = dice.split('d');
    let d1 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let d2 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (d1, d2)
}

/// Build the recall environment for a live player and race index.
pub fn env_for(ps: &PlayerState, gd: &GameData, def_idx: usize) -> RecallEnv {
    RecallEnv {
        dun_level: ps.depth as i32,
        player_level: ps.level as i32,
        kills: ps.kills.get(&(def_idx as u32)).copied().unwrap_or(0),
        unique_slain: gd
            .monsters
            .get(def_idx)
            .is_some_and(|m| m.unique && ps.kills.contains_key(&(def_idx as u32))),
    }
}

/// `monster_description_out` (monster1.cc:1347): the recall name followed
/// by the body.  `screen_roff`/`display_roff` differ only in where the
/// original draws; the port shows one text page, so all three use this.
pub fn monster_description_out(gd: &GameData, def_idx: usize, env: &RecallEnv) -> Vec<String> {
    let Some(r) = gd.monsters.get(def_idx) else {
        return vec!["Nothing is known about it.".to_string()];
    };
    let mut lines = vec![roff_name(r), String::new()];
    for line in wrap(&roff_aux(r, env), 78) {
        lines.push(line);
    }
    lines
}

/// `screen_roff` (monster1.cc:1327).
pub fn screen_roff(gd: &GameData, def_idx: usize, env: &RecallEnv) -> Vec<String> {
    monster_description_out(gd, def_idx, env)
}

/// `display_roff` (monster1.cc:1357).
pub fn display_roff(gd: &GameData, def_idx: usize, env: &RecallEnv) -> Vec<String> {
    monster_description_out(gd, def_idx, env)
}

/// Simple greedy word wrap (the original wraps at the terminal width).
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            if line.is_empty() {
                line.push_str(word);
            } else if line.len() + 1 + word.len() <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;

    fn env(kills: u32, unique_slain: bool) -> RecallEnv {
        RecallEnv {
            dun_level: 5,
            player_level: 10,
            kills,
            unique_slain,
        }
    }

    #[test]
    fn roff_name_uses_the_original_format() {
        let gd = load_game_data();
        let sparrow = gd
            .monsters
            .iter()
            .find(|m| m.name == "Sparrow")
            .expect("Sparrow");
        assert!(roff_name(sparrow).starts_with("The Sparrow ('"));
        let uniq = gd.monsters.iter().find(|m| m.unique).expect("unique");
        assert!(roff_name(uniq).starts_with(&uniq.name));
    }

    #[test]
    fn roff_aux_covers_kills_location_speed_and_attacks() {
        let gd = load_game_data();
        let cat = gd.monsters.iter().find(|m| m.name == "Scrawny cat").unwrap();
        let e = env(3, false);
        let text = roff_aux(cat, &e);
        assert!(text.starts_with("You have killed at least 3 of these creatures.  "));
        assert!(text.contains("A skinny little furball"));
        // A depth-0 race "lives in the town or the wilderness".
        assert!(text.contains("lives in the town or the wilderness"));
        // RAND_25 only, speed 110: erratic clause joins the normal speed.
        assert!(text.contains("moves a bit erratically at normal speed"));
        assert!(text.contains("may notice from 300 feet"));
        assert!(text.contains("can claw to attack with damage 1d1"));
        assert!(text.ends_with(".  \n"));
    }

    #[test]
    fn roff_aux_matches_the_original_armor_and_life_rating() {
        let gd = load_game_data();
        let cat = gd.monsters.iter().find(|m| m.name == "Scrawny cat").unwrap();
        let text = roff_aux(cat, &env(0, false));
        assert!(text.contains("has an armor rating of 1"));
        assert!(text.contains("and a life rating of 1d2"));
        assert!(text.contains("No battles to the death are recalled."));
    }

    #[test]
    fn roff_aux_reports_unique_slain_and_immortality() {
        let gd = load_game_data();
        let uniq = gd.monsters.iter().find(|m| m.unique).expect("unique");
        let text = roff_aux(uniq, &env(0, true));
        assert!(text.starts_with("You have slain this foe.  "));
        assert!(text.contains("mortal") || text.contains("immortal"));
        let text2 = roff_aux(uniq, &env(0, false));
        assert!(!text2.contains("You have slain this foe."));
    }

    #[test]
    fn roff_aux_reports_death_specials() {
        let gd = load_game_data();
        let d = gd
            .monsters
            .iter()
            .find(|m| m.name == "Death mold")
            .expect("Death mold");
        let text = roff_aux(d, &env(0, false));
        assert!(text.contains("does not deign to chase intruders"));
        assert!(text.contains("resists acid"));
        assert!(text.contains("moves incredibly quickly"), "speed 140");
    }

    #[test]
    fn monster_description_out_has_name_and_wrapped_body() {
        let gd = load_game_data();
        let cat = gd
            .monsters
            .iter()
            .position(|m| m.name == "Scrawny cat")
            .unwrap();
        let lines = monster_description_out(&gd, cat, &env(1, false));
        assert!(lines[0].starts_with("The Scrawny cat ("));
        assert!(lines.len() > 3);
        assert!(lines.iter().all(|l| l.chars().count() <= 78));
    }
}
