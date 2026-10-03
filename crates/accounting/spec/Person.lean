-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::person

规定 `crates/accounting/src/person.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::person` 旁的测试守住。
-/

/-!
### 8-8 accounting::person：这个人自己的那一层（形状 4 适配器）

```rust
pub fn read() -> Result<PreferencesAnswer, AxError>;       // Query::Preferences 的全部
pub fn put(patch: PreferencePatch) -> Result<(), AxError>; // Command::PutPreferences 的全部
pub enum CorePriority { Raised, Normal }                   // 人的设置；Normal 即「关掉高优先级」
pub fn core_priority() -> Result<CorePriority, AxError>;   // ConfigInvalid：priority 既不是 "raised" 也不是 "normal"
pub enum CorePlacement { Off, Soft, SoftShares }           // [core] placement 的 "none"、"soft"（缺省）、"soft_shares"；每一臂开关什么见 `crates/sprawling/spec/Serving/Placement.lean` D47
pub fn core_placement() -> Result<CorePlacement, AxError>; // ConfigInvalid：placement 不是这三个拼写之一（"pinned" 建成之前也在其中）
```

- **文件在每一座城之外**：`<home>/.sprawling/config.toml`，路径由 `accounting::home`（§8-7）给，本模块不拼路径。把城拷到另一台机器，它不跟着走；在同一台机器上换一个浏览器，画出来的仍是这份文件说的样子。
- **`[ui]` 一节就是 `PreferencesAnswer` 的序列化**（`crates/wire/Spec.lean` §8-39 第七条）：文件能写的键与答案能说的字段是**同一份声明**，因此本模块只做读与写，不陈述「一项偏好是什么」。一条补丁落在记录上的效果同理，归 `PreferencesAnswer::apply` —— `Chord("")` 是解绑还是绑一个空串，只有一个地方回答。
- **别的节原样留下**：写是一次读-改-写，经 `city::edit_document`（`crates/city/Spec.lean` §8-27）持锁并整份替换。「要么整份要么不动」只有一份实现，人层与城层共用它；再写一份就是给 B-49 立第二个权威。
- **读不动的文件不覆写**：解析失败报 `E_CONFIG_INVALID`，主题带上文件与是哪一节，恢复语请人手工修或删掉那一节重选。能读回来的才配被改写——写它的人是唯一能修它的人。
- **不入账**：偏好不属于城的历史，任何 run 都观测不到它。因此这条命令被接受时城无话可播，`adversary` 第四世界据此把「静默」读作接受，而它真正的关门条件是读回来那一组断言（`tools/adversary/src/Sprawling/Person.lean`，叶子 5.6）。
- **文件缺席不是失败**：那是一个什么都还没定的人，答案是本 build 画的那几档（`PreferencesAnswer::default`）。`lang` 缺席就是缺席，不填 `en`——没人选过之前，只有浏览器自己的语言标签是证据。

**本章测试**：`what_the_file_states_and_what_the_answer_states_are_one_record`、`a_section_this_build_does_not_read_survives_a_write`、`a_file_that_does_not_parse_is_refused_rather_than_replaced`（`accounting::person::tests`）。
- **`CorePriority` 住在这里而不在 `bin::serving::standing`**：它是这份文件里 `[core] priority` 读出来的值；真去抬高一条线程的做法归 `serving::standing`（`crates/sprawling/Spec.lean` §8-93），它从这里取值。
-/
