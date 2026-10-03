-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.sprawling.spec.Serving.Placement.Plan

/-!
# 热线程的座位：计划里的处理器，一个座位一条线程，坐下就不换

规定 `crates/sprawling/src/serving/placement.rs`（`bin::serving::placement`，形状：状态机）里核心热线程的软放置（§8-93 的放置一半，AF1 的 (b) 与 (c)）。座位从哪里来是放置计划（`crates/sprawling/spec/Serving/Placement/Plan.lean`，`bin::serving::placement::plan`）：一个纯函数，从读到的拓扑给出最快一档每个物理核一个逻辑处理器；只有一档、拓扑不自洽、平台读不到时计划是空的。本模型只管座位表：计划给出 `n` 个座位，热线程起动时拿一个空座位，没有空座位就不拿——那条线程由操作系统放；座位表从不让两条热线程共用一个座位，于是坐着的线程数永不超过最快一档的物理核数。`crates/sprawling/src/serving/placement/tests.rs` 在 Rust 的座位表上逐条检查下面的性质：至多四个座位、三条线程六次起动与退出的每一条轨迹，穷举而不抽样。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

热线程是：账本线程 `sprawling-runs`（`bin::assembly::attending`）、视图折叠线程 `sprawling-views`（`bin::serving::folding`）、socket 服务的 tokio worker（`serving_runtime`），以及每个 run 的 lane（`accounting::worker::pool` 起的 OS 线程）。座位随 `Seat` 析构交还。lane 要座位的那一处见 D47。

性质，都对任意的起动与退出序列成立：

* **一座一人**——从空表起，任意轨迹之后，没有两条线程坐同一个座位，没有一条线程坐两个座位，每个座位都在计划里（`every_trace_keeps_the_table_good`），所以坐着的线程数不超过座位数（`seated_threads_never_exceed_the_seats`）；
* **有空就给**——没有座位的线程起动时，有空座位就坐第一个空座位（`a_start_takes_the_first_free_seat`）；拿不到座位只发生在每个座位都有人时（`a_start_is_left_to_the_os_only_when_every_seat_is_taken`）；
* **坐下不换**——一条线程有了座位，只要它自己没有退出，座位就不变（`a_seat_is_kept_while_its_thread_lives`）；退出就交还（`an_exit_gives_the_seat_back`）。

模型不说的事：座位交给操作系统的是软的理想处理器，那个核忙时调度器照样把线程放到别的核（AF1 要的「忙了立刻换下一个核」）；线程在一步计算中途是否换了核，由四臂对照在阶段边界采样处理器号读出，不在模型里。
-/

namespace Sprawling.Serving.Placement

/-- 座位表：`(thread, seat)`，seat 是计划里的下标。 -/
abbrev Seats := List (Nat × Nat)

/-- 一个座位上坐着几条热线程。 -/
def load (seats : Seats) (seat : Nat) : Nat :=
  (seats.filter (fun s => s.2 == seat)).length

/-- 计划有 `n` 个座位时，第一个空座位。 -/
def free (n : Nat) (seats : Seats) : Option Nat :=
  (List.range n).find? (fun i => load seats i == 0)

/-- 一条热线程现在坐在哪里。 -/
def seatOf (seats : Seats) (thread : Nat) : Option Nat := seats.lookup thread

/-- 座位表看到的事：一条热线程起动，或退出。 -/
inductive Event where
  | start (thread : Nat)
  | exit (thread : Nat)
  deriving Repr, DecidableEq

/-- 一件事之后的座位表。已有座位的线程再报起动不换座位；没有空座位就不给。 -/
def step (n : Nat) (seats : Seats) : Event → Seats
  | .start t =>
    match seatOf seats t with
    | some _ => seats
    | none =>
      match free n seats with
      | some c => (t, c) :: seats
      | none => seats
  | .exit t => seats.filter (fun s => s.1 != t)

/-- 一串事之后的座位表。 -/
def run (n : Nat) (seats : Seats) (events : List Event) : Seats :=
  events.foldl (step n) seats

/-- 座位表是好的：线程不重复，座位不重复，座位都在计划里。 -/
def Good (n : Nat) (seats : Seats) : Prop :=
  (seats.map (·.1)).Nodup ∧ (seats.map (·.2)).Nodup ∧ ∀ s ∈ seats, s.2 < n

