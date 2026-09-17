# Mechanical coverage report

`DATA`=在 lib/edit 数据中出现过；`RUST`=在 bevy/src 代码中作为标识串出现。
RUST=0 且 DATA>0 的条目是**高优先级缺口候选**；DATA=0 为数据死条目（n/a）。

## FF: feature flags (feature_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|
| `NO_WALK` | 0 | 0 | data-dead |
| `NO_VISION` | 0 | 0 | data-dead |
| `CAN_LEVITATE` | 0 | 0 | data-dead |
| `CAN_PASS` | 0 | 0 | data-dead |
| `FLOOR` | 8 | 1 |  |
| `WALL` | 246 | 1 |  |
| `PERMANENT` | 0 | 0 | data-dead |
| `CAN_FLY` | 0 | 0 | data-dead |
| `REMEMBER` | 0 | 0 | data-dead |
| `NOTICE` | 1 | 17 |  |
| `DONT_NOTICE_RUNNING` | 0 | 0 | data-dead |
| `CAN_RUN` | 0 | 0 | data-dead |
| `DOOR` | 1458 | 0 | **GAP?** |
| `SUPPORT_LIGHT` | 0 | 0 | data-dead |
| `CAN_CLIMB` | 0 | 0 | data-dead |
| `TUNNELABLE` | 0 | 0 | data-dead |
| `WEB` | 370156 | 163 |  |
| `ATTR_MULTI` | 0 | 0 | data-dead |
| `SUPPORT_GROWTH` | 0 | 0 | data-dead |

