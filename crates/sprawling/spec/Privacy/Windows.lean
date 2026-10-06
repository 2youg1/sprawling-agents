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
  访问拒绝与其他 IO 失败分别报告；写入用原类型码与原字节；原值缺席时恢复只删除本值，保留父键；
  REG_SZ/REG_EXPAND_SZ 的原字节无法无损重写（例如未终止的字符串）时 apply 前拒绝。
- environment_variable_user：用户持久环境，即 HKCU\Environment 的同名值，读写规则同上；
  每次写入后广播环境变更（WM_SETTINGCHANGE "Environment"），广播与 PATH 安装共用
  `bin::install::environment_broadcast` 一处实现。新进程环境与持久值分开：已运行的进程与
  sprawling 清洗过的 exec 环境不继承此值，页面不声称它们已生效。
- scheduled_task_enabled：经受保护安装目录下的 Windows PowerShell（与身份读取共用同一路径解析，
  Privacy.Cli D54）调用 ScheduledTasks 模块：读取任务是否存在、是否启用、去掉启用标志后的任务 XML
  的 SHA-256；写入只用 Disable-ScheduledTask 与 Enable-ScheduledTask，从不删除或注册任务。
  脚本输出是一行 JSON，未知字段或状态拒绝。

机器作用域（HKLM 与计划任务）的写入经 `bin::privacy::elevation`（D57）；用户作用域在本进程写入。
主机信息：答案带该主机 EditionID（只读 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion）与本进程
是否已提升。非 Windows 主机没有这些适配器：所有条目只读，页面说明仅 Windows。

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
