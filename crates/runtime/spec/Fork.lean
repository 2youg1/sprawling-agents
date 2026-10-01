-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::fork 的模型

证明 `fork`、`fork::lineage`、`fork::indexed`（`crates/runtime/src/` 下同名的文件）必须守住的性质。分叉：一个新 run，它窗口里的历史是另一个 run 的逐字节前缀，从母 run 自己的记录重建。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Fork

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `fork::prefix`：验过的母序列从第一行到 `at_seq` 的原始行（A19）。`at_seq` 过了尾就拒绝，恒不静默截到末尾。行的内容与本模型无关，所以行是任意类型。`prefix` 在 Lean 里是关键字，故写作 `«prefix»`。 -/
def «prefix» {Line : Type} (mother : List Line) (at_seq : Nat) : Except Code (List Line) :=
  if at_seq < mother.length then .ok (mother.take (at_seq + 1)) else .error .InvalidArgs

/-- 分叉点过了母序列的尾，拒绝，而不是截到末尾。 -/
theorem a_fork_past_the_tail_is_refused {Line : Type} (mother : List Line) (at_seq : Nat)
    (past : mother.length ≤ at_seq) : «prefix» mother at_seq = .error .InvalidArgs := by
  unfold «prefix»
  rw [if_neg (by omega)]

/-- 分叉前缀恰好是母序列的头 `at_seq + 1` 行，一行不多一行不少。 -/
theorem a_fork_is_the_mothers_first_lines {Line : Type} (mother : List Line) (at_seq : Nat)
    (lines : List Line) (forked : «prefix» mother at_seq = .ok lines) :
    lines = mother.take (at_seq + 1) ∧ lines.length = at_seq + 1 := by
  unfold «prefix» at forked
  split at forked
  · injection forked with taken
    subst taken
    exact ⟨rfl, by simp; omega⟩
  · cases forked

/-- 母 run 的一行，按切点关心的样子：一条带 `calls` 条调用的 `model_returned`、一条 `tool_result`、其余任何一行。 -/
inductive Line where
  | returned (calls : Nat)
  | result
  | other
  deriving DecidableEq, Repr

/-- 读完这些行之后，开着的一波还欠几条 `tool_result`（`fold_run` 的 `Wave::expect`）。 -/
def owed (lines : List Line) : Nat :=
  lines.foldl (fun waiting line => match line with
    | .returned calls => calls
    | .result => waiting - 1
    | .other => waiting) 0

/-- 第 `i` 行之后能不能切：读到这一行为止，没有一条调用还在等它的结果。 -/
def safeAt (lines : List Line) (i : Nat) : Bool :=
  owed (lines.take (i + 1)) == 0

/-- 从 `n` 往回找第一个能切的位置。 -/
def retreat (safe : Nat → Bool) : Nat → Option Nat
  | 0 => if safe 0 then some 0 else none
  | n + 1 => if safe (n + 1) then some (n + 1) else retreat safe n

/-- `fold_run` 的切点：要求在 `at_seq` 切，实际用到的是不晚于它的最后一个安全点，并如实交回（`Inherited::at`）。 -/
def cut (lines : List Line) (at_seq : Nat) : Option Nat :=
  retreat (safeAt lines) at_seq

/-- 往回找到的位置能切，且不晚于要求的那一行。 -/
theorem retreat_lands_on_a_safe_line (safe : Nat → Bool) :
    ∀ (n i : Nat), retreat safe n = some i → safe i = true ∧ i ≤ n
  | 0, i, found => by
    cases here : safe 0 with
    | true =>
      simp [retreat, here] at found
      subst found
      exact ⟨here, Nat.le_refl 0⟩
    | false => simp [retreat, here] at found
  | n + 1, i, found => by
    cases here : safe (n + 1) with
    | true =>
      simp [retreat, here] at found
      subst found
      exact ⟨here, Nat.le_refl _⟩
    | false =>
      simp [retreat, here] at found
      have earlier := retreat_lands_on_a_safe_line safe n i found
      exact ⟨earlier.1, by omega⟩

/-- 往回只退到必须退的地方：找到的位置与要求的那一行之间，没有一个能切的位置被跳过。 -/
theorem retreat_goes_back_no_further_than_needed (safe : Nat → Bool) :
    ∀ (n i : Nat), retreat safe n = some i → ∀ j, i < j → j ≤ n → safe j = false
  | 0, _, _, _, after, within => by omega
  | n + 1, i, found, j, after, within => by
    cases here : safe (n + 1) with
    | true =>
      simp [retreat, here] at found
      omega
    | false =>
      simp [retreat, here] at found
      by_cases top : j = n + 1
      · subst top
        exact here
      · exact retreat_goes_back_no_further_than_needed safe n i found j after (by omega)

/-- 要求的那一行本身能切，就不退。 -/
theorem a_safe_cut_is_kept (safe : Nat → Bool) (n : Nat) (here : safe n = true) :
    retreat safe n = some n := by
  cases n with
  | zero => simp [retreat, here]
  | succ m => simp [retreat, here]

/-- **分支从不继承半个交换。** 切点之前的每一波都收齐了它的结果：一条有调用没有答案的助手消息，任何 provider 都拒。 -/
theorem the_cut_never_splits_a_wave (lines : List Line) (at_seq i : Nat)
    (found : cut lines at_seq = some i) :
    owed (lines.take (i + 1)) = 0 ∧ i ≤ at_seq := by
  have landed := retreat_lands_on_a_safe_line (safeAt lines) at_seq i found
  exact ⟨by simpa [safeAt] using landed.1, landed.2⟩

/-- 切点落在一波之内时往回退到这一波之前，并如实交回实际用到的那一行。 -/
example : cut [.other, .returned 2, .result, .result, .returned 1] 4 = some 3 := by decide

/-- 一行开场之后立刻切，切在开场那一行。 -/
example : cut [.other, .returned 1, .result] 0 = some 0 := by decide

/-- 开篇的写法（`kernel::event::record::Opening`，runtime 再导出）。 -/
inductive Opening where
  | FromJob
  | Inherited
  | WithPerson
  deriving DecidableEq, Repr

/-- `fork::rebuilt`：照 `run_started.opening` 重建母 run 第一条消息的写法（§8-58 的表）。第二个参数是 `run_started.job` 在不在场，只用来读加键之前的行。 -/
def rebuilt : Option Opening → Bool → Opening
  | some .FromJob, _ => .Inherited
  | some .Inherited, _ => .Inherited
  | some .WithPerson, _ => .WithPerson
  | none, true => .Inherited
  | none, false => .WithPerson

/-- 只有 `FromJob` 被改写；另外两种照母 run 发出的写法重建，provider 的前缀缓存从第一条消息起命中。 -/
theorem only_from_job_is_rewritten (opening : Opening) (job : Bool) :
    rebuilt (some opening) job ≠ opening → opening = .FromJob := by
  cases opening <;> cases job <;> decide

/-- 分支永远不说「任务在上面的 JOB.md 里」：它的前缀带的是它自己的 brief，不是母 run 的那一份。 -/
theorem a_branch_never_points_at_a_job_file_it_was_not_given (opening : Option Opening)
    (job : Bool) : rebuilt opening job ≠ .FromJob := by
  cases opening with
  | none => cases job <;> decide
  | some spoken => cases spoken <;> cases job <;> decide

end Runtime.Fork
