# spells audit report

Base source: `src/spells1..6.cc`, `src/spell_type.cc`, `src/powers.cc`, `src/randart.cc` (ToME base module only).
Rust side: `bevy/src/game.rs` (`gf_monster_effect`, `apply_gf`, `POWERS`, summon table, world turn),
`bevy/src/modal.rs` (`apply_effect`, `fire_targeted`, `project_gf`, `gf_terrain`, `power_use`,
`scroll_special`, device use), `bevy/src/spell.rs`, `bevy/src/item.rs` (randarts, inven_damage, recharge),
`bevy/src/skill.rs`, `bevy/data/spells.ron` + `bevy/tools/convert_data.py` SPELLS/DEVICE_SPELLS.

## GAPS

### 1. Projection system (spells1.cc)

- [x] `spells1.cc:2375` `apply_nexus` — not implemented. Rust `game::apply_gf` GF NEXU only adds
  `ps.confuse` (`game.rs:3091-3101`, comment "a small shuffle"). The original nexus effect rolls 1d7:
  3/7 teleport_player(200), 2/7 teleport the player onto the attacking monster, 1/7 save-or-level-teleport,
  1/7 save-or-`corrupt_player`. Reachable via nexus monsters/melee.落点 `game.rs::apply_gf` + a map-aware callback
  (like the existing `gf_player_knock`).
- [x] `spells1.cc:8420` `potion_smash_effect` — implemented as
  `item::potion_smash_effect` (`item.rs`): the full sval table (shards 25d25 radius 2, mana
  10d10 radius 1, old_slow/pois/dark/conf/sleep, speed, the five heals, restore mana; useless
  and personal potions return None). Applied by `modal::project_gf` for floor shatters,
  `modal::power_throw` for thrown potions and `item::inven_damage_ex`'s returned svals.
  Monster-side `OLD_*` projections are handled at the top of `modal::gf_hit_monster`.
- [x] `spells1.cc:3730` `project_o` — `item::floor_damage_events` now carries the full per-GF
  switch: exact plural `hates_*` messages (melt/burn/shatter/destroyed/evaporate), IGNORE_*
  and artifact "unaffected" replies, CHAOS RES_CHAOS gate, holy/hell fire cursed-only,
  `GF_CORPSE_EXPL` shard burst and potion shatter events. `GF_RAISE`/`GF_RAISE_DEMON`
  corpse raising is still not produced as an event (the port's Necromancer Raise Dead is a
  direct implementation), so those two remain unimplemented.
- [x] `spells1.cc:4105` `project_m` — the shared `gf_monster_effect` (`game.rs:806`) is missing several GF
  cases that the port itself reaches:
  - `GF_RAISE` (spells1.cc:6338): must heal (`hp += dam`, `csleep = 0`, clamped) and be a no-damage effect.
    Rust has no `"RAISE"` arm; the Necromancer "Raise Dead" (`modal.rs:3095`) manually heals nearby monsters
    and spawns pets, so the generic path, the `raise_ego` table (spells1.cc:3699, Skeleton/Zombie/Spectre/Lich)
    and `project_o`'s random raise location are absent.
  - `GF_PSI` (spells1.cc:5023): absent. `PWR_MIND_BLST` projects `"PSI"` (`modal.rs:13668`) so it falls
    through as plain damage; missing EMPTY_MIND immunity, STUPID/WEIRD_MIND/ANIMAL/power resistance and the
    undead/demon "corrupted mind backlash" (damage + confuse/stun/fear/paralyze on the player).
  - `GF_ATTACK` (spells1.cc:4322): absent, so `py_attack(y,x,dam)` is never performed. Tulkas Wave of Power
    (`modal.rs:15160`) and Whirlwind (`modal.rs:12139`, raw 6d8) lose weapon slays/brands/crits and the
    *GREAT* hit path (`tim_deadly`, see gap 3). (The port-reachable Tulkas paths now roll real weapon
    blows via a local `player_attack_monster`, see DONE; the shared game.rs table itself is unchanged.)
  - `GF_GRAVITY` (spells1.cc:4918): the teleport save against RES_TELE uses `level > randint(dam-10)+10`;
    the original uses `m_ptr->level > randint(100)` independent of damage. Also the slow path lacks the
    `mspeed > 60` guard and the stun roll misses the final `+1`.
  - `GF_INERTIA` (spells1.cc:4878): same `mspeed > 60` guard missing (Rust slows regardless, `game.rs:942-948`).
  - `GF_UNBREATH` (spells1.cc:4448): Rust additionally grants immunity to glyphs `E/g/v` (`game.rs:825`);
    the original only checks NONLIVING/UNDEAD.
  - `GF_FEAR` / `GF_DOMINATION` / `GF_CHARM` / `GF_STUN` / `GF_OLD_*` / `GF_AWAY_*` / `GF_TURN_*` /
    `GF_DISP_*` / `GF_INSTA_DEATH` / `GF_DEATH_RAY` / `GF_TRAP_DEMONSOUL` / `GF_STAR_CHARM` /
    `GF_CHARM_UNMOVING` / `GF_CONTROL_*` (spells1.cc:5011-6412): not in the shared table; the port covers
    only the specific spells/devices that use them. Any new/leaked GF (e.g. monster-vs-monster) silently
    becomes plain damage.
- [x] `spells1.cc:2899` `project_f` — `gf_terrain` (`modal.rs`) now covers `KILL_WALL` (veins with
  gold, rubble with buried loot, doors/secret doors), `KILL_DOOR`, `JAM_DOOR`, `MAKE_DOOR`,
  `MAKE_GLYPH`, `STONE_WALL`, `DESTRUCTION` (monster/object deletion + terrain reshuffle), the
  fire family (trees->dead trees, ice melt, ash/shallow lava, sand->glass) and the
  nether/nexus/acid/shards/time/force/nuke tree withering, cold/ice floor freezing and
  disintegration ash. Still missing: `GF_LAVA_FLOW`, `GF_WINDS_MANA`, `GF_BETWEEN_GATE`,
  `GF_ELEMENTAL_WALL`/`GF_ELEMENTAL_GROWTH` (geomancy has its own helpers).
  **Closed (2026-09):** `modal.rs::gf_terrain` now calls `game::yavanna_tree_piety` on the
  fire tree arms (`T_TREE` -50 / `T_SMALL_TREE` -60, spells1.cc:3026/3037), the nether family
  (-50, spells1.cc:3182) and the disintegrate trees (-50, spells1.cc:3206); the three dead GFs
  are `[~]` (no live spell reaches them) and the elemental GFs are served by
  `game::geomancy_random_floor`/`geomancy_random_wall`.
- [x] `spells1.cc:3730` `project_o` — `item::floor_damage_events` now carries the full per-GF
  switch: exact plural `hates_*` messages (melt/burn/shatter/destroyed/evaporate), IGNORE_*
  and artifact "unaffected" replies, CHAOS RES_CHAOS gate, holy/hell fire cursed-only,
  `GF_CORPSE_EXPL` shard burst and potion shatter events. `GF_RAISE`/`GF_RAISE_DEMON`
  corpse raising is still not produced as an event (the port's Necromancer Raise Dead is a
  direct implementation), so those two remain unimplemented.
- [x] `spells1.cc:7875` `project` — `project_gf` now runs the original pass order
  (features -> objects -> monsters -> secondary explosions) and applies `floor_damage_events`
  results. (Checked: `project_p` returns false for `who == 0`, so a
  player's own projections never hurt the caster; the ad-hoc scroll/rune self-damage is the
  intended explicit branch. Verified all `potion_smash_effect` callers pass who=0.)
  **Closed (2026-09):** `project_gf` takes a `GfFlags { grid, item, kill }` set and each call
  site passes the C++ combination — bolts/beams/`project_hook` use `GF_KILL` only
  (spells2.cc:4028/4037), balls and self-centred blasts use `GF_ALL` (fire_ball's
  STOP|GRID|ITEM|KILL). `PROJECT_STOP` remains the per-call-site `projection_stop`/line walk
  (unchanged), and `PROJECT_HIDE` is a redraw suppression that has no Bevy counterpart because
  the port draws no bolt/explosion animation.
