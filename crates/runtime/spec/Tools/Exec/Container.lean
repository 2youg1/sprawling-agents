-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::exec::container

规定 `crates/runtime/src/tools/exec/container.rs`，kernel 的输入值住
`crates/kernel/src/config/container.rs`，既有放置与副本契约仍由父分部 §8-13-2 规定。

## 1 需求分解

D32 的 container 选择需要三种独立证据：可调用的 Linux daemon、已存在的固定镜像、
真正可强制的 CPU／内存／进程额度。PATH 上存在 CLI 不是任一种隔离保证。

## 2 验收标准

`ContainerRuntime::admit` 对 docker 的 info JSON 与 podman 的 info JSON 分别读其官方字段；
未知字段形状、Windows daemon、未报告 cgroup v2 或缺少控制器都以 E_SANDBOX_DENIED 拒。
`ContainerRuntime::create_command` 只构造直接 argv，不执行 shell，不自行拉镜像。
Rust 的公开入口测试覆盖未知／缺少能力与含 shell 元字符的命令参数；它们不证明真实 daemon 隔离。

## 3 假设与歧义

info 的内容是 daemon 的自述，不是已执行的隔离验收；daemon、OCI runtime 与镜像属于受信环境。
实际起动仍须 inspect 验证限额与挂载，然后用同一容器跑 cgroup 与网络对拍。
镜像、daemon 接口或 OCI runtime 更新时应重跑实测。

五轴的生产声明按实际边界解释；admission 不表示五轴已经实测：

| 轴 | 生产执行与验证 | 保证的范围 |
|---|---|---|
| 文件 | 仅 bind 副本到 /work，inspect 拒额外挂载与可写根 | 写入工作目录不改源树；镜像内容仍可读，不承诺读隔离 |
| 网络 | create 与 inspect 均要求 network=none | 禁止外部与宿主回环连接；容器自己的 loopback 仍可用 |
| 进程树 | Backlog 持有 daemon 名字，终止时 force rm，失败保留 owner | 确认清理后没有该容器的后代；父轮询超时不表示树已终止 |
| 用户 | kernel 拒零 UID，create 与 inspect 核对冻结 UID，drop ALL 与 no-new-privileges | 非 root 身份；未要求 user namespace，也未要求数值 UID 与宿主不同 |
| 资源 | admission 要 cgroup v2 控制器，inspect 核对 CPU、memory、swap 与 pids | daemon／OCI 受信前提下强制每容器额度，不是每 RunId 聚合额度 |

用户轴的非 root 身份不等于独立宿主安全主体：rootful daemon 可让相同数值 UID
出现在宿主和容器中。若要求两者必不同或要求 user namespace，当前臂没有该保证。

Docker 的 CapDrop 必须明确含 ALL；Podman 的 CapDrop 是相对默认集合的差值，
因此起动前改核对其 EffectiveCaps 与 BoundingCaps 两个实际集合，二者均须明确为空。
Podman 的这两个不省略字段来自 OCI spec 的切片，空切片可输出 null 或 []，二者都表示空集合；
缺字段、非数组的其它值或任一非空集合都拒绝，不能把非 root 用户的空 EffectiveCaps
误当成全部 capabilities 已移除。字段来源为 Podman 的
https://github.com/containers/podman/blob/v4.9.3/libpod/define/container_inspect.go 与
https://github.com/containers/podman/blob/v4.9.3/libpod/container_inspect.go 。

Podman 的 stopped-container inspect 兼容范围尚有未决：能力准入通过后仍可能在起动前的
边界核对中拒绝配置，须将被拒的真实 JSON 与冻结限额、挂载、capabilities 及 security options
逐项比较，确认应拒的配置与需要支持的合法字段表示；`.github/workflows/on-demand.yml`
的 container job 保存这份 JSON 并运行同一生产检查，未知表示仍以 E_SANDBOX_DENIED 拒绝，
不从 capability admission 推出这个后端已经通过五轴验收。

