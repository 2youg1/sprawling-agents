-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# `guard::wall`：唯一一张自己的 lint 表，与它记下的差异

规定 `tools/xtask/src/guard/wall.rs` 的 `judge`（`tools/xtask/Spec.lean` §8-46）：叶子
`crates/desktop/ffi` 的 `[lints]` 与根 `[workspace.lints]` 逐键比对，例外只在 `RECORDED`。

参照定义只陈述一个键的结论；两张表怎样读成键、成员怎样列出、别的成员怎样判 `workspace = true`
不在这里，那一半在 `guard` 的 Rust 里，由它自己的测试守着。
-/

namespace Xtask.Guard.Wall

/-- 一个 lint 键的结论。 -/
inductive KeyVerdict where
  /-- 两侧相等且没有记下差异，或两侧不等而差异记在 `RECORDED` 里。 -/
  | Agrees
  /-- 两侧不等，而没有人记下这个差异。 -/
  | Diverged
  /-- 记下的差异已不再是差异：那一行须划掉。记录因此会自清理：两侧重新相等时，那一行是一条违规，
  而不是一条沉默的许可。 -/
  | Spent
deriving DecidableEq, Repr

/-
D14 **guard 只判一张自己的 lint 表；`members` 只列工作区成员。**desktop 并回工作区之后
（`crates/desktop/Spec.lean` D14），本仓没有一个在工作区之外构建的包，唯一的抄件是 FFI 叶子
`crates/desktop/ffi` 那张 `[lints]`，它与根表只差 `unsafe_code` 一行。guard 因此判这一张表，再判
其余每个成员都写 `lints.workspace = true`；墙的元数据、依赖版本与四处抄过去的常量（协议修订、
`_meta` 键、错误码、质量域）不再有第二份，比对随之删去。成员名单取自 `members`（`cargo metadata`），
不从根清单的 `members` 数组读，因为 cargo 会把工作区目录里的 path 依赖自动收为成员，数组里没写的
成员照样存在。`members` 的 `Reach` 与「经 path 依赖进来的墙外包」那一支一起删去：没有那样的包，
一个永远不出现的变体只会让每个读者多判一臂。嵌套的包（`crates/desktop/ffi` 在 `crates/desktop`
之下）由 `members::owner` 判归属：持有一个路径的包里目录最长的那一个。被击败的备选：①叶子也抄包
元数据与依赖版本、guard 照旧比对一整堵墙——这些在工作区里都能继承，抄了就是第二个家；②guard 从
根清单的 `members` 数组读成员——漏掉 cargo 自动收进来的成员；③留着 `Reach::PathDependency` 等
下一个墙外包——没有读者的变体是死代码。重开参数：cargo 允许一个成员继承工作区 lint 表而只改一行，
那时叶子写 `workspace = true` 加一行覆盖，本门的比对删去。
-/

/-- 一个键：它是否在 `RECORDED` 里，两侧是否相等。 -/
def judge (recorded same : Bool) : KeyVerdict :=
  match recorded, same with
  | true, true => .Spent
  | false, false => .Diverged
  | true, false => .Agrees
  | false, true => .Agrees

/-- 过门的键恰是「记下了」与「不相等」一致的键：一个差异过门当且仅当它被记下，一条记录留得住
当且仅当它仍记着一个差异。 -/
theorem a_key_agrees_exactly_when_the_record_matches_the_difference (recorded same : Bool) :
    judge recorded same = .Agrees ↔ recorded = !same := by
  cases recorded <;> cases same <;> decide

end Xtask.Guard.Wall
