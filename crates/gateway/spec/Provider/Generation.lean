-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::generation

规定 `provider::generation`（`crates/gateway/src/provider/generation.rs`）：一个
`(Endpoint, model, dialect)` 的采样参数支持情况、人在模型编辑器里保存的默认值，以及新
Session 冻下的有效生成配置。本文件是 `crates/gateway/Spec.lean` 的分部 §8-41。

模型只含纯判定；文本到 JSON 数字的转换、账本与文件 I/O 不在模型里。Rust 对应检查在
`provider::generation::tests`，请求体检查在 `gateway::endpoint::call` 与
`gateway::dialect::request`。
-/

namespace Gateway.Provider.Generation

/-- 一个参数的支持事实；`unknown` 允许显式设置，但不承诺上游接受。 -/
inductive Support where
  | unknown
  | supported
  | unsupported
  deriving DecidableEq, Repr

/-- 说出支持事实的一级；无人说时没有来源。 -/
inductive Source where
  | person
  | upstream
  | preset
  deriving DecidableEq, Repr

/-- 页面读到的一项支持答案。 -/
structure Answer where
  support : Support
  source : Option Source
  deriving DecidableEq, Repr

/-- D64 人的纠正压过上游与预置，上游压过预置；每一级整项作答，不拼合来源。

理由：同一个参数在两个 provider 或 dialect 上能力不同，而一次报错不是能力声明。
被否：按一次 HTTP 400 写成 `unsupported`，那会把临时或歧义错误永久化。 -/
def resolve (person upstream preset : Option Support) : Answer :=
  match person with
  | some support => { support, source := some .person }
  | none =>
    match upstream with
    | some support => { support, source := some .upstream }
    | none =>
      match preset with
      | some support => { support, source := some .preset }
      | none => { support := .unknown, source := none }

theorem person_answers_first (person upstream preset : Option Support) (support : Support)
    (said : person = some support) :
    resolve person upstream preset = { support, source := some .person } := by
  simp [resolve, said]

theorem upstream_answers_when_the_person_is_silent (upstream preset : Option Support)
    (support : Support) (said : upstream = some support) :
    resolve none upstream preset = { support, source := some .upstream } := by
  simp [resolve, said]

theorem preset_answers_when_nearer_rungs_are_silent (preset : Option Support)
    (support : Support) (said : preset = some support) :
    resolve none none preset = { support, source := some .preset } := by
  simp [resolve, said]

theorem nobody_states_unknown : resolve none none none = { support := .unknown, source := none } := by
  rfl

/-- 输出偏好高过已知硬上限的唯一拒绝。 -/
inductive OutputRefusal where
  | AboveHardCeiling
  deriving DecidableEq, Repr

/-- 输出偏好先作答；没有偏好时沿用硬上限；偏好高过硬上限时拒绝，不截断。 -/
def output (preferred hard : Option Nat) : Except OutputRefusal (Option Nat) :=
  match preferred, hard with
  | some wanted, some ceiling =>
    if wanted > ceiling then .error .AboveHardCeiling else .ok (some wanted)
  | some wanted, none => .ok (some wanted)
  | none, _ => .ok hard

theorem output_never_passes_a_hard_ceiling (wanted ceiling : Nat)
    (accepted : output (some wanted) (some ceiling) = .ok (some wanted)) : wanted ≤ ceiling := by
  simp only [output] at accepted
  split at accepted
  · contradiction
  · rename_i within
    exact Nat.le_of_not_gt within

theorem no_preference_keeps_the_existing_ladder (hard : Option Nat) :
    output none hard = .ok hard := by
  rfl

/-- 类型化值之后仍可应用原始 override；后者缺席时才留下前者。 -/
def overridden {Value : Type} (typed raw : Option Value) : Option Value :=
  match raw with
  | some value => some value
  | none => typed

theorem raw_override_answers_last {Value : Type} (typed : Option Value) (raw : Value) :
    overridden typed (some raw) = some raw := by
  rfl

theorem no_raw_override_keeps_the_typed_value {Value : Type} (typed : Option Value) :
    overridden typed none = typed := by
  rfl

/-- 正常路径可实现：无人说能力、没有偏好、没有覆盖时仍是当前请求。 -/
example :
    resolve none none none = { support := .unknown, source := none }
      ∧ output none (some 8192) = .ok (some 8192)
      ∧ overridden (some "0.2") none = some "0.2" := by
  exact ⟨nobody_states_unknown, no_preference_keeps_the_existing_ladder _, rfl⟩

end Gateway.Provider.Generation