D50 Backlog 在分配输出文件之前取得唯一名字与副本的清理 owner，然后在 create 之前登记成员；
计数、表锁或输出文件失败同样不能丢失副本的清理责任，成员的进程值同时拥有 daemon 身份
与可选的 attach 子进程。start／inspect／attach／create 应答丢失均按登记身份清理。
停止时先置 stopping，再请求 rm --force --volumes；删除失败保留成员与副本，下一次
harvest 重试并返回 typed failure，release 的 Drop 调用只记录待清理责任，不能声称删除成功。
只有成功的删除应答或成功 inventory 确认身份缺席才释放副本。CLI 结束后 inspect State.ExitCode
才决定目标程序结果；只有 Status 为 exited／dead 且 Running=false 才是终止结果，
created、缺少 Status 或与 Running 不一致都以 E_SANDBOX_DENIED 拒绝并保留清理责任，
因为未起动目标的默认 ExitCode=0 不代表执行成功。CLI 提前结束而 daemon 仍 Running 为失败并清理。
父进程等待 ready、create 应答与 disarm 的轮询次数有界；普通 daemon 请求与每次 rm／inventory
使用 control 的有界预算，失败后成员不得重新 start。这里的界限是轮询预算，不是 OS 调度、
文件 I/O 或 kill／wait 系统调用的硬实时期限，也不限定独立 guardian 的存活时间。
D52 生产 doctor 给 runtime 绑定当前 harness executable；Backlog 在 create 前起动独立 guardian，
它读取 scratch 中唯一 ownership 文件（cleanup record 与无损 OsString argv）后通过 file-backed stdout 报 ready，再守住只由父进程持有的 stdin。
create 请求从既有 stdin 下达，应答写同一 scratch 中的文件，避免把整条 argv 再塞进进程命令行。
父进程确认 daemon 删除与副本释放后发 done；EOF、父进程异常终止或读失败都转为独立清理，
复用同一个有界 rm／inventory 规则并保留责任重试，直到确认删除。未取得 ready 时不能 create。
guardian 在收到 create 指令之前不执行请求；收到后独自拥有 create CLI，父进程只按有界窗口读应答文件。
EOF 不终止在途 CLI；guardian 等待并回收该 child，随后才清理，这个等待没有截止，
因为杀死 CLI 或一次 inventory 缺席都不能证明 daemon 不会迟到创建。只有 create 成功应答之后
的缺席确认可以释放副本；create 失败或应答未知时，必须取得 rm 成功应答，缺席不能解除责任。
父进程等待超时、读失败或取消时关闭通道并保留 Backlog 成员，直到 guardian 已清理并被回收。
普通库测试可只用进程内 owner；它不声称异常终止保证。假设 guardian 与 daemon 仍存活，
本模型不声称断电或 guardian 自身被外部终止后删除。独立进程而非父线程是所选方案，
因为父进程的 abort 不执行 Drop，也不会留下线程。

## 4 现状分析

SandboxLimits.container 由 city 的整值解析冻结，accounting 从 ExecHost 的 doctor 探测取得
ContainerRuntime，再交给 ExecTool::with_container。未声明显式 arm 或 container 保留既有平台默认；
声明但不可给则拒绝，不退回宿主。container 目标 argv 不经过宿主 nice 或宿主路径判定，
因为目标文件属于镜像；CLI 客户端照样通过 Backlog。doctor 默认臂不因此变成 container。
D51 container 的 shell System 指 Linux 镜像里的 /bin/sh -c，Pwsh 指镜像里的 pwsh
及既有非交互 flags；主机缺该解释器不证明镜像缺席。shell=false 仍拒绝 shell 臂。

## 5 权威信源

Docker create/start 的命令文法来自
https://github.com/docker/cli/blob/master/docs/reference/commandline/container_create.md 与
https://github.com/docker/cli/blob/master/docs/reference/commandline/container_start.md 。
Podman 的字段来自 https://docs.podman.io/en/latest/markdown/podman-info.1.html ，
命令文法来自 https://docs.podman.io/en/latest/markdown/podman-create.1.html 。
Docker 的 info 字段来自 https://docs.docker.com/reference/api/engine/ 。

## 6 命名统一

ContainerEngine 是 Docker 或 Podman；ContainerRuntime 持有已通过 info admission 的 CLI 路径与
ContainerEngine，后续 argv 文法按该值选择，不能从 CLI 文件名重新猜测后端。
ContainerLimits 的整数限额与 ContainerImage 的固定 ID 只由 kernel 定义。

## 7 模块边界

