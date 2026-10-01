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
| `playback` | 一段历史怎样成为一份可以重算、可以复核的 playback bundle；一个 playback page 怎样分五项查过，导出件怎样整份落下；按 UTC 时间选一段，调用的耗时何时是量出来的，一个提交改了什么、出自哪几次调用；模型调用的耗时，证据怎样只读到 cutoff | 8-12、8-13、8-17、8-25 |
| `trace` | 一个提交是这个 run 哪几次调用的结果，同一栋楼里还有谁在那一段里调用过工具；一份历史逐行折到宣告行时怎样答一个提交 | 8-16、8-25 |

表里的模块全部在本 crate。`RunWorker` 与它的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例住 `worker`；它经构造时收下的一个 `Hands` 碰这台电脑，生产的那一份由二进制的装配根 `bin::assembly::production::hands` 造出（8-11）。

## 2 验收标准

九条断言。前三条在 `crates/sprawling/tests/model_factory.rs`，都驱动一个由 `bin::assembly::hands` 造出、再接收脚本 `ModelFactory` 的 `accounting::worker::RunWorker`：

| 测试 | 它钉住的事 |
|---|---|
| `a_dispatch_reaches_the_model_the_worker_was_handed` | run 的模型走端口。端点指向一个拒绝连接的 loopback 端口，所以一个绕过端口、自己造适配器的 worker 碰不到脚本模型，历史里也就没有脚本写下的那句回答。 |
| `an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone` | 没有 session 的 dispatch 按规则从任务的词里取房间名（sprawling-SPEC.md 8-86），不做命名调用：工厂只被问到 run 的主模型，run 在规则给出的房间里开始。 |
| `a_confidential_building_refuses_before_the_factory_is_asked` | 机密楼的拒绝在 worker 的选择里，不在工厂里：端点不是 loopback 地址时 dispatch 以 `GateDenied` 被拒，脚本工厂一次也没被问到。换掉工厂不能绕开机密。 |

第四条在 `crates/sprawling/tests/connectors.rs`：`a_run_is_offered_the_tools_the_worker_was_handed`。楼的配置写了一个 MCP server，它的命令在任何主机上都不存在；worker 接收了一个脚本 `Connectors`，它给出一个工具。一个绕过端口、自己启动 server 的 worker 起不来这个 server，模型收到的工具表里也就没有那个工具。

第五条在 `crates/sprawling/tests/clock.rs`：`a_worker_stamps_its_lines_with_the_clock_it_was_handed`。worker 接收一个停在固定时刻的脚本 `Clock` 之后写下的每一行，`t` 都是那个时刻；一个自己读墙钟的写点写下的 `t` 是现在的时间。

第六条在 `crates/sprawling/tests/machine.rs`：`a_refresh_counts_the_items_the_machine_it_was_handed_answered`。worker 接收一个脚本 `Machine`，它的回答只有一个条目；`DoctorRefresh` 之后 worker 在诊断里报的条目数是 1。一个自己去问主机的 worker 报的是需求表的全部条目数。测试从 `DoctorRefresh` 进而不从 `DoctorInstall` 进：对着一个绕过端口自己动手的 worker，后者会在宿主上真的启动包管理器。

第七至第九条是验收覆盖，在 `crates/sprawling/tests/acceptance/`，一个测试二进制（nextest 过滤器 `binary(acceptance)`）。它们同样驱动一个由 `bin::assembly::hands` 造出、再接收脚本 `ModelFactory` 的 `accounting::worker::RunWorker`；脚本只替模型说话，工作台、效果层、检查点、账本与书架都是生产件。

| 测试 | 它钉住的事 |
|---|---|
| `every_tool_a_builder_is_offered_is_called_and_answered` | 一栋 `Builds` 楼里的 run，第一次请求收到的每一件工具都被脚本调用一次；每条 `tool_called` 恰有一条同 id 的 `tool_result`；每件工具的回答与它自己的 SPEC 一致，逐件的判定在 `episodes.rs`。该调用哪些工具不写名单，取模型收到的工具表：工作台多登记一件工具而脚本里没有它的一段，这条测试点名那件工具变红。 |
| `every_tool_city_hall_is_offered_is_called_and_answered` | 同一条验收标准，对 `Plans` 楼（市政厅）。多出的 `city` 和 `rules` 一样在效果层被拒（`crates/city/Spec.lean` §8-2b、§8-23）；判定看的是调用之后城根下的目录与楼的 `RULES.toml` 一字未变，不钉拒绝码。 |
| `every_shipped_skill_a_building_admits_is_read_by_name_and_pinned` | 仓库 `skills/` 下每个技能包经 `city::install_skill` 装进城库、由楼的阅览室按名准入之后：`run_started.skills` 按目录顺序列出每一件，哈希等于装入时 `Installed::holding` 报告的 `SKILL.md` 哈希（整包哈希是 CAS 的键，答的是另一个问题，`crates/city/Spec.lean` §8-28）；`read <名>` 交给模型的就是那份 `SKILL.md`；包内附属文件按 `<名>/<相对路径>` 读得到。技能集合取 `skills/` 目录本身，不另写名单。 |

城外工具（浏览器、MCP、桌面）不在这张覆盖表里：它们经端口交进来的路由各有一条测试（`browsers.rs`、`connectors.rs`、`desktop.rs`），在真实浏览器与真实 MCP server 上的行为不由白盒判定。

## 3 假设与歧义

- citysim 有一个场景经脚本的 `Hands`（计数时钟、内存里的 vault）与 `ModelFactory` 驱动一次 dispatch：`tests/proposal_baseline.rs`（citysim D22）。它判的是结局，不判同一场景跑两遍账本逐字节相同，所以 ARCHITECTURE.md §11 的 V6 缺口还差这一步；能定下它的证据是那个场景在 citysim 里逐字节重放。
- 模块从 `sprawling` 搬过来时，它在 sprawling-SPEC.md 里的那一节留在原处，只把模块路径改成新的拼写：`bin::views::x` 写作 `accounting::views::x`，`bin::assembly::x` 写作 `accounting::worker::x`。这些节在 S4 迁 `Spec.lean` 时一次进入本 crate 的规格（§12-15）；8-7、8-8、8-9 是早先整节搬进来的，保持原样。未定的只有 S4 的切分：哪几节归 `views`、哪几节归 `worker`，按 `architecture.toml` 里各行的 `spec` 锚点定。
- `views::mcp_health` 自己用 `agent_protocols::McpLink` 启动一个 MCP server 去问它的健康，不经 `Connectors`。未定的是这次读要不要也经端口：`views` 搬进本 crate 时它照原样搬（`agent_protocols` 本来就是本 crate 的依赖）；能定下它的证据是一个脚本场景需不需要回答 MCP 健康查询。
- `playback` 的导出在 40 万行的城上要 27 s，分段计时里约七成是投影对 cutoff 以内每一行做的 credential 扫描（`kernel::secret::scan`，每行约 60 µs，在 `opt-level = "z"` 下量；读数见 8-25）。范围外的行也要扫，因为配对表（`links`）、`hidden_calls` 与 `policies` 都读一行对读者是否可见。未定的是走哪条路：让 `kernel::secret::scan` 本身变快（它是「什么像凭证」的唯一权威，每个扫描者都受益），还是让投影只扫可见性会被某张表读到的行（判定从一处变成两处）。能定下它的证据是 `kernel::secret::scan` 在这类行上的单行读数：降到几 µs，导出就只剩逐行核对的 5.5 s 与视图折叠，投影不必拆开判定。
- `playback`（8-12、8-13）的两个上限是待测初值：`BUNDLE_MAX_BYTES` 与 `PAGE_MAX_BYTES` 要在多日夹具上量过导出峰值、页面解析与首屏成本才定值，定值的证据是 citysim 的多日场景读数。居民导出位置（8-13）里崩溃留下的暂存文件 `<名>.partial-<pid>` 没有人收走：它不会被当成导出件读（`check` 只认 `.json`/`.html` 的名字），能定下要不要收的是这类文件在真实城里出现的频率。

## 4 现状分析

本 crate 现有四个端口。`ModelFactory` 的生产适配器是 `accounting::worker::models::GatewayModels`，`Connectors` 的是 `accounting::worker::mcp::Residents`，两者都由 `RunWorker` 的构造器装上（§12-18）；`Clock` 的生产适配器是 `bin::assembly::production::SystemClock`，经 `Hands.clock` 交进来；`Machine` 的是 `bin::doctor::ThisMachine`，经 `Hands.machine` 交进来。生产的 `Hands` 只由 `bin::assembly::production::hands` 一处造出（8-11）。四个端口的第二实现依次在 `crates/sprawling/tests/model_factory.rs`、`connectors.rs`、`clock.rs`、`machine.rs`。

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
| `doctor`（`REQUIREMENTS`、`Platform`、`host`、`Presence`、`PATIENCE`、`ThisMachine`） | 经端口：看与装经 `Machine`，需求表的查法经 `RunWorker.recipe_for`（sprawling-SPEC.md 中 `doctor_install` 那一节） | 需求表查找、执行引擎的路径 | 主机上有什么，`bin::doctor` 是唯一权威（本节上文）；`views::lines::harnesses_answer` 找一条命令的程序经 `Views.programs`，生产交的是 `bin::doctor::host::find_program`（8-10）；exec 的主机半经 `Hands.exec_host`（8-11） |
| `monitor::memory::read`、`monitor::volume::read` | 经 `Hands`：`read_memory` 与 `read_volume`，都是 `fn` 指针（sprawling-SPEC.md 8-46-3、8-94；8-11） | 新工作进门时读内存与卷的余量 | 读主机的计数器；读数的类型 `Memory` 与记账线程的计数 `Health` 随 worker 搬进本 crate，读数的做法留在 `bin::monitor` |
| `revealing` | 经端口：`RunWorker` 的 `reveal` 字段，一个 `fn` 指针（sprawling-SPEC.md 8-60） | `Reveal` 在主机的文件管理器里打开一个地址 | 启动主机的一个程序 |
| `browser_tool` | 经端口：`RunWorker::with_browsers` 交进来的 `fn` 指针（sprawling-SPEC.md 8-45-2） | 按楼的规则给 run 的浏览器工具 | 启动浏览器，经 BiDi 说话 |
| `release` | 经端口：`Views.registry`，一个由 `views::served` 放进来的 `fn` 指针（sprawling-SPEC.md 中 `Views.machine` 旁的那一条） | `views` 回答 `NewestRelease` 查询 | 向 npm 注册表发请求 |
| `console` | 留在装配根 | — | 只有 `listening` 用它；它是终端，不是 worker |
| `serving` 的其余部分（`folding`、`output_ring`、`Serving`、`open_vault`） | 留在装配根 | — | 只有 `attending`、`listening` 与 `production` 用它们 |
| `assembly` 的 `production`（`SystemClock`、`hands`、`init_city`、`form_city`）、`listening`、`attending` 的起线程那一半（`spawn_worker`）、`chain_watch` 的起审计线程那一半、`dropping` | 留在装配根 | — | 起线程、绑端口、造生产的手：§12-17、§12-18 |
| `models::GatewayModels`、`mcp::Residents` | 随 worker 搬进本 crate | 新 worker 默认的模型工厂与 MCP 连接表 | 它们只经 `gateway` 与 `agent_protocols` 伸手，而 worker 本来就经这两个 crate 探端点、读 MCP 健康（§12-18） |

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
// accounting::worker::models（形状 4 适配器）
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
// accounting::worker::mcp（形状 4 适配器）
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
// bin::assembly::production（形状 4 适配器）
pub struct SystemClock;                   // 生产：墙钟，唯一被许可的采样点（clippy.toml disallowed-methods）
impl RunWorker {
    pub fn with_clock(self, clock: Arc<dyn accounting::Clock + Send + Sync>) -> RunWorker;
}
```

- **worker 读的每一个时刻都经 `RunWorker.clock`**：它写的行、它排的期限。它量的用时（派活准备、`mcp_tools`、probe 的 `elapsed_ms`）读另一只手 `Hands.monotonic`：城钟会被拨、会跳，一段时长要的是只往前走的钟（sprawling-SPEC.md 8-129-2）。lane 线程从 `DriveContext` 拿到同一个时钟的克隆，所以一个 run 的行与 worker 自己的行读的是同一个钟。worker 调用的自由函数（`captured_until`、`reach_of`）把时刻或钟当参数收下，不自己采样。
- **worker 之外只有三个读点用 `SystemClock`**：`bin::assembly::production::hands` 把它装进 `Hands`；`bin::assembly::listening` 用它取 serve 打开账本的时刻，交给 `folds::fold_city`；`bin::serving::journal` 用它标诊断日志行的时间（那不是城的记录）。worker 的构造器打开账本、`holding` 为 `last_tick` 取起点、`worker::genesis::form` 写创世两行，读的都是交进来的 `hands.clock`，所以一个脚本钟从第一行起就生效。
- **固定值**：生产的 `Hands` 装上 `SystemClock`，测试的 `Hands`（`fixture::hands`）装上读墙钟的测试钟；构造之后 `with_clock` 是唯一换掉它的门。`Send + Sync` 与 `Arc`，是因为 lane 线程与 worker 同时读它。
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

// accounting::worker::commanding::machine
impl RunWorker {
    pub fn with_machine(self, machine: Box<dyn accounting::Machine + Send>) -> RunWorker;
}
```

- **只有 `Recipe::command` 造得出 `Runnable`，它证明的是配方的种类，不是许可。** `Runnable` 的构造函数在本 crate 之外不可见，所以持有一个 `Runnable` 只证明它来自一个 `Command` 配方：打印的配方与手动的配方在 `Recipe::command` 被拒。`Recipe::Command` 的字段是 `pub`，任何 crate 都能拼出一个装任意程序的配方，所以挡住表外程序的是 `doctor_install`（`crates/accounting/src/worker/commanding/machine.rs`）先在 requirement 表里查这个名字：表里没有的名字以 `InvalidArgs` 被拒，端口根本不会被调用。
- **失败**：`install` 原样传 `bin::doctor::running` 的 `AxError`；`Recipe::command` 的拒绝是 `E_TOOL_UNAVAILABLE`，恢复说明人该做什么。端口不另造错误码。
- **worker 读的两处都经 `RunWorker.machine`**：`doctor_install` 的安装与它之后的重看，以及 `DoctorRefresh` 的 `look_at_this_machine`。需求表里没有的名字与没有配方的平台由 worker 交到的 `recipe_for`（生产是 `bin::doctor::recipe_for`）拒绝，这一步在端口被问到之前。
- **一扇安装的门**：终端的 `sprawling doctor --install` 与 worker 的 `doctor_install` 都经 `accounting::Machine::install` 启动安装程序。`bin::doctor::Machine` 是它的子 trait，只多一个逐项的 `look`，自己不声明 `install`，所以一个装东西的实现只有一处要写，也只有一处能被脚本换掉。
- **固定值**：生产的 `Hands`（`bin::assembly::production::hands`）装上 `ThisMachine`，经 `Hands.machine` 交给构造器；构造之后 `with_machine` 是唯一换掉它的门。
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

/// 一跑对共享计划做的事。两种而无第三种：效应按次序重放到盘上那份、每条都还对得上，或有一条认领对不上、一个字也不写（§8-27）。
pub enum Claims {
    Landed(Box<Landing<Closing>>),
    Stale { node: NodeId, released: Box<Landing<Closing>> },   // 第一条对不上的认领报给人；released 关掉本跑已落账的认领
}
impl Claims { pub fn of(effects: &[ClaimEffect], on_disk: &str, path: PathBuf, room: &Address, who: &str) -> Result<Claims, AxError>; }

