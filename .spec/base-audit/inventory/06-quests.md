# Method inventory: 06-quests

## q_main.cc (10 defs)

- [~] `q_main.cc:23` **quest_describe**(int q_idx) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_main.cc:33` **quest_main_monsters_hook**(void *, void *in_, void *)
- [x] `q_main.cc:61` **quest_morgoth_hook**(void *, void *, void *)
- [~] `q_main.cc:113` **quest_morgoth_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_main.cc:132` **quest_morgoth_init_hook**()
- [x] `q_main.cc:142` **quest_sauron_hook**(void *, void *, void *)
- [x] `q_main.cc:168` **quest_sauron_resurrect_hook**(void *, void *in_, void *) — synced from report
- [x] `q_main.cc:190` **quest_sauron_init_hook**()
- [x] `q_main.cc:200` **quest_necro_hook**(void *, void *, void *) — main-chain rework: Galadriel's mirror gate on the Sauron stage replaces the necro plot step (input.rs:5411)
- [x] `q_main.cc:225` **quest_necro_init_hook**() — subsumed by the curated plot chain and the mirror gate (input.rs:5411)

## q_betwen.cc (7 defs)

- [x] `q_betwen.cc:33` **quest_between_move_hook**(void *, void *in_, void *) — synced from report
- [x] `q_betwen.cc:93` **quest_between_gen_hook**(void *, void *in_, void *)
- [x] `q_betwen.cc:133` **quest_between_finish_hook**(void *, void *in_, void *)
- [x] `q_betwen.cc:169` **quest_between_death_hook**(void *, void *, void *) — synced from report
- [~] `q_betwen.cc:202` **quest_between_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_betwen.cc:215` **quest_between_forbid_hook**(void *, void *in_, void *)
- [x] `q_betwen.cc:230` **quest_between_init_hook**()

## q_bounty.cc (7 defs)

- [x] `q_bounty.cc:24` **lua_mon_hook_bounty**(monster_race const *r_ptr) — synced from report
- [x] `q_bounty.cc:55` **get_new_bounty_monster**(int lev) — synced from report
- [x] `q_bounty.cc:76` **bounty_item_tester_hook**(object_type const *o_ptr)
- [x] `q_bounty.cc:81` **quest_bounty_init_hook**()
- [x] `q_bounty.cc:86` **quest_bounty_drop_item**() — modal.rs::bounty_assign
- [x] `q_bounty.cc:108` **quest_bounty_get_item**() — synced from report
- [~] `q_bounty.cc:160` **quest_bounty_describe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核

## q_dragons.cc (4 defs)

- [x] `q_dragons.cc:24` **quest_dragons_gen_hook**(void *, void *in_, void *) — synced from report
- [x] `q_dragons.cc:113` **quest_dragons_death_hook**(void *, void *, void *) — synced from report
- [x] `q_dragons.cc:152` **quest_dragons_finish_hook**(void *, void *in_, void *)
- [x] `q_dragons.cc:171` **quest_dragons_init_hook**()

## q_eol.cc (6 defs)

- [x] `q_eol.cc:29` **quest_eol_gen_hook**(void *, void *, void *)
- [x] `q_eol.cc:112` **quest_eol_finish_hook**(void *, void *in_, void *)
- [x] `q_eol.cc:144` **quest_eol_fail_hook**(void *, void *in_, void *) — synced from report
- [x] `q_eol.cc:165` **quest_eol_death_hook**(void *, void *in_, void *)
- [x] `q_eol.cc:190` **quest_eol_stair_hook**(void *, void *in_, void *)
- [x] `q_eol.cc:231` **quest_eol_init_hook**()

## q_evil.cc (4 defs)

- [x] `q_evil.cc:24` **quest_evil_gen_hook**(void *, void *in_, void *)
- [x] `q_evil.cc:84` **quest_evil_death_hook**(void *, void *, void *) — synced from report
- [x] `q_evil.cc:122` **quest_evil_finish_hook**(void *, void *in_, void *)
- [x] `q_evil.cc:141` **quest_evil_init_hook**()

## q_fireprof.cc (11 defs)

- [x] `q_fireprof.cc:55` **get_item_points_remaining**()
- [x] `q_fireprof.cc:61` **set_item_points_remaining**(s32b v)
- [x] `q_fireprof.cc:67` **item_tester_hook_eligible**(object_type const *o_ptr) — synced from report
- [x] `q_fireprof.cc:94` **fireproof_enough_points**(object_type *o_ptr, int *stack) — This function makes sure the player has enough 'points' left to fireproof stuff. — synced from report
- [x] `q_fireprof.cc:147` **fireproof**() — synced from report
- [x] `q_fireprof.cc:229` **quest_fireproof_building**(bool *paid, bool *recreate) — synced from report
- [x] `q_fireprof.cc:365` **fireproof_get_hook**(void *, void *in_, void *) — synced from report
- [x] `q_fireprof.cc:385` **fireproof_stair_hook**(void *, void *, void *)
- [~] `q_fireprof.cc:424` **quest_fireproof_describe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_fireprof.cc:465` **fireproof_gen_hook**(void *, void *in_, void *) — game.rs:11393 (scroll marked pval2/inscription, y=3..5 x=3..47 drop)
- [x] `q_fireprof.cc:509` **quest_fireproof_init_hook**()

