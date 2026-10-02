-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Delivery：信与派活在发出时生效

规定 `crates/collab/src/signal_tool.rs`（`collab::signal_tool`）与 `crates/collab/src/delegate_tool.rs`（`collab::delegate_tool`）在城里的投递语义，以及装配层（`accounting::worker::waking`、`accounting::worker::rooms`）为它守住的那一半。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。`spec/Inbox.lean` 管一个房间的队列内部（去重、lane、bandwidth）；本模型管全城：谁在什么时候收到哪一件。

城的投递状态：

- `sent` 是账本里的 `signal_enqueued` 行，按 seq 排；一件 Signal 的 id 就是它那一行的 seq。`send` 在调用时就追加这一行（`collab D7`），不等发信 run 冻结。
- `consumed` 是账本里的 `signal_consumed` 行。
- `held` 是房间里正在跑的 run 已在安全点取走、还没被一次落账的模型回答读过的那些（`collab D8`）。
- `running` 是此刻有 run 在跑的房间；一个房间的队列同一时刻只借给一个 run（`accounting::worker::rooms::RoomQueues::lend`）。
- `knocks` 是因为没人在而要敲门开 run 的房间。
- `waits` 是同步 `send` 停在安全点上的 run：等某个房间回信，或等到 deadline（`collab D9`）。

派活（`delegate`）在本模型里是一件发往子房间、等它回程的 `send`：调用时落账，子房间没人就敲门开 run，所以子 run 的开始不等父 run 冻结。

性质，每一条都对所有 trace 成立：

1. **seq 即发出次序**（`seq_is_send_order`）：`sent` 的 seq 恰是 `0, 1, …, next - 1`；重放读账本按 seq 得到同一个次序。
2. **信不丢**（`no_loss`、`sent_grows`）：发出的每一件，此后任何时刻要么已消费，要么被它房间里正在跑的 run 拿着，要么在它房间的队列里。
3. **不被消费两次**（`consumed_once`）：`signal_consumed` 行里同一个 seq 不出现两次。
4. **同一发信方的信按发出次序到达**（`take_is_oldest`）：安全点取到的恒是该房间队列里 seq 最小的一件，所以同一发信方（乃至所有发信方）发往一个房间的信按 seq 到达。
5. **没人在的房间只敲一次门**（`knock_once`）：一个房间在 `knocks` 里至多出现一次，且有 run 在跑的房间不被敲门。
6. **每次等待都会结束**（`wait_bounded`、`waits_end`）：每个等待的 deadline 至多在此刻之后 `patience` 拍；时钟再走 `patience + 1` 拍后没有等待剩下。两个 run 互等也由此解开。
7. **F8：被取消的 run 拿过的信重新投递**（`leave_requeues`、`consumed_stays`）：一个 run 无论以 Done、失败还是取消结束，它拿着而没被落账的回答读过的信回到房间队列，并为那个房间敲一次门；已消费的不再回来。

`withoutRequeue_loses` 是咬得动的演示：照今天的代码在安全点取走即记消费，被取消的 run 拿走的那件就既不在队列里，也没有人读过。

模型与平台无关：时间由注入的时钟一拍一拍走（`tick`），Windows、macOS、Linux 上同一段 trace 得同一个结果。
-/

namespace Collab.Delivery

/-- 一件 Signal：账本 seq、发信房间、收信房间。 -/
structure Signal where
  seq : Nat
  sender : Nat
  room : Nat
  deriving DecidableEq, Repr

/-- 一个停在安全点上的同步发信 run：它的房间、它等谁回信、等到哪一拍。 -/
structure Wait where
  room : Nat
  on : Nat
  deadline : Nat
  deriving DecidableEq, Repr

structure City where
  next : Nat
  sent : List Signal
  consumed : List Nat
  held : List Signal
  running : List Nat
  knocks : List Nat
  clock : Nat
  waits : List Wait
  deriving DecidableEq, Repr

def City.empty : City := ⟨0, [], [], [], [], [], 0, []⟩

