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

worker 的两个读写面 `effect` 与 `plan_view` 已在本 crate；`RunWorker` 的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例与 `views` 仍在 `crates/sprawling/src/assembly` 与 `crates/sprawling/src/views`。

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

- `RunWorker`、它的六个对象、全部用例与 `views` 还没有搬进本 crate，所以 citysim（不依赖 `sprawling`）仍驱动不了一次 dispatch。worker 除了经四个端口与本 crate 的 `effect`、`plan_view` 之外，还直接用到 `bin` 的这些模块：`views`、`serving`、`doctor`、`console`、`held_vault`、`toolkit_broker`、`mcp_stdio`、`mcp_link`、`revealing`、`person`、`home`、`browser_tool`。未定的是每一个随 worker 搬进本 crate，还是留在 `sprawling`、由装配根经一个端口交进来。能定下它的证据是该模块的形状：worker 的决定与读面（`views`、`person`）随之搬，`effect` 与 `plan_view` 已经搬来；通往主机、网络或终端的适配器（`mcp_stdio`、`mcp_link`、`console`、`browser_tool`、`serving`）留下，经端口进来。`views` 的 `Governance` 由读侧拥有，写侧从那里取用（sprawling-SPEC.md 8-92），所以它随 `views` 一起搬。

## 4 现状分析

本 crate 现有三个端口。`ModelFactory` 的生产适配器在装配根（`bin::assembly::models`），第二实现在 `crates/sprawling/tests/model_factory.rs`；`Connectors` 的生产适配器是 `bin::assembly::mcp::Residents`，第二实现在 `crates/sprawling/tests/connectors.rs`；`Clock` 的生产适配器是 `bin::assembly::SystemClock`，第二实现在 `crates/sprawling/tests/clock.rs`。
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
- **worker 读的两处都经 `RunWorker.machine`**：`doctor_install` 的安装与它之后的重看，以及 `DoctorRefresh` 的 `look_at_this_machine`。worker 仍自己拒绝需求表里没有的名字与没有配方的平台，这一步在端口被问到之前。
- **一扇安装的门**：终端的 `sprawling doctor --install` 与 worker 的 `doctor_install` 都经 `accounting::Machine::install` 启动安装程序。`bin::doctor::Machine` 是它的子 trait，只多一个逐项的 `look`，自己不声明 `install`，所以一个装东西的实现只有一处要写，也只有一处能被脚本换掉。
- **固定值**：`RunWorker::new` 与 `over` 装上 `ThisMachine`；`with_machine` 是唯一换掉它的门。
- **依赖**：`report` 交回线上的 `channels::DoctorAnswer`，所以本 crate 依赖 `channels`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-5 accounting::effect：一条效应先成为账本行，再成为这座城（形状 2 值类型）

```rust
// crates/accounting/src/effect.rs —— `architecture.toml` 的 accounting::effect，形状 2（值类型）
pub struct Line { who: String, addr: Address, kind: EventKind, data: Payload }

/// 一张桌子留下的全部效应：它们成为的行，以及行之后才允许发生的变化。
pub struct Landing { lines: Vec<Line>, then: Then }   // 两个字段都是私有的

pub enum Then { Nothing, Deliver(Vec<collab::Signal>),
                       Roadmap { path: PathBuf, text: String }, Shelf(Vec<Filing>) }

impl Landing {
    pub fn signals(Vec<SignalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
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

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只依赖 `kernel` 与 `gateway` 的接口类型。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。
2. **`with_models` 是一个消费 `self` 的方法，而不是 `new` 的第四个参数。** 理由：生产只有一种工厂，`new` 的每个调用方（serve、doctor、测试）都会写同一个 `GatewayModels`；换工厂的只有 citysim 与测试。被否决的做法：`new` 加参数——四个调用点重复同一个值，而这个值只有一个权威。
3. **端口参数是 `Chosen` 与 `Redemption`，不含 dialect 头。** 理由：dialect 头由 `Chosen` 的 dialect 决定，把它交给调用方算，两个调用点就各有一份拼法。被否决的做法：照抄 `gateway::adapter_for` 的三参数签名。
4. **`Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `protocol::Outbound`。** 理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `protocol`，不归 worker。
5. **`Connectors::connect` 取 `&mut self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`。** 理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
5. **`Machine` 回答整页，而不是逐项回答 `look(&Requirement) -> Presence`。** 理由：worker 要的是一页答案与一次安装；逐项的端口要把 `Requirement`、`Detection`、`Family`、`PerPlatform`、`Platform` 与 `Presence` 整个搬进本 crate，而且沙箱与凭据保管这两项机器级的读仍然绕过端口直接碰主机，脚本也就换不掉它们。被否决的做法：逐项端口——搬走 doctor 的整个模型，却仍留两条通向主机的路。`bin::doctor::Machine` 是本 trait 的子 trait，给 doctor 自己逐项判定时加一个 `look`，它的第二实现在 doctor 的测试里；它不另设 `install`，安装只有本 trait 这一扇门。
6. **`install` 收 `Runnable`，不收程序名加参数；`Recipe` 与 `Runnable` 因此一起住在本 crate。** 理由：`Runnable` 证明配方是 `Command`，只有与它同住一个 crate 的 `Recipe::command` 能造它，所以一个 `Machine` 实现不会被递到一条打印的或手动的配方。哪些程序可以跑由 `doctor_install` 对 requirement 表的查找决定，不由这个类型决定。被否决的做法：收 `&str` 与 `&[&str]`——拒绝打印配方的规则就只剩每个调用方的自觉。让 `Recipe` 的字段私有、只让表能构造，可以把许可也放进类型，但表住在 `sprawling`、类型住在本 crate，没有一种 crate 布局能便宜地做到。
7. **`effect` 与 `plan_view` 先于 `RunWorker` 搬进本 crate。** 理由：它们只依赖 `kernel`、`city` 与 `collab`，不碰 §3 列出的任何一个 `bin` 模块，搬它们不需要新端口，而 worker 搬过来时它们必须已经在这里。被否决的做法：等 worker 整体搬迁时一起搬——那一次改动就同时背着机械的搬移与 §3 未定的端口设计，审的人分不开两者。