// 装配层那一扇门（accounting::worker::settling::landing）：每张桌子都走它，`Then` 的 match 穷尽
impl RunWorker { fn settle(&mut self, at: &Assignment, run: RunId, landing: effect::Landing, chain: &KnockChain) -> Result<(), AxError>; }
```

**原因**：先把效应变成账本行、再变成状态，是 Ledger 的定义（`docs/glossary.md`：「Every effect becomes an EventRecord first」；ARCHITECTURE.md §5 步 4）。写反的代价是具体的：先上书架后落账，落账失败就在架上留下一条历史没有的记录；先改共享计划后落账，而 `roadmap_claimed` 是 `storage::hot` 与 `storage::projection` 判断谁拿着哪一行的依据，写进了文件而没落账的认领是一行看上去有人占着、历史里却无人占着的行。

**形状**：先后是类型的性质，而不是写桌子的人的纪律。`Then` 只能从 `Landing::record` 里拿到，而 `record` 先把所有行送进去才返回它；要把顺序写反，得先拿到一个拿不到的值。

- **批而不是逐条**：一张桌子的行全部落完，才轮到它的变化。signal 一支因此先落完所有 `signal_enqueued` 再投递；`deliver` 与 `knock` 都不写账，所以账本字节与逐条交错时相同。
- **计划那一支是全有全无的**，所以它自己一个穷尽枚举 `Claims`：效应按次序重放、只有认领核盘上的状态（§8-27），任一条认领对不上，就一字不写，把那个节点报给人，并用 `released` 里的 `roadmap_released` 行关掉本跑已经落账的认领（sprawling-SPEC.md §8-16 的形制）。效应重放到 `on_disk` 上，而不是写回派活时的副本，所以别的 run 在此期间落下的行保留。记账线程在模型认领时已经拒绝了另一个在飞 run 持有的节点（`accounting::worker::booking`，sprawling-SPEC.md 8-42-8）；`Claims::of` 是后盾，接住从旧副本认领了已被别人落地的节点的 run。
- **归档行不需要先写盘**：账本行要的 `kind`／`day`／`subject` 由 `city::archive_entry` 从入参算出（`crates/city/Spec.lean` §8-9）。不在装配层另算 `day_of`，因为那会是「一条归档记录长什么样」的第二个权威。
- **`raised`（待批项）不进本模块**：它不是桌子交出来的效应，而是驱动期间暂存的项，本身就先落账后改状态。

**pr 那两支不走 `Landing`，理由记在这里**：

- `PrEffect::Opened` 里的 `storage::Checkpoint::land` 先于 `pr_opened` 落账，**但它不是「先动世界」**。它铸出的是那条账本行所指向的 commit，与 `run_started` 之前把 brief 放进 CAS 同形：没有任何记录指向的 git commit 不改变任何人读到的东西。
- `PrEffect::Merged` 先经 `Worktrees::plan_merge` 定下这次合并会落在哪个 commit，干线已经动过的拒绝（`MergeStale`）在这一步就报出，然后才写 `pr_merged`。所以不会有一条 `pr_merged` 是替一次注定被拒的合并写的（sprawling-SPEC.md「合并也排到它那条行后面」）。

**测试**：`what_a_run_changes_is_changed_after_the_line_that_announces_it`（`accounting::worker::driving::tests::ledger`）。一跑归档一条决定、又从共享计划里拿一行；`RunWorker::observe` 的 sink 在一行耐久之后才跑，所以它正是看得见「先」的位置。断言：`asset_archived` 落时书架上还没有它，`roadmap_claimed` 落时盘上那一行还没被拿走；跑完两者都在位。

**影响面**：`accounting` 公开面有 `effect` 模块，因为写它的桌子在 `bin::assembly`；`city` 的 `archive_entry` 与 `collab` 的效应类型是它的入参，所以本 crate 依赖 `city` 与 `collab`（ARCHITECTURE.md §3 的 `depmap`）。

### 8-27 accounting::effect：计划的效应按次序重放，只有认领核盘上的状态（形状 2 值类型；collab D6）

```rust
// crates/accounting/src/effect.rs
pub enum Claims {
    Landed(Box<Landing<Closing>>),
    Stale { node: NodeId, released: Box<Landing<Closing>> },
}
impl Claims { pub fn of(effects: &[ClaimEffect], on_disk: &str, path: PathBuf, room: &Address, who: &str) -> Result<Claims, AxError>; }
```

- **按次序重放，每条在前面几条留下的文本上核。** `Claims::of` 从盘上此刻的那份出发，对每条效应先问 `collab::still_true`，再用 `ClaimEffect::apply` 改文本；本 run 自己的前几条效应因此是后几条的前提，不需要「每个节点只核第一条」的例外。本 run 分了自己握着的一行、再认领其中一片叶子，那条认领核的是拆完的文本，两条都落下。
- **只有认领会对不上。** 认领要那一行仍是 `Not started`；放下与拆分只作用于本 run 握着的那一行（collab D6），握持之前的那条认领已经在重放里核过，落地不为它们另判一个期待状态。「分一行要不要握着它」于是只有桌子那一处判定（`crates/collab/spec/Claim.lean` 的 `land` 与 `admitted_lands`）。
- **过时报第一条对不上的认领，重放就停在那里。** `Stale.node` 是那一行；`released` 照旧关掉本跑每一条已落账的认领。停下而不是接着核：被丢下的那条认领之后可能跟着它拆出的子行，接着核会把还不存在的子行报成「被动过」，而接着重放就得吞掉 `apply` 对一行已被别人改掉的拒绝。
- **装配层**：`accounting::worker::settling::desks` 的 `Stale` 一臂为那一行留一条诊断（`collab::claim_tool`，Refuse 级），其余不变。

**测试**：`a_run_that_splits_its_row_and_claims_a_leaf_lands_both`（`accounting::effect::tests`）；`a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call`（`collab::claim_tool::split_tests`）。

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
- **`[ui]` 一节就是 `PreferencesAnswer` 的序列化**（`crates/wire/Spec.lean` §8-39 第七条）：文件能写的键与答案能说的字段是**同一份声明**，因此本模块只做读与写，不陈述「一项偏好是什么」。一条补丁落在记录上的效果同理，归 `PreferencesAnswer::apply` —— `Chord("")` 是解绑还是绑一个空串，只有一个地方回答。
- **别的节原样留下**：写是一次读-改-写，经 `city::edit_document`（`crates/city/Spec.lean` §8-27）持锁并整份替换。「要么整份要么不动」只有一份实现，人层与城层共用它；再写一份就是给 B-49 立第二个权威。
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

### 8-19 视图的第二份是克隆；一次性重建每行只核对一遍；房间的各段 session（`accounting::views::snapshot`、`accounting::views::snapshot::start`、`accounting::views::sessions`，形状 7 投影；sprawling-SPEC.md 8-144，`crates/wire/Spec.lean` §8-71）

```rust
// accounting::views
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Views { /* 折叠状态，私有 */ }
impl Views {
    pub fn twin(&self) -> Result<Views, AxError>;   // Ok(self.clone())
}
// accounting::views::snapshot::start
/// 一次从已证明的历史起步，和这一路逐行核对过的行数（证明的与折叠的合计）。
pub struct Audited<F> { pub started: Started<F>, pub lines_checked: u64 }
pub fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Audited<F>, AxError>;
// accounting::views::sessions
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomSessions { /* 每个地址一列 wire::SessionLine */ }
impl RoomSessions {
    pub(crate) fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>; // 读不出 session_opened 的载荷即拒
    pub(crate) fn answer(&self, room: &Address) -> wire::SessionsAnswer;
}
```

(a) **第二份视图是克隆。** 折叠线程轮换两份视图（sprawling-SPEC.md 8-99），第二份原先经快照编码复制：编码再解码，再把共享的句柄接回去。`Views` 的每个字段都能 `Clone`：折出来的字段深拷贝；账本索引、计划缓存、金库与停机值都在 `Arc` 里，克隆之后仍与原份共享；`machine`、`registry`、`upstream`、`programs` 照值带过去。所以 `#[derive(Clone)]` 给出的正是 `twin` 原先的结果，字段清单仍只写在 `Views` 的定义里。40 万行夹具城上，编码再解码 350 ms，克隆 22 ms（sprawling-SPEC.md 8-144）。为此 `storage::HotView`、`storage::Attribution`、`Governance` 与 `CommitFacts` 派生 `Clone`。

(b) **一次性重建每行只核对一遍。** `start_audited` 先判起点，再决定要不要先证明：
- 快照合身（`SnapshotStart::Resume`）：先 `storage::prove_chain`（只读记录），`Whole` 才解码快照、折尾部；快照之前的行起步时不看，只有这次证明看。
- 不能用快照（`SnapshotStart::Whole`）：直接从创世全量折叠。全量折叠经 `runtime::replay::fold_ledger_dir` 让每一行过同一个 `LineCheck`，它的判定就是不带记录的证明的判定，先证明一遍只是同一批行多核对一次。

`Audited.lines_checked` 是证明逐行核对的行数加上折叠核对的行数（尾部行数，或从创世折到的最后一行的 `seq + 1`）。没有快照、没有记录时它等于账本行数，不是两倍；快照合身、记录写满时它等于记录之后长出的行数加尾部行数，与历史长度无关。两条都在 `worker::folds::views_start::tests` 以两种规模断言，这是整城重建的回退门；墙钟只进 `budgets.toml`。

(c) **每个地址的各段 session 是视图里的一张表。** `accounting::views::sessions::RoomSessions` 为每个地址存一列 `wire::SessionLine`，`Views::apply` 每行调一次 `RoomSessions::absorb`：`session_opened` 在它的地址下开新的一段；`run_started` 在一个还没有任何一段的地址下开一段 `Dispatched`，否则给当前这一段的 `runs` 加一；任何带地址的记录都把这个地址当前这一段的 `last`、`at` 挪到自己。没有地址的记录不碰这张表。`Query::Sessions { room }` 由 `RoomSessions::answer` 在锁内作答（`crates/wire/Spec.lean` §8-71）。表随视图快照存取，所以视图快照的编码变了，`VIEWS_FOLD_RULES` 随之重取（sprawling-SPEC.md 8-91）；旧快照解不开就按 `WholeFold::Damaged` 从创世折一次，不报错。

### 8-24 快照格式的进位由夹具摘要钉住，夹具里有每一种出现在快照里的摘要（`accounting::views::snapshot`、`accounting::worker::folds::standing_start`，形状 7 投影；`crates/kernel/Spec.lean` §8-84）

```rust
// accounting::views::snapshot
const VIEWS_FOLD_RULES: &str = "views-fold-<16 位十六进制>";       // 视图夹具编码的 blake3 前缀
// accounting::worker::folds::standing_start
const STANDING_FOLD_RULES: &str = "standing-fold-<16 位十六进制>"; // Standing 夹具编码的 blake3 前缀
```

- **门只看得见夹具里有的类型。** 两个常量的后缀是一份固定夹具经快照编码之后的 blake3 前缀，编码一变，`the_fold_rules_name_carries_the_digest_of_the_views_encoding` 与 Standing 的同名测试给出新值并失败（sprawling-SPEC.md 8-91）。夹具里没有 `GitOid`，摘要的编码从十六进制改成字节时摘要不动，旧快照就会以同一个 `fold_version` 交给新的解码：postcard 把一个长度前缀加 40 个十六进制字符当成 20 个字节读，读错的视图可能照样解得开。
- **所以夹具折进每一种出现在快照里的摘要。** 视图的夹具多折一条 `checkpoint_committed`（`commits`、`commit_seqs`、`last_commit` 各存一个 `GitOid`）与一条 `rules_changed`（`governance.rules` 存一个 `B3Hash`）；Standing 的夹具多折同一条 `rules_changed`（它的 `governance` 是同一个类型）。kernel 里摘要的编码再变，两道门都变红，旧快照按 `WholeFold::OtherFoldVersion` 从创世折一次。
- **摘要在快照里是字节。** `views::snapshot::tests` 断言一个 `GitOid` 与一个 `B3Hash` 的快照编码就是它们的 20 与 32 个字节、读回相等，JSON 拼写仍是十六进制（`crates/kernel/Spec.lean` §8-84）。
- **被否：夹具不动，改编码的人记得改常量。** 这正是两个常量带摘要后缀要免掉的那种记忆；而这次的改动在 kernel，改它的人看不见 accounting 的常量。

### 8-11 accounting::worker：城的唯一写者，和它从外面收下的手（形状 1 数据 + 形状 4 适配器）

```rust
// accounting::worker::hands（形状 1 数据）
/// worker 伸向这台电脑的每一只手，构造时一次交进来。
pub struct Hands {
    pub vault: gateway::Custodian,                       // 生产：Custodian::probe 打开的那一个；脚本：Custodian::in_memory()
    pub clock: Arc<dyn Clock + Send + Sync>,             // 8-3
    pub monotonic: fn() -> Instant,                      // 量时长的单调钟：派活准备、mcp_tools、probe 的用时（sprawling-SPEC.md 8-129-2）
    pub machine: Box<dyn Machine + Send>,                // 8-4
    pub read_memory: fn() -> Memory,                     // sprawling-SPEC.md 8-46-3
    pub read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,   // sprawling-SPEC.md 8-116
    pub reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,         // sprawling-SPEC.md 8-60
    pub browsers: Browsers,                              // sprawling-SPEC.md 8-45-2
    pub desktop_program: DesktopProgram,                 // sprawling-SPEC.md 8-4d
    pub recipe_for: fn(&str) -> Result<&'static Recipe, AxError>,
    pub exec_host: ExecHost,
}
/// exec 工具取自这台电脑的三件事。
pub struct ExecHost {
    pub python_wasm: fn() -> Option<PathBuf>,            // 可用的那一份，坏的不算
    pub shell: fn() -> Option<PathBuf>,
    pub engine: fn() -> Result<Box<dyn runtime::Sandbox>, AxError>,
}
pub type Browsers = fn(&Path, &storage::BlockOrigin, &city::BuildingRules) -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;
pub type DesktopProgram = fn() -> std::io::Result<PathBuf>;

// accounting::worker::pool
pub struct Memory { pub physical: u64, pub available: u64 }
// accounting::worker::health
pub struct Health(/* 私有 */);   // 记账线程的两个计数，别的线程可读（sprawling-SPEC.md 8-98）

// accounting::worker
pub struct RunWorker { /* 私有 */ }
impl RunWorker {
    pub fn new(city_root: &Path, log: Diagnostics, hands: Hands) -> Result<RunWorker, AxError>;
    pub fn over(city_root: &Path, log: Diagnostics, hands: Hands, opened: (JsonlLedger, OpenReport)) -> Result<RunWorker, AxError>;
    pub fn holding(city_root: &Path, log: Diagnostics, hands: Hands, held: (JsonlLedger, OpenReport, Standing)) -> Result<RunWorker, AxError>;
    // 换掉其中一只手的门照旧：with_models、with_connectors、with_clock、with_machine、with_browsers、with_desktop_program
}
// accounting::worker::genesis
pub fn form(city_root: &Path, adopt: Adopt, hands: Hands) -> Result<InitReport, AxError>;
// accounting::worker::attend
pub fn attend(worker: &mut RunWorker, desk: &CommandDesk);
// accounting::worker::chain_halt
impl RunWorker { pub fn chain_under_audit(&mut self) -> ChainUnderAudit; }   // 挂上 halt，交回审计线程要的 halt、账本目录与位置
```

```rust
// bin::assembly::production（形状 4 适配器，留在 sprawling）
pub struct SystemClock;                                  // 墙钟，唯一被许可的采样点
pub fn hands(vault: gateway::Custodian) -> accounting::worker::Hands;
pub fn init_city(city_root: &Path) -> Result<InitReport, AxError>;          // form(city_root, Adopt::Nothing, hands(Custodian::probe))
pub fn form_city(city_root: &Path, adopt: Adopt) -> Result<InitReport, AxError>;
```

- **`Hands` 只装直接碰这台电脑的东西。** 墙钟、doctor、内存与卷的计数器、文件管理器、浏览器、正在运行的可执行文件、需求表、exec 的解释器与引擎，以及 vault。它们各自的生产实现住在 `sprawling`，由 `bin::assembly::production::hands` 一处装好。`ModelFactory` 与 `Connectors` 的生产实现不在里面：`GatewayModels` 与 `Residents` 随 worker 住在本 crate，由 `new` 装上（§12-18）。
- **构造器收一个值，不收九个参数。** 生产的调用方写 `RunWorker::new(root, log, bin::assembly::hands(vault))`；换掉一只手写 `Hands { clock: …, ..hands(vault) }`，或者构造之后调原有的 `with_*` 门。
- **worker 读的每一个时刻都经 `hands.clock`**，包括打开账本、`holding` 为 `last_tick` 取起点、`form` 写创世两行的时刻（8-3）。
- **exec 的判定留在 worker，读数来自主机。** `limits.shell` 为假时 worker 根本不问 `exec_host.shell`；「坏掉的不交给 run」是 doctor 的判定，所以 `ExecHost` 的两个路径函数只交可用的那一份（`bin::doctor::host::usable_python_wasm`、`usable_shell`），引擎按本构建带不带 `sandbox` feature 由 `bin::doctor::host::execution_engine` 选。
- **装配根只留自由函数与直接碰主机的生产适配器。** `bin::assembly` 里剩下：`production`（上面四项）、`listening`（占端口、开写者）、`attending`（起写者线程、接上视图折叠线程与广播，交回 vault 与 `Health`）、`chain_watch`（起审计线程并报告）、`dropping`（拖进对话框的文件）。它们只经本节与 8-10 列出的 `pub` 面碰 worker。
- **失败**：构造器与 `form` 原样传账本、CAS、`city` 与 `Standing::fold` 的 `AxError`，本节不另造错误码。
- **测试的手**：`accounting::worker::fixture::hands()` 交一份不碰主机的 `Hands`——内存里的 vault、读墙钟的测试钟、一台什么都没有的机器、宽裕的内存与卷、拒绝的文件管理器、空的浏览器表、拒绝的桌面程序、拒绝的需求表、没有解释器与 shell、`runtime::AbsentSandbox`。要真的某一只手的测试，自己换上那一只。

### 8-18 重开时冻结死掉的 run；上手指南的进度与 `gh` 的候选 ID 从哪一处答（`accounting::worker::genesis::lost`、`accounting::guide`、`accounting::views::answering::github`）

#### 8-18-1 启动扫描为死掉的 run 写冻结

```rust
// accounting::worker::genesis::lost
pub(super) struct OpenRuns { /* 每个还开着的 run：它 run_started 的 seq，与它最近一行的作者 */ }
impl OpenRuns {
    pub(super) fn observe(&mut self, record: &EventRecord);        // 与 DanglingCalls 骑同一遍验链
    pub(super) fn into_drafts(self, t: TimeMs) -> Result<Vec<EventDraft>, AxError>;   // 按 run_started 的 seq 升序
}
// accounting::worker
pub struct ScanReport { /* …既有字段… */ pub frozen_runs: usize }
```

- **哪些 run 死了。** `startup_scan` 只在 `RunWorker::new` 拿到写者锁之后跑（`sprawling resume`），这时这座城没有一次 run 在驱动：账上有 `run_started`、没有 `run_frozen` 的每一次 run，都是上一个进程死时正在跑的。harness run 与模型 run 一样开、一样冻（`crates/runtime/Spec.lean` §8-52），一并计入。
- **次序。** 先补写悬空调用的 `E_TOOL_OUTCOME_UNKNOWN`（它们属于那次 run，要落在它的冻结之前），再为每次死掉的 run 写一行 `RunFrozen::lost()`（`crates/kernel/Spec.lean` §8-82-2），按 `run_started` 的 seq 升序。
- **作者。** 冻结行写成那次 run 最近一行的作者（它的居民），与补写的 `tool_result` 写成那次调用的作者同一条理由：按居民计数冻结的读者（`city::resident` 的档案）不因进程死过而少计一次。只写过 `run_started`、没写别的就死了的 run，用 `run_started` 的作者。`addr` 缺席，与 `Charter::close` 写的冻结行同形。
- **幂等。** 第二次扫描看到的每次 run 都已冻结，什么都不写；`ScanReport.frozen_runs` 是这一次写了几行，`summary` 把它与关掉的调用数并列告诉人。
- **不做的事。** 不写 `handoff_written`：死掉的 run 没留下交接，替它编一份是假话；不起后继：冻结的 run 是历史，接手由人或计划另派（ARCHITECTURE §13.7）。
- 验收：`genesis::lost::tests` 的 `a_run_the_process_died_in_is_frozen_once`（冻结行的载荷、作者与次数，第二次扫描不再写）；崩溃验收（sprawling-SPEC 8-127）钉住城景里那次 run 的最后一行是冻结、结局是 `cancelled`。

#### 8-18-2 上手指南的进度（`accounting::guide`，形状 4 适配器）

```rust
// accounting::guide
pub fn read(city_root: &Path) -> Result<wire::GuideProgress, AxError>;
pub fn put(city_root: &Path, progress: &wire::GuideProgress) -> Result<(), AxError>;
```

