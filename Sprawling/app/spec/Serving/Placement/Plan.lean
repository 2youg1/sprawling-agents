-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 放置计划：从读到的处理器拓扑到热线程的座位

规定 `crates/sprawling/src/serving/placement/plan.rs`（`bin::serving::placement::plan`，形状：决定）。放置计划是一个纯函数：输入是平台读到的拓扑描述，输出是热线程可以坐的逻辑处理器，按处理器号排好；不调平台、不读钟、不看环境。`crates/sprawling/src/serving/placement/plan/tests.rs` 用 proptest 在生成的拓扑上检查下面每一条性质，并用按厂商公开核数写成的处理器家族表逐台检查计划的结果。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

拓扑里每个逻辑处理器带五项：平台的处理器号 `id`；效率档 `cls`，数越大越快（Windows 的 `EfficiencyClass` 就是这个方向，Linux 由 `cpu_capacity` 或 Intel 的 `cpu_core`／`cpu_atom` 换算过来）；物理核 `core`；末级缓存组 `cache`；本进程能不能用它 `allowed`（进程的亲和、Job Object、容器的 cpuset、虚拟机给的处理器）。档的个数不限：两档（P 与 E）、三档（Meteor Lake 的 P、E 与 SoC 上的低功耗 E；ARM 的 prime、big、little）都是同一个模型。

计划的规则（D45 起各条决定写在 `crates/sprawling/spec/Serving/Placement.lean`）：

* 拓扑不自洽（处理器号重复，或同一个物理核报出两个档）就不放置，交给操作系统；
* 本进程能用的处理器只有一档就不放置——不论末级缓存分几组（双 CCD 的 X3D 只有一档、两种 L3，由 AMD 自己的驱动去分，第二个意见只会与它打架）；
* 有几档时，计划只含最快一档，每个物理核取本进程能用的、号最小的那个逻辑处理器；最慢一档（低功耗 E 核）不进计划；其余交给操作系统。

性质，对模型容许的每一张拓扑都成立：

* **只用能用的**——计划里的每个处理器都在拓扑里，且本进程能用（`a_planned_cpu_is_one_the_process_may_use`）；
* **一档不放置**——能用的处理器只有一档，不论缓存，计划是空的（`one_class_gives_no_plan`，缓存不进计划见 `the_plan_never_reads_the_cache`）；
* **不自洽不放置**（`an_inconsistent_topology_gives_no_plan`）；
* **一个物理核至多一个座位**——计划里两项同核即同一项，所以座位数不超过最快一档的物理核数（`one_seat_per_physical_core`）；
* **最慢一档不被偏好**——有几档时，计划里的每一项都有一个能用的处理器比它慢（`a_planned_cpu_is_never_of_the_slowest_class`）；
* **确定**——平台以任意顺序报同一张拓扑，计划含的处理器相同（`the_plan_ignores_the_order_it_is_read_in`）；Rust 再按处理器号排序，于是输出逐项相同。
-/

namespace Sprawling.Serving.Placement.Plan

/-- 一个逻辑处理器，按平台读到的样子。 -/
structure Cpu where
  id : Nat
  cls : Nat
  core : Nat
  cache : Nat
  allowed : Bool
  deriving DecidableEq, Repr

/-- 平台读到的拓扑，顺序随平台。 -/
abbrev Topology := List Cpu

/-- 本进程能用的处理器。 -/
def usable (t : Topology) : List Cpu := t.filter (·.allowed)

/-- 自洽：处理器号不重复，同一个物理核只有一档。 -/
def consistent (t : Topology) : Bool :=
  decide (t.map (·.id)).Nodup && t.all (fun a => t.all (fun b => !(a.core == b.core) || a.cls == b.cls))

/-- 能用的处理器不止一档。 -/
def several (t : Topology) : Bool :=
  (usable t).any (fun a => (usable t).any (fun b => a.cls != b.cls))

/-- `c` 在能用的处理器里属最快一档。 -/
def fastest (t : Topology) (c : Cpu) : Bool := (usable t).all (fun b => b.cls ≤ c.cls)

/-- `c` 是它那个物理核上能用的、号最小的逻辑处理器。 -/
def leads (t : Topology) (c : Cpu) : Bool :=
  (usable t).all (fun b => !(b.core == c.core) || c.id ≤ b.id)

