-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::cli 的只读入口
规定 `crates/sprawling/src/privacy/cli.rs` 与它读取真实身份的
`crates/sprawling/src/privacy/identity.rs`。本文件是接口说明，不是形式证明：
status 是一次无状态查询，没有可以量化的轨迹；披露性质由类型持有（见 D54）。

pub fn status() -> Result<String, AxError> 依次做三件事：读取真实 Windows 身份，
失败即返回且不打开日志；通过 Home::detect().privacy_history() 调用 journal::read；
把 History::disclose 交出的摘要序列化成一行 JSON。没有 city 参数，没有网络或 wire。
main::verbs 的唯一 spelling 为 privacy status，Effect::ReadsOnly；main::router
直接调用 status，输出一行 JSON，失败输出 AxError 与 recovery 并退出失败。
摘要不包含 owner、raw bytes、绝对 home 路径；unresolved 不是失败或成功。
HistoryFault 的稳定 code/action/recovery 映射归 state，不在路由器重写。

当前入口只检查实时身份，不查询当前 registry 值，也不产生确认或修改。
inspect/confirm/apply/restore 尚未接入，读取已有 history 不能冒充首个控制完成；
未来入口仍走同一个 coordinator。
-/

/-! D54 status 的摘要也是身份披露，只交给 owner 引用在平台 Vault 中绑定的身份
History::fold 已核对所有 Prepared 的 owner 引用一致；History::disclose(authorize)
是摘要唯一的出口，非空摘要与它的 owner 引用存在同一个值里，所以
「有摘要而没有 owner」不可表示，调用者也无法绕过 authorize 序列化摘要。
status 的 authorize 是 gateway::verify_platform_identity(owner, 实时身份)：
只 get 平台 Vault，不运行写读删除的启动 probe，不使用环境遮蔽，不创建或覆盖绑定；
缺失返回 CredentialMissing，锁定保留平台错误码，不匹配返回 ConfigInvalid。
空历史没有 owner，不访问 Vault，返回空摘要。身份读取先于日志读取，
所以读不到身份时不报告日志是否损坏。每次查询保持日志字节不变。
此边界只核对 owner，不证明 journal 与引用的随机身份绑定；后续 writer 必须
同时建立该绑定，不能以本查询接口授权 apply/restore。
被否：①在 Lean 中把查询写成快照序列上的 map 再证明——每次查询相互独立，
定理只是复述定义的一支；②cli 先取 owner 再取摘要的两个 getter——
漏掉核对的调用者照样能序列化摘要。
Rust 检查在 privacy::cli::tests：proptest 生成合法历史（apply/restore、
NotApplied、Unknown 与未结操作），经真实 status_at 与
gateway::verify_identity_binding 的读取缝重放；同账号得到全部摘要，
跨账号无摘要，空历史不读 Vault，日志字节不变。
-/

/-! D55 身份由受保护安装目录下的 PowerShell 读取，不从 PATH 选可执行文件
Windows 安装路径通过 winreg 的安全接口，只读 HKLM 的
SOFTWARE\Microsoft\Windows NT\CurrentVersion/SystemRoot，再拼接
System32\WindowsPowerShell\v1.0\powershell.exe；不从 PATH、SystemRoot
环境变量或当前目录选可执行文件，因为同一用户的任何进程都能改这三处，
而 HKLM 需要管理员。路径读失败或不是绝对路径即拒绝。
命令关闭 profile、非交互，只输出
[Security.Principal.WindowsIdentity]::GetCurrent().User.Value；
经 doctor::asking 询问，只保留第一行，最多等 300 次 knock
（300 × asking::TICK = 15 秒，因为冷启动的 Windows PowerShell 要几秒才开始回答）。
超时、非零退出、无法启动均以 ToolUnavailable 拒绝，不保留 stopping 诊断中的路径或身份。
答案必须符合 SID 文法 `S-1-<authority>(-<sub-authority>)+`，每段是非空十进制数字；
不符合即拒绝，拒绝文字不回显答案。身份放在 Zeroizing 中，不写日志、不落盘。
非 Windows 没有对应 control，返回 ToolUnavailable，不伪造身份。
Rust 检查在 privacy::identity::tests（仅 Windows）：SID 文法的接受与拒绝；
以及经真实安装路径、真实 PowerShell 与 asking 只读查询一次，得到 SID——
只有这条走生产路径的检查能发现「答案带行尾」「保留了错误的行」一类缺陷。
被否：①whoami /user——输出带账户名，要解析再丢弃明文身份；
②windows crate 的 GetTokenInformation——需要 unsafe，按 AGENTS.md 平台调用的次序，
有安全接口时不用；③windows-registry crate——计划中的 privacy::windows 读写原始注册表值，
winreg 的 get_raw_value/set_raw_value 已是它要的安全接口，同一个 OS 接口只留一个依赖。winreg 的版本由 workspace 决定。
-/
