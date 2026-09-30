-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Workshop：只派就绪的节点，一个节点只派一次

规定 `crates/collab/src/workshop.rs`（`collab::workshop`，`Workshop::ready`）与 `crates/collab/src/workshop/underway.rs`（`collab::workshop::underway`，`Underway::hand_next`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一张图是一组节点，每个节点点名它依赖的节点。`done` 是这个房间的 join 已收下 `Artifact` 的节点；`handed` 是这个房间此前已派出的节点。一个节点在「就绪」与「已汇合」之间是「在飞」，只看 `done` 分不出它与「未派」，所以 `Underway` 带着已派集。

三条性质：

1. **只派就绪的节点**（`handed_are_ready`）：交出去的每个节点都还没汇合，它的依赖都已汇合。一个依赖未汇合的节点若也派出，它读到的是还不存在的产出。
2. **一个节点不派两次**（`handNext_fresh`、`handNext_again`）：交出去的节点不在已派集里；带着更新后的已派集再问一次，同一个 `done` 下什么也不交。所以图被再摆一次、或两个 handback 先后到达，都不会把在飞的节点再派一次。
3. **拿掉已派集，第二次会重派**（`withoutHanded_hands_twice`）：本模型咬得动的演示。

构造时拒重名、悬空依赖与环（`Workshop::new`），故一张存在的图是一张跑得完的图；调度确定（有序集合、按 id 破平），这是判负与重放的前提。这两条由 `crates/collab/src/workshop/tests.rs` 经生产入口断言，本模型不重述。
-/

namespace Collab.Workshop

structure Node where
  id : Nat
  deps : List Nat
  deriving DecidableEq, Repr

/-- 就绪集：没汇合、依赖都汇合了的节点。 -/
def ready (nodes : List Node) (done : List Nat) : List Nat :=
  (nodes.filter fun n => decide (n.id ∉ done) && n.deps.all fun d => decide (d ∈ done)).map Node.id

/-- `hand_next`：就绪集里这个房间还没派过的那些，派出并记为已派。 -/
def handNext (nodes : List Node) (handed done : List Nat) : List Nat × List Nat :=
  let fresh := (ready nodes done).filter fun x => decide (x ∉ handed)
  (fresh, handed ++ fresh)

theorem handed_are_ready {nodes : List Node} {handed done : List Nat} {x : Nat}
    (h : x ∈ (handNext nodes handed done).1) :
    ∃ n ∈ nodes, n.id = x ∧ x ∉ done ∧ ∀ d ∈ n.deps, d ∈ done := by
  simp only [handNext, ready, List.mem_filter, List.mem_map, Bool.and_eq_true,
    decide_eq_true_eq, List.all_eq_true] at h
  obtain ⟨⟨n, ⟨mem, notDone, deps⟩, rfl⟩, _⟩ := h
  exact ⟨n, mem, rfl, notDone, deps⟩

theorem handNext_fresh {nodes : List Node} {handed done : List Nat} {x : Nat}
    (h : x ∈ (handNext nodes handed done).1) : x ∉ handed := by
  simp only [handNext, List.mem_filter, decide_eq_true_eq] at h
  exact h.2

theorem handNext_again (nodes : List Node) (handed done : List Nat) :
    (handNext nodes (handNext nodes handed done).2 done).1 = [] := by
  simp only [handNext]
  rw [List.filter_eq_nil_iff]
  intro x hx
  by_cases hh : x ∈ handed
  · simp [hh]
  · simp [hh, hx]

/-!
## 咬得动的演示

把 `hand_next` 里「已派过的跳过」那条守卫拿掉，同一个 `done` 下问两次，同一个节点会被派两次。
-/

def handNextWithoutHanded (nodes : List Node) (done : List Nat) : List Nat :=
  ready nodes done

theorem withoutHanded_hands_twice :
    let nodes := [Node.mk 1 []]
    handNextWithoutHanded nodes [] ++ handNextWithoutHanded nodes [] = [1, 1] := by
  decide

end Collab.Workshop
