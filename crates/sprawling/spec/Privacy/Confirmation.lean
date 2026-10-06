-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.sprawling.spec.Privacy

/-!
# privacy 的确认：页面逐项确认，命令绑定确认时的当前快照
规定确认在生产中由谁守住：expected 由 `bin::privacy::target` 换回快照（换不回即拒绝，wire D49），
由 `bin::privacy::plan` 与 fresh read 比较（不同即 `Changed`），由 `bin::privacy::service` 按 idem
只执行一次（Privacy.Service D69）；没有单独的确认模块，因为绑定只是这一次比较，不需要服务端状态。
确认行为复用 crates.sprawling.spec.Privacy 的 Request、planned 与 Step，本分部不定义第二份确认状态；
它证明三件事：
写入的原值就是人确认时看到的值，unknown 之后不产生新的写入，核对不写系统。
时间由 assembly 的 SystemClock 取得；期限在协调者接受命令时确定（Privacy §14）。
-/

/-! D57 确认是页面上的逐项明确操作，命令携带确认时的当前快照；机器作用域另经 UAC
人在页面上对一个控制点「应用」或「恢复」，确认框显示当前值、写入值、作用域、是否需要管理员，
以及该控制容易忽略之处与恢复说明（会删除不可找回的数据或需要重启的控制在点下之前说明这一点）；
确认后发送的命令带 expected（页面显示的当前快照）、control 与 action。协调者 fresh read 与
expected 不同即拒绝（ApplyPlan.changed／RestorePlan.changed），所以写入的原值必定是人看到的值，
不需要服务端保存挂起的确认。一次性由 operation 的新鲜性与 IdemKey 的结果保留共同保证：
同一 idem 的重放取回已有结果，不再执行。
机器作用域（HKLM 与计划任务）经 Windows UAC 提升子进程写入（Privacy.Windows D59）；UAC 在安全
桌面上要求本人操作，Agent 驱动页面不能替人通过。用户作用域（HKCU 与用户环境变量）没有 UAC：
同一用户的 Agent 本来就能直接写 HKCU，这一剩余威胁如实保留。
命令只从本地页面与本地 CLI 进入：wire 上的操作命令归 LocalOnly，工具目录、MCP、ACP 没有入口。
被否：控制台验证码（请求时在 console 显示一次性码，页面回填后才执行）——在用户作用域它不增加
实际边界（同用户进程能直接写 HKCU），在机器作用域 UAC 已是更强的本人操作；而安装后常见的
无控制台服务进程永远无法显示验证码，人就永远不能应用任何一项。
重开参数：若出现 UAC 之外能被 Agent 代为完成的机器作用域写入路径，或用户作用域需要本人操作的边界，
改为两步交换（请求 → console 显示码 → 带码确认），其余协调次序不变。
-/

namespace Sprawling.Privacy.Confirmation
open Sprawling.Privacy

/-- 写入的原值就是确认时页面显示的快照，写入的控制就是命令的控制。 -/
theorem prepared_matches_confirmation {s : State} {r : Request} {keyExisted : Bool} {i : Intent}
    (plan : planned s r keyExisted = some i) :
    i.original = r.expected ∧ i.control = r.control := by
  obtain ⟨control, original, _, _⟩ := planned_facts plan
  exact ⟨original, control⟩

/-- 任意未知状态的一步协调或环境操作均不能增加 OS 写入历史。 -/
theorem unknown_step_preserves_write_history {s t : State}
    (move : Step s t) (blocked : s.phase = .unknown) : t.writes = s.writes := by
  cases move <;> simp_all

/-- 未知结果后的任意协调轨迹都不能产生新的写入；离开 unknown 只经 Reconcile。 -/
theorem unknown_trace_preserves_write_history {s t : State}
    (trace : Trace Step s t) (blocked : s.phase = .unknown) : t.writes = s.writes := by
  induction trace with
  | rest => rfl
  | next move _ ih =>
      exact (ih (step_unknown move blocked)).trans
        (unknown_step_preserves_write_history move blocked)

/-- 人的核对不写系统：它只给未结操作一个 Finished。 -/
theorem reconcile_writes_nothing {s t : State} (move : Reconcile s t) : t.writes = s.writes := by
  cases move
  rfl

end Sprawling.Privacy.Confirmation
