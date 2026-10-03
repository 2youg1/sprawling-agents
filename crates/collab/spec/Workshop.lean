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

性质 4–7 管一张图在父房间里的整段 trace：图在派活那一刻登记，子节点与父 run 谁先落地都行，父 run 被取消或失败时关图（见「图在派活那一刻登记」一节与 collab D14）。

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

/-!
## 图在派活那一刻登记（collab D7、D14）

上面三条只看一次 `hand_next`。下面的模型看父房间的一张图从摆出到关掉的整段 trace，规定 `crates/accounting/src/worker/waking/handing.rs`（调用时登记与派出，`end_hand_over` 按父 run 的结局关图）与 `crates/accounting/src/worker/dispatching/handback.rs`（`hand_down_what_is_ready`）。

- `layOut`：父 run 调 `workshop lay_out` 的那一刻。图就在这时登记给父房间（状态 `live`），就绪的节点当场派出，不等父 run 落地（D7）。图还开着时这个房间后来的 run 再摆一次，新图取代旧图，已派集照带；同一个 run 的第二次摆出在 Rust 里当场被拒（`WorkshopDesk::lay_out`），模型不分 run，所以不写这道拒。构造时重名的图被拒（`Workshop::new`），模型里是一个空操作。
- `land x back`：在飞的子节点 `x` 落地。它的 handback 投进父房间（`backs`）；`Finished` 才汇入 join（`done`），`Stopped` 只投信不汇合。图还开着，就按新的 `done` 再派一次就绪集；全部汇合就关图。子节点落地与父 run 落地的先后不受约束。
- `parentEnds e`：父 run 落地。`Done` 与 `Limit` 不改图；`Cancelled` 与 `Failed` 关图（`holds`，D14）：不再派新节点，在飞的节点照跑完、照回程。

图关掉之后，同一个房间后来的 run 再摆的图是另一张图，不在这段 trace 里；所以 `closed` 上的 `layOut` 是空操作。

性质，对从空房间出发的每一段 trace 成立：

4. **每个就绪节点恰好派一次**（`handed_once`、`ready_is_handed`）：已派序列不重复；图开着时，就绪集里的每个节点都已派出。
5. **子节点的 handback 恰好到达一次**（`backs_once`、`a_landing_hands_back`、`a_second_landing_adds_nothing`）：落地一次投一次，同一子节点再落地不再投；投到的都是派出过的节点。
6. **关图之后不再派**（`closed_hands_nothing_more`）。
7. **比父先落地的节点照样派出后继**（`an_early_child_hands_down`）：不论父 run 落没落地，一次 `Finished` 之后新就绪的节点都已派出。

`withoutCallRegistration_strands` 是被否决的那半步改法的反例：节点在调用时派出，图却照今天在父 run 落地时才登记，那么比父先落地的节点什么也派不下去，后继停在就绪而未派。

模型只读动作的次序：次序取自账本 seq 与注入的时钟，Windows、macOS、Linux 上同一段 trace 得同一结果。
-/

inductive Status where
  | idle
  | live
  | closed
  deriving DecidableEq, Repr

/-- 父 run 的落地结局：`kernel::Completion` 的三种，加上 drive 自己的失败。 -/
inductive Ending where
  | done
  | limit
  | cancelled
  | failed
  deriving DecidableEq, Repr

/-- 子节点的 handback：`Handback::Finished` 或 `Handback::Stopped`。 -/
inductive Back where
  | finished
  | stopped
  deriving DecidableEq, Repr

inductive Act where
  | layOut (nodes : List Node)
  | land (x : Nat) (back : Back)
  | parentEnds (ending : Ending)
  deriving DecidableEq, Repr

/-- 父房间里的一张图：节点、状态、已派序列（派活记录，按派出次序）、已汇合、已投到的 handback。 -/
structure Room where
  nodes : List Node
  status : Status
  handed : List Nat
  done : List Nat
  backs : List Nat
  deriving DecidableEq, Repr

def Room.empty : Room := ⟨[], .idle, [], [], []⟩

def joined (nodes : List Node) (done : List Nat) : Bool :=
  nodes.all fun n => decide (n.id ∈ done)

/-- D14：哪些结局关图。 -/
def holds : Ending → Bool
  | .done => false
  | .limit => false
  | .cancelled => true
  | .failed => true

/-- 按此刻的 `done` 派出新就绪的节点。 -/
def Room.handDown (r : Room) : Room :=
  { r with handed := (handNext r.nodes r.handed r.done).2 }

