# accounting-SPEC.md

> crate：`accounting`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。

## 1 需求分解

`accounting` 是 accounting thread 的家：城的唯一写者，以及它向外伸手时经过的端口。它存在的理由是 ARCHITECTURE.md §11 记下的 V6 缺口——`RunWorker` 自己造模型适配器而不是接收一个，所以 citysim 的脚本只能复现一次 run，复现不了一次 dispatch。把写者移进一个只经端口向外伸手的 crate，端口的第二实现就能从外面把它驱动起来。

| 模块 | 这个模块回答的问题 | §8 |
|---|---|---|
| `models` | 一次 run 拿什么模型适配器去说话 | 8-1 |
| `connectors` | 一栋楼配置里写的 MCP server，怎样连上并变成工具 | 8-2 |
| `clock` | 现在几点 | 8-3 |
| `machine` | city 所在的主机上有哪些工具，以及装上一个人同意过的那一个 | 8-4 |
| `effect` | 一张桌子留下的效应，怎样先成为账本行、再成为这座城 | 8-5 |
| `plan_view` | 每栋楼的计划，折叠记录而只在记录点到它时重读 | 8-6 |
| `home` | 这个人的家目录在哪，本产品在它下面放什么 | 8-7 |
| `person` | 这个人自己定下的那一层：偏好与核心线程的档位 | 8-8 |
| `held_vault` | 一个锁着的 vault 怎样变成解析器，锁中毒时怎样拒绝 | 8-9 |
| `toolkit_broker` | 一个外部应用的 broker 钥匙登记在哪，这座城对它是谁 | 8-9 |

worker 的两个读写面 `effect` 与 `plan_view`、以及 worker 与 `views` 共用的四个叶子模块 `home`、`person`、`held_vault`、`toolkit_broker` 已在本 crate；`RunWorker` 的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例与 `views` 仍在 `crates/sprawling/src/assembly` 与 `crates/sprawling/src/views`，按 §7 的归属表搬。

## 2 验收标准

三条断言，都在 `crates/sprawling/tests/model_factory.rs`，都驱动一个接收了脚本 `ModelFactory` 的 `RunWorker`：

| 测试 | 它钉住的事 |
|---|---|
| `a_dispatch_reaches_the_model_the_worker_was_handed` | run 的模型走端口。端点指向一个拒绝连接的 loopback 端口，所以只要 worker 还自己造适配器，这次 run 就碰不到脚本模型，历史里也就没有脚本写下的那句回答。 |
| `an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone` | 没有 session 的 dispatch 按规则从任务的词里取房间名（sprawling-SPEC.md 8-86），不做命名调用：工厂只被问到 run 的主模型，run 在规则给出的房间里开始。 |
| `a_confidential_building_refuses_before_the_factory_is_asked` | 机密楼的拒绝在 worker 的选择里，不在工厂里：端点不是 loopback 地址时 dispatch 以 `GateDenied` 被拒，脚本工厂一次也没被问到。换掉工厂不能绕开机密。 |

第四条在 `crates/sprawling/tests/connectors.rs`：`a_run_is_offered_the_tools_the_worker_was_handed`。楼的配置写了一个 MCP server，它的命令在任何主机上都不存在；worker 接收了一个脚本 `Connectors`，它给出一个工具。只要 worker 还自己启动 server，这个 server 就起不来，模型收到的工具表里也就没有那个工具。

第五条在 `crates/sprawling/tests/clock.rs`：`a_worker_stamps_its_lines_with_the_clock_it_was_handed`。worker 接收一个停在固定时刻的脚本 `Clock` 之后写下的每一行，`t` 都是那个时刻；只要还有一个写点自己读墙钟，这一行的 `t` 就是现在的时间。

第六条在 `crates/sprawling/tests/machine.rs`：`a_refresh_counts_the_items_the_machine_it_was_handed_answered`。worker 接收一个脚本 `Machine`，它的回答只有一个条目；`DoctorRefresh` 之后 worker 在诊断里报的条目数是 1。只要 worker 还自己去问主机，报的就是需求表的全部条目数。测试从 `DoctorRefresh` 进而不从 `DoctorInstall` 进：对着一个还自己动手的 worker，后者会在宿主上真的启动包管理器。

## 3 假设与歧义

- `RunWorker` 与它的六个对象、全部用例还在 `crates/sprawling/src/assembly`，`views` 还在 `crates/sprawling/src/views`，所以 citysim（不依赖 `sprawling`）仍驱动不了一次 dispatch。归属由 §7 的表和 §12-9 至 §12-12 定下；还没做的按这个次序：
  1. 还缺的端口。`revealing`（`RunWorker.reveal`）、`monitor::memory`（`DrivingPool` 的 `read_memory`）、`monitor::volume`（`RunWorker.read_volume`）、`release`（`Views.registry`）、`browser_tool`（`RunWorker.browsers`）与需求表的查法（`RunWorker.recipe_for`）已经是交进来的 `fn` 指针；还直接碰 `bin` 的只剩 `workbench::engine` 读的 `doctor::host` 与 `Presence`。
  2. `views` 搬进本 crate。它的测试里有一部分造一个 worker（`views/tests.rs` 经 `crate::assembly` 的 fixture，`document`、`listing`、`skills` 的测试调 `init_city`），它们要么随 worker 搬、要么先留在 `sprawling` 经 `views` 的公开面测。
  3. `RunWorker`、`relay`、`pool`、`desk`、`drive_run` 与六个对象、全部用例在一次改动里搬（§12-11）；`genesis`、`listening`、`attending`、`chain_watch` 与生产适配器留在装配根（§12-12）。
  4. citysim 经本 crate 的端口驱动一次 dispatch，ARCHITECTURE.md §11 的 V6 缺口随之关闭。
  5. 每搬走一个模块，它在 sprawling-SPEC.md 里的那一节就搬进本 SPEC（8-7、8-8、8-9 就是这样来的）。
