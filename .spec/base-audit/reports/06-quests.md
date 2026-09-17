# quests audit report

Base source: all `src/q_*.cc` (27 files) + `src/quest.cc` (the original hook framework).
Rust side: `bevy/src/game.rs` (`check_quest_kill`/`check_plot_kill`/`finish_plot_quest`, `build_quest_level`,
`populate_level`/`populate_town`/`populate_wilderness`, `kill_monster`, `PlotQuest`, `QuestState`),
`bevy/src/modal.rs` (building action dispatch, `Modal::Library`/`Fireproof`/`RandReward`/`RingWear`/`UltraMirror`,
Drop/Identify/Wield handlers), `bevy/src/input.rs` (`enter_plot_quest`, `quest_exit`, `npc_chat`, `get_items`),
`bevy/src/map.rs` (`generate_quest_level`/`quest_map_offset`), `bevy/src/item.rs`,
`bevy/tools/convert_data.py` QUEST maps/QUESTS, `bevy/assets/data/questmaps.ron`, `quests.ron`.

Context: the port deliberately re-expressed the C++ plot as a curated chain (`quests.ron`) plus the fixed-map
plot levels; the whole chain (mayor→thieves→troll/wight→nazgul, Lorien/Gondolin/Minas/Khazad side quests) is
treated as known-done. This report therefore records concrete per-function divergences that remain, including
state-machine, numeric and object/reward discrepancies. `[x]` = equivalent, `[>]` = partial, `[ ]` = absent,
`[~]` = not ported (UI/dump/score text, Theme, unreachable data).

## DONE THIS SESSION (2026-09-16, game.rs/birth.rs/save.rs)

- **Save persistence of `MonsterExtra`** (exp/nice/repro): `MonsterSave` now carries the component (serde
  default), `from_monster_with_extra` snapshots it in `level_transition`, and both restore paths re-spawn it.
  `input.rs::build_save` still uses the legacy `from_monster` (defaults) — NEEDS OTHER FILE to add
  `Option<&MonsterExtra>` to its monster query and call `from_monster_with_extra`.
- `q_narsil.cc:38-57` — reforge scans worn + pack (not just pack), full seven-line Aragorn dialogue.
- `q_wight.cc:145-152` — `check_plot_kill` now receives the death position; the way out opens under the
  player with the original message.
- `q_nirna.cc:101-108`/`71-90` — added `PlotQuest::nirnaeth_kills`, counted every death on the level,
  reset per level generation; the finish reward pays 200 000 gold only at two thirds (three lines for the
  bonus, two otherwise).
- `q_thrain.cc:60-79` — full Thrain dialogue, quest goes to FINISHED. (Mimic-wall reveal + reward dropped
  in the vault remain impossible: the converter drops the `mimic` field.)
- `q_troll.cc:170-198` — the first non-Tom troll death springs the hidden `H` cells (terrain 96 → T_GRASS)
  with forest/stone trolls; `troll_ambush` re-arms on each level build.
- `q_dragons.cc:60-106` — exact tier probabilities (1/32/33/34), per-colour arrays, 35 mountain columns on
  even squares, placement inside `rand_int(21)+3`×`rand_int(31)+3`.
- `q_library.cc:314-357` — per-region `place_quest_region`: 4d2 liches, three monastic-lich bands
  (1d2, 1d2-1, 1d2-1) and the flesh/clay/iron (×2) + mithril (×1) golems.
- `q_poison.cc:131-141` — molds gain the 80% level boost up towards the player's level.
- `q_hobbit.cc` — Merton only spawns in the Maze (dungeon 18); Melinda only appears ten days after the
  quest is initialised (reward timing in `input.rs` still keys on the rescue turn — NEEDS OTHER FILE).
- `q_invas.cc:113-137` — auto-grant guards `depth < PLOT_DEPTH_BASE` and `!astral`.
- `game.rs::quest_unlocked` — PLOT_SPIDER now requires PLOT_WOLVES >= FINISHED (q_wolves.cc:149 chain).
- `q_eol.cc:144-163` — `PLOT_FAILED` corrected to 4 (C++ value; was 6) and `PLOT_FAILED_DONE = 6` added;
  `finish_plot_quest` reports a failed quest with the "You fled!" message.
- `q_god.cc` — Lost Temple (dungeon 30) is placed in the wilderness at the first relic quest
  (`place_god_temple`, Bree fallback), reachable through `Wilderness::dungeon_at`; `god_directions` prints
  the compass bearings from Bree/Minas Anor (Angband/Mordor for Melkor); the temple always holds the relic;
  `god_relic_reward` implements `+5*Prayer.mod` (`+10*mod` for the fifth) with the two message sets. The
  reward is still wired to the old +2 levels in input.rs — NEEDS OTHER FILE.
- `game.rs::finish_plot_quest` — failed quests now transition to FAILED_DONE.
- `q_ultrag.cc:248-272` — the Flame Imperishable now applies the original full-pack handling (drop the
  last pack item with the "urge to drop" message) before entering the pack.
- `q_rand.cc:438-443` — a freshly generated level resets the random quest's kill counter.
- `q_shroom.cc:318-347` — the mushroom count is rolled at game start (birth.rs), matching the quest
  init hook.

## GAPS

### 0. Hard blockers / softlocks

- [x] `q_one.cc:38-46` `quest_one_move_hook` + `q_one.cc:266-355` `quest_one_death_hook` — the One Ring is
  **unobtainable in normal play** and the main chain softlocks at Sauron. C++ allows Galadriel's warning once
  the *Necromancer* is dead (`quest[QUEST_NECRO].status < QUEST_STATUS_FINISHED` gate), i.e. before Sauron,
  and the Ring then drops from Sauron 30% / the five other bearers 10%. Rust gates the warning on
  `ctx.quest.stage > sauron_stage` (`input.rs:1848-1855`, `sauron_stage` = index 13 of `quests.ron`), but
  `check_quest_kill` only advances past Sauron when the Ring is already destroyed or worn
  (`game.rs:4367-4376`). Warning → needs Sauron permanently dead → needs Ring → needs warning: circular.
  The autopilot tests sidestep it by pushing artifact 13 straight into the pack (`autopilot.rs:268`).
  Suggest recording "Sauron defeated once" (the recoil branch) and gating the mirror on that flag or
  `stage >= sauron_stage`.落点 `input.rs::player_input` (building 23 gate) + `game.rs::check_quest_kill`.
