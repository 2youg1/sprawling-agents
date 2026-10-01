-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.runtime.spec.Tools.ChosenPath

/-!
# runtime::tools::bound_reader 的模型

证明 `tools::bound_reader`（`crates/runtime/src/` 下同名的文件）必须守住的性质。模型点名的字节，经读界判过后按字节读的那一扇门。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Tools.BoundReader

open Runtime.Tools.ChosenPath

/-- 模型写下的参数，按门怎么读它分三种：一条城内路径、一个 `cas:` 块（以它的哈希识别）、一个 `file:` Locator（`<addr>@<oid>`）。哪一种由前缀定：以 `cas:` 或 `file:` 开头的是 Locator，其余是路径，城内地址不含冒号，所以两者不相交。 -/
inductive Asked where
  | path (written : String)
  | cas (hash : Nat)
  | file (written : String)

/-- `bound_reader::Named`：字节从哪里来。 -/
inductive Named (Address : Type) where
  | File (addr : Address)
  | Block (hash : Nat)

/-- 块仓记下的来源：一个块为哪几栋楼存过（`Cas::origins`，storage-SPEC §8-3）。 -/
structure Store (Address : Type) where
  origins : Nat → List Address

/-- 来源里第一栋读界开着的楼。 -/
def first_open {Address : Type} (rules : Rules Address) : List Address → Option Address
  | [] => none
  | building :: rest => match rules.bound building with
    | .Open => some building
    | .Confidential => first_open rules rest
    | .RulesUnreadable => first_open rules rest

/-- `read::locator::judged_at`：`cas:` 块按哪栋楼判读取界的唯一判定处。取读者能读的第一栋；一栋都读不了就取第一条来源，让判定按那栋楼的理由拒绝；没有来源的块拒绝。 -/
def judged_at {Address : Type} (rules : Rules Address) (origins : List Address) :
    Except Code Address :=
  match first_open rules origins with
  | some building => .ok building
  | none => match origins with
    | [] => .error .GateDenied
    | first :: _ => .ok first

/-- `BoundReader::open`：判定是 `read` 的那一套，次序也一样。路径走 `admit` 再走 `land`，判定时不在的不打开；`file:` 按自己的地址过 `admit`，读的是那一次提交里的字节；`cas:` 按 `judged_at` 选出的楼过同一道判定。`open` 在 Lean 里是关键字，故写作 `«open»`。 -/
def «open» {Address : Type} (rules : Rules Address) (disk : Disk Address)
    (store : Store Address) : Asked → Except Code (Named Address)
  | .cas hash => match judged_at rules (store.origins hash) with
    | .error code => .error code
    | .ok building => match judge rules building with
      | .error code => .error code
      | .ok _ => .ok (.Block hash)
  | .file written => match admit rules written with
    | .error code => .error code
    | .ok addr => .ok (.File addr)
  | .path written => match admit rules written with
    | .error code => .error code
    | .ok addr => match land rules disk addr with
      | .error code => .error code
      | .ok (.Present _) => .ok (.File addr)
      | .ok (.Absent _) => .error .InvalidArgs

/-- `first_open` 找到的楼是来源之一，而且读界开着。 -/
theorem first_open_is_an_open_origin {Address : Type} (rules : Rules Address) :
    ∀ (origins : List Address) (building : Address),
      first_open rules origins = some building →
        building ∈ origins ∧ rules.bound building = .Open
  | [], _, found => by simp [first_open] at found
  | head :: rest, building, found => by
    cases verdict : rules.bound head with
    | Open =>
      simp [first_open, verdict] at found
      subst found
      exact ⟨by simp, verdict⟩
    | Confidential =>
      simp [first_open, verdict] at found
      have later := first_open_is_an_open_origin rules rest building found
      exact ⟨by simp [later.1], later.2⟩
    | RulesUnreadable =>
      simp [first_open, verdict] at found
      have later := first_open_is_an_open_origin rules rest building found
      exact ⟨by simp [later.1], later.2⟩