- `views::mcp_health` 自己用 `protocol::McpLink` 启动一个 MCP server 去问它的健康，不经 `Connectors`。未定的是这次读要不要也经端口：`views` 搬进本 crate 时它照原样搬（`protocol` 本来就是本 crate 的依赖）；能定下它的证据是一个脚本场景需不需要回答 MCP 健康查询。

## 4 现状分析

本 crate 现有四个端口。`ModelFactory` 的生产适配器在装配根（`bin::assembly::models`），第二实现在 `crates/sprawling/tests/model_factory.rs`；`Connectors` 的生产适配器是 `bin::assembly::mcp::Residents`，第二实现在 `crates/sprawling/tests/connectors.rs`；`Clock` 的生产适配器是 `bin::assembly::SystemClock`，第二实现在 `crates/sprawling/tests/clock.rs`；`Machine` 的生产适配器是 `bin::doctor::ThisMachine`，第二实现在 `crates/sprawling/tests/machine.rs`。

## 5 权威信源

ARCHITECTURE.md §3（依赖律与 `depmap`）、§4（端口表）、§11（V6 缺口）；`gateway::adapter_for`（一个 `Chosen` 怎样变成一个适配器）。

## 6 命名统一

ModelFactory｜Connectors｜Clock｜Machine｜Recipe｜Runnable｜accounting thread｜Chosen｜Redemption。「适配器」专指 `kernel::Model` 的一个实现；「端口」专指本 crate 声明、外层实现的 trait。

## 7 模块边界

- 怎样按 endpoint、dialect、凭据造出一个适配器，归 `gateway::adapter_for`：本 crate 只声明「造一个」这个动作。
- 挑哪个模型（`EndpointBook::select`）与何时续期凭据，归 `RunWorker`：端口拿到的是已经选好的 `Chosen` 与已经兑换好的 `Redemption`。
- 把生产适配器接到 worker 上，归装配根 `bin::assembly`。
- MCP 的生命周期（`initialize`、`notifications/initialized`、`tools/list`）怎样说，归 `protocol`；一个工具能不能在机密楼里存在，归 `protocol::McpTool::new`。端口只声明「连上一个 server，交回它的工具」。
- 机密楼根本不启动 server，这一步在 worker 的 `mcp_tools` 里、端口被问到之前。
- 主机上有什么、每一项怎样判定、怎样折叠成一页，归 `bin::doctor`（city 所在主机有什么，它是唯一权威）；一条安装配方能不能跑，归 `Recipe::command`；worker 只决定一个名字在不在需求表里、这个平台有没有配方。

**worker 用到的每个 `bin` 模块归哪一边。** 规则是 §12-9：worker 的决定与读面搬进本 crate；通往主机、网络或终端的做法留在 `sprawling`，经一个端口交进来。

