-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Acp：外部编辑器的请求，怎样变成一次普通的 Dispatch

规定 `crates/agent_protocols/src/acp.rs`（`agent_protocols::acp`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

配对在本模块之前判定：令牌住 `wire`，判定住入站中间件，未配对的请求到不了这里（`crates/agent_protocols/Spec.lean` §9）。本模块只守三条：

1. **入站文法只有一个入口**：`Incoming` 只能由 `parse` 造出，它造出的值三个字段都在、task 与 goal 不空（`a_parsed_request_has_every_field`）。令牌不是 `Incoming` 的字段，所以一个能被打印的请求值不可能带着令牌。
2. **持有效令牌也够不到 reserved prefix**（`an_admitted_request_never_names_the_citys_own_subtree`）。
3. **成为工作的请求恰是一次 Dispatch 的三个字段**（`admission_keeps_the_three_fields`），外部请求要不到人在控制面要不到的东西。正常路径可达（`an_ordinary_request_is_admitted`）。

地址的文法与「哪些段是 reserved」归 `kernel::address`（`Address::parse`、`is_reserved`），这里写成参数 `addressOk` 与 `reserved`，不另抄一份。
-/

namespace AgentProtocols.Acp

/-- 请求体里本模块读的三个键；别的键（令牌在内）不读，所以不是字段。缺席与非字符串都是 `none`。 -/
structure Body where
  addr : Option String
  task : Option String
  goal : Option String
  deriving DecidableEq, Repr

/-- 读过、尚未准入的请求。 -/
structure Incoming where
  addr : String
  task : String
  goal : String
  deriving DecidableEq, Repr

/-- 准入的结果只有一种：开一个 run。 -/
structure Dispatch where
  addr : String
  task : String
  goal : String
  deriving DecidableEq, Repr

/-- 一次拒绝：`E_INVALID_ARGS` 带缺的那个键，或 `E_OUTSIDE_WRITE_DOMAIN`。 -/
inductive Refusal where
  | missing (key : String)
  | badAddress
  | outsideWriteDomain
  deriving DecidableEq, Repr

/-- 一个非空的字符串值，否则报出缺的那个键。 -/
def text (key : String) : Option String → Except Refusal String
  | some value => if value = "" then .error (.missing key) else .ok value
  | none => .error (.missing key)

/-- `Incoming::parse`：先读地址并过它的文法，再读 task、goal；第一个读不成的键就是拒词里的那一个。 -/
def parse (addressOk : String → Bool) (body : Body) : Except Refusal Incoming :=
  match text "addr" body.addr with
  | .error e => .error e
  | .ok addr =>
    if addressOk addr then
      match text "task" body.task with
      | .error e => .error e
      | .ok task =>
        match text "goal" body.goal with
        | .error e => .error e
        | .ok goal => .ok ⟨addr, task, goal⟩
    else .error .badAddress

/-- `admit`：落在 reserved 里的地址拒，其余原样成为 Dispatch。 -/
def admit (reserved : String → Bool) (request : Incoming) : Except Refusal Dispatch :=
  if reserved request.addr then .error .outsideWriteDomain
  else .ok ⟨request.addr, request.task, request.goal⟩

theorem text_ok {key : String} {raw : Option String} {value : String}
    (h : text key raw = .ok value) : value ≠ "" := by
  cases raw with
  | none => simp [text] at h
  | some v =>
    by_cases e : v = ""
    · simp [text, e] at h
    · simp only [text, e, if_false, Except.ok.injEq] at h
      rw [← h]; exact e

/-- `parse` 造出的请求，地址过了文法，task 与 goal 不空。 -/
theorem a_parsed_request_has_every_field (addressOk : String → Bool) (body : Body)
    (request : Incoming) (h : parse addressOk body = .ok request) :
    addressOk request.addr = true ∧ request.task ≠ "" ∧ request.goal ≠ "" := by
  obtain ⟨a, t, g⟩ := request
  unfold parse at h
  cases ha : text "addr" body.addr with
  | error e => simp [ha] at h
  | ok addr =>
    cases hok : addressOk addr with
    | false => simp [ha, hok] at h
    | true =>
      cases ht : text "task" body.task with
      | error e => simp [ha, hok, ht] at h
      | ok task =>
        cases hg : text "goal" body.goal with
        | error e => simp [ha, hok, ht, hg] at h
        | ok goal =>
          simp [ha, hok, ht, hg] at h
          obtain ⟨rfl, rfl, rfl⟩ := h
          exact ⟨hok, text_ok ht, text_ok hg⟩

/-- 准入的请求恒不落在 reserved prefix 里，不论它持什么令牌。 -/
theorem an_admitted_request_never_names_the_citys_own_subtree (reserved : String → Bool)
    (request : Incoming) (dispatch : Dispatch) (h : admit reserved request = .ok dispatch) :
    reserved dispatch.addr = false := by
  unfold admit at h
  cases hr : reserved request.addr <;> simp [hr] at h
  rw [← h]; exact hr

/-- 准入不增不减：Dispatch 的三个字段就是请求的三个字段。 -/
theorem admission_keeps_the_three_fields (reserved : String → Bool) (request : Incoming)
    (dispatch : Dispatch) (h : admit reserved request = .ok dispatch) :
    dispatch.addr = request.addr ∧ dispatch.task = request.task ∧ dispatch.goal = request.goal := by
  unfold admit at h
  cases hr : reserved request.addr <;> simp [hr] at h
  rw [← h]; exact ⟨rfl, rfl, rfl⟩

/-- 正常路径可达：一份三个键都在、地址在一栋楼里的请求成为一次 Dispatch。 -/
theorem an_ordinary_request_is_admitted :
    (parse (fun _ => true) ⟨some "lab/room1", some "fix the kiln timer", some "the timer test passes"⟩
      >>= admit (fun _ => false)) =
      .ok ⟨"lab/room1", "fix the kiln timer", "the timer test passes"⟩ := by
  simp [parse, admit, text, bind, Except.bind]

/-!
## 咬得动的演示

拿掉 `admit` 里的 reserved 判断，一个持有效令牌的外部编辑器就能在城自己的子树上开工。
-/

def admitWithoutReserved (request : Incoming) : Except Refusal Dispatch :=
  .ok ⟨request.addr, request.task, request.goal⟩

theorem withoutReserved_reaches_the_citys_own_subtree :
    admitWithoutReserved ⟨".sprawling/ledger", "t", "g"⟩ = .ok ⟨".sprawling/ledger", "t", "g"⟩ := rfl

end AgentProtocols.Acp
