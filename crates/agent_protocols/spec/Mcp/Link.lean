-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Link：请求交出之后丢了答，效果未知

规定 `crates/agent_protocols/src/mcp/link.rs`（`agent_protocols::mcp::link`，一台可达的 server，不论经哪种传输）背后三种传输共同守的一条：一次失败可不可以重试，取决于请求有没有交给对侧。三种传输各自实现它：`crates/agent_protocols/src/mcp/stdio.rs`、`crates/agent_protocols/src/mcp/http.rs`、`crates/agent_protocols/src/mcp/sse.rs`，读回来的答复被拒时都经 `mcp::reading::answer_unread`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

「交出」的刻度是传输的：stdio 是那一行写完并 flush，HTTP 与 SSE 是 POST 得到回应。交出之后期限内没有答案（`E_TIMEOUT`）、对侧在作答前关了输出或流断了（`E_TOOL_UNAVAILABLE`）、答复被读端拒（`E_WIRE_MISMATCH`），都标 `Retry::Unknown`：server 可能已经做了，再发一次可能把一次写做两遍，由看得见这次调用的人决定要不要再问。交出之前的失败（写管道失败、POST 本身失败）按对侧没收下读，`Retry::No`。HTTP 上带会话 id 的 404 是唯一的 `Retry::Yes`：按规范 server 对已结束会话的请求一律答 404，调用没有执行。

剩余情形，模型不分：stdio 的行体写完、换行没写出去时管道断了，对侧可能在输入结束时把没有换行的最后一行当一条消息读；这一刻本 crate 分不出，按没收下报（`crates/agent_protocols/Spec.lean` §11）。
-/

namespace AgentProtocols.Mcp.Link

/-- kernel 的三态重试（`kernel::Retry`）。 -/
inductive Retry where
  | yes
  | no
  | unknown
  deriving DecidableEq, Repr

/-- 一次调用失败在哪一刻。 -/
inductive Failure where
  /-- 请求还没交给对侧：写管道失败、POST 本身失败。 -/
  | beforeHandover
  /-- 请求已交出，期限内没有答案、对侧关了输出、流断了，或答复被读端拒。 -/
  | afterHandover
  /-- HTTP 上带着会话 id 的 404：server 结束了会话，这次调用没有执行。 -/
  | sessionEnded
  deriving DecidableEq, Repr

/-- 一次失败的重试三态。 -/
def retryOf : Failure → Retry
  | .beforeHandover => .no
  | .afterHandover => .unknown
  | .sessionEnded => .yes

/-- 一个照 `Retry` 行事的调用者：`yes` 就再发一次，其余不发。`acted` 是对侧有没有执行过第一次。
返回这件事一共被做了几次。 -/
def performed (acted : Bool) (r : Retry) : Nat :=
  (if acted then 1 else 0) + (if r = .yes then 1 else 0)

/-- 对侧可能已经执行过的那些失败（交出之后），恒不被自动再发。 -/
theorem a_call_handed_over_is_never_sent_again : retryOf .afterHandover ≠ .yes := by decide

/-- 不论对侧做没做，照重试三态行事的调用者都不会把一次写做两遍：唯一自动再发的失败，
是对侧明说没有执行的那一种。 -/
theorem no_failure_is_performed_twice (f : Failure) (acted : Bool)
    (ranOnlyIfHandedOver : f ≠ .afterHandover → acted = false) :
    performed acted (retryOf f) ≤ 1 := by
  cases f <;> cases acted <;> simp_all [performed, retryOf]

/-!
## 咬得动的演示

把「交出之后丢了答」标成可重试，对侧已经执行过的一次调用会被做第二遍。
-/

/-- 把交出之后的失败当成可重试的分类。 -/
def retryOfEager : Failure → Retry
  | .beforeHandover => .no
  | .afterHandover => .yes
  | .sessionEnded => .yes

theorem eagerRetry_performs_twice : performed true (retryOfEager .afterHandover) = 2 := by decide

end AgentProtocols.Mcp.Link
