# objects audit report

Scope: `src/object1.cc` (75), `src/object2.cc` (75), `src/object_filter.cc` (11),
`src/object_flag_meta.cc` (0) vs `bevy/src/{item,modal,game,input,town,data,render,spell,mimic}.rs`
and `bevy/assets/data/*.ron` / `bevy/tools/convert_data.py`.

Known-done context (item instances, inventory slots, identification *system*,
ego/artifact application, make_item/make_object/make_artifact_special,
drops/floor stacks, charges/fuel/devices, activation mapping, weight/burden,
curse handling, stack merging, k_info full data) was not re-reported as a
whole; only concrete mismatches inside those areas are listed.

## GAPS

### A. Object kind selection / allocation

- [x] `object2.cc:568 get_obj_num` + `object2.cc:524 get_obj_num_prep` + `init2.cc:715-775`
  — the whole allocation table is built from the k_info `A:level/chance` lines
  (`prob1 = 100/chance`), and `get_obj_num` (a) boosts the level 1/60
  (`GREAT_OBJ`), (b) re-picks the "better" of 2 (p<60) / 3 (p<10) rolls using
  `entry.level`. Rust `item::gen_object_kind` (`item.rs:859-889`) ignores `A:`
  entirely: it filters `o.depth <= depth + 2` and weights by `10/rarity`,
  where `rarity` is the **W: second field** (`convert_data.py:504-509`), which
  `init1.cc:2405-2418` parses as *unused* — 568/596 kinds have it 0
  (converter `max(1,0)=1`), so nearly every kind gets weight 10 (e.g.
  "Restoring" `A:20/8:30/4:40/1` is as common as a ration from depth 20).
  No level boost, no 60%/10% best-of-N. — fix in `convert_data.py` (emit
  allocation rows) + `item.rs::gen_object_kind`.

- [x] `object2.cc:4108 init_match_theme` / `object2.cc:4129 kind_is_theme`
  — the per-dungeon `d_info` `O:treasure:combat:magic:tools` theme
  (28 live `O:` records in `lib/edit/d_info.txt`) restricts objects per
  dungeon. Rust has no equivalent (grep `kind_is_theme|match_theme` in
  `bevy/src` is empty); `make_object`/`place_object`/`scatter_objects` ignore
  the dungeon object theme completely. — fix in `data.rs` (parse `O:`),
  `item.rs::gen_object_kind`.  Reconciled: `kind_is_theme`/`gen_object_kind_themed` exist and are
  used for vault objects and F: markers (game.rs:6666-6682/6928); `item::scatter_objects`
  (item.rs:3851) still calls `make_object` with the empty theme — pass `gd.dungeon(dungeon).theme`.
  **Done (2026-09-16):** `item::scatter_objects` now takes the dungeon id and rolls
  `gd.dungeon(dungeon).theme` through `make_object_themed`; all floor/vault/spec/quest placements are
  themed.  The C++ allocation cache (`init_match_theme`'s `kind_table_valid`) is not modelled (the
  table is rebuilt per call, so the RNG stream differs but the distribution is the same).

- [x] `object2.cc:4354 kind_is_good` — the "good object" allocation
  (`make_object` sets `get_object_hook = kind_is_good`, 4507-4515) restricts
  good drops to undamaged armour/weapons, silver+ rods, expensive tips,
  good books, Ring of Speed and 7 named amulets. Rust `make_object_ex`
  (`item.rs:1661-1685`) calls the *unrestricted* `gen_object_kind` for both
  normal and good objects; `good` only affects `make_item`'s ego/artifact
  chances. — fix in `item.rs`.

- [x] `object2.cc:4311 kind_is_legal` — Rust `gen_object_kind` excludes
  skeleton/corpse/egg/gold but not `TV_HYPNOS`: k_info 653 "Symbiote" (no
  SPECIAL_GENE, depth 127) can be generated on the floor with `note=0`
  (C++ returns false for `TV_HYPNOS`, 4335-4338). C++ also suppresses the
  generic corpse svals and `SV_RING_SPECIAL` (4329-4344). C++ `NORM_ART`
  uniqueness (`k_ptr->artifact`, 4324-4327 + `rescue_artifact` 372-392) is
  absent: NORM_ART kinds **without** SPECIAL_GENE can be created repeatedly
  in Rust — `Blood of Life` (items.ron id 573), `Ring of Precognition` (700),
  `Old Scroll of Deincarnation` (720), `Potion of Learning` (743),
  `Home Summoning` (776), `Greater Ration of Health` (801), `Crumpled Scroll
  of Mass Resurrection` (802), `Gnarled Staff of Holy Fire of Mithrandir`
  (811). — fix in `item.rs::gen_object_kind` + a created-kind set.

- [x] `object2.cc:4445 kind_is_artifactable` — `generate.cc:7601-7620`
  (FATE_FIND_A randart) allocates with this filter (good kinds that can carry
  at least one `ra_info` power). Rust `game.rs:5148-5158` uses
  `gen_object_kind(gd, depth+5)` with no restriction, then `create_artifact`;
  a fate randart can therefore be a potion/scroll/junk. Rust `item::artifactable`
  (`item.rs:1750`) is the *different* spells2 scroll filter (tval list only).
  — fix in `game.rs`.

- [x] `object2.cc:4484 make_object` / `object2.cc:4603 place_object` — Rust
  approximates both (`item.rs:1649-1685`, `scatter_objects` 2190-2232): no
  allocation cache/hook, no theme, no `where` (`OBJ_FOUND_VAULT/FLOOR/SPECIAL/
  RUBBLE`), no `found`/`found_aux` bookkeeping (see object_out_desc below),
  no object count/level rating (`rating`, `good_item_flag`, 4574-4586),
  and vault/`OBJ_FOUND_*` placement is not distinguishable.  Reconciled: the theme now flows
  through `make_object_themed`; still missing the `OBJ_FOUND_*` `where`/`found_aux` bookkeeping
  (no `found` fields on `Item`), the `rating`/`good_item_flag` counters and the allocation cache
  (item.rs:3002-3052).
  **Done (2026-09-16):** `found`/`found_aux1..4` are set for FLOOR (dungeon+level), VAULT
  (dungeon+level), SPECIAL and RUBBLE; `item::rating_of` ports the apply_magic/make_object rating
  contributions (artifact +10/+10, randart +40, ego X: rating, dragon gear, speed/lordly/magi rings,
  Blood of Life, out-of-depth, good_item_flag) and `populate_level`/`scatter_objects` update
  `Map.rating`/`good_item`/`feeling`.  The allocation cache and the `invprob` luck term (needs the
  player at generation time) are not modelled.
- [x] `object2.cc:5123 acquirement` — the Scrolls of Acquirement pass
  `great=true` and `*Acquirement*` drops `randint(2)+1` objects
  (`cmd6.cc:3045-3054`); Rust's scroll path (`modal.rs:11195-11201`) calls
  `make_object(good=true)` (great=false, `item.rs:1649-1657`), always drops
  exactly 2 for the starred scroll, and rolls at `ps.depth + 10`
  (`modal.rs:12244-12248`) instead of the current object level.  Reconciled: closed in the
  modal.rs/item.rs session — the effect calls `make_object_ex(..., good=true, great=true)` at the
  current object level (the +10 good base is applied inside make_object) and loops
  `randint(2)+1` for the starred scroll.

### B. Item value

- [x] `object2.cc:756 flag_cost` — the entire flag price table (STR 1000×pval,
  speed 10000+2500×pval, immunities 10000, ESP 12500 each, curses negative,
  ACTIVATE extra by activation id, LEVELS `elevel*2000`, ...) is not ported.
  Rust `Item::cost` (`item.rs:156-169`) is `base + ego.cost + artifact.cost +
  30*(|to_h|+|to_d|+|to_a|)` ×count (cursed→0, and `artifact.cost` can
  double-count base). Missing all flag/pval/activation value and
  `TR_TEMPORARY`/`TR_CURSE_NO_DROP`→0 early-outs. Every shop price and
  `object_value` consumer (incl. pack sort and recharge/enchant) is wrong.
