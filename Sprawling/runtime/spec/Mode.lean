-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::mode

规定 `mode`（`crates/runtime/src/` 下同名的文件）。每个 mode 在目录里怎么介绍，以及一次 run 的产出准不准合并。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-12 runtime::mode（形状 6；dev 入口）


```rust
pub const DEV_ENTRY: &str = "dev";
pub fn dev_entry() -> CatalogEntry;   // 一行披露，全部细则归 expansion
```

- **一个 Run 只被告知它所在的那个 mode**，于是没有任何 Agent 知道这座城自己的代码与 SPEC 是可改的。`dev` 行补上这一句，**而且只补一句**：三种准入证据要求与两种落地策略的意思、阅读次序（SPEC → 代码 → 旁边的测试）与「下一步去跟人要它们」全在 expansion 里，由 `read` 按需取。**大多数会话不改这座城，就只付一行的价。**
-/

/-!
### 8-12b runtime::mode 原有面


```rust
pub fn catalog_entry(mode: kernel::Mode) -> CatalogEntry;     // chat 与 work 两行
```

`Chat` 的目录行只有一句：专心同人交谈，就对方说的话作答。除这一行提示之外它什么也不做，它存在的理由是让一句闲话不被当成一件要做的活。`Work` 的目录行说：朝人给的目标干活，任务要一份计划时先用 `plan` 工具写进 `Roadmap.md`，目标达成时报告。

哪些 mode 存在、各自拼成什么词，只由 `kernel::Mode` 回答（线、账本都读它）；本模块只持每个 mode 在目录里怎么介绍、以及一次 run 的产出准不准合并（§8-54）。runtime 不再有自己的 `Mode`：两份同成员的枚举要靠装配层一个恒等的 `match` 维系，新增一个 mode 时那是第二处必须同步改的地方。
-/

/-!
### 8-54 合并时的准入，按运行策略判（`runtime::mode::admits`，形状 1 判定）


```rust
pub struct Produced { pub tests_passed: Option<bool>, pub contract_moved: bool,
                      pub held_in: Option<bool>, pub held_out: Option<bool> }
pub enum Admission { Lands, Refused { because: &'static str, alternative: &'static str } }
pub fn admits(policy: &kernel::RunPolicy, produced: &Produced) -> Admission;
```

- **判定序**：先看落地策略——`Experiment` 恒 `Refused`（试验的产出不合并，学到的写进 `Memo.md`，换一次常规落地的派活再做）；`Ordinary` 再看准入证据要求：`Standing` 恒 `Lands`（楼自己的规矩已经在别处判过，本函数不加检查）；`Tested` 要 `tests_passed == Some(true)`，`Some(false)` 与 `None` 各有自己的拒词；`ContractKept` 在 `contract_moved` 时拒；`DoubleValidated` 要 held-in 与 held-out 两半都是 `Some(true)`，缺一半与任一半为 `Some(false)` 各有拒词。
- **判的是 run 结束时格里的策略**：会话中换过运行策略的 run（§8-62），合并时读它最后一次在 `BeforeWave` 取用的那一份，即 `PolicyCell` 在 run 冻结时的值；没被它取用的改动属于下一个 run。
- **mode 不参与准入**：交谈与干活产出的东西走同一道合并，要不要证据由证据要求一个值回答（kernel D12）。
- **唯一的调用方是合并那一刻**：`accounting::worker::reviewing` 在 `PrEffect::Merged` 写 `pr_merged` 之前问它，`Refused` 写 `pr_rejected`，理由是 `because; alternative` 两句（`crates/sprawling/Spec.lean` §8-133）。评审说「另一位居民看过」，准入说「这次派活要的证据在」，两个问题两道门。
- **`ContractKept` 今天以城看不见的方式成立**：城读不出一个契约动没动，`Produced.contract_moved` 由装配层恒填 `false`，所以这一要求只在 run 自己报出契约动了的那一天才会拒。这一点照旧写在 §3 而不是假装已经量过。
- 验收：`mode` 测试 `a_work_run_without_the_evidence_it_chose_does_not_land`（`work`＋`tested`、没跑测试 → `Refused`），以及每种要求、每种落地各自的拒与放。
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

/-- `mode::admits`：先看落地策略，再看证据要求。mode 不是参数：交谈与干活产出的东西走同一道合并（kernel D12）。 -/
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