- **一份记录，一种文法。** 文件 `CityLayout::guide`（`<城>/.sprawling/GUIDE.toml`）就是 `wire::GuideProgress` 的 TOML 序列化，与 `person` 的 `[ui]` 是 `PreferencesAnswer` 的序列化同一条理（§8-8）：本模块只读与写，什么是一步、什么是标记由线上的类型说。
- **读。** 文件不在即 `GuideProgress::default()`；读不出或解析不了答 `E_CONFIG_INVALID`（文件与原因）或 `E_STORAGE_FATAL`（读不了），视图把两者答成 `Unavailable`，不拿缺省值冒充。
- **写。** 经 `city::edit_document` 整份替换，读者在写的途中只会读到写之前或写之后的那一份；保留子树不存在时先建它。`Command::PutGuide` 由 `worker::commanding::routing` 直接交给 `put`，不写账本行（`crates/wire/Spec.lean` §8-68）。
- 验收：`worker::commanding::tests::guide` 的 `the_guide_keeps_its_progress_across_a_reopen`。

#### 8-18-3 `gh` 的候选 ID 从哪一处答（`accounting::views::answering::github`，形状 1 判定）

```rust
// accounting::views::answering::github
pub(crate) const DEFAULT_HOST: &str = "github.com";
pub(crate) fn github_answer(ask: Option<fn(&str) -> wire::GithubReading>, host: Option<&str>) -> wire::Answer;
// accounting::views::served
impl Views { pub fn ask_github_through(&mut self, login: fn(&str) -> wire::GithubReading); }
// bin::doctor::github（sprawling）
pub(crate) fn login(host: &str) -> wire::GithubReading;
```

- **视图只判两件事。** 问哪台主机（`host` 缺席即 `DEFAULT_HOST`），以及它是不是一个主机名（字母、数字、`.`、`-`、`:`，不以 `-` 开头、不为空）；不是就答 `NotAHost`，不调用交进来的函数。其余交给交进来的那一个，在 `Prepared::finish` 里、快照放开之后调用（§12 第 13 条同形）。没有交的视图答 `Unavailable`。
- **二进制跑 gh。** `bin::doctor::github::login` 经 `doctor::host::find_program("gh")` 找程序，找不到答 `NoCli`；找到就起 `gh api --hostname <host> user --jq .login`，stdin 为空、stdout 接管道、stderr 丢弃、`GH_PROMPT_DISABLED=1`，以固定间隔数次数等它结束（与 `doctor::running` 同一种有界的等），数完仍未结束就经 `running::stop` 停下它。退出码 0 且 stdout 第一行是一个 login 答 `Found`；4 答 `NotLoggedIn`；其余答 `Failed { exit }`；起不来、被停下或 0 却读不出 login 答 `Failed`（起不来与被停下时 `exit` 为 `None`）；超时之后 `running::stop` 也停不下它时答 `Stuck`，带 `stop` 说的原因。退出码到答复的映射是一个纯函数，三种答复各有一条测试。
- 验收：`views::answering::github` 的 `a_github_login_is_asked_of_the_reader_the_views_were_handed`；`bin::doctor::github` 的 `no_gh_on_the_search_path_is_no_cli`、`exit_four_is_not_logged_in`、`a_login_on_the_first_line_is_found`。

### 8-12 accounting::playback：一段历史成为一份可以重算的 playback bundle（形状 7 投影）

`playback` 把一座城 Ledger 的一段折成一份 **playback bundle**：带来源的、字节确定的 JSON，人或 agent 拿它回看一段工作流。它是账务读面上的一个共享投影，CLI（`sprawling playback export/check`，sprawling-SPEC.md 8-126）与以后的居民工具都是它的薄适配器。必须守住的性质的权威是 `crates/accounting/spec/Playback/Select.lean`（选择、次序与去重、cutoff 不读未来）与 `crates/accounting/spec/Playback/Project.lean`（读不到的行不流进派生表、真实关闭的单调性）；本节是接口与做法。

```rust
// accounting::playback
pub const SCHEMA: &str = "sprawling.playback/3";
pub const PROJECTION_RULES: u32 = 4;
pub const BUNDLE_MAX_BYTES: usize = 32 * 1024 * 1024;
pub enum Cutoff { Latest, At(Seq) }
pub struct Request { pub selection: Selection, pub reader: Reader, pub cutoff: Cutoff }
pub struct Bundle { /* 规范字节，私有 */ }
impl Bundle {
    pub fn bytes(&self) -> &[u8];
    pub fn digest(&self) -> B3Hash;
    pub fn events(&self) -> usize;
}
/// 只读：严格校验从创世到 cutoff 的每一行，再投影。
pub fn export(city_root: &Path, request: &Request) -> Result<Bundle, AxError>;
// check、Report、Verdict 与页面见 8-13。

// accounting::playback::select（形状 1 决策）
pub struct Selection { /* 私有：first、last、run、building、span（时间条件，8-17） */ }
impl Selection {
    pub fn everything() -> Selection;
    /// seq 闭区间 [first, last]；first > last 以 E_INVALID_ARGS 拒绝。
    pub fn new(first: Option<Seq>, last: Option<Seq>, run: Option<RunId>, building: Option<Address>) -> Result<Selection, AxError>;
}

// accounting::playback::reader（形状 1 决策）
pub enum Confidential { Withheld, Included }
pub enum Reader { Person(Confidential), Resident(Address) }
```

模块：`playback`（索引、`export` 与 `check` 的入口）、`playback::select`、`playback::reader`、`playback::walk`（严格校验一遍、固定 cutoff）、`playback::project`（折叠）、`playback::links`（关键时刻与消息的两端）、`playback::document`（bundle 的 schema，形状 6 数据）、`playback::encode`（规范序列化、安全嵌入编码、摘要）、`playback::consistency`（一份 bundle 自洽的判定）、`playback::check`。

**快照与选择。**

- **cutoff 在导出开始时固定。** `walk` 经 `storage::LedgerIndex::folding` 读段、经 `storage::LineCheck::advance` 逐行判定，从创世连续走到最后一个完整行（`Cutoff::Latest`）或到给定 seq（`Cutoff::At`，`check --city` 用）；被选择条件排除的行照样过 `LineCheck`，缺行、坏链、重复 seq、版本超前都在这一遍上报错，整个导出失败，不产出任何字节。尾部半行按 storage 既有的读取规则不算一行，导出不修账。本模块不自己数段文件，也不信索引判断缺行。
- **范围内事件**恰好是 seq ≤ cutoff 与选择的交集（`Select.lean` 的 `mem_selected`）：`--from`/`--through` 是含端点的 seq 闭区间，`--run` 比 `record.run()`，`--building` 用 `Address::is_within` 比信封地址（没有地址的行不属于任何一栋楼，`lab` 不匹配 `laboratory`），时间条件比信封的 `t`（8-17）。区间越过 cutoff 时只读到 cutoff，`source.selection` 照原样记下给的条件。合法的空选择输出带范围信息的空 bundle。

**读界与隐去。**

- **一行碰到的楼**：信封地址的楼；它所在 run 的房间的楼（`run_started`/`run_forked` 的信封地址）；它关闭的那一对的打开行碰到的楼（`approval_resolved` 继承 `approval_requested`，`signal_consumed` 继承 `signal_enqueued`，`pr_merged`/`pr_rejected` 继承 `pr_opened`，`run_frozen` 继承 `run_started`）；载荷里任一字符串，只要它是一个 `Address`、头一段是这本账到此为止立过的楼（`building_created` 的 `addr`，或出现过的信封地址的头一段）。楼由 `city::Building::of` 求。
- **判定复用 `kernel::ReadVerdict` 的三臂。** `Reader::Resident(b)` 调 `kernel::address::may_read(b, 楼, 规则)`；`Reader::Person(Confidential::Withheld)` 对每栋楼问同一份规则：`confidential = false` 为 `Open`，`true` 为 `Confidential`，读不了为 `RulesUnreadable`；`Reader::Person(Confidential::Included)` 全部 `Open`。规则经 `city::RulesCache` 读，按楼缓存一次导出。碰到的楼全是 `Open` 的行才可见；另外两臂关闭。规则是导出时刻的规则。
- **凭据扫描仍生效。** 可见的行再过 `kernel::secret::scan`（`storage::hunks` 用的同一个扫描）；命中的行按凭据隐去，只计数，不回显字节。
- **派生表只从可见行求。** 事件、上下文、run、关键时刻、消息、费用与 checkpoint 全由可见行求出；被隐去的行只进 `withheld`：条数、种类计数、关闭的楼名与原因（`confidential`、`rules_unreadable`）、凭据隐去的条数。这些是明示的元数据披露，不宣称隐藏机密活动的存在；自由文字里转述的秘密，地址规则认不出来。
- **不认识的可忽略行**（更新的写者、`ig:true`）只给 seq，列进 `unknown`，载荷一字不出；它们没有可读的 run 与地址，所以 seq 在区间内就列出，不论 `--run`/`--building`。

**bundle 的内容**（`playback::document`，字段按下面的次序写出）：

| 段 | 内容 |
|---|---|
| `schema` | `SCHEMA` |
| `source` | `city`（`storage::Provenance::city_of`，创世行的链哈希）、`selection`（给的条件：`from`、`through`、`run`、`building`、`since`、`until`）、`cutoff`（seq 与该行的 `chain_hash`）、`rules`（`PROJECTION_RULES`）、`reader`（`{"person":"withheld"}`、`{"person":"included"}` 或 `{"resident":"<楼>"}`） |
| `events` | 范围内的可见行，按 seq 升序、各一次：`seq`、`moment`（`EventRecord::moment`，账本版本给出的逐行时刻依据；`null` 是没量过，不是零耗时）、`line`（账本原行，逐字节） |
| `context` | 范围外、cutoff 以内、可见、被引用的行（run 的首行、关键时刻与消息范围外的那一端），同形 |
| `unknown` | 区间内不认识的可忽略行的 seq |
| `runs` | 有范围内事件的 run，取 `accounting::lineage::Lineage` 折到 cutoff 的那一行：`run`、`addr`、`session`、`parent`、`forked_at`、`predecessor`、`first_seq`、`last_seq`、`state`、`unanswered`、`policy`（8-17）；`parent`/`predecessor` 是 `{"run":id}`、`"withheld"`（那个 run 的房间读者读不到）或 `"missing"`（本账到 cutoff 没有它） |
| `moments` | 关键时刻：`family`（`run`、`approval`、`pr`）、稳定键（run id、approval id、`<branch>@<pr_opened 的 seq>`）、`opened`、`closed`、`seqs`（范围内可见的成员） |
| `messages` | 每封信（`signal_enqueued` 的 id）：`from`、`room`、`sent`、`consumed` |
| `calls` | 每次工具调用（run 与调用 id）与每次模型调用（run 与它的 `model_called` 的 seq）：`run`、`callee`、`called`、`answered`、`took`（8-17、8-25） |
| `checkpoints` | 范围内可见、点名一个提交或钉住一份 job 的行，按 seq 升序：`checkpoint_committed` 的 `JobPinned` 写 `{"pinned":{"job":…}}`，`Committed` 写 `{"committed":{"oid","scope","files","base","diff","trace"}}`（后三项见 8-17）；`pr_merged` 写 `{"merged":{"oid"}}`，`oid` 是落地的提交。哪些行点名提交、点名的是哪个 oid，由 `accounting::views::commits::commit_facts` 一处回答 |
| `costs` | 范围内可见行上折的 `storage::Attribution`：`billed_usd_micros`、`by_run`、`unpriced_calls`、`unpriced_tokens`；只涵盖所选可见范围 |
| `withheld` | 见上 |

一端的状态分五种，互不混同：`{"at":seq}`（在范围内）、`{"outside":seq}`（cutoff 以内、范围外，行在 `context`）、`"withheld"`、`"pending"`（关闭端到 cutoff 还没有出现）、`"missing"`（打开端不在本账到 cutoff 的历史里）。关键时刻、消息与调用只在至少一个成员在范围内可见时出现。闭合只看真实的关闭事件（`run_frozen`、`approval_resolved`、`pr_merged`/`pr_rejected`、`signal_consumed`），窗口的右端不是关闭（`Project.lean` 的 `closure_ignores_the_window`）。PR 按 `branch` 与打开它的那一行识别，所以同一分支重开是另一个关键时刻。

**规范字节与安全嵌入。** `playback::encode` 是唯一的序列化：serde 按结构体字段次序写紧凑 JSON，再把字符串里的 `<`、`>`、`&`、U+2028、U+2029 写成 `\u` 转义，所以同一份字节原样放进 HTML 的 `<script type="application/json">` 也不会提前结束那个块。所有 u64（seq、时刻、金额、计数）写成十进制字符串，JS 的 `Number` 不经手它们；`rules` 是小整数。摘要是这份字节的 BLAKE3（`B3Hash::digest`）。读回时先比尺寸上限，再按 `deny_unknown_fields` 解析，再重新编码并与原字节逐字节比较：重复键、多余空白、字段次序、未知字段、非规范的十进制都在这一步被拒。

**自洽与来源复核。** 一份 bundle 自洽，是指它按上一段读得回，并且：`schema` 是 `SCHEMA`；`events` 与 `context` 各自 seq 严格递增、互不相交；每条 `line` 过 `storage::read_line` 且 seq 与条目一致；每个 `{"at":seq}` 指向 `events`，每个 `{"outside":seq}` 指向 `context`；`runs` 的每个 run 在 `events` 里出现过。自洽只说这份文件内部不矛盾，不说它没被改过。复核有两种：

- **与另一份 bundle 比**：两份都自洽时逐字节比较，不同时给出第一个不同的段。
- **与它的城比**：读者由调用入口给出，不信 bundle 自述。`source.rules` 不是本构建的 `PROJECTION_RULES`、`source.reader` 与入口的读者不同、城在 cutoff 之前就结束，都是「复核不了」，并说明要用哪个版本或哪个读者重新导出；否则按 `source.selection` 与 `Cutoff::At(source.cutoff.seq)` 用同一个投影重算并逐字节比较。改摘要、改费用、删事件而保留 `source`，重算的内容就不同。
- 链与摘要不提供签名，也不证明现实世界里的陈述为真：整本账与 bundle 一起被换掉时没有外部信任根。

两种复核与页面的检查怎样分项报出，见 8-13。

**失败。** 全部是 `AxError`：`Selection::new` 的矛盾区间、bundle 读不懂或不规范是 `E_INVALID_ARGS`（action `select a playback range` / `read a playback bundle`）；链上的行按 `LineFault::into_ax` 报（`E_CAS_CORRUPT`、`E_LOG_VERSION_UNSUPPORTED`），recovery 指向 `sprawling replay`；序列化后超过 `BUNDLE_MAX_BYTES` 是 `E_INVALID_ARGS`，recovery 是用 `--from`/`--through`、`--run` 或 `--building` 收窄；规则、payload 读不了按各自的 `AxError` 原样上抛。任何失败都不交回部分的 bundle。错误文字不回显被隐去的内容。

**资源。** 一遍读，一个段的字节常驻；另外常驻的是 `Lineage`（每个 run 一行）、关键时刻与消息的两端（每个键一项）、可能成为上下文的范围外可见行（run 的首行与各对的两端），范围内的投影，以及折到当前行的视图（8-25）。`BUNDLE_MAX_BYTES` 只限最终字节，不限这些常驻量。32 MiB 是待测的初值：多日夹具上的导出峰值与读取成本量出来之前，不把它当作内存上界。

**版本。** bundle 的内容或某张表的求法每变一次，`PROJECTION_RULES` 进一位，于是旧构建导出、本构建读得开的 bundle 复核时报「复核不了」，而不是报「不同」。字段增减是形状的变化，`SCHEMA` 随之进一位：读回时先只读 `schema` 一个键，不是本构建的 `SCHEMA` 就在结构一项报出两个版本、要求用写它的那个版本复核，而不报一条字段缺失。

**模型与实现的比较。** `Select.lean` 的 `scenes` 是一张场景表（选择、cutoff、应选中的 seq），`scenes_agree` 证明模型对表里每一项给出那组 seq；`playback::tests::model` 从同一个 `.lean` 文件读出这张表，对每一项跑生产的 `export` 并比较 seq。表只在 Lean 里写一次。这是行为比较，不是 Rust 实现的精化证明。

### 8-13 accounting::playback 的页面、分项复核与导出件落盘（形状 1 决策，`page`、`landing` 为形状 4 适配器）

agent 把一份 bundle 做成一个自包含的单文件 HTML，叫 **playback page**。它的数据契约、引用规则、导出流程与验证要求写在随发行包发出的 `skills/playback/SKILL.md`；画面不在本节。本节定三件事：产品怎样把 bundle 放进页面，怎样把一份文件查成五个分开的结论，导出件怎样落盘。CLI（sprawling-SPEC.md 8-126、8-132）与居民的城工具 `playback`（sprawling-SPEC.md 8-132）都是这些函数的薄适配器。

```rust
// accounting::playback
pub const PAGE_MAX_BYTES: usize = 48 * 1024 * 1024;
pub const BUNDLE_BLOCK: &str = r#"<script type="application/json" id="playback-bundle"></script>"#;
/// 模板里恰好一处 BUNDLE_BLOCK，换成装着 bundle 字节的同一个元素；结果过结构与静态离线两项才交出。
pub fn embed(template: &[u8], bundle: &Bundle) -> Result<Vec<u8>, AxError>;
pub struct City<'a> { pub root: &'a Path, pub reader: Reader }
pub struct Asked<'a> { pub bundle: Option<&'a [u8]>, pub city: Option<City<'a>>, pub observed: Option<&'a [u8]> }
pub enum Verdict { Passed, Failed { found: String }, Unasked { why: &'static str }, Unable { why: String } }
pub struct Report {
    pub file: B3Hash,
    pub digest: Option<B3Hash>,
    pub events: Option<usize>,
    pub structure: Verdict,
    pub bundle: Verdict,
    pub source: Verdict,
    pub offline: Verdict,
    pub browser: Verdict,
    pub covered: Vec<String>,
}
impl Report {
    /// 没有 Failed，也没有 Unable。
    pub fn holds(&self) -> bool;
    /// 一行 JSON：file、digest、events 与五项。
    pub fn line(&self) -> serde_json::Value;
}
/// 不返回错误：每一项读不下去的原因写在它自己的结论里。
pub fn check(file: &[u8], asked: &Asked<'_>) -> Report;
pub enum Place<'a> { Chosen(&'a Path), Exports { city_root: &'a Path, file: &'a Path } }
pub fn land(place: Place<'_>, bytes: &[u8]) -> Result<(), AxError>;
```

模块：`playback::page`（用 html5ever 的树构建器按浏览器的解析算法读页面，交出一张平面元素表；树构建器写进的 sink 在 `playback::page::sink`）、`playback::offline`（静态离线规则）、`playback::observed`（浏览器观察记录）、`playback::landing`（新文件整份落下或不落）；`playback::check` 把五项排在一起。

