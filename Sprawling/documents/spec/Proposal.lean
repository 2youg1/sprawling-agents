-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# proposal：一张修改提案卡怎样切成句子、怎样按逐句的决定合成、一张卡能被处理几次

规定 `crates/documents/src/proposal.rs`（`documents::Review`、`Slice`、`Offer`、`decide`）与 `crates/documents/src/proposal/slices.rs`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

Rust 的 `Review::of(before, after)` 把两段文本各切成句子（每一句连着它前面的空白与最后一句之后的空白，接起来恰是原文），再对齐成一串 `Slice`：`Same` 两边都有，`Delete` 只在原文里，`Insert` 只在提议里。本模型把一句写成它的全部字节 `piece`（Rust 的 `lead ++ text ++ trail`），把对齐的结果当作给定的一串句子；「对齐出的句子按种类过滤之后恰是两边的句子」由 Rust 的测试断言，不在这里证明。Rust 的决定是一组「序号 ↦ 判词」，本模型把它写成与句子一一对应的 `Option Verdict` 列表，没被点名的句子是 `none`；`amend` 在 Rust 里换掉的是句子的 `text`、留着它的空白，这里换掉整句，两者对性质没有区别。

证明的性质：

1. **整张拒绝就是原文**（`rejectAll_is_before`）：没有一句被点名时，合成的字节是按序接起的非插入句子，也就是原文，逐字节（D14、D16）。
2. **整张接受就是提议**（`acceptAll_is_after`）：每一句都被接受时，合成的字节是按序接起的非删除句子，也就是提议，逐字节（D14）。
3. **一张卡只被处理一次**（`decided_is_final`、`withdrawn_is_final`、`open_moves`）：开着的卡可以被决定或被收回，决定过或收回过的卡不能再动（D19）。

**用去掉空白的句子作对齐的键，整张接受就不再是提议**（`looseKey_loses_the_after_side`）：RefRain 的做法，本模型咬得动的演示。
-/

namespace Documents.Proposal

/-- 一个字节；取值范围与性质无关。 -/
abbrev Bytes := List Nat

/-- 一句在 diff 里的角色。 -/
inductive Kind where
  | same
  | delete
  | insert
  deriving DecidableEq

/-- 卡上的一句：它的角色与它的全部字节（前导空白、句子、尾随空白）。 -/
structure Slice where
  kind : Kind
  piece : Bytes

/-- 原文：按序接起不是插入的句子。 -/
def beforeOf : List Slice → Bytes
  | [] => []
  | s :: t => (if s.kind = .insert then [] else s.piece) ++ beforeOf t

/-- 提议：按序接起不是删除的句子。 -/
def afterOf : List Slice → Bytes
  | [] => []
  | s :: t => (if s.kind = .delete then [] else s.piece) ++ afterOf t

/-- 人对一句改动的判词：接受，或改成 `text` 再接受。 -/
inductive Verdict where
  | accept
  | amend (text : Bytes)

/-- 一句在合成里留下的字节。没被点名（`none`）即拒绝这句改动：删除的句子留着，插入的句子不进来。 -/
def keep (s : Slice) : Option Verdict → Bytes
  | none => if s.kind = .insert then [] else s.piece
  | some .accept => if s.kind = .delete then [] else s.piece
  | some (.amend text) => if s.kind = .insert then text else s.piece

/-- 按逐句的判词合成：判词比句子少时，余下的句子没被点名。 -/
def merged : List Slice → List (Option Verdict) → Bytes
  | [], _ => []
  | s :: t, [] => keep s none ++ merged t []
  | s :: t, v :: vs => keep s v ++ merged t vs

theorem rejectAll_is_before (s : List Slice) : merged s [] = beforeOf s := by
  induction s with
  | nil => rfl
  | cons x t ih => simp [merged, keep, beforeOf, ih]

theorem acceptAll_is_after (s : List Slice) :
    merged s (s.map fun _ => some Verdict.accept) = afterOf s := by
  induction s with
  | nil => rfl
  | cons x t ih => simp [merged, keep, afterOf, ih]

/-- 一张卡的状态。 -/
inductive Status where
  | opened
  | decided
  | withdrawn
  deriving DecidableEq

/-- 对一张卡能做的两件事：人决定它，提出它的 run 收回它。 -/
inductive Move where
  | decide
  | withdraw

/-- 只有开着的卡能动；`none` 即这一步被拒（Rust 的 `E_INVALID_ARGS`），卡的状态不变。 -/
def step : Status → Move → Option Status
  | .opened, .decide => some .decided
  | .opened, .withdraw => some .withdrawn
  | .decided, _ => none
  | .withdrawn, _ => none

theorem open_moves (m : Move) : (step .opened m).isSome = true := by
  cases m <;> rfl

theorem decided_is_final (m : Move) : step .decided m = none := by
  cases m <;> rfl

theorem withdrawn_is_final (m : Move) : step .withdrawn m = none := by
  cases m <;> rfl

/-!
## 咬得动的演示

RefRain 用去掉前后空白的句子作对齐的键，键相同的两句就算 `Same`，合成时取原文那一边的字节。一句只在尾随空白上不同的提议（原文 `[1]`，提议 `[1, 0]`，`0` 是一个空格）于是对齐成一句 `Same`，整张接受之后得到的是 `[1]`，提议里的空格丢了。本仓库用整句的字节作键（D14），两边不同的句子一定是一删一插。
-/

/-- RefRain 的一句：键相同的 `Same` 两边各有自己的字节。 -/
structure LooseSlice where
  kind : Kind
  before : Bytes
  after : Bytes

/-- RefRain 的整张接受：`Same` 取原文那一边。 -/
def looseAcceptAll : List LooseSlice → Bytes
  | [] => []
  | s :: t =>
    (match s.kind with
      | .same => s.before
      | .delete => []
      | .insert => s.after) ++ looseAcceptAll t

/-- 那一串句子拼出的提议。 -/
def looseAfterOf : List LooseSlice → Bytes
  | [] => []
  | s :: t => (if s.kind = .delete then [] else s.after) ++ looseAfterOf t

theorem looseKey_loses_the_after_side :
    looseAcceptAll [⟨.same, [1], [1, 0]⟩] ≠ looseAfterOf [⟨.same, [1], [1, 0]⟩] := by
  decide

end Documents.Proposal