/-- `judged_at` 选的楼恒是这个块的来源之一。 -/
theorem judged_at_names_an_origin {Address : Type} (rules : Rules Address)
    (origins : List Address) (building : Address)
    (chosen : judged_at rules origins = .ok building) : building ∈ origins := by
  unfold judged_at at chosen
  cases found : first_open rules origins with
  | some open_ =>
    simp [found] at chosen
    subst chosen
    exact (first_open_is_an_open_origin rules origins open_ found).1
  | none =>
    simp [found] at chosen
    cases origins with
    | nil => simp at chosen
    | cons first rest =>
      simp at chosen
      subst chosen
      simp

/-- **一个块只在为它存过的、读界开着的楼里读得到。** 归属按存块时记下的来源判，不按账本里谁写了这个哈希判：模型写的文字会落进带地址的行，那样的归属可以伪造。 -/
theorem a_block_is_read_only_at_a_building_it_was_put_for {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (hash : Nat)
    (named : Named Address) (opened : «open» rules disk store (.cas hash) = .ok named) :
    ∃ building ∈ store.origins hash,
      rules.is_reserved building = false ∧ rules.bound building = .Open := by
  simp only [«open»] at opened
  cases chosen : judged_at rules (store.origins hash) with
  | error _ => simp [chosen] at opened
  | ok building =>
    cases judged : judge rules building with
    | error _ => simp [chosen, judged] at opened
    | ok answer =>
      obtain ⟨_, notReserved, open_⟩ :=
        judge_answers_the_address_it_was_given rules building answer judged
      exact ⟨building, judged_at_names_an_origin rules _ building chosen, notReserved, open_⟩

/-- 存块时没有记下任何楼的块，`E_GATE_DENIED`。 -/
theorem a_block_put_for_no_building_is_refused {Address : Type} (rules : Rules Address)
    (disk : Disk Address) (store : Store Address) (hash : Nat)
    (unclaimed : store.origins hash = []) :
    «open» rules disk store (.cas hash) = .error .GateDenied := by
  simp [«open», judged_at, first_open, unclaimed]

/-- **门对一条路径的拒绝与 `read` 同码。** `read` 拒在 `admit` 上的，门以同一个码拒；模型读到的是同一句话，只是 action 换成了调用它的那件工具。 -/
theorem a_path_read_refuses_is_refused_with_the_same_code {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (written : String)
    (code : Code) (refused : admit rules written = .error code) :
    «open» rules disk store (.path written) = .error code ∧
      «open» rules disk store (.file written) = .error code := by
  simp [«open», refused]

/-- **按路径打开的文件，是在它真实落下的地方被判过、判定时在场的那一个。** -/
theorem a_file_opened_by_path_was_judged_where_it_lands {Address : Type}
    (rules : Rules Address) (disk : Disk Address) (store : Store Address) (written : String)
    (addr : Address) (opened : «open» rules disk store (.path written) = .ok (.File addr)) :
    admit rules written = .ok addr ∧
      ∃ real, disk.real addr = .inside real ∧ rules.is_reserved real = false ∧
        rules.bound real = .Open ∧ disk.present real = true := by
  simp only [«open»] at opened
  cases admitted : admit rules written with
  | error _ => simp [admitted] at opened
  | ok found =>
    cases landed : land rules disk found with
    | error _ => simp [admitted, landed] at opened
    | ok located =>
      obtain ⟨real, where_, notReserved, open_, shape⟩ :=
        what_lands_was_judged_where_it_lands rules disk found located landed
      cases located with
      | Present _ =>
        simp [admitted, landed] at opened
        subst opened
        cases present : disk.present real with
        | true => exact ⟨rfl, real, where_, notReserved, open_, present⟩
        | false => simp [present] at shape
      | Absent _ => simp [admitted, landed] at opened

end Runtime.Tools.BoundReader
