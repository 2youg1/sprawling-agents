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

/-! ## argv 保全（D55）

D55：`crates/desktop/ffi/src/confinement/packet.rs` 的 `quoted` 是唯一生产编码器；
这里规定保全性质，runtime 与 Zig 不再编码 argv。微软 CRT 对普通参数的
反斜杠／引号规则规定 `CrtTail`，程序名的独立规则规定 `CrtProgram`；
Microsoft [Parsing C command-line arguments](https://learn.microsoft.com/en-us/cpp/c-language/parsing-c-command-line-arguments)
是两者的环境前提，操作系统与 CRT 实现该规则不是本模型证明的事。

支持域为非零 UTF16 units，包含空参数与孤立 surrogate；程序名另外要求是
非空文件路径，不含引号且末尾不是反斜杠，允许路径中有空格。`quoteChecked`
只规定 NUL 内容拒绝，生产 `packet::encode` 还可因 checked arithmetic、
command line 或 environment 长度、profile 与限额验证而答 `Action::Encode`。
保全性质以编码成功为条件，不保证任意大小输入都能启动。

`encoded_tail_preserves_units` 与 `encoded_arguments_preserve_order` 量化所有
admitted 输入；它们证明 parser relation 接受编码结果，未证明 parser 的
确定性。Rust 侧的独立 decoder 派生检查与 disposable child 的真实 `args_os`
对拍检验模型和生产的对应，不能把 Lean 定理本身当成 CRT 执行证据。

`cmd /C`、`/K` 的剩余文本还经过 cmd 的脚本解释，CRT argv 保全不保证脚本
含空格、引号、元字符或开关时的执行结果；`/D /S /C` 的 echo 与 exit 探针
单独记录结果，不以它们代签一般脚本域。Microsoft
[cmd](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd)
规定这些开关的脚本行为。击败的备选是在 runtime 或 Zig 再写编码器，
因为调用链只需传递 packet，复制规则会产生第二权威；重开条件是调用方
需要另一种明确的解析语法，此时该语法应有自己的契约。
-/
namespace Argv

/-- quoted 的输入是 UTF16 code units；surrogate 不做归一化，NUL 不属于 admitted。 -/
def admitted (units : List Nat) : Prop :=
  ∀ unit ∈ units, 0 < unit ∧ unit < 65536

/-- 结束引号后必须是参数边界；相邻引号还在当前参数内，不能无条件结束。 -/
def argumentBoundary (suffix : List Nat) : Prop :=
  suffix = [] ∨ suffix.head? = some 32 ∨ suffix.head? = some 9

/-- 待决反斜杠只在后继已知时输出，对应 packet::quoted 的 slashes。 -/
def encodeTail (slashes : Nat) : List Nat → List Nat → List Nat
  | [], suffix => List.replicate (2 * slashes) 92 ++ 34 :: suffix
  | unit :: rest, suffix =>
    if unit = 92 then encodeTail (slashes + 1) rest suffix
    else if unit = 34 then
      List.replicate (2 * slashes + 1) 92 ++ 34 :: encodeTail 0 rest suffix
    else List.replicate slashes 92 ++ unit :: encodeTail 0 rest suffix

/-- CRT 已打开引号内的规则；suffix 在结束引号之后，因而覆盖边界。
规则按 Microsoft 文档独立规定，不以 encodeTail 的结果定义解析成功。 -/
inductive CrtTail : List Nat → List Nat → List Nat → Prop where
  | close (slashes : Nat) (suffix : List Nat) (boundary : argumentBoundary suffix) :
      CrtTail (List.replicate (2 * slashes) 92 ++ 34 :: suffix)
        (List.replicate slashes 92) suffix
  | escaped (slashes : Nat) (input output suffix : List Nat)
      (next : CrtTail input output suffix) :
      CrtTail (List.replicate (2 * slashes + 1) 92 ++ 34 :: input)
        (List.replicate slashes 92 ++ 34 :: output) suffix
  | literal (slashes unit : Nat) (input output suffix : List Nat)
      (notSlash : unit ≠ 92) (notQuote : unit ≠ 34) (notNul : unit ≠ 0)
      (next : CrtTail input output suffix) :
      CrtTail (List.replicate slashes 92 ++ unit :: input)
        (List.replicate slashes 92 ++ unit :: output) suffix

/-- 循环不变量量化所有 admitted 参数、所有待决反斜杠数量与以参数边界开头的 suffix。 -/
theorem encoded_tail_preserves_units (units : List Nat) (slashes : Nat)
    (suffix : List Nat) (valid : admitted units) (boundary : argumentBoundary suffix) :
    CrtTail (encodeTail slashes units suffix)
      (List.replicate slashes 92 ++ units) suffix := by
  induction units generalizing slashes with
  | nil => simpa [encodeTail] using CrtTail.close slashes suffix boundary
  | cons unit rest ih =>
    have restValid : admitted rest := by
      intro value member
      exact valid value (List.mem_cons_of_mem unit member)
    have notNul : unit ≠ 0 := by
      have positive := (valid unit (by simp)).1
      omega
    by_cases slash : unit = 92
    · subst unit
      simpa [encodeTail, List.replicate_succ', List.append_assoc] using
        ih (slashes + 1) restValid
    · by_cases quote : unit = 34
      · subst unit
        simpa [encodeTail] using
          CrtTail.escaped slashes (encodeTail 0 rest suffix) rest suffix
            (by simpa using ih 0 restValid)
      · simpa [encodeTail, slash, quote] using
          CrtTail.literal slashes unit (encodeTail 0 rest suffix) rest suffix
            slash quote notNul (by simpa using ih 0 restValid)

/-- argv 尾部只有加引号的参数与单空格边界。 -/
def encodeArguments : List (List Nat) → List Nat
  | [] => []
  | units :: rest => 34 :: encodeTail 0 units
      (if rest.isEmpty then [] else 32 :: encodeArguments rest)

inductive CrtArguments : List Nat → List (List Nat) → Prop where
  | empty : CrtArguments [] []
  | next (input units suffix : List Nat) (rest : List (List Nat))
      (argument : CrtTail input units
        (if rest.isEmpty then [] else 32 :: suffix))
      (remaining : CrtArguments suffix rest) :
      CrtArguments (34 :: input) (units :: rest)

/-- 任意长度 argv[1..] 保留所有 units 与次序，包含空参数；不规定 cmd 脚本解释。 -/
theorem encoded_arguments_preserve_order (args : List (List Nat))
    (valid : ∀ units ∈ args, admitted units) :
    CrtArguments (encodeArguments args) args := by
  induction args with
  | nil => exact CrtArguments.empty
  | cons units rest ih =>
    apply CrtArguments.next
    · simpa using encoded_tail_preserves_units units 0
        (if rest.isEmpty then [] else 32 :: encodeArguments rest)
        (valid units (by simp)) (by
          cases rest.isEmpty <;> simp [argumentBoundary])
    · exact ih (by
        intro units member
        exact valid units (List.mem_cons_of_mem _ member))

/-- 内容检查拒绝 NUL；Nat 模型不包含生产 checked arithmetic 或 packet 长度拒绝。 -/
def quoteChecked (units : List Nat) : Option (List Nat) :=
  if 0 ∈ units then none else some (34 :: encodeTail 0 units [])

theorem nul_is_exactly_the_content_refusal (units : List Nat) :
    quoteChecked units = none ↔ 0 ∈ units := by
  simp [quoteChecked]

/-- 无引号的文件路径使用 CRT 独立的程序名规则；末尾反斜杠是目录，不在该支持域。 -/
theorem program_body_preserves_path (pathFront : List Nat) (last slashes : Nat)
    (suffix : List Nat) (noQuotes : 34 ∉ pathFront)
    (lastNotQuote : last ≠ 34) (lastNotSlash : last ≠ 92) :
    encodeTail slashes (pathFront ++ [last]) suffix =
      List.replicate slashes 92 ++ pathFront ++ last :: 34 :: suffix := by
  induction pathFront generalizing slashes with
  | nil => simp [encodeTail, lastNotQuote, lastNotSlash]
  | cons unit rest ih =>
    have unitNotQuote : unit ≠ 34 := by
      intro equal
      exact noQuotes (by simp [equal])
    have restNoQuotes : 34 ∉ rest := by
      intro member
      exact noQuotes (List.mem_cons_of_mem unit member)
    by_cases slash : unit = 92
    · subst unit
      simpa [encodeTail, List.replicate_succ', List.append_assoc] using
        ih (slashes + 1) restNoQuotes
    · simp [encodeTail, slash, unitNotQuote, ih 0 restNoQuotes, List.append_assoc]

inductive CrtProgram : List Nat → List Nat → List Nat → Prop where
  | quoted (program suffix : List Nat) (noQuotes : 34 ∉ program)
      (boundary : argumentBoundary suffix) :
      CrtProgram (34 :: (program ++ 34 :: suffix)) program suffix

theorem encoded_program_preserves_path (pathFront : List Nat) (last : Nat)
    (suffix : List Nat) (noQuotes : 34 ∉ pathFront)
    (lastNotQuote : last ≠ 34) (lastNotSlash : last ≠ 92)
    (boundary : argumentBoundary suffix) :
    CrtProgram (34 :: encodeTail 0 (pathFront ++ [last]) suffix)
      (pathFront ++ [last]) suffix := by
  rw [program_body_preserves_path pathFront last 0 suffix noQuotes lastNotQuote lastNotSlash]
  simpa [List.append_assoc] using
    CrtProgram.quoted (pathFront ++ [last]) suffix (by simp [noQuotes, lastNotQuote]) boundary

end Argv

end Runtime.NativeWindows
