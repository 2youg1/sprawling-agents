-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Layout
import crates.city.spec.Building

/-!
# city::config_layers::ladder

规定 `config_layers::ladder` 与 `config_layers::ladder::tests`（`crates/city/src/` 下同名的文件）。一个地址被哪几级配置治理、每一级说了什么、哪一级说的算，以及一张只许写在较远几级的表写得太近时怎样被拒。接口与取舍写在 `crates/city/spec/ConfigLayers.lean` 的 §8-4，本文件只放它们的模型与证明；本文件是 `crates/city/Spec.lean` 的一个分部，决定引作 `city D<n>`。
-/

/-!
## 模型：一条梯子是一个值，最近的那一级说了算

`Ladder::read` 按 `Layer::ALL` 由远及近读每一级的文件，落点与更远一级相同的那一级丢掉；`tagged` 把每一级说的话放进 `kernel::LayeredValue` 它那一格，`resolve` 是 `tagged` 去掉那一级。哪一格压过哪一格是 `kernel::LayeredValue::resolve` 的答案（`crates/kernel/Spec.lean` §8-22）：居民格、楼格、城格依次。kernel 的规格把这条写成散文、没有它的 Lean 模型，所以下面的 `LayeredValue.resolve` 是那句散文的 Lean 读法，不是第二条规则；kernel 的 `Config` 分部长出模型时，这个定义搬过去，本文件改为 import 它（§3）。

* 设置页答的值就是 run 被治理的值（`the_setting_page_and_the_run_read_one_value`）：`settled_effort` 与 `load` 爬的是同一条梯子，`resolve` 只是 `tagged` 丢掉那一级。
* 最近的那一级说了算，且说出它是哪一级（`the_nearest_rung_that_speaks_wins`）；一级都没说就是 `none`，城有意把这件事交给缺省或供应方。
* 地址就是楼时只有两级、同一份文件只读一次（`an_address_that_is_its_building_reads_two_rungs`）；房间的地址读三级（`a_room_reads_three_rungs`）。文件的位置是 kernel 布局的模型（`crates/kernel/spec/Layout.lean` 的 `config`），楼取地址首段是 `crates/city/spec/Building.lean` 的 `head`。
* 有的表只许写在较远的几级：`[skills] shelves` 与 `[remote]` 只在城那一级，`[search]` 在城与楼两级（`crates/city/spec/ConfigLayers.lean` §8-4c）。一级写了一张比它够得到的那一级更近的表，读文件时即拒（`a_table_written_too_near_is_refused`）；一级写的表它全都够得到就照读（`a_rung_reads_the_tables_it_reaches`），城那一级够得到每一张（`the_city_states_its_own_tables`）。
* 梯子给一个解析拒词只加上文件，码与恢复语照解析器给的（`the_ladder_keeps_the_parsers_recovery`，D8 (a)）。
* 一层只点名一种居民，地址自己那一层有会话记录时梯子上的 harness 不生效（`a_session_that_opened_on_a_model_keeps_it`），没有记录时最近的那一级点名的 harness 生效（`without_a_record_the_nearest_harness_runs`，D6）。
-/

namespace City.ConfigLayers.Ladder

open Kernel.Address
open Kernel.Layout

/-- `Layer`：梯子的一级，由远及近。 -/
inductive Layer where
  | City
  | Building
  | Resident
  deriving DecidableEq, Repr

/-- `Layer::ALL`：每一级，最远的在前；读梯子的顺序，也是近的压过远的那个顺序。 -/
def Layer.ALL : List Layer :=
  [.City, .Building, .Resident]

/-- `Layer::ALL` 列出每一级：Rust 一侧 `Layer` 与 `ALL` 由同一张表（`rungs!`）声明，所以加一级不会漏在梯子外。 -/
theorem Layer.ALL_complete (layer : Layer) : layer ∈ Layer.ALL := by
  cases layer <;> simp [Layer.ALL]

/-- `kernel::LayeredValue`：每一级一格。 -/
structure LayeredValue (T : Type) where
  city : Option T
  building : Option T
  resident : Option T

/-- `kernel::LayeredValue::resolve` 的 Lean 读法：居民格、楼格、城格依次，第一个有值的胜。 -/
def LayeredValue.resolve {T : Type} (v : LayeredValue T) : Option T :=
  match v.resident, v.building with
  | some held, _ => some held
  | none, some held => some held
  | none, none => v.city

/-- 每一格各自换一下。 -/
def LayeredValue.map {S T : Type} (f : S → T) (v : LayeredValue S) : LayeredValue T :=
  ⟨v.city.map f, v.building.map f, v.resident.map f⟩

/-- 读过的一级：它是哪一级，它那份文件说了什么。 -/
abbrev Rung (C : Type) := Layer × C

