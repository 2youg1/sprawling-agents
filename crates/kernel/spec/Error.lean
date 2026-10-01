-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Event.Kind

/-!
# kernel::error

规定 `kernel::error`（`crates/kernel/src/error.rs` 与 `crates/kernel/src/error/` 下的 `code`、`shape`、`refusal`、`provider`）：码、恢复语必填的错误与它的构造路径、carrier 声明。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-1 kernel::error

**类型**：

```rust
pub enum AxCode { PathNotFound, /* …其余 variant，serde 呈现名见下表 */ }

pub struct GateRefusal {                // three-part refusal；三段必填
    rule: String, violation: String, alternative: String,
}

pub struct AxError { code: AxCode, detail: Box<ErrorDetail> }   // 线上是扁平的一个对象，序列化字段序＝声明序
struct ErrorDetail {
    action: String, subject: String, nearby: Vec<String>, recovery: String, retry: Retry,
    retry_after_ms: Option<u64>,        // 缺省即不写出；只随 Retry::Yes 出现
    gate: Option<GateRefusal>,
    provider: Option<ProviderFailureKind>, // 缺席时不上线
}

/// 同一个请求能否再发一次，连同它的效果是否已经落地；线上拼作 "yes"／"no"／"unknown"。
pub enum Retry {
    Yes,      // 效果没有落地，同一请求稍后可能成功：可以
    No,       // 同一请求再发一次注定同样失败：不可以
    Unknown,  // 请求已经发出而回答丢了，效果是否落地无从得知：不知道
}
pub enum ProviderFailureKind {  // 携 serde，内标签 kind
    Exchange, Cut, Silence, Refused { status: u32 }, Overflow { status: u32 }, Unreadable, Reported, Unbuilt,
}

pub enum Carrier { Event(EventKind), Loadtime }
impl AxCode {
    pub fn carrier(&self) -> Carrier;   // 穷尽 match，无 catch-all；唯一声明位
    pub fn as_str(&self) -> &'static str; // "E_…" 拼写，serde 与 Display 共用
}
```

**构造**：字段私有；两个构造子都返回 `ErrorDraft`，`ErrorDraft::with_recovery` 是通向 `AxError` 的唯一门。两条不变量因此由构造路径保证：「Gate 拒绝码 ⇒ gate 三段在场」，以及「凡构造出的 AxError 必带一句恢复语」。

```rust
impl AxError {
    /// Non-gate failure. `retry` defaults to `Retry::No` (fail-closed).
    pub fn failure(code: AxCode, action: impl Into<String>, subject: impl Into<String>) -> ErrorDraft;
    /// Gate refusal. Sets `gate` to the mandatory three parts.
    pub fn refusal(code: AxCode, action: impl Into<String>, subject: impl Into<String>, gate: GateRefusal) -> ErrorDraft;
    /// 模型调用失败的 E_PROVIDER：种类在场；`retry` 起于 `No`，由 gateway 的 `ProviderFailure::retry` 在 draft 上设定。
    pub fn provider(kind: ProviderFailureKind, action: impl Into<String>, subject: impl Into<String>) -> ErrorDraft;
    pub fn code(&self) -> &AxCode;  pub fn gate(&self) -> Option<&GateRefusal>;
    pub fn retry(&self) -> Retry;
    pub fn provider_failure(&self) -> Option<ProviderFailureKind>;
    /// 改写下层抛上来的恢复语；与 `ErrorDraft::with_recovery` 分名，因为它毁掉一句而不是补上第一句。
    #[must_use] pub fn rewrite_recovery(self, recovery: impl Into<String>) -> Self;
}

#[must_use = "an ErrorDraft becomes an AxError only through with_recovery"]
pub struct ErrorDraft { /* 私有：尚未拿到恢复语的那六个字段 */ }
impl ErrorDraft {
    pub fn with_nearby(self, nearby: Vec<String>) -> Self;
    pub fn retriable(self) -> Self;     // Retry::Yes，显式声明，默认 No
    pub fn retriable_after(self, wait_ms: u64) -> Self; // Retry::Yes，且对端说了最早何时再问
    pub fn effect_unknown(self) -> Self; // Retry::Unknown：请求已发出，回答丢了
    pub fn with_recovery(self, recovery: impl Into<String>) -> AxError;   // 唯一出口
}
```

