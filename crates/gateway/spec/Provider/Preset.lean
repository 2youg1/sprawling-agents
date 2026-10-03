-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::preset

规定 `provider::preset`（`crates/gateway/src/provider/preset.rs`）：一家厂商文档写下的路径、形状与上限，每行注出处；一个模型 id 由哪一行作答。本文件是 `crates/gateway/Spec.lean` 的一个分部；别处引它的决定作 `gateway D<n>`。

§8-17（`crates/gateway/spec/Provider.lean`）写预置表的接口、行与出处；本分部是 `model_for` 选哪一行的模型：host 自己的行先答，没有自己模型行的远端 host 再借厂商的行，这台电脑上的服务一行也不借，前缀最长者胜。

与 Rust 的对应：Rust 的 `model_for` 取 `base_url`，`own` 是 `host_row(base_url)` 那一行的模型行，厂商的行是 `PRESETS` 全表按序展开，`isLocal` 是 `reach::is_local(base_url)`，读的只是 URL 的 host 文字，所以三个平台相同。派生检查：`provider::preset` 的 `a_documented_model_is_matched_by_the_longest_prefix_that_fits` 与 `a_relay_forwarding_a_vendors_id_reads_the_vendors_row_and_a_local_server_does_not` 各判几个 id；三条定理还没有在随机的行表与 id 上驱动 Rust 的检查（`model_for` 读的是静态的 `PRESETS`，要先把选行那一步拆成取行表为参数的函数），记为债。
-/

namespace Gateway.Provider.Preset

/-- 预置表的一个模型行（`provider::preset::ModelPreset`）；模型只留下判「哪一行作答」要读的那一列。 -/
structure ModelPreset where
  id_prefix : String
  deriving DecidableEq, Repr

/-- 前缀最长的那一行；一样长时取靠后的那一行，与 Rust 的 `Iterator::max_by_key` 相同。 -/
def longest : List ModelPreset → Option ModelPreset
  | [] => none
  | row :: rest =>
    match longest rest with
    | none => some row
    | some other => if other.id_prefix.length < row.id_prefix.length then some row else some other

/-- `provider::preset::model_for`。`own` 是 base URL 所在 host 自己的模型行；`vendors` 是全表的模型行，按表的顺序；`isLocal` 是 `reach::is_local(base_url)`。host 没有自己的模型行、又不在这台电脑上时（一个转发厂商 id 的中转站），厂商的行也来作答。 -/
def model_for (own vendors : List ModelPreset) (isLocal : Bool) (id : String) :
    Option ModelPreset :=
  let relayed := own.isEmpty && !isLocal
  longest ((own ++ (if relayed then vendors else [])).filter (fun row => id.startsWith row.id_prefix))

theorem longest_is_a_member : ∀ (rows : List ModelPreset) (row : ModelPreset),
    longest rows = some row → row ∈ rows
  | [], _, found => by simp [longest] at found
  | head :: rest, row, found => by
    unfold longest at found
    cases hrest : longest rest with
    | none =>
      rw [hrest] at found
      simp at found
      simp [found]
    | some other =>
      rw [hrest] at found
      have inRest := longest_is_a_member rest other hrest
      by_cases longer : other.id_prefix.length < head.id_prefix.length
      · simp [longer] at found
        simp [found]
      · simp [longer] at found
        subst found
        simp [inRest]

/-- 这台电脑上的服务一行也不借：它服务的模型的窗口是它自己的配置，套用厂商图表就是把一个它放不下的数字发给它。 -/
theorem a_server_on_this_machine_borrows_no_vendor_row (vendors : List ModelPreset) (id : String) :
    model_for [] vendors true id = none := by
  simp [model_for, longest]

/-- host 有自己的模型行时只由它们作答，厂商的行不进来。 -/
theorem a_host_with_rows_of_its_own_answers_from_them (own vendors : List ModelPreset)
    (isLocal : Bool) (id : String) (row : ModelPreset) (hasRows : own ≠ [])
    (found : model_for own vendors isLocal id = some row) : row ∈ own := by
  have empty : own.isEmpty = false := by
    cases own with
    | nil => exact absurd rfl hasRows
    | cons _ _ => rfl
  simp only [model_for, empty, Bool.false_and] at found
  have member := longest_is_a_member _ row found
  simp at member
  exact member.1

/-- 作答的那一行的前缀确实是这个 id 的前缀。 -/
theorem the_answer_is_a_prefix_of_the_id (own vendors : List ModelPreset) (isLocal : Bool)
    (id : String) (row : ModelPreset) (found : model_for own vendors isLocal id = some row) :
    id.startsWith row.id_prefix = true := by
  simp only [model_for] at found
  exact (List.mem_filter.mp (longest_is_a_member _ row found)).2

/-- 一个没有自己模型行的远端 host（中转站）读厂商的行：转发 `claude-sonnet-4-5` 的中转站得到厂商那一行。 -/
example : model_for [] [⟨"claude-haiku-4"⟩, ⟨"claude-sonnet-4"⟩] false "claude-sonnet-4-5"
    = some ⟨"claude-sonnet-4"⟩ := by
  decide +kernel

/-- 前缀更长的那一行胜：一个家族与它的一员可以各有一行，更具体的那一行作答。 -/
example : model_for [⟨"claude-sonnet"⟩, ⟨"claude-sonnet-4"⟩] [] false "claude-sonnet-4-5"
    = some ⟨"claude-sonnet-4"⟩ := by
  decide +kernel

end Gateway.Provider.Preset