**一份文件是什么。** 首字节是 `{` 的文件按 bundle 读；其余按 UTF-8 的 HTML 页面读，不是合法 UTF-8 的页面在结构一项失败。

**嵌入。** `embed` 要求模板里恰好一处 `BUNDLE_BLOCK` 这串字节，把 bundle 的规范字节原样放进这对标签之间，再对结果跑结构与静态离线两项；任一不过就以 `E_INVALID_ARGS` 拒绝（action `embed a playback bundle`），subject 是第一条发现，不交出页面。字节原样放进去是安全的：`encode` 已把 `<`、`>`、`&` 写成转义（8-12），解析器在这个块里遇不到 `</script>`。页面用 `JSON.parse` 读块的文本，u64 仍是十进制字符串，没有一个值经过 JS 的 `Number`。一处之外的写法（零处、两处、标记写在注释或另一个原始文本元素里）都会让结构一项失败，因为结构一项判的是解析器建出来的元素，而不是源文本。

**五项，分开报。** 状态词三个：`passed`、`failed`、`unchecked`。`Unasked` 与 `Unable` 都写作 `unchecked`，区别只在 `why`：前者是没有人要这一项（没给 `--bundle`、`--city`、观察记录，或被查的是 bundle 而不是页面），后者是要了而做不了。`Report::holds` 为真，当且仅当没有 `Failed` 也没有 `Unable`。

| 项 | 名字 | 通过 | 失败 | 未检查 |
|---|---|---|---|---|
| 结构与引用 | `structure` | bundle 自洽（8-12）；页面另有下面四条 | 任一条不成立 | 从不 |
| 与指定 bundle 一致 | `bundle` | 两份都自洽且逐字节相等 | 第一个不同的段 | 没给另一份（`Unasked`）；另一份或这份读不出 bundle（`Unable`） |
| 来源复核 | `source` | 用入口给的读者重算，逐字节相等 | 第一个不同的段 | 没给城（`Unasked`）；读不出 bundle、版本或读者对不上、城在 cutoff 前结束、城的账读不下去（`Unable`） |
| 静态离线 | `offline` | 下面的规则全部成立 | 第一条发现，另计余下的条数 | 被查的是 bundle（`Unasked`） |
| 浏览器观察 | `browser` | 观察记录说的是这份字节，四张表都空 | 第一项观察到的行为 | 没给记录（`Unasked`）；记录读不懂、说的是另一份字节、没有路径（`Unable`） |

页面的结构另有四条，判的是 `page` 交出的元素表：恰好一个元素的 `id` 是 `playback-bundle`，它是 HTML 命名空间的 `script`、`type` 是 `application/json`、文本是一份自洽的 bundle；全页的 `id` 不重复；每个以 `#` 开头的 `href`（含 xlink 的 `href`）指向页面里一个存在的 `id`，单独一个 `#` 除外；每个带 `data-seq` 的元素，值是十进制 seq，且在 bundle 的 `events` 或 `context` 里。由脚本在运行时画出的链接不在源里，它们由浏览器观察的 `unresolved` 表核对。

**静态离线规则**（`playback::offline`）。规则判的是浏览器解析器会建出的元素——SVG 与 MathML 命名空间里的、`template` 内容里的都算——不是源文本：

1. **CSP 在前。** 第一个 `meta http-equiv="Content-Security-Policy"` 的父元素是 `head`，在它之前建出的元素只有 `html`、`head`、`title` 和不带 `http-equiv` 的 `meta`。它的策略有 `default-src`，`connect-src`、`base-uri`、`form-action` 三条恰好是 `'none'`。页面上每一条 CSP 的每一个值都在这张表里：`'none'`、`'unsafe-inline'`、`'unsafe-eval'`、`'wasm-unsafe-eval'`、`data:`、`blob:`、`'sha256-…'`、`'sha384-…'`、`'sha512-…'`、`'nonce-…'`。主机、`'self'`、`*`、`http:` 一类的 scheme、`'strict-dynamic'`，以及取值不是来源表的指令（`sandbox`、`report-uri`、`report-to` 等）都是发现。
2. **不出现的元素。** HTML 命名空间的 `base`、`form`、`iframe`、`frame`、`frameset`、`object`、`embed`、`portal`、`applet`。`meta` 的 `http-equiv` 只许 `content-type` 与 `content-security-policy`，`refresh` 等都是发现。
3. **URL 属性。** 任何命名空间的 `href`、`src`、`poster`、`action`、`formaction`、`data`、`background`、`cite`、`longdesc`、`manifest`、`ping`、`codebase`、`archive` 与 xlink 的 `href`：按 URL 规范去掉首尾的 C0 控制字符与空格、删掉其中的制表符与换行之后，值以 `#` 开头，或 scheme 是 `data`、`blob`。空值也是发现。`srcset`、`imagesrcset` 一律是发现。
4. **CSS。** `style` 元素的文本、任何元素的 `style` 属性、SVG 与 MathML 元素的其余属性，都用 CSS 语法的分词器读（`cssparser`），逐层进入函数与块：`url()`、`src()` 的参数与 `image-set()`、`-webkit-image-set()` 里的字符串按第 3 条判；`@import`、坏的 url token、超过分词器嵌套上限的块都是发现。

静态离线通过，说的只是页面声明的资源与策略：内联 JS 可以给 `location` 赋值、动态建链接，这些路径静态检查看不见。CSP 也不是任意 JS 的沙箱。所以这一项从不说「不联网」；在某些路径下没看到联网，是浏览器观察一项的话。

**浏览器观察**（`playback::observed`）。产品不执行被查的页面，也不打包浏览器。skill 用宿主已有的浏览器或自动化能力打开页面、走它点名的交互路径，把看到的写成一个 JSON（`deny_unknown_fields`，至多 1 MiB）：

```json
{"page":"<页面字节的 BLAKE3，十六进制>","paths":["…"],"requests":["…"],"navigations":["…"],"popups":["…"],"unresolved":["…"]}
```

`requests` 是页面自身之外发出的请求（`data:`、`blob:` 不算），`navigations` 是离开页面的导航，`popups` 是打开的新窗口，`unresolved` 是点了之后什么也没指到的证据链接。`page` 与被查文件的摘要（`Report.file`，复核那一行的 `file`）不同、`paths` 为空，是 `Unable`；四张表都空是 `Passed`，`Report.covered` 是 `paths`；否则 `Failed`，给出第一项。记录是跑浏览器的 agent 自报的：产品核对的是它说的是这一份字节，不核对浏览器真的跑过；它也只说在这些路径下没看到，不说别的路径。

**落盘**（`playback::landing`）。两种落点，一个做法：同一目录写 `<文件名>.partial-<pid>`，`sync_all`，以硬链接落到目标名，删掉暂存文件；任何一步失败都删暂存文件，目标要么整份出现，要么不出现。

- `Place::Chosen(path)` 是人的 `--out`：父目录经 `std::fs::canonicalize` 解开链接之后，路径里任一段是受保护的元数据（`kernel::PROTECTED_METADATA`）则以 `E_OUTSIDE_WRITE_DOMAIN` 拒绝；目标已存在、或目标在某个 git 仓库里且被索引跟踪，以 `E_INVALID_ARGS` 拒绝。被删掉而仍被跟踪的文件名不存在于盘上，落下去却等于改了历史里的那个文件，所以存在与跟踪分开查。
- `Place::Exports { city_root, file }` 是城里的保留导出位置，`file` 在 `CityLayout::playback_exports()` 之下：从城根到 `file` 的每一段经 `storage::WriteTarget::within` 查，链接与 junction 一律拒绝；缺的目录建出来；目标已存在、被跟踪以 `E_INVALID_ARGS` 拒绝；目标在 git 仓库里而没有被忽略，以 `E_OUTSIDE_WRITE_DOMAIN` 拒绝，因为它会进历史。`/.sprawling/` 在城根的 `.gitignore` 里（`crates/city/Spec.lean` §8-21），所以正常的城里这一条成立。
- git 的两问经 `git2` 读：从目标的父目录向上找仓库，找不到就两问都不适用；工作区与目标都先 canonicalize 再求相对路径。

**失败与资源。** `check` 不返回错误；`embed` 与 `land` 的失败是 `AxError`，subject 是第一条发现或目标路径，不交回部分的页面或文件。被查文件的上限是 `PAGE_MAX_BYTES`：bundle 的上限加 16 MiB 留给页面自身与内嵌的字体、图，与 `BUNDLE_MAX_BYTES` 一样是待测初值（§3）。解析一遍建一张平面元素表，常驻量与页面字节同阶；CSS 的嵌套深度由 `cssparser` 的上限截住。

### 8-17 accounting::playback 的时间选择、调用的耗时、运行策略与提交的证据（形状 7 投影；`diff`、`traced` 为形状 4 适配器）

回看一段工作流的人还要四样东西：按 UTC 时间选一段；每次工具调用花了多久，以及这个数什么时候是量出来的；一次 run 要求了什么准入证据；一个提交改了什么、出自哪几次调用。本节定这四样在 bundle 里的形状与求法；泳道与播放怎样画是页面的事（`skills/playback/SKILL.md`）。CLI 与城工具读同一组条件（sprawling-SPEC.md 8-143）。时间条件的性质在 `crates/accounting/spec/Playback/Select.lean`。

```rust
// accounting::playback
pub const DIFF_MAX_BYTES: usize = 64 * 1024;
impl Selection {
    /// 加上时间条件：信封 `t` 落在 `span` 里的行。
    #[must_use]
    pub fn during(self, span: runtime::clock::UtcSpan) -> Selection;
}
/// 一次导出给的时间条件，原文。
pub struct Window<'a> { pub since: Option<&'a str>, pub until: Option<&'a str>, pub day: Option<&'a str> }
impl Window<'_> {
    /// 三项取交集，成为一个 `UtcSpan`；没给任何一项时是不限的区间。
    pub fn span(&self) -> Result<runtime::clock::UtcSpan, AxError>;
}
```

模块：`playback::select`（时间条件与 `Window`）、`playback::links`（调用这一对）、`playback::project`（运行策略、提交证据的接入）、`playback::traced`（一行 checkpoint 持有什么；`Committed` 那一行的调用归属与基准，按读者能看到的写）、`playback::diff`（一个提交从基准到它、逐文件的 diff）。

**时间选择。**

- 一行在选择里，另须它信封的 `t` 在 `[since, until)` 里（`UtcSpan::contains`）。`t` 不随 seq 单调（kernel D10：四种等来的行各记各的时刻，墙钟也会回拨），所以逐行判断，走到 cutoff 为止，不在第一条越过 `until` 的行处停下（`Select.lean` 的 `a_line_whose_time_steps_back_is_judged_on_its_own`）。与 seq 区间、run、楼取交集。不认识的可忽略行仍只按 seq 判（8-12）。
- `Window::span`：`since`、`until` 经 `runtime::clock::parse_iso` 读，只收 `iso` 写出的形状。`day` 是 `YYYY-MM-DD`，展开成 `[那一天 00:00:00Z, 次日 00:00:00Z)`：拼成 `<day>T00:00:00Z` 经 `parse_iso` 读，所以历法与校验仍是 `runtime::clock` 那一份，2 月 30 日照样被拒；上界是下界加一个 UTC 日的毫秒数（checked）。给了几项就交几项：下界取最晚的，上界取最早的，再交给 `UtcSpan::new`，它拒绝上界不晚于下界的区间。所以 `--day` 与 `--since`/`--until` 一起给时是交集，交出空区间就是矛盾的范围，以 `E_INVALID_ARGS` 拒绝；合法而一行都没选中的区间输出带范围信息的空 bundle。
- `source.selection` 多记 `since`、`until`（毫秒的十进制字符串，没给为 `null`）。`day` 只记成它展开的两端，因为复核按区间重算，同一个区间不该有两种写法。

**调用与耗时（`calls`）。**

- 一次工具调用是 links 里的一对（模型调用是另一种对，见 8-25）：`tool_called` 的 `id` 打开、同一 run 里 `tool_result` 的 `tool_use_id` 关闭，与 `views::rounds` 配对用的是同一对键。两端的状态与关键时刻、消息一样是五种之一（8-12）。关闭行继承打开行碰到的楼，所以一条调用碰到机密楼时，它的答复同样隐去。至少一端在范围内可见的调用才出现，按它在范围内的第一行的 seq 升序。
- 行：`run`、`callee`（工具调用写 `{"tool":{"id","name"}}`，`name` 在打开行可见时取它载荷的 `name`，否则 `null`；模型调用见 8-25）、`called`、`answered`、`took`。
- `took` 是 `{"measured":"<毫秒>"}`，当且仅当两端都可见、`views::rounds::answered_timing` 判这一对的时刻是量出来的（两行都有 `EventRecord::moment`，答复不是城在重启后补写的 `E_TOOL_OUTCOME_UNKNOWN`），并且答复的 `t` 不早于调用的 `t`；其余一律是 `"unknown"`。所以版本早于逐行时刻的账本、混合区间里旧版本的那几次调用、补写的答复、还没答、有一端被隐去，都不给耗时，页面也就不会画出零耗时。判定量没量的规则只在 `views::rounds` 一处；逐行的精度仍是每条 `events` 的 `moment`，混合的区间逐行、逐调用各自保留。
- 一条两端都落在范围外、范围内又没有成员的调用，在它关闭时就从「可能成为上下文的范围外行」里删掉：调用占账本行数的大头，留着它们会让一次窄选择的常驻量与整本账同阶；删掉以后常驻的只有还没关闭的调用。

**运行策略（`runs[].policy`）。** 每个 run 行带这个 run 的 `run_started.policy`（`crates/kernel/Spec.lean` §8-77：`mode`、`write`、`admit`、`landing`），早于策略入账的行写 `null`。`admit` 是这次派活要求的准入证据；它的结果是这个 run 打开的 PR 怎样关闭：合并时准入不过写成 `pr_rejected`，`by` 与 `why` 是拒绝它的一方与理由（`crates/runtime/Spec.lean` §8-54），合并写成 `pr_merged`，带 `reviewed_commit` 与 `verified_by`。这些行在 `events` 里，关键时刻 `pr` 一项指向它们。要求与结局各是记下的事实，playback 不从一次合并推断测试跑过没有。

**提交的证据。** 范围内可见的每个 `Committed` checkpoint 多三项；读这三项要读账本之外的输入（git 对象与 `accounting::trace` 对整本账的折叠），确定性的条件是这些输入相同，而 git 对象按 oid 不可变：

- **`trace`**（`playback::traced`）：这一行是 cutoff 以内第一次宣告这个 oid 的行时，是 `accounting::trace` 按这一行求出的答（8-25：视图折到这一行时的 `Query::Commit`，调用与同楼的别人经 walk 的索引读），写 `{"traced":{"calls","nearby"}}`：`calls` 是这个 run 在区间里的每次调用，各是 `{"at":seq}`（在 `events` 里）、`{"elsewhere":seq}`（读者看得到，在选择之外，行不随 bundle 携带，要看它就把选择放宽到它）或 `"withheld"`（读者看不到那条 `tool_called`）；`nearby` 是同一栋楼里别的 run 的调用数，各是 `{"run","actor","calls"}`，`run` 照 `runs` 的写法是 `{"run":id}`、`"withheld"` 或 `"missing"`，`actor` 只在 `run` 读得到时写出。这一行之前已经宣告过同一个 oid（同一棵树再次宣告，它的调用归在第一次宣告上），或视图答「没有这个提交」时，写 `"untraced"`；`trace` 失败（视图拒绝折 cutoff 以内的某一行，或这一段的行读不了）写 `{"unread":"<错误码>"}`，导出不因此失败。这两种情形下没有 `trace` 的答，`base` 是 `"none"`，`diff` 为空表。
- **`base`**：`trace` 答的 `commit.previous`（同一 run 的上一个提交），写 `{"previous":oid}`；没有时，`commit.parents` 恰好一个，写 `{"parent":oid}`；否则 `"none"`。全城紧邻的上一个提交不是基准：它可能属于别的 run。
- **`diff`**（`playback::diff`）：`base` 是 `"none"` 时为空表；否则按 `files` 的次序，每个路径一项 `{"path","change"}`。`change` 是六种之一，互不混同：
  - `{"patch":{"lines","credential"}}`：经 `storage::hunks::of_file(city_root, base, Head::Commit(oid), path)` 读出的整段 patch。它只比较两个不可变的 oid，从不读工作区；`lines` 是 `{"number","text"}`，`credential` 是被凭据扫描隐去的行的 `{"number","reason"}`，与 `storage::hunks` 同一个扫描，不回显字节。
  - `{"truncated":{"lines","credential","cut"}}`：这一个提交显示的 patch 文字超过 `DIFF_MAX_BYTES`，这个文件只给出预算之内的头几行，`cut` 是没给出的行数。之后的文件照样读、照样分类，只是它们的行都在预算之外。
  - `"empty"`：两个提交之间这个文件没有动。
  - `"binary"`：patch 没有文字行，而提交（文件被删时是基准）里的这个 blob 是二进制。
  - `"missing"`：仓库不在城根，或它没有这两个对象之一。
  - `"withheld"`：这个路径所在的楼对读者关闭，文件不读。
  预算按 UTF-8 字节计，只限 bundle 里 diff 的文字，不限 git 读取本身。

**失败。** `Window::span` 的失败是 `E_INVALID_ARGS`：`since`/`until` 是 `parse_iso` 的拒绝；`day` 读不了时 action 是 `select a playback day`，subject 是原文，recovery 给出 `2026-05-14` 的写法；交集为空时是 `UtcSpan::new` 的拒绝。`diff` 与 `trace` 从不让导出失败：账本之外的输入缺了、读不了，各有一个写明的状态。

**资源。** 时间条件只多读每行信封的 `t`，walk 已经解析了它。调用表常驻每个有成员的调用一项，加上还没关闭的范围外调用。每个范围内、第一次宣告的 `Committed` 提交问一次折到它的视图、按 walk 的索引读一段（代价见 8-25），再对 `files` 里每个路径读一次 git：一份 patch 一次物化，显示的部分不超过 `DIFF_MAX_BYTES`。

**本节测试**：`accounting::playback::tests::span`：`t` 回退的行按它自己的时刻取舍；`day` 与 `since` 取交集，交出空区间时拒绝，`day` 读不了时拒绝；不重叠的合法区间给出空 bundle 且 `source.selection` 记着两端。`accounting::playback::tests::model`：`Select.lean` 场景表里带时间的几项在生产的 `export` 上给出同样的 seq。`accounting::playback::tests::evidence`：旧版本的调用与新版本的调用在同一区间里，前者 `took` 为 `"unknown"`、后者是量出来的毫秒，补写的答复为 `"unknown"`；run 行带它的策略，PR 的拒绝行在 `events` 里；一个提交的 `diff` 把文字、凭据行、空、二进制、缺失、隐去分开，基准是同一 run 的上一个提交；它的 `trace` 把区间里的调用按读者能看到的写出；导出之后别的 run 再宣告同一个提交，复核仍逐字节相同。

