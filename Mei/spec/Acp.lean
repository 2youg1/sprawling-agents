-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 作为 ACP agent 的会话相位

规定 `Mei/src/acp.rs`：无头核在 `--acp` 下以 ACP version 1 的 **agent** 角色
在 stdio 上回答 `initialize` / `session/new` / `session/prompt`，并接受
`session/cancel`。城侧 `agent_protocols::harness::session` 是对称的 **client**；
HTTP `POST /acp` 不是本模型。本面是文本会话子集：一条连接一次 `session/new`，
不连接 `mcpServers`。

本模型守四条：

1. 方法只在对应相位合法（`method_ok`）。
2. `session/prompt` 取 `text` 与 `resource_link`，并至少有一段非空内容（`promptText`）。
3. 一轮 prompt 的结束原因只有 `end_turn` 与 `cancelled`（`Stop`）。
4. 取消在回合开始前直接结束；回合进行中见到取消后，结束原因仍是 `cancelled`
   （`finish`），即使模型已经产出过文本或同回合返回了错误。
-/

namespace Mei.Acp

inductive Phase where
  | idle
  | introduced
  | session
  deriving DecidableEq, Repr

inductive Method where
  | initialize
  | sessionNew
  | sessionPrompt
  | sessionCancel
  | other
  deriving DecidableEq, Repr

inductive Stop where
  | endTurn
  | cancelled
  deriving DecidableEq, Repr

def method_ok : Phase → Method → Bool
  | .idle, .initialize => true
  | .introduced, .sessionNew => true
  | .session, .sessionPrompt => true
  | .session, .sessionCancel => true
  | _, _ => false

inductive Block where
  | text (body : String)
  | resourceLink (name uri : String)
  | other
  deriving DecidableEq, Repr

def linkLine (name uri : String) : String :=
  "resource_link name=" ++ name ++ " uri=" ++ uri

def promptText : List Block → Option String
  | [] => none
  | .text body :: rest =>
    if body = "" then promptText rest
    else
      match promptText rest with
      | none => some body
      | some more => some (body ++ "\n" ++ more)
  | .resourceLink name uri :: rest =>
    if name = "" && uri = "" then promptText rest
    else
      let line := linkLine name uri
      match promptText rest with
      | none => some line
      | some more => some (line ++ "\n" ++ more)
  | .other :: rest => promptText rest

theorem promptText_rejects_empty :
    promptText [] = none ∧ promptText [.text ""] = none ∧
      promptText [.other] = none ∧
      promptText [.resourceLink "" ""] = none := by
  simp [promptText, linkLine]

theorem promptText_keeps_nonempty_text :
    promptText [.text "a", .other, .text "b"] = some "a\nb" := by
  simp [promptText]

theorem promptText_keeps_resource_link :
    promptText [.resourceLink "notes.txt" "file:///notes.txt"] =
      some "resource_link name=notes.txt uri=file:///notes.txt" := by
  simp [promptText, linkLine]

def advance : Phase → Method → Phase
  | .idle, .initialize => .introduced
  | .introduced, .sessionNew => .session
  | phase, _ => phase

theorem initialize_then_session_new_reaches_session :
    advance (advance .idle .initialize) .sessionNew = .session := rfl

/-- 见过 `session/cancel` 则结束为 `cancelled`，不论回合是否已开始产出文本。 -/
def finish (sawCancel : Bool) (produced : Stop) : Stop :=
  if sawCancel then .cancelled else produced

theorem cancel_overrides_end_turn :
    finish true .endTurn = .cancelled := by
  simp [finish]

theorem quiet_turn_keeps_end_turn :
    finish false .endTurn = .endTurn := by
  simp [finish]

end Mei.Acp