**供应方失败的种类上线，句子不上线**：`provider` 只在 `AxError::provider` 造出的 E_PROVIDER 上在场（gateway 的 `provider_err` 是它的调用处；不是模型调用的 E_PROVIDER，如连接器的 http 客户端，不带种类）。客户端按 `kind` 从 `lang.json` 取人读的出路，城写的 `recovery` 折进「城的原话」供排错；城若只给英文句子，中文界面就只能照抄英文。种类不判「能不能再试一次」：那一判住在 gateway 的 `ProviderFailure::retry`，因为它要看种类之外的东西——连接是否建起（`Exchange` 分 `Yes` 与 `Unknown`）、拒绝的状态码（408／429／5xx 可重试）、流中报错的类型。**被否**：让种类另带一个 `is_retriable`——它看不见这些，会与 `retry` 成为同一事实的两个已经不一致的权威。

**恢复语必填由 typestate 执行，不由四参构造子执行**：`refusal` 已占满四参，再塞恢复语就是第五参，越过参数上限；而恢复语只有 `ErrorDraft::with_recovery` 一个设定处，四参方案则要把 `with_recovery` 留作改写器，同一个名字两份职责。于是「每个 AxError 带恢复语」由编译器检查，grep 查不了它。`AxError` 一经存在即已完工，`with_nearby`／`retriable` 只在 draft 上，故链式调用中 `with_recovery` 恒为最后一环。

**`retry_after_ms` 是失败方对「再问之前至少等多久」的陈述**，读取处是 `AxError::retry_after_ms() -> Option<u64>`。它只能经 `retriable_after` 写入，而该方法同时置 `Retry::Yes`；`effect_unknown` 置 `Unknown` 时一并清掉它，所以无论构造器按什么次序调用，「不是 `Yes` 却带等待」都拼不出来。`None` 时字段不上线、不入账；账本里没有这个键的行读作 `None`（`serde(default)`）。

**`retry` 是三态信封，不是布尔。** 一个 `bool` 只能说「可以」与「不可以」，于是请求已经发出、回答却丢了的那一类失败（流中断、静默超时、发送途中断开）只能被说成「可以」，而那是假话：对端也许已经执行了它、计了费，一个有副作用的动作再发一次就是第二次副作用。第三态 `Unknown` 把「效果是否已经落地」说进信封，决定再发与否的是知道这个动作是否幂等的调用方（runtime 的 watchdog 对一次模型调用照样退避重问，因为它的效果只是一份城里从未收到的回答）。落选的是另加一个 `landed: bool` 字段：「可以」蕴含未落地、「不可以」与落地无关，两字段会拼出「可以且已落地」这样没有意义的组合。线上字段名是 `retry`；`serde(alias = "retriable")` 让账本里写作 `retriable: true/false` 的行读作 `Yes`／`No`。落选的是把等待时长写进 `recovery` 文本：那是给读者的一句话，重试节奏的持有者（runtime 的 watchdog）不该从句子里解析数字。

「gate 码走 `refusal`」由构造纪律＋单测保证；`kernel::gate` 是全库唯一 gate 码生产者，citysim 不变量 8 号在系统层复验。derive `Serialize/Deserialize`（Ledger 载荷需要）、`Clone/Debug/PartialEq`；`thiserror::Error` 提供 Display（`{code}: {action} on {subject}`）。
-/

/-!
> 协作组的 `E_WORKTREE_BUSY` 与 `E_DIGEST_SUSPECT` 在各自 SPEC 里答过「能否定义掉」：不能，各自有一个真实的运行期情境。
-/

/-!
装载期七码（`E_CONFIG_INVALID` `E_CAS_CORRUPT` `E_STORAGE_FATAL` `E_WIRE_MISMATCH` `E_LOG_VERSION_UNSUPPORTED` `E_LEDGER_HELD` `E_HISTORY_UNPROVEN`）＝C9 唯一例外白名单；`Carrier::Loadtime` 即其类型面。白名单封闭，进表的条件只有一条：**这个码只在本进程此刻写不了账本时出现**，因为它若有账本可写，就必须有 carrier。每一码进表的理由逐条记在 `crates/kernel/Spec.lean` §12。
`E_BUSY` 的 carrier 与 `E_WORKTREE_BUSY` 一样是 `tool_result`，而两者的区别在名字里：那一个说的是工作树这个机制，这一个说的是**同一个地址上有 run 正在工作**，拒绝里点名那条 run，调用方据此先停它再动手。
两条呈现约束：`E_SECRET_EGRESS` 的 subject 只写 SecretRef 与位置、恒不回显命中字节；`E_DISCARD_IRREVERSIBLE` 的 alternative 必须可执行。执行点在各生产模块，此处记为 carrier 表随附契约。

