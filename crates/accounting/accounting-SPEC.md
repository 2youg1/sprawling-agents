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

目标形状里还有一个端口与一批搬迁，现在都还不在本 crate：

| 端口 | 它回答的问题 | 现在的住处 |
|---|---|---|
| `Machine` | city 所在的主机上有哪些工具，以及装上一个 | `bin::doctor::probe::Machine`，`pub(crate)` |

`RunWorker` 的六个对象（凭据、协作、计划、治理、入口、飞行中的 run）、全部用例与 `views` 仍在 `crates/sprawling/src/assembly` 与 `crates/sprawling/src/views`。

## 2 验收标准

三条断言，都在 `crates/sprawling/tests/model_factory.rs`，都驱动一个接收了脚本 `ModelFactory` 的 `RunWorker`：

| 测试 | 它钉住的事 |
|---|---|
| `a_dispatch_reaches_the_model_the_worker_was_handed` | run 的模型走端口。端点指向一个拒绝连接的 loopback 端口，所以只要 worker 还自己造适配器，这次 run 就碰不到脚本模型，历史里也就没有脚本写下的那句回答。 |
| `an_unnamed_dispatch_asks_the_factory_for_the_run_model_alone` | 没有 session 的 dispatch 按规则从任务的词里取房间名（sprawling-SPEC.md 8-86），不做命名调用：工厂只被问到 run 的主模型，run 在规则给出的房间里开始。 |
| `a_confidential_building_refuses_before_the_factory_is_asked` | 机密楼的拒绝在 worker 的选择里，不在工厂里：端点不是 loopback 地址时 dispatch 以 `GateDenied` 被拒，脚本工厂一次也没被问到。换掉工厂不能绕开机密。 |

第四条在 `crates/sprawling/tests/connectors.rs`：`a_run_is_offered_the_tools_the_worker_was_handed`。楼的配置写了一个 MCP server，它的命令在任何主机上都不存在；worker 接收了一个脚本 `Connectors`，它给出一个工具。只要 worker 还自己启动 server，这个 server 就起不来，模型收到的工具表里也就没有那个工具。

第五条在 `crates/sprawling/tests/clock.rs`：`a_worker_stamps_its_lines_with_the_clock_it_was_handed`。worker 接收一个停在固定时刻的脚本 `Clock` 之后写下的每一行，`t` 都是那个时刻；只要还有一个写点自己读墙钟，这一行的 `t` 就是现在的时间。

## 3 假设与歧义

- `Machine` 端口的签名还没定。worker 用到的两处是 `doctor_install`（跑一条人同意过的安装命令）与 `look_at_this_machine`（`doctor::report()`，交回 `channels::DoctorAnswer`）。未定的是：`install` 收 `bin::doctor::Runnable`——它只由 `Recipe::command` 造出，持有它就证明这条命令被问过——还是收 program 与 args；前者要求 `Runnable` 与 `Requirement`、`Presence` 一起搬到本 crate 或更低处。能定下它的证据是：`views`／`doctor`／`serving` 之间的环断开之后（sprawling-SPEC.md 8-92 的表），`doctor` 的类型还被谁用。红测不能对着现在的代码去跑 `DoctorInstall`，因为那会在宿主上真的启动包管理器；它要从 `DoctorRefresh` 进，看诊断里 worker 数到的条目数。
- `views` 在读侧与写侧各有一份 `Governance`，搬进本 crate 时哪一侧拥有这个类型，取决于 `bin::views` 与 `bin::assembly` 之间的环断在哪里（sprawling-SPEC.md 8-92 的表）。

## 4 现状分析

本 crate 现有三个端口。`ModelFactory` 的生产适配器在装配根（`bin::assembly::models`），第二实现在 `crates/sprawling/tests/model_factory.rs`；`Connectors` 的生产适配器是 `bin::assembly::mcp::Residents`，第二实现在 `crates/sprawling/tests/connectors.rs`；`Clock` 的生产适配器是 `bin::assembly::SystemClock`，第二实现在 `crates/sprawling/tests/clock.rs`。

## 5 权威信源

ARCHITECTURE.md §3（依赖律与 `depmap`）、§4（端口表）、§11（V6 缺口）；`gateway::adapter_for`（一个 `Chosen` 怎样变成一个适配器）。

## 6 命名统一

ModelFactory｜Connectors｜Clock｜accounting thread｜Chosen｜Redemption。「适配器」专指 `kernel::Model` 的一个实现；「端口」专指本 crate 声明、外层实现的 trait。

## 7 模块边界

- 怎样按 endpoint、dialect、凭据造出一个适配器，归 `gateway::adapter_for`：本 crate 只声明「造一个」这个动作。
- 挑哪个模型（`EndpointBook::select`）与何时续期凭据，归 `RunWorker`：端口拿到的是已经选好的 `Chosen` 与已经兑换好的 `Redemption`。
- 把生产适配器接到 worker 上，归装配根 `bin::assembly`。
- MCP 的生命周期（`initialize`、`notifications/initialized`、`tools/list`）怎样说，归 `protocol`；一个工具能不能在机密楼里存在，归 `protocol::McpTool::new`。端口只声明「连上一个 server，交回它的工具」。
- 机密楼根本不启动 server，这一步在 worker 的 `mcp_tools` 里、端口被问到之前。

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

## 12 决策

1. **生产适配器住装配根，不住本 crate。** 理由：它把 `gateway` 的具体构造接到端口上，这正是 ARCHITECTURE.md §3 说的装配边；本 crate 只依赖 `kernel` 与 `gateway` 的接口类型。被否决的做法：在 `gateway` 里实现本 trait——那要让 `gateway` 依赖 `accounting`，依赖就朝外指了。
2. **`with_models` 是一个消费 `self` 的方法，而不是 `new` 的第四个参数。** 理由：生产只有一种工厂，`new` 的每个调用方（serve、doctor、测试）都会写同一个 `GatewayModels`；换工厂的只有 citysim 与测试。被否决的做法：`new` 加参数——四个调用点重复同一个值，而这个值只有一个权威。
3. **端口参数是 `Chosen` 与 `Redemption`，不含 dialect 头。** 理由：dialect 头由 `Chosen` 的 dialect 决定，把它交给调用方算，两个调用点就各有一份拼法。被否决的做法：照抄 `gateway::adapter_for` 的三参数签名。
4. **`Connectors` 交回整条连接（握手之后的工具与握手结果），而不是一个裸的 `protocol::Outbound`。** 理由：一个 server 的每个工具各持有同一条链接的一份克隆，而 `Outbound` 是 trait object，不能克隆；交回裸链接，worker 就得再要一个「造链接」的工厂。被否决的做法：端口只负责 `McpLink::open`——那要多一个端口，而握手的说法本来就归 `protocol`，不归 worker。
5. **`Connectors::connect` 取 `&mut self` 并交回 `Reached`，生产适配器就是常驻连接表 `Residents`。** 理由：常驻表要持有链接本身，才能判断子进程是否已经退出、并按 `confidential` 重新铸出工具；一个只交回工具的无状态端口挡在表前面，表就看不到链接。被否决的做法：无状态端口加 worker 侧的缓存——缓存只能存工具，存不下判断存活所需的链接。
