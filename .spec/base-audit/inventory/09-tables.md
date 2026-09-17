# Method inventory: 09-tables

Audit annotations (see `reports/00-core.md`). `[x]` ported/equivalent, `[>]` partial, `[ ]` missing,
`[~]` not needed (UI/util/dead data/module system).

## tables.cc (0 defs, table-by-table)

The file is pure table definitions; each table was audited against `bevy/src/*.rs` / RON data.

- [x] `tables.cc:52` **ddd/ddx/ddy** keypad direction tables — Rust `input.rs:49 DIRS`
- [~] `tables.cc:79` **hexsym** — 仅被 util.cc ascii_to_text（宏编辑转义显示）与 main-win 使用；Bevy UI 无此转义表（accept.py 数据表类机械复核）
- [x] `tables.cc:89` **adj_mag_mana** — Rust max-mana formula (`skill.rs:306`, `birth.rs:280`) omits the `adj_mag_mana*lev/4` term — synced from report
- [x] `tables.cc:135` **adj_mag_fail** — `mimic.rs:467` — synced from report
- [x] `tables.cc:181` **adj_mag_stat** — `mimic.rs:458` — synced from report
- [x] `tables.cc:227` **adj_chr_gold** — `town.rs:258`
- [x] `tables.cc:273` **adj_wis_sav** — Rust `player_sav` uses `stat_bonus(WIS)*5` + 2*level instead — synced from report
- [x] `tables.cc:319` **adj_dex_ta** — Rust AC uses `stat_bonus(DEX)` (`game.rs:3662`) — synced from report
- [x] `tables.cc:365` **adj_str_td** — melee damage has no STR term in Rust (`input.rs:2339`) — synced from report
- [x] `tables.cc:411` **adj_dex_th** — Rust to-hit uses a STR-derived `melee_bonus` (`game.rs:1619`) — synced from report
- [x] `tables.cc:457` **adj_str_th** — same — synced from report
- [x] `tables.cc:503` **adj_str_wgt** — `game.rs:2805`
- [x] `tables.cc:549` **adj_str_hold** — bevy/src/game.rs:3543 ADJ_STR_HOLD + game.rs:3551 heavy_bow_penalty（xtra1.cc:3419 语义）；测试 heavy_bow_penalty_uses_adj_str_hold
- [x] `tables.cc:595` **adj_str_dig** — Rust digging is `DIG_TURNS - tunnel` (`input.rs:1479,2752`) with no STR table — synced from report
- [x] `tables.cc:641` **adj_str_blow** — `game.rs:2670` — synced from report
- [x] `tables.cc:687` **adj_dex_blow** — `game.rs:2647`
- [x] `tables.cc:733` **adj_dex_safe** — door-bash save (`cmd2.cc:1514`) approximated without DEX — synced from report
- [x] `tables.cc:779` **adj_con_fix** — `game.rs:2676`
- [x] `tables.cc:825` **adj_con_mhp** — Rust uses the table for sanity only; HP/level uses flat `(CON-10)/2` — synced from report
- [x] `tables.cc:895` **blows_table** — `game.rs:2652` — synced from report
- [x] `tables.cc:962` **extract_energy[300]** — bevy/src/game.rs:4048 exact table (consumed game.rs:12883/13742)
- [x] `tables.cc:1002` **player_exp[PY_MAX_LEVEL]** — bevy/src/game.rs:5787 `PLAYER_EXP` (exact table) + `exp_needed_at` game.rs:6064
- [~] `tables.cc:1060` **color_names** — 仅 cmd4.cc 颜色编辑器使用；Bevy 无该编辑器（数据表类）
- [x] `tables.cc:1084` **stat_names** — bevy/src/birth.rs:345 STAT_NAMES 与 tables.cc:1084 完全一致；game.rs:16447 同表
- [~] `tables.cc:1092` **stat_names_reduced** — 仅 files.cc/xtra1.cc 受伤状态标签使用；Bevy HUD 用整组标签（数据表类）
- [~] `tables.cc:1112` **window_flag_desc** — 仅 cmd4.cc/files.cc/init2.cc 多窗口配置使用；Bevy 无子窗口系统（数据表类）
- [x] `tables.cc:1150` **artifact_names_list** — converter `parse_artifact_names` -> randarts.ron — synced from report
- [x] `tables.cc:1755` **martial_arts ma_blows / bear_blows** — bevy/src/input.rs:6216 `MA_BLOWS` / input.rs:6383 `BEAR_BLOWS` + `py_attack_hand` input.rs:6471
- [x] `tables.cc:1789` **mindcraft_powers / necro_powers / mimic_powers / symbiotic_powers** — `modal.rs:1180+`, `mimic.rs`, `game.rs:3571`
- [~] `tables.cc:1990` **deity_info** — god names in `birth.rs:234`; descriptions are UI text — synced from report
- [x] `tables.cc:2156` **tactic_info** — `game.rs:1666` — synced from report
- [x] `tables.cc:2173` **activation_info** — converter `parse_junkarts` + `spell.rs:528 artifact_activation` — synced from report
- [x] `tables.cc:2231` **move_info** — `game.rs:1680` — synced from report
- [x] `tables.cc:2248` **inscription_info** — grid rune system implemented — synced from report
- [x] `tables.cc:2303` **flags_groups()** — bevy/src/game.rs:5798 `FLAGS_GROUPS` + `gain_flag_group` game.rs:5981 (`gain_flag_group_flag` game.rs:6038)
- [~] `tables.cc:2405` **quest[]** — curated `quests.ron` + q_*.cc plot ports (level gates implemented per plot)
- [x] `tables.cc:3094` **monster_powers** — `modal.rs:1087+` — synced from report
- [~] `tables.cc:3190` **tvals** — 仅 wizard2.cc 调试向导使用（数据表类）
- [x] `tables.cc:3237` **tval_descs** — bevy/src/item.rs:531 tval_desc（47 条与 tables.cc:3237 逐条对应）；observe 屏使用（modal.rs:6277）；测试 tval_desc_covers_the_original_table
- [~] `tables.cc:3481` **between_exits** — no base producer of FEAT_BETWEEN2 (Theme-only path); Rust pairs FEAT_BETWEEN gates per map — synced from report
- [x] `tables.cc:3507` **max_body_part** — caps hardcoded in `mimic.rs` — synced from report
- [~] `tables.cc:3520` **gf_names** — UI display text; bevy/src/game.rs:769 `gf_description` covers the used types (wording differs) — synced from report
- [~] `tables.cc:3616` **modules[]** — module system intentionally not ported

