-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::cas::ranges

规定 `cas::ranges`（`crates/storage/src/cas/ranges.rs`）。Locator 的范围文法，在对象上读出而不整读对象。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-23 `storage::cas::ranges`：Locator 范围文法住一处（形状 4 适配器）

`get_range` 的文法——`B` 0 起闭区间、`L` 1 起闭区间、行间 `\n` 保留、末行终止符不返回、越界拒不夹取——是一套读法，不是 CAS 的存取。它因此住 `crates/storage/src/cas/ranges.rs`，`Cas::get_range` 只解析对象路径、判定存在，再把 `&dyn Vfs` 与路径交给 `of_object`。

**块长 64 KiB 是本模块的内部事务**（`SCAN_CHUNK_BYTES`）：`L` 式要知道第几行从哪开始，只能从对象开头扫换行，于是代价与**答案之前**的字节同阶，而与对象大小无关——一份 200 MB 的 offload 取第二行，读的是 64 KiB。`B` 式一次定位读即可，不必扫。

**这套文法没有第二个家**：`Cas::get` 仍是唯一会复算 BLAKE3 的读法，`ranges` 一次也不哈希。两条断言守着这件事，都在 `crates/storage/src/cas.rs` 的测试里——`byte_and_line_ranges_follow_locator_semantics` 守文法，`a_range_read_lifts_the_range_rather_than_the_object` 守代价（用 `FaultFs::bytes_read()` 数字节：一个五字节范围移动的字节数必须以十计，而不是以对象长度计）。
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
