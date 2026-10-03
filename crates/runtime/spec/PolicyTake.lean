-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime：会话中改运行策略，在下一个 `BeforeWave` 取用

本文件是 `crates/runtime/Spec.lean` 的一个分部；下面一节保留它在 runtime 规格里的标签 §8-62，别处引作 `crates/runtime/Spec.lean §8-62`。它规定 `crates/runtime/src/mode.rs`（策略格 `PolicyCell`）、`crates/runtime/src/run/lifecycle.rs`（安全点上的取用）与 `crates/runtime/src/tools/edit.rs`、`crates/runtime/src/tools/exec.rs`（写门读格）。
-/

/-!
### 8-62 策略格：一个 run 在会话中换运行策略（`crates/runtime/src/mode.rs` 的 `PolicyCell`，形状 1 判定＋形状 3 状态）

```rust
// runtime::mode
pub struct PolicyCell { /* 生效的 Arc<RwLock<kernel::RunPolicy>> 与到了还没取用的最后一条 —— 私有 */ }
impl PolicyCell {
    pub fn new(start: kernel::RunPolicy) -> PolicyCell;               // run_started.policy
    pub fn reader(&self) -> PolicyReader;                             // 交给 EditTool 与 ExecSetup，只读
    pub(crate) fn arrive(&mut self, policy: kernel::RunPolicy);       // 任一安全点到的改动，后到的盖过先到的
    pub(crate) fn take_at_wave(&mut self) -> Option<kernel::RunPolicy>; // 只有 run 的驱动循环在 SafePoint::BeforeWave 调；返回取用的那条，供追加说明
}
pub struct PolicyReader { /* 同一个格 —— 私有 */ }
impl PolicyReader { pub fn now(&self) -> kernel::RunPolicy; }  // 每次写时问一次
// runtime::turn::boundary
pub enum Interrupt { None, Cancel, Steer { source: String, text: String },
                     Policy { policy: kernel::RunPolicy } }  // 一条 run_policy_changed 到了这个 run 的房间
```

- **改动从哪来**：人经 `Command::ChangeRunPolicy` 让城写一行 `run_policy_changed`（`crates/kernel/Spec.lean` D21）；记账线程把这一行投进那个房间正在跑的 run 的 `Mailslot`，与 steer 同一扇门（`crates/sprawling/Spec.lean` §8-133）。run 在任一安全点都可能读到它，但只在 `SafePoint::BeforeWave` 取用：取之前到的那几条，最后一条算数，其余被它盖过。
- **取用做三件事，次序固定**：把格里的策略换成新的；在本回合要发给模型的下一段消息（这一波的工具结果之后）末尾追加一句说明——新的 mode、写的限制、落地方式，用词取自 `kernel::RunPolicy` 的渲染；从这一波的第一次调用起，写门按新策略判。
- **冻结的前缀一个字节不动**：`ChatRequest.tools` 从会话开始就是各 mode 常驻核心的并集（§8-60），`edit` 与 `exec` 的工具说明不带写限制的那一句（写限制只出现在追加的说明里），Resident 段的 mode 行写的是 run 开始时的 mode。所以换策略只在窗口末尾加字节，provider 的前缀缓存不失效。
- **一波之内只有一个策略**：工具读的是格，不是信箱；格只在一波开始之前变。模型在上一回合看到的是旧说明，这一波按新策略判，是取在 `BeforeWave` 的代价，换来的是一波里的几次调用不会一半按旧、一半按新。
- **合并时的准入读 run 结束时格里的策略**（§8-54）：那是这个 run 最后一次取用的；run 最后一个 `BeforeWave` 之后才到的改动没被它取用，落到下一个 run 的 `run_started.policy`。
- **平台**：纯内存与账本次序上的判定，Windows、macOS、Linux 行为一致。

- **派生检查**（Rust，`crates/runtime/src/mode.rs` 的测试 `policy_take_holds_on_every_trace`）：一个 proptest 生成器在 `{change(随机 RunPolicy), wave, call}` 上抽序列，按序列调 `arrive`、`take_at_wave` 与 `PolicyReader::now`，断言两件事——同一 `BeforeWave` 之后到下一个之前的每次读数相同且等于那一波开始时生效的策略；每个 `BeforeWave` 取的是它之前最后一条改动，没有改动就什么也不取。坏的变体：`arrive` 当场写进生效的策略，即工具读信箱里最新的那条（被否①），第一条断言在序列 `[change, call]` 与 `[call, change, call]` 上红。「每次请求的 `tools` 与已发出的字节是上一次请求的前缀」由驱动循环的测试担，见 §3。

