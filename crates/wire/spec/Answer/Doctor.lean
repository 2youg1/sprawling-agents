-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::doctor

规定 `answer::doctor`（`crates/wire/src/` 下同名的文件）。运行中的机器有什么，逐项写成页面贴标签的值而不是句子。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-25 `Query::Doctor`：运行中的机器有什么

```rust
// Query 第 24 条（声明序，QUERY_NAMES 同序追加）
Doctor,                                   // → Answer::Doctor(Box<DoctorAnswer>)

pub struct DoctorAnswer { pub items: Vec<DoctorItem>, pub tiers: Vec<DoctorVerdict>,
                          pub sandbox: DoctorSandbox, pub custody: DoctorCustody,
                          pub core: DoctorCore }
pub struct DoctorItem { pub name: String, pub tier: DoctorTier, pub need: DoctorNeed,
                        pub homepage: Option<String>, pub state: DoctorState,
                        pub install: DoctorInstall }
pub enum DoctorTier { Use, Develop }
pub enum DoctorNeed { Required, Optional }
pub enum DoctorState { Present { at, version }, Broken { at, fault }, Absent { absence } }
pub enum DoctorVersion { Said { text }, Silent, Unreadable, Late }
pub enum DoctorFault { WillNotStart { said }, HalfWritten, Unreadable { said } }
pub enum DoctorAbsence { NotOnSearchPath, VariableNamesNothing { variable, path },
                         NoComponent { dir }, NoHome, NotInThisBuild }
pub enum DoctorInstall { Command { spelled }, Print { spelled }, Manual { how }, UnknownPlatform }
pub struct DoctorVerdict { pub tier: DoctorTier, pub missing: Vec<String> }

// 与逐件的 items 并列的两道整机读数：命令跑在什么盒子里、凭据住在哪里。
pub struct DoctorSandbox { pub arm: DoctorSandboxArm, pub coverage: Vec<DoctorGuarantee> }
pub enum DoctorSandboxArm { LinuxNamespaces, WindowsJobObject, CopiedTree,
                            Unavailable { missing: DoctorSandboxMissing } }
pub enum DoctorSandboxMissing { ScratchDirectory }
pub struct DoctorGuarantee { pub axis: DoctorGuaranteeAxis, pub kept: DoctorCoverage }
pub enum DoctorGuaranteeAxis { Filesystem, Network, ProcessTree, User, Resources }
pub enum DoctorCoverage { Kept, NotKept }
pub struct DoctorCustody { pub store: DoctorCustodyStore, pub keeps: DoctorCustodyLifetime,
                           pub refusal: Option<String> }
pub enum DoctorCustodyStore { PlatformService, EncryptedFile, SessionMemory }
pub enum DoctorCustodyLifetime { AcrossReboots, WithPassphrase, UntilReboot, ThisProcess }
pub enum DoctorCore { Raised, HeldBySetting, Refused { said }, LoweredByValve, Unasked { said } }
```

- **每一种状态都是枚举，不是句子**。终端那份报告是一台机器的散文，而浏览器说两种语言；线上若携措辞，页面的用词就成了服务端的选择。「有了它能做什么」那一句也不上线：`name` 就是这一项的 id（需求表里的名字，如 `cargo-nextest`），页面按它从 `lang.json` 的 `machine_enables_<name>`（连字符写成 `_`）取两种语言的那一句，终端的英文留在需求表（sprawling D6「doctor 的「能做什么」各面自持」）。线上仍携的句子只有 `said`：平台或程序自己说的话，读者推不出来。
- **答的是城启动时看到的那一眼，不是现问现看**。每一项都是起一个进程问版本；一次查询若这么做，会把答一切读的那条线程按住数秒。城若没看过（一次一条命令驱动的工人就是），答 `Unavailable`——与「一栋没人盖过的楼」同口径：**「我没看」是它自己的答案**，而一台空机器会让页面告诉人他手上每件工具都缺。
- **`install` 把平台不明单列一支**。三个平台之外的机器上，本项目没有任何配方；此时拼一条别的平台的命令是错的，沉默也是错的。
- **沙箱的保证逐轴作答，不是一句「已隔离」**：`coverage` 逐轴一行，`Kept`／`NotKept` 两个字而不是布尔——页面两态都要有词，布尔会让每个读者自己给 `false` 选一个。它存在的理由，是 agent 在动手前要读得到哪几条保证没成立。臂与轴的定义住 ``crates/runtime/Spec.lean` §8-13-2`（`Confinement` 与 `Guarantee`），线上重拼一份，逐臂对应只住 `sprawling::doctor::report` 的穷尽匹配——上游加一臂即编译红。
- **凭据的存放与寿命一起答，`refusal` 是平台服务自己的话**：三者同出 `gateway::Custodian::probe` 的一次往返（`gateway::Custody`，`crates/gateway/Spec.lean` §8-4；寿命的全部档位见 §8-21），线上重拼 `Store` 与 `Persistence` 两套词，逐臂对应同住 `sprawling::doctor::report`；`refusal` 缺席读作服务没有拒——或该 store 由城自选，没有服务可拒。
- **核心线程站在哪一档，`said` 是平台自己的话**：`core` 是主机此刻会给核心线程的档位（`crates/sprawling/Spec.lean` §8-93、§8-166）——升到正常档之上一级、按人的 `[core] priority` 留在正常档、平台拒绝（Unix 上没有 `CAP_SYS_NICE`）、被安全阀降回，或 doctor 没能问到。派出的命令不在这里：它们总是低一档，降档从不被拒（`crates/runtime/Spec.lean` §8-13-3）。
- **服务端**：`sprawling::doctor::report` 把 findings 与这两道整机读数折成本形状，`Views` 存一份（`crates/sprawling/Spec.lean` §8-54）。
-/

