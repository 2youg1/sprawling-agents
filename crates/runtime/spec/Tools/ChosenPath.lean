-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::chosen_path 的模型

证明 `tools::chosen_path`（`crates/runtime/src/` 下同名的文件）必须守住的性质。模型选路的唯一判定：文法、保留区、读界，以及判的是盘打开的那个地址。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Tools.ChosenPath

/-- 读界对一个地址的答案（`kernel::address::ReadVerdict`）。`RulesUnreadable` 带的原因只进拒词，不进判定。 -/
inductive ReadVerdict where
  | Open
  | Confidential
  | RulesUnreadable
  deriving DecidableEq, Repr

/-- 拒绝的稳定码：`E_INVALID_ARGS`、`E_GATE_DENIED`、`E_STORAGE_FATAL`。 -/
inductive Code where
  | InvalidArgs
  | GateDenied
  | StorageFatal
  deriving DecidableEq, Repr

/-- 判一条模型选的路径要的三个事实。它们各有权威，都不在本 crate：文法是 `kernel::Address::parse`，保留区是 `Address::is_reserved`，读界是装配层把 `kernel::address::may_read` 闭合在读者的楼与城的规则上得到的 `ReadBound`（city-SPEC §8-2）。模型不重述它们，只把它们当参数，所以地址也是一个任意类型。 -/
structure Rules (Address : Type) where
  parse : String → Option Address
  is_reserved : Address → Bool
  bound : Address → ReadVerdict

/-- 一个已经解析的地址过后两道：保留区，然后读界。保留区在前，因为它不碰盘；读界可能为他楼读一次规则，排最后。 -/
def judge {Address : Type} (rules : Rules Address) (addr : Address) : Except Code Address :=
  if rules.is_reserved addr then .error .GateDenied
  else match rules.bound addr with
    | .Open => .ok addr
    | .Confidential => .error .GateDenied
    | .RulesUnreadable => .error .GateDenied

/-- `chosen_path::admit`：文法、保留区、读界，次序固定。 -/
def admit {Address : Type} (rules : Rules Address) (asked : String) : Except Code Address :=
  match rules.parse asked with
  | none => .error .InvalidArgs
  | some addr => judge rules addr

/-- 判过的地址就是交回的地址。 -/
theorem judge_answers_the_address_it_was_given {Address : Type} (rules : Rules Address)
    (addr judged : Address) (passed : judge rules addr = .ok judged) :
    judged = addr ∧ rules.is_reserved addr = false ∧ rules.bound addr = .Open := by
  unfold judge at passed
  cases reserved : rules.is_reserved addr with
  | true => simp [reserved] at passed
  | false =>
    cases verdict : rules.bound addr with
    | Open =>
      simp [reserved, verdict] at passed
      exact ⟨by rw [passed], rfl, rfl⟩
    | Confidential => simp [reserved, verdict] at passed
    | RulesUnreadable => simp [reserved, verdict] at passed

/-- **准入的地址读得通、不在保留区里、读界说开着。** -/
theorem an_admitted_path_is_open_and_outside_every_reserved_subtree {Address : Type}
    (rules : Rules Address) (asked : String) (addr : Address)
    (admitted : admit rules asked = .ok addr) :
    rules.parse asked = some addr ∧ rules.is_reserved addr = false ∧ rules.bound addr = .Open := by
  unfold admit at admitted
  cases parsed : rules.parse asked with
  | none => simp [parsed] at admitted
  | some found =>
    rw [parsed] at admitted
    have judged := judge_answers_the_address_it_was_given rules found addr admitted
    obtain ⟨same, notReserved, open_⟩ := judged
    subst same
    exact ⟨rfl, notReserved, open_⟩

/-- 读不出的路径是 `E_INVALID_ARGS`，与它可能指向哪里无关。 -/
theorem a_path_that_does_not_parse_is_invalid {Address : Type} (rules : Rules Address)
    (asked : String) (unparsed : rules.parse asked = none) :
    admit rules asked = .error .InvalidArgs := by
  simp [admit, unparsed]

/-- 保留区里的路径在读界被问之前就拒绝：不论读界怎么答，答案都是 `E_GATE_DENIED`。 -/
theorem a_reserved_path_is_refused_before_the_bound_is_asked {Address : Type}
    (rules : Rules Address) (asked : String) (addr : Address)
    (parsed : rules.parse asked = some addr) (reserved : rules.is_reserved addr = true)
    (bound : Address → ReadVerdict) :
    admit { rules with bound := bound } asked = .error .GateDenied := by
  simp [admit, judge, parsed, reserved]

/-- 盘对一个地址的回答：它的真实位置在城里的哪个地址（沿途的链接都解开），或在城外。 -/
inductive Landing (Address : Type) where
  | inside (real : Address)
  | outside

/-- `chosen_path::Located`：真实位置上有没有东西。判定时不在的，之后也不打开。 -/
inductive Located (Address : Type) where
  | Present (real : Address)
  | Absent (real : Address)

/-- 盘：`real_location` 求出的真实位置，以及那里此刻有没有东西。 -/
structure Disk (Address : Type) where
  real : Address → Landing Address
  present : Address → Bool

/-- `chosen_path::land`：判的是盘打开的那个地址，不只是模型写下的那个。真实位置落在城外是 `E_GATE_DENIED`；落在城里，再过一次同一道判定。 -/
def land {Address : Type} (rules : Rules Address) (disk : Disk Address) (addr : Address) :
    Except Code (Located Address) :=
  match disk.real addr with
  | .outside => .error .GateDenied
  | .inside real => match judge rules real with
    | .error code => .error code
    | .ok _ => .ok (if disk.present real then .Present real else .Absent real)

/-- **落下的地方也被判过。** `land` 交回的真实位置不在保留区里、读界说开着，不论模型写下的地址经过了什么链接。 -/
theorem what_lands_was_judged_where_it_lands {Address : Type} (rules : Rules Address)
    (disk : Disk Address) (addr : Address) (located : Located Address)
    (landed : land rules disk addr = .ok located) :
    ∃ real, disk.real addr = .inside real ∧ rules.is_reserved real = false ∧
      rules.bound real = .Open ∧
      located = (if disk.present real then .Present real else .Absent real) := by
  unfold land at landed
  cases where_ : disk.real addr with
  | outside => simp [where_] at landed
  | inside real =>
    cases judged : judge rules real with
    | error _ => simp [where_, judged] at landed
    | ok answer =>
      simp only [where_, judged] at landed
      obtain ⟨_, notReserved, open_⟩ :=
        judge_answers_the_address_it_was_given rules real answer judged
      have same := Except.ok.inj landed
      exact ⟨real, rfl, notReserved, open_, same.symm⟩

/-- 拿掉第二次判定就是一个反例：一个读界开着的地址，经一条链接落进一栋机密楼。只判字面地址，就等于把 `admit` 拒掉的东西从侧门交出去；`land` 拒绝它。 -/
theorem judging_only_the_written_address_lets_a_link_out :
    let rules : Rules Bool :=
      { parse := fun _ => some true, is_reserved := fun _ => false,
        bound := fun building => if building then .Open else .Confidential }
    let disk : Disk Bool := { real := fun _ => .inside false, present := fun _ => true }
    admit rules "open/link" = .ok true ∧ land rules disk true = .error .GateDenied := by
  intro rules disk
  exact ⟨rfl, rfl⟩

end Runtime.Tools.ChosenPath