## q_god.cc (29 defs)

- [x] `q_god.cc:52` **compass**(int y, int x, int y2, int x2) — Returns the direction of the compass that y2, x2 is from y, x the return value will be one of the following: north, south, east, west, north-east, south-east, south-west, north-west, or "close" if it  — synced from report
- [x] `q_god.cc:109` **approximate_distance**(int y, int x, int y2, int x2) — Returns a relative approximation of the 'distance' of y2, x2 from y, x. — synced from report
- [x] `q_god.cc:133` **MAX_NUM_GOD_QUESTS**() — synced from report
- [x] `q_god.cc:148` **get_relic_num**()
- [x] `q_god.cc:177` **get_home_coordinates**(int *home1_y, int *home1_x, const char **home1, int *home2_y, int *home2_x, const char **home2) — synced from report
- [x] `q_god.cc:233` **make_directions**(bool feel_it) — synced from report
- [~] `q_god.cc:284` **quest_god_describe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_god.cc:301` **quest_god_place_rand_dung**() — synced from report
- [x] `q_god.cc:369` **quest_god_generate_relic**() — synced from report
- [~] `q_god.cc:432` **quest_god_set_god_dungeon_attributes_eru**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:494` **quest_god_set_god_dungeon_attributes_manwe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:558` **quest_god_set_god_dungeon_attributes_tulkas**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:605` **quest_god_set_god_dungeon_attributes_melkor**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:660` **quest_god_set_god_dungeon_attributes_yavanna**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:713` **quest_god_set_god_dungeon_attributes_aule**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:757` **quest_god_set_god_dungeon_attributes_varda**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:818` **quest_god_set_god_dungeon_attributes_ulmo**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:865` **quest_god_set_god_dungeon_attributes_mandos**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:911` **quest_god_level_end_gen_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_god.cc:968` **quest_god_player_level_hook**(void *, void *in_, void *) — synced from report
- [x] `q_god.cc:1030` **quest_god_get_hook**(void *, void *in_, void *) — synced from report
- [~] `q_god.cc:1087` **quest_god_char_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:1123` **set_god_dungeon_attributes**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:1179` **quest_god_dungeon_setup**(int d_idx) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:1191` **quest_god_enter_dungeon_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:1198` **quest_god_gen_level_begin_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_god.cc:1204` **quest_god_stair_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_god.cc:1210` **quest_god_birth_objects_hook**(void *, void *, void *)
- [x] `q_god.cc:1223` **quest_god_init_hook**()

## q_haunted.cc (4 defs)

- [x] `q_haunted.cc:24` **quest_haunted_gen_hook**(void *, void *in_, void *)
- [x] `q_haunted.cc:99` **quest_haunted_death_hook**(void *, void *, void *) — synced from report
- [x] `q_haunted.cc:142` **quest_haunted_finish_hook**(void *, void *in_, void *)
- [x] `q_haunted.cc:161` **quest_haunted_init_hook**()

## q_hobbit.cc (7 defs)

- [x] `q_hobbit.cc:31` **quest_hobbit_town_gen_hook**(void *, void *in_, void *) — game.rs:7628 (Melinda spawns in Bree after DAY*10)
- [x] `q_hobbit.cc:66` **quest_hobbit_gen_hook**(void *, void *, void *) — game.rs:7277 (Merton in the Maze, dungeon 18, at hobbit_depth)
- [x] `q_hobbit.cc:98` **quest_hobbit_give_hook**(void *, void *in_, void *)
- [~] `q_hobbit.cc:128` **quest_hobbit_speak_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_hobbit.cc:145` **quest_hobbit_chat_hook**(void *, void *in_, void *) — synced from report
- [~] `q_hobbit.cc:197` **quest_hobbit_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_hobbit.cc:209` **quest_hobbit_init_hook**() — birth.rs:1244 (hobbit_depth rolled at game start)

## q_invas.cc (7 defs)

- [x] `q_invas.cc:23` **quest_invasion_gen_hook**(void *, void *, void *)
- [x] `q_invas.cc:71` **quest_invasion_ai_hook**(void *, void *in_, void *out_)
- [x] `q_invas.cc:113` **quest_invasion_turn_hook**(void *, void *, void *) — synced from report
- [~] `q_invas.cc:139` **quest_invasion_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_invas.cc:155` **quest_invasion_death_hook**(void *, void *in_, void *)
- [x] `q_invas.cc:180` **quest_invasion_stair_hook**(void *, void *in_, void *)
- [x] `q_invas.cc:222` **quest_invasion_init_hook**()

