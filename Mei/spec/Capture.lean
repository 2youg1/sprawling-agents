-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Mei 的输出捕获所有权模型

`inline` 的完整内容进入消息且路径不暴露，文件归当前 run；`spilled` 的路径进入消息，
文件在交互中归 session，在无头运行中归调用者。模型不覆盖宿主崩溃后的物理删除。
-/

namespace Mei.Capture

inductive Kind where
  | inline | spilled
  deriving DecidableEq, Repr

inductive Surface where
  | interactive | headless
  deriving DecidableEq, Repr

inductive Owner where
  | run | session | caller
  deriving DecidableEq, Repr

def owner : Kind → Surface → Owner
  | .inline, _ => .run
  | .spilled, .interactive => .session
  | .spilled, .headless => .caller

/-- Python 退出只回收 run 所有的文件；session/caller 所有的完整输出仍可经旧路径读取。 -/
def fileSurvivesInterpreterExit (kind : Kind) (surface : Surface) : Bool :=
  owner kind surface != .run

/-- 未提交 run 的两类捕获都没有会话引用，均可由 run owner 清理。 -/
def reclaimUncommitted : Kind → Bool
  | .inline | .spilled => true

theorem inline_file_ends_with_run (surface : Surface) :
    fileSurvivesInterpreterExit .inline surface = false := by
  cases surface <;> native_decide

theorem spilled_file_survives_resume (kind : Kind) (surface : Surface)
    (spilled : kind = .spilled) :
    fileSurvivesInterpreterExit kind surface = true := by
  subst kind
  cases surface <;> native_decide

theorem uncommitted_captures_are_reclaimable (kind : Kind) :
    reclaimUncommitted kind = true := by
  cases kind <;> rfl

example : owner .spilled .interactive = .session := by native_decide
example : owner .spilled .headless = .caller := by native_decide

end Mei.Capture