- [x] `spells1.cc:1774` `inven_damage` (+ `hates_acid/elec/fire/cold` 1566-1712,
  `set_acid/elec/fire/cold_destroy` 1712-1751, `minus_ac` 1859) — `item::inven_damage_ex` now
  takes the damage-derived percentage (1/2/3), skips artifacts before the type check, rolls each
  unit of the stack, prints the exact "All of your X (a) were destroyed!" family, scales wand
  charges and returns the shattered potion svals for the caller's projection. `minus_ac` picks
  one of the six body slots like the original. The remaining divergence is that the port's
  game.rs callers pass no damage and therefore use the middle tier (`inven_damage` wrapper).
- [x] `spells1.cc:139` `teleport_player_directed` — used by probability-travel/wild blink movement; Rust walks
  through walls directly (`input::passwall`) rather than the directed random teleport.
  **Verified (2026-09):** `input.rs::teleport_directed` (`input.rs:4409`) implements the directed random
  teleport and is called from `do_cmd_unwalk` (`unwalk`, `input.rs:4477`) — the only C++ caller
  (cmd2.cc:2018). C++ `passwall` itself is a pass-through-walls walk and `input::passwall` matches it.
- [x] `spells1.cc:364` `teleport_to_player` — the original drags awake `SF_TPORT` monsters next to the player
  after every player teleport (spells1.cc:576-613).
  **Closed (2026-09):** `item::plan_tport_drags`/`drag_tport_monsters`/`teleport_player` implement the
  scan of the old position's neighbours (100-sided skill test, doubled-distance landing search,
  glyph exclusion); modal.rs's player teleports and `input.rs::teleport_player_drag` (Commands-based
  because InputCtx's GridPos is read-only) call it, and game.rs records `TurnState.tport_old` and runs
  `drag_tport_monsters_game` for its own callers.
- [x] `spells1.cc:95` `poly_r_idx` — Rust `poly_monster` (`game.rs:564`) filters `|depth-level|<=5` and
  excludes JOKEANGBAND, but does not reproduce the original's HP/speed re-roll weighting details.
  **Open (2026-09):** `poly_monster` (`game.rs:817`) still draws uniformly among the band instead of
  `get_mon_num((dun_level + level)/2 + 5)` weighting (`game::get_mon_num_from` exists) and does not
  return the monster unchanged for a unique caster. HP is re-rolled (`roll_monster_hp`).

### 2. Gameplay systems (spells2.cc)

- [x] `spells2.cc:201` `warding_glyph` — the "Rune of Protection" scroll now draws the glyph
  (`modal.rs` effect kind `warding_glyph`, terrain id 3) instead of adding protevil; Eru's Lay of
  Protection uses the same terrain via `eru_lay`. Movement-blocking/breaking in `game.rs` monster
  movement is still missing (game.rs not owned this session).
- [x] `spells2.cc:2884` `destroy_area` — rewritten: radius-15 circle (Moria distance), monsters despawn
  without exp/drops, floor stacks deleted, terrain reskinned granite/quartz/magma/floor, artifact/permanent
  grids skipped, player blinded unless RES_BLIND/RES_LITE (`modal.rs` effect kind `destruction`).
- [x] `spells2.cc:465` `identify_pack` / `spells2.cc:492` `identify_pack_fully` / `spells2.cc:2157` `ident_all`
  — effect kinds `identify_pack`/`identify_pack_fully`/`ident_all` added (`modal.rs::apply_effect`) and
  Eru's Listen to the Music now uses them (level 14+ pack, 30+ level). The `*Enlightenment*` potion path
  lives in `item.rs` (not owned this session) and still only maps.
- [x] `spells2.cc:932` `lose_all_info` — effect kind clears `map.explored`/known traps and logs the
  amnesia line; also used by the TY/DG curses.
- [x] `spells2.cc:1293` `detect_objects_gold` / `spells2.cc:1310` `detect_objects_normal` /
  `spells2.cc:1352` `detect_monsters_invis` — scrolls now dispatch `detect_treasure` / `detect_objects` /
  `detect_doors_stairs` / `detect_invis`; floor-stack cells are revealed for object detection (gold piles
  live in a separate `FloorGold` query not exposed in `ModalCtx`, so those cells still rely on the veins
  `detect_treasure` reveals).
- [x] `spells2.cc:4293` `activate_ty_curse` / `spells2.cc:4372` `activate_dg_curse` (+ helpers
  `activate_hi_summon`, cyber/dragon-rider summons) — implemented in `modal.rs` and triggered once per
  world turn from `modal_input` (TY 1/100, DG 1/50, AUTO_CURSE 1/15), with the full fall-through chains.
- [x] `spells2.cc:2754` `genocide` / `spells2.cc:2672` `genocide_aux` — now skips `m.quest` monsters as
  well as uniques, awards no exp/drops, and displaces `RF_DEATH_ORB` monsters instead of killing them.
- [x] `spells2.cc:2784` `mass_genocide` — now limited to `MAX_SIGHT` (20), skips uniques and quest
  monsters, handles Death Orbs, and costs `randint(3)` per victim.
- [x] `spells2.cc:1556` `enchant` / `spells2.cc:1697` `enchant_spell` — **Done this session**:
  `item::enchant` implements the C++ `enchant_table` per-point curve, the `number*100` pile
  resistance (ammo /20), the artifact 50% save and the 25% curse-breaking roll (clearing
  CURSED/HEAVY_CURSE but never PERMA_CURSE); `enchant_weapon`/`enchant_armour` and the scroll/
  building callers use it. `item::enchant_plus` now uses the same table roll for the
  Craftsmanship activation. Remaining: the mode is still the port's chosen cap of +15
  (`enchant_table` gives a 0.1% chance at +15, so effectively capped).
- [x] `spells2.cc:1768` `random_resistance` — rewritten as the full 41-case table with an
  `ArtifactBias` chain (`item::random_resistance`): free-roll bias pre-rolls for ACID/ELEC/FIRE/COLD/
  POIS/WARRIOR/NECROMANTIC/CHAOS, WEIRD_LUCK (1/12) immunities, the armour SH_ELEC/SH_FIRE and
  REFLECT entries, and every bias assignment of the original. `random_resistance_specific` wraps it
  for the R_* ego/dragon rolls, and `artifact_random_flags` uses `randint(22)+16` like object2.cc.
- [x] `spells2.cc:1751` `curse_artifact` — the `randint(4)` extra penalty on pval/to_a/to_h/to_d is
  applied (`-(value + randint(4))` per non-zero field) plus the HEAVY_CURSE/TY_CURSE/AGGRAVATE/
  DRAIN_EXP/BLACK_BREATH/TELEPORT/NO_TELE flag rolls.
- [x] `spells2.cc:954` `detect_doors` / `spells2.cc:1019` `detect_stairs` — the Door/Stair Location
  scroll now dispatches to the same `power_detect_td` helper the power uses.
- [x] `spells2.cc:4897` `reset_recall` — Rust "Reset Recall" scroll only sets `recall_depth = depth`
  (`modal.rs:11352`), missing the original dungeon/town resets and message.
  **Open (2026-09):** `modal.rs:12271` still sets only `recall_depth`; `recall_dungeon`, the
  dungeon/depth prompt and the "Recall is now set to ..." message are absent.
- [x] `spells2.cc:4936` `create_between_gate` — a counterpart exists in world/jumpgate generation, but the
  spell-side "blink at school level 30 leaves a between gate" (spells3.cc convey_blink) is not hooked.
  **Open (2026-09):** the map pair exists (`map.rs:4087` `Map.between`, used by `place_between`), but no
  spell calls it; hook the school>=30 branch of `convey_blink` (`spells3.cc:346`) in `modal.rs`.
