-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# `packaged`：哪些项进不了任何一次生产编译

规定 `tools/xtask/src/packaged.rs` 的 `excludes_production`（`tools/xtask/Spec.lean` §8-49）：
一个 `cfg` 谓词在 `test` 为假时不可能成立，它标着的项连同它声明的文件就不进 `packaged` 的判定。

参照定义把谓词读成一棵树，按 Rust 参考的 `cfg` 语义求值；`excludes_production` 是门的读法。
两条性质：门判「进不了」的谓词在任何一次 `test` 为假的构建里都不成立（门不会漏判一个会编译的
文件）；门判「可能成立」的谓词里确有在生产构建里成立的（规则不是空真）。门往另一个方向错——
把一个其实进不了生产的项当作可能成立——只多报一条人会读的违规，`packaged.rs` 的 rustdoc 记着
这一取舍。
-/

namespace Xtask.Packaged

/-- 一个 `cfg` 谓词，按门读得出的形状：`test` 这一个名字、别的名字或 `key = "value"`、
`all(…)`、`any(…)`、`not(…)`，以及门读不出的写法。 -/
inductive Cfg where
  | test
  | other (name : String)
  | all (parts : List Cfg)
  | any (parts : List Cfg)
  | not (inner : Cfg)
  | unreadable
deriving Repr

/-- 一次构建：`test` 开没开，以及别的每个名字成不成立；门读不出的写法在这次构建里取什么值。 -/
structure Build where
  test : Bool
  holds : String → Bool
  unreadable : Bool

mutual
/-- 谓词在一次构建里成不成立（Rust 参考的 `cfg` 语义）。 -/
def holds (build : Build) : Cfg → Bool
  | .test => build.test
  | .other name => build.holds name
  | .all parts => holdsEvery build parts
  | .any parts => holdsSome build parts
  | .not inner => !holds build inner
  | .unreadable => build.unreadable

/-- `all(…)`：每一支都成立；空表恒成立。 -/
def holdsEvery (build : Build) : List Cfg → Bool
  | [] => true
  | part :: rest => holds build part && holdsEvery build rest

/-- `any(…)`：至少一支成立；空表恒不成立。 -/
def holdsSome (build : Build) : List Cfg → Bool
  | [] => false
  | part :: rest => holds build part || holdsSome build rest
end

/-
D16 **`packaged` 沿 cargo 的 target 走模块树，不按文件名猜哪些是测试。**一个包被打包之后，验证构建
与 `cargo install` 只编译它的 lib、bin 与构建脚本，测试、bench、example 都不编；所以「这个
`include_str!` 会不会在包外落空」只对生产编译单元里的调用成立。门从 `cargo metadata` 交出的
target 源文件起，按 Rust 参考的模块路径规则跟 `mod` 声明走，凡 cfg 谓词在 `test` 为假时不可能
成立的项（`test`，或含 `test` 的 `all(…)`，或每一支都如此的 `any(…)`）连同它声明的文件一并
不进（`tools/xtask/Spec.lean` §8-49）。理由：本仓用 `#[path = "…/tests.rs"]`、
`#[path = "grammar_tests.rs"]`、`#[cfg(test)] mod tests;` 几种写法放测试，文件名说不准；而模块树
就是编译器自己的答案，没有第二种读法。被击败的备选：①像 `boundary`、`depmap` 那样按 `tests`、
`tests.rs`、`_tests.rs` 认测试文件——一个生产文件叫 `fixture.rs`、一个测试文件叫
`grammar_tests.rs` 之外的名字时，前者放过、后者误报，且失败无声；②在门里跑 `cargo package`
加验证构建——要编译整个包，分钟级，而门在 `gates` 里毫秒级并行（§8-31），这份证据由人发布前的
`cargo publish --dry-run` 给；③判每个文件里的每个 `include_str!`，测试也算——测试读
`tools/fixtures/` 是本仓的惯例，而打包之后它们根本不编译，拒它们是错的。**重开参数**：一个可发布
的包开始用宏生成 `mod` 声明，或用 `cfg_attr` 给 `mod` 换路径，那时门改为读
`cargo rustc -- -Zunpretty` 一类编译器给出的模块表。
-/

