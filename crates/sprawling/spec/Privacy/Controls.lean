-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy 控制表：privacy control 与 original item
规定 `bin::privacy::controls`（行按类别放在 `crates/sprawling/src/privacy/controls/` 下各一个文件）、
`bin::privacy::originals` 与 `bin::privacy::target`，以及 wire 上的名字闭集 `wire::privacy`
（`crates/wire/spec/Privacy.lean` §8-85）。
本文件是接口说明，不是形式证明：控制表是固定数据，没有可量化的轨迹；控制表进入写入路径的
性质（不写的控制没有写入，apply 的目标只来自控制表）由 crates.sprawling.spec.Privacy 的
reachable_catalogued 与 planApply 证明。

控制表的形状：
- privacy control 是一个可写目标：Target、写入值（Written）、类别、版本清单（honoured 与 ignored）、
  build effect。operation kind 与作用域由 Target 推出，不在行里另存（D65）：
  Registry{HKLM} 为 registry_value_hklm，Registry{HKCU} 为 registry_value_hkcu，
  UserEnvironment 为 environment_variable_user，ScheduledTask 为 scheduled_task_enabled；
  前者与计划任务是机器作用域（经 UAC），HKCU 与用户环境变量是用户作用域。
- Target 是 Registry{hive, path, name}、UserEnvironment{name}（即 HKCU\Environment 的同名值）、
  ScheduledTask{path, name}。Written 是 Dword（REG_DWORD）、Text（REG_SZ，带结尾 NUL）或
  Disabled（停用任务，定义不变），对应模型的 Recommendation；行的 Written 与其 operation kind
  相配（DWORD 配注册表值，Text 配环境变量，Disabled 配计划任务），由编译期断言检查。
  Snapshot 是 Registry(RawValue) 或 Task(TaskState)；TaskState 为 Absent、
  Enabled{definition_sha256} 或 Disabled{definition_sha256}，摘要取去掉启用标志后的任务 XML，
  所以停用不改变摘要，任务的任何其他改动都会改变它。对应模型的 Snapshot 与 TaskState。
- original item 是需求清单的一行原文，编号 K01–K52，原文照人写下的样子保存（含拼写错误），
  页面把人自己的话给人看。每行映射到 Writes(control) 或 NotWritten{reason, alternatives}；
  reason 是闭集 Absent、Obsolete、Undeterminable、NeedsOperationKind，对应模型的 Reason；
  alternatives 是最接近的可写控制，可以为空。
- 同义原项共享一个控制：两行原文指向同一目标时映射到同一个 privacy control，页面把原文并列。
- 控制表是唯一权威：路径、值名、写入值、类别、版本清单与 build effect 只在 `bin::privacy::controls`
  定义一次；`definition(PrivacyControl)` 是从名字到行的唯一穷尽映射，缺行即编译失败。
  wire 只带名字（PrivacyControl、PrivacyOriginal、PrivacyNotWritten、PrivacyCategory、
  PrivacyEdition、PrivacyBuildEffect、PrivacyEditionFit 七个闭集），客户端不复制路径、值或版本清单，
  完整目标路径由服务答案给出。每个控制的说明文字（标题、做什么、隐私收益、影响、容易忽略之处、
  恢复说明、版本说明）与每个不写原项的原因住在客户端 lang.json，按这些闭集取键，缺键即编译失败。
- 页面分九个类别，次序即 PrivacyCategory 的声明次序：诊断与遥测、语音键入与手写、定位与传感器、
  搜索、推荐广告与云端内容、活动历史与跨设备、云服务与网络连接、应用权限、Windows AI。
  PrivacyControl 的声明次序就是页面次序：先按类别，类别内按控制表次序；两者一致由测试检查。
- 版本清单取自 Microsoft 文档，版本是闭集 home、pro、enterprise、education、iot_enterprise、
  server；两张清单不相交（测试检查），都不含的版本是「未说明」。build effect 是闭集
  documented、uncertain、no_current_effect：后两者表示该设置在当前 Windows 上可能或确定不改变行为。

当前控制表：52 行原文中 39 行可写，映射到 37 个控制（K30 与 K23、K31 与 K22 两对同义原项）；
13 行不写：8 行 Absent（K04、K06、K39、K48–K52），4 行 Undeterminable（K02、K09、K11、K26），
1 行 NeedsOperationKind（K03）。原文之外另有 51 个控制（D61），共 88 个控制：
74 个 registry_value_hklm、8 个 registry_value_hkcu、2 个 environment_variable_user、
4 个 scheduled_task_enabled，即 78 个机器作用域、10 个用户作用域。
原项总数、可写与不写的行数、控制总数与四种 kind 的个数由 Rust 的编译期断言固定。

需要人知道的控制表取舍：
- 正式策略存在但模板没有给出启用值、也没有 Microsoft 来源说明省略时写什么的原项不写，
  原因 Undeterminable（K26）；补上编码后改为可写只是数据变更。
