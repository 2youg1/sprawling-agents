-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy::cli 的本地入口
规定 `crates/sprawling/src/privacy/cli.rs`、它读取真实身份的
`crates/sprawling/src/privacy/identity.rs`，以及写入动词交给 coordinator 的生产 Host
`bin::privacy::windows::host`。本文件是接口说明，不是形式证明：写入动词的次序与性质由
crates.sprawling.spec.Privacy 证明，CLI 只把参数交给同一个 `bin::privacy::coordinator`；
status 与 inspect 是无状态查询，披露性质由类型持有（见 D53）。

动词（main::verbs 各登记一次，main::router 直接调用；时钟由 router 交入 assembly 的 SystemClock，
不在 privacy 里取样）：
- privacy status（ReadsOnly）：依次读取真实 Windows 身份（失败即返回且不打开日志）、
  通过 Home::detect().privacy_history() 调用 journal::read、把 History::disclose 交出的摘要
  序列化成一行 JSON。没有 city 参数，没有网络或 wire。
- privacy inspect [control]（ReadsOnly）：先读真实身份与历史（同 status 的次序），经
  History::holdings 以只读的 () owner 核对后（Privacy.State），每个控制一行 JSON（给出 control 时只那一行）：
  control、reading（读到的快照，即 apply 与 restore 的 expected）与 key_existed，或读失败时的
  unreadable（access_denied 或 failed）；owned（本应用仍拥有的最近操作的 operation、original、
  modified，否则为 null）；unresolved（该控制是否有未结操作）。所有控制都可写
  （Privacy D66），不写的原项不是控制，所以不出现在这里。
- privacy apply <control> <expected> 与 privacy restore <control> <expected>（Changes）：
  expected 是 inspect 为该控制打印的 reading JSON，原样传回；它就是 Privacy.Confirmation D55 的绑定，
  缺少或不符即拒绝。
- privacy restore-all（Changes）：按控制逐个恢复本应用仍拥有的修改（Privacy D60），每个控制
  用恢复前刚读到的值作 expected，输出每个控制各自的结果；第一个失败之后不再继续，
  已完成的结果照样输出，退出失败。
- privacy reconcile <expected>（Changes）：对未结操作执行人的核对（Privacy 的 Reconcile），
  expected 是 inspect 为该操作的控制打印的 reading；不写系统。
- privacy elevated-write <write>（Changes）：只供 `bin::privacy::elevation` 的提升子进程使用，
  write 是一次机器作用域写入的十六进制 JSON，见 Privacy.Windows D57；它不经 coordinator、不读日志、
  不读系统、不输出读数，校验失败或写入失败时以 AxError 退出失败。
apply、restore、reconcile 成功时输出一行 JSON：{"done":"applied"|"restored","operation":n}、
{"done":"already_written"} 或 {"done":"reconciled","operation":n,"settlement":…}；restore-all 每个
控制一行 {"control":…,"done":…}。失败时把 AxError 与 recovery 写到 stderr 并退出失败；
--json 时 stderr 是一行 AxError JSON，subject 以 Privacy §12 的稳定码开头，一次性 runner 的
验收按它判定。CLI 是一次性 runner 验收进入生产路径的入口。
输出不包含 owner、绝对 home 路径；读数与原值按 Privacy.State D52 明文。
HistoryFault 的稳定 code 与 recovery 映射归 `bin::privacy::fault`，调用者只给出自己的 action，不在路由器重写。

生产 Host（`bin::privacy::windows::host`）：
- identity 是 `bin::privacy::identity` 读到的 SID；owner(recorded, identity)：有记录的引用时经
  gateway::verify_platform_identity 核对后原样返回；空历史时抽取 64 位随机数，作名字
  owner-<16 位小写十六进制> 在 realm privacy 下建一个新引用，经 gateway::bind_platform_identity
  把 SID 写进平台 Vault（gateway D30），返回这个引用。空历史第一次写入之前的失败（例如 changed）
  会留下一条没有历史引用的绑定，它只在本人的凭据库里保存本人的 SID。
- read 与 write 按控制表的目标交给适配器（Privacy.Windows）：HKCU 值与用户环境在本进程，
  HKLM 值与计划任务经 `bin::privacy::elevation`；适配器的访问拒绝映射为 Privacy §12 的
  access_denied，UAC 被拒映射为 elevation_declined，其他失败带原因。计划任务没有父键，
  它的读数记录 key_existed 为 true，所以从不报告残留的空键。
