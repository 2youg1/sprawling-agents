-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.storage.spec.ChainAudit

/-!
# 服务中的城在后台证明整条链

规定 `crates/sprawling/src/assembly/chain_watch.rs` 的 `audit_in_background`（`bin::assembly::chain_watch`，形状：adapter；sprawling-SPEC.md 8-90）把证明线程的结局交给停机值的那一处。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。停机值本身（判定之前拒绝追加、第一个判定有效）的权威是 `crates/storage/spec/ChainAudit.lean`，这里只引用它。

证明线程有四种结局：链完好、链断了、读账本本身失败、线程没给出判定就结束（恐慌；它持有的守卫在退栈时跳闸）。只有第一种让写者放行；其余三种都跳闸，因为没读完的证明既没有证明链断了，也没有证明它完好，而写在一条未经证明的链后面的行与写在断链后面的行一样收不回来。

两条性质：

* **每种结局都给出判定**——关城时写者 `await_proof` 等的是停机值有判定，所以关城从不永远等下去，此后写者也不再答 `Unproven`；
* **只有完好才放行**——其余结局之后写者答断链的原因。
-/

namespace Sprawling.Assembly.ChainWatch

open Storage.ChainAudit

/-- 证明线程的结局。`Reason` 是原因的类型（Rust 里是一个 `AxError`）；`unfinished` 是守卫在退栈时给出的那一条原因。 -/
inductive Outcome (Reason : Type) where
  | whole
  | broken (reason : Reason)
  | unreadable (reason : Reason)
  | panicked
  deriving Repr, DecidableEq

variable {Reason : Type} (unfinished : Reason)

/-- 结局对停机值的那一次调用：完好 `prove`，其余 `trip`。 -/
def Outcome.call : Outcome Reason → Call Reason
  | .whole => .prove
  | .broken r => .trip r
  | .unreadable r => .trip r
  | .panicked => .trip unfinished

/-- 服务中的城的停机值：判定之前拒绝（`ChainHalt::awaiting_proof()`）。 -/
def awaiting : ChainHalt Reason := ⟨none, .Refuse⟩

/-- 每种结局之后停机值都有判定，写者不再答 `Unproven`。 -/
theorem every_outcome_settles (o : Outcome Reason) :
    admit (calls awaiting [o.call unfinished]) ≠ .Unproven := by
  cases o <;> simp [Outcome.call, calls, settle, awaiting, admit, Call.verdict]

/-- 只有链完好才放行。 -/
theorem only_whole_admits (o : Outcome Reason) :
    admit (calls awaiting [o.call unfinished]) = .Admitted ↔ o = .whole := by
  cases o <;> simp [Outcome.call, calls, settle, awaiting, admit, Call.verdict]

/-- 证明之前写者不写：还没有结局时答 `Unproven`。 -/
theorem nothing_admitted_before_an_outcome :
    admit (awaiting : ChainHalt Reason) = .Unproven := rfl

end Sprawling.Assembly.ChainWatch
