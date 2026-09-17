# 00-core audit report

Group: `game.cc` (non-power), `files.cc`, `init1.cc`, `init2.cc`, `modules.cc`, `options.cc`,
`message.cc`, `messages.cc`, `notes.cc`, `hiscore.cc`, `loadsave.cc`, `level_data.cc`,
`level_marker.cc`, `levels.cc`, `quest.cc`, `hooks.cc`, `dice.cc`, `z-rand.cc`, `tables.cc`,
`z-form.cc`, `z-util.cc`, `squeltch.cc`.
Inventories: `inventory/00-core.md`, `inventory/09-tables.md`.

Method: every `init*_txt` parser field and every `tables.cc` table was compared against
`bevy/tools/convert_data.py` and the Rust consumers. Line numbers are C++ source unless stated.

---

## GAPS

### A. Data-parser fields dropped by convert_data.py (gameplay impact)

- [x] `init1.cc:2474-2511` `init_k_info_txt` `A:` allocation / `init2.cc:684-777` `init_alloc`
      (`k_info` `A:<level>[/<chance>]` pairs) — done: converter emits `ObjectDef.alloc` and
      `item.rs get_obj_num` builds the locale table with `prob = max(1, 100/chance)` and the
      original 60%/10% better-level re-rolls.

- [x] `init1.cc:2424-2439` `init_k_info_txt` `T:<btval>:<bsval>` — done: `ObjectDef.btval/bsval`
      and the `TR_NORM_ART` fallback in `item.rs make_item` (k_info 296/298/573/700/720/743/776/
      801/802/810/811).

- [x] `init1.cc:2545` k_info `f:` obvious flags — verified no flag is `f:`-only in base data
      (k_info/a_info/e_info `f:` sets are subsets of the `F:` sets), so no gap.

- [x] `init1.cc:4157-4178` `init_r_info_txt` `E:` monster body parts (884 lines) — fixed this
      session: `MonsterDef.body_parts` + converter parse/emit (883 records). Consumed nowhere yet
      (Rust hardcodes standard body parts); see NEEDS CONSUMER. E.g.
      `r_info.txt:221 E:0:1:0:2:1:0`, DeathMold-style 4-finger/0-head entries.

- [x] `init1.cc:4181-4198` `init_r_info_txt` `O:<treasure>:<combat>:<magic>:<tools>` monster drop
      theme (886 lines) — converter emits `MonsterDef.objs` and `item.rs monster_carried_treasure`
      passes it into `make_object_themed` / `kind_is_theme`.

- [x] `init1.cc:4098-4106` `init_r_info_txt` `D:` monster description/memory text (1533 lines) — (19th-round follow-up: consumed by Modal::SymbolBrowser, modal.rs:7792)
      fixed this session: `MonsterDef.desc` + converter parse/emit (joined exactly as the C++
      `strappend` does, preserving trailing spaces). No recall/observe UI consumes it yet; see
      NEEDS CONSUMER.  Reconciled: still no consumer — `MonsterDef.desc` (data.rs:154-157) is
      unread; the monster-recall/knowledge page would go in `modal.rs::knowledge_text`
      (modal.rs:5798).

- [x] `init1.cc:4668-4691` `init_re_info_txt` `S:1_IN_n` — done: converter stores
      `spell_freq = 100/n` and `apply_monster_ego` takes `max(base, ego)`.

- [x] `init1.cc:4693-4733` `init_re_info_txt` `T:MF_ALL` (3 records: re_info lines 65/102/192) —
      done: `EgoMonDef.remove_spells` + `apply_monster_ego` clears all inherited spells on
      `MF_ALL`.

- [x] `init1.cc:4490-4509` `init_re_info_txt` `W:<mlev>lev:<rar>:<mwt>wt:<mexp>exp:<pos>` — done:
      `apply_monster_ego` applies the weight/level/exp/speed/sleep modifiers. This session also
      added the C++ `MODIFY` floors (speed >=50, sleep >=0, weight >=10).

- [x] `init1.cc:3531-3551 / 3578-3586 / 3630-3640 / 3642-3682` `init_e_info_txt`
      `R:` flag-rarity groups, `r:N:`/`r:F:` need/forbid base flags, `a:` activation,
      `W:` rarity1 — done by a previous agent and verified this session:
      - `R:` tiers are in `EgoDef.groups` and rolled per group with `magik(chance)` in
        `item.rs apply_ego`; this session also added the group `f:` obvious flags (`oflags`).
        Remaining consumer defect: `item.rs item_flags` still ORs the whole `EgoDef.flags` union;
        the converter now limits that flat list to `R:100` (always-on) flags + generation flag
        names, so <100 flags no longer leak, but the consumer should stop reading `e.flags` at all
        (rolled flags are already on the instance). See NEEDS CONSUMER.
      - `r:N:`/`r:F:` are in `EgoDef.need_flags/forbid_flags` and filter bases in `apply_ego`.
      - `a:` activations are in `EgoDef.activate` (9 egos).
      - `W:` stores `rarity1` and `rarity` (=mrarity) separately; `apply_ego` uses both.