/-- 一个房间的队列：发给它的、没消费、没被拿着的那些，按 seq。它只由账本行推出，所以重放得到同一个队列。 -/
def City.queue (c : City) (r : Nat) : List Signal :=
  c.sent.filter fun s => decide (s.room = r) && decide (s.seq ∉ c.consumed) && decide (s ∉ c.held)

/-- 城里会发生的事。`leave` 不分 Done、失败与取消：一个 run 离开房间就是离开。 -/
inductive Act where
  | send (sender room : Nat)
  | start (room : Nat)
  | take (room : Nat)
  | ack (s : Signal)
  | leave (room : Nat)
  | park (room on : Nat)
  | tick
  deriving DecidableEq, Repr

/-- 敲门：房间没人在跑、也没有一次敲门在等，才敲。 -/
def City.knock (c : City) (r : Nat) : List Nat :=
  if r ∈ c.running ∨ r ∈ c.knocks then c.knocks else c.knocks ++ [r]

/-- 一个 run 离开房间：房间空出来，它拿着的信放回去，它的等待一并结束。 -/
def City.vacate (c : City) (r : Nat) : City :=
  { c with
    running := c.running.filter (· ≠ r)
    held := c.held.filter (·.room ≠ r)
    waits := c.waits.filter (·.room ≠ r) }

def step (patience : Nat) (c : City) : Act → City
  | .send a r =>
    { c with
      next := c.next + 1
      sent := c.sent ++ [⟨c.next, a, r⟩]
      knocks := c.knock r
      waits := c.waits.filter fun w => !(decide (w.room = r) && decide (w.on = a)) }
  | .start r =>
    if r ∈ c.running then c
    else { c with running := r :: c.running, knocks := c.knocks.filter (· ≠ r) }
  | .take r =>
    if r ∈ c.running then
      match c.queue r with
      | [] => c
      | s :: _ => { c with held := c.held ++ [s] }
    else c
  | .ack s =>
    if s ∈ c.held then
      { c with held := c.held.filter (·.seq ≠ s.seq), consumed := c.consumed ++ [s.seq] }
    else c
  | .leave r =>
    if r ∈ c.running then
      if (c.vacate r).queue r = [] then c.vacate r
      else { c.vacate r with knocks := (c.vacate r).knock r }
    else c
  | .park r o =>
    if r ∈ c.running then { c with waits := c.waits ++ [⟨r, o, c.clock + patience⟩] } else c
  | .tick =>
    { c with clock := c.clock + 1, waits := c.waits.filter fun w => decide (c.clock + 1 < w.deadline) }

/-- 一段 trace 之后的城。 -/
def run (patience : Nat) (as : List Act) : City := as.foldl (step patience) City.empty

/-! ## 不变式 -/

/-- 每条 trace 都守住的那组事实。 -/
structure Inv (patience : Nat) (c : City) : Prop where
  order : c.sent.map Signal.seq = List.range c.next
  once : c.consumed.Nodup
  heldFresh : ∀ s ∈ c.held, s.seq ∉ c.consumed
  heldSent : ∀ s ∈ c.held, s ∈ c.sent
  heldRunning : ∀ s ∈ c.held, s.room ∈ c.running
  knocksOnce : c.knocks.Nodup
  knocksIdle : ∀ k ∈ c.knocks, k ∉ c.running
  waitsBounded : ∀ w ∈ c.waits, w.deadline ≤ c.clock + patience

theorem inv_empty (patience : Nat) : Inv patience City.empty := by
  refine ⟨rfl, List.nodup_nil, ?_, ?_, ?_, List.nodup_nil, ?_, ?_⟩ <;> simp [City.empty]

theorem mem_queue {c : City} {r : Nat} {s : Signal} :
    s ∈ c.queue r ↔ s ∈ c.sent ∧ s.room = r ∧ s.seq ∉ c.consumed ∧ s ∉ c.held := by
  simp [City.queue, and_assoc]

