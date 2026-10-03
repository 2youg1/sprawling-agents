-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::frames::monitor

规定 `frames::monitor`（`crates/wire/src/` 下同名的文件）。性能监视器的一对帧与一次读数。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-47g 性能监视器的一对帧（`wire::frames::monitor`，形状：值类型）

监视页和 `sprawling top` 读同一份历史（`crates/sprawling/Spec.lean` §8-94），它们从线上拿到它。线协议为此加一种帧，两个方向各一个变体：

- `ClientFrame::Monitor(Monitoring)`：`Monitoring` 是 `Watch`、`WatchSummary` 或 `Release`。`Watch` 让这个会话算作一个看整页的人，`WatchSummary` 让它算作一个只看性能摘要的人，`Release` 让它不再算；会话结束等于 `Release`。
- `ServerFrame::Monitor(Sample)`：一次读数，只发给正在看的会话。`Sample` 的字段即 `crates/sprawling/Spec.lean` §8-94 列出的 13 个 `u64`，加上 §8-64 的 `view_backlog` 与 `read_nanos`；它定义在 `wire::frames::monitor`，`bin::monitor` 用的就是这一个类型，不再另写一份。
- `Watched`：看的人看什么，`Everything`（整页）或 `Summary`（设置树「性能」条目旁那一行摘要）。它不上线，是 `MonitorFeed::watch` 的参数，`bin::monitor::Monitor` 按它分两类计数。
- `decide_frame` 把一个已打开会话的 `Monitor(Watch)` 答成 `SessionStep::Watch(Watched::Everything)`，`Monitor(WatchSummary)` 答成 `SessionStep::Watch(Watched::Summary)`，`Monitor(Release)` 答成 `SessionStep::Release`；未打开的会话照旧拒绝并关闭。外壳收到 `Watch(watched)` 时调用 `ServeConfig::monitor` 的 `watch(watched)` 拿一个看的凭据，已有凭据时换掉它、保留已有的 `samples` 订阅，没有时订阅 `samples`；收到 `Release` 时把两者都丢掉。重复的 `Watch` 不叠加计数：一个会话至多持有一个凭据。

**决定。**

1. 看与不看是会话里的两个帧，而不是一个 `Query`。`Query` 问一次答一次，而监视是一段持续的订阅：它的结束（`Release` 或断开）必须让城停止采样，这件事只有持有会话的外壳能保证。另一种做法是每秒一个 `Query`，它让每个看的人每秒多一次往返，且城无法知道人已经走了。
2. `watch` 是一个返回不透明凭据的函数，而不是把计数器交给本 crate。有没有人在看由 `bin::monitor::Monitor` 一处决定；外壳只持有凭据，丢掉它就是不看。
3. 读数经 `broadcast` 发出，与 `deltas`、`logs` 同形：错过的一次读数不必补，下一秒还有一个。
4. 摘要是第三个变体，而不是 `Watch` 带一个参数。摘要在设置面打开的任何一页上都看得见，所以它的看法必须比整页便宜得多：只看摘要时城只读本进程（`crates/sprawling/Spec.lean` §8-96 的 `OwnProcess`，约 1 µs），不打开整机与卷的计数器。已有的 `"watch"` 拼写保持原义，新的 `"watch_summary"` 让 `WIRE_V` 加一。另一种做法是让 `Watch` 带上 `Watched`，它改掉已有帧的拼写，而得到的东西相同。
-/

/-!
### 8-64 `Sample` 多两项，`ModelTag` 多一值

```rust
pub struct Sample { /* …既有 13 项… */ pub view_backlog: u64, pub read_nanos: u64 }
pub enum ModelTag { /* …既有… */ Ocr }      // 线上 "ocr"
```

- `view_backlog`：写者已经交给视图线程、还没折完广播的已提交记录条数，读 `bin::serving::folding::Backlog::records`（`crates/sprawling/Spec.lean` §8-123）；采样线程每一拍读一次。
- `read_nanos`：上一拍读计数器花了多少纳秒，由采样线程用单调钟在读取前后各量一次；第一拍为 0（`crates/sprawling/Spec.lean` §8-129-6）。
- `ModelTag::Ocr`：人登记的一个能读图的模型，城的 OCR 工具读这一次选择（`crates/gateway/Spec.lean` §8-34）。二进制里不带任何模型（D18），这个值只是一个登记位。
- 三项都是名字不变的改形，共用 45。
-/

/-!
### 8-47i 采样节拍由页面调，按城记住

```rust
pub enum Monitoring { Watch, WatchSummary, Release, Beat(BeatMs) }   // 线上 {"beat": 250}
#[serde(try_from = "u32", into = "u32")]
pub struct BeatMs(u32);                       // BEAT_MIN_MS ..= BEAT_MAX_MS 之内才造得出
impl BeatMs { pub const DEFAULT: BeatMs;      // 100 ms（Roadmap M0 第 7 条）
              pub fn new(ms: u32) -> Result<BeatMs, AxError>; pub fn ms(self) -> u32; }
pub const BEAT_MIN_MS: u32 = 10;
pub const BEAT_MAX_MS: u32 = 1_000;
pub struct Sample { /* …既有 15 项… */ pub beat_ms: u64 }
pub enum SessionStep { /* …既有… */ Beat(BeatMs) }
pub struct MonitorFeed { /* …既有… */ pub beat: Arc<dyn Fn(BeatMs) + Send + Sync> }
```

- `decide_frame` 把已打开会话的 `Monitor(Beat(b))` 答成 `SessionStep::Beat(b)`，外壳调用 `MonitorFeed::beat(b)`；未打开的会话照旧拒绝并关闭。范围之外的数在反序列化时就被拒（`E_INVALID_ARGS`），所以外壳拿到的节拍总在范围之内。
- 节拍是内存节拍（`crates/sprawling/spec/Monitor.lean` §8-96）：私有字节每拍读一次，`Sample` 每十拍发一次，所以默认 100 ms 时页面仍是每秒一点。`Sample.beat_ms` 报这一点读数时的节拍，页面上的控件从它读出当前的值，不另存一份。
- 三个平台上相同：节拍只是采样线程 `sleep` 的长度。

-/

/-! D44 采样节拍是 `Monitoring` 的第四个变体，而不是一个 `Command`

**理由**：它与看不看一样只改这座城此刻怎样采样，不写账本、不改城的任何状态，执行者是持有监视器的外壳，而 `Command` 的执行者是记账线程，它够不到采样线程；城记住它（`crates/sprawling/spec/Monitor.lean` D44），是为了下一次 serve 不回到默认。

**被否**：①一个 `Command::SetMonitorBeat`：要过记账线程，再由它转交外壳，多一条只为转交的路；②放进 User 的偏好文件：偏好跟人走，而节拍是这座城这台机器上的测量设定，两座城可以不同。

**定下的值**：范围 10–1000 ms 是推断的选择：低于 10 ms 时读一次私有字节的代价（约 1 µs，§8-96）开始占到节拍的可见比例，高于 1 s 时一点的间隔超过十秒；改范围只改这两个常量。与本版其他改形同一次 `WIRE_V` 进位（D22）。

**重开参数**：节拍要按会话各不相同时（两个人在两台设备上看同一座城、各要各的节拍），它改成会话的状态。
-/

