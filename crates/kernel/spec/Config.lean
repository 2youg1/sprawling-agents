-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::config

规定 `kernel::config`（`crates/kernel/src/config.rs` 与 `crates/kernel/src/config/interpreter.rs`）：分层配置与 Run 起点冻结的那一份。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::config::tests` 守住。
-/

/-!
### 8-22 kernel::config

```rust
pub enum ClockStampGranularity { Off, Minute, FiveMinute, Hour }   // 类型住 kernel 非 runtime
pub struct LayeredValue<T> { pub city: Option<T>, pub building: Option<T>, pub resident: Option<T> }
impl<T> LayeredValue<T> { pub fn resolve(&self) -> Option<&T>; }  // resident→building→city 下层覆盖上层

pub struct FrozenConfig {                                            // Run 起点冻结；字段逐条说明见下
    pub clock_stamp: ClockStampGranularity, pub clock_zones: Vec<ClockZone>,
    pub sandbox: SandboxLimits, pub mcp: Vec<McpServer>,
    pub effort: Option<Effort>, pub second_threshold: Option<SecondThreshold>,
    #[serde(default)] pub search: SearchConfiguration,               // 缺省 Default
}
pub struct LiveConfig {}                                             // 热载面；今天没有字段
pub struct StatedConfig {                                            // 梯子上每个关切各一格，一起交给 freeze
    pub clock_stamp: LayeredValue<ClockStampGranularity>, pub clock_zones: LayeredValue<Vec<ClockZone>>,
    pub effort: LayeredValue<Effort>, pub sandbox: LayeredValue<SandboxLimits>,
    pub mcp: LayeredValue<Vec<McpServer>>, pub second_threshold: LayeredValue<SecondThreshold>,
    pub search: LayeredValue<SearchConfiguration>,
}
pub fn freeze(stated: &StatedConfig) -> FrozenConfig;
```

- **`freeze` 收一个 `StatedConfig`，不收七个参数**：每个关切的 `LayeredValue` 总是一起从 `city::load_config` 走到 `freeze`，一起走的值是一个具名的值；`search` 是第七个关切，七个参数会越过每个函数四个参数的上限。加一个关切因此是 `StatedConfig` 多一个字段，`freeze` 的签名不再变。

- **无字段交集可机械判**：单测将两型缺省值 serde 成 JSON，断言键集交集为空；新增字段自动入判。
- `CLOCK_STAMP_DEFAULT: ClockStampGranularity = Minute` 落 consts_policy：没有一级写 `[clock] stamp` 的城，`Timestamped` 结果每条带戳，`Timeless` 结果每分钟至多一条（runtime D8）。

**时钟分区（config）**：`ClockZone { id, offset_min }`（已解析偏移，恒不记时区名——重解会随时区库版本分叉重放历史）；`FrozenConfig.clock_zones` 由 `freeze` 的同名梯解析；zones 梯整表覆盖（下层写即替换上层全表）。本段属 kernel::config（§8-22），就近登记于此避免拆章。

**思考强度（config）**：`FrozenConfig.effort: Option<Effort>`（类型住 §8-24），缺省 `None`＝不写该字段、由 provider 自行决定。

**沙箱限额（config）**：`SandboxLimits { shell: bool, interpreter: Interpreter, fuel: u64, mounts: Vec<Address>, env_passthrough: Vec<EnvVarName>, trusted: Vec<ServerLabel>, container: Option<ContainerLimits>, arm: Option<SandboxArm> }`，即 `FrozenConfig.sandbox`。三条口径：①**整值解析而非逐字段合并**——一层说到 sandbox 就说全部，于是欠说的层只会收窄而恒不会悄悄放开上层没提过的能力；②**主机事实不入城**（CPython 工件路径、shell 可执行文件位置走环境变量）——一座城被搬到另一台机器时不该带着运行中的机器的路径；③冻结的理由与工具表相同：**能改变可达范围的东西恒不在回合中变宽**，否则变宽的那一刻没有人审过。缺省 `fuel = SANDBOX_FUEL_DEFAULT`（`consts_policy`，2×10⁸），`shell = false`——shell 是唯一一条从参数读不出可达范围的臂。

**shell 臂的解释器（config）**：`pub enum Interpreter { System, Pwsh }`，`SandboxLimits.interpreter`，缺省 `System`，文件里写作 `[sandbox] interpreter = "system"` 或 `"pwsh"`（serde 小写，`#[serde(default)]`，所以没写这个键的层与线上旧帧读作 `System`）。配置文件里的值只经 `Interpreter::parse` 构造（`city::config_layers` 把这个键读成文本再交给它；serde 只读线上帧与冻结配置）：别的拼写以 `E_CONFIG_INVALID` 拒，主语是这个键与写下的值，恢复语给出两种拼法；不猜，也不当作缺省。它与 `shell` 是两件事：`shell` 决定 shell 臂给不给，`interpreter` 决定给的时候是哪一个；写的是名字而不是路径，理由同「主机事实不入城」。口径与 `mounts` 同形：整值上梯、Run 起点冻结、解析点拒。为什么要它、在主机上怎么解析、缺席时怎么拒，住 `crates/runtime/Spec.lean` §8-13-2 D30。

