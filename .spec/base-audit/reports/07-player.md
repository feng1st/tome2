# player audit report

Scope: `src/xtra1.cc` (66), `src/xtra2.cc` (88), `src/skills.cc` (40),
`src/birth.cc` (33), `src/corrupt.cc` (16), `src/mimic.cc` (26),
`src/gods.cc` (10), `src/player_type.cc` (0), `src/util.cc` (98),
`src/variable.cc` (2) vs `bevy/src/{game,item,skill,birth,corrupt,mimic,
modal,input,render,hud,spell,data}.rs` and `bevy/assets/data/*.ron`.

Known-done context (player status plumbing, skills tree/abilities,
subraces, base-14 corruptions, mimicry, sanity, gods/grace/piety/boons,
birth race/subrace/class/spec/god/quest selection, object-flag effects in
`totals_for`, traps) was not re-reported wholesale; only concrete
mismatches inside those areas are listed below. Pure UI (`prt_*`, `fix_*`,
target/look pickers, `text_out`, message I/O, macros, file backends) is
marked `[~]`.

## DONE THIS SESSION (2026-09-16, game.rs/birth.rs/skill.rs/save.rs)

- **Statuses `blessed` / `holy` / `tim_deadly` / `tim_roots`** now exist on `PlayerState` with the
  `set_blessed`/`set_holy`/`set_tim_deadly`/`set_roots` helpers, activations/expiry messages, their
  calc_bonuses effects (+5 AC/+10 to_h, holy light, forced *GREAT* crits, roots AC/damage + stun clear)
  and save fields. `tim_precognition` already used `turn.precog_pending`.
- **xtra1.cc:3224-3244** — hero +12 / shero +24 / blessed +10 to-hit are now applied before
  `BTH_PLUS_ADJ` (×3); shero −10 AC, invuln +100 AC, blessed +5 AC, roots AC, hero-or-shero fear
  resistance (`player_ac`, `player_hit_chance`, TERRIFY/SCARE checks).
- **xtra2.cc set_stun/set_cut** — tier messages ("You have been stunned./heavily stunned./knocked
  out.", the seven cut tiers, "You are no longer …"), the head-blow INT/WIS drains and the scar CHR
  drain, via `set_*_full`; the legacy 2-arg wrappers remain for call sites in other files.
- **Status expiry messages** in `upkeep` (fast/slow/hero/shero/blessed/holy/tim_deadly/roots/protevil/
  oppose_*/strike/invis/wraith/project/reflect/absorb_soul/disrupt/invuln, …).
- **LITE cap** — `player_lite` (calc_torch: +2 temporary, +1 holy, hard cap 5) is used for the FOV
  radius in game.rs; render.rs now calls it too (WIRED THIS SESSION).
- **Monk** — `monk_heavy_armor`, `monk_empty_hands`, bare-body AC bonuses and the unencumbered speed
  bonus in `num_blows`/`player_ac`/`player_speed`.
- **forbid_non_blessed / Sorcery staves** — `wield_malus` subtracts 15 for Eru's edged weapons (unless
  a worn BLESSED item) and the Sorcery non-mage-staff malus in `player_hit_chance`.
- **Melkor** — always resists fire, praying faithful at grace > 15000 are immune (`apply_gf`), +30
  invisibility power while praying at grace > 5000 (`player_invisible`), `melkor_sacrifice` costs
  10 max HP and grants `wisdom_scale(4)` damage (`recalc_max_hp`, `melkor_sacrifice_damage`).
- **God piety** — `god_piety_on_kill` rewritten to xtra2.cc:3118-3170 (Eru/Manwe GOOD penalties,
  Melkor good/normal, Tulkas evil + praying/demon, Yavanna praying penalty).
- **Grace decay/gain** — `upkeep` now uses the wisdom_scale formulas per 100/300/400 turns with the
  elf/ent/praying modifiers (dungeon.cc:1464-1540).
- **Antimagic** — following a god with Antimagic > 0 abandons it ("You no longer believe.",
  skills.cc:476).
- **Experience** — kill exp is `mexp*monster_level/max_plv` with the 16-bit fractional carry
  (`exp_frac`, new `max_plv`); LEVELS items use `player_exp[elevel-1]*5/2` and cap at 50.
- **calc_hitpoints** — Melkor sacrifices, the Possessor body formula `(rhp+sroot+mhp)/3` and the
  CLASS_UNDEAD divisor added to `recalc_max_hp`.
- **Birth** — `roll_stats` (5+1d3+1d4+1d5, 42<sum<57 re-roll, adjusted stats clamped 3..40),
  `roll_gold` (randint(100)+300 minus stat deductions, min 100), `roll_player_hp` (pre-rolled 50-level
  array, 3/8..5/8 band) consumed by level-ups, `luck_base` (±5) captured, torches roll their own 3-7.
- **make_wish** — typed wishes parsed into `Wish::Object` / `Wish::Monster { def, ego, status }`
  (base object names + base/ego monster names, status prefixes). `device_wish` is now wired to the
  `Modal::Wish` text entry (WIRED THIS SESSION); `game.rs::melkor_sacrifice_damage`/`roots_damage_bonus`
  are consumed by `modal.rs::player_attack_monster`.
- **do_cmd_suicide** helper (bypasses Ring/Undead saves) for the retire path.
- **calc_mana** — a Possessor's mana now uses the borrowed body's spell frequency
  (`21 - 100/freq_spell`, xtra1.cc:1548).
- **drunk_takes_wine** — helper for the Give handler (Ale/Wine -> empty bottle).
- **LITE/intrinsics**: `luck_base`, `holy` hold-life/luck/light consumers in item.rs and the
  blessed/holy scroll/spell mappings in modal.rs are now wired (WIRED THIS SESSION);
  `drunk_takes_wine` still needs its input.rs Give call site (NEEDS OTHER FILE).

## GAPS

### A. Stat / bonus derivations (`calc_bonuses` family)

- [x] `xtra1.cc:2676 calc_bonuses` + `xtra1.cc:130 modify_stat_value` +
  `tables.cc:273-548 adj_*` — the entire `adj_*` stat-modifier table family
  is missing. Rust folds stats with `PlayerState::stat_bonus = (stat-10)/2`
  (`game.rs:1615`) for to-hit/to-damage/AC and uses direct linear stats
  (`birth.rs:266`, potions do `stat_base += 1`), whereas C++ computes
  `modify_stat_value(stat, add)` (18 → 18/10 → 18/20 steps, `xtra1.cc:130`)
  and then indexes `adj_dex_th/adj_str_th/adj_str_td/adj_dex_ta/adj_wis_sav/
  adj_str_dig/adj_con_mhp/...`. Example: DEX 16 in C++ is `adj_dex_th` +1 /
  `adj_str_th` all the way to 18 → +3; Rust `(16-10)/2 = +3`. The whole
  curve and the 18/xx folded values are different, so every derived
  to_h/to_d/AC/save/dig/sanity number is approximate.