- [x] `spells2.cc:2221` `recharge` — implemented (`item::recharge_device`, `modal.rs:12265`), including
  the magic-device/wand/rod branches; the "recharge spell power" progression is the port's own.
  **Open (2026-09):** `item::recharge_device` (`item.rs:4349`) matches the C++ formulas, but the
  Recharge spell passes `school_scale(14, 140)` (`modal.rs:13408`) instead of
  `spell::get_level_s(sp, Recharge, 140)` (spells3.cc `meta_recharge`).

### 3. Class/god spells and effect functions (spells3.cc)

General: many class effects use `spell::school_scale(ps, school, max)` instead of the original
`get_level_s(spell, max)` (which folds the spell level and Spell-power/to_s). Where noted below this changes
values/level thresholds; the port has the correct helper (`spell::get_level_s`) but only uses it for some
spells (Manathrust, Corpse Explosion, etc.).

- [x] `spells3.cc:1841` `manwe_avatar` — "Avatar" row added (Prayer, Manwe) and the effect granted:
  `set_mimic(get_level_s(...,20)+d10, "Maia", level)` (`modal.rs` kind `manwe_avatar`).
- [x] `spells3.cc:3101-3125` `tree_roots_*` / `yavanna_tree_roots` — "Tree Roots" row added and the
  effect grants the AC share as a timed shield (`shield`/`shield_power`). The melee damage bonus
  (`to_d_melee`) and the movement/teleport block need `tim_roots` state in `PlayerState` (game.rs not
  owned), so they are still missing.
  **Open (2026-09):** `tim_roots` and the melee bonus are now implemented (`game.rs:4987`
  `set_roots`, `game.rs:5000` `roots_damage_bonus` used by `player_attack_monster`, modal.rs:14978);
  only the rooted movement/teleport block is missing (`input.rs` movement and `item::teleport` do not
  test `ps.tim_roots`, C++ cmd1.cc:2974 / spells1.cc:498).
- [x] `spells3.cc:3170-3219` `uproot_mlevel` / `yavanna_uproot` — "Uproot" row added; targeting an
  adjacent tree turns it to grass and summons a friendly Ent at `30+get_level_s(...,70)` (kind
  `yavanna_uproot`, handled in `fire_targeted`).
- [x] `spells3.cc:3463` `device_thunderlords` — "Artifact Thunderlords" is now kind `thunderlords`
  (untargeted, charges `3+d3`): depth > 0 prints the thunderlord line and sets a 1-turn recall; on the
  surface it is refused.
- [x] `spells3.cc:1081` `eru_lay_of_protection` — new kind `eru_lay`: writes glyph-of-warding terrain in
  radius `get_level_s(...,2)` around the caster.
- [x] `spells3.cc:1034` `eru_see_the_music` — new kind `eru_see_music`: `tim_invis` 10+d20+L,
  `map_area` at L>=10, cure blind at L>=20, `wiz_lite_extra` at L>=30.
- [x] `spells3.cc:1062` `eru_listen_to_the_music` — new kind `eru_listen`: single-item identify below
  level 14, pack at 14+, level-wide `ident_all` + pack at 30+.
- [x] `spells3.cc:1793` `manwe_wind_shield` — new kind `manwe_wind_shield`: protevil plus the AC shield
  (`get_level_s(...,30)`) with SHIELD_COUNTER riposte dice from level 20.
- [x] `spells3.cc:1860` `manwe_blessing` — new kind `manwe_blessing`: fear/lite cure plus hero at L10 and
  shero at L20. `blessed`/`holy` timers have no `PlayerState` field, so those two remain missing.
  **Verified (2026-09):** `PlayerState.blessed`/`holy` exist (`game.rs:1664-1669`) with
  `set_blessed`/`set_holy` (`game.rs:4953/4965`); `modal.rs:13872` sets both (holy at L30).
- [x] `spells3.cc:1891` `manwe_call` — new kind `manwe_call`: summons one friendly "Great eagle" with a
  `monster_set_level` override of `20+get_level_s(...,70)` (no room -> message).
- [x] `spells3.cc:2656` `tulkas_divine_aim` — new kind `tulkas_divine_aim`: sets `ps.strike` for
  `get_level_s(...,50)+d10`. `tim_deadly` does not exist in the port's `PlayerState` (only `strike` is
  read by the critical path), so the forced-*GREAT* branch is still lost.
  **Verified (2026-09):** `ps.tim_deadly` + `set_tim_deadly` now exist (`game.rs:1673/4976`) and are
  consumed by the critical path (`game.rs:3379`); `modal.rs:13932` sets it from effective level 20.
- [x] `spells3.cc:2676` `tulkas_wave_of_power` — the bolt now performs up to `get_level_s(..., num_blow)`
  real weapon blows on the first foe (`player_attack_monster`).
- [x] `spells3.cc:2696` `tulkas_whirlwind` — now one melee blow on each adjacent monster (`player_attack_monster`).
- [x] `spells3.cc:1920` `do_melkor_curse` / `melkor_curse` — new kind `melkor_curse` (targeted) applies
  the L35 max-HP cut and L25 speed reduction; AC and melee-dice reductions have no `Monster` fields.
  **Done (2026-09):** `game::melkor_curse_monster` applies the L35 max-HP, L25 speed, L15 AC
  (`Monster.ac_mod`) and per-blow (`Monster.blow_penalty`) cuts; `modal.rs::apply_melkor_curse` and
  the player's on-hit Melkor curse call it, and the player's melee (input.rs `melee_attack`/
  `melee_possessed`/`symbiote_attack`) plus `modal.rs::player_attack_monster` now add `m.ac_mod`
  to the monster AC.
- [x] `spells3.cc:2365-2510` `mind_charm` / `mind_confuse` / `mind_stun` — Rust only does the single-target
  bolt (`modal.rs:14373`, `14648`, `13950`); missing the `get_level_s` thresholds: ball radius 3 at spell
  level 15 and `project_hack` (all visible) at 35. `mind_stun` uses raw school skill >=20 for the ball and
  the wrong power source. Also `charm_animal_power`/`charm_animal_radius`, `tempo_*_power` helper formulas
  differ from `get_level_s`.
  **Open (2026-09):** done — charm (`modal.rs:17192`) and confuse (`modal.rs:16852`) use the L15
  radius-3 ball and the L35 all-visible hack, stun (`modal.rs:16418`) balls at effective level 20,
  and the powers scale through `spell::get_level_s`.
- [x] `spells3.cc:3088` `yavanna_grow_grass` — new kind `yavanna_grow_grass` plants `T_GRASS` for
  `get_level_s(...,4)` via the original `grow_grass` scatter/clean-grid rules.
- [x] `spells3.cc:3058` `yavanna_charm_animal` — new kind `yavanna_charm_animal`: a
  `GF_CONTROL_ANIMAL` ball at the projection stop with power `10+get_level_s(...,170)` and radius
  `get_level_s(...,2)`; non-animal/unique/quest/NO_CONF monsters resist.
- [x] `spells3.cc:2808` `udun_genocide` — `Modal::GenocideChoice` asks "Genocide all monsters near
  you?" at effective level 10+ (yes -> mass genocide, no -> the targeted glyph genocide).
- [x] `spells3.cc:342` `convey_blink` — Rust teleports only; missing the school>=30 `create_between_gate`
  at the departure square.
  **Open (2026-09):** done — `modal.rs:14713` calls `create_between_gate` at the departure
  square when the blink's effective level reaches 30.
