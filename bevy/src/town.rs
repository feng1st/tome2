//! Town shops: stock generation (from st_info) and pricing.
//! Includes the Black Market (ego goods, hefty markup) and the Home
//! (free storage; its stock is the player's deposited items).

use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::data::{GameData, OwnerDef, StoreDef};
use crate::item::{self, Item};

/// A random rumour line (files.cc get_rnd_line over rumors.txt: the first
/// line is the valid-line count, the next is the buffer marker).
pub fn rumor(rng: &mut impl Rng) -> String {
    const RUMORS_TXT: &str = include_str!("../assets/data/rumors.txt");
    let lines: Vec<&str> = RUMORS_TXT.lines().collect();
    let count: usize = lines
        .first()
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or(0);
    if count == 0 {
        return String::new();
    }
    // randint(count) picks the k-th line after the count line; the buffer
    // line at index 1 is part of the numbering but never returned.
    let k = rng.gen_range(1..=count);
    lines.get(k + 1).copied().unwrap_or("").to_string()
}

/// say_comment_1 (store.cc:98): the shopkeeper's grunt on a successful
/// haggle; one time in RUMOR_CHANCE (8) a rumour follows.
pub fn say_comment_1(rng: &mut impl Rng) -> Vec<String> {
    const COMMENT_1: [&str; 6] = ["Okay.", "Fine.", "Accepted!", "Agreed!", "Done!", "Taken!"];
    let mut out = vec![COMMENT_1[rng.gen_range(0..COMMENT_1.len())].to_string()];
    if rng.gen_range(1..=8) == 1 {
        out.push("The shopkeeper whispers something into your ear:".to_string());
        out.push(rumor(rng));
    }
    out
}

/// say_comment_4 (store.cc:116): the two lines for kicking a failed thief
/// out of the store.
pub fn say_comment_4(rng: &mut impl Rng) -> (String, String) {
    const COMMENT_4A: [&str; 4] = [
        "Enough!  You have abused me once too often!",
        "Arghhh!  I have had enough abuse for one day!",
        "That does it!  You shall waste my time no more!",
        "This is getting nowhere!  I'm going to Londis!",
    ];
    const COMMENT_4B: [&str; 4] = [
        "Leave my store!",
        "Get out of my sight!",
        "Begone, you scoundrel!",
        "Out, out, out!",
    ];
    (
        COMMENT_4A[rng.gen_range(0..COMMENT_4A.len())].to_string(),
        COMMENT_4B[rng.gen_range(0..COMMENT_4B.len())].to_string(),
    )
}

/// st_info id of the Home (its "stock" is what the player deposited).
pub const HOME_STORE: u32 = 7;

#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct ShopStocks {
    /// (town, store) key -> items currently for sale (or stored, for Home).
    pub stocks: HashMap<u32, Vec<Item>>,
    /// (town, store) key -> owner id (ow_info; assigned on first visit).
    #[serde(default)]
    pub owners: HashMap<u32, u32>,
    /// Stores whose owner caught the player stealing (store_open): the
    /// turn until which the shop refuses service (turn + 500000 +
    /// randint(500000)).
    #[serde(default)]
    pub open_until: HashMap<u32, u64>,
    /// Last day each store was maintained (daily turnover).
    #[serde(default)]
    pub last_day: HashMap<u32, u64>,
}

/// Per-town store key (towns share st_info indices).
pub fn skey(town: u32, store: u32) -> u32 {
    town.saturating_mul(1000).saturating_add(store)
}

/// store_open (store.cc): is the shop still refusing to serve the player?
pub fn store_banned(stocks: &ShopStocks, key: u32, turn: u64) -> bool {
    stocks
        .open_until
        .get(&key)
        .map(|t| *t >= turn)
        .unwrap_or(false)
}

/// mass_roll (store.cc:334): the sum of `num` rolls of rand_int(max), i.e.
/// each roll is in 0..=max-1 (z-rand.cc rand_int).
pub fn mass_roll(num: i32, max: i32, rng: &mut impl Rng) -> i32 {
    (0..num)
        .map(|_| {
            if max < 1 {
                0
            } else {
                rng.gen_range(0..max)
            }
        })
        .sum()
}

/// mass_produce (store.cc:346): cheap goods appear in discounted piles.
pub fn mass_produce(gd: &GameData, item: &mut Item, rng: &mut impl Rng) {
    let cost = item.cost(gd);
    let mut size = 1;
    match gd.objects[item.def].tval {
        crate::data::TV_FOOD | crate::data::TV_FLASK | crate::data::TV_LITE => {
            if cost <= 5 {
                size += mass_roll(3, 5, rng);
            }
            if cost <= 20 {
                size += mass_roll(3, 5, rng);
            }
        }
        crate::data::TV_POTION | crate::data::TV_POTION2 | crate::data::TV_SCROLL => {
            if cost <= 60 {
                size += mass_roll(3, 5, rng);
            }
            if cost <= 240 {
                size += mass_roll(1, 5, rng);
            }
        }
        crate::data::TV_SYMBIOTIC_BOOK
        | crate::data::TV_MUSIC_BOOK
        | crate::data::TV_DRUID_BOOK
        | crate::data::TV_DAEMON_BOOK
        | crate::data::TV_BOOK => {
            if cost <= 50 {
                size += mass_roll(2, 3, rng);
            }
            if cost <= 500 {
                size += mass_roll(1, 3, rng);
            }
        }
        crate::data::TV_SOFT_ARMOR
        | crate::data::TV_HARD_ARMOR
        | crate::data::TV_SHIELD
        | crate::data::TV_GLOVES
        | crate::data::TV_BOOTS
        | crate::data::TV_CLOAK
        | crate::data::TV_HELM
        | crate::data::TV_CROWN
        | crate::data::TV_SWORD
        | crate::data::TV_AXE
        | crate::data::TV_POLEARM
        | crate::data::TV_HAFTED
        | crate::data::TV_DIGGING
        | crate::data::TV_BOW => {
            if item.ego == 0 {
                if cost <= 10 {
                    size += mass_roll(3, 5, rng);
                }
                if cost <= 100 {
                    size += mass_roll(3, 5, rng);
                }
            }
        }
        crate::data::TV_SPIKE
        | crate::data::TV_SHOT
        | crate::data::TV_ARROW
        | crate::data::TV_BOLT => {
            if cost <= 5 {
                size += mass_roll(5, 5, rng);
            }
            if cost <= 50 {
                size += mass_roll(5, 5, rng);
            }
            if cost <= 500 {
                size += mass_roll(5, 5, rng);
            }
        }
        crate::data::TV_ROD | crate::data::TV_WAND | crate::data::TV_STAFF => {
            if cost < 1601 {
                size += mass_roll(1, 5, rng);
            } else if cost < 3201 {
                size += mass_roll(1, 3, rng);
            }
        }
        _ => {}
    }
    let mut discount = 0;
    if cost < 5 {
        discount = 0;
    } else if rng.gen_range(0..25) == 0 {
        discount = 25;
    } else if rng.gen_range(0..150) == 0 {
        discount = 50;
    } else if rng.gen_range(0..300) == 0 {
        discount = 75;
    } else if rng.gen_range(0..500) == 0 {
        discount = 90;
    }
    // No discount on random artifacts (store.cc mass_produce).
    if !item.artifact_name.is_empty() {
        discount = 0;
    }
    item.discount = discount;
    item.count = (size - (size * discount / 100)) as u32;
}

/// purchase_analyze (store.cc:173): the shopkeeper's reaction to the
/// agreed price versus what the item was worth and what we guessed.
pub fn purchase_analyze(
    price: i32,
    value: i32,
    guess: i32,
    rng: &mut impl Rng,
) -> Option<&'static str> {
    const COMMENT_7A: [&str; 4] = [
        "Arrgghh!",
        "You moron!",
        "You hear someone sobbing...",
        "The shopkeeper howls in agony!",
    ];
    const COMMENT_7B: [&str; 4] = [
        "Darn!",
        "You fiend!",
        "The shopkeeper yells at you.",
        "The shopkeeper glares at you.",
    ];
    const COMMENT_7C: [&str; 4] = [
        "Cool!",
        "You've made my day!",
        "The shopkeeper giggles.",
        "The shopkeeper laughs loudly.",
    ];
    const COMMENT_7D: [&str; 4] = [
        "Yippee!",
        "I think I'll retire!",
        "The shopkeeper jumps for joy.",
        "The shopkeeper smiles gleefully.",
    ];
    let mut pick = |c: &[&'static str]| Some(c[rng.gen_range(0..c.len())]);
    if value <= 0 && price > value {
        pick(&COMMENT_7A)
    } else if value < guess && price > value {
        pick(&COMMENT_7B)
    } else if value > guess && value < 4 * guess && price < value {
        pick(&COMMENT_7C)
    } else if value > guess && price < value {
        pick(&COMMENT_7D)
    } else {
        None
    }
}

