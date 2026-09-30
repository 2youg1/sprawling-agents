# citysim-SPEC.md

> 工作区成员：`citysim`（dev-only，第二个 Main，不占产品拓扑）。本 SPEC 先于代码存在。
> 内容：内存 Ledger 与链检查器（§8）、剧本设施与薄执行器（§8-2）、真适配器换入（§8-3）、两个测量二进制（§8-5、§8-6）与评估仪器（§8-7、§8-8）。

## 1 需求分解

| 件 | 一句话 |
|---|---|
| `mem_ledger` | kernel Ledger 的第二适配器：全内存、确定性、conformance 的对照实现 |
| `checker` | 不变量检查器：链完整且 seq 连续（`check_chain`，复用 `runtime::replay::verify_lines`） |
| `script_model` | kernel Model 的第二适配器：脚本驱动的 ModelReturn 序列，确定性 |
| `script_tools` | kernel Tool 的第二适配器：脚本工具加按名分发，全失败模式可注入 |
| `executor` | 薄执行器：把剧本世界（计数时钟、剧本中断、检查点网、工具台）交给 `runtime::run::drive`，驱动到 `run_frozen` |
| `sieving` | 场景里的 `exec` 结果经筛子打包（`SieveWorld`） |
| `red_team` | 有无验证 run 两臂的结论质量（§8-7） |
| `suite`、`score`、`metabolism`、`nesting`、`ablation` | 评估仪器（§8-8）；除 `suite` 外只在测试下编译 |
| `bin/bench` | 负载场景与读数行（§8-6） |
| `bin/bench_startup` | 四个动作的冷启动测量（§8-5） |

## 2 验收标准

- `MemLedger` 过 kernel conformance 六断言（与 JsonlLedger 同一套——这就是「缝」的兑现）。
- 同一 draft 序列灌 MemLedger 与 JsonlLedger，raw 行逐字节相同（规范字节住 kernel 的实证）。
- checker 对合法序列静默通过；对篡改序列报出首个断点行。
- 跨 OS 字节夹具：`tools/fixtures/golden-s1/` 的脚本化序列重建后与夹具逐字节相同（CI 三平台恒跑同一断言）。

## 3 假设与歧义

citysim 的模块登记在 `architecture.toml`，但 `modmap` 只判 `crates/` 下的条目，故本 crate 的条目写给读者看、不受那道门判；MPL 头、lexicon、lints 全库同规。`just sim` 的入口是本 crate 测试（固定剧本即测试用例）。**本 crate 没有随机源，也没有种子**：确定性由三件事持有——剧本是写死的、时钟是 `executor` 里的 tick 计数器、执行是单线程；复现一次失败靠的是重跑那个剧本。Dispatch 的「先落 JOB.md 再产事件」在 sim 里以 `checkpoint_committed`（确定性假 oid，由 JOB.md 内容的 B3Hash 派生前 40 个 hex 字符）代文件面——模拟适配器的职责即伪造外部世界，事件序与真城同形。

### 3-1 决定：没有种子，而不是造一个吃种子的生成器

种子此刻没有消费者：剧本是测试用例，随机剧本批要等故障面有东西可随机，现在造一个生成器等于先立一个没有被任何断言驱动的第二权威。故 `justfile` 的 `sim` recipe 不接参数，crate 文档写的是计数时钟与固定剧本。**重开参数**：随机剧本批落地时，种子成为 `Scenario` 的一个字段，由它派生每一处分叉，`sim` recipe 同批接回一个参数。

### 3-2 决定：计时边界取「动作的可观察端点」，进程动作以退出为端点

四动作里两个跨进程（安装的落位确认、启动）：端点是被拉起进程**退出被观察到**，因为「可接受命令」在产品外部可观察的最短证据就是一条轻命令被应答完毕。落盘动作（建城、开 session）以公面调用**返回**为端点，因为返回即账本已带自身屏障落盘（落账先于效果）。被击败的备选：以进程内部时点（参数解析完成、监听就绪）为端点——那要在产品里插桩，为测量加一条不发货的分支，改写被测路径。

### 3-3 决定：安装边界含归档摘要校验、不含 PATH 写入

摘要校验（sha256）是 `install.sh`／`install.ps1` 从归档就位到解包之间必经的一步，删掉它测的就是不验摘要的安装，故计为安装的子步并单列读数。签名验签不在这次测量里：发行件签名的验签侧有设计而签名动作未接（xtask-SPEC §8-29），读数里这一子步记 **0** 并注明；签名动作接上之后，此子步只增不删。PATH 写入（`sprawling install` 的注册表写与桌面广播）在边界外：它是一次性的桌面状态写入，第二次运行幂等（`PathEdit::AlreadyPresent`），计进每样本会把桌面状态写入误报成安装成本。被击败的备选：整段 `install.sh` 全测——含网络下载与 shell 启动，而网络不在本族的计时口径内。

### 3-4 决定：计量主语是 Rust measuring Main，不是 tools/adversary/ 也不是 criterion

四动作零行为断言，只计时；`tools/adversary/` 量化行为轨迹，Lean 侧不为墙钟定价。criterion 会是第二套仪表：本族挂 `just bench` 族，同一 wall-clock 口径（测而不门）。它拉起产品二进制——被测动作本身即进程边界；boundary 门判的是**检查**站哪一侧，其越过面 token（`CARGO_BIN_EXE`／`SPRAWLING_BIN` 等）本族一个不写，被测二进制取自构建档目录（`cargo build` 同时放置两个产物的地方），`just bench-startup` 先构建后测量，故不接手工路径也不会测到旧产物。被击败的备选：把四动作写进 `tools/adversary/`——那里没有秒表也没有本仓词汇，量出来的东西无法与 `just bench` 对表。

### 3-5 决定：被测可执行文件的名字在本 crate 只重述一处，注释点名它的权威