## RF: monster race flags (monster_race_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|
| `UNIQUE` | 0 | 0 | data-dead |
| `QUESTOR` | 0 | 0 | data-dead |
| `MALE` | 1027 | 1 |  |
| `FEMALE` | 0 | 0 | data-dead |
| `CHAR_CLEAR` | 0 | 0 | data-dead |
| `CHAR_MULTI` | 0 | 0 | data-dead |
| `ATTR_CLEAR` | 0 | 0 | data-dead |
| `ATTR_MULTI` | 0 | 0 | data-dead |
| `FORCE_DEPTH` | 0 | 0 | data-dead |
| `FORCE_MAXHP` | 0 | 0 | data-dead |
| `FORCE_SLEEP` | 0 | 0 | data-dead |
| `FORCE_EXTRA` | 0 | 0 | data-dead |
| `FRIEND` | 0 | 0 | data-dead |
| `FRIENDS` | 0 | 0 | data-dead |
| `ESCORT` | 0 | 0 | data-dead |
| `ESCORTS` | 0 | 0 | data-dead |
| `NEVER_BLOW` | 0 | 0 | data-dead |
| `NEVER_MOVE` | 0 | 0 | data-dead |
| `RAND_25` | 0 | 0 | data-dead |
| `RAND_50` | 0 | 0 | data-dead |
| `ONLY_GOLD` | 0 | 0 | data-dead |
| `ONLY_ITEM` | 0 | 0 | data-dead |
| `DROP_60` | 0 | 0 | data-dead |
| `DROP_90` | 0 | 0 | data-dead |
| `DROP_1D2` | 0 | 0 | data-dead |
| `DROP_2D2` | 0 | 0 | data-dead |
| `DROP_3D2` | 0 | 0 | data-dead |
| `DROP_4D2` | 0 | 0 | data-dead |
| `DROP_GOOD` | 0 | 0 | data-dead |
| `DROP_GREAT` | 0 | 0 | data-dead |
| `DROP_USEFUL` | 0 | 0 | data-dead |
| `DROP_CHOSEN` | 0 | 0 | data-dead |
| `STUPID` | 0 | 0 | data-dead |
| `SMART` | 0 | 0 | data-dead |
| `CAN_SPEAK` | 0 | 0 | data-dead |
| `REFLECTING` | 0 | 0 | data-dead |
| `INVISIBLE` | 0 | 0 | data-dead |
| `COLD_BLOOD` | 0 | 0 | data-dead |
| `EMPTY_MIND` | 0 | 0 | data-dead |
| `WEIRD_MIND` | 0 | 0 | data-dead |
| `DEATH_ORB` | 0 | 0 | data-dead |
| `REGENERATE` | 0 | 0 | data-dead |
| `SHAPECHANGER` | 0 | 0 | data-dead |
| `ATTR_ANY` | 9 | 1 |  |
| `POWERFUL` | 0 | 0 | data-dead |
| `ELDRITCH_HORROR` | 0 | 0 | data-dead |
| `AURA_FIRE` | 0 | 0 | data-dead |
| `AURA_ELEC` | 0 | 0 | data-dead |
| `OPEN_DOOR` | 0 | 0 | data-dead |
| `BASH_DOOR` | 0 | 0 | data-dead |
| `PASS_WALL` | 0 | 0 | data-dead |
| `KILL_WALL` | 0 | 0 | data-dead |
| `MOVE_BODY` | 0 | 0 | data-dead |
| `KILL_BODY` | 0 | 0 | data-dead |
| `TAKE_ITEM` | 0 | 0 | data-dead |
| `KILL_ITEM` | 0 | 0 | data-dead |
| `BRAIN_1` | 0 | 0 | data-dead |
| `BRAIN_2` | 0 | 0 | data-dead |
| `BRAIN_3` | 0 | 0 | data-dead |
| `BRAIN_4` | 0 | 0 | data-dead |
| `BRAIN_5` | 0 | 0 | data-dead |
| `BRAIN_6` | 0 | 0 | data-dead |
| `BRAIN_7` | 0 | 0 | data-dead |
| `BRAIN_8` | 0 | 0 | data-dead |
| `ORC` | 370156 | 163 |  |
| `TROLL` | 9 | 0 | **GAP?** |
| `GIANT` | 0 | 0 | data-dead |
| `DRAGON` | 0 | 0 | data-dead |
| `DEMON` | 0 | 0 | data-dead |
| `UNDEAD` | 0 | 0 | data-dead |
| `EVIL` | 246 | 1 |  |
| `ANIMAL` | 0 | 0 | data-dead |
| `THUNDERLORD` | 0 | 0 | data-dead |
| `GOOD` | 6533 | 2 |  |
| `AURA_COLD` | 0 | 0 | data-dead |
| `NONLIVING` | 0 | 0 | data-dead |
| `HURT_LITE` | 0 | 0 | data-dead |
| `HURT_ROCK` | 0 | 0 | data-dead |
| `SUSCEP_FIRE` | 0 | 0 | data-dead |
| `SUSCEP_COLD` | 0 | 0 | data-dead |
| `IM_ACID` | 60 | 47 |  |
| `IM_ELEC` | 36 | 43 |  |
| `IM_FIRE` | 106 | 72 |  |
| `IM_COLD` | 39 | 48 |  |
| `IM_POIS` | 0 | 48 | data-dead |
| `RES_TELE` | 0 | 0 | data-dead |
| `RES_NETH` | 0 | 0 | data-dead |
| `RES_WATE` | 0 | 0 | data-dead |
| `RES_PLAS` | 0 | 0 | data-dead |
| `RES_NEXU` | 0 | 0 | data-dead |
| `RES_DISE` | 0 | 0 | data-dead |
| `NO_FEAR` | 0 | 9 | data-dead |
| `NO_STUN` | 0 | 1 | data-dead |
| `NO_CONF` | 145 | 42 |  |
| `NO_SLEEP` | 1 | 1 |  |
| `AQUATIC` | 0 | 0 | data-dead |
| `CAN_SWIM` | 0 | 0 | data-dead |
| `CAN_FLY` | 0 | 0 | data-dead |
| `FRIENDLY` | 0 | 0 | data-dead |
| `PET` | 370156 | 163 |  |
| `MORTAL` | 0 | 0 | data-dead |
| `SPIDER` | 0 | 0 | data-dead |
| `NAZGUL` | 0 | 0 | data-dead |
| `DG_CURSE` | 0 | 0 | data-dead |
| `POSSESSOR` | 0 | 0 | data-dead |
| `NO_DEATH` | 0 | 1 | data-dead |
| `NO_TARGET` | 0 | 0 | data-dead |
| `AI_ANNOY` | 0 | 0 | data-dead |
| `AI_SPECIAL` | 8 | 0 | **GAP?** |
| `NEUTRAL` | 0 | 0 | data-dead |
| `DROP_RANDART` | 0 | 0 | data-dead |
| `AI_PLAYER` | 1 | 0 | **GAP?** |
| `NO_THEFT` | 0 | 0 | data-dead |
| `SPIRIT` | 0 | 0 | data-dead |
| `WILD_ONLY` | 0 | 0 | data-dead |
| `WILD_TOWN` | 0 | 0 | data-dead |
| `WILD_SHORE` | 0 | 0 | data-dead |
| `WILD_OCEAN` | 0 | 0 | data-dead |
| `WILD_WASTE` | 0 | 0 | data-dead |
| `WILD_WOOD` | 0 | 0 | data-dead |
| `WILD_VOLCANO` | 0 | 0 | data-dead |
| `WILD_MOUNTAIN` | 0 | 0 | data-dead |
| `WILD_GRASS` | 0 | 0 | data-dead |
| `NO_CUT` | 0 | 0 | data-dead |
| `JOKEANGBAND` | 0 | 0 | data-dead |
| `WILD_TOO` | 0 | 0 | data-dead |
| `DROP_CORPSE` | 0 | 0 | data-dead |
| `DROP_SKELETON` | 0 | 0 | data-dead |
| `HAS_LITE` | 0 | 0 | data-dead |
| `MIMIC` | 0 | 0 | data-dead |
| `HAS_EGG` | 0 | 0 | data-dead |
| `IMPRESED` | 0 | 0 | data-dead |
| `SUSCEP_ACID` | 0 | 0 | data-dead |
| `SUSCEP_ELEC` | 0 | 0 | data-dead |
| `SUSCEP_POIS` | 0 | 0 | data-dead |
| `KILL_TREES` | 0 | 0 | data-dead |
| `WYRM_PROTECT` | 0 | 0 | data-dead |
| `DOPPLEGANGER` | 0 | 0 | data-dead |
| `ONLY_DEPTH` | 0 | 0 | data-dead |
| `SPECIAL_GENE` | 0 | 0 | data-dead |
| `NEVER_GENE` | 0 | 0 | data-dead |