下面的模型对任意一条输入序列证明：工具表与已发出的字节只增不改；一波里每次调用按这一波开始时格里的策略判；一波之中到的改动不改这一波；下一个 `BeforeWave` 取的是最后一条改动。
-/

namespace Runtime.PolicyTake

/-- 一个 run 依次遇到的事。`change` 是一条 `run_policy_changed` 进了信箱，`wave` 是 `SafePoint::BeforeWave`，`call` 是这一波里一次问写门的调用。 -/
inductive Step (P : Type) where
  | change (policy : P)
  | wave
  | call
  deriving DecidableEq

/-- 格里的策略、信箱里还没取用的最后一条、冻结前缀之后已经写下的说明。 -/
structure State (P N : Type) where
  inForce : P
  pending : Option P
  window : List N

variable {P N : Type}

/-- D28 **策略格：会话中换运行策略的唯一写方是 run 的驱动循环，取在 `SafePoint::BeforeWave`；工具只读格。**

格的形状：`PolicyCell` 一个写方（`run::drive` 在 `BeforeWave` 调 `take`）、多个读方（`EditTool` 与 `ExecSetup` 各持一个 `PolicyReader`，每次写时问 `now`）。改动经 `Interrupt::Policy` 到达驱动循环，与 steer 同一扇门，所以「这个 run 看到了哪一条改动」只有信箱一个答案；工具表定成各 mode 常驻核心的并集（D25），mode 只改门与追加的那句说明，于是 `ChatRequest.tools` 在整个会话里逐字节不变（roadmap A15 的定规）。
被否：①工具直接读信箱里最新的那条——一波里的几次调用会跨两个策略，下面 `eager_reading_splits_a_wave` 给出反例；②在安全点按新策略重建工具——工具说明随之变，前缀缓存失效，而且工具在一处造、在另一处换，「这件工具按什么判」有了两个答案；③任一安全点都取用——`BeforeCall` 取用时模型这一回合已按旧说明出了调用，`BeforeToolCall` 取用会在一波中间换策略，只有 `BeforeWave` 让一波整体落在一个策略里。
重开参数：一波之内需要立刻收紧（例如人要立刻停写），那时收紧走 `Cancel`，不走本格。 -/
def step (note : P → N) (s : State P N) : Step P → State P N
  | .change policy => { s with pending := some policy }
  | .wave =>
    match s.pending with
    | none => s
    | some policy => { inForce := policy, pending := none, window := s.window ++ [note policy] }
  | .call => s

/-- 一条序列走完后的状态。 -/
def run (note : P → N) (s : State P N) (trace : List (Step P)) : State P N :=
  trace.foldl (step note) s

/-- 写门按序列里每一次调用判时用的策略，按调用的次序。 -/
def enforced (note : P → N) : State P N → List (Step P) → List P
  | _, [] => []
  | s, .call :: rest => s.inForce :: enforced note s rest
  | s, .change policy :: rest => enforced note (step note s (.change policy)) rest
  | s, .wave :: rest => enforced note (step note s .wave) rest

/-- 一段序列里有几次调用。 -/
def calls : List (Step P) → Nat
  | [] => 0
  | .call :: rest => calls rest + 1
  | .change _ :: rest => calls rest
  | .wave :: rest => calls rest

/-- 一段没有 `BeforeWave` 的序列不改格。 -/
theorem no_wave_keeps_the_policy (note : P → N) (s : State P N) (seg : List (Step P))
    (within : ∀ x ∈ seg, x ≠ .wave) : (run note s seg).inForce = s.inForce := by
  induction seg generalizing s with
  | nil => rfl
  | cons x rest ih =>
    have later : ∀ y ∈ rest, y ≠ .wave := fun y hy => within y (by simp [hy])
    cases x with
    | change policy => exact ih { s with pending := some policy } later
    | wave => exact absurd rfl (within .wave (by simp))
    | call => exact ih s later

/-- 一波里每一次调用都按这一波开始时格里的策略判。 -/
theorem a_wave_is_gated_by_the_policy_at_its_start (note : P → N) (s : State P N)
    (seg : List (Step P)) (within : ∀ x ∈ seg, x ≠ .wave) :
    enforced note s seg = List.replicate (calls seg) s.inForce := by
  induction seg generalizing s with
  | nil => rfl
  | cons x rest ih =>
    have later : ∀ y ∈ rest, y ≠ .wave := fun y hy => within y (by simp [hy])
    cases x with
    | change policy => exact ih { s with pending := some policy } later
    | wave => exact absurd rfl (within .wave (by simp))
    | call => simp [enforced, calls, ih s later, List.replicate_succ]

