-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::ceiling

规定 `provider::ceiling`（`crates/gateway/src/provider/ceiling.rs`）：关于一次调用输出上限的几种说法谁胜、是谁说的。本文件是 `crates/gateway/Spec.lean` 的一个分部；别处引它的决定作 `gateway D<n>`。

§8-17（`crates/gateway/spec/Provider.lean`）写这架梯子的接口与理由；本分部是它的模型：人填 → 上游陈述 → 钉版目录与预置表 → 策略缺省，后两档只在这一面要一个数时作答。
-/

namespace Gateway.Provider.Ceiling

/-- 一个上限，恒为正（`kernel::Ceiling`）：零不是上限，Anthropic 的线拒它，OpenAI 的线带着它答不出内容。 -/
structure Ceiling where
  tokens : Nat
  positive : 0 < tokens

/-- `Ceiling::new`：零答 `none`。 -/
def Ceiling.new (tokens : Nat) : Option Ceiling :=
  if h : 0 < tokens then some ⟨tokens, h⟩ else none

/-- 谁说出了一次调用所带的上限（`provider::ceiling::CeilingSource`），从高到低。 -/
inductive CeilingSource where
  | Person
  | Upstream
  | Preset
  | Policy
  deriving DecidableEq, Repr

/-- 请求写在哪一面（`kernel::DialectKind`）。kernel 的规格还没有迁到 Lean，这里照它的三个变体写一份模型里的类型。 -/
inductive DialectKind where
  | Anthropic
  | OpenAi
  | OpenAiResponses
  deriving DecidableEq, Repr

/-- 一面的每个请求要不要写上限（`provider::ceiling` 里私有的 `Field`）。 -/
inductive Field where
  | Required
  | Optional
  deriving DecidableEq, Repr

/-- `field_on`：messages 面拒绝不带 `max_tokens` 的请求，chat 与 responses 两面的上限字段可选。 -/
def field_on : DialectKind → Field
  | .Anthropic => .Required
  | .OpenAi => .Optional
  | .OpenAiResponses => .Optional

/-- 一次调用所带的上限（`provider::ceiling::OutputCeiling`）：一个数字与说出它的那一档，或者不写这个字段、由供应方按模型取缺省。 -/
inductive OutputCeiling where
  | Sent (tokens : Ceiling) (source : CeilingSource)
  | ProviderDefault

/-- 人与上游说过的话（`provider::ceiling::Stated`）。 -/
structure Stated where
  person : Option Ceiling
  upstream : Option Ceiling

/-- `OutputCeiling::resolve`。`pinned` 是钉版目录按精确 id 的那一行；`preset` 是 `preset::ceiling_for(target.base_url, target.id)` 的答案，与 `pinned` 是同一档的两个索引；`policy` 是 `kernel::consts_policy::OUTPUT_CEILING_DEFAULT`。答 `none` 只在这一面要一个数而 `policy` 本身是零时，而 policy 模块自己的测试禁止那个状态。 -/
def resolve (stated : Stated) (pinned preset : Option Ceiling) (policy : Nat)
    (wire : DialectKind) : Option OutputCeiling :=
  match stated.person, stated.upstream, field_on wire with
  | some tokens, _, _ => some (.Sent tokens .Person)
  | none, some tokens, _ => some (.Sent tokens .Upstream)
  | none, none, .Optional => some .ProviderDefault
  | none, none, .Required =>
    match pinned.or preset with
    | some tokens => some (.Sent tokens .Preset)
    | none => (Ceiling.new policy).map (fun tokens => .Sent tokens .Policy)

/-- 人填的数压过其余三档：那是一个决定，不是推断。 -/
theorem the_person_outranks_every_other_rung (stated : Stated) (pinned preset : Option Ceiling)
    (policy : Nat) (wire : DialectKind) (tokens : Ceiling) (said : stated.person = some tokens) :
    resolve stated pinned preset policy wire = some (.Sent tokens .Person) := by
  unfold resolve
  rw [said]

/-- 人没说时，上游 `/models` 说过的话压过本城钉下的任何数字。 -/
theorem upstream_outranks_what_this_city_pinned (stated : Stated) (pinned preset : Option Ceiling)
    (policy : Nat) (wire : DialectKind) (tokens : Ceiling) (silent : stated.person = none)
    (said : stated.upstream = some tokens) :
    resolve stated pinned preset policy wire = some (.Sent tokens .Upstream) := by
  unfold resolve
  rw [silent, said]

/-- 不要这个字段的一面上，人与上游都没说就不写：本城的任何数字要么低于厂商按模型给的缺省，要么在对话变长后被拒。 -/
theorem a_face_that_takes_no_figure_is_sent_none_nobody_stated (stated : Stated)
    (pinned preset : Option Ceiling) (policy : Nat) (wire : DialectKind)
    (person : stated.person = none) (upstream : stated.upstream = none)
    (optional : field_on wire = .Optional) :
    resolve stated pinned preset policy wire = some .ProviderDefault := by
  unfold resolve
  rw [person, upstream, optional]

/-- 要这个字段的一面上，梯子恒给一个数：一个目录不认识的模型照样叫得通。前提是策略缺省不为零。 -/
theorem a_face_that_needs_a_figure_always_gets_one (stated : Stated)
    (pinned preset : Option Ceiling) (policy : Nat) (wire : DialectKind)
    (required : field_on wire = .Required) (positive : 0 < policy) :
    ∃ tokens source, resolve stated pinned preset policy wire = some (.Sent tokens source) := by
  unfold resolve
  cases hp : stated.person with
  | some tokens => exact ⟨tokens, .Person, rfl⟩
  | none =>
    cases hu : stated.upstream with
    | some tokens => exact ⟨tokens, .Upstream, rfl⟩
    | none =>
      rw [required]
      cases hr : pinned.or preset with
      | some tokens => exact ⟨tokens, .Preset, rfl⟩
      | none => exact ⟨⟨policy, positive⟩, .Policy, by simp [Ceiling.new, positive]⟩

/-- 钉版目录与预置表是同一档的两个索引，目录在前。 -/
theorem the_catalogue_outranks_the_preset_table (stated : Stated) (preset : Option Ceiling)
    (policy : Nat) (wire : DialectKind) (tokens : Ceiling) (person : stated.person = none)
    (upstream : stated.upstream = none) (required : field_on wire = .Required) :
    resolve stated (some tokens) preset policy wire = some (.Sent tokens .Preset) := by
  unfold resolve
  rw [person, upstream, required]
  rfl

/-- 本城钉下的数与策略缺省只在这一面要一个数时作答。 -/
theorem the_city_states_a_figure_only_where_the_face_needs_one (stated : Stated)
    (pinned preset : Option Ceiling) (policy : Nat) (wire : DialectKind) (tokens : Ceiling)
    (source : CeilingSource)
    (answered : resolve stated pinned preset policy wire = some (.Sent tokens source))
    (fromCity : source = .Preset ∨ source = .Policy) :
    field_on wire = .Required := by
  unfold resolve at answered
  cases hp : stated.person <;> cases hu : stated.upstream <;> cases hf : field_on wire <;>
    simp_all
  all_goals
    cases fromCity <;> simp_all

/-- 一个目录与预置表都不认识的模型，在 messages 面上以策略缺省叫得通。 -/
example : resolve ⟨none, none⟩ none none 8192 .Anthropic
    = some (.Sent ⟨8192, by decide⟩ .Policy) := by
  simp [resolve, field_on, Ceiling.new]

end Gateway.Provider.Ceiling
