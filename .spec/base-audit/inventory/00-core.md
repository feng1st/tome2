# Method inventory: 00-core

Audit annotations (see `reports/00-core.md`). `[x]` ported/equivalent, `[>]` partial, `[ ]` missing,
`[~]` not needed (UI/backend/util/hook-mechanism/dead data).

## game.cc (0 defs)

All functions here are the `PWR_*` power table (`Game::Game()`); powers were audited separately and
are ported (see handoff). No checkbox items.

## files.cc (61 defs)

- [~] `files.cc:75` **name_file_note**(std::string_view sv) — [~] 固定文件名（savegame.ron/notes.txt）取代原命名规则（save.rs path()/notes.rs default_path）；accept.py file-naming 类机械复核
- [~] `files.cc:84` **name_file_pref**(std::string_view sv) — [~] 固定文件名（savegame.ron/notes.txt）取代原命名规则（save.rs path()/notes.rs default_path）；accept.py file-naming 类机械复核
- [~] `files.cc:93` **name_file_save**() — [~] 固定文件名（savegame.ron/notes.txt）取代原命名规则（save.rs path()/notes.rs default_path）；accept.py file-naming 类机械复核
- [~] `files.cc:98` **name_file_save**(std::string_view sv) — [~] 固定文件名（savegame.ron/notes.txt）取代原命名规则（save.rs path()/notes.rs default_path）；accept.py file-naming 类机械复核
- [~] `files.cc:107` **name_file_dungeon_save**(std::string const &ext) — [~] 固定文件名（savegame.ron/notes.txt）取代原命名规则（save.rs path()/notes.rs default_path）；accept.py file-naming 类机械复核
- [~] `files.cc:129` **tokenize**(char *buf, s16b num, char **tokens, char delim1, char delim2) — [~] pref 文本解析由 RON 数据与 Options 结构取代（options.rs/squeltch.rs）；accept.py pref-parser 类机械复核
- [~] `files.cc:258` **process_pref_file_aux**(char *buf) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:749` **process_pref_file_expr**(char **sp, char *fp) — Helper function for "process_pref_file()" Input: v: output buffer array f: final character Output: result [audit: frontend/UI utility; bevy implements its own (UI 单独立项)]
- [~] `files.cc:922` **process_pref_file**(std::string const &name) — [~] pref 文本解析由 RON 数据与 Options 结构取代（options.rs/squeltch.rs）；accept.py pref-parser 类机械复核
- [~] `files.cc:1036` **prt_lnum**(const char *header, s32b num, int row, int col, byte color) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:1050` **prt_num**(const char *header, int num, int row, int col, byte color, const char *space) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:1066` **prt_str**(const char *header, const char *str, int row, int col, byte color) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:1084` **display_player_middle**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [x] `files.cc:1259` **likert**(int x, int y) — [x] bevy/src/files.rs:11 likert（原版档位逐条对应）；测试 likert_matches_the_original_bands
- [~] `files.cc:1341` **display_player_various**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:1509` **wield_monster_flags**() — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [x] `files.cc:1547` **apply_lflags**(LF const &lflags) — race/class level flags applied in xtra1.cc (Rust game.rs) [audit: ported (bevy/src; behaviour equivalent)]
- [~] `files.cc:1559` **player_flags**() — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [x] `files.cc:1786` **number_to_digit**(int n) — [x] bevy/src/files.rs:31 number_to_digit（|n| 封顶 '*'）；测试 number_to_digit_caps_at_star
- [x] `files.cc:1797` **object_flag_types_are_compatible**(char type_a, char type_b) — [x] bevy/src/files.rs:53 object_flag_types_are_compatible；测试 flag_cells_append_by_type
- [x] `files.cc:1864` **object_flag_cell_append**(object_flag_cell const &a, object_flag_cell const &b) — [x] bevy/src/files.rs:66 object_flag_cell_append（含 to_char 变体 files.rs:111）；测试 flag_cell_chars_follow_the_original
- [~] `files.cc:1993` **display_flag_row**(int y, int x0, std::vector<std::tuple<char, int, object_flag_set>> const &slots, std::vector<object_flag_meta const *> c...) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:2082` **display_player_ben_one**(int page) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:2163` **display_player**(int mode) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:2270` **describe_player_location**() — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [~] `files.cc:2376` **file_character_print_grid_check_row**(const char *buf) — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [~] `files.cc:2397` **file_character_print_grid**(FILE *fff, bool show_gaps, bool show_legend) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:2439` **file_character_print_item**(FILE *fff, char label, object_type *obj) — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [~] `files.cc:2454` **file_character_print_store**(FILE *fff, wilderness_type_info const *place, std::size_t store) — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [~] `files.cc:2484` **file_character_check_stores**(std::unordered_set<store_type *> *seen_stores, wilderness_type_info const *place, int store) — [~] 角色表/flag 汇总（display_player 家族输出）；port HUD 自行计算（Inventory::totals_for）；accept.py char-sheet 类机械复核
- [~] `files.cc:2506` **file_character**(const char *name) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:2875` **show_file_aux**(const char *name, const char *what, int line) — [~] 在线帮助/截图查看器（UI 立项），port 无游戏内 help 浏览器；accept.py help-viewer 类机械复核
- [~] `files.cc:3456` **show_string**(const std::string &lines, const char *title, int line) — [~] 在线帮助/截图查看器（UI 立项），port 无游戏内 help 浏览器；accept.py help-viewer 类机械复核
- [~] `files.cc:3474` **show_file**(const char *name, const char *what, int line) — [~] 在线帮助/截图查看器（UI 立项），port 无游戏内 help 浏览器；accept.py help-viewer 类机械复核
- [~] `files.cc:3479` **cmovie_clean_line**(int y, char *abuf, char *cbuf) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:3537` **help_file_screenshot**(const char *name) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:3590` **html_screenshot**(const char *name) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:3676` **do_cmd_help**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [x] `files.cc:3691` **process_player_name**(std::string const &name) — ported (save.rs / birth.rs / item.rs / game.rs) [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:3738` **set_player_base**(std::string const &name) — ported (save.rs / birth.rs / item.rs / game.rs) [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:3754` **get_name**() — Gets a name for the character, reacting to name changes. Assumes that "display_player(0)" has just been called Perhaps we should NOT ask for a name (at "birth()") on Unix machines? XXX XXX What a horr [audit: frontend/engine utility; bevy implements its own]
- [x] `files.cc:3798` **do_cmd_suicide**() — Rust shift+Q saves & quits instead of killing the character ("Quitting" death, high score, last words, @-verification missing) — synced from report [audit: frontend/engine utility; bevy implements its own]
- [x] `files.cc:3848` **remove_cave_view**(bool remove) — HACK - Remove / set the CAVE_VIEW flag, since view_x / view_y is not saved, and the visible locations are not lighted correctly when the game is loaded again Alternatively forget_view() and update_vie [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:3875` **do_cmd_save_game**() — Save the game [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:3928` **autosave_checkpoint**() — [x] bevy/src/input.rs autosave_request（autosave_l）+ consume_turn 的 timed 判定 + 存档执行（player_input autosave_due 分支）；files.rs:136 timed_autosave_due；测试 timed_autosave_due_matches_the_original_formula
- [x] `files.cc:3942` **total_points**() — Hack -- Calculates the total number of points earned -JWT- (known done; options multiplier omitted by design) [audit: frontend/engine utility; bevy implements its own]
- [~] `files.cc:4031` **print_tomb**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4091` **show_info**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4222` **display_scores_aux**(int highscore_fd, int from, int to, int note, high_score *score) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4390` **show_highclass**(int building) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4511` **race_score**(int race_num) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4591` **race_legends**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [x] `files.cc:4614` **top_twenty**() — high-score list known done (scores.rs) [audit: ported (bevy/src; behaviour equivalent)]
- [~] `files.cc:4747` **predict_score**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4843` **predict_score_gui**(bool *initialized_p, bool *game_in_progress_p) — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [~] `files.cc:4905` **kingly**() — [~] 终端屏幕（print_tomb/show_info/scores 等）；Bevy 由 hud::setup_dead/scores.rs/UI 承担；accept.py terminal-UI 类机械复核
- [x] `files.cc:4957` **wipe_saved**() — Wipe the saved levels [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:4990` **close_game**() — Close up the current game (player may or may not be dead) This function is called only from "main.c" and "signals.c". [audit: frontend/engine utility; bevy implements its own]
- [x] `files.cc:5076` **get_rnd_line**(const char *file_name, char *output) — random speech line (speech.rs) [audit: ported (bevy/src; behaviour equivalent)]
- [x] `files.cc:5153` **get_line**(const char* fname, const char *fdir, char *linbuf, int line) — [x] bevy/src/files.rs:128 get_line；测试 get_line_reads_nth_line
- [x] `files.cc:5201` **get_xtra_line**(const char *file_name, monster_type *m_ptr, char *output) — unique speech (speech.rs) [audit: ported (bevy/src; behaviour equivalent)]

## init1.cc (43 defs)

- [x] `init1.cc:508` **color_char_to_attr**(char c) — [x] bevy/src/colors.rs:42 color_char_to_attr（16 色字母表与 init1.cc 一致）；测试 color_letters_map_to_the_palette_order
- [~] `init1.cc:575` **monster_ego_modify**(char c) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:599` **my_strdup**(const char *s) — [~] C 字符串助手（strdup/realloc）→ Rust String；accept.py c-runtime 类机械复核
- [~] `init1.cc:614` **strappend**(char **s, const char *t) — [~] C 字符串助手（strdup/realloc）→ Rust String；accept.py c-runtime 类机械复核
- [~] `init1.cc:643` **grab_one_class_flag**(std::array<u32b, 2> &choice, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:664` **grab_one_race_allow_flag**(std::array<u32b, 2> &choice, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:688` **grab_one_skill_flag**(skill_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:708` **grab_one_player_race_flag**(player_race_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:727` **get_activation**(char *activation) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:745` **object_flag_set_from_string**(const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:761` **grab_object_flag**(object_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:779` **read_skill_modifiers**(skill_modifiers *skill_modifiers, const char *buf) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:810` **read_proto_object**(std::vector<object_proto> *protos, const char *buf) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:840` **read_ability**(std::vector<player_race_ability_type> *abilities, char *buf) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:899` **init_player_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:1834` **init_v_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:1978` **grab_one_feature_flag**(const char *what, feature_flag_set *flags) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:2000` **init_f_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:2249` **init_k_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:2571` **init_a_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:2807` **init_set_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:2946` **init_s_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3184` **init_ab_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3389` **lookup_ego_flag**(const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3407` **grab_one_ego_item_flag**(object_flag_set *flags, ego_flag_set *ego, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3438` **init_e_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3769` **init_ra_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3977` **grab_monster_race_flag**(monster_race_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:3999` **grab_one_monster_spell_flag**(monster_spell_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:4021` **init_r_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:4354` **init_re_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:4747` **grab_one_dungeon_flag**(dungeon_flag_set *flags, const char *str) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:4765` **post_d_info**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:4803` **init_d_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5314` **grab_one_race_flag**(owner_type *ow_ptr, int state, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5342` **grab_one_store_flag**(store_flag_set *flags, const char *what) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5362` **init_st_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5600` **init_ba_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5715` **init_ow_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:5849` **init_wf_info_txt**(FILE *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:6020` **process_dungeon_file_aux**(char *buf, int *yval, int *xval, int xvalstart, int ymax, int xmax, bool full) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:6627` **process_dungeon_file_expr**(char **sp, char *fp) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init1.cc:6844` **process_dungeon_file**(const char *name, int *yval, int *xval, int ymax, int xmax, bool init, bool full) [audit: parser plumbing replaced by convert_data.py (fields land in data.rs/RON)] — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核

## init2.cc (13 defs)

- [~] `init2.cc:120` **init_file_paths**(char *path) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:255` **note**(const char *str) — [~] 终端行显示 → Bevy HUD/消息；accept.py terminal-UI 类机械复核
- [~] `init2.cc:497` **init_v_info**() [audit: init-time assembly replaced by convert_data.py + game.rs data load (parser/converter)] — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:505` **init_basic**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:526` **init_misc**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:550` **init_towns**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:581` **create_stores_stock**(int t) [audit: init-time assembly replaced by convert_data.py + game.rs data load (parser/converter)] — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:606` **init_other**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:684` **init_alloc**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:865` **init_sets_aux**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:889` **init_guardians**() — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:938` **init_angband_aux**(const char *why) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核
- [~] `init2.cc:1005` **init_angband**(program_args const &args) — [~] C++ 数据文件解析（init1/init2）：全部由 tools/convert_data.py 预转换为 RON、data.rs 加载；accept.py data-parser 类机械复核

## modules.cc (28 defs)

- [~] `modules.cc:56` **private_check_user_directory**(const char *dirpath) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:71` **module_reset_dir_aux**(char **dir, const char *new_path) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:86` **module_reset_dir**(const char *dir, const char *new_path) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:132` **dump_modules**(int sel, int max) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:170` **activate_module**(int module_idx) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:194` **init_module**(module_type *module_ptr) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:210` **module_savefile_loadable**(std::string const &tag) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:216` **find_module**(const char *name) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:231` **select_module**(program_args const &args) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:331` **dleft**(byte c, const char *str, int y, int o) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:366` **dright**(byte c, const char *str, int y, int o) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:407` **show_intro**(intro_text intro_texts[]) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:434` **tome_intro**() — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:477` **theme_intro**() — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:525` **auto_stat_gain_hook**(void *data, void *in, void *out) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:542` **drunk_takes_wine**(void *, void *in_, void *) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:572` **hobbit_food**(void *, void *in_, void *) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:593` **smeagol_ring**(void *data, void *in_, void *out) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:614` **longbottom_leaf**(void *, void *in_, void *) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:629` **food_vessel**(void *, void *in_, void *ut) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:651` **erebor_stair**(void *, void *in_, void *out_) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:692` **orthanc_stair**(void *, void *in_, void *out_) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:730` **theme_push_past**(void *data, void *in_, void *out_) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:775` **race_in_list**(int r_idx, int race_idxs[]) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:793` **theme_race_status**(int r_idx) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:1122` **theme_level_end_gen**(void *, void *, void *) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:1138` **theme_new_monster_end**(void *, void *in_, void *) — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核
- [~] `modules.cc:1151` **init_hooks_module**() — [~] 模块/主题装载器：port 单一 ToME 模块、数据烘焙进 RON，无模块选择/主题 hook（既定 Theme 边界）；accept.py module-theme 类机械复核

## options.cc (0 defs)

`options::reset_cheat_options` (cheat/wizard options) — `[~]` wizard/debug. Gameplay-relevant option
semantics are audited in the report (smart_learn, auto_scum, small/empty levels, wear_confirm).

## message.cc (0 defs)

`message::text_with_count` — equivalent implemented by MessageLog (`<Nx>` dedup). `[x]`.

## messages.cc (0 defs)

`Messages::size/at/add` — equivalent message log implemented. `[x]`.

## notes.cc (5 defs)

- [x] `notes.cc:27` **note_path**() — Get note path name — bevy/src/notes.rs:57 (`default_path`/`Notes.path`)
- [x] `notes.cc:37` **show_notes_file**() — bevy/src/notes.rs:64 show_notes_file（caption+内容）；modal.rs Knowledge 页 10 使用；测试 show_notes_file_prefixes_the_caption
- [x] `notes.cc:52` **output_note**(const char *final_note) — Output a string to the notes file. This is the only function that references that file. — synced (ported)
- [x] `notes.cc:73` **add_note**(char *note, char code) — Add note to file using a string + character symbol to specify its type so that the notes file can be searched easily by external utilities. — synced (ported)
- [x] `notes.cc:102` **add_note_type**(int note_number) — Append a note to the notes file using a "type". — synced (ported)

## hiscore.cc (5 defs)

Known done (scores.rs, own persistence). `[x]` for all.

- [x] `hiscore.cc:8` **highscore_seek**(int highscore_fd, int i) [audit: ported (scores.rs HighScores)]
- [x] `hiscore.cc:14` **highscore_read**(int highscore_fd, high_score *score) [audit: ported (scores.rs HighScores)]
- [x] `hiscore.cc:20` **highscore_write**(int highscore_fd, high_score *score) [audit: ported (scores.rs HighScores)]
- [x] `hiscore.cc:26` **highscore_where**(int highscore_fd, high_score *score) [audit: ported (scores.rs HighScores)]
- [x] `hiscore.cc:49` **highscore_add**(int highscore_fd, high_score *score) [audit: ported (scores.rs HighScores)]

## loadsave.cc (60 defs)

Save/load is known done with bevy's own RON format (`save.rs` + `LevelStore`); the serializer
details below are backend-specific, so all `[~]`. State round-trips (inventory, stores, quests,
wilderness, randarts, message log, options) are covered by the RON save.

- [~] `loadsave.cc:55` **note**(const char *msg) — [~] 终端消息行显示 → Bevy HUD/消息日志；accept.py terminal-UI 类机械复核
- [~] `loadsave.cc:93` **sf_get**() — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:105` **sf_put**(byte v) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:114` **do_byte**(byte *v, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:132` **do_char**(char *c, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:137` **do_std_bool**(bool *x, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:155` **do_u16b**(u16b *v, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:176` **do_s16b**(s16b *ip, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:181` **do_u32b**(u32b *ip, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:205` **do_s32b**(s32b *ip, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:210` **do_int**(int *sz, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:227` **save_std_string**(std::string const *s) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:239` **load_std_string**() — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:259` **do_std_string**(std::string &s, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:272` **do_option_value**(option_value *option_value, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:423` **do_bytes**(ls_flag_t flag, std::uint8_t *buf, std::size_t n) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:431` **do_seed**(seed_t *seed, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:449` **do_boost_optional**(boost::optional<T> &maybe_v, ls_flag_t flag, F f) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:488` **do_quick_start**(ls_flag_t flag, birther &previous_char) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:508` **do_skill_modifier**(skill_modifier *s, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:516` **do_skill_modifiers**(skill_modifiers *skill_modifiers, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:521` **do_player_level_flag**(player_level_flag *lflag, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:530` **do_subrace**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:576` **do_random_spell**(random_spell *s_ptr, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:589` **do_level_marker**(level_marker *marker, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:614` **do_extra**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:990` **do_monster**(monster_type *m_ptr, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [x] `loadsave.cc:1045` **wearable_p**(object_type *o_ptr) — [x] bevy/src/item.rs:541 wearable_p（沿用 ObjectDef::wearable 的 tval 开关）；测试 wearable_p_follows_the_original_tval_switch
- [~] `loadsave.cc:1097` **do_item**(object_type *o_ptr, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1291` **do_cave_type**(cave_type *c_ptr, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1303` **do_grid**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1314` **do_objects**(ls_flag_t flag, bool no_companions) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1417` **do_monsters**(ls_flag_t flag, bool no_companions) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1528` **do_dungeon**(ls_flag_t flag, bool no_companions) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1613` **save_dungeon**() — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1636` **do_store**(store_type *str, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [x] `loadsave.cc:1678` **do_randomizer**(ls_flag_t flag) — [x] bevy/src/rng.rs:220 do_randomizer_state / :225 do_randomizer_restore；save.rs 存 `rng_state`、game.rs setup_level 恢复
- [x] `loadsave.cc:1708` **do_options**(ls_flag_t flag) — [x] options.rs 序列化 + save.rs `options` 字段（SaveGame.options）；input.rs build_save / game.rs setup_level 存取；测试 rng_state/选项随存档往返
- [~] `loadsave.cc:1856` **do_inventory**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1952` **do_message**(message &msg, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1963` **do_messages**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:1991` **load_dungeon**(std::string const &ext) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2029` **do_timers**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2042` **do_stores**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2088` **do_monster_lore**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2107` **do_object_lore**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2122` **do_towns**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2204` **do_quests**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2225` **do_wilderness**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2265` **do_randart**(random_artifact *ra_ptr, ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2276` **do_randarts**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2282` **do_artifacts**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2295` **do_fates**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2316` **do_floor_inscriptions**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2327` **do_player_hd**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2344` **do_savefile_aux**(ls_flag_t flag) — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [~] `loadsave.cc:2528` **rd_savefile**() — [~] C++ 二进制 savefile 原语（ls_flag_t）；port 用 serde/RON 序列化同一状态（save.rs SaveGame）；accept.py save-serialization 类机械复核
- [x] `loadsave.cc:2566` **load_player**(program_args const &args) — [x] bevy/src/save.rs:299 load_player
- [x] `loadsave.cc:2709` **save_player_aux**(char const *name) — [x] bevy/src/save.rs:279 save_player_aux（RON 写 savegame.ron）
- [x] `loadsave.cc:2758` **save_player**() — [x] bevy/src/save.rs:290 save_player（input.rs:1083 退出存档调用）

## level_data.cc (0 defs)

`level_marker_values()` — used by the save/level-marker mechanism; LevelStore done. `[x]`.

## level_marker.cc (0 defs)

Empty wrapper. `[~]`.

## levels.cc (4 defs)

- [x] `levels.cc:24` **get_branch**() [audit: engine level table plumbing replaced by GameData/RON]
- [x] `levels.cc:29` **get_fbranch**() [audit: engine level table plumbing replaced by GameData/RON]
- [x] `levels.cc:34` **get_flevel**() [audit: engine level table plumbing replaced by GameData/RON]
- [x] `levels.cc:54` **get_level_flags**() [audit: engine level table plumbing replaced by GameData/RON]

## quest.cc (1 defs)

- [~] `quest.cc:7` **init_hooks_quests**() — [~] hook 机制由直接派发取代（quests 在 quest/plot 状态机直接处理）；accept.py hooks 类机械复核

## hooks.cc (3 defs)

- [~] `hooks.cc:63` **add_hook_new**(int h_idx, hook_func_t hook_func, const char *name, void *data) — hooks.cc 注册表；port 直接派发（如 HOOK_MONSTER_DEATH 语义在 game.rs:6639 check_quest_kill）
- [~] `hooks.cc:72` **del_hook_new**(int h_idx, hook_func_t hook_func) — hooks.cc 注册表；port 直接派发
- [~] `hooks.cc:84` **process_hooks_new**(int h_idx, void *in, void *out) — hooks.cc 注册表；port 直接派发

## dice.cc (5 defs)

- [x] `dice.cc:8` **dice_init**(dice_type *dice, long base, long num, long sides)
- [x] `dice.cc:17` **dice_parse**(dice_type *dice, const char *s) — C++ format `base+numdside`; Rust `Dice::parse` is `nds+b`; device charges special-cased with an off-by-one — synced from report
- [x] `dice.cc:54` **dice_parse_checked**(dice_type *dice, const char *s) — bevy/src/data.rs:1853 `Dice::parse` + checked call bevy/src/item.rs:3768
- [x] `dice.cc:62` **dice_roll**(dice_type *dice) — charge roll is `base + 0..M-1` in Rust vs `base + 1..M` — synced from report
- [x] `dice.cc:68` **dice_print**(dice_type *dice, char *output) — [x] bevy/src/data.rs Dice::print（dice.cc:68 格式）；测试 dice_print_matches_the_original

## z-rand.cc (18 defs)

- [x] `z-rand.cc:27` **reseed_rng**(rng_t *rng) — bevy/src/rng.rs:185 `reseed_rng(&mut Pcg64, seed)` reseeds a PCG64 instance; used by `new_seeded_rng` and `set_quick_rng` (z-rand.cc:27)
- [x] `z-rand.cc:43` **new_seeded_rng**(seed_t const &seed) — bevy/src/rng.rs:190 allocates and seeds a PCG64 (z-rand.cc:43)
- [x] `z-rand.cc:53` **quick_rng**() — bevy/src/rng.rs:135 `Registry::quick_rng` is the fixed-seed instance, reseeded by `set_quick_rng` (z-rand.cc:53)
- [x] `z-rand.cc:65` **complex_rng**() — bevy/src/rng.rs:140 `Registry::complex_rng` is the entropy-seeded instance whose stream survives quick excursions (z-rand.cc:65)
- [x] `z-rand.cc:80` **get_current_rng**() — bevy/src/rng.rs:145 dispatches to quick or complex per `use_quick` (z-rand.cc:80)
- [x] `z-rand.cc:92` **set_quick_rng**(seed_t const &seed) — bevy/src/rng.rs:197 reseeds the quick instance and selects it; used by town/wilderness/flavour code paths via `crate::rng::set_quick_rng` (z-rand.cc:92)
- [x] `z-rand.cc:98` **set_complex_rng**() — bevy/src/rng.rs:206 selects the complex instance without touching its stream (z-rand.cc:98)
- [x] `z-rand.cc:103` **get_complex_rng_state**() — bevy/src/rng.rs:211 hex `state:inc` (z-rand.cc:103); stored in `SaveGame.rng_state` (save.rs:227)
- [x] `z-rand.cc:110` **set_complex_rng_state**(std::string const &state) — bevy/src/rng.rs:216 parses and installs the saved state; called in game.rs setup_level on continue (z-rand.cc:110)
- [x] `z-rand.cc:121` **round_stochastic**(double x) — bevy/src/rng.rs:224 keeps the original n-1 quirk (z-rand.cc:121); tested in rng::tests
- [x] `z-rand.cc:154` **randnor**(int mean, int stand) — bevy/src/rng.rs:240 normal deviate, stand<1 => 0, s16b clamp, stochastic rounding; `rng::tests::randnor_matches_degenerate_and_clamp_behaviour`
- [x] `z-rand.cc:194` **damroll**(s16b num, s16b sides) — bevy/src/rng.rs:253 sum of `randint(sides)` per die; degenerate sides<=0 => num (z-rand.cc:194)
- [x] `z-rand.cc:206` **maxroll**(s16b num, s16b sides) — bevy/src/rng.rs:262 num*sides (z-rand.cc:206)
- [x] `z-rand.cc:211` **magik**(s32b p) — bevy/src/rng.rs:267 `rand_int(100) < p` (z-rand.cc:211)
- [x] `z-rand.cc:215` **rand_int**(s32b m) — bevy/src/rng.rs:272 0..=m-1, m<1 => 0 (z-rand.cc:215); game code uses `gen_range(0..m)` equivalents
- [x] `z-rand.cc:227` **randint**(s32b m) — bevy/src/rng.rs:281 1..=m, m<2 => 1 (z-rand.cc:227)
- [x] `z-rand.cc:239` **rand_range**(s32b a, s32b b) — bevy/src/rng.rs:290 a..=b, b<a => a (z-rand.cc:239)
- [x] `z-rand.cc:251` **rand_spread**(s32b a, s32b d) — bevy/src/rng.rs:299 `rand_range(a-d, a+d)`; negative d returns a-d (z-rand.cc:251); map::rand_spread delegates here

## squeltch.cc (0 defs, whole subsystem)

- [x] `squeltch.cc:63/90` **squeltch_grid/squeltch_inventory** — automatizer rules applied to floor
      and inventory every turn — bevy/src/input.rs:2585/2645 (called input.rs:836/948)
- [x] `squeltch.cc:249` **do_cmd_automatizer** — `= T` rule editor — synced from report
- [x] `squeltch.cc:452/515` **easy_add_rule/automatizer_add_rule** — destroy/pickup/inscribe rules —
      bevy/src/squeltch.rs:495 + wiring input.rs:3396 (interactive T/F/N/S prompt is UI; destroy adds TSVAL)
- [x] `squeltch.cc:557/574` **automatizer_init/automatizer_load** — JSON `.atm` persistence —
      bevy/src/main.rs:76 `init_resource` + bevy/src/squeltch.rs:537 (RON instead of JSON)