**carrier() 依赖 `EventKind`**：故它与 `event` 模块同一变更集落码；本表与其余全部（AxError／GateRefusal／AxCode／serde／构造子）不依赖它。
-/

namespace Kernel.Error

open Kernel.Event.Kind

/-- 一个码在历史里由谁携带：它的 carrier 事件，或装载期（本进程此刻写不了账本，C9 唯一的例外）。与 `kernel::Carrier` 逐变体同名。 -/
inductive Carrier where
  | Event (kind : EventKind)
  | Loadtime
  deriving DecidableEq, Repr

/-- 错误码的闭集，与 `kernel::AxCode` 逐变体同名，`cargo xtask gates specalign` 双向对账。线上拼写由 `AxCode::as_str` 一处给出（`E_` 加全大写的蛇形词），本模型不抄那张拼写表，只证明它必须满足的性质（下面的 `parse_inverts_as_str`）。 -/
inductive AxCode where
  -- 基表
  | PathNotFound
  | ToolUnknown
  | ToolUnavailable
  | InvalidArgs
  | OutsideWriteDomain
  | VersionConflict
  | GateDenied
  | BudgetExhausted
  | Timeout
  | Provider
  | EvidenceMissing
  | LoopSuspected
  | LocatorInvalid
  | SandboxDenied
  | Busy
  -- 协作
  | DraftStale
  | GoalConflict
  | TaintedAction
  | RepairBusy
  | DelegationDepth
  -- 治理与设施
  | ApprovalPending
  | ApprovalDenied
  | CrossBuildingDenied
  | DigestSuspect
  | CredentialMissing
  | ModelUnchosen
  | ConfigInvalid
  | CasCorrupt
  | StorageFatal
  | WorktreeBusy
  | BrowserUnavailable
  | EndpointDialectUnsupported
  | WireMismatch
  | LogVersionUnsupported
  | LedgerHeld
  | HistoryUnproven
  -- 隐私与 Discard
  | SecretEgress
  | DiscardIrreversible
  -- 背压
  | BackpressureShed
  -- 运行未知
  | ToolOutcomeUnknown
  -- 计划
  | PlanMissing
  deriving DecidableEq, Repr

/-- carrier 的唯一声明位（`AxCode::carrier`，C9）：穷尽，没有通配臂，所以新增一个码而不给它 carrier 是编译错误。每一臂一行，`specalign` 逐臂与 kernel 编译出来的 `carrier()` 对账。

装载期那几臂是封闭的白名单，进表的条件只有一条：这个码只在本进程此刻写不了账本时出现；它若有账本可写，就必须有 carrier。每一码进表的理由在 `crates/kernel/Spec.lean` §12。 -/
def AxCode.carrier : AxCode → Carrier
  -- 基表
  | .PathNotFound => .Event .ToolResult
  | .ToolUnknown => .Event .ToolResult
  | .ToolUnavailable => .Event .ToolResult
  | .InvalidArgs => .Event .ToolResult
  | .OutsideWriteDomain => .Event .GateDenied
  | .VersionConflict => .Event .ToolResult
  | .GateDenied => .Event .GateDenied
  | .BudgetExhausted => .Event .BudgetLimit
  | .Timeout => .Event .ToolResult
  | .Provider => .Event .ProviderDegraded
  | .EvidenceMissing => .Event .ToolResult
  | .LoopSuspected => .Event .WatchdogFired
  | .LocatorInvalid => .Event .ToolResult
  | .SandboxDenied => .Event .ToolResult
  | .Busy => .Event .ToolResult
  -- 协作
  | .DraftStale => .Event .ToolResult
  | .GoalConflict => .Event .ToolResult
  | .TaintedAction => .Event .GateDenied
  | .RepairBusy => .Event .ToolResult
  | .DelegationDepth => .Event .GateDenied
  -- 治理与设施
  | .ApprovalPending => .Event .ApprovalRequested
  | .ApprovalDenied => .Event .ApprovalResolved
  | .CrossBuildingDenied => .Event .GateDenied
  | .DigestSuspect => .Event .ToolResult
  | .CredentialMissing => .Event .ToolResult
  | .ModelUnchosen => .Event .ToolResult
  | .ConfigInvalid => .Loadtime
  | .CasCorrupt => .Loadtime
  | .StorageFatal => .Loadtime
  | .WorktreeBusy => .Event .ToolResult
  | .BrowserUnavailable => .Event .ToolResult
  | .EndpointDialectUnsupported => .Event .EndpointLost
  | .WireMismatch => .Loadtime
  | .LogVersionUnsupported => .Loadtime
  | .LedgerHeld => .Loadtime
  | .HistoryUnproven => .Loadtime
  -- 隐私与 Discard
  | .SecretEgress => .Event .GateDenied
  | .DiscardIrreversible => .Event .GateDenied
  -- 背压
  | .BackpressureShed => .Event .ToolResult
  -- 运行未知
  | .ToolOutcomeUnknown => .Event .ToolResult
  -- 计划
  | .PlanMissing => .Event .ToolResult

