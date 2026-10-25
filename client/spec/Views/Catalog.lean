-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# catalog：能力节的草稿

规定 `client/src/views/building/catalog.ts` 的 `pressRow`、`pressGroup`、`effectiveOf` 与 `meterOf`，以及 `catalog.svelte` 保存与被拒时怎样对待草稿（`client/Spec.lean` 4-50 其十）。TypeScript 是「怎样守住」的权威，本模型是「必须守住哪些性质」的权威；`catalog.test.ts` 逐条回放本文件末尾 `#eval` 打印的编辑序列。

能力节一次编辑一级表：本楼那一级，或全城那一级（`crates/wire/Spec.lean` §8-99）。草稿就是那一级的整张 `[catalog]` 表；保存把整张表发给城。一行的档位按城的读法算（`kernel::config::CatalogExposure::stated`）：固定的那两件恒为常驻；更近一级说了的（编辑全城那一级时，本楼那一级的表）压过草稿；否则先看草稿的工具键、再看草稿的命名空间键，都不说时取这一行的「继承值」——编辑本楼时是城那一级与缺省给它的，编辑全城时是缺省。继承值与更近一级的值由页面从 `CatalogAnswer` 算好放进行里，本模型把它们当作行的已知字段。

模型里工具与命名空间都写成自然数。

性质：

1. **一行只在与继承值不同时写下**（`a_row_is_written_only_when_it_departs`）：按下一行的某一档之后，草稿里这一行的工具键在，当且仅当那一档不同于去掉工具键时这一行的档位；设回继承值就是删掉覆盖，而不是写一份相同的值。
2. **按下哪一档，这一行就显示哪一档**（`a_pressed_row_shows_what_was_pressed`）：没有更近一级压着、又不是固定的一行，按下之后的档位就是按下的那一档。
3. **固定行不动**（`a_fixed_row_never_moves`）：固定的一行在任何草稿下都是常驻，按它不改草稿。
4. **整组按下，组里每一行都显示那一档**（`a_group_press_moves_every_member`）：没有更近一级压着的非固定成员，档位都成了按下的那一档。
5. **计量是移入常驻的字节减去移出的字节**（`the_meter_is_moved_in_minus_moved_out`）。
6. **被拒之后草稿不变**（`a_refusal_keeps_the_draft`）：城拒了这次保存，页面留着草稿；城收下了，已保存的就是发出的那一份。
-/

namespace Client.Views.Catalog

/-- 三档：常驻、按需、关。 -/
inductive Exposure where
  | core
  | dormant
  | off
  deriving DecidableEq, Repr

/-- 一行：工具、它的命名空间、它的定义的字节、固定与否、去掉这一级的表时它的档位，以及更近一级说了的档位。 -/
structure Row where
  name : Nat
  ns : Nat
  bytes : Nat
  fixed : Bool
  inherited : Exposure
  nearer : Option Exposure
  deriving DecidableEq, Repr

/-- 一级 `[catalog]` 表：工具键与命名空间键，各是一张键到档位的表。 -/
structure Table where
  tools : List (Nat × Exposure)
  namespaces : List (Nat × Exposure)
  deriving DecidableEq, Repr

