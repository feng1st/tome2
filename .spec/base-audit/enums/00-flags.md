# Flag enumeration checklists

自动生成：`src/*_flag_list.hpp` 的全部 X-macro 旗标。

## feature_flag_list.hpp — 地形旗标 feature flags（19）

- [x] `FF_NO_WALK` (tier 1, idx 0) — parsed as TerrainDef field
- [x] `FF_NO_VISION` (tier 1, idx 1) — parsed as TerrainDef field
- [x] `FF_CAN_LEVITATE` (tier 1, idx 2) — parsed as TerrainDef field
- [x] `FF_CAN_PASS` (tier 1, idx 3) — parsed as TerrainDef field
- [x] `FF_FLOOR` (tier 1, idx 4) — parsed as TerrainDef field
- [x] `FF_WALL` (tier 1, idx 5) — parsed as TerrainDef field
- [x] `FF_PERMANENT` (tier 1, idx 6) — parsed as TerrainDef field
- [x] `FF_CAN_FLY` (tier 1, idx 7) — parsed as TerrainDef field
- [x] `FF_REMEMBER` (tier 1, idx 8) — parsed as TerrainDef field
- [x] `FF_NOTICE` (tier 1, idx 9) — parsed as TerrainDef field
- [x] `FF_DONT_NOTICE_RUNNING` (tier 1, idx 10) — parsed as TerrainDef field
- [x] `FF_CAN_RUN` (tier 1, idx 11) — parsed as TerrainDef field
- [x] `FF_DOOR` (tier 1, idx 12) — parsed as TerrainDef field
- [x] `FF_SUPPORT_LIGHT` (tier 1, idx 13) — parsed as TerrainDef field
- [x] `FF_CAN_CLIMB` (tier 1, idx 14) — parsed as TerrainDef field
- [x] `FF_TUNNELABLE` (tier 1, idx 15) — parsed as TerrainDef field
- [x] `FF_WEB` (tier 1, idx 16) — parsed as TerrainDef field
- [x] `FF_ATTR_MULTI` (tier 1, idx 17) — parsed as TerrainDef field
- [x] `FF_SUPPORT_GROWTH` (tier 1, idx 18) — parsed as TerrainDef field

## monster_race_flag_list.hpp — 怪物种族旗标（141）

