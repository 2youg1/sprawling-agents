-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# session：握手完成之前，工具不出手

规定 `crates/desktop/src/session.rs`（`desktop::session`，`Server::answer` 与 `Phase`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

MCP 的生命周期是 `initialize` → `notifications/initialized` → 其余。一台不强制这个次序的 server 会让客户端的次序错误在别处以别的形状爆出来（§10 第 2 条）。`Phase` 是 typestate 在运行期的投影：`Fresh`、`Initializing`、`Ready`。

三条性质：

1. **没收到 `notifications/initialized` 之前，`tools/list` 与 `tools/call` 恒被拒**（`no_tool_before_the_notification`）。
2. **两步握手之后工具可用**（`the_two_steps_open_the_tools`）。
3. **`ping` 在任何阶段都答**（`ping_is_answered_in_every_phase`）。

本模型写的是代码今天的转移：`notifications/initialized` 在任何阶段都让连接进 `Ready`，包括没有先收到 `initialize` 的 `Fresh`（入口 §4 记着这一处与 §10 第 2 条字面的出入）。

**拿掉阶段的核对，一个刚连上的客户端不握手就拿到工具表**（`withoutTheGate_lists_tools_on_a_fresh_connection`）：本模型咬得动的演示。
-/

namespace Desktop.Session

inductive Phase where
  | fresh
  | initializing
  | ready
  deriving DecidableEq, Repr

/-- 一行请求的方法：`opening` 是 `initialize`，`opened` 是 `notifications/initialized`，`list`、`call` 是 `tools/list`、`tools/call`。 -/
inductive Method where
  | opening
  | opened
  | ping
  | list
  | call
  | unknown
  deriving DecidableEq, Repr

/-- 一行之后的阶段。 -/
def next (phase : Phase) : Method → Phase
  | .opening => .initializing
  | .opened => .ready
  | .ping => phase
  | .list => phase
  | .call => phase
  | .unknown => phase

/-- 这一行是否被答成功（拒绝即 `false`）。 -/
def served (phase : Phase) : Method → Bool
  | .opening => true
  | .opened => true
  | .ping => true
  | .list => phase = .ready
  | .call => phase = .ready
  | .unknown => false

/-- 一串请求之后的阶段。 -/
def run (phase : Phase) (methods : List Method) : Phase :=
  methods.foldl next phase

theorem never_ready_without_the_notification (methods : List Method) :
    ∀ phase, phase ≠ .ready → .opened ∉ methods → run phase methods ≠ .ready := by
  induction methods with
  | nil => intro phase h _; exact h
  | cons m rest ih =>
    intro phase h absent
    simp only [List.mem_cons, not_or] at absent
    apply ih (next phase m)
    · cases m with
      | opening => simp [next]
      | opened => exact absent.1 rfl |>.elim
      | ping => exact h
      | list => exact h
      | call => exact h
      | unknown => exact h
    · exact absent.2

theorem no_tool_before_the_notification (methods : List Method)
    (absent : .opened ∉ methods) (tool : Method) (h : tool = .list ∨ tool = .call) :
    served (run .fresh methods) tool = false := by
  have notReady := never_ready_without_the_notification methods .fresh (by decide) absent
  rcases h with rfl | rfl <;> simp [served, notReady]

theorem the_two_steps_open_the_tools :
    served (run .fresh [.opening, .opened]) .list = true ∧
      served (run .fresh [.opening, .opened]) .call = true := by
  decide

theorem ping_is_answered_in_every_phase (phase : Phase) : served phase .ping = true := rfl

/-!
## 咬得动的演示

不核对阶段，`no_tool_before_the_notification` 不再成立：一个刚连上、什么都没说的客户端拿到工具表。
-/

def servedWithoutTheGate (_phase : Phase) : Method → Bool
  | .unknown => false
  | _ => true

theorem withoutTheGate_lists_tools_on_a_fresh_connection :
    servedWithoutTheGate (run .fresh []) .list = true := rfl

end Desktop.Session
