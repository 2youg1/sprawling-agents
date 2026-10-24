-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime 的机器级预约：一份机器预算，按字节预约，超出就按号排队

规定机器级预约的判定（`crates/sprawling/spec/Serving/Memory.lean` §8-173「整台机器」与待做第 6 项）：重活开始前从一份机器预算里预约自己的上界，上界之和超过预算就排队，归还之后按号放行。本文件是 `crates/runtime/Spec.lean` 的一个分部。实现它的 Rust 模块还没有写；本模型先于它，是它必须守住哪些性质的权威，那个模块写成时它的 doc comment 链回本文件，下面「从模型导出的 Rust 检查」是它的派生检查。

## 模型

账本（`Book`）有一份预算 `budget`（字节）、持有中的预约 `held`、按拿号次序排着的等待者 `waiting`，以及放行的次序 `started`（只为陈述性质而记）。一份预约（`Ask`）是一个号与它的字节数。两种事件：

* `ask a`：`a` 排到队尾，然后放行；
* `giveBack t`：号为 `t` 的预约从持有中拿掉，然后放行。

放行（`advance`）从队首起，队首「放得下」就把它挪进持有、记进 `started`，再看下一个；队首放不下就停，后面的不越过它。「放得下」是：持有中没有别人，或者持有的字节加上它的字节不超过预算。

## 三条性质，都对从空账本开始的每一条事件序列成立

1. 持有的字节不超过预算，只有一种例外：持有中恰好只有一份，它自己就超过预算（`held_within_budget`）。一份比整份预算还大的预约若要等「放得下」就永远等不到，所以它在没有别人持有时单独放行。
2. 放行的次序就是拿号的次序（`started_in_ask_order`）：`started` 接上 `waiting` 的号，恰好是事件序列里每次 `ask` 的号，次序不变。所以一份小的预约不会越过排在它前面的大预约。
3. 队伍只在有人持有时才等（`waits_only_behind_a_holder`）：等待者不空，持有就不空。所以不会出现谁都没持有、队伍却停着的状态；一旦持有的都归还了，队首就被放行。

## 从模型导出的 Rust 检查

那个 Rust 模块旁边的 `proptest`：生成 `ask`／`giveBack` 的任意序列（号单调递增，字节取 0..2×预算，让「一份超过预算」那条分支被走到，`giveBack` 的号从已发出的号里取，也取未发出的号），每一步之后断言上面三条：持有的字节不超过预算或只有一份；放行次序等于拿号次序；等待者不空时持有不空。一个「允许小预约越过放不下的队首」（回填）的版本在性质 2 上变红。
-/

/-! D103 机器级预约按号先来后到，不回填；一份超过整份预算的预约在没有别人持有时单独放行

**决定**：放行只看队首，队首放不下时后面的不越过它；队首在持有为空时总是放行，即使它比预算大。

**理由**：预约的对象是编译、测试与解释器这类大小相差悬殊的活。回填（让放得下的小预约越过放不下的队首）能让机器更满，但一条要大块内存的编译会被源源不断的小活一直压在队首，等待没有上界；先来后到的等待只取决于排在前面的那些活各自多久结束。比预算还大的预约若不单独放行就永远等不到，而它被放行时机器上没有别的重活，那是它能得到的最好条件。

**被否**：①回填：见上，大预约的等待没有上界；②比预算大的预约直接拒绝：人填的上限或估计值比机器预算大并不说明这件活做不成，拒绝它只是把决定推给模型；③复用 gateway 的并发名额（`crates/gateway/spec/Concurrency.lean` §8-40）：那是按「一次调用」计数的名额，上限随 429 收窄、随成功放宽，这里是按字节加权、预算不随结果变的预约；借的是它「按号先来后到」的规则与在任意轨迹上量化的证明形状，不共用代码。

**重开参数**：读数显示队首长期被一份大预约占着而机器大半空闲（那时考虑带时限的回填）；或者出现按内存压力信号动态收紧预算的需要。
-/

namespace Runtime.Reserve

/-- 一份预约：号与它要的字节数。 -/
structure Ask where
  ticket : Nat
  bytes : Nat
  deriving Repr, DecidableEq

/-- 账本：预算、持有中的预约、排着的等待者、放行的次序。 -/
structure Book where
  budget : Nat
  held : List Ask
  waiting : List Ask
  started : List Nat
  deriving Repr

inductive Event where
  | ask (a : Ask)
  | giveBack (ticket : Nat)

/-- 持有的字节。 -/
def heldBytes (l : List Ask) : Nat := (l.map (·.bytes)).sum