`executable_name()` 拼的是 `install.rs` 装出来的那个名字：`INSTALLED_STEM` 加本平台后缀。该事实的权威是 `tools/xtask/src/platform.rs` 每平台的 `binary` 字段，`cargo xtask artifact` 把发行侧的四种拼法（工作流矩阵、两个安装脚本、npm shim）钉在它上面；citysim 这一处不在那四种之内，它是唯一需要这个名字的**测量**读者。够不到权威的原因是位置而非取舍：`install` 模块住在 `crates/sprawling/src/main.rs`，二进制的模块不可 import，而 `xtask` 是工具不是依赖。本 crate 内只留这一处拼写——`shipped_binary` 找的路径名与 `archive_of` 写出的 zip 成员名都读它。**重开参数**：这个名字若移进 `sprawling` lib 成为公共面，本函数改为读它，重述随之删除。

### 3-6 决定：评估仪器住在 citysim，不另立 crate

五件仪器（§8-8）没有产品调用点：`suite` 只由 `tests/evaluation.rs` 驱动，其余四件只由自己的测试驱动。为它们在产品拓扑里立一个 crate，换来的是一个不进二进制却占一格依赖图的单元，以及 `sprawling` 为一个交接探针多背一条边。citysim 本就是 dev-only 的第二个 Main，仪器与剧本同住，产品图少一个单元。交接探针有生产调用点，归它的拥有者 `accounting::worker::probing`（sprawling-SPEC §8-39）。落选方案：独立的 `eval` crate——它唯一的生产面是那个探针。**重开条件**：一件仪器得到生产调用点。

### 3-7 决定：场景只在回合边界取消

`CancelPoint` 三个变体（`BeforeAssemble`、`BeforeCall`、`BeforeWave`，各带 `turn`）对应 `runtime::run::SafePoint` 里按回合编号的三个；`SafePoint` 另有 `BeforeToolCall` 与 `BeforeSpawn`，它们在一个回合里被问很多次，剧本按回合编号够不到它们，执行器在这两处恒答 `Interrupt::None`（`executor::answer_at`）。被击败的备选：给 `CancelPoint` 补齐五个变体——那要给回合内的每一次问询编号，而剧本只写回合。**重开参数**：一条剧本需要在一波中途取消时，`CancelPoint` 加一个带调用序号的变体。

### 3-8 决定：读数带着它所量字节的摘要，登记的夹具钉住这个摘要

两条读数只在量的是同一份字节时可比。bench 共用的 `draft` 是写在代码里的负载形状，`Fixture` 的字段说不出它：改了 `draft`，同一张登记表里前后两条读数量的就是两份负载，而读数行本身看不出差别。所以登记的夹具有一个摘要：`draft` 的前 `PINNED_DRAFTS`（1,000）行经 `storage::JsonlLedger` 写成一本账，取这本账的 `ledger_digest`，再把这 32 字节与 `Fixture` 各数值字段的小端字节拼在一起取一次摘要。这个值钉在 `REGISTERED.pinned`。bench 在量任何东西之前先算一遍，与钉住的值不等就拒绝测量；`scenarios/tests.rs` 的 `the_registered_fixture_writes_the_bytes_its_digest_pins` 在 `just check` 里做同一件事。改 `draft` 或改一个字段的提交因此必须同时改钉住的值，并且单独成一个提交，与 `[local_latency]` 行「加胖夹具单独提交」是同一条纪律。每条 `perf` 读数行带 `fixture=<摘要的前 16 位十六进制>`，这 16 位只由 `citysim::fixture_label` 拼出。

`bench_startup` 的夹具城不钉：`init_city` 用真实时钟写创世行，每次生成的字节都不同，而后面每一行的 `prev` 都接着它。它的读数照样带那座城账本的 `ledger_digest`，两条首字节读数只在摘要相等时可比；夹具城在构建档目录旁复用，所以同一台机器上前后两次读数通常量的是同一份字节。

被否：摘要只打印、不钉。打印出来的值要靠人去比，而在 `just check` 里变红的测试不需要谁记得去比。被否：给 `init_city` 加一个时钟参数，好让夹具城也能钉住。那是为 bench 给产品的创城入口开一个参数，而复用同一座夹具城已经让同一台机器上的读数可比。**重开参数**：需要跨机器比较首字节读数时，夹具城改为从一份检入的账本复制，而不是在量它的主机上生成。

### 3-9 决定：被量的产品 feature 集只写在 justfile 一处

人下载的二进制带执行引擎（`sprawling` 包的 `sandbox` feature）。justfile 的变量 `product_features` 是这套 feature 的唯一写法：`dist`、`bench`、`bench-startup`、`mem` 四个 recipe 都用它构建，所以 install 解包的、startup 拉起的、首字节服务的、内存读数量到的，与人下载的是同一个二进制，`bench` 里的场景与仪表也在同一套 feature 下编译。被否：每个 recipe 自己写 `--features`。漏写的那个 recipe 量的是一个没人下载的二进制，而读数本身看不出它漏了。**重开参数**：`sandbox` 成为 `sprawling` 包的默认 feature 时，这个变量与它的四处引用一起删去。

## 4 现状分析

`just sim` 跑全部场景测试，单线程、无 I/O 等待，耗时由编译主导。测量二进制（§8-5、§8-6）的读数写在 `tools/xtask/budgets.toml` 各自的行里，只入册不入门。

## 5 权威信源

kernel-SPEC §8-9（Ledger 缝与 conformance）；runtime-SPEC §8-1（驱动器）与 §8-39（每 run 一条 `prompt_assembled`）；`tools/xtask/budgets.toml`（测量读数）。

## 6 命名统一

MemLedger、checker、Scenario、CancelPoint、ScenarioReport；事件名取 kernel 的 `EventKind`。

## 7 模块边界

```
mem_ledger ──▶ kernel（Ledger trait＋event＋conformance feature）
checker    ──▶ runtime::replay（verify_lines 复用，不建第二验证权威）
executor   ──▶ runtime::run::drive（循环只住 runtime，本 crate 只供世界）
```

**不做什么**：`MemLedger` 不落盘；不采时钟（t 由 tick 计数器给出）；不另写 FaultFs（它住 storage）。