/-- 放置计划：热线程可以坐的处理器。 -/
def plan (t : Topology) : List Cpu :=
  if consistent t && several t then (usable t).filter (fun c => fastest t c && leads t c) else []

theorem mem_usable {t : Topology} {c : Cpu} : c ∈ usable t ↔ c ∈ t ∧ c.allowed = true := by
  simp [usable]

theorem mem_plan {t : Topology} {c : Cpu} (h : c ∈ plan t) :
    consistent t = true ∧ several t = true ∧ c ∈ usable t ∧ fastest t c = true ∧ leads t c = true := by
  unfold plan at h
  split at h
  · rename_i hc
    simp only [Bool.and_eq_true] at hc
    simp only [List.mem_filter, Bool.and_eq_true] at h
    exact ⟨hc.1, hc.2, h.1, h.2.1, h.2.2⟩
  · simp at h

theorem a_planned_cpu_is_one_the_process_may_use (t : Topology) (c : Cpu) (h : c ∈ plan t) :
    c ∈ t ∧ c.allowed = true :=
  mem_usable.mp (mem_plan h).2.2.1

theorem one_class_gives_no_plan (t : Topology)
    (h : ∀ a ∈ usable t, ∀ b ∈ usable t, a.cls = b.cls) : plan t = [] := by
  have hs : several t = false := by
    simp only [several, Bool.eq_false_iff, ne_eq, List.any_eq_true, bne_iff_ne, not_exists, not_and]
    intro a ha b hb hne
    exact hne (h a ha b hb)
  simp [plan, hs]

theorem an_inconsistent_topology_gives_no_plan (t : Topology) (h : consistent t = false) :
    plan t = [] := by
  simp [plan, h]

/-- 缓存不进计划：把每个处理器的缓存组改成任何值，计划含的处理器号不变。 -/
theorem the_plan_never_reads_the_cache (t : Topology) (f : Cpu → Nat) :
    (plan (t.map (fun c => { c with cache := f c }))).map (·.id) = (plan t).map (·.id) := by
  simp [plan, consistent, several, fastest, leads, usable, List.filter_map, Function.comp_def]
  split <;> simp [Function.comp_def]

theorem eq_of_id_eq {l : List Cpu} (h : (l.map (·.id)).Nodup) {a b : Cpu} (ha : a ∈ l) (hb : b ∈ l)
    (hid : a.id = b.id) : a = b := by
  induction l with
  | nil => simp at ha
  | cons x rest ih =>
    rw [List.map_cons, List.nodup_cons] at h
    simp only [List.mem_cons] at ha hb
    rcases ha with rfl | ha <;> rcases hb with rfl | hb
    · rfl
    · exact (h.1 (List.mem_map.mpr ⟨b, hb, hid.symm⟩)).elim
    · exact (h.1 (List.mem_map.mpr ⟨a, ha, hid⟩)).elim
    · exact ih h.2 ha hb

theorem one_seat_per_physical_core (t : Topology) (a b : Cpu) (ha : a ∈ plan t) (hb : b ∈ plan t)
    (hcore : a.core = b.core) : a = b := by
  obtain ⟨hcons, -, hua, -, hla⟩ := mem_plan ha
  obtain ⟨-, -, hub, -, hlb⟩ := mem_plan hb
  simp only [leads, List.all_eq_true, Bool.or_eq_true, Bool.not_eq_true', beq_eq_false_iff_ne,
    ne_eq, decide_eq_true_eq] at hla hlb
  have hab : a.id ≤ b.id := by
    rcases hla b hub with h | h
    · exact absurd hcore.symm h
    · exact h
  have hba : b.id ≤ a.id := by
    rcases hlb a hua with h | h
    · exact absurd hcore h
    · exact h
  have hid : a.id = b.id := Nat.le_antisymm hab hba
  simp only [consistent, Bool.and_eq_true, decide_eq_true_eq] at hcons
  have hnodup := hcons.1
  exact eq_of_id_eq hnodup (mem_usable.mp hua).1 (mem_usable.mp hub).1 hid

