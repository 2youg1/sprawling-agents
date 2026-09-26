# citysim-SPEC.md

> 工作区成员：`citysim`（dev-only，第二个 Main，不占产品拓扑）。本 SPEC 先于代码存在。
> 两件基底（内存 Ledger＋链检查器）、三件剧本设施（剧本模型／脚本工具／薄执行器，§8-2）与真适配器换入（§8-3）。

## 1 需求分解

| 件 | 一句话 |
|---|---|
| `mem_ledger` | kernel Ledger 的第二适配器：全内存、确定性、conformance 的对照实现 |
| `checker` | 不变量检查器首条：链完整且 seq 连续 |
| `script_model` | kernel Model 的第二适配器：脚本驱动的 ModelReturn 序列，确定性 |
| `script_tools` | kernel Tool 的第二适配器：脚本工具＋按名分发闭包，全失败模式可注入 |
| `executor` | 薄执行器：Dispatch→四相回合×N→run_frozen；取消注入点＝A9 先行 |

## 2 验收标准

- `MemLedger` 过 kernel conformance 六断言（与 JsonlLedger 同一套——这就是「缝」的兑现）。
- 同一 draft 序列灌 MemLedger 与 JsonlLedger，raw 行逐字节相同（规范字节住 kernel 的实证）。
- checker 对合法序列静默通过；对篡改序列报出首个断点行。
- 跨 OS 字节夹具：`fixtures/golden-s1/` 的脚本化序列重建后与夹具逐字节相同（CI 三平台恒跑同一断言）。

## 3 假设与歧义

