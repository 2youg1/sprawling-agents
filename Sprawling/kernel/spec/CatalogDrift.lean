-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::event::record::CatalogDrift

规定 skill、connector server 与 connector tool 的 pin 比较和 `digest_invalidated` 去重入账。Rust 类型在 `crates/kernel/src/event/record/catalog.rs`，生产去重表在 `accounting::worker::workbench`。本模型量化任意观察轨迹：相同版本不产漂移，同一 `(building, subject, pinned, found)` 至多入账一次。

D118 漂移事件只记 pin 与后来发现的 digest，不记服务器定义或工具正文。工具定义 digest 同时是其规范 JSON 的 CAS 地址，skill digest 是其 canonical manifest 的 CAS 地址，因此页面可以离线比较两个版本；服务器命令、URL、环境引用与 header 引用留在 CONFIG.toml，不进 Ledger。被否：事件携带完整定义，会把主机事实与未批准外部文本复制进历史。重开参数：某类 pin 不能用同一 digest 定位其已批准字节时，先为该域定义独立的验证边界。
-/

namespace Kernel.CatalogDrift

/-- 一个 catalog 主体；模型只要求它能判等。 -/
abbrev Subject := Nat

/-- 一个 digest；模型只要求它能判等。 -/
abbrev Digest := Nat

/-- 一栋楼；模型只要求它能判等。 -/
abbrev Building := Nat

/-- 一次观察留下的差异。`none` 在 pinned 侧表示新工具，在 found 侧表示已消失。 -/
structure Drift where
  subject : Subject
  pinned : Option Digest
  found : Option Digest
  deriving DecidableEq, Repr

/-- 一条漂移归属哪栋楼。事件信封的 `addr` 携带它，载荷不重复。 -/
structure Located where
  building : Building
  drift : Drift
  deriving DecidableEq, Repr

/-- pin 与发现不同、且不是两侧都缺席时才有漂移。 -/
def between (subject : Subject) (pinned found : Option Digest) : Option Drift :=
  if pinned = found then none
  else match pinned, found with
    | none, none => none
    | _, _ => some ⟨subject, pinned, found⟩

theorem no_drift_for_the_same_version (subject : Subject) (version : Option Digest) :
    between subject version version = none := by
  simp [between]

theorem a_new_item_and_a_missing_item_do_not_form_a_drift (subject : Subject) :
    between subject none none = none := by
  simp [between]

theorem between_covers_the_two_versions (subject : Subject) (pinned found : Option Digest)
    (drift : Drift) (h : between subject pinned found = some drift) :
    drift.subject = subject ∧ drift.pinned = pinned ∧ drift.found = found ∧ pinned ≠ found := by
  unfold between at h
  split at h
  · contradiction
  · split at h
    · contradiction
    · cases h
      exact ⟨rfl, rfl, rfl, by assumption⟩

/-- 去重表已经记下的键不再追加；新键追加一次。 -/
def record (book : List Located) (next : Located) : List Located :=
  if next ∈ book then book else next :: book

theorem record_keeps_no_duplicates (book : List Located) (next : Located)
    (h : book.Nodup) : (record book next).Nodup := by
  unfold record
  by_cases seen : next ∈ book
  · simpa [seen] using h
  · simp [seen, List.nodup_cons, h]

theorem foldl_keeps_no_duplicates (observations : List Located) (book : List Located)
    (h : book.Nodup) : (observations.foldl record book).Nodup := by
  induction observations generalizing book with
  | nil => exact h
  | cons next rest ih =>
    rw [List.foldl_cons]
    exact ih _ (record_keeps_no_duplicates book next h)

theorem every_trace_has_no_duplicate_drift (observations : List Located) :
    (observations.foldl record []).Nodup := by
  exact foldl_keeps_no_duplicates observations [] List.nodup_nil

theorem an_existing_drift_is_not_recorded_again (book : List Located) (next : Located)
    (h : next ∈ book) : record book next = book := by
  simp [record, h]

end Kernel.CatalogDrift