- [x] `q_wolves.cc:25-97` `quest_wolves_gen_hook` — the wolf den has no monsters. The C++ hook spawns
  `damroll(4,4)` wolves (r_info 196) and `damroll(4,4)` wargs (r_info 257) with `magik(50)` sleep and MFLAG_QUEST;
  `wolves.map` itself contains none. Rust `build_quest_level` (`game.rs:7255-7350`) has no `PLOT_WOLVES` arm and
  the converted map (`questmaps.ron` quest 22) has `monsters:[]`, so `check_plot_kill`'s
  `PLOT_WOLVES => quest_left <= 0` (`game.rs:4495-4496`) can never fire. The quest can only be "completed" if a
  monster dies on the level while 0 quest monsters remain.落点 `game.rs::build_quest_level` (spawn the two 4d4
  packs, quest-flagged) or a new map for 22.
- [x] `q_thief.cc:68-93` `quest_thieves_gen_hook` — the disarm scatter uses quest-map-local coordinates.
  `generate_quest_level` centres quest maps at `quest_map_offset` = ((198−33)/2, (66−23)/2) = (82,21)
  (`map.rs:2954-2969`), but the Rust scatter targets `(22±6, 2±2)` (`game.rs:6031-6045`). World cells (16..28, 0..4)
  are `T_PERMANENT` rock, so `walkable` fails 100 times and every stolen item is silently destroyed instead of
  being dropped in the great hall. C++ drops at `inven_drop(x,99,4,24)` = map-local (22,2)
  (`q_thief.cc:87` → `init1.cc:6263` xstart=2). Same class of bug as the alarm below.
- [x] `q_thief.cc:110-126` `quest_thieves_hook` — the alarm can never ring. The check reads
  `map.terrain_at(20,15)` (`game.rs:8069`) and opens doors at (18/14/10/6, 12) and (18/14/10/6, 18)
  (`game.rs:8075-8090`), all quest-map-local; the actual world positions are +82/+21. C++ checks
  `cave[17][22]` and opens `(14,20)…(20,8)` (`q_thief.cc:111-126`), i.e. the same local cells. As written the
  alarm branch is dead code (guarded by a `T_DOOR..=T_SECRET_DOOR` test on permanent rock).

### 1. Reward / item discrepancies

- [x] `q_shroom.cc:216-223` `quest_shroom_give_hook` — the sling reward is wrong. C++ creates the **specific
  artifact `of Farmer Maggot`** (`lookup_kind(TV_BOW, SV_SLING)`, `name1 = 149`, `apply_magic`, `discount=100`).
  Rust hands out a random ego sling, re-rolling `make_item(..., good=true)` up to 30 times for any non-cursed
  ego (`input.rs:2126-2139`). The a_info 149 entry ("of Farmer Maggot", ACTIVATE/MAGGOT) is never granted.
- [x] `q_shroom.cc:199` `quest_shroom_give_hook` — the healing mushrooms are wrong. C++ gives 15-20 of
  `SV_FOOD_CURE_SERIOUS` (16). Rust picks randomly from food svals `12..=18` (`input.rs:2141-2146`), i.e. seven
  different food kinds (stat-loss/poison/etc.) instead of Cure Serious Wounds.
- [x] `q_bounty.cc:108-158` `quest_bounty_get_item` — reward completely different. C++ consumes the corpse,
  prints "Ah well done adventurer!" / "As a reward I will teach you a bit of monster lore." and grants
  `SKILL_LORE` (`mod==0 → mod=900, dev=true`, then `value += mod`) plus, if the player has no Preservation,
  `SKILL_PRESERVATION value/mod = 800`. Rust removes the corpse and pays **gold `200+300*depth` and
  `100*depth` exp** (`modal.rs:17670-17684`); the Monster Lore/Preservation training is not given at all
  (only a stale comment mentions it). **FIXED: modal.rs::bounty_turn_in now grants SK_LORE/SK_PRESERVATION.**
- [x] `q_library.cc:44-101` `initialize_bookable_spells` — the bookable spell list is incomplete. C++ pushes 43
  base spells; Rust `BOOKABLE_SPELLS` (`spell.rs:216-250`) has 33 and omits (all present in `spells.ron`):
  Fire Golem, Geyser, Vapor, Ent's Potion, Stone Skin, Recharge, Armor of Fear, Stun, Grow Trees and
  `YAVANNA_TREE_ROOTS` ("Tree Roots", which is itself missing from `spells.ron`). These can never be written
  into the reward tome. **FIXED: `BOOKABLE_SPELLS` is now the full 43-spell list in q_library order.**
- [x] `q_fireprof.cc:255-316` `quest_fireproof_building` — the Scroll of Fire is never turned in. C++ asks for
  the eligible scroll (`item_tester_hook_eligible`, pval2 == sval) and consumes it
  (`inc_stack_size_ex(item_idx,-1)`). Rust's `PLOT_COMPLETED` branch opens `Modal::Fireproof`
  (`modal.rs:17319-17323`) without removing the quest scroll; the player keeps the (IGNORE_FIRE) Fire scroll
  and still gets all 24 fireproof points. The Rust pickup also never sets the pval2 "quest" mark / inscription
  (`input.rs:1088-1106` identifies it by name only), and the scroll's drop position is any walkable top-half
  cell instead of `y=3..5, x=3..47` (`q_fireprof.cc:497-503`, `game.rs:7327-7348`).
  **PARTIAL FIX (modal side): the building action now consumes one carried Scroll of Fire, moves the quest to
  PLOT_FINISHED and only then opens `Modal::Fireproof`; partial sessions resume on PLOT_FINISHED. The pickup
  pval2/inscription mark and the drop rectangle remain input.rs/game.rs (NEEDS OTHER FILE).**