theorem lookup_none_not_mem (seats : Seats) (t : Nat) (h : seats.lookup t = none) :
    t ∉ seats.map (·.1) := by
  induction seats with
  | nil => simp
  | cons s rest ih =>
    obtain ⟨a, b⟩ := s
    by_cases hta : t = a
    · subst hta; simp [List.lookup] at h
    · have hne : (t == a) = false := by simp [hta]
      simp only [List.lookup, hne] at h
      simp only [List.map_cons, List.mem_cons, not_or]
      exact ⟨hta, ih h⟩

theorem load_zero_not_mem (seats : Seats) (c : Nat) (h : load seats c = 0) :
    c ∉ seats.map (·.2) := by
  intro hm
  obtain ⟨s, hs, hsc⟩ := List.mem_map.mp hm
  have hin : s ∈ seats.filter (fun s => s.2 == c) := List.mem_filter.mpr ⟨hs, by simp [hsc]⟩
  simp only [load, List.length_eq_zero_iff] at h
  rw [h] at hin
  simp at hin

theorem free_spec {n : Nat} {seats : Seats} {c : Nat} (h : free n seats = some c) :
    c < n ∧ load seats c = 0 := by
  have hm := List.mem_of_find?_eq_some h
  have hp := List.find?_some h
  simp only [List.mem_range] at hm
  simp only [beq_iff_eq] at hp
  exact ⟨hm, hp⟩

theorem filter_good (n : Nat) (seats : Seats) (p : Nat × Nat → Bool) (h : Good n seats) :
    Good n (seats.filter p) := by
  obtain ⟨h1, h2, h3⟩ := h
  have hs := List.filter_sublist (p := p) (l := seats)
  exact ⟨h1.sublist (hs.map _), h2.sublist (hs.map _),
    fun s hm => h3 s (List.mem_filter.mp hm).1⟩

theorem step_good (n : Nat) (seats : Seats) (e : Event) (h : Good n seats) :
    Good n (step n seats e) := by
  cases e with
  | start t =>
    simp only [step]
    split
    · exact h
    · rename_i hnone
      split
      · rename_i c hc
        obtain ⟨hlt, hload⟩ := free_spec hc
        obtain ⟨h1, h2, h3⟩ := h
        refine ⟨?_, ?_, ?_⟩
        · rw [List.map_cons, List.nodup_cons]
          exact ⟨lookup_none_not_mem seats t hnone, h1⟩
        · rw [List.map_cons, List.nodup_cons]
          exact ⟨load_zero_not_mem seats c hload, h2⟩
        · intro s hs
          rcases List.mem_cons.mp hs with rfl | hs
          · exact hlt
          · exact h3 s hs
      · exact h
  | exit t => exact filter_good n seats _ h

theorem run_good (n : Nat) (seats : Seats) (events : List Event) (h : Good n seats) :
    Good n (run n seats events) := by
  induction events generalizing seats with
  | nil => exact h
  | cons e rest ih =>
    simp only [run, List.foldl_cons]
    exact ih _ (step_good n seats e h)

/-- 从空表起，任意轨迹之后座位表都是好的。 -/
theorem every_trace_keeps_the_table_good (n : Nat) (events : List Event) :
    Good n (run n [] events) :=
  run_good n [] events ⟨List.nodup_nil, List.nodup_nil, fun _ h => by simp at h⟩

theorem nodup_bounded_length (l : List Nat) (n : Nat) (hd : l.Nodup) (hb : ∀ x ∈ l, x < n) :
    l.length ≤ n := by
  simpa using hd.length_le_of_subset (fun x hx => List.mem_range.mpr (hb x hx))

/-- 坐着的线程数不超过座位数。 -/
theorem seated_threads_never_exceed_the_seats (n : Nat) (seats : Seats) (h : Good n seats) :
    seats.length ≤ n := by
  obtain ⟨-, h2, h3⟩ := h
  have hlen := nodup_bounded_length (seats.map (·.2)) n h2
    (fun x hx => by
      obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hx
      exact h3 s hs)
  simpa using hlen