theorem a_planned_cpu_is_never_of_the_slowest_class (t : Topology) (c : Cpu) (h : c ∈ plan t) :
    ∃ b ∈ usable t, b.cls < c.cls := by
  obtain ⟨-, hsev, -, hfast, -⟩ := mem_plan h
  simp only [several, List.any_eq_true, bne_iff_ne, ne_eq] at hsev
  obtain ⟨a, ha, b, hb, hne⟩ := hsev
  simp only [fastest, List.all_eq_true, decide_eq_true_eq] at hfast
  have hac := hfast a ha
  have hbc := hfast b hb
  by_cases hca : a.cls = c.cls
  · exact ⟨b, hb, by omega⟩
  · exact ⟨a, ha, by omega⟩

theorem perm_all {l l' : List Cpu} (hp : l.Perm l') (p : Cpu → Bool) : l.all p = l'.all p := by
  rw [Bool.eq_iff_iff]
  simp only [List.all_eq_true]
  exact ⟨fun h x hx => h x (hp.mem_iff.mpr hx), fun h x hx => h x (hp.mem_iff.mp hx)⟩

theorem perm_any {l l' : List Cpu} (hp : l.Perm l') (p : Cpu → Bool) : l.any p = l'.any p := by
  rw [Bool.eq_iff_iff]
  simp only [List.any_eq_true]
  exact ⟨fun ⟨x, hx, h⟩ => ⟨x, hp.mem_iff.mp hx, h⟩, fun ⟨x, hx, h⟩ => ⟨x, hp.mem_iff.mpr hx, h⟩⟩

theorem the_plan_ignores_the_order_it_is_read_in (t t' : Topology) (hp : t.Perm t') (c : Cpu) :
    c ∈ plan t ↔ c ∈ plan t' := by
  have hu : (usable t).Perm (usable t') := hp.filter _
  have hall : t.all (fun a => t.all (fun b => !(a.core == b.core) || a.cls == b.cls)) =
      t'.all (fun a => t'.all (fun b => !(a.core == b.core) || a.cls == b.cls)) := by
    rw [perm_all hp]
    congr 1
    funext a
    exact perm_all hp _
  have hcons : consistent t = consistent t' := by
    unfold consistent
    rw [hall]
    congr 1
    exact decide_eq_decide.mpr (hp.map (·.id)).nodup_iff
  have hsev : several t = several t' := by
    simp only [several]
    rw [perm_any hu]
    congr 1
    funext a
    exact perm_any hu _
  have hfast : ∀ x, fastest t x = fastest t' x := fun x => perm_all hu _
  have hleads : ∀ x, leads t x = leads t' x := fun x => perm_all hu _
  simp only [plan, hcons, hsev, hfast, hleads]
  split
  · simp only [List.mem_filter]
    exact ⟨fun ⟨hx, h⟩ => ⟨hu.mem_iff.mp hx, h⟩, fun ⟨hx, h⟩ => ⟨hu.mem_iff.mpr hx, h⟩⟩
  · exact Iff.rfl

/-- i5-1340P 的形状缩小一半：两个 P 核（各带超线程兄弟）、四个 E 核，计划是两个 P 核各自的第一个逻辑处理器。 -/
example :
    (plan [⟨0, 1, 0, 0, true⟩, ⟨1, 1, 0, 0, true⟩, ⟨2, 1, 1, 0, true⟩, ⟨3, 1, 1, 0, true⟩,
      ⟨4, 0, 2, 0, true⟩, ⟨5, 0, 3, 0, true⟩, ⟨6, 0, 4, 0, true⟩, ⟨7, 0, 5, 0, true⟩]).map (·.id)
      = [0, 2] := by decide

/-- 双 CCD 的 X3D 缩小：一档、两组缓存，计划是空的。 -/
example : plan [⟨0, 0, 0, 0, true⟩, ⟨1, 0, 1, 0, true⟩, ⟨2, 0, 2, 1, true⟩, ⟨3, 0, 3, 1, true⟩] = [] := by
  decide

/-- 三档（P、E、低功耗 E），容器只给了 E 与低功耗 E：能用的最快一档是 E，低功耗 E 不进计划。 -/
example :
    (plan [⟨0, 2, 0, 0, false⟩, ⟨1, 1, 1, 0, true⟩, ⟨2, 1, 2, 0, true⟩, ⟨3, 0, 3, 1, true⟩]).map (·.id)
      = [1, 2] := by decide

end Sprawling.Serving.Placement.Plan
