-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::chain_audit

规定 `chain_audit`、`chain_audit::wave`、`verified_prefix`（`crates/storage/src/` 下同名的文件）。从创世逐段证明整条链、已验证前缀按摘要复用、一波至多八段并行读，以及断链时让写者停下的停机值。已验证前缀与按波读段的性质住 `crates/storage/spec/Snapshot.lean`，本分部引用而不复写。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
-/

/-!
## 模型：停机值只有一个判定，判定之前按它的造法放行或拒绝

证明一条链之后的判定（`Whole` 或 `Broken`）由 `ChainHalt` 在写者与证明线程之间传递。性质：

* 第一个判定有效，之后的 `prove`／`trip` 都不改变它（`the_first_verdict_stands`），所以先证明完好、后又被叫停的停机值仍是完好，先跳闸的仍是断链；复位只能是一个新的停机值。
* `append_all` 在组帧之前问 `admit`：已跳闸答 `ChainHalted`（带第一次跳闸的原因），`awaiting_proof()` 造的停机值在判定之前答 `Unproven`，`default()` 造的在判定之前放行，判定为完好之后放行（`admission_follows_the_verdict`）。
* 一次证明的波数只取决于段数：段数除以 `PROOF_WAVE` 向上取整（`waves_depend_only_on_the_segments`），所以 `ProofCount.waves` 不随核数或线程完成的次序变（§8-37）。

已验证前缀的记录怎样代替逐行核对、按波先算摘要为什么不改变判定，由 `crates/storage/spec/Snapshot.lean` 的 `cachedVerifyIsStrict`、`wavesAreCached`、`wavesAreStrict` 证明；从记录覆盖的前缀起尾部恢复与从头恢复相同，由 `crates/storage/spec/Jsonl/Barrier.lean` 的 `reopenFromVerifiedPrefix` 证明。本分部引用它们，不复写。
-/

namespace Storage.ChainAudit

/-- 证明给出的判定；`Broken` 带着原因（Rust 里是一个 `AxError`）。 -/
inductive Verdict (Reason : Type) where
  | Whole
  | Broken (reason : Reason)
  deriving DecidableEq, Repr

/-- 判定之前写者怎么做：`Admit`（`ChainHalt::default()`，历史在打开时已同步证明过）或 `Refuse`（`ChainHalt::awaiting_proof()`，服务中的城）。 -/
inductive Before where
  | Admit
  | Refuse
  deriving DecidableEq, Repr

structure ChainHalt (Reason : Type) where
  verdict : Option (Verdict Reason)
  before : Before
  deriving DecidableEq, Repr

/-- 对停机值的一次调用。 -/
inductive Call (Reason : Type) where
  | prove
  | trip (reason : Reason)
  deriving DecidableEq, Repr

variable {Reason : Type}

/-- `ChainHalt::settle`：还没有判定才写下这一个。 -/
def settle (h : ChainHalt Reason) (v : Verdict Reason) : ChainHalt Reason :=
  match h.verdict with
  | none => { h with verdict := some v }
  | some _ => h

def Call.verdict : Call Reason → Verdict Reason
  | .prove => .Whole
  | .trip r => .Broken r

/-- 依次调用之后的停机值。 -/
def calls (h : ChainHalt Reason) (cs : List (Call Reason)) : ChainHalt Reason :=
  cs.foldl (fun h c => settle h c.verdict) h

/-- 写者问停机值时的三种回答（`ChainHalt::admit`）。 -/
inductive Admission (Reason : Type) where
  | Admitted
  | ChainHalted (reason : Reason)
  | Unproven
  deriving DecidableEq, Repr

def admit (h : ChainHalt Reason) : Admission Reason :=
  match h.verdict, h.before with
  | some .Whole, _ => .Admitted
  | none, .Admit => .Admitted
  | some (.Broken r), _ => .ChainHalted r
  | none, .Refuse => .Unproven

theorem settled_stays (h : ChainHalt Reason) (v : Verdict Reason) (cs : List (Call Reason))
    (settled : h.verdict = some v) : (calls h cs).verdict = some v := by
  induction cs generalizing h with
  | nil => exact settled
  | cons c rest ih =>
    apply ih
    simp [settle, settled]

/-- **第一个判定有效。** 从没有判定的停机值起，一串调用之后的判定就是第一次调用的判定。 -/
theorem the_first_verdict_stands (h : ChainHalt Reason) (c : Call Reason) (rest : List (Call Reason))
    (fresh : h.verdict = none) : (calls h (c :: rest)).verdict = some c.verdict := by
  have first : calls h (c :: rest) = calls (settle h c.verdict) rest := rfl
  rw [first]
  apply settled_stays
  simp [settle, fresh]

theorem admission_follows_the_verdict (h : ChainHalt Reason) :
    (h.verdict = none → admit h = (match h.before with | .Admit => .Admitted | .Refuse => .Unproven)) ∧
      (h.verdict = some .Whole → admit h = .Admitted) ∧
      (∀ r, h.verdict = some (.Broken r) → admit h = .ChainHalted r) := by
  obtain ⟨v, b⟩ := h
  refine ⟨fun e => ?_, fun e => ?_, fun r e => ?_⟩ <;> subst e <;> cases b <;> rfl

/-- 一次服务：先证明完好、后来又有一条跳闸，写者照旧放行。 -/
example : admit (calls (⟨none, .Refuse⟩ : ChainHalt String) [.prove, .trip "late"]) = .Admitted := by
  decide

/-- 证明一波读的段数（`chain_audit::PROOF_WAVE`）。 -/
def PROOF_WAVE : Nat := 8

/-- `ProofCount.waves`：段数除以 `PROOF_WAVE` 向上取整。 -/
def waves (segments : Nat) : Nat := (segments + PROOF_WAVE - 1) / PROOF_WAVE

/-- 一波取段序上接下来的至多 `PROOF_WAVE` 段，直到段取完；这样分出来的波数就是 `waves`。 -/
def inWaves (segments : List Nat) : List (List Nat) :=
  if _h : segments.length ≤ PROOF_WAVE then (if segments = [] then [] else [segments])
  else segments.take PROOF_WAVE :: inWaves (segments.drop PROOF_WAVE)
termination_by segments.length
decreasing_by simp only [List.length_drop, PROOF_WAVE] at _h ⊢; omega

theorem waves_depend_only_on_the_segments :
    ∀ (n : Nat) (segments : List Nat), segments.length = n → (inWaves segments).length = waves n := by
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro segments hl
    unfold inWaves
    split
    · rename_i small
      split
      · rename_i empty
        subst empty
        simp at hl
        subst hl
        rfl
      · rename_i nonempty
        have pos : 0 < segments.length := List.length_pos_iff.mpr nonempty
        simp only [List.length_singleton, waves, PROOF_WAVE] at small ⊢
        omega
    · rename_i big
      simp only [List.length_cons]
      have shorter : (segments.drop PROOF_WAVE).length < n := by
        simp [PROOF_WAVE] at big ⊢; omega
      rw [ih _ shorter _ rfl]
      simp only [List.length_drop, waves, PROOF_WAVE] at big ⊢
      omega

end Storage.ChainAudit
