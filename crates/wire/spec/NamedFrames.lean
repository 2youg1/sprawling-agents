-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::named_frames

规定 `named_frames`（`crates/wire/src/` 下同名的文件）。一个帧族只声明一次，枚举、帧名与名表都由它生成。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-38 一帧的拼写只有一处：名表由枚举生成

**规则**：`Query` 与 `Command` 的变体表是这个 crate 里唯一拼写帧名的地方；`name()` 与 `QUERY_NAMES`／`COMMAND_NAMES` 由 `wire::named_frames!` 从同一份变体表生成。

**一个拼写，一个家**。手写时一个 `Query` 的拼写写在三处：变体自身、`name()` 的手写臂、`QUERY_NAMES` 数组；握手赖以成立的 `schema_hash()` 只读第三个。穷尽 `match` 挡得住「新变体不写 `name()`」，挡不住「新变体不进名表」。生成之后，名表就是变体表本身，它不能与枚举不一致，因为它没有第二份内容可以不一致。

**为什么它值一次升版**。名字一个没改、字段一个没动、语义一个没变，但生成出来的两张表按**声明顺序**排，而 `COMMAND_NAMES` 的手写顺序不是声明顺序（`Wake` 手写在第 2 位，声明在第 25 位）。`schema_hash()` 按表的顺序混入名字，于是哈希变了。**哈希变即旧页面必须被拒绝**，这正是 `WIRE_V` 存在的理由，故 31→32 与本次同集。客户端侧只有 `client/src/wire.ts` 由 `xtask wire-ts --write` 重生，没有手改。

**为什么是一个宏而不是两个**。`Query` 与 `Command` 的差别只有一个泛型载体（`Secret`），其余逐字相同；`carried_name!` 已经为四个 newtype 用过同一手法，这是复用既有机制而不是造相似物。

**被否**：①保留手写表、加一道 `xtask` 闸去比对——那是给两个家配一个裁判，而不是把它们合成一个；②用 `strum` 之类的派生宏——多一个依赖换一段本仓库五十行就写得出、且要按本仓库的文档口径读的代码。
-/
