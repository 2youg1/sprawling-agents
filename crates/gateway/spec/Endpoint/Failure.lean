-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Error

/-!
# gateway::endpoint::failure

规定 `endpoint::failure`（`crates/gateway/src/endpoint/failure.rs`）：一次 provider 调用哪里失败了、同一请求能不能再发，与恢复语。本文件是 `crates/gateway/Spec.lean` 的一个分部；别处引它的决定作 `gateway D<n>`。

本分部没有 §8 的节：「能否再试一次」「窗口溢出」与「换不换账号」是三条决定（D2、D3、D32），各自站在它所管的定义正上方。模型证明：没发出去的请求再发、发出去丢了回答的效果不明、普通的拒绝恰在 408／429／5xx 时再问、溢出从不再试、额度用尽不再问；换账号恰在 401、结构化的额度用尽与流内的 `insufficient_quota` 时成立，效果不明的失败从不换账号，换账号的失败都不再发。
-/

namespace Gateway.Endpoint.Failure

open Kernel.Error (AccountDisposition)

/-- 同一个请求能不能再发一次，与它的效果落没落在对端（`kernel::Retry` 的三态）。 -/
inductive Retry where
  | Yes
  | No
  | Unknown
  deriving DecidableEq, Repr

/-- 一次 provider 调用哪里失败了（`endpoint::failure::ProviderFailure`）。模型只留下判「能否再试」与「换不换账号」要读的那一格：`Exchange` 带 `reqwest::Error::is_connect` 的答案，`Refused`、`AccountUnavailable` 与 `Overflow` 带状态码，`Reported` 带流里那一帧的类型或码。 -/
inductive ProviderFailure where
  | Exchange (connecting : Bool)
  | Cut
  | Silence
  | Refused (status : Nat)
  | AccountUnavailable (status : Nat)
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

/-- `ProviderFailure::refusal`。`outgrew` 是拒词（小写比对）含 `WINDOW_MARKERS` 之一；`quota` 是拒绝体的结构化 `error.code`（缺席或为 null 时取 `error.type`）恰为 `insufficient_quota`（D32）。读不出拒绝体时两者都为假，拒绝只按状态码算。 -/
def refusal (status : Nat) (outgrew quota : Bool) : ProviderFailure :=
  if (status == 400 || status == 413) && outgrew then .Overflow status
  else if status == 429 && quota then .AccountUnavailable status
  else .Refused status

/-! D2 「能否再试一次」只有一个家：`endpoint::failure::ProviderFailure`

。调用点只说它看见了哪一种失败，retriable 与恢复语由该枚举一处给出，恒不在调用点第二次判定。往返未完成（send／execute／读体／读帧／静默超时）＝`Exchange`／`Cut`／`Silence`，按请求是否已经发出分两态（kernel 的三态 `Retry`）：连接没建起来的 `Exchange` 标 `Yes`，请求没有到达对端；其余标 `Unknown`，因为请求已经发出、回答丢了，对端也许已经算完并计了费，恢复语如实说出这一点。对端已答非 2xx＝`Refused`，按状态码分两类：408（对端等请求等到超时）、429（限流；结构化地说额度用尽的 429 是 D32 的 `AccountUnavailable`）、5xx（含 Anthropic 的 529 过载）是对端说「现在忙或坏了」，同一请求稍后可能成功，标 `retriable`，恢复语说由 watchdog 退避后再问；其余 4xx 是对端说「这个请求本身不对」，不标，恢复语指向端点的模型名、凭证与兼容格式。body 形状不可读（`Unreadable`）与本侧 `.build()` 失败（`Unbuilt`，确定性重演同一失败）不标。retriable 只是 opt-in：退避节奏与上限住 runtime 的 watchdog，本 crate 不循环。对端在可重试的拒绝上给了等待时长时，`Refused` 把它带进 `AxError::retry_after_ms`（`ErrorDraft::retriable_after`）：先读 `retry-after-ms`（毫秒，OpenAI 发它），再读 `retry-after` 的秒数形式；HTTP 日期形式不读，落回 watchdog 自己的退避表，因为把日期换成时长要在本 crate 采样时钟，而时钟只在 `bin::assembly` 采样。读不成数的头同样不读，不报错：它只是一个提示，退避表本身已经成立。对端在已答 200 的流里报错＝`Reported { kind }`，三种兼容格式各有一种拼法：Anthropic 发 `type: error` 帧，`kind` 取 `error.type`；OpenAI chat 发一个带 `error` 对象而没有 `choices` 的块，`kind` 取 `error.code`，没有 code 时取 `error.type`（限流把类别放在 `type`、原因放在 `code`）；Responses 发 `type: error` 事件，`kind` 取顶层 `code`。分类只在 `ProviderFailure::retry` 一处：OpenAI 的 `server_error`、`rate_limit_exceeded` 与 Anthropic 的 `overloaded_error`、`api_error`、`rate_limit_error`、`timeout_error` 是对端说「现在忙或坏了」，标 `Yes`；其余（如 `insufficient_quota`、`invalid_prompt`）不标。主题只点名那个类型，不引对端的 message，与 `Refused` 不引 body 同理。落选的是把它当作普通的流中断：那样对端说出的原因被丢掉，读者只看见「流没有说完」。流在给出停止原因之前结束（`mismatch::stream_cut`）标 `Unknown`，与 `Cut` 同理。
-/

