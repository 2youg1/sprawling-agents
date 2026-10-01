-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# `length` 的文件面：登记表只会变短

规定 `tools/xtask/src/length.rs` 的 `judge_file`（`tools/xtask/Spec.lean` §10 第 7 条）：
`tools/xtask/budgets.toml` 的 `[file_length]` 给一个预算，`[file_length.predating]` 把先于规则
存在的文件各钉在划线时的行数上。

参照定义只陈述一份文件在这条规则下的结论；行数怎样量（Rust 减去顶层 `#[cfg(test)]` 项、Zig 减去
`test` 声明、客户端只量文件面）不在这里，那一半在 `length` 的 Rust 里，由它自己的测试守着。
-/

namespace Xtask.Length

/-- 一份文件在文件面上的结论。 -/
inductive FileVerdict where
  /-- 在预算之内，且登记表没有它；或登记表有它、它仍超出预算而没有长过钉子。 -/
  | Kept
  /-- 登记表没有它，而它超出预算（`too_long`）。 -/
  | TooLong
  /-- 登记表钉着它，而它长过了钉子（`grew`）。 -/
  | Grew
  /-- 登记表钉着它，而它已回到预算之内，那一行须划掉（`no_longer_an_exception`）。 -/
  | NoLongerAnException
deriving DecidableEq, Repr

/-- 一份 `lines` 行的文件，预算 `limit`，登记表上的钉子 `pinned`（没有则 `none`）。 -/
def judge_file (limit lines : Nat) : Option Nat → FileVerdict
  | none => if lines > limit then .TooLong else .Kept
  | some pinned =>
    if lines > pinned then .Grew
    else if lines ≤ limit then .NoLongerAnException
    else .Kept

/-- 登记表没有的文件，过门恰是它在预算之内。 -/
theorem an_unpinned_file_passes_exactly_inside_the_budget (limit lines : Nat) :
    judge_file limit lines none = .Kept ↔ lines ≤ limit := by
  unfold judge_file
  by_cases h : lines > limit <;> simp [h] <;> omega

/-- 被钉住的文件过门时不长于它的钉子：欠债不会长大。 -/
theorem a_pinned_file_never_grows (limit lines pinned : Nat)
    (kept : judge_file limit lines (some pinned) = .Kept) : lines ≤ pinned := by
  unfold judge_file at kept
  by_cases h : lines > pinned
  · simp [h] at kept
  · omega

/-- 被钉住的文件过门时仍超出预算：一行回到预算之内的钉子留不下，所以登记表只会变短。 -/
theorem a_kept_pin_is_still_needed (limit lines pinned : Nat)
    (kept : judge_file limit lines (some pinned) = .Kept) : lines > limit := by
  unfold judge_file at kept
  by_cases g : lines > pinned
  · simp [g] at kept
  · by_cases h : lines ≤ limit
    · simp [g, h] at kept
    · omega

/-- 规则不是空真：预算之内的新文件、正好在钉子上而仍超出预算的旧文件都过门。 -/
theorem kept_files_exist :
    judge_file 400 120 none = .Kept ∧ judge_file 400 520 (some 520) = .Kept := by
  decide

end Xtask.Length
