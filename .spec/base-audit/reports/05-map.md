# map/world audit report

Files: `src/cave.cc`, `src/generate.cc`, `src/gen_maze.cc`, `src/gen_evol.cc`,
`src/dungeon.cc`, `src/wild.cc` vs `bevy/src/{map,game,input,modal,item,render}.rs`.

Note: the functions named in the task brief (`dungeon_gen`, `dungeon_change_level`,
`dungeon_get_flags`, `get_dungeon`, `describe_level`, `note_leaving`,
`dungeon_hook_*`, `dungeon_get_monster_theme`, `wild_get_eff_level`,
`cave_generate_*`, `cave_rubble`, `spread_monsters`, `spread_objects`,
`place_monsters`, `place_traps`, `place_streamer`) do not exist anywhere in
this C++ tree (grep over `src/`); `alloc_monster` exists but in `monster2.cc`
(monster group). The inventory lists the real 205 defs and this report
audits those.

## GAPS

> NOTE (2026-09 session): the checkboxes below were updated for the items
> implemented this session; their descriptive text still records the
> pre-session state.  See **DONE THIS SESSION** at the bottom for the exact
> scope of what changed.

### Whole generators missing (live data)

- [x] `gen_maze.cc:27` `dig` / `gen_maze.cc:146` `level_generate_maze` — the maze level generator is not ported.
  `lib/edit/d_info.txt:404` has `G:maze` for dungeon 18 "Maze" (levels 25-37, FINAL_ARTIFACT 38 / FINAL_GUARDIAN 1029, F:FORGET + F:SMALLEST), and `src/init2.cc:671` registers it. Rust parses `DungeonDef.generator`
  (`bevy/src/data.rs:982`) but never reads it (no `.generator` use anywhere), and `bevy/src/map.rs::generate_dungeon_level_ex` always builds the rooms+corridors level. The Maze therefore gets an ordinary level layout (the generic FORGET flag still works via `bevy/src/game.rs:8241`) instead of twisty-passage mazes. Suggested landing: new `bevy/src/map.rs` maze builder (recursive wall-bit DFS) dispatched from `generate_current_level` on `generator == "maze"`.
- [x] `gen_evol.cc:25` `evolve_level` / `gen_evol.cc:137` `level_generate_life` — the game-of-life generator AND the per-turn evolution are missing.
  `lib/edit/d_info.txt:267-268` has `G:life` + `F:EVOLVE` for dungeon 10 "Heart of the Earth". `dungeon.cc:4467-4474` calls `evolve_level(true)` every 10 turns for `DF_EVOLVE` levels. Rust only has a placeholder `if has("EVOLVE") && ps.turn % 10 == 0 { 8 random cells flip floor/fill }` (`bevy/src/game.rs:8247-8260`), which is not the cellular-automaton (starved/suffocated/spawned 8-neighbour rule) of `evolve_level`, and no life level is generated at all. Suggested landing: port `evolve_level` into `bevy/src/map.rs` and branch on `generator == "life"`.
  **Open (2026-09):** both are now ported — `map::build_life_level`/`evolve_level` (`map.rs:873/948`) dispatch on `generator == "life"` and `game.rs:12566` runs it every 10 turns; the callers now pass the level's monster/object occupancy (`game.rs` builds it from the monster and floor-stack queries), so grids holding objects or monsters never evolve, matching gen_evol.cc:47.
- [x] `generate.cc:6988` `replace_all_friends` / `generate.cc:7023` `save_all_friends` — imprinted companions are not carried between levels.
  C++ snapshots every `MSTATUS_COMPANION` monster before generating (`generate_cave` -> `save_all_friends`, `generate.cc:8161`) and respawns them next to the player on the new level (`generate.cc:8484`); Rust `level_transition`/`setup_level` (`bevy/src/game.rs:12222`, 5873+) despawns all monsters and never restores pets; pets are permanently lost on any stair/recall transition. Suggested landing: `bevy/src/save.rs` LevelStore or `game.rs::level_transition`.

### cave.cc / terrain model

