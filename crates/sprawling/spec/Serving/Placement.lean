-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 热线程的理想处理器：好核在先，最空的先给，坐下就不换

规定核心热线程的软放置（§8-93 的放置一半，AF1 的 (b) 与 (c)）。实现它的模块是 W6 新建的 bin::serving::placement（先登记进模块图，再建文件；建好之后本段改成指向那个文件的路径）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

热线程是：账本线程 `sprawling-runs`、视图折叠线程 `sprawling-views`、socket 服务的 tokio worker，以及每个 run 的 lane 线程。每条热线程起动时向放置表要一个座位（一个逻辑处理器的下标），把它设成自己的理想处理器，退出时交还。

处理器表是一张按「好坏」排好的表：每一项是一个逻辑处理器的等级 `rank`，数越小越好，表按等级从小到大排。等级由平台给出（见 D41）；模型只要求表是排好的（`RankedBest`），不关心等级是怎么来的，所以三档、两档、一档（不是混合架构的机器）都在模型里。

挑座位的规则只有一条：在处理器表里挑坐着的热线程最少的那一个，一样少就取靠前的。于是好核先坐满一轮，才轮到下一档；热线程退出空出来的座位在下一次起动时最先被填上；这就是「好核优先、轮流分配」。坐下之后座位不变，直到这条线程退出：放置表从不搬动一条活着的线程。

两组性质，都对任意的起动与退出序列成立：

* **好核先坐**——一次起动挑到等级为 `r` 的处理器，当时每一个等级比 `r` 好的处理器上都至少坐着一条热线程（`a_start_takes_a_worse_core_only_when_every_better_core_is_seated`，以及对每一条轨迹的 `on_every_trace_a_worse_core_waits_for_the_better_ones`）；
* **坐下不换**——一条线程有了座位，之后不论别的线程怎样起动与退出，只要它自己没有退出，座位就不变（`a_seat_is_kept_while_its_thread_lives`）；线程退出就交还座位（`an_exit_gives_the_seat_back`）。

模型不说的事：理想处理器是软偏好，操作系统可以在那个核忙时把线程放到别的核上（这正是 AF1 要的「忙了立刻换下一个核」）；模型只规定放置表交给操作系统的偏好。线程在一步计算中途是否换了核，由 §8-93 的四臂对照在阶段边界采样处理器号读出，不在模型里。
-/

namespace Sprawling.Serving.Placement

/-- 处理器表：每一项是一个逻辑处理器的等级，数越小越好。 -/
abbrev Cores := List Nat

/-- 表按等级排好：靠前的处理器不比靠后的差。 -/
def RankedBest (cores : Cores) : Prop :=
  ∀ i j a b : Nat, i < j → cores[i]? = some a → cores[j]? = some b → a ≤ b

/-- 放置表：`(thread, core)`，core 是处理器表里的下标。 -/
abbrev Seats := List (Nat × Nat)

/-- 一个处理器上坐着几条热线程。 -/
def load (seats : Seats) (core : Nat) : Nat :=
  (seats.filter (fun s => s.2 == core)).length

/-- 从下标 `i` 起，在余下的处理器里挑坐的线程最少的一个，一样少取靠前的。 -/
def pickFrom (seats : Seats) : Nat → Cores → Option Nat
  | _, [] => none
  | i, _ :: rest =>
    match pickFrom seats (i + 1) rest with
    | none => some i
    | some j => if load seats j < load seats i then some j else some i

/-- 起动的线程拿到的座位（Rust：放置表的 `claim`）。 -/
def pick (seats : Seats) (cores : Cores) : Option Nat := pickFrom seats 0 cores

/-- 一条热线程现在坐在哪里。 -/
def seatOf (seats : Seats) (thread : Nat) : Option Nat := seats.lookup thread

/-- 放置表看到的事：一条热线程起动，或退出。 -/
inductive Event where
  | start (thread : Nat)
  | exit (thread : Nat)
  deriving Repr, DecidableEq

/-- 一件事之后的放置表。已有座位的线程再报起动不换座位；处理器表是空的就不给座位。 -/
def step (cores : Cores) (seats : Seats) : Event → Seats
  | .start t =>
    match seatOf seats t with
    | some _ => seats
    | none =>
      match pick seats cores with
      | some c => (t, c) :: seats
      | none => seats
  | .exit t => seats.filter (fun s => s.1 != t)

/-- 一串事之后的放置表。 -/
def run (cores : Cores) (seats : Seats) (events : List Event) : Seats :=
  events.foldl (step cores) seats