### 8-25 accounting::playback 的模型调用耗时，与只读到 cutoff 的提交证据（形状 7 投影；`views::rounds::Attempts` 为形状 1 决策）

8-17 给工具调用配了耗时，给提交配了调用归属。这一节补两样：模型调用（一条 `model_called` 到答它的 `model_returned`）的耗时；提交的证据只读到 cutoff——cutoff 之后的行写了什么、审不审得过，都改变不了 bundle，所以 `check --city` 在一段 cutoff 之后才坏掉的历史上照样逐字节重现。必须守住的性质在 `crates/accounting/spec/Playback/Project.lean`：`took_is_measured`、`an_unmeasured_end_is_unknown`、`evidence_ignores_lines_after_the_cutoff`。

```rust
// accounting::views::rounds（形状 1 决策）
/// 每条 `model_returned` 答的是它的 run 在它之前最近的一条 `model_called`。
#[derive(Debug, Default)]
pub(crate) struct Attempts { /* 私有：每个 run 最近一次尝试的 seq */ }
impl Attempts {
    pub(crate) fn note(&mut self, record: &EventRecord);
    pub(crate) fn answered_by(&self, reply: &EventRecord) -> Option<Seq>;
}

// accounting::trace
/// 一座城的历史，按严格校验走过的次序一行一行折进视图。
pub(crate) struct History { /* 私有：折到当前行的视图，或第一次拒绝 */ }
impl History {
    pub(crate) fn new(city_root: &Path) -> History;
    pub(crate) fn absorb(&mut self, record: &EventRecord);
    /// 折到当前行为止的视图答 `Query::Commit` 的那个值；没有一行宣告过它时 `Ok(None)`。
    pub(crate) fn commit(&self, oid: GitOid) -> Result<Option<wire::CommitAnswer>, AxError>;
}
pub(crate) fn trace_through(index: &LedgerIndex, ledger_dir: &Path, commit: wire::CommitAnswer) -> Result<Trace, AxError>;
```

模块：`views::rounds`（`Attempts`）、`playback::links`（调用的第二种对）、`playback::document`（`Callee`）、`trace`（`History`、`trace_through`）、`playback::walk`（交回它建的索引）、`playback::traced`（`Evidence`：在 walk 里折历史，记下每个第一次宣告的答）。

**模型调用（`calls`）。**

- 一次模型调用是 links 里调用的第二种对：键是 run 与那条 `model_called` 的 seq；关闭它的是同一 run 的 `model_returned`，它答哪一条由 `views::rounds::Attempts` 判：这个 run 在它之前最近的那条 `model_called`。rounds 的 `turns` 把答复放进回合时也经 `Attempts`，所以页面的回合与 bundle 的调用用同一条配对规则。每次尝试都入账，修复后的重发是第二条 `model_called`（`crates/runtime/spec/Turn/Recovery.lean` §8-50），被它替下的那次尝试不会有答复，写 `"pending"`，`took` 为 `"unknown"`。
- 行：`run`、`callee`、`called`、`answered`、`took`。`callee` 是两种之一：`{"tool":{"id","name"}}`（工具调用，8-17）或 `{"model":{"name"}}`（模型调用：打开行可见时是 `ModelCalled.model`，请求里写的端点自己的模型 id，否则 `null`）。
- `took` 与工具调用同一条规则（8-17）：两端都可见，`views::rounds::answered_timing` 判两端量过（两行都有 `EventRecord::moment`），答复的 `t` 不早于调用的 `t`，才写 `{"measured":"<毫秒>"}`，其余一律 `"unknown"`。模型的答复不带 `error`，所以「城在重启后补写的答复」这一条对它不成立；规则仍只在 rounds 一处。`ModelReturned.first_at` 是首个内容到达的时刻，不进 `took`：它答的是另一个问题（首字延迟），那一行在 `events` 里，页面要时自己读。
- 两端都在范围外、范围内没有成员的模型调用，与工具调用一样在关闭时从候选里删掉（8-17）。

**只读到 cutoff 的提交证据。**

- `playback::walk` 交回它经 `storage::LedgerIndex::folding` 建出的索引，`project` 把它交给证据那一步。cutoff 只有一处定义，是 walk 停下的那一行（8-12）；证据那一步不另读一遍账本，也不另建索引。
- `playback::traced::Evidence` 在 walk 里收下每一条核对过的行（`Walked::Known`）：先折进 `trace::History`，再记下这一行是不是第一次宣告它点名的提交（`views::commits::commit_facts`）。范围内可见的 `Committed` checkpoint 是第一次宣告时，在折完这一行的那一刻问 `History::commit(oid)`：此刻的视图恰好折到这一行，`previous` 是这个 run 在它之前宣告的最近一个提交，run、`actor`、`seq` 就是这一行自己的，`parents` 由 git 按 oid 给出。答只取决于这一行与它之前的历史，以后的宣告与 cutoff 之后的行都改变不了它（`evidence_ignores_lines_after_the_cutoff`）。
- walk 结束后，每个这样的答交给 `trace::trace_through`，经 walk 的索引读这个 run 在这一行之前的行与区间里的行，区间规则是 8-16 那一条；读到的 seq 全都小于这一行，所以不越过 cutoff。
- `History::absorb` 遇到视图拒绝折的第一行时停下，记住那条拒绝；之后每次 `commit` 都答它，这些提交写 `{"unread":"<错误码>"}`，导出不因此失败（8-17）。视图折的是 walk 已核对过的同一批记录，被拒绝的行在 cutoff 以内，所以一份 bundle 里的 `unread` 只取决于 cutoff 以内的历史，复核时照样重现。
- `whose --trace`（sprawling-SPEC.md 8-136）不变：`trace(city_root, oid)` 照旧经 `views::ask` 求这个 oid 最近一次的宣告、重建一次索引，再经 `trace_through` 按同一条区间规则读。

**代价。** 一份 bundle 只折一遍视图，与 walk 同一遍、同一批记录；它从不调用 `views::ask`，也不重建索引。每个范围内第一次宣告的提交另有一次视图查询（在内存里）、一次打开仓库读父提交（`storage::parents_of`），再按索引读它的 run 的行（倒着读到下界前最近的 `model_called`）与区间里的行。判同楼的别人时从区间的下界起读索引（`LedgerIndex::seqs_from`，`crates/storage/Spec.lean` §8-38），读到区间之后的第一行为止：一个提交读过的索引项是它的区间长度加一，与它之前的账本有多长无关，一次导出在这一项上的代价是各区间长度之和，不是提交数乘行数。确定性计数：`accounting::playback::tests::tracing` 在 N 与 2N 个提交的历史上各导出一次，数这次导出开始了几次视图折叠，两种规模下都是 1；再数一个提交判同楼的别人时最多读过几条索引项，两种规模下都是 6（第一个提交的区间从它的 run 的第一行起，五行，加上区间之后那一行）。两个数都只在测试里编译（`trace::counted`），理由与第 37(c) 条相同：要挡住的是按 N 增长的代价，墙钟在小夹具上看不出它。毫秒读数由同一文件的仪表 `instrument_evidence_cost` 给出（`cargo nextest run -p sprawling-accounting --release --run-ignored only -E 'test(instrument_evidence_cost)' --no-capture`）：50、100、200 个提交（1,003、2,003、4,003 行）的一次导出依次约 100–115、160–170、365–430 ms，随提交数线性增长。

**在 40 万行的城上。** 读数取自 40 万行夹具城（`bench_startup first-byte` 的 `l400k`：400,003 行、64,000 个提交；windows-x86_64、16 线程的笔记本处理器、release，`opt-level = 3`）：`sprawling playback export` 选 `--from 2 --through 8`（范围里没有提交）26.8–27.2 s，选 `--from 399000`（1,003 行、约 160 个提交）26.9–27.0 s，选 `--from 380000`（20,003 行、约 3,200 个提交）31.2 s，各三次。同一座城上分段计时（在 `opt-level = "z"` 下量，那时同一次导出是 34 s）：只走 walk（逐行核对）5.5 s；walk 加视图折叠 6.6–7.3 s，所以视图折叠约 1.1–1.8 s，是一次导出的 4% 上下；walk 加整个投影 33–37 s，其中 credential 扫描约 25 s、`named_in_payload` 2.3 s、视图折叠 1.4 s、`links` 0.9 s。导出的秒数因此不在视图折叠里，而在投影对每一行的扫描（§3），范围里没有提交时不折视图最多省下这 4%（第 37(b) 条）。范围里每多一个提交，另付约 1.3 ms（读父提交、区间里的行与同楼的别人）。

**失败。** `History::commit` 的失败是视图拒绝折某一行的那条 `AxError`；`trace_through` 的失败是读行的 `StorageError`（经 `into_ax`）、一行解析不了、actor 落在保留子树里（同 8-16）。两者在 playback 里都写成 `unread`。

**本节测试**：`accounting::playback::tests::tracing`：旧版本的一次模型调用、被重发替下的一次尝试、量过的一次与没有答复的一次在同一区间里，`took` 依次是 `"unknown"`、`"unknown"`（答复端 `"pending"`）、量出来的毫秒、`"unknown"`；导出之后在 cutoff 之后追加两行、改坏其中第一行，`check --city` 的来源一项仍通过；N 与 2N 个提交时一次导出都只开始一次视图折叠，一个提交判同楼的别人时读过的索引项也不随 N 变。`accounting::playback::tests::evidence` 的调用表照 `callee` 的形状写。

### 8-15 页面要的几样新东西，从哪一处答（`accounting::views::answering`、`accounting::worker::commanding`、`accounting::worker::freezing`）

**身份。** `Query::Identity` 在锁外读两份治理文档（`city::read_naming`），答 `StatedIdentity` 或带行号的 `Unreadable`（`crates/wire/Spec.lean` §8-59）。`PutDocument` 与 `PutIdentity` 由 `commanding::governing` 执行：先经 `city` 带基线落盘，再写一行 `governed_document_written`，写 `MAYOR.md`／`PREFERENCES.md` 时 `naming` 是落盘之后此刻的身份版本。

**一个 session 冻一版身份**（`worker::freezing::naming`）。冻前缀时先看房间这一层有没有 `[identity] version`：有，就从内容库读回那一版（读不回即拒 `E_STORAGE_FATAL`，不悄悄换成此刻的名字）；没有，就读此刻的身份，放进内容库，写进房间这一层。city 段是 `City.md` 之后接 `Naming::context()`，resident 段对 `hall/mayor` 以冻下的名字开头，`RunPlan.naming` 是那一版的摘要。所以同一个 session 的每次 run 请求里的名字一样，`/new` 之后的第一次 run 换成此刻的名字，页面经 `run_started.naming` 读回的是请求里真正用的那一版。

- 验收：`worker::freezing::tests::naming` 的 `a_new_session_freezes_the_name_the_page_shows`。

**楼规与城一层。** `PutRules` 由 `commanding::configure` 执行：`city::write_rules_against` 落盘之后，读回文件的摘要，与折叠里这份文件上一次的摘要比，记一行 `rules_changed`。`ConfigureCity` 同样落盘之后记 `rules_changed { scope: city, which: Config }`。两条都只在有一项写了时记行；什么都没写的 `ConfigureCity` 什么都不记。`PreferencePatch::CorePriority` 由 `person::put` 落在 `[core]`，其余臂照旧落在 `[ui]`；`CorePriority` 的值集是 `wire::CorePriority`，`person` 再导出它，`bin` 的调用方不必改路径。

### 8-20 工作台的两件读外来字节的工具：`ocr` 与 `transcribe`（`accounting::worker::workbench::tools::ocr`、`…::tools::transcribe`，形状 4 适配器；sprawling-SPEC 8-131、8-142）

```rust
// accounting::worker::workbench::tools::endpoints（登记里的一段）
impl Laying {
    // 造一个 BoundReader，按序交回这座楼可用的 `transcribe` 与 `ocr`；lay_out_workbench 把它们接在按用途加的工具之后。
    pub(super) fn endpoint_tools(&self, site: &Site, bound: &runtime::ReadBound)
        -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;
}
// accounting::worker::workbench::tools::{transcribe, ocr}
impl Laying {
    pub(super) fn transcription_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<TranscribeTool>, AxError>;
    pub(super) fn ocr_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<OcrTool>, AxError>;
}
pub(super) struct TranscribeTool { /* reader、transcriber: Mutex<Transcriber>、meta —— 私有 */ }
pub(super) struct OcrTool { /* reader、policy、recogniser: Mutex<Recogniser>、meta —— 私有 */ }
// 两件都是 kernel::Tool；参数 `{ path }`；答 `{ path, text }`
```

- **两件工具读字节只经 `runtime::BoundReader`**（`crates/runtime/Spec.lean` §8-59）。`endpoint_tools` 用交给 `read` 与 `search` 的同一个 `ReadBound`、同一个 run 的树根与城的块仓造一个 `BoundReader`，克隆给两件工具；它是工作台登记里自成一段的一步，因为这两件工具共用这一扇门，而 `lay_out_workbench` 已在函数与文件的长度上限边上。本 crate 不判路径：`Address::is_within`、`Address::is_reserved` 与 `storage::WriteTarget::within` 曾在 `transcribe` 里替这扇门判，门落地后删去。
- **有没有这件工具，是一次 `select`。** `transcribe` 读 `ModelTag::Transcribe`，`ocr` 读 `ModelTag::Ocr`，都按 run 所在那座楼的楼规（`site.rules.policy()`）问端点账本；拒了，工具不上表。设施由 gateway 造：`gateway::transcriber_for` 与 `gateway::recogniser_for`，后者带上 `credentials::dialect_headers` 给这个 face 的头，与主模型的适配器同一张。
- **容器的认法：** 图按 `runtime::pipeline::connector::png_picture` 认（读界判过的字节整份读进来，再交它），录音按 `Named` 分：文件看扩展名（`gateway::AudioType::of_file_name`，`file:` Locator 也是文件），块看开头的字节（`gateway::Recording::read_unlabelled`）。
- **`ocr` 的设施在一把锁后面**，理由同 `transcribe`：凭据解析器是 `Send` 而不是 `Sync`，`recognise` 又要 `&mut`；同一个 run 的两次 OCR 轮流进行。
- **在表上的位置**：`transcribe` 之后、`playback` 之前，两件都在内置那一段（sprawling-SPEC 8-142）。
- 验收：两件工具各自模块的测试经一个没有设施的工具判拒绝（别楼的机密路径、reserved subtree、认不得的容器、不在的文件、没有任何楼的 `cas:`），设施的拒绝原样交回；接上设施的那一半由 `crates/sprawling/tests/acceptance/` 的 `ocr` 与 `transcribe` 测试经回环端点证明。

### 8-16 accounting::trace：一个提交倒推到它之前的那些调用（形状 7 投影）

`trace` 回答拿着一个提交 oid 的人在 `whose` 之后问的下一句：这个提交是哪几次调用的结果。CLI（`sprawling whose --trace`，sprawling-SPEC.md 8-136）是它的薄适配器；下一轮 playback 的调用归属与验收工具从坏提交归因到写它的居民，都读同一个值。

```rust
// accounting::trace
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub commit: wire::CommitAnswer,   // 与 `Query::Commit` 的答是同一个值：run、actor、previous、parents
    pub calls: Vec<wire::Call>,       // 这个 run 在区间里的调用，按 seq 升序
    pub nearby: Vec<Nearby>,          // 同一栋楼里别的 run 在区间里的调用，每个 run 一项，按 run id 升序
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby { pub run: RunId, pub actor: Address, pub calls: u64 }
/// 只读。这座城没写过这个提交时 `Ok(None)`。
pub fn trace(city_root: &Path, oid: GitOid) -> Result<Option<Trace>, AxError>;
/// 经 `index` 读 `commit` 的 run 的行与区间里的行；只读 seq 小于 `commit.seq` 的行。
pub(crate) fn trace_through(index: &LedgerIndex, ledger_dir: &Path, commit: wire::CommitAnswer) -> Result<Trace, AxError>;
// 逐行折到宣告行时答 `Query::Commit` 的 `History` 见 8-25。
```

- **提交的事实从 `views::ask` 来。** `trace` 先问 `Query::Commit`，这一步审计整条链、折叠视图（sprawling-SPEC.md 8-41），所以「这座城写没写过它」与 `whose` 同一个答案；答不是 `Answer::Commit` 时就是 `Ok(None)`。
- **区间。** 上界是宣告这个提交的那一行（不含）；下界是 `commit.previous` 那一行（不含），没有 `previous`（这是这个 run 的第一个提交）时是这个 run 的第一行（含）。`previous` 是同一个 run 上一个宣告的提交（`views::commits` 的折叠）：别的 run 在中间的提交不是界，git 的父提交也不是，因为父提交可能出自别的 run 或人自己。
- **调用。** 这个 run 在区间里的 `tool_called`，经 `views::turns` 与各自的 `tool_result` 配对成 `wire::Call`：工具、subject、参数、结局、输出、`effect`、两个时刻与 `timing` 的读法只有 rounds 那一份。`turns` 只把一条 `model_called` 之后的调用归进回合，而区间的下界可以落在一个回合中间，所以往回读过下界之后，继续读到下界之前最近的一条 `model_called` 为止（含），折完再按 `Call.at` 只留区间里的调用。读的是本 run 的行（`LedgerIndex::run_seqs_before`），不读别的 run。
- **同楼的别人。** 区间里信封 `run` 不是这个 run、信封地址在 `city::Building::of(commit.actor)` 那栋楼里（`Address::is_within`）的 `tool_called`，按 run 计数；`actor` 是这个 run 在区间里第一条这样的行的地址。同一栋楼共用一棵工作树，这些 run 的写也可能落进这个提交，所以它们是本 run 之外的候选。只计数，不列调用。
- **第一次宣告。** 同一个 oid 可以被再次宣告（一次没有改动的检查点交回同一棵树），别的 run 也可以宣告它；视图为这个 oid 记的 run、地址与 seq 是最近那一次，`previous` 是第一次时的那个。`trace` 回答的是最近那一次。playback 要的是第一次宣告的那一行：它在视图折到那一行时问 `History::commit`，此刻视图里这个 oid 的 run、`actor`、`seq` 就是那一行的，再经 `trace_through` 按同一条区间规则求；所以它的答只取决于那一行及其之前的历史，以后的宣告与 cutoff 之后的行都改变不了它，playback 的复核才能逐字节重现（8-17、8-25）。
- **代价。** `ask` 之后再建一次 `LedgerIndex`，再按索引读两段：本 run 的行（倒着读到下界前最近的 `model_called`）与区间里的每一行（判同楼的别人）。读的行数与区间长度成正比；建索引与 `ask` 的审计各与账本字节数成正比，与一次性的 `whose` 同阶。playback 不走这条路：它不调用 `ask`、不重建索引，只按 walk 建的索引读这两段（8-25）。
- **失败。** `ask` 的失败原样上抛；建索引与读行的 `StorageError` 经 `into_ax`；一行解析不了按 `EventRecord::parse_line` 的错误上抛，审过的链上出现它，说明账本在 `ask` 之后被改了；actor 落在保留子树里时 `Building::of` 的拒绝上抛（城写的提交不会落在那里）。

