-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 账本的回答与重开后磁盘上的内容

规定 `crates/storage/src/jsonl/barrier.rs`：`append_all` 在一波之前查询、在该波最后一次 sync 之后修补的状态 `Barrier`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪条性质」的权威（storage-SPEC.md 8-1）。

磁盘是一列格子：一格要么是一条带 seq 的完整记录，要么是一波写到一半中断时留下的字节。开账本时保留到第一个不是完整记录的格子为止，这就是尾部恢复。一个 handle 记着一个位置，以及它答过 `Ok` 的那些 seq。

一条性质：**handle 答过 `Ok` 的每个 seq，重开后的账本里都有**。它成立，是因为一个有一波失败过的 handle 拒绝此后的每一波；`withoutBarrier` 给出 handle 继续写时打破这条性质的轨迹。

这里一波就是一条记录。一波带几条记录，不改变撕裂相对于此前已答记录能落在哪里。
-/

namespace Storage.Jsonl.Barrier

/-- 段里一个位置上放着什么。 -/
inductive Cell where
  | record (seq : Nat)
  | torn
  deriving DecidableEq, Repr

def Cell.isRecord : Cell → Bool
  | .record _ => true
  | .torn => false

/-- 尾部恢复：保留到第一个不是记录的格子为止。 -/
def reopen (disk : List Cell) : List Cell :=
  disk.takeWhile Cell.isRecord

inductive Barrier where
  | whole
  | broken
  deriving DecidableEq, Repr

structure Ledger where
  disk : List Cell
  next : Nat
  barrier : Barrier
  claimed : List Nat

def Ledger.empty : Ledger := ⟨[], 0, .whole, []⟩

/-- 一波在设备上的结局。 -/
inductive Outcome where
  /-- 写完并 sync 过：这一波答 `Ok`。 -/
  | synced
  /-- 写到一半中断，留下不成记录的字节。 -/
  | tore
  /-- 记录已进文件，但 sync 失败。 -/
  | unsynced

/-- 守屏障的 handle 写一波。 -/
def Ledger.append (l : Ledger) : Outcome → Ledger
  | o => match l.barrier with
    | .broken => l
    | .whole => match o with
      | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
      | .tore => { l with disk := l.disk ++ [.torn], barrier := .broken }
      | .unsynced => { l with disk := l.disk ++ [.record l.next], barrier := .broken }

/-- 依次排好的记录 `0 .. n`：一块完好的磁盘就是这样。 -/
def records (n : Nat) : List Cell :=
  (List.range n).map Cell.record

/-- 每个可达的 handle 都守着两件事：`Barrier` 为 `whole` 时，磁盘恰好是位置之下的那些记录；每个答过的 seq 都在位置之下，并且重开后还在。 -/
def Ledger.Sound (l : Ledger) : Prop :=
  (l.barrier = .whole → l.disk = records l.next) ∧
  ∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk

theorem takeWhile_records_append (n : Nat) (rest : List Cell) :
    (records n ++ rest).takeWhile Cell.isRecord
      = records n ++ rest.takeWhile Cell.isRecord := by
  induction n generalizing rest with
  | zero => simp [records]
  | succ k ih =>
    have split : records (k + 1) = records k ++ [Cell.record k] := by
      simp [records, List.range_succ]
    rw [split, List.append_assoc, ih]
    simp [List.takeWhile_cons, Cell.isRecord]

theorem reopen_records (n : Nat) : reopen (records n) = records n := by
  have h := takeWhile_records_append n []
  simpa [reopen] using h

theorem mem_records {s n : Nat} : Cell.record s ∈ records n ↔ s < n := by
  simp [records]

theorem empty_sound : Ledger.empty.Sound := by
  refine ⟨fun _ => rfl, ?_⟩
  simp [Ledger.empty]