theorem pickFrom_in_range (seats : Seats) (cores : Cores) (i j : Nat)
    (h : pickFrom seats i cores = some j) : i ≤ j ∧ j < i + cores.length := by
  induction cores generalizing i j with
  | nil => simp [pickFrom] at h
  | cons c rest ih =>
    simp only [pickFrom] at h
    split at h
    · simp at h; simp; omega
    · rename_i j' hj'
      have := ih (i + 1) j' hj'
      simp only [List.length_cons]
      split at h <;> (simp at h; omega)

/-- 挑到的是第一个最空的：它前面的每一个处理器都坐得比它满。 -/
theorem pickFrom_first_least (seats : Seats) (cores : Cores) (i j : Nat)
    (h : pickFrom seats i cores = some j) :
    ∀ k, i ≤ k → k < j → load seats j < load seats k := by
  induction cores generalizing i j with
  | nil => simp [pickFrom] at h
  | cons c rest ih =>
    intro k hik hkj
    simp only [pickFrom] at h
    split at h
    · simp at h; omega
    · rename_i j' hj'
      split at h
      · rename_i hlt
        simp at h
        subst h
        by_cases hk : k = i
        · subst hk; exact hlt
        · exact ih (i + 1) j' hj' k (by omega) hkj
      · simp at h; omega

/-- 处理器表不空，起动的线程总拿得到座位：好核先坐不是从一个永远不给座位的表推出来的。 -/
theorem pick_seats_somebody (seats : Seats) (c : Nat) (rest : Cores) :
    ∃ j, pick seats (c :: rest) = some j := by
  simp only [pick, pickFrom]
  split
  · exact ⟨0, rfl⟩
  · split
    · exact ⟨_, rfl⟩
    · exact ⟨0, rfl⟩

/-- 挑到等级为 `r` 的处理器时，每一个更好的处理器上都坐着热线程。 -/
theorem pick_takes_a_worse_core_only_when_every_better_core_is_seated
    (cores : Cores) (hr : RankedBest cores) (seats : Seats) (j r : Nat)
    (hj : pick seats cores = some j) (hjr : cores[j]? = some r) :
    ∀ p q, cores[p]? = some q → q < r → 0 < load seats p := by
  intro p q hp hq
  have hpj : p < j := by
    rcases Nat.lt_trichotomy p j with h | h | h
    · exact h
    · subst h; simp_all
    · have := hr j p r q h hjr hp; omega
  have := pickFrom_first_least seats cores 0 j hj p (Nat.zero_le p) hpj
  omega

theorem seatOf_cons_self (seats : Seats) (t c : Nat) : seatOf ((t, c) :: seats) t = some c := by
  simp [seatOf, List.lookup]