/-- 派出新就绪的节点；全部汇合就关图。 -/
def Room.settle (r : Room) : Room :=
  if joined r.nodes r.done then { r.handDown with status := .closed } else r.handDown

def Room.receive (r : Room) (x : Nat) : Back → Room
  | .finished => { r with backs := r.backs ++ [x], done := r.done ++ [x] }
  | .stopped => { r with backs := r.backs ++ [x] }

def Room.layOut (r : Room) (nodes : List Node) : Room :=
  match r.status with
  | .closed => r
  | .idle | .live =>
    if (nodes.map Node.id).Nodup then Room.settle { r with nodes := nodes, status := .live } else r

def Room.land (r : Room) (x : Nat) (back : Back) : Room :=
  if x ∈ r.handed ∧ x ∉ r.backs then
    match r.status with
    | .live => (r.receive x back).settle
    | .idle | .closed => r.receive x back
  else r

def Room.parentEnds (r : Room) (e : Ending) : Room :=
  match r.status with
  | .live => if holds e then { r with status := .closed } else r
  | .idle | .closed => r

def step (r : Room) : Act → Room
  | .layOut nodes => r.layOut nodes
  | .land x back => r.land x back
  | .parentEnds e => r.parentEnds e

def run (r : Room) (acts : List Act) : Room := acts.foldl step r

/-- 每一段 trace 都守住的不变式。 -/
structure Inv (r : Room) : Prop where
  ids : (r.nodes.map Node.id).Nodup
  handed : r.handed.Nodup
  backs : r.backs.Nodup
  backs_handed : ∀ x ∈ r.backs, x ∈ r.handed
  ready_handed : r.status = .live → ∀ x ∈ ready r.nodes r.done, x ∈ r.handed

theorem ready_nodup {nodes : List Node} (h : (nodes.map Node.id).Nodup) (done : List Nat) :
    (ready nodes done).Nodup := by
  unfold ready
  exact h.sublist ((List.filter_sublist).map _)

theorem handNext_nodup {nodes : List Node} {handed : List Nat} (done : List Nat)
    (ids : (nodes.map Node.id).Nodup) (h : handed.Nodup) :
    (handNext nodes handed done).2.Nodup := by
  simp only [handNext]
  rw [List.nodup_append]
  refine ⟨h, (ready_nodup ids done).filter _, ?_⟩
  intro a ha b hb e
  subst e
  simp only [List.mem_filter, decide_eq_true_eq] at hb
  exact hb.2 ha

theorem handNext_covers (nodes : List Node) (handed done : List Nat) :
    ∀ x ∈ ready nodes done, x ∈ (handNext nodes handed done).2 := by
  intro x hx
  simp only [handNext, List.mem_append, List.mem_filter, decide_eq_true_eq]
  by_cases hh : x ∈ handed
  · exact Or.inl hh
  · exact Or.inr ⟨hx, hh⟩

theorem handNext_keeps (nodes : List Node) (handed done : List Nat) :
    ∀ x ∈ handed, x ∈ (handNext nodes handed done).2 := by
  intro x hx
  simp only [handNext, List.mem_append]
  exact Or.inl hx

theorem settle_fields (r : Room) :
    r.settle.nodes = r.nodes ∧ r.settle.done = r.done ∧ r.settle.backs = r.backs ∧
      r.settle.handed = (handNext r.nodes r.handed r.done).2 := by
  unfold Room.settle
  split <;> exact ⟨rfl, rfl, rfl, rfl⟩

theorem settle_inv {r : Room} (ids : (r.nodes.map Node.id).Nodup) (h : r.handed.Nodup)
    (b : r.backs.Nodup) (bh : ∀ x ∈ r.backs, x ∈ r.handed) : Inv r.settle := by
  obtain ⟨sn, sd, sb, sh⟩ := settle_fields r
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · rw [sn]; exact ids
  · rw [sh]; exact handNext_nodup r.done ids h
  · rw [sb]; exact b
  · intro x hx
    rw [sb] at hx
    rw [sh]
    exact handNext_keeps _ _ _ x (bh x hx)
  · intro _ x hx
    rw [sn, sd] at hx
    rw [sh]
    exact handNext_covers _ _ _ x hx

theorem receive_fields (r : Room) (x : Nat) (back : Back) :
    (r.receive x back).nodes = r.nodes ∧ (r.receive x back).handed = r.handed ∧
      (r.receive x back).status = r.status ∧ (r.receive x back).backs = r.backs ++ [x] := by
  cases back <;> exact ⟨rfl, rfl, rfl, rfl⟩

