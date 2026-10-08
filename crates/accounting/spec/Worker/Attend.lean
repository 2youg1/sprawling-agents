-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 记账线程与它唯一的收件队列

规定 `crates/accounting/src/worker/attend.rs` 里的循环 `attend`：写一座城 Ledger 的唯一线程，以及它服务的三张嘴——一条 lane 的中转请求、一个从 lane 回家的 run、一条来自 desk 的命令——外加 desk 关门。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（`crates/sprawling/Spec.lean` §8-42-4）。

每张嘴都送进同一条队列。线程阻塞在第一条消息上（`recv`），取走已经排在它后面的每一条（`try_recv` 直到取空），按到达次序服务这一批。排程欠下的截止时刻到了，与来一条消息一样唤醒它。

三条性质，各一组定理：

* **每条消息在有限步内被服务**——线程醒来时正在等的消息由这次醒来服务，所以已到达的消息由下一次醒来服务；
* **追加次序即 seq 次序**——线程追加的记录从第一条起带连续的 seq，次序与追加次序相同；
* **不忙等**——没有消息在等、也没有截止时刻到期时线程不醒，而每次醒来都消耗工作，所以醒来的次数不超过引起它们的消息与截止时刻。

被服务的消息除了追加之外还做什么（结算一个 run、回答一个人），归 Rust 代码，这里不建模；这里只建模它追加几条记录。
-/

namespace Accounting.Worker.Attend

/-- 到达记账线程的东西。每个分支带着服务它要追加几条记录；关门在这里不追加，因为一座正在关的城要写的，由它仍在服务的消息写。 -/
inductive Message where
  | relay (records : Nat)
  | home (records : Nat)
  | command (records : Nat)
  | close
  deriving Repr, DecidableEq

/-- 服务一条消息追加几条记录。 -/
def Message.records : Message → Nat
  | .relay n | .home n | .command n => n
  | .close => 0

/-- 线程的状态有四项：队列，按到达次序；至今追加过的每条记录的 seq，按追加次序；服务过的每条消息，按服务次序；以及排程的截止时刻是否已到。 -/
structure Loop where
  inbox : List Message
  ledger : List Nat
  served : List Message
  deadlineDue : Bool

/-- 发送方把消息放在队尾，从不调整次序。 -/
def arrive (m : Message) (s : Loop) : Loop :=
  { s with inbox := s.inbox ++ [m] }

/-- 排程欠下的截止时刻到了。 -/
def due (s : Loop) : Loop :=
  { s with deadlineDue := true }

/-- 服务一条消息就追加它的记录，每条取下一个 seq。 -/
def serve (ledger : List Nat) (m : Message) : List Nat :=
  ledger ++ (List.range m.records).map (ledger.length + ·)

/-- 醒来一次之后的循环：整批按到达次序服务完，截止时刻已读过。 -/
def drained (s : Loop) : Loop :=
  { inbox := []
    ledger := s.inbox.foldl serve s.ledger
    served := s.served ++ s.inbox
    deadlineDue := false }

/-- 线程的一次醒来；它在 `recv` 里睡着时为 `none`。 -/
def wake (s : Loop) : Option Loop :=
  if s.inbox.isEmpty && !s.deadlineDue then none else some (drained s)

/-! ## 每条消息在有限步内被服务 -/

theorem wake_serves_every_waiting (s : Loop) (h : s.inbox ≠ []) :
    wake s = some (drained s) ∧ (drained s).inbox = [] ∧
      (drained s).served = s.served ++ s.inbox := by
  simp [wake, h, drained]

theorem arrived_is_served_by_the_next_wake (m : Message) (s : Loop) :
    ∃ s', wake (arrive m s) = some s' ∧ m ∈ s'.served := by
  refine ⟨drained (arrive m s), ?_, ?_⟩
  · exact (wake_serves_every_waiting (arrive m s) (by simp [arrive])).1
  · simp [drained, arrive]

/-! ## 追加次序即 seq 次序 -/

/-- 账本的 seq 按追加次序是 `0, 1, 2, …`。 -/
def Ordered (ledger : List Nat) : Prop :=
  ledger = List.range ledger.length

theorem serve_keeps_order {ledger : List Nat} (m : Message) (h : Ordered ledger) :
    Ordered (serve ledger m) := by
  unfold Ordered serve
  rw [List.length_append, List.length_map, List.length_range, List.range_add, ← h]

theorem batch_keeps_order (batch : List Message) {ledger : List Nat}
    (h : Ordered ledger) : Ordered (batch.foldl serve ledger) := by
  induction batch generalizing ledger with
  | nil => exact h
  | cons m rest ih => exact ih (serve_keeps_order m h)

theorem wake_keeps_order {s s' : Loop} (h : Ordered s.ledger) (w : wake s = some s') :
    Ordered s'.ledger := by
  unfold wake at w
  split at w
  · contradiction
  · cases w
    exact batch_keeps_order s.inbox h

/-! ## 不忙等 -/

/-- 一次醒来能消耗的工作：在等的消息，和已到期的截止时刻。 -/
def work (s : Loop) : Nat :=
  s.inbox.length + if s.deadlineDue then 1 else 0

theorem idle_does_not_wake (s : Loop) (empty : s.inbox = [])
    (quiet : s.deadlineDue = false) : wake s = none := by
  simp [wake, empty, quiet]