- [x] `xtra1.cc:3787-3827 saving throw` — C++
  `skill_sav = 0 + adj_wis_sav[WIS] + scale(SPIRITUALITY,75) + tactic +
  (anti_magic→min 95)`, then `min 10 else +10` (`xtra1.cc:3924`).
  Rust `player_sav` (`game.rs:3266`) is
  `20 + 2*level + stat_bonus(WIS)*5 + tactic`: no WIS table, no
  Spirituality, no antimagic minimum. Every monster fear/paralysis/confuse
  save uses a different number.
- [x] `xtra1.cc:2480-2498 apply_flags` — numeric mismatches:
  `TR_STEALTH` in C++ adds `pval` (`xtra1.cc:2492`), Rust
  `item.rs:517` does `t.stealth += pval.max(1)` (a 0 or negative pval
  becomes +1); `TR_TUNNEL` C++ adds `pval * 20` (`xtra1.cc:2498`), Rust
  `item.rs:522` stores raw `pval`; `TR_INVIS` C++ adds `pval * 10`
  (`xtra1.cc:2514`), Rust stores only a boolean (`item.rs:530`).
  `TR_BLESSED` C++ only sets `bless_blade` (`xtra1.cc:2525`); Rust lists it
  as a damage slay (`item.rs:502`) and gives a self-documented extra ×2 vs
  EVIL (`item.rs:2433`). **FIXED (item.rs): stealth += pval, tunnel += pval*20,
  `invis_power` += pval*10 (bool set when positive), BLESSED removed from the
  slay flags and the ×2-vs-EVIL bonus. `input.rs::skill_dig` must drop its own
  `* 20` (NEEDS OTHER FILE).**
