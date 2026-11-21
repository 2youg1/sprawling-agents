-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 内容寻址验证图

规定 `Tools/xtask/src/ci.rs` 及其分部：验证节点的身份、变更闭包、结果复用与执行结论。
这里只模型化纯判定；文件读取、进程执行与 GitHub 缓存传输是 Rust 适配器的工作。
-/

/-! D47 **结果的身份是完整输入闭包的内容，不是提交号、路径过滤器或文件时间。** 一个绿色结果只在
节点定义、执行环境、直接输入内容以及全部依赖节点的内容键都相同时复用；任何失败都不写入可信绿色
结果。变更范围先取读取了变化内容的节点，再取它们的全部反向依赖；执行一个受影响节点时，它的前置
依赖也进入必需闭包。完整范围则要求每个必需节点都有本次执行或满足上述条件的绿色复用。被击败的备选：
mtime／整包 target 恢复（输入身份与传输大小脱钩），以及只按 `rust/client/lean` 粗判路径（不能表达
节点级输入与反向依赖）。
-/

namespace Xtask.Ci

/-- 一个结果能否复用的政策；`never` 用于非 hermetic 或带外部世界的检查。 -/
inductive Reuse where
  | successOnly
  | never
deriving DecidableEq, Repr

/-- 一次执行的外部结论。缓存只承认 `green`，其余两态都必须重新执行。 -/
inductive Verdict where
  | green
  | refused
  | broken
deriving DecidableEq, Repr

/-- 结果命名的输入视图：源文件、实际运行的验证器、执行环境分别保持自己的身份。
Rust 适配器从同一次读取的字节生成 owned 副本和源身份；实际检查只读该副本。
模型不证明文件系统权限、字节复制或散列无碰撞，这些前提由实现回归和真实运行核对。 -/
structure View where
  source : String
  verifier : String
  environment : String
deriving DecidableEq, Repr

/-- 一次检查所命名的视图和实际读取的视图；原工作树之后的状态不是结果的身份。 -/
structure Execution where
  named : View
  observed : View
  verdict : Verdict
deriving DecidableEq, Repr

/-- 只有实际读取了被命名视图的绿色结果才能发布。Rust 以 owned view 的生命周期落实此约束。 -/
def publishable (execution : Execution) : Bool :=
  decide (execution.named = execution.observed ∧ execution.verdict = .green)

/-- 发布成功同时要求输入身份一致与真实检查成功；只比较运行前后的原工作树不足以建立此前提。 -/
theorem publication_requires_the_observed_view (execution : Execution)
    (published : publishable execution = true) :
    execution.named = execution.observed ∧ execution.verdict = .green := by
  exact of_decide_eq_true published

/-- 即使检查通过，也不能把另一份输入的结果登记到原身份下。 -/
theorem another_view_cannot_publish (named observed : View)
    (different : named ≠ observed) :
    publishable { named, observed, verdict := .green } = false := by
  simp [publishable, different]

/-- 相同 owned 视图上的成功可以发布，因此约束没有通过拒绝一切检查而成立。 -/
theorem a_green_owned_view_can_publish (view : View) :
    publishable { named := view, observed := view, verdict := .green } = true := by
  simp [publishable]

/-- 一个存储结果及其内容键；键不匹配时，即使它是绿色也与当前节点无关。 -/
structure Cached where
  key : String
  verdict : Verdict
deriving DecidableEq, Repr

/-- 是否允许把 `cached` 当作 `current` 的结果（Rust：`store::may_reuse`）。 -/
def may_reuse (policy : Reuse) (cached : Cached) (current : String) : Bool :=
  match policy, cached.verdict with
  | .successOnly, .green => cached.key == current
  | _, _ => false

/-- 不可复用节点不会因为存储里恰好有同键绿色结果而跳过。 -/
theorem never_reuses (cached : Cached) (current : String) :
    may_reuse .never cached current = false := by
  rfl

/-- 失败与红色结果永不复用，不论键是否相同。 -/
theorem no_failure_is_reused (policy : Reuse) (key : String) (verdict : Verdict)
    (bad : verdict ≠ .green) :
    may_reuse policy { key, verdict } key = false := by
  cases policy <;> cases verdict <;> simp_all [may_reuse]

/-- 成功复用恰好要求政策允许、结果绿色、键相同。 -/
theorem reuse_iff_success_and_same_content (policy : Reuse) (cached : Cached) (current : String) :
    may_reuse policy cached current = true ↔
      policy = .successOnly ∧ cached.verdict = .green ∧ cached.key = current := by
  cases policy <;> cases cached <;> rename_i key verdict <;>
    cases verdict <;> simp [may_reuse]

/-- 一个验证节点：读取的路径身份与必须先行成功的节点。路径对应的实际内容在 Rust 侧散列。 -/
structure Node where
  id : String
  reads : List String
  needs : List String
  required : Bool
  policy : Reuse
deriving DecidableEq, Repr

