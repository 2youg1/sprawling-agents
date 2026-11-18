-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 的 Magpie 目录选择模型

这里只拥有无 `HARNESS_MODEL` 时的目录选择与上下文用量算术。
网络、进程与终端渲染不在模型内。目录顺序由 Magpie 给出，`first_id`
是它为用户路由准备的首选；首选不在可见条目中时才退回首个条目。
-/

namespace Mei.Gateway

structure Model where
  id : String
  contextTokens : Option Nat
  efforts : List String
  deriving DecidableEq, Repr

structure Catalog where
  firstId : Option String
  models : List Model
  deriving DecidableEq, Repr

/-- 首选必须仍是一个可见模型；空目录没有选择，不能编造模型名。 -/
def select (catalog : Catalog) : Option Model :=
  match catalog.firstId with
  | some id =>
    match catalog.models.find? (fun model => model.id == id) with
    | some model => some model
    | none => catalog.models.head?
  | none => catalog.models.head?

theorem selected_is_visible (catalog : Catalog) (model : Model)
    (selected : select catalog = some model) : model ∈ catalog.models := by
  unfold select at selected
  cases first : catalog.firstId with
  | none =>
    have head : catalog.models.head? = some model := by
      simpa [first] using selected
    apply List.mem_of_mem_head?
    simp [head]
  | some id =>
    cases found : catalog.models.find? (fun candidate => candidate.id == id) with
    | some value =>
      simp [first, found] at selected
      cases selected
      exact List.mem_of_find?_eq_some found
    | none =>
      have head : catalog.models.head? = some model := by
        simpa [first, found] using selected
      apply List.mem_of_mem_head?
      simp [head]

/-- Anthropic 的总输入包含未缓存、缓存写入与缓存读取三部分。 -/
def usedTokens (input cacheCreation cacheRead : Nat) : Nat :=
  input + cacheCreation + cacheRead

/-- 百分之一个百分点，供终端四舍五入显示两位小数；零窗口没有比例。 -/
def percentHundredths (used window : Nat) : Option Nat :=
  if window = 0 then none else some ((used * 10000 + window / 2) / window)

example : usedTokens 231 0 256 = 487 := by native_decide
example : percentHundredths 487 1048576 = some 5 := by native_decide
example : percentHundredths 1 3 = some 3333 := by native_decide
example : percentHundredths 10 0 = none := by rfl

example :
    let route := Model.mk "group/plan-first" (some 1048576) ["none", "low", "high", "max"]
    let other := Model.mk "provider/model" (some 200000) ["low", "high"]
    select (Catalog.mk (some "group/plan-first") [other, route]) = some route := by
  native_decide

example :
    let first := Model.mk "first/model" none []
    select (Catalog.mk (some "missing/model") [first]) = some first := by
  native_decide

example : select (Catalog.mk none []) = none := by rfl

end Mei.Gateway
