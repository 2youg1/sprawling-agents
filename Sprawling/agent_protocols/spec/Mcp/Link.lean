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

/-! 对侧可能已经执行过的那些失败（交出之后），恒不被自动再发。 -/

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

/-! HTTP 会话的寿命由传输持有，Residents 只消费失效状态；这里不表示 HTTP 字节、
Mutex 或线程交错。Rust 回归从 Residents.tools 与 McpTool.invoke 进入，验证真实握手次序、
重新 listing、原调用不重发、Unknown 不重开与克隆共享失效。

`crates/accounting/src/worker/mcp.rs` 的
`failure_traces_keep_shared_lifetime_and_choose_the_next_dispatch` 从本模型派生 proptest：
初态 live 由真实 handshake 建立，初态 ended 由带 id 的 404 前缀建立；生成器覆盖
Failure 三个构造子、空序列及不含 sessionEnded 的序列，并在每步选择两个工具克隆之一。
HTTP fixture 在收到请求后不回 HTTP response 表示 beforeHandover，2xx 的非 UTF-8 body
表示 afterHandover，带 id 的 404 表示 sessionEnded；无 id 的 404 是模型之外的边界，
另检查它保持 live。ended 之后的候选失败不再交给网络：旧句柄须拒绝且不发送，
因此这些候选输入代表 ended 的吸收后缀，不宣称 server 实际执行过这些失败。
期望值取下面三个定理的前提与结论：含 sessionEnded 或初态 ended 时必须 connect，
其余保持 resident；检查实际请求序列、两份工具句柄、重握手与新 listing，
不另写一份 Rust afterFailures 状态机。固定用例另走同一个 trace driver：
ended 初态与空后缀必须重连，live session 的单个 NotFound 必须使两个旧句柄失效，
且从任意一个克隆发起都得到相同结果。有限生成检查不是对 Rust 的全 trace 证明。 -/

/-- 没有会话 id 的连接仍可用，不能把「没有 id」当成「已经失效」。 -/
inductive Lifetime where
  | live
  | ended
  deriving DecidableEq, Repr

/-- 一次 HTTP 失败后，只有带会话 id 的 404 使原连接失效。 -/
def afterFailure (held : Lifetime) : Failure → Lifetime
  | .sessionEnded => .ended
  | .beforeHandover => held
  | .afterHandover => held

/-- 居民表下一次派活复用连接或重新打开、握手和 listing。 -/
inductive Dispatch where
  | resident
  | connect
  deriving DecidableEq, Repr

def nextDispatch : Lifetime → Dispatch
  | .live => .resident
  | .ended => .connect

/-- 对任意失败序列，旧连接一旦失效就不复活；重新握手必须打开新连接。 -/
def afterFailures : Lifetime → List Failure → Lifetime
  | held, [] => held
  | held, failure :: rest => afterFailures (afterFailure held failure) rest

theorem ended_remains_ended (failures : List Failure) :
    afterFailures .ended failures = .ended := by
  induction failures with
  | nil => rfl
  | cons failure rest ih => cases failure <;> simpa [afterFailures, afterFailure] using ih

/-- sessionEnded 之后的任意失败序列都要求下一 dispatch 重连。 -/
theorem session_ended_requires_connection (held : Lifetime) (failures : List Failure) :
    nextDispatch (afterFailures held (.sessionEnded :: failures)) = .connect := by
  simp [afterFailures, afterFailure, ended_remains_ended, nextDispatch]

/-- 没有 sessionEnded 的失败序列保持原寿命，效果未知不触发重握手。 -/
theorem other_failures_preserve_lifetime (held : Lifetime) (failures : List Failure)
    (noEnded : ∀ failure ∈ failures, failure ≠ .sessionEnded) :
    afterFailures held failures = held := by
  induction failures generalizing held with
  | nil => rfl
  | cons failure rest ih =>
      have tail : ∀ f ∈ rest, f ≠ .sessionEnded := by
        intro f member
        exact noEnded f (List.mem_cons_of_mem failure member)
      have head := noEnded failure (List.mem_cons_self)
      cases failure <;> simp_all [afterFailures, afterFailure]

end AgentProtocols.Mcp.Link