/-- 队首放得下：没有别人持有，或者加上它也不超过预算。 -/
def fits (budget : Nat) (held : List Ask) (a : Ask) : Bool :=
  held.isEmpty || decide (heldBytes held + a.bytes ≤ budget)

/-- 从队首起放行，直到队首放不下或队伍空了。 -/
def advance (budget : Nat) (held : List Ask) (started : List Nat) :
    List Ask → List Ask × List Ask × List Nat
  | [] => (held, [], started)
  | a :: rest =>
    if fits budget held a then advance budget (held ++ [a]) (started ++ [a.ticket]) rest
    else (held, a :: rest, started)

/-- 换上新的持有与等待者之后放行一次。 -/
def settle (b : Book) (held waiting : List Ask) : Book :=
  let r := advance b.budget held b.started waiting
  { b with held := r.1, waiting := r.2.1, started := r.2.2 }

def step (b : Book) : Event → Book
  | .ask a => settle b b.held (b.waiting ++ [a])
  | .giveBack t => settle b (b.held.filter (·.ticket != t)) b.waiting

def empty (budget : Nat) : Book := ⟨budget, [], [], []⟩

def replay (budget : Nat) (events : List Event) : Book :=
  events.foldl step (empty budget)

/-- 事件序列里每次 `ask` 的号，按次序。 -/
def asked : List Event → List Nat
  | [] => []
  | .ask a :: rest => a.ticket :: asked rest
  | .giveBack _ :: rest => asked rest

/-! ### 性质 1：持有的字节不超过预算，或只有一份 -/

/-- 持有的字节不超过预算，或持有中只有一份。 -/
def Within (budget : Nat) (held : List Ask) : Prop :=
  heldBytes held ≤ budget ∨ held.length = 1

theorem heldBytes_append (l : List Ask) (a : Ask) :
    heldBytes (l ++ [a]) = heldBytes l + a.bytes := by
  simp [heldBytes, List.sum_append]

theorem heldBytes_filter_le (l : List Ask) (p : Ask → Bool) :
    heldBytes (l.filter p) ≤ heldBytes l := by
  induction l with
  | nil => simp [heldBytes]
  | cons x rest ih =>
    simp only [List.filter_cons]
    split
    · simp only [heldBytes, List.map_cons, List.sum_cons] at ih ⊢; omega
    · simp only [heldBytes, List.map_cons, List.sum_cons] at ih ⊢; omega

theorem within_filter (budget : Nat) (l : List Ask) (p : Ask → Bool)
    (h : Within budget l) : Within budget (l.filter p) := by
  rcases h with hle | hone
  · exact Or.inl (Nat.le_trans (heldBytes_filter_le l p) hle)
  · have hlen : (l.filter p).length ≤ 1 := hone ▸ List.length_filter_le p l
    rcases Nat.le_one_iff_eq_zero_or_eq_one.mp hlen with h0 | h1
    · left
      rw [List.length_eq_zero_iff.mp h0]
      simp [heldBytes]
    · exact Or.inr h1

theorem within_advance (budget : Nat) (waiting : List Ask) :
    ∀ (held : List Ask) (started : List Nat), Within budget held →
      Within budget (advance budget held started waiting).1 := by
  induction waiting with
  | nil => intro held started h; simpa [advance] using h
  | cons a rest ih =>
    intro held started h
    unfold advance
    split
    · rename_i hfit
      apply ih
      unfold fits at hfit
      cases hheld : held with
      | nil => right; simp
      | cons x xs =>
        left
        rw [hheld] at hfit
        simp only [List.isEmpty_cons, Bool.false_or, decide_eq_true_eq] at hfit
        rw [heldBytes_append]
        exact hfit
    · exact h

theorem within_step (b : Book) (e : Event) (h : Within b.budget b.held) :
    Within (step b e).budget (step b e).held := by
  cases e with
  | ask a => exact within_advance b.budget _ b.held b.started h
  | giveBack t =>
    exact within_advance b.budget _ _ b.started (within_filter b.budget b.held _ h)

theorem step_budget (b : Book) (e : Event) : (step b e).budget = b.budget := by
  cases e <;> rfl

theorem foldl_budget (events : List Event) :
    ∀ b : Book, (events.foldl step b).budget = b.budget := by
  induction events with
  | nil => intro b; rfl
  | cons e rest ih => intro b; rw [List.foldl_cons, ih, step_budget]

theorem within_foldl (events : List Event) :
    ∀ b : Book, Within b.budget b.held →
      Within b.budget (events.foldl step b).held := by
  induction events with
  | nil => intro b h; exact h
  | cons e rest ih =>
    intro b h
    have h' := ih (step b e) (within_step b e h)
    rw [step_budget] at h'
    exact h'