### 8-23 accounting::views::answering::preview：一个 Markdown 版本的一个窗口读成块（形状 7 投影）

```rust
// accounting::views::answering::preview（锁外，sprawling-SPEC.md 8-100）
impl Views { pub(in crate::views) fn preview_ask(&self, version: B3Hash, viewport: documents::Span) -> Prepared; }
pub(in crate::views) fn preview_answer(city_root: &Path, version: B3Hash, viewport: documents::Span) -> wire::Answer;
```

- **读内容库，判定全在 `documents`。** 读内容库的那一步与 `Range` 是同一个函数（`range::stored`，§8-21）；然后 `documents::preview` 判编码、切窗口、止于块末、读出块（`crates/documents/Spec.lean` D20–D26）。本模块不写一条判定，所以预览、`Range` 与 `Document` 对「这一版是什么编码、窗口在哪里切」只有一个答案。
- **答复。** 读出来就是 `Answer::Preview`：`Laid` 带实际读的区间与块，UTF-16 的版本是 `Unsupported`（`crates/wire/Spec.lean` §8-74）。内容库没有这一版、打不开、或抬起的字节在这种编码下拼不出文本，答 `Unavailable { query: "Preview(<version>)" }`，与 `Range` 同一个口径：「我没能看」。
- **代价。** 只读内容库里这一窗要的那几个字节，加一次 comrak 读一个至多 64 KiB 的窗口；读数还没有，属 refrain 路线图 A11 的那一组。
- **Markdown 的版本都在内容库里。** 答 `Document` 的读面把每一个 Markdown 版本放进内容库，不论它的第一个窗口盖不盖得住整份（§12 第 33 条），所以从 `Document` 打开的任何一份 Markdown 文件都能按版本预览。
- 验收：`views::answering::preview::tests`——内容库里没有的版本答 `Unavailable`；超过一个窗口的 Markdown 文件经 `Document` 打开后，从 0 起按答复的 `span.end` 逐窗预览，每一窗止于块末，读到末尾时每一段恰好出现一次；整份放得下一个窗口的 Markdown 文件经 `Document` 打开后预览出它的块（`a_short_document_previews_by_its_version`）；字节不是文本的版本答 `Unavailable`；UTF-16 的版本答 `Preview::Unsupported`。

### 8-29 accounting::views::answering::reply：一段回复读成块（形状 7 投影）

```rust
// accounting::views::answering::reply（锁外，sprawling-SPEC.md 8-100）
pub(in crate::views) fn reply_answer(text: &str, state: documents::ReplyState) -> wire::Answer;
// Views::prepare 的一臂：Query::Reply { text, state } → Prepared::Reply { text, state }
```

- **判定全在 `documents::reply`**（`crates/documents/Spec.lean` D30、D31）：结算的回复照预览读，还在说的读到收束点。本模块不读盘、不读内容库、不读视图，只把答复拼成 `Answer::Reply`；读不出（文字里有 NUL）答 `Unavailable { query: "Reply" }`（`crates/wire/Spec.lean` §8-75）。
- **在视图锁外读**：`prepare` 只把文字拷进 `Prepared::Reply`，comrak 读一窗在锁放开之后做，理由同预览（§12 第 41 条）。
- **代价。** 一次收束点的扫描（与送来的文字同长，逐字节），加一次 comrak 读至多一个窗口；读数还没有，属 refrain 路线图 A11 的那一组。
- 验收：`views::answering::reply::tests`——一段带标题、列表、表、代码块与脚注的回复存成一个版本，经 `Query::Preview` 读出的 `Laid` 与经 `Query::Reply { state: Settled }` 读出的相等；同一段文字截在一个开着的段落里、以 `Streaming` 问，只答闭合的块，`span.end` 停在那一段之前；含 NUL 的文字答 `Unavailable`。

### 8-21 accounting::views::document 与 views::answering::range：文档的一版与它的窗口（形状 7 投影）

```rust
// accounting::views::document（锁外，sprawling-SPEC.md 8-100）
pub(super) fn document_answer(city_root: &Path, at: Address) -> wire::DocumentAnswer;
pub(super) fn read_bytes(bytes: &[u8]) -> Reading;      // Content 与 Prefix 的文本判定，经 documents::Reading::of
// accounting::views::answering::range（锁外）
pub(in crate::views) fn range_answer(city_root: &Path, version: B3Hash, range: documents::Span) -> wire::Answer;
```

- **读盘只读一次，判定全在 `documents`。** `document_answer` 读文件的全部字节：读不到且是 `NotFound` 答 `Missing`，别的读错（目录、无权限）答 `Unreadable`，带系统的原话；零字节答 `Empty`；否则 `B3Hash::digest` 得版本，`Format::of_name` 读文件名，`Reading::of` 判文本，是文本就 `documents::head` 取第一个窗口（`crates/wire/Spec.lean` §8-69）。本模块不写任何一条判定，所以页面、`Content` 与 `Prefix` 对「这是不是文本」只有一个答案。
- **第一个窗口盖不住整份时，或格式是 Markdown 时，这一版进内容库**（`storage::Cas::put`，`crates/storage/Spec.lean` §8-36），然后才作答：之后的 `Query::Range` 与 `Query::Preview` 读的是这一版。放不进去（盘满、目录不可写）答 `Unreadable`，原话是内容库的拒因：答一个之后读不到的版本等于许诺一件做不到的事。整份放得下的纯文本版本不存（wire D8，§12 第 33 条）。
- **读内容库只有一处：`range::stored`。** `Cas::size` 得这一版的长度，前三个字节经 `Encoding::of_mark` 得编码，`documents::lift` 说要抬起哪一段，`Cas::get_range` 读它；交回编码与抬起的字节。`range_answer` 拿它经 `documents::cut` 切出窗口，`preview_answer` 拿它经 `documents::preview` 读出块（§8-23）。内容库没有这一版、或切出的字节不是文本，答 `Unavailable { query: "Range(<version>)" }`。
- **`read_bytes` 留给 `Content` 与 `Prefix`。** 两者的答复形状不变（头 `DOC_BYTES_MAX` 字节、`truncated`、`binary`），判定换成 `Reading::of`，切法换成 `documents::cut`：一个块在内容库里、又是城里的一份文件时，两处给同一个判断。
- 验收：`views::document::tests` 与 `views::answering::range::tests`，名字见 `crates/wire/Spec.lean` §8-69、§8-70。

### 8-22 保存、修改提案与提交说明（`accounting::worker::commanding::saving`，形状 4 adapter；`accounting::views::proposals`，形状 7 投影）

```rust
// accounting::worker::commanding::saving（worker 线程）
impl RunWorker {
    pub(in crate::worker) fn put_range(&mut self, write: &wire::RangeWrite) -> Result<(), AxError>;
    pub(in crate::worker) fn decide_proposals(&mut self, decisions: &wire::ProposalDecisions) -> Result<(), AxError>;
}
// accounting::views::proposals（视图与 worker 共用的折叠，住 Governance 里）
pub struct Proposals { /* 开着的卡：身份 ↦ documents::Offer；处理过的卡：身份 ↦ 怎样处理的 */ }
impl Proposals {
    pub(crate) fn offered(&mut self, run: RunId, payload: &Payload) -> Result<(), AxError>;   // proposal_offered
    pub(crate) fn decided(&mut self, payload: &Payload) -> Result<(), AxError>;               // proposal_decided
    pub(crate) fn withdrawn(&mut self, payload: &Payload) -> Result<(), AxError>;             // proposal_withdrawn
    pub(crate) fn open_on(&self, doc: &Address, id: &B3Hash) -> Result<&documents::Offer, AxError>;   // 不开着、不在这份文档上 → E_INVALID_ARGS
    pub(crate) fn all_open_on(&self, doc: &Address) -> Vec<documents::Offer>;                // 按提出的先后
}
pub(super) fn proposals_answer(city_root: &Path, doc: Address, open: Vec<documents::Offer>) -> wire::Answer;   // 锁外
// accounting::views::commits（锁外，与 parents 同一刻）
fn give_messages(city_root: &Path, commits: &mut [wire::CommitAnswer]);
```

- **一次保存是三步，都在 worker 线程上。** `put_range` 先拒保留子树（`Address::is_reserved`，`E_OUTSIDE_WRITE_DOMAIN`），再经 `city::revise_document`（`crates/city/Spec.lean` §8-40）在这份文档的锁里读出此刻的字节、交给 `documents::save` 判定并换上，最后写一行 `document_written`（`crates/kernel/Spec.lean` §8-83），它带着这条命令的 `idem`（`commanding::entrance::stamped`）。被拒的保存什么也不写，账上没有它。
- **决定修改提案也是一次保存。** `decide_proposals` 从 worker 的 `Governance.proposals` 找出点名的每一张卡（不在这份文档上、已经处理过、没有的都拒 `E_INVALID_ARGS`），经同一扇 `revise` 在锁里交给 `documents::decide`；有改动时写一行 `document_written`，然后每张卡一行 `proposal_decided`。卡的状态不在这里改：worker 写下的每一行都经 `RunWorker::absorb` 交给同一个折叠，重开的城读账本得到同一个答案。
- **提案的折叠住 `Governance`，视图与 worker 各持一份、折法一处**（第 34 条）。`proposal_offered` 与 `proposal_withdrawn` 由 run 经工作台的 `proposal` 工具写下（§8-30）。`proposal_offered` 经 `documents::Offer::of` 读成一张卡，身份由它算出；读不出的一行（区间颠倒、超长）与别的读不出的治理行一样拒绝，让这座城停在打开那一步，而不是少一张人等着决定的卡（sprawling-SPEC 8-74 的同一条理由）。`proposal_decided` 与 `proposal_withdrawn` 把卡从开着挪到处理过；处理过的卡只记身份与怎样处理的，不留原文与提议。
- **`Query::Proposals` 在锁内拷出这份文档上开着的卡，锁外读盘。** 文件此刻的版本要读一次全部字节（`B3Hash::digest`），所以与 `Document` 一样在快照放开之后做；卡的句子由 `Offer::review` 在那时切。文件缺失或读不了时 `version` 为 `None`，卡照答。
- **提交说明读自 git，与父提交同一刻。** `CommitsAsk::read` 在快照放开之后先经 `storage::parents_of` 读父提交，再经 `give_messages` 读说明：一次打开仓库（`git2::Repository::open`），每个 oid 一次 `find_commit`，`Commit::message` 不是 UTF-8 或对象不在时为 `None`（`crates/wire/Spec.lean` §8-54）。
- 验收：`worker::commanding::tests::saving`（`crates/wire/Spec.lean` §8-72、§8-73 列的那几条；`Query::Proposals` 的答复——开着的卡按提出的先后、文件此刻的版本——在其中经 `views::ask` 读）；`views::commits::tests::a_page_of_commits_carries_each_ones_message_from_git`。

### 8-30 工作台的 `proposal`：run 提出与收回修改提案（`accounting::worker::workbench::tools::proposal`，形状 4 适配器；documents D35、D36）

```rust
// accounting::worker::workbench::tools::proposal
pub(in crate::worker) struct Proposing { /* relay: worker::Relay、clock —— 私有 */ }
impl Proposing {
    pub(in crate::worker) fn new(relay: Relay, clock: Arc<dyn Clock + Send + Sync>) -> Proposing;
}
impl Laying {
    pub(super) fn proposal_tool(&self, site: &Site, room: &Address, bound: &runtime::ReadBound)
        -> Result<ProposalTool<Relay>, AxError>;
}
pub(super) struct ProposalTool<L: kernel::Ledger> { /* reader、filing、desk: Mutex<Desk<L>>、meta —— 私有 */ }
// accounting::worker::workbench::tools::proposal::quoting（形状 1 判定：引文在城里那一版里是哪一段）
pub(super) fn offered(reader: &runtime::BoundReader, asked: &Offering) -> Result<ProposalOffered, AxError>;
// kernel::Tool。参数 { action: "offer", path, old, new } 或 { action: "withdraw", proposal }；
// offer 答 { proposal, doc, baseline, start, end }，withdraw 答 { proposal, withdrawn: true }
```