/-!
### 8-33 `DoctorInstall` 与 `DoctorRefresh`：机器上的两个动词

`Query::Doctor` 答的是开城那一刻的快照（§8-25），于是机器页只能复制一行命令去终端跑，跑完还得重启城才看得见结果。两条命令补上这段：

```rust
Command::DoctorInstall { item: String, idem: IdemKey }   // 按需求表里的名字装一件
Command::DoctorRefresh { idem: IdemKey }                 // 重新探一遍，取代启动快照
```

- **只跑 `Recipe::Command`**。`Print` 与 `Manual` 各自带着「人自己去做什么」被拒——管道进 shell 的脚本是没人读过的代码，这条纪律不因为请求来自页面而不是终端就松一格。执行走的是终端那条 `Machine::install`，不是第二个安装器。
- **进度就是日志行**。安装是本城起的一个进程并等它，值得报告的两件事——将要跑什么、怎么结束的——正好是一行日志的形状。第二条进度通道会是同一件事的第二个权威。
- **`DoctorInstall` 装完自己再探一遍**，而不是让页面记得补一帧：装完仍答启动快照的城，会告诉人他刚装的东西还是没有。
- **两者都不入账本**。机器有什么不是这座城里发生的事：它在本进程之外被改变，写进历史就是写进一份会错的历史。答案沿 `RunWorker::examine` 交给服务层的 views，与开城那一次的写法同一条。
- **为什么不是 `Query::Doctor { fresh: true }`**：读要拿着 views 的锁答，而探测是十几个进程各被起一次的几秒钟；那会让一次读把其他每一次读都堵住。命令在写线程上跑，那里本来就是本城把工作排成一列的地方。
-/

/-!
### 8-50 依赖页的三个版本与一包：`DoctorItem.pinned`／`pack`、`Query::UpstreamVersion`

```rust
pub struct DoctorItem { …, pub pinned: Option<String>, pub pack: Option<DoctorPack> }
pub enum DoctorPack { RustTools }
// Query 追加在声明序末尾
UpstreamVersion { item: String },          // → Answer::Upstream(Box<DoctorUpstream>)
pub struct DoctorUpstream { pub item: String, pub newest: DoctorNewest }
pub enum DoctorNewest { Asking, Read { version: String }, Unread { why: DoctorUnread }, Refused { said: String } }
pub enum DoctorUnread { WithToolchain, ManyBrands, MatchesBrowser, ThisProject, NoSource, UnknownItem }
```

- **每项一问，而不是一份答案里的一个字段**：上游版本来自六个不同的站点，一个慢的站点不能拖住整页；页面对每一项各问一次，答一个填一个。塞进 `DoctorAnswer` 就得等最慢的那一个，或者要第二条推送通道。
- **`Asking` 让问题不等网络**：一个会话的问题按到达的次序一个一个答，一个要出网几秒的问题会挡住它后面的每一个；城先答 `Asking`，在后台去读，页面过一会儿再问。
- **`pack` 是一个枚举而不是一个字符串**：页面要给这一包起名字、写说明，所以它必须是页面认得的封闭集合；新的一包是每个读者处的编译错误。
- **`DoctorUnread` 是封闭的原因**：「读不到」有几种，每一种页面各有一句话，线上不带句子（§8-25 同一条理由）。`Refused.said` 带的是网络在哪一步停下，那是平台自己的话。
- **`pinned` 是版本号本身**，已从仓库的文件里读好；没有钉子的项为 `None`。
- 城那一侧从哪里读、怎么记住读数，见 `crates/sprawling/Spec.lean` §8-120。
-/