- [x] `object2.cc:990 object_value_real` — missing per-tval pricing: wand/staff
  `spell_level * ((pval3>>16)+low)/2 / 6 + charges` (1138-1168), rod main tip
  cost (1180-1200), book spell 255 `value * skill_level` (1169-1178), bow /
  armour / weapon / ammo bonus pricing incl. extra damage dice and exploding
  arrow ×14 (1239-1288), `TR_SPELL_CONTAIN` 5000+500×skill (1058-1065),
  egg monster level (1126-1136), pval credits (CRIT 500, SPEED 30000,
  BLOWS 2000, ... 1095-1117), artifact/ego "worthless" cost==0 handling.
- [x] `object2.cc:1300 object_value` — `Item` has no `discount` field; the
  `value -= value*discount/100` path (and the `object_desc` `"N% off"`
  inscription, object1.cc:1688-1691) is missing. `merge_target` also cannot
  absorb the largest discount (`object_absorb`, 1649).

### C. Identification / flavors

- [x] `object1.cc:752 object_desc_aux` + `object1.cc:242 object_easy_know` +
  `object1.cc:327 flavor_init` — flavored consumables are never shown as an
  unidentified kind.  C++ uses base strings `"& Potion~"`, `"& Scroll~"`,
  `"& Wand~"`, ... and only appends the real kind name when `aware`
  (851, 886, 923, 934...); Rust `Item::base` sets
  `identified: !needs_identify(tval) || ...` where `needs_identify` is only
  true for equippable slots (`item.rs:139,356-358`), so every potion/scroll/
  wand/staff/rod starts `identified=true` and `Item::label`'s generic-name
  branch (`item.rs:196-207`) is dead for them.  Consequence: floor and shop
  consumables display their true effect name at all times and
  `render.rs:497` (`known = it.identified || inv.known.contains`) never
  applies the shuffled flavor colour to them.  `input.rs:830` shows the
  intended pattern (`potion.identified = ctx.inv.known.contains(&def)`).
  C++'s `object_aware`/`object_known` on `sense_floor`/pickup
  (`object1.cc:5242-5244`, `object2.cc:5498-5500`) also means the original
  hides the name only until the grid is stepped on; a fixed Rust port should
  either copy that or gate on the `known` set consistently.
- [x] `object2.cc:740 object_aware` / `object2.cc:749 object_aware_p` —
  "awareness" is per-kind (`k_ptr->aware`) in C++ and per-item in Rust
  (`identified`) plus a per-pack `known` set; the two are not synchronised
  (nothing calls `Inventory::learn` on pickup/step, and `Item::base` never
  starts a flavored consumable unaware), the same root cause as above.
- [x] `object1.cc:242 object_easy_know` — the "whole-tval easy know" list
  (druid/music/symbiotic books, flask/egg/bottle/skeleton/corpse/hypnos/spike/
  junk, food/potion/scroll/rod unless `TR_NORM_ART`, ring/amulet/lite with
  `TR_EASY_KNOW`) is not modelled as awareness; it only happens to be
  effectively true because consumables are force-identified (above).
- [~] `object1.cc:327 flavor_init` — Rust models only colours
  (`data.rs:1538-1563`, `item.rs:3214-3229`); no `aware` for no-flavor kinds
  (C++ 359-362), no per-kind `easy_know` field.  Shuffle order is per-group
  over sval-sorted members instead of the fixed `object_colors_*` arrays; the
  colour multiset happens to match, but the glyph/attr derivation
  (`object_attr/object_char`) is not used for flavored kinds (base G: glyph is
  constant per group, so visually equivalent).  Mostly a consequence of the
  item above.  Reconciled: colours + per-kind EASY_KNOW/easy_know are now modelled
  (`flavor_color` item.rs:5267, `ObjectDef::easy_know` data.rs:631); the remaining differences are
  cosmetic (per-seed shuffle order over sval-sorted members, glyph/attr derivation), so this is
  `[~]` frontend.

### D. Magic application / generation tables

- [x] `object2.cc:2192 make_ego_item` + `object2.cc:3724 apply_magic` +
  `convert_data.py:603-643` — the ego pipeline is materially simplified:
  * `EgoDef` (`data.rs:608-628`) has no `min_sval/max_sval` (converter keeps
    only the tval of each `T:` line), no `need_flags`/`forbid_flags` (`r:N:`/
    `r:F:` are not parsed), and `apply_ego` (`item.rs:904-1060`) never
    distinguishes good (`cost>0`) from bad (`cost==0`) egos (C++ 2230-2232),
    never rolls the out-of-depth `rand_int(e.level - dun_level)` (2255-2266)
    or `rand_int(mrarity - luck) > rarity` (2268-2272, uses weight `10/mrarity`),
    and never tries the 7% double ego `name2b` with prefix/suffix constraints
    (2286-2327).
  * The e_info `R:` rarity groups are lost by the converter: C++
    `apply_magic` (4002-4011) rolls `magik(e_ptr->rar[j])` per group (group 0
    is `R:100` = always, later groups are random powers) and calls
    `add_random_ego_flag(e_ptr->fego[j])`.  `parse_e_info` merges every `F:`
    into one set, so 32 egos (67 power flags) get their random powers as
    always-on: e.g. "of Resistance" always gets `R_HIGH`, "of the Magi"
    always gets `SPELL`, `SPELL_CONTAIN`, `WIELD_CAST`, "(Defender)" always
    gets `RES_POIS`.
  * As a direct consequence `PVAL_M1/M2/M3` (generation powers that *add*
    pval: `object2.cc:3468-3489`, `+m_bonus(n,dun_level)`) are misread by
    `item.rs:924-929` as a **fixed negative pval**: ego 60 "of Speed"
    (`egos.ron`, C: pval 10) becomes `pval = -3` (boots of Speed slow you!),
    likewise "of Mana/Power/Wizardry/Extra Might/Extra Shots/of Aman/Dragon/
    Thunderlords/Magi/Eldar".  This is a behaviour bug, not just balance.
  * C: plusses use `randint(max)` (1..=max) in C++ (4023-4032); Rust
    `roll_signed` (`item.rs:891-897`) allows 0.  Reconciled: sval filters, need/forbid flags,
  R: groups, PVAL_M* and randint plusses are now in `apply_ego` (item.rs:2010-2122); the 7%
  double-ego `name2b` roll with the same-ego and prefix/suffix constraints is implemented via
  `Item.ego2` and a second `apply_ego_effects` pass (object2.cc:2286-2327).
  **Done (2026-09-16):** the pipeline is `apply_ego` (sval filters, good/bad cost gate, need/forbid
  flags, out-of-depth and mrarity/rarity rolls, R: groups, C: randint plusses, double ego) driven from
  `apply_magic`/a_m_aux_* exactly where the original calls make_ego_item (power > 1 / < -1).
- [x] `object2.cc:3724 apply_magic` power curve — C++ derives the item's power
  from `f1 = lev+10+luck(-15,15)` (cap 75) and `f2 = f1/2` (cap 20), with
  `good`/`great` forcing power 1/2, `power<0` for cursed, and up to 4 artifact
  rolls when `great`. Rust `make_item` (`item.rs:1592-1640`) replaces this
  with flat probabilities (2%/0.4% artifact, 12% ego, 4% cursed, 10% enchant),
  one artifact roll, no `force_power`, no `NORM_ART` "tricked us" re-prep
  (3745-3785), no `ART_POWER` quest gating (3859-3865).  Reconciled: the NORM_ART fallback
  (item.rs:2831-2845) and the One Ring quest gate (item.rs:3069-3071) now exist; the f1/f2 power
  curve and `force_power` are still replaced by flat probabilities (item.rs:2906-2954).
  **Done (2026-09-16):** `item::apply_magic` implements the full curve: `lev = depth + luck(-7,7)`
  capped at MAX_DEPTH-1, `f1 = lev+10+luck(-15,15)` cap 75, `f2 = f1/2` cap 20, good/great forcing,
  cursed/broken negatives, `force_power`, 1 roll at power 2 / 4 rolls when `great`, the artifact stat
  copy and the per-tval a_m_aux_1/2/3/4 calls.  `make_item` is now `apply_magic(okay=true, great=false)`.
  `force_power` is exposed but only used by tests (the port has no wizard UI); store stock still calls
  `make_item` (okay=true) in town.rs — cross-file.
