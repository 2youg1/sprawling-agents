-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::hot

规定 `hot`（`crates/storage/src/` 下同名的文件）。内存热视图：界面查询在此命中，不读盘。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-5 storage::hot（形状 7）

```rust
pub struct HotView { /* runs: BTreeMap<RunId, RunHot>、evicted: BTreeSet<RunId> —— 私有 */ }
pub struct RunHot { pub phase: RunPhase, pub last_seq: Seq, pub last_kind: EventKind, pub who: String,
                    pub addr: Option<Address>, pub started: Option<TimeMs>,     // 房间与开始时刻
                    pub completion: Option<String>, pub pr: Option<String>, pub ask: Option<String>,  // 结局、PR、所等之事
                    pub task: Option<String>, pub goal: Option<String> }                              // 人交给它的任务与目标
pub enum RunPhase { Active, Frozen }
impl HotView {
    pub fn new() -> HotView;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError>;   // 增量；重复 seq 幂等（只前进）
    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunHot)>;              // BTreeMap 序
    pub fn get(&self, run: &RunId) -> Option<&RunHot>;
    pub fn was_evicted(&self, run: &RunId) -> bool;                            // 墓碑：这次跑冻结后被逐出
    pub fn active_count(&self) -> u64;  pub fn frozen_count(&self) -> u64;
}
pub const RECENT_FROZEN: usize = 32;
```

- **热视图只留活跃的跑和最近冻结的 `RECENT_FROZEN` 个**：一次跑冻结后，若留着的冻结跑超过 `RECENT_FROZEN`，`last_seq` 最小的那个被逐出，只留一块墓碑（它的 RunId）。所以 `runs()` 本身就是一页城景该带的那几次跑，城景的大小只随活跃数增长，热视图的内存也一样（墓碑每次跑 16 字节）；更早的冻结跑经分页的 `History`／`RunHistory` 读，`frozen_count` 把墓碑也数进去，页面知道列表之外还有多少。每次冻结至多逐出一个，找最小 `last_seq` 扫一遍留着的跑，O(活跃＋N)，不另建按 seq 排的索引。`RECENT_FROZEN` 是线上答复的大小上界，不随机器变，所以是常量；它只在这里定义一次。
- **落在墓碑上的记录归冷的一侧**：冻结在热视图里是终态，被逐出的跑不会再活跃，所以一条记录的 RunId 在墓碑里时，`apply` 什么也不改——那条记录在账本里，分页的历史读得到它。没有墓碑的话，一条没有开场的尾巴会被当成一次新跑的检查点，把旧跑重新记成活跃的（活跃数多一，城景多一行）。

- 界面查询在此命中不读盘；run_started→Active，run_frozen→Frozen；其余事件只推进 last_seq/last_kind。
- **`addr` 与 `started` 从 `run_started` 记下**：`record.addr()` 是这次跑的房间，`record.t()` 是它开始的时刻；二者只在这一种记录上赋值，其余记录不动它们，所以一次跑的房间不会被后来的城市级记录改写。`Option`，因为热视图可能在 `run_started` 之前先看到同一次跑的 `checkpoint_committed`（检查点先于开场落账），也可能只看到一段没有开场的尾巴——**看不到的事不猜**。理由：`RunSummary.who` 是首条记录的作者、恒为 `city`，单靠它无法把一次跑归到 `hall/mayor` 这个房间，「与 Mayor 的对话」就在线上拼不出来。
- **`completion`、`pr`、`ask` 各从一种记录记下**：`run_frozen` 的 `completion` 字段；`pr_opened` 的 `branch` 字段（后一条覆盖前一条）；`approval_requested` 的 `action_desc` 字段，且只活到这次跑的下一条记录——任何别的记录清掉它，所以 `ask` 有值当且仅当 `last_kind` 是 `approval_requested`，页面据 `last_kind` 判「在等」，据 `ask` 写「等什么」，两者同源。按键读字段而不整条 `Payload::read`：热视图每条记录都折，整条反序列化要复制整个 map；字段缺失记 `None`，同 `addr` 的口径，看不到的事不猜。
- **`task` 与 `goal` 从 `run_started` 记下**：那条记录的同名两个字段，与 `addr`、`started` 同一处赋值、同一个口径——只在这一种记录上写，其余记录不动它们；字段缺失或是空串记 `None`，因为一句空的任务不是一个名字。理由：run 板以它们给一行 run 起名，而重载后的页面只有 `RunSummary`（wire-SPEC §8-48e）。
- **城市级记录不进 run 表**：`RunId::CITY`（nil）标记的是属于城而不属于任何 Run 的记录——创世记录、`building_created`。把它们折进 run 表会让 `active_count()` 在一座**从未派过活的城**里返回 1：城市页读服务端的这个数、写「1 run in flight」，而总览页折同一条流写「什么都没在跑」——**一个问题两个答案，而错的那个是服务端的**。
-/