/-- 变更直接触及的节点及其全部反向依赖。 -/
inductive Affected (nodes : List Node) (changed : List String) : String → Prop where
  | reads (node : Node) (path : String) :
      node ∈ nodes → path ∈ node.reads → path ∈ changed →
      Affected nodes changed node.id
  | dependent (node : Node) (dependency : String) :
      node ∈ nodes → dependency ∈ node.needs → Affected nodes changed dependency →
      Affected nodes changed node.id

/-- 读取变化内容的节点必在受影响闭包中。 -/
theorem changed_input_selects_its_reader (nodes : List Node) (changed : List String)
    (node : Node) (path : String) (in_nodes : node ∈ nodes)
    (reads : path ∈ node.reads) (moved : path ∈ changed) :
    Affected nodes changed node.id :=
  Affected.reads node path in_nodes reads moved

/-- 反向依赖可以沿任意有限链传播；每条边都使命名它的节点受影响。 -/
theorem changed_dependency_selects_dependent (nodes : List Node) (changed : List String)
    (node : Node) (dependency : String) (in_nodes : node ∈ nodes)
    (needs : dependency ∈ node.needs) (affected : Affected nodes changed dependency) :
    Affected nodes changed node.id :=
  Affected.dependent node dependency in_nodes needs affected

/-- 一次执行所需的节点：受影响节点，以及它们递归需要的前置节点。 -/
inductive Required (nodes : List Node) (changed : List String) : String → Prop where
  | affected (id : String) : Affected nodes changed id → Required nodes changed id
  | prerequisite (node : Node) (dependency : String) :
      node ∈ nodes → dependency ∈ node.needs → Required nodes changed node.id →
      Required nodes changed dependency

/-- 任何受影响节点的前置都进入执行闭包，因此「跳过前置却执行后继」不成立。 -/
theorem affected_execution_includes_prerequisites (nodes : List Node) (changed : List String)
    (node : Node) (dependency : String) (in_nodes : node ∈ nodes)
    (needs : dependency ∈ node.needs) (required : Required nodes changed node.id) :
    Required nodes changed dependency :=
  Required.prerequisite node dependency in_nodes needs required

/-- 一个节点在一次运行中的处置：真实执行，或读取一个候选结果。 -/
inductive Disposition where
  | execute
  | reuse (cached : Cached)
deriving DecidableEq, Repr

/-- 一个处置对节点 `node` 的当前内容键 `current` 是否有效。 -/
def Disposition.valid (policy : Reuse) (current : String) : Disposition → Bool
  | .execute => true
  | .reuse cached => may_reuse policy cached current

/-- 执行永远是一条可行路径；缓存缺席或失效不会让完整验证变得不可实现。 -/
theorem execution_is_always_available (policy : Reuse) (current : String) :
    Disposition.valid policy current .execute = true := by
  rfl

/-- 一次完整运行的记录：节点、它的内容键与所取处置。 -/
structure Judgment where
  node : String
  current : String
  disposition : Disposition
deriving DecidableEq, Repr

/-- 完整覆盖：每个必需节点都有一条本次执行或有效绿色复用的记录。 -/
def Complete (nodes : List Node) (judgments : List Judgment) : Prop :=
  ∀ node : Node, node ∈ nodes → node.required = true →
    ∃ judgment : Judgment, judgment ∈ judgments ∧ judgment.node = node.id ∧
      Disposition.valid node.policy judgment.current judgment.disposition = true

/-- 每个节点都真实执行是一份可实现的全量验证，不依赖缓存中存在任何结果。 -/
theorem full_execution_is_realizable (nodes : List Node) :
    Complete nodes (nodes.map fun node =>
      { node := node.id, current := "", disposition := .execute }) := by
  intro node in_nodes _required
  refine ⟨{ node := node.id, current := "", disposition := .execute }, ?_, rfl, rfl⟩
  simp only [List.mem_map]
  exact ⟨node, in_nodes, rfl⟩

/-- 非必需节点也不能借一个失败结果进入绿色闭包。 -/
theorem cached_failure_cannot_complete (cached : Cached) (bad : cached.verdict ≠ .green)
    (current : String) :
    Disposition.valid .successOnly current (.reuse cached) = false := by
  cases cached with | mk key verdict =>
    cases verdict <;> simp_all [Disposition.valid, may_reuse]

/-- 必需的 Never 节点在完整记录中必有本次执行，不能由同键绿色缓存代替。 -/
theorem complete_runs_never_nodes (nodes : List Node) (judgments : List Judgment)
    (complete : Complete nodes judgments) (node : Node)
    (in_nodes : node ∈ nodes) (required : node.required = true)
    (never : node.policy = .never) :
    ∃ judgment : Judgment, judgment ∈ judgments ∧ judgment.node = node.id ∧
      judgment.disposition = .execute := by
  obtain ⟨judgment, in_judgments, named, valid⟩ := complete node in_nodes required
  refine ⟨judgment, in_judgments, named, ?_⟩
  cases disposition : judgment.disposition with
  | execute => rfl
  | reuse cached => simp [never, disposition, Disposition.valid, may_reuse] at valid

end Xtask.Ci