- **一件工具两个动作，按写登记。** `Effect::Write { domain: 房间 }`，与 `goal`、`signal` 同形：它写的是账本行，所以同一波里的 `offer` 与 `withdraw` 按序执行，不被当作读提前跑；`writes` 答 `Nothing`，因为树里没有文件动。它在表上排在 `playback` 之后、城外工具之前，所以每栋楼的 run 都有它。
- **`offer`：读城里那一版，找出原文，判长度，写一行。** `path` 经一个建在城根上的 `runtime::BoundReader` 打开（交给 `read` 的同一个读界，第 32 条）：保留子树、读界关着的楼由那扇门拒；Locator 与块被拒，因为提案是关于城里那份文件此刻的字节（documents D35）。读出的字节不是文本（`Reading::Opaque`）时拒；是文本就整份解码，`old` 必须恰好出现一次，零次与多次各一句拒词，带次数；区间按那一版的编码换成字节：UTF-8 的两种，解码出的文字就是版本的字节（documents D5），UTF-16 的两种，每个码元两个字节。`old` 为空或与 `new` 相同时拒。然后 `documents::Offer::of` 判长度（documents D18），工具写一行 `proposal_offered`，记在这次 run、这个居民、这个房间名下，回答卡的身份（documents D13）。同一张卡再 `offer` 一次不再写行，答同一个身份；收回过的卡再 `offer` 被拒（documents D19）。
- **行经 lane 的 relay 写下。** relay 是 lane 唯一能写账本的门（第 11 条）；记账线程写下这一行之后把它交给 `Governance` 的折叠（§8-22），`Query::Proposals` 从此答出这张卡，人的决定也从同一个折叠找它。
- **`withdraw` 只收这次 run 自己提出、还开着的卡。** 判它的是工具自己的一本小账（身份 ↦ 开着／收回过）：run 的身份每次派活新铸（`run_id_for` 读时刻），所以这次 run 提出的卡只出自这件工具的这一个实例。别的 run 的卡、没提出过的身份、收回过的卡都拒 `E_INVALID_ARGS`，不写行。
- **当前状态：人在 run 还在跑时决定了它的一张卡，run 随后收回同一张卡，会多写一行 `proposal_withdrawn`。** 工具看不见那次决定；`views::proposals::Proposals::close` 照最后写下的一行把卡记成收回过。卡上的字节已经由人的决定落下，`open_on` 对两种处理都拒，所以没有字节写错，错的是折叠记下的「怎样处理的」，与 documents D19「处理过的卡不再动」不合。补法是折叠对已经处理过的卡不再改（`close` 保留第一次处理，`views/proposals.rs` 里一行），它也让任何一条迟到的收回成为被拒的一步；另一条路是收回改走记账线程的问询，像 `goal` 的登记那样由 `Governance` 当场判（sprawling-SPEC.md 8-42-8），那要在 `relay::Wake` 加一臂。
- 验收：`worker::workbench::tools::proposal::tests`（一次 `offer` 恰写一行、文档字节不动；收回别人的卡、收回两次、重提收回过的卡都被拒；UTF-16 文档上引出的原文经 `documents::decide` 落得下）；`crates/sprawling/tests/acceptance/` 的 catalogue 里 `proposal` 一段（`views::ask` 的 `Query::Proposals` 答出这张卡，属于这次 run，文档字节不动）；citysim 的 `tests/proposal_baseline.rs`（citysim D22：文档被城外的写者挪动之后，人接受这张卡以 `E_VERSION_CONFLICT` 被拒，文档留着挪动之后的字节，卡仍开着）。

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只用 `gateway` 的接口类型，不构造适配器。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。`GatewayModels` 在 worker 搬进来时一同搬进本 crate，理由见 §12-18；本条对 `SystemClock`、`ThisMachine` 这样直接碰主机的生产适配器仍然成立。
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
12. **装配根留在 `bin::assembly`：只留自由函数与直接碰主机的生产适配器。** `listening`、`attending` 的起线程那一半、`chain_watch` 的起审计线程那一半、`dropping` 与 `production`（`SystemClock`、`hands`、`init_city`、`form_city`）。理由：它们起线程、绑端口、造城的目录、装生产的手，是 ARCHITECTURE.md §3 说的知道每个具体类型的那一层；worker 搬走之后，它们对本 crate 的依赖是朝内的。`impl RunWorker` 的块一个也不留（§12-17）。被否决的做法：把 `genesis` 整个留在装配根——它的三个方法是 worker 的用例，而 worker 自己的测试要经它造城（§12-19）。
13. **harness 页找程序经 `Views.programs` 这个 `fn` 指针，不经 `Machine`，也不在开城时算好。** 理由：这一问读的是此刻的搜索路径，与 `registry`、`upstream` 同形——没有状态、服务中的城交一次、`None` 就答 `Unavailable`（§12-10）；它不启动任何程序，所以不必等 `DoctorRefresh`。被否决的做法：给 `Machine` 加一个方法——`Machine` 属于 worker，读面拿不到它，而且 doctor 的逐项查法已经在 `bin::doctor::Machine::look` 里，再加一个方法就是第二条查法；在开城时把 harness 的有无算进 `DoctorAnswer`——那是一个线上的形状改动，而且人在 harness 页上装完一个程序，要等到下一次 `DoctorRefresh` 才看得到它。
14. **`views` 搬进来时，本 crate 加 `storage` 与 `runtime` 两条边。** 理由：`views` 折叠的就是 `storage` 的 `HotView`、`Attribution`、`LedgerIndex`，快照起步与 worker 的 `Standing` 共用 `runtime::replay::fold_ledger_dir` 的同一遍；worker 搬过来后本来也要这两条边（§12-11）。被否决的做法：把这两处读经端口交进来——端口会把一份折叠的权威分到两个 crate，与 §12-9 否决的是同一件事；把 `fold_ledger_dir` 挪进 `storage`——那改的是 `runtime` 的公开面，与这次迁移无关。
15. **sprawling-SPEC.md 里写这些模块的节不随模块搬，只改模块路径的拼写；S4 迁 `Spec.lean` 时一次搬进本 crate 的规格。** 理由：两处都是 Markdown 时，搬一次、S4 再改写一次，是两遍约两千行的重写；两份 SPEC 的节号相撞（sprawling 的 8-3、8-6 与本 SPEC 的 8-3、8-6），sprawling-SPEC 自己也有重号，逐节搬要先重新编号，而代码与文档里引用 `sprawling-SPEC.md 8-xx` 的地方都得跟着改。被否决的做法：照 §3 早先的第 5 步逐节搬——8-7、8-8、8-9 那样的小节可以，几十节不行。
16. **`views` 的测试经 `worker::fixture::init_city` 造城，与 worker 的测试是同一个创世。** 理由：页面读到的城就是创世留下的城；测试用真正的 `worker::genesis::form`（带测试的手），市政厅的布局、创世两行与 `City.md` 改了，读面的测试跟着看到。被否决的做法：保留一份只写读面测试读到的东西的第二份创世——worker 搬进本 crate 之前确实这样做过，因为那时本 crate 够不到 `genesis`；它与真正的创世没有东西把两者拴在一起，一旦市政厅的布局变了，读面的测试就在一座不存在的城上判定。
17. **`impl RunWorker` 的块全部住本 crate；装配根里混着两种东西的三个文件先拆开再搬。** 理由：Rust 只允许在定义类型的 crate 里写固有 `impl`，而 `attending`、`chain_watch`、`genesis` 各有一半读 worker 的私有字段。拆法按「谁起线程、谁碰主机」：循环 `attend`、挂 halt 的 `chain_under_audit`、造城的 `form` 与三个造楼方法随 worker 走；`spawn_worker`、审计线程、`init_city`/`form_city` 两个生产入口留下。被否决的做法：在 `sprawling` 里用扩展 trait 给 `RunWorker` 加方法——调用方要先把 trait 引进作用域，而 trait 方法仍然碰不到私有字段，只能再开公开的门。
18. **`GatewayModels` 与 `Residents` 随 worker 搬进本 crate，`Hands` 只装直接碰这台电脑的东西。** 理由：worker 自己的测试有上百处经生产的 `GatewayModels` 对一个回环地址上的假 provider 说话，经 `Residents` 连一个 conformance 子进程；两者留在 `sprawling`，测试就得在本 crate 再写一份 dialect 头加 `gateway::adapter_for`，那是「一个 `Chosen` 怎样变成适配器」的第二个权威。它们自己不碰主机：出网在 `gateway` 的适配器里，起进程在 `agent_protocols` 的链接里，worker 探端点、读 MCP 健康时本来就经这两个 crate 伸手。被否决的做法：把模型工厂与连接表也放进 `Hands`——测试的手要么复制生产实现，要么换成脚本，后者会改变几百条测试测的东西。
19. **worker 的测试经 `worker::genesis::form` 与 `fixture::hands()` 造城。** 理由：`init_city` 在 worker 的测试里被调用一百多次，它必须在本 crate 里可达；`form` 需要的时钟与 vault 本来就是交进来的手。`bin::assembly::production::init_city` 只把生产的 `Hands`（`Custodian::probe` 打开的 vault）交给同一个 `form`。被否决的做法：测试继续经 `sprawling` 造城——本 crate 不能依赖 `sprawling`，dev-dependency 成环会链接两份 `accounting`，类型对不上。
20. **宿主的手是一个值 `Hands`，由构造器收下。** 理由：搬过来以后 `new` 叫不出 `sprawling` 里的适配器，生产的那一份只能从外面来；九样东西总是一起到、一起用，是一个值（AGENTS.md）；参数上限是四个。换一只手有两种写法：结构体更新语法，或者构造之后的 `with_*` 门。被否决的做法：`Host` trait——§12-10 已否决，理由不变（脚本为换一只手要实现全部）；`fn` 指针组成的结构体没有这个代价。九个参数——超出 4 的上限，而且每个调用点都要把九样东西排一遍。
21. **vault 也放进 `Hands`。** 理由：生产的 vault 打开的是这台电脑的凭据服务（`Custodian::probe`），脚本给的是内存里的一份，它与其余几只手一样是构造时从外面交进来的；放进去以后三个构造器都不超过四个参数。被否决的做法：把 `vault` 与 `log` 捆成一个值——两者没有共同的意思，捆起来只是为了凑参数个数。
22. **驾驶 lane 的线程从 `accounting::worker::pool` 起。** 理由：`pool` 与 `relay`、`drive_run`、`RunWorker` 成环，必须一起搬（§12-11）；lane 的寿命仍然恰好是它驾驶的那个 run。ARCHITECTURE.md 的确定性规则 3 因此把它列为库 crate 起线程的第六处。被否决的做法：经 `Hands` 交一个起线程的 `fn`——它只有一个实现，而且只是把 `std::thread::Builder` 换个名字。
23. **验收覆盖从模型收到的工具表算出应当调用的集合；城外工具不进这张表；效果层拒绝的工具按「没有东西变」判定；技能经城库装入。** 理由：哪些工具存在，唯一的权威是工作台的那一次登记（sprawling-SPEC.md §8-27），模型第一次请求里的工具表就是它的输出。测试若照抄一份名单，下一次加工具时名单会悄悄漏掉那一件；从工具表算，漏掉的那件会被点名。城外工具的集合随楼的配置与主机而变，放进来就要在测试里配一台浏览器或一台 MCP server，而它们的路由已经各有一条端口测试。`rules` 与 `city` 声明 `Effect::Govern`，拒绝码取决于工具是否给出自己的 `subject`（`crates/city/Spec.lean` §8-2b 写了两种拒词）；钉住拒绝码，补上 `subject` 的那次改动就会打红验收，而验收要守的规矩——run 不改写审判它的规则、不立楼——在那次改动前后都成立。技能经 `city::install_skill` 装进城库：那条路把一个包的每个文件落在城内，所以验收连包里附属文件的按名读取一起判；城外书架上的一件由 catalog 携着正文交给 run（`crates/runtime/Spec.lean` §8-29-6），它的验收是另一条测试，判的是同一条阅览室、catalog、`read` 与 `SkillPin` 的链。被否决的做法：手写工具名单再逐件断言（第二个权威）；把城外工具一并覆盖（重复端口测试，并让验收依赖主机）；按拒绝码断言效果层的拒绝（钉死一个 SPEC 已说明会变的细节）。
24. **playback 是账务读面上的一个投影，按整行判定可见，逐字节携带账本行，只读一遍严格校验过的字节，复核靠重算。**
    (a) 一行可见，当且仅当它碰到的每一栋楼对读者都是 `Open`；碰到的楼由信封地址、run 的房间、关闭的那一对的打开行与载荷里以已知楼开头的地址求出，一条规则管所有事件种类。理由：读界要对未来新加的种类也关着，一张按种类列可公开字段的表，每加一个种类就要加一行，漏一行就漏字段；整行判定漏不了。代价是一行只要碰到一栋关闭的楼就整行隐去，连同它本可公开的字段。被否决的做法：按种类逐字段投影（维护面随种类增长，缺行时无声地开或关）；只按信封 `addr` 删行（`approval_resolved` 记在 city run 上、`addr` 为空，handback 的内容来自子 run）。
    (b) `events` 里每条是账本原行的字符串，外加十进制字符串的 `seq` 与 `moment`。理由：原行就是账本的权威字节，读者可以对它重算 `chain_hash`；把记录展开成 JSON 对象会让 seq、`t` 与金额在 JS 的 `Number` 里丢精度，也等于第二种写法。被否决的做法：展开成对象、u64 写成数字。
    (c) 严格校验自己走一遍 `LedgerIndex::folding` 加 `LineCheck::advance`，不用 `runtime::replay::fold_ledger_dir`。理由：导出要每一行的原字节（cutoff 行的链哈希、逐字节的 `line`、凭据扫描）和不认识的可忽略行的 seq，`fold_ledger_dir` 两样都不交出；事后按索引重读原行，会把审过的字节和重读而未审的字节混在一起。被否决的做法：`fold_ledger_dir` 加按索引重读。
    (d) `check --city` 用同一个投影重算并逐字节比较，读者取入口给的。理由：只比 `source` 与末行链哈希时，保留 `source` 而改摘要、删事件都比不出来；信 bundle 自述的读者，就能用一份伪造的 `{"person":"included"}` 扩大权限。被否决的做法：比末行链哈希；按 bundle 的 `reader` 重算。
    (e) PR 的关键时刻键是 `<branch>@<pr_opened 的 seq>`，关闭行关掉同一分支最近打开的那一个。理由：请求在账本上的身份就是分支（`collab::OpenRequest`），同一分支会重开；只用分支作键会把两次请求并成一个。被否决的做法：只用分支。
    (f) `checkpoints` 里哪些行点名一个提交、点名哪个 oid，问 `accounting::views::commits::commit_facts`（它对本 crate 可见）；`JobPinned` 不点名提交，按 `CheckpointCommitted` 自己的类型读出，与 `Committed`、`pr_merged` 分开写。理由：识别提交的权威只能有一个，`views` 的提交页与 playback 的表必须对同一行给出同一个答案；`pr_merged` 的 oid 写在手写键里，抄一份读法，两处就会在那个键改名时分开。被否决的做法：在 playback 里再写一份识别提交的匹配。
    (g) 人的入口在 `Confidential::Withheld` 时按楼的规则取三臂，而不调 `may_read`。理由：`may_read` 要一个读者所在的楼，人不住在任何一栋楼里；为了调它而编一栋楼，会让「人的楼」成为一个不存在的地址。三臂的类型仍是 `kernel::ReadVerdict`，居民入口仍调 `may_read`。被否决的做法：给人编一个地址。
25. **playback page 由产品嵌入、用浏览器的解析算法查、五项分开报，导出件经一个落盘函数写下。**
    (a) 页面用 html5ever 的树构建器读，本模块只写一个记下元素的 sink。理由：静态检查要判浏览器会建出的元素，而 HTML 的分词受树构建影响——`<svg>` 里的 `<style>`、`<title>` 按普通标记读，`<noscript>` 在开着脚本时是原始文本；只有分词器的做法（html5gum 加按标签名切状态）在外来内容里会把一个 `<img src>` 当成样式文本漏过去。html5ever 是 Servo 的解析器，跟着 WHATWG 的解析算法走。代价：markup5ever、tendril、string_cache 等几个包进锁，发行二进制变大，读数归整合者。被否决的做法：正则或字符串扫描（`</script>`、注释里的标记、实体编码都会骗过它）；html5gum（无依赖，但外来内容里与浏览器不一致）；scraper（多带 selectors，且它锁的 cssparser 与这里的版本不同，锁里会有两份）。
    (b) CSS 用 `cssparser` 分词，不开默认特性。理由：转义（`u\rl(`）、注释、嵌套函数与坏 url token 只有按 CSS 语法分词才判得对；默认特性只换来更快的字节匹配与颜色表，页面里的 CSS 是千字节级，差别在微秒以下，不值一个过程宏和一张 phf 表。被否决的做法：手写分词器（第二份 CSS 语法）；lightningcss（整个样式引擎）。
    (c) 静态规则按「哪类元素与属性能取外部资源或送走读者」整类拒绝，而不是逐个判它的 URL：`form`、`base`、`iframe` 一类不出现，`srcset` 不出现。理由：参考模板与 agent 写的页面都用不着它们，而它们各自的 URL 语义（`srcset` 的逗号、`srcdoc` 继承 CSP、表单的提交目标）要一份份写判定，漏一份就漏一个出口。代价：一个确实想用 `srcset` 的页面写不进来，要改用 `data:` 的单张图。
    (d) CSP 只许不取外部来源的值，`connect-src`、`base-uri`、`form-action` 要显式写 `'none'`。理由：`default-src` 只为取资源的指令兜底，不管 `base-uri` 与 `form-action`；`'self'` 在 `file://` 下的含义随浏览器而变；`'strict-dynamic'` 让受信脚本加载任意 URL。被否决的做法：只要求 `default-src 'none'`。
    (e) bundle 由产品嵌进页面：模板里留一个空的 `BUNDLE_BLOCK`，`embed` 原样拼进规范字节，再用同一套检查查结果。理由：agent 自己粘贴或经 JS 重写 bundle，u64 会在 `Number` 里丢精度，粘错一个字节结构就失败；拼接由产品做，页面里的字节就是导出的字节。标记写成固定的一串字节而不在解析树里找位置，因为解析器不给源偏移；拼完再解析一遍，标记落在注释里之类的情形由结构一项挡住。被否决的做法：让 agent 照 skill 自己嵌入。
    (f) 浏览器观察由 skill 借宿主的浏览器完成，产品只读它写下的记录，并核对记录说的是这份字节。理由：二进制里不带浏览器，也不替人启动一个——前者体积与维护都大，后者要在每台机器上找浏览器、处理它的权限；宿主（城外的 agent 或居民的浏览器工具）本来就有。代价：记录是自报的，产品核对不了浏览器真的跑过，`browser` 一项因此只说「这份记录说在这些路径下没看到」。被否决的做法：随产品打包无头浏览器；产品自己启动系统浏览器。
    (g) `check` 交回五项而不返回错误，`Unasked` 与 `Unable` 在 Rust 里分开、对外同写 `unchecked`。理由：一项读不下去（城的账坏了、观察记录读不懂）不该挡住另外几项的结论；而「没人要」与「要了做不了」对退出码的意思不同，前者不算失败，后者算。被否决的做法：一个总结论（把「内容正确」「不联网」混成一个标识）；用错误终止整个检查。
    (h) 写新文件的做法从 `bin::main::playback` 搬到 `playback::landing`，人的 `--out` 与居民的导出位置共用；本 crate 因此直接依赖 `git2`，只读索引与忽略规则。理由：两扇门要的是同一个保证（整份落下、不覆盖、失败不留半成品、不进历史），写两份就会在某一份加了检查而另一份没加时分开。`git2` 已是 `storage` 的依赖、同一份 libgit2，按 ARCHITECTURE.md §4 直接用，不为两问开一个端口。被否决的做法：CLI 与工具各写一份；在 `storage` 加一个只为这两问的函数（这两问不属于 checkpoint，也不属于 worktree）。
    (i) `Select.lean` 的场景表只写在 Lean 里，`scenes_agree` 证明模型给出表里的结果，Rust 测试从同一个 `.lean` 文件读表、跑生产的 `export`。理由：表只有一份，模型与实现分别对它负责；Lean 输出一份 JSON 再由 Rust 读，要多一个必须与 `.lean` 保持同步的生成物，测试还要先跑一次 Lean。被否决的做法：Lean 生成 JSON 夹具；Rust 里另写一份场景表。
27. **一个 session 的身份冻在房间那一层，读回失败就拒，不换成此刻的名字。** 理由：session 的形状（模型、强度）已经记在房间那一层，`/new` 清的也是它，身份跟着同一个边界就不需要另一条「何时重读身份」的规则（city D11）；读不回冻下的那一版时换成此刻的名字，等于在 session 中途悄悄改名，而这正是冻结要防的。被否决的做法：每次 run 现读身份——改名立刻改掉正在进行的 session 的前缀，provider 的前缀缓存从 city 段起失效，页面上的旧 session 与请求里的名字也对不上。
28. **`whose --trace` 的逻辑是读面上的一个模块 `accounting::trace`，从 `Query::Commit` 的答出发再读账本，不加线上查询；同楼的别人只计数。** 理由：区间的两端已经在 `CommitAnswer` 的 `seq` 与 `previous` 里，调用的读法已经在 `views::turns` 里；今天的读者是读盘的 CLI，下一轮的 playback 与验收工具都在本 crate 里或经本 crate 读。同楼别的 run 的写也可能落进这个提交，但把它们的调用与本 run 的并列，会把「候选」读成「原因」，所以只给条数与地址，要细看的人拿 `view --run` 去读。被否决的做法：①加 `Query::Trace`：线上多一个形状、`WIRE_V` 进一位、`wire.ts` 与 adversary 的门面都要跟着改，换来的只是把这几步搬到服务端，而 CLI 本来就读盘；②按 `Call.effect` 只留写调用：读调用决定了写什么，去掉它们就去掉了归因的一半证据，`effect` 留在每条调用上由读者判断；③区间以 git 的父提交或全城紧邻的上一个提交为界：两者都可能属于别的 run，会把别人的调用算成这个 run 的。重开参数：页面要显示一个提交的调用时（那时要一个线上查询，本模块搬到 `views` 后面作答）；或同一栋楼里几个 run 同写一棵树成为常态、条数不够区分时。
29. **playback 的时间条件比信封 `t`、逐行判断，`--day` 展开成同一个区间；调用的耗时只在 rounds 判为量出来时给出；提交的证据经 `trace` 与 `storage::hunks` 读，读不到就写明读不到。**
    (a) 时间条件读每一行信封的 `t`，与 `view --since/--until`（sprawling-SPEC.md 8-137）同一个 `UtcSpan`，不按种类去载荷里挑时间字段，也不靠 `t` 有序提前停或二分。理由：`t` 就是这一行记下的那一刻，任何种类都有；它不随 seq 单调，在第一条越过 `until` 的行处停下会漏掉回退的行。被否决的做法：只对四种记时刻的行判时间、其余行跟着它所在的回合走（同一件事两个家，且旧账本里根本分不出回合的边界）；在索引里存时间列再二分（要 `t` 有序）。
    (b) `day` 拼成 `<day>T00:00:00Z` 交给 `parse_iso`，加一个 UTC 日的毫秒数作上界；与 `since`/`until` 同给时取交集。理由：历法与校验只有 `runtime::clock` 那一份，日期另写一个解析器就是第二份；一天在 UTC 里总是 86 400 秒（Unix 时间不计闰秒）。交集而不拒绝同给，是因为「这一天里九点以后」本来就是一个合法的问题。被否决的做法：`day` 与 `since`/`until` 互斥（把一个合法的问题拒掉）；在本模块写一个 `YYYY-MM-DD` 解析器。
    (c) 工具调用作为 links 的第四种对，键是 run 与调用 id；耗时由 `views::rounds::answered_timing` 判量没量，playback 只做减法。理由：配对的两端状态（在范围内、范围外、隐去、未答、缺）与关键时刻、消息是同一套，放进 links 就不必为调用另写一份；「补写的答复不算量过」这条规则在 rounds 里，把它搬成一个函数让两处共用，就不会在 rounds 改它时分开。代价：关闭行继承打开行碰到的楼，一条碰过机密楼的调用，它的答复也隐去，比以前多隐去一些行。被否决的做法：把 `views::turns` 用在选中的行上（回合的开头可能在范围外，turns 会丢掉没有回合的调用，也不交出答复那一行的 seq）；让页面拿两条 `moment` 自己相减（读者看不出补写的答复，旧行与真的零毫秒也分不开）。
    (d) 提交的调用归属只经 `accounting::trace`，问的是第一次宣告的那一行：区间按这一行求；再次宣告的行写 `untraced`；范围外的调用只给 seq（`elsewhere`），行不进 `context`。理由：归因的权威是 `accounting::trace`，playback 再写一份区间规则就是两个家；`trace` 按视图里最近一次宣告求区间，而没有改动的检查点会把同一棵树再宣告一次，别的 run 也会，所以导出之后的一次宣告就能让复核与原 bundle 不同——两名居民的验收先撞上了这一点。按第一次宣告那一行求，答只取决于那一行之前的历史。范围外的调用要进 `context`，就得在整遍读里把所有调用行留着，常驻量与整本账同阶。被否决的做法：直接用 `trace`（导出之后的宣告改变复核）；在 playback 的折叠里按「同一 run 上一个提交」自己求区间（第二个家）；把归属里的调用行都放进 `context`。这一行的答怎样只读到它自己、不越过 cutoff，见第 37 条。
    (e) diff 的基准是同一 run 的上一个提交，没有时才用唯一的父提交；文件的六种状态分开写，凭据扫描是 `storage::hunks` 那一个；读不到 git 不让导出失败。理由：全城紧邻的提交可能属于别的 run，git 的父提交也可能出自别人或人自己（与 §12-28 同一条理由）；缺失、二进制、空、截断、隐去对读者是不同的事，混成一个「没有 diff」就让读者把「没动」读成「读不到」。账本是 bundle 的核心，git 是附件：附件缺了，核心那一段照样能回看。被否决的做法：对比工作区（不是历史）；用全城紧邻的 checkpoint 作基准；git 读不到就整个导出失败。
    (f) 字段一变，`SCHEMA` 与 `PROJECTION_RULES` 一起进位；读回时先只读 `schema`。理由：多了字段是形状变了，旧构建读不懂新 bundle，新构建也读不懂旧的；先看 `schema`，结构一项报的是「这是第几版、用哪一版复核」，而不是一条字段缺失。被否决的做法：只进 `PROJECTION_RULES`（旧 bundle 在解析时就失败，到不了「复核不了」那一步，报出来的是一条难懂的字段错误）。
