# 03-monsters audit report

Scope: `src/melee1.cc`, `src/melee2.cc`, `src/monster1.cc`, `src/monster2.cc`,
`src/monster3.cc`, `src/monster_spell.cc`, `src/monster_type.cc` vs
`bevy/src/{game,input,item,modal,render}.rs`.

Pre-verified per instructions:
- `SUMMON_SPIDER` / `SUMMON_BUG` / `SUMMON_RNG`: Rust `summon_pool`
  (`game.rs:12013`) uses `("S")`, `("bug" = name contains "Software bug")`,
  `("rng" = name contains "Random Number Generator")`, matching the C++
  type filters in `monster2.cc:2931/3118/3124`; no gap in those specific
  branches. The *general* summon machinery around them is partial (see the
  `summon_specific_okay`/`summon_specific` gaps below).
- Known-done modules (blow effect table, spell/breath matrix, ego monsters,
  corpses, monster control, target selector, NEUTRAL/FORCE_MAXHP/KILL_TREES/
  FORCE_DEPTH, speech, DROP_*) are marked `[x]` and not re-reported.

## GAPS

### melee1.cc — monster blows

- [x] `melee1.cc:1391` `make_attack_normal` — player dodge is never rolled.
  C++ computes `chance = p_ptr->dodge_chance - rlev*5/6` before the hit and
  `continue`s on success. Rust `GameData` has `dodge_chance()` (`game.rs:2741`)
  but it is only used by the character-sheet modal (`modal.rs:763,3812`); the
  monster melee port `game.rs:9223-9232` checks only protevil. Suggest adding
  the roll right after `hit` in `monster_turns` (`game.rs:9223`).
- [x] `melee1.cc:1404-1420` `make_attack_normal` — Eru's intervention is
  missing: `praying_to(GOD_ERU)` + `grace - rlev*300` chance (0..50000) to
  stop an evil blow. No Rust equivalent ("hand of Eru" appears only in
  prayer flavour text `modal.rs:3543`).
- [x] `melee1.cc:1720-1773` `make_attack_normal` RBE_UN_POWER — Rust
  (`game.rs:10158-10171`) drains any carried item with `charges > 0`, while
  C++ only drains non-artifact `TV_STAFF`/`TV_WAND` and gives the monster
  `rlev * pval * number` HP (`m_ptr->hp += …`, capped). The monster heal and
  the tval/artifact filters are missing.
- [x] `melee1.cc:1775-1844` `make_attack_normal` RBE_EAT_GOLD — several
  divergences in `game.rs:10172-10180`: (a) C++ saving throw is
  `rand_int(100) < adj_dex_safe[dex] + p_ptr->lev`, Rust uses the generic
  `skill_save`; (b) the `gold > 5000` second formula (`au/20+randint(3000)`)
  is missing; (c) Rust's `(au/10 + randint(25)).min(au).max(2)` can steal
  more than the player owns when `au == 1` (min before max); (d) stolen gold
  must be transferred to the monster via `monster_carry` (gold objects), and
  the "thief flees laughing" blink/teleport is missing (see below).
- [x] `melee1.cc:1846-1948` `make_attack_normal` RBE_EAT_ITEM — Rust
  (`game.rs:10181-10193`) removes the whole `Item` stack instead of one
  item (`inc_stack_size_ex(i,-1)`), never copies the stolen object into the
  monster's inventory (`o_pop`+held), and on a successful save does not set
  `blinked = true` as C++ does.
- [x] `melee1.cc:1950-1989` RBE_EAT_FOOD — Rust removes the whole food stack
  (`inv.pack.remove(i)`, `game.rs:10199`); C++ removes exactly one item.
  Rust's extra "You feel hungry!" fallback does not exist in C++.
- [x] `melee1.cc:1991-2017` RBE_EAT_LITE — C++ requires `o_ptr->pval > 0`
  and `!artifact_p(o_ptr)`; Rust (`game.rs:10207-10218`) tests the `FUEL_LITE`
  item flag only, so FUEL_LITE *artifacts* can be drained, and the
  "Your light dims." message is printed even while blind.
- [x] `melee1.cc:2263-2365` RBE_EXP_10/20/40/80 — Rust drains a fixed
  maximum roll: `let amount = (n * 6)` (`game.rs:10276`) instead of
  `damroll(n, 6)` (random 10d6/20d6/40d6/80d6). Average drain is roughly
  doubled. Also the C++ "You keep hold of your life force!" /
  "You feel your life slipping away!" distinct messages are collapsed into
  one message in `item::lose_exp` (`item.rs:2554-2575`).
- [x] `melee1.cc:2405-2472` RBE_TIME — C++ cases 6-9 do
  `stat_cur[stat] = stat_cur[stat]*3/4` (min 3) and case 10 does it for all
  six stats; Rust (`game.rs:10279-10293`) calls `drain_stat`, which
  subtracts 1 only (`item.rs:2530-2540`). The case-1-5 XP branch must not
  apply `hold_life` (C++ calls `lose_exp` directly), while Rust passes
  `block_pct=0` through `lose_exp` which still halves the loss when
  HOLD_LIFE is worn (`item.rs:2562-2570`).
- [x] `melee1.cc:937-958` (and 2367-2387) RBE_DISEASE — C++
  `dec_stat(A_CON, randint(10), perm)` with `perm = (randint(10)==1)`; Rust
  (`game.rs:10142-10153`) does `ps.stats[CON] -= 1` with no random amount and
  no 1% permanent loss. Same for the CON branch of `make_attack_normal`.
- [x] `melee1.cc:720-807`/`2162-2242` RBE_LOSE_* — C++ `do_dec_stat`
  (`spells2.cc:273-311` → `dec_stat`, `spells1.cc:2055`) removes 5 points
  from stats above 18 (`loss = max(adjusted_roll, amount/2)` with
  `amount=10`), not 1; Rust `item::drain_stat` always uses -1. C++ also uses
  the "You feel very weak." message.
- [x] `melee1.cc:2580-2587` RBE_EXPLODE — C++ kills the attacker only when
  the blow whose `method == RBM_EXPLODE` itself hit; Rust kills it whenever
  *any* blow hit and the monster has an EXPLODE blow in its list
  (`game.rs:9438-9442`).
- [x] `melee1.cc:2589-2672` shield ripostes — C++ gates all
  fire/elec/mystic/fiery/fear shield reactions on `touched`, which is only
  set by HIT/TOUCH/PUNCH/KICK/CLAW/BITE/STING/BUTT/CRUSH/ENGULF/CHARGE/CRAWL.
  Rust triggers them on `hit_any` for any hit blow (e.g. GAZE/DROOL/SPORE)
  (`game.rs:9347-9433`), and applies one riposte per monster, not per
  touching blow.
- [x] `melee1.cc:226-1180` `carried_make_attack_normal` — the symbiote
  rebellion in `upkeep` (`game.rs:7487-7513`) only sums all blow dice and
  calls `player_hurt`; C++ rolls to-hit per blow (`check_hit`), consumes every
  RBE_* effect (poison, disenchant, eat gold/item, sanity, …), applies
  protection-from-evil, shields, cut/stun, and prints "%s breaks free from
  hypnosis!" with per-method verbs. Also the C++ rebellion trigger check
  `randint(1000) < rlevel - (plev*2+skill)` is present, but the attack itself
  is not a port.
