-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# mei::agent 的反馈模型

本模型只描述 `Mei/src/agent.rs` 的串行控制面：一个可变 Messages 历史、一次响应、
一波预检、顺序执行与追加。HTTP、Python 异常、进程死亡的外部效果不在模型内；
`Exec` 返回 `none` 表示私有信道或解释器死亡，此时允许此前效果已经发生，但不伪造
完整的 tool-result 消息。
-/

namespace Mei.Agent

inductive Role where
  | user | assistant
  deriving DecidableEq, Repr

inductive Block where
  | text (body : String)
  | thinking (body signature : String)
  | python (name id code : String)
  | client (name id code : String)
  | server (name : String)
  | result (id output : String)
  deriving DecidableEq, Repr

inductive Content where
  | string (text : String)
  | items (blocks : List Block)
  deriving DecidableEq, Repr

structure Message where
  role : Role
  content : Content
  deriving DecidableEq, Repr

abbrev History := List Message

inductive Stop where
  | endTurn | toolUse | pauseTurn | maxTokens | other
  deriving DecidableEq, Repr

structure Response where
  stop : Stop
  content : Content
  deriving DecidableEq, Repr

structure Call where
  name : String
  id : String
  code : String
  deriving DecidableEq, Repr

/-- `server_tool_use` 不进入客户端工具波；其它客户端块按名称参与整波预检。 -/
def Block.call : Block → Option Call
  | .python name id code => some ⟨name, id, code⟩
  | .client name id code => some ⟨name, id, code⟩
  | .text _ | .thinking _ _ | .server _ | .result _ _ => none

def Content.calls : Content → List Call
  | .string _ => []
  | .items blocks => blocks.filterMap Block.call

def Content.text : Content → String
  | .string text => text
  | .items blocks => blocks.foldr (fun block rest =>
      match block with
      | .text body => body ++ rest
      | _ => rest) ""

def validCall (call : Call) : Bool :=
  call.name == "python" && !call.id.isEmpty

def distinctIds : List String → Bool
  | [] => true
  | id :: rest => !rest.contains id && distinctIds rest

/-- 一波至少一个调用、全部只能调用 python、ID 非空且本波唯一。 -/
def validWave (calls : List Call) : Bool :=
  !calls.isEmpty && calls.all validCall && distinctIds (calls.map Call.id)

abbrev Exec := List Call → Option (List String)

inductive Outcome where
  | again (history : History) (outputs : List String)
  | answered (history : History) (text : String)
  | failed (history : History)
  deriving DecidableEq, Repr

def Outcome.history : Outcome → History
  | .again history _ => history
  | .answered history _ => history
  | .failed history => history

/-- 当前迭代的可观察效果；协议错误在任何 Python 效果之前失败。 -/
def Outcome.effects : Outcome → List String
  | .again _ outputs => outputs
  | .answered _ _ | .failed _ => []

def assistant (content : Content) : Message := ⟨.assistant, content⟩

def userResults (calls : List Call) (outputs : List String) : Message :=
  ⟨.user, .items ((calls.zip outputs).map (fun pair => .result pair.1.id pair.2) )⟩

/-- 先追加 assistant；整波通过预检后才调用 `exec`，随后才追加同一条 user 结果消息。 -/
def step (exec : Exec) (history : History) (response : Response) : Outcome :=
  let appended := history ++ [assistant response.content]
  match response.stop with
  | .endTurn => .answered appended response.content.text
  | .pauseTurn => .again appended []
  | .maxTokens | .other => .failed appended
  | .toolUse =>
    let calls := response.content.calls
    if !validWave calls then .failed appended
    else match exec calls with
      | none => .failed appended
      | some outputs =>
        if outputs.length != calls.length then .failed appended
        else .again (appended ++ [userResults calls outputs]) outputs

/-- 每次转移都以刚收到的 assistant 消息开头，历史只有追加，没有改写。 -/
theorem assistant_precedes_every_outcome (exec : Exec) (history : History) (response : Response) :
    ∃ suffix, (step exec history response).history =
      history ++ [assistant response.content] ++ suffix := by
  cases response with | mk stop content =>
  cases stop <;> simp [step, Outcome.history]
  by_cases valid : validWave content.calls = true
  · simp [valid]
    cases ran : exec content.calls with
    | none => exact ⟨[], rfl⟩
    | some outputs =>
      by_cases same : outputs.length = content.calls.length
      · exact ⟨[userResults content.calls outputs], by simp [same]⟩
      · exact ⟨[], by simp [same]⟩
  · exact ⟨[], by simp [valid]⟩

