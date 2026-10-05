-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Windows native execution
规定 `crates/runtime/src/tools/exec/native_windows.rs` 与
`crates/desktop/ffi/src/confinement.rs`、`crates/desktop/ffi/zig/confinement.zig`。

D40：Windows native 使用无 capability 的 AppContainer 与匿名 Job Object；
CreateProcessW 以 CREATE_SUSPENDED 创建，设置 Job 的 aggregate committed-memory
上限、CPU hard cap 与 kill-on-close，AssignProcessToJobObject 成功之后才 ResumeThread。
没有准入安全 API 的平台调用进入既有 Zig leaf；win32job 与 winsafe 的现有安全
CreateProcess 面没有 SECURITY_CAPABILITIES / STARTUPINFOEX，不能承担此契约。

D41：native 的 CPU hard cap 默认 50% 整机份额，未设置 run memory 时默认
256 MiB aggregate committed-memory；配置的 Shares::CpuAndMemory 覆盖这个默认值。
这两个默认数只在 native_windows.rs 定义。run job 在 table lock 内先创建并设同一
memory 上限，native command 同时加入 command job 与 run job，再恢复；多个命令的
aggregate 内存不能通过每条命令分别领限额来扩大。拒绝设置限额时停止启动。
默认值的理由是没有配置的 native 仍需有界；它们是保守初值，不宣称性能最优。

输入由 Rust 决定：已复制工作目录、明确程序和 argv、允许的环境、非零内存与
CPU 上限、唯一 profile 名及 output files；叶子只执行平台操作。AppContainer SID
只获这次副本的继承读写 ACL，不获源树 ACL，无网络 capability 与 loopback exemption。
scratch 的继承 mandatory-integrity label 为 Low，避免 medium 默认标签即使
DACL 已授权仍因 write-up 禁止而拒绝 AppContainer 写入；只修改这次复制目录，
使用 LABEL_SECURITY_INFORMATION，不索取 SeSecurityPrivilege。
profile 的私有存储也是这次执行拥有的资源。harness/provider 不进入 AppContainer。

失败拒绝，不再起动普通子进程；cleanup 拒绝时保留错误及未释放资源的 owner，
模型的 closing 只表示不能恢复执行，closed 才表示 cleanup 已成功。创建后任何失败均终止挂起进程、关闭句柄和
删除 profile；正常结束、取消与 owner 释放均先终止 Job 中剩余进程，再删除 profile。
读取退出状态或清理失败保留操作与 OS code，并由 runtime 转成 E_SANDBOX_DENIED。

下面模型量化所有平台回答组成的 trace；Win32 按文档实现悬挂、job 继承、ACL
与 capability 检查属于环境假设，定理不证明操作系统。Rust-reference/leaf packet
等价与两侧 fuzz 验证边界；Windows disposable acceptance 验证实际五轴与清理。
权威文档：Microsoft CreateAppContainerProfile、SECURITY_CAPABILITIES、CreateProcessW、
UpdateProcThreadAttribute、AssignProcessToJobObject、Job Objects、SetNamedSecurityInfoW。
-/
namespace Runtime.NativeWindows

inductive Phase where
  | empty | profile | confined | suspended | assigned | running | closing | closed
  deriving DecidableEq, Repr

inductive Answer where
  | profile | limitsAndAcl | createSuspended | assign | resume | close | failure
  deriving DecidableEq, Repr

def step : Phase → Answer → Phase
  | .closed, _ => .closed
  | .closing, .close => .closed
  | .closing, _ => .closing
  | .empty, .profile => .profile
  | .profile, .limitsAndAcl => .confined
  | .confined, .createSuspended => .suspended
  | .suspended, .assign => .assigned
  | .assigned, .resume => .running
  | _, .close => .closed
  | _, .failure => .closing
  | state, _ => state

def run (state : Phase) : List Answer → Phase
  | [] => state
  | answer :: rest => run (step state answer) rest

/-- 唯一能开始执行的 transition 来自已装进 job 的挂起进程。 -/
theorem resume_requires_assignment (state : Phase) (answer : Answer)
    (before : state ≠ .running) (after : step state answer = .running) :
    state = .assigned ∧ answer = .resume := by
  cases state <;> cases answer <;> simp_all [step]

/-- 正常路可达；拒绝所有操作不构成兑现保障。 -/
theorem launch_is_reachable :
    run .empty [.profile, .limitsAndAcl, .createSuspended, .assign, .resume] = .running := rfl

/-- 已清理的 owner 不能被任意后续回答重新执行。 -/
theorem closed_is_terminal (answers : List Answer) : run .closed answers = .closed := by
  induction answers with
  | nil => rfl
  | cons answer rest ih =>
    cases answer <;> simp [run, step, ih]

/-- 未成功清理的资源仍有 owner，不能通过后续回答再次恢复执行。 -/
theorem closing_cannot_resume (answers : List Answer) :
    run .closing answers = .closing ∨ run .closing answers = .closed := by
  induction answers with
  | nil => simp [run]
  | cons answer rest ih =>
    cases answer <;> simp [run, step, closed_is_terminal, ih]

/-- 失败停止起动；OS 拒绝清理时保持 closing，而不是宣称资源已经释放。 -/
theorem failure_cannot_fall_back (state : Phase) (rest : List Answer) :
    run state (.failure :: rest) ≠ .running := by
  have stopped := closing_cannot_resume rest
  cases state <;> simp_all [run, step, closed_is_terminal]

/-- 每个 string span 必须在借来的 packet 内，尾部 NUL 不进入其内容。 -/
def terminated (units : List Nat) : Bool := !units.isEmpty && units.getLast? == some 0

def packetValid (units : List Nat) : Bool :=
  terminated units && terminated (units.dropLast)

theorem accepted_packet_has_two_terminators (units : List Nat)
    (accepted : packetValid units = true) :
    terminated units = true ∧ terminated units.dropLast = true := by
  simpa [packetValid, Bool.and_eq_true] using accepted

end Runtime.NativeWindows