theorem receive_inv {r : Room} {x : Nat} (h : Inv r) (fresh : x ∈ r.handed ∧ x ∉ r.backs)
    (back : Back) :
    ((r.receive x back).nodes.map Node.id).Nodup ∧ (r.receive x back).handed.Nodup ∧
      (r.receive x back).backs.Nodup ∧
      ∀ y ∈ (r.receive x back).backs, y ∈ (r.receive x back).handed := by
  obtain ⟨hn, hh, _, hb⟩ := receive_fields r x back
  refine ⟨by rw [hn]; exact h.ids, by rw [hh]; exact h.handed, ?_, ?_⟩
  · rw [hb, List.nodup_append]
    refine ⟨h.backs, List.pairwise_singleton _ x, ?_⟩
    intro a ha c hc e
    simp only [List.mem_singleton] at hc
    subst hc
    subst e
    exact fresh.2 ha
  · intro y hy
    rw [hb, List.mem_append, List.mem_singleton] at hy
    rw [hh]
    rcases hy with hy | hy
    · exact h.backs_handed y hy
    · subst hy
      exact fresh.1

theorem layOut_inv {r : Room} (h : Inv r) (nodes : List Node) : Inv (r.layOut nodes) := by
  unfold Room.layOut
  split
  · exact h
  all_goals
    split
    · next ids => exact settle_inv ids h.handed h.backs h.backs_handed
    · exact h

theorem land_inv {r : Room} (h : Inv r) (x : Nat) (back : Back) : Inv (r.land x back) := by
  unfold Room.land
  split
  · next fresh =>
    obtain ⟨ids', hd', b', bh'⟩ := receive_inv h fresh back
    have hs := (receive_fields r x back).2.2.1
    split
    · exact settle_inv ids' hd' b' bh'
    all_goals
      next hst => exact ⟨ids', hd', b', bh', fun e => by rw [hs, hst] at e; cases e⟩
  · exact h

theorem parentEnds_inv {r : Room} (h : Inv r) (e : Ending) : Inv (r.parentEnds e) := by
  unfold Room.parentEnds
  split
  · split
    · exact ⟨h.ids, h.handed, h.backs, h.backs_handed, fun e => nomatch e⟩
    · exact h
  all_goals exact h

theorem step_inv {r : Room} (h : Inv r) : ∀ a, Inv (step r a)
  | .layOut nodes => layOut_inv h nodes
  | .land x back => land_inv h x back
  | .parentEnds e => parentEnds_inv h e

theorem empty_inv : Inv Room.empty where
  ids := List.nodup_nil
  handed := List.nodup_nil
  backs := List.nodup_nil
  backs_handed := fun _ h => nomatch h
  ready_handed := fun h => nomatch h

theorem run_inv {r : Room} (h : Inv r) : ∀ acts, Inv (run r acts) := by
  intro acts
  induction acts generalizing r with
  | nil => exact h
  | cons a rest ih => exact ih (step_inv h a)

theorem reachable_inv (acts : List Act) : Inv (run Room.empty acts) :=
  run_inv empty_inv acts

theorem handed_once (acts : List Act) : (run Room.empty acts).handed.Nodup :=
  (reachable_inv acts).handed

theorem ready_is_handed (acts : List Act) (h : (run Room.empty acts).status = .live) :
    ∀ x ∈ ready (run Room.empty acts).nodes (run Room.empty acts).done,
      x ∈ (run Room.empty acts).handed :=
  (reachable_inv acts).ready_handed h

theorem backs_once (acts : List Act) :
    (run Room.empty acts).backs.Nodup ∧
      ∀ x ∈ (run Room.empty acts).backs, x ∈ (run Room.empty acts).handed :=
  ⟨(reachable_inv acts).backs, (reachable_inv acts).backs_handed⟩

theorem a_landing_hands_back {r : Room} {x : Nat} (back : Back)
    (handed : x ∈ r.handed) (fresh : x ∉ r.backs) :
    (step r (.land x back)).backs = r.backs ++ [x] := by
  show (r.land x back).backs = _
  unfold Room.land
  rw [if_pos ⟨handed, fresh⟩]
  split
  · rw [(settle_fields _).2.2.1]; exact (receive_fields r x back).2.2.2
  all_goals exact (receive_fields r x back).2.2.2

theorem a_second_landing_adds_nothing {r : Room} {x : Nat} (back : Back) (h : x ∈ r.backs) :
    step r (.land x back) = r := by
  show r.land x back = r
  unfold Room.land
  rw [if_neg (fun c => c.2 h)]

