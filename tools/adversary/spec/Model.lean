-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 欠哪一种拒绝：守序

规定检验器 `Sprawling.Model` 的 `refusal` 必须守的性质（`tools/adversary/src/Sprawling/Model.lean`）：
一个没挂 provider、或挂了 provider 的城，在停摆、地址保留、楼已立这几个事实上，对立楼与派活
各欠哪一种失败。这里只谈**哪一种失败**，从不谈任何被计算出来的值：错误码是门对调用方的承诺
（`AGENTS.md`），钉住它钉的是承诺，而不是对某条规则的重算。

事实以布尔值进来，而不是以地址进来：一个地址是不是保留的、一栋楼是否立着、一个范围是否
停摆，权威在产品（`kernel::address`、城的视图），模型只问它们怎样排序。

**三条守序是量出来的**（`Spec.lean` §10）：停摆压过配置；对立楼，地址良构压过占用；对派活，
停摆压过地址良构。停摆是派活最外层的门，而立楼不是派活，所以停摆盖不住立楼——这一条在类型上
就成立：`owedOnRaise` 根本不读停摆。
-/

namespace Adversary.Model

/-- 模型断言的几种稳定错误码。只列模型会欠的那几个，全集的权威在 `kernel::AxCode::ALL`。 -/
inductive Code where
  /-- `E_GATE_DENIED` -/
  | gateDenied
  /-- `E_INVALID_ARGS` -/
  | invalidArgs
  /-- `E_MODEL_UNCHOSEN` -/
  | modelUnchosen
  /-- `E_WIRE_MISMATCH` -/
  | wireMismatch
deriving DecidableEq, Repr

/-- 立楼欠什么：保留地址先于占用。 -/
def owedOnRaise (reserved held : Bool) : Option Code :=
  if reserved then some .invalidArgs
  else if held then some .invalidArgs
  else none

/-- 派活欠什么：停摆先于地址，地址先于配置。 -/
def owedOnWork (halted reserved attached : Bool) : Option Code :=
  if halted then some .gateDenied
  else if reserved then some .invalidArgs
  else if !attached then some .modelUnchosen
  else none

/-- 一个线上拼得出、城不执行的动词：恒拒，码是线的。 -/
def owedOnBatch : Option Code := some .wireMismatch

/-- 停摆压过一切派活上的别的原因：一个把它与配置调换的实现，会在城停摆时叫人去挂 provider。 -/
theorem a_halt_answers_first (reserved attached : Bool) :
    owedOnWork true reserved attached = some .gateDenied := rfl

/-- 保留地址压过配置：恢复建议指向地址，而不是去挂 provider。 -/
theorem a_reserved_address_answers_before_configuration (attached : Bool) :
    owedOnWork false true attached = some .invalidArgs := rfl

/-- 保留地址压过占用。 -/
theorem a_reserved_address_answers_before_occupancy (held : Bool) :
    owedOnRaise true held = some .invalidArgs := rfl

/-- 每一种被欠的失败都是一种拒绝，而派活在挂了 provider、没停摆、地址普通时不欠任何失败：
模型不会对每个动作都说「要失败」，否则它什么也没检查。 -/
theorem work_is_owed_success_when_nothing_stands_in_the_way :
    owedOnWork false false true = none := rfl

/-- 一条命令被拒，它的幂等键也花掉了；只有查询不花键（`Spec.lean` §4 第三个发现）。

`spent` 数一串结果里花掉的键：命令无论接受还是被拒都数一次。 -/
inductive Outcome where
  | commandAccepted
  | commandRefused
  | query
deriving DecidableEq, Repr

def spends : Outcome → Bool
  | .commandAccepted | .commandRefused => true
  | .query => false

def spent (outcomes : List Outcome) : Nat := (outcomes.filter spends).length

/-- 下一把键与已经答过的键不同：被拒的命令同样把计数往前推。 -/
theorem a_refused_command_spends_its_key (outcomes : List Outcome) :
    spent (outcomes ++ [.commandRefused]) = spent outcomes + 1 := by
  simp only [spent, List.filter_append, List.length_append]
  rfl

/-- 查询不花键。 -/
theorem a_query_spends_nothing (outcomes : List Outcome) :
    spent (outcomes ++ [.query]) = spent outcomes := by
  simp only [spent, List.filter_append, List.length_append]
  rfl

end Adversary.Model
