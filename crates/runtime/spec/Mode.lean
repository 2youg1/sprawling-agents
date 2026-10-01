-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::mode 的模型

证明 `mode`（`crates/runtime/src/` 下同名的文件）必须守住的性质。每个 mode 在目录里怎么介绍，以及一次 run 的产出准不准合并。runtime 的规格仍是 `crates/runtime/runtime-SPEC.md`，它的 §16 引本分部作为这些性质的权威。
-/

namespace Runtime.Mode

/-- 一次派活的产出走不走常规的路（`kernel::LandingPolicy`）。 -/
inductive LandingPolicy where
  | Ordinary
  | Experiment
  deriving DecidableEq, Repr

/-- 一次派活的产出要带什么证据才合并（`kernel::AdmissionRequirement`）。 -/
inductive AdmissionRequirement where
  | Standing
  | Tested
  | ContractKept
  | DoubleValidated
  deriving DecidableEq, Repr

/-- `mode::Produced`：合并那一刻城读得到的证据。`Option Bool` 的 `none` 是「没量过」，不是「没过」。 -/
structure Produced where
  tests_passed : Option Bool
  contract_moved : Bool
  held_in : Option Bool
  held_out : Option Bool
  deriving DecidableEq, Repr

/-- 每一种拒绝各有一句拒词（Rust 里是 `because` 与 `alternative` 两句）；这里按拒因区分。 -/
inductive Refusal where
  | ExperimentNeverLands
  | TestsFailed
  | NoTests
  | ContractMoved
  | WorseHeldIn
  | FailedHeldOut
  | HalfMissing
  deriving DecidableEq, Repr

/-- `mode::Admission`。 -/
inductive Admission where
  | Lands
  | Refused (because : Refusal)
  deriving DecidableEq, Repr

/-- `mode::admits_evidence`：常规落地时按证据要求判。 -/
def admits_evidence (required : AdmissionRequirement) (produced : Produced) : Admission :=
  match required with
  | .Standing => .Lands
  | .Tested => match produced.tests_passed with
    | some true => .Lands
    | some false => .Refused .TestsFailed
    | none => .Refused .NoTests
  | .ContractKept => if produced.contract_moved then .Refused .ContractMoved else .Lands
  | .DoubleValidated => match produced.held_in, produced.held_out with
    | some true, some true => .Lands
    | some false, _ => .Refused .WorseHeldIn
    | _, some false => .Refused .FailedHeldOut
    | _, _ => .Refused .HalfMissing

/-- `mode::admits`：先看落地策略，再看证据要求。mode 不是参数：交谈与干活产出的东西走同一道合并（kernel-SPEC §12.12）。 -/
def admits (landing : LandingPolicy) (required : AdmissionRequirement) (produced : Produced) :
    Admission :=
  match landing with
  | .Experiment => .Refused .ExperimentNeverLands
  | .Ordinary => admits_evidence required produced

/-- 试验的产出从不合并，不论它带了什么证据。 -/
theorem an_experiment_never_lands (required : AdmissionRequirement) (produced : Produced) :
    admits .Experiment required produced = .Refused .ExperimentNeverLands := rfl

/-- 常规落地时每一种证据要求要的东西：恰是这些在场时合并。没量过的与量了没过的同样不合并。 -/
def evidenced (required : AdmissionRequirement) (produced : Produced) : Bool :=
  match required with
  | .Standing => true
  | .Tested => produced.tests_passed == some true
  | .ContractKept => !produced.contract_moved
  | .DoubleValidated => produced.held_in == some true && produced.held_out == some true

/-- **合并当且仅当常规落地且证据在场。** -/
theorem lands_exactly_with_its_evidence (landing : LandingPolicy) (required : AdmissionRequirement)
    (tests : Option Bool) (contract : Bool) (heldIn heldOut : Option Bool) :
    admits landing required ⟨tests, contract, heldIn, heldOut⟩ = .Lands ↔
      (landing = .Ordinary ∧ evidenced required ⟨tests, contract, heldIn, heldOut⟩ = true) := by
  cases landing <;> cases required <;> rcases tests with _ | ⟨_ | _⟩ <;>
    rcases contract with _ | _ <;> rcases heldIn with _ | ⟨_ | _⟩ <;>
    rcases heldOut with _ | ⟨_ | _⟩ <;> decide

/-- 一次没跑测试的 `work`＋`tested` 不合并，拒因是「没有自己的测试」，而不是「测试没过」。 -/
example : admits .Ordinary .Tested ⟨none, false, none, none⟩ = .Refused .NoTests := rfl

end Runtime.Mode