/-- 把一级说的话放进它那一格（`Ladder::tagged` 里的那个穷尽 `match`）。 -/
def put {T : Type} (across : LayeredValue T) (rung : Layer) (held : Option T) : LayeredValue T :=
  match rung with
  | .City => { across with city := held }
  | .Building => { across with building := held }
  | .Resident => { across with resident := held }

/-- `Ladder::tagged`：一个关切在整条梯子上，每一句带着说它的那一级。 -/
def tagged {C T : Type} (rungs : List (Rung C)) (stated : C → Option T) : LayeredValue (T × Layer) :=
  rungs.foldl (fun across (rung, layer) => put across rung ((stated layer).map (·, rung)))
    ⟨none, none, none⟩

/-- `Ladder::resolve`：`tagged` 去掉那一级。 -/
def resolve {C T : Type} (rungs : List (Rung C)) (stated : C → Option T) : LayeredValue T :=
  (tagged rungs stated).map Prod.fst

/-- 从最近的一级往远处找，第一句说了话的。 -/
def nearest {C T : Type} (rungs : List (Rung C)) (stated : C → Option T) : Option (T × Layer) :=
  rungs.reverse.findSome? fun (rung, layer) => (stated layer).map (·, rung)

theorem the_setting_page_and_the_run_read_one_value {C T : Type} (rungs : List (Rung C))
    (stated : C → Option T) :
    (resolve rungs stated).resolve = (tagged rungs stated).resolve.map Prod.fst := by
  simp only [resolve, LayeredValue.map, LayeredValue.resolve]
  cases (tagged rungs stated).resident <;> cases (tagged rungs stated).building <;> rfl

/-- `Ladder::read` 读出的两种梯子：城、楼，再加上房间自己那一级（地址不是楼时）。 -/
def ladder {C : Type} (city building : C) (room : Option C) : List (Rung C) :=
  [(.City, city), (.Building, building)] ++ room.toList.map (.Resident, ·)

theorem the_nearest_rung_that_speaks_wins {C T : Type} (city building : C) (room : Option C)
    (stated : C → Option T) :
    (tagged (ladder city building room) stated).resolve = nearest (ladder city building room) stated := by
  cases room with
  | none =>
    simp only [ladder, tagged, nearest, put, Option.toList, List.map, List.append_nil, List.foldl,
      List.reverse_cons, List.reverse_nil, List.nil_append, List.cons_append, List.findSome?,
      LayeredValue.resolve]
    cases stated building <;> cases stated city <;> rfl
  | some room =>
    simp only [ladder, tagged, nearest, put, Option.toList, List.map, List.foldl,
      List.reverse_cons, List.reverse_nil, List.nil_append, List.cons_append, List.findSome?,
      LayeredValue.resolve]
    cases stated room <;> cases stated building <;> cases stated city <;> rfl

/-- `Layer::file`：每一级的文件。城那一级是城根的保留子树；楼那一级是地址首段那栋楼的；居民那一级是地址自己的。 -/
def file (names : Names) (city : CityLayout) (addr : Address) : Layer → List String
  | .City => governed_root names city ++ [names.config]
  | .Building => config names city (City.Building.head addr)
  | .Resident => config names city addr

/-- `Ladder::read`：按 `ALL` 读每一级，落点已被更远一级读过的那一级丢掉。`stated` 是一级的文件读出来的东西。 -/
def read {C : Type} (files : Layer → List String) (stated : Layer → C) : List (Rung C) :=
  (Layer.ALL.foldl
    (fun (acc : List (Rung C) × List (List String)) rung =>
      if files rung ∈ acc.2 then acc else (acc.1 ++ [(rung, stated rung)], acc.2 ++ [files rung]))
    ([], [])).1

theorem an_address_that_is_its_building_reads_two_rungs {C : Type} (names : Names)
    (city : CityLayout) (name : String) (stated : Layer → C) :
    read (file names city ⟨[name]⟩) stated = ladder (stated .City) (stated .Building) none := by
  simp [read, Layer.ALL, file, ladder, config, governed, governed_root, scope, City.Building.head]

theorem a_room_reads_three_rungs {C : Type} (names : Names) (city : CityLayout)
    (name room : String) (rest : List String) (stated : Layer → C) :
    read (file names city ⟨name :: room :: rest⟩) stated =
      ladder (stated .City) (stated .Building) (some (stated .Resident)) := by
  have tail : rest ++ [names.reserved, names.config] ≠ [names.config] := fun same => by
    have := congrArg List.length same
    simp at this
  simp [read, Layer.ALL, file, ladder, config, governed, governed_root, scope, City.Building.head, tail]

/-- `refuse::Confined`：只许写在较远几级的表。 -/
inductive Confined where
  | Shelves
  | Remote
  | Search
  | Agents
  deriving DecidableEq, Repr