theorem knock_nodup {c : City} {r : Nat} (h : c.knocks.Nodup) : (c.knock r).Nodup := by
  unfold City.knock
  split
  · exact h
  · rename_i hr
    rw [List.nodup_append]
    refine ⟨h, List.pairwise_singleton _ r, ?_⟩
    intro a ha b hb
    simp only [List.mem_singleton] at hb
    subst hb
    intro e; subst e
    exact hr (Or.inr ha)

theorem knock_idle {c : City} {r : Nat} (h : ∀ k ∈ c.knocks, k ∉ c.running) :
    ∀ k ∈ c.knock r, k ∉ c.running := by
  unfold City.knock
  split
  · exact h
  · rename_i hr
    intro k hk
    simp only [List.mem_append, List.mem_singleton] at hk
    rcases hk with hk | hk
    · exact h k hk
    · subst hk; exact fun m => hr (Or.inl m)

theorem knock_mem {c : City} {r : Nat} (h : r ∉ c.running) : r ∈ c.knock r := by
  unfold City.knock
  split
  · rename_i hr
    rcases hr with hr | hr
    · exact absurd hr h
    · exact hr
  · simp

theorem knock_sub {c : City} {r : Nat} : ∀ k ∈ c.knocks, k ∈ c.knock r := by
  intro k hk
  unfold City.knock
  split
  · exact hk
  · simp [hk]