citysim 不受 ARCHITECTURE §6 模块表约束（表只辖 crates/**），但 MPL 头、lexicon、lints 全库同规。`just sim` 的入口是本 crate 测试（固定剧本＝测试用例）。**本 crate 没有随机源，也没有种子**：确定性由三件事持有——剧本是写死的、时钟是 `executor` 里的 tick 计数器、执行是单线程；复现一次失败靠的是重跑那个剧本。种子驱动的随机剧本批随故障面落地，届时它会有一个真吃种子的生成器。Dispatch 的「先落 JOB.md 再产事件」在 sim 里以 `checkpoint_committed`（确定性假 oid＝B3Hash 派生前 20 字节 hex）代文件面——模拟适配器的职责即伪造外部世界，事件序与真城同形。

### 3-1 决定：按现实修文档，而不是造一个吃种子的生成器

五处文档曾写「一次失败可以从它的 seed 复现」，而实现里没有任何随机源被种子驱动：`AGENTS.md` 的命令表行与 Tests 一条、`docs/CONTRIBUTING.md` 的命令表行、`docs/glossary.md` 的 **driving pool** 行、`citysim/src/lib.rs` 的 crate 文档（「seeded RNG」）。六处修的是这些表述；`citysim/src/executor.rs` 的时钟注释同改。

选修文档而不是造生成器，理由是种子此刻没有消费者：剧本是测试用例，随机剧本批要等故障面（§8-3 之后）才有东西可随机，现在造一个生成器等于先立一个没有被任何断言驱动的第二权威。**重开参数**：随机剧本批落地时，种子成为 `Scenario` 的一个字段，由它派生每一处分叉，并同批改回这几处表述与 `justfile` 的 `sim` recipe。

**仍未落盘（跨文件，见交付报告）**：`justfile` 的 `sim seed=""` 接一个它不使用的参数；`ARCHITECTURE.md` 三处、`citysim/tests/sieve.rs` 与 `citysim/tests/scenario.rs` 各一处仍写着种子。

### 3-2 决定：计时边界取「动作的可观察端点」，进程动作以退出为端点

四动作里两个跨进程（安装的落位确认、启动）：端点是被拉起进程**退出被观察到**，因为「可接受命令」在产品外部可观察的最短证据就是一条轻命令被应答完毕。落盘动作（建城、开 session）以公面调用**返回**为端点，因为返回即账本已带自身屏障落盘（落账先于效果）。被击败的备选：以进程内部时点（参数解析完成、监听就绪）为端点——那要在产品里插桩，改写被测路径（fx 报告「不抄」第五条：测量专用分支守着一条不发货的路径）。

### 3-3 决定：安装边界含归档摘要校验、不含 PATH 写入

摘要校验（sha256）是 `install.sh`／`install.ps1` 从归档就位到解包之间必经的一步，删掉它测的就是不验签的安装——红线「验签不删」在测量口径里同样成立，故计为安装的子步并单列读数。验签哈希（T1 签名验签）另记 **0**：T1 尚 stub，签名验签今日不存在，记 0 并注明，T1 落地后此子步只增不删。PATH 写入（`sprawling install` 的注册表写与桌面广播）在边界外：它是一次性的桌面状态写入，第二次运行幂等（`PathEdit::AlreadyPresent`），计进每样本会把桌面状态写入误报成安装成本。被击败的备选：整段 `install.sh` 全测——含网络下载与 shell 启动，网络不计是 T14 既定口径。

### 3-4 决定：计量主语是 Rust measuring Main，不是 adversary/ 也不是 criterion

四动作零行为断言，只计时；`adversary/` 量化行为轨迹，Lean 侧不为墙钟定价。criterion 是第二套仪表（T13「不另造第二套仪表」）：本族挂 `just bench` 族，同一 wall-clock 口径（测而不门）。它拉起产品二进制——被测动作本身即进程边界（口径点名「进程拉起到可接受命令」）；boundary 门判的是**检查**站哪一侧，其越过面 token（`CARGO_BIN_EXE`／`SPRAWLING_BIN` 等）本族一个不写，被测二进制取自构建档目录（`cargo build` 同时放置两个产物的地方），`just bench-startup` 先构建后测量，故不接手工路径也不会测到旧产物。被击败的备选：把四动作写进 `adversary/`——那里没有秒表也没有本仓词汇，量出来的东西无法与 `just bench` 对表。

### 3-5 决定：被测可执行文件的名字在本 crate 只重述一处，注释点名它的权威

`executable_name()` 拼的是 `install.rs` 装出来的那个名字：`INSTALLED_STEM` 加本平台后缀。该事实的权威是 `xtask/src/platform.rs` 每平台的 `binary` 字段，`cargo xtask artifact` 把发行侧的四种拼法（工作流矩阵、两个安装脚本、npm shim）钉在它上面；citysim 这一处不在那四种之内，它是唯一需要这个名字的**测量**读者。够不到权威的原因是位置而非取舍：`install` 模块住在 `crates/sprawling/src/main.rs`，二进制的模块不可 import，而 `xtask` 是工具不是依赖。本 crate 内只留这一处拼写——`shipped_binary` 找的路径名与 `archive_of` 写出的 zip 成员名都读它。**重开参数**：这个名字若移进 `sprawling` lib 成为公共面，本函数改为读它，重述随之删除。

## 4 现状分析

空壳 lib。无性能议题。

## 5 权威信源

citysim 定位、四件模拟适配器、检查器十五条；kernel-SPEC §8-9；runtime-SPEC §8-1。

## 6 命名统一

MemLedger、checker、invariant（编号取检查器清单）。

## 7 模块边界

```
mem_ledger ──▶ kernel（Ledger trait＋event＋conformance feature）
checker    ──▶ runtime::replay（verify_lines 复用，不建第二验证权威）
```

**不做什么**：不落盘；不采时钟（t 由剧本注入）；不实现除 Ledger 外的三件模拟适配器（剧本模型与脚本工具见 §8-2，FaultFs 已住 memory）。

## 8 接口先行

```rust
pub struct MemLedger { /* lines: Vec<Vec<u8>>, next_seq, prev */ }
impl MemLedger { pub fn new() -> Self;  pub fn raw_lines(&self) -> &[Vec<u8>]; }
impl kernel::Ledger for MemLedger { … }
impl kernel::conformance::LedgerInspect for MemLedger { … }   // citysim 恒开 conformance feature

/// Invariant 1: chain intact, seq contiguous.
pub fn check_chain(lines: Vec<Vec<u8>>) -> Result<(), AxError>;   // replay::verify_lines 薄封
```

### 8-2 剧本适配器与薄执行器

```rust
pub struct ScriptModel { /* VecDeque<ModelReturn> */ }
impl ScriptModel { pub fn new(script: Vec<ModelReturn>) -> Self; }
impl kernel::Model for ScriptModel { /* 逐次弹出；耗尽后恒回空 calls（自然收束） */ }