def find (k : Nat) : List (Nat × Exposure) → Option Exposure
  | [] => none
  | (k', v) :: rest => if k = k' then some v else find k rest

def drop (k : Nat) : List (Nat × Exposure) → List (Nat × Exposure)
  | [] => []
  | (k', v) :: rest => if k = k' then drop k rest else (k', v) :: drop k rest

def put (k : Nat) (v : Exposure) (m : List (Nat × Exposure)) : List (Nat × Exposure) :=
  (k, v) :: drop k m

theorem find_drop (k : Nat) : ∀ m : List (Nat × Exposure), find k (drop k m) = none
  | [] => rfl
  | (k', v) :: rest => by
    have ih := find_drop k rest
    by_cases h : k = k'
    · subst h
      simpa [drop] using ih
    · simp [drop, find, h, ih]

theorem find_drop_other (k j : Nat) (h : k ≠ j) :
    ∀ m : List (Nat × Exposure), find k (drop j m) = find k m
  | [] => rfl
  | (k', v) :: rest => by
    by_cases hj : j = k'
    · subst hj
      simp [drop, find, h, find_drop_other k j h rest]
    · by_cases hk : k = k'
      · simp [drop, find, hj, hk]
      · simp [drop, find, hj, hk, find_drop_other k j h rest]

theorem find_put (k : Nat) (v : Exposure) (m : List (Nat × Exposure)) : find k (put k v m) = some v := by
  simp [put, find]

/-- 不看这一行的工具键时它的档位：草稿的命名空间键，或继承值。 -/
def without (t : Table) (r : Row) : Exposure :=
  (find r.ns t.namespaces).getD r.inherited

/-- 草稿 `t` 下这一行的档位。 -/
def effective (t : Table) (r : Row) : Exposure :=
  if r.fixed then .core
  else match r.nearer with
    | some v => v
    | none => (find r.name t.tools).getD (without t r)

/-- 按下一行的一档：与去掉工具键时的档位相同就删掉工具键，否则写下它；固定行不动。 -/
def pressRow (t : Table) (r : Row) (v : Exposure) : Table :=
  if r.fixed then t
  else if v = without t r then { t with tools := drop r.name t.tools }
  else { t with tools := put r.name v t.tools }

/-- 整组按下一档：删掉组里每个非固定成员的工具键；成员的继承值都是这一档时删掉命名空间键，否则写下它。 -/
def pressGroup (rows : List Row) (t : Table) (ns : Nat) (v : Exposure) : Table :=
  let members := rows.filter (fun r => r.ns = ns && !r.fixed)
  { tools := members.foldl (fun acc r => drop r.name acc) t.tools
    namespaces :=
      if members.all (fun r => r.inherited = v) then drop ns t.namespaces
      else put ns v t.namespaces }

theorem a_row_is_written_only_when_it_departs (t : Table) (r : Row) (v : Exposure) (h : r.fixed = false) :
    find r.name (pressRow t r v).tools = if v = without t r then none else some v := by
  unfold pressRow
  by_cases hv : v = without t r
  · simp [h, hv, find_drop]
  · simp [h, hv, find_put]

theorem pressRow_namespaces (t : Table) (r : Row) (v : Exposure) :
    (pressRow t r v).namespaces = t.namespaces := by
  unfold pressRow
  split
  · rfl
  · split <;> rfl

theorem a_pressed_row_shows_what_was_pressed (t : Table) (r : Row) (v : Exposure)
    (h : r.fixed = false) (free : r.nearer = none) :
    effective (pressRow t r v) r = v := by
  have hw : without (pressRow t r v) r = without t r := by
    simp only [without, pressRow_namespaces]
  unfold effective
  rw [a_row_is_written_only_when_it_departs t r v h, hw]
  by_cases hv : v = without t r <;> simp [h, free, hv]

theorem a_fixed_row_never_moves (t : Table) (r : Row) (v : Exposure) (h : r.fixed = true) :
    effective t r = .core ∧ pressRow t r v = t := by
  simp [effective, pressRow, h]

theorem find_after_drops (k : Nat) :
    ∀ (members : List Row) (acc : List (Nat × Exposure)), (∃ r ∈ members, r.name = k) →
      find k (members.foldl (fun acc r => drop r.name acc) acc) = none
  | [], _, ⟨_, hr, _⟩ => absurd hr (by simp)
  | r :: rest, acc, ⟨s, hs, hk⟩ => by
    simp only [List.foldl_cons]
    by_cases hin : ∃ q ∈ rest, q.name = k
    · exact find_after_drops k rest (drop r.name acc) hin
    · have hsr : s = r := by
        rcases List.mem_cons.mp hs with h | h
        · exact h
        · exact absurd ⟨s, h, hk⟩ hin
      subst hsr
      have keep : ∀ (rest : List Row) (acc : List (Nat × Exposure)), (¬ ∃ q ∈ rest, q.name = k) →
          find k acc = none → find k (rest.foldl (fun acc r => drop r.name acc) acc) = none := by
        intro rest
        induction rest with
        | nil => intro acc _ h; simpa using h
        | cons q more ih =>
          intro acc hnot hacc
          simp only [List.foldl_cons]
          apply ih
          · intro ⟨p, hp, hpk⟩; exact hnot ⟨p, List.mem_cons_of_mem _ hp, hpk⟩
          · have hq : k ≠ q.name := by
              intro e; exact hnot ⟨q, List.mem_cons_self .., e.symm⟩
            rw [find_drop_other k q.name hq]; exact hacc
      apply keep rest (drop s.name acc) hin
      rw [← hk]; exact find_drop s.name acc

theorem a_group_press_moves_every_member (rows : List Row) (t : Table) (ns : Nat) (v : Exposure)
    (r : Row) (hr : r ∈ rows) (hns : r.ns = ns) (h : r.fixed = false) (free : r.nearer = none) :
    effective (pressGroup rows t ns v) r = v := by
  have member : r ∈ rows.filter (fun r => r.ns = ns && !r.fixed) := by
    simp [List.mem_filter, hr, hns, h]
  have gone : find r.name (pressGroup rows t ns v).tools = none := by
    unfold pressGroup
    exact find_after_drops r.name _ t.tools ⟨r, member, rfl⟩
  unfold effective without
  rw [gone]
  simp only [h, free, Bool.false_eq_true, ↓reduceIte, Option.getD_none]
  unfold pressGroup
  by_cases hall : (rows.filter (fun r => r.ns = ns && !r.fixed)).all (fun r => r.inherited = v) = true
  · simp only [hall, ↓reduceIte]
    rw [hns, find_drop]
    have := List.all_eq_true.mp hall r member
    simpa using this
  · simp only [hall, Bool.false_eq_true, ↓reduceIte]
    rw [hns, find_put]
    rfl

/-- 一行在草稿 `t` 下是否常驻。 -/
def inCore (t : Table) (r : Row) : Bool := effective t r = .core

/-- 计量：草稿下常驻的字节减去已保存的表下常驻的字节。 -/
def meter (saved draft : Table) : List Row → Int
  | [] => 0
  | r :: rest =>
    ((if inCore draft r then (r.bytes : Int) else 0) - (if inCore saved r then (r.bytes : Int) else 0))
      + meter saved draft rest

/-- 移入常驻的行的字节和。 -/
def movedIn (saved draft : Table) (rows : List Row) : Int :=
  ((rows.filter (fun r => inCore draft r && !inCore saved r)).map (fun r => (r.bytes : Int))).sum

/-- 移出常驻的行的字节和。 -/
def movedOut (saved draft : Table) (rows : List Row) : Int :=
  ((rows.filter (fun r => inCore saved r && !inCore draft r)).map (fun r => (r.bytes : Int))).sum

theorem the_meter_is_moved_in_minus_moved_out (saved draft : Table) :
    ∀ rows : List Row, meter saved draft rows = movedIn saved draft rows - movedOut saved draft rows
  | [] => by simp [meter, movedIn, movedOut]
  | r :: rest => by
    have ih := the_meter_is_moved_in_minus_moved_out saved draft rest
    simp only [meter, movedIn, movedOut] at ih ⊢
    cases hd : inCore draft r <;> cases hs : inCore saved r <;>
      simp [hd, hs] <;> omega

/-- 保存走到哪一步。 -/
inductive Phase where
  | idle
  | sending (sent : Table)
  | refused
  deriving DecidableEq, Repr

/-- 能力节的状态：草稿、已保存的表与保存走到哪一步。 -/
structure State where
  draft : Table
  saved : Table
  phase : Phase
  deriving DecidableEq, Repr

/-- 保存的三件事：发出、城收下、城拒了。 -/
inductive SaveEvent where
  | send
  | accept
  | refuse
  deriving DecidableEq, Repr

def save (s : State) : SaveEvent → State
  | .send => { s with phase := .sending s.draft }
  | .accept => match s.phase with
    | .sending sent => { s with saved := sent, phase := .idle }
    | .idle | .refused => s
  | .refuse => match s.phase with
    | .sending _ => { s with phase := .refused }
    | .idle | .refused => s

theorem a_refusal_keeps_the_draft (s : State) (sent : Table) (h : s.phase = .sending sent) :
    (save s .refuse).draft = s.draft ∧ (save s .refuse).saved = s.saved := by
  simp [save, h]

theorem an_acceptance_saves_what_was_sent (s : State) (sent : Table) (h : s.phase = .sending sent) :
    (save s .accept).saved = sent := by
  simp [save, h]

/-- 回放用的一次编辑：按一行，或按一组。 -/
inductive Edit where
  | row (name : Nat) (v : Exposure)
  | group (ns : Nat) (v : Exposure)
  deriving DecidableEq, Repr

def apply (rows : List Row) (t : Table) : Edit → Table
  | .row name v => match rows.find? (fun r => r.name = name) with
    | some r => pressRow t r v
    | none => t
  | .group ns v => pressGroup rows t ns v

/-- 回放的夹具：两个命名空间，命名空间 0 有一件缺省常驻（如 `edit`）与一件缺省按需，命名空间 1 有一件缺省按需与一件固定行（如 `read`）；第五行被更近一级压成关。 -/
def rows : List Row :=
  [ ⟨0, 0, 100, false, .core, none⟩,
    ⟨1, 0, 50, false, .dormant, none⟩,
    ⟨2, 1, 30, false, .dormant, none⟩,
    ⟨3, 1, 20, true, .core, none⟩,
    ⟨4, 1, 40, false, .dormant, some .off⟩ ]

def empty : Table := ⟨[], []⟩

def vectors : List (List Edit) :=
  [ [.row 1 .core],
    [.row 1 .core, .row 1 .dormant],
    [.row 0 .off, .row 3 .off],
    [.group 1 .core],
    [.group 1 .core, .row 2 .dormant],
    [.group 0 .off, .row 0 .core],
    [.group 1 .dormant] ]

/-- 一个序列的结果：工具键与命名空间键（按写下的次序），每行的档位，与相对空表的计量。 -/
def replay (edits : List Edit) : Table × List Exposure × Int :=
  let t := edits.foldl (apply rows) empty
  (t, rows.map (effective t), meter empty t rows)

#eval vectors.map fun edits => (edits, replay edits)

end Client.Views.Catalog
