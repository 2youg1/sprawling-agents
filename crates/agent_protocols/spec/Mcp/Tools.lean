-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Tools：外部工具的名字、构造与它的答复

规定 `crates/agent_protocols/src/mcp/tools.rs`（`agent_protocols::mcp::tools`）与 `crates/agent_protocols/src/mcp/handshake.rs` 里 `Rpc::call_tool` 的浮点检查。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

四件事：

1. **两台 server 的同名工具恒是两个工具**（`two_servers_offering_one_verb_stay_two_tools`）：本城的工具名是 `{label}_{sanitised}`，而 `ServerLabel` 只由小写字母与数字组成（`kernel::tool`，`crates/kernel/Spec.lean` §8-23），所以第一个下划线之前就是标签。
2. **浮点入参按调用拒，并报出它的位置**（`floatAt_names_a_fractional_leaf`、`floatAt_none_iff`）：拒词里的路径指向一个真是小数的叶子，没有小数的入参恒不被拒。
3. **一件出站工具在 confidential 楼里不存在，也不能没有期限**（`a_constructed_tool_is_outside_confidential_and_bounded`）。
4. **工具自己报的错是一次失败；`_meta` 说效果未知、或说得读不懂，都按「不知道」读**（`read_unknown_iff`、`a_reading_safe_to_repeat_is_one_the_server_never_doubted`）。

JSON 怎样摊平成叶子与路径（`a.b[1]` 的写法）、文字块怎样连接与截断（`ERROR_TEXT_CAP_BYTES`）是 Rust 的事，不在模型里；模型只取判决依赖的那几位。
-/

namespace AgentProtocols.Mcp.Tools

/-! ## 名字 -/

/-- 本城给一件远端工具起的名字：标签、下划线、远端名净化后的样子。 -/
def localName (label sanitised : List Char) : List Char := label ++ '_' :: sanitised

/-- 一个名字里第一个下划线之前的部分。 -/
def labelOf (name : List Char) : List Char := name.takeWhile (· != '_')

/-- 标签里没有下划线时，名字的第一个下划线之前恰是标签。 -/
theorem labelOf_localName (label sanitised : List Char) (clean : '_' ∉ label) :
    labelOf (localName label sanitised) = label := by
  induction label with
  | nil => simp [labelOf, localName]
  | cons c rest ih =>
    have hc : c ≠ '_' := fun e => clean (by simp [e])
    have hrest : '_' ∉ rest := fun e => clean (List.mem_cons_of_mem _ e)
    have := ih hrest
    simp only [labelOf, localName, List.cons_append] at this ⊢
    simp [hc, this]

/-- 两台标签不同的 server，即使提供同一个动词，也给出两个不同的工具名。 -/
theorem two_servers_offering_one_verb_stay_two_tools (label₁ label₂ s₁ s₂ : List Char)
    (clean₁ : '_' ∉ label₁) (clean₂ : '_' ∉ label₂) (differ : label₁ ≠ label₂) :
    localName label₁ s₁ ≠ localName label₂ s₂ := by
  intro same
  apply differ
  rw [← labelOf_localName label₁ s₁ clean₁, ← labelOf_localName label₂ s₂ clean₂, same]

/-! ## 浮点入参 -/

/-- 入参摊平后的一个叶子：它在 JSON 里的路径，以及它是不是一个小数。 -/
structure Leaf where
  path : String
  fractional : Bool
  deriving DecidableEq, Repr

/- D4 浮点入参怎么处置：拒这一次调用（选中），不拒整个工具（被否）。禁浮点的真实来源是 Ledger 载荷：一次记不下来的调用就是一次重演不出来的调用。而工具的 schema 里有一个数值字段，并不说明这个工具不可用；拿一次坏参数把整件能力下架，惩罚的是下一次本来正确的调用。故按调用拒，并在拒词里报出那个值的路径（`a.b[1]`），让模型改的是那一处而不是猜整张表。 -/
/-- `float_at`：第一个小数叶子的路径。`call_tool` 在它为 `some` 时拒这一次调用，拒词带这条路径。 -/
def floatAt : List Leaf → Option String
  | [] => none
  | leaf :: rest => if leaf.fractional then some leaf.path else floatAt rest

/-- 拒词里的路径指向一个真是小数的叶子：模型下一步要改的是那一处。 -/
theorem floatAt_names_a_fractional_leaf :
    ∀ (leaves : List Leaf) (path : String), floatAt leaves = some path →
      ∃ leaf ∈ leaves, leaf.path = path ∧ leaf.fractional = true
  | [], _, h => by simp [floatAt] at h
  | leaf :: rest, path, h => by
    unfold floatAt at h
    by_cases frac : leaf.fractional = true
    · simp only [frac, ite_true, Option.some.injEq] at h
      exact ⟨leaf, List.mem_cons_self .., h, frac⟩
    · simp only [frac] at h
      obtain ⟨found, mem, p, f⟩ := floatAt_names_a_fractional_leaf rest path h
      exact ⟨found, List.mem_cons_of_mem _ mem, p, f⟩

/-- 没有小数叶子的入参恒不被拒。 -/
theorem floatAt_none_iff :
    ∀ leaves : List Leaf, floatAt leaves = none ↔ ∀ leaf ∈ leaves, leaf.fractional = false
  | [] => by simp [floatAt]
  | leaf :: rest => by
    have ih := floatAt_none_iff rest
    cases h : leaf.fractional <;> simp [floatAt, h, ih]

/-! ## 构造 -/

/-- 构造一件出站工具时，判决依赖的两位。 -/
structure Registration where
  confidential : Bool
  /-- 声明的期限，毫秒。 -/
  timeout : Option Nat
  deriving DecidableEq, Repr

