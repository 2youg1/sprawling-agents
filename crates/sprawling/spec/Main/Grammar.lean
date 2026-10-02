-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 命令行的优先规则

规定 `crates/sprawling/src/main/grammar.rs` 的 `parse` 在读任何动词的参数之前做的那几个决定（sprawling-SPEC.md 8-89）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。命令表 `VERBS`（`main/verbs.rs`）是数据，按名找行（`find`）与读一行的参数（`arguments`）在模型里是参数，因为这里要钉的只是次序：

* `--version`／`-V` 出现在 `--` 之前的任何位置都得 `Version`；
* 其次 `--help`／`-h` 出现在 `--` 之前的任何位置都得那个动词的帮助，没有动词时得总览；
* 两者都在任何动词运行之前决定，所以 `up --help` 不写创世记录、`install --help` 不改 PATH；
* 什么都没敲是首屏。

`--` 之后的词是交给动词的文字，不参与这两个判断。
-/

namespace Sprawling.Main.Grammar

/-- 命令行读出的东西（Rust：`Invocation`）。`verb` 与 `args` 的类型是参数。 -/
inductive Invocation (Verb Args : Type) where
  | firstScreen
  | overview
  | help (verb : Verb)
  | version
  | run (verb : Verb) (args : Args)
  deriving Repr

/-- `--` 之前的那些词：版本与帮助只在这里找。 -/
def own (words : List String) : List String :=
  words.takeWhile (· ≠ "--")

def asksVersion (words : List String) : Bool :=
  (own words).any (fun w => w == "--version" || w == "-V")

def asksHelp (words : List String) : Bool :=
  (own words).any (fun w => w == "--help" || w == "-h")

variable {Verb Args Error : Type}
  (find : String → List String → Except Error (Verb × List String))
  (arguments : Verb → List String → Except Error Args)

/-- `parse`：先版本，再空行，再 `help <verb>` 与开头的 `--help`，再按名找行；找到之后帮助先于参数。 -/
def parse (words : List String) : Except Error (Invocation Verb Args) :=
  if asksVersion words then .ok .version
  else match words with
    | [] => .ok .firstScreen
    | first :: rest =>
      if first = "help" then
        match rest with
        | [] => .ok .overview
        | verb :: after => (find verb after).map (fun found => .help found.1)
      else if first = "--help" ∨ first = "-h" then .ok .overview
      else
        (find first rest).bind fun found =>
          if asksHelp words then .ok (.help found.1)
          else (arguments found.1 found.2).map (.run found.1)

/-- 要了版本，得到的就是版本，不论动词是什么、存不存在。 -/
theorem version_wins (words : List String) (h : asksVersion words = true) :
    parse find arguments words = .ok .version := by
  simp [parse, h]

/-- 要了帮助，就没有动词运行：结果不是 `run`。 -/
theorem help_runs_nothing (words : List String) (h : asksHelp words = true)
    (verb : Verb) (args : Args) :
    parse find arguments words ≠ .ok (.run verb args) := by
  intro heq
  unfold parse at heq
  split at heq
  · cases heq
  · split at heq
    · cases heq
    · split at heq
      · split at heq
        · cases heq
        · simp only [Except.map] at heq
          split at heq <;> cases heq
      · split at heq
        · cases heq
        · simp only [Except.bind] at heq
          split at heq
          · cases heq
          · simp_all

/-- 什么都没敲是首屏。 -/
theorem nothing_is_the_first_screen :
    parse find arguments ([] : List String) = .ok .firstScreen := by
  simp [parse, asksVersion, own]

/-- `--` 之后的 `--help` 是交给动词的文字。 -/
example : asksHelp ["call", "--", "--help"] = false := by decide
example : asksVersion ["dispatch", "lab", "-V"] = true := by decide

end Sprawling.Main.Grammar
