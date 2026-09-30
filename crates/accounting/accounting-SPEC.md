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
| `views` | 页面问的每个问题，怎样从折叠的记录里答 | 8-10 |
| `lineage` | 每个 run 怎样折成一行，带上它的父指针 | 8-10 |
| `worker` | 城的唯一写者 `RunWorker`：它持有的状态、它执行的命令、它驱动的 run | 8-11 |

worker 的两个读写面 `effect` 与 `plan_view`，以及 worker 与 `views` 共用的四个叶子模块 `home`、`person`、`held_vault`、`toolkit_broker`，已在本 crate；`views` 与 `lineage` 也在本 crate。`RunWorker` 与它的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例还住 `crates/sprawling/src/assembly`，按 §3 的次序搬进来。

## 2 验收标准

六条断言。前三条在 `crates/sprawling/tests/model_factory.rs`，都驱动一个接收了脚本 `ModelFactory` 的 `RunWorker`：

| 测试 | 它钉住的事 |
|---|---|
| `a_dispatch_reaches_the_model_the_worker_was_handed` | run 的模型走端口。端点指向一个拒绝连接的 loopback 端口，所以一个绕过端口、自己造适配器的 worker 碰不到脚本模型，历史里也就没有脚本写下的那句回答。 |
| `an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone` | 没有 session 的 dispatch 按规则从任务的词里取房间名（sprawling-SPEC.md 8-86），不做命名调用：工厂只被问到 run 的主模型，run 在规则给出的房间里开始。 |
| `a_confidential_building_refuses_before_the_factory_is_asked` | 机密楼的拒绝在 worker 的选择里，不在工厂里：端点不是 loopback 地址时 dispatch 以 `GateDenied` 被拒，脚本工厂一次也没被问到。换掉工厂不能绕开机密。 |

第四条在 `crates/sprawling/tests/connectors.rs`：`a_run_is_offered_the_tools_the_worker_was_handed`。楼的配置写了一个 MCP server，它的命令在任何主机上都不存在；worker 接收了一个脚本 `Connectors`，它给出一个工具。一个绕过端口、自己启动 server 的 worker 起不来这个 server，模型收到的工具表里也就没有那个工具。

第五条在 `crates/sprawling/tests/clock.rs`：`a_worker_stamps_its_lines_with_the_clock_it_was_handed`。worker 接收一个停在固定时刻的脚本 `Clock` 之后写下的每一行，`t` 都是那个时刻；一个自己读墙钟的写点写下的 `t` 是现在的时间。

第六条在 `crates/sprawling/tests/machine.rs`：`a_refresh_counts_the_items_the_machine_it_was_handed_answered`。worker 接收一个脚本 `Machine`，它的回答只有一个条目；`DoctorRefresh` 之后 worker 在诊断里报的条目数是 1。一个自己去问主机的 worker 报的是需求表的全部条目数。测试从 `DoctorRefresh` 进而不从 `DoctorInstall` 进：对着一个绕过端口自己动手的 worker，后者会在宿主上真的启动包管理器。

## 3 假设与歧义

- `RunWorker` 与它的六个对象、全部用例还在 `crates/sprawling/src/assembly`，所以 citysim 的场景库仍驱动不了一次 dispatch：citysim 的 bench 二进制依赖 `sprawling`，场景库只经 `runtime::run::drive` 驱动一次 run，从不造 worker。归属由 §7 的表与 §12-9 至 §12-12 定下，§12-13 至 §12-16 补上 `views` 那一半。还没做的按这个次序：
  1. 还缺的端口。`views::lines::harnesses_answer` 找 harness 的程序时直接读 `doctor::host::search_path` 与 `doctor::on_search_path`；它改经 `Views.programs`（8-10、§12-13）。`revealing`（`RunWorker.reveal`）、`monitor::memory`（`DrivingPool` 的 `read_memory`）、`monitor::volume`（`RunWorker.read_volume`）、`release`（`Views.registry`）、`browser_tool`（`RunWorker.browsers`）与需求表的查法（`RunWorker.recipe_for`）已经是交进来的 `fn` 指针；worker 这边还直接碰 `bin` 的是 `workbench::engine` 读的 `doctor::host` 与 `Presence`，以及 `relay`、`pool`、`driving::flight` 用到的 `monitor::health::Health`、`monitor::memory::Memory` 两个类型。
  2. `RunWorker`、`relay`、`pool`、`desk`、`drive_run` 与六个对象、全部用例在一次改动里搬（§12-11）。`views` 的测试此前经 `views::tests::founded` 造城，它只写创世的两行、立起市政厅、写一份 `City.md`；worker 搬进来以后它们改回真正的创世，`founded` 随之删去（§12-16）。快照折叠的 trait `views::snapshot::start::SnapshotFold` 在此之前有两个实现分住两个 crate（`Views` 与 `bin::assembly::folds::standing_start` 的 `StandingFolds`），所以是 ARCHITECTURE.md §4 缝表里的一行；worker 搬进来以后两个实现同住本 crate，它收回 `pub(crate)`，那一行随之删去。
  3. citysim 经本 crate 的端口驱动一次 dispatch，ARCHITECTURE.md §11 的 V6 缺口随之关闭。写这个场景是 citysim 的活，不在本次迁移里。
  4. 模块搬走时，它在 sprawling-SPEC.md 里的那一节留在原处，只把模块路径改成新的拼写：`bin::views::x` 写作 `accounting::views::x`，`bin::assembly::x` 写作 `accounting::worker::x`。这些节在 S4 迁 `Spec.lean` 时一次进入本 crate 的规格（§12-15）。8-7、8-8、8-9 是早先整节搬进来的，保持原样。
