-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Event.Kind

/-!
# kernel::event

规定 `kernel::event`（`crates/kernel/src/event.rs` 与 `crates/kernel/src/event/` 下的 `identity`、`payload`、`scope`、`who`、`moment`）以及 `kernel::origin`（`crates/kernel/src/origin.rs`）：信封、规范字节、`EventRef` 的铸造、信封 `t` 的语义与可忽略性。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-4 kernel::event

```rust
pub struct RunId(Uuid);                 // uuid v7 仅人读标识；kernel 不生成
impl RunId { pub const CITY: RunId;     // nil UUID：city 级事件哨兵
             pub fn from_bytes([u8;16]) -> Self;  pub fn parse(&str) -> Result<Self, AxError>; }
// 反序列化：人读格式（JSON 行、帧）只收 parse 认的那一种拼写；二进制格式（views 快照的 postcard）
// 按 uuid 的 16 字节读回，与序列化对称——否则存着 RunId 的 views 快照与 twin 都解不回来。
pub struct Seq(u64);
impl Seq { pub const FIRST: Seq;        // 0；创世行
           pub fn next(self) -> Result<Seq, AxError>; }   // checked_add
pub struct TimeMs(u64);                 // UTC 整数毫秒；只入参不采样

pub enum EventKind { CityInitialized, /* …余下 variant 与两分见本节表，serde 蛇形 */ }
pub enum WindowClass { InWindow, RecordOnly }
impl EventKind {
    pub fn window_class(&self) -> WindowClass;  // 穷尽 match；二分权威
    pub const ALL: [EventKind; N];              // N＝本节表的行数；与本节表同一名册：specalign 与计数断言的数据面
}

pub struct Payload(serde_json::Map<String, Value>);
impl Payload {
    /// Sole constructor: rejects any float anywhere in the tree
    /// (determinism rule 6) and any nesting deeper than
    /// PAYLOAD_DEPTH_MAX (126: the 127 containers serde_json reads, minus the envelope).
    /// Deserialize re-validates on read.
    pub fn new(map: Map<String, Value>) -> Result<Self, AxError>;  // E_INVALID_ARGS
    pub fn empty() -> Self;
    /// 写侧唯一门：record 结构 -> 载荷；非对象即 E_INVALID_ARGS。
    pub fn of(record: &impl Serialize) -> Result<Self, AxError>;
    /// 读侧唯一门：载荷 -> record 结构；读不动即 E_WIRE_MISMATCH。
    /// 借出而不复制：serde_json 的 `&Map` 反序列化器直接读持有的载荷，
    /// 读一次只分配 T 自己拥有的字段，不先克隆整张 map。
    pub fn read<T: DeserializeOwned>(&self) -> Result<T, AxError>;
}

pub struct EventDraft {                 // 调用方给的一半：语义内容
    pub run: RunId, pub t: TimeMs, pub who: String,
    pub addr: Option<Address>, pub kind: EventKind, pub data: Payload,
    pub ig: bool,                       // 「可忽略」标记；写方默认 false
}
pub struct EventRecord { /* v, run, seq, prev, t, who, addr, kind, data, ig —— 字段私有 */ }
impl EventRecord {
    /// Adapter-side assembly: the Ledger implementation owns seq/prev/v.
    pub fn from_draft(draft: EventDraft, seq: Seq, prev: B3Hash) -> Self;  // v ＝ EVENT_LOG_V
    /// Canonical bytes: serde_json, struct field order = declaration order,
    /// payload keys sorted (serde_json BTreeMap), no trailing newline.
    pub fn canonical_line(&self) -> Result<Vec<u8>, AxError>;
    pub fn to_ref(&self) -> EventRef;   // 铸造需持有整条记录
    pub fn parse_line(raw: &[u8]) -> Result<Self, AxError>;  // 读侧：逐字段复验
    pub fn seq(&self) -> Seq;  pub fn kind(&self) -> EventKind;  pub fn v(&self) -> u32;
}
// event::moment：一行的 t 记的是不是它自己那一刻（见下「信封 t 记的是什么」）
impl EventKind { pub fn records_a_moment(&self) -> bool; }   // model_called、model_returned、tool_called、tool_result
impl EventRecord { pub fn moment(&self) -> Option<TimeMs>; }  // 记时刻的种类且 v ≥ 2 → Some(t)；其余 None
pub struct EventRef { seq: Seq, kind: EventKind }   // 字段私有；无公开构造子
// event::who：载荷里一行的行动者（信封的 who 仍是字符串，见 kernel 规格 §3 第 5 条）
pub enum Who { City, Person, Resident(Address) }    // 线上 "city" | "person" | 居民地址，与账本旧行同拼
impl Who {
    pub fn resident(addr: Address) -> Result<Who, AxError>;  // 地址拼作 city 或 person 即 E_INVALID_ARGS
    pub fn parse(raw: &str) -> Result<Who, AxError>;          // as_str 的逆；既非保留词也非规范地址即 E_INVALID_ARGS
    pub fn as_str(&self) -> &str;
    pub fn handed_down_by(&self, predecessor: Option<RunId>, parent: Option<RunId>) -> String;
        // 模型在任务文件与开场消息里读到的「由谁交下来」：居民带交活的 run（继任者优先，其次委派的父 run），
        // 敲门两者皆无只写地址；任务文件与开场消息由两个 crate 写，所以拼法只在这里（city SpineFiles D21）
}
pub fn escape_markup(text: &str) -> String;  // `<` `>` `&` 写成实体：标记段落里的唯一转义器，读回三个实体即得原文
```
-/