| `bin` 里的东西 | 归属 | worker 或 `views` 用它做什么 | 依据 |
|---|---|---|---|
| `views` | 搬进本 crate | `Views`、`Published`、`Governance`、`pursued`、`snapshot::start` | worker 的读面；`Governance` 由读侧拥有，写侧从那里取用（sprawling-SPEC.md 8-92） |
| `home` | 搬进本 crate | 阅览室与 `views::skills` 取这个人的家目录 | 只读一个环境变量、拼路径，不启动任何东西；`person` 与 `views` 都从它取路径 |
| `person` | 搬进本 crate | `PutPreferences` 写、`Preferences` 查询读 | 人的那一层是一份文件，读写它和读写城的文件同类，不是主机的能力 |
| `serving::standing::CorePriority` | 随 `person` 搬进本 crate | 偏好里核心线程抬不抬高的那个值 | 它是 `person` 读出来的值；真去抬高线程的 `raise_this_thread` 留在 `serving` |
| `held_vault` | 搬进本 crate | 把一个锁着的 vault 变成解析器，锁中毒时的拒绝 | 纯函数，只碰已经打开的 vault |
| `toolkit_broker` | 搬进本 crate | 一个外部应用的 broker 钥匙登记在哪 | 纯函数，`views::toolkits` 与连接动作读同一组事实 |
| `doctor`（`REQUIREMENTS`、`Platform`、`host`、`Presence`、`PATIENCE`、`ThisMachine`） | 经端口：看与装经 `Machine`，需求表的查法经 `RunWorker.recipe_for`（sprawling-SPEC.md 中 `doctor_install` 那一节） | 需求表查找、执行引擎的路径 | 主机上有什么，`bin::doctor` 是唯一权威（本节上文） |
| `monitor::memory::read`、`monitor::volume::read` | 经端口：`DrivingPool` 的 `read_memory` 与 `RunWorker` 的 `read_volume`，都是 `fn` 指针（sprawling-SPEC.md 8-46-3、8-94） | 新工作进门时读内存与卷的余量 | 读主机的计数器；`read_volume` 已经这样交进来 |
| `serving::door::random_token` | 随 `credentials` 搬进本 crate | OAuth 登录的 verifier 与 state | 它的熵必须不可预测：一个第三方能预测的 verifier 就是一个第三方能完成的登录，所以没有哪个脚本场景可以换掉它，端口在这里只会开一个让它变得可预测的门；它经 `getrandom` 这个安全接口取熵，不启动任何东西 |
| `revealing` | 经端口：`RunWorker` 的 `reveal` 字段，一个 `fn` 指针（sprawling-SPEC.md 8-60） | `Reveal` 在主机的文件管理器里打开一个地址 | 启动主机的一个程序 |
| `browser_tool` | 经端口：`RunWorker::with_browsers` 交进来的 `fn` 指针（sprawling-SPEC.md 8-45-2） | 按楼的规则给 run 的浏览器工具 | 启动浏览器，经 BiDi 说话 |
| `release` | 经端口：`Views.registry`，一个由 `views::served` 放进来的 `fn` 指针（sprawling-SPEC.md 中 `Views.machine` 旁的那一条） | `views` 回答 `NewestRelease` 查询 | 向 npm 注册表发请求 |
| `console` | 留在装配根 | — | 只有 `listening` 用它；它是终端，不是 worker |
| `serving` 的其余部分（`folding`、`output_ring`、`Serving`、`open_vault`） | 留在装配根 | — | 只有 `attending`、`listening` 与 `genesis` 用它们 |
| `assembly` 的 `listening`、`attending`、`chain_watch`、`genesis`、`models`、`mcp::Residents`、`SystemClock` | 留在装配根 | — | 起线程、绑端口、造城的目录、生产适配器：§12-1 与 §12-12 |

## 8 接口先行

### 8-1 accounting::models（形状 3 端口）

```rust
pub trait ModelFactory {
    /// # Errors
    /// Whatever building the adapter refuses, such as a malformed
    /// endpoint. Confidentiality is not decided here: the worker has
    /// already refused a remote model under the building's policy.
    fn build(
        &self,
        chosen: &gateway::Chosen<'_>,
        redemption: gateway::Redemption,
    ) -> Result<Box<dyn kernel::Model + Send>, AxError>;
}
```

```rust
// bin::assembly::models（形状 4 适配器）
pub(crate) struct GatewayModels;          // 生产：dialect 头 + gateway::adapter_for
impl RunWorker {
    pub fn with_models(self, models: Box<dyn accounting::ModelFactory + Send>) -> RunWorker;
}
```

- **失败**：原样传 `gateway::adapter_for` 的 `AxError`（它自带 action、subject、code 与 recovery）；端口不另造错误码。
- **机密不归端口**：端口被问到之前，worker 已经在楼的 policy 下调过 `EndpointBook::select`；机密楼配上不是 loopback 地址的端点，在那里就以 `GateDenied` 被拒，端口根本不会被调用。所以任何一个实现——生产的也好，脚本的也好——都放不宽这条规则。`gateway` 的 `Endpoint` 在调用时按请求的 policy 再拒一次，那是适配器自己的防线，不是这条规则的权威。
- **一个调用点**：dispatch 的同意阶段（`agreeing`）经 `RunWorker.models` 造适配器，再套上 keep-warm 门（sprawling-SPEC.md 8-93）。房间名按规则取，不调模型，所以没有第二个调用点。dialect 头只在生产适配器里算一次。
- **固定值**：`RunWorker::new` 与 `over` 装上 `GatewayModels`；`with_models` 是唯一换掉它的门。
- **一致性套件**：端口的断言就是 §2 那条测试——拿到的适配器必须就是被调用的那一个。

### 8-2 accounting::connectors（形状 3 端口）

```rust
pub trait Connectors {
    /// # Errors
    /// Whatever starting the server, its handshake or its listing
    /// refuses.
    fn connect(
        &mut self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<protocol::McpTool>, Reached), AxError>;
}

pub enum Reached {
    Connected(protocol::Handshake),   // 这次调用启动或打开、并握过手
    Resident,                         // 早先一次调用连上、仍在运行
}
```

```rust
// bin::assembly::mcp（形状 4 适配器）
#[derive(Default)]
pub(crate) struct Residents;              // 生产：McpLink::open + protocol::handshake + tools/list，连接在 run 之间保持（sprawling-SPEC.md 8-4）
impl RunWorker {
    pub fn with_connectors(self, connectors: Box<dyn accounting::Connectors + Send>) -> RunWorker;
}
```