**外部 MCP server（config）**：`McpServer { label: ServerLabel, transport: McpTransport }`，`McpTransport { Stdio { command, args, env }, Http { url, headers }, Sse { url, headers } }`——**穷尽枚举而非两个裸字段**：一行既写 command 又写 url 就是一行要读者去猜的配置，故配置层当场拒（`ServerLabel` 住 §8-23）。**枚举是闭的**（无 `#[non_exhaustive]`）：读者全在这一个二进制里，通配臂只会把下一种 transport 从必须表态的模块面前藏起来。`env` 与 `headers` 皆为名在前、值在后的成对表，值可以是 `secret:realm/name` 引用——交给子进程的名字收不回来，故兑付发生在起进程／发请求的那一格，而恒不写进配置文件。`Sse` 自成一支而不是 `Http` 的一个开关：两者开法与败法都不同。`FrozenConfig.mcp: Vec<McpServer>` 缺省空表＝这栋楼不接任何外部 server。三条口径：①**整表覆盖**，与 zones／sandbox 同一条理由——一层说到 `[[mcp]]` 就说全部，欠说的层只会收窄而恒不会悄悄接上上层没提过的服务；②**冻结的理由就是工具表本身**——外部工具在 Run 起点入 catalog，而 provider 把工具数组哈希在 system prompt 之前，Run 内变宽的工具表既自毁缓存又没有人审过；③**命令与参数是主机事实**（一个可执行文件在运行中的机器上的位置），故它们住 `CONFIG.toml` 而恒不入 Ledger 载荷——一座城被搬到另一台机器时不该带着运行中的机器的路径。

**网络搜索（config）**：子模块 `config::search` 定义两个值，`FrozenConfig.search` 持有后一个。

```rust
pub struct SearchSupplier {                // 一家经 MCP streamable HTTP 接入的搜索服务
    pub id: ServerLabel,                   // 供应方的名字，也是 web_search 的 Connector label
    pub url: String,                       // MCP HTTP 地址；判定同 [[mcp]] 的 url（city D25）
    pub remote: String,                    // tools/list 里那个远端工具的名字
    pub query_field: String,               // web_search 的 query 交给远端哪个参数
    pub objective_field: Option<String>,   // objective 交给哪个参数；None＝这家不收 objective
    pub count_field: Option<String>,       // num_results 交给哪个参数；None＝这家不收条数
    pub accounts: Vec<ProviderAccount>,    // 有序；匿名账号是 reference 缺席的那一行
}
pub enum SearchConfiguration {
    Default,                                                      // 缺省的那一家
    Custom { selected: ServerLabel, suppliers: Vec<SearchSupplier> },
    Off,                                                          // 这里不提供 web_search
}
```

- **三臂穷尽，没有 `enabled` 开关**：「取第一个启用的」要读者去猜两家都启用时谁说了算；`selected` 直接点名用哪一家，`Off` 直接说不用。`Default` 是一个被说出来的值，不是缺席：一层写 `Default` 就盖住更远一级的 `Custom`。
- **本模块只持有形状**：`Default` 指哪一家、`selected` 怎么找到它、什么配置被拒、写在哪几级，全住 `config_layers::search`（`crates/city/spec/ConfigLayers.lean` §8-4c，city D24）。缺省那一家的地址与参数因此只有那一处声明，kernel 里没有它的副本。
- **整值上梯、Run 起点冻结**：理由同 `mcp`——`web_search` 在 Run 起点进工具表，它的参数表由这一家的映射决定，Run 内换一家既自毁缓存又换掉了数据接收方而没有人审过。
- **凭据只以引用出现**：`accounts` 的每一行与模型端点的账号是同一个类型 `ProviderAccount`（`crates/kernel/spec/Event/Record.lean`），Key 原文只在 vault；这个值不进 Ledger 载荷，它住 `CONFIG.toml`。