- [x] `init1.cc:1008-1022` `init_player_info_txt` `R:E:` race body parts (21 lines; e.g. DeathMold
      `R:E:1:1:1:4:0:0`) — fixed this session: `RaceDef.body_parts` + converter parse/emit (and the
      same for class `C:E:`). C++ `xtra1.cc:1977-2007 calc_body` sums race+subrace+class body parts
      to decide weapon/torso/finger/head/arms/legs slots; Rust still hardcodes the standard body
      (`mimic.rs slot_usable`), so the values are carried but not consumed yet; see NEEDS CONSUMER.

- [x] `init1.cc:1455-1490` `init_player_info_txt` `C:D:1:` class level titles (60 lines) — fixed (19th-round follow-up: consumed by the death screen, hud.rs:323-334)
      this session: `ClassDef.titles` + converter parse/emit (10 per class). C++ displays
      `cp_ptr->titles[(lev-1)/5]` (`files.cc:4072`, `xtra1.cc:328`); no Rust UI consumes the list
      yet; see NEEDS CONSUMER.  Reconciled: still no consumer — `ClassDef.titles`
      (data.rs:350-353) is unread by `hud.rs`/`modal.rs`; needs the character-sheet display.

- [x] `init1.cc:2000-2243` `init_f_info_txt` + `feature_flag_list.hpp` — data completed by the
      data-pipeline session: all 19 feature flags plus `M:` mimic, `D:0/1/2` texts and `E:` damage
      effects are in `TerrainDef`/`terrain.ron` (`web`, `can_run`, `notice`, `dont_notice_running`,
      `door`, `support_light`, `attr_multi`, `mimic`, `desc`, `tunnel_desc`, `block_desc`, `effects`).
      Consumers (map/input session): `WEB` blocks non-spiders in `input.rs::player_can_walk`
      (possessed RF_SPIDER body / Spider mimic pass; monsters already handled in
      `game.rs::monster_can_enter`); `CAN_RUN`/`DONT_NOTICE_RUNNING` drive the run algorithm
      (`see_obstacle_grid`/`run_test`) from the parsed flags; `tunnel_desc`/`block_desc` are the
      tunneling/blocked messages; `TerrainDef.effects` is pinned to `map::terrain_effect` by a test
      (frequencies were off by 10x).  `NOTICE` is only used by travel (`xtra2.cc interesting_feature`),
      which the port does not have; `support_light` is never read in base C++ (flag-list-only) and
      `attr_multi` is a `map_info` render shimmer (render.rs).  `TerrainDef.mimic` is now used by
      `Map::display_terrain`/`is_wall` and map/wild-border mimics.

- [x] `init1.cc:4976-4993` `init_d_info_txt` `O:` dungeon object theme (28 lines) — converter
      emits `DungeonDef.theme`; consumed this session at the game.rs call sites
      (`populate_level` vault objects, spec/quest `F:*` random objects). `item::make_object`
      still passes the empty theme for the item.rs `scatter_objects` path.

- [x] `init1.cc:6020-6618` `process_dungeon_file_aux` `F:` random markers — fixed this session as
      data: `parse_pref` now preserves the `*` flag/offset and `build_quest_map` emits
      `random_monsters`/`random_objects` (level offset + x/y) plus `mimics` per cell into
      `questmaps.ron` / `speclevels.ron` (`data.rs QuestMapRandom`/`QuestMapMimic`). The old
      wrong-fixed placements are gone:
      - `s_gates.map:44 F:8:1:0:*94:*92` → random monster at depth+94 and object at depth+92
        (no longer fixed r_info 94 / k_info 92).
      - `s_doom.map:30 F:*:86:0:0:*` → random object at depth+0 (bare `*`).
      - `thieves.map:29 F:b:...:43:*:...` → random ego: the original parses `RANDOM_EGO` but never
        reads it when placing (same for `RANDOM_FEATURE`/`RANDOM_ARTIFACT`), so dropping it is
        exact.
      - `F:` field 8 (`mimic`) is now parsed (thrain.map 43 cells, s_doom.map 12).
      No consumer yet in `map.rs`/`game.rs`; see NEEDS CONSUMER.

- [~] `init1.cc:5250-5298` `init_d_info_txt` `@:<d>:S:<ext>` save extension — converter still drops
      it (`@:` handles N/D/U/F/B only). Only 1 occurrence (`d_info.txt:155 @:14:S:mdm`), and bevy
      has its own save format (no level save extensions), so this is not portable; note only.

### B. tables.cc tables not reflected in Rust