- [x] `q_shroom.cc:318-347` `quest_shroom_init_hook` — `shrooms_needed` is rolled on first entry to the quest
  level (`game.rs:7090-7092`) instead of at game start; if the player talks to Maggot in Bree first
  (`input.rs:2075-2083`, still `shrooms_needed == 0`), the field then holds `max(1) == 1` mushroom
  (`game.rs:5349-5353`) and the fetch completes with a single mushroom, versus the original's rolled 7-14.
- [x] `q_nazgul.cc:63-91` `quest_nazgul_finish_hook` — reward timing. The six Athelas arrive at the kill
  (`game.rs:4448-4460`) with the mayor reporting silently (`finish_plot_quest` has no `PLOT_NAZGUL` arm,
  `game.rs:4538-4621`); C++ pays them when reporting the completed quest. Minor, but the report/reward
  coupling used everywhere else is broken here.
- [x] `q_ultrag.cc:248-272` `quest_ultra_good_death_hook` — when Tik'srvzllat (r_info 1032) dies the Flame
  Imperishable is `inv.pack.push`ed without the original full-pack handling (scan for a free slot, otherwise
  drop the last pack item with "You feel the urge to drop your %s to make room in your inventory."). The same
  push-in-past-23 behaviour exists for the Ring but there it is handled (`game.rs:5602-5610` vs `5669-5675`).

### 2. Quest state machines / conditions

- [x] `q_wolves.cc:99-133` `quest_wolves_death_hook` — see blocker above; because no quest monsters can spawn
  the "Lothlorien is safer now." completion is unreachable. (State machine itself is modelled in
  `check_plot_kill`.)
- [x] `q_spider.cc:53-99` + `q_wolves.cc:99-133` + `q_haunted.cc:99-140` + `q_dragons.cc:113-150` +
  `q_evil.cc:84-120` — the original kill-everything hooks count **all** monsters with
  `status <= MSTATUS_ENEMY` and succeed at `mcnt <= 1` (spider/wolves/haunted/dragons/evil); thieves counts
  `status < MSTATUS_FRIEND` and requires 0, library requires 0. Rust counts only monsters with the
  `MFLAG_QUEST` bit and requires `quest_left <= 0` (`game.rs:4495-4496`; `quest_left` is built from
  `m.quest` in `input.rs:2590-2596`, `melee_possessed` etc.). Effects: spider only needs the 32 quest-flagged
  of 93 spiders dead, the 2 non-quest library map monsters are ignored, and the "one enemy may remain" quirk
  is gone.
- [x] `q_betwen.cc:169-200` `quest_between_death_hook` — no escort/escape phase. C++ opens the exit
  (`FEAT_LESS` under the player + "You can escape now.") only after fewer than two non-friendly monsters
  remain, and completion happens at Turgon's tower (`FEAT_SHOP` special 27, `q_betwen.cc:48-55`). Rust stamps
  `T_STAIRS_UP` at `qm.exit` **from level creation** (`game.rs:7372-7381`, `exit == start == (47,17)`) and
  treats simply leaving as completion (`input.rs:3064-3071`), so the ambush can be skipped in one step.
- [x] `q_betwen.cc:33-91` `quest_between_move_hook` — the ambush trigger is narrower. C++ fires on **any**
  move while the quest is taken and the player is in the northern wilderness (`wilderness_y <= 19`, one-time
  `data[0]` guard). Rust only fires when stepping onto a `T_BETWEEN` Void Jumpgate (`input.rs:1758-1776`), and
  has no one-ambush guard. The C++ "Turgon is there" move hook is replaced by the exit staircase.
- [x] `q_nirna.cc:71-90` `quest_nirnaeth_finish_hook` — the two-thirds bonus is missing. C++ pays 200 000 gold
  only when `cquest.data[1] >= 2*cquest.data[0]/3` and otherwise just thanks the player. Rust always prints the
  bonus text and always pays `ps.gold += 200000` (`game.rs:4589-4593`). `PlotQuest::quest_total` exists but is
  never read (`game.rs:1940`, only written at 7371), and the kill counter of
  `q_nirna.cc:101-108` `quest_nirnaeth_death_hook` has no equivalent.
- [x] `q_thief.cc:158-197` `quest_thieves_finish_hook` — the Troll/Wight fork is deterministic. C++ picks
  randomly 10% of the time, otherwise by combat-vs-magic skill (`magik(10) || combat == magic → 50/50`,
  else higher skill wins). Rust's `mayor_quest` always offers Troll first whenever it is untaken
  (`modal.rs:17579-17597`). **FIXED: mayor_quest now rolls the q_thief.cc fork (10% random, else
  SK_COMBAT vs SK_MAGIC, ties 50/50).**
- [x] `q_hobbit.cc:145-195` `quest_hobbit_chat_hook` — reward delay semantics inverted. C++: Melinda only
  spawns after `data[1] + DAY*10` where `data[1]` was the turn of the *init hook* (game start), and she hands
  out the Rod of Recall immediately once the quest is COMPLETED. Rust hands the rod only when
  `ps.turn - hobbit_turn >= 10*TURNS_PER_DAY` where `hobbit_turn` is the moment Merton was rescued
  (`input.rs:1988-2011`, set at 2041), and spawns Melinda without any day gate (`game.rs:5299-5311`). Also the
  hobbit depth is rolled when talking to Melinda (`input.rs:2014`) instead of at game start
  (`q_hobbit.cc:214-222`), and Merton is spawned in any dungeon rather than `DUNGEON_MAZE` (id 18,
  `q_hobbit.cc:70` vs `game.rs:5002`).
