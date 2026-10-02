-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::sessions

规定 `answer::sessions`（`crates/wire/src/` 下同名的文件）。一个房间的各段 session，新的在前。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-71 一个房间的各段 session：`Query::Sessions { room }`、`Answer::Sessions`

```rust
Query::Sessions { room: Address }
Answer::Sessions(SessionsAnswer)

pub const SESSIONS_MAX: usize = 64;
pub struct SessionsAnswer {
    pub room: Address,               // 问题的回声
    pub sessions: Vec<SessionLine>,  // 新的在前，至多 SESSIONS_MAX 段
    pub earlier: u64,                // 比这一答最旧的一段还早、没列出的段数
}
pub struct SessionLine {
    pub began: Seq,          // 这一段的第一行：它的 `session_opened`，或房间由派活打开时的第一个 `run_started`
    pub start: SessionStart,
    pub runs: u64,           // 这一段里起过的 run
    pub last: Seq,           // 这一段里账本记在这个地址下的最后一行
    pub at: TimeMs,          // 那一行的时刻
}
pub enum SessionStart {
    Dispatched,                                     // 房间由派活打开，没有 `session_opened`
    Opened { carry: Carry, from: Option<Origin> },  // `/new`：带过去的是什么，从哪里分叉（分叉时）
}
```

- **为什么。** 页面对一个房间的历史只知道热视图里的 run（最近冻结的 32 个加上在跑的）和本次打开以后收到的记录，所以重载之后看不到更早一段的边界，房间信箱的「最近」段（refrain 路线图 S7.5）没有东西可列。session 是房间的一段（`docs/glossary.md` 的 Session），它的边界与起法都在账本里：`session_opened` 带着 `carried` 与 `from`，房间第一次被派活打开时没有这一行，第一行是那个 `run_started`。
- **字段读自账本的哪一行。** `began` 是那一段的第一行；`start` 是 `Opened` 时，`carry` 由 `session_opened.carried` 读出（`true` 即 `Carry::Handoff`：上一段的交接真的带过去了），`from` 原样是它的 `from`；`runs` 数这一段里地址是这个房间的 `run_started`；`last` 与 `at` 是这一段里地址是这个房间的最后一行与它的 `t`，与 `storage::sessions` 为这个房间切的那份切片是同一组行（`crates/storage/Spec.lean` §8-24）。地址不是房间的记录（模型调用、工具调用）不挪 `last`：它们属于 run，run 的进度由 `Query::RunView` 回答。
- **作答在锁内，不读盘。** 视图折叠一张按地址的表（`crates/accounting/Spec.lean` §8-19(c)），`Query::Sessions` 只从这张表里拷出这个房间最新的至多 `SESSIONS_MAX` 段，没有一行账本被读；一个房间从没有过 session 时答空表、`earlier` 为 0，与「这个地址不存在」不作区分，因为房间是目录，答它在不在是 `Query::Listing` 的事。
- **新的在前，一次至多 64 段，没有翻页。** 读它的是「最近」段，要的是最近几段；`earlier` 说出更早的还有多少，页面据此说「更早的 n 段不在此处」而不是假装没有。
- **只动名字表，不进 `WIRE_V`。** 新加一个查询与一个答复，旧帧一个也没改形；名字表多一项，schema 哈希随之变（D1、§4）。
-/

/-! D9 一个房间的 session 列表从视图折叠作答，不从按地址的索引读账本

**决定**：视图多折一张按地址的表，每个地址一列 `SessionLine`；`Query::Sessions` 在锁内拷出这个房间最新的至多 64 段（§8-71）。问题只带 `room`，不带翻页游标。

**理由**：每一段要的四个事实（起点、起法、run 数、最近一次活动）在折叠时一行一行就能定下，表的大小与 session 数同阶，而 session 数不超过 run 数；40 万行夹具城（8,000 个 run、40 个房间）上这张表让视图快照多 948 B（共 15,859,057 B），一个房间作答在 25 µs 以内，不读账本。按地址的索引要在作答时把这个房间的每个 `run_started` 与 `session_opened` 从盘上读回、解析，才能数出 run 数与起法；同一座夹具城上一个房间就是 200 行。

**被否**：①`storage::index` 加一张按地址的表——索引多出与带地址的行同阶的条目（夹具城上 80,000 条），每次作答再读几百行；②问题带 `before` 与 `limit` 翻页——读者是「最近」段，最近 64 段之外它只需要知道还有多少，`earlier` 已经说出。

**重开参数**：页面要浏览一个房间全部的段（不只是最近）时，加 `before` 游标；房间数或 session 数大到这张表在视图快照里成为可见的一段解码时间时，改成按地址的索引。
-/
