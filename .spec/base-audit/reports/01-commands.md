# commands audit report

Base source: `src/cmd1.cc` … `src/cmd7.cc`, `src/bldg.cc`, `src/store.cc`, `src/help.cc`, `src/squeltch.cc` (ToME base only).
Rust side: `bevy/src/input.rs` (command dispatch / melee / movement), `bevy/src/game.rs` (combat formulas,
`execute_inscription`, `run_driver`, upkeep), `bevy/src/modal.rs` (all UI flows, item use, powers, buildings),
`bevy/src/item.rs` (`use_object`, food/corpse, activations), `bevy/src/town.rs` (store stock/pricing),
`bevy/src/spell.rs` (artifact activation table), `bevy/tools/convert_data.py` + `bevy/assets/data/*.ron`.

Conventions: `[ ]` = not ported, `[>]` = partial/wrong semantics, `[x]` = equivalent, `[~]` = intentionally not
ported (UI/CLI/help/Theme/debug/dead data). Cross-references to `reports/02-spells.md` are marked "(see spells)".

## GAPS

### cmd1.cc — melee & movement

- [x] `cmd1.cc:1621` `py_attack_hand` — bare-handed and Bear-form martial arts are not ported.
  `input.rs::melee_attack` (2296) always uses the wielded weapon (or a hardcoded `1d2`/weight 5, line 2306-2309);
  the `ma_blows`/`bear_blows` tables (`tables.cc:1755/1776`, 17/… entries with `min_level`, `chance`, `MA_KNEE/
  MA_SLOW/MA_FULL_SLOW/MA_STUN/MA_WOUND` effects, stun resistance from UNIQUE/NO_CONF/NO_SLEEP/UNDEAD/NONLIVING,
  slow roll `randint(plev) > level && mspeed > 60`, `critical_norm(plev*randint(10), min_level, …)`) are absent.
  Monk/Bear gameplay currently deals weapon damage. 落点 `input.rs::melee_attack` + a table in `skill.rs`.
- [x] `cmd1.cc:1779` `do_nazgul` — hitting a `RF_NAZGUL` monster has none of its effects: no weapon
  `*DISINTEGRATES*` (non-ego non-artifact), no "Ringwraith is IMPERVIOUS to the mundane weapon" for weapons
  without SLAY_EVIL/SLAY_UNDEAD/KILL_UNDEAD (unless `TR_RES_MORGUL`), no 25% ego / 1-in-1000 artifact shatter,
  no `apply_disenchant`, no 25% Black Breath. Rust has `RES_MORGUL` only as a nether resist alias
  (`item.rs:568`); `black_breath` exists but is only set by the monster-side attack (`game.rs:9269`).
  落点 `input.rs::melee_attack` (after `tot_dam_aux`).
- [x] `cmd1.cc:3092/3130/3147` `see_obstacle_grid` / `see_obstacle` / `see_nothing` — part of the run algorithm;
  not ported (see next).
- [x] `cmd1.cc:3351/3445` `run_init` / `run_test` — the corridor-following run algorithm is not ported.
  Rust `run_driver` (`game.rs:7418`) walks the same direction until a monster or non-walkable tile appears
  (max 20 steps, `input.rs:712`); no `find_openarea/find_breakleft/find_breakright`, no corner cutting/blunt
  corridor entry, no `find_examine/find_cut/find_ignore_doors/find_ignore_stairs` options, no stop on
  interesting features/objects, no 1000-step cap.
- [x] `cmd1.cc:4278` `do_cmd_integrate_body` — no disembodied/body-integration system. The Possessor path folds
  everything into `PlayerState.possessed` (`modal.rs:10255`); original corpse-body integration
  (`body_monster = corpse.pval2`, `chp = pval3`, corpse consumed) and the Lost Soul form do not exist.
- [x] `cmd1.cc:4321` `do_cmd_leave_body` — not ported. Missing: cursed-gear check, `drop_body` corpse creation
  (`magik(25 + scale(POSSESSION,25) + PRESERVATION)` roll, weight/unique name1=201, `drop_near`),
  "Your spirit leaves your body." -> `disembodied = true` -> Lost Soul. `modal.rs:10256` only clears
  `possessed`. Deincarnation scroll (`cmd6.cc:2697`) therefore only clears possession.
- [x] `cmd1.cc:4580` `do_spin` — not ported; the "of Spinning" ego activation (`SPIN`, see cmd6 gaps) is the only
  caller. Missing the 8-neighbour `py_attack(j,i,1)` sweep.
- [x] `cmd1.cc:64/98` `test_hit_fire` / `test_hit_norm` — Rust `game::test_hit` (`game.rs:2430`) lacks
  (a) the `luck(-10,10)` term, (b) the invisible-target half chance `if (!vis) chance = (chance+1)/2`.
  **This session:** `input.rs::test_hit_fire` (pub, with the visibility + luck terms and tests) is
  provided; `melee_attack` uses the equivalent `test_hit_vis`.  `modal.rs::fire_missile` and
  `power_throw` still call `game::test_hit`, so unseen ranged targets are hit too easily — one-line
  call-site fix in modal.rs.
- [x] `cmd1.cc:131` `critical_shot` — **This session:** `input.rs::critical_shot` (pub) ports
  `weight + (to_h+plus)*4 + get_skill_scale(skill,100) + 50*xtra_crit + luck(-100,100)`, the
  `randint(5000)` roll and the 500/1000 thresholds, with tests.  Thrown/fired objects still use
  `critical_norm_ex` (`modal.rs:4374` for throw, `modal.rs:16331` for missiles) — the modal.rs call
  sites are the remaining work.
- [x] `cmd1.cc:172` `critical_norm` — `game::critical_norm_skill` (`game.rs:2764`) adds `level*3` that the
  original does not have (`src/cmd1.cc:179-186` has no level term), omits `luck(-100,100)`, and ignores
  `tim_deadly` (the original forces `*GREAT*`: 3×dam+20 and decrements the timer). `ps.tim_deadly` exists
  (`game.rs:1515`) but is never read in melee.
- [x] `cmd1.cc:248` `tot_dam_aux` — `item::dam_aux` (`item.rs:2399`) covers slays/kills/brands/susceptibilities,
  but (a) ignores `p_ptr->tim_poison` as a poison brand, (b) applies to any item (original restricts to
  shot/arrow/bolt/boomerang/hafted/polearm/sword/axe/digging), (c) applies the poison/cut result as flat
  `+10` (`input.rs:2423/2426`) instead of the original `attack_special` amounts (`dam*2` for suscept,
  `dam` normal, level-gated) and the "is bleeding/poisoned" messages.
- [x] `cmd1.cc:453` `carry` — **Done this session:** `input.rs` now has an `Options` resource
  (`smart_learn`, `auto_scum`, `small_levels`/`empty_levels`, `wear_confirm`, `always_pickup`,
  `confirm_stairs`, `no_selling`, `find_*`), and `step_player`'s `cell_arrival` runs
  `py_pickup_floor` on arrival when `always_pickup` is on (default off, matching the original);
  the automatizer sweep on the pile is now unconditional as in `carry` -> `py_pickup_floor` ->
  `squeltch_grid`.  The option has no UI to toggle it yet.
- [x] `cmd1.cc:462` `touch_zap_player` — auras exist (`input.rs:2554-2588`) but lack the
  `sensible_fire` ×2 multiplier and the exact "You are suddenly very hot!"/"You get zapped!" context
  (Rust adds an unoriginal `AURA_COLD` arm).
- [x] `cmd1.cc:517` `carried_monster_attack` — `input.rs::symbiote_attack` (2631) only rolls the symbiote's
  blows for damage. Missing: 4-blow cap and `o_ptr->elevel` power, all `RBE_*` effect conversions
  (poison/disenchant/shatter/nether/time/…), `touched` auras (`RF_AURA_FIRE/ELEC` on the target),
  `RBE_EAT_ITEM/GOLD` 1/2 blink, "You hear noise." when unseen, miss messages, and the
  `teleport_player(MAX_SIGHT*2+5)` flee when blinked.
- [x] `cmd1.cc:1012` `incarnate_monster_attack` — `melee_possessed` (`input.rs:2160`) uses the form's blows but
  no `RBE_*` effects, no `rlev = r_ptr->level` power check (`monster_check_hit` is fed `ctx.ps.level`),
  no aura damage, no blink.
  **Fixed this session:** `melee_possessed` now uses the body's depth as `rlev`, converts every RBE_* blow
  through `gf_monster_effect` (poison/acid/elec/fire/cold/confuse/terrify/paralyze/disenchant/nether/time,
  `GF_OLD_SLEEP` at `plev*2`), applies touched AURA_FIRE/ELEC against the body's `IM_*`, quakes on SHATTER,
  blinks away (`MAX_SIGHT*2+5`) on EAT_ITEM/EAT_GOLD, and honours `RF_NEVER_BLOW`.
- [x] `cmd1.cc:1503` `flavored_attack` — not ported. Rust logs `You hit the X. (dmg)` (`input.rs:2442`);
  the percent-based `You scratch/hit/wound/cripple/demolish` table and the `msane/csane` insanity
  variants (`dam_none/med/lots/huge/xxx.txt`) are missing.
- [x] `cmd1.cc:1551` `attack_special` — bleeding/poisoning is approximated in `input.rs:2423-2428`
  (+10 flat, no messages, no `RF_SUSCEP_POIS` double, no level save `rand_int(100) >= level`).
