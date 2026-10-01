-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::tool

规定 `kernel::tool`（`crates/kernel/src/tool.rs` 与 `crates/kernel/src/tool/` 下的 `writes`、`conformance`）：工具端口、效果与工具的元数据。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-23 kernel::tool（缝清单文件）

```rust
pub struct ToolName(String);        // 非空；ascii 小写/数字/下划线（进 catalog 与事件的名）
impl ToolName {
    pub const BROWSER: &'static str = "browser";            // 内建浏览器工具的名：ToolMeta、楼的准入规则与账本夹具共用的唯一拼写
    pub const USER_BROWSER: &'static str = "usersbrowser";  // 驱使人自己开着的浏览器的工具
    pub const EXEC: &'static str = "exec";                  // 三臂执行工具的名：discard 预报、命令计数与 sieve 三处按名路由，拼写只此一家
}
pub struct ServerLabel(String);     // 非空；ascii 小写/数字，恒不含下划线（见下）
pub struct TimeoutMs(u64);          // 声明即承诺可协作取消
pub enum Effect { Read, Write { domain: Address }, Egress,
                                    Connector { label: ServerLabel }, Spawn, Govern,
                                    AttachUserBrowser { address: Option<String> }, Spend }   // 决定过哪道门
// Spawn：起第二个 Agent。不归 Read——一次派生能花多少、能碰什么，调用方自己的任何一道门都不管；
// 管它的是深度规则：`kernel::gate::spawn` 判一层深，被派生的位置再派生恒 Deny。
// Govern：改写一个 scope 被判的规则。刻意不归 Write——保留子树在每个写域之外，写门本就会拒；
// 而它拒的理由是「一个 run 不得改写审判它自己的规则」，故效果层对这一臂恒拒，规则由人改 TOML。
// AttachUserBrowser：附着到人自己开着的浏览器。地址由登记固定而不是每次调用命名（同 Connector 的理由）；
// `None` 是人启用了工具却没说地址，`gate::attach` 据此答 Ask 而不是猜。
pub enum GateSubject { Area(Address), Room(Address), Scope(String), Host(String), None }
// Tool::subject 的返回：一条调用说的是什么，由工具自己的文法读出来。
// `None` 是文法读完成参数后的答案「这条调用没有主体」，不是遗漏：Egress 与
// AttachUserBrowser 收到它就按 `EgressTarget::Loopback` 判，密钥扫描照跑，
// 因为跳过门会让一条凭据静悄悄进页面；Govern 收到它即拒，因为改规则的调用
// 必须自己说出改哪个 scope，替它编一个等于把工具的错误说成事实。
// 文法读不出的调用返回 `Err`，bench 原样拒收。
pub enum Temporal { Timeless, Timestamped }
pub enum CostTier { Free, Light, Heavy }        // 三档
pub enum RenderIntent { Generic, Terminal, Diff { locations: Vec<Address> } }
                                    // meta 级声明用空 locations；逐调用的 locations 是 args 的纯函数（工具侧）
pub struct ToolMeta { pub name: ToolName, pub disclosure: String, pub params: Payload,
                      pub effect: Effect, pub cost_tier: CostTier, pub timeout: Option<TimeoutMs>,
                      pub render: RenderIntent, pub temporal: Temporal }   // 八字段，缺一不可
pub struct ToolCall { pub id: String, pub name: ToolName, pub args: Payload }
                                    // id：tool_use↔tool_result 对号是两 Dialect 的 wire 硬性要求；
                                    // 脚本适配器用确定性合成 id（call-<n>）
impl ToolCall {
    /// The bytes that say what this call does: the name, then the
    /// arguments. `IdemKey::derive` takes them as `action_canonical`;
    /// `id` stays out, because two calls differing only by wire id are
    /// the same action.
    pub fn action(&self) -> Result<Vec<u8>, AxError>;
}

pub struct ToolOutcome { pub result: Payload, #[serde(default)] pub attachments: Vec<ImageRef> }

pub trait Tool: Send + Sync {
    fn meta(&self) -> &ToolMeta;
    /// Fail-closed identity: a call whose name differs from meta().name
    /// must return E_INVALID_ARGS, never route silently.
    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError>;
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError>;   // 默认 `GateSubject::None`
    fn writes(&self, call: &ToolCall) -> Writes;   // 默认按 `meta().effect`：`Read` → `Nothing`，其余 → `Domain`
}
pub enum Writes { Nothing, Paths(Vec<Address>), Domain }
// Tool::writes 的返回：一条跑完的调用可能写了城里树上的哪些路径，由工具自己的文法读出来。
// `Paths` 只给确知自己写了哪些文件的工具（`edit` 答它的 `path`）；说不清的（`exec`、协作桌、改规则）答 `Domain`，
// checkpoint 于是扫整个写域。默认实现不猜：只有声明 `Effect::Read` 的工具答 `Nothing`。
// `Writes::and` 合并两条答案：`Domain` 吸收一切，`Nothing` 是单位元，两组 `Paths` 取并集。
#[cfg(feature = "conformance")]
pub fn assert_tool_conformance<T: Tool>(tool: &mut T);   // 八字段完备＋name 文法＋错名调用拒收

pub enum ExecArm { Program { path: String, args: Vec<String> }, Python { code: String }, Shell { text: String } }
                                    // 三臂恒三（L0 冻结面），故穷尽不标 non_exhaustive；discard::forecast 的入参
```

