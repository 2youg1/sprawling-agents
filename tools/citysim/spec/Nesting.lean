-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::nesting

规定 `citysim::nesting`（`tools/citysim/src/nesting.rs`）与它的读法 `citysim::nesting::reading`（`tools/citysim/src/nesting/reading.rs`），一件只在测试构型里编译的仪器（D21）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-8-3，别处引作 `tools/citysim/Spec.lean §8-8-3`。

本分部只有节注释：`Fault` 的排序与 `grade`、`recommended` 的判法由 Rust 的穷尽枚举与 `nesting::tests` 守着（§16），三种格式的文法是 TOML、JSON 与 Markdown 各自的，模型若重写它们就成了第二份读法。
-/

/-!
#### 8-8-3 nesting（形状 1 判定；仅测试构型）

计划树要住在一个模型每天编辑的文件里，TOML／JSON／Markdown 三选一由这件仪器的数字决定，不由口味决定。`Fault` 是穷尽枚举，**按破坏力排序**：`LostField`（能解析、少了一个字段——唯一一种文件仍可读而一个计划节点悄悄不存在的结局）＞ `ChangedBystander` ＞ `Unparseable` ＞ `Truncated` ＞ `NotApplied`。`grade` 报最坏的那一个；`recommended` 先比错误率，平手比各自最坏的错法，再平手比格式本身，所以同一组成绩两次推荐同一种格式；一种格式一次都没试过时不参加推荐，全都没试过时什么也不推荐。

它不调用模型：`Attempt` 是某个模型已经产出的东西，一个自持 provider 的 suite 无法离线跑、无法重放。语料自己解析不了时拒绝而不是记分（`E_INVALID_ARGS`，recovery 指向语料）。三种格式读成同一组叶子（`path -> value`，`nesting/reading.rs`）；Markdown 那条刻意严格，因为会修复松散缩进的读法会藏掉这件仪器正在计数的失败。
-/