- **失败**：原样传 `McpLink::open`、`protocol::handshake` 与 `protocol::tools_from` 的 `AxError`。worker 把失败写进 diagnostics、把这个 server 留在外面，run 照常开始；端口不另造错误码。
- **`confidential` 原样传给 `McpTool::new`**：那是工具层的权威。worker 在机密楼里一个 server 都不启动，所以生产路径上它总是 `false`；它仍在签名里，是为了任何实现都不能造出一个绕过工具层拒绝的工具。
- **端口有状态**：`connect` 取 `&mut self`，因为生产适配器把连上的 server 按声明与 run root 留在表里，下一次 dispatch 直接拿它的工具；子进程已经退出的那一行在这里被丢掉、重新启动。
- **固定值**：`RunWorker::new` 与 `over` 装上 `Residents::default()`；`with_connectors` 是唯一换掉它的门。
- **依赖**：本 crate 因此依赖 `protocol`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-3 accounting::clock（形状 3 端口）

```rust
pub trait Clock {
    /// # Errors
    /// A clock that cannot be read, such as a wall clock set before the
    /// unix epoch.
    fn now(&self) -> Result<kernel::TimeMs, AxError>;
}
```

```rust
// bin::assembly（形状 4 适配器）
pub(crate) struct SystemClock;            // 生产：墙钟，唯一被许可的采样点（clippy.toml disallowed-methods）
impl RunWorker {
    pub fn with_clock(self, clock: Arc<dyn accounting::Clock + Send + Sync>) -> RunWorker;
}
```

- **worker 读的每一个时刻都经 `RunWorker.clock`**：它写的行、它量的耗时、它排的期限。lane 线程从 `DriveContext` 拿到同一个时钟的克隆，所以一个 run 的行与 worker 自己的行读的是同一个钟。worker 调用的自由函数（`captured_until`、`reach_of`）把时刻或时钟当参数收下，不自己采样。
- **worker 之外的两个读点用 `SystemClock`**：`form_city`（city 在任何 worker 存在之前诞生）与 `serving::journal`（诊断日志行的时间，不是城的记录）。`RunWorker::new` 打开 ledger 时 worker 还不存在，也用 `SystemClock`。
- **固定值**：`RunWorker::new` 与 `over` 装上 `SystemClock`；`with_clock` 是唯一换掉它的门。`Send + Sync` 与 `Arc`，是因为 lane 线程与 worker 同时读它。
- **等待也读这个钟**：lane 等 provider 的退避时，一片一片地睡，直到这个钟过了期限。所以一个永远不走的脚本钟，会让遇上退避的 run 一直等下去；脚本要让钟往前走。

### 8-4 accounting::machine（形状 3 端口）

```rust
pub trait Machine {
    /// Asks this machine every question the requirement table holds.
    fn report(&self) -> channels::DoctorAnswer;
    /// # Errors
    /// A program this machine cannot start, one that ended in failure,
    /// and one still running when its patience ran out.
    fn install(&self, item: &str, runnable: &Runnable<'_>) -> Result<(), AxError>;
}

pub enum Recipe {
    Command { program: &'static str, args: &'static [&'static str] },
    Print(&'static str),
    Manual(&'static str),
}
impl Recipe {
    pub fn spelled(&self) -> String;
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` for a printed recipe and a manual one, with
    /// what the person does instead.
    pub fn command(&self, item: &str) -> Result<Runnable<'_>, AxError>;
}

pub struct Runnable<'a> { /* private */ }
impl<'a> Runnable<'a> {
    pub fn program(&self) -> &'a str;
    pub fn args(&self) -> &'a [&'a str];
    pub fn spelled(&self) -> String;
}
```

```rust
// bin::doctor::probe（形状 4 适配器）
impl accounting::Machine for ThisMachine { /* doctor::answer(self) 与 doctor::running::run */ }

// bin::assembly::commanding::machine
impl RunWorker {
    pub fn with_machine(self, machine: Box<dyn accounting::Machine + Send>) -> RunWorker;
}
```

- **只有 `Recipe::command` 造得出 `Runnable`，它证明的是配方的种类，不是许可。** `Runnable` 的构造函数在本 crate 之外不可见，所以持有一个 `Runnable` 只证明它来自一个 `Command` 配方：打印的配方与手动的配方在 `Recipe::command` 被拒。`Recipe::Command` 的字段是 `pub`，任何 crate 都能拼出一个装任意程序的配方，所以挡住表外程序的是 `doctor_install`（`crates/sprawling/src/assembly/commanding/machine.rs`）先在 requirement 表里查这个名字：表里没有的名字以 `InvalidArgs` 被拒，端口根本不会被调用。
- **失败**：`install` 原样传 `bin::doctor::running` 的 `AxError`；`Recipe::command` 的拒绝是 `E_TOOL_UNAVAILABLE`，恢复说明人该做什么。端口不另造错误码。
- **worker 读的两处都经 `RunWorker.machine`**：`doctor_install` 的安装与它之后的重看，以及 `DoctorRefresh` 的 `look_at_this_machine`。需求表里没有的名字与没有配方的平台由 worker 交到的 `recipe_for`（生产是 `bin::doctor::recipe_for`）拒绝，这一步在端口被问到之前。
- **一扇安装的门**：终端的 `sprawling doctor --install` 与 worker 的 `doctor_install` 都经 `accounting::Machine::install` 启动安装程序。`bin::doctor::Machine` 是它的子 trait，只多一个逐项的 `look`，自己不声明 `install`，所以一个装东西的实现只有一处要写，也只有一处能被脚本换掉。
- **固定值**：`RunWorker::new` 与 `over` 装上 `ThisMachine`；`with_machine` 是唯一换掉它的门。
- **依赖**：`report` 交回线上的 `channels::DoctorAnswer`，所以本 crate 依赖 `channels`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-5 accounting::effect：一条效应先成为账本行，再成为这座城（形状 2 值类型）

```rust
// crates/accounting/src/effect.rs —— `architecture.toml` 的 accounting::effect，形状 2（值类型）
pub struct Line { who: String, addr: Address, kind: EventKind, data: Payload }