/-- 构造被拒的两种原因。 -/
inductive Refusal where
  /-- `E_GATE_DENIED`：confidential 楼。 -/
  | gateDenied
  /-- `E_INVALID_ARGS`：没有期限。 -/
  | invalidArgs
  deriving DecidableEq, Repr

/- D5 confidential 怎么落：构造时就不存在（选中），不是调用时由 egress 门拒（被否）。被否的那一种依赖每条路径都记得问那道门；选中的这一种让「这栋楼有一个出站工具」这件事本身不成立，那时还没有任何东西可泄。两者不冲突：egress 门仍在，这只是把同一条判断挪到更早、更便宜、更难绕的位置。 -/
/-- `McpTool::new`：confidential 先判，再要期限；成了就得到这件工具的耐心。 -/
def construct (r : Registration) : Except Refusal Nat :=
  if r.confidential then .error .gateDenied
  else match r.timeout with
    | some patience => .ok patience
    | none => .error .invalidArgs

/-- 构造出来的工具不在 confidential 楼里，且它的耐心就是声明的期限。 -/
theorem a_constructed_tool_is_outside_confidential_and_bounded (r : Registration) (patience : Nat)
    (h : construct r = .ok patience) : r.confidential = false ∧ r.timeout = some patience := by
  unfold construct at h
  cases hc : r.confidential <;> simp [hc] at h
  · cases ht : r.timeout <;> simp [ht] at h
    simp [h]

/-- confidential 楼恒不构造出站工具，不论它声明了什么期限。 -/
theorem a_confidential_building_never_holds_one (r : Registration) (h : r.confidential = true) :
    construct r = .error .gateDenied := by
  simp [construct, h]

/-- 正常的构造可达：一栋普通楼里、带期限的注册成为一件工具。 -/
theorem an_ordinary_registration_constructs :
    construct ⟨false, some 60000⟩ = .ok 60000 := rfl

/-! ## 一次 `tools/call` 的答复 -/

/-- `_meta` 里 `sprawling/effect-unknown` 这个键：没有、是 `false`、是 `true`、或是别的值。 -/
inductive Flag where
  | absent
  | falseValue
  | trueValue
  | other
  deriving DecidableEq, Repr

/-- 本城怎么读一份答复。 -/
inductive Reading where
  /-- 一份答案，交给窗口。 -/
  | answer
  /-- 工具报的错：`E_TOOL_UNAVAILABLE`，`Retry::No`。 -/
  | failed
  /-- 工具说这次也许已经生效了一部分：`E_TOOL_OUTCOME_UNKNOWN`，`Retry::Unknown`。 -/
  | unknown
  deriving DecidableEq, Repr

/- D1 工具报的错成为 `Err`（选中），不原样当成一份答案交给窗口（被否）。被否的那一种让模型读到的 `is_error` 为假，账本记成 `Answered`，一次失败看起来像一个古怪的结果。值写错的 `_meta` 也按「不知道」读：错读成「没生效」，模型会把一次可能已经落地的动作再做一遍（文末 `strictReading_repeats_a_misspelled_flag`）。4 KiB 的文字上限（`ERROR_TEXT_CAP_BYTES`）是我们的选择：一句拒词加一句恢复写得下；更长的错误文字多半是 server 把堆栈或整页内容塞了进来，而这条路上没有 CAS 可以把它存下再分窗，窗口与账本为它付的代价比它能告诉模型的多。也被否：读 JSON-RPC error 的 `data.code`、`data.recovery`、`data.retry`，那是规格外的约定，城要为每一家 server 猜一次形状。 -/
/-- `isError` 是否为真，与 `_meta` 的那一个键，决定读法。 -/
def read (isError : Bool) (flag : Flag) : Reading :=
  if isError then
    match flag with
    | .absent | .falseValue => .failed
    | .trueValue | .other => .unknown
  else .answer

/-- `isError` 不为真就是一份答案。 -/
theorem not_an_error_is_an_answer (flag : Flag) : read false flag = .answer := rfl

/-- 读成「不知道」，当且仅当工具报了错，且那个键在、值不是 `false`。 -/
theorem read_unknown_iff (isError : Bool) (flag : Flag) :
    read isError flag = .unknown ↔ isError = true ∧ flag ≠ .absent ∧ flag ≠ .falseValue := by
  cases isError <;> cases flag <;> simp [read]

/-- 一个可以原样再试的读法（`failed`），恒不是 server 说过「也许已经生效」的那一种：
写错的值也按「不知道」读。 -/
theorem a_reading_safe_to_repeat_is_one_the_server_never_doubted (isError : Bool) (flag : Flag)
    (h : read isError flag = .failed) : flag = .absent ∨ flag = .falseValue := by
  cases isError <;> cases flag <;> simp [read] at h ⊢

/-!
## 咬得动的演示

标签里允许下划线，两台 server 就能给出同一个工具名；只把 `true` 读成「不知道」，一个写错的值就变成一次可以原样再做的失败，模型会把一个也许已经落地的动作再做一遍。
-/

theorem withUnderscoredLabels_two_servers_collide :
    localName "a_b".toList "c".toList = localName "a".toList "b_c".toList := by decide

/-- 只认 `true` 的读法。 -/
def readStrict (isError : Bool) (flag : Flag) : Reading :=
  if isError then (if flag = .trueValue then .unknown else .failed) else .answer

theorem strictReading_repeats_a_misspelled_flag : readStrict true .other = .failed := rfl

end AgentProtocols.Mcp.Tools
