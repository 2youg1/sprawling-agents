-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 还在跑的命令写出的字节，给后来打开页面的会话留一段

规定 `crates/sprawling/src/serving/output_ring.rs` 的 `OutputRing::keep` 对一个 run 的那一段做的事（`bin::serving::output_ring`，形状：value；sprawling-SPEC.md 8-115）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一个 run 留着的是到达次序的一列块。模型里块的类型是参数，`size` 是它的字节数；Rust 另存一个 `bytes` 字段免得每次求和，它恒等于各块之和，这里直接求和。按 run 分开的那张表与 `settle`（`tool_result` 落账即清空这个 run）是一次 `BTreeMap::remove`，不另建模。

三条性质：

* 最新的一块总留着，哪怕它自己超过上界；
* 留下的总字节不超过上界，或者只剩最新的那一块——每个 run 的内存有界；
* 只从最旧的整块丢起：留下的是原来那一列加上新块之后的一个后缀，次序不变。
-/

namespace Sprawling.Serving.OutputRing

/-- 每个 run 留多少字节（Rust：`KEPT_BYTES_PER_RUN`，64 KiB，等于 runtime 一秒最多读出的字节）。 -/
def keptBytesPerRun : Nat := 64 * 1024

variable {α : Type} (size : α → Nat) (bound : Nat)

def total (pieces : List α) : Nat := (pieces.map size).sum

/-- 从最旧的整块丢起，直到不超过上界，或只剩一块。 -/
def trim : List α → List α
  | [] => []
  | oldest :: rest =>
    if total size (oldest :: rest) > bound ∧ rest ≠ [] then trim rest else oldest :: rest

/-- `keep`：接在最后，再修剪。 -/
def keep (pieces : List α) (piece : α) : List α :=
  trim size bound (pieces ++ [piece])

theorem trim_suffix (pieces : List α) : (trim size bound pieces) <:+ pieces := by
  induction pieces with
  | nil => simp [trim]
  | cons x rest ih =>
    unfold trim
    split
    · exact List.IsSuffix.trans ih (List.suffix_cons x rest)
    · exact List.suffix_refl _

theorem trim_bounded (pieces : List α) :
    total size (trim size bound pieces) ≤ bound ∨ (trim size bound pieces).length ≤ 1 := by
  induction pieces with
  | nil => simp [trim]
  | cons x rest ih =>
    unfold trim
    split
    · exact ih
    · rename_i h
      by_cases hr : rest = []
      · subst hr; right; simp
      · left
        have : ¬ total size (x :: rest) > bound := fun hg => h ⟨hg, hr⟩
        omega

theorem trim_keeps_last (pieces : List α) (h : pieces ≠ []) :
    (trim size bound pieces).getLast? = pieces.getLast? := by
  induction pieces with
  | nil => contradiction
  | cons x rest ih =>
    unfold trim
    split
    · rename_i hc
      rw [ih hc.2]
      cases rest with
      | nil => exact absurd rfl hc.2
      | cons y ys => simp [List.getLast?_cons_cons]
    · rfl

/-- 最新的一块总留着。 -/
theorem newest_stays (pieces : List α) (piece : α) :
    (keep size bound pieces piece).getLast? = some piece := by
  unfold keep
  rw [trim_keeps_last size bound _ (by simp)]
  simp

/-- 每个 run 的内存有界：不超过上界，或只剩一块。 -/
theorem kept_is_bounded (pieces : List α) (piece : α) :
    total size (keep size bound pieces piece) ≤ bound ∨
      (keep size bound pieces piece).length ≤ 1 :=
  trim_bounded size bound _

/-- 只丢最旧的整块，次序不变。 -/
theorem drops_only_the_oldest (pieces : List α) (piece : α) :
    keep size bound pieces piece <:+ pieces ++ [piece] :=
  trim_suffix size bound _

end Sprawling.Serving.OutputRing
