# 收敛循环协议（第 19 轮收尾）

**判定权在机器，不在我。** `loop/accept.py --full` 返回 0 = ACCEPTED；否则把打印出的
违例当作下一轮工作项，直到它返回 0。`check.py` 只是轻量状态检查，验收以 `accept.py` 为准。

## 不变量（每轮必须同时满足）

1. `loop/accept.py` 通过（无 `[ ]`/`[>]`、证据逐条复核、无报告层矛盾）。
2. `cargo test` 全绿（连跑 ≥3 次）。
3. autoplay 冒烟无 panic（默认/20 层/城镇/存档读档至少 4 个钩子）。
4. `MASTER.md` 由脚本从 reports 重新生成，数字与 check.py 一致。

## 标注规则（防止自欺）

- `[x]` 必须满足其一：
  a. 有实现落点（`bevy/src/*.rs:line`）**且**有测试覆盖或冒烟截图可证；或
  b. 与原版逐行对照后确认行为等价，并在 bullet 内注明对照点（C++ `file:line` ↔ Rust）。
- `[~]` 必须给出**可复核证据**：`grep` 证明数据 0 引用 / `MODULE_THEME` / 平台前端 /
  引擎无对应结构（并说明缺什么结构、影响是什么）。禁止用"太麻烦"作理由。
- `[>]` 视为未完成，等于 `[ ]`。

## 每轮流程

1. 跑 `check.py`，把 OPEN/PARTIAL 抄成工作项（含 C++ 对照点、落点文件、验收行为）。
2. 按文件所有权并行实现（跨文件项必须指定一个 owner，并在实现后由 owner 跑
   `cargo check` + 相关测试；不允许只留 "NEEDS OTHER FILE"）。
3. 更新对应 bullet 为 `[x]`/`[~]`，附证据。
4. 跑 `accept.py --full`（含 cargo test ×3 + 5 项冒烟）；非 0 回到 1。
5. CONSISTENT 后更新 `handoff.md` 与 `README.md` 数字。

## 反模式（本轮曾犯，禁止）

- 只提取 `[ ]` 而忽略 `[>]`（导致 160 条部分项在最后一轮才浮现）。
- 跨文件项两边都写 "NEEDS OTHER FILE" 却无人认领。
- 把功能性的提示文案/子窗口流程/输入框当"UI 单独立项"跳过。
- 对语义不确定的项（如商店 `apply_magic(okay=false)`）不做验证就搁置：必须先读原版
  代码确认行为，再决定实现或给出 `[~]` 证据。


## 事故记录（2026-09-16，必须遵守）

第 19 轮一度用 `sync_checklists.py` 批量改标记、再给无法核实的行批量写
`[audit: ...]` 自述理由，然后让验收器把这些自述当证据——**自证循环，已作废**。
现规则：

1. `[audit: ...]` 只能作为文档备注，`accept.py` 明确拒绝作为证据。
2. `[x]` 的证据只能是：reports 行为条目（带 C++ 对照 + Rust 落点/测试）、
   代码里真实存在的符号（`fn name(`/调用点）、或 RON/数据事实。
3. `[~]` 的证据只能是机器可复核的类别（数据 0 引用、C++ 死代码、Theme 集、
   前端文件限定），不接受“UI/引擎/太麻烦”等自述。
4. 2026-09-16 重置后 inventory 有 721 项 `[ ]`；这些必须逐项真实补齐或给出
   机器证明，不得再用标签批量转绿。


## TUI 看门狗（模拟输入驱动本会话，不用 headless）

`python3 .spec/base-audit/loop/tui_watchdog.py`（另起进程，可 `nohup ... &`）

- 活动监测：`~/.local/share/opencode/opencode.db-wal` 的 mtime；停止 `IDLE` 秒
  且严格验收未 ACCEPTED 时，用 Konsole D-Bus `Session.sendText` 向本窗口
  注入继续指令 + 回车（真实模拟键盘输入，不启动新的 opencode 进程）。