theorem step_inv {patience : Nat} {c : City} (h : Inv patience c) (a : Act) :
    Inv patience (step patience c a) := by
  obtain ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩ := h
  cases a with
  | send a r =>
    refine ⟨?_, once, heldFresh, ?_, heldRunning, knock_nodup knocksOnce, knock_idle knocksIdle, ?_⟩
    · simp [step, List.range_succ, order]
    · intro s hs; simp only [step, List.mem_append]; exact Or.inl (heldSent s hs)
    · intro w hw
      simp only [step, List.mem_filter] at hw
      exact waitsBounded w hw.1
  | start r =>
    simp only [step]
    split
    · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
    · refine ⟨order, once, heldFresh, heldSent, ?_, ?_, ?_, waitsBounded⟩
      · intro s hs; exact List.mem_cons_of_mem _ (heldRunning s hs)
      · exact List.Pairwise.filter _ knocksOnce
      · intro k hk
        simp only [List.mem_filter, decide_eq_true_eq] at hk
        simp only [List.mem_cons, not_or]
        exact ⟨hk.2, knocksIdle k hk.1⟩
  | take r =>
    simp only [step]
    split
    · rename_i hr
      split
      · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
      · rename_i s rest hq
        have hs : s ∈ c.queue r := by rw [hq]; exact List.mem_cons_self
        obtain ⟨sent, room, fresh, _⟩ := mem_queue.mp hs
        refine ⟨order, once, ?_, ?_, ?_, knocksOnce, knocksIdle, waitsBounded⟩
        · intro t ht
          simp only [List.mem_append, List.mem_singleton] at ht
          rcases ht with ht | ht
          · exact heldFresh t ht
          · subst ht; exact fresh
        · intro t ht
          simp only [List.mem_append, List.mem_singleton] at ht
          rcases ht with ht | ht
          · exact heldSent t ht
          · subst ht; exact sent
        · intro t ht
          simp only [List.mem_append, List.mem_singleton] at ht
          rcases ht with ht | ht
          · exact heldRunning t ht
          · subst ht; rw [room]; exact hr
    · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
  | ack s =>
    simp only [step]
    split
    · rename_i hs
      refine ⟨order, ?_, ?_, ?_, ?_, knocksOnce, knocksIdle, waitsBounded⟩
      · rw [List.nodup_append]
        refine ⟨once, List.pairwise_singleton _ _, ?_⟩
        intro a ha b hb
        simp only [List.mem_singleton] at hb
        subst hb
        intro e; subst e
        exact heldFresh s hs ha
      · intro t ht
        simp only [List.mem_filter, decide_eq_true_eq] at ht
        simp only [List.mem_append, List.mem_singleton, not_or]
        exact ⟨heldFresh t ht.1, ht.2⟩
      · intro t ht
        simp only [List.mem_filter] at ht
        exact heldSent t ht.1
      · intro t ht
        simp only [List.mem_filter] at ht
        exact heldRunning t ht.1
    · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
  | leave r =>
    simp only [step]
    split
    · have running' : ∀ s ∈ c.held.filter (·.room ≠ r), s.room ∈ c.running.filter (· ≠ r) := by
        intro s hs
        simp only [List.mem_filter, decide_eq_true_eq] at hs
        simp only [List.mem_filter, decide_eq_true_eq]
        exact ⟨heldRunning s hs.1, hs.2⟩
      have fresh' : ∀ s ∈ c.held.filter (·.room ≠ r), s.seq ∉ c.consumed := by
        intro s hs
        simp only [List.mem_filter] at hs
        exact heldFresh s hs.1
      have sent' : ∀ s ∈ c.held.filter (·.room ≠ r), s ∈ c.sent := by
        intro s hs
        simp only [List.mem_filter] at hs
        exact heldSent s hs.1
      have idle' : ∀ k ∈ c.knocks, k ∉ c.running.filter (· ≠ r) := by
        intro k hk m
        simp only [List.mem_filter] at m
        exact knocksIdle k hk m.1
      have waits' : ∀ w ∈ c.waits.filter (·.room ≠ r), w.deadline ≤ c.clock + patience := by
        intro w hw
        simp only [List.mem_filter] at hw
        exact waitsBounded w hw.1
      by_cases hq : (c.vacate r).queue r = []
      · rw [if_pos hq]
        exact ⟨order, once, fresh', sent', running', knocksOnce, idle', waits'⟩
      · rw [if_neg hq]
        exact ⟨order, once, fresh', sent', running', knock_nodup knocksOnce,
          knock_idle idle', waits'⟩
    · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
  | park r o =>
    simp only [step]
    split
    · refine ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, ?_⟩
      intro w hw
      simp only [List.mem_append, List.mem_singleton] at hw
      rcases hw with hw | hw
      · exact waitsBounded w hw
      · subst hw; exact Nat.le_refl _
    · exact ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, waitsBounded⟩
  | tick =>
    refine ⟨order, once, heldFresh, heldSent, heldRunning, knocksOnce, knocksIdle, ?_⟩
    intro w hw
    simp only [step, List.mem_filter] at hw
    have := waitsBounded w hw.1
    simp only [step]
    omega

theorem foldl_inv {patience : Nat} {c : City} (h : Inv patience c) (as : List Act) :
    Inv patience (as.foldl (step patience) c) := by
  induction as generalizing c with
  | nil => exact h
  | cons a as ih => exact ih (step_inv h a)

theorem run_inv (patience : Nat) (as : List Act) : Inv patience (run patience as) :=
  foldl_inv (inv_empty patience) as

/-! ## 1 seq 即发出次序 -/

theorem seq_is_send_order (patience : Nat) (as : List Act) :
    (run patience as).sent.map Signal.seq = List.range (run patience as).next :=
  (run_inv patience as).order

/-! ## 2 信不丢 -/

theorem step_sent_grows {patience : Nat} (c : City) (a : Act) :
    c.sent <+: (step patience c a).sent := by
  cases a with
  | send a r => exact List.prefix_append _ _
  | start r => simp only [step]; split <;> exact List.prefix_refl _
  | take r =>
    simp only [step]
    split
    · split <;> exact List.prefix_refl _
    · exact List.prefix_refl _
  | ack s => simp only [step]; split <;> exact List.prefix_refl _
  | leave r =>
    simp only [step]
    split
    · split <;> exact List.prefix_refl _
    · exact List.prefix_refl _
  | park r o => simp only [step]; split <;> exact List.prefix_refl _
  | tick => exact List.prefix_refl _

