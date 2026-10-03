-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::endpoint::failure

规定 `endpoint::failure`（`crates/gateway/src/endpoint/failure.rs`）：一次 provider 调用哪里失败了、同一请求能不能再发，与恢复语。本文件是 `crates/gateway/Spec.lean` 的一个分部；别处引它的决定作 `gateway D<n>`。

本分部没有 §8 的节：「能否再试一次」与「窗口溢出」是两条决定（D2、D3），各自站在它所管的定义正上方。模型证明：没发出去的请求再发、发出去丢了回答的效果不明、对端非 2xx 恰在 408／429／5xx 时再问、溢出从不再试。
-/

namespace Gateway.Endpoint.Failure

/-- 同一个请求能不能再发一次，与它的效果落没落在对端（`kernel::Retry` 的三态）。 -/
inductive Retry where
  | Yes
  | No
  | Unknown
  deriving DecidableEq, Repr

/-- 一次 provider 调用哪里失败了（`endpoint::failure::ProviderFailure`）。模型只留下判「能否再试」要读的那一格：`Exchange` 带 `reqwest::Error::is_connect` 的答案，`Refused` 与 `Overflow` 带状态码，`Reported` 带流里那一帧的类型或码。 -/
inductive ProviderFailure where
  | Exchange (connecting : Bool)
  | Cut
  | Silence
  | Refused (status : Nat)
  | Overflow (status : Nat)
  | Unreadable
  | Reported (kind : String)
  | Unbuilt
  deriving DecidableEq, Repr

/-- 对端说「现在忙或坏了」的状态码：408、429 与 5xx（含 Anthropic 的 529）。 -/
def busy (status : Nat) : Bool :=
  status == 408 || status == 429 || (decide (500 ≤ status) && decide (status ≤ 599))

/-- 流里报错帧说「现在忙或坏了」的那几个类型或码。 -/
def RETRIABLE_KINDS : List String :=
  ["server_error", "rate_limit_exceeded", "overloaded_error", "api_error", "rate_limit_error",
    "timeout_error"]

/-! D3 窗口溢出是自己的一族：`Overflow`

。对端以 400 或 413 拒绝、且拒词（小写比对）含 `context_length_exceeded`、`prompt is too long`、`maximum context length` 或 `context window` 之一，由 `ProviderFailure::refusal(url, status, &body)` 判为 `Overflow`，否则为 `Refused`；读不出拒词的拒绝只按状态码算 `Refused`。`Overflow` 不标 retriable（同一请求再发一遍同样放不下），恢复语写城里真有的出路：`/new --carry` 带着摘要开新会话，或换一个窗口更大的模型。`Refused` 的恢复语让人去查模型名、凭证与兼容格式，对溢出而言这三项都没错，照做只会改坏。对侧正文只读来分类，subject 仍只写 URL 与状态码，恒不回显。落选的是「只按 413 判溢出」：两家兼容格式都用 400 报窗口溢出，只看状态码会漏掉最常见的那一种。
-/

/-- `ProviderFailure::refusal`。`outgrew` 是拒词（小写比对）含 `WINDOW_MARKERS` 之一；读不出拒词时它为假，拒绝只按状态码算。 -/
def refusal (status : Nat) (outgrew : Bool) : ProviderFailure :=
  if (status == 400 || status == 413) && outgrew then .Overflow status else .Refused status

/-! D2 「能否再试一次」只有一个家：`endpoint::failure::ProviderFailure`