- [x] `object2.cc:2120 make_artifact` — Rust `apply_artifact` (`item.rs:1433-1475`)
  filters tval/sval/insta-art/SPECIAL_GENE/created but uses WeightedIndex
  `10/rarity`; C++ also rolls the out-of-depth `rand_int((a.level-dun)*2)`
  (2151-2159) and `rand_int(rarity - luck)` (2161-2162) per candidate in list
  order, and applies artifact pval/ac/dice/weight/to_* (3876-3884) — Rust
  does not copy `a.weight` (only pval/ac/dice/to_h/to_d/to_a).  Reconciled: list-order candidates
  with the out-of-depth and rarity rolls are now in `apply_artifact` (item.rs:2666-2712); still
  missing the artifact weight copy (object2.cc:3884) — `ArtifactDef` (data.rs:775) has no weight
  field, the converter would have to emit a_info W:.
  **Done (2026-09-16):** the weight copy is in `apply_artifact`/`imbue_artifact`/`specific_artifact_item`
  via the new `Item.weight` + `ArtifactDef.weight` (converter now emits a_info W: field 3).  The
  `input.rs`/`modal.rs` weight readers still consult the base kind (cross-file).
- [x] `object2.cc:2350 a_m_aux_1` — missing: exploding missiles
  (`power==1 && !name2`, 30%, `pval2 = one of 27 GF_*`, 2419-2442) with its
  `(exploding)` description (object1.cc:1449), ×14 value (object2.cc:1283) and
  on-hit projection; `TV_MSTAFF` gets `TR_SPELL_CONTAIN|TR_WIELD_CAST`
  (2414-2418); enchant amounts `randint(5)+m_bonus(5,level)` / `m_bonus(10,
  level)` vs Rust `gen_range(1..=3)` (`item.rs:1615-1627`).  **Done (2026-09):** exploding
  missiles, the MSTAFF flags and the `randint(5)+m_bonus(5,level)` plus rolls for both the
  cursed and the enchant branch (weapons and armour) are in `item::make_item`.
- [x] `object2.cc:2468 a_m_aux_2` + `object2.cc:2447 dragon_resist` — missing
  elven cloak `pval = randint(4)` (2526-2531) and the dragon shield/helm
  `dragon_resist` rolls + rating boost (2552-2583).  `dragon_resist` only
  exists for the ego `R_DRAGON` power (`item.rs:980-993`).
- [x] `object2.cc:3028 a_m_aux_4` — missing:
  * random spellbooks `sval == 255`: `get_random_spell(SKILL_MAGIC(75%) |
    SKILL_SPIRITUALITY, level)` stored in pval (3056-3081).  Rust `make_item`
    returns before book handling (`item.rs:1588-1591`), so a generated
    "Spellbook of #" (items.ron id 757) has pval 0, an empty substituted name
    (`item.rs:224-236`) and no castable spell (`spell.rs:364-385` only knows
    sval 50/52/school tomes); the store path (`town.rs:165-177`) does resolve
    one.
  * lite fuel: `timeout = randint(k_ptr->pval2)` for `FUEL_LITE` (3083-3097);
    Rust `Item::base` sets `fuel = o.fuel` (full) for every lite (`item.rs:103`).
  * horns of Dragonkind: `pval2 = GF_ELEC/FIRE/COLD/ACID` for ego
    `EGO_INST_DRAGONKIND` (3253-3281) — `is_ego_p` use; absent.
  * corpse/egg/hypnos monster rolls and derived values (3099-3167): corpses
    come only from monster death in Rust, and the weight/meat/exp/elevel
    formulas (`o_ptr->pval = r_ptr->weight*3 + rand_int(weight)+1`,
    `elevel`, `exp=0`, `pval3=maxroll`) are not applied to eggs/hypnos.  Reconciled: random books
  (item.rs:2860-2875), lite fuel (2876-2881) and exploding missiles are done, and the
  corpse/egg/hypnos monster rolls (get_mon_num, HAS_EGG filter, weight/elevel) are now in
  `make_item`; still missing the horn of Dragonkind pval2 (EGO_INST_DRAGONKIND; the BA_*_H horn
  activations themselves are not in the port either) (object2.cc:3253-3281).
  **Done (2026-09-16):** the Dragonkind horn `pval2` GF_* roll (ego 130) is in `a_m_aux_4`; the
  BA_*_H horn activations were already mapped in `spell::artifact_activation`, and random books,
  lite fuel, corpse/egg/hypnos rolls and the potion/morphic rolls are all ported.
- [x] `object2.cc:2597 a_m_aux_3` — the cursed Amulet of the Serpents reverses
  only `pval` (2912-2919), as before. Random-resistance rolling now uses the
  full 41-case `item::random_resistance` table with the `ArtifactBias` chain
  for the specific-id mapping `random_resistance(o, randint(34)+4)` on the
  Resistance amulet / Lordly ring, so `SH_*`/`REFLECT` outcomes match (see
  `reports/02-spells.md`).
- [x] `object2.cc:3292 add_random_ego_flag` — inlined in `apply_ego`; all
  live `ETR_*` powers in the kept egos are handled *except* that PVAL_M* are
  wrong (see above) and DAM_SIZE/DAM_DIE have no live ego (`DAM_DIE` sits on
  ego 120 "of Slaying", which has no `T:` line → unreachable).

### E. Stacking / inventory

- [x] `object2.cc:1341 object_similar` / `object2.cc:1631 object_absorb` /
  `object2.cc:5496 inven_carry` — Rust `merge_target` (`item.rs:782-807`)
  only merges ammo/potions/scrolls/food/same-charge wands+staves; C++ also
  stacks fully-known weapons/armour/rings/amulets/lites with identical
  to_h/to_d/to_a/pval/timeout/art_flags/name1/name2/name2b (1501-1586), and
  permits merging when only one inscription is present (1613-1617).  Missing
  keys: `TR_RECHARGED` (1437-1438, 1468-1469 — `recharge_device`,
  `item.rs:2666-2775`, never sets an instance RECHARGED flag either),
  exploding `pval2` (1555), pval2 for potions/scrolls/food.  `object_absorb`'s
  known-blend, inscription blend and max-discount (1638-1649) are absent.
  `inven_carry`'s pre-reorder by tval/sval/aware/known/value (5546-5612) is
  absent: Rust `add_with` appends (`item.rs:769-778`), so pack order is
  insertion order.  Reconciled: `items_similar`/`absorb_item` now cover the keys incl. pval2 and
  RECHARGED (item.rs:967-1036); remaining: `recharge_device` never SETS RECHARGED (item.rs:4349,
  only compared) and `inven_carry`'s tval/sval/aware/value pre-reorder is still absent
  (`add_with` appends, item.rs:922).
- [x] `object2.cc:5838 combine_pack` / [x] `object2.cc:5911 reorder_pack` —
  Rust merges only when adding to the pack (`add_with`, `item.rs:769-778`);
  there is no combine pass over the pack and no reordering, no
  "You combine/reorder some items in your pack." messages.  Reconciled: unchanged — no
  `combine_pack` pass exists (grep `combine_pack|reorder_pack` in bevy/src = 0); `add_with` merges
  on insertion only (item.rs:922).  **Closed (2026-09):** `item::combine_pack` and the new
  `item::reorder_pack` (decreasing tval, awareness, increasing sval, identification, rod recharge
  time, decreasing value; "You reorder some items in your pack.") run in sequence every input
  frame (input.rs), and `items_similar` compares `ego2`.