## 8 接口先行

```rust
// 8-1 mem_ledger 与 checker（形状 4 适配器＋形状 1 判定）
pub struct MemLedger { /* lines: Vec<Vec<u8>>, next_seq, prev */ }
impl MemLedger { pub fn new() -> Self;  pub fn raw_lines(&self) -> &[Vec<u8>]; }
impl kernel::Ledger for MemLedger { … }
impl kernel::conformance::LedgerInspect for MemLedger { … }   // citysim 恒开 conformance feature

/// Invariant: chain intact, seq contiguous.
pub fn check_chain(lines: Vec<Vec<u8>>) -> Result<(), AxError>;   // replay::verify_lines 薄封
```

### 8-2 剧本适配器与薄执行器

```rust
pub struct ScriptModel { /* VecDeque<ModelReturn> */ }
impl ScriptModel { pub fn new(script: Vec<ModelReturn>) -> Self; }
impl kernel::Model for ScriptModel { /* 逐次弹出；耗尽后恒回空 calls（自然收束） */ }

pub struct ScriptTool { /* meta、outcomes: VecDeque<Result<ToolOutcome, AxError>> */ }   // impl kernel::Tool
pub struct ScriptToolSet { /* BTreeMap<String, ScriptTool> */ }
                     // 未知工具名 → E_TOOL_UNKNOWN＋nearby＝已注册名；耗尽脚本 → E_TOOL_UNAVAILABLE

pub enum CancelPoint { BeforeAssemble { turn: u32 }, BeforeCall { turn: u32 }, BeforeWave { turn: u32 } }
pub struct Scenario { pub run: RunId, pub who: String, pub addr: Address, pub task: String,
                      pub goal: String, pub job_md: String, pub model: ScriptModel,
                      pub bench: ToolBench, pub config: FrozenConfig,
                      pub checkpoint: Option<(Checkpoint, Vec<String>)>,
                      pub cancel: Option<CancelPoint>, pub steer: Option<(u32, String)>,
                      pub sieve: Option<SieveWorld> }
                      // run/who 由剧本注入（citysim 无随机）；job_md 是 JOB.md 内容，假 oid 由其哈希派生
pub struct ScenarioReport { pub lines: Vec<Vec<u8>>, pub completion: &'static str /* Completion::name() */ }
/// Deterministic: t 单调递增每步 +1ms，无时钟采样；无随机。
pub fn run_scenario(scenario: Scenario) -> Result<ScenarioReport, AxError>;
/// 在一本已有历史的账本上续跑：两个 run 同链，seq 按一座城写下的次序排。
pub fn run_scenario_on(ledger: &mut MemLedger, scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

- 事件序（无取消正常收束）：`checkpoint_committed`（JOB.md 先落）→ `run_started` → `prompt_assembled`（每 run 一条，runtime-SPEC §8-39 第 5 条）→ 每回合 `prompt_shape_compared→model_called→model_returned[→tool_called→tool_result]*` → 空 calls 回合后 `handoff_written` → `run_frozen{completion:done, evidence:[末 model_returned]}`。
- 取消在指定边界注入 `Interrupt::Cancel`（§3-7）：事件序断言是 `cancel_received` 后无新 `model_called`／`tool_called`，且 `handoff_written` 恒先于 `run_frozen`，三个边界各一条剧本。
- `steer` 在给定回合的波边界递一句人话：它追加到下一个结果里，不打断正在进行的动作，故剧本断言循环照常继续。同一边界上取消压过 steer。
- 回合数没有上限：驱动器跑到模型回空 calls 或被取消为止（`runtime::run::drive`）。

### 8-3 真适配器换入（单 Resident 全链走完一条回路）

剧本仍确定性（无时钟采样、无网络、无随机），四个替换点接的是产品本身：

| 替换点 | 接的是 |
|---|---|
| Ledger | MemLedger（真 jsonl 对拍已在 conformance；换盘不增新证据） |
| 工具面 | 真 `ToolBench`（edit、status、exec 的 Program 臂）对 tempdir 城根；门路由在回合层 |
| 模型 | `ScriptModel` 登录 wire 形：剧本写 Anthropic wire JSON，经 `gateway::dialect::response_from_wire` 解成 canonical 再出 ModelReturn（翻译面进链路） |
| 时钟／配置 | 逐步 +1ms，加 `FrozenConfig` 求值（clock_stamp 三层覆盖）接入 `StampGate` |

门路由归 `ToolBench`，写域住 bench 内，executor 不手写 domain 门。`ScriptToolSet` 是 `kernel::tool` 缝的第二适配器（已登记的 conformance 证据），**注册进真 ToolBench**，于是脚本工具与真 L0 工具走同一条门路由。波前的检查点是**每波一次**而非只在 exec forecast 命中时：`ToolBench` 内的 forecast 检查点是它在 exec 臂上的加强，两者不互相替代。空波仍提交（同树 oid），因为链可重建优于省一次提交。tool_result 的信封由 executor 挂（`pipeline::package` 加 `StampGate`），与 serve 同位。

事件序断言：edit 成功的波携 `checkpoint_committed`（波前，断言形：每个 `tool_called` 之前最近的 `checkpoint_committed` 晚于最近的 `model_returned`）；tool_result 信封可携 ClockStamp（非 Off 时）；越域写被 domain 门拒且 refusal 以 tool_result 回流；链恒可验；双跑字节对拍。gateway 进 sim 的只有 dialect 翻译面（纯函数，确定性保持）；endpoint 的 HTTP 面不入 sim（网络即非确定），其验证住 gateway 自身的回环假服务测试（gateway-SPEC §2）。

### 8-4 一次工具调用的键：每跑一个的位次，加上整个动作的字节

```rust
// citysim::executor——驱动块内
let placed = Cell::new(0u64);              // 位次：每跑一个计数器，与 accounting::worker 同形
let key = IdemKey::derive(&run, Seq::new(at), &call.action()?);   // 动作字节：kernel::tool 唯一一份
```

**动作字节取 `ToolCall::action`**（kernel-SPEC §8-23，全库唯一一份），**位次取每跑一个的计数器**，与 `accounting::worker` 同形，故两个驱动器对「一次工具调用的键怎么算」只有一份读法。被击败的读法：位次取回合时钟 `t`、动作字节只取工具名。`runtime::run` 给一波里的每次调用同一个 `t`（一波是一个瞬间），于是一波之内两次同名调用拿到相同的 `IdemKey`，`ToolBench::invoke` 的去重把第二次判成重复，参数不同也不救；而且那个 `Seq` 是钟读数换了个类型，违反确定性规则「位次不从时钟来」。

`two_reads_in_one_wave_are_two_calls` 钉住这一条：一个回合携两次同名、参数不同的调用，两条 `tool_result` 都带结果、都不带 `error`。`IdemKey` 不进任何 payload，故账本字节与 `golden-p0` 不受它影响。

### 8-5 bench_startup 族：四动作压档的测量面（只实测，不优化）

四动作各给三件套（能否进 p99≤1ms／极限读数／主导成本件），本族只测不优化。本族是 `just bench` 的同族仪表：同一 citysim bin 面、同一 wall-clock 口径（测而不门，读数标注机器类属）。四行读数落在 `tools/xtask/budgets.toml` 的 `[install]`／`[startup]`／`[raise_city]`／`[open_session]`，无预算键故不门；机器类属、样本数与四个动作的子指标数字都写在行内，本 SPEC 不重抄它们。

```rust
// tools/citysim/src/bin/bench_startup.rs —— measuring Main：本族唯一计时采样点（`stamp()`，同 bin::bench 先例）；`SAMPLES` 是每动作次数的唯一之家，循环、预分配与报告同读它
// tools/citysim/src/bin/bench_startup/samples.rs —— shape: decision（时间以 Duration 入参；无时钟、无 I/O）
pub struct Samples { /* 一次构造点持有 ≥1 个 Duration；invalid state 不可拼写 */ }
pub enum Share { P50, P95, P99 }
pub enum SampleKind { Plain, Suspicious }   // > 3 × p50 标可疑：Defender 实时扫描等外扰的标注位
pub enum Tier { Within, Outside }           // 第二档：p99 ≤ 1 ms
impl Samples {
    pub fn of(head: Duration, tail: Vec<Duration>) -> Samples;
    pub fn p(&self, Share) -> Duration;     // nearest-rank
    pub fn floor(&self) -> Duration;  pub fn peak(&self) -> Duration;
    pub fn kind_at(&self, usize) -> SampleKind;
    pub fn suspicious(&self) -> usize;
    pub fn tier(&self) -> Tier;
}
// tools/citysim/src/bin/bench_startup/actions.rs —— shape: adapter（薄驱动产品公面，口径即边界；两处跨进程动作走的都是产品自己的路径）
pub const SAMPLES: usize;
pub struct PerSample { pub processes: u64, pub files: u64, pub barriers: u64 }
pub struct Action { pub total: Samples, pub steps: Vec<(&'static str, Samples)>, pub per_sample: PerSample }
pub fn install(scratch: &Path, archive_path: &Path) -> Result<Action, AxError>;
pub fn startup(binary: &Path) -> Result<Action, AxError>;
pub fn raise_city(scratch: &Path) -> Result<Action, AxError>;
pub fn open_session(city: &Path) -> Result<Action, AxError>;
pub fn dominant(steps: &[(&'static str, Samples)]) -> Option<&'static str>;
// tools/citysim/src/bin/bench_startup/actions/archive.rs —— shape: adapter（发行档的格式读写都在这里：摘要、解包、写出 fixture 条目；`sha2`／`zip` 各在 workspace 清单里一个名字，打包步与本次读数不会对它们生出第二种拼法）
// tools/citysim/src/bin/bench_startup/actions/footprint.rs —— shape: adapter（一次 walk 供所有读者：一个样本在一棵树下写了多少文件、多少账行、某个名字在哪、以及样本之间如何把目录清平）
```

**四个计时边界**（起止、子步、决定见 §3-2／§3-3）。样本 200／动作（≥100）；p50/p95/p99 取 nearest-rank；读数全样本给出（基线永不减除），可疑样本只标注不剔除。

| 动作 | 起点 | 终点 | 子步 |
|---|---|---|---|
| ① install | 归档已就位（zip 在 scratch，网络不计） | 解出的可执行文件拉起 `version` 应答并退出 | 归档摘要校验（sha256）｜解包写可执行文件｜落位确认（进程创建＋运行到退出） |
| ② startup | `CreateProcess` 发出 | 轻命令 `version` 退出被观察到 | 进程创建（spawn 返回）｜程序运行（返回→退出） |
| ③ raise_city | `assembly::init_city` 调用发出 | 调用返回（创世记录落盘，每条账各带自己的屏障） | 无子步切分（不插桩产品）；主导件由计数×地板归因 |
| ④ open_session | `RunWorker::handle(Command::OpenSession)` 发出 | 调用返回（`session_opened` 已落账，房内下一 run 可开工即可接输入） | 同上 |

**子指标拆分（各自计数，先行）**：进程创建（①1／样、②1／样、③0、④0，时间取子步）；文件创建（`PerSample.files`：①＝解包写出的文件数，③＝创世城市树的文件数，④＝首样本前后 city 文件数之差；②自身不落盘，记 0 而不是把它被指向的那棵树算进来）与耐久屏障（`PerSample.barriers`，一账一屏障，地板引用 `just bench` 的 `durability_barrier` 行，不另起第二仪表）；验签哈希＝0（签名动作未接，记 0 并注明，见 §3-3）；Defender 实时扫描干扰＝可疑样本数与下标（`SampleKind`），①③ 每样本全新首触（必扫），② 复用同一映像（首样本后转热）。

**失败**：产品公面的失败原样抛 `AxError`，不新增码；测量自体的失败（被测二进制不在构建档目录等）用既有码走三段式（动作/主体/`AxCode`/recovery）。

#### 8-5-1 首字节：`sprawling serve` 拉起到第一个字节（`ttfb`）

四个动作之外的第五行读数，量的是人开一座城要等多久：`CreateProcess` 发出（`serve <城> 127.0.0.1:<端口> --no-console --no-open`，`SPRAWLING_OPEN=never`）到对 `GET /` 读到第一个字节。端点取第一个字节而不是端口开始监听，因为人看见的是页面，而一个接受了连接却还答不出页的服务在人眼里仍是没开。轮询间隔 2 ms，单样本上限 300 s。

三座夹具城，同一个历史形状、三种长度：

| 城 | 记录 | 样本 |
|---|---|---|
| `empty` | `init` 出来的 3 条 | 20 |
| `l100k` | 2,000 个 run × 50 条 | 5 |
| `l400k` | 8,000 个 run × 50 条 | 3 |

一个 run 是 `run_started`、八个回合（`prompt_assembled`、`model_called`、`model_returned`、`tool_called`、`tool_result`、`checkpoint_committed`）与 `run_frozen`，正文长度与实测城市的记录相近。这是每回合一条 `prompt_assembled` 的账本形状：产品写的是每 run 一条 `prompt_assembled` 加每回合一条 `prompt_shape_compared`（runtime-SPEC §8-39 第 5 条），而每回合一条的账本仍被读入，故夹具是合法输入，其折叠代价与一座真正工作过的城同量级，但不逐条同形。账本经 `storage::JsonlLedger::append_all` 按每批 10,000 条写入：分段、链与字节规范都是产品自己的，本族不拼一行账。

**夹具城留在 `<构建档目录>/../bench-cities/<名>`**，下次复用：40 万条是 376 MB，每次重写要付的时间比量它还多。复用只看那座城在不在；`xtask mem --city` 读的就是同一座城（xtask-SPEC §8-30），于是首字节与启动峰值出自同一份历史。

**每座夹具城的读数旁打印它账本的摘要**（`citysim::ledger_digest`，§3-8），两条首字节读数只在摘要相等时可比。每个样本那次 `serve` 的标准错误写进 `<构建档目录>/../bench-cities/<名>.serve.log`（后一个样本覆盖前一个），报告里打印这个路径。其中以 `opened the city in` 开头的那一行是产品自己拆出的开城各段耗时（sprawling-SPEC 8-121）：本族不解析它，只把它和首字节读数放在同一次开城旁边给人读。

```rust
// tools/citysim/src/bin/bench_startup/actions/history.rs —— shape: adapter（一座有历史的夹具城：init 之后经产品的 Ledger 写入）
pub enum History { Empty, Runs(u32) }
pub fn fixture_city(cities: &Path, name: &str, history: History) -> Result<PathBuf, AxError>;
// tools/citysim/src/bin/bench_startup/actions/first_byte.rs —— shape: adapter
pub fn first_byte(binary: &Path, city: &Path, samples: usize, log: &Path) -> Result<Samples, AxError>;
```

**红**：`samples.rs` 的 nearest-rank 分位、可疑标注、第二档判定三个测试先行，跑一次见红再实现。`footprint` 三条（计数只数文件不数目录、按名找文件不论深度且不认目录、账行按行数而非按文件数）与 `actions` 一条（主导子步取中位最大者，无子步切分答 `None`）守的是**读数本身**：数错一个文件或指错一个主导件，报告就在说假话。

## 8.5 两个设计

**A（选中）：checker 复用 runtime::replay**——验证语义一处；citysim 只加「检查器」这个角色名。
**B（落选）：checker 自写链验证**——citysim 独立性更强（不依赖 runtime），但即刻成为第二验证权威，与 replay 漂移时两边都对不上夹具。落选理由：「重放与分叉共用重建器」的同一论证在此适用；citysim 依赖任何产品 crate 本就合法（第二 Main）。

### 8-6 负载场景骨架与读数行（bench）

五个负载场景都由 `just bench` 一键复测。其中四个是本 crate bench Main 的场景：大账本 fold（`large_ledger_fold`）、大 worktree 放置（`large_worktree_placement`）、再领一棵留着的 worktree（`kept_worktree_reclaim`）、长会话流式转发（`long_session_forwarding`）。第五个，多 run 并行，由 `sprawling` 的 `instrument_relay_round_trip` 量（sprawling-SPEC 8-84）：它驱动城里在跑的那个记账循环 `attend`，`just bench` 在本 crate 那一行之后跑它。bench Main 产读数，`tools/xtask/budgets.toml` 记基线行，不另造仪表。剧本执行器继续用计数时钟；本 crate 内凡计时都住在 bench Main 的模块树里，采样点仍是 `bench::stamp()` 那一个。

读数形（`bench::reading`，shape 2 value，一次构造点）：

```rust
pub enum MachineClass { General }   // 参照类属：盘、内存、CPU 均为一般水平
pub enum Load { LargeLedgerFold, LargeWorktreePlacement, KeptWorktreeReclaim, LongSessionForwarding }
pub enum SubMetric { Harness, Whole }
/// 一条读数在什么条件下量的：机器类属与所量字节的摘要。两者总是一起走，所以是一个值。
pub struct Taken { pub machine: MachineClass, pub fixture: B3Hash }
pub struct Reading { /* load, sub, taken, samples, p50, p95, p99 */ }
impl Reading {
    pub fn of(load: Load, sub: SubMetric, taken: Taken,
              samples: Vec<std::time::Duration>) -> Result<Reading, String>;
    pub fn line(&self) -> String;   // 唯一渲染家，键序固定
}
```

一行读数的文法（`Reading::line` 是唯一权威，测试按字节对拍）：

`perf load=<load> sub=<sub> machine_class=<general> fixture=<16 位十六进制> samples=<n> floor_us=<n> p50_us=<n> p95_us=<n> p99_us=<n>`

`fixture` 取登记夹具摘要（§3-8）的前 16 位十六进制，由 `citysim::fixture_label` 拼出；完整的 64 位写在 `REGISTERED.pinned`。

`floor_us` 是最小样本：机器安静时这条路径本身要花多少。它与 `p50_us` 并列，因为两者回答的不是一个问题——floor 贴着设计的下限，p50 带着机器的其余负载——而挂钟读数不设棘轮，两者就都得留在读数里，下一个读者才分得清一次回归是设计变慢了还是机器变忙了。

`machine_class` 是读数自带的字段而非行头批注：异类机器的读数不与参照类属同表比较。口径是 harness 自身路径的处理耗时（测量机的类属见 `machine_class`）——不含动画时长、不含网络传输。`SubMetric` 两值把定标拆开计：

| 子指标 | 量的是什么 | 对它定档的是什么 |
|---|---|---|
| `harness` | harness 纯开销，路径下无持久化提交 | 两档延迟目标（第一档 p95、第二档 p99，值住 `[local_latency]` 行，第二档严于并覆盖第一档） |
| `whole` | 路径本体就是盘上作业、缝口不拆的（worktree 放置） | 无 |

场景（`bench::scenarios`，shape 4 adapter，套在产品公共面上，无自有政策）：

```rust
pub struct Fixture { /* fold_records, fold_rounds, tree_files, tree_file_bytes,
                       placements, forward_events, pinned: &'static str（64 位十六进制） */ }
pub const REGISTERED: Fixture = Fixture { … };   // 既定负载：读数只在该 fixture 内可比，只降不升
pub const PINNED_DRAFTS: u64 = 1_000;
impl Fixture {
    /// `draft` 的前 PINNED_DRAFTS 行写进 `scratch` 下一本新账，取它的 `ledger_digest`，
    /// 再与各数值字段（不含 `pinned`）的小端字节拼接后取一次摘要。
    pub fn digest(&self, scratch: &Path) -> Result<B3Hash, String>;
}
pub fn all(scratch: &Path, fixture: &Fixture, taken: Taken) -> Result<Vec<Reading>, String>;
// tools/citysim/src/fixture_digest.rs —— shape: value；两个 bench 族共用
/// 一本账的字节摘要：按 `storage::ledger_segments_at` 的顺序，每段取 `B3Hash::digest(段字节)`，
/// 32 字节依次拼接后再取一次。一次持一段字节。
pub fn ledger_digest(ledger_dir: &Path) -> Result<B3Hash, AxError>;
/// 读数行与报告点名一份夹具用的 16 位十六进制：摘要的前 16 位。
pub fn fixture_label(digest: &B3Hash) -> String;
```

bench Main 在第一项读数之前算 `REGISTERED.digest`：与 `pinned` 不等即以 `bench failed:` 加一个三段式错误退出非零（`InvalidArgs`，主体是算出的 64 位摘要，recovery 说「在一个单独的提交里把 `REGISTERED.pinned` 重钉为这个值，并重取登记册里受影响的行」）；相等则先打印一行 `fixture <16 位>`，之后每条 `perf` 行的 `Taken.fixture` 都是它。

| 场景 | 驱动的公共面 | 子指标 |
|---|---|---|
| `large_ledger_fold` | `accounting::views::ask`，重建每个视图的生产全路径 | `harness` |
| `large_worktree_placement` | `storage::Checkpoint::ensure_base` 之后 `Worktrees::claim`／`release` | `whole` |
| `kept_worktree_reclaim` | 同一座城里同一个节点的第二次及以后的 `Worktrees::claim`，其间干线不动（storage-SPEC 8-9 的再领） | `whole` |
| `long_session_forwarding` | `wire::ServerFrame::Event` 装帧＋序列化，即 socket 之前的本地半段 | `harness` |

失败出口：域错误按其 `AxError`（动作/主体/稳定码/恢复语）格式化成一行；bench 自身的失败（零样本）构造 `AxError::failure(AxCode::InvalidArgs, …)`＋`with_recovery`，不新增码（§9-16 的口径）；Main 打 `bench failed: …` 且退出非零（既有形）。

#### 8-6.1 两个设计

**A（选中）：读数行一个文法、机器类属进字段，基线读数（含机器类属）记在 `tools/xtask/budgets.toml` 各场景的行里，棘轮纪律挂 `tools/xtask/budgets.toml` 的 `[local_latency]` 行——只降不升、放宽需单独提交。** 理由：register 的既有定规是「只有机器能两次同样测量的量才设门」（budgets.toml 头注），wall-clock 记录不设门；机器类属字段使异类机器的读数天然不进同一张表。

**B（落选）：像体积那样把延迟读数设门（超标即 CI 红）。** 落选理由：同一处定规写着「gating them would make a busy runner look like a defect」（budgets.toml 头注与 ARCHITECTURE §11 同句）；读数回归由棘轮纪律与单独提交的放宽手续治理，不由 CI 红绿治理。

**红**：`a_reading_line_is_stable_and_carries_its_machine_class`——一行读数按字节对拍既有文法且带 `machine_class` 与 `fixture` 字段；`a_reading_line_carries_its_floor_beside_the_middle`——同一文法下 floor 与 p50 并列；`every_load_scenario_reruns_and_emits_the_stable_format`——本 crate 的每个场景各跑两遍，每行键序恒为文法键序（可复跑、格式稳定）；`the_registered_fixture_writes_the_bytes_its_digest_pins`——`REGISTERED.digest` 等于 `REGISTERED.pinned`（§3-8）。

**多 run 并行不在本 crate 里量。** relay、`serve_flight` 与 desk 都是 `sprawling` 的 `pub(crate)`，本 crate 够不到，这里的场景只能抄一份 relay 的形状：数条 lane 经 mpsc 汇到一条线程，计时只包住那条线程上的一次 `append`。抄件量的是抄件：它的 `harness` 是 MemLedger 一次追加（5 µs 量级），而一次往返的代价取决于生产循环怎么等，抄件没有那份等法，也就量不到它；它的 `persist` 每条一道屏障，生产的 `append_all` 一批一道；盘的份额由那件仪表 `store=disk` 与 `store=memory` 两行之差读出，所以 `SubMetric` 没有 `persist`。**败给的方案**：给 `sprawling` 开一扇公共门让本 crate 驱动 `serve_flight`。那扇门没有生产调用者，而仪表放在 crate 内已经能驱动生产循环本身（sprawling-SPEC 8-84 的决定）。

### 8-7 红队：有无验证 run 两臂的结论质量（`citysim::red_team`）

形状：decision。红队剧本是一组固定的 `Case`：每个 case 是一份草稿，外加作者 run 交出的结论，每条结论带一条 `collab::Citation` 与红队写下的真相 `Plant`（`Faithful`，或三种埋下的缺陷 `Misquote`／`OtherVersion`／`PastEnd`，与 `collab::Reading` 的三种不成立读数一一对应）。剧本就是脚本化 provider 在这里的角色：同一剧本两臂各跑一次，逐字节可复跑。

```rust
pub enum Arm { Unverified, Verified }
pub enum Plant { Faithful, Misquote, OtherVersion, PastEnd }
pub struct Claim { pub citation: Citation, pub plant: Plant }
pub struct Case { pub draft: String, pub claims: Vec<Claim> }
pub struct Tally { pub kept_faithful: usize, pub kept_planted: usize, pub dropped_faithful: usize, pub dropped_planted: usize }
impl Tally { pub fn precision_per_mille(&self) -> Option<usize> }
pub struct Comparison { pub unverified: Tally, pub verified: Tally }
pub fn compare(cases: &[Case]) -> Comparison
```

- `Arm::Unverified` 留下作者交出的每条结论；`Arm::Verified` 把每条引文对草稿钉住的版本（`cas:` 草稿全文摘要、无区间）跑一次 `Citation::against`，只留 `Reading::Holds` 的结论。
- 结论质量＝留下的结论里忠实者的千分比（`precision_per_mille`）；一条都没留下时为 `None`，因为零分之零不是质量。另两格（误删的忠实结论、放行的缺陷）照实计数，使验证 run 的代价与收益在同一张表上。
- 无失败出口：一条不成立的引文是验证 run 要报的结果，不是故障（与 `collab::Reading` 同一口径）。

决定：两臂共用同一份剧本与同一个判定函数 `collab::Citation::against`，只差「判定是否被调用」。落选的做法是在 `suite`（§8-8-1）里建套件：`Suite` 量的是 held-in／held-out 的通过率，没有「同一结论集、去掉一个环节」这一维，而且 citysim 已经有固定剧本与计数时钟。真实 provider 的读数替换的是剧本里的作者，不是判定；判定可复跑，所以 CI 只跑脚本化这一侧。

**红**：`the_verified_arm_keeps_only_faithful_conclusions`——同一剧本下，验证臂的千分比为 1000、放行缺陷为 0，未验证臂低于它；`every_planted_defect_is_dropped_by_its_own_reading`——三种埋下的缺陷各自被验证臂删掉，忠实结论一条不误删。

### 8-8 仪器：suite、score、metabolism、nesting、ablation

五件仪器回答「城拿什么证据评估自己」。它们**恒不是合并门**：一件仪器说某样东西变差了，是给人看的证据，不是 CI 的红灯。量的是模型行为，两次不一样是常态，所以它们出证据不出红灯；设阈值的门归 `xtask budget`，依据是「机器两次量得一样」。

| 模块 | 它回答的问题 | 构型 |
|---|---|---|
| `suite`（含 held-out 判定） | 一批真实任务怎么组织、怎么跑两次而结果可比 | 库面，`tests/evaluation.rs` 驱动 |
| `score`、`metabolism` | 哪些沉淀资产在升值、哪些该退场 | 仅测试构型 |
| `nesting` | 模型编辑哪种嵌套格式错得最少，错时怎么错 | 仅测试构型 |
| `ablation` | 拿掉 City.md 的某一段，居民做不了什么 | 仅测试构型 |

三条前提只消费不重议：**评分对象是资产不是 Agent**（会话冻结即终结，Ephemeral 恒不进评分与 metabolism）；**语料只取自真实工作**（一份合成任务集测出来的分数，测的是出题人）；**登记归 `kernel::registry`**（Asset 是什么、登记在哪由它答；这里只答「这份登记值多少」，成本读数归 `storage::attribution`）。统计全用整数，比率以千分数（`per_mille`）表达，不引入统计库。

**仪器只在测试构型里编译。** `score`、`metabolism`、`nesting`、`ablation` 在 `lib.rs` 写作 `#[cfg(test)] mod`：它们回答的是「这套规则算得对不对」，答法是自己的测试，提问者是读测试的人；没有剧本调用它们。dead_code 因此不是被 `#[allow]` 压掉的，是不存在的。**重开条件**：出现一个生产调用点要对资产排序或退场，例如城层的资产清单视图；届时那个模块搬到拥有该视图的 crate。

#### 8-8-1 suite（形状 2 值类型＋形状 1 判定）

```rust
pub enum Half { HeldIn, HeldOut }
pub struct Task { pub id: String, pub at: Locator, pub half: Half }
pub struct Outcome { pub id: String, pub passed: bool }
pub struct Tally { pub tried: u32, pub passed: u32 }   // per_mille() 整数千分比
pub struct Report { pub held_in: Tally, pub held_out: Tally, pub unknown: u32 }
pub struct Suite { /* BTreeMap<String, Task> —— 私有 */ }
impl Suite {
    pub fn new(tasks: Vec<Task>) -> Result<Suite, AxError>;   // 同一 id 两次即拒（泄漏在构造点）
    pub fn half(&self, half: Half) -> Vec<&Task>;             // id 序＝执行序
    pub fn report(&self, outcomes: &[Outcome]) -> Report;
}
```

- **泄漏是构造点的拒绝，不是事后的告警**：同一个 id 出现两次即拒，无论落在同半还是异半。一份被看过的 held-out 集在它被看过之后就不值钱了。
- **任务只携 Locator 不携正文**：抄一份正文进来就会与它来自的那件活漂开。
- **不认识的 outcome 计入 `unknown` 而非计入分母**：一次回答了没人问过的问题的运行，不是这份 suite 的运行。

#### 8-8-2 score 与 metabolism（形状 1 判定；仅测试构型）

```rust
pub struct AssetUse { pub uses: u32, pub resident: ByteLen, pub idle_days: u32 }
pub struct Score { pub per_mille: u32, pub idle_days: u32 }
pub fn score(usage: &AssetUse) -> Score;
pub fn worst_first<T: Clone>(assets: &[(T, Score)]) -> Vec<(T, Score)>;

pub const ASSET_IDLE_DAYS: u32 = 90;
pub const ASSET_FLOOR_PER_MILLE: u32 = 1_000;
pub enum Disposal { Keep, Warn { because: String }, Retire { because: String } }
pub fn dispose(usage: &AssetUse, score: Score, warned_already: bool) -> Disposal;
pub fn sweep<T: Clone>(assets: &[(T, AssetUse, Score, bool)]) -> Vec<(T, Disposal)>;
```

- **分子是被取用次数，分母是常驻字节**：同样的有用程度，占的地方越大越贵——那是它在每一次披露它的 prompt 里都要付的账。
- **`idle_days` 并列而不折进分数**：便宜且无用与昂贵且不可或缺是两回事。
- **最重的处置是 `Retire`，不是删除**：退场＝不再被披露，字节仍在盘上与历史里。
- **先警告后退场，理由随处置同行**：没有任何东西在第一次被注意到的同一轮里停止被提供——那一轮正是人说「它重要」的机会。

#### 8-8-3 nesting（形状 1 判定；仅测试构型）

计划树要住在一个模型每天编辑的文件里，TOML／JSON／Markdown 三选一由这件仪器的数字决定，不由口味决定。`Fault` 是穷尽枚举，**按破坏力排序**：`LostField`（能解析、少了一个字段——唯一一种文件仍可读而一个计划节点悄悄不存在的结局）＞ `ChangedBystander` ＞ `Unparseable` ＞ `Truncated` ＞ `NotApplied`。`grade` 报最坏的那一个；`recommended` 先比错误率，平手比各自最坏的错法。

它不调用模型：`Attempt` 是某个模型已经产出的东西，一个自持 provider 的 suite 无法离线跑、无法重放。语料自己解析不了时拒绝而不是记分。三种格式读成同一组叶子（`path -> value`，`nesting/reading.rs`）；Markdown 那条刻意严格，因为会修复松散缩进的读法会藏掉这件仪器正在计数的失败。

#### 8-8-4 ablation（`ablation.rs` 形状 1 判定，`ablation/capabilities.rs` 形状 6 数据；仅测试构型）

`docs/City.md` 是每个居民读到的第一份文本，它每多一段就向每一次 prefix 收一次租。这把尺把文档按段切开，逐段拿掉，量一个居民因此做不了什么。入口是一条 `#[ignore]` 测试，只在有人点名时跑：

```
cargo nextest run -p citysim --run-ignored all -E 'test(city_md)' --no-capture
```

可测量的替身是**能力与凭据**：一条 `Capability` 是居民必须能做的一件事，它的 `cue` 是文中授予这件事的那句逐字短语。

```rust
pub(crate) struct Capability { pub(crate) name: &'static str, pub(crate) cue: &'static str }
pub(crate) struct Passage { pub(crate) index: u32, pub(crate) opening: String, pub(crate) removed: ByteLen }
pub(crate) enum Cost { Untouched, Restated { also_said: Vec<&'static str> }, Sole { lost: Vec<&'static str> } }
pub(crate) struct Charge { pub(crate) passage: Passage, pub(crate) cost: Cost }
pub(crate) struct Ablation { /* 私有：passages、corpus */ }
impl Ablation {
    pub(crate) fn new(document: &str, corpus: &'static [Capability]) -> Result<Ablation, AxError>;
    pub(crate) fn charges(&self) -> Vec<Charge>;
    pub(crate) fn costliest_first(&self) -> Vec<Charge>;
}
```

- **三值而非布尔**：`Restated` 让人看得见文档在哪里重复自己。
- **语料自己不能给自己打分**：`new` 在整份文档里找不到某条 cue 即拒（`E_INVALID_ARGS`，recovery 指向语料）。
- **切段规则**：空行切块，以 `- ` 开头的块并入上一段。
- **`costliest_first` 先比失去的能力数（降），平手比被删字节数（升），末位比 `index`**：排序两次必须一样。

## 9–16 工作流程／实现／边界／错误／依赖／硬编码／影响面／测试

八节并成一节，因为本 crate 的实现是适配器、剧本与仪器，每一节只有一两行可说。

- 流程：测试构造 drafts→MemLedger append→checker／conformance／对拍 JsonlLedger；另有 Scenario→run_scenario→check_chain 加事件序断言。
- 实现：MemLedger 的 append 是 from_draft→canonical_line→chain_hash 推进；无别的逻辑。
- 边界：空 Ledger check 通过；单创世行通过。
- 错误：透传 kernel/replay 与产品公面的 AxError，不新增码。
- 依赖：kernel（features=["conformance"]）、storage（对拍与夹具）、runtime（驱动器与 verify）、gateway（dialect 翻译面）、collab（引文判定）、wire（帧）、sprawling（测量二进制驱动的产品公面）、serde_json、toml（nesting 读它评分的 TOML 形状）、sha2 与 zip（安装动作读发行档）；dev：tempfile、zeroize。
- 硬编码：策略常数只在仪器里（§8-8-2 的两个阈值）与测量二进制里（`SAMPLES`、夹具规模），各自注释点名它的理由。
- 影响面：`just sim` 与 `just bench` 的全部场景建于本 crate 之上。
- 测试：conformance 双实现、字节对拍、夹具对拍、篡改检出、事件序断言。

## 17 模型体验

零字节：dev-only 设施，恒不进任何 prefix。

## 18 文档同步

增一个场景、测量或仪器时，本 SPEC 同集增节；夹具更新须与 memory/kernel 的字节规范同一变更集。