/-- 不合法的一波在任何执行效果之前终止；执行闭包不会被调用。 -/
theorem invalid_wave_has_no_effects (exec : Exec) (history : History) (content : Content)
    (invalid : validWave content.calls = false) :
    (step exec history ⟨.toolUse, content⟩).effects = [] := by
  simp [step, invalid, Outcome.effects]

/-- 合法的一波按 content 顺序产出同 ID、同数量的结果，然后才进入下一轮。 -/
theorem valid_wave_preserves_call_order (exec : Exec) (history : History) (content : Content)
    (outputs : List String)
    (valid : validWave content.calls = true)
    (ran : exec content.calls = some outputs)
    (sameLength : outputs.length = content.calls.length) :
    step exec history ⟨.toolUse, content⟩ =
      .again (history ++ [assistant content] ++ [userResults content.calls outputs]) outputs := by
  simp [step, valid, ran, sameLength]

/-- `pause_turn` 只回传 assistant，不添加 user，也不执行客户端工具。 -/
theorem pause_reposts_assistant_only (exec : Exec) (history : History) (content : Content) :
    step exec history ⟨.pauseTurn, content⟩ =
      .again (history ++ [assistant content]) [] := by
  simp [step]

/-- `max_tokens` 与其它 stop reason 不执行可能截断的调用。 -/
theorem non_continuing_stops_fail (exec : Exec) (history : History) (content : Content) :
    (step exec history ⟨.maxTokens, content⟩).effects = [] ∧
    (step exec history ⟨.other, content⟩).effects = [] := by
  simp [step, Outcome.effects]

/-- string content 宽容读作一个 text block；写回历史的仍是原来的 string 值。 -/
example (history : History) (exec : Exec) :
    step exec history ⟨.endTurn, .string "answer"⟩ =
      .answered (history ++ [assistant (.string "answer")]) "answer" := by
  simp [step, Content.text]

/-- 直接答复是可达的正常出口。 -/
example : (step (fun _ => some []) [] ⟨.endTurn, .items [.text "a", .thinking "t" "s", .text "b"]⟩)
    = .answered [assistant (.items [.text "a", .thinking "t" "s", .text "b"])] "ab" := by
  simp [step, Content.text]

/-- 两个调用严格顺序执行是可达的；thinking 与 signature 原样留在 assistant 历史中。 -/
example :
    let content := Content.items [
      .thinking "reason" "signature",
      .python "python" "a" "x = 40",
      .python "python" "b" "x += 2; print(x)"
    ]
    let calls := content.calls
    step (fun _ => some ["40", "42"]) [] ⟨.toolUse, content⟩ =
      .again [assistant content, userResults calls ["40", "42"]] ["40", "42"] := by
  native_decide

/-- 第二个调用不合法时，即使第一个合法，整波也没有 Python 副作用。 -/
example (exec : Exec) :
    (step exec [] ⟨.toolUse, .items [
      .python "python" "a" "write_file()",
      .client "shell" "b" "rm_file()"
    ]⟩).effects = [] := by
  simp [step, Content.calls, Block.call, validWave, validCall, distinctIds, Outcome.effects]

/-- 重复 ID 也在副作用之前拒绝。 -/
example (exec : Exec) :
    (step exec [] ⟨.toolUse, .items [
      .python "python" "same" "first()",
      .python "python" "same" "second()"
    ]⟩).effects = [] := by
  simp [step, Content.calls, Block.call, validWave, validCall, distinctIds, Outcome.effects]

/-- 解释器死亡不伪造完整的 tool-result 消息。 -/
example :
    (step (fun _ => none) [] ⟨.toolUse, .items [.python "python" "a" "exit(7)"]⟩)
    = .failed [assistant (.items [.python "python" "a" "exit(7)"])] := by
  simp [step, Content.calls, Block.call, validWave, validCall, distinctIds]

end Mei.Agent