theorem seatOf_cons_self (seats : Seats) (t c : Nat) : seatOf ((t, c) :: seats) t = some c := by
  simp [seatOf, List.lookup]

theorem seatOf_cons_other (seats : Seats) (t t' c : Nat) (h : t ≠ t') :
    seatOf ((t', c) :: seats) t = seatOf seats t := by
  have : (t == t') = false := by simp [h]
  simp [seatOf, List.lookup, this]

/-- 有空就给：没有座位的线程起动，坐第一个空座位。 -/
theorem a_start_takes_the_first_free_seat (n : Nat) (seats : Seats) (t c : Nat)
    (hnone : seatOf seats t = none) (hc : free n seats = some c) :
    seatOf (step n seats (.start t)) t = some c := by
  simp only [step, hnone, hc]
  exact seatOf_cons_self seats t c

/-- 拿不到座位只发生在每个座位都有人时。 -/
theorem a_start_is_left_to_the_os_only_when_every_seat_is_taken (n : Nat) (seats : Seats) (t : Nat)
    (hnone : seatOf seats t = none) (hleft : seatOf (step n seats (.start t)) t = none) :
    ∀ i < n, 0 < load seats i := by
  intro i hi
  cases hf : free n seats with
  | some c =>
    rw [a_start_takes_the_first_free_seat n seats t c hnone hf] at hleft
    simp at hleft
  | none =>
    simp only [free, List.find?_eq_none, List.mem_range, beq_iff_eq] at hf
    have := hf i hi
    omega

theorem lookup_without_another (seats : Seats) (t t' : Nat) (h : t ≠ t') :
    (seats.filter (fun s => s.1 != t')).lookup t = seats.lookup t := by
  induction seats with
  | nil => rfl
  | cons s rest ih =>
    obtain ⟨a, b⟩ := s
    by_cases ha : a = t'
    · subst ha
      have : (t == a) = false := by simp [h]
      simp [List.filter, List.lookup, this, ih]
    · have hk : (a != t') = true := by simp [ha]
      simp only [List.filter, hk]
      by_cases hta : t = a
      · subst hta; simp [List.lookup]
      · have : (t == a) = false := by simp [hta]
        simp [List.lookup, this, ih]

theorem lookup_without_self (seats : Seats) (t : Nat) :
    (seats.filter (fun s => s.1 != t)).lookup t = none := by
  induction seats with
  | nil => rfl
  | cons s rest ih =>
    obtain ⟨a, b⟩ := s
    by_cases ha : a = t
    · subst ha; simp [List.filter, ih]
    · have hk : (a != t) = true := by simp [ha]
      have : (t == a) = false := by simp [Ne.symm ha]
      simp [List.filter, hk, List.lookup, this, ih]

/-- 别的线程的一件事不动这条线程的座位。 -/
theorem a_step_keeps_the_seat (n : Nat) (seats : Seats) (t c : Nat) (e : Event)
    (hs : seatOf seats t = some c) (he : e ≠ .exit t) :
    seatOf (step n seats e) t = some c := by
  cases e with
  | start t' =>
    simp only [step]
    split
    · exact hs
    · rename_i hnone
      have ht : t ≠ t' := by
        intro heq; subst heq; simp_all
      split
      · rw [seatOf_cons_other seats t t' _ ht]; exact hs
      · exact hs
  | exit t' =>
    have ht : t ≠ t' := by
      intro heq; subst heq; exact he rfl
    simp only [step, seatOf]
    rw [lookup_without_another seats t t' ht]
    exact hs

/-- 坐下不换：只要这条线程自己没有退出，任意一串别的起动与退出之后它还坐在原处。 -/
theorem a_seat_is_kept_while_its_thread_lives (n : Nat) (events : List Event)
    (seats : Seats) (t c : Nat) (hs : seatOf seats t = some c)
    (hlive : ∀ e ∈ events, e ≠ .exit t) :
    seatOf (run n seats events) t = some c := by
  induction events generalizing seats with
  | nil => exact hs
  | cons e rest ih =>
    simp only [run, List.foldl_cons]
    exact ih (step n seats e)
      (a_step_keeps_the_seat n seats t c e hs (hlive e (by simp)))
      (fun e' he' => hlive e' (by simp [he']))

/-- 退出交还座位。 -/
theorem an_exit_gives_the_seat_back (n : Nat) (seats : Seats) (t : Nat) :
    seatOf (step n seats (.exit t)) t = none :=
  lookup_without_self seats t

/-- 两个座位、三条线程：第三条拿不到座位，由操作系统放。 -/
example : run 2 [] [.start 1, .start 2, .start 3] = [(2, 1), (1, 0)] := by decide

/-- 坐第一个座位的线程退出，空出的座位给下一条线程，别的线程不挪。 -/
example : run 2 [] [.start 1, .start 2, .exit 1, .start 3] = [(3, 0), (2, 1)] := by decide

/-- 计划是空的（一档、不自洽、读不到）：谁也不拿座位。 -/
example : run 0 [] [.start 1, .start 2] = [] := by decide

end Sprawling.Serving.Placement

/-! D41 操作系统的调度器做主：harness 只给软的理想处理器，一步计算不跨线程，硬亲和只作对照臂

**接口**

```rust
// bin::serving::placement —— shape: state machine（本分部的座位模型）
pub(crate) struct Holder(pub(crate) u64);       // 一条热线程，活多久就坐多久
pub(crate) struct SeatTable;                    // 模型的 Seats 与 step
impl SeatTable {
    pub(crate) fn new(seats: Vec<Processor>) -> Self;  // 座位就是放置计划给出的处理器
    pub(crate) fn start(&mut self, holder: Holder);    // 第一个空座位；没有空座位、或已有座位，什么也不做
    pub(crate) fn exit(&mut self, holder: Holder);     // 交还座位，别的座位不动
    pub(crate) fn seat_of(&self, holder: Holder) -> Option<Processor>;
}
pub(crate) struct Seat;                         // 析构即交还
pub(crate) fn seat_this_thread(name: &'static str) -> Seat; // 要座位并设理想处理器；平台拒绝时向标准错误说一次
pub(crate) fn report() -> String;               // doctor 的一行：读到的拓扑与计划做了什么（D47）
```

全进程一张座位表，放在一个 `Mutex` 后面，第一次要座位时读拓扑、算计划、建表：只在线程起动与退出时取锁，不在热路径上。哪一臂由常量 `ARM` 定（`Off` 不读拓扑、不要座位、不调平台；`Soft` 是默认），见 D47。

**决定**（AF1 的 (b) 与 (c)，D88 第 3 条）：

- **调度器做主。** harness 只给提示与软偏好：理想处理器是软的，那个核忙时 Windows 立刻把线程放到别的核上；默认从不设硬亲和。座位只从放置计划里来（D45），计划空时谁也不拿座位，线程全由操作系统放。座位比热线程少时，多出来的线程不拿座位（模型的「一座一人」），由操作系统放，而不是两条热线程挤在同一个首选核上。
- **(b) 理想处理器，三个平台。**
  - Windows：热线程起动时向座位表要座位，再调 `thread_priority::windows::set_current_thread_ideal_processor`（`thread-priority` 3.1.1，对外是安全接口，内部是 `SetThreadIdealProcessor`；第一档），退出时交还。理想处理器是线程所在处理器组里的下标，所以计划只含读拓扑那条线程所在组里的处理器（D45）；起动的线程不在那个组里时不设理想处理器，照实向标准错误说一次。
  - macOS：没有理想处理器的接口（`THREAD_AFFINITY_POLICY` 在 Apple 芯片上不受支持），座位表不建；性能核由 D40 与 `crates/runtime/spec/Tools/Exec.lean` D29 的 QoS 分工争取，拓扑只读给 doctor（D45）。
  - Linux：没有软的理想处理器调用，`sched_setaffinity` 是硬亲和（下面的对照臂）；座位表不建。内核调度器在 Intel 混合架构（ITMT）与 ARM（EAS）上本来就把忙线程放到大核上；拓扑只读给 doctor（D45）。
- **(c) 一步计算从头到尾在一条线程上。** 一步就是 §8-93 的一轮：热线程从一次醒来到下一次阻塞做完的一件事（折叠一条记录、执行一次工具调用、落一次 relay）。一步之内不把工作交给另一条线程：不经 channel 转手、不 spawn 后再 join、async 任务不在一步中途转去阻塞池。lane 本来各是一条 OS 线程；tokio worker 的一步是一次任务轮询，tokio 的偷任务只发生在两次轮询之间。这是代码的写法，不要平台接口，三个平台同一条。它守住了没有，由下面四臂对照的「中途换核次数」读出。
- **硬亲和只作对照臂。** Windows：harness 进程经 `win32job` 2.0.3 的 `Job::assign_current_process` 装进一只 job，再以 `ExtendedLimitInfo::limit_affinity` 设成计划里处理器的掩码；每个 run 的 job 设成其余处理器的掩码（同一 crate 的安全接口，第一档）。Linux：整个二进制在 `taskset -c` 下起动，子进程同样包一层 `taskset`（外部命令，第一档；`sched_setaffinity` 本身要 `unsafe`）。macOS：没有硬亲和，这一臂不跑，照实说。

**被否**：①硬亲和作默认（每个 session 一个核）——核忙时宁可排队也不换核，正与 D88 第 3 条「算不过来就立即下一个核」相反；②座位不够时两条热线程共用最空的座位——两条都首选同一个核，那个核一忙两条一起被挪，等于没有偏好，还把座位数抬到最快一档的物理核数之上；③按负载随时改理想处理器——活着的线程偏好一变，就是「计算中途跳来跳去」，模型的「坐下不换」排除了它。

**重开参数**：四臂对照里 (a)+(b)+(c) 臂的工具调用与 relay 等待 p99 不优于「不做」臂（在噪声以内）——那就删掉 (b)，只留 (a) 与 (c)；或者硬亲和臂的 p99 与 p999 都明显更好。
-/

/-! D45 拓扑是读出来的，从不假设；放置计划是拓扑上的纯函数

**接口**

```rust
// bin::serving::placement::plan —— shape: decision（`spec/Serving/Placement/Plan.lean`）
pub(crate) struct Processor { pub(crate) group: u16, pub(crate) number: u32 } // 平台的处理器号：组与组内下标；Ord 先组后号
pub(crate) enum Access { Allowed, Barred }      // 本进程能不能用（亲和、Job Object、cpuset、虚拟机）
pub(crate) struct Cpu { pub(crate) processor: Processor, pub(crate) class: u8, pub(crate) core: u32, pub(crate) cache: u32, pub(crate) access: Access }
pub(crate) struct Topology;                     // 读到的样子，顺序随平台
impl Topology { pub(crate) fn new(cpus: Vec<Cpu>) -> Self; pub(crate) fn shape(&self) -> Shape; }
pub(crate) struct Shape { pub(crate) classes: Vec<Class>, pub(crate) caches: usize, pub(crate) usable: usize, pub(crate) logical: usize }
pub(crate) struct Class { pub(crate) class: u8, pub(crate) cores: usize, pub(crate) logical: usize } // 只数能用的；最快的在前
pub(crate) enum Plan { Seats(Vec<Processor>), LeftToOs(Left) }  // Seats 恒不空，按处理器号排好
pub(crate) enum Left { OneClass, Inconsistent, NothingUsable }
pub(crate) fn plan(topology: &Topology) -> Plan;

// bin::serving::placement::reading —— shape: adapter
pub(crate) fn read() -> Result<Topology, Unread>;  // 每个平台一条臂
pub(crate) struct Unread(String);                   // 为什么没读到，写给 doctor 与标准错误
```

**决定**：

- **读，不假设。** 档的个数不限（Intel Meteor Lake 与 Lunar Lake 有 P、E、SoC 上的低功耗 E 三档；AMD Zen 5 加 Zen 5c；ARM 的 prime、big、little），物理核、超线程兄弟、末级缓存组、处理器组、本进程能用的处理器都从平台读。
  - Windows：`GetSystemCpuSetInformation` 的每条记录给 `Group`、`LogicalProcessorIndex`、`CoreIndex`、`LastLevelCacheIndex`、`EfficiencyClass`（数越大越快）、`Allocated` 与 `AllocatedToTargetProcess`；本进程能用的是读拓扑那条线程的 `GetThreadGroupAffinity` 掩码里的处理器（Job Object 的亲和限额已经算进这张掩码），去掉分配给别的进程独占的。逻辑处理器多于 64 个的机器（两组的 Threadripper）只用这条线程所在的一组，因为理想处理器是组内下标。`Parked` 是此刻的电源状态，计划不看它。接口档位：`std`、`thread-priority` 3.1.1、`win32job` 2.0.3、`winsafe` 0.0.29 都不给这两项，所以走第二档：`crates/desktop/ffi` 的 Zig 叶子只做调用，把原始记录写进 Rust 借出的缓冲（`(ptr, len)`），记录由安全的 Rust（`desktop_ffi::cpu_set`）逐条按每条记录自己的 `Size` 解析，那一段在每个平台上都编译，所以本机实读的记录作为测试夹具在三个平台上都被解析（`desktop_ffi` D4）。
  - Linux：本进程能用的处理器读 `/proc/self/status` 的 `Cpus_allowed_list`（`sched_getaffinity` 的同一个答案，标准库读文件，第一档；容器的 cpuset 与 `taskset` 都算进去）；档：有 `/sys/devices/cpu_core/cpus` 与 `/sys/devices/cpu_atom/cpus`（Intel 混合架构）就按它们分两档，否则按 `/sys/devices/system/cpu/cpu<n>/cpu_capacity` 的不同取值分档（ARM 的 big.LITTLE 与 DynamIQ），都没有就是一档；物理核是 `topology/physical_package_id` 与 `topology/core_id`，末级缓存组是 `cache/index3/id`（没有就 `index2`）。`amd_pstate` 的 prefcore 排的是同一档里的核，计划把同一档交给操作系统，所以不读。读出来的拓扑只给 doctor：Linux 没有软的理想处理器调用（D41）。
  - macOS：`sysctl -n hw.nperflevels` 与每一级的 `hw.perflevel<n>.physicalcpu`、`hw.perflevel<n>.logicalcpu`（`std::process::Command` 起 `/usr/sbin/sysctl`，第一档；`sysctlbyname` 要 `unsafe`），`perflevel0` 最快；只给 doctor，因为 macOS 没有放置调用。
- **计划是纯函数**（`Plan.lean` 证明它的性质）：拓扑不自洽（处理器号重复、同一个物理核报两档）或没有能用的处理器就不放置；能用的只有一档就不放置，不论末级缓存分几组——单 CCD 的 X3D、大多数台式机与服务器、Snapdragon X、CI 的虚拟机都是一档；双 CCD 的 7950X3D、9950X3D 是一档两种 L3，AMD 的 `amd_3d_vcache` 驱动与 Windows 的游戏模式已经在分它们，第二个意见只会与它打架；有几档时，计划是最快一档每个物理核上能用的、号最小的逻辑处理器，按处理器号排好，最慢一档恒不进计划，其余处理器与多出来的热线程交给操作系统。同一张拓扑以任何顺序读进来，计划都一样。
- **读不到就不放置。** 叶子报错、sysfs 缺字段、解析出不自洽的记录，都是「交给操作系统」，doctor 说出读不到的原因；harness 照常起动。

**被否**：①只认两档（P 与 E）、把 `EfficiencyClass` 非零当 P——三档的 Meteor Lake 上低功耗 E 核会被当作 E 核与 E 核一起排，Zen 5c 与 ARM 的三档也一样，而「最快一档、其余交给操作系统」对任何档数都对；②按缓存给双 CCD 的 X3D 排座位——AMD 的驱动按负载选 CCD，我们只知道哪一块缓存大，不知道这时哪一块更合适；③沿用 `available_parallelism` 个处理器一档的退路去分散热线程——在一档的机器上那是第二个意见，在混合机器上它会把热线程首选到 E 核上；④在 Rust 里直接写 `unsafe` 调 `GetSystemCpuSetInformation`——平台调用的次序只在测量表明它最好时才许。

**重开参数**：一个对外安全的 crate 给出 CPU set 信息（那时叶子的这两个函数离开）；或四臂对照读出：在一档的机器上分散热线程明显降低 p99（那时「一档不放置」改成「一档每个物理核一个座位」，`Plan.lean` 的 `one_class_gives_no_plan` 随之改写）。
-/

/-! D46 每个 run 的 lane 在 lane 起动的那一处要座位

**决定**：lane 是 `accounting::worker::pool` 起的 OS 线程，`accounting` 不依赖本 crate；座位经 `accounting::worker::hands::Hands` 交进去的一个起动钩子要，像 `Hands.monotonic` 交进单调钟那样：`bin::assembly` 把 `placement::seat_this_thread` 包成钩子交进去，lane 线程在闭包开头调它一次，拿到的 `Seat` 活到线程结束。这是 lane 要座位的唯一一处。

**被否**：`accounting` 直接依赖 `thread-priority` 自己设理想处理器——座位表就有了两份，一座一人守不住。

**重开参数**：lane 不再是一条 OS 线程（例如改成 tokio 任务），那时它的座位就是它所在 worker 的座位，钩子删去。
-/

/-! D47 一个设置关掉放置；doctor 用 User 读得懂的话说出读到的拓扑与计划

**决定**：

- 关掉放置的是常量 `ARM`（`Off` 不读拓扑、不要座位、不调平台），对照时只改这一个值；读数定下默认之后它挪到人的配置 `[core] placement`（下面四臂对照），那是 `accounting::person` 的一个读者加一行，这一步与四臂对照的读数一起做。
- doctor 一行，例：「CPU: 2 classes — 4 performance cores (8 threads), 8 efficiency cores; hot threads prefer the 4 performance cores」；「CPU: one class, 8 cores; left to the operating system」；「CPU: one class, 16 cores in 2 cache groups; left to the operating system and its cache steering」；读不到时「CPU: topology unread (<原因>); left to the operating system」；macOS 与 Linux 上计划有座位时，句末写「this platform has no placement call; its scheduler places threads」。措辞只在 `placement::report` 一处。

**被否**：doctor 打出原始的记录表——User 要的是机器被怎样对待，不是 `EfficiencyClass` 的数。
-/

/-! ## 四臂对照（AF1 的完成条件，测量计划）

测量归波后的 mid 读数（Roadmap §0 第 7 条），这里只写计划，实现照它留出开关与采样点。

- **开关是一个配置值**：人的配置文件 `[core]` 一节的 `placement`，与 `priority` 同处（`accounting::person`，`crates/accounting/Spec.lean` §8-8），取 `"none"`、`"soft"`、`"soft_shares"`、`"pinned"`，分别是下面四臂。读数出来之前的默认是 `"soft"`；读数定下默认之后只改这一个默认值。
- **四臂**：①不做——不关 EcoQoS、不设理想处理器、run 的 job 不设份额；②(a)+(b)+(c)；③再加 (d)，即 `crates/runtime/spec/Tools/Exec.lean` D29 的按 run CPU 权重与作业级内存上限；④硬亲和（D41 的对照臂）。不做那一臂就是 `ARM = Off`。
- **负载**：TP1 吞吐台的同一个 citysim 场景，并发 run 数 4 与 16，另在后台跑 N ∈ {0, 4, 16} 个 `cargo build`，每个都经一个 run 的 exec 起动（于是它们落在各自 run 的 job 里，与真实城一样）。每臂每格重复 5 次，报中位数，写明机器类别（核数与 P/E 之分）。
- **读数**：工具调用的 harness 开销（`tool_called` 到 `tool_result`）与 relay 往返，各自的 p50、p99、p999；每步计算中途换核的次数——在 M2 的阶段边界（醒来、工具执行前、工具执行后、再次阻塞）各采一次当前处理器号，一步里前后不同就计一次，按热线程的种类分开计。
- **处理器号怎么采**：Windows 是 `GetCurrentProcessorNumberEx`，没有安全接口，放进 D41 的同一个 Zig 叶子；Linux 读 `/proc/thread-self/stat` 的第 39 个字段（标准库读文件，第一档；`sched_getcpu` 要 `unsafe`），只在测量构建里读，因为每次一个系统调用；macOS 没有读当前处理器的接口，只报延迟，换核次数写「不可测」。
- **按读数定默认**：p99 最低、且 p999 不比「不做」差的一臂；相差在噪声以内时取机制最少的一臂。读数与选择写进 `tools/xtask/budgets.toml` 的新行，由做测量的那一道写。
-/