## RSF: monster spell flags (monster_spell_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|

## OF: object flags (object_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|

## EF: ego flags (ego_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|

## PRF: player flags (player_race_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|

## SKF: skill flags (skill_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|
| `HIDDEN` | 0 | 0 | data-dead |
| `AUTO_HIDE` | 0 | 0 | data-dead |
| `RANDOM_GAIN` | 0 | 0 | data-dead |

## STF: store flags (store_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|
| `DEPEND_LEVEL` | 0 | 0 | data-dead |
| `SHALLOW_LEVEL` | 0 | 0 | data-dead |
| `MEDIUM_LEVEL` | 0 | 0 | data-dead |
| `DEEP_LEVEL` | 0 | 0 | data-dead |
| `RARE` | 370156 | 163 |  |
| `VERY_RARE` | 0 | 0 | data-dead |
| `COMMON` | 0 | 0 | data-dead |
| `ALL_ITEM` | 0 | 0 | data-dead |
| `RANDOM` | 0 | 0 | data-dead |
| `FORCE_LEVEL` | 0 | 0 | data-dead |
| `MUSEUM` | 0 | 0 | data-dead |

## DF: dungeon flags (dungeon_flag_list.hpp)

| name | DATA | RUST | note |
|---|---|---|---|
| `PRINCIPAL` | 0 | 0 | data-dead |
| `MAZE` | 1027 | 1 |  |
| `SMALLEST` | 0 | 0 | data-dead |
| `SMALL` | 9 | 0 | **GAP?** |
| `BIG` | 370156 | 163 |  |
| `NO_DOORS` | 1 | 0 | **GAP?** |
| `WATER_RIVER` | 0 | 0 | data-dead |
| `LAVA_RIVER` | 0 | 0 | data-dead |
| `WATER_RIVERS` | 0 | 0 | data-dead |
| `LAVA_RIVERS` | 0 | 0 | data-dead |
| `CAVE` | 1027 | 1 |  |
| `CAVERN` | 0 | 0 | data-dead |
| `NO_UP` | 0 | 0 | data-dead |
| `HOT` | 370156 | 163 |  |
| `COLD` | 6533 | 2 |  |
| `FORCE_DOWN` | 0 | 0 | data-dead |
| `FORGET` | 0 | 0 | data-dead |
| `NO_DESTROY` | 0 | 0 | data-dead |
| `SAND_VEIN` | 0 | 0 | data-dead |
| `CIRCULAR_ROOMS` | 0 | 0 | data-dead |
| `EMPTY` | 2 | 0 | **GAP?** |
| `DAMAGE_FEAT` | 0 | 0 | data-dead |
| `FLAT` | 1494 | 45 |  |
| `TOWER` | 0 | 0 | data-dead |
| `RANDOM_TOWNS` | 0 | 0 | data-dead |
| `DOUBLE` | 0 | 0 | data-dead |
| `LIFE_LEVEL` | 0 | 0 | data-dead |
| `EVOLVE` | 0 | 0 | data-dead |
| `ADJUST_LEVEL_1` | 0 | 0 | data-dead |
| `ADJUST_LEVEL_2` | 0 | 0 | data-dead |
| `NO_RECALL` | 1 | 1 |  |
| `NO_STREAMERS` | 0 | 0 | data-dead |
| `ADJUST_LEVEL_1_2` | 0 | 0 | data-dead |
| `NO_SHAFT` | 0 | 0 | data-dead |
| `ADJUST_LEVEL_PLAYER` | 0 | 0 | data-dead |
| `NO_TELEPORT` | 12 | 3 |  |
| `ASK_LEAVE` | 0 | 0 | data-dead |
| `NO_STAIR` | 0 | 0 | data-dead |
| `SPECIAL` | 0 | 0 | data-dead |
| `NO_NEW_MONSTER` | 0 | 0 | data-dead |
| `NO_GENO` | 0 | 0 | data-dead |
| `NO_BREATH` | 0 | 0 | data-dead |
| `WATER_BREATH` | 0 | 0 | data-dead |
| `ELVEN` | 0 | 0 | data-dead |
| `DWARVEN` | 0 | 0 | data-dead |
| `NO_EASY_MOVE` | 0 | 0 | data-dead |
| `NO_RECALL_OUT` | 0 | 0 | data-dead |