/-- `ProviderFailure::retry`，一种失败一臂。 -/
def retry : ProviderFailure → Retry
  | .Exchange true => .Yes
  | .Exchange false => .Unknown
  | .Cut => .Unknown
  | .Silence => .Unknown
  | .Refused status => if busy status then .Yes else .No
  | .AccountUnavailable _ => .No
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
theorem a_refusal_is_an_overflow_exactly_when_it_names_the_window (status : Nat) (outgrew quota : Bool) :
    refusal status outgrew quota = .Overflow status ↔ ((status = 400 ∨ status = 413) ∧ outgrew = true) := by
  unfold refusal
  by_cases window : (status == 400 || status == 413) && outgrew
  · simp only [window, if_true]
    simp [Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at window
    simp [window]
  · simp only [window]
    simp [Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at window
    constructor
    · intro same
      by_cases exhausted : (status == 429 && quota) = true <;> simp [exhausted] at same
    · intro ⟨code, said⟩
      simp [window code] at said

theorem an_overflow_is_never_asked_again (status : Nat) : retry (.Overflow status) = .No := rfl

/-! D32 「这个账号还能不能接这个请求」与「能否再试」同住 `endpoint::failure`：`ProviderFailure::account_disposition`

**决定**：每一种失败除了 `retry` 还答一格 `AccountDisposition`（`crates/kernel/spec/Error.lean` D53），也只在这里答一次，`ProviderFailure` 造 `AxError` 时经 `ErrorDraft::account_unusable` 写进去。答 `Advance` 的恰是三种：对端以 401 拒绝（这个 Key 不被认）；对端以 429 拒绝且拒绝体的结构化 `error.code`（缺席或为 null 时取 `error.type`）恰为 `insufficient_quota`，判为 `AccountUnavailable`；已答 200 的流里报错帧的 `kind` 恰为 `insufficient_quota`。其余一律 `Keep`：408、限流的 429、5xx 与列明的流内错误是对端忙，原号稍后再问（`Retry::Yes`）；400、403、404、422 是请求、地域、权限或模型的问题，没有证据说换一个账号就能修好，所以不再发也不换号；`Overflow`、`Unreadable`、`Unbuilt` 与账号无关；往返未完成的 `Exchange`、`Cut`、`Silence` 不换号，其中效果不明的那几种由 `unknown_keeps_account` 守住。

`AccountUnavailable` 是 D3 之外的又一族：429 本来说「忙」，而额度用尽不是忙，等多久再问都一样，所以它的 `retry` 是 `No`。落选的是保留 `Yes` 只加 `Advance`：单账号端点会对一个用尽的额度退避重试到 `Halt`，多账号时还要让选号模块推翻 `retry`，那是同一事实的第二个权威。它上线时的种类是 `ProviderFailureKind::Quota { status }`，恢复语说出真有的出路：给这个账号补额度，或在 Provider 页再加一个账号；`Refused` 的恢复语（查模型名、密钥与兼容格式）对它是错的出路。分类只读拒绝体里结构化的 `error` 对象，不读 message 文本，subject 仍只写 URL 与状态码；读体失败、坏 JSON、403 都不授换号。判 `Overflow` 先于判额度，与 D3 的次序一致。

端点兑付凭据（`endpoint::redemption`）造的 `E_CREDENTIAL_MISSING`——这个账号的引用在 vault 里不在、环境变量也没给——同样标 `Advance`：缺的是这个账号的凭据，下一个账号可能有。vault 锁住或不可用的错误不标，换号修不好共用的存储（`crates/kernel/spec/AccountRecovery.lean` D55）。

**信源**：OpenAI 的错误码文档把 429「You exceeded your current quota」与 429「Rate limit reached for requests」分成两条，前者的出路是充值或换计划（<https://platform.openai.com/docs/guides/error-codes>）；`insufficient_quota` 是 OpenAI 兼容格式拒绝体里 `error.code`／`error.type` 的取值。Anthropic 兼容格式以别的类型报计费问题，今天不判为 `Advance`。只凭 429 的状态码不证明额度用尽。

**与 Rust 的对应**：`account_disposition` ↔ `ProviderFailure::account_disposition`，`refusal` 的 `quota` ↔ `ProviderFailure::refusal` 读拒绝体的那一步；`endpoint::failure` 旁的测试遍历状态码 × 窗口拒词 × 额度标记，逐格与下面的 `a_refusal_advances_exactly_for_a_rejected_key_or_an_exhausted_quota`、`a_refusal_is_an_overflow_exactly_when_it_names_the_window` 对拍；它是行为比对，不是精化证明。

**重开参数**：某个供应方以别的结构化码报额度用尽，或 403 带上了可读的「这个 Key 被停用」的结构化码。
-/

/-- `ProviderFailure::account_disposition`，一种失败一臂（D32）。 -/
def account_disposition : ProviderFailure → AccountDisposition
  | .Exchange _ => .Keep
  | .Cut => .Keep
  | .Silence => .Keep
  | .Refused status => if status == 401 then .Advance else .Keep
  | .AccountUnavailable _ => .Advance
  | .Overflow _ => .Keep
  | .Unreadable => .Keep
  | .Reported kind => if kind == "insufficient_quota" then .Advance else .Keep
  | .Unbuilt => .Keep

/-- 对端以状态码拒绝时，换号恰在两种情形：401，或结构化的额度用尽随 429 而来。一个只说忙的 429、以及 400／403／404／422 都不换号。 -/
theorem a_refusal_advances_exactly_for_a_rejected_key_or_an_exhausted_quota (status : Nat)
    (outgrew quota : Bool) :
    account_disposition (refusal status outgrew quota) = .Advance ↔
      (status = 401 ∨ (status = 429 ∧ quota = true)) := by
  unfold refusal
  split
  · rename_i window
    simp [Bool.and_eq_true, Bool.or_eq_true, beq_iff_eq] at window
    rcases window with ⟨code | code, _⟩ <;> simp [account_disposition, code]
  · split
    · rename_i exhausted
      simp [Bool.and_eq_true, beq_iff_eq] at exhausted
      simp [account_disposition, exhausted]
    · rename_i exhausted
      simp [Bool.and_eq_true, beq_iff_eq] at exhausted
      simp only [account_disposition]
      split <;> simp_all

/-- 流里的报错帧：恰在它的类型或码是 `insufficient_quota` 时换号。 -/
theorem a_reported_error_advances_exactly_for_an_exhausted_quota (kind : String) :
    account_disposition (.Reported kind) = .Advance ↔ kind = "insufficient_quota" := by
  simp [account_disposition]

/-- 效果不明的失败从不换号：请求已经发出，换号会在另一个账号上再计一次费，而丢了回答不证明是账号的问题。 -/
theorem unknown_keeps_account (failure : ProviderFailure) (lost : retry failure = .Unknown) :
    account_disposition failure = .Keep := by
  cases failure with
  | Exchange connecting => cases connecting <;> simp_all [retry, account_disposition]
  | Cut => rfl
  | Silence => rfl
  | Refused status => simp only [retry] at lost; split at lost <;> cases lost
  | AccountUnavailable status => cases lost
  | Overflow status => cases lost
  | Unreadable => cases lost
  | Reported kind => simp only [retry] at lost; split at lost <;> cases lost
  | Unbuilt => cases lost

/-- 说「换号」的失败都说「不再发」，与 `ErrorDraft::account_unusable` 同时置 `Retry::No` 一致（kernel D53）。 -/
theorem an_advance_is_never_asked_again (failure : ProviderFailure)
    (advances : account_disposition failure = .Advance) : retry failure = .No := by
  cases failure with
  | Exchange connecting => cases connecting <;> simp [account_disposition] at advances
  | Cut => cases advances
  | Silence => cases advances
  | Refused status =>
    by_cases rejected : status = 401
    · simp [retry, busy, rejected]
    · simp [account_disposition, rejected] at advances
  | AccountUnavailable status => rfl
  | Overflow status => cases advances
  | Unreadable => cases advances
  | Reported kind =>
    by_cases quota : kind = "insufficient_quota"
    · simp [retry, RETRIABLE_KINDS, quota]
    · simp [account_disposition, quota] at advances
  | Unbuilt => cases advances

/-- 流里的报错帧：恰在它的类型或码说「现在忙或坏了」时再问。 -/
theorem a_reported_error_is_asked_again_exactly_when_it_says_busy (kind : String) :
    retry (.Reported kind) = .Yes ↔ RETRIABLE_KINDS.contains kind = true := by
  simp only [retry]
  cases RETRIABLE_KINDS.contains kind <;> simp

end Gateway.Endpoint.Failure
