-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.storage.spec.Jsonl.Verify

/-!
# 预分配的段：段尾的零字节不是撕裂

规定段文件预分配（`File::set_len` 把段先撑到一个容量）之后，`open` 的尾段扫描（`crates/storage/src/jsonl/open.rs`）怎样读段尾（`crates/storage/Spec.lean` §8-1，屏障的各平台臂见 `crates/storage/spec/Jsonl/Barrier.lean` storage D24）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。今天的代码不预分配；本文件是 TF2 的那一臂被选中之前必须守住的不变式。

预分配之后，最后一条记录之后不再是文件末尾，而是一串零字节。一段于是是一列槽：一个槽要么是今天的一项 `Raw`（一行，或读不出信封的字节），要么是零。读者先剥掉段尾连续的零，再把剩下的交给今天的 `scan`；剥不掉的零（后面还跟着非零字节）读作 `notALine`，与撕裂同一种处置。

三条性质：
* **段尾补零不改变扫描的结论**：同一段，有没有、补多少段尾零，`scan` 给出同一个结局，找到同一条最后的有效记录（`trailing_zeros_change_no_scan`）；
* **撕裂照样被截**：最后一项是撕裂、其后只有零时，结局仍是截在撕裂之前（`a_tear_before_zeros_is_still_truncated`）；
* **夹在中间的零是撕裂，不是段尾**：零之后还有带信封的行时，`open` 拒开而不截，与今天同一处的撕裂相同（`zeros_before_a_line_are_not_the_end`）。

写者的位置因此是剥零之后的长度，而不是文件长度；把位置取成文件长度，下一条记录就写在零之后，而读者在第一串零处就停下。
-/

namespace Storage.Jsonl.Preallocate

open Storage.Jsonl.Verify

/-- 预分配段里的一个槽。 -/
inductive Slot where
  | raw (r : Raw)
  | zero
  deriving DecidableEq, Repr

def Slot.isZero : Slot → Bool
  | .zero => true
  | .raw _ => false

/-- 不在段尾的零读不出信封。 -/
def Slot.toRaw : Slot → Raw
  | .raw r => r
  | .zero => .notALine

/-- 剥掉段尾连续的零。 -/
def stripTail (slots : List Slot) : List Slot :=
  (slots.reverse.dropWhile Slot.isZero).reverse

/-- 预分配之后的尾段扫描。 -/
def scanSlots (hash : String → String) (s : LineCheck) (slots : List Slot) : Scan :=
  scan hash s ((stripTail slots).map Slot.toRaw)

theorem dropWhile_zeros (k : Nat) (rest : List Slot) :
    (List.replicate k Slot.zero ++ rest).dropWhile Slot.isZero = rest.dropWhile Slot.isZero := by
  induction k with
  | zero => simp
  | succ n ih => simp [List.replicate_succ, List.dropWhile_cons, Slot.isZero, ih]

theorem dropWhile_raws (seg : List Raw) :
    (seg.map Slot.raw).dropWhile Slot.isZero = seg.map Slot.raw := by
  cases seg with
  | nil => simp
  | cons r rest => simp [Slot.isZero]

theorem stripTail_of_zeros (seg : List Raw) (k : Nat) :
    stripTail (seg.map Slot.raw ++ List.replicate k Slot.zero) = seg.map Slot.raw := by
  simp only [stripTail, List.reverse_append, List.reverse_replicate, dropWhile_zeros,
    ← List.map_reverse, dropWhile_raws, List.reverse_reverse]

theorem toRaw_raws (seg : List Raw) : (seg.map Slot.raw).map Slot.toRaw = seg := by
  induction seg with
  | nil => rfl
  | cons r rest ih => simp [Slot.toRaw, ih]

/-- **段尾补零不改变扫描的结论**：任意一段、任意多的段尾零，结局与不补零时逐字相同。 -/
theorem trailing_zeros_change_no_scan (hash : String → String) (s : LineCheck) (seg : List Raw)
    (k : Nat) :
    scanSlots hash s (seg.map Slot.raw ++ List.replicate k Slot.zero) = scan hash s seg := by
  simp only [scanSlots, stripTail_of_zeros, toRaw_raws]

/-- **撕裂照样被截**：一段已收下的行之后是撕裂，再之后只有零，结局是截在撕裂之前，保留的正是那些行。 -/
theorem a_tear_before_zeros_is_still_truncated (hash : String → String) (s s' : LineCheck)
    (good : List Raw) (k : Nat) (walked : walk hash s good = .ok s') :
    scanSlots hash s ((good ++ [Raw.notALine]).map Slot.raw ++ List.replicate k Slot.zero)
      = .truncated good.length s' := by
  rw [trailing_zeros_change_no_scan]
  induction good generalizing s with
  | nil =>
    simp only [walk, Except.ok.injEq] at walked
    subst walked
    simp [scan, advance, dispose]
  | cons r rest ih =>
    simp only [walk] at walked
    split at walked
    · simp at walked
    · rename_i c t ht
      simp only [List.cons_append, scan, ht, List.length_cons]
      rw [ih t walked]
      rfl

theorem stripTail_keeps_a_raw_end (pre : List Slot) (r : Raw) :
    stripTail (pre ++ [.raw r]) = pre ++ [.raw r] := by
  simp [stripTail, Slot.isZero]

/-- **夹在中间的零是撕裂，不是段尾**：零之后还有带信封的行时，读者把那串零读作读不出信封的字节，而不是段的终点。 -/
theorem zeros_before_a_line_are_not_the_end (hash : String → String) (s : LineCheck)
    (pre : List Slot) (k : Nat) (e : Envelope) :
    scanSlots hash s (pre ++ List.replicate (k + 1) .zero ++ [.raw (.line e)])
      = scan hash s (pre.map Slot.toRaw ++ List.replicate (k + 1) Raw.notALine ++ [.line e]) := by
  simp only [scanSlots]
  rw [stripTail_keeps_a_raw_end]
  simp [Slot.toRaw]

/-- 撕裂之后跟着零、再跟着一条带信封的行：`open` 拒开而不截。 -/
example :
    scanSlots id (LineCheck.at_genesis "g")
      [.raw (.line ⟨"a", .Current, 0, "g", .known true⟩), .zero,
       .raw (.line ⟨"c", .Current, 2, "b", .known true⟩)] = .refused .RefuseEnvelope := by
  decide

end Storage.Jsonl.Preallocate
