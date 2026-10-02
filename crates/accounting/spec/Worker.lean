-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::worker

规定 `crates/accounting/src/worker.rs` 与 `crates/accounting/src/worker/hands.rs`：城的唯一写者，和它从外面收下的手。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-11 accounting::worker：城的唯一写者，和它从外面收下的手（形状 1 数据 + 形状 4 适配器）

```rust
// accounting::worker::hands（形状 1 数据）
/// worker 伸向这台电脑的每一只手，构造时一次交进来。
pub struct Hands {
    pub vault: gateway::Custodian,                       // 生产：Custodian::probe 打开的那一个；脚本：Custodian::in_memory()
    pub clock: Arc<dyn Clock + Send + Sync>,             // §8-3
    pub monotonic: fn() -> Instant,                      // 量时长的单调钟：派活准备、mcp_tools、probe 的用时（sprawling-SPEC.md §8-129-2）
    pub machine: Box<dyn Machine + Send>,                // §8-4
    pub read_memory: fn() -> Memory,                     // sprawling-SPEC.md §8-46-3
    pub read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,   // sprawling-SPEC.md §8-116
    pub reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,         // sprawling-SPEC.md §8-60
    pub browsers: Browsers,                              // sprawling-SPEC.md §8-45-2
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
pub struct Health(/* 私有 */);   // 记账线程的两个计数，别的线程可读（sprawling-SPEC.md §8-98）

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

- **`Hands` 只装直接碰这台电脑的东西。** 墙钟、doctor、内存与卷的计数器、文件管理器、浏览器、正在运行的可执行文件、需求表、exec 的解释器与引擎，以及 vault。它们各自的生产实现住在 `sprawling`，由 `bin::assembly::production::hands` 一处装好。`ModelFactory` 与 `Connectors` 的生产实现不在里面：`GatewayModels` 与 `Residents` 随 worker 住在本 crate，由 `new` 装上（D18）。
- **构造器收一个值，不收九个参数。** 生产的调用方写 `RunWorker::new(root, log, bin::assembly::hands(vault))`；换掉一只手写 `Hands { clock: …, ..hands(vault) }`，或者构造之后调原有的 `with_*` 门。
- **worker 读的每一个时刻都经 `hands.clock`**，包括打开账本、`holding` 为 `last_tick` 取起点、`form` 写创世两行的时刻（§8-3）。
- **exec 的判定留在 worker，读数来自主机。** `limits.shell` 为假时 worker 根本不问 `exec_host.shell`；「坏掉的不交给 run」是 doctor 的判定，所以 `ExecHost` 的两个路径函数只交可用的那一份（`bin::doctor::host::usable_python_wasm`、`usable_shell`），引擎按本构建带不带 `sandbox` feature 由 `bin::doctor::host::execution_engine` 选。
- **装配根只留自由函数与直接碰主机的生产适配器。** `bin::assembly` 里剩下：`production`（上面四项）、`listening`（占端口、开写者）、`attending`（起写者线程、接上视图折叠线程与广播，交回 vault 与 `Health`）、`chain_watch`（起审计线程并报告）、`dropping`（拖进对话框的文件）。它们只经本节与 §8-10 列出的 `pub` 面碰 worker。
- **失败**：构造器与 `form` 原样传账本、CAS、`city` 与 `Standing::fold` 的 `AxError`，本节不另造错误码。
- **测试的手**：`accounting::worker::fixture::hands()` 交一份不碰主机的 `Hands`——内存里的 vault、读墙钟的测试钟、一台什么都没有的机器、宽裕的内存与卷、拒绝的文件管理器、空的浏览器表、拒绝的桌面程序、拒绝的需求表、没有解释器与 shell、`runtime::AbsentSandbox`。要真的某一只手的测试，自己换上那一只。
-/

/-! D10 没有状态的主机读写经构造时交进来的 `fn` 指针进来，和 `read_volume` 一样；只有持有状态的适配器（`Machine`、`Connectors`）才是 trait

理由：一个只包一个函数的 trait 没有第二个方法可换，脚本场景交一个自己的 `fn` 就够了，而且 `fn` 指针不装箱、不经虚表。被否决的做法：一个把内存、卷、随机令牌、打开文件管理器与浏览器捆在一起的 `Host` trait——这些做法的失败各不相同，脚本为了换掉其中一个就得实现全部。
-/

/-! D11 `relay`、`pool`、`desk` 与 `drive_run` 和 `RunWorker` 在同一次改动里搬

理由：它们成环——`relay` 经 worker 的账本写，`pool` 的每条车道跑 `drive_run`，`drive_run` 经 `relay` 写回，`desk` 为 worker 排队命令；先搬其中任何一个，都要一个指回留在 `sprawling` 的 worker 的临时端口，而下一次改动就会删掉它。被否决的做法：一个一个搬、中间架临时端口——每个临时适配器都是一个只活一次改动的第二权威。
-/

/-! D12 装配根留在 `bin::assembly`：只留自由函数与直接碰主机的生产适配器

`listening`、`attending` 的起线程那一半、`chain_watch` 的起审计线程那一半、`dropping` 与 `production`（`SystemClock`、`hands`、`init_city`、`form_city`）。理由：它们起线程、绑端口、造城的目录、装生产的手，是 ARCHITECTURE.md §3 说的知道每个具体类型的那一层；worker 搬走之后，它们对本 crate 的依赖是朝内的。`impl RunWorker` 的块一个也不留（D17）。被否决的做法：把 `genesis` 整个留在装配根——它的三个方法是 worker 的用例，而 worker 自己的测试要经它造城（D19）。
-/

/-! D17 `impl RunWorker` 的块全部住本 crate；装配根里混着两种东西的三个文件先拆开再搬

理由：Rust 只允许在定义类型的 crate 里写固有 `impl`，而 `attending`、`chain_watch`、`genesis` 各有一半读 worker 的私有字段。拆法按「谁起线程、谁碰主机」：循环 `attend`、挂 halt 的 `chain_under_audit`、造城的 `form` 与三个造楼方法随 worker 走；`spawn_worker`、审计线程、`init_city`/`form_city` 两个生产入口留下。被否决的做法：在 `sprawling` 里用扩展 trait 给 `RunWorker` 加方法——调用方要先把 trait 引进作用域，而 trait 方法仍然碰不到私有字段，只能再开公开的门。
-/

/-! D18 `GatewayModels` 与 `Residents` 随 worker 搬进本 crate，`Hands` 只装直接碰这台电脑的东西

理由：worker 自己的测试有上百处经生产的 `GatewayModels` 对一个回环地址上的假 provider 说话，经 `Residents` 连一个 conformance 子进程；两者留在 `sprawling`，测试就得在本 crate 再写一份 dialect 头加 `gateway::adapter_for`，那是「一个 `Chosen` 怎样变成适配器」的第二个权威。它们自己不碰主机：出网在 `gateway` 的适配器里，起进程在 `agent_protocols` 的链接里，worker 探端点、读 MCP 健康时本来就经这两个 crate 伸手。被否决的做法：把模型工厂与连接表也放进 `Hands`——测试的手要么复制生产实现，要么换成脚本，后者会改变几百条测试测的东西。
-/

/-! D19 worker 的测试经 `worker::genesis::form` 与 `fixture::hands()` 造城

理由：`init_city` 在 worker 的测试里被调用一百多次，它必须在本 crate 里可达；`form` 需要的时钟与 vault 本来就是交进来的手。`bin::assembly::production::init_city` 只把生产的 `Hands`（`Custodian::probe` 打开的 vault）交给同一个 `form`。被否决的做法：测试继续经 `sprawling` 造城——本 crate 不能依赖 `sprawling`，dev-dependency 成环会链接两份 `accounting`，类型对不上。
-/

/-! D20 宿主的手是一个值 `Hands`，由构造器收下

理由：搬过来以后 `new` 叫不出 `sprawling` 里的适配器，生产的那一份只能从外面来；九样东西总是一起到、一起用，是一个值（AGENTS.md）；参数上限是四个。换一只手有两种写法：结构体更新语法，或者构造之后的 `with_*` 门。被否决的做法：`Host` trait——D10 已否决，理由不变（脚本为换一只手要实现全部）；`fn` 指针组成的结构体没有这个代价。九个参数——超出 4 的上限，而且每个调用点都要把九样东西排一遍。
-/

/-! D21 vault 也放进 `Hands`

理由：生产的 vault 打开的是这台电脑的凭据服务（`Custodian::probe`），脚本给的是内存里的一份，它与其余几只手一样是构造时从外面交进来的；放进去以后三个构造器都不超过四个参数。被否决的做法：把 `vault` 与 `log` 捆成一个值——两者没有共同的意思，捆起来只是为了凑参数个数。
-/

/-! D22 驾驶 lane 的线程从 `accounting::worker::pool` 起

理由：`pool` 与 `relay`、`drive_run`、`RunWorker` 成环，必须一起搬（D11）；lane 的寿命仍然恰好是它驾驶的那个 run。ARCHITECTURE.md 的确定性规则 3 因此把它列为库 crate 起线程的第六处。被否决的做法：经 `Hands` 交一个起线程的 `fn`——它只有一个实现，而且只是把 `std::thread::Builder` 换个名字。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/sprawling-SPEC.md` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-4d | `accounting::worker::workbench::desktop` |
| §8-17 | `accounting::worker::folds::tests::vocabulary` |
| §8-31 | `accounting::worker::driving::placing`、`accounting::worker::driving::tests::placing` |
| §8-38 | `accounting::worker::desk` |
| §8-39 | `accounting::worker::lifetime`、`accounting::worker::fixture::rules`、`accounting::worker::fixture`、`accounting::worker::recording`、`accounting::worker::genesis`、`accounting::worker::genesis::tests`、`accounting::worker::genesis::adoption_tests`、`accounting::worker::naming`、`accounting::worker::folds`、`accounting::worker::folds::collaboration`、`accounting::worker::folds::tests`、`accounting::worker::folds::tests::standing`、`accounting::worker::folds::tests::history`、`accounting::worker::building_page_tests`、`accounting::worker::credentials`、`accounting::worker::credentials::signing`、`accounting::worker::credentials::endpoints`、`accounting::worker::credentials::environment`、`accounting::worker::credentials::tests`、`accounting::worker::credentials::tests::signing`、`accounting::worker::mcp`、`accounting::worker::dispatching`、`accounting::worker::dispatching::agreeing`、`accounting::worker::dispatching::running`、`accounting::worker::dispatching::handback`、`accounting::worker::dispatching::tests`、`accounting::worker::waking`、`accounting::worker::waking::tests`、`accounting::worker::workbench`、`accounting::worker::workbench::standing`、`accounting::worker::workbench::desks`、`accounting::worker::workbench::servers`、`accounting::worker::workbench::tests`、`accounting::worker::freezing::frozen_handoff`、`accounting::worker::freezing`、`accounting::worker::freezing::tests`、`accounting::worker::freezing::tests::prefix`、`accounting::worker::freezing::tests::dispatches`、`accounting::worker::freezing::tests::lineage`、`accounting::worker::freezing::tests::ceilings`、`accounting::worker::driving::tests`、`accounting::worker::driving::tests::turns`、`accounting::worker::driving::tests::confidential`、`accounting::worker::driving::tests::ledger`、`accounting::worker::driving::tests::sieving`、`accounting::worker::settling`、`accounting::worker::settling::desks`、`accounting::worker::settling::landing`、`accounting::worker::settling::tests`、`accounting::worker::settling::tests::ending`、`accounting::worker::settling::tests::landing`、`accounting::worker::settling::tests::succession`、`accounting::worker::settling::tests::halting`、`accounting::worker::reviewing`、`accounting::worker::probing`、`accounting::worker::probing::probe`、`accounting::worker::reviewing::tests`、`accounting::worker::reviewing::tests::kept`、`accounting::worker::reviewing::tests::landing`、`accounting::worker::reviewing::tests::refusal`、`accounting::worker::plans`、`accounting::worker::plans::tests`、`accounting::worker::plans::tests::rows`、`accounting::worker::plans::tests::goals`、`accounting::worker::plans::tests::graph`、`accounting::worker::commanding::routing`、`accounting::worker::commanding::governing`、`accounting::worker::commanding::tests`、`accounting::worker::commanding::tests::answering`、`accounting::worker::commanding::tests::clockwork`、`accounting::worker::commanding::tests::entrance` |
| §8-40 | `accounting::worker::driving::tests::rules_account` |
| §8-41 | `accounting::worker::freezing::run_slot`、`accounting::worker::commanding`、`accounting::worker::commanding::door`、`accounting::worker::commanding::entrance` |
| §8-42 | `accounting::worker::booking`、`accounting::worker::registering`、`accounting::worker::relay` |
| §8-43 | `accounting::worker::driving` |
| §8-46 | `accounting::worker::fixture::provider`、`accounting::worker::rooms`、`accounting::worker::rooms::tests`、`accounting::worker::driving::owing`、`accounting::worker::driving::flight`、`accounting::worker::driving::entering`、`accounting::worker::driving::lane`、`accounting::worker::driving::tests::flight`、`accounting::worker::settling::landing::discharging`、`accounting::worker::settling::landing::filing`、`accounting::worker::plans::pursuing`、`accounting::worker::pool` |
| §8-50 | `accounting::worker::credentials::tests::endpoints`、`accounting::worker::workbench::engine` |
| §8-55 | `accounting::worker::commanding::configure` |
| §8-60 | `accounting::worker::commanding::tests::revealing` |
| §8-62 | `accounting::worker::credentials::probing`、`accounting::worker::credentials::tests::probing` |
| §8-64 | `accounting::worker::commanding::machine` |
| §8-71 | `accounting::worker::credentials::endpoints::choosing` |
| §8-79 | `accounting::worker::dispatching::session_shape`、`accounting::worker::dispatching::session_shape::tests` |
| §8-81 | `accounting::worker::credentials::tests::kept` |
| §8-82 | `accounting::worker::folds::session`、`accounting::worker::freezing::inherited`、`accounting::worker::commanding::sessions`、`accounting::worker::commanding::sessions::tests`、`accounting::worker::commanding::sessions::tests::origin` |
| §8-84 | `accounting::worker::driving::tests::instruments` |
| §8-85 | `accounting::worker::freezing::model_note`、`accounting::worker::freezing::tests::model_note` |
| §8-86 | `accounting::worker::dispatching::session` |
| §8-87 | `accounting::worker::dispatching::custody`、`accounting::worker::dispatching::custody::tests` |
| §8-90 | `accounting::worker::chain_halt` |
| §8-91 | `accounting::worker::folds::views_start`、`accounting::worker::folds::views_start::tests` |
| §8-98 | `accounting::worker::health` |
| §8-101 | `accounting::worker::folds::standing_start`、`accounting::worker::folds::standing_start::tests` |
| §8-107 | `accounting::worker::commanding::restoring`、`accounting::worker::commanding::tests::restoring` |
| §8-110 | `accounting::worker::credentials::held`、`accounting::worker::collaborating` |
| §8-111 | `accounting::worker::doorstep`、`accounting::worker::plans::held`、`accounting::worker::plans::tests::holders` |
| §8-112 | `accounting::worker::keeping_warm` |
| §8-113 | `accounting::worker::dispatching::preparing` |
| §8-114 | `accounting::worker::commanding::removing`、`accounting::worker::commanding::removing::tests` |
| §8-116 | `accounting::worker::commanding::shedding`、`accounting::worker::commanding::tests::shedding` |
| §8-121 | `accounting::worker::opening_cost` |
| §8-124 | `accounting::worker::dispatching::harness`、`accounting::worker::dispatching::harness::tests`、`accounting::worker::dispatching::harness::tests::turn`、`accounting::worker::driving::harness`、`accounting::worker::driving::harness::turn` |
| §8-133 | `accounting::worker::dispatching::tests::experiment` |
| §8-145 | `accounting::worker::dispatching::preparing::tests` |
-/