30. **死掉的 run 由启动扫描冻结，冻结行写成它的居民，结局读 `RunFrozen::lost`。** (a) 理由：只有拿到写者锁的那一刻才知道没有别的进程在驱动它，而 `startup_scan` 正是那一刻的那一遍验链；视图与 worker 的折叠都从账本来，账上一行冻结让服务中的城与重开的城对同一次 run 说同一个结局。写成居民而不是城，与补写结果未知的调用同一条理由，按居民计数的读者不必为死亡另写一条规则。被否决的做法：在服务时由视图把「没有冻结行、进程已重开过」的 run 读作死掉——那是视图的第二条冻结规则，而且一次性的 `views::ask` 与服务中的视图会各算一次；由 `RunWorker::new` 冻结——`new` 也在 `serve` 里跑，那时冻结要跟账本证明的次序对齐，而 `resume` 本来就是收拾死亡的那一步（sprawling-SPEC 8-109）。重开参数：`serve` 也要在起步时收拾死亡（不经 `resume`）时，把这一遍挪进它的起步路径，次序仍是先补调用、后冻 run。 (b) **指南进度住 `accounting::guide`，一个与 `person` 平行的模块，读写各一扇门。** 理由：页面读与命令写读的是同一份文件，文件的文法只能有一处；它不属于视图的折叠，也不属于 worker 的状态，`person` 已经是「一份人改的文件，读整份、写整份」的样子。被否决的做法：读放在 `views::answering`、写放在 `worker::commanding`——两处各知道一遍文件的形状。(c) **跑 gh 的函数经 `Views::ask_github_through` 交进来，主机名的判定与缺省主机留在视图。** 理由：起子进程碰主机，按第 9、10 条住 `sprawling`、经 `fn` 指针交进来；而「问哪台主机、这个串能不能交给 gh」是城对输入的判定，测试不必起 gh 就能判它。被否决的做法：经 `Hands` 交给 worker——这是一条查询，worker 不答查询；在二进制里判主机名——测试就要经过子进程才看得到拒绝。
31. **(a) 视图的第二份按值克隆，不经快照编码。** 理由：两份视图要的是同一个折叠状态加上同一组共享句柄，派生的 `Clone` 正好如此，而编码再解码在 40 万行城上要 350 ms，是开城最长的一段之一（§8-19）。被否决的做法：①留在编码路径上，把复制挪到视图线程——首字节不再等它，但视图线程开头的每一批照样等 350 ms，而且要改装配根起线程的次序；②手写逐字段复制——字段清单的第二份拼写，加一个字段就要改两处。**(b) 重建从创世时不先证明。** 理由：全量折叠逐行核对每一行，判定与不带记录的证明相同；先证明再全量折叠是同一批行核对两遍（§8-19）。被否决的做法：照旧先证明，把证明的结果交给全量折叠跳过核对——折叠要的是每一行解析出的记录，跳过核对仍要解析，省下的只是规范回显的比较，却让「这一行核对过」有了两处来源。 **(c) 房间的各段 session 由视图折叠，表按地址存在 `views::sessions`。** 理由：作答不读盘，表的大小与 session 数同阶（wire D9）。被否决的做法：把这张表并进 `worker::folds::SessionOrigins`——那张表回答的是派活要问的「这一段还欠不欠一段对话」，只留当前一段，worker 的 `Standing` 也不由页面读；把各段 session 并进 `lineage`——`lineage` 由读盘的 CLI 每次重建，服务中的城不持有它。
32. **城的工具读别楼的文件与 `cas:` 块，只经 runtime 的 `BoundReader`。** 理由：读界与 reserved subtree 的判定住 `runtime::tools::chosen_path`，`read` 与 `search` 用它；一件读字节的工具若在本 crate 自己判，就是那份判定的第二个权威，而且只判得了本楼（`is_within`），连接器存进 CAS 的录音与截图都读不到（§8-20）。被否决的做法：①保留「只收本楼」并为 `cas:` 另写一段（两套判定，一套跟着 `read` 变，一套不跟）；②在本 crate 复制 `admit` 与 `land`（同上，且链接的判定要拷两遍）。重开参数：要读的字节不在读界之内（例如人拖进来、只给这一次 run 的文件），那时它是一个新的入口，而不是放宽这扇门。
33. **文档的版本在第一个窗口盖不住整份时、或格式是 Markdown 时进内容库，由答 `Document` 的读面放进去。** 理由：之后的 `Range` 与 `Preview` 要读的是这一版，而版本的身份本来就是内容库的地址（documents D3），放进去之后按版本读就是按地址读对象，不需要第二个存放处；整份已经在答复里的纯文本版本页面不会再按版本要，存它只是让每一次打开多付一份拷贝，而 Markdown 版本不论长短页面都按版本预览（§8-23）。内容库按内容去重，同一版第二次放入不多写一份。放进去的是读面而不是写者：这是一次查询的副作用，但它只添一个按内容寻址、重复放入即去重的对象，不改任何一条历史，写者也不知道哪个页面在读哪一版。被否决的做法：①`Range` 读文件此刻、版本不符就拒——居民在写的文件每几秒动一次，读到一半的页面要从头重读；②每次打开都存——小文件的每一次打开都多一份拷贝；③由写者在每次保存时存——城之外的写者（人的编辑器、居民的 `edit`）不经过写者；④小的 Markdown 版本让 `Preview` 读文件此刻、摘要相符才答——两条读法各判一次「这是不是那一版」，而文件在两次读之间可能被改。重开参数：内容库长出回收时，被回收的版本要有自己的答复；页面要对比一份小的纯文本文件的两个版本时，小文件也存。
34. **(a) 修改提案的折叠住 `views::Governance`，与待答的审批同一个值。** 理由：一张提案卡与一条审批同是「等人决定的事」，worker 判一次决定要的状态与页面画卡要的状态是同一个，`Governance` 正是「一份定义、两处持有、`what_a_worker_holds_is_what_a_restart_rebuilds` 判它们相等」的那个值；放进去之后，快照、重开、worker 写下一行就折一行，都不需要新的接线。被否决的做法：①worker 另折一份 `Standing` 字段、视图另折一份——两份折法；②决定时按身份回账本找那一行——要一个按内容找行的索引，而且「已经处理过」还要再扫一遍。**(b) 提交说明在本 crate 用 `git2` 直接读，与 `storage::parents_of` 各开一次仓库。** 理由：本轮 storage 的公开契约不改（它的规格在迁移），而本 crate 已经为 playback 链接 `git2`；读说明只是 `find_commit` 之后的一个字段，一页至多 `HISTORY_MAX` 个提交多开一次仓库。被否决的做法：①在本 crate 里连父提交一起读、不再调 `parents_of`——两处各有一份「父提交怎样读」，`parents_of` 留下来没有调用方；②把说明写进账本——提交对象就是它的权威，账本记的是 oid。重开参数：storage 的契约下一次能动时，`parents_of` 换成一次读出父提交与说明的读者，本 crate 的 `give_messages` 删去。
35. **预览由读面按版本从内容库读，判定全在 `documents`，读出的块不缓存。** 理由：版本就是内容库的地址（documents D3），按版本读与 `Range` 同一条路，读面在视图锁外读，worker 不答查询；一个版本的字节不变，预览也就不会过时，页面不因任何事件重问。读出的块不进视图：它与一个窗口的大小同阶，而视图里的每一样东西都要随快照编码、随每一次重建折叠。被否决的做法：①按地址读文件此刻——与第 33 条的①同一个缺陷；②把读出的块按版本存在视图里——第二份拷贝要随快照走，而一次读出只是一窗的 comrak；③由页面先 `Range` 再把文字送回城里读——文字在线上走两趟。重开参数：在固定语料上量出一窗的读出超过毫秒级（D41），或同一版本被许多页面同时预览成为常态时，按版本缓存。
36. **快照的格式门用夹具摘要，夹具里放进每一种出现在快照里的 kernel 摘要类型（§8-24）。** 理由：`fold_version` 只在夹具的编码变了时才动，夹具缺哪一种类型，哪一种类型的编码就能悄悄改变，而旧快照照样被当成新格式读。被否决的做法：①为每个出现在快照里的类型各写一条字节数断言——那是编码的第二份拼写，加一个类型就要多写一条；②把快照格式的版本号写成手改的常量——改 kernel 的人看不见它。重开参数：快照换掉 postcard、或快照的编码有了自己的模式描述（schema）可以直接取摘要时，改由模式描述钉住版本。
37. **模型调用的耗时按 rounds 的那一条配对规则求；提交的证据在 walk 里折到宣告行时问视图，再经 walk 的索引读行，所以只读到 cutoff（§8-25）。**
    (a) 模型调用作 links 里调用的第二种对，答复由 `views::rounds::Attempts` 配到它的 run 最近一条 `model_called`，`turns` 也经它配；`calls` 的行换成 `callee` 两种之一，`SCHEMA` 进到 `sprawling.playback/3`，`PROJECTION_RULES` 进到 4。理由：「一条答复答哪次尝试」原来只写在 `turns` 的 `folded.last_mut()` 里，playback 再写一份就是第二个家；做成两处共用的一个小值，rounds 改这条规则时 bundle 跟着改。模型调用没有 id，键用 `model_called` 的 seq：它在一本账里唯一，复核时也不变。用 `callee` 而不是在原来的行上加一个可空的 `model` 字段，因为工具调用的 `id` 对模型调用没有意义，可空字段会让读者分不清「没有」与「读不到」。被否决的做法：①让 `wire::Turn` 交出答复行的 seq（线上形状变，`WIRE_V` 进位，换来的只是 playback 少写几行）；②页面拿 `events` 里两条 `moment` 自己相减（看不出哪条答复配哪次尝试，一次重发会被读成一次很长的调用）；③把 `first_at` 当耗时（那是首字延迟，另一个问题）。
    (b) 提交的证据从 walk 里逐行折的 `trace::History` 读：范围内第一次宣告的 `Committed` 行，在折完它的那一刻问 `Query::Commit`；调用与同楼的别人经 walk 建的索引读。理由：cutoff 的唯一定义是 walk 停下的那一行；视图折的是 walk 核对过的同一批记录，问的是折到宣告行时的视图，所以答只取决于那一行之前的历史，cutoff 之后的行写了什么、审不审得过都碰不到它。一遍折叠代替了每个提交一次 `views::ask`（每次审一遍整条链、折一遍视图）与一次 `LedgerIndex::rebuild`，一份 bundle 的代价不再是提交数乘账本字节数。被否决的做法：①给 `trace` 一个停在某个 seq 上的 `views::ask`（要 storage 的整链审计与快照起点都能停在一个 seq 上，那是 storage 的公开契约；而且每个提交仍各折一遍）；②walk 之后从创世再折一遍到 cutoff（多读一遍账本，读到的字节不是这一遍核对过的）；③在 playback 里只折提交，按「同一 run 上一个提交」自己求 `previous`（那是 `views::commits` 那条规则的第二个家，第 29(d) 条已否决）。代价：没有提交的窄选择也要折一遍视图；在 40 万行的城上这一遍约 1.1–1.8 s，是一次导出的 4% 上下（8-25），所以照现在这样折。被否决的还有④范围里没有提交时不折：walk 读到范围之后才知道范围里有没有提交，要么从创世再读一遍到第一个提交（同②，读到的字节不是这一遍核对过的），要么先把行留在内存里（40 万行的城要几百 MB）。重开参数：视图折叠在导出里占到一半以上时（例如投影的 credential 扫描变快之后，§3），再看「遇到第一个范围内的提交才开始折」。
    (c) 「一次导出开始了几次视图折叠」由一个只在测试里编译的计数读出：`trace` 里开始一次折叠的两处（`trace` 的 `views::ask` 与 `History::new`）各数一次，`playback::tests::tracing` 在 N 与 2N 个提交上比较。理由：要挡住的回退是「每个提交又把整本账折一遍」，墙钟读数在小夹具上看不出它，计数与机器快慢无关。被否决的做法：只记墙钟读数（看不出按 N 增长的代价）；经 storage 的 `Vfs` 缝数读了几遍段（那是 storage 的接口，这里不改它）。
39. **计划的效应按次序重放、每条在前面几条留下的文本上核，只有认领核盘上的状态；过时报第一条对不上的认领（§8-27）。** 理由：「分一行要不要握着它」只由桌子判（collab D6），落地若再为拆分、放下各判一个期待状态，就是同一条规则的第二份拼写，`tools/adversary/Spec.lean` §4 的第八个发现正是两份拼写不一致的样子。按次序重放让本 run 的前几条效应成为后几条的前提，「每个节点只核第一条效应」那条例外随之没有了，本 run 拆出又认领的子行也不再被判过时。被否决的做法：①照旧只核每个节点的第一条效应、对盘上原文判——拆出的子行在盘上还不存在，认领它的那条被判过时，工具答了成功、落地一字不写，与第八个发现同一类；②过时之后接着重放，把每一条对不上的认领都报出来——被丢下的拆分之后的子行认领会被误报，而接着重放就得吞掉 `apply` 的拒绝。重开参数：一个 run 能同时握多行时，一行过时不该挡住别的行，那时按行分组判。
41. **(a) 回复在视图锁外读，读出的块不缓存。** `prepare` 只把文字拷进 `Prepared::Reply`，`documents::reply` 在锁放开之后读（§8-29）。理由：读一窗是一次 comrak，与预览同阶，锁里多一次它，等着折叠的每一行都多等一次；同一段文字恒读出同一棵树，页面自己留着答复就够了。被否决的做法：①在 `prepare` 里当场作答——它不读盘，看上去是「视图里就答得了」的一类，但它的代价随文字长短走，不随视图走；②按文字的摘要缓存读出的块——一份要随快照编码的拷贝，换来的只是页面重问同一段文字时省一次 comrak，而页面不重问。**(b) 读内容库只有 `range::stored` 一处**，`Range` 与 `Preview` 都经它拿到编码与抬起的字节（§8-21、§8-23）。理由：打开、长度、标记、抬起这四步是「按版本读一个窗口」的前一半，两条查询的差别只在后一半（切还是读成块）；四步写两遍，改其中一处（例如内容库换了读法）时另一处不会跟着改。放在 `range` 而不另开模块，是因为 `range` 本来就拥有「按版本读一个窗口」，`preview` 是它的第二个读者。被否决的做法：①`preview` 先答一个 `Range` 再拿窗口的文字读——读法相同，却要把 `Window` 的文本再交回 `documents`，而 `documents::preview` 要的是抬起的字节，好在窗口之外判块末；②新开一个只有这一个函数的模块——两个读者都在 `answering` 里，一个函数不值得一个模块名。
42. **提案的原文由 run 引出，不给字节区间；工具经 lane 的 relay 写行；收回由工具自己那本小账判（§8-30）。**
    (a) 引文：模型读文件经 `read`，看到的是文字，不是字节偏移；它给出原话，区间由工具在那一版里找出，找不到或不止一处就拒，与 `edit` 的 `old` 同一种约定。被否决的做法：①收 `start`、`end` 字节偏移——模型要自己按编码数字节，数错一个就切进字符中间或切错句子；②收行号——要第二套「行怎样数」的规则，而 `documents` 的区间都按字节。
    (b) 写行经 relay：relay 是 lane 唯一能写账本的门（第 11 条），记账线程写下之后把这一行交给每个折叠，与 lane 写的 `tool_called` 同一条路。被否决的做法：把卡放进一张桌子、落地时由结算写——卡要等 run 结束才出现在人面前，而 `offer` 在行写下之前就把身份答给了模型。
    (c) 收回的判定在工具里，开着与否的权威仍是 `Governance`：工具只判「这是不是我提出的、我收回过没有」，这两件只有它知道。被否决的做法：派活时把 `Governance.proposals` 拷进工具——这次 run 在那一刻还没有任何一张卡，拷来的只会是别人的，而且拷贝是折叠的第二份。重开参数：§8-30 当前状态那一条在真实的城里出现。