theorem seatOf_cons_other (seats : Seats) (t t' c : Nat) (h : t ≠ t') :
    seatOf ((t', c) :: seats) t = seatOf seats t := by
  have : (t == t') = false := by simp [h]
  simp [seatOf, List.lookup, this]

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

/-- 一次起动：线程原来没有座位，起动之后坐在等级为 `r` 的处理器上，那么当时每一个更好的处理器上都坐着热线程。 -/
theorem a_start_takes_a_worse_core_only_when_every_better_core_is_seated
    (cores : Cores) (hr : RankedBest cores) (seats : Seats) (t c r : Nat)
    (hnone : seatOf seats t = none)
    (hc : seatOf (step cores seats (.start t)) t = some c) (hcr : cores[c]? = some r) :
    ∀ p q, cores[p]? = some q → q < r → 0 < load seats p := by
  simp only [step, hnone] at hc
  split at hc
  · rename_i j hj
    rw [seatOf_cons_self] at hc
    simp at hc
    subst hc
    exact pick_takes_a_worse_core_only_when_every_better_core_is_seated cores hr seats _ r hj hcr
  · simp [hnone] at hc

/-- 同一条性质对每一条从空放置表开始的轨迹上的每一次起动成立。 -/
theorem on_every_trace_a_worse_core_waits_for_the_better_ones
    (cores : Cores) (hr : RankedBest cores) (before : List Event) (t c r : Nat)
    (hnone : seatOf (run cores [] before) t = none)
    (hc : seatOf (step cores (run cores [] before) (.start t)) t = some c)
    (hcr : cores[c]? = some r) :
    ∀ p q, cores[p]? = some q → q < r → 0 < load (run cores [] before) p :=
  a_start_takes_a_worse_core_only_when_every_better_core_is_seated
    cores hr (run cores [] before) t c r hnone hc hcr

/-- 别的线程的一件事不动这条线程的座位。 -/
theorem a_step_keeps_the_seat (cores : Cores) (seats : Seats) (t c : Nat) (e : Event)
    (hs : seatOf seats t = some c) (he : e ≠ .exit t) :
    seatOf (step cores seats e) t = some c := by
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
theorem a_seat_is_kept_while_its_thread_lives (cores : Cores) (events : List Event)
    (seats : Seats) (t c : Nat) (hs : seatOf seats t = some c)
    (hlive : ∀ e ∈ events, e ≠ .exit t) :
    seatOf (run cores seats events) t = some c := by
  induction events generalizing seats with
  | nil => exact hs
  | cons e rest ih =>
    simp only [run, List.foldl_cons]
    exact ih (step cores seats e)
      (a_step_keeps_the_seat cores seats t c e hs (hlive e (by simp)))
      (fun e' he' => hlive e' (by simp [he']))

/-- 退出交还座位。 -/
theorem an_exit_gives_the_seat_back (cores : Cores) (seats : Seats) (t : Nat) :
    seatOf (step cores seats (.exit t)) t = none :=
  lookup_without_self seats t

/-- 两个 P 核、一个 E 核：先坐满两个 P 核，再坐 E 核，再回到第一个 P 核。 -/
example : run [0, 0, 1] [] [.start 1, .start 2, .start 3, .start 4] = [(4, 0), (3, 2), (2, 1), (1, 0)] := by
  decide

/-- 坐在 P 核上的线程退出，空出的 P 核先于 E 核给下一条线程，别的线程不挪。 -/
example : run [0, 0, 1] [] [.start 1, .start 2, .start 3, .exit 2, .start 5] =
    [(5, 1), (3, 2), (1, 0)] := by
  decide

/-- 已有座位的线程再报起动，不换座位。 -/
example : run [0, 1] [] [.start 1, .start 1] = [(1, 0)] := by decide

/-- 处理器表是排好的：两个 P 核、一个 E 核、一个 P 核的超线程兄弟。 -/
example : RankedBest [0, 0, 1, 2] := by
  unfold RankedBest
  intro i j a b hij hi hj
  match i, j with
  | 0, 1 | 0, 2 | 0, 3 | 1, 2 | 1, 3 | 2, 3 => simp at hi hj; subst hi; subst hj; decide
  | i + 4, _ => simp at hi
  | _, j + 4 => simp at hj
  | 0, 0 | 1, 0 | 1, 1 | 2, 0 | 2, 1 | 2, 2 | 3, 0 | 3, 1 | 3, 2 | 3, 3 => omega

end Sprawling.Serving.Placement

/-! D41 热线程设软的理想处理器，好核在先、最空的先给、坐下不换；一步计算不跨线程；硬亲和只作对照臂

**决定**（AF1 的 (b) 与 (c)，D88 第 3 条）：

- **(b) 理想处理器，三个平台。**
  - Windows：热线程起动时向放置表要座位，再调 `thread_priority::windows::set_current_thread_ideal_processor`（`thread-priority` 3.1.1，对外是安全接口，内部是 `SetThreadIdealProcessor`；第一档），退出时交还座位。处理器表的等级来自 `GetSystemCpuSetInformation` 的 `EfficiencyClass`（数越大越快）、`CoreIndex` 与 `LogicalProcessorIndex`：`std`、`thread-priority` 3.1.1、`win32job` 2.0.3 都不给这一项，所以它走第二档——Zig 叶子，一次调用把记录填进调用方给的缓冲区，边界就是 `(ptr, len)`。等级：最快一档的每个物理核的第一个逻辑处理器为 0，其余档的每个物理核的第一个逻辑处理器为 1，所有超线程兄弟为 2。叶子缺席或报错时退路是：`std::thread::available_parallelism` 个处理器全是 0 级，放置表只做「最空的先给」的分散，不做好核在先——P 与 E 由 Windows 的混合调度按 D40 的 QoS 去分；doctor 说出用的是哪一种处理器表。理想处理器是线程所在处理器组里的下标，所以逻辑处理器多于 64 个的机器只用第 0 组，并照实说。
  - macOS：没有理想处理器的接口（`THREAD_AFFINITY_POLICY` 在 Apple 芯片上不受支持），(b) 不可用；性能核由 D40 的线程 QoS 争取，放置表在 macOS 上不建。
  - Linux：没有软的理想处理器调用，`sched_setaffinity` 是硬亲和（下面的对照臂）；(b) 不可用。内核调度器在 Intel 混合架构（ITMT）与 ARM（EAS）上本来就把忙线程放到大核上；放置表在 Linux 上不建，doctor 照实说。
- **(c) 一步计算从头到尾在一条线程上。** 一步就是 §8-93 的一轮：热线程从一次醒来到下一次阻塞做完的一件事（折叠一条记录、执行一次工具调用、落一次 relay）。一步之内不把工作交给另一条线程：不经 channel 转手、不 spawn 后再 join、async 任务不在一步中途转去阻塞池。lane 本来各是一条 OS 线程；tokio worker 的一步是一次任务轮询，tokio 的偷任务只发生在两次轮询之间。这是代码的写法，不要平台接口，三个平台同一条。它守住了没有，由下面四臂对照的「中途换核次数」读出。
- **硬亲和只作对照臂。** Windows：harness 进程经 `win32job` 2.0.3 的 `Job::assign_current_process` 装进一只 job，再以 `ExtendedLimitInfo::limit_affinity` 设成 P 核的掩码；每个 run 的 job 设成其余处理器的掩码（同一 crate 的安全接口，第一档）。Linux：整个二进制在 `taskset -c` 下起动，子进程同样包一层 `taskset`（外部命令，第一档；`sched_setaffinity` 本身要 `unsafe`）。macOS：没有硬亲和，这一臂不跑，照实说。

**被否**：①硬亲和作默认（每个 session 一个核）——核忙时宁可排队也不换核，正与 D88 第 3 条「算不过来就立即下一个核」相反；②不设理想处理器、全交给调度器——Windows 按进程从一个随机起点轮流给理想处理器，不分 P 与 E，热线程可能被首选到 E 核上（这是推断，由四臂对照的「不做」臂读出）；③按负载随时改理想处理器——活着的线程偏好一变，就是「计算中途跳来跳去」，模型的「坐下不换」排除了它。

**重开参数**：四臂对照里 (a)+(b)+(c) 臂的工具调用与 relay 等待 p99 不优于「不做」臂（在噪声以内）——那就删掉 (b)，只留 (a) 与 (c)；或者硬亲和臂的 p99 与 p999 都明显更好。
-/

/-! ## 四臂对照（AF1 的完成条件，测量计划）

测量归波后的 mid 读数（Roadmap §0 第 7 条），这里只写计划，W6 的实现照它留出开关与采样点。

- **开关是一个配置值**：人的配置文件 `[core]` 一节的 `placement`，与 `priority` 同处（`accounting::person`，`crates/accounting/Spec.lean` §8-8），取 `"none"`、`"soft"`、`"soft_shares"`、`"pinned"`，分别是下面四臂。读数出来之前的默认是 `"soft"`；读数定下默认之后只改这一个默认值。
- **四臂**：①不做——不关 EcoQoS、不设理想处理器、run 的 job 不设份额；②(a)+(b)+(c)；③再加 (d)，即 `crates/runtime/spec/Tools/Exec.lean` D29 的按 run CPU 权重与作业级内存上限；④硬亲和（D41 的对照臂）。
- **负载**：TP1 吞吐台的同一个 citysim 场景，并发 run 数 4 与 16，另在后台跑 N ∈ {0, 4, 16} 个 `cargo build`，每个都经一个 run 的 exec 起动（于是它们落在各自 run 的 job 里，与真实城一样）。每臂每格重复 5 次，报中位数，写明机器类别（核数与 P/E 之分）。
- **读数**：工具调用的 harness 开销（`tool_called` 到 `tool_result`）与 relay 往返，各自的 p50、p99、p999；每步计算中途换核的次数——在 M2 的阶段边界（醒来、工具执行前、工具执行后、再次阻塞）各采一次当前处理器号，一步里前后不同就计一次，按热线程的种类分开计。
- **处理器号怎么采**：Windows 是 `GetCurrentProcessorNumberEx`，没有安全接口，放进 D41 的同一个 Zig 叶子；Linux 读 `/proc/thread-self/stat` 的第 39 个字段（标准库读文件，第一档；`sched_getcpu` 要 `unsafe`），只在测量构建里读，因为每次一个系统调用；macOS 没有读当前处理器的接口，只报延迟，换核次数写「不可测」。
- **按读数定默认**：p99 最低、且 p999 不比「不做」差的一臂；相差在噪声以内时取机制最少的一臂。读数与选择写进 `tools/xtask/budgets.toml` 的新行，由做测量的那一道写。
-/