- 只有偏好键、而研究核实了正式替代策略的原项，写替代策略（CloudContent），原偏好键不写，
  页面说明这一点。
- 组件被弃用但策略仍在现行模板中的原项按可写处理，不算 Obsolete。
- 模板说只适用于旧版本、而 CSP 列出现行版本的策略（K14）照写，build effect 记为 uncertain。
- 存在同名 HKCU 策略的机器策略，只写 HKLM 一层。
- 会削弱安全防护的设置不列出（例如 Defender 的云保护与样本提交）：隐私不是安全，
  页面第一条事实说明这一点；这样的候选不进控制表，也不上 wire。
- 用户环境变量只写用户持久环境；sprawling 自己的 exec 环境按白名单清洗，它启动的进程不继承
  此变量，页面如实说明。
- 有些写入会删除 Windows 不再交还的数据，有些需要重启；这些事实在每个控制的容易忽略之处与
  恢复说明里，确认框在人点下之前显示它们（Privacy.Confirmation D55）。
- 计划任务在该主机不存在时不写，页面说明该主机上没有这个任务（模型 ApplyPlan.targetAbsent）。
-/

/-! D56 是否写由研究结论决定，不逐项取得生产写入资格（人的决定）
一个原项可写，当且仅当研究给出了它的正式目标、原始类型与写入值；研究说目标不存在、已废弃、
无法确定或需要本版本没有的 operation kind 的原项不写，页面给出原因。每种 operation kind 在
一次性 GitHub Actions Windows runner 上各实测一次 apply 与 restore，并在同一测试里逐个扫过全部
可写控制；每次 apply 在运行时读回，不符即报错并恢复（Privacy §9）。所以正确性由运行时读回保证，
而不是由逐项的事先资格保证。
被否：每个控制在写入前取得生产资格（逐项证明在真实主机上生效）——它让控制表的每一项都依赖一次
无法在人的主机上安全进行的实验，并且资格结论会随 Windows 版本漂移而需要持续复核；运行时读回
对每台主机、每次写入都成立。
重开参数：若某种 operation kind 的读回不能反映写入是否发生（例如目标被组策略刷新覆盖），
该 kind 需要自己的生效检查。
-/

/-! D61 控制可以超出人的原始清单；页面只告知，不劝导（人的决定）
控制表收入研究核实过的、原始清单之外的隐私设置（51 个），每个都写明它的代价与容易忽略之处。
页面不预选、不排序推荐、没有「全部应用」，也没有写着「推荐」的标记；写入值一栏叫「写入值」。
每一项只在人逐项明确点击并确认后写入。
被否：只收原始清单——人要求在研究允许的范围内尽量保护用户，清单之外的设置同样可能收集数据；
以推荐的口吻呈现——页面替人做了价值判断，而多数设置以便利为代价。
重开参数：若控制多到逐项浏览不可行，再讨论分组折叠，而不是预选。
-/

/-! D62 一次启用写多个值的策略，在有原子多值写入的 operation kind 之前不写
模板在一个 enabledList 里同时写多个值的策略（例如「关闭自动学习」同时写文本与墨迹两个值），
只写其中一个是组策略从不产生的状态，效果没有文档；拆成两个控制、让人分两次点，页面就能造出这种
半状态。所以这类策略作为候选记录、不写，直到有一种 operation kind 能原子地写入并恢复多个值。
被否：拆成两个独立控制——两次点击之间与只点其一时，主机处于未写明效果的状态。
重开参数：新增原子多值写入的 operation kind 时，这些候选改为可写只是数据变更。
-/

/-! D64 版本是否适用由服务器按控制表与主机 EditionID 判定一次
答案带主机的 EditionID 与它映射到的版本（映射规则在 `bin::privacy::windows`，按前缀，一处定义，
Privacy.Windows），每个控制带 PrivacyEditionFit：主机版本在 honoured 里为 Honoured，在 ignored 里
为 Ignored，否则（包括主机版本不属于六个具名版本）为 NotStated。判定是控制表的 Editions::fit，
客户端只显示结果，不重算。
被否：客户端拿两张清单与主机版本自己比较——版本清单与比较规则会在客户端再有一份，成为第二个权威。
重开参数：若 Microsoft 的适用范围依赖 build 号而不只是版本，判定要加入 build。
-/

/-! D65 operation kind、作用域与是否需要管理员由 Target 推出，不在行里另存
目标在哪里就决定了怎样写、写谁的设置：HKLM 值与计划任务是机器作用域，经 UAC；HKCU 值与用户环境
变量是用户作用域。行只存 Target，`Target::kind` 与 `OperationKind::scope` 用穷尽匹配推出其余两项。
被否：照研究表的形状，每行同时存 kind、scope 与 admin——同一事实写三遍，其中一份改错时没有东西
把它们拉回一致，例如一个 HKLM 目标被标成不需要管理员，就会绕开提升子进程。
重开参数：若出现同一位置可按两种方式写入的目标（例如一个值既可本进程写也必须提升），kind 才需要
成为行的独立字段。
-/
