-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::ceiling

规定 `provider::ceiling`（`crates/gateway/src/provider/ceiling.rs`）：关于一次调用输出上限的几种说法谁胜、是谁说的。本文件是 `crates/gateway/Spec.lean` 的一个分部；别处引它的决定作 `gateway D<n>`。

§8-17（`crates/gateway/spec/Provider.lean`）写这架梯子的接口与理由；本分部是它的模型：人填 → 上游陈述 → 钉版目录与预置表 → 策略缺省，后两档只在这一面要一个数时作答。

这里没有状态机：`resolve` 是一架梯子，每一档压过下一档是它的一条 `match` 臂，读定义即可，不另写定理。证明的是两条跨过所有臂的性质：要这个字段的一面恒得到一个数（`a_face_that_needs_a_figure_always_gets_one`），本城钉下的数与策略缺省只在那样的一面上作答（`the_city_states_a_figure_only_where_the_face_needs_one`）。Rust 的 `resolve` 取 `Target`（`base_url`、`id`、`wire`），在梯子走到那一档时才问预置表；模型把预置表的答案当作一个值传进来，结果相同。平台：纯判定，三个平台相同。

派生检查：`provider::ceiling` 的测试各判一个输入（`the_higher_rung_wins_and_says_that_it_did`、`on_a_face_that_takes_no_figure_the_provider_picks_when_nobody_stated_one`、`an_unknown_model_at_an_unknown_host_is_still_callable`）；两条性质还没有覆盖整个输入空间的 proptest（人、上游、钉版、预置各有无，三面），记为债。
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

/-- 请求写在哪一面（`kernel::DialectKind`）。kernel 的 Lean 规格（`crates/kernel/Spec.lean`）不定义这个类型，这里照它的三个变体写一份模型里的类型。 -/
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