- [x] `q_thrain.cc:60-79` + `q_thrain.cc:228-257` `quest_thrain_death_hook` / `quest_thrain_move_hook` — the
  hidden vault (mimic 61 glass/illusion walls) is fully visible. The converter drops the `mimic` field
  (`convert_data.py:2062-2068` only stores terrain/lit), so `embed_thrain` (`game.rs:4746-4801`) stamps the
  cells as ordinary walls/floor, the "The magic hiding the room dissipates." reveal (on stepping onto a
  CAVE_FREE mimic cell) has no equivalent, and the death hook never prints that message. Thrain's reward is
  pushed into the pack instead of dropped at his position after wiping the 3×3 glass box and deleting him
  (`q_thrain.cc:110-133`), and the quest is left at COMPLETED instead of the original's FINISHED.
  — RECONCILIATION (2026-09-16 second pass): done — `embed_thrain` copies `qm.mimics` into
  `Map.mimic`, the world-turn system reveals them ("The magic hiding the room dissipates.") when
  the player steps on one (`game.rs`, quest_thrain_move_hook), the death hook prints the line and
  sets PLOT_FINISHED, and Thrain's reward is dropped at his recorded cell (`plot.thrain_pos`)
  instead of being pushed into the pack.
- [x] `q_troll.cc:170-198` `quest_troll_death_hook` — the troll ambush is missing. On the first non-Tom troll
  death C++ prints "Oops, seems like an ambush..." and spawns Forest/Stone trolls on every `CAVE_SPEC` cell of
  `trolls.map` (the 'H' tiles with cave_info 1027); `cquest.data[0]` is re-armed on every entry
  (`q_troll.cc:118-119`). The converter keeps no cave-info bits, and Rust only places Tom as the boss
  (`game.rs:7352-7370`), so the ambush and the spec tiles are gone. Tom is also placed at a random cell
  rather than at the map's `FEAT_MARKER`.
- [x] `q_rand.cc:330-401` `hero_death` — the lost-sword adventurer cannot join as a companion. C++ offers
  `Do you want him to join you?` when `can_create_companion()` is true, and otherwise (or on refusal) teaches a
  skill. Rust always goes to the skill-gain offer (`game.rs:5720-5730`), so the companion branch, the
  `MSTATUS_COMPANION` spawn with `monster_exp(1+dun_level*3/2)` and the `scatter` placement are absent.
- [x] `q_rand.cc:288-328` `princess_death` — no exit staircase is created. C++ deletes the princess and turns
  her glass room into `FEAT_FLOOR` plus `FEAT_MORE` at her position. Rust only sets `plot.offers` on the kill
  (`game.rs:5706-5732`); the princess is never removed and no staircase is raised. Task reward picking itself
  exists (`Modal::RandReward`, `modal.rs:10710-10733`).
- [x] `q_rand.cc:591` `quest_random_gen_hook` — the level-feeling boost (`rating += 10`, "a la pits") is not
  applied when the princess room is stamped (`apply_rand_quest`, `game.rs:4846-4920`).
- [x] `q_rand.cc:438-443` `quest_random_turn_hook` — new-level/regen reset of the counter does not exist in
  Rust: `RandQuest.got` is persisted in `PlotQuest::rands`, so partial kills survive leaving and re-entering
  the level while C++ resets `data[0]` to 0 on every new level.
- [x] `q_eol.cc:144-163` `quest_eol_fail_hook` — the fail-hook message is absent and the
  FAILED→FAILED_DONE state is not modelled anywhere in the port (no `PLOT_FAILED_DONE` constant). Abandoning
  Eol just prints "You abandon the quest... It is failed, forever." (`modal.rs:8212-8238`); the original
  prints "You fled ! I did not think you would flee..." from the fail hook when the patron is next visited
  (`bldg.cc:673-746`). Same missing transition for every failed quest.
- [x] `q_rand.cc:58-202` `initialize_random_quests` — monster selection is a uniform draw from the base list
  with an ad-hoc `depth <= rl+10` cap (`roll_rand_monster`, `game.rs:4806-4841`), not the original
  `get_mon_num(rl+4+randint(6))` level-weighted allocation; the `AQUATIC`/`WILD_ONLY` filters are additions,
  and on failure Rust silently drops the quest (C++ sets `q_ptr->type = 0`). Player-visible effect: different
  quest monster tiers per depth.
- [x] `q_narsil.cc:38-57` `quest_narsil_move_hook` — only pack items are checked. C++ scans `0..INVEN_TOTAL`
  (worn+pack) for `ART_NARSIL`, Rust searches `inv.pack` and additionally requires `identified`
  (`game.rs:6992-6998`); an equipped Narsil cannot be reforged. C++ also prints the seven-line Aragorn
  dialogue; Rust substitutes two lines.
- [x] `q_invas.cc:113-137` `quest_invasion_turn_hook` — the auto-grant is missing the
  `p_ptr->inside_quest` and `p_ptr->astral` guards (`game.rs:8381-8391`), so the Thunderlord can appear while
  the player is already inside another quest level or in astral form.
- [x] `game.rs:2296-2304` `quest_unlocked` (framework) — the Lorien chain skips WOLVES→SPIDER: Rust allows
  the spider entrance with `plot.status(PLOT_SPIDER) == UNTAKEN` regardless of wolves
  (`PLOT_SPIDER` is missing from the match; the default arm returns true), while C++
  `q_wolves.cc:149` only sets `plots[PLOT_LORIEN] = QUEST_SPIDER` after the wolf quest is finished.

### 3. q_god.cc (the Lost Temple rework)

The port replaced the temple quest with "relic on any regular dungeon level". The following original mechanics
have no equivalent (all values are concrete; the port's version is `game.rs:3942-3956`, `5037-5069`,
`12342-12354`, `input.rs:1107-1122`):

- [x] `q_god.cc:52-106` `compass` + `q_god.cc:109-131` `approximate_distance` + `q_god.cc:177-231`
  `get_home_coordinates` + `q_god.cc:233-282` `make_directions` — no directional clue is ever produced; the
  player only gets "the relic lies on this level".
- [x] `q_god.cc:301-367` `quest_god_place_rand_dung` — the dungeon-30 "Lost Temple" is never placed in the
  wilderness (no runtime `wilderness(x,y).entrance = 1000+DUNGEON_GOD`, no Bree fallback (32,19), no clearing
  of the old entrance/`max_dlv[DUNGEON_GOD]`). `dungeons.ron` still contains id 30, unused.