- [x] `tables.cc:962 extract_energy[300]` — no Rust equivalent. C++ turns speed into fractional
      energy (`dungeon.cc:3734 p_ptr->energy += extract_energy[speed_use]`; values 1..49). Rust
      `game.rs:8057-8062` gives the player a *probabilistic* free world turn
      (`if speed > 0 && rng.gen_range(0..100) < speed.min(50) { return; }`), which (a) never
      penalises **negative** speed (slow/hasted monsters cannot get extra turns vs a slowed
      player) and (b) has a different distribution at every speed. Suggested landing:
      `bevy/src/game.rs` player energy accumulator + `extract_energy` table.

- [x] `tables.cc:1002 player_exp[PY_MAX_LEVEL]` — Rust replaces it with a quadratic
      (`game.rs:3963 10 * level * (level+1) * exp_factor / 100`, used by
      `PlayerState::exp_needed` and `check_experience`). At factor 100: level 50 requires
      **25 500** exp in Rust vs **5 000 000** in C++; level 30: 9 300 vs 150 000. The whole
      levelling curve (and LEVELS-artifact `elevel` thresholds, `game.rs:5558`) is off by orders
      of magnitude. This is the largest single gameplay discrepancy found.

- [x] `tables.cc:319/365/411/457` `adj_dex_ta`, `adj_str_td`, `adj_dex_th`, `adj_str_th` — Rust
      substitutes `PlayerState::stat_bonus = (stat-10)/2` (`game.rs:1619`). C++ adds
      `adj_*` lookups (`xtra1.cc:3396-3405`). Consequences: melee to-hit uses a STR bonus that has
      no C++ counterpart (`game.rs:1608 melee_bonus`), melee damage gets **no** STR bonus at all
      (`input.rs:2339 damage_bonus` has only totals/tactic/Combat skill), and AC uses a linear DEX
      bonus (`game.rs:3653 player_ac`) instead of `adj_dex_ta`.

- [x] `tables.cc:273 adj_wis_sav` — Rust saving throw is
      `20 + 2*level + stat_bonus(WIS)*5 + tactic` (`game.rs:3266 player_sav`), C++ is
      `tactic + adj_wis_sav[stat_ind] + scale(SPIRITUALITY,75) + 10` (`xtra1.cc:3155,3787,3796,3924`).
      Different values and no Spirituality term.

- [x] `tables.cc:825 adj_con_mhp` — the table exists in Rust only for sanity
      (`game.rs:3686 ADJ_CON_MHP` + `calc_sanity`). HP per level C++ is
      `player_hp[lev-1] + (adj_con_mhp[CON]-128)*lev/2` (`xtra1.cc:1708-1718`), Rust uses a flat
      `(CON-10)/2` (birth `birth.rs:270`, rerate `game.rs:4065`) — CON has almost no effect on HP.

- [x] `tables.cc:89 adj_mag_mana` + mana formula — Rust `sync_magic_mana` (`skill.rs:306-320`) and
      birth (`birth.rs:280-284`) compute max mana as `(1 + (max(INT,WIS)-10)/2) * mana_mult/100`;
      C++ is `scale(MAGIC,200) + adj_mag_mana[max(INT,WIS) index]*level/4 + 1`, then race/class
      mana %, Eru grace bonus (`xtra1.cc:1536-1600`). The INT/WIS table term is missing entirely.

- [x] `tables.cc:595 adj_str_dig` (+ skill_dig) — Rust digging is a constant
      `(DIG_TURNS - tunnel)` strike count (`input.rs:1479,2752`); C++ builds `skill_dig` from
      STR + Digging skill + level and rolls against per-terrain thresholds
      (`xtra1.cc:3790`, `cmd2.cc:1148-1353`).

- [x] `tables.cc:733 adj_dex_safe` — used by C++ door bashing fall/stun save
      (`cmd2.cc:1514`) and falling. Rust `force_door` (`input.rs:1414-1430`) approximates bash
      chance from `melee_bonus` and never checks DEX or the "fall through the door" path.

- [x] `tables.cc:2303 flags_groups()` (12 realms: Fire/Cold/Acid/Lightning/Poison/Air/Earth/Mind/
      Shield/Chaos/Magic/Antimagic) — not ported. `object1.cc:5477-5650 gain_flag_group`,
      `get_flag`, `gain_flag_group_flag` are the LEVELS-artifact growth: on a level roll ≥66 the
      weapon gains a realm and then a random unused flag from an owned realm; roll 33-65 has a
      `NEW_GROUP_CHANCE` 40 % group gain (`object_gain_level`). Rust `game.rs:5551-5591` only rolls
      `+to_h/to_d/pval`, so level-gaining weapons never acquire realm powers/properties.

