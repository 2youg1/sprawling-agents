-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::worker::freezing::naming：页面要的身份、楼规与城一层

规定 `crates/accounting/src/worker/freezing/naming.rs`，以及身份、楼规与城一层从哪一处答（`crates/accounting/src/views/answering/identity.rs`、`crates/accounting/src/worker/commanding/governing.rs`、`crates/accounting/src/worker/commanding/configure.rs`）。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-15 页面要的几样新东西，从哪一处答（`accounting::views::answering`、`accounting::worker::commanding`、`accounting::worker::freezing`）

**身份。** `Query::Identity` 在锁外读两份治理文档（`city::read_naming`），答 `StatedIdentity` 或带行号的 `Unreadable`（`crates/wire/Spec.lean` §8-59）。`PutDocument` 与 `PutIdentity` 由 `commanding::governing` 执行：先经 `city` 带基线落盘，再写一行 `governed_document_written`，写 `MAYOR.md`／`PREFERENCES.md` 时 `naming` 是落盘之后此刻的身份版本。

**一个 session 冻一版身份**（`worker::freezing::naming`）。冻前缀时先看房间这一层有没有 `[identity] version`：有，就从内容库读回那一版（读不回即拒 `E_STORAGE_FATAL`，不悄悄换成此刻的名字）；没有，就读此刻的身份，放进内容库，写进房间这一层。city 段是 `City.md` 之后接 `Naming::context()`，resident 段对 `hall/mayor` 以冻下的名字开头，`RunPlan.naming` 是那一版的摘要。所以同一个 session 的每次 run 请求里的名字一样，`/new` 之后的第一次 run 换成此刻的名字，页面经 `run_started.naming` 读回的是请求里真正用的那一版。

- 验收：`worker::freezing::tests::naming` 的 `a_new_session_freezes_the_name_the_page_shows`。

**楼规与城一层。** `PutRules` 由 `commanding::configure` 执行：`city::write_rules_against` 落盘之后，读回文件的摘要，与折叠里这份文件上一次的摘要比，记一行 `rules_changed`。`ConfigureCity` 同样落盘之后记 `rules_changed { scope: city, which: Config }`。两条都只在有一项写了时记行；什么都没写的 `ConfigureCity` 什么都不记。`PreferencePatch::CorePriority` 由 `person::put` 落在 `[core]`，其余臂照旧落在 `[ui]`；`CorePriority` 的值集是 `wire::CorePriority`，`person` 再导出它，`bin` 的调用方不必改路径。
-/

/-! D27 一个 session 的身份冻在房间那一层，读回失败就拒，不换成此刻的名字

理由：session 的形状（模型、强度）已经记在房间那一层，`/new` 清的也是它，身份跟着同一个边界就不需要另一条「何时重读身份」的规则（city D11）；读不回冻下的那一版时换成此刻的名字，等于在 session 中途悄悄改名，而这正是冻结要防的。被否决的做法：每次 run 现读身份——改名立刻改掉正在进行的 session 的前缀，provider 的前缀缓存从 city 段起失效，页面上的旧 session 与请求里的名字也对不上。
-/