- [x] `RF_UNIQUE` (tier 1, idx 0) — consumed in bevy/src
- [~] `RF_QUESTOR` (tier 1, idx 1) — C++ has no semantic use either
- [x] `RF_MALE` (tier 1, idx 2) — consumed in bevy/src
- [x] `RF_FEMALE` (tier 1, idx 3) — consumed in bevy/src
- [~] `RF_CHAR_CLEAR` (tier 1, idx 4) — display-only: cave.cc:1174/1200 hides the glyph over floors; Bevy draws monsters as render.rs sprites
- [~] `RF_CHAR_MULTI` (tier 1, idx 5) — C++ has no semantic use either
- [~] `RF_ATTR_CLEAR` (tier 1, idx 6) — display-only: cave.cc:1174/1207 inherits the grid attr; render.rs concern
- [~] `RF_ATTR_MULTI` (tier 1, idx 7) — display-only shimmer (cave.cc:1152-1170) plus redraw hints (monster2.cc:2481, monster3.cc:235/310); render.rs concern
- [x] `RF_FORCE_DEPTH` (tier 1, idx 8) — consumed in bevy/src
- [x] `RF_FORCE_MAXHP` (tier 1, idx 9) — consumed in bevy/src
- [x] `RF_FORCE_SLEEP` (tier 1, idx 10) — consumed in bevy/src
- [~] `RF_FORCE_EXTRA` (tier 1, idx 11) — C++ has no semantic use either
- [~] `RF_FRIEND` (tier 1, idx 12) — r_info has 0 refs; the 4 `O:FRIEND` hits are re_info ego-monster flags (handled); C++ use is recall prose only (monster1.cc:349)
- [x] `RF_FRIENDS` (tier 1, idx 13) — consumed in bevy/src
- [x] `RF_ESCORT` (tier 1, idx 14) — consumed in bevy/src
- [x] `RF_ESCORTS` (tier 1, idx 15) — consumed in bevy/src
- [x] `RF_NEVER_BLOW` (tier 1, idx 16) — consumed in bevy/src
- [x] `RF_NEVER_MOVE` (tier 1, idx 17) — consumed in bevy/src
- [x] `RF_RAND_25` (tier 1, idx 18) — consumed in bevy/src
- [x] `RF_RAND_50` (tier 1, idx 19) — consumed in bevy/src
- [x] `RF_ONLY_GOLD` (tier 1, idx 20) — consumed in bevy/src
- [x] `RF_ONLY_ITEM` (tier 1, idx 21) — consumed in bevy/src
- [x] `RF_DROP_60` (tier 1, idx 22) — consumed in bevy/src
- [x] `RF_DROP_90` (tier 1, idx 23) — consumed in bevy/src
- [x] `RF_DROP_1D2` (tier 1, idx 24) — consumed in bevy/src
- [x] `RF_DROP_2D2` (tier 1, idx 25) — consumed in bevy/src
- [x] `RF_DROP_3D2` (tier 1, idx 26) — consumed in bevy/src
- [x] `RF_DROP_4D2` (tier 1, idx 27) — consumed in bevy/src
- [x] `RF_DROP_GOOD` (tier 1, idx 28) — consumed in bevy/src
- [x] `RF_DROP_GREAT` (tier 1, idx 29) — consumed in bevy/src
- [~] `RF_DROP_USEFUL` (tier 1, idx 30) — C++ has no semantic use either
- [x] `RF_DROP_CHOSEN` (tier 1, idx 31) — consumed in bevy/src
- [x] `RF_STUPID` (tier 2, idx 0) — consumed in bevy/src
- [x] `RF_SMART` (tier 2, idx 1) — consumed in bevy/src
- [x] `RF_CAN_SPEAK` (tier 2, idx 2) — consumed in bevy/src
- [x] `RF_REFLECTING` (tier 2, idx 3) — consumed in bevy/src
- [x] `RF_INVISIBLE` (tier 2, idx 4) — consumed in bevy/src
- [x] `RF_COLD_BLOOD` (tier 2, idx 5) — consumed in bevy/src
- [x] `RF_EMPTY_MIND` (tier 2, idx 6) — consumed in bevy/src
- [x] `RF_WEIRD_MIND` (tier 2, idx 7) — consumed in bevy/src
- [x] `RF_DEATH_ORB` (tier 2, idx 8) — consumed in bevy/src
- [x] `RF_REGENERATE` (tier 2, idx 9) — consumed in bevy/src
- [x] `RF_SHAPECHANGER` (tier 2, idx 10) — consumed via dungeons.ron M: mflag rules (monster_ok_in_dungeon generic has()); C++ probe code is commented out
- [~] `RF_ATTR_ANY` (tier 2, idx 11) — display-only: cave.cc:1163 picks a random attr each frame; render.rs concern
- [x] `RF_POWERFUL` (tier 2, idx 12) — consumed in bevy/src
- [x] `RF_ELDRITCH_HORROR` (tier 2, idx 13) — consumed in bevy/src
- [x] `RF_AURA_FIRE` (tier 2, idx 14) — consumed in bevy/src
- [x] `RF_AURA_ELEC` (tier 2, idx 15) — consumed in bevy/src
- [x] `RF_OPEN_DOOR` (tier 2, idx 16) — consumed in bevy/src
- [x] `RF_BASH_DOOR` (tier 2, idx 17) — consumed in bevy/src
- [x] `RF_PASS_WALL` (tier 2, idx 18) — consumed in bevy/src
- [x] `RF_KILL_WALL` (tier 2, idx 19) — consumed in bevy/src
- [x] `RF_MOVE_BODY` (tier 2, idx 20) — consumed in bevy/src
- [x] `RF_KILL_BODY` (tier 2, idx 21) — consumed in bevy/src
- [x] `RF_TAKE_ITEM` (tier 2, idx 22) — consumed in bevy/src
- [x] `RF_KILL_ITEM` (tier 2, idx 23) — consumed in bevy/src
- [~] `RF_BRAIN_1` (tier 2, idx 24) — C++ has no semantic use either
- [~] `RF_BRAIN_2` (tier 2, idx 25) — C++ has no semantic use either
- [~] `RF_BRAIN_3` (tier 2, idx 26) — C++ has no semantic use either
- [~] `RF_BRAIN_4` (tier 2, idx 27) — C++ has no semantic use either
- [~] `RF_BRAIN_5` (tier 2, idx 28) — C++ has no semantic use either
- [~] `RF_BRAIN_6` (tier 2, idx 29) — C++ has no semantic use either
- [~] `RF_BRAIN_7` (tier 2, idx 30) — C++ has no semantic use either
- [~] `RF_BRAIN_8` (tier 2, idx 31) — C++ has no semantic use either
- [x] `RF_ORC` (tier 3, idx 0) — consumed in bevy/src
- [x] `RF_TROLL` (tier 3, idx 1) — consumed in bevy/src
- [x] `RF_GIANT` (tier 3, idx 2) — consumed in bevy/src
- [x] `RF_DRAGON` (tier 3, idx 3) — consumed in bevy/src
- [x] `RF_DEMON` (tier 3, idx 4) — consumed in bevy/src
- [x] `RF_UNDEAD` (tier 3, idx 5) — consumed in bevy/src
- [x] `RF_EVIL` (tier 3, idx 6) — consumed in bevy/src
- [x] `RF_ANIMAL` (tier 3, idx 7) — consumed in bevy/src
- [x] `RF_THUNDERLORD` (tier 3, idx 8) — consumed in bevy/src
- [x] `RF_GOOD` (tier 3, idx 9) — consumed in bevy/src
- [x] `RF_AURA_COLD` (tier 3, idx 10) — consumed in bevy/src
- [x] `RF_NONLIVING` (tier 3, idx 11) — consumed in bevy/src
- [x] `RF_HURT_LITE` (tier 3, idx 12) — consumed in bevy/src
- [x] `RF_HURT_ROCK` (tier 3, idx 13) — consumed in bevy/src
- [x] `RF_SUSCEP_FIRE` (tier 3, idx 14) — consumed in bevy/src
- [x] `RF_SUSCEP_COLD` (tier 3, idx 15) — consumed in bevy/src
- [x] `RF_IM_ACID` (tier 3, idx 16) — consumed in bevy/src
- [x] `RF_IM_ELEC` (tier 3, idx 17) — consumed in bevy/src
- [x] `RF_IM_FIRE` (tier 3, idx 18) — consumed in bevy/src
- [x] `RF_IM_COLD` (tier 3, idx 19) — consumed in bevy/src
- [x] `RF_IM_POIS` (tier 3, idx 20) — consumed in bevy/src
- [x] `RF_RES_TELE` (tier 3, idx 21) — consumed in bevy/src
- [x] `RF_RES_NETH` (tier 3, idx 22) — consumed in bevy/src
- [x] `RF_RES_WATE` (tier 3, idx 23) — consumed in bevy/src
- [x] `RF_RES_PLAS` (tier 3, idx 24) — consumed in bevy/src
- [x] `RF_RES_NEXU` (tier 3, idx 25) — consumed in bevy/src
- [x] `RF_RES_DISE` (tier 3, idx 26) — consumed in bevy/src
- [x] `RF_NO_FEAR` (tier 3, idx 28) — consumed in bevy/src
- [x] `RF_NO_STUN` (tier 3, idx 29) — consumed in bevy/src
- [x] `RF_NO_CONF` (tier 3, idx 30) — consumed in bevy/src
- [x] `RF_NO_SLEEP` (tier 3, idx 31) — consumed in bevy/src
- [x] `RF_AQUATIC` (tier 4, idx 0) — consumed in bevy/src
- [x] `RF_CAN_SWIM` (tier 4, idx 1) — consumed in bevy/src
- [x] `RF_CAN_FLY` (tier 4, idx 2) — consumed in bevy/src
- [x] `RF_FRIENDLY` (tier 4, idx 3) — consumed in bevy/src
- [x] `RF_PET` (tier 4, idx 4) — consumed in bevy/src
- [x] `RF_MORTAL` (tier 4, idx 5) — consumed in bevy/src
- [x] `RF_SPIDER` (tier 4, idx 6) — consumed in bevy/src
- [x] `RF_NAZGUL` (tier 4, idx 7) — consumed in bevy/src
- [x] `RF_DG_CURSE` (tier 4, idx 8) — consumed in bevy/src
- [x] `RF_POSSESSOR` (tier 4, idx 9) — consumed in bevy/src
- [x] `RF_NO_DEATH` (tier 4, idx 10) — consumed in bevy/src
- [x] `RF_NO_TARGET` (tier 4, idx 11) — consumed in bevy/src
- [x] `RF_AI_ANNOY` (tier 4, idx 12) — consumed in bevy/src
- [x] `RF_AI_SPECIAL` (tier 4, idx 13) — consumed in bevy/src
- [x] `RF_NEUTRAL` (tier 4, idx 14) — consumed in bevy/src
- [x] `RF_DROP_RANDART` (tier 4, idx 16) — consumed in bevy/src
- [x] `RF_AI_PLAYER` (tier 4, idx 17) — consumed in bevy/src
- [~] `RF_NO_THEFT` (tier 4, idx 18) — 0 refs in r_info.txt; the only consumer (cmd2.cc:4010 steal guard) is unreachable for unflagged monsters
- [x] `RF_SPIRIT` (tier 4, idx 19) — consumed in bevy/src
- [x] `RF_WILD_ONLY` (tier 5, idx 0) — consumed in bevy/src
- [x] `RF_WILD_TOWN` (tier 5, idx 1) — consumed in bevy/src
- [x] `RF_WILD_SHORE` (tier 5, idx 3) — consumed in bevy/src
- [x] `RF_WILD_OCEAN` (tier 5, idx 4) — consumed in bevy/src
- [x] `RF_WILD_WASTE` (tier 5, idx 5) — consumed in bevy/src
- [x] `RF_WILD_WOOD` (tier 5, idx 6) — consumed in bevy/src
- [x] `RF_WILD_VOLCANO` (tier 5, idx 7) — consumed in bevy/src
- [x] `RF_WILD_MOUNTAIN` (tier 5, idx 9) — consumed in bevy/src
- [x] `RF_WILD_GRASS` (tier 5, idx 10) — consumed in bevy/src
- [x] `RF_NO_CUT` (tier 5, idx 11) — consumed in bevy/src
- [x] `RF_JOKEANGBAND` (tier 5, idx 15) — consumed in bevy/src
- [~] `RF_WILD_TOO` (tier 5, idx 31) — C++ has no semantic use either
- [x] `RF_DROP_CORPSE` (tier 6, idx 0) — consumed in bevy/src
- [x] `RF_DROP_SKELETON` (tier 6, idx 1) — consumed in bevy/src
- [x] `RF_HAS_LITE` (tier 6, idx 2) — consumed in bevy/src
- [x] `RF_MIMIC` (tier 6, idx 3) — consumed in bevy/src
- [x] `RF_HAS_EGG` (tier 6, idx 4) — consumed in bevy/src
- [x] `RF_IMPRESED` (tier 6, idx 5) — consumed in bevy/src
- [x] `RF_SUSCEP_ACID` (tier 6, idx 6) — consumed in bevy/src
- [x] `RF_SUSCEP_ELEC` (tier 6, idx 7) — consumed in bevy/src
- [x] `RF_SUSCEP_POIS` (tier 6, idx 8) — consumed in bevy/src
- [x] `RF_KILL_TREES` (tier 6, idx 9) — consumed in bevy/src
- [x] `RF_WYRM_PROTECT` (tier 6, idx 10) — consumed in bevy/src
- [x] `RF_DOPPLEGANGER` (tier 6, idx 11) — consumed in bevy/src
- [x] `RF_ONLY_DEPTH` (tier 6, idx 12) — consumed in bevy/src
- [x] `RF_SPECIAL_GENE` (tier 6, idx 13) — consumed in bevy/src
- [x] `RF_NEVER_GENE` (tier 6, idx 14) — consumed in bevy/src