- 周期跑 `accept.py --quick --no-smoke`，出现 `VERDICT: ACCEPTED` 即退出。
- 停止：`touch .spec/base-audit/loop/tui_watchdog.stop`。
- 已验证：`sendText` 注入通道 rc=0（空格+退格探针）；dry-run 能正确解析
  `verdict=REJECTED open=686` 并触发注入判定。


## 当前剩余工作（2026-09-17 深夜状态）

严格验收剩 36 项；已发现其中 17 项背后的真实缺口：
**spells5.cc 注册的 130 个法术有 25 个不在 spells.ron 里**（4 个神系各 4 个：
Aule/Varda/Ulmo/Mandos，加 9 个 Geomancy 与 Grow Athelas）。Geomancy 在 port
里走 Modal::Geomancy 菜单（效果已实现），但 16 个神术完全缺失。

下一步（按序）：
1. `spells4.cc schools_init` 里 `god_school_new(&SCHOOL_X, GOD_X)` 使用 SKILL_PRAY、
   按创建顺序分配 school id；需要在 schools.ron/convert_data 里补 Aule/Varda/Ulmo/
   Mandos/Nature 的 4+1 个学校，并在 spell.rs 的 god_provider/effective_school_value
   中接上（神术等级=Prayer 技能按 god_school 的 mol/div）。
2. convert_data.py 增加 17 行（Grow Athelas + 16 神术），字段取 spells5.cc 注册：
   mana/difficulty/描述行/charges/alloc；kind 用 C++ 效果函数名去掉 `_spell`。
3. modal.rs 加 17 个 cast 分支，逐条对照 spells3.cc 的效果函数（fire_brand 投射、
   aule 附魔物品选择、召唤 Dwarven warrior/Water spirit/Experience spirit、
   mandos 治疗/预知、ulmo 直线/墙、varda 照明/驱邪/地图+鉴定/多段光爆、
   grow_athelas 清 Black Breath）。
4. info 文本已在 spell.rs spell_info 里（按法术名匹配），无需再动。
5. 完成后：inventory 16 个 `*_spell` + nature_grow_athelas 改 `[x]`，跑 accept。
其余 19 项：spells1/2/4/5 的辅助/显示函数（详见 accept 的 UNRESOLVED 列表）。


### 神术批次执行清单（已核查，直接照做）

前置：port 的 god 体系只到 1-5（Eru/Manwe/Tulkas/Melkor/Yavanna）。C++ 有 9 神，
Aule=6/Varda=7/Ulmo=8/Mandos=9 需要补：
- `spell.rs god_name/god_id/god_id_full`（tables.cc deity_info：Aule、Varda、
  Ulmo、Mandos + 全名 `Aule`/`Varda Elentari`?/`Ulmo`/`Mandos`，以 tables.cc 为准）
- `game.rs god_boons`（xtra1.cc calc_gods 对应分支）与 `god_feather` 等阈值
- birth 的神选择列表（按 class/spec god 列表；p_info C:g: 已含这些神？核对
  classes.ron god 列表来自 convert；若缺，加到 convert_data 的 class god 源）
- 神术学校：统一 school:53（Prayer），god 字段填名字（port 已有 53=Prayer、
  spell.rs 按 god 字段判 WIS 与 piety 施法）。

