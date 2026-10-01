-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::config

规定 `answer::config`（`crates/wire/src/` 下同名的文件）。一个范围的生效配置，每个值旁边是说出它的那一级。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-47e 缺省也是一层

`ConfigLayer` 多一个 `Default`（线上拼 `default`）；`ConfigAnswer::second` 从 `Option<SettledSecond>` 变成 `SettledSecond`：没有一级文件说过时，它是 `CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`。字段换了形而名字没换，故同集进位 38→39。

- **缺席不是一个值**：`None` 让页面自己补上缺省，于是客户端带着第二份 `65`，和 kernel 的那一份之间没有任何东西把它们拴住。回答里写明生效值和它的来源，页面画出城说的数，只有一份。
- **`from = default` 是页面判断「这一级有没有写过」的依据**：`default` 时框留空、把生效值画成提示；否则框里是那一级写下的数。
- **合法域随值一起回答**：`SettledSecond::domain` 是 `SecondThreshold` 构造点读的那两个常量。页面仍不判域（拒因由城带回），但它画给人看的「30 到 90」不再是客户端自己写的第二份。
- **`effort` 不跟着变**：没有一级说过时生效的是提供方的缺省，城不知道那个值；为它编一个级别就是说一句城说不出的话。重开条件：城自己开始为 effort 定一个缺省值。
- **被否的方案**：保留 `Option` 并在旁边加一个 `default_percent` 字段——那样一个值有两个字段，读者要自己拼出生效值，拼法又多一个家。
-/