/-- 一级离城多远：城 0、楼 1、房间 2。 -/
def Layer.depth : Layer → Nat
  | .City => 0
  | .Building => 1
  | .Resident => 2

/-- `Confined::nearest`：这张表最近能写到哪一级。 -/
def Confined.nearest : Confined → Layer
  | .Shelves => .City
  | .Remote => .City
  | .Search => .Building
  | .Agents => .City

/-- 这一级够不够得到这张表：不比它最近的那一级更近。 -/
def reaches (rung : Layer) (table : Confined) : Bool :=
  rung.depth ≤ table.nearest.depth

/-- 梯子上的拒词：哪一份文件、什么码、恢复语。 -/
structure Refusal where
  file : List String
  code : String
  subject : String
  recovery : String
  deriving DecidableEq, Repr

/-- `ladder::stated`：一级的文件读出的东西。解析拒了就把文件加在 subject 前、其余照原样（`in_file`）；这一级写了它够不到的表即拒（`refuse::too_near`），`confined` 列出这一层写了的每一张受限的表（`ConfigLayer::confined`）。 -/
def statedAt {C : Type} (confined : C → List Confined) (tooNear : List String → Confined → Refusal)
    (file : List String) (rung : Layer) (parsed : Except Refusal C) : Except Refusal C :=
  match parsed with
  | .error refused => .error { refused with file := file }
  | .ok layer =>
    match (confined layer).find? (fun table => !reaches rung table) with
    | none => .ok layer
    | some table => .error (tooNear file table)

theorem a_table_written_too_near_is_refused {C : Type} (confined : C → List Confined)
    (tooNear : List String → Confined → Refusal) (file : List String) (rung : Layer)
    (layer : C) (table : Confined) (writes : table ∈ confined layer)
    (near : reaches rung table = false) :
    ∃ named, reaches rung named = false ∧
      statedAt confined tooNear file rung (.ok layer) = .error (tooNear file named) := by
  simp only [statedAt]
  cases found : (confined layer).find? (fun table => !reaches rung table) with
  | none =>
    have := List.find?_eq_none.mp found table writes
    simp [near] at this
  | some named =>
    exact ⟨named, by simpa using List.find?_some found, rfl⟩

theorem a_rung_reads_the_tables_it_reaches {C : Type} (confined : C → List Confined)
    (tooNear : List String → Confined → Refusal) (file : List String) (rung : Layer)
    (layer : C) (within : ∀ table ∈ confined layer, reaches rung table = true) :
    statedAt confined tooNear file rung (.ok layer) = .ok layer := by
  simp only [statedAt]
  rw [List.find?_eq_none.mpr fun table writes => by simp [within table writes]]

theorem the_city_states_its_own_tables {C : Type} (confined : C → List Confined)
    (tooNear : List String → Confined → Refusal) (file : List String) (layer : C) :
    statedAt confined tooNear file .City (.ok layer) = .ok layer :=
  a_rung_reads_the_tables_it_reaches confined tooNear file .City layer
    fun table _ => by simp [reaches, Layer.depth]

theorem the_ladder_keeps_the_parsers_recovery {C : Type} (confined : C → List Confined)
    (tooNear : List String → Confined → Refusal) (file : List String) (rung : Layer)
    (refused : Refusal) :
    statedAt confined tooNear file rung (.error refused : Except Refusal C) =
      .error { refused with file := file } := by
  simp [statedAt]

/-- 一层的 `[model] name` 与 `[resident] harness`：`ConfigLayer::parse` 是唯一构造点，两键并存的值构造不出来（`one_resident`）。 -/
structure Seat where
  model : Option String
  harness : Option String
  one : model = none ∨ harness = none

/-! D6 落在这里：`settled_harness` 先看地址自己那一层有没有会话写下的 `[model] name`，有就答 `none`；没有才爬梯子取最近一级点名的 harness。决定、理由与被否的备选写在 `crates/city/spec/ConfigLayers.lean` 的 D6。 -/
def settledHarness (own : Seat) (rungs : List (Rung Seat)) : Option (String × Layer) :=
  match own.model with
  | some _ => none
  | none => (tagged rungs Seat.harness).resolve

theorem a_session_that_opened_on_a_model_keeps_it (own : Seat) (rungs : List (Rung Seat))
    (model : String) (recorded : own.model = some model) : settledHarness own rungs = none := by
  simp [settledHarness, recorded]

theorem without_a_record_the_nearest_harness_runs (own : Seat) (city building : Seat)
    (room : Option Seat) (unrecorded : own.model = none) :
    settledHarness own (ladder city building room) =
      nearest (ladder city building room) Seat.harness := by
  simp only [settledHarness, unrecorded]
  exact the_nearest_rung_that_speaks_wins city building room Seat.harness

end City.ConfigLayers.Ladder
