-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::frames::monitor

规定 `frames::monitor`（`crates/wire/src/` 下同名的文件）。性能监视器的一对帧与一次读数。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-47g 性能监视器的一对帧（`wire::frames::monitor`，形状：值类型）

监视页和 `sprawling top` 读同一份历史（sprawling-SPEC.md 8-94），它们从线上拿到它。线协议为此加一种帧，两个方向各一个变体：

- `ClientFrame::Monitor(Monitoring)`：`Monitoring` 是 `Watch`、`WatchSummary` 或 `Release`。`Watch` 让这个会话算作一个看整页的人，`WatchSummary` 让它算作一个只看事实条摘要的人，`Release` 让它不再算；会话结束等于 `Release`。
- `ServerFrame::Monitor(Sample)`：一次读数，只发给正在看的会话。`Sample` 的字段即 sprawling-SPEC.md 8-94 列出的 13 个 `u64`，加上 §8-64 的 `view_backlog` 与 `read_nanos`；它定义在 `wire::frames::monitor`，`bin::monitor` 用的就是这一个类型，不再另写一份。
- `Watched`：看的人看什么，`Everything`（整页）或 `Summary`（事实条上的摘要）。它不上线，是 `MonitorFeed::watch` 的参数，`bin::monitor::Monitor` 按它分两类计数。
- `decide_frame` 把一个已打开会话的 `Monitor(Watch)` 答成 `SessionStep::Watch(Watched::Everything)`，`Monitor(WatchSummary)` 答成 `SessionStep::Watch(Watched::Summary)`，`Monitor(Release)` 答成 `SessionStep::Release`；未打开的会话照旧拒绝并关闭。外壳收到 `Watch(watched)` 时调用 `ServeConfig::monitor` 的 `watch(watched)` 拿一个看的凭据，已有凭据时换掉它、保留已有的 `samples` 订阅，没有时订阅 `samples`；收到 `Release` 时把两者都丢掉。重复的 `Watch` 不叠加计数：一个会话至多持有一个凭据。

**决定。**

1. 看与不看是会话里的两个帧，而不是一个 `Query`。`Query` 问一次答一次，而监视是一段持续的订阅：它的结束（`Release` 或断开）必须让城停止采样，这件事只有持有会话的外壳能保证。另一种做法是每秒一个 `Query`，它让每个看的人每秒多一次往返，且城无法知道人已经走了。
2. `watch` 是一个返回不透明凭据的函数，而不是把计数器交给本 crate。有没有人在看由 `bin::monitor::Monitor` 一处决定；外壳只持有凭据，丢掉它就是不看。
3. 读数经 `broadcast` 发出，与 `deltas`、`logs` 同形：错过的一次读数不必补，下一秒还有一个。
4. 摘要是第三个变体，而不是 `Watch` 带一个参数。事实条在每个页面上，所以它的看法必须比整页便宜得多：只看摘要时城只读本进程（sprawling-SPEC.md 8-96 的 `OwnProcess`，约 1 µs），不打开整机与卷的计数器。已有的 `"watch"` 拼写保持原义，新的 `"watch_summary"` 让 `WIRE_V` 加一。另一种做法是让 `Watch` 带上 `Watched`，它改掉已有帧的拼写，而得到的东西相同。
-/

/-!
### 8-64 `Sample` 多两项，`ModelTag` 多一值

```rust
pub struct Sample { /* …既有 13 项… */ pub view_backlog: u64, pub read_nanos: u64 }
pub enum ModelTag { /* …既有… */ Ocr }      // 线上 "ocr"
```

- `view_backlog`：写者已经交给视图线程、还没折完广播的已提交记录条数，读 `bin::serving::folding::Backlog::records`（sprawling-SPEC 8-123）；采样线程每一拍读一次。
- `read_nanos`：上一拍读计数器花了多少纳秒，由采样线程用单调钟在读取前后各量一次；第一拍为 0（sprawling-SPEC 8-129-6）。
- `ModelTag::Ocr`：人登记的一个能读图的模型，城的 OCR 工具读这一次选择（`crates/gateway/Spec.lean` §8-34）。二进制里不带任何模型（D18），这个值只是一个登记位。
- 三项都是名字不变的改形，共用 45。
-/