## GF_* spell types

| name | DATA | RUST | note |
|---|---|---|---|
| `GF_ELEC` | 36 | 43 |  |
| `GF_POIS` | 0 | 48 | data-dead |
| `GF_ACID` | 60 | 47 |  |
| `GF_COLD` | 39 | 48 |  |
| `GF_FIRE` | 106 | 72 |  |
| `GF_UNBREATH` | 0 | 2 | data-dead |
| `GF_CORPSE_EXPL` | 0 | 0 | data-dead |
| `GF_MISSILE` | 19 | 15 |  |
| `GF_ARROW` | 0 | 18 | data-dead |
| `GF_PLASMA` | 0 | 5 | data-dead |
| `GF_WAVE` | 0 | 4 | data-dead |
| `GF_WATER` | 0 | 18 | data-dead |
| `GF_LITE` | 0 | 20 | data-dead |
| `GF_DARK` | 1 | 22 |  |
| `GF_LITE_WEAK` | 0 | 0 | data-dead |
| `GF_DARK_WEAK` | 0 | 0 | data-dead |
| `GF_SHARDS` | 0 | 12 | data-dead |
| `GF_SOUND` | 0 | 23 | data-dead |
| `GF_CONFUSION` | 1 | 3 |  |
| `GF_FORCE` | 0 | 17 | data-dead |
| `GF_INERTIA` | 0 | 5 | data-dead |
| `GF_MANA` | 9 | 20 |  |
| `GF_METEOR` | 0 | 4 | data-dead |
| `GF_ICE` | 1 | 17 |  |
| `GF_CHAOS` | 0 | 25 | data-dead |
| `GF_NETHER` | 2 | 27 |  |
| `GF_DISENCHANT` | 0 | 5 | data-dead |
| `GF_NEXUS` | 0 | 15 | data-dead |
| `GF_TIME` | 10 | 12 |  |
| `GF_GRAVITY` | 0 | 6 | data-dead |
| `GF_KILL_WALL` | 34 | 10 |  |
| `GF_KILL_DOOR` | 0 | 0 | data-dead |
| `GF_MAKE_WALL` | 0 | 0 | data-dead |
| `GF_MAKE_DOOR` | 0 | 0 | data-dead |
| `GF_OLD_CLONE` | 0 | 0 | data-dead |
| `GF_OLD_POLY` | 0 | 0 | data-dead |
| `GF_OLD_HEAL` | 0 | 0 | data-dead |
| `GF_OLD_SPEED` | 0 | 0 | data-dead |
| `GF_OLD_SLOW` | 0 | 0 | data-dead |
| `GF_OLD_CONF` | 0 | 0 | data-dead |
| `GF_OLD_SLEEP` | 0 | 0 | data-dead |
| `GF_OLD_DRAIN` | 0 | 0 | data-dead |
| `GF_AWAY_UNDEAD` | 0 | 0 | data-dead |
| `GF_AWAY_EVIL` | 0 | 0 | data-dead |
| `GF_AWAY_ALL` | 0 | 0 | data-dead |
| `GF_TURN_UNDEAD` | 0 | 0 | data-dead |
| `GF_TURN_EVIL` | 0 | 0 | data-dead |
| `GF_TURN_ALL` | 0 | 0 | data-dead |
| `GF_DISP_UNDEAD` | 0 | 0 | data-dead |
| `GF_DISP_EVIL` | 1 | 1 |  |
| `GF_DISP_ALL` | 0 | 0 | data-dead |
| `GF_DISP_DEMON` | 0 | 0 | data-dead |
| `GF_DISP_LIVING` | 0 | 0 | data-dead |
| `GF_ROCKET` | 9 | 10 |  |
| `GF_NUKE` | 0 | 12 | data-dead |
| `GF_MAKE_GLYPH` | 0 | 0 | data-dead |
| `GF_STASIS` | 0 | 0 | data-dead |
| `GF_STONE_WALL` | 0 | 7 | data-dead |
| `GF_DEATH_RAY` | 0 | 0 | data-dead |
| `GF_STUN` | 0 | 1 | data-dead |
| `GF_HOLY_FIRE` | 0 | 5 | data-dead |
| `GF_HELL_FIRE` | 1 | 7 |  |
| `GF_DISINTEGRATE` | 0 | 3 | data-dead |
| `GF_CHARM` | 0 | 2 | data-dead |
| `GF_CONTROL_UNDEAD` | 0 | 0 | data-dead |
| `GF_CONTROL_ANIMAL` | 0 | 0 | data-dead |
| `GF_PSI` | 0 | 1 | data-dead |
| `GF_PSI_DRAIN` | 0 | 0 | data-dead |
| `GF_TELEKINESIS` | 1 | 1 |  |
| `GF_JAM_DOOR` | 0 | 0 | data-dead |
| `GF_DOMINATION` | 0 | 0 | data-dead |
| `GF_DISP_GOOD` | 0 | 0 | data-dead |
| `GF_RAISE` | 1 | 0 | **GAP?** |
| `GF_DESTRUCTION` | 0 | 9 | data-dead |
| `GF_STUN_CONF` | 0 | 0 | data-dead |
| `GF_STUN_DAM` | 0 | 0 | data-dead |
| `GF_CONF_DAM` | 0 | 0 | data-dead |
| `GF_STAR_CHARM` | 0 | 0 | data-dead |
| `GF_IMPLOSION` | 0 | 0 | data-dead |
| `GF_LAVA_FLOW` | 0 | 0 | data-dead |
| `GF_FEAR` | 0 | 9 | data-dead |
| `GF_BETWEEN_GATE` | 0 | 0 | data-dead |
| `GF_WINDS_MANA` | 0 | 0 | data-dead |
| `GF_DEATH` | 0 | 1 | data-dead |
| `GF_CONTROL_DEMON` | 0 | 0 | data-dead |
| `GF_RAISE_DEMON` | 0 | 0 | data-dead |
| `GF_TRAP_DEMONSOUL` | 0 | 0 | data-dead |
| `GF_ATTACK` | 0 | 1 | data-dead |
| `GF_CHARM_UNMOVING` | 0 | 0 | data-dead |
| `GF_INSTA_DEATH` | 0 | 0 | data-dead |
| `GF_ELEMENTAL_WALL` | 0 | 0 | data-dead |
| `GF_ELEMENTAL_GROWTH` | 0 | 0 | data-dead |

