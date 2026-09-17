# audit2：从头开始的逐项审计

起因：旧验收（`.spec/base-audit/loop/accept.py`）用自定 n/a 类与存在性检查，
让"主线任务链未对齐"这种问题都能整批通过 —— 属假通过，已作废。
本目录按用户要求从 0 重建：

## 规则

1. 抽取：
   - `extract_base.py`：`src/*.cc/*.hpp` 每个函数/方法一行；`src/*.hpp`
     的 `#define`/`enum`（名称+值）。
   - `parse_base_data.py`：按原版解析语法逐条/逐行解析 **lib/ 下全部数据**
     （除 lib/mods/theme）——
     `*_info.txt` 用 init1.cc 的 `N:`+字段记录语法；`*.map`/`t_*.txt`/
     `misc.txt` 用 process_dungeon_file 语法（`?:` 条件、`%:` 包含、`F:`
     特性、`D:`/原始地图行）；`lib/pref/*.prf` 用 pref 指令语法。
     产出 `checklist/data/<file>.md`；当前覆盖 285 文件 / 22929 条：
     - 必需：edit 64/6385、file+help 168/14935、pref 9/1186（合计 22506）
     - 列出但豁免/待确认：字体 hud 4/383、xtra 资源 32/32、
       注释文档 doc 2/85、delete.me 占位 8/8
     每行带行号与内容；lib/file 含书/怪物台词/消息文本，lib/help 为帮助文本。
   - `verify_data.py`：数据对数据的逐记录/逐行比对（原版 -> 新版资产），
     已覆盖 11 个 edit 文件共 2167 条记录（r 891、k 596、f 133、e 174、
     a 195、re 13、d 28、ow 70、set 4、ba 47、s 16）全部 MATCH；
     lib/file 的 13 个同名文本资产逐行 MATCH（约 1984 行）。
     解析修正：RON 字段解析要跳过嵌套元组与字符串内括号；s_info 的
     `cast`（A:17）此前丢失，已补进 schools.ron。
   - 豁免：`lib/mods/theme`（不移植）；HUD 类（终端/前端/绘制/消息显示）
     列出但不要求与新版逐行等价（用户规则）。
2. 验收（`audit.py`）只认机器证据：
   - methods：`- [x]` 必须同时有 `impl:bevy/src/<f>.rs:<line>`（文件与行号
     有效）和 `test:<模块::测试名>`（bevy/src 中真实存在的 `#[test]`）；
     缺一判未完成。`logic:` 只是比对结论，供人工复核，不作证据。
   - defines：名字须出现在 bevy/src；纯数字值须同行出现同值。
   - data：映射的转换产物存在且记录数不少于原版（`DATA_MAP`）。
   - 任何人写的 n/a、标签、类名一律不作证据；只有 HUD/theme 两类豁免。
3. 循环：`audit.py` 未 ACCEPTED 前，看门狗（`tui_watchdog.py`）会持续
   注入"继续"指令；`AUDIT_CMD` 默认已指向本目录的 audit.py。

## 用户裁定（2026-09-17）

- **能移植的内容数据全部移植**；font/音乐等与 Bevy HUD 渲染实现不兼容的
  才豁免。据此：`lib/pref/*.prf`（窗口/字体/终端设置）全部豁免，只有
  `colors.prf` 必需 —— 已由 `bevy/src/colors.rs::parse_colors_prf` 应用。
- 已移植并逐行 MATCH：`lib/file` 全部 46 个文件（含 book-*.txt、news、
  dam_*、dead、monspeak 等；rart_s/rart_f 对照 randarts.ron 的 junk 名单）、
  `lib/help` 全部 122 个文件、`lib/pref/colors.prf` 15 行。

## 当前基线

- methods: required 2177 / hud 329；defines/enums: 1835（证据待填）。
- data: 21335 条必需；机器 verifier 已覆盖 17117（含 11 个 edit 文件
  2167 条记录、lib/file 46 文件、lib/help 122 文件、colors.prf 15 行）。
- 数据未决 4218 条 / 58 文件：p_info 1401、ra_info 190、v_info 103、
  w_info 84、ab_info、城镇 t_*、地图 s_*.map/volcano/numenor 等，
  待逐文件加 verifier（多数已移植，只差对照）。
- `audit.py` 未决：4070（methods 2177 + defines 1833 + 数据 58 文件聚合）。

## 每项怎么做

1. 在 `checklist/methods.md` 找到原版条目（文件:行 -> 打开 `src/` 原码）。
2. 在 `bevy/src` 找到对应实现，逐行比对（公式、边界、顺序、常量、消耗、
   消息、随机调用次数与顺序）。
3. 有必要就补/改 Rust 实现与测试。
4. 在该行写入：
   `- [x] ... — impl:bevy/src/game.rs:1234 test:game::tests::name logic:<结论>`
5. 跑 `python3 .spec/audit2/audit.py`，未决数必须下降。

## 明确的既有结论（必须修正的历史问题）

- 主线链曾用 port 自造 `quests.ron`（kobold…）；已删除，改为原版
  `PLOT_NECRO(1)→PLOT_ONE(17)→PLOT_SAURON(2)→PLOT_MORGOTH(3)→ULTRA_GOOD(20)/
  ULTRA_EVIL(21)`，含 Sauron/Morgoth 生成门禁与 guardian 门禁。
- 旧 `.spec/base-audit` 的 ACCEPTED 不再代表完成，仅作历史参考。