- `views::mcp_health` 自己用 `agent_protocols::McpLink` 启动一个 MCP server 去问它的健康，不经 `Connectors`。未定的是这次读要不要也经端口：`views` 搬进本 crate 时它照原样搬（`agent_protocols` 本来就是本 crate 的依赖）；能定下它的证据是一个脚本场景需不需要回答 MCP 健康查询。

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
- MCP 的生命周期（`initialize`、`notifications/initialized`、`tools/list`）怎样说，归 `agent_protocols`；一个工具能不能在机密楼里存在，归 `agent_protocols::McpTool::new`。端口只声明「连上一个 server，交回它的工具」。
- 机密楼根本不启动 server，这一步在 worker 的 `mcp_tools` 里、端口被问到之前。
- 主机上有什么、每一项怎样判定、怎样折叠成一页，归 `bin::doctor`（city 所在主机有什么，它是唯一权威）；一条安装配方能不能跑，归 `Recipe::command`；worker 只决定一个名字在不在需求表里、这个平台有没有配方。

**worker 用到的每个 `bin` 模块归哪一边。** 规则是 §12-9：worker 的决定与读面搬进本 crate；通往主机、网络或终端的做法留在 `sprawling`，经一个端口交进来。

| `bin` 里的东西 | 归属 | worker 或 `views` 用它做什么 | 依据 |
|---|---|---|---|
| `views` | 搬进本 crate | `Views`、`Published`、`Governance`、`pursued`、`snapshot::start` | worker 的读面；`Governance` 由读侧拥有，写侧从那里取用（sprawling-SPEC.md 8-92） |
| `lineage` | 搬进本 crate | `sprawling view` 的 run 列表，以及 playback 的共享投影 | 它只折记录，与 `views` 同形（形状 7）；二进制的 `main::view` 与本 crate 的读面都够得到它 |
| `home` | 搬进本 crate | 阅览室与 `views::skills` 取这个人的家目录 | 只读一个环境变量、拼路径，不启动任何东西；`person` 与 `views` 都从它取路径 |
| `person` | 搬进本 crate | `PutPreferences` 写、`Preferences` 查询读 | 人的那一层是一份文件，读写它和读写城的文件同类，不是主机的能力 |
| `serving::standing::CorePriority` | 随 `person` 搬进本 crate | 偏好里核心线程抬不抬高的那个值 | 它是 `person` 读出来的值；真去抬高线程的 `raise_this_thread` 留在 `serving` |
| `held_vault` | 搬进本 crate | 把一个锁着的 vault 变成解析器，锁中毒时的拒绝 | 纯函数，只碰已经打开的 vault |
| `toolkit_broker` | 搬进本 crate | 一个外部应用的 broker 钥匙登记在哪 | 纯函数，`views::toolkits` 与连接动作读同一组事实 |
| `doctor`（`REQUIREMENTS`、`Platform`、`host`、`Presence`、`PATIENCE`、`ThisMachine`） | 经端口：看与装经 `Machine`，需求表的查法经 `RunWorker.recipe_for`（sprawling-SPEC.md 中 `doctor_install` 那一节） | 需求表查找、执行引擎的路径 | 主机上有什么，`bin::doctor` 是唯一权威（本节上文）；`views::lines::harnesses_answer` 找一条命令的程序经 `Views.programs`，生产交的是 `bin::doctor::host::find_program`（8-10） |
| `monitor::memory::read`、`monitor::volume::read` | 经端口：`DrivingPool` 的 `read_memory` 与 `RunWorker` 的 `read_volume`，都是 `fn` 指针（sprawling-SPEC.md 8-46-3、8-94） | 新工作进门时读内存与卷的余量 | 读主机的计数器；`read_volume` 已经这样交进来 |
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
- **一个调用点**：dispatch 的同意阶段（`agreeing`）经 `RunWorker.models` 造适配器，再套上 keep-warm 门（sprawling-SPEC.md 8-112）。房间名按规则取，不调模型，所以没有第二个调用点。dialect 头只在生产适配器里算一次。
- **固定值**：`RunWorker::new` 与 `over` 装上 `GatewayModels`；`with_models` 是唯一换掉它的门。
- **一致性套件**：端口的断言就是 §2 那条测试——拿到的适配器必须就是被调用的那一个。

