-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 性能监视器的历史：有人看才采样，每项 300 点

规定 `crates/sprawling/src/monitor.rs` 的 `Monitor::tick`（`bin::monitor`，形状：状态机；sprawling-SPEC.md 8-94）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一拍之内只发生一件事：看最多的那个人要什么，就读什么一次，放进历史；没人看就不读，并把历史释放。读数 `Sample` 的类型是参数（它定义在 `wire::frames::monitor`，`crates/wire/Spec.lean` §8-47），读取函数也是参数，所以「没人看不读」在模型里是「读取函数没有被调用」：`tick` 返回它调用读取函数时传的那一类，没调用就是 `none`。

四条性质：没人看时不读、历史为空（D1）；历史不超过 `capacity`；满了丢最旧的、从旧到新；有人看整页就读整页，只有看摘要的人才只读摘要。
-/

namespace Sprawling.Monitor

/-- 看的人看什么（Rust：`wire::frames::monitor::Watched`）。 -/
inductive Watched where
  | everything
  | summary
  deriving Repr, DecidableEq

/-- 每秒一点，5 分钟（Rust：`CAPACITY`）。 -/
def capacity : Nat := 300

/-- 监视器的状态：两类看的人各自的计数，与历史（从旧到新）。 -/
structure Monitor (Sample : Type) where
  pageWatchers : Nat
  summaryWatchers : Nat
  history : List Sample

variable {Sample : Type}

/-- 看最多的那个人要什么（Rust：`Monitor::watched`）。 -/
def Monitor.watched (m : Monitor Sample) : Option Watched :=
  if m.pageWatchers > 0 then some .everything
  else if m.summaryWatchers > 0 then some .summary
  else none

/-- 满了丢最旧的，再把新的放在最后。 -/
def keep (history : List Sample) (s : Sample) : List Sample :=
  (if history.length = capacity then history.drop 1 else history) ++ [s]

/-- `Monitor::tick`：返回这一拍之后的监视器，与读取函数被调用时收到的那一类（没调用为 `none`）。 -/
def Monitor.tick (m : Monitor Sample) (read : Watched → Sample) :
    Monitor Sample × Option Watched :=
  match m.watched with
  | none => ({ m with history := [] }, none)
  | some w => ({ m with history := keep m.history (read w) }, some w)

/-- D1：没人看时不读计数器，也不留历史。 -/
theorem unwatched_reads_nothing (m : Monitor Sample) (read : Watched → Sample)
    (h : m.pageWatchers = 0 ∧ m.summaryWatchers = 0) :
    (m.tick read).2 = none ∧ (m.tick read).1.history = [] := by
  simp [Monitor.tick, Monitor.watched, h.1, h.2]

/-- 有人看整页就读整页。 -/
theorem a_page_watcher_reads_everything (m : Monitor Sample) (read : Watched → Sample)
    (h : m.pageWatchers > 0) : (m.tick read).2 = some .everything := by
  simp [Monitor.tick, Monitor.watched, h]

/-- 只有看摘要的人时只读摘要。 -/
theorem only_summary_watchers_read_the_summary (m : Monitor Sample) (read : Watched → Sample)
    (hp : m.pageWatchers = 0) (hs : m.summaryWatchers > 0) :
    (m.tick read).2 = some .summary := by
  simp [Monitor.tick, Monitor.watched, hp, hs]

theorem keep_is_bounded (history : List Sample) (s : Sample)
    (h : history.length ≤ capacity) : (keep history s).length ≤ capacity := by
  unfold keep
  split <;> simp [capacity] at * <;> omega

/-- 历史不超过 `capacity`：每一拍都保住这条。 -/
theorem tick_is_bounded (m : Monitor Sample) (read : Watched → Sample)
    (h : m.history.length ≤ capacity) : (m.tick read).1.history.length ≤ capacity := by
  unfold Monitor.tick
  split
  · simp
  · exact keep_is_bounded _ _ h

/-- 新读数总在最后：历史从旧到新。 -/
theorem newest_is_last (history : List Sample) (s : Sample) :
    (keep history s).getLast? = some s := by
  unfold keep
  split <;> simp

/-- 没满时什么也不丢。 -/
theorem nothing_dropped_below_capacity (history : List Sample) (s : Sample)
    (h : history.length < capacity) : keep history s = history ++ [s] := by
  have : history.length ≠ capacity := by omega
  simp [keep, this]

end Sprawling.Monitor