theorem every_wake_consumes_work {s s' : Loop} (w : wake s = some s') :
    work s' < work s := by
  unfold wake at w
  split at w
  · contradiction
  · rename_i busy
    cases w
    cases hi : s.inbox <;> cases hd : s.deadlineDue <;>
      simp_all [work, drained]

/-! ## 名字在这条线程之外去要，开房间仍是第一次写

派给一座还没有房间的楼的活，要向摘要模型要一个名字，这次调用要等 provider 好几秒。下面的步骤是这样一次派活，按发生次序排列（`assembly::dispatching::asking_name`）：同意这一步什么也不写，名字回来之后再问一次，因为其间可能已经到了一次停摆。 -/

/-- 需要起名的一次派活里的一步。 -/
inductive DispatchStep where
  | agree
  | askName
  | openRoom
  | laterWrite
  deriving Repr, DecidableEq

/-- 这一步是否往盘上写东西。 -/
def DispatchStep.writes : DispatchStep → Bool
  | .openRoom | .laterWrite => true
  | .agree | .askName => false

/-- 这一步是否跑在记账线程上；跑在那里，每个中转请求都要等它。 -/
def DispatchStep.onAccountingThread : DispatchStep → Bool
  | .askName => false
  | .agree | .openRoom | .laterWrite => true

def namedDispatch : List DispatchStep := [.agree, .askName, .agree, .openRoom, .laterWrite]

theorem the_naming_wait_is_off_the_accounting_thread :
    ∀ step ∈ namedDispatch, step.onAccountingThread = false → step = .askName := by decide

/-! ## 历史证明之前，服务照常、写一条也不写

服务中的城从快照起步，快照之前的历史由后台的证明走一遍（`crates/sprawling/Spec.lean` §8-122）。证明有三个结局之前的状态：还在走（`pending`）、链完好（`whole`）、链断了（`broken`）。写者在 `whole` 之前与 `broken` 之后拒绝每一次追加：一条命令照样被服务——人收到 `E_HISTORY_UNPROVEN` 或断链的原因——只是什么也不写（Q3 选的是拒绝，不是扣住：扣住的命令要一个新的唤醒理由，才能不破坏 `idle_does_not_wake`）。

这里把证明状态作为服务一批的参数：它只改变追加几条，不改变谁被服务、何时醒来，所以上面三组性质原样成立（`proved_is_the_ungated_loop`）。关门在 Rust 里先等证明有结局再写交接（`attend` 的关门一臂），这一步是等待而不是服务，不在这里建模。 -/

/-- 后台证明走到哪一步。 -/
inductive Proof where
  | pending
  | whole
  | broken
  deriving Repr, DecidableEq

/-- 在证明状态 `p` 下服务一条消息：只有 `whole` 时才追加。 -/
def serveUnder (p : Proof) (ledger : List Nat) (m : Message) : List Nat :=
  if p = .whole then serve ledger m else ledger

/-- 在证明状态 `p` 下醒来一次之后的循环。 -/
def drainedUnder (p : Proof) (s : Loop) : Loop :=
  { inbox := []
    ledger := s.inbox.foldl (serveUnder p) s.ledger
    served := s.served ++ s.inbox
    deadlineDue := false }

theorem proved_is_the_ungated_loop (s : Loop) : drainedUnder .whole s = drained s := by
  have same : serveUnder .whole = serve := by
    funext ledger m
    simp [serveUnder]
  simp [drainedUnder, drained, same]

theorem batch_appends_nothing_unless_proved (p : Proof) (h : p ≠ .whole)
    (batch : List Message) (ledger : List Nat) : batch.foldl (serveUnder p) ledger = ledger := by
  induction batch generalizing ledger with
  | nil => rfl
  | cons m rest ih =>
    simp only [List.foldl_cons, serveUnder, ite_eq_right h]
    exact ih ledger

/-- 证明完成之前，一次醒来什么也不写。 -/
theorem nothing_is_written_before_the_proof (s : Loop) :
    (drainedUnder .pending s).ledger = s.ledger :=
  batch_appends_nothing_unless_proved .pending (by decide) s.inbox s.ledger

/-- 链断了之后同样什么也不写。 -/
theorem nothing_is_written_after_a_break (s : Loop) :
    (drainedUnder .broken s).ledger = s.ledger :=
  batch_appends_nothing_unless_proved .broken (by decide) s.inbox s.ledger

/-- 被拒的命令仍由这次醒来服务：人得到回答，队列不留下它。 -/
theorem a_refused_command_is_still_answered (p : Proof) (m : Message) (s : Loop) :
    m ∈ (drainedUnder p (arrive m s)).served ∧ (drainedUnder p (arrive m s)).inbox = [] := by
  simp [drainedUnder, arrive]

/-- 无论证明走到哪一步，追加次序仍是 seq 次序。 -/
theorem gated_batch_keeps_order (p : Proof) (batch : List Message) {ledger : List Nat}
    (h : Ordered ledger) : Ordered (batch.foldl (serveUnder p) ledger) := by
  induction batch generalizing ledger with
  | nil => exact h
  | cons m rest ih =>
    simp only [List.foldl_cons, serveUnder]
    split
    · exact ih (serve_keeps_order m h)
    · exact ih h

end Accounting.Worker.Attend

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-84 | `accounting::worker::attend` |
-/