### 8-2 accounting::connectors（形状 3 端口）

```rust
pub trait Connectors {
    /// # Errors
    /// Whatever starting the server, its handshake or its listing
    /// refuses.
    fn connect(
        &self,
        server: &kernel::McpServer,
        write_root: &std::path::Path,
        confidential: bool,
        resolve: &gateway::SecretResolver,
    ) -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError>;
}

pub enum Reached {
    Connected(agent_protocols::Handshake),   // 这次调用启动或打开、并握过手
    Resident,                         // 早先一次调用连上、仍在运行
}
```

```rust
// bin::assembly::mcp（形状 4 适配器）
#[derive(Default)]
pub(crate) struct Residents;              // 生产：McpLink::open + agent_protocols::handshake + tools/list，连接在 run 之间保持（sprawling-SPEC.md 8-4）
impl RunWorker {
    pub fn with_connectors(self, connectors: Box<dyn accounting::Connectors + Send + Sync>) -> RunWorker;
}
```

- **失败**：原样传 `McpLink::open`、`agent_protocols::handshake` 与 `agent_protocols::tools_from` 的 `AxError`。worker 把失败写进 diagnostics、把这个 server 留在外面，run 照常开始；端口不另造错误码。
- **`confidential` 原样传给 `McpTool::new`**：那是工具层的权威。worker 在机密楼里一个 server 都不启动，所以生产路径上它总是 `false`；它仍在签名里，是为了任何实现都不能造出一个绕过工具层拒绝的工具。
- **端口有状态，可被几条线程同时问**：生产适配器把连上的 server 按声明与 run root 留在表里，下一次 dispatch 直接拿它的工具；子进程已经退出的那一行在这里被丢掉、重新启动。`connect` 取 `&self`、实现是 `Sync`，因为准备派活的 lane 各自问同一张表（sprawling-SPEC.md 8-113），表自己按键上锁（sprawling-SPEC.md 8-4）。
- **固定值**：`RunWorker::new` 与 `over` 装上 `Residents::default()`；`with_connectors` 是唯一换掉它的门。
- **依赖**：本 crate 因此依赖 `agent_protocols`（ARCHITECTURE.md §3 的 `depmap`）。

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
- **worker 之外的两个读点用 `SystemClock`**：`form_city`（city 在任何 worker 存在之前诞生）与 `serving::journal`（诊断日志行的时间，不是城的记录）。`RunWorker::new` 打开 ledger 时、`RunWorker::holding`（`new` 与 `over` 都经过它）为 `last_tick` 取起点时，worker 都还不存在，`with_clock` 来不及生效，所以两处也用 `SystemClock`。
- **固定值**：`RunWorker::new` 与 `over` 装上 `SystemClock`；`with_clock` 是唯一换掉它的门。`Send + Sync` 与 `Arc`，是因为 lane 线程与 worker 同时读它。
- **等待也读这个钟**：lane 等 provider 的退避时，一片一片地睡，直到这个钟过了期限。所以一个永远不走的脚本钟，会让遇上退避的 run 一直等下去；脚本要让钟往前走。

### 8-4 accounting::machine（形状 3 端口）

```rust
pub trait Machine {
    /// Asks this machine every question the requirement table holds.
    fn report(&self) -> wire::DoctorAnswer;
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
- **依赖**：`report` 交回线上的 `wire::DoctorAnswer`，所以本 crate 依赖 `wire`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-5 accounting::effect：一条效应先成为账本行，再成为这座城（形状 2 值类型）

```rust
// crates/accounting/src/effect.rs —— `architecture.toml` 的 accounting::effect，形状 2（值类型）
pub struct Line { pub who: String, pub addr: Address, pub kind: EventKind, pub data: Payload }
pub struct Filing { pub entry: city::ArchiveEntry, pub body: String }

/// 一张桌子留下的全部效应：它们成为的行，以及行之后才允许发生的变化。
pub struct Landing<L = Line> { lines: Vec<L>, then: Then }        // 两个字段都是私有的
/// 计划落地的一行，与它关掉的那个认领；拆分关掉它的父节点。
pub struct Closing { pub line: Line, pub closes: Option<NodeId> }

pub enum Then { Nothing, Deliver(Vec<collab::Signal>),
                Roadmap { path: PathBuf, base: String, text: String }, Shelf(Vec<Filing>) }