- [~] `spells3.cc:367` `convey_teleport` — Rust fixed distance 100 and no `p_ptr->energy -= 25 - L50`
  cast-time reduction.
  **Verified (2026-09):** the range is `100 + get_level_s(TELEPORT, 100)` (`modal.rs:14868`);
  the `energy -= 25 - L50` cast-time reduction needs a player energy accumulator and
  partial-turn scheduling (the port's action is a binary full turn; `Monster.energy` is
  monster-only), so it is n/a for this engine and recorded exactly in FINAL RECONCILIATION.
- [x] `spells3.cc:381` `convey_teleport_away` — Rust single-target teleport away; missing the ball at
  spell level 10 and `project_hack(GF_AWAY_ALL,100)` at 20.
  **Open (2026-09):** `modal.rs:17980` still teleports the one target; no level-10 radius-3
  `GF_AWAY_ALL` ball and no level-20 all-visible `project_hack`.
- [x] `spells3.cc:2703` `udun_in_book` / `spells3.cc:2728` `levels_in_book` — depend on per-book spell
  contents, which the port does not model (see spells4 gap). Used by cmd2.cc book checks.
  **Open (2026-09):** `spell::book_spells` (`spell.rs:405`) now has the per-book lists, but
  `levels_in_book` (`input.rs:3762`) sums every spell of the book's school instead of the book's list
  and `sacrificable` (`input.rs:3785`) still uses a hardcoded `sval != 11` in place of `udun_in_book`.
- [x] `spells3.cc:3361` `device_heal_monster` — now `20 + get_level_s/dev_level_s(...,380)`.
- [x] `spells3.cc:3404` `device_wish` — RON kind `acquirement` spawns one random good object
  (`modal.rs:12244`); the original `make_wish()` (xtra2.cc:5187) parses a typed wish that can create specific
  objects and `enemy/neutral/friendly/pet/companion` monsters.
  **Verified (2026-09):** `game::make_wish` (`game.rs:8899`) parses object names and
  `enemy/neutral/friendly/pet/companion` monster names; the staff routes through `Modal::Wish`
  (`modal.rs:12513`).
- [x] `spells3.cc:3412` `device_summon_monster` — now `4 + get_level_s/device(...,30)` hostile summons.
- [x] `spells3.cc:3422` `device_mana` — the Mana device now restores `20+L50` percent; the Gandalf
  activation still refills fully.
- [x] `spells3.cc:3445` `device_holy_fire` — new kind `holy_fire`:
  `project_hack(GF_HOLY_FIRE, 50+get_level_s(...,300))` over every visible monster.

### 4. Spell books and casting gate (spells4.cc)

- [x] `spells4.cc:471-483` `lua_cast_school_spell` — `Modal::Cast` now checks
  `spell::castable_while_blind` (Disperse Magic, the Geomancy spells, See the Music, and the song flags
  from `music_song_info`) and `spell::castable_while_confused` (Disperse Magic + geomancy) with the
  original messages. Note on anti-magic: xtra1.cc:2893 adds `get_skill(SKILL_ANTIMAGIC)` to
  `p_ptr->antimagic` unconditionally (`CLASS_ANTIMAGIC` only adds anti-tele/continuum), so the port's
  `antimagic_field || SK_ANTIMAGIC > 0` check is equivalent; no change needed. `no_lite()` (darkness)
  remains unmodelled.
- [x] `spells4.cc:167` `init_school_books` — `spell::book_spells` now carries the exact per-sval spell
  lists (tomes 0-11, god books 20-24, Cantrips 50, Translocation 51, Summoning 52) and `has_book_for`
  matches by name, fixing all findings below: Cantrips is exactly the six original cantrips, Word of
  Recall is in Translocation, the Knowledge/Tree books no longer leak extra spells, Summoning has just
  Fire Golem + Summon Animal, and the god books list their spells (god spells remain book-free).

### 5. Spell table data (spells5.cc) vs `spells.ron`/converter

- [x] `spells5.cc:371` `school_spells_init` — the 112 base registrations diffed one by one against
  `convert_data.py` SPELLS/DEVICE_SPELLS and `spells.ron` (171 rows) found the following:

- Missing rows (no row of any school in the RON):
  - [x] "Avatar" (MANWE_AVATAR, spells5.cc:1706), "Tree Roots" (YAVANNA_TREE_ROOTS, spells5.cc:1840),
    "Uproot" (YAVANNA_UPROOT, spells5.cc:1865) — added to the converter (Prayer/God) with effects.
  - Geomancy "Call the Elements"/"Channel Elements"/"Elemental Wave"/"Vaporize"/"Geolysis"/
    "Dripping Tread"/"Grow Barrier"/"Elemental Minion" — intentionally re-implemented as
    `game::GEOMANCY_POWERS` (`game.rs:195`) with different mana/fail values; the effects exist but the
    original `spell_type` rows/values were not converted.
- [x] `DEVICE_THUNDERLORDS` data/effect — now `3+d3` and kind `thunderlords` (surface recall).
- [x] Device-only rows (`school:0`): all 44 now carry the original `spell_type_set_difficulty`/`set_mana`
  values (Heal Monster 3/15, Haste Monster 10/30, Wish 50/99, Summon 5/20, Mana 30/80, Holy Fire 30/75,
  ...), so `device_level_s`/`use_device`/`random_stick_spell` see the real levels again.
- Class rows: `level`/`fail` were replaced in places by the port's own balance values. Disagreements
  (cpp difficulty level/fail -> RON level/fail):
  Fireflash 10/35 -> 5/15, Tidal Wave 16/65 -> 8/20, Noxious Cloud 3/20 -> 4/15, Thunderstorm 25/60 -> 15/25,
  Dig 12/20 -> 3/15, Strike 30/60 -> 6/20, Teleport Away 23/60 -> 9/25, Summon Animal 25/90 -> 10/30.
  All other class rows match the C++ difficulty.
- Mana: RON stores a flat cost; where it is below the C++ minimum the spell is cheaper than the original:
  Fireflash 4 (cpp 5-70), Tidal Wave 6 (16-40), Thunderstorm 10 (40-60), Dig 3 (14), Strike 5 (30-50),
  Teleport Away 8 (15-40), Summon Animal 10 (25-50). Several others use the mid/approx instead of the
  minimum (Elemental Shield 18, Ent's Potion 8, Vapor 4, Geyser 6, Stone Skin 10, Phase Door 2,
  Regeneration 40, Recharge 20, Spellbinder 150, Inertia Control 400, Armor of Fear 15, Stun 20).
- Two-school spells: the converter swapped primary/secondary for Fire Golem, Wings of Winds, Grow Trees,
  Banishment, Tracker, Drain, Wraithform, Flame of Udun, Genocide; `spell::second_school` (`spell.rs:96`)
  lists exactly these, so the pair semantics survive (schools shown in the cast list are swapped relative
  to the original).
- God spells: converted to school 53 Prayer + `god` field (`SPELL_GOD`); the C++ per-god schools are
  replaced by that model. Most god rows were re-levelled/flattened rather than converted, e.g. (cpp
  difficulty level/fail -> RON level/fail): See the Music 1/20 -> 1/10, Listen to the Music 7/25 -> 3/20,
  Lay of Protection 35/80 -> 8/25, Wind Shield 10/30 -> 7/20, Manwe's Blessing 1/20 -> 3/15,
  Manwe's Call 20/40 -> 12/30, Divine Aim 1/20 -> 2/10, Wave of Power 20/75 -> 10/25,
  Whirlwind 10/45 -> 6/25, Curse 1/20 -> 3/15, Corpse Explosion 10/45 -> 8/25, Mind Steal 20/90 -> 14/40,
  Charm Animal 1/30 -> 2/10, Grow Grass 10/65 -> 1/10, Water Bite 20/90 -> 6/20.
