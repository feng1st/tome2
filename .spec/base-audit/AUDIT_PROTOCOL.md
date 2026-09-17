# Base 审计协议（agent 必读）

## 目标

逐函数对照 `src/*.cc`（原版 ToME C++，唯一事实来源）与 `bevy/src/*.rs`（Rust 重制），
找出**尚未移植的基础功能行为**。产出：把对应 `inventory/*.md` 的复选框标注好，并在
`reports/` 写一份缺口报告。

## 判定标准

- `[x]` 已移植且行为等价（允许变量名/结构不同；逻辑/数值/消息语义等价即可）
- `[>]` 部分移植（缺分支、缺数值表、缺消息、缺边界条件）→ 必须写进报告
- `[ ]` 未移植 → 必须写进报告
- `[~]` 不需移植，仅限以下情形：
  - 前端/UI 布局/按键绑定差异（UI 已约定重做，**不算缺口**）
  - wizard2.cc/debug 命令、main-*.cc 平台前端、z-term/z-form 终端绘制
  - Theme 模块专属（`MODULE_THEME`、`game_module_idx == MODULE_THEME`、Aule/Varda/Ulmo/Mandos 神术、Theme 腐化 14-33、Theme subrace/race）
  - 数据死条目：该旗标/表项在 `lib/edit/*` 中 0 次引用（先 grep 验证）
  - 存档格式/后端实现细节（bevy 用自己的存档格式）

## 方法

1. 打开你的 inventory 文件，逐个函数看。
2. 在 bevy 里找对应：**优先 grep 独有字符串/消息/数值常量/旗标名**，因为 Rust 把很多逻辑并进了
   `game.rs::monster_turns` / `modal.rs::apply_effect` / `game.rs::gf_monster_effect` 等大函数。
   例：查 C++ `py_attack` 是否移植，不要 grep `py_attack`，而是 grep Rust 中"击中消息/伤害公式关键常量"。
3. 每个函数给一个标注；`[>]`/`[ ]` 写进报告。
4. 对照 `lib/edit/*.txt` 数据确认条目是否真的会被用到（数据死条目算 `[~]`）。
5. **只报真实功能缺口**；不确定的放进"UNCERTAIN"节，不要猜。

## 报告格式（`reports/<group>.md`）

```md
# <group> audit report

## GAPS
- [ ] `cmd1.cc:1234` `func_name` — 缺什么，为什么（对照 Rust 哪个函数/分支）— 建议落点 `bevy/src/xxx.rs`
- [ ] ...

## UNCERTAIN
- `foo.cc:99` `fn` — 疑问点

## COVERAGE SUMMARY
- 总函数 N；[x]=n1, [>]=n2, [ ]=n3, [~]=n4
```

## 注意

- 不要修改 `bevy/src/*.rs` 或任何 C++ 源文件。
- 只允许修改：你自己的 `inventory/<group>.md` 和 `reports/<group>.md`。
- 已知的"明确不移植"清单：Theme、UI/HUD 专门设计（界面布局）、TUI/平台前端、wizard/debug。
- 已知不完整但**已确认无缺口**的方面不要重复报（见 `.spec/handoff.md` 顶部第 0 节）。
- 报告要具体到"缺哪个数值表/哪个分支/哪个消息"，不要写"可能未实现"。
