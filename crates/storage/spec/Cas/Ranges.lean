-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::cas::ranges

规定 `cas::ranges`（`crates/storage/src/` 下同名的文件）。Locator 的范围文法，在对象上读出而不整读对象。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
-/

/-!
## 模型：`B` 式范围只答名下的字节，越界即拒，不夹取

`B` 式是 0 起的闭区间 `[from, to]`，`Range` 的构造已保 `from ≤ to`。规格是 `byteRange`：区间整个落在对象之内就答那 `to - from + 1` 个字节，否则拒（`RangeOutOfBounds`）。实现是 `byRead`：一次 `Vfs::read_at` 定位读 `to - from + 1` 字节，短答即越界——`read_at` 只报事实（短答表示文件在那里结束），越界由提出范围的这个模块判（§8-15）。

`a_short_answer_is_out_of_bounds` 陈述两者对每个对象、每个合法区间相同，所以「只读要答的那一段」没有改变答案；`never_clamps` 陈述答出来的恰是名下的字节数。`L` 式（1 起的闭区间行号，按 64 KiB 块扫换行）由 `cas::ranges` 的测试守住，不在模型里。
-/

namespace Storage.Cas.Ranges

/-- `Vfs::read_at`：从 `offset` 起至多 `len` 字节，文件到头就短答。 -/
def readAt (object : List Nat) (offset len : Nat) : List Nat :=
  (object.drop offset).take len

/-- 规格：`[fro, to]` 整个落在对象之内就答那一段，否则拒。 -/
def byteRange (object : List Nat) (fro to : Nat) : Option (List Nat) :=
  if to < object.length then some ((object.drop fro).take (to - fro + 1)) else none

/-- 实现：一次定位读，短答即越界。 -/
def byRead (object : List Nat) (fro to : Nat) : Option (List Nat) :=
  if (readAt object fro (to - fro + 1)).length = to - fro + 1 then
    some (readAt object fro (to - fro + 1))
  else none

theorem a_short_answer_is_out_of_bounds (object : List Nat) (fro to : Nat) (ordered : fro ≤ to) :
    byRead object fro to = byteRange object fro to := by
  unfold byRead byteRange readAt
  simp only [List.length_take, List.length_drop]
  by_cases h : to < object.length
  · have : min (to - fro + 1) (object.length - fro) = to - fro + 1 := by omega
    simp [this, h]
  · have : min (to - fro + 1) (object.length - fro) ≠ to - fro + 1 := by omega
    simp [this, h]

theorem never_clamps {object : List Nat} {fro to : Nat} {answer : List Nat} (ordered : fro ≤ to)
    (h : byteRange object fro to = some answer) : answer.length = to - fro + 1 := by
  unfold byteRange at h
  split at h
  · rename_i inside
    simp only [Option.some.injEq] at h
    subst h
    simp only [List.length_take, List.length_drop]
    omega
  · simp at h

/-- 恰触界合法（§11）：末字节是 `len - 1`。 -/
example : byteRange [7, 8, 9] 1 2 = some [8, 9] := by decide
/-- 越过末尾一个字节即拒。 -/
example : byteRange [7, 8, 9] 1 3 = none := by decide

end Storage.Cas.Ranges