container.rs 是 decision，lifetime 是 adapter，control 拥有普通请求的有界 CLI 轮询
与 guardian 专属 create child 的持续等待，inspection
拥有 daemon JSON 契约；cleanup 独家定义 daemon 删除／缺席确认与副本释放，guardian
拥有独立进程的 stdin 生命周期和 ready／done 协议。只有 doctor 读 PATH；Backlog 管理起动、停止、结果与副本释放。

## 8 接口先行

`ContainerRuntime::admit(engine, path, info) -> Result<ContainerRuntime, AxError>`。
`ContainerRuntime::info_command(engine, path) -> Command` 供 doctor 的有界探测调用。
`create_command(&self, limits, launch) -> Result<Command, AxError>`，其中 ContainerLaunch
持有唯一容器名、副本路径与目标 Command 的借用。参数与环境取自 Command 的公开 API。
`ExecTool::with_container` 接收冻结限额及 doctor admitted runtime；已有 ExecSetup 构造不变。
`ContainerRuntime::guarded(harness: PathBuf) -> ContainerRuntime` 绑定可信的执行文件。
`run_container_guard(args: &[String]) -> Option<Result<(), AxError>>` 仅在主 CLI grammar
拒绝其内部 argv 后承接 ownership 文件路径，正常 help／version 优先级不变。

## 9 工作流程

Rust 的 admission 是一次性能力合取，没有多步探测状态：先判 JSON 形状，再判 Linux／cgroup／
控制器，最后才构造值。controls 模型规定这个合取；ResourceState 规定 Backlog 的清理责任，
CreationState 规定 guardian 对在途 create 与未知应答的所有权，两者不把父轮询超时当成删除确认。

## 10 实现逻辑

D39 container 的镜像与 argv 契约

镜像只能是本地不可变 image ID，由 User 自行准备；不推断默认镜像，不下载。
工作目录固定为容器里的 /work，只 bind 副本，根文件系统只读，关闭网络，去掉全部 capabilities，
禁止新特权，非零 UID，CPU 用整数 millicpu 转十进制字符串，内存与 swap 总额相同，
进程数有界。entrypoint 必须解成仅含目标 program 的单元素列表：Docker CLI 收原始 program；
Podman CLI 收由 serde_json 编码的单元素字符串数组，因为 Podman 优先将 entrypoint 当 JSON 数组解码。
Podman 的非 UTF-8 program 在创建前以 E_SANDBOX_DENIED 拒绝，不能有损转换改变程序身份。
args 逐项递交；拒空 program、移除环境变量的 Command
与含 mount 文法分隔符的副本路径，避免部分转译。
镜像必须没有声明 VOLUME，否则创建时会产生额外可写卷，起动前的 image inspect 必须拒绝它。
image inspect 的 Id 必须与 kernel 的固定 ID 相同，或等于其省略 sha256: 前缀的 digest，
因为 Podman 输出后者；Config 必须是对象，Volumes 可省略、为 null 或为空对象，
这些是未声明卷的合法格式，其余形状与非空卷表均拒绝。Podman 的容器 inspect
把单元素 entrypoint 报为字符串，Docker 报为数组；验收按后端 schema 核对同一 program。
固定 /work 是容器路径的唯一权威，宿主路径绝不被当成容器可执行文件路径。

被否：拼成 shell 文本、继承镜像 entrypoint、隐式 pull、只看 CLI version 即声明五轴已保。
这些替代会分别引入参数注入、改变请求语义、外部下载与虚假的隔离声明。

## 11 边界枚举

坏 JSON、未知 info 字段、cgroup v1、缺 cpu/memory/pids、Windows daemon、空 argv、
非 UTF-8 或含逗号／换行的 bind 路径、已移除的环境变量、标签式镜像、零额度均拒。
daemon 的权限／通信失败应在探测面以原失败与恢复语拒，不能生成一个成功的 info 值。

## 12 错误处理

输入值非法为 E_CONFIG_INVALID；daemon 能力与命令转译不成立为 E_SANDBOX_DENIED。
恢复语给出修正固定镜像、使 Linux daemon 的 cgroup v2 控制器可用或选择已有沙箱臂。
不能取得能力证据时不 fallback 到宿主或 copied_tree。

## 13 依赖选型

沿用 kernel、serde_json 与标准库 Command，不加依赖或新的生命周期管理器。