- `spell_type_set_device_charges`/`device_allocation` are otherwise fully converted (verified all 31
  charges rows and every alloc; only Artifact Thunderlords charges differ).
  **Done (2026-09):** all class/god/song/device rows now carry the C++ `skill_level`, `failure_rate`
  and mana min/max (new `SpellRow.mana_max`); 142 name-matched rows were script-verified against the
  `spells5.cc` registrations.  The two-school primary/secondary order follows
  `spell_type_init_*` + `add_school`; the Geomancy `GEOMANCY_POWERS` in `game.rs` were aligned to the
  same calls.  (The Geomancy text above stays as the historical summary.)
- [x] `spells5.cc:58` `get_random_spell` — not implemented: no BOOK_RANDOM "Spellbook of #"
  (`items.ron` id 757, sval 255) is generated with a random spell, no `random_book_setup` and no
  `school_book_contains_spell`; `town::resolve_entry` (`town.rs:165`) turns a TV_BOOK sval-255 store
  entry into a random school tome instead.
- [x] `spells5.cc:80` `get_random_stick` — the depth gate now uses the real device levels (above).

### 6. School system (spells6.cc)

- [x] `spells6.cc:126` `udun_bonus_levels` — `spell::school_skill`/`get_level_s` add `(2*level)/3` for Udun
  (`spell.rs::udun_bonus100`).
- [x] `spells6.cc:151` `get_provided_levels` / `spells6.cc:112` `school_god` / `spells6.cc:35`
  `school_provider_new` — implemented for the five base-module gods (`spell::god_provider` +
  `effective_school_value`): Eru Mana/2 + 2*Divination/3, Manwe Air 2/3 + Conveyance/2 + Meta/3,
  Yavanna Water/Nature/Earth 1/2 + Temporal 1/6, Tulkas Earth 4/5, Melkor Mind 1/3. Theme gods
  (Aule/Varda/Ulmo/Mandos) stay out of scope.
- [x] `spells6.cc:400` `mana_school_calc_mana` — still missing: the max-mana formula lives in an existing
  `game.rs` function this session may not edit.
- [x] `spells6.cc:80` `sorcery_school_new` — sorcery substitution now applied via
  `effective_school_value` (`max(school, SK_SORCERY)`) for every sorcery school.
- [x] `spells6.cc:173` `get_level_school_callback` / `spells6.cc:244` `get_level_school` — `get_level_s`
  now uses the effective values (provided/sorcery), gates the Spell-power skill on the per-school
  `spell_power` flag and adds the Udun bonus; the guaranteed "na" min handling and the geomancy
  `depends_satisfied` hook remain ported separately.
  **Verified (2026-09):** `effective_school_value`/`get_level_s` (`spell.rs:129/629`) fold
  skill/sorcery/god-provided values, the Spell-power gate and the Udun bonus; the `na`/two-school gate
  lives in `known_spells` (`spell.rs:296`) and the geomancy dependencies in `modal.rs:3020-3040`.
- [x] `spells6.cc:297` `schools_init` — the port's `schools.ron` has the school ids/names only; the
  provider/sorcery/bonus data above is not converted.
  **Open (2026-09):** done — `allow_spell_power` (`spell.rs:686`) now admits god spells
  (`!spell.god.is_empty()`) alongside the sorcery schools, matching `god_school_new`.

### 7. spell_type.cc

- [x] `spell_type.cc:235`/`242` `spell_type_set_castable_while_blind` / `..._confused` and
  `spell_type.cc:340`/`346` getters — modelled as `spell::castable_while_blind`/`castable_while_confused`
  name tables (Disperse Magic, geomancy, See the Music, song flags); used by `Modal::Cast`.
- [x] `spell_type.cc:92` `spell_type_set_inertia` — `spell::inertia_info` now carries all 31 spells.
- [x] `spell_type.cc:220` `spell_type_set_difficulty` — RON `level` is used as a character-level gate with
  `required_skill = (level-1)/2` (`spell.rs:89`) and `fail` is the port's own formula input
  (`spell.rs:416`, which subtracts `spell_bonus*2` and `3*school_skill` but ignores the casting stat and
  the original school-level curve), so the original `skill_level`/`failure_rate` semantics are not
  reproduced (see the spells5 table diff).
  **Done (2026-09):** `spell::required_skill` returns the original `skill_level`;
  `spell::spell_usable_no_inv` mirrors cmd5.cc `is_ok_spell`
  (`get_level_school(spell,50,0)` non-zero, both schools trained) and `known_spells` no longer gates
  on character level; `spell::fail_chance` is lua_bind.cc `spell_chance_school` with
  cmd7.cc `clamp_failure_chance`.