theorem closed_step {r : Room} (h : r.status = .closed) :
    ∀ a, (step r a).status = .closed ∧ (step r a).handed = r.handed
  | .layOut nodes => by
    show (r.layOut nodes).status = _ ∧ (r.layOut nodes).handed = _
    unfold Room.layOut
    rw [h]
    exact ⟨h, rfl⟩
  | .land x back => by
    show (r.land x back).status = _ ∧ (r.land x back).handed = _
    unfold Room.land
    rw [h]
    split
    · exact ⟨by rw [(receive_fields r x back).2.2.1, h], (receive_fields r x back).2.1⟩
    · exact ⟨h, rfl⟩
  | .parentEnds e => by
    show (r.parentEnds e).status = _ ∧ (r.parentEnds e).handed = _
    unfold Room.parentEnds
    rw [h]
    exact ⟨h, rfl⟩

theorem closed_hands_nothing_more {r : Room} (h : r.status = .closed) :
    ∀ more, (run r more).status = .closed ∧ (run r more).handed = r.handed := by
  intro more
  induction more generalizing r with
  | nil => exact ⟨h, rfl⟩
  | cons a rest ih =>
    obtain ⟨hs, hh⟩ := closed_step h a
    obtain ⟨hs', hh'⟩ := ih hs
    exact ⟨hs', hh'.trans hh⟩

theorem joined_ready_empty {nodes : List Node} {done : List Nat} (h : joined nodes done = true) :
    ready nodes done = [] := by
  unfold joined at h
  unfold ready
  rw [List.all_eq_true] at h
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro n hn
  have := h n hn
  simp only [decide_eq_true_eq] at this
  simp [this]

theorem an_early_child_hands_down {r : Room} (h : Inv r) {x : Nat} (opened : r.status = .live)
    (out : x ∈ r.handed) (fresh : x ∉ r.backs) :
    ∀ y ∈ ready (step r (.land x .finished)).nodes (step r (.land x .finished)).done,
      y ∈ (step r (.land x .finished)).handed := by
  have settled : step r (.land x .finished) = (r.receive x .finished).settle := by
    show r.land x .finished = _
    unfold Room.land
    rw [if_pos ⟨out, fresh⟩, opened]
  rw [settled]
  intro y hy
  obtain ⟨ids', hd', b', bh'⟩ := receive_inv h ⟨out, fresh⟩ .finished
  obtain ⟨sn, sd, _⟩ := settle_fields (r.receive x .finished)
  cases j : joined (r.receive x .finished).nodes (r.receive x .finished).done with
  | false =>
    have live : (r.receive x .finished).settle.status = .live := by
      unfold Room.settle
      simp only [j, Bool.false_eq_true, if_false]
      exact (receive_fields r x .finished).2.2.1.trans opened
    exact (settle_inv ids' hd' b' bh').ready_handed live y hy
  | true =>
    rw [sn, sd, joined_ready_empty j] at hy
    exact nomatch hy

/-- 从空房间出发的任一段 trace 上，图开着时落地的一个在飞节点照样派出后继，不问父 run 落没落地。 -/
theorem a_child_landing_first_hands_down (acts : List Act) {x : Nat}
    (opened : (run Room.empty acts).status = .live)
    (out : x ∈ (run Room.empty acts).handed) (fresh : x ∉ (run Room.empty acts).backs) :
    let r := step (run Room.empty acts) (.land x .finished)
    ∀ y ∈ ready r.nodes r.done, y ∈ r.handed :=
  an_early_child_hands_down (reachable_inv acts) opened out fresh

/-!
## 被否决的半步：调用时派出，落地时才登记

危险在于：节点在调用时经 `DelegateDesk` 派出，`hand_down_what_is_ready` 读的却是父 run 落地时才插进 `collaborating.workshops` 的图。下面把登记推迟到父 run 以 `Done` 或 `Limit` 落地：比父先落地的节点 1 什么也派不下去，父落地后节点 2 就绪而未派，再也没有一次 handback 叫它。
-/

def stepRegisteringAtLanding (r : Room) : Act → Room
  | .layOut nodes => { r with nodes := nodes, handed := (handNext nodes r.handed r.done).2 }
  | .land x back => r.land x back
  | .parentEnds e =>
    match r.status with
    | .idle => if holds e then r else { r with status := .live }
    | .live | .closed => r

