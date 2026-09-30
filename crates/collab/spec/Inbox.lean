-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Inbox：至少一次投递，副作用只发生一次

规定 `crates/collab/src/inbox.rs`（`collab::inbox`）与 `crates/collab/src/steer.rs`（`collab::steer`）交给 Inbox 的那一半。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个房间的 Inbox 是两条队列：急件 lane 与普通 lane。一件 Signal 由 `id` 与 `kind` 构成；lane 由 kind 推出，调用方给不了。去重由 `storage::EventQueue` 的 `seen` 给（IdemKey 由 `SignalId` 派生），本模块不另建第二张表，这里把两条队列的 `seen` 合写成一个集合：同一个 id 恒落同一条 lane，所以两种写法说的是同一件事。

四条性质：

1. **重复投递不产生第二件**（`deliver_twice`）：同一件 Signal 投两次，与投一次得到同一个 Inbox。
2. **lane 由 kind 推出**（`lane`）：急件 lane 恰是 `Steer`，于是「取一件能插队的东西」不需要再按 kind 筛一次（`takeSteer_is_steer`）。
3. **急件先出，一次最多 bandwidth 件**（`pull_bounded`、`pull_is_prefix`）：接收方的上下文窗口由接收方定上限，发送方推不动它。
4. **拿掉去重，第二件就会出现**（`withoutSeen_duplicates`）：这是本模型咬得动的演示，见文末。

`SignalKind` 与两条线上载荷（`SignalEnqueued`、`SignalConsumed`）的形状住 kernel（kernel-SPEC §8-4）；`lane` 仍写进载荷给 crate 外的读者，回读时不采信，由 `kind` 再推一次。
-/

namespace Collab.Inbox

/-- 四种 Signal。紧急与否是 Signal 自己的属性：同一件从两个调用点进来，落的是同一条 lane。 -/
inductive SignalKind where
  | mention
  | thread
  | broadcast
  | steer
  deriving DecidableEq, Repr

/-- 插队首的一条 lane 与普通的一条。插队就是「先被排干的那条 lane」，不是队内的优先级字段：一个结构里共存两种顺序，就会有人读错其中一种。 -/
inductive Lane where
  | urgent
  | ordinary
  deriving DecidableEq, Repr

/-- lane 由 kind 推出。 -/
def lane : SignalKind → Lane
  | .steer => .urgent
  | .mention => .ordinary
  | .thread => .ordinary
  | .broadcast => .ordinary

structure Signal where
  id : Nat
  kind : SignalKind
  deriving DecidableEq, Repr

structure Inbox where
  urgent : List Signal
  ordinary : List Signal
  seen : List Nat
  deriving Repr

def Inbox.empty : Inbox := ⟨[], [], []⟩

/-- 投递：见过的 id 原样退回，没见过的按 lane 入队并记下。 -/
def Inbox.deliver (q : Inbox) (s : Signal) : Inbox :=
  if s.id ∈ q.seen then q
  else match lane s.kind with
    | .urgent => { q with urgent := q.urgent ++ [s], seen := s.id :: q.seen }
    | .ordinary => { q with ordinary := q.ordinary ++ [s], seen := s.id :: q.seen }

theorem deliver_twice (q : Inbox) (s : Signal) :
    (q.deliver s).deliver s = q.deliver s := by
  unfold Inbox.deliver
  by_cases seen : s.id ∈ q.seen
  · simp [seen]
  · cases lane s.kind <;> simp [seen]

/-- 急件 lane 只装 `steer`，普通 lane 不装 `steer`。 -/
def Inbox.Sorted (q : Inbox) : Prop :=
  (∀ s ∈ q.urgent, s.kind = .steer) ∧ (∀ s ∈ q.ordinary, s.kind ≠ .steer)

theorem lane_urgent_iff (k : SignalKind) : lane k = .urgent ↔ k = .steer := by
  cases k <;> simp [lane]

theorem empty_sorted : Inbox.empty.Sorted := by
  simp [Inbox.Sorted, Inbox.empty]

theorem deliver_sorted (q : Inbox) (s : Signal) (h : q.Sorted) : (q.deliver s).Sorted := by
  obtain ⟨u, o⟩ := h
  unfold Inbox.deliver
  by_cases seen : s.id ∈ q.seen
  · simp only [seen, if_true]; exact ⟨u, o⟩
  · simp only [seen, if_false]
    cases hl : lane s.kind with
    | urgent =>
      have steer := (lane_urgent_iff s.kind).mp hl
      refine ⟨?_, o⟩
      intro t ht
      simp only [List.mem_append, List.mem_singleton] at ht
      rcases ht with ht | ht
      · exact u t ht
      · rw [ht]; exact steer
    | ordinary =>
      have not_steer : s.kind ≠ .steer := by
        intro e; rw [(lane_urgent_iff s.kind).mpr e] at hl; cases hl
      refine ⟨u, ?_⟩
      intro t ht
      simp only [List.mem_append, List.mem_singleton] at ht
      rcases ht with ht | ht
      · exact o t ht
      · rw [ht]; exact not_steer

/-- `take_steer`：只从急件 lane 取一件。它不碰普通 lane，因为插队与拆信是两件事。 -/
def Inbox.takeSteer (q : Inbox) : Option Signal × Inbox :=
  match q.urgent with
  | [] => (none, q)
  | s :: rest => (some s, { q with urgent := rest })

theorem takeSteer_is_steer (q : Inbox) (h : q.Sorted) (s : Signal) (q' : Inbox)
    (taken : q.takeSteer = (some s, q')) : s.kind = .steer := by
  unfold Inbox.takeSteer at taken
  cases hu : q.urgent with
  | nil => rw [hu] at taken; simp at taken
  | cons t rest =>
    rw [hu] at taken
    simp only [Prod.mk.injEq, Option.some.injEq] at taken
    rw [← taken.1]
    exact h.1 t (by rw [hu]; exact List.mem_cons_self)

/-- `pull`：急件先出，一次最多 `bandwidth` 件。 -/
def Inbox.pull (q : Inbox) (bandwidth : Nat) : List Signal :=
  (q.urgent ++ q.ordinary).take bandwidth

theorem pull_bounded (q : Inbox) (bandwidth : Nat) : (q.pull bandwidth).length ≤ bandwidth := by
  simp only [Inbox.pull, List.length_take]
  exact Nat.min_le_left _ _

theorem pull_is_prefix (q : Inbox) (bandwidth : Nat) :
    q.pull bandwidth <+: q.urgent ++ q.ordinary :=
  List.take_prefix bandwidth _

/-!
## 咬得动的演示

把 `deliver` 里「见过的 id 原样退回」那条守卫拿掉，`deliver_twice` 不再成立：同一件 Signal 投两次，队里有两件，副作用会发生两次。下面的定理就是那条反例，`lake build` 证明它。
-/

def Inbox.deliverWithoutSeen (q : Inbox) (s : Signal) : Inbox :=
  match lane s.kind with
  | .urgent => { q with urgent := q.urgent ++ [s] }
  | .ordinary => { q with ordinary := q.ordinary ++ [s] }

theorem withoutSeen_duplicates :
    let s : Signal := ⟨7, .mention⟩
    ((Inbox.empty.deliverWithoutSeen s).deliverWithoutSeen s).ordinary.length = 2 := by
  decide

end Collab.Inbox