## RBE_* blow effects

| name | DATA | RUST | note |
|---|---|---|---|
| `RBE_ANY` | 0 | 0 | data-dead |
| `RBE_HURT` | 1365 | 2 |  |
| `RBE_POISON` | 158 | 5 |  |
| `RBE_UN_BONUS` | 48 | 2 |  |
| `RBE_UN_POWER` | 36 | 2 |  |
| `RBE_EAT_GOLD` | 23 | 2 |  |
| `RBE_EAT_ITEM` | 12 | 2 |  |
| `RBE_EAT_FOOD` | 7 | 2 |  |
| `RBE_EAT_LITE` | 3 | 2 |  |
| `RBE_ACID` | 60 | 47 |  |
| `RBE_ELEC` | 36 | 43 |  |
| `RBE_FIRE` | 106 | 72 |  |
| `RBE_COLD` | 39 | 48 |  |
| `RBE_BLIND` | 175 | 20 |  |
| `RBE_CONFUSE` | 50 | 4 |  |
| `RBE_TERRIFY` | 45 | 3 |  |
| `RBE_PARALYZE` | 59 | 4 |  |
| `RBE_LOSE_STR` | 36 | 2 |  |
| `RBE_LOSE_INT` | 12 | 2 |  |
| `RBE_LOSE_WIS` | 12 | 2 |  |
| `RBE_LOSE_DEX` | 27 | 2 |  |
| `RBE_LOSE_CON` | 18 | 2 |  |
| `RBE_LOSE_CHR` | 8 | 2 |  |
| `RBE_LOSE_ALL` | 12 | 3 |  |
| `RBE_SHATTER` | 22 | 3 |  |
| `RBE_EXP_10` | 5 | 5 |  |
| `RBE_EXP_20` | 20 | 5 |  |
| `RBE_EXP_40` | 39 | 5 |  |
| `RBE_EXP_80` | 58 | 3 |  |
| `RBE_DISEASE` | 15 | 2 |  |
| `RBE_TIME` | 10 | 12 |  |
| `RBE_SANITY` | 0 | 4 | data-dead |
| `RBE_HALLU` | 9 | 4 |  |
| `RBE_PARASITE` | 0 | 3 | data-dead |
| `RBE_ABOMINATION` | 3 | 3 |  |