## q_library.cc (19 defs)

- [x] `q_library.cc:38` **push_spell**(s16b spell_idx) — spell.rs::BOOKABLE_SPELLS (spell.rs:327, full 43-spell list)
- [x] `q_library.cc:44` **initialize_bookable_spells**() — synced from report
- [x] `q_library.cc:103` **library_quest_place_random**(int minY, int minX, int maxY, int maxX, int r_idx)
- [x] `q_library.cc:110` **library_quest_place_nrandom**(int minY, int minX, int maxY, int maxX, int r_idx, int n)
- [x] `q_library.cc:121` **library_quest_book_get_slot**(int slot)
- [x] `q_library.cc:126` **library_quest_book_set_slot**(int slot, s32b spell)
- [x] `q_library.cc:131` **library_quest_book_slots_left**()
- [x] `q_library.cc:144` **library_quest_book_contains_spell**(int spell)
- [x] `q_library.cc:157` **quest_library_finalize_book**()
- [x] `q_library.cc:167` **library_quest_add_spell**(int spell)
- [x] `q_library.cc:177` **library_quest_remove_spell**(int spell)
- [~] `q_library.cc:191` **library_quest_print_spells**(int first, int current) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_library.cc:237` **library_quest_fill_book**()
- [x] `q_library.cc:314` **quest_library_gen_hook**(void *, void *in_, void *) — synced from report
- [x] `q_library.cc:359` **quest_library_stair_hook**(void *, void *, void *)
- [x] `q_library.cc:390` **quest_library_monster_death_hook**(void *, void *, void *) — game.rs::check_plot_kill (enemy_left reaches 0)
- [x] `q_library.cc:423` **quest_library_building**(bool *paid, bool *recreate)
- [~] `q_library.cc:478` **quest_library_describe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_library.cc:498` **quest_library_init_hook**()

## q_narsil.cc (4 defs)

- [x] `q_narsil.cc:18` **quest_narsil_move_hook**(void *, void *in_, void *) — synced from report
- [~] `q_narsil.cc:87` **quest_narsil_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_narsil.cc:99` **quest_narsil_identify_hook**(void *, void *in_, void *) — modal.rs:10130 (identifying Narsil starts the reforge)
- [x] `q_narsil.cc:126` **quest_narsil_init_hook**()

## q_nazgul.cc (6 defs)

- [x] `q_nazgul.cc:26` **quest_nazgul_gen_hook**(void *, void *in_, void *)
- [x] `q_nazgul.cc:63` **quest_nazgul_finish_hook**(void *, void *in_, void *) — synced from report
- [~] `q_nazgul.cc:93` **quest_nazgul_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_nazgul.cc:105` **quest_nazgul_forbid_hook**(void *, void *in_, void *)
- [x] `q_nazgul.cc:124` **quest_nazgul_death_hook**(void *, void *in_, void *)
- [x] `q_nazgul.cc:148` **quest_nazgul_init_hook**()

