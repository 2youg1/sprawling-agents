-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::guide

规定 `guide`（`crates/wire/src/` 下同名的文件）。这个人在上手指南里走到了哪。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-68 上手指南的进度按城保存：`Query::Guide`、`Command::PutGuide`

```rust
// Query
Guide,                                       // → Answer::Guide(GuideProgress)
// Command
PutGuide { progress: GuideProgress, idem: IdemKey },
pub struct GuideProgress {
    pub at: Option<GuideStep>,               // 指南下次从哪一步打开；没走过时缺席
    pub state: GuideState,                   // Open：开城时仍给出指南；Left：人离开过，开城直接进对话
    pub dependencies: Option<GuideMark>,     // 第 2 步：依赖项
    pub texts: Option<GuideMark>,            // 第 3 步：文本与称呼
    pub skills: Option<GuideMark>,           // 第 4 步：导入 skill
    pub mcp: Option<GuideMark>,              // 第 5 步：连接 MCP
}
pub enum GuideStep { Provider, Dependencies, Texts, Skills, Mcp }   // "provider" | "dependencies" | "texts" | "skills" | "mcp"
pub enum GuideState { Open, Left }                                   // "open" | "left"；缺省 Open
pub enum GuideMark { Seen, Skipped }                                 // "seen" | "skipped"；缺席即还没看过
```

- **存的是「走到哪、看过什么、跳过什么」，不存「做完了没有」。** 每一步做完没有，由已保存的配置与检测结果推出：第 1 步看服务端已确认的端点与 `main` 的选择，第 2 步看 doctor，其余各看各自的文件与连接。这里存的是另一种进度：人看过哪一步、选了跳过哪一步、下次从哪一步继续、是否已经离开指南。点开一步不算完成，跳过不画成已配置。
- **第 1 步没有标记。** 它是唯一必做的一步，完成与否只由服务端的配置答，没有「跳过」可选；类型里于是没有它的标记格，`at` 仍可以停在它上面。
- **按城保存，换浏览器也在。** 进度写在城的保留子树里（`kernel::layout::CityLayout::guide`，`<城>/.sprawling/GUIDE.toml`），文件就是 `GuideProgress` 的 TOML 序列化，没有第二份键表；写经 `city::edit_document` 整份替换（与 `accounting::person` 写人的配置同一条路）。浏览器的副本只是缓存。没有文件即 `GuideProgress::default()`：从头开始、仍给出指南。读不出的文件答 `Unavailable`，`PutGuide` 拒 `E_CONFIG_INVALID` 并说出文件与原因，不拿缺省值盖掉它。
- **整份写，后到者为准。** `PutGuide` 带整份进度；两个浏览器同时移动指南时，后写的那一份留下。它是一个光标，不是人写的正文，不设基线守卫：被盖掉的最多是一个「看过」的标记，而守卫会让一次普通的翻页被拒。
- **不写账本行。** 进度是界面的位置，不是这座城做过的事；与 `PutPreferences` 一样只落盘。回执是命令的答复本身，页面再问一次 `Guide` 即得此刻的值。
- **`GuideProgress` 住 `wire::guide`，与 `wire::preference` 并列**：它与人的设置一样，是一份页面读、也整份写回的记录，不只是一种答复。
- **`WIRE_V` 不另进位**：三个都是新名字（D1）。
- 验收：accounting 的 `the_guide_keeps_its_progress_across_a_reopen`（`PutGuide` 写下的进度，在 worker 丢掉、城经 `views::ask` 重开读之后原样答回；没写过的城答缺省）。
-/