/// store_object_similar (store.cc:484): can `b` stack into `a` in a
/// store's stock? Wands merge charges; rods stay separate; random
/// artifacts never stack.
pub fn store_object_similar(gd: &GameData, a: &Item, b: &Item) -> bool {
    if a.def != b.def {
        return false;
    }
    let tval = gd.objects[a.def].tval;
    // Different charges cannot be stacked, unless wands.
    if a.charges != b.charges && tval != crate::data::TV_WAND {
        return false;
    }
    if a.stick_spell != b.stick_spell
        || a.pval2 != b.pval2
        || a.pval3 != b.pval3
        || a.to_h != b.to_h
        || a.to_d != b.to_d
        || a.to_a != b.to_a
        || a.artifact != b.artifact
        || a.ego != b.ego
        || a.ego2 != b.ego2
        || !a.artifact_name.is_empty()
        || !b.artifact_name.is_empty()
        || a.flags != b.flags
        || a.ac != b.ac
        || a.dice != b.dice
        || a.discount != b.discount
    {
        return false;
    }
    if tval == crate::data::TV_LITE {
        // Lites stack only with identical fuel.
        if a.fuel != b.fuel {
            return false;
        }
    } else if a.timeout != 0 || b.timeout != 0 {
        // Never stack recharging items.
        return false;
    }
    true
}

/// store_object_absorb (store.cc:551): combine quantity (capped at 99)
/// and wand charges.
pub fn store_object_absorb(gd: &GameData, a: &mut Item, b: &Item) {
    let total = a.count + b.count;
    a.count = total.min(99);
    if gd.objects[a.def].tval == crate::data::TV_WAND {
        a.charges += b.charges;
    }
}

/// store_check_num (store.cc:571): is there room (or a stack to merge
/// into) for `item` in this store?
pub fn store_check_num(gd: &GameData, store: &StoreDef, stock: &[Item], item: &Item) -> bool {
    if stock.len() < store.max_items as usize {
        return true;
    }
    let home_like = store.is_home() || store.flags.iter().any(|f| f == "MUSEUM");
    stock.iter().any(|j| {
        if home_like {
            crate::item::items_similar(gd, j, item)
        } else {
            store_object_similar(gd, j, item)
        }
    })
}

/// is_blessed (store.cc:614): the item's *known* flags include BLESSED
/// (object_flags_known hides everything until the item is identified).
pub fn is_blessed(gd: &GameData, item: &Item) -> bool {
    crate::item::item_flags_known(gd, item)
        .iter()
        .any(|f| *f == "BLESSED")
}

/// home_carry (store.cc:847): merge into a similar stack, otherwise insert
/// sorted by decreasing tval, increasing sval and decreasing value.
pub fn home_carry(
    gd: &GameData,
    stock: &mut Vec<Item>,
    item: Item,
    capacity: usize,
) -> Option<usize> {
    if let Some(i) = stock
        .iter()
        .position(|j| crate::item::items_similar(gd, j, &item))
    {
        crate::item::absorb_item_tval(gd, &mut stock[i], item);
        return Some(i);
    }
    if stock.len() >= capacity {
        return None;
    }
    let tval = gd.objects[item.def].tval;
    let sval = gd.objects[item.def].sval;
    let value = item.cost(gd);
    let mut slot = stock.len();
    for (i, j) in stock.iter().enumerate() {
        let jt = gd.objects[j.def].tval;
        if tval > jt {
            slot = i;
            break;
        }
        if tval < jt {
            continue;
        }
        // object_aware_p is kind-wide in the port and not visible here;
        // object_known_p maps to Item::identified.
        if !item.identified {
            continue;
        }
        if !j.identified {
            slot = i;
            break;
        }
        let js = gd.objects[j.def].sval;
        if sval < js {
            slot = i;
            break;
        }
        if sval > js {
            continue;
        }
        if tval == crate::data::TV_ROD_MAIN {
            if item.timeout < j.timeout {
                slot = i;
                break;
            }
            if item.timeout > j.timeout {
                continue;
            }
        }
        if value > j.cost(gd) {
            slot = i;
            break;
        }
    }
    stock.insert(slot, item);
    Some(slot)
}

/// black_market_crap (store.cc:1064): an item is "crap" when an identical
/// base kind is already sold elsewhere in town. Ego/good items are never
/// crap.
pub fn black_market_crap(gd: &GameData, stocks: &ShopStocks, town: u32, item: &Item) -> bool {
    if item.ego != 0 {
        return false;
    }
    if item.to_a > 0 || item.to_h > 0 || item.to_d > 0 {
        return false;
    }
    for store in &gd.stores {
        if store.is_home() || store.flags.iter().any(|f| f == "MUSEUM") {
            continue;
        }
        let key = skey(town, store.id);
        let Some(stock) = stocks.stocks.get(&key) else {
            continue;
        };
        if stock.iter().any(|o| o.def == item.def) {
            return true;
        }
    }
    false
}

/// store_item_increase (store.cc:1021): signed stack-size delta clamped
/// to 0..=255 (this can zero an item).
pub fn store_item_increase(stock: &mut [Item], item: usize, num: i32) {
    if let Some(o) = stock.get_mut(item) {
        o.count = (o.count as i32 + num).clamp(0, 255) as u32;
    }
}

/// store_item_optimize (store.cc:1040): drop a stock slot once it holds
/// no items.
pub fn store_item_optimize(stock: &mut Vec<Item>, item: usize) {
    if item < stock.len() && stock[item].count == 0 {
        stock.remove(item);
    }
}

/// store_delete (store.cc:1102): destroy (part of) a random stock slot,
/// keeping piles where possible and scaling wand charges.
pub fn store_delete(gd: &GameData, stock: &mut Vec<Item>, rng: &mut impl Rng) {
    if stock.is_empty() {
        return;
    }
    let what = rng.gen_range(0..stock.len());
    let mut num = stock[what].count as i32;
    if rng.gen_range(0..100) < 50 {
        num = (num + 1) / 2;
    }
    if rng.gen_range(0..100) < 50 {
        num = 1;
    }
    if gd.objects[stock[what].def].tval == crate::data::TV_WAND {
        // Wands: scale the covered charges down with the destroyed count.
        let pval = stock[what].charges;
        stock[what].charges -= num * pval / stock[what].count.max(1) as i32;
    }
    store_item_increase(stock, what, -num);
    store_item_optimize(stock, what);
}

/// adjust_store_top_item_removed (store.cc:1906): keep the page start in
/// range after a slot is removed.
#[allow(dead_code)]
pub fn adjust_store_top_item_removed(stock_len: usize, store_top: i32) -> i32 {
    if stock_len == 0 || store_top == 0 {
        0
    } else if store_top >= stock_len as i32 {
        store_top - 12
    } else {
        store_top
    }
}

/// kind_is_storeok (store.cc:1156): a kind sold by a T:<tval> entry must
/// not be a fixed artifact, must be legal, match the tval and (when the
/// store forces a level) not be too shallow.
pub fn kind_is_storeok(
    gd: &GameData,
    def: usize,
    store_tval: i32,
    store_level: u32,
    rng: &mut impl Rng,
) -> bool {
    let o = &gd.objects[def];
    if o.flags.iter().any(|f| f == "NORM_ART" || f == "INSTA_ART") {
        return false;
    }
    if !crate::item::kind_is_legal_plain(gd, def, rng) {
        return false;
    }
    if o.tval != store_tval {
        return false;
    }
    if o.depth < store_level / 2 {
        return false;
    }
    true
}

/// Daily stock turnover (store.cc store_maint): prune to a random keep
/// count, then refill to a random target; the black market also purges
/// "crap" (plain items sold elsewhere).
pub fn maintain(
    gd: &GameData,
    stocks: &mut ShopStocks,
    town: u32,
    store_id: u32,
    day: u64,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) {
    let key = skey(town, store_id);
    if stocks.last_day.get(&key) == Some(&day) {
        return;
    }
    stocks.last_day.insert(key, day);
    maintain_impl(gd, stocks, town, store_id, created, depth, player_level, rng);
}