/-!
- 序列化细节：`addr` 为 None 与 `ig` 为 false 时省略键；其余八键恒在；键序＝声明序 `v,run,seq,prev,t,who,addr,kind,data,ig`。此即 V8 跨平台字节一致的规范。

**信封 `t` 记的是什么。** 本段是时间语义的唯一权威；读者经 `EventRecord::moment` 取用，不从导出它的构建、也不从相邻行的 `t` 推断。
- `t` 是写方经注入的时钟采到的 UTC 整数毫秒，kernel 从不采样。
- `v` 为 2 起，`model_called`、`model_returned`、`tool_called`、`tool_result` 四种行记这一行自己那件事的时刻：一次模型尝试发出、一次回复到齐、一次工具调用开始（放行之后、工具起跑之前）、一次调用答复。`EventKind::records_a_moment` 是这四种的名册，`moment` 对这样的行答 `Some(t)`。
- `v` 为 1 的行里，这四种带的是它所在回合的时间戳，同一回合的行同值。`moment` 答 `None`，意思是「这一刻没有量过」，不是零耗时。
- 其余种类的 `t` 是写方为这一行采的一次读数；一个回合里的其余行沿用回合时间戳（`crates/runtime/Spec.lean` §8-15）。
- `t` 不随 `seq` 单调：并行只读调用的开始可以早于前一条调用的答复，壁钟也会回拨。次序以 `seq` 为准。
- 重启之后由 `runtime::replay::outcome_unknown_draft` 补上的 `tool_result`（错误码 `E_TOOL_OUTCOME_UNKNOWN`）记的是城补上它的时刻，不是工具答复的时刻；量工具用时的读者跳过这样的行。

- 铸造纪律：`EventRef` 唯二铸造路径＝Ledger append 流程（适配器持刚组装的 EventRecord 调 `to_ref`）与 replay 验链后逐条 `to_ref`。字段私有，所以在 crate 外写不出 `EventRef` 的字面量。
- `parse_line` 是读侧唯一入口：serde 反序列化＋Payload 复验；未知 kind 在此报错（呈现语义见 runtime::replay 章——携 `ig` 的行例外）。
-/

namespace Kernel.Event

open Kernel.Event.Kind

/-! ### 可忽略性：`ig` 授的是跳过权，只在种类未知时有用

读者在一行的 `kind` 里读到的，要么是本构建认得的种类，要么是一个更新的构建写下的词。本构建认得的照常全解，`ig` 不改变什么；不认得的，写方标了 `ig: true` 就跳过它的类型化解读，没有标就拒读整本账本（`E_LOG_VERSION_UNSUPPORTED`，说出方向）。跳过的行照样在链上：它的字节被它的后继取过摘要（`crates/kernel/spec/Ledger.lean`），跳过的只是解读。这条规则的生产实现是 `storage::jsonl::verify` 的 `classify`（以及 `runtime::replay`），本节是它的权威。 -/

/-- 读者在一行的 `kind` 里读到的东西。 -/
inductive Spelled where
  | known (kind : EventKind)
  | unknown (word : String)

/-- 一行在读者那里的去向：类型化全解、跳过、或拒读。 -/
inductive LineClass where
  | typed (kind : EventKind)
  | skipped
  | refused (word : String)
  deriving DecidableEq

/-- 一行的种类与它的 `ig` 决定它的去向。 -/
def classify : Spelled → Bool → LineClass
  | .known kind, _ => .typed kind
  | .unknown _, true => .skipped
  | .unknown word, false => .refused word

/-- 本构建认得的种类恒被类型化全解，带不带 `ig` 都一样：`ig` 不能把一行认得的历史藏起来。 -/
theorem ig_never_hides_a_known_kind (kind : EventKind) (ig : Bool) :
    classify (.known kind) ig = .typed kind :=
  rfl

