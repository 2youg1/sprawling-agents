-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import Mei.spec.Agent

/-!
# Mei 的显式 JSONL 会话模型

User 要求在现有 `sprawling mei` CLI 中只增加 `/new` 与 `/resume <jsonl>`。
每次 `/new` 建立一个新日志文件，旧文件不动；恢复路径由 User 明示。
本模型拥有消息历史、会话代次与完整日志恢复，不拥有 Python 进程现场、文件副作用或模型请求。
assistant `stop` 是日志元数据，恢复时剥去。文件命名与追加 I/O 由 Rust 适配器验证。
-/

namespace Mei.Session

open Mei.Agent

structure Entry where
  message : Message
  stop : Option Stop
  deriving DecidableEq, Repr

abbrev Log := List Entry

/-- 完整日志以 user prompt（字符串或内容块）开始，以已观察为 `end_turn` 的 assistant 结束。 -/
def completed (entries : Log) : Bool :=
  match entries.head? with
  | some first =>
    first.message.role == Role.user &&
      (match entries.getLast? with
       | some last => last.message.role == Role.assistant && last.stop == some Stop.endTurn
       | none => false)
  | none => false

/-- 恢复保留消息内容与顺序，剥去仅属于日志的 stop 元数据；不完整日志没有恢复结果。 -/
def restore (entries : Log) : Option History :=
  if completed entries then some (entries.map Entry.message) else none

theorem restore_requires_completion (entries : Log) (history : History)
    (restored : restore entries = some history) : completed entries = true := by
  by_cases complete : completed entries = true
  · exact complete
  · simp [restore, complete] at restored

theorem restore_preserves_messages (entries : Log) (complete : completed entries = true) :
    restore entries = some (entries.map Entry.message) := by
  simp [restore, complete]

/-- 顶层 AGENTS 只在空历史的新会话进入首个 user 消息；无文件时保持字符串 prompt。 -/
def openingMessage (guide : Option String) (prompt : String) : Message :=
  match guide with
  | some body => ⟨.user, .items [
      .text ("<workspace-instructions source=\"AGENTS.md\">\n" ++ body ++ "\n</workspace-instructions>"),
      .text prompt
    ]⟩
  | none => ⟨.user, .string prompt⟩

def begin (guide : Option String) (prompt : String) : History :=
  [openingMessage guide prompt]

/-- 同一会话的新任务只追加 prompt；没有读取或补注 AGENTS 的输入。 -/
def appendPrompt (history : History) (prompt : String) : History :=
  history ++ [⟨.user, .string prompt⟩]

theorem begin_is_one_message (guide : Option String) (prompt : String) :
    (begin guide prompt).length = 1 := rfl

theorem appendPrompt_preserves_history (history : History) (prompt : String) :
    (appendPrompt history prompt).take history.length = history := by
  simp [appendPrompt]

/-- 当前消息与供下次 `/new` 使用的单调文件代次；恢复外部文件不消耗新代次。 -/
structure State where
  current : History
  nextGeneration : Nat
  deriving DecidableEq, Repr

/-- `/new` 清空历史并消耗一个新文件代次；旧代次及其日志不变。 -/
def new (state : State) : State :=
  { current := [], nextGeneration := state.nextGeneration + 1 }

/-- `/resume <jsonl>` 只在日志完整时导入其历史，不消耗下次 `/new` 的文件代次。 -/
def resume (state : State) (entries : Log) : Option State :=
  (restore entries).map fun history => { state with current := history }

theorem new_clears_and_advances (state : State) :
    (new state).current = [] ∧ (new state).nextGeneration = state.nextGeneration + 1 := by
  exact ⟨rfl, rfl⟩

theorem resume_requires_completion (state : State) (entries : Log) (next : State)
    (resumed : resume state entries = some next) : completed entries = true := by
  unfold resume at resumed
  cases restored : restore entries with
  | none => simp [restored] at resumed
  | some history =>
    exact restore_requires_completion entries history restored

theorem resume_preserves_messages (state : State) (entries : Log)
    (complete : completed entries = true) :
    resume state entries = some { state with current := entries.map Entry.message } := by
  simp [resume, restore_preserves_messages entries complete]

theorem resume_then_appendPrompt_uses_only_logged_opening (state : State) (entries : Log)
    (prompt : String) (complete : completed entries = true) :
    (resume state entries).map (fun next => appendPrompt next.current prompt) =
      some (entries.map Entry.message ++ [⟨.user, .string prompt⟩]) := by
  simp [resume, restore_preserves_messages entries complete, appendPrompt]

example : restore [] = none := by native_decide

example : restore [⟨Mei.Agent.assistant (.string "done"), some Stop.endTurn⟩] = none := by
  native_decide

example :
    let entries := [
      ⟨(⟨Role.user, Content.items []⟩ : Message), none⟩,
      ⟨Mei.Agent.assistant (.string "done"), some Stop.endTurn⟩
    ]
    restore entries = some (entries.map Entry.message) := by
  native_decide

example :
    let entries := [
      ⟨(⟨Role.user, Content.string "start"⟩ : Message), none⟩,
      ⟨Mei.Agent.assistant (.string "done"), some Stop.endTurn⟩
    ]
    restore entries = some [⟨Role.user, Content.string "start"⟩, Mei.Agent.assistant (.string "done")] := by
  native_decide

end Mei.Session