/// store_maint without the once-a-day guard (store_stole calls it ten
/// times to refill a looted store).
pub fn maintain_impl(
    gd: &GameData,
    stocks: &mut ShopStocks,
    town: u32,
    store_id: u32,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) {
    const STORE_TURNOVER: i32 = 9;
    const STORE_MIN_KEEP: i32 = 6;
    const STORE_MAX_KEEP: i32 = 18;
    let key = skey(town, store_id);
    if key == skey(town, HOME_STORE) {
        return;
    }
    let Some(store) = gd.stores.iter().find(|s| s.id == store_id).cloned() else {
        return;
    };
    // Museum-like buildings keep only what is donated.
    if store.flags.iter().any(|f| f == "MUSEUM") {
        return;
    }
    // Black market prune (store.cc:3492): plain goods sold elsewhere go.
    if store.is_black_market() {
        let mut doomed = Vec::new();
        if let Some(stock) = stocks.stocks.get(&key) {
            for (i, it) in stock.iter().enumerate() {
                if black_market_crap(gd, stocks, town, it) {
                    doomed.push(i);
                }
            }
        }
        if let Some(stock) = stocks.stocks.get_mut(&key) {
            for i in doomed.into_iter().rev() {
                if i < stock.len() {
                    stock.remove(i);
                }
            }
        }
    }
    let mut fresh = gen_stock(gd, &*stocks, town, store_id, created, depth, player_level, rng);
    let Some(stock) = stocks.stocks.get_mut(&key) else {
        return;
    };
    // Sell a few items.
    let mut keep = stock.len() as i32 - rng.gen_range(1..=STORE_TURNOVER);
    keep = keep.min(STORE_MAX_KEEP).max(STORE_MIN_KEEP).max(0);
    while (stock.len() as i32) > keep {
        store_delete(gd, stock, rng);
    }
    // Buy some more.
    let mut target = stock.len() as i32 + rng.gen_range(1..=STORE_TURNOVER);
    target = target.min(STORE_MAX_KEEP).max(STORE_MIN_KEEP);
    if target >= store.max_items as i32 {
        target = store.max_items as i32 - 1;
    }
    let mut tries = 0;
    while (stock.len() as i32) < target && tries < 100 {
        tries += 1;
        if let Some(it) = fresh.pop() {
            if let Some(slot) = stock.iter().position(|j| store_object_similar(gd, j, &it)) {
                store_object_absorb(gd, &mut stock[slot], &it);
            } else {
                stock.push(it);
            }
        } else {
            break;
        }
    }
}

/// The stock of a store, generating it on first visit. Also assigns the
/// store's owner from the store's candidate list (st_info O: lines).
pub fn stock_for(
    gd: &GameData,
    stocks: &mut ShopStocks,
    town: u32,
    store_id: u32,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) -> Vec<Item> {
    let key = skey(town, store_id);
    if let Some(s) = stocks.stocks.get(&key) {
        return s.clone();
    }
    if let Some(store) = gd.stores.iter().find(|s| s.id == store_id) {
        if !store.owners.is_empty() && !stocks.owners.contains_key(&key) {
            let oid = store.owners[rng.gen_range(0..store.owners.len())];
            stocks.owners.insert(key, oid);
        }
    }
    let stock = gen_stock(gd, &*stocks, town, store_id, created, depth, player_level, rng);
    stocks.stocks.insert(key, stock.clone());
    stock
}

/// retire_owner_p (store.cc:1884): the shopkeeper retires when the store
/// has at most one candidate owner and the 1/STORE_SHUFFLE roll hits.
/// Note the original's condition: `owners.size() > 1` returns false, so
/// with ToME's four-owner shops retirement never happens in practice.
pub fn retire_owner_p(store: &StoreDef, rng: &mut impl Rng) -> bool {
    const STORE_SHUFFLE: i32 = 21;
    if store.owners.len() > 1 {
        return false;
    }
    rng.gen_range(0..STORE_SHUFFLE) == 0
}

/// store_shuffle (store.cc:3422): pick a different owner, clear the ban
/// and discount the surviving stock 50% with an "on sale" inscription.
/// The original loops forever when only one owner exists; such stores
/// cannot retire here, so the owner simply stays.
pub fn store_shuffle(
    gd: &GameData,
    stocks: &mut ShopStocks,
    town: u32,
    store_id: u32,
    rng: &mut impl Rng,
) {
    let key = skey(town, store_id);
    let Some(store) = gd.stores.iter().find(|s| s.id == store_id).cloned() else {
        return;
    };
    if store.owners.len() > 1 {
        let cur = stocks.owners.get(&key).copied();
        let mut new = cur;
        while new == cur {
            new = Some(store.owners[rng.gen_range(0..store.owners.len())]);
        }
        stocks.owners.insert(key, new.expect("owner"));
    }
    stocks.open_until.remove(&key);
    if let Some(stock) = stocks.stocks.get_mut(&key) {
        for it in stock.iter_mut() {
            // store_shuffle: half price for non-randarts, all "on sale".
            if it.artifact_name.is_empty() {
                it.discount = 50;
            }
            it.inscription = "on sale".to_string();
        }
    }
}

/// The emptied-stock event shared by store_purchase and store_stole
/// (store.cc:2065-2097/2331-2360): the owner may retire, otherwise the
/// shopkeeper brings out new stock, and ten maintain passes run either
/// way. `created`/`rng` are threaded through the refill.
#[allow(clippy::too_many_arguments)]
pub fn emptied_stock(
    gd: &GameData,
    stocks: &mut ShopStocks,
    town: u32,
    store_id: u32,
    log: &mut crate::game::MessageLog,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) {
    if store_id == HOME_STORE {
        return;
    }
    if !stocks
        .stocks
        .get(&skey(town, store_id))
        .map(|s| s.is_empty())
        .unwrap_or(false)
    {
        return;
    }
    let Some(store) = gd.stores.iter().find(|s| s.id == store_id).cloned() else {
        return;
    };
    if store.flags.iter().any(|f| f == "MUSEUM") {
        return;
    }
    if retire_owner_p(&store, rng) {
        log.add("The shopkeeper retires.");
        store_shuffle(gd, stocks, town, store_id, rng);
    } else {
        log.add("The shopkeeper brings out some new stock.");
    }
    for _ in 0..10 {
        maintain_impl(gd, stocks, town, store_id, created, depth, player_level, rng);
    }
}

/// The owner of a store, if one has been assigned.
pub fn store_owner<'a>(
    gd: &'a GameData,
    stocks: &ShopStocks,
    town: u32,
    store_id: u32,
) -> Option<&'a OwnerDef> {
    let oid = stocks.owners.get(&skey(town, store_id))?;
    gd.owners.iter().find(|o| o.id == *oid)
}

/// return_level (store.cc:1128): the level an item is created at, from
/// the store's STF_* flags.
fn return_level(store: &StoreDef, depth: u32, player_level: u32, rng: &mut impl Rng) -> u32 {
    let has = |f: &str| store.flags.iter().any(|x| x == f);
    let mut level = if has("RANDOM") {
        0
    } else {
        rng.gen_range(1..=5)
    };
    if has("DEPEND_LEVEL") {
        level += depth;
    }
    if has("SHALLOW_LEVEL") {
        level += 5 + rng.gen_range(0..5);
    }
    if has("MEDIUM_LEVEL") {
        level += 25 + rng.gen_range(0..25);
    }
    if has("DEEP_LEVEL") {
        level += 45 + rng.gen_range(0..45);
    }
    if has("ALL_ITEM") {
        level += player_level;
    }
    level
}

/// Resolve a T: entry. sval 255 books become a random school tome; the
/// 255/256 wildcard entries pick a random kind that passes
/// kind_is_storeok (store.cc choose_k_idx).
fn resolve_entry(
    gd: &GameData,
    tval: i32,
    sval: i32,
    level: u32,
    force_level: bool,
    rng: &mut impl Rng,
) -> Option<usize> {
    if tval == crate::data::TV_BOOK && sval == 255 {
        // A "Spellbook of #" (BOOK_RANDOM): one random Magic (75%) or
        // Spirituality spell, filled in by make_item.
        return gd.object_by_tval_sval(crate::data::TV_BOOK, 255);
    }
    // T:<tval>:255/256 = any sval of that tval (the smiths and jewellers).
    if sval >= 255 {
        let store_level = if force_level { level } else { 0 };
        let cands: Vec<usize> = (0..gd.objects.len())
            .filter(|&i| {
                gd.objects[i].tval == tval
                    && gd.objects[i].depth > 0
                    && kind_is_storeok(gd, i, tval, store_level, rng)
            })
            .collect();
        if cands.is_empty() {
            return None;
        }
        return Some(cands[rng.gen_range(0..cands.len())]);
    }
    gd.object_by_tval_sval(tval, sval)
}