**信任的连接器（config）**：`SandboxLimits.trusted: Vec<ServerLabel>`，缺省空表；`SandboxLimits::trusts(&ServerLabel)` 是这张表的唯一读者。它回答 `gate::undoable` 的问题——这楼层准哪个连接器伸到运行中的运行这座城的机器上。口径与 `mounts` 同形：整值上梯、Run 起点冻结、在解析点拒。缺省空表的意思是「这楼层不准任何连接器碰运行这座城的机器」，而一条写在 `CONFIG.toml` 里的信任是人在看得见整张表时做的决定，比在模型等着时做的决定更值得信。

**环境变量透传（config）**：`SandboxLimits.env_passthrough: Vec<EnvVarName>`，缺省空表；`EnvVarName` 是本模块的值类型（形状 2），唯一构造点 `EnvVarName::parse`。

- **为什么需要它**：只留 PATH 的环境里，rustc 的 MSVC 链接器找不到 `vswhere.exe` 所在的那组环境变量，于是退回 PATH 上第一个 `link.exe`（Git 附带的 coreutils 那个），链接失败；resident 在城里跑 `cargo build` 需要楼把这几个名字透传进去。
- **解法不是加长 `ENV_ALLOWLIST`**：那份常量旁边的注释正是为阻止这件事而写的——**子进程继承到的东西，它忘不掉**。加长它会让每一栋楼、每一次 `exec` 都多继承一份没人审过的东西。改成由**楼自己逐名声明**：说得出名字的那几个才进得去。
- **口径与 `mounts` 逐条同形**：整值上梯（一层说到 `[sandbox]` 就说全部）、同一条冻结理由（可达范围恒不在 Run 内变宽）、同一个「在解析点拒」的位置。`mounts` 拒保留区，`env_passthrough` 拒凭据形状的名字。
- **`EnvVarName::parse` 拒四类**：空名；含 `=`（那是赋值号，不是名字的一部分）；含 NUL 或控制字符；以及 `secret::names_a_credential` 判为凭据形状的名字（`consts_policy::CREDENTIAL_NAME_MARKERS`，子串命中即判，大小写不敏感）。**拒在解析点而不在使用点**：一个名字一旦递给子进程就收不回来，所以判定必须发生在配置被读进来的那一刻。
- **凭据形状的名字为何由 `kernel::secret` 判**：这座城已经有一处「什么东西看起来像凭据」的权威，名字这一面长在同一处而不是第二处。依据是标记词子串（`SECRET`／`TOKEN`／`KEY`／`PASSWORD`／`PASSWD`／`CREDENTIAL`／`AUTH`／`SESSION`／`COOKIE`／`PRIVATE`／`SIGNATURE`），**故意宁滥勿缺**：`KEYBOARD` 一并被拒是可接受的代价，因为拒绝带着三段式的替代路径，而漏掉一个 `AWS_SECRET_ACCESS_KEY` 不带任何提示。

**上下文提醒的第二道阈值（config）**：`SecondThreshold`（形状 2 值）回答「上下文提醒第二道阈值响在窗口的哪一格」，是整数百分比，唯一构造点 `SecondThreshold::parse`，读数是 `percent()`，合法域 31–90（含端点，三个端点数落 `consts_policy`；下端为何是 31 见 D19）。域外的值在解析点拒（`E_INVALID_ARGS`，动作/主体/码/恢复语四段由类型给出，恢复语带合法域），**不钳位**——一个写下 30 的人必须被告知这不被接受，而不是被悄悄改成 31。`FrozenConfig.second_threshold: Option<SecondThreshold>`，缺省 `None`＝没有一层说话，读它的地方（`runtime::reminder`）取 `CTX_REMINDER_SECOND_DEFAULT`。口径与 `trusted`／`mounts` 同形：整值上梯、Run 起点冻结、解析点拒。**冻结的理由是提醒自己的记账**：第二道阈值决定一个 run 何时被告知该写 handoff，而「每道阈值一跑恰响一次」不能取决于有人在哪一刻改了文件。文件与线上的边界同样只过这一个构造点：`TryFrom<u64>`（serde 的 `try_from`）直接委派 `parse`，`From<SecondThreshold> for u64` 只取内层那一个数——域的判定在整棵库里因此只有一处。

- **为何必须冻结**：provider 官方文档记明「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」——强度是缓存前缀的一部分。Run 内可变的强度＝Run 内自毁的缓存，故它落 `FrozenConfig` 而非 `LiveConfig`；设置面改它对**下一个 Run** 生效。
- `None` 与 `Some(Effort::None)` 是两件事：前者不写字段（provider 缺省，Anthropic 新模型即 adaptive thinking），后者显式关闭思考。不用 `Effort::None` 兼任「未声明」，否则「没设过」与「设成关」在类型上不可分辨。
-/

/-! D7 定规：上下文提醒第二道阈值的缺省与合法域只有一个家，可选覆盖走既有配置梯子

这一条是人定的。