pub struct ScriptTool { /* meta、outcomes: VecDeque<Result<ToolOutcome, AxError>> */ }   // impl kernel::Tool
pub struct ScriptToolSet { /* BTreeMap<String, ScriptTool> */ }
impl ScriptToolSet { pub fn new(tools: Vec<ScriptTool>) -> Self;  pub fn empty() -> Self;
                     pub fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError>; }
                     // 执行器以 `&mut |c| tools.invoke(c)` 适配 turn 的闭包形参；不另造 invoker 工厂
                     // 未知工具名 → E_TOOL_UNKNOWN＋nearby＝已注册名；耕尽脚本 → E_TOOL_UNAVAILABLE

pub enum CancelPoint { BeforeAssemble { turn: u32 }, BeforeCall { turn: u32 }, BeforeWave { turn: u32 } }
pub struct Scenario { pub run: RunId, pub who: String, pub addr: Address, pub task: String,
                      pub goal: String, pub job_md: String, pub model: ScriptModel,
                      pub tools: ScriptToolSet, pub cancel: Option<CancelPoint>, pub budget_turns: u32 }
                      // run/who 由剧本注入（citysim 禁随机）；job_md 是 JOB.md 内容，假 oid 由其哈希派生
pub struct ScenarioReport { pub lines: Vec<Vec<u8>>, pub completion: &'static str /* done|cancelled|limit */ }
/// The thin executor (second Main): Dispatch → N four-phase turns → freeze.
/// Deterministic: t 单调递增每步 +1ms，无时钟采样；无随机。
pub fn run_scenario(scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

- 事件序（无取消正常收束）：`checkpoint_committed`（JOB.md 先落）→ `run_started` → 每回合 `prompt_assembled→model_called→model_returned[→tool_called→tool_result]*` → 空 calls 回合后 `handoff_written` → `run_frozen{completion:done, evidence:[末 model_returned]}`。
- 取消在指定边界注入 `Interrupt::Cancel`：事件序断言＝cancel_received 后无新 model_called/tool_called，恒有 handoff_written 先于 run_frozen（A9 先行，逐边界三剧本）。
- `budget_turns` 是执行器的回合上限（到限即 `run_frozen{completion:limit}`）：真预算梯随 gate 挂剧本接入。

### 8-3 真适配器换入（单 Resident 全链走完一条回路）

骨架的四个替换点逐一换真，剧本仍确定性（无时钟采样、无网络、无随机）：

| 替换点 | 原骨架 | 换入 |
|---|---|---|
| Ledger | MemLedger | 仍 MemLedger（真 jsonl 对拍已在 conformance；换盘不增新证据） |
| 工具面 | ScriptToolSet 闭包 | 真 ToolBench（edit＋status＋exec Program 臂）对 tempdir 城根；门路由在回合层 |
| 模型 | ScriptModel 直造 ModelReturn | ScriptModel 登录 wire 形：剧本写 Anthropic wire JSON，经 gateway::dialect::response_from_wire 解成 canonical 再出 ModelReturn（翻译面进链路） |
| 时钟／配置 | 逐步 +1ms | 同＋FrozenConfig 求值（clock_stamp 三层覆盖）接入 StampGate |

`Scenario` 的 `write_domain` 字段撤销——门路由归 ToolBench，域住 bench 内，executor 不再手写 domain 门。`ScriptToolSet` 未删：它仍是 `kernel::tool` 缝的第二适配器（已登记的 conformance 证据），改为**注册进真 ToolBench**，于是脚本工具与真 L0 工具走同一条门路由。波前围栏是**每波一次**而非只在 exec forecast 命中时：A14 的先行半链管的是波，`ToolBench` 内的 forecast 围栏是它在 exec 臂上的加强，两者不互相替代。空波仍提交（同树 oid），因为链可重建优于省一次提交。tool_result 的信封由 executor 挂（`pipeline::package`＋`StampGate`），与 serve 同位。

事件序新增断言：edit 成功波携 checkpoint_committed（波前，断言形＝每个 tool_called 之前最近的 checkpoint_committed 晚于最近的 model_returned）；tool_result 信封可携 ClockStamp（非 Off 时）；越域写被 domain 门拒且 refusal 以 tool_result 回流；链恒可验；双跑字节对拍。“真 gateway 适配器换入”的取义：dialect 翻译面入链（纯函数，确定性保持）；endpoint 的 HTTP 面不入 sim（网络即非确定），其验证住 gateway 自身的回环假服务测试（gateway-SPEC §2）。

### 8-4 一波里的两次调用不再被当成同一次

```rust
// citysim::executor——驱动块内
let placed = Cell::new(0u64);              // 位次：每跑一个计数器，与 bin::assembly 同形
let key = IdemKey::derive(&run, Seq::new(at), &call.action()?);   // 动作字节：kernel::tool 唯一一份
```

**缺陷所在**：`IdemKey::derive(&run, Seq::new(t.value()), call.name.as_str().as_bytes())` ——位次处塞的是**钟读数**，动作字节里**只有工具名**。两处各自都不致命，合起来致命：`runtime::run` 每回合采一次 `t`，再把同一个 `t` 发给一波里的每次调用（那是驱动器故意的：一波是一个瞬间）。于是一波之内两次同名调用拿到**完全相同的 `IdemKey`**，`ToolBench::invoke` 的 dedup 当场判 `Duplicate`，第二次以 `E_INVALID_ARGS`／「this call was already made」回流给模型，且 `recovery` 为空串——一条无路可走的拒绝。参数不同不救（参数不进键），call id 不同也不救。跨回合不撞：本 crate 的 `tick` 每次 `now()` 加一。

**它是潜伏的，不是在燃的**：在此之前全部场景皆绿，因为没有一条现有剧本在一波里发两次同名调用。它咬的是下一个写这种剧本的人，而那个人会去查自己的剧本——因为错误说的是「这次调用已经发生过」。

**为何是本 crate 的错而不是驱动器的**：一波共用一个 `t` 是 `runtime::run` 已记录的设计（一波是一个瞬间，而时间参数化是确定性第 2 条）。把一个“同一波内恒相等”的量当作位次，是本 crate 单方面的读法，并且逐字违反确定性第 7 条（「never from a clock」）——那个 `Seq` 是钟读数换了个类型。

**现形**：动作字节改调 `ToolCall::action`（kernel-SPEC §8-23，全库唯一一份）；位次改用每跑一个的计数器，与 `bin::assembly` 同形。两个驱动器于是对「一次工具调用的键怎么算」只有一份读法。

**红**：`two_reads_in_one_wave_are_two_calls`——一个回合携两次同名、参数不同的调用，断言两条 `tool_result` 都带结果、都不带 `error`。

**不变的东西**：`IdemKey` 不进任何 payload，故账本字节不变，`golden-p0`（由 `run_scenario` 现跑重生）不需重生——这是带原型跑完全套验过的，不是读一个 payload 推的。

### 8-5 bench_startup 族：四动作压档的测量面（只实测，不优化）

四动作各给三件套（能否进 p99≤1ms／极限读数／主导成本件），优化另波。本族是 `just bench` 的同族仪表：同一 citysim bin 面、同一 wall-clock 口径（测而不门，读数标注机器类属）。四行读数落在 `xtask/budgets.toml` 的 `[install]`／`[startup]`／`[raise_city]`／`[open_session]`，无预算键故不门；机器类属、样本数与四个动作的子指标数字都写在行内，本 SPEC 不重抄它们。

```rust
// citysim/src/bin/bench_startup.rs —— measuring Main：本族唯一计时采样点（`stamp()`，同 bin::bench 先例）；`SAMPLES` 是每动作次数的唯一之家，循环、预分配与报告同读它
// citysim/src/bin/bench_startup/samples.rs —— shape: decision（时间以 Duration 入参；无时钟、无 I/O）
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
// citysim/src/bin/bench_startup/actions.rs —— shape: adapter（薄驱动产品公面，口径即边界；两处跨进程动作走的都是产品自己的路径）
pub const SAMPLES: usize;
pub struct PerSample { pub processes: u64, pub files: u64, pub barriers: u64 }
pub struct Action { pub total: Samples, pub steps: Vec<(&'static str, Samples)>, pub per_sample: PerSample }
pub fn install(scratch: &Path, archive_path: &Path) -> Result<Action, AxError>;
pub fn startup(binary: &Path) -> Result<Action, AxError>;
pub fn raise_city(scratch: &Path) -> Result<Action, AxError>;
pub fn open_session(city: &Path) -> Result<Action, AxError>;
pub fn dominant(steps: &[(&'static str, Samples)]) -> Option<&'static str>;
// citysim/src/bin/bench_startup/actions/archive.rs —— shape: adapter（发行档的格式读写都在这里：摘要、解包、写出 fixture 条目；`sha2`／`zip` 各在 workspace 清单里一个名字，打包步与本次读数不会对它们生出第二种拼法）
// citysim/src/bin/bench_startup/actions/footprint.rs —— shape: adapter（一次 walk 供所有读者：一个样本在一棵树下写了多少文件、多少账行、某个名字在哪、以及样本之间如何把目录清平）
```

**四个计时边界**（起止、子步、决定见 §3-2／§3-3）。样本 200／动作（≥100）；p50/p95/p99 取 nearest-rank；读数全样本给出（基线永不减除），可疑样本只标注不剔除。

| 动作 | 起点 | 终点 | 子步 |
|---|---|---|---|
| ① install | 归档已就位（zip 在 scratch，网络不计） | 解出的可执行文件拉起 `version` 应答并退出 | 归档摘要校验（sha256）｜解包写可执行文件｜落位确认（进程创建＋运行到退出） |
| ② startup | `CreateProcess` 发出 | 轻命令 `version` 退出被观察到 | 进程创建（spawn 返回）｜程序运行（返回→退出） |
| ③ raise_city | `assembly::init_city` 调用发出 | 调用返回（创世记录落盘，每条账各带自己的屏障） | 无子步切分（不插桩产品）；主导件由计数×地板归因 |
| ④ open_session | `RunWorker::handle(Command::OpenSession)` 发出 | 调用返回（`session_opened` 已落账，房内下一 run 可开工即可接输入） | 同上 |

**子指标拆分（各自计数，先行）**：进程创建（①1／样、②1／样、③0、④0，时间取子步）；文件创建（`PerSample.files`：①＝解包写出的文件数，③＝创世城市树的文件数，④＝首样本前后 city 文件数之差；②自身不落盘，记 0 而不是把它被指向的那棵树算进来）与耐久屏障（`PerSample.barriers`，一账一屏障，地板引用 `just bench` 的 `durability_barrier` 行，不另起第二仪表）；验签哈希＝0（T1 尚 stub，记 0 并注明，见 §3-3）；Defender 实时扫描干扰＝可疑样本数与下标（`SampleKind`），①③ 每样本全新首触（必扫），② 复用同一映像（首样本后转热）。

**失败**：产品公面的失败原样抛 `AxError`，不新增码；测量自体的失败（被测二进制不在构建档目录等）用既有码走三段式（动作/主体/`AxCode`/recovery）。

#### 8-5-1 首字节：`sprawling serve` 拉起到第一个字节（`ttfb`）

四个动作之外的第五行读数，量的是人开一座城要等多久：`CreateProcess` 发出（`serve <城> 127.0.0.1:<端口> --no-console --no-open`，`SPRAWLING_OPEN=never`）到对 `GET /` 读到第一个字节。端点取第一个字节而不是端口开始监听，因为人看见的是页面，而一个接受了连接却还答不出页的服务在人眼里仍是没开。轮询间隔 2 ms，单样本上限 300 s。

三座夹具城，同一个历史形状、三种长度：

| 城 | 记录 | 样本 |
|---|---|---|
| `empty` | `init` 出来的 3 条 | 20 |
| `l100k` | 2,000 个 run × 50 条 | 5 |
| `l400k` | 8,000 个 run × 50 条 | 3 |

一个 run 是 `run_started`、八个回合（`prompt_assembled`、`model_called`、`model_returned`、`tool_called`、`tool_result`、`checkpoint_committed`）与 `run_frozen`，正文长度与实测城市的记录相近，所以折叠这份账本的代价与一座真正工作过的城同形。账本经 `memory::JsonlLedger::append_all` 按每批 10,000 条写入：分段、链与字节规范都是产品自己的，本族不拼一行账。

**夹具城留在 `<构建档目录>/../bench-cities/<名>`**，下次复用：40 万条是 376 MB，每次重写要付的时间比量它还多。复用只看那座城在不在；`xtask mem --city` 读的就是同一座城（xtask-SPEC §8-30），于是首字节与启动峰值出自同一份历史。

```rust
// citysim/src/bin/bench_startup/actions/history.rs —— shape: adapter（一座有历史的夹具城：init 之后经产品的 Ledger 写入）
pub enum History { Empty, Runs(u32) }
pub fn fixture_city(cities: &Path, name: &str, history: History) -> Result<PathBuf, AxError>;
// citysim/src/bin/bench_startup/actions/first_byte.rs —— shape: adapter
pub fn first_byte(binary: &Path, city: &Path, samples: usize) -> Result<Samples, AxError>;
```

**红**：`samples.rs` 的 nearest-rank 分位、可疑标注、第二档判定三个测试先行，跑一次见红再实现。`footprint` 三条（计数只数文件不数目录、按名找文件不论深度且不认目录、账行按行数而非按文件数）与 `actions` 一条（主导子步取中位最大者，无子步切分答 `None`）守的是**读数本身**：数错一个文件或指错一个主导件，报告就在说假话。

## 8.5 两个设计

**A（选中）：checker 复用 runtime::replay**——验证语义一处；citysim 只加「检查器」这个角色名。
**B（落选）：checker 自写链验证**——citysim 独立性更强（不依赖 runtime），但即刻成为第二验证权威，与 replay 漂移时两边都对不上夹具。落选理由：「重放与分叉共用重建器」的同一论证在此适用；citysim 依赖任何产品 crate 本就合法（第二 Main）。

### 8-6 负载场景骨架与读数行（bench，T13 第一段）

四个负载场景都由 `just bench` 一键复测。其中三个是本 crate bench Main 的场景：大账本 fold（`large_ledger_fold`）、大 worktree 放置（`large_worktree_placement`）、长会话流式转发（`long_session_forwarding`）。第四个，多 run 并行，由 `sprawling` 的 `instrument_relay_round_trip` 量（sprawling-SPEC 8-84）：它驱动城里在跑的那个记账循环 `attend`，`just bench` 在本 crate 那一行之后跑它。bench Main 产读数，`xtask/budgets.toml` 记基线行，不另造仪表。剧本执行器继续用计数时钟；本 crate 内凡计时都住在 bench Main 的模块树里，采样点仍是 `bench::stamp()` 那一个。

读数形（`bench::reading`，shape 2 value，一次构造点）：

```rust
pub enum MachineClass { General }   // 参照类属：盘、内存、CPU 均为一般水平
pub enum Load { LargeLedgerFold, LargeWorktreePlacement, LongSessionForwarding }
pub enum SubMetric { Harness, Whole }
pub struct Reading { /* load, sub, machine, samples, p50, p95, p99 */ }
impl Reading {
    pub fn of(load: Load, sub: SubMetric, machine: MachineClass,
              samples: Vec<std::time::Duration>) -> Result<Reading, String>;
    pub fn line(&self) -> String;   // 唯一渲染家，键序固定
}
```

一行读数的文法（`Reading::line` 是唯一权威，测试按字节对拍）：

`perf load=<load> sub=<sub> machine_class=<general> samples=<n> floor_us=<n> p50_us=<n> p95_us=<n> p99_us=<n>`

`floor_us` 是最小样本：机器安静时这条路径本身要花多少。它与 `p50_us` 并列，因为两者回答的不是一个问题——floor 贴着设计的下限，p50 带着机器的其余负载——而挂钟读数不设棘轮，两者就都得留在读数里，下一个读者才分得清一次回归是设计变慢了还是机器变忙了。

`machine_class` 是读数自带的字段而非行头批注：异类机器的读数不与参照类属同表比较。口径是 harness 自身路径的处理耗时（测量机的类属见 `machine_class`）——不含动画时长、不含网络传输。`SubMetric` 两值把定标拆开计：

| 子指标 | 量的是什么 | 对它定档的是什么 |
|---|---|---|
| `harness` | harness 纯开销，路径下无持久化提交 | 两档延迟目标（第一档 p95、第二档 p99，值住 `[local_latency]` 行，第二档严于并覆盖第一档） |
| `whole` | 路径本体就是盘上作业、缝口不拆的（worktree 放置） | 无 |

场景（`bench::scenarios`，shape 4 adapter，套在产品公共面上，无自有政策）：

```rust
pub struct Fixture { /* fold_records, fold_rounds,
                       tree_files, tree_file_bytes, placements, forward_events */ }
pub const REGISTERED: Fixture = Fixture { … };   // 既定负载：读数只在该 fixture 内可比，只降不升
pub fn all(scratch: &Path, fixture: &Fixture, machine: MachineClass)
    -> Result<Vec<Reading>, String>;
```

| 场景 | 驱动的公共面 | 子指标 |
|---|---|---|
| `large_ledger_fold` | `sprawling::ask`，重建每个视图的生产全路径 | `harness` |
| `large_worktree_placement` | `memory::Checkpoint::ensure_base` 之后 `Worktrees::claim`／`release` | `whole` |
| `long_session_forwarding` | `channels::ServerFrame::Event` 装帧＋序列化，即 socket 之前的本地半段 | `harness` |

失败出口：域错误按其 `AxError`（动作/主体/稳定码/恢复语）格式化成一行；bench 自身的失败（零样本）构造 `AxError::failure(AxCode::InvalidArgs, …)`＋`with_recovery`，不新增码（§9-16 的口径）；Main 打 `bench failed: …` 且退出非零（既有形）。

#### 8-6.1 两个设计

**A（选中）：读数行一个文法、机器类属进字段，基线读数（含机器类属）进 ARCHITECTURE.md 性能册，棘轮纪律挂 `xtask/budgets.toml` 的 `[local_latency]` 行——只降不升、放宽需单独提交。** 理由：register 的既有定规是「只有机器能两次同样测量的量才设门」（budgets.toml 头注），wall-clock 记录不设门；机器类属字段使异类机器的读数天然不进同一张表。

**B（落选）：像体积那样把延迟读数设门（超标即 CI 红）。** 落选理由：同一处定规写着「gating them would make a busy runner look like a defect」（budgets.toml 头注与 ARCHITECTURE §11 同句）；读数回归由棘轮纪律与单独提交的放宽手续治理，不由 CI 红绿治理。

**红**：`a_reading_line_is_stable_and_carries_its_machine_class`——一行读数按字节对拍既有文法且带 `machine_class` 字段；`every_load_scenario_reruns_and_emits_the_stable_format`——本 crate 的每个场景各跑两遍，每行键序恒为文法键序（可复跑、格式稳定）。

**多 run 并行不在本 crate 里量。** relay、`serve_flight` 与 desk 都是 `sprawling` 的 `pub(crate)`，本 crate 够不到，这里的场景只能抄一份 relay 的形状：数条 lane 经 mpsc 汇到一条线程，计时只包住那条线程上的一次 `append`。抄件量的是抄件：它的 `harness` 是 MemLedger 一次追加（5 µs 量级），而一次往返的代价取决于生产循环怎么等，抄件没有那份等法，也就量不到它；它的 `persist` 每条一道屏障，生产的 `append_all` 一批一道；盘的份额由那件仪表 `store=disk` 与 `store=memory` 两行之差读出，所以 `SubMetric` 没有 `persist`。**败给的方案**：给 `sprawling` 开一扇公共门让本 crate 驱动 `serve_flight`。那扇门没有生产调用者，而仪表放在 crate 内已经能驱动生产循环本身（sprawling-SPEC 8-84 的决定）。

## 9–16 工作流程／实现／边界／错误／依赖／硬编码／影响面／测试

- 流程：测试构造 drafts→MemLedger append→checker／conformance／对拍 JsonlLedger；另有 Scenario→run_scenario→check_chain＋事件序断言。
- 实现：append＝from_draft→canonical_line→chain_hash 推进；无别的逻辑。
- 边界：空 Ledger check 通过；单创世行通过。
- 错误：透传 kernel/replay 的 AxError，不新增码。
- 依赖：kernel（features=["conformance"]）、memory（对拍＋夹具）、runtime（复用 verify）；dev：tempfile。
- 硬编码：无。
- 影响面：剧本执行器建于本 crate 之上；夹具脚本是后续验证工作的起点。
- 测试：conformance 双实现、字节对拍、夹具对拍、篡改检出。

## 17 模型体验

零字节：dev-only 设施，恒不进任何 prefix。

## 18 文档同步

落剧本执行器时增章；夹具更新须与 memory/kernel 的字节规范同一变更集。