- [x] `tables.cc:1755 martial_arts ma_blows[MAX_MA]` / `1776 bear_blows` — not ported. Rust
      unarmed/bear attacks use `("1d2", 5)` and the generic "You hit the X." message
      (`input.rs:2306-2309,2442`); C++ uses the per-skill-rank tables with distinct verbs and
      effects (`MA_KNEE`, `MA_SLOW`, `MA_STUN`, `MA_WOUND`, `MA_FULL_SLOW`). Barehand/Bearform
      combat loses its damage dice and status effects.

- [x] `tables.cc:895 blows_table`, `52 ddd/ddx/ddy` (`game.rs:2640-2700`, `input.rs:49 DIRS`),
      `1084/1092 stat_names`, `1060 color_names` (UI), `2156 tactic_info` (`game.rs:1666`),
      `2231 move_info` (`game.rs:1680`), `2248 inscription_info` (engrave system),
      `2173 activation_info` (converter `parse_junkarts` + `spell.rs:528`), `1150
      artifact_names_list` (converter), `1789-1982 magic_power` tables (modal.rs:1180+),
      `3094 monster_powers` (modal.rs:1087+) — all covered.

- [~] `tables.cc:1990 deity_info` (names only in `birth.rs:234`), `3190/3237 tvals/tval_descs`
      (UI labels), `3481 between_exits` (no base producer of `FEAT_BETWEEN2`),
      `3507 max_body_part` (Rust caps hardcoded in mimic), `3520 gf_names` (Rust
      `game.rs:510 gf_description` covers used types), `3616 modules` (module system not ported).

### C. z-rand / dice semantics

- [x] `z-rand.cc:154-187 randnor` — Rust `item.rs:1763-1771 randnor` approximates a normal
      distribution by averaging four uniforms and, when `stand <= 0`, returns `mean` instead of
      C++'s **0** (`z-rand.cc:160` returns 0 for stand<1). `m_bonus` (`object2.cc:1808`, Rust
      `item.rs:1773-1786`) depends on it, so `m_bonus(1..3, level)` (ammo pval, potions;
      `cmd7.cc:1875,1911,1945`) is always 0 in C++ but can be >0 in Rust, and the value
      distribution differs for every caller.

- [x] `dice.cc:17-66 dice_parse/dice_roll` vs device charges — Rust special-cases the `"N+dM"`
      charge strings but rolls `base + rng.gen_range(0..die)` (`item.rs:1839-1843`), i.e.
      `base..base+M-1`; C++ `dice_roll` is `base + damroll(1, M)` = `base + randint(M)` =
      `base+1..base+M`. Every wand/staff charge count is off by one (can also be 0 extra).

- [x] `rand_int` (0..m-1), `randint` (1..m), `rand_range` (inclusive), `rand_spread`, `damroll`,
      `maxroll`, `magik` — behaviour matched where used. RNG plumbing `[~]` (bevy RNG).

### D. Options with default-on gameplay semantics not reproduced

- [x] `options.hpp:51 smart_learn = false` — **Partial this session**: the flag now exists on the
      `options::Options` resource with the false default (`bevy/src/options.rs`).  Rust
      `game.rs:10396 update_smart_learn` / `game.rs:10564 remove_bad_spells` still have no option
      check, so monsters always learn (default difficulty remains harder than base); the two
      early-return guards are the remaining one-line game.rs change.

- [x] `generate.cc:8433` `auto_scum = true` — **Partial this session**: `Options::auto_scum`
      exists with the true default.  The generator still re-rolls nothing
      (`game.rs:12201 map::generate_dungeon_level` calls `map::generate_level` once).

- [x] `options.hpp:52-53 small_levels = true / empty_levels = true` (and `always_small_level`,
      `ironman_rooms`) — **Partial this session**: all four flags exist on `options::Options` with
      the original defaults.  `map.rs` sizing still only honours `SMALLEST`/`SMALL`/`BIG` d_info
      flags (`map.rs:866-872`); the random small/empty levels (`generate.cc:8338`) and arena levels
      (`generate.cc:6503`) are not implemented.  Reconciled (second pass): `small_levels`/
      `empty_levels`/`always_small_level` are consumed in `map::level_size` (map.rs:3978) and the
      empty/arena levels (map.rs:4285), and `ironman_rooms` now drives the unusual-room rolls —
      `RoomsGen.ironman` skips the `roomdep` gates and forces the type-8/7 vault rolls
      (generate.cc:6367/6632-6638).  The map.rs pre-pass vault roll is the legacy test generator
      and is unaffected.

- [x] `options.hpp:38 wear_confirm = true` — **Partial this session**: the flag and the
      `options::wear_confirm_needed` helper (cursed + `object_known_p`) exist.  Rust
      `modal.rs::wield_item` still has no call, so no prompt is shown yet.  Reconciled (second
      pass): done — `modal.rs:12353` gates wielding on `ctx.options.wear_confirm &&
      options::wear_confirm_needed(...)` and opens the confirmation modal.