/-- 持有的字节不超过预算，只有持有中恰好一份时例外。 -/
theorem held_within_budget (budget : Nat) (events : List Event) :
    heldBytes (replay budget events).held ≤ budget ∨ (replay budget events).held.length = 1 :=
  within_foldl events (empty budget) (by left; simp [empty, heldBytes])

/-! ### 性质 2：放行的次序就是拿号的次序 -/

theorem advance_order (budget : Nat) (waiting : List Ask) :
    ∀ (held : List Ask) (started : List Nat),
      (advance budget held started waiting).2.2 ++
          (advance budget held started waiting).2.1.map (·.ticket) =
        started ++ waiting.map (·.ticket) := by
  induction waiting with
  | nil => intro held started; simp [advance]
  | cons a rest ih =>
    intro held started
    unfold advance
    split
    · rw [ih]; simp
    · rfl

theorem step_order (b : Book) (e : Event) :
    (step b e).started ++ (step b e).waiting.map (·.ticket) =
      b.started ++ b.waiting.map (·.ticket) ++ asked [e] := by
  cases e with
  | ask a =>
    simp only [step, settle]
    rw [advance_order]
    simp [asked]
  | giveBack t =>
    simp only [step, settle]
    rw [advance_order]
    simp [asked]

theorem asked_cons (e : Event) (rest : List Event) : asked (e :: rest) = asked [e] ++ asked rest := by
  cases e <;> simp [asked]

theorem foldl_order (events : List Event) :
    ∀ b : Book, (events.foldl step b).started ++ (events.foldl step b).waiting.map (·.ticket) =
      b.started ++ b.waiting.map (·.ticket) ++ asked events := by
  induction events with
  | nil => intro b; simp [asked]
  | cons e rest ih =>
    intro b
    rw [List.foldl_cons, ih, step_order, asked_cons e rest]
    simp

/-- 放行的次序接上还在等的号，恰好是拿号的次序。 -/
theorem started_in_ask_order (budget : Nat) (events : List Event) :
    (replay budget events).started ++ (replay budget events).waiting.map (·.ticket) =
      asked events := by
  have h := foldl_order events (empty budget)
  simpa [replay, empty] using h

/-! ### 性质 3：队伍只在有人持有时才等 -/

theorem advance_waits (budget : Nat) (waiting : List Ask) :
    ∀ (held : List Ask) (started : List Nat),
      (advance budget held started waiting).2.1 = [] ∨ (advance budget held started waiting).1 ≠ [] := by
  induction waiting with
  | nil => intro held started; left; rfl
  | cons a rest ih =>
    intro held started
    unfold advance
    split
    · exact ih _ _
    · rename_i hfit
      right
      intro hnil
      subst hnil
      simp [fits] at hfit

theorem step_waits (b : Book) (e : Event) :
    (step b e).waiting = [] ∨ (step b e).held ≠ [] := by
  cases e with
  | ask a => exact advance_waits b.budget _ b.held b.started
  | giveBack t => exact advance_waits b.budget _ _ b.started

/-- 等待者不空时，持有就不空。 -/
theorem waits_only_behind_a_holder (budget : Nat) (events : List Event)
    (h : (replay budget events).waiting ≠ []) : (replay budget events).held ≠ [] := by
  have key : ∀ (es : List Event) (b : Book), (b.waiting = [] ∨ b.held ≠ []) →
      ((es.foldl step b).waiting = [] ∨ (es.foldl step b).held ≠ []) := by
    intro es
    induction es with
    | nil => intro b hb; exact hb
    | cons e rest ih => intro b _; exact ih (step b e) (step_waits b e)
  rcases key events (empty budget) (Or.inl rfl) with hw | hh
  · exact absurd hw h
  · exact hh

/-! 每条分支各走一次：放得下就放行；放不下就排队；比预算大的在空时单独放行；归还之后队首放行。 -/

example : (replay 10 [.ask ⟨0, 4⟩, .ask ⟨1, 5⟩]).held.length = 2 := by decide
example : (replay 10 [.ask ⟨0, 6⟩, .ask ⟨1, 5⟩]).waiting.length = 1 := by decide
example : (replay 10 [.ask ⟨0, 11⟩]).held.length = 1 := by decide
example : (replay 10 [.ask ⟨0, 6⟩, .ask ⟨1, 5⟩, .giveBack 0]).started = [0, 1] := by decide
example : (replay 10 [.ask ⟨0, 6⟩, .ask ⟨1, 5⟩, .ask ⟨2, 1⟩]).started = [0] := by decide

end Runtime.Reserve