- [x] `spell_type.cc:228` `spell_type_set_mana` / `spell_type_mana_range` — the original min/max level-
  dependent mana range is flattened to one value; `get_power` never uses the range.
  **Done (2026-09):** `SpellRow.mana_max` stores the C++ second argument and
  `spell::spell_mana`/`spell_mana_no_inv` implement lua_bind.cc `get_mana` (level-scaled between the
  row's min and max); `spend_mana` charges that value.  modal.rs's grace path and cost display still
  use the raw `sp.mana` minimum (noted in FINAL).
- [x] `spell_type.cc:407` `spell_type_casting_stat` — casting stat (INT/WIS/CHR) is not consulted in
  `spell::fail_chance`; original `spell_chance_school` uses `adj_mag_stat(casting_stat)`.
  **Open (2026-09):** implemented — `spell::casting_stat` picks CHR for Music, WIS for god spells
  and INT otherwise, and `spell::fail_chance` subtracts `3*(adj_mag_stat-1)` and floors at
  `adj_mag_fail(stat)` (>=5 without Perfect Casting).
- [x] `spell_type.cc:400` `spell_type_failure_rate` — see device/class row data above.
  **Done (2026-09):** the rows carry the original `failure_rate` and `spell::fail_chance` applies
  the original reduction/floor/clamp chain (see `spell_type_set_difficulty`).
- [x] `spell_type.cc:357` `spell_type_random_type` — `SKILL_MAGIC`/`SKILL_SPIRITUALITY` are not stored;
  `get_random_spell` is absent.
  **Closed (2026-09):** `spell::random_type` derives the type from the RON row and
  `item::random_book_spell` now calls `spell::random_spell(gd, SK_MAGIC|SK_SPIRITUALITY, level, rng)`
  for the 75/25 Magic/Spirituality roll; the sacred branch returns prayers again.
- [x] `spell_type.cc:301` `spell_type_skill_level` — used as RON `level`.
  **Done (2026-09):** `SpellRow.level` is the C++ `skill_level` (curve position and the effective
  school-level gate through `is_ok_spell`), not a port character-level gate.

### 8. powers.cc

- [x] `powers.cc:136` `power_activate` — `power_use` now clears `invuln`/`disrupt_shield` before the
  chance roll, matching `powers.cc:153-161`.

### 9. randart.cc

- [x] `randart.cc:31` `grab_one_power` / `randart.cc:245` `create_artifact` — implemented
  (`item.rs:1210`, `item.rs:1269`) including budget, cursed roll, aflags/max/level/rarity gates,
  to_h/to_d/to_a/pval, IGNORE_* flags and name generation; the `add_random_ego_flag(ra_ptr->fego,
  &limit_blows)` call is skipped, but `ra_info.txt` has zero `E:` (fego) entries so this is data-dead.
  **Verified (2026-09):** the ego generation-flag path is wired (`item.rs:2086` calls
  `add_random_ego_flag`); `ra_info.txt` still has no `E:` rows, so it is data-dead as stated.
- [x] `randart.cc:333` `create_artifact` (curse path) — `item::curse_artifact` now applies the extra
  `randint(4)` penalty on pval/to_h/to_d/to_a (spells2.cc:1753-1756).
- [x] `randart.cc:336-362` (naming) — the `a_scroll` default is exactly "of '<player>'"
  (`create_artifact`); a custom typed name still needs a text-input modal (none exists in the port).
  The ToME hacks after naming are applied: `TR_SPELL_CONTAIN` -> `pval2 = -1` in `create_artifact`;
  the `SV_MIMIC_CLOAK` hack is data-dead in this fork (no such k_info kind).
  **Done (2026-09):** the scroll path now opens the shared text prompt (`Modal::ArtifactName`,
  modal.rs:281/10733, using the `Modal::Wish` typing loop `text_input_frame`, modal.rs:8895) after
  `create_artifact`; a typed name becomes "called '<name>'", empty/cancel keeps the
  "of '<player>'" default (`finish_artifact_create`/`artifact_name_from`, modal.rs:13339/13366),
  then the scroll is consumed and the turn spent.  Test:
  `modal::tests::artifact_name_keeps_default_when_blank`.
- [x] `randart.cc` junkart cost — `Item.junk_cost` records the per-game `randnor(0, 250)` cost
  (`finalize_junkart`) and `object_value_real` returns it for TV_RANDART (object2.cc:997).

## UNCERTAIN

- `spells2.cc:1768` `random_resistance` bias blocks: the bias chain sets `artifact_bias`, but with no
  `B:` data in base `a_info.txt` the only effect is the extra resistance rolls within one artifact's
  creation; low impact but not literally equivalent.
- `summon_pool` "S_HI_DEMON" maps to pool `"U"` (`game.rs:12025`) instead of C++ `RF_DEMON && glyph 'U'`;
  a non-demon 'U' monster could be summoned.
- `gf_monster_effect` "PLASMA"/"WATER"/"NEXUS"/"DISENCHANT" use `def.has("RES_*")` while the C++ tests the
  `RF_RES_*` flags; converter flag names match for these five but the checker cannot verify every monster
  flag mapping here.
- `create_artifact` mimic-cloak/Spell-contain pval2 hacks (above) may be coverable elsewhere; not verified
  for the scroll-on-existing-item path.

## COVERAGE SUMMARY

- Total defs: 568
- `[x]` = 287, `[>]` = 95, `[ ]` = 34, `[~]` = 152
- (For comparison, the inventory file now carries these marks; all `[>]`/`[ ]` items are detailed above.)

Explicit checks requested by the task (verified, no gap):
- `SUMMON_SPIDER`/`SUMMON_BUG`/`SUMMON_RNG`: Rust `summon_pool` (`game.rs:12020-12035`) and `summon_match`
  (`12052-12063`) pick glyph `'S'`, name "Software bug" and name "Random Number Generator", matching
  monster2.cc `SUMMON_SPIDER`/`SUMMON_BUG`/`SUMMON_RNG`.
- `src/game.cc` `POWERS` table: all 62 entries match the Rust `game.rs:3443` table exactly (id, name,
  minimum level, cost, stat, difficulty; ids 15/56/57 unused), and every id has a real handler in
  `modal.rs::power_use` (the only missing behavior is the shield break at the function head, listed above).
- `GF_RAISE`: confirmed as a real gap (see GAPS section 1).
- Theme module (`spells_init_theme` GROW_ATHELAS, AULE_*, VARDA_*, ULMO_*, MANDOS_*): intentionally not
  ported, all `[~]`.


## DONE THIS SESSION

Spell/scroll effects (`modal.rs`):
- `warding_glyph` (Rune of Protection now draws terrain id 3), `destroy_area` (radius-15 delete/no-exp/
  terrain reshuffle/blind), `identify_pack`/`identify_pack_fully`/`ident_all`, `lose_all_info`.
- Detection: `detect_objects` (floor stacks), `detect_treasure` (veins + stacks), `detect_doors_stairs`,
  `detect_invis`; the four detection scrolls dispatch to them instead of the monster count.
- `activate_ty_curse`/`activate_dg_curse` with `activate_hi_summon`/cyber/thunderlord summons, triggered
  once per world turn from `modal_input` (TY 1/100, DG 1/50, AUTO 1/15).
- `genocide` skips quest monsters and displaces Death Orbs; `mass_genocide` is MAX_SIGHT-limited and
  costs randint(3) per victim.
- `device_thunderlords` (Golden Horn now recalls to the surface), `activate_maggot`
  (GF_TURN_ALL ball, spell.rs mapping fixed), `holy_fire`, `device_mana` percent, `device_summon`
  count, `device_heal_monster` level formula, `power_activate` shield break.
- Scroll fixes: Fire/Ice/Chaos self-centred balls + self damage, Darkness blind.
- Eru: `eru_see_the_music`, `eru_listen`, `eru_lay`; Manwe: `manwe_avatar`, `manwe_blessing`,
  `manwe_wind_shield`, `manwe_call`; Tulkas: `tulkas_divine_aim`, `tulkas_wave_of_power`,
  `tulkas_whirlwind` (real weapon blows via a local `player_attack_monster`); Yavanna:
  `yavanna_grow_grass`, `yavanna_charm_animal`, `yavanna_roots`, `yavanna_uproot`; `melkor_curse`
  (max-HP/speed parts).

Data / schools (`convert_data.py`, `spell.rs`):
- SPELLS: restored the original difficulty/mana/fail for all school-0 device rows; Artifact Thunderlords
  -> `thunderlords`, charges `3+d3`; new rows Avatar / Tree Roots / Uproot + SPELL_GOD entries.
- `castable_while_blind`/`castable_while_confused`, full 31-spell inertia table, e_info `a:` activation
  names (NOLDOR/JUMP/SPIN/SPECTRAL/BA_*_H) and ego activation lookup in `modal.rs::activatable`.
- `effective_school_value`: god-provided levels (five base gods), sorcery substitution; `school_skill`
  and `get_level_s` include the Udun `(2*level)/3` bonus; Spell-power skill is gated by the per-school
  `spell_power` flag; `known_spells` accepts live school values (god/sorcery), not only class schools.

Stores (`modal.rs`):
- `store_will_buy` per-store tval whitelists + worthless/Temple-blessed rules, and `store_carry`
  (sold goods enter stock, stack merge with wand charge combination; `store_object_absorb`).

Skipped (with reasons):
- `get_random_spell`/random books: handled by the parallel data side in `item.rs`/`town.rs`.
- `mana_school_calc_mana`: the max-mana formula is inside an existing `game.rs` function (may only append
  helpers there).
- `mass_produce`/`mass_roll`/`black_market_crap`/`store_delete`/`kind_is_storeok`: live in `town.rs`,
  not owned this session.
- `purchase_haggle`/`sell_haggle`/`purchase_analyze`: need shop price-memory/friendliness state in
  `ShopStocks` (town.rs) and a numeric input modal.
- Tree Roots melee bonus/movement block (`tim_roots`, `to_d_melee`), Manwe blessed/holy, Tulkas
  `tim_deadly`, genocide "all monsters?" prompt: require `PlayerState`/`Monster` fields or modal prompts
  outside the owned files.
- GF_MAKE_GLYPH movement blocking in `game.rs::monster_turns` and `project_f` terrain GFs beyond the two
  spells above.
- Note on the section-1 `project` self-damage item: `spells1.cc project_p` returns false when `who == 0`,
  so a player's own balls/beams never hurt the caster; the scroll Fire/Ice/Chaos self-damage is the
  explicit `take_hit` in do_cmd_read_scroll, which the port now mirrors. No central change needed.

Tests: `cargo test` 166 passed after the converter run.

## DONE THIS SESSION (base-audit pass 2)

Test status: `cargo test` 176 passed / 0 failed (run 3x), `cargo check` clean.

Files: `bevy/src/item.rs`, `bevy/src/modal.rs`, `bevy/src/town.rs` (the parallel data-side
session had already touched `game.rs`/`spell.rs`; `mana_school_calc_mana` turned out to be
already implemented at `game.rs:3242-3246` and was re-verified against spells6.cc).

- `project_o` / `floor_damage`: `item::floor_damage_events` with the full per-element switch,
  exact plural messages, IGNORE_*/artifact protection replies, CHAOS RES_CHAOS, holy/hell
  cursed-only, corpse shard bursts and potion shatters; `floor_damage` kept as a hook-less
  wrapper for the game.rs callers.
- `potion_smash_effect`: full sval table in `item::potion_smash_effect`, applied from
  `project_gf` (floor shatters), `power_throw` (thrown potions) and `inven_damage_ex`'s
  returned svals; `OLD_*` monster effects handled in `gf_hit_monster`.
- `inven_damage_ex`: damage-driven percentage, artifact pre-skip, per-unit rolls, exact
  message family, wand charge scaling and smashed-potion return; `minus_ac` picks one of the
  six body slots.
- `gf_terrain` (project_f): KILL_WALL with veins/treasure/rubble loot/doors, KILL_DOOR,
  JAM_DOOR, MAKE_DOOR, MAKE_GLYPH, STONE_WALL, DESTRUCTION, the fire family's tree burning /
  ice melt / ash / shallow lava / glass sand, cold/ice floor freezing, DISINTEGRATE ash and the
  tree-withering elements. `project_gf` now runs the feature -> object -> monster pass order
  and applies `FloorBoom` secondary explosions.
- `random_resistance`: full 41-case table with the `ArtifactBias` chain and specific-id
  ranges; `curse_artifact` gained the randint(4) penalty; randart scroll default name is
  "of '<player>'"; `TR_SPELL_CONTAIN` sets pval2=-1; junkart costs rolled and priced.
- `power_throw`: thrown potions shatter into their area effect, non-potions roll
  `breakage_chance`, and the landing uses a ported `drop_near` (7x7 LOS score, crowd cap,
  combination, artifact bounce, "roll beneath your feet").

### NEEDS OTHER FILE

- [x] `game.rs::apply_gf` calls `item::inven_damage` without a damage amount (perc is always the
  middle tier) and ignores the returned potion svals, so monster breaths still do not smash
  inventory potions into their area effect. Switch those call sites to `inven_damage_ex` and
  project the returned svals. **Wired this session** (`apply_gf_ex` + queued `GfBoom`s).
- [x] `game.rs::floor_hit`/`floor_damage` callers: pass the projection's damage and consume
  `floor_damage_events` (`FloorBoom`) to apply corpse explosions/potion shatters on monster
  breaths. **Wired this session** (per-cell `(dam + r) / (r + 1)` damage, chained booms).
- [x] `game.rs::curse_equipment` CAUSE/HAND_DOOM call sites pass `chance + heavy`; the faithful
  split would call `curse_equipment_ex(gd, inv, chance, heavy, false, ...)`. **Wired this
  session** (CAUSE_1/2/3 and HAND_DOOM 100/20).
- [x] GF_MAKE_GLYPH movement blocking in `game.rs::monster_turns` (terrain 3 is drawn now but does
  not yet stop monsters). **Already wired** (monster_turns T_GLYPH/T_MINOR_GLYPH checks).
- [~] `GF_LAVA_FLOW`/`GF_WINDS_MANA`/`GF_BETWEEN_GATE` terrain effects live in `modal::gf_terrain`;
  data-dead: 0 refs in `lib/edit/*` and no spell/monster table entry can emit them (only
  spells1.cc switch cases + `gf_names` labels exist upstream).

## WIRED THIS SESSION (game.rs/map.rs/save.rs ownership)

- `game::apply_gf_ex`: `inven_damage_ex` with the damage-derived percentage (1/2/3 by <30/
  <60/else), the original both-resist suppression for the four elements, the fixed percents for
  NUKE/CHAOS/SHARDS/PLAS/WATE and GF_ICE's double cold pass; returns the shattered potion svals.
- `game::monster_cast` breath/ball/bolt paths queue `inven_damage_ex` svals and floor item
  `FloorBoom`s; `game::apply_gf_booms` (run after the monster loop) projects them on the player,
  the monsters (GF_OLD_HEAL/SPEED/SLOW/SLEEP/CONF included) and the floor, with recursion.
- `game::floor_hit` now uses `item::floor_damage_events` with the per-grid `(dam + r)/(r + 1)`
  scaling and returns corpse/potion booms to the same queue.
- `game::curse_equipment_ex` for CAUSE_1/2/3 (`chance`, `heavy`) and HAND_DOOM (`100`, `20`).


## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- modal.rs `apply_effect`: Manwe's Blessing (set_blessed + hero/shero/holy at 10/20/30), Tulkas'
  Divine Aim (`set_tim_deadly` at effective level 20+), Yavanna's Tree Roots (`set_roots` with the
  original duration/AC/damage) and the Blessing/Holy Chant/Holy Prayer scroll values now use the
  `game::set_*` helpers. Mediator sets blessed and all five oppose_* timers; Dispel Magic clears
  blessed.