- [x] `files.cc:3798 do_cmd_suicide` / `dungeon.cc:3610` — **Done in a previous session**:
      Ctrl+Q is the original `Q` suicide (two presses within 5 s -> `AppState::Dead`), Shift+Q is
      the port's save+quit.  `died_from`/highscore text is not modelled (death screen shows the
      port's own message).

- [~] `options.hpp:38-60` `carry_query_flag`, `always_pickup` (Rust matches the false default: only ammo is picked up
      while walking, `input.rs:1709`), `find_*`/`disturb_*` (no run command in Rust), `preserve`
      (CreatedArtifacts covers it), `point_based`/`autoroll`/`linear_stats`/`view_*` (birth/UI),
      `autosave_l/t` (default off), `no_selling`/`fate_option`/`joke_monsters` (implemented/off).

### E. Other core files

- [x] `squeltch.cc` (whole file) + `src/tome/squelch/*` — **Done this session**
      (`bevy/src/squeltch.rs`): the full rule model (all `src/squelch` conditions incl.
      inventory/equipment/status/skill/ability, and/or/not, destroy/pickup/inscribe), first-match
      application on the floor (walk/after kill) and in the pack every turn, `object_status`,
      `object_aware_p`, `easy_add_rule` from `do_cmd_destroy`, and RON save/load
      (`automat.ron` / `<player>.automat.ron`) via `automatizer_load` on game entry.  The
      full-screen `= T` editor is UI and remains unported (rules are added from destroy and the
      RON file is hand-editable).

- [x] `notes.cc` (whole file) — **Done this session** (`bevy/src/notes.rs`): append-only
      `notes.txt` with the exact `"Turn %12d <depth> %c: text"` format, the user note command
      (`:` in-game), and the birth / winner / save-game / enter-dungeon type blocks.  Hooks:
      level-up (`notes_tick`), unique kill and artifact pickup (input.rs), session start/end
      (OnEnter, quit path).  The Knowledge-window notes page is UI and remains unported.

- [x] `init2.cc:889-928 init_guardians` — done: the converter bakes the init2 marking into
  `monsters.ron` (`SPECIAL_GENE` on every final guardian, plus `DROP_RANDART` when the dungeon has
  no FINAL_ARTIFACT/FINAL_OBJECT). Only Glaurung 715 and the two flag-less guardians
  (Melkor 1044, Glass Golem 1033) actually changed; `item.rs monster_carried_treasure` already
  reads `DROP_RANDART` from the static monster, so no Rust change was needed.

- [x] `modules.cc:542-570 drunk_takes_wine` (common, non-Theme HOOK_GIVE) — giving Ale/Wine
      (`TV_FOOD` sval 38/39) to "Singing, happy drunk" (r_info 15) prints "'Hic!'" and leaves an
      empty bottle. Rust `Modal::Give` has no such hook (no `Hic`/`happy drunk` hits).

- [~] `files.cc:749 process_pref_file_expr` + `files.cc:664-688 X:/Y:` option switches — user pref
      files can force options on/off.  `[~]`: the port has no pref/user-file parser at all
      (`grep -i pref bevy/src/` = 0 in input.rs), options are bevy-native; nothing to hook into.

- [~] `z-form.cc`, `z-util.cc` (Rust `std::fmt` / own logging), `message.cc`/`messages.cc`
      (MessageLog does add/count/dedup), `hiscore.cc` (known done), `loadsave.cc` (own save
      format), `level_data.cc`/`level_marker.cc`/`levels.cc` (branch stairs, special level flags
      and LevelStore done), `game.cc` (power table only — audited separately).

---

## UNCERTAIN

- `init1.cc:1432-1449` `C:N:<index>:<display_order>:<name>` — C++ index is the first number
  (Warrior 0 … Priest 5); `convert_data.py:269-270` stores the **display order** as `ClassDef.id`
  (0,3,1,2,5,4). Rust looks classes up by name/vector position everywhere I checked, so this looks
  harmless, but the `id` field is not comparable to the C++ index.
- `init1.cc:2424 k_info T:` — the 11 entries look like runtime-specific "specific artifact kinds";
  I could not find a normal play path that reaches `object2.cc:3752` (requires `TR_NORM_ART` +
  a runtime `k_ptr->artifact` marker), so impact may be nil.
- `tables.cc:3507 max_body_part` — Rust mimic code appears to cap arms/legs at 3/2; I did not
  verify every cap (2 weapons → 3, fingers 6, etc.) against `xtra1.cc:2010-2018`.
- `files.cc:3942 total_points` multiplier is skipped on purpose in `scores.rs:56-63`
  ("we have no preserve/scum switches"), so default scores differ by the ×26/20 factor; flagged
  as known-done in the task context, not counted as a new gap.
- `init2.cc:497-500 init_v_info` / `v_info.txt` `Y:` records — parser reads them but no consumer
  found; treated as dead data (`[~]`).

---

## COVERAGE SUMMARY