mutual
/-- 门的读法：谓词在 `test` 为假时不可能成立。`not(…)`、别的名字与读不出的写法都算可能成立。 -/
def excludes_production : Cfg → Bool
  | .test => true
  | .other _ => false
  | .all parts => excludesSome parts
  | .any parts => excludesEvery parts
  | .not _ => false
  | .unreadable => false

/-- `all(…)` 里某一支进不了生产。 -/
def excludesSome : List Cfg → Bool
  | [] => false
  | part :: rest => excludes_production part || excludesSome rest

/-- `any(…)` 的每一支都进不了生产；空的 `any()` 恒不成立，所以也算。 -/
def excludesEvery : List Cfg → Bool
  | [] => true
  | part :: rest => excludes_production part && excludesEvery rest
end

mutual
/-- 门判「进不了生产」的谓词，在任何一次 `test` 为假的构建里都不成立：门不会因此漏判一个
生产构建会编译的文件。 -/
theorem an_excluded_item_never_builds (build : Build) (production : build.test = false) :
    (predicate : Cfg) → excludes_production predicate = true → holds build predicate = false
  | .test, _ => by simp [holds, production]
  | .other _, excluded => by simp [excludes_production] at excluded
  | .all parts, excluded => by
    simp only [excludes_production] at excluded
    simp only [holds]
    exact some_excluded_part_never_builds build production parts excluded
  | .any parts, excluded => by
    simp only [excludes_production] at excluded
    simp only [holds]
    exact every_excluded_part_never_builds build production parts excluded
  | .not _, excluded => by simp [excludes_production] at excluded
  | .unreadable, excluded => by simp [excludes_production] at excluded

theorem some_excluded_part_never_builds (build : Build) (production : build.test = false) :
    (parts : List Cfg) → excludesSome parts = true → holdsEvery build parts = false
  | [], excluded => by simp [excludesSome] at excluded
  | part :: rest, excluded => by
    simp only [excludesSome, Bool.or_eq_true] at excluded
    simp only [holdsEvery, Bool.and_eq_false_iff]
    cases excluded with
    | inl here => exact Or.inl (an_excluded_item_never_builds build production part here)
    | inr later => exact Or.inr (some_excluded_part_never_builds build production rest later)

theorem every_excluded_part_never_builds (build : Build) (production : build.test = false) :
    (parts : List Cfg) → excludesEvery parts = true → holdsSome build parts = false
  | [], _ => by simp [holdsSome]
  | part :: rest, excluded => by
    simp only [excludesEvery, Bool.and_eq_true] at excluded
    simp only [holdsSome, Bool.or_eq_false_iff]
    exact ⟨an_excluded_item_never_builds build production part excluded.1,
      every_excluded_part_never_builds build production rest excluded.2⟩
end

/-- 一次生产构建：`test` 为假，`unix` 成立，别的名字都不成立，读不出的写法取不成立。 -/
def productionOnUnix : Build where
  test := false
  holds := fun name => name == "unix"
  unreadable := false

/-- 门判「可能成立」的谓词里确有生产构建会编译的：`not(test)` 与 `unix` 都进生产，`all(test, unix)`
与 `any(test)` 都进不了，所以这条读法既不是全拒也不是全放。 -/
theorem the_reading_is_not_vacuous :
    excludes_production (.not .test) = false ∧ holds productionOnUnix (.not .test) = true ∧
    excludes_production (.other "unix") = false ∧ holds productionOnUnix (.other "unix") = true ∧
    excludes_production (.all [.test, .other "unix"]) = true ∧
    excludes_production (.any [.test]) = true := by
  simp [excludes_production, excludesSome, excludesEvery, holds, productionOnUnix]

end Xtask.Packaged