- **`invoke` 取 `&self`，trait 要求 `Send + Sync`。** 一波里开头连续的只读调用由 `Turn::execute_concurrent` 同时起跑（`crates/runtime/Spec.lean` §8-3），同一张工作台上的工具因此会被几个线程同时借用；`&self` 加 `Sync` 让「这件工具能被并行调用」由类型回答，而不是由调用方记住。有内部状态的工具把状态放在自己的锁后面（`Mutex`），锁只罩住那份状态，不罩整次调用。**被否**：①保留 `&mut self`，由工作台给每件工具套一把锁——同名的两条只读调用（两次 `read`）会在这把锁上排队，并行只剩不同名的调用；②每次调用克隆一件工具——持有子进程、连接或目录的工具克隆不出同一件东西。
- **`params` 复用 `Payload`**：键序 BTreeMap＋拒浮点白拿；schema 约定属各工具的实现。
- **`ToolName::EXEC`**：`exec` 的唯一拼写；discard 预报、命令计数与 sieve、citysim 的执行器都读这个常量。`BROWSER` 同理。
- **conformance 三断言**：①meta 八字段形状合法（name 文法、disclosure 非空）；②错名调用拒收（E_INVALID_ARGS）；③拒收后工具仍可用（再次正确调用不受污染）。
- ExecArm 住本模块而非 runtime：`discard::forecast` 读它，而 kernel 不能依赖 runtime；工具面参数枚举属 tool 面（「可枚举的必用枚举」）。
- **`Effect::Connector { label }`**：目的地由**登记**而非逐调用参数定的那一类出站。`Egress` 的主语是一次调用（去哪台主机写在 args 里），`Connector` 的主语是一件工具（它恒只通往那一台 server）。**两者不得合并**：合并后要么让模型去填一个城自己已经知道的 `host`（一个可以填错的事实），要么让出站门拿不到目标而无法判定。发现它的时刻就是接线的时刻：`Effect::Egress` 写下时没有调用方，而第一次真调用当场拿到 `E_INVALID_ARGS: declares Egress but named no host`。
- **`ToolCall::action` 住本模块**：`IdemKey::derive` 的第三个入参由什么构成，§8-6 写明「属工具面」，所以由 `ToolCall` 自己回答，因为**它就是那个动作**；`bin::assembly` 与 `citysim::executor` 都调它。两个调用方各写一遍时两遍会漂移：只取 name 的那一遍让一波之内两次同名调用得同一把键（一波共用一个 `t`），`ToolBench::invoke` 的 dedup 把第二次判为 `Duplicate`，模型只能读作自己出错。`id` 不进动作字节：两次只有 wire id 不同的调用是同一个动作。
- **`action` 上报序列化失败而不吞掉它**：取默认值会产空串，让两次参数不同的调用得同一把键——正是本条要消灭的那种碰撞。`Payload` 拒浮点且键恒为字符串，故这条失败臂今天不可达；但「不可达所以取默认值」与「不可达所以据实上报」之间，只有后者在它变得可达那天仍然是对的。
- **位次仍归调用方**：`seq` 说的是「这次调用坐在这一跑的第几位」，只有驱动那一跑的一方知道。把它一并收进 `ToolBench` 会让键在一次驱动内恒不重复，于是 dedup 永不触发，`dedup_runs_before_the_side_effect`（同一把键调两次、断言第二次不落地）连同它守的那条不变量一起变得写不出来。**收窄接口不值这个价**，故本模块只给动作字节；citysim 与 `bin::assembly` 用同形的每跑计数器作位次，是因为钟读数当位次逐字违反确定性第 7 条（「never from a clock」），而不是因为位次该归本模块。
- **`ServerLabel` 住本模块而非 agent_protocols**：它是一台 MCP server 在城里的名字，也是它每件工具名的第一段（`{label}_{tool}`），故它的文法就是 `ToolName` 的文法减下划线——写在两个 crate 里就是一条规则两个权威。**减下划线是判定而非口味**：允许它会让 `apps_foo_bar` 同时读作两种拆法，而这个名字要路由一次调用。迁入后 `city::config_layers` 在文件边界就能解析它（city 只见 kernel），于是「非法标签」在 Run 存在之前就不可表示。
- **`GateSubject::None` 是一条判定而不是遗漏**：`None` 说的是「工具的 grammar 读完成参数，这条调用没有主体」。门收到它就按调用真正有的东西判：`Effect::Egress` 与 `Effect::AttachUserBrowser` 按 `EgressTarget::Loopback` 过密钥扫描（未指名去向的字节留在运行中的机器上，扫描照跑，因为跳过门曾让一条凭据静悄悄进页面），`Effect::Write` 的收窄没有更窄的区域可问（声明的 domain 已判过），`Effect::Govern` 则拒收（改规则的调用必须自己说出改哪个 scope，替它编一个等于把工具的错误说成事实）。**被否**：`None` 即跳过门——浏览器工具的非导航调用（`snapshot`、`act` 等）从此不过扫描；`None` 即拒——同一批调用全被拦下，而工具拿不出页面主机：`subject` 只读调用参数，浏览器工具不存当前页地址。文法读不出参数的调用返回 `Err`，bench 原样拒收，两种失败因此在类型上可分辨。
- **`Area`／`Room` 今天没有生产者，`Scope` 由两件治理工具答出**：三种主体各有一个消费者（Write 的收窄用 Area／Room，Govern 用 Scope）。`rules`／`city` 答各自治理的 scope（city-SPEC §8-36），Govern 的拒词因此说出那个 scope；能回答 Area／Room 的工具是 `edit`／`exec`／`archive`／`claim`／`goal`／`pr`／`signal`（各自的写入目标），它们的 `subject` 仍取 trait 默认，故收窄今天不生效。`Effect::Govern` 的 `None` 被拒收而不是回落到 run 地址。每个写工具补上自己的解析即闭合这一段（M-17）。
-/

/-!
### 8-52 一次工具调用产出的图

`ToolOutcome` 多一个字段 `attachments: Vec<ImageRef>`，`#[serde(default)]`，旧历史读成空表。

**为什么不放进 `result` 里**：`result` 是给模型读的文本载荷，一个埋在 JSON 里的 `cas:` 定位符对模型永远只是一串字。要让模型**看见**这张图，它必须成为 `ContentBlock::ToolResult.attachments` 的一员——那是 `ContentBlock::Image` 备好的位置，而 `runtime::turn::wave` 是唯一一处把 `ToolOutcome` 变成 `ContentBlock` 的地方，于是这个字段是那条路上唯一缺的一段。

**字节不在这里**：`ImageRef` 携定位符与两个整数边长，字节住 `storage::cas`，出线前的最后一刻才由 `gateway::endpoint` 取出来编码。账本因此仍是一份人能读的文件。

**唯一的生产者是 `browser` 工具的 `screenshot`**（`bin::browser_tool`）；其余每一个工具显式写空表，因为「没有图」是一句要说出口的话，不是一个可以省略的默认。
-/
