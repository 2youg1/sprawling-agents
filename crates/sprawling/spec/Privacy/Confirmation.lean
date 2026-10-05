-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.sprawling.spec.Privacy

/-!
# privacy 的一次确认许可
本节复用协调者的状态迁移关系，证明未知结果不能获得新写入许可；确认 adapter 尚未接线。
确认行为与状态迁移由 crates.sprawling.spec.Privacy 的 Request、Step.answer、planValid
统一定义，本分部复用该迁移关系而不定义第二份确认状态；许可绑定 control/definition/owner/current/target/operation/expiry。
错误、取消、期限已过和重放均不得写入；durablePrepared 和 writeStarted 都
必须重新检查期限和资格。时间由 assembly 的 SystemClock 取得，熵由 OS 提供。
console 才可显示 code，页面、模型、MCP/ACP 和远程请求不能直接取得许可。
当前没有 production confirmation adapter；不能把确认文字当安全授权。
-/


namespace Sprawling.Privacy.Confirmation
open Sprawling.Privacy

/-- 任意未知状态的一步确认、取消或读操作均不能增加 OS 写入历史。 -/
theorem unknown_step_preserves_write_history {s t : State}
    (move : Step s t) (blocked : s.phase = .unknown) : t.writes = s.writes := by
  cases move <;> simp_all

/-- 未知结果后的任意操作轨迹都不能取得新的写入许可。
生产 coordinator 的派生轨迹检查必须核对该性质；当前确认 adapter 尚未实现。 -/
theorem unknown_trace_preserves_write_history {s t : State}
    (trace : Trace s t) (blocked : s.phase = .unknown) : t.writes = s.writes := by
  induction trace with
  | rest => rfl
  | next move remaining ih =>
      exact (ih (step_unknown move blocked)).trans
        (unknown_step_preserves_write_history move blocked)

end Sprawling.Privacy.Confirmation
