-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# fan-in：写的人不验自己的活

规定 `crates/collab/src/fanin.rs`（`collab::fanin`）与 `crates/collab/src/pr.rs`（`collab::pr`）的判定。Rust 代码是「怎样守住」的权威：`Artifact` 没有公开构造子，`Pr<Open>` 拿不到验证者的名字，两条都由 `crates/collab/tests/ui/` 的编译失败反例钉住。本模型是「必须守住哪些性质」的权威。

未验证的产出是 `Claim`；`Claim.verified` 是造出 `Artifact` 的唯一一处。汇合（`FanIn`）只收 `Artifact`，于是「Claim 进汇合」在 Rust 的类型层拼不出来。这里把那条唯一的路写成函数，性质都说它的输出：

1. **实现者不自测**（`verified_not_by_producer`）：得到的 `Artifact`，验证者与生产者不是同一个人。
2. **验不过就没有 Artifact**（`verified_needs_check`）。
3. **PR 的验证者不是实现者，且产出属于这个节点**（`merge_needs_another`）。`Pr<Open> → Pr<Verified>` 是判定梯的全部；没有 `Merged` 相位，落地事实由 `EventKind::PrMerged` 与 `storage::worktree` 记。
4. **答对私题要读过全文**（`decide_iff`）：答案按字节求 BLAKE3，等于 artifact 的 digest 才放行；工具参数经 JSON 传递时常去掉结尾换行，所以缺的那一个换行补回一次再比。摘要函数是参数：它是 BLAKE3，不是这里的事实。

私题是围栏不是证明：能断言的只是「一眼未看就判」被拒。拒词恒不回显正确答案，回显即教会那条捷径。题面给出 artifact 的 locator，而 `cas:b3-…` 本身拼出了内容 digest，所以任何由 digest 派生的答案（例如前八位）都写在题面上；要全文就是为此。被否决的方案：按 digest 另派一个秘密的问题——它要 artifact 在城里另存内容，而 `Artifact` 只持 locator 与 digest。
-/

namespace Collab.Fanin

structure Claim where
  node : Nat
  producer : String
  deriving DecidableEq, Repr

structure Artifact where
  node : Nat
  producer : String
  verifiedBy : String
  deriving DecidableEq, Repr

inductive Refusal where
  | selfVerified
  | checkFailed
  | otherNode
  deriving DecidableEq, Repr

/-- 造出 `Artifact` 的唯一一处：先问验证者是不是生产者，再问 done check 过没过。 -/
def Claim.verified (c : Claim) (passed : Bool) (verifier : String) : Except Refusal Artifact :=
  if verifier = c.producer then .error .selfVerified
  else if passed then .ok ⟨c.node, c.producer, verifier⟩
  else .error .checkFailed

theorem verified_not_by_producer {c : Claim} {passed : Bool} {v : String} {a : Artifact}
    (h : c.verified passed v = .ok a) : a.verifiedBy ≠ a.producer := by
  unfold Claim.verified at h
  by_cases self : v = c.producer
  · simp [self] at h
  · cases passed
    · simp [self] at h
    · simp only [self, if_false, if_true, Except.ok.injEq] at h
      rw [← h]
      exact self

theorem verified_needs_check {c : Claim} {passed : Bool} {v : String} {a : Artifact}
    (h : c.verified passed v = .ok a) : passed = true := by
  unfold Claim.verified at h
  by_cases self : v = c.producer
  · simp [self] at h
  · cases passed
    · simp [self] at h
    · rfl

/-- `Pr<Open>::verified`：这份产出是这个节点的，且验证者不是实现者。后一条近乎不可达（`Artifact` 已携「非生产者跑过 done check」），保留它，是因为「近乎」正在替一场没人做的评审干活。 -/
def verifyPr (node : Nat) (implementer : String) (a : Artifact) : Except Refusal String :=
  if a.node ≠ node then .error .otherNode
  else if a.verifiedBy = implementer then .error .selfVerified
  else .ok a.verifiedBy

theorem merge_needs_another {node : Nat} {implementer by_ : String} {a : Artifact}
    (h : verifyPr node implementer a = .ok by_) : by_ ≠ implementer ∧ a.node = node := by
  unfold verifyPr at h
  by_cases other : a.node ≠ node
  · simp [other] at h
  · by_cases self : a.verifiedBy = implementer
    · simp [other, self] at h
    · simp only [other, self, if_false, Except.ok.injEq] at h
      rw [← h]
      exact ⟨self, Decidable.of_not_not other⟩

/-- 私题：答案或补回一个结尾换行的答案，摘要等于 artifact 的 digest。 -/
def decide' (digestOf : String → Nat) (digest : Nat) (answer : String) : Bool :=
  digestOf answer == digest || digestOf (answer ++ "\n") == digest

theorem decide_iff (digestOf : String → Nat) (digest : Nat) (answer : String) :
    decide' digestOf digest answer = true ↔
      digestOf answer = digest ∨ digestOf (answer ++ "\n") = digest := by
  simp [decide']

/-!
## 咬得动的演示

把 `Claim.verified` 里「验证者是生产者即拒」那条守卫拿掉，`verified_not_by_producer` 不再成立：一个人验自己的活，照样得到 `Artifact`。
-/

def Claim.verifiedWithoutGuard (c : Claim) (passed : Bool) (verifier : String) :
    Except Refusal Artifact :=
  if passed then .ok ⟨c.node, c.producer, verifier⟩ else .error .checkFailed

theorem withoutGuard_self_verifies :
    (Claim.verifiedWithoutGuard ⟨1, "hana"⟩ true "hana") = .ok ⟨1, "hana", "hana"⟩ := by
  rfl

end Collab.Fanin
