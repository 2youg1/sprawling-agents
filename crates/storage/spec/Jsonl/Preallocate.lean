-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.storage.spec.Jsonl.Verify

/-!
# 预分配的段：段尾的零字节不是撕裂

规定段文件预分配（`File::set_len` 把段先撑到一个容量）之后，`open` 的尾段扫描（`crates/storage/src/jsonl/open.rs`）怎样读段尾（`crates/storage/Spec.lean` §8-1，屏障的各平台臂见 `crates/storage/spec/Jsonl/Barrier.lean` storage D24）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。默认的臂不预分配，但 `open` 的读者已经守着这三条性质（`crates/storage/src/jsonl/open/preallocated.rs` 在真实的段上逐条检查），所以预分配那一臂被选中时读路径不必再改。

预分配之后，最后一条记录之后不再是文件末尾，而是一串零字节。一段于是是一列槽：一个槽要么是今天的一项 `Raw`（一行，或读不出信封的字节），要么是零。读者先剥掉段尾连续的零，再把剩下的交给今天的 `scan`；剥不掉的零（后面还跟着非零字节）读作 `notALine`，与撕裂同一种处置。

三条性质：
* **段尾补零不改变扫描的结论**：同一段，有没有、补多少段尾零，`scan` 给出同一个结局，找到同一条最后的有效记录（`trailing_zeros_change_no_scan`）；
* **撕裂照样被截**：最后一项是撕裂、其后只有零时，结局仍是截在撕裂之前（`a_tear_before_zeros_is_still_truncated`）；
* **夹在中间的零是撕裂，不是段尾**：零之后还有带信封的行时，`open` 拒开而不截，与今天同一处的撕裂相同（`zeros_before_a_line_are_not_the_end`）。

写者的位置因此是剥零之后的长度，而不是文件长度；把位置取成文件长度，下一条记录就写在零之后，而读者在第一串零处就停下。下面两条接续的性质把这一句写成定理：
* **在剥零之后的位置写下，接续成立**：写者把位置取成剥零之后的长度（`writer_position_is_the_stripped_end`），在那里写下一条（覆盖段尾的零，或零已用完时接在末尾），读者扫出的结局与「这一段加这一条、没有零」逐字相同（`a_record_written_at_the_stripped_end_resumes_the_segment`）；
* **在文件长度处写下，记录落在零之后**：读者看到的是这一段、一串读不出信封的字节、再这一条，与把零读作撕裂相同（`a_record_written_at_the_file_length_sits_behind_the_zeros`），于是 `open` 拒开而不是接续。

这两条与上面三条合起来，就是预分配那一臂（storage D31）的读与接续的结论；`crates/storage/src/jsonl/open/preallocated.rs` 的 proptest `a_preallocated_writer_resumes_after_every_reopen_and_every_cut` 从它们导出：任意几波、任意容量，开了预分配的写者写、重开、再写，读回的恰是全部记录、链完整，段文件仍不短于容量。
-/

/-! D31 段的预分配是一个可选的臂：`SegmentPreallocation { Grow, ToRollSize }`，每一个本构建打开的账本都用常量 `jsonl::ledger::SEGMENT_PREALLOCATION` 给的那一臂，今天是 `Grow`，即预分配之前的行为。
**两臂。** `Grow`：段随每一波在文件末尾 `append` 而长，`open` 截去段尾的零。`ToRollSize`：新段建好、写下第一波之后，以 `Vfs::truncate`（`File::set_len`）把它设到滚动尺寸 `SEGMENT_ROLL_BYTES`；之后每一波以 `Vfs::write_at` 写在记录末尾（写者的位置，`writer_position_is_the_stripped_end`），`open` 没有撕裂时留着段尾的零，有撕裂时截到最后一条有效记录，与 `Grow` 相同。失败的一波照样由 `jsonl::unwind` 截回波前的长度，截掉的是预分配的零与撕裂的字节，之后的写越过文件尾即延长。
**为什么可能更快。** 文件长度不变时，屏障不必把长度这一项元数据一起落盘：Linux 的 `fdatasync` 只在长度变了时写 inode，Windows 的 `FlushFileBuffers` 与 macOS 的 `F_FULLFSYNC` 在长度变了时同样多写一次文件系统的元数据。三个平台的 `set_len` 都是延长文件的逻辑长度：Windows 是 `SetFileInformationByHandle(FileEndOfFileInfo)`，Linux 与 macOS 是 `ftruncate`；ext4 与 APFS 上那是一个稀疏的洞，块不在这时分配，所以省下的是长度的更新，不是块的分配（推断：依据 `fdatasync(2)` 的文档与三个平台的 `set_len` 实现，未在掉电下测过）。
**为什么默认仍是 `Grow`。** 选哪一臂要一份发布构建上、每个平台各一份的屏障读数，这一份读数还欠着（本版本不跑基准）；没有读数时保留已经在用的行为。另一个代价也要一起读：`index::ledger` 的刷新按段文件的长度判断有没有新记录，预分配的段从建起就是滚动尺寸，刷新于是每次都读一遍记录末尾到段尾的零（结果仍然对：它只把完整的行记为已读）。选 `ToRollSize` 之前，刷新要先学会在零处停。
**被否：用 `fallocate` 或 `F_PREALLOCATE` 真正分配块。** 标准库没有安全接口，要 `unsafe` 的 FFI 或新的依赖（AGENTS.md 平台调用的次序），而要读的问题是长度元数据的代价，不是块分配。**被否：让 `Grow` 也按位置写。** 那会改变默认臂的写法，而这一项的范围是加一个臂、默认不变。
**重开参数。** 发布构建上屏障的读数（每个平台、每一臂）显示 `ToRollSize` 在某个平台上明显更快，且 `index::ledger` 的刷新已经在零处停；那时改的只是 `SEGMENT_PREALLOCATION` 这一个值。
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