- modal.rs `gf_terrain`: `GF_KILL_DOOR` and a new `GF_WAVE`/`GF_WATER` arm use
  `map::place_floor_convert_glass` (so a depth-rolled glass wall becomes molten glass).
- spell.rs: the Library's `BOOKABLE_SPELLS` list is the full q_library 43; a staff of Wish routes to
  `game::make_wish` through `Modal::Wish` instead of a random acquirement.
- The `### NEEDS OTHER FILE` items above (`inven_damage_ex`/`floor_damage_events`/`GF_MAKE_GLYPH`
  blocking) remain game.rs call-site work.

## WIRED/VERIFIED THIS SESSION (game.rs/modal.rs/input.rs)

- `spells1.cc:2375` `apply_nexus`: **verified already implemented** in `game::gf_player_knock` (the
  NEXU arm rolls 1d7: 3/7 `teleport_player(200)`, 2/7 onto the attacker, 1/7 save-or-level-teleport,
  1/7 save-or-corrupt) with the `@:...:NO_TELEPORT` special-level guard. Checkbox flipped; the earlier
  "only adds confuse" text was stale.
- `GF_LAVA_FLOW`/`GF_WINDS_MANA`/`GF_BETWEEN_GATE`: verified **no base spell, device or scroll in
  `lib/edit` references these types** (`tables.cc` lists the names but no `s_info`/device allocation
  reaches them), so they are data dead entries in the base module and are marked `[~]` (Theme
  leftovers); `modal::gf_terrain` needs no arms.


## DONE THIS SESSION (final gaps pass)

Files: `bevy/src/game.rs`, `bevy/src/item.rs`, `bevy/src/data.rs`.
`cargo test`: 200 passed / 0 failed.

- `spells1.cc:4105 project_m`: verified the shared `game::gf_monster_effect`
  now carries and consumes the reported cases — `RAISE` (heal/no damage),
  `PSI`/`PSI_DRAIN` (EMPTY_MIND/STUPID etc. plus the undead/demon
  backlash to `psi_backlash`), `ATTACK` (`e.attack` performs a real
  weapon blow), `GRAVITY` (`mspeed > 60` slow guard, `+1` stun,
  RES_TELE randint(100) save), `INERTIA` (`mspeed > 60` guard) and
  `UNBREATH` (UNDEAD/NONLIVING only, no glyph immunity).  The remaining
  `project_o` GF_RAISE corpse raising is called out in the bullet above.
- `spells5.cc:58 get_random_spell`: `item::random_book_spell` (Magic 75%
  / Spirituality, level-gated) now feeds `make_item`'s TV_BOOK sval 255
  path, the `[Spell]` label and the store sval-255 path; the "Spellbook
  of #" has a real spell again.
- `spells6.cc:400 mana_school_calc_mana`: verified `game::mana_base` adds
  `msp * (skill(Mana) - 34) / 100` past skill 35 alongside adj_mag_mana,
  the class/race multipliers, Eru grace and the MANA equipment boon.
- `z-rand.cc randnor` / `dice.cc dice_parse` are also fixed this session
  (report 00-core) and feed the spell/device dice.

## FINAL RECONCILIATION

