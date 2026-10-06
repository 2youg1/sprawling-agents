-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::dialect::responses

规定 `dialect::responses`（`crates/gateway/src/dialect/responses.rs`）：第三支笔：OpenAI responses 面。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

请求与最终 reply 的接口由文字描述；stream 的保留性质由下方模型证明，Rust 实现由 `gateway::dialect::responses::reply`、`gateway::dialect::responses::request`、`gateway::dialect::responses::stream` 旁的测试守住。
-/

/-!
### 8-20 第三支笔：OpenAI responses 面

`dialect/responses/` 三个文件——`request.rs`（规范请求 → `input` 数组）、`reply.rs`（`output` 数组 ↔ `ChatResponse`）、`stream.rs`（具名事件 → `Increment`，终帧 → 已定答案）。翻译入口与 stream 保留路由对 `DialectKind` 穷尽。

**形状取自供应方自己的规格**：`openai/openai-openapi` 的 `openapi.yaml`，读到的提交只记在 docs/third-party.md §1，那一行看着的就是这一面的请求、`output` 项与具名流事件。不取自任何客户端库，也不取自记忆。看着不对的字段通常是搬过家的字段——改这里之前先读那份文档。

**它不是第二支笔的一个开关。** chat 面发 `messages`、读 `choices`；这面发 `input`、读 `output`——一个数组，成员是消息、函数调用与推理项，而不是一条挂着若干字段的消息。流是第三套文法：具名事件（`response.output_text.delta`）而不是匿名 chunk。折在一起就是一个在每一步上分支的写入器。

**这面载得动而 chat 面载不动的两件事：**

- **显式缓存断点**。标了 `cache` 的 `SystemBlock` 成为带 `prompt_cache_breakpoint` 的 `input_text` 片段。本城本来就在算段边界，于是把它说出来，而不是交给前缀匹配去猜。system 段因此走 `input[0]`（`role: developer`，一段一片）而非 `instructions`——后者是一个字符串，装不下四段与它们的边界。
- **流自带已定答案**。终帧（`response.completed`／`incomplete`／`failed`）携整个 response 对象，重组读它而不是缝合碎片。**一个解析器因此得以保持**：流式调用与阻塞调用交给 `response_from` 的是同样的字节，两者得不出不同的结论。

**它主动放弃的一件事**：上一轮的 `Thinking` 块不回发。这面只接受带供应方自己签发的标识与密文的 `reasoning` 项，本城两样都不存；拼一个出来会在第一次调用处被拒。规范记录仍留着那个块，历史不缺东西。

**读不认识的输出项不算失败**：`Item::Unread`——内建工具调用本城从没要过，本 build 之后新增的项也一样；本城要的字段就在它们旁边，全都在。判定写成具名枚举而不是 `_ =>`，于是「放过去」是一个有名字的决定。

**停止原因来自整体而非某条消息**：这面报的是 response 的 `status`，以及 `incomplete` 时的 `reason`。产出了调用就是 `ToolUse`（无论 status 说什么——调用方两种情况下都有一个调用要跑），`incomplete` ＋ `max_output_tokens` 是 `MaxTokens`，`completed` 是 `EndTurn`，其余拒绝而不猜。

**`ConnectionKind::wire()` 从此指向真的那一支**（§8-18 那条「那一天到来时改的是 `wire()` 的一条臂」兑现）：`Responses` → `OpenAiResponses`。

**`store: false` 每条请求都写**：本城自持历史，上游留一份副本就是第二份，且它比本城「决定忘记」这个动作活得更久。

**`provider::stability` 的守卫因此长出第三条读法**：system 文本在这面是 `input[0].content[*].text`。那道闸的两条断言（两次派活逐字节相等、上线文本与交给供应层的几块逐字节相等且前面不多任何东西）现在覆盖全部三种连接。
-/