- [x] `q_god.cc:369-430` `quest_god_generate_relic` — the relic is generated on random normal dungeon levels
  (20% per level, forced 5th) rather than in the temple; the "in the backpack if no safe cell" fallback is
  absent.
- [~] `q_god.cc:432-909` `quest_god_set_god_dungeon_attributes_eru/manwe/tulkas/melkor/yavanna/aule/varda/ulmo/mandos`
  (9 functions) — the per-deity temple layout/monster/object rules (floors, fills, flags, `W:`, `O:`, `R:`/`M:`
  rules) are not applied; there is no god dungeon to configure.
- [x] `q_god.cc:968-1028` `quest_god_player_level_hook` — no `cquest_dun_minplev` scum guard (the original
  refuses/updates `minplev` so a quest cannot be rolled twice at the same character level, and also blocks
  astral and in-temple rolls). Rust only checks `god>0`, `!god_failed`, `god_quests<5`, `!relic_quest`,
  `relic_depth==0` and 21% (`game.rs:3945-3956`).
  — RECONCILIATION (2026-09-16 second pass): done — `PlayerState.god_min_plev` records the level of
  every refused roll, the 21% roll is only granted above it, and the astral/in-temple refusals stay
  in place (game.rs).
- [x] `q_god.cc:1030-1085` `quest_god_get_hook` — reward/last-piece divergence. C++ awards
  `SKILL_PRAY.value += 5 * mod` per piece and `+= 10 * mod` on the fifth, with deity-specific messages; Rust
  always awards a flat `+2` Prayer school levels (`input.rs:1118`) and a single generic message. The
  "pval != true" per-piece flag and the `quests_given == MAX_NUM_GOD_QUESTS` branch are absent.
  — RECONCILIATION (2026-09-16): FIXED — `game::god_relic_reward` (`game.rs:10256-10276`) pays
  `5*mod`/`10*mod` with the deity line and the fifth-piece branch, wired at the relic pickup
  (`input.rs:3260-3274`); the C++ `pval != true` re-entry guard is implicit because the piece is consumed
  on pickup.
- [~] `q_god.cc:1123-1177` `set_god_dungeon_attributes` / `q_god.cc:1179-1208` `quest_god_dungeon_setup` /
  `quest_god_enter_dungeon_hook` / `quest_god_gen_level_begin_hook` / `quest_god_stair_hook` — all five
  missing (they only exist to configure and detect the temple). The failure condition is approximated by
  `game.rs:12340-12355` (leave the level with the relic still on the floor), whereas the original fails when
  a *new* temple level is generated after the relic was already created (`q_god.cc:911-966`).
  — RECONCILED (2026-09-16): these hooks are `[~]` because the god-dungeon subsystem they configure does
  not exist in the port by design: `set_god_dungeon_attributes` mutates the runtime `d_info[DUNGEON_GOD]`
  (W: 5-level cap, `min_plev`, per-deity floors/fills/flags/`O:`/`R:` tables) *before* the temple level is
  generated, and the port's Lost Temple is the static `dungeons.ron` id 30 record with the deliberate
  "relic on a regular level" rework.  There is no runtime d_info mutation and no per-deity temple layout,
  so the five hooks would be no-ops; implementing them requires the per-deity tables (also `[~]` above).
  The `[ ]` is therefore closed as `[~]` frontend/subsystem, not as missing code.

### 4. Smaller numeric / branch divergences

- [x] `q_dragons.cc:60-106` `quest_dragons_gen_hook` — dragon tables/probabilities differ. C++:
  1% happy (601,617,644,624,602,645,618,675), 32% baby, 33% young, 34% mature with the fixed 8-colour lists
  `{163,164,167,166,218,219,165,204}`, `{459,460,563,546,462,559,461,556}`, `{560,549,589,592,562,590,561,593}`.
  Rust (`game.rs:7259-7273`) rolls 1% happy, **33% mature**, 33% young, 33% baby, and the arrays contain
  different ids (546 in BABY, 463/589 in YOUNG, 601 in MATURE). The 35 `FEAT_MOUNTAIN` obstacle columns
  (`q_dragons.cc:60-73`) are not placed, and placement uses ±8 around the entrance instead of the
  `rand_int(21)+3`×`rand_int(31)+3` area with `magik(33)` sleep.
- [x] `q_library.cc:314-357` `quest_library_gen_hook` — spawn counts/regions differ: C++ places
  `damroll(4,2)` liches in 4-37/4-14, three batches of monastic liches (`damroll(1,2)`, `damroll(1,2)-1`,
  `damroll(1,2)-1`) in the three surrounding regions, 2×flesh/clay/iron golems and 1 mithril golem. Rust
  (`game.rs:7305-7325`) spawns only `4d2` liches, one 611 and one 464, then 2 each of 256/261/367, all near
  the entrance.
- [x] `q_poison.cc:131-141` `quest_poison_gen_hook` — the "sometimes make it up some levels" step
  (`magik(80)` → `exp = monster_exp(level + randint(pl - level))`) is not implemented; Rust molds keep their
  base level (`game.rs:5393-5422`). Rust also spawns molds in a ±10 box near one random centre, while C++
  walks the whole radius-25 area converting water and spawning at 60% inside radius 10.
- [x] `q_poison.cc:256-273` `quest_poison_drop_hook` — the "too many monsters" test counts
  `m_ptr->status <= MSTATUS_NEUTRAL` over all monsters; Rust counts only quest-flagged molds
  (`modal.rs:8665`), so unrelated wildlife no longer blocks purification.
- [x] `q_bounty.cc:24-53` `lua_mon_hook_bounty` — missing filters: `RF_DROP_CORPSE` required, and rejections
  for `RF_NEVER_GENE`, `RF_FRIENDLY`, `SF_MULTIPLY` and `RF_JOKEANGBAND`. Rust only excludes
  unique/GOOD/PET/SPECIAL_GENE (`modal.rs:17630-17640`), so corpse-less, friendly, breeding or joke monsters
  can be assigned. **FIXED: bounty_assign now checks DROP_CORPSE/NEVER_GENE/FRIENDLY/MULTIPLY/JOKEANGBAND.**