## q_nirna.cc (5 defs)

- [x] `q_nirna.cc:17` **quest_nirnaeth_gen_hook**(void *, void *, void *)
- [x] `q_nirna.cc:61` **quest_nirnaeth_finish_hook**(void *, void *in_, void *) — synced from report
- [x] `q_nirna.cc:101` **quest_nirnaeth_death_hook**(void *, void *, void *) — synced from report
- [x] `q_nirna.cc:110` **quest_nirnaeth_stair_hook**(void *, void *, void *)
- [x] `q_nirna.cc:130` **quest_nirnaeth_init_hook**()

## q_one.cc (10 defs)

- [x] `q_one.cc:31` **quest_one_move_hook**(void *, void *in_, void *) — synced from report
- [x] `q_one.cc:101` **quest_one_drop_hook**(void *, void *in_, void *) — synced from report
- [x] `q_one.cc:140` **quest_one_wield_hook**(void *, void *in_, void *)
- [x] `q_one.cc:207` **quest_one_hp_hook**(void *, void *in_, void *out_)
- [x] `q_one.cc:228` **quest_one_die_hook**(void *, void *, void *) — synced from report
- [x] `q_one.cc:248` **quest_one_identify_hook**(void *, void *in_, void *)
- [x] `q_one.cc:266` **quest_one_death_hook**(void *, void *in_, void *) — synced from report
- [~] `q_one.cc:357` **quest_one_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_one.cc:373` **quest_one_gen_hook**(void *, void *, void *)
- [x] `q_one.cc:410` **quest_one_init_hook**()

## q_poison.cc (7 defs)

- [x] `q_poison.cc:37` **create_molds_hook**(monster_race const *r_ptr)
- [x] `q_poison.cc:48` **quest_poison_gen_hook**(void *, void *, void *) — synced from report
- [x] `q_poison.cc:155` **quest_poison_finish_hook**(void *, void *in_, void *) — input.rs:3281 (Blue Dragon Scale Mail of Elvenkind reward)
- [~] `q_poison.cc:187` **quest_poison_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_poison.cc:199` **quest_poison_quest_hook**(void *, void *in_, void *)
- [x] `q_poison.cc:223` **quest_poison_drop_hook**(void *, void *in_, void *) — synced from report
- [x] `q_poison.cc:308` **quest_poison_init_hook**()

## q_rand.cc (13 defs)

- [x] `q_rand.cc:58` **initialize_random_quests**(int n) — synced from report
- [x] `q_rand.cc:204` **is_randhero**(int level)
- [x] `q_rand.cc:221` **do_get_new_obj**(int y, int x) — synced from report
- [x] `q_rand.cc:288` **princess_death**(s32b m_idx, s32b r_idx) — synced from report
- [x] `q_rand.cc:330` **hero_death**(s32b m_idx, s32b r_idx) — synced from report
- [x] `q_rand.cc:403` **quest_random_death_hook**(void *, void *in_, void *)
- [x] `q_rand.cc:438` **quest_random_turn_hook**(void *, void *, void *) — synced from report
- [~] `q_rand.cc:445` **quest_random_feeling_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_rand.cc:472` **quest_random_gen_hero_hook**(void *, void *, void *)
- [x] `q_rand.cc:506` **quest_random_gen_hook**(void *, void *in_, void *) — synced from report
- [~] `q_rand.cc:596` **quest_random_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_rand.cc:642` **quest_random_describe**() — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_rand.cc:682` **quest_random_init_hook**()

## q_shroom.cc (9 defs)

- [~] `q_shroom.cc:28` **quest_shroom_speak_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_shroom.cc:29` **quest_shroom_chat_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_shroom.cc:36` **quest_shroom_town_gen_hook**(void *, void *in_, void *) — map.rs::generate_shroom_field (map.rs:6659) + game.rs:10986
- [x] `q_shroom.cc:123` **quest_shroom_death_hook**(void *, void *in_, void *) — game.rs::check_plot_kill (dog kill fails the quest, game.rs:6631)
- [x] `q_shroom.cc:144` **quest_shroom_give_hook**(void *, void *in_, void *) — synced from report
- [x] `q_shroom.cc:240` **check_dogs_alive**(s32b m_idx) — input.rs:5794 (`dogs_alive` + fail messages)
- [~] `q_shroom.cc:265` **quest_shroom_speak_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [~] `q_shroom.cc:288` **quest_shroom_chat_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_shroom.cc:318` **quest_shroom_init_hook**() — synced from report