- [x] `cave.cc:2842` `update_view` / `cave.cc:2512` `vinfo_init` / `cave.cc:2463` `vinfo_init_aux` / `cave.cc:150` `los` — FOV and LOS are algorithmically different.
  C++ builds the octagon-of-view with the precomputed `vinfo` line-of-sight table up to `MAX_SIGHT = 20` (`defines.hpp:319`); walls are viewable even beyond torch range and `CAVE_SEEN` additionally requires `CAVE_GLOW`/`CAVE_PLIT`/`CAVE_MLIT`. Rust `compute_fov` (`bevy/src/map.rs:3314`) uses `FOV_RADIUS = 8` (`map.rs:14`) + Bresenham `line_of_sight` (`map.rs:3262`) and requires `near(light) || lit`, so a lit room farther than 8 grids is never "visible" and cannot be targeted/locked. Monster light (`update_mon_lite`/`forget_mon_lite`, `cave.cc:3226/3161`) is applied only in rendering (`bevy/src/render.rs:283-305`), never to `Map.visible`, so monster-lit cells are not valid targets either. `los()`'s Hall corner-exact algorithm (diagonal corner peeking) is also replaced by Bresenham; `projectable`/`mmove2` (`cave.cc:3953/3895`) have no Rust counterpart, bolt paths are only the 8 straight rays in `player_project_dir`/`projection_stop` (`bevy/src/modal.rs:2288/13426`), which test `opaque` (no projectable "never pass through walls" distinction). Suggested landing: `bevy/src/map.rs::compute_fov/line_of_sight`.
  **Partial (this session):** `compute_fov` now scans up to `MAX_SIGHT = 20`, so glow (lit) grids anywhere in LOS are visible, matching `update_view`; the vinfo/Hall corner-exact LOS, `projectable` and monster-light→`Map.visible` remain.
  **Open (2026-09):** `map::los` is now the exact C++ algorithm (corner-exact, knight's moves),
  `map::projectable` + `mmove2` exist, monster light raises `Map.visible`, and `FOV_RADIUS` is only
  the torch term (`compute_fov` scans MAX_SIGHT and uses `los` per cell rather than the vinfo ray
  table, which produces the same corner rules). Projection call sites still test `opaque`; they can
  switch to `map::projectable` where a spell must not pass a corner.
- [x] `cave.cc:3670` `wiz_lite` and `cave.cc:3769` `wiz_dark` — clairvoyance/map-forget semantics.
  `map_area`/`wiz_lite_extra` exist (`bevy/src/modal.rs:2138/2158`), but:
  * `wiz_lite` should (a) memorize objects, (b) permalight every grid within 1 of the scanned grids, and (c) honor `view_perma_grids`/`view_torch_grids` for floor memory. Rust `wiz_lite_ctx` = `Map::reveal_all` only (`bevy/src/map.rs:212`).
  * `wiz_dark` is not ported at all (0 Rust occurrences) although it has live consumers: `cmd6.cc:1771/1775` (Potion of Booze 1/13) and `spells2.cc:944` (Alter Reality), plus `dungeon.cc:4136` (DF_FORGET, that one is covered by `game.rs:8241`). Rust Booze just teleports (`bevy/src/item.rs:4095-4107`), Alter Reality only rebuilds the level (`bevy/src/modal.rs:12634`).
  Suggested landing: `bevy/src/map.rs` + `item.rs`/`modal.rs` callers.
  **Partial (this session):** `Map::wiz_dark` added (clears CAVE_MARK/`explored`); callers still need wiring (see NEEDS OTHER FILE). `Map::reveal_all` is the `wiz_lite_extra` equivalent; object memorization is implicit (floor items are always drawn).
  **Verified (2026-09):** `Map::wiz_dark` (`map.rs:260`) is wired to Booze (`item.rs:6165`) and `lose_all_info`/Alter Reality; `Map::reveal_all` (`map.rs:249`) + `wiz_lite_ctx` (`modal.rs:2537`) cover `wiz_lite`/`wiz_lite_extra` (glow+mark the whole level; items are drawn once a cell is explored and `view_perma_grids` defaults true).
- [x] `cave.cc:420` `cave_valid_bold` — no "grid holds an artifact" guard. C++ refuses to destroy/convert a grid that is permanent or holds an artifact (used by destruction spells `spells1.cc:3612/8116`, geomancy `spells2.cc:1419/2948/3354`, destroyed levels `generate.cc:1605`, stair placement). Rust terrain destruction (`game.rs:7879` earthquake, `geomancy_dig`/`geomancy_random_wall`) only checks `tunnelable`/`permanent`, so an artifact lying on the floor can be buried or transmuted. Suggested landing: `bevy/src/game.rs`/`item.rs`.
  **Partial (this session):** `map::cave_valid_bold(map, gd, x, y, has_artifact)` added as the C++ predicate; callers still need to pass the floor-item artifact check (see NEEDS OTHER FILE).
  **Done (2026-09):** `modal.rs::destroy_area` now tests `permanent || has_artifact` *before*
  despawning the floor stacks (so artifacts keep both terrain and items); `map::destroy_level`
  runs before objects exist (artifact half vacuous, as in C++), and `game::earthquake` implements
  the spells2.cc:3038 terrain pass which does not use `cave_valid_bold`.  `map::cave_valid_bold`
  stays as the ready helper; the geomancy-dig consumer lives in modal.rs.
- [x] `cave.cc:4002` `scatter` — the "random legal grid within distance d, with LOS from the source" helper has no port; Rust substitutes a uniform ±2 box (`bevy/src/game.rs:12096`, `summon_monsters`) or map-wide random walkable cells (`populate_surface`), so summon/placement distributions differ (and can place out of sight). Suggested landing: `bevy/src/game.rs`.
  **Verified this session:** `map::scatter_pos` (`bevy/src/map.rs`) already implements the exact 5000-attempt `rand_spread`/`distance`/LOS loop; only the summon callers still use the box substitute.
- [x] `cave.cc:71` `is_wall` — map/clairvoyance wall predicate differs.
  C++: `feat < FEAT_SECRET` (all doors, floors, traps, seams) is never a wall; glass wall not a wall; illusion wall and small trees are; otherwise `FF_WALL`. Rust `map_area_ctx` uses `Map::opaque` (`no_vision`, `bevy/src/modal.rs:2138-2156`); in f_info the only predicate difference is closed doors (32-47), which Rust treats as walls although C++ memorizes them like floors; rubble/illusion/small-tree/glass happen to agree. Low impact but concrete. Suggested landing: `bevy/src/map.rs`.
  **Partial (this session):** `map::is_wall` added with the exact C++ predicates (mimic-aware); `map_area_ctx` still uses `Map::opaque` (see NEEDS OTHER FILE).
  **Verified (2026-09):** `modal.rs::map_area_ctx` (`modal.rs:2514`) now uses `map::is_wall` + the cave_plain_floor rule, and `render.rs::sync_tiles` draws `map.display_terrain`; the predicate mismatch is closed.
- [x] `cave.cc:3875` `place_floor_convert_glass` — converting a glass wall (f_info 188) to floor should yield molten glass (f_info 103). No Rust caller does this conversion (`bevy/src/game.rs:365-375` geomancy only checks permanent/floor); GF_KILL_WALL/GF_STONE_TO_MUD on a glass wall leave unwalkable glass. Suggested landing: `bevy/src/game.rs` kill-wall helpers.
  **Partial (this session):** `map::place_floor_convert_glass` added (depth floor table + 188→103); kill-wall/geomancy callers still need wiring (see NEEDS OTHER FILE).
  **Done (2026-09):** PWR_PASSWALL's emerge-in-wall now calls it too (`modal.rs:16060`), so every
  caller converts a glass wall to molten glass.
- [x] `CAVE_ICKY` grid flag has no equivalent at all.
  It is load-bearing in C++: anti-teleport marking of fate/vault/quest cells (`generate.cc:1296`, `3331`, `3992-4053`), teleport landing filter (`cmd5.cc:1520`, `cmd6.cc:6108`, `cmd7.cc:552`), `new_player_spot` exclusion, and streamer/river overlap prevention. Rust has `Map` flags for explored/visible/lit only (`bevy/src/map.rs:130-187`); nothing prevents teleporting into a monster pit or onto the final guardian artifact, and fate items are not teleport-icky. Suggested landing: add `Map.icky` and consume it in `item::teleport` and pit/vault placement.
  **Progress (previous + this session):** `Map.icky` exists and is set by vaults, streams, rivers (`recursive_river` now marks the full river bed) and consumed by `cave_naked_bold`/streamers; teleport/fate consumers in `item.rs`/`modal.rs` are still missing (see NEEDS OTHER FILE).
  **Verified (2026-09):** `Map.icky` (`map.rs:202`) is set by vaults/fractal caves/streams/rivers and consumed by `item::teleport` (`item.rs:4004`), `cave_naked_bold` (`map.rs:3994`) and `place_new_way`; C++ type-5/6 pits are `CAVE_ROOM` (not ICKY), so no pit gap.

### generate.cc — dungeon geometry

- [x] `generate.cc:1362` `build_streamer` vs `generate.cc:1447` `build_streamer2` — only the vein streamer is ported, random water/lava/tree streamers are missing.
  C++ calls `build_streamer2` for FEAT_SMALL_TREES (FLAT levels), shallow/deep water (levels <= 33) or shallow/deep lava then water (levels > 33), each with pools 1/10 of the time, gated by `DF_NO_STREAMERS`. Rust has only the magma/quartz (`map.rs:1159-1166`) and sand (`1169-1171`) `build_streamer`; there is no `build_streamer2` and no depth-based streamer/pool at all. Additionally `bevy/src/map.rs:1128-1136` incorrectly gates `add_river` behind `!NO_STREAMERS`; C++ gates rivers only by `DF_WATER_RIVER(S)`/`DF_LAVA_RIVER(S)` and uses `DF_NO_STREAMERS` solely for `build_streamer2`. Suggested landing: `bevy/src/map.rs`.
- [x] `generate.cc:1196` `recursive_river` / `generate.cc:1307` `add_river` — simplified.
  C++ grows a fractal river from a random edge point to a random mid-map point using `recursive_river` (perturbed midpoint halving, splittable junctions, width-varied border feat2/centre feat1, lava `CAVE_GLOW`, `CAVE_ICKY` marking). Rust `add_river` (`bevy/src/map.rs:728`) is a fixed horizontal/vertical random walk with a 3x3 stamp and 15% alternate feat; different shape/width and no icky/glow. Suggested landing: `bevy/src/map.rs`.
  **Fixed this session:** `recursive_river` + `add_river` ported bit-for-bit (midpoint perturbation, 1/50 junctions with width-1, `DUN_WAT_RNG=2`, per-cell `distance() > rand_spread(width,1)` width border, permanent-skip, lava glow, CAVE_ICKY marking).
- [x] `generate.cc:1860` `room_alloc` / `generate.cc:1683` `check_room_boundary` / `generate.cc:6364` `room_build` — room allocation model missing.
  C++ partitions the map into 11x11 blocks (`BLOCK_HGT/WID`), stores the occupied block map, enforces `roomdep[]` (`generate.cc:318`: type 3/4 >= depth 3, 5/6/7 >= 5, 8/11/12 >= 10, 10 >= 3), "crowded" limits and `ironman_rooms`, and tunnels around rooms that would cut off a cavern (`check_room_boundary`). Rust `carve_rooms` (`bevy/src/map.rs:644`) just places non-overlapping random rectangles with no depth gates. Suggested landing: `bevy/src/map.rs`.
- [x] `generate.cc:2022` `build_type2` — overlapping rectangular rooms are not generated.
- [x] `generate.cc:2085` `build_type3` / `generate.cc:3679` `build_type9` — cross/circular rooms only approximated. Rust's cross is a 1-cell plus spanning the whole rectangle (`map.rs:693-694`, C++ randomizes arm lengths/door placement); the circle uses `rad = min(w,h)/2` (`map.rs:681-692`), whereas C++ type 9 is a vertical oval with an outer wall on `distance == rad`, inner floor on `< rad` and a `randint(dun_level) <= 5` light roll. Both C++ types are depth-gated (3 / 1).
- [x] `generate.cc:2730` `build_type5` / `generate.cc:2970` `build_type6` — monster nests/pits only approximated. C++ fills **every** cell of a 5x19 region from a 64-entry pool of `dun_level+10` monsters picked through the `vault_aux_*` filter (jelly/animal/undead/chapel/kennel/`vault_aux_treasure`/`vault_aux_clone`/`vault_aux_symbol`/orc/troll/giant/demon, with `randint(dun_level)` theme roll), rates the level +10 and rolls the special feeling; Rust `NestSpawn` fills only 33% (nest) / 50% (pit) of the room cells (`bevy/src/map.rs:713-722`), uses the 10-theme `nest_theme_ok` list without treasure/clone and with a placeholder symbol rule (`bevy/src/game.rs:134-149`, `"oTPUACZg".contains(g)` instead of the template-race comparison) and spawns at `bevy/src/game.rs:4951-4967`, without the pit's symmetric monster-level layout or the `symbol clone` depth gate.
  **Verified (2026-09):** `map::build_type5/6` (`map.rs:1786/1872`) fill every cell, use the 64/16-entry themed pools via `vault_aux_*` (`map.rs:3412`), the treasure/clone/symbol and depth gates, the +10 rating/special-feeling roll, and the pit's sorted every-other layout.
- [x] `generate.cc:2256` `build_type4` — the large inner-room room type is missing entirely. It carries 5 sub-layouts (inner room + monster, treasure vault with nested room + 80% object / 20% random stairs, pillar variants, checkerboard maze, four small rooms) and calls `build_small_room`, `add_door`, `fill_treasure` — none of which exist in Rust (`build_small_room`, `add_door`, `fill_treasure` = `[ ]`, no caller).
- [x] `generate.cc:4334` `build_type10` / `generate.cc:4295` `build_cavern` — fractal caves are not used for dungeons.
  `generate_hmap`/`generate_fracave` were ported, but only as the private helper `generate_eol_cave` (`bevy/src/map.rs:3028`) for q_eol. `DF_CAVE` (7 d_info records) is never read and `DF_CAVERN` (4 records) is mis-handled: Rust always builds a whole-level random-walk cavern whenever `CAVERN` is present (`bevy/src/map.rs:1049-1050`, `cavern_level` at `:598`), whereas C++ rolls `rand_int(dun_level/2) > DUN_CAVERN(30)` and drops a `build_cavern()` fractal cave into the centre of an otherwise normal rooms+corridors level; `DF_CAVE` instead makes every trivial room a type-10 fractal cave. Suggested landing: `bevy/src/map.rs`.
- [x] `generate.cc:5701` `build_type11` — random vaults are missing with all seven builders: `build_bubble_vault` (`:4630`), `build_room_vault` (`:4840`), `build_cave_vault` (`:4898`), `r_visit`/`build_maze_vault` (`:4966`/`:5067`), `build_mini_c_vault` (`:5139`), `build_recursive_room`/`build_castle_vault` (`:5228`/`:5439`), `build_target_vault` (`:5568`) and helpers `convert_extra` (`:4762`), `build_room` (`:4786`), `add_outer_wall` (`:5485`), `dist2` (`:5537`), `build_small_room` (`:4386`), `add_door` (`:4430`), `fill_treasure` (`:4484`). Type 11 has a 5% "unusual room" roll and requires depth >= 10; Rust never generates these vaults/pits.
- [x] `generate.cc:5794` `build_type12` — crypt rooms only approximated. Rust's "distorted" branch (`map.rs:679-703`) reproduces the `dist2` blob, but omits `add_outer_wall` (visible-wall lighting/marking), the `light` roll, the inner `build_small_room` + treasure + `vault_monsters` chance, and the depth >= 10 gate.
- [x] `generate.cc:5912` `build_tunnel` / `generate.cc:6169` `next_to_corr` / `generate.cc:6216` `possible_doorway` / `generate.cc:6246` `try_doors` — corridors and doors simplified.
  C++ uses wall-piercing with `FEAT_WALL_SOLID` bookkeeping, junction queueing and `try_doors` (open/broken/locked/jammed probabilities `place_random_door`, `generate.cc:919`). Rust draws L-shaped corridors between room centres and then `place_doors` heuristically picks <=3 boundary cells per room with flat 8%/17%/75% (`bevy/src/map.rs:2884`). `place_random_door` has no counterpart.
- [x] `generate.cc:1571` `destroy_level` — substantially different.
  C++ drops `randint(5)` blast epicenters of radius 16, **deletes monsters and objects** inside, and converts each valid grid with the 200-roll table (granite<20 / quartz<60 / magma<90 / sand<110 / floor) while clearing `CAVE_ROOM|CAVE_ICKY|CAVE_MARK|CAVE_GLOW`. Rust (`bevy/src/map.rs:936`) only nudges granite next to floor plus a random floor<->granite scatter, keeps vault contents alive, and never clears memory/light. Suggested landing: `bevy/src/map.rs`.
- [x] `generate.cc:684` `new_player_spot` / `generate.cc:448` `place_new_way` / `generate.cc:428` `is_safe_floor` — placement semantics.
  C++ drops the player on a random `cave_naked_bold` grid with no `CAVE_ICKY` (flat levels use `place_new_way`, which walks from a map edge to a legal floor cell), then (dungeon_stair default on) replaces the player grid with up/down stairs unless arriving via a branch; quest levels force up only. Rust picks the first staircase placed (`bevy/src/map.rs:1220-1258`) and `place_stairs` accepts any `walkable` cell (can land in deep lava), no icky check, no branch case. Suggested landing: `bevy/src/map.rs`.
  **Open (2026-09):** done — `map::random_naked_spot` (`map.rs`) picks a random non-permanent,
  non-icky walkable floor grid free of the level's pre-placed monsters/objects/traps for the
  player start (new_player_spot), falling back to the placed up-stair only if none is found.
- [x] `generate.cc:875` `place_random_stairs` / `generate.cc:919` `place_random_door` / `cave.cc:4313` `cave_clean_bold` — no counterparts; `place_random_stairs` (which requires `cave_clean_bold`: floor, no objects, not permanent) and `place_random_door` are only used by the missing type-4/type-11/vault code, so they disappear with those features.
- [x] `generate.cc:1069` `alloc_object` — object/feature budget differs.
  C++ places `randnor(DUN_AMT_ROOM=9,3)` room objects + `randnor(3,3)` objects + `randnor(3,3)` gold + `randnor(1,3)` altars + `randnor(2,3)` between + `randnor(1,3)` fountains, all with `cave_naked_bold` and room/corridor sets; Rust `item::scatter_objects` (`bevy/src/item.rs:2190`) places a flat `3+depth/2` items/gold with a 30% gold split, and `generate_dungeon_level_ex` places exactly one 10% altar, two between pairs and one fountain (`bevy/src/map.rs:1173-1194`). Suggested landing: `bevy/src/item.rs`/`map.rs`.
  **Done (2026-09):** `item::scatter_objects` now uses the faithful `randnor(9,3)+randnor(3,3)`
  objects + `randnor(3,3)` gold budget (`item.rs:3996`); the room/corridor split is the only
  divergence (the port's `Map` has no `CAVE_ROOM` bit).
- [x] `generate.cc:797` `place_fountain` — two concrete deviations in `bevy/src/map.rs:900-932` (and again in `bevy/src/input.rs:790-811`):
  * only `TV_POTION` is drawn from; `TV_POTION2` FOUNTAIN entries exist and are live (Cure Light/Serious/Critical/Insanity at `lib/edit/k_info.txt:5218-5251`, `F:FOUNTAIN` on each), so those fountain flavours are never generated or drunk (the C++ `special` encoding is `sval + SV_POTION_LAST`);
  * draughts are rolled as `rng.gen_range(3..=12) + 3` = 6..15 instead of `damroll(3,4)` = 3..12.
  **Verified (2026-09):** `place_fountain` (`map.rs:4024`) draws TV_POTION + TV_POTION2 with the `sval + SV_POTION_LAST` encoding and `damroll(3,4)`, and the input.rs fountain consumer decodes both tvals (`input.rs:1648-1684`).
- [x] `generate.cc:846` `place_between` — C++ pairs the chosen grid with a *random naked floor anywhere* on the level; Rust picks two random room cells (`map.rs:1179-1193`, can silently skip when `a == b`). Same-level gate pairing is otherwise fine.
- [x] `generate.cc:974` `alloc_stairs` — Rust equivalent exists but branch stairs (`cave.special = branch`) are only written for the branch/fbranch calls; quest (`is_quest`) and town (`dun_level==0`) stair-type forcing are missing (see `is_quest`).
  **Verified (2026-09):** `generate_dungeon_level_for(..., is_quest)` turns every requested down staircase into an up one (`map.rs:4493`) and depth 0 keeps the down feature; branch stairs still record `Map.special` (C++ sets `special = branch` for all, 0 for the rest).
- [x] `generate.cc:7403` `cave_gen` — empty-level lighting/feelings.
  C++ `if (empty_level && (randint(DARK_EMPTY=5) != 1 || randint(100) > dun_level)) wiz_lite()` lights most arenas. Rust handles DF_EMPTY only as "fill with floors" (`bevy/src/map.rs:1052-1055`) and never lights them. Also `get_level_flags()` (`@:depth:F:...` per-level flags) is not merged into the generation flags for normal generation (only `special_at` reads them). Suggested landing: `bevy/src/map.rs`.
- [x] `generate.cc:6480` `level_generate_dungeon` / `generate.cc:8127` `generate_cave` — surrounding semantics:
  * `is_quest(dun_level)` must forbid destroyed levels (`generate.cc:6537`) and force up-only stairs (`alloc_stairs`, `new_player_spot`); Rust's `is_quest` equivalent doesn't exist (`PlotQuest.rands` is only consumed in `populate_level`, `bevy/src/game.rs:5032-5037`), so random-quest levels can be destroyed and give free down stairs.
  * feeling: `if (good_item_flag && !options->preserve) feeling = 1`, the 1000-turn recharge (`turn - old_turn`) and special-level `good_item_flag`/`rating += 40` (`generate.cc:7969-7975`) are not modelled. Rust `build_spec_level` returns `feeling = 0` and `feeling_from_rating` is only computed for the ordinary generator (`bevy/src/map.rs:1282-1289`), so special levels never show "special feeling".
  * `ADJUST_LEVEL_*` is applied by C++ to the *spawned monster's* level (`monster2.cc:2432-2449`), while Rust uses it to choose the allocation level (`bevy/src/game.rs:6137-6154`); the race pool therefore differs in Moria/Nogrod/etc. (`DF_ADJUST_LEVEL_PLAYER` is never consumed in C++ either).
  * `max_vault_ok` reduction when a panel dimension is missing (`cave_gen:7429-7433`) is not modelled.
  **Open (2026-09):** `is_quest` destroyed-level/up-only rules and per-spawn `ADJUST_LEVEL`
  (`game::dungeon_adjust_level` -> `monster_set_level`, `game.rs:8476`) are wired; special levels
  now set `good_item_flag`/`rating += 40` and the special feeling (`game.rs::build_spec_level`).
  The `max_vault_ok` panel reduction is not modelled — the port has a fixed full-size map, so the
  C++ reduction (`cave_gen:7429-7433`) can never apply (max_vault_ok is always 2).
- [x] `generate.cc:7910` `build_special_level` — the s_*.map layout is built (`bevy/src/game.rs:4630`), but most `@:` level flags (`lib/edit/d_info.txt:95-99,104-108,112-116,158-162,389-393,439-443,525-529,681-685`) are not consulted. `NO_GENO` (`game.rs:2194`) and `NO_NEW_MONSTER` (`game.rs:8433`) are handled, and `NO_BREATH`/`WATER_BREATH` happen to merge `spec.flags` (`game.rs:8163-8168`), but `ASK_LEAVE` is checked on the *dungeon* (`bevy/src/input.rs:2878`) although all 7 uses are `@:`-level flags, so the "are you sure you want to leave forever" prompt never fires; `NO_TELEPORT` (`item.rs:678-682`) and `NO_STAIR` likewise read only dungeon flags. `good_item_flag/rating += 40` missing as above. Suggested landing: `bevy/src/input.rs`/`game.rs`.
  **Open (2026-09):** all items are closed — `ASK_LEAVE`/`NO_TELEPORT` read the merged `@:` flags
  (`input.rs:7076`, `input.rs:3070`), `build_spec_level` honours `NO_STAIR` (no guaranteed
  staircases) and sets `good_item_flag`/`rating += 40`/feeling = 1.

### dungeon.cc — world ticks

- [x] `dungeon.cc:392` `apply_effect` — terrain self-damage is not implemented.
  C++ runs it at the player's grid every 10 game turns; f_info `E:` entries are live data: deep lava `E:-1d2:1:FIRE`, shallow lava `E:-1d1:1:FIRE` (both walkable FLOOR terrain), Great Fire `E:150d2:1:HELL_FIRE`, blazing fire `E:-1d2:1:FIRE`, ice `E:1d1:50:ICE`, nether mist `E:1d1:40:NETHER` (`lib/edit/f_info.txt:767/777/812/944/1078/1297`). Rust never parses those fields (`TerrainDef`, `bevy/src/data.rs:10-38`) and has no such tick; standing in deep lava is harmless. Suggested landing: `bevy/src/game.rs` upkeep.
- [x] `dungeon.cc:445` `process_world_corruptions` — the random-teleport corruption is implemented (`bevy/src/game.rs:8526-8543`) but always fires the "Your corruption takes over you, you teleport!" branch; C++ first offers `get_check("Teleport?")` (accept silently teleports), so the player-choice branch and message are missing. The anti-teleport mana drain is present (`bevy/src/game.rs:7764-7777`). Suggested landing: `bevy/src/game.rs` modal prompt.
- [x] `dungeon.cc:725` `process_lasting_effects` — clouds only damage monsters.
  C++ applies `project(0,0,y,x,dam,type,PROJECT_KILL|PROJECT_ITEM|PROJECT_HIDE)` per affected grid, i.e. the player standing in a lasting fire/poison/water cloud takes damage and items on the grid can be destroyed; the effect is also suppressed on grids without LOS and supports directional waves (EFF_DIR1..9) and player-centred storms (EFF_STORM). Rust (`bevy/src/game.rs:9838-9905`) walks monsters inside a Chebyshev disc, applies `gf_monster_effect`, never calls `apply_gf` for the player, never calls `floor_damage`, ignores LOS, and `Cloud.dir` is only a half-plane test rather than the per-direction octant front. Suggested landing: `bevy/src/game.rs`.
- [~] `dungeon.cc:509` `process_world_gods` + `dungeon.cc:488` `grace_delay_trigger` — passive piety of the base gods is missing.
  Every 15 world turns Varda/Ulmo/Aule/Mandos adjust grace by race (light, Edain, Dwarves, LostSoul/vampires...), inventory (tridents, axes/hammers) and praying state, and Aule's praying can grant free Stone Skin (`set_shield` with grace-scaled dice). Rust has none of this: `bevy/src/game.rs:6700-6900` only does stat boons, Manwe blessings, Melkor demon aid and Yavanna healing; `grep grace_delay` = 0 hits. Suggested landing: `bevy/src/game.rs::god_sync`.
- [x] `dungeon.cc:272` `regen_monsters` — monster hit-point regeneration every 100 turns is missing (both the carried symbiote at `INVEN_CARRY` pval2->pval3 and every monster: `maxhp/100` per 100 turns, doubled with `RF_REGENERATE`, skipped while bleeding/poisoned). Rust never heals monsters (`bevy/src/game.rs` only decrements corpse/egg fuel). Suggested landing: `bevy/src/game.rs` upkeep.
- [x] `dungeon.cc:152` `regenhp` / `dungeon.cc:210` `regenmana` — replaced by a flat +1 hp / +1 mana every 10 (5 with REGEN) turns (`bevy/src/game.rs:7516-7534`). Missing: the `(mhp*percent+PY_REGEN_HPBASE)>>16` + fractional accumulator formulas, food thresholds (`PY_REGEN_WEAK/FAINT/0`), the faint roll and message, resting doubling, `p_ptr->regenerate` doubling, INT mana bonus (`adj_str_blow[INT]*3`), pet-upkeep mana penalty (`upkeep_factor`), poison/cut/`cave_no_regen` no-heal rules, and Yavanna grass regen (`dungeon.cc:1380-1580`).   Suggested landing: `bevy/src/game.rs::upkeep`.
  **Done (2026-09):** the hp/mana formulas, 16.16 accumulators, food thresholds, faint roll,
  resting/REGEN doubling, poison/cut/`cave_no_regen`, Yavanna grass and INT mana bonus are in
  (`game.rs` upkeep); the `upkeep_factor` pet drain is implemented from the monster query's
  charmed/PET monsters and `did_nothing` (resting) now gates the god piety trickle
  (`game.rs::upkeep`).
- [x] `dungeon.cc:107` `recharged_notice` — an item inscribed `!!` never reports "Your X is recharged" on recharge. Rust inscriptions exist but no recharge notification (`grep "recharged"` only item recharge internals). Suggested landing: `bevy/src/item.rs`/`game.rs` rod/activation recharge loop.
- [x] `dungeon.cc:2081-2090` `process_player` suffocation — Rust combines `WATER_BREATH`/`NO_BREATH` into one check gated on `water_breath` only (`bevy/src/game.rs:8166-8172`); `NO_BREATH` must be gated on `magical_breath` (C++ builds `magical_breath` from `TR_MAGIC_BREATH`, Manwe grace>15000 while praying and Air skill 50, `xtra1.cc:2371/2381/2611/3197`; Rust computes `tot.magic_breath` at `item.rs:558-562` but never reads it, folding Air>=50 into `water_breath` instead), and the distinct messages ("You cannot breathe water!" / "There is no air here!") are missing. Suggested landing: `bevy/src/game.rs`.
- [~] `dungeon.cc:4207-4230` `dungeon` — player-arrival stair creation (`create_down_stair`/`create_up_stair` from cmd2) is not modelled; Rust levels are snapshotted so stairs always exist, but a level re-entered without its matching feature gets a synthetic stair (`bevy/src/game.rs:12370-12382`) which is a different rule. Low impact, snapshot design.

### wild.cc — wilderness/towns

- [x] `wild.cc:481-570` `wilderness_gen` border `mimic` — C++ keeps the map border as permanent walls whose `mimic` shows the neighbour area's terrain; Rust copies the neighbour's *terrain* into row/column 0 and MAP-1 (`bevy/src/map.rs:2241-2277`), making the border walkable/stair-placeable in a way the original forbids. Also C++ clamps the resident count to the floor count (`lim > hack_floor - 1`) and calls `player_place(oldpy,oldpx)` (edge continuity); Rust uses `pending_pos` for walking transitions but always re-centres on other entries.
  **Fixed this session:** `Map.mimic` added; `generate_wild_area` now makes the border `T_PERMANENT` with the neighbour terrain as `mimic`, and `step_player` tests the mimicked feature before crossing (`player_can_walk_t`), exactly like `move_player_aux`. The resident clamp and `player_place` continuity are `populate_surface`/`level_transition` concerns (see NEEDS OTHER FILE).
- [x] `wild.cc:653` `reveal_wilderness_around_player` — radius test is Euclidean-squared `< r*r` (`bevy/src/game.rs:2110-2120`) instead of `distance() = max + min/2 < w`; diagonal reach differs by up to one cell. The `h != 0` rectangular mode is unused by data.
- [x] `wild.cc:1210` `town_gen_hidden` — Rust places 1% townspeople over the hidden layout too (`bevy/src/map.rs:1984-2007` runs for all three layouts) while C++ only calls `place_townspeople` from `town_gen_hack`/`town_gen_circle`; the hidden dungeon town is extra-populated.
  **Verified this session:** the townspeople pass is gated by `free.is_empty()` and `town_gen_hidden` never fills `free`, so hidden towns get none (fixed by an earlier session).
- [x] `wild.cc:157` `generate_area` — mostly ported (`bevy/src/map.rs:2215`), but `p_ptr->town_num` and the `mimic` border plumbing are not modelled (Rust uses `map.town`); `wf.level` for object/monster generation is applied in `populate_surface` (`bevy/src/game.rs:5202`), which is fine. `[>]` only for the border.
  **Fixed this session** (border plumbing, see above); `map.town` remains the town_num equivalent.

## UNCERTAIN

- `cave.cc:50` `distance` — the exact helper exists as `map::pref_distance` (`bevy/src/map.rs:1406`) but a large share of C++ call sites were replaced by `map::chebyshev` (monster AI, summoning, geomancy, fates, …). Where the original uses the max+min/2 approximation the Rust result can differ by one cell; this audit did not enumerate every call site.
- `generate.cc:4295` `build_cavern`/`4334` `build_type10` — `generate_hmap`/`generate_fracave` are faithfully ported for q_eol (`bevy/src/map.rs:3028`), so the algorithm itself is not gone, only unreachable for dungeons. Marked `[>]`/`[ ]` above.
- `cave.cc:2842` `update_mon_lite` — monster light is only a render-time effect (`bevy/src/render.rs:283`); whether the final visibility/targeting should be affected is treated as part of the FOV gap.
- `cave.cc:3462-3550` `update_flow` / `update_flow_aux` — noise-flow field absent but gated behind `options->flow_by_sound`, which defaults **false** (`src/options.hpp:50`); monster hearing is implemented through the melee2.cc noise roll (`bevy/src/game.rs:8927-8950`). Marked `[~]`.
- `cave.cc:4085` `disturb` / `cave.cc:4132/4145` `disturb_on_*` — resting/running cancel semantics; Rust keypress handling resets `TurnState.resting` (`bevy/src/input.rs:211-213`), judged equivalent. Marked `[~]` (front-end).
- Feature hooks (`process_hooks_new(HOOK_GEN_LEVEL/HOOK_BUILD_ROOM1/HOOK_LEVEL_REGEN/HOOK_LEVEL_END_GEN/HOOK_GILD_LEVEL/HOOK_WILD_GEN/HOOK_FORBID_TRAVEL)`) have no generic port; the known quest uses were checked by hand and are implemented, but any hook not re-derived from a q_*.cc call site would be invisible here.
- `DF_NO_STAIR` semantics: C++ appears inverted at `dungeon.cc:4222` (`if (!(dungeon_flags & DF_NO_STAIR)) create_* = false`) — the Rust snapshot model bypasses the whole arrival-stair system, so this is not reported as a gap.

## COVERAGE SUMMARY

- 总函数 205；[x]=83, [>]=52, [ ]=38, [~]=32（原审计数）
- By file: cave.cc 66 (x22 >15 ?1 ~28), generate.cc 95 (x41 >26 ?28),
  gen_maze.cc 2 (?2), gen_evol.cc 2 (?2), dungeon.cc 22 (x6 >7 ?5 ~4),
  wild.cc 18 (x14 >4).
- 本会话后（approximate）：gen_maze.cc 2 → [x]; gen_evol.cc 2 → [>];
  generate.cc 的 room/streamer/destroy/type* 项大量转为 [x]/[>]（见上）。


## DONE THIS SESSION

Implemented in `bevy/src/map.rs` (no changes to other files except none):

- [x] **Maze generator** `G:maze` (dungeon 18): `maze_dig` + `build_maze_level`
  port `gen_maze.cc` bit-for-bit (sentinel maze array, 50% turn, wall
  closing against the start cell, the `(h/2)-2 x (w/2)-2` translation).
  `generate_dungeon_level_ex` dispatches on `DungeonDef.generator`; unknown
  generators (incl. the dead `G:dungeon2` of the Test dungeon) fall back to
  the standard generator instead of the original's `at()` abort.
- [x] **Life generator** `G:life` (Heart of the Earth): `build_life_level`
  (45% floor fill, CAVE_ROOM|GLOW|MARK) and a faithful `evolve_level`
  (starved/suffocated/spawned 8-neighbour rule, the noise pass with the
  `cw > cf` majority choice, permanent/player-grid skips).  The port cannot
  skip grids that hold objects/monsters (they are ECS entities, not map
  data).  **Hook needed (game.rs):** `bevy/src/game.rs` still runs the old
  "8 random floor/fill flips" placeholder for `EVOLVE`; replace it with
  `map::evolve_level(&mut map, gd, &floors, &fills, px, py, true, map.w,
  map.h, rng)` (`init_feat_info(d, depth)` gives the tables).
  **Wired (2026-09):** `game.rs:12566` now calls `map::evolve_level` with
  `init_feat_info` instead of the placeholder; the only residual is the
  object/monster-grid skip noted above (tracked in the GAPS bullet).
- [x] **build_streamer2** with pools + correct `DF_NO_STREAMERS` gating:
  rivers are now gated by `DF_WATER_RIVER(S)`/`DF_LAVA_RIVER(S)` only (with
  `rand_int(4)` for the singular forms and the 3+d2 / 2+d2 multi-loops),
  and `build_streamer2` implements streams (9 cells/step, direction change
  1/20) and the diamond pool variant, skipping CAVE_ICKY/permanent grids.
  Note: the original's `!(dun_level <= 33)` condition makes the lava branch
  unreachable; the port reproduces the reachable behavior (water only at
  depth > 33) and keeps the lava branch as dead code for fidelity.
- [x] **Room allocation model** `room_alloc`/`check_room_boundary` with the
  11x11 block map, 100-cent cap, `roomdep` depth gates and the crowded
  (nests/pits) limit.  Room loop matches `level_generate_dungeon` including
  the two nested `DUN_UNUSUAL` rolls, the `max_vault_ok`/panel caveat (panels
  are not modelled: both vault slots are always available), the destroyed
  level branch and the `DF_CAVE`/circular trivial rooms.
- [x] **Room types**: 1 (walls + pillar/ragged variants), 2 (overlapping),
  3 (cross with all four sub-cases), 4 (all five sub-layouts: inner room,
  nested treasure room, pillars, checkerboard maze, four rooms), 5 (nests:
  real 64-race pools through the `vault_aux_*` filters incl. treasure/clone/
  symbol, full room fill, +10 rating, special-feeling roll), 6 (pits: 16-race
  sorted every-other selection and the exact symmetric layout), 7/8 (v_info
  vaults through room_alloc), 9 (vertical oval, `distance()` ring), 10
  (fractal caves via a reusable `generate_hmap`/`generate_fracave` port), 11
  (all seven random-vault builders: bubble, room, cave, maze, mini
  checkerboard, castle, target), 12 (crypt with `dist2`, `add_outer_wall`,
  inner vault).  Rooms carry CAVE_ROOM/GLOW state and vault content is
  pushed as concrete `VaultSpawns` monsters/objects.
- [x] **build_tunnel / next_to_corr / possible_doorway / try_doors /
  place_random_door**: wall piercing bookkeeping with FEAT_WALL_SOLID
  restored to outer walls afterwards, junction queueing, corridor
  pre-emptive termination, and the 75%/crossroad/T-junction locked-door
  branches; `place_random_stairs` used by type 4.
- [x] **destroy_level**: randint(5) blast epicenters of radius 16 using
  `distance() < 16`, per-grid 200-roll terrain table (granite/quartz/magma/
  sand/floor), perma-grid guard, CAVE_ROOM/ICKY/MARK/GLOW clearing, and the
  deferred vault spawns inside the blasts are dropped (the port's stand-in
  for delete_monster/delete_object).
- [x] **place_fountain**: TV_POTION + TV_POTION2 with the
  `sval + SV_POTION_LAST` encoding and damroll(3,4) draughts.
  **Consumer gap (input.rs):** `fountain_command` still only resolves
  `TV_POTION` svals < 64; fountains generated with a TV_POTION2 flavour
  report "the water is strange and useless" until input.rs decodes
  `sval >= SV_POTION_LAST`.
- [x] **place_between**: partner is now a random naked floor grid anywhere
  on the level (not a second room cell), pairs recorded in `Map.between`.
- [x] **`Map.icky`** added (CAVE_ICKY equivalent): set by every vault
  builder / random vault / fractal cave and consumed by streams,
  `cave_naked_bold`/alloc_object and place_new_way.  Consumers in
  item.rs/modal.rs (teleport landing, fate drops) are still not wired.
- [x] **Terrain `E:` damage**: `terrain_effect` in map.rs implements the
  six live f_info entries (85 deep lava / 86 shallow lava / 90 ice / 102
  nether mist / 178 Great Fire / 205 blazing fire) including the
  `dice == -1 -> player level` substitution and frequency check.  **Hook
  needed (game.rs):** call it at the player's grid every world tick
  (`dungeon.cc:392 apply_effect` is run every 10 game turns in
  `process_player`) and route the damage through `apply_gf`.
  **Wired (2026-09):** `game.rs:12586` calls `map::terrain_effect` every
  10 turns and routes the damage through `apply_gf` + `player_hurt`.
- [x] **`get_level_flags`**: `@:<depth>:F:` flags are merged with the
  dungeon flags for generation (`NO_DOORS`, `NO_STREAMERS`, `CAVERN`,
  `EMPTY`, `FORCE_DOWN`, ...).
- [x] **Empty (arena) level lighting**: `if EMPTY && (randint(DARK_EMPTY)
  != 1 || randint(100) > depth) wiz_lite()` (approximated with
  `Map::reveal_all`).
- [x] **place_new_way / alloc_stairs**: flat dungeons now place their
  way up/down features with a faithful `place_new_way` (edge start,
  safe-floor scan, room bending); the player entry uses `new_player_spot`'s
  random naked (non-icky) grid for every generator.
  **Done (2026-09):** `map::random_naked_spot` replaces the `gen.start`
  entry pick.
- [x] **F: random markers**: `parse_f_marker`/`parse_f_markers` implement
  the `F:<letter>:<terrain>:<cave_info>:<monster>:<object>:<ego>:<artifact>:
  <special>:<mimic>:<mflag>` grammar with the `*` random bits.
  **Converter gap (fixed by the data-pipeline session):** the converter now
  preserves the `*` markers as `random_monsters`/`random_objects` plus
  `mimics` per cell on `QuestMapDef`/`SpecLevelDef`; `map.rs` consumes them
  (see DONE THIS SESSION).

### Still open (not in map.rs ownership or out of this session's scope)

- `replace_all_friends`/`save_all_friends` (pets are lost on transition) —
  game.rs/save.rs.
- FOV/LOS/vinfo corner-exact `los`, monster-light→visibility — map.rs is
  only partially able to fix these (needs monster positions), game.rs/render.rs.
- `wiz_lite` object memorization, `cave_valid_bold` artifact guard,
  `place_floor_convert_glass` callers, teleport consumers of `Map.icky` —
  game.rs/modal.rs/item.rs.
- `alloc_object` item budget (objects/gold/altars were approximated; the
  room/corridor altar/between/fountain allocation now uses the faithful
  `alloc_object` search), `regen_monsters`, `recharge` notices,
  `process_world_gods`, regen formulas, suffocation breath split — game.rs.
- `is_quest` gating (destroyed levels / up-only stairs), `ADJUST_LEVEL_*`
  semantics, `max_vault_ok` panel reduction, `build_special_level` `@:`
  level flags other than generation — game.rs/input.rs.
- `wild.cc` reveal radius (`game.rs`), resident-count clamp and
  `player_place` edge continuity (`game.rs`).

## DONE THIS SESSION

Implemented in `bevy/src/map.rs`, `bevy/src/input.rs`, `bevy/src/mimic.rs`:

- **Terrain data consumers (`TerrainDef` fields)**
  - `web`: `player_can_walk` (input.rs) now blocks webs for everyone except
    a possessed RF_SPIDER body or the Spider mimic shape (wraiths still pass
    via CAN_PASS, flyers are blocked like the original).
  - `can_run`/`dont_notice_running`: the hardcoded `CAN_RUN` and
    `DONT_NOTICE_RUNNING` tables in input.rs are gone; `see_obstacle_grid`
    and `run_test` read the parsed flags.  `see_obstacle_grid` also restores
    the C++ lava/dark-pit fall-through (levitation skips all five, fire
    immunity only the lava cases).
  - `block_desc`/`tunnel_desc`: blocked-movement and tunneling messages use
    f_info `D:2`/`D:1` ("There is a web blocking your way.", "You chop away
    at the tree.", ...) with the original defaults.
  - `TerrainEffectDef.effects`: `terrain_effect` frequencies corrected to the
    real x10 values (ice 500, nether 400, lava 10) and pinned to the parsed
    data by the new `terrain_effects_match_f_info_data` test.
  - `mimic`: `Map.mimic` + `Map::display_terrain` implement `c_ptr->mimic`
    (f_info default mimic fallback); used by `is_wall` and the wild border.
  - `support_light` is never read by base C++ (`FF_SUPPORT_LIGHT` only
    appears in the flag list) and `attr_multi` is a `map_info` render
    shimmer; both are renderer-only and left to render.rs.
- **Wilderness border** (wild.cc wilderness_gen): `generate_wild_area` sets
  the rim to `T_PERMANENT` with the neighbour area's terrain as `mimic`;
  `step_player` uses the mimicked feature for the exit check
  (`player_can_walk_t`).  Wild-overview `wild_step` now applies the
  `player_can_enter` wild_mode safety checks (deep-water weight, lava fire
  protection).
- **recursive_river/add_river**: full perturbed-midpoint fractal with
  junction splits, `DUN_WAT_RNG=2` width, `feat2` border + `feat1` centre,
  permanent-skip, lava glow and CAVE_ICKY marking.
- **Fixed-map random markers**: `quest_level` (tables.cc `quest[].level`),
  `roll_map_markers` and `VaultSpawns.random_monsters/random_objects`;
  `generate_quest_level` records the `F:...:*N` markers of quest maps and
  populates `Map.mimic` from `QuestMapDef.mimics`.
- **FOV**: `compute_fov` scans `MAX_SIGHT = 20` now, so permanently lit
  grids anywhere in LOS are visible (`update_view`); torch radius semantics
  unchanged.
- **Helpers for callers**: `Map::wiz_dark`, `is_wall`,
  `place_floor_convert_glass`, `cave_valid_bold`.
- **mimic.rs `calc_body`**: exact xtra1.cc body-part sum
  (race + subrace + class, Bear zeroing, `max_body_part` caps, extra limbs),
  `slot_usable_body`, and body-aware `enforce_body`/`drop_unusable_slots`
  (DeathMold loses head/leg slots, possessed bodies drop gear for missing
  parts).
- **incarnate_monster_attack** (input.rs `melee_possessed`): body-level
  (rlev) hit checks, RBE_* → GF conversions (`gf_monster_effect`), OLD_SLEEP
  power, shatter quake, touched AURA_FIRE/ELEC against the body, EAT_*
  blink-flee (`MAX_SIGHT*2+5`), RF_NEVER_BLOW guard.
- **Fountain flavours** (input.rs): unknown fountains now roll TV_POTION2
  FOUNTAIN entries and the stored `sval + SV_POTION_LAST` encoding is
  decoded for both bottle-fill and drink; draughts are `damroll(3,4)`.

## NEEDS OTHER FILE

- **game.rs `build_spec_level`**: d_info `@:` special levels ignore
  `SpecLevelDef.random_monsters/random_objects/mimics`.  Call
  `map::roll_map_markers(0, &sl.random_monsters, &sl.random_objects, (ox, oy),
  &mut vault, rng)` and spawn `vault.random_monsters` (a monster of that
  level, `spawn_one_monster`) / `vault.random_objects` (`make_object_ex`
  with the good/great flags), and copy `sl.mimics` into `Map.mimic`.
- **game.rs `build_quest_level`**: same for the quest maps' vault records
  (`gen.vault.random_monsters`/`random_objects`; `generate_quest_level`
  already fills them) and use `Map.mimic` when drawing/describing.
- **game.rs `replace_all_friends`**: on `level_transition`, snapshot
  `MonsterSave::from_monster_with_extra` for every `m.companion` before the
  old level is stored and respawn them (`spawn_companion_hp` plus the extra
  component) near the player on the new level via a free-spot search;
  skip while `wild_mode`/`old_wild_mode`.
- **game.rs `populate_surface`**: clamp residents to `floor_count - 1` and
  restore `player_place(oldpy, oldpx)` edge continuity.
- **render.rs**: use `Map::display_terrain(&gd, x, y)` instead of
  `terrain_at` in `cell_render` so border/map `mimic`s display; monster
  light should also raise `Map.visible` within `MAX_SIGHT`, not `FOV_RADIUS`.
  **DONE (this session).**
- **modal.rs `map_area_ctx`/clairvoyance**: use `map::is_wall` (doors are not
  walls); **modal.rs/item.rs** remove-map callers (Booze, Alter Reality) use
  `Map::wiz_dark`. **DONE: map_area_ctx uses `is_wall` + plain-floor
  (`is_floor && !remember`); the Booze potion calls `map.wiz_dark()` after its
  teleport. Alter Reality rebuilds the level (`turn.regen_level`).**
- **game.rs/item.rs kill-wall/geomancy**: use
  `map::place_floor_convert_glass` so destroyed glass walls become molten
  glass, and `map::cave_valid_bold(..., has_artifact)` before destroying a
  grid. **PARTIAL (modal.rs): `KILL_DOOR` and the new `WAVE`/`WATER`
  conversions go through `place_floor_convert_glass`; the game.rs kill-wall
  callers remain.**
- **game.rs `summon_monsters`/`populate_surface`**: use `map::scatter_pos`
  instead of the ±2 box so summons land in LOS and within `d`.
- **item.rs `teleport`/fate drops**: skip `Map.icky` cells (and charm/guardian
  cells) as C++ does. **DONE: `item::teleport` skips permanent and
  `Map.icky` cells (C++ teleport_player).**
- **game.rs `generate_dungeon_level`**: pass `DungeonDef.theme` into
  `item::make_object_themed` for level object generation.

## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- render.rs `sync_tiles`: the FOV radius uses `game::player_lite` (calc_torch: tim +2, holy +1,
  cap 5); `cell_render` draws `map.display_terrain` so border/vault mimics show; monster-carried
  light follows `update_mon_lite` (MAX_SIGHT + 1, only already-viewed cells, opaque wall skipped
  when the monster is not in LOS).
- modal.rs `map_area_ctx` now uses `map::is_wall` and `cave_plain_floor_grid` semantics
  (`is_floor && !remember`) instead of `opaque`.
- modal.rs `gf_terrain`: `GF_KILL_DOOR` and a new `GF_WAVE`/`GF_WATER` branch use
  `map::place_floor_convert_glass`; `is_wall` has its first consumer.
- item.rs `teleport` skips `Map.icky` and permanent grids; item.rs `Booze` calls `Map::wiz_dark`
  (lose_all_info) after its random teleport.
- Remaining 05-map entries (spec-level markers, replace_all_friends, populate_surface, summon
  scatter, theme objects) are all in game.rs.

## WIRED THIS SESSION (game.rs/map.rs/save.rs ownership)

Status of the `NEEDS OTHER FILE` bullets above (game.rs side):

- **`build_spec_level` markers**: `map::roll_map_markers(0, ...)` now runs for
  `@:` special levels, `sl.mimics` populate `Map.mimic`, and the recorded
  random monsters/objects are spawned (`random_monster_of_level`,
  `make_object_themed`).
- **`build_quest_level` markers**: `gen.vault.random_monsters/random_objects`
  are spawned the same way.
- **`replace_all_friends`**: `level_transition` keeps a `companion` snapshot
  across the transition (companions are excluded from the outgoing level
  snapshot) and `place_friends` drops them beside the player on both restored
  and freshly generated levels (`get_pos_player(5)`).
- **`populate_surface`**: resident count clamped to `floor_count - 1`.
- **summon scatter**: `summon_monsters` now uses `map::scatter_pos`
  (`d = i/15 + 1`, LOS, glyphs excluded).
- **theme objects**: `populate_level`'s vault objects call
  `item::make_object_themed` with the dungeon `O:` theme.
- **kill-wall callers**: the two monster glyph/rune breaks in `monster_turns`
  use `map::place_floor_convert_glass`; `cave_valid_bold(..., has_artifact)`
  remains unwired (earthquake callers have no floor-item query).
- **`generate_dungeon_level`**: now split into
  `generate_dungeon_level` (C++ default options, tests) and
  `generate_dungeon_level_for` (live `Options` + `is_quest`), consuming
  `small_levels` (1/6), `empty_levels` (1/15), `always_small_level`,
  `auto_scum` and the `@:` `is_quest` stair/destroyed rules.
- **`@:` level flags**: `game::level_has_flag` (dungeon + `special_at` flags)
  used for GF_NEXUS `NO_TELEPORT`; `ASK_LEAVE`/`NO_STAIR` stay input.rs.


## WIRED THIS SESSION (game.rs/input.rs/modal.rs)

- `dungeon.cc:392` `apply_effect`: verified the terrain `E:` hook is called every 10 world turns through
  `apply_gf` (game.rs `monster_turns`), checkbox flipped.
- `dungeon.cc:445` `process_world_corruptions`: the random-teleport corruption now raises a
  `Modal::CorruptTeleport` "Teleport?" prompt (deferred through `TurnState.corrupt_teleport`); accepting
  teleports silently, refusing prints "Your corruption takes over you, you teleport!" and still
  teleports, exactly as the original `get_check`.
- `dungeon.cc:725` `process_lasting_effects`: clouds now damage the player through `apply_gf` and use
  `PROJECT_ITEM`-style floor damage at the player's cell; every grid test requires LOS and the
  directional half-plane/octant front, and waves still expand.
- `dungeon.cc:152/210` `regenhp`/`regenmana`: replaced the flat +1 hp/+1 mana with the C++
  `(mhp*percent + PY_REGEN_HPBASE)>>16` / `(msp*percent + PY_REGEN_MNBASE)>>16` formulas and 16.16
  fractional accumulators (`TurnState.regen_hp_frac/regen_mana_frac`), the food thresholds
  (WEAK/FAINT/STARVE), faint roll, resting/REGEN doubling, poison/cut/`cave_no_regen`, Yavanna grass and
  INT mana bonus. Still missing: the `upkeep_factor` pet penalty (`total_friends` needs the monster
  query) and `did_nothing` accounting.
- `dungeon.cc:509` `process_world_gods`: marked `[~]` — Varda/Ulmo/Aule/Mandos are Theme-only gods
  (their `deity_info` records carry no `MODULE_TOME`), so the function is unreachable in the base
  module.
- `wild.cc:653` `reveal_wilderness_around_player`: `Wilderness::reveal` now uses
  `distance() = max + min/2 < w` instead of the Euclidean squared test.
- `@:` level flags: `input.rs` now reads `ASK_LEAVE` and the probability-travel `NO_TELEPORT` test
  through `game::level_has_flag` (dungeon + special-level merge). `NO_STAIR` remains unconsumed because
  the port snapshots levels and always materialises both staircases.
- `regen_monsters`: checkbox flipped (implemented in an earlier session, verified in `monster_turns`).
- Arrival stairs (`dungeon.cc:4207`) marked `[~]`: the snapshot level model replaces
  `create_up/down_stair`, so `NO_STAIR` never applies at arrival.

## DONE THIS SESSION (final, 2026-09-16)

All three remaining bullets verified against the current code and closed:

- **`replace_all_friends`/`save_all_friends`**: `level_transition` snapshots every
  `m.companion` monster (`MonsterSave::from_monster_with_extra`, excluding it from the
  outgoing level snapshot) and `place_friends` respawns them beside the player with the
  original `get_pos_player(5)` spot search (500 tries, `pref_distance` in `[2,5]`,
  `cave_naked_bold` floor + non-icky + unoccupied); `carry_friends = !old_wild_mode &&
  !ps.wild_mode` matches the `wild_mode` early-outs of both C++ functions. Both the
  freshly generated and restored-snapshot paths place them; MonsterExtra (exp/nice/repro)
  travels with the snapshot. Equivalent.
- **`recharged_notice`**: already implemented in `upkeep` — rods that reach `pval2`,
  pack activatables whose `timeout` hits 0 and worn activatable items inscribed `!!` all
  print "Your X is [are] recharged." (dungeon.cc:2366/2420/2434). Verified, no change.
  Also surfaced `maintain_sum` partial-summon upkeep in the same upkeep pass.
- **suffocation**: already implemented at `game.rs:11889-11903` — `magical_breath` is
  `tot.magic_breath || SK_AIR >= 50 || (Manwe grace > 15000 while praying)` and the two
  distinct messages plus `damroll(3, level)` damage are present; `water_breath` keeps the
  `TR_MAGIC_BREATH → also water breath` behaviour through `item.rs`. Verified, no change.

## FINAL RECONCILIATION

Reconciliation pass 2026-09-16 (third pass, spell/game-side session): the three `[>]`
bullets were re-checked against the current modal.rs/item.rs and are all closed by the
parallel session; this report now has no `[>]` bullets.  The 2 `[~]` entries are
unchanged.

`[x]` this pass:
- `cave_valid_bold` (`cave.cc:420`) — `modal.rs::destroy_area` now tests
  `has_artifact` (any floor stack with `artifact != 0`) together with `permanent`
  *before* despawning the stacks (`modal.rs:13986-13990`), so artifact grids keep both
  terrain and items, matching the C++ ordering.
- `place_floor_convert_glass` (`cave.cc:3875`) — PWR_PASSWALL's emerge-in-wall now calls
  `map::place_floor_convert_glass` (`modal.rs:16060`) instead of setting `T_FLOOR`.
- `alloc_object` (`generate.cc:1069`) — `item::scatter_objects` (`item.rs:3996`) now uses
  the faithful `randnor(9,3) + randnor(3,3)` object budget plus `randnor(3,3)` gold with
  the `alloc_object_cell`-style legal-grid search; the room/corridor split is the only
  remaining divergence (the port's `Map` carries no `CAVE_ROOM` bit).

`[~]` (2, unchanged):
- `process_world_gods`/`grace_delay_trigger` — Varda/Ulmo/Aule/Mandos are Theme-only gods
  (no `MODULE_TOME`), so the function is unreachable in the base module.
- `dungeon` arrival stairs — the snapshot level model replaces `create_up/down_stair`.

Notes (this session's owned-file changes):
- `map::destroy_level`'s per-grid guard still checks only `permanent`: it runs during
  generation before objects exist (as in C++), so the artifact half of `cave_valid_bold`
  is vacuous there; `game::earthquake` implements the C++ spells2.cc:3038 terrain pass
  (that function does not use `cave_valid_bold` either).
- `map::cave_valid_bold` therefore stays as a ready helper with no live caller in the
  owned files; the geomancy dig entry point that needs it (spells2.cc:1419) lives in
  modal.rs.