/-- 每个码，依 `AxCode::ALL` 的次序。 -/
def AxCode.all : List AxCode := [
  .PathNotFound,
  .ToolUnknown,
  .ToolUnavailable,
  .InvalidArgs,
  .OutsideWriteDomain,
  .VersionConflict,
  .GateDenied,
  .BudgetExhausted,
  .Timeout,
  .Provider,
  .EvidenceMissing,
  .LoopSuspected,
  .LocatorInvalid,
  .SandboxDenied,
  .Busy,
  .DraftStale,
  .GoalConflict,
  .TaintedAction,
  .RepairBusy,
  .DelegationDepth,
  .ApprovalPending,
  .ApprovalDenied,
  .CrossBuildingDenied,
  .DigestSuspect,
  .CredentialMissing,
  .ModelUnchosen,
  .ConfigInvalid,
  .CasCorrupt,
  .StorageFatal,
  .WorktreeBusy,
  .BrowserUnavailable,
  .EndpointDialectUnsupported,
  .WireMismatch,
  .LogVersionUnsupported,
  .LedgerHeld,
  .HistoryUnproven,
  .SecretEgress,
  .DiscardIrreversible,
  .BackpressureShed,
  .ToolOutcomeUnknown,
  .PlanMissing
]

theorem AxCode.all_complete : ∀ code : AxCode, code ∈ AxCode.all := by
  intro code
  cases code <;> decide

/-- **装载期白名单恰是这七个码。** 白名单封闭：让一个码改走装载期，要先在这里改这条定理，而改它就是一次需要理由的决定（`crates/kernel/Spec.lean` §12 逐码写着理由）。 -/
theorem the_loadtime_whitelist_is_closed :
    AxCode.all.filter (fun code => code.carrier == .Loadtime) =
      [.ConfigInvalid, .CasCorrupt, .StorageFatal, .WireMismatch, .LogVersionUnsupported,
        .LedgerHeld, .HistoryUnproven] := by
  decide

/-- 门的拒绝码都由 `gate_denied` 携带：`kernel::gate` 是全库唯一的 gate 码生产者，它写下的拒绝落在同一种行上。 -/
theorem gate_codes_are_carried_by_gate_denied :
    AxCode.all.filter (fun code => code.carrier == .Event .GateDenied) =
      [.OutsideWriteDomain, .GateDenied, .TaintedAction, .DelegationDepth, .CrossBuildingDenied,
        .SecretEgress, .DiscardIrreversible] := by
  decide

/-- 读侧的解析（`AxCode::parse`）：按 `ALL` 的次序找第一个拼写相同的码，找不到即 `none`，由调用方走拒绝的那一支。拼写函数 `as_str` 是参数，它的值只住在 Rust。 -/
def parse (as_str : AxCode → String) (raw : String) : Option AxCode :=
  AxCode.all.find? (fun code => as_str code == raw)

private theorem find_first_spelled {α : Type} (spell : α → String)
    (injective : ∀ a b, spell a = spell b → a = b) :
    ∀ (codes : List α) (code : α), code ∈ codes →
      codes.find? (fun other => spell other == spell code) = some code := by
  intro codes code member
  induction codes with
  | nil => cases member
  | cons head tail ih =>
    rw [List.find?_cons]
    by_cases same : head = code
    · subst same
      simp
    · have differs : (spell head == spell code) = false := by
        simp only [beq_eq_false_iff_ne, ne_eq]
        exact fun equal => same (injective _ _ equal)
      rw [differs]
      simp only
      apply ih
      cases member with
      | head => exact absurd rfl same
      | tail _ rest => exact rest