`00-core.md` checkboxes: total **250**; `[x]=70`, `[>]=15`, `[ ]=10`, `[~]=155`.
(`game.cc`, `options.cc`, `message.cc`, `messages.cc`, `level_data.cc` and `level_marker.cc` have
no per-function checkboxes in the inventory; `tables.cc` tables were added to `09-tables.md`.)

`09-tables.md`: total **51**; `[x]=17`, `[>]=12`, `[ ]=2`, `[~]=20` — of which tables.cc has 43
table entries (`[x]=17`, `[>]=12`, `[ ]=2`, `[~]=12`), z-form 5 `[~]`, z-util 3 `[~]`.

---

## DONE THIS SESSION

Data-pipeline session (`bevy/tools/convert_data.py`, `bevy/src/data.rs`, generated
`bevy/assets/data/*.ron` only):

- `k_info` `A:`/`T:`/W: — already implemented by a previous agent; verified against
  `init2.cc init_alloc` and `object2.cc apply_magic` (A: locale/chance, NORM_ART `btval`/`bsval`
  fallback). No further change needed.
- `r_info`:
  - `E:<weapon>:<torso>:<arms>:<finger>:<head>:<legs>` parsed into `MonsterDef.body_parts`
    (883 records) and emitted.
  - `D:` text parsed into `MonsterDef.desc` (joined exactly like the C++ `strappend`).
  - `I:` now also exposes `hdice`/`hside` on `MonsterDef` (needed by the monster level-up code).
  - `O:` drop theme verified present and consumed (`item.rs monster_carried_treasure`).
- `re_info` `apply_monster_ego` now applies the exact `MODIFY()` floors of `monster2.cc`
  (`speed >= 50`, `sleep >= 0`, `weight >= 10`); `S:1_IN_n`, `T:MF_ALL`, W: modifiers re-verified.
- `e_info`:
  - `f:` obvious flags are parsed into per-group `EgoFlagGroup.oflags`.
  - `EgoDef.flags` is now limited to the `magik(100)` always-on flags plus generation flag names;
    the per-rarity `R:` groups carry the rolled flags (previously the flat union defeated rarity).
  - `r:N:`/`r:F:`, `a:`, `W:` re-verified.
- `p_info`: `R:E:` race body parts and `C:E:` class body parts parsed; `C:D:1:` level titles parsed
  into `ClassDef.titles` (60 titles).
- `f_info`: every flag in `feature_flag_list.hpp` is now represented on `TerrainDef`
  (`web`, `can_run`, `notice`, `dont_notice_running`, `door`, `support_light`, `attr_multi` were
  missing); `M:` mimic, `D:0/1/2` texts and `E:` damage effects (freq x10) are parsed too.
- `d_info`: `O:` object theme already emitted; `@:S:` save extension documented as not portable
  (bevy save format), see `[~]` above.
- `init2.cc init_guardians`: baked into `monsters.ron` — `SPECIAL_GENE` on every final guardian
  and `DROP_RANDART` when the dungeon has no final artifact/object (only Glaurung 715, Melkor 1044
  and Glass Golem 1033 changed; now consumed via the existing `DROP_RANDART` path).
- Fixed maps: `F:...:*N` random monster/object markers are preserved as
  `QuestMapRandom` lists (`random_monsters`/`random_objects`) on `QuestMapDef` and `SpecLevelDef`,
  and `M:` mimic cells as `QuestMapMimic` (`mimics`). The previous wrong fixed placements are gone.
  The original's `RANDOM_FEATURE`/`RANDOM_EGO`/`RANDOM_ARTIFACT` bits are parsed but never used
  when placing in `process_dungeon_file_aux`, so dropping those is exact.

Verification: `tools/.venv/bin/python tools/convert_data.py` is idempotent, `cargo check`
and `cargo test` are green (166 passed) with the regenerated RON.

## NEEDS CONSUMER

Fields are now in `data.rs`/RON but nothing reads them yet (do not hand-edit; add consumers
where noted):

- `bevy/src/mimic.rs::slot_usable` — **consumed this session**: `calc_body` sums
  `RaceDef`+`RaceModDef`+`ClassDef`/`MonsterDef.body_parts` with the `max_body_part` caps,
  Bear zeroing and extra limbs; `slot_usable_body`/`enforce_body`/`drop_unusable_slots` use it.
  Remaining: the UI call sites (`modal.rs`, `item.rs`) still show slots via the gd-less
  `slot_usable`, so DeathMold head/leg slots are only enforced on transform.
- Monster recall / observe / possession UI — `MonsterDef.desc` (memory text) still unread.
- ~~`bevy/src/input.rs::player_can_enter` + `walkable` — `TerrainDef.web`~~ **done this session**
  (spiders pass, all others blocked; `can_run`/`dont_notice_running` drive the run algorithm;
  `notice` remains travel-only).