convert_data.py SPELLS 追加（name, school, level, mana_min, fail, mana_max,
kind, arg, targeted；数值取自 spells5.cc set_difficulty/set_mana）：
  ("Firebrand",53,1,10,20,100,"aule_firebrand","",false)
  ("Enchant Weapon",53,10,100,20,200,"aule_enchant_weapon","",false)
  ("Enchant Armour",53,15,100,20,200,"aule_enchant_armour","",false)
  ("Child of Aule",53,20,200,40,500,"aule_child","",false)
  ("Light of Valinor",53,1,1,20,100,"varda_light_of_valinor","",false)
  ("Call of Almaren",53,10,5,20,150,"varda_call_of_almaren","",false)
  ("Evenstar",53,20,20,20,200,"varda_evenstar","",false)
  ("Star Kindler",53,30,50,20,250,"varda_star_kindler","",true)
  ("Song of Belegaer",53,1,1,25,100,"ulmo_song_of_belegaer","",true)
  ("Draught of Ulmonan",53,15,25,50,200,"ulmo_draught_of_ulmonan","",false)
  ("Call of the Ulumuri",53,20,50,75,300,"ulmo_call_of_the_ulumuri","",false)
  ("Wrath of Ulmo",53,30,100,95,400,"ulmo_wrath_of_ulmo","",true)
  ("Tears of Luthien",53,5,10,25,100,"mandos_tears_of_luthien","",false)
  ("Feanturi",53,10,40,50,200,"mandos_feanturi","",false)
  ("Tale of Doom",53,25,60,75,300,"mandos_tale_of_doom","",false)
  ("Call to the Halls",53,30,80,95,400,"mandos_call_to_the_halls","",false)
  ("Grow Athelas",6,30,60,95,100,"grow_athelas","",false)
SPELL_GOD 追加：Aule 4 条→"Aule"；Varda 4 条→"Varda"；Ulmo 4 条→"Ulmo"；
Mandos 4 条→"Mandos"；Grow Athelas 不加（Nature）。

效果（spells3.cc 原文已核对；modal.rs `apply_effect` 加分支，用
`spell::get_level_s(&ctx.ps,&ctx.inv,&ctx.gd,sp,max)`）：
- aule_firebrand → 仿 demon_blade：tim_project = level+randint(20)；
  gf = level>30 ? HOLY_FIRE : FIRE；tim_project_dam = 4+level；rad = level>=15。
- aule_enchant_weapon/armour → 新模态选包内物品（过滤：武器 TVals / Not(IsArtifact)，
  护甲 TVals；port `item::is_standard_artifact` 已有），应用
  num_h=1+randint(level/12)（护甲 num_a=1+randint(level/10)），
  level>=5/20 加 num_d、>=45/40 加 num_p；`Modal::AuleEnchant{armour}`。
- aule_child → god 召唤 "Dwarven warrior"，level=20+get_level(AULE_CHILD,70)（0 下限）。
- mandos_tears_of_luthien → hp_player(10*ls(30)); stun/cut/fear 清零。
- mandos_feanturi → fear/confused=0；level>=20 restore WIS+INT；>=30 image=0、
  回复 sanity（ps.msane*level/100，检查 csane 字段）。
- mandos_tale_of_doom → set_tim_precognition(5+ls(10))（port 有 precog 吗？
  有 `turn.precog_pending`；若无计时字段需加 `ps.precog_timer` 并在 turn 处理）。
- mandos_call_to_the_halls → 随机 "Experienced spirit"/"Wise spirit"，
  level=20+get_level(MANDOS_CALL_HALLS,70)。
- ulmo_song_of_belegaer → 定向 bolt/beam GF_WATER：dice=ls(10), sides=3+ls(35)，
  fire_bolt_or_beam(2*ls(85), ...)；port 找 beam 实现（modal 有 fire_bolt 吗）。
- ulmo_draught_of_ulmonan → hp_player(5*ls(50))；清 poison/cut/stun/blind；
  >=10 restore STR/CON/DEX；>=20 清 parasite/mimic（`ps.parasite`/`ps.mimic`）。
- ulmo_call_of_the_ulumuri → 随机 "Water spirit"/"Water elemental"，
  level=30+get_level(ULMO_CALL_ULUMURI,70)。
- ulmo_wrath_of_ulmo → 定向墙 fire_wall(GF_WATER 或 >=30 GF_WAVE，
  dam=40+ls(150), dur=10+ls(14))。