## z-form.cc (5 defs)

Rust uses `std::fmt` formatting. `[~]` for all.

- [~] `z-form.cc:178` **vstrnfmt**(char *buf, unsigned int max, const char *fmt, va_list vp) — C varargs（va_list），Rust 由 format!/fmt 承担（varargs 类）
- [~] `z-form.cc:557` **vformat**(const char *fmt, va_list vp) — Do a vstrnfmt (see above) into a (growable) static buffer. This buffer is usable for very short term formatting of results. [audit: terminal form frontend replaced by bevy UI]
- [~] `z-form.cc:606` **strnfmt**(char *buf, unsigned int max, const char *fmt, ...) — C varargs（... 与定长缓冲），Rust 由 String/format! 承担（varargs 类）
- [x] `z-form.cc:634` **format**(const char *fmt, ...) — bevy/src/zutil.rs:45 format(Arguments)->String（z-form.cc:634）
- [x] `z-form.cc:658` **quit_fmt**(const char *fmt, ...) — bevy/src/zutil.rs:51 quit_fmt(Arguments)->!（z-form.cc:658，Rust 用 format_args! 调用）

## z-util.cc (3 defs)

- [x] `z-util.cc:11` **capitalize**(char *s) — bevy/src/zutil.rs:7（跳过前导空白的原语义）；input.rs:6628 委托；测试 capitalize_skips_leading_space_and_only_touches_the_first_word
- [x] `z-util.cc:40` **plog**(const char *str) — bevy/src/zutil.rs:22（stderr，与原版 fprintf(stderr) 一致）
- [x] `z-util.cc:62` **quit**(const char *str) — bevy/src/zutil.rs:29（None=>0，+/- 为退出码，其余 plog 后 -1）