- [x] `skills.cc:1261 forbid_gloves` + `skills.cc:1274 forbid_non_blessed`
  — neither exists in Rust (grep `forbid_gloves|forbid_non_blessed` over
  `bevy/src` is empty; `SK_SORCERY`/spellbook gloves logic is unused).
  Missing effects: the `cumber_glove` mana penalty (`xtra1.cc:1579-1602`,
  “Your covered hands feel unsuitable for spellcasting.” + 3/4 mana), the
  `TR_SPELL_CONTAIN` exemption, the Eru priest −15/−15 edged-weapon penalty
  and `icky_wield` state (`xtra1.cc:3726-3741`), and the Sorcery
  non-mage-staff penalty (`xtra1.cc:3744-3772`). Rust wizards can wear
  gauntlets with no cost and Eru priests wield axes unpenalised.
  — RECONCILIATION (2026-09-16 second pass): `wield_icky` (`game.rs`) implements the full
  `icky_wield` predicate (Eru's non-blessed edge or Sorcery non-staff), `upkeep` stores it in
  `PlayerState.icky_wield` and `spell::fail_chance` adds the +25% (lua_bind.cc:158); the
  `cumber_glove` state + transition messages are now in `game::cumber_glove`/`upkeep`
  ("Your covered hands feel unsuitable for spellcasting.").
  — DONE (2026-09-16 third pass): the parallel session also wired the `to_d` half
  (`input.rs::damage_bonus -= game::wield_malus(...)`, xtra1.cc:3732/3752).
- [x] `xtra1.cc:2676 calc_bonuses` `SKILL_HAND` branch — `monk_heavy_armor`
  (`xtra1.cc:4206`) is missing: no armour-weight threshold
  `100 + HAND*4`, so no unencumbered monk speed `scale(HAND,5)`, no free
  action at skill ≥25, no half blows when heavy, and none of the
  bare-body AC bonuses (`xtra1.cc:3009-3042`). Rust `player_ac`
  (`game.rs:3653`) only has equipment/dex/AC_LEVEL/tactic.
  — RECONCILIATION (2026-09-16 second pass): the threshold, unencumbered speed, heavy half-blows, the
  SK_HAND ≥25 free action (`game::monk_free_act`) and the shield-slot bare-body AC term
  (`player_ac`) are all implemented in game.rs; what remains is that `item.rs`/`mimic.rs` free-act
  consumers still read `totals.free_act` only, so the monk free action is only honoured on the
  game.rs defense paths (which call `monk_free_act`).
  — DONE (2026-09-16 fourth pass, item/game session): `game::sanity_blast` now calls
  `item::has_free_act`, and `mimic.rs` has no free-act reader (the shapes only grant into
  `totals`, which `has_free_act` includes).  The only remaining bare `totals.free_act` readers are
  in `modal.rs` (`15198`/`21231`/`21371`, cross-file modal.rs session).
- [x] `xtra1.cc:2633 monk_empty_hands` — C++ only uses the barehand blow
  branch when no weapon is wielded; Rust `num_blows` (`game.rs:2703`)
  branches on `melee_style == SK_HAND` alone and `choose_melee`
  (`modal.rs:713-731`) never takes off wielded weapons (C++ `skills.cc:815-831`
  unequips them, or refuses if cursed). A monk holding a sword gets the
  barehand blow count and weapon damage simultaneously.
  — RECONCILIATION (2026-09-16 third pass): `num_blows` respects `monk_empty_hands` (`game.rs:3305`,
  `3290`), and `choose_melee` (modal.rs) now strips both weapon slots into the pack before switching,
  refusing with "Hmmm, your %s seems to be cursed." when a wielded weapon is cursed (skills.cc:777).
- [x] `xtra1.cc:3783-3813 skill limits` — Rust never applies the
  `skill_stl` +1 baseline/0..30 clamp, `skill_dig ≥ 1` clamp or the
  `anti_magic → skill_sav = 95` rule; `game.rs:8932` recomputes stealth
  from scratch instead.
- [x] `xtra1.cc:3386-3389` encumbrance/bloating speed penalty counts only
  `calc_total_weight`/`food` in Rust (`game.rs:2883-2892`) but not the
  `skill_stl`/`skill_sav` parts above; `player_speed` also hardcodes
  `+10 if fast` instead of C++ `p_ptr->speed_factor` (`xtra1.cc:3304-3307`),
  which differs for e.g. `cmd7.cc:656` (factor `plev/5`) and the potion
  “speed only adds +5 when already hasted” (`cmd6.cc:1908`).
- [x] `xtra1.cc:3224-3244` hero/sheero/blessed/invuln terms — C++ gives
  hero +12 to_h, shero +24 to_h and −10 AC, `invuln` +100 AC, `blessed`
  +5 AC/+10 to_h, and `resist_fear` for hero *or* shero
  (`xtra1.cc:3357`). Rust `player_hit_chance` (`game.rs:2616`) adds a flat
  +12 for hero and +12 for shero (C++ values are ×`BTH_PLUS_ADJ`=3 in the
  attack formula); the shero AC penalty, the invuln AC bonus and the
  hero/shero fear resistance (only `shero` is checked, e.g.
  `game.rs:10242`) are missing.
- [x] `xtra1.cc:4547 luck` — the `luck(min,max)` helper is not ported at
  all (grep `fn luck` in `bevy/src` is empty; `game.rs:6` documents “Luck
  is neutral (luck(x,y) = midpoint)”). Every consumer therefore uses a
  fixed midpoint: melee/monster to-hit rolls (`cmd1.cc:82/116/140/186`,
  `melee1.cc:101`), artifact/ego rarity rolls (`object2.cc:2073-2315`,
  `3733`, `3795`, `4491`) and scroll-of-Learning skill points
  (`cmd6.cc:2223`). `totals.luck` is only used for a flat `*2` hit bonus
  (`game.rs:2624`), so LUCK gear/race has almost no effect and object
  generation ignores luck entirely.

### B. Hit points / mana

- [x] `xtra1.cc:1529 calc_mana` — Rust never reproduces the formula.
  `birth.rs:281-285` starts at `(1+spell_bonus)*mana_mult/100` and
  `skill.rs:315` only adds `scale(MAGIC,200)*mana_mult/100`; C++ is
  `scale(MAGIC,200) + adj_mag_mana[max(INT,WIS)]*lev/4`, `+1` if nonzero,
  then `rmp->mana%`, then `cp->mana%` (`xtra1.cc:1566`), Eru grace
  (`xtra1.cc:1569-1576`), gloves (`1579`), `to_m/5` (`1605`) and armour
  encumbrance (`1626`). Missing tables/terms: `adj_mag_mana`
  (`tables.cc:89`), the class mana percentage (only a boolean `mana_user`
  is taken), the per-level INT/WIS term, `forbid_gloves`/`cumber_glove`.
  Rust's max mana is essentially level-independent apart from the Magic
  skill.
- [x] `xtra1.cc:1709 calc_hitpoints` — Rust only keeps `hp_rolls` + `hp_mod`
  (`game.rs:3914-3957`, `4043-4071`). Missing: Sorcery HP penalty
  `mhp -= mhp*scale(SORCERY,50)/100` (`xtra1.cc:1728`), Melkor
  `-10*melkor_sacrifice` (`1734`), hero `+10` / shero `+30` (`1742`),
  the Possessor body formula `(rhp + sroot(rhp)+mhp)/3` (`1750`) and the
  `CLASS_UNDEAD` divisor (`1763`). `hero`/`shero`/invuln AC/HP are also
  absent from `player_ac` (`game.rs:3653` misses `invuln +100`,
  `blessed +5`, `shero −10`).
- [x] `xtra1.cc:1833 calc_torch` — no `cur_lite > 5` cap: `render.rs:280`
  uses the raw total `totals.lite + tim_lite` as FOV radius, so stacking
  LITE items exceeds the original max radius 5 (`xtra1.cc:1871`). The
  `if cur_lite==0 && p_ptr->lite` fallback and `holy`/`sh_fire → lite`
  rules (`xtra1.cc:1867`, `3045`) are not replicated. **FIXED (render.rs):
  the FOV radius now goes through `game::player_lite` (tim +2, holy +1, cap 5).
  The `cur_lite == 0 && lite` fallback and `sh_fire → lite` remain.**
  — RECONCILIATION (2026-09-16 second pass): done — `game::player_lite` now iterates the worn items,
  skips burned-out FUEL_LITE lights, adds the non-lite LITE1/2/3 and `sh_fire` intrinsic fallback
  (1 when cur_lite == 0), and keeps the tim_lite/holy terms and the cap of 5.

### C. Gods

- [x] `xtra1.cc:2242 calc_gods` Melkor branch — missing `resist_fire`
  always (`xtra1.cc:2280`), `invis +30` at grace>5000 and
  `immune_fire` at grace>15000 while praying (`2275-2279`). No Melkor
  handling in `god_sync`/`totals_for` (grep `MELKOR` in `game.rs` only
  finds demon summoning/quest code).
- [x] `xtra1.cc:3137-3144` Melkor sacrifice — `wisdom_scale(4) *
  melkor_sacrifice` added to `to_d`/`dis_to_d` is absent; Rust has no
  `melkor_sacrifice` field (`wisdom_scale` itself is also absent,
  `gods.cc:155`).
- [x] `cmd2.cc:3758-3855 do_cmd_sacrifice` — Rust's temple menu only
  supports “100 gp → +50 grace” (`modal.rs:17489-17502`). Missing: the
  Melkor HP sacrifice (10 HP → `wisdom_scale(6)*300` piety or +1
  `melkor_sacrifice`, `cmd2.cc:3790-3815`), corpse/book sacrifices
  (`2*monster_level`, `2*levels_in_book`, `cmd2.cc:3827-3847`), and the
  C++ altar conversion starts grace at **−200**, not +100
  (`cmd2.cc:3783-3785`).
- [x] `xtra2.cc:3064-3170` kill piety — Rust `god_piety_on_kill`
  (`game.rs:6921`) implements its own table: Eru gains `2*lvl` for UNDEAD
  kills and nothing for GOOD kills, whereas C++ gives Eru `−7*mlev` and
  Manwe `−10*mlev` for GOOD and Melkor `+3*mlev` (`xtra2.cc:3118-3128`);
  Tulkas gains on any EVIL kill (`max(2,inc)/2`, doubled while praying,
  plus `inc` for demons, `xtra2.cc:3130-3151`) but Rust only gives
  `2*lvl` for DEMON/UNDEAD; Yavanna's praying penalty
  (`−max(1,lvl/2)`, animals `×3`, `xtra2.cc:3161-3170`) is missing.
- [x] `dungeon.cc:1464-1540` grace decay/gain — Rust `game.rs:7820-7836`
  uses fixed periods (Eru +1/100 turns always; praying gods decay every
  15 turns) instead of C++ `wisdom_scale`-scaled amounts per 300 turns
  (Yavanna 400), the `!did_nothing && !wild_mode && dun_level` gates, the
  elf race modifiers, Tulkas-only-while-praying and the Eru-only-while-not-
  praying rule. `p_ptr->did_nothing` has no Rust equivalent at all.
- [x] `skills.cc:476-482 recalc_skills` — when Antimagic skill becomes > 0
  while following a god, C++ prints “You no longer believe.” and calls
  `abandon_god(GOD_ALL)`. Rust `increase_skill`/`sync_magic_mana` never
  enforce this (only the temple `follow_god` path checks antimagic).
- [~] `gods.cc:83-111 follow_god` — the Melkor Udun reveal message
  (“You feel the dark powers of Melkor in you…”) is missing, though the
  skill is unhidden (`skill.rs:175`).
- [x] `spells3.cc:1034-4115` god spell lists — three base god spells are
  absent from the Rust spell data and `GOD_SPELLS` (`spell.rs:236-250`):
  **Manwe’s Avatar** (`spells3.cc:1841`, mimic Maia for
  `get_level_s(MANWE_AVATAR,20)+d10`), **Yavanna’s Tree Roots**
  (`spells3.cc:3116`, the only base setter of `set_roots`), and
  **Yavanna’s Uproot** (`spells3.cc:3175`, tree → friendly Ent of
  `uproot_mlevel`). The whole `tim_roots` state (`xtra2.cc:222`, +AC and
  melee damage for its duration, `xtra1.cc:3254-3260`, `set_stun(0)` each
  tick) has no Rust equivalent. In addition several ported rows are
  simplified/mis-mapped: Lay of Protection is `kind:"protevil"`
  (`spells.ron:160`) but C++ fires a `GF_MAKE_GLYPH` ball of radius
  `1+get_level(ERU_PROT,2)` (`spells3.cc:1081`); Wind Shield is
  `kind:"resist"` (`spells.ron:162`) instead of protevil + tiered
  `set_shield` (`spells3.cc:1793`); See the Music / Listen to the Music /
  Manwe’s Blessing lose their skill-tier thresholds
  (`spells3.cc:1034-2100`, `1859`).

### D. Experience / object leveling

- [x] `xtra2.cc:1894-1918 check_experience` + `tables.cc:1002 player_exp[]`
  — the 50-entry experience table is not ported. Rust `exp_needed_at`
  (`game.rs:3963`) uses `10*level*(level+1)*exp_factor/100`; for level 1
  the threshold is 20 instead of `player_exp[0]=10`, level 10 is 1100 vs
  500, level 49 is 24500 vs 4,500,000. The whole progression curve is
  wrong. `PY_MAX_EXP`/`max_exp` (drain recovery `max_exp += amount/5`,
  `xtra2.cc:2038`) are also absent (`PlayerState` has no `max_exp`).
- [x] `xtra2.cc:3064-3089 mon_take_hit` experience — Rust `kill_monster`
  grants `def.exp.max(def.depth)` (`game.rs:5540`), while C++ computes
  `(mexp * m_ptr->level) / max_plv` with fractional carry
  (`xtra2.cc:3070-3085`), and for LEVELS weapons
  `(mexp*level)/(max_plv*2)` (`xtra2.cc:3101`). Rust's LEVELS growth uses
  `def.exp*mon_level/(2*player_level)` and an approximated requirement
  `5*10*l*(l+1)/2` (`game.rs:5547-5564`) instead of
  `player_exp[elevel-1]*5/2` (`check_experience_obj`, `xtra2.cc:1989`).
- [x] `xtra1.cc:1353-1486 calc_powers` — power gain/lose notifications are
  gone: C++ prints `power->gain_text`/`lose_text` on the sorted set
  difference (`xtra1.cc:1442-1483`, e.g. “You can turn into a Balrog at
  will.”/“You no longer feel the fire of Udun in you.”). Rust
  `available_powers` (`game.rs:3521`) is computed on demand and `POWERS`
  (`game.rs:3458+`) stores no such texts.

### E. Status effects (`xtra2.cc set_*`)

- [x] `xtra2.cc:1284 set_stun` + `xtra2.cc:1445 set_cut` — Rust `set_stun`/
  `set_cut` (`game.rs:3640-3650`) only clamp the counter. Missing:
  the tier-change messages (“You have been stunned./heavily stunned./
  knocked out.”, “You are no longer stunned.”, graze/light/bad/nasty/
  severe/deep/mortal cut, “You are no longer bleeding.”), the head-blow
  INT/WIS drain (`xtra2.cc:1365-1394`) and the scar CHR drain
  (`randint(1000)<v || randint(16)==1` → “You have been horribly
  scarred.”, `xtra2.cc:1597-1605`). These are functional (permanent stat
  loss) as well as message gaps. `set_stun(0)` is called every
  calc-bonuses tick while Roots is active in C++ (`xtra1.cc:3256`) — also
  missing.
- [x] `xtra2.cc:147-1470` status notice semantics — all reachable activation/expiry messages are now emitted
  (setters in `game.rs` for blessed/deadly/roots/poison/thunder; `upkeep` expiry lines incl. fast/slow/hero/
  shero/protevil/oppose_*/poison/paralyze/image/esp/wraith/regen; activation lines at every modal.rs call site:
  invis/infra (`Your eyes...`), wraith (`You feel less material.`), prob_travel (`You feel instable.`),
  absorb_soul (`You start absorbing...`), strike (`You feel very accurate.`), project (guarded).  The only
  strings without a reachable call site are the `set_tim_breath` pair ("Air seems to fill your lungs…" /
  "You need to breathe again."): `tim_water_breath`/`tim_magic_breath` have no writer anywhere in base
  (grep `src/*.cc`: only dungeon.cc:1695-1701 decrement and loadsave.cc serialisation), so they are dead
  timers.  `SHAPECHANGER` probe code is commented out upstream, `FRIEND` is recall prose only.
  — SESSION (modal.rs/input.rs/hud.rs) 2026-09-16: the modal.rs activation lines were added where
  the fields transition 0 -> positive: `set_absorb_soul` ("You start absorbing the souls of your
  foes."), `set_tim_invis` ("Your eyes feel very sensitive!"), `set_tim_infra` ("Your eyes begin
  tingling."), `set_prob_travel` ("You feel instable.") and `set_strike` ("You feel very
  accurate."); the `tim_project`/`tim_reflect`/`tim_regen`/`tim_wraith` effect arms already print
  their own lines.  `set_tim_breath` is dead data in base (nothing assigns
  `tim_water_breath`/`tim_magic_breath`; grep of src/*.cc shows only the expiry calls), so its
  missing lines are unreachable.  What stays open: `set_slow`'s player activation (the GF_OLD_SLOW
  player branch lives in game.rs, cross-file) and the exact C++ wording for the arms that print
  school-flavoured text instead of the set_* text.
- [x] `xtra2.cc:869-995 set_holy` + `xtra2.cc:825 set_blessed` — **these
  statuses do not exist in Rust** (`PlayerState` has no `blessed`/`holy`).
  C++ effects: `blessed` = +5 AC, +10 to_h (`xtra1.cc:3225-3231`);
  `holy` = `hold_life`, luck +5, light +1 (`xtra1.cc:3218-3222`).
  Consequences: scrolls Blessing/Holy Chant/Holy Prayer are remapped to
  hero/shero/protevil (`modal.rs:11326-11336`) with the wrong values
  (C++ `randint(12)+6 / randint(24)+12 / randint(48)+24`), Manwe’s Blessing
  is `kind:"shero"` only (`spells.ron:161`) instead of
  blessed+afraid 0+lite 0+hero at level≥10+shero≥20+holy≥30
  (`spells3.cc:1859-1881`), Mediator/Berserk activations lose
  `set_blessed` (`cmd6.cc:4783`, `5880`) and Dispel Magic does not clear
  it (`spells3.cc:2246`). **FIXED (modal.rs/item.rs): scrolls call
  `set_blessed` with randint(12/24/48)+6/12/24; Manwe's Blessing sets
  blessed + hero/shero/holy at 10/20/30 (and clears tim_lite, not blind);
  Mediator sets blessed + all five opposes; Dispel Magic clears blessed.
  `item.rs::totals_for` grants holy hold_life and +5 luck; the +1 light is in
  `game::player_lite`.**
- [x] `xtra2.cc:350 set_tim_deadly` — no `tim_deadly` field/consumer.
  Tulkas’ Divine Aim is `kind:"hero"` (`spells.ron:164`) instead of
  `set_strike(dur)` plus `set_tim_deadly(dur)` at scale≥20
  (`spells3.cc:2659-2666`); C++ `critical_norm` then forces a
  “*GREAT* hit!” (`cmd1.cc:189-195`). The deadliness/guaranteed-crit
  mechanic is missing. **FIXED (modal.rs): tulkas_divine_aim calls
  `game::set_tim_deadly` at effective level 20+; `game::critical_norm`
  already forces *GREAT* hits while `tim_deadly > 0`.**
- [x] `xtra2.cc:1647 drop_from_wild` — C++ force-exits small-scale
  wilderness when food crosses Faint/Weak (`set_food` → `drop_from_wild`,
  `xtra2.cc:1810-1816`). Rust `set_food` logic (`game.rs:7552-7558`) has
  no FOOD_FAINT (500) tier, no faint message, and never drops out of
  `wild_mode`.

### F. Birth / character creation

- [x] `birth.cc:326 get_stats` — C++ rolls each stat as
  `5 + 1d3+1d4+1d5` with the `42 < sum < 57` re-roll loop and applies
  race/class bonuses via `modify_stat_value`; Rust `birth.rs:266` sets
  `stats = 10 + race + subrace + class` deterministically. There is no
  stat rolling, no `stat_match`/`auto_round`, no maximise option and no
  re-roll (`BirthStep` has no stat/roll stage).
- [x] `birth.cc:2102 player_birth_aux_point` — the point-based creation
  (10-18 stats, `adjust_stat` cost table, `birth.cc:271`) is not ported at
  all.
  — RECONCILIATION (2026-09-16 second pass): done — `BirthStep::Stats` (point-buy mode) edits the
  10..18 base stats with the 48-point budget, folds the race/subrace/class bonuses, and passes the
  resulting array plus `point_buy_gold(cost)` into `make_player_spec_with_stats`.
- [x] `birth.cc:2250 player_birth_aux_auto` — the autoroller
  (`auto_roll`, `p_ptr->maximize`, 1,000,000-round cap, “Auto-roll %6ld”
  display) is not ported.
  — RECONCILIATION (2026-09-16 second pass): done — `BirthStep::Stats` (autoroll mode) shows the
  rolled stats and per-stat minimums, `roll_auto_stats` runs `auto_roll` with the 1,000,000-round
  cap and displays the round count.
- [x] `birth.cc:505 get_money` — Rust hardcodes `gold: 200`
  (`birth.rs:315`); C++ is `randint(100)+300` minus stat-based deductions
  (`stat_use ≥ 18/50 → −300`, `≥18/20 → −200`, `>18 → −150`, else
  `(stat-8)*10`), minimum 100 (`birth.cc:505-525`).
- [x] `birth.cc:391-393` luck — `luck_base = race.luck + subrace.luck +
  rand_range(-5,5)`; Rust only sums the racial luck (`item.rs:693/698`),
  the random ±5 is missing. **FIXED (item.rs): `totals_for` starts from
  `ps.luck_base` (birth value) and the holy +5; the birth test now compares
  against `ps.luck_base`.**
- [x] `birth.cc:399 roll_player_hp` — C++ pre-rolls the whole 50-level HP
  array with the cumulative total constrained to
  `[50*(hd-1)*3/8+50, 50*(hd-1)*5/8+50]`; Rust starts `hp_rolls=[hitdie]`
  (`birth.rs:308`) and rolls one unconstrained die per level in
  `check_experience` (`game.rs:3928-3933`). The distribution/HP curve
  differs and there is no birth time reroll.
- [x] `birth.cc:742-978 player_outfit` — minor: Rust reuses the food
  `qty` (3-7) for torches (`birth.rs:530-543`) where C++ rolls a
  separate `rand_range(3,7)` for the torch count (`birth.cc:964-971`).

### G. Corruption / mimicry / possession

- [x] `corrupt.cc:45-112` dynamic vampire subrace — C++
  `player_gain_vampire_teeth` copies the current subrace into
  `SUBRACE_SAVE`, adds `PR_VAMPIRE|PR_UNDEAD|NO_SUBRACE_CHANGE` and
  `PWR_VAMPIRISM`; `player_gain_vampire_strength` edits that record and
  calls `do_rebirth()` (`switch_subrace` + `do_rebirth`,
  `xtra2.cc:5342/5366`: expfact + hitdie recompute, `do_cmd_rerate`,
  `check_experience`). Rust approximates with a fixed “Vampire” subrace
  plus corruption flags (`birth.rs:863-879`, `game.rs:3598-3603`) and just
  adds +1 max_hp/+100 exp (`corrupt.rs:232-238`). Gaining the vampire set
  mid-game via the corruption cannot rename/flag the subrace and does not
  re-rate HP or recompute the experience factor.
  — RECONCILIATION (2026-09-16 second pass): `corrupt::gain_full` now calls `game::do_rebirth`
  after the manual +1 max HP/+100 exp when the vampire strength corruption is acquired, and the
  birth vampire chain (birth.rs) uses it; subrace display-name/flags remain approximated
  (the port's fixed "Vampire" record).
- [x] `mimic.cc:51-73 mouse_calc` — C++ `p_ptr->to_d = p_ptr->to_d / 5`
  runs **before** the equipment scan (mimic is folded at
  `xtra1.cc:2859`, equipment at `2942`), i.e. it divides 0 and mouse keeps
  full gear to-damage; Rust `t.to_d /= 5` (`mimic.rs:227`) runs after the
  equipment totals and removes 4/5 of the player’s gear damage.
- [x] `xtra1.cc:1969-2134 calc_body/calc_body_bonus` — the Possessor “body”
  system is a stub: Rust `possessed` only changes melee blows/damage soak
  (`input.rs:2160`, `game.rs:6730`) and suppresses race flags
  (`item.rs:686`) but never applies the body monster’s AC, speed, or
  `RF_*` intrinsic flags (`xtra1.cc:2108-2133`), the body HP formula
  (`calc_hitpoints`, `xtra1.cc:1750-1759`; Rust rolls `def.hp` once in
  `modal.rs:10660`), `calc_body` body parts and the forced unequip of
  slots the body lacks (`xtra1.cc:2078-2086`), or the `disembodied`
  wraith/AC special cases.
- [x] `xtra1.cc:1930 calc_wield_monster` — Rust symbiote flags
  (`item.rs:652-667`) differ from C++: `RF_CAN_FLY` in C++ grants
  **ffall** only (`xtra1.cc:1951-1954`) while Rust grants fly+feather, and
  `RF_AQUATIC` in C++ grants **water_breath** (`1956-1959`) while Rust
  sets `magic_breath` (also water).

### H. Wishes / corpses / coins (player-scope xtra2 helpers)

- [x] `xtra2.cc:4996-5320 test_object_wish/clean_wish_name/make_wish` — the
  entire wish system is missing (grep `make_wish|Wish for what` in
  `bevy/src` is empty). It is reachable in base through the staff of Wish
  (`k_info.txt:2992`, `I:55:20:-1:SPELL=Wish` →
  `spells3.cc:3402 device_wish`), but Rust maps the spell to `kind:"acquirement"`
  (`spells.ron:88`, `convert_data.py:1519`), so a staff of Wish produces a
  random object/monster wish instead of the typed wish (objects, or
  `enemy/neutral/friendly/pet/companion <monster>`, with
  `mega`/`SPECIAL_GENE`/`NEVER_GENE`/unique filters). **FIXED (modal.rs):
  `use_device` intercepts the Wish row and opens `Modal::Wish`, a text
  entry that calls `game::make_wish`; only a fulfilled wish discharges the
  staff, the object lands via `drop_near`, monsters scatter within 5 with
  the requested status.**
- [x] `xtra2.cc:2070 get_coin_type` — not ported. C++ makes “Creeping
  copper/silver/gold/mithril/adamantite coins” (r_info 85/117/195/239/423,
  live in `monsters.ron`) drop only the matching coin type when killed
  (`xtra2.cc:2410` `force_coin`); Rust `drop_loot`/`monster_carried_treasure`
  rolls generic gold.
- [x] `xtra2.cc:2100 place_corpse` — Rust `kill_monster`
  (`game.rs:5811-5818`) drops a corpse/skeleton with `note=def_idx`,
  decay timer and weight, but misses C++ `pval3` corpse HP
  (`(maxroll + player mhp)/2 - randint/3`, used by resurrection/raising),
  `name1=201` for unique corpses, and the `OBJ_FOUND_MONSTER` etc. found
  bookkeeping (`xtra2.cc:2113-2167`).
  — RECONCILIATION (2026-09-16 third pass): unique corpses are marked, `Item` carries
  `found`/`found_aux1..4`, `item::init_corpse` sets `OBJ_FOUND_MONSTER` + `found_aux1`, and both
  game.rs corpse drops set `found_aux3` (dungeon) and `found_aux4` (`level_or_feat`).  Only
  `found_aux2` (monster ego) stays 0 because the port has no per-monster ego field.

### I. Skills screen / misc

- [x] `skills.cc:365-427 increase_skill/decrease_skill` — Rust commits
  point-by-point and only applies `increases` to already-known targets by
  `mod * pct/100`; the C++ session model (`recalc_skills_theory`,
  `skills.cc:498-558`) computes `base_val + base_mod*invest + bonus` where
  `bonus` accumulates the percentage boosts, and raises the invested
  skill’s own bonus; exclusions give back the session skill points
  (`skills.cc:526-536`) which Rust never refunds (`skill.rs:405-409`).
  — RECONCILIATION (2026-09-16 second pass): done — `PlayerState.skill_invest` tracks the session
  investments, exclusion refunds them (`skill.rs`), `decrease_skill` only takes back session points,
  and `skill::clear_skill_session` releases the session.  modal.rs's skill screen should call
  `clear_skill_session` when it closes to commit.
- [x] `skills.cc:1382-1543 do_get_new_skill` — the Lost Sword reward
  ignores the mutual-exclusion confirmation (“This skill is mutually
  exclusive with at least X, continue?”, `skills.cc:1503-1521`) and
  silently wipes opposing skills (`skill.rs:650-660`).

## NOT GAPS (recorded `[~]`)

- `xtra2.cc:5327 corrupt_corrupted` — 45% lose / 55% gain random
  corruption is not ported, but its only base callers are gated on
  `PR_CORRUPT` (`xtra2.cc:1971-1976`, `2021-2027`) which no base race/mod
  uses → data-dead.
- `xtra2.cc:2202 monster_death_gods` / `dungeon.cc:490-700
  process_world_gods` — Aule/Ulmo/Mandos/Varda branches only → Theme.

## UNCERTAIN

- `xtra1.cc:2466 apply_flags` `IMPACT`/`VORPAL` consumers exist
  (`input.rs:2317/2477`) but are keyed to `EquipTotals.slays`; verifying
  exact parity with `tot_dam_aux` is in the objects/commands audits.
- `xtra1.cc:1385 calc_powers`: C++ mutates a persistent sorted set and
  notifies between calls; Rust recomputes each menu open. Behaviour is
  equivalent for granting, but any consumer that expects one-time gain
  effects (`p_ptr->powers` polling) has no analogue.
- `birth.cc:1295 player_birth_aux_ask`: the Rust `BirthFlow` skips
  gender/options (`linear_stats`, `maximize`, quick-start/savefile load)
  since all are UI/option state; only the stat rolling itself is reported
  above.
- `gods.cc:155 wisdom_scale`: unused in Rust; only the missing
  Melkor-sacrifice/Theme-god features would need it.
- `xtra2.cc:4986 set_grace` clamps: C++ `set_grace` just assigns and lets
  `inc_piety` do overflow recovery to ±300000; Rust clamps inline in
  `god_piety_on_kill`/`god_sync` — same effective range, different
  structure. Not reported as a gap.
- `mimic.cc:705 calc_mimic` return value (extra blows added to
  `extra_blows` before the weapon scan) is reproduced via `t.blows` in
  `apply_mimic_totals`; ordering vs equipment `num_blows` clamp
  (`game.rs:2700 .clamp(0,2)`) may clamp Balrog+Maia blows differently
  (C++ `extra_blows` is uncapped from items). Low impact, needs a
  dedicated diff.

## COVERAGE SUMMARY

- 总函数 379；[x]=109, [>]=85, [ ]=12, [~]=173 (per-function marks in
  `inventory/07-player.md`; documented `[~]` = UI/front-end/save backend/
  Theme-module/data-dead).
_2026-09-16 session: +22 boxes resolved ([x]/[>]) — see DONE THIS SESSION above._

## WIRED THIS SESSION (modal.rs/spell.rs/item.rs/render.rs ownership)

- `xtra1.cc:2480-2498` apply_flags (item.rs): `TR_STEALTH` adds `pval`; `TR_TUNNEL` adds
  `pval * 20`; `TR_INVIS` accumulates `invis_power += pval * 10` (sets `invis` when positive);
  `TR_BLESSED` is no longer a slay flag and the port's ×2-vs-EVIL bonus is removed (`bless_blade`
  only, checked via item flags in game.rs).
- `birth.cc:391` luck (item.rs): `totals_for` starts from `ps.luck_base` (race+subrace+rand(-5,5))
  instead of re-summing racial luck; the holy aura adds `hold_life` and +5 luck.
- `xtra2.cc:825/869` blessed/holy (modal.rs): Blessing/Holy Chant/Holy Prayer scrolls call
  `set_blessed` with `randint(12)+6`/`randint(24)+12`/`randint(48)+24`; Manwe's Blessing sets
  blessed + clears fear/tim_lite + hero/shero/holy at L50 10/20/30; Dispel Magic clears blessed;
  Mediator activation sets blessed + the five oppose_* timers.
- `xtra2.cc:350` tim_deadly (modal.rs): Divine Aim calls `set_tim_deadly` at effective level 20+
  (`critical_norm` already forces *GREAT* hits); `melkor_sacrifice_damage`/`roots_damage_bonus`
  are added to `modal.rs::player_attack_monster`.
- `spells3.cc:3116` tree roots (modal.rs): `yavanna_roots` now uses `game::set_roots(dur, AC, dam)`
  (10+L30 / 10+L60 / 10+L20) instead of a shield.
- `xtra2.cc:4996-5320` make_wish (modal.rs): a staff of Wish opens `Modal::Wish` (typed text);
  `game::make_wish` resolves it, objects drop via `drop_near`, monsters scatter within 5 with the
  requested status; a refused wish does not spend the charge.
- `xtra1.cc:1833` calc_torch (render.rs): the FOV radius uses `game::player_lite` (temp +2, holy +1,
  cap 5); `cell_render` draws `Map::display_terrain` (mimics); `update_mon_lite` now uses
  `MAX_SIGHT + 1`, lights only already-visible cells and skips opaque cells when the monster is
  out of LOS.
- `monster2.cc:1724` WEIRD_MIND (render.rs): `monster_sensed` rolls the 10% occasional telepathy
  deterministically per turn/entity.
- `birth.rs` test only: `base_subraces_carry_their_birth_effects` now asserts `t.luck == ps.luck_base`
  (the old `-3` asserted the pre-luck_base behavior).

Still outside these files: `drunk_takes_wine` call site (input.rs Give direction), Farmer Maggot
artifact 149 + Cure Serious mushrooms and `god_relic_reward`/`do_cmd_suicide` wiring (input.rs).


## DONE THIS SESSION (final gaps pass, 2026-09-16)

Files: `bevy/src/game.rs`, `bevy/src/item.rs`, `bevy/src/birth.rs`,
`bevy/src/skill.rs`, `bevy/src/mimic.rs`, `bevy/src/input.rs`,
`bevy/src/data.rs`.  `cargo test`: 200 passed / 0 failed.

- **`calc_powers` notifications** (`xtra1.cc:1353-1486`): the full 62-row
  `POWER_TEXTS` gain/lose table, a `known_powers` `PlayerState` set and
  `game::sync_powers` (called from `upkeep`) printing the sorted-set diff;
  `known_powers` is seeded in `birth::start_game` so birth is silent
  (calc_powers_silent).  `mimic`/spell-granted powers (mouse invis,
  spider web, continuum, realm) now report their gain/lose lines.
- **Dynamic vampire subrace** (`corrupt.cc:45-112`): `display_race_name`
  already prefixes/renames from `SUBRACE_SAVE` semantics via the teeth
  corruption; `game::do_rebirth` (exp factor + hit die + `rerate_hp` +
  `check_experience`) added for the strength step.  The one remaining
  wiring is `corrupt::gain(CORRUPT_VAMPIRE_STRENGTH)` calling it —
  `corrupt.rs` is outside this session's ownership (the stat deltas,
  `+1 max HP`/`+100 exp` and flags are already applied there).
- **Possessor body** (`xtra1.cc:1969-2134`): `item::totals_for` now folds
  `calc_body_bonus` for `ps.possessed` (body AC, absolute body speed via
  `t.speed += m.speed - 110`, NEVER_MOVE/STUPID/SMART/REFLECTING/
  INVISIBLE(+20)/REGENERATE/AURA_*/PASS_WALL/SUSCEP_FIRE/IM_*/RES_*/
  NO_FEAR/NO_SLEEP/NO_CONF/CAN_FLY/AQUATIC) and applies the disembodied
  wraith form with the race flags suppressed (body_monster semantics).
  Body parts/forced unequip already live in `mimic::calc_body`/
  `drop_unusable_slots`; the HP formula and `undead_form` were present.
- **`calc_wield_monster`** (`xtra1.cc:1930`): `RF_CAN_FLY` now grants only
  ffall, `RF_AQUATIC` only water breathing, and `RF_INVISIBLE` adds 20
  invisibility power (was fly+feather / magic_breath / bool).
- **`get_coin_type`** (`xtra2.cc:2070`): `item::get_coin_type` +
  `make_gold_coin`; `monster_carried_treasure` forces the matching
  copper/silver/gold/mithril/adamantite kind and creeping coins no longer
  leave a corpse (`!force_coin`).
- **`mouse_calc`** (`mimic.cc:51-73`): the `/5` no longer hits equipment
  damage; `input.rs` divides the stat-derived `adj_str_td` instead, so a
  mouse keeps full gear to-damage.
- **`drop_from_wild`** (`xtra2.cc:1647`): digestion now uses the exact
  set_food tier crossings (faint/weak/hungry/no-longer-full/no-longer-
  gorged) and force-exits the world overview (`Goto::Wild`) at the
  faint/weak lines.
- **Stat limits / `modify_stat_value`** (`xtra1.cc:130`): added as
  `game::modify_stat_value` (folded ±clamp 3..40 = 18/220 cap) and used
  by the stat potions/Augmentation; the `adj_*` table family, WIS save
  table + Spirituality + antimagic 95, skill_stl 0..30, skill_dig >= 1,
  encumbrance/bloating speed and the calc_mana terms (adj_mag_mana,
  gloves, Eru/MANA boons, school mana) were verified present.  A
  `speed_factor` field + `set_fast` helper replace the hardcoded haste
  +10 (`player_speed`); the cmd7.cc factor setter remains in modal.rs.
- **`increase_skill`/`decrease_skill`** (`skills.cc:365-427`): still the
  point-by-point commit; the session-refund model needs a skill-screen
  session (base values + invest counts) in modal.rs, so the exclusion
  refund is not reproduced (marked `[>]`).
- **`do_get_new_skill`** (`skills.cc:1382-1543`): random gains no longer
  wipe the opposing skills (the original only asks "This skill is
  mutually exclusive with at least X, continue?"); `opposing_skill` is
  exported for the modal prompt, which modal.rs does not show yet.
- **Point-based birth / autoroller** (`birth.cc:2082/2102/2250`):
  `BIRTH_STAT_COSTS`, `point_buy_cost`, `point_buy_gold` and the capped
  `auto_roll` (1,000,000 rounds, per-stat match counts) are implemented
  with a test.  The interactive point-buy screen/autoroll minimum entry
  is still a `BirthStep` UI to add (`birth.rs`); the APIs are ready.

## FINAL RECONCILIATION

Reconciliation pass 2026-09-16 (third pass, spell/game-side session) over this report's
`[>]` bullets.  One owned-file item was completed; the rest are cross-file:

- `[x]` `place_corpse` — `item::Item` now carries `found`/`found_aux1..4` and
  `item::init_corpse` sets `OBJ_FOUND_MONSTER` + `found_aux1 = r_idx`; this session added
  the missing `found_aux3` (dungeon type) and `found_aux4` (`level_or_feat`) at both
  `game.rs` corpse-drop sites.  `found_aux2` (the monster ego) still has no per-monster
  field in the port (ego variants are folded into the monster def), so it stays 0.
- `[x]` `forbid_gloves`/`forbid_non_blessed` — `wield_icky` + `icky_wield` state, the
  spell failure +25 and the `cumber_glove` state/message are done; the parallel session added
  the `to_d` half (`input.rs::damage_bonus -= game::wield_malus(...)`, xtra1.cc:3727-3772), so
  both to-hit and damage carry the Eru/Sorcery malus.
- `[x]` `calc_bonuses` SKILL_HAND branch — threshold/speed/half-blows/AC/free
  action/shield-slot AC are all in game.rs; `item.rs`'s free-act consumers use
  `item::has_free_act` (equipment `FREE_ACT` + `god_free_act` + `monk_free_act`).
  This session additionally routed `game::sanity_blast`'s brain-smash paralysis check
  through `item::has_free_act` (it read `totals.free_act` only).  `mimic.rs` has no
  free-act reader: the shapes only *grant* `t.free_act`, which flows into `totals` and is
  therefore honoured by every `has_free_act` caller.  Remaining `modal.rs` readers
  (`modal.rs:15198`, `21231`, `21371`) still test `totals.free_act` directly — the
  monk/god free action is not honoured there (cross-file, modal.rs session).
- `[x]` `monk_empty_hands` — blow branch guarded and `choose_melee` (modal.rs) now strips both
  weapon slots into the pack before switching, refusing (with "Hmmm, your %s seems to be
  cursed.") when a wielded weapon is cursed (skills.cc:777).
- `[>]` status notice semantics — deactivations and the tim_lite/tim_poison/tim_thunder
  expiries print in `upkeep`; `game::set_tim_poison`/`set_tim_thunder`/`set_blessed`/
  `set_holy`/`set_tim_deadly`/`set_roots` carry the activation lines.  The parallel session
  routed `modal.rs`'s poison-blood (`set_tim_poison`), thunderstorm (`set_tim_thunder`) and
  Manwe's Blessing (`set_lite(0)`) through the message helpers; this session (modal.rs)
  added the missing 0 -> positive activation lines for `absorb_soul`, `tim_invis`,
  `tim_infra`, `prob_travel` and `strike`, and verified the `tim_project`/`tim_reflect`/
  `tim_regen`/`tim_wraith` arms print their own school lines.  Still open: the player-side
  `set_slow` activation (game.rs GF_OLD_SLOW branch) and the exact C++ wording where a
  school-flavoured line is printed instead of the set_* text.  `set_tim_breath` is dead
  data in base (nothing assigns `tim_water_breath`/`tim_magic_breath`), so its lines are
  unreachable.  Verified in game.rs: the only raw timer write is the `check_music` Song of
  the Sun tick (`ps.tim_lite = 5`), which has no C++ message.
- `[~]` `gods.cc:83-111` `follow_god` — Melkor Udun reveal message only; the skill is
  already unhidden (`skill.rs:175`).

Session log (spell-table/game-side, 2026-09-16; `bevy/src/game.rs`, `bevy/src/spell.rs`):
- `place_corpse` aux: both `game.rs` corpse-drop sites now set `found_aux3` (dungeon type)
  and `found_aux4` (`game::level_or_feat`: depth, or the wilderness terrain on the
  surface) in addition to the `found`/`found_aux1` set by `item::init_corpse`.
  `found_aux2` (monster ego) has no field in the port's `Monster` (ego variants are folded
  into the def index), so it stays 0.
- Class titles (`ClassRow.titles`, p_info `C:D:1:`) are parsed but still unconsumed: the
  C++ uses them in the death screen (`files.cc:4072`, "Name / the / <title>") and the
  high-score names; both live in `scores.rs`/`hud.rs`, outside this session's ownership.
  **SESSION (hud.rs) 2026-09-16:** the death screen now prints the base-class title
  (`gd.classes[base_class].titles[(level-1)/5]`, "Magnificent" above level 50) on the
  C++ "Name / the / title / spec title / Level" lines.  The high-score entry names in
  `scores.rs` still use the specialisation title only (cross-file, not owned).
- `cargo test` 209 passed / 0 failed.
