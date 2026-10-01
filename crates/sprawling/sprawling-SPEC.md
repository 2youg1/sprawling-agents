# sprawling-SPEC.md — bin（main＋assembly＋嵌入链）

> crate：`sprawling`（唯一 bin）。§1–§8 说这个 crate 整体；8-2 起每节一个接口，各节自带形状、签名、失败与决定。

## 1 需求分解

`sprawling` 是这座城唯一的二进制，由五块组成：命令行（`bin::main`，动词表在 `main::verbs`，8-89）；装配点（`bin::assembly`：造 `RunWorker`、把端口接上、墙钟的唯一采样点 `assembly::SystemClock`）；serve 时的服务面（`bin::serving`：门、进程日志、视图的折叠线程，8-92）；读面（`accounting::views`）与机器体检（`bin::doctor`）；以及把客户端包嵌进二进制的 `build.rs`（8-83）。

## 2 验收标准

每节写出钉住它的测试。退出码是一张表（8-103）：读不懂的命令行退 2，并给出最近的动词名。

## 3 假设与歧义

评审楼的 worktree 按房间保留：`Site::place_tree` 以 `room-<地址 BLAKE3 摘要前 16 位十六进制>` 为名认领，同一房间的下一轮活取回上一轮留下的树（storage-SPEC 8-9），不再每轮全量检出、再整目录删除；`RunWorker::over` 拿到账本写者后解开上一个写者留下的全部 worktree 锁。树按楼的 scope 领、按同一个 scope 立检查点和献出（`workbench::tree_scope` 是这一个 scope 的唯一定义）：再领一棵留着的树只检出 scope（storage-SPEC 8-9），献出的 `Checkpoint::land` 只按 scope 暂存，于是合并进干线的提交只动 scope。第一次放置仍是全量检出：`Worktree::add` 总做一次全量检出，git2 0.21 没有把 `git_worktree_add_options.checkout_options` 暴露成安全接口，而 `storage` 禁 `unsafe`。上限只称一次检出、不称留着的树之和，定在 storage-SPEC 8-9。树的放置与 MCP 缺表时的那次连接，连同 `lay_out_workbench`、`freeze_plan`，在驾驶这个 run 的 lane 里做（8-113）。

## 4 现状分析

各节的「本节接口的当前状态」写该接口还没做完的部分，本节不另列。

## 5 权威信源

bin 子命令面；装配层是 Main；ARCHITECTURE.md §2（客户端嵌入链）与 `architecture.toml`（模块图里 bin 段那些条目）。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。（`gateway::credential::Custodian` 住 `custodian` 同例，公共拼写不变）。（`wire::command::Command` 住 `kind` 同例）。

## 7 模块边界

`main`：CLI 分发与呈现；`assembly`：唯一知情点，句柄/时钟/种子/spawn 注入处。
**本 crate 不做什么**：不含任何判定（判定住 kernel）；账本与内容仓库的写盘住 storage。

## 8 接口先行

```rust
// bin::assembly
pub struct SystemClock;   // 墙钟的唯一采样点（clippy.toml 的 disallowed-methods 只在这里放行），生产的 accounting::Clock
```

## 8-2 派活回路

```rust
// bin::assembly —— 字段按所持状态分成六组（8-110、8-111），与整城共用的账本、CAS、日志
pub struct RunWorker { /* 私有 */ }
impl RunWorker {
    pub fn new(city_root: &Path, vault: gateway::Custodian, log: runtime::diagnostics::Diagnostics) -> Result<Self, AxError>;
    pub(crate) fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>);
}
// bin::assembly::listening —— 先占端口，再开写者（8-88）
pub async fn listen(serving: Serving) -> Result<Listening, AxError>;
impl Listening { pub async fn serve(self) -> Result<(), AxError>; }
```

- **单写者**：账本只在写者线程（`assembly::attending`）上写；lane 写的行经 `accounting::worker::relay` 交给它（8-42-2），所以一座城的历史只有一个追加者。
- **命令受理与命令执行分开**：socket 任务只把命令放上命令台（`accounting::worker::desk`），命令在写者线程上执行。刷新页面不会杀掉工作，而进展从事件流回流——「关掉界面再打开」与「从未关过」在服务端看来因此无差别。
- **事件流的源头是 Ledger 本人**：`JsonlLedger::observe` 在**持久化之后**逐条回调，回调把记录扔进 broadcast。服务端因此推不出一条历史里没有的事实。
- **工具名录与工具台同源**：`Catalog::admit_tool` 产 `ToolDef`（模型看到的），`ToolBench::register` 负责路由（实际跑的），一次登记喂两边——否则“模型以为存在的工具”与“真能跑的工具”会成为两份名单。
- **RunId 是推导而非抽取**：`dispatching::agreeing::run_id_for(job, addr, now)`，`b3(job|addr|now)` 前 16 字节。同一毫秒对同一地址派同一件活就是同一个 Run，且标识符里不进随机数（确定性第 7 条的同一条理由）。
- **一跑没有回合上限**：刹车是 `Cancel` 与 `Halt`（先判定后动手那一节 8-40）。
- **`init` 写 `City.md` 入城**：二进制携默认本，城里那份是用户可改的权威；每次组装 prefix 读城里的那一份，代码里恒不长第二份副本。

## 8-2b CLI 补齐

- **`resume <city>`**＝启动扫描：一遍流式验链（`runtime::replay::fold_ledger_dir`），验过的每条记录交给 `runtime::replay::DanglingCalls` → 逐个补记 `E_TOOL_OUTCOME_UNKNOWN` 的 `tool_result`（幂等：已闭的账不重补）→ 报待批数。`runtime::replay` 补写面自此有生产消费者。续跑已批准的活仍在 serve 的 `answer_approval` 路径上，两者不重叠。
- **`fork <city> <run> <seq> [addr]`**＝世系记录形：验链 → `runtime::fork::prefix` 验界 → 节点归属验（seq 处的事件必属于母 Run，否则拒）→ `run_forked` 落账携新 RunId。**不自动发车**：驱动新 Run 是人的下一步 Dispatch；逐字节母前缀入窗属并发期的 must-read 网，不在本形。`Command::Fork` 同路。
- **`adopt <city> <addr>`**＝收编已存在目录为楼（语义住 `city-SPEC.md` §8-3）。
- **`serve` 增 `--web-dir <dir>`**：开发回路逐请求读盘；发布形恒走嵌入表（`wire::ClientAssets`，语义住 wire-SPEC §8-2）。
- **默认地址住 `kernel::consts_policy::DEFAULT_AT`**（`"127.0.0.1:8787"`）。`up`／`serve`／首屏／`call` 与 `enrol` 的 `--at` 缺省，以及「不是 socket 地址」那条恢复语里的示例，全部读这一个值：一个人被告知的地址与真正绑上的地址不能是两处写法。安装脚本与 README 引用同一个值，改它即改这一处。

## 8-3 视图与 Spine

```rust
pub(crate) struct Views { city_root, hot: HotView, attribution: Attribution, approvals: BTreeMap<String, ApprovalSummary> }
impl Views { fn apply(&mut self, &EventRecord) -> Result<(), AxError>; fn prepare(&self, &Query) -> Prepared; fn answer(&mut self, &Query) -> Answer; }
impl Prepared { fn finish(self) -> Answer; }                                 // 锁外读盘，见 8-100
impl Views { pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>; }   // 一次性的读：先审计整条链，再从快照起步（8-91）
impl CityAsk { fn read(self) -> wire::Answer; }                     // 放下快照后列楼、读计划，见 8-100
```

- **视图冷重建与热折叠共用 `apply`**：启动时把 Ledger 逐行喂进去，其后由 `JsonlLedger::observe` 喂。测试断言两条路径答案逐字段相等——这就是「projection 可弃」的可执行形式。
- **计划的投影是 `Roadmap.md` 的缓存**：那份文件就是计划，Agent 用 edit 工具改它。`Views` 持 `Arc<Mutex<accounting::plan_view::PlanView>>`，`roadmap_*` 记录经 `apply` 让缓存失效，查询在锁外读盘回填（8-100，accounting-SPEC.md 8-6）。
- **读不懂的行照显**：`problems` 随答案回到界面；没有 Roadmap 的楼答 `Progress::Unplanned`（它没有 ratio 方法，故界面画不出百分比不是守规矩，是无从下手）。
- **保留前缀不是楼**：`.` 开头的目录跳过，`.sprawling/` 因此恒不被当成一栋楼。

## 8-4 MCP 接线

三种传输、兑付与 `McpLink` 住 `agent_protocols::mcp`（签名见 `crates/agent_protocols/Spec.lean` §8-17）；本 crate 只把一栋楼的 `[[mcp]]` 表接成工具表。

```rust
// accounting::worker::workbench::servers —— impl Laying（lane 准备派活时持有，8-113）
fn mcp_tools(&self, config: &FrozenConfig, write_root: &Path, confidential: bool)
    -> Vec<agent_protocols::McpTool>;   // 起不来的 server 缺席并留下诊断，恒不拒整次 dispatch

// accounting::worker::mcp（形状 3 常驻表）：worker 持有，一台 server 一项，每项一把锁
pub(crate) struct Residents { /* 私有：Mutex<Vec<Arc<Keyed>>>，键 (McpServer, write_root)，每个 Keyed 自带 Mutex<Option<Resident>> */ }
impl Residents {
    pub(crate) fn tools(&self, server: &kernel::McpServer, write_root: &Path, confidential: bool,
                        resolve: &gateway::SecretResolver)
        -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError>;   // Reached::{Connected(Handshake), Resident}
}
```

- **兑付只有一处，三种 transport 共用**（`agent_protocols::mcp::redeeming`）：一个子进程要的环境变量与一台主机要的请求头是同一件事——一个名、一个值、以及一个可能是凭证的值。兑付发生在读配置的那一刻而不是第一次调用时：金库里没有的引用是一个配置错误，而能处理它的人正在编辑那份文件，不是一小时后工具不应答的那个模型。`Redeemed` 的 `Debug` 只报名字，明文的唯一出口是 `expose`。
- **交给子进程的名字收不回来**，故 `start` 收到的 `env` 已经是兑付好的值，明文只在这一次调用里存在；`env` 是**添加**到本进程已有的环境上，否则一台 server 会找不到 PATH 与 HOME。
- **一台 server 一个子进程，寿命是 worker 的寿命**（`accounting::worker::mcp::Residents`，常驻连接表）。每次 dispatch 都起子进程、握手、list，是 `[prepare_dispatch_ms]` 里 servers 那一半的全部开销；表把它降到一次。键是整条 `McpServer` 声明加上运行根 `write_root`：声明任一字段变了就是另一台 server，起在另一个目录里的子进程也不是同一台，因为子进程的工作目录在启动时定下、之后改不了。命中且子进程仍在运行时，这次 dispatch 用表里记下的清单（`ToolMeta` 与远端名成对）和同一条连接构造 `McpTool`，不握手也不 list；子进程已退出或问不出状态，就丢掉这一项并重连，这就是 server 死后的重连。HTTP 与 SSE 的表项恒视为可用：连接逐次请求，一台死掉的主机在那次调用里失败并留名。表按键上锁：表自己的锁只在找键、添键的那一刻持有，连接、握手与 list 在这一键自己的锁里做，于是同一台 server 的两次查询排在同一把键锁上，先到的连接、后到的命中，不同 server 的连接互不等待；`tools` 取 `&self`，表是 `Sync`，可以被几条线程同时问。几条 lane 同时用一条 stdio 连接时由连接自己的锁排队。代价有两件：清单在连接那一刻读定，server 在两次 run 之间改了清单，要等子进程重启才看得见；配置里删掉的 server 的子进程活到 worker 落地。worker 落地时表落地，最后一个句柄落地时 `Drop` 杀子进程，于是「谁来回收」仍不需要第二份名单。缺表的那一次连接在准备这个 run 的 lane 里建立，lane 借的是表的 `Arc`（8-113）。
- **读取线程是句柄的一部分，spawn 点仍在 bin**（确定性七条③的口径：并发归装配层）。同步读一根管道没有期限，而一个不回答的 server 会把整个 Run 挂死。故 `start` 起一条只读 stdout 的线程，`call` 用 `recv_timeout` 等它；超时即杀子进程并三段式拒。**线程恒不泄漏**：杀子进程关掉管道，读到 EOF 即结束。
- **一台 server 一个子进程，一次 dispatch 一条命**：工具表随 Run 冻结，子进程的寿命因此就是 Run 的寿命。最后一个 `McpTool` 落地时 `Drop` 杀子进程，于是「谁来回收」不需要第二份名单。
- **读取线程是句柄的一部分**（ARCHITECTURE 确定性规则 3 为 `agent_protocols::mcp::stdio` 与 `agent_protocols::mcp::sse` 各留一个每连接一条的 reader）。同步读一根管道没有期限，而一个不回答的 server 会把整个 Run 挂死。故 `start` 起一条只读 stdout 的线程，`call` 用 `recv_timeout` 等它；超时即杀子进程并三段式拒。**线程恒不泄漏**：杀子进程关掉管道，读到 EOF 即结束。
- **期限从声明里来，不在适配器里另写一个数**：`ToolMeta.timeout`（`tools_from` 写的 `TimeoutMs(60_000)`）既是对模型的承诺，就应当是真正被执行的那一个；否则该字段只是装饰。故期限随 `Outbound::call` 入参。
- **起不来的 server 缺席而不拒 dispatch**：与 `city::library` 对「楼里点名却不在架上的 SKILL」同形——模型看到的名录恒等于真能跑的工具表，缺席的那一件在诊断里留名。一个外部服务今天起不来，不是这栋楼今天不能干活。
- **confidential 楼：一条规则两层后果，不是两份判定**。工具能不能存在归 `agent_protocols::McpTool::new`（构造点拒，恒是权威）；**进程该不该被拉起归装配层**，因为进程寿命本来就是这一层的职责，而一台 MCP server 可能在启动那一刻就出网。故 confidential 楼在拉起任何子进程之前就跳过整张 `[[mcp]]` 表并留一条诊断；工具层的拒仍在，它是那一层失守时的兵底。两层同向，因此不会出现「只改一处」的漂移。
- **`Effect::Connector` 是这次接线带来的 kernel 变更**（语义住 kernel-SPEC §8-23）：接线前 `tools_from` 写的是 `Effect::Egress`，而出站门从 **调用参数**里读 `host`——外部工具的参数表由 server 的 `inputSchema` 决定，里面恒没有 `host`。第一次真调用当场拿到 `E_INVALID_ARGS: declares Egress but named no host`：这就是「一个适配器是假想缝」的同一条道理在工具面上的实例——没有调用方的声明从未被那道门验过。
- **discover 先于 list，但今天不据它分支**：它当下的作用是在把任何工具交给模型之前，先证明对侧真的会应答；版本协商要有第二个版本才成立，而字段名本库今天无法从一台真的 server 上核对。读到什么写进诊断，不写进判定。
- **口径不变的那两件**：外部工具与 L0 工具同落 `kernel::tool` 缝，故结果恒自动进污染环，装配层无解包面；调用由 `ToolBench` 路由，故 `tool_called`／`tool_result` 两行自动落 Ledger，不为它另写一条入账路径。

## 8-4b MCP 的第二条传输

`agent_protocols::mcp::http` 是 `Outbound` 缝的第三个适配器（stdio 子进程、ScriptedOutbound、HTTP），也是这条缝第一次真正被两条生产路径共用。`McpServer.transport` 从两个裸字段改成穷尽枚举 `McpTransport { Stdio{command,args}, Http{url,header} }`：**一行既写 command 又写 url 就是一行要读者去猜的配置**，故配置层当场三段式拒。差异全部花在 `agent_protocols::McpLink` 这一个枚举里，其上的接线仍是一条路。

两条口径：①**事件流只取第一条 `data:`**，读不出就拒，恒不把两条答案拼成一条工具结果；②**拒词不引用对侧正文**（服务端的错误页是别人写的字），只说状态码与该查什么。

## 8-4c 会话、凭据与报错地址

- **`HttpServer` 持会话，且克隆共享它**（规范详 `crates/agent_protocols/Spec.lean` §8-3）。一台 server 就是一个会话，不论一次 Run 拿了它几件工具；两个克隆各发一个 session id 就是与同一台 server 开两场对话，而它只开过一场。协商版本从**携 `result.protocolVersion` 的那一条答案**学得——按生命周期，那就是 `initialize` 的答案，因为它是一条连接的第一句话，没有更早的答案能持有该字段。
- **`header` 兑付 `secret:` 引用**。`redeem_header` 把 `Name: secret:realm/name` 在最后一刻换成真值，与 endpoint 凭据同一条路。不这么做，一把付费档的 key 会明文躺在楼里的 `CONFIG.toml`，而 `xtask secret` 看不见它（城市配置不在仓库内）。不是引用的值原样通过：头里是个账号名或固定标记的 server 无物可兑。
- **报错地址指向真正出事的传输**。`agent_protocols::McpLink::site()` 按 `McpTransport` 分派；把所有 MCP 失败都挂在一个传输名下，会把跟着它去查的人引到错的文件。成功时同样留一行：对侧叫什么、说哪个版本、给了几件工具。

**真机验收**：一座真城接一台托管 server，诊断行为 `exa is exa-search-server speaking 2025-06-18, offering 2 tool(s)`；模型自主调用其搜索工具、读回真实结果、一个回合内给出答案并 `run_frozen{completion: done}`。

## 8-4d 桌面是这个二进制自带的工具（`accounting::worker::workbench::desktop`、`bin::main::desktop`）

一栋楼的 `RULES.toml` 写 `desktop = true`，它的 run 就拿到这台电脑的桌面那六件工具（`crates/desktop/Spec.lean` §8-7）。人不在 `CONFIG.toml` 里手写 `[[mcp]]`，也不另装程序。

```rust
// bin::assembly —— 起桌面 server 的那个程序；装配点收下它，而不是自己去问
pub type DesktopProgram = fn() -> std::io::Result<std::path::PathBuf>;   // 生产交 std::env::current_exe
impl RunWorker { pub fn with_desktop_program(self, program: DesktopProgram) -> RunWorker; }

// accounting::worker::workbench::desktop（形状 1 判定）
pub(in crate::assembly) const DESKTOP_LABEL: &str = "desktop";
impl Laying {
    /// 这次 run 要连的 server：楼的配置写下的那些，再加上规则要桌面时这个二进制自己那一台。
    pub(in crate::assembly) fn servers(&self, site: &Site) -> Vec<kernel::McpServer>;
}

// bin::main::desktop（形状 4 适配器）
// `sprawling desktop [scope]`：在 stdin／stdout 上当桌面 MCP server；scope 是 DESKTOP.toml 的路径，缺席即全拒
pub(super) fn verb(scope: Option<&str>) -> ExitCode;
```

- **同一个二进制，另一个进程**：规则要桌面时，`servers` 给这栋楼添一条 stdio 声明：`command` 是正在运行的这个可执行文件，`args` 是 `["desktop", <这栋楼的 DESKTOP.toml>]`（`city::desktop_scope_path`，住城根下这栋楼的保留子树，评审楼的 worktree 里没有它）。之后它与任何一条 `[[mcp]]` 走同一条路：经 `accounting::Connectors` 连上、握手、list，常驻连接表管它的寿命（§8-4）。子进程这条边界是故意留的：UI Automation 的 COM 状态、`SendInput` 与 `crates/desktop/` 里那些带 `SAFETY:` 的 `unsafe` 都跑在子进程里，城那个唯一写者的进程里一行也不跑；COM 出错或子进程 abort，结束的是子进程，城只在那一次调用上拿到 `E_TIMEOUT` 或 `E_TOOL_UNAVAILABLE`；两者都发生在请求交出之后，所以都带 `Retry::Unknown`（`crates/agent_protocols/Spec.lean` §8-17）。桌面自己的拒绝以 `isError` 结果到达，城把它读成一次失败，拒词全文进 subject（`crates/agent_protocols/Spec.lean` §8-1c）。MCP 这条路上已有的规矩一条不改：工具表随 run 冻结，`kernel::gate::undoable` 按远端名前缀 `desktop.` 升给人，期限到即杀子进程。
- **程序路径是收下的，不是问出来的**：`RunWorker` 持一个 `DesktopProgram`，生产交 `std::env::current_exe`，测试交 `CARGO_BIN_EXE_sprawling`。理由与 `Browsers`（§8-45-2）相同：它在主机上起一个程序。问不出路径或路径不是 UTF-8 时，这栋楼这一次没有桌面工具，诊断里留一条 `Refuse`，派活照常。这与起不来的 `[[mcp]]` server 是同一个答法。
- **楼自己写了 `label = "desktop"` 的 `[[mcp]]`，就用它那一条**：两台 server 用一个标签，同一个工具名就指向两个进程（city 对同层重名的拒绝是同一个理由）。人明写的那一条优先，自带的这台不起，诊断里留一条 `Decide`（诊断的级别里没有「提醒」一级，`Decide` 说的正是一个判定为什么取了这个值）：要用自带的，删掉那一行。
- **装配层不按平台分支**：非 Windows 的机器上这台 server 照样起，每次调用答 `E_TOOL_UNAVAILABLE` 并报出平台名（`crates/desktop/Spec.lean` §10 设计四）。在这里再判一次平台，这条规则就有了第二个家。
- **confidential 楼够不着它**：`city::policy` 解析时就拒绝 `confidential` 与 `desktop` 同真，`mcp_tools` 对 confidential 楼也不起任何进程。这里不判第三次。
- **体积**：把 `crates/desktop/` 链进来使 release 二进制变大多少，读数只记在 `tools/xtask/budgets.toml` 的 `[release_binary]`。增量来自 `crates/desktop/` 自己的代码、`image` 的编码器与 `windows` 绑定；std、serde_json、toml、png 两边共用，只算一份。
- **不内置任何模型（定规）**：桌面给模型的文字反馈先取 accessibility tree；OCR 与 ASR 都经人接入的端点，二进制里不带任何模型的权重。`crates/desktop/Spec.lean` §15.2 记着这条线后面还欠的东西。
- **被否的两条路**：①照旧另发一个 `sprawling-desktop` 可执行文件，由人放上搜索路径再手写 `[[mcp]]`：一件功能成了两个制品，版本要对齐，人还得知道那一行怎么写；②把单独编出的桌面可执行文件的字节嵌进本二进制，运行时写到盘上再起：运行时往盘上写可执行文件，std 也多带一份。**重开参数**：`crates/desktop/` 使 release 二进制增大超过 1 MiB（`[release_binary]` 的 slack），就回到第一条路重新比较。

**验收**：`crates/sprawling/tests/desktop.rs` 的 `a_building_given_the_desktop_is_offered_its_six_tools_from_this_binary`。一栋楼的 `RULES.toml` 写 `desktop = true`，没有任何 `[[mcp]]`，城起真的 `sprawling desktop` 子进程，模型收到的工具表里有 `desktop_desktop_windows` 等六件。

## 8-146 Windows 上构建这个二进制要有 Zig（`bin::doctor::table::toolchain` 的 `zig` 一行）

`sprawling` 按路径链接 `crates/desktop/`，`crates/desktop/` 在 Windows 上链接它的 FFI 缝 `crates/desktop/ffi`，而那个包的构建脚本用 `zig build-lib` 编一片 Zig 叶子（`crates/desktop/Spec.lean` §8-12）。所以 Windows 上编这个二进制、跑 `just check` 都要一个 Zig，且是 `crates/desktop/ffi/zig-version` 钉住的那一版。

```rust
// bin::doctor::table::toolchain（形状 6 数据）
pub(super) const ZIG: Requirement;   // develop 层，Required；探测 `zig version` 以钉子开头的一行
const ZIG_PIN: &str = include_str!("../../../../desktop/ffi/zig-version").trim_ascii_end();
```

- **钉子只在一处**：`ZIG_PIN` 是 `crates/desktop/ffi/zig-version` 的 `include_str!`，与 `LEAN_PIN` 读 `lean-toolchain` 同一种写法（8-58）；探测是 `zig version` 的输出以钉子开头，Windows 的装法是 `winget install --id zig.zig -e --version <钉子> --scope user`，macOS 是 `brew install zig`，Linux 印出官方下载页。构建脚本与 CI 的安装步骤读同一个文件。
- **`Required` 而不是 `Optional`**：Windows 上没有它，`just check` 编不出这个二进制；在别的平台上 Zig 叶子不编，有它也不多花什么，而 `Need` 不按平台分（8-58），两害取其轻是把它列为必需。
- **位置**：表里 `lean` 之后、`uv` 之前：它与 Rust、Lean 同属编译这份代码要的工具，装法不依赖表里更早的任何一行。

**验收**：`prereqs.tsv` 与表渲染出的文本逐字相等（8-58 的现有测试）；`zig` 一行的探测与装法读的是钉子文件里的那一版。

## 8-4e harness 居民的一次 run（性质已证明）

一个房间的居民可以是五家官方 harness 之一（`crates/agent_protocols/Spec.lean` §8-19）。派活到这样的房间时，城起那一家的进程，在房间自己的 worktree 里开一场 ACP 会话，把它汇报的东西记进账本，在它答出停止原因时结束这次 run。

**性质的权威是 `crates/agent_protocols/spec/Harness/Session.lean`**，六组定理：

- **汇报恒不是准入历史**（`a_report_is_never_admitted`）：一次 harness run 的准入记录只有它的开始、检查点、它对城那次 prompt 的回答与冻结，中间汇报多少、汇报什么都不改变这一点。
- **截断先变成取消**（`a_halt_is_a_cancel_before_anything_else`、`a_second_halt_sends_nothing`）：城观察到一个罩住这个房间的停摆、或楼规的墙钟上限到了之后，run 发出的下一件事就是 `session/cancel`，此后的汇报排在它后面；第二次截断什么也不发，结局读头一次的那个。
- **树先提交，冻结的 run 是历史**（`nothing_follows_the_stop_reason`）：停止原因到了，run 先把树提交成检查点，再记下回答、冻结，此后什么都不记、什么都不发。
- **会话断了就不记回答**（`a_lost_session_freezes_cancelled_with_no_answer`）：没答出停止原因的会话冻成 `Cancelled`，账本里没有一条 harness 没给过的回答。
- **Done 要一次说了话的 `end_turn`**（`done_needs_an_end_turn_that_spoke`、`a_deadline_never_freezes_cancelled`）：其余停止原因与空回答都冻成 `Limit` 或 `Cancelled`，墙钟上限恒不读作 `Cancelled`。
- **只有准入的记录能作证**（`a_cited_record_is_admitted`、`no_report_is_admitted`）：Done 引的那一行是 harness 对城那次 prompt 的回答；汇报恒不在准入历史里，所以恒不作证。

**它守的是 ARCHITECTURE §5 第 4 步的弱形**。harness 自己执行工具，城准不了也拒不了它做的事，只能记它选择汇报的东西。所以「每个效果先成为事件」在这里分成两半：城自己决定的事（run 开始了、停摆变成了取消、城那次 prompt 得到了什么回答、run 怎么结束）是准入历史；harness 一路上说它做了什么是汇报，在它说了之后才落账，恒不被读回来当作一个判定。

**已定的十条**：

1. **汇报是一个新的 record-only 种类 `harness_reported`**：载荷是 `kernel::event::record::HarnessReported`（kernel-SPEC §8-4），一条汇报一行：回答或推理的一段、harness 开始的一次工具调用与它的状态、一种本城没有读法的汇报（只记名字），以及 permission 的问与城的答。种类名进握手哈希，旧页面在握手处被拒，`WIRE_V` 不为此进位（wire-SPEC §12.1）。回答「城做了什么」的 fold 恒不读它，只有回答「harness 说了什么」的视图读它。
2. **停摆到取消**：城在把下一条汇报落账之前查一次这个房间所在的停摆；罩住了，就先落 `cancel_received`，再发 `session/cancel`，然后照常读到 `stopReason: cancelled` 为止。复用已有的 `cancel_received`，因为它记的正是「这个 run 收到了一次取消」。
3. **停止原因到结局**：`cancelled` 冻成 `Completion::Cancelled`，截断它的是墙钟上限时冻成 `Completion::Limit`；`max_tokens`、`max_turn_requests` 与 `refusal` 冻成 `Completion::Limit`，因为 run 是撞上了什么而停，不是做完了；`end_turn` 见第 8 条。每一种停止原因都先提交检查点、写一条 `harness_answered`，再冻结：`Session.lean` 里 `checkpoint`、`answer`、`freeze` 依次发出，对五种停止原因都一样。这张表只在 `runtime::run::harness` 一处判（runtime-SPEC §8-52）。
4. **confidential 楼拒绝 harness 居民**：`agree_to_work` 在写任何东西之前答 `E_GATE_DENIED`。harness 执行自己的工具，并把房间的内容送到它自己厂商的服务器，confidential 楼「数据进来不出去」的承诺对它不成立。
5. **harness 恒在房间自己的 worktree 里跑**：用评审楼的同一种租约，不论楼的 `review` 设了什么。城管不了它写什么，但管得了它写在哪：它的写入只经已有的评审合并进入城的主树，而合并是准入的。harness 够不着城的协作工具（第 7 条），所以一次冻成 `Done` 的 run 落地时由城替它开请求（§8-124）。
6. **进程归 `agent_protocols::harness`**：起 `Launch` 的程序与参数，工作目录是那棵 worktree，走管道，落地即杀。理由与 `agent_protocols::mcp::stdio` 相同：字节怎么走归协议那个 crate（`crates/agent_protocols/Spec.lean` §7）。
7. **`session/new` 的 `mcpServers` 这一版给空表**：城的工具要经一台城自己的 MCP server 交出去，那是另一件活。楼里 `[[mcp]]` 的 server 与城自带的桌面（§8-4d）也不转交：harness 直接调它们，`kernel::gate::undoable` 那道升给人的门就够不着了。

8. **`end_turn` 由城自己的记录作证**：`session/prompt` 答出停止原因之后，城先把这棵 worktree 提交成自己的检查点（`checkpoint_committed`，harness 改了什么由城记下；五种停止原因都提交，一次被截断的 run 改了什么同样要记），再写一条 record-only 种类 `harness_answered`：载荷携停止原因，与这一回合 harness 回答城的那段文字（`agent_message_chunk` 依次拼起来）。run 冻成 `Completion::Done`，证据引这一行，`kernel::completion::Evidence` 因此在 `tool_result`、`model_returned` 之外多收一种。这与模型 run 的证据同一个标准：模型的 run 以它最后那条 `model_returned` 作证（`runtime::run::lifecycle::concluded`），harness 的 run 以它对城那次请求的回答作证。回答是空的，冻成 `Limit`，与 `concluded` 对空回复的判法相同。**被否**：给 `Completion` 加一个「harness 自称完成」的变体（动 kernel 的公开面与每一个按结局分支的读者）；在 bench 上立一件「harness」工具，把 prompt 包成 `tool_called`／`tool_result`（那是一件没有模型调用过、也没有门判过的工具）。
9. **permission 一律答它给出的第一个 `allow_once`（定规）**：没有 `allow_once` 就答第一个 `reject_once`，两者都没有就答 `cancelled`。恒不答 `allow_always`：那是替以后的调用做决定。问与答各记成一条 `harness_reported`。理由：城不能按工具名授权（ACP 规格），能兜住的是这棵 worktree（第 5 条）；一律拒绝会让默认要问的 harness 什么都做不成；交给人则要让 Approval Inbox 收动作，而它今天只收设计问题，且一次挂起的 prompt 占着一条车道。**重开参数**：Approval Inbox 开始收动作。
10. **房间的 harness 由 `CONFIG.toml` 的 `[resident] harness` 点名（定规）**：值是 `agent_protocols::Harness::as_str` 的五个拼写之一，在城／楼／房间的梯子上取最近一级（`city::settled_harness`，city-SPEC §8-4）。房间在派活时才开，所以这个键实际写在楼层或城层。`city` 只把它当字符串读进来：`city` 只见 `kernel`，五个拼写的权威在 `agent_protocols::harness::roster`。`agree_to_work` 在写任何东西之前用 `agent_protocols::Harness::parse` 判它，不认识的拼写答 `E_CONFIG_INVALID` 并列出五个。`[model] name` 不在梯子上，它是房间自己那一层的会话记录（city-SPEC §8-14）；房间有这条记录时，这段会话以模型走完，harness 从 `/new` 开的下一段会话起生效。同一层同时写 `[model] name` 与 `[resident] harness` 在解析时即拒：一个房间的居民是模型还是 harness，要读者去猜就是配置写错了；城自己的写路径也写不出这样一份文件（city-SPEC §8-4b）。**被否**：派活帧上加一个字段，那要动 wire，而居民是一个站着的身份（词汇表 Resident），不是每次派活现选的；梯子上的 harness 压过会话记录，那会让一段会话中途换居民（city-SPEC §12.6）。

**接线四段**：①`agent_protocols`：`Harness::parse`、会话的取消与起进程的那一半（`crates/agent_protocols/Spec.lean` §8-19）；②`kernel`：`harness_reported`、`harness_answered` 两个种类与 `Evidence` 多收的一种；③`city`：`[resident] harness`（city-SPEC §8-4）；④派活路径上的 harness run：`runtime::run::harness` 写它的每一行（runtime-SPEC §8-52），`accounting::worker` 判居民、借树、起 harness、落地（§8-124）。

## 8-124 harness 居民的派活路径（`accounting::worker::dispatching::harness`、`accounting::worker::driving::harness`）

§8-4e 定了一次 harness run 必须守的性质，本节是它在派活路径上的接线：判一次居民，借树，起 harness，驱动一个回合，落地。模型 run 的路径一字不动，两条路在 `agree_to_work` 分开，在池与 `land` 的入口汇合。

```rust
// dispatching —— agree_to_work 判一次居民，只有派活路径用
pub(super) enum Seat { Model(Agreed), Harness(HarnessSeat) }
pub(super) struct HarnessSeat { building: city::Building, rules: city::BuildingRules, harness: agent_protocols::Harness }
impl RunWorker { pub(super) fn agree_to_work(&mut self, at: &Assignment) -> Result<Seat, AxError>; }

// driving::harness —— 第二个驱动函数，与 drive_run 并列
pub(crate) type Prompting = Box<dyn FnMut(&str, &mut agent_protocols::Listener<'_>)
    -> Result<agent_protocols::Answer, AxError> + Send>;             // 一场开好的会话：一次 prompt，读到它结束
pub(crate) type StartHarness = Arc<dyn Fn(Harness, &Path) -> Result<Prompting, AxError> + Send + Sync>;
pub(crate) fn drive_harness<L: Ledger>(half: HarnessHalf, ledger: &mut L, context: DriveContext) -> HarnessDriven;
impl RunWorker { pub(crate) fn with_harnesses(self, start: StartHarness) -> RunWorker; }   // 测试的门
```

- **居民只判一次**：`agree_to_work` 在写任何东西之前得出 `Seat`。`Seat::Model` 带着原来的 `Agreed` 走原来的路；`Seat::Harness` 不选模型、不造适配器、不调 `choose_shape`，所以城恒不把 `[model] name` 写进一个点名了 harness 的层（city-SPEC §8-4b 的复读仍在，那是第二道）。`Staged`、`Flown` 与 `Continuation` 各是两臂的枚举，池只调 `Staged::fly` 与 `land`，不看居民是谁。
- **判哪一个地址**：这次派活会开一间新房间时（楼地址，或带会话名），判它所在的楼；否则判地址本身。新房间自己那一层是空的，梯子的答案就是楼的答案；旧房间自己那一层的会话记录说的是那间旧房间，不是新开的这一间。「会不会开新房间」由 `dispatching::session` 一处回答，`session_for` 与这里读同一句。
- **harness 一臂的三条拒绝**，次序即调用方有权先听到的次序：拼写不是五家之一答 `E_CONFIG_INVALID`，subject 是 `<地址>: <拼写>`，`nearby` 是 `Harness::ALL` 的五个拼写，恢复语给出点名它的那份 `CONFIG.toml`；楼是 confidential 答 `E_GATE_DENIED`（§8-4e 第 4 条）；派活点名了模型（`-m`）答 `E_CONFIG_INVALID`：居民是 harness 的房间不接一个模型，改点名要去 `CONFIG.toml`。三条都在写之前，拒绝之后磁盘与账本一字不动。
- **程序装没装，不在答应时判**：找程序的权威是 `bin::doctor::host::find_program`，accounting 看不见它，worker 接触宿主只经 `StartHarness`。起不来的 harness 由 `HarnessProcess::start` 在车道里答 `E_TOOL_UNAVAILABLE`，恢复语给出这家自己的 `docs()`；它发生在 `run_started` 之前，账本上只多一条 `worktree_opened`，与模型 run 的准备在车道里失败同形（见本节未决）。
- **车道里的次序**：先借房间的树（与评审楼同一个 `workbench::standing::lend_tree`，不论 `review`），再在那棵树里起 harness 并开会话，然后 `HarnessRun::open` 写开篇两行，最后以房间的 brief 发出唯一一次 prompt：有 `JOB.md` 时是它的原文，没有目标的派活是任务本身（harness 没有前缀可放 `PRINCIPAL_BRIEF`）。
- **`Listener` 的四个回调**：`halted` 读这个 run 的截断：罩住房间的停摆（`driving::lane::scope_stopping`，与模型 run 同一条规则）或人对这个 run 的 `Cancel` 是 `Cut::Halt`，墙钟上限到了是 `Cut::Deadline`，先查墙钟；`cancelling` 调 `HarnessRun::cancel`；`report` 把 `Update` 穷尽映射成 `HarnessReported` 交给 `HarnessRun::report`；`permit` 照 §8-4e 第 9 条选，问与答各记一条 `harness_reported`。记不进账本的问答让这一问答 `cancelled`，失败在回合结束后照会话失败处理。
- **回合结束**：答出停止原因后，在城的那把 checkpoint 闸下把树 `wave_pre` 成检查点，交给 `HarnessRun::conclude`（`checkpoint_committed`、`harness_answered`、冻结）。会话在答出停止原因之前断了（`E_PROVIDER`、`E_WIRE_MISMATCH`），或检查点提交不了，`HarnessRun::abandon` 冻成 `Cancelled`，原错误照模型 run 的做法向上抛：账本得到判决，调用方得到诊断。会话与子进程在驱动返回时一起丢掉，丢掉即杀（`crates/agent_protocols/Spec.lean` §8-19）。
- **落地**（账本线程）：离开 backlog；冻成 `Done` 时城替它开请求：把树落成房间分支上的一个 commit，写 `pr_opened`，进请求簿，与居民自己 `pr open` 的那一臂是同一段代码（`reviewing::offer`），核与合并走已有的评审流程；这条分支已有一份请求在等时不开第二份，只记一行诊断；然后归还树，按 `Owing` 付账。模型 run 落地要放回的适配器、工作台与转录，harness run 一样都没有。
- **墙钟上限**：楼规 `RULES.toml` 的 `harness_minutes`（city-SPEC §8-2），缺省 60 分钟。车道从 `run_started` 那一刻起算，到时按停摆同一条路走：先 `cancel_received`，再 `session/cancel`，读到 harness 答出 `cancelled`，冻成 `Limit`。一个既不说话也不结束的 harness 占着 `DRIVING_LANES` 条车道之一（§8-46-3），上限是它自己让出车道的唯一一条路。
- **缝**：`StartHarness` 的生产值是 `HarnessProcess::start` 在那棵树里起那一家；测试在管道另一头用一条线程扮演 agent，经 `Lines::over` 与 `AcpSession::open` 开会话，与 `agent_protocols` 的会话测试同法，不起真的 harness。用闭包而不立 trait：第二个实现是测试的，只有一个调用方。

**未决**：

- **程序的预查**：要在答应时就拒一个不在搜索路径上的 harness，worker 需要一只找程序的手，生产接线在 `bin::assembly`（`listening`／`production`）里交 `doctor::host::find_program`。要证明的是：`StartHarness` 在车道里的拒绝晚于开房间与写 `JOB.md`，是人能看到的一间空房间。
- **截断的缘由**：`cancel_received` 的载荷是空对象，一次冻成 `Limit` 的取消是墙钟上限，冻成 `Cancelled` 的是停摆或人。要在那一行上直接读出缘由，kernel 的事件表要给它加键，归线协议的车道。
- **steer**：ACP 在一个回合中间没有给 agent 的消息，人发给 harness run 的 steer 被取走后只记一行诊断。可选的答案是回合结束后把它当下一次 prompt 发出，那要一个 run 有第二个回合。
- **分支会话**：从另一段会话分出来的房间，第一次 run 若是 harness，它继承的那段对话没有地方放：ACP 的 `session/new` 不收历史。今天这段对话被丢下，`run_forked` 仍在账本上。
- **进程树**：四家经 `npx`，Windows 上是 `npx.cmd`，丢掉句柄杀的是直接子进程；一家不读 stdin 结束的 harness 会留下 node 进程。证据是起一次真的 `npx.cmd -y pi-acp@0.0.34`、丢掉句柄后看进程表；Windows 上收整棵树要 Job Object，按 AGENTS 的平台调用顺序另开一张卡。
- **城开请求用的门**：`collab::PrDesk` 没有让城放一个 `Opened` 效果的公开面，所以落地调 `reviewing::offer`，不经 desk。collab 的 SPEC 迁到 Lean 之后，要不要给 desk 一扇「城替居民开」的门，由它的 SPEC 定。

## 8-133 一次派活的运行策略在派活路径上走到哪里（`accounting::worker::dispatching`、`accounting::worker::workbench`、`accounting::worker::reviewing`）

kernel-SPEC §8-77 定了运行策略的四个值，本节是它们在派活路径上的接线：从帧到账，到写门，到这次 run 写在哪棵树里，到合并。

```rust
pub(super) struct Assignment { /* …既有字段… */ pub(super) policy: kernel::RunPolicy }
pub(super) struct Knock { /* …既有字段… */ pub(super) policy: kernel::RunPolicy }
impl Site {
    pub(in crate::worker) fn name_tree(&mut self, at: &Assignment) -> Result<(), AxError>;
    pub(in crate::worker) fn place_tree<L: Ledger>(&mut self, at: &Assignment, placing: &Placing<'_>, lines: &mut Stamping<'_, L>) -> Result<(), AxError>;
}
```

- **帧到账**：`Command::Dispatch.policy` 原样进 `Assignment.policy`，经 `RunPlan.run_policy`（harness run 经 `Chartered.policy`）由 `runtime::run::Charter::open` 写进 `run_started.policy`。城自己派的活——计划节点、追的目标、编辑器经 ACP、控制台与 `sprawling dispatch` 这个动词——取 `RunPolicy::of(Mode::Work)`；委派、交回与敲门叫醒继承说话那一方的整份策略，所以一次只读可新建的试验不会因为交给了别的居民就放宽或落地。
- **写门**：`workbench::tools` 把 `policy.write` 交给 `EditTool` 与 `ExecSetup`，楼的写域照旧取自 `RULES.toml`，两道判定在每次写时各问一次（city-SPEC §8-32、runtime-SPEC §8-55）。`policy.mode` 只决定目录里那一行（`Catalog::set_mode`）。
- **试验写在自己的树里**：一次 run 在不在自己的工作树里写，由 `Site::works_apart` 一处回答——`landing` 为 `Experiment` 时恒是，`Ordinary` 时照楼的 `review`。所以没有评审的楼里，试验也借房间的树（与评审楼同一个 `lend_tree`，写 `worktree_opened`），它写的东西不出现在楼里；树与分支留着，人与居民都读得到。
- **合并时的准入**：`reviewing::settle_requests` 在 `PrEffect::Merged` 写 `pr_merged` 之前问 `runtime::admits(&at.policy, produced)`（runtime-SPEC §8-54）：试验恒拒，常规落地按准入证据要求判；拒时写 `pr_rejected`，理由是拒词的两句。没有任何一条路把试验的树自动合并，所以「没有隐式落地」不靠一条额外的检查。
- 验收：`dispatching` 测试 `an_experiment_writes_in_a_tree_of_its_own_in_a_building_without_review`（没有评审的楼里，一次试验新建的文件不出现在楼里，账上有它的 `worktree_opened` 与带 `"landing":"experiment"` 的 `run_started`）。

## 8-5 订阅额度走 harness，不走登录

本城没有登录命令，也不持订阅令牌（gateway-SPEC §8-5）。一个人要用自己的 Codex、Claude Code、Grok Build、Kimi Code 或 Pi 订阅，是在那个 harness 里自己登录，再把它作为 harness 居民接进城；provider 页只收 API key。

- **旧账本照样折得回**：旧版本写过的 `login_started` 仍是一个事件种类，没有读者；`secret_captured` 行里的 `expires_at` 与 `<provider>-subscription`／`<provider>-renewal` 来源读入时照收，本城不再据此续期。
- **旧快照不接**：`StandingFolds` 少了到期表，`STANDING_FOLD_RULES` 随之换值，旧快照按版本不符从创世重折（8-101）。
- **harness 页读名单与这台电脑**：`accounting::views::lines::harnesses_answer` 把 `agent_protocols::Harness::ALL` 逐家抄成 `wire::HarnessLine`，`found` 经 `Views.programs` 问服务中的城交进来的查找（accounting-SPEC.md 8-10）；生产的查找是 `bin::doctor::host::find_program`，与 doctor 找程序读同一条搜索路径（wire-SPEC §8-52）。
- **provider 页先给厂商表**：`views::lines::known_hosts_answer` 把 `gateway::known_hosts` 逐行抄成 `wire::KnownHostsAnswer`，不加不减（wire-SPEC §8-51）。

## 8-6 五个视图不再答 unavailable

`Views` 增四份折叠与一次读盘，`answer` 的 catch-all 臂随之消失——**穷尽 match 是这次改动的验收之一**：此后新增一个 Query 不写答案就编译不过，而不是在运行时答一句 `Unavailable`。

| 查询 | 出处 | 口径 |
|---|---|---|
| `InboxView` | 折 `signal_enqueued` 减 `signal_consumed`：前者经 `collab::Signal::from_payload` 读、按它的 `room` 入队，后者经 `collab::SignalConsumed::from_payload` 读、按 `id` 从每个房间的队列里出队，与 `CollaborationFold` 同一种读法（`signal_consumed` 只写 `id` 与 `by`；行的 `addr` 是取走它的房间，没有什么强制它等于入队时的房间，按 `addr` 出队的视图会在两者不同时让已取走的信号一直等着）；读不回的一行让折叠报错而不是被跳过 | **看队列不靠消费**：`Inbox::pull` 要拿走才给得出内容，一个看一眼就把东西取走的视图会改变它所报告的对象。**与写者同一把尺**：视图自己按键名读这两行时，消费行里没有 `room`，被取走的信号在视图里永远等着 |
| `DiscardView` | 折 `file_discarded`／`discard_restored`，按路径归键；本版本读不回的一行整行跳过，不以空路径或空还原顶替 | 每行自带回去的路（`restoration`）；还原是**关掉它开的那一行**，不是另开一行 |
| `RegistryView` | 折 `asset_archived`；没有房间或本版本读不回的一行整行跳过，`kind` 与 `subject` 不以默认值顶替 | 「这座城认定值得留下的东西」；空表就是空表，与「本版本答不了」在类型上已经不可混淆 |
| `ArchiveSearch` | 被问的那一刻读盘（同 `BuildingView`） | 文件是权威，另存索引就是第二个权威 |
| `Metrics` | 上面几份＋`hot`＋`read_spine` | **恒不携钱**：钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。这里每个数都已被别的视图证明过，它存在只为让画一条读数花一次问答；唯一自有的数是 `events`（本视图折过多少条），因为没有别的答案能推出它 |

## 8-7 ACP 入站

```rust
fn acp_dispatch(desk: &CommandDesk, body: wire::AcpBody, authentic: bool)
    -> Result<wire::AcpProgress, AxError>;          // 外来请求 → 普通 Dispatch
```

- **令牌在门那侧判，判定在协议那侧措辞**：`wire` 持配对令牌，故常数时间比对住 `/acp` 路由；`authentic` 这一位传进来，由 `agent_protocols::admit` 说拒词——未配对者只学到一位，这句话的权威只有一个。
- **入站不是第二个 control surface**：admit 之后就是人按派活条时走的同一条路（同一个 `CommandDesk`、同一个 `Command::Dispatch`）。回给编辑器的只有 progress 三字段，且 run id 是工人接单时才铸的，故此刻诚实的答案是「已受理、尚未完成」。

## 8-8 首次运行与交付形态

**原因**：release 里的 exe 是控制台程序。无参启动只向 stderr 写一行用法并退 2，从资源管理器双击即闪退——没有安装过程，也没有任何成败提示。从 exe 到 WebUI 之间还压着 `init`／`serve`／自行输入地址三步手工操作，而 `serve` 打印的是裸 socket 地址不是 URL。终端、双击、脚本三类到达方式被挤在同一个入口上。

**不猜启动方式**：判断「我是被双击的还是在终端里跑的」，可靠办法是 `GetConsoleProcessList`，需 `unsafe`——workspace lints 恒禁。故以**显式入口**取代探测：三扇门各自命名，背后共用同一段序列。

```rust
// bin::firstrun
pub(crate) enum FirstScreen { Start(PathBuf), Quit }

pub(crate) enum BesideBinary { Writable, ReadOnly }

pub(crate) fn default_city(exe_dir: &Path, home: Option<&Home>, beside: BesideBinary) -> PathBuf;
pub(crate) fn writability(dir: &Path) -> BesideBinary;
pub(crate) fn ask<R: BufRead, W: Write>(city: &Path, input: &mut R, out: &mut W) -> io::Result<FirstScreen>;
pub(crate) fn open_in_browser(url: &str) -> io::Result<()>;
pub(crate) fn local_url(bind: SocketAddr) -> String;
```

- **`up <dir>`＝序列的唯一定义**：目录里没有 ledger 就先 `init`，随后 `serve`，随后开浏览器。无参屏与 `start.cmd` 都落到它，`init`／`serve` 仍各自独立可用——一段序列一处权威。
- **genesis 要人同意**：写 Ledger 第 0 行是全系统唯一一次不可撤销的语义写入，不因「有人双击了一个文件」而发生。无参屏在按键**之前**把最终路径显示出来，人按回车才开城；`q` 退出并打印命令表。
- **非交互 stdin 无此问**：`read_line` 得 EOF（管道、CI、无人值守）即 `Quit`，主流程打印命令表退 2。这条让该路径在没有 TTY 的地方也可测。
- **默认位置取 exe 同级 `city/`**：整座城随文件夹可拷、可备、可删，与「一座城市就是一个目录」同构。`writability` 探到不可写（解压进 Program Files）就回退到 `Home::default_city()`（accounting-SPEC.md 8-7）；回退可见而非暗中，因为路径印在第一屏上。
- **「同级目录可不可写」是二元枚举而不是 bool**（Roadmap G-25）：`writability` 产出 `BesideBinary`，`default_city` 只收它，于是这一个事实在探测端与决定端是同一个拼写，调用点读起来是 `BesideBinary::ReadOnly` 而不是一个无名的 `false`。
- **回退路径只有 `accounting::home` 一处权威**：`~/sprawling/city` 由 `Home::default_city()` 给出，`firstrun` 不再自己拼 `join("sprawling").join("city")`（Roadmap G-10 的第四处）；目录名 `city` 由 `home::CITY_DIR` 一处定义，exe 同级与家目录两种落点共用它。
- **开浏览器恒非致命**：`open_in_browser` 失败只记一行，`serve` 照跑——URL 在这之前已经打印。命名不取 `browser`：`crates/browser` 已占住「Agent 驱动真实浏览器」这个概念，一名一义。
- **横幅给人读**：city 目录、WebUI 的完整 URL、客户端完整与否、`Ctrl-C` 停城，四行。bind 是未指定地址（`0.0.0.0`）时 URL 仍给回环形，因为那才是运行中的机器打得开的那一个。横幅在端口已经绑定、写者已经持锁之后才印（§8-88）。

**交付形态**：`just package` 产 `sprawling-<version>-<target>.zip`＝二进制＋`start.cmd`／`start.sh`＋`QUICKSTART.md`；裸 exe 不再单独作附件，双击的目标因此永远是启动器。`release.yml` 由 tag 触发，三平台各跑 `just dist`，`xtask budget` 在打包前拦下页壳客户端（`CLIENT_COMPLETE=false` 的二进制），通过后才附件。手工上传的产物来历不明，是本次全部症状的链头，这条把它关掉。

**本章测试**：`default_city` 取 `BesideBinary::Writable` 得同级、取 `ReadOnly` 得 `Home::default_city()` 本身（而不是第二次拼出的同一串）；`ask` 空行得 `Start`、`q` 得 `Quit`、EOF 得 `Quit`；第一屏文本在返回前已含最终路径（证明「先示后写」）；`local_url` 对未指定地址给回环形。

## 8-9 让二进制成为一个词

**原因**：解压之后，那个 exe 不在任何搜索路径上。唯一的入口是找到那个文件夹再双击 `start.cmd`——找一个脚本比敲一条命令难，而桌面快捷方式比两者都难。`sprawling` 今天不是一个可以敲出来的词。

```rust
// bin::install（形状 4 adapter；决定纯，落地薄）
// 判定四项只在 Windows 编译：非 Windows 不改搜索路径（见下「非 Windows 拷贝照做」一条），
// 于是它们在别的平台没有调用方，dead_code 在 `-D warnings` 下即是错误。
#[cfg(target_os = "windows")] pub(crate) enum PathEdit { AlreadyPresent, Append(String) }
#[cfg(target_os = "windows")] pub(crate) enum PathRemoval { Absent, Rewrite(String) }

pub(crate) fn program_dir(local_app_data: Option<&Path>, home: Option<&Path>) -> Option<PathBuf>;
pub(crate) fn installed_name() -> String;                    // 恒为 sprawling + EXE_SUFFIX
#[cfg(target_os = "windows")] pub(crate) fn plan_append(current: &str, dir: &str) -> PathEdit;
#[cfg(target_os = "windows")] pub(crate) fn plan_remove(current: &str, dir: &str) -> PathRemoval;
pub(crate) fn install(uninstall: bool) -> Result<Report, AxError>;
```

- **一次安装做两件事，撤销就撤销这两件**：把正在运行的这个二进制拷进用户级程序目录，并把该目录写进用户级搜索路径。`--uninstall` 删掉它拷过去的那个文件、删掉它追加过的那一段，别的一概不碰。**恒不要管理员权限**，因为这两件事都在用户自己的 profile 里。
- **装进去的名字是推导的，不是抄来的**：`installed_name()` 恒给 `sprawling` 加平台后缀，不取当前 exe 的文件名。归档里的文件被改过名字，敲出来的那个词也仍然是 `sprawling`——否则「让它成为一个词」这件事取决于谁解压的。
- **搜索路径的判定住 Rust，落地住 PowerShell**：`plan_append`／`plan_remove` 是两个纯函数，输入是那条字符串本身，输出是穷尽枚举。适配器只负责取回原值、写回新值、广播。**幂等因此是一条可单测的判定**，而不是一次要在真注册表上观察的行为。
- **判定的编译面等于它的调用面**：这四项与 `PathOutcome::Rewritten` 原本无条件编译，而只有 Windows 那一支调用它们，故 macOS 的 clippy 以五条 `dead_code` 报错——而推送门只跑 Windows。修法是让平台条件跟着调用方走（`#[cfg(target_os = "windows")]`），而不是加一条 `allow`：**平台不同不是要压制的告警，是要写进类型里的事实**。`PathOutcome::Rewritten` 是枚举变体、两平台共用同一枚举，故取同文件已有的先例——`#[cfg_attr(not(target_os = "windows"), expect(dead_code, …))]`，与 `SelfService` 对称。
- **Windows 必须直接改注册表，且必须保住值类型**。`[Environment]::SetEnvironmentVariable(..., 'User')` 是所有教程里的写法，也是错的：它**恒写 REG_SZ**，把 `HKCU\Environment\Path` 的 `REG_EXPAND_SZ` 降级，其中的 `%VAR%` 从此不再展开（dotnet/runtime#1442、chocolatey/choco#699）。实测该值确为 `ExpandString`，故适配器读原值时用 `DoNotExpandEnvironmentNames`、写回时用读到的那个 `RegistryValueKind`——**读到什么类型就写回什么类型**，键不存在时才取 `ExpandString`（Path 在 Windows 上的默认类型）。
- **值经临时文件进出，不经命令行**：用户名含非 ASCII 字符时，命令行要穿过控制台代码页（例如 936），而 `PATH` 的整条值也可能逼近命令行长度上限。故 Rust 与 PowerShell 之间用一个 UTF-8 临时文件传值，文件路径经环境变量交接，两侧都不需要引号规则。
- **改完必须广播 `WM_SETTINGCHANGE`，否则新窗口也读不到**：Explorer 缓存环境块，从它启动的新控制台继承的是缓存。`#![forbid(unsafe_code)]` 关掉了在 Rust 里调 `SendMessageTimeout` 这条路，故广播由 PowerShell 的 `Add-Type` P/Invoke 完成（`HWND_BROADCAST=0xffff`、`WM_SETTINGCHANGE=0x1A`、`SMTO_ABORTIFHUNG=2`、5 秒上限）。实测一次约 1.1 秒。**广播失败不致命**：路径已经写下了，报一行提示说「注销后生效」，而不是把已经成功的一半说成失败。
- **非 Windows 拷贝照做，改 shell rc 不做**：装进 `~/.local` 下的 `bin`（该目录在现代发行版上默认已在 PATH 上）。**不写 shell rc**，理由记在这里而不是留一个静默的空分支：rc 文件有 bash／zsh／fish 三套语法与 `.profile`／`.bashrc`／`.zshrc` 多个候选，选错就是往人的登录脚本里写一行没有作用却要人自己删的东西；而从 Windows 交叉编译到 Linux 已知走不通（`aws-lc-sys` 需 C 交叉工具链），故这一支只能由 CI 编译与 lint，不能由我运行验收——**该 job 是 `platforms.yml` 的 macOS job，不再是 ubuntu**（ubuntu 已被裁出流水线）。**没有跑过的写入动作不写**。目录不在 PATH 上时，报告里给出该加的那一行，人自己贴。

  **Linux 进流水线**：上段论证的是「从无 Linux 的开发机器**交叉编译**走不通（`aws-lc-sys` 需 C 交叉工具链）」，不是「Linux 构建不成立」；GitHub 的原生 runner 在 Linux 上原生构建，根本不碰交叉工具链。落法两件，分开决策：①`release.yml` 矩阵加 `x86_64-unknown-linux-musl` **静态**一行——NixOS 没有 `/lib64/ld-linux-x86-64.so.2`，一份动态链接的 ubuntu 产物在 NixOS 上起不来，而静态一份同时覆盖 NixOS／Alpine／老发行版／容器。**待探项已探明，后端不换**：`aws-lc-sys` 由 `reqwest → hyper-rustls → rustls → aws-lc-rs` 引入（`cargo tree -i aws-lc-sys`，实测），而 aws-lc-sys 0.44.0 的 crate 源码内自带 `src/x86_64_unknown_linux_musl_crypto.rs` 且 README 的 Pregenerated Bindings 表列出该三元组——**该目标在预生成绑定名单上**，故不需要 bindgen，rustls 后端保持 `aws-lc-rs`，不切 `ring`（少一个 crypto 后端就少一处与 Windows／macOS 产物不同的实现）。它仍需一套 musl 的 C 工具链编译 AWS-LC 源码，故该 job 装 `musl-tools` 并令 `CC_x86_64_unknown_linux_musl=musl-gcc`；cmake 已在 runner 镜像上。**这一条只在 CI 上成立**（Windows 开发机上没有 Linux，也没有 musl 工具链），证据是依赖树与 crate 自带文件，不是一次绿色构建。**那次构建已经发生，两半各自有了答案。** aws-lc 那一半站住：`aws-lc-sys 0.44.0` 与 `aws-lc-rs 1.18.0` 在 musl 上开编且未报错，预生成绑定与 `musl-tools` 这套安排没有被证伪。构建停在另一处，而这一处上段根本没有论到：`keyring` 的 Linux 后端是 secret-service，树因此另到 `libdbus-sys`，它的 build script 跨目标边界问 pkg-config，而 pkg-config 默认拒答跨编译查询。**装 `libdbus-1-dev` 不是解法**：那会把宿主的 glibc D-Bus 递给一次静态 musl 链接，那是一个矛盾而不是一项配置——**一份静态 Linux 二进制与一个 D-Bus 凭据库不能同时为真**。故**那个先于构建的问题已经有答案，该行随之进矩阵。** 问题是：**Linux 装上之后，一把 API key 存在哪里**。答：内核自带的 keyring——凭证库的 Linux store 取 keyutils（`linux-keyutils-keyring-store`，gateway-SPEC §8-4），那是一组 syscall，不需要会话总线、不需要动态库，静态 musl 与容器里同样成立，`libdbus-sys` 随之离树（实测：`Cargo.lock` 删 `dbus`／`dbus-secret-service`／`libdbus-sys`，增 `linux-keyutils`）。代价写在类型上：`KeyringVault::PERSISTENCE` 在 Linux 上恒为 `Persistence::ThisBoot`，`Persistence::consequence()` 是那句话的唯一权威，`resolve` 未命中时把它接在 recovery 后面，故重启吃掉的 key 自己会说话而不是静默失败（gateway-SPEC §8-4）。仍欠的只剩一项：**真机验收必须在 Linux 上跑**，开发机器是 Windows，故 keyutils 的写—读—删探针只能到 CI 或一台 Linux 上才算数。该行除此之外所需的都已建成并保留：`just package` 收三元组、归档名带三元组、`install.sh` 认得那个名字。**那一处债已清**：`binary_path` 认了三元组并迁进 `xtask::package`，`just package <triple>` 一条配方打三行矩阵，`release.yml` 里重抄的步骤随之删除，musl 归档的名字带三元组（xtask-SPEC.md §8-11）；②`flake.nix` 只管 devshell 与 `nix run`，版本从 `rust-toolchain.toml` **派生而不复述**（需要额外版本即红），不接管 Windows／macOS 发布路径，CI 上进 `platforms.yml` 的 nightly 而非 push 流水线。
- **`Report` 说的是已经发生的事**：拷到哪、搜索路径改没改（`AlreadyPresent` 与 `Append` 是两句不同的话）、广播成不成、以及「PATH 变更不会进已经开着的窗口」。**恒不说「安装成功」四个字**——人要知道的是下一步该开一个新窗口。

**本章测试**：`program_dir` 在两个平台各取本平台约定；`plan_append` 对空串、已含该目录（含大小写不同与带尾分隔符两形）、含其它目录三类输入分别给出正确的穷尽枚举；`plan_remove` 删得干净且保住其余段（含空段）；`plan_append` 之后 `plan_remove` 回到原值——**幂等与可逆是一对性质测试，不是一次手工观察**。判定既然只在 Windows 编译，这组测试也只在 Windows 编译：**测一个在本平台不存在的函数，测的是空**；推送门跑的正是 Windows，故这组测试每次推送都跑。

**本章验收（必须真做）**：`install` 之后**开一个新的 PowerShell 窗口**敲 `sprawling`；随后 `--uninstall`，再开新窗口确认 `Get-Command sprawling` 为空。

## 8-10 第二个 wire 客户端

**为什么存在**：ARCHITECTURE §8 写着「the wire is the whole API；一个第二客户端就照着它写」，而今天只有一个客户端——按仓库自己的依据（§4：一个适配器是假想缝，两个才成立），`wire::frames` 因此是一条假想缝。

```rust
// bin::wire_client（形状 4 adapter）
pub(crate) struct Listen { pub quiet: Duration, pub until: Until }  // 何时停止收听，两者同行
pub(crate) enum Until { Quiet, Event(kernel::EventKind), Run { under: Address, milestone: Milestone } }  // 调用方在等什么
pub(crate) enum Milestone { Started, Frozen }                         // 由 run_started / run_frozen 标记
pub(crate) struct Heard { pub frames: u32, pub refusals: u32, pub answers: u32, pub awaited: Awaited, pub run: Option<RunId> }
pub(crate) enum Awaited { Nothing, Arrived, Missing }
pub(crate) enum Spoken { Refused, Answered, Quiet, Unfinished }
pub(crate) fn call(at: &str, frame: &str, token: Option<&str>, listen: Listen) -> Result<Heard, Unheard>;
pub(crate) fn send(at: &str, outgoing: &wire::ClientFrame, token: Option<&str>, listen: Listen) -> Result<Heard, Unheard>;
pub(crate) fn enrol(at: &str, realm: &str, name: &str, value: &str) -> Result<String, AxError>;
pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)>;   // "realm/name"
```

- **握手在进程内算，不手抄**。`WIRE_V` 与 `schema_hash()` 直接取自 `wire`，故改一条命令名字时本客户端**不可能**落后。因此删掉了那个一次性的 Python 探针——它在工作区外复刻了 `schema_hash()` 与 `IdemKey::derive()`，那本身就是第二个权威。
- **一个查询恰好一个答复，收到就走**。发出的是 `Ask` 时，`call` 在收到第一帧 `Answered`（其 `AskOutcome` 是答复或拒绝）或 `Refusal` 时打印它并退出，之前推来的 `Event`／`Log`／`Delta` 照样逐行打印；城的答复在十几毫秒内到达，再等一整段安静窗口只是让进程白占两秒。安静窗口在这里只剩上限的作用：答复迟迟不来时，`call` 退 3——什么都没回来是 `Quiet`，只回来了别的帧是 `Unfinished`。
- **一条命令收到城安静为止，或收到调用方点名的那种事件为止**。发出的是 `Command` 时，“安静”是一段无帧的时长（`--quiet-ms`，默认 2000），而不是帧数：一条 Dispatch 会产生多少事件是城的事，客户端猜不到。调用方知道自己在等哪件事时，写 `--until <kind>`（`kind` 取 `EventKind` 的 snake_case 拼写，由 serde 读，不另立名表）：`call` 在打印第一条该种类的 `Event` 或一条 `Refusal`（被拒的命令不会再产生事件）后退出，不再白等一整段安静窗口。窗口此时仍是两帧之间的上限；窗口先到而点名的事件没来，`Heard.awaited` 为 `Missing`，`spoken()` 给 `Unfinished`，退出码是 3 而不是 0——在等的事没发生，读成成功就是把失败读成成功。查询只有一个答复，`--until` 对查询不起作用。
- **何时结束由 `wire_client::Ending` 一处决定**。`Ending::of` 按发出帧的种类与 `Until` 穷尽匹配，得 `Reply`（查询与握手）、`Quiet`、`Event(kind)`、`Run { under, milestone, run }` 四者之一；每一帧由 `Reply::of` 经 `wire::ServerFrame` 读一次，`Reply::Event { kind, run, addr }` 同时供 `--until` 比种类、供 dispatch 认出自己的 run。在等的帧到没到只记在 `Heard.awaited` 一处，`call` 与 `dispatch` 读的是同一个 `spoken()`。被否决的备选：把「收到答复就走」做成一个布尔参数——它会让 `call(…, false)` 这样的调用点说不出自己在等什么；以及让 dispatch 另记一份「里程碑到没到」——它与 `Awaited` 答的是同一个问题，两份记录迟早说两样话。
- **`sprawling dispatch <addr> <task> [--detach] [-m/--model <id>] [--at] [--token]`（`bin::main::dispatch`，形状 adapter）是一次派活的整条路**：进程内铸幂等键（`IdemKey::derive(RunId::CITY, Seq::FIRST, 16 字节 OS 熵)`，与控制台每行一把键同一个构造；键从命令行拿进来就等于让人或 agent 再抄一遍 wire 的键格式），模式取 `Mode::PlanGoal`、强度与目标留空、`session` 为 `None`（地址是楼时城向模型要房间名，是房间时续写那个会话）。它等的是 `Until::Run { under, milestone }`，`Ending::of` 把它变成 `Ending::Run { under, milestone, run }`：`under` 是派去的地址，第一条 `addr` 等于它或在它之下的 `run_started` 认定这次的 run，`milestone = Milestone::Frozen` 时等到**那个** run 的 `run_frozen`（别的 run 冻结不算），`milestone = Milestone::Started`（`--detach`）时收到 `run_started` 就走；两者收到 `Refusal` 都立即结束，安静窗口（默认 120000 ms：一次模型调用返回前不出帧，派到楼时城先调一次模型给房间起名，run 才开始；每来一帧都重新计时）只作上限，窗口在里程碑之前关上时 `Heard.awaited` 为 `Missing`，城说过话则 `spoken()` 给 `Spoken::Unfinished`，一帧没回则给 `Quiet`，都退 3 而不是 0。`--detach` 的 stdout 只有 run id 一行，逐帧 JSONL 改走 stderr，于是 `id=$(sprawling dispatch --detach …)` 可直接用；不带 `--detach` 时逐帧 JSONL 走 stdout，与 `call` 一致。退出码表与 `call` 同一张，映射只写在 `main::calling` 的 `exit_of`（`Spoken` → `Exit`）与 `tell_unheard`（`Unheard` → `Exit`）两处，两个动词都调它们：0 答复（里程碑已到）、1 拒绝、2 命令行、3 安静或未到里程碑、4 `--at` 没有城。被否决的备选：让 `dispatch` 拼一段 JSON 再交给 `call`——`call --until run_frozen` 等的是任何一个 run 的冻结，认不出这一次派出的那个 run，而不带 `--until` 只能等安静；一次 run 的长短是模型的事，安静窗口要么截断它要么让每次派活白等。
- **`-m/--model <id>` 点名这一次的模型**：它原样进 `WireCommand::Dispatch.model`（wire-SPEC §8-48），不带时为 `None`：房间自己那层已冻结的模型仍有 tag 登记时取那个 tag，否则取 `main` tag 的模型——冻结的模型已无 tag 登记时，`main` 的模型由 `choose_shape` 以换模型拒掉（§8-79），拒绝语点名动过的那一项，恢复语给出人现在做得到的出路；在这里以「无 tag 登记」拒，两者都说不出。继任交接、唤醒敲门和不带 `-m` 的后续派活都带 `None`，若 `None` 一律取 `main`，在用 `-m` 开的房间里它们会被 `choose_shape` 以换模型拒掉，接力就断在半路。装配在 `agree_to_work` 里按这个 id 在 `ModelBook::choices()` 找到登记它的 tag，同一个 id 登记在几个 tag 下时取 `ModelTag` 次序里最前的那个，`main` 最前——`choices()` 按 `BTreeMap<ModelTag, _>` 的键序给出，所以这个取法不随登记先后变，再经 `ModelBook::select` 取端点与登记行，所以保密楼只用回环端点的检查、端点是否仍挂着、订阅凭证续期都与 `main` 同一条路；找不到时以 `E_CONFIG_INVALID` 拒，恢复语是「在设置页把它登记到一个 tag 下再派」，拒在写任何东西之前。选中的 id 由 `choose_shape` 冻进房间自己那层 `CONFIG.toml`，与 tag 解析出的模型同一扇门，所以一个已冻结的会话照旧拒绝换模型。被否决的备选：用 `SelectModel` 先改 tag 再派——那是整座城的配置，会在同时跑着的别人的 run 底下换模型。
- **输出是 JSONL，一行一帧**。发明一种人看的排版就是为 wire 里的每一个类型再写一遍它长什么样，而那份渲染一定会漂。
- **退出码带信息**：收到过 `Refusal` 退 1；命令之后一帧都没回来（`Quiet`），或回来了帧而在等的那一帧没来（`Unfinished`），退 3；否则退 0。`Quiet` 与 `Unfinished` 分成两支，是因为前者连城是否收到这一帧都无从断言，后者城在说话、只是事情还没做到。一个驱动它的 agent 不应当为了知道「成不成」去解析 JSON。
- **`enrol` 只从 stdin 读，恒不从 argv 读**。argv 进进程表、进 shell 历史、进父进程的日志；这比浏览器路径更好的地方就在这里，因为页面那条路要先把明文拿进一个标签页的内存。**输出只有引用**，恒不回显值。
- **依赖不新增包**：`tokio-tungstenite` 正是 axum 的 `ws` 特性已经携带的那一份，直接依赖它在 `Cargo.lock` 里**增加零个包**（实测 496 → 496）；换一个别的 WebSocket 库就是把同一个协议的两份实现放进同一个二进制。不开 TLS：控制面走 `ws://`，而一座要经 TLS 到达的城是一座前面站着终结器的城。
- **未做且已知**：`/enroll` 仍在工人取走凭据之前就答 201（详 `wire-SPEC.md` §8）。`enrol` 因此报的是「已受理」而不是「已入库」，这句话写在输出里而不是留给人去撞。

**本章测试**：`split_reference` 对 `realm/name`、缺斜杠、空段、多斜杠四类输入给出正确答案；握手帧的 `wire_v` 与 `schema` 逐字节等于 `wire` 自己的值（这条断言就是「不存在第二份握手权威」的可执行形式）。`wire_client::tests` 用一个替身城证明：查询在窗口到期之前随答复返回；带 `--until` 的命令在窗口到期之前随点名的事件返回，之前的事件照样打印；点名的事件没来而别的事件来了，结果是 `Unfinished` 而不是已答复。`wire_client::ending::tests` 证明：派到楼的等待认出在它房间里开始的 run，别的 run 冻结不结束它；`--detach` 在 run 开始时结束并带回 run id；窗口在里程碑之前关上是 `Unfinished`。真城验收：`call` 一条必被拒的命令，收到 `refusal` 且退 1。

## 8-11 控制台：服务中的那个终端不再是死胡同

**已有工作区的人**：`form_city(root, Adopt)` 取代 `init_city` 成为唯一的成城路径（后者是 `Adopt::Nothing` 的别名）。`Adopt` 是穷尽枚举而不是布尔：在既有工作旁边形成一座城、与把那些工作放到规则之下，是两件事，一个布尔会把它们说成一件事的一个设置。采纳走的是 `sprawling adopt` 的同一道门，于是创世时收进来的文件夹与一个月后收进来的受同一套规则治理。首屏因此长出第三个答案 `FirstScreen::Use(path)`——回车之外、`q` 之外的任何输入都是一个路径（去掉文件管理器加的引号）；**路径存不存在由调用方查并报**，屏幕自己去猜要么把真文件夹当错字拒了，要么在没人看过的位置造一座城。

**原因**：`sprawling up` 打四行字然后阻塞到 Ctrl-C。那块屏幕是产品白白扔掉的一个面，也是一台没有浏览器的机器**仅有的**那一个面。

**写不出去的一行归一处**（G-23）：`console::terminal::say` 是这个文件里唯一决定「控制台写失败怎么办」的地方——写不进去的控制台是没人在读的控制台，而城不归控制台停，故失败止于此。十三处 `let _ = writeln!(out, …)` 因此不再各自决定一次。同章的两个布尔入参改为枚举（G-25）：`serve_city` 的 `open: bool` 成 `Open::{Browser, Nothing}`，`install::install` 的 `uninstall: bool` 成 `Direction::{Install, Uninstall}`——`install(true)` 在调用点说不出它做了什么。

```rust
// bin::console（形状 1 decision；壳在一条线程里，判定全在纯函数）
pub(crate) enum Line {
    Nothing,
    Help,
    OpenWeb,
    Select(Address),
    Quit,
    Frame(Box<wire::ClientFrame>),   // 一个 wire 动词
    Work(String),                        // 普通一行：派给当前选中的 room
    Unknown { verb: String, nearest: Vec<String> },
    Malformed { verb: String, reason: String },  // 动词认得，JSON 体读不出：reason 是 serde 的原话，写出缺的字段名
}
pub(crate) fn parse(line: &str, selected: Option<&Address>, idem: IdemKey) -> Line;
struct LineKeys { origin: [u8; 16], lines: Seq }   // 每行一把键
impl LineKeys { fn drawn() -> Result<LineKeys, AxError>; fn next(&mut self) -> IdemKey; }
pub(crate) fn verbs() -> Vec<String>;      // 控制动词 ⊕ wire 动词
pub(crate) fn snake(camel: &str) -> String;
```

- **wire 动词表是投影，不是第二份手写清单**。`verbs()` 从 `wire::COMMAND_NAMES` 与 `QUERY_NAMES` 逐个转 snake_case 得来；一份手写清单就是第二套词汇，而它漂开时没有任何东西会发出声音。一条断言钉住这件事：每个 wire 名字都在动词表里。
- **控制动词另成一个穷尽枚举**（`/help`、`/web`、`/at`、`/quit`）。它们是**控制台自己的**动词，不在 wire 上，故不属于那张投影。两张表合并后仍不得重名，一条断言钉住。
- **控制台不做任何判定**。一行变成 `Command` 之后，走的是人在页面上点按钮走的**同一张桌子**（`CommandDesk`）与同一个 `Reply`。拒绝因此自动回到控制台，不需要为它另写一条回程——这正是那条回信地址的第二个消费者。
- **每一行一把幂等键，由控制台铸**。`LineKeys::drawn` 在控制台启动时取 16 字节 OS 熵作 `origin`，`next` 按行计数：键 = `IdemKey::derive(RunId::CITY, 行号, origin)`。城把见过的键连同第一次的答复一起记住，且跨重启记住；故键只由 `地址+任务` 派生时，同一房间同一句话第二次被吞，先被拒（例如还没配模型）、配好模型后再打同一行仍拿到那次拒绝。行号使同一进程内的两行不同，`origin` 使两次启动的同一行号不同。熵取不到时控制台说出原因并关闭，城照跑——一把可预测的键会被上一个进程的答复吞掉。**被否掉的做法**：把时间放进键——时间只在 `bin::assembly` 取样，且同一毫秒内两行仍撞。
- **wire 动词的 JSON 体缺 `idem` 时由控制台补上这一行的键**；人在没有浏览器的机器上刹住整座城只需 `/halt {"scope":"city"}`。体读不出时返回 `Malformed`，打印 serde 说出的原因（含缺的字段名），而不是「没有这个动词」。
- **投影只取 socket 能带的 Command**：`put_secret`（`WireCommand` 里没有这个值）与 `auth`（配对令牌在握手里证明，不在命令里）是 `Command::idem()` 返回 `None` 的恰好那两个，控制台既不列它们也不认它们。
- **`/quit` 只关控制台，城继续服务**；帮助说的是同一句话。停城是 Ctrl-C，那是只有起城的进程按得动的收口（下文）。
- **普通一行就是派活**。要人为一件活敲 `/dispatch {"addr":…}` 是把 JSON 当人机界面；选中一个 room（`/at`）后直接写任务，才是终端本来的手势。未选中任何 room 时拒，并说该敲什么。
- **不是 TTY 就不进控制台**。stdin 读到 EOF（管道、服务、CI）即退出控制台循环而**城照跑**：一座因为没人敲键盘而停止服务的城是一个以交互换服务的回归。
- **拒长表与图**。查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形；表格与图归浏览器。一个同时伺候两个主人的 CLI 是 CLI 文献里的反面教材。`sprawling view` 是另一个动词、另一个进程，按 stdout 是不是终端把两个主人分开（§8-105、§8-117）。
- **`/web` 携配对令牌**，故没有人需要手拷一串东西。令牌在 `serve` 里只被读一次，控制台拿到的是那一次的副本，不重新读环境。
- **Ctrl-C 是有序收口**：`serve` 在 `wire::serve` 与 `tokio::signal::ctrl_c` 之间 `select!`。收到信号后先停止接受连接，再 `CommandDesk::close(Closing::Chosen)` 告诉 worker，worker **在读队列的同一处**读到它，于是正在跑的那条命令先跑完，`handoff_written` 是最后一行而不是某一行的中间。主线程 join worker 线程再返回——先返回的 main 会在那一行写出来之前结束进程。
  - **`DeskWait::Close` 与 `Gone` 不是一回事**：前者是城要停了，值一份 Handoff；后者是桌子自己坏了，那座城已经写不出 Handoff 了。
  - **收口带着它的缘由**：`CommandDesk::close(Closing)`，`Closing` 是穷尽枚举（`accounting::worker::lifetime`）：`Chosen`——人按了 Ctrl-C；`Broken { cause }`——`wire::serve` 返回了错误。serve 的结果只有一处读法：`Closing::of(&served)`，`Ok` 读作 `Chosen`，`Err` 读作 `Broken`。信号处理器装不上不是 `Broken`：`serve` 在标准错误上说一句，然后继续等 `wire::serve`，城照常服务，只是 Ctrl-C 回到不写 handoff 的硬停；装不上信号处理器只拿走了有序收口，没有坏掉服务，为它关掉一座正常服务的城是把小故障放大成停城。`serve` 只从 `select!` 的结果里判这一次：`Ok` 即 `Chosen`，`Err` 即 `Broken`，错误的文字就是 `cause`。`close_city(&Closing)` 按缘由写 handoff：只有 `Chosen` 写「the city was closed by the person running it」；`Broken` 写「the city stopped because serving failed」并带上 `cause`，下一步是先修 `cause` 点名的东西。**原因**：没有人做过的事不能记成人做的；一份把失败写成人主动关城的交接件，会让下一任以为什么都没坏。**否决的方案**：失败时不写 handoff——那样失败与崩溃在记录里又成了同一种沉默，而 worker 此时仍然写得出这一行。
  - **收口不是一条 Command**：能被拼出来的线上帧就是陌生人停掉别人城市的一条路。`closing` 是台子上的一个 `OnceLock<Closing>`，只有起城的那个进程按得动；先到的缘由作数，第二次 `close` 不改写它。
  - **Windows 交两个信号，本城两个都收**：控制台会发 Ctrl-C 与 Ctrl-Break。一座在其中一个上有序收口、在另一个上暴死的城，等于同一个手势有两种行为，而决定用哪一种的是人碰巧按了哪个键。其他平台只有一个。
  - **代价**：根 `Cargo.toml` 给 tokio 开 `signal` feature。unix 上它引入 `signal-hook-registry`（Apache-2.0/MIT，deny 表内）。

**本章测试**：每个 `COMMAND_NAMES`／`QUERY_NAMES` 都在 `verbs()` 里（这就是「投影而非第二份清单」的可执行形式）；控制动词与 wire 动词不重名；`snake` 对 `AttachEndpoint`／`RunView` 给出预期串；空行、`/quit`、`/at <addr>`、普通文本（选中与未选中两情形）、未知动词（携最接近的几个）各得正确枚举。

## 8-12 prefix 自己带上它要求模型读的东西

**原因**（把四个段拼出来才看得见）：Building 段是 12 字节的地址，Run 段是 71 字节的 `cas:b3-…` 内容哈希。而 City.md 要求模型「read `RULES.toml`」「`FULL READ:` 给出你的 `JOB.md` 的路径」——**两句话指的东西一个都不在 prompt 里，而城里八个工具没有一个解析 `cas:`**。第三处：City.md 无条件说「你的第一条消息正好有三行」，这对没有 `JOB.md` 的主 Agent 是假的。

```rust
// bin::assembly（形状 4 适配器；四个段的填充点）
fn building_segment(city_root: &Path, addr: &Address, building: &Address) -> Vec<u8>;
// Building 段 = 地址 + `RULES.toml` + （若在）`<city>/AGENTS.md` + （若在）
// `<building>/AGENTS.md`。后两者是**项目自带的约定**，按同一句理由直接给而不让居民去取：
// 要先 fetch 才能遵守的规则，要么晚一回合遵守，要么不遵守。**只看两个固定地址**：城根
// 与楼根。城根是这座城围起来的那个工作区，它要么就是项目本身（楼是项目的一级子目录），
// 要么是装着几个项目的父目录（楼就是那几个项目），所以项目的约定只可能在这两层；
// 不向城根之上找、不认其它拼法、不做搜索——一个读者无法从规则推出来的地址，没人核得了。
// 次序由泛到专：工作区那份在前，楼那份在后，最后读到的是离活最近的那份。两份都排在
// `RULES.toml` **之后**并自述位次：二者会冲突（一份叫你跑测试的 AGENTS.md 遇上一栋
// 没有 `exec` 的楼），而城执行的是楼规。文件不在就不写标题：空标题会让居民去
// 遵守一份不存在的约定。
fn run_segment(city_root: &Path, building: &Address, brief: &city::RunBrief) -> Vec<u8>;
```

| 槽 | 装什么 | 稳定性依据 |
|---|---|---|
| City | 城里那份 `City.md` | 整座城不变 |
| Building | 地址 ＋ `.sprawling/RULES.toml` 全文 | 人写、任何写域够不到、整个 Run 不变 |
| Resident | `URBANITE.md`（或无身份那 106 字节）＋ catalog | 同一个 Resident 每次 Run 同样的字节 |
| Run | `Handoff.md`（写过的话）＋ 本次 brief | 每次 Run 一份 |

- **注入的依据是稳定性，不是重要性**。`Roadmap.md` 与 `Memo.md` **故意不注入**：Run 自己会改它们（`plan` 工具持有 roadmap 全文），冻进 prefix 就是第二个权威——模型会读到自己刚改过的旧副本。它们的正路是工具，不是 prefix。
- **Run 段的次序是「上一场留下的」在前、「这一次要做的」在后**：最后读到的东西是被执行的东西。
- **空白表单不占 prefix 字节**：`city::handoff` 认出还是模板原样的 `Handoff.md` 并答 `None`。依据是模板自己的括号提示行——写过的会话会把它们换掉。
- **内容哈希整个退出 prompt**。它在 Ledger 里记了两遍（`run.rs:126` 的 pin 与 `:141` 的 started），溯源不依赖模型看见它；`FULL READ:` 那一行随之消失。
- **CAS 的 pin 改钉 brief 的正文**，两条臂都钉：一次没人派任务的会话，pin 里是「说明没有人派」的那几句，于是 Ledger 的 `job` locator 恒解析得到 Run 段真正携带过的字节，而不是一个从未被写出的文件。

**本章测试**：一次真派活后，provider 收到的请求里含楼规原文（`confidential = false`）、含上一场的 Handoff 正文、含本次 Goal，且**不含** `FULL READ` 与 `cas:b3-`；一次无 Goal 的派活不写 `JOB.md`，请求里说出「working with the person directly」且不把人那句话包成 `Task:` 表单。

## 8-13 一封信与一次敲门

**原因**：两位居民在同一栋楼里谈价，发信的那一跑连续五次 `signal pull` 等一封**在它自己那一跑里物理上不可能到达**的回信，最后以 `limit` 冻结；而收信人根本没有在跑。证据：同一条链上 `signal_enqueued` 落在 `run_frozen` 之后两行。

```rust
// bin::assembly
struct Knock { addr: Address, from: String, mode: kernel::Mode, chain: KnockChain }
impl RunWorker {
    fn knock(&mut self, signal: &Signal, speaker: &Address, mode, chain: &KnockChain) -> Result<(), AxError>;
    fn answer_knocks(&mut self);   // 成波排干，循环而非递归
}
```

**两种送达，分法是收信人在不在**：

| 收信人的状态 | 机制 | 落点 |
|---|---|---|
| 正在跑 | 信从门缝塑进去——steer 型 Signal，`SignalDesk::take_steer` 在安全点取走 | 追在下一次工具结果末尾，前缀 `@发件人地址` |
| 没在跑 | 敲门——投递后入 `knocks`，这次派活结束后 `answer_knocks` 为他开一跑 | 新 Run 的 brief，同样写明 `@发件人地址` |

- **人压过居民**：中断源先问人的命令队列（Cancel 再 Steer），空手才问本屋信箱。
- **属名不是装饰，是回信地址**：另一个 agent 的话氒不得以人的身份进窗口。类型已经把它变成判定（只有 `Steer::from_person` 写得出 `user`）；同一条规则延到敲门路上——被叫醒的一跑，其 brief 第一句就是「@X signalled you. This run exists because that signal arrived: nobody else asked for it.」。一份读起来像人写的 brief 会让每一封回信寄错地方。
- **敲门敲的是 Resident，不是一段已封存的对话**：冻结的 Run 是历史，历史只读而不叫醒；被开出来的是那个地址上住户的**一跑新的 Run**，它靠 `Handoff.md` 接住上一场——那正是为穿过一次冻结而造的那件东西。没有 `URBANITE.md` 的地址因此不敲：它是一间房而不是一个人，信就在那儿等到人派个住户过去。
- **叫醒有一个上限，但不是一份预算**：一条敲门链与一条继任链各有一个接力上限，住
  `accounting::worker::driving::owing`（`CONVERSATION_HOPS_MAX` / `SUCCESSION_HOPS_MAX`，§8-46-12）。
  上限管的是「没有人在里面的链条不许无限长」，不给一轮活定价，也不规定居民之间能谈多少轮；
  花多少仍事后从 Ledger 报出来，停一片仍是 `Halt`。§8-13 早先「不设叫醒预算」的口径
  以「刹车就在被链条堵住的线程上」为前提，H-04 之后这个前提不再成立。
- **一次对话一道底都没有**：**这座城没有金额上限，也没有回合上限**，那是决定而不是遗漏：什么时候停下来归对话里的居民，花了多少事后从 Ledger 报出来。从无调用方的 spend 门连同它判的 ladder 、以及 `DISPATCH_TURN_BUDGET` 一并删除（kernel-SPEC §11-7、本文 §8-40），刹车此后只剩 `Cancel`（停一件）、`Halt`（停一片），以及两条接力链各自的上限（§8-46-12）。
- **一个敲不成不连坐发件人**：叫不醒的人进诊断日志，不把发件那一跑的 dispatch 弄成失败。

**本章测试**：一位居民向另一位发信，无人再派活而收信人自己跑了一跑，且其 brief 里带着发件人的地址；向一个无 `URBANITE.md` 的房间发信不开任何 Run，信仍在队里。

## 8-14 幂等键里的那个时钟

**原因**：同一跑里两次 `read` 被拒为 `this call was already made`，下一回合同一路径又读得干净。原因在一行里：`IdemKey::derive(&run_id, Seq::new(t.value()), call.name.as_bytes())`——

一、**它取了一个时钟**（回合的毫秒戳），而确定性第七条写着「IdemKey 恒不得源于时钟或随机数」；二、**它不含参数**，于是同一回合内对同一件工具的任两次调用归为一键——两次 `edit` 也会，而那是丢写。

现形：`(run_id, 本跑内的调用序号, 工具名＋参数 JSON)`。序号由闭包自己的计数器给，重放同一段历史得同一串键。两次不同的调用是两个键，都跑；同一个位置被重放是同一个键，去重正是为此而存在。

## 8-15 装配层长出一扇门

```rust
// crates/sprawling/src/lib.rs —— 索引文件，只准声明（modmap 已看守）
pub mod assembly;
pub mod console;
pub mod firstrun;

// assembly：跨出 crate 的项，逐个放行
pub struct InitReport { pub ledger_dir, pub genesis, pub standing, pub adopted }
pub enum Adopt { Nothing, EveryFolder }
pub enum History { Absent, Present }   // 目录不存在或为空是 Absent；读不了是 Err，不是 Absent
pub fn has_history(&Path) -> Result<History, AxError>;   // StorageFatal：账本目录存在却列不出来
pub fn init_city(&Path) -> Result<InitReport, AxError>;
pub fn form_city(&Path, Adopt) -> Result<InitReport, AxError>;
pub fn open_vault() -> (gateway::Custodian, Option<Payload>);
pub struct Serving { /* 八个字段全 pub：调用方构造它 */ }
pub async fn listen(Serving) -> Result<Listening, AxError>;   // §8-88
impl Listening { pub async fn serve(self) -> Result<(), AxError>; }
pub struct ScanReport { pub waiting_approvals: usize /* lines、closed_calls 不跨出 */ }
impl ScanReport { pub fn summary(&self) -> String; }
pub struct RunWorker;
impl RunWorker {
    pub fn new(&Path, gateway::Custodian, Diagnostics) -> Result<Self, AxError>;
    pub fn handle(&mut self, wire::Command) -> Result<(), AxError>;
    pub fn startup_scan(&mut self) -> Result<ScanReport, AxError>;
    pub fn fork(&mut self, RunId, Seq, Option<Address>) -> Result<RunId, AxError>;
    pub fn adopt_building(&mut self, Address) -> Result<(), AxError>;   // 收楼即立基线 checkpoint（storage-SPEC 8-8 base_checkpoint），进度写诊断
}

// console
pub struct Terminal { pub url: String, pub token: Option<String> }

// firstrun
pub enum FirstScreen { Start(PathBuf), Use(PathBuf), Quit }
pub fn ask<R: BufRead, W: Write>(&Path, &mut R, &mut W) -> std::io::Result<FirstScreen>;
pub fn default_city(&Path, Option<&Path>, bool) -> PathBuf;
pub fn is_writable(&Path) -> bool;
pub fn local_url(SocketAddr) -> String;
pub fn open_when_ready(SocketAddr, String);
```

- **为什么需要一个 lib target**：`crates/sprawling` 至今只有 `src/main.rs`，`mod assembly` 是私有模块，于是工作区里**没有任何东西能依赖它**——4377 行生产代码（含 1058 行的 `dispatch_in`）只由同文件内的 66 个测试看守，citysim 与任何 `tests/` 都够不到。同一个事实还有第二个后果：它是唯一带 SPEC 却逃过 `apisync` 的 crate，因为 `spec_crates` 以 `src/lib.rs` 是否存在为依据。加一个 lib target 一并了结两件。
- **`pub mod` 而非扁平 facade**：§12 模块表以 `bin::assembly`／`bin::console`／`bin::firstrun` 命名模块，模块名本身是已记录的架构事实；折成 `sprawling::init_city` 会抹掉这层限定，而本 crate `publish = false`，C-REEXPORT 要替第三方省的那段路径没有受益人。**取窄的地方在项，不在模块**：只有跨出 crate 的项改 `pub`，其余留 `pub(crate)`——公开面因此是逐项决定的，不是逐模块授予的。
- **`install` 与 `wire_client` 留在 bin**：前者把二进制放上 PATH，后者从终端连一座已服务的城并从 stdin 读 enrolment——两者都是关于命令行的，不是关于城的，且除 `main` 外零引用。留在 bin 让公开面少六项。
- **`handle` 进公开面不是为测试拓宽**：AGENTS.md 写着「Tests use the same doors as production code」。`handle` 正是服务中的 worker 循环走的那扇门，把它命名出来是承认已有的门。反过来，那 66 个内部测试**不搬去 `tests/`**：它们触及 `Views::rebuild`／`read_building`／`run_id_for` 这类内部项，搬迁会为测试拓宽公开面，正是同一条规矩禁止的事。本 crate 的文件长度因此不变——它变短要等拆 `dispatch_in` 时把生产代码连同其测试一起搬走。
- **`ScanReport` 只放行一个字段**：`main` 读 `waiting_approvals` 决定是否多印一行，`lines` 与 `closed_calls` 只进 `summary()`。按需放行而非按结构对齐——`InitReport` 四个字段全跨出，是因为 `report_standing` 四个全读。
- **零行为变更**：`main.rs` 只改开头的声明块（七行 `mod` → 两行 `mod` ＋ 一行 `use sprawling::{assembly, console, firstrun}`），其余调用点逐字节不变。`Cargo.toml` 不改：Cargo 对同一 package 自动发现 `src/lib.rs` 与 `src/main.rs` 两个 target，OUT_DIR 对两者相同，`include!(client_embed.rs)` 与 `DEPENDENCIES` 因此留在 `main.rs` 原地。
- **红**：`crates/sprawling/tests/assembly_door.rs` 走 `init_city → RunWorker::new → handle(Command::CreateBuilding) → 读 InitReport.ledger_dir 下的账本`，断言 `building_created` 落账。改动之前它连编译都过不去（`sprawling` 这个 crate 名不存在），这就是「这条测试咬得动」的证据。
- **门禁连带**：`apisync` 把 `sprawling` 纳入契约，`tools/xtask/api-baselines/sprawling.txt` 随之生成（`guard` 的 `PRODUCED_PREFIXES` 已豁免该目录，不需 `Verdict:`）；`header` 要求 `lib.rs` 与新测试文件各带三行 MPL 通告；`modmap` 对 `*/lib.rs` 自动按索引文件判定，只准 `mod`／`use`／`pub use`／注释／属性——facade 因此只能是声明，正是要的形状。
- **一处文档更正**：ARCHITECTURE.md §3 写着「citysim is a second assembly layer: the same code with simulated adapters」。此句与现实不符——`tools/citysim/Cargo.toml` 依赖 kernel／storage／runtime／gateway，其中没有 sprawling；`run_scenario` 手工构造 `RunPlan`，够到的最高层是 `runtime::run::drive`。这次改动使 assembly **可被依赖**，但没有让 citysim 依赖它：模型适配器仍由 `adapter_for` 从 `EndpointBook` 内部构造，那条缝要不要倒置是另一个决定。按 AGENTS.md「reality wins and the document is corrected first, with its reason」，先把这句改成现实。

## 8-16 读不了的计划不再被报成被人改过的计划

**原因**：`dispatch_in` 里三处把失败抹平成默认值。

```rust
let plan_text = std::fs::read_to_string(&plan_path).unwrap_or_default();          // 驱动前：喂给 ClaimDesk
let shelf = city::archive_index(&self.city_root, building.addr()).unwrap_or_default();  // 驱动前：喂给 ArchiveDesk
let on_disk = std::fs::read_to_string(&plan_path).unwrap_or_default();           // 驱动后：落盘前的 compare-and-swap
```

第三处最重。那一段的注释自述它存在的理由——「each effect is checked against the file **as it stands now** … the losing claim is dropped with a diagnostic instead of overwriting somebody's row」。但读失败使 `on_disk` 成为空串，`still_true` 对空文档恒为 `false`，于是每一条 claim 都落入 stale 分支，人收到的诊断是「row … moved before this run's claim landed」——**一个从未发生的并发冲突**。他们会去查另一个居民，而真正要修的是一个读不开的文件。

（更重的那个后果——读失败→`stale` 为空→`write_plan` 覆盖真实计划——不成立：空文档下 `check_roadmap_shape` 不产 `WellFormed`，`still_true` 因此恒返 `false`。不存在数据丢失，只存在误报。）

第二处：`city::archive::index` 自己已经实现了正确契约（目录不在 → `Ok(空)`，真失败 → `Err`），所以 `.unwrap_or_default()` 恰好只扯掉真失败；换成 `?` 即可，不需新机制。

**现形**：

- `accounting::worker::plan_path` 删除。它在 `city` 之外拼了一遍 `city_root/<addr>/Roadmap.md`，而 `ROADMAP_FILE` 住在 `city::spine_files`——两份「计划在哪里」的权威。改走新增的 `city::roadmap_path`。
- 两处读全走 `city::roadmap`：仅 `NotFound` 答空串，其余以 `E_STORAGE_FATAL` 上报并带路径。一栋还没铺计划的楼确实没有计划，那不是失败；其余一切都是。
- `archive_index(…).unwrap_or_default()` → `?`。

**拒而不是降级**：计划是共享地面。读不到它就开跑，会花掉一次模型调用去产生一批注定被丢弃的 claim。在派活口上拒，人拿到的是路径和修法。

**红**：向一栋 `Roadmap.md` 是**目录**的楼派活（`read_to_string` 因此以非 `NotFound` 失败，无需权限把戏）。改动之前：派活成功，诊断行说「row moved」。改动之后：派活被拒，错误点名那个文件。

**影响面**：`city` 公开面增两项（基线同提交更新）。正常楼不受影响——`spine_files::lay_out` 给每栋新楼都铺了 `Roadmap.md`，而未铺的情形仍走 `NotFound` 答空串这条。`assembly.rs:219`（楼页读 Roadmap）同属一族但爆炸半径不同——那里读不到只是页上少一块，不会变成误报——此处不动。

## 8-17 一次验证遍历，三个折叠

```rust
pub(crate) struct Standing { pub(crate) book: gateway::EndpointBook, governance: views::Governance, collaboration: Collaboration }
impl Standing { pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError>; }
// serve 的那一遍：同一份已验证记录，逐条先给 Views 再给 Standing
pub(crate) fn fold_city(ledger_dir: &Path, cost: &mut OpeningCost) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // cost 见 8-121
impl RunWorker { pub(crate) fn holding(city_root: &Path, vault: Custodian, log: Diagnostics, held: (JsonlLedger, Standing)) -> Result<RunWorker, AxError>; }

// views/governance.rs —— 判定面与读面共用的那一个定义
impl Governance { pub(crate) fn empty() -> Governance; fn absorb(&mut self, record: &EventRecord); }
struct CollaborationFold { … }   // 暂存 enqueued／consumed，`settle` 产 Collaboration
```

`rebuild_book`／`rebuild_governance`／`rebuild_collaboration` 三个函数删除。

- `Standing::fold` 与视图的起步（`Views::rebuild`，经 `views::snapshot::start`）折叠的是 `runtime::replay::fold_ledger_dir` 逐行借出的已解析记录，不对原始行调第二次 `EventRecord::parse_line`。理由有二：同一行只解析一次；更要紧的是，逐行检查放行的 `ig: true` 未知种类行（`VerifiedLine::IgnoredUnknown`）在第二次解析时会失败，于是一份能通过验证的历史却起不了城。两个折叠对 `IgnoredUnknown` 都跳过，与 `runtime::fork` 的读法一致。由 `a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary` 判定。
- serve 只验证并折叠历史一遍：`fold_city` 在 serve 线程上先 `JsonlLedger::open`（取得写锁、修复撕裂尾），再经 `runtime::replay::fold_ledger_dir` 逐段流式读一次，每条已知记录先给 `Views::apply`、再给 `Standing` 的折叠；打开的账本与折好的 `Standing` 作为一个值经 `assembly::attending::Opening` 交给 `RunWorker::holding`，worker 线程既不打开账本、也不读史。两个折叠对同一份记录作答，所以 worker 判定用的治理与页面读到的治理出自同一遍，不会因两遍之间账本变了而不等。`RunWorker::new`／`over` 自己折，它们的调用者（genesis、命令行、测试）手里没有现成的 `Standing`。`Standing` 是 worker 判定的依据，所以它必须在写锁之下折：锁先于读史取得，别的进程在折叠与打开之间追加的一行（例如 `CityHalted`）就无从落在 worker 的治理之外。serve 要求账本目录已存在（`init` 建好的城市），`JsonlLedger::open` 对缺失目录报错。
- `Standing::fold` 与视图的起步同走 `fold_ledger_dir`：三处启动折叠都不同时持有全部原始行与全部记录，常驻的只是一段字节与一条记录。
- `LedgerIndex` 在 `fold_ledger_dir` 的同一遍里建：`fold_city` 与视图的起步把那一遍返回的索引交给 `Views::hold_index(index, ledger_dir)`，`Views::new` 与 `Views::over` 只放一个空索引，不另扫一遍历史；没经过折叠的 `Views`（测试、命令行）在第一次查询前由 `refresh` 补齐。`Standing::fold` 不要索引，丢弃它。
- 尚未落地的部分：`Governance` 与 `EndpointBook` 仍在 `Views` 与 worker 各有一份。验收是 5 万条记录时首字节 ≤ 500 ms、l100k 启动峰值 ≤ 稳定值 + 8 MiB，由 `just bench-startup` 量。
- `Standing::fold` 与 `Views::rebuild` 折叠的是 `runtime::replay::VerifiedLedger::lines()` 里那份已解析的记录，不再对原始行调第二次 `EventRecord::parse_line`。理由有二：同一行只解析一次；更要紧的是，逐行检查放行的 `ig: true` 未知种类行（`VerifiedLine::IgnoredUnknown`）在第二次解析时会失败，于是一份能通过验证的历史却起不了城。两个折叠对 `IgnoredUnknown` 都跳过，与 `runtime::fork` 的读法一致。由 `a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary` 判定。
- 尚未落地的部分：`Views`、`Standing` 与 `LedgerIndex` 合成一个 `CityFold`，在同一遍里建立，并让 `Governance` 与 `EndpointBook` 各只留一份；验收是 5 万条记录时首字节 ≤ 500 ms、l100k 启动峰值 ≤ 稳定值 + 8 MiB。这需要 `assembly::attending::spawn_worker` 把 serve 线程上折好的 `Standing` 交给 `RunWorker`，而不是让 `RunWorker::new` 再读一遍。

- **这不是缺陷修复**。三处实现漂移的假设（`rebuild_governance` 管 `granted` 与 `CityHalted`，`govern` 不管，`answer_approval`／`set_admission` 各自直改字段）不成立：新测试 `what_a_worker_holds_is_what_a_restart_rebuilds` 否定了它——派一次活、发一条信号之后，活 worker 与重建结果逐项相等。那条测试因此不是这次的红，而是让合并安全的护栏；它同时把一条四处代码都依赖、却从未被断言过的形状-7 性质变成了可红的。
- **以测量收口而非以红转绿收口**，理由写在上一条：没有可咬的红，因为没有缺陷。实测（windows-x86_64、16 核，release，外部探针经 `sprawling` 的 lib 门驱动 `RunWorker::new`）：

  | 记录数 | 改前 | 改后 |
  |---:|---:|---:|
  | 5,000 | 136.6 ms | 44.6 ms |
  | 20,000 | 538.7 ms | 175.6 ms |
  | 50,000 | 1856.1 ms | 436.2 ms |

  这是每一次 `serve`、`resume`、`fork`、`adopt` 都要付的钱。
- **为何不是 4 → 1 而是 4 → 2**：`RunWorker::new` 自己还要 `JsonlLedger::open`（尾部恢复）读一遍，而 `serve` 另走 `Views::rebuild` 一遍。把 `Views` 也并进来要改 `serve` 的所有权形状（它住在 `Arc<Mutex<_>>` 里与查询侧共享，而 worker 在自己线程上）——那属于 `dispatch_in` 的拆分，不在此顺手做。
- **暂存只给真需要的一项**：`CollaborationFold` 只暂存 signals，因为队列是 `enqueued` 减 `consumed` 而两者到达顺序任意；book、governance、goals、requests 都是逐条即结的，所以不暂存。
- **验证不动位**：链验仍在折叠之前。一部不能自证的历史，不是这三个视图中任何一个可以建在上面的历史。

## 8-18 审查中的 run 不再把自己的决策直接放上楼的书架

**原因**：`dispatch_in` 的档案回收段写的是 `city::file_archive(&self.city_root, …)`，而不是 `&write_root`。ARCHITECTURE.md §5 把输线设计写在明处——「A building under review gives every run its own tree … Nothing it writes is visible until somebody else checks it — the losing line of the design made physical rather than promised」。档案不在那棵树里。

具体危害：书架回头喂给 `ArchiveDesk`，成为模型看到的「这栋楼已经知道什么」。一个**被驳回**的 PR 里的决策因此会留在架上，变成后续 run 的前提。

**现形两步，缺一不可**：

1. `city::file_archive(&write_root, …)`——落进检查点。
2. 持有租约时，`checkpoint_scope` 从**房间**改为**楼**。否则第一步把泄漏换成了静默丢失：`wave_pre` 只暂存 `<scope>/*`，而档案在 `<building>/Archive/…`，在房间作用域之外，不会进提交，租约一释放就没了。在**自己独占的** worktree 里暂存整栋楼是安全的：那棵树里变化过的东西全是这个 run 的。无租约时检查点仍在房间——那才是一个 run 在城里唯一可写的地方。

**与计划的不对称是故意的**：`Roadmap.md` 恒写回城里，因为它是共享地面且带着对当前文件的 compare-and-swap（那段注释自述了理由）。档案没有这样的声明，也没有守卫，所以它是漏而不是决定。

**四处检查点调用点**：（`workbench/standing.rs` 的 `ensure_base`、`workbench/tools.rs` 的 `with_checkpoint`、`driving.rs` 的逐波检查点、`reviewing.rs` 的 `PrEffect::Opened`）都从 `Site` 与 `RunWorker` 手上凑齐一个 `storage::Provenance`（run id、resident 地址、选中的模型 id 与思考档位、城的创世哈希）交给 storage；城的创世哈希由 `storage::Provenance::city_of(ledger_dir)` 只读账本首段第一行得出。

**红**：在 `review = true` 的楼里派一次带 `archive` 工具调用的活，断言书架仍空。改动之前它拿到 `[Entry { kind: Decision, … lab\Archive\decision\… }]`。同一条测试接着让第二位居民检查并合入，断言书架变为 1 条——**两半同一条测试**，因为只测前半的修法可以是「干脆不写」。

## 8-19 沙箱接上

**原因**：`dispatch_in` 把 `Box::new(runtime::AbsentSandbox)` 写成字面量，而 `crates/sprawling/Cargo.toml` 没有任何 feature 到达 `runtime/wasm`。于是 `runtime::WasmtimeSandbox` 在 `runtime/tests/sandbox_a10.rs` 之外**没有调用方**，任何 sprawling 构建都到不了执行引擎。

`AbsentSandbox` 的三段式拒绝里写着 recovery：「use the program arm, or install a build with the `wasm` feature」。**那样的构建不存在。** 只写在文里、无人执行的 recovery 等于没有 recovery——这次是只写在错误消息里、无人可安装的构建。

ARCHITECTURE.md §2 把 wasmtime 列进技术栈并声明了代价（「Cost: an optional feature; a build without it refuses tool execution in three parts rather than pretending」），措辞预设了存在带该 feature 的构建。

**现形**：
- `crates/sprawling/Cargo.toml` 增 `[features] sandbox = ["runtime/wasm"]`。
- 引擎的选择收进一个函数 `execution_engine()`，两条 `#[cfg]` 臂各一个实现，`dispatch_in` 的构造点因此不随 feature 改变形状。
- **带引擎的构建起不来引擎就拒派活，不回落**。回落是「人以为跑在沙箱里、其实没有」的由来。

**默认仍为关**：这是 ARCHITECTURE.md 记录过的取舍（wasmtime 是一大块二进制），不改默认，只让开关存在。`just check` 走 `--all-features`，所以带 feature 的那条臂进门禁；`just dist` 不带，所以交付形态与体积预算不变。

**红是编译红而非行为红，这里说明白**：改动前 `execution_engine` 不存在，测试连编译都过不去。行为面的红需要一个真的 `python.wasm` 与 `SPRAWLING_PYTHON_WASM`，那是交付形态的事，不在此范围内。新测试只在 `cfg(feature = "sandbox")` 下存在，断言引擎给出的不是「this build carries no execution engine」那句话。

## 8-20 交接件读不了不再等于没有交接件

`city::handoff` 的 `.ok()?` 与 §8-16 修掉的那三处同族，且它喂的是 **prefix 的 run 段**——下一次会话读到的第一样东西。三件事（文件不在／读不了／仍是空白表单）原先并为一个 `None`。

现形与 `roadmap` 同：`Result<Option<String>, AxError>`，`None` 只说「没有值得带走的东西」，`NotFound` 归入其中，其余上报并带路径。同时补 `handoff_path` 与把 `HANDOFF_FILE` 转 `pub`——红测要点名那个文件，而在别处拼一遍文件名就是第二份权威。

`run_segment` 因此转为 `Result<Vec<u8>, AxError>`；它只有一个调用方（prefix 的四段装配），所以波及面就是那一处 `?`。

**红**：向一栋 `Handoff.md` 是目录的楼派活，`expect_err` 撞上 `Ok(())`。`city` 侧另加一条单测，把三件事排成三行断言。

## 8-21 控制台读得到它身处的那座城

**原因（两处，同一个不对称）**：控制台与 socket 拿的是同一张桌子（`CommandDesk`）与同一条事件流，唯独**读**这一路没接上。

1. `console::post` 对 `ClientFrame::Ask` 只印一句「a question is answered over the wire: `sprawling call '…'`」——它请人开第二个终端，去问一座人已经身处其中的城。而 `accounting::worker::serve` 早已构造出 `queries: wire::Answering` 并只交给 socket。§8-11 自述「查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形」——**这句话今天是假的**，这次改动使它为真。
2. `serve_city` 起城时印的四行（city／WebUI／client）随事件流滚走。一个远程盯着城的人于是再也看不到自己开在哪个端口、有几条 run 在跑。

```rust
// bin::console（形状仍为 1 decision；渲染是纯函数，I/O 仍在壳里）
pub struct Terminal {
    pub url: String,
    pub token: Option<String>,
    pub city: String,     // 新增：城在磁盘上的位置
    pub client: String,   // 新增：客户端从哪来（嵌入／目录）
    pub bind: SocketAddr, // 新增：真正绑住的那个地址
}

/// socket 用的那一个答询函数，控制台拿到的是它的副本。
pub(crate) type Answering =
    Arc<dyn Fn(wire::Query) -> Result<wire::Answer, AxError> + Send + Sync>;

pub(crate) enum Line { …, Serving }   // 控制动词从四个变五个

/// 进程自己知道的事实 ⊕ 一次 Metrics 的答案 → 一屏。纯。
pub(crate) fn serving(terminal: &Terminal, vitals: &wire::MetricsAnswer, pid: u32) -> String;
```

- **答询走同一个函数，不是第二个权威**。`post` 的 Query 臂改调 `Answering`，与 `wire::server` 的 `SessionStep::Answer` 是同一个 `Arc`；控制台答出来的数字与浏览器看到的数字不可能不同，因为它们是同一次调用。
- **`/serving` 是渲染，不是来源**。城侧那几个数（几条 run 在跑、几件事等人、几栋楼）全部来自一次 `Query::Metrics`；`/metrics` 仍印它的 JSONL 原样，与 `sprawling call` 同形。两个动词，两个问题，无重叠：`/serving` 答「这个进程开在哪、门朝谁开」，`/metrics` 答「这座城里有多少什么」。
- **动词名不与既有概念撞车**。`status` 在 `docs/glossary.md` 里已经是**工具**的名字（「答一次 run 自己的处境」），一名一义是门禁事项，故控制台这个动词叫 `serving`——它印的正是 `accounting::worker::Serving` 持有的那几样东西，沿用已在库内的词。
- **常驻内存不进这一屏，理由记在这里**。「resident 在本平台叫什么」的唯一权威是 `xtask::mem`（它读哪三个计数器、各平台叫什么，见 `tools/xtask/xtask-SPEC.md` §8-30），而 `xtask` 只依赖 `kernel`——让它依赖产品会使每次门禁编译整个 workspace。在 bin 里再抄一张三平台表，正是那个模块自己的 doc comment 警告的「三份权威」。`/serving` 因此印出本进程 **pid**，`cargo xtask mem <pid>` 只差一次粘贴。**翻案条件**：新增第十三个 unit 承载这一个计数器（ARCHITECTURE.md §3 的拓扑是 add-only），届时两个调用方共用一份定义。

**红**：一条测试把 `Line::Serving` 之外的路径全部钉住不动，另一条驱动 `drive` 读入 `/metrics`，断言输出里有 `MetricsAnswer` 的 JSON 而**不含** `sprawling call`——改动之前它撞上那句转介。第三条断言 `serving()` 的那一屏同时含端口、`runs`、与 pid。

## 8-22 面向网络的那扇门自己铸钥匙，页面把钥匙递上去

**原因（一条端到端全断的链，四段里断三段）**：把 WebUI 暴露到回环之外这件事，今天**做不到**。

| 段 | 今天 | 依据 |
|---|---|---|
| 铸 | `PairingToken::mint` **在产品里没有调用方**，只有测试用它 | `grep mint(` 只命中 `wire/tests` |
| 拒 | `decide_bind` 在没有 `SPRAWLING_PAIRING_TOKEN` 时拒绝任何非回环地址 | `server.rs:63` |
| 携 | `console::web_url` 把 `?token=…` 挂到 URL 上 | `console.rs` |
| 递 | `web::app` 写死 `Link::new(None)`，且 `socket_url()` 只取 `host`，**查询串整段丢掉** | `app.rs:1679`、`socket.rs:311` |

于是：不配置令牌起不来；配置了令牌，页面握手时不出示任何东西，`decide_handshake` 照 `server.rs:306` 拒之。**一座暴露出去的城连自己的 WebUI 都进不来。** 这不是两个缺陷，是一条链，所以一次修完，不留「钥匙铸出来了但没人能用」的中间态。

**对用户提案的更正，记为落选方案**：「BLAKE3 随机抽一个文件的 hex 值当口令」的熵是 `log2(候选文件数)`，不是摘要宽度——十万个文件约 17 bit，可当场穷举；且被抽中的文件内容常常是公开的（仓库里的源文件、依赖的许可证）。攻击者只要知道文件集合就把 256 bit 的外观还原成一次目录枚举。`PairingToken::mint` 收 32 字节 OS 熵、经 29 符号字母表给出四组五位（约 97 bit），**且它的 doc 明写就是为「显示一次」而设**。这里用它，不另造。

```rust
// bin::keying（形状 1 decision；纯，穷尽，无 I/O、无熵）
pub(crate) enum Keying {
    /// 回环：运行中的机器自己，什么都不用出示。
    NothingToPresent,
    /// 人配置过的：我们没见过它被铸出来，故不显示。
    Adopt,
    /// 这次服务当场铸一把，显示一次，不落任何地方。
    Mint,
}
impl Keying { pub(crate) fn decide(bind: SocketAddr, configured: bool) -> Self; }

// bin::assembly（形状 4 适配器；熵在这里取，与 `random_token` 同一处出身）
pub enum Keyed { NothingToPresent, Adopted(String), Minted(String) }
pub fn key_for(bind: SocketAddr, configured: Option<String>) -> Result<Keyed, AxError>;
```

- **`decide_bind` 不动**。它仍是那条守卫；assembly 只是**在 socket 存在之前先把它满足了**，所以它 doc 里那句「there is no window in which the port is open and unauthenticated」原样成立。拒绝臂在 `wire` 自己的测试与任何第三方 embedder 处仍可达。
- **人配置过的优先**。`SPRAWLING_PAIRING_TOKEN` 在场就 `from_configured` 采纳，不覆盖、不显示——我们没见过它被铸出来，印它就是把一个长期口令又抄进一处日志。
- **铸出来的 code 不落盘、不进 Ledger、不进 diagnostics**。进程结束即失效，这就是「一次性」。它只经两处：显示一次的那一行，与 `Terminal.token`（`/web` 据此拼出带钥匙的 URL）——后者今天已经持有明文，不扩大它的存放面。
- **页面这一段是纯函数加一次读**（Humble Object，ARCHITECTURE.md §9）：`web::socket::token_in(search) -> Option<String>` 对查询串取值，可在非 wasm 目标上测；`pairing_token()` 只在 wasm 下多一次 `location.search()`，不含判定。`app.rs` 的 `Link::new(None)` 改为 `Link::new(crate::socket::pairing_token())`。
- **令牌留在查询串里是既有取舍的延续**：`console::web_url` 的 doc 已写明这一取舍（「A token in a query string is a token in the browser's history, and that is the trade this makes deliberately」），替代方案是人在两个窗口之间手抄一个秘密，然后抄错并粘到更糟的地方。改存 `sessionStorage` 会把钥匙放进同源 JS 读得到的地方——对一座**公网暴露**的城，那比浏览器历史更坏，故不改。

**红（三条，每条咬住一段）**：`Keying::decide` 对四格（回环／暴露 × 配置过／没有）给出的枚举——改动之前 `keying` 不存在，是编译红；`token_in` 对 `?token=abc`、`?a=1&token=abc`、`?token=`、空串的四个答案；以及 `web::socket` 那条握手测试，断言 `Link::new(token_in(...))` 发出的 `Hello.token` 非空——改动之前 `Link::new(None)` 使它恒 `None`。端到端那一段（真浏览器对真暴露端口）落在 V9，是人跑的命令而非门禁，如 ARCHITECTURE.md §11 所记。

## 8-24 一条效应先成为账本行，再成为这座城（`accounting::effect`）

`accounting::effect` 的接口、理由与测试住 accounting-SPEC.md 8-5，这里不留第二份。装配层那一扇门是 `RunWorker::settle(&mut self, at: &Assignment, run: RunId, landing: effect::Landing, chain: &KnockChain)`（`accounting::worker::settling::landing`）：每张桌子交出的 `Landing` 都走它，`record` 先落完行，再把 `Then` 交给一个穷尽的 match。

## 8-25 一个答复接上的活，不靠重读全部历史找到，也不丢掉它的天花板

```rust
// Governance —— 现在是 RunWorker 的一个字段，而不是四个散字段加一份重写
struct Governance {
    pending: BTreeMap<String, ApprovalItem>, autonomy: Autonomy,
    granted: Vec<ClusterKey>, halted: BTreeSet<kernel::event::Scope>,
    rules: BTreeMap<(kernel::event::Scope, GoverningDocument), B3Hash>, // 从 rules_changed 折；每份治理文档上次记下的摘要（§8-40）
    sent: BTreeMap<RunId, Sent>,          // 从 run_started 折；task、goal、budget
    origins: BTreeMap<String, BlockedJob>, // 从 approval_requested 折；答复时 O(log n)
}
impl Governance {
    fn sent(&mut self, RunId, task: &str, goal: &str, BudgetCap);   // 两个调用方，一个形状
    fn absorb(&mut self, EventKind, RunId, Option<&Address>, &Payload);
}
struct BlockedJob { addr: Address, task: String, goal: String, budget: BudgetCap }
```

**三个原因，一个改动**（§8-23 留下的那一半）：

1. **`blocked_job` 扫全史**。每次审批应答都 `verify_ledger_dir` 一遍再解析两遍，只为找 `(addr, task, goal)`，随历史线性增长。（量级取自 §8-17 留下的同类读数：`verify_ledger_dir` 约 215k 记录/秒，于是 50k 的历史光验链就是百毫秒量级；没有重测。）
2. **天花板归零**。`fn dispatch` 写死 `BudgetCap::default()`，而它正是审批应答后续活走的那条路。一跑带着天花板派出、因待批停下、被批准后续上的那一跑，向模型报 `0 usd_micros, 0 tokens`。
3. **`self.pending.remove(item)` 先于落账**，与 `set_admission` 相反，且是冗余的——`record → govern(ApprovalResolved)` 本就移除它。`self.granted.push(…)` 同理。

**为什么三件一起改**：它们是同一个结构问题的三个面。治理状态本来有两份实现：`Governance::absorb`（重启折）与 `RunWorker::govern` 加上 `set_admission`／`answer_approval` 里直改字段的几行（活折）。§8-17 测过两者不漂移，但那只是当时恰好相等；**再加一份 origins 折就是第三份**。这里把四个散字段换成 `RunWorker.governance`，`govern` 就是 `absorb`，于是新的两张表只有一个折法。

**选甲而不选乙**。乙案是让 `approval_requested` 的 payload 自述所阻之活；但那份 payload 就是 `ApprovalItem` 本体，改它得改 `kernel::ApprovalItem` 的公开面与每一个构造点。更重要的是：**账本已经说得出一项是哪一跑提的**（envelope 的 `run`），它没说的是那一跑被派去做什么、在什么天花板下。那是 `run_started` 的事，不是每一项待批的事。

**一个未决问题，用测试回答了**：甲案是进程内存，重启后的 worker 从账本重建，那「重启前提出、重启后才批」的项接不接得上活？答：接得上，因为 `origins` 就在 `Governance` 里，而 `Standing::fold` 对每一行调的正是 `absorb`——与 `pending` 同一折、同一遍。`what_a_worker_holds_is_what_a_restart_rebuilds` 增一条断言盯住它。

**不裁剪 `sent`，写明代价**。每跑一条（两个短字串加 16 字节），与 `storage::HotView` 的墓碑同一增长级（每跑一条）。**不能按 `RunFrozen` 裁**：`freeze` 在 drive 内落账，而装配层的待批项清扫在 drive 之后，账本顺序是 `RunStarted … RunFrozen … ApprovalRequested`，按 freeze 裁会先删掉待用条目。`origins` 则在 `ApprovalResolved` 上裁，因为答过的项不再阻着任何东西。

**读在落账之前，派活在落账之后**：`answer_approval` 先取一份 `origins`（读，不是变化），再落 `approval_resolved`（它自身就是关闭动作，`absorb` 随之丢掉 pending 与 origin），最后才派活。与 §8-24 同一条规矩。

**红（两条）**：

- `work_resumed_by_an_answer_is_done_under_the_ceiling_that_sent_it`：以 `BudgetCap { usd: 250_000, tokens: 4_000 }` 派一跑，它读一次 `status`（对照组），然后提一项待批；批准后续上的那一跑再读一次。**两跑同地址**，所以依据不是 `addr:` 而是该地址上读到的天花板去重后的**集合**：改动之前是两个值（`0 …` 与 `250000 …`），之后是一个。数请求体不能作依据（§8-23 已记）。
- `what_a_worker_holds_is_what_a_restart_rebuilds` 增一条：活 worker 的 `origins` 与 `Standing::fold` 重建的逐项相等。改动之前 `origins` 不存在，是编译红。

**性能以结构收口而不以计时收口**：一次审批应答从「验链一遍加解析两遍全史」变为一次 `BTreeMap` 查找；`blocked_job` 连同它的两个循环一并删除，因此这不是一个快了多少的问题——那条路径不存在了。

**影面**：`runtime` 公开面增一字段（`RunPlan.budget`），基线与 runtime-SPEC 同提交；`RunPlan` 的三个构造点（assembly、citysim、runtime 集成测）各加一行；`tools/fixtures/golden-p0` 重生。

## 8-26 读不到一份文件不等于那份文件写错了

```rust
fn city_segment(city_root: &Path) -> Result<Vec<u8>, AxError>;  // NotFound → 内置副本；其余 → Err
```

§8-20（交接件）与 §8-16（计划）定下的形制是：**「还没有」答默认值，「读不了」带着路径上报**。这里收尾同族剩下的两处。

**一、楼页把「读不开」报成「表写错了」**。「读不到只是页面少一块，不会变成误报」不成立。`read_building` 把读失败抹成空串，而 `check_roadmap_shape("")` 并不返回空结果：`header_seen` 为假使它推出 `Malformed { problems: ["no four-column table found"] }`。于是页面向人断言一件它无从得知的事：那张表的形状不对。人于是去修表格，而要修的是一个打不开的文件。

**现形**：改走 `city::roadmap`（§8-16 立的那扇门），读失败时**把失败本身放进 `problems`**——那正是这个字段的用途，也是页面已经会画的东西。`read_building` 不改返回类型：`None` 的意思是「没这栋楼」，把「计划读不了」塑成那个形状会让一栋存在的楼从城里消失。

**二、关城时把零字节当成城的规范**。`close_city` 的 `std::fs::read(&city_file).unwrap_or_default()` 使 must-read 指向空字节的 CAS 哈希：下一任被告知「先读这份」，读到的是什么都没有。

**现形**：不新建读法，改用同文件已有的 `city_segment`——「这座城的规范是什么」应当只有一个答案，而 prefix 装配已经在问同一个问题。同时把 `city_segment` 自己改成同一形制：它原本的 `unwrap_or_else(|_| CITY_MD)` 注释自述为「falling back to the built-in copy **when a city predates it**」，而那只描述了 `NotFound`；其余失败下它静默地拿内置副本冗作人编过的那份，而两份可以完全不同。修后：`NotFound` 仍答内置副本（那是已记录的契约），其余一律带路径上报，于是一跑在读不了的城规范下开跑这件事也一并没了。

**关城于是会失败，这是有意的**。一次说不出下一任该读什么的关闭不是一次有序关闭；`serve` 的循环已经写着 `eprintln!("the city could not write its handoff: {err}")`，于是人在终端上拿到路径与修法，而不是一条指向空白的交接件。

**红（两条，各咬一处）**：把 `Roadmap.md`／`City.md` 各做成**同名目录**（§8-20／§8-16 用过的手法，不碰权限，在 Windows 上稳定）。一：楼页的 `problems` 必须点名 `Roadmap.md`——改动之前它说的是 `no four-column table found`。二：`close_city` 必须以点名 `City.md` 的错误拒绝——改动之前它返回 `Ok` 并写下一条指向空字节的 must-read。

## 8-27 一次登记喂到两处，于是只写一遍

**原因不是缺陷，是两个权威**。`dispatch_in` 里目录准入与工作台注册是两份各十三行的名单，而同一段的注释自述「one registration feeds both」。两份名单今天相等，但相等是人维护出来的：只上工作台的工具是没人能叫的工具，只上目录的工具是告诉了模型、叫下去却不存在的工具。

**现形**：一个 `Vec<Box<dyn kernel::Tool>>`，一个循环里先 `admit_tool(tool.meta())` 再 `bench.register(tool)`。順序取**目录的**那一份：`Catalog::render` 按准入顺序把工具摆在模型面前，而 resident 段是要算哈希的，所以这个顺序是缓存面的一部分。十三件的次序逐字照旧代码排（archive、exec、claim、edit、status、signal、goal、pr、delegate、workshop、rules、neighbours、read，然后 MCP），故字节不变。

**它以什么收口**（照 §8-17 的写法）：**没有可咬的红**，因为两份名单今天并未漂移，十三对十三逐项相等。一条「两集合相等」的断言今天就绿，而且改完之后它恒绿（不可能不相等），那不是测试而是装饰。收口在于：变化后两份名单不可能不相等，且 141 条现有测试（包括多条断言工具名与 prefix 内容的）全绿。**不为了凑一个红而补一条前后都绿的测试。**

**尺寸不是这次改动的理由**：`dispatch_in` 983 → 977。五十行准入换成四十五行名单加循环，净值接近零；换来的是一个权威而不是两个。

**一个曾被推翻、现已了结的假设**：`invoke` 里 `match bench.invoke(…)` 的 `_ =>` 臂当时看似死代码，删掉却得 `E0004`——`BenchOutcome` 带 `#[non_exhaustive]`，本 crate 在它定义的 crate 之外，穷尽匹配不可写。那一臂因此保留并注明不可达。

——**该接口问题已结**（G-22 / 叶子 7.10）：`runtime` 不发布，工作区之外没有第三方，`#[non_exhaustive]` 只换来每个下游一条永不执行的分支。全库枚举现已撤下该属性，这一臂与同类的四十余条一并删除，穷尽性回到编译器手里。

## 8-28 一次调用的键，只有一份读法

```rust
// driving::placing::Placing::admit 内：六行手写的动作字节换成一个问句
let key = match call.action() { Ok(action) => IdemKey::derive(&self.run, Seq::new(at), &action), Err(e) => return Admitted::Answered(Err(e)) };
```

**这里不修 `bin::assembly` 的缺陷，因为这里没有缺陷**。被修的是 citysim（citysim-SPEC §8-4）；本文件变的是「谁来回答动作字节」。原先这六行把 name 与 `serde_json::to_string(&call.args)` 拼起来，是全库两份实现中对的那一份；对的那一份待在装配层，正是另一份能静静漂走的原因。`kernel-SPEC §8-6` 早写着这条规则「属 S2 工具面」，而它一处也不在那里。现在它在（`ToolCall::action`，kernel-SPEC §8-23），本文件改为问它。

**字节逐字不变**：`action()` 内部就是搬过去的同一句（name 字节接 args 的 JSON 字节），位次是 `Placing` 在放行时按调用序数出的计数器。唯一的行为差异是 `unwrap_or_default()` 换成 `?`：一个序列化失败以前产空串（于是两次参数不同的调用得同一把键），现在上报。`Payload` 拒浮点且键恒为字符串，故这一臂今天不可达。

**一个被推翻的假设**：「让 `ToolBench` 自己持 run 与位次、`invoke` 内部铸键」看上去是更好的形——传钟进来这件事就没有参数可传。它不成立：`ToolBench::seen` 恒从空集起，且键在过门之后才记入（runtime-SPEC §532），所以一个恒递增的内部位次会让键在一次驱动内永不重复，`BenchOutcome::Duplicate` 随之变成**任何门都达不到的变体**，`turn.rs` 那条 `dedup_runs_before_the_side_effect`（同键调两次、断言文件未再变）连同它守的不变量一起写不出来。**把一个可测的防御换成不可测的死代码，不是加固。** 位次因此留在调用方。

**顺手记下、这里不动的一件事**：dedup 是一道**今天接不到任何东西的防御**。`kernel::idem` 自述它存在是为了「resume 与 replay 重派出同一把键」的双付防御，而 `seen` 从不从历史播种，`sprawling resume` 也不重跑一跑（ARCH §5 末：它只验链、把丢了结果的调用关成 unknown、并报告等人的事）。此后，`Duplicate` 在两个驱动器里都不会再出现，而这是**对的**：它本就是重放路径上的结果。要让它真正接上，得让 `seen` 从账本重建——那是另一件事，它自己的红在「重建后的 worker 不会把已经付过的钱再付一遍」上。

## 8-29 行没落下，城就没动

```rust
impl RunWorker {
    pub fn new(city_root, vault, log) -> Result<Self, AxError>;              // = open ➕ over
    pub(crate) fn over(city_root, vault, log, ledger: JsonlLedger) -> Result<Self, AxError>;
}
```

这是 §8-24 那条性质的另一半。§8-24 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）；这里问的是「**行没落下，城就没动**」。

**选甲而不选乙**。一种说法是甲案（`RunWorker::over`）「只有一个生产调用方，近乎为测试拓宽」，而乙案（倒置 `kernel::Ledger`）才是 ARCH 点名的那类动作。两头都不对：

- **甲不是测试拓宽，是 ARCH §3 自己提的那条批评**。§3 末段写着 `RunWorker`「builds its model adapter **instead of receiving one**」，并把它列为 V6 停在装配层下方的原因。同一句逐字适用于账本：一个自己 `open` 账本的 worker 同样无法被驱动到第二份实现上。把「账本从哪来」从构造子里取出去，是把一个不属于它的决定交回给调用方。
- **乙今天买不起**。`RunWorker` 对账本用的不只 `append`，还有 `position()`（两处）与 `observe()`。把 `observe` 推上 `kernel::Ledger` 等于让最内层去定义什么是「耐久后通知」——那是持久化适配器的事，不是「一个 Ledger 是什么」的事；代价是全库 **七个 `impl Ledger`** 各长出一个它们不需要的方法，加 conformance 套件。而这里根本不需要第二个类型：两条路上都是具体的 `JsonlLedger`，**不同的是它下面的 `Vfs`**。既然缝不必动，就不动。
- 丙（只在 `storage` 内写红）**已经存在**：`power_cut_matrix_over_every_op_keeps_acknowledged_waves`。它证的是账本自己的耐久契约，不是装配层的不变量，所以它不替代这一条。

**它以什么收口：没有红，照 §8-17／§8-27 的写法说清楚**。`a_line_the_history_refused_is_a_change_the_city_never_made` **首跑即绿**，因为这条性质 §8-24 已经用类型持住了：`record` 遇拒即 `?` 返回，`Then` 随之丢弃，改变无从发生。**未止步于一条前后都绿的测试**：把 `Landing::record` 的 `append(line)?` 改成 `let _ = append(line);` 后重跑，它当场红，且红在实质那条断言上——盘上的计划被写成了 `| 1 | wire the kiln | In progress |  |`，而宣布它的那一行从未落地。恢复后又绿。这条测试因此是一张网，不是一条红，而它能咬是量出来的不是声明出来的。

**测试里两个世界各归各位**：账本在 `FaultFs` 的内存平面上，城的文件（`Roadmap.md`）在真盘上。这正是要问的形状：被断言的东西是一份人事后真能去打开的文件。`Standing::fold` 仍读真目录（那是「城到目前为止知道什么」），而本跑新落的行进虚拟账本——两者不相干，因为断言不靠账本内容，只靠盘上那份计划。

**影面**：`storage` 公开面在 `fault` 下增 `open_faulty`、`FaultPlan` 增一字段（storage-SPEC §8-2 同提交）；`sprawling` 公开面**不变**（`over` 是 `pub(crate)`）；`crates/sprawling/Cargo.toml` 的 dev-dependencies 打开 `storage/fault`，发行构建不含它。

## 8-30 合并也排到它那条行后面

§8-24 把五张桌子搬进 `accounting::effect` 时，把 `PrEffect::Merged` 留在原地，理由写得很清楚：`trees.merge` 确实先动世界，但「先落账在这里更坏」——`merge` 有一条可达的失败臂 `MergeStale`，先落账就是把一句谎写进历史里的可达路径。它同时写下了解法：`storage::Worktrees` 得先能答「这一合并会落在哪个 commit」且能先验干线。这里做的就是那一条（storage-SPEC §8-2），于是两头不再互斥：

```rust
let planned = trees.plan_merge(&name)?;    // 全部拒绝在此，世界未动
record_for(…, EventKind::PrMerged, … planned.commit() …)?;   // 行
planned.apply()?;                           // 才是变化
```

于是：一个会被拒的合并永远不会先得到一条行（`MergeStale` 早于落账）；一条没落下的行也永远不会已经改了干线（`apply` 需要一个只能从 `plan_merge` 拿到的值，而行写在它之前）。

**红**：`a_merge_the_history_refused_leaves_the_building_where_it_was`。一个 `review = true` 的楼，一跑改文并提交请求，第二跑去检——而第二跑的账本是 `open_faulty`（§8-29 的工具）且 `cut_on_write: Some("pr_merged")`。断言：楼里那份文件仍是 `before`。**改动之前它是 `after`**：干线已经移了，而宣布它的那一行从未落地——一座楼站在它自己的历史说从来没有并入过的工作上。

**影面**：`storage` 公开面去 `Worktrees::merge`、增 `plan_merge` 与 `PlannedMerge`（基线与 storage-SPEC 同提交）；四个读写方全部迁完后旧入口删除，不留适配。`sprawling` 公开面不变。

## 8-31 dispatch_in 向 ARCH §5 的十二步靠拢（逐次拆分）

**目标不是「把某一段搬走」，是「让 `dispatch_in` 成为 ARCHITECTURE.md §5 已经写好的那个序列」**。两者的区别是形状问题的生死：按行号切出来的一块叫不出 §9 的名字，而一个相位叫得出来——§5 已经给了它名字。

**第一次拆分：驱动（§5 步 7–11）。**

```rust
struct Driven {
    outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,   // 仍是 Result：跑败也要结桌子
    checkpointed: Vec<String>, ran: (u32, u32), raised: Vec<ApprovalItem>,
}
impl RunWorker {
    fn drive_dispatch(&mut self, plan, handoff, adapter, bench, signals,
                      write_root, checkpoint_scope, who, run_id) -> Result<Driven, AxError>;
}
```

三个钩子住在一起，理由不是它们相邻，而是**它们是唯一在驱动器持有账本期间碰账本的代码**（`invoke` 里那句自述：「the ledger is the driver's for the length of the run」）。它们收集的三样东西也只在那段时间里可写，所以一并作为 `Driven` 返回，而不是留四个 `Rc<RefCell<…>>` 让调用方自己保持同步——四个单元格是四个可以忘记读的东西，一个值不是。

工具面是 `accounting::worker::driving::placing::Placing`（形状：adapter），runtime-SPEC §8-3 的 `ConcurrentInvoke` 在装配层的实现：

```rust
pub(super) struct Placing<'f> { /* bench、sieving、run、next、ran —— 私有；checkpointing: &'f Checkpointing */ }
impl<'f> Placing<'f> {
    pub(super) fn new(bench: ToolBench, sieving: Sieving, run: RunId, checkpointing: &'f Checkpointing) -> Placing<'f>;
    pub(super) fn ran(&self) -> (u32, u32);   // 本跑命令的 (通过, 失败)
}
impl ConcurrentInvoke for Placing<'_> { /* admit：定位次、派生键、bench.clear；tool：bench.tool_for；account：bench.account，再记 checkpoint 与命令计数、exec 结果过筛 */ }
```

键的位次在 `admit` 里定、checkpoint 记录与命令计数在 `account` 里记，两段都按调用序跑，所以一波开头的只读调用并行跑完之后，键、checkpoint 列表与计数与串行波逐字相同。位次与计数是 `Placing` 的普通字段：先前它们是闭包捕获的 `Cell` 与 `Rc<RefCell<…>>`，因为闭包只能借不能拥有。`checkpointed` 仍是一个 `RefCell`，借给 `Placing` 与波前 checkpoint 钩子两处写，因为清扫读的是**最早**立起的那道 checkpoint（`settling::desks`），两份各自的列表合不出时间次序。失败码不新增：`admit` 与 `account` 的失败原样是 bench 与工具的失败，回给模型。

`outcome` 刻意仍是 `Result` 而不在方法里 `?`：一跑失败了它的桌子照样要结，而结桌子正是把它最后几行放上历史的动作。把它提到方法边界上会静静跳过它们。

**尺寸**：`dispatch_in` 975 → **833**；`drive_dispatch` 171。尺寸不是这次拆分的理由（照 §8-27 的写法），但它是尺寸门的前提，而那道门的门限是量出来的 200。

**它以什么收口**：纯结构，无可咬的红——行为逐字不变（钩子体原样搬迁，`checkpoint_scope` 由计算改为传入）。143 条 `sprawling` 测试全绿，其中包括直接盯驱动行为的 §8-24／§8-29／§8-30 三条。

### 剩下的五次拆分

`dispatch_in` 今为 **833** 行（@3611），相位实测如下。目标 <200；每次拆分都是同一个形制：相位成为 `RunWorker` 的一个方法，多个活值归并为一个归位值类型（如 `Driven`），而不是一排得保持同步的局部变量。

已切九次，**975 → 158**，产出的方法均在阀值内：`drive_dispatch` 171、`settle_desks` 124、`settle_requests` 122、`conclude` 104、`stand_up` 92、`admit_reading_room` 32。三个归位值类型：`Driven`（驱动期间写、驱动之后读的四样东西）、`Desks`（一起出借、一起收回的五张桌子）与 `Site`（一次跑站在哪儿）。

**第七次拆分：桌子（§5 步 3–4）。**

```rust
struct Desks { signals, goals, plan, shelf, pr, plan_path: PathBuf, waiting: u32 }
impl RunWorker {
    fn open_desks(&mut self, site: &Site, addr: &Address) -> Result<Desks, AxError>;
}
```

**`pr` 与 `waiting` 入伙，`Desks` 的理由随之改写**。建 `Desks` 时写的理由是「一起结算」，而 `pr` 不与它们一起结（它等 `produced`，在 `settle_requests` 里）。但五张桌子**一起出借、一起收回**，而这正是它自己标题已经写着的那一句。理由换成出借，`pr` 于是入伙；否则它就是唯一一个被抛在值外面、靠人记得的桌子。`waiting`（`lent.pending()`，`u32`）同理：它只能在队列交给桌子**之前**数，数不到就永远数不到了。

**一个名字在相位内改了**：原来的局部 `shelf`（`Vec<Held>`）与 `memory_desk` 在归位值里叫 `shelf`，于是前者改叫 `held`——一个名字对一个东西，而“书架”指的是那张桌子。

**尺寸**：`dispatch_in` 495 → **423**；`open_desks` 76。十行的 `Desks` 手工构造（原在驱动之前）随之消失：归位值由相位自己交出来，不再由调用方拼。

**它以什么收口**：纯结构，无可咬的红。五张桌子的构造顺序、采钟的位置、`inboxes.remove` 与 `pending()` 的先后均逐字不变。143 条 `sprawling` 测试全绿，其中 `a_signal_one_run_sends_is_read_by_the_run_that_pulls_it` 与 `a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke` 走的就是“队列借出去、再收回”这一支。

**第八次拆分：工作台（§5 步 6）。**

```rust
struct Workbench {
    catalog: Rc<RefCell<runtime::Catalog>>, bench: ToolBench,
    delegates: Rc<RefCell<collab::DelegateDesk>>,
}
impl RunWorker {
    fn lay_out_workbench(&mut self, site, desks, addr, depth, mode, budget, job_locator)
        -> Result<Workbench, AxError>;
    fn status_tool(&self, site, desks, addr, mode, budget, seen, delegates)
        -> Result<StatusTool, AxError>;
}
```

**`Workbench` 持目录而不持它渲出的 `tools`**：后者是前者的投影，两份都存就是同一件事的两个权威。`ToolBench` 路由一次调用，`Workbench` 是一次跑工作的那张台子——名字相邻而职责不同，差别写在 rustdoc 第一句。

**工具块需要的不是两个方法而是三个，这是量出来的**。原先写「工具块需要两个」（阅览室一个、剩下一个）；切完一量，`lay_out_workbench` **206 行**，越过将要执行的 200。纪律是「不为通过而放宽门」，于是再切一次。切在 `status`：它是十三件里**唯一一件要读 worker 治理状态（`governance.autonomy`）与目标登记册（`self.goals`）的工具**，也是唯一一件带活闭包的（`children` 读委派桌，因为一跑边跑边派活）。它回答的那个问题与周围不同：**这一跑对自己怎么交代**。

**两个局部变量随它走了**：`writable` 与 `neighbours` 原本只为 `status` 而算（`write_domain()` 本就在同一方法里被叫三次，多一次不改变任何东西），现在各自在 `status_tool` 内部算。

**尺寸**：`dispatch_in` 423 → **237**；`lay_out_workbench` 164；`status_tool` 51。

**它以什么收口**：纯结构，无可咬的红。十三件工具的**构造顺序与登记顺序逐字不变**——而登记顺序是缓存面的一部分（§8-27），prefix 字节一变即有测试当场发作。

**第九次拆分：冻结（§5 步 5）。**

```rust
impl Freezing<'_> {
    pub(super) fn freeze_plan(self, site: &Site, workbench: &Workbench, at: &Assignment, given: Given)
        -> Result<(RunPlan, runtime::handoff::Handoff), AxError>;
}
```

**两个值而不是一个新类型**：`RunPlan` 与 `Handoff` 类型不同、谁也不会认错，再包一层只是给元组取个名字。它们同属一相位的理由是读一遍就看得见的：prefix 为这份 plan 而装配并与它一同冻结，handoff 引的是 plan 自己的 `task_line`，而 job locator 两边都在。

**冻结的交接就是房间里那份**：房间的 `Handoff.md` 填过时（`city::handoff` 答 `Some`），它的原字节入 CAS 并排在 must-read 末尾，`overview`／`progress`／`next_step` 取 `city::handoff_sections` 读出的对应节，`context` 在派活来源与 transcript 地址之后接上该节。一节没写就写明「没有记录」（`NOT_RECORDED`），不再写「see the city roadmap」「resume from the job locator」这类指向别处的占位：那两句读起来像交接，其实一个字的信息也不带，而下一个读者会照着去找一份交接里从未提过的路线图。`overview` 在文件没写时仍取 plan 的 `task_line`，因为那是真事实。

**它以什么收口**：纯结构，无可咬的红。prefix 四段的装配顺序、must-read 的入列顺序（先 norms 后 job）均逐字不变；两者一变即有多条盯 prefix 字节与交接件内容的测试发作。143 条全绿。

**剩下四次拆分**（目标 <200，预计落在 ~160）：四个相位都还在 `dispatch_in` 里，四个都要切。

| 拆分 | 相位（§5 步） | 长度 | 归位值 |
|---|---|---|---|
| e | 规则／配置／选型／身份／租约（步 3） | ≈70 | `Site` |
| f | 五张 desk 的构造（步 3–4） | ≈75 | `Desks`（扩 `pr` 与 `waiting`） |
| g | catalog＋十三件工具＋bench（步 6） | ≈175 | `Workbench`（catalog、bench、delegates） |
| h | prefix＋RunPlan＋handoff（步 5） | ≈90 | `(RunPlan, Handoff)` |

**依赖序即执行序，而且依赖是真的**：工具块读十五个局部（`write_root`／`rules`／`config`／`building`／`who`／`depth`／`model` …），先有 `Site` 才能让 g 收得下参数，而不是把十五个形参排成一列。

**第六次拆分：站位（§5 步 3）。**

```rust
struct Site {
    building: city::Building, rules: city::BuildingRules, config: kernel::FrozenConfig,
    model: gateway::ModelEntry, adapter: Box<dyn Model + Send>,
    identity: city::Identity, who: String, run_id: RunId,
    lease: Option<storage::WorktreeLease>, write_root: PathBuf, branch: Option<String>,
}
impl RunWorker {
    fn stand_up(&mut self, addr: &Address, job: &Locator,
                task: &str, goal: &str, budget: kernel::BudgetCap) -> Result<Site, AxError>;
}
```

**一个值而不是三个，理由是时钟而不是口味**。这一相位读上去是三件事（规则与选型、身份与登记、检查点与租约），而它们在代码里互相咀合：租约要 `run_id` 与 `who`，而 `run_id` 在选型**之后**采钟。拆成三个方法就得把身份块提到选型之前，那会改变 `run_id` 的时间戳落在哪一步之后——而这是纯结构改动，行为需逐字不变。**一个采钟点的先后不是重构可以顺手改的东西**（ARCH §10：全库只有一个采样点，它采到的值进了账本）。于是相位按原序整体搬迁，归位值一个。

**`Site` 不收 `addr`**：`Address` 是 `dispatch_in` 的形参，它在相位之前就在，放进去就是同一个值的第二份，故它不在字段里。

**归位值在调用点以整值接住（`let mut site = …`）而不拆开**，理由是量出来的：拆开之后，剩下三个相位要从调用点接过去的名字共 **102 处引用、十一个名字**（`building` 23、`model` 18、`who` 17、`write_root` 11 …），即每个相位方法都得排一列十五个形参——而那正是归位值要消掉的东西。`Driven` 可以拆，因为它四个字段只在驱动之后被读一次；`Site` 不行，因为它要穿过剩下每一个相位。

**尺寸**：`dispatch_in` 548 → **495**；`stand_up` 92。

**它以什么收口**（照 §8-17／§8-27 的写法）：纯结构，无可咬的红。搬迁逐字，唯一的改动是 `&addr`／`&job` 从局部变成形参，且两者在相位内部的用法不变；采钟点的个数与先后不变（`run_id_for` 一次、`ensure_base` 一次）。143 条 `sprawling` 测试全绿，其中 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯租约这一支。不补前后都绿的测试冒充红转绿。

工具那一块原为 214 行，先把阅览室（`admit_reading_room`）切出去，余下 ≈175 才能装进一个合格方法。**这正是阀值取 200 的一个副作用**：它不允许把一堆东西搬到另一处冒充分解。

**尺寸门须等这五次拆分完成**：纪律是「不为通过而放宽门」，所以门不能先落地再给自己开例外。门限与单位由全库测量定下，数字就写在这里：单位是**生产函数**（以首个 `#[cfg(test)]` 截断），门限 **200 行**。依据：1646 个生产函数中位数 9、p90 为 37、p99 为 114；超过 200 的只有六个，而其中五个是数据与标记（`web::lang::phrase` 是译文表，属 §9 形状 6；`Settings`／`CityView`／`BuildingView`／`Root` 是 Dioxus 组件，函数体即标记），故这两类需在门里声明为数据。排掉它们，全库超阀的生产函数只剩 `dispatch_in` 一个，第二名 `serve` 为 233——它也在网内，这是故意的，把门开到 240 去放它过就是为通过而放宽门。而按**文件**计不行：任何诚实阀值都会在四个 crate 里同时点燃八处（800 行阀 → 8 个文件），那是工程而不是一道门。提交须带 `Verdict: user-approved`。

## 8-32 一座城的那一个写者，自己有个名字

```rust
fn spawn_worker(city_root, vault, vault_notice, log, views, to_clients, worker_desk)
    -> Result<std::thread::JoinHandle<()>, AxError>;
```

**为什么是它**：`serve` 233 行，是那道尺寸门报出的两个对象之一。三个相位里（存储与视图、写者线程、socket 配置与关城），线程那一段是唯一一段带着**自己的契约**的：账本在线程**里面**打开且永不离开（一座城只有一个写者，而这件事不靠约定靠类型），并且它带着一次**握手**：`ready_rx.recv()` 回来之前，没人能把 socket 架在一座没打开的城上。把握手包进方法里，返回的 `JoinHandle` 于是自带一句断言：拿到它，就意味着那个写者已经在跑。

**尺寸**：`serve` 233 → **162**；`spawn_worker` 91。

**它以什么收口**：纯结构，无可咬的红。线程体逐字搬迁；原先在 `serve` 里各自 `Arc::clone` 的 `worker_desk` 与 `to_clients` 改为在调用点克隆后传入，克隆的**个数与时机不变**。143 条 `sprawling` 测试全绿。

## 8-33 同一把键的第二次，不是第二件活

```rust
pub(crate) struct CommandDesk { waiting: Mutex<Waiting>, arrived: Condvar, closing: AtomicBool }
struct Waiting { queue: VecDeque<Posted>, keys: BTreeSet<IdemKey> }   // 队列与在途键同一把锁

impl CommandDesk {
    pub(crate) fn post(&self, command: wire::Command, reply: wire::Reply);
    fn wait(&self, patience: Duration) -> DeskWait<'_>;               // Command(Posted, Underway<'_>)
}

/// 正在被办的那一条命令的键，办完即释放（Drop）。
struct Underway<'desk> { desk: &'desk CommandDesk, key: Option<IdemKey> }
```

**「让 `ToolBench::seen` 从账本重建」量完是错的**。键由 `(run_id, 本次驱动内的位次, action)` 铸成（§8-28），而 `run_id_for(job, addr, clock.now())` 把采钟拌进了身份，**没有任何生产路径会用同一个 `run_id` 再驱动一次**：审批放行后接着干的那段活，是 `answer_approval` 重新 `dispatch_in` 出来的一次**新 run**（§8-25），`resume` 只验链并把丢了结果的调用关成 unknown（ARCH §5 末）。往 `ToolBench::seen` 里播种历史，播进去的键在那一层永远比不中——那是把可测的防御换成不可测的死代码，正是 §8-28 拒绝过的那件事。

**没人守的那道门在上一层，而它今天就在漏钱**。`wire::frames` 的模块文档写着「每一条改状态的 Command 都带 `IdemKey`……『双击两次开出两个 Run』在这个类型里拼不出来」，而 `run_command` 的每一条臂都用 `..` 把 `idem` 丢掉：全库没有一处读 `Command::idem()`（只有 `wire/tests/wire_contract.rs` 与 `web::reach` 的两条测试读它）。四个发送端却都是照「服务端会去重」写的——`web::app::dispatch_command` 铸 `addr|task`、`web::city_view::create_command` 铸 `addr`、`console::dispatch` 铸 `console:addr:task`、`acp_dispatch` 铸 `acp:addr:task`，同一次提交两次就是同一把键；`web::reach` 甚至有一条测试叫 `saving_twice_configures_once`，它断言的却只是两条命令的键相等，**「只配置一次」这半句今天由谁兑现，答案是没有人**。于是双击一次、编辑器超时重发一次、控制台重敲一行，都是两次全款的模型账单。

**规则住在桌子上，而不是住在 `handle` 里**。`CommandDesk` 是每一条命令在 socket 与写者之间必经的那一处，它本来就按键之外的理由扫过自己的队列（`interrupt_for` 找 Cancel／Steer）。判定复用 `kernel::dedup`——`kernel::gate` 自述「seen 集合是调用方的状态，kernel 只判成员关系」，这里就是那个调用方的第二个实例（第一个是 `ToolBench`）。两个集合不是两处权威：一个管**工具调用**，一个管**人递进来的命令**，主体不同。

**在途，而不是永远**。一把键从 `post` 起在途，到那条命令**办完**为止：`wait` 交出 `Posted` 时一并交出 `Underway`，写者循环让它活到那一条命令服务完毕，Drop 释放键。于是——

- 双击、传输重发、编辑器超时重试：第二帧在第一件活还没办完时到达，被丢掉，**一次派活一次账单**；
- 人看完结果、想再跑一遍同一件活：键早已不在途，第二次照常受理，**不会静默吞掉**；
- 集合大小由队列深度加一封顶，不随城的寿命增长，也不需要时钟或任何窗口常数。

**`interrupt_for` 消耗掉的那条命令也要释放键**，否则同一个 run 的第二次 Cancel（`web::live` 铸的是 `cancel-from-the-control-surface`，每个 run 一把定键）会被永远丢掉。删队列与删键在同一把锁里完成。

**被丢掉的那一帧不回话**：发送者要的那件事正在办，`Reply` 只承载拒绝，而这里没有拒绝可言。**留下的残余**：控制台里同一行敲两遍，第二遍在第一遍办完前无声消失。真要给它一句话，得在 `AxCode` 上开一个「已在办」的码并让 `post` 交回受理与否——那是另一件事，这里不动，条件是这件事真的绊到人。

**它以什么收口**：一条会咬的红。`a_repeat_of_a_command_already_underway_is_not_a_second_piece_of_work` 首跑即红——今天两帧都进队列，第二次 `wait` 交出第二条命令而不是 `Idle`；实现后转绿，并在同一条测试里证明另一半：办完之后同一把键再来照常受理。既有测试 `a_cancel_reaches_the_run_it_cancels_without_waiting_for_it_to_end` 原先给三条不同命令共用一把 `b"i"` 键（图省事的夹具，真实客户端不会这么铸），改为一条一把——它测的路由与优先级不变。

## 8-34 计划的投影（`accounting::plan_view`）

`PlanView` 与 `PlanReading` 住在 `accounting`，接口、理由与测试见 accounting-SPEC.md 8-6，这里不留第二份；`views` 与 `RunWorker.plan_holders` 从那里读。哪些记录会动计划见 8-76，锁外读盘见 8-100。

## 8-35 谁在追一个目标，谁替它派活（`RunWorker.pursuits`）

- **值住在工人身上，事实住在账本里。** `kernel::Pursuit` 由 `Delegator::root()` 铸出，而这座城里**唯一一处 `Delegator::root()` 就在 `RunWorker::over`**——于是「子代理不能让全城通宵干活」是一件关于代码的事实，而不是一条谁去遵守的规则。设置／暂停／恢复／清除各落一条 `pursuit_changed`，`Views` 折它来画，重启后工人从同一批记录把值重新铸出来。两处折叠都经 `Payload::read::<kernel::event::record::PursuitChanged>` 与它的 `held` 读这一行，读不回的一行让折叠报错而不是被跳过：跳过它，一座被清除目标的楼在重启后会继续追下去。
- **`Views` 不持 `Pursuit`，只持文本与状态**：一个能铸出 `Pursuit` 的视图，就是那道守卫上的第二扇门。判定仍由 `kernel::observe_pursuit` 给出，措辞由 `verdict_line` 一处写出——页面、控制台与日志说同一句话。
- **`pursue` 会终止，理由在集合上而不在计数器上**：认领把节点移出就绪集，而一个结束时还持有节点的 run 会把它留成 Blocked（`ClaimDesk::abandon`），所以就绪集严格变小；唯一让它变大的是拆分，而那是这座城找到了更多活，不是在打转。派活之后若该节点仍在就绪集里，追求暂停并留一条诊断——**看的是集合本身，不是一个凭空定的上限。**
- **值住在工人身上，事实住在账本里。** `kernel::Pursuit` 由 `Delegator::root()` 铸出，而这座城里**唯一一处 `Delegator::root()` 就在 `RunWorker::over`**——于是「子代理不能让全城通宵干活」是一件关于代码的事实，而不是一条谁去遵守的规则。设置／暂停／恢复／清除各落一条 `pursuit_changed`，`Views` 折它来画，重启后工人从同一批记录把值重新铸出来。
- **`Views` 不持 `Pursuit`，只持文本与状态**：一个能铸出 `Pursuit` 的视图，就是那道守卫上的第二扇门。判定仍由 `kernel::pursuit::observe` 给出，并以 `kernel::PursuitVerdict` 原样放进 `PursuitLine.verdict`；城不替它写句子，人读的措辞只在客户端的 `lang.json` 里按种类取。
- **`pursue` 会终止，理由在集合上而不在计数器上**：认领把节点移出就绪集，而一个结束时还持有节点的 run 会把它留成 Blocked（`ClaimDesk::abandon`），所以就绪集严格变小；唯一让它变大的是拆分，而那是这座城找到了更多活，不是在打转。派活之后若该节点仍在就绪集里，循环停下并留一条诊断——**看的是集合本身，不是一个凭空定的上限。**

## 8-36 一个节点红了，站在它后面的人会知道（`tell_whoever_is_behind`）

- **由事实触发的交流。** 这座城里居民互相够到彼此的每一种方式，都从「有人决定要说话」开始；这一种从「一个节点红了」开始，并且恰好够到那些手上的活现在动不了的房间。
- **信号 id 由楼与节点推出**（`blocked-<building>-<node>`）：同一处卡住宣布两次是同一条信号，信箱按 id 去重。一个房间为一个问题被通知四次，是一个会停止读信箱的房间。
- **「谁持有哪个节点」只有一份**，折在 `CollaborationFold.plan_holders` 里：认领那条记录的 `addr` 就是房间，所以这里不推导任何别人已经写下的东西。`plan_view` 不再持第二份——它只画，不派信。

## 8.5 两个设计

**A（选中）**：`build.rs` 把 `just build-web` 留在工作区 `target/web-dist`（`BUNDLE_DIR`，8-83）的客户端包压缩进 OUT_DIR，再由 `include_bytes!` 嵌入——嵌入只有一处。**B（落选）**：`include_bytes!` 直指源码树里的一个目录——少一步拷贝，但把「产物在哪」写死进源码路径，换产物就要改代码，且没有 `rerun-if-changed` 的粒度。

## 9 工作流程

`main` 经 `main::grammar` 把 argv 读成一次调用（8-89），`main::router` 分派给动词；serve 的次序见 8-88，一次派活见 8-2 与 8-113。

## 10 实现逻辑

`build.rs` 的失败经 `cargo::error=` 显性报出（cargo ≥ 1.84 语法），而不是留下一份过期或缺失的资产让 `include_bytes!` 去撞。

## 11 边界枚举

客户端包缺失在构建期即红，不是运行期的意外（8-83）。命令行读不懂退 2（8-103）。

## 12 Decisions

**测量读数不进 Ledger，也不常驻城里的盘；时长只取单调钟**（8-129）。一件事发生的时刻是城的历史，写在每行的 `t` 里，含义由 `EventRecord::moment` 给出；主机为城的一段工作花了多久、机器的资源被用了多少，是运行这座城的主机在那一刻的事实：前者写成诊断行，后者只在有人看时进监视器的 300 点历史，要留下来就由看的人（`sprawling gauge`）写到它自己的 stdout。时长一律是 `bin::serving::standing::monotonic_now` 两次读数之差，城钟只用来给账本行打时刻。**被否掉的**：资源读数作为一个整数单位的新事件种类写进 Ledger——每秒一行都要过记账线程的屏障，整链审计、重放与每个折叠随之变长，而进了哈希链的读数删不掉；城在保留子树里常驻写一份读数文件——城树多一个写者，要自带保留与轮转，没人看的城也一直付开销。条件变了就重议：城里有一个决定要读资源的历史，而不只是此刻的读数。

**`replay <ledger-dir>`：「这里没有账本」不得与「验过且为空」同形**。本子命令的路径是人敲的，故它先问 `storage::ledger_segments_at`，一段都没有即报 `E_PATH_NOT_FOUND` 并给 recovery，不进验链。有段时经 `storage::audit_chain` 一段一段地验，每行过同一个 `LineCheck`，常驻一段；它答的只是行数与 tail seq（`chain verified: <n> line(s), tail seq <n-1 或 none>`，`main::tests` 的 `replay_names_the_lines_it_verified_and_the_tail_seq` 钉住这个形状），用不着 `VerifiedLedger` 那份整本的原始行与记录；断链照旧是那一行的三段式拒绝。**依据为什么在这一层而不在 `runtime::replay` 或 `storage::audit_chain`**：它们的生产调用方都自持城根算出路径，而已开未写的城就是一个无段目录（`JsonlLedger::open` 只建目录），在那一层报错会把合法启动打红，并迫使每个调用方各写一份相同的守卫。空账本仍然合法，故问的是「有没有段」而不是「有没有行」。参见 runtime-SPEC §8-1、storage-SPEC §8。

**开城不等整条链的证明，接受命令等**（8-122）。首字节之前只读尾部与两份快照，整条链在写者起好之后由后台证明，证明完成之前写者拒绝每一次追加（`E_HISTORY_UNPROVEN`），查询按从快照起步的视图作答。**被否：轻量审计，封好的段只取 `prev`、`seq` 并哈希整行。** 它不再证明每一行都能被这个构建逐字节规范地写出来，与 `LineCheck` 不等价，而开城之后读这些行的正是按 `LineCheck` 解析它们的视图与索引；已验证前缀记录让「只哈希」只用在已经逐行核对过、字节没有变的段上（storage-SPEC 8-30）。**被否：证明完成之前接受命令、断链再停写。** 那是 8-101 已被否的做法：写下的行接在一条可能断了的链后面，收不回来。
**开城变快靠少做重复的事，不靠放松保证**（8-144）。末段照样逐字节哈希、与记录比对，记录之后的行照样逐行核对；第二份视图是同一个状态的克隆；重建从创世时由全量折叠逐行核对，不再先证明一遍。**被否：开账本信任末段，只核对记录之后的字节、不哈希前缀。** 省下 23 ms，却接受了段被截短再接写、或被同长改写的那种意外，而 8-30 的记录正是为挡它才带摘要。**被否：把复制第二份视图挪到视图线程上，首字节不等它。** 首字节确实不等了，但视图线程的第一批等它，装配根起线程的次序也要改；克隆把这一段从 350 ms 降到 22 ms，不需要挪。

**开城之后的证明在一个库 crate 里起线程。** 证明按波读段（storage-SPEC 8-37）的作用域线程起在 `storage::chain_audit` 里，不在装配根：一波的线程只读段、哈希前缀，一波 join 完才往下走，它们的寿命被一次 `prove_chain` 包住，与 `runtime::turn::wave::reorder` 的作用域线程同一类（ARCHITECTURE.md §10 第 3 条）。装配根仍只起那一条证明线程（`bin::assembly::chain_watch`）。**被否：在 `chain_watch` 里按段起线程、把各段的摘要交给 storage。** 前缀摘要与记录的判定就要分在两个 crate，`SegmentRecord` 也得公开；开账本（storage-SPEC 8-34）判末段用的是同一个判定，它不经过 `chain_watch`。**重开参数**：证明要与别的后台工作共用一个有上限的线程池时。

**doctor 的「能做什么」各面自持：终端的英文住需求表，页面的两种语言住 `lang.json`，线上只携 id**（wire-SPEC §8-25）。`Requirement::enables` 是 `sprawling doctor` 那份终端报告的措辞，终端只说英文，这句话与它描述的那一行同住 `bin::doctor::table`，改一行的人在同一处看见它；页面按 `DoctorItem::name` 从 `client/src/lang.json` 的 `machine_enables_<name>`（name 里的连字符写成 `_`，因为词表的键一律 snake_case）取 en 与 zh，与 doctor 其余每一种状态同口径——终端的 `absent` 由 `paint` 拼，页面的 `machine_absent` 由 `lang.json` 给，两面各说各的，线上只携枚举与 id。两面的集合由 `bin::doctor::tests` 的 `every_item_has_the_page_clause_in_both_languages` 钉在一起：需求表多一行而 `lang.json` 没给它词，测试红。**被否掉的**：终端也从 `lang.json` 取英文（二进制在编译期读 `client/` 的一份源文件，每次 doctor 都要解析整张词表，而终端从不说第二种语言）；线上继续携英文句子（页面的词成了服务端的选择，中文读者看到的是英文）。条件变了就重议：终端要说第二种语言时，两面合用一张词表。

**城的工具读别楼的文件与 `cas:` 块，只经 runtime 的一扇门**（8-131、8-142，runtime-SPEC §8-59、§12.16）。`ocr` 与 `transcribe` 的字节都由 `runtime::BoundReader` 读：它把 `read` 的那一份判定（换成城里的拼写、文法、reserved subtree、读界、解开链接再判、打开后核对）公开成一个调用，交回一个 `Read` 与字节的来处。`transcribe` 原先只收本楼的文件，用 `Address::is_within`、`is_reserved` 与 `WriteTarget::within` 自己判，那是这份判定在 accounting 里的替身，门落地后删去。**被否掉的**：两件工具各自在 accounting 里判（两份判定，一份跟着 `read` 变，一份不跟）；只为 `cas:` 另开一条路、文件仍只收本楼（连接器存下的截图与录音能读，读界之内别楼的文件却不能，同一个读界有两个答案）。

**一条命令的戳从驱动最近的读数渲染，不给工具面一个钟**（8-125，runtime-SPEC §12.8）。`drive_run` 把它交给驱动的 `now` 包一层，每个读数先记进这一跑的 `ClockReading`；`Sieving` 打包时读最新的那个。回合在调用工具面的 `account` 之前读答复时刻，所以最新的那个就是这条命令的答复时刻，戳上的秒数等于它 `tool_result` 的 `t`；一跑仍只有一个采样点，计数时钟下的剧本字节不变。**被否掉的**：`Sieving` 打包时自己读 `hands.clock`——一跑多了一个采样点，戳与它 `tool_result` 的 `t` 可以差一秒；把戳挪进回合的 `account`——那是 runtime 的 `turn` 的事，不是装配层的，而读数的位置已经给出同一个秒数。

**Lean 是开发这份代码必需的工具**（§8-58）。各 crate 的规格正从 `<crate>-SPEC.md` 迁成 `Spec.lean`（ARCHITECTURE.md §11「Specifications in Lean」），`just check` 里的 `models` 一步是这些规格在本地被证明过的唯一证据。Lean 列为可选时，没装 Lean 的机器上 `models` 静默通过，本地的绿就不再说明规格被证明过，只有 CI 知道。所以 `elan` 与 `lean` 两行是 `required`：缺了它们，`just prereqs` 在编译之前报出来并给出装法，`just models` 自己也报错而不是跳过。被否决的备选：保持可选、只靠 CI 的 `models` job 证明——那样每次本地验证都得另外说明「规格没有证过」，而这句话没有哪道门会替人说。

**Zig 是在 Windows 上开发这份代码必需的工具**（8-146）。桌面 server 没有准入安全接口的四组 Win32 调用经一片 Zig 叶子（`crates/desktop/Spec.lean` D12），叶子在构建时编译，所以 Windows 上没有 Zig 就编不出这个二进制。`zig` 一行因此是 `required`，版本只读 `crates/desktop/ffi/zig-version`。被否决的备选：把 Zig 叶子预编译成一个提交进树里的静态库——那是一份没人能从源码复现的二进制，`release` 的逐字节重建也就无从谈起。

**崩溃验收在盘上造死亡，不在进程里杀**（8-127）。一次 run 真跑完，丢掉 worker 释放写者锁，再把账截在一行的半途、删去其后的行；重开走 `RunWorker::new` 与 `startup_scan`，与 `sprawling resume` 同一条路。理由：被杀的进程留在盘上的就是这样一份账——锁已释放，最后一行写了一半——而盘上的截法可以精确指定死在哪一行的哪一个字节，每次重跑都是同一次死亡。被否：①起真二进制再杀掉它——那是从外面进城的检查，按边界规则归 `tools/adversary/` 的 Lean 黑盒（G1e），而且杀在哪一刻取决于调度，失败不能逐字节重演；②把账本放在 `storage` 的故障文件系统上断电——它只承载账本，`Standing::fold` 与视图读真目录，重开的不是一座完整的城，断电的耐久契约已由 storage 自己的测试证过。重开参数：账本之外的文件（检查点、快照、CAS）也要在一次死亡里与账本错开时，验收要能在同一刻截断它们，那时改为在 `Vfs` 缝之上承载整座城。

**死掉的 run 在重开时冻结，验收比它的整行状态**（8-127）。启动扫描补完悬空调用之后为死掉的 run 写冻结，验收第 4 项比城景里那次 run 的冻结、结局、最后一行与序号，并读账上那一行的 `cause`。理由：ARCHITECTURE §13.7 画的 `Lost --> Frozen` 是账本上的一行，服务中的城与重开的城读同一份账本，必须对这次 run 说同一个结局；只比最后一行的种类，看不出视图把它当成仍在跑。被否：验收只比 `last_kind` 与 `last_seq`、冻结留给以后——那是在把「死掉的 run 永远未冻结」钉成规则；在验收里自己补写冻结——验收就成了它要判的那段代码的第二份。重开参数：`serve` 不经 `resume` 也要收拾死亡时（accounting-SPEC §12 第 30 条），验收改为经 `serve` 的起步路径重开。

**不从别的工具的配置里读 provider 表（人的决定）**。`bin::import` 的五个文件读 Codex 的 `~/.codex/config.toml` 与 pi 的 `models.json`，把其中的 provider 折成本城的词汇；但没有任何 `mod` 声明过它们，所以它们从没被编译，clippy 与测试也从没看过它们，模块图却把它们记为 built。本城删去这五个文件，首次上手的第 1 步由人手填端点，或选一个已知主机。理由：没有调用点的代码是一份没人维护的第二文法，它记下的 Codex 与 pi 键名会随上游改版静默过时。**被否**：接上它，作为首次上手第 1 步「从别的工具已写好的配置读入」的候选来源——那要在 wire 上加一条 Query、在设置页加一行，属于线协议与页面的改动，本版不做。**重开参数**：首次上手要给出「别的工具已配置的 provider」这一步时，从 git 历史取回这两种文法，先在 `lib.rs` 声明模块，让测试与 clippy 看见它们，再接 Query。

**远程门的五行经 worker 的 relay 写，不经命令桌**。远程门在装配层，而城的唯一写者在 `accounting::worker`；本城给 `RunWorker` 一个公开的 `relay()`，交出驾驶线程用的那一种 `kernel::Ledger`，门的看守拿它写 `remote_opened` 等五行（§8-139）。理由：relay 本就是「一个不是写者的线程把一行交给写者并等它落盘」的那扇门，答回来时这一行已经持久，门的下一步可以靠它；城仍只有一个写者。**被否**：①给 `wire::Command` 加五个没有线上形式的命令（如 `PutSecret`），由 `run_command` 落账：每一个都要进 `COMMAND_NAMES`、§19-2 的 reach 与 class、控制台的投影与客户端的生成类型，五个只为写一行而存在的动词散进六处；②让门的看守自己开一个账本写者：一座城就有了两个写者。**重开参数**：门需要等一行落盘之外的答复（例如写者先判这一步合不合规矩）时，改成一条桌上的进程内命令。

**居民判一次，harness 走第二个驱动函数**。`agree_to_work` 得出 `Seat` 枚举，模型一臂带着原来的 `Agreed`，harness 一臂带着楼、规则与那一家；`Staged::fly` 按它分到 `drive_run` 或 `drive_harness`，`land` 按它分到原来的落地或 harness 的落地。理由：`Driving` 的十二个字段里，适配器、工作台、sieve、计划与 bench 只有模型用得到，`Driven` 带回的适配器、工作台与冻结的 `Run<Frozen>` 也是；把居民枚举放进 `Driving`，`drive_run` 开头就要 match 一次，结果还是两条路，而每个只有模型才有的字段都要变成 `Option`。**被否**：`Driving` 里的居民枚举（同一个判断在 `Driving`、`Driven`、`Site`、`land` 四处各 match 一次）；在 bench 上立一件「harness」工具（§8-4e 第 8 条已否）。**重开参数**：harness 的 run 开始有第二个回合、或开始用城的工具（MCP 转交）时，两条路共享的部分会变大，那时重议。

**试验借一棵自己的树，而不是在合并时才拦**。落地策略为 `experiment` 的 run 不论楼要不要评审都在房间的工作树里写（§8-133），合并时 `runtime::admits` 再拒一次。理由：没有评审的楼里一次 run 直接写进楼的文件，到合并那一步时已经没有东西可拦——试验的「什么都不落地」只有在写的位置上才守得住；把它放在树上，楼的文件从头到尾不变，而试验的产出仍在分支上，读得到、比得出。**被否**：只在合并时拒（没有评审的楼里试验照样改了楼）；试验 run 结束时把它改过的文件还原（还原之前别的 run 与人已经读到了改动，而且要为每一种写路径各写一种撤销）。**重开参数**：工作树放置的读数（8-31 的 `[large_worktree_placement]`）大到让一次小试验的开销以秒计时，重议「小试验用副本、不借树」。

**补备树放在 lane 里、run 驾驶完之后，不在记账线程上，也不另起一条线程**（8-145）。放置的秒数是一次全量检出等实时扫描逐个放行新建文件（storage-SPEC 8-35）；把检出挪进备树，放置只剩改名，而补备树本身仍是一次全量检出，它必须落在没人等的地方。lane 在 run 驾驶完之后、交回 `Flown` 之前正是这样的地方：run 的最后一条记录已经写下、已经广播，只有同一房间的下一次派活要多等它一次。**被否**：在 `land` 里补——`land` 在记账线程上，记账线程等一次检出，每条 lane 的 append 都排在它后面（8-113）；开城时补——一次全量检出加进首字节（8-122）；为补树另起一条后台线程——库 crate 与 `sprawling` 起线程的地方都是 ARCHITECTURE §10 第 3 条点名的，补树不值一条新线程，lane 已经在那里。**重开参数**：同一房间的派活常常紧接着上一次落地、使补树的那一次等待被人看见时，改成由 `DrivingPool` 在空闲的 lane 上补。

**`view` 在终端前给每一行标上它的链哈希，不加字段，也不给 agent 看**（8-105、8-117）。人要一个能记下来、以后拿来对照某一行的值，账本每一行已经有一个：`kernel::ledger::chain_hash` 对这一行规范字节算出的 BLAKE3，下一行的 `prev` 存的就是它，它覆盖整行，`t` 与载荷都在内。在载荷里或旁边再存一份同源的哈希，就是同一件事有两个权威。哈希从盘上的字节现算：一行一次 BLAKE3，比解析这一行便宜，交互界面又只给屏上的行算，四十万行的账本也不多付。谁要哈希仍按 8-105 决定 1 的 TTY 规则分：管道与文件里仍是账本原行，agent 解析的字节不变。哈希放在行前而不是行后，因为定宽的一列在终端折行后仍然对齐，而交互界面按栏宽截行，行后的哈希根本画不出来；列表只放前 12 位，因为 64 位会在半屏宽的列表里挤掉整行，完整的值在详情栏第一行。12 位是 48 bit，四十万行里出现一对同前缀的行的机会约为万分之三，所以前缀只用来凭眼睛找行，要记下来就用完整的值。**被否掉的**：每行后面接哈希，理由见上；`Row` 在折叠时就带上哈希，整遍折叠要给每一行多算一次、多存 32 字节，而人一次只看一屏；加一个 `--hash` 参数，人坐在终端前就要哈希，参数只是让人多敲一次。条件变了就重议：常见的 agent 宿主改在伪终端里跑命令、并解析 `view` 的输出时，TTY 就分不开两个主人，那时改为显式参数。

**`view --since/--until` 按每一行信封的 `t` 流式过滤，不建时间索引**（8-137）。时间窗读的是信封 `t`，因为它就是这一行记下的那一刻，任何种类都有它；按种类去载荷里挑时间字段，就是同一件事有两个家。不二分也不提前停：`t` 不随 `seq` 单调，在第一条越过 `until` 的行处停下会漏掉后面回退的行。**被否掉的**：在 `LedgerIndex` 里给每行多存一个 8 字节的时间列再二分——它要 `t` 有序，而 `t` 无序；也让索引多一份要维护的事实。条件变了就重议：四十万行的账本上 `view --since` 超过一秒，或页面需要按时间跳转。

**`whose --trace` 读盘作答，候选就说是候选**（8-136，accounting-SPEC.md §12-28）。区间以同一个 run 的上一个提交为界，调用经 rounds 的那一份配对规则读出，同楼别的 run 只给条数。CLI 读盘而不走线上查询，城不在服务也答得出，与 `whose` 本身同一条理由。**被否掉的**：只列写调用——读调用决定了写什么；把同楼别人的调用并列出来——读者会把候选读成原因。条件变了就重议：页面要显示一个提交的调用时。

**`playback` 是一个两个词的动词，导出写 stdout 或一个新文件，只在 bundle 完整之后写**（8-126、accounting-SPEC.md 8-12）。回看一段工作流有两个动作：导出与复核，它们的标志不同（导出读选择与 `--out`，复核读 `--bundle`/`--city`），所以是两行；放在一个词 `playback` 之下，是因为总览里两者挨着，人找到一个就找到另一个。命令表的一行可以带两个词，解析先试两个词，其余不变，8-89 决定 1「动词需要子动词」的重议条件因此被这一个最小的扩展满足，没有换参数库。输出不沿用 `view` 的做法：`view | head` 截断仍算成功，是因为账本原行本来就一行一行有意义；一份截断的 bundle 读不回来，却可能被当成一份完整的东西留下，所以 bundle 先整份算好、量过尺寸再写，管道中途关闭是失败，`--out` 经暂存文件与硬链接落位、已有目标不覆盖。**被否掉的**：`export --playback` 之类挂在现有动词上的标志（`export` 打包整座城，两件事的输入与输出都不同）；`--out` 默认写进城里（导出件不进 git，城里的保留导出位置由布局 owner 在居民入口落地时定）；`rename` 落位（在 Unix 上会覆盖已有目标）。条件变了就重议：人也要一个不必自己选路径的去处时，`--out` 缺省可以写到城里的保留导出位置（8-132 给居民的那一处），而不是 stdout。

**居民的 `playback` 是一件工具、两个动作，按写登记，读者由上下文定**（8-132、accounting-SPEC.md 8-13）。导出要写文件，复核只读；工具的效应按登记而不是按调用，一件工具只能登记一种，所以取两者中较强的写：代价是复核也走写门、不被推测执行提前跑，而一次复核本来要重算整段历史，提前跑它省不下什么。导出件落在城根保留子树下的 `playback/<楼>/`，而不是居民的房间或 worktree：保留子树没有写域够得到，居民改不了一份已经写下的报告，只能另导出一份；它被城根的 `.gitignore` 挡在历史之外；它不随 worktree 清扫消失，没有 worktree 的 run 也有同一个去处。按楼分目录，是因为复核要用同一个读者重算，一栋楼的导出只有同一栋楼的居民复核得了，机密楼的导出也就不会被别的楼读到。**被否掉的**：`playback_export` 与 `playback_check` 两件工具（复核可以按 `Read` 登记，但一个动作拆成两件工具，模型要多认一个名字，而两者的参数一半相同）；写进房间并靠 `.gitignore` 挡住（worktree 会被清扫，居民能用 `edit` 改报告）；让居民在参数里选读者（读者是读界的输入，交给被读界约束的一方去选，读界就没有意义）。条件变了就重议：工具的效应可以按调用声明时，`check` 改回 `Read`。

**playback 的时间条件是两扇门各读三个原文、交给同一个函数**（8-143；accounting-SPEC.md 8-17 与 §12 第 29 条）。`--since`、`--until`、`--day` 与城工具的 `since`、`until`、`day` 都只是原文，`accounting::playback::Window::span` 一处把它们读成一个 `UtcSpan`；这样人与居民导出同一天时，`source.selection` 记下同样的两端，复核也按同一个区间重算。`--day` 不进 `view`：`view` 是看行的透镜，`--since/--until` 已经够它用，多一个旗标就多一种写法要维护。**被否掉的**：CLI 复用 `view` 的 `--since/--until` 读法而城工具另写一份（两扇门读同一个条件就会有两处）；`--day` 与 `--since/--until` 互斥（「这一天九点以后」是一个合法的问题，交集就是它）。条件变了就重议：人要按本地日期回看时，那时要一个时区，而城今天只认 UTC（runtime-SPEC 8-10）。


## 13 依赖选型

依赖以 `crates/sprawling/Cargo.toml` 为准；每个依赖旁的注释写它为哪一节而在。

## 14 硬编码声明

客户端包的位置 `target/web-dist`，相对工作区根，由 `build.rs` 的 `BUNDLE_DIR` 一处声明（§8-83）。

`bin::install` 引入四处，全部是外部世界的事实而非我们的选择，故各自注明出处：`%LOCALAPPDATA%\Programs\<app>` 是 Windows 用户级程序目录的约定；`~/.local` 下的 `bin` 是 XDG 用户级可执行目录的约定；`HKCU\Environment` 是用户级环境变量在注册表里的位置；`WM_SETTINGCHANGE=0x1A`／`HWND_BROADCAST=0xffff`／`SMTO_ABORTIFHUNG=2` 是 Win32 的常量值。这四处一旦被平台改掉，改点各只有一个。

## 15 影响面

`just build-web` 产出客户端包，`build.rs` 读它（8-83）；交付形态见 §18。

## 16 测试与约束

workspace lints 全量适用（含 build.rs）；各节写出钉住它的测试。

## 17 模型体验

模型读到的前缀由 `accounting::worker::freezing` 组装，四段全部入内容仓库（8-12、8-67、8-85）。

## 18 文档同步

每加一个动词：本 SPEC 增一节，`architecture.toml` 登记它的模块。

交付形态入册：`just package` 的产物名、`QUICKSTART.md`、README 与 `docs/getting-started.md` 的首次运行段、`release.yml` 的附件清单，五处同改。

## 8-37 每个问题的答案住在读面（`accounting::views`）

`Views` 折账本、答每一个 `Query`，删掉重折得到同样的字节——那是 `projection`（ARCHITECTURE.md §9 的形状 7），而 `bin::assembly` 是 `adapter`：装配点、唯一全知。一个模块里两个形状，就是它们分家的依据。`views` 是 `assembly` 的兄弟模块而不是 `assembly/` 下的子目录，因为 `Views` 是投影，不是 `RunWorker` 的一部分；两者之间的依赖方向见 8-92。

测试跟着被测的东西走：`views` 的断言住在 `views` 下，一份留在原处的断言会让下一个人以为那里还有代码。

## 8-38 一座城怎么被端上来，与一轮活怎么跑完（`bin::serving`）

「一座城怎么被端上来」与「一轮活怎么跑完」不是同一件事，所以前者住 `bin::serving`，后者住 `RunWorker`。serving 持有门口那把钥匙（`serving::door` 的 `key_for` 与 `random_token`——熵在本 crate 只有这一处）、金库的开启（`open_vault`）与一次 serve 由调用方填好的那个值（`serve::Serving`）；写者线程、命令台与 lane 属于装配点（8-92）。

写者线程由 `assembly::attending::spawn_worker(opening: Opening, outward: Outward)` 开，账本在它里面打开且从不离开。两个参数各是一个值：`Opening`（一个写者是用什么打开的：城根、金库、金库探测的发现、日志、serve 线程在写锁下已经折好的账本与 `Standing`、审计的日志、核心线程的档位）与 `Outward`（它的活从哪来、结果到哪去：命令台、发布出去的视图与备用的一份、给页面与观察者的广播、账本头、还在跑的命令的输出）。两个值代替一长串参数，没有 `#[expect(clippy::too_many_arguments)]`。

`CITY_VERIFIER`（`accounting::worker::workbench`）与 `local_model_facts`（`accounting::worker::credentials`）住在 `assembly`，因为它们是 `RunWorker` 在一轮活里用的东西，不是端城用的；执行引擎由 `doctor::host::execution_engine` 按平台选出。

## 8-39 装配点成为一棵模块树，十五条签名被消掉（`bin::assembly::*`）

`bin::assembly` 10,695 → 一棵树，每个文件在 1000 行以内，`[file_length.predating]` 的最后一行被划掉。

### 为什么是子模块，不是兄弟模块

`RunWorker` 22 个私有字段。**子模块看得见父模块的私有项**——Rust Reference 的 *Visibility and Privacy*：
"If an item is private, it may be accessed by the current module and its descendants"。
于是 `RunWorker` 的定义留在 `assembly.rs`，`impl RunWorker` 的方法散进 `assembly/*.rs`，
**可见性一个字不动**：crate 里 `assembly` 之外的任何模块看到的仍是今天那张脸。
兄弟模块做不到这件事，§8-37 与 §8-38 因此付了 `pub(crate)` 的价；这里不付。

### 缝在哪：先量再切

切缝取自一次 LCOM 测量（66 个方法对 21 个字段的接触矩阵），不取行数。读数：
`city_root` 被 21 个方法碰，`ledger` 与 `governance` 各 8，`inboxes` 5，`vault` 4，**其余 15 个字段各 ≤ 2 且成簇不交叉**：

| 簇 | 碰它的方法 | 落到 |
|---|---|---|
| `vault` | `put_secret`／`resolver` | `accounting::worker::credentials` |
| `inboxes`／`joins`／`requests`／`goals` | `lay_out_workbench`／`open_desks`／`settle_desks`／`settle`／`deliver_handback` | `accounting::worker::workbench`＋`accounting::worker::settling` |
| `pursuits`／`delegator` | `set_pursuit`／`pursue` | `accounting::worker::commanding` |
| `interrupts`／`watching` | `attach_interrupts`／`watch`／`drive_dispatch` | `assembly.rs`（装的两个钩子）＋`accounting::worker::driving` |
| `knocks` | `knock`／`answer_knocks` | `accounting::worker::dispatching` |

**这份读数说的是 `RunWorker` 是五个类**，而这里只把它们搬进各自的文件、让边界看得见；
把它们变成真的对象要先分开「判定」与「记账」（每个簇的方法都在 `self.record(...)` 写账本），那是 ARCHITECTURE §5 的
invert the model seam，仍未动手。**这里不假装做过它。**

### 门给这次拆分定的价：十五条签名必须被修好

`tools/xtask/src/length.rs` 的豁免键是 `路径::函数名`，而 `guard::strikes_only_exemptions` 的 rustdoc 写死了
"an over-long signature may be fixed or left alone, never relocated with its excuse"。
`assembly.rs` 里有十五条超标签名，**它们随文件搬家就失去豁免**，所以逐条消掉。
消法是同一条：**总在一起走、从不被单独选择的值，是一个还没有名字的值**（`Reporter` 的 doc 写下的先例）。

| 新值 | 它是什么 | 消掉了 |
|---|---|---|
| `Assignment` | 一次派活是什么：地址、模式、天花板、谁把它交下来（深度由它推出，不再第二次传） | `dispatch_in` 6→3、`lay_out_workbench` 7→4、`stand_up` 5→2、`settle` 5→3 |
| `Given` | 这一轮活被给了什么：brief、task、goal，与那份字节的 pin | `freeze_plan` 9→4 |
| `Driving` | 一次 drive 跑在什么上面：适配器、工作台、信号桌、写根、检查点域、身份 | `drive_dispatch` 9→3 |
| `Ending` | 一次 drive 以什么结束：结局、被抬起来的审批、代表桌 | `conclude` 10→3 |
| `Sweep` | drive 之后要收的东西：检查点、被抬起来的审批、job locator | `settle_desks` 11→4 |
| `Reach` | 这一轮活够得到谁：邻里与代表 | `status_tool` 7→4 |
| `Entered` | 一个人为接一个 endpoint 输入了什么：名字、base URL、兼容格式、凭证（`Credential` 枚举，不是「密钥＋鉴权头」两个 `Option`） | `endpoint_of` 5→1、`probe_endpoint` 5→1、`attach_endpoint` 6→2 |
| `Ceilings` | 一行模型声明的两个上限：上下文与最大输出（后者 `Option<Ceiling>`；人没填就沿用这个模型上一次登记的值，再退回目录行，见 §8-71） | `select_model` 5→4 |

`record_for` 的五参消得不需要新类型：`effect::Line` 已经装着 `who`／`addr`／`kind`／`data`，
调用点原本就在把它拆开再递进去，改成整份递。
`settle_requests` 与 `settle_desks` 另外收掉四个参数，因为 `who`／`run_id`／`write_root`／`building`
**本来就是 `Site` 的字段**，调用点在一个一个地从 `site` 里取出来递；`checkpoint_scope` 成为 `Site` 上的方法，
于是「检查点落在楼上还是落在房间上」在本模块只有一个答案。
三处 `#[expect(clippy::too_many_arguments)]` 随之报「这条压制没有被用到」而自己清掉——**修好之后压制自己消失，正是它该有的形状**。

### 十六个子模块，与两次为了行数之外的理由再切的缝

`assembly.rs` 756 行，十六个子模块各在 1000 行以内。其中两次是重新分配时切的，而切缝仍取自形状：
`settling` 里 `settle_requests` 回答的是「一个不能直接写的楼怎么收下这次改动」，那是评审与合并，成 `reviewing`；
`dispatching` 里 `wake`／`knock`／`answer_knocks` 回答的是「一个没在干活的居民怎么被叫起来」，成 `waking`。
`configure_building`／`create_building`／`adopt_building`／`startup_scan` 从动词表挪进 `genesis`：
**`form_city` 本来就在调 `adopt_building`**，一座城怎么长出楼、重启后看见什么，和一个人发一个动词不是一件事。

### 测试跟着它咬的那个模块，一份夹具留在父模块

`mod tests` 5,576 行，一百个测试函数。**先量后放**：3,490 行只用这个 crate 已经公开的面，
本来可以按 `assembly_door.rs` 的先例去 `crates/sprawling/tests/`。**没有那样做，理由是夹具。**
`fake_openai`（一台按脚本作答的 OpenAI 服务器）、`worker_with_provider`、`completion` 这一簇 443 行，
被两边同时需要：`what_a_worker_holds_is_what_a_restart_rebuilds` 要用它造一段历史再去核 `Standing::fold`，
而 `tests/` 里的验收测试也要用它。**`#[cfg(test)]` 的东西到不了 `tests/`，`tests/` 的东西到不了 `src/`**——
分家就要养两份同名夹具，那是一个夹具两个权威。

所以整套留在 `src/`：夹具成为 `accounting::worker::fixture`（父模块下的 `#[cfg(test)] mod`，十六个子模块都从 `super` 够得到），
每个测试搬到**它咬的那个模块**旁边。**crate 的公开面因此一个条目都没有增加**——
`Views`／`Standing`／`CommandDesk` 全部仍是 `pub(crate)`，`api-baselines` 只多了两行
`impl accounting::worker::RunWorker`：`RunWorker` 的方法现在写在三个文件里，`cargo public-api` 就记三个 impl 块。

### 验收

`cargo xtask length` 里 `assembly.rs` 的钉子被划掉而不是被调小；`[argument_count.predating]` 少十五行。
两者都是纯删除，所以 `guard::strikes_only_exemptions` 放行，不需要 `Verdict:` trailer；
budgets.toml 里那两段已经失真的注释单独一枚提交改，因为改注释会让豁免形状判定失效。

### 交接探针（`accounting::worker::probing::probe`，形状 2 值类型）

探针的唯一生产调用点是 `accounting::worker::probing`，所以它住在调用者之下，不另占产品拓扑的一个单元（仪器与探针分家的理由见 citysim-SPEC §3-6）。

```rust
pub(crate) struct ProbeId { pub(crate) name: String, pub(crate) version: u32 }
pub(crate) struct Probe { /* id、questions —— 私有 */ }
impl Probe {
    pub(crate) fn new(id: ProbeId, questions: Vec<String>) -> Result<Probe, AxError>;
    pub(crate) fn answered(&self, answers: Vec<String>) -> Result<Answers, AxError>;  // 数目对不上即拒
}
pub(crate) struct Comparison { pub(crate) kept: u32, pub(crate) lost: Vec<u32> }
pub(crate) fn compare(before: &Answers, after: &Answers) -> Result<Comparison, AxError>;
pub(crate) fn handoff_probe() -> Result<Probe, AxError>;   // 名 handoff、版本 1、固定四问
```

- **跨版本比较恒拒**：问题改过的探针是另一件仪器，混算测的是仪器不是被测物。`handoff_probe` 是数据不是判定：问题改了就是版本 2。
- **报位置不报分数**：`lost` 是问题的序号，人自己去读那两个答案——一个摘要在这里正好会掩盖它要报告的那类损失。
- **探针不去采集**：问问题的是 `probing` 驱动的一个 Run；`probe` 只持问题与比较，恒不在它所测量的那条回路里。

## 8-40 先判定后动手：一次派活在城答应之前不写任何东西

`stage_dispatch` 的开篇注释一字不差地写着这条规矩——「Nothing is written before the city agrees to take
the work: a halted city that laid a job file down would leave a task in a room no run ever opened」——
而代码只守住了停摆那一道。**本节把那句注释变成代码的形状。**

### 量出来的现状

一次派活从人发出的动词到第一次写，走过三段，其中两段先写后判：

| 顺序 | 在哪 | 做什么 | 能不能拒绝 | 写不写 |
|---|---|---|---|---|
| 1 | `commanding::run_command` Dispatch 臂 | `session_for` | `E_INVALID_ARGS`（取不到名字） | 否，但**要花一次 Digest 模型调用** |
| 2 | 同上 | `room_for` → `city::open_room` | 存储错 | **写：房间目录**（`create_dir`） |
| 3 | 同上 | `choose_shape` → `city::write_session` | `E_CONFIG_INVALID`（形状已在会话里冻下） | **写：房间的 CONFIG.toml**（只在会话的第一个 Run） |
| 4 | `dispatching::stage_dispatch` | `halted_by` | `E_GATE_DENIED` | 否 |
| 5 | 同上 | `city::write_brief` | 存储错 | **写：`JOB.md`** |
| 6 | 同上 | `cas.put` | 存储错 | 写：CAS 对象（`.sprawling/` 内，内容寻址） |
| 7 | `workbench::stand_up` | `Building::of`／`city::load`／`load_config`／`Router::select`／`adapter_for`／`Identity::load` | `E_INVALID_ARGS`／`E_CONFIG_INVALID`／`E_MODEL_UNCHOSEN`／`E_GATE_DENIED` | 否 |
| 8 | 同上 | `run_id_for`／`governance.sent`／worktree 租约 | 存储错 | 写 |

实测（`sprawling call` 打到一座刚 init 的城）：派活到从没立过的楼 `gamma`，得到
`E_MODEL_UNCHOSEN「no model is chosen for this tag」`——**来自第 7 段的 `Router::select`**——
而磁盘上留下 `<city>/gamma/one/JOB.md`，账本只有 `seq 0 city_initialized`。
**第 2 段与第 5 段跑在第 7 段之前，这就是全部的病因。** 不是写域逃逸：`Work ".sprawling/evil"`
得到 `E_INVALID_ARGS` 且一字节未落，保留子树守得住。

### 判定的依据

**一次派活在城答应之前不写任何东西；城一答应，第一件被写下的就是房间。**

「城答应」由一处回答，穷尽如下，且每一条都只读不写：保留子树（`Building::of`）、停摆
（`halted_by`）、楼的规矩读得出（`city::load`）、房间的居民是 harness 时它的拼写认得、楼不是 confidential、派活没点名模型（`city::settled_harness` 与 `agent_protocols::Harness::parse`，§8-124；居民是 harness 就不再往下判模型）、tag 后面有模型且端点还在且不违反 confidential
（`Router::select`）、适配器造得出（`adapter_for`）。

**留在答应之后的两条拒绝，各有其理由，写在这里而不是被含糊过去**：

- `city::load_config` 读城／楼／居民三层。它**必须**在 `choose_shape` 之后，因为同一次派活写下的
  模型与 effort 要被这一次跑读到（「Chosen once, it holds for every later run in that room」）。把它提前
  会让这次派活看不见自己刚写下的那一层——那是行为改变，不是顺序整理。
- `city::Identity::load` 只在**文件权限**上拒绝；文件不存在读作 ephemeral。那是机器的故障而不是
  这座城的依据，与「派活到没立过的楼」不是一类。

两者拒绝时留下的是**一间空房间**，而那间房间是城已经答应之后开的：人要的房间开出来了，然后他自己写坏的
配置挡住了这次跑。这与「城拒绝了却留下一间没人开过的屋子」不是同一件事。

### 接口

```rust
/// 城在写下任何东西之前答应的那些事。
pub(super) struct Standing {
    building: city::Building,
    rules: city::BuildingRules,
    model: gateway::ModelEntry,
    adapter: Box<dyn Model + Send>,
}

impl RunWorker {
    /// 每一条这次派活可能欠下的拒绝，在它花掉任何东西之前。
    pub(super) fn agree_to_work(&mut self, addr: &Address) -> Result<Standing, AxError>;
    /// 站位，接过城已经答应的那部分。
    pub(super) fn stand_up(&mut self, agreed: Standing, at: &Assignment, given: &Given)
        -> Result<Site, AxError>;
}
```

`Standing` 的四样东西在 `stand_up` 里原样进 `Site`，**不留第二份**：`Site` 的字段一个不增一个不减，
拆开归位即可。`agree_to_work` 住在 `dispatching.rs` 而不是 `workbench.rs`，理由是尺寸也是位置：
`workbench.rs` 997 行、离 1000 行的文件门只剩三行，而这条规矩的那句注释本来就写在 `dispatching.rs`。

### `Assignment` 多两个字段，理由是规矩不能有两个家

`session_for`／`room_for`／`write_effort` 必须搬进 `stage_dispatch`，**否则这条规矩就有两个家**：
`stage_dispatch` 是唯一被七个派活入口共用的地方（人发的动词、批准后续跑的活、`wake`／`tick`／`knock`／
委派子活、继任），而房间是在人发的那条臂上开的。把答应放进 `stage_dispatch` 而把开房间留在臂上，
等于让敲门那条路照旧先写后判；把答应也放到臂上，就要在三个调用点各算一次。

于是 `Assignment` 从四个字段变六个：

```rust
pub(super) struct Assignment {
    /// 活被送到哪儿。还要开房间时它是一座楼，已经有房间时它就是那个房间。
    addr: Address,
    /// 要在那座楼下开的房间，当调用方点了名。序幕用掉它，而序幕正是 `addr`
    /// 从「人要的地方」变成「跑干活的地方」的那一行。
    session: Option<kernel::SessionName>,
    /// 那间房里的跑从此想多久，当调用方说了。写进房间自己的配置层，
    /// 所以它答不到房间存在之前去。
    effort: Option<kernel::Effort>,
    mode: kernel::Mode,
    budget: kernel::BudgetCap,
    parent: Option<RunId>,
}
```

`session` 与 `effort` 只活到序幕结束，而 `addr`／`mode`／`budget`／`parent` 穿过每一个相位——
一个值里两种寿命，这是自认的代价。**换来的是这条规矩只有一处能被违反**，而另一种形状是
它有两处、且其中一处没人看着。四个已有字段的语义逐字不变；三个不开房间的调用点写 `session: None,
effort: None`，那正是 `session_for`／`room_for` 对它们本来就有的答案。

### 序幕的顺序，以及为什么是这个顺序

```rust
let agreed = self.agree_to_work(&at.addr)?;        // 只读；第一条拒绝在这里
self.book_rules(&agreed.building)?;                // 答应之后的第一句：rules_changed 先落账再生效
let session = self.session_for(&at.addr, at.session.take(), &task, agreed.rules.policy())?;
// ↑ 可能花一次 Digest 调用，而那次调用带着这座楼的策略
at.addr = self.room_for(at.addr, session.as_ref())?;                  // ← 第一次写
// 会话冻下的形状写在这里，而已经开着的会话在这里被拒：两件事一个决定。
self.choose_shape(&at, &agreed.model)?;                              // §8-79
let brief = city::write_brief(...)?;
...
let mut site = self.stand_up(agreed, &at, &given)?;
```

**`session_for` 排在答应之后**：它可能向 Digest 模型要一个名字，而为一件城不会接的活付一次模型调用，
是这条规矩的钱那一面。

**命名这一次调用带着楼的策略走，与跑自己的调用同一格**（S-04）。`agree_to_work` 刚读出的 `rules`
就在手边，`session_for` 与 `name_the_work` 因此收一个 `&kernel::BuildingPolicy` 参数，由 `EndpointBook::select`
据它拒绝跑在城之外的端点——判定只有 `select` 一处，这里不复述规则。confidential 楼上的 Digest 端点跑在城之外时，
这条拒绝**原样上抛**（`E_GATE_DENIED`）而不是塌成「取不到名字」：模型在哪台机器上是人要处理的事，
恢复语因此同时给出两条出路（自己写 `building/name`，或选一个与城同在一台机器上的 digest 模型）。
取不到名字的其余失败仍是 `E_INVALID_ARGS`。

**验收**：`accounting::worker::dispatching::tests::a_confidential_building_will_not_name_a_room_with_a_model_off_this_machine`——
confidential 楼、主模型与城同机、Digest 端点在机器之外，往楼名派活以 `E_GATE_DENIED` 告终，任务原文不上任何一条线。

**`book_rules` 是答应之后的第一句：改规则先落账再生效**。派工是规则的生效点：人手改 `RULES.toml`／`CONFIG.toml`
之后，城在下一次派工时才把新规则交给一个 run。`RunWorker::book_rules(&Building)` 把 city 层的 `CONFIG.toml`，
以及这栋楼**真实存在**时它自己的 `CONFIG.toml` 与 `RULES.toml`，逐份对比治理 fold 上次记下的摘要（`Governance.rules`，§8-25），
动了就先落一行 `rules_changed` 再让 run 起步。载荷携 scope、`which: GoverningDocument`（一栋楼两份，scope 单独说不清是哪份）、
before、after 与字节数，恒不携正文；`before` 缺席即开账行，此后本行的 `after` 等于下一行的 `before`，链断本身就说明有人绕过一切门改了文件。
摘要状态只由 fold 从 `rules_changed` 行得出，重启重建同一本账。缺席的文件按零字节记摘要，与 `city::load` 把缺席读成普通楼是同一个判断。
读不了的文件以 `E_STORAGE_FATAL` 拒绝这次派工：一个 run 不能站在城说不清的规则之下。

- **记在 `agree_to_work` 之后，不在它之前**：本节的规矩是城答应之前不写任何东西，一次被拒的派工因此不留行。
  被否：`stage_dispatch` 的第一句——停城、被拒的模型也会先落一行账，拒绝不再是「什么都没写」。
- **只为存在的楼记账**：楼的根目录不存在时，只记 city 层那一份。
  理由：往一个打错的楼名派活会给不存在的楼开账，账上就多出一座城没有的楼。存在与否用 `try_exists` 问，问不出来按读不了拒绝，而不是当作不存在。
- **记账点在派工（生效点），不在写入门**：这两份文件有一个在编辑器里动笔的写者，写入门看不见那次写入；
  生效点看见的是文档本身，两种写者一视同仁。每一个入口——人的派工、城自发的活、计划自己的一行——都经 `stage_dispatch` 起步。

验收：`accounting::worker::driving::tests::flight` 的三条 `rules_changed` 测试——首见开账、链上前后相接并先于它治理的 run 的每一行、
重启 fold 出同一本账；往不存在的楼派活只记 city 层一行。

**`halted_by` 并入 `agree_to_work`**：它本来就是唯一守住的那道门，
现在与其余五道站在一起，于是「城答应什么」读一处就够。

**采钟点不动**（ARCH §10）：`run_id_for` 读的 `clock.now()` 仍在选型之后，
采样次数与相对先后逐字不变；变的只是两者之间多了几次文件写，而那不是任何账本值的输入。

### 谁答哪一个错误码，逐字不变

`agree_to_work` 里依据的先后就是今天的先后，因此**没有一个调用方会看到与今天不同的码**：
停摆答 `E_GATE_DENIED` 而不是 `E_CONFIG_INVALID`（否则会把一个能自己解除的停摆说成要去接 provider），
保留地址答 `E_INVALID_ARGS`。派活到没立过的楼仍答 `E_CONFIG_INVALID`——
**这一条是刻意保下来的**：轨道二的模型把「楼在不在这里不问」记为一件量出来的产品事实
（`tools/adversary/src/Sprawling/Model.lean` 的 `refusal`），改码等于要那份模型跟着改，
而这里不碰 `tools/adversary/`。加一道「楼必须存在」的前置判断会正好破坏它——这是不选那个修法的第二个理由，
第一个理由是它只修一半（`acme` 真在而没挂 provider 时 `<city>/acme/one/JOB.md` 照旧留下）。

### CAS 那一次 `put` 留在原位，理由写在这里

`cas.put(brief.segment_text())` 仍在 `stand_up` 之前，所以严格地说「答应之后」并非一个字节都不写：
`.sprawling/cas/` 会多一个对象。**留它的理由**：CAS 是内容寻址且去重的，同样的字节写第二次就是同一份，
它不可能在城里留下一间没人开过的屋子；而把它挪到答应之后，`run_id_for` 就得从一个此刻还没入库的
摘要拼出 `cas:b3-…`，于是「locator 钉住的是库里的那些字节」这条权威会有第二处。
本节的依据因此写作「城里人看得见的东西」，而不是含糊的「任何东西」。

### 验收

1. **红转绿（Rust，本仓）**：`a_dispatch_the_city_will_not_take_leaves_no_room_behind`——
   一座刚 init、没挂任何 provider 的城，派活到 `gamma/one`，必须得到 `E_CONFIG_INVALID`，
   且 `gamma` 目录不存在。今天它红在第二条断言上。
2. **红转绿（轨道二的检验器，仓外）**：`a refusal costs nothing` 那一组两条转绿——`nothingBehind`
   （被拒的派活不改变城的目录树）与 `listsOnlyRaised`（`city_view` 只列被立起来过的楼）。
   这两条断言是验收标准，不许为了让它绿而改动它们。
3. **不回归**：`sprawling` 全部单测绿；`a_dispatch_with_no_goal_leaves_no_job_file_and_says_the_person_is_here`
   与 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯着序幕与租约这两支。
4. `just check` 绿。

### 文档同步

本节；`ARCHITECTURE.md` §5 第 3–4 步（派活先答应再写，房间是第一件被写下的东西）；
`crates/sprawling/sprawling-SPEC.md` §8-31 的相位表（`stand_up` 多收一个归位值）。
`city` 与 `wire` 的公开面不变，故 `api-baselines` 不动。

## 8-27 落点一 · 逆携带：变化带着它的撤销值来

§8-24 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）。
这里问的是下一句：**变化落到一半失败，城是什么形状**。今天 `settle` 里五个臂都是
「推过去，错了就把错抛上去」——`Deliver` 循环里第三个 signal 投递失败，前两个已经在
Inbox里；`Shelf` 循环里第二个 filing 写盘失败，第一个已经在架上；`Roadmap` 写半截，
`Hold` 无失败面所以没事。账本是对的（行全落了），城是撕裂的：历史说五个都到了，
城里只到了两个。

**逆携带**：`Then` 的每个臂与其同构造子的撤销值一起走。`Deliver` 带着「这一轮推进了
几个、knocks 推了几个」回来，失败时调用方把没投递的留下（它们本来就在调用方的
`Vec` 里，没丢）、把已推进的 knocks 截回进入时的长度；`Shelf` 带着已写下的路径回来，
失败时把它们删掉（架上无历史，这是 §8-24 那句话的另一半）；`Roadmap` 经 `city::edit_against`
以落地时读到的文本为基线替换，替换是原子的，失败时文件仍是基线，撤销值是空；`Hold` 无失败面，
撤销值是空。撤销值不是第二个权威：它只在 `settle` 的一次调用里活着，调用结束就丢掉。

**为什么不是两阶段提交**：两阶段要一个所有参与者都认的准备态，inbox 的队列、
goal 登记、书架的文件三者没有公共的准备态。逆携带要的只是一个调用里「做了什么」
的记录，失败时按记录往回走。往回走本身也可能失败（删文件时盘掉了）——那时返回
原始错误并把回滚失败写进 recovery：两个事实都得说，丢哪一个都是撒谎。

**红**：`a_half_settled_landing_leaves_no_torn_city`。一跑投递三个 signal，
第二个投递失败（inbox 满）；断言：第一个 signal 不在Inbox里（调用方截回），
knocks 长度与进入时相等，账本三行都在（行不受牵连）。`Shelf` 同形：第二个 filing
写盘失败，第一个 filing 的路径不存在，账本两行都在。

**影面**：`sprawling` 公开面不变（`effect` 不是 `pub mod`）。`settle` 签名不变，
行为变（失败时回滚），红钉住它。

## 8-28 落点二 · epoch 机器：「依赖快照」回答、「轮询」不回答

哲学一句话的后半句是「每个依赖驱动其激活」。这里先回答它的一半：
**快照是依赖的形状，轮询是依赖的反形状。**「散装 notify 轮询」不是指某一个
timer——`SCHEDULE_TICK`（20s，`serving.rs`）本身留着——而是说：今天「什么该醒」
这个问题的答案散在五处，每处各读一遍磁盘，各用各的「上次」：

| 谁问 | 在哪 | 读什么 | 记住什么 |
|---|---|---|---|
| `tick` | `commanding/routing.rs` | `Schedule::load` 全表 | `last_tick`（worker 字段） |
| `wake` | `waking.rs` | `Watch::load` 全表＋`buildings` 全量 | 无（每次全算） |
| `knock` | `waking.rs` | `Identity::load`（逐 room） | 无 |
| `answer_knocks` | `waking.rs` | 无（只 drain） | `knocks`（worker 字段） |
| `stage_dispatch` | `dispatching.rs` | `halted_by`（治理折叠） | 无 |

**三处可量**：`wake` 一次读两遍磁盘（watch 表＋全部楼目录）而只为投递一个 arrival；
`tick` 一次读全表而只为问「自上次以来谁到期」；`Watch::listening` 的「楼还在」
每次现算，而楼的生死是账本里变化最慢的事实之一。

### 动的与不动的

**动的只有一处**：`commanding/routing.rs` 的 `tick` 不再读全表，而是读
`city::Schedule::due_after(path, last_tick, now)`——到期判断（`last_firing`
区间比较）下沉到 `city`，`assembly` 只剩 dispatch 循环。`Schedule::due`
（返回 `Vec<&Entry>` 全量引用）保留：它是 `city` 自己的公共面，删它是
`city` 的 breaking，故不碰。

**不动的三处，理由各写一条**：

- `SCHEDULE_TICK` 不动：tick 间隔是 serve 面的节奏（§8-38），到期判断是 city
  面的语义，两者不在同一层，换一处不动另一处。
- `wake` 的双读不动：watch 表是人的文件（`listening` 语义含「楼已拆即失聪」），
  到达即读即算正是「文件是人的」这个归属的形状；快照它等于替人记住，归属错。
- `knocks` 不动：它是逆携带的截回点（§8-27），形状已钉，动它等于重开逆携带。

### 接口

```rust
impl Schedule {
    /// 自 `after` 以来到 `now` 之间到期的条目。`due` 的区间判断原样下沉，
    /// 返回拥有权的 `(Address, String, String)` 三元组：调用方只剩 dispatch。
    pub fn due_after(&self, after: TimeMs, now: TimeMs) -> Vec<(Address, String, String)>;
}
```

`tick` 此后三行：`load` → `due_after(last_tick, now)` → 逐条 `dispatch`。
`last_tick = now` 的位置不变（先推进再跑：到期判断的输入是读到的那一刻）。

### epoch / LOADING / UNLOADING 在哪

epoch／LOADING／UNLOADING 是**「Assembly 显式化」那一步的
主题**，不在这里。这里只把「到期判断」这一处依赖收成快照的形状
（`due_after` 即运行级依赖快照的最小形态：调用方拿着「到期了什么」，
而不是「全表＋上次」），并为那一步留下一句依据：**凡调用方仍在做区间比较、
仍在记 `last_*` 的，皆是 epoch 机器要收走的东西**（`knocks` 除外，它是
§8-27 的逆携带点）。

### 验收

1. `tick` 的三行与 `due_after` 的区间语义由既有红守着：
   `a_scheduled_job_starts_by_itself_and_only_once_per_firing`
   （三调用：到期 1、期内 0、一小时宕机仍 1）逐字绿，不改一字。
2. `commanding` 一分为二（`routing`：动词路由＋tick；`governing`：halt／
   autonomy／approval／fork）与 `settling` 一分为二（`desks` 四桌顺序；
   `landing` 单落点＋结论）只搬家：跨文件调用的可见性收成
   `pub(in crate::assembly)`，行为零变，`sprawling` 158 全绿。
3. `just check` 绿；`city` 公开面只增一函数（`api-baselines` 同集改写）。

### 文档同步

本节；`ARCHITECTURE.md` §6 新增六行（commanding::routing／governing、
settling::desks／landing、commanding::tests::answering／clockwork、
settling::tests 已有、waking::tests）；`city-SPEC` 的 schedule 节记
`due_after` 与 `due` 并存的理由（删 `due` 是 breaking）。

## 8-29 落点三 · Assembly 显式化：接线是一处，判定住 kernel，搬运是值

§8-28 留下的依据是「凡调用方仍在做区间比较、仍在记 `last_*` 的，皆是
epoch 机器要收走的东西」。这里把它收走，并连带回答那三句话。
**三句话各是一处代码动作，不多不少**：

### 接线留——`adapter_for` 搬出 `credentials.rs`

今天 `agree_to_work`（`dispatching.rs:252`）与 `name_the_work`
（`dispatching.rs:497`）各调一次 `self.adapter_for(&chosen)`，而
`adapter_for` 住在 `credentials.rs:581`——凭据簇里住着一条装配线。
搬家：`adapter_for(chosen, resolver)` 成为 `gateway` 的自由函数
（`endpoint/adapter.rs`，与 `Endpoint::new` 同簇），`resolver` 由调用方传入。
`credentials.rs` 留下 `resolver`（赎回闭包是凭据的形状），`dispatching`
的两个调用点各多传一个 `self.resolver()`。

**为什么是值参不是方法**：`adapter_for` 读的只有 `chosen` 与 `resolver`，
`self` 的其余 21 个字段与它无关；挂在 `RunWorker` 上等于说「装配需要整座城」。
搬出去后 `credentials.rs` 少一个 `impl RunWorker` 方法，多一个跨 crate 调用——
接线只有一处（`gateway::endpoint::adapter`），这就是「接线留」。

### 判定进 kernel——`halted_by` 的归属不变，调用点收敛

`halted_by` 住在 `commanding/governing.rs:45`（`pub(in crate::assembly)`），
读的是 `governance` 折叠（`HALTED`／`RELEASED`）。「判定进 kernel」的
含义经核对后收窄：停摆判定读的是**本进程的折叠状态**（`self.governance`），
不是纯函数能回答的问题；硬搬进 kernel 等于把 `Governance` 也搬过去，
那是另一步的事（`folds.rs` 957 行）。这里只做收敛：`halted_by` 的两个调用点
（`dispatching.rs:229` 与 gate 面）确认走同一函数——量过，只有一处定义，
调用点已收敛，**本句的验收是「无代码变更」，理由记在这里而不是被含糊过去**。

### 搬运下沉 adapter——`Driving.adapter` 由 `&mut dyn Model` 改为拥有值

今天 `Driving<'a>`（`driving.rs:28`）的 `adapter` 字段是 `&'a mut dyn Model`，
由 `dispatching.rs:350` 的 `site.adapter.as_mut()` 出借。`Agreed.adapter`
与 `Site.adapter` 是 `Box<dyn Model + Send>` 拥有值，`Driving` 是唯一的
出借点。搬运下沉：`Driving` 改为拥有 `Box<dyn Model + Send>`（调用点 `move`），
`drive_dispatch` 结束时把 `adapter` 还回——还法是 `Driven` 多一个字段
`adapter: Box<dyn Model + Send>`，调用方拆开归位（`Site` 字段名不增不减，
`adapter` 的类型由 `Box` 变为 `Option<Box>`——`Option` 是这次搬运的载具而非新状态，
跨过调用时两侧皆为 `Some`，`None` 不可观察；`None` 分支以 `E_CONFIG_INVALID` 拒绝告之而非 panic，
§8-40 的先例）。

**为什么**：`&mut` 出借把「谁拥有 adapter」这个问题悬在一次调用上；
拥有值随 `Driven` 回来，适配器的来去在类型上闭合——这就是「搬运下沉」，
与 §8-40 `Standing` 四样东西「拆开归位」的同一条道理。

### epoch 机器——`last_tick` 的区间比较收归一处

§8-28 的依据点名 `last_*`。量过：`last_tick` 是全仓唯一的 `last_*`
（`grep last_` 全仓仅 `assembly.rs:185` 定义＋`routing.rs` 读写＋测试）。
收走：`tick` 的「读表→判断→推进 `last_tick`」三步收成
`RunWorker::tick_after(now)` 仍三步，但 `last_tick` 的读写只在此一函数——
今天已是如此（`routing.rs` 的 `tick` 是唯一读写点），**本句的验收同样是
「无代码变更」**：epoch 机器的第一条轨道（到期判断下沉 `city`）已在上一步
落定，剩下的 `last_tick` 字段本身是 worker 状态而非散装轮询，
删它等于把「开机不补跑昨日」这个产品语义（§8-6）一并删掉，不删的理由在此。

### LOADING / UNLOADING 在哪

LOADING／UNLOADING 落在 `RunWorker::over`（`assembly.rs:239`）：
`Standing::fold` 即全量 LOADING（一次验证、三折叠，一句注释已写明），
而 UNLOADING 是 `close_city` 写 handoff（`assembly.rs:394`）。
两者皆已有名有主，不给它们改名——**给已存在的东西改名是第二权威，
§8-39 的教训**。这里只在 `over` 的 doc 上加一句：「此即 LOADING；
UNLOADING 见 `close_city`」，让设计里的词与代码的名在文档里相遇。

### `RunWorker` 立面只减不增

`adapter_for` 搬出后，`impl RunWorker` 方法数减一；`halted_by`／`tick`／
`last_tick` 零增；`Driving`／`Driven` 的字段变化是 `driving.rs` 内部形状，
不进立面。验收：`cargo public-api -p sprawling` 基线零漂移（`impl` 块行数
随文件搬家增减已有口径：同集改写，预计 `commanding/governing`
与 `commanding/routing` 的 impl 行各一，同集处理）。

### 验收

1. `gateway::endpoint::adapter::adapter_for` 新建，`dispatching` 两调用点
   传 `self.resolver()`；`credentials.rs` 的 `adapter_for` 删除。
   既有测试 `a_loopback_endpoint_with_a_credential_sends_it_on_every_call`
   与 `a_dispatch_without_a_provider_fails_saying_what_to_configure`
   逐字绿（它们咬的正是这条装配线）。
2. `Driving` 拥有 adapter，`Driven` 带回 adapter；`dispatching.rs:350`
   处拆开归位。`sprawling` 全绿。
3. `over` 的 doc 增 LOADING／`close_city` 互指一句；`halted_by`／`tick`／
   `last_tick` 零代码变更（本节即其理由）。
4. `just check` 绿；`sprawling` 基线按口径同集改写（只增减 impl 行）。

### 文档同步

本节；`ARCHITECTURE.md` §6（`gateway::endpoint::adapter` 新行；
`commanding::governing` 职责减一句）；`gateway-SPEC` 的 endpoint 节记
`adapter_for` 的归属理由（装配线住适配器簇，凭据只出 `resolver`）。

## 8-40 运行中的机器有什么，这座城要什么（`bin::doctor`）

**原因**：README 说「别 `cargo install` 这个东西」，`just check` 要 `just` 与 `cargo-nextest`，客户端要一个版本与 crate 版本相等的 `wasm-bindgen` CLI，exec 工具的 python 臂要一个 CPython-WASI 组件，而这些要求今天散在四份文档里。一个人装不全的时候，得到的是某一条命令的失败信息，而不是一句「运行中的机器缺什么」。

```rust
// bin::doctor（形状 1 decision：表与判定；驱动只读写它拿到的那两个句柄）
pub(crate) enum Tier { Use, Develop }                  // 两层：用得起来，改得动
pub(crate) enum Need { Required, OneOf(Group), Optional }   // 一组任一即可，见 §8-57
pub enum Platform { Windows, MacOs, Linux }            // `bin::install` 经它进入，遵 lib.rs 的约定：二进制进入即公开
pub(crate) struct PerPlatform<T> { windows: T, macos: T, linux: T }
pub(crate) enum Detection { Program { program, version_arg, places }, Environment { variable } }
pub(crate) struct Requirement { name, tier, need, enables, detect, homepage, recipe }   // recipe: PerPlatform<accounting::Recipe>（accounting-SPEC 8-4）
pub(crate) enum Presence { Present(String), Absent }
pub(crate) struct Finding { requirement: &'static Requirement, presence: Presence }
pub(crate) fn examine(machine: &dyn Machine) -> Vec<Finding>;
pub(crate) fn finding_line(finding: &Finding) -> String;      // present <v> | absent | optional-absent
pub(crate) fn verdict(findings: &[Finding], tier: Tier) -> Verdict;
pub(crate) fn verdict_line(tier: Tier, verdict: &Verdict) -> String;

// bin::doctor::table（形状 6 data：编辑它就是编辑行为，无分支）
pub(crate) const REQUIREMENTS: &[Requirement];
pub(crate) const WASM_BINDGEN_VERSION: &str;                  // 与工作区 Cargo.toml 的钉子相等，由测试守住

// bin::doctor::screen（形状 4 adapter：屏幕上的那份报告与那一问）
pub(crate) struct Asked { install: bool }
pub(crate) fn asked(args: &[String]) -> Asked;
pub(crate) fn run<R: BufRead, W: Write>(asked: &Asked, machine: &dyn Machine, input: &mut R, out: &mut W) -> io::Result<bool>;
pub(crate) fn verb(args: &[String]) -> ExitCode;              // 必需项有缺则退 1

// bin::doctor::probe（形状 4 adapter：运行中的机器）
pub(crate) trait Machine: accounting::Machine { fn look(&self, r: &Requirement) -> Presence; }   // 安装经父 trait 的 install（accounting-SPEC.md 8-4）
pub(crate) trait Machine { fn look(&self, r: &Requirement) -> Presence; fn install(&self, name: &str, recipe: &Recipe) -> Result<(), AxError>;
                           fn core_standing(&self) -> Result<Standing, AxError>; } // §8-93 的档位，由平台实际给出
pub(crate) struct ThisMachine { platform: Option<Platform>, patience: Duration }
pub(crate) fn answer(machine: &dyn Machine) -> wire::DoctorAnswer;   // bin::doctor::report：一页答案，ThisMachine 的 report 就是它
pub(super) fn names_of(program: &str) -> Vec<String>;         // Windows 上 .exe／.cmd／.bat 在先，无扩展名在后
```

- **两层而不是一张清单**：一个只想让这座城跑起来的人与一个要改这份代码的人，缺的不是同一批东西。把 Firefox 与 `cargo-nextest` 摆进同一张「缺失」清单，等于告诉前者他缺一个他永远不会用的测试跑器——**判定因此按层给两句话**（`ready to use`／`ready to develop`），而不是一句总分。
- **逐项征求同意，而不是一次总同意**：`--install` 对每一个缺项先印出**运行中的机器上要跑的那条命令**，再在 stdin 上问 `y/N`，默认是 N。一次总同意会让人对一串他没读过的命令点头，而这些命令改的是他自己的机器。**没被问到的东西恒不安装**。
- **恒不提权**：这里的每条命令都是用户级的（`winget`／`brew`／`cargo install`／`rustup`），`sudo`／`apt` 那一支落在 `Recipe::Print`，人自己贴。一个默认会请求管理员权限的 doctor，是把「检查」变成了「让我动你的系统」，与 §8-9 的 install 同一条理由：**只碰这个人 profile 里的东西**。
- **curl 脚本只印不跑（被否决的备选：`curl | sh` 自动安装）**：bun 在没有包管理器的平台上的官方装法是把一段脚本管进 shell。跑它意味着这座城代替人接受了一份它没读过、也无法在此刻校验的远端代码——**被否决**。那一支是 `Recipe::Print`：命令印在屏幕上，人自己决定。
- **探测是「在不在 PATH 上」加「`--version` 说什么」，且带时限**：一个装坏了的工具会挂在启动上，而 doctor 挂住等于比不装还糟。子进程的读法沿用 `agent_protocols::mcp::stdio` 的形状——读在一个线程里，等在一个带 deadline 的 channel 上，超时就杀掉子进程；`patience` 是参数，**不在这里采时钟**。浏览器另加平台标准安装路径与 Windows 的两个注册表键，因为 Windows 与 macOS 上它常常不在 PATH 上（§8-57）；**浏览器的版本不问它本人**，读它旁边的文件（§8-80），于是这条带时限的子进程路径只剩驱动与命令行工具在走。
- **四个文件而不是两个，理由是尺寸与形状**：`doctor.rs` 只留判定（表的形状、`finding_line`、`verdict`），表落 `table.rs`，屏幕与那一问落 `screen.rs`，跑子进程的落 `probe.rs`。判定与驱动同住一个文件时 `doctor.rs` 是 399 行——`xtask length` 的 400 之下一行，即下一次编辑必红。**这不是把文件切碎，是把「判断」与「跟人说话」分开**，两者本就不是一件事。
- **Windows 上先找带扩展名的那个文件**：`bun` 若由 npm 装出来，同一目录下既有无扩展名的 shell 脚本 `bun`（Windows 起不动）又有 `bun.cmd`。先取无扩展名的那个，报出来的是「装了但不说版本」——一个装好的工具被报成半坏的。故 `names_of` 在 Windows 上按 `.exe`／`.cmd`／`.bat`／无扩展名的次序找，这条次序有它自己的测试。
- **一项是环境变量而不是程序**：exec 工具 python 臂要的 CPython-WASI 组件由 `PYTHON_WASM_ENV` 指路（`accounting::worker::workbench::tools`），故它的探测是「那个变量指的文件在不在」，安装那一栏是 `Manual`——没有包管理器发它。它是 `Optional`，行尾说明它开启的是什么。
- **终端里的词是英文，这不违反 wording 门**：AGENTS.md 的语言表把词表的管辖写在客户端上，`xtask wording` 扫的目录是 `client/src`（见 `tools/xtask/src/wording.rs` 的 `CLIENT` 常量）。控制台是操作者的，与 `install`／`console`／`firstrun` 同一口径。
- **机器面是一条缝，而不是一个假想缝**：`Machine` 有两个实现——`ThisMachine`（真跑子进程）与测试里的 `ScriptedMachine`（一张 name→Presence 的表，外加它经 `accounting::Machine::install` 记下的安装请求）。终端与 worker 的安装走同一个 `accounting::Machine::install`，所以测试记下的就是终端真正会启动的那一次。判定因此不需要测试机上真装着什么就能被咬。
- **核心线程实际站在哪一档（§8-93）**：报告在各层的判定之后多一段 `priority`，一行 `core threads`：`one step above normal`；`normal, as config.toml [core] priority asks`；`normal, the platform refused: <原因>`（Unix 上没有 `CAP_SYS_NICE`）；读不出设置、或问档位的那条临时线程起不来或没答话就结束时是 `unknown: <错误>`（线程 panic 时只报它没答话，不带 panic 的内容）。`ThisMachine` 读人的设置，在一条临时线程上调 `raise_this_thread` 得到这一档，线程随即结束，所以 doctor 自己的线程不换档。报的是运行 doctor 的主机此刻会给核心的档位，而不是某座正在跑的城的线程被安全阀降回之后的档位。派出的命令总是低一档（runtime-SPEC §8-13-3，降档从不被拒），这一段不重复它。证据：`crates/sprawling/src/doctor/tests/reading.rs` 的 `the_doctor_says_where_the_platform_lets_the_core_stand`（平台拒绝升档的机器，报告里是那一行与平台给的原因）。页面读同一读数：`report::fold` 从同一个 `Machine::core_standing` 折出 `DoctorAnswer::core`（wire-SPEC §8-25），词由页面选；证据是 `doctor::report::tests` 的 `the_page_is_told_where_the_core_stands`。
- **机器面是一条缝，而不是一个假想缝**：`Machine` 有两个实现——`ThisMachine`（真跑子进程）与测试里的 `ScriptedMachine`（一张 name→Presence 的表，外加它记下的安装请求）。判定因此不需要测试机上真装着什么就能被咬。

**本章测试**：表的完整性（每一项都有探测方法，且三个平台各自要么给出命令要么明说 `manual`；每一个 `Optional` 项都说出它开启什么）；`verdict` 对「全在」「缺一个必需项」「只缺一个可选项」三类输入给出正确的穷尽枚举（可选项缺失不拖垮该层）；`finding_line` 的三种写法；`--install` 在答 `n` 时**一件也不装**、答 `y` 时只装被问的那一件（由 `ScriptedMachine` 记账）。

**本章验收**：`cargo run -p sprawling -- doctor`，输出逐项与两句判定，必需项有缺则退 1。

## 8-41 门说的话与门做的事：静默有自己的退出码，重放的命令只做一次（`bin::wire_client`、`accounting::worker::commanding::entrance`）

仓外的对抗性检验器（`tools/adversary/Spec.lean` §4）留了两条未修的发现。两条都只在**门外**可观测，
两条都伤同一类调用方——一个拿退出码分支、拿重试兜底的 agent。本节一次答完，因为它们是同一个承诺的两半：
**门说出口的话必须等于门做的事**。

### 发现一：静默不是接受，故它不是 0

**原因**：窗口内没收到拒绝，不等于城照办了。`AttachEndpoint` 指向一个连不上的 base URL 时，产品侧探测 15 s，客户端默认窗口 2 s（`tools/adversary/Spec.lean` §4 第一个发现）；把没有拒绝读成 0，「拒绝没赶上静默窗口」与「城照办了」就是同一个码。

**依据**：`call` 与 `dispatch` 有四种结局。

```rust
// bin::wire_client::ending（形状 1 decision：四支穷尽枚举，壳只做映射）
pub(crate) enum Spoken { Refused, Answered, Quiet, Unfinished }
pub(crate) struct Heard { frames: u32, refusals: u32, answers: u32, awaited: Awaited, run: Option<RunId> }
impl Heard { pub(crate) fn spoken(&self) -> Spoken; }
```

| 结局 | 退出码 | 它断言的事实 |
|---|---|---|
| `Refused` | 1 | 城在窗口内拒绝了这一帧 |
| `Answered` | 0 | 城在窗口内答了话，且没有拒绝 |
| `Quiet` | 3 | 帧发出去了，窗口内**一帧都没回来**——城是否受理，此处无法断言 |
| `Unfinished` | 3 | 城回了话，但调用方在等的那一帧（查询的答复、点名的事件、派出的 run 的里程碑）没在窗口内来——活可能还在做，也可能没开始 |
| （用法错误） | 2 | 参数或帧不是这条命令能读的东西；帧在开 socket 之前解析，所以与城无关 |
| （没有城） | 4 | `--at` 那里没有东西完成握手；帧没有被任何城听见 |

- **`answers` 与 `frames` 是两件事**。握手的 `Welcome` 也是一帧，故 `frames` 恒 ≥ 1；能区分静默的只有
  「命令发出**之后**回来的帧数」。把这一个数放进 `Heard` 而不是在壳里减一，是因为「减一」会把握手协议的形状
  抄到第二个地方。
- **3 而不是复用 1**（被否决的备选：静默即拒绝）。静默不是拒绝：城可能已经受理，只是答案比窗口慢。
  把它读成拒绝，会让一个 agent 在城正在照办的时候重试——而重试的无害性正是发现二在修的东西。
- **0 与 1 的含义一个字不改**，故已有的脚本只在原本被误读为成功的那一档上改变行为，这正是要改的那一档。
- **窗口不变长**（被否决的备选：把默认 `--quiet-ms` 提到 20000）。窗口多长是调用方的事；把它调大只是把同一个
  歧义推后 18 秒，而 `Quiet` 让调用方**知道自己撞上了窗口**，于是加窗口重试是它能做的一个决定。

**本节测试**：`a_city_that_says_nothing_inside_the_window_is_not_a_success`——一个脚本化的 WebSocket 服务端
答完 `Welcome` 后闭口不言；`call` 返回的 `Heard` 的 `spoken()` 必须是 `Quiet`，`answers` 为 0。

### 发现二：一把必须带而无人读的钥匙

**原因**：23 个状态变更命令每一个都带 `IdemKey`，`kernel::gate::dedup` 把这道门实现成纯函数，
而它在自身模块之外**没有调用者**。同一条 `Halt` 发两次，账本里两条 `city_halted`。
`accounting::worker::desk` 只合并**还在队列上或正在被执行**的同键命令（`clockwork.rs` 那条测试钉的就是它），
一旦第一条跑完，重放就是第二次副作用。

**依据：判在命令入口，判在任何副作用之前**（`kernel-SPEC.md` §8.2 的原话）。

```rust
// accounting::worker::commanding::entrance（形状 1 decision：状态是集合，判定借 kernel::gate::dedup）
pub(in crate::assembly) struct Entrance { /* seen: BTreeSet<IdemKey>, refused: BTreeMap<…>, carrying: Option<IdemKey> */ }
impl Entrance {
    pub(in crate::assembly) fn answered(&self, key: &IdemKey) -> Option<Result<(), AxError>>;
    pub(in crate::assembly) fn begin(&mut self, key: IdemKey);
    pub(in crate::assembly) fn settle(&mut self, outcome: &Result<(), AxError>);
    pub(in crate::assembly) fn absorb(&mut self, data: &Payload);   // 账本回放
    pub(in crate::assembly) fn carrying(&self) -> Option<IdemKey>;   // 正在处理的命令的键
}
// 盖键：键与载荷的纯函数，记账线程上的 RunWorker::record_for 与不借 worker 的 Stamping::record_for 共用
pub(in crate::assembly) fn stamped(key: Option<IdemKey>, data: Payload) -> Result<Payload, AxError>;
pub(in crate::assembly) fn repeated(name: &str) -> String;   // 重复命令留下的那行诊断
pub(in crate::assembly) const IDEM_FIELD: &str = "idem";
```

- **门是 `serve_one`，不是 `handle`**。`serve_one` 是「一个人发出的命令变成什么」的唯一权威
  （`bin::assembly` 的 rustdoc 原话），也是 wire、控制台与 ACP 三条路唯一的汇合点——`assembly::attending`
  是它在产品里的唯一调用方。`handle` 是执行者，留给夹具与内部调用方按顺序驱动一座城；
  **门与执行者分开，是因为「判过了吗」与「怎么做」是两个问题**，而把它们合成一个方法会让
  每一个内部调用方都被迫带一把它并没有从人那里收到的钥匙。
- **判定借 `kernel::gate::dedup`，不在这里重写**。`Entrance` 持有那个 `BTreeSet`，kernel 只回答成员关系——
  这正是那个纯函数的 SPEC 说的「seen 集合是调用方的状态」。因此不改 kernel 的立面。
- **重复的键得到第一次的答案，且不再写第二次**。第一次是 `Ok` 就答 `Ok`（沉默地成功，因为那件事已经做过了）；
  第一次是拒绝就把**同一份** `AxError` 再交一次，于是重试的人两次读到同一句话，而不是第二次读到
  「没有东西在等」这种由第一次的副作用造出来的第二种拒绝。
- **重启后靠账本认出做过的事**：`serve_one` 在一条命令的执行期间把钥匙挂在 `carrying` 上，
  `record_where` 与 `record_for` 把它写进那条记录的 payload（键名 `idem`）。
  `Standing::fold` 已经在开城时逐行走一遍账本，`Entrance::absorb` 搭在同一趟上，不多读一遍。
  于是**一条命令留下了历史，它的钥匙就在历史里**。
- **已知边界，如实写在这里**：`run_started` 由 `runtime::run::lifecycle` 直接写进账本，装配点碰不到它，
  故一条**跑到一半就被进程死亡打断**的派活，其钥匙不在账本上，重启后重发会再跑一次。这恰好是重试**应当**
  被允许的那一档——那次派活没有结论。跑完的派活会经 `settling::landing` 的 `record_for` 落下带钥匙的记录，
  于是重启后再发同一把钥匙，答的是第一次的结果。
- **被否决的备选一：在 `accounting::worker::desk` 上记住所有见过的钥匙**。桌子没有账本，重启即失忆；且第一次的结果
  在桌子上不可得，它只能沉默地丢弃重放，而不是回答。
- **被否决的备选二：把 `IdemKey` 从线格式上撤掉**。那是把承诺删掉而不是兑现它，且 23 个命令的重试语义会
  一起消失。

**本节测试**：`the_same_dispatch_twice_under_one_key_opens_one_room_and_starts_one_run`——同一条 `Dispatch`
经 `serve_one` 送两次，钥匙相同：账本里恰有一条 `run_started`，房间恰有一个（此前是 `["one", "one-2"]`）；
`a_repeat_is_answered_with_what_the_first_ask_was_answered`——被拒的命令重发收到逐字相同的那份拒绝，
成功的命令重发不被拒也不再落账；`a_key_already_in_the_history_is_recognised_after_a_restart`——
一座重新打开的城认得账本里那把钥匙。

### `bin::wire_client` 的两件事各有文件

socket 上的一次对话与 HTTP 上的一次托管是两件事，同处一个文件时 `wire_client.rs` 越过了 400 行上限。

| 文件 | 管什么 |
|---|---|
| `wire_client.rs` | 与城的一次 WebSocket 对话：`Heard`／`Spoken` 与三支退出码、握手 `hello`、`call`／`converse`／`next_frame`／`report`，以及两处共用的不可达判词 `unreachable_city` |
| `wire_client/enrolment.rs` | 把一份明文交给同机的 `/enroll` 路由并取回替代它的引用：`split_reference` 与 `enrol`，连同钉住引用形状的 2 个 `#[test]` |

- **`unreachable_city` 仍只有一个家**：它留在父模块，`enrolment.rs` 经 `use super::unreachable_city` 取用，于是「城连不上」这句话不会有第二种说法。
- 父模块以 `pub(crate) use enrolment::{enrol, split_reference};` 转出，`main/data.rs` 的两处调用路径一字未改。
- 公开面不涉：两项都在二进制内部，`api-baselines` 不动。

### 文档同步

本节；`ARCHITECTURE.md` §12 增 `accounting::worker::commanding::entrance` 与两个测试文件的行；
`kernel-SPEC.md` 的 `gate::dedup` 一节记下它的承兑人；`tools/adversary/Spec.lean` §4 两条发现标注已修。
`docs/operating.md` 增退出码表。公开面：`bin::wire_client` 与 `bin::assembly` 都是二进制内部（`pub(crate)`
以下），`RunWorker::handle` 的签名不变，故 `api-baselines` 不动。

## 8-42 并发地板：一条记账线程，一个驾驶池

这一节回答一个问题：**这座城怎样同时跑两轮活，而账本上的字节仍然逐字节可重放。**
答案是把今天那一条 `sprawling-runs` 线程一分为二，两半各自持有互不相交的东西。

### 8-42-1 两半各持有什么

**记账线程（accounting thread）**，也就是今天那条唯一写入者，独占下列全部状态，一件不外借：

| 它独占的 | 今天住在哪 |
|---|---|
| `JsonlLedger` | `RunWorker.ledger` |
| 端点书 `EndpointBook` | `RunWorker.book` |
| 计划（`plan_holders`／`accounting::plan_view`） | `RunWorker.plan_holders` |
| 追求 `pursuits` | `RunWorker.pursuits` |
| 治理 `Governance`（待批、放行、停摆） | `RunWorker.governance` |
| 五张桌子（inbox／join／pr／goal／shelf 的**归位**那一半） | `RunWorker.inboxes`／`joins`／`requests`／`goals` |
| 准入计数（同时在跑几轮活） | 新增，见 8-42-4 |

**驾驶池（driving pool）**的线程只持有一个东西：一次 `Driving`。它没有账本、没有书、没有桌子的所有权，
也没有工人。它把 `Driven` 送回来，然后什么都不记得。

**从池里发出的每一次写，都走 relay**（8-42-2）。池线程手上唯一的 `kernel::Ledger` 实现是 `Relay`，
于是「一座城只有一个写者」这条性质由类型持有而不是由纪律持有：池线程根本拿不到 `JsonlLedger`。

**`settle` 永远在记账线程上跑，且按 `Driven` 到达顺序跑**。到达顺序而不是发起顺序：发起顺序会要求
记账线程为一轮还没回来的活留位置，那就是一个隐式的队头阻塞，而它挡住的正是已经跑完、正等着落账的那一轮。
到达顺序由 `mpsc` 的接收顺序给出，接收顺序由记账线程单线程读取，因此**同一批 `Driven` 的到达顺序
就是账本上 settle 各行的顺序**——这是 ARCH §10 规则 5「并行执行，串行记账」在这里的具体形状。

**`Interrupt` 按 run 注册**。今天 `RunWorker.interrupts` 是一个 `FnMut(RunId) -> Interrupt` 的钩子，
一次派活借走、结束还回（`driving.rs` 的 `self.interrupts.take()`）。一个池意味着同时有 N 轮活在问
「有人打断我吗」，于是这个钩子从「借走一个」变成「每轮活各注册一份」：`CommandDesk::interrupt_for`
本来就按 `RunId` 挑命令，这里只是让 N 份 `Relay` 各自带一份指向同一张桌子的注册。

**车道在驾驶的那轮活，它的 Cancel／Steer 留在桌上等车道来取**。记账线程每次被敲醒都读桌子，车道只在安全点读，
所以两个读者抢同一条命令时几乎总是记账线程先到——它若照 FIFO 取走，路由只能答「没有在飞的 run 认这个 id」，
而那轮活其实正在跑。于是 `CommandDesk::next(driving)` 带一个「这个 run 此刻在车道上吗」的判定（由
`RunWorker::drives` 回答，它读 `Flight.driving` 那一张表）：命中的 Cancel／Steer 原位留下，只归
`interrupt_for` 取；排在它后面的命令照常往下走。挑哪一条、Cancel 压过 Steer 仍只由 `interrupt_for` 一处决定，
桌子上的这个判定只决定「谁有资格取」。那轮活落地后 `drives` 转为否，还留在桌上的那条才交给路由，
拒绝的措辞此时是真的。**另一条路被否决**：记账线程先取走、再塞进车道的一个信箱，会让「Cancel 压过 Steer」
在桌子与信箱两处各判一次。判定与派活都在记账线程上，派活把 run 登记进 `Flight.driving` 之前不会读下一条命令，
所以「刚派出、还没登记」的窗口不存在。

### 8-42-2 `accounting::worker::relay`——`kernel::Ledger` 的第三个适配器

形状：**adapter**（ARCH §9 第 4 种）。它不做任何判断，判断全在记账线程那一侧。

```rust
/// 一次追加，以及它的回信地址。
pub(crate) struct RelayRequest {
    draft: EventDraft,
    back: std::sync::mpsc::SyncSender<Result<EventRef, AxError>>,
}

/// 池那一侧的脸：唯一的 `kernel::Ledger` 实现，池线程只有它。
pub(crate) struct Relay { /* 一个 Sender */ }
impl kernel::Ledger for Relay {
    /// 把 draft 发下去，然后**阻塞**等回信。
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
}

/// 唤醒记账线程的一切，同在一条队列上：每张嘴一个变体，再加关门。
pub(crate) enum Wake { Relay(RelayRequest), Claim(ClaimAsk), Goal(GoalAsk), Home(Box<Arrival>), Command, Close }

/// 一次看队列最多等多久。
pub(crate) enum Patience { Now, For(Duration), Unbounded }

/// 记账那一侧的脸：那一条队列的 Receiver ＋ 一个用来发牌的 Sender。
pub(crate) struct RelayGate { /* … */ }
impl RelayGate {
    pub(crate) fn open() -> RelayGate;
    pub(crate) fn issue(&self) -> Relay;
    /// 车道回家与 desk 用的 Sender。
    pub(crate) fn bell(&self) -> mpsc::Sender<Wake>;
    /// 按 `patience` 等第一条消息，再用 `try_recv` 把排在后面的一次取尽：relay 请求**合成一道屏障**
    /// 写下去，回家的活按到达次序放进 `homes`，Command 与 Close 只负责唤醒。
    /// 认领按队列次序当场答复；目标登记不在这里答，按到达次序交回。
    pub(crate) fn serve(&mut self, patience: Patience, ledger: &mut impl kernel::Ledger,
                        homes: &mut VecDeque<Arrival>) -> Drained;
}
/// 一次看队列留给记账线程的东西：账本收下的每一行（按账本次序，交给各份折叠），
/// 与等着目标表答复的登记（按到达次序，交给 `RunWorker::answer_goal`）。
pub(crate) struct Drained { pub(crate) written: Vec<EventDraft>, pub(crate) goals: Vec<GoalAsk> }
```

**为什么 `append` 阻塞**：`kernel::Ledger` 的契约写着「`Ok(ref)` 意味着这条记录在那个适配器的介质里已经耐久」。
一个不阻塞的 relay 会在记录还没落盘时就回 `Ok`，那是把契约改写成「已经排队」——
于是 `run_started` 可能排在它自己那轮活的 `model_called` 后面。阻塞是这条契约的价钱，也是它的全部内容。

**回信通道是 `sync_channel(0)`**（会合信道）：一次 `append` 一个回信地址，不留缓冲，
所以「记账线程写完了」与「池线程知道写完了」之间没有第三种状态。

**签名的一处出入，如实记在这里**：另一种说法是「阻塞等 `Result<EventRecord>`」，
而 `kernel::Ledger::append` 的返回类型是 `Result<EventRef, AxError>`。**以端口为准**——
一致性套件是契约，而 `EventRef` 是那个套件检验的东西。
`EventRecord` 会把整条记录复制过河，`EventRef` 不会，且 `EventRef` 无法伪造（ARCH §9）。

**一致性套件原样通过**：`kernel::ledger::conformance::assert_ledger_conformance` 一个字不改地跑在 relay 上。
套件要求的 `LedgerInspect` 是「只为验证而设」的读回面（`ledger.rs` 的模块文档），
本节因此把它实现在测试里的一层包装上，而不是在 relay 的生产面上开一个读洞：
套件检验的是 relay 的 `append` 路径（seq、prev、字节、两次新实例产生同样的字节），那正是契约。

**服务顺序：relay 请求排在 desk 命令之前**。理由是一个已经在跑、已经花了钱的活，不该排在一条还没开始的命令后面；
反过来排会让一次 `Dispatch` 命令挡住三轮正在写 `tool_result` 的活。

### 8-42-3 `accounting::worker::pool`

形状：**adapter**。N 条 `std::thread`，从 `bin` 里那唯一的 spawn 点起（ARCH §10 规则 3）。
入口 `(RunId, Driving)`，出口 `(RunId, Driven)`。

**线程 panic 不是一种情况**：发布档是 `panic = "abort"`（ARCH §2），没有可以接住的东西；
一次失败作为 `Driven::Failed` 走回来，而不是作为一个 join 出来的 `Err(Box<dyn Any>)`。

**池大小 = `min(准入天花板, 配置值)`**。准入天花板住在记账线程上（`gateway::admission` 已经持有
provider 的并发上限），配置值是人写的。取小的那个：比天花板大的池只会让线程停在 admission 上排队，
那是把排队从一个会算数的地方搬到一个不会算数的地方。**citysim 跑池大小 1**——
六个场景必须逐字节重放同样的账本，而池大小 1 时 `Driven` 的到达顺序就是发起顺序，
于是确定性不依赖调度器。

### 8-42-4 记账线程的三张嘴

人从界面派的活在把 `Driving` 交给一条车道之后就回到主循环，于是记账线程有三张嘴，按这个次序：

1. **relay 请求**（先服务，理由见 8-42-2）
2. **`Driven` 到达**（车道的出口）
3. **desk 命令**（`CommandDesk::wait`）

`settling` 一个字不改：它本来就只在记账线程上跑，触发点从「drive 返回」换成「`Driven` 到达」。
**敲门仍在 settle 之后发出**。落地的类型、幂等键的落定语义与主循环的等待节奏写在 8-46-2。

准入计数住在记账线程上：它是「同时有几轮活在跑」的唯一权威，而唯一权威必须在唯一写者那一侧，
否则两条线程各数各的，就有了两个答案。

**这条循环必须成立的三条性质由 Lean 模型定**：`crates/accounting/spec/Worker/Attend.lean`（`just models`
证明它）。三张嘴与关门都送进同一条队列，线程阻塞在第一条消息上，醒来后把已经排在后面的
一次取尽，按到达次序服务。模型证明：醒来时在等的每条消息都在这一次醒来里被服务（于是到达的消息在下一次
醒来里被服务）；追加的次序就是 seq 的次序；没有消息、也没有到期的排程截止时刻时线程不醒，而每次醒来都
消耗掉至少一件工作，所以醒来的次数以消息数加截止时刻数为上界。模型只管追加了几条记录，一条消息落地时
做的其余事（结账、回话）归 Rust。

**`attend` 就是模型里那一条队列**：`Wake` 每张嘴一个变体再加关门，`RelayGate::serve` 先阻塞在
第一条上，醒来后 `try_recv` 取尽，relay 请求合成一次 `append_all`。desk 仍然自己存命令（运行中的活在
安全点上要从 desk 里找 Cancel），`post` 与 `close` 只往队列里送一条 `Command` 或 `Close` 去唤醒；
线程每次醒来都先服务口子，再看 desk，desk 空时才再睡。一条被 `pursue` 的内层循环吃掉的 `Command`
不会丢命令：命令在 desk 上，外层循环回来时先看 desk，再睡。这个枚举不叫 `Inbox`：词汇表里 Inbox 是
Approval Inbox，人的待答队列。

### 8-42-5 被否决的备选

**备选一：多进程。** 一轮活一个进程，各自持有自己的一份状态，用管道汇总。
**否决理由有三条，任何一条都够。**
第一，账本的 `seq` 与 `prev` 是一条链，`kernel::ledger` 的契约把 seq/prev 的分配交给实现；
多进程要么共享一个写者进程（那就是这里的 relay，只是把 `mpsc` 换成了一个需要序列化、需要重连、
需要处理半个写入的 socket），要么让每个进程自己发 seq（那就有 N 个权威，链断）。
第二，ARCH §1 写的是「一个进程，一个页面」，§6 写的是「一座城是一个目录」——
多进程要给每个子进程一份 git worktree 之外的东西（金库句柄、redb 句柄），而 redb 与 keyring 都不是
可以被两个进程同时打开的东西。
第三，确定性：citysim 用一个种子重放一次跑（ARCH §11 V6），而进程边界会把「谁先写」交给操作系统的调度器，
且这件事在崩溃恢复时不可重现。

**备选二：把一轮活改成 async。** 让 `turn` 与 `drive` 变成 `async fn`，用 tokio 的多线程 runtime 跑 N 轮。
**否决理由**写在 ARCH §2 那张表里，而且是这份设计已经付过钱的一条：
「回合循环是刻意同步的——一个会 await 的决策就是一个会交错的决策。async 停在进程边界。」
把 `drive` 改成 async 会让四个取消安全点（`runtime::turn` 的 typestate）从「四个可以枚举的点」
变成「每一个 `.await` 都是一个点」，而 typestate 之所以能说「相位内部的中断拼不出来」，
靠的正是那四个点是可以数清的。第二条理由是钱：async 会把 `reqwest` 的阻塞客户端换成异步客户端，
而 ARCH §2 说这个 workspace 只有一个 HTTP 客户端，两个客户端就是一个二进制里两套 TLS。
第三条理由是这次并发要的东西 async 给不了：我们要的是**并行驾驶、串行记账**，
而 async 在一个 runtime 上给的是并发交错——交错的是同一条线程上的决策，那恰恰是被禁止的那件事。

### 8-42-6 验收

1. **红转绿**：`the_relay_passes_the_ledger_conformance_suite`——
   `kernel::ledger::conformance::assert_ledger_conformance` 原样跑在 relay 上，
   记账那一侧是另一条线程持有的账本。
2. **红转绿**：`driving/tests` 里两次派活交错——账本 `seq` 保持单调，
   每一轮活的 `run_started` 排在它自己的 `model_called` 之前。
3. citysim 六个场景在池大小 1 下逐字节重放同样的账本。
4. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings` 与
   `cargo nextest run -p sprawling --locked --all-features` 绿。

### 8-42-7 `Driving` 为什么必须拥有它驾驶所需的一切

一条车道是一条线程，所以 `Driving` 的每一个字段都要 `Send`，而其中两个曾经不是：
`bench` 借的是 `ToolBench`，`kernel::Tool` 当时没有 `Send` 上界；`signals` 借的是 `Rc<RefCell<SignalDesk>>`。
两者都不是 `bin` 能单独修好的——工具立面横跨 kernel／runtime／collab／city／agent_protocols／browser 六个 crate。
落地形状写在 §8-44：这六个 crate 的 trait 各加 `Send` 上界，七张桌子换成 `Arc<Mutex<_>>`，
`Driving` 于是拥有自己的 signals 句柄、一份 `Cas` 第二句柄（§8-43）与 backlog 成员号（runtime-SPEC §8-28-2）。
`driving/tests/turns::a_drive_can_be_handed_to_another_thread` 钉住这条性质：`Driving: Send` 是编译期事实。

### 8-42-8 `accounting::worker::booking`——计划认领在调用时由记账线程判定（形状 4 适配器）

每条车道的 `ClaimDesk` 持有派活那一刻的 `Roadmap.md`，并排派出的两轮活读的是同一份文件，所以只凭桌子自己的副本，
两轮活都会认领同一个节点、都把活做完，第二个到落地时才被 `still_true` 丢掉。认领因此在模型调用 `plan claim` 的那一刻
交给记账线程判定：它按队列次序看见每一个认领，先问的拿到节点，后问的当场被拒，一次模型调用都不白花。

```rust
pub(crate) struct Claimant { pub(crate) building: Address, pub(crate) room: Address, pub(crate) run: RunId, pub(crate) who: String }
pub(crate) struct ClaimAsk { /* building、node、roadmap_claimed 那一行的 EventDraft、放回行 effect::Line、回信的 SyncSender —— 私有 */ }
#[derive(Default)]
pub(crate) struct ClaimBook { /* (building, node) → 在飞的 RunId 与放回行 —— 私有 */ }
impl ClaimBook {
    pub(crate) fn answer(&mut self, ask: ClaimAsk, ledger: &mut impl Ledger); // 入账并登记，或拒绝；然后回信
    pub(crate) fn release(&mut self, run: RunId) -> OpenClaims; // 这轮活回家时放开它持有的节点，交出它们的放回行
}
#[must_use] pub(crate) struct OpenClaims { /* run 与 node → 放回行 —— 私有 */ }
impl OpenClaims {
    pub(crate) fn close(&mut self, node: &NodeId);           // 这个节点的一条收尾行已在账上：不再放回它
    pub(crate) fn owed(self) -> (RunId, Vec<effect::Line>);  // 尚未合上的放回行，归在哪一轮活名下
}
// effect.rs：计划那一步的每一行带着它合上的节点；split 行合上它的父节点
pub(crate) struct Closing { pub(crate) line: Line, pub(crate) closes: Option<NodeId> }
// effect.rs：放回行的唯一拼法，陈旧落地的 released() 与车道的认领共用
pub(crate) fn handed_back(claim: &ClaimEffect, note: &str, room: &Address, who: &str) -> Result<Option<Line>, AxError>;
pub(crate) fn booking(bell: mpsc::Sender<Wake>, claimant: Claimant) -> collab::Booking;
```

- **走同一条队列**：认领是 `Wake::Claim`，与 relay 的 append 同在记账线程那一条队列上（8-42-4），而不是第二条通道；
  `RelayGate` 持有 `ClaimBook`，`serve` 在排空队列时逐个答复。车道阻塞在一个 rendezvous 通道上等回信，与 append 一样。
  入账的认领行随 `serve` 的返回值交回，经 `RunWorker::absorb` 进活的 `PlanHolders`，所以那轮活还没回家时，记账线程读到的持有者已经与重启折出的一致。
- **拒词**：`InvalidArgs`，动作 `claim a plan node`，说出节点与持有它的 run，恢复是「list the plan and claim a node that is ready」。
  记账线程已经不在时是 `StorageFatal`，与 relay 的 `gone` 同一形状。
- **先入账再登记**：`answer` 接受认领时，在记账线程上把 `roadmap_claimed` 追加进账本，追加成功才登记节点并回 `Ok`；
  账本拒绝那一行时不登记，拒绝原样回给模型——于是没有哪轮活持有一个历史上看不出它持有的节点。那一行由车道在调用那一刻
  用 `ClaimEffect::kind`／`payload` 拼好（时刻取自 `Claimant.clock`，即 worker 自己的 `accounting::Clock`），归在那轮活的房间下；`Claimant` 是派活时就定下、
  随每次认领一起走的四个值。被拒的认领不留任何一行。
- **登记持续到那轮活回家**：`Flight::arrived` 放开它，而它的落地在同一线程上、在下一次 `serve` 之前跑完，
  所以没有任何认领会在「放开」与「盘上的计划写明节点结局」之间被答复。
- **落地时的 `still_true` 比对保留为兜底**（`effect::Claims::of`）：一条车道在别人落地之后才用旧副本认领一个已经做完的节点，
  这里不拦它，落地时仍被丢弃并告诉人。那条认领已在账本上，所以 `Claims::Stale` 带着给每条认领补的 `roadmap_released`
  （`StopCause::HandedBack`，说明是计划在落地前变了）一起落账；只丢弃不补，`folds::collaboration` 会把那一行读成永远有人占着。
  落地成功时 `Claims::of` 照样重放 `Claimed` 改文本，但不再写它的行，否则同一次认领在历史里数成两次。
- **落地重放效应，不写桌子的副本**：`Claims::of` 把本轮的效应按次序经 `ClaimEffect::apply` 重放到落地时读到的盘上文本，
  `Then::Roadmap` 带着那份文本作基线，经 `Planning::write_plan` 替换；生产里它就是 `city::edit_against`，测试换上一个拒绝的写者，
因为只读文件拦不住 Unix 上目录可写时的 rename，「改写被拒」要在每个 OS 上都成立才能钉住下面那条规则。桌子的副本是派活那一刻的文件，写它会把别的轮在这期间落下的行
  改回派活时的状态；重放只动本轮碰过的行。基线与读盘之间只隔同一线程上的落账，能在这里改动文件的只有城外的写者（人的编辑器），
  那时替换以 `E_VERSION_CONFLICT` 拒绝，行已在账本上而文件未动，错误原样交给 `settle` 的调用方。
- **回家的每一条路都要合上本轮开着的认领**：`roadmap_claimed` 在调用时入账，所以每一条认领都得有一条 `roadmap_released`
  或完成行把它合上，否则重启后 `folds::collaboration` 把那个节点读成永远被那间房占着，`plans` 还会给它发阻塞通知。
  车道在调用那一刻把认领行与它的放回行（`PutDown`，`StopCause::HandedBack`，说明这轮活回家却没有落地）一起拼好交给记账线程，
  `ClaimBook` 登记节点时连放回行一起记下；`Flight::arrived` 放开这轮活时 `release` 交出仍登记着的放回行（`OpenClaims`），
  随 `Home` 交给落地。`settle_desks` 的计划那一步每把一条收尾行写上账本（`Claims::Landed` 的放下／完成行，或 `Claims::Stale` 的
  `released()`），就对那一行的 `Closing::closes` 调 `OpenClaims::close`：`RunWorker::record_closing` 在追加成功之后、各份折叠看见那一行之前合上节点，因为折叠随后拒绝那一行也撤不回账上的收尾行，再补放回行会让历史说一个已完成的节点又被放回。`folds::collaboration` 按每个节点的最后一行判定持有，
  任何一条收尾行都让节点空出来，所以账本在其中一条上拒绝时，已写上收尾行的节点不再放回，只有最后一行仍是 `roadmap_claimed`
  的节点还欠放回行；一次清空整份 `OpenClaims` 会给已完成或已阻塞的节点再补一条放回行。落地结束后，不论成败，`serve_flight` 经 `record_for` 追加余下的放回行（时刻在追加时取，
  各份折叠照常看见它们）：落地在计划那一步之前的任何一个 `?` 上失败——驱动本身返回 `Err`（`driven?`）、目标行、清扫、
  `city::roadmap`、`Claims::of` 拒绝重放——余下的就是本轮全部的认领。计划那一步之后的失败（书架、请求、租约、结论）不再补行，
  因为那时认领已经合上，补一条放回行会让历史说一个已完成的节点又被放回。落地成功而追加失败时返回追加的错误；落地已经失败时
  返回落地的原错误，追加的失败记进诊断——账本已经拒绝过一行，第二次拒绝不改变人要做的事。认领在它的收尾行（finished、blocked、released 或拆分父节点的 split）写上账本时就算关闭——拆分之后这一轮什么也不持有，split 行就是父节点的去向，不等 `Roadmap.md` 改写返回：改写被拒（`E_VERSION_CONFLICT` 或文件不可写）时收尾行已在账上，不再补放回行。
- **目标登记同一条路**（`accounting::worker::registering`，形状 4 适配器）：`goal` 工具的桌子不留目标表的副本，只铸 id、拼 `GoalEntry`，
  经 `registering::booking` 把它作为 `Wake::Goal` 送上同一条队列并阻塞等回信。`serve` 不自己答它，而是按到达次序交回；
  记账线程在给各份折叠看过这一次写下的行之后逐个答复（`RunWorker::answer_goal`）：对着全城的目标表
  `collab::arbitrate`，不撞就先把 `goal_registered` 写上账本，撞了就先写 `goal_conflict`（载荷 `collab::conflict_payload`），
  两种都经 `record_for` 写、经 `absorb` 进活的目标表，然后才回 `Ok` 或 `collab::conflict_refusal`。
  于是并排的第二轮活在调用那一刻被拒，同一轮活的第二次登记也撞上第一次；落地不再写任何目标行（`Landing` 没有目标那一扇门）。
  目标表只由 `goal_registered` 折出，活的与重启的走同一个定义 `collaborating::register_goal`。
  **不在 `serve` 里答**：目标表住在 `RunWorker` 上，`serve` 只借得到账本；在那里答就得给门一份目标表的副本，
  那是第二个「这片地归谁」的答案，同一次排空里的第二个登记还会读不到第一个。
- 验收：`cargo nextest run -p sprawling -E 'test(/second_run_to_ask_for_a_node|two_runs_claiming_one_node_through_the_served_gate|two_runs_landing_different_nodes|a_claim_whose_landing_failed|a_claim_closed_on_the_ledger|a_landing_refused_part_way|a_split_closes_the_claim_on_its_parent|a_claim_booked_through_the_gate|two_runs_registering_one_ground/)'`；
  `cargo nextest run -p sprawling-collab -E 'test(/two_runs_read_as_ready/)'` 在桌子一侧钉住「第二个认领当场被拒、什么都不留」。

## 8-41 一次提交出自哪次运行，从账本回答（`accounting::views::commits`、`sprawling whose`）

这座城作出的每一个提交都带上五条 git trailers 与一个
`<actor>@<city 前 12 位>.sprawling` 的署名。那是**给城外读者的投影**：拿着 `git log`
的人看得见谁写了这一行。反方向还欠着——**手里只有一个 oid 的人，问不出它出自哪次会话**，
而这正是一个人在 `git blame` 之后会问的下一句话。

### 折叠，不是去读 git

```rust
// accounting::views::commits（形状 7 projection）
pub(super) struct CommitFacts { /* run、seq、actor、model、effort —— 私有 */ }
pub(super) fn commit_facts(record: &EventRecord) -> Option<(GitOid, CommitFacts)>;
impl CommitFacts { pub(super) fn answer(&self, oid: GitOid) -> wire::CommitAnswer; }
```

- **两种记录进这张表**：`checkpoint_committed`（键 `oid`，工具波的检查点与基线提交）与
  `pr_merged`（键 `commit`，一次评审落地时 trunk 收到的那个合并提交）。这两种是
  这座城**唯一**会造出提交的两处。
- **派活开场那条 `checkpoint_committed` 不进表**：它是「活钉在这里了」的销钉，载荷只有
  `job`，没有 oid。依据因此是「读得出一个 oid」而不是「是不是这个 kind」——
  一个不带 oid 的检查点行不是一次提交。
- **`run`、`seq` 与 `actor` 取自记录自己的身份**（`EventRecord::run`／`seq`／`addr`），
  `model` 与 `effort` 取自载荷（storage-SPEC §8-18）。**没有一个字段是从 git 读的**：
  投影读权威，不读另一份投影。
- **重复的 oid 后写覆盖前写**：同一个 oid 只可能由同一次提交产生，两条记录说同一件事时
  它们说的是同一件事。
- **`session` 由地址算出而不是另存一份**：房间是地址的最后一段，`city::open_room` 当初
  就是拿人给的名字开的它；派到楼根的运行没有房间，故答 `None`。

### 城外那扇门：`accounting::views::ask`（住 `accounting::views::holding`）

```rust
pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError>;
```

**住 `views` 而不住 `assembly`**：它的全部内容就是「造一份 `Views`、问一句、丢掉」，
而 `Views` 的一生是 `holding` 的；另一个理由是 `assembly.rs` 已站在 400 行预算上（开工时
401 行），而为一行重导出把一道门推得更红是拿门当对手。

一次性折叠这座城的账本并回答一个 Query，然后把视图扔掉。**它与被端上来的城读的是同一条路：
`answer_outside_the_lock` 用的 `Views::prepare` 与 `Prepared::finish`**——若 CLI 自己另写一份读法，同一个问题在这座城里就有两个答案，
而漂开的总是没人看的那一个。链先被 `runtime::replay::verify_ledger_dir` 验过：
历史不成立的城，它的视图不该被端出来。

### `sprawling whose <city> <oid>`（`bin::main::whose`）

四行输出：run、actor（带 session）、model 与 effort、以及账本位置 seq；之后是 8-136 的 `previous` 行，带 `--trace` 时再列区间里的调用。
**自己一个文件而不是 `main::data` 多一个臂**：`data` 里每一个动词不是搬字节就是验链，
而这一个是从城的历史里读出一个答案并把它渲染给人看；且 `data` 已到 360 行，
把四十行放进去得拿别处的行数去换。
- **不接受短 oid**：`GitOid::parse` 只认 40 位小写十六进制，长度不对即拒而不是补齐去猜。
- **退出码**：0 答上了；1 这座城没写过这个提交（`Unavailable`）；2 命令行读不了。
  「城没写过它」是 1 而不是 0，因为一个脚本据此判断「这一行是机器写的吗」时，
  把「不知道」读成「不是」会把审计写成假话。

### 红转绿

`a_commit_the_city_made_says_which_run_wrote_it`（`accounting::worker::driving::tests::ledger`）：
一次真派活跑完，从账本里取出那条带 oid 的 `checkpoint_committed`，用它的 oid 问
`Query::Commit`；答必须给出这次运行的 run、房间地址、模型 id 与档位，以及那一行的 seq。
未落地时这条查询答 `Unavailable`，红就红在这里。

### 文档同步

本节；`wire-SPEC.md` §8-17 与 §2 的 golden（WIRE_V 13→14，Query 15 个）；
`storage-SPEC.md` §8-18；`ARCHITECTURE.md` §7 的两个数与 §12 的 `accounting::views::commits` 一行；
`README.md` 的 History 一节；`tools/adversary/Spec.lean` §2 的线面计数。
公开面：`wire` 增 `Query::Commit`／`Answer::Commit`／`CommitAnswer`，
`storage` 增 `Provenance::model_fields`／`model_choice_of`／`effort_word`，
`sprawling` 增 `ask`，故 `api-baselines` 三份随之重算。

**接线到哪一步了，以及四处 `#[expect]` 的账**：记账那一侧已经接上——`spawn_worker` 在循环外开一个 `RelayGate`，
循环里第一件事是 `worker.serve_relay(&relay)`，然后才 `worker_desk.wait(...)`，
于是「relay 请求排在 desk 命令之前」这条规矩现在由代码持有而不是由一段文字持有。
池那一侧还没有生产调用方，于是 `Relay`、`RelayGate::issuing`、`RelayGate::issue` 与 `gone` 在非测试构建里是死代码。
这里的处理是四处 `#[cfg_attr(not(test), expect(dead_code, reason = …))]`——
**用 `expect` 而不是 `allow`，正是因为它会自己清掉**：池一落地，这四条期待就变成「未兑现的期待」而编译失败，
删掉它们是那一步必须做的事，而不是某个人必须记得的事。

### 城市立起来时就有市政厅，和一条写下来的代答

**`form_city` 在 line zero 之后多做三件事**，顺序固定：

1. 追加一条 `autonomy_changed`，值 `delegate:hall/clerk`。**不改 `AUTONOMY_DEFAULT`**：缺省值说的是「没人说过话时怎么办」，而这里是这座城市作出的一个决定，人可以改它，改它要有一行历史可改。`folds` 读回这条线，重启后 clerk 依旧是代答者，无需第二处记忆。
2. 用 `city::CityPlan::new(None).hall()` 拿到那栋楼，走 `create_building` 落 `RULES.toml` 与脊柱文档，并记 `building_created`。走这扇门而不是另写一段，是为了让市政厅与任何一栋楼在历史里长得一样。
3. `city::lay_out_hall_identities` 把 `MAYOR.md` 与 `CLERK.md` 写进 `<city>/.sprawling/`，已存在的不覆盖。

**影响面（一处真实回归，已改）**：从此每座城市至少有两栋楼。`views::tests` 里两处按 `buildings[0]` 取楼的断言改成按地址找 `lab`——它们原本靠「城里只有一栋楼」这个此后不再成立的前提。改的是测试对现实的假设，不是把依据放宽。

**留给后续卡**：`hall` 的居民目前拿到的仍是 `workbench::tools` 给所有人的同一套工具表，`exec`／`delegate`／`workshop` 都在里面；这张表按地址裁，而不是只由写域挡住（`Documents` 拒非 `.md`），不由工具表挡住。

## 8-136 `sprawling whose --trace`：一个提交倒推到产生它的调用与居民（`bin::main::whose`；accounting-SPEC.md 8-16）

**形状。** `main/whose.rs` 仍是 adapter：读命令行，不带 `--trace` 时问 `accounting::views::ask`，带 `--trace` 时问 `accounting::trace::trace`，把答写成几行给人读。写 stdout 失败时与 `view` 一样：读者提前停下（`| head`）退出 0，别的写失败退出 1。

**输出。** 8-41 的四行（`run`、`actor`、`model`、`ledger`）与各 `replaced` 行之后，总有一行 `previous`：`previous <oid> at seq <n>`，这个 run 的第一个提交写 `previous none, the run's first commit`。带 `--trace` 再写：

- `span    after seq <n>, before seq <m>`；没有 `previous` 时是 `span    from the run's first line, before seq <m>`。
- 每条调用一行 `call    seq <n>  <called 的 iso>  <工具>  <effect>  <结局>  <subject>`，结果有输出时下一行是 `        > <输出的第一行>`。`effect` 与结局按它们的 serde 拼法写（`read`、`{"write":{"domain":"lab"}}`；`answered`、`failed`、`waiting`），没有登记的工具写 `unregistered`；没有 subject 时取参数的第一行，两者都没有写 `-`。区间里一条调用都没有时写 `call    none`。版本早于逐行时刻的账本里，`called` 是回合的时刻。
- 同楼的每个别的 run 一行：`nearby  <run> at <actor>: <k> call(s)`。最后一行 `note    the calls are candidates; a nearby run may have written in the same span`，只在有 `nearby` 行时写。

**退出码。** 同 8-41：0 答上了；1 这座城没写过这个提交；2 命令行读不了。

**测试。** `whose_trace_names_the_run_its_calls_and_who_else_called_in_the_building`（`bin::main::tests`）在一座城的账本上逐字节比输出；区间与配对的规则由 `accounting::trace::tests` 钉住。

## 8-43 筛子接进产品：`driving` 把 `exec` 结果经 `pipeline::package` 交给模型（`accounting::worker::driving`、`runtime::pipeline::exec`）

**量出来的现状**：runtime-SPEC §8-27 的筛子完整落地，`pipeline::package` 也已带 `SieveRequest` 臂，但它在产品里没有调用方——`accounting::worker::driving` 的 `invoke` 钩子把 `BenchOutcome::Ran` 的结果原样交回 `runtime::turn`，于是压缩器只在 citysim 跑，城里的模型读的是 `cargo check` 的一千两百行原文。§8-27-9 末尾的「已知未接」说的就是这一处。

### 一个门，两个调用方

citysim 的 `citysim::sieving::package_exec` 是「一份 `exec` 结果怎样变成模型读到的东西」的第二份定义，产品接线若再写一份就是第三份。这里把它搬进 runtime，一个权威、两个调用方：

```rust
// runtime::pipeline::exec（形状 1 判定；文件 crates/runtime/src/pipeline/exec.rs）
pub struct SieveSite<'a> { pub offload: OffloadSite<'a>, pub table: &'a FilterTable, pub history: &'a mut SieveHistory }
/// 一份 exec 结果：stdout＋stderr 合成一段文本，按命令键过 package；
/// 结果里 stdout／stderr 换成 content，其余字段（arm、exit_code、env、background、handle……）原样留着，
/// 再加 sieve:[result_offloaded 载荷]。没有 stdout 也没有 stderr 的结果（backgrounded 形）原样返回。
pub fn package_exec(call: &ToolCall, outcome: ToolOutcome, site: SieveSite<'_>, stamp: Option<ClockStamp>) -> Result<ToolOutcome, AxError>;
pub const EXEC_CAP_BYTES: u64 = 16_384;   // citysim 一直用的那个值，现在只写一次
```

citysim 的 `sieving.rs` 改为调它；旧函数删除（迁移做完，不留适配层）。

### 装配层持有的三样东西

| 东西 | 谁持有 | 何时定 |
|---|---|---|
| `FilterTable` | `Site.filters` | `stand_up` 读 `<city>/.sprawling/FILTERS.toml` 与 `<building>/.sprawling/FILTERS.toml`，走 `FilterTable::resolve`（楼＞城＞内建，整值覆盖）。文件不存在＝`None`；读不到＝错误（§8-26：读不到不等于写错） |
| `SieveHistory` | 一次 `drive_dispatch` 内的局部 | 每跑一份；跨调用差分只在本 run 内成立 |
| `OffloadSite` | `Driving.sieving`：一份 `Cas` 第二句柄＋rest 目录 | rest 目录是 `<write_root>/<addr>/.rest`——`read` 只放行模型选的非保留路径，rest 文件放在保留区里就是给模型一个它够不着的地址 |

`Cas` 开第二个句柄而不借工人的：CAS 按内容寻址、经临时文件写入，同一目录开两次是同一个库；而 §8-42 的池线程不能借工人的任何东西，这一份句柄正是它以后要带走的。

**`stamp` 由这一跑的 `StampGate` 给**：`Sieving` 持有它与这一跑的 `ClockReading`，打包前问一次（8-125）；citysim 的闭包工具面按同一扇门自己问它的那一个。

### 验收

1. **红转绿（`driving/tests/sieving`）**：一次真实派活，`exec` 打印一份超过 2 KiB 的输出；模型收到的工具结果含 `[sieve:` 页脚且短于原文；账本 `tool_result` 载荷的 `sieve[0].original` 是 `cas:b3-` 且能从 CAS 读回原文，`rest_path` 在磁盘上。
2. `citysim::sieve::the_window_holds_the_diagnostics_and_the_way_back_and_only_the_news_the_second_time` 绿。
3. 同种子 citysim 逐字节重放不变（`the_same_seed_and_table_replay_a_byte_identical_window`）。

**留给 gitignore 的一句**：`.rest/` 住房间里，楼的 `.gitignore`（city-SPEC §8-21）应忽略它，否则检查点提交会把一份 rest 文件收进历史。那份文件不在本节范围内。

## 8-44 `Driving` 跨过线程：`kernel::Tool` 加 `Send`，五张桌子从 `Rc<RefCell<_>>` 到 `Arc<Mutex<_>>`（§8-42-7 量出来的前置条件）

**问题**：§8-42-7 逐字段量过——`Driving` 今天跨不过线程边界，卡住它的是两件不在 `bin` 里的事实：
`kernel::Tool` 没有 `Send` 上界，于是 `ToolBench.tools: BTreeMap<String, Box<dyn Tool>>` 不是 `Send`；
五张桌子（signals／goals／plan／shelf／pr，加 delegates 与 workshop 两张只在 workbench 里的）全是 `Rc<RefCell<_>>`，工具持有它们的克隆。
没有这次拆分，池只能写成没有第二实现的空壳。

### 依据：什么要变、什么不变

| 东西 | 今天 | 此后 | 理由 |
|---|---|---|---|
| `kernel::Tool` | `pub trait Tool` | `pub trait Tool: Send` | `Box<dyn Tool>` 由此自动 `Send`；一个不能跨线程的工具在这座城里没有位置——它会被池线程调用 |
| `agent_protocols::mcp::Outbound` | 无上界 | `: Send` | `McpTool` 持 `Box<dyn Outbound>`；两个适配器（stdio 子进程、HTTP 客户端）本来都是 `Send` |
| collab 七张桌子的句柄 | `Rc<RefCell<Desk>>` | `Arc<Mutex<Desk>>` | 桌子本身没有 `Rc`，换句柄不换桌子 |
| `runtime::ReadTool.catalog` | `Rc<RefCell<Catalog>>` | `Arc<Mutex<Catalog>>` | 同上 |
| `runtime::StatusTool.children` | `Box<dyn Fn() -> Vec<ChildStatus>>` | `+ Send` | 闭包持派生台句柄 |
| `agent_protocols::mcp::stdio` 连接 | `Rc<RefCell<Connection>>` | `Arc<Mutex<Connection>>` | 同上 |
| `accounting::worker::workbench::{Desks, Workbench, Reach}` | `Rc<RefCell<_>>` | `Arc<Mutex<_>>` | 出借与收回的地方 |
| `Driving.signals` | `&Rc<RefCell<SignalDesk>>` | `Arc<Mutex<SignalDesk>>`（拥有） | 池线程不借工人的东西 |
| `storage::vfs::Vfs`（内缝） | 无上界 | `: Send` | 红测试量出的第七处：`Cas` 持 `Box<dyn Vfs>`，而 §8-43 让 `Driving` 带一份 `Cas`；`RealFs` 本来就是 `Send`，`FaultFs` 的 `Rc<RefCell<State>>` 换 `Arc<Mutex<State>>` |

**`try_borrow_mut` 失败 → 锁中毒**：`RefCell` 的「桌子在用」拒绝换成 `Mutex::lock` 的阻塞——那正是要的语义：两条线程同时到一张桌子前，后到的等，不是被拒。
`lock()` 的 `Err` 只有一种含义——持锁线程 panic 了——而发布档 `panic = "abort"` 下它不会发生；映射成 `E_STORAGE_FATAL`「桌子被一条死掉的线程留在锁里」，与 `runtime::backlog::hold` 同一句话。

**不变的**：桌子的内容、每张桌子的 `take_effects`／`take` 语义、`settle_desks` 的顺序、账本上的每一个字节。
这是一次句柄类型的迁移，不是一次行为变更；citysim 六个场景逐字节重放不变是它的验收。

**红测试**：`accounting::worker::driving::tests::turns::a_drive_can_be_handed_to_another_thread`——
`fn crosses_threads<T: Send>()` 对 `Driving<'static>`；今天这一行不编译（`Rc<RefCell<SignalDesk>>` cannot be sent between threads safely），改动后编译并通过。
类型层面的红转绿正是 ARCH §9「unrepresentable 本身是需要测试的断言」那一条的用法。

**迁移一次做完**（AGENTS.md「完成每一次迁移」）：每个读者与写者一起搬，旧形状删除，不留 `Rc` 版本的构造函数。

### 8-40 花费闸删除后的装配面

`Assignment`／`Given`／`Knock`／`Sent`／`BlockedJob` 五个结构各去掉一个 `budget` 字段，`DISPATCH_TURN_BUDGET` 与 `RunPlan.budget_turns`／`RunPlan.budget` 一并删除，`run_started` 载荷不再写 `usd_micros` 与 `tokens`，`JobBrief` 不再有 `budget` 一节，`StatusTool` 的十三字段变十二。

- **一条派活不再有回合上限**，`runtime::run::drive` 循环到这次跑自己结束为止：一回合作出结论、一次带 carrier 的失败、或一个安全点送到的中断。停一件正在跑的事仍是 `Cancel`，停一片仍是 `Halt`——后者现在真的会终止那片里的后台成员。
- **`assembly/freezing/tests/ceilings.rs` 删去两条断言**（派下去的活与被批准接着跑的活各自「在派它的上限下」跑）。它们检验的性质不存在了，留着就是在检验一个没有主语的句子；文件保留 effort 那一条，模块头写明删了什么、为什么。
- **golden-p0 账本随之重生**（`GOLDEN_WRITE=1`）：`run_started` 少两个整数键。V8 跨版本字节夹具本来就为这种形状变更而存在。
- **未做（不在此范围）**：`tools/xtask/api-baselines/` 下 kernel／wire／web／sprawling 四份基线需 `just api-baseline` 重生——kernel 去掉 `BudgetCap` 一族、增 `GovernedDocumentWritten`，wire 去掉 `BudgetCap` 再导出、增本线三帧与三个答面类型，web 增 `put_document_command`。

### 8-41 治理两帧的执行与答

- `RunWorker::put_document` 写 `<city>/.sprawling/` 下三份文件之一（经 `city::write_governed`，路径由 `city::Governed` 决定而不由帧决定），随后记一行 `governed_document_written`，载荷携文件名与字节数、**恒不携正文**——正文在盘上可读，抄进账本就是同一段话有了两个权威。
- `Views` 新增两个字段：`autonomy`（折自 `autonomy_changed`）与 `decided`（折自 `approval_resolved`，旧在前）。两者一起答 `Query::Governance`。`decided` 收下每一条被答过的审批，包括人自己答的——只列代答的清单会让「我答过」与「从没人答」在界面上长得一样。
- **`views::apply` 迁入 `views::holding`**：`answering.rs` 加上这两臂后越过 400 行，而折叠本来就是 `holding` 自称拥有的东西（「what the views hold and how one record folds in」）。切完 `holding` 318、`answering` 335，无新模块行。

### 8-42 `Query::Hunks` 的答

`views::answer` 的新臂调 `storage::of_file`，把 `storage::PatchLine`／`Withheld` 逐字段搬成线上的同名形状。这座城没写过的 oid 答 `Unavailable`，与 `Changes`／`Commit` 同口径：「没有变化」与「我看不了」是两个答案，读的人对它们的下一步不同。

## 8-45 一个能看见自己造出来的东西的居民（`bin::browser_bidi`、`bin::browser_tool`）

### 8-45-1 谁按启动键

`browser` crate 是纯的：帧进帧出，无套接字、无进程、无异步。启动一个引擎与端着一条 WebSocket 因此落在装配层，这正是 ARCHITECTURE §3 的「装配边」——运行期存在、只在 `bin` 里存在的那一类边。

**Firefox 是第一引擎**：它原生说 BiDi，不需要任何驱动，所以一台只装了 Firefox 的机器就是一台能用的机器。命令行三件：`--remote-debugging-port <随机端口>`、`-profile <这栋楼的 profile 目录>`、按需 `-headless`。端口随机是因为同一台机器上可能有第二座城在开着第二个浏览器，而一个固定端口会让第二座城静默连到第一座城的浏览器上。

**Chromium 是第二条路**，且只在 `chromedriver` 已在 PATH 上时存在。这不是并列的两个后端：Chromium 的 BiDi 要经 `chromedriver` 转，驱动不在就是不在，此时回一句点名的拒绝而不是沉默降级。

```rust
pub(crate) enum Engine { Firefox { program: String }, Chromium { driver: String } }
pub(crate) struct LaunchPlan { pub(crate) program: String, pub(crate) args: Vec<String>, pub(crate) port: u16 }
impl Engine {
    pub(crate) fn plan(&self, profile: &Path, port: u16, headless: bool) -> LaunchPlan;
}
```

**本模块是三个文件**（400 行的价目表逼出来的一次切分，切在三件事之间而不是切在行数上）：`engine.rs` 判「哪个引擎、命令行长什么样」，`lazy.rs` 持「还没起的那个引擎」与端口推导，`socket.rs` 只运字节。`browser_bidi.rs` 因此是索引，一行逻辑也没有。

`plan` 是纯函数，端口由调用方给，于是「参数长什么样」这件事在没有浏览器的机器上也逐字可断言；`launch` 只是 `Command::spawn` 加一个「进程死了就报出来」。**时间与随机都不在这里取**（ARCHITECTURE §10 第 2、4 条）：端口由 `port_for(city_root)` 从城目录的 BLAKE3 摘要推出，落在 40000–59999。这既不采时钟也不取熵，而同一台机器上的两座城本来就在不同目录里——用已经把它们区分开的那件事去区分端口，比再引入一个随机源更少一处不确定性。等待浏览器起来靠**敲门次数**而不是截止时刻，因为读时钟的地方只有 `bin::assembly` 一处。

### 8-45-2 `bin::browser_tool`——一张动作表，一个会话

工具住这里而不是 `browser` crate，理由是截图要落 `storage::cas`，而 `browser` 依赖图里没有 `storage`，也不该有。把 CAS 塞进 browser 会多一条本可不存在的依赖边；把工具放在装配层，`browser::verb` 的判定与 `storage::cas` 的字节各自留在自己那侧，中间只有一个 `Shot` 值。

工具持有：一个 `Box<dyn BrowserPort>`、一个 `Session`、当前 `ContextId`、上一次 `PageSnapshot`（快照的 generation 由它递增）、一个 CAS 句柄。动作即 `browser::verb::Verb` 的变体（`crates/browser/Spec.lean` 的 D5），一个不多一个不少；`fetch` 的回复在这里变成 `status`、`type`、`url`、`text`、`cut`、`chars` 六个字段的结果，非文本的回复变成 `binary` 与 `bytes`，页面自己的拒绝变成 `error`。

`effect` 是 `Effect::Egress`：浏览器打开的每个 URL 都离开运行中的机器，所以它过出网门，confidential 楼因此天然拿不到它。

**`act` 的结果说出它落在哪里**（`browser_tool::answering`）：`acted` 一格是快照给的 ref；指针动作没有 ref 时是 `viewport`；按键（`Action::Press`）是 `focus`，因为一次按键落在页面此刻的焦点上，不带 ref，也不碰视口（`crates/browser/Spec.lean` 的 D12）。三个词按 `Action` 的臂穷尽给出，不从「有没有 ref」反推——那样按键会被记成 `viewport`，读账本的人看到的是一次没发生过的指针动作。

**`usersbrowser` 披露的参数表与 `Verb::read` 读的参数一一对应**（`browser_tool::person`）：`kind` 的枚举含 `press`，`key` 与 `modifiers` 各有一格；键名与修饰键名的清单不抄进描述，描述只说它们取 DOM `KeyboardEvent.key` 的名字，未知的名字由 `Key::parse`、`Modifier::parse` 拒，拒词列出能用的名字——清单的家仍是 `browser::keyboard` 一处。同一个参数名只写一格：`ref` 与 `refs` 的说明把 `act`、`measure` 与 `screenshot` 写在同一句里，因为 JSON 对象里后写的同名键会静默盖掉先写的，模型只会读到后一种用法。

**worker 经它被交到的 `fn` 指针拿这些工具**：

```rust
/// 楼的规则要的浏览器工具：先是楼自己的，再是这个人声明过的那一个。
pub type Browsers = fn(&Path, &storage::BlockOrigin, &city::BuildingRules) -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;
impl RunWorker {
    pub fn with_browsers(self, browsers: Browsers) -> RunWorker;   // 生产装 browser_tool::for_rules
}
```

`lay_out_workbench` 调的是 `RunWorker.browsers`，不直接调 `browser_tool::for_rules`：本模块起浏览器、经 BiDi 说话，worker 搬进 `accounting` 时它留在 `sprawling`（accounting-SPEC.md §7、§12-10）。换掉它放不宽机密：`city::policy` 在机密楼上拒绝 `browser` 与 `usersbrowser` 两项设置，楼的规则里就没有要浏览器的那一句。钉住它的测试是 `crates/sprawling/tests/browsers.rs` 的 `a_run_is_offered_the_browser_the_worker_was_handed`。

### 8-45-3 截图成为证据

一次 `screenshot` 的落点有三处，缺一处这张图就不是证据：

1. 字节进 `storage::cas`，得到一个 `cas:b3-…` 定位符——历史里恒不出现图片字节；
2. 结果载荷带上定位符与两个整数尺寸，于是模型即使不看图也知道它有多大；
3. `ToolOutcome.attachments` 带上 `ImageRef`，`runtime::turn::wave` 把它原样放进 `ContentBlock::ToolResult.attachments`，于是这张图真的到得了模型眼前。

**`kernel::ToolOutcome` 因此加一个字段** `attachments: Vec<ImageRef>`，`#[serde(default)]`，旧历史读成空列表。这是唯一一处跨 crate 的形状变更，波及每一个构造 `ToolOutcome` 的工具（全部改为显式空列表），不改任何一个的行为。`ContentBlock::ToolResult.attachments` 与两条 dialect 备好，`wave.rs` 里那句「a tool that produces a picture fills this in where it runs」等的就是这一步。

### 8-45-4 `RULES.toml` 的 `browser` 的真假两值

`city::policy` 多读一个键。默认 **false**：一栋楼不写这行，它的居民就没有浏览器。这与 `confidential` 的「不写即报错」不同，理由是两者的失败方向相反——隐私设置读成宽松的一侧是事故，而工具没给到只是少一件工具。confidential 楼恒为 false，写了 `browser = true` 即拒，因为一个能开任意 URL 的浏览器就是一条出网路径，而「数据不出去」是那栋楼的全部意思。

### 8-45-5 验收

| 单元 | 完成的定义 |
|---|---|
| browser_bidi | Firefox 与 Chromium 的参数各自逐字断言；驱动不在即点名拒绝；两次 plan 的端口来自参数而非采样 |
| browser_tool | 录制适配器上重放 open→snapshot→act→screenshot 一整条；截图后 CAS 里有字节、载荷里有定位符与尺寸、attachments 里有一个 `ImageRef` |
| RULES.toml | 不写 `browser` 即没有；confidential 楼写 `browser = true` 即拒 |

## 8-46 同一栋楼里的并发：驾驶池、一次派活切成三段，与拿走整个 ready set 的 `pursue`

这一节把 §8-42 的设计落成类型：一条车道是什么（8-46-3）、一次派活怎么切（8-46-2）、
一个追求怎样拿走整个 ready set（8-46-4）。前置是 §8-44：`Driving` 要能跨过线程边界。

### 8-46-1 `Driving` 拥有它驾驶所需的一切，`drive_dispatch` 变成自由函数

§8-44 让 `Driving<'static>: Send` 成立，但今天构造出来的 `Driving<'a>` 仍借着三样东西：
`&mut workbench.bench`、`&site.write_root`、`&site.who`，而 `drive_dispatch` 还是 `&mut self` 的方法，
用着工人的 `ledger`／`watching`／`interrupts`／`backlog`。一个借着调用栈上局部变量的值送不进线程。

| 字段 | 今天 | 此后 | 理由 |
|---|---|---|---|
| `bench` | `&'a mut ToolBench` | `ToolBench`（拥有） | `Workbench.bench` 改为 `Option<ToolBench>`，由 `take_bench` 取走一次。与 `Site.adapter` 同一手法：`Option` 是搬运的车，不是新状态 |
| `write_root` | `&'a Path` | `PathBuf` | 一次 clone，一次驾驶 |
| `who` | `&'a str` | `String` | 同上 |
| `plan`／`handoff` | `drive_dispatch` 的两个参数 | `Driving` 的两个字段 | 驾驶要的东西在一个值里，池的入口才是一个值 |

`drive_dispatch` 拆成两半：

```rust
/// 一次驾驶从工人那里拿走的四样东西，克隆而不借。
pub(super) struct DriveContext {
    watching: Option<Arc<dyn Fn(wire::Delta) + Send + Sync>>,
    person: Option<Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    backlog: runtime::Backlog,
}

/// 一次驾驶，泛型于它写进哪个账本：记账线程上是 `JsonlLedger`，
/// 车道线程上是 `Relay`，而两条路上跑的是同一段代码。
pub(super) fn drive_run<L: Ledger>(driving: Driving, ledger: &mut L, context: DriveContext) -> Result<Driven, AxError>;
```

**`interrupts` 从「借走一个」变成「每轮活各持一份」**：`RunWorker.interrupts` 由
`Option<Box<dyn FnMut(RunId) -> Interrupt + Send>>` 改为 `Option<Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>`。
`CommandDesk::interrupt_for` 本来就取 `&self` 并按 `RunId` 挑命令，所以 N 份克隆指向同一张桌子，
`steer` 与 `cancel` 各自走自己那一轮活的 `Interrupt`，这是 §8-42-1 早就写下的形状。
`take()`／放回那一对动作随之删除：一个被借走的钩子在 N 轮活同时跑时只有一个借用人。

### 8-46-2 一次派活切成三段，在飞的那些收成一张表

```rust
/// 驾驶之前城已经做完的一切，与驾驶之后要用到的一切。
pub(in crate::assembly) struct Continuation { at: Assignment, site: Site, desks: Desks, workbench: Workbench, job_locator: Locator, member: Option<runtime::BacklogId> }

fn stage_dispatch(&mut self, at: Assignment, task: String, goal: String) -> Result<(Staged, Continuation), AxError>;
fn land(&mut self, continuation: Continuation, driven: Result<Driven, AxError>) -> Result<Dispatched, AxError>;
```

**在飞的活只有一张表**，住在 `RunWorker.flight`（`accounting::worker::driving::flight`）：车道（`DrivingPool`）、
车道写历史的那道口子（`RelayGate`）、以及每一轮在飞的活欠着什么。

```rust
pub(in crate::assembly) struct Flight { pool: DrivingPool, gate: RelayGate, driving: BTreeMap<RunId, InLane> }

/// 一轮活回家之后城还欠着什么。**七个派活入口只在这里不同。**
pub(in crate::assembly) enum Owed {
    /// 人派的活：它谈过话的邻居接着答。
    Asked,
    /// 追求认领的一行计划：它回家时那一行若仍然 ready，追求停下而不是再派一次。
    Row { addr: Address, node: NodeId },
    /// 城自己起的活：排程、外来到达、敲门、批准放行。没有人在等答案。
    Unasked(Unasked),
    /// 某轮活交下来的子活：干完之后由提问的房间收 handback。
    Child { parent: Address },
}

/// 城自己起一轮活的四个理由。起不来时诊断行按理由归因，故是穷尽枚举而不是一个 bool。
pub(in crate::assembly) enum Unasked { Schedule, Arrival, Knock, Unblocked }

/// 城欠这轮活什么、拒绝回哪儿去、这条链已经走了多远。三者同行，因为义务比承载它的
/// 那轮活活得久：继任者对的是「谁要了被它替换的那轮活」，而链条的上限要看得见它
/// 已经走了几跳。`reply` 用 `Arc` 共享而不是搬走，因为落地要在把义务交给继任者
/// 之后仍能回一句拒绝。两个计数器住 `Owing` 而不住 `Owed`：它们是义务的属性而不是
/// 欠着什么这件事的属性，`Owed` 的每个读者（`discharge`、`rows_of`）也不必为它们
/// 多改一处 match。
pub(in crate::assembly) struct Owing { owed: Owed, reply: Arc<wire::Reply>, relays: Relays }

/// 一次「服务口子＋接一轮活回家」做了什么。
pub(in crate::assembly) enum Landed { Nothing, Row { addr: Address, node: NodeId }, Elsewhere }

fn dispatch_into_lane(&mut self, at: Assignment, task: String, goal: String, owing: Owing) -> Result<RunId, AxError>;
fn start_unasked(&mut self, addr: Address, task: String, goal: String, because: Unasked) -> Option<RunId>;
fn land(&mut self, continuation: Continuation, driven: Result<Driven, AxError>, owing: Owing) -> Result<Landed, AxError>;
fn serve_flight(&mut self, wait: Duration) -> Result<Landed, AxError>;
```

**一张表而不是两张，改掉的是第二个权威**。窄形里 `pursue` 自带一个池与一道口子，主循环若再开一套，
`DRIVING_LANES` 就有两个主语（一座城因而能同时跑八轮），而主循环服务不到 `pursue` 那道口子——
一条车道在它的 append 上停多久，取决于另一条线正好在做什么。收成一张之后：车道数是一个数，
口子是一道，`pursue` 与 desk 派的活在同一批车道里排队，谁回家由 `Owed` 决定接着做什么。

**幂等键在「活起飞」那一刻落定**（§8-41 欠的那条语义）。理由：`Dispatch` 这条命令做完的事就是
**让一轮活跑起来**，跑本身不是命令的一部分。落在起飞：同一个键的第二帧在活还没跑完时到达，
`Entrance` 认得它，于是不会有第二轮活——这正是那扇门存在的理由。落在落地则反过来：
键要在 `serve_one` 返回之后继续被 `carrying` 持有，而 `carrying` 是「此刻正在写的记录该盖谁的章」，
两轮活同时在飞时它答不出一个。

**主循环只在消息到达与排程截止时刻醒来**（8-42-4）：车道的 append、回家的活、desk 上的命令都把线程
唤醒，所以一条车道不必等定时器才前进；desk 刚交出一条命令时下一次看队列不睡（desk 上可能还有），
其余时候最多睡到下一个排程截止时刻（`SCHEDULE_TICK_MS`，20 s），一座闲城就是闲的。排程因此自带节律
——有活在飞时 `DeskWait::Idle` 随每次 relay 往返而来，而一座每次都打开排程文件的城把时间花在开文件上。

**关城要把车道等回来**：`DeskWait::Close` 之后不再接新活，但口子照服务、回家的照落账，直到没有活在飞，
然后才写交接。一条停在 append 上的车道被丢下，丢掉的是城已经答应它耐久的那些行。

**七个入口全部进车道，`dispatch_in` 退出生产（H-04）**。此前只有 `Command::Dispatch` 走车道，
而排程（`tick`）、外部到达（`wake`）、敲门（`answer_knocks`）、委派子活与继任（`conclude`）、
批准放行续活（`answer_approval`）六个入口调同步的 `dispatch_in`，在记账线程上跑完整个 drive。
代价不是「有界的等待」而是命令台整段关闭：`serve_flight` 只在 attending 的主循环里被调用，
一次同步 drive 持续几分钟，期间在飞的车道全部停在 relay 追加上，`Halt` 与 `Cancel` 也读不进来。

改法是「谁在等它」由 `Owed` 回答，于是那个问题不再拦路：

| 入口 | `Owed` | 落地时做什么 |
|---|---|---|
| `Command::Dispatch` | `Asked` | 醒来的邻居接着答 |
| `pursue` 的一行 | `Row { addr, node }` | 该行仍 ready 则追求停下 |
| `tick`／`wake`／`answer_knocks`／`answer_approval` | `Unasked(_)` | 无人可答，起不来时记一条 `Refuse` 诊断 |
| 委派子活 | `Child { parent }` | 向提问的房间投 handback |
| 继任 | 继承前任的 `Owing` | 义务随活走：链条结束时才兑现 |

**义务跟着活走，不跟着轮次走**：继任者是同一件工作接着做，所以要 handback 或要回信的那一位，
等的是链条的末端而不是每一环。`Owed` 因此不需要 `Successor` 这一格——前任的 `Owing` 原值搬给继任者即可，
谁被替换由 `Assignment.succession.predecessor` 说，一个事实一个家。

**污点随派活走，不随工人走**：`wake` 从前置 `RunWorker.tainted_arrival`、派完再清；
活进车道之后清旗标的那一刻远在落地之前，被清掉的正是那轮外来活自己的 C15 标记。
污点因此成为 `Assignment` 的字段 `taint: kernel::TaintSet`，子活与继任者继承它。
写点给出真实来路：`waking` 把到达的来路记成标签 `arrival:<source>`，装进 `Unasked::Arrival(TaintSource)` 交给
`driving::flight::start_unasked`，由 `Unasked::taint` 一处换成 `TaintSet`；其余派活写 `TaintSet::empty()`。
标签与 `runtime::bench::outside` 的 `mcp:<server>`、`web` 同一种写法，拒绝文案逐个念出来路，
模型与人都看得见是哪一处外来内容挡住了命令。
`lay_out_workbench` 把 `Assignment.taint` 原样放进这张桌子的 `TaintSet`，不再另造标签，
于是桌子问的每一道门都看得见它，`gate::command` 据此拒掉 `exec`（`E_TAINTED_ACTION`）。
读点 `settling::landing` 转交 `at.taint`，待批项的 C15 标记取 `!taint.is_empty()`，一个事实一个家。

**车道满不是拒绝，是排队**：`DrivingPool::start` 对每一个派活入口都是同一道闸——人派的活、
城自己起的活、敲门、委派、workshop、继任、追求，谁都一样。车道满时，已经准备好的那一轮活
（`Driving` 连同它的写口与 `DriveContext`）进池里的先到先起队列，不开线程；`landed` 空出
一条车道之后，`serve_flight` 在落地这一轮之前调 `start_waiting`，按到达顺序把排着的活起到
车道满为止。排在前面的先起，所以落地时新派的子活、敲门排在已经在等的活后面。
`in_flight` 只数车道里的，排队的不算；因为只有车道满才会排队，「有活在排」必然意味着
「有车道在跑」，`land_the_rest` 的循环条件因此不变。起不来的那一轮（线程开不出）离开
在飞表，拒绝交给它欠着的那一位。队列放在池里而不是放在 provider 的 admission 上，因为
池数得清谁在等，admission 只让线程停着等，一个 20 节点的 workshop 就是 20 条线程。

```rust
impl DrivingPool {
    fn start(&mut self, driving: Driving, ledger: Relay, context: DriveContext) -> Result<(), AxError>;
    /// 车道空出之后，按到达顺序起排着的活，直到车道满；返回起不来的那几轮与各自的拒绝。
    fn start_waiting(&mut self) -> Vec<(RunId, AxError)>;
}
```

**`handle` 仍然是同步的一扇门**：`RunWorker::handle` = 一条命令 ＋ `land_the_rest`，
于是命令行与测试看到的仍是「调用返回即事情做完」，而落地过程中起的子活、敲门与继任
都在同一次 `land_the_rest` 里排空。

### 8-46-3 `accounting::worker::pool`

形状：**adapter**（ARCH §9 第 4 种）。文件 `crates/accounting/src/worker/pool.rs`。

```rust
/// 一轮跑完的活回到记账线程时带的两样东西。它们成对，因为
/// 一个 `Driven` 不说自己属于哪一轮，一个 run id 也不说要归位什么。
pub(crate) struct Arrival { run: RunId, driven: Result<Driven, AxError> }

pub(crate) struct DrivingPool { /* lanes、回家的那一头、每条车道的 JoinHandle、排着的活、read_memory */ }
impl DrivingPool {
    /// `read_memory` 是池判断内存紧不紧时唯一的读数来源；生产交 `bin::monitor::memory::read`。
    pub(crate) fn open(lanes: u32, home: mpsc::Sender<Wake>, read_memory: fn() -> Memory) -> DrivingPool;
    pub(crate) fn full(&self) -> bool;
    pub(crate) fn in_flight(&self) -> u32;
    /// 交出一次驾驶：起一条车道。run id 取自 `driving` 自己，不另传一份——
    /// 两处说同一件事就有两处说错的机会。
    pub(crate) fn start(&mut self, driving: Driving, ledger: Relay, context: DriveContext) -> Result<(), AxError>;
    /// 取回下一轮跑完的活；`wait` 内没有就返回 `None`，让调用者去服务 relay。
    pub(crate) fn arrived(&mut self, wait: Duration) -> Option<Result<Arrival, AxError>>;
}
```

**一条车道就是一条只活一轮活那么久的线程**，而不是常驻的 N 条。理由：一条常驻线程在没活时持有的东西是零，
而它要活着就得有一个入口通道与一次关闭协议；一轮活一条线程把「这条线程的一生就是这轮活的一生」写成事实，
`JoinHandle` 于是同时是「这轮活还在跑」的凭据。spawn 点仍在 `bin`（ARCH §10 规则 3）。

**线程 panic 不是一种情况**：发布档 `panic = "abort"`（ARCH §2）。车道把 `Result<Driven, AxError>` 送回来，
送不回来（`join` 报错）在测试档下也只是一次 `E_STORAGE_FATAL`，而不是一个 `Box<dyn Any>` 的分支。

**车道数 `DRIVING_LANES = 4`，写在本模块里，并且如实说明它不是 provider 天花板的第二个权威**：
`gateway::admission` 的 `ADMISSION_MAX_IN_FLIGHT` 是 `pub(crate)`，`bin` 读不到它。
两个数字今天相等是刻意的，而把 provider 的天花板变成一个可读的公开值会改动 `gateway` 的公开面、
需要重算 api-baseline，那是一次独立的卡，不该塞进这一张。**在它落地之前，比天花板大的车道数只会让线程停在
admission 上排队**——§8-42-3 早就写下这句话，这里把它从设计变成一个带理由的常量。

**内存紧时计划的下一行排队**：`full` 在车道都占满时为真，另外在已有 run 在跑、而整机可用内存低于物理内存的十分之一时也为真（`admits(in_flight, lanes, memory)` 是这一条规则的唯一出处）。读数来自 `open` 时交给池的 `read_memory`，池自己不碰主机：生产交 `bin::monitor::memory::read`，worker 搬进 `accounting` 时 `monitor` 留在 `sprawling`、经这个 `fn` 指针进来（accounting-SPEC.md §7、§12-10），脚本场景交一个自己的读数就能造出内存紧的机器。`full` 每次被问都读一次，所以跟着实时的可用内存走；问它的有三处：`DrivingPool::start`（每一轮进车道都经过的门，满了就排队）、`start_waiting`（一轮回家后按到达顺序起排着的活）与 `Flight::full`（计划推进循环 `accounting::worker::plans::pursuing` 每次决定是否起下一行）。读一次约 1.6 µs（Windows x86-64 桌面级机器、测试档构建），只发生在起一轮之前。没有 run 在跑时总放一轮进来：否则一台内存一直紧的机器上城永远不动，而一轮自己占的内存远小于它派出的构建。排着的计划行在下一轮回家时再判一次。取物理内存的十分之一而不是一个字节数，是因为一个字节数只适合某一类机器；十分之一留给人的其他程序与页缓存。**被否**：按「每轮估计占用」算出可同时驱动的轮数——一轮的边际内存还没有测过，估计值就是一个没有来源的常数。**重开参数**：测得一轮的边际内存之后，改成「可用内存 ≥ 留给别人的那一份 + 一轮的实测边际」。证据：`crates/accounting/src/worker/pool.rs` 的 `a_new_run_waits_while_memory_is_tight` 与 `a_pool_judges_memory_by_the_reader_it_was_handed`。

### 8-46-4 `pursue` 拿走整个 ready set（`accounting::worker::plans::pursuing`）

`pursue` 住 `plans` 下自己的模块（`plans.rs` 留 `set_pursuit` 与读计划的那几个私有方法；子模块看得见父模块的
私有项，所以没有字段因此变公开）。**它由事件推进，不是命令里的一个循环**：命令台只在主循环里读，
一个在 `Pursue` 命令里转到底的循环会把 `Pause`、`Clear`、`Halt`、新派活、审批与排程全都关在门外。

1. `set_pursuit` 记下 `pursuit_changed` 之后调一次 `pursue`：读整个 ready set，对其中每一个节点，只要车道没满就
   `stage_dispatch` 并交给池，然后**返回**，命令台随即空出来。
2. 车道满了就不再取：`kernel::observe(state, &ready, in_flight)` 的 `in_flight` 是真值，
   于是「没什么可取但有人在跑」如实答 `Waiting { in_flight }`。
   **已经在别人手上的节点不在问它的那个 ready set 里**：ready 的意思是「现在可以有人接手」，
   而一个已经被接手的节点不能被接手两次。过滤在调用点做，依据仍然是 `kernel::pursuit` 的。
3. 每一轮活落地之后，`serve_flight` 调 `advance_pursuits`：落地的是某个追求的一行（`Landed::Row`）而那个节点
   仍然 ready，说明这一轮什么也没认领——记一条 `Refuse` 诊断，并经 `set_pursuit` 把这个追求**暂停**，
   `pursuit_changed` 因此写下 `pause`，页面不再说「working on …」，人 `Resume` 即可再试；然后对每一个
   追求再取一次 ready work。任何一轮活落地都可能空出车道或让新节点变 ready，所以不只看自己的行。
4. 这一步取活失败（计划读不出、派活被拒）落一条 `Refuse` 诊断，不让刚落地的那一轮活的结局跟着失败：
   两件事没有因果。

**账本上的写者仍然只有一个**：车道线程手上唯一的 `kernel::Ledger` 是 `Relay`，`JsonlLedger` 一步不离记账线程。
`RelayGate` 由 `pursue` 自己开一扇，发出去的 `Relay` 与它一一对应；`assembly::attending` 主循环服务的那一扇仍在，
属于同一张 `Flight`（8-46-2），于是 desk 派的活与追求派的活在同一批车道里排队。

**评审楼一个房间一棵 worktree 是既有事实，只验证不重做**：`stand_up` 按房间地址给工作树起名，
三轮活分属三个房间时就是三棵树。同理，Handoff 住房间而不住楼，
三轮活分属三个房间时各写各的 `Handoff.md`；这也是只读不改的东西。

### 8-46-5 citysim：一条账本、两轮活、逐字节重放

池大小 1 是模拟器的确定性条件（§8-42-3），所以 citysim 不跑池。它要证的是另一半：
**两轮活的记录交错在同一条账本上时，同一批脚本重放出同样的字节**。
`run_scenario` 今天每次自己造一个 `MemLedger`，于是「两轮活一条账本」在 citysim 里根本拼不出来。
这里加一个入口：

```rust
pub fn run_scenario_on(ledger: &mut MemLedger, scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

`run_scenario` 变成「造一条账本，调它」——一个权威，两个调用方。新场景
`two_runs_interleaved_on_one_ledger_replay_byte_identically`：两个 run id、两个房间、各自的脚本模型，
跑在同一条 `MemLedger` 上，`seq` 单调、`prev` 成链（`check_chain`），再跑一次逐字节相同。

### 8-46-6 文档停止过度承诺

`README.md` 两处写着「多个 agent 同时工作」。此前这句话在产品里没有主语：`pursue` 一次只跑一轮活。
此后它成立，但只在一个位置成立，文档必须把那个位置说出来，而不是继续说一句听上去更大的话。
`ARCHITECTURE.md` §11 的性能记录增一行**并发墙**：一座城同时驾驶的轮数由车道数决定，
而车道之外的第一堵墙是 provider 的 admission 天花板；账本仍然是串行的，那是 §10 规则 5 的价钱。

### 8-46-7 验收

1. **红转绿**：`accounting::worker::plans::tests::goals::three_ready_nodes_drive_three_runs_at_once`——
   三个 ready 节点、一个 pursuit，假 provider 记下同时在飞的请求峰值。串行时峰值为 1，红就红在这里。
2. **红转绿**：`citysim::scenario::two_runs_interleaved_on_one_ledger_replay_byte_identically`。
3. **红转绿**：`accounting::worker::driving::tests::flight::two_dispatches_from_the_desk_drive_at_once`——
   两次派活各自进一条车道，主循环接它们回家；账本 `seq` 单调、`prev` 成链，
   每一轮活的 `run_started` 排在它自己的 `model_called` 之前，且第一轮冻结之前两轮都已开始。
   串行时红在最后那条：第二轮要等第一轮冻结之后才开始。
4. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings`、
   `cargo nextest run -p sprawling --locked --all-features`、`cargo nextest run -p citysim --locked --all-features` 绿。
5. `cargo xtask modmap`、`length`、`header` 绿。

### 8-46-8 车道数今天为什么是一个写死的常数

`DRIVING_LANES` 与 `gateway::admission` 的 provider 并发上限今天相等，而且是分开写的两个数。
让 `bin` 取 `min(天花板, 配置)` 要求 `gateway` 多一个公开的读法，那是它公开面的一次变更，
要连带重算 api-baseline。在那之前，比天花板大的车道数只会让线程停在 admission 上排队——
把队列从一个会算数的地方搬到一个不会算数的地方。

## 8-50 三种读法的答：回合、证据、一个节点的花费（`accounting::views::rounds`、`views::evidence`、`views::cost_of`）

`wire` 定形状（wire-SPEC §8-21），这里折出答。三个新模块都是形状 7 投影：把记录折成页要的读法，删掉重折逐字节相同。

### 8-50-1 `accounting::views::rounds`——一次会话折成回合

`records_of(run)` 用 `LedgerIndex::run_seqs_before` 取这次跑最新的 `HISTORY_MAX` 条 seq，逐条读行、逐条解析，旧在前。**上界与客户端原来问的那一段等宽**：`web::live::page` 一直是 `RunHistory { limit: HISTORY_MAX }`，所以搬到服务端之后一个会话能被读到的范围一字未变——搬家不该顺手改答案。读不回来的一行**截断而不清空**：读到的那些仍然是真的（同 `Views::history`）。

折叠本身逐字从 `web::turn::rounds` 搬来，一条不改：`model_called` 开一个回合；`tool_called` 挂进当前回合并按 runtime 给的 id 记下等答；`model_returned` 落到最近一个回合上；`tool_result` 按 id 找回它自己的那次调用——**恒不按位置配对**，因为两个调用可以先后发出而后发的先答；其余记录按 `wire::reading::note_of` 判是否成为一条 `Note`。`opened_at` 是本会话第一条 `Checkpointed` 的 oid：那是这份活开始时的树，取最新的一个检查点答的是另一个问题（「上一波动了什么」）。

**等人的终点从城自己的 run 里配回来**：`approval_resolved` 记在 `RunId::CITY` 下，不在这次会话的记录里，所以折完回合之后 `answer_waits` 按 approval id 把答复的 `t` 写回 `Note::Waiting.answered`。只有这次会话的窗口里有 `approval_requested` 时才读城的那一段（同样最新的 `HISTORY_MAX` 条）——没等过人的会话不多付一次读。请求或答复落在各自窗口外、或载荷读不回来时，`answered` 为 `None`，页面把请求之后整段画给人，而不是猜一个终点。备选是让治理折叠常驻一张 id→答复时刻的表：它随城一生里答过的每一件批准只增不减，而这里的读只在打开一个等过人的会话时才付。

### 8-50-2 `accounting::views::evidence`——写下来的证据，不携字节

同一份记录再走一遍，只认两种：`tool_result` 载荷 `result.image` 是 `cas:` 定位符的，成 `EvidenceKind::Screenshot`（连同 `width`／`height`／`media_type` 三项，缺一则 `picture` 为 `None` 而行仍在）；`roadmap_finished` 载荷 `evidence` 能读成定位符的，成 `EvidenceKind::Finished`。定位符读不回来的记录**不成行**——发明一条指不到任何东西的证据比少一行糟。

### 8-50-3 `accounting::views::cost_of`——节点到跑，跑到钱

`Views` 多一张 `claims: BTreeMap<NodeId, BTreeSet<RunId>>`，由 `roadmap_claimed` 折出（载荷 `node` 加记录自己的 `run`）。`BTreeMap` 而非 hash：这是答一个查询的路径，顺序必须是确定的。答时把这些跑在 `storage::attribution` 的 `by_run` 里各自的数取出来相加——**这里不计价**，计价是 `gateway::cost` 的，归因是 `storage::attribution` 的，本模块只做一次求和。

### 8-50-4 验收

- `views::rounds::tests` 与 `views::rounds::reading_tests` 把 `web::turn` 的两份测试逐字搬来，跑在服务端的折叠上：**服务端算出来的回合等于视图层对同一批记录算出来的**。红是在真账本上取的——`Query::Rounds` 先答 `Unavailable`，`asking_for_rounds_answers_the_fold_the_view_layer_ran` 在 `init_city` 铺出的城上写三条记录再问，失败于「Rounds answers with rounds」。
- `views::evidence::tests`：一张截图与一条完成证据各成一行，读不回定位符的载荷不成行。
- `views::cost_of::tests`：认领过的节点报出那次跑的钱；没人认领过的节点报 0 与空明细，而不是 `Unavailable`。

### 8-46-13 一个仓一次检查点：`drive_context.checkpoint_gate`

**一轮检查点是一个动作，不是一个 run 的一部分。** 一个 ready set 里的每个节点各占一条 lane 同时跑（§8-46-4），而它们都落在**同一个仓库**里：`Checkpoint::wave_pre` 会 stage 这个 run 的作用域并提交，libgit2 为此取 `.git/index.lock`。两条 lane 的检查点重叠时，后者拿到的是「the index is locked; this might be due to a concurrent or crashed process」——那句话是真的，而「并发的进程」就是这座城自己，锁只被持有几毫秒，而不论是拒绝语还是人看到的那句 recovery（「retry the wave」）都没有人替它重试。**代价不是一条日志**：输了这场竞争的 run 以 `cancelled` 冻结，它的节点被当作「自己的 done check 没过」交回，父 run 因此收到一个它无法据以行动的失败。

**所以这条规则是「一个仓一次检查点」，不是「一个 run 一次检查点」**：`RunWorker` 持一个 `checkpoint_gate`，每条 lane 的 DriveContext 拿到它的克隆，检查点闭包在调用 `wave_pre` 的那一小段里持锁。**宽度是关键**——锁的宽度是一次检查点（stage＋commit＋读回），不是一条 lane 的寿命：两条 lane 的模型调用、工具执行、账本写入全都照旧并行，只有那个动作排成一列。

**两道保险各管一个对手，理由写在各自的位置**：这里的闸门管**同一个进程里**的两条 lane；`storage::checkpoint::scan::write_index` 的等待管**另一个 sprawling 进程**压在同一座城上，那是任何互斥量都看不见的对手。把两者合成一个机制会让其中一侧假装看见了它看不见的东西。

### 8-46-9 一个房间一个队列：`accounting::worker::rooms`

形状：**深模块**（ARCH §9 第 1 种）。文件 `crates/accounting/src/worker/rooms.rs`。

`RunWorker.inboxes` 从前是 `BTreeMap<Address, Inbox>`，字段注释写着「一个房间恰好一个队列」，而**没有任何东西执行这句话**：
`open_desks` 以 `remove` ＋ `unwrap_or_else(new_inbox)` 借队列，`settle_desks` 以 `insert` 还队列。
在 `DRIVING_LANES = 4` 下，`pursue` 把同一栋楼的多行派到**同一个房间地址**，于是第二轮活借到一个空队列，
先落地那一份被后落地的整份覆盖——账本上写着 `signal_enqueued`，内存里那条信号不存在，而只增账本无法区分
「本来就没有」与「被覆盖了」。

```rust
/// 一个房间的队列，以及它在谁手上。
enum RoomQueue {
    Home(collab::Inbox),
    /// 借出期间送来的信号在同一格里等，持有者落地时一并交过去。
    Lent { to: RunId, waiting: Vec<collab::Signal> },
}

/// 借到的东西，与这轮活是不是持有者。
pub(in crate::assembly) enum QueueTenure { TheRoomQueue, ASpare { held_by: RunId } }
pub(in crate::assembly) struct Lent { inbox: collab::Inbox, tenure: QueueTenure }

pub(in crate::assembly) struct RoomQueues { rooms: BTreeMap<Address, RoomQueue> }
impl RoomQueues {
    fn folded(rooms: BTreeMap<Address, collab::Inbox>) -> RoomQueues;
    fn lend(&mut self, addr: &Address, to: RunId) -> Lent;
    fn give_back(&mut self, addr: &Address, from: RunId, returned: collab::Inbox) -> Result<(), AxError>;
    fn deliver(&mut self, signal: &collab::Signal) -> Result<(), AxError>;
    fn pending(&self, addr: &Address) -> u32;
}
```

- **借出不是取走**：`Lent` 仍在表里，第二个同房间的 run 得到一份自己的空队列（`QueueTenure::ASpare`）并被记一条
  `Refuse` 诊断。为什么不是三段式拒绝：`pursue` 的整个 ready set 都派在同一个地址上，拒绝会把并发追求
  （§8-46-4）整条打掉，而「一个房间一个读者」本身就是对的——两轮活分读一个队列，每轮只看到一半的信。
  **名字说的是持有关系而不是被持有之物**：城的书架那一件叫 `city::Holding`（图书馆的 holdings），
  这里说的是这轮活以何种名分拿着队列，故叫 `QueueTenure`。
- **只有持 `run_id` 的能还**：`give_back` 校验 `Lent.to`，不匹配即 `E_STORAGE_FATAL`。这条把「谁借的谁还」
  从纪律变成类型之外的运行时断言，而 `QueueTenure` 让调用点根本写不出「拿着 spare 去还」。
- **借出期间的投递有地方落**：`deliver` 对 `Lent` 推进 `waiting`，上限仍是 `INBOX_CAPACITY`，满了照旧
  `E_BACKPRESSURE_SHED`。**满溢的措辞只有一个家**：`landing.rs` 里手写的第二份 Shed 判定随之删除。
- **队列一定回家**：`give_back` 里等候的信号若被拒，队列先放回表再把错误抛出去——一个在回家路上失败的队列
  从前就是一个城忘掉的队列。

**B-28 与它是同一件事的另一半**。`land` 从前在 `driven?` 处提前返回，`settle_desks` 与 `conclude` 都不执行：
房间队列丢了，worktree 租约不归还——而这正是磁盘出问题时集中发生的那条路径。此后 `land` 先调
`return_borrowed`（还队列、还租约），再读 drive 自己的结果；`conclude` 不再释放租约，`settle_desks` 不再收队列，
两件事各剩一个家。

**backlog 席位与队列是同一批借来的东西**：`land` 从前先以 `?` 交还 backlog 席位，再调 `return_borrowed`，
于是一把中毒的 backlog 锁连房间的信一起吞掉。此后两次归还都先做完，再按顺序抛出第一个失败——
归还路径上没有提前返回。

### 8-46-10 服务态是一个值：`Serving`

`RunWorker` 从前有三个各自 `Some` 的 `Option`——`interrupts`／`watching`／`machine`——三个 setter
（`watch`／`examine`／`attach_interrupts`）由 `assembly::attending` 在同一口气里各调一次，文档各自写着
「None in every worker but the one behind a live control surface」。三个字段容许八种状态而只有两种可达，
而加第四个 sink 意味着记得加第四个 setter。

```rust
pub(crate) struct Serving {
    deltas: Arc<dyn Fn(wire::Delta) + Send + Sync>,
    machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    interrupts: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
}
impl RunWorker { pub(crate) fn serve(&mut self, serving: Serving); }
```

一个 `Option<Serving>`，一个注入点，漏一个即编译错。`assembly::attending` 的三次调用合为一次；
测试要只听中断时经 `fixture::only_interrupts` 明写它不听什么，而不是另开一扇门。

### 8-46-11 排程窗口逐条走完，撤销靠反向命令

`tick` 从前先把 `last_tick` 推到 `now` 再逐条派活，`?` 在第 k 条上返回时第 k+1..n 条到期作业**既没跑、没入账、
也不会回来**：一栋楼地址写错就能让同一分钟里其它所有定时活消失。此后每条到期作业都经 `start_unasked` 尝试，
起不来的记一条 `Refuse` 诊断并继续下一条，窗口只在整段走完之后才关。

**入账落在诊断日志而不是账本**：`schedule_dispatch_failed` 需要一个新的 `EventKind`，而 wire schema hash
随 `EventKind::ALL` 变动（wire-SPEC §8-1），本波 wire 冻结在 32。同一处境下 `answer_knocks` 早已用诊断行记
「敲不开的门」，此处复用同一机制而不是造第二种。**欠账**：`city::Schedule::due_after` 仍返回三元组，
不带 `fired_at`，所以窗口只能整段推进而不能逐条推进——补 `fired_at` 要改 `crates/city`，属另一张卡。

**撤销不是一个动词**：`attach` / `select_model` / `set_autonomy` 之后的一键回退，做法是**发同一条命令、带旧值**，
账本两条线都留着。后端这一侧因此只欠一条性质：设置类命令重复发送与发送一次等价。
`set_autonomy` 与 `select_model` 折叠取最后一条，故成立（`crates/sprawling/tests/undoing.rs`）。
**两处不成立，都需要它们各自的卡**：

1. `attach` 没有反向动词——`Command` 里没有 detach，`EndpointLost` 只由 `E_ENDPOINT_DIALECT_UNSUPPORTED` 的
   carrier 产生，没有命令能写它。撤销 attach 要么加 `Command::DetachEndpoint`（wire 变更），要么这一格不做。
2. `select_model` 的反向命令要求「上一次选择」存在且那个端点仍然挂着；第一次选择之前没有可回退的值。

### 8-46-12 两条接力链各有上限

**一条没有人在里面的链不许无限长。** H-04 把七个入口都送进车道之后，敲门链与继任链不再堵住
记账线程，`Halt` 与 `Cancel` 也读得进来了；但两条链本身仍然无界——A 叫醒 B，B 再叫醒 C，
或者一轮活连着把自己交给下一轮，每一步都花一次真跑，而没有人答应过这场开销。§8-13 早先
记的口径是「不设叫醒预算，什么时候停下是对话里那几位居民的事」；那条判断成立的前提是
链条在记账线程上跑，它的唯一刹车就在被它堵住的线程上。刹车够得着之后，剩下的问题是
**没有人在场时谁来停**，而一个由居民自己决定长度的环不会自己停。

```rust
/// 一条链把同一件活带到了哪里。两个计数器分开，因为两条链回答不同的问题：
/// 这场对话敲醒过几个人，这件活换过几次人。
#[derive(Clone, Default)]
struct Relays { chain: KnockChain, successions: u32 }

/// 一轮活在叫醒它的那场对话里站在哪儿：`hops` 是本分支的深度，只有本分支推它；
/// `woken` 是整场对话一共叫醒了几轮 run，所有分支共用同一个计数。
#[derive(Clone, Default)]
struct KnockChain { hops: u32, woken: Arc<AtomicU32> }

/// 一次敲门链最多唤醒这么多轮 run；一次继任链最多接力这么多轮；
/// 一场对话的所有分支加起来最多唤醒这么多轮。
const CONVERSATION_HOPS_MAX: u32 = 16;
const SUCCESSION_HOPS_MAX: u32 = 64;
const CONVERSATION_RUNS_MAX: u32 = 64;

impl Owing {
    /// 由敲门起的那轮活欠什么：对话向前一跳，超上限即 `E_LOOP_SUSPECTED`。
    fn knocked(chain: KnockChain) -> Result<Owing, AxError>;
    /// 继任者接过的同一份义务：接力向前一跳，超上限即 `E_LOOP_SUSPECTED`。
    fn after_succession(&self) -> Result<Owing, AxError>;
}
struct Knock { addr: Address, from: String, mode: kernel::Mode, chain: KnockChain }
fn knock(&mut self, signal: &Signal, speaker: &Address, mode, chain: &KnockChain) -> Result<(), AxError>;
```

- **接力次数记进义务，不记进工人**。两个计数器随 `Owing` 走：继任者拿走的是前任的义务，
  故它自然继承并加一；敲门推的是 `Knock { chain }`，`answer_knocks` 由此造出的新
  一轮活从上一跳加一。委派的子活继承原值而不加——换的是干活的人，不是这条链的位置。
  计数器不记在 `RunWorker` 上：那正是 C15 删掉的形状（一个 worker 字段描述的是碰巧在
  落地的哪一轮活），落地完成后字段属于谁没有答案。
- **一个上限，不是一份预算**。上限管的是「门敲下去会不会没完没了」，不给一轮活定价、
  不改居民之间能谈多少轮：`16` 是「十六个居民被连着叫醒之后这已经不是一场对话，是一个环」
  的位置，`64` 是「同一件活换过六十四个住户之后不管人在不在看都该停下」的位置。两个数字
  都是刹车而不是调过的参数，故都在明处，重开一次对话或再派一次活即可继续。
- **深度之外还有宽度**。只限深度时，一轮活给 N 位居民发信、每位再给 N 位发信，十六跳之内
  叫醒的 run 按 N 的幂增长；被叫醒的 run 又是 `Root` 深度，可以再委派，「一层深」管不住
  一条链的总量。所以 `KnockChain.woken` 由整场对话的所有分支共用（`Arc<AtomicU32>`，
  随最后一个持有者一起释放，不在 `RunWorker` 上留一张永不清空的表），`after_knock` 先判
  `hops`、再原子地把 `woken` 加一并判 `CONVERSATION_RUNS_MAX`，两处超限都是同一个
  `E_LOOP_SUSPECTED` 与同一句恢复语。委派的子活共用父的 `KnockChain`，所以子活发的信
  也记在同一场对话的账上。`64` 与继任上限同值，同样是刹车：一场对话叫醒了六十四轮之后，
  它已经是一次广播而不是一场对话。
- **敲门超限不连坐发件人**。超限在 `answer_knocks` 里判：这一敲不开始新一轮活，落一条
  `Refuse` 诊断（带地址与拒绝的 subject）后继续下一敲；信已经在房间里，人仍可从 Inbox
  读它。继任超限在 `conclude` 里判：诊断落 `Refuse`，拒绝随 `hand_back` 回给要这轮活的
  那一位（`Asked` 到人，`Child` 到父房间，无人时成诊断行），**本轮活照常 `discharge`**
  ——链断在第一端，不能把已经跑完的那一轮的结局一起吞掉。
- **敲门先看房间里有没有人**。`answer_knocks` 在派一敲之前问 `RoomQueues::worked_by`：房间的队列
  借出去了，就说明有一轮活正在那里读，此时再派一轮只会拿到一个空的备用信箱（`QueueTenure::ASpare`），
  而信在 `Lent.waiting` 里要等持有者落地才回家。所以这一敲不派，存进 `Doorstep.deferred`（每个房间
  一敲），持有者 `give_back` 之后由 `return_borrowed` 放回 `knocks`，本轮落地末尾的 `answer_knocks`
  再派它——这时信已经在房间队列里，新一轮活读得到。判定只在派的那一刻做一次，而不是在 `knock` 入队时，
  因为从入队到派出之间 `conclude` 可能已经把别的活派进同一个房间。
- **handback 与普通的信走同一个敲门判定**。`discharge` 的 `Child` 分支投完 handback、派完图里新就绪的
  节点之后，调用同一个 `knock`：说话者是子房间，模式是子活的模式，对话计数是子活继承来的那一个
  （敲醒时照常加一）。父房间的图还有节点在外面时不敲——那几个节点会各自回来，最后一个回来、图被丢下时
  才敲一次，免得每回来一个节点就花一轮父的真跑。父房间没有 `URBANITE.md` 时照旧不敲，结果在它的信箱里等人。

**本章测试**：`accounting::worker::waking::tests` 里 `a_knock_at_a_room_somebody_is_working_in_waits_for_them_to_leave`
（房间借出时一敲不开 run，归还之后同一敲开 run）与 `what_comes_back_wakes_the_resident_who_asked_for_it`
（子活落地后父房间的居民被敲醒，brief 写明是子房间说的话）；`owing::tests` 里两个边界（上限减一放行、到达上限拒绝，拒绝码
`E_LOOP_SUSPECTED` 且恢复语非空）与 `a_conversation_that_fans_out_stops_at_its_width`（同一场对话
六十四次敲门放行、第六十五次拒绝）；`accounting::worker::waking::tests` 里一条 `hops` 为 `u32::MAX`
的敲门不开始任何 run（对照：既有的 `a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke`
证明上限之下一敲照常开跑）。

## 8-47 六处探测收成一处：doctor 是「运行中的机器有什么」的唯一权威（`bin::doctor::host`、`bin::doctor::presence`）

**原因**：运行中的机器被问了六次，每次一套读法——`SPRAWLING_PYTHON_WASM` 在 `accounting::worker::mcp` 与 `doctor::table` 各拼一次（§8-40 记下的债）；`host_shell()` 与 `execution_engine()` 住 `accounting::worker::workbench::engine`；Firefox 与 `chromedriver` 由 `bin::browser_bidi::lazy` 按名字盲起（Windows 上 Firefox 不在 PATH，于是 doctor 说「有」而浏览器工具说「没有」）；`ffmpeg` 在 `crates/desktop/` 里按名字起。六个答案各自漂，一个人看到的「缺什么」与 run 撞上的「缺什么」不是同一份。

**主机事实住机器层**（沿 kernel-SPEC §8-22「主机事实不入城」）。doctor 装的东西落 `~/.sprawling/components/<item>/`，**恒不落进任何一座城**——一座城搬到另一台机器时不该带着运行中的机器的组件。城里 `CONFIG.toml` 仍只说能力位（`sandbox.shell`）与限额，不说路径。

```rust
// lib.rs：doctor 从二进制半边搬进 lib，装配层与浏览器层才够得着它
pub mod doctor;                                   // 公开面唯一新增：`pub use screen::verb`

// bin::doctor::presence（形状 2 value）：运行中的机器对一项东西的回答，三态而非两态
pub(crate) enum Presence {
    Present { at: PathBuf, version: Version },     // 在，且起得来
    Broken  { at: PathBuf, fault: Fault },         // 在，但用不了——对判定等于缺，对人必须说出为什么
    Absent(Absence),                               // 不在，且说出是哪一种不在
}
pub(crate) enum Version { Said(String), Silent, Unreadable, Late }
pub(crate) enum Fault   { WillNotStart(String), HalfWritten, Unreadable(String) }
pub(crate) enum Absence { NotOnSearchPath, VariableNamesNothing { variable, path },
                          NoComponent { dir }, NoHome, NotInThisBuild }
impl Presence { fn usable(&self) -> bool; fn at(&self) -> Option<&Path>; fn describe(&self) -> String; }

// bin::doctor（Detection 增三臂；表因此能说出四种「怎么找」）
pub(crate) enum Detection {
    Program     { program, version_arg, places },
    Component   { variable, file },                // 变量优先；否则 ~/.sprawling/components/<name>/<file>
    Interpreter { variable: PerPlatform<&str>, fallback: PerPlatform<&str> },   // COMSPEC／SHELL
    Built       { carried: bool },                 // 这份构建带不带（sandbox 引擎）
}

// bin::doctor::host（形状 4 adapter）：二进制里其他模块问运行中的机器的那一扇门
pub(crate) fn firefox() -> Presence;        // Gecko 族里运行中的机器有的那个牌子（§8-57）
pub(crate) fn chromedriver() -> Presence;   // chromedriver 或 msedgedriver，先答上来的那个
pub(crate) fn python_wasm() -> Presence;
pub(crate) fn shell() -> Presence;
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError>;   // 从 workbench::engine 搬来
pub(crate) const ENGINE_CARRIED: bool;                                             // cfg!(feature = "sandbox") 的唯一拼写
pub(crate) fn components_dir() -> Option<PathBuf>;                                 // ~/.sprawling/components

// accounting::worker::workbench::engine（仍是 adapter，但不再自己探测）
pub(super) struct MachineHalf { python_wasm: Option<PathBuf>, shell: Option<PathBuf>, engine: Box<dyn Sandbox> }
pub(super) fn machine_half(limits: &kernel::SandboxLimits) -> Result<MachineHalf, AxError>;

// bin::browser_bidi::engine（decision）：吃 doctor 的三态答案，拒绝时说出是哪一种
pub(crate) fn Engine::choose(firefox: &Presence, chromedriver: &Presence) -> Result<Engine, AxError>;
```

- **谁问谁**：`workbench::engine::machine_half` 问 `doctor::host` 三次（组件、shell、引擎），它自己只留一条判定——shell 只在冻结配置说 `shell = true` 时才递给 bench，组件缺席不拦派活（python 臂在被调用时才拒），引擎起不来则拒派活。`browser_bidi::lazy::start` 不再按名字盲起，改为 `Engine::choose(&host::firefox(), &host::chromedriver())` 后按路径起。`accounting::worker::mcp::PYTHON_WASM_ENV` 删除。
- **`SPRAWLING_PYTHON_WASM` 只拼一次，且保留为兼容读法**：拼写唯一处是 `doctor::table::PYTHON_WASM_VARIABLE`。**选的是「变量优先、组件目录次之」**：一个人显式指了一处，就该用那一处；指错了（变量设了但文件不在）报 `Absent(VariableNamesNothing)` 而**不悄悄落到组件目录**——被否决的备选是「目录优先、变量兜底」，它会让一个设错的变量永远没人发现。没设变量时看 `~/.sprawling/components/python-wasi/python.wasm`。测试遍历本 crate 的 `src/`，断言含该字面量的文件恰好一个。
- **`Broken` 是第三态，不是 `Absent` 的别名**：一个在 PATH 上却起不来的二进制（权限、坏文件、架构不符）、一个存在却没有那份文件的组件目录（下载中断）、一个读不了的目录（权限），三者对 verdict 都算缺，但每一个都带着自己的原因进报告行——**绝不以「absent」一词吞掉一个可以说清的故障**。`Version` 的四态同理：说了、没说、说的不是文本、超时没说；后三者仍算 Present（§8-40 已定：不说话的工具仍是装了的工具）。
- **本二进制起的每个子进程都由 `doctor::running::stop` 结束**：`ask_version` 读到第一行后杀掉子进程，用的是安装程序超时后走的同一段——杀不掉或收不了尸都不是可以丢掉的 `Result`，而是一句带进 `Fault::Unreadable` 的话，于是「本城起了一个它停不掉的进程」这件事排在它印出的版本号之前给人看。`Fault::Unreadable` 因此是「这台电脑不让本城把这一项做完」的那一态，它携带的那句话就是全部解释，`describe` 原样印出。
- **`Detection::Built` 的探测是真起一次引擎**，而不是读一个 cfg：一份声称带引擎却起不来的构建，doctor 必须报 `Broken { WillNotStart }`；`ENGINE_CARRIED` 是那个 cfg 的唯一拼写，表引用它。
- **`ffmpeg` 进表但 doctor 管不到 `crates/desktop/`**：`crates/desktop/` 跑在 `sprawling desktop` 这个子进程里，它在录制时按名字起 `ffmpeg`，与 doctor 的 `on_search_path` 走同一条 PATH，两个答案因此一致而非因此合一。doctor 报它（Optional，Use 层），`crates/desktop/` 不改——这是这里的边界，如实记。桌面 server 本身不进表：它是本二进制的一个动词（§8-4d），没有要装的东西。
- **`browser::profile` 没有探测可搬**：读 `crates/browser/Spec.lean` 的 D4 确认 profile 是「楼的登录态住城的保留区」这条纯判定，浏览器探测住 `bin::browser_bidi::lazy`，故不改 `crates/browser`。
- **doctor 进 lib 的公开面只多一行**：`pub use screen::verb`，二进制半边 `main/router.rs` 改调 `sprawling::doctor::verb`；`Machine` 仍是 `pub(crate) trait`，不上缝清单。`tools/xtask/api-baselines/sprawling.txt` 随之重算。

**探测可失败的路径，逐条**（本节与 §8-48 共用，测试点名「丑的那几条」）：

| 路径 | 答案 | 报告行 |
|---|---|---|
| 程序在 PATH 上却起不来 | `Broken { WillNotStart(err) }` | `firefox  broken at <path>: will not start: <err>` |
| 起来了但一行也不说／说的不是文本／超时 | `Present { version: Silent／Unreadable／Late }` | `present <path> (said nothing／unreadable version／no version within the deadline)` |
| 变量设了却指向不存在的文件 | `Absent(VariableNamesNothing)` | `absent: SPRAWLING_PYTHON_WASM names <path>, which is not there` |
| 组件目录在、文件不在（半写） | `Broken { HalfWritten }` | `broken at <dir>: half-written; delete it and install again` |
| 组件目录或文件读不了 | `Broken { Unreadable(err) }` | `broken at <dir>: <err>` |
| 找不到 home | `Absent(NoHome)` | `absent: neither USERPROFILE nor HOME is set` |
| 构建带引擎却起不来 | `Broken { WillNotStart }` | `sandbox-engine  broken at <exe>: will not start: <err>` |
| `--install` 的网络超时 | 包管理器自己的退出码 → `E_TOOL_UNAVAILABLE`，recovery「run the printed line yourself」 | doctor 自己不上网、不重试、不静默 |

**本章测试**：`the_python_variable_is_spelled_once`（红：今天两处）；`a_program_that_will_not_start_is_broken_not_absent`（临时目录里放一个不是可执行文件的 `broken.exe`／`broken`）；`a_variable_that_names_nothing_is_said_so`；`a_half_written_component_directory_is_broken`；`a_present_tool_that_says_nothing_is_still_present`；`a_broken_firefox_is_refused_by_its_fault_not_as_absent`（`Engine::choose`）。

**本章验收**：`cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings`、`cargo nextest run -p sprawling --locked --all-features` 绿；`cargo xtask modmap`／`length`／`header`／`specalign` 绿；`cargo run -p sprawling -- doctor` 仍报 Firefox 那一行。

## 8-48 doctor 按城回答，按错误码解释（`bin::doctor::needs`、`bin::doctor::visit`、`bin::doctor::explain`）

**原因**：§8-40 的 doctor 回答的是「运行中的机器对这个仓库」，而一个人真正的问题是「我这座城跑得起来吗」：一栋写了 `browser = true` 的楼在没有 Firefox 的机器上，今天要等到 run 撞上 `E_BROWSER_UNAVAILABLE` 才知道。而那条拒绝的 `recovery` 是一句通用话，没有接到运行中的机器的事实上。

```rust
// bin::doctor::needs（形状 1 decision）：一栋楼的能力位要什么，运行中的机器给不给
pub(crate) struct Bits { pub browser: bool, pub shell: bool }
pub(crate) enum Capability { Browser, Shell }
impl Capability { pub(crate) fn any_of(self) -> &'static [&'static str]; pub(crate) fn as_str(self) -> &'static str; }
pub(crate) struct Lack { building: Address, capability: Capability, tried: Vec<(&'static str, Presence)> }
pub(crate) fn lacks(building: &Address, bits: &Bits, findings: &[Finding]) -> Vec<Lack>;
pub(crate) fn lack_line(lack: &Lack) -> String;

// bin::doctor::visit（形状 4 adapter）：走一遍城里的楼，读每一栋的位
pub(crate) enum Visited { Bits { building: Address, bits: Bits }, Unreadable { building: Address, err: AxError } }
pub(crate) fn visit(city_root: &Path) -> Result<Vec<Visited>, AxError>;

// bin::doctor::explain（形状 1 decision）：一个错误码接到运行中的机器
pub(crate) enum Explanation { NoSuchCode(String), NotAboutThisMachine(AxCode), Lines(Vec<String>) }
pub(crate) fn explain(code: &str, findings: &[Finding], platform: Option<Platform>) -> Explanation;

// bin::doctor::screen：`doctor [<city>] [--install] [--explain <code>]`
pub(crate) struct Asked { install: bool, city: Option<PathBuf>, explain: Option<String> }
```

- **能力位 → 项目，是一张穷尽表**：`Browser → [gecko, chromedriver, msedgedriver, webkit]`（任一即可，Gecko 在前，因为它不要驱动；见 §8-57）；`Shell → [shell]`。`browser` 读自 `RULES.toml`（`city::load`），`shell` 读自该楼冻结配置的 `sandbox.shell`（`city::load_config`）——两者合称「RULES.toml 的能力位」，实际住两份文件，这里如实记。`desktop` 不是能力位：它要的东西都在本二进制里（§8-4d）。一栋楼的一个位缺时，报告行点名**那栋楼**与它试过的每一项及各自的三态答案：`lab: browser: true, and this machine has no firefox (not on the search path) and no chromedriver (not on the search path)`。
- **读不了的楼是一行，不是沉默**：`RULES.toml` 解析失败或 `CONFIG.toml` 无效，那一栋报 `Visited::Unreadable`，屏幕上是 `lab: its rules will not read: <err>`；楼列表本身读不到（不是城）才是 `Err`。**doctor 永不静默**。
- **`--explain <code>` 是「错误码 → 主机项目」的一张表**：`E_TOOL_UNAVAILABLE → [sandbox-engine, python-wasi, shell, ffmpeg]`，`E_BROWSER_UNAVAILABLE → [gecko, chromium, chromedriver, msedgedriver, webkit]`。其它已知码答 `NotAboutThisMachine`（它由城里的判定决定，不由运行中的机器决定）；不认识的码答 `NoSuchCode`。每一行是**那一项的三态答案加这平台上的下一步**：`python-wasi  absent: no component at ~/.sprawling/components/python-wasi/python.wasm -> manual: put a CPython wasi build there`——一个人读完那一行就能动手。
- **边界**：doctor 不从源码构建、不 vendor、不静默。它探测一切，只安装有官方可验证来源的东西，并逐项先问。CPython-WASI 今天没有 python.org 发布的二进制，故它仍是 `Manual`，指向组件目录；这里不下载任何东西。`~/.sprawling/components/` 因此暂时只是 doctor 探测、人填入的约定——记在这里，免得下一步以为那里有个下载器。
- **退出码**：`doctor <city>` 在该城任一楼缺任一位时退 1，与 §8-40 的「必需项有缺退 1」同一口径；`--explain` 退 0（它是解释，不是判定），只有码本身不存在时退 1——一个拼错的码是一次问错，脚本该知道。

**本章测试**：`a_building_that_asks_for_a_browser_is_named_when_this_machine_has_none`（红：`lacks` 不存在）；`a_building_whose_rules_will_not_read_is_reported_not_skipped`（`visit` 在真目录上）；`explain_connects_a_refusal_code_to_what_this_machine_has`（`E_TOOL_UNAVAILABLE` 给出一行每项、含 recipe；未知码与无关码各自的答案）；`doctor_with_a_city_names_the_building_on_the_screen`（`run` 经 `ScriptedMachine`）。

**本章验收**：同 §8-47；另加 `cargo run -p sprawling -- doctor --explain E_TOOL_UNAVAILABLE` 一行一项。

## 8-49 `Reviewed-by:` 说的是真话：这次合并没有人看过（`accounting::worker::reviewing`）

合并落成一个真正的合并提交，于是 `settle_requests` 给 `storage::Landing` 传的是写死的 `reviewed_by_person: true`。**那是一句写进永久历史的假话**：`PrEffect::Merged` 里的 `by` 是 `PrDesk::who`，也就是跑这次检查的那个 resident 的地址；评审楼的全部意思正是「另一个 resident 检查它」，而不是「一个人看过它」。运行中的机器的 git config 里若有 `user.name`／`user.email`，那句写死的 `true` 就会把仓库主人的名字挂到一份他从未读过的改动上。

**改为 `false`，并写下它为什么恒为假**：今天这条路径上不存在人的复核——`pr` 工具由模型调用，`who` 恒是城里的一个地址。`storage::Landing` 的这个字段不因此作废：它的两个取值在 `storage` 那侧各有一条断言（`a_person_who_looked_is_named_from_the_repositorys_own_config` 判真，`a_merge_lands_as_a_two_parent_commit_carrying_the_merging_runs_trailers` 判假），这里在 `bin` 这侧加第三条，从城外读回 trunk 的提交消息作证。**翻案条件**：当合并这一步真的经过一个人（例如合并成为一件需要 Approval 的事，由 `kernel::Answerer::Human` 答复），这里的取值就由那次答复得出，而不是再写一个字面量。

## 8-50 一次回合一个内容库句柄（`accounting::worker::credentials::endpoints`）

`redemption()` 造的取图闭包里写着 `storage::Cas::open(&cas_dir)`：**每张图开一次库**。一次带四张图的回合就开四次，每次都要建目录、探路径；而内容库是按内容寻址的只读读取，一个句柄答得了整场对话。

**改法**：`redemption` 在造闭包之前开一次库，把它放进 `Arc<Mutex<storage::Cas>>` 让闭包捕获。`Mutex` 不是为了并发而是为了类型：`ImageResolver` 要求 `Send + Sync`，而 `storage::Cas` 的 `Vfs` 缝只承诺 `Send`——`Mutex<Cas>` 在 `Cas: Send` 时即是 `Sync`，这比把 `Vfs: Send + Sync` 拓宽给所有适配器要窄。

**签名随之带上失败**：`redemption(&self) -> Result<gateway::Redemption, AxError>`，因为开库会失败，而失败的时刻从「第一张图到达时」提前到「装配适配器时」——这正是想要的：一个读不了自己内容库的城，应当在造适配器时说出来，而不是在模型已经开口之后。两个调用点各改一处（`dispatching::agreeing` 用 `?`，`dispatching::session::name_the_work` 在 Option 语境里用 `.ok()?`）。

**本节的断言把 `credentials/tests.rs` 顶过 400 行，故它按责任一分为二**（`length` 门报的红，修的是因而不是门）：`credentials/tests/endpoints.rs`（一座城够得着哪些模型，以及适配器在线上兑现什么）与 `credentials/tests/signing.rs`（凭证怎么进城：录入），`tests.rs` 只剩两行 `mod`。切分对着源文件的两半（`endpoints.rs`／`signing.rs`）而不是对着行数切。

**未由机器作证的那一半，写在明处**：「只开一次」本身没有断言，因为 `storage::Cas` 不数自己被开过几次；加一个计数缝只为这一条断言，代价大于它买到的东西。作证的是行为面——一次 `redemption` 解得开对话里的每一张图。

## 8-51 城的创世哈希一座城读一次（`bin::assembly` 与 `accounting::worker::workbench::standing`）

`storage::Provenance::city_of` 读的是账本首段的第一行，而 `provenance()` 每造一次署名就读一次：一次波里的每个检查点、每次落地、每次合并各读一次盘。**这个事实在一座城的一生里恒定不变**——创世行写下就不再改，改了那也不是同一座城。

**记在 `RunWorker` 上，用 `OnceLock` 惰性读一次**：不在 `over()` 里急读，因为一个刚被造出来、账本还空着的 worker 是合法状态（`RunWorker::new` 在一座尚未 init 的城上就是这样被测试用的），急读会把「还没有创世行」变成造不出 worker。

**缓存一份副本在这里不构成第二权威**：第二权威的危险来自**会变的**事实被抄了一份；创世哈希不会变。真正被消掉的风险是相反的一个——每次重读都可能读出不同的答案（有人换了账本），而一次运行里换了城的身份是比陈旧副本坏得多的事。

- **接口随之改形**：`workbench::standing::provenance` 与 `Site::provenance` 收 `city: B3Hash` 而不再收 `&Path`，于是「谁去读盘」这件事只剩 `RunWorker::city_hash` 一个答案，四个调用点都从它取。
- **作证方式**：城的创世哈希被读过一次之后，把账本首段从盘上删掉，`city_hash` 仍答同一个值——记住了才可能如此。

## 8-52 新客户端要问的四件事，服务端怎么答（`accounting::views::listing`、`accounting::views::document`、`views::rounds`、`views::holding`；wire-SPEC §8-23）

- **`Views.governance: views::Governance`**（原 `Views.halted`／`Views.approvals`／`Views.autonomy` 三个散字段）。停摆作用域、待答项与自治档由 `city_halted`／`approval_requested`／`approval_resolved`／`autonomy_changed` 折出，**折法只有 `Governance::absorb` 一处**：工作线程持一份判定面，`Views` 持一份读面，同一个类型、同一遍折，重建即相等（`views::tests`、`what_a_worker_holds_is_what_a_restart_rebuilds`）。此前 `Views` 把这四条记录另拼了一遍，两份拼法对「读不懂的决定算什么」答得不一样。`CityAnswer.halted` 是 `governance.halted` 的 `Vec` 形。
- **`summarize` 把 `RunHot.addr`／`started`／`completion`／`pr`／`ask` 抄进 `RunSummary`**，不读账本（wire-SPEC §8-48）。
- **`rounds_answer` 多读两条记录**：窗口里第一条 `run_started` 成 `Opening { task, goal, at: record.t() }`，第一条 `run_frozen` 成 `Closing { completion, at }`；`turns()` 本身一字不动。
- **`accounting::views::listing`**（新文件）：`at` 为 `None` 读城根，否则读 `city_root/<at>`；`read_dir` 一层，目录在前、文件在后、各按名字 UTF-8 序；读不了的目录答空表而不是拒绝——同 `read_building` 的口径，一个读不了的目录在页面上是一个空目录。符号链接按 `metadata` 判：指向目录的算目录。文件大小 `u64`。
- **`accounting::views::document`**：路径同上；答什么、怎样判文本、在哪里切，见 accounting-SPEC §8-21 与 wire-SPEC §8-69。**不经密钥扫描**：这是城内的文件给城的主人看，而 `Hunks` 的扫描针对的是把补丁文本挂上线的那条路——但 `.sprawling/CONFIG.toml` 里只有 `secret:` 引用，明文本来就不落盘（`xtask secret` 门保证），所以这里没有可泄露的东西。
- **验收**：`views::listing::tests`——`init_city` 铺出的城根列出 `.sprawling` 与 `hall` 两个目录；`hall` 下列出 `Roadmap.md` 等文件且目录先于文件；不存在的路径答空表。`views::document::tests`——读城里 `hall` 楼自己的 `RULES.toml` 得到原文，第一个窗口盖住整份；其余见 accounting-SPEC §8-21。`views::rounds::tests`——三条记录的会话答出 `opening.task`；冻结后答出 `closing.completion == "done"`。`views::tests`——`city_halted` 后 `city_view.halted == ["city"]`，`released` 后为空。

## 8-53 一座楼做过的提交，倒序分页（`accounting::views::commits`、`views::holding`；wire-SPEC §8-24）

- **`Views.commit_seqs: BTreeMap<Seq, GitOid>`**，与按 oid 键的 `commits` 表由 `fold_commit` 同一处写入：一条记录若宣告了提交，两张表各得一行。按 oid 的表答 `Commit`，按 seq 的表答 `Commits`；两表从同一条流折出，重建即相等。
- **`commits_answer(building, before, limit)`**：在 `commit_seqs` 上从 `before` 的独占上界（`None` 即尾）向前走，按 `actor` 地址前缀过滤（`actor == building` 或以 `<building>/` 起头；`None` 不过滤），取 `limit.clamp(1, HISTORY_MAX)` 条；再多走一步得 `more`。每条经 `CommitFacts::answer` 与 `lineage_of` 成 `CommitAnswer`，故列举与反查答同一形状。
- **验收**（`views::commits::tests`）：三条不同 seq 的 `checkpoint_committed`（两条在 `lab/room1`、一条在 `hall/mayor`）折入后，按 `lab` 列举答两条且 seq 递减、`more == false`；`limit: 1` 答一条且 `more == true`；以那条的 seq 作 `before` 再问答下一条；按 `hall` 列举不含 `lab` 的提交；`None` 答三条。

## 8-128 一次提交带出同一次 run 的上一个提交，与它的父提交（`accounting::views::commits`；wire-SPEC §8-54）

- **`CommitFacts.previous: Option<wire::CommitAt>`**，`fold_commit` 写入：`Views.last_commit: BTreeMap<RunId, CommitAt>` 记每次 run 最近宣告的那个提交，一条记录宣告提交时，先把这张表里同一 run 的那一项取作 `previous`，再换成本提交。同一个 oid 被宣告第二次时不拿自己当 `previous`，沿用第一次折出的那个。折叠从快照起步也一样：`last_commit` 随视图进快照，`VIEWS_FOLD_RULES` 随编码变。
- **`parents` 不进视图。** `Query::Commit` 与 `Query::Commits` 在锁内由视图答出其余各字段，交给 `Prepared::Commits(CommitsAsk)`；锁放开之后 `CommitsAsk::read` 经 `storage::changes::parents_of` 一次开仓库读这一页每个 oid 的父提交。读不到仓库时 `parents` 全为 `None`，答复照样发出：父提交是这一行多出的一格，不是它成立的条件。8-41 说的「没有一个字段是从 git 读的」指 trailer 能答的那几项；父提交 trailer 答不出，它的权威是提交对象本身。
- **验收**（`views::commits::tests`）：同一 run 的两条检查点之间夹着另一 run 的一条，第二条的 `previous` 指向第一条（oid 与 seq），另一 run 的那条与第一条的 `previous` 为 `None`；一个真仓库里的两个提交经 `CommitsAsk::read` 答出后一个的父提交是前一个，前一个是根提交（`Some(vec![])`），不在仓库里的 oid 答 `None`。

## 8-54 运行中的机器有什么，页面从城那里问（`bin::doctor::report`、`accounting::views::holding`；wire-SPEC §8-25）

首跑屏的第一步原本只是一条可以复制的命令，没有任何办法知道它跑过没有、跑成了没有。本节让那一步答得出来。

- **`bin::doctor::report`**：`answer(machine: &dyn Machine) -> wire::DoctorAnswer` 问一次交给它的机器并折成答案。机器是参数，所以一个测试交一台假机器，就说出机器答了什么而不必有那样一台机器；生产的调用方是 `ThisMachine` 实现的 `accounting::Machine::report`。**它不判断任何事**——哪一项在这里、一个档次缺什么，权威在 `doctor` 与 `table`；这里只换一种说法。`screen` 把同一批 findings 折成一台机器的散文，两者从同一处折出。
- **`Views.machine: Option<wire::DoctorAnswer>`**，由 `found_on_this_machine` 从外面放进来，**不由任何记录折出**：这是本文件里唯一一个关于机器而非关于历史的答案，所以重建账本不碰它。`None` 答 `Unavailable`。
- **`Views.registry: Option<fn() -> wire::ReleaseAnswer>`**，由 `views::served` 的 `ask_the_registry_through` 从外面放进来，与 `machine`、`vault` 同形：serve 一座城时装配根交 `bin::release::answer`，`twin` 把它带到另一份。`NewestRelease` 在快照放开之后调它（它要出网）；`None` 是一份没人 serve 的 views（重建、测试），答 `Unavailable` 而不去问注册表。views 因此不直接碰 `bin::release`，搬进 `accounting` 时 `release` 留在 `sprawling`（accounting-SPEC.md §7）。钉住它的测试是 `a_newest_release_is_asked_of_the_registry_the_views_were_handed`。
- **`Views.programs: Option<fn(&str) -> Option<PathBuf>>`**，由 `find_programs_through` 从外面放进来，与 `registry` 同形：serve 一座城时装配根交 `bin::doctor::host::find_program`。`Query::Harnesses` 在快照放开之后调它；`None` 答 `Unavailable`（accounting-SPEC.md 8-10、§12-13）。
- **探测只由 `DoctorRefresh` 触发，服务一座城时一次也不跑**：表从 12 行长到 32 行，其中大半是起一个进程问它的版本（六件 cargo 子命令各起一次 cargo），windows-x86_64 暖缓存四核一档机器上量得 3.3–4.1 s。先前的决定把它放在开门之前，给出的参数是「12 项约 2 秒」，**两个数都已经移动**：项数翻了一倍有余，而问它的那一屏不再是第一屏（`#/` 是对话，机器那一屏在设置页的「依赖项安装」组里）。它当时否决后台探测的理由是「要多一条『还没答上来』的状态」，而那条状态今天已经存在、有夹具、也有它的动作（`MachineSkeleton` 与 `MachineUnchecked`）——那笔代价早已付过。服务因此不再为一个没人问的答案把套接字关着几秒。
- **客户端**：`client/src/views/machine.svelte` 画一份答案（`MachineReport`）与问一次（`Machine`）；首跑屏第一步换成它。**那一屏每次打开都发一次 `DoctorRefresh`**（与「重新检查」同一条命令，不是第二条路），每次打开至多一次；城里已有的答案先画出来，探完的那一份到了再换上（§8-120）。每一行是「状态词 + 名字 + 版本或装它的命令」，状态词取自 `lang.json`，版本与命令是城给的值——页面上没有句子。`#/gallery` 有一份夹具，三行各处于人会采取不同行动的三种状态。
- **不因事件失效**：这份答案说的是城启动时看到的那一眼，账本上没有任何记录能改变它，所以 `asking` 的 `staleBy` 对它落在 `default`（不失效）。
- **验收**：`doctor::report::tests`——没有任何浏览器引擎的假机器答出 `Absent { NotOnSearchPath }`、`use` 档 `missing == ["a browser engine"]` 而 `develop` 档为空；平台不明时每一项的 `install` 都是 `UnknownPlatform`。

## 8-134 页面读到历史证明到哪里（`accounting::views::city`、`bin::assembly::attending`；wire-SPEC §8-63）

服务中的城在后台证明整条链（8-90、8-122），证明完成之前写者拒绝每一次追加。页面需要在同一刻知道这件事，否则一次被拒的命令看起来是坏了。`CityAnswer.proved` 回答它。

- **一个句柄，两个读者。** 写者线程挂上的 `storage::ChainHalt` 是判定的唯一一处：写者用它决定接不接一行，视图用它决定答不答「已证明」。`bin::assembly::attending` 在起写者之前造这只 halt，经 `RunWorker::chain_under_audit(halt)` 挂给写者，同一只的克隆经 `Views::watch_proof` 交给发布与备用两份视图。
- **答什么。** `halt.proved()` 为真时答视图此刻的头（`Some(head)`）：证明完成时写者还没写过一行，之后的每一行都由这个已证明的写者接上；为假（还在证明，或证明发现链断了）答 `None`。不经 halt 起步的视图（一次性查询、测试）起步前已同步证明过整条链，答它们的头。
- **不进账本。** 证明是这台主机对这份账本的一次核对，不是城的历史；它记在 `the history is proved` 那一行诊断里（8-121），页面读的是此刻的判定。
- 验收：`accounting::views::city` 的 `a_city_answer_says_the_history_is_proved_only_once_the_halt_says_so`。

## 8-55 一栋楼的桌面白名单，走配置那条帧（`accounting::worker::commanding::configure`；wire-SPEC §8-26、§8-45、city-SPEC §8-26）

城这一侧的两件事（楼级 `desktop:` 与 `kernel::gate::undoable`）早已落地，缺的是**把那份 allowlist 从人手里送到盘上的那一段**。

- **不新起一条命令**：`ConfigureBuilding` 问的就是「这栋楼的 runs 够得到什么」，沙箱、外部服务器与运行中的机器上的窗口是同一个问题的三面，各自可缺省。
- **不解析**：`city::write_desktop_scope` 整份覆写，字节即人给的字节。语法的权威是读它的那台 server，且它 fail closed。
- **载荷是四个面**：`city::Written { sandbox, mcp, desktop, context }` 取代四个裸布尔——调用点写 `(true, false, true, false)` 说不出哪一位是哪一面；四个面进来时也已经是一个值（`commanding::configure::Reconfiguration`），所以 `configure_building` 收两个参数而不是五个。
- **页面**：`client/src/views/desktop.svelte` 一个框装整份文件，读用 `Query::Document`（`<building>/.sprawling/DESKTOP.toml`），写用 `configure_building`。一个「每个窗口一行」的表单会是这一侧对那份语法的第二次解读。
- **验收**：`accounting::worker::building_page::tests::the_desktop_allowlist_is_written_where_no_resident_reaches_it`——人写的字节落在 `desktop_scope_path` 上，且那条地址 `is_reserved` 为真（任何写域都够不到）。

## 8-56 说出来的那句话：录音进城，一行字出来（`accounting::views::hearing`、`bin::assembly::listening::hearing`；wire-SPEC §8-27）

服务端此前只有 `gateway::transcribe` 这个适配器：一件没有任何路可以走到的东西。本节把路修通。

- **人填 URL 与 key 走既有的 attach 表单**。「哪个 endpoint、哪个 model 答这一类活」已有机制——`ModelTag`。第三个 tag `Transcribe` 因此是全部的新增面：第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。
- **`Views::transcriber`**（`views::hearing`）：锁内读出选择、造出 `Transcriber`，锁外发请求。一次转写是数秒，而那把锁是全部读的答案所在。录音到达的是城一级的门、身上没有地址，读不出任何一座楼的规矩，故这里**写明** `BuildingPolicy::new(false)` 而不是取默认值——把「口述按普通楼出门」这件事摆在读者眼前。上传带上它所属的那座楼之后，这个值同样从楼规来。
- **`assembly::listening::hearing`**：把 views 与金库收成一条 `TranscribeSink`。金库是**工人开的那一把**，经启动握手那条通道交出来（`Started.vault`）——第二个 `Custodian` 会是同一批机密的第二扇门。
- **容器从请求头读**：`AudioType::of_media_type` fail closed，拒词列出这座城发得出去的五种。浏览器录进它手上有的容器，而只有它知道是哪一个。
- **页面**：`core/speaking.ts` 管录音与上传，composer 多一个按钮，**转写结果落进输入框而不是直接发出去**——机器听错的那一句必须能改，否则它会花掉一次 run。没有 `transcribe` 选择的城不画这个按钮（`useHearing`）。
- **同一个选择也给 run 一件工具**：`transcribe`，有没有它读的是同一次 `select`，只是按 run 所在那座楼的楼规读（8-131）。
- **验收**：`wire` 的 `/transcribe` 路由在没有 content-type 时按名拒绝；`gateway::transcribe::recording` 的 `of_media_type` 认得五种容器、拒第六种并列出前五种。

## 8-131 城给 run 的转写工具 `transcribe`（`accounting::worker::workbench::tools::transcribe`；gateway-SPEC §8-12、§8-33、§8-34，runtime-SPEC §8-59）

**原因**：computer use 要能把声音变成字，而二进制里不内置任何模型（定规），所以模型只能经人接入的转写端点转写。composer 的麦克风已经经这个端点把录音变成字（8-56），run 却没有任何一条路用它。

```rust
// accounting::worker::workbench::tools::transcribe（形状 4 适配器）
impl Laying {
    pub(super) fn transcription_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<TranscribeTool>, AxError>;
}
pub(super) struct TranscribeTool { /* reader: runtime::BoundReader、transcriber: Mutex<Transcriber>、meta —— 私有 */ }
impl kernel::Tool for TranscribeTool { … }   // 名 `transcribe`；参数 `{ path }`：路径或 `cas:`／`file:` Locator；答 `{ path, text }`

// accounting::worker::workbench::Laying 多一格
book: gateway::EndpointBook,   // 派活立起那一刻的端点账本
```

- **有没有这件工具，与 composer 的麦克风读同一个选择。** `EndpointBook::select(ModelTag::Transcribe, 楼的 policy)` 成了，`gateway::transcriber_for` 把它变成设施，工具上表；拒了，工具不上表。没有第二个「这座城能不能转写」的开关。与 `views::hearing` 只差 policy：run 有楼，读楼规，于是机密楼只拿得到回环地址上的转写端点；口述没有楼，按普通楼。`select` 的三种拒绝——没选、端点已撤、机密楼而端点离机——对模型是同一件事：这座楼没有转写，一件叫了必败的工具不放上表。`transcriber_for` 的失败（HTTP 客户端造不起来）照常上抛，因为它对主模型的调用同样成立。设施放在一把锁后面：它里面的凭据解析器是 `Send` 而不是 `Sync`，而一件工具要在一波调用之间共享，于是同一个 run 的两次转写轮流进行。
- **账本在 `RunWorker::laying` 里复制一份进 `Laying`。** 工作台在驾驶这个 run 的 lane 里摆（8-113），账本属于 accounting 线程；`Laying` 本来就带着「派活立起那一刻」的值（`waiting`、`locks`、`trust`），账本是同一种值，几个端点的复制是微秒量级。被否：在 `agree_to_work` 里造好设施随 `Agreed` 带过去——那样工具在一处造、在另一处登记，「这件工具有没有」就有两处答案；只把 `Transcriber` 放进 `Laying`——`laying()` 手里没有楼规，造不出对的 policy。
- **录音从读界之内的一个文件或一个 `cas:` 块进来，只经 `runtime::BoundReader`**（runtime-SPEC §8-59）。参数与 `read` 同一种写法：相对城根的路径（审查楼里是这个 run 的树）、城内的绝对路径，或 Locator。连接器把 desktop 的录音存进 CAS，窗口里那一行 `[recording attached: …, cas:…]` 的 locator 原样交给它（runtime-SPEC §12.15）。判定是 `read` 的那一份：文法、reserved subtree、读界，链接解开后再判一次，打开后核对没换过；拒绝与 `read` 同码。读界之内的别楼也收：模型能 `read` 的东西本来就进了它对主模型的对话，录音出门去的是按本楼的 policy 选出的端点，与那段对话同一种出门；机密楼的文件楼外读不到，机密楼里的 run 只拿得到回环上的转写端点。**容器**：文件（`file:` 在内）看扩展名（`AudioType::of_file_name`），字节经 `Recording::read_from` 读到上限为止（gateway-SPEC §8-33）；块没有名字，看开头的字节，经 `Recording::read_unlabelled`（gateway-SPEC §8-34）。
- **效果是 `Effect::Read`。** 它读一个文件，城里什么都不写；录音出门去的是人为这类活选定、已按本楼 policy 判过的端点，与 run 的对话去主模型端点是同一种出门，而模型调用不是工具效果。被否：`Effect::Egress`——那一类的定义是「目的地由这次调用指名」，出网门扫的是参数里的字节，而这里参数只有一条路径，出去的是录音，判它的是已经判过的 `select`。`CostTier::Heavy`：一次 provider 往返，数秒，可能计费。`timeout: None`：设施自己的 `TRANSCRIBE_TIMEOUT_MS` 已是上限，第二个期限会是第二个权威。`render: Generic`：它产出的是文字。
- **在工具表上的位置**：内置工具与按用途加的那几件之后、城外工具（浏览器、MCP）之前。provider 按位置缓存工具数组，所以新的内置工具接在内置那一段的末尾。
- **验收**：`crates/sprawling/tests/acceptance/episodes.rs` 末尾一段。选了转写端点的城，run 调 `transcribe` 拿回脚本端点转出的字，端点收到的模型名是人选的那一个；机密楼配离机转写端点时，工具不在表上。没选转写的城不上这件工具，由 catalogue 的两条覆盖测试守着：上了表而没被调用，会被点名。连接器存下的录音：一个为这座楼存进 CAS 的 wav 块，run 以它的 `cas:` 调 `transcribe`，拿回脚本端点转出的字。工具自己的拒绝（机密楼的路径、reserved subtree、认不得的容器、不存在的文件与块）由 `transcribe` 模块的测试守着。

## 8-142 城给 run 的 OCR 工具 `ocr`（`accounting::worker::workbench::tools::ocr`；gateway-SPEC §8-34，runtime-SPEC §8-59）

**原因**：computer use 读不到字的窗口（画在画布上的界面、远程桌面）只剩截图，模型要文字反馈得有 OCR；二进制里不内置任何模型（定规），所以 OCR 与转写一样经人接入的端点。人在端点账本里为 `ModelTag::Ocr` 选一个能读图的模型，城给 run 一件 `ocr`，把 run 有权读的一张图交给那个模型，取回文字。desktop 那一侧不做 OCR，截图经连接器进 CAS（`crates/desktop/Spec.lean` §15.2「还欠的」第 1 条）。

```rust
// accounting::worker::workbench::tools::ocr（形状 4 适配器）
impl Laying {
    pub(super) fn ocr_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<OcrTool>, AxError>;
}
pub(super) struct OcrTool { /* reader、policy、recogniser: Mutex<gateway::Recogniser>、meta —— 私有 */ }
impl kernel::Tool for OcrTool { … }   // 名 `ocr`；参数 `{ path }`：PNG 的路径或 `cas:`／`file:` Locator；答 `{ path, text }`
```

- **有没有这件工具，读的是 `select(ModelTag::Ocr, 楼的 policy)`。** 成了，`gateway::recogniser_for` 把它变成设施（带上这个 face 的头，与主模型的适配器同一张），工具上表；拒了（没选、端点已撤、机密楼而端点离机），工具不上表，理由同 8-131：一件叫了必败的工具不放上表。
- **图只经 `runtime::BoundReader` 读进来**，与 `transcribe` 同一扇门、同一份判定（8-131）：楼里的一个 PNG、读界之内别楼的一个 PNG、连接器存进 CAS 的截图（窗口里那一行 `[picture attached: image/png WxH, cas:…]` 的 locator）都收。读进来的字节交 `runtime::pipeline::connector::png_picture`，与连接器存图同一种认法：只认 PNG，超过 `IMAGE_MAX_BYTES` 或头读不出，`E_INVALID_ARGS`。
- **发出去的是所选 face 的图片内容**（gateway-SPEC §8-34）：一条 system、一条带图的 user 消息，没有工具；`policy` 是这座楼的。选中的模型登记为只读字时，端点在发出前拒绝，拒词让人把 `ocr` 指向一个 `text_image` 的模型。
- **效果照 8-131**：`Effect::Read`（读一个文件，城里什么都不写；图出门去的是按本楼 policy 选出的端点）、`CostTier::Heavy`（一次 provider 往返，可能计费）、`timeout: None`（端点自己的调用期限是上限）、`render: Generic`（产出的是文字）。
- **在工具表上的位置**：紧接 `transcribe` 之后、`playback` 之前。provider 按位置缓存工具数组，所以内置工具的新成员接在内置那一段的末尾；以后的提案工具排在 `ocr` 之后。
- **验收**：`crates/sprawling/tests/acceptance/ocr.rs`。选了 OCR 模型的城（回环端点、chat 面），run 以楼里的一个 PNG 与一个为这座楼存进 CAS 的截图各调一次 `ocr`，拿回端点读出的字；端点收到的两次请求走 `chat/completions`、带人选的模型名与那张图的 base64。没选 OCR 的城不上这件工具，由 catalogue 的两条覆盖测试守着。工具自己的拒绝（不是 PNG、机密楼的路径、没有设施）由 `ocr` 模块的测试守着。
- **本节接口的当前状态**：线上已有的 `SelectModel` 能为 `Ocr` 选模型，但一个模型能不能读图（`InputKinds`）只从内置目录（`gateway::MarketSnapshot::builtin`）读，那里今天只有一行 `text_image`；其余模型按 `Text` 登记，`ocr` 在它们上面由端点拒绝。人登记一个模型时能说「它读图」，或选型点也读预置表（`provider::preset` 里各行已写着 `input`），二者之一落地，`ocr` 才能用在内置目录之外的模型上；判定它的证据是一个 `text_image` 的预置模型被选为 `Ocr` 之后仍被拒。人登记 OCR 模型的设置页控件归前端（client-SPEC 的模型表仍是三行，kernel-SPEC §8-80）。

## 8-57 浏览器是一族引擎，不是一个牌子（`bin::doctor::family`、`family::gecko`／`chromium`／`webkit`、`bin::doctor::registry`）

**原因**：doctor 的 `firefox` 一行只认 PATH 上的 `firefox` 与两条固定路径，于是一台装了 Zen 的机器被判「运行层缺 firefox」，而 `browser_bidi::engine` 起浏览器只用 `--remote-debugging-port`、`-profile`、`--no-remote`、`-headless` 四个参数——**任何 Gecko 内核的浏览器都接受这四个参数**。表按牌子问，引擎按内核跑，两者本来就不是同一个问题；Chromium 一侧更反了一层：列的是 `chromedriver`，而人装的是浏览器。

```rust
// bin::doctor::family（形状 1 decision）：三族引擎，一族一行
pub(crate) enum Family { Gecko, Chromium, WebKit }
pub(crate) enum Confidence { Tried, NeedsConfirmation, Experimental }
pub(crate) struct Member { name, program, homepage, confidence, places: PerPlatform<&[&str]>, start_menu }
pub(crate) const BROWSER_VARIABLE: &str = "SPRAWLING_BROWSER";   // 本 crate 唯一拼写
pub(crate) const GECKO_ROW / CHROMIUM_ROW / WEBKIT_ROW: Requirement;
pub(crate) fn member_at(family: Family, at: &Path) -> Option<&'static Member>;
pub(super) fn look(family, platform, search_path) -> Presence;   // 版本读自文件，故无 patience（§8-80）

// bin::doctor（表因此能说出「任一即可」与「点进它自己的站」）
pub(crate) enum Need { Required, OneOf(Group), Optional }
pub(crate) enum Group { BrowserEngine }
pub(crate) enum Detection { …, Family(Family) }
pub(crate) struct Requirement { …, homepage: Option<&'static str>, … }

// bin::doctor::registry（形状 4 adapter）：Windows 记下的那两把钥匙
pub(super) fn installed_at(program: &str, start_menu: &str) -> Option<PathBuf>;
```

- **三族，而不是三个牌子**：Gecko（firefox、zen、librewolf、waterfox、floorp、firefox-developer、firefox-nightly、tor-browser）、Chromium（chrome、edge、brave、chromium、vivaldi）、WebKit（safari，经 `safaridriver`）。每个成员给出三平台的安装路径；一族的答案是**第一个答得上来的成员**。
- **驱动是另一行，不是这一行**：一个 Chromium 浏览器自己开不出会话，进得去的是它旁边那个主版本号相符的驱动。故 `chromium` 一行是 `Optional`（它回答的是「运行中的机器上有哪个 Chromium」），而 `chromedriver` 与 `msedgedriver` 各自成行并进 `BrowserEngine` 组——这与 `Engine::choose` 吃的两件东西一一对上，**不制造第二份「怎样才算有浏览器」的权威**。被否决的备选：让 `chromium` 一行在找到浏览器却没有驱动时报 `Broken`，那要借用 `Fault::HalfWritten` 的措辞（「删掉重装」），而它对一个装好的浏览器是假话。
- **`OneOf(Group)` 而不是四行各自 `Required`**：四条进得去的路，三条关着一条开着，对一个人是「有一个浏览器引擎」而不是「缺三样」。`verdict` 因此把一组折成一个名字（`a browser engine`），排在逐项点名的那些之后；`paint::count` 用同一条规则数总结行，于是总结行与判定行**恒不互相矛盾**。
- **`Need::OneOf` 在线上说 `Required`**：`wire::DoctorNeed` 只有两个词，而页面要的是「它挡不挡路」；「这一组满足没有」由该档次的 `missing` 回答。第三个词会让页面必须同时读两处才能给一行上色，而 `DoctorNeed` 加一个变体会让今天在跑的客户端解不出整份答案。
- **`SPRAWLING_BROWSER` 只拼一次，且是引擎读的同一处**：变量压过一切探测。路径按文件名主干认族；**认不出的归 Gecko**，因为无驱动的那条起法是任何分支都接受的那一条，而 xtask render 正是这样设它。被否决的备选是「认不出就当没有」：那会对着一个人正看着的文件报「不在 PATH 上」。
- **Windows 的两把钥匙经 `reg.exe` 读，不引新依赖**：per-user 安装落在带账户名的目录里，表里的字面路径找不到它；`App Paths\<程序>.exe` 与 `StartMenuInternet\<牌子>\shell\open\command` 两个键说得出。只 `reg query`，不写；找不到就换下一个成员。被否决的备选是加一个 registry crate——一个平台、一个问题，换一份人要下载的二进制里的第三方代码。
- **`enables` 不再说 WebUI 要某个浏览器**：WebUI 任何浏览器都打得开，要 Gecko／Chromium／WebKit 的只有 browser tool。
- **`Tried` 之外的成员排在最后并带一句说明**：Tor Browser 的启动器与代理会挡在会话前面（`, confirm it by hand once`），Safari 的 BiDi 支持是局部的（`, experimental`）。`member_at` 把找到的路径认回牌子，于是报告说 `zen` 而不只是 `gecko`，`DoctorItem.homepage` 给的也是**那个牌子自己的站**。

**本章测试**：`every_family_is_a_row_and_every_member_says_where_it_is_installed`；`a_found_browser_is_reported_by_the_brand_it_is`（Developer Edition 的程序文件也叫 `firefox`，只有目录分得开）；`a_member_that_needs_confirming_is_never_the_first_answer`；`a_verdict_counts_the_required_items_of_its_own_tier_only` 的三段——只有 Zen、只有 Edge 加驱动、四条路全关。

**本章验收**：只装 Zen 的 Windows 机器上 `cargo run -p sprawling -- doctor` 报 `gecko present … (zen)` 且 `ready to use`。

## 8-58 一次装齐开发这份代码要的全部工具（`bin::doctor::table::toolchain`、`just prereqs`）

**原因**：一个人在一台新机器上想开发 sprawling，要的是 Git、`rust-toolchain.toml` 钉住的 Rust 工具链连同 rustfmt 与 clippy、just、cargo-nextest、cargo-deny、bun、`lean-toolchain` 钉住的 Lean（经 elan）、Python（经 uv），以及 render 门要的那个 Chromium 系浏览器。这份清单若在 doctor 的表与 `just prereqs` 里各写一份，一份加了 Lean 另一份没加，照着其中一份装齐的人照样在另一份面前红。

```rust
// bin::doctor::table::toolchain（形状 6 data）
pub(super) const DEVELOP: [Requirement; N];   // Develop 档的全部行，按安装先后排列
const LEAN_PIN: &str;                         // include_str!("lean-toolchain")，去掉行尾
// bin::doctor（Detection 多一支）
Detection::Listed { program, args, line }     // 程序在，且它按 args 列出的某一行以 line 开头才算在
// bin::doctor::table
pub(crate) fn prereqs() -> String;            // Develop 档渲染成 prereqs.tsv 的全文
```

- **表是权威，`just prereqs` 读它的渲染**：`crates/sprawling/src/doctor/table/prereqs.tsv` 每行 `class<TAB>name<TAB>program<TAB>windows<TAB>macos<TAB>linux<TAB>purpose`，由 `table::prereqs()` 从 Develop 档渲染；测试 `the_prereqs_file_is_the_develop_tier_rendered` 要求文件逐字等于渲染结果，不等时把应有的全文印出来。`just prereqs` 在编译之前跑，只能读文件而不能问二进制，所以读的是这份渲染而不是另一张清单；`command -v <program>` 是它的探测，`program` 为 `-` 的行（浏览器家族）归 doctor 与 render 门自己去找。被否决的备选：justfile 当权威、doctor 在编译期读它——justfile 不写三平台的装法，doctor 就得再拼一遍。
- **`class` 就是 `Need`**：`required` 是 `just check` 离了它跑不起来的（git、bash、rustup、rust、rustfmt、clippy、just、cargo-nextest、bun、render 用的浏览器、elan、lean、zig），`optional` 是 `just check` 缺了会跳过、或只由人主动跑的 recipe 调用的（cargo-deny、uv、python，以及 cargo-mutants、cargo-fuzz、cargo-public-api、kani）。elan 与 lean 为什么必需，见 §12「Lean 是开发这份代码必需的工具」。
- **行序就是安装顺序**：后一行的装法用到前一行装出的程序——rustup 之后才有 `rustup component add` 与 `cargo install`，elan 之后才有 `elan toolchain install`，uv 之后才有 `uv python install`。页面的「全部安装」按表序逐项跑，所以顺序写在表里而不是写在页面上。
- **Lean 的版本只写在 `lean-toolchain`**：`LEAN_PIN` 是那个文件的 `include_str!`（`str::trim_ascii_end` 在 const 里去掉换行），装法是 `elan toolchain install <pin>`，探测是 `elan toolchain list` 里有以 pin 开头的一行。换 Lean 版本只改那一个文件。
- **Windows 上能用 winget 的都用 winget，且按用户装**：Git、just、bun、uv、ffmpeg 的 winget 清单都有用户级安装程序，配方带 `--scope user`，于是装的时候不弹 UAC——§8-40「恒不提权」在 winget 上就是这个参数。rustup 的清单没有 scope 字段，而 rustup-init 本来就只写这个人的 profile，所以不带（带了 winget 答「找不到适用的安装程序」）；Chrome 的清单只有机器级安装程序，而每台 Windows 都有 Edge，Chromium 一族在 Windows 上由 Edge 答上，Chrome 那条配方很少被走到。
- **elan 在 Windows 上由城跑它的官方安装脚本**：winget 上没有 elan，官方装法是 `elan-init.ps1`。理由：一台新机器应当一次走完，而让人把一行 PowerShell 贴进终端正是走不完的那一步；这一条由人定下，是 §8-40「脚本只印不跑」在 Windows 的 elan 上的例外。配方是 `powershell -NoProfile -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((irm <elan-init.ps1>))) -NoPrompt 1 -DefaultToolchain none"`：`-NoPrompt` 让它不问，`-DefaultToolchain none` 让 Lean 的版本仍只由 `lean` 一行按 `lean-toolchain` 装。它仍是 `Recipe::Command`，页面仍要人按一下才跑，所以「每一条会改动计算机的命令都先给人看过」这条不变；Linux 上的 `curl … | sh` 仍是 `Print`，例外只到 Windows 的 elan 为止。
- **cargo 工具是一包，包里有哪些由仓库自己点名的地方决定**：`Requirement.pack == Some(Pack::RustTools)` 的行是 cargo-nextest、cargo-deny、cargo-mutants、cargo-fuzz、cargo-public-api、kani。测试 `the_rust_tools_pack_is_every_cargo_tool_this_repository_calls` 读 `justfile`、`.github/` 下的工作流、`tools/xtask/src/` 与 `docs/`，把其中调用的 cargo 子命令（`cargo <sub>` 里 `<sub>` 不是 cargo 自带的那些）与 install-action 的 `tool:` 行收成一个集合，要求它恰好等于这一包；多一个或少一个都点名。cargo-audit 与 cargo-llvm-cov 因此不在表里：仓库里没有任何 recipe、工作流或文档调用它们。线上每一项仍是自己的一行（`just prereqs` 与判定逐项读），页面把同一包的行画成一行，一个按钮按表序装完缺的那几项，每一项一份日志。
- **Rust 本身是一行**：`rust` 探 `rustc --version`，配方 `rustup default stable`（rustup 装好后给出一个在 PATH 上的 rustc；进仓库后 rustup 按 `rust-toolchain.toml` 自取钉住的那一版）。它与 `lean` 两行带 `Pin`：页面把钉住的版本与装着的、上游最新的并排画出（§8-120）。
- **探测与安装子进程看到同一条搜索路径**（`doctor::host::search_path`）：进程的 `PATH` 之后补上这个人的几个用户级 bin 目录——home 下 `.cargo`、`.elan`、`.local`、`.bun` 各自的 `bin`，Windows 上再加 `%LOCALAPPDATA%\Microsoft\WinGet\Links`——已在 `PATH` 上的不重复。安装程序改的是注册表或 shell 启动文件，本进程的 `PATH` 是启动时那一份；不补这几个目录，刚装好的 rustup 让下一行的 `cargo install` 找不到 cargo，刚装好的 elan 让 `elan toolchain install` 找不到 elan，整批装要重启城才能走完。
- **`same_command` 一处拼写三平台**：三平台装法相同的工具只写一次，三列各抄一遍只会让其中一列悄悄落后。

## 8-130 `just` 跑配方的那个 bash 是 develop 层的一行（`bin::doctor::table::toolchain` 的 `BASH`）

**原因**：`justfile` 开头是 `set shell := ["bash", "-uc"]`，每一个配方都在 bash 里跑，而 develop 层没有这一行，于是一台照着 `just prereqs` 装齐的新 Windows 机器仍然在 `just check` 的第一步就红。Windows 上还多一层：`just` 按搜索路径的次序找 `bash`，而系统的 `PATH` 排在这个人的 `PATH` 之前，`C:\Windows\System32` 又在系统那一段里；装了 WSL 的机器上，`System32\bash.exe` 是 WSL 的启动器，没有发行版时它打一句错误就退出。实测：同一台机器上，`PATH` 里 System32 在 Git 的 `bin` 之前时，`just` 起的是这个启动器，配方失败；Git 的 `bin` 在前时，起的是 Git 的 bash，配方通过。

```rust
// bin::doctor::table::toolchain（形状 6 data）
pub(super) const BASH: Requirement;   // Develop 档，Required，排在 GIT 之后
```

- **探测问的是 `just` 会拿到的那一个 bash**：`Detection::Listed { program: "bash", args: ["--version"], line: "GNU bash" }`。探测按 `doctor::host::search_path` 的次序找程序，与 `just` 找 `bash` 是同一个次序（进程自己的 `PATH` 在前），WSL 启动器不打印以 `GNU bash` 开头的行，于是被报为缺，而那正是 `just` 会撞上的情形。`prereqs.tsv` 里这一行渲染成 `bash --version | grep -q '^GNU bash'`。
- **Windows 的装法是一句话，不是一条命令**：`Recipe::Manual`，要人在 Git Bash 里跑 `just`。Git for Windows（git 那一行装的）带着 bash，Git Bash 的终端把它自己的 bash 放在搜索路径最前。能让别处起的 `just` 找到它的另一条路，是把 Git 的 `bin` 放到 System32 之前，那要改系统的 `PATH`，要管理员权限，而 §8-40 恒不提权。macOS 自带 bash，配方 `brew install bash`；Linux 是 `Print("sudo apt install bash")`。被否：在 `justfile` 里用 `set windows-shell` 写 Git bash 的绝对路径——按用户装的 Git 在 `%LOCALAPPDATA%\Programs\Git`，按机器装的在 `C:\Program Files\Git`，写哪一个都有一半机器找不到。
- **测试从 `justfile` 读出 shell，不从表里抄**：`doctor::tests::the_shell_every_recipe_runs_in_is_a_required_develop_row` 读 `set shell` 那一行里的程序名，要求 Develop 档有一行 `Required` 的探测程序就是它。`justfile` 换了 shell 而表没跟上，测试红。

**本节接口的当前状态**：MSVC 的链接器不是一行。`x86_64-pc-windows-msvc` 编译要 `link.exe` 与 Windows SDK，rustc 经 Visual Studio 的实例清单找它们，不经搜索路径，所以现有的四种探测（搜索路径、已知位置、程序的清单、本二进制自己）一种都问不到；装 Build Tools 的安装程序又要提权。它缺不缺，要在一台干净的 Windows 上读：`Get-Command bash, link.exe -ErrorAction SilentlyContinue`；`winget install --id Git.Git -e --scope user` 之后再读一次 `Get-Command bash`；`winget install --id Rustlang.Rustup -e` 之后 `cargo build` 能否找到链接器；最后 `just prereqs`。读数说缺，而装法只能是一句话时，这一行按本节 bash 的形状加进来，探测是一种新的 `Detection`。

## 8-59 CLI 自己的样子：一张表，四个状态词，一句下一步（`bin::doctor::paint`、`bin::doctor::screen`）

**原因**：`doctor` 的输出是左对齐的散文，版本整行贴出（`ffmpeg version N-125649-g8d3942 Copyright (c) 2000-2026 …`），没有分组也没有总结，一个人要逐行读完才找得到红的那一行。

```rust
// bin::doctor::paint（形状 3 projection：findings → 人读的行，不问机器、不读环境）
pub(crate) enum Ink { Colour, Plain }
impl Ink { pub(crate) fn or_plain(self, no_color: Option<OsString>) -> Ink; }
pub(crate) enum Status { Present, Missing, Broken, Optional }
pub(crate) enum Part { Required, Recommended }
pub(crate) fn row(finding: &Finding, ink: Ink) -> String;
pub(crate) fn count(findings: &[Finding], part: Part) -> Counted;   // 一组算一件
pub(crate) fn summary(findings: &[Finding], ink: Ink) -> Vec<String>;

// bin::doctor::screen
pub(crate) struct Asked { install, city, explain, ink }
pub(crate) fn asked(args: &[String], no_color: Option<OsString>) -> Asked;   // --no-color 与 NO_COLOR
```

- **定宽两列加一句细节**：名字列 18、状态列 9，状态是四个词之一——有／无／坏／可选。人读的是一列词，而不是一句句子。
- **版本只取第一段数字**：先找带点的十进制串（`133.0.3`、`2.43.0`），没有的话取第一个含数字的词并截到 12 个字符（ffmpeg 的 `N-125649-g8d`）。整行贴出会把其它列挤出屏幕，而 `Copyright … 2000-2026` 里的年份正是「取第一个数字」这条更笨的规则会取到的东西。
- **必备／推荐两段**：必备 = Use 档里挡路的项（`Required` 与 `OneOf`），其余全是推荐（Use 档的可选项加整个 Develop 档）。`Part::of` 是终端这条划分的唯一权威。页面不按这两段而按层分段（运行一座城／开发 sprawling），因为页面每段的进度条与「还差」读的是那一层的 verdict，而「推荐」一段混着两层，它的进度条对不上任何一句 verdict。
- **颜色是一份终端可以拒绝的提议**：`NO_COLOR`（无论它设成什么）与 `--no-color` 任一即可，且 `paint` 自己不读环境——ink 是 `screen` 决定后传进来的值，于是测试不必动运行中的机器上的变量就能要到两种答案。
- **先说话，再探测；探测并行**：标题行在第一项探测开始之前就写出并 flush，于是人面对的不是一块空屏；`examine` 为表里每一项各开一个作用域线程同时问，整份报告的等待是最慢那一项而不是所有项之和（逐项串行时首行要等 2.5 秒）。线程数就是表的行数，不按机器调：每项的成本是等一个子进程回答，不是占一个核。`Machine: Sync` 因此是 trait 的一部分。
- **每一行在它能写的那一刻写出**：`examine_each` 把每项的答复按答到的次序交给调用者，屏幕按屏幕次序（先「required」各项、再「recommended」各项，各自保持表序）缓存行文，某一行连同它前面的所有行都答了，就立刻写出并 flush。于是第一行在屏幕次序里第一项答到时就出现，而不是等最慢那一项。不按答到的次序直接写，是因为行归在两个标题之下，按答到的次序写会让行落到错的标题下，并让同一台机器的两次报告排法不同；代价是一项慢的会压住它后面的行，而这份等待由 `running` 的 patience 封顶。
- **总结与下一步**：两段各一行 `n / m ready`（一组算一件），末行是从这里往下的那一条命令——必备齐了是 `sprawling up`，不齐是 `sprawling doctor --install`。

**本章测试**：`one_row_per_item_carries_one_of_four_status_words`、`no_color_is_honoured_from_the_environment_and_from_the_flag`、`a_version_is_the_number_out_of_whatever_the_tool_printed`、`the_report_is_grouped_into_required_and_recommended`、`a_family_of_browsers_counts_once_in_the_summary`。

### 8-60 `bin::revealing`：把一条路径交给人自己的文件管理器（形状 4 适配器）

```rust
pub(crate) fn reveal(city_root: &Path, at: &Address) -> Result<(), AxError>;
// accounting::worker::RunWorker 的字段：打开时装上 revealing::reveal
reveal: fn(&Path, &Address) -> Result<(), AxError>,
```

- **worker 经它被交到的 `fn` 指针碰这里**：生产的 `Hands`（`bin::assembly::production::hands`）装上 `revealing::reveal`，经 `Hands.reveal` 交给 worker 的构造器，`Command::Reveal` 调的是那个字段，不直接调本函数。本模块启动主机的一个程序，worker 搬进 `accounting` 时它留在 `sprawling`（accounting-SPEC.md §7、§12-10）；脚本场景交一个自己的 `fn`，就不会在宿主上起文件管理器。钉住它的测试是 `a_reveal_reaches_the_file_manager_the_worker_was_handed`。

- **为什么是一条命令而不是一个链接**：浏览器打不开 `file://` 之外的东西，而 `file://` 打开的是一个目录列表而不是人平时用的那个窗口。城代为执行，于是「在文件管理器里指出来」这件事在三个平台上各自用它们自己的办法完成：Windows `explorer /select,<路径>`、macOS `open -R <路径>`、其余 `xdg-open <父目录>`。
- **文法即闸**：入参是 `Address`，它在语法上爬不出城，所以「请求城外的一个路径」这句话在线上拼不出来。此处不再加第二道路径检查——那会是同一条规则的第二个权威。
- **`/select,` 后面没有空格**：有空格时 `explorer` 把它解析成两个参数，打开的是人的主目录，退出码是 0。这条只有跑起来才会现形，所以钉在测试里。
- **两种拒绝各说各的**：地址在盘上不存在是 `E_PATH_NOT_FOUND`（页面比树旧）；文件管理器起不来是 `E_TOOL_UNAVAILABLE`（这台桌面没有处理程序）。两者都不是「城坏了」，所以都带可执行的恢复语。
- **Linux 只开父目录**：`xdg-open` 没有选中参数，而各文件管理器的选中写法互不相同——那会是一张这座城得跟着上游改的表。父目录是所有桌面都能兑现的承诺。

**本章测试**：`a_path_this_city_does_not_hold_is_refused_rather_than_opened`、`the_selected_path_travels_as_one_argument`。

### 8-62 `accounting::worker::credentials::probing`：一次 probe 答的是读数（形状 4 适配器）

```rust
pub(super) struct Probing { pub reach: kernel::Reach, pub served: Result<Vec<gateway::ModelFacts>, AxError> }
pub(super) fn reach_of(base_url: &str, proxying: Proxying, monotonic: fn() -> Instant) -> Result<kernel::Reach, AxError>;
pub(super) fn probed_payload(name: &str, base_url: &str, found: Probing) -> Result<Payload, AxError>;   // 经 Payload::of(&kernel::event::record::EndpointProbed)
pub(super) fn tuning_of(wire: wire::EndpointTuning) -> gateway::EndpointTuning;  // credentials.rs
```

- **`probe_endpoint` 不再因读不出模型表而拒绝**。它记一条 `endpoint_probed`，里面是分段读数（`gateway::reach` 量出，用时是调用方以 `Hands.monotonic` 前后两读之差，8-129-2）、模型表、以及读不出时那条拒绝自己的 code 与 subject。理由是这四段对填表的人是四个不同的下一步，而作为一次拒绝返回时它们在界面上塌成传输库的一句话。**它仍会拒绝的两件事**：凭据引用拼不出来、载荷账本不收——两者都没走到发请求那一步，因此没有读数可报。
- **`attach_endpoint` 的拒绝语义一个字没改**：probe 失败而人没点名任何模型，仍然是拒绝，因为那样的城连一个可调用的模型 id 都没有。
- **probe 按调用时的那套头与期限发出**：一个需要自定义请求头的网关，在 probe 不带那个头时答 401，人于是读到「密钥无效」，而那把密钥是好的。`request_max_retries` 在这里被兑现一次——设置页上有人正在等这一个请求；模型调用的那一份由同一个数走另一条路兑现：`dispatching::agreeing` 在选定端点处把这个 `kernel::Retries` 原样冻进 `RunPlan`，`runtime::run::drive` 据此决定一次可重试的失败之后还有没有下一次。
- **`tuning_of` 是线上词汇与 gateway 词汇之间唯一的翻译点**：零读成缺省（清空一个数字框到达线上是 `Some(0)`，而没有请求能在 0 ms 内完成），空名字的头与不以 `/` 开头的 pointer 被丢掉（表单在人打字时留着空行），`stream_idle_timeout_ms` 成为 `stream_deadline_ms`。
- **`EndpointsAnswer` 的每一行带 `label`**，取 `AttachedEndpoint::label()`，缺省即 name。

### 8-61 已经在等的那一批，合成一道屏障

`RelayGate::serve` 醒来后先把队列里等着的请求**全部取空**，再一次 `Ledger::append_all` 交下去，按位回信。

- **理由是屏障的价钱与记录条数无关**：实测（`durability_barrier`，windows-x86_64 NVMe 一档机器）一条一屏障 585.2 µs／条，五十条一屏障 13.2 µs／条，而其中真正的写只有约 2.5 µs。四条车道同时在跑、每一行都要越到这一条记账线程上来，所以一次排空手里常常不止一件；一件一件交下去，交的是同样的字节，付的是四倍的屏障。
- **等待的语义一个字没改**：回信仍然在落盘之后才发出，因为「`Ok` 即已落盘」正是 `EventRef` 之所以是一条已存在历史的引用（storage-SPEC §8-1）。否决「给端口加一个显式屏障动作、`append` 只写不同步」：那会让一条已经发出的 `EventRef` 指向一条可能还不存在的历史。
- **整波失败即整波拒**：`append_all` 的第一条拒绝结束整波，每个在等的调用方都收到同一条拒绝——与单条 `append` 在它后面那条失败时给出的承诺相同。
- **计数店是证据**：`everything_already_waiting_reaches_the_store_in_one_wave` 用一家数波数的店断言四条同时到达的 draft 不花四次波，这是端口早就允许的第二实现，而不是为这条断言新开的洞。

### 8-63 bin::serving::journal（形状 4 适配器）：一行诊断离开本进程的唯一出口

**日志给人看，而「人」不止坐在终端前。** `docs/logging.md` 把日志与账本分得干净，但没有说日志不许给浏览器看；记录页第四个透镜先前是空的，因为线上没有帧携得动一行。

```rust
pub type Clock = std::sync::Arc<dyn accounting::Clock + Send + Sync>;
pub struct Journal { /* lines: broadcast::Sender<wire::LogLine>, clock: Clock —— 私有 */ }
impl Journal {
    pub fn new(clock: Clock) -> Journal;                 // 调用方交 `assembly::SystemClock`
    pub fn sink(&self) -> runtime::diagnostics::Sink;  // 终端一份，看的人一份
    pub(crate) fn lines(&self) -> broadcast::Sender<wire::LogLine>;
}
```

- **一个 sink 两张嘴，不是两份日志**：页面读到的是终端读到的同一条 entry、同一个层底、同一个 sink。本文件不把任何一行读回来，所以「判定与恢复逻辑不读日志」照旧成立。
- **时钟在构造时交进来**：`docs/logging.md` §8 认可装配层是唯一可以采样的地方，写 entry 的库不许有第二个时间源，本模块也不许；所以 `Journal::new` 收下 `assembly::SystemClock`，每一行问它一次，而 serving 不写出 assembly 的名字（8-92）。读不到钟即 `t` 缺席，而不是把这一行丢掉——锚是 `seq`，为一个时间戳丢诊断是把代价付错了地方。
- **`Journal` 先于 `Diagnostics` 存在**：sink 是往通道里写的那一半，所以它必须先有；`Serving` 因此同时携 `log` 与 `journal`，在 `assembly::listen` 里取出 `lines()` 交给 `ServeConfig::logs`。
- **窗口 512 行**：比增量通道宽、比事件通道窄。`wire` 层底的一座城写得比人读得快，而这里丢掉的是一条诊断而不是一段历史。
- **`Level` 五个名字的映射住在这里**：`wire` 依赖图上够不到 `runtime`，一条测试把 `LogLevel` 的五个 serde 名与 `Level::as_str()` 逐个钉成相等。`Level` 现已是闭枚举（G-22），所以第六级会在这张映射表上编译失败，而不是悄悄落到某一档——归错一档与看不见的一行都不再可能。

### 8-64 机器上的两个动词：`DoctorInstall` 与 `DoctorRefresh`（形状 4 适配器）

`Query::Doctor` 答的是开城那一刻的快照（§8-53）。于是机器页只能把一行命令复制到终端，装完还要重启城才看得见结果。两条命令补上这段，执行点是 `accounting::worker::commanding::machine`。

```rust
// accounting::machine（Recipe 的唯一拒绝语；Runnable 只由它造，accounting-SPEC 8-4）
impl Recipe {
    pub fn command(&self, item: &str) -> Result<Runnable<'_>, AxError>;
}
pub struct Runnable<'a> { /* 私有：program、args */ }
// bin::doctor::running（本二进制起安装程序的唯一一处）
pub(crate) const PATIENCE: u32 = 3_600; // knocks, TICK apart
pub(crate) fn run(item: &str, runnable: &accounting::Runnable, patience: u32, log: &Path) -> Result<(), AxError>;
// bin::doctor::host：一项安装的输出写到哪个文件
pub(crate) fn install_log(item: &str) -> PathBuf; // <系统临时目录>/sprawling-install/<item>.log
// accounting::worker::commanding::machine：worker 经 RunWorker.machine（accounting::Machine）探与装，生产实现是 doctor::ThisMachine；终端的 --install 经同一个 accounting::Machine::install
impl RunWorker {
    pub(in crate::assembly) fn doctor_install(&mut self, item: &str) -> Result<(), AxError>;
    pub(in crate::assembly) fn look_at_this_machine(&mut self);
}
// bin::doctor::table：一个名字在这个平台上的配方；RunWorker.recipe_for 在打开时装上它
pub(crate) fn recipe_for(item: &str) -> Result<&'static accounting::Recipe, AxError>; // InvalidArgs：表里没有；ToolUnavailable：这个平台没有配方
```

- **查表与取平台在表旁边的 `doctor::recipe_for`，写进度行在 `doctor_install`**：worker 搬进 `accounting` 时需求表留在 `sprawling`（accounting-SPEC.md §7），所以 worker 经打开时交给它的 `RunWorker.recipe_for` 这个 `fn` 指针拿配方，而不是自己读 `REQUIREMENTS`。它不是一层穿透：它是表的查法，与表同住 `doctor::table`，一个名字要不要被拒只在那里回答。进度行仍由 `doctor_install` 写，写的时刻因此就是安装到达的时刻。钉住这条的测试是 `an_install_takes_its_recipe_from_the_table_the_worker_was_handed`。
- **只跑 `Recipe::Command`，走的是终端那条 `Machine::install`**，不是第二个安装器。`Print` 与 `Manual` 各自带着「人自己去做什么」被拒：管道进 shell 的脚本是没人读过的代码，这条纪律不因请求来自页面而松一格。需求表里没有的名字在起任何进程之前就被拒，因为页面问的是这份构建不认识的东西。
- **「这条配方这座城可不可以跑」只有 `Recipe::command` 一个家**（H-12）。它要么给出 `Runnable`，要么给出那句带恢复语的拒绝；终端（`screen`）、页面（`commanding::machine`）与机器适配器（`probe`）三处都问它，所以同一条打印配方在三扇门后读到的是同一句话。`Machine::install` 收的是 `Runnable` 而不是 `Recipe`，于是「不可跑的配方」在这一层已经不可表达，`runnable()` 与 `probe` 里那第二段措辞随之删除。
- **`bin::doctor::running` 是本二进制起安装程序的唯一一处，等待有上限**（B-25／F-10）。三件事一起成立：`stdin` 是 `Stdio::null()`，于是要人同意源协议、要人输密码的包管理器立刻读到输入结束而不是坐在一台没有人的终端前；等待是 `try_wait` 的**计数敲门**，而不是 `Command::status()` 那种没有尽头的阻塞；敲完即杀掉子进程并带着 `E_TIMEOUT` 返回，恢复语是「自己在终端里跑这一行」。**上限用敲门次数而不是墙钟，因为本二进制读时钟的地方只有 `bin::assembly` 一处**（ARCHITECTURE §10 第 4 条）；这同时让上限可断言——测试要三次敲门就得到三次，而对着墙钟的断言问的是它跑在哪台机器上。上限由 `running::patience_for(runnable)` 一处决定：`cargo install` 从源码编译，给 `BUILD_PATIENCE = 24_000` 次 × `TICK = 50ms` = 20 分钟，因为 cargo-mutants、cargo-deny 这一类在四核机器上冷编译要 5–12 分钟，3 分钟的上限让它们每次都被杀在半路；其余的包管理器下载现成的包，仍是 `PATIENCE = 3_600` 次 = 180 秒。杀不掉或收不了尸都写进那条错误的主题——本城起的一个停不掉的进程是人必须知道的事实。
- **每一项的输出一份文件**：`stdout` 与 `stderr` 写进 `host::install_log(item)`，每次安装从空文件写起。安装失败时，人要的是包管理器自己说了什么；这份文件的路径写进开始那一行日志，也写进失败的恢复语。放在系统临时目录而不是 `~/.sprawling/components/` 下，因为 components 下一个名字对应的目录本身就是 `Detection::Component` 的探测对象。
- **进度就是日志行**（`bin::doctor` 模块名）。安装是本城起的一个进程并等它，值得报告的两件事——将要跑什么、怎么结束——正好是一行日志的形状；第二条进度通道会是同一件事的第二个权威。
- **`doctor_install` 装完自己再探一遍，探不到就拒绝**：装完仍答启动快照的城，会告诉人他刚装的东西还是没有。包管理器报成功而这座城仍找不到这一项（最常见的是它装进了本进程启动之后才加进 PATH 的目录），`doctor_install` 在交出新答案之后以 `E_TOOL_UNAVAILABLE` 拒绝，主题以 `<item>:` 开头，恢复语是重启这座城。于是每一次安装恰好以两种方式之一结束——新答案里这一项在，或者一条点名这一项的拒绝——页面的「全部安装」按这两种结局逐项往下走，而不必读日志行的措辞。钉住它的测试是 `an_install_that_leaves_the_item_absent_is_refused_by_name`。
- **答案不入账本**：机器有什么不是这座城里发生的事——它在本进程之外被改变，写进历史就是写进一份会错的历史。它沿 `RunWorker::examine` 这个 sink 交给服务层的 views，与开城那一次写进去的是同一处。没有 sink 的 worker（命令行逐条驱动的那种）照样探、照样写那一行日志：一个行为取决于有没有人在看的动词，是两个动词。
- **跑在写线程上，代价有上限**：探测是十几个程序各被起一次的几秒钟，安装是一个包管理器，两者都占住其他命令排的那条队。但另一条路更糟——让读去起进程，会拿着 views 的锁把其他每一次读都堵住，而且没人要求它这么做。**此条先前接受的是「占住写线程」本身，那在无期限时等于永远**：一个等在输入上的安装程序会让 `Halt` 与 `Cancel` 都进不来，而那正是人最需要它们的一刻。故代价此后由 `patience_for` 封顶（上一条）：写线程最多被一次下载占住 180 秒、被一次 `cargo install` 占住 20 分钟。后者是明知的代价：页面在这段时间里把那一项画成「正在装」，人看得见城在等什么；把安装搬出写线程要一条把结局送回页面的新通道（今天结局是命令的回答），那是剩下的一步，不在本节。

### 8-65 `agent_protocols::mcp::sse`：一台在流上应答的 server（形状 4 适配器；实现 `agent_protocols::Outbound`）

第三种 transport，也是唯一一种「请求与它的答不是同一次交换」的。

```rust
pub(crate) struct SseServer { /* 私有：消息端点、已兑付的 headers、client、事件接收端 */ }
impl SseServer {
    pub(crate) fn open(url: &str, headers: &[(String, String)],
                       resolve: &gateway::SecretResolver) -> Result<SseServer, AxError>;
}
impl agent_protocols::Outbound for SseServer { /* call：先 POST 再等流；notify：只 POST */ }
```

- **先开流，再说话**：规范让 server 把消息端点作为第一个事件播出来，故在流开口之前无处可投。开流因此自带期限（15 s），一台始终不播端点的 server 被拒，而不是被投到一个猜出来的路径上。
- **15 s 只管「开口并播出端点」，不管流本身**：reqwest 的请求级 `timeout` 从连接一直算到响应体读完，而这条流的响应体就是整段对话，挂在 GET 上它会在 15 s 后把一条好好的流掐断，此后每次调用都读到「流已结束」。故 GET 不带请求级期限，连同发送一起放进读取线程；读取线程送出的第一条要么是开流失败（`Err`，原样是 `unreachable`／`refused` 的那条错误），要么是播出的端点，这一侧对这第一条带 15 s 期限等——连不上、不答头、不播端点三种挂法都落在同一个期限里，而流一旦开口就只由每次调用自己的 `patience` 约束。阻塞 client 自带 30 s 的整请求默认期限，同理对这条流致命，故 client 以 `timeout(None)` 建成，每次 POST 各带自己的 `patience`。播出的多半是一条路径而不是整条地址，故按流自己的地址解析——一台在反向代理后面的 server 只知道它自己那条路径。
- **读取线程与 `agent_protocols::mcp::stdio` 同形、同理由**：读流没有自己的期限，故一条线程把阻塞读变成这一侧可以带期限等的通道；丢掉最后一个句柄即丢掉接收端，下一次发送结束读取线程。
- **先开流，再说话**：规范让 server 把消息端点作为第一个事件播出来，故在流开口之前无处可投。开流因此自带期限（15 s），一台始终不播端点的 server 被拒，而不是被投到一个猜出来的路径上。播出的多半是一条路径而不是整条地址，故按流自己的地址解析——一台在反向代理后面的 server 只知道它自己那条路径。
- **读取线程与 `agent_protocols::mcp::stdio` 同形、同理由**：读流没有自己的期限，故一条线程把阻塞读变成这一侧可以带期限等的通道；丢掉最后一个句柄即丢掉接收端，下一次发送结束读取线程。
- **一次 POST 不是一个答**：对侧用 202 收下并一言不发，答随后作为事件到达。故 `call` 先投再等流，`notify` 投完即止。
- **克隆共用同一条流**：一台 server 是一次对话，无论一次 Run 握着它几件工具；两条流会让一次调用的答落在调用方没有在读的那一条上。
- **401／403 抬 `E_CREDENTIAL_MISSING`**，与 `agent_protocols::mcp::http` 同一条口径：server 在、也听懂了，缺的是一个账号，而人接下来要做的是登录而不是检查地址。健康视图（§8-66）据此把它画成「认证中」而不是「失败」。

### 8-66 `accounting::views::mcp_health`：一个地址够得到的每台 server 此刻站在哪（形状 7 投影）

```rust
impl LiveAsk {
    pub(super) fn mcp_health_answer(&self, addr: &Address) -> wire::McpHealthAnswer;
}
```

- **现问现握手，恒不折自账本**。一台 server 通不通是关于此刻的事实——一个起得来的程序、一台应答的主机、一个仍然有效的账号——记下来的那一份会在它停掉一小时后仍说它在。读配置的方式与 `Query::Document`／`Listing` 读这棵树的方式相同。
- **用的就是 Run 起点那一次握手**（`agent_protocols::handshake` ＋ `tools/list`，经 `McpLink`），故人读到的与模型拿到的不可能不一致。三种状态的判断无一处要猜：`E_CREDENTIAL_MISSING` 即 `Authenticating`，其余拒绝整条上线。
- **这是唯一一条按秒计的读，代价写在这里**：它拿着 views 的锁，每台 server 一次握手。页面因此只在人打开 MCP 页或加完一台 server 时问一次，恒不上定时器、也恒不由记录触发（`staleBy` 对它答 false）。
- **金库是借来的，不是这里开的**（`Views::lend_the_vault`，开城时交一次，与 `found_on_this_machine` 同形）：同一批凭证上的第二个句柄就是第二扇门。没有金库的 `Views`（重建、测试）把要凭证的那台 server 报成兑付不了，而不是把引用当成值发出去。

### 8-67 四段全部入库，以及读回它们的三个投影

**装配点。** `freeze_plan` 把四段全部 `intern` 进内容仓库，而不再只落 must-read 名单上那几份。先前 city 段与 JOB 段有对应对象、building 与 resident 两段没有，于是 `prompt_assembled` 里四个哈希有两个在 `cas/` 里找不到——一个人拿着哈希读不回原文。内容寻址天然去重：一栋楼的规则无论被多少次 run 冻结，仓库里都只有一份。

city 段与 building 段同时携上它们的来源文档（`Assembled`：字节与 `Vec<SegmentSource>` 恒同行），地址由 `city::building_path`／`city::agents_path` 反算而来，**不在这里重述一栋楼的文件布局**。resident 段与 run 段不携来源：前者由身份与目录拼成，后者由 brief 与交接文本拼成，都不是可被打开的文档。一条位于城内却拼不成 `Address` 的路径是失败而不是猜测——一行读者点不开的来源，比一次回绝更糟。

**三个投影。**

```rust
// views::prefix
fn prefix_answer(&mut self, run: RunId) -> Option<wire::PrefixAnswer>;
fn content_answer(city_root: &Path, locator: &Locator) -> Option<wire::ContentAnswer>; // 锁外：读内容仓库（8-100）
// views::skills
type SkillPins = BTreeMap<(String, B3Hash), Vec<RunId>>;
fn skills_answer(city_root: &Path, building: &Address, pins: &SkillPins) -> Option<wire::SkillsAnswer>; // 锁外：扫书架（8-100）
// views::git_status
fn git_status_ask(&self, building: &Address) -> GitStatusAsk; // 锁内：城根、楼、最近一次检查点
impl GitStatusAsk { fn read(self) -> wire::Answer; } // 锁外：读工作树（8-100）
```

**五条口径：**

1. **`prefix_answer` 取该 run 最早的一条 `prompt_assembled`。** prefix 一次冻结管一次 run 的一生，之后每一轮记的是同样四个哈希；取最早的那一条，一次没走过第一轮的 run 也仍有答案。
2. **字节的可读性判定只有一处。** `views::document` 的 `read_bytes` 同时服务树上的文件与仓库里的对象——什么样的字节算文本，不取决于它被存在哪里。
3. **技能的书架在被问的那一刻扫盘，而 pin 出自历史。** `city::Library` 是书架的权威，旁边再留一份索引就是磁盘说法的第二份副本；而「哪些 run 用过」折自 `run_started` 里那张 `skills` 表（`Views::skill_pins`，键为名字与哈希成对），不是第二次扫盘。pin 表在锁内复制出来，扫盘在锁外对着这份副本做（8-100）。
4. **`git_status_ask` 的比较基准取自历史而不是 HEAD。** 该楼最近一条 `checkpoint_committed`／`pr_merged` 就是基准，它由 `commits_answer(Some(building), None, 1)` 给出——变更栏旁边显示的那一行，正是提交列表打开时的第一行。
5. **仓库句柄按次打开。** 这是投影里唯一一处伸向它不拥有的目录的读；跨重建留着的句柄会活得比开它的那座城还长。

### 8-68 `bin::release`：这是哪一版，以及唯一一次去问注册表（形状 4 适配器）

```rust
pub enum Built { Released(Release), FromSource }
pub fn built() -> Result<Built, AxError>;      // option_env!("SPRAWLING_RELEASE_TAG")
pub fn newest() -> Result<Release, AxError>;   // GET registry.npmjs.org/sprawling/latest
pub fn answer() -> ReleaseAnswer;              // 两读合判，恒不失败
```

**五条口径：**

1. **人问才发生。** 没有定时器，没有首次运行时的探测，也不搭另一条命令的车：`status` 只读编译进来的东西、一个套接字都不碰，只有 `status --check` 会出网。`QUICKSTART.md` 的开场承诺是「什么都没装、没注册服务、删掉文件夹就干净」，一个按自己时间表去够注册表的二进制，是在拿那句承诺换一个没人问过的问题。
2. **什么都不更新。** 二进制住在哪，归当初装它的人管——归档路径归 `sprawling install`，npm 路径归 npm（`tools/xtask/src/channel/shim.js` 已立此规）。故本模块只报告然后停下，答案里印的是该跑的命令，选哪条仍由选了安装渠道的那个人决定。
3. **`Built` 两态而不是 `Option<Release>`。** 缺席不是一个缺失的值，而是关于这次构建的一个事实：从工作树构建出来的二进制没有可比的对象，把它报成「过期」是在回答另一个二进制的问题。tag 由 `release.yml` 经 `SPRAWLING_RELEASE_TAG` 传入，`build.rs` 声明该变量（`cargo::rerun-if-env-changed`），否则 cargo 会拿上一个 tag 编出来的二进制顶数，而它的每一份都会报错版本。
4. **问 npm，不问 GitHub。** 本项目每一次发布都是 pre-release，而 `GET /repos/{owner}/{repo}/releases/latest` 按设计排除 pre-release，对本仓库答 404。npm 的 `latest` dist-tag 才是 `bunx sprawling` 真正解析的东西，问它才是问人真正有的那个问题。
5. **失败说清停在哪一阶段。** 只在失败路径上多发一次 `gateway::reach`，把 `kernel::reach` 已定义的分阶段读数——名字没解析、连不上、握手失败、对方答了什么状态——放进 recovery。「它没成功」不是一个人能据以行动的答案，而这条路径上多一次请求换一句能行动的话是划算的。退出码报的是问题有没有被回答，而不是答案是什么：版本过期是消息不是故障，而读不到注册表会让人以为自己查过了。

### 8-69 真端点验收闸 `just e2e`（`crates/sprawling/tests/e2e.rs`）

这个仓库的其余检查都在回答一个它自己写的 provider。这一条把一个人粘贴的 base URL、key 与模型名拿来，attach → 选模型 → 派活，再回头读账本——于是「key 填了，从来没跑通」是一次红，而不是一份报告。

**六条口径：**

1. **缺席就一行说明并通过。** 轮次是一张表：`gateway::known_hosts()`（由 `PRESETS` 算出）的每个主机的每个 face 各一轮，那个主机的 key 与模型名在 `SPRAWLING_E2E_KEY_<主机>`／`SPRAWLING_E2E_MODEL_<主机>` 里（主机名大写、非字母数字换成 `_`，如 `SPRAWLING_E2E_KEY_API_DEEPSEEK_COM`），一个 key 用于这个主机的全部 face；再加一轮「人粘贴的那个端点」，由四个环境变量 `SPRAWLING_E2E_BASE_URL`／`SPRAWLING_E2E_KEY`／`SPRAWLING_E2E_MODEL`／`SPRAWLING_E2E_DIALECT` 给出，接表外的中转或回环地址上的服务。每一轮缺什么由测试自己印出来，justfile 不抄第二份；一个主机只给了 key 与模型名中的一个是写坏的配置，同样印出。表不手写，加一行预置主机就多一组轮次（gateway-SPEC §8-32 是同一张表的白盒一半）。每个测试对每一轮各立一座城，失败时点名是哪一轮。这与 `just adversary` 是同一个诚实形状：静默跳过的闸比没有闸更糟，而在每台没有 key 的机器上都红的闸没有人会跑。持有凭据的作业设 `SPRAWLING_E2E_REQUIRED=1`：此时粘贴端点那一轮的四个变量缺任何一个都判红，按主机的一轮只给了一半也判红，并印出同一行缺了什么——凭据在那里本该齐全，缺席是配置坏了，而不是所在机器正当地没有 key。只认 `1`；其他值与未设同义，免得一个拼错的开关悄悄换回跳过。
2. **不进 `just check`。** 它花的是别人的钱与别人的网络；有没有 key 是一台机器可以正当地没有的东西。夜间作业跑它，凭据住在仓库的变量与密钥里：只有设了仓库变量 `SPRAWLING_E2E_BASE_URL` 的仓库才跑这个作业，没设的（分叉、新克隆）作业显示为跳过而不是红或绿；一旦设了，其余三个就是欠着的，`SPRAWLING_E2E_REQUIRED=1` 让缺哪个就红哪个。开关选 base URL 而不是 key，因为作业级 `if` 读得到变量、读不到密钥。
3. **从 `RunWorker::handle` 进城，不另起进程。** 本闸要判的真端点是 provider 的那一个；`CARGO_BIN_EXE` 与套接字那一侧归 `tools/adversary/`（xtask boundary 闸：白盒 Rust、黑盒 Lean）。Roadmap §20.6 写的是「经真二进制 HTTP 面」，此处按既有的边界规则改为「经 `wire::server` 递帧的那扇门」——多起一个进程只会多付一道边界成本，而断言仍然共享产品的类型，正是那道闸判为两头不讨好的形状。
4. **兼容格式随它的端点走，不做命令行开关。** §24.3 写的是 `just e2e --dialect messages --relay`。兼容格式是「你指向的那个端点说哪种线」的属性，与 base URL、key 同源，故与它们并列为环境变量；旗标是这个事实的第二个家，且能与另外三个变量互相矛盾。字面拼法由 `DialectKind` 的 serde 命名给出（`anthropic`／`open_ai`），测试不另列一张表。
5. **三个断言分三个测试，因为它们的波次不同。** ①`model_called` 出现且账本不带 `E_CONFIG_INVALID`／`E_WIRE_MISMATCH`（W1 关门）；②`model_called` 说得出输出上限来自哪一环（`ceiling_from` 非空）——**这一条在 1.1 上限链条落地前是红的，它是钉在那片叶子前面的桩**，与 ① 合并就成了两个事实一个判决；③ 故意断 key 必红，且 recovery 非空、错误码不是 `E_CONFIG_INVALID`——被拒的凭据是端点的答复，不是一份写坏的配置。
6. **时长由两次 cargo 调用与两个调参守住。** 编译不占额度：`--no-run` 先付编译（一台 windows-msvc 机器上冷构建实测 2m18s），`timeout 180` 只罩测试进程。`request_max_retries = 0` 让 gateway 的退避一次都不睡，`timeout_ms = 60000` 让一次请求封顶一分钟（Roadmap §0.0：单次 `sleep` ≤ 10 秒、单个测试进程 `timeout` ≤ 180 秒）。三个测试各把每一轮跑一遍，所以 180 秒罩住的是「轮数 × 三次派活」：一次填进的主机多到装不下时，分几次跑，每次只导出其中几家的变量。

**attach 就是一次真调用。** `admit` 为空表示「这个端点服务什么就收什么」，于是登记当场去问它的模型清单——一把被拒的 key 在 attach 处就被回绝，走不到派活。故三个用例都把 attach 与派活串成一个 `Result` 来判，而不是假定拒绝只会在最后一步出现。

### 8-71 空着的上限不是被抹掉的上限（`credentials::endpoints::select_model`）

- **缺陷**：设置页每次选模型都把整行发上来，于是一个人重选自己已经登记过的模型，就把当初填的上限用一个空框覆盖掉了；下一次 messages 兼容格式的调用因为写不出 `max_tokens` 被拒（A 章 B-01 的第二段）。`None` 从此表示「这次没说」，而不是「这次要清空」。
- **权威从高到低**：人这次填的 → 这个 endpoint 与这个 model id 上一次登记的 → 钉版目录行。**按 endpoint 与 model id 读上一次，而不是只按 tag**：把一个 tag 指向另一个模型时，旧模型的上限不得跟过去。上一次登记从 `book.choices()` 读回——书是「这座城登记了什么」的唯一陈述，在它旁边另存一份就是第二个权威。
- **`context_tokens` 同理**，`0` 是「这次没说」；两个数字读法一致，因为它们来自同一个空表单。窗口梯是人这次填的 → 上一次登记的 → 钉版目录行 → `gateway::provider::preset::window_for`（gateway-SPEC §8-17）；四档都沉默时窗口为 `0`，上下文提醒随之不响。
- **输出上限的其余几档住 gateway**（`provider::ceiling`，gateway-SPEC §8-17）：上游 `/v1/models` 的陈述、预设表与策略缺省 `OUTPUT_CEILING_DEFAULT`，以及 chat 与 responses 两面在人与上游都沉默时的 `ProviderDefault`。装配层不复写那条规则，只把人层、书里的值与这个端点的兼容格式交给它——一条规则两个家，漂开的总是没人看的那个。
- **来源入账**：`model_selected` 带 `ceiling_from: person | upstream | preset | policy | provider`，拼写取 `OutputCeiling::word`，载荷由 `gateway::router::payload` 一处写。`provider` 行的 `max_output_tokens` 缺席：请求里没有这个字段。账本里看得见来源，因此一次被截断的跑是读出来的，不是猜出来的。

### 8-73 run 标识直接取自摘要，而停不下来的疑问算「停」（`accounting::worker::dispatching::agreeing::run_id_for`、`accounting::worker::driving::lane`）

- **缺陷**：`run_id_for` 把摘要印成十六进制再逐对解回字节，两步各带一个 `unwrap_or`——`from_utf8` 失败取 `"00"`，`from_str_radix` 失败取 `0`。一次解不出的摘要于是变成全零的 run id，而两条不同的活会得到同一个标识。
- **改法**：`B3Hash::as_bytes()` 的前十六字节即标识，解析这一步整个消失。字节与旧写法逐位相同（印出来的十六进制正是这些字节），故账本与 replay 的字节不变。
- **`Interrupting::ask` 的同一类默认**：`backlog.stopping(id)` 的 `Err` 此前读作「没停」。读不到那张表的城答不出这个作用域还开着，于是改答为「停」——一次多余的取消看得见，一次漏掉的取消让 run 跑在人已经关掉的作用域里。
- **`SignalDesk::take_steer` 的拒绝不折平**：desk 返回 `Result<Option<Steer>, AxError>`——`Ok(None)` 是空队列，`Err` 是一件已离队、却读不成插队信的信（非 steer 型，或载荷里没有文字）。`ask` 对 `Err` 答 `Interrupt::None`，与 `accounting::worker::desk` 给人那一侧一个空 steer 的答案同字：安全点不是为一封读不懂的信停下来，而没有文字的信也没有内容可以交给这一跑。区别不在答案而在不折平——desk 把每一件取走的信都记成 `Consumed`，于是它不再在两个出口之间消失。

**本章测试**：`two_jobs_at_one_millisecond_get_two_run_ids`（`accounting::worker::dispatching::tests`）。

### 8-74 折叠读不懂的那一行就说出来，而「关」与「开」只有一种拼法（`accounting::worker::folds`、`views::holding`）

```rust
fn scope_of(scope: &wire::HaltScope) -> kernel::event::Scope;   // accounting::worker::naming：唯一的翻译
impl Governance { pub(crate) fn absorb(..) -> Result<(), AxError>;   // views/governance.rs
                  halted: BTreeSet<kernel::event::Scope> }
impl RunWorker { fn halted_by(&self, addr: &Address) -> Option<kernel::event::Scope>; }
```

- **`absorb` 改 `Result`**：读不懂的审批项与缺字段的 `run_started` 此前被静默丢掉，于是「历史里有一条这个 build 读不懂的线」表现为开城后少了一批待答项与一段活的说明。两处改为 `E_WIRE_MISMATCH`，恢复语指向写下这段历史的那个 build。
- **「关／开」与「谁来答」的拼法都搬进了 kernel**：`Admittance`、`Scope`、`autonomy_word` 住 `kernel::event`，两个折叠与写方经 `Payload::of`／`Payload::read` 读同一个结构。此前 `Governance` 把不认识的状态词读成「开」、`Views` 忽略整行，而 `read_autonomy` 把读不懂的委派静静读成「人自己答」——三处默认都没了，读不懂的行是一次 `E_WIRE_MISMATCH`。
- **`halted` 存 `Scope` 而不是字符串**：`halted_by` 用 `Scope::covers` 问包含关系，`split_once(':')` 连同它解不出地址时的静默跳过一并消失；`CityAnswer.halted` 在答的边界上用 `Display` 拼回同样的词。
- **写的一侧同源**：`set_admission` 收 `kernel::event::record::Admittance`，载荷由该枚举的 serde 拼写写出。

**本章测试**：`an_unreadable_approval_item_stops_the_fold`（`accounting::worker::folds::tests`）、`an_appointment_this_build_cannot_read_is_refused_rather_than_defaulted`（`kernel::event::record::governance`）。

### 8-76 哪些记录会动计划，是一张穷尽表（`accounting::plan_view::reach::may_move_plan`；accounting-SPEC.md 8-6）

```rust
enum PlanReach { Untouched, Stale, NodeFreed, NodeStopped }
fn may_move_plan(kind: EventKind) -> PlanReach;
```

- **失效判定是按 `EventKind` 的穷尽表，没有通配臂**：加一种事件，就要在这里说它动不动计划。
- **没有地址的记录**：只要它的类别会动计划，就清掉每一份解析；一条没有地址的 `checkpoint_committed` 因此不会让每栋楼继续报旧的表。
- **`pr_merged` 读作 `Untouched`**：它同样会把文件落进楼里，改成 `Stale` 要连着改 `views::commits` 的期望，那是这张表之外的一件事。

**本章测试**：`a_record_with_no_address_stales_every_plan_it_could_have_moved`、`every_event_kind_has_a_reach`（`accounting::plan_view::tests`）。

### 8-78 一次派活在会计线程上花了多久，城自己说出来（`accounting::worker::dispatching::running`、`accounting::worker::workbench::servers`）

```rust
// stage_dispatch：进出各读一次城钟，Level::Trace 报出
"prepare_dispatch took {ms} ms for {addr}"
// mcp_tools：同法，单独一行
"mcp_tools took {ms} ms over {n} declared server(s), offering {k} tool(s)"
```

- **两行而不是一行**：`mcp_tools` 是常驻连接表（F-13）唯一能省掉的那一段，混进总数就说不清省了多少。总数含协商、开房间、写简报、立 run、铺工作台与冻结计划；这一切都在会计线程上，期间没有任何车道的追加被服务。
- **量在先，门在后**：`tools/xtask/budgets.toml [prepare_dispatch_ms]` 只记读数与机器，不设上限——上限若写在读数之前，要么形同虚设，要么挡住正是要修它的那次改动。
- **读钟读不出不丢工具**：`mcp_tools` 无法报出 `Result`，因此钟失败时只是不报这一行；连接是工作，读数是诊断。
- **本章测试**：`a_dispatch_says_what_it_spent_before_the_drive`（`accounting::worker::dispatching::tests`）盯住「读数被说出来」这一件事，不断言数值——墙钟数值属于跑它的那台机器，属于 `budgets.toml` 的那一行。

### 8-79 会话冻下的形状只选一次（`accounting::worker::dispatching::session_shape`、`accounting::worker::dispatching::running`）

```rust
impl RunWorker {
    /// 记下这次派活冻下的形状，或拒掉一次会移动会话已冻下形状的派活。
    pub(super) fn choose_shape(&mut self, at: &Assignment, model: &gateway::ModelEntry)
        -> Result<(), AxError>;
}
```

- **不变量一句话**：一个会话的调用形状选一次。房间的 `CONFIG.toml` 在它的第一个 Run 写下 `[model] name`（人选了强度就一并写下 `[model] effort`），此后每次派活读回来与自己要冻下的形状对拍，不等即拒。
- **为什么住在派活面**：模型、输出上限与 effort 都上供应方的线，中途任何一个移动都会让会话此后每一轮为一段字节从未变过的 prompt 付全价。这道对拍在写简报之前，因此被拒的派活不落一个字节、不动会话一行记录——`stage_dispatch` 的序幕顺序（§8-40）就是这条拒绝的位置理由。
- **对拍本身是 runtime 的**：`runtime::turn::CallShape::verified_against` 拥有比较与三句拒绝语，恢复语指向人现在就做得到的两件事——**把动过的那一项改回去，或换一个地址派这件活**（runtime-SPEC §8-4-1；`/new` 上线后改的是那一个常量）。派活面只负责造出两个形状：会话记下的那个，与这次派活将要冻下的那个（模型来自 endpoint book，effort 来自梯子）。
- **effort 比的是梯子在这里解析出的值**，不是房间那一行：`[model] effort` 一个键都没写过的会话，冻下的是「让供应方决定」，而从这一档挪到任何一档同样是形状移动。
- **模型比的是登记面现在的答案**：`SelectModel` 改的是城级 tag→model 登记，在那里拦会把正常配置一起禁掉；被挡的是它落到一个已开会话上的那一步。会话记下的模型仍在 endpoint book 里时，上限与窗口从那一行读（它们是模型的属性，抄一份进房间的配置就是同一个事实的第二个家）；模型已不在登记面时，模型那一臂先拒。
- **`[model] name` 不是「这个 Run 用哪个模型」的权威**：Run 的模型仍由 `EndpointBook::select` 选，会话记录只说它当初从哪个模型开始。两者不一致时这次派活被拒，而不是两个家各说各话（city-SPEC §8-14、§8-4 的 `own_layer`）。
- **地址就是楼时，地址自己就是会话**：那种地址的「自己那份 `CONFIG.toml`」就是楼的 `CONFIG.toml`（`Layer::Resident` 与 `Layer::Building` 同一文件，city-SPEC §8-4）。
- **本章测试**：`accounting::worker::dispatching::session_shape::tests::a_session_keeps_the_shape_it_froze_and_refuses_a_different_effort`——同一房间连续两次派活，四条前缀逐槽位哈希相同且 CAS 里的字节相等；第三次改 effort 被拒（`E_CONFIG_INVALID`，恢复语给出两条出路），会话的 effort 一行未动、没有多出一条 `prompt_assembled`；第四次相同请求照跑并再次冻下同一批字节。`a_model_chosen_after_a_session_opened_does_not_reach_it`——已开会话之后改城级登记，往该会话的派活被拒，房间记录仍是原来的模型，而新会话拿到新模型。

### 8-80 浏览器的版本读自它旁边的文件，而不是跑它一次（`bin::doctor::version_file`）

**原因**：`family::look` 对找到的每个浏览器跑一次 `ask_version(path, "--version")`。Windows 上 Chromium 系的浏览器收到 `--version` 不打印版本，而是**开一个窗口**；浏览器已在运行时，新进程把请求交给已有实例后自己退出，于是 `running::stop` 杀掉的是那个壳，窗口留在屏幕上。这条探测在每次服务城时跑一遍（§8-54），设置页的「重新检查」再跑一遍，于是一台装了两个 Chromium 牌子的机器每次启动都被弹出两个窗口。**一次健康检查的副作用不该是替人开浏览器**。

```rust
// bin::doctor::version_file（形状 4 适配器）：装它的那个程序在它旁边写下的版本号
pub(super) fn beside(program: &Path) -> Version;

// bin::doctor::family：一族的答案不再经过任何子进程
pub(super) fn look(family, platform, search_path) -> Presence;
```

- **三种读法一条顺序，且不按族分支**：`application.ini` 的 `[App] Version`（Gecko 在 Windows 与 Linux 把它写在程序旁，在 macOS 写在 `../Resources/`）→ 程序旁以版本号命名的目录（Chromium 在 Windows 的 `Application\<x.y.z.w>\`，在 macOS 的 `Contents/Frameworks/*.framework/Versions/`）→ 程序上方 bundle 的 `Contents/Info.plist` 里的 `CFBundleShortVersionString`（macOS 三族通用，Safari 只有这一条）。**族不是参数**：「这个牌子属于哪一族」的权威是 `family::claims` 与 `member_at`，在这里再判一次就是同一个事实的第二个家。
- **版本目录按数字逐段比，不按字典序**：升级过的 `Application\` 下常常同时留着两个版本目录，而 `154.0.4258.9` 与 `154.0.4258.32` 的字典序把旧的那个排在后面，于是报出的是已被换掉的版本。
- **读不出版本仍然是 Present，`Version::Silent` 的意思随之扩成一句**：「版本号读不出来」——一个印了空行的程序与一个旁边没有版本文件的浏览器，对报告是同一件事，而两者都不使这一项缺席（§8-40 已定：不说话的工具仍是装了的工具）。`describe` 因此是 `no version` 而不是 `said nothing`：后者对一个从未被问过版本的浏览器是假话。
- **失去的那一件事如实记**：`ask_version` 顺带证明了「这个程序起得来」，读文件不证明。一个在盘上却起不来的浏览器此后报 Present，而真相在 `browser_bidi::lazy` 起它时以 `E_BROWSER_UNAVAILABLE` 出现。这是用「每次服务都开窗」换「探测期发现起不来」：前者每次启动都发生，后者只在真要用浏览器那一次才要紧。
- **驱动仍然问**：`chromedriver`／`msedgedriver` 是 `Detection::Program`，它们真的打印版本且不开窗，`ask_version` 与它的 deadline 因此留在那条路上，只是不再有浏览器走它。
- **被否决的备选**：① 只在 Windows 上改读法——同一个事实（浏览器的版本）会有两个家，而另外两个平台上的 spawn 同样是几百毫秒与一个别人的进程；② 读 `HKCU\Software\<牌子>\BLBeacon\version`——每个牌子一把钥匙、每个人一份注册表，而版本目录一条规则答全五个 Chromium 牌子，且它是装它的程序刚写下的那一个；③ 给 `Version` 加一个「来源」变体上线——`DoctorVersion` 的四个词回答的是「版本是什么」，来源是城的内政，上线会让今天在跑的客户端解不出整份答案（§8-57 对 `DoctorNeed` 的同一条理由）。

**本章测试**：`version_file::tests::a_browser_says_its_version_through_the_files_beside_it`（三种布局各一份夹具目录：Gecko 的 `application.ini`、Chromium 的两个版本目录取数字大的那个、macOS bundle 的 `Info.plist`；旁边什么都没有的程序答 `Silent`）；`no_browser_is_started_to_learn_its_version`（`family.rs` 的正文里没有 `ask_version` 这个拼写，于是这条不变量在唯一能破它的那个文件上被钉住）。

**本章验收**：装了 Edge 与 Zen 的 Windows 机器上 `sprawling up` 与设置页「重新检查」期间不出现任何新的浏览器窗口；`cargo run -p sprawling -- doctor` 仍报 `gecko present … (zen)` 并带版本号。

### 8-81 空着的密钥框不是删除（`accounting::worker::credentials::endpoints::kept_credential`）

**原因**：供应方表单把 vault 答复的引用只存在组件实例里（`setup/providers/form.svelte` 的 `held`），页面一卸载就没了；`EndpointSummary` 上线只带 `has_credential: bool`，不带引用，所以表单也无从向城要回来。于是第二次打开设置页按「看看」或「接上」时，帧里 `secret: None`：城读成「没有凭据」，探测不带任何 `Authorization` 发出（人读到的是 401「密钥无效」，而那把密钥是好的），接上则把已归档的引用覆盖成空——**静默丢 key**。

```rust
// accounting::worker::credentials（形状 2 value）
pub(super) enum Credential { Absent { header: Option<String> }, Key { .. } }

// accounting::worker::credentials::endpoints（形状 1 decision）
fn kept_credential(&self, name: &str, dialect: DialectKind, header: Option<String>) -> gateway::AuthSpec;
```

- **一条规则一个家**：`endpoint_of` 是 probe 与 attach 共用的那道门，空引用的读法因此只有它一处，`ProbeEndpoint` 与 `AttachEndpoint` 不可能对同一个空框给出两种答案。
- **空不是删，删是另一个动词**：拿掉一个端点的凭据要一个自己的命令，现在还没有（见 8-46-11 撤销 attach 那一格）；空着的框永远不承担这个意思。一个既能表示「不改」又能表示「删掉」的字段，会让每一次不相干的编辑都带着删除凭据的风险。
- **留引用、重算头**：归档的是 `AuthSpec`（头 + 引用），沿用的只是引用，头按这次进来的接口形态重算——同一把 key 从 chat 面挪到 messages 面要从 `Authorization: Bearer` 变成 `x-api-key`，照抄旧头会对一把好 key 答 401。人自己命名的头仍然压过推导，`Credential::Absent` 因此带着 `header`。
- **头由接口形态一处推出**：`Absent` 与 `Key` 两条路都经 `gateway::AuthSpec::for_dialect`，人自己命名的头压过推导。被否决的备选：按归档的头原样沿用——那会让 API key 在换面时带着旧头 401。
- **前端说的话此后是真话**：`lang.json` 的 `setup_key_stored`（「此名下已有密钥，留空则沿用」）先前只在表单自己还记得引用时出现，而城当时并不沿用。现在城沿用，那句话改为在**城说这个 id 有凭据**时出现——一句话一个家，不新增第二个键。
- **被否决的备选**：① 把 `secret:realm/name` 放进 `EndpointSummary` 让表单送回来——凭据引用是城的内政，上线只为让页面把它原样送回，等于给同一个事实开第二个家，还多一条泄露面；② 让表单按约定重新拼出引用（`referenceOf(id)`）——那只对这张表单自己登记过的 key 成立，`import` 与环境变量来的端点引用不同名，会把别人的引用送进这一个端点。

**本章测试**：`credentials::tests::kept::an_empty_key_keeps_the_credential_this_city_has_archived`——带 key 接上后，再一次空框 probe 与空框 attach，三次模型表请求都带 `authorization: Bearer sk-archived`，且端点的 `auth` 仍是原引用。

- **`attach_endpoint` 不以探测为准入条件（gateway-SPEC §8-10 是权威，这里只记装配侧）**：鉴权头由 `gateway::AuthSpec::for_dialect` 按兼容格式产出（人填的头优先），于是 Anthropic 兼容端点拿到的是 `x-api-key` 而不是必然 401 的 `Authorization: Bearer`。探测失败时，若 `admit` 非空则按人报的型号登记（`probed: false`，另写一条 `effect` 级诊断点名探测的错），`admit` 为空才拒，恢复语是「把要用的 model id 报上来，再登记一次」。落选的是「探测失败即拒、让人先修好 `/models`」：多数兼容端点根本不服务这个接口，那条路等于让人去修一个对端从未承诺过的东西。
- **`Credential` 是穷举枚举**：`Absent { header }`／`Key { reference, header }`。「这次没填」与「填了一把 key」是两件事，前者保留城已有的凭证，两个 `Option` 拼不出这个区别。`Credential::entered` 是线上命令的唯一入口。

### 8-82 同一个地址上的新一段：`/new`（`accounting::worker::commanding::sessions`、`Command::OpenSession`、`EventKind::SessionOpened`）

**原因**：房间的第一个 run 把模型与强度冻进它自己的 `CONFIG.toml`，此后形状不同的派活都被 `E_CONFIG_INVALID` 拒（§8-79）。换过主模型的人因此再也派不出去，而拒绝的恢复语原先指向两件做不到的事（§8-4-1 已改成诚实的那一条）。设计本身没错——缓存前缀不能中途换模型——错在拒绝之后没有出口，这一章给的就是那个出口。

```rust
// wire::command（形状 2 value）
pub enum Carry { Nothing, Handoff }        // Nothing 是第一个变体，即默认
Command::OpenSession { addr: Address, carry: Carry, from: Option<Origin>, idem: IdemKey }
// Origin = kernel::Origin { run, at_seq }，由 kernel 拥有，wire 与 runtime 都读它

// kernel（形状 2 value；事件表逐变体登记）
EventKind::SessionOpened                   // payload：{ carried: bool }；地址在记录自己的 addr 字段

// accounting::worker::commanding::sessions（形状 1 decision + 一次写）
fn open_session(&mut self, addr: &Address, carry: Carry) -> Result<(), AxError>;
```

- **行李里只有行李。** `carried` 在 payload 里，房间在记录自己的 `addr` 字段里（`record_at`）——一条属于某个地址的线本来就在信封上说了地址，payload 再抄一份就是同一个地址的第二个家（kernel-SPEC §8-4）。
- **一段会话是房间上的一段，不是房间本身**：`/new` 不换地址、不换身份。市长身份按 `hall/mayor` 精确匹配（`city::spine_files::hall`），换到 `hall/mayor-2` 就把身份丢了。
- **继承进的是 window，不是 prefix**（S2 改写了路线图早先那句话，理由记在这里）。四段 prefix 各是一份**文档**（`PrefixPlan` 的 `SourceDoc`），由人写、可编辑、逐字节哈希；而一段对话是模型说了什么、工具答了什么，`Window` 自己的文档就写着「frozen prefix 的字节永不落在这里」。把历史塞进 prefix 要有第五个槽位（而 prefix 是四段的类型），会让 `prompt_assembled` 声称历史属于它并不属于的那一段，还会让被缓存的前缀每回合都长。**所以 `RunPlan.inherited` 是 window 的材料**（`run/lifecycle.rs` 在开场任务之前推入），而它可重建的证据不是段哈希而是 `run_forked { from, at_seq }` 加上母亲自己的那些行（runtime §8-2 的 `inherited`）。
- **一份继承只属于开这一段的那一跑。** 房间的当前一段从 `session_opened` 带上来的 `from` 落在折叠里（`accounting::worker::folds::session`），第一次派活取走它并写下 `run_forked`（这一行同时也把「用掉了」记进折叠）；同一段里的第二次派活不再继承。一个分支是一个开头，而开头的那一跑就是继承的那一跑。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。这一验只读 `from` 指的那一行：worker 常驻一份 `storage::LedgerIndex`，验之前 `refresh`（只读上次之后追加的字节），再 `line_at(at_seq)` 取那一行、解出它属于哪条 run。不走 `runtime::replay::verify_ledger_dir`，因为那一步把整本历史读进内存、逐行验链再解析——94 MB 的账本上是一次 +67 MiB 的瞬时内存和几十毫秒，只为回答一行的归属；而 worker 是这本账唯一的写者，打开时已经验过它（`JsonlLedger::open` 的尾部恢复），整链的校验属于 replay 与 `verify`。被否决的备选：每次验前重建索引——它仍然扫全部段。
- **`--carry` 带摘要，也带上一跑对话的地址。** 摘要说找到了什么，只有 transcript 说是怎么找到的；接手的 run 本来就在 run 槽位里读到 `Predecessor transcript: <room>/<run>.jsonl`，带过来的一段的第一跑同样读到，指向这个房间里最后开始的那一跑。这份「欠着的前任」与 `from` 同住 `accounting::worker::folds::session`：折叠按 `run_started` 记下每个房间最后开始的 run（重建时读这一行，在世的 worker 在 `freeze_plan` 冻好一跑时自己记，因为 runtime 写的那一行它看不见），`session_opened { carried: true }` 把那一跑记为欠着的前任，`carried: false` 清掉它，这一段的第一跑开始即用掉。不往 `session_opened` 里加字段：前任是哪一跑已由历史里的 `run_started` 决定，payload 再写一份就是第二个家。被否决的备选：`/new` 时把整本账重验一遍去找最后一跑——一次人按下的命令，其代价随城的历史线性增长。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。
- **分支先验再清。** `from` 指的行必须是那条 run 自己的行（`origin_is_real`），否则 `E_INVALID_ARGS` 现在就到人手里——而不是先把这个房间的形状清掉，再让一跑扑空。这一验只读 `from` 指的那一行：worker 常驻一份 `storage::LedgerIndex`，验之前 `refresh`（只读上次之后追加的字节），再 `line_at(at_seq)` 取那一行、解出它属于哪条 run。不走 `runtime::replay::verify_ledger_dir`，因为那一步把整本历史读进内存、逐行验链再解析——94 MB 的账本上是一次 +67 MiB 的瞬时内存和几十毫秒，只为回答一行的归属；而 worker 是这本账唯一的写者，打开时已经验过它（`JsonlLedger::open` 的尾部恢复），整链的校验属于 replay 与 `verify`。被否决的备选：每次验前重建索引——它仍然扫全部段。两条测试（`checking_a_branch_origin_does_not_verify_the_history` 与分支重建的 `inheriting_a_branch_does_not_verify_the_history`）以一本 genesis 之后断了链的账本作证：`verify_ledger_dir` 拒它，这一验与这次重建照常作答。被否决的读数：拿一次验与一次 verify 的墙钟之比——它量的是跑测试那台机器的负载，并行跑时会翻转。
- **`Carry::Nothing` 必须真的清掉 `Handoff.md` 的槽位**：`accounting::worker::freezing` 无条件读 `city::handoff(root, room)` 并把它折进下一个 run 的 prompt，只清配置而留着文件，新一段仍会继承上一段的摘要，于是开关不起作用（city-SPEC §8-14b 拥有那一步）。
- **默认不带，理由是 `/new` 对人意味着什么** ——「在这个工作区新开一个会话」，：`/new` 对人意味着「在这个工作区新开一个会话」，带上上一段的摘要是需要说出来的例外；**没有交接时 `--carry` 不弹问、不拒绝**，事件里如实写 `carried: false`——一段新会话就是人要的那件事，没有理由因为交接槽位空着而拒他。被否决的备选：默认带、`--fresh` 不带（路线图早先的建议）——它把例外当成了常态，而且换模型后的新一段仍受旧摘要影响。
- **拒绝只有一条**：地址上有活跃 run → `E_BUSY`，恢复语「先 `/stop`，再 `/new`」。一次派活正在写这一段的形状时把它换掉，等于让两个 run 各自以为冻的是同一份前缀。
- **分叉是同一件事多一个起点**：S2 把它做成 `OpenSession { from: Option<Origin> }`，而不是第二个动词；`Fork` 帧的退休随 S2 一起落地。
- **客户端走同一条路**：`core/commands.ts` 的 `openSession(addr, carry)`；`core/slash.ts` 的 `/new [--carry]`；composer 上方那行的「新对话」按钮；以及拒绝通知上的动作（§8-4-1 的那句话从此指向一个真存在的动词）。

**本章测试**（路线图原先写的 citysim `session_rotates.toml` 没有落点：citysim 的场景是 Rust 结构体，不是 toml，而且这条链路——派活被拒、`/new`、再派活——属于动词所在的 `sprawling`，不属于 runtime 的回合循环；改记在这里）：
- `accounting::worker::dispatching::session_shape::tests::a_new_session_lets_the_room_use_the_model_chosen_since`：dispatch（模型 A）→ `select_model`（B）→ dispatch 被 `E_CONFIG_INVALID` 拒 → `open_session` → dispatch 成功，且房间这次冻的是 B。
- `accounting::worker::commanding::sessions::tests` 的五条：房间里有 run 工作时 `E_BUSY` 且房间一字未动；`Nothing` 清形状也清槽位、事件写 `carried: false`；`Handoff` 留摘要、照样清形状、事件写 `carried: true`；没有摘要时 `--carry` 不拒也不撒谎；带过来的一段的第一跑读到上一跑 transcript 的地址（`a_carried_session_names_the_previous_runs_transcript`）。

**本章验收**：`cargo nextest run -p sprawling -p sprawling-city -p sprawling-wire -p sprawling-kernel` 绿；`cargo xtask wire-ts`、`wiring`、`specalign`、`apisync` 绿。

### 8-141 分支的第一个请求接着母 run 最后一个请求的字节（`accounting::worker::freezing`、`runtime::fork`；runtime-SPEC §8-58）

**要什么。** provider 只为逐字节相同的前缀复用缓存：分支的第一个请求要以母 run 最后一个请求的字节开头——冻结前缀的四段，经 gateway 按兼容格式渲染之后的整份请求体，再到母 run 发过的每一条消息。消息那一半由 runtime 按 `run_started.opening` 重建（runtime-SPEC §8-58）；前缀那一半由 `accounting::worker::freezing` 为分支重新组装，本节判两半合在一起、经真实渲染之后的字节。

**同一间房、`Carry::Nothing`、两次都是与人交谈的派活时，字节相同。** city 段、building 段、resident 段读的是同一组文件与同一版冻下的身份；`Carry::Nothing` 清掉了房间的 `Handoff.md`，没有 `--carry` 的前任，所以 run 段只剩 `city::RunBrief::Principal` 那一句，两次相同；模型、上限与工具表也相同。

**设计上就不同的三种，不改，理由在各自的规则里。**

1. 母 run 以 `FromJob` 开篇：第一条消息改写，理由在 runtime-SPEC §8-58。
2. 分支开在另一间房：building 段以房间地址开头（`freezing::building_segment`），房间换了，从这一段起就不同；分支在哪间房是人的选择。
3. `Carry::Handoff` 或派活带了目标：run 段带上交接或分支自己的 JOB.md，run 段不同；这一段本来就不进缓存断点（`BreakpointPlan::marks_edge`），它的字节是这一次 run 自己的任务。

- 验收：`accounting::worker::freezing::tests::lineage` 的 `a_branch_first_request_carries_the_bytes_of_the_mothers_last`：一座真城、一个记下每个请求体的假 provider；母 run 在 `lab/room1` 答一句，`OpenSession { carry: Nothing, from: 母 run 的最后一行 }` 之后同一间房再派一次活；分支第一个请求体去掉 `messages` 之外的每个键与母 run 最后一个请求体相同，`messages` 以母 run 的 `messages` 开头。

### 8-83 客户端包落在工作区的 `target/web-dist`，与 cargo 的输出目录无关（`build.rs` 的 `BUNDLE_DIR`）

**原因**：「客户端包在哪」有三个读者、两种答法。`client/vite.config.ts` 把包写进工作区的 `target/web-dist`；`xtask::bundle::dist` 在工作区的 `target/` 下找它；`build.rs` 却在 `CARGO_TARGET_DIR` 下找。三者只在没有设这个变量时一致。设了之后，`just build-web` 写出的真包没人嵌入，二进制带着占位页通过构建，只留一条 cargo warning。

- **权威是 `build.rs` 的 `const BUNDLE_DIR: &str = "target/web-dist"`，值是相对工作区根、以 `/` 分段的整条路径**，不再只是目录名。父目录 `target` 以前在三处各写一次（vite 的 `../../target`、`xtask` 的 `root.join("target")`、`build.rs` 的 `CARGO_TARGET_DIR` 分支），名字只有一个家而位置有三个；把整条路径放进一个常量，位置才只有一个家。`xtask::bundle` 用 `syn` 读这个常量（xtask-SPEC §8-18），`artifact` 门要求 `client/vite.config.ts` 与 `justfile` 拼出同一条路径。
- **`build.rs` 不读 `CARGO_TARGET_DIR`。** 客户端包是 bun 的产物，不是 cargo 的产物；它的位置由写它的那一步决定，与 cargo 把编译产物放在哪无关。
- **生成的 `client_embed.rs` 多一个常量 `CLIENT_BUNDLE_DIR`**，值取自 `BUNDLE_DIR`。`serve` 在只有占位页时提示 `--web-dir <路径>`，路径读这个常量，不再手写一份。
- **被否决的备选**：让 vite 读 `CARGO_TARGET_DIR`，由 justfile 注入。那样每个读者都要复刻 cargo 解析目标目录的规则：环境变量、`.cargo/config.toml` 的 `build.target-dir`、相对路径按当前目录解析。这条规则会在 TypeScript、`build.rs`、`xtask` 里各有一份，而 `build.rs` 原先那份已经与 cargo 不同（它按工作区根解析相对值，也不读 `build.target-dir`）。重开条件：客户端包改由 cargo 自己构建（例如 wasm 客户端在 `build.rs` 里编译），那时它才真是 cargo 的产物。

**本章测试**：`main::tests::the_embedded_client_is_the_bundle_the_workspace_built`——工作区 `target/web-dist` 下有完整的包时，嵌入表的路径集合与盘上的文件集合相等，且 `CLIENT_COMPLETE` 为真；没有完整的包时，`CLIENT_COMPLETE` 为假。这条测试只在 `CARGO_TARGET_DIR` 指向工作区以外时才能区分对错。

### 8-84 记账线程的循环是一个有名字的函数，两件仪表直接驱动它（`accounting::worker::attend::attend`、`accounting::worker::driving::tests::instruments`）

```rust
// accounting::worker::attend —— shape: adapter
/// 记账线程的主循环：relay 请求、至多一轮回家的活、desk，按这个次序（§8-42-4），直到 desk 关门。
pub(crate) fn attend(worker: &mut RunWorker, desk: &CommandDesk);

// accounting::worker::driving::flight —— 只在测试构建里有
#[cfg(test)]
pub(in crate::assembly) fn measuring_relay(&self) -> Relay;   // 与车道同一个 gate 发出的写面
```

**为什么把循环从闭包里拿出来**：仪表要驱动的是生产在跑的那个循环本身。一份抄来的 relay 形状，只要记账侧与生产的等法有一处不同，量出的就是抄件：生产的循环在两个定时等待之间轮询时，抄件量出 5 µs 一次往返，同一次往返在服务中的城里是 31.7 ms。所以 citysim 不再留那份抄件，多 run 并行这一类负载就由 `instrument_relay_round_trip` 量（citysim-SPEC 8-6）。循环只要还有第二份写法，仪表就会量错对象。`spawn_worker` 装好 `Serving` 与观察者之后调用 `attend`，这是它唯一的生产调用者。

**两件仪表**，都在 `accounting::worker::driving::tests::instruments`，都标 `#[ignore]`：它们量墙钟，一次要跑十几秒，不属于 `just check`；`just bench` 在 citysim 那一行之后跑它们（`cargo nextest run -p sprawling --release --run-ignored only -E 'test(/::instrument_/)' --no-capture`）。两件都经过同一套生产部件：`attend` 跑在自己的线程上，命令经 `CommandDesk::post` 进门，模型是回环上的假 provider（`fixture::provider`，带一个 `pace` 钩子决定何时作答）。

| 仪表 | 场景 | 读数 |
|---|---|---|
| `instrument_relay_round_trip` | 一轮活停在它的第一次模型调用上（provider 不作答），于是循环处在「有车道在跑」的那个形状里；另一条线程拿 `measuring_relay` 连续追加 200 条，逐条计时 | `store=disk`：城自己的 `JsonlLedger`，每条一道屏障；`store=memory`：同一个 `JsonlLedger` 开在 `storage::FaultFs` 上，屏障是一次内存拷贝；另给 `store=memory` 不过河时自己的追加耗时，两者之差就是过河本身 |
| `instrument_dispatch_gap` | 移植自 perf-latency 的双派活场景：run A 在 `lab/east` 跑 30 个 `status` 回合；A 的第五次模型调用到达时，往楼 `lab`（不带房间，按 8-86 的规则取名）派 run B；provider 把任何含取名提示的调用压 3 s，所以一旦派活重新向模型要名字，读数会显示出来 | A 相邻两条记录 `t` 之差的最大值与中位数，单位 ms |

`instrument_relay_round_trip` 另断言内存存储过河的 p50 不超过 1 ms（`ROUND_TRIP_P50`）：那条往返里除了一次内存拷贝全是 harness，所以它量的就是 harness。磁盘存储只报读数不断言：它的中位数是设备的 fsync，这是物理下限，因机器而异，一个写死的毫秒数只对一类机器成立；它与内存那一行之差才是磁盘的份额。目标还有一条没写成断言：内存存储过河的 p50 不超过 10 µs，它只在 `--release` 下有意义，而 `crossing=none` 那一行在调试构建里已是 20 µs 量级。

读数行由仪表模块自己渲染，一行一个读数：`<仪表> <键=值>… machine=<os>-<arch>, <n> core(s)`，每行都带 `samples`、`floor_us`、`p50_us`（空档那一行是 `max_ms`、`median_ms`）。它不用 citysim 的 `perf load=…` 文法：那份文法属于 citysim 的 bench Main，本 crate 够不到它。`instrument_relay_round_trip` 是四个负载场景里多 run 并行那一个的读数；`instrument_dispatch_gap` 不属于四个负载场景。

**决定**：仪表放在 crate 内的测试里，而不是给 citysim 开一扇公共门。relay、`serve_flight` 与 desk 都是 `pub(crate)`；为量它们而开的公共面没有生产调用者，而且要进 apisync 基线。**败给的方案**：citysim 经 `RunWorker::handle(Dispatch)` 从外面驱动，再用 provider 两次请求之间的空隙推算 relay 往返。那个空隙里还有检查点（每波 20–90 ms）与工具，推算出来的是每回合剩余，不是一次往返。

**重开参数**：两件仪表只经过 desk、relay 与 provider 三个面；`attend` 的等法再怎么改，只要这三个面不变，仪表就不用改。

### 8-85 每个模型一段追加提示词（`accounting::worker::freezing::model_note`，形状 4 适配器）

同一套城规、楼规与身份，交给不同的模型时常常要补一两句只对那个模型说的话（例如某个模型爱省略测试输出，某个模型需要被提醒先读再改）。这段话放在城里一处、按 provider 与模型分文件：

```rust
pub(super) const MODELS_DIR: &str = "models";            // <city>/.sprawling/models/<provider>/<model>.md
pub(super) struct ModelNote { pub(super) at: Address, pub(super) bytes: Vec<u8> }
/// 这次派活选中的 endpoint 名与模型 id 下的那段话；文件不在即 None。
pub(super) fn model_note(city_root: &Path, provider: &str, model: &str) -> Result<Option<ModelNote>, AxError>;
```

- **位置只有一处，在城一级的保留区里**：`<city>/.sprawling/models/<provider>/<model>.md`，`<provider>` 是派活时选中的 endpoint 名（`AttachEndpoint` 的 `name`），`<model>` 是模型 id 原样。放在保留区而不是城根，是因为城根下的目录名就是楼的地址，一栋叫 `models` 的楼会与它撞名；保留区也是居民写不到的地方，一段约束模型的话不该由被约束的模型改写。楼一级不设第二处：两处就要回答「楼的那段覆盖还是追加城的那段」，而今天没有一个读者需要这个区分。
- **追加在 run 段末尾，因而在整段 system prompt 的末尾。** 与楼段接 `AGENTS.md` 走同一扇门（`Assembled::extend`：空一行接上，记一行来源）。它随 run 段一同冻结、一同入库，整个 run 内字节不动，所以每一回合的缓存前缀都盖得住它；`prompt_assembled` 的 run 段哈希盖住它的字节，来源行写出它的地址与长度，WebUI 的提示词视图（sprawling-SPEC §8-67）因此照原样列出「这个 run 用的是哪个模型的哪一段」。重放读的是账本里那一行，不重新读文件，故逐字节一致。
- **文件不在，行为逐字节与没有这个功能时相同**：run 段不多一个字节，来源表不多一行。
- **文件在却读不出**（权限、目录占位、不是文件）是 `E_STORAGE_FATAL`，指名那条路径，派活被拒；不静默跳过，理由与 `city_segment` 相同：人写下的那段话静静躺在盘上没被读，而 run 照着缺了它的提示词干活，没有人会被告知。
- **地址先于路径。** 位置先按地址文法拼出（`Address::parse`），再落到盘上。模型 id 若含地址文法拒绝的字符（`:`、`\`、`.`／`..` 段），它在城里就没有可指名的那段话，按「没有文件」处理；先拼路径再读则会让一个含 `..` 的 id 读到保留区以外的文件。
- **被否决的备选**：把它做成第五段。四段恒四是 `FrozenPrefix` 的类型（runtime-SPEC §8-4），`verified_system_hashes` 与 `rebuild_prefix` 都按四段对拍；为一段随模型而变、随 run 冻结的文字加一个槽位，要改的是三个 crate 的冻结不变量，换来的只是一个本来就能从来源行读出的边界。重开条件：有读者需要按模型单独缓存这段话（例如跨 run 的缓存断点放在它之前）。

**本章测试**：`freezing::tests::model_note::a_note_for_the_chosen_model_ends_the_system_prompt`——为 `house/m-local` 放一个文件，派活发出的请求里最后一条 system 消息以这段文字结尾；`freezing::tests::model_note::a_note_for_another_model_is_not_sent`——别的模型的那段话不出现。

### 8-86 房间名按规则取，起名不调用模型（`accounting::worker::dispatching::session`）

```rust
// accounting::worker::dispatching::session —— shape: decision（`session_for`、`rule_name`）
/// 人给了名字、或地址已含房间时原样返回；地址只有一段且没有名字时返回 `rule_name(task)`。不调用模型。
pub(super) fn session_for(addr: &Address, session: Option<SessionName>, task: &str) -> Result<Option<SessionName>, AxError>;
/// 任务原文里前四个 ASCII 字母数字词，小写，用 `-` 连起来；一个也没有、或拼出保留名时是 `work`。
/// 结果总是合法的 session 名；`SessionName::parse` 仍是判定者，所以签名带着它的错误。
pub(super) fn rule_name(task: &str) -> Result<SessionName, AxError>;
```

**规则**：派到一栋楼（地址只有一段、旁边没有 session）上的活，房间名由 `rule_name` 从任务原文里取：按非 ASCII 字母数字字符切词，丢掉空词，取前四个，转小写，用 `-` 连起来；每个词最多留前 15 个字符（`RULE_WORD_MAX`），所以四个词加三个连字符最长 63 个字符，总在 `SessionName::parse` 的上限 64 之内，规则名永远是合法的 session 名。一个 ASCII 词也没有、或这些词拼出城自己保留的名字而被 `SessionName::parse` 拒绝时，名字是 `work`。重名由 `city::open_room` 加序号（`work`、`work-2`……），规则本身不查盘。

**为什么**：模型的回答不受字符集约束，而 `SessionName::parse` 接受任何不含分隔符的文字：一个回了「收到。」的模型会让 run 落进人找不到的房间 `shop/收到。`。问模型要名字还挡在首字前面：派活要先等一次完整的非流式调用，再发出 run 的请求。规则名在记账线程上用几微秒算出，不读 book、不读 vault、不起线程，派活帧到达后发出的第一条 provider 请求就是 run 本身（`a_bare_building_is_named_by_rule_and_the_run_is_the_first_call`）。机密楼的任务原文也不会为了起名被发给任何模型。

**决定**：完全不调用模型起名。**败给的方案**：保留模型起名，放到 run 开始之后，再校验长度与字符集、失败时回落到规则名。那样房间要么在 run 开始后改名（房间是 dispatch 在盘上写的第一件事，ARCHITECTURE §5 第 3 步，run 的文件已经在里面），要么多出一个只为改名存在的线程与在飞计数；一个人要一个好找的名字时，发到 `building/name` 就有。只含非 ASCII 文字的任务都叫 `work`、`work-2`，这是这条规则的代价；让它重新值得调用模型的参数是：一个名字能在 run 开始前、不增加首字延迟地取到。

### 8-87 粘进派活里的 key 进 vault，文字里只留引用（`accounting::worker::dispatching::custody`）

**原因**：人在页面上把一把 provider key 粘进任务或目标时，这段文字原样进了三处：发给模型的请求、账本里的派活记录、房间里的 `JOB.md`。三处都能被城里的居民 `grep` 到，账本还会随城搬走；而 `kernel::secret::scan` 早已能认出这些形状，只是派活这道门从不问它。

```rust
// accounting::worker::dispatching::custody（形状 4 适配器）
impl RunWorker {
    /// 把文字里每一段 provider 形状的 key 存进 vault，原处换成它的 `secret:` 引用。
    pub(in crate::assembly) fn take_custody(&mut self, text: String) -> Result<String, AxError>;
}
```

- **一道门，一处权威**：调用点是 `stage_dispatch` 里 `agree_to_work` 之后、`session_for` 之前。人打的派活、外面敲门的唤醒、计划推进起的活都经过这里，所以三种入口不会各有一套规矩；放在同意之后，是因为存 key 是一次写入，而城不肯接的活什么都不写；放在命名之前，是因为命名要把任务文字发给摘要模型。
- **认什么由 `kernel::secret::scan` 定**：只换形状表命中的段（`SecretSpan::provider` 为 `Some`）。熵命中不换：提示里的提交哈希、校验和也是高熵串，换掉它们，居民就读不到它要处理的那个值，而形状表列出的前缀不会出现在这类值里。
- **存进现有的 `gateway::Custodian`**：它按机器选后端——系统的凭据服务，或用口令派生密钥、逐条 ChaCha20-Poly1305 加密的 `encrypted-file`（gateway-SPEC 8-21）。另起一把 AES-GCM 密钥就是同一件事的第二个家，也会多出一处密钥要保管。
- **引用是 `secret:pasted/<provider>-<seq>`**，`<seq>` 是存这把 key 时账本的下一个序号。每存一把 key 都追加一条 `secret_captured`（`origin: "pasted"`），所以序号不会重复：同一毫秒的两次派活也不会让后一把覆盖前一把。名字里不放时间，也不放随机数，citysim 回放时仍逐字节一致；也不放 key 的哈希或尾巴，`secret_captured` 的规矩是记录行里不带明文，也不带哈希前缀。
- **失败**：vault 拒绝写入时整个派活失败，错误原样返回（`E_CONFIG_INVALID`，恢复语由 vault 给出）。此时房间和 `JOB.md` 都还没写；如果照原文继续派活，正是这一节要堵的泄露。
- **被否决的备选**：① 在页面上拦下粘贴，让人先去设置页登记——人粘 key 的时候，多半就是要居民用它，拦下来只会让人换个地方再粘一次；② 在请求出门时替换——账本和 `JOB.md` 在出门之前就已写下原文。

**居民手里的工具**：居民往城里写文字、读城里的文字，都经过工作台上的工具：`edit` 写文件，`exec` 的命令行可以写文件，`read`、`search`、`exec` 的输出回到模型。装配台把**每一件**工具登记上 bench 之前包上一层 `accounting::worker::workbench::tools::kept`：

```rust
// accounting::worker::workbench::tools::kept（形状 4 适配器）
pub(in crate::assembly) struct Keeper { /* vault, 摆台时账本的位置, 计数 */ }
pub(in crate::assembly) struct Kept { /* Box<dyn kernel::Tool>, Arc<Keeper> */ }
impl kernel::Tool for Kept {
    /* invoke: 参数里每一个字符串先交给 custody，再交给工具；
       工具的结果里每一个字符串交给 custody，再回到模型 */
}
```

- 同一段替换逻辑：派活文字、工具参数和工具结果都经过 `custody::kept_text`，所以认什么、怎么切、切歪了怎么报错只有一处。
- **参数**：`edit` 的 `new`、`exec` 的命令行，以及其它工具的每一个字符串参数，工具收到的都是引用。所以 `edit` 写进文件的、`exec` 命令行里 `echo … > f` 写进文件的，都只有引用；回给模型的 diff 里也只有引用。
- **结果**：`read` 读到的文件、`exec` 打印的输出里的 key，在回到模型之前换成引用，所以下一个发给模型的请求里没有原文。
- **引用是 `secret:written/<provider>-<pos>-<n>`（参数）和 `secret:output/<provider>-<pos>-<n>`（结果）**：`<pos>` 是装配台摆出时账本的位置，每次派活在摆台之前都写过记录，所以两次派活的 `<pos>` 不同；`<n>` 是这次派活里第几把不同的 key，一次派活的所有工具共用一份记录，所以两件工具不会把两把 key 存到同一个名字下。同一把 key 在同一侧再次出现时交回第一次的引用，不再写 vault：记录按 key 的 BLAKE3 摘要（`kernel::B3Hash::digest`）找到它的引用，所以 vault 随一次派活里不同 key 的个数增长，而不是随工具调用的次数增长；摘要只留在这次派活的内存里，不进账本也不进 vault，`<n>` 就是记录里已有的条数加一，不会绕回。工具运行在 drive 里，拿不到 `&mut RunWorker`，写不了账本，所以不能像派活那样用存 key 时的下一个序号。
- 没有命中的参数和结果原样交出，不复制。
- vault 拒绝写入时：参数里的 key 存不进去，这次调用失败，工具不运行，文件不动；结果里的 key 存不进去，工具的效果已经发生，所以调用照常成功，结果交给模型时那把 key 换成 `[key withheld: <错误码>]`（错误码是 vault 给的，比如 `E_STORAGE_FATAL`），原文与引用都不出现。回一个错误会告诉模型这次调用失败，它可能重做一件做过就收不回的事，比如又发一次请求、又写一次文件；换成标记，模型知道那里有一把 key，也知道它为什么拿不到引用。
- 账本里的 `tool_called`、`tool_result`、`model_returned` 本来就经过 `runtime::turn::ledger::Journal::append_redacted`，key 在写进账本之前已换成指纹标记；这一层管的是工具本身和模型看到的东西。
- **被否决的备选**：① 在 `runtime::EditTool`、`ExecTool` 里各自扫描——那要让 `runtime` 认得 vault，而 vault 属于装配层，也会让每件工具各有一份规矩；包一层就不必改 `runtime` 的公开面。② 只包 `edit`——`exec` 的命令行和每件工具的输出照样把 key 带进文件和请求。

**尚未覆盖**：① 工具存下的 key 没有 `secret_captured` 记录，因为工具拿不到账本；要补就得把引用放进一张 desk，drive 结束后由装配层补记。② `exec` 的命令自己取到的 key（比如从城外的文件复制进来）写进文件时不经过这一层，只有它打印出来的部分会被换掉。③ 模型回复里的工具参数原样回到后续请求：模型只可能写出它自己编出来的 key，因为它看到的都是引用。

**本章测试**：`accounting::worker::dispatching::custody::tests::a_pasted_key_reaches_the_vault_and_nothing_else`——任务文字里夹一把 `sk-ant-` 形状的 key 派活，断言：模型收到的每个请求、城目录下的每个文件（账本和 `JOB.md` 都在其中）都不含原文；请求里带着 `secret:pasted/anthropic-…` 引用；vault 按这个引用解出的正是原文。`accounting::worker::workbench::tools::kept::tests::a_written_key_reaches_the_vault_and_not_the_file`——模型用 `edit` 新建一个含 key 的文件，断言：文件里没有原文，只有 `secret:written/anthropic-…` 引用，vault 按这个引用解出原文。`accounting::worker::workbench::tools::kept::tests::a_key_a_tool_reads_reaches_the_vault_and_not_the_model`——城里的文件里有一把 key，模型用 `read` 读它，断言：之后发给模型的请求里没有原文，只有 `secret:output/anthropic-…` 引用，vault 按这个引用解出原文。`accounting::worker::workbench::tools::kept::tests::one_key_seen_twice_is_kept_under_one_reference`——同一把 key 两次交给工具、再交另一把，断言：前两次工具收到同一个引用，第三次收到下一个编号。

## 8-60 提示词语料的分层：哪类事实住哪一层（`docs/City.md`＋`ToolMeta`＋`Catalog`）

**依据是成本的形状，不是篇幅的偏好。** 四段前缀与工具 schema 在 `runtime::turn` 的**每一趟请求**里全文重发（`turn.rs:108-117`），于是同一批字节有两种代价：**窗口**是进上下文的一次性入场费（请求累积，前缀不随 turn 增长），**钱**是每 turn 重付（`prompt_cache_breakpoint` 命中后按 `cache_read_price` 折价）。两种读法下窗口占用完全相同，所以「多写一句」永远是全城每次请求少一份工作空间。

由此定下分工，`ToolMeta` 的两个字段各管一半：

| 层 | 字段 | 落点 | 只写 | 执法者 |
|---|---|---|---|---|
| 目录行 | `disclosure` | `catalog.render()` 进 Resident 段 | 它是什么、什么时候该用 | `claim_tool/tests.rs` 的 548 B 预算 |
| 说明书 | `params` 各字段的 `description` | `tool_defs()` 随请求 | 怎么用：字段、取值、默认、拒绝条件 | 同上（量的是两者之和） |
| 二级展开 | `CatalogEntry::expansion` | 按需 `read` | 整套纪律 | — |

`plan` 的六个动作就住在说明书里（`action` 那句），目录行只剩 84 B 余量而原文已占 83 B——**把动作抄进目录行既付两遍钱也放不下**，那条预算就是这个决定的执法者。`signal` 的四类 kind 同理。

**搬走的**：市长与书记的角色描述（`docs/templates/MAYOR.md`、`CLERK.md` 已逐条写着）、`signal`／`goal`／`pr` 的动词解释（各自 disclosure）、委托的一层上限（`delegate` 的 disclosure）、模式语义（`Mode::catalog_entry()`）、六份文档清单（各模板开头的自述已逐条重复）、浏览器语义（`BUILDING_DISCLOSURE`）、Python 子集（`exec` 的 `arm` 描述）。

**补进 City.md 的**：读全再动手、外置记忆（照模板写、按需读）、通信（先读别人留下的、加入前先问、方式由现场定）、隐私（**上下文本身是泄露面**，不问不找不需要的隐私与密钥值，拿到就叫人换）、环境探测、licence 与 copyright、引用保留完整上下文与出处、重要事实对第二来源交叉验证。这些都是四类身份都成立的话，才留在这个每跑都要付的段落里。

**一句话规则**：City.md 的每一句都必须对 `EPHEMERAL_SEGMENT` 扮下的工人成立——它只有 `JOB.md`、不共享楼、不写 Memo。不成立的降级到 `RULES.toml` 或模式的 catalog 条目，**不降级到 `URBANITE.md`**：那份文件坐在常驻自己的地址上，它改得动，把城级规则放进去等于让被约束者起草规则（`hall.rs` 把市长与书记的身份放在保留子树，理由同一条）。

**`citysim::ablation` 是这一章的尺**：它按段落切除文档、报出每段独占哪些能力。当前读数是 12 段、32 项能力、**全部独占**（无一 `Restated`），即每项能力恰好一个家。

## 8-88 开城的次序：先占端口，再写第一行，最后才说 running（`bin::assembly::listening`、`main::city`）

**原因**：一座城曾经可以同时有两个写者。第二个进程对同一座城执行 `serve` 时，写者线程在 bind 之前就已经打开；bind 失败后它照常收口，往账本里写了一行「人主动关城」的 handoff。那是没有人做过的事，而且是一次分叉：两个进程各自用同一个 `prev` 写了同一个 seq。`sprawling is running.` 这行横幅则在这一切之前就印了出来。

**次序**，`assembly::listen` 是它唯一的定义：
1. 判定绑定面，然后 bind（`wire::bind`，wire-SPEC §8-46）。端口被占、或者暴露的地址没有令牌，都在这里拒绝，账本一字未动。
2. 建 CAS 目录，开账本、从两份快照一遍读起视图与 Standing（8-122）。
3. 开写者线程：`RunWorker::new` → `JsonlLedger::open` 先取写者锁（storage-SPEC §8-1），再做断尾恢复，再 `open_for_service`。锁被别的进程持着，就是 `E_LEDGER_HELD`；已经绑定的监听器随之放掉，账本一字未动。
4. 返回 `Listening`。到这一步，一次 serve 能做的拒绝都已经做完。

然后调用方印横幅、打开浏览器，再调用 `Listening::serve` 开始应答。

```rust
pub async fn listen(serving: Serving) -> Result<Listening, AxError>;
#[must_use] pub struct Listening { /* Bound、ServeConfig、命令台、应答函数、写者线程、控制台 —— 私有 */ }
impl Listening {
    /// 应答，直到人停城；返回前 join 写者线程。
    pub async fn serve(self) -> Result<(), AxError>;
}
```

- **横幅排在后面，是类型排的**：`serve_city` 只有拿到 `Listening` 才走得到印横幅的那一段（`main::city::print_banner`），`listen` 失败就报错退出。所以「running」只在监听已经开始、写者已经持锁之后出现。日志级别那一行和「这个终端就是控制台」两行跟在横幅后面；控制台本身在 `Listening::serve` 里才启动，它的输出因此也排在横幅之后。
- **bind 在取锁之前**：锁在 `JsonlLedger::open` 里取，而 `open` 可能做断尾恢复、写一行 `log_truncated`。要把取锁挪到 bind 之前，就得把取锁从打开里拆出来，给城的写者第二个入口。bind 在前的代价是：同一座城、同一个端口上的第二个 `serve`／`up` 报的是端口被占（`E_CONFIG_INVALID`，recovery 叫人停掉占着端口的进程），而不是 `E_LEDGER_HELD`。两种拒绝都发生在写任何东西之前。
- **`resume`、`fork`、`adopt` 不 bind**，它们直接经 `RunWorker::new` 打开账本，所以城在服务时，它们得到 `E_LEDGER_HELD`。
- **`form_city` 把自己开的写者交给 `RunWorker::over`**，而不是写完头两行之后再用 `RunWorker::new` 开第二个。同一个进程里的两个 `JsonlLedger` 各有一份 seq 与 prev；有了写者锁，这种情形在打开时就被拒。
- **被否：保留单个 `serve(Serving)`，把横幅做成 `Serving` 里的一个回调**。回调在库的异步任务里执行，打印什么、何时打印就由库决定。两步的形状让调用方自己决定监听开始后做什么，库只保证那个时刻已经到了。
- **`Listening` 不 serve 就丢掉，写者线程会一直等到进程结束**，也写不出 handoff；`#[must_use]` 让这种写法在编译时就有警告。
- **重开参数**：如果出现一个在 `listen` 返回之后仍可能失败、失败时又需要收回横幅的步骤，就重新考虑这个切分。

**本章测试**：`assembly::listening::tests::a_serve_refused_at_the_socket_writes_no_line`：测试自己先占住端口，`listen` 返回错误，账本的每一行与之前逐字节相同。锁的那一半由 storage 的 `a_second_writer_of_a_city_is_refused_until_the_first_lets_go` 守住。

### 8-99 视图在自己的线程上折叠并发布快照，写线程与折叠都不等读者（`bin::serving::folding`、`accounting::views::answering`）

```rust
// bin::serving::folding —— shape: adapter
pub(super) struct Folding {
    pub(super) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(super) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(super) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,
    pub(super) thread: std::thread::JoinHandle<()>,
}
pub(crate) struct Copies { pub(crate) published: Arc<Published>, pub(crate) spare: Views }
pub(crate) struct Broadcast { pub(crate) to_clients: broadcast::Sender<wire::Committed>, pub(crate) head: Arc<wire::LedgerHead> }
pub(crate) fn spawn_folding(
    copies: Copies,
    broadcast: Broadcast,
    setting: CorePriority, // 视图线程是否升档（8-93）
    clock: fn() -> Instant, // 服务中切快照的节奏用的钟，由 bin::assembly 交进来（8-91）
) -> Result<Folding, AxError>; // StorageFatal「start the view fold」：线程起不来
// accounting::views::answering
pub(crate) struct Published { /* 私有：Mutex<Arc<Views>> */ }
impl Published {
    pub(crate) fn new(views: Views) -> Published;
    pub(crate) fn snapshot(&self) -> Arc<Views>;                    // 读者拿的快照
    pub(crate) fn replace(&self, latest: Arc<Views>) -> Arc<Views>; // 发布新的一份，交回被换下的那份
}
// accounting::worker::folds
impl Views { pub(crate) fn twin(&self) -> Result<Views, AxError>; } // 克隆出第二份，共用账本索引与计划缓存（8-144）
```

**写线程只做一次 `send`。** 观察者（`observer`）、机器体检的落点（`machine`）与借出金库的 `lend` 都只把一份 `Fold`（一条已提交的记录、一份 `DoctorAnswer`，或工作线程打开的金库）放进无界 `mpsc` 通道，然后返回。名为 `sprawling-views` 的线程每次把通道里已到的全部取出为一批，按到达顺序折叠。被拒：观察者在写线程上直接锁视图——读者持锁多久，写线程的下一次落盘就晚多久。

**两份视图轮换，读者拿快照。** 折叠线程持有备用的一份 `Views`，`Published` 持有发布出去的那份 `Arc<Views>`。每一批：折进备用份，用 `replace` 把它发布，广播这批记录，再收回换下来的那份（`Arc::try_unwrap`；读者还拿着就让出时间片再试），把同一批折进去，它成为下一批的备用份。读者用 `snapshot` 拷一只 `Arc`，锁只罩住这一次指针拷贝；`answer_outside_the_lock` 在快照上 `prepare`，放下快照再 `finish`。于是读者之间不互等，读者不等折叠，折叠在广播之前不等任何读者，收回时只等在换下之前拿到快照、还在做纯内存 `prepare` 的读者。`prepare` 因此只做纯内存的事：连时间也要花在读盘、git 或网络上的查询在快照里只拷走它要的小数据，`finish` 在快照放下后才去读；`McpHealth` 的握手（每台服务器最多等 `HANDSHAKE_PATIENCE`）与 `Toolkits` 的中介书架都是这样，拷走的是 `LiveAsk`（城根、城地址、保管人的 `Arc`）。在快照里握手会让收回一直自旋到握手超时，其间没有一批能折叠或广播。第二份由 `Views::twin` 在 `start_served_views` 折完之后克隆出来（8-144），不再读一遍历史；两份共用同一只账本索引与计划缓存的 `Arc`，所以多出的内存只是折叠本身的一份。计划缓存在快照里按它自己编码，锁不进快照。被拒：每批克隆一份整的视图发布——每条记录一次整份拷贝，40 万行城上一份 22 ms；`RwLock<Views>`——折叠的写锁要等每个在读的读者，新读者又排在写锁后面。代价是折叠常驻两份，每条记录折两次。

**积压的读数是一件仪表。** `serving::folding::tests::instruments` 的 `instrument_view_backlog`（`#[ignore]`，`just bench` 按名字跑它，与 8-84 的两件同一个过滤器）起一个真实的 `spawn_folding`：一条线程经 `observer` 连续送入 2,000 条记录，一条读者线程在其间反复拿快照做一次 `answer_outside_the_lock`，测试线程从 `to_clients` 收广播。读数一行：每条记录从 `send` 到广播收到的时延（`samples`、`floor_us`、`p50_us`、`p99_us`、`max_us`），突发中已送出未广播的最大条数（`max_backlog`），整次突发用时（`burst_ms`），行尾带 `machine=<os>-<arch>, <n> core(s)`，与 8-84 的仪表同一形。它量的是视图线程在突发下落后多少，不加断言：墙钟因机器而异。**被否：在视图线程里写诊断或加原子计数。** 前者在服务中的城上每批一行、淹没别的日志；后者要一个线上字段才有读者，而线格式归一批一进的纪元。

**广播排在发布之后。** 客户端收到一条记录再去查询，拿到的快照已经含有这条记录。

**没有读者能毒化的视图锁。** 读者只读不可变的快照，恐慌不会撕坏它。`Published` 的锁里只有一次 `Arc` 拷贝或交换，没有会恐慌的操作，换下来的那份在锁外交回；即便锁中毒，锁里仍是一只完整的 `Arc`，所以照取不误（`PoisonError::into_inner`）。视图拒绝折叠的记录照旧写一条诊断到标准错误后跳过；折叠线程自己恐慌则线程结束，观察者此后每次 `send` 失败都说出这条记录到不了视图，恢复办法是重启服务，视图从 Ledger 重建。

**金库经通道借给两份视图。** 金库在工作线程里打开，`spawn_worker` 收到它之后调用 `lend`，两份视图各拿到同一只句柄；在这之前的快照里没有金库，要用凭证的读法按「借不到」作答。

**线程随写线程结束。** `attend` 返回后写线程丢掉 `RunWorker`（连同观察者与 `machine`），通道随之关闭，折叠线程把通道里剩下的折完、发布、广播完再退出；写线程 join 它之后才结束，所以 `serve` 对写线程的 join 也等到了最后一次广播。

### 8-100 做 I/O 的查询只在锁内取小数据，I/O 在锁外做（`accounting::views::answering`、`accounting::views::prepared`）

```rust
// accounting::views::prepared —— shape: projection
pub(crate) enum Prepared {
    Held(wire::Answer),          // 视图自己答完了
    GitStatus(GitStatusAsk),         // 锁内取了楼的地址与最近一次检查点，锁外读工作树
    Preferences,                     // 锁外读这个人的设置文件
    Config { city_root: PathBuf, addr: Address }, // 锁外读配置阶梯
    Listing { city_root: PathBuf, at: Option<Address> }, // 锁外列一层目录
    Document { city_root: PathBuf, at: Address },       // 锁外读一个文件的开头
    City(CityAsk),                   // 锁内取 run 摘要、停工范围与各追求，锁外列楼、读计划
    Building { city_root: PathBuf, addr: Address, plans: Arc<Mutex<PlanView>> }, // 锁外读楼的目录与它的计划
    Prefix(PrefixAsk),               // 锁内取折叠记下的那条记录的序号，锁外读账本那一行与内容仓库
    Changes { city_root: PathBuf, base: GitOid, head: Option<GitOid> }, // 锁外让 git 比较两个检查点
    Hunks { city_root: PathBuf, oid_a: GitOid, oid_b: GitOid, path: String }, // 锁外读一个文件的补丁
    Content { city_root: PathBuf, locator: Locator }, // 锁外读内容仓库里的一个对象
    Archives { city_root: PathBuf, needle: String }, // 锁外逐楼读档案架
    Skills { city_root: PathBuf, building: Address, pins: SkillPins }, // 锁内拷出钉住表，锁外扫书架
    Metrics { city_root: PathBuf, held: wire::MetricsAnswer },   // 锁内填好折叠里的数，锁外数楼
    Release,                         // 锁外经网络问发布页
    History { ledger: LedgerAsk, before: Option<Seq>, limit: u32 },      // 锁外刷新索引、读一段历史
    HistoryRange { ledger: LedgerAsk, from: Seq, to: Seq, limit: u32 },  // 锁外读一个区间
    RunHistory { ledger: LedgerAsk, run: RunId, before: Option<Seq>, limit: u32 }, // 锁外读一个 run 的记录
    Rounds { ledger: LedgerAsk, run: RunId },                            // 锁外读一个 run 的记录再折成回合
    Evidence { ledger: LedgerAsk, run: RunId },                          // 锁外读一个 run 留下的定位符
}
// accounting::views::answering
impl Views { pub(crate) fn prepare(&self, query: &wire::Query) -> Prepared; } // 只读视图
impl Prepared { pub(crate) fn finish(self) -> wire::Answer; } // accounting::views::prepared
pub(crate) fn answer_outside_the_lock(
    views: &Published,
    query: &wire::Query,
) -> wire::Answer; // 在快照上 prepare，放下快照再 finish（8-99）
```

**I/O 在放下快照之后。** `answer_outside_the_lock` 取一份快照（8-99），`prepare` 把查询要的小数据（地址、检查点那一行、城根路径）拷出来，放下快照，再由 `finish` 做磁盘、git 或网络的 I/O。控制面（socket 与终端）的查询都经过它。被拒：在锁内跑 `git status`——变更页开着时每次刷新都持锁几十毫秒，折叠线程等它，run 的相邻事件到达客户端的间隔就被这个页面拉长。

**变体是穷尽的枚举，而不是一个 `Box<dyn FnOnce>`。** 每种锁外的 I/O 有名字，`match` 列全，新加一种要在这里写出它锁内拿什么；闭包会把这件事藏进调用点。

**`Prepared` 与它的 `finish` 自成一个模块。** `answering` 管「一个查询在锁内拿什么」，`prepared` 管「锁外怎样把它读完」；两者分开，锁外新添一种读法时不必动锁内那张表。

**`Skills` 在锁内拷走整张钉住表。** 表的大小是「run 数 × 每个 run 钉住的技能数」，拷它是纯内存的一段；按这栋楼的书架先筛再拷，就得在锁内扫书架，正是要挪出去的那次读盘。

```rust
// bin::plan_view
pub(crate) fn plans_of(
    shared: &Mutex<PlanView>,
    city_root: &Path,
    addrs: BTreeSet<Address>,
) -> BTreeMap<Address, PlanReading>; // 锁内描述缓存里有的；放锁读没读过的表；再锁一次放回代数没动过的读数
// accounting::views::city —— shape: projection
pub(crate) struct CityAsk { city_root: PathBuf, plans: Arc<Mutex<PlanView>>, held: wire::CityAnswer, pursuits: Vec<(Address, String, PursuitState)>, in_flight: u32 }
impl CityAsk { pub(super) fn read(self) -> wire::Answer; } // 锁外列楼的目录、读计划、算每个追求的判词
// accounting::views::prefix
pub(crate) struct PrefixAsk { ledger: LedgerAsk, run: RunId, first: Option<Seq> }
// accounting::views::prepared
pub(crate) struct LedgerAsk { city_root: PathBuf, index: Arc<Mutex<storage::LedgerIndex>> } // 历史、回合与证据带出快照的那一份账本
impl PrefixAsk { pub(super) fn read(self) -> wire::Answer; } // 读不到那一行或内容仓库打不开：Unavailable
```

**账本索引有自己的锁。** 索引是账本文件的缓存，折叠从不碰它（它在查询时才刷新），所以它不属于视图的锁：`Views` 持 `Arc<Mutex<storage::LedgerIndex>>`，`Prefix` 把这只 `Arc` 带出视图的锁，在 `finish` 里锁索引、刷新、读一行。读者之间仍为索引互等，折叠线程不等。取索引只有一道门，`LedgerAsk::indexed`：锁、刷新，交出持锁的索引。索引的锁中毒时，这道门把锁里的索引换成一份空索引、解除中毒，再照常刷新；空索引的刷新就是一次整本扫描，所以中毒只让下一个读者多扫一遍，之后回到增量刷新。中毒的那份不能照读，因为半刷新的偏移表会把别的行当成要找的那一行。刷新失败时读者照旧按读不到作答（`Prefix` 答 `Unavailable`，历史答空页）。被拒：中毒后一律按刷新失败作答——中毒在进程里不会自己消失，此后每个历史、轮次与 `Prefix` 问题都悄悄答空；被拒：`finish` 里每次另建一份索引——每问一次就是一次整本扫描（五万条约 14 ms），比锁内那一段还长。

**第一条 `prompt_assembled` 的序号由折叠记下。** `apply` 为每个 run 记它第一条 `prompt_assembled` 的 `Seq`（每个 run 一个数），`finish` 只读这一行；原先是锁内倒着读这个 run 的每一行找它。整条记录不进折叠：四段的来源表随文档数增长，而每个会话多占的内存是这个进程要压低的量。

**计划缓存有自己的锁，回填看代数。** `Views` 持 `Arc<Mutex<PlanView>>`：折叠在 `apply` 里锁它作废读数，读者只在锁内做纯内存的描述，读表在锁外。`PlanView` 为每栋楼记一个代数，任何可能动到这栋楼计划的记录（`apply` 作废它的那一刻）都让代数加一，不带地址的这类记录让全城的代数加一。每楼的代数最多记 1024 栋（`GENERATIONS_HELD`）：再来一栋新楼时全部清掉、全城代数加一，于是在途的回填各被拒一次，而不是让这张表随进程见过的每栋楼增长；只清每楼的数而不动全城的数是错的，因为清零后那栋楼再动一次，它的数又回到读者问时的值。`plans_of` 在第一次锁内给每栋没读过的楼拷出停因表与当时的代数，锁外读表，第二次锁内只放回代数仍相等的读数：两次锁之间折进来的记录可能刚让这份读数过时，放回去就会让缓存一直报旧计划，直到下一条动它的记录。被拒：读到的一律不回填——每次问都要重读每栋楼的表，这正是缓存要省掉的读盘。计划缓存的锁中毒时，下一个持锁者（读者或折叠）用 `PlanView::take_back` 把它收回：丢掉读数与代数（恐慌可能撕了它们，于是每栋楼重读一次表，在途的回填因全城代数加一而被拒），保留停因表（每条停因由一次插入整条写入），再解除中毒。被拒：中毒后弃用缓存——那样每个红节点的句子都悄悄退回表格上的状态词，没人知道为什么；而答 `StorageFatal` 在这里代价过高，因为缓存里没有一样东西是恐慌能弄假而重读修不回来的。

**`CityView` 与 `BuildingView` 都在锁外读盘。** `prepare` 只拷 run 摘要、停工的范围、各追求（地址、目标、状态）与在飞的 run 数，`CityAsk::read` 在锁外列楼的目录，用 `plans_of` 取这些楼与各追求所在地址的计划，再算楼的进度与每个追求的判词。`BuildingView` 带出计划缓存的 `Arc`，在 `finish` 里读楼的目录并用同一个 `plans_of` 取它的计划。

**历史、回合与证据只带出账本。** `History`、`HistoryRange`、`RunHistory`、`Rounds`、`Evidence` 不读折叠里的任何东西，`prepare` 只拷城根并克隆索引的 `Arc`（`LedgerAsk`），刷新索引、读行、折成回合或挑出定位符都在 `finish` 里做。于是在锁内作答的查询都只读折叠，不碰盘：`Commit`、`Commits` 也在此列。

### 8-90 服务中的城在后台证明整条链，证明之前与链断之后写者都不写（`bin::assembly::chain_watch`）

```rust
// accounting::worker::chain_halt —— shape: adapter，随 worker 住（它读写者的私有字段）
pub struct ChainUnderAudit {
    pub halt: storage::ChainHalt,           // awaiting_proof()：判定之前拒绝追加
    pub ledger_dir: PathBuf,
    pub at: Seq,
    pub records: storage::ProofRecords,     // 这个写者持着锁，所以能写
}
impl RunWorker {
    pub fn chain_under_audit(&mut self) -> ChainUnderAudit; // 把等证明的停机值接到写者上，交回证明要的四样
    pub fn await_proof(&self);                              // 关城之前等证明有结局
}

// bin::assembly::chain_watch —— shape: adapter，留在装配根（它起线程）
pub(super) fn audit_in_background(
    watch: ChainUnderAudit,
    log: runtime::diagnostics::Diagnostics,
    began: Instant,                          // 开城的起点，就绪时刻从它量起（8-122）
) -> Result<std::thread::JoinHandle<()>, AxError>; // StorageFatal「start the chain audit」：线程起不来
```

**两步，先接上停机再起线程。** `chain_under_audit` 新建一个 `storage::ChainHalt::awaiting_proof()`，经 `JsonlLedger::halt_on` 接到这个 worker 的写者上，交回停机值、账本目录、写者此刻的位置与能写的记录（`JsonlLedger::proof_records`，记录目录由 `accounting::views::snapshot::start::proof_dir` 给出）；`audit_in_background` 再在名为 `sprawling-chain-audit` 的线程上跑 `storage::prove_chain`（storage-SPEC 8-30）。前一步读写者的私有字段，所以随 worker 住；后一步起线程，所以留在装配根（accounting-SPEC.md §12-17）。线程只持有账本目录、停机值、记录和自己的 `Diagnostics`，不碰写者，所以写线程从不等证明。`serve` 的写线程在 `open_for_service` 之后依次调用这两步；起不来的线程与起不来的写线程一样，让 `serve` 失败。citysim 与测试不走这条路，所以它们的时序里没有第二个线程。

**证明之前写者不写。** 停机值在判定之前拒绝每一次追加，答 `E_HISTORY_UNPROVEN`：页面上发来的命令照样被服务，人收到的是这个码与它的恢复办法，账本一行不多（`crates/accounting/spec/Worker/Attend.lean` 的 `nothing_is_written_before_the_proof`）。`open_for_service` 写的开城那几行在停机值接上之前写下，它们是城自己的开门，不是按历史作出的判断。关城时写者先 `await_proof`，交接那一行在证明有了结局之后写：完好就照常写下，断链就被拒并说出断链的原因。证明线程若没有给出判定就结束（恐慌），它持有的守卫在退栈时以「证明没有走完」跳闸，关城不会永远等下去。

**结果作为诊断推给页面。** 证明线程的 `Diagnostics` 与写者的那一份同一个落点（`serving::Journal` 的 sink）、同一个级别下限，所以页面在日志里读到这一行：`Whole` 先 `prove`，再写一条 `Effect`：

`the history is proved: <n> lines, <k> segment(s) by digest, <c> lines checked, <r> bytes read, <h> bytes hashed, in <ms> ms, <m> ms after opening began; commands are taken`

毫秒由 `opening_cost::millis` 渲染（8-121），前一个时长是线程开头与结尾两次 `serving::standing::monotonic_now` 之差，后一个从 `listen` 交进来的开城起点量起（就绪时刻 M3，8-122）。记录写不成（`Proven.unkept`）再写一条 `Refuse`：下一次开城这些段逐行核对，结果相同，只慢一些。`Broken(reason)` 先 `trip(reason)`，再写一条 `Refuse`，内容就是原因与恢复办法；读账本本身失败（`StorageError`）同样先 `trip`，原因是那次读取的失败：没读完的证明没有证明链断了，也没有证明它完好，而视图与 Standing 从快照起步（8-122）时，快照之前的行只有这次证明会看；写在一条未经证明的链后面的行，与写在断链后面的行一样收不回来。

**视图不需要第二个停机值。** 视图只折写者已经写下的记录（8-99），写者停了，视图也就不再有新工作；给视图再接一个 `ChainHalt` 会让「这座城还收不收工作」有两处定义。被拒的命令经写者的 `StorageError`（`Unproven` 或 `ChainHalted`）回到页面，说的与诊断是同一件事。

**被拒：证明放在启动路径上同步做完再开端口。** 那是开城前的全链读取；证明的价值在于它不挡首字节。

### 8-91 视图从快照起步：编码、切快照、只折尾部（`accounting::views::snapshot`、`accounting::views::snapshot::start`、`accounting::worker::folds::views_start`）

```rust
// accounting::views::snapshot::start —— shape: projection
pub(crate) fn city_root_of(ledger_dir: &Path) -> &Path;   // 账本目录上两级
pub(crate) trait SnapshotFold: Sized {
    const DIR: &'static str;                  // <city>/.sprawling/snapshot/ 下这个折叠自己的目录
    fn fold_version() -> u32;
    fn empty(city_root: &Path) -> Self;
    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Self, AxError>;
    fn encode(&self) -> Result<Vec<u8>, AxError>;
    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>;
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError>;
}
pub(crate) struct Started<F> { pub(crate) folded: F, pub(crate) from: FoldStart, /* 最后一行：切快照用 */ }
pub(crate) enum FoldStart { Resumed { tail: usize }, Whole(storage::WholeFold), Alongside } // Alongside 见 8-122
pub(crate) fn start<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError>;
pub(crate) fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError>; // 先 storage::prove_chain（只读记录），Broken(reason) 原样返回
pub(crate) fn cut<F: SnapshotFold>(ledger_dir: &Path, started: &Started<F>) -> Result<(), AxError>;
pub(crate) fn cut_at<F: SnapshotFold>(ledger_dir: &Path, folded: &F, last: Option<&(Seq, Vec<u8>)>) -> Result<(), AxError>;
pub(crate) fn last_line(index: &LedgerIndex, ledger_dir: &Path) -> Result<Option<(Seq, Vec<u8>)>, AxError>;

// accounting::views::snapshot —— shape: projection
impl Views {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, AxError>;                         // StorageFatal「encode the views」
    pub(crate) fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError>;  // CasCorrupt「decode the views」
}
pub(crate) fn views_fold_version() -> u32;
impl SnapshotFold for Views { const DIR: &'static str = "views"; /* … */ }
impl Views { pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>; } // start_audited::<Views>，不切快照
impl Views { pub(crate) fn cut_snapshot_at(&self, record: &EventRecord) -> Result<(), AxError>; } // 在已折的最后一条记录处切，行是它的 canonical_line

// accounting::worker::folds::views_start —— shape: projection
pub(crate) fn start_served_views(ledger_dir: &Path, log: &mut Diagnostics, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // fold_city 的错误原样返回；切快照的失败只进 log；cost 见 8-121
```

**起步路径放在视图一侧。** `snapshot::start` 由 `views` 持有：一次性查询经 `Views::rebuild` 从这里起步，而 `views` 的生产代码不点名 `crate::assembly`（8-92 的方向块）；`Standing` 的起步（`accounting::worker::folds::standing_start`）从 assembly 指向 views，方向与 assembly 取用 `views::Governance` 相同。**被否：留在 `accounting::worker::folds`，由 `assembly` 提供 `Views::rebuild`。** 那让读面为了一次性查询重新点名组装点，正是方向块拒绝的边。

**编码由各类型自己的 crate 给出。** `storage::HotView`、`storage::Attribution`、`gateway::EndpointBook` 与本 crate 的 `Governance`、`PlanView`、`CommitFacts` 各在定义处派生 `serde::Serialize`／`Deserialize`，`Views` 本身也派生，字节格式是 postcard。`city_root`、`index`（side cache 的 seq→偏移表）、`machine` 与 `vault` 不进快照：前两个由 `Views::new` 从盘上重建（`decode` 用结构更新语法取 `Views::new` 的这两个值，所以重建规则只有一处），后两个本来就不是从账本折出来的。`PlanView` 只存 `causes`，已解析的计划是缓存，下一次提问时重新读。**被否：JSON。** `skill_pins` 以 `(String, B3Hash)` 为键，JSON 的键只能是字符串；而且 JSON 读写都比 postcard 慢、体积更大，这些都记在首字节上。**被否：手写逐字段编码。** 那是每个字段的第二份拼写，加一个字段就要改两处。

**`fold_version` 不靠人记得改。** `views_fold_version()` 取 blake3(`CARGO_PKG_VERSION` ‖ `VIEWS_FOLD_RULES`) 的前四字节（LE）：换一个版本的二进制就丢弃旧快照、从创世折一次；同一版本内改了折叠规则或 `Views` 的字段，改 `VIEWS_FOLD_RULES` 这个常量。这条规则由 `views::snapshot::tests` 机器核对：常量写成 `views-fold-<16 位十六进制>`，后缀是一份固定夹具（三条手写记录，分别填进 Inbox、弃置箱与登记册；时间与序号都是常数，不经过时钟）折出的 `Views` 的 postcard 编码的 blake3 前缀；编码一变，测试给出新的常量值并失败，所以常量不可能停在旧编码上。夹具里一直为空的字段只在增删时改变字节，换了类型不会。**被否：只钉常量，或另钉一个哈希。** 前者什么也没核对；后者改字段时只需改哈希，常量照旧，旧快照照样被接受。字节解不开（postcard 报错）同样按 `WholeFold::Damaged` 退回全量折叠。

**两个折叠，一条起步路径。** 视图与 `Standing`（8-101）各有一份快照，放在 `<city>/.sprawling/snapshot/` 下各自的目录（`views/`、`standing/`）里，各有自己的 `fold_version`，因为两者的编码各自改变；核对快照、折尾部、退回全量折叠、切快照只在 `snapshot::start` 写一次，由 `SnapshotFold` 接两种折叠。**被否：两个折叠共用一份快照。** 那让只改了一边编码的构建丢掉两边的快照，而且 `Standing` 在每个 worker 打开时就切，视图在 `serve` 起步时和服务中的折叠线程上切，两者的最后一行并不总相同。

**起步只折尾部。** `start::<Views>` 调 `storage::start_from_snapshot(ledger_dir, <city>/.sprawling/snapshot/views/, views_fold_version())`（storage-SPEC 8-28）。`Resume`：解码快照里的 views，再用 `ChainSnapshot::resume()` 给出的 `LineCheck` 逐行核对并折尾部——尾部仍然过同一个逐行检查，行号从 `seq + 1` 数起。`Whole`：与没有快照时完全相同，`runtime::replay::fold_ledger_dir` 从创世一段一段地核对并折叠；`Whole` 只带原因，不带行（storage-SPEC 8-28）。快照之前的行在起步时不再逐行核对：切快照时它们已经过一次完整的核对（从创世或从上一份快照起），`fit` 用一行的链哈希证明它们还是那些行；之后被改坏的行在服务中的城里由后台的证明（8-90）抓住；一次性的查询（`views::ask` 经 `Views::rebuild`）旁边没有后台证明，所以 `Views::rebuild` 经 `start_audited::<Views>` 起步：先同步跑一次 `storage::prove_chain`，带着只读的已验证前缀记录（storage-SPEC 8-30：记录命中的段只读、只哈希，其余逐行核对，从不写记录），只有它返回 `Whole` 才从快照起步作答，`Broken(reason)` 与读不了账本都原样拒绝。**被否：一次性查询从创世全量折叠。** 证明逐行核对的只是没有记录覆盖的行，内存是一段字节；全量折叠要解析并折叠每一行，建出整份视图。**快照校验失败绝不信任它**：任何一种 `WholeFold` 都走全量折叠，原因放在 `from` 里；`serve` 把两份折叠各自的 `from` 写成一条 `Effect` 诊断（`the views <起步>; the standing <起步>`），说明这次从哪里起步、为什么。

**切快照：服务起步时，折叠越过快照就切一次。** `serve` 调 `start_served_views`：它先经 `fold_city` 让视图与 worker 的 `Standing` 从各自的快照一遍读起（8-122），再在视图折过的最后一行切一份视图快照（`snapshot::start::cut_at`），最后交出视图；这一遍没有折到任何新行时什么也不写。这个频率不含常数：切一次的代价是一次编码加一次 `sync`，与视图大小成正比；它省下的是下一次起步重折这段尾部的时间，与尾部长度成正比，而尾部只在上次起步之后增长。一次性的查询（`views::ask`）经 `Views::rebuild` 只读快照，不切：读命令不写盘。**写不下快照不让 `serve` 失败**：失败写成一条 `Refuse` 诊断，内容是失败原因与恢复办法，视图照常交出。快照只是下一次起步的捷径：没切成，下一次起步从旧快照或从创世多折一段，结果逐字节相同，只慢一些；而账本写不下时历史本身就缺了，两者不能同样对待。**被否：写不下快照就让 `serve` 失败。** 那让一个只影响下次起步速度的故障（快照目录满、权限错）挡住整座城。

**切快照：服务中，按测得的折叠成本。** 折叠线程（`serving::folding`）每批把记录折进两份视图之后，在这一批最后一条已提交记录处用备用份切一份视图快照（`Views::cut_snapshot_at`）：那条记录的 `canonical_line` 就是账本里的那行字节，所以切快照不再读账本，而备用份此刻没有读者。节奏由 `folding` 里的 `Cadence` 定：它累计自上次切快照以来折叠已提交记录所花的时间（下一次起步要重折的尾部，大致就是这么多时间），累计到上一次切快照所花时间的 `CUT_SHARE_INVERSE`（= 10）倍时再切；还没量过切快照时，第一批之后就切，并量出它。通道关闭（写线程与另两处落点都已放手）时，自上次切快照以来折过记录就再切一次，于是正常停下的城下一次起步不折尾部。于是切快照占折叠线程的时间不超过十分之一，而下一次起步要重折的尾部不超过十次切快照的时间；两者都从运行这座城的主机上测出的时间推出，没有按某一类机器调的行数或秒数。时间由 `bin::assembly` 以 `clock` 参数交进来，交的是单调时钟唯一的采样点 `serving::standing::monotonic_now`，`folding` 的节奏自己不采样，测试可以交另一只钟。**视图拒折过一条记录之后，本进程不再切**：快照会把被拒的那条当作已折叠，下一次从快照起步就接受了全量折叠会拒绝的历史。切不成（编码失败、写盘失败）只写一行 stderr，视图照常发布：快照只是下一次起步的捷径。**被否：另起一条线程再读一遍账本来切。** 那是每次切快照多读一遍尾部的行，而折叠线程手里已经有这些字节。**被否：固定每 N 条记录切一次。** N 在慢盘上太密、在快盘上太疏，切快照的代价与视图大小成正比，不与记录条数成正比。

**本节接口的当前状态**：切快照（编码与 `sync`）在折叠线程上做，这一批之后的下一批要等它写完；把写盘交给另一条线程、折叠线程只编码，是这一接口余下的一步。页面上看不到这次开城从哪里起步、历史证明到哪一行：两者要成为 `CityAnswer` 的字段（改线格式），随波次 4 的 W6 批进；在那之前它们在日志里（上文与 8-90）。

### 8-101 worker 的 Standing 从快照起步（`accounting::worker::folds::standing_start`）

```rust
// accounting::worker::folds::standing_start —— shape: projection
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct StandingFolds { book, governance, collaboration: CollaborationFold, entrance, origins }
impl SnapshotFold for StandingFolds { const DIR: &'static str = "standing"; /* … */ }
impl StandingFolds { pub(super) fn settle(self, cut: Result<(), AxError>) -> Result<Standing, AxError>; }
pub(in crate::assembly) fn as_json_text<T: Serialize, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error>;
pub(in crate::assembly) fn from_json_text<'de, T: DeserializeOwned, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error>;
```

**快照存的是还没 settle 的折叠。** `Standing` 里的 `Collaboration` 是 `CollaborationFold::settle` 的结果：信号的入队与消费按工作发生的顺序到达，队列要到最后一行之后才能算出；尾部再折进来时还要那些被搁置的信号，所以快照存 `CollaborationFold`，不存 `Collaboration`。五个折叠对一条记录的处理只写在 `StandingFolds::absorb` 一处，从创世与从快照起步都走它。

**postcard 装不下的字段写成 JSON 文本。** `Entrance.refused` 里的 `AxError` 用 `#[serde(flatten)]`，postcard 不支持；`CollaborationFold.enqueued` 里的信号带 `Payload`（JSON 值），postcard 读不回来。前者整张表经 `as_json_text` 写成一段 JSON 文本；后者先经写者自己的 `Signal::enqueued_payload` 变回入队记录的载荷，读回时经它的逆 `Signal::from_payload`，所以信号的形状只有一处定义。`Entrance.carrying` 是本进程正在执行的命令，不是从历史折出来的，不进快照。**被否：给 `AxError` 另写一份不带 flatten 的编码。** 那是这个类型的第二份拼写，线上的 JSON 形状与快照的形状会各自漂移。

**起步之前先证明整条链。** `Standing::fold` 经 `start_audited::<StandingFolds>` 起步：先同步跑一次 `storage::prove_chain`，带着只读的已验证前缀记录（storage-SPEC 8-30），只有它返回 `Whole` 才调 `start`；证明后起步只在 `start_audited` 写一次，一次性查询（8-91）与它共用。`Broken(reason)` 原样返回，worker 打不开，读不了账本同样拒绝。起步本身看不到快照之前的行：`fit` 只核对快照那一行，`JsonlLedger::open` 的尾部扫描只读最后一段；`resume`、`fork`、`adopt` 与一次性命令打开的 worker 旁边没有后台证明（8-90）。于是某个封好的段里一行被改写成另一行，仍然规范、仍接得上前一行，只断了下一行的 `prev`，除了这次证明谁都看不到；有了它，从快照起步绝不接受一条全量折叠会拒绝的链。记录命中的段只读、只哈希，所以每次打开 worker 仍要读一遍整条链的字节，逐行核对的只是记录之后长出的行。服务中的城不走这里：它的 `Standing` 在 `fold_city` 里与视图一遍起步，证明在后台，证明完成之前写者不写（8-90、8-122）。**被否：服务中的 worker 在后台证明完成之前就按历史接受命令。** `Standing` 决定 worker 接不接一条命令，按一段没证明过的历史接受命令，写下的行接在一条可能断了的链后面。

**每次打开 worker 都切。** `Standing::fold` 先 `start::<StandingFolds>`，折过了快照之后的行就切一份新快照，再 settle。切不成不是折叠的错：结果放在 `Standing.cut` 里，`RunWorker::over` 把它写成一条 `Refuse` 诊断，worker 照常打开，理由与 8-91 相同。账本目录不存在时什么也不读、不切。

**`fold_version`**：blake3(`CARGO_PKG_VERSION` ‖ `STANDING_FOLD_RULES`) 的前四字节（LE）；同一版本内改了折叠规则或 `StandingFolds` 的字段，改 `STANDING_FOLD_RULES`。这条规则由 `accounting::worker::folds::standing_start::tests` 机器核对：常量写成 `standing-fold-<16 位十六进制>`，后缀是一份固定夹具（两条手写记录，一条信号入队、一条认领，填进协作折叠的信号队列与计划持有表；时间与序号都是常数）折出的 `StandingFolds` 的 postcard 编码的 blake3 摘要前 16 位，编码一变测试就给出新值。

**本节接口的当前状态**：夹具只填了协作折叠；另外四个折叠（`EndpointBook`、`Governance`、`Entrance`、`SessionOrigins`）在夹具里是空的，摘要只钉住它们空时的编码。其中一个在非空时改了编码（字段顺序不变、含义变了）而忘了改常量，同一版本的二进制仍会接受旧快照；给夹具补上这四个折叠各自的一条记录即可合上。

### 8-121 开一座服务中的城，每一段花了多少（`accounting::worker::opening_cost`，形状：value）

```rust
// accounting::worker::opening_cost —— shape: value
pub(crate) enum Phase {
    Bind,                           // 拿端口
    OpenLedger,                     // 取写者锁、探版本、恢复尾段
    FoldTail { lines: u64, from: TailFrom }, // 从较早的快照切点（或创世）一遍核对并折两份（8-122）
    CutStanding,                    // 切 Standing 快照
    CutViews,                       // 切视图快照
    Twin,                           // 克隆第二份视图（8-99、8-144）
    StartWorker,                    // 起写者线程
}
pub(crate) struct OpeningCost { /* 私有：钟、起点、上一记、各段 (Phase, Duration)、折叠累计 */ }
impl OpeningCost {
    pub(crate) fn begin(clock: fn() -> Instant) -> OpeningCost;   // 钟由 bin::assembly 交进来
    pub(crate) fn lap(&mut self, phase: Phase);                    // 记下上一记到此刻这一段
    pub(crate) fn began(&self) -> Instant;                         // 开城的起点，交给后台证明量 M3（8-122）
    pub(crate) fn line(&self) -> String;                           // 唯一的渲染
}
/// 一段时长的毫秒写法：整数微秒换算出三位小数，不经浮点；超出 u64 微秒的饱和到 u64::MAX。
pub(crate) fn millis(span: Duration) -> String;
// accounting::worker::folds
pub(crate) fn fold_city(ledger_dir: &Path, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>;
// accounting::worker::folds::views_start
pub(crate) fn start_served_views(ledger_dir: &Path, log: &mut Diagnostics, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>;
```

**要什么。** 首字节之前 `listen` 做的每件事各花多少，由产品自己说出来，不靠从外面拿首字节减来减去推算。`listen` 在拿端口之前 `begin(serving::standing::monotonic_now)`，之后每做完一段 `lap` 一次；`fold_city` 与 `start_served_views` 在自己做的那几段后 `lap`。写者线程起好之后，`listen` 在与 `serve` 其余诊断同一个落点、同一个下限上写一条 `Effect`：

`opened the city in <总> ms: bind <a> ms, open the ledger <b> ms, fold <n> lines from the snapshots <c> ms, cut the standing snapshot <d> ms, cut the views snapshot <e> ms, copy the views <g> ms, start the worker <h> ms`（从创世读起时写 `from genesis`）

毫秒由 `millis` 渲染。下限为 `off` 时不写。后台整链审计（8-90）在首字节之后才结束，另写它自己那一行，毫秒同样经 `millis`。

**时间从哪来。** `OpeningCost` 不自己读钟：钟是 `begin` 收下的函数指针，与 `spawn_folding` 的 `clock`（8-99）同一种交法，采样仍只发生在 8-93 那一个单调采样点上。一段的长度是两次单调采样之差，墙钟被人调过也不会让它变成负数。

**决定。**
1. 一行，不是七行。七段是同一次开城的七个部分，读的人要看的是它们之间的比例。被否：每段一条 `Trace`——默认下限看不见它们，打开 `trace` 又会被别的行淹没。
2. 读数是诊断，不是账本记录。开城用了多久是运行这座城的主机在那一刻的事实，不属于城的历史（`docs/logging.md` §2）。被否：写一条账本事件——同一段历史在两台机器上就不再逐字节相同。
3. `line()` 是唯一的渲染，没有读者解析它。要逐段比较的人读 `bench_startup` 留在夹具城旁边的日志（citysim-SPEC 8-5-1），那一行与首字节读数出自同一次开城。
4. 派活不在这里拆段。派活的两次读数已在 `prepare_dispatch` 那一行（`[prepare_dispatch_ms]`），它走城钟、按毫秒；把 `stage_dispatch` 内部拆成微秒级的段，要一个能在 accounting 里取单调时间的端口，那是记账线程长任务那一项自己的工作。

**测试。** `assembly::listening::tests::a_listening_city_says_what_opening_it_cost`：在一座刚 `init` 的城上 `listen`，`log` 取 `Effect` 下限、sink 是同一个 `Journal`，事先订阅 `Journal::lines()`；收到恰好一条以 `opened the city in ` 开头的 `Effect`，七个段名按上面的次序出现，并含 `fold 3 lines from genesis`。

### 8-122 开城从两份快照一遍读起，历史在后台证明，四个就绪时刻（`accounting::worker::folds`、`accounting::views::snapshot::start`，形状 7 投影；`bin::assembly::chain_watch`，形状：状态机）

```rust
// accounting::views::snapshot::start —— shape: projection
pub struct BothStarted<A, B> {
    pub first: Started<A>,
    pub second: Started<B>,
    pub checked: u64,        // 这一遍逐行核对的行数
    pub from: TailFrom,      // 这一遍从哪里读起
}
pub enum TailFrom { Snapshots, Genesis }
pub enum FoldStart { Resumed { tail: usize }, Whole(WholeFold), Alongside } // Alongside：另一份要从创世折，同一遍带上它
pub fn start_both<A: SnapshotFold, B: SnapshotFold>(ledger_dir: &Path) -> Result<BothStarted<A, B>, AxError>;
pub fn proof_dir(city_root: &Path) -> PathBuf;   // <city>/.sprawling/snapshot/verified/，已验证前缀记录的唯一路径
// accounting::worker::folds
pub(crate) fn fold_city(ledger_dir: &Path, now: TimeMs, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // 开账本，再 start_both::<Views, StandingFolds>
```

**要什么。** 开一座城到首字节从 40 万行城的 8.2 s 落到毫秒级（D41）：首字节之前只做与尾部长度、快照大小同阶的工作，整条链的证明不挡首字节，接受命令等证明完成。

**两份快照，一遍读。** 视图与 Standing 各有一份快照（8-91、8-101），切点各不相同。`start_both` 先读两份快照，各自核对 `fold_version` 并解码；两份都能用时，从较早的切点起经 `storage::start_from_snapshot` 取那一段尾部，每一行过同一个逐行检查（`ChainSnapshot::resume` 给出的 `LineCheck`），再只交给切点在它之前的那一份折叠。较晚的那份快照在这一遍里核对自己那一行（它的 `seq` 上那一行的链哈希），核对不上就在这一遍之后从创世再折它一次（`FoldStart::Whole(Stale)`，罕见的一支）。一份快照不能用（没有、坏了、版本不对、账本比它短），这一遍就从创世走，另一份也在同一遍里从创世折（`FoldStart::Alongside`），所以任何情形下首字节之前至多一遍逐行核对。性质是 `crates/storage/spec/Snapshot.lean` 的 `twoCutsOnePass`。**被否：两份各自经 `start` 起步。** 较晚切点之后的那段尾部被读、被核对两次。

**账本索引随视图快照恢复**（storage-SPEC 8-4）：从快照起步的视图带着切快照时的索引，第一次历史查询只 `refresh` 之后长出来的字节。从创世走的那一遍照旧顺手建索引。

**历史在后台证明。** 快照之前的行在首字节之前一个字节也不核对；写者起好之后，`bin::assembly::chain_watch` 的线程用 `storage::prove_chain` 带着已验证前缀记录走整条链（8-90、storage-SPEC 8-30）：记录命中的段只读、只哈希，不解析；记录之后长出的行与没有记录的段逐行核对，然后写新记录。所以一次开城至多一遍逐行核对全量——升级之后第一次开城、或记录被删之后——其余开城只核对上次证明之后长出的行。证明完成之前写者拒绝每一次追加（`E_HISTORY_UNPROVEN`，8-90）；查询不等证明，按从快照起步的视图作答。

**四个就绪时刻。**

| 时刻 | 意思 | 从哪读 | 目标 |
|---|---|---|---|
| M1 页面可见 | `GET /` 有首字节 | 本版与 M2 是同一刻：`wire::serve` 在 `listen` 返回之后才应答 | 毫秒级；见下文「本节接口的当前状态」 |
| M2 首个有效查询 | 两份折叠从快照起步、尾部核对完、视图发布 | `opened the city in …` 那一行的总时长（8-121），其中 `fold <n> lines from the snapshots` 是这一遍 | 毫秒级；读量与尾部、快照同阶 |
| M3 接受命令 | 证明为 `Whole`，停机值放行 | `the history is proved: …` 那一行的后一个时长（8-90） | 由 F1 在 40 万行城、记录命中时的读数定 |
| M4 完整证明 | 每一段都有本构建规则版本下的判定 | 同一行 | 与 M3 是同一刻：接受命令不早于完整证明（8-101 的决定），证明完成的那一刻才放行 |

**首字节之前读什么**（`storage::snapshot::start::tests` 经 `Vfs` 缝计数）：首段的第一行（版本探测，8-1 第 ③ 步）、前一段的最后一行（至多一个 16 KiB 的窗口，8-1 第 ④ 步）、整个末段（尾部恢复逐行核对它，8-1 第 ④ 步）、以及从较早切点所在的段起的各段。更早的段一个字节也不读，所以 M2 的读量与历史长度无关。

**计时。** `OpeningCost`（8-121）的阶段改为 `fold <n> lines from the snapshots` 或 `from genesis`，读的人从同一行看出这次开城有没有从快照起步；`OpeningCost::began` 把开城起点交给证明线程，M3 从同一个起点量起。

**本节接口的当前状态。** M1 与 M2 是同一刻：`wire::serve` 在 `listen` 返回之后才开始应答，把静态页面的应答提前到折叠之前要改 `wire` 的服务入口，归 W 车道。尾部恢复从末段的已验证前缀起只逐行核对之后的字节（8-144，storage-SPEC 8-34）。`SnapshotStart::Resume.tail` 仍把尾部整段读进内存；正常关闭的城尾部为空，崩溃后的尾部是上次切视图快照之后写下的记录。Standing 的快照只在开城时切（8-101），所以 Standing 的切点落后于视图，这一遍从 Standing 的切点读起：按与视图相同的节奏在服务中切 Standing，要先核对写者对 Standing 的增量更新与 `StandingFolds::absorb` 是同一条规则。页面从 `CityAnswer.proved` 读到历史已证明到哪一条（8-134）。

### 8-144 开城的两段与一次性重建：末段按记录证明、第二份视图按值克隆、重建每行只核对一遍（`accounting::worker::folds`、`accounting::views::snapshot`、`accounting::views::snapshot::start`，形状 7 投影；storage-SPEC 8-34，accounting-SPEC.md 8-19）

**原因是一组拆开的读数。** 40 万行夹具城（`bench_startup first-byte` 的 `l400k`，8-122 之后，windows-x86_64、16 核、NVMe、release）开城 1220 ms，其中 `open the ledger` 576 ms、`fold 0 lines from the snapshots` 279 ms、`copy the views` 357 ms。逐段拆开量：
- `open the ledger` 几乎全是尾部恢复逐行核对末段（40,843,081 B）；同一段读一遍 15 ms、BLAKE3 一遍 23 ms。
- `copy the views` 是经快照编码复制第二份视图：编码 214 ms、解码 139 ms（15,858,109 B）；按值克隆同一份 22 ms。
- `fold … from the snapshots` 里，视图快照解码 139 ms（其中提交折叠 105 ms：64,000 个提交，oid 每个按 40 位十六进制读三次）、`storage::tail_after` 读末段并逐字节分行 50 ms、读两份快照 20 ms。

整城重建（`just bench` 的 `large_ledger_fold`，5 万行、没有快照也没有记录）p50 1286 ms：`Views::rebuild` 先经 `prove_chain` 逐行核对整条链，从创世全量折叠时又逐行核对一遍。

**四处改动，保证不变。**
1. `fold_city` 经 `JsonlLedger::open_reusing` 开账本，记录取自 `proof_dir` 的只读一份（storage-SPEC 8-34）：末段的前缀按摘要证明，逐行核对的只有上一次后台证明（8-90）之后写下的行。正常关闭的城，那几行就是关城时写的交接。
2. `Views::twin` 是派生的 `Clone`（accounting-SPEC.md 8-19(a)）：共享的句柄在 `Arc` 里，克隆后仍共享，结果与编码再解码相同。
3. `start_audited` 从创世折叠时不先证明，从快照起步时才先证明（accounting-SPEC.md 8-19(b)）。
4. `storage::tail_after` 从含快照那一行的段尾往回找它，不再从段首把整段切成行（storage-SPEC 8-28）。

**回退门是计数，墙钟只登记。** storage 的 `jsonl::open::tests` 在 N 与 2N 行上断言：记录覆盖末段之后再写 2 行，开账本逐行核对 2 行、按摘要复用 1 段。accounting 的 `worker::folds::views_start::tests` 在两种规模上断言：没有快照与记录时重建逐行核对的行数等于账本行数；快照与记录都在时等于之后长出的行数的两倍（证明一遍、折尾部一遍），与历史长度无关。开城各段与首字节的毫秒数由 `bench_startup first-byte` 与 `opened the city in` 那一行（8-121）读出，进 `tools/xtask/budgets.toml` 的 `[first_byte]` 与 `[views_rebuild_per_mb]`。

**本节接口的当前状态。**
- `copy the views` 仍在开城那一行里（8-121 的 `Phase::Twin`），`twin` 仍答 `Result`：它不再会失败，去掉 `Result` 与这一段要同时改 `bin::assembly::listening` 的调用点。
- 快照里的十六进制摘要与单线程的证明由 8-154 接手。

### 8-154 开城与证明再降一档：快照里的摘要是字节，证明按波读段（`accounting::views::snapshot`、`accounting::worker::folds::standing_start`，形状 7 投影；`bin::assembly::chain_watch`，形状：状态机；kernel-SPEC §8-84，storage-SPEC 8-37，accounting-SPEC.md 8-24）

**原因是 8-144 留下的两处读数。** 40 万行夹具城（`bench_startup first-byte` 的 `l400k`，windows-x86_64、16 核、NVMe、release，三轮各 3 个样本）首字节 p50 296–298 ms；`opened the city in` 约 261 ms，其中 `fold 0 lines from the snapshots` 189–190 ms、`open the ledger` 41–42 ms、`copy the views` 22 ms；`the history is proved` 那一行：6 段全按记录命中、0 行逐行核对、读与哈希各 376,383,853 B，证明 390–395 ms，开城起点之后 651–656 ms 接受命令（M3）。10 万行城（`l100k`）：首字节 p50 127–130 ms，折叠 53 ms，证明 98–99 ms，M3 192–193 ms。

**两处改动。**
1. 视图与 Standing 的快照里，`GitOid` 与 `B3Hash` 写成字节本身（kernel-SPEC §8-84）。两份快照的夹具里都有这两种摘要，`fold_version` 随之进位，升级之后第一次开城从创世折一次、切新快照（accounting-SPEC.md 8-24）。
2. 后台证明按波读段：一波至多 8 段，各段的读与记录前缀的哈希同时做，链仍按段序判（storage-SPEC 8-37）。`chain_watch` 的那一行不变。

**保证不变。** 账本行、线上帧、`golden-p0`／`golden-s1`、wire 的两份 golden 都不变；证明的判定与逐行核对相同（`crates/storage/spec/Snapshot.lean` 的 `wavesAreStrict`）。回退门仍是计数：storage 的 `chain_audit::tests` 在 N 与 2N 段上断言五个计数（逐行核对的行数、按摘要的段数、读与哈希的字节、波数），accounting 的两个 fold-rules 测试钉住快照格式。

## 8-89 一张命令表，一个纯解析器（`bin::main::verbs`、`bin::main::grammar`）

**形状。** `main/verbs.rs` 是 data：一张静态表 `VERBS`，每个动词一行 `Row { verb, name, aliases, positionals, flags, says, effect }`。位置参数是 `(名字, Need::Required | Need::Optional)`；标志是 `Flag { name, takes: Takes::Nothing | Takes::Value(<占位名>), says }`；`effect` 是 `Effect::ReadsOnly | Effect::Changes`，标出这个动词运行时会不会改一座城或它所在的主机。总览（`help`、`--help`）、单个动词的帮助（`help <verb>`、`<verb> --help`）与首屏退出时的清单都由这张表生成，没有第二份手写的命令清单或用法字符串。

`main/grammar.rs` 是 grammar：纯函数，不做 I/O。

```rust
pub(super) fn parse(words: &[String]) -> Result<Invocation, LineError>;
pub(super) enum Invocation { FirstScreen, Overview, Help(Verb), Version, Run(Verb, Arguments) }
impl Arguments {
    pub(super) fn positional(&self, nth: usize) -> Option<&String>; // 从 1 数
    pub(super) fn has(&self, flag: &str) -> bool;
    pub(super) fn value(&self, flag: &str) -> Option<&str>;        // 同一标志给两次，后者胜
}
pub(super) enum LineError {
    UnknownVerb { given, nearest }, UnknownFlag { verb, given, nearest },
    MissingValue { verb, flag }, MissingPositional { verb, name }, ExtraPositional { verb, given },
}
```

**优先规则。** `--version`/`-V` 出现在任何位置都得 `Version`；其次 `--help`/`-h` 出现在任何位置都得那个动词的 `Help`（没有动词时得 `Overview`）。两者都在任何动词运行之前决定，所以 `up --help` 不写创世记录、`install --help` 不改 PATH。`version` 是 `status` 的别名，`enroll` 是 `enrol` 的别名。

**标志与位置参数。** 以 `--` 开头的词（单独的 `-` 除外，它是 `call` 的「从 stdin 读帧」）只按该动词这一行的标志表读；带值的标志吃掉下一个词，所以 `serve <city> --log debug` 的地址取默认值而不是 `debug`。表里没有的标志是 `UnknownFlag`，多出来的位置参数是 `ExtraPositional`，缺了必填的位置参数是 `MissingPositional`。一个词恰好等于这一行某个标志的短名（`dispatch` 的 `-m`）时总读作那个标志，哪怕它本想是位置参数：`sprawling dispatch lab -m` 缺的是 `-m` 的值而不是任务。要把 `-m` 当任务文字交出，把它放进更长的一句里。

**命令行错误。** 每个 `LineError` 在 stderr 上写一行 `sprawling: <哪个参数、错在哪>. Did you mean '<最近的合法写法>'?`，退出码 2，不再倾倒整张表。恢复语就是那条「最近的写法」：`nearest` 取与所敲的词共享最长前缀（至多四个字符）的已知名字，所以 `stauts` 得 `["status"]`。

**本节接口的当前状态。** `call`、`enrol`、`install`、`fork`、`status`、`serve` 的标志值与 `doctor` 的全部参数，仍由各自的函数从原始 argv 读（`router::flag_value`、`args.iter().any`），解析器只替它们校验；改为只读 `Arguments` 之后 `flag_value` 删除。README、README.zh-CN、LLM.md、`docs/operating.md` 里的命令语法块还是手写的，改由 `cargo xtask docnum` 从这张表生成（一个名为 `cli_verbs` 的受管区块）是下一步；`nearest` 与 `console::language` 里找近似动词的那一份是同一条规则的两份，合并到一处也是下一步。

**决定。**

1. 不用 clap。命令表是数据，解析器约两百行；启动时间几乎全是操作系统的开销（Windows x86-64 桌面级机器上，`--version` 首字节 7.98 ms，空进程下限 5.40 ms），没有给一个参数库的依赖、编译时间与体积留出位置。重新考虑的条件：动词需要子动词或 shell 补全以外的、这张表表达不了的结构。命令表的一行可以叫两个词（`playback export`），解析先试头两个词，这是子动词在这张表里的全部写法（8-126）。
2. 不用 `+` 前缀区分动词。现有动词不改名，一个词仍然是一个动词，文档与肌肉记忆都不必迁移。
## 8-102 开城时修过什么，要说给人（`accounting::worker::lifetime`、`accounting::worker::genesis`、`bin::assembly::attending`）

**原因**：`JsonlLedger::open` 在断尾恢复时截掉撑裂的尾行，并返回 `OpenReport`（storage-SPEC §8-1）。账上虽然多了一行 `log_truncated`，但页面不画它，CLI 也不读它；`RunWorker::new` 与 `form_city` 把报告丢掉，人于是不知道上一次进程死时丢了几个字节。

**形状**：值（形状 2）。`RunWorker` 持有 `LedgerOpening`，它是 `OpenReport` 在本 crate 的类型化状态，只由 `LedgerOpening::from(OpenReport)` 生成：

```rust
pub(crate) enum LedgerOpening { Intact, TailDropped { bytes: u64 } }
impl From<storage::OpenReport> for LedgerOpening { … }
impl LedgerOpening {
    /// 给人看的一句：截掉了什么、为什么、怎么恢复；`Intact` 答 `None`。
    pub(crate) fn notice(self) -> Option<String>;
}
impl RunWorker {
    // 第四个参数就是 JsonlLedger::open 返回的那一对：账与它开时修过什么一起到，调用方没法只交一半。
    pub(crate) fn over(&Path, Custodian, Diagnostics, (JsonlLedger, OpenReport)) -> Result<Self, AxError>;
    pub(crate) fn opening(&self) -> LedgerOpening;
}
```

- **到人的两条路**：`sprawling resume` 的 `ScanReport::summary()` 在断尾时多一句 `notice()`；`sprawling serve` 的写者线程在开账之后、`open_for_service` 之前把同一句印到 stderr，先于横幅出现。两处读的都是同一个 `notice()`，措辞只有这一个来源。
- **`form_city` 不丢报告**：它只在目录没有账本时开账，报告因此恒为 `Intact`；它照样把 `open` 返回的那一对原样交给 `over`，让「worker 知道自己的账是怎么开的」对每条构造路径都成立，而不是靠一句注释说这里不会发生。
- **恢复**：截掉的是进程死时没写完的那一行，它之前的每一行都已按链校验。人要做的是确认最后一次动作是否需要重做；要逐字节看原状，就在再次打开之前从备份拷回 `.ledger`。
- **仍未到页面**：页面对 `log_truncated` 什么也不画（`client/src/core/belief.ts` 把它归进不显示的一组）。让城页说出这件事，需要一个视图字段与客户端的一个位置，这是本节接口尚未覆盖的一半。
- **不进公开面**：`ScanReport::summary()` 是跨出 crate 的唯一读法，`LedgerOpening` 因此留在 `pub(crate)`，`apisync` 基线不动。
- **被否：只把 `notice` 写进 `Diagnostics`**。`serve` 默认不开日志，`resume` 用的是 `Diagnostics::off()`，写进去就等于没说。

**本节测试**：`accounting::worker::lifetime::tests::a_torn_tail_is_told_in_the_startup_scan`：写一座城，在账尾追加半行，`RunWorker::new` 后 `startup_scan().summary()` 必须说出截掉的字节数。

## 8-103 退出码是一张表（`bin::main::exit`、`bin::main::calling`、`bin::main::refusal`、`bin::wire_client`）

```rust
pub(super) enum Exit { Done, Refused, Line, Quiet, NoCity }   // 0 1 2 3 4
impl From<Exit> for std::process::ExitCode;
pub(crate) enum Unheard { Unreadable(AxError), NoCity(AxError), Broken(AxError) }
pub(crate) fn call(at: &str, frame: &str, token: Option<&str>, quiet: Duration) -> Result<Heard, Unheard>;
pub(super) enum Form { Human, Json }
pub(super) fn written(err: &AxError, form: Form) -> String;
```

| 码 | `Exit` | 它断言的事实 |
|---|---|---|
| 0 | `Done` | 做完了 |
| 1 | `Refused` | 城或所在机器拒绝了，stderr 上有 `AxError` 的失败行与 recovery 行 |
| 2 | `Line` | 这条命令行读不懂，包括 `call` 的帧不是 wire 能载的东西 |
| 3 | `Quiet` | `call` 与 `dispatch`：帧发出去了，安静窗口内什么都没回来（`Spoken::Quiet`），或城回了话而等的那一帧没来（`Spoken::Unfinished`） |
| 4 | `NoCity` | `--at` 那里没有城在答：连不上，或连上了却没有回握手 |

`Exit` 是退出码的唯一定义；数字只在 `From<Exit> for ExitCode` 里出现一次。`wire_client::call` 用 `Unheard` 说清没听到回答的原因：帧解析失败是 `Unreadable`（映射到 2），连接、发送问候或等 `Welcome` 失败是 `NoCity`（映射到 4），握手之后连接断了或本进程起不了 runtime 是 `Broken`（映射到 1）。三者都照常在 stderr 上印 `AxError` 的失败行与 recovery 行。

拒绝怎么写由 `refusal::written` 一处决定，它只管文字，不管写到哪里；调用方把结果写到 stderr。`Form::Human` 是失败行、`recovery:` 行，`AxError` 带 `nearby` 时再加一行 `nearby: a, b`，给人读的拒绝也指出附近能用的名字。`Form::Json` 是一行 `AxError` 的 serde（与 wire 上 `Refusal` 同形），反序列化回来与原值相等。`call` 带 `--json` 时用 `Form::Json`，其余调用方用 `Form::Human`。

**本节接口的当前状态。** `call` 与 `dispatch` 经 `Exit` 退出；其余动词返回 `ExitCode`，经 `city::report` 得 1。`--json` 目前只有 `call` 接受。还没做的是：其余动词的 `--json`、kernel 的命令行错误码（先改 kernel-SPEC 的 `AxCode` 表）、以及「动词 × 模式（终端、管道、`--json`）→ 退出码与输出流」的整张表。`doctor` 已经只在 stdout 是终端且没有 `NO_COLOR` 时着色（§8-59）。

**决定。**

1. 没有城单列为 4，而不与 1 共用。1 说的是城读了这一帧并拒绝，一个 agent 据此改帧重试；没有城时帧没有被任何城读过，该做的是起城或改 `--at`。两件事共用一个码，就把后者读成了前者。
2. 帧写错退 2 而不是 1。帧在开 socket 之前解析，错在这条命令行本身，与城无关；被否决的备选是沿用 `WireMismatch` 的 1，它让一个拼错的帧和城的真实拒绝无法区分。
3. 握手之后断开归 1 而不是 4。那时已经有城答过 `Welcome`，城在；断开是这次对话的失败，不是地址上没有城。
4. `--json` 的拒绝是 `AxError` 自己的 serde，而不是另起一个命令行专用的 JSON 形状。wire 上的 `Refusal` 已经是这个形状，一个 agent 用同一个反序列化读城的拒绝和命令行的拒绝；另起一种形状，就要在两处维持同一组字段。

## 8-104 `sprawling check <city>`：每份 TOML 一行错，行列可点（`main::check`；city-SPEC §8-29）

只读动词。它把 `city::check` 的每条 Finding 印成一行，写到 stderr：有位置时 `路径:行:列: 码: 消息`，没有位置时 `路径: 码: 消息`；路径是城根下的相对路径，用 `/` 分隔，编辑器与终端都能把它当成跳转目标。码是 `AxCode::as_str`，消息是错误的 subject。

- 退出码：全部读过 → 0；有任一条 Finding → 1；命令行读不懂 → 2（与 §8-89 同一规矩）；城本身读不了（列楼失败、I/O 失败）→ 1，并按一条普通拒绝印出。
- 全部读过时 stdout 印一行 `ok: <n> file(s)`，n 是读到的文件数。


## 8-105 `sprawling view`：给 agent 的一面（`bin::main::view`、`accounting::lineage`）

**形状。** `main/view.rs` 是 adapter：读命令行、开账本索引、把选中的行写到 stdout。`lineage.rs`（库里，`accounting::lineage`）是 projection（ARCHITECTURE §9 形状 7）：把账本折成每个 run 一条 `RunLine`，查看器的 `tree` 透镜（S5.8I）与 WebUI 以后的 run 树读的都是它。`view` 只读，`Effect::ReadsOnly`。

```rust
// bin::main::view
pub(super) fn verb(read: &Arguments) -> ExitCode;
pub(super) struct Selection { tail: Option<usize>, from: Option<Seq>, run: Option<RunId>, kind: Option<EventKind>, who: Option<String>, grep: Option<String>, span: UtcSpan }  // span 见 8-137
pub(super) enum Audience { Agent, Person }  // 谁读 stdout：管道或文件后面的 agent，终端前的人
pub(super) fn write_records(dir: &Path, chosen: &Selection, audience: Audience, out: &mut impl Write) -> Result<(), ViewError>;
enum HashWidth { Whole, Glance }             // 64 位，或前 GLANCE_DIGITS 位
const GLANCE_DIGITS: usize;                  // 12
fn chain_label(line: &[u8], width: HashWidth) -> String; // 十六进制位，后接两个空格
// accounting::lineage
pub struct RunLine { run, addr, session, parent, forked_at, predecessor, first_seq, last_seq, state, unanswered }
pub struct Lineage;                       // fold：apply(&EventRecord) -> Result<(), AxError>
impl Lineage { pub fn lines(&self) -> impl Iterator<Item = RunLine>; }
pub fn lineage_of(ledger_dir: &Path) -> Result<Lineage, AxError>;
```

**`records` 透镜（非终端时的输出）。** 输出账本原行，逐字节相同，每行一个 `\n`，按 seq 升序。条件同时成立才选中：`--from <seq>`（含）、`--run <id>`（走 `LedgerIndex::run_seqs_before`，不读别的 run 的行）、`--kind <k>`（信封的 `kind`）、`--who <addr前缀>`（信封 `addr` 以它开头；没有 `addr` 的行不中）、`--grep <子串>`（原行按字节含这个子串，不是正则，glossary 的搜索规则）。`--tail N` 最后作用：只留选中的最后 N 行，从尾部倒着找，找够就停。`--since <utc>`/`--until <utc>` 按信封的 `t` 选（8-137）。信封只借用解析 `run`、`kind`、`addr`、`t` 四个字段。

**终端前的 `records` 透镜。** 谁读 stdout 只在 `verb` 里判一次（`Audience::of_stdout`）：stdout 是终端就是 `Person`，否则是 `Agent`。`Person` 而命令行一个过滤参数都没带时进 8-117 的交互界面；带了过滤参数，`write_records` 按上一段的规则选行，每个选中的行写成：这一行的链哈希、两个空格、原行、一个 `\n`。链哈希是 `kernel::ledger::chain_hash` 对这一行在盘上的字节（不含行尾 `\n`）算出的 BLAKE3，64 位小写十六进制；链完好时它就是下一行的 `prev`。`view` 不核对链，打出来的只是这行字节的哈希，链断没断由 `sprawling replay` 核对。哈希前缀与两个空格只由 `chain_label` 写出，交互界面的列表调同一个函数，只取前 `GLANCE_DIGITS` 位（8-117）。`Agent` 时的输出与上一段逐字节相同。测试不经过终端，直接把 `Audience` 交给 `write_records`。

**`--runs`（`tree` 透镜给 agent 的画法）。** 每个 run 一行 JSON：`run`、`addr`、`session`、`parent`、`forked_at`、`predecessor`、`first_seq`、`last_seq`、`state`、`unanswered`，按 `first_seq` 升序。`parent` 是 `run_forked.from`，没有分叉记录时是 `run_started.parent`；`forked_at` 是 `run_forked.at_seq`；`predecessor` 是 `run_started.predecessor`；`session` 是这个 run 开始前、同一地址上最近一条 `session_opened` 的 seq（这段 stretch 的名字），没有就是 `null`。`state` 取自 `storage::HotView` 的 `RunPhase`（`active`、`frozen`），与 Views 的 run 列表同一份折叠，不另算。`unanswered` 是这个 run 提出、还没有 `approval_resolved` 答复的 `approval_requested` 条数：请求按它的 `id` 记在提出它的 run 名下，答复按同一个 `id` 销掉，不管答复落在哪个 run 上；两种记录的 payload 都经 kernel 的类型读（`ApprovalItem`、`ApprovalResolved`），读不了就拒，与 `views::governance` 同一条规则（8-74）。父指针都在行里，agent 不需要第二次查询就能拼出树。

**失败。** `--kind` 不是 `EventKind` 的 snake_case 名：stderr 一行 `sprawling: view: no event kind '<k>'. Did you mean '<近似名>'?`，退出 2（近似名用 `grammar::nearest` 那一条规则）。`--run` 不是 RunId、`--from`/`--tail` 不是数：同形，退出 2。账本目录读不了：`AxError` 的正文与 recovery，退出 1。

**决定。**

1. 两个主人按 TTY 分开，不违背 §8-11「拒长表与图」。stdout 不是终端时 `view` 只输出账本原行，与 `call` 同形；表格、树与颜色只在人坐在终端前时才画（S5.8I），控制台仍然只写 JSONL。被否掉的是在 `call` 上加过滤参数：`call` 走 wire，要城在服务；`view` 读盘，城不在服务也能答。
2. 交互界面不给每种事件写说明：一行只画信封字段加压缩后的 `data`，详情画通用 JSON 树，事件种类再多也不加一行。
3. 树为主（D-15）：城 › 楼 › 房间 › 会话 › run › 回合 › 调用，分叉挂在父 run 的分叉点下，每个节点只有一个父；委派、敲门、handback 是详情里的链接，不是树的边。被否掉的：列表加详情为主（人要在交错的行里自己拼出一件活）；fx 式 JSON 树为主（就地展开推走下面的行，也看不出分叉）。
4. 行选择与 run 折叠都消费 `storage::LedgerIndex` 这一个索引，不自己数段文件（storage-SPEC §8：`storage` 不对外暴露段）。

## 8-126 `sprawling playback export` 与 `sprawling playback check`（`bin::main::playback`；accounting-SPEC.md 8-12）

**形状。** `main/playback.rs` 是 adapter：读命令行，把人的入口交给 `accounting::playback` 的共享投影，把字节写到 stdout 或一个新文件，把复核结果写成一行 JSON。投影、选择、读界、规范字节与复核都在 accounting-SPEC.md 8-12，本节不重述。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--include-confidential] [--out <file>]
sprawling playback check <bundle> [--bundle <file>] [--city <city>] [--include-confidential]
```

```rust
// bin::main::playback
pub(super) fn export(read: &Arguments) -> ExitCode;
pub(super) fn check(read: &Arguments) -> ExitCode;
```

- **两个词是一个动词。** 命令表的一行可以叫 `playback export`：`grammar::parse` 先看头两个词能否拼成某一行的名字，再看头一个词；只敲 `playback` 或 `playback <错词>` 是 `UnknownVerb`，近似名列出 `playback export`、`playback check`。帮助、总览与解析读的仍是同一张表（8-89）。
- **选择。** `--from`/`--through` 是含端点的 seq，与 `view --from` 同一种读法（`view::seq_flag`）；`--run` 与 `view --run` 同一种读法（`view::run_flag`）；`--building` 经 `Address::parse`，按 `Address::is_within` 选楼，不是 `view --who` 的字符串前缀。三者交给 `Selection::new`，矛盾的区间由它拒绝。
- **读者是人。** 入口交 `Reader::Person(Confidential::Withheld)`；带 `--include-confidential` 交 `Confidential::Included`，并在 stderr 写一行 `sprawling: playback: this bundle includes confidential buildings`，bundle 的 `source.reader` 写成 `{"person":"included"}`。`check --city` 用同一个标志决定复核时的读者，不读 bundle 自述。
- **输出。** 不给 `--out` 时，bundle 完整、尺寸检查通过之后才开始写 stdout；写到一半管道关闭是失败（退出 1，stderr 说明 bundle 不完整），不沿用 `view | head` 的成功语义。给 `--out` 时经 `accounting::playback::land(Place::Chosen)` 写（accounting-SPEC.md 8-13）：同一目录先写一个 `<file>.partial-<pid>`，写完再以硬链接落到目标名、删掉暂存文件；目标已存在、或被 git 跟踪，则拒绝，不覆盖；目标的父目录经 `std::fs::canonicalize` 解开链接之后，路径里任一段是受保护的元数据（`kernel::PROTECTED_METADATA`：`.sprawling`、`.git`，不分大小写）则拒绝，所以账本与 git 元数据写不进去。任何失败都删掉暂存文件，不留可被误认的最终文件。
- **复核。** `check` 的文件、标志、五项与它的退出码见 8-132。
- **退出码**（8-103 的表）：0 导出完成；1 拒绝（账本坏、bundle 超限、目标已存在、被跟踪或在受保护的元数据里、stdout 中途关闭）；2 命令行读不懂。

**本节测试**：`accounting::playback::tests::landing`：`--out` 的目标整份落下、不留暂存文件；已存在的目标被拒且原文件不变；指进 `.sprawling` 或 `.GIT` 被拒且没有留下文件；被 git 跟踪而已从盘上删掉的文件名被拒。`main::playback::tests`：矛盾的区间与读不了的楼、读不了的 seq 分成两种拒绝。`grammar::tests::a_verb_of_two_words_is_read_from_two_words`：`playback export` 从两个词读成一行，`help playback check` 是那一行的帮助，`playback` 单独与 `playback <错词>` 是 `UnknownVerb`、近似名恰是那两行。导出的字节、读界与复核由 `accounting::playback::tests` 判定（accounting-SPEC.md 8-12）。

## 8-143 playback 的时间选择：`--since`、`--until`、`--day` 与城工具的同名参数（`bin::main::playback`、`accounting::worker::workbench::tools::playback`；accounting-SPEC.md 8-17）

**形状。** 两扇门各读三个原文，交给 `accounting::playback::Window::span`，把得到的 `UtcSpan` 经 `Selection::during` 加进选择。解析、交集与拒绝都在 accounting-SPEC.md 8-17；本节只定两扇门的拼法。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--since <utc>] [--until <utc>] [--day <yyyy-mm-dd>] [--include-confidential] [--page <template>] [--out <file>]
```

```json
{"action": "export", "name": "day-1", "day": "2026-05-14", "since": "2026-05-14T09:00:00Z"}
```

- **人的门。** `--since <utc>` 与 `--until <utc>` 的写法与 `view --since/--until`（8-137）相同：UTC、到秒、以 `Z` 结尾；`--day <yyyy-mm-dd>` 是那一个 UTC 日。区间是 `[since, until)`，三者同给时取交集。读不了的值、交出空区间的组合，stderr 写那条 `AxError` 的人读形式，退出 2，什么也不导出；合法而什么都没选中的区间照常退出 0，写出带范围信息的空 bundle。
- **居民的门。** `export` 多三个可缺的字符串参数 `since`、`until`、`day`，同一种写法、同一个函数读；读不了或交集为空时工具以 `E_INVALID_ARGS` 拒绝，不写文件。
- **不改 `view`。** `view` 的时间过滤照 8-137；两处共用 `runtime::clock::{parse_iso, UtcSpan}`，所以是同一个时间语义。`view` 没有 `--day`：它是看账本行的透镜，按天的回看是 playback 的事。

**本节测试**：`main::playback::tests`：`--day` 读不了与 `--day` 和 `--since` 交出空区间，各以退出 2 拒绝。`accounting::worker::workbench::tools::playback::tests`：居民的 `day` 进到导出的 `source.selection`，写成那一天的两端。区间本身的选择由 `accounting::playback::tests::span` 判定（accounting-SPEC.md 8-17）。

## 8-132 `playback check` 的五项、`export --page`，与居民的城工具 `playback`（`bin::main::playback`、`accounting::worker::workbench::tools::playback`；accounting-SPEC.md 8-13）

**形状。** 两个适配器。`main/playback.rs` 给人用，城工具 `playback` 给居民用；页面嵌入、五项检查、落盘都是 `accounting::playback` 的（accounting-SPEC.md 8-13），这里只定两扇门各自读什么、写到哪、怎样报。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--since <utc>] [--until <utc>] [--day <yyyy-mm-dd>] [--include-confidential] [--page <template>] [--out <file>]
sprawling playback check <file> [--bundle <file>] [--city <city>] [--include-confidential] [--observed <file>]
```

**人的门。**

- **`export --page <template>`**：读模板（超过 `PAGE_MAX_BYTES` 先拒绝），经 `accounting::playback::embed` 把这次导出的 bundle 放进去，写出的是页面而不是 bundle；去处照 8-126，stdout 或 `--out`。页面过不了结构或静态离线一项时退出 1，什么也不写。
- **`check <file>`**：文件可以是 bundle，也可以是页面（accounting-SPEC.md 8-13 按首字节分）。`--bundle` 与 `--city` 可以一起给，各自成一项；`--observed <file>` 是 skill 写下的浏览器观察记录。stdout 写一行 JSON：`file` 是被查文件的 BLAKE3（十六进制，观察记录的 `page` 要写它）；`digest`（bundle 的 BLAKE3）与 `events`（十进制字符串）在读出了 bundle 时出现；`structure`、`bundle`、`source`、`offline`、`browser` 五项各是 `{"status": "passed" | "failed" | "unchecked"}`，`failed` 带 `found`，`unchecked` 带 `why`，`browser` 通过时带 `covered`（观察走过的路径）。
- **退出码**（8-103 的表）：0 是 `Report::holds`——没有一项失败，也没有一项要了而做不了；1 是其余，包括 `--city` 复核不了、观察记录说的是另一份字节；2 是命令行读不懂。没要的项写 `unchecked` 但不影响退出码。

**居民的门：城工具 `playback`。**

```json
{"action": "export", "name": "day-1", "from": 12, "through": 340, "run": "…", "building": "newsroom", "page": "<!doctype html>…"}
{"action": "check", "file": "day-1.html"}
```

- **读者由上下文绑定。** 工具在铺工作台时拿到这个 run 所在的楼，导出与复核都用 `Reader::Resident(这栋楼)`。参数里没有读者：`reader`、`include_confidential` 之类不认识的字段以 `E_INVALID_ARGS` 拒绝，所以居民扩大不了读者，人的 `--include-confidential` 也不会下放给居民。按现行读界，居民可以回看别的非机密楼。
- **`export`**：`name` 是 1 到 64 个 ASCII 字母、数字、`-`、`_`，首字符是字母或数字；`from`、`through` 是 seq，`run` 是 run id，`building` 是楼的地址，与人的门的选择同义（8-126）；`since`、`until`、`day` 是时间条件（8-143）。给 `page` 时经 `embed` 写 `<name>.html`，否则写 `<name>.json`。落点是 `CityLayout::playback_exports()` 下以本楼地址命名的目录，经 `accounting::playback::land(Place::Exports)` 写：不覆盖、不跟随链接、在 git 仓库里时必须被忽略、失败不留半成品。结果是 `{"file", "digest", "events"}`，页面另带 `structure`、`offline` 两项；bundle 与页面的字节不进结果，因为工具结果会写进账本。
- **`check`**：`file` 是本楼导出目录里的一个 `<name>.json` 或 `<name>.html`，别的名字与别处的文件都拒绝。它用同一个读者对城复核，结果是 `Report::line`：五项，`bundle` 与 `browser` 为 `unchecked`（居民的门不收另一份 bundle 与观察记录）。
- **效应与写门。** 登记 `Effect::Write { domain: 房间 }`，与 `signal`、`pr` 等写桌子的工具同样走写门；`writes` 回答 `Writes::Nothing`，因为导出件落在工作树之外，checkpoint 没有东西可收。工具的效应按登记而不是按调用，所以 `check` 也不会被推测执行提前跑（只有 `Effect::Read` 会，runtime-SPEC 的 speculation）。
- **寿命。** 导出件跟着城：它在城根的保留子树下，不在任何 worktree 里，清扫 worktree、run 冻结或重开都不碰它；没有独立 worktree 的 run 也写在同一处。两次导出用了同一个名字，后一次被拒绝。崩溃留下的暂存文件见 accounting-SPEC.md §3。
- **登记。** 每一栋楼的工作台都有它，排在内置工具的末尾、`transcribe` 之后、城外工具之前：新工具接在末尾，前面已缓存的工具表前缀不动。

**本节测试**：`main::playback::tests`：五项写成一行 JSON，没要的项写 `unchecked` 而退出 0，失败或要了而做不了的项退出 1。`accounting::playback::tests::page`：`embed` 写出的页面逐字节带着 bundle，行里的 `</script>` 关不掉数据块，结构与静态离线两项通过；重复的数据块、重复的 `id`、指不到的链接与 `data-seq`、数据块里重复的 JSON 键在结构一项失败；外部资源（`img`、转义过的 CSS `url()`、`@import`、SVG 的 `image`、`srcset`）、`base`、`form`、外链、`meta refresh` 各是一条静态离线的发现；CSP 缺了、在别的元素之后、放进一个主机、缺 `form-action` 各是一条发现；观察记录说的是另一份字节为 `unchecked`，记下一次请求为失败。`accounting::worker::workbench::tools::playback::tests`：带 `include_confidential` 或 `reader` 的调用被拒；机密楼的行到不了居民的导出，导出用同一个读者复核通过；同名的第二次导出被拒且原文件不变，过不了静态离线的页面什么也不留；没有 worktree 的 run 把页面写进城里，`check` 只认本楼导出目录里的名字。已存在、被跟踪、不在导出位置、会进历史的目标由 `accounting::playback::tests::landing` 判定。两名居民（记者、编辑）生成、核对并留存报告由 `tests/acceptance/playback.rs` 判定。

## 8-106 城景有界（`accounting::views::answering`、`storage::hot`；storage-SPEC §8-5、wire-SPEC `CityAnswer`）

- **`CityView` 的 `runs` 取自 `HotView::runs`**：热视图只留活跃的全部，加 `last_seq` 最近的 `storage::RECENT_FROZEN` 个冻结跑（storage-SPEC §8-5），按 RunId 序。`active`／`frozen` 两个数仍是全城的数，页面拿 `frozen` 减去列表里冻结的行数，就知道还有多少在列表之外；那些跑经分页的 `History`／`RunHistory` 读。理由：城景是页面最常问的答复，它的大小原先与城的全部历史同阶（8,000 次跑的城约 1.37 MB），有界之后只与活跃数同阶。答复的语义变了，`WIRE_V` 加一。
- **客户端的 `staleBy` 只在城景真会变的记录上作废它**：`run_started`、`run_frozen`（列表的成员变了），楼与 pursuit 的那组记录（`buildings`／`pursuits`），`city_halted`（`halted`）。一次跑中途的记录只推进 `last_seq`／`last_kind`，页面自己的折叠（`core/belief`）已经从同一条记录读到了，重拉城景只是把同一件事再运一遍。
- **`CostView` 的 `by_run` 同样有界**：活跃的跑一个不少（顶栏「这次跑花了多少」读的正是正在动的那次），其余从 `Attribution` 仍持有行的跑（即热视图没逐出的冻结跑，见下）里只取花得最多的 `views::answering::TOP_BILLED`（32）个，同额按名字，按名字序输出。选法是对非活跃行做一次 `select_nth_unstable_by`，O(runs)。`total` 仍是全城的权威总额，`by_run` 的和因此可以小于它；界面本就按 `total` 算占比，列表之外的钱留作看得见的余额，而不是被摊掉。另外四个维度（actor、segment、tool、skill）的桶数不随跑数增长，不截。`TOP_BILLED` 与 `RECENT_FROZEN` 一样是线上答复的大小上界，不随机器变。
- **`RunView` 问到一次被逐出的跑时读冷的一侧**：`hot.get` 答不出而 `hot.was_evicted` 认得它，就从账本索引取这次跑的全部序号，按序号从旧到新把它的记录折进一个只装这一次跑的新 `HotView`，答那一行。折法仍是 `HotView::apply`，冷热两侧没有第二份「一条记录怎么变成一行」。代价是这次跑的记录数，只在有人打开一次旧跑时付。既不在热视图里、也没有墓碑的跑答 `None`；账本读不出时答 `Unavailable`，因为被逐出的跑必有记录，读不到是「没能看」而不是「没有这次跑」。
- **一次跑花了多少只有一处答：`Views::billed_to`**（`views::billed`）：`Attribution::billed_to` 持这次跑的行就答它；没有行而热视图逐出过它，就从账本索引取这次跑的全部序号，按序号从旧到新折进一个只装这一次跑的新 `storage::Attribution`，再读它的行——与 `RunView` 的冷侧同一种读法，折法仍是 `Attribution::apply`，冷热两侧没有第二份计价；两者都不是，这次跑没花钱，答零。账本读不出时答 `None`。`CostOf` 的逐跑明细、`Commit`／`Commits` 的 `spent` 与 `RunCosts` 都经它，不再为每一行先把整份 `report()` 折出来。
- **`RunCosts { runs }` 是 `by_run` 之外的冷查询，一问最多一页**：一次最多 `wire::RUN_COSTS_MAX`（64）个 RunId，按问的顺序答 `(run, spent)`，超出的部分不答；楼的目录页按新到旧列跑、只问一次，所以它只显示最新的 `RUN_COSTS_MAX` 个冷跑的花费，更旧的跑显示为未知；账本读不出的跑不出行，于是「没出行」是「没能看」，「零」是「没花钱」。楼的目录页对不在 `cost_view.by_run` 里的跑用它。新增一条 Query，`WIRE_V` 加一。被否：按 RunId 游标翻全城的 `by_run`——那要么让 `Attribution` 继续为每次跑留一行，要么每页扫一遍整本账本；按名字问，代价只与被问的跑的记录数同阶。
- **`Attribution` 的 `by_run` 只留热视图没逐出的跑**：一条 `run_frozen` 折进之后、或一条记录落在已逐出的跑上时，`Views::apply` 让 `Attribution::retain_runs` 丢掉热视图逐出的那些跑的行；`total` 与另外四个维度不动。于是 `by_run` 的行数与活跃数加 `RECENT_FROZEN` 同阶，被丢的钱经 `billed_to` 的冷侧仍答得出。
- **验收**（`views::standing_tests`、`a_city_of_eight_thousand_runs_answers_in_a_bounded_view`）：折入 8,000 次开始又冻结的跑后，`city_view` 序列化成 JSON 不超过 16 KiB，列出的正是最近冻结的 `RECENT_FROZEN` 个，`frozen == 8000`。
- **验收**（`views::standing_tests`、`an_evicted_run_still_answers_its_run_view`）：`RECENT_FROZEN`＋1 次跑都开始又冻结、写进账本后，最早那次已被逐出热视图，`RunView` 仍答出它：冻结、房间与开始时刻都在。
- **验收**（`views::standing_tests`、`a_cost_view_of_eight_thousand_billed_runs_names_the_top_few`）：8,000 次各自计费的跑都冻结后，一个仍在跑的也计了费；`cost_view` 的 `by_run` 恰是那次活跃的跑加花得最多的 `TOP_BILLED` 个，`total` 仍是全部的和。
- **验收**（`views::standing_tests`、`an_evicted_run_still_answers_what_it_cost`）：`RECENT_FROZEN`＋1 次各自计费的跑都开始又冻结、写进账本后，`RunCosts` 问最早那次与一个从没出现过的跑，答出前者计的费与后者的零，按问的顺序。
- **验收**（`views::standing_tests`、`the_attribution_holds_only_the_runs_the_hot_view_holds`）：同一座城里，`Attribution` 的 `by_run` 不再有被逐出的那次跑，`total` 仍是全部的和，`RunCosts` 对它仍答出它计的费。

## 8-107 回收站的一行放回原处（`accounting::worker::commanding::restoring`，形状：适配器）

```rust
impl RunWorker {
    /// `Command::RestoreDiscard` 的执行者。
    pub(in crate::assembly) fn restore_discard(&mut self, restoration: &kernel::Restoration) -> Result<(), AxError>;
}
```

- **先写盘，后落账**：`Tracked(file:<addr>@<oid>)` 经 `storage::Checkpoint::restore` 把那个 blob 写回城根下同一路径，成功之后才追加 `discard_restored`。反过来的次序会让历史说一个文件回来了，而盘上没有它。载荷与被关掉的那条 `file_discarded` 同形（`paths: ["file:<addr>"]`、`restoration`），于是 `DiscardView` 用同一个 `discard_lines` 读两种记录，按路径关掉那一行（§8-6）。
- **写回的是城根，不是 lease 的 worktree**：检查点的对象在城的对象库里，一个评审运行的 worktree 与城共用它；人要回的是自己丢的文件，放回主干的那个位置。
- **拒绝**：`Interred` 与带 `range` 的定位符答 `E_INVALID_ARGS`（前者从内容仓库取回尚未接线，后者不是整个文件）；`Rebuildable` 答 `E_INVALID_ARGS`，recovery 就是那条重建的理由；提交找不到或路径不在提交里，是 storage 的 `E_WORKTREE_BUSY`，原样上抛。
- **被否：按路径在写线程上查回收站**。写线程不持有 `DiscardView`；为一次还原把整份历史再折一遍，是在人按下按钮的那一刻付一整次重放。页面手里的那一行已经带着路（wire-SPEC §8-47）。

## 8-108 开城时收走崩溃留下的树（`accounting::worker::lifetime`；storage-SPEC §8-9）

**原因**：评审楼的树按房间保留（§3），归还后留在盘上等同一房间的下一轮活；开城时账本的写锁保证没有 run 持有任何树，所以城造过的每一棵树都是上一次服务（包括崩溃的那一次）留下的：git 的登记、保留子树下的目录与 git 为它建的分支，一棵最多 `WORKTREE_MAX_BYTES`。

- **在哪里做**：`RunWorker::over` 在开账之后调 `storage::Worktrees::sweep_abandoned(city_root, &[])`。`held` 为空，因为账本的独占锁保证此刻没有别的进程在用这座城，而这个 worker 还没有派出任何一轮；`over` 是 `serve`、`resume` 与 `form_city` 共同的构造点，所以每条开城路径都清扫。
- **失败不挡开城**：清扫失败（例如 Windows 上一个文件还被别的程序打开）以 `Level::Effect` 写进 `Diagnostics`，城照常打开；留下的树下一次开城再收。一棵收不走的树不该让人进不了自己的城。
- **收走了什么也说**：收走至少一棵时，同一级别写一行，列出名字；什么都没收时不写。
- **不碰的东西**：人加的 worktree、人的分支、带着未进 HEAD 的提交的租约分支、`refs/sprawling/runs/` 下的检查点引用（storage-SPEC §8-9）。

**本节测试**：`storage::worktree::sweep::tests` 的四条——崩溃留下的租约被收走（`a_crash_left_lease_is_swept_when_the_city_opens`）；检查点引用留下；另一座城的家目录下的树留下；活着的 run 的树与人自己的树留下——以及 `accounting::worker::lifetime` 的 `a_crash_left_worktree_is_gone_once_the_city_opens`，它从开城这一侧看同一件事。

## 8-127 进程死在写账的半途之后，城照规则重开（验收：`crates/sprawling/tests/acceptance/crash.rs`）

**要什么。** 服务进程在写一行账的半途被杀，或机器断了电，人再打开这座城时，城说出它截掉了什么，链照样验得过，丢了答复的调用关成「结果未知」而不是「失败」，视图从幸存的历史与补写的行折出，那间房照样派得出活。这一条把这几件事放在同一座真城上一次验收，作为 V0.1.0 崩溃验收（G5）的白盒证据。

**怎么造一次死亡。** 一座真城，脚本化的模型（`ModelFactory` 缝）让一次 run 调一次 `status`；run 写完后丢掉 worker——这释放写者锁，与被杀的进程释放它一样——然后在盘上把账截在回答那次调用的 `tool_result` 那一行的一半处，那一行之后的行与之后的段都不存在：它们正是一个死在那一刻的进程来不及写的东西。之后经 `RunWorker::new` 重开，跑 `startup_scan`（`sprawling resume` 的那一遍），再经一次性查询 `accounting::views::ask` 问城景，最后向同一间房再派一次活。

**验收的五件事，一个值比较**（`Reopened`）：

1. `startup_scan` 报 `TailDropped { bytes }`，字节数就是被截的那半行的长度（8-102）；
2. 账里多一行 `log_truncated`，整条链经 `verify_ledger_dir` 验过；
3. 那次调用恰有一个答复，码是 `E_TOOL_OUTCOME_UNKNOWN`，`startup_scan` 报关了一个（ARCHITECTURE §5 末）；
4. 城景列出的那次 run 已冻结，结局是 `cancelled`，`last_kind` 是启动扫描写的那行 `run_frozen`、`last_seq` 是它的序号，账上那一行带 `cause: process_died`——视图从截过、补过的历史折出，没有拿一份比账长的快照作答（8-91、8-101），死掉的 run 按 ARCHITECTURE §13.7 的 `Lost --> Frozen` 冻结（kernel-SPEC §8-82-2，accounting-SPEC §8-18-1）；
5. 同一间房的下一个任务立起一次新 run 并冻结它：死掉的 run 没有把房间占住。

**能咬住，量过。** 把 `startup_scan` 里补写结果未知的那一步拿掉再跑，测试红在第 3、4 两项上（答复为空，城景的 `last_kind` 停在 `tool_called`）；不冻结死掉的 run 时，红在第 4 项上（城景把它列为未冻结，`last_kind` 停在补写的 `tool_result`）。

**断电那一半归 storage。** 断电比被杀多丢的，是平台还没落盘的字节；那是账本自己的耐久契约，由 `storage` 在它的故障文件系统上证（`power_cut_matrix_over_every_op_keeps_acknowledged_waves`，storage-SPEC §8-2）。这里不重证它：故障文件系统只承载账本，城的其余文件与 `Standing::fold`、视图的折叠读的是真目录，在它上面重开的不是一座完整的城（§12「崩溃验收在盘上造死亡」）。

**死掉的 run 由谁冻结。** `startup_scan` 补完悬空调用之后，为每一次有 `run_started`、没有 `run_frozen` 的 run 写一行 `RunFrozen::lost()`：结局 `cancelled`，载荷 `cause: process_died`，作者是那次 run 的居民（accounting-SPEC §8-18-1；为什么不是第四种结局，见 kernel-SPEC §12.14）。所以第 4 项比的是城景里那次 run 的整行状态：已冻结、结局、最后一行与它的序号。

## 8-109 `sprawling up --supervise`：崩溃 → `resume` → `serve`（`bin::supervising`、`bin::supervising::children`）

**要什么。** 一座城的服务进程崩了，人不在键盘前时没有人把它拉起来。`--supervise` 让 `up`（与 `serve`，两者共用一张 flag 表与同一个 `serve_city`）不自己服务，而是守着一个子进程：子进程就是 `sprawling serve <city> <addr> …`，崩了先跑一次 `sprawling resume <city>`（验链、收掉进程死亡留下的悬空调用），再起一个新的 `serve`。

**两个模块。**

- `bin::supervising`（形状 1 decision）：`CrashBudget` 与它的判定，无 I/O、无时钟，时间作为参数进来。

  ```rust
  pub(crate) const CRASH_WINDOW_MS: u64 = 60_000;
  pub(crate) const CRASH_LIMIT: usize = 3;
  pub(crate) struct CrashBudget { /* 窗口内的崩溃时刻，最多 CRASH_LIMIT 个 */ }
  pub(crate) enum Next { Stop, Restart(CrashBudget), Degraded { crashes: usize, cause: String } }
  impl CrashBudget {
      pub(crate) fn fresh() -> Self;
      pub(crate) fn after(self, closing: &Closing, at: TimeMs) -> Next;
  }
  ```

  `Closing::Chosen` → `Stop`；`Closing::Broken` 记下这一刻，丢掉距今已满 `CRASH_WINDOW_MS` 的旧崩溃，窗口里凑满 `CRASH_LIMIT` 次即 `Degraded`，否则 `Restart`。
- `bin::supervising::children`（形状 4 adapter）：`pub fn supervise(city: &Path, addr: &str, child: &Child) -> Result<Ended, AxError>`，`pub enum Ended { Chosen, Degraded }`。它起子进程、等它退出、按退出状态造一个 `Result<(), AxError>` 交给 `Closing::of`，再把结果交给 `CrashBudget::after`，自己不做判断。交给 `after` 的时刻是守护起步以来经过的毫秒数，取自单调时钟 `serving::standing::monotonic_now`，不是墙钟：系统把墙钟往回拨会让一次旧崩溃一直留在窗口里，往前拨会让一次新崩溃提前出窗口。

**子进程怎么读成 `Closing`。** 退出码 0 是 `serve` 在人按 Ctrl-C 后有序收口的结果，读作 `Chosen`，守护随之结束；其余一切（非零码、被信号杀掉而没有码）读作 `Broken`，`cause` 是退出状态的文字。这一次读法只有 `Closing::of` 一处，守护不另造一套分类。

**degraded 要人显式解除。** 窗口里第三次崩溃后守护不再重启，打印最后一次的 `cause` 与崩溃次数，然后等标准输入的一行：人按 Enter 即解除（预算清零，`resume` 后再 `serve`）；读到 EOF（没有人在）即以失败退出；终端读不了时先打印读失败的原因，再同样以失败退出。**原因**：一个 60 s 内崩了三次的服务，再拉起来多半还是崩，且每次崩溃都可能留下一条悬空调用；让它无限重启是把一个需要人看的故障藏进循环里。**否决的方案**：指数退避后永远重试——那样故障永远不会被人看见。

**子进程的那一行。** 开不开浏览器（`opening()`）与进不进控制台（`wanted`）都由 `serve_city` 算一次，守护只把结果变成 flag：`pub struct Child { forwarded, first: Window, console: Console }`。只有第一次起的子进程带 `--open`（当 `opening()` 判为 `Open::Browser`），之后每次都带 `--no-open`，因为重启不该每次弹一个新窗口；子进程带 `--console` 当且仅当不带 `--supervise` 的同一行会进控制台，否则带 `--no-console`。其余 flag 由 bin 的 `verbs::forwarded` 按 `SERVED` 表的 `Takes::Value` 连同其值原样转给 `serve`，守护不再解析 argv。子进程继承这个终端。

**不做的事。** 不接管开机启动：守护活在人起它的那个终端里，终端关了它就走。锁（S5.02 的单写者锁）由子进程持有、随子进程死亡释放，守护本身不碰城的锁，所以 `resume` 与下一个 `serve` 都拿得到它。Ctrl-C 同时落到守护与子进程：子进程照 8-11 有序收口写 handoff，守护不拦截信号。

**决定。**

1. 守护与服务是两个进程，不是同一进程里的一个 `catch_unwind`：一次 abort、栈溢出或内存耗尽不回到 unwind，只有进程边界接得住。
2. 退出码 2（命令行被拒、没有城）也算崩溃，交给预算，而不单列一种「不必重启」：那是第二套分类；三次之内它就进 degraded，人看得见原因。
**尚未做到的（本节接口的当前状态）**：查询仍在锁内作答，`GitStatus` 等做 I/O 的查询仍在锁内做 I/O，所以读者之间、以及读者与折叠线程之间仍会互等；发布 `Arc<ViewsSnapshot>` 供查询无锁读取、把 I/O 移到锁外（锁内只取所需的小数据），是这一接口余下的两步。

### 8-110 `RunWorker` 按所持状态拆开：凭据一组（`accounting::worker::credentials::held`）与协作一组（`accounting::worker::collaborating`），形状 1 数据

```rust
// accounting::worker::credentials::held —— shape: data
pub(in crate::assembly) struct Credentials {
    pub(in crate::assembly) book: gateway::EndpointBook,        // 接了哪些端点、每个 tag 选了哪个模型
    pub(in crate::assembly) vault: Arc<Mutex<gateway::Custodian>>,
}
impl Credentials {
    pub(in crate::assembly) fn opened(book, vault: gateway::Custodian) -> Credentials;
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError>;
}
// accounting::worker::collaborating —— shape: data
pub(in crate::assembly) struct Collaborating {
    pub(in crate::assembly) rooms: RoomQueues,                                   // 每个房间的队列，以及哪个 run 借着它
    pub(in crate::assembly) joins: BTreeMap<Address, collab::FanIn>,             // 每个房间从下派的工作收回了什么
    pub(in crate::assembly) workshops: BTreeMap<Address, collab::Underway>,      // 每个房间摆出、尚未全部汇合的图及其已派集；handback 到达时由它派下一组
    pub(in crate::assembly) requests: Vec<collab::OpenRequest>,                  // 等人检查的 pull request
    pub(in crate::assembly) goals: Vec<kernel::GoalEntry>,                       // 居民认领的地盘，按认领顺序
}
// accounting::worker::recording
impl RunWorker {
    fn absorb(&mut self, kind: EventKind, run: RunId, addr: Option<&Address>, data: &Payload) -> Result<(), AxError>;
}
```

`RunWorker` 拆成六个对象：凭据、治理、协作、计划、入口、飞行中的 run。每个对象带走自己的字段、方法与测试，`RunWorker` 只持有这六个对象和账本、CAS、日志这些整城共用的东西。凭据一组是 `book`、`vault` 两个字段：它们回答同一个问题——这座城能以谁的身份去叫哪个模型——而且改动它们的是同一族记录（`endpoint_attached`、`model_selected`、`endpoint_lost`、`secret_captured`）。协作一组是 `rooms`、`joins`、`requests`、`goals` 四个字段：它们都从信号、handback、pull request 与 `goal_registered` 这几族记录折出，回答的是「居民之间正在交接什么」。`pursuits`、`plan_holders` 与 `delegator` 虽然也由协作折叠（`folds::Collaboration`）折出，却回答「每栋楼在朝什么推进」，属于计划一组，不归这里。治理早已是一个对象（`views::Governance`），判定面与读面共用那一个定义，这一步不动它。计划、入口、飞行中的 run 三组见 8-111。

**worker 写下的每一行，会话起点、治理、计划、凭据四份折叠都要看到，不论这行以城的名义还是以某个 run 的名义写。** 重启走 `Standing::fold`，那条路把账本上每一行都交给每一份折叠，不问是谁写的；活着的 worker 若只在以城的名义写时才折凭据与会话起点，一行由 run 写下的 `endpoint_attached` 就在账本上、却不在 worker 的 `book` 里，直到进程重启——这正是 8-17 已经排除的那类「活城与重启折出两份」。所以 `record_where` 与 `record_for` 都经过同一个 `RunWorker::absorb`，它把这一行交给会话起点、治理、计划、凭据四份折叠，每份都看过之后才交出第一份的错误：行已经在账本上，一份折叠出错不该让排在它后面的折叠漏看这一行、在重启之前与重启折出两份。哪份折叠看哪几种记录由各自的 `absorb` 决定，这里不再筛。协作一组与入口不经过 `absorb`：协作由写下那一行的效果处理器当场改，入口由 `entrance.stamp` 改；重启时 `Standing::fold` 把每一行也交给这两份折叠。

**红**：一个 run 以自己的名义写下一行 `endpoint_attached`（`record_for`），随后 worker 的 `book` 与从同一账本重折出来的 `book` 应当列出同样的端点。改动之前，worker 的 `book` 为空而重折的那份有这一端点。

**为何字段仍是 `pub(in crate::assembly)`**：这一步是纯搬移，读写这些字段的调用只把 `self.book` 改拼为 `self.credentials.book`、`self.rooms` 改拼为 `self.collaborating.rooms`；把它们收进 `Credentials` 的方法是另一次改动，混进来会让搬移与行为变化无法分开审。

### 8-111 计划、入口、飞行中的 run 三组离开 `RunWorker`（`accounting::worker::plans::held`、`accounting::worker::doorstep`、`accounting::worker::driving::flight`），形状 1 数据

```rust
// accounting::worker::plans::held —— shape: data
pub(in crate::assembly) struct Planning {
    pub(in crate::assembly) pursuits: BTreeMap<Address, kernel::Pursuit>, // 每栋楼在朝什么推进
    pub(in crate::assembly) delegator: kernel::Delegator,                // 深度零的位置：只有它能宣布一个 pursuit
    pub(in crate::assembly) holders: PlanHolders,                        // 每栋楼的计划里，哪个节点由哪个房间认领着
    pub(in crate::assembly) write_plan: fn(&Path, &[u8], &[u8]) -> Result<(), AxError>, // 落地替换 Roadmap.md 的那一扇门（8-42-8）
}
impl Planning {
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, addr: Option<&Address>, data: &Payload);
}
pub(in crate::assembly) struct PlanHolders(BTreeMap<Address, BTreeMap<kernel::NodeId, String>>);
impl PlanHolders {
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, addr: Option<&Address>, data: &Payload);
    pub(in crate::assembly) fn in_building(&self, building: &Address) -> BTreeMap<kernel::NodeId, String>;
}
// accounting::worker::doorstep —— shape: data
pub(in crate::assembly) struct Doorstep {
    pub(in crate::assembly) entrance: Entrance,   // 这座城按 key 答过哪些命令、答了什么（8-41）
    pub(in crate::assembly) knocks: Vec<Knock>,   // 没人在家时被搭话的居民，等说话的 run 冻结后再叫醒
}
impl Doorstep {
    pub(in crate::assembly) fn opened(entrance: Entrance) -> Doorstep;
}
// accounting::worker::driving::flight —— 已有的 Flight 收下两个字段
pub(in crate::assembly) struct Flight {
    pool, gate, driving, homes,                                          // 原有：在 lane 里的 run、写历史的关口、回家的顺序
    pub(in crate::assembly) checkpoint_gate: Arc<Mutex<()>>,                  // 一城一次一个检查点：一个仓库只有一个 index
    pub(in crate::assembly) backlog: runtime::Backlog,                   // run 留下仍在跑的命令，halt 从这里找到它们
}
```

计划一组是 `pursuits`、`delegator` 与认领表三样：它们回答「每栋楼在朝什么推进、谁在做哪一块」，而 `pursuit_changed` 与 `roadmap_*` 这几族记录只改动它们。`delegator` 跟着 `pursuits` 走，因为宣布一个 pursuit 只能经过深度零的位置，而这个位置只在开城时铸一次（`lifetime`）。

入口一组是 `entrance` 与 `knocks`：两样都是「已经到了城门口、还没变成 run 的工作」——按 key 来的命令、居民之间的搭话。名字取 `Doorstep` 而不是「入口」的直译，因为 `Entrance` 已经是其中按 key 去重的那一份（8-41）。飞行一组把 `checkpoint_gate` 与 `backlog` 收进已有的 `Flight`：检查点是 lane 里的 run 轮流去过的那道门，`backlog` 是它们留下仍在跑的命令，两者和 `Flight` 一样一城一份、只随 run 的起落变化。这两组是纯搬移，读写处只把 `self.knocks` 改拼为 `self.doorstep.knocks`、`self.backlog` 改拼为 `self.flight.backlog`。

**认领表只有一份定义。** `roadmap_claimed` 把房间记进它所属楼的表，`roadmap_finished`、`roadmap_released`、`roadmap_blocked` 与拆分父节点的 `roadmap_split` 把节点移出（拆分之后这一轮什么也不持有，8-42-8）；房间就是这行记录的 `addr`，楼是 `addr` 的第一段，节点是载荷的 `node`。重启的协作折叠与活着的 worker 调用同一个 `PlanHolders::absorb`，所以两边不会各写一份规则。一行读不出楼或节点的记录不进表：认领表只记确知的持有者，不去猜。
**认领表只有一份定义。** `roadmap_claimed` 把房间记进它所属楼的表，`roadmap_finished`、`roadmap_released`、`roadmap_blocked` 把节点移出；房间就是这行记录的 `addr`，楼是 `addr` 的第一段，节点是载荷的 `node`。重启的协作折叠与活着的 worker 调用同一个 `PlanHolders::absorb`，所以两边不会各写一份规则。一行读不出楼或节点的记录不进表：认领表只记确知的持有者，不去猜。

**红**：一个 run 落下一行 `roadmap_claimed`（`record_for`，认领效果正是这样落地的），随后 worker 读到的持有者（`holders_in`）应当与从同一账本重折出来的一样。改动之前，worker 的表在开城之后再不更新：左边是空表，右边是 `{2: "lab/room1"}`。

**`pursuits` 由 `plans` 直接改写，且只在 `pursuit_changed` 落账之后改**：宣布、暂停、恢复、撤下一个 pursuit 时，`plans` 先判定（有没有计划、有没有正在追的目标）、铸出要宣布的 `Pursuit` 并算出要写的 `goal`，追加 `pursuit_changed`，追加成功后才改 `Planning::pursuits`。追加失败时进程里的 pursuit 不变，与重启从账本折出的一致。被否：先改表再追加——追加一失败，活着的 worker 就持有一个账本从未记下的 pursuit。被否：让 `Planning::absorb` 折这一行、由记录铸回 `Pursuit`——铸造要经深度零的 `Delegator`，把它交给折叠会让每个折叠点都能宣布目标。

### 8-92 装配根与 serving、doctor 之间的依赖只朝一个方向

`bin::assembly` 是 `sprawling` 里唯一知道所有具体类型的地方，别的模块不应当反过来知道它（ARCHITECTURE.md §3）。一条从 serving 或 doctor 指回装配根的边，意味着改装配根的内部可能改坏一个读面，而读面本来只该依赖它读的那份事实的权威。城的唯一写者与读面都住 `accounting` 之后，装配根只剩自由函数与直接碰主机的生产适配器（accounting-SPEC.md 8-11、§12-12）：`production`（`SystemClock`、`hands`、`init_city`、`form_city`）、`listening`（占端口、开写者）、`attending`（起写者线程）、`chain_watch`（起审计线程）与 `dropping`（拖进对话框的文件）。

**账本在哪，由 `kernel::layout::CityLayout::ledger` 一处回答。** 每个读账本的地方直接调用 `CityLayout::new(city_root).ledger()`，装配点不另设转发函数：转发只给同一件事换了名字，却会让读面与 serving 为了一个路径去依赖装配点。

**从账本重建视图是视图自己的事。** 一次性的读经 `Views::rebuild(ledger_dir)`（`accounting::views::asking`）起步：先经 `snapshot::start_audited` 同步审计整条链，再从合适的快照起步、只折尾部（8-91）。服务中的城由 `accounting::worker::folds::fold_city` 一遍折完：`Views::over(ledger_dir)` 造出空视图，`Views` 与 `StandingFolds` 在 `runtime::replay::fold_ledger_dir` 的同一遍里各折每一条记录，这一遍返回的账本索引经 `Views::hold_index(index, ledger_dir)` 交给视图。打开账本的时刻由 `listening` 读 `SystemClock` 交进来，`fold_city` 自己不读钟。

**写者与驱动 run 的机器属于 `accounting::worker`，起线程的一半留在装配根。** 命令等在其上的命令台（`accounting::worker::desk`）、驱动 run 的 lane（`accounting::worker::pool`）、lane 写回账本的中继（`accounting::worker::relay`）、记账线程的循环（`accounting::worker::attend`）与调用时判定计划认领的 `accounting::worker::booking`（8-42-8）都在驱动 `RunWorker`，所以与它同住；`relay` 持有 `pool::Arrival`，`desk` 持有 `relay::Wake`，它们住在一处才没有反向边。起写者线程的 `assembly::attending`、占端口并开写者的 `listen` 与它返回的 `Listening`（`assembly::listening`）留在装配根，它们只经 `accounting::worker` 的 `pub` 面碰写者。serving 留下的是：门上的钥匙与金库（`door`）、一次 serve 由调用方填好的那个值（`serve::Serving`）、进程日志的出口（`journal`）、写者旁边折叠视图的线程（`folding`）、核心线程站的档位与单调钟（`standing`，8-93），以及还在跑的命令已写出的字节（`output_ring`，8-115）。serving 的产品代码不写出 `crate::assembly`，也不写出 `accounting::worker`。

**serving 不往命令台投命令，所以不需要句柄。** 投命令的是 `listen` 交给 socket 的那几个闭包、控制台与 ACP 入站；前一个随 `listen` 住在装配根，后两个本来就在 serving 之外。另一种做法是把 `listen` 拆成传输的一半（留在 serving，持一个 serving 自己定义的命令发送端）与写者的一半（进装配根）；它多出一个类型和一条通道，换来的只是 `listen` 住在 serving 里，而 `listen` 的次序（8-88）恰好是「先占端口，再开写者」这一件事，拆开后这个次序要由两边共同守。

**日志行的时间由构造者交进来。** `Journal::new` 收一个 `serving::journal::Clock`，即 `Arc<dyn accounting::Clock + Send + Sync>`；`main::city` 交的是 `assembly::SystemClock`，墙钟只在 `bin::assembly::production` 采样（8-63，accounting-SPEC.md 8-3）。

反方向是组装点应有的方向，保留：装配根用 serving 的 `open_vault`、`Serving`、`folding`、`output_ring::OutputRing`、`standing::monotonic_now`，用 doctor 的 `ThisMachine`、`Platform`、`PATIENCE`、`host`、`recipe_for`，用 `accounting::worker` 的 `RunWorker`、`CommandDesk`、`Hands`、`genesis::form` 与 `folds::start_served_views`。

**方向由门守。** ARCHITECTURE.md 的 `directions` 块逐个模块写下它的产品代码永不写出的路径，`cargo xtask depmap` 读它（xtask-SPEC 8-33）。块里有三行：`accounting` 的 views 不写出 `crate::worker`；doctor 与 serving 不写出 `crate::assembly` 与 `accounting::worker`。读面与写者同住 `accounting` 之后，「写者知道读面、读面不知道写者」由第一行守；`accounting` 不依赖 `sprawling`，读面与写者指回装配根由编译器拒绝。城有没有历史由 `city::has_history` 回答（city-SPEC 8-29），doctor 与写者都从那里取用。

读面用到的五样东西各归其主，worker 从那里取用：

| 事实 | 住处 | 理由 |
|---|---|---|
| 一个居民或房间叫什么 | `kernel::Address::name`（kernel-SPEC `Address`） | 地址的最后一段是地址自己的事实；城的名册与楼的页面都从这里读 |
| 一栋楼的页面、`DOC_BYTES_MAX` | `accounting::views::building_page` | 页面是一个读面：按问的那一刻读盘，不持有第二份 |
| 一台 MCP server 经哪种传输到达（`McpLink`） | `agent_protocols::mcp::link` | 三种传输（`agent_protocols::mcp` 下的 `stdio`、`http`、`sse`）与把它们合成 `agent_protocols::Outbound` 的那个枚举同住 `agent_protocols`；读面与装配点都经 `agent_protocols` 的握手与列工具说话 |
| broker 的钥匙登记在哪、这座城对 broker 是谁（`broker_for`） | `accounting::toolkit_broker`（accounting-SPEC.md 8-9） | 页面与命令读同一组事实；连接动作 `connect_toolkit` 仍是 worker 的 |
| 一个锁着的 vault 的解析器与锁中毒时的拒绝（`resolving`、`poisoned_vault`） | `accounting::held_vault`（accounting-SPEC.md 8-9） | worker、读面与 serving 都要一次性的解析器；拒绝的措辞只有一处 |


### 8-112 保温的门：run 的模型调用经 `Warmed` 走，落地后留在 `RunWorker` 上（`accounting::worker::keeping_warm`，形状 1 数据）

```rust
// accounting::worker::keeping_warm —— shape: data
pub(crate) type Door = runtime::prefix::warmth::Warmed<fn() -> Result<TimeMs, AxError>>;
#[derive(Default)]
pub(in crate::assembly) struct Kept { /* 每个房间一扇门 */ }
impl Kept {
    pub(in crate::assembly) fn keep(&mut self, room: String, door: Door); // next_due 为 None 的门直接丢弃
    pub(in crate::assembly) fn next_due(&self) -> Option<u64>;         // 所有门里最早的一次续期
    pub(in crate::assembly) fn renew_due(&mut self, now_ms: u64) -> Vec<(Address, CacheRenewed)>;
}
impl RunWorker {
    pub(crate) fn warm_due(&self) -> Option<u64>;
    pub(crate) fn renew_warm(&mut self, now: TimeMs); // 每次续期写一条 cache_renewed；写不进账本才进诊断日志
}
```

`dispatching::agreeing` 选定适配器后，用 `city::keep_warm(city_root, building)` 读到的设置和 worker 的 `clock`（`accounting::Clock`），把它包成 `Door`；`Agreed`、`Site`、`Driving`、`Driven` 与 `driving::lane` 带的都是这扇门，所以 run 发出的每个请求都进了保温账（runtime-SPEC 8-4-2），转向循环不知道保温存在。`settling::landing` 在 run 结束时把门交给 `Kept`，按房间地址存：同一房间后来的 run 换掉前一扇门，因为后一个前缀才是下一次会用到的。`next_due` 为 `None` 的门（设置为 `Off`，或已续过一次）不留，所以默认设置下 `Kept` 一直是空的，worker 不多占一个字节，也不多发一个请求。

`assembly::attending` 的空闲分支先读日程，再调用 `renew_warm(now)`，下一次睡眠取「距下次读日程」与「距 `warm_due`」中较短者，所以一次续期不会等到日程的整点之后。每次续期，不论成败，都在房间地址下写一条 `cache_renewed`（kernel-SPEC 8-4）：成功时是 provider 自报的用量与账单额，失败时是它的拒绝原样，所以人从账本上读得出保温花了多少、哪个 provider 拒了续期。续期失败不停城；只有这条记录本身写不进账本时才写进诊断日志，与日程读取失败同样处理。

失败：`renew_due` 不传回 `Err`，每扇到期的门各得一条结果，模型的失败原样放进 `CacheRenewed::Refused`；一扇门失败不妨碍其他门续期，失败的门被丢弃，所以拒绝续期的 provider 不会在每次醒来时再被问一遍。

当前状态：保温已完整：设置默认关、默认下不发任何额外请求，打开后到期前续期一次，每次续期的用量或拒绝都记在账本上；lead 由门在每次调用前后量出（runtime-SPEC 8-4-2）。

决定：门按房间存，而不是按 run 存。按 run 存时，一个房间连续跑十次会留下十扇门，其中九扇续的是已经被下一个前缀覆盖的缓存，花的钱没有用处；按房间存时门的数目以房间数为上限。重新考虑的条件：同一房间里并行的会话各有自己的前缀。

### 8-145 备树：lane 在 run 驾驶完之后补一棵，下一次放置接管它（`accounting::worker::dispatching::preparing`）

```rust
// storage —— storage-SPEC 8-35
impl storage::Worktrees { pub fn stock(&self) -> Result<storage::FileWork, storage::StorageError>; }
// accounting::worker::dispatching::preparing —— Staged::fly 的模型臂
// drive_run 返回之后、Flown 交回之前：
//   if 这个 run 借了树（site.lease 为 Some）{ Worktrees::open(city_root)?.stock() }
```

- **为什么在这里。** 一次放置等的是实时扫描对每个新建文件的放行，512 个 16 KB 文件的树以秒计（storage-SPEC 8-31、8-35）。备树把这次检出挪到没人等的地方，放置只改名。没人等的地方只有一处：一条 lane 的 run 已经驾驶完、最后一条记录已经写下并广播之后。记账线程不行（8-113：记账线程不等盘）；`land` 在记账线程上；开城时补一棵会把一次全量检出加进首字节（8-122）；一条专门补树的后台线程不在 ARCHITECTURE §10 第 3 条点名的起线程处之列。
- **只在借了树的 run 之后补。** 没有评审、也不是试验的楼从不借树，它的 run 补一棵只会白占一棵树的盘。`stock` 在备树已在干线上时什么都不写（只称城），所以连续的 run 之后补树的代价是一次称重。
- **代价落在谁身上。** 补树期间这条 lane 还没交回 `Flown`，房间队列也还没还（`land` 先还队列与树）：同一房间紧接着的下一次派活多等一次补树；别的房间不受影响（lane 各自独立）。只有备树刚被接管过（又一个新节点第一次放置）时补树才是一次全量检出，其余时候是差量。
- **失败。** `stock` 的失败不改变 run 的结局：run 已经结束，补不上备树只让下一次放置退回全量检出。失败写一条 `Refuse` 诊断行，锚在 lane 的账本位置快照上，与 8-113 lane 半段的诊断行同一个写端。
- **试验借树（8-133）的重开参数读的是这里。** §12「试验借一棵自己的树」以放置读数是否以秒计为重开条件：接管备树之后放置以毫秒计，那条决定的前提回到它被定下时的样子。

当前状态：storage 一半已落地（storage-SPEC 8-35：`stock`，放置先接管备树），citysim 的 `large_worktree_placement` 在每轮之前调 `stock`（citysim-SPEC §3-13、§8-12）。lane 里的这一次调用还没有写：`dispatching::preparing` 不在落地 storage 一半的那次改动里，所以生产路径上还没有人调 `stock`，每一次放置都退回全量检出。补上它的检验：一座评审楼里两个房间先后各派一次活，第二个房间的 `worktree_opened` 之前那次 `claim` 的 `FileWork.created` 为 0。
### 8-113 派活的准备进 lane：记账线程只做决定，树、MCP 连接与冻结在 lane 里（`accounting::worker::dispatching::running`、`accounting::worker::dispatching::preparing`、`accounting::worker::driving::flight`）

**为什么切。** 一次派活的准备里等盘或等别的进程的有三样：评审树的放置（一次提交加一次检出，时长随树的大小）、MCP 缺表的那次启动与握手、冻结时的 CAS 写。它们在记账线程上时，每条 lane 的 append 都排在它们后面，因为 lane 写历史的那道口子只有记账线程在服务（8-46-2）。所以准备切成两段，切在第一个等待之前。

- **记账线程：`stage_dispatch(&mut self, at, task, goal) -> Result<(Staged, Continuation), AxError>`。** 同意、托管、规则、房间、定形、brief、job，配置与身份的读取，run id（连同它那一次时钟采样），`governance.sent`，滤表，`Site::name_tree`（评审楼的分支就是树的名字，`tree_of(addr)` 的纯函数，于是 `open_desks` 造 PR 桌时已知分支，树还在 lane 里放），`open_desks`，`inherited`，以及 lane 要读的状态快照：正在处理的命令的幂等键、`collaborating.rooms` 里挂着信的房间集合、`ledger.position`、治理的自治级别、`city_hash`；最后 `enrol_run`。这些要么写账本，要么改记账线程持有的折叠，要么是一次时钟采样。留在这里，run 与 run 之间的决定次序只由记账线程定。`inherited` 留下，是因为它读账本索引、写 lineage 一行，账本与 `LedgerIndex` 只属于记账线程。`Staged` 拥有它的全部字段，是 `Send`。
- **lane：`Staged::fly<L: Ledger>(self, ledger: &mut L, context: DriveContext) -> Flown`**（`dispatching::preparing`），先准备、再 `drive_run`，这是 lane 的全部工作；准备这一段依次做：`Site::place_tree`，`Stamping` 包着这条 lane 的 relay，于是 `worktree_opened` 经 relay 写；`lay_out_workbench`，MCP 缺表时的连接在这里；`Freezing::freeze_plan`，CAS 用 lane 共用的句柄；`probe_after`、`Sieving::for_run`、`checkpoint_scope`。`ensure_base` 在 `checkpoint_gate` 里做：城还没有提交时它提交一次城的 index，与检查点争同一把 `.git/index.lock`，8-46-13 的理由原样适用。`claim` 只动本房间的树与分支，不进这道闸。`Site` 与 `Assignment` 无论准备成败都装在 `Flown` 里、随 `Arrival` 回家，因为 `land` 要还 `Site` 借到的树；`Workbench` 经 `Driving` 进驾驶、随 `Driven` 回家，`land` 读它的 `delegates` 与 `succession`。准备失败时 `Flown.driven` 就是那个错误，`land` 照常先还房间队列与树，再把错误交给问的人，和驾驶失败走同一条路（8-46-9）。
- **`DrivingPool` 排队与起 lane 的单位是 `Staged`**：满了的时候等着的是一份还没准备的派活，它不占线程，也还没有占树、没有连 server。`Staged` 带着 `site.run_id`，池按它认重复。

**lane 借什么。** 8-110、8-111 拆出的对象里，lane 只借本来就是共享句柄的那几样，每样是一个 `Arc` 的克隆或一个值，收在 `Staged` 的 `Laying` 里：`Flight.backlog`（`exec` 与 `status` 用）、`Flight.checkpoint_gate`、`Credentials.vault`（工具解析密钥用）、`city_root`、lane 共用的 CAS 句柄、MCP 常驻表、诊断的写端、时钟。房间的六张桌子（信、目标、计划、书架、PR、作坊）本来就是 `Arc<Mutex<_>>`，`Desks::for_bench` 把它们的克隆与 `waiting` 作为 `BenchDesks` 交给 lane，`Desks` 本身（连同队列的 `tenure`）留在 `Continuation` 里，只有它还队列。常驻表：`accounting::Connectors::connect` 取 `&self`（accounting-SPEC.md 8-2），`RunWorker.connectors` 是 `Arc<dyn Connectors + Send + Sync>`，lane 借一个克隆；表的寿命仍是 worker 的寿命，最后一个 `Arc` 落地时 `Drop` 杀子进程（8-4）。CAS 是 lane 共用的一个句柄 `RunWorker::lane_store`（`Arc<Mutex<Cas>>`），随 worker 打开，冻结与筛子的 offload 都经它写，每次在锁里做完：`storage::Cas::open` 会清扫 `tmp/` 里的半成品，而一次写的半成品名只由对象哈希决定，一条 lane 自己开句柄就会扫掉另一条 lane 正在写的对象，两条 lane 写同一个对象也会共用同一个半成品文件。记账线程仍用自己的 `RunWorker::cas`，它写 brief 与 job，不写冻结的四段。检查点同理：一座城第一次打开检查点时建出仓库，两条 lane 同时建会争仓库的 config 锁，所以 `Laying` 打开检查点时拿 `checkpoint_gate`（8-46-13）。`Collaborating`、`Planning`、`Doorstep`、`Governance`、账本与 `LedgerIndex` 不借：记账线程的折叠改写它们，lane 读到的只能是 `Laying` 里的快照——`collaborating.rooms` 里挂着信的房间集合、这个居民持有的目标、治理的自治级别、`ledger.position`、`city_hash`。快照在 `stage_dispatch` 里取，lane 看到的是这个 run 被决定那一刻的城。`origins.started`（房间的会话从这个 run 起算）也在 `stage_dispatch` 里、紧跟在读 `carried_from` 之后：它改的是记账线程的折叠；准备在 lane 里失败的 run 与驾驶失败的 run 一样，都已开始了房间的会话。

**lane 半段的诊断行。** `mcp_tools` 为每台 server 写一行（连上、已连着、拒绝）并写它自己的计时，`admit_reading_room` 为阅览室点名却不在架上的名字写一行。诊断的写端是可共享的句柄 `recording::Notes`（一个 `Arc<Mutex<Diagnostics>>`），`RunWorker.log` 就是它，lane 借一份克隆，行锚在 `Laying` 里的账本位置快照上。锚取快照是因为诊断只写不读：`runtime::diagnostics` 没有读回一行的方法，决定与恢复的代码因此不可能依赖锚；读者是人，人用 `seq` 把一行对到历史上的位置，准备阶段的行锚在准备开始的位置，正是它说的事发生的地方。锁中毒时照样取出写端：每次写在锁内一步做完，中毒只说明别的线程在锁外崩了。被否：`Prepared` 把行带回家、由 `land` 写——锚落在整个 run 之后，一台拒绝握手的 server 要等 run 落地才被人看见，而 `Refuse` 这一档写给正在看的人。

**冻结前缀为什么逐字节不变。** 前缀由三段文件、一个 run slot 与工具表拼成，每个输入在记账线程上与在 lane 里是同一个值。三段读的是城根下的文件，从 `stage_dispatch` 到 lane 冻结之间记账线程不写它们：它写的是账本、brief 与 job。run slot 由 `Given`、模型注记与继承的对话拼成，三者都在 `Staged` 里。树的路径是 `city_root` 与 `tree_of(addr)` 的纯函数，与哪条线程去认领无关。工具表的顺序由 `lay_out_workbench` 的准入顺序定；其中 MCP 那部分，命中时是表项记下的 `listed`，缺表时是这一次 list 的结果，与握手在哪条线程上无关。run id 仍在记账线程上铸，从它铸 id 的三件工具拿到的还是同一个 id。唯一换了线程的时钟读数是计时用的 `[prepare_dispatch_ms]`，它不进任何记录；铸 run id 的那一次采样不搬（`stand_up` 的文档：结构改动不得移动时钟采样）。

**行的次序。** 准备阶段在 lane 里写的行有两种：评审楼的 `worktree_opened`，与继任 run 的 `eval_run`（`probe_after` 在继任者起步前问一次模型，写下与前任答案的比较）。它们和驾驶写的行走同一个 relay，所以一个 run 自己的行仍是准备的行在前、驾驶的行在后，两个 run 的行怎样交错由 relay 决定，重放的确定性由 8-46-5 保证。relay 服务写下的每一行——lane 的 append 与调用时入账的认领行——都由 `serve` 交回，`serve_flight` 逐行交给 `RunWorker::absorb`，与记账线程自己写的行走同一条路（8-110）：活的折叠与重启折叠读同一份历史，一条被这四份折叠（会话起点、治理、计划、凭据）读的记录搬进 lane 不用另接线。折叠拒绝一行时那一行已在账上、回信已发出，拒绝记进诊断，不改变车道收到的答复。`record_for` 还给记账线程写的每一行盖上正在处理的命令的幂等键，relay 的行不经它。重启后认出一条重复的命令，靠的是 `Entrance::absorb` 在历史里见到带这个键的某一行；一次评审楼的派活在规则没变时（`book_rules` 只在规则变了才写 `rules_changed`），带键的行只有 `worktree_opened` 这一行。所以 `Staged` 带上命令的键（`Entrance::carrying`，没有命令时为空），`Stamping::record_for` 用同一条盖键的规则给 `worktree_opened` 与 `eval_run`（`Site::probe_after` 也经 `Stamping` 写）盖上，这两行由 lane 写与由记账线程写逐字节相同。盖键的规则是键与载荷的纯函数 `commanding::entrance::stamped`，`RunWorker::record_for` 与 `Stamping::record_for` 都调它。`a_review_dispatch_sent_again_after_a_restart_is_answered_once` 经 `serve_one`（认键的那道门；`handle` 不经它）守着这一点。

**验收的两条测试。** 其一是 `accounting::worker::mcp::tests::a_dispatch_whose_server_still_shakes_hands_leaves_the_desk_free`：楼的配置里一台握手要等测试放行的 stdio MCP server，`serve_one` 派一轮活；握手放行之前 `serve_one` 已经返回，于是记账线程在这次派活里不等握手。测试自己在十秒后放行，所以一个等握手的记账线程让断言失败而不是挂住。树的一半是 `accounting::worker::reviewing::tests::kept::a_tree_that_waits_on_the_index_lock_is_placed_in_the_lane`：一座还没有提交的城、一个评审房间，先放一把 `.git/index.lock`；`serve_one` 之后这个 run 已在一条 lane 里，第一次放置的提交在 lane 里等锁并失败，失败随 `Flown` 回家、由 `land` 交还。其二是守护（`a_review_dispatch_freezes_the_prefix_it_froze_on_the_accounting_thread`）：一座新城、一个评审房间、同一条 job，这个 run 在 `prompt_assembled` 里记下的四段哈希等于测试里钉住的那一组，它们取自记账线程上准备一切时的路径。不需要钉时钟：铸 run id 的那次时钟采样不进前缀，两座新城派同一条 job 冻结出同样的四段。它的作用是让准备换线程改不了前缀；有意改动新城前缀文字的改动，从它的失败里取新的哈希。

## 8-114 移走一栋楼（`accounting::worker::commanding::removing`，形状 3 决定；wire-SPEC §19、city-SPEC §8-3）

```rust
impl RunWorker {
    pub(in crate::assembly) fn remove_building(&mut self, addr: &Address) -> Result<(), AxError>;
}
impl RoomQueues {
    pub(in crate::assembly) fn worked_within(&self, building: &Address) -> Option<(Address, RunId)>;
}
```

- `Command::RemoveBuilding` 走这里。城拥有 `city::remove_building` 不知道的两件事：楼里有没有 run 正在跑，以及 Ledger 记什么。
- **一个拒绝**：楼里某个房间的队列正借给一个 run（`worked_within`），拒 `E_BUSY`，主语点名房间与 run，恢复是先停下它。房间队列是城里唯一说「这里有人在干活」的账，另开一本会与它分歧；逐个看全部房间，因为地址序把 `lab-2` 排在 `lab` 与 `lab/room1` 之间，按前缀截区间会漏。
- **先搬后记**：文件先搬进 `.sprawling/removed/`，再写 `building_removed`（载荷 addr、kept），因为这一行说的是「已经搬了」。楼以前写过的每一行都留在账里，什么都不删。
- 客户端（`client/src/core/removal.ts`）在 City Hall 上不画这个控件，楼里有 run 在跑时画成不可用并说明原因；确认走 `parts/dialog`。城仍自己拒这两种情况，页面只是不把一个只会被拒的按钮交给人。

### 8-115 还在跑的命令写出的字节，给后来打开页面的会话留一段（`bin::serving::output_ring`）

```rust
// bin::serving::output_ring —— shape: value
pub(super) struct OutputRing { /* Mutex<BTreeMap<RunId, Kept>> */ }
impl OutputRing {
    pub(super) fn keep(&self, piece: &wire::LiveOutput); // 追加；超过上界时丢最旧的整块
    pub(super) fn settle(&self, record: &EventRecord);       // tool_result 落账：清空这个 run
    pub(super) fn so_far(&self) -> Vec<wire::LiveOutput>; // 每个 run 按到达次序
}
```

- **没有失败返回**：它存的是可丢弃的预览（runtime-SPEC §8-28-3），账本里的结果才是权威；锁中毒时照样取出里面的表，因为每个操作都在一步之内让表保持一致，中毒只说明别的线程在锁外崩了。
- **上界 `KEPT_BYTES_PER_RUN = 64 KiB`**，等于 runtime 一秒钟最多读出的字节（`READ_BYTES_PER_MS = 64`），也就够页面按 `LIVE_LINES = 400` 行画满两条流（每行约八十字节）。超过就从最旧的一块丢起，但最新的一块总留着，哪怕它自己超过上界。每个 run 的内存因此有上界，没有 run 在跑命令时表是空的。
- **喂与清在记账线程上同步发生**：`attending` 装给 `Serving::outputs` 的闭包先 `keep` 再广播；装给 `worker.observe` 的观察者先 `settle` 再交给视图折叠。块在这次调用的结果落账之前读出，所以同一线程上的次序保证清空之后不会再收到这次调用的块。
- **决定**：缓冲住在装配层而不是 `wire`。清空要认出 `tool_result` 这一行，而记账线程的观察者就在这里；放进 `wire` 要让它为这件事再订阅一次事件流。被否的另一种是让每个会话自己记：那只能记它打开之后的块，正好漏掉这个缓冲要补的那一段。

## 8-125 生产的命令结果带时钟行，`status` 报这一刻（`accounting::worker::driving`、`accounting::worker::workbench`；runtime-SPEC §8-10、§8-53）

**接线**：

| 东西 | 谁持有 | 何时定 |
|---|---|---|
| `runtime::ClockReading` | `Site.clock` | `stand_up` 造一个空的；这一跑的 `status` 工具（`StatusTool::clocked`）与 `Sieving` 各拿一份克隆 |
| `runtime::StampGate` | `Sieving.stamps` | `Sieving::for_run` 按 `Site.config` 的 `clock_stamp` 与 `clock_zones` 造，一跑一个 |
| 写读数 | `drive_run` 交给 `RunHooks` 的 `now` | 每读一次墙钟，先 `keep` 进 `Sieving.clock` 再交给驱动 |

`Sieving::package` 打包一份 `exec` 结果之前，拿 `ClockReading::latest` 与这条调用的工具声明的 `Temporal` 问 `StampGate::observe`，得到的戳交给 `runtime::package_exec`，戳成为结果 `content` 的附件行。驱动在工具面打包之前读这条调用的答复时刻（runtime-SPEC §8-15），所以一条命令的戳是它答复的那一秒，与账本里它那一行 `tool_result` 的 `t` 同一个读数（runtime-SPEC §3 第 5 条）。

- **只有 `exec` 的结果挂戳**：`status` 自己有 `now:` 一栏，连接器与浏览器的结果不经 `package_exec`，它们的时钟行要一种挂在 JSON 结果上的形状，本节未定；它们不经 `StampGate`，所以「一跑的第一条结果带一次戳」在生产里是「第一条命令结果」。
- **不给工具面一个钟**：`Placing` 与 `Sieving` 手里只有读数。换成工具面自己读 `hands.clock` 会让一跑有第二个采样点，这是 runtime-SPEC §12.8 否掉的那一条。

**验收**：`driving/tests/sieving` 的 `a_served_command_result_ends_with_the_second_its_call_answered`：一次真实派活，城里没写 `[clock]`，`exec` 的结果在账本上以 `clock: <ISO>;` 结尾，秒数等于这条调用 `tool_result` 的 `t`。戳等于答复时刻由 runtime 的 `a_stamp_the_face_reads_is_the_moment_its_answer_records` 钉住。

## 8-93 核心线程站在正常档之上，空转就降回（`bin::serving::standing`，形状：状态机）

agent 派出的命令从低于正常的档位起动（runtime-SPEC §8-13-3）；本节补上另一半：核心自己的线程起动时把自己升到正常档之上一级，于是一台被构建占满的机器上，视图的折叠与广播仍排在派出的命令前面。升在线程一级而不是进程一级：进程档位在 Windows 上要对自身句柄调 `SetPriorityClass`，没有安全接口；线程档位有。

```rust
// bin::serving::standing —— shape: state machine
pub(crate) enum Standing { Raised, Normal(Held) } // 这条线程实际站在哪一档
pub(crate) enum Held { ByTheSetting, Refused(String), ByTheValve }
pub(crate) enum Verdict { Keep, Lower }
pub(crate) const BUSY_LIMIT: Duration;            // 10 s
pub(crate) fn raise_this_thread(setting: CorePriority) -> Standing;
pub(crate) fn lower_this_thread() -> Result<Standing, thread_priority::Error>;
pub(crate) struct Valve;                           // 忙了多久，是否该降回
impl Valve {
    pub(crate) fn new(limit: Duration, now: Instant) -> Self;
    pub(crate) fn record(&mut self, woke: Instant, slept: Instant); // 一次醒来到下一次阻塞
    pub(crate) fn verdict(&self) -> Verdict;
}
pub(crate) struct CoreThread;                      // 一条核心线程的档位与它的阀
impl CoreThread {
    pub(crate) fn raise(name: &'static str, setting: CorePriority, now: Instant) -> Self;
    pub(crate) fn record_turn_lowering_when_busy(&mut self, woke: Instant, slept: Instant);
}
pub(crate) fn setting_telling_a_refusal() -> CorePriority; // 读不了就 Normal，并向标准错误说出拒绝
pub(crate) fn serving_runtime(setting: CorePriority) -> std::io::Result<tokio::runtime::Runtime>;
// bin::assembly
pub(crate) fn monotonic_now() -> Instant; // 单调钟的唯一取样点，与墙钟的 `assembly::SystemClock` 并列；阀量的是时长，墙钟会跳
// accounting::person（accounting-SPEC.md 8-8）
pub enum CorePriority { Raised, Normal }                 // 人的设置；Normal 即「关掉高优先级」
pub fn core_priority() -> Result<CorePriority, AxError>; // ConfigInvalid：priority 既不是 "raised" 也不是 "normal"
```

- **升到哪一档**：`thread-priority` 的跨平台值 70，在 Windows 上是 `THREAD_PRIORITY_ABOVE_NORMAL`（正常档进程里基准优先级 9，派出的 `BELOW_NORMAL_PRIORITY_CLASS` 子进程是 6）；降回用 50，即正常档。读回档位的测试（`serving::standing::tests` 里的 `a_raised_core_thread_stands_above_normal` 等）只在 Windows 上编译，Unix 上 70 与 50 各落到哪个 nice 值没有测试读回，要查它得加一条 `cfg(unix)` 的读回测试，并让它在没有 `CAP_SYS_NICE` 的 CI 主机上只断言被拒的那一支。Unix 上升档要 `CAP_SYS_NICE`；没有时操作系统拒绝，线程留在正常档，`Standing::Normal(Held::Refused(原因))` 把原因带回来，`CoreThread::raise` 向标准错误说一次——相对效果由子进程的 `nice` 给出，不靠这一步。
- **设置**：人的配置文件（`Home::person_config`，即 `config.toml`）里 `[core]` 一节的 `priority`，`"raised"` 或 `"normal"`，缺省为 `"raised"`。`"normal"` 让 `raise_this_thread` 不调任何平台接口，返回 `Standing::Normal(Held::ByTheSetting)`。文件读不了、解析不了或值拼错时，服务照常起动，核心留在正常档，并向标准错误说出拒绝与恢复办法：升档是有全机代价的一方，要有一个说「可以」的读数才做。设置在服务起动时读一次，改了要重启服务。
- **安全阀**：升了档的线程若空转，会拖住整台机器。每条升档线程带一个 `Valve`，每处理完一件事记一次「醒来—再次阻塞」；窗口从上一个窗口关上时开始，第一件结束在窗口开出 `BUSY_LIMIT` 之后的事关上它；关上时忙的时间占到窗口的 15/16 以上，`verdict` 变成 `Lower`，此后一直是 `Lower`。`record_turn_lowering_when_busy` 见到站在升档上的线程得了 `Lower`，就调 `lower_this_thread` 回到正常档，并向标准错误写一行说给人：哪条线程、忙满了多少秒、已降回正常档。平台拒绝降档时同样写一行，说出平台的原因，此后这条线程不再试：两个平台都不拒绝线程给自己降档，所以拒绝是一次缺陷报告，每一轮都重试只会让标准错误每轮多一行，而不会让它降下来。时间作参数传入，`Valve` 本身不读钟，所以它的判定可以用编造的时刻逐点验证。
- **量的是墙钟，不是 CPU 时间**：线程在两次阻塞之间走过的墙钟时间是它占着核的时间的上界；升了档的线程很少被抢占，二者接近。
- **哪些线程**：视图线程 `sprawling-views`（`bin::serving::folding`）起动时升档，每折完、广播完一件记一次。socket 服务所在的 tokio 多线程运行时由 `serving_runtime` 建（`bin::main::city` 用它代替 `Runtime::new`）：每条 worker 第一次被唤醒（`on_thread_unpark`）时升档，此后每次唤醒到下一次停放（`on_thread_park`）记一次，档位与阀放在线程本地。只在唤醒时升，因为只有 worker 会停放与唤醒：阻塞池的线程（`spawn_blocking`）从不唤醒，所以永远不会成为没有阀的升档线程；代价是一条起动后从未停放过的 worker 留在正常档，而这正是安全的一侧。
- 证据：`crates/sprawling/src/serving/standing/tests.rs` 的 `a_raised_core_thread_stands_above_normal`（Windows 臂：升档后读回 `AboveNormal`）与 `a_thread_busy_through_the_window_is_lowered`（忙满一个窗口的线程判降回，忙一半的不判）；`a_socket_worker_stands_above_normal_once_it_has_woken`（Windows 臂：`serving_runtime` 的 worker 睡过一次之后读回 `AboveNormal`）；`crates/sprawling/src/person.rs` 的 `a_person_who_turns_the_raise_off_gets_normal_priority`。

**决定**

1. 线程档位取第一档「安全 Rust」：`thread-priority`（MIT）对外只给安全接口，内部的 `SetThreadPriority`／`GetThreadPriority` 由它负责，本仓不写 `unsafe`。不取 Zig 叶子：一次只传句柄与常量的调用没有 `(ptr, len)` 边界可以放在 Zig 后面，Rust 侧调用 `extern` 的那一处 `unsafe` 也省不掉。被否：本仓自己写 `unsafe` 调 `SetPriorityClass`，把整个进程升到 `HIGH_PRIORITY_CLASS`——没有测量表明它全局最佳，而且会把 tokio 的每条线程连同它们的空转风险一起升上去。重开参数：线程档升满之后，后台负载占满所有核时 relay 往返 p50 与空闲之比仍大于 1.2。
2. 安全阀量墙钟而不量 CPU 时间：读线程自己的 CPU 时间在 Windows 上是 `GetThreadTimes`，没有第一档的路；在一轮之内，墙钟只会高估忙的程度，所以对已经结束的轮，阀只会判得早，不会判得晚。阀只在一轮结束时判：一条从不停放的 tokio worker（持续有任务时，tokio 的维护性 `park_timeout(0)` 不调停放与唤醒回调）和一次不返回的折叠都没有「轮结束」，所以它们一直留在升档上，阀对它们不起作用。重开参数：出现对外只给安全接口、读线程 CPU 时间的 crate。
3. 降回之后不再升：一条线程忙满过一个窗口，就说明它的工作量不该排在派出的命令前面；反复升降只会让人看到忽快忽慢。

**尚未做到的（本节接口的当前状态）**：阀只在一轮结束时判定（决定 2），所以一条持续有任务、从不停放的 worker 和一次不返回的折叠永远不会被降回，而这正是阀要防的情形；在忙的期间也作判定——tokio worker 按每次任务轮询记（`tokio_unstable` 下的 `on_before_task_poll`／`on_after_task_poll`），或在每次唤醒与任务边界处拿正在进行的一轮已走过的时间比窗口——是这一接口余下的一步。写线程 `sprawling-runs`（记账）还没有升档——它的循环在 `serve_flight` 里面阻塞，循环看不到它醒来的时刻，而没有阀的升档线程正是本节禁止的；把醒来的时刻从 `serve_flight` 交出来之后，它按视图线程的办法升档。Unix 上没有 `CAP_SYS_NICE` 时，每条 worker 各说一次它留在正常档。降回时写的是标准错误，还不是一条类型化的 Ledger 事件（事件种类表的一行加 kernel-SPEC 的表）。doctor 还不报告每个平台实际站在哪一档。

## 8-94 性能监视器的历史：有人看才采样，每项 300 点（`bin::monitor`，形状：状态机）

WebUI 的监视页、事实条上的摘要与 `sprawling gauge --at <地址>` 读的是同一份历史：每秒一个 `Sample`，最近 300 个（5 分钟）。`bin::monitor` 只管两件事：此刻有没有人在看，以及看的人读到的那 300 个点。计数器从哪里读（核心进程、Job Object、整机、城所在的卷、记账线程）由调用方传进来的读取函数决定，本模块不碰平台接口。

**接口。**

- `Sample`：一次读数，全部是 `u64` 的整数计数，没有浮点（它会上线协议，定义在 `wire::frames::monitor`，见 wire-SPEC.md 8-47，本模块用的就是那一个类型）。字段：`core_cpu_permille`、`core_private_bytes`、`core_working_set_bytes`、`core_read_bytes`、`core_written_bytes`、`machine_cpu_permille`、`machine_available_bytes`、`volume_free_bytes`、`ledger_queue_depth`、`durable_lag`、`relay_p50_nanos`、`event_to_screen_p50_nanos`、`queued_runs`。
- `Monitor::new()`：不分配。
- `Monitor::watch(&self, watched: Watched) -> Watch`：一个在看的人，看整页（`Watched::Everything`）或只看事实条摘要（`Watched::Summary`，`Watched` 定义在 `wire::frames::monitor`）。两类分开计数。`Watch` 被丢弃时这个人就不再算数；它可以跨线程持有（socket 线程持有，采样线程计数）。
- `Monitor::tick(&mut self, read: impl FnOnce(Watched) -> Sample)`：每秒调用一次。有人看整页时以 `Watched::Everything` 调用 `read` 一次；只有看摘要的人时以 `Watched::Summary` 调用，读的一方只读本进程；把结果放进历史，满 300 个时丢掉最旧的。没人在看时不调用 `read`，并释放历史占的内存。
- `Monitor::is_watched(&self) -> bool`：此刻有没有人持有 `Watch`，不论哪一类。
- `Monitor::history(&self) -> impl Iterator<Item = &Sample>`：从最旧到最新。
- 没有失败路径：计数是 `AtomicUsize` 的加减，历史的容量在第一次放入时一次预留。

**定下的值。** `CAPACITY = 300`（每秒一点，5 分钟）；`HISTORY_BUDGET = 64 KiB`，`CAPACITY × size_of::<Sample>()` 超过它时编译失败（13 个 `u64`，现为 31 200 字节）。

**决定。**

1. 没人看时既不读计数器也不留历史。零开销指的是 CPU 与内存两轴：不读就没有系统调用，释放就没有常驻的 31 KiB；代价是重新打开监视页时曲线从空开始。另一种做法是一直采样、页面打开就有 5 分钟的曲线，它让每个没人看的城都多付一份开销，而这正是人要求避免的。重新考虑的条件：人要求打开页面就看到过去 5 分钟。
2. 读取函数由调用方传入，而不是一个 trait。现在只有一种读法（生产的平台计数器）；测试传一个计数的闭包即可验证「没人看不读」，不必为一个没有第二实现的接缝造 trait。
3. 历史是一个 `VecDeque`，第一次放入时 `reserve_exact(CAPACITY)`，之后不再分配。

**测试。** `monitor::tests`：没人看时 `read` 一次也不被调用、历史为空，人走了以后历史被释放；放入 301 个点后只剩最后 300 个、从旧到新；只有看摘要的人时 `read` 收到 `Summary`，再来一个看整页的人时收到 `Everything`，他走了以后回到 `Summary`。

**本节接口的当前状态。** 计数器读取与每秒一拍的采样线程见 8-96；线上的一对监视帧见 wire-SPEC.md 8-47：会话发 `Watch` 时经 `assembly::listening` 交给它的 `watch` 在这里的 `Monitor` 上计一个看的人，发 `Release` 或断开时不再计。WebUI 监视页 `client/src/views/monitor.svelte` 在 `#/monitor`，经这对帧打开时计一个看的人、关闭时释放，读数与曲线由 `client/src/core/monitor.ts` 按 8-95 的规则算出。`sprawling gauge --at <地址>` 经这对帧看监视器，见 8-97。事实条摘要 `client/src/views/facts.svelte` 经 `WatchSummary` 看监视器，只显示本进程的 CPU 与工作集；只有它在看时采样线程只读 `OwnProcess`，不打开 `sysinfo`。其余尚未落地：采样一次 ≤ 50 µs、占 CPU ≤ 0.1% 的仪表。

**内存的读数**（`bin::monitor::memory`）：整机物理内存与可用内存经 `sysinfo`（只开 `system` 特性，只刷新 RAM）一次读出成 `Memory { physical, available }`，这是城里读内存的唯一一处；`read` 刷新本线程留着的一个 `System`，不每次新建：同一台 Windows x86-64 桌面级机器、测试档构建上，新建句柄时一次读数 2.2–2.3 µs，留着句柄时 1.6–1.7 µs，余下的是平台调用本身；按线程留而不是全局一把锁，读数的线程之间不互等；计划推进按它决定下一行是否排队（§8-46-3，只管计划行；其他入口的现状见那里）。平台不报时两项都是零，此时不算紧。取 `sysinfo`，因为它是对外只给安全接口的现成路，本 crate 不写 `unsafe`。

## 8-95 一座城的读数：一行 JSON 与一屏曲线（`bin::monitor::top`，形状：projection）

`sprawling gauge --at <地址>`（别名 `top`，8-129-4）读 8-94 的历史，按 stdout 那头是谁（`bin::audience`）选一种输出。本模块只把历史投影成文本，不碰终端、不碰 socket：判断 stdout 那头是谁、每秒重画一次、从城里取历史，都是调用方的事。

**接口。**

- `json_line(sample: &Sample) -> serde_json::Result<String>`：一个 JSON 对象，键就是 `Sample` 的 13 个字段名，值是整数；不含换行。stdout 不是终端时每秒打印的那一行由 `gauge::lines::city_line` 写出：同样的键，前面多一个 `"line":"city"`（8-129-4）。agent 按行读，一行一个完整的读数。
- `sparkline(values: impl IntoIterator<Item = u64>, width: usize) -> String`：终端画面里一项计数器的曲线。只取最后 `width` 个值，每个值一个字符，从 `▁` 到 `█` 共 8 级，按这几个值自己的最小值到最大值线性分级（整数运算，最小值画 `▁`，最大值画 `█`）；全部相等时整条画 `▁`。值不足 `width` 个时曲线就短一些，不补空白。
- 失败：`sparkline` 没有失败路径。`json_line` 只转交 `serde_json` 的错误；13 个整数字段没有可被拒绝的内容，所以它实际上不会出现，调用方把它当作写 stdout 失败处理，而不是在这里用一个隐藏的 `unwrap` 吞掉。
- `screen(samples: &[Sample], curve_width: usize) -> String`：终端画面的一屏（不含清屏与光标控制，那是调用方的事）。每个计数器一行，共 13 行，以 `
` 分隔，顺序与 `Sample` 的字段相同：左对齐 24 列的英文标签，右对齐 10 列的最新读数，两个空格，再是这一项最近 `curve_width` 个点的曲线。没有样本时返回空串，调用方在第一秒什么也不画。
- `Unit::{Permille, Bytes, Nanos, Count}` 与 `Unit::reading(&self, value: u64) -> String`：一个读数按单位写成文字，是城里单位换算的唯一一处；`gauge::lines` 写给人的行也调它（8-129-4）。读数的写法按单位定：千分比写成一位小数的百分数（`123` → `12.3%`）；字节按 1024 进位取最大的、读数不小于 1 的单位，写一位小数（`B` 只写整数，其后是 `KiB`、`MiB`、`GiB`、`TiB`）；纳秒按 1000 进位，同样写一位小数（`ns` 只写整数，其后是 `µs`、`ms`、`s`）；计数原样写。小数一律截断而不是四舍五入，整数运算，没有浮点。

**定下的值。** 8 级字符 `▁▂▃▄▅▆▇█`（U+2581–U+2588），与常见终端字体都有的块元素一致。标签列宽 24、读数列宽 10：最长的标签 `event to screen p50` 与最长的读数 `1023.9 GiB` 都放得下。

**决定。**

1. 曲线按窗口内自己的最小值到最大值分级，而不是从 0 分级。监视器看的是变化：一个 3.1 GiB 到 3.2 GiB 之间抖动的工作集，从 0 分级会画成一条平线。代价是两行曲线的高度不能互相比较，读数本身印在曲线旁边。
2. JSON 用 `serde_json` 按字段名写出，而不是手拼字符串。`serde_json` 已是本 crate 的依赖，字段名与 `Sample` 同处一地，手拼会让键名在两处各写一遍。
3. 一屏是一个纯函数返回的字符串，而不是直接写终端的绘制器。交互（按键、窗口尺寸、重画节奏）留在调用方，这一屏的内容才能用整串比较来测。
4. 单位换算写在这里，不借 `runtime::sieve` 的 `size`：那一个只到 `KiB`、属于另一个 crate 的私有实现，而这里还要换算纳秒。
5. WebUI 面板在浏览器里按同样的规则自己算读数与曲线（`client/src/core/monitor.ts`），而不是让城把画好的行随帧发过去。曲线取多少个点取决于面板在屏幕上有多宽，只有浏览器知道；标签要从 `lang.json` 取两种语言，终端这一侧只有英文。代价是分级与单位换算在 Rust 与 TypeScript 各写一遍，两边由同一组样本对照：`client/src/core/monitor.test.ts` 用的样本与期望读数和 `monitor::top::tests` 的一屏测试逐项相同，改规则时两份测试的期望一起改；没有机器门把这两份期望绑在一起。重新考虑的条件：监视帧改为携带已画好的行，或者面板改用非字符的画法。

**测试。** `monitor::top::tests`：一行 JSON 解析回来正好是 13 个键、值等于读数、不含换行；0 到 7 画成 `▁▂▃▄▅▆▇█`，宽度不足时只画最新的几个，全部相等时画 `▁`；两份样本的一屏逐行等于预期的 13 行，没有样本时为空。

## 8-96 采样线程与计数器读取（`bin::monitor::sampler`、`bin::monitor::counters`，形状：状态机 / adapter）

8-94 的 `Monitor` 只回答「有没有人在看」与「看到了哪 300 个点」；每秒调用一次 `tick`、把新读数发给看的会话、以及读数从哪个平台接口来，是这两个模块的事。

**接口。**

- `sampler::beat(monitor: &Mutex<Monitor>, samples: &broadcast::Sender<Sample>, read: impl FnOnce() -> Sample)`：一拍。调用 `Monitor::tick(read)`；这一拍读了计数器，就把这一个读数发到 `samples`（即 `wire::MonitorFeed::samples`）。没人在看时 `read` 不被调用，什么也不发。发送时一个订阅者也没有不是失败：看的会话在两拍之间走了，它没有错过自己要的东西。锁中毒时照常取用：计数是原子的，历史是完整的 `VecDeque`，中毒不留下写了一半的状态。
- `sampler::spawn_sampler(monitor: Weak<Mutex<Monitor>>, samples: broadcast::Sender<Sample>) -> Result<(), AxError>`：起名为 `sprawling-monitor` 的线程，每秒一拍，读数来自 `counters::Counters`。线程只持 `Weak`：`ServeConfig` 连同 `MonitorFeed::watch` 被丢弃后 `upgrade` 失败，线程在下一拍结束。起不了线程时返回 `StorageFatal`，recovery 是检查进程的线程上限（与 `serving::folding` 相同）。
- `counters::Counters::open(volume: PathBuf) -> Counters` 与 `Counters::read(&mut self, watched: Watched, elapsed: Duration) -> Sample`：核心进程的五项取自 `OwnProcess`；`Watched::Everything` 时再读整机 CPU（千分比）、可用内存与城所在卷的剩余空间。城的路径在 `open` 时经 8-116 的 `volume::resolved` 解析一次，之后每拍用解析过的路径找盘：城以相对路径或 verbatim 拼写起来时，没有一个挂载点是原样路径的前缀，卷的剩余空间就一直读作 0。解析不出时退回原样路径。`sysinfo` 的句柄在第一次 `Everything` 读数时才打开，一次 `Summary` 读数把它们丢掉，所以只有摘要在看时它们不常驻。第一次读数没有上一次可比，两项 CPU 为 0。
- `counters::own_process::OwnProcess::new() -> OwnProcess` 与 `OwnProcess::read(&mut self, elapsed: Duration) -> OwnReading`：只问本进程、不遍历进程表的读数，`elapsed` 是距上一次读数的墙钟时间。`OwnReading { cpu_permille, private_bytes, working_set_bytes, read_bytes, written_bytes }`：CPU 是两次读数之间本进程累计 CPU 时间的增量除以墙钟增量与核数（千分比，整数运算，截到 `0..=1000`），第一次为 0；private 在 Windows 上是 PagefileUsage（即 PrivateUsage），其他平台是虚拟内存；工作集是驻留内存；读写字节是本进程累计经存储读写的字节数，Linux 上取自 `/proc/self/io` 的 `read_bytes` 与 `write_bytes`（std 读文件，不需要 unsafe），其他平台读作 0（决定 1）。平台拒绝某一项时这一项读作 0，与尚未接入的项同样处理：`Sample` 是给人看的读数，没有携带失败的位置，而一秒后下一拍会再读一次。`Counters` 只在有人看时存在：`beat` 之后历史为空（没人看）时采样线程丢掉它，平台句柄与进程表不常驻。

**定下的值。** 一拍的间隔 1 s（8-94 的「每秒一点」）；线程名 `sprawling-monitor`。

**决定。**

1. 本进程的读数来自 `memory-stats`（工作集与 private）与 `cpu-time`（本进程 CPU 时间），整机与卷的读数来自 `sysinfo`，关掉默认特性、只开 `system` 与 `disk`。本工作区 `unsafe_code = forbid`，「只取需要的几个接口」在这里只能是另一个把平台调用包成安全接口的 crate，所以比较的是同一个 crate 的两种裁剪，以一个空的 release 探针（`lto`、`codegen-units = 1`、`strip`）实测：在 windows-msvc 的桌面级机器上（同时有别的编译在跑），空探针 123 904 字节，读齐本节这些计数器的探针 201 216 字节，多 77 312 字节（约 75.5 KiB）。启动时间不受影响：`Counters` 只在第一个人开始看时才打开，城的启动路径上没有它；打开一次（`System::new`、首次刷新进程与 CPU、列出磁盘）热缓存 0.42–0.56 s、冷缓存 2.8 s；之后每读一次 18–74 ms，其中本进程的刷新占 17–65 ms（`sysinfo` 在 Windows 上即使只问一个 pid 也遍历整张进程表），整机 CPU 0.7–8 ms，磁盘 0.2–0.8 ms，内存约 5 µs。本进程那一段因此换成只问本进程句柄的 `memory-stats` 加 `cpu-time`：同一类机器上的 release 探针读一次约 1.1 µs，比空探针多 1 024 字节，两者依赖的 `winapi` 与 `windows-sys` 已在依赖树里。代价是本进程的累计读写字节：没有找到以安全接口只读本进程 I/O 计数的 crate。Linux 上 `/proc/self/io` 是一个普通文件，std 读它即可；Windows 的 `GetProcessIoCounters` 只能经 unsafe 调用，所以在出现包好它的安全 crate 之前这两项在 Windows 与其他平台上读作 0，而不是为它们留下每拍 17–65 ms 的整表刷新。默认特性的 `sysinfo` 在 LTO 之后并不更大（没用到的代码被去掉），裁掉特性省的是编译时间与依赖数。重新考虑的条件：出现以安全接口只读本进程 I/O 计数或整机 CPU、比 `sysinfo` 显著更快的 crate，或者 `unsafe_code` 的政策改变。
2. 采样放在一条自己的线程上，而不是 tokio 任务：它每秒做一次阻塞的系统调用，放进异步运行时会占住一个工作线程；它与 `serving::folding` 一样是一条命名线程。没人看时线程每秒醒一次、读一个原子数，不读计数器也不留内存（8-94 决定 1）。
3. CPU 份额的墙钟分母由调用方以 `elapsed` 传入，采样线程传一拍的名义间隔 `BEAT`，而不是在这里读 `Instant::now`：取时间的地方只有 `bin::assembly`（clippy 的 `disallowed_methods` 守着），测试也因此能给出确定的分母。代价是 `sleep` 睡过头的那几毫秒让份额偏高同样的比例（1 s 里多睡 15 ms 就偏高 1.5%），对一条给人看的曲线可以忽略。重新考虑的条件：监视器的读数进入任何决定。
4. 读数经 `Sample` 发出，不在这里换单位：换单位是 8-95 与 `client/src/core/monitor.ts` 的事。
5. `f32` 的整机 CPU 负载是 `sysinfo` 唯一给出的形式，先截到 `0..=100` 再换成千分比；这一处 `as` 以 `#[expect]` 注明，它是本模块唯一的浮点。

**测试。** `monitor::sampler::tests`：有人看时一拍把读到的那一个读数发给订阅者；没人看时不读、不发。`monitor::counters::tests`：读本进程得到非零的工作集、整机可用内存与卷剩余空间；以相对路径 `.` 打开的 `Counters` 读到的卷剩余空间非零。`monitor::counters::own_process::tests`：第一次读数的 CPU 为 0，本进程忙过一段之后第二次读数的 CPU 大于 0，工作集与 private 非零。

**本节接口的当前状态。** 整机可用内存经 8-94 的 `bin::monitor::memory` 读出，与计划推进的内存闸（§8-46-3）读同一处。核心健康里记账队列深度与持久水位线的两项经 8-98 的 `Health` 读出；其余几项现为 0，缺的是来源而不是采样，8-98 的当前状态逐项写明。派出的命令按 run 装进各自的 Job Object，`runtime::Backlog::processes` 给出每个 run 此刻的进程（runtime-SPEC §8-13-3）；按这些 pid 读每个进程的内存与 CPU、并把逐 run 的明细送上线，还没有做：`Sample` 是一行固定的 13 个数，逐 run 的明细要一种新的帧（WIRE_V 加一）。磁盘延迟没有字段。本进程的累计读写字节在 Linux 以外读作 0（决定 1）。一拍里剩下的大头是 `sysinfo` 的整机 CPU（0.7–8 ms）与磁盘（0.2–0.8 ms），离「采样一次 ≤ 50 µs」还差这两项；采样一次 ≤ 50 µs、占 CPU ≤ 0.1% 的仪表尚未落地。

## 8-97 `sprawling gauge --at`：经线协议看监视器（`bin::wire_client::watching`，形状：adapter）

`sprawling gauge [--at host:port] [--token T]`（别名 `top`）与 `call` 一样经 `--at` 找到一座在跑的城（默认同一个 `DEFAULT_AT`），握手后发 `ClientFrame::Monitor(Watch)`，此后每收到一个 `ServerFrame::Monitor` 就输出一次，直到城关闭连接或人按 Ctrl-C。城是由地址找到的，而不是由城名：一座城的地址就是它被 `serve`/`up` 时占的端口，`call` 与 `enrol` 已经这样找城。`bin::main::gauge` 选中 `City` 这个对象之后调本模块（8-129-4）。

**接口。**

- 输出的形式由 `bin::audience::Audience` 定：`Person`（stdout 是终端）画一屏，`Agent` 每个读数一行。
- `shown(text: &str, history: &mut VecDeque<Sample>, audience: Audience, curve_width: usize) -> Option<String>`：收到的一帧文本变成要写到 stdout 的文字。不是监视读数的帧（欢迎、事件等）返回 `None`。读数先放进 `history`（满 `monitor::CAPACITY` 丢最旧的），`Agent` 返回 `gauge::lines::city_line` 加一个换行；`Person` 返回清屏并把光标移到左上角的 `ESC[H ESC[2J`，接 8-95 的 `screen(history, curve_width)` 与换行。`city_line` 失败时返回 `None`（8-95：实际不会出现）。
- `top(at: &str, token: Option<&str>, audience: Audience) -> Result<(), Unheard>`：连接、握手、发 `Watch`、逐帧调用 `shown` 并写出。连不上是 `Unheard::NoCity`（退 4，与 `call` 同一张表）；握手被拒、读帧出错与写 stdout 失败是 `Unheard::Broken`（退 1）；连续 5 s 没有一帧视为城已停，正常返回。
- 曲线宽度：环境变量 `COLUMNS` 能读成数时取它减去标签与读数占的 36 列，否则按 80 列算（44 个点）；标准库不给终端尺寸，不为这一个数引入依赖。

**决定。**

1. 文字由纯函数 `shown` 算出，socket 与 stdout 留在 `top`：一帧变成哪几个字节可以用整串比较来测，而连接只在端到端里测。
2. 沉默 5 s 即结束，而不是永远等：读数每秒一个，5 个空拍说明城已停或已不再发，一个 agent 读到 EOF 比读到永远的阻塞有用。重新考虑的条件：采样的节拍变长。

**测试。** `wire_client::watching::tests`：一帧读数在 `Agent` 下是一行 `city` JSON 加换行，在 `Person` 下是清屏序列接一屏；不是读数的帧什么也不输出、不进历史。

## 8-129 测量工具箱：一个动词 `gauge`，一处读机器，一个单调钟（`bin::main::gauge`、`bin::main::gauge::lines`、`bin::monitor::tree`、`bin::monitor::spread`、`bin::audience`，形状：adapter / projection / value）

开发这份代码用的测量手段——按微秒计的时长、进程与整机的资源读数、样本的分位数——随产品发出：一个动词 `sprawling gauge`，一份随发行包发出的 skill `skills/gauge/`。读它的是 agent：在这座城上做性能工作的，和在别的项目里要量一条命令的。本节定下五件事：每种读数住在哪里；每种读数只在哪一处采样，谁只读；采样本身花多少、由哪个确定性计数守住；动词的参数与输出；skill 教的流程。8-94 至 8-98 讲的监视器照旧，本节只写它们之外的部分和它们要改的地方。

### 8-129-1 三种读数，三个家

| 读数 | 例子 | 住在哪里 | 谁读它 |
|---|---|---|---|
| 城里一件事发生的时刻 | `model_called`、`model_returned`、`tool_called`、`tool_result` 的 `t`（整数毫秒） | Ledger；含义只经 `EventRecord::moment` 读（kernel-SPEC「信封 `t` 记的是什么」） | `view` |
| 这台主机为城的一段内部工作花了多久，以及那段工作的确定性计数 | 开城各段（8-121）、整链审计一遍（8-90）、派活准备（`[prepare_dispatch_ms]`） | 诊断行，每种一个渲染处，按下限写出（`docs/logging.md` §2、§3） | 服务所在的终端、记录页的 log 透镜、`bench_startup` 留下的日志 |
| 机器的资源被用了多少 | CPU、private、工作集、读写字节、卷剩余空间、进程树 | 城内：监视器的 300 点历史（8-94），有人看才采样；城外：`gauge` 的 stdout | `gauge`、WebUI 监视页 |

第三种读数不常驻盘上。要一段后台记录，就让一个看的人把它写出去：`sprawling gauge --at <地址> > readings.jsonl` 在后台跑多久，城就记多久；它是一个看整页的人，停下它，采样就停（8-94 决定 1 不变）。

### 8-129-2 一个权威：谁采样，谁只读

| 事实 | 唯一的采样处 | 只读的一方 | 并进来的第二份 |
|---|---|---|---|
| 进程、整机、卷、进程树的计数 | `bin::monitor`：`counters`、`counters::own_process`、`memory`、`volume`、`tree` | 采样线程（8-96）、计划推进的内存闸（§8-46-3）、入口按卷卸载（8-116）、`gauge` | 无；`xtask mem` 留在城外，见决定 4 |
| 时长 | `bin::serving::standing::monotonic_now`，以 `fn() -> Instant` 交给用它的模块；记账一侧经 `accounting::worker::Hands.monotonic` 收下 | `OpeningCost`（8-121）、整链审计（8-90）、视图切快照的节奏（8-99）、采样线程量自己一拍的读取用时、`gauge` 的 `wall_us` 与 `read_cost_us`、`stage_dispatch` 与 `Laying::mcp_tools` 的用时（`[prepare_dispatch_ms]`）、`credentials::probing` 的 `elapsed_ms`（8-62） | 无；后三处的单位与线上字段都还是毫秒 |
| 时刻 | 驱动的 `RunHooks::now`，只在串行阶段采样 | Ledger 的 `t` | 无 |
| 计数 | 由拥有它的模块自己数：`Health` 由 relay 数（8-98），视图积压由视图线程数（8-123），开城时核对的行数、读过与哈希过的字节由开城路径自己数（8-122） | 采样线程读 `Health` 与积压；开城的计数随 8-121 那一行写出 | 无 |
| 分位数 | `bin::monitor::spread::Spread` | `gauge`、citysim 的 `bench`（`Reading::of`）与 `bench_startup`（`Samples::of`）、crate 内的仪表（8-84、8-99、`instrument_monitor_beat`） | 无；两处 citysim 读法都经 `Spread`，取最近秩（决定 8） |
| stdout 那头是谁 | `bin::audience::Audience::of_stdout` | `gauge`、`wire_client::watching`（8-97）、`view`（8-105） | 无；`main::view` 留着自己的同名枚举（它还带写一行的方法），它的 `of_stdout` 只把库里的判定换成本地的臂 |

时长只取单调钟，理由与 8-121「时间从哪来」相同：墙钟被人调过，一段就可能是负数；城钟又只到毫秒，而人的定规要内部热路径按微秒读。计数不是时间，所以可以在 accounting 与 storage 里累计，不必经过 `bin::assembly` 的采样点。

### 8-129-3 采样花多少，由什么守住

每一拍读什么，由看的人决定，并且只有下面这几种读法：

| 谁在看 | 每拍的平台读取 | 目标（每拍） | 现状 |
|---|---|---|---|
| 没有人 | 0：不建 `Counters`，只读两只原子数 | 0 | 已如此（8-94） |
| 只看摘要 | 本进程 1 次（`memory-stats` 加 `cpu-time`），外加 `Health` 与积压的几次原子读 | ≤ 50 µs | 本进程约 1.1 µs（8-96 决定 1） |
| 看整页 | 本进程 1 次，整机 1 次（CPU、内存、磁盘列表） | ≤ 50 µs；上限 1 ms，即 1 Hz 下一个核的 0.1% | 整机 CPU 0.7–8 ms、磁盘 0.2–0.8 ms，超出上限 |
| `gauge` 量一棵进程树 | 进程表 1 次，在 `gauge` 自己的进程里 | 上限：每拍至多一次整表读取，拍距不短于 250 ms | Windows 上一次 17–65 ms（8-96 决定 1） |

城的采样线程没有一条路径去遍历进程表：整表读取只在 `monitor::tree` 里，城的采样线程不调用它。

- **门是计数。** `Counters` 与 `tree::Tree` 各自数平台读取的次数，以一个 `Reads { own: u64, machine: u64, table: u64 }` 交出。测试按看的人走 N 拍，断言三项的精确值：没有人是 0/0/0，只看摘要是 N/0/0，看整页是 N/N/0，`gauge` 的进程树是 0/0/N。这几条断言是 CI 的回退门，与 `index_refresh_read` 同一种做法：它们与机器快慢无关，而共享 CI 机器上的墙钟噪声会让一个墙钟门时红时绿。
- **墙钟只记录。** `instrument_monitor_beat`（`#[ignore]`，`just bench` 按 `/::instrument_/` 跑它）对每种读取各采 200 次，经 `Spread` 印出 floor、p50、p99（µs），读数登记到 `budgets.toml` 的新行 `[monitor_beat]`，不设门，理由与 `[local_latency]` 相同。
- **线上看得见开销。** 采样线程用单调钟量这一拍的读取用了多久，下一拍的 `Sample.read_nanos` 带上它，所以在看的人从同一条读数流里就能看到采样本身花了多少。
- 看整页时超出上限的那部分（`sysinfo` 的整机 CPU 与磁盘列表）仍是性能上的待办，重开条件见 8-96 决定 1。

### 8-129-4 `sprawling gauge`

```text
sprawling gauge [--at <addr>] [--token <t>]                                一座服务中的城（别名 top）
sprawling gauge --pid <pid> [--every <ms>] [--samples <n>]                 一个在跑的进程和它的子孙
sprawling gauge [--every <ms>] [--samples <n>] -- <program> [<arg>...]     一条命令，跑 n 次
```

```rust
// bin::main::gauge —— shape: adapter
pub(super) fn verb(read: &Arguments) -> ExitCode;
pub(super) enum Subject {
    City { at: String, token: Option<String> },
    Process(Watching),
    Command(Running),
}
pub(super) struct Watching { pid: u32, every: Every, beats: Option<NonZeroU32> }  // beats 缺省：直到根进程退出
pub(super) struct Running { program: OsString, args: Vec<OsString>, every: Every, samples: NonZeroU32 } // samples 缺省 1；程序名单独一个字段，空命令写不出来
pub(super) fn subject(read: &Arguments) -> Result<Subject, Misread>;
pub(super) enum Misread {                   // 每一臂退 2，Display 写出最近的合法写法
    TwoSubjects,                            // --pid 与 -- 同时出现
    NotForThisSubject { flag: &'static str, subject: Measured },
    OutOfRange { flag: &'static str, given: String, range: &'static str },
    NothingAfterDashes,
}
pub(super) enum Measured { City, Process, Command } // Misread 说出是哪个对象，并写出它的合法写法
pub(super) struct Every(Duration);          // 250 ms ..= 60 s，缺省 1 s
pub(super) struct Run { index: u32, exit: Option<i32>, wall: Duration, watched: Watched, child: ChildPeaks }
pub(super) struct Watched { beats: u64, seen: Option<Seen>, read_cost: Duration } // 拍线程交回的东西
pub(super) struct ChildPeaks { private_bytes: Option<u64>, working_set_bytes: Option<u64> }
pub(super) struct Host { cores: NonZeroUsize, physical_bytes: u64 } // available_parallelism 与 monitor::memory::read
// bin::main::gauge::running —— shape: adapter；一条命令跑 n 次：spawn 到 wait 的单调钟用时、拍线程 sprawling-gauge、子进程峰值
pub(super) fn run(running: &Running, audience: Audience, out: &mut impl Write) -> Result<(), AxError>;
// bin::main::gauge::lines —— shape: projection；每种行只在这里变成文字
pub(crate) fn city_line(sample: &Sample) -> serde_json::Result<String>; // {"line":"city", 其后是 Sample 的字段}；watching 调它
pub(super) fn tree_line(at: Duration, reading: &TreeReading, audience: Audience) -> String;
pub(super) fn run_line(run: &Run, audience: Audience) -> String;
pub(super) fn spread_line(spread: &Spread, failed: u32, host: Host, audience: Audience) -> String;
// bin::monitor::tree —— shape: adapter；pub，因为读它的 gauge 在二进制那一半
pub struct Tree;                            // 私有：sysinfo::System、根 pid、每个见过的 pid 的上一读数、各峰值、读取次数
impl Tree {
    pub fn open(root: u32) -> Tree;
    pub fn read(&mut self, elapsed: Duration) -> Option<TreeReading>; // 根已不在：None
    pub fn seen(&self) -> Option<Seen>;     // 一拍都没读到：None
    pub(crate) fn reads(&self) -> Reads;
}
pub struct TreeReading { pub processes: u64, pub cpu_permille: Option<u64>, pub private_bytes: u64, pub working_set_bytes: u64, pub read_bytes: u64, pub written_bytes: u64 }
pub struct Seen { pub cpu_ms: u64, pub read_bytes: u64, pub written_bytes: u64, pub peak_private_bytes: u64, pub peak_working_set_bytes: u64 }
// bin::monitor::counters
pub(crate) struct Reads { pub(crate) own: u64, pub(crate) machine: u64, pub(crate) table: u64 }
impl Counters { pub(crate) fn reads(&self) -> Reads; }
// bin::monitor::spread —— shape: value
pub struct Spread { /* 私有：samples、floor、p50、p95、p99、peak、suspicious */ }
impl Spread {
    pub fn of(head: Duration, tail: impl IntoIterator<Item = Duration>) -> Spread; // 头是参数，空集写不出来
    pub fn p(&self, share: Share) -> Duration;  // 最近秩：第 ⌈n·p/100⌉ 个（一起）
    pub fn floor(&self) -> Duration;  pub fn peak(&self) -> Duration;
    pub fn samples(&self) -> u64;     pub fn suspicious(&self) -> u64; // 超过 p50 的 SUSPICIOUS_TIMES 倍
}
pub enum Share { P50, P95, P99 }
pub const SUSPICIOUS_TIMES: u32 = 3;
// bin::audience —— shape: value；库里的模块，二进制的 view、watching、gauge 都读它
pub enum Audience { Agent, Person }
impl Audience { pub fn of_stdout() -> Audience; } // stdout 是终端即 Person
// 为二进制那一半打开的三处：采样点、读内存、单位换算各只有一份
pub fn bin::serving::standing::monotonic_now() -> Instant;
pub fn bin::monitor::memory::read() -> Memory;
pub enum bin::monitor::top::Unit { Permille, Bytes, Nanos, Count }  impl Unit { pub fn reading(&self, value: u64) -> String; }
// bin::wire_client::watching
pub(crate) fn top(at: &str, token: Option<&str>, audience: Audience) -> Result<(), Unheard>; // 连不上是 Unheard::NoCity
// bin::main::verbs / grammar
pub(super) enum AfterDashes { Refused, Command } // Row.after_dashes：这个动词收不收 `--` 之后的词
impl Arguments { pub(super) fn after_dashes(&self) -> Option<&[String]>; } // 没有 `--`：None；`--` 后无词：Some(&[])
// accounting::worker::Hands（accounting-SPEC 8-11）
pub monotonic: fn() -> Instant,             // 生产交 monotonic_now；派活准备与 probe 的用时都读它
```

**选哪个对象。** 给了 `--pid` 是 `Process`，`--` 之后有词是 `Command`，都没有是 `City`。命令行错误退 2（8-103 的 `Line`），一行说明哪里错、最近的合法写法是什么：`--pid` 与 `--` 同时出现；`--at`、`--token` 配 `Process` 或 `Command`；`--every`、`--samples` 配 `City`（城的拍距由城定，8-96）；`--every` 在范围外或不是整数；`--samples` 为 0 或不是整数；`--` 之后没有词。

**`--` 之后原样交出。** 第一个 `--` 结束 sprawling 自己的参数，之后每个词原样成为被量命令的 argv，不当作标志读。8-89 的优先规则（`--help`、`--version` 出现在任何位置都先生效）只扫描到第一个 `--` 为止，所以 `sprawling gauge -- cargo --version` 量的是 cargo。命令表每行多一项：这个动词收不收 `--` 之后的词，只有 `gauge` 收。

**一次 run 从哪里量到哪里。** 从 `spawn` 之前到 `wait` 返回：可观察的端点是看到进程退出（citysim-SPEC §3-2）。`wait` 在调用线程上阻塞，拍在一条名为 `sprawling-gauge` 的线程上走，所以一次 run 的结束由 `wait` 看到，`wall_us` 没有一拍那么大的误差。被量命令的 stdin 是空的；它的 stdout 与 stderr 都接到 `gauge` 的 stderr（`Stdio::from(io::stderr())`），`gauge` 的 stdout 上只有读数行。n 次 run 依次跑，第一次不丢：它是冷的那一次，`run` 行的 `index` 为 0，spread 里 floor 与 p50 的差有一部分是它留下的，其余是机器的负载。

**给 agent 的输出（stdout 不是终端）。** 每行一个 JSON 对象，值只有整数、`null` 与 `line` 这一个字符串键，键序固定，单位写在键名末尾（`_us`、`_ms`、`_bytes`、`_permille`），没有单位的是计数。没有量到的值是 `null`，不是 0。

- `City`：每秒一行 `{"line":"city", …}`，其余键是 `Sample` 的字段名（8-95 的 `json_line`，多出 `line` 一个键）。`Sample` 没读到的项仍按 8-96 读作 0。
- `Process`：每拍一行 `{"line":"tree","at_us":…,"processes":…,"cpu_permille":…,"private_bytes":…,"working_set_bytes":…,"read_bytes":…,"written_bytes":…}`，是这一拍还活着的整棵树。第一拍没有上一拍可比，`cpu_permille` 为 `null`；CPU 是两拍之间整棵树的 CPU 时间增量除以墙钟增量与核数，截到 `0..=1000`。根进程退出、或走满 `--samples` 拍，就结束。
- `Command`：每次 run 结束一行 `{"line":"run","index":…,"exit":…,"wall_us":…,"beats":…,"seen_cpu_ms":…,"seen_read_bytes":…,"seen_written_bytes":…,"seen_peak_private_bytes":…,"seen_peak_working_set_bytes":…,"child_peak_private_bytes":…,"child_peak_working_set_bytes":…,"read_cost_us":…}`，最后一行 `{"line":"spread","samples":…,"failed":…,"floor_us":…,"p50_us":…,"p95_us":…,"p99_us":…,"peak_us":…,"suspicious":…,"cores":…,"physical_bytes":…}`。`exit` 是被量命令的退出码，被信号结束时为 `null`；`failed` 是 `exit` 不为 0 的 run 数。`seen_*` 取自拍：CPU 与读写字节是每个在某一拍出现过的进程最后一次读数之和，两项峰值是各拍整棵树之和里最大的那一拍，所以它们都是下界，两拍之间生灭的进程不在内；这次 run 一拍都没走（比 `--every` 短）时 `beats` 为 0，`seen_*` 全为 `null`。`child_peak_*` 是直接子进程由平台记下的峰值：在 Windows 上 `wait` 之后、句柄关闭之前经 `win32job::utils::get_process_memory_info` 读出，是精确值而不是拍上的读数；其他平台为 `null`。`read_cost_us` 是这次 run 里读进程表花掉的时间，agent 据此判断测量本身扰动了多少。`cores` 与 `physical_bytes` 写出机器的等级，不写机器是谁。

**给人的输出（stdout 是终端）。** `City` 是 8-95 的一屏。`Process` 每拍一行，`Command` 每次 run 一行、最后一行 spread，读数按 8-95 的单位规则写（`µs`、`ms`、`MiB`，一位小数，截断）。单位换算只有 `monitor::top` 那一份，`lines` 调它。

**退出码**（8-103 的 `Exit`）：0，测量做完了，被量命令失败也是 0，它的退出码是 `run` 行里的数据；1，起不了被量的程序（找不到程序是 `E_PATH_NOT_FOUND`，subject 是程序名，recovery 是检查 PATH 或写全路径；其余起不来是 `E_TOOL_UNAVAILABLE`，带平台给的原因），或者没有这个 pid 的进程（`E_INVALID_ARGS`，recovery 是给一个在跑的进程的 id）；2，命令行读不懂；4，`City` 的 `--at` 那里没有城（与 8-97 相同）。`City` 在连续 5 s 没有读数时正常结束（8-97 决定 2）。

### 8-129-5 skill `skills/gauge/`

`skills/gauge/SKILL.md`，许可 MPL-2.0，列进 `skills/README.md` 的表与 `skills/LICENSES.md`；发行包按目录收 `skills/`，不必另列。它教 agent 按这个次序做：

1. 先把负载钉住：同一份输入、同一个构建（产品的 feature 集，citysim-SPEC §3-9）；输入是文件时记下它的摘要，两条读数只在摘要相同时可比（citysim-SPEC §3-8）。
2. 取基线：`sprawling gauge --samples 20 -- <命令> > before.jsonl`，读最后那行 spread。floor 贴着设计的下限，p50 带着机器其余的负载（citysim-SPEC 8-6）；`suspicious` 不为 0 时先看是哪几次、是不是第一次。
3. 一次只改一处，取 after；前后交替跑几轮（一轮先 before 后 after），只在同一机器等级上比较 floor 与 p50。
4. 回退门用确定性计数，墙钟只记录：找一个随规模增长的计数（读过的字节、核对的行数、文件操作数），在 N 与 2N 两种规模下断言它。
5. 资源：长命令看 `run` 行的 `seen_*` 与 `child_peak_*`；一个已在跑的进程用 `--pid`；一座服务中的城用 `gauge --at <地址> > 文件` 在后台记录。`city` 行里的 0 可能是「没读到」（8-96），其余行里没读到的是 `null`。
6. 在 sprawling 自己身上：一次调用花多久，读 `view` 给出的 `tool_called` 与 `tool_result`、`model_called` 与 `model_returned` 的 `t`（账本版本 2 起才是这一行自己的时刻）；开城各段读 `serve` 写出的 `opened the city in` 那一行；派活准备打开 `--log trace`。人能感知的操作落在毫秒、内部热路径落在微秒，以秒计的读数就是要做的活（人的定规）。
7. 报读数时写机器等级（核数、内存、盘的种类）和负载的摘要，不写机器名、账户名和路径（AGENTS.md *Privacy*）。

### 8-129-6 对 wire 的需求（wire-SPEC 8-47）

- `Sample.view_backlog: u64`：视图积压的条数，读 8-123 给出的读法：`spawn_sampler` 收下 `bin::serving::folding::Folding` 交出的 `Backlog` 句柄，每一拍读一次 `records()`。
- `Sample.read_nanos: u64`：上一拍的读取用时，由采样线程用单调钟量出：`spawn_sampler` 收下一只 `fn() -> Instant`（生产交 `serving::standing::monotonic_now`），在读计数器前后各取一次，差值填进下一拍；第一拍为 0。
- 加上这两项，`Sample` 是 15 个 `u64`，历史 300 × 15 × 8 = 36 000 字节，仍在 `HISTORY_BUDGET`（64 KiB）之内，编译期断言不必改。
- `monitor::top::screen` 随之多两行（`view backlog` 记录条数、`read` 微秒）；WebUI 监视页的两行随页面的改动做。
- 不要求：`relay_p50_nanos`、`event_to_screen_p50_nanos`、`queued_runs` 的来源（8-98 的当前状态）。

**决定。**

1. **读数不进 Ledger，时刻除外。** 时刻是城的历史，已经写在每行的 `t` 里；时长与资源读数是运行这座城的主机在那一刻的事实，与 8-121 决定 2、`docs/logging.md` §2 同一条界线。被否：给资源读数一个新事件种类，按整数单位经单写者写入——1 Hz 一天 86 400 行，每行都要过 relay 与记账线程的屏障，而记账线程上的长任务正是 8-123 要设界的；整链审计、重放、快照与每个折叠都随它变长；读数进了哈希链就删不掉，而资源读数随时可丢。重开条件：某个城内的决定要读资源的历史（今天的决定都只读此刻：内存闸与卷卸载）。
2. **城不在自己的保留子树里常驻记录。** 被否：在城里放一份只追加的读数文件，一直写。那是城树里的第二个写者，要为它设计保留与轮转（轮转归操作系统，`docs/logging.md` §7），没人看的城每天多写 86 400 行，违背 8-94 决定 1；在别的项目里量一条命令的 agent 手里也没有城。后台记录由一个看的人写出（8-129-1）。重开条件：人要看一座在没人看时崩溃的城在崩溃前的资源曲线。
3. **一个动词，收下 `top`。** `top` 成为 `gauge` 的别名，`sprawling top` 的行为不变，只多出 `line` 键。被否：在 `top` 旁边再加一个动词——两个动词读同一条读数流，参数与输出要在两处维持；也被否：测量手段只留在 `xtask` 与 citysim——它们不随发行包走，在别的项目里的 agent 手里什么也没有。名字取 `gauge`：`measure` 已是 browser 工具的一个动作（量元素的框，glossary 的 **browser**），同一个词会担两个概念；`bench` 是开发者跑固定场景的配方 `just bench`。
4. **进程树经 `sysinfo` 整表读取，`xtask mem` 留在城外。** 以安全接口读直接子进程以外的进程，`sysinfo` 是依赖树里唯一的路，它在 Windows 上每次都遍历整张进程表（17–65 ms），所以 `gauge` 的拍距不短于 250 ms，每拍至多一次（8-129-3）。被否：把被量命令放进一个 Job Object，读它的累计 CPU、读写字节与峰值——要 `QueryInformationJobObject`，`win32job` 的安全接口只给 pid 列表与限额；工作区 `forbid` 了 `unsafe`，Zig 叶子也要一处 `extern` 的 `unsafe`，而 `forbid` 在 crate 内提不起来。`xtask mem` 读 PowerShell 的 `PeakPagedMemorySize64`（峰值 commit），`sysinfo` 不给峰值，所以它不改为调用 `gauge --pid`。重开条件：出现以安全接口给出 Job Object 累计计数、或任意 pid 的峰值 commit 与读写字节的 crate；那时 `seen_*` 换成精确值，`xtask mem` 改为调用 `gauge --pid <pid> --samples 1`。
5. **`gauge` 不转述被量命令的退出码。** `Exit` 是退出码的唯一定义（8-103）；把子进程的 3 或 4 原样退出，会被读成「城没回话」「没有城」。被量命令的退出码在 `run` 行里，失败的次数在 spread 行里。
6. **被量命令的 stdout 接到 `gauge` 的 stderr。** 被否：继承 stdout——它的输出会和读数行交错，agent 解析不了；丢弃——失败的 run 就没有诊断可看。
7. **没量到写 `null`。** agent 会把 0 当成一个读数。`city` 行照旧沿用 `Sample` 的 0（8-96），skill 写明这一点；让 `Sample` 的每一项都能为空是线协议的改动，本节不要求。
8. **分位数取最近秩。** 登记册的 `[install]` 一行已写明「p50/p95/p99 nearest-rank」，`bench_startup` 的 `Samples::of` 就这样算；`bench` 的 `Reading::of` 改为经 `Spread`，它登记在 `budgets.toml` 里的各行要在新规则下重取。被否：两份各留各的——同一个 p50 在两份读数里指的是不同的样本，读数行上看不出这个差别。
9. **开销用确定性计数守，墙钟只记录**（8-129-3）。被否：给一拍的用时设墙钟门——共享 CI 机器上它时红时绿，而拍的读取次数在任何机器上都一样。
10. **没有 Lean 模型。** 采样线程的读法由看的人一次选定，`gauge` 的 run 依次进行，两处都没有交错；要守的性质是读取次数，由 8-129-3 的计数断言与 `Subject` 这个穷尽枚举守住。重开条件：`gauge` 同时量多个对象，或采样改为多线程。
11. **`gauge` 自己的命令行错误是它自己的枚举 `Misread`，不并进 `grammar::LineError`。** `LineError` 只说命令表判得了的事：动词、标志、位置参数各认不认得；「`--pid` 与 `--` 不能同时给」「`--every` 只配进程与命令」是 `gauge` 选对象的规则，放进 `LineError` 就要让解析器知道一个动词的对象怎么选。两者退出码相同（2），写法都是一行说明加最近的合法写法。被否：给 `LineError` 加通用的「冲突」「越界」臂——它们只有 `gauge` 一个读者，说明文字又各不相同。
12. **记账一侧的单调钟经 `Hands.monotonic` 交进来，是一只 `fn() -> Instant`。** 与 `read_memory`、`read_volume` 同形：worker 碰这台电脑的每只手都在构造时一次交进来（accounting-SPEC 8-11），生产交 `monotonic_now`，测试的 `Hands` 交测试夹具里同样读单调钟的那一只。被否：给 `accounting::Clock` 加一个返回 `Instant` 的方法——城钟是 Ledger 的 `t` 的来源，单调钟只量时长，两者放进一个端口，换城钟（测试里拨快、拨停）的地方就得同时编一个单调钟；四个实现都要改。

**测试。** `monitor::counters::tests`：按看的人走 N 拍，`Reads` 等于 8-129-3 的精确值。`monitor::spread::tests`：n 为 1、2、100、200 时各分位与手算的最近秩相等，`suspicious` 数对。`monitor::tree::tests`：起一个子进程，第一次读数 `processes ≥ 1`、工作集非零、`cpu_permille` 为 `null`，它退出后 `read` 为 `None`，`reads().table` 等于读的次数。`audience::tests`：测试运行器的 stdout 不是终端，读作 `Agent`。`main::grammar::tests`：`--` 之后的词原样交出，其中的 `--help`、`--version` 不生效；`gauge` 与 `top` 解析成同一个动词。`main::gauge::tests`：每条命令行错误退 2；三种行按整串比较；`--samples 3` 量一个很快退出的程序，stdout 恰好三行 `run`、一行 `spread`，程序以非零退出时 `gauge` 仍退 0、`failed` 为 3。citysim：`Reading::of` 与 `Samples::of` 的既有测试按最近秩改期望。accounting：城钟每读一次拨快一分钟时，派活准备、`mcp_tools` 与 probe 的用时仍小于一分钟。

**本节接口的当前状态。** 8-129-1 至 8-129-6 的接口都已落地：`Spread` 是唯一的分位算法，citysim 两处读它；`Counters` 与 `Tree` 数自己的平台读取；`gauge` 是命令表的一行，`top` 是它的别名，`--` 之后的词原样交给被量的命令；`bin::audience` 是 TTY 规则唯一的一处；三处时长读 `Hands.monotonic`；`skills/gauge/` 随发行包走。`Sample` 带上 `view_backlog` 与 `read_nanos`（8-129-6）；`instrument_monitor_beat` 的读数登记在 `budgets.toml` 的 `[monitor_beat]`，`[prepare_dispatch_ms]` 的 `measured_by` 写的是单调钟。还没有的：`main::view` 的同名枚举换成 `bin::audience::Audience`（它的 `write_line` 方法要随之搬走）。`xtask mem` 仍读 PowerShell（决定 4）。

## 8-98 核心健康：记账队列与持久水位线跨线程可读（`accounting::worker::health`，形状：数据）

`Sample` 里核心健康的两项由记账线程与 lane 在各自的线程上改，由采样线程（8-96）读：`ledger_queue_depth` 是 lane 经 relay 交给记账线程、还在队列里没被取走的 append 条数；`durable_lag` 是记账线程已取进一批、这批的磁盘屏障还没返回、所以还没有答复的条数。两项都以记录条数计。它们的属主是 `accounting::worker::relay`：只有它知道一条 append 何时进队、何时被取、何时变得持久；本模块只给它一处能跨线程读的地方。

**接口。**

- `Health`：`Clone`，同一只 `Arc` 里的两个 `AtomicU64`；克隆是同一份计数的另一个句柄。
- 写（只由 `accounting::worker::relay` 调）：`asked(&self)`——一条 append 进队之前；`withdrawn(&self)`——进队失败（记账线程已不在），收回那一次 `asked`；`taken(&self, n: u64)`——记账线程取出 `n` 条放进一批，队列减 `n`、未持久加 `n`；`answered(&self, n: u64)`——这一批的屏障返回、答复发出，未持久减 `n`。
- 读：`read(&self, into: Sample) -> Sample`——把此刻的两项填进 `into`，其余字段原样。
- `RelayGate::open()` 开一份新的 `Health`，`RelayGate::health()` 交出它的句柄；worker 起好时把 `Flight` 里那只 gate 的句柄经 `Started.health` 交给 `listening`，`listening` 交给采样线程：`spawn_sampler(monitor, samples, volume, health)`，每一拍的读数都经 `Health::read`，摘要与整页都有，因为读它只是两次原子读。
- 没有失败路径。减法饱和：计数是给人看的读数，两条线程各自的加减在某一刻读到的先后可以错开，饱和让一个瞬间的错位读成 0 而不是一个巨大的数。

**决定。**

1. 计数放在 relay 进队、取出、答复这三处，而不是去数 `mpsc` 队列的长度：标准库的 `mpsc` 不报长度，而且队列里还有 claim、回家的 run 与命令，它们不是 append。被否：记账线程每一轮开头把自己看到的队列长度写进一个原子数——它只在记账线程醒着时更新，而队列最长的时候正是记账线程在写盘、没醒的时候。
2. 计数的类型放在 `bin::monitor`，由 assembly 持有并写入：依赖朝 assembly → monitor，与 `monitor::memory` 相同；monitor 不点名 assembly。
3. 用 `Relaxed` 次序：两项互不约束，读数不参与任何决定（与 8-96 决定 3 同一个重开条件）。

**测试。** `accounting::worker::relay::tests` 的 `the_accounting_queue_counts_what_waits_and_what_is_not_yet_durable`：一条 lane 的 append 进队后，记账线程服务之前读到队列 1、未持久 0；服务之后两项都回到 0，append 拿到了答复。

**本节接口的当前状态。** `Sample` 余下的核心健康字段仍读作 0：`relay_p50_nanos` 与 `event_to_screen_p50_nanos` 要一个按次记录往返时间的直方图，属主分别是 relay 与 socket 的发送一侧，都还没有；`queued_runs` 没有一处权威的计数——计划行在 `Flight::full` 为真时停在 `pursue` 的循环外，没有被数进任何队列，人派的 run 在满时的去向见 §8-46-3。S5.9M 的降级状态没有 `Sample` 字段。

## 8-123 记账线程不写可丢的投影，视图积压有读数也有界（`bin::serving::folding`，形状：adapter；`accounting::worker::attend`）

```rust
// bin::serving::folding
pub(crate) struct Folding {
    pub(crate) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(crate) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(crate) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,
    pub(crate) keep_slices: Box<dyn FnOnce(storage::Sessions) + Send>, // 会话切片交给视图线程
    pub(crate) thread: std::thread::JoinHandle<()>,
}
#[derive(Clone, Default)]
pub(crate) struct Backlog { /* Arc<AtomicU64> */ }
impl Backlog {
    pub(crate) fn records(&self) -> u64;   // 已送进视图线程、还没折完广播的已提交记录条数
}
pub(crate) const CUT_WAITS_ABOVE: u64 = 256; // 积压多于这个数，服务中的切快照往后放
// accounting::worker::attend
impl RunWorker {
    pub fn hand_off_session_slices(&mut self) -> Option<storage::Sessions>; // 把会话切片交出去（storage-SPEC 8-24）
}
```

**记账线程只做记账。** 记账线程是所有 lane 共用的一条线程（8-42-4）：它每多做一件与落账无关的事，每条 lane 的下一次 append 就多等一段。会话切片（storage-SPEC 8-24）是给人翻看的投影，原先在记账线程上写：每一波落盘之后逐条 `absorb`，缺文件时整份重建并 `sync_data`，进程里第一次缺文件时 `first_seen` 还要把全部段读一遍。服务中的城在写者起好之后把切片交给视图线程：写者线程在接上观察者之前调 `hand_off_session_slices`，把拿到的 `storage::Sessions` 经 `keep_slices` 送进视图线程的通道；视图线程此后每批折完、广播完，再按账本序逐条 `absorb`。切片写不成照旧写一行 stderr、跳过，与它在记账线程上时一样。于是一次 append 在记账线程上的代价只剩这一波本身：组帧、写、`sync_data`、观察者的一次 `send`。**被否：另起一条线程只写切片。** 视图线程已经按账本序收到每一条已提交记录，另起一条就是第二条通道、第二份次序；切片在视图线程上比账本晚一个批次，这是投影可以有的落后。**被否：切片留在记账线程上，只把 `first_seen` 换成读索引。** 缺文件时整份重建的那次读写与 `sync_data` 仍在记账线程上。

**积压有读数。** `Backlog` 是一个原子计数：观察者在 `send` 之前加一，视图线程广播完一批之后减去这批已提交记录的条数，所以任何时刻读到的是「写者已写下、页面还看不到」的条数。读法是 `bin::serving::folding::Backlog::records`，今天它唯一的读者是下一段的界；波次 4 的 W 车道把它交给采样线程，填进 `Sample.view_backlog`，本节不改线格式。加减饱和：两条线程的加减在一次读里可以错开。

**积压有界：追得上之前不切快照。** 服务中切视图快照（8-91 的 `Cadence`）是视图线程上最长的一件事：编码整份视图、写盘、`sync`。积压多于 `CUT_WAITS_ABOVE` 条时，到期的切快照往后放，视图线程先把积压折完、发布、广播；积压回到界内之后的下一批再切。通道关闭时的最后一次切快照不看积压。界取 256 条：视图线程一批就是一次醒来时通道里已到的全部，积压多过几百条说明它已经落后于写者，这时再花一次整份编码只会让页面更晚看见新记录；这个数由 `instrument_view_backlog`（8-99）在突发里读到的 `max_backlog` 复核。**被否：积压过多时让观察者阻塞写者。** 那把视图线程的落后转嫁给每一条 lane，而视图只是投影。

**测试。** `serving::folding::tests` 的 `a_cut_waits_while_the_fold_is_behind`：同一个到期的 `Cadence`，积压在界内时交出要切的那条记录，积压超过界时不交出。storage 的 `sessions::tests::a_ledger_that_handed_off_its_slices_files_nothing_itself`：交出切片之后写者追加的记录不落切片，拿到切片的一方 `absorb` 之后落下。

**本节接口的当前状态。** `resume`、`fork`、`adopt` 与一次性命令没有视图线程，切片仍在它们的写者线程上写（storage-SPEC §3 第 6 条）。记账线程上其余的长任务——派活时起名字、读计划——各在自己的节里（8-42-4、8-86）。

## 8-116 城所在卷快满时不接新活（`bin::monitor::volume`，形状：adapter；`accounting::worker::commanding::shedding`，形状：decision）

kernel-SPEC 8-74 的 `degradation::admit_work` 判定卷低于地板时不接新活；本节给它生产的读数和生产的入口。读数不取监视器的 `Sample`：监视器只在有人看时采样（8-94 决定 1），而不接新活不能取决于此刻有没有人开着监视页。

**接口。**

- `volume::space(disks: &sysinfo::Disks, city: &Path) -> Option<kernel::degradation::VolumeSpace>`：挂载点是城路径最长前缀的那块盘的剩余空间与总容量；没有一块盘的挂载点是它的前缀时为 `None`。「城在哪块盘上」只有这一个家：8-92 的 `Counters` 读卷的剩余空间也经它。
- `volume::resolved(city: &Path) -> Option<PathBuf>`：先以 `std::fs::canonicalize` 把城的路径解析成真实路径（跟随符号链接与 junction），失败时退回 `std::path::absolute`；在 Windows 上再把 verbatim 盘符前缀 `\\?\C:` 还原成普通盘符前缀 `C:`。「城的路径拿什么拼写去比挂载点」只有这一个家，入口与监视器的 `Counters` 都经它。
- `volume::read(city: &Path) -> Option<kernel::degradation::VolumeSpace>`：经 `resolved` 解析城的路径，然后列出一次盘、调用 `space`。生产的入口经它读卷。不解析链接，指向另一块盘的城会读到放链接的那块盘；不补全相对路径或不还原 verbatim 前缀，没有一个挂载点是它的前缀，读作 `None`，盘满时照常接活。
- `RunWorker` 持有一个 `fn(&Path) -> Option<VolumeSpace>` 的读卷函数，生产时是 `volume::read`。人发来的 `Dispatch` 在命名房间、写下任何东西之前读一次卷并调用 `admit_work`；拒绝时回 `BackpressureShed`，subject 是城的根目录，recovery 给出至少要腾出的字节数（`Recovery::FreeDiskSpace`）。读不到卷（`None`）时照常接活：读不到不等于盘满，拒活要有读数作依据。

**决定。**

1. 在入口读卷，而不是由一条常驻线程每秒读一次再把最近的读数放在入口：受理新活是人一次次发来的，一次读卷（`sysinfo` 列盘加刷新）远小于随后一次模型调用；常驻读取让每座没人发活的城都付一份开销。代价是 `sysinfo` 只会列出全部的盘，一次读卷也问到与城无关的盘；只问城所在的那一块，要一个以安全接口给出单个路径剩余空间与容量的 crate，它还不在依赖树里。重新考虑的条件：实测一次读卷超过一次 `Dispatch` 在同一台机器上花掉时间的 10%，或者这样的 crate 进了依赖树，或者有一块与城无关的盘（网络盘、休眠的外置盘）让列盘停住。
2. 读卷函数是函数指针，不是 trait：生产只有一种读法，测试给一个返回低剩余空间的函数即可注入「盘快满」。

**测试。** `accounting::worker::commanding::tests::shedding`：读卷函数报告卷低于地板时，`Dispatch` 被拒为 `BackpressureShed`，recovery 说出要腾出的字节数，房间没有被建起来。`monitor::volume::tests`：以相对路径 `.` 读卷、以 `canonicalize` 给出的 verbatim 拼写读卷，都与以工作目录的绝对路径读到同一块盘。

**本节接口的当前状态。** `Wake` 等不经人的入口尚未接入；事实条与 doctor 尚不显示降级；盘慢、内存紧、CPU 被占满三种状态还没有生产的读数（kernel-SPEC 8-74）。
## 8-117 `sprawling view`：给人的一面（`bin::main::view::keys`、`bin::main::view::arrange`、`bin::main::view::rounds`、`bin::main::view::frame`、`bin::main::view::detail`、`bin::main::view::follow`、`bin::main::view::terminal`）

**形状。** 五个纯模块，不碰终端也不碰盘。`keys` 是 decision：一个按键对应哪个 `Action`。`arrange` 是 projection：把 `accounting::lineage` 的 `RunLine` 排成一棵树，按显示顺序平铺成 `Entry`，每个 `Entry` 记着深度和父的下标。`rounds` 是 projection：把一个 run 在 `records` 里的行经 `accounting::views::turns`（`views::rounds::turns` 的公开投影，与 Views 的回合页同一份折叠）折成回合，再把回合与其中的调用排成那个 run 下面的 `Entry`。`frame` 是 state machine：`Face` 持有两个透镜共用的选中物、展开集合与详情模式，`apply(Action)` 改状态，`frame()` 按当前尺寸画出一帧文本行。`detail` 是 projection：任何记录都画成同一种缩进 JSON 树。`follow` 是 adapter：`open` 经 `storage::TailLines` 只读账本最新的 `FIRST_WINDOW_LINES` 行，折出窗口里的 lineage 与 `records` 行，同时在一条后台线程上跑整遍的 `LedgerIndex::rebuild` 与 lineage 折叠；`poll` 在整遍折完之前只看它到了没有，到了就交出整份（`Polled::Filled`），此后持有常驻的 `LedgerIndex` 与 lineage，只折上次之后追加的行（`Polled::Appended`）。`terminal` 是 adapter：stdout 是终端且没有任何过滤参数时，`view` 用 `follow` 读城，进 raw 模式与备用屏，读键、调 `apply`、画 `frame()`，退出时无论成败都把终端还原。它不做任何决定。`list` 是 projection：哪些树行可见、每行标什么、滚到光标可见的那一屏，以及账本透镜落在这一屏上的行。尚未做的：T8–T12 的 `ttyprobe` 验收。

```rust
// bin::main::view::keys
pub(super) enum Key { Char(char), Up, Down, Left, Right, Enter, Tab, Esc, PageUp, PageDown, Home, End, Interrupt }
pub(super) enum Action { Up, Down, PageUp, PageDown, First, Last, Collapse, Expand, SwitchLens, OpenDetail, CloseDetail, Quit }
pub(super) fn action_for(key: Key) -> Option<Action>;
// bin::main::view::arrange
pub(super) enum NodeKey { City, Building(String), Room(Address), Session(Address, Option<Seq>), Run(RunId), Round(RunId, u32), Call(RunId, Seq) }
pub(super) struct Entry { key: NodeKey, depth: usize, parent: Option<usize>, seq: Seq, label: String, detail: serde_json::Value }
pub(super) fn arrange(runs: &[RunLine], rounds: &Rounds) -> Vec<Entry>;
// bin::main::view::list
pub(super) fn visible(entries: &[Entry], expanded: &BTreeSet<usize>) -> Vec<usize>;
pub(super) fn tree_lines(entries: &[Entry], shown: &[usize], expanded: &BTreeSet<usize>, rounds: &Rounds) -> Vec<String>;
pub(super) fn scrolled(lines: Vec<String>, cursor: usize, rows: usize) -> Vec<String>;
pub(super) fn record_lines(records: &[Row], cursor: usize, rows: usize) -> Vec<String>; // 只给这一屏的行算哈希
// bin::main::view::rounds
pub(super) type Rounds = BTreeMap<RunId, Result<Vec<wire::Turn>, AxError>>;
pub(super) fn fold(run: RunId, rows: &[Row]) -> Result<Vec<wire::Turn>, AxError>;
pub(super) fn append_below(entries: &mut Vec<Entry>, at: usize, folded: &Result<Vec<wire::Turn>, AxError>);
// sprawling（库）
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn>;
// bin::main::view::frame
pub(super) struct Size { columns: usize, rows: usize }
pub(super) struct Face;
pub(super) const FILLING: &str; // 状态行
impl Face {
    pub(super) fn open(runs: &[RunLine], records: Vec<Row>, size: Size) -> Face;
    pub(super) fn open_window(runs: &[RunLine], window: Vec<Row>, size: Size) -> Face;
    pub(super) fn fill(&mut self, runs: &[RunLine], whole: Vec<Row>);
    pub(super) fn apply(&mut self, action: Action);
    pub(super) fn resize(&mut self, size: Size);
    pub(super) fn frame(&self) -> Vec<String>;
    pub(super) fn is_closed(&self) -> bool;
    pub(super) fn follow(&mut self, runs: &[RunLine], appended: Vec<Row>);
}
// bin::main::view::follow
pub(super) const FOLLOW_TICK: Duration; // 100 ms
pub(super) struct Row { seq: Seq, run: RunId, line: String }
pub(super) struct Follow;
pub(super) const FIRST_WINDOW_LINES: usize; // 1000
pub(super) type Folded = (Vec<RunLine>, Vec<Row>);
pub(super) enum Polled { Filled(Folded), Appended(Folded) }
impl Follow {
    pub(super) fn open(dir: &Path) -> Result<(Follow, Folded), ViewError>; // 只有窗口
    pub(super) fn poll(&mut self) -> Result<Option<Polled>, ViewError>;
}
// bin::main::view::detail
pub(super) fn json_lines(value: &serde_json::Value) -> Vec<String>;
pub(super) fn line_lines(line: &str) -> Vec<String>; // 第一行 chain_hash，其后是 JSON 树；不是 JSON 的行画成一个字符串
// bin::main::view::terminal
pub(super) fn show(dir: &Path) -> Result<(), ViewError>;
```

**键。** `j`/`↓` 下一行，`k`/`↑` 上一行，`h`/`←` 折叠（已折叠时跳到父），`l`/`→` 展开（已展开时进第一个子），`PageDown`/`PageUp` 翻一屏，`g`/`Home` 第一行，`G`/`End` 最后一行，`Tab` 换透镜，`Enter` 进全屏详情，`Esc` 退出全屏详情，`q` 与 Ctrl-C（raw 模式下它是一个键，不是信号）关掉查看器。别的键没有动作；Windows 另报的松开与重复不算按键。与 WebUI 的 run 板同一套键（D-15）。

**树。** 城 › 楼（地址的第一段）› 房间（整个地址）› 会话（`RunLine.session`，没有就是房间的第一段 stretch）› run。父 run 在账本里时，run 挂在父 run 下面（分叉挂在分叉点下，标 `fork @<at_seq>`）；否则挂在自己的会话下；没有地址的 run 直接挂在城下。同一个父下的子按 `first_seq` 排；接替的 run 带 `after <predecessor>`。每个节点只有一个父。run 第一次被展开（`l`/`→`）时才折它的行：`rounds::fold` 从 `records` 里挑出这个 run 的行交给 `accounting::views::turns`，结果按 run 记在 `Face` 里，树重排时回合（`round <n> @<opened>`）排在这个 run 的分叉之前，每个回合下是它的调用（`<tool> <subject>`）。回合节点的 `seq` 是打开它的 `model_called`，调用节点的是它的 `tool_called`，所以 `Tab` 落在那一行上。一行解析不了时，这个 run 下只有一个节点，标签写出解析错误，而不是少掉几个回合。跟随时，已折过的 run 有新行就重折。详情画 `wire::Turn`、`wire::Call` 的 JSON。

**打开时的光标。** 最新的等人批的 run（`unanswered > 0`）；没有就是最新的 `active` run；再没有就是最新的 run；一个 run 都没有就是城。「最新」按 `first_seq`。只展开它的祖先。

**两个透镜共用选中物。** `Tab` 从树到账本：选中第一条 `seq ≥` 节点 `seq` 的行（run 的 `seq` 是它的 `first_seq`，别的节点是子树里最早的 `first_seq`）。从账本到树：选中这行所属的 run 节点并展开它的祖先；城自己的行选中城。

**首屏从尾部读。** 查看器打开时只付最新 `FIRST_WINDOW_LINES` 行的字节与解析，`Face::open_window` 按这些行画树，最后一行是状态行 `FILLING`，列表让出这一行。lineage 与 `HotView` 是按 seq 正向的折叠：窗口里看不到 `run_started` 的 run 没有地址（挂在城下），状态也只按窗口里的行定。整遍折叠到了，`Face::fill` 换上整份 lineage 与整份 `records`（窗口之前的行排在窗口前面，折叠看到的窗口之后的行排在后面），重折每个已折过回合的 run，状态行消失；选中的仍是同一个节点并展开它的祖先，账本行的光标仍在同一个 `seq` 上。整遍折完之前 `poll` 不读账本的追加：整遍折叠开始于窗口之后，追加的行都在它里面。

**跟随。** 查看器开着时，`terminal` 等键最多 `FOLLOW_TICK`（100 ms）；没等到就 `poll` 一次：`LedgerIndex::refresh` 说没变就什么都不做，有追加就把新行折进 lineage 并交给 `Face::follow`。所以服务中的城里新开的 run 最迟一个 tick 加一次折叠之后出现在树上，远在 250 ms 之内。`Face::follow` 换上新的树、把新行接到 `records` 末尾，并保持：选中的仍是同一个节点（按 `NodeKey` 找回；它不在了就是城），展开过的仍展开，此前不在树上的 run 展开它的祖先，让人看得见它。账本行的光标不动。

**帧。** 宽度 ≥ `SIDE_PANE_MIN_WIDTH`（110 列）时右侧常驻详情栏，左右各占一半，中间一列 `|`；窄时只画当前透镜，`Enter` 进全屏详情。全屏详情在任何宽度下都占满整屏。每行按字符截到栏宽；光标行以 `>` 开头；列表滚动到光标恰好可见。树行是缩进 + `+`（有子、折叠）/`-`（展开）/空格 + 标签；还没折过回合的 run 也标 `+`，因为它有没有回合要到第一次展开时才知道（决定 6），标空格会告诉人那里什么都没有。账本行是这一行链哈希的前 `GLANCE_DIGITS`（12）位、两个空格、原行，前缀由 8-105 的 `chain_label` 写出。`record_lines` 先按 `scrolled` 的同一个公式定出这一屏从哪一行起，只给这一屏的行算哈希，所以一帧的代价跟屏高走，不跟账本长度走。哈希按 `Row.line` 的字节算：`follow` 只收解析成功的行，JSON 必是 UTF-8，`Row.line` 的字节与盘上一致；`Row` 以后若收未解析的行，哈希要改为从原字节算。详情对 run 节点画 `RunLine::to_json()`；对账本行先画一行 `chain_hash: <64 位十六进制>`，再画解析后的 JSON，解析不了就画原文字符串。

**决定。**

1. 键到动作是一张纯表，帧是 `Face` 的纯函数；终端 adapter 只做读键、调 `apply`、写 `frame()`。被否掉的：在事件循环里直接改光标——那样每个动作只能在真终端里测。
2. `crossterm` 只开 `events` 与 `windows` 两个特性：这一面不画颜色、不读粘贴与剪贴板，默认特性只会多链接没人调用的代码。被否掉的：自己写 Windows 控制台与 termios 两套 raw 模式——那是两份平台代码，换来的只是少一个依赖。
3. 全屏详情在宽屏上也占满整屏，而不是在宽屏上忽略 `Enter`：同一个键在任何宽度下意思一样，大记录也能用满宽度看。
4. 「等人」按 run 数在 `accounting::lineage` 里，而不是读 `views::governance` 的待批表：待批表按 `id` 记，不记是哪个 run 提的，从它答不出「哪个 run 在等人」。代价是「请求加一、答复按 `id` 减一」这条规则有两处折叠，靠同一对 kernel 类型保持一致。条件变了就重议：待批表记下提出它的 run 时，lineage 改读它，删掉自己这一份。被否掉的：光标只看 `active`（等人批的 run 往往已经冻结，最需要人的那一个反而不在光标下）。
5. 跟随靠轮询 `LedgerIndex::refresh`，不开文件系统通知，也不连服务中的城的 socket：没变时一次 refresh 只是一次目录列表加每段一次 `stat`，100 ms 一次对任何盘都是噪声；而通知在 Windows、inotify 与网络盘上是三套行为，socket 又要求查看器先知道城在不在服务。城不在服务时轮询什么都读不到，所以跟随不区分两种情况。条件变了就重议：`refresh` 在没变时也要读字节时。被否掉的：只在打开时读一次（人得退出重开才看得见新 run）。
6. 回合在展开时才折，从已在内存里的 `records` 行折，不回盘：打开时就给每个 run 折回合，会让 40 万行的账本在首屏前多解析一遍，而人一次只看几个 run；回盘按 `run_seqs_before` 读会让纯的 `Face` 碰盘。代价是展开一个 run 要扫一遍 `records` 挑它的行、再解析这些行。条件变了就重议：`records` 不再整本常驻内存时（从尾部倒读首屏之后），改由 `follow` 按 `LedgerIndex::run_seqs_before` 读这个 run 的行。被否掉的：在 `view` 里另写一份回合折叠（与 Views 的回合页是同一个规则的两份）。
7. 首屏的窗口按行数定（`FIRST_WINDOW_LINES` = 1000），不按终端行数，也不按字节：树要的是足够多的 run，一屏的行数给不出几个 run；一千行是几百 KB 的读与解析，在任何盘上都是首屏里的小头，而整遍折叠在后台，不挡第一帧。后台用一条 `std::thread`，因为查看器是同步的终端循环，没有运行时可以借；它只活到整遍折完，结果经一条 channel 交回，查看器退出时它随进程结束。条件变了就重议：run 索引快照（S5.22）落地后，首屏直接读快照，窗口与后台折叠一起删掉。被否掉的：打开时同步读完整本（40 万行的账本首屏要等整遍折叠）。

## 8-137 `sprawling view --since/--until`：按 UTC 选行（`bin::main::view`；runtime-SPEC 8-10、8-57）

**形状。** 不变：`main/view.rs` 仍是 adapter。`Selection` 多一个条件 `span: runtime::clock::UtcSpan`，由 `--since <utc>` 与 `--until <utc>` 给出，两者都经 `runtime::clock::parse_iso` 读，再由 `UtcSpan::new` 组成一个值。

```rust
// bin::main::view
pub(super) struct Selection { tail, from, run, kind, who, grep, span: UtcSpan }
```

**选中。** 一行选中当且仅当它信封的 `t` 在 `[since, until)` 里（`UtcSpan::contains`），其余条件照 8-105 同时成立。`t` 是这一行自己记下的那一刻（kernel-SPEC 8-4；回合里等来的四种行各记各的，runtime-SPEC 12.5），所以时间窗精确到单次调用；版本早于逐行时刻的账本里，同一回合的行共用回合的 `t`，时间窗就只精确到回合。

**逐行判断，不靠 `t` 有序。** `t` 不随 `seq` 单调：并行只读段的开始时刻可以早于前一条的答复，墙钟也会回拨。所以 `--since/--until` 与 `--grep` 一样是流式过滤：走完 `--from`/`--run` 给出的整段，每一行解析一次信封再判，不在第一条越过 `until` 的行处停，也不二分。`--tail N` 仍最后作用。代价与读到的行数成正比，与 `--kind`/`--who` 相同。

**失败。** `--since` 或 `--until` 不是 `iso` 写出的形状：stderr 一行 `sprawling: view: --since '<原文>': <parse_iso 的 recovery>`，recovery 给出正确写法（`2026-05-14T09:31:07Z`，UTC，到秒，以 `Z` 结尾），退出 2。`--until` 不晚于 `--since`：`sprawling: view: --since and --until: <UtcSpan::new 的 recovery>`，退出 2。合法而什么都没选中的区间照常退出 0、写零行。

**人的那一面。** 带了 `--since` 或 `--until` 就是带了过滤参数：终端前不进 8-117 的交互界面，按 8-105 写选中的行、行前带链哈希。

**测试。** `the_records_lens_keeps_the_lines_inside_a_time_window`、`a_line_whose_time_steps_back_is_judged_on_its_own`、`a_moment_view_cannot_read_names_the_shape_it_wants`（`bin::main::view::tests`）。

## 8-118 发布前在回环地址上跑一遍 `install.sh`（`install.sh`、`.github/install-loopback.sh`、`release.yml` 的 `archive`）

**形状：适配器的验收。** `install.sh` 是 `curl | sh` 那条通道：它向发布 API 要最新一版，按平台后缀挑归档，下载、比对 sha256、解包、问一句 `status`，再把二进制交给 `sprawling install` 落位。这一串里每一步都只在真的发布之后才被执行，所以它坏了，第一个知道的是下载的人。

**接口。**
- `install.sh` 读 `SPRAWLING_API`：发布列表的地址，缺省是 `https://api.github.com/repos/${SPRAWLING_REPO}/releases`。设了它，列表（`?per_page=1`）与单个 tag（`/tags/<tag>`）都从这个地址问；归档的下载地址仍然取自列表里每个资产的 `browser_download_url`，不由脚本拼。这个变量同样服务镜像与 GitHub Enterprise，它不是只为测试开的口子。
- `.github/install-loopback.sh <archive-dir>`：`<archive-dir>` 里恰好一份 `.zip`（`just package` 在一个 matrix 行上产出的那份）。脚本在 `127.0.0.1` 上起 `python3 -m http.server`（端口取 0，由内核分配，从服务的第一行读出），写一份与 GitHub 同形的发布列表（`tag_name`；每个资产的 `name`、`size`、`digest: sha256:<hex>`、`browser_download_url`），然后以沙箱 `HOME` 跑 `install.sh`，最后执行 `sprawling install` 放进沙箱 `HOME` 的 `.local` 下 `bin` 里的那个文件的 `status`，打印第一行。
- 判定，三条都成立才绿：`install.sh` 退出 0；落位的文件与归档里的 `sprawling` 逐字节相同（`cmp`）；落位的二进制的 `status` 第一行以 `sprawling ` 开头。任何一条不成立，脚本以非 0 退出并说出是哪一条。
- 离机的请求一律失败：脚本给 `install.sh` 设 `https_proxy`／`http_proxy` 指向 `127.0.0.1:9`，`no_proxy=127.0.0.1`。所以一个不认 `SPRAWLING_API` 的 `install.sh` 不会偷偷装上 GitHub 上已发布的那一版，而是在第一次请求就红。
- `release.yml` 的 `archive` job 在 `just package` 之后、上传之前对 macOS 与 Linux 两行调它；Windows 那一行走 `install.ps1`，不在本节。

**决定。**
1. **比字节，不比版本行。** 版本行只说版本与发布日，两次构建同一个 tag 的二进制说同一句话，GitHub 上已发布的那一版也可能说同一句话；落位的文件与本次归档逐字节相同，才证明装上的是本次构建的这一份。败给的方案：断言版本行含 tag——`status` 的第一行根本不印 tag。
2. **端口由内核分配。** 固定端口在并行的 runner 或开发机上会撞；端口 0 在每台机器上都成立，不需要按机器调。
3. **归档与列表都经 HTTP 提供，而不是 `file://`。** `install.sh` 真实走的是 `curl -fsSL` 的 HTTP 路径，`file://` 会绕开状态码与重定向的处理，测到的不是人跑的那条路。

## 8-119 拖进对话框的文件存进城里（`bin::assembly::dropping`，形状：adapter）

浏览器不把被拖进页面的文件的绝对路径告诉页面。拖放携带 `file://` URI（`text/uri-list`）时页面直接用那条本地路径，不经过城；否则页面把字节经本页同源的 `POST /drop`（wire-SPEC 8-49）交给城，城存下它，答出它的绝对路径，页面把这条路径插进对话框。本节是城这一半。

**接口。**

```rust
pub(super) const DROPPED_DIR: &str = "dropped";
pub(super) fn keep_dropped(city_root: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, AxError>;
pub(super) fn dropping(city_root: PathBuf) -> wire::DropSink;   // listening 把它装进 ServeConfig
```

- 存放处是 `<city>/hall/dropped/<hash>/<name>`：`<hash>` 是字节的 blake3 前 16 个十六进制字符，`<name>` 是浏览器给的文件名的最后一段。答出的是 `std::path::absolute` 给出的绝对路径。
- 名字由地址文法判：`hall/dropped/<hash>/<name>` 必须能被 `kernel::Address::parse` 读出来，否则 `E_INVALID_ARGS`，recovery 说改个名字再拖。空名字、`..`、带路径分隔符的名字都在这一步被拒。
- 同样的字节、同样的名字再拖一次，落在同一个文件上并答出同一条路径，文件只写一次；同名而字节不同的两个文件落在两个目录里，互不覆盖。
- 写失败是 `E_STORAGE_FATAL`，subject 是那条路径。

**决定。**

1. **存在 hall 的 `dropped/` 下，而不是保留子树 `.sprawling/` 或城根下自己的目录。** 居民的 `read` 只收城内相对地址，并且不进保留子树（runtime-SPEC `tools::chosen_path`）；放进 `.sprawling/` 的文件人能看见，市长却读不到。城根下每个顶层目录都是一栋楼（`city::buildings`），放在城根下的 `dropped/` 会作为一栋楼出现在城里和工作区的菜单里。hall 是每座城都有的楼（`kernel::consts_policy::HALL_BUILDING`），市长在自己的楼里读它，其他楼按 `kernel::address::may_read` 读它（除非 hall 的规则把它标为机密），而绝对路径的末尾就是那条相对地址。
2. **目录按内容命名。** 用时间或序号命名，同一个文件拖两次就是两份；用名字命名，两个同名的文件互相覆盖。内容哈希让两件事都不发生，也不需要任何状态。
3. **名字交给地址文法判，不另写一份清洗规则。** 一条存得下却不能被读成地址的路径，居民拿到了也打不开；地址文法已经是「城里什么路径合法」的唯一回答。

**测试。** `assembly::dropping::tests`：一个文件存下后答出的路径在 hall 的 `dropped/` 之下、内容与拖进来的字节相同；同样的字节再存一次答出同一条路径；`..` 与带分隔符的名字被拒为 `E_INVALID_ARGS`，城里什么也没有写。

## 8-120 每一项的三个版本：装着的、钉住的、上游最新的（`bin::doctor::upstream`、`bin::doctor::pin`，形状：adapter；wire-SPEC §8-50）

一个人打开依赖页，要知道的不只是「有没有」，还有「是不是旧了」。装着的版本由探测读出（`DoctorVersion::Said`），钉住的版本写在仓库的两份文件里，上游最新的版本只有发布它的人知道。

```rust
// bin::doctor（Requirement 多三个字段，每一行都写明）
pub(crate) enum Pack { RustTools }                         // §8-58：同一包的行在页面上是一行
pub(crate) enum Pin { Unpinned, RustToolchain, LeanToolchain }
pub(crate) enum Upstream { Crate(&'static str), GitHub(&'static str), RustChannel, PythonOrg,
                           Unread(wire::DoctorUnread) }
// bin::doctor::pin：钉子读自仓库里的那一份文件，编译时 include_str!
pub(crate) fn pinned(pin: Pin) -> Option<String>;
// bin::doctor::upstream：一项的上游最新版本，出网
pub(crate) fn newest(item: &str) -> wire::DoctorUpstream;   // 不失败、不等：没读过的项起一条线程去读，先答 Asking
```

- **上游只问官方的来源**：cargo 工具与 just 问 crates.io（`/api/v1/crates/<crate>` 的 `crate.max_stable_version`），Rust 问 static.rust-lang.org 上 stable 发布通道的清单里 `[pkg.rust]` 的 `version`，Python 问 python.org 的 release API（取已发布、非预发布版本里最大的一个），其余问项目自己的 GitHub releases 的 `latest`（git-for-windows、rustup、bun、uv、elan、lean4）。版本号取答案里第一个点分数字，于是 `bun-v1.2.3` 与 `v2.55.0.windows.3` 都读成页面比得了的样子。
- **读不到的项说它读不到，并说为什么**：rustfmt 与 clippy 随工具链发布（`WithToolchain`），浏览器一族有好几个牌子各自发布（`ManyBrands`），驱动的版本跟着浏览器走（`MatchesBrowser`），sandbox 引擎是这份代码自己（`ThisProject`），shell、python-wasi 与 ffmpeg 没有一个官方的机读来源（`NoSource`）。网络上失败的一问答 `Refused { said }`，`said` 是停在哪一步。
- **经这座城自己的网络设置出去**：请求由 `gateway::client_for(Proxying::ExceptLocal, url)` 造，与发布检查、模型调用同一条代理规则。
- **页面打开依赖组时才问，每项一问，后台逐个填上，问题本身从不等网络**：一个会话的问题是一个接一个答的（wire 的 `server::socket`），三十来个出网的问题排成一队，会把这一页其余的每一个问题都堵在最慢的那一个后面。所以 `Query::UpstreamVersion { item }` 立即作答：头一次问到某一项时起一条线程去问它的发布者，答 `Asking`；线程把读数留在进程里，页面对还是 `Asking` 的项隔 1.5 秒再问一次，读到为止。线程的请求有 10 秒上限，所以轮询有尽头。这与 `NewestRelease` 的「只在人按下时问」是同一个原则的两种按法：打开依赖页就是人在问自己的工具旧没旧。一次成功的读数在本进程里记住，下一次打开不再出网；失败的读数交出去一次就忘掉，下次打开再问；GitHub 对不带凭据的请求每小时只给 60 次，每开一次页都重问会在一个小时里把它用完。记住的读数不按时间过期，因为本二进制读时钟只在 `bin::assembly` 一处（ARCHITECTURE §10）；城重启即重问。
- **钉子只在一处写**：`Pin::RustToolchain` 读 `rust-toolchain.toml` 的 `channel`，`Pin::LeanToolchain` 读 `lean-toolchain` 冒号之后的部分。改版本只改那一份文件。
- **比较在页面**：`installed < newest` 时那一行标「有更新」。比较按点分数字逐段比，读不出数字的一方不比。
- **打开依赖组即探一次**：页面每次打开这一组发一次 `DoctorRefresh`（§8-54 的同一条命令），不再等人先按「重新检查」；城里已有的答案先画出来，新的一份到了再换上。

**测试**：`doctor::pin::tests` 读出的两个钉子与两份文件相等；`doctor::upstream::tests` 从固定的答案文本里读出版本（crates.io、通道清单、python.org、GitHub 各一份），以及每一行的 `Upstream` 与表对得上；`doctor::tests::the_rust_tools_pack_is_every_cargo_tool_this_repository_calls`。

## 8-139 远程门进城：门的看守、远程监听与逐帧授权（`bin::outside`，形状：adapter；crates/remote_access/Spec.lean §8-10、§8-11，kernel-SPEC §8-81，wire-SPEC §8-65、§8-66）

一个人把城留在家里出门，要从手机上看城、答提问、叫停。`remote_access` 判谁能进、持有密码学，但它不认识帧、不开端口；这一节是只有本二进制做得了的那一半。

```rust
// bin::outside::keeper：一把锁后面的一扇门
pub(crate) struct Senses { pub(crate) clock: Clock, pub(crate) entropy: Entropy }   // 在 bin::assembly 造
pub(crate) struct Keeping { pub(crate) devices: PathBuf, pub(crate) ledger: Box<dyn Ledger + Send>,
                            pub(crate) route: Option<Box<dyn Route + Send>>, pub(crate) senses: Senses }
pub(crate) enum Revoking { Named(String), All }
pub(crate) enum Phase { Open, Closed }
#[derive(Clone)] pub(crate) struct Doorway { /* Arc<Mutex<…>> 与 Senses */ }
impl Doorway {
    pub(crate) fn keep(keeping: Keeping) -> Result<Doorway, AxError>;            // 读设备表，门关着
    pub(crate) fn open(&self, local: SocketAddr, lasting: Lasting) -> Result<Opened, AxError>;
    pub(crate) fn attend(&self, listening: Box<dyn Send>) -> Result<(), AxError>;
    pub(crate) fn close(&self, why: RemoteClosing) -> Result<Phase, AxError>;
    pub(crate) fn tick(&self) -> Result<Phase, AxError>;
    pub(crate) fn invite(&self, name: &str, authority: Authority) -> Result<Inviting, AxError>;
    pub(crate) fn pair_reply(&self, hello: &PairHello) -> Result<(PairReply, CityPairing), AxError>;
    pub(crate) fn claim(&self, pairing: CityPairing, sealed: &[u8]) -> Result<Vec<u8>, AxError>;
    pub(crate) fn session_reply(&self, hello: &Hello) -> Result<(Reply, CityWaiting), AxError>;
    pub(crate) fn admit(&self, waiting: CityWaiting, finish: &Finish) -> Result<(SessionId, Session), AxError>;
    pub(crate) fn authority(&self, session: SessionId) -> Result<Option<Authority>, AxError>;
    pub(crate) fn devices(&self) -> Result<Vec<Device>, AxError>;
    pub(crate) fn revoke(&self, which: &Revoking) -> Result<Vec<Device>, AxError>;
}
// bin::outside::verbs：一帧的类（wire-SPEC §19-2 的 class 列）
pub(super) enum Passage { Judged(VerbClass), Greeting(wire::Hello) }
pub(super) fn passage(frame: wire::ClientFrame) -> Passage;
pub(super) fn command_class(command: &wire::WireCommand) -> VerbClass;          // 穷尽匹配
// bin::outside::conduit：一段会话的帧，一帧一判
pub(super) enum Step { Forward(String), Answer(Vec<u8>), Lock }
impl Conduit { pub(super) fn judge(&mut self, sealed: &[u8]) -> Result<Step, AxError>;
               pub(super) fn seal_for_device(&mut self, text: &str) -> Result<Vec<u8>, AxError>; }
// bin::outside::listener：回环上的远程监听
pub(crate) const PAIR_PATH: &str = "/remote/pair";
pub(crate) const SESSION_PATH: &str = "/remote/session";
pub(crate) struct Reaching { pub(crate) runtime: Handle, pub(crate) city: SocketAddr, pub(crate) token: Option<String> }
// bin::outside::devices：设备表
pub(super) fn read(path: &Path) -> Result<Vec<Device>, AxError>;
pub(super) fn write(path: &Path, devices: &[Device]) -> Result<(), AxError>;
// bin::assembly::remote_door：门由什么做成
pub(super) struct Outdoors { city_root, relay, city, token }   // keep(self) -> Result<Remote, AxError>
```

- **一扇门，一把锁**：`Doorway` 是控制台的线程与远程监听的每一条连接共用的句柄。门的一次判定、它写的那一行账、设备表的落盘在同一把锁下发生，所以账本上的次序就是门里发生的次序。
- **五行经 worker 的 relay 写**（kernel-SPEC §8-81）：`RunWorker::relay()` 交出与驾驶线程同一种 `kernel::Ledger`，草稿过同一个队列到唯一的写者，城仍只有一个写者。`who` 是 `person`，门到时自己关上那一行是 `city`。一行写不进去时，门的那一步不算发生：开门时通路随即关上，配对时设备不留下。
- **设备表**在 `CityLayout::devices`（保留子树下远程门自己的目录里，文件名是 `kernel::layout::DEVICES_FILE`），每台设备一个 `[[device]]`（`id`、`name`、`authority`、`key`，写法见 crates/remote_access/Spec.lean §8-11）。整份写到旁边的新文件再改名盖过去，崩溃只留下改前或改后的一份。文件不存在是空表；读不成的表以 `E_STORAGE_FATAL` 拒并写出文件路径，不猜谁能进。配对与撤销各写一次；写失败时这次配对不算数。
- **名字在门外判唯一**：`invite` 拒一个已配对设备用过的名字（`E_INVALID_ARGS`），门本身不看名字（crates/remote_access/Spec.lean §11）。
- **远程监听**：`/remote open` 在 `127.0.0.1` 上绑一个系统给的端口，先开门、再起接收的任务，任务的中止把手交给门（`attend`），门一关它就停。监听每秒问一次门到没到时；到了就以 `Expired` 关门，写一行。两条路径与上面的消息见 crates/remote_access/Spec.lean §8-10。门的看守里写账、写盘的那几步放到阻塞线程池上做：relay 等的是唯一的写者，套接字任务陪它等会占住一个反应器线程。
- **逐帧授权**：`Conduit::judge` 打开封装，`Lock` 交回给监听去关门；线协议帧读成 `ClientFrame`，`Hello` 换上城的令牌再发（wire-SPEC §8-66），其余按 `verbs::passage` 判类、问 `Doorway::authority` 与 `permits`。放行的原文发给城的 `/ws`；拒绝的不到城，设备收到一帧封好的 `Refusal`（`E_GATE_DENIED`）；会话已不被门持有时返回错误，连接结束。中继以线协议客户端的身份连城自己的监听，绑在 `0.0.0.0` 的城从回环连。
- **城的签名密钥每个进程一把**：`Doorway::keep` 取 32 字节熵派生它，不存。跨重启保存它要在 vault 之外多一个兑现点，即放宽 `xtask secret` 的名单，未决（crates/remote_access/Spec.lean §3）；在那之前，城一重启，设备就要重新配对，`/remote open` 照实说。
- **没有通路被选中**：`Keeping.route` 在生产装配里是 `None`，因为本批没有读者读 `[remote]` 表；`open` 以 `E_CONFIG_INVALID` 拒，恢复语说明这一点（crates/remote_access/Spec.lean §3）。测试用 `route::scripted::ScriptedRoute`。
- **门的看守取不起来时城照常服务**：设备表读不成、熵取不到，`/remote` 每一个子动词都打印那一句拒绝，其余控制台与页面不受影响。

**测试**：`outside::tests` 用一台 Rust 写的设备、一条脚本化通路、计数的时钟与计数的熵走门：`the_door_writes_its_five_lines_in_the_order_they_happen`（开、配对、会话开始、撤销、关，五行按此次序入账）、`a_local_only_frame_is_refused_and_reaches_no_city`（`Act` 设备发 `Reveal`，答一帧 `E_GATE_DENIED` 的 `Refusal`，没有一个字节放行）、`a_watching_device_is_refused_a_verb_that_acts_and_may_still_ask`（`Watch` 设备的 `Cancel` 被拒、`Ask` 原文放行）、`the_device_table_survives_a_reopen`（两台设备，含一个中文名，重开后设备表相等）。

## 8-140 控制台的 `/remote`（`bin::outside::console`、`bin::console::language`；remote_access D4）

```rust
pub(crate) enum RemoteLine { Open(Lasting), Pair { name: String, authority: Authority }, Close, Devices,
                             Revoke(Revoking), Unreadable }
pub(crate) struct Lasting(u64);                                   // 毫秒；ms()
pub(crate) struct Remote { pub(crate) doorway: Doorway, pub(crate) reaching: Reaching }
pub(crate) fn parse(tail: &str) -> RemoteLine;                    // 纯
pub(crate) fn carry(remote: &Result<Remote, AxError>, line: RemoteLine) -> String;
// bin::console::terminal：控制台够进城的三样东西
pub(crate) struct Inside { pub(crate) desk: Arc<CommandDesk>, pub(crate) answering: Answering,
                           pub(crate) remote: Result<Remote, AxError> }
```

- **五个子动词**：`/remote open [--for <时长>]`、`/remote pair <名字> [--watch]`、`/remote close`、`/remote devices`、`/remote revoke <名字>|--all`。`remote` 是控制台自己的第六个动词（§8-11 的 `CONTROL`），不在线上，所以浏览器、远程设备与居民手里的任何工具都发不出它。
- **时长**：一个整数加单位 `m`、`h`、`d`，至少一分钟，至多七天；不写 `--for` 是 12 小时。读不成的一行打印用法。
- **名字可以有空格**：`pair` 与 `revoke` 后面除 `--watch` 之外的词连起来就是名字，所以「客厅的 iPad」不必加引号。`--watch` 写在名字前后都行；不写就是 `act`。
- **`/remote pair` 印二维码**：邀请链接（`https://<主机名>/#pair=<配对码>&city=<指纹>`，crates/remote_access/Spec.lean §8-6）用 `qrcodegen` 以低纠错级编码，两行模块合一行字符，亮模块印成方块、暗模块印成空格，四周留两格亮边：终端是浅字深底，扫码器要深码浅底。码下面再印链接与分组的配对码，说明它十分钟内有效、只用一次。
- **`/remote open`** 印出门开多久、设备打开的地址；通路是 `PerStart` 时说明每次重启要重新配对；并说明城的密钥只在这个进程里。**`/remote devices`** 每台一行：名字、权限、设备 id。**`/remote revoke`** 印出被撤销的名字，它们的会话随之结束。
- **控制台的循环拿一个 `Inside`**：发命令的桌子、问问题的那一个函数、远程门，三样合成一个值，`drive` 的参数从五个回到四个。

**测试**：`outside::tests::a_remote_line_reads_as_the_verb_it_names`（时长、带空格的中文名与 `--watch`、`--all`、多出的词与读不成的时长）；`console::tests::parsing::the_control_verbs_are_the_six_it_owns`。