- [x] `melee1.cc:303-458`/`1439-1610` attack messages — Rust always prints
  `"The {} hits you. (N)"` (`game.rs:9344`) regardless of `method`; the
  per-method verb table ("claws you.", "gazes at you.", "begs you for
  money.", Mathilde's special moan lines, …) and the visibility gate on
  misses (`m_ptr->ml`, `melee1.cc:2696`) are missing.
- [x] `melee1.cc:101` `check_hit` — C++ subtracts `luck(-10,10)` from the
  attack roll (`randint(i - luck(-10,10))`); `monster_check_hit`
  (`game.rs:2442-2449`) has no luck term.
- [x] `melee1.cc:1242`/`1389` — the hostile melee roll uses the *race* depth
  (`monster_check_hit(..., def.depth, ...)`, `game.rs:9239`) instead of the
  monster's current level (`m_ptr->level` = `monster_level()`); a monster
  raised by `monster_set_level`/summons attacks at its base level. (The
  controlled-monster and companion paths correctly use `monster_level`,
  `game.rs:8734`.)
- [x] `melee1.cc:2712-2716` — the end-of-attack "The thief flees laughing!"
  + `teleport_away(m_idx, MAX_SIGHT*2+5)` blink is not implemented; Rust's
  EAT_GOLD/EAT_ITEM never set a blink flag.
- [x] `melee1.cc:146-221` `get_attack_power` — Rust's single `blow_power`
  (`game.rs:7864-7875`) follows `make_attack_normal`'s inline table
  (ABOMINATION=20) but C++ `monst_attack_monst` uses `get_attack_power`
  (ABOMINATION=30, and SANITY=60). Monster-vs-monster melee therefore rolls
  the wrong to-hit power in Rust.

### melee2.cc — AI, movement, spells

- [x] `melee2.cc:71-217` `mon_take_hit_mon` — Rust applies monster-vs-monster
  damage in `game.rs:9660-9668` (raw `tm.hp -= hit.dam`) and kills via
  `kill_monster`. Missing: (a) `m_ptr->csleep = 0` (victim never wakes on
  being hit by another monster); (b) `monster_gain_exp(s_idx, mexp*level/dive)`
  so the winning monster levels up (the whole monster-XP subsystem is absent,
  see below); (c) unique/neutral protection `m_ptr->hp = 1` for
  `RF_UNIQUE && status <= MSTATUS_NEUTRAL_P`; (d) death messages " is
  destroyed."/" is killed." selected by race flags/glyph.
- [x] `melee2.cc:220-278` `mon_handle_fear` — not ported at all. Rust never
  rolls fear when a monster takes damage: the only `m.fear` gains are the Fear
  spell (`game.rs:9735-9740`) and Armor of Fear (`game.rs:9431`). Missing:
  pain cancels fear (`randint(dam)`), the `hp <= 10%`/`dam >= hp` panic
  formula, and the visible "flees in terror!" message (C++ `melee2.cc:1814`).
  Flee movement itself exists (`game.rs:9052-9076`).
- [x] `melee2.cc:634-640` `bolt` / `melee2.cc:858-902` `breath`,
  `monst_breath_monst`, `monst_bolt_monst` — Rust monster spells resolve
  against a single target only. C++ `project()` passes over monsters, stops
  bolts at the first monster (which can save the player), and damages every
  monster in the ball radius/line. `monster_cast` (`game.rs:11332-11635`)
  only touches the player + floor items (`floor_hit`), and
  `companion_cast_monst` (`game.rs:11138-11164`) queues damage to exactly
  one `MmSpell` target, with no radius. A STUPID caster's bolt is therefore
  never blocked by an ally in Rust.
- [x] `melee2.cc:2322-2374` `curse_equipment` — Rust `item::curse_equipment`
  (`item.rs:2617-2645`) loops over all worn slots until one triggers, while
  C++ picks one random slot in `[INVEN_WIELD, INVEN_TOTAL)` (including empty
  slots, then returns). The `heavy_chance` second roll and `TR_HEAVY_CURSE`
  application for artifacts are missing (callers pass `chance + heavy` as a
  single chance, `game.rs:11719`), the blessed save uses `chance` instead of
  `randint(888)`, and the "There is a malignant black aura surrounding
  you..." message is absent.
- [x] `melee2.cc:2377-2431` `curse_equipment_dg` — not ported; no code path
  ever applies `DG_CURSE` at runtime (Rust only reads the flag,
  `game.rs:2217,8104`). Callers `xtra2.cc:3018` (possessor death) and
  `spells2.cc:4392` (DG curse spell) have no Rust equivalent adding
  `DG_CURSE`+`HEAVY_CURSE`.
- [x] `melee2.cc:2936-2999` `make_attack_spell` balls — wrong rlev
  multipliers in `monster_cast`'s ball table (`game.rs:11355-11475`):
  BA_ELEC uses `rmul=2` (C++ `randint(rlev*3/2)`), BA_FIRE `rmul=4`
  (C++ `rlev*7/2`), BA_COLD `rmul=2` (C++ `rlev*3/2`), BA_WATE `rmul=2`
  (C++ `rlev*5/2`); BA_POIS in `monst_spell_damage` adds `+rlev` although
  `melee2.cc:2974` is `damroll(12,2)`. The same table in the
  monster-vs-monster port (`monst_spell_damage`, `game.rs:10974-10980`) has
  the correct values, so the player-target path is the outlier.
- [x] `melee2.cc:3445-3472` SF_HASTE — C++ permanently raises `mspeed`
  (+10, or +2 up to base+20) with "starts moving faster" messages; Rust's
  player-target `monster_cast` only grants a one-shot `m.energy += 100`
  (`game.rs:11887-11890`), so the haste vanishes after the turn.
- [x] `melee2.cc:3557-3571` SF_BLINK/SF_TPORT — C++ always teleports the
  monster away; Rust refuses the blink when the *player* has NEXUS resistance
  (`game.rs:11891-11909`). The player's resistance should not affect the
  monster's own escape.
- [x] `melee2.cc:3834-3881` `mon_will_run` — not ported. There is no
  "strong player scares strong monsters" calculation
  (`p_val*m_mhp > m_val*p_mhp`, morale `(m_idx & 8)+25`, cdis>5 gate); Rust
  monsters only flee when an explicit fear effect was cast on them.
- [~] `melee2.cc:3916-3995` `get_fear_moves_aux` — not ported. The opt-in
  flow/smell swerve (`options->flow_by_sound`, `cave[].when/cost`,
  `MONSTER_FLOW_DEPTH`, `r_ptr->aaf`) has no Rust equivalent; `monster_turns`
  flees straight away from the player (`game.rs:9052-9076`).
- [x] `melee2.cc:4012-4080` `find_safety` — not ported. Fleeing monsters
  never choose a "not a clean shot" cell to duck behind walls; they always
  run directly away (same code path).
- [x] `melee2.cc:4091-4152` `find_hiding` — not ported. `RF_FRIENDS` +
  `RF_ANIMAL` packs never use hiding places to lure the player out of
  corridors (the `room < 8 && chp > 3/4*mhp` check is absent).
- [x] `melee2.cc:4156-4220` `find_corpse` — the possessor search in
  `game.rs:8870-8925` picks the *nearest* corpse and never checks
  `corpse_level <= monster_level` or line of sight; C++ requires both and
  picks the highest-level eligible corpse.
- [x] `melee2.cc:4351-4357` `RF_DEATH_ORB` — no LOS check in Rust
  (`DEATH_ORB` is used by 11 races, e.g. `r_info.txt`); C++ returns "no move"
  when the target is not in LOS. Rust `monster_turns` treats death orbs as
  ordinary movers.
- [x] `melee2.cc:4367-4428` `RF_FRIENDS` surround/hiding — Rust has no
  "monster groups try to surround the player" logic (fill an empty square
  among the 8 neighbours of the target, `(m_idx+i)&7` ordering). Groups in
  Rust are spawned with 1-2 buddies (see `place_monster_group`/
  `place_monster_aux` gaps below) and then each acts independently.
- [x] `melee2.cc:4276-4280` pet follow — C++ friends with
  `cdis > p_ptr->pet_follow_distance` move toward the player. In Rust a
  companion with no enemy target simply `continue`s
  (`let Some(...) = best else { continue; }`, `game.rs:8789`), so pets never
  follow the player around.
- [x] `melee2.cc:4660-5179` `monst_attack_monst` — Rust queues plain damage
  (`MmHit`, `game.rs:9660-9668`). Missing: per-effect GF conversion
  (POISON/DISEASE→GF_POIS, UN_*/ABOMINATION→GF_DISENCHANT,
  TERRIFY→GF_TURN_ALL, EXP_*→GF_NETHER, etc.), `damage *= 3` anti-monster
  bonus, aura fire/elec ripostes on `touched`, thief blink (`RBE_EAT_*` sets
  `blinked`), explode self-kill, and the no-target "You hear noise." message.
- [x] `melee2.cc:5188-5221` `player_invis` — Rust `player_invisible`
  (`game.rs:3213-3263`) omits the forced-visibility cases
  `mflag & MFLAG_QUEST` and `mflag & MFLAG_CONTROL` (`inv = 0`), and derives
  the monster level from the race depth instead of `m_ptr->level`.
- [x] `melee2.cc:5263-5339` bleeding/poisoned — C++ deals
  `1 + maxhp/50` (bleeding) and `poisoned/10` (poison) damage *every* turn and
  prints "is no longer bleeding/poisoned."; Rust decrements `cut`/`poison` by
  1 and deals 1 damage only every 5th turn (`game.rs:8596-8614`), with no
  recovery messages.
- [x] `melee2.cc:5392-5432` stun — C++ saving throw
  `rand_int(5000) <= level*level` fully cures; Rust just decrements
  (`game.rs:8625-8629`). Confusion/fear recovery in C++ uses
  `randint(level/10+1)` (`melee2.cc:5435-5466`, `5493-5526`); Rust
  decrements by 1 (`game.rs:9053-9054`, `9458-9460`) and never prints
  "is no longer confused." / "recovers its courage.".
- [x] `melee2.cc:5468-5491` gets_angry — not ported. A friendly monster
  (`status > NEUTRAL && < COMPANION`) must turn hostile when
  `p_ptr->aggravate` is active or when it is a non-pet unique (outside
  wizard), with "suddenly becomes hostile!" + `change_side`.
- [x] `melee2.cc:5533-5536` multiply gating — C++ only multiplies when
  `num_repro < MAX_REPRO`; Rust has no reproducer count (`no_breeders`
  models only the Sterilize aura). `ai_multiply`'s adjacency formula
  `k < 4 && (!k || !rand_int(k*MON_MULT_ADJ))` (`monster3.cc:125-161`) is
  approximated as `neighbours < 4 && 30%` (`game.rs:11963-11971`).
- [x] `melee2.cc:5679-5920` glyphs/runes — Rust bars **every** monster at
  `T_MINOR_GLYPH` (`game.rs:9500-9509`); the break chance is
  `randint(99)<level` instead of `randint(BREAK_MINOR_GLYPH) < level`, the
  "The rune explodes!" manaball (`fire_ball(GF_MANA, …, 2*((lev/2)+
  damroll(7,7)), 2)`) when the player stands on it is missing, and
  CAN_FLY/CAN_LEVITATE monsters should pass over it.
- [x] `melee2.cc:5737-5748` CAN_FLY over CAN_LEVITATE terrain — `monster_can_enter`
  (`game.rs:7950-7968`) only special-cases deep/shallow water and lava.
  Feature `CAN_LEVITATE` (dark pit `f_info.txt:87`, rubble `:49`, explosive
  rune `:64`) lets `RF_CAN_FLY` monsters cross in C++, but Rust's
  `map.walkable` returns false there (`NO_WALK`), so flyers are blocked.
- [x] `melee2.cc:5785-5791` webs — `RF_SPIDER` monsters may enter webs
  (`f_info.txt:16 F:WEB`); Rust has no web branch in the movement code at
  all (`T_WEB` is not `is_floor`, so `map.walkable` is false for everyone).
- [x] `melee2.cc:5793-5883` doors — Rust (`game.rs:9528-9540`) opens any
  closed non-secret door and bashes with a flat 50%. Missing: secret door
  opening (`FEAT_SECRET`), locked-door unlock chance
  `rand_int(m_ptr->hp/10) > door_power`, bash chance based on
  `m_ptr->hp/10`, and bash outcome `FEAT_BROKEN` 50% / `FEAT_OPEN`.
  `p_ptr->pet_open_doors` and the friend check are also absent.
- [x] `melee2.cc:6004-6049` KILL_BODY/MOVE_BODY — Rust kills/swaps any
  occupant (`game.rs:9481-9492`). Missing C++ gates: attacker
  `mexp > victim mexp`, victim not `RF_UNIQUE`, victim without
  `MFLAG_QUEST|MFLAG_QUEST2`, victim not a friend, not both friends, and
  the "leaving a wall" floor check for MOVE_BODY. As written Rust can
  silently despawn quest/unique monsters with no `kill_monster` bookkeeping
  (`silent_deaths` only logs "is crushed").
- [x] `melee2.cc:6143-6249` TAKE_ITEM/KILL_ITEM — Rust (`game.rs:9786-9817`)
  despawns the *whole* floor stack for KILL_ITEM, including artifacts, and
  refuses to take a stack if *any* member is an artifact (C++ processes
  each object, artifacts immune). Missing: the slaying-brand refusal
  (`TR_SLAY_*` matching the monster's race), and the pet
  `p_ptr->pet_pickup_items` option.
- [x] `melee2.cc:6301-6338` `summon_maint` — not ported. Partial summons
  (`MFLAG_PARTIAL`, e.g. Partial Totem) never charge
  `p_ptr->maintain_sum` (cost `(ml/cl-10000)/4`, floor `ml*19/990+80000/199`)
  and there is no "You lose control of %s." when the mana runs out.
- [x] `melee2.cc:6394-6512` `process_monsters` — Rust's energy loop
  (`game.rs:8575-8630`) has no range/sensing gate: C++ skips monsters with
  `cdis >= 100` and only lets a monster act when `cdis <= aaf`, LOS/aggravate
  within `MAX_SIGHT`, or flow-smell (`cave.cost < MONSTER_FLOW_DEPTH`) matches.
  `MFLAG_PARTIAL`, `MFLAG_CONTROL` and the `summon_maint` upkeep call are
  absent. Rust monsters with `energy >= 100` always act.
- [x] `melee2.cc:5599-5629` random movement — C++ gives a combined
  `RAND_50 && RAND_25` monster a flat 75% random-move chance (checked
  before the individual rolls); Rust ORs the two rolls
  (`game.rs:9461-9463`), yielding only ~62.5%.

### monster1.cc — terrain hooks

- [x] `monster1.cc:1537-1541` `monster_can_cross_terrain` — the
  "AQUATIC && !CAN_FLY → false on any non-water terrain" branch is missing
  from `monster_can_enter` (`game.rs:7950-7968`), so aquatic races can walk
  around on land in Rust.
- [x] `monster1.cc:1555-1574` `set_monster_aux_hook` — never ported.
  Spawn placement (`spawn_one_monster`, `game.rs:6370-6415`) only tests
  `map.walkable`, so `AURA_FIRE` monsters can be placed on shallow water and
  non-`IM_FIRE`/non-flying monsters on lava (all of deep/shallow water and
  lava are `FLOOR`, `f_info.txt:84-86,187`). The three predicates
  `monster_deep_water` / `monster_shallow_water` / `monster_lava`
  (`monster1.cc:1441-1471`) have no Rust caller.
- [x] `monster1.cc:1441-1471` `monster_deep_water`, `monster_shallow_water`,
  `monster_lava` — present in Rust only as `wild_monster_ok`/`monster_can_enter`
  behaviour for race selection and *movement*, not as spawn-restriction hooks
  (see previous item).

### monster2.cc — creation, leveling, allocation, summons, drops

- [x] `monster2.cc:61-155` `monster_exp` / `monster_check_experience` /
  `monster_gain_exp` / `monster_set_level` — the whole monster experience and
  level-up subsystem is absent. `Monster` (`game.rs:30-115`) has no `exp`
  field; `mon_level` is only a static override used for a few checks. No
  gains of hp (`hside` 80%), speed (40%, +1..2) AC (50%), or blow dice (30%)
  ever happen; `mon_take_hit_mon` cannot feed the killer and summoned
  monsters never get the `summon_specific_level` override
  (`monster2.cc:3251-3255`).
- [x] `monster2.cc:950-1112` `get_mon_num` — Rust's `pick_dungeon_monster`
  (`game.rs:6158-6191`) excludes **all** uniques (`!d.unique`, line 6172),
  so unlike C++ (`RF_UNIQUE` allowed until `cur_num >= max_num`,
  `monster2.cc:1005-1008`) no unique ever appears through random dungeon
  generation/allocation. Also missing: the two `NASTY_MON` level boosts
  (`monster2.cc:960-982`) and the "pick 2-3 candidates, keep the hardest"
  power boost (`monster2.cc:1057-1107`); the substitute is a flat
  `10/rarity` weight.
  **Open (2026-09):** `game::get_mon_num_from` (`game.rs:8520`) carries the
  100/rarity level table, the two NASTY_MON boosts and the 50%/10% harder
  re-rolls, and `pick_dungeon_monster` now admits unused uniques: `PlayerState.unique_seen`
  records every randomly allocated and every slain unique (`cur_num >= max_num` / `max_num = 0`),
  and the Sauron/Nazgul resurrect hook (q_main.cc:168) removes them again.  Non-uniques have no
  max_num limit, exactly like r_info's default 100.
- [~] `monster2.cc:1993-2507` `place_monster_one` — port is `spawn_flagged_held`
  (`game.rs:6568-6620`), which lacks: `RF_SPECIAL_GENE` rejection (42 races,
  including non-unique "Adventurer" and "Fire golem", can spawn randomly in
  Rust), `RF_NEVER_GENE`, unique `max_num`/`on_saved`/`cur_num` limits,
  `CAVE_FREE`/altar/glyph placement restrictions,
  `monster_can_cross_terrain` at the spot, random starting energy
  `rand_int(100)` (Rust always 0), non-unique `mspeed += rand_spread(0,
  extract_energy/10)`, and `ADJUST_LEVEL_*` -> `monster_set_level(min..max)`
  (Rust's `dungeon_monster_level` only shifts *race selection*, not the
  spawned monster's level/hp).
- [x] `monster2.cc:2454-2462` `RF_FORCE_SLEEP` — never consulted in Rust
  (flag is used by 396 `r_info` entries). C++ sets `MFLAG_NICE` +
  `repair_monsters` so the monster cannot act until the player has moved
  once; Rust monsters act as soon as their energy fills.
- [x] `monster2.cc:2518-2604` `place_monster_group` — C++ places
  `randint(13)` ± level difference monsters (max 32) breadth-first around
  the leader; Rust `FRIENDS` spawns only 1-2 identical companions
  (`game.rs:6389-6415`).
- [x] `monster2.cc:2615-2634` `place_monster_okay` — not ported. Escorts
  must match the leader's dungeon flag, glyph, and be lower level, non-unique,
  not identical; Rust's escort picker filters only `!unique` and
  `depth <= leader.depth+2` (`game.rs:6395-6404`).
- [x] `monster2.cc:2681-2739` `place_monster_aux` escorts — C++ tries up to
  50 escort placements using the restricted allocation table and may add
  groups for `FRIENDS`/`ESCORTS`; Rust spawns 1-2 `def.escort` monsters.
  `RF_ESCORTS` is not parsed at all (`data.rs:1309` only sets `escort` from
  `ESCORT`).
- [x] `monster2.cc:2923-3154` `summon_specific_okay` — Rust `summon_match`
  (`game.rs:12041-12064`) implements only ANT/SPIDER/HOUND/HYDRA/ANGEL/
  DEMON/UNDEAD/DRAGON/HI_*/WRAITH/UNIQUE/KIN/ANIMAL/THUNDERLORD/BUG/RNG.
  Missing types and filters: `SUMMON_GHOST`, `SUMMON_BIZARRE1-6`,
  `SUMMON_DAWN`, `SUMMON_ANIMAL_RANGER`, `SUMMON_PHANTOM`,
  `SUMMON_ELEMENTAL`, `SUMMON_BLUE_HORROR`, `SUMMON_MINE`, `SUMMON_HUMAN`,
  `SUMMON_SHADOWS`, `SUMMON_QUYLTHULG`, `SUMMON_NO_UNIQUES`,
  `SUMMON_HI_UNDEAD_NO_UNIQUES`, `SUMMON_HI_DRAGON_NO_UNIQUES` (player-side
  callers exist in `spells2.cc:4232-4279`, `cmd6.cc:886-954`,
  `cmd7.cc:2148,3130`, `powers.cc:829`; some have bespoke Rust clients such
  as `modal.rs:1489,3417`). Also the `monster_dungeon()` (WILD_ONLY) filter
  at `monster2.cc:2916` is absent from `summon_monsters`.
- [x] `monster2.cc:3183-3259` `summon_specific` — Rust (`game.rs:12069-12104`)
  picks uniformly among all candidates instead of via `get_mon_num`'s
  prob/level weighting and power-boost, ignores `summon_specific_level`,
  and places without checking glyphs/the Between (walkable only).
- [x] `monster2.cc:3452-3515` `multiply_monster` — Rust's MULTIPLY branch
  (`game.rs:11960-11986`) always clones the same race; C++ has a 3% mutation
  into a nastier same-glyph race, a `charm` (MSTATUS_PET) flag, a
  `SM_CLONED` marker, and the `no_breeds` ("It tries to breed but it
  fails!") message. The `MAX_REPRO` cap and `ai_multiply` formula are covered
  by the `process_monster` multiply-gating gap above.
- [x] `monster2.cc:3527-3644` `message_pain_hook` / `message_pain` — not
  ported. No damage-reaction messages by glyph class
  ("barely notices."/"flinches."/"yelps in pain."/"shrieks in agony."/…) and
  no "is unharmed." for 0-damage hits.
- [~] `monster2.cc:1164-1379` `monster_desc` — Rust's `hallucinated_name`
  (`game.rs:3716-3752`) covers hallucination only. The gender/pronoun modes
  (0x01/0x02/0x10/0x20/0x22/0x23), hidden-monster "something", indefinite
  "a kobold", and unique-name-without-article handling are absent; every
  message uses `"The {name}"` (uniques included).
- [x] `monster2.cc:1724-1730` `update_mon` WEIRD_MIND — Rust
  `monster_sensed` (`render.rs:374-385`) always senses a matching ESP race;
  the 10% per-turn roll for `RF_WEIRD_MIND` is missing. **FIXED (render.rs):
  `monster_sensed` takes a turn/entity seed and applies the 10% roll.**
- [x] `monster2.cc:455-546` `delete_monster_idx` — the Death-dungeon
  completion (`dungeon_type == DUNGEON_DEATH && !m_cnt` →
  "You overcome your fate, mortal!" + surface teleport) is not implemented
  in `kill_monster` (`game.rs:5503`); Rust's `DEATH_DUNGEON` handling only
  wipes stairs (`game.rs:5967`).
- [x] `monster2.cc:960`/`2879-2896` `alloc_monster` — Rust
  `alloc_wandering_monster` (`game.rs:6239-6291`) matches the 1/5000 horde
  roll and distance, but uses the unique-excluding picker described in the
  `get_mon_num` gap above and the flat `10/rarity` weighting instead of
  `get_mon_num`.
  **Open (2026-09):** `alloc_wandering_monster` draws through
  `get_mon_num_from` (level table / NASTY boosts / harder re-rolls) and marks the allocated race
  in `unique_seen` after a successful placement; horde leaders still reject uniques as in
  `alloc_horde` (monster2.cc:2805).

### monster3.cc — sides, possessor, companions

- [x] `monster3.cc:68-80` `is_enemy` — the "monsters hate breeders" rule
  (`spells & SF_MULTIPLY` and `num_repro > MAX_REPRO*2/3` and different
  glyph) is missing from `acquire_monster_target` (`game.rs:7986-8029`).
- [x] `monster3.cc:94-122` `change_side` — not ported. Used by
  `melee2.cc:5488` (gets_angry) and `spells2.cc:2631` (charm break); also
  feeds Yavanna piety. Rust has no way to flip friend↔enemy except the
  neutral/complex flags.
  **Open (2026-09):** done — `game::change_side` (game.rs) implements the FRIEND/PET→ENEMY,
  neutral flip and the COMPANION refusal, with the Yavanna `-4*level` grace for ANIMAL non-EVIL
  friends.  gets_angry/aggravate use it (`game.rs` upkeep), the GF get_angry list marks
  `GfMonsterEffect.angry` and rune blasts/clouds call it.  Thrown/missile/potion anger
  (cmd2.cc:2635/3073/3122) can adopt the same helper.
- [x] `monster3.cc:247-320` `ai_deincarnate` — not ported and not
  representable: `Monster` has no `possessor` field, `ai_possessor` just
  overwrites `m.def` (`game.rs:8887`), and `kill_monster` despawns the body.
  C++ (`xtra2.cc:3003-3010`) reverts a possessor whose hp drops below 0 to
  its previous race instead of killing it.
- [x] `monster3.cc:323-345` `can_create_companion` — Rust's cap check in
  `do_cmd_companion` (`modal.rs:15403-15414`) uses
  `if count > cap` instead of `count >= cap`, allowing one extra companion.
- [x] `monster3.cc:662-698` `do_cmd_companion` — Rust has no
  MSTATUS_PET vs MSTATUS_COMPANION distinction (`Monster.companion` covers
  both pets and companions, `game.rs:56-58`); the order at
  `modal.rs:15392-15423` therefore only checks `m.companion` (already true
  for pets) and sets `true` again — a no-op. The "agrees to follow you."
  state change and the `target` is a pet check have no effect; only the
  off-by-one cap (see previous item) is observable.
  **Open (2026-09):** the cap is fixed (`modal.rs:17279`, `count >= 1 +
  skill_scale(LORE,6)`), but the distinction is still missing: pets are
  `friendly=true, companion=false`, so the `if !m.companion` test
  (`modal.rs:17283`) refuses real pets and re-setting `companion=true` on an
  existing companion is a no-op.
  **SESSION (modal.rs/input.rs) 2026-09-16:** fixed — `Monster.pet` now marks
  MSTATUS_PET and `companion && !pet` MSTATUS_COMPANION.  `do_cmd_companion` checks the cap over
  `companion && !pet`, requires the target to be `m.pet`, clears `pet` and prints "<name> agrees to
  follow you." / "<name> is not your pet!"; the pet menu's two dismiss commands and the target
  orders follow the same split (see the cmd1.cc bullet in 01-commands).  MSTATUS_FRIEND has no
  third marker, so a few game.rs MSTATUS_FRIEND spawns count as companions.
  **GAME.RS SESSION 2026-09-16:** the field exists (serde default, saved in `MonsterSave`);
  `spawn_flagged_held` seeds `pet` from the caller plus the r_info PET rule, `spawn_companion`/
  `spawn_partial_pet`/`spawn_companion_hp` are pets, `spawn_hero_companion`/IMPRESED eggs are
  loyal companions (pet=false), Fire Golem is pet=false (spells3.cc:1268 MSTATUS_FRIEND) and the
  upkeep pet count uses `m.pet`.  `Wish` `WISH_COMPANION` and `summon_friendly` still pass only
  `companion` through `spawn_flagged`, so they become pets until modal.rs passes a pet flag.

## UNCERTAIN

- `monster2.cc:2914-3155` — whether the missing `SUMMON_*` types that are
  used by player powers (`SUMMON_BIZARRE1`, `SUMMON_MINE`, `SUMMON_DAWN`, …)
  are fully covered by their bespoke Rust clients (`modal.rs:1489,3417`,
  `modal.rs:12084`, `spell.rs:575`); I only audited the generic
  `summon_specific` path, which is incomplete.
- `monster2.cc:1993` — Rust deliberately moves monster loot generation from
  birth (`place_monster_one`) to death (`drop_loot`, known-done) and lazily
  rolls the satchel (`looted`); whether stealing from a not-yet-rolled
  monster is fully equivalent (`items`/`looted` handling in
  `input.rs`) is outside this group's files.
- `monster2.cc:1164` — some `monster_desc` modes (possessive, reflexive)
  may be re-worded in Rust messages rather than missing behaviour; only
  gender/hidden/definite forms are clearly absent.
- `melee2.cc:4491-4629` — Rust's greedy `(dx,dy)/(dx,0)/(0,dy)` direction
  order (`game.rs:9468-9471`) was not compared move-for-move against the
  C++ `get_moves` mm[] table; it may produce different corridor choices
  even where the same target is chosen.
- `melee2.cc:5679-5715` — the Rust `T_MINOR_GLYPH` handling may also cover
  the explosive-rune feature via a different terrain id; the
  396-entry `RF_FORCE_SLEEP` gap and the fly-over rule are certain.

## COVERAGE SUMMARY

> Post-session note: 55 of the gap checkboxes are now `[x]` and 18 remain
> `[ ]` (see DONE THIS SESSION / PARTIAL / NEEDS sections below).

- Total functions: 118 (6+33+22+42+15)
- `[x]` = 49, `[>]` = 38, `[ ]` = 20, `[~]` = 11
- `[~]`: 6 `roff_*` monster-recall renderers (UI), `compact_monsters*`/
  `wipe_m_list`/`m_pop` (save/backend internals), `dump_companions` (UI).
- Biggest clusters of gaps: monster XP/leveling (absent entirely),
  fear/damage panic + flee AI (`mon_handle_fear`, `mon_will_run`,
  `find_safety`, `find_hiding`), monster-vs-monster blow effects,
  blow side-effect numerics (EAT_*/EXP/TIME/LOSE_*/UN_POWER), unique and
  group/escort generation, and the player-target ball/bolt area semantics.

## DONE THIS SESSION

All changes are in `bevy/src/game.rs` only (the report text above records the
pre-session state; the checkboxes were flipped accordingly).

Monster experience / levels (monster2.cc):
- New `MonsterExtra` ECS component (kept out of `Monster`/`MonsterSave` so
  external literals in modal.rs/save.rs stay valid): `exp`, `nice`, `repro`.
- `monster_exp`, `monster_check_experience` (80% hit dice via rDef `hside`,
  40% `mspeed_mod` speed), `monster_gain_exp`, `monster_set_level` (cap 150).
  Spawns in game.rs initialise `exp = monster_exp(level)`.
- `mon_take_hit_mon` semantics in the deferred MmHit resolution: wake the
  victim, unique/neutral protection (`hp = 1`), "is destroyed."/" is killed."
  /silent death lines, killer XP (`mexp * victim level / killer level`) with
  full level-up, `mon_handle_fear` + "flees in terror!".
- Death-dungeon completion: last monster in `DEATH_DUNGEON` ⇒
  "You overcome your fate, mortal!" + `Goto::Surface`.

Player melee fidelity (melee1.cc make_attack_normal):
- Dodge roll (`dodge_chance - rlev*5/6`) and Eru's intervention
  (`ps.god == 1 && praying`, `grace - rlev*300` capped 50000).
- Hit roll uses the monster's current level (`monster_level`) and the
  player's `luck(-10,10)` (`monster_check_hit_luck`).
- Per-method attack messages (all RBM_* verbs, insults/moan/Show tables,
  Mathilde lines), visibility-gated misses, touched/cut/stun flags.
- RBE_UN_POWER: staff/wand only, non-artifact, `rlev*pval*number` heal.
- RBE_EAT_GOLD: dex/level save, the `gold > 5000` second formula,
  gold transferred to the monster satchel, full message set.
- RBE_EAT_ITEM: one item (`count -= 1`), a copy into `m.items` (wand charges
  split), "One of your/Your X was stolen!".
- RBE_EAT_FOOD: exactly one food item; RBE_EAT_LITE: non-artifact fuel/`pval`
  drain, message suppressed while blind.
- RBE_EXP_*: `damroll(n,6) + exp/100*MON_DRAIN_LIFE` with the distinct
  HOLD_LIFE block ("keep hold", "slipping away", /10) and drain messages.
- RBE_TIME: 3/4 stat reduction (min 3) with per-stat messages.
- RBE_DISEASE: `dec_stat(CON, randint(10), 1% permanent)` via a new
  `blow_dec_stat`/`dec_stat_value` (high stats lose the quarter/half-bonus
  formula); RBE_LOSE_* use the same amount-10 dec_stat, `desc_stat_neg`
  messages and sustain handling.
- RBE_EXPLODE kills the attacker only when the exploding blow itself hit;
  shield ripostes fire once per *touching* blow and only while alive.
- Thief blink: EAT_ITEM always / EAT_GOLD 2-in-3 set `blinked`; the monster
  teleports `MAX_SIGHT*2+5` and "The thief flees laughing!".

Monster AI (melee2.cc):
- `mon_will_run` (morale `(m_idx & 8)+25`, power comparison), `find_safety`
  (walls ducking) and `find_hiding` (animal packs luring). `get_fear_moves_aux`
  is still not ported: it is gated behind `flow_by_sound` (default false).
- Pet follow when no enemy target and `dist > 6`.
- RF_FRIENDS surround: BFS ring placement near the target; RF_DEATH_ORB
  requires LOS; breeders are hated (`num_repro > 66`, is_enemy).
- Bleeding `1 + maxhp/50` and poison `poisoned/10` every turn with recovery
  messages; stun save `rand_int(5000) <= level^2`; confusion/fear recover by
  `randint(level/10+1)` with messages; gets_angry (aggravate / unique
  friendlies) turns friends hostile.
- Random movement: RAND_50+RAND_25 rolls a flat 75%.
- Glyphs: FEAT_GLYPH rune of protection broken with `randint(550) < level`;
  explosive rune `randint(99) < level`, "The rune explodes!" manaball against
  the player standing on it; flyers cross CAN_LEVITATE/CAN_FLY terrain.
- Doors: secret door opening, locked-door unlock (`rand_int(hp/10) > power`),
  bash with door power, 50% FEAT_BROKEN / else FEAT_OPEN; pet door options
  (default off, command menu still missing).
- KILL_BODY/MOVE_BODY gates (mexp, uniques, quest, friends, wall-leaving) and
  per-object TAKE_ITEM/KILL_ITEM with artifact + slaying-brand immunity and
  pick-up/crush messages.
- `process_monsters` proximity gate (cdis >= 100, aaf, sight/aggravate,
  bleeding/poisoned; controlled bypass).
- `ai_multiply` formula (`k<4 && (!k || !rand_int(k*10))`) + 3% same-glyph
  mutation and the `no_breeds` failure message.
- `player_invis` forced visibility for quest/controlled monsters and the
  monster's own level.
- `monst_attack_monst`: damage ×3, per-effect GF conversion
  (POIS/UN_*/EXP_*/TIME/…), GF side effects, aura fire/elec ripostes on
  touched blows, SHATTER earthquake, EAT_* blink, explode self-kill,
  `message_pain` glyph-class reaction messages.

Monster creation / summons (monster2.cc, monster1.cc):
- `monster_can_enter`: AQUATIC land restriction, webs (spiders), CAN_FLY over
  CAN_LEVITATE/CAN_FLY, CAN_PASS for PASS_WALL/KILL_WALL.
- `monster_spawn_ok` implements `set_monster_aux_hook` (deep water AQUATIC,
  shallow water no AURA_FIRE, lava IM_FIRE/CAN_FLY and no AURA_COLD) and is
  applied to dungeon/wandering summon placements.
- `pick_dungeon_monster`: SPECIAL_GENE/NEVER_GENE refusals.
- Spawn details: random starting energy `rand_int(100)`, non-unique
  `mspeed` spread, FORCE_SLEEP → `nice` (gated until the player's turn),
  `MonsterExtra` initialisation.
- `place_monster_group` BFS (up to 32) and occupancy-aware escorts
  (`spawn_one_monster_occ`) with `place_monster_okay`-style filters, used by
  dungeon population, wandering monsters and summons.
- `summon_pool`/`summon_match`: GHOST, BIZARRE1-6, DAWN, ANIMAL_RANGER,
  PHANTOM, ELEMENTAL, BLUE_HORROR, MINE, HUMAN, SHADOWS, QUYLTHULG,
  NO_UNIQUES and HI_*_NO_UNIQUES; `monster_dungeon` WILD_ONLY filter;
  rarity-weighted `get_mon_num`-style selection.

GF / projections assigned to game.rs:
- `gf_player_knock`: full NEXUS table (teleport 200 / dragged to the attacker
  / save-or-level-teleport / save-or-corrupt), gravity respects
  NO_TELEPORT, shove unchanged.
- `gf_monster_effect`: GF_PSI/PSI_DRAIN (EMPTY_MIND immunity, resist /3,
  confuse/stun/fear/slow riders, undead/demon backlash), GF_RAISE (heal, no
  damage), GF_ATTACK marker, GF_UNBREATH (15% thick poison, undead/unliving
  immunity, no glyph shortcut), GF_POIS riders, GF_INERTIA/GRAVITY saves
  (`randint(100)` teleport save, `mspeed > 60` slow guard, `damroll` stun).
- Monster ball rlev: BA_ELEC/BA_COLD `3/2`, BA_FIRE `7/2`, BA_WATE `5/2`;
  `monst_spell_damage` BA_POIS no longer adds rlev.
- SF_HASTE permanently raises `mspeed_mod` (+10 then +2, base+10/+20 cap);
  SF_BLINK/SF_TPORT ignore the player's nexus resistance.

Map hooks assigned to game.rs:
- Terrain `E:` damage via `map::terrain_effect` every 10 game turns through
  `apply_gf`.
- `DF_EVOLVE` now calls `map::evolve_level` (noise pass included).
- `regen_monsters` (every 100 turns, maxhp/100 doubled with REGENERATE,
  skipped while bleeding/poisoned; symbiote pval2/pval3).
- `recharged_notice`: rods, pack activatables and worn items inscribed "!!".
- NO_BREATH/WATER_BREATH split using `magical_breath` (TR_MAGIC_BREATH,
  Air 50, Manwe grace while praying) with the original messages.
- `curse_equipment_dg` implemented and triggered by dying RF_DG_CURSE
  monsters.

Tests: `cargo test` 166 passed / 0 failed (run four times).

## PARTIAL / SKIPPED (reason)

- `curse_equipment` (item.rs) and `curse_equipment_dg`'s spell path
  (`modal.rs activate_dg_curse`): outside file ownership; the death-path DG
  curse is now wired in game.rs.
- `monster-vs-monster project()` area semantics (bolt stopped by allies,
  balls damaging every monster) and `summon_maint`/MFLAG_PARTIAL: need a
  monster query inside `monster_cast`/`process_monsters`; not attempted.
- `get_fear_moves_aux`: option `flow_by_sound` defaults false; no caller can
  enable it in the port.
- `carried_make_attack_normal` (symbiote rebellion): still the simplified
  summed-dice attack.
- `monster_desc` gender/hidden modes, `update_mon` WEIRD_MIND roll
  (render.rs), `ai_deincarnate` (no possessor field), companion menu items
  (modal.rs), `change_side` Yavanna piety: untouched, as documented above.
- `get_mon_num` unique allocation (`cur_num`/`max_num`) and the 2-3 candidate
  power boost: no per-race `cur_num`; uniques stay excluded from random
  allocation.
- `place_monster_one` ADJUST_LEVEL monster level and unique max_num: the
  allocation level is still only used for race selection.
- `summon_specific` power boost and `summon_specific_level` override: the
  spell-side callers use modal.rs' bespoke `spawn_friendly_level`.
- `multiply_monster` charm/SM_CLONED markers and the global MAX_REPRO count.
- `message_pain` 0-damage "is unharmed." is implemented but only game.rs
  damage paths call it; player attacks live in input.rs.

## NEEDS DATA FIELD / CROSS-FILE

- Monster per-instance AC and blow dice: `monster_check_experience` cannot
  grant the 50% AC / 30% blow-dice level-up gains without an instance `ac`
  and per-monster `blows` (current `MonsterDef` is shared). The hp and speed
  gains are implemented.
- Monster XP persistence: `save.rs::MonsterSave` does not carry the new
  `MonsterExtra` component, so XP/nice are reset when a level is revisited.
  A `save.rs` field (or serialising the component) is needed.
- `curse_equipment_dg` callers in `modal.rs` (DG curse spell) must call the
  new `game::curse_equipment_dg` equivalent to apply DG_CURSE runtime flags;
  `item::curse_equipment` itself still ignores heavy curses/blessed saves.
- `update_mon` WEIRD_MIND 10% sensing roll lives in render.rs
  (`monster_sensed`).
- `do_cmd_companion`/`can_create_companion` (modal.rs) off-by-one cap.
- ADJUST_LEVEL semantics for the *spawned monster's* level need a level
  parameter threaded through the game.rs spawn helpers (or a Monster field).

## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- `monster2.cc:1724-1730` WEIRD_MIND (render.rs): `monster_sensed` now takes a turn/entity seed and
  only returns telepathy for a `RF_WEIRD_MIND` monster on a 10% roll (rand_int(100) < 10), matching
  `update_mon`. `render.rs` monster light also follows `update_mon_lite` (MAX_SIGHT + 1, lit cells
  must already be in the player's view; a wall is skipped when the monster itself is out of LOS).
- The `curse_equipment_dg` spell path (`modal.rs::activate_dg_curse`) already calls
  `item::curse_equipment_ex(..., dg=true)`; no further change was needed.
- Remaining 03-monsters cross-file entries (Monster XP persistence in save.rs, ADJUST_LEVEL at spawn,
  monster-vs-monster project() areas, companion modal) are outside the four owned files.

## WIRED THIS SESSION (game.rs/map.rs/save.rs ownership)

- **Monster XP persistence**: verified `save.rs::MonsterSave.extra` already carries `MonsterExtra`
  (`from_monster_with_extra`/`to_extra`), so the earlier cross-file entry is closed.
- **Summon placement**: `game::summon_monsters` now uses `map::scatter_pos` with the original
  `d = i/15 + 1` distance progression, `cave_empty_bold`-style emptiness and glyph refusal
  (monster2.cc:3183 summon_specific).
- **Potion shatter vs monsters**: monster breaths/balls/bolts now project shattered inventory
  potions and floor `FloorBoom`s onto nearby monsters via `game::apply_gf_booms`; kills are queued
  into the shared `dead_monsters` pass so loot/XP are granted exactly once.
- **Companion persistence**: `level_transition` carries `m.companion` monsters across levels
  (`save_all_friends`/`replace_all_friends` semantics, generate.cc:6988/7023) and `place_friends`
  restores them with `get_pos_player(5)`.
- **MAX_REPRO**: the `num_repro > 66` breeder-hate count is already computed inline in
  `acquire_monster_target`; `MonsterExtra.repro` tracks the per-monster flag (no global counter
  exists in C++ beyond this count).
- Still open in game.rs: monster-vs-monster bolt/ball radius semantics in `monster_cast` (the
  existing single-target `MmSpell`/`MmHit` queues would need a radius field) and
  `summon_maint` (Summoner pet upkeep, needs the `maintain_sum` state and companion commands).
- `process_world_gods` (dungeon.cc:509): not implemented. It needs a `grace_delay` counter on
  `PlayerState` (owned by birth.rs) or a separate resource; left open.


## WIRED THIS SESSION (game.rs/game-adjacent modal.rs)

- **get_mon_num (monster2.cc:950)**: new `game::get_mon_num_from` implements the level table weighting
  (`100/rarity`, entries `level <= arg`), the two `NASTY_MON` (1/50) level boosts capped at +5 and the
  50%/10% "keep the harder monster" re-rolls. `pick_dungeon_monster`, `random_monster_of_level`,
  `roll_rand_monster` (q_rand) and `bounty_assign` (q_bounty) all draw through it. Unique `cur_num`/
  `max_num` bookkeeping is still absent, so randomised uniques remain excluded; ADJUST_LEVEL still only
  shifts race selection.
- **can_create_companion (monster3.cc:323)**: the modal.rs companion command now refuses at
  `count >= 1 + skill_scale(LORE,6)` (was `count > cap`, one extra companion). The MSTATUS_PET vs
  MSTATUS_COMPANION state distinction remains absent.
- **roll_rand_monster**: now filters the complete q_rand.cc predicate set and uses the level-weighted
  allocation instead of the uniform `depth <= rl+10` draw.

## DONE THIS SESSION (final, 2026-09-16)

All changes in `bevy/src/game.rs` / `input.rs` / `modal.rs` / `item.rs` / `save.rs`.

- `melee1.cc:226-1180` **carried_make_attack_normal**: new `game::carried_make_attack` runs the
  symbiote's own four blows: `get_attack_power` + `monster_check_hit_luck` (effectless blows
  auto-hit), protection-from-evil, the full per-method verb table (INSULT/MOAN/SHOW/…),
  `damroll`, the complete RBE_* switch via the shared `apply_blow_side_effects` (poison,
  disease, disenchant, eat gold/item/food/lite, exp/time/stat drains, sanity, abyss, …),
  cut/stun criticals and `player_hurt`. The rebellion trigger (`randint(1000) < rlev - (plev*2+skill)`)
  is unchanged. Only divergence: the port has no `symbiote_name()` naming table, so the
  attacker is always "your symbiote".
- `melee2.cc:634-640/858-902` **bolt/breath radius**: `GfBoom` gained `monsters_only`;
  breaths queue a PROJECT_KILL boom at the player of radius 3 (POWERFUL) / 2, every ball
  uses its C++ radius (NUKE/ACID/ELEC/FIRE/COLD/POIS/NETH 2, CHAO/WATE/MANA/DARK 4 —
  the Rust table's wrong `rmul` radii are gone), and bolts use a new `line_blocker`
  (PROJECT_STOP) so the first monster on the path is hit instead of the player. The
  monster radius test now uses `cave.cc distance()` = `pref_distance`, matching `project()`.
- `melee2.cc:2322-2374` **curse_equipment**: verified item.rs already has the random worn
  slot, the separate `heavy_chance` roll, `TR_HEAVY_CURSE` on artifacts, the `randint(888)`
  blessed save and the malignant-aura message; no change needed.
- `melee2.cc:3916-3995` **get_fear_moves_aux**: `[~]` — gated behind `options->flow_by_sound`
  (default false) and the port has no `cave.when/cost` flow field; unreachable.
- `melee2.cc:5533-5536` **multiply gating**: new `ai_multiply_monster` (exact
  `k < 4 && (!k || !rand_int(k*MON_MULT_ADJ))`, 18 scatter tries, no_breeds message) is now
  attempted per turn in `monster_turns` for SF_MULTIPLY breeders while the live breeder
  count is `< MAX_REPRO` (100); `monster_cast`'s "MULTIPLY" case is a no-op like
  make_attack_spell.
- `melee2.cc:6301-6338` **summon_maint**: `MonsterExtra.partial`, `PlayerState.maintain_sum`,
  the `(ml/cl-10000)/4` / `ml*19/990+80000/199` cost, the "You lose control of %s." loss and
  the dungeon.cc:2231 mana drain (with the dungeon.cc:4176 per-turn reset) are implemented;
  partial totems spawn as partial pets.
- `monster2.cc:1993-2507` **place_monster_one**: `[~]` — ADJUST_LEVEL (`dungeon_adjust_level`
  + per-spawn `monster_set_level`), random energy, mspeed spread, FORCE_SLEEP and the
  SPECIAL_GENE/NEVER_GENE allocation filters are in, but the port has no per-race
  `cur_num`/`max_num`/`on_saved` counters and does not thread the CAVE_FREE/altar/glyph
  placement vetoes through explicit vault/quest spawns (random summoners do check glyphs).
- `monster2.cc:2681-2739` **place_monster_aux escorts**: `place_monster_okay` fixed
  (same glyph, level <= leader, same WILD_ONLY class, non-unique, not identical — the old
  `glyph !=` test was inverted); RF_ESCORTS flag now triggers the escort/group path and
  FRIENDS escorts/ESCORTS leaders get their own `place_friend_group` ring.
- `monster2.cc:3183-3259` **summon_specific**: `summon_monsters` now draws through
  `get_mon_num_from` (100/rarity weights, NASTY_MON boosts, 50%/10% harder re-rolls).
- `monster2.cc:3452-3515` **multiply_monster**: 3% same-glyph mutation, `charm` spawns the
  copy as a pet, `SM_CLONED` is set by the new `spawn_clone` (TR_CLONE weapons/paths).
- `monster2.cc:1164-1379` **monster_desc**: `[~]` — the full mode table (pronouns,
  possessive/reflexive, indefinite "a kobold", unique-without-article, hidden "something",
  "your" for pets) is implemented as `game::monster_desc` and used by the player-melee
  attack messages, the kill/death messages and fear recovery; the remaining speech/cast/
  summon messages still use the old fixed `"The {name}"` form.
- `monster3.cc:247-320` **ai_deincarnate**: `Monster.possessor` (+ MonsterSave field),
  `deincarnate_monster` (restore race/hp/exp/reset state, "The soul of … deincarnates!"),
  set on corpse inhabitation and honoured on the player-melee, demon-blade, projectile and
  shared monster-death paths.

## FINAL RECONCILIATION

Reconciliation pass 2026-09-16 (fourth pass, item/game/data session).  The single `[>]`
bullet is now resolved:

`[x]` (1):
- `do_cmd_companion` — the `pet` field landed: `#[serde(default)] pub pet: bool` on
  `game::Monster` (mirrored in `save::MonsterSave`), `spawn_flagged_held` sets
  `pet = pet_param || (r_info PET && !companion)`, `spawn_companion`/`spawn_companion_hp`/
  `spawn_partial_pet` are pets, `spawn_hero_companion` and the IMPRESED egg promotion use
  `spawn_loyal_companion` (pet = false), `spawn_controlled` (Fire Golem) stays
  MSTATUS_FRIEND, and the mana-upkeep pet count reads `m.pet` (melee2.cc:6406).  The
  `modal.rs` session consumed it for `do_cmd_companion` (checks the LORE cap over
  `companion && !pet`, requires `m.pet`, clears `pet`, prints "agrees to follow you." /
  "is not your pet!"), the pet menu and `change_side`.  Note: `spawn_flagged(companion =
  true)` callers outside game.rs (Wish `WISH_COMPANION`, summon_friendly) still get
  `pet = true`; their `pet` value should be split at those call sites in modal.rs.

`[~]` (3, unchanged):
- `get_fear_moves_aux` — gated behind `options->flow_by_sound` (default false); no
  `cave.when/cost` flow field.
- `place_monster_one` — no per-race `cur_num`/`max_num`/`on_saved`; CAVE_FREE/altar/glyph
  vetoes not threaded through explicit vault/quest spawns.
- `monster_desc` — full mode table implemented, but speech/cast/summon messages still use
  the fixed `"The {name}"` form.

Session log (spell-table/game-side, 2026-09-16): the PET/COMPANION split was left to the
`modal.rs`/`item.rs` session because it needs a `Monster.pet` field added to every
`Monster { .. }` literal (one of them is in modal.rs).  `game.rs` is otherwise clean:
`change_side` already refuses `companion`, `place_friends` carries companions only, and
the unique bookkeeping is the `PlayerState.unique_seen` pass recorded above.  No
monster-recall data access lives in `render.rs` (the Shift+K pages are modal.rs).