fn gen_stock(
    gd: &GameData,
    stocks: &ShopStocks,
    town: u32,
    store_id: u32,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) -> Vec<Item> {
    let Some(store) = gd.stores.iter().find(|s| s.id == store_id).cloned() else {
        return vec![];
    };
    if store.is_home() || store.flags.iter().any(|f| f == "MUSEUM") {
        return vec![];
    }
    if store.is_black_market() {
        return gen_black_market(gd, stocks, town, &store, created, depth, player_level, rng);
    }
    // return_level uses the current dungeon depth and the player's level
    // (store.cc:1128).
    let level = return_level(&store, depth, player_level, rng);
    let force_level = store.flags.iter().any(|f| f == "FORCE_LEVEL");
    let mut out: Vec<Item> = Vec::new();
    for e in &store.entries {
        if rng.gen_range(0..100) >= e.proba as i32 {
            continue;
        }
        // store_create tries up to four times before giving up on a slot
        // (store.cc:1284-1451): a pruned "worthless" roll is retried.
        for _ in 0..4 {
            // A named I: entry is a fixed kind; T: entries pick by tval.
            let def = if !e.name.is_empty() {
                gd.object_by_name(&e.name)
            } else {
                resolve_entry(gd, e.tval, e.sval, level, force_level, rng)
            };
            let Some(d) = def else { continue };
            // store_carry would merge duplicates; the port keeps one slot/Kind.
            if out.iter().any(|it| it.def == d) {
                break;
            }
            let o = &gd.objects[d];
            // BOOK_RANDOM books get their single random spell.  Every
            // other kind runs the store pipeline: object_prep then
            // apply_magic(level, false, false, false) (store.cc:1399-1402).
            let mut it = if o.tval == crate::data::TV_BOOK && o.sval == 255 {
                item::make_item(gd, d, level, false, created, rng)
            } else {
                item::apply_magic(gd, d, level, false, false, false, None, created, rng)
            };
            it.identified = true;
            // Lite fuel: FUEL_LITE kinds are charged with pval2 fuel
            // (store.cc:1404-1412; the port's ObjectDef.fuel is pval2).
            if o.tval == crate::data::TV_LITE && o.flags.iter().any(|f| f == "FUEL_LITE") {
                it.fuel = o.fuel.max(0);
            }
            // No "worthless" items (store.cc:1430-1434); try again.
            // C++ calls object_value here, which also zeroes cursed items;
            // object_value_real is used as specified, so cursed stock stays
            // (mass_produce still gives it no discount).
            if item::object_value_real(gd, &it) <= 0 {
                continue;
            }
            mass_produce(gd, &mut it, rng);
            // Wand charges are per wand, so multiply by the pile size.
            if gd.objects[d].tval == crate::data::TV_WAND {
                it.charges *= it.count.max(1) as i32;
            }
            out.push(it);
            break;
        }
        if out.len() >= store.max_items as usize {
            break;
        }
    }
    // Merge duplicates (store_carry) and enforce the stock cap.
    let mut stock: Vec<Item> = Vec::new();
    for it in out {
        if let Some(slot) = stock.iter().position(|j| store_object_similar(gd, j, &it)) {
            store_object_absorb(gd, &mut stock[slot], &it);
        } else if stock.len() < store.max_items as usize {
            stock.push(it);
        }
    }
    stock
}

/// Black Market: the STF_ALL_ITEM path of store_create (store.cc:1321-
/// 1447).  roll the kind at return_level(), apply low-level magic, then
/// prune "crap" (plain goods already sold elsewhere) and anything worth
/// under 10; cheap goods mass-produce as usual.
#[allow(clippy::too_many_arguments)]
fn gen_black_market(
    gd: &GameData,
    stocks: &ShopStocks,
    town: u32,
    store: &StoreDef,
    created: &mut HashSet<u32>,
    depth: u32,
    player_level: u32,
    rng: &mut impl Rng,
) -> Vec<Item> {
    let level = return_level(store, depth, player_level, rng);
    let mut out: Vec<Item> = Vec::new();
    // store_create considers up to four items per call; the port fills the
    // whole shelf, retrying pruned rolls (store.cc:1284-1451).
    while out.len() < store.max_items as usize {
        let mut placed = false;
        for _ in 0..4 {
            // get_obj_num with kind_is_legal (no theme; illegal kinds are
            // skipped by the allocator, store.cc:1331-1340).
            let Some(def) = item::gen_object_kind(gd, level, rng) else {
                continue;
            };
            let o = &gd.objects[def];
            if o.flags.iter().any(|f| f == "NORM_ART" || f == "INSTA_ART") {
                continue;
            }
            let mut it = item::apply_magic(gd, def, level, false, false, false, None, created, rng);
            // object_known (store.cc:1417).
            it.identified = true;
            // Lite fuel (store.cc:1404-1412).
            if o.tval == crate::data::TV_LITE && o.flags.iter().any(|f| f == "FUEL_LITE") {
                it.fuel = o.fuel.max(0);
            }
            if black_market_crap(gd, stocks, town, &it) {
                continue;
            }
            if it.cost(gd) < 10 {
                continue;
            }
            mass_produce(gd, &mut it, rng);
            if gd.objects[def].tval == crate::data::TV_WAND {
                it.charges *= it.count.max(1) as i32;
            }
            out.push(it);
            placed = true;
            break;
        }
        if !placed {
            // No acceptable roll in four tries; stop like store_create
            // returning without adding a slot.
            break;
        }
    }
    out
}

/// adj_chr_gold (tables.cc): buyer/seller charisma adjustment by CHR
/// value, clamped to the table's 3..=40 range.
fn adj_chr_gold(chr: i32) -> i32 {
    const TABLE: [i32; 38] = [
        130, 125, 122, 120, 118, 116, 114, 112, 110, 108, 106, 104, 103, 102, 101, 100, 99, 98, 97,
        96, 95, 94, 93, 92, 91, 90, 88, 86, 84, 82, 80, 78, 76, 74, 72, 70, 68, 66,
    ];
    TABLE[(chr.clamp(3, 40) - 3) as usize]
}

/// Pricing context: the store's owner and the player's race/class/CHR.
pub struct ShopCtx<'a> {
    pub owner: Option<&'a OwnerDef>,
    pub race: &'a str,
    pub class: &'a str,
    pub chr: i32,
    /// options->no_selling: shops pay nothing (store.cc:304).
    pub no_selling: bool,
}

impl<'a> ShopCtx<'a> {
    /// No owner, average charisma (tests, ownerless shops).
    pub fn neutral() -> ShopCtx<'a> {
        ShopCtx {
            owner: None,
            race: "",
            class: "",
            chr: 18,
            no_selling: false,
        }
    }
}

/// `is_state_aux` (bldg.cc:58): the owner's ow_info entry for the given
/// state (STORE_HATED = 0, STORE_LIKED = 1) lists the player's race or
/// class.  The original tests `races[state]`/`classes[state]` bitmasks;
/// the port's OwnerDef stores the names instead.
pub fn is_state_aux(owner: &OwnerDef, race: &str, class: &str, state: i32) -> bool {
    let list = match state {
        0 => &owner.hated,
        1 => &owner.liked,
        _ => return false,
    };
    list.iter().any(|n| n == race || n == class)
}

/// `is_state` (bldg.cc:84): STORE_NORMAL (2) means the player is neither
/// liked nor hated; any other state checks its own list.
pub fn is_state(owner: &OwnerDef, race: &str, class: &str, state: i32) -> bool {
    if state == 2 {
        !is_state_aux(owner, race, class, 1) && !is_state_aux(owner, race, class, 0)
    } else {
        is_state_aux(owner, race, class, state)
    }
}

/// The owner's greed and mood factor for this customer (store.cc).
fn owner_mood(ctx: &ShopCtx) -> (i32, i32) {
    let chr = adj_chr_gold(ctx.chr);
    match ctx.owner {
        None => (100, 100 + chr),
        Some(o) => {
            let liked = is_state(o, ctx.race, ctx.class, 1);
            let hated = is_state(o, ctx.race, ctx.class, 0);
            let cost = if liked {
                o.costs.2
            } else if hated {
                o.costs.0
            } else {
                o.costs.1
            };
            (o.inflation, cost + chr)
        }
    }
}

