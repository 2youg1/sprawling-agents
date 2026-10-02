-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 退出码是一张表

规定 `crates/sprawling/src/main/exit.rs` 的 `Exit`，与 `crates/sprawling/src/main/calling.rs` 把 `wire_client::Unheard` 读成退出码的那一处（sprawling-SPEC.md 8-103）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

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

/-- D1：没有城单列为 4，与「城读了这一帧并拒绝」的 1 分开，agent 据此决定是改帧重试还是起城、改 `--at`。 -/
theorem no_city_is_not_a_refusal : Unheard.noCity.exit ≠ Exit.refused := by decide

/-- D2：帧写错退 2：错在这条命令行本身，不与城的真实拒绝共用 1。 -/
theorem unreadable_frame_is_the_line : Unheard.unreadable.exit = .line := rfl

/-- D3：握手之后断开归 1：那时已经有城答过 `Welcome`。 -/
theorem broken_after_welcome_is_refused : Unheard.broken.exit = .refused := rfl

/-- 没听到回答从不读作做完了。 -/
theorem unheard_is_never_done (u : Unheard) : u.exit ≠ .done := by
  cases u <;> decide

end Sprawling.Main.Exit
