# Method inventory: 03-monsters

## melee1.cc (6 defs)

- [x] `melee1.cc:46` **monster_critical**(int dice, int sides, int dam) — Critical blow. All hits that do 95% of total possible damage, and which also do at least 20 damage, or, sometimes, N damage. This is used only to determine "cuts" and "stuns".
- [x] `melee1.cc:84` **check_hit**(int power, int level) — Determine if a monster attack against the player succeeds. Always miss 5% of the time, Always hit 5% of the time. Otherwise, match monster power against player armor. — synced from report
- [x] `melee1.cc:146` **get_attack_power**(int effect) — Get the "power" of an attack of given effect type. — synced from report
- [x] `melee1.cc:226` **carried_make_attack_normal**(int r_idx) — Attack the player via physical attacks. — synced from report
- [x] `melee1.cc:1186` **black_breath_attack**(int chance) — Give unprotected player the Black Breath with a 1 in (chance) probability
- [x] `melee1.cc:1199` **make_attack_normal**(int m_idx, byte divis) — Attack the player via physical attacks. — synced from report

## melee2.cc (33 defs)

- [x] `melee2.cc:71` **mon_take_hit_mon**(int s_idx, int m_idx, int dam, const char *note) — Based on mon_take_hit... all monster attacks on other monsters should use — synced from report
- [x] `melee2.cc:220` **mon_handle_fear**(monster_type *m_ptr, int dam, bool *fear) — synced from report
- [x] `melee2.cc:313` **int_outof**(std::shared_ptr<monster_race> r_ptr, int prob) — Internal probability routine
- [x] `melee2.cc:327` **remove_bad_spells**(int m_idx, monster_spell_flag_set *spells_p) — Remove the "bad" spells from a spell list — synced from report
- [x] `melee2.cc:560` **summon_possible**(int y1, int x1) — Determine if there is a space near the player in which a summoned creature can appear
- [x] `melee2.cc:598` **clean_shot**(int y1, int x1, int y2, int x2) — Determine if a bolt spell will hit the player. This is exactly like "projectable", but it will return false if a monster is in the way.
- [x] `melee2.cc:634` **bolt**(int m_idx, int typ, int dam_hp) — Cast a bolt at the player Stop if we hit a monster Affect monsters and the player — synced from report
- [x] `melee2.cc:646` **compute_bolt_mask**() — Calculate the mask for "bolt" spells
- [x] `melee2.cc:663` **compute_summoning_mask**() — Calculate mask for summoning spells
- [x] `melee2.cc:680` **compute_smart_mask**() — Calculate mask for spells requiring SMART flag
- [x] `melee2.cc:697` **compute_innate_mask**() — Calculate mask for spells requiring SMART flag
- [x] `melee2.cc:858` **breath**(int m_idx, int typ, int dam_hp, int rad) — Cast a breath (or ball) attack at the player Pass over any monsters that may be in the way Affect grids, objects, monsters, and the player — synced from report
- [x] `melee2.cc:878` **monst_breath_monst**(int m_idx, int y, int x, int typ, int dam_hp, int rad) — Monster casts a breath (or ball) attack at another monster. Pass over any monsters that may be in the way Affect grids, objects, monsters, and the player — synced from report
- [x] `melee2.cc:897` **monst_bolt_monst**(int m_idx, int y, int x, int typ, int dam_hp) — Monster casts a bolt at another monster Stop if we hit a monster Affect monsters and the player — synced from report
- [x] `melee2.cc:905` **monster_msg**(const char *fmt, ...)
- [x] `melee2.cc:924` **monster_msg_simple**(const char *s)
- [x] `melee2.cc:965` **monst_spell_monst**(int m_idx)
- [x] `melee2.cc:2322` **curse_equipment**(int chance, int heavy_chance) — synced from report
- [x] `melee2.cc:2377` **curse_equipment_dg**(int chance, int heavy_chance) — synced from report
- [x] `melee2.cc:2474` **make_attack_spell**(int m_idx) — Creatures can cast spells, shoot missiles, and breathe. Returns "true" if a spell (or whatever) was (successfully) cast. XXX XXX XXX This function could use some work, but remember to keep it as optim — synced from report
- [x] `melee2.cc:3834` **mon_will_run**(int m_idx) — Returns whether a given monster will try to run from the player. Monsters will attempt to avoid very powerful players. See below. Because this function is called so often, little details are important — synced from report
- [~] `melee2.cc:3916` **get_fear_moves_aux**(int m_idx, int *yp, int *xp) — Provide a location to flee to, but give the player a wide berth. A monster may wish to flee to a location that is behind the player, but instead of heading directly for it, the monster should "swerve" — n/a per report — synced from report
- [x] `melee2.cc:4012` **find_safety**(int m_idx, int *yp, int *xp) — Choose a "safe" location near a monster for it to run toward. A location is "safe" if it can be reached quickly and the player is not able to fire into it (it isn't a "clean shot"). So, this will caus — synced from report
- [x] `melee2.cc:4091` **find_hiding**(int m_idx, int *yp, int *xp) — Choose a good hiding place near a monster for it to run toward. Pack monsters will use this to "ambush" the player and lure him out of corridors into open space so they can swarm him. Return true if a — synced from report
- [x] `melee2.cc:4156` **find_corpse**(monster_type *m_ptr, int *y, int *x) — Find an appropriate corpse — synced from report
- [x] `melee2.cc:4225` **get_target_monster**(int m_idx) — Choose target
- [x] `melee2.cc:4265` **get_moves**(int m_idx, int *mm) — Choose "logical" directions for monster movement — game.rs::move_priority ports the mm[] direction-priority table (move_val + diamond-maneuver prevention, melee2.cc:4470-4610, game.rs:435); monster_turns uses it and the four `ddd[]` random tries for RAND_*/confused (melee2.cc:5599-5635, game.rs:15093); test `get_moves_priority_table` (game.rs:20481).
- [x] `melee2.cc:4638` **check_hit2**(int power, int level, int ac)
- [x] `melee2.cc:4660` **monst_attack_monst**(int m_idx, int t_idx) — Monster attacks monster — synced from report
- [x] `melee2.cc:5188` **player_invis**(monster_type * m_ptr) — Determine whether the player is invisible to a monster — synced from report
- [x] `melee2.cc:5249` **process_monster**(int m_idx) — Process a monster The monster is known to be within 100 grids of the player In several cases, we directly update the monster lore Note that a monster is only allowed to "reproduce" if there are a limi — synced from report
- [x] `melee2.cc:6301` **summon_maint**(int m_idx) — synced from report
- [x] `melee2.cc:6373` **process_monsters**() — Process all the "live" monsters, once per game turn. During each game turn, we scan through the list of all the "live" monsters, (backwards, so we can excise any "freshly dead" monsters), energizing e — synced from report

## monster1.cc (22 defs)

- [x] `monster1.cc:53` **roff_aux**(std::shared_ptr<monster_race const> r_ptr) — bevy/src/recall.rs:88 roff_aux（怪物1.cc:53 全段移植：杀伤/描述/地点/速度/经验/灵光/护卫/先天攻击/吐息/法术/护甲生命/特殊能力/抗性/感知/掉落/blow 描述）；测试 roff_aux_* 6 个
- [x] `monster1.cc:1262` **roff_name**(int r_idx, int ego) — bevy/src/recall.rs:72 roff_name（"The <name> ('c')/('c'):"）；测试 roff_name_uses_the_original_format
- [x] `monster1.cc:1313` **roff_top**(int r_idx, int ego) — bevy/src/recall.rs:84 roff_top（顶部名字行）
- [x] `monster1.cc:1327` **screen_roff**(int r_idx, int ego) — bevy/src/recall.rs:427 screen_roff（与 monster_description_out 同屏显示；Bevy 单一文本模态承载）
- [x] `monster1.cc:1347` **monster_description_out**(int r_idx, int ego) — bevy/src/recall.rs:414 monster_description_out（名字+换行后的正文，78 列折行）
- [x] `monster1.cc:1357` **display_roff**(int r_idx, int ego) — bevy/src/recall.rs:432 display_roff（同文本页）
- [x] `monster1.cc:1382` **monster_quest**(monster_race const *r_ptr)
- [x] `monster1.cc:1396` **monster_dungeon**(const monster_race *r_ptr) — synced from report
- [x] `monster1.cc:1401` **monster_ocean**(monster_race const *r_ptr)
- [x] `monster1.cc:1406` **monster_shore**(monster_race const *r_ptr)
- [x] `monster1.cc:1411` **monster_waste**(monster_race const *r_ptr)
- [x] `monster1.cc:1416` **monster_town**(monster_race const *r_ptr)
- [x] `monster1.cc:1421` **monster_wood**(monster_race const *r_ptr)
- [x] `monster1.cc:1426` **monster_volcano**(monster_race const *r_ptr)
- [x] `monster1.cc:1431` **monster_mountain**(monster_race const *r_ptr)
- [x] `monster1.cc:1436` **monster_grass**(monster_race const *r_ptr)
- [x] `monster1.cc:1441` **monster_deep_water**(monster_race const *r_ptr) — synced from report
- [x] `monster1.cc:1451` **monster_shallow_water**(monster_race const *r_ptr) — synced from report
- [x] `monster1.cc:1461` **monster_lava**(monster_race const *r_ptr) — synced from report
- [x] `monster1.cc:1474` **reset_get_monster_hook**()
- [x] `monster1.cc:1523` **monster_can_cross_terrain**(byte feat, std::shared_ptr<monster_race> r_ptr) — Check if monster can cross terrain — synced from report
- [x] `monster1.cc:1555` **set_monster_aux_hook**(int y, int x) — synced from report

## monster2.cc (42 defs)

- [x] `monster2.cc:61` **monster_exp**(s16b level) — synced from report
- [x] `monster2.cc:67` **monster_check_experience**(int m_idx, bool silent) — synced from report
- [x] `monster2.cc:126` **monster_gain_exp**(int m_idx, u32b exp) — synced from report
- [x] `monster2.cc:141` **monster_set_level**(int m_idx, int level) — synced from report
- [x] `monster2.cc:158` **modify_aux**(s32b a, s32b b, char mod) — Will add, sub, ..
- [x] `monster2.cc:181` **mego_ok**(monster_race const *r_ptr, int ego) — Is this ego ok for this monster ?
- [x] `monster2.cc:234` **pick_ego_monster**(monster_race const *r_ptr) — Choose an ego type
- [x] `monster2.cc:455` **delete_monster_idx**(int i) — Delete a monster by index. When a monster is deleted, all of its objects are deleted. — synced from report
- [x] `monster2.cc:552` **delete_monster**(int y, int x) — Delete the monster, if any, at a given location — synced from report
- [~] `monster2.cc:570` **compact_monsters_aux**(int i1, int i2) — C++ 手工 m_list 搬移；port ECS 负责（离开层 game.rs:19554 cleanup_level despawn）
- [~] `monster2.cc:641` **compact_monsters**(int size) — C++ 手工 m_list 压缩；port ECS 负责怪物存储（game.rs:19554 cleanup_level）
- [~] `monster2.cc:719` **wipe_m_list**() — C++ 手工清空 m_list；port 离层 despawn（game.rs:19554 cleanup_level）
- [~] `monster2.cc:767` **m_pop**() — C++ m_list 空闲槽分配；port ECS 实体分配替代
- [x] `monster2.cc:821` **get_mon_num_prep**() — Apply a "monster restriction function" to the "monster allocation table"
- [x] `monster2.cc:851` **apply_rule**(monster_race const *r_ptr, byte rule) — Some dungeon types restrict the possible monsters. Return true is the monster is OK and false otherwise
- [x] `monster2.cc:901` **restrict_monster_to_dungeon**(int r_idx)
- [x] `monster2.cc:950` **get_mon_num**(int level) — Choose a monster race that seems "appropriate" to the given level This function uses the "prob2" field of the "monster allocation table", and various local information, to calculate the "prob3" field  — synced from report
- [~] `monster2.cc:1164` **monster_desc**(char *desc, monster_type const *m_ptr, int mode) — Build a string describing a monster in some way. We can correctly describe monsters based on their visibility. We can force all monsters to be treated as visible or invisible. We can build nominatives — n/a per report — synced from report
- [x] `monster2.cc:1381` **monster_race_desc**(char *desc, int r_idx, int ego) — ego names precomputed by data::apply_monster_ego (data.rs:1489); article/unique handling in game::monster_desc/killed_monster_desc (game.rs:5456/5427)
- [x] `monster2.cc:1426` **sanity_blast**(monster_type * m_ptr, bool necro) — synced from report
- [x] `monster2.cc:1621` **update_mon**(int m_idx, bool full) — This function updates the monster record of the given monster This involves extracting the distance to the player, checking for visibility (natural, infravision, see-invis, telepathy), updating the mo — synced from report
- [x] `monster2.cc:1867` **update_monsters**(bool full) — This function simply updates all the (non-dead) monsters (see above).
- [x] `monster2.cc:1885` **monster_carry**(monster_type *m_ptr, int m_idx, object_type *q_ptr) — synced from report
- [x] `monster2.cc:1952` **kind_is_randart**(object_kind const *k_ptr)
- [~] `monster2.cc:1993` **place_monster_one**(int y, int x, int r_idx, int ego, bool slp, int status) — n/a per report — synced from report
- [x] `monster2.cc:2518` **place_monster_group**(int y, int x, int r_idx, bool slp, int status) — Attempt to place a "group" of monsters around the given location — synced from report
- [x] `monster2.cc:2615` **place_monster_okay**(monster_race const *r_ptr) — Hack -- help pick an escort type — synced from report
- [x] `monster2.cc:2655` **place_monster_aux**(int y, int x, int r_idx, bool slp, bool grp, int status) — Attempt to place a monster of the given race at the given location Note that certain monsters are now marked as requiring "friends". These monsters, if successfully placed, and if the "grp" parameter  — synced from report
- [x] `monster2.cc:2751` **place_monster**(int y, int x, bool slp, bool grp) — Hack -- attempt to place a monster at the given location Attempt to find a monster appropriate to the "monster_level" — game::random_monster_of_level + marker spawn (game.rs:6948/9030) — synced from report
- [x] `monster2.cc:2781` **alloc_horde**(int y, int x) — synced from report
- [x] `monster2.cc:2849` **alloc_monster**(int dis, bool slp) — Attempt to allocate a random monster in the dungeon. Place the monster at least "dis" distance from the player. Use "slp" to choose the initial "sleep" status Use "monster_level" for the monster level — synced from report
- [x] `monster2.cc:2914` **summon_specific_okay**(monster_race const *r_ptr) — Hack -- help decide if a monster race is "okay" to summon — synced from report
- [x] `monster2.cc:3183` **summon_specific**(int y1, int x1, int lev, int type) — synced from report
- [x] `monster2.cc:3263` **summon_specific_friendly**(int y1, int x1, int lev, int type, bool Group_ok) — synced (ported)
- [x] `monster2.cc:3333` **monster_swap**(int y1, int x1, int y2, int x2) — Swap the players/monsters (if any) at two locations XXX XXX XXX
- [x] `monster2.cc:3437` **mutate_monster_okay**(monster_race const *r_ptr) — Hack -- help decide if a monster race is "okay" to summon — inlined in game::ai_multiply_monster (game.rs:17685-17699)
- [x] `monster2.cc:3452` **multiply_monster**(int m_idx, bool charm, bool clone) — Let the given monster attempt to reproduce. Note that "reproduction" REQUIRES empty space. — synced from report
- [x] `monster2.cc:3527` **message_pain_hook**(const char *message, const char *name) — synced from report
- [x] `monster2.cc:3544` **message_pain**(int m_idx, int dam) — synced from report
- [x] `monster2.cc:3651` **update_smart_learn**(int m_idx, int what) — Learn about an "observed" resistance. — synced from report
- [x] `monster2.cc:3777` **player_place**(int y, int x) — Place the player in the dungeon XXX XXX — synced from report
- [x] `monster2.cc:3793` **monster_drop_carried_objects**(monster_type *m_ptr) — Drop all items carried by a monster

## monster3.cc (15 defs)

- [x] `monster3.cc:36` **is_friend**(monster_type const *m_ptr) — Is the mon,ster in friendly state(pet, friend, ..) -1 = enemy, 0 = neutral, 1 = friend
- [x] `monster3.cc:61` **is_enemy**(monster_type *m_ptr, monster_type *t_ptr) — Should they attack each others — synced from report
- [x] `monster3.cc:94` **change_side**(monster_type *m_ptr) — synced from report
- [x] `monster3.cc:125` **ai_multiply**(int m_idx) — Multiply !! — synced from report
- [x] `monster3.cc:164` **ai_possessor**(int m_idx, int o_idx) — Possessor incarnates — synced from report
- [x] `monster3.cc:247` **ai_deincarnate**(int m_idx) — synced from report
- [x] `monster3.cc:323` **can_create_companion**() — Returns if a new companion is allowed — synced from report
- [x] `monster3.cc:349` **do_control_walk**() — Player controlled monsters
- [x] `monster3.cc:373` **do_control_inven**()
- [x] `monster3.cc:389` **do_control_pickup**()
- [x] `monster3.cc:443` **do_control_drop**()
- [x] `monster3.cc:452` **do_control_magic**()
- [x] `monster3.cc:638` **do_control_reconnect**() — Finds the controlled monster and "reconnect" to it
- [x] `monster3.cc:662` **do_cmd_companion**() — Turns a simple pet into a faithful companion — synced from report
- [x] `monster3.cc:703` **dump_companions**(FILE *outfile) — bevy/src/game.rs:5673 dump_companions + modal Knowledge 页 4；测试 dump_companions_lists_name_and_level

## monster_spell.cc (0 defs)


## monster_type.cc (0 defs)