/// price_item (store.cc:249): the per-item price. `flip` = the store is
/// buying. Never lets the shop lose money on a purchase; the Black Market
/// halves what it pays and doubles what it charges (STF_ALL_ITEM).
pub fn price_item(
    gd: &GameData,
    store: Option<&StoreDef>,
    item: &Item,
    ctx: &ShopCtx,
    flip: bool,
) -> i32 {
    let mut price = item.cost(gd);
    if price <= 0 {
        return 0;
    }
    let (greed, factor) = owner_mood(ctx);
    let tval = gd.objects[item.def].tval;
    let all_item = store
        .map(|s| s.flags.iter().any(|f| f == "ALL_ITEM"))
        .unwrap_or(false);
    let adjust;
    if flip {
        // Mega Hack^3: only shot, arrows and bolts are quartered
        // (store.cc:285-292); boomerangs keep their value.
        if matches!(
            tval,
            crate::data::TV_SHOT | crate::data::TV_ARROW | crate::data::TV_BOLT
        ) {
            price /= 5;
        }
        adjust = (100 + (300 - (greed + factor))).min(100);
        if all_item {
            price /= 2;
        }
        if ctx.no_selling {
            price = 0;
        }
    } else {
        adjust = (100 + ((greed + factor) - 300)).max(100);
        if all_item {
            price *= 2;
        }
        if price <= 0 {
            price = 1;
        }
    }
    (price * adjust + 50) / 100
}

/// Price for the player buying from a store (store.cc price_item,
/// flip=false).
pub fn buy_price(gd: &GameData, store: Option<&StoreDef>, item: &Item, ctx: &ShopCtx) -> i32 {
    price_item(gd, store, item, ctx, false)
}

