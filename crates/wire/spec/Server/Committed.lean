-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::server::committed

规定 `server::committed`（`crates/wire/src/` 下同名的文件）。一条已提交的记录与它在线上的那一帧。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `wire::server::committed` 旁的测试守住。
-/

/-!
### 8-47 一条记录只序列化一次：`Committed`

```rust
/// 一条已提交的记录，与它在线上的那一帧。克隆只加两个引用计数。
#[derive(Debug, Clone)]
pub struct Committed { /* record: Arc<EventRecord>, frame: Utf8Bytes */ }
impl Committed {
    /// 拼帧：`{"event":` ＋ 记录的 JSON ＋ `}`。
    /// # Errors  `E_WIRE_MISMATCH`：记录序列化不出来（恢复：该记录不推；下一条记录到达时 seq 断口使会话发出 `Lagged`，页面按它补拉，§8-41）。
    pub fn new(record: EventRecord) -> Result<Self, AxError>;
    pub fn record(&self) -> &EventRecord;
    pub(crate) fn frame(&self) -> Utf8Bytes;
}
```

- **seq 断口只能出自这一处**：广播的唯一写入者是 `sprawling` 的 folding 观察者，每条提交的记录都经它进这一个广播，一城一条全局 seq；所以 `Even(Some)` 时的 seq 断口只意味着某条记录没拼出帧，`decide_lag` 据此发 `Lagged`（§8-41）。在广播前过滤记录的任何改动都会让每个会话收到假 `Lagged`，须先改 `decide_lag`。
- **帧在广播之前拼好，socket 只写字节**。每个订阅者 `recv` 时广播要克隆一次载荷；载荷若是 `EventRecord`，16 个 socket 就是 16 次深拷贝加 16 次 `serde_json::to_string`。拼好的 `Utf8Bytes` 与 `Arc<EventRecord>` 让每个 socket 的代价降到两次引用计数和一次写。**被否**：socket 各自序列化——同一份字节算 N 遍，且 N 正是人开着的标签页数。
- **拼法的唯一权威是 `Committed::new`**，守护夹具断言它拼出的帧与 `serde_json::to_string(&ServerFrame::Event(..))` 逐字节相等：`ServerFrame` 是外部标签的 snake_case 枚举，`Event` 臂的形状恰是 `{"event":<记录>}`，两者一旦漂开页面读到的是另一条历史。
- **读数**：`instrument_fanout_cpu_per_event`（`--release --run-ignored only`）在 1／4／16 个订阅者上量每事件的 CPU，门槛是 16 个订阅者的每事件代价不超过 1 个订阅者的两倍——即序列化只算一次，不随标签页数增长。门槛取比值而不取绝对值，因为绝对值随机器档次变；现在的每事件代价由那一次 `EventRecord` 序列化主导（约 10 µs 量级），16 个订阅者时 ≤ 1 µs 是下一条的目标，不是现在成立的性质。
- 这份帧目前由 `EventRecord` 序列化而来；改为直接取 `append_all` 写下的账本行字节（使广播环只留一份字节，`Lagged` 补拉亦回账本字节）是本接口的下一步，前提是账本行与 `serde_json::to_vec(&EventRecord)` 逐字节相等——同一条守护夹具会判定它。
-/
