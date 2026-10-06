-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Windows privacy 平台契约
规定 `bin::privacy::windows` 及其三个适配器 `bin::privacy::windows::registry`、
`bin::privacy::windows::environment`、`bin::privacy::windows::task`，以及
`bin::privacy::elevation`。本节是平台接口说明，不是形式证明：适配器只读写控制表给出的目标，
是否写、写什么、写后怎样判定由 crates.sprawling.spec.Privacy 的 plan 与 Step 决定。

四种 operation kind：
- registry_value_hklm 与 registry_value_hkcu：winreg 的安全原始值接口，64 位视图
  （KEY_WOW64_64KEY）；读取保留值缺席、父键是否存在（Privacy.State D58）、类型码与原始字节，
  访问拒绝与其他 IO 失败分别报告；写入用原类型码与原字节，父键不存在时创建；原值缺席时恢复只删除
  本值，保留父键；
  REG_SZ/REG_EXPAND_SZ 的原字节无法无损重写（例如未终止的字符串）时 apply 前拒绝。
- environment_variable_user：用户持久环境，即 HKCU\Environment 的同名值，读写规则同上；
  每次写入后广播环境变更（WM_SETTINGCHANGE "Environment"），广播与 PATH 安装共用
  `bin::environment_broadcast` 一处实现。它在库 crate 中：install 是二进制 crate 的模块，库里的
  环境适配器调用不到那里。广播失败不致命：值已写下，写入报告「广播未送达，新登录后的程序才读到」，
  与 Install §8-9 的约定相同。新进程环境与持久值分开：已运行的进程与
  sprawling 清洗过的 exec 环境不继承此值，页面不声称它们已生效。
- scheduled_task_enabled：经受保护安装目录下的 Windows PowerShell（与身份读取共用同一路径解析，
  Privacy.Cli D54）调用 ScheduledTasks 模块：读取任务是否存在、是否启用、去掉启用标志后的任务 XML
  的 SHA-256；写入只用 Disable-ScheduledTask 与 Enable-ScheduledTask，从不删除或注册任务。
  脚本输出是一行 JSON，未知字段或状态拒绝。

注册表目标的根不限于 SOFTWARE\Policies：控制表里还有 SOFTWARE\Microsoft\Windows\CurrentVersion\Policies
下的值、SOFTWARE\Microsoft\OneDrive 下的值与 HKCU 的 Control Panel 下的值，适配器按控制表给出的
路径读写，不按前缀筛选。ERROR_ACCESS_DENIED 读作访问拒绝，与其他 IO 失败分开报告。

机器作用域（HKLM 与计划任务）的写入经 `bin::privacy::elevation`（D57）；用户作用域在本进程写入，
从不提升（D63）。
受保护的 Windows PowerShell 路径（Privacy.Cli D54）在 `bin::privacy::windows` 一处解析，
身份读取、计划任务适配器、提升与环境广播共用它。
主机信息：答案带该主机的 EditionID（原样）、它映射到的版本、CurrentBuild 与 UBR 组成的 build、
DisplayVersion，全部只读 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion，以及本进程是否已提升。
EditionID 按前缀映射，规则在 `bin::privacy::windows` 一处定义：Core 为 home，Professional 为 pro，
Education 为 education，Enterprise 为 enterprise，IoTEnterprise 为 iot_enterprise，Server 为 server，
其他为无映射（页面显示原样的 EditionID，每个控制的版本适用都是 NotStated，Privacy.Controls D64）。
非 Windows 主机没有这些适配器：所有条目只读，页面说明仅 Windows。

验收：只读用例在任何 Windows 上运行且不写注册表（不存在的随机键读作缺席且父键不存在；类型表往返）；
真实写入只在一次性 GitHub Actions Windows runner 上，按 Privacy §16 执行，绝不改人的主机设置。
-/

/-! D57 机器作用域的写入经短命的 UAC 提升子进程，父进程读回是唯一判定
父进程以 Start-Process 的 RunAs 动词启动当前可执行文件的 privacy elevated-write 动词，参数是一批
写入（目标、种类、类型码与原始字节）。子进程按 `bin::privacy::controls` 校验每个目标确在控制表中、
种类一致、字节长度不超过上界，然后只写，不读、不记录、不判定；父进程等待它退出后自己读回，
按 Privacy 的 judgeReadback 判定。UAC 被拒、子进程非零退出或访问拒绝都不单独当作结论，
一律由读回决定（多数情况读回原值，即 NotApplied）。回滚写入同样经提升子进程。
被否：①整个 server 以管理员运行——城市的所有工具与模型调用都会获得机器级写权限，而需要它的
只是这一小批注册表值与一个计划任务；②把日志放进 ProgramData 由管理员进程记录——日志的
拥有者会与人的身份分离，Vault 绑定（Privacy.State D52）无从核对，且需要一个常驻的提升进程。
重开参数：若某个机器作用域目标在提升子进程中写入后，非提升的父进程读不到（权限或视图不同），
该目标的读回也要移入子进程，并重新论证「父进程读回是唯一判定」。
-/

/-! D63 HKCU 的写入从不提升；访问拒绝以 NotApplied 结束
HKCU\Software\Policies 下的值（以及其他 HKCU 目标）在本进程写入。标准账户能否写 Software\Policies
没有来源说明；写入被拒时目标读回原值，按现有判定以 NotApplied 结束，错误带稳定码 access_denied，
说明这个账户不能写这个策略键、什么都没有改变。不新增状态。
被否：把 HKCU 写入也交给提升子进程——在「越肩」UAC（输入另一个管理员账户的凭据）下，提升子进程的
HKCU 是那个管理员账户的配置单元，写入会落到另一个人的设置上，而父进程读回的仍是本人的值。
重开参数：若需要在标准账户下写 HKCU 策略，只能经能代表本人写入的机制（例如本人账户的计划任务），
而不是提升。
-/