/-- 写者的位置：剥零之后的槽数。 -/
def writerPos (slots : List Slot) : Nat :=
  (stripTail slots).length

/-- 在位置 `pos` 写下一项：覆盖那里的槽，位置在段尾之外时接在末尾。 -/
def writeAt (slots : List Slot) (pos : Nat) (r : Raw) : List Slot :=
  slots.take pos ++ [Slot.raw r] ++ slots.drop (pos + 1)

theorem writer_position_is_the_stripped_end (seg : List Raw) (k : Nat) :
    writerPos (seg.map Slot.raw ++ List.replicate k Slot.zero) = seg.length := by
  simp [writerPos, stripTail_of_zeros]

/-- **在剥零之后的位置写下，接续成立**：任意一段、任意多的段尾零，在写者位置写下一项，读者的结局与这一段接上这一项、没有零时相同。 -/
theorem a_record_written_at_the_stripped_end_resumes_the_segment (hash : String → String)
    (s : LineCheck) (seg : List Raw) (k : Nat) (r : Raw) :
    scanSlots hash s
        (writeAt (seg.map Slot.raw ++ List.replicate k Slot.zero)
          (writerPos (seg.map Slot.raw ++ List.replicate k Slot.zero)) r)
      = scan hash s (seg ++ [r]) := by
  rw [writer_position_is_the_stripped_end]
  have hlen : (seg.map Slot.raw).length = seg.length := List.length_map _
  have htake : (seg.map Slot.raw ++ List.replicate k Slot.zero).take seg.length
      = seg.map Slot.raw := by
    rw [← hlen, List.take_left']
    rfl
  have hdrop : (seg.map Slot.raw ++ List.replicate k Slot.zero).drop (seg.length + 1)
      = List.replicate (k - 1) Slot.zero := by
    rw [← hlen, ← List.drop_drop, List.drop_left', List.drop_replicate]
    rfl
  simp only [writeAt, htake, hdrop]
  have hshape : seg.map Slot.raw ++ [Slot.raw r] ++ List.replicate (k - 1) Slot.zero
      = (seg ++ [r]).map Slot.raw ++ List.replicate (k - 1) Slot.zero := by
    simp
  rw [hshape, trailing_zeros_change_no_scan]

theorem writeAt_length (slots : List Slot) (r : Raw) :
    writeAt slots slots.length r = slots ++ [Slot.raw r] := by
  simp [writeAt]

/-- **在文件长度处写下，记录落在零之后**：段尾至少有一个零时，写在文件末尾的一条与写在撕裂之后的一条读法相同。 -/
theorem a_record_written_at_the_file_length_sits_behind_the_zeros (hash : String → String)
    (s : LineCheck) (seg : List Raw) (k : Nat) (e : Envelope) :
    scanSlots hash s
        (writeAt (seg.map Slot.raw ++ List.replicate (k + 1) Slot.zero)
          (seg.map Slot.raw ++ List.replicate (k + 1) Slot.zero).length (.line e))
      = scan hash s (seg ++ List.replicate (k + 1) Raw.notALine ++ [.line e]) := by
  rw [writeAt_length, zeros_before_a_line_are_not_the_end, toRaw_raws]

/-- 撕裂之后跟着零、再跟着一条带信封的行：`open` 拒开而不截。 -/
example :
    scanSlots id (LineCheck.at_genesis "g")
      [.raw (.line ⟨"a", .Current, 0, "g", .known true⟩), .zero,
       .raw (.line ⟨"c", .Current, 2, "b", .known true⟩)] = .refused .RefuseEnvelope := by
  decide

end Storage.Jsonl.Preallocate