## q_spider.cc (4 defs)

- [x] `q_spider.cc:20` **quest_spider_gen_hook**(void *, void *, void *)
- [x] `q_spider.cc:53` **quest_spider_death_hook**(void *, void *, void *) — synced from report
- [x] `q_spider.cc:101` **quest_spider_finish_hook**(void *, void *in_, void *)
- [x] `q_spider.cc:131` **quest_spider_init_hook**()

## q_thief.cc (5 defs)

- [x] `q_thief.cc:27` **quest_thieves_gen_hook**(void *, void *in_, void *) — synced from report
- [x] `q_thief.cc:101` **quest_thieves_hook**(void *, void *, void *) — synced from report
- [x] `q_thief.cc:158` **quest_thieves_finish_hook**(void *, void *in_, void *) — synced from report
- [~] `q_thief.cc:199` **quest_thieves_feeling_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_thief.cc:215` **quest_thieves_init_hook**()

## q_thrain.cc (6 defs)

- [x] `q_thrain.cc:35` **quest_thrain_death_hook**(void *, void *in_, void *) — synced from report
- [x] `q_thrain.cc:144` **quest_thrain_gen_hook**(void *, void *in_, void *) — game.rs::embed_thrain (game.rs:6983, vault + mimic cells + gaglers)
- [~] `q_thrain.cc:215` **quest_thrain_feeling_hook**(void *, void *, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_thrain.cc:228` **quest_thrain_move_hook**(void *, void *in_, void *) — synced from report
- [x] `q_thrain.cc:259` **quest_thrain_turn_hook**(void *, void *, void *)
- [x] `q_thrain.cc:266` **quest_thrain_init_hook**()

## q_troll.cc (4 defs)

- [x] `q_troll.cc:30` **quest_troll_gen_hook**(void *, void *, void *) — game.rs::build_quest_level (trolls.map) + Tom/Glamdring at game.rs:8253
- [x] `q_troll.cc:123` **quest_troll_finish_hook**(void *, void *in_, void *)
- [x] `q_troll.cc:146` **quest_troll_death_hook**(void *, void *in_, void *) — synced from report
- [x] `q_troll.cc:203` **quest_troll_init_hook**()

## q_ultrae.cc (1 defs)

- [x] `q_ultrae.cc:5` **quest_ultra_evil_init_hook**()

## q_ultrag.cc (6 defs)

- [x] `q_ultrag.cc:34` **quest_ultra_good_move_hook**(void *, void *in_, void *)
- [x] `q_ultrag.cc:111` **quest_ultra_good_stair_hook**(void *, void *in_, void *)
- [x] `q_ultrag.cc:178` **quest_ultra_good_recall_hook**(void *, void *, void *)
- [x] `q_ultrag.cc:189` **quest_ultra_good_death_hook**(void *, void *in_, void *) — synced from report
- [~] `q_ultrag.cc:278` **quest_ultra_good_dump_hook**(void *, void *in_, void *) — [~] 任务 hook 由直接处理取代（quest/plot 状态机 + input.rs 对话；quests.ron 描述）；accept.py quest-hook 类机械复核
- [x] `q_ultrag.cc:306` **quest_ultra_good_init_hook**()

## q_wight.cc (4 defs)

- [x] `q_wight.cc:26` **quest_wight_gen_hook**(void *, void *, void *)
- [x] `q_wight.cc:134` **quest_wight_death_hook**(void *, void *in_, void *) — synced from report
- [x] `q_wight.cc:163` **quest_wight_finish_hook**(void *, void *in_, void *)
- [x] `q_wight.cc:186` **quest_wight_init_hook**()

## q_wolves.cc (4 defs)

- [x] `q_wolves.cc:25` **quest_wolves_gen_hook**(void *, void *in_, void *) — synced from report
- [x] `q_wolves.cc:99` **quest_wolves_death_hook**(void *, void *, void *) — synced from report
- [x] `q_wolves.cc:135` **quest_wolves_finish_hook**(void *, void *in_, void *)
- [x] `q_wolves.cc:154` **quest_wolves_init_hook**()

