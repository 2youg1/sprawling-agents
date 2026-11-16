-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::answering::github

规定 `crates/accounting/src/views/answering/github.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-18-3 `gh` 的候选 ID 从哪一处答（`accounting::views::answering::github`，形状 1 判定）

```rust
// accounting::views::answering::github
pub(crate) const DEFAULT_HOST: &str = "github.com";
pub(crate) fn github_answer(ask: Option<fn(&str) -> wire::GithubReading>, host: Option<&str>) -> wire::Answer;
// accounting::views::served
impl Views { pub fn ask_github_through(&mut self, login: fn(&str) -> wire::GithubReading); }
// bin::doctor::github（sprawling）
pub(crate) fn login(host: &str) -> wire::GithubReading;
```

- **视图只判两件事。** 问哪台主机（`host` 缺席即 `DEFAULT_HOST`），以及它是不是一个主机名（字母、数字、`.`、`-`、`:`，不以 `-` 开头、不为空）；不是就答 `NotAHost`，不调用交进来的函数。其余交给交进来的那一个，在 `Prepared::finish` 里、快照放开之后调用（D13 同形）。没有交的视图答 `Unavailable`。
- **二进制跑 gh。** `bin::doctor::github::login` 经 `doctor::host::find_program("gh")` 找程序，找不到答 `NoCli`；找到就起 `gh api --hostname <host> user --jq .login`，stdin 为空、stdout 接管道、stderr 丢弃、`GH_PROMPT_DISABLED=1`，以固定间隔数次数等它结束（与 `doctor::running` 同一种有界的等），数完仍未结束就经 `running::stop` 停下它。退出码 0 且 stdout 第一行是一个 login 答 `Found`；4 答 `NotLoggedIn`；其余答 `Failed { exit }`；起不来、被停下或 0 却读不出 login 答 `Failed`（起不来与被停下时 `exit` 为 `None`）；超时之后 `running::stop` 也停不下它时答 `Stuck`，带 `stop` 说的原因。退出码到答复的映射是一个纯函数，三种答复各有一条测试。
- 验收：`views::answering::github` 的 `a_github_login_is_asked_of_the_reader_the_views_were_handed`；`bin::doctor::github` 的 `no_gh_on_the_search_path_is_no_cli`、`exit_four_is_not_logged_in`、`a_login_on_the_first_line_is_found`。
-/

/-! ## 模型：视图只判主机，跑 gh 的是交进来的那一个

`githubAnswer` 是 `github_answer` 的读法：没给主机就问 `DEFAULT_HOST`；没人交读者的视图答 `Unavailable`；不是主机名的串答 `NotAHost`，交进来的读者不被调用。主机名按字符的列表写（Rust 的 `chars()`），所以判定在内核里算得出。本模型不是 Rust 实现的证明，对应由 `views::answering::github` 的测试检查（§16）。
-/

namespace Accounting.Views.Answering.Github

/-- 一个字符能不能出现在交给 `gh --hostname` 的值里：ASCII 字母、数字、`.`、`-`、`:`。 -/
def hostChar (c : Char) : Bool :=
  ('a' ≤ c && c ≤ 'z') || ('A' ≤ c && c ≤ 'Z') || ('0' ≤ c && c ≤ '9') ||
    c == '.' || c == '-' || c == ':'

/-- `is_host`：不为空、不以 `-` 开头、每个字符都是主机名的字符。 -/
def isHost : List Char → Bool
  | [] => false
  | c :: cs => c != '-' && hostChar c && cs.all hostChar

def defaultHost : List Char := ['g', 'i', 't', 'h', 'u', 'b', '.', 'c', 'o', 'm']

/-- 一次导入的答：没人交读者、不是主机、或读者对这台主机说的话。 -/
inductive Reply (A : Type) where
  | Unavailable (host : List Char)
  | NotAHost (host : List Char)
  | Read (host : List Char) (reading : A)
  deriving Repr, DecidableEq

def githubAnswer {A : Type} (ask : Option (List Char → A)) (host : Option (List Char)) : Reply A :=
  let h := host.getD defaultHost
  match ask with
  | none => .Unavailable h
  | some login => if isHost h then .Read h (login h) else .NotAHost h

/-- 不是主机名的串答 `NotAHost`，不论交进来的读者是什么，它都不被调用。 -/
theorem a_string_that_is_no_host_never_reaches_gh {A : Type} (login login' : List Char → A)
    (h : List Char) (hh : isHost h = false) :
    githubAnswer (some login) (some h) = .NotAHost h ∧
      githubAnswer (some login) (some h) = githubAnswer (some login') (some h) := by
  simp [githubAnswer, hh]

/-- 没人交读者的视图答 `Unavailable`，不碰这台电脑。 -/
theorem views_nobody_served_answer_unavailable {A : Type} (host : Option (List Char)) :
    githubAnswer (none : Option (List Char → A)) host = .Unavailable (host.getD defaultHost) := by
  rfl

/-- 正常路径可实现：不给主机就问 `github.com`；以 `-` 开头的串被当作一个选项拒掉。 -/
theorem the_default_host_is_asked_and_an_option_is_refused :
    githubAnswer (some fun h => h.length) none = .Read defaultHost 10 ∧
      githubAnswer (some fun h => h.length) (some ['-', 'x']) = .NotAHost ['-', 'x'] := by
  decide

end Accounting.Views.Answering.Github