Reconciliation pass 2026-09-16 (third pass, spell-table session) against the current
`bevy/src/*.rs`.  This pass converted the class/god/song rows to the original
`spells5.cc` `spell_type_set_difficulty`/`set_mana` values and replaced the port's
casting gates/failure/mana formulas with the C++ ones.  6 more bullets are now `[x]`;
4 remain `[>]`, plus 2 `[~]`.

`[x]` this pass:
- `spells5.cc:371 school_spells_init` — 142 name-matched RON rows verified against the
  C++ registrations (script-checked): `level` is the original `skill_level`, `fail` the
  original `failure_rate`, `mana`/`mana_max` the original min/max range
  (`convert_data.py` + regenerated `spells.ron`).  The Geomancy `GEOMANCY_POWERS`
  values in `game.rs` were aligned to the same calls; the two-school primary/secondary
  order now follows `spell_type_init_*` + `add_school` (Fire Golem [Fire+Mind],
  Grow Trees [Nature+Temporal], Tracker [Meta+Conveyance], Drain/Wraithform/Flame of
  Udun/Genocide [Udun+…], Banishment [Temporal+Conveyance], Wings of Winds [Air+…]).
  The only "Recall" name match left is the rod-of-Recall row (object-name resolved).
- `spell_type_set_difficulty` — `spell::required_skill` is the original school-level
  gate and `spell::spell_usable_no_inv` mirrors cmd5.cc `is_ok_spell`
  (`get_level_school(spell,50,0)` non-zero); `known_spells` no longer applies a
  character-level gate.  `spell::fail_chance` is lua_bind.cc `spell_chance_school`
  (base fail − 3*(L−1) − 3*(adj_mag_stat−1) + not-enough-mana penalty, adj_mag_fail
  floor with Perfect Casting, icky-wield +25, cmd7.cc `clamp_failure_chance` stun
  +25/+15 and 95 cap).  FAST_CAST is a port-only concept and is now ignored.
- `spell_type_set_mana` / `spell_type_mana_range` — `SpellRow.mana_max` carries the
  second argument and `spell::spell_mana`/`spell_mana_no_inv` implement lua_bind.cc
  `get_mana` (level-scaled between the row's min and max); `spend_mana` uses it.
- `spell_type_skill_level` — see above: `SpellRow.level` is the C++ field, no longer a
  port character-level/skill gate.
- `teleport_to_player` — `item::teleport_player`/`plan_tport_drags` plus the input.rs
  wire-up landed in the parallel session; game.rs's own player teleports (GF_GRAVITY,
  GF_NEXUS, monster TELE_AWAY, the cursed `teleport_itis`) record
  `TurnState.tport_old` and `game::drag_tport_monsters_game` runs the drag at the end
  of the same world turn (awake SF_TPORT, RES_TELE save, glyph exclusion).
- `spells5.cc:58 get_random_spell` — `spell::random_type` + `spell::random_spell`
  implement the C++ function (skill-level*3 gate, Magic/Spirituality/Music/NO_RANDOM);
  the RON rows no longer need a separate `random_type` field because it derives from
  the row (school/god).

`[>]` (0): the last one (`randart.cc:336-362` naming) closed this session — the
Artifact Creation scroll now prompts through `Modal::ArtifactName` (modal.rs:10733).

Closed since the third pass (modal.rs/item.rs session): `project_f` (Yavanna piety now wired
into `gf_terrain`'s fire/nether/disintegrate tree arms), `project` (`GfFlags` pass set added;
bolts/beams pass `GF_KILL`, balls `GF_ALL`; HIDE remains visual-only with no Bevy counterpart)
and `spell_type_random_type` (`random_book_spell` now calls `spell::random_spell`).

`[~]` (2):
- `convey_teleport` cast-time discount — the range (`100 + get_level_s(TELEPORT,100)`)
  is faithful, but `p_ptr->energy -= 25 - get_level_s(TELEPORT,50)` needs a player
  energy accumulator and partial-turn scheduling.  The port's action model is a binary
  full turn (`TurnState.world_turn`; `Monster.energy` is monster-only), so a discount
  below a turn or a bonus partial action cannot be represented; implementing it means
  introducing a player energy pool and acting on the world tick when it crosses 100.
- `GF_LAVA_FLOW`/`GF_WINDS_MANA`/`GF_BETWEEN_GATE` — data-dead in the base module (no
  live caller).

Notes (outside the `[>]` scope, cross-file, left open):
- modal.rs's god-spell path subtracts the raw `sp.mana` from grace
  (`modal.rs:9457/9475`) instead of the level-scaled `spell::spell_mana`; the cast-list
  display still shows the minimum.
- `item.rs::random_stick_spell` keeps its own get_random_stick roll (the C++ picks by alloc
  rarity, not by `random_spell`); only `random_book_spell` routes through `spell::random_spell`.
- The `project_o` bullets still self-document the `GF_RAISE`/`GF_RAISE_DEMON`
  corpse-raise as unimplemented (no RAISE arm in `item::floor_damage_events`).

Session log (spell-table/game-side, 2026-09-16; `bevy/src/game.rs`, `bevy/src/spell.rs`,
`bevy/src/data.rs`, `bevy/tools/convert_data.py`, `bevy/assets/data/spells.ron`):
- Converter/table: every class/god/song/device row now uses the original `skill_level`,
  `failure_rate` and mana min/max (new `mana_max` field, 142 rows script-verified against
  `spells5.cc`); two-school primary/secondary order matches `spell_type_init_*`+`add_school`.
- `spell.rs`: `required_skill` = `skill_level`; `spell_usable_no_inv` mirrors cmd5.cc
  `is_ok_spell`; `known_spells` drops the character-level gate; `fail_chance` = lua_bind.cc
  `spell_chance_school` + cmd7.cc `clamp_failure_chance`; `spell_mana`/`spell_mana_no_inv` =
  lua_bind.cc `get_mana`; `random_type`/`random_spell` = spells5.cc `get_random_spell`;
  `level_s_from` keeps the original min parameter.
- `game.rs`: Geomancy table aligned to the C++ registrations; `TurnState.tport_old` +
  `drag_tport_monsters_game` (teleport_to_player for the game.rs player teleports);
  corpse `found_aux3/4` (`level_or_feat`).
- Tests: `cargo test` 209 passed / 0 failed.

Session log (modal.rs/render.rs, 2026-09-16): detect_all + mimic reveal closed.
- `detect_all_ctx` (modal.rs:2822) runs the seven sub-detections of spells2.cc
  detect_all (doors/stairs via `detect_doors_stairs_ctx`, treasure via
  `power_detect_treasure`, gold/normal objects via `detect_objects_ctx`, invisible/normal
  monsters); the `"detect"` arm (modal.rs:16074) routes ACT_DETECT_ALL/ACT_DETECT_XTRA/
  ACT_DRUEDAIN/ACT_THRAIN through it while ACT_ORCHAST keeps the orc-only count.
- `detect_monsters_string_ctx` (modal.rs:2732) marks the object-glyph mimics inside the
  object/treasure scans ("!=?|" / "$"); `render.rs:431-438` now shows a detected sleeping
  mimic as its imitated object even out of sight.
- Randart naming: `Modal::ArtifactCreate` now creates the artifact with the default
  "of '<player>'" name and opens the shared `Modal::ArtifactName` prompt
  (modal.rs:281/10733), reusing the `Modal::Wish` typing loop (`text_input_frame`,
  modal.rs:8895).  A typed name becomes "called '<name>'" (`artifact_name_from`,
  modal.rs:13366); empty/blank or Esc keeps create_artifact's default (randart.cc:336-362).
  The scroll is consumed and the turn spent in `finish_artifact_create` (modal.rs:13339).
- Tests: `artifact_name_keeps_default_when_blank`, `wield_slot_checks_current_body_parts`
  (`cargo test` 221 passed / 0 failed).