- [x] `object2.cc:5653 inven_takeoff` / `object2.cc:5761 inven_drop` /
  `object2.cc:5234 inven_item_optimize` — Rust `Modal::TakeOff`
  (`modal.rs:9114-9159`) and `Modal::Drop` (`modal.rs:8590-8708`) differ:
  no symbiote special case ("You carefully drop the poor monster on the
  floor", inven_takeoff 5727-5734), no per-slot takeoff messages
  ("You were wielding/holding/...", 5688-5722), no partial amounts
  (`amt`), no dropping worn equipment directly (must take off first), no
  wand charge split (`o_ptr->pval * amt / number`, inven_drop 5805-5812),
  and taking off/wielding pushes the old item into the pack with no 23-slot
  check (`inven_carry` overflow path, 5496-5639; pack can exceed 23 via
  swaps).  Reconciled: quantity/charge-split drops (input.rs picker_drop) and taking off into the
  pack exist; still missing the symbiote "carefully drop the poor monster" case, the per-slot
  takeoff messages and the 23-slot check when taking off/wielding (modal.rs:9823/11802).
  **SESSION (modal.rs) 2026-09-16:** done — `Modal::TakeOff` prints the per-slot verb
  ("You were wielding/holding/carrying in your quiver/using/wearing X (a).") and gently drops a
  symbiotic host ("You carefully drop the poor monster on the floor."); both take-off and wielding
  now overflow past 23 slots by dropping the extra item with "Your pack overflows!" (dungeon.cc).
  Dropping worn equipment directly stays a two-step operation (take off, then drop), as the port's
  picker does not mix the slot windows.
- [x] `object2.cc:5435 inven_carry_okay` — Rust checks only pack length +
  `merge_target` (`input.rs:1054`); C++ also scans for a similar stack.
  Gold exclusion is implicit (Rust gold is a separate `FloorGold` entity).

### F. Floor / pickup / gold

- [x] `object2.cc:4834 drop_near` — `item::drop_near` now implements breakage
  + "The X disappears.", the 7×7 LOS score search with the
  23-objects-per-grid cap, the artifact "bounce to a useful grid" fallback,
  `items_similar` combination on the floor and "You feel something roll
  beneath your feet." (`drop_near` 4864-5115). It is wired into thrown
  objects (`modal::power_throw`); the game.rs `drop_loot`/`floor_hit`
  callers still place directly and need the same helper.  Reconciled (2026-09): the algorithm is
  public as `item::drop_near` (modal's wrapper delegates) and `item::drop_loot_ex` applies it to
  monster-death loot; only the game.rs `drop_loot`/`floor_hit` callers (game.rs:8402) still place
  directly, which needs map/occupied passed from game.rs (cross-file).
- [x] `object2.cc:6022 floor_carry` — `place_floor_item` combines similar
  stacks and enforces the >23 cap.
- [x] `object2.cc:4714 make_gold` — 18 treasure kinds (`OBJ_GOLD_LIST 480`,
  `i = ((randint(object_level+2)+2)/2)-1`, 1/60 extra boost, `coin_type`,
  `base + 8*randint(base)+randint(8)`, ×5 `no_selling`) are not ported.
  `place_gold` (`item.rs:2178-2187`) takes a bare amount;
  `scatter_objects` (2216-2218) and `monster_carried_treasure` (2285) invent
  amounts.  The TV_GOLD kind names ("copper", "silver", ...) and
  `absorb_gold`'s "You have found N gold pieces worth of %s." are lost
  (`object1.cc:5301-5337`; Rust `input.rs:1127-1135` prints "You collect
  N gold pieces.").  Creeping-coins `coin_type` is absent.
- [x] `object1.cc:5209 can_carry_heavy` — no encumbrance "Pick up X?" prompt;
  `always_pickup`/`carry_query_flag` options do not exist (grep in
  `bevy/src` is empty).  Walking over objects only prints messages
  (`input.rs:1127-1150`).
- [x] `object1.cc:5339 sense_floor` — stepping on a grid does not
  auto-ID/pseudo-ID the objects there (only the ammo quiver merge,
  `input.rs:1708-1755`, and gold collection happen).
- [x] `object1.cc:5233 object_pickup` / `object1.cc:5365 py_pickup_floor` —
  Rust `get_items` (`input.rs:1037-1124`) lacks the C++ auto-identify
  (`object_aware`+`object_known`, 5242-5244), picks whole stacks without the
  "Pick up X?" heavy check, and describes the pile with no
  `squeltch_grid`; the hypnos-skill check and 23-slot message exist.
- [x] `object2.cc:6083 pack_decay` / `object2.cc:6170 floor_decay` — Rust
  (`game.rs:8274-8323`) just deletes rotted corpses ("A corpse in your pack
  rots away.") and hatches eggs.  Missing: head→skull with
  `weight = wt/60 + rand_int(wt)/600`, corpse→skeleton when `RF_DROP_SKELETON`
  with `wt/4 + rand_int(wt)/40`, the named-skeleton artifact (`name1 = 201`)
  when the corpse was known, and the visible-only "You see X decompose."
  (`floor_decay` 6194-6203).  `pack_decay` in C++ can also transform in-pack
  corpses (6124-6163).  Reconciled (2026-09): `item::decayed_corpse` computes the head→skull and
  DROP_SKELETON corpse→skeleton replacements (count/monster/named-remains carried), but game.rs's
  decay tick (game.rs:12625-12667, cross-file) still removes the corpse instead of calling it; the
  per-instance weight re-roll and the visible-only "You see X decompose." prose are still absent.
  **Done (2026-09-16):** the game.rs decay tick now calls `item::decayed_corpse` (head→skull,
  DROP_SKELETON corpse→skeleton) with the object2.cc weight re-roll (`Item.weight`), prints
  "You feel X decompose." in the pack and "You see X decompose." on visible floor grids only; pack
  eggs hatch as MSTATUS_PET (IMPRESED hatchlings are promoted to companions when the LORE cap
  allows), floor eggs as enemies.

### G. Sentient objects (LEVELS)

- [x] `object1.cc:5476 gain_flag_group` / `object1.cc:5510 get_flag` /
  `object1.cc:5550 gain_flag_group_flag` — the whole "realm" power system is
  missing: `object_gain_level`'s +1 point + `magik(NEW_GROUP_CHANCE)`
  (5603-5610) and the 66+ branch that grants a realm then a random unused
  flag from it, with the "gains access to the X realm." / "gains a new power
  from the X realm." messages (5519-5584).  Rust's `object_gain_level`
  (`game.rs:5540-5583`) only does to_h/to_d/pval2/pval.
- [x] `object1.cc:683 calc_object_need_exp` / `object2.cc:1708 init_obj_exp` /
  `object2.cc:1718 object_prep` — Rust uses an ad-hoc formula
  (`5*(10*elevel*(elevel+1))/2`, `game.rs:5558`) instead of
  `player_exp[elevel-1]*5/2`; `object_prep` should initialise
  `elevel = k_ptr->level/10+1`, `exp = player_exp[elevel-1]`,
  `pval2 = 1`, `pval3 = 0` for LEVELS kinds (1759-1765) — `Item::base`
  (`item.rs:101-153`) leaves elevel=1/mon_exp=0 and never uses pval3.
  Also missing `object_prep`'s `pval2 = k_ptr->pval2` (1735) and the base
  `TR_CURSED` OR-in (1753-1757, see H below).

### H. Flag/flag-consumption issues

- [x] `object1.cc:542 object_flags` — C++ **replaces** the base kind flags
  with `a_info.flags` for artifacts (559) and mixes in set flags (563); Rust
  `item_flags` (`item.rs:307-318`) always unions base kind + instance + ego +
  artifact flags, so an artifact that does not list a base-kind flag still
  inherits it.  Set flags are applied in `Inventory::totals` (600-640) and
  `item_slays_with_set` (729-755) but not in the flag list used for display
  or `item_ignores`, so e.g. an IGNORE_* granted by a completed set does not
  protect the item.
- [x] `object1.cc:5865 artifact_p` — Rust uses `item.artifact != 0`
  (`item.rs:344`, and many call sites) which is false for TV_RANDART
  junkarts (`finalize_junkart` keeps `artifact=0`, `item.rs:830-854`) and
  for `TR_NORM_ART` base kinds such as Blood of Life.  Effects: junkarts/
  NORM_ART potions are not protected as artifacts from destruction
  (`item_ignores`, `item.rs:343-353`), and `drop_near`-style "artifacts don't
  disappear" rules cannot be applied.  (`ego_item_p`/`is_ego_p` are fine as
  long as no double egos exist.)
- [x] `object1.cc:5893 cursed_p` — C++ `cursed_p` checks `art_flags & TR_CURSED`,
  which `object2.cc:1753-1757` seeds from the base kind `F:CURSED`.  Rust
  never reads `o.flags` for CURSED in `Item::base`/`make_item`; only
  ego/artifact/random rolls set `item.cursed`.  Live cursed kinds that are
  missed: Ring of Teleportation (items.ron id 138, sval 4), Ring of Aggravate
  Monster (154, sval 1), Amulet of Teleportation (166, sval 1) — so these are
  removable at will and are priced as uncursed.
- [x] `AUTO_CURSE` (a_info 7 records + e_info 3 records) is parsed into
  `flags` but never consumed: C++ `dungeon.cc:2314-2320` re-curses the item
  1/15 per turn (`o_ptr->art_flags |= TR_CURSED`).  Rust has no equivalent
  (grep `AUTO_CURSE` in `bevy/src` = 0 hits).  `RECHARGED` is likewise never
  set (`spells2.cc:2333`), affecting device stacking only.  **Done (2026-09):** AUTO_CURSE is processed
  per turn and `recharge_device` now sets the RECHARGED flag on rods/wands/staves (success and
  failure), so `items_similar` stops merging recharged with fresh devices.
- [~] `object_flag_meta.cc` — no gameplay consumer is missing: ESP_* are
  consumed by prefix (`item.rs:575-576`), the pval mask is only a display
  helper, and the metadata name/negation table is only used for the C++
  knowledge/describe prose.  Rust prints raw flag names (observe window);
  no GAP.

### I. Equipment slots / body parts

- [x] `object1.cc:3119 get_slot` / `object1.cc:3152 wield_slot_ideal` —
  C++ builds `p_ptr->body_parts[]` from `R:E:`/`S:E:`/`C:E:`
  (`xtra1.cc:2021-2072`); the base data has `R:E:1:1:1:4:0:0` for DeathMold
  (4 ring slots, 0 head, 0 feet) and the generic `R:E:1:1:1:2:1:1`.
  Rust parses `RaceModDef.body_parts` (`data.rs:215`) but never consumes it
  (grep `.body_parts` outside the struct = 0); only mimicry's
  `CLASS_ARMS|CLASS_LEGS` extra slots exist (`mimic.rs:478-488`).  Also
  `get_slot`'s "all same-type slots full → return the base slot" fallback and
  multi-head extra neck slots are not modelled (rings are special-cased in
  `modal.rs:10917`).  Reconciled: `mimic::calc_body`/`slot_usable_body` consume R:E:/C:E:/S:E:
  (mimic.rs:614-699) and the transform paths enforce them; the wield/display call sites now use
  `mimic::slot_usable_body` too, so a DeathMold and the mimic extra limbs use their real slots at
  wield time (modal.rs:6271/12345).
  **Done (2026-09-16):** `wield_apply` now routes through `wield_slot_for` (modal.rs:13378),
  which returns the base slot only when `mimic::slot_usable_body` accepts it, falls back to a
  usable same-type extra slot (mimic limbs) otherwise, and refuses with "You have no body part
  to wear that." when the body has no part at all (C++ `get_slot`'s -1).  The ring overflow also
  requires a second usable finger.  Test: `wield_slot_checks_current_body_parts`
  (DeathMold head/feet refused, Human helm/boots/ring accepted).
  Multi-head extra neck slots do not exist in the ported races.

### J. Descriptions (behavioural, not just layout)

- [x] `object1.cc:752 object_desc_aux` details missing from `Item::label`
  (`item.rs:173-302`): plural `~` adds "es" after s/h (1294-1296) vs Rust's
  unconditional `format!("{} {}s")`; wand/staff charges are printed even
  when unknown and regardless of `mode` (1559-1564 prints at mode≥2 only);
  `TV_ROD` "(N Mana to cast)" and `TV_ROD_MAIN` "(timeout/pval2)"
  (1570-1581) vs Rust's `(charging)` for `TV_ROD`; pval prose ("to speed",
  "attack(s)", "% of critical hits", "to stealth", "to infravision",
  1590-1643) vs bare `(+N)`; activation "(charging)"/egg "(stopped)"
  (1647-1658); `MANA`/`LIFE` "(N%)" (1541-1549); LEVELS "(E:n, L:n)"
  (1422-1429); `(exploding)` (1448-1450); corpse/egg/hypnos monster names
  (1005-1038, 1142-1150); random book `[Spell]` (1404-1408); the
  mode-3 inscription assembly ("cursed", "N% off", join, 75-char clip,
  1668-1710); `TR_FULL_NAME` (1087-1091).  Rust's corpse/egg labels show
  only "corpse"/"Egg" (`init_corpse` stores `note`, `item.rs:3251-3261`, but
  `label` never uses it for TV_CORPSE/TV_EGG).  TV_HYPNOS label uses `note`
  while C++ stores the monster in `pval` (object1.cc:1030-1038,
  object2.cc:3148-3167); `make_item` never fills it.  Reconciled: still missing in
  `Item::label` (item.rs:187-330): the plural `~` "es" rule, corpse/egg monster names, pval prose,
  rod mana prose, LEVELS "(E:n, L:n)", `TR_FULL_NAME`, and the mode-3 inscription assembly
  (incl. "N% off", 75-char clip).
  **Done (2026-09-16):** `Item::label` now has the pval prose (speed/attacks/critical hits/stealth/
  infravision, HIDE_TYPE suppresses it), MANA/LIFE percentages, rod-main `(timeout/pval2)`, rod-tip
  `(N Mana to cast)`, FUEL_LITE `(with N turns of light)`, SPELL_CONTAIN `[Spell]`, ACTIVATE
  `(charging)`/egg `(stopped)`, the plural "es" rule, corpse/egg monster names and the mode-3
  inscription assembly (cursed + "N% off" + inscription joined with ", " and clipped to 75 chars).
  FULL_NAME is effectively a no-op (the port shows the full kind name once identified); the port
  keeps a single description mode rather than object_desc's 0-3 verbosity split (display only).
- [x] `object1.cc:1752 item_activation` — Rust activations use the raw data
  names (`modal.rs:5007-5030`, `observe_text` `modal.rs:5161-5169`).  Missing
  the two special cases: eggs ("stop or resume the egg development",
  1771-1774) and default horns ("aggravate monster every 100 turns",
  1776-1787).  The corresponding `cmd6.cc:4433-4474` egg timeout
  stop/resume (`timeout=-1`/`0`) and `cmd6.cc:4484-4494` horn
  "Your instrument emits a loud sound!" + `aggravate_monsters(1)` +
  `timeout=100` are absent (grep `EGG` in `modal.rs` = 0).  **Done (2026-09):** eggs are listed in `activatable`
  and stop/resume development (timeout/fuel), plain horns print "Your instrument emits a loud
  sound!", aggravate monsters and recharge 100; `observe_text` shows both default descriptions.
- [x] `object1.cc:2066 object_out_desc_where_found` / `object2.cc:4603
  place_object` — `Item` now has `found`/`found_aux1..4` (item.rs),
  `OBJ_FOUND_FLOOR`/`RUBBLE`/`MONSTER` are set by `scatter_objects`, the
  kill-wall rubble path and `init_corpse` (game.rs adds the dungeon/level aux for
  corpses), and `item::where_found_text` renders the full object1.cc:2977-3019
  prose in `modal::observe_text` (VAULT/SPECIAL callers in game.rs can set the
  remaining tags later); the old text follows:
  "You found it lying on the ground on level N of X / in a vault / in the
  remains of ..." prose (object1.cc:2977-3011) is absent.  C++
  `OBJ_FOUND_*` values are only set by `place_object`/`make_object` callers.  Reconciled:
  unchanged — `Item` (item.rs:30-99) still has no `found`/`found_aux1..4`, so the where-found
  prose is absent.
- [~] The rest of `object_out_desc` / `describe_device` / compare-weapons
  output is UI; Rust's `Modal::Observe`/`Modal::CompareWeapons` carry the
  essential numbers but omit device spell level/fail info (object1.cc:2021-2057)
  and full flag prose.  No gameplay gap beyond the bullets above.
- [~] `object1.cc:5209 mention_use`/`describe_use`/`index_to_label`/
  `get_item*`/`show_*`: UI windowing differences, intentionally not audited.

## UNCERTAIN

- `object2.cc:5496 inven_carry` — Rust's append order may be intentional;
  C++'s pre-reorder and `reorder_pack` affect only pack layout, but they
  also change the 23-slot overflow result (which item is dropped when
  overfull).  No gameplay evidence found either way.
- `object1.cc:5242 object_pickup` auto-identify: C++ fully identifies every
  object that is picked up (and `sense_floor` identifies everything stepped
  on), which makes the original's identification almost cosmetic.  Rust's
  `known`/`identified` model is closer to classic ToME; the DIVERGENCE is
  intentional-looking, but the *display* bug in section C is not.
- `object2.cc:3724 apply_magic` — Rust's flat good/great probabilities are
  clearly a deliberate rebalance; the per-class missing branches listed in
  section D are still factual.
- `object2.cc:1718 object_prep` — the Rust `pval2` field is overloaded
  (symbiote hp / rod mana / morphic oil shape); a LEVELS `pval2=1` init
  would collide with those uses, so the sentient-realm port needs a design
  decision.
- `object1.cc:242 object_easy_know` — Rust's `identified` default for
  non-wearables means EASY_KNOW is *effectively* true for them; whether the
  `TR_NORM_ART` guard is needed for any kept consumable was not verified.
- `object_filter.cc:27 IsArtifact` (name1>0) — only referenced by Theme
  Aule spells in base (`spells3.cc:3796,3858`), so no base consumer exists to
  verify against.

## DATA COMPARISON (`lib/edit/k_info.txt` vs `bevy/assets/data/items.ron`)

- Counts: k_info 596 records → converter `keep()` keeps exactly 596; RON has
  exactly those 596 ids (0 missing, 0 extra).  All k_info tvals present in
  KEEP_TVALS; no chest tval exists in this fork; food svals kept =
  {0..19, 32, 33, 35, 36, 37, 38, 39, 40, 41, 42}.
- Per-record flag loss: none (checked `F:` ∪ `f:` per record vs RON for
  k_info/a_info/e_info; lower-case `f:` obvious flags are dropped by the
  converter but every such flag also appears on an `F:` line and no per-record
  case was found where only `f:` had it).
- Device spells: all 44 distinct `SPELL=` names used by kept wands/staves/rods
  resolve to a `spells.ron` row (0 dangling).  All activation names —
  items.ron 21 + artifacts.ron 80 — are covered by
  `spell::artifact_activation` (`spell.rs:528-700`), 0 fizzle names left.
- Behaviour flags not consumed anywhere in Rust: `AUTO_CURSE` (live: 7
  a_info + 3 e_info) and `RECHARGED` (runtime-generated by recharging).
  `ATTR_MULTI` (7 k_info) is colour-only (UI).  `FULL_NAME`/`HIDE_TYPE`
  (31/19+112+16) are description modifiers whose effect is accidentally
  reproduced by Rust always printing the kind name/raw plusses.
- Ego `T:` min/max sval, `r:N:`/`r:F:` need/forbid flags, the `R:` flag
  rarity groups and `a:` activations are now parsed (see DONE).
- The k_info `A:` allocation rows are now parsed and drive `get_obj_num`
  (see A); the k_info `W:` second field is still stored as `rarity` but no
  longer used in generation.

## COVERAGE SUMMARY

- object1.cc: 75 defs — [x]=16, [>]=17, [ ]=5, [~]=37
- object2.cc: 75 defs — [x]=15, [>]=34, [ ]=7, [~]=19
- object_filter.cc: 11 defs — [x]=10, [>]=0, [ ]=0, [~]=1
- object_flag_meta.cc: 0 defs
- Total 161: [x]=41, [>]=51, [ ]=12, [~]=57

## DONE THIS SESSION

Files: `bevy/src/item.rs`, `bevy/src/data.rs`, `bevy/tools/convert_data.py`
(+ assets regenerated). `cargo test` at session end: 166 passed, 0 failed
(run 3x).

- **A. Allocation / kind selection**
  - `convert_data.py`: k_info `A:<level>[/<chance>]` -> `ObjectDef.alloc`,
    `T:<btval>:<bsval>` -> `btval`/`bsval`.
  - `item.rs::get_obj_num`/`gen_object_kind_themed` now implement
    `init_alloc` + `get_obj_num` faithfully: 100/chance weights, `GREAT_OBJ`
    1/20 level boost, 60%/10% best-of-2/3 by allocation level, plus
    `kind_is_theme`, `kind_is_legal` (SPECIAL_GENE, NORM_ART uniqueness via
    `norm_art_key`, generic corpse svals, HYPNOS, `SV_RING_SPECIAL`),
    `kind_is_good` (full tval table incl. SV_ROD_SILVER=100,
    SV_BOOK_MAX_GOOD=49, SV_RING_SPEED=31, the seven good amulets) and
    `kind_is_artifactable` (kind_is_good + ra_info kind filters).
  - `make_object_ex` uses the good restriction and sets the created key for
    NORM_ART kinds; `make_item` gives NORM_ART kinds the "already created"
    T: fallback and skips magic; base `F:CURSED` is set in `Item::base`.
  - Monster drop theme `r_info O:` parsed into `MonsterDef.objs` and wired
    into `monster_carried_treasure` via `make_object_themed`.
  - `kind_is_artifactable` is exported but the FATE_FIND_A call site in
    `game.rs` is out of the ownership scope.
- **B. Object value**
  - `flag_cost` (full STR/CHA stat rates, slays/brands/kills, resists,
    immunities, SH_*, ignores, ESP per flag, curses, LEVELS, ACTIVATE name
    table) + `object_value_real` per-tval pricing (wand/staff spell level ×
    pval3 average / 6 + charges, rod tip, sval-255 books, egg level,
    SPELL_CONTAIN, pval credits, bow/armour/weapon/ammo extra dice,
    exploding ×14) + `object_value` discount.
  - `Item.discount` added; `Item::cost` now calls `object_value`.
  - `absorb_item` keeps the target's discount (largest-discount blend is
    still not implemented).
- **C. Identification / flavours**
  - `Item::base` now starts flavoured kinds (potions/scrolls/wands/staves/
    rods/rings/amulets/mushrooms) unidentified; EASY_KNOW kinds known on
    sight; unflavoured kinds aware. `label` shows the base type names
    ("Potion", "Scroll", "Wand", "Staff", "Ring", ...) for unknown kinds
    and the random-book spell name. `flavor_color` applies the per-seed
    shuffle unless the kind carries EASY_KNOW.
- **D. Ego / artifact generation**
  - `convert_data.py`: e_info `T:` min/max sval, `R:` rarity groups
    (split real flags vs ETR_* generation flags), `r:N:`/`r:F:`, `a:`
    (also exposed as `activate:` powers for the spell-side activation
    mapping), correct `W:` rarity1/rarity2; re_info `S:1_IN_n` ->
    `spell_freq`, `T:MF_ALL` -> `remove_spells`, `W:` weight applied.
  - `apply_ego`: tval/sval filters, good (cost>0) vs bad (cost==0) egos,
    out-of-depth `rand_int(level-depth)` and `rand_int(mrarity ± luck) >
    rarity` rolls, R: groups with `magik(chance)`, `add_random_ego_flag`
    for every live ETR_* power (PVAL_M1/2/3/5, AC/TH/TD_M*, DAM_SIZE,
    SUSTAIN/OLD_RESIST/ABILITY/R_*/R_IMMUNITY/...), `randint(max)` C:
    plusses, SPELL_CONTAIN `pval2 = -1`.
  - `apply_artifact`: list-order candidates with out-of-depth and
    `rand_int(rarity ± luck)` rolls (weighted `10/rarity` removed);
    `make_artifact_special` rolled the same rarity formula.
  - `roll_stick`: device charges now `base + randint(die)` (the old
    off-by-one); random book sval 255 rolls a Magic (75%)/Spirituality
    spell; FUEL_LITE lites roll `randint(fuel)`; exploding missiles roll
    `pval2` + display "(exploding)" (on-hit projection still needs
    `input.rs`).
- **E/F. Inventory / floor / gold**
  - `items_similar` implements `object_similar` for books, totems,
    potions/scrolls/food, wands/staves (spell/pval3/RECHARGED), rods,
    weapons/armour/rings/amulets/lites/missiles (known status, plusses,
    pval, dice, timeout, curses, instance flags) + inscription rule;
    `absorb_item` blends count/known/inscription/charges.
  - `place_floor_item` now merges similar stacks and enforces the
    23-object cap (`floor_carry`); the 7x7 LOS scatter of `drop_near`,
    breakage and the "roll beneath your feet" message still need map
    access in the locked callers.
  - `make_gold` picks one of the 18 treasure kinds with the original
    `((randint(level+2)+2)/2)-1` + 1/20 boost + `base + 8*randint(base) +
    randint(8)`; `scatter_objects` and monster drops use it and carry the
    coin name on `FloorGold` (the pickup message in `input.rs` is out of
    scope).
  - `inven_carry_okay` / `inven_item_optimize` helpers added.
- **Damage fidelity**
  - `hates_element` implements the full `hates_acid/elec/fire/cold`
    tables; `inven_damage` destroys hated items unit-by-unit at the
    original low percentage, protects artifacts/NORM_ART/junkarts through
    `is_artifact`, applies the armor `minus_ac` acid branch and scales
    wand charges; `floor_damage` uses the `project_o` do_kill tables
    (always destroys, artifacts/IGNORE_* survive, CHAOS respects
    RES_CHAOS).
- **Flags / sentient objects**
  - `item_flags` now replaces base-kind flags with a_info flags for real
    artifacts (object1.cc object_flags).
  - `Item::base` seeds `elevel = level/10 + 1` for LEVELS kinds; the
    player_exp-based `mon_exp` curve is unchanged (00-core table gap).
  - `armour_magic` (a_m_aux_2): elven cloak `pval = randint(4)`, dragon
    shield/helm `dragon_resist`; mage staffs gained
    SPELL_CONTAIN|WIELD_CAST; exploding missiles label "(exploding)";
  `jewelry_magic` serpent curses reverse only pval and Resistance/Lordly
  resists use the specific-id rolls; `absorb_item` keeps the largest
  discount.
- **Tests added**: allocation/good/NORM_ART, make_gold table+ranges,
  flag_cost/object_value, items_similar; updated
  `gen_object_respects_depth` (A: rows instead of W: level), wand/staff
  charge ranges (now 8..=17 / 8..=14), `artifacts_are_unique_per_game`
  (more rolls for the depth/rarity gates) and the fire-breath test (loop
  until burned).

### Left undone this session (with reasons)

- `drop_near` LOS scatter / breakage / "roll beneath your feet" and the
  vault/`found` bookkeeping need `map`/location plumbing in callers outside
  the owned files.
- `py_pickup`/`sense_floor` auto-identify, heavy "Pick up X?" prompt,
  gold pickup prose: `input.rs` is outside the ownership.
- `combine_pack`/`reorder_pack` and the inven_carry pre-reorder: UI/order
  only; overflow semantics unchanged.
- Sentient `gain_flag_group`/realm powers (`tables.cc flags_groups`) and
  `player_exp`-based object exp: separate systems outside object scope.
- `AUTO_CURSE` runtime re-cursing: needs the per-turn hook in `game.rs`.
- Dungeon-level `O:` theme wiring: `scatter_objects`/`make_object` callers
  do not carry the dungeon id; `kind_is_theme` is ready for it.
- `a_m_aux_4` remaining special cases (horn of Dragonkind pval2, corpse/
  egg/hypnos monster rolls, RANDART junkart cost) and `name2b` double egos.
  (`a_m_aux_2` elven cloak pval + dragon shield/helm resist and
  `a_m_aux_3` serpent/resistance/lordly paths are now implemented.)

## DONE THIS SESSION (base-audit pass 2)

Files: `bevy/src/item.rs`, `bevy/src/modal.rs`, `bevy/src/town.rs`,
`bevy/src/data.rs` (no data change needed). `cargo check` clean; `cargo test`
176 passed / 0 failed, run 3x at session end.

- `project_o`/`floor_damage_events`: full per-element object destruction
  (exact plural messages, IGNORE_*/artifacts, CHAOS RES_CHAOS, holy/hell
  cursed-only, corpse shard bursts), `potion_smash_effect` table and
  `drop_near` (see `reports/02-spells.md` for details).
- `inven_damage_ex`: damage-driven perc, artifact skip, per-unit rolls, exact
  message family, wand charge scaling, smashed-potion svals; `minus_ac`
  six-slot pick. `Item.junkart` costs rolled and priced (`object_value_real`).
- Randarts: `curse_artifact` randint(4) penalties, `random_resistance` 41-case
  table + bias, "of '<player>'" default, SPELL_CONTAIN pval2=-1.
- Stores (`town.rs`): `mass_roll`, `mass_produce` (piles + discounts),
  `price_item` (STF_ALL_ITEM flag, no_selling), `purchase_analyze`,
  `store_object_similar`/`absorb`, `store_check_num`, `black_market_crap`,
  `store_delete`, `kind_is_storeok`, `home_carry` sorted insert,
  `return_level`, `store_maint` prune/refill, `store_open` timed bans.
- `modal.rs`: `StoreHaggle` confirm modal (purchase_haggle/sell_haggle),
  store prune/refill on steal, `ExamineShop` item descriptions, `drop_near`
  for throws, OLD_* monster projections, `gf_terrain` full table.
- Tests added: `potion_smash_table_matches_spells1`, `breakage_chances_match_cmd2`,
  `inven_damage_destroys_and_reports_smashed_potions`,
  `curse_equipment_heavy_and_dg_marks_artifacts`,
  `mass_produce_piles_and_discounts`, `store_stacking_and_deletion`,
  `price_item_honours_flags_and_no_selling`, `purchase_analyze_reacts_to_prices`;
  updated `randarts_are_imbued` for the faithful "of 'Frodo'" default.

### NEEDS OTHER FILE

- [x] `game.rs`: pass projection damage into `inven_damage_ex` and consume its
  returned smashes; use `floor_damage_events` + `FloorBoom` in `floor_hit`;
  call `curse_equipment_ex(..., heavy, ...)` for CAUSE/HAND_DOOM; wire
  `drop_near` into `drop_loot`. **Wired this session** except `drop_loot`:
  `item::drop_near` still does not exist (only the private `modal::drop_near`
  covers throws), so `item::drop_loot` in item.rs still places directly and
  the game.rs monster-death loot path uses `place_floor_item`.

## WIRED THIS SESSION (game.rs/map.rs/save.rs ownership)

- `game::apply_gf_ex` + `game::apply_gf_booms`: monster breaths/balls/bolts
  now smash inventory potions with the damage-derived `inven_damage_ex`
  percentage and project the results (`potion_smash_effect`) on the player,
  the monsters and the floor; `floor_hit` passes the projection damage and
  chains `item::FloorBoom` corpse explosions / potion shatters.
- `game::curse_equipment_ex` at the CAUSE_1/2/3 and HAND_DOOM call sites.
- `item::make_object_themed` now receives the dungeon `O:` theme at the
  game.rs call sites: `populate_level` vault objects, spec/quest random
  object markers (newly spawned, see 05-map), and the q_rand princess
  reward now uses `make_object_ex(..., great = true)` (do_get_new_obj).
- FATE_FIND_A randarts (game.rs) now allocate through
  `item::kind_is_artifactable` at `max_dlv + randint(10)` instead of the
  unrestricted `gen_object_kind(depth + 5)`.
- `drop_loot` / `drop_near`: **not wired** — `item::drop_near` was not added
  by the item.rs session; `modal::drop_near` remains private.


## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- item.rs `apply_equip_flag`/`totals_for`: `TR_STEALTH` adds `pval` (was `pval.max(1)`),
  `TR_TUNNEL` adds `pval * 20` (was raw pval; `input.rs::skill_dig` must drop its own `* 20`),
  `TR_INVIS` accumulates `invis_power += pval * 10` and only sets `invis` for a positive pval,
  `TR_BLESSED` is no longer treated as a damage slay (the port's ×2-vs-EVIL is removed), base luck
  comes from `ps.luck_base`, and a holy aura adds `hold_life` + 5 luck. `item::teleport` also skips
  `Map.icky`/permanent grids.
- The `### NEEDS OTHER FILE` items above (game.rs `inven_damage_ex`, `floor_damage_events`,
  `curse_equipment_ex` call sites, `drop_near` in `drop_loot`) are all in game.rs and remain.


## DONE THIS SESSION (final gaps pass)

Files: `bevy/src/input.rs`, `bevy/src/game.rs`, `bevy/src/item.rs`.
`cargo test`: 200 passed / 0 failed.

- `object1.cc:5339 sense_floor`: `input.rs::sense_floor` runs on every
  `cell_arrival` (and again in `pickup_floor`), making the pile's kinds
  aware (`Inventory::learn`) and identifying the instances before the
  automatizer; `object_pickup`'s awareness/known calls are covered by the
  same pass.  The gold pickup prose is now the original "You have found
  N gold pieces worth of <coin>.".
- `object1.cc:5209 can_carry_heavy`: `game::can_carry_heavy` computes the
  old/new encumbrance tier from `weight_limit`/`calc_total_weight`;
  `pickup_floor` skips items that would raise the tier (the original's
  "Pick up X?" prompt needs a modal.rs `get_check`, so the port declines
  with a message instead).
- `object1.cc:5476-5584 gain_flag_group` / `get_flag` /
  `gain_flag_group_flag`: the full `tables.cc:2303 flags_groups()` table
  (`game::FLAGS_GROUPS`, 12 realms with prices and member flags) is
  ported; the object level-up path now implements `object_gain_level`'s
  exact branches (33/66 rolls, NEW_GROUP_CHANCE 40, realm point spending
  in `pval2`/`pval3`, "gains access to the X realm." / "gains a new power
  from the X realm."), and LEVELS artifacts initialise `elevel`/`pval2`
  in `apply_artifact`/`imbue_artifact`/`specific_artifact_item`
  (object2.cc:3907 `init_obj_exp`).

## DONE THIS SESSION (modal/item/input cross-file residuals, 2026-09-16)

Files: `bevy/src/item.rs`, `bevy/src/modal.rs`, `bevy/src/input.rs`.
`cargo test`: 209 passed / 0 failed.

- `object2.cc:5123 acquirement`: `great=true`, `randint(2)+1` for the starred scroll
  and the current object level (the +10 good base lives in `make_object`).
- `object2.cc:2286-2327` double ego: `Item.ego2`, `apply_ego_effects` split and the
  7%+luck second roll with same-ego/prefix-suffix constraints; `item_flags`, `Item::label`,
  `items_similar` and `create_artifact` consume it.
- `object2.cc:5911 reorder_pack`: ported with the exact sort key and wired after
  `combine_pack` in input.rs; `items_similar` also compares `ego2`.
- `object2.cc:3099-3167 a_m_aux_4`: random corpse/egg/hypnos monster rolls
  (`get_mon_num_from`, HAS_EGG filter, egg weight/pval, hypnos elevel/pval3).
- `object2.cc:4484/4603 found bookkeeping`: `Item.found/found_aux1..4` + `OBJ_FOUND_*`
  constants; `scatter_objects` sets FLOOR, the kill-wall rubble sets RUBBLE and
  `init_corpse` sets MONSTER/aux1 (game.rs sets note/pval3 today; aux2-4 cross-file).
- `object2.cc:6083/6170 decay`: `item::decayed_corpse` returns the head→skull /
  DROP_SKELETON skeleton replacement; game.rs must call it from its decay tick.
- `object2.cc:4834 drop_near`: public `item::drop_near` + `drop_loot_ex`; the private
  `modal::drop_near` now delegates, so thrown/dropped objects and loot use one algorithm.
- `object1.cc:3119 get_slot`: wield/display use `mimic::slot_usable_body` (race body
  parts + mimic limbs).

## DONE THIS SESSION (item/game/data object fidelity, 2026-09-16)

Files: `bevy/src/item.rs`, `bevy/src/game.rs`, `bevy/src/data.rs`,
`bevy/src/save.rs` (pet mirror), `bevy/tools/convert_data.py` + assets regenerated.
`cargo test`: 216 passed / 0 failed (3 runs); default autoplay screenshot smoke clean.

- **apply_magic**: new `item::apply_magic` (f1/f2 power curve, `force_power`,
  1/4 artifact rolls, NORM_ART/One-Ring gates) and a_m_aux_1/2/3/4 split out of the old
  flat-probability `make_item`; `make_ego_item` now runs exactly at power > 1 / < -1.
- **make_object/place_object**: dungeon theme in `scatter_objects`, `found`/`found_aux`
  for FLOOR/VAULT/SPECIAL/RUBBLE, `item::rating_of` + `Map.rating`/`good_item`/`feeling`
  at scatter/vault/spec/quest placements, `damroll(6,7)` ammo stacks, no-artifact towns.
- **make_artifact/weights**: `ArtifactDef.weight` (a_info W: 3) + `Item.weight` copied by
  `apply_artifact`/`imbue_artifact`/`specific_artifact_item`; `item::item_weight` used by
  the game.rs weight consumers.
- **a_m_aux_4**: Dragonkind horn `pval2`, egg fallback r_info 940, NEVER_MOVE hypnos
  fallback, egg instance weight.
- **decay**: game.rs tick calls `item::decayed_corpse`, prints "You feel/see X decompose.",
  visible-only floor prose, pack eggs MSTATUS_PET vs floor eggs enemies, IMPRESED
  companion promotion, weight re-roll.
- **labels**: pval prose, MANA/LIFE, rod main/tip, FUEL_LITE, SPELL_CONTAIN, activation
  charging/stopped, mode-3 inscription assembly (incl. "N% off" + 75-char clip).
- **Monster.pet**: new serde-default field seeded by `spawn_flagged_held`/`spawn_companion`/
  `spawn_loyal_companion`; consumed by the modal.rs session (do_cmd_companion et al).
- **free act**: `game::sanity_blast` uses `item::has_free_act`; no mimic.rs reader remains.
- Tests added: apply_magic power curve, label prose/inscription, rating_of/artifact weight,
  corpse decay, theme restriction.

## FINAL RECONCILIATION

Second reconciliation pass 2026-09 (input/modal/item session). Flipped to `[x]` this
pass: `a_m_aux_1` (the cursed/enchant plus rolls now use
`randint(5) + m_bonus(5, level)` for weapons and armour), the `AUTO_CURSE` bullet's
last gap (`recharge_device` now sets the RECHARGED flag on rods/wands/staves, success
and failure, so object_similar stops merging recharged with fresh devices) and
`item_activation` (eggs are listed and stop/resume development via `timeout`/`fuel`;
plain horns aggravate monsters every 100 turns). The prior 10 flips stand.

Fourth reconciliation pass 2026-09-16 (item/game/data session).  Flipped to `[x]` this
pass: `init_match_theme`/`kind_is_theme` (scatter_objects now passes the dungeon theme),
`make_object`/`place_object` (`found`/`found_aux` + `rating_of`/`good_item` wired at
generation), the ego pipeline (`apply_ego` driven from aux), `apply_magic` (full f1/f2
power curve, `force_power`, the 1/4 artifact rolls), `make_artifact` (the a_info weight
copy through the new `Item.weight`/`ArtifactDef.weight` + converter W: field 3),
`a_m_aux_4` (Dragonkind horn pval2), `pack_decay`/`floor_decay` (game.rs tick now calls
`decayed_corpse`, weight re-roll, "You feel/see X decompose.", pack pets vs floor enemies,
IMPRESED companion promotion) and `object_desc_aux` (pval/rod/charge/MANA-LIFE prose,
activate charging/stopped, mode-3 inscription assembly).  The modal.rs session flipped
`inven_takeoff`/`inven_drop` (per-slot verbs, symbiote gentle drop, pack overflow) and wired
`drop_loot_ex` into monster death from this session.  Prior flips stand.

Still `[>]`:

- Allocation-cache / `invprob` luck term / store `apply_magic(okay=false)` (town.rs) and the
  `input.rs`/`modal.rs` `Item.weight` readers are non-blocking cross-file residuals noted on
  the individual bullets (they do not affect generation outcomes beyond the RNG stream).
  The last `[>]` `object1.cc:3119 get_slot` bullet closed this session via `wield_slot_for`
  (modal.rs:13378).

Still `[~]` (unchanged, reason as in the bullet text):

- `object_flag_meta.cc` — no gameplay consumer missing (display metadata only).
- `object_out_desc`/`describe_device`/compare-weapons prose — UI only.
- `mention_use`/`describe_use`/`index_to_label`/`get_item*`/`show_*` — UI windowing not
  audited.
- `object1.cc:327 flavor_init` — cosmetic shuffle order/glyph derivation (frontend).

No `[ ]` bullets remain in this report.