。调用点只说它看见了哪一种失败，retriable 与恢复语由该枚举一处给出，恒不在调用点第二次判定。往返未完成（send／execute／读体／读帧／静默超时）＝`Exchange`／`Cut`／`Silence`，按请求是否已经发出分两态（kernel 的三态 `Retry`）：连接没建起来的 `Exchange` 标 `Yes`，请求没有到达对端；其余标 `Unknown`，因为请求已经发出、回答丢了，对端也许已经算完并计了费，恢复语如实说出这一点。对端已答非 2xx＝`Refused`，按状态码分两类：408（对端等请求等到超时）、429（限流）、5xx（含 Anthropic 的 529 过载）是对端说「现在忙或坏了」，同一请求稍后可能成功，标 `retriable`，恢复语说由 watchdog 退避后再问；其余 4xx 是对端说「这个请求本身不对」，不标，恢复语指向端点的模型名、凭证与兼容格式。body 形状不可读（`Unreadable`）与本侧 `.build()` 失败（`Unbuilt`，确定性重演同一失败）不标。retriable 只是 opt-in：退避节奏与上限住 runtime 的 watchdog，本 crate 不循环。对端在可重试的拒绝上给了等待时长时，`Refused` 把它带进 `AxError::retry_after_ms`（`ErrorDraft::retriable_after`）：先读 `retry-after-ms`（毫秒，OpenAI 发它），再读 `retry-after` 的秒数形式；HTTP 日期形式不读，落回 watchdog 自己的退避表，因为把日期换成时长要在本 crate 采样时钟，而时钟只在 `bin::assembly` 采样。读不成数的头同样不读，不报错：它只是一个提示，退避表本身已经成立。对端在已答 200 的流里报错＝`Reported { kind }`，三种兼容格式各有一种拼法：Anthropic 发 `type: error` 帧，`kind` 取 `error.type`；OpenAI chat 发一个带 `error` 对象而没有 `choices` 的块，`kind` 取 `error.code`，没有 code 时取 `error.type`（限流把类别放在 `type`、原因放在 `code`）；Responses 发 `type: error` 事件，`kind` 取顶层 `code`。分类只在 `ProviderFailure::retry` 一处：OpenAI 的 `server_error`、`rate_limit_exceeded` 与 Anthropic 的 `overloaded_error`、`api_error`、`rate_limit_error`、`timeout_error` 是对端说「现在忙或坏了」，标 `Yes`；其余（如 `insufficient_quota`、`invalid_prompt`）不标。主题只点名那个类型，不引对端的 message，与 `Refused` 不引 body 同理。落选的是把它当作普通的流中断：那样对端说出的原因被丢掉，读者只看见「流没有说完」。流在给出停止原因之前结束（`mismatch::stream_cut`）标 `Unknown`，与 `Cut` 同理。
-/

/-- `ProviderFailure::retry`，一种失败一臂。 -/
def retry : ProviderFailure → Retry
  | .Exchange true => .Yes
  | .Exchange false => .Unknown
  | .Cut => .Unknown
  | .Silence => .Unknown
  | .Refused status => if busy status then .Yes else .No
  | .Overflow _ => .No
  | .Reported kind => if RETRIABLE_KINDS.contains kind then .Yes else .No
  | .Unreadable => .No
  | .Unbuilt => .No

/-- 对端答了非 2xx：恰在它说「现在忙或坏了」时再问，其余的拒绝下一次照样会给。 -/
theorem a_refusal_is_asked_again_exactly_when_the_provider_is_busy (status : Nat) :
    retry (.Refused status) = .Yes ↔ busy status = true := by
  simp only [retry]
  cases busy status <;> simp

/-- 拒词说窗口满了、状态码是 400 或 413，才是溢出；溢出从不再试，同一请求再发一遍同样放不下。被否的「只按 413 判溢出」漏掉最常见的那一种：两家兼容格式都以 400 报窗口溢出，本定理的 `status = 400` 一支覆盖它。 -/
theorem a_refusal_is_an_overflow_exactly_when_it_names_the_window (status : Nat) (outgrew : Bool) :
    refusal status outgrew = .Overflow status ↔ ((status = 400 ∨ status = 413) ∧ outgrew = true) := by
  unfold refusal
  by_cases window : (status == 400 || status == 413) && outgrew
  · simp only [window, if_true]
    simp [Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at window
    simp [window]
  · simp only [window]
    simp [Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at window
    constructor
    · intro same
      cases same
    · intro ⟨code, said⟩
      simp [window code] at said

theorem an_overflow_is_never_asked_again (status : Nat) : retry (.Overflow status) = .No := rfl

/-- 流里的报错帧：恰在它的类型或码说「现在忙或坏了」时再问。 -/
theorem a_reported_error_is_asked_again_exactly_when_it_says_busy (kind : String) :
    retry (.Reported kind) = .Yes ↔ RETRIABLE_KINDS.contains kind = true := by
  simp only [retry]
  cases RETRIABLE_KINDS.contains kind <;> simp

end Gateway.Endpoint.Failure
