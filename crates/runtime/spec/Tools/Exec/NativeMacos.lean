-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::exec::native_macos

## 1 需求分解
规定 `crates/runtime/src/tools/exec/native_macos.rs`：Seatbelt 只允许副本路径写入，
禁止网络，显式选择失败时拒绝。聚合内存、树级终止与独立身份不由 Seatbelt 提供。

## 2 验收标准
经 Confined::place 与 ExecTool::invoke 的实际 macOS 对拍检查副本写入成功、绝对路径
与逃逸链接写入失败、TCP／UDP／后代网络失败、初始化失败不执行目标。
本模型证明每一步子进程操作都携带策略；不证明 XNU 执行策略。

## 3 假设与歧义
内核是策略执行者。探测成功不保证之后的包装起动成功，真实目标仍必须携带同一策略。
包装初始化失败与目标退出码的区别尚无起动握手；非零结果保留原 stderr，不宣称已区分。
macOS host 的同 RunId 多命令聚合硬内存机制尚未成立；RLIMIT、taskpolicy 与采样不满足。

## 4 现状分析
副本同步与后台持有归 confinement::placing；叶子不建立第二张 run／副本表。
原有 macOS 缺省 copied_tree 由 wire D26 规定；显式 native 接本叶子，不重定缺省。

## 5 权威信源
Apple App Sandbox： https://developer.apple.com/documentation/security/app-sandbox 。
macOS 26.6.2（25G83）的 sandbox-exec(1) 自带手册写明 “execute within a sandbox (DEPRECATED)”
与 “Set the profile parameter key to value.”，CI 的 macOS mechanism artifact 保存原文；
Chromium 的 sandbox/policy/mac/renderer.sb 使用 (allow process-info-setcontrol (target self))，
这是对拍前采用的 SBPL 操作依据，不代替目标 macOS 的执行证据。
弃用状态不等于已移除，但不保证未来系统继续提供此程序，缺席或拒绝必须拒开。
App Sandbox entitlement 文档不能证明命令行 profile 的行为；sandbox_init(3) 与真实拒绝
实验仍须分别核实，支持范围只随已运行的系统证据扩大。

## 6 命名统一
wrapper 是 sandbox-exec 程序；copy 是唯一可写工作树；profile 是固定 SBPL 策略。

## 7 模块边界
adapter：标准库 Command 的直接 argv，不调用 unsafe，不增加 shell，不管理 run 生命周期。

## 8 接口先行
ExecTool::confined(self, confinement: Confined) -> Self 在构造后、首次调用前替换执行边界，
并从同一 statement 重新生成 disclosure。
wrap(wrapper: &Path, copy: &Path, command: &Command) -> Result<Command, AxError>；
probe(wrapper: &Path, copy: &Path) -> Result<(), AxError> 通过同构造器执行 /usr/bin/true。

## 9 工作流程
先规范化副本、验证目录，再构造固定 profile 及独立参数，探测通过才交给 backlog。
每次放置重新探测，子命令的 fork／exec 继承内核策略而不是继承可改写的 profile 文件。

## 10 实现逻辑
D40 Seatbelt 的写路径经 -D 参数递交，profile 是内联常量，以 deny default 开始。
允许读取、执行、fork、sysctl-read 和必要 Mach lookup；process-info-setcontrol 仅允许
(target self)，使 nice 能在自身降优先级，真实 runner 的 setpriority 拒绝回归规定这项权限。
网络默认拒且显式 deny network*。
仅 subpath WORKDIR 可写，/dev/null 只放行数据写入，不放行设备创建或任意临时目录。
被否：把路径插进 SBPL 源码、放开整个 TMPDIR、探测失败执行裸命令。
路径参数保留引号／反斜杠而不产生新的 profile 表达式；非 UTF-8 路径拒绝。
同一 Command 的 env_clear、显式设置／移除变量保持；stdio 由 backlog 在包装之后设置。

## 11 边界枚举
缺席 wrapper、坏 profile／权限拒绝、副本不存在／非目录／根目录、非 UTF-8 路径、
空 program 均拒。cwd 改为规范副本，目标 program 前有 -- 终结 wrapper 选项，args 保持独立参数，包括以连字符开头的参数。

## 12 错误处理
E_SANDBOX_DENIED 保留失败动作、路径或原始 stderr 与恢复：修复 native 或明确选择其他臂。
叶子没有裸目标 fallback，没有将初始化失败称为网络拒绝成功。

## 13 依赖选型
仅标准库与既有 kernel::AxError；不新增平台 FFI 或依赖。

## 14 硬编码声明
固定 profile 仅在 Rust 的 PROFILE 定义；路径与 argv 是参数。wrapper 的系统位置由机器选择层提供。

## 15 影响面
Confinement::MacosSeatbelt 的 assurances 为文件／网络保，其余不保，disclosure／doctor 读取
同一权威。显式配置消费者经 ExecTool 的构造面接入；父分部与 wire D26 保持选择规则。

## 16 测试与约束
Rust argv 回归覆盖路径注入、环境移除与空 program；macOS 实际 ExecTool 对拍不跳过拒绝。
Lean 量化任意 fork／exec／退出序列，证明模型里的策略身份不随操作丢失；
这是假定内核继承正确后的平台义务，不是 Rust 实现的 refinement 证明。Rust 参数 proptest
验证任意目标 argv 都使用同一固定策略与独立路径参数，并不作为 fork 继承的 derived 证据。
真实 macOS ExecTool 后代网络测试检查已运行的继承轨迹，不能推广为任意系统轨迹证明。
完整平台验收需运行目标系统，不以 Windows 上的字符串断言替代。

## 17 文档关系
父分部 D32、§8-13-2 定选择与副本；D29 定 Shares 与聚合内存要求；本分部不复制这些定义。
-/

namespace Runtime.Tools.Exec.NativeMacos

inductive Operation where
  | forkChild
  | execProgram
  | finish
  deriving Repr, DecidableEq

structure ConfinedProcess where
  profile : Nat
  running : Bool
  deriving Repr, DecidableEq

/-- fork 与 exec 改变执行状态，不能改变内核授予的策略身份。 -/
def step (process : ConfinedProcess) (operation : Operation) : ConfinedProcess :=
  match operation with
  | .forkChild => process
  | .execProgram => { process with running := true }
  | .finish => { process with running := false }

def trace (process : ConfinedProcess) (operations : List Operation) : ConfinedProcess :=
  operations.foldl step process

/-- 任意长度／顺序的后代操作都保留原策略。 -/
theorem every_descendant_trace_keeps_profile (process : ConfinedProcess)
    (operations : List Operation) : (trace process operations).profile = process.profile := by
  induction operations generalizing process with
  | nil => rfl
  | cons operation rest ih =>
    change (trace (step process operation) rest).profile = process.profile
    rw [ih]
    cases operation <;> rfl

end Runtime.Tools.Exec.NativeMacos