/-- 一波之中到的改动不改这一波：去掉它，这一波每次调用判时用的策略逐个相同。 -/
theorem a_change_inside_a_wave_leaves_that_wave_alone (note : P → N) (s : State P N)
    (before after : List (Step P)) (policy : P)
    (within : ∀ x ∈ before ++ after, x ≠ .wave) :
    enforced note s (before ++ .change policy :: after) = enforced note s (before ++ after) := by
  have with_change : ∀ x ∈ before ++ .change policy :: after, x ≠ .wave := by
    intro x hx
    simp only [List.mem_append, List.mem_cons] at hx
    rcases hx with hx | rfl | hx
    · exact within x (List.mem_append_left _ hx)
    · exact fun same => by cases same
    · exact within x (List.mem_append_right _ hx)
  have counted : ∀ seg : List (Step P),
      calls (seg ++ .change policy :: after) = calls (seg ++ after) := by
    intro seg
    induction seg with
    | nil => rfl
    | cons x rest ih => cases x <;> simp [calls, ih]
  rw [a_wave_is_gated_by_the_policy_at_its_start note s _ with_change,
    a_wave_is_gated_by_the_policy_at_its_start note s _ within, counted]

/-- 只有调用的一段不碰信箱。 -/
theorem calls_keep_the_mailslot (note : P → N) (s : State P N) (seg : List (Step P))
    (only_calls : ∀ x ∈ seg, x = .call) : run note s seg = s := by
  induction seg generalizing s with
  | nil => rfl
  | cons x rest ih =>
    have later : ∀ y ∈ rest, y = .call := fun y hy => only_calls y (by simp [hy])
    rw [only_calls x (by simp)]
    exact ih s later

/-- 改动在下一个 `BeforeWave` 生效，取的是最后一条：一条改动之后、下一个 `BeforeWave` 之前只有调用，那一波开始时格里就是它。 -/
theorem the_next_wave_takes_the_last_change (note : P → N) (s : State P N) (policy : P)
    (between : List (Step P)) (only_calls : ∀ x ∈ between, x = .call) :
    (run note s (.change policy :: between ++ [.wave])).inForce = policy := by
  simp only [run, List.cons_append, List.foldl_cons, List.foldl_append]
  have kept := calls_keep_the_mailslot note { s with pending := some policy } between only_calls
  simp only [run] at kept
  simp [kept, step]

/-- 冻结的前缀之后，已经写下的字节只会在末尾增长：任一序列走完，走之前的窗口是走之后窗口的前缀。工具表不在状态里变，是因为状态里根本没有它。 -/
theorem the_window_only_grows (note : P → N) (s : State P N) (trace : List (Step P)) :
    s.window <+: (run note s trace).window := by
  induction trace generalizing s with
  | nil => exact List.prefix_refl _
  | cons x rest ih =>
    have grows : s.window <+: (step note s x).window := by
      cases x with
      | change policy => exact List.prefix_refl _
      | wave =>
        cases hp : s.pending with
        | none => simp [step, hp]
        | some policy => simp [step, hp]
      | call => exact List.prefix_refl _
    exact grows.trans (ih (step note s x))

/-- 没有改动就不追加：一个不换策略的 run，它的窗口一字节都不因策略而多。 -/
theorem no_change_appends_nothing (note : P → N) (s : State P N) (trace : List (Step P))
    (quiet : s.pending = none) (no_change : ∀ x ∈ trace, ∀ policy, x ≠ .change policy) :
    (run note s trace).window = s.window ∧ (run note s trace).pending = none := by
  induction trace generalizing s with
  | nil => exact ⟨rfl, quiet⟩
  | cons x rest ih =>
    have later : ∀ y ∈ rest, ∀ policy, y ≠ .change policy :=
      fun y hy => no_change y (by simp [hy])
    cases x with
    | change policy => exact absurd rfl (no_change (.change policy) (by simp) policy)
    | wave =>
      have same : step note s .wave = s := by simp [step, quiet]
      simp only [run, List.foldl_cons, same]
      exact ih s quiet later
    | call => exact ih s quiet later

/-- 被否的①：工具直接读信箱里最新的那条。 -/
def enforcedEager (note : P → N) : State P N → List (Step P) → List P
  | _, [] => []
  | s, .call :: rest => s.pending.getD s.inForce :: enforcedEager note s rest
  | s, .change policy :: rest => enforcedEager note (step note s (.change policy)) rest
  | s, .wave :: rest => enforcedEager note (step note s .wave) rest

/-- 反例：照①，一波里一条改动之前与之后的两次调用按两个策略判。 -/
theorem eager_reading_splits_a_wave :
    enforcedEager (fun (_ : Bool) => ()) ⟨false, none, []⟩ [.call, .change true, .call] =
      [false, true] := rfl

end Runtime.PolicyTake