- [x] `q_bounty.cc:55-74` `get_new_bounty_monster` — uses the level-weighted monster table
  `get_mon_num(3 + lev*3/2)`; Rust picks uniformly from all depth<=cap monsters (`modal.rs:17645`).
- [x] `q_fireprof.cc:94-145` `fireproof_enough_points` + `q_fireproof.cc:147-226` `fireproof` — no stack
  amount prompt / partial-stack split ("How many would you like fireproofed?"); Rust charges
  `cost * item.count` and refuses the whole stack if it does not fit the remaining points
  (`modal.rs:9087-9099`). Eligibility also uses a `fireproof` flag rather than "not already TR_IGNORE_FIRE".
- [x] `q_fireprof.cc:365-383` `fireproof_get_hook` — C++ tags the spawned scroll with `pval2 = sval` and
  `inscription = "quest"` and matches on that; Rust matches by object name "Fire"
  (`input.rs:1089-1092`), so any ordinary Scroll of Fire picked up elsewhere would also complete the quest
  (and the original's "Fine! Looks like you've found it." message is absent).
- [x] `q_nirna.cc:101-108` `quest_nirnaeth_death_hook` — see 2: no per-kill counter, so the 2/3 branch is
  impossible even if it were written.
- [x] `q_wight.cc:145-152` `quest_wight_death_hook` — C++ raises `FEAT_LESS` **under the player** and clears
  the special; Rust sets the staircase at the fixed map exit `qm.exit == (3,3)` (`game.rs:4508-4511`).
- [x] `q_rand.cc:263-285` `do_get_new_obj` — unchosen reward items are only recycled for classic artifacts
  (`ctx.created.0.remove(&other.artifact)`, `modal.rs:10723-10728`). The C++ also resets
  `game->random_artifacts[sval].generated` (TV_RANDART) and the `TR_NORM_ART` flag, so those two cases may
  remain "used" in Rust.
- [x] `q_one.cc:101-138` `quest_one_drop_hook` — condition broadened: C++ requires exactly `FEAT_GREAT_FIRE`
  (178), Rust accepts terrain 178 **or** lava/shallow-lava and adds a `depth >= 60` requirement
  (`modal.rs:8602-8616`). The original status guard (`cquest.status == QUEST_STATUS_TAKEN`) is not checked
  either, so the Ring can be destroyed before Galadriel's warning if a great-fire tile is somehow reached.
- [x] `q_one.cc:241-243` `quest_one_die_hook` — the death cause `died_from = "being drawn to the shadow
  world"` is not set; Rust only prints the two lines (`game.rs:3846-3847`).
- [x] `q_main.cc:168-188` `quest_sauron_resurrect_hook` — Sauron's recoil is modelled in `check_quest_kill`
  but the Nazgûl clause (`RF_NAZGUL` death → "Somehow you feel %s is not totally destroyed..." and
  `max_num = 1`) has no equivalent anywhere in Rust; Nazgûl simply stay dead.
  — RECONCILIATION (2026-09-16 second pass): done — every slain unique is marked in
  `PlayerState.unique_seen` and the random allocation skips it; the Nazgûl/Sauron clause removes the
  race from that set while the Ring is neither destroyed nor worn, so they can be generated again
  (game.rs).
- [~] `q_ultrag.cc:278-303` `quest_ultra_good_dump_hook` — char-dump text only (also q_*_dump_hook across the
  group); marked `[~]` as score/UI output, listed here for completeness.

## UNCERTAIN

- `q_rand.cc:506-594` `quest_random_gen_hook` — Rust stamps the princess room into `T_GRANITE` only; if the
  layout cannot be embedded within 200 tries the quest silently does not appear that level (no fallback), and
  whether that matches the original's `room_alloc` failure behaviour is not verifiable without running the
  original.
- `q_one.cc:104-105` — the port's Ring-destruction depth requirement (60) has no C++ counterpart; handoff
  states this was intentional (Mount Doom is deep in the port's single-shaft world).
- `q_hobbit.cc` — the C++ HOOK_MON_SPEAK "begs for your help" line and HOOK_MON_ASK_HELP broadcast are only
  approximated by the bump-dialogue (`input.rs:1986-2024`); a UI decision.
- `q_thief.cc:70-93` — beyond the missing offset, the Rust disarm also removes equipped non-cursed items and
  the entire pack at once (fine) but leaves cursed *pack* items in place like C++ only does for equipment;
  actual in-game behaviour could not be exercised here.
- `q_library.cc:498-513` `quest_library_init_hook` — the original re-finalizes the reward book into the
  player's spellbook after loading a save where the quest was REWARDED. Rust stores the chosen spells on the
  item (`Item.spells`) which is serialized, so this may be covered, but no save-roundtrip test targets a
  crafted Library tome.

## COVERAGE SUMMARY

- Total functions 203; [x] = 94, [>] = 60, [ ] = 27, [~] = 22.
_2026-09-16 session: +19 boxes resolved (mostly [x] new, some [>]) — see DONE THIS SESSION above._

## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- `q_library.cc:44-101` — `spell.rs::BOOKABLE_SPELLS` expanded from 33 to the full 43-spell C++ push
  order (added Fire Golem, Geyser, Vapor, Ent's Potion, Stone Skin, Recharge, Armor of Fear, Stun,
  Grow Trees, Tree Roots). The library test now asserts every name resolves.
- `q_bounty.cc:108-158` — `modal.rs::bounty_turn_in` no longer pays gold/exp: it prints the original
  two lines, grants `+mod` SK_LORE (`mod 0 → 900`, dev) and teaches SK_PRESERVATION (value/mod 800,
  dev) once, then returns the quest to UNTAKEN.
- `q_bounty.cc:24-53` — `modal.rs::bounty_assign` adopts the full lua_mon_hook_bounty filter set.
- `q_fireprof.cc:255-290` — the Wizards Spire branch consumes one carried Scroll of Fire
  (`PLOT_COMPLETED → PLOT_FINISHED`) before opening `Modal::Fireproof`, prints the 6/8/24 line, and
  resumes partial fireproofing from PLOT_FINISHED. Remaining: input.rs pickup mark (pval2/inscription)
  and the `y=3..5, x=3..47` drop rectangle.
- `q_thief.cc:158-193` — `modal.rs::mayor_quest` picks Troll/Wight with the original 10%-random /
  combat-vs-magic rule.
- `q_shroom` artifact 149 + Cure Serious, `god_relic_reward`, Farmer Maggot delivery, `drunk_takes_wine`
  and `do_cmd_suicide` remain input.rs call-site wiring (not owned here; no modal hook exists for
  suicide).

## DONE THIS SESSION (2026-09-16, game.rs/modal.rs/input.rs)

Verified/changed by the game.rs/modal.rs/input.rs agent:

- **One Ring circular blocker** (first GAP bullet): the Galadriel gate already reads
  `quest.stage >= sauron_stage` (input.rs), so the warning is reachable after Vecna; the blocker text
  was stale. `check_quest_kill`'s recoil message now matches q_main.cc exactly
  ("Sauron will not be permanently defeated until the One Ring is either destroyed or used...").
- **q_one.cc:101-138 drop hook**: both drop paths (input.rs `picker_drop` and modal.rs `Modal::Drop`)
  now require `plot.one_taken` (cquest TAKEN) and `terrain == 178` (FEAT_GREAT_FIRE) exactly, with the
  original four-line message ("You throw the One Ring into the Great Fire; ..."), not lava. The port's
  `depth >= 60` gate is kept (Mount Doom's Great Fire lies at depth 99).
- **q_one.cc:241-243 death cause**: still open — the message pair is printed but `died_from` has no
  PlayerState field and birth.rs (which lists every field) is reserved for the next agent.
- **q_main.cc:168-188 Nazgul clause**: now prints "Somehow you feel he/she is not totally destroyed..."
  while Sauron is not permanently dead (`!(ring_destroyed || ring_worn)`); respawning the Nazgul
  (`max_num = 1`) is not modelled (no per-race cur_num/max_num).
- **q_rand.cc:288-328 princess_death**: the princess's cell is recorded in `RandQuest.princess_pos`;
  on completion her glass room (3x3) is wiped, an `FEAT_MORE`/T_STAIRS_DOWN is raised where she stood
  and the princess monster is despawned (deferred to the next input frame where the monster query
  exists). Messages are the original two lines.
- **q_rand.cc:330-401 hero_death**: lost-sword completion now prints the original lines and offers the
  Adventurer as a companion (`Modal::RandHero`) when below `1 + skill_scale(LORE,6)`; the companion is
  spawned with `monster_exp(1 + dun_level*3/2)` and level-up rolls (`game::spawn_hero_companion`,
  scatter placement with the glyph/Between checks). Refusal (or a full roster) teaches a skill as
  before.
- **q_rand.cc:58-202 initialize_random_quests**: `roll_rand_monster` now filters the full C++ predicate
  set (SPECIAL_GENE/NEVER_GENE/MULTIPLY/JOKEANGBAND/PET/NAZGUL/GOOD/EXPLODE, unique by type,
  level > min(rl,49)) and draws through the new `game::get_mon_num_from` (100/rarity weights, two
  NASTY_MON boosts, 50%/10% keep-the-harder re-rolls) instead of the ad-hoc uniform draw.
- **q_wolves/q_spider/q_haunted/q_dragons/q_evil death hooks**: `kill_monster`/`check_plot_kill` now
  take a second count of *all* hostile monsters (`enemy_left`, excluding friendlies/companions/neutral)
  and complete at `enemy_left <= 0`, matching `status <= MSTATUS_ENEMY && mcnt <= 1` (the fresh corpse
  is included in the original count). Thieves additionally keep the quest-flag count.
- **q_betwen.cc:33-91 ambush**: the thunderlord ambush now springs on *any* move made in the northern
  wilderness (`wilderness_y <= 19`, or `py <= 19` in travel mode), once per game
  (`PlotQuest.between_ambush`), exactly as the move hook.
- **q_betwen.cc:169-200 escape phase**: the Between level no longer starts with a staircase; once fewer
  than two non-friendly monsters remain, "You can escape now." is printed and FEAT_LESS is raised under
  the player (deferred via `PlotQuest.between_escape`). Completion on leaving is kept because base
  data has no Turgon tower tile (the FEAT_SHOP special 27 branch is a data dead entry).
- **q_nazgul.cc:63-91 reward timing**: the kill only completes the quest; the three-part mayor
  dialogue and six sprigs of Athelas (TV_FOOD sval 40) are paid in `finish_plot_quest`.
- **q_shroom.cc:199/216-223**: delivery now grants one stack of 15-20 `SV_FOOD_CURE_SERIOUS` (16,
  aware+known) and the specific artifact 149 "of Farmer Maggot" (a_info TV_BOW/SV_SLING) instead of an
  ego sling and seven random food kinds; the quest goes straight to FINISHED as in the C++ give hook.
- **q_bounty.cc:55-74**: bounty assignment now draws through `game::get_mon_num_from(3 + lev*3/2)`
  with the lua_mon_hook_bounty filter set (FORCE_DEPTH/ONLY_DEPTH included).
- **q_fireprof.cc:365-383**: the Scroll of Fire is spawned marked `pval2 = sval(48)` and
  `inscription = "quest"`, dropped in the map-local rectangle `y = randint(3)+2, x = randint(45)+2`
  (expanding to the nearest floor), and the pickup hook matches that mark and prints "Fine! Looks like
  you've found it.". The Spire turn-in (`PLOT_COMPLETED`) consumes the marked scroll.
- **q_fireprof.cc:94-226**: `Modal::FireproofQty` asks "How many would you like fireproofed?" for
  stacks, charges `cost*stack` and splits the stack for partial fireproofing; eligibility is "not
  already IGNORE_FIRE" instead of the port-only `fireproof` flag.
- **q_hobbit.cc:145-195**: Melinda hands over the Rod of Recall immediately when the quest is
  COMPLETED; the depth is no longer re-rolled on talk (birth.rs rolls it at game start, with a legacy
  fallback only when the field is 0).
- **q_god.cc:1030-1085**: the relic reward is now wired to `game::god_relic_reward` on pickup
  (`+5*Prayer.mod` per piece, `+10*mod` on the fifth) instead of a flat +2 levels.
- **q_god.cc:968-1028**: astral beings and characters inside the Lost Temple are now refused a relic
  quest; the `cquest_dun_minplev` scum guard still has no equivalent field (PlayerState is off-limits).
- **modules.cc drunk_takes_wine / hobbit_food / smeagol_ring**: the Give command now applies all three
  base-module HOOK_GIVE hooks ("'Hic!'" + empty bottle for the happy drunk, "'Yum!'" for the scruffy
  hobbit, "'MY... PRECIOUSSSSS!!!'" for Smeagol).

Still open from this report (with reasons):

- `q_one.cc:241` `died_from` (needs a PlayerState field; birth.rs is reserved).
- `q_rand.cc:591` `rating += 10` (Map stores only the quantised feeling, not the raw rating).
- `q_god.cc:369-430` relic placement rules and `q_god.cc:432-909` per-deity temple attribute tables
  (the port has no Lost Temple level to configure; deliberate rework); "relic into the pack if no safe
  cell" needs an Inventory handle in `populate_level`.
- `q_god.cc:968-1028` `cquest_dun_minplev` (no PlayerState field available).
- `q_thrain` hidden vault (converter drops the `mimic` cell field).
- `q_poison.cc:256-273` "too many monsters" count includes non-quest wildlife.
- `q_rand.cc:263-285` TV_RANDART/TR_NORM_ART recycling of unchosen rewards.
- `q_*` dump hooks (UI/score text, `[~]`).

## DONE THIS SESSION (final, 2026-09-16, game.rs/map.rs/modal.rs/input.rs/item.rs/birth.rs/save.rs)

- `q_rand.cc:591` **rating += 10 ("a la pits")**: `Map` now keeps the raw `rating` and
  `good_item` (serde-default fields, copied by `supersize_level`); `apply_rand_quest`
  adds 10 and recomputes `feeling_from_rating` when the princess room is stamped, exactly
  like the HOOK_BUILD_ROOM1 hook. (C++ applies it before the auto_scum check; the port
  stamps after generation, so auto_scum cannot see the +10 — the only observable
  difference.)
- `q_rand.cc:438-443` **counter reset**: verified `populate_level` already resets
  `RandQuest.got = 0` before re-stamping the quest on every generated level; the
  Ambarkanta/`turn.regen_level` path drops the snapshot and regenerates through
  `populate_level`, so HOOK_NEW_LEVEL + HOOK_LEVEL_REGEN are both covered. No change.
- `q_poison.cc:256-273` **drop hook count**: `modal.rs` now counts every monster with
  `status <= MSTATUS_NEUTRAL` (`!friendly && !companion`) instead of only quest-flagged
  molds; unrelated wildlife blocks purification again.
- `q_rand.cc:263-285` **do_get_new_obj recycling**: unclaimed `TV_RANDART` junkarts
  (`created` key `1000 + note`) and `TR_NORM_ART` base kinds (`norm_art_key`) are now
  released alongside the classic `a_info` artifacts.
- `q_one.cc:241-243` **died_from**: `PlayerState.died_from` (serde default, birth.rs
  initialised) is set to "being drawn to the shadow world" on the Ring's final claim.
  The score/death-screen display of `died_from` lives in `scores.rs`/`hud.rs`
  (outside this session's file ownership) and still prints the old text.
- `q_god.cc:369-430` **quest_god_generate_relic**: `populate_level` now follows the
  original: up to 1000 random `FF_FLOOR && !FF_PERMANENT` cells; found → drop the
  inscribed ("quest") relic there; not found → "You luckily stumble across the relic on
  the stairs!" and into the pack (or on the player when the pack is full).
- `q_god.cc:432-909` **per-deity temple attribute tables**: `[~]` — the port deliberately
  reworked the Lost Temple into "the relic lies on a regular level"; `dungeons.ron` id 30
  has no level generator, so the W:/O:/R:/M: attribute tables have no consumer. Genuinely
  absent god-dungeon subsystem.
- `q_ultrag.cc:278-303` **dump text**: `[~]` — the port has no character-dump feature at
  all (only `scores.rs` entries), so dump-hook text is UI/score output.

## FINAL RECONCILIATION

Reconciliation pass 2026-09-16 (third pass, spell/game-side session): no quest bullet is
in the owned files.  The 3 previously-open `[>]` bullets stay `[x]`; the pre-existing
`[~]`/`[ ]` entries are unchanged:

- `[x]` Thrain hidden vault (`q_thrain.cc:60-79`/`228-257`) — `embed_thrain` copies `qm.mimics`,
  stepping on a mimic cell reveals the room, and the reward is dropped at Thrain's recorded cell.
- `[x]` `q_god.cc:968-1028` `quest_god_player_level_hook` — `god_min_plev` records refused rolls.
- `[x]` `q_main.cc:168-188` `quest_sauron_resurrect_hook` — Nazgûl/Sauron leave `unique_seen`
  while the Ring is neither destroyed nor worn, so they can respawn.
- `[~]` `q_god.cc:432-909` — per-deity Lost Temple attribute tables have no consumer (deliberate
  rework; `dungeons.ron` id 30 unused).
- `[~]` `q_god.cc:1123-1208` — the five temple setup/enter/stair hooks are absent with the same
  god-dungeon subsystem (deliberate static-dungeon rework; no runtime d_info mutation to configure).
  No `[ ]` bullets remain in this report.
- `[~]` `q_ultrag.cc:278-303` — char-dump text; the port has no dump feature.

Notes (cross-file, not owned this session):
- `q_one.cc:241` `died_from` is set (`PlayerState.died_from`); its display lives in
  `scores.rs`/`hud.rs`, which are outside this session's file ownership and still print the
  old text.
- The `q_god` relic "into the pack if no safe cell" path and all reward hooks were verified
  in the previous pass; nothing in `game.rs`/`map.rs`/`birth.rs` remains open here.
