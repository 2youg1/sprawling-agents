-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::queue

规定 `queue`（`crates/storage/src/` 下同名的文件）。一份实现服务三队列；admit 先于入队，去重先于副作用。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-10 storage::queue（形状 7）

```rust
pub struct EventQueue { /* items: BTreeMap<u64, QueueItem>、next_id、seen: BTreeMap<IdemKey, TimeMs>、stats —— 私有 */ }
pub struct QueueItem { pub id: u64, pub key: IdemKey, pub payload: Payload }
impl EventQueue {
    pub fn new(lane: QueueLane, capacity: u64) -> EventQueue;
    /// Admission first (kernel::backpressure), then enqueue; Shed returns
    /// the verdict to the caller (who accounts backpressure_shed).
    pub fn enqueue(&mut self, key: IdemKey, payload: Payload, now: TimeMs) -> Result<Admission, StorageError>;
    /// Dedup before side effects: a key already consumed is Duplicate and
    /// must not reach the consumer twice.
    pub fn consume(&mut self) -> Option<QueueItem>;
    pub fn stats(&self) -> QueueStats;   pub fn len(&self) -> u64;   pub fn is_empty(&self) -> bool;
}
pub enum QueueLane { Signal, Approval, Repair }   // 一份实现三队列
```

- 容量入构造子（`new(lane, capacity)`）而非写死常量——三 lane 容量不同是装配事。**重复键返回 `Admit` 而非 `Shed`**：发送方已尽职，告知失败只会招致无效重试；`seen` 持久于队列寿命（消费后仍认得出重复，因为副作用已跑过一次）。被 shed 项不入 `seen`，故重试不算重复。
- **去重记忆按时间有界**：`seen` 记下每个键的入队时刻，`enqueue` 先按 `now` 驱逐早于 `IDEM_WINDOW_MS`（六小时）的键。
  只增不减的集合会让长跑的城为它曾经入过队的每一条事件各留一个键，而重试发生在一次投递的窗口内，不发生在一天之后。
  时钟倒退时一个键也不驱逐——队列对时钟不持观点，而记得太久只会少跑一次副作用。
  **常量住 `storage::queue`**：它只有这一个读者，放进 `kernel::consts_policy` 会让 kernel 的公开面多一个只有 storage 读的数。
- 重建性：队列状态＝（signal_enqueued − signal_consumed）的 projection；持久性不在本模块（Ledger 已是历史）。lane 只定账目名字段，三队列零分支差异——差异出现之日即分模块之日（反推式合并的退出条件写在明处）。
-/