/-- 账本只追加：一件发出的 Signal 此后一直在 `sent` 里。 -/
theorem sent_grows (patience : Nat) (as bs : List Act) :
    (run patience as).sent <+: (run patience (as ++ bs)).sent := by
  unfold run
  rw [List.foldl_append]
  generalize as.foldl (step patience) City.empty = c
  induction bs generalizing c with
  | nil => exact List.prefix_refl _
  | cons b bs ih => exact List.IsPrefix.trans (step_sent_grows c b) (ih _)

/-- 发出的每一件，此刻要么已消费，要么被它房间里正在跑的 run 拿着，要么在它房间的队列里。 -/
theorem no_loss (patience : Nat) (as : List Act) (s : Signal) (hs : s ∈ (run patience as).sent) :
    let c := run patience as
    s.seq ∈ c.consumed ∨ (s ∈ c.held ∧ s.room ∈ c.running) ∨ s ∈ c.queue s.room := by
  intro c
  by_cases hc : s.seq ∈ c.consumed
  · exact Or.inl hc
  · by_cases hh : s ∈ c.held
    · exact Or.inr (Or.inl ⟨hh, (run_inv patience as).heldRunning s hh⟩)
    · exact Or.inr (Or.inr (mem_queue.mpr ⟨hs, rfl, hc, hh⟩))

/-! ## 3 不被消费两次 -/

theorem consumed_once (patience : Nat) (as : List Act) : (run patience as).consumed.Nodup :=
  (run_inv patience as).once

/-! ## 4 按发出次序到达 -/

theorem sent_ascending (patience : Nat) (as : List Act) :
    (run patience as).sent.Pairwise fun a b => a.seq < b.seq := by
  have h := List.pairwise_lt_range (n := (run patience as).next)
  rw [← seq_is_send_order, List.pairwise_map] at h
  exact h

/-- 安全点取到的是该房间队列里 seq 最小的一件。 -/
theorem take_is_oldest (patience : Nat) (as : List Act) (r : Nat) (s : Signal) (rest : List Signal)
    (hq : (run patience as).queue r = s :: rest) : ∀ t ∈ rest, s.seq < t.seq := by
  have h : ((run patience as).queue r).Pairwise fun a b => a.seq < b.seq :=
    List.Pairwise.filter _ (sent_ascending patience as)
  rw [hq, List.pairwise_cons] at h
  exact h.1

/-! ## 5 只敲一次门 -/

theorem knock_once (patience : Nat) (as : List Act) :
    (run patience as).knocks.Nodup ∧ ∀ k ∈ (run patience as).knocks, k ∉ (run patience as).running :=
  ⟨(run_inv patience as).knocksOnce, (run_inv patience as).knocksIdle⟩

/-! ## 6 每次等待都会结束 -/

theorem wait_bounded (patience : Nat) (as : List Act) :
    ∀ w ∈ (run patience as).waits, w.deadline ≤ (run patience as).clock + patience :=
  (run_inv patience as).waitsBounded

/-- 时钟走 `n` 拍，其间别的什么都不发生。 -/
def ticks (patience : Nat) : Nat → City → City
  | 0, c => c
  | n + 1, c => step patience (ticks patience n c) .tick

theorem ticks_clock (patience n : Nat) (c : City) : (ticks patience n c).clock = c.clock + n := by
  induction n with
  | zero => rfl
  | succ n ih => simp only [ticks, step, ih]; omega

theorem tick_waits {patience : Nat} (x : City) (w : Wait) :
    w ∈ (step patience x .tick).waits ↔ w ∈ x.waits ∧ x.clock + 1 < w.deadline := by
  simp [step]

