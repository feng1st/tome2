# Base 审计索引（第 19 轮，2026-09-16）

本目录是"原版 ToME base 是否全部移植"的可复核索引，回答"哪些函数/数据/旗标已经被逐条检查过、结果是什么"。

## 怎么用

1. 先看 `MASTER.md`：从各报告汇总的带状态清单（`[x]` 已实现、`[>]` 仍有小缺口（附说明）、`[~]` 确认 n/a（附理由）、`[ ]` 未完成），带总览统计。第 19 轮结束时为 489 / 5 / 0 / 22。
2. 找子系统细节看 `reports/*.md`：每个报告有 GAPS 列表、UNCERTAIN、`DONE THIS SESSION` 记录。
3. 找"原版有没有这个函数/我是不是漏了"看 `inventory/*.md`：按文件分组的逐函数清单（含签名与文档注释摘要）。
4. 找枚举类问题（item/spell/monster/terrain/flag）看 `enums/`。
5. `gen_inventory.py` / `gen_enums.py` / `gen_data_enums.py` 可重新生成清单（注意：重新生成会覆盖标注）。

## 文件地图

| 路径 | 内容 |
|---|---|
| `MASTER.md` | 汇总总清单 + 统计（回答"还剩什么没做"） |
| `AUDIT_PROTOCOL.md` | 审计/标注协议（判定标准、不移植范围） |
| `reports/00-core.md` | 内核：文件解析、tables、随机、dice、选项、squeltch、notes |
| `reports/01-commands.md` | cmd1-7/bldg/store/help：玩家命令与商店动作 |
| `reports/02-spells.md` | spells1-6/spell_type/powers/randart：法术与 GF 效果 |
| `reports/03-monsters.md` | melee1-2/monster1-3/monster_spell：怪物 AI、近战、施法 |
| `reports/04-objects.md` | object1-2/filter：物品生成、鉴定、破坏、价值 |
| `reports/05-map.md` | cave/generate/gen_maze/gen_evol/dungeon/wild：地图生成与世界 |
| `reports/06-quests.md` | q_*.cc：全部剧情任务 |
| `reports/07-player.md` | xtra1-2/skills/birth/corrupt/mimic/gods：玩家系统 |
| `inventory/00-core.md` … `09-tables.md` | 原版逐函数清单（9 组，含 `file.cc:line`） |
| `enums/00-flags.md` | 9 张 X-macro 旗标全表 |
| `enums/01-gf-blow.md` | GF_*、gf_names、RBE_* |
| `enums/02-spells.md` | spells5.cc 全部 130 条法术（含 Theme 标注） |
| `enums/03-summons.md` | SUMMON_* 枚举 |
| `enums/04-data-counts.md` | lib/edit 各文件记录数 |
| `enums/05-coverage.md` | 机械覆盖报告：每个旗标在数据中出现次数 + 在 Rust 代码中作为字符串出现次数 |
| `enums/10-f_info.md` … `20-world-towns-maps.md` | `lib/edit/*.txt` 逐条记录清单（terrain/monster/item/artifact/ego/p_info/d_info/vault/store/ability/skill/owner/ego-monster/set/randart/world/towns/maps） |

## 判定范围（第 19 轮）

- **base 全部实现**；唯一明确不移植：Theme 模块（四神 Aule/Varda/Ulmo/Mandos、腐化 14-33、Theme subrace/race、Theme 法术）。
- UI/HUD 专门设计仍按既有约定单独立项（功能窗口已做，布局/美术不照抄原版）。
- 数据死条目（`lib/edit` 中 0 引用）与平台/前端/wizard 代码记 `[~]`。

## 机械覆盖报告说明

`enums/05-coverage.md` 由 `check_coverage.py` 生成：对每个旗标统计
`DATA`（在 `lib/edit/*.txt` 中出现次数）与 `RUST`（在 `bevy/src/*.rs` 中以字符串字面量出现次数）。
`DATA>0 且 RUST=0` 是"可能未被消费"的高优先级候选；反过来 `DATA=0` 是数据死条目。
注意它只是粗筛：字符串出现≠语义正确，最终判定以 `reports/*.md` 为准。