## 14 硬编码声明

固定容器工作目录的唯一 Rust 定义为 WORKDIR。CPU 单位换算由标准 millicpu 的 1000 定义。
镜像 ID 文法由 ContainerImage::parse 唯一定义，CLI 位置是探测输入，不进配置。

## 15 影响面

接入需要 city::config_layers 的整值解析、accounting::worker::workbench::tools 的冻结值传递、
Backlog 的成员同时拥有 CLI Child 与 daemon 清理句柄；halt、release、settle、harvest 与
spawn/enrol 失败必须走同一清理权威。容器收下限额之前不得执行目标程序。

## 16 测试与约束

`lake build crates.runtime.spec.Tools.Exec.Container` 证明任意能力列表中缺一项即拒。
Rust 的公开 admission／argv 测试检查实际实现，entrypoint 回归覆盖两个后端、JSON 数组形状、
空 JSON 数组、null、引号与普通路径，以及 Unix 下不可编码的 OsStr program；
非 UTF-8 输入直接经公开 create_command 检查 typed refusal，不经过有损字符串转换。
真实隔离验收由 `.github/workflows/on-demand.yml` 的 container job 在 disposable Linux runner 上运行 docker/podman，
先显式准备带 python3 与 /bin/sh 的镜像，再将不可变本地 image ID 交给生产 create；
测试检查副本写入、宿主回环拒绝、非 root UID、超额内存／进程分配、CPU 配额与清理后的后代缺席。
它不证明 rootless 委派、Desktop VM 或任意 OCI runtime 的行为，替换这些环境时须重跑。
`guardian_eof_waits_for_late_create_and_reaps_its_client` 检查模型允许的具体 EOF 轨迹：
经真实 ExecTool／Backlog 启动受控 CLI，create 在 FIFO 屏障内、父进程被终止、inventory 确认缺席，
放行迟到创建后要求 container／copy 消失、guardian 与 create child 被回收；成功与未知应答分别覆盖。
`guardian_timeout_retains_the_member_until_harvest` 经同一入口检查父进程仍存活时的轨迹：
create 屏障跨过父等待预算，typed failure 返回后成员与副本仍在，放行后由 harvest 回收，
失败 create 后 inventory 缺席不解除责任，直到实际删除成功。两项是模型的派生轨迹检查，
不是任意轨迹的实现精化证明。模型里的 created／failed 事件只在 create child 已被回收后发生。
未知应答后的删除成功与副本释放失败分开记忆，重试不丢已取得的删除确认。
若 daemon 在应答未知后永久不产生身份，也不给实际删除确认，owner 持续重试；模型只保证安全，
不声称这个环境下的清理活性。五轴声明须等真实挂载、回环联网、孙进程、身份与超额分配测试，不能从本模型推出。

## 17 文档关系

父分部 D32 与 wire D26 定选择名及默认；本分部规定 container admission 与登记后的生命周期，不另定选择表。
kernel Config 分部拥有 ContainerLimits；模块图登记 Rust 文件与本分部的关系。
-/

namespace Runtime.Tools.Exec.Container

/-- daemon 的全部必需能力都必须明确报告，缺一项即拒。 -/
def controls : List Bool → Bool
  | [] => true
  | first :: rest => first && controls rest

/-- 任意数量、任意顺序的能力证据中有拒绝项，合取就不能批准。 -/
theorem missing_control_refuses (checks : List Bool) (missing : false ∈ checks) :
    controls checks = false := by
  induction checks with
  | nil => simp at missing
  | cons first rest ih =>
    cases first <;> simp_all [controls]

example : controls [true, true, true, true, true] = true := rfl

/-- 清理责任从登记开始，未知应答不能撤销责任。 -/
inductive ResourceState where
  | registered
  | running
  | stopping
  | removed
  deriving DecidableEq, Repr

inductive ResourceEvent where
  | started
  | stop
  | uncertain
  | parentExited
  | removalConfirmed
  deriving DecidableEq, Repr

def advance : ResourceState → ResourceEvent → ResourceState
  | .removed, _ => .removed
  | _, .removalConfirmed => .removed
  | .registered, .started => .running
  | .running, .started => .running
  | .stopping, .started => .stopping
  | _, .stop => .stopping
  | _, .parentExited => .stopping
  | state, .uncertain => state