## SUMMON_* types

| name | DATA | RUST | note |
|---|---|---|---|
| `SUMMON_ANT` | 0 | 0 | data-dead |
| `SUMMON_SPIDER` | 11 | 0 | **GAP?** |
| `SUMMON_HOUND` | 0 | 0 | data-dead |
| `SUMMON_HYDRA` | 0 | 0 | data-dead |
| `SUMMON_ANGEL` | 0 | 0 | data-dead |
| `SUMMON_DEMON` | 41 | 18 |  |
| `SUMMON_UNDEAD` | 130 | 30 |  |
| `SUMMON_DRAGON` | 83 | 5 |  |
| `SUMMON_HI_UNDEAD` | 0 | 0 | data-dead |
| `SUMMON_HI_DRAGON` | 0 | 0 | data-dead |
| `SUMMON_WRAITH` | 2 | 1 |  |
| `SUMMON_UNIQUE` | 169 | 1 |  |
| `SUMMON_BIZARRE1` | 0 | 0 | data-dead |
| `SUMMON_BIZARRE2` | 0 | 0 | data-dead |
| `SUMMON_BIZARRE3` | 0 | 0 | data-dead |
| `SUMMON_BIZARRE4` | 0 | 0 | data-dead |
| `SUMMON_BIZARRE5` | 0 | 0 | data-dead |
| `SUMMON_BIZARRE6` | 0 | 0 | data-dead |
| `SUMMON_HI_DEMON` | 0 | 3 | data-dead |
| `SUMMON_KIN` | 0 | 0 | data-dead |
| `SUMMON_DAWN` | 1 | 2 |  |
| `SUMMON_ANIMAL` | 261 | 13 |  |
| `SUMMON_ANIMAL_RANGER` | 0 | 0 | data-dead |
| `SUMMON_HI_UNDEAD_NO_UNIQUES` | 0 | 0 | data-dead |
| `SUMMON_HI_DRAGON_NO_UNIQUES` | 0 | 0 | data-dead |
| `SUMMON_NO_UNIQUES` | 0 | 0 | data-dead |
| `SUMMON_PHANTOM` | 0 | 0 | data-dead |
| `SUMMON_ELEMENTAL` | 0 | 0 | data-dead |
| `SUMMON_THUNDERLORD` | 10 | 2 |  |
| `SUMMON_BLUE_HORROR` | 0 | 0 | data-dead |
| `SUMMON_BUG` | 2 | 0 | **GAP?** |
| `SUMMON_RNG` | 1 | 0 | **GAP?** |
| `SUMMON_MINE` | 0 | 0 | data-dead |
| `SUMMON_HUMAN` | 0 | 0 | data-dead |
| `SUMMON_SHADOWS` | 0 | 0 | data-dead |
| `SUMMON_GHOST` | 0 | 0 | data-dead |
| `SUMMON_QUYLTHULG` | 0 | 0 | data-dead |