/// Price the player gets when selling (store.cc price_item, flip=true);
/// sell_haggle caps the pile at the owner's purse.
pub fn sell_price(gd: &GameData, store: Option<&StoreDef>, item: &Item, ctx: &ShopCtx) -> i32 {
    let mut price = price_item(gd, store, item, ctx, true);
    if let Some(o) = ctx.owner {
        if price > o.max_cost {
            price = o.max_cost;
        }
    }
    price.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::load_game_data;

    #[test]
    fn stock_generates_resolvable_items() {
        let gd = load_game_data();
        let mut stocks = ShopStocks::default();
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        for store in &gd.stores {
            // Museum-style buildings (Mathom-house) have no sellable stock;
            // chance-based I: entries (Soothsayer) may roll empty.
            if store.entries.is_empty() {
                continue;
            }
            let stock = stock_for(&gd, &mut stocks, 1, store.id, &mut created, 0, 1, &mut rng);
            if store.is_home() {
                assert!(stock.is_empty(), "the Home starts empty");
                continue;
            }
            if store.entries.iter().all(|e| e.proba < 100) {
                continue; // may legitimately roll empty
            }
            // store_create prunes "worthless" rolls (store.cc:1433); a
            // single-entry shop can roll a cursed item all four tries
            // (e.g. Speed Ring Market, Rare Footwear Shop).
            if stock.is_empty() {
                assert_eq!(store.entries.len(), 1, "{} has an empty stock", store.name);
                continue;
            }
            // Deterministic: second call returns the same stock.
            let again = stock_for(&gd, &mut stocks, 1, store.id, &mut created, 0, 1, &mut rng);
            assert_eq!(stock, again);
        }
        // The Book Store sells spellbooks.
        let book_store = gd.stores.iter().find(|s| s.name == "Book Store").unwrap();
        let stock = stock_for(&gd, &mut stocks, 1, book_store.id, &mut created, 0, 1, &mut rng);
        assert!(stock
            .iter()
            .any(|it| gd.objects[it.def].tval == crate::data::TV_BOOK));
    }

    #[test]
    fn black_market_sells_ego_goods() {
        let gd = load_game_data();
        let bm = gd.stores.iter().find(|s| s.is_black_market()).unwrap();
        let mut stocks = ShopStocks::default();
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        let stock = stock_for(&gd, &mut stocks, 1, bm.id, &mut created, 0, 1, &mut rng);
        assert!(!stock.is_empty(), "black market is empty");
        // The STF_ALL_ITEM path rolls any legal kind and applies magic
        // (store.cc:1321-1447): plain goods are allowed as long as they
        // are not "crap" (sold elsewhere) and worth at least 10.
        assert!(stock.iter().all(|it| it.cost(&gd) >= 10));
        assert!(stock.iter().all(|it| !black_market_crap(&gd, &stocks, 1, it) || it.cost(&gd) >= 10));
        // The markup is real.
        let plain = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        let neutral = ShopCtx::neutral();
        assert!(
            buy_price(&gd, Some(bm), &plain, &neutral) > buy_price(&gd, None, &plain, &neutral)
        );
    }

    #[test]
    fn store_state_lists_match_race_and_class() {
        // bldg.cc is_state_aux/is_state over a synthetic ow_info entry.
        let owner = OwnerDef {
            id: 0,
            name: "Tester".to_string(),
            max_cost: 0,
            inflation: 100,
            costs: (150, 500, 0),
            liked: vec!["Hobbit".to_string(), "Mage".to_string()],
            hated: vec!["Orc".to_string()],
        };
        // STORE_LIKED: race match and class match both count.
        assert!(is_state_aux(&owner, "Hobbit", "Warrior", 1));
        assert!(is_state_aux(&owner, "Human", "Mage", 1));
        assert!(!is_state_aux(&owner, "Human", "Warrior", 1));
        // STORE_HATED.
        assert!(is_state_aux(&owner, "Orc", "Warrior", 0));
        assert!(!is_state_aux(&owner, "Hobbit", "Warrior", 0));
        // STORE_NORMAL is "neither liked nor hated".
        assert!(is_state(&owner, "Human", "Warrior", 2));
        assert!(!is_state(&owner, "Hobbit", "Warrior", 2));
        assert!(!is_state(&owner, "Orc", "Warrior", 2));
        // The price path reacts to a class match.
        let gd = load_game_data();
        let sword = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        let ctx = |class: &'static str| ShopCtx {
            owner: Some(&owner),
            race: "Human",
            class,
            chr: 18,
            no_selling: false,
        };
        assert!(buy_price(&gd, None, &sword, &ctx("Mage")) < buy_price(&gd, None, &sword, &ctx("Warrior")));
    }

    #[test]
    fn owners_price_by_race_and_greed() {
        let gd = load_game_data();
        let hobbit_owner = gd
            .owners
            .iter()
            .find(|o| o.liked.iter().any(|r| r == "Hobbit"))
            .unwrap();
        let sword = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        // A liked race pays less than a hated one at the same owner.
        let liked = ShopCtx {
            owner: Some(hobbit_owner),
            race: "Hobbit",
            class: "Warrior",
            chr: 18,
            no_selling: false,
        };
        let hated_race = hobbit_owner.hated.first().cloned().unwrap_or_default();
        let hated = ShopCtx {
            owner: Some(hobbit_owner),
            race: &hated_race,
            class: "Warrior",
            chr: 18,
            no_selling: false,
        };
        let p_liked = buy_price(&gd, None, &sword, &liked);
        let p_hated = buy_price(&gd, None, &sword, &hated);
        assert!(p_liked <= p_hated, "liked {} hated {}", p_liked, p_hated);
        // Charisma matters: a low-charisma buyer pays more.
        let low_chr = ShopCtx {
            owner: Some(hobbit_owner),
            race: "Hobbit",
            class: "Warrior",
            chr: 3,
            no_selling: false,
        };
        assert!(buy_price(&gd, None, &sword, &low_chr) >= p_liked);
        // The owner never pays above max_cost.
        let mut pricey = sword.clone();
        pricey.to_h += 500;
        let cheap_owner = gd
            .owners
            .iter()
            .find(|o| o.max_cost < pricey.cost(&gd))
            .unwrap();
        let sc = ShopCtx {
            owner: Some(cheap_owner),
            race: "Hobbit",
            class: "Warrior",
            chr: 18,
            no_selling: false,
        };
        assert!(sell_price(&gd, None, &pricey, &sc) <= cheap_owner.max_cost);
        // Stock generation assigns an owner from the store's list.
        let mut stocks = ShopStocks::default();
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        let general = gd.stores.iter().find(|s| !s.owners.is_empty()).unwrap();
        stock_for(&gd, &mut stocks, 1, general.id, &mut created, 0, 1, &mut rng);
        assert!(store_owner(&gd, &stocks, 1, general.id).is_some());
    }

    #[test]
    fn random_town_stocks_are_isolated_by_town() {
        let gd = load_game_data();
        let mut stocks = ShopStocks::default();
        let mut created = HashSet::new();
        let mut rng = crate::rng::current();
        // A RANDOM-flagged store with guaranteed entries (a smith).
        let store = gd
            .stores
            .iter()
            .find(|s| {
                s.flags.iter().any(|f| f == "RANDOM") && s.entries.iter().any(|e| e.proba >= 100)
            })
            .expect("a RANDOM store");
        // Two random dungeon towns (ids 20/21) keep separate stocks.
        let a = stock_for(&gd, &mut stocks, 20, store.id, &mut created, 0, 1, &mut rng);
        let b = stock_for(&gd, &mut stocks, 21, store.id, &mut created, 0, 1, &mut rng);
        assert!(!a.is_empty() && !b.is_empty());
        assert!(stocks.stocks.contains_key(&skey(20, store.id)));
        assert!(stocks.stocks.contains_key(&skey(21, store.id)));
        // The stocks are cached per town, not shared.
        assert_eq!(
            stock_for(&gd, &mut stocks, 20, store.id, &mut created, 0, 1, &mut rng),
            a
        );
        assert_eq!(
            stock_for(&gd, &mut stocks, 21, store.id, &mut created, 0, 1, &mut rng),
            b
        );
    }

    #[test]
    fn mass_produce_piles_and_discounts() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let food = gd.object_by_name("Ration of Food").expect("food");
        let mut saw_pile = false;
        for seed in 0..40 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut it = Item::base(&gd, food);
            mass_produce(&gd, &mut it, &mut rng);
            assert!(it.count >= 1);
            assert!(matches!(it.discount, 0 | 25 | 50 | 75 | 90));
            if it.count > 1 {
                saw_pile = true;
            }
        }
        assert!(saw_pile, "mass_produce never made a pile");
        // Random artifacts never get a discount.
        let mut rng = StdRng::seed_from_u64(1);
        let mut art = Item::base(&gd, food);
        art.artifact_name = "of Testing".to_string();
        mass_produce(&gd, &mut art, &mut rng);
        assert_eq!(art.discount, 0);
        // Ego gear never piles (store.cc:402 `if (o_ptr->name2) break`),
        // but a name1 artifact still gets the normal pile sizing. Use a
        // costless artifact so its kind's cost stays in the piling band.
        let armour = gd.object_by_name("Robe").expect("robe");
        let mut ego = Item::base(&gd, armour);
        ego.ego = 1;
        mass_produce(&gd, &mut ego, &mut rng);
        assert_eq!(ego.count, 1, "ego armour never piles");
        let cheap_art = gd.artifacts.iter().find(|a| a.cost == 0).map(|a| a.id);
        if let Some(id) = cheap_art {
            let mut saw_art_pile = false;
            for seed in 0..40 {
                let mut rng = StdRng::seed_from_u64(seed);
                let mut art = Item::base(&gd, armour);
                art.artifact = id;
                mass_produce(&gd, &mut art, &mut rng);
                saw_art_pile |= art.count > 1;
            }
            assert!(saw_art_pile, "name1 armour never piled");
        }
    }

    #[test]
    fn store_create_applies_magic_fuel_and_prune() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let torch = gd.object_by_name("Wooden Torch").expect("torch");
        let lantern = gd.object_by_name("Brass Lantern").expect("lantern");
        let mut saw_full_lite = false;
        for seed in 0..25 {
            let mut stocks = ShopStocks::default();
            let mut created = HashSet::new();
            let mut rng = StdRng::seed_from_u64(seed);
            for store in &gd.stores {
                if store.entries.is_empty() || store.is_home() {
                    continue;
                }
                let stock = stock_for(&gd, &mut stocks, 1, store.id, &mut created, 0, 1, &mut rng);
                for it in &stock {
                    // store_create's worthless prune (store.cc:1430-1434).
                    assert!(
                        item::object_value_real(&gd, it) > 0,
                        "seed {seed}: {} stocked a worthless item",
                        store.name
                    );
                    if it.def == torch || it.def == lantern {
                        saw_full_lite = true;
                        // FUEL_LITE timeout is k_info pval2 (store.cc:1410).
                        assert_eq!(it.fuel, gd.objects[it.def].fuel);
                    }
                }
            }
        }
        assert!(saw_full_lite, "no store stocked a FUEL_LITE");
    }

    #[test]
    fn store_stacking_and_deletion() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let wand_def = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_WAND && o.spell == "Magic Missile")
            .or_else(|| {
                gd.objects
                    .iter()
                    .position(|o| o.tval == crate::data::TV_WAND)
            })
            .expect("a wand");
        let mut a = Item::base(&gd, wand_def);
        a.charges = 5;
        let mut b = a.clone();
        b.charges = 7;
        // Wands stack despite differing charges (store_object_similar).
        assert!(store_object_similar(&gd, &a, &b));
        store_object_absorb(&gd, &mut a, &b);
        assert_eq!(a.count, 2);
        assert_eq!(a.charges, 12);
        // Different discounts never stack.
        let mut c = Item::base(&gd, wand_def);
        c.discount = 25;
        assert!(!store_object_similar(&gd, &a, &c));
        // Different second egos never stack (object_type.name2b).
        let sword_def = gd.object_by_name("Long Sword").unwrap();
        let mut e1 = Item::base(&gd, sword_def);
        e1.ego = 1;
        e1.ego2 = 1;
        let mut e2 = Item::base(&gd, sword_def);
        e2.ego = 1;
        e2.ego2 = 2;
        assert!(!store_object_similar(&gd, &e1, &e2));
        // A second ego that matches still stacks.
        let e3 = e1.clone();
        assert!(store_object_similar(&gd, &e1, &e3));
        // store_delete can destroy part or all of a pile.
        let mut stock = vec![a.clone(), a.clone()];
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..20 {
            store_delete(&gd, &mut stock, &mut rng);
            if stock.is_empty() {
                break;
            }
        }
        assert!(stock.is_empty() || stock.iter().all(|it| it.count >= 1));
    }

    #[test]
    fn price_item_honours_flags_and_no_selling() {
        let gd = load_game_data();
        let bm = gd.stores.iter().find(|s| s.is_black_market()).unwrap();
        let plain = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        let mut ctx = ShopCtx::neutral();
        // Black Market charges twice and pays half (STF_ALL_ITEM).
        let buy_bm = price_item(&gd, Some(bm), &plain, &ctx, false);
        let sell_bm = price_item(&gd, Some(bm), &plain, &ctx, true);
        let buy_plain = price_item(&gd, None, &plain, &ctx, false);
        let sell_plain = price_item(&gd, None, &plain, &ctx, true);
        assert!(buy_bm > buy_plain);
        assert!(sell_bm < sell_plain);
        ctx.no_selling = true;
        assert_eq!(price_item(&gd, None, &plain, &ctx, true), 0);
    }

    #[test]
    fn purchase_analyze_reacts_to_prices() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(4);
        // Paid way over the value for a worthless item: anger.
        assert!(purchase_analyze(100, 0, 0, &mut rng).is_some());
        // Paid less than the true value: joy.
        assert!(purchase_analyze(10, 100, 50, &mut rng).is_some());
        // Fair price: no comment.
        assert!(purchase_analyze(50, 50, 50, &mut rng).is_none());
    }

    #[test]
    fn say_comment_1_uses_the_comment_table_and_rumour_chance() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        const LINES: [&str; 6] = ["Okay.", "Fine.", "Accepted!", "Agreed!", "Done!", "Taken!"];
        let real: Vec<&str> = include_str!("../assets/data/rumors.txt")
            .lines()
            .filter(|l| !l.starts_with("********") && l.parse::<u32>().is_err())
            .collect();
        let mut saw_rumour = false;
        for seed in 0..400 {
            let mut rng = StdRng::seed_from_u64(seed);
            let lines = say_comment_1(&mut rng);
            assert!(LINES.contains(&lines[0].as_str()), "{}", lines[0]);
            if lines.len() > 1 {
                saw_rumour = true;
                assert_eq!(lines.len(), 3);
                assert_eq!(lines[1], "The shopkeeper whispers something into your ear:");
                assert!(real.contains(&lines[2].as_str()), "{}", lines[2]);
            }
        }
        assert!(saw_rumour, "the 1/8 rumour chance never triggered");
    }

    #[test]
    fn say_comment_4_bundles_both_kick_lines() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        const A: [&str; 4] = [
            "Enough!  You have abused me once too often!",
            "Arghhh!  I have had enough abuse for one day!",
            "That does it!  You shall waste my time no more!",
            "This is getting nowhere!  I'm going to Londis!",
        ];
        const B: [&str; 4] = [
            "Leave my store!",
            "Get out of my sight!",
            "Begone, you scoundrel!",
            "Out, out, out!",
        ];
        for seed in 0..32 {
            let mut rng = StdRng::seed_from_u64(seed);
            let (a, b) = say_comment_4(&mut rng);
            assert!(A.contains(&a.as_str()), "unexpected line {a}");
            assert!(B.contains(&b.as_str()), "unexpected line {b}");
        }
    }

    #[test]
    fn mass_roll_sums_zero_based_rand_int() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(9);
        // rand_int(5) is 0..=4, so three rolls span 0..=12 (not 3..=15).
        for _ in 0..200 {
            let v = mass_roll(3, 5, &mut rng);
            assert!((0..=12).contains(&v), "mass_roll out of range: {v}");
        }
        assert_eq!(mass_roll(4, 1, &mut rng), 0);
        assert_eq!(mass_roll(0, 5, &mut rng), 0);
        let mut saw_min = false;
        let mut saw_max = false;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let v = mass_roll(3, 5, &mut rng);
            saw_min |= v == 0;
            saw_max |= v == 12;
        }
        assert!(saw_min && saw_max, "min {saw_min} max {saw_max}");
    }

    #[test]
    fn store_check_num_checks_room_then_merges() {
        let gd = load_game_data();
        let store = gd.stores.iter().find(|s| s.name == "General Store").unwrap();
        let food = Item::base(&gd, gd.object_by_name("Ration of Food").unwrap());
        assert!(store_check_num(&gd, store, &[], &food));
        // A full store only accepts merges (store_object_similar).
        let full: Vec<Item> = (0..store.max_items as usize).map(|_| food.clone()).collect();
        assert!(store_check_num(&gd, store, &full, &food));
        let other = Item::base(&gd, gd.object_by_name("Iron Spike").unwrap());
        assert!(!store_check_num(&gd, store, &full, &other));
        // The Home follows the player's object_similar rules instead.
        let home = gd.stores.iter().find(|s| s.is_home()).unwrap();
        let full_home: Vec<Item> = (0..home.max_items as usize).map(|_| food.clone()).collect();
        assert!(store_check_num(&gd, home, &full_home, &food));
    }

    #[test]
    fn is_blessed_requires_known_flags() {
        let gd = load_game_data();
        let Some(def) = gd
            .objects
            .iter()
            .position(|o| o.flags.iter().any(|f| f == "BLESSED"))
        else {
            return;
        };
        let mut it = Item::base(&gd, def);
        it.identified = false;
        assert!(!is_blessed(&gd, &it), "unknown items hide their flags");
        it.identified = true;
        assert!(is_blessed(&gd, &it));
        let plain = Item::base(&gd, gd.object_by_name("Long Sword").unwrap());
        assert!(!is_blessed(&gd, &plain));
    }

    #[test]
    fn home_carry_merges_sorts_and_caps() {
        let gd = load_game_data();
        let food_def = gd.object_by_name("Ration of Food").unwrap();
        let mut food = Item::base(&gd, food_def);
        food.identified = true;
        // Merge into an identical stack.
        let mut stock = vec![food.clone()];
        let mut more = food.clone();
        more.count = 2;
        assert_eq!(home_carry(&gd, &mut stock, more, 24), Some(0));
        assert_eq!(stock[0].count, 3);
        // Sorted insert by decreasing tval.
        let spike_def = gd.object_by_name("Iron Spike").unwrap();
        let mut spike = Item::base(&gd, spike_def);
        spike.identified = true;
        let (food_t, spike_t) = (gd.objects[food_def].tval, gd.objects[spike_def].tval);
        let first = if food_t >= spike_t { food_def } else { spike_def };
        let mut stock = Vec::new();
        home_carry(&gd, &mut stock, food.clone(), 24);
        home_carry(&gd, &mut stock, spike.clone(), 24);
        assert_eq!(stock[0].def, first);
        // Rods sort by increasing recharge time.
        let rod_def = gd.objects.iter().position(|o| o.tval == crate::data::TV_ROD_MAIN);
        if let Some(rod_def) = rod_def {
            let mut slow = Item::base(&gd, rod_def);
            slow.identified = true;
            slow.timeout = 100;
            let mut fast = slow.clone();
            fast.timeout = 50;
            let mut stock = Vec::new();
            home_carry(&gd, &mut stock, slow, 24);
            home_carry(&gd, &mut stock, fast, 24);
            assert_eq!(stock[0].timeout, 50);
        }
        // The slot cap holds.
        let mut stock = Vec::new();
        assert!(home_carry(&gd, &mut stock, food.clone(), 2).is_some());
        assert!(home_carry(&gd, &mut stock, spike.clone(), 2).is_some());
        let third = Item::base(&gd, gd.object_by_name("Wooden Torch").unwrap());
        assert_eq!(home_carry(&gd, &mut stock, third, 2), None);
    }

    #[test]
    fn store_item_helpers_clamp_and_erase() {
        let gd = load_game_data();
        let food = Item::base(&gd, gd.object_by_name("Ration of Food").unwrap());
        let mut stock = vec![food.clone(), food.clone()];
        store_item_increase(&mut stock, 0, -1);
        assert_eq!(stock[0].count, 0);
        store_item_optimize(&mut stock, 0);
        assert_eq!(stock.len(), 1);
        store_item_increase(&mut stock, 0, 1000);
        assert_eq!(stock[0].count, 255);
        store_item_increase(&mut stock, 0, -1000);
        assert_eq!(stock[0].count, 0);
        store_item_increase(&mut stock, 5, 1);
        store_item_optimize(&mut stock, 5);
        assert_eq!(stock.len(), 1);
    }

    #[test]
    fn black_market_crap_skips_items_sold_elsewhere() {
        let gd = load_game_data();
        let food = Item::base(&gd, gd.object_by_name("Ration of Food").unwrap());
        let mut stocks = ShopStocks::default();
        assert!(!black_market_crap(&gd, &stocks, 1, &food));
        // Sold by another town store: crap for the Black Market.
        let general = gd
            .stores
            .iter()
            .find(|s| s.name == "General Store")
            .unwrap();
        stocks
            .stocks
            .insert(skey(1, general.id), vec![food.clone()]);
        assert!(black_market_crap(&gd, &stocks, 1, &food));
        // Ego items and plussed items are never crap.
        let mut ego = food.clone();
        ego.ego = 1;
        assert!(!black_market_crap(&gd, &stocks, 1, &ego));
        let mut good = food.clone();
        good.to_a = 1;
        assert!(!black_market_crap(&gd, &stocks, 1, &good));
        // Another town's stock does not count.
        assert!(!black_market_crap(&gd, &stocks, 2, &food));
        // A different kind is not crap.
        let other = Item::base(&gd, gd.object_by_name("Iron Spike").unwrap());
        assert!(!black_market_crap(&gd, &stocks, 1, &other));
    }

    #[test]
    fn return_level_follows_store_flags() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(11);
        let base = gd.stores[0].clone();
        let mut plain = base.clone();
        plain.flags.clear();
        for _ in 0..50 {
            assert!((1..=5).contains(&return_level(&plain, 0, 0, &mut rng)));
        }
        let mut random = base.clone();
        random.flags = vec!["RANDOM".to_string()];
        assert_eq!(return_level(&random, 0, 0, &mut rng), 0);
        let mut depend = base.clone();
        depend.flags = vec!["DEPEND_LEVEL".to_string()];
        for _ in 0..50 {
            assert!((11..=15).contains(&return_level(&depend, 10, 0, &mut rng)));
        }
        let mut shallow = base.clone();
        shallow.flags = vec!["RANDOM".to_string(), "SHALLOW_LEVEL".to_string()];
        for _ in 0..50 {
            assert!((5..=9).contains(&return_level(&shallow, 0, 0, &mut rng)));
        }
        let mut all = base.clone();
        all.flags = vec!["RANDOM".to_string(), "ALL_ITEM".to_string()];
        assert_eq!(return_level(&all, 0, 7, &mut rng), 7);
    }

    #[test]
    fn kind_is_storeok_filters_artifacts_and_shallow_kinds() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let mut rng = StdRng::seed_from_u64(12);
        let sword = gd.object_by_name("Long Sword").unwrap();
        assert!(kind_is_storeok(
            &gd,
            sword,
            crate::data::TV_SWORD,
            0,
            &mut rng
        ));
        assert!(!kind_is_storeok(
            &gd,
            sword,
            crate::data::TV_BOOTS,
            0,
            &mut rng
        ));
        for flag in ["NORM_ART", "INSTA_ART"] {
            if let Some(d) = gd.objects.iter().position(|o| o.flags.iter().any(|f| f == flag)) {
                let tval = gd.objects[d].tval;
                assert!(
                    !kind_is_storeok(&gd, d, tval, 0, &mut rng),
                    "{flag} kind stocked"
                );
            }
        }
        // The FORCE_LEVEL depth gate excludes kinds below level/2.
        assert!(!kind_is_storeok(
            &gd,
            sword,
            crate::data::TV_SWORD,
            10_000,
            &mut rng
        ));
    }

    #[test]
    fn emptied_stock_refills_or_retires_per_store_cc() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let store = gd
            .stores
            .iter()
            .find(|s| s.owners.len() > 1 && !s.is_home() && !s.is_black_market())
            .unwrap();
        // retire_owner_p (store.cc:1890): with more than one candidate
        // owner the original always refuses, whatever the roll.
        let mut rng = StdRng::seed_from_u64(0);
        for seed in 0..64 {
            let mut r = StdRng::seed_from_u64(seed);
            assert!(!retire_owner_p(store, &mut r));
        }
        // A hypothetical single-owner store retires on the 1/21 roll.
        let mut single = store.clone();
        single.owners = vec![store.owners[0]];
        let hits = (0..210)
            .filter(|s| retire_owner_p(&single, &mut StdRng::seed_from_u64(*s)))
            .count();
        assert!((1..209).contains(&hits), "1/21 roll produced {hits} hits");

        // store_shuffle: new owner, ban cleared, stock discounted "on
        // sale" (store.cc:3422).
        let key = skey(1, store.id);
        let mut stocks = ShopStocks::default();
        stocks.owners.insert(key, store.owners[0]);
        stocks.open_until.insert(key, 999_999);
        stocks.stocks.insert(
            key,
            vec![Item::base(&gd, gd.object_by_name("Ration of Food").unwrap())],
        );
        store_shuffle(&gd, &mut stocks, 1, store.id, &mut rng);
        assert_ne!(stocks.owners[&key], store.owners[0]);
        assert!(store.owners.contains(&stocks.owners[&key]));
        assert!(!stocks.open_until.contains_key(&key), "ban not cleared");
        assert_eq!(stocks.stocks[&key][0].discount, 50);
        assert_eq!(stocks.stocks[&key][0].inscription, "on sale");

        // An emptied store always brings out new stock for these shops
        // (the retire branch needs <=1 owner) and runs ten maintain
        // passes (store.cc store_purchase).
        let mut stocks = ShopStocks::default();
        stocks.owners.insert(key, store.owners[0]);
        stocks.stocks.insert(key, Vec::new());
        let mut log = crate::game::MessageLog::default();
        let mut created = HashSet::new();
        let mut rng = StdRng::seed_from_u64(3);
        emptied_stock(
            &gd,
            &mut stocks,
            1,
            store.id,
            &mut log,
            &mut created,
            0,
            1,
            &mut rng,
        );
        assert!(log
            .lines
            .iter()
            .any(|l| l == "The shopkeeper brings out some new stock."));
        assert!(!stocks.stocks[&key].is_empty(), "stock was not refilled");
        // A non-empty stock is left alone (no message, no maintain).
        let before = stocks.stocks[&key].clone();
        let mut log = crate::game::MessageLog::default();
        emptied_stock(
            &gd,
            &mut stocks,
            1,
            store.id,
            &mut log,
            &mut created,
            0,
            1,
            &mut rng,
        );
        assert_eq!(stocks.stocks[&key], before);
        assert!(log.lines.is_empty());
    }

    #[test]
    fn maintain_prunes_and_refills_stock() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let gd = load_game_data();
        let store = gd.stores.iter().find(|s| s.name == "General Store").unwrap();
        let mut stocks = ShopStocks::default();
        let mut created = HashSet::new();
        let mut rng = StdRng::seed_from_u64(5);
        let first = stock_for(&gd, &mut stocks, 1, store.id, &mut created, 0, 1, &mut rng);
        assert!(!first.is_empty());
        let key = skey(1, store.id);
        // The first maintenance of the day runs, then the once-a-day
        // guard leaves the stock alone.
        maintain(&gd, &mut stocks, 1, store.id, 1, &mut created, 0, 1, &mut rng);
        assert_eq!(stocks.last_day[&key], 1);
        let before = stocks.stocks[&key].clone();
        maintain(&gd, &mut stocks, 1, store.id, 1, &mut created, 0, 1, &mut rng);
        assert_eq!(stocks.stocks[&key], before);
        // Later days turn the stock over, always inside the slot cap.
        let mut changed = false;
        for day in 2..40 {
            maintain(&gd, &mut stocks, 1, store.id, day, &mut created, 0, 1, &mut rng);
            let stock = &stocks.stocks[&key];
            assert!(stock.len() <= store.max_items as usize);
            changed |= *stock != before;
        }
        assert!(changed, "turnover never changed the stock");
        // The Home is never refilled by maintain.
        let before_home = stocks.stocks.get(&skey(1, HOME_STORE)).cloned();
        maintain(&gd, &mut stocks, 1, HOME_STORE, 50, &mut created, 0, 1, &mut rng);
        assert_eq!(stocks.stocks.get(&skey(1, HOME_STORE)).cloned(), before_home);
    }

    #[test]
    fn store_banned_only_while_the_ban_lasts() {
        let mut stocks = ShopStocks::default();
        let key = skey(1, 1);
        assert!(!store_banned(&stocks, key, 100));
        stocks.open_until.insert(key, 500);
        assert!(store_banned(&stocks, key, 499));
        assert!(store_banned(&stocks, key, 500));
        assert!(!store_banned(&stocks, key, 501));
        assert!(!store_banned(&stocks, skey(1, 2), 100));
    }

    #[test]
    fn adjust_store_top_keeps_the_page_in_range() {
        // Nothing left: back to the first page.
        assert_eq!(adjust_store_top_item_removed(0, 24), 0);
        // Already on the first page: untouched.
        assert_eq!(adjust_store_top_item_removed(30, 0), 0);
        // The page start no longer exists: step back one page.
        assert_eq!(adjust_store_top_item_removed(20, 24), 12);
        // Still within the stock: untouched.
        assert_eq!(adjust_store_top_item_removed(30, 12), 12);
    }

    #[test]
    fn price_item_quarters_only_shot_arrows_and_bolts() {
        let gd = load_game_data();
        let ctx = ShopCtx::neutral();
        if let Some(def) = gd
            .objects
            .iter()
            .position(|o| o.tval == crate::data::TV_BOOMERANG)
        {
            let it = Item::base(&gd, def);
            let sell = price_item(&gd, None, &it, &ctx, true);
            assert!(
                sell * 2 > it.cost(&gd).max(1),
                "boomerang sell {sell} vs cost {}",
                it.cost(&gd)
            );
        }
        // Shot really is quartered.
        if let Some(def) = gd.objects.iter().position(|o| o.tval == crate::data::TV_SHOT) {
            let it = Item::base(&gd, def);
            let sell = price_item(&gd, None, &it, &ctx, true);
            assert!(sell * 4 <= it.cost(&gd).max(1));
        }
    }

    #[test]
    fn store_actions_resolve_to_building_actions() {
        let gd = load_game_data();
        for store in &gd.stores {
            for &a in &store.actions {
                assert!(
                    gd.building_actions.iter().any(|b| b.id == a),
                    "{} action {a} missing",
                    store.name
                );
            }
        }
        let smith = gd.stores.iter().find(|s| s.name == "Weaponsmith").unwrap();
        assert!(!smith.actions.is_empty());
    }
}