- 用户环境的写入在广播失败时仍是成功的写入：值已写下，读回判定结论；广播失败只影响已运行的
  程序何时读到它。
-/

/-! D53 status 的摘要也是身份披露，只交给 owner 引用在平台 Vault 中绑定的身份
History::fold 已核对所有 Prepared 的 owner 引用一致；History::disclose(authorize)
是摘要唯一的出口，非空摘要与它的 owner 引用存在同一个值里，所以
「有摘要而没有 owner」不可表示，调用者也无法绕过 authorize 序列化摘要。
status 的 authorize 是 gateway::verify_platform_identity(owner, 实时身份)：
只 get 平台 Vault，不运行写读删除的启动 probe，不使用环境遮蔽，不创建或覆盖绑定；
缺失返回 CredentialMissing，锁定保留平台错误码，不匹配返回 ConfigInvalid。
空历史没有 owner，不访问 Vault，返回空摘要。身份读取先于日志读取，
所以读不到身份时不报告日志是否损坏。每次查询保持日志字节不变。
inspect 与 restore-all 经只读的 History::holdings 取得拥有栈栈顶与未结操作，非空历史的 authorize
与 status 相同；空历史同样不访问 Vault。
此边界只核对 owner；写入动词经 coordinator 用同一核对，身份不符时在披露任何历史之前拒绝。
被否：①在 Lean 中把查询写成快照序列上的 map 再证明——每次查询相互独立，
定理只是复述定义的一支；②cli 先取 owner 再取摘要的两个 getter——
漏掉核对的调用者照样能序列化摘要。
Rust 检查在 privacy::cli::tests：proptest 生成合法历史（apply/restore、
NotApplied、Unknown 与未结操作），经真实 status_at 与
gateway::verify_identity_binding 的读取缝重放；同账号得到全部摘要，
跨账号无摘要，空历史不读 Vault，日志字节不变。
-/

/-! D54 身份由受保护安装目录下的 PowerShell 读取，不从 PATH 选可执行文件
Windows 安装路径通过 winreg 的安全接口，只读 HKLM 的
SOFTWARE\Microsoft\Windows NT\CurrentVersion/SystemRoot，再拼接
System32\WindowsPowerShell\v1.0\powershell.exe；不从 PATH、SystemRoot
环境变量或当前目录选可执行文件，因为同一用户的任何进程都能改这三处，
而 HKLM 需要管理员。路径读失败或不是绝对路径即拒绝。
命令关闭 profile、非交互，只输出
[Security.Principal.WindowsIdentity]::GetCurrent().User.Value；
经 doctor::asking 询问，只保留第一行，最多等 1200 次 knock（1200 × asking::TICK = 一分钟）。
这个上界由 `bin::privacy::windows` 一处定义，身份查询、计划任务查询与环境广播共用：冷启动的
Windows PowerShell 在同时有其他构建的工作站上空命令就要 5.1 秒，与整个测试套件并行时身份查询用了
13.9 秒、任务查询超过 15 秒；一分钟是所见最慢回答的四倍，而查询只在人等页面时等待。
超时、非零退出、无法启动均以 ToolUnavailable 拒绝，不保留 stopping 诊断中的路径或身份。
答案必须符合 SID 文法 `S-1-<authority>(-<sub-authority>)+`，每段是非空十进制数字；
不符合即拒绝，拒绝文字不回显答案。身份放在 Zeroizing 中，不写日志、不落盘。
非 Windows 没有对应 control，返回 ToolUnavailable，不伪造身份。
Rust 检查在 privacy::identity::tests（仅 Windows）：SID 文法的接受与拒绝；
以及经真实安装路径、真实 PowerShell 与 asking 只读查询一次，得到 SID——
只有这条走生产路径的检查能发现「答案带行尾」「保留了错误的行」一类缺陷。
被否：①whoami /user——输出带账户名，要解析再丢弃明文身份；
②windows crate 的 GetTokenInformation——需要 unsafe，按 AGENTS.md 平台调用的次序，
有安全接口时不用；③windows-registry crate——`bin::privacy::windows::registry` 读写原始注册表值，
winreg 的 get_raw_value/set_raw_value 已是它要的安全接口，同一个 OS 接口只留一个依赖。winreg 的版本由 workspace 决定。
-/
