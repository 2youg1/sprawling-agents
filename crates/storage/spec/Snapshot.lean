-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 快照加上它之后的尾部，就是全量折叠

`storage::snapshot` 保存一次折叠在 Ledger 某一行之后的状态，连同那一行的 seq 与它的链哈希。开城时找到快照，只核对那一行，再只折叠它之后的行。这样开城靠得住，全凭一条性质：从某个切点上折叠已到达的状态出发，走完切点之后的行，到达的状态与在整本账本上折叠到达的状态相同。

**本模块对每一种折叠、每一个切点陈述这条性质。** 折叠是一个不透明的步进函数，作用在不透明的行上，所以产品折叠的任何视图都不会与它漂移：`Views`、`Standing` 以及链状态 `storage::LineCheck` 本身各是它的一个实例。Rust 一侧由 `proptest` 在随机账本与随机切点上守住同一性质（`storage::snapshot` 的测试），那里的折叠就是链检查：从快照恢复的一遍走，接受的行与终态都必须和从创世走的一遍相同。

**行哈希买到什么、买不到什么。** 快照只经它最后一行的哈希指明自己切自哪本账本。当这个哈希与盘上同一 seq 的那一行相符，并且摘要函数在被覆盖的行上是单射（这条假设归 `Kernel.Ledger`，见 `crates/kernel/spec/Ledger.lean`），快照所折的行就是盘上账本的前缀；`resumeIsWhole` 就陈述在这个前缀事实之上，所以它根本不需要摘要函数。
-/

namespace Storage.Snapshot

/-- 从 `init` 出发折叠完 `lines` 之后的状态。 -/
def fold {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) : σ :=
  lines.foldl step init

/-- 切在 `k` 的快照：折叠完前 `k` 行之后的状态。 -/
def cut {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) (k : Nat) : σ :=
  fold step init (lines.take k)

/-- 从快照的状态出发折叠切点之后的行，结果就是全量折叠。对每一个切点都成立，包括 `0`（没有快照）和越过末尾的切点（整本账本的快照，尾部为空）。 -/
theorem snapshotPlusTailIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (lines : List α) (k : Nat) :
    fold step (cut step init lines k) (lines.drop k) = fold step init lines := by
  unfold cut fold
  rw [← List.foldl_append, List.take_append_drop]

/-- 在 `pre` 上取的快照，对以 `pre` 为前缀的任何账本都能正确恢复：快照写下之后账本变长了，尾部就是它长出来的部分。 -/
theorem resumeIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (pre lines : List α) (h : pre <+: lines) :
    fold step (fold step init pre) (lines.drop pre.length) = fold step init lines := by
  obtain ⟨tail, rfl⟩ := h
  unfold fold
  rw [List.drop_left, List.foldl_append]

end Storage.Snapshot