/-! D27 EOF 结算从已保留帧投影出最后带 `response` 字段的 terminal 与首个 reported error。
「有效」指可读的事件名且 `response` 字段存在，不提前验证 response 内容，最终仍由
`response_from` 决定；缺字段的 terminal 不覆盖已有答案，存在的 null 仍交给原解析器拒绝。
端点保留全部帧到 EOF，error 不阻断后续 delta；读失败或 silence 先于结算的 error 与 terminal。
两槽在线保留方案虽减少长流存储与耗时，却在匹配测量中出现小流回退，因此采用帧缓冲；
性能接受条件与读数由 `tools/xtask/budgets.toml` 的 `[responses_stream]` 持有。
落选：提前返回 error 会改变 EOF 与错误优先级；在线 projection 尚不满足性能接受条件。
模型只描述 EOF 结算的语义 projection，不描述读取期间的帧数或字节数；Nat 是不透明
response、error、delta 身份，解析器不是模型的第二份。
`responses_keep_the_lean_trace_properties` 按任意事件轨迹比较完整结算结果与增量。
-/
namespace Gateway.Dialect.Responses

inductive Frame where
  | terminal (response : Option Nat)
  | reported (code : Nat)
  | delta (text : Nat)
  | unread
  deriving DecidableEq

structure Projection where
  terminal : Option Nat := none
  reported : Option Nat := none
  deriving DecidableEq

def observe (held : Projection) : Frame → Projection
  | .terminal (some response) => { held with terminal := some response }
  | .terminal none => held
  | .reported code => { held with reported := held.reported.or (some code) }
  | .delta _ => held
  | .unread => held

def project (frames : List Frame) (held : Projection := {}) : Projection :=
  frames.foldl observe held

/-- 首个 error 在任意后续轨迹中保持。 -/
theorem first_reported_survives (frames : List Frame) (held : Projection) (code : Nat)
    (first : held.reported = some code) : (project frames held).reported = some code := by
  induction frames generalizing held with
  | nil => exact first
  | cons frame frames ih =>
    apply ih
    cases frame with
    | terminal response => cases response <;> simpa [observe] using first
    | reported next => simp [observe, first]
    | delta text => exact first
    | unread => exact first

/-- 不携 response 的 terminal 与 delta 都不能抹去已定答案。 -/
theorem terminal_survives (frames : List Frame) (held : Projection)
    (noRevision : ∀ frame ∈ frames, ∀ response, frame ≠ .terminal (some response)) :
    (project frames held).terminal = held.terminal := by
  induction frames generalizing held with
  | nil => rfl
  | cons frame frames ih =>
    have tail := ih (observe held frame) (by
      intro f member response
      exact noRevision f (List.mem_cons_of_mem frame member) response)
    change (project frames (observe held frame)).terminal = held.terminal
    rw [tail]
    cases frame with
    | terminal response =>
      cases response with
      | none => rfl
      | some response => exact False.elim (noRevision _ (by simp) response rfl)
    | reported code => rfl
    | delta text => rfl
    | unread => rfl

/-- 任意前缀与不再修订的后缀之间，最后一份 response 作答。 -/
theorem last_terminal_wins (before after : List Frame) (response : Nat)
    (noRevision : ∀ frame ∈ after, ∀ value, frame ≠ .terminal (some value)) :
    (project (before ++ [.terminal (some response)] ++ after)).terminal = some response := by
  simp only [project, List.foldl_append, List.foldl_cons, List.foldl_nil]
  exact terminal_survives after _ noRevision

def increment : Frame → Option Nat
  | .delta text => some text
  | .terminal _ | .reported _ | .unread => none

/-- reported error 不改变后缀增量的交付。 -/
theorem reported_keeps_deltas (before after : List Frame) (code : Nat) :
    (before ++ [Frame.reported code] ++ after).filterMap increment =
      before.filterMap increment ++ after.filterMap increment := by
  simp [List.filterMap_append, List.filterMap_cons, increment]

end Gateway.Dialect.Responses