- varda_light_of_valinor → >=3 lite_area(10,4) 否则 lite_room；>=15
  fire_ball(GF_LITE, 0, 10+ls(100), 5+ls(6))。
- varda_call_of_almaren → power=5*lev；>=20 dispel_evil(power) 否则 banish_evil(power)。
- varda_evenstar → wiz_lite_extra（modal `wiz_lite_ctx`）；>=40 identify_pack。
- varda_star_kindler → 定向；n=lev/5；n×fire_ball(GF_LITE, dir, 20+ls(100), 10)。
- grow_athelas → 若 ps.black_breath 清除并消息（game.rs 测试已存在）。
完成后跑 cargo test 与 accept，把 16 个 `*_spell` + `nature_grow_athelas` 标 `[x]`。


### 神术批次进度（2026-09-17）

已完成：
- `birth.rs gods_from_mask 0..=9` + `god_name_full` 补 Aule/Varda/Ulmo/Mandos；
  `spell.rs god_name/god_id/god_id_full` 同步；测试已更新（263 通过）。
- convert_data.py 加 17 行 + SPELL_GOD；spells.ron 重新生成（含 16 神术 +
  Grow Athelas，school 53/6）。
- `game.rs spawn_companion_level`（带 mon_level 的同伴召唤，供神术用）。

待做（只剩这一步 + 回填）：
1. modal.rs `apply_effect`（非指向）加分支：
   aule_firebrand（tim_project 三字段）、aule_enchant_weapon/armour（新模态
   `Modal::AuleEnchant{armour}` + 渲染 + 字母输入；过滤：武器 TV_MSTAFF/BOW/
   HAFTED/POLEARM/SWORD/AXE  或 护甲 BOOTS/GLOVES/HELM/CROWN/SHIELD/CLOAK/
   SOFT_ARMOR/HARD_ARMOR/DRAG_ARMOR，且 `item::is_standard_artifact` 为 false；
   应用 num_h=1+randint(level/12) [armour: num_a=1+randint(level/10)]，
   level>=5/20 加 num_d，level>=45/40 加 pval 1）、aule_child（Dwarven warrior,
   level=20+l0(70)）、mandos_tears_of_luthien（hp 10*ls(30)+清 stun/cut/fear）、
   mandos_feanturi（清 fear/confused；>=20 restore WIS/INT；>=30 image=0 +
   sanity 回复；port 字段核对 csane/msane）、mandos_tale_of_doom
   （precognition 计时，必要时加 ps 字段并在 world turn 递减）、
   mandos_call_to_the_halls（Experience/Wise spirit，20+l0(70)）、
   ulmo_draught_of_ulmonan（hp 5*ls(50)+清 poison/cut/stun/blind；>=10 restore
   STR/CON/DEX；>=20 清 parasite/mimic）、ulmo_call_of_the_ulumuri
   （Water spirit/elemental，30+l0(70)）、varda_call_of_almaren（>=20 dispel_evil
   (5*lev) else banish）、varda_evenstar（wiz_lite_ctx；>=40 identify_pack）、
   grow_athelas（black_breath 清除）。
2. modal.rs 投射段（`p.kind ==` 区，~17792 附近）加：
   ulmo_song_of_belegaer（bolt/beam GF_WATER，dice=ls(10), sides=3+ls(35)，
   威力 2*ls(85)）、ulmo_wrath_of_ulmo（wall：可照 firewall 循环写 cloud；
   >=30 GF_WAVE else GF_WATER；dam 40+ls(150), dur 10+ls(14)）、
   varda_light_of_valinor（>=3 先 lite_area/lite_room；>=15 fire_ball GF_LITE
   dam 10+ls(100) rad 5+ls(6)）、varda_star_kindler（n=lev/5 次 GF_LITE rad10
   dam 20+ls(100)）。
3. cargo test；把 inventory 16 个 `*_spell` + nature_grow_athelas 标 `[x]`
   （cite modal.rs 分支+测试）；跑 accept。