/-- 无论这一波结局如何，守得住的 handle 写完之后仍守得住。 -/
theorem append_sound (l : Ledger) (o : Outcome) (h : l.Sound) : (l.append o).Sound := by
  obtain ⟨shape, kept⟩ := h
  cases hb : l.barrier with
  | broken =>
    simp only [Ledger.append, hb]
    exact ⟨shape, kept⟩
  | whole =>
    have disk := shape hb
    have below : ∀ s ∈ l.claimed, s < l.next := by
      intro s hs
      have := kept s hs
      rw [disk, reopen_records] at this
      exact mem_records.mp this
    cases o with
    | synced =>
      have grown : l.disk ++ [Cell.record l.next] = records (l.next + 1) := by
        rw [disk]; simp [records, List.range_succ]
      refine ⟨fun _ => by simp [Ledger.append, hb, grown], ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [grown, reopen_records, mem_records]
      simp only [List.mem_append, List.mem_singleton] at hs
      rcases hs with hs | hs
      · exact Nat.lt_succ_of_lt (below s hs)
      · omega
    | tore =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [disk, reopen, takeWhile_records_append]
      simp [mem_records, below s hs]
    | unsynced =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_⟩
      intro s hs
      simp only [Ledger.append, hb] at hs ⊢
      rw [disk, reopen, takeWhile_records_append]
      simp [mem_records, below s hs]

/-- 从空账本起，任意一串波写完之后，答过的每个 seq 在重开后的磁盘上都还在。 -/
theorem answered_survives_reopen (waves : List Outcome) :
    (waves.foldl Ledger.append Ledger.empty).Sound := by
  suffices ∀ l : Ledger, l.Sound → (waves.foldl Ledger.append l).Sound from
    this _ empty_sound
  induction waves with
  | nil => exact fun _ h => h
  | cons o rest ih => exact fun l h => ih _ (append_sound l o h)

/-- 不守屏障的 handle：一波失败后位置不动，下一波照写。 -/
def Ledger.appendAnyway (l : Ledger) : Outcome → Ledger
  | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
  | .tore => { l with disk := l.disk ++ [.torn] }
  | .unsynced => { l with disk := l.disk ++ [.record l.next] }

/-- `Barrier` 要挡住的反例：一波撕裂，下一波答了 `Ok`，尾部恢复却把它截掉。 -/
theorem withoutBarrier :
    let l := [Outcome.synced, .tore, .synced].foldl Ledger.appendAnyway Ledger.empty
    ¬ ∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk := by
  decide

/-! ## 从已验证前缀起重开

一段的前 `L` 个格子已由记录证明是完整记录（storage-SPEC.md 8-30）时，尾部恢复不必从第 0 格看起：前缀原样保留，只看 `L` 之后。记录只写在完整记录组成的前缀上，这是 `verified` 这个前提；Rust 一侧由 `storage::verified_prefix` 只为核对过的完整行写记录守住它。 -/

/-- 前 `L` 格都是记录时，重开等于前缀加上从 `L` 起的重开。 -/
theorem reopenFromVerifiedPrefix (disk : List Cell) (L : Nat)
    (verified : (disk.take L).all Cell.isRecord = true) :
    reopen disk = disk.take L ++ reopen (disk.drop L) := by
  unfold reopen
  induction L generalizing disk with
  | zero => simp
  | succ k ih =>
    cases disk with
    | nil => simp
    | cons c rest =>
      simp only [List.take_succ_cons, List.all_cons, Bool.and_eq_true] at verified
      obtain ⟨hc, hrest⟩ := verified
      simp only [List.takeWhile_cons, hc, if_true, List.take_succ_cons, List.drop_succ_cons,
        List.cons_append]
      rw [ih rest hrest]

/-- 于是守屏障的 handle 答过的每个 seq，从任何已验证前缀起重开之后也都还在。 -/
theorem answered_survives_reopen_from_a_verified_prefix (waves : List Outcome) (L : Nat)
    (verified :
      ((waves.foldl Ledger.append Ledger.empty).disk.take L).all Cell.isRecord = true) :
    ∀ s ∈ (waves.foldl Ledger.append Ledger.empty).claimed,
      Cell.record s ∈ (waves.foldl Ledger.append Ledger.empty).disk.take L
        ++ reopen ((waves.foldl Ledger.append Ledger.empty).disk.drop L) := by
  intro s hs
  rw [← reopenFromVerifiedPrefix _ L verified]
  exact (answered_survives_reopen waves).2 s hs

end Storage.Jsonl.Barrier