- Terrain messages/effects — **done this session** for `tunnel_desc`/`block_desc`/`effects`
  (frequencies corrected to the x10 values); `desc` (D:0 look text) and `support_light`/
  `attr_multi` remain renderer-only (`support_light` is dead in base C++; `attr_multi` is a
  `map_info` shimmer).
- Character sheet UI — `ClassDef.titles` (`titles[(lev-1)/5]`, `xtra1.cc:328`).
- `bevy/src/map.rs`/`game.rs` fixed-map generation — **consumed this session**:
  `map::roll_map_markers` records the markers and the game.rs spawner now
  spawns `VaultSpawns.random_monsters`/`random_objects` for both quest
  (`build_quest_level`) and `@:` special (`build_spec_level`) levels, and
  copies `SpecLevelDef.mimics` into `Map.mimic` (render.rs already draws
  `display_terrain`).
- `bevy/src/item.rs::item_flags` — stop ORing `EgoDef.flags`; rolled group flags already live on
  `Item.flags`, and the flat list still carries generation flag names (`SUSTAIN`, `PVAL_M2`, ...).
- ~~`bevy/src/map.rs` level object generation — pass `DungeonDef.theme` into
  `item::make_object_themed` (d_info `O:` currently never reaches generation).~~
  **Done this session** at the game.rs call sites; `item::scatter_objects`
  (item.rs) still rolls the empty theme.
- Note: the guardian `DROP_RANDART`/`SPECIAL_GENE` marking is consumed already; `hdice` is not
  (only `hside` is used by monster level-up).

## DONE THIS SESSION (consumers, map/input/mimic ownership)

- **f_info terrain flags** (`init1.cc:2000-2243`): `web` (player entry), `can_run`,
  `dont_notice_running`, `tunnel_desc`/`block_desc`, `effects` (frequencies fixed), `mimic`
  (`Map::display_terrain`, wild borders, `is_wall`) are now consumed — see the updated items above.
- **p_info body parts** (`R:E:`/`C:E:`/`S:E:`): `mimic::calc_body`/`slot_usable_body`/
  `enforce_body`/`drop_unusable_slots` consume them (race+subrace+class sums, caps, Bear,
  possessed bodies).
- **Fixed-map `F:`/`M:` markers**: `map::roll_map_markers` + `VaultSpawns.random_*` +
  `Map.mimic`; the spawner wiring (spec/quest `build_spec_level`/`build_quest_level`) is
  **done this session** in game.rs — see the `## WIRED THIS SESSION` section below.
- **`files.cc` `X:/Y:` pref expression**: `[~]` n/a (no pref parser in the port).
- `TerrainDef.door`: still only used via the `32..=48` ranges in `map.rs`; jammed doors have
  `door:false` in the data while the ranges include them, so the flag is not a safe replacement
  (low impact; rendering-only in C++).

## DONE THIS SESSION (automatizer / notes / options, input-side)

Files: `bevy/src/squeltch.rs`, `bevy/src/notes.rs`, `bevy/src/options.rs` (new),
`bevy/src/input.rs`, `bevy/src/main.rs`.  See the matching section in
`reports/01-commands.md` for the full description.

- `squeltch.cc` + `src/squelch/*`: full rule engine ported and wired (floor on
  walk/kill/pickup, pack every turn), RON persistence, `easy_add_rule` from
  destroy; the `= T` screen is UI-only and unported.
- `notes.cc`: append-only `notes.txt`, `add_note`/type blocks and the
  birth/level/unique/artifact/win/save/enter hooks; the Knowledge notes page is
  UI-only and unported.
- `options.hpp`: `options::Options` resource with the base defaults; enforced
  semantics: `always_pickup`; `wear_confirm` helper exported.  `smart_learn` and
  `auto_scum` still need their `game.rs` consumers (documented in `options.rs`);
  `small_levels`/`empty_levels`/`always_small_level` **are consumed now** in
  `map::level_size` (see below).

Test status: `cargo check` clean, `cargo test` **196 passed / 0 failed**.

## WIRED THIS SESSION (game.rs/map.rs/save.rs ownership)

- **Dungeon `O:` theme**: `DungeonDef.theme` now reaches `item::make_object_themed`
  at the game.rs call sites (`populate_level` vault objects, spec/quest
  `F:...:*N` random objects).
- **`options::Options` generation consumers**: `map::level_size` honours
  `small_levels` (1 in 6), `empty_levels` (1 in 15) and `always_small_level`
  incl. the `DF_BIG` suppression; `game::generate_current_level` passes the live
  resource and gates `auto_scum` on it.
- **Fixed-map `F:`/`M:` markers**: spec levels now roll and spawn
  `VaultSpawns.random_monsters`/`random_objects` and populate `Map.mimic`;
  quest levels spawn their recorded markers too.
- **`is_quest`** (random princess quests): no destroyed levels and up-only
  staircases, via `generate_dungeon_level_for`.