## monster_spell_flag_list.hpp — 怪物法术旗标（0）


## object_flag_list.hpp — 物品旗标（0）


## ego_flag_list.hpp — ego/artifact 旗标（0）


## player_race_flag_list.hpp — 种族/职业玩家旗标（0）


## skill_flag_list.hpp — 技能旗标（3）

- [x] `SKF_HIDDEN` (tier 1, idx 0) — consumed in bevy/src
- [x] `SKF_AUTO_HIDE` (tier 1, idx 1) — consumed in bevy/src
- [x] `SKF_RANDOM_GAIN` (tier 1, idx 2) — consumed in bevy/src

## store_flag_list.hpp — 商店旗标（11）

- [x] `STF_DEPEND_LEVEL` (tier 1, idx 0) — consumed in bevy/src
- [x] `STF_SHALLOW_LEVEL` (tier 1, idx 1) — consumed in bevy/src
- [x] `STF_MEDIUM_LEVEL` (tier 1, idx 2) — consumed in bevy/src
- [x] `STF_DEEP_LEVEL` (tier 1, idx 3) — consumed in bevy/src
- [x] `STF_RARE` (tier 1, idx 4) — consumed in bevy/src
- [x] `STF_VERY_RARE` (tier 1, idx 5) — consumed in bevy/src
- [x] `STF_COMMON` (tier 1, idx 6) — consumed in bevy/src
- [x] `STF_ALL_ITEM` (tier 1, idx 7) — consumed in bevy/src
- [x] `STF_RANDOM` (tier 1, idx 8) — consumed in bevy/src
- [x] `STF_FORCE_LEVEL` (tier 1, idx 9) — consumed in bevy/src
- [x] `STF_MUSEUM` (tier 1, idx 10) — consumed in bevy/src

