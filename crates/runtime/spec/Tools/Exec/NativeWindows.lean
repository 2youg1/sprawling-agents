-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Windows native execution
规定 `crates/runtime/src/tools/exec/native_windows.rs` 与
`crates/desktop/ffi/src/confinement.rs`、`crates/desktop/ffi/zig/confinement.zig`。

D53：Windows native 使用无 capability 的 AppContainer 与匿名 Job Object；
CreateProcessW 以 CREATE_SUSPENDED 创建，设置 Job 的 aggregate committed-memory
上限、CPU hard cap 与 kill-on-close，AssignProcessToJobObject 成功之后才 ResumeThread。
没有准入安全 API 的平台调用进入既有 Zig leaf；win32job 与 winsafe 的现有安全
CreateProcess 面没有 SECURITY_CAPABILITIES / STARTUPINFOEX，不能承担此契约。

D54：native 的 CPU hard cap 默认 50% 整机份额，未设置 run memory 时默认
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
叶子保留 GetNamedSecurityInfo 分配的原 security descriptor，跨调用保留的只有
OS 自己分配的资源，Rust 借出的 packet 地址不保留；command 结束后恢复副本根的
原 DACL 与 mandatory label，再释放 descriptor，避免复用副本时积累旧 SID 的 ACE。
原目录已被命令删除时视为权限资源已经不存在，不尝试改动它的父目录。
环境继续从 Command 的显式 allowlist 取值，Windows native loader 的 OS 根目录
变量由安全 GetSystemDirectory API 的父目录提供，并通过 Command 的 Windows
case-insensitive key 规则替换别名；它们是平台启动信息，不从 host environment 继承，
也不携带用户或 provider 凭据。USERPROFILE、APPDATA、LOCALAPPDATA、TEMP 与 TMP
由 disposable working directory 提供，不继承 host 路径；Microsoft 的
Implementing an AppContainer「Creating the Profile」规定启动时会重定向
LOCALAPPDATA/TEMP/TMP 到 profile。显式环境缺失这些初始化字段时，平台可能
以 ERROR_ENVVAR_NOT_FOUND 拒绝；实际 disposable test 验证该启动条件。
错误携带真实失败阶段，避免把 loader 拒绝记作 guard 通过。
profile 的私有存储也是这次执行拥有的资源。harness/provider 不进入 AppContainer。

失败拒绝，不再起动普通子进程；cleanup 拒绝时保留错误及未释放资源的 owner，
模型的 closing 只表示不能恢复执行，closed 才表示 cleanup 已成功。
launch failure 的未释放资源随 typed Failure 返回，由 run Job 保留，后续 native
准入先重试清理；release 时仍由这个 owner 执行最后的 teardown，并报告拒绝。创建后任何失败均终止挂起进程、关闭句柄和
删除 profile；正常结束、取消与 owner 释放均先终止 Job 中剩余进程，再删除 profile。
终止、root wait 或 Job tree wait 拒绝时，process/job 句柄、security descriptor 与 profile
保持在同一 owner，后续重试重新验证；只有肯定退出后才关闭句柄、恢复权限与删除 profile。
读取退出状态或清理失败保留操作与 OS code，并由 runtime 转成 E_SANDBOX_DENIED；
Backlog 的 settle/harvest 遇 native poll/cleanup 拒绝时返回该错误并保留 member，
下一次 harvest 由同一 owner 重试，不能借用 host 的 Unknown ending 删除资源。

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
  have neverRunning : run .closing rest ≠ .running := by
    rcases stopped with retained | removed
    · simp [retained]
    · simp [removed]
  cases state <;> simp_all [run, step, closed_is_terminal]

structure Readiness where
  runAssigned : Bool
  commandAssigned : Bool
  identityVerified : Bool

def mayResume (ready : Readiness) : Bool :=
  ready.runAssigned && ready.commandAssigned && ready.identityVerified

/-- 恢复前的实际叶子守卫要求两次 job membership 与独立 AppContainer 身份皆已读回。 -/
theorem resume_requires_all_guards (ready : Readiness) (accepted : mayResume ready = true) :
    ready.runAssigned = true ∧ ready.commandAssigned = true ∧ ready.identityVerified = true := by
  cases ready with
  | mk run command identity =>
    cases run <;> cases command <;> cases identity <;> simp_all [mayResume]

/-- 每个 string span 必须在借来的 packet 内，尾部 NUL 不进入其内容。 -/
def terminated (units : List Nat) : Bool := !units.isEmpty && units.getLast? == some 0

def packetValid (units : List Nat) : Bool :=
  terminated units && terminated (units.dropLast)

theorem accepted_packet_has_two_terminators (units : List Nat)
    (accepted : packetValid units = true) :
    terminated units = true ∧ terminated units.dropLast = true := by
  simpa [packetValid, Bool.and_eq_true] using accepted

/-- 清理的每一轮必须重新得到终止与整棵 Job 退出的肯定回答；失败不消耗资源所有权。 -/
structure CleanupOwner where
  process : Bool
  job : Bool
  security : Bool
  profile : Bool
  deriving DecidableEq, Repr

inductive CleanupAnswer where
  | refused | timedOut | stopped
  deriving DecidableEq, Repr

def retainUntilStopped (owner : CleanupOwner) : CleanupAnswer → CleanupOwner
  | .refused => owner
  | .timedOut => owner
  | .stopped => ⟨false, false, false, false⟩

/-- 任意次拒绝或超时的 trace 都保留同一 owner；重试不能因丢失句柄伪报成功。 -/
theorem cleanup_failure_trace_preserves_owner (owner : CleanupOwner)
    (answers : List CleanupAnswer)
    (failed : ∀ answer ∈ answers, answer ≠ .stopped) :
    answers.foldl retainUntilStopped owner = owner := by
  induction answers generalizing owner with
  | nil => rfl
  | cons answer rest ih =>
    have notStopped := failed answer (by simp)
    have remaining : ∀ answer ∈ rest, answer ≠ .stopped := by
      intro answer member
      exact failed answer (by simp [member])
    cases answer <;> simp_all [retainUntilStopped]

end Runtime.NativeWindows
