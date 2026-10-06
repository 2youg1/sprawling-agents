-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy 控制表：privacy control 与 original item
规定 `bin::privacy::controls`、`bin::privacy::originals` 与 `bin::privacy::target`。
本文件是接口说明，不是形式证明：控制表是固定数据，没有可量化的轨迹；控制表进入写入路径的
性质（不写的控制没有写入，apply 的目标只来自控制表）由 crates.sprawling.spec.Privacy 的
reachable_catalogued 与 planApply 证明。

控制表的形状：
- privacy control 是一个可写目标：目标（Target）、operation kind、推荐快照、作用域（用户或机器）、
  是否需要管理员。operation kind 是闭集：registry_value_hklm、registry_value_hkcu、
  environment_variable_user、scheduled_task_enabled。机器作用域（HKLM 与计划任务）需要管理员，
  用户作用域（HKCU 与用户环境变量）不需要。
- Target 是 Registry{hive, path, name}、UserEnvironment{name}、ScheduledTask{path, name}；
  Snapshot 是 Registry(RawValue) 或 Task(TaskState)，TaskState 为 Absent 或
  Present{enabled, definition_sha256}，摘要取去掉启用标志后的任务 XML。
  对应模型的 Target 种类、Snapshot 与 Recommendation（Privacy §6）。
- original item 是需求清单的一行原文，编号 K01–K52，共 52 行，长度由编译期断言固定。
  每行映射到 Writes(control) 或 NotWritten(Reason)；Reason 是闭集 Absent、Obsolete、
  Undeterminable，对应模型的 Reason。
- 同义原项共享一个控制：两行原文指向同一目标时映射到同一个 privacy control，页面把原文并列。
- 控制表是唯一权威：路径、值名、推荐值、作用域只在 `bin::privacy::controls` 定义一次；
  wire 只带控制与原项的名字（PrivacyControl、PrivacyOriginal 两个闭集），客户端不复制路径或值，
  完整目标路径由服务答案给出。每个控制的说明文字（做什么、隐私收益、影响）与每个不写原项的原因
  住在客户端 lang.json，按这两个闭集取键，缺键即编译失败。
- 页面按类别分组（遥测与诊断、语音与输入、定位与传感器、搜索与内容、应用权限、计划任务）；
  类别是控制的一个字段，同样只在控制表定义。

当前目录：52 行原文中 37 行可写，映射到 35 个控制（两对同义原项）；15 行不写，原因都是
Undeterminable。35 个控制中 28 个是机器作用域（27 个 HKLM 值、1 个计划任务），7 个是用户作用域
（6 个 HKCU 值、1 个用户环境变量）。

需要人知道的控制表取舍：
- 正式策略存在但研究没有取得模板中的原始注册表类型与启用值的原项不写，原因 Undeterminable；
  补上模板中的编码后，它们改为可写只是数据变更。
- 只有偏好键、而研究核实了正式替代策略的原项，写替代策略（CloudContent），原偏好键不写，
  页面说明这一点。
- 组件被弃用但策略仍在现行模板中的原项按可写处理，不算 Obsolete。
- 存在同名 HKCU 策略的机器策略，只写 HKLM 一层。
- 用户环境变量只写用户持久环境；sprawling 自己的 exec 环境按白名单清洗，它启动的进程不继承
  此变量，页面如实说明。
- 某些策略只在特定 Windows 版本执行；答案带主机的 EditionID，页面在相关控制旁写明。
- 计划任务在该主机不存在时不写，页面说明该主机上没有这个任务（模型 ApplyPlan.targetAbsent）。
-/

/-! D56 是否写由研究结论决定，不逐项取得生产写入资格（人的决定）
一个原项可写，当且仅当研究给出了它的正式目标、原始类型与推荐值；研究说目标不存在、已废弃
或无法确定的原项不写，页面给出原因。每种 operation kind 在一次性 GitHub Actions Windows
runner 上各实测一次 apply 与 restore，并在同一测试里逐个扫过全部可写控制；每次 apply 在运行时
读回，不符即报错并恢复（Privacy §9）。所以正确性由运行时读回保证，而不是由逐项的事先资格保证。
被否：每个控制在写入前取得生产资格（逐项证明在真实主机上生效）——它让控制表的每一项都依赖一次
无法在人的主机上安全进行的实验，并且资格结论会随 Windows 版本漂移而需要持续复核；运行时读回
对每台主机、每次写入都成立。
重开参数：若某种 operation kind 的读回不能反映写入是否发生（例如目标被组策略刷新覆盖），
该 kind 需要自己的生效检查。
-/