**决定**：第二道阈值缺省 65% 与合法域 31–90（含端点）的唯一家是 `consts_policy`（`CTX_REMINDER_SECOND_DEFAULT`／`CTX_REMINDER_SECOND_MIN`／`CTX_REMINDER_SECOND_MAX`）；可选覆盖走 `config_layers` 既有三层梯子（`LayeredValue` 整值上梯、Run 起点冻结、解析点拒），域外在解析点拒、拒因带合法域、不钳位。第一道阈值不在此列，它不可调（它的值与第二道合法域下端的关系见 D19）。

**理由**：写 handoff 的紧急度因工作方式而异，阈值应由人定。域的下限紧贴第一道阈值之上（D19）。域的上限来自算术——阈值越晚，「剩余预算仍够写 handoff 并 `succeed`」这句提醒越可能说不出口；让这句话从算术派生是收窄它的正解，在那之前 90 是宽容的上端。

**被否**：①第四种配置机制（浏览器偏好存储）——同一个值两个家，浏览器副本会越过文件成为第二个权威（client/Spec.lean §4-28 同一条理）；②域外钳位——钳位把一个写错的值变成一个没人被告知的决定。

**重开参数**：出现「提醒到得太晚、handoff 写不下」的实际数据时，重开的是上限 90，不是本定规。
-/

/-! D19 定规：第一道上下文提醒在窗口的 30%，第二道的合法域从 31% 起

这一条是人定的。

**决定**：`CTX_REMINDER_FIRST_PERCENT = 30`；`CTX_REMINDER_SECOND_MIN = 31`，上端 90 与缺省 65 不动。两道阈值因此恒不重合：第二道最早也比第一道晚一个百分点。

**理由**：第一道只报用量，响得太早就是每次 run 都来的噪声，30% 是人定的那一格。第二道的下端跟着第一道走而不是各自定一个数：两道落在同一格时，「一跳越过两道只响高的那一道」（`crates/runtime/Spec.lean` §8-34）会让第一道永远不响，可调范围里就多了一个让梯子少一级的值。所以下端在 `consts_policy` 里由第一道推出（`CTX_REMINDER_FIRST_PERCENT` 加一），而不是另写一个 31：两个数之间的关系只有这一处定义。

**被否**：①下端保持 30——第一道改成 30 之后，写下 30 的城得到一把只有一级的梯子，而没有人被告知；②把第一道也做成可调——一个数一个意思，第一道是所有城共用的读数，可调之后同一句「[context] N% 用掉」在不同的城里是不同的提示等级。

**重开参数**：人改第一道的值时，第二道的下端随之移动，无须另改；要让两道之间隔开不止一格，重开的是本定规。
-/

/-!
### container 的输入值（config）

`crates/kernel/src/config/container.rs` 定义 `ContainerLimits { image: ContainerImage,
user: NonZeroU32, cpu_millis: NonZeroU32, memory_bytes: NonZeroU64, pids: NonZeroU32 }`。
没有默认镜像或默认额度，整份值由 User 指定，限额只容许非零整数。
`ContainerImage::parse` 只收本地 image ID `sha256:` 后接 64 位小写十六进制；serde 也只过
同一构造点，标签、仓库名、空白与 CLI 选项以 E_CONFIG_INVALID 拒绝。这个值不保存主机路径。

D50 `SandboxLimits.container: Option<ContainerLimits>` 缺省缺席，serde 省略缺席值以兼容原来的
冻结记录与配置写回。`[sandbox.container]` 声明整份显式输入即选择 container；缺一个值或
零额度即拒。更近的 `[sandbox]` 没有 container 即取消远层 container，与既有整值覆盖一致。
拒绝另一条 runtime 环境变量配置，因为那会在冻结配置之外改变执行边界。
后端契约与接线边界见 `crates/runtime/spec/Tools/Exec/Container.lean`。
-/

/-! D52 冻结的显式沙箱臂

`kernel::SandboxArm` 是 none／copied_tree／native／container／python 的唯一枚举定义；
wire 重导出这个值，避免配置与 doctor 的同名选项各定义一份。`SandboxLimits.arm` 缺席
保留平台缺省；arm 缺席但 container 有值时选择 Container，兼容显式容器子表。
显式 Container 必须有完整 container；别的显式臂同时带 container 被拒，不能忽略输入。
Native 不可给时 E_SANDBOX_DENIED，不退到 copied_tree；CopiedTree 明确用副本；None
明确请求 host，仍受 Create 写限制；Python 臂拒绝 sandbox program/shell，python guest
沿用既有 WASI 接口。选择在 Run 起点冻结，在工具入 catalogue 之前接入并描述其实际保证。
-/