/-- 走 `n + 1` 拍之后还在等的，原本就在等，且 deadline 晚于出发时刻之后 `n + 1` 拍。 -/
theorem ticks_waits (patience n : Nat) (c : City) :
    ∀ w ∈ (ticks patience (n + 1) c).waits, w ∈ c.waits ∧ c.clock + (n + 1) < w.deadline := by
  induction n with
  | zero =>
    intro w hw
    have h := (tick_waits (patience := patience) c w).mp hw
    exact ⟨h.1, by omega⟩
  | succ n ih =>
    intro w hw
    have h := (tick_waits (patience := patience) (ticks patience (n + 1) c) w).mp hw
    have prev := ih w h.1
    have clk := ticks_clock patience (n + 1) c
    refine ⟨prev.1, ?_⟩
    have := h.2
    omega

/-- 从任何可达的城出发，时钟走 `patience + 1` 拍后没有等待剩下：回信没来也一样，两个 run 互等也一样。 -/
theorem waits_end (patience : Nat) (as : List Act) :
    (ticks patience (patience + 1) (run patience as)).waits = [] := by
  apply List.eq_nil_iff_forall_not_mem.mpr
  intro w hw
  obtain ⟨hw0, late⟩ := ticks_waits patience patience (run patience as) w hw
  have := wait_bounded patience as w hw0
  omega

/-! ## 7 F8：被取消的 run 拿过的信重新投递 -/

/-- 一个 run 离开房间（Done、失败或取消），它拿着而没被落账的信回到房间队列，并为那个房间敲一次门。 -/
theorem leave_requeues (patience : Nat) (as : List Act) (r : Nat) (s : Signal)
    (hr : r ∈ (run patience as).running) (hs : s ∈ (run patience as).held) (room : s.room = r) :
    let c' := step patience (run patience as) (.leave r)
    s ∈ c'.queue r ∧ r ∈ c'.knocks := by
  intro c'
  have inv := run_inv patience as
  have hv : s ∈ ((run patience as).vacate r).queue r := by
    apply mem_queue.mpr
    refine ⟨inv.heldSent s hs, room, inv.heldFresh s hs, ?_⟩
    simp [City.vacate, room]
  have ne : ((run patience as).vacate r).queue r ≠ [] := List.ne_nil_of_mem hv
  have idle : r ∉ ((run patience as).vacate r).running := by simp [City.vacate]
  simp only [c', step, hr, if_true, ne, if_false]
  exact ⟨hv, knock_mem idle⟩

/-- 已消费的不再回来：`signal_consumed` 行只追加。 -/
theorem consumed_stays {patience : Nat} (c : City) (a : Act) :
    c.consumed <+: (step patience c a).consumed := by
  cases a with
  | send a r => exact List.prefix_refl _
  | start r => simp only [step]; split <;> exact List.prefix_refl _
  | take r =>
    simp only [step]
    split
    · split <;> exact List.prefix_refl _
    · exact List.prefix_refl _
  | ack s =>
    simp only [step]
    split
    · exact List.prefix_append _ _
    · exact List.prefix_refl _
  | leave r =>
    simp only [step]
    split
    · split <;> exact List.prefix_refl _
    · exact List.prefix_refl _
  | park r o => simp only [step]; split <;> exact List.prefix_refl _
  | tick => exact List.prefix_refl _

/-!
## 咬得动的演示

今天的代码在安全点取走一件就当场记 `Consumed`（`collab::signal_tool::SignalDesk::take_steer`、`pull`）。把 `take` 写成那样，被取消的 run 拿走的那件就既不在队列里、也不在任何 run 手里，而它的回答从没落账：信丢了。下面的定理就是那条反例。
-/

def stepConsumingAtTake (patience : Nat) (c : City) : Act → City
  | .take r =>
    if r ∈ c.running then
      match c.queue r with
      | [] => c
      | s :: _ => { c with consumed := c.consumed ++ [s.seq] }
    else c
  | a => step patience c a

theorem withoutRequeue_loses :
    let c := [Act.send 0 1, .start 1, .take 1, .leave 1].foldl (stepConsumingAtTake 5) City.empty
    c.queue 1 = [] ∧ c.held = [] ∧ c.knocks = [] := by
  decide

end Collab.Delivery