def applyTrace (state : ResourceState) (events : List ResourceEvent) : ResourceState :=
  events.foldl advance state

/-- 任意不含删除确认的轨迹均保留清理责任。 -/
theorem responsibility_survives_uncertain_trace (state : ResourceState)
    (owned : state ≠ .removed) (events : List ResourceEvent)
    (unconfirmed : ResourceEvent.removalConfirmed ∉ events) :
    applyTrace state events ≠ .removed := by
  induction events generalizing state with
  | nil => simpa [applyTrace] using owned
  | cons event rest ih =>
    have noHead : event ≠ .removalConfirmed := by
      intro h
      exact unconfirmed (by simp [h])
    have noTail : ResourceEvent.removalConfirmed ∉ rest := by
      intro h
      exact unconfirmed (by simp [h])
    have retained : advance state event ≠ .removed := by
      cases state <;> cases event <;> simp_all [advance]
    simpa [applyTrace, List.foldl] using ih (advance state event) retained noTail

/-- 已请求停止的成员不能由迟到的起动应答重新起动。 -/
theorem stop_cannot_restart : advance .stopping .started = .stopping := rfl

/-- 在途 create 与清理确认独立建模；EOF 不代表请求已经落定。 -/
inductive CreationState where
  | unsubmitted
  | pending
  | settled
  | uncertain
  | removed
  deriving DecidableEq, Repr

inductive CreationEvent where
  | submit
  | created
  | failed
  | parentExited
  | parentTimedOut
  | absent
  | deleted
  deriving DecidableEq, Repr

/-- guardian 持有唯一 create child；未知创建只由实际删除解除。 -/
def advanceCreation : CreationState → CreationEvent → CreationState
  | .removed, _ => .removed
  | .unsubmitted, .submit => .pending
  | .pending, .created => .settled
  | .pending, .failed => .uncertain
  | .unsubmitted, .absent => .removed
  | .settled, .absent => .removed
  | .unsubmitted, .deleted => .removed
  | .settled, .deleted => .removed
  | .uncertain, .deleted => .removed
  | state, _ => state

def creationTrace (state : CreationState) (events : List CreationEvent) : CreationState :=
  events.foldl advanceCreation state

/-- 任意 EOF／缺席／删除轨迹不能使仍在途的 create 失去 owner。 -/
theorem pending_creation_keeps_owner (events : List CreationEvent)
    (noCreated : CreationEvent.created ∉ events)
    (noFailed : CreationEvent.failed ∉ events) :
    creationTrace .pending events = .pending := by
  induction events with
  | nil => rfl
  | cons event rest ih =>
    have headCreated : event ≠ .created := by
      intro h
      exact noCreated (by simp [h])
    have headFailed : event ≠ .failed := by
      intro h
      exact noFailed (by simp [h])
    have tailCreated : CreationEvent.created ∉ rest := by
      intro h
      exact noCreated (by simp [h])
    have tailFailed : CreationEvent.failed ∉ rest := by
      intro h
      exact noFailed (by simp [h])
    have unchanged : advanceCreation .pending event = .pending := by
      cases event <;> simp_all [advanceCreation]
    simpa [creationTrace, List.foldl, unchanged] using ih tailCreated tailFailed

/-- 未知应答后的任意缺席轨迹不能充当实际删除。 -/
theorem uncertain_creation_requires_deletion (events : List CreationEvent)
    (noDeletion : CreationEvent.deleted ∉ events) :
    creationTrace .uncertain events = .uncertain := by
  induction events with
  | nil => rfl
  | cons event rest ih =>
    have head : event ≠ .deleted := by
      intro h
      exact noDeletion (by simp [h])
    have tail : CreationEvent.deleted ∉ rest := by
      intro h
      exact noDeletion (by simp [h])
    have unchanged : advanceCreation .uncertain event = .uncertain := by
      cases event <;> simp_all [advanceCreation]
    simpa [creationTrace, List.foldl, unchanged] using ih tail

#eval creationTrace .unsubmitted
    [.submit, .parentExited, .absent, .created, .deleted]
#eval creationTrace .unsubmitted
    [.submit, .parentTimedOut, .absent, .failed, .absent, .deleted]

end Runtime.Tools.Exec.Container