theorem withoutCallRegistration_strands :
    let r := [Act.layOut [⟨1, []⟩, ⟨2, [1]⟩], .land 1 .finished, .parentEnds .done].foldl
      stepRegisteringAtLanding Room.empty
    r.status = .live ∧ ready r.nodes r.done = [2] ∧ r.handed = [1] := by
  decide

/-!
## 导出检查：轨迹向量与两个故意改坏的实现

Rust 测试经生产入口（`RunWorker::hand_over_at_call`、子 run 的落地与 handback、`end_hand_over`）重放下面的动作序列，断言派活记录（`handed`，按 `run_started` 次序）、汇合集与关图与否等于这里的值：`crates/accounting/src/worker/waking/vector_tests.rs` 重放全部四条向量。`vectorFailed` 的父 run 是一次失败的驱动，经 `GraphAfter::of` 读成 `GraphAfter::Closed`，与 `vectorCancelled` 同一臂；节点 3 的模型调用被扣到父 run 结束之后，所以落地次序是向量的次序，不是 lane 谁快。`vectorStopped` 的节点 2 以空回复落成 `Limit`，即 `Handback::Stopped`；它的末一步（节点 2 第二次落地）在生产里没有入口，一个 run 只落地一次，所以 Rust 重放前五步。节点 id 是子房间地址的序号；`Back.finished` 是子 run 以 Done 落地并通过 done check，`Back.stopped` 是它停下；`Ending` 是父 run 的落地结局。每条向量由 `lake build` 经 `#guard` 判定，模型改了而向量没跟上，构建就红。

两个故意改坏的实现必须各让至少一条向量变红，在模型里由下面的 `#guard` 判，在 Rust 里由同名测试判：Rust 测试服务到汇合数够了或已无 run 在驱动为止，所以一个什么也不派的实现让断言变红，而不是让测试挂住。

- **落地才登记**（`stepRegisteringAtLanding`，即被否决的半步）：`vectorEarly`、`vectorCancelled`、`vectorFailed` 都变红，三条下都只派出节点 1，节点 2 与 3 不派（紧跟四条向量之后的那条 `#guard` 判这件事）。
- **关图不挡**（`stepIgnoringEnding`）：`hand_down_what_is_ready` 不看父 run 的结局（`holds`），`vectorCancelled` 与 `vectorFailed` 变红，节点 4 被派出。

`vectorStopped` 记下 `crates/collab/Spec.lean` §3 那个未定的问题：停下的子节点投了 handback 却不汇合，图永远开着、节点 4 永远不派；要改这条，先改模型。
-/

/-- 菱形：1 在前，2、3 依赖 1，4 依赖 2 与 3。 -/
def diamond : List Node := [⟨1, []⟩, ⟨2, [1]⟩, ⟨3, [1]⟩, ⟨4, [2, 3]⟩]

def vectorEarly : List Act :=
  [.layOut diamond, .land 1 .finished, .land 2 .finished, .land 3 .finished, .parentEnds .done,
    .land 4 .finished]

def vectorCancelled : List Act :=
  [.layOut diamond, .land 1 .finished, .parentEnds .cancelled, .land 2 .finished,
    .land 3 .finished]

def vectorFailed : List Act :=
  [.layOut diamond, .land 1 .finished, .land 2 .finished, .parentEnds .failed, .land 3 .finished]

def vectorStopped : List Act :=
  [.layOut diamond, .parentEnds .limit, .land 1 .finished, .land 2 .stopped, .land 3 .finished,
    .land 2 .finished]

#guard run Room.empty vectorEarly ==
  ⟨diamond, .closed, [1, 2, 3, 4], [1, 2, 3, 4], [1, 2, 3, 4]⟩
#guard run Room.empty vectorCancelled ==
  ⟨diamond, .closed, [1, 2, 3], [1, 2, 3], [1, 2, 3]⟩
#guard run Room.empty vectorFailed ==
  ⟨diamond, .closed, [1, 2, 3], [1, 2, 3], [1, 2, 3]⟩
#guard run Room.empty vectorStopped ==
  ⟨diamond, .live, [1, 2, 3], [1, 3], [1, 2, 3]⟩
#guard [vectorEarly, vectorCancelled, vectorFailed].all fun v =>
  (v.foldl stepRegisteringAtLanding Room.empty).handed == [1]

/-- 第二个改坏的实现：父 run 的结局不关图。 -/
def stepIgnoringEnding (r : Room) : Act → Room
  | .parentEnds _ => r
  | a => step r a

#guard [vectorCancelled, vectorFailed].all fun v =>
  (v.foldl stepIgnoringEnding Room.empty).handed == [1, 2, 3, 4]

end Collab.Workshop