## dungeon_flag_list.hpp — 地城旗标（47）

- [x] `DF_PRINCIPAL` (tier 1, idx 0) — consumed in bevy/src
- [~] `DF_MAZE` (tier 1, idx 1) — C++ has no semantic use either
- [x] `DF_SMALLEST` (tier 1, idx 2) — consumed in bevy/src
- [x] `DF_SMALL` (tier 1, idx 3) — consumed in bevy/src
- [x] `DF_BIG` (tier 1, idx 4) — consumed in bevy/src
- [x] `DF_NO_DOORS` (tier 1, idx 5) — consumed in bevy/src
- [x] `DF_WATER_RIVER` (tier 1, idx 6) — consumed in bevy/src
- [x] `DF_LAVA_RIVER` (tier 1, idx 7) — consumed in bevy/src
- [x] `DF_WATER_RIVERS` (tier 1, idx 8) — consumed in bevy/src
- [x] `DF_LAVA_RIVERS` (tier 1, idx 9) — consumed in bevy/src
- [x] `DF_CAVE` (tier 1, idx 10) — consumed in bevy/src
- [x] `DF_CAVERN` (tier 1, idx 11) — consumed in bevy/src
- [x] `DF_NO_UP` (tier 1, idx 12) — consumed in bevy/src
- [~] `DF_HOT` (tier 1, idx 13) — 0 refs in d_info.txt; only object decay (dungeon.cc:2446/2566)
- [x] `DF_COLD` (tier 1, idx 14) — consumed in bevy/src
- [x] `DF_FORCE_DOWN` (tier 1, idx 15) — consumed in bevy/src
- [x] `DF_FORGET` (tier 1, idx 16) — consumed in bevy/src
- [x] `DF_NO_DESTROY` (tier 1, idx 17) — consumed in bevy/src
- [x] `DF_SAND_VEIN` (tier 1, idx 18) — consumed in bevy/src
- [x] `DF_CIRCULAR_ROOMS` (tier 1, idx 19) — consumed in bevy/src
- [x] `DF_EMPTY` (tier 1, idx 20) — consumed in bevy/src
- [x] `DF_DAMAGE_FEAT` (tier 1, idx 21) — consumed in bevy/src
- [x] `DF_FLAT` (tier 1, idx 22) — consumed in bevy/src
- [~] `DF_TOWER` (tier 1, idx 23) — 0 refs in d_info.txt; set at runtime for the Lost Temple (q_god.cc:473/531/792) and only read by the HUD depth label ("-%d", xtra1.cc:580)
- [x] `DF_RANDOM_TOWNS` (tier 1, idx 24) — consumed in bevy/src
- [x] `DF_DOUBLE` (tier 1, idx 25) — consumed in bevy/src
- [~] `DF_LIFE_LEVEL` (tier 1, idx 26) — C++ has no semantic use either
- [x] `DF_EVOLVE` (tier 1, idx 27) — consumed in bevy/src
- [x] `DF_ADJUST_LEVEL_1` (tier 1, idx 28) — consumed in bevy/src
- [x] `DF_ADJUST_LEVEL_2` (tier 1, idx 29) — consumed in bevy/src
- [x] `DF_NO_RECALL` (tier 1, idx 30) — consumed in bevy/src
- [x] `DF_NO_STREAMERS` (tier 1, idx 31) — consumed in bevy/src
- [x] `DF_ADJUST_LEVEL_1_2` (tier 2, idx 0) — consumed in bevy/src
- [x] `DF_NO_SHAFT` (tier 2, idx 1) — consumed in bevy/src
- [~] `DF_ADJUST_LEVEL_PLAYER` (tier 2, idx 2) — 0 refs in d_info.txt; set by q_god.cc but never read anywhere in C++ (dead)
- [x] `DF_NO_TELEPORT` (tier 2, idx 3) — consumed in bevy/src
- [x] `DF_ASK_LEAVE` (tier 2, idx 4) — consumed in bevy/src
- [x] `DF_NO_STAIR` (tier 2, idx 5) — consumed in bevy/src
- [~] `DF_SPECIAL` (tier 2, idx 6) — only HUD depth label "Special" (xtra1.cc:557), the fate-feeling gate (cmd4.cc:2667) and stair_creation (spells2.cc:1431); stair_creation is unreachable (sole caller cmd6.cc:4880 ACT_ANGUIREL, no a_info activation uses it and Rust Anguirel has activate:"")
- [x] `DF_NO_NEW_MONSTER` (tier 2, idx 7) — consumed in bevy/src
- [x] `DF_NO_GENO` (tier 2, idx 9) — consumed in bevy/src
- [x] `DF_NO_BREATH` (tier 2, idx 10) — consumed in bevy/src
- [x] `DF_WATER_BREATH` (tier 2, idx 11) — consumed in bevy/src
- [x] `DF_ELVEN` (tier 2, idx 12) — themed town ego: game::roll_monster_ego applies only "Elven" in ELVEN towns (map.mflag from t_*.txt `f:ELVEN`, game.rs:9355-9368); test `themed_town_monster_ego`
- [x] `DF_DWARVEN` (tier 2, idx 13) — same path selects "Dwarven" (game.rs:9355-9368); Khazad-dum mflag DWARVEN
- [x] `DF_NO_EASY_MOVE` (tier 2, idx 14) — consumed in bevy/src
- [x] `DF_NO_RECALL_OUT` (tier 2, idx 15) — consumed in bevy/src

