-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 命令行的优先规则

规定 `crates/sprawling/src/main/grammar.rs` 的 `parse` 在读任何动词的参数之前做的那几个决定（§8-89）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。命令表 `VERBS`（`main/verbs.rs`）是数据，按名找行（`find`）与读一行的参数（`arguments`）在模型里是参数，因为这里要钉的只是次序：

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

/-- `--` 之后的 `--help` 是交给动词的文字。 -/
example : asksHelp ["call", "--", "--help"] = false := by decide
example : asksVersion ["dispatch", "lab", "-V"] = true := by decide

end Sprawling.Main.Grammar

/-!
## 8-89 一张命令表，一个纯解析器（`bin::main::verbs`、`bin::main::grammar`）

优先规则的权威是本文件上面的模型：版本先于一切，帮助先于任何动词运行，`--` 之后的词不参与这两个判断，空行是首屏；本节是接口与做法。

**形状。** `main/verbs.rs` 是 data：一张静态表 `VERBS`，每个动词一行 `Row { verb, name, aliases, positionals, flags, says, effect }`。位置参数是 `(名字, Need::Required | Need::Optional)`；标志是 `Flag { name, takes: Takes::Nothing | Takes::Value(<占位名>), says }`；`effect` 是 `Effect::ReadsOnly | Effect::Changes`，标出这个动词运行时会不会改一座城或它所在的主机。总览（`help`、`--help`）、单个动词的帮助（`help <verb>`、`<verb> --help`）与首屏退出时的清单都由这张表生成，没有第二份手写的命令清单或用法字符串。

`main/grammar.rs` 是 grammar：纯函数，不做 I/O。

```rust
pub(super) fn parse(words: &[String]) -> Result<Invocation, LineError>;
pub(super) enum Invocation { FirstScreen, Overview, Help(Verb), Version, Run(Verb, Arguments) }
impl Arguments {
    pub(super) fn positional(&self, nth: usize) -> Option<&String>; // 从 1 数
    pub(super) fn has(&self, flag: &str) -> bool;
    pub(super) fn value(&self, flag: &str) -> Option<&str>;        // 同一标志给两次，后者胜
}
pub(super) enum LineError {
    UnknownVerb { given, nearest }, UnknownFlag { verb, given, nearest },
    MissingValue { verb, flag }, MissingPositional { verb, name }, ExtraPositional { verb, given },
}
```

**优先规则。** `--version`/`-V` 出现在任何位置都得 `Version`；其次 `--help`/`-h` 出现在任何位置都得那个动词的 `Help`（没有动词时得 `Overview`）。两者都在任何动词运行之前决定，所以 `up --help` 不写创世记录、`install --help` 不改 PATH。`version` 是 `status` 的别名，`enroll` 是 `enrol` 的别名。

**标志与位置参数。** 以 `--` 开头的词（单独的 `-` 除外，它是 `call` 的「从 stdin 读帧」）只按该动词这一行的标志表读；带值的标志吃掉下一个词，所以 `serve <city> --log debug` 的地址取默认值而不是 `debug`。表里没有的标志是 `UnknownFlag`，多出来的位置参数是 `ExtraPositional`，缺了必填的位置参数是 `MissingPositional`。一个词恰好等于这一行某个标志的短名（`dispatch` 的 `-m`）时总读作那个标志，哪怕它本想是位置参数：`sprawling dispatch lab -m` 缺的是 `-m` 的值而不是任务。要把 `-m` 当任务文字交出，把它放进更长的一句里。

**命令行错误。** 每个 `LineError` 在 stderr 上写一行 `sprawling: <哪个参数、错在哪>. Did you mean '<最近的合法写法>'?`，退出码 2，不再倾倒整张表。恢复语就是那条「最近的写法」：`nearest` 取与所敲的词共享最长前缀（至多四个字符）的已知名字，所以 `stauts` 得 `["status"]`。

**本节接口的当前状态。** `call`、`enrol`、`install`、`fork`、`status`、`serve` 的标志值与 `doctor` 的全部参数，仍由各自的函数从原始 argv 读（`router::flag_value`、`args.iter().any`），解析器只替它们校验；改为只读 `Arguments` 之后 `flag_value` 删除。README、README.zh-CN、LLM.md、`docs/operating.md` 里的命令语法块还是手写的，改由 `cargo xtask docnum` 从这张表生成（一个名为 `cli_verbs` 的受管区块）是下一步；`nearest` 与 `console::language` 里找近似动词的那一份是同一条规则的两份，合并到一处也是下一步。

**决定。**

1. 不用 clap。命令表是数据，解析器约两百行；启动时间几乎全是操作系统的开销（Windows x86-64 桌面级机器上，`--version` 首字节 7.98 ms，空进程下限 5.40 ms），没有给一个参数库的依赖、编译时间与体积留出位置。重新考虑的条件：动词需要子动词或 shell 补全以外的、这张表表达不了的结构。命令表的一行可以叫两个词（`playback export`），解析先试头两个词，这是子动词在这张表里的全部写法（§8-126）。
2. 不用 `+` 前缀区分动词。现有动词不改名，一个词仍然是一个动词，文档与肌肉记忆都不必迁移。
-/