- **`@:` level flags**: `game::level_has_flag` merges `special_at` flags for
  the GF_NEXUS `NO_TELEPORT` check.


## WIRED THIS SESSION (input.rs)

- `modules.cc:542-570` `drunk_takes_wine` plus the two sibling base-module HOOK_GIVE hooks
  `hobbit_food` and `smeagol_ring` are now applied by the Give command (`input.rs` "give" pending
  command): the happy drunk quaffs Ale/Wine, prints "'Hic!'" and leaves an Empty Bottle on the floor;
  the scruffy hobbit eats any food ("'Yum!'"); Smeagol keeps any ring
  ("'MY... PRECIOUSSSSS!!!'").
- The remaining 00-core `[ ]` entries are stat/energy tables that live in `item.rs`/`skill.rs`/`spell.rs`
  and were outside this agent's file ownership.


## DONE THIS SESSION (final core gaps)

Files: `bevy/src/item.rs`, `bevy/src/data.rs`, `bevy/src/game.rs`.
`cargo test`: 200 passed / 0 failed.

- `z-rand.cc:154-187 randnor`: exact algorithm — `stand < 1` returns **0**,
  a Box-Muller normal sample, the `round_stochastic` helper's `n-1`/`n+1`
  branches including the 0.5 tie roll and the s16b clamp.  `m_bonus` now
  has the original distribution (and `m_bonus(1..3, ...)` is 0-based).
- `dice.cc:17-66 dice_parse/dice_roll`: `data::Dice::parse` accepts the
  full grammar `B+NdM`, `B+dM`, `dM`, `NdM`, `N` (plus the port's
  `NdM+B`), so device charge strings no longer need special cases and
  `Dice::roll` is `base + damroll(num, sides)`.
- `tables.cc:2303 flags_groups()` + `object1.cc:5476-5584`: the 12-realm
  table (`game::FLAGS_GROUPS`) and `gain_flag_group`/`get_flag`/
  `gain_flag_group_flag` are ported and called from the LEVELS-artifact
  level-up path; `pval2`/`pval3` are initialised for LEVELS artifacts
  (see reports/04-objects.md).
- `tables.cc:962 extract_energy`, `tables.cc:1002 player_exp`,
  `adj_dex_ta/adj_str_td/adj_dex_th/adj_str_th`, `adj_wis_sav`,
  `adj_con_mhp`, `adj_mag_mana`, `adj_str_dig`, `adj_dex_safe` and the
  martial-arts blow tables were re-verified as present and consumed
  (`game::PLAYER_EXP`/`exp_needed_at`, `ADJ_CON_MHP`, `mana_base`,
  `skill_dig`, `force_door`'s dex save, `input.rs::py_attack_hand`).
- `xtra1.cc:130 modify_stat_value`: added in the port's folded
  representation (19 = 18/10 ... 40 = 18/220) and used by stat potions
  and Augmentation; the 3..40/18-220 caps now apply.

## FINAL RECONCILIATION

Audited every `[>]` bullet in this report against the current `bevy/src/*.rs`. Flipped to `[x]`:
`r_info E:` body parts (mimic.rs:614-693), `R:E:`/`C:E:` body parts (mimic.rs calc_body),
`F:`/`M:` fixed-map markers (game.rs:6559-6683/10656-10662, map.rs roll_map_markers),
`smart_learn` (game.rs:16058/16227), `auto_scum` (game.rs:18175), `ironman_rooms` (second pass:
`RoomsGen.ironman` forces the unusual/vault rolls and skips `roomdep`), `wear_confirm`
(modal.rs:12353).

Still `[>]` (2):

- `init1.cc:4098-4106` `D:` monster description — `MonsterDef.desc` (data.rs:154-157) still unread; needs the recall/knowledge page in `modal.rs::knowledge_text`.
- `init1.cc:1455-1490` `C:D:1:` class titles — `ClassDef.titles` (data.rs:350-353) has no consumer; needs the character sheet in `hud.rs`/`modal.rs`.

Still `[~]` (unchanged, reason as in the bullet text):

- `init1.cc:5250-5298` `@:S:` save extension — bevy has its own save format (not portable).
- `tables.cc:1990 deity_info`, `3190/3237 tvals`, `3481 between_exits`, `3507 max_body_part`, `3520 gf_names`, `3616 modules` — UI labels / data-dead / module system not ported.
- `carry_query_flag`/`find_*`/`disturb_*`/`preserve`/`autosave_l/t` — defaults match, UI or CreatedArtifacts covers them.
- `files.cc:749` pref-expression `X:/Y:` — no pref/user-file parser in the port.
- `z-form.cc`/`z-util.cc`/`message.cc`/`hiscore.cc`/`loadsave.cc`/level files — Rust equivalents own these subsystems.

No `[ ]` bullets remain in this report.