impl<L> Landing<L> {
    /// 先走完每一行，再把变化交出去。这是 `Then` 唯一的出口。
    pub fn record(self, append: &mut impl FnMut(L) -> Result<(), AxError>) -> Result<Then, AxError>;
}
impl Landing {
    pub fn signals(Vec<SignalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub fn discards(Vec<Payload>, room: &Address, who: &str) -> Landing;
    pub fn shelf(Vec<ArchiveEffect>, write_root: &Path, building: &Address, at: TimeMs, room: &Address, who: &str) -> Result<Landing, AxError>;
}

/// 一跑对共享计划做的事。两种而无第三种：每条效应都还对得上盘上那份、全部重放上去，或有一条对不上、一个字也不写。
pub enum Claims {
    Landed(Box<Landing<Closing>>),
    Stale { nodes: Vec<String>, released: Box<Landing<Closing>> },   // 动过的节点报给人；released 关掉本跑已落账的认领
}
impl Claims { pub fn of(effects: &[ClaimEffect], on_disk: &str, path: PathBuf, room: &Address, who: &str) -> Result<Claims, AxError>; }

// 装配层那一扇门（bin::assembly::settling::landing）：每张桌子都走它，`Then` 的 match 穷尽
impl RunWorker { fn settle(&mut self, at: &Assignment, run: RunId, landing: effect::Landing, chain: &KnockChain) -> Result<(), AxError>; }
```

**原因**：先把效应变成账本行、再变成状态，是 Ledger 的定义（`docs/glossary.md`：「Every effect becomes an EventRecord first」；ARCHITECTURE.md §5 步 4）。写反的代价是具体的：先上书架后落账，落账失败就在架上留下一条历史没有的记录；先改共享计划后落账，而 `roadmap_claimed` 是 `storage::hot` 与 `storage::projection` 判断谁拿着哪一行的依据，写进了文件而没落账的认领是一行看上去有人占着、历史里却无人占着的行。

**形状**：先后是类型的性质，而不是写桌子的人的纪律。`Then` 只能从 `Landing::record` 里拿到，而 `record` 先把所有行送进去才返回它；要把顺序写反，得先拿到一个拿不到的值。

- **批而不是逐条**：一张桌子的行全部落完，才轮到它的变化。signal 一支因此先落完所有 `signal_enqueued` 再投递；`deliver` 与 `knock` 都不写账，所以账本字节与逐条交错时相同。
- **计划那一支是全有全无的**，所以它自己一个穷尽枚举 `Claims`：任一条效应对不上盘上的那份，就一字不写，把动过的节点报给人，并用 `released` 里的 `roadmap_released` 行关掉本跑已经落账的认领（sprawling-SPEC.md §8-16 的形制）。效应重放到 `on_disk` 上，而不是写回派活时的副本，所以别的 run 在此期间落下的行保留。记账线程在模型认领时已经拒绝了另一个在飞 run 持有的节点（`assembly::booking`，sprawling-SPEC.md 8-42-8）；`Claims::of` 是后盾，接住从旧副本认领了已被别人落地的节点的 run。
- **归档行不需要先写盘**：账本行要的 `kind`／`day`／`subject` 由 `city::archive_entry` 从入参算出（city-SPEC §8-9）。不在装配层另算 `day_of`，因为那会是「一条归档记录长什么样」的第二个权威。
- **`raised`（待批项）不进本模块**：它不是桌子交出来的效应，而是驱动期间暂存的项，本身就先落账后改状态。

**pr 那两支不走 `Landing`，理由记在这里**：

- `PrEffect::Opened` 里的 `storage::Checkpoint::land` 先于 `pr_opened` 落账，**但它不是「先动世界」**。它铸出的是那条账本行所指向的 commit，与 `run_started` 之前把 brief 放进 CAS 同形：没有任何记录指向的 git commit 不改变任何人读到的东西。
- `PrEffect::Merged` 先经 `Worktrees::plan_merge` 定下这次合并会落在哪个 commit，干线已经动过的拒绝（`MergeStale`）在这一步就报出，然后才写 `pr_merged`。所以不会有一条 `pr_merged` 是替一次注定被拒的合并写的（sprawling-SPEC.md「合并也排到它那条行后面」）。

**测试**：`what_a_run_changes_is_changed_after_the_line_that_announces_it`（`bin::assembly::driving::tests::ledger`）。一跑归档一条决定、又从共享计划里拿一行；`RunWorker::observe` 的 sink 在一行耐久之后才跑，所以它正是看得见「先」的位置。断言：`asset_archived` 落时书架上还没有它，`roadmap_claimed` 落时盘上那一行还没被拿走；跑完两者都在位。

**影响面**：`accounting` 公开面有 `effect` 模块，因为写它的桌子在 `bin::assembly`；`city` 的 `archive_entry` 与 `collab` 的效应类型是它的入参，所以本 crate 依赖 `city` 与 `collab`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-6 accounting::plan_view：计划从每问一次重解析，变成一次投影（形状 7 投影）

`CityView` 与 `Metrics` 每被问一次都要每栋楼的计划。页面是轮询的，而一份计划一小时改不了几次；每问一次就把 `Roadmap.md` 从盘上读出来重新解析，是**为一个几乎不变的答案，按提问频率付钱**。

```rust
pub struct PlanView { /* read、causes —— 私有 */ }
pub struct PlanReading {
    pub progress: Progress,
    pub problems: Vec<String>,
    pub rows: Vec<wire::PlanRow>,
    pub blocked: Vec<wire::BlockedLine>,
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

1. **三处派生合一。** 读 `USERPROFILE || HOME` 的地方只此一处：doctor 的组件目录、安装目录、默认城的位置与人层配置都由它派生，而不是各读一遍环境。
2. **住在本 crate，因为读者跨三处。** `person`、`views::skills` 与 run 的阅览室在本 crate，`doctor` 在 `sprawling` 的库那一半，`install` 与 `router` 在它的二进制那一半；`sprawling` 的两半都够得到本 crate，本 crate 够不到它们。
3. **`detect` 失败是类型化错误，调用方各自决定是否致命。** 探组件时家目录缺席只是「看不到」，报告里由 `Absence::NoHome` 说明；装二进制时 Windows 还有 `LOCALAPPDATA` 可落，两者皆无才由 `install::no_home` 拒绝。两处都显式 `match` 错误臂而不是 `.ok()`，于是「没有家目录」是一个被做过的决定。
4. **城不住点目录，因为城是这个人的东西。** `default_city()` 给 `~/sprawling/city`：点目录下装的是与这台电脑绑定的状态（组件、这个人的配置层），而一座城是人要打开、编辑、备份、拷到另一台机器上的工作，看不见的城是备份不了的城。`Absence::NoHome` 那句「neither USERPROFILE nor HOME is set」由 `accounting::home::NO_HOME` 一处定义，`detect` 的拒绝与 doctor 的报告读的是同一句。
5. **`~/.sprawling` 与城里的保留子树共用 `kernel::RESERVED_PREFIX`。** 这是本产品拥有的那一个点目录名，一个名字一个家；它在家目录下装的是属于这个人的东西，不属于任何一座城。`person_config()` 用小写 `config.toml`，与城内各层的 `CONFIG.toml` 不同名——两者是不同的层，同名会诱使某个读者把其中一个当成另一个。本模块只给路径，读写与分层归配置阶梯（H-10）。

### 8-8 accounting::person：这个人自己的那一层（形状 4 适配器）

```rust
pub fn read() -> Result<PreferencesAnswer, AxError>;       // Query::Preferences 的全部
pub fn put(patch: PreferencePatch) -> Result<(), AxError>; // Command::PutPreferences 的全部
pub enum CorePriority { Raised, Normal }                   // 人的设置；Normal 即「关掉高优先级」
pub fn core_priority() -> Result<CorePriority, AxError>;   // ConfigInvalid：priority 既不是 "raised" 也不是 "normal"
```

- **文件在每一座城之外**：`<home>/.sprawling/config.toml`，路径由 `accounting::home`（8-7）给，本模块不拼路径。把城拷到另一台机器，它不跟着走；在同一台机器上换一个浏览器，画出来的仍是这份文件说的样子。
- **`[ui]` 一节就是 `PreferencesAnswer` 的序列化**（wire-SPEC §8-39 第七条）：文件能写的键与答案能说的字段是**同一份声明**，因此本模块只做读与写，不陈述「一项偏好是什么」。一条补丁落在记录上的效果同理，归 `PreferencesAnswer::apply` —— `Chord("")` 是解绑还是绑一个空串，只有一个地方回答。
- **别的节原样留下**：写是一次读-改-写，经 `city::edit_document`（city-SPEC §8-27）持锁并整份替换。「要么整份要么不动」只有一份实现，人层与城层共用它；再写一份就是给 B-49 立第二个权威。
- **读不动的文件不覆写**：解析失败报 `E_CONFIG_INVALID`，主题带上文件与是哪一节，恢复语请人手工修或删掉那一节重选。能读回来的才配被改写——写它的人是唯一能修它的人。
- **不入账**：偏好不属于城的历史，任何 run 都观测不到它。因此这条命令被接受时城无话可播，`adversary` 第四世界据此把「静默」读作接受，而它真正的关门条件是读回来那一组断言（`tools/adversary/src/Sprawling/Person.lean`，叶子 5.6）。
- **文件缺席不是失败**：那是一个什么都还没定的人，答案是本 build 画的那几档（`PreferencesAnswer::default`）。`lang` 缺席就是缺席，不填 `en`——没人选过之前，只有浏览器自己的语言标签是证据。

**本章测试**：`what_the_file_states_and_what_the_answer_states_are_one_record`、`a_section_this_build_does_not_read_survives_a_write`、`a_file_that_does_not_parse_is_refused_rather_than_replaced`（`accounting::person::tests`）。
- **`CorePriority` 住在这里而不在 `bin::serving::standing`**：它是这份文件里 `[core] priority` 读出来的值；真去抬高一条线程的做法归 `serving::standing`（sprawling-SPEC.md 8-93），它从这里取值。

### 8-9 accounting::held_vault 与 accounting::toolkit_broker：读面与写者共用的两组事实（形状 2 值类型）

```rust
// accounting::held_vault
pub fn resolving(vault: Arc<Mutex<gateway::Custodian>>) -> gateway::SecretResolver;
pub fn poisoned_vault() -> AxError;   // E_STORAGE_FATAL：vault 的锁中毒
// accounting::toolkit_broker
pub fn broker_for(/* toolkit 地址、城根、vault */) -> Result<Option<(agent_protocols::Broker, String)>, AxError>;
```

- **一个锁着的 vault 的解析器与锁中毒时的拒绝各只有一处**：装配点、读面与 serving 都要一次性的解析器，拒绝的措辞只写一次。
- **broker 的钥匙登记在哪、这座城对 broker 是谁，页面与命令读同一组事实**：连接动作 `connect_toolkit` 仍是 worker 的。

### 8-10 accounting::views 与 accounting::lineage：页面问的每个问题，从折叠里答（形状 7 投影）

```rust
// accounting::views
pub struct Views { /* 折叠状态，私有 */ }
impl Views {
    pub fn new(city_root: &Path) -> Views;
    pub fn over(ledger_dir: &Path) -> Views;
    /// 先审计整条链，再从合适的快照起步、只折尾部（sprawling-SPEC.md 8-91）。
    pub fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError>;
    /// 锁内只取小数据；读盘、读库、出网在 `Prepared::finish` 里做（sprawling-SPEC.md 8-100）。
    pub fn prepare(&self, query: &wire::Query) -> Prepared;
    pub fn twin(&self) -> Result<Views, AxError>;

    // 服务中的城从外面交进来的五样东西（`views::served`）。都不由记录折出，重建不碰它们，`twin` 把它们带到另一份。
    pub fn found_on_this_machine(&mut self, report: wire::DoctorAnswer);
    pub fn lend_the_vault(&mut self, vault: Arc<Mutex<gateway::Custodian>>);
    pub fn ask_the_registry_through(&mut self, newest: fn() -> wire::ReleaseAnswer);
    pub fn ask_upstream_through(&mut self, newest: fn(&str) -> wire::DoctorUpstream);
    pub fn find_programs_through(&mut self, find: fn(&str) -> Option<PathBuf>);
}
pub enum Prepared { /* 锁放开之后还要做的那一步 */ }
impl Prepared { pub fn finish(self) -> wire::Answer; }
pub struct Published { /* 私有 */ }
pub fn answer_outside_the_lock(views: &Published, query: &wire::Query) -> (Seq, Result<wire::Answer, AxError>);
/// 不 serve 一座城，只从它自己的历史答一问。
pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError>;
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn>;
pub struct Governance { /* 读侧那一份治理折叠 */ }
pub fn pursued(record: &EventRecord) -> Result<Address, AxError>;

// accounting::lineage
pub struct Lineage { /* … */ }
pub struct RunLine { /* 公开字段不变 */ }
pub fn lineage_of(ledger_dir: &Path) -> Result<Lineage, AxError>;
```

- **读面对这台电脑只有五个入口，都经 `views::served` 交进来。** `machine` 是城启动后 doctor 看到的那一眼；`vault` 是 worker 打开的那一个；`registry` 与 `upstream` 各问一次网络；`programs` 回答「这台电脑的搜索路径上有没有这个程序」。五个都是服务中的城交的，所以一份没人 serve 的 `Views`（重建、`ask`、测试）对它们一律答 `Unavailable`，不去碰这台电脑。
- **harness 页经 `Views.programs` 找程序。** `Query::Harnesses` 在快照放开之后作答：`Some(find)` 时对 `agent_protocols::Harness::ALL` 里每一家的启动程序调一次 `find`，`found` 是它有没有交回一条路径；`None` 时答 `Unavailable { query: "Harnesses" }`。生产交的是 `bin::doctor::host::find_program`，它读的是 doctor 读的同一条搜索路径（`host::search_path` 加 `probe::on_search_path`），所以 harness 页与 doctor 对同一个程序给同一个答案。钉住它的测试是 `a_harness_is_looked_for_through_the_search_the_views_were_handed` 与 `a_harness_page_nobody_served_answers_unavailable`（`accounting::views::served::tests`）。
- **对 `sprawling` 公开的是这一节列出的面。** 模块在 `sprawling` 里时 `pub(crate)` 的条目，搬过来以后是 `pub`：装配根、服务面与二进制照原样读它们。`Views::answer` 仍只在本 crate 的测试里存在；`sprawling` 的测试写 `prepare(&query).finish()`，那是生产走的同一条路。
- **`lineage` 与 `views` 同住本 crate，因为读者跨两处。** `sprawling view` 的 run 列表在二进制里，playback 的共享投影在本 crate 的读面里；二进制够得到本 crate，本 crate 够不到二进制。
- **依赖**：`views` 折叠 `storage::HotView`、`storage::Attribution` 与 `storage::LedgerIndex`，快照起步经 `runtime::replay::fold_ledger_dir`，所以本 crate 依赖 `storage` 与 `runtime`（ARCHITECTURE.md §3 的 `depmap`，§12-14）。

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只用 `gateway` 的接口类型，不构造适配器。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。
2. **`with_models` 是一个消费 `self` 的方法，而不是 `new` 的第四个参数。** 理由：生产只有一种工厂，`new` 的每个调用方（serve、doctor、测试）都会写同一个 `GatewayModels`；换工厂的只有 citysim 与测试。被否决的做法：`new` 加参数——四个调用点重复同一个值，而这个值只有一个权威。
3. **端口参数是 `Chosen` 与 `Redemption`，不含 dialect 头。** 理由：dialect 头由 `Chosen` 的 dialect 决定，把它交给调用方算，两个调用点就各有一份拼法。被否决的做法：照抄 `gateway::adapter_for` 的三参数签名。
4. **`Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `agent_protocols::Outbound`。** 理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `agent_protocols`，不归 worker。
5. **`Connectors::connect` 取 `&self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`。** 理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。取 `&self` 而不是 `&mut self`，是因为 MCP 缺表时的连接在 lane 里做，几条 lane 借同一张表的 `Arc`；`&mut self` 会把所有 lane 的握手排成一队。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
6. **`Machine` 回答整页，而不是逐项回答 `look(&Requirement) -> Presence`。** 理由：worker 要的是一页答案与一次安装；逐项的端口要把 `Requirement`、`Detection`、`Family`、`PerPlatform`、`Platform` 与 `Presence` 整个搬进本 crate，而且沙箱与凭据保管这两项机器级的读仍然绕过端口直接碰主机，脚本也就换不掉它们。被否决的做法：逐项端口——搬走 doctor 的整个模型，却仍留两条通向主机的路。`bin::doctor::Machine` 是本 trait 的子 trait，给 doctor 自己逐项判定时加一个 `look`，它的第二实现在 doctor 的测试里；它不另设 `install`，安装只有本 trait 这一扇门。
7. **`install` 收 `Runnable`，不收程序名加参数；`Recipe` 与 `Runnable` 因此一起住在本 crate。** 理由：`Runnable` 证明配方是 `Command`，只有与它同住一个 crate 的 `Recipe::command` 能造它，所以一个 `Machine` 实现不会被递到一条打印的或手动的配方。哪些程序可以跑由 `doctor_install` 对 requirement 表的查找决定，不由这个类型决定。被否决的做法：收 `&str` 与 `&[&str]`——拒绝打印配方的规则就只剩每个调用方的自觉。让 `Recipe` 的字段私有、只让表能构造，可以把许可也放进类型，但表住在 `sprawling`、类型住在本 crate，没有一种 crate 布局能便宜地做到。
8. **`effect` 与 `plan_view` 先于 `RunWorker` 搬进本 crate。** 理由：它们只依赖 `kernel`、`city` 与 `collab`，不碰 §7 归属表里的任何一个 `bin` 模块，搬它们不需要新端口，而 worker 搬过来时它们必须已经在这里。被否决的做法：等 worker 整体搬迁时一起搬——那一次改动就同时背着机械的搬移与端口设计，审的人分不开两者。
9. **worker 的决定与读面搬进本 crate，通往主机、网络或终端的做法留在 `sprawling`、经端口交进来（§7 归属表）。** 理由：端口正是 citysim 插第二实现的地方；把一个适配器搬进来，它碰主机的那一步就跟着进了 citysim 驱动的 crate，脚本场景会真的起浏览器、读内存、打开文件管理器。被否决的做法：全部搬进来——本 crate 就要依赖 `thread-priority`、`sysinfo`、浏览器与终端，citysim 换不掉其中任何一个；把 `views` 留在 `sprawling`、经端口交给 worker——`Governance` 由读侧拥有，写侧在写下记录之前就要同步地从它作决定（sprawling-SPEC.md 8-92），端口会把一个 trait 放到决定路径上，并把一份折叠的权威分到两个 crate。
10. **没有状态的主机读写经构造时交进来的 `fn` 指针进来，和 `read_volume` 一样；只有持有状态的适配器（`Machine`、`Connectors`）才是 trait。** 理由：一个只包一个函数的 trait 没有第二个方法可换，脚本场景交一个自己的 `fn` 就够了，而且 `fn` 指针不装箱、不经虚表。被否决的做法：一个把内存、卷、随机令牌、打开文件管理器与浏览器捆在一起的 `Host` trait——这些做法的失败各不相同，脚本为了换掉其中一个就得实现全部。
11. **`relay`、`pool`、`desk` 与 `drive_run` 和 `RunWorker` 在同一次改动里搬。** 理由：它们成环——`relay` 经 worker 的账本写，`pool` 的每条车道跑 `drive_run`，`drive_run` 经 `relay` 写回，`desk` 为 worker 排队命令；先搬其中任何一个，都要一个指回留在 `sprawling` 的 worker 的临时端口，而下一次改动就会删掉它。被否决的做法：一个一个搬、中间架临时端口——每个临时适配器都是一个只活一次改动的第二权威。
12. **装配根留在 `bin::assembly`：`listening`、`attending`、`chain_watch`、`genesis`、生产适配器（`models::GatewayModels`、`mcp::Residents`、`SystemClock`、`doctor::ThisMachine`）与把它们装上 worker 的构造。** 理由：它们起线程、绑端口、打开 vault、造城的目录，是 ARCHITECTURE.md §3 说的知道每个具体类型的那一层；worker 搬走之后，它们对本 crate 的依赖是朝内的。被否决的做法：把 `genesis` 当作 worker 的用例搬进来——它在任何 worker 存在之前造城，并经 `serving::open_vault` 打开 vault，搬进来就要为一次性的建城多开一个端口。
13. **harness 页找程序经 `Views.programs` 这个 `fn` 指针，不经 `Machine`，也不在开城时算好。** 理由：这一问读的是此刻的搜索路径，与 `registry`、`upstream` 同形——没有状态、服务中的城交一次、`None` 就答 `Unavailable`（§12-10）；它不启动任何程序，所以不必等 `DoctorRefresh`。被否决的做法：给 `Machine` 加一个方法——`Machine` 属于 worker，读面拿不到它，而且 doctor 的逐项查法已经在 `bin::doctor::Machine::look` 里，再加一个方法就是第二条查法；在开城时把 harness 的有无算进 `DoctorAnswer`——那是一个线上的形状改动，而且人在 harness 页上装完一个程序，要等到下一次 `DoctorRefresh` 才看得到它。
14. **`views` 搬进来时，本 crate 加 `storage` 与 `runtime` 两条边。** 理由：`views` 折叠的就是 `storage` 的 `HotView`、`Attribution`、`LedgerIndex`，快照起步与 worker 的 `Standing` 共用 `runtime::replay::fold_ledger_dir` 的同一遍；worker 搬过来后本来也要这两条边（§12-11）。被否决的做法：把这两处读经端口交进来——端口会把一份折叠的权威分到两个 crate，与 §12-9 否决的是同一件事；把 `fold_ledger_dir` 挪进 `storage`——那改的是 `runtime` 的公开面，与这次迁移无关。
15. **sprawling-SPEC.md 里写这些模块的节不随模块搬，只改模块路径的拼写；S4 迁 `Spec.lean` 时一次搬进本 crate 的规格。** 理由：两处都是 Markdown 时，搬一次、S4 再改写一次，是两遍约两千行的重写；两份 SPEC 的节号相撞（sprawling 的 8-3、8-6 与本 SPEC 的 8-3、8-6），sprawling-SPEC 自己也有重号，逐节搬要先重新编号，而代码与文档里引用 `sprawling-SPEC.md 8-xx` 的地方都得跟着改。被否决的做法：照 §3 早先的第 5 步逐节搬——8-7、8-8、8-9 那样的小节可以，几十节不行。
16. **`views` 搬进本 crate 之后、worker 搬进来之前，`views` 的测试用 `views::tests::founded` 造城。** 理由：真正的创世（`genesis`）经 `RunWorker` 立起市政厅，worker 搬进来之前它还在 `sprawling`，本 crate 够不到它；`founded` 只写那些测试读到的东西，worker 搬进来时删掉。被否决的做法：把这些测试留在 `sprawling`——它们读 `Views` 的私有字段（`index`）与只在测试里存在的 `Views::answer`，留下就得为测试开公开的门。
