-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 让二进制成为一个词：搜索路径的两个判定

规定 `crates/sprawling/src/install.rs` 里的 `plan_append` 与 `plan_remove`（`bin::install`，形状 4 adapter：决定纯，落地薄）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（sprawling-SPEC.md 8-9）。拷贝、注册表读写与广播是适配器的事，这里不建模。

一条搜索路径在模型里是它按分隔符切开的各段，按原来的次序。段的类型 `α` 是参数，因为判定只需要两件事：两段是不是同一个目录（`same`，Rust 里是 `same_directory`：去掉首尾空白与尾随的斜杠，Windows 上再不分大小写），以及一段是不是空段（`blank`）。空白的整串在模型里是空表，于是「空串变成那一个目录」不是一个特例。

四组性质：

* **追加是幂等的**——追加之后再判一次，答「已经在了」；
* **追加不遮挡**——原来的各段（去掉尾随的空段）原样在前，新目录在最后，所以装进去的目录不会压过系统已经解析到的东西；
* **追加再移除回到原值**——原值里没有这个目录、也没有尾随空段时，撤销恰好撤销那一次追加；
* **移除只动那一个目录**——其余每段按原次序留下，空段也留下；没有可移除的就答 `absent`，一字不写。
-/

namespace Sprawling.Install

variable {α : Type} (same : α → α → Bool) (blank : α → Bool)

/-- 搜索路径上已经有这个目录（Rust：`on_search_path`）。 -/
def onPath (entries : List α) (dir : α) : Bool :=
  entries.any (same · dir)

/-- 去掉尾随的空段：Rust 在拼接之前 `trim_end_matches(SEPARATOR)`，于是 `a;b;` 追加之后不会多出一个空段。 -/
def trimEnd (entries : List α) : List α :=
  (entries.reverse.dropWhile blank).reverse

/-- 安装对搜索路径做的事（Rust：`PathEdit`）。 -/
inductive Edit (α : Type) where
  | alreadyPresent
  | append (next : List α)
  deriving Repr, DecidableEq

/-- 卸载对搜索路径做的事（Rust：`PathRemoval`）。 -/
inductive Removal (α : Type) where
  | absent
  | rewrite (next : List α)
  deriving Repr, DecidableEq

/-- `plan_append`：已经在就不动；否则接在最后，而不是放在最前。 -/
def planAppend (entries : List α) (dir : α) : Edit α :=
  if onPath same entries dir then .alreadyPresent
  else .append (trimEnd blank entries ++ [dir])

/-- `plan_remove`：滤掉与这个目录相同的每一段；一段也没滤掉就是 `absent`。 -/
def planRemove (entries : List α) (dir : α) : Removal α :=
  if (entries.filter (fun e => !same e dir)).length = entries.length then .absent
  else .rewrite (entries.filter (fun e => !same e dir))

/-! ## 追加是幂等的 -/

theorem append_then_append_is_already_present (entries next : List α) (dir : α)
    (reflexive : same dir dir = true)
    (h : planAppend same blank entries dir = .append next) :
    planAppend same blank next dir = .alreadyPresent := by
  unfold planAppend at h
  split at h
  · cases h
  · cases h
    simp [planAppend, onPath, reflexive]

theorem present_is_left_alone (entries : List α) (dir : α)
    (h : onPath same entries dir = true) :
    planAppend same blank entries dir = .alreadyPresent := by
  simp [planAppend, h]

/-! ## 追加不遮挡 -/

theorem append_keeps_what_was_first (entries next : List α) (dir : α)
    (h : planAppend same blank entries dir = .append next) :
    next = trimEnd blank entries ++ [dir] := by
  unfold planAppend at h
  split at h
  · cases h
  · cases h; rfl

/-! ## 移除只动那一个目录 -/

theorem removal_is_the_filter (entries next : List α) (dir : α)
    (h : planRemove same entries dir = .rewrite next) :
    next = entries.filter (fun e => !same e dir) := by
  unfold planRemove at h
  split at h
  · cases h
  · cases h; rfl

theorem removal_leaves_no_copy (entries next : List α) (dir : α)
    (h : planRemove same entries dir = .rewrite next) :
    onPath same next dir = false := by
  rw [removal_is_the_filter same entries next dir h]
  simp [onPath, List.any_eq_false]

theorem absent_exactly_when_not_on_path (entries : List α) (dir : α) :
    planRemove same entries dir = .absent ↔ onPath same entries dir = false := by
  unfold planRemove onPath
  constructor
  · intro h
    split at h
    · rename_i hlen
      have hall := List.length_filter_eq_length_iff.mp hlen
      simpa [List.any_eq_false] using hall
    · cases h
  · intro h
    have hall : ∀ e ∈ entries, (!same e dir) = true := by
      simpa [List.any_eq_false] using h
    simp [List.filter_eq_self.mpr hall]

/-! ## 追加再移除回到原值 -/

theorem append_then_remove_restores (entries next : List α) (dir : α)
    (reflexive : same dir dir = true)
    (trimmed : trimEnd blank entries = entries)
    (h : planAppend same blank entries dir = .append next) :
    planRemove same next dir = .rewrite entries := by
  have absent : onPath same entries dir = false := by
    unfold planAppend at h
    split at h
    · cases h
    · simpa using ‹¬onPath same entries dir = true›
  have keep : entries.filter (fun e => !same e dir) = entries := by
    apply List.filter_eq_self.mpr
    simpa [onPath, List.any_eq_false] using absent
  rw [append_keeps_what_was_first same blank entries next dir h, trimmed]
  simp [planRemove, List.filter_append, keep, reflexive]

/-! ## 可实现性：每一支都有输入走到 -/

example : planAppend (· == ·) (· == "") ([] : List String) "bin" = .append ["bin"] := by decide
example : planAppend (· == ·) (· == "") ["a", "bin"] "bin" = .alreadyPresent := by decide
example : planAppend (· == ·) (· == "") ["a", ""] "bin" = .append ["a", "bin"] := by decide
example : planRemove (· == ·) ["a", "", "bin", "c"] "bin" = .rewrite ["a", "", "c"] := by decide
example : planRemove (· == ·) ["a", "c"] "bin" = .absent := by decide

end Sprawling.Install