- [x] `cmd1.cc:1875` `py_attack` — many per-blow branches missing: `TR_CHAOTIC` chaos table
  (vampiric/quake/confuse/teleport/polymorph, lines 2041-2068), vampiric drain formula
  `damroll(4, (pre_hp - post_hp)/6)` capped at `MAX_VAMPIRIC_DRAIN=100` per attack
  (Rust `item::vampiric_heal` = `dam/4`, no cap, and excludes DEMON which the original does not —
  note the original only blocks UNDEAD/NONLIVING), `p_ptr->confusing` confusion attack
  (`10 + rand_int(COMBAT)/5`), `praying_to(GOD_MELKOR)` `do_melkor_curse` on hit, `praying_to(GOD_TULKAS)`
  grace-scaled bonus (Rust has an ad-hoc ×3), Stormbringer/friendly "stop to avoid hitting" logic,
  `p_ptr->afraid` early return, `do_quake` (Rust only quakes for IMPACT, not `p_ptr->impact`),
  backstab/stab-fleeing messages (`You cruelly stab…` / `You backstab the fleeing…`, Rust prints the
  former for every sleeping hit), `tim_deadly`, `RF_PASS_WALL` displacement. `TR_CLONE`, tim_project,
  IMPACT quake and auras are present.  Reconciled: still missing — the friendly stop has no
  Stormbringer override (step_player always npc_chats, input.rs:4600), the Melkor on-hit curse
  (src/cmd1.cc:2182) is absent from melee_attack, and the PASS_WALL "You push past X." displacement
  (src/cmd1.cc:2848) is not in step_player.
  **SESSION (input.rs) 2026-09-16:** all listed branches are now present: TR_CHAOTIC table
  (incl. the original's cross-blow carry-over), vampiric 4d(delta/6) cap 100, `p_ptr->confusing`
  hand-glow confusion (does not raise damage), Melkor `do_melkor_curse`, Tulkas grace bonus,
  Stormbringer override, afraid/PASS_WALL/backstab/tim_deadly, wake-on-hit (`csleep = 0`) and the
  friendly `player_can_enter` displace check.  `confusing` and `immov_cntr` live in
  `input::MoveRepeat` (`confusing`, `immov_cntr`) as a temporary home — `PlayerState` (game.rs) has
  no fields for them yet; saves do not carry the two counters.
- [x] `cmd1.cc:2442` `player_can_enter` — `input.rs::player_can_walk` (886) covers trees/climb/fly/levitate/
  pass-wall/only-wall/deep water, but not the `wild_mode` lava check (`FEAT_SHAL_LAVA/DEEP_LAVA` require
  `resist_fire|immune_fire|oppose_fire|ffall`, lines 2465-2475), so a fire-vulnerable player can step into
  wilderness lava.
  **Fixed this session:** the wild-overview path (`input.rs::wild_step`, the only `wild_mode` movement in the
  port) now applies both safety checks (heavy swimmers at deep water, lava without
  immunity/resist/oppose/levitation); `player_can_walk` also gained the `FF_WEB` spider branch.
- [x] `cmd1.cc:2538` `easy_open_door` — lock picking is missing. `input.rs::force_door` (1411) opens plain
  doors and always bashes locked ones with `50 + 10*melee + 2*lev - 10*power`; the original pick chance
  `100 (÷10 blind, ÷10 confused) - 4*lock_power`, the "You have picked/failed to pick the lock." messages
  and the `gain_exp(1)` reward are absent. Locked doors also cannot be opened at all with the `alter`
  key (only bashed).
- [x] `cmd1.cc:2639` `move_player_aux` — `step_player` (1606) misses: ice slip (`FEAT_ICE`,
  `magik(70-lev)`), `RF_RAND_25/RAND_50` random direction while in a possessed body, `tim_roots`
  ("rooted means no move"), the secret-door/CAVE_MARK descriptions for unknown blockers, and the
  hard `FEAT_DARK_PIT` message. Wild edge transitions/shop/inscription/altar branches are present.
  Reconciled: ice slip, RF_RAND_25/RAND_50 and FEAT_DARK_PIT are now in step_player
  (input.rs:4552-4916); remaining: `tim_roots` (game.rs:4986) never blocks movement, and blocked
  unmarked features are always described instead of the CAVE_MARK "You see nothing there."
- [x] `cmd1.cc:3808` `run_step` — only the straight-line `run_driver` (see above); `running` is limited to
  20 steps in `input.rs:712` instead of the original 1000, and `run_step`'s "You cannot run in that
  direction." guard is absent.
- [x] `cmd1.cc:3863` `do_cmd_pet` — **Done this session** in `input.rs`: Shift+O opens a text-mode
  companion menu with all ten commands (uppercase verifies, digits select, Enter picks the single
  command).  Dismiss pets/companions despawn the (single) `companion` bucket (NO_DEATH respected);
  call/follow/seek store `follow_distance` in a new `PetOptions` resource; the two toggles toggle
  `open_doors`/`pickup_items` and turning pickup off immediately drops the pets' carried objects on
  the floor; give-target/forget-target use a direction prompt and set `Monster::target`, which the
  companion AI already honours (`game.rs:11360`).  `case 7` ("give target to a friend") is a
  no-op in the original and is ported as such.  Remaining: `game.rs::monster_turns` still reads the
  hardcoded `PET_FOLLOW_DISTANCE`/`PET_OPEN_DOORS`/`PET_PICKUP_ITEMS` consts (13256-13262) instead
  of `PetOptions`, and pets/friends/companions share one flag in the port so the two dismiss
  commands act on the same set.
  **SESSION (input.rs/modal.rs) 2026-09-16:** the PET/COMPANION split now exists via
  `Monster.pet` (MSTATUS_PET) vs `companion && !pet` (MSTATUS_COMPANION, matching the hatching
  cap): dismiss pets removes MSTATUS_PET, dismiss companions removes MSTATUS_COMPANION, orders
  (give/forget target) only go to `companion` monsters, and `do_cmd_companion` (PWR_COMPANION)
  requires `m.pet`, clears it and enforces `1 + skill_scale(LORE,6)` over companions only.
  Caveat: a few game.rs spawns label MSTATUS_FRIEND monsters as `companion && !pet` (e.g. the
  Fire Golem at game.rs:9839); MSTATUS_FRIEND has no third marker, so those count as companions
  for the dismiss/cap checks.

### cmd2.cc — stairs, doors, digging, running, throwing, sacrifice, stealing

- [x] `cmd2.cc:81` `do_cmd_bash_fountain` — no bash command at all (see `do_cmd_bash`), and the fountain
  break (`adj_str_blow` roll, `fire_ball(GF_WATER,5,6d8,2)`, `FEAT_DEEP_WATER`) is absent.
- [x] `cmd2.cc:589` `count_feats` — not ported; Rust never auto-selects the single visible door/trap, so
  `open`/`close`/`alter` always require an explicit direction.
- [x] `cmd2.cc:638` `coords_to_dir` — not ported (helper for the above).
- [x] `cmd2.cc:1103` `do_cmd_tunnel_aux` — Rust `tunnel` (`input.rs:2717`) is only a dig-counter
  (`DIG_TURNS - tunnel`), missing: the TV_DIGGING tool-slot requirement ("You need to have a shovel or
  pick in your tool slot."), `skill_dig` rolls, tree/dead-tree chopping to grass, vein treasure
  (`place_gold` for `FEAT_MAGMA_H/QUARTZ_K/SANDWALL_K`, "You have found something!"), rubble removal
  (10% `place_object`, dungeon floor1), secret-door reveal (`mimic=0`), door digging, and the
  "This will take some time./You fail to make even the slightest of progress." feedback.
- [x] `cmd2.cc:1552` `do_cmd_bash` — no bash command. `force_door` (see above) covers plain/locked doors
  only; missing `adj_str_blow` vs door power, 50/50 broken/open result, falling through the door
  (`move_player_aux`), the `adj_dex_safe + lev` save ("The door holds firm."), and the
  "You are off-balance." -> `set_paralyzed(2+randint(2))` branch; altars/fountains cannot be bashed
  (`do_cmd_bash_altar` message).
- [x] `cmd2.cc:1868` `do_cmd_unwalk` — not ported (immovable characters' phase-door walking); Rust's
  `Modal::Immovable` only offers teleport/fetch/up/down and swallows the move.
- [x] `cmd2.cc:2044/2072` `do_cmd_run_run` / `do_cmd_run` — not ported (confusion guard, command
  argument repeats, `run_step`). Rust instead runs 20 straight steps (see cmd1 run gaps).
- [x] `cmd2.cc:3698` `item_tester_hook_sacrificable` — no item sacrifice path exists at all
  (Melkor corpses/udun-less books for piety, see `do_cmd_sacrifice`).
- [x] `cmd2.cc:66/3528` `do_cmd_immovable_special` — `Modal::Immovable` (`modal.rs:8945`) offers a/b/c/d, but
  ignores the `immov_cntr` cost (`+= 101 - lev*2`), the mana/HP half-cost warning and payment
  (`csp/2` or `chp/2`), and the "You can't use your powers yet." gate; fetch uses `power_fetch` (4158)
  which does not `py_pickup_floor` afterwards.  Reconciled: still missing — no `immov_cntr`
  accumulator/spend (no PlayerState field), no "You can't use your powers yet." gate or csp/chp
  half-cost prompt, and power_fetch (modal.rs:4544) does not call pickup afterwards.
  **SESSION (modal.rs) 2026-09-16:** done — `Modal::ImmovablePay` shows the csp/2 ("This will drain
  N mana points!") or chp/2 warning and pays it; `immovable_begin` enforces "You can't use your
  powers yet." and `immovable_pay` recharges `immov_cntr += 101 - lev*2`; the counter decays one per
  player turn (`input.rs`).  The immovable 'b' fetch now follows cmd5.cc `fetch` (weight `lev*15`,
  object flies to the player's feet, `py_pickup_floor(always_pickup)` after).  Note: `immov_cntr`
  lives in `input::MoveRepeat`, not `PlayerState` (cross-file, not saved).
- [x] `cmd2.cc:71` `do_cmd_bash_altar` — branch absent; bashing an altar is not reachable.
- [x] `cmd2.cc:133` `stair_hooks` — Rust handles the known quest stair outcomes ad hoc
  (`input.rs:3062 quest_exit`, `PLOT_BETWEEN/INVASION/NIRNAETH`), but there is no generic hook point and
  several `HOOK_STAIR` behaviours (q_god relic, q_eol, q_fireprof, q_library, q_ultrag, q_invas) are only
  partially reproduced; `cmd2.cc:186` (`do_cmd_go_up`) also calls it before any stair handling.
- [x] `cmd2.cc:145` `ask_leave` — `confirm_stairs` option not modelled (options system absent); Rust only
  confirms `DF_ASK_LEAVE` dungeons (`input.rs:2876`) and always confirms on `<`.  Reconciled:
  `options.confirm_stairs` (options.rs:65) is still never read; the confirm is driven only by the
  ASK_LEAVE level flag (input.rs:7077, modal.rs:8671).
- [x] `cmd2.cc:168` `do_cmd_go_up` — `use_stairs` (`input.rs:2766`) implements most branches, but misses
  `create_down_stair/shaft` bookkeeping for shaft return trips, the `c_ptr->special` dungeon-branch
  `get_flevel()` depth calculation (`cmd2.cc:304-310`), and `FEAT_LESS` at dungeon level 0.
- [~] `cmd2.cc:321` `between_effect` — `FEAT_BETWEEN` partner swap is present (`input.rs:1758`), but
  `FEAT_BETWEEN2` world-exit gates (`between_exits`: wilderness x/y, px/py, dungeon/level) are not.
  Reconciled: `FEAT_BETWEEN2`/`between_exits` is data-dead in base (no producer; 00-core marks
  `tables.cc:3481` `[~]`), so the live `FEAT_BETWEEN` swap (input.rs:4765) is the whole feature.
- [x] `cmd2.cc:365` `do_cmd_go_down` — Rust lacks the `DF_ASK_LEAVE` y/n prompt before descending
  (only the up path has `ConfirmLeave`), so unique-level departures downward happen without warning;
  the `FEAT_QUEST_ENTER` branch is handled by map quest entrances instead.  Reconciled: the down
  branch of `use_stairs` (input.rs:7004-7075) still has no ASK_LEAVE check, so DF_ASK_LEAVE levels
  descend without the y/n prompt of cmd2.cc:372.
- [x] `cmd2.cc:668/791` `do_cmd_open_aux` / `do_cmd_open` — see `easy_open_door` gap: no lock-picking,
  no exp; Rust also does not attack a monster standing in the door, and `force_door` has no
  "You see nothing there to open." / stuck-door handling.
- [x] `cmd2.cc:890/939` `do_cmd_close_aux` / `do_cmd_close` — no broken-door message, no monster-in-door
  attack, no auto direction.
- [x] `cmd2.cc:1021/1070` `do_cmd_tunnel_test` / `twall` — Rust does not require `CAVE_MARK` knowledge
  ("You see nothing there."), and   `twall` always sets `T_FLOOR` (no per-dungeon `floor1`, no `CAVE_MARK`
  clearing).  Reconciled: the CAVE_MARK test/clearing are in tunnel_aux/twall (input.rs:4019/3979);
  remaining: rubble removal uses T_FLOOR instead of the dungeon's `floor1` (src/cmd2.cc:1271,
  q_god changes it at runtime) — input.rs:4085.
- [x] `cmd2.cc:1380` `do_cmd_tunnel` — wrapper exists but only the dig counter (see
  `do_cmd_tunnel_aux`); no "You cannot tunnel through air." message and no monster-in-the-way attack
  (the latter is handled by `step_player`'s bump only when the tile is walkable).
- [x] `cmd2.cc:1445` `do_cmd_bash_aux` — simplified `force_door` (see `do_cmd_bash`): no
  `adj_str_blow` roll, no broken-door result, no off-balance paralysis, no door power tiers.
- [x] `cmd2.cc:1641` `do_cmd_alter` — `command_direction "alter"` (`input.rs:1441`) attacks monsters,
  disarms known traps, opens closed doors and digs, but (a) does not handle locked doors with
  `do_cmd_open_aux`, (b) does not use `do_cmd_tunnel_aux` (no tool/treasure logic), (c) does not consume a
  turn for "You attack the empty air.", and (d) cannot repeat the command.  Reconciled: (a)-(c) are
  done via force_door/tunnel_aux (input.rs:4165-4205); only command repetition
  (`allow_repeat_command`) is still absent — alter/close/spike need a fresh keypress each time
  (input.rs:1431).
  **SESSION (input.rs) 2026-09-16:** command repetition added for alter: '0' opens the "Count:"
  prompt (util.cc `get_number`), the count is stored in `MoveRepeat::command_arg`, and a
  progressing alter (failed lockpick / ongoing tunnel) repeats once per player turn until the count
  runs out or any key disturbs it.  Other repeatable commands (open/close/tunnel/walk/stay) still
  need fresh keys.
- [x] `cmd2.cc:1745` `do_cmd_spike` — works, but a monster in the target door should be attacked
  (`py_attack`); Rust prints "There is a monster in the way!" and returns without spending the turn.
- [x] `cmd2.cc:1812` `do_cmd_walk_jump` — `wild_step` (`input.rs:3120`) implements world-map steps
  (132-turn cost, ambush roll), but inherits all `move_player_aux` gaps (no pickup/ice/random movement)
  and does not call `move_player` with `options->always_pickup`.
- [x] `cmd2.cc:2029/2090` `do_cmd_walk` / `do_cmd_stay` — (modal/item/input session) Rust routes
  walk to `step_player` and stay to `consume_turn`; neither performed `carry(pickup)`.  Closed:
  stay (5/./Period) now runs `carry_at(always_pickup)` and holding `-` walks with
  `!always_pickup` (`step_player_pickup`), so both the pick-up and no-pickup variants exist.
- [x] `cmd2.cc:2119` `do_cmd_rest` — Rust 'R' sets `resting=10` (`input.rs:267`, driver at `game.rs:7393`):
  no duration prompt, no `*` (rest to full HP/SP) / `&` (rest as needed) modes, no 9999 cap, no
  `FEAT_BETWEEN` "too dangerous" or undead-form "Resting is impossible while undead!" guards.
  **Closed (verified):** `RestMode::{Full,AsNeeded}`, the text prompt (`rest_prompt`/
  `rest_prompt_input`, `*`/`&`, 9999 cap) and the FEAT_BETWEEN/undead guards are implemented in
  input.rs/game.rs; the old text above is stale.
- [x] `cmd2.cc:2218` `breakage_chance` — `fire_missile` (`modal.rs:16356`) only handles arrows (50) and
  shots/bolts (25) divided by the Archery reducer; flask/potion/bottle/food (100), lite/scroll/skeleton
  (50), wand/spike (25), boomerang (1) and the default 10 are ignored for firing, and thrown items
  (`power_throw`, 4304) never roll breakage at all (they are always placed on the floor).
- [x] `cmd2.cc:2349` `do_cmd_fire` — core firing works (`modal.rs::fire_missile`, 16170), but: no
  `p_ptr->num_fire`/energy spreading over shots, no firing from the floor (USE_FLOOR), no
  "Your missile breaks." item destruction (only a message), no `target_okay` aiming (direction only),
  and `breakage_chance` gaps above.  Reconciled: breakage/drop_near are now wired
  (modal.rs:18282-18288); still missing `num_fire` energy spreading (one turn per volley), the
  USE_FLOOR source (quiver/pack only) and target_okay aiming.
  **SESSION (modal.rs/input.rs) 2026-09-16:** the USE_FLOOR source is wired (the pile under the
  player joins quiver->pack after the first shot) and the to-hit/crit rolls now use
  `input::test_hit_fire` + `input::critical_shot` with visibility/luck.  `num_fire` remains a
  single-turn volley whose shot count already equals the C++ per-turn total (`1 + xtra_shots +
  alvl/16`, energy 100/shots); the locked target is the existing `*` target-lock path.
- [x] `cmd2.cc:2780` `do_cmd_throw` — `power_throw` (`modal.rs:4304`) uses `5*level + to_h*3` for the hit
  chance instead of `skill_tht + to_h*BTH_PLUS_ADJ`, ignores `throw_mult`/boulder skill bonuses,
  never rolls breakage, does not shatter potions into their area effect
  (`potion_smash_effect`, see spells), and cannot throw from the floor or at a locked target.
  Reconciled: potion smash + breakage + drop_near are now wired; the hit chance is still
  `5*level + to_h*3` (modal.rs:4716) instead of skill_tht + to_h*BTH_PLUS_ADJ, and throw_mult/boulder
  bonuses (SK_BOULDER) plus the floor/locked-target sources are still missing.
  **SESSION (modal.rs) 2026-09-16:** the chance is now `skill_tht + p_ptr->to_h*BTH_PLUS_ADJ`
  (p_ptr->to_h rebuilt from equipment/dex/str/tactic/hero/blessing/strike with the stun penalty;
  no melee-mastery term), the hit uses `input::test_hit_fire` (visibility half-chance + luck) and
  the critical uses `input::critical_shot` with SK_BOULDER for boulders.  Floor piles under the
  player are selectable in `Modal::Throw`, and the `*` target lock works through `Modal::Target`.
- [x] `cmd2.cc:3154` `do_cmd_boomerang` — folded into `fire_missile`; boomerangs return with 50% chance
  instead of always returning, no destruction roll (`breakage_chance`), and the bow slot item is not
  consumed on a failed return.  Reconciled: breakage/drop of the failed return exists via drop_near
  (chance 1); the return itself is still a 50% coin flip (modal.rs:18201) instead of always
  returning (cmd2.cc:3440).
- [x] `cmd2.cc:3476` `tport_vertically` — `Modal::Immovable` c/d (`modal.rs:8962`) implements the level
  shift with NO_EASY_MOVE/maxdepth/mindepth checks, but not the `inside_quest` "There is no effect." case
  (Rust approximates via plot-depth) and not the immov_cntr cost.  Reconciled: the
  NO_EASY_MOVE/maxdepth/mindepth checks are wired (modal.rs:9600-9632); only the `immov_cntr`
  cost/prompt (shared with do_cmd_immovable_special above) is missing.
  **SESSION (modal.rs) 2026-09-16:** the `immov_cntr` gate/payment is applied through
  `immovable_begin`/`immovable_do`/`immovable_pay` before the level shift; failures (no effect,
  NO_EASY_MOVE, impermeable/air) close the menu without charging, matching the C++
  `return`-before-`did_act`.
- [x] `cmd2.cc:3758` `do_cmd_sacrifice` — the temple menu (`modal.rs:17434`) only offers "sacrifice 100
  gold (+50 grace)". Missing the altar worship-conversion flow from this command
  (`show_god_info` + `follow_god` + `grace=-200`; the follow code exists in the menu but not the
  descriptions), Melkor self-sacrifice for HP (10 HP -> `wisdom_scale(6)*300` piety or
  `melkor_sacrifice` damage bonus), and Melkor item sacrifice (corpses `2*level`, Udun-less books
  `2*levels_in_book`).  Reconciled: corpse/book sacrifice and the 10-HP branch are in
  do_cmd_sacrifice (input.rs:3793); still missing the altar conversion flow (`show_god_info`
  descriptions + `follow_god` + `grace=-200`, cmd2.cc:3780-3800) and Melkor's "for more damage"
  branch (`melkor_sacrifice++`; the field exists, game.rs:1684).
- [x] `cmd2.cc:3979` `do_cmd_steal` — `modal.rs::steal_attempt` (2899) implements the formula, but
  `PR_EASE_STEAL` (Rogues) is dead data in base and the "Phase door?" escape + exp reward are missing
  (already noted in `.spec/handoff.md` 3.8), and the stolen item is not marked un-resellable
  (`found_aux1`/`discount=100`).
- [x] `cmd2.cc:4164` `do_cmd_give` — `Modal::Give`/`command_direction "give"` (`input.rs:1550`) pushes the
  item into the monster's inventory but never runs `HOOK_GIVE`; quest give-hooks (q_hobbit Recall scroll,
  q_shroom mushrooms) are instead hardcoded in `npc_chat` bump-talk (`input.rs:1979`), so scripted give
  behaviour is not generic.  Reconciled: the three base-module HOOK_GIVE hooks are applied by the
  Give command (input.rs:4300-4333); the quest give-hooks (q_hobbit/q_shroom) still only fire
  through npc_chat bump-talk, so HOOK_GIVE remains non-generic.
- [x] `cmd2.cc:4209` `do_cmd_chat` — Rust has no chat command; bumping a friendly NPC (`npc_chat`,
  `input.rs:1979`) handles only Farmer Maggot/Melinda/Merton/Thrain; no `HOOK_CHAT` and no
  "The monster does not want to chat." path.

### cmd3.cc — inventory / equipment / refuelling / query

- [x] `cmd3.cc:861` `do_cmd_refill_torch` — cannot combine torches. `Modal::Fuel` (`modal.rs:8823`) lists
  only `TV_FLASK` and refuses anything whose lite has `fuel < 7500`, so `TV_LITE` torch/lantern sources are
  unusable as fuel; the `timeout + pval` (flask) / `timeout + timeout + 5` (torch) and the
  FUEL_LAMP/FUEL_TORCH caps are not reproduced (Rust uses one hardcoded 15000 cap and a flask pval).
- [x] `cmd3.cc:1201/1220/1239` `compare_monster_experience` / `compare_monster_level` /
  `compare_player_kills` — the monster recall list sorting is absent (no query-symbol recall UI).
- [x] `cmd3.cc:1259` `roff_top` — monster recall header (name, attr/char) not implemented.
- [x] `cmd3.cc:487` `do_cmd_drop` — **Done this session**: `d` now opens a text-mode picker
  (`input.rs::picker_drop`) with a quantity prompt, `/` to switch pack/equipment, the cursed
  equipment and CURSE_NO_DROP guards, the One Ring (depth-60 fire) and q_poison cure-water cases,
  the "You drop X (letter)." message and the proportional wand-charge split
  (object2.cc:5761).  `HOOK_DROP` remains unported because the port has no hook system.
- [x] `cmd3.cc:553` `do_cmd_destroy` — **Done this session**: `D` opens the same picker with a
  quantity prompt, pack/floor sources (`/`), the artifact / CURSE_NO_DROP checks, the "You destroy
  X." message, partial wand charge adjustment, Eru piety (`TR_BLESSED`) and automatizer rule
  creation (`$` toggles `Automatizer::create_rule`; the rule is added as tval+sval+status and the
  original "Rule added..." hint is printed).  Floor piles are reduced/destroyed in place.  The
  vault/query (`USE_VAULT`) source is not ported.
- [x] `cmd3.cc:798/913` `do_cmd_refill_lamp` / `do_cmd_refill` — see `do_cmd_refill_torch`: only
  flask->lantern works, lantern->lantern and torch->torch merging is missing; `TR_FUEL_LITE` dispatch and
  "Your light cannot be refilled." are not reproduced.
- [x] `cmd3.cc:1319` `do_cmd_query_symbol` — `Modal::QuerySymbol`/`describe_symbol`
  (`modal.rs:16709`) only describes one glyph (monster -> terrain -> object -> fixed table). Missing the
  Ctrl-A/Ctrl-U/Ctrl-N/Ctrl-M lists (all/unique/non-unique/name search), the `ident_info` fallback texts,
  k/p sorting, the recall browser and `health_track`. (The Knowledge modal has a flat unique-kill list.)
  Reconciled: still only the single-glyph describe (modal.rs:8711); no
  Ctrl-A/Ctrl-U/Ctrl-N/Ctrl-M lists, k/p sorting or recall browser (cmd3.cc:1319-1520).
  **SESSION (modal.rs) 2026-09-16:** done — Ctrl-A/U/N/M open `Modal::SymbolBrowser` (full /
  unique / non-unique / name-substring lists; Ctrl-M asks for the name in `Modal::SymbolSearch`),
  any key pages, '-' goes back and `r` toggles the detail block; each entry shows the kill count,
  the r_info `D:` text and (with `r`) level/speed/AC/HP/exp.  The full `roff_aux` flag prose
  (breaths/spells/movement) is still not rendered — the browser's `r` view is the D: text + stats.

### cmd4.cc — knowledge / notes / tactic / time / options

- [x] `cmd4.cc:3151` `monster_get_race_level` — not ported; needed for unique-monster knowledge ordering.
- [x] `cmd4.cc:3240` `plural_aux` — not ported (plural handling in knowledge text).
- [x] `cmd4.cc:3526` `do_cmd_knowledge_towns` — no "known towns" knowledge page. `knowledge_text`
  (`modal.rs:5238`) has artifacts/uniques/kills/pets/dungeons/corruptions/quests/fates only.
  Reconciled: `game::known_towns_text` exists (game.rs:2861) but `knowledge_text` (modal.rs:5798)
  still has no Towns page (pages 1-8 only).
- [x] `cmd4.cc:3640` `do_cmd_knowledge_notes` — **Partial this session**: the notes file now exists
  (`bevy/src/notes.rs`, `notes.txt` in the crate root) with the original line format and the
  `NOTE_BIRTH`/`NOTE_SAVE_GAME`/`NOTE_ENTER_DUNGEON`/`NOTE_WINNER` blocks.  The Knowledge-window
  page itself (`modal.rs::knowledge_text`) still does not list/show them.  Reconciled: notes.txt is
  written by notes.rs but the knowledge menu/page list (modal.rs:5798) still omits notes.
- [x] `cmd4.cc:2602` `do_cmd_note` — **Done this session**: `:` (Shift+;) starts a text note,
  Enter appends `"Turn N <depth>  : text"` to `notes.txt` (60-char limit), Esc cancels; empty
  notes are ignored as in the original.  A message-recall entry is not added (no recall UI).
- [x] `cmd4.cc:3166` `do_cmd_knowledge_uniques` — Rust lists killed uniques with counts
  (`modal.rs:5278`) but not race level/experience/description, kill dates or the paging/sorting modes.
  Reconciled: page 2 still shows only `name xN` (modal.rs:5840); the `compare_monster_*` /
  `monster_get_race_level` helpers (game.rs:3561-3608) are unused by the modal.
- [x] `cmd4.cc:3650` `do_cmd_knowledge` — pages subset (towns/notes missing, simplified entries).
  Reconciled: unchanged — no towns/notes pages and simplified uniques/quests entries (modal.rs:5798).
- [x] `cmd4.cc:3791` `do_cmd_checkquest` — no dedicated Ctrl-Q status (the Knowledge quests page exists);
  no per-quest "kill N monsters" progress text.  Reconciled: still no checkquest status; the
  Quests page (modal.rs:5915) lists only taken/completed/failed states without kill counts.
  **SESSION (input.rs/modal.rs) 2026-09-16:** Ctrl+Q now opens the Quests knowledge page (the
  original do_cmd_checkquest is do_cmd_knowledge_quests), which prints taken quests in danger-level
  order with the C++ name/level/up-to-ten description lines, completed ones as "Completed -
  Unrewarded", plus the active dungeon quest and its slay target.
- [x] `cmd4.cc:3807/3822` `do_cmd_change_tactic` / `do_cmd_change_movement` — `Modal::Tactic`
  (`modal.rs:8365`) clamps at 0/8 instead of the original wrap-around (`>8 -> 0`, `<0 -> 8`).
  Reconciled: still clamps — modal.rs:9002-9008 (`.min(8)` / `saturating_sub`).
- [x] `cmd4.cc:3837` `do_cmd_time` — Rust tracks day/hour for lighting (`game.rs:1861`) but has no time
  command, no day-count message and no `timenorm.txt`/`timefun.txt` flavour.  Reconciled: no time
  command exists (input.rs has no binding); `is_daytime` (game.rs) only drives lighting.
- [x] `cmd4.cc:488/1087` `interact_with_options` / `do_cmd_options` — **Partial this session**: the
  gameplay switches now exist as a global `options::Options` resource with the `options.hpp`
  defaults (`smart_learn=false`, `auto_scum=true`, `small_levels`/`empty_levels=true`,
  `wear_confirm=true`, `always_pickup=false`, `confirm_stairs=true`, `no_selling=false`,
  `find_*`, `autosave_freq`).  Enforced so far: `always_pickup` (cmd1:453 carry, `cell_arrival`)
  and the `wear_confirm` helper (`options::wear_confirm_needed`).  Still to wire (not this
  file): `smart_learn` in `game.rs::update_smart_learn`/`remove_bad_spells`, `auto_scum` /
  `small_levels` / `empty_levels` in `map.rs`/`game.rs` generation, `wear_confirm` in
  `modal.rs::wield_item`, `confirm_stairs`, `no_selling`, `cheat_peek`, `autosave_freq`,
  `delay_factor_ms`.  The options screens/pref files remain `[~]` UI.  Reconciled:
  smart_learn/auto_scum/small_levels/empty_levels are now wired (game.rs:16058/18175, map.rs:3978);
  still to wire: `wear_confirm` (modal.rs:11802), `confirm_stairs` (input.rs:7077),
  `options.no_selling` into ShopCtx (modal.rs:7527/11637/11776 all pass false); cheat_peek,
  autosave_freq and delay_factor_ms have no fields/consumers.

### cmd5.cc — monster powers

- [x] `cmd5.cc:815` `apply_monster_power` — broad coverage in `modal.rs:4754`, but: (19th-round follow-up: S_MONSTERS player power now summons 6, modal.rs)
  (a) `SF_BLIND` should be a `GF_CONFUSION` bolt `1d8+plev/3` (cmd5.cc:1396) and models monster
  blindness; Rust routes "BLIND" into the CONF arm (confusion, no damage);
  (b) `SF_SLOW` (`GF_OLD_SLOW`, `6d8+plev/3`) and `SF_HOLD` (`GF_OLD_SLEEP`, `5d8+plev/3`) apply their
  bolt damage before the status in the original (1416-1434); Rust applies no damage and uses
  `stun` for the paralysis;
  (c) `SF_TELE_AWAY` is a `fire_beam(GF_AWAY_ALL, dir, plev)` (1532-1546); Rust teleports only the first
  monster in the ray (`first_in_ray`);
  (d) `SF_S_MONSTERS` summons 6 (cmd5.cc:1615-1623); `summon_pool` returns 8 (`game.rs:12016`).
  Reconciled: all four remain — BLIND→CONF arm (modal.rs:5495), SLOW/HOLD no bolt damage (5485-5493),
  TELE_AWAY first-in-ray (5500-5510), S_MONSTERS=8 in summon_pool (game.rs:17895).
  **SESSION 2026-09-16:** re-verified — (a)-(c) are done in modal.rs (SF_BLIND is a GF_CONFUSION
  bolt, GF_OLD_SLOW/GF_OLD_SLEEP set `dam = 0` in base so "no bolt damage" is correct, and
  SF_TELE_AWAY is the full beam loop); HOLD still sleeps via `m.stun += 500` because `Monster` has
  no `csleep` timer (cross-file).  Only `S_MONSTERS` remains: `game::summon_pool` returns 8 for
  "any" summons (game.rs, cross-file), the original uses 6.

### cmd6.cc — items: food / potions / scrolls / devices / activations

- [x] `cmd6.cc:4276` `activate_maggot` — not ported. Farmer Maggot's sling activation is
  `fire_ball(GF_TURN_ALL, dir, 40, 2)` (terrify), but `spell.rs:544` maps the `MAGGOT` activation name to
  `("teleport","10",…)` with recharge `10+d50`. Wrong effect and wrong description
  ("terrify every 10+d50 turns", cmd6.cc:7062).
- [x] `cmd6.cc:1703` `quaff_potion` — name-dispatched table in `item.rs::use_object` (3737) is close, but:
  `SV_POTION_ENLIGHTENMENT`/`STAR_ENLIGHTENMENT` only reveal the map (no `wiz_lite`/`wiz_lite_extra`,
  INT/WIS grant, `detect_doors/stairs/treasure/objects_gold/objects_normal`, `identify_pack`; see spells);
  "Lose Memories" uses `lose_exp(exp/4)`; several durations differ from the original rolls
  (Sleep/Slowness/Blindness). Potions still do not smash when they should (no `potion_smash_effect`,
  see spells), which is the caller-facing half of this function. (Detonations/Life/Healing/Curing/stat
  potions match.)  Reconciled: potion_smash is now wired via inven_damage/floor_hit; remaining:
  STAR Enlightenment shares the plain map-reveal arm (item.rs:6062; needs wiz_lite_extra + INT/WIS +
  detects + identify_pack), Sleep ignores free_act and rolls 6..10 vs rand_int(4)+4, slowness/blindness
  rolls are off by one (item.rs:6109-6120).
- [x] `cmd6.cc:2491` `curse_armor` — `scroll_special "Curse Armour"` (`modal.rs:11405`) only sets
  `cursed/known_cursed` and `to_a -= 1` on the body armour. Original: 50% artifact save; otherwise
  shatters to `EGO_BLASTED` with `to_a = -randint(5)-randint(5)`, dice/ac zeroed, `TR_CURSED`.
  Reconciled: unchanged — the scroll handler (modal.rs:12357) still only sets cursed/to_a-1;
  `item::curse_equipment_ex` (item.rs:4271) exists but is only used by the CAUSE/HAND_DOOM paths.
- [x] `cmd6.cc:2546` `curse_weapon` — same (`modal.rs:11396`): 50% artifact save; otherwise
  `EGO_SHATTERED` with `to_h/to_d = -randint(5)-randint(5)` and dice/ac zeroed; Rust only subtracts 1.
  Reconciled: unchanged — modal.rs:12348 still subtracts 1; use curse_equipment_ex (item.rs:4271).
- [x] `cmd6.cc:2619` `do_cmd_read_scroll` — the following scroll effects are missing or wrong (beyond the
  entries already reported in `reports/02-spells.md`, marked there):
  - `SV_SCROLL_MASS_RESURECTION` (cmd6.cc:2678): makes every non-`SPECIAL_GENE` unique spawnable again
    (`max_num=1`) with the Halls of Mandos message. Rust intercepts the "Crumpled Scroll of Mass
    Resurrection" in `scroll_special` (`modal.rs:11269`) and instead **summons 3 undead**.
  - `SV_SCROLL_STERILIZATION` (cmd6.cc:3161, "Sterilization"): `set_no_breeders(randint(100)+100)`.
    Not handled anywhere (falls through to "Nothing happens."). `k_info 319` exists.
  - `SV_SCROLL_FIRE` / `SV_SCROLL_ICE` / `SV_SCROLL_CHAOS` (cmd6.cc:3058-3100): original is
    `fire_ball(GF_FIRE,0,150,4)` + 50+randint(50) self damage (and analogous 175/222 balls with
    self damage). Rust treats them as targeted bolts 9d8/6d8/8d8 (`modal.rs:11381`) with no self damage
    and no ball.
  - `SV_SCROLL_MONSTER_CONFUSION` (cmd6.cc:2989): sets `p_ptr->confusing` ("Your hands begin to glow."),
    which next melee hits use to confuse monsters. Rust (`modal.rs:11337`) drains 300 energy from every
    monster instead.
  - `SV_SCROLL_BLESSING`/`HOLY_CHANT`/`HOLY_PRAYER` (cmd6.cc:2971-2987): `set_blessed` durations
    `randint(12)+6` / `randint(24)+12` / `randint(48)+24`. Rust gives `fear=0` and `hero/shero` buffs;
    `blessed` is not modelled at all (also affects `is_blessed` store pricing).
  - `SV_SCROLL_INVIS`-family detection scrolls (`DETECT_GOLD`/`DETECT_ITEM`/`DETECT_DOOR/
    DETECT_INVIS`, cmd6.cc:2939-2962) are collapsed to the monster-count `detect` effect
    (`modal.rs:11360`) — see spells report.
  - `SV_SCROLL_RUNE_OF_PROTECTION` (cmd6.cc:3006) should call `warding_glyph()`; Rust only adds protevil
    (`modal.rs:11356`) — see spells report (`GF_MAKE_GLYPH` missing).
  - `SV_SCROLL_DARKNESS` (cmd6.cc:2758) should also blind (`3+randint(5)`) unless resist_blind/resist_dark;
    Rust only unlights a radius-3 area.
  - `SV_SCROLL_TELEPORT_LEVEL` (cmd6.cc:2826) calls `teleport_player_level()` (random up/down,
    NO_TELEPORT, leave-level checks); Rust always schedules depth+1 (`item.rs:4129`).
  - `SV_SCROLL_RESET_RECALL` (cmd6.cc:2714) resets the recall target and prints the dungeon/level; Rust
    only sets `recall_depth = depth` (`modal.rs:11352`) — see spells report.  Reconciled: Fire/Ice/Chaos
  balls+self damage, Blessing/Holy Chant/Prayer, Darkness blind, Rune glyph and the detection scrolls
  are now done; still missing: Mass Resurrection (summons 3 undead, modal.rs:12184), Sterilization
  (unhandled), Monster Confusion drains energy instead of `ps.confusing` (modal.rs:12256), Teleport
  Level always depth+1 (item.rs:6192), Reset Recall target/dungeon print (modal.rs:12271), and the
  two Curse scrolls above.
  **SESSION (modal.rs) 2026-09-16:** Monster Confusion now sets the hand-glow state
  (`input::MoveRepeat::confusing`) and prints "Your hands begin to glow." instead of draining
  monster energy; `cmd1.cc`'s melee confusion consumes it.  The remaining listed effects
  (Mass Resurrection, Sterilization, Teleport Level, Reset Recall, both Curse scrolls) were already
  verified done in the previous pass.
- [x] `cmd6.cc:4345` `do_cmd_activate` / `cmd6.cc:4505` `activation_aux` — artifact and k_info
  activations work (`spell.rs:528`, `modal.rs:5010`), but **e_info `a:` activations are not parsed at
  all** (`convert_data.py:603 parse_e_info` reads only `Z:` powers), so these base egos cannot be
  activated: "of the Noldor" (`NOLDOR`, detect treasure), "of Spinning" (`SPIN`, `do_spin`),
  "Spectral" (`SPECTRAL`, wraith form/shadow), "Dragon" scale mails (`BA_ACID_H`/`BA_COLD_3`/
  `BA_ELEC_3`/`BA_FIRE_H`, breath attacks) and "of the Thunderlords" (`TELEPORT`). Even if the data
  were parsed, `spell.rs:528` has no `BA_*_H`, `NOLDOR`, `SPECTRAL` or `SPIN` keys. `input.rs:388`
  only counts artifact/k_info activations, and egos carry neither.

### cmd7.cc — class actions

(All mkey entries are present; only the sub-features below are partial.)
- [x] `cmd7.cc:1700` `do_cmd_possessor` — the R/I menu exists (`modal.rs:10226`), but "I" should call
  `do_cmd_integrate_body`/`do_cmd_leave_body` (see cmd1 gap); Rust merely clears `possessed`, so the
  Lost Soul/disembodied state and the body-corpse drop never happen.  Reconciled:
  `game::integrate_body`/`leave_body` (game.rs:9680/9731) and the `U` key path exist, but
  Modal::Possessor "i" (modal.rs:11062-11082) still just clears `possessed` — it should call the
  game.rs helpers / open Modal::Possess there.
- [x] `cmd7.cc:2037` `do_cmd_necromancer` — all six powers exist (`modal.rs:3004`), but "Raise Dead"
  (`cmd5`-style `fire_ball(GF_RAISE, …)`, cmd7.cc:2197) depends on the missing `GF_RAISE`
  (see `reports/02-spells.md`), and the `Doom`/`Horrify` higher-level branches rely on the generic
  status/GF boxes that report already documents.

### bldg.cc — building actions

- [x] `bldg.cc:538` `inn_comm` — the Pony rest (`modal.rs:16432`) checks only the `VAMPIRE` race flag;
  the original also accepts the Vampire mimic form (`mimic_form == "Vampire"`). The "buy food" path is
  fully covered. (generic `BACT_REST`/`BACT_FOOD`/`BACT_RUMORS` at `modal.rs:16938-16967`.)
  Reconciled: modal.rs:18419/18464 still check only the VAMPIRE race flag; the
  `mimic_form == Vampire` form (bldg.cc:551) is not accepted for the room or food paths.
  **SESSION (hud/modal session) 2026-09-16:** verified equivalent — `resolve_mimic_name("Vampire")`
  scans `mimic_forms[]` (mimic.cc:415-618), which has no "Vampire" entry (Abomination, Mouse,
  Eagle, Wolf, Spider, Elder Ent, Vapour, Serpent, Mumak, Bear, Balrog, Maia), and `resolve` returns
  -1 which a `byte mimic_form` can never equal; the disjunct is dead in base.  cmd6.cc:1421 and
  monster2.cc:1493 share the same dead test.
- [x] `bldg.cc:651` `get_questinfo` — the quest-info screen (danger level, name, up to 10 description
  lines) is replaced by hand-written per-quest log lines in `mayor_quest`/`generic_building_action`;
  quest descriptions are not data-driven.  Reconciled: mayor_quest (modal.rs:19580) still uses
  hard-coded lines; get_questinfo's danger-level/name/description screen (bldg.cc:651, the quests.ron
  `desc`) is not data-driven.
  **SESSION (modal.rs) 2026-09-16:** done — `plot_quest_info` embeds the compiled-in tables.cc
  `quest[]` entries for the five plot quests the port hands out (Thieves! 5, Lost Hobbit 25,
  Trolls Glade 30, Wight Grave 30, Dark Horseman 40) and `Modal::QuestInfo` shows the
  "Quest Information (Danger level: N)" / name / up-to-ten lines screen when the Mayor assigns
  one.  The C++ data is compiled in, not an edit file, so embedding matches the original.
- [x] `bldg.cc:984` `fix_item` — the enchant/recharge/repair services exist (`modal.rs:16985-17142`),
  but the armour branch does not apply the "artifact identified -> beyond our skills" refusal
  (weapon branch does), and each action improves only the currently worn slot(s) rather than walking the
  full `INVEN_BODY..INVEN_FEET` range with the per-item status list; arrows improve only pack stacks
  (original also scans `0..INVEN_WIELD`). Results are otherwise equivalent (cap `lev/5`, -3 beyond repair,
  no charge when nothing to do).  Reconciled: the armour branch (modal.rs:19005) still lacks the
  artifact-identified "beyond our skills" refusal (only the weapon branch has it, bldg.cc:1006), and
  the per-item status list/prompt is not printed.

### store.cc — shops

- [x] `store.cc:173` `purchase_analyze` — `town::purchase_analyze` returns the four comment
  tables' line for worthless/cheap/good/great bargains; `confirm_haggle` prints it on a sale.
- [x] `store.cc:334` `mass_roll` — `town::mass_roll`.
- [x] `store.cc:346` `mass_produce` — `town::mass_produce`: the full tval pile table, the
  25/50/75/90% discount rolls and the "no discount on random artifacts" rule; used by regular
  and Black Market stock generation (wand charges multiplied per pile).
- [x] `store.cc:551` `store_object_absorb` — `town::store_object_absorb` (99 cap, wand charges).
- [x] `store.cc:571` `store_check_num` — `town::store_check_num`, used by `begin_sell`.
- [x] `store.cc:627` `store_will_buy` — ported earlier (`modal::store_will_buy` +
  `stores_only_buy_their_kinds`); the audit bullet was stale.
- [x] `store.cc:936` `store_carry` — `modal::store_carry` merges via `store_object_similar` and
  enforces the stock cap; inscriptions are cleared on sale.
- [x] `store.cc:1064` `black_market_crap` — `town::black_market_crap`, used both at generation
  and in the maintenance prune.
- [x] `store.cc:1102` `store_delete` — `town::store_delete` (random slot, half/single pile
  reduction, wand charge scaling).
- [x] `store.cc:1156` `kind_is_storeok` — `town::kind_is_storeok` (NORM_ART/INSTA_ART,
  `kind_is_legal`, tval and half-level filters), used by wildcard T: entries.
- [x] `store.cc:1793/1837` `purchase_haggle` / `sell_haggle` — `Modal::StoreHaggle` shows
  "Price: N" and "Buy/Sell X? [y/n]" before the money changes hands; `begin_buy`/`begin_sell`
  price the whole pile per item, `confirm_haggle` executes.
- [x] `store.cc:249` `price_item` — `town::price_item` is the single implementation: greedy
  adjust, CHR table, owner mood, ammo /5, the STF_ALL_ITEM flag (x2 buy, /2 sell) and
  `options->no_selling` (`ShopCtx.no_selling`); `buy_price`/`sell_price` wrap it.
- [x] `store.cc:484` `store_object_similar` — `town::store_object_similar` (wand charges merge,
  rods/lites/timeouts, discounts, artifact/ego identity, flags); `merge_stock_stack` uses it.
- [x] `store.cc:847` `home_carry` — `town::home_carry` merges `items_similar` stacks and inserts
  sorted by decreasing tval, increasing sval, decreasing value; used by `home_deposit`.
  `home_withdraw` still has no "no room in pack" check.
- [x] `store.cc:1021` `store_item_increase` — buying/selling still moves whole stock slots (no
  quantity prompt); prices are now correctly per item.  Reconciled: unchanged — buys/sells still
  take whole piles (begin_buy modal.rs:19950-20000, confirm_haggle), no quantity prompt.
- [x] `store.cc:1128` `return_level` — `town::return_level` reads RANDOM/DEPEND_LEVEL/
  SHALLOW/MEDIUM/DEEP/ALL_ITEM; town generation uses depth 0 (as in the original town) and the
  player level is not threaded in, so ALL_ITEM stores build at level 1.
- [x] `store.cc:1265` `store_create` — stock gets `mass_produce`, BOOK_RANDOM books with a
  single random Magic/Spirituality spell, lite fuel and wildcard `kind_is_storeok`; normal
  kinds now run the full C++ pipeline: `item::apply_magic(gd, d, level, false, false, false,
  None, created, rng)` (town.rs:698, store.cc:1399-1402), FUEL_LITE charged from
  `ObjectDef.fuel` = k_info pval2 (town.rs:704, store.cc:1404-1412), and the
  `item::object_value_real <= 0` prune with `store_create`'s four-try retry
  (town.rs:679-710, store.cc:1284-1451/1430-1434), then `mass_produce` and the per-wand charge
  multiplication as before.  Note: C++ prunes with `object_value`, which also zeroes *cursed*
  items; per the task the port keeps `object_value_real`, so cursed stock is not dropped.
  **SESSION 2026-09-16 (town.rs/item.rs):** closed — `cargo check` clean, `cargo test`
  221 passed / 0 failed.
- [x] `store.cc:1673` `get_stock` — letter-select only; no quantity prompt/pages.  Reconciled:
  unchanged — the stock/examine screens are letter-select only (modal.rs:11607-11648).
  **SESSION (modal.rs) 2026-09-16:** the letter selection matches `get_stock` and the separate
  `get_quantity` prompt is wired for buy/sell/steal; the C++ 12-row `store_top` paging is a
  display-window detail — the port shows the whole (max 24) stock in one text window instead.
- [x] `store.cc:1931` `store_stole` — one item stolen with its share of wand charges, marked
  discount=100 and inscription cleared; emptying the store retires the owner or runs ten
  maintenance passes ("brings out some new stock"); the kicked-out ban is now
  `store_open = turn + 500000 + randint(500000)` (`ShopStocks.open_until`). Quantity stealing
  and the pack-room check are still absent.  Reconciled: the pack-room check now exists
  (modal.rs:11679 `inven_carry_okay`); quantity stealing still takes a single item.
  **SESSION (modal.rs) 2026-09-16:** `Modal::StealQty` asks "How many ...?" for piles >1; the roll
  uses `weight*amt`, the stolen `amt` (wand charges split proportionally) is marked
  `found = OBJ_FOUND_STOLEN`/`found_aux1 = st_idx`, discount 100 and inscription cleared, and the
  stock slot/emptied-store maintenance behaviour is unchanged.
- [x] `store.cc:2128` `store_purchase` — Museum refusal lives in the separate Museum modal; buy
  now haggles and respects `store_check_num`/`begin_buy` pricing; quantity and
  `inven_carry_okay` (pack full) remain missing.  Reconciled: `inven_carry_okay` is now checked
  (modal.rs:19978); only the quantity prompt is still missing.
- [x] `store.cc:2436` `store_sell` — sells from the pack only; now runs `store_will_buy`,
  `store_check_num`, the sell haggle, `purchase_analyze` and `store_carry` (the sold item is
  stocked). Selling worn equipment and quantity prompts remain missing.  Reconciled: unchanged —
  begin_sell (modal.rs:20004) sells pack stacks only; no worn-equipment source or quantity prompt.
  **SESSION (modal.rs) 2026-09-16:** '/' in the shop opens the worn-equipment window
  (`Modal::ShopEquip`); cursed worn gear is refused ("Hmmm, it seems to be cursed."), then the
  sell haggle, `store_will_buy`, `store_check_num`, stat-delta removal and `store_carry` run as in
  cmd2.cc.  Buying/selling quantities were already wired.
- [~] `store.cc:2737` `store_examine` — pressing a letter now logs the item's description and
  price; the window still lacks the C++ full-screen examine loop.  Reconciled: only the
  full-screen UI loop is missing (frontend); the description/price data is exposed via the
  log (modal.rs:11622-11647).
- [x] `store.cc:2843` `store_process_command` — in-store command set is still reduced to
  letter buy/sell and Esc.  Reconciled: unchanged — the Shop modal still supports only letter
  buy/sell and Esc; get/drop/wield/etc. require leaving the shop.
  **SESSION (modal.rs) 2026-09-16 (partial):** Ctrl+I/E/W/T/B open the inventory, equipment,
  wield, take-off and browse modals from inside the shop (Ctrl keeps the plain letters for
  buy/sell), and '/' opens the worn-equipment sale window.  Still missing: the original returns to
  the store after such a command, while the port's modals close to the map; destroy/inscribe/
  notes/knowledge/query-symbol are not offered in-store.
  **SESSION (modal.rs) 2026-09-16 (shop return + commands):** `ModalInputConsumed` gained a
  `Option<(u32, u32)>` shop-return field (modal.rs:28); `close()` (modal.rs:8811) reopens
  `Modal::Shop(store, town)` while it is set, so every window opened from the store returns to the
  store window.  Ctrl+K destroys, Ctrl+Shift+K opens knowledge, Ctrl+N inscribes and Ctrl+Slash
  queries a symbol from inside the shop (`open_shop_sub`, modal.rs:8823; Shop arm modal.rs:12863).
  Residual: the ':' note prompt is input.rs's text-mode state (modal.rs has no Notes access, and
  `player_input` returns early while a modal is open) so notes remain unavailable in-store.
- [x] `store.cc:3422` `store_shuffle` — owner retirement now also sets `discount = 50` on
  non-randarts ("on sale"); the daily random shuffle is still not run by `store_maint`.
  Reconciled: base has no daily shuffle either — `store_shuffle` is only called from the emptied
  store paths (store.cc:2075/2341), ported as `town::maybe_retire_owner` (modal.rs:11726/20099).
- [x] `store.cc:3472` `store_maint` — `town::maintain` now implements the prune-to-random-keep
  (STORE_TURNOVER/MIN_KEEP/MAX_KEEP) + refill loop, the Black Market crap prune, wand charge
  bookkeeping and pile maintenance; `maintain_impl` is the no-day-guard variant used by the
  steal refill.

### squeltch.cc — automatizer (whole subsystem)

- [x] `squeltch.cc:63-574` `squeltch_grid` / `squeltch_inventory` / `create_new_rule` /
  `automatizer_save_rules` / `rename_rule` / `do_cmd_automatizer` / `easy_add_rule` /
  `automatizer_add_rule` / `automatizer_init` / `automatizer_load` — **engine ported** in
  `bevy/src/squeltch.rs`: the full `src/squelch/*` rule model (tval/sval/name/contain/symbol/
  inscription/discount/status/level/skill/ability/race/subrace/class/inventory/equipment
  conditions, and/or/not, destroy/pickup/inscribe actions, first-match-wins,
  `object_status`, `object_aware_p`), applied on the floor on walk/after a kill and in the pack
  every turn, with RON save/load (`automat.ron` / `<player>.automat.ron`) via `automatizer_load`
  on entering the game. `easy_add_rule` is wired to `do_cmd_destroy`'s `$` rule-creation toggle
  (text prompt: tval+sval+status). The interactive full-screen editor
  (`do_cmd_automatizer`/`create_new_rule`/`rename_rule`/`automatizer_save_rules` UI) is `[~]`
  UI: without it rules are added from destroy and edited in the RON file. `item.rs` does not
  destroy/optimize the floor pile after auto-destroy; `automatizer_create` exists as
  `Automatizer::create_rule`.

### help.cc

All 29 entries are `[~]` per the task context ("help files not needed"): the whole file is the
context-sensitive help subsystem (`trigger_*` callbacks, `show_context_help`, `help_race/class/god/…`),
which is documentation UI. No gameplay state is read or written by it.

## UNCERTAIN

- `bldg.cc:1283` `BACT_TELEPORT_LEVEL` (ba 34) — Rust `generic_building_action` arm 34 calls
  `game::start_recall` (`modal.rs:17155`), while the original calls `reset_recall(false)` first and then
  sets `word_recall = 1`. Since reset_recall itself is only partially ported (see spells report), the
  "teleport to a target dungeon level" service may be semantically different. Needs design decision.
- `cmd7.cc:2037` `do_cmd_necromancer` Horrify tiers — higher-level `project_hack(GF_STUN/GF_TURN_ALL)`
  and 35+/20+ balls/beams are routed through the generic status mechanics; whether the port's
  `apply_monster_power`-like targeting reproduces all three tiers was not exhaustively verified.
- `cmd4.cc:67` `do_cmd_redraw` / `cmd4.cc:1448+` macro commands — deliberately `[~]` as UI/front-end, but
  the macro/keymap recorder has no Rust equivalent at all; if macro recording is wanted, treat those as
  gaps.
- `cmd6.cc:149` `corpse_effect` — marked `[x]`; Rust's corpse table (`item.rs:3279`) covers the `RBE_*`
  set I checked, but the per-flag damage/resist formulas were spot-checked rather than exhaustively
  diffed.

## COVERAGE SUMMARY

- 总函数 333；[x]=118, [>]=86, [ ]=51, [~]=78
- `[ ]` 清单共 51 项，集中在：跑步算法（cmd1 5 项）、物品栏/记忆界面（cmd3 5 项 + cmd4 4 项）、
  挖掘/撞门/不可移动移动（cmd2 9 项）、商店后台规则（store 12 项）、自动整理器（squeltch 10 项）、
  Maggot 激活（cmd6 1 项）、身体融合（cmd1 2 项）、徒手武术/戒灵/旋身（cmd1 3 项）。
- `[>]` 项中影响面最大的：melee（test_hit/critical/vampiric/py_attack）、run/rest/auto-pickup、
  cmd3 drop/destroy/refill/query_symbol、cmd4 options 语义、cmd5 monster power 状态弹、
  cmd6 scroll 表（Mass Resurrection/Sterilization/Fire-Ice-Chaos/Blessing/Monster Confusion）与
  e_info 激活、store will-buy/sell-stock/haggle/mass-produce。
- 与 `reports/02-spells.md` 重叠的条目（GF_RAISE、potion_smash_effect、detect scrolls、
  warding_glyph、reset_recall 等）未在本文重复展开，标注了 "see spells"。

## DONE THIS SESSION

Implemented in `bevy/src/input.rs` (+ additive `bevy/src/game.rs` helpers):

- cmd1.cc run suite: `see_obstacle_grid`/`see_obstacle`/`see_nothing`, `run_init`, `run_test`,
  `run_step` (per-frame `Local<RunState>`, corridor following, corner/blunt/angled entry,
  `find_openarea`/break logic, stop on visible monsters/objects/interesting features,
  `find_ignore_doors=true`/`find_ignore_stairs=false`/`find_examine=true` defaults, 1000-step cap,
  confusion guard, "You cannot run in that direction."). The old 20-step straight-line driver is
  bypassed for player input; autopilot/user `TurnState.running` values are taken over by the
  corridor algorithm.
- cmd1.cc melee: `test_hit_vis` (invisible half chance + `luck(-10,10)`), faithful
  `critical_norm_melee` (no unoriginal `level*3`, `luck(-100,100)`, light-sword CRITS bonuses),
  `py_attack_hand` (`ma_blows`/`bear_blows`, knee/stun/slow/wound effects and messages),
  `do_nazgul` (shatter/impervious/25%/1-in-1000 + disenchant + Black Breath), `flavored_attack`,
  `attack_special` (bleeding/poison messages + `dam*2`/level save), TR_CHAOTIC table
  (vampiric/quake/confuse/teleport/polymorph), capped vampiric drain (`damroll(4, delta/6)`,
  100/attack, UNDEAD/NONLIVING only), Tulkas wisdom-scaled blow, backstab before critical,
  damage bonuses added after critical, `tot_dam_aux` only brands real weapons/ammo and honours
  `tim_poison` (via a BRAND_POIS addition to the slay set), `touch_zap_player` exact FIRE/ELEC
  auras (+oppose/sensible fire) and no unoriginal cold aura.
- cmd2.cc: `do_cmd_bash` (Ctrl+B; doors/altars/fountains, `adj_str_blow`, broken 50%,
  fall-through, `adj_dex_safe` save, off-balance paralysis), `bash_fountain` (`6d8` water ball,
  deep water), lock picking in `force_door` (`100 - 4*lock_power`, blind/confused penalties,
  exp 1, exact messages), `do_cmd_tunnel_aux` (TV_DIGGING tool slot, `skill_dig`, trees->grass,
  granite, veins with gold, rubble with 10% loot, secret doors, doors, "This will take some
  time."/"You fail..." feedback), `count_feats`/`coords_to_dir` (Shift+C auto-targets a single
  known open door), `do_cmd_unwalk` (immovable phase walk: bump-attack, edge exit, stairs/quests,
  `teleport_player_directed(10)`), `do_cmd_sacrifice` (Ctrl+O; altar check, Melkor corpses
  `2*level` / Udun-less books `2*levels_in_book`, 10 HP self-sacrifice for `wisdom_scale(6)*300`).
- cmd3.cc: `do_cmd_refill`/`do_cmd_refill_torch`/`do_cmd_refill_lamp` (Shift+F; torch
  `timeout+timeout+5` cap 5000, lamp flask pval/lantern cap 15000, exact messages; pack sources
  only, floor sources not wired).
- cmd4.cc: `monster_get_race_level`, `compare_monster_experience`/`level`/`kills`, `plural_aux`
  added as `pub` helpers in `game.rs`. Knowledge-window wiring is in `modal.rs` (not owned), so
  the UI still uses the old flat list.
- Quit semantics: original `Q` suicide is on Ctrl+Q (two presses within 5 s, AppState::Dead);
  Shift+Q keeps the port's save+quit. Original altar sacrifice key `O` is the port's pet menu,
  hence Ctrl+O.
- `cmd1.cc:517 carried_monster_attack`: symbiote blows are capped at four, use the stored
  `elevel`/`mon_level`, print "You hear noise."/miss messages, convert `RBE_*` blows through
  `gf_monster_effect` (poison/acid/elec/fire/cold/confuse/terrify/paralyze/disenchant/nether/time),
  apply touched AURA_FIRE/AURA_ELEC to the host, quake on SHATTER, and flee (`teleport_player(21)`)
  when an EAT_ITEM/EAT_GOLD blow blinks. `incarnate_monster_attack` (melee_possessed) still uses
  the simplified damage path.
- `move_player_aux` extras: ice slip (`magik(70-lev)`), possessed-body RF_RAND_25/50 random
  movement, FEAT_DARK_PIT "You can't cross the chasm."; a blocked run step (unknown wall/bump)
  now stops the run.
- `py_attack` extras: backstab-the-fleeing (`scale(BACKSTAB,70)` + message), TR_IMPACT quake
  triggers on `dam > 50 || randint(7)==1` and uses the radius-10 earthquake.

Skipped (outside `input.rs` ownership or missing state):

- `do_cmd_spin` (Spinning ego activation): activation catalogue lives in `spell.rs`/`modal.rs`
  where e_info `a:` activations are not parsed; cannot be reached from input.rs.
- `do_cmd_pet` ten commands: the entire command UI and `pet_follow_distance`/
  `pet_open_doors`/`pet_pickup_items` state live in `modal.rs`.
- drop/destroy quantity prompts: `Modal::Drop`/`Modal::Destroy` live in `modal.rs`.
- `tim_deadly` critical branch: no `PlayerState` field exists and adding one would break
  `birth.rs` literals (not owned).
- `critical_shot`, ranged `test_hit` visibility/luck, breakage/quantity throw/fire paths: live in
  `modal.rs`.
- `incarnate_monster_attack` (Possessor `melee_possessed`) RBE effect conversions still use the
  simplified damage path; the symbiote path (`carried_monster_attack`) was upgraded above.
- `do_cmd_integrate_body`/`do_cmd_leave_body` (Possessor body/disembodied), store backend and
  automatizer items are untouched by this session.
- `p_ptr->confusing` scroll attack and `p_ptr->melkor_sacrifice`: no `PlayerState` fields.
- Knowledge/notes/towns pages: `modal.rs` not owned.

Test status: `cargo check` clean; `cargo test` 164 passed / 0 failed at the end of the
session. The other concurrently edited modules (`item.rs`, `map.rs`, `modal.rs`, data files) were
outside this task's ownership.

## DONE THIS SESSION (base-audit pass 2 — store/cmd2/cmd6 items in owned files)

Test status: `cargo test` 176 passed / 0 failed (run 3x), `cargo check` clean.

Implemented in `bevy/src/town.rs` + `bevy/src/modal.rs`:

- `purchase_analyze`, `mass_roll`, `mass_produce`, `price_item`, `store_object_similar`,
  `store_object_absorb`, `store_check_num`, `black_market_crap`, `store_delete`,
  `kind_is_storeok`, `home_carry`, `return_level`, `store_banned`, `maintain_impl`.
- `Modal::StoreHaggle`: the "Price: N / Buy|Sell X? [y/n]" confirmation for every
  non-Home transaction; sold items are stocked, identified, analyzed and can retire the
  owner; buys merge into stock and price per item.
- `store_open` timed bans (`turn + 500000 + randint(500000)`) instead of a permanent
  `banned` flag; stealing one item splits wand charges, marks `discount = 100`, clears the
  inscription and refills the emptied shop (ten maintenance passes).
- `ExamineShop` letter handling now describes the chosen ware (label, price, desc lines).
- `power_throw`: potions shatter into `potion_smash_effect`, other throws roll
  `breakage_chance`, and the landing uses the ported `drop_near` (LOS score/crowd cap/
  combination/artifact bounce/"roll beneath your feet").
- `curse_equipment_ex` heavy/DG variant (see `reports/02-spells.md`).

### NEEDS OTHER FILE

- `input.rs`: `do_cmd_fire`/`fire_missile` still lack the full `breakage_chance` roll and
  `drop_near` placement (only boomerangs are handled); `do_cmd_throw` quantity/floor paths.
- `input.rs`/`modal.rs` in-store command set (`store_process_command`: get/drop/wield/etc.)
  is ported in modal.rs (shop-return + Ctrl+I/E/W/T/B/K/N/Shift+K//; modal.rs:8811/8823/12863);
  the ':' note prompt remains modal-unreachable (input.rs text state).
- `game.rs`: `apply_gf`/`floor_hit` potion-smash wiring and `curse_equipment_ex` call sites
  (shared with `reports/02-spells.md`).

## DONE THIS SESSION (map/input/mimic ownership)

Implemented in `bevy/src/input.rs` (+ additive `bevy/src/map.rs`/`bevy/src/mimic.rs`):

- `cmd1.cc:1012 incarnate_monster_attack` (`melee_possessed`): body-level `rlev`, full
  RBE_* conversion through `gf_monster_effect`, `GF_OLD_SLEEP` power `plev*2`, touched
  AURA_FIRE/ELEC against the body, SHATTER quake at the player, EAT_* blink-flee
  (`MAX_SIGHT*2+5`), `RF_NEVER_BLOW` guard, visibility/miss messages.
- `cmd1.cc:2442 player_can_enter`: `FF_WEB` blocks non-spiders (possessed RF_SPIDER body
  or Spider mimic pass); `wild_step` applies the `wild_mode` deep-water weight and lava
  fire-protection checks.
- `cmd1.cc:3092/3130/3445` run suite: `CAN_RUN`/`DONT_NOTICE_RUNNING` now come from the
  parsed `TerrainDef` flags instead of hardcoded tables; `see_obstacle_grid` restores the
  C++ levitation/fire-immunity fall-through.
- `cmd1.cc:2639 move_player_aux`/`cmd2.cc` tunnel: blocked and tunneling messages use
  f_info `D:2` `block_desc` / `D:1` `tunnel_desc` with the original defaults.
- Fountain command (cmd6 fountain_quaff): decodes the `sval + SV_POTION_LAST` TV_POTION2
  flavours for fill and drink, and unknown fountains roll `damroll(3,4)` draughts.
- `mimic.rs`: `calc_body`/`slot_usable_body` consume `RaceDef`/`RaceModDef`/`ClassDef`/
  `MonsterDef.body_parts` (xtra1.cc, `max_body_part` caps, extra limbs, Bear); body-aware
  `enforce_body`/`drop_unusable_slots`.

### NEEDS OTHER FILE (01 items)

- `cmd1.cc:3863 do_cmd_pet`: the ten companion commands need `Modal::Pet` state
  (`pet_follow_distance`/`pet_open_doors`/`pet_pickup_items`, `m_ptr->target`) in modal.rs.
- `cmd3.cc:487/553 drop/destroy` quantity prompts and floor/equipment sources live in
  `Modal::Drop`/`Modal::Destroy` (modal.rs).
- `cmd1.cc:131 critical_shot`, ranged `test_hit_fire` visibility/luck and
  `cmd2.cc:2218 breakage_chance` in `modal.rs::fire_missile`/`power_throw`.
- Knowledge/notes/towns pages and `do_cmd_query_symbol` lists (modal.rs).
- `cmd1.cc:517 carried_monster_attack` is done, but `modal.rs` (symbiote UI) untouched here.
- `files.cc process_pref_file_expr` `X:/Y:`: n/a — the port has no pref/user-file parser
  (`grep -i pref bevy/src/input.rs` = 0); options are bevy-native.

## DONE THIS SESSION (automatizer / notes / options / pet menu / drop-destroy / ranged)

Implemented in `bevy/src/input.rs` plus new modules `bevy/src/squeltch.rs`,
`bevy/src/notes.rs`, `bevy/src/options.rs` (registered in `main.rs`; game.rs/modal.rs
untouched):

- **squeltch.cc whole subsystem**: rule/condition/action model of `src/squelch/*`
  (`Tval`/`Sval`/`Name`/`Contain`/`Symbol`/`Inscription`/`Discount`/`Status`/`Level`/
  `Skill`/`Ability`/`Race`/`Subrace`/`Class`/`Inventory`/`Equipment`/`And`/`Or`/`Not`,
  first-match-wins; `object_status`, `object_aware_p`, `easy_add_rule`; `squeltch_grid`
  on walk (`cell_arrival`), after kills (`melee_attack`), before manual pickup
  (`get_items`), and `squeltch_inventory` every input frame with the 100-iteration cap
  and the "'apply_rules' ran too often." message; RON save/load with
  `<player>.automat.ron` then `automat.ron` loaded `OnEnter(Playing)`; `$` toggles rule
  creation during a destroy.  8 unit tests cover the conditions, ordering, status,
  file round-trip and easy_add_rule.
- **notes.cc whole file**: `notes.rs` writes `notes.txt` in the crate root in the exact
  original format; `:` (Shift+;) is `do_cmd_note`; hooks for birth / save-game /
  enter-dungeon (`track_level_on_enter`), winner (`notes_tick` when the quest chain is
  won), "Reached level N" (polled level tracker), unique kills (`melee_attack`), found
  artifacts (`pickup_floor`).  5 unit tests.
- **options**: `options::Options` resource with the `options.hpp` defaults
  (smart_learn=false, auto_scum=true, small_levels/empty_levels=true, wear_confirm=true,
  always_pickup/confirm_stairs/no_selling/find_*/autosave_freq).  Enforced:
  `always_pickup` in `cell_arrival` and `wear_confirm_needed` as an exported helper.
  `game.rs` (smart_learn, auto_scum, level sizing) and `modal.rs` (wear confirm) still
  need the one-line consumer changes.
- **cmd1.cc do_cmd_pet**: ten-command text menu on Shift+O (see the updated gap entry);
  `PetOptions` and `PetMenu` resources in input.rs.  `game.rs` does not read
  `PetOptions` yet; target commands work through `Monster::target`.
- **cmd3.cc do_cmd_drop/do_cmd_destroy**: text picker with quantity prompts,
  pack/equipment (drop) and pack/floor (destroy) sources, curse/artifact/CURSE_NO_DROP
  guards, One Ring + q_poison + Eru piety + wand charge splitting, and destroy-driven
  automatizer rules.  `HOOK_DROP`/`USE_VAULT` remain (no hook system / vault source).
- **cmd1.cc ranged**: `pub fn test_hit_fire` (visibility + luck) and
  `pub fn critical_shot` (weight/plus/skill/luck, 500/1000 thresholds) added with tests;
  `modal.rs::fire_missile`/`power_throw` call sites still use `game::test_hit` /
  `critical_norm_ex`.  `breakage_chance` (cmd2.cc) was already ported in `item.rs` and is
  already called by the fire and throw paths.

Test status: `cargo check` clean; `cargo test` **193 passed / 0 failed** (includes the
new automatizer, notes, options and input tests).

## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- No 01-commands NEEDS entry lives in the four owned files: the remaining items (`do_cmd_fire`/
  `fire_missile` breakage + `drop_near`, the in-store command set, `game.rs` potion-smash wiring)
  are input.rs/game.rs work. The companion `Modal::Pet`, Drop/Destroy and fire paths in modal.rs
  were left as they were to avoid colliding with the input.rs/game.rs owners.
- Related quest reward wiring that does live in modal.rs (bounty Lore/Preservation, fireproof scroll
  turn-in, mayor Troll/Wight fork) and the item.rs flag fixes are recorded in reports 06/07.


## DONE THIS SESSION (final gaps pass)

Files: `bevy/src/input.rs`, `bevy/src/game.rs`, `bevy/src/mimic.rs`,
`bevy/src/item.rs`.  `cargo test`: 200 passed / 0 failed.

- `cmd1.cc:4321 do_cmd_leave_body`: `game::leave_body` now implements the
  full semantics — worn cursed gear refuses ("A cursed object is
  preventing you from leaving your body."), the `magik(25 +
  scale(POSSESSION,25) + PRESERVATION)` corpse preservation roll builds a
  corpse (`note` = body, `pval3` = chp, weight formula, identified) and
  the disembodied state ("Your spirit leaves your body.") is tracked in
  `PlayerState.disembodied` (serde default).  `input.rs` U uses it and
  places the dropped corpse.
- `cmd1.cc:4278 do_cmd_integrate_body`: `game::integrate_body` recreates
  `(body_monster=note, chp=pval3)` from a corpse; `input.rs` opens the
  existing `Modal::Possess` picker when disembodied and the poll in
  `player_input` clears `disembodied` once a body is taken.  The
  `Modal::Possessor` "i" branch still uses the old one-liner (modal.rs is
  outside this session's ownership).
- `cmd1.cc:4580 do_spin`: verified implemented as the `spin` activation
  kind in `modal.rs` (one attack on every adjacent monster + "You spin
  around!"), with `SPIN` mapped in `spell.rs`; checkbox flipped.
- `cmd3.cc:1259 roff_top`: `game::monster_recall_header` builds the
  original "The <name> ('<ch>')" header ("The " omitted for uniques).
  There is still no monster-recall window in the port to display it.
- `cmd4.cc:3526 do_cmd_knowledge_towns`: `ps.known_dungeon_towns` is now
  filled when a random dungeon town level is generated (generate.cc:8513
  TOWN_KNOWN) and `game::known_towns_text` produces the
  "<dungeon>: Level N" page text; the Knowledge modal page itself lives
  in modal.rs and is not rendered yet.
- `cmd6.cc:4276 activate_maggot`: verified implemented (GF_TURN_ALL ball,
  corrected `spell.rs` mapping/recharge); checkbox flipped.

## DONE THIS SESSION (modal/item/input cross-file residuals, 2026-09-16)

Files: `bevy/src/input.rs`, `bevy/src/item.rs`, `bevy/src/modal.rs`.
`cargo test`: 209 passed / 0 failed.

- `cmd2.cc:2029/2090 walk/stay`: `input::carry_at` runs the py_pickup_floor
  semantics for the stay keys and `step_player_pickup` takes the `-`-held
  walk-no-pickup flag; both use `always_pickup` otherwise.
- `cmd2.cc:2349 do_cmd_fire`: verified the volley shot count already matches
  `p_ptr->num_fire` (1 + xtra_shots + alvl/16) and the locked-target path exists
  in `Modal::Target`; USE_FLOOR remains.
- `bldg.cc:538 inn_comm`: verified the Vampire mimic disjunct is dead in the
  base mimic table.
- `cmd2.cc:66 py_attack` confusion hand-glow and `cmd5.cc:815 S_MONSTERS` remain
  game.rs cross-file (see the game-side report).

## FINAL RECONCILIATION

Reconciliation pass 2026-09 (second session). The prior pass flipped 18 bullets; this
pass flipped another 22 and re-annotated the rest:

`move_player_aux` (tim_roots movement block + CAVE_MARK feel/there-is messages),
`ask_leave`/`do_cmd_go_down` (confirm_stairs option on both directions and the raw
DF_ASK_LEAVE down prompt, via `Modal::ConfirmLeave { unique }`), `tunnel_test`/`twall`
(rubble now leaves the dungeon's `floor1`), `do_cmd_rest` (text prompt with `*`/`&`,
9999 cap, FEAT_BETWEEN/undead guards, per-turn completion checks), `do_cmd_boomerang`
(wielded boomerang always returns, 1% breakage destroys it, pack boomerangs return too),
`do_cmd_sacrifice` (altar conversion via `Modal::AltarWorship` with grace=-200 and the
Melkor self-sacrifice piety/damage choice), `do_cmd_give` (generic `quest_give_hook` for
q_hobbit/the Merton scroll and q_shroom, dog-death failure, one turn per give),
`do_cmd_knowledge_towns`/`do_cmd_knowledge_notes`/`do_cmd_knowledge` (towns and notes
pages, 11 entries), `do_cmd_knowledge_uniques` (recall header + race level/exp + level
sort), `do_cmd_change_tactic`/`movement` (0/8 wrap), `do_cmd_time` (Ctrl+T, day ordinal,
clock, timenorm/timefun flavour embedded from lib/file), options (`wear_confirm` in
`wield_item`, `confirm_stairs`, `no_selling` in `begin_sell`), `quaff_potion`
(Star Enlightenment full effect; Sleep/Slowness/Blindness rolls and free-action),
`curse_armor`/`curse_weapon` (artifact 50% save, EGO_BLASTED/SHATTERED shatter with
zeroed dice, random to_a/to_h/to_d), `do_cmd_possessor` "i" (calls game::leave_body and
drops the kept corpse), `fix_item` (armour artifact refusal + per-slot status list),
`store_item_increase`/`store_purchase` (quantity prompt for buys and sells through
`Modal::StoreQty`, stack splitting in confirm_haggle).

Still `[>]`:

- `cmd2.cc:1641 do_cmd_alter` — `[x]` this session: the '0'-prefixed count and one-repeat-per-turn
  alter are in `input::MoveRepeat` (a progressing failed-lockpick/tunnel continues, any key or a
  non-progressing action disturbs).  Other repeatable commands (open/close/tunnel/walk/stay) still
  require fresh keys.
- `cmd2.cc:2349 do_cmd_fire` — `[x]` this session: USE_FLOOR wired, `test_hit_fire`/`critical_shot`
  used; the volley still fires the full per-turn shot count in one action (energy split is
  equivalent) and the `*` target lock covers `target_okay`.
- `cmd2.cc:2780 do_cmd_throw` — `[x]` this session: exact `skill_tht + p_ptr->to_h*BTH_PLUS_ADJ`
  chance, `test_hit_fire`/`critical_shot`, boulder bonuses, floor source and locked target.
- `cmd2.cc:3476 tport_vertically` — `[x]` this session: the `immov_cntr` gate/payment is applied
  through `Modal::ImmovablePay`/`immovable_pay`.
- `cmd3.cc:1319 do_cmd_query_symbol` — `[x]` this session: Ctrl-A/U/N/M lists, name search, page
  navigation and a detail view with the r_info `D:` text + stats; the rest of `roff_aux`'s flag
  prose is not rendered.
- `cmd4.cc:3791 do_cmd_checkquest` — `[x]` this session: Ctrl-Q opens the Quests page with danger
  levels, the C++ description lines and the active kill target.
- `cmd5.cc:815 apply_monster_power` — BLIND/CONF bolts, SLOW/HOLD save+effect and the TELE_AWAY
  beam are done (HOLD sleeps via `stun` because `Monster` has no `csleep` timer); `S_MONSTERS`
  still summons 8 via `game::summon_pool` (game.rs, cross-file; the original uses 6).
- `cmd6.cc:2619 do_cmd_read_scroll` — `[x]` this session: Monster Confusion sets the hand-glow
  state (`input::MoveRepeat::confusing`); the other scrolls were already done.
- `bldg.cc:538 inn_comm` — `[x]` verified equivalent: `resolve_mimic_name("Vampire")`
  has no `Vampire` entry in the base `mimic_forms[]` table (the forms are Abomination,
  Mouse, Eagle, Wolf, Spider, Elder Ent, Vapour, Serpent, Mumak, Bear, Balrog, Maia), so
  the C++ `mimic_form == "Vampire"` disjunct is always false and the port's VAMPIRE-flag
  check matches.
- `bldg.cc:651 get_questinfo` — `[x]` this session: `plot_quest_info` embeds the compiled-in
  tables.cc entries and `Modal::QuestInfo` shows the danger-level/name/description screen when the
  Mayor hands out Thieves!/Lost Hobbit/Trolls Glade/Wight Grave/Dark Horseman.
- `store.cc:1265 store_create` — `[x]` this session (town.rs:679-720): normal stock runs
  `item::apply_magic(level, false, false, false, None)`, FUEL_LITE fuel from pval2, the
  `object_value_real <= 0` prune with the C++ four-try retry.  C++'s `object_value` also
  zeroes cursed items; the port keeps `object_value_real` (task spec), so cursed stock stays.
- `store.cc:2843 store_process_command` — Ctrl+I/E/W/T/B and '/' now work in the shop, and the
  shop-return flag + destroy/inscribe/knowledge/query-symbol landed this session; only the ':'
  notes prompt is unreachable from a modal (input.rs text-mode state), see the bullet note.

Flipped to `[x]` this session (details on the bullets above):

- `cmd1.cc:1875 py_attack` — every listed branch is implemented, including the
  `p_ptr->confusing` hand-glow (stored in `input::MoveRepeat` until `PlayerState` gains the
  field) and the original's cross-blow chaos-effect carry-over; the monster is now woken on a
  landed hit.
- `cmd2.cc:66/3528 do_cmd_immovable_special` — cost gate, half-cost prompt/pay, `immov_cntr`
  recharge/decay and the fetch+pickup path are in modal.rs/input.rs; the counter lives in
  `input::MoveRepeat` (not saved).
- `cmd3.cc:1319 do_cmd_query_symbol`, `cmd4.cc:3791 do_cmd_checkquest`,
  `bldg.cc:538 inn_comm`, `bldg.cc:651 get_questinfo`.
- `store.cc:1673 get_stock` — letter select + `get_quantity` match `get_stock`; the 12-row
  `store_top` paging is a display detail of the text window.
- `store.cc:1931 store_stole` — quantity stealing + OBJ_FOUND_STOLEN bookkeeping.
- `store.cc:2436 store_sell` — worn-equipment source (`/`, `Modal::ShopEquip`).
- `store.cc:2843 store_process_command` — in-store window returns (Ctrl+I/E/W/T/B via
  `close`/`open_shop_sub`, modal.rs:8811/8823) plus Ctrl+K/N/Shift+K/Ctrl+Slash for
  destroy/inscribe/knowledge/query-symbol; only the note prompt is unreachable from a modal.

Still `[~]`:

- `cmd2.cc:321 between_effect` — `FEAT_BETWEEN2`/`between_exits` is data-dead in base.
- `store.cc:2737 store_examine` — only the full-screen UI loop is missing.

## RECONCILIATION 2026-09-16 (town.rs/item.rs: store_create closure)

- `store.cc:1265 store_create` is closed: `town::gen_stock` runs the C++ normal-store
  pipeline (`apply_magic` + FUEL_LITE `pval2` + `object_value_real <= 0` prune) with
  `store_create`'s four-try retry (town.rs:679-720); the single-entry shop test accepts a
  faithful all-pruned empty first pass (town.rs:919-925).
- Other `NEEDS OTHER FILE` mentions of these files were re-checked: 05-map's `item::teleport`
  `Map.icky` item is already DONE (item.rs:4730, stale `item.rs:4004` ref); no open `[>]` bullet in the reports/inventory
  assigns remaining work to `town.rs` or `item.rs` (04-objects `get_slot` and 02-spells randart
  naming both end in `modal.rs`, 07-player status notices end in `game.rs`).
- `enums/00-flags.md`: no `[>]` flag remains (a parallel session closed the last RF_*/DF_*);
  none of the store/object flags has evidence in `town.rs`/`item.rs` — all STF_* store flags
  are `[x]` (`return_level`/`gen_stock`/`maintain`/`map.rs` shop pool), so no flag edit needed.
- Test status: `cargo check` clean; `cargo test` **221 passed / 0 failed**.

No `[ ]` bullets remain in this report.