/-- 一行被跳过，当且仅当它的种类本构建不认得、且写方标了 `ig`。 -/
theorem a_line_is_skipped_exactly_when_unknown_and_marked (spelled : Spelled) (ig : Bool) :
    classify spelled ig = .skipped ↔ (∃ word, spelled = .unknown word) ∧ ig = true := by
  cases spelled <;> cases ig <;> simp [classify]

/-- 不认得又没有 `ig` 的一行被拒读，拒读带着那个词。 -/
theorem an_unmarked_unknown_kind_is_refused (word : String) :
    classify (.unknown word) false = .refused word :=
  rfl

/-- D10 信封 `t` 记这一行等来的时刻，`EVENT_LOG_V` 因此为 2

**决定**：四种等来的行的 `t` 记各自那一刻（runtime D5），账本版本随之从 1 进到 2；`EventRecord::moment` 读 `v` 与种类，答这一行的 `t` 是不是量出来的时刻。

**理由**：`v` 是每一行自带的版本，读者据它区分两种含义，不用猜。一个 `t` 同时服务结果上的戳、时间窗与调用用时，不在载荷里另加时刻字段。

**被否**：①载荷加时刻字段：一行两个时间；②不进版本，让读者按导出构建或相邻行的 `t` 是否相等推断：同一回合里真的零毫秒的两次调用与旧行分不开；③在 `run_started` 记一个时间语义标记：一次跨越二进制升级继续的 run 会让两种行落在同一个标记之下（推断：`resume` 路径能让 run 跨重启继续）。

**代价**：0.0.7 的构建读到 `v` 为 2 的行，按 `LogVersion::Ahead` 拒开整座城（`E_LOG_VERSION_UNSUPPORTED`，说出方向）。本版新增的事件种类已经让它打不开，所以人看到的结果不变，只是拒因改说版本。

**重开参数**：出现第二种需要逐行区分的时间含义时重开本条，例如回合里其余的行也改记自己的时刻。
-/
def records_a_moment : EventKind → Bool
  | .ModelCalled => true
  | .ModelReturned => true
  | .ToolCalled => true
  | .ToolResult => true
  | _ => false

/-- 一行自己的时刻（`EventRecord::moment`）：记时刻的种类、且账本版本不低于 `since`（Rust 的 `MOMENTS_SINCE_V`）时是它的 `t`，其余一律 `none`。`none` 的意思是「这一刻没有量过」，不是零耗时。 -/
def moment (since : Nat) (kind : EventKind) (v t : Nat) : Option Nat :=
  if records_a_moment kind = true ∧ since ≤ v then some t else none

/-- 早于 `since` 写下的行没有自己的时刻，哪一种都一样：那时这四种行带的是回合时间戳。 -/
theorem an_older_line_has_no_moment (since : Nat) (kind : EventKind) (v t : Nat) (older : v < since) :
    moment since kind v t = none := by
  simp only [moment]
  split
  · omega
  · rfl

/-- 有时刻的行恰是那四种之一，且它的时刻就是信封的 `t`。 -/
theorem a_moment_is_the_envelope_t (since : Nat) (kind : EventKind) (v t m : Nat)
    (measured : moment since kind v t = some m) :
    records_a_moment kind = true ∧ since ≤ v ∧ m = t := by
  simp only [moment] at measured
  split at measured
  · rename_i held
    cases measured
    exact ⟨held.1, held.2, rfl⟩
  · cases measured

/-- 记自己时刻的四种都入窗：时刻属于决定模型请求的那几种行，只入账的行沿用写方采的那一次读数。 -/
theorem a_kind_with_a_moment_is_in_window (kind : EventKind) (records : records_a_moment kind = true) :
    kind.windowClass = .InWindow := by
  cases kind <;> simp_all [records_a_moment, EventKind.windowClass]

/-- `Seq::next`：checked 加一。`bound` 是 `u64` 能装下的值的个数，到顶时拒绝（`E_INVALID_ARGS`）而不回绕，所以下一行的 `seq` 恒比上一行大一。 -/
def Seq.next (bound seq : Nat) : Option Nat :=
  if seq + 1 < bound then some (seq + 1) else none

theorem Seq.next_is_one_more (bound seq after : Nat) (stepped : Seq.next bound seq = some after) :
    after = seq + 1 := by
  simp only [Seq.next] at stepped
  split at stepped
  · cases stepped
    rfl
  · cases stepped

/-- 正常路径可实现：创世行之后的一行是 1。 -/
example : Seq.next (2 ^ 64) 0 = some 1 := by decide

end Kernel.Event