/// 一张桌子留下的全部效应：它们成为的行，以及行之后才允许发生的变化。
pub struct Landing { lines: Vec<Line>, then: Then }   // 两个字段都是私有的

pub enum Then { Nothing, Deliver(Vec<collab::Signal>), Hold(Vec<GoalEntry>),
                       Roadmap { path: PathBuf, text: String }, Shelf(Vec<Filing>) }

impl Landing {
    pub fn signals(Vec<SignalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub fn goals(Vec<GoalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub fn discards(Vec<Payload>, room: &Address, who: &str) -> Landing;
    pub fn shelf(Vec<ArchiveEffect>, write_root, building, at, room, who) -> Result<Landing, AxError>;
    /// 先走完每一行，再把变化交出去。这是 `Then` 唯一的出口。
    pub fn record(self, &mut impl FnMut(Line) -> Result<(), AxError>) -> Result<Then, AxError>;
}

/// 一跑对共享计划做的事。两种而无第三种：计划是整份写回去的。
pub enum Claims { Landed(Box<Landing>), Stale(Vec<u64>) }
impl Claims { pub fn of(&[ClaimEffect], on_disk: &str, text: String, path, room, who) -> Result<Claims, AxError>; }

// 装配层那一扇门（bin::assembly）：五张桌子都走它，`Then` 的 match 穷尽
impl RunWorker { fn settle(&mut self, RunId, from: &Address, Mode, BudgetCap, Landing) -> Result<(), AxError>; }
```

**原因**：`dispatch_in` 驱动之后有六段 `take_effects()`，每段都在做同一件事——把效应变成账本行，再把它变成状态。这条顺序在三份文件里各写过一次：`docs/glossary.md` 对 Ledger 的定义是「Every effect becomes an EventRecord first」，ARCHITECTURE.md §5 步 4 是「that ordering is the design's load-bearing rule, not a logging preference」，signal 那段自己的注释是「Recorded, then delivered. The queue may only change as a consequence of a line the history already has」。**六段里有两段是反的**：

```rust
write_plan(&plan_path, &text)?;                            // 先改共享计划
for effect in &claim_effects { self.record_for(…)?; }      // 后落账

let entry = city::file_archive(…)?;                        // 先上书架
self.record_for(…, EventKind::AssetArchived, …)?;          // 后落账
```

第二段的注释与它自己的代码相反：「Filed after the drive, like every other effect, **so nothing is on the shelf that the history does not already carry**」。按现行顺序，落账失败就在架上留下一条历史没有的记录，那句话就是假的。计划那一段更重：`roadmap_claimed` 是 `memory::hot` 与 `memory::projection` 判断谁拿着哪一行的依据，写进了文件而没落账的 claim 是一行看上去有人占着、历史里却无人占着的行。

**现形**：模块 `accounting::effect`。它不是把那五段搬个地方，而是把「先后」从人的纪律换成类型的性质：`Then` 只能从 `Landing::record` 里拿到，而 `record` 先把所有行送进去才返回它。要把顺序写反，得先拿到一个拿不到的值。

- **批而不是逐条**：一张桌子的行全部落完，才轮到它的变化。这改变了 signal 一支的交错方式（原先是 A 落账、A 投递、B 落账…），**但不改变账本字节**：`deliver` 与 `knock` 都不写账（`knock` 只往 `self.knocks` 推一条，由 drive 之后的 `answer_knocks` 统一开跑），所以 `signal_enqueued` 之间的先后原样。
- **计划那一支是全有全无的**，因此它自己一个穷尽枚举 `Claims`：任一条效应对不上盘上的那份，就一行不写、一行不落，只把动过的行号报给人——这是 sprawling-SPEC.md §8-16 定下的形制，这里只把它从 `dispatch_in` 里搬出来并把写盘移到落账之后。
- **`city::archive` 因此拆成两步**（详见 city-SPEC §8-9）：账本行要的 `kind`／`day`／`subject` 全是入参的函数，不需要先写盘就能算出来。不把 `day_of` 搬到装配层算一遍，是因为那会是「一条归档记录长什么样」的第二个权威。
- **`raised`（待批项）不进本模块**：它不是桌子交出来的效应，而是驱动期间被暂存的项，并且在落账前还要受 `tainted_arrival` 改写。它本来就是先落账后改状态的。

**pr 那两支不是同一类，故不动，理由记在这里**：

- `PrEffect::Opened` 里的 `wave_pre` 先于 `pr_opened` 落账，**但它不是「先动世界」**。它铸出的是那条账本行所指向的对象，与 `run_started` 之前那句 `self.cas.put(brief…)` 同形：没有任何记录指向的 git commit 不改变任何人读到的东西。
- `PrEffect::Merged` 里的 `trees.merge` 确实先于 `pr_merged` 落账，而且它真的改变大家读到的干线。**先落账在这里更坏**：`merge` 有一条可达的失败臂 `MergeStale`（分支后干线又动了），先落账就是把一句谎写进历史里的可达路径，而不只是崩溃时的撕裂。要两边都对，`memory::Worktrees` 得先能回答「这一合并会落在哪个 commit」（它就是分支尖，`merge` 今天返回的也正是 `theirs.id()`）且能先验干线。那是另一件事，它自己的红在 `MergeStale` 那一臂上。

**红**：`what_a_run_changes_is_changed_after_the_line_that_announces_it`。一跑归档一条决定、又从共享计划里拿一行；`RunWorker::observe` 的 sink **在一行耐久之后才跑**（`memory::jsonl` 自说：「runs on the appending thread after durability」），所以它正是「先」唯一看得见的位置。断言：`asset_archived` 落时书架上还没有它，`roadmap_claimed` 落时盘上的那一行还没被拿走；跑完两者都在位（只是排了序，不是丢了）。改动之前两条断言各自撞红。

**影面**：`accounting` 公开面有 `effect` 模块，因为写它的桌子仍在 `bin::assembly`；`city` 的 `archive_entry` 与 `collab` 的效应类型是它的入参，所以本 crate 依赖 `city` 与 `collab`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-6 accounting::plan_view：计划从每问一次重解析，变成一次投影（形状 7 投影）

`CityView` 与 `Metrics` 过去每被问一次，就把每栋楼的 `Roadmap.md` 从盘上读出来重新解析一遍。页面是轮询的，而一份计划一小时改不了几次——这是**为一个几乎不变的答案，按提问频率付钱**。

```rust
pub struct PlanView { /* read、causes —— 私有 */ }
pub struct PlanReading {
    pub progress: Progress,
    pub problems: Vec<String>,
    pub rows: Vec<channels::PlanRow>,
    pub blocked: Vec<channels::BlockedLine>,
    pub ready: Vec<NodeId>,
}
impl PlanView {
    pub fn apply(&mut self, record: &EventRecord);
    pub fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading;
}
```

- **文件仍然是计划**。变的只是谁去读：`kernel::WriteMoment` 说这张表只在三个时刻被写，而每一个时刻都是一条记录，于是折叠记录、只在有记录点到那栋楼时才回去读文件。
- **失效由两类记录触发，理由不同**。`roadmap_*` 说一个 run 动了计划——既是忘掉已解析副本的理由，也是一件本身值得留着的事实（红的原因）。`checkpoint_committed` 只说一波工具写过文件——**用 edit 工具改了表的 agent 不留 `roadmap_*` 记录**，一个忽略工具波的缓存会继续报改动之前的计划。
- **它是投影不是副本**：这里不存计划说了什么，只存**上一次读到的时候它是什么**，并在任何可能改变它的事情发生时丢掉。删掉整个它、把同一批记录再折一遍，得到同样的字节——因为它做的全部事情就是折叠。
- **它折的唯一一件文件装不下的事，是节点为什么红**。表格有位置说 `Blocked`；人需要的那句话在 `roadmap_blocked` 的记录里，在表里再放一份就是同一句话的第二个权威。没有记录撑着的 `Blocked` 行仍然算红，措辞退回状态词本身——一个人手改的行仍然是一行说着活停了的行。
- **`BuildingView` 也走这一份**：楼的对象页从这里拿计划，只有文档、房间与档案仍在被问的那一刻读盘。让对象页自己再解析一次，就是「什么卡住了、为什么」有两个答案，而只有一个在折记录。

### 8-7 accounting::home：这个人的家目录，以及本产品放在它下面的东西（形状 4 适配器）

```rust
pub const CITY_DIR: &str;                       // "city"，exe 同级的默认城
pub const NO_HOME: &str;                        // detect 的拒绝与 doctor 的报告共用这一句
pub struct Home { /* root —— 私有 */ }
impl Home {
    pub fn detect() -> Result<Home, AxError>;   // USERPROFILE，其次 HOME；E_PATH_NOT_FOUND
    pub fn at(root: impl Into<PathBuf>) -> Home; // detect 读完环境后造的就是它；比较路径的调用方不读环境直接造
    pub fn path(&self) -> &Path;
    pub fn components(&self) -> PathBuf;        // ~/.sprawling/components
    pub fn person_config(&self) -> PathBuf;     // ~/.sprawling/config.toml
    pub fn default_city(&self) -> PathBuf;      // ~/sprawling/city
}
```

**五条口径：**

1. **三处派生合一。** `doctor::host::components_dir`、`install::dirs`、`main::router::default_city_location` 此前各读一遍 `USERPROFILE || HOME`，而 C 章 3.1 的人层配置本要写第四遍（Roadmap G-10）。读环境的地方只此一处，其余全部由它派生。
2. **住在本 crate，因为读者跨三处。** `person`、`views::skills` 与 run 的阅览室在本 crate，`doctor` 在 `sprawling` 的库那一半，`install` 与 `router` 在它的二进制那一半；`sprawling` 的两半都够得到本 crate，本 crate 够不到它们。
3. **`detect` 失败是类型化错误，调用方各自决定是否致命。** 探组件时家目录缺席只是「看不到」，报告里由 `Absence::NoHome` 说明；装二进制时 Windows 还有 `LOCALAPPDATA` 可落，两者皆无才由 `install::no_home` 拒绝。两处都显式 `match` 错误臂而不是 `.ok()`，于是「没有家目录」是一个被做过的决定。
4. **城不住点目录，因为城是这个人的东西。** `default_city()` 给 `~/sprawling/city`：点目录下装的是与这台电脑绑定的状态（组件、这个人的配置层），而一座城是人要打开、编辑、备份、拷到另一台机器上的工作，看不见的城是备份不了的城。`Absence::NoHome` 那句「neither USERPROFILE nor HOME is set」由 `accounting::home::NO_HOME` 一处定义，`detect` 的拒绝与 doctor 的报告读的是同一句。
5. **`~/.sprawling` 与城里的保留子树共用 `kernel::RESERVED_PREFIX`。** 这是本产品拥有的那一个点目录名，一个名字一个家；它在家目录下装的是属于这个人的东西，不属于任何一座城。`person_config()` 用小写 `config.toml`，与城内各层的 `CONFIG.toml` 不同名——两者是不同的层，同名会诱使某个读者把其中一个当成另一个。本模块只给路径，读写与分层归配置阶梯（H-10）。

### 8-8 accounting::person：这个人自己的那一层（形状 4 适配器；叶子 3.1）

```rust
pub fn read() -> Result<PreferencesAnswer, AxError>;       // Query::Preferences 的全部
pub fn put(patch: PreferencePatch) -> Result<(), AxError>; // Command::PutPreferences 的全部
pub enum CorePriority { Raised, Normal }                   // 人的设置；Normal 即「关掉高优先级」
pub fn core_priority() -> Result<CorePriority, AxError>;   // ConfigInvalid：priority 既不是 "raised" 也不是 "normal"
```

- **文件在每一座城之外**：`<home>/.sprawling/config.toml`，路径由 `accounting::home`（8-7）给，本模块不拼路径。把城拷到另一台机器，它不跟着走；在同一台机器上换一个浏览器，画出来的仍是这份文件说的样子。
- **`[ui]` 一节就是 `PreferencesAnswer` 的序列化**（channels-SPEC §8-39 第七条）：文件能写的键与答案能说的字段是**同一份声明**，因此本模块只做读与写，不陈述「一项偏好是什么」。一条补丁落在记录上的效果同理，归 `PreferencesAnswer::apply` —— `Chord("")` 是解绑还是绑一个空串，只有一个地方回答。
- **别的节原样留下**：写是一次读-改-写，经 `city::edit_document`（city-SPEC §8-27）持锁并整份替换。「要么整份要么不动」只有一份实现，人层与城层共用它；再写一份就是给 B-49 立第二个权威。
- **读不动的文件不覆写**：解析失败报 `E_CONFIG_INVALID`，主题带上文件与是哪一节，恢复语请人手工修或删掉那一节重选。能读回来的才配被改写——写它的人是唯一能修它的人。
- **不入账**：偏好不属于城的历史，任何 run 都观测不到它。因此这条命令被接受时城无话可播，`adversary` 第四世界据此把「静默」读作接受，而它真正的关门条件是读回来那一组断言（`adversary/src/Sprawling/Person.lean`，叶子 5.6）。
- **文件缺席不是失败**：那是一个什么都还没定的人，答案是本 build 画的那几档（`PreferencesAnswer::default`）。`lang` 缺席就是缺席，不填 `en`——没人选过之前，只有浏览器自己的语言标签是证据。

**本章测试**：`what_the_file_states_and_what_the_answer_states_are_one_record`、`a_section_this_build_does_not_read_survives_a_write`、`a_file_that_does_not_parse_is_refused_rather_than_replaced`（`accounting::person::tests`）。
- **`CorePriority` 住在这里而不在 `bin::serving::standing`**：它是这份文件里 `[core] priority` 读出来的值；真去抬高一条线程的做法归 `serving::standing`（sprawling-SPEC.md 8-93），它从这里取值。

### 8-9 accounting::held_vault 与 accounting::toolkit_broker：读面与写者共用的两组事实（形状 2 值类型）

```rust
// accounting::held_vault
pub fn resolving(vault: Arc<Mutex<gateway::Custodian>>) -> gateway::SecretResolver;
pub fn poisoned_vault() -> AxError;   // E_STORAGE_FATAL：vault 的锁中毒
// accounting::toolkit_broker
pub fn broker_for(/* toolkit 地址、城根、vault */) -> Result<Option<(protocol::Broker, String)>, AxError>;
```

- **一个锁着的 vault 的解析器与锁中毒时的拒绝各只有一处**：装配点、读面与 serving 都要一次性的解析器，拒绝的措辞只写一次。
- **broker 的钥匙登记在哪、这座城对 broker 是谁，页面与命令读同一组事实**：连接动作 `connect_toolkit` 仍是 worker 的。

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只依赖 `kernel` 与 `gateway` 的接口类型。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。
2. **`with_models` 是一个消费 `self` 的方法，而不是 `new` 的第四个参数。** 理由：生产只有一种工厂，`new` 的每个调用方（serve、doctor、测试）都会写同一个 `GatewayModels`；换工厂的只有 citysim 与测试。被否决的做法：`new` 加参数——四个调用点重复同一个值，而这个值只有一个权威。
3. **端口参数是 `Chosen` 与 `Redemption`，不含 dialect 头。** 理由：dialect 头由 `Chosen` 的 dialect 决定，把它交给调用方算，两个调用点就各有一份拼法。被否决的做法：照抄 `gateway::adapter_for` 的三参数签名。
4. **`Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `protocol::Outbound`。** 理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `protocol`，不归 worker。
5. **`Connectors::connect` 取 `&mut self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`。** 理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
6. **`Machine` 回答整页，而不是逐项回答 `look(&Requirement) -> Presence`。** 理由：worker 要的是一页答案与一次安装；逐项的端口要把 `Requirement`、`Detection`、`Family`、`PerPlatform`、`Platform` 与 `Presence` 整个搬进本 crate，而且沙箱与凭据保管这两项机器级的读仍然绕过端口直接碰主机，脚本也就换不掉它们。被否决的做法：逐项端口——搬走 doctor 的整个模型，却仍留两条通向主机的路。`bin::doctor::Machine` 是本 trait 的子 trait，给 doctor 自己逐项判定时加一个 `look`，它的第二实现在 doctor 的测试里；它不另设 `install`，安装只有本 trait 这一扇门。
7. **`install` 收 `Runnable`，不收程序名加参数；`Recipe` 与 `Runnable` 因此一起住在本 crate。** 理由：`Runnable` 证明配方是 `Command`，只有与它同住一个 crate 的 `Recipe::command` 能造它，所以一个 `Machine` 实现不会被递到一条打印的或手动的配方。哪些程序可以跑由 `doctor_install` 对 requirement 表的查找决定，不由这个类型决定。被否决的做法：收 `&str` 与 `&[&str]`——拒绝打印配方的规则就只剩每个调用方的自觉。让 `Recipe` 的字段私有、只让表能构造，可以把许可也放进类型，但表住在 `sprawling`、类型住在本 crate，没有一种 crate 布局能便宜地做到。
8. **`effect` 与 `plan_view` 先于 `RunWorker` 搬进本 crate。** 理由：它们只依赖 `kernel`、`city` 与 `collab`，不碰 §7 归属表里的任何一个 `bin` 模块，搬它们不需要新端口，而 worker 搬过来时它们必须已经在这里。被否决的做法：等 worker 整体搬迁时一起搬——那一次改动就同时背着机械的搬移与端口设计，审的人分不开两者。
9. **worker 的决定与读面搬进本 crate，通往主机、网络或终端的做法留在 `sprawling`、经端口交进来（§7 归属表）。** 理由：端口正是 citysim 插第二实现的地方；把一个适配器搬进来，它碰主机的那一步就跟着进了 citysim 驱动的 crate，脚本场景会真的起浏览器、读内存、打开文件管理器。被否决的做法：全部搬进来——本 crate 就要依赖 `thread-priority`、`sysinfo`、浏览器与终端，citysim 换不掉其中任何一个；把 `views` 留在 `sprawling`、经端口交给 worker——`Governance` 由读侧拥有，写侧在写下记录之前就要同步地从它作决定（sprawling-SPEC.md 8-92），端口会把一个 trait 放到决定路径上，并把一份折叠的权威分到两个 crate。
10. **没有状态的主机读写经构造时交进来的 `fn` 指针进来，和 `read_volume` 一样；只有持有状态的适配器（`Machine`、`Connectors`）才是 trait。** 理由：一个只包一个函数的 trait 没有第二个方法可换，脚本场景交一个自己的 `fn` 就够了，而且 `fn` 指针不装箱、不经虚表。被否决的做法：一个把内存、卷、随机令牌、打开文件管理器与浏览器捆在一起的 `Host` trait——这些做法的失败各不相同，脚本为了换掉其中一个就得实现全部。
11. **`relay`、`pool`、`desk` 与 `drive_run` 和 `RunWorker` 在同一次改动里搬。** 理由：它们成环——`relay` 经 worker 的账本写，`pool` 的每条车道跑 `drive_run`，`drive_run` 经 `relay` 写回，`desk` 为 worker 排队命令；先搬其中任何一个，都要一个指回留在 `sprawling` 的 worker 的临时端口，而下一次改动就会删掉它。被否决的做法：一个一个搬、中间架临时端口——每个临时适配器都是一个只活一次改动的第二权威。
12. **装配根留在 `bin::assembly`：`listening`、`attending`、`chain_watch`、`genesis`、生产适配器（`models::GatewayModels`、`mcp::Residents`、`SystemClock`、`doctor::ThisMachine`）与把它们装上 worker 的构造。** 理由：它们起线程、绑端口、打开 vault、造城的目录，是 ARCHITECTURE.md §3 说的知道每个具体类型的那一层；worker 搬走之后，它们对本 crate 的依赖是朝内的。被否决的做法：把 `genesis` 当作 worker 的用例搬进来——它在任何 worker 存在之前造城，并经 `serving::open_vault` 打开 vault，搬进来就要为一次性的建城多开一个端口。
