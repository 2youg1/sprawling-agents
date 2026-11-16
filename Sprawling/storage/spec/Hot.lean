-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::hot

规定 `hot`（`crates/storage/src/` 下同名的文件）。内存热视图：界面查询在此命中，不读盘。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部大半是文字，由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）；`waiting` 这一折（D29）有一个小模型与它的证明，模块旁的 proptest 从同一条性质导出。
-/

/-!
### 8-5 storage::hot（形状 7）

```rust
pub struct HotView { /* runs: BTreeMap<RunId, RunHot>、evicted: BTreeSet<RunId> —— 私有 */ }
pub struct RunHot { pub phase: RunPhase, pub last_seq: Seq, pub last_kind: EventKind, pub who: String,
                    pub addr: Option<Address>, pub started: Option<TimeMs>,     // 房间与开始时刻
                    pub completion: Option<String>, pub pr: Option<String>, pub ask: Option<String>,  // 结局、PR、所等之事
                    pub task: Option<String>, pub goal: Option<String>,                               // 人交给它的任务与目标
                    pub waiting: Option<RunWaiting> }                                                 // 停在同步 send 上等的房间与期限
pub struct RunWaiting { pub on: Address, pub until: TimeMs }
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
- **`task` 与 `goal` 从 `run_started` 记下**：那条记录的同名两个字段，与 `addr`、`started` 同一处赋值、同一个口径——只在这一种记录上写，其余记录不动它们；字段缺失或是空串记 `None`，因为一句空的任务不是一个名字。理由：run 板以它们给一行 run 起名，而重载后的页面只有 `RunSummary`（`crates/wire/Spec.lean` §8-48e）。
- **城市级记录不进 run 表**：`RunId::CITY`（nil）标记的是属于城而不属于任何 Run 的记录——创世记录、`building_created`。把它们折进 run 表会让 `active_count()` 在一座**从未派过活的城**里返回 1：城市页读服务端的这个数、写「1 run in flight」，而总览页折同一条流写「什么都没在跑」——**一个问题两个答案，而错的那个是服务端的**。
-/

/-! D29 `RunHot.waiting` 是这次跑的最后一行 `signal_wait_started` 还没被 `signal_wait_ended` 或 `run_frozen` 结束时的那对值

**决定**：`signal_wait_started`（kernel D32）把 `waiting` 设成 `{ on, until = deadline_ms }`，按键读那一行的 `on` 与 `deadline_ms`，与本节其余字段同一个口径——字段缺失或读不成就记 `None`，看不到的事不猜；`signal_wait_ended` 或 `run_frozen` 把它清掉；别的记录不动它。冻结之后到的 `signal_wait_started` 不再设它，因为冻结在热视图里是终态：一次冻结了的跑不在等任何人。一次跑同时只开一个等待（collab D9 拒绝第二个），所以结束行不按 `signal` 配对，任何一条结束行都清掉当前的等待。`until` 直接是 `deadline_ms`：那是城的注入时钟（epoch 毫秒）读出的时刻，wire D34 说的「换成墙钟」在三个平台上都是恒等。

**理由**：页面与 watchdog 要把「停着等回信」和「卡住」分开，而 `last_kind` 在两者上一样；重放与远程设备读到的状态要从账本的两行折出，所以这一折放在已经折每一条记录的热视图里，`accounting` 的 `summarize` 只是搬过去。

**被否**：①按 `signal` 配对、存下等待的 `SignalId`：一次跑只有一个开着的等待，配对只多一个字段而不多一种答案；②让 `accounting` 另折一遍这对种类：同一个状态两处折，两份答案。

**重开参数**：一次跑可以同时等两个房间时（wire D34 的重开参数），`waiting` 改成按 `signal` 键的表，结束行按 `signal` 配对。
-/
namespace Storage.Hot

/-- 一条记录在 `waiting` 这一折里是什么。房间与期限是抽象的值；其余种类一律是 `other`。 -/
inductive Line where
  | waitStarted (on deadline : Nat)
  | waitEnded
  | frozen
  | other

structure Waiting where
  on : Nat
  deadline : Nat
deriving DecidableEq

/-- 一次跑在这一折里的状态：是否已冻结，以及在等什么。 -/
structure Fold where
  frozen : Bool
  waiting : Option Waiting

def Fold.start : Fold := ⟨false, none⟩

/-- 一条记录怎样折进来，对应 `RunHot::absorb`。 -/
def step (s : Fold) : Line → Fold
  | .waitStarted on deadline => if s.frozen then s else { s with waiting := some ⟨on, deadline⟩ }
  | .waitEnded => { s with waiting := none }
  | .frozen => ⟨true, none⟩
  | .other => s

def fold (trace : List Line) : Fold := trace.foldl step Fold.start

/-- 不变式：冻结的跑不在等。 -/
def Settled (s : Fold) : Prop := s.frozen = true → s.waiting = none

theorem step_keeps_settled (s : Fold) (line : Line) (h : Settled s) : Settled (step s line) := by
  cases line with
  | waitStarted on deadline =>
    unfold step
    by_cases hf : s.frozen = true
    · simp [hf]; exact h
    · simp [hf, Settled]
  | waitEnded => intro _; rfl
  | frozen => intro _; rfl
  | other => exact h

theorem foldl_keeps_settled (trace : List Line) (s : Fold) (h : Settled s) :
    Settled (trace.foldl step s) := by
  induction trace generalizing s with
  | nil => exact h
  | cons line rest ih => exact ih (step s line) (step_keeps_settled s line h)

/-- 任何一段记录折完，一次冻结了的跑都不在等。 -/
theorem frozen_run_never_waits (trace : List Line) :
    (fold trace).frozen = true → (fold trace).waiting = none :=
  foldl_keeps_settled trace Fold.start (fun h => nomatch h)

/-- 一条结束行之后不在等。 -/
theorem ended_wait_is_gone (trace : List Line) :
    (fold (trace ++ [.waitEnded])).waiting = none := by
  simp [fold, List.foldl_append, step]

/-- 一次没冻结的跑，最后一行是开始等待时，`waiting` 正是那一行的房间与期限。 -/
theorem started_wait_is_shown (trace : List Line) (on deadline : Nat)
    (h : (fold trace).frozen = false) :
    (fold (trace ++ [.waitStarted on deadline])).waiting = some ⟨on, deadline⟩ := by
  simp only [fold, List.foldl_append, List.foldl_cons, List.foldl_nil] at *
  simp [step, h]

end Storage.Hot