/-- **每个码的拼写读回它自己，条件是没有两个码共用一个拼写。** 账本、线上帧与客户端都按拼写读码，所以这一条就是「码的拼写稳定」在读侧的意思：写下的拼写读回的恒是写它的那个码。Rust 的 `axcode_parse_roundtrips_every_variant` 逐值检查真实的拼写表，`axcode_is_the_declared_length_and_spelling_is_bijective` 检查它是单射。 -/
theorem parse_inverts_as_str (as_str : AxCode → String)
    (injective : ∀ a b, as_str a = as_str b → a = b) (code : AxCode) :
    parse as_str (as_str code) = some code :=
  find_first_spelled as_str injective AxCode.all code (AxCode.all_complete code)

/-- 单射不是摆设：两个码共用一个拼写时，其中一个永远读不回来。这里 `ToolUnknown` 与 `PathNotFound` 同拼，读回的是排在前面的 `PathNotFound`。 -/
theorem a_shared_spelling_loses_a_code :
    parse (fun code => if code = .PathNotFound ∨ code = .ToolUnknown then "E_PATH_NOT_FOUND" else "other")
        "E_PATH_NOT_FOUND" ≠ some .ToolUnknown := by
  decide

/-- 同一个请求能否再发一次，连同它的效果是否已经落地。与 `kernel::Retry` 逐变体同名。 -/
inductive Retry where
  | Yes
  | No
  | Unknown
  deriving DecidableEq, Repr

/-- `ErrorDraft` 上与重试有关的两个字段：`retry` 与 `retry_after_ms`。 -/
structure Draft where
  retry : Retry
  retry_after_ms : Option Nat
  deriving DecidableEq, Repr

/-- `AxError::failure`／`refusal`／`provider` 造出的 draft：`retry` 起于 `No`（fail-closed），不带等待。 -/
def Draft.fresh : Draft := { retry := .No, retry_after_ms := none }

/-- `ErrorDraft` 上改重试的三个方法。 -/
inductive Step where
  | retriable
  | retriable_after (wait_ms : Nat)
  | effect_unknown

/-- 三个方法各自做的事，照 `error::shape` 的实现：`retriable_after` 记下等待再置 `Yes`；`effect_unknown` 置 `Unknown` 并清掉等待；`retriable` 只置 `Yes`。 -/
def Draft.apply (draft : Draft) : Step → Draft
  | .retriable => { draft with retry := .Yes }
  | .retriable_after wait => { retry := .Yes, retry_after_ms := some wait }
  | .effect_unknown => { retry := .Unknown, retry_after_ms := none }

/-- 「带着等待」蕴含「可以再试」。 -/
def Draft.coherent (draft : Draft) : Prop :=
  draft.retry_after_ms.isSome = true → draft.retry = .Yes

/-- **不论构造器按什么次序调用，「不是 `Yes` 却带等待」都拼不出来。** 这是 `retry_after_ms` 只能经 `retriable_after` 写入、`effect_unknown` 一并清掉它这两条实现选择要买的性质。 -/
theorem no_order_of_calls_waits_without_retrying (steps : List Step) :
    (steps.foldl Draft.apply Draft.fresh).coherent := by
  have keep : ∀ (draft : Draft) (step : Step), draft.coherent → (draft.apply step).coherent := by
    intro draft step held
    cases step with
    | retriable => intro _; rfl
    | retriable_after _ => intro _; rfl
    | effect_unknown => intro waits; simp [Draft.apply] at waits
  have start : Draft.fresh.coherent := by
    intro waits
    simp [Draft.fresh] at waits
  suffices ∀ (draft : Draft), draft.coherent → (steps.foldl Draft.apply draft).coherent from
    this _ start
  induction steps with
  | nil => intro draft held; exact held
  | cons step rest ih => intro draft held; exact ih _ (keep draft step held)

/-- 正常路径可实现：一次 `retriable_after` 之后，draft 可以再试且带着那段等待。 -/
example : Draft.fresh.apply (.retriable_after 500) = { retry := .Yes, retry_after_ms := some 500 } :=
  rfl

end Kernel.Error
