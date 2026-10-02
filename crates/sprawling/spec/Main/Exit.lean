-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 退出码是一张表

规定 `crates/sprawling/src/main/exit.rs` 的 `Exit`，与 `crates/sprawling/src/main/calling.rs` 把 `wire_client::Unheard` 读成退出码的那一处（§8-103）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

退出码是给调用方（人或 agent）的一个事实，所以五个码各说一件事，任何两件不共用一个数。`Unheard` 的三种原因各落到它真正断言的那一件上：帧读不懂是命令行的错，地址上没有城与城拒绝不是一回事，握手之后断开是一次对话失败而不是没有城。

拒绝的文字（`refusal::written` 的 `Form::Human` 与 `Form::Json`）是文字的事，这里不建模。
-/

namespace Sprawling.Main.Exit

/-- 进程的结局（Rust：`Exit`），次序即 Rust 的变体次序。 -/
inductive Exit where
  | done
  | refused
  | line
  | quiet
  | noCity
  deriving Repr, DecidableEq

/-- 数字只在这里出现一次（Rust：`From<Exit> for ExitCode`）。 -/
def Exit.code : Exit → Nat
  | .done => 0
  | .refused => 1
  | .line => 2
  | .quiet => 3
  | .noCity => 4

/-- 没听到回答的原因（Rust：`wire_client::Unheard`）。 -/
inductive Unheard where
  /-- 帧在开 socket 之前就解析失败。 -/
  | unreadable
  /-- 连接、发送问候或等 `Welcome` 失败。 -/
  | noCity
  /-- 握手之后连接断了，或本进程起不了 runtime。 -/
  | broken
  deriving Repr, DecidableEq

/-- `calling` 把一个 `Unheard` 读成退出码的唯一一处。 -/
def Unheard.exit : Unheard → Exit
  | .unreadable => .line
  | .noCity => .noCity
  | .broken => .refused

/-- 五个码两两不同：一个码只断言一件事。 -/
theorem code_injective (a b : Exit) (h : a.code = b.code) : a = b := by
  cases a <;> cases b <;> simp [Exit.code] at h ⊢

/-- 只有做完了才是 0，脚本据此判断成败。 -/
theorem zero_is_done (e : Exit) : e.code = 0 ↔ e = .done := by
  cases e <;> simp [Exit.code]

/-- 码落在 0 到 4 之间，任何平台的退出码都装得下。 -/
theorem code_small (e : Exit) : e.code ≤ 4 := by
  cases e <;> simp [Exit.code]

/-- §8-103 决定 1：没有城单列为 4，与「城读了这一帧并拒绝」的 1 分开，agent 据此决定是改帧重试还是起城、改 `--at`。 -/
theorem no_city_is_not_a_refusal : Unheard.noCity.exit ≠ Exit.refused := by decide

/-- §8-103 决定 2：帧写错退 2：错在这条命令行本身，不与城的真实拒绝共用 1。 -/
theorem unreadable_frame_is_the_line : Unheard.unreadable.exit = .line := rfl

/-- §8-103 决定 3：握手之后断开归 1：那时已经有城答过 `Welcome`。 -/
theorem broken_after_welcome_is_refused : Unheard.broken.exit = .refused := rfl

/-- 没听到回答从不读作做完了。 -/
theorem unheard_is_never_done (u : Unheard) : u.exit ≠ .done := by
  cases u <;> decide

end Sprawling.Main.Exit

/-!
## 8-103 退出码是一张表（`bin::main::exit`、`bin::main::calling`、`bin::main::refusal`、`bin::wire_client`）

五个码两两不同、`Unheard` 落到哪个码、决定 1–3 的权威是本文件上面的模型；本节是接口、文字的写法与理由。

```rust
pub(super) enum Exit { Done, Refused, Line, Quiet, NoCity }   // 0 1 2 3 4
impl From<Exit> for std::process::ExitCode;
pub(crate) enum Unheard { Unreadable(AxError), NoCity(AxError), Broken(AxError) }
pub(crate) fn call(at: &str, frame: &str, token: Option<&str>, quiet: Duration) -> Result<Heard, Unheard>;
pub(super) enum Form { Human, Json }
pub(super) fn written(err: &AxError, form: Form) -> String;
```

| 码 | `Exit` | 它断言的事实 |
|---|---|---|
| 0 | `Done` | 做完了 |
| 1 | `Refused` | 城或所在机器拒绝了，stderr 上有 `AxError` 的失败行与 recovery 行 |
| 2 | `Line` | 这条命令行读不懂，包括 `call` 的帧不是 wire 能载的东西 |
| 3 | `Quiet` | `call` 与 `dispatch`：帧发出去了，安静窗口内什么都没回来（`Spoken::Quiet`），或城回了话而等的那一帧没来（`Spoken::Unfinished`） |
| 4 | `NoCity` | `--at` 那里没有城在答：连不上，或连上了却没有回握手 |

`Exit` 是退出码的唯一定义；数字只在 `From<Exit> for ExitCode` 里出现一次。`wire_client::call` 用 `Unheard` 说清没听到回答的原因：帧解析失败是 `Unreadable`（映射到 2），连接、发送问候或等 `Welcome` 失败是 `NoCity`（映射到 4），握手之后连接断了或本进程起不了 runtime 是 `Broken`（映射到 1）。三者都照常在 stderr 上印 `AxError` 的失败行与 recovery 行。

拒绝怎么写由 `refusal::written` 一处决定，它只管文字，不管写到哪里；调用方把结果写到 stderr。`Form::Human` 是失败行、`recovery:` 行，`AxError` 带 `nearby` 时再加一行 `nearby: a, b`，给人读的拒绝也指出附近能用的名字。`Form::Json` 是一行 `AxError` 的 serde（与 wire 上 `Refusal` 同形），反序列化回来与原值相等。`call` 带 `--json` 时用 `Form::Json`，其余调用方用 `Form::Human`。

**本节接口的当前状态。** `call` 与 `dispatch` 经 `Exit` 退出；其余动词返回 `ExitCode`，经 `city::report` 得 1。`--json` 目前只有 `call` 接受。还没做的是：其余动词的 `--json`、kernel 的命令行错误码（先改 `crates/kernel/spec/Error.lean` 的 `AxCode` 表）、以及「动词 × 模式（终端、管道、`--json`）→ 退出码与输出流」的整张表。`doctor` 已经只在 stdout 是终端且没有 `NO_COLOR` 时着色（§8-59）。

**决定。**

1. 没有城单列为 4，而不与 1 共用。1 说的是城读了这一帧并拒绝，一个 agent 据此改帧重试；没有城时帧没有被任何城读过，该做的是起城或改 `--at`。两件事共用一个码，就把后者读成了前者。
2. 帧写错退 2 而不是 1。帧在开 socket 之前解析，错在这条命令行本身，与城无关；被否决的备选是沿用 `WireMismatch` 的 1，它让一个拼错的帧和城的真实拒绝无法区分。
3. 握手之后断开归 1 而不是 4。那时已经有城答过 `Welcome`，城在；断开是这次对话的失败，不是地址上没有城。
4. `--json` 的拒绝是 `AxError` 自己的 serde，而不是另起一个命令行专用的 JSON 形状。wire 上的 `Refusal` 已经是这个形状，一个 agent 用同一个反序列化读城的拒绝和命令行的拒绝；另起一种形状，就要在两处维持同一组字段。
-/
