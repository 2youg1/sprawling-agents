# sprawling-SPEC.md — bin（main＋assembly＋嵌入链）

> crate：`sprawling`（唯一 bin）。本章只覆盖 Stage 0 范围；子命令随各期扩章（init/serve/resume/replay/fork/status 的全量语义随各期在本 SPEC 落章）。

## 1 需求拆解

S0 三件：①CLI 壳（`status` 可用；未到期的子命令给出诚实拒绝而非 stub）；②`assembly` 骨架（Main＝唯一知情点的形状先立，S1 起逐缝接线）；③`build.rs` 嵌入链（web 产物 → OUT_DIR → `include_bytes!`，S0 用占位页验证链本身）。

## 2 验收标准

`sprawling status` 打印版本、嵌入字节数、已接适配器数并退出 0；未知子命令退出 2 且指明「哪些随期解锁」；单测证明嵌入字节非空且含标记（卡 S0.09 收口）。

## 3 假设与歧义

「验证这条链先于验证页面内容」——S0 不起 HTTP 服务，HTTP 属 channels::server（S4）；嵌入的取证面是测试与 `status` 输出。

## 4 现状分析

空壳。无。

## 5 权威信源

bin 子命令面；装配层是 Main；ARCHITECTURE.md §2（客户端嵌入链）与 §12（模块图的 bin 段）。

## 6 命名统一

**跨 crate 类型住处（card-1.1–1.3 起）**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。（card-2.2 起 `gateway::credential::Custodian` 住 `custodian` 同例，公共拼写不变）。（card-2.4 起 `channels::command::Command` 住 `kind` 同例）。

Assembly／assemble；WEB_INDEX（嵌入资产常量）。

## 7 模块边界

`main`：CLI 分发与呈现；`assembly`：唯一知情点，句柄/时钟/种子/spawn 注入处。
**本 crate 不做什么（否定式三条）**：不含任何判定（判定住 kernel）；不直接落盘（落盘住 memory）；S0 不起网络服务（channels S4）。

## 8 接口先行

```rust
pub(crate) struct Assembly { /* 适配器随缝落地逐字段进驻 */ }
pub(crate) fn assemble() -> Assembly;   // 全库唯一的时钟采样与 spawn 授权点（届时以 #[expect] 标注）
```

## 8-2 派活回路（P1.02）

```rust
pub(crate) struct RunWorker { city_root, ledger: JsonlLedger, cas: Cas, model: Box<dyn Model + Send> }
impl RunWorker {
    pub(crate) fn new(city_root: &Path, model: Box<dyn Model + Send>) -> Result<Self, AxError>;
    pub(crate) fn observe(&mut self, sink: memory::WriteObserver);
    pub(crate) fn handle(&mut self, command: channels::WireCommand) -> Result<(), AxError>;
}
pub(crate) fn configured_model() -> Box<dyn Model + Send>;
pub(crate) async fn serve(city_root, addr, token, index_html, model) -> Result<(), AxError>;
```

- **单写者，且它不跨线程**：`RunWorker` 在自己的线程里 `new`，JsonlLedger 因此**从未跨过线程边界**——不需要为了一个 `Send` 约束去改 memory 的内缝（改了就会造成 `FaultFs` 的 `Rc` 不合法，而故障适配器本来不需要跨线程）。启动错误经一个 `sync_channel(0)` 回报，所以「开不了账」仍然是 `serve` 的错而不是一条日志。
- **命令受理与命令执行分开**：socket 任务只 `mpsc::Sender::send`，回合循环在工作线程。刷新页面不会杀掉工作，而进展从事件流回流——「关掉界面再打开」与「从未关过」在服务端看来因此无差别。
- **事件流的源头是 Ledger 本人**：`JsonlLedger::observe` 在**持久化之后**逐条回调，回调把记录扔进 broadcast。服务端因此推不出一条历史里没有的事实。
- **无 provider 时仍然能起服务**：`UnconfiguredModel` 把「没配模型」变成一条三段式拒绝，而不是拒绝启动。一座城在没有推理服务时仍然可读（重放、浏览）。配置面：`SPRAWLING_MODEL_URL`（回环）＋`SPRAWLING_MODEL`。
- **工具名录与工具台同源**：`Catalog::admit_tool` 产 `ToolDef`（模型看到的），`ToolBench::register` 负责路由（实际跑的），一次登记喂两边——否则“模型以为存在的工具”与“真能跑的工具”会成为两份名单。本期只挂 `edit` 与 `status`：`exec` 需要一个真 sandbox 与 Python WASI 配置，随出网与 sandbox 那张卡一并接。
- **RunId 是推导而非抽取**：`b3(job|addr|now)` 前 16 字节。同一毫秒对同一地址派同一件活就是同一个 Run，且标识符里不进随机数（确定性第 7 条的同一条理由）。
- **预算不可缺**：`DISPATCH_TURN_BUDGET = 24`。调用方还不能设它，但无上限地向付费 provider 循环是唯一没有天花板的失败模式。——**此条已被 card-11.7 推翻（见 §8-40）**：常量删除，一跑不再有回合上限，刹车只剩 `Cancel` 与 `Halt`。
- **`init` 写 `City.md` 入城**：二进制携默认本，城里那份是用户可改的权威；每次组装 prefix 读城里的那一份，代码里恒不长第二份副本。

## 8-2b CLI 补齐（整修卡 R1.03）

- **`resume <city>`**＝启动扫描：验链 → `dangling_tool_calls` 逐个补记 `E_TOOL_OUTCOME_UNKNOWN` 的 `tool_result`（幂等：已闭的账不重补）→ 报待批数。`runtime::replay` 补写面自此有生产消费者（P3.03 留下的台账行翻过）。续跑已批准的活仍在 serve 的 `answer_approval` 路径上，两者不重叠。
- **`fork <city> <run> <seq> [addr]`**＝世系记录形：验链 → `runtime::fork::prefix` 验界 → 节点归属验（seq 处的事件必属于母 Run，否则拒）→ `run_forked` 落账携新 RunId。**不自动发车**：驱动新 Run 是人的下一步 Dispatch；逐字节母前缀入窗属并发期的 must-read 网，不在本形。`Command::Fork` 同路。
- **`adopt <city> <addr>`**＝收编已存在目录为楼（语义住 `city-SPEC.md` §8-3）。
- **`serve` 增 `--web-dir <dir>`**：开发回路逐请求读盘；发布形恒走嵌入表（`channels::ClientAssets`，语义住 channels-SPEC §8-2）。

## 8-3 视图与 Spine（P1.03／P1.04）

```rust
pub(crate) struct Views { city_root, hot: HotView, attribution: Attribution, approvals: BTreeMap<String, ApprovalSummary> }
impl Views { fn apply(&mut self, &EventRecord) -> Result<(), AxError>; fn answer(&self, &Query) -> Answer; }
pub(crate) fn rebuild_views(ledger_dir: &Path) -> Result<Views, AxError>;   // 启动时冷重建
fn read_spine(city_root: &Path) -> Vec<BuildingProgress>;                    // 查询时读盘
```

- **视图冷重建与热折叠共用 `apply`**：启动时把 Ledger 逐行喂进去，其后由 `JsonlLedger::observe` 喂。测试断言两条路径答案逐字段相等——这就是「projection 可弃」的可执行形式。
- **Roadmap 查询时读盘，不入投影**：那份文件**就是**计划，Agent 用 edit 工具改它。把它复制进投影就是为同一件事立第二个说法，而漂开的总是没人看的那个。
- **读不懂的行照显**：`problems` 随答案回到界面；没有 Roadmap 的楼答 `Progress::Unplanned`（它没有 ratio 方法，故界面画不出百分比不是守规矩，是无从下手）。
- **保留前缀不是楼**：`.` 开头的目录跳过，`.sprawling/` 因此恒不被当成一栋楼。

## 8-4 MCP 接线（整修卡 R1.13）

```rust
// bin::mcp_stdio（形状 4 适配器；实现 protocol::Outbound）
pub(crate) struct StdioServer { /* 私有：Rc<RefCell<Inner>>，克隆即同一个子进程的第二个句柄 */ }
impl StdioServer {
    pub(crate) fn start(command: &str, args: &[String], cwd: &Path) -> Result<StdioServer, AxError>;
}
impl protocol::Outbound for StdioServer {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError>;
}

// bin::assembly
fn mcp_tools(&mut self, config: &FrozenConfig, addr: &Address, confidential: bool)
    -> Vec<protocol::McpTool>;   // 起不来的 server 缺席并留下诊断，恒不拒整次 dispatch
```

- **一台 server 一个子进程，一次 dispatch 一条命**：工具表随 Run 冻结，子进程的寿命因此就是 Run 的寿命。最后一个 `McpTool` 落地时 `Drop` 杀子进程，于是「谁来回收」不需要第二份名单。
- **读取线程是句柄的一部分，spawn 点仍在 bin**（确定性七条③的口径：并发归装配层）。同步读一根管道没有期限，而一个不回答的 server 会把整个 Run 挂死——本期已经吃过一次这个形（真机 provider 在 `model_called` 之后五分钟无返回）。故 `start` 起一条只读 stdout 的线程，`call` 用 `recv_timeout` 等它；超时即杀子进程并三段式拒。**线程恒不泄漏**：杀子进程关掉管道，读到 EOF 即结束。
- **期限从声明里来，不在适配器里另写一个数**：`ToolMeta.timeout`（`tools_from` 写的 `TimeoutMs(60_000)`）既是对模型的承诺，就应当是真正被执行的那一个；否则该字段只是装饰。故期限随 `Outbound::call` 入参。
- **起不来的 server 缺席而不拒 dispatch**：与 `city::library` 对「楼里点名却不在架上的 SKILL」同形——模型看到的名录恒等于真能跑的工具表，缺席的那一件在诊断里留名。一个外部服务今天起不来，不是这栋楼今天不能干活。
- **confidential 楼：一条规则两层后果，不是两份判定**。工具能不能存在归 `protocol::McpTool::new`（构造点拒，恒是权威）；**进程该不该被拉起归装配层**，因为进程寿命本来就是这一层的职责，而一台 MCP server 可能在启动那一刻就出网。故 confidential 楼在拉起任何子进程之前就跳过整张 `[[mcp]]` 表并留一条诊断；工具层的拒仍在，它是那一层失守时的兵底。两层同向，因此不会出现「只改一处」的漂移。
- **`Effect::Connector` 是本卡推出来的 kernel 变更**（语义住 kernel-SPEC §8-23）：接线前 `tools_from` 写的是 `Effect::Egress`，而出站门从 **调用参数**里读 `host`——外部工具的参数表由 server 的 `inputSchema` 决定，里面恒没有 `host`。第一次真调用当场拿到 `E_INVALID_ARGS: declares Egress but named no host`：这就是「一个适配器是假想缝」的同一条道理在工具面上的实例——没有调用方的声明从未被那道门验过。
- **discover 先于 list，但今天不据它分支**：它当下的作用是在把任何工具交给模型之前，先证明对侧真的会应答；版本协商要有第二个版本才成立，而字段名本库今天无法从一台真的 server 上核对。读到什么写进诊断，不写进判定。
- **口径不变的那两件**：外部工具与 L0 工具同落 `kernel::tool` 缝，故结果恒自动进污染环，装配层无解包面；调用由 `ToolBench` 路由，故 `tool_called`／`tool_result` 两行自动落 Ledger，不为它另写一条入账路径。

## 8-4b MCP 的第二条传输（整修卡 R1.17）

`bin::mcp_http` 是 `Outbound` 缝的第三个适配器（stdio 子进程、ScriptedOutbound、HTTP），也是这条缝第一次真正被两条生产路径共用。`McpServer.transport` 从两个裸字段改成穷尽枚举 `McpTransport { Stdio{command,args}, Http{url,header} }`：**一行既写 command 又写 url 就是一行要读者去猜的配置**，故配置层当场三段式拒。装配层把差异全部花在 `McpLink` 这一个枚举里，其上的接线仍是一条路。

三条口径：①**事件流只取第一条 `data:`**，读不出就拒，恒不把两条答案拼成一条工具结果；②**拒词不引用对侧正文**（服务端的错误页是别人写的字），只说状态码与该查什么；③当时写的「**HTTP 无会话**，故克隆只共享地址」**已于 P5.01 推翻**，见下。

## 8-4c 会话、凭据与报错地址（P5.01）

三处修正，三处都是真机跑出来的。

- **`HttpServer` 持会话，且克隆共享它**。旧口径「HTTP 无会话」读错了规范（详 `protocol-SPEC.md` §8-3）。一台 server 就是一个会话，不论一次 Run 拿了它几件工具；两个克隆各发一个 session id 就是与同一台 server 开两场对话，而它只开过一场。协商版本从**携 `result.protocolVersion` 的那一条答案**学得——按生命周期，那就是 `initialize` 的答案，因为它是一条连接的第一句话，没有更早的答案能持有该字段。
- **`header` 兑付 `secret:` 引用**。`redeem_header` 把 `Name: secret:realm/name` 在最后一刻换成真值，与 endpoint 凭据同一条路。不这么做，一把付费档的 key 会明文躺在楼里的 `CONFIG.toml`，而 `xtask secret` 看不见它（城市配置不在仓库内）。不是引用的值原样通过：头里是个账号名或固定标记的 server 无物可兑。
- **报错地址指向真正出事的传输**。`transport_site()` 按 `McpTransport` 分派；此前所有 MCP 失败都挂在 `bin::mcp_stdio` 名下，上一个跟着它去查的人被引到了错的文件。成功时同样留一行：对侧叫什么、说哪个版本、给了几件工具。

**真机验收**：一座真城接一台托管 server，诊断行为 `exa is exa-search-server speaking 2025-06-18, offering 2 tool(s)`；模型自主调用其搜索工具、读回真实结果、一个回合内给出答案并 `run_frozen{completion: done}`。

## 8-5 订阅登录接线（整修卡 R1.14）

```rust
fn login(&mut self, provider: &str, step: channels::LoginStep) -> Result<(), AxError>;
fn login_with(&mut self, profile: &gateway::OauthProfile, provider: &str,
              step: channels::LoginStep) -> Result<(), AxError>;   // 查表之外的全部
fn random_token(bytes: usize) -> Result<String, AxError>;          // OS 熵，非种子 RNG
fn dialect_of(provider: &str) -> Result<DialectKind, AxError>;     // 已知 provider 才有答案
```

- **熵不走种子**：`random_token` 用 `getrandom`（OS 熵），**恒不**用装配持有的仿真种子——一个第三方能预测的 verifier 就是一个第三方能完成的登录。这是全二进制里唯一一处「可复现即缺陷」的地方，故写在这里而不是留给读者推断。
- **pending 只活在进程里**：PKCE 的 verifier 证明「来兑的就是当初请求的那个进程」，一个活过进程的 verifier 什么也不证明。重启＝重新开始登录，代价是一次浏览器访问。
- **`login_with` 是查表之外的全部**：生产路径先查 `oauth_profiles` 再进它；测试把一台自己控制的 server 当 profile 传进去。**恒不为测试在生产路径上加环境变量开关**——那个开关在生产里没有人会设，却永远在那里可被设。
- **登录完即 attach**：人是为了用它才登录的，故 `api_base` 一到手就接上端点，而不是留下第二件要记得做的事。`api_base` 为空的 provider 在这里三段式拒，且拒词说明令牌已在保管库里——已经发生的事恒要说出来。
- **未做且已知**：令牌续期。`expires_in` 与 refresh token 都已入库，但到期前自动换新还没有接线；在它到来之前，过期就是重新登录一次。写成明账而不是留给用户去撞。

## 8-6 五个视图不再答 unavailable（整修卡 R1.16）

`Views` 增四份折叠与一次读盘，`answer` 的 catch-all 臂随之消失——**穷尽 match 是本卡的验收之一**：此后新增一个 Query 不写答案就编译不过，而不是在运行时答一句 `Unavailable`。

| 查询 | 出处 | 口径 |
|---|---|---|
| `InboxView` | 折 `signal_enqueued` 减 `signal_consumed` | **看队列不靠消费**：`Inbox::pull` 要拿走才给得出内容，一个看一眼就把东西取走的视图会改变它所报告的对象 |
| `DiscardView` | 折 `file_discarded`／`discard_restored`，按路径归键 | 每行自带回去的路（`restoration`）；还原是**关掉它开的那一行**，不是另开一行 |
| `RegistryView` | 折 `asset_archived` | 「这座城认定值得留下的东西」；空表就是空表，与「本版本答不了」在类型上已经不可混淆 |
| `ArchiveSearch` | 被问的那一刻读盘（同 `BuildingView`） | 文件是权威，另存索引就是第二个权威 |
| `Metrics` | 上面几份＋`hot`＋`read_spine` | **恒不携钱**：钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。这里每个数都已被别的视图证明过，它存在只为让画一条读数花一次问答；唯一自有的数是 `events`（本视图折过多少条），因为没有别的答案能推出它 |

## 8-7 ACP 入站与令牌续期（整修卡 R1.18）

```rust
fn acp_dispatch(desk: &CommandDesk, body: channels::AcpBody, authentic: bool)
    -> Result<channels::AcpProgress, AxError>;          // 外来请求 → 普通 Dispatch
fn renew_if_stale(&mut self, provider: &str) -> Result<(), AxError>;   // 用之前先换，不等 401
```

- **令牌在门那侧判，判定在协议那侧措辞**：`channels` 持配对令牌，故常数时间比对住 `/acp` 路由；`authentic` 这一位传进来，由 `protocol::admit` 说拒词——未配对者只学到一位，这句话的权威只有一个。
- **入站不是第二个 control surface**：admit 之后就是人按派活条时走的同一条路（同一个 `CommandDesk`、同一个 `Command::Dispatch`）。回给编辑器的只有 progress 三字段，且 run id 是工人接单时才铸的，故此刻诚实的答案是「已受理、尚未完成」。
- **续期在用之前做，不在 401 之后做**：一次 401 要花掉一整个回合才发现，而 provider 说过的到期时刻这座城已经写下来了（`secret_captured` 携 `expires_at`，非密文）。留一分钟余量；**没有记过到期时刻的 provider 不碰**——不知道什么时候过期，不是每次都换一遍的理由。
- **换新与兑付共用一次发送**：`send_token_request` 是两种 grant 的同一条路，故「不引用对侧正文」这条只写一次、也只可能对一次。

## 8-8 首次运行与交付形态（P7.01／P7.02／P7.03）

**病灶**：release 里的 exe 是控制台程序。无参启动只向 stderr 写一行用法并退 2，从资源管理器双击即闪退——没有安装过程，也没有任何成败提示。从 exe 到 WebUI 之间还压着 `init`／`serve`／自行输入地址三步手工操作，而 `serve` 打印的是裸 socket 地址不是 URL。终端、双击、脚本三类到达方式被挤在同一个入口上。

**不猜启动方式**：判断「我是被双击的还是在终端里跑的」，可靠办法是 `GetConsoleProcessList`，需 `unsafe`——workspace lints 恒禁。故以**显式入口**取代探测：三扇门各自命名，背后共用同一段序列。

```rust
// bin::firstrun
pub(crate) enum FirstScreen { Start(PathBuf), Quit }

pub(crate) fn default_city(exe_dir: &Path, home: Option<&Path>, exe_dir_writable: bool) -> PathBuf;
pub(crate) fn is_writable(dir: &Path) -> bool;
pub(crate) fn ask<R: BufRead, W: Write>(city: &Path, input: &mut R, out: &mut W) -> io::Result<FirstScreen>;
pub(crate) fn open_in_browser(url: &str) -> io::Result<()>;
pub(crate) fn local_url(bind: SocketAddr) -> String;
```

- **`up <dir>`＝序列的唯一定义**：目录里没有 ledger 就先 `init`，随后 `serve`，随后开浏览器。无参屏与 `start.cmd` 都落到它，`init`／`serve` 仍各自独立可用——一段序列一处权威。
- **genesis 要人同意**：写 Ledger 第 0 行是全系统唯一一次不可撤销的语义写入，不因「有人双击了一个文件」而发生。无参屏在按键**之前**把最终路径显示出来，人按回车才开城；`q` 退出并打印命令表。
- **非交互 stdin 无此问**：`read_line` 得 EOF（管道、CI、无人值守）即 `Quit`，主流程打印命令表退 2。这条让该路径在没有 TTY 的地方也可测。
- **默认位置取 exe 同级 `city/`**：整座城随文件夹可拷、可备、可删，与「一座城市就是一个目录」同构。`is_writable` 探到不可写（解压进 Program Files）就回退 `home/sprawling/city`；回退可见而非暗中，因为路径印在第一屏上。
- **开浏览器恒非致命**：`open_in_browser` 失败只记一行，`serve` 照跑——URL 在这之前已经打印。命名不取 `browser`：`crates/browser` 已占住「Agent 驱动真实浏览器」这个概念，一名一义。
- **横幅给人读**：city 目录、WebUI 的完整 URL、客户端完整与否、`Ctrl-C` 停城，四行。bind 是未指定地址（`0.0.0.0`）时 URL 仍给回环形，因为那才是本机打得开的那一个。

**交付形态**：`just package` 产 `sprawling-<version>-<target>.zip`＝二进制＋`start.cmd`／`start.sh`＋`QUICKSTART.md`；裸 exe 不再单独作附件，双击的目标因此永远是启动器。`release.yml` 由 tag 触发，三平台各跑 `just dist`，`xtask budget` 在打包前拦下页壳客户端（`CLIENT_COMPLETE=false` 的二进制），通过后才附件。手工上传的产物来历不明，是本次全部症状的链头，这条把它关掉。

**本章测试**：`default_city` 可写取同级、不可写取 home；`ask` 空行得 `Start`、`q` 得 `Quit`、EOF 得 `Quit`；第一屏文本在返回前已含最终路径（证明「先示后写」）；`local_url` 对未指定地址给回环形。

## 8-9 让二进制成为一个词（P0）

**病灶**：解压之后，那个 exe 不在任何搜索路径上。唯一的入口是找到那个文件夹再双击 `start.cmd`——找一个脚本比敲一条命令难，而桌面快捷方式比两者都难。`sprawling` 今天不是一个可以敲出来的词。

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
- **判定的编译面等于它的调用面（P4.05 修正）**：这四项与 `PathOutcome::Rewritten` 原本无条件编译，而只有 Windows 那一支调用它们，故 macOS 的 clippy 以五条 `dead_code` 判红——推送门只跑 Windows，这条错误在四次夜间 `platforms` 里红了四次而没有人看见。修法是让平台条件跟着调用方走（`#[cfg(target_os = "windows")]`），而不是加一条 `allow`：**平台不同不是要压制的告警，是要写进类型里的事实**。`PathOutcome::Rewritten` 是枚举变体、两平台共用同一枚举，故取同文件已有的先例——`#[cfg_attr(not(target_os = "windows"), expect(dead_code, …))]`，与 `SelfService` 对称。
- **Windows 必须直接改注册表，且必须保住值类型**。`[Environment]::SetEnvironmentVariable(..., 'User')` 是所有教程里的写法，也是错的：它**恒写 REG_SZ**，把本机 `HKCU\Environment\Path` 的 `REG_EXPAND_SZ` 降级，其中的 `%VAR%` 从此不再展开（dotnet/runtime#1442、chocolatey/choco#699）。本机实测该值确为 `ExpandString`，故适配器读原值时用 `DoNotExpandEnvironmentNames`、写回时用读到的那个 `RegistryValueKind`——**读到什么类型就写回什么类型**，键不存在时才取 `ExpandString`（Path 在 Windows 上的默认类型）。
- **值经临时文件进出，不经命令行**：用户名含非 ASCII 字符时，命令行要穿过控制台代码页（本机 936），而 `PATH` 的整条值也可能逼近命令行长度上限。故 Rust 与 PowerShell 之间用一个 UTF-8 临时文件传值，文件路径经环境变量交接，两侧都不需要引号规则。
- **改完必须广播 `WM_SETTINGCHANGE`，否则新窗口也读不到**：Explorer 缓存环境块，从它启动的新控制台继承的是缓存。`#![forbid(unsafe_code)]` 关掉了在 Rust 里调 `SendMessageTimeout` 这条路，故广播由 PowerShell 的 `Add-Type` P/Invoke 完成（`HWND_BROADCAST=0xffff`、`WM_SETTINGCHANGE=0x1A`、`SMTO_ABORTIFHUNG=2`、5 秒上限）。实测一次约 1.1 秒。**广播失败不致命**：路径已经写下了，报一行提示说「注销后生效」，而不是把已经成功的一半说成失败。
- **非 Windows 拷贝照做，改 shell rc 不做**：装进 `~/.local` 下的 `bin`（该目录在现代发行版上默认已在 PATH 上）。**不写 shell rc**，理由记在这里而不是留一个静默的空分支：rc 文件有 bash／zsh／fish 三套语法与 `.profile`／`.bashrc`／`.zshrc` 多个候选，选错就是往人的登录脚本里写一行没有作用却要人自己删的东西；而本机无 Linux/macOS，交叉编译到 Linux 已知走不通（`aws-lc-sys` 需 C 交叉工具链），故这一支只能由 CI 编译与 lint，不能由我运行验收——**该 job 自 P4.02 起是 `platforms.yml` 的 macOS job，不再是 ubuntu**（ubuntu 已被裁出流水线）。**没有跑过的写入动作不写**。目录不在 PATH 上时，报告里给出该加的那一行，人自己贴。

  **P4.02 改判（card-12.1，裁决 D33）**：Linux 重新进流水线。改判的理由是原判的**前提消失**而非原判错误：上段论证的是「从这台无 Linux 的开发机器**交叉编译**走不通（`aws-lc-sys` 需 C 交叉工具链）」，不是「Linux 构建不成立」；GitHub 的原生 runner 在 Linux 上原生构建，根本不碰交叉工具链。落法两件，分开决策：①`release.yml` 矩阵加 `x86_64-unknown-linux-musl` **静态**一行——NixOS 没有 `/lib64/ld-linux-x86-64.so.2`，一份动态链接的 ubuntu 产物在 NixOS 上起不来，而静态一份同时覆盖 NixOS／Alpine／老发行版／容器。**待探项已探明，后端不换**：`aws-lc-sys` 由 `reqwest → hyper-rustls → rustls → aws-lc-rs` 引入（`cargo tree -i aws-lc-sys`，2026 年本机实测），而 aws-lc-sys 0.44.0 的 crate 源码内自带 `src/x86_64_unknown_linux_musl_crypto.rs` 且 README 的 Pregenerated Bindings 表列出该三元组——**该目标在预生成绑定名单上**，故不需要 bindgen，rustls 后端保持 `aws-lc-rs`，不切 `ring`（少一个 crypto 后端就少一处与 Windows／macOS 产物不同的实现）。它仍需一套 musl 的 C 工具链编译 AWS-LC 源码，故该 job 装 `musl-tools` 并令 `CC_x86_64_unknown_linux_musl=musl-gcc`；cmake 已在 runner 镜像上。**这一条只在 CI 上成立而未在本机跑过**（本机无 Linux 亦无 musl 工具链），证据是依赖树与 crate 自带文件，不是一次绿色构建。**另记一处未清的债**：`xtask package` 经 `budget::binary_path` 只认 `target/release`，而 `--target` 构建落在 `target/<triple>/release`，故静态那一行不能调 `just package`，只能在 workflow 里把静态产物摆到打包器看的位置再走 `xtask sbom`／`xtask package`——正解是让 `binary_path` 认三元组，那属于 xtask 的权威，不在本卡的文件集内；②`flake.nix` 只管 devshell 与 `nix run`，版本从 `rust-toolchain.toml` **派生而不复述**（需要额外版本即红），不接管 Windows／macOS 发布路径，CI 上进 `platforms.yml` 的 nightly 而非 push 流水线。本条改判后，本节「只能由 CI 编译与 lint、不能由我运行验收」仍然成立（本机仍无 Linux），变的只是那个 job 不再只有 macOS。
- **`Report` 说的是已经发生的事**：拷到哪、搜索路径改没改（`AlreadyPresent` 与 `Append` 是两句不同的话）、广播成不成、以及「PATH 变更不会进已经开着的窗口」。**恒不说「安装成功」四个字**——人要知道的是下一步该开一个新窗口。

**本章测试**：`program_dir` 在两个平台各取本平台约定；`plan_append` 对空串、已含该目录（含大小写不同与带尾分隔符两形）、含其它目录三类输入分别给出正确的穷尽枚举；`plan_remove` 删得干净且保住其余段（含空段）；`plan_append` 之后 `plan_remove` 回到原值——**幂等与可逆是一对性质测试，不是一次手工观察**。判定既然只在 Windows 编译，这组测试也只在 Windows 编译：**测一个在本平台不存在的函数，测的是空**；推送门跑的正是 Windows，故这组测试每次推送都跑。

**本章验收（必须真做）**：本机 `install` 之后**开一个新的 PowerShell 窗口**敲 `sprawling`；随后 `--uninstall`，再开新窗口确认 `Get-Command sprawling` 为空。

## 8-10 第二个 wire 客户端（P3）

**为什么存在**：ARCHITECTURE §8 写着「the wire is the whole API；一个第二客户端就照着它写」，而今天只有一个客户端——按仓库自己的判据（§4：一个适配器是假想缝，两个才成立），`channels::wire` 因此是一条假想缝。

```rust
// bin::wire_client（形状 4 adapter）
pub(crate) struct Heard { pub frames: u32, pub refusals: u32 }
pub(crate) fn call(at: &str, frame: &str, token: Option<&str>, quiet: Duration) -> Result<Heard, AxError>;
pub(crate) fn enrol(at: &str, realm: &str, name: &str, value: &str) -> Result<String, AxError>;
pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)>;   // "realm/name"
```

- **握手在进程内算，不手抄**。`WIRE_V` 与 `schema_hash()` 直接取自 `channels`，故改一条命令名字时本客户端**不可能**落后。本卡因此删掉了那个一次性的 Python 探针——它在工作区外复刻了 `schema_hash()` 与 `IdemKey::derive()`，那本身就是第二个权威。
- **一帧发出，所有帧收回，直到城安静**。“安静”是一段无帧的时长（`--quiet-ms`，默认 2000），而不是帧数：一条 Dispatch 会产生多少事件是城的事，客户端猬不到。
- **输出是 JSONL，一行一帧**。发明一种人看的排版就是为 wire 里的每一个类型再写一遍它长什么样，而那份渲染一定会漂。
- **退出码带信息**：收到过 `Refusal` 退 1，否则退 0。一个驱动它的 agent 不应当为了知道「成不成」去解析 JSON。
- **`enrol` 只从 stdin 读，恒不从 argv 读**。argv 进进程表、进 shell 历史、进父进程的日志；这比浏览器路径更好的地方就在这里，因为页面那条路要先把明文拿进一个标签页的内存。**输出只有引用**，恒不回显值。
- **依赖不新增包**：`tokio-tungstenite` 正是 axum 的 `ws` 特性已经携带的那一份，直接依赖它在 `Cargo.lock` 里**增加零个包**（实测 496 → 496）；换一个别的 WebSocket 库就是把同一个协议的两份实现放进同一个二进制。不开 TLS：控制面走 `ws://`，而一座要经 TLS 到达的城是一座前面站着终结器的城。
- **未做且已知**：`/enroll` 仍在工人取走凭据之前就答 201（详 `channels-SPEC.md` §8）。`enrol` 因此报的是「已受理」而不是「已入库」，这句话写在输出里而不是留给人去撞。

**本章测试**：`split_reference` 对 `realm/name`、缺斜杠、空段、多斜杠四类输入给出正确答案；握手帧的 `wire_v` 与 `schema` 逐字节等于 `channels` 自己的值（这条断言就是「不存在第二份握手权威」的可执行形式）。真城验收：`call` 一条必被拒的命令，收到 `refusal` 且退 1。

## 8-11 控制台：服务中的那个终端不再是死胡同（P1）

**已有工作区的人（P0.01）**：`form_city(root, Adopt)` 取代 `init_city` 成为唯一的成城路径（后者是 `Adopt::Nothing` 的别名）。`Adopt` 是穷尽枚举而不是布尔：在既有工作旁边形成一座城、与把那些工作放到规则之下，是两件事，一个布尔会把它们说成一件事的一个设置。采纳走的是 `sprawling adopt` 的同一道门，于是创世时收进来的文件夹与一个月后收进来的受同一套规则治理。首屏因此长出第三个答案 `FirstScreen::Use(path)`——回车之外、`q` 之外的任何输入都是一个路径（去掉文件管理器加的引号）；**路径存不存在由调用方查并报**，屏幕自己去猜要么把真文件夹当错字拒了，要么在没人看过的位置造一座城。

**病灶**：`sprawling up` 打四行字然后阻塞到 Ctrl-C。那块屏幕是产品白白扔掉的一个面，也是一台没有浏览器的机器**仅有的**那一个面。

```rust
// bin::console（形状 1 decision；壳在一条线程里，判定全在纯函数）
pub(crate) enum Line {
    Nothing,
    Help,
    OpenWeb,
    Select(Address),
    Quit,
    Frame(Box<channels::ClientFrame>),   // 一个 wire 动词
    Work(String),                        // 普通一行：派给当前选中的 room
    Unknown { verb: String, nearest: Vec<String> },
}
pub(crate) fn parse(line: &str, selected: Option<&Address>) -> Line;
pub(crate) fn verbs() -> Vec<String>;      // 控制动词 ⊕ wire 动词
pub(crate) fn snake(camel: &str) -> String;
```

- **wire 动词表是投影，不是第二份手写清单**。`verbs()` 从 `channels::COMMAND_NAMES` 与 `QUERY_NAMES` 逐个转 snake_case 得来；一份手写清单就是第二套词汇，而它漂开时没有任何东西会发出声音。一条断言钉住这件事：每个 wire 名字都在动词表里。
- **控制动词另成一个穷尽枚举**（`/help`、`/web`、`/at`、`/quit`）。它们是**控制台自己的**动词，不在 wire 上，故不属于那张投影。两张表合并后仍不得重名，一条断言钉住。
- **控制台不做任何判定**。一行变成 `Command` 之后，走的是人在页面上点按钮走的**同一张桌子**（`CommandDesk`）与同一个 `Reply`。拒绝因此自动回到控制台，不需要为它另写一条回程——这正是 P2 那条回信地址的第二个消费者。
- **普通一行就是派活**。要人为一件活敲 `/dispatch {"addr":…}` 是把 JSON 当人机界面；选中一个 room（`/at`）后直接写任务，才是终端本来的手势。未选中任何 room 时拒，并说该敲什么。
- **不是 TTY 就不进控制台**。stdin 读到 EOF（管道、服务、CI）即退出控制台循环而**城照跑**：一座因为没人敲键盘而停止服务的城是一个以交互换服务的回归。
- **拒长表与图**。查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形；表格与图归浏览器。一个同时伺候两个主人的 CLI 是 CLI 文献里的反面教材。
- **`/web` 携配对令牌**，故没有人需要手拷一串东西。令牌在 `serve` 里只被读一次，控制台拿到的是那一次的副本，不重新读环境。
- **Ctrl-C 已是有序收口（P3.05）**：`serve` 在 `channels::serve` 与 `tokio::signal::ctrl_c` 之间 `select!`。收到信号后先停止接受连接，再 `CommandDesk::close()` 告诉 worker，worker **在读队列的同一处**读到它，于是正在跑的那条命令先跑完，`handoff_written` 是最后一行而不是某一行的中间。主线程 join worker 线程再返回——先返回的 main 会在那一行写出来之前结束进程。
  - **`DeskWait::Close` 与 `Gone` 不是一回事**：前者是人选择停城，值一份 Handoff；后者是桌子自己坏了，那座城已经写不出 Handoff 了。
  - **收口不是一条 Command**：能被拼出来的线上帧就是陌生人停掉别人城市的一条路。`closing` 是台子上的一个 `AtomicBool`，只有起城的那个进程按得动。
  - **Windows 交两个信号，本城两个都收**：控制台会发 Ctrl-C 与 Ctrl-Break。一座在其中一个上有序收口、在另一个上暴死的城，等于同一个手势有两种行为，而决定用哪一种的是人碰巧按了哪个键。其他平台只有一个。
  - **裁定与代价**：根 `Cargo.toml` 给 tokio 开 `signal` feature。TODO 当时写的「不新增包」是错的——unix 上它引入 `signal-hook-registry`（Apache-2.0/MIT，deny 表内），依赖数 496 → 497。这一条如实记在这里。

**本章测试**：每个 `COMMAND_NAMES`／`QUERY_NAMES` 都在 `verbs()` 里（这就是「投影而非第二份清单」的可执行形式）；控制动词与 wire 动词不重名；`snake` 对 `AttachEndpoint`／`RunView` 给出预期串；空行、`/quit`、`/at <addr>`、普通文本（选中与未选中两情形）、未知动词（携最接近的几个）各得正确枚举。

## 8-12 prefix 自己带上它要求模型读的东西（P6.03）

**病灶**（把四个段拼出来才看得见，不是读代码读出来的）：Building 段是 12 字节的地址，Run 段是 71 字节的 `cas:b3-…` 内容哈希。而 City.md 要求模型「read `BUILDING.md`」「`FULL READ:` 给出你的 `JOB.md` 的路径」——**两句话指的东西一个都不在 prompt 里，而城里八个工具没有一个解析 `cas:`**。第三处：City.md 无条件说「你的第一条消息正好有三行」，这对没有 `JOB.md` 的主 Agent 是假的。

```rust
// bin::assembly（形状 4 适配器；四个段的填充点）
fn building_segment(city_root: &Path, addr: &Address, building: &Address) -> Vec<u8>;
fn run_segment(city_root: &Path, building: &Address, brief: &city::RunBrief) -> Vec<u8>;
```

| 槽 | 装什么 | 稳定性依据 |
|---|---|---|
| City | 城里那份 `City.md` | 整座城不变 |
| Building | 地址 ＋ `.sprawling/BUILDING.md` 全文 | 人写、任何写域够不到、整个 Run 不变 |
| Resident | `URBANITE.md`（或无身份那 106 字节）＋ catalog | 同一个 Resident 每次 Run 同样的字节 |
| Run | `Handoff.md`（写过的话）＋ 本次 brief | 每次 Run 一份 |

- **注入的判据是稳定性，不是重要性**。`Roadmap.md` 与 `Memo.md` **故意不注入**：Run 自己会改它们（`plan` 工具持有 roadmap 全文），冻进 prefix 就是第二个权威——模型会读到自己刚改过的旧副本。它们的正路是工具，不是 prefix。
- **Run 段的次序是「上一场留下的」在前、「这一次要做的」在后**：最后读到的东西是被执行的东西。
- **空白表单不占 prefix 字节**：`city::handoff` 认出还是模板原样的 `Handoff.md` 并答 `None`。判据是模板自己的括号提示行——写过的会话会把它们换掉。
- **内容哈希整个退出 prompt**。它在 Ledger 里记了两遍（`run.rs:126` 的 pin 与 `:141` 的 started），溯源不依赖模型看见它；`FULL READ:` 那一行随之消失。
- **CAS 的 pin 改钉 brief 的正文**，两条臂都钉：一次没人派任务的会话，pin 里是「说明没有人派」的那几句，于是 Ledger 的 `job` locator 恒解析得到 Run 段真正携带过的字节，而不是一个从未被写出的文件。

**本章测试**：一次真派活后，provider 收到的请求里含楼规原文（`confidential: false`）、含上一场的 Handoff 正文、含本次 Goal，且**不含** `FULL READ` 与 `cas:b3-`；一次无 Goal 的派活不写 `JOB.md`，请求里说出「working with the person directly」且不把人那句话包成 `Task:` 表单。

## 8-13 一封信与一次敲门（P3.07）

**病灶**（一次真机会话拿出来的，不是读代码读出来的）：两位居民在同一栋楼里谈价，发信的那一跑连续五次 `signal pull` 等一封**在它自己那一跑里物理上不可能到达**的回信，最后以 `limit` 冻结；而收信人根本没有在跑。证据：同一条链上 `signal_enqueued` 落在 `run_frozen` 之后两行。

```rust
// bin::assembly
struct Knock { addr: Address, from: String, mode: runtime::Mode, budget: kernel::BudgetCap }
impl RunWorker {
    fn knock(&mut self, signal: &Signal, speaker: &Address, mode, budget) -> Result<(), AxError>;
    fn answer_knocks(&mut self);   // 成波排干，循环而非递归
}
```

**两种送达，分法是收信人在不在**：

| 收信人的状态 | 机制 | 落点 |
|---|---|---|
| 正在跑 | 信从门缝塑进去——steer 型 Signal，`SignalDesk::take_steer` 在安全点取走 | 追在下一次工具结果末尾，前缀 `@发件人地址` |
| 没在跑 | 敲门——投递后入 `knocks`，本轮派活结束后 `answer_knocks` 为他开一跑 | 新 Run 的 brief，同样写明 `@发件人地址` |

- **人压过居民**：中断源先问人的命令队列（Cancel 再 Steer），空手才问本屋信箱。
- **属名不是装饰，是回信地址**：另一个 agent 的话氒不得以人的身份进窗口。类型已经把它变成判定（只有 `Steer::from_person` 写得出 `user`）；本卡把同一条规则延到敲门路上——被叫醒的一跑，其 brief 第一句就是「@X signalled you. This run exists because that signal arrived: nobody else asked for it.」。一份读起来像人写的 brief 会让每一封回信寄错地方。
- **敲门敲的是 Resident，不是一段已封存的对话**：冻结的 Run 是历史，历史只读而不叫醒；被开出来的是那个地址上住户的**一跑新的 Run**，它靠 `Handoff.md` 接住上一场——那正是为穿过一次冻结而造的那件东西。没有 `URBANITE.md` 的地址因此不敲：它是一间房而不是一个人，信就在那儿等到人派个住户过去。
- **不设叫醒预算（人的定谳）**：什么时候该停下来是对话里那几位居民的事，城市的活是把话送到。人要让某个居民不再被打扰，用的是已有的 Halt，`dispatch_in` 当场拒一个被 halt 的 scope。
- **一次对话一道底都没有**：**这座城没有金额上限，也没有回合上限**，那是定谳而不是遗漏：什么时候停下来归对话里的居民，花了多少事后从 Ledger 报出来。card-11.7 把从无调用方的 spend 门连同它判的 ladder 、以及 `DISPATCH_TURN_BUDGET` 一并删除（kernel-SPEC §11-7、本文 §8-40），刹车此后只剩 `Cancel`（停一件）与 `Halt`（停一片）。
- **一个敲不成不连坐发件人**：叫不醒的人进诊断日志，不把发件那一跑的 dispatch 弄成失败。

**本章测试**：一位居民向另一位发信，无人再派活而收信人自己跑了一跑，且其 brief 里带着发件人的地址；向一个无 `URBANITE.md` 的房间发信不开任何 Run，信仍在队里。

## 8-14 幂等键里的那个时钟（P3.08）

**病灶**（真机会话抓出来的）：同一跑里两次 `read` 被拒为 `this call was already made`，下一回合同一路径又读得干净。原因在一行里：`IdemKey::derive(&run_id, Seq::new(t.value()), call.name.as_bytes())`——

一、**它取了一个时钟**（回合的毫秒戳），而确定性第七条写着「IdemKey 恒不得源于时钟或随机数」；二、**它不含参数**，于是同一回合内对同一件工具的任两次调用归为一键——两次 `edit` 也会，而那是丢写。

现形：`(run_id, 本跑内的调用序号, 工具名＋参数 JSON)`。序号由闭包自己的计数器给，重放同一段历史得同一串键。两次不同的调用是两个键，都跑；同一个位置被重放是同一个键，去重正是为此而存在。

## 8-15 装配层长出一扇门（整修卡 R2.01）

```rust
// crates/sprawling/src/lib.rs —— 索引文件，只准声明（modmap 已看守）
pub mod assembly;
pub mod console;
pub mod firstrun;
mod mcp_http;      // 只经 assembly 到达
mod mcp_stdio;     // 同上

// assembly：跨出 crate 的项，逐个放行
pub struct InitReport { pub ledger_dir, pub genesis, pub standing, pub adopted }
pub enum Adopt { Nothing, EveryFolder }
pub fn has_history(&Path) -> bool;
pub fn init_city(&Path) -> Result<InitReport, AxError>;
pub fn form_city(&Path, Adopt) -> Result<InitReport, AxError>;
pub fn open_vault() -> (gateway::Custodian, Option<Payload>);
pub struct Serving { /* 八个字段全 pub：调用方构造它 */ }
pub async fn serve(Serving) -> Result<(), AxError>;
pub struct ScanReport { pub waiting_approvals: usize /* lines、closed_calls 不跨出 */ }
impl ScanReport { pub fn summary(&self) -> String; }
pub struct RunWorker;
impl RunWorker {
    pub fn new(&Path, gateway::Custodian, Diagnostics) -> Result<Self, AxError>;
    pub fn handle(&mut self, channels::Command) -> Result<(), AxError>;
    pub fn startup_scan(&mut self) -> Result<ScanReport, AxError>;
    pub fn fork(&mut self, RunId, Seq, Option<Address>) -> Result<RunId, AxError>;
    pub fn adopt_building(&mut self, Address) -> Result<(), AxError>;
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

- **这张卡为什么存在**：`crates/sprawling` 至今只有 `src/main.rs`，`mod assembly` 是私有模块，于是工作区里**没有任何东西能依赖它**——4377 行生产代码（含 1058 行的 `dispatch_in`）只由同文件内的 66 个测试看守，citysim 与任何 `tests/` 都够不到。同一个事实还有第二个后果：它是唯一带 SPEC 却逃过 `apisync` 的 crate，因为 `spec_crates` 以 `src/lib.rs` 是否存在为判据。加一个 lib target 一并了结两件。
- **`pub mod` 而非扁平 facade**：§12 模块表以 `bin::assembly`／`bin::console`／`bin::firstrun` 命名模块，模块名本身是已记录的架构事实；折成 `sprawling::init_city` 会抹掉这层限定，而本 crate `publish = false`，C-REEXPORT 要替第三方省的那段路径没有受益人。**取窄的地方在项，不在模块**：只有跨出 crate 的项改 `pub`，其余留 `pub(crate)`——公开面因此是逐项决定的，不是逐模块授予的。
- **`mcp_http` 与 `mcp_stdio` 保持私有**：只经 `assembly` 到达（`assembly.rs` 的 `McpLink`），没有第二个调用方。
- **`install` 与 `wire_client` 留在 bin**：前者把二进制放上 PATH，后者从终端连一座已服务的城并从 stdin 读 enrolment——两者都是关于命令行的，不是关于城的，且除 `main` 外零引用。留在 bin 让公开面少六项。
- **`handle` 进公开面不是为测试拓宽**：AGENTS.md 写着「Tests use the same doors as production code」。`handle` 正是服务中的 worker 循环走的那扇门，把它命名出来是承认已有的门。反过来，那 66 个内部测试**不搬去 `tests/`**：它们触及 `rebuild_views`／`read_building`／`run_id_for` 这类内部项，搬迁会为测试拓宽公开面，正是同一条规矩禁止的事。本 crate 的文件长度因此在本卡内不变——它变短要等拆 `dispatch_in` 那张卡把生产代码连同其测试一起搬走。
- **`ScanReport` 只放行一个字段**：`main` 读 `waiting_approvals` 决定是否多印一行，`lines` 与 `closed_calls` 只进 `summary()`。按需放行而非按结构对齐——`InitReport` 四个字段全跨出，是因为 `report_standing` 四个全读。
- **零行为变更**：`main.rs` 只改开头的声明块（七行 `mod` → 两行 `mod` ＋ 一行 `use sprawling::{assembly, console, firstrun}`），其余调用点逐字节不变。`Cargo.toml` 不改：Cargo 对同一 package 自动发现 `src/lib.rs` 与 `src/main.rs` 两个 target，OUT_DIR 对两者相同，`include!(client_embed.rs)` 与 `DEPENDENCIES` 因此留在 `main.rs` 原地。
- **红**：`crates/sprawling/tests/assembly_door.rs` 走 `init_city → RunWorker::new → handle(Command::CreateBuilding) → 读 InitReport.ledger_dir 下的账本`，断言 `building_created` 落账。本卡之前它连编译都过不去（`sprawling` 这个 crate 名不存在），这就是「这条测试咬得动」的证据。
- **门禁连带**：`apisync` 自本卡起把 `sprawling` 纳入契约，`xtask/api-baselines/sprawling.txt` 随本卡生成（`guard` 的 `PRODUCED_PREFIXES` 已豁免该目录，不需 `Verdict:`）；`header` 要求 `lib.rs` 与新测试文件各带三行 MPL 通告；`modmap` 对 `*/lib.rs` 自动按索引文件判定，只准 `mod`／`use`／`pub use`／注释／属性——facade 因此只能是声明，正是要的形状。
- **一处文档更正**：ARCHITECTURE.md §3 写着「citysim is a second assembly layer: the same code with simulated adapters」。此句与现实不符——`citysim/Cargo.toml` 依赖 kernel／memory／runtime／gateway／eval，其中没有 sprawling；`run_scenario` 手工构造 `RunPlan`，够到的最高层是 `runtime::run::drive`。本卡使 assembly **可被依赖**，但没有让 citysim 依赖它：模型适配器仍由 `adapter_for` 从 `EndpointBook` 内部构造，那条缝要不要倒置是另一个决定。按 AGENTS.md「reality wins and the document is corrected first, with its reason」，本卡先把这句改成现实。

## 8-16 读不了的计划不再被报成被人改过的计划（整修卡 R2.06）

**病灶**：`dispatch_in` 里三处把失败抹平成默认值。

```rust
let plan_text = std::fs::read_to_string(&plan_path).unwrap_or_default();          // 驱动前：喂给 ClaimDesk
let shelf = city::archive_index(&self.city_root, building.addr()).unwrap_or_default();  // 驱动前：喂给 ArchiveDesk
let on_disk = std::fs::read_to_string(&plan_path).unwrap_or_default();           // 驱动后：落盘前的 compare-and-swap
```

第三处最重。那一段的注释自述它存在的理由——「each effect is checked against the file **as it stands now** … the losing claim is dropped with a diagnostic instead of overwriting somebody's row」。但读失败使 `on_disk` 成为空串，`still_true` 对空文档恒为 `false`，于是每一条 claim 都落入 stale 分支，人收到的诊断是「row … moved before this run's claim landed」——**一个从未发生的并发冲突**。他们会去查另一个居民，而真正要修的是一个读不开的文件。

（我先假设的是更重的后果——读失败→`stale` 为空→`write_plan` 覆盖真实计划。核实 `still_true` 后否定了它：空文档下 `check_roadmap_shape` 不产 `WellFormed`，因此恒返 `false`。不存在数据丢失，只存在误报。）

第二处：`city::archive::index` 自己已经实现了正确契约（目录不在 → `Ok(空)`，真失败 → `Err`），所以 `.unwrap_or_default()` 恰好只扯掉真失败；换成 `?` 即可，不需新机制。

**现形**：

- `assembly::plan_path` 删除。它在 `city` 之外拼了一遍 `city_root/<addr>/Roadmap.md`，而 `ROADMAP_FILE` 住在 `city::spine_files`——两份「计划在哪里」的权威。改走新增的 `city::roadmap_path`。
- 两处读全走 `city::roadmap`：仅 `NotFound` 答空串，其余以 `E_STORAGE_FATAL` 上报并带路径。一栋还没铺计划的楼确实没有计划，那不是失败；其余一切都是。
- `archive_index(…).unwrap_or_default()` → `?`。

**拒而不是降级**：计划是共享地面。读不到它就开跑，会花掉一次模型调用去产生一批注定被丢弃的 claim。在派活口上拒，人拿到的是路径和修法。

**红**：向一栋 `Roadmap.md` 是**目录**的楼派活（`read_to_string` 因此以非 `NotFound` 失败，无需权限把戏）。本卡之前：派活成功，诊断行说「row moved」。本卡之后：派活被拒，错误点名那个文件。

**影响面**：`city` 公开面增两项（基线同提交更新）。正常楼不受影响——`spine_files::lay_out` 给每栋新楼都铺了 `Roadmap.md`，而未铺的情形仍走 `NotFound` 答空串这条。`assembly.rs:219`（楼页读 Roadmap）同属一族但爆炸半径不同——那里读不到只是页上少一块，不会变成误报——本卡不动。

## 8-17 一次验证遍历，三个折叠（整修卡 R2.02）

```rust
pub(crate) struct Standing { pub(crate) book: gateway::EndpointBook, governance: Governance, collaboration: Collaboration }
impl Standing { pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError>; }

impl Governance { fn empty() -> Governance; fn absorb(&mut self, record: &EventRecord); }
struct CollaborationFold { … }   // 暂存 enqueued／consumed，`settle` 产 Collaboration
```

`rebuild_book`／`rebuild_governance`／`rebuild_collaboration` 三个函数删除。

- **这张卡不是缺陷修复，我测过了**。我原本怀疑三处实现会漂移（`rebuild_governance` 管 `granted` 与 `CityHalted`，`govern` 不管，`answer_approval`／`set_admission` 各自直改字段）。新测试 `what_a_worker_holds_is_what_a_restart_rebuilds` 实验否定了它：派一次活、发一条信号之后，活 worker 与重建结果逐项相等。那条测试因此不是本卡的红，而是让合并安全的护栏；它同时把一条四处代码都依赖、却从未被断言过的形状-7 性质变成了可红的。
- **本卡以测量收口而非以红转绿收口**，理由写在上一条：没有可咬的红，因为没有缺陷。本机实测（windows-x86_64, 16 core，release，外部探针经 `sprawling` 的 lib 门驱动 `RunWorker::new`）：

  | 记录数 | 改前 | 改后 |
  |---:|---:|---:|
  | 5,000 | 136.6 ms | 44.6 ms |
  | 20,000 | 538.7 ms | 175.6 ms |
  | 50,000 | 1856.1 ms | 436.2 ms |

  这是每一次 `serve`、`resume`、`fork`、`adopt` 都要付的钱。
- **为何不是 4 → 1 而是 4 → 2**：`RunWorker::new` 自己还要 `JsonlLedger::open`（尾部恢复）读一遍，而 `serve` 另走 `rebuild_views` 一遍。把 `Views` 也并进来要改 `serve` 的所有权形状（它住在 `Arc<Mutex<_>>` 里与查询侧共享，而 worker 在自己线程上）——那属于 `dispatch_in` 拆分那张卡，不在本卡内顺手做。
- **暂存只给真需要的一项**：`CollaborationFold` 只暂存 signals，因为队列是 `enqueued` 减 `consumed` 而两者到达顺序任意；book、governance、goals、requests 都是逐条即结的，所以不暂存。
- **验证不动位**：链验仍在折叠之前。一部不能自证的历史，不是这三个视图中任何一个可以建在上面的历史。

## 8-18 审查中的 run 不再把自己的决策直接放上楼的书架（整修卡 R2.03）

**病灶**：`dispatch_in` 的档案回收段写的是 `city::file_archive(&self.city_root, …)`，而不是 `&write_root`。ARCHITECTURE.md §5 把输线设计写在明处——「A building under review gives every run its own tree … Nothing it writes is visible until somebody else checks it — the losing line of the design made physical rather than promised」。档案不在那棵树里。

具体危害：书架回头喂给 `ArchiveDesk`，成为模型看到的「这栋楼已经知道什么」。一个**被驳回**的 PR 里的决策因此会留在架上，变成后续 run 的前提。

**现形两步，缺一不可**：

1. `city::file_archive(&write_root, …)`——落进围栏。
2. 持有租约时，`fence_scope` 从**房间**改为**楼**。否则第一步把泄漏换成了静默丢失：`wave_pre` 只暂存 `<scope>/*`，而档案在 `<building>/Archive/…`，在房间作用域之外，不会进提交，租约一释放就没了。在**自己独占的** worktree 里暂存整栋楼是安全的：那棵树里变化过的东西全是这个 run 的。无租约时围栏仍在房间——那才是一个 run 在城里唯一可写的地方。

**与计划的不对称是故意的**：`Roadmap.md` 恒写回城里，因为它是共享地面且带着对当前文件的 compare-and-swap（那段注释自述了理由）。档案没有这样的声明，也没有守卫，所以它是漏而不是决定。

**card-2.2**：四处检查点调用点（`workbench/standing.rs` 的 `ensure_base`、`workbench/tools.rs` 的 `with_checkpoint`、`driving.rs` 的逐波围栏、`reviewing.rs` 的 `PrEffect::Opened`）都从 `Site` 与 `RunWorker` 手上凑齐一个 `memory::Provenance`（run id、resident 地址、选中的模型 id 与思考档位、城的创世哈希）交给 memory；城的创世哈希由 `memory::Provenance::city_of(ledger_dir)` 只读账本首段第一行得出。

**红**：在 `review: true` 的楼里派一次带 `archive` 工具调用的活，断言书架仍空。本卡之前它拿到 `[Entry { kind: Decision, … lab\Archive\decision\… }]`。同一条测试接着让第二位居民检查并合入，断言书架变为 1 条——**两半同一条测试**，因为只测前半的修法可以是「干脆不写」。

## 8-19 沙箱接上（整修卡 R2.04）

**病灶**：`dispatch_in` 把 `Box::new(runtime::AbsentSandbox)` 写成字面量，而 `crates/sprawling/Cargo.toml` 没有任何 feature 到达 `runtime/wasm`。于是 `runtime::WasmtimeSandbox` 在 `runtime/tests/sandbox_a10.rs` 之外**没有调用方**，任何 sprawling 构建都到不了执行引擎。

`AbsentSandbox` 的三段式拒绝里写着 recovery：「use the program arm, or install a build with the `wasm` feature」。**那样的构建不存在。** 这正是 P4.03 立下的判据——只写在文里、无人执行的 recovery 等于没有 recovery——这次是只写在错误消息里、无人可安装的构建。

ARCHITECTURE.md §2 把 wasmtime 列进技术栈并声明了代价（「Cost: an optional feature; a build without it refuses tool execution in three parts rather than pretending」），措辞预设了存在带该 feature 的构建。

**现形**：
- `crates/sprawling/Cargo.toml` 增 `[features] sandbox = ["runtime/wasm"]`。
- 引擎的选择收进一个函数 `execution_engine()`，两条 `#[cfg]` 臂各一个实现，`dispatch_in` 的构造点因此不随 feature 改变形状。
- **带引擎的构建起不来引擎就拒派活，不回落**。回落是「人以为跑在沙箱里、其实没有」的由来。

**默认仍为关**：这是 ARCHITECTURE.md 记录过的取舍（wasmtime 是一大块二进制），本卡不改默认，只让开关存在。`just check` 走 `--all-features`，所以带 feature 的那条臂进门禁；`just dist` 不带，所以交付形态与体积预算不变。

**红是编译红而非行为红，这里说明白**：改动前 `execution_engine` 不存在，测试连编译都过不去。行为面的红需要一个真的 `python.wasm` 与 `SPRAWLING_PYTHON_WASM`，那是交付形态的事，不在本卡内。新测试只在 `cfg(feature = "sandbox")` 下存在，断言引擎给出的不是「this build carries no execution engine」那句话。

## 8-20 交接件读不了不再等于没有交接件（整修卡 R2.05）

`city::handoff` 的 `.ok()?` 与 R2.06 修掉的那三处同族，且它喂的是 **prefix 的 run 段**——下一次会话读到的第一样东西。三件事（文件不在／读不了／仍是空白表单）原先并为一个 `None`。

现形与 `roadmap` 同：`Result<Option<String>, AxError>`，`None` 只说「没有值得带走的东西」，`NotFound` 归入其中，其余上报并带路径。同时补 `handoff_path` 与把 `HANDOFF_FILE` 转 `pub`——红测要点名那个文件，而在别处拼一遍文件名就是第二份权威。

`run_segment` 因此转为 `Result<Vec<u8>, AxError>`；它只有一个调用方（prefix 的四段装配），所以波及面就是那一处 `?`。

**红**：向一栋 `Handoff.md` 是目录的楼派活，`expect_err` 撞上 `Ok(())`。`city` 侧另加一条单测，把三件事排成三行断言。

## 8-21 控制台读得到它身处的那座城（整修卡 R2.07）

**病灶（两处，同一个不对称）**：控制台与 socket 拿的是同一张桌子（`CommandDesk`）与同一条事件流，唯独**读**这一路没接上。

1. `console::post` 对 `ClientFrame::Query` 只印一句「a question is answered over the wire: `sprawling call '…'`」——它请人开第二个终端，去问一座人已经身处其中的城。而 `assembly::serve` 早已构造出 `queries: Arc<dyn Fn(Query) -> Result<Answer, AxError>>` 并只交给 socket。§8-11 自述「查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形」——**这句话今天是假的**，本卡使它为真。
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
    Arc<dyn Fn(channels::Query) -> Result<channels::Answer, AxError> + Send + Sync>;

pub(crate) enum Line { …, Serving }   // 控制动词从四个变五个

/// 进程自己知道的事实 ⊕ 一次 Metrics 的答案 → 一屏。纯。
pub(crate) fn serving(terminal: &Terminal, vitals: &channels::MetricsAnswer, pid: u32) -> String;
```

- **答询走同一个函数，不是第二个权威**。`post` 的 Query 臂改调 `Answering`，与 `channels::server` 的 `SessionStep::Answer` 是同一个 `Arc`；控制台答出来的数字与浏览器看到的数字不可能不同，因为它们是同一次调用。
- **`/serving` 是渲染，不是来源**。城侧那几个数（几条 run 在跑、几件事等人、几栋楼）全部来自一次 `Query::Metrics`；`/metrics` 仍印它的 JSONL 原样，与 `sprawling call` 同形。两个动词，两个问题，无重叠：`/serving` 答「这个进程开在哪、门朝谁开」，`/metrics` 答「这座城里有多少什么」。
- **动词名不与既有概念撞车**。`status` 在 `docs/glossary.md` 里已经是**工具**的名字（「答一次 run 自己的处境」），一名一义是门禁事项，故控制台这个动词叫 `serving`——它印的正是 `assembly::Serving` 持有的那几样东西，沿用已在库内的词。
- **常驻内存不进这一屏，理由记在这里**。「resident 在本平台叫什么」的唯一权威是 `xtask::mem`（Linux `smaps_rollup` Pss／macOS `ps rss`／Windows `WorkingSet64`），而 `xtask` 只依赖 `kernel`——让它依赖产品会使每次门禁编译整个 workspace。在 bin 里再抄一张三平台表，正是那个模块自己的 doc comment 警告的「三份权威」。`/serving` 因此印出本进程 **pid**，`cargo xtask mem <pid>` 只差一次粘贴。**翻案条件**：人裁定新增第十三个 unit 承载这一个计数器（ARCHITECTURE.md §3 的拓扑是 add-only 且需裁决），届时两个调用方共用一份定义。

**红**：一条测试把 `Line::Serving` 之外的路径全部钉住不动，另一条驱动 `drive` 读入 `/metrics`，断言输出里有 `MetricsAnswer` 的 JSON 而**不含** `sprawling call`——本卡之前它撞上那句转介。第三条断言 `serving()` 的那一屏同时含端口、`runs`、与 pid。

## 8-22 面向网络的那扇门自己铸钥匙，页面把钥匙递上去（整修卡 R2.08）

**病灶（一条端到端全断的链，四段里断三段）**：把 WebUI 暴露到回环之外这件事，今天**做不到**。

| 段 | 今天 | 判据 |
|---|---|---|
| 铸 | `PairingToken::mint` **在产品里没有调用方**，只有测试用它 | `grep mint(` 只命中 `channels/tests` |
| 拒 | `decide_bind` 在没有 `SPRAWLING_PAIRING_TOKEN` 时拒绝任何非回环地址 | `server.rs:63` |
| 携 | `console::web_url` 把 `?token=…` 挂到 URL 上 | `console.rs` |
| 递 | `web::app` 写死 `Link::new(None)`，且 `socket_url()` 只取 `host`，**查询串整段丢掉** | `app.rs:1679`、`socket.rs:311` |

于是：不配置令牌起不来；配置了令牌，页面握手时不出示任何东西，`decide_handshake` 照 `server.rs:306` 拒之。**一座暴露出去的城连自己的 WebUI 都进不来。** 这不是两个缺陷，是一条链，所以一张卡修完，不留「钥匙铸出来了但没人能用」的中间态。

**对用户提案的更正，记为落选方案**：「BLAKE3 随机抽一个文件的 hex 值当口令」的熵是 `log2(候选文件数)`，不是摘要宽度——十万个文件约 17 bit，可当场穷举；且被抽中的文件内容常常是公开的（仓库里的源文件、依赖的许可证）。攻击者只要知道文件集合就把 256 bit 的外观还原成一次目录枚举。`PairingToken::mint` 收 32 字节 OS 熵、经 29 符号字母表给出四组五位（约 97 bit），**且它的 doc 明写就是为「显示一次」而设**。本卡用它，不另造。

```rust
// bin::keying（形状 1 decision；纯，穷尽，无 I/O、无熵）
pub(crate) enum Keying {
    /// 回环：这台机器自己，什么都不用出示。
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

- **`decide_bind` 不动**。它仍是那条守卫；assembly 只是**在 socket 存在之前先把它满足了**，所以它 doc 里那句「there is no window in which the port is open and unauthenticated」原样成立。拒绝臂在 `channels` 自己的测试与任何第三方 embedder 处仍可达。
- **人配置过的优先**。`SPRAWLING_PAIRING_TOKEN` 在场就 `from_configured` 采纳，不覆盖、不显示——我们没见过它被铸出来，印它就是把一个长期口令又抄进一处日志。
- **铸出来的 code 不落盘、不进 Ledger、不进 diagnostics**。进程结束即失效，这就是「一次性」。它只经两处：显示一次的那一行，与 `Terminal.token`（`/web` 据此拼出带钥匙的 URL）——后者今天已经持有明文，本卡不扩大它的存放面。
- **页面这一段是纯函数加一次读**（Humble Object，ARCHITECTURE.md §9）：`web::socket::token_in(search) -> Option<String>` 对查询串取值，可在非 wasm 目标上测；`pairing_token()` 只在 wasm 下多一次 `location.search()`，不含判定。`app.rs` 的 `Link::new(None)` 改为 `Link::new(crate::socket::pairing_token())`。
- **令牌留在查询串里是既有裁决的延续**，不是本卡新开的：`console::web_url` 的 doc 已写明这一取舍（「A token in a query string is a token in the browser's history, and that is the trade this makes deliberately」），替代方案是人在两个窗口之间手抄一个秘密，然后抄错并粘到更糟的地方。改存 `sessionStorage` 会把钥匙放进同源 JS 读得到的地方——对一座**公网暴露**的城，那比浏览器历史更坏，故不改。

**红（三条，每条咬住一段）**：`Keying::decide` 对四格（回环／暴露 × 配置过／没有）给出的枚举——本卡之前 `keying` 不存在，是编译红；`token_in` 对 `?token=abc`、`?a=1&token=abc`、`?token=`、空串的四个答案；以及 `web::socket` 那条握手测试，断言 `Link::new(token_in(...))` 发出的 `Hello.token` 非空——本卡之前 `Link::new(None)` 使它恒 `None`。端到端那一段（真浏览器对真暴露端口）落在 V9，是人跑的命令而非门禁，如 ARCHITECTURE.md §11 所记。

## 8-23 委派下去的活带着派它的那份预算（整修卡 R2.09；card-11.7 后已作废）

**本节记的是历史，不是现状**：card-11.7 删去 `BudgetCap`／`SpendVerdict`／`admit_spend` 与 `kernel::gate::spend`，`Dispatch` 不再携 `budget`，本节修的那条传递路径连同被传的值一起不存在了。留下它是因为它记着一件仍然成立的事——**危害的形状是「对模型说假话」**——那正是 card-11.7 选择删而不是补执行者的理由。以下按当时的时态读。

**病灶**：`knock` 携父 run 的 `budget`，其注释明写「Carried rather than defaulted: an answer belongs to the same piece of work as the question, and a run with no ceiling is the one failure with no floor under it」；而同一个文件里 `dispatch_in` 的**委派**分支写 `kernel::BudgetCap::default()`。委派比敲门更是同一件活——一个 delegate 就是替父 run 做事的——却是唯一被清零的那条路。

**它今天可达，不是潜在的**。`channels::WireCommand::Dispatch` 带 `budget` 字段，`sprawling call` 与 `protocol::acp` 都能填。人在页面上填不了（ARCHITECTURE.md §5 步 1：「The frame carries no budget」），但**页面不是唯一的客户端**，而 wire 就是全部 API。

**危害的形状是「对模型说假话」，不是超支**。`BudgetCap` 今天没有执行者：`kernel::budget` 的 ladder 与 `SpendVerdict` 只有 `kernel::gate` 自己的测试在走，它在本 crate 里唯一的消费者是 `StatusTool`。所以一个 delegate 向模型报告自己预算为零，而它的父 run 报告的是真数。这与 §8-12 记下的那一类同族：「City.md 让模型调 `status` 问这些，而一个照做的模型拿到一排零，于是学会不再问」。修它不是为了今天省钱，是为了那一行不再是假的。

**修**：`dispatch_in` 的委派把 `budget` 传下去，与 `knock` 同形。`BudgetCap` 是 `Copy`，所以是一个词。

**红**：以 `BudgetCap { usd: 250_000, tokens: 4_000 }` 派一次会委派的活，父子两个 run 各调一次 `status`，按**状态块里的 `addr:`** 分辨谁读的哪一行。父为对照组（两边都绿），子为受试组（本卡之前是 `0 usd_micros, 0 tokens`）。

——**数个请求体里包含那串字」不能作判据**：对话携带自己的历史，同一个 run 的 status 答案会出现在它之后每一次请求里，数体等于把父数了两遍。我的首版测试就是这么写的，**未改代码即绿**，记在这里以免重踩。

**本卡另查出一处更深的，归入待办**：`fn dispatch(addr, task, goal)`（审批应答后续活的那条路）同样写死 `BudgetCap::default()`，而它**无法只靠改一个词修好**：`BlockedJob { addr, task, goal }` 没有装天花板的字段，而 `blocked_job` 是从 `run_started` 的账本记录重建它的——那条记录里没有预算。于是一个带天花板派出、因委派而停下来等人批的 run，被批准后续上的那一跑天花板归零。

**它与交接件上的第 4 项（`blocked_job` 不再扫全史）是同一个改动**：甲案（`Governance` 加 `origins: BTreeMap<RunId, BlockedJob>`）一并解开两者——内存里的 `BlockedJob` 想带几个字段就带几个，不动账本 payload，也不需核黄金账本；乙案（改 `approval_requested` payload）则要把预算一并写进去。**未定事项**：甲案是进程内存，而重启后的 worker 从账本重建；若 origins 不重建，重启前提出、重启后才被批的项就接不上活。这一点在选定甲案前必须先用测试回答（现行全史扫描没有这个问题，这是它唯一的优点）。

## 8-24 一条效应先成为账本行，再成为这座城（整修卡 R2.10）

```rust
// crates/sprawling/src/effect.rs —— ARCHITECTURE.md §12 bin::effect，形状 2（值类型）
pub(crate) struct Line { who: String, addr: Address, kind: EventKind, data: Payload }

/// 一张桌子留下的全部效应：它们成为的行，以及行之后才允许发生的变化。
pub(crate) struct Landing { lines: Vec<Line>, then: Then }   // 两个字段都是私有的

pub(crate) enum Then { Nothing, Deliver(Vec<collab::Signal>), Hold(Vec<GoalEntry>),
                       Roadmap { path: PathBuf, text: String }, Shelf(Vec<Filing>) }

impl Landing {
    pub(crate) fn signals(Vec<SignalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub(crate) fn goals(Vec<GoalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub(crate) fn discards(Vec<Payload>, room: &Address, who: &str) -> Landing;
    pub(crate) fn shelf(Vec<ArchiveEffect>, write_root, building, at, room, who) -> Result<Landing, AxError>;
    /// 先走完每一行，再把变化交出去。这是 `Then` 唯一的出口。
    pub(crate) fn record(self, &mut impl FnMut(Line) -> Result<(), AxError>) -> Result<Then, AxError>;
}

/// 一跑对共享计划做的事。两种而无第三种：计划是整份写回去的。
pub(crate) enum Claims { Landed(Box<Landing>), Stale(Vec<u64>) }
impl Claims { pub(crate) fn of(&[ClaimEffect], on_disk: &str, text: String, path, room, who) -> Result<Claims, AxError>; }

// 装配层那一扇门（assembly）：五张桌子都走它，`Then` 的 match 穷尽
impl RunWorker { fn settle(&mut self, RunId, from: &Address, Mode, BudgetCap, Landing) -> Result<(), AxError>; }
```

**病灶**：`dispatch_in` 驱动之后有六段 `take_effects()`，每段都在做同一件事——把效应变成账本行，再把它变成状态。这条顺序在三份文件里各写过一次：`docs/glossary.md` 对 Ledger 的定义是「Every effect becomes an EventRecord first」，ARCHITECTURE.md §5 步 4 是「that ordering is the design's load-bearing rule, not a logging preference」，signal 那段自己的注释是「Recorded, then delivered. The queue may only change as a consequence of a line the history already has」。**六段里有两段是反的**：

```rust
write_plan(&plan_path, &text)?;                            // 先改共享计划
for effect in &claim_effects { self.record_for(…)?; }      // 后落账

let entry = city::file_archive(…)?;                        // 先上书架
self.record_for(…, EventKind::AssetArchived, …)?;          // 后落账
```

第二段的注释与它自己的代码相反：「Filed after the drive, like every other effect, **so nothing is on the shelf that the history does not already carry**」。按现行顺序，落账失败就在架上留下一条历史没有的记录，那句话就是假的。计划那一段更重：`roadmap_claimed` 是 `memory::hot` 与 `memory::projection` 判断谁拿着哪一行的依据，写进了文件而没落账的 claim 是一行看上去有人占着、历史里却无人占着的行。

**现形**：新模块 `bin::effect`。它不是把那五段搬个地方，而是把「先后」从人的纪律换成类型的性质：`Then` 只能从 `Landing::record` 里拿到，而 `record` 先把所有行送进去才返回它。要把顺序写反，得先拿到一个拿不到的值。

- **批而不是逐条**：一张桌子的行全部落完，才轮到它的变化。这改变了 signal 一支的交错方式（原先是 A 落账、A 投递、B 落账…），**但不改变账本字节**：`deliver` 与 `knock` 都不写账（`knock` 只往 `self.knocks` 推一条，由 drive 之后的 `answer_knocks` 统一开跑），所以 `signal_enqueued` 之间的先后原样。
- **计划那一支是全有全无的**，因此它自己一个穷尽枚举 `Claims`：任一条效应对不上盘上的那份，就一行不写、一行不落，只把动过的行号报给人——这是 R2.06 定下的形制，本卡只把它从 `dispatch_in` 里搬出来并把写盘移到落账之后。
- **`city::archive` 因此拆成两步**（详见 city-SPEC §8-9）：账本行要的 `kind`／`day`／`subject` 全是入参的函数，不需要先写盘就能算出来。不把 `day_of` 搬到装配层算一遍，是因为那会是「一条归档记录长什么样」的第二个权威。
- **`raised`（待批项）不进本模块**：它不是桌子交出来的效应，而是驱动期间被暂存的项，并且在落账前还要受 `tainted_arrival` 改写。它本来就是先落账后改状态的。

**pr 那两支不是同一类，本卡不动，理由记在这里以免下一个人重新查一遍**：

- `PrEffect::Opened` 里的 `wave_pre` 先于 `pr_opened` 落账，**但它不是「先动世界」**。它铸出的是那条账本行所指向的对象，与 `run_started` 之前那句 `self.cas.put(brief…)` 同形：没有任何记录指向的 git commit 不改变任何人读到的东西。交接件把它列为缺陷，我核完否定了。
- `PrEffect::Merged` 里的 `trees.merge` 确实先于 `pr_merged` 落账，而且它真的改变大家读到的干线。**先落账在这里更坏**：`merge` 有一条可达的失败臂 `MergeStale`（分支后干线又动了），先落账就是把一句谎写进历史里的可达路径，而不只是崩溃时的撕裂。要两边都对，`memory::Worktrees` 得先能回答「这一合并会落在哪个 commit」（它就是分支尖，`merge` 今天返回的也正是 `theirs.id()`）且能先验干线。那是另一张卡，它自己的红在 `MergeStale` 那一臂上。

**红**：`what_a_run_changes_is_changed_after_the_line_that_announces_it`。一跑归档一条决定、又从共享计划里拿一行；`RunWorker::observe` 的 sink **在一行耐久之后才跑**（`memory::jsonl` 自说：「runs on the appending thread after durability」），所以它正是「先」唯一看得见的位置。断言：`asset_archived` 落时书架上还没有它，`roadmap_claimed` 落时盘上的那一行还没被拿走；跑完两者都在位（只是排了序，不是丢了）。本卡之前两条断言各自撞红。

**影面**：`city` 公开面换一项、增一项（`file_archive` 改签名，新增 `archive_entry`），基线与 city-SPEC 同提交更新；`sprawling` 公开面不变（`effect` 是 `mod`，不是 `pub mod`）。

**尺寸**：`dispatch_in` 1069 → 983 行。搬走的结结实实是五段共 ≈150 行，其中 60 行以 `RunWorker::settle` 的形式回到本文件——那是五张桌子共用的那一扇门，不是 `dispatch_in` 的一段。**尺寸门要等这个数字降到门限以下才能开，本卡只是第一刀**；剩下最大的两块是驱动块（≈150）与目录及工具准入（≈120）。

## 8-25 一个答复接上的活，不靠重读全部历史找到，也不丢掉它的天花板（整修卡 R2.11）

```rust
// Governance —— 现在是 RunWorker 的一个字段，而不是四个散字段加一份重写
struct Governance {
    pending: BTreeMap<String, ApprovalItem>, autonomy: Autonomy,
    granted: Vec<ClusterKey>, halted: BTreeSet<String>,
    sent: BTreeMap<RunId, Sent>,          // 从 run_started 折；task、goal、budget
    origins: BTreeMap<String, BlockedJob>, // 从 approval_requested 折；答复时 O(log n)
}
impl Governance {
    fn sent(&mut self, RunId, task: &str, goal: &str, BudgetCap);   // 两个调用方，一个形状
    fn absorb(&mut self, EventKind, RunId, Option<&Address>, &Payload);
}
struct BlockedJob { addr: Address, task: String, goal: String, budget: BudgetCap }
```

**三个病灶，一个改动**（交接件第 4 项与§8-23 留下的那一半）：

1. **`blocked_job` 扫全史**。每次审批应答都 `verify_ledger_dir` 一遍再解析两遍，只为找 `(addr, task, goal)`，随历史线性增长。（量级取自 R2.02 在本机留下的同类读数：`verify_ledger_dir` 约 215k 记录/秒，于是 50k 的历史光验链就是百毫秒量级；本卡没有重测。）
2. **天花板归零**。`fn dispatch` 写死 `BudgetCap::default()`，而它正是审批应答后续活走的那条路。一跑带着天花板派出、因待批停下、被批准后续上的那一跑，向模型报 `0 usd_micros, 0 tokens`。
3. **`self.pending.remove(item)` 先于落账**，与 `set_admission` 相反，且是冗余的——`record → govern(ApprovalResolved)` 本就移除它。`self.granted.push(…)` 同理。

**为什么三件一起改**：它们是同一个结构问题的三个面。治理状态本来有两份实现：`Governance::absorb`（重启折）与 `RunWorker::govern` 加上 `set_admission`／`answer_approval` 里直改字段的几行（活折）。R2.02 测过两者不漂移，但那只是当时恰好相等；**再加一份 origins 折就是第三份**。本卡把四个散字段换成 `RunWorker.governance`，`govern` 就是 `absorb`，于是新的两张表只有一个折法。

**选甲而不选乙，理由比交接件写的强**。乙案是让 `approval_requested` 的 payload 自述所阻之活；但那份 payload 就是 `ApprovalItem` 本体，改它得改 `kernel::ApprovalItem` 的公开面与每一个构造点。更重要的是：**账本已经说得出一项是哪一跑提的**（envelope 的 `run`），它没说的是那一跑被派去做什么、在什么天花板下。那是 `run_started` 的事，不是每一项待批的事。

**交接件那个未决问题，用测试回答了**。它问：甲案是进程内存，重启后的 worker 从账本重建，那「重启前提出、重启后才批」的项接不接得上活？答：接得上，因为 `origins` 就在 `Governance` 里，而 `Standing::fold` 对每一行调的正是 `absorb`——与 `pending` 同一折、同一遍。`what_a_worker_holds_is_what_a_restart_rebuilds` 本卡增一条断言盯住它。

**不裁剪 `sent`，写明代价**。每跑一条（两个短字串加 16 字节），与 `memory::HotView` 同一增长级。**不能按 `RunFrozen` 裁**：`freeze` 在 drive 内落账，而装配层的待批项清扫在 drive 之后，账本顺序是 `RunStarted … RunFrozen … ApprovalRequested`，按 freeze 裁会先删掉待用条目。`origins` 则在 `ApprovalResolved` 上裁，因为答过的项不再阻着任何东西。

**读在落账之前，派活在落账之后**：`answer_approval` 先取一份 `origins`（读，不是变化），再落 `approval_resolved`（它自身就是关闭动作，`absorb` 随之丢掉 pending 与 origin），最后才派活。与 §8-24 同一条规矩。

**红（两条）**：

- `work_resumed_by_an_answer_is_done_under_the_ceiling_that_sent_it`：以 `BudgetCap { usd: 250_000, tokens: 4_000 }` 派一跑，它读一次 `status`（对照组），然后提一项待批；批准后续上的那一跑再读一次。**两跑同地址**，所以判据不是 `addr:` 而是该地址上读到的天花板去重后的**集合**：本卡之前是两个值（`0 …` 与 `250000 …`），之后是一个。数请求体不能作判据（§8-23 已记）。
- `what_a_worker_holds_is_what_a_restart_rebuilds` 增一条：活 worker 的 `origins` 与 `Standing::fold` 重建的逐项相等。本卡之前 `origins` 不存在，是编译红。

**性能以结构收口而不以计时收口**：一次审批应答从「验链一遍加解析两遍全史」变为一次 `BTreeMap` 查找；`blocked_job` 连同它的两个循环一并删除，因此这不是一个快了多少的问题——那条路径不存在了。

**影面**：`runtime` 公开面增一字段（`RunPlan.budget`），基线与 runtime-SPEC 同提交；`RunPlan` 的三个构造点（assembly、citysim、runtime 集成测）各加一行；`fixtures/golden-p0` 重生。

## 8-26 读不到一份文件不等于那份文件写错了（整修卡 R2.12）

```rust
fn city_segment(city_root: &Path) -> Result<Vec<u8>, AxError>;  // NotFound → 内置副本；其余 → Err
```

R2.05（交接件）与 R2.06（计划）定下的形制是：**「还没有」答默认值，「读不了」带着路径上报**。本卡收尾同族剩下的两处。

**一、楼页把「读不开」报成「表写错了」**。交接件把它记为「读不到只是页面少一块，不会变成误报」——**我核完否定了这个判断**。`read_building` 把读失败抹成空串，而 `check_roadmap_shape("")` 并不返回空结果：`header_seen` 为假使它推出 `Malformed { problems: ["no four-column table found"] }`。于是页面向人断言一件它无从得知的事：那张表的形状不对。人于是去修表格，而要修的是一个打不开的文件。

**现形**：改走 `city::roadmap`（R2.06 立的那扇门），读失败时**把失败本身放进 `problems`**——那正是这个字段的用途，也是页面已经会画的东西。`read_building` 不改返回类型：`None` 的意思是「没这栋楼」，把「计划读不了」塑成那个形状会让一栋存在的楼从城里消失。

**二、关城时把零字节当成城的规范**。`close_city` 的 `std::fs::read(&city_file).unwrap_or_default()` 使 must-read 指向空字节的 CAS 哈希：下一任被告知「先读这份」，读到的是什么都没有。

**现形**：不新建读法，改用同文件已有的 `city_segment`——「这座城的规范是什么」应当只有一个答案，而 prefix 装配已经在问同一个问题。同时把 `city_segment` 自己改成同一形制：它原本的 `unwrap_or_else(|_| CITY_MD)` 注释自述为「falling back to the built-in copy **when a city predates it**」，而那只描述了 `NotFound`；其余失败下它静默地拿内置副本冗作人编过的那份，而两份可以完全不同。修后：`NotFound` 仍答内置副本（那是已记录的契约），其余一律带路径上报，于是一跑在读不了的城规范下开跑这件事也一并没了。

**关城于是会失败，这是有意的**。一次说不出下一任该读什么的关闭不是一次有序关闭；`serve` 的循环已经写着 `eprintln!("the city could not write its handoff: {err}")`，于是人在终端上拿到路径与修法，而不是一条指向空白的交接件。

**红（两条，各咬一处）**：把 `Roadmap.md`／`City.md` 各做成**同名目录**（R2.05／R2.06 用过的手法，不碰权限，在 Windows 上稳定）。一：楼页的 `problems` 必须点名 `Roadmap.md`——本卡之前它说的是 `no four-column table found`。二：`close_city` 必须以点名 `City.md` 的错误拒绝——本卡之前它返回 `Ok` 并写下一条指向空字节的 must-read。

## 8-27 一次登记喂到两处，于是只写一遍（整修卡 R2.13）

**病灶不是缺陷，是两个权威**。`dispatch_in` 里目录准入与工作台注册是两份各十三行的名单，而同一段的注释自述「one registration feeds both」。两份名单今天相等，但相等是人维护出来的：只上工作台的工具是没人能叫的工具，只上目录的工具是告诉了模型、叫下去却不存在的工具。

**现形**：一个 `Vec<Box<dyn kernel::Tool>>`，一个循环里先 `admit_tool(tool.meta())` 再 `bench.register(tool)`。順序取**目录的**那一份：`Catalog::render` 按准入顺序把工具摆在模型面前，而 resident 段是要算哈希的，所以这个顺序是缓存面的一部分。十三件的次序逐字照旧代码排（archive、exec、claim、edit、status、signal、goal、pr、delegate、workshop、rules、neighbours、read，然后 MCP），故字节不变。

**它以什么收口**（照 §8-17 的写法）：**没有可咬的红**，因为两份名单今天并未漂移——我逐项对过，十三对十三。一条「两集合相等」的断言今天就绿，而且改完之后它恒绿（不可能不相等），那不是测试而是装饰。收口在于：变化后两份名单不可能不相等，且 141 条现有测试（包括多条断言工具名与 prefix 内容的）全绿。**不为了凑一个红而补一条前后都绿的测试。**

**尺寸不是本卡的理由**：`dispatch_in` 983 → 977。五十行准入换成四十五行名单加循环，净值接近零；换来的是一个权威而不是两个。

**本卡推翻的一个假设（写在这里以免重走）**：我判断 `invoke` 里 `match bench.invoke(…)` 的 `_ =>` 臂是死代码——`BenchOutcome` 四个变体已全部列出，且 ARCHITECTURE.md §7 的纪律是「新增一种答案而不回答它就不编译」。删掉它即得 `E0004`：`BenchOutcome` 带 `#[non_exhaustive]`，而本 crate 在它定义的 crate 之外，因此永远无法穷尽匹配。那一臂因此保留，并注明它为何不可达。

——**留给下一个人的问题**：`runtime` 不发布（ARCHITECTURE.md §3：「nothing here is published」），而 `#[non_exhaustive]` 是为 crate 外的第三方准备的。在一个工作区内部的判定输出上用它，换来的是每一个下游 match 都得写一个永不执行的分支，而代价正是 §7 想要的那个编译期穷尽性。runtime-SPEC 第 123 行已写下一条相关规则（「14.3 的 non_exhaustive 规则辖 wire 冻结枚举，不辖判定输出」），而 `BenchOutcome` 正是一个判定输出。**这一条看上去是规则与实现不符，但改它动的是 `runtime` 公开面且没有红，故本卡不动，只点名。**

## 8-28 一次调用的键，只有一份读法（整修卡 R2.15）

```rust
// dispatch_in 驱动块内：六行手写的动作字节换成一个问句
let key = kernel::IdemKey::derive(&run_id, kernel::Seq::new(at), &call.action()?);
```

**本卡不修 `bin::assembly` 的缺陷，因为这里没有缺陷**。被修的是 citysim（citysim-SPEC §8-4）；本文件变的是「谁来回答动作字节」。原先这六行把 name 与 `serde_json::to_string(&call.args)` 拼起来，是全库两份实现中对的那一份；对的那一份待在装配层，正是另一份能静静漂走的原因。`kernel-SPEC §8-6` 早写着这条规则「属 S2 工具面」，而它一处也不在那里。现在它在（`ToolCall::action`，kernel-SPEC §8-23），本文件改为问它。

**字节逐字不变**：`action()` 内部就是搬过去的同一句（name 字节接 args 的 JSON 字节），位次仍是本地 `placed` 计数器。唯一的行为差异是 `unwrap_or_default()` 换成 `?`：一个序列化失败以前产空串（于是两次参数不同的调用得同一把键），现在上报。`Payload` 拒浮点且键恒为字符串，故这一臂今天不可达。

**本卡推翻的一个假设（写在这里以免重走）**：我先判「让 `ToolBench` 自己持 run 与位次、`invoke` 内部铸键」是更好的形——传钟进来这件事就没有参数可传。核完否定：`ToolBench::seen` 恒从空集起，且键在过门之后才记入（runtime-SPEC §532），所以一个恒递增的内部位次会让键在一次驱动内永不重复，`BenchOutcome::Duplicate` 随之变成**任何门都达不到的变体**，`turn.rs` 那条 `dedup_runs_before_the_side_effect`（同键调两次、断言文件未再变）连同它守的不变量一起写不出来。**把一个可测的防御换成不可测的死代码，不是加固。** 位次因此留在调用方。

**顺手记下、本卡不动的一件事**：dedup 是一道**今天接不到任何东西的防御**。`kernel::idem` 自述它存在是为了「resume 与 replay 重派出同一把键」的双付防御，而 `seen` 从不从历史播种，`sprawling resume` 也不重跑一跑（ARCH §5 末：它只验链、把丢了结果的调用关成 unknown、并报告等人的事）。本卡之后，`Duplicate` 在两个驱动器里都不会再出现，而这是**对的**：它本就是重放路径上的结果。要让它真正接上，得让 `seen` 从账本重建——那是另一张卡，它自己的红在「重建后的 worker 不会把已经付过的钱再付一遍」上。

## 8-29 行没落下，城就没动（整修卡 R2.17）

```rust
impl RunWorker {
    pub fn new(city_root, vault, log) -> Result<Self, AxError>;              // = open ➕ over
    pub(crate) fn over(city_root, vault, log, ledger: JsonlLedger) -> Result<Self, AxError>;
}
```

这是 §8-24（R2.10）那条性质的另一半。R2.10 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）；这里问的是「**行没落下，城就没动**」。

**选甲而不选乙，而且交接件对甲的反对意见不成立**。交接件写着甲案（`RunWorker::over`）「只有一个生产调用方，近乎为测试拓宽」，而乙案（倒置 `kernel::Ledger`）才是 ARCH 点名的那类动作。核完两头都不对：

- **甲不是测试拓宽，是 ARCH §3 自己提的那条批评**。§3 末段写着 `RunWorker`「builds its model adapter **instead of receiving one**」，并把它列为 V6 停在装配层下方的原因。同一句逐字适用于账本：一个自己 `open` 账本的 worker 同样无法被驱动到第二份实现上。把「账本从哪来」从构造子里取出去，是把一个不属于它的决定交回给调用方。
- **乙今天买不起**。`RunWorker` 对账本用的不只 `append`，还有 `position()`（两处）与 `observe()`。把 `observe` 推上 `kernel::Ledger` 等于让最内层去定义什么是「耐久后通知」——那是持久化适配器的事，不是「一个 Ledger 是什么」的事；代价是全库 **七个 `impl Ledger`** 各长出一个它们不需要的方法，加 conformance 套件。而本卡根本不需要第二个类型：两条路上都是具体的 `JsonlLedger`，**不同的是它下面的 `Vfs`**。既然缝不必动，就不动。
- 丙（只在 `memory` 内写红）**已经存在**：`power_cut_matrix_over_every_op_keeps_acknowledged_waves`。它证的是账本自己的耐久契约，不是装配层的不变量，所以它不替代本卡。

**它以什么收口：一张没有红的卡，照 §8-17／§8-27 的写法说清楚**。`a_line_the_history_refused_is_a_change_the_city_never_made` **首跑即绿**，因为这条性质 R2.10 已经用类型持住了：`record` 遇拒即 `?` 返回，`Then` 随之丢弃，改变无从发生。**我没有补一条前后都绿的测试就算完事**：把 `Landing::record` 的 `append(line)?` 改成 `let _ = append(line);` 后重跑，它当场红，且红在实质那条断言上——盘上的计划被写成了 `| 1 | wire the kiln | In progress |  |`，而宣布它的那一行从未落地。恢复后又绿。这条测试因此是一张网，不是一条红，而它能咬是量出来的不是声明出来的。

**测试里两个世界各归各位**：账本在 `FaultFs` 的内存平面上，城的文件（`Roadmap.md`）在真盘上。这正是要问的形状：被断言的东西是一份人事后真能去打开的文件。`Standing::fold` 仍读真目录（那是「城到目前为止知道什么」），而本跑新落的行进虚拟账本——两者不相干，因为断言不靠账本内容，只靠盘上那份计划。

**影面**：`memory` 公开面在 `fault` 下增 `open_faulty`、`FaultPlan` 增一字段（memory-SPEC §8-2 同提交）；`sprawling` 公开面**不变**（`over` 是 `pub(crate)`）；`crates/sprawling/Cargo.toml` 的 dev-dependencies 打开 `memory/fault`，发行构建不含它。

## 8-30 合并也排到它那条行后面（整修卡 R2.18）

§8-24 把五张桌子搬进 `bin::effect` 时，把 `PrEffect::Merged` 留在原地，理由写得很清楚：`trees.merge` 确实先动世界，但「先落账在这里更坏」——`merge` 有一条可达的失败臂 `MergeStale`，先落账就是把一句谎写进历史里的可达路径。它同时写下了解法：`memory::Worktrees` 得先能答「这一合并会落在哪个 commit」且能先验干线。本卡做的就是那一条（memory-SPEC §8-2），于是两头不再互斥：

```rust
let planned = trees.plan_merge(&name)?;    // 全部拒绝在此，世界未动
record_for(…, EventKind::PrMerged, … planned.commit() …)?;   // 行
planned.apply()?;                           // 才是变化
```

于是：一个会被拒的合并永远不会先得到一条行（`MergeStale` 早于落账）；一条没落下的行也永远不会已经改了干线（`apply` 需要一个只能从 `plan_merge` 拿到的值，而行写在它之前）。

**红**：`a_merge_the_history_refused_leaves_the_building_where_it_was`。一个 `review: true` 的楼，一跑改文并提交请求，第二跑去检——而第二跑的账本是 `open_faulty`（§8-29 的工具）且 `cut_on_write: Some("pr_merged")`。断言：楼里那份文件仍是 `before`。**本卡之前它是 `after`**：干线已经移了，而宣布它的那一行从未落地——一座楼站在它自己的历史说从来没有并入过的工作上。我把次序改回去跑了一遍看它撞红，再改回来。

**影面**：`memory` 公开面去 `Worktrees::merge`、增 `plan_merge` 与 `PlannedMerge`（基线与 memory-SPEC 同提交）；四个读写方全部迁完后旧入口删除，不留适配。`sprawling` 公开面不变。

## 8-31 dispatch_in 向 ARCH §5 的十二步靠拢（整修卡 R2.19，逐刀）

**目标不是「把某一段搬走」，是「让 `dispatch_in` 成为 ARCHITECTURE.md §5 已经写好的那个序列」**。两者的区别是形状问题的生死：按行号切出来的一块叫不出 §9 的名字，而一个相位叫得出来——§5 已经给了它名字。

**第一刀：驱动（§5 步 7–11）。**

```rust
struct Driven {
    outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,   // 仍是 Result：跑败也要结桌子
    fenced: Vec<String>, ran: (u32, u32), raised: Vec<ApprovalItem>,
}
impl RunWorker {
    fn drive_dispatch(&mut self, plan, handoff, adapter, bench, signals,
                      write_root, fence_scope, who, run_id) -> Result<Driven, AxError>;
}
```

三个钩子住在一起，理由不是它们相邻，而是**它们是唯一在驱动器持有账本期间碰账本的代码**（`invoke` 里那句自述：「the ledger is the driver's for the length of the run」）。它们收集的三样东西也只在那段时间里可写，所以一并作为 `Driven` 返回，而不是留四个 `Rc<RefCell<…>>` 让调用方自己保持同步——四个单元格是四个可以忘记读的东西，一个值不是。

`outcome` 刻意仍是 `Result` 而不在方法里 `?`：一跑失败了它的桌子照样要结，而结桌子正是把它最后几行放上历史的动作。把它提到方法边界上会静静跳过它们。

**尺寸**：`dispatch_in` 975 → **833**；`drive_dispatch` 171。尺寸不是本刀的理由（照 §8-27 的写法），但它是 R2.20 尺寸门的前提，而那道门的门限是量出来的 200。

**它以什么收口**：纯结构，无可咬的红——行为逐字不变（钩子体原样搬迁，`fence_scope` 由计算改为传入）。143 条 `sprawling` 测试全绿，其中包括直接盯驱动行为的 §8-24／§8-29／§8-30 三条。

### 剩下的五刀（本会话未完，按 AGENTS.md 「把剩下的写进它所属的 SPEC 节」）

`dispatch_in` 今为 **833** 行（@3611），相位实测如下。目标 <200；每刀都是同一个形制：相位成为 `RunWorker` 的一个方法，多个活值归并为一个归位值类型（如 `Driven`），而不是一排得保持同步的局部变量。

已切九刀（R2.19a–h），**975 → 158**，产出的方法均在阀值内：`drive_dispatch` 171、`settle_desks` 124、`settle_requests` 122、`conclude` 104、`stand_up` 92、`admit_reading_room` 32。三个归位值类型：`Driven`（驱动期间写、驱动之后读的四样东西）、`Desks`（一起出借、一起收回的五张桌子）与 `Site`（一次跑站在哪儿）。

**第七刀：桌子（§5 步 3–4，整修卡 R2.19f）。**

```rust
struct Desks { signals, goals, plan, shelf, pr, plan_path: PathBuf, waiting: u32 }
impl RunWorker {
    fn open_desks(&mut self, site: &Site, addr: &Address) -> Result<Desks, AxError>;
}
```

**`pr` 与 `waiting` 入伙，`Desks` 的理由随之改写**。R2.19c 建 `Desks` 时写的理由是「一起结算」，而 `pr` 不与它们一起结（它等 `produced`，在 `settle_requests` 里）。但五张桌子**一起出借、一起收回**，而这正是它自己标题已经写着的那一句。理由换成出借，`pr` 于是入伙；否则它就是唯一一个被抛在值外面、靠人记得的桌子。`waiting`（`lent.pending()`，`u32`）同理：它只能在队列交给桌子**之前**数，数不到就永远数不到了。

**一个名字在相位内改了**：原来的局部 `shelf`（`Vec<Held>`）与 `memory_desk` 在归位值里叫 `shelf`，于是前者改叫 `held`——一个名字对一个东西，而“书架”指的是那张桌子。

**尺寸**：`dispatch_in` 495 → **423**；`open_desks` 76。十行的 `Desks` 手工构造（原在驱动之前）随之消失：归位值由相位自己交出来，不再由调用方拼。

**它以什么收口**：纯结构，无可咬的红。五张桌子的构造顺序、`now_ms()` 的采样位置、`inboxes.remove` 与 `pending()` 的先后均逐字不变。143 条 `sprawling` 测试全绿，其中 `a_signal_one_run_sends_is_read_by_the_run_that_pulls_it` 与 `a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke` 走的就是“队列借出去、再收回”这一支。

**第八刀：工作台（§5 步 6，整修卡 R2.19g）。**

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

**工具块需要的不是两个方法而是三个，这是量出来的**。R2.19a 写「工具块需要两个」（阅览室一个、剩下一个）；切完一量，`lay_out_workbench` **206 行**，越过 R2.20 将要执行的 200。纪律是「不为通过而放宽门」，于是再切一刀。切在 `status`：它是十三件里**唯一一件要读 worker 治理状态（`governance.autonomy`）与目标登记册（`self.goals`）的工具**，也是唯一一件带活闭包的（`children` 读委派桌，因为一跑边跑边派活）。它回答的那个问题与周围不同：**这一跑对自己怎么交代**。

**两个局部变量随它走了**：`writable` 与 `neighbours` 原本只为 `status` 而算（`write_domain()` 本就在同一方法里被叫三次，多一次不改变任何东西），现在各自在 `status_tool` 内部算。

**尺寸**：`dispatch_in` 423 → **237**；`lay_out_workbench` 164；`status_tool` 51。

**它以什么收口**：纯结构，无可咬的红。十三件工具的**构造顺序与登记顺序逐字不变**——而登记顺序是缓存面的一部分（§8-27），prefix 字节一变即有测试当场发作，这正是 143 条全绿在本卡的分量。

**第九刀：冻结（§5 步 5，整修卡 R2.19h）。**

```rust
fn freeze_plan(&mut self, site, workbench, addr, brief, task, goal, job, parent, budget)
    -> Result<(RunPlan, runtime::handoff::Handoff), AxError>;
```

**两个值而不是一个新类型**：`RunPlan` 与 `Handoff` 类型不同、谁也不会认错，再包一层只是给元组取个名字。它们同属一相位的理由是读一遍就看得见的：prefix 为这份 plan 而装配并与它一同冻结，handoff 引的是 plan 自己的 `task_line`，而 job locator 两边都在。

**尺寸**：`dispatch_in` 237 → **158**；`freeze_plan` 100。至此 `dispatch_in` 不再是本文件最长的函数（`drive_dispatch` 171 是），九刀合计 **975 → 158**。

**它以什么收口**：纯结构，无可咬的红。prefix 四段的装配顺序、must-read 的入列顺序（先 norms 后 job）均逐字不变；两者一变即有多条盯 prefix 字节与交接件内容的测试发作。143 条全绿。

**剩下四刀**（目标 <200，预计落在 ~160）。上一版此处写「三刀」而表里四行，是笔误：四个相位都还在 `dispatch_in` 里，四刀都要切。

| 刀 | 相位（§5 步） | 长度 | 归位值 |
|---|---|---|---|
| e | 规则／配置／选型／身份／租约（步 3） | ≈70 | `Site` |
| f | 五张 desk 的构造（步 3–4） | ≈75 | `Desks`（扩 `pr` 与 `waiting`） |
| g | catalog＋十三件工具＋bench（步 6） | ≈175 | `Workbench`（catalog、bench、delegates） |
| h | prefix＋RunPlan＋handoff（步 5） | ≈90 | `(RunPlan, Handoff)` |

**依赖序即执行序，而且依赖是真的**：工具块读十五个局部（`write_root`／`rules`／`config`／`building`／`who`／`depth`／`model` …），先有 `Site` 才能让 g 收得下参数，而不是把十五个形参排成一列。

**第六刀：站位（§5 步 3，整修卡 R2.19e）。**

```rust
struct Site {
    building: city::Building, rules: city::BuildingRules, config: kernel::FrozenConfig,
    model: gateway::ModelEntry, adapter: Box<dyn Model + Send>,
    identity: city::Identity, who: String, run_id: RunId,
    lease: Option<memory::WorktreeLease>, write_root: PathBuf, branch: Option<String>,
}
impl RunWorker {
    fn stand_up(&mut self, addr: &Address, job: &Locator,
                task: &str, goal: &str, budget: kernel::BudgetCap) -> Result<Site, AxError>;
}
```

**一个值而不是三个，理由是时钟而不是口味**。这一相位读上去是三件事（规则与选型、身份与登记、围栏与租约），而它们在代码里互相咀合：租约要 `run_id` 与 `who`，而 `run_id` 在 `renew_if_stale`（一次可能走网的凭证续期）**之后**采钟。拆成三个方法就得把身份块提到选型之前，那会把 `run_id` 的时间戳提前一次网络往返——而本卡是纯结构卡，行为需逐字不变。**一个采钟点的先后不是重构可以顺手改的东西**（ARCH §10：全库只有一个采样点，它采到的值进了账本）。于是相位按原序整体搬迁，归位值一个。

**`Site` 不收 `addr`**：`Address` 是 `dispatch_in` 的形参，它在相位之前就在，放进去就是同一个值的第二份。上一版草案把 `addr` 列在字段里，按这条删。

**归位值先在调用点拆开，下一卡改回整值——记在这里以免重走**。本卡初版写的是 `let Site { building, rules, … } = self.stand_up(…)?;`，理由是 diff 最小（搬走七十行、接替十七行、其余四百行逐字未动）。R2.19f 将它改成 `let mut site = …`，理由是量出来的：拆开之后，剩下三个相位要从调用点接过去的名字共 **102 处引用、十一个名字**（`building` 23、`model` 18、`who` 17、`write_root` 11 …），即每个相位方法都得排一列十五个形参——而那正是归位值要消掉的东西。`Driven` 可以拆，因为它四个字段只在驱动之后被读一次；`Site` 不行，因为它要穿过剩下每一个相位。

**尺寸**：`dispatch_in` 548 → **495**；`stand_up` 92。

**它以什么收口**（照 §8-17／§8-27 的写法）：纯结构，无可咬的红。搬迁逐字，唯一的改动是 `&addr`／`&job` 从局部变成形参，且两者在相位内部的用法不变；采钟点的个数与先后不变（`run_id_for` 一次、`ensure_base` 一次）。143 条 `sprawling` 测试全绿，其中 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯租约这一支。不补前后都绿的测试冒充红转绿。

工具那一块原为 214 行，本卡先把阅览室（`admit_reading_room`）切出去，余下 ≈175 才能装进一个合格方法。**这正是阀值取 200 的一个副作用**：它不允许把一堆东西搬到另一处冒充分解。

**R2.20 尺寸门须等这五刀完成**：纪律是「不为通过而放宽门」，所以门不能先落地再给自己开例外。门限与单位由全库测量定下，数字就写在这里：单位是**生产函数**（以首个 `#[cfg(test)]` 截断），门限 **200 行**。依据：1646 个生产函数中位数 9、p90 为 37、p99 为 114；超过 200 的只有六个，而其中五个是数据与标记（`web::lang::phrase` 是译文表，属 §9 形状 6；`Settings`／`CityView`／`BuildingView`／`Root` 是 Dioxus 组件，函数体即标记），故这两类需在门里声明为数据。排掉它们，全库超阀的生产函数只剩 `dispatch_in` 一个，第二名 `serve` 为 233——它也在网内，这是故意的，把门开到 240 去放它过就是为通过而放宽门。而按**文件**计不行：任何诚实阀值都会在四个 crate 里同时点燃八处（800 行阀 → 8 个文件），那是工程而不是一道门。提交须带 `Verdict: user-approved`，依据与 `a9b1522` 同：常驻指令即那条裁定。

## 8-32 一座城的那一个写者，自己有个名字（整修卡 R2.20a）

```rust
fn spawn_worker(city_root, vault, vault_notice, log, views, to_clients, worker_desk)
    -> Result<std::thread::JoinHandle<()>, AxError>;
```

**为什么是它**：`serve` 233 行，是 R2.20 那道门报出的两个对象之一。三个相位里（存储与视图、写者线程、socket 配置与关城），线程那一段是唯一一段带着**自己的契约**的：账本在线程**里面**打开且永不离开（一座城只有一个写者，而这件事不靠约定靠类型），并且它带着一次**握手**：`ready_rx.recv()` 回来之前，没人能把 socket 架在一座没打开的城上。把握手包进方法里，返回的 `JoinHandle` 于是自带一句断言：拿到它，就意味着那个写者已经在跑。

**尺寸**：`serve` 233 → **162**；`spawn_worker` 91。

**它以什么收口**：纯结构，无可咬的红。线程体逐字搬迁；原先在 `serve` 里各自 `Arc::clone` 的 `worker_desk` 与 `to_clients` 改为在调用点克隆后传入，克隆的**个数与时机不变**。143 条 `sprawling` 测试全绿。

## 8-33 同一把键的第二次，不是第二件活（整修卡 R2.21）

```rust
pub(crate) struct CommandDesk { waiting: Mutex<Waiting>, arrived: Condvar, closing: AtomicBool }
struct Waiting { queue: VecDeque<Posted>, keys: BTreeSet<IdemKey> }   // 队列与在途键同一把锁

impl CommandDesk {
    pub(crate) fn post(&self, command: channels::Command, reply: channels::Reply);
    fn wait(&self, patience: Duration) -> DeskWait<'_>;               // Command(Posted, Underway<'_>)
}

/// 正在被办的那一条命令的键，办完即释放（Drop）。
struct Underway<'desk> { desk: &'desk CommandDesk, key: Option<IdemKey> }
```

**交接件说「让 `ToolBench::seen` 从账本重建」，量完是错的，改正记在这里**。键由 `(run_id, 本次驱动内的位次, action)` 铸成（§8-28），而 `run_id_for(job, addr, now_ms())` 把采钟拌进了身份，**没有任何生产路径会用同一个 `run_id` 再驱动一次**：审批放行后接着干的那段活，是 `answer_approval` 重新 `dispatch_in` 出来的一次**新 run**（§8-25），`resume` 只验链并把丢了结果的调用关成 unknown（ARCH §5 末）。往 `ToolBench::seen` 里播种历史，播进去的键在那一层永远比不中——那是把可测的防御换成不可测的死代码，正是 §8-28 拒绝过的那件事。

**没人守的那道门在上一层，而它今天就在漏钱**。`channels::wire` 的模块文档写着「每一条改状态的 Command 都带 `IdemKey`……『双击两次开出两个 Run』在这个类型里拼不出来」，而 `run_command` 的每一条臂都用 `..` 把 `idem` 丢掉：全库没有一处读 `Command::idem()`（只有 `channels/tests/wire_contract.rs` 与 `web::reach` 的两条测试读它）。四个发送端却都是照「服务端会去重」写的——`web::app::dispatch_command` 铸 `addr|task`、`web::city_view::create_command` 铸 `addr`、`console::dispatch` 铸 `console:addr:task`、`acp_dispatch` 铸 `acp:addr:task`，同一次提交两次就是同一把键；`web::reach` 甚至有一条测试叫 `saving_twice_configures_once`，它断言的却只是两条命令的键相等，**「只配置一次」这半句今天由谁兑现，答案是没有人**。于是双击一次、编辑器超时重发一次、控制台重敲一行，都是两次全款的模型账单。

**规则住在桌子上，而不是住在 `handle` 里**。`CommandDesk` 是每一条命令在 socket 与写者之间必经的那一处，它本来就按键之外的理由扫过自己的队列（`interrupt_for` 找 Cancel／Steer）。判定复用 `kernel::dedup`——`kernel::gate` 自述「seen 集合是调用方的状态，kernel 只判成员关系」，本卡就是那个调用方的第二个实例（第一个是 `ToolBench`）。两个集合不是两处权威：一个管**工具调用**，一个管**人递进来的命令**，主体不同。

**在途，而不是永远**。一把键从 `post` 起在途，到那条命令**办完**为止：`wait` 交出 `Posted` 时一并交出 `Underway`，写者循环让它活到那一条命令服务完毕，Drop 释放键。于是——

- 双击、传输重发、编辑器超时重试：第二帧在第一件活还没办完时到达，被丢掉，**一次派活一次账单**；
- 人看完结果、想再跑一遍同一件活：键早已不在途，第二次照常受理，**不会静默吞掉**；
- 集合大小由队列深度加一封顶，不随城的寿命增长，也不需要时钟或任何窗口常数。

**`interrupt_for` 消耗掉的那条命令也要释放键**，否则同一个 run 的第二次 Cancel（`web::live` 铸的是 `cancel-from-the-control-surface`，每个 run 一把定键）会被永远丢掉。删队列与删键在同一把锁里完成。

**被丢掉的那一帧不回话**：发送者要的那件事正在办，`Reply` 只承载拒绝，而这里没有拒绝可言。**留下的残余**：控制台里同一行敲两遍，第二遍在第一遍办完前无声消失。真要给它一句话，得在 `AxCode` 上开一个「已在办」的码并让 `post` 交回受理与否——那是它自己的一张卡，本卡不动，条件是这件事真的绊到人。

**它以什么收口**：一条会咬的红。`a_repeat_of_a_command_already_underway_is_not_a_second_piece_of_work` 首跑即红——今天两帧都进队列，第二次 `wait` 交出第二条命令而不是 `Idle`；实现后转绿，并在同一条测试里证明另一半：办完之后同一把键再来照常受理。既有测试 `a_cancel_reaches_the_run_it_cancels_without_waiting_for_it_to_end` 原先给三条不同命令共用一把 `b"i"` 键（图省事的夹具，真实客户端不会这么铸），本卡改为一条一把——它测的路由与优先级不变。

## 8-34 计划从每问一次重解析，变成一次投影（V3.17 顺手；`bin::plan_view`）

`CityView` 与 `Metrics` 过去每被问一次，就把每栋楼的 `Roadmap.md` 从盘上读出来重新解析一遍。页面是轮询的，而一份计划一小时改不了几次——这是**为一个几乎不变的答案，按提问频率付钱**。

```rust
pub(crate) struct PlanView { /* read、causes —— 私有 */ }
pub(crate) struct PlanReading {
    pub(crate) progress: Progress,
    pub(crate) problems: Vec<String>,
    pub(crate) rows: Vec<channels::PlanRow>,
    pub(crate) blocked: Vec<channels::BlockedLine>,
    pub(crate) ready: Vec<NodeId>,
}
impl PlanView {
    pub(crate) fn apply(&mut self, record: &EventRecord);
    pub(crate) fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading;
}
```

- **文件仍然是计划**。变的只是谁去读：`kernel::WriteMoment` 说这张表只在三个时刻被写，而每一个时刻都是一条记录，于是折叠记录、只在有记录点到那栋楼时才回去读文件。
- **失效由两类记录触发，理由不同**。`roadmap_*` 说一个 run 动了计划——既是忘掉已解析副本的理由，也是一件本身值得留着的事实（红的原因）。`checkpoint_committed` 只说一波工具写过文件——**用 edit 工具改了表的 agent 不留 `roadmap_*` 记录**，一个忽略工具波的缓存会继续报改动之前的计划。
- **它是投影不是副本**：这里不存计划说了什么，只存**上一次读到的时候它是什么**，并在任何可能改变它的事情发生时丢掉。删掉整个它、把同一批记录再折一遍，得到同样的字节——因为它做的全部事情就是折叠（V3.20 收口断言）。
- **它折的唯一一件文件装不下的事，是节点为什么红**。表格有位置说 `Blocked`；人需要的那句话在 `roadmap_blocked` 的记录里，在表里再放一份就是同一句话的第二个权威。没有记录撑着的 `Blocked` 行仍然算红，措辞退回状态词本身——一个人手改的行仍然是一行说着活停了的行。
- **`BuildingView` 也走这一份**：楼的对象页从这里拿计划，只有文档、房间与档案仍在被问的那一刻读盘。让对象页自己再解析一次，就是「什么卡住了、为什么」有两个答案，而只有一个在折记录。

## 8-35 谁在追一个目标，谁替它派活（V3.22；`RunWorker.pursuits`）

- **值住在工人身上，事实住在账本里。** `kernel::Pursuit` 由 `Delegator::root()` 铸出，而这座城里**唯一一处 `Delegator::root()` 就在 `RunWorker::over`**——于是「子代理不能让全城通宵干活」是一件关于代码的事实，而不是一条谁去遵守的规则。设置／暂停／恢复／清除各落一条 `pursuit_changed`，`Views` 折它来画，重启后工人从同一批记录把值重新铸出来。
- **`Views` 不持 `Pursuit`，只持文本与状态**：一个能铸出 `Pursuit` 的视图，就是那道守卫上的第二扇门。判定仍由 `kernel::observe_pursuit` 给出，措辞由 `verdict_line` 一处写出——页面、控制台与日志说同一句话。
- **`pursue` 会终止，理由在集合上而不在计数器上**：认领把节点移出就绪集，而一个结束时还持有节点的 run 会把它留成 Blocked（`ClaimDesk::abandon`），所以就绪集严格变小；唯一让它变大的是拆分，而那是这座城找到了更多活，不是在打转。派活之后若该节点仍在就绪集里，循环停下并留一条诊断——**看的是集合本身，不是一个凭空定的上限。**

## 8-36 一个节点红了，站在它后面的人会知道（V3.20；`tell_whoever_is_behind`）

- **由事实触发的交流。** 这座城里居民互相够到彼此的每一种方式，都从「有人决定要说话」开始；这一种从「一个节点红了」开始，并且恰好够到那些手上的活现在动不了的房间。
- **信号 id 由楼与节点推出**（`blocked-<building>-<node>`）：同一处卡住宣布两次是同一条信号，信箱按 id 去重。一个房间为一个问题被通知四次，是一个会停止读信箱的房间。
- **「谁持有哪个节点」只有一份**，折在 `CollaborationFold.plan_holders` 里：认领那条记录的 `addr` 就是房间，所以这里不推导任何别人已经写下的东西。`plan_view` 不再持第二份——它只画，不派信。

## 8.5 两个设计

**A（选中）**：`build.rs` 拷贝资产入 OUT_DIR＋`include_bytes!`——单点嵌入，S4 换 wasm 产物时只改拷贝源。**B（落选）**：`include_bytes!` 直指 `../web/assets`——少一步拷贝，但把「产物在哪」写死进源码路径，S4 换源即改代码；且无 `rerun-if-changed` 粒度。翻案条件：无。

## 9 工作流程

`main` → 解析 argv → `status`：`assemble()` → 打印三行 → 退出码。

## 10 实现逻辑

零依赖（clap 待 S1 真子命令出现时引入——现在只有一个子命令，一个 match 不值一个依赖）；`cargo::error=` 使 build.rs 失败显性（cargo ≥1.84 语法）。

## 11 边界枚举

资产文件缺失（build 期即红，不是运行期惊喜）；OUT_DIR 缺失（同上）；无参调用（用法＋退出 2）。

## 12 错误处理

build.rs 内 `Result<(), String>` 汇到 `cargo::error`；运行期无可失败路径（S0）。逐码消解：无新增码。

**`replay <ledger-dir>`：「这里没有账本」不得与「验过且为空」同形**（issue #3）。本子命令的路径是人敲的，故它先问 `memory::ledger_segments_at`，一段都没有即报 `E_PATH_NOT_FOUND` 并给 recovery，不进验链。**判据为什么在这一层而不在 `runtime::replay`**：`verify_ledger_dir` 的四个生产调用方均自持城根算出路径，而已开未写的城就是一个无段目录（`JsonlLedger::open` 只建目录），在那一层报错会把合法启动打红，并迫使四个调用方各写一份相同的守卫。空账本仍然合法，故问的是「有没有段」而不是「有没有行」。参见 runtime-SPEC §8-1、memory-SPEC §8。

## 13 依赖选型

零运行时依赖（见 10）。

## 14 硬编码声明

资产相对路径 `../web/assets/index.html`（S4 随构建管线改为 wasm 产物目录，改点唯一在 build.rs）。

P0（`bin::install`）引入四处，全部是外部世界的事实而非我们的选择，故各自注明出处：`%LOCALAPPDATA%\Programs\<app>` 是 Windows 用户级程序目录的约定；`~/.local` 下的 `bin` 是 XDG 用户级可执行目录的约定；`HKCU\Environment` 是用户级环境变量在注册表里的位置；`WM_SETTINGCHANGE=0x1A`／`HWND_BROADCAST=0xffff`／`SMTO_ABORTIFHUNG=2` 是 Win32 的常量值。这四处一旦被平台改掉，改点各只有一个。

## 15 影响面

justfile／CI 无涉；S4 前端框架结论书将改写 build.rs 拷贝源与 `just build-web`。

## 16 测试与约束

单测：嵌入字节非空且含 `sprawling` 标记。约束：workspace lints 全量适用（含 build.rs）。

## 17 模型体验

零字节：bin 不产生任何入窗内容。

## 18 文档同步

子命令每扩一个：本 SPEC 增章、ARCHITECTURE.md §12 状态翻转、CLI 三栏表核对。

P7 起交付形态入册：`just package` 的产物名、`QUICKSTART.md`、README 与 `docs/getting-started.md` 的首次运行段、`release.yml` 的附件清单，五处同改。

## 8-37 每个问题的答案搬出装配点（V3.30；`bin::views`）

**动手的理由是尺寸，留下来的理由是形状。** `bin::assembly` 12,078 行，`xtask length` 的文件面（1000 行）从此判它红，
而 `[file_length.predating]` 把它钉在 12,078——只准变小。这一刀先切最干净的那条缝。

**缝在哪，由模块表自己说。** ARCHITECTURE §9 写着「一个说不出自己形状的模块，通常装着两件想分家的东西」。
`bin::assembly` 的行是 `adapter`：装配点、最脏、唯一全知。而 `Views` 折账本、答每一个 `Query`、删掉重折得到同样的字节——
**那是 `projection`，§9 的形状 7。** 一个文件里两个形状，正是那一行说的判据。

**先例已经在这个 crate 里**：V3.17 把 `assembly::read_spine` 搬成 `bin::plan_view`，同样是从装配点里取出一个投影。
本卡沿用它的落法：**兄弟模块，不是 `assembly/` 子目录**。

> **V3.43 改正了下面这条理由，它读错了门。** 原文写：「子目录会让 `assembly.rs` 变成 `modmap` 眼里的索引文件
> （`xtask/src/modmap.rs` 的 `is_index_name`），而索引文件只准放声明——那等于要求 12,078 行**一次全部**拆完。」
> `xtask/src/modmap.rs` 的判定顺序是 `if let Some(row) = table.get(rel) { … } else if is_index_name(…)`：
> **登记在模块表里的文件永远走不到索引检查。** `assembly.rs` 可以保留自己的行、继续持有代码，
> 同时 `assembly/` 挂子模块。兄弟模块对本卡仍是对的选择（`Views` 是投影，不是 `RunWorker` 的一部分），
> 但那个选择的理由是形状，不是这条不存在的限制。

### 搬走什么

`Views` 结构与它的两个 `impl`、`endpoints_answer`、`verdict_line`、`pursuit_from`、`buildings_of`、
`signal_line`、`discard_lines`、`registry_line`、`summarize`，以及**咬它们的那十条断言**。
测试跟着被测的东西走：一份留在原处的断言会让下一个人以为那里还有代码。

### 不搬走什么，以及这件事本身的发现

`NAME_THE_WORK`、`NAME_TOKENS`、`mode_of`、`not_built`、`Reporter`、`building_of`、`plan_node_of`
在原文件里**物理上坐在 `Views` 那一簇的中间**，而它们的使用者是 `RunWorker` 与 `CollaborationFold`：
`not_built` 六处、`Reporter` 三处、`mode_of` 一处，`Views` 一处都不用。
**这就是那个文件长成这样的机制**——没有边界的地方，新东西落在光标所在的行，而不是落在它属于的地方。

### 验收

`Views` 与 `rebuild_views` 在本 crate 外没有任何引用（已查），所以搬动不动任何公开面，`apisync` 基线不变。
`cargo xtask length` 里 `bin::assembly` 的钉子随之降低；降不下来就是没搬干净。

## 附记：`ClientAssets` 与 `Command` 的定义模块变了，接口没变（V3.36／V3.38）

本 crate 的 API 基线里 `Serving::client` 的类型路径从 `channels::server::ClientAssets`
变成 `channels::assets::ClientAssets`。**这不是一次接口变更**：公开路径仍是
`channels::ClientAssets`，字段与签名一字未动，变的只是 `cargo public-api` 记录的定义模块——
客户端资产从 `channels::server` 搬进了自己的文件（channels-SPEC §8-2）。
记在这里是因为 `apisync` 判的是「基线动了就要有一份 SPEC 同行」，而基线确实动了。

V3.38 同理：`RunWorker::handle` 的参数从 `channels::wire::Command` 变成
`channels::command::Command`。公开路径仍是 `channels::Command`，签名一字未动；
`Command` 从 `channels::wire` 搬进了自己的文件（channels-SPEC §8-1）。


## 8-38 一座城怎么被端上来，与一轮活怎么跑完（V3.42；`bin::serving`）

`bin::assembly` 11,461 → 10,695，`bin::serving` 830。**交接文件点名的第一刀**，切的是形状而不是行数：
`Serving`／`spawn_worker`／`serve`／`CommandDesk` 回答「一座城怎么被端上来」，`RunWorker` 回答「一轮活怎么跑完」，这不是同一件事。

**搬走什么**：门口那把钥匙（`Keyed`／`key_for`／`random_token`——熵在本 crate 只有这一处）、
金库的开启（`open_vault`）、socket 与唯一写入者之间的那张桌子（`CommandDesk`／`Waiting`／`Posted`／`DeskWait`／`Underway`）、
`Serving` 与 `serve`，以及**开唯一那条写入线程的 `spawn_worker`**——账本在它里面打开且从不离开，这条性质现在写在它自己的模块文档里。

**一条超长签名被消掉而不是被搬走（V3.35a 的规矩）**：`spawn_worker` 八个参数，现在两个——
`Opening { city_root, vault, notice, log }`（一个工人是用什么打开的）与 `Outward { desk, views, to_clients, to_watchers }`（它的活从哪来、结果到哪去）。
**`#[expect(clippy::too_many_arguments)]` 随之消失**：一条压制在修好之后自己清掉，这正是 rust-hardening 要的形状。

**没搬走什么**：`execution_engine`／`CITY_VERIFIER`／`local_model_facts` 留在 `assembly`，因为它们是 `RunWorker` 在一轮活里用的东西，不是端城用的。

**剩下的债写在这里**：`assembly.rs` 仍有 10,695 行，其中 `RunWorker` 一个类型约 3,700 行、22 个私有字段，
`mod tests` 约 5,600 行。

> **V3.43 改正了下面这条结论。** 原文写：「把 `impl RunWorker` 的方法散到兄弟模块要把 22 个字段改成 `pub(crate)`——
> 那是用一个诚实的大文件换几个不诚实的小文件」，并据此判定这条债不是拆分能还的。
> **前半句只对兄弟模块成立。** 子模块看得见父模块的私有项（Rust Reference, *Visibility and Privacy*:
> "If an item is private, it may be accessed by the current module and its descendants"），
> 所以 `impl RunWorker` 拆进 `assembly/` 的子模块**一个字段的可见性都不必动**。详见 §8-39。


## 8-39 装配点成为一棵模块树，十五条签名被消掉（V3.43；`bin::assembly::*`）

`bin::assembly` 10,695 → 一棵树，每个文件在 1000 行以内，`[file_length.predating]` 的最后一行被划掉。
**这一刀先改正两条被记录在案、而且都是假的理由**（§8-37 与 §8-38 的引文块），再动代码。

### 为什么是子模块，不是兄弟模块

`RunWorker` 22 个私有字段。**子模块看得见父模块的私有项**——Rust Reference 的 *Visibility and Privacy*：
"If an item is private, it may be accessed by the current module and its descendants"。
于是 `RunWorker` 的定义留在 `assembly.rs`，`impl RunWorker` 的方法散进 `assembly/*.rs`，
**可见性一个字不动**：crate 里 `assembly` 之外的任何模块看到的仍是今天那张脸。
兄弟模块做不到这件事，V3.30 与 V3.42 因此付了 `pub(crate)` 的价；本卡不付。

### 缝在哪：先量再切

切缝取自一次 LCOM 测量（66 个方法对 21 个字段的接触矩阵），不取行数。读数：
`city_root` 被 21 个方法碰，`ledger` 与 `governance` 各 8，`inboxes` 5，`vault` 4，**其余 15 个字段各 ≤ 2 且成簇不交叉**：

| 簇 | 碰它的方法 | 落到 |
|---|---|---|
| `vault`／`expiries`／`logins` | `renew_if_stale`／`login_with`／`put_secret`／`resolver` | `assembly::credentials` |
| `inboxes`／`joins`／`requests`／`goals` | `lay_out_workbench`／`open_desks`／`settle_desks`／`settle`／`deliver_handback` | `assembly::workbench`＋`assembly::settling` |
| `pursuits`／`delegator` | `set_pursuit`／`pursue` | `assembly::commanding` |
| `interrupts`／`watching` | `attach_interrupts`／`watch`／`drive_dispatch` | `assembly.rs`（装的两个钩子）＋`assembly::driving` |
| `knocks` | `knock`／`answer_knocks` | `assembly::dispatching` |

**这份读数说的是 `RunWorker` 是五个类**，而本卡只把它们搬进各自的文件、让边界看得见；
把它们变成真的对象要先分开「判定」与「记账」（每个簇的方法都在 `self.record(...)` 写账本），那是 ARCHITECTURE §5 的
invert the model seam，仍然没有卡，仍然要人先裁一次。**本卡不假装做过它。**

### 门给这一刀定的价：十五条签名必须被修好

`xtask/src/length.rs` 的豁免键是 `路径::函数名`，而 `guard::strikes_only_exemptions` 的 rustdoc 写死了
"an over-long signature may be fixed or left alone, never relocated with its excuse"。
`assembly.rs` 里有十五条超标签名，**它们随文件搬家就失去豁免**，所以逐条消掉。
消法是同一条：**总在一起走、从不被单独选择的值，是一个还没有名字的值**（`Reporter` 的 doc 写下的先例）。

| 新值 | 它是什么 | 消掉了 |
|---|---|---|
| `Assignment` | 一次派活是什么：地址、模式、天花板、谁把它交下来（深度由它推出，不再第二次传） | `dispatch_in` 6→3、`lay_out_workbench` 7→4、`stand_up` 5→2、`settle` 5→3 |
| `Given` | 这一轮活被给了什么：brief、task、goal，与那份字节的 pin | `freeze_plan` 9→4 |
| `Driving` | 一次 drive 跑在什么上面：适配器、工作台、信号桌、写根、围栏域、身份 | `drive_dispatch` 9→3 |
| `Ending` | 一次 drive 以什么结束：结局、被抬起来的审批、代表桌 | `conclude` 10→3 |
| `Sweep` | drive 之后要收的东西：围栏、被抬起来的审批、job locator | `settle_desks` 11→4 |
| `Reach` | 这一轮活够得到谁：邻里与代表 | `status_tool` 7→4 |
| `Entered` | 一个人为接一个 endpoint 输入了什么：名字、base URL、兼容格式、凭证（card-1.1 后为 `Credential` 枚举，不再是「密钥＋鉴权头」两个 `Option`） | `endpoint_of` 5→1、`probe_endpoint` 5→1、`attach_endpoint` 6→2 |
| `Ceilings` | 一行模型声明的两个上限：上下文与最大输出 | `select_model` 5→4 |

`record_for` 的五参消得不需要新类型：`effect::Line` 已经装着 `who`／`addr`／`kind`／`data`，
调用点原本就在把它拆开再递进去，改成整份递。
`settle_requests` 与 `settle_desks` 另外收掉四个参数，因为 `who`／`run_id`／`write_root`／`building`
**本来就是 `Site` 的字段**，调用点在一个一个地从 `site` 里取出来递；`fence_scope` 成为 `Site` 上的方法，
于是「围栏落在楼上还是落在房间上」在本模块只有一个答案。
三处 `#[expect(clippy::too_many_arguments)]` 随之报「这条压制没有被用到」而自己清掉——**修好之后压制自己消失，正是它该有的形状**。

### 十六个子模块，与两次为了行数之外的理由再切的缝

`assembly.rs` 756 行，十六个子模块各在 1000 行以内。其中两刀是重新分配时切的，而切缝仍取自形状：
`settling` 里 `settle_requests` 回答的是「一个不能直接写的楼怎么收下这次改动」，那是评审与合并，成 `reviewing`；
`dispatching` 里 `wake`／`knock`／`answer_knocks` 回答的是「一个没在干活的居民怎么被叫起来」，成 `waking`。
`configure_building`／`create_building`／`adopt_building`／`startup_scan` 从动词表挪进 `genesis`：
**`form_city` 本来就在调 `adopt_building`**，一座城怎么长出楼、重启后看见什么，和一个人发一个动词不是一件事。

### 测试跟着它咬的那个模块，一份夹具留在父模块

`mod tests` 5,576 行，一百个测试函数。**先量后放**：3,490 行只用这个 crate 已经公开的面，
本来可以按 `assembly_door.rs` 与 `web/tests/pages.rs` 的先例去 `crates/sprawling/tests/`。**没有那样做，理由是夹具。**
`fake_openai`（一台按脚本作答的 OpenAI 服务器）、`worker_with_provider`、`completion` 这一簇 443 行，
被两边同时需要：`what_a_worker_holds_is_what_a_restart_rebuilds` 要用它造一段历史再去核 `Standing::fold`，
而 `tests/` 里的验收测试也要用它。**`#[cfg(test)]` 的东西到不了 `tests/`，`tests/` 的东西到不了 `src/`**——
分家就要养两份同名夹具，那是一个夹具两个权威。

所以整套留在 `src/`：夹具成为 `assembly::fixture`（父模块下的 `#[cfg(test)] mod`，十六个子模块都从 `super` 够得到），
每个测试搬到**它咬的那个模块**旁边。**crate 的公开面因此一个条目都没有增加**——
`ledger_dir`／`Views`／`Standing`／`CommandDesk` 全部仍是 `pub(crate)`，`api-baselines` 只多了两行
`impl sprawling::assembly::RunWorker`：`RunWorker` 的方法现在写在三个文件里，`cargo public-api` 就记三个 impl 块。

### 验收

`cargo xtask length` 里 `assembly.rs` 的钉子被划掉而不是被调小；`[argument_count.predating]` 少十五行。
两者都是纯删除，所以 `guard::strikes_only_exemptions` 放行，不需要 `Verdict:` trailer；
budgets.toml 里那两段已经失真的注释单独一枚提交改，因为改注释会让豁免形状判定失效。

## 8-40 判据先于副作用：一次派活在城答应之前不写任何东西（轨道一卡 1）

`dispatch_in` 的开篇注释一字不差地写着这条规矩——「Nothing is written before the city agrees to take
the work: a halted city that laid a job file down would leave a task in a room no run ever opened」——
而代码只守住了停摆那一道。**本节把那句注释变成代码的形状。**

### 量出来的现状

一次派活从人发出的动词到第一次写，走过三段，其中两段先写后判：

| 顺序 | 在哪 | 做什么 | 能不能拒绝 | 写不写 |
|---|---|---|---|---|
| 1 | `commanding::run_command` Dispatch 臂 | `session_for` | `E_INVALID_ARGS`（取不到名字） | 否，但**要花一次 Digest 模型调用** |
| 2 | 同上 | `room_for` → `city::open_room` | 存储错 | **写：房间目录**（`create_dir`） |
| 3 | 同上 | `city::write_effort` | 配置错 | **写：房间的 CONFIG.toml** |
| 4 | `dispatching::dispatch_in` | `halted_by` | `E_GATE_DENIED` | 否 |
| 5 | 同上 | `city::write_brief` | 存储错 | **写：`JOB.md`** |
| 6 | 同上 | `cas.put` | 存储错 | 写：CAS 对象（`.sprawling/` 内，内容寻址） |
| 7 | `workbench::stand_up` | `Building::of`／`city::load`／`load_config`／`Router::select`／`renew_if_stale`／`adapter_for`／`Identity::load` | `E_INVALID_ARGS`／`E_CONFIG_INVALID`／`E_GATE_DENIED` | 否 |
| 8 | 同上 | `run_id_for`／`governance.sent`／worktree 租约 | 存储错 | 写 |

本机实测（`sprawling call` 打到一座刚 init 的城）：派活到从没立过的楼 `gamma`，得到
`E_CONFIG_INVALID「no model is chosen for this tag」`——**来自第 7 段的 `Router::select`**——
而磁盘上留下 `<city>/gamma/one/JOB.md`，账本只有 `seq 0 city_initialized`。
**第 2 段与第 5 段跑在第 7 段之前，这就是全部的病因。** 不是写域逃逸：`Work ".sprawling/evil"`
得到 `E_INVALID_ARGS` 且一字节未落，保留子树守得住。

### 判据

**一次派活在城答应之前不写任何东西；城一答应，第一件被写下的就是房间。**

「城答应」由一处回答，穷尽如下，且每一条都只读不写：保留子树（`Building::of`）、停摆
（`halted_by`）、楼的规矩读得出（`city::load`）、tag 后面有模型且端点还在且不违反 confidential
（`Router::select`）、订阅凭证续得上（`renew_if_stale`）、适配器造得出（`adapter_for`）。

**留在答应之后的两条拒绝，各有其理由，写在这里而不是被含糊过去**：

- `city::load_config` 读城／楼／居民三层。它**必须**在 `write_effort` 之后，因为同一次派活写下的
  effort 要被这一次跑读到（「Chosen once, it holds for every later run in that room」）。把它提前
  会让这次派活看不见自己刚写下的那一层——那是行为改变，不是顺序整理。
- `city::Identity::load` 只在**文件权限**上拒绝；文件不存在读作 ephemeral。那是机器的故障而不是
  这座城的判据，与「派活到没立过的楼」不是一类。

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

`session_for`／`room_for`／`write_effort` 必须搬进 `dispatch_in`，**否则这条规矩就有两个家**：
`dispatch_in` 是唯一被所有派活入口共用的地方（人发的动词、批准后续跑的活、`wake`／`tick`／`knock`／
委派共用的 `dispatch`），而房间是在人发的那条臂上开的。把答应放进 `dispatch_in` 而把开房间留在臂上，
等于让敲门那条路照旧先写后判；把答应也放到臂上，就要在三个调用点各算一次，`renew_if_stale` 会走两趟网。

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
    mode: runtime::Mode,
    budget: kernel::BudgetCap,
    parent: Option<RunId>,
}
```

`session` 与 `effort` 只活到序幕结束，而 `addr`／`mode`／`budget`／`parent` 穿过每一个相位——
一个值里两种寿命，这是本卡自认的代价。**换来的是这条规矩只有一处能被违反**，而上一版的形状是
它有两处、且其中一处没人看着。四个已有字段的语义逐字不变；三个不开房间的调用点写 `session: None,
effort: None`，那正是 `session_for`／`room_for` 对它们本来就有的答案。

### 序幕的顺序，以及为什么是这个顺序

```rust
let agreed = self.agree_to_work(&at.addr)?;        // 只读；第一条拒绝在这里
let session = self.session_for(&at.addr, at.session.take(), &task)?;  // 可能花一次 Digest 调用
at.addr = self.room_for(at.addr, session.as_ref())?;                  // ← 第一次写
if let Some(effort) = at.effort { city::write_effort(&self.city_root, &at.addr, effort)?; }
let brief = city::write_brief(...)?;
...
let mut site = self.stand_up(agreed, &at, &given)?;
```

**`session_for` 排在答应之后**：它可能向 Digest 模型要一个名字，而为一件城不会接的活付一次模型调用，
是这条规矩的钱那一面。**`halted_by` 并入 `agree_to_work`**：它本来就是唯一守住的那道门，
现在与其余五道站在一起，于是「城答应什么」读一处就够。

**采钟点不动**（ARCH §10）：`run_id_for` 的 `now_ms()` 仍在 `renew_if_stale` 之后，
采样次数与相对先后逐字不变；变的只是两者之间多了几次文件写，而那不是任何账本值的输入。

### 谁答哪一个错误码，逐字不变

`agree_to_work` 里判据的先后就是今天的先后，因此**没有一个调用方会看到与今天不同的码**：
停摆答 `E_GATE_DENIED` 而不是 `E_CONFIG_INVALID`（否则会把一个能自己解除的停摆说成要去接 provider），
保留地址答 `E_INVALID_ARGS`。派活到没立过的楼仍答 `E_CONFIG_INVALID`——
**这一条是刻意保下来的**：轨道二的模型把「楼在不在这里不问」记为一件量出来的产品事实
（`adversary/src/Sprawling/Model.hs` 的 `refusal`），改码等于要那份模型跟着改，
而本卡不碰 `adversary/`。加一道「楼必须存在」的前置判断会正好破坏它——这是不选那个修法的第二个理由，
第一个理由是它只修一半（`acme` 真在而没挂 provider 时 `<city>/acme/one/JOB.md` 照旧留下）。

### CAS 那一次 `put` 留在原位，理由写在这里

`cas.put(brief.segment_text())` 仍在 `stand_up` 之前，所以严格地说「答应之后」并非一个字节都不写：
`.sprawling/cas/` 会多一个对象。**留它的理由**：CAS 是内容寻址且去重的，同样的字节写第二次就是同一份，
它不可能在城里留下一间没人开过的屋子；而把它挪到答应之后，`run_id_for` 就得从一个此刻还没入库的
摘要拼出 `cas:b3-…`，于是「locator 钉住的是库里的那些字节」这条权威会有第二处。
本节的判据因此写作「城里人看得见的东西」，而不是含糊的「任何东西」。

### 验收

1. **红转绿（Rust，本仓）**：`a_dispatch_the_city_will_not_take_leaves_no_room_behind`——
   一座刚 init、没挂任何 provider 的城，派活到 `gamma/one`，必须得到 `E_CONFIG_INVALID`，
   且 `gamma` 目录不存在。今天它红在第二条断言上。
2. **红转绿（Haskell，轨道二的检验器）**：`open finding` 那一组两条转绿——`nothingBehind`
   （被拒的派活不改变城的目录树）与 `listsOnlyRaised`（`city_view` 只列被立起来过的楼）。
   跑法见 `HANDOFF-2-adversary.md`；这两条断言是本卡的验收标准，不许为了让它绿而改动它们。
3. **不回归**：`sprawling` 全部单测绿；`a_dispatch_with_no_goal_leaves_no_job_file_and_says_the_person_is_here`
   与 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯着序幕与租约这两支。
4. `just check` 绿。

### 文档同步

本节；`ARCHITECTURE.md` §5 第 3–4 步（派活先答应再写，房间是第一件被写下的东西）；
`crates/sprawling/sprawling-SPEC.md` §8-31 的相位表（`stand_up` 多收一个归位值）。
`city` 与 `channels` 的公开面不变，故 `api-baselines` 不动。

## 8-27 落点一 · 逆携带：变化带着它的撤销值来（路线图卡 3-1）

§8-24 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）。
本卡问的是下一句：**变化落到一半失败，城是什么形状**。今天 `settle` 里五个臂都是
「推过去，错了就把错抛上去」——`Deliver` 循环里第三个 signal 投递失败，前两个已经在
Inbox里；`Shelf` 循环里第二个 filing 写盘失败，第一个已经在架上；`Roadmap` 写半截，
`Hold` 无失败面所以没事。账本是对的（行全落了），城是撕裂的：历史说五个都到了，
城里只到了两个。

**逆携带**：`Then` 的每个臂与其同构造子的撤销值一起走。`Deliver` 带着「这一轮推进了
几个、knocks 推了几个」回来，失败时调用方把没投递的留下（它们本来就在调用方的
`Vec` 里，没丢）、把已推进的 knocks 截回进入时的长度；`Shelf` 带着已写下的路径回来，
失败时把它们删掉（架上无历史，这是 §8-24 那句话的另一半）；`Roadmap` 写之前先读回
原文，失败时写回原文（计划是整份写的，整份写回就是没写）；`Hold` 无失败面，
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

## 8-28 落点二 · epoch 机器：「依赖快照」回答、「轮询」不回答（路线图卡 3-2）

哲学一句话的后半句是「每个依赖驱动其激活」。本卡先回答它的一半：
**快照是依赖的形状，轮询是依赖的反形状。**「散装 notify 轮询」不是指某一个
timer——`SCHEDULE_TICK`（20s，`serving.rs`）本身留着——而是说：今天「什么该醒」
这个问题的答案散在五处，每处各读一遍磁盘，各用各的「上次」：

| 谁问 | 在哪 | 读什么 | 记住什么 |
|---|---|---|---|
| `tick` | `commanding/routing.rs` | `Schedule::load` 全表 | `last_tick`（worker 字段） |
| `wake` | `waking.rs` | `Watch::load` 全表＋`buildings` 全量 | 无（每次全算） |
| `knock` | `waking.rs` | `Identity::load`（逐 room） | 无 |
| `answer_knocks` | `waking.rs` | 无（只 drain） | `knocks`（worker 字段） |
| `dispatch_in` | `dispatching.rs` | `halted_by`（治理折叠） | 无 |

**三处可量**：`wake` 一次读两遍磁盘（watch 表＋全部楼目录）而只为投递一个 arrival；
`tick` 一次读全表而只为问「自上次以来谁到期」；`Watch::listening` 的「楼还在」
每次现算，而楼的生死是账本里变化最慢的事实之一。

### 本卡动的与不动的

**动的只有一处**：`commanding/routing.rs` 的 `tick` 不再读全表，而是读
`city::Schedule::due_after(path, last_tick, now)`——到期判断（`last_firing`
区间比较）下沉到 `city`，`assembly` 只剩 dispatch 循环。`Schedule::due`
（返回 `Vec<&Entry>` 全量引用）保留：它是 `city` 自己的公共面，删它是
`city` 的 breaking，本卡不碰。

**不动的三处，理由各写一条**：

- `SCHEDULE_TICK` 不动：tick 间隔是 serve 面的节奏（§8-38），到期判断是 city
  面的语义，两者不在同一层，换一处不动另一处。
- `wake` 的双读不动：watch 表是人的文件（`listening` 语义含「楼已拆即失聪」），
  到达即读即算正是「文件是人的」这个归属的形状；快照它等于替人记住，归属错。
- `knocks` 不动：它是 3-1 逆携带的截回点（§8-27），形状已钉，动它等于重开 3-1。

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

路线图卡题里的 epoch／LOADING／UNLOADING 是**下一卡（3-3 Assembly 显式化）的
主题**，不是本卡的。本卡只把「到期判断」这一处依赖收成快照的形状
（`due_after` 即运行级依赖快照的最小形态：调用方拿着「到期了什么」，
而不是「全表＋上次」），并给 3-3 留下一句判据：**凡调用方仍在做区间比较、
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

## 8-29 落点三 · Assembly 显式化：接线是一处，判定住 kernel，搬运是值（路线图卡 3-3）

§8-28 给 3-3 留的判据是「凡调用方仍在做区间比较、仍在记 `last_*` 的，皆是
epoch 机器要收走的东西」。3-3 把它收走，并连带回答卡题的三句话。
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
读的是 `governance` 折叠（`HALTED`／`RELEASED`）。「判定进 kernel」在本卡的
含义经核对后收窄：停摆判定读的是**本进程的折叠状态**（`self.governance`），
不是纯函数能回答的问题；硬搬进 kernel 等于把 `Governance` 也搬过去，
那是 3-4 的题（`folds.rs` 957 行）。本卡只做收敛：`halted_by` 的两个调用点
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

§8-28 的判据点名 `last_*`。量过：`last_tick` 是全仓唯一的 `last_*`
（`grep last_` 全仓仅 `assembly.rs:185` 定义＋`routing.rs` 读写＋测试）。
收走：`tick` 的「读表→判断→推进 `last_tick`」三步收成
`RunWorker::tick_after(now)` 仍三步，但 `last_tick` 的读写只在此一函数——
今天已是如此（`routing.rs` 的 `tick` 是唯一读写点），**本句的验收同样是
「无代码变更」**：epoch 机器的第一条轨道（到期判断下沉 `city`）已在 3-2
落定，剩下的 `last_tick` 字段本身是 worker 状态而非散装轮询，
删它等于把「开机不补跑昨日」这个产品语义（§8-6）一并删掉，不删的理由在此。

### LOADING / UNLOADING 在哪

卡题的 LOADING／UNLOADING 落在 `RunWorker::over`（`assembly.rs:239`）：
`Standing::fold` 即全量 LOADING（一次验证、三折叠，一句注释已写明），
而 UNLOADING 是 `close_city` 写 handoff（`assembly.rs:394`）。
两者皆已有名有主，3-3 不给它们改名——**给已存在的东西改名是第二权威，
§8-39 的教训**。本卡只在 `over` 的 doc 上加一句：「此即 LOADING；
UNLOADING 见 `close_city`」，让卡题的词与代码的名在文档里相遇。

### `RunWorker` 立面只减不增

`adapter_for` 搬出后，`impl RunWorker` 方法数减一；`halted_by`／`tick`／
`last_tick` 零增；`Driving`／`Driven` 的字段变化是 `driving.rs` 内部形状，
不进立面。验收：`cargo public-api -p sprawling` 基线零漂移（`impl` 块行数
随文件搬家增减在 2-1 已有口径：同集改写，本卡预计 `commanding/governing`
与 `commanding/routing` 的 impl 行各一，如 2-1 口径同集处理）。

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

## 8-30 余部拆净：views／serving／console／main 按缝归位（路线图卡 3-4）

阶段 3 关版卡。`assembly.rs` 经 3-1→3-3 已成树（本卡零动），余下四文件：

- `views.rs` 995→`holding`（持有＋`apply`）／`answering`（`answer` 面）／
  `lines`（记录→行纯函数）＋`tests.rs`。`Views` 字段改 `pub(super)`
  （两兄弟读），`lines` 八函数改 `pub(crate)`（`folds.rs` 经
  `views::pursuit_from` 仍直达）。
- `serving.rs` 830→`door`（钥匙＋vault）／`desk`（命令台）／`serve`
  （`Serving`＋`Opening` 值）／`worker`（单写者线程＋`serve`）＋`tests.rs`。
  `serve` 与 `Serving` 保持 `pub`（binary 经 `sprawling::serving` 直达，
  公开面零变）；`DeskWait` 改经 `serving::desk::DeskWait` 全路径
  （`pub(crate) use` 转给只在测试出现的名会被门禁记未用——量过，
  全路径是诚实的写法）。
- `console.rs` 781→`language`（`Line`＋`CONTROL`＋解析）／`terminal`
  （`Terminal`＋`Answering`＋`drive` 循环）＋`tests/helpers|parsing|terminal`。
  `drive` 提 `pub(super)`（helpers 直达），`Terminal` 保持 `pub`
  （`Serving.console` 字段,*公开面零变）。
- `main.rs`（bin 根）900→`router`（分派＋flags）／`city`（起服 verbs）／
  `data`（搬运＋查询 verbs）＋`tests.rs`。bin 根的子模块需 `#[path]`
  声明（`mod city` 在 `main.rs` 里指 `src/city.rs`，这是 Rust 的规则不是
  本卡的发明）。`COMMANDS`／`DEFAULT_AT`／`DEPENDENCIES` 各留一处定义，
  跨文件用 `super::` 直达。

钉行 4 删（views/serving/console/main），`drive` 豁免键随文件搬家
（`console.rs::drive`→`console/terminal.rs::drive`），`view_record`
按 C7 口径标 `#[cfg(test)]` 豁免（门禁认属性不认文件）。
`sprawling` 158 全绿，18 门绿，`sprawling` 基线零漂移（`pub` 项未动）。

## 8-31 装配簇归零：在册的最后十一文件（路线图卡 5-2b）

关版卡的 `sprawling` 部分。刀法沿 D7：先迁测试，再按缝切，字段不为跨文件而开；带参数豁免的函数不搬家。逐文件：

- `assembly.rs` 772→365：`fixture`（302 行的 `pub(super) mod fixture`）搬 `assembly/fixture.rs`，路径 `assembly::fixture` 不变，故十六个子模块的测试 `use` 一行未改；`new`／`over`／`close_city` 搬 `assembly/lifetime.rs`（LOADING 与 UNLOADING 是一个生命周期的两端，`over` 的 rustdoc 本就这样写）。子模块读父模块私有字段是 Rust 的规则，`RunWorker` 二十二个字段**无一开放**。`Locator` 的引入随 `close_city` 走，`commanding/tests/answering.rs` 原经 `assembly::*` 借到它，现自引 `kernel::Locator`。
- `workbench.rs` 1000→188：值留父文件（`Site`／`Workbench`／`Reach`／`Desks`／`Situation`＋`status_snapshot`＋`fence_scope`），方法按阶段归子文件：`standing`（`stand_up`）／`desks`（`open_desks`）／`tools`（`lay_out_workbench`＋`status_tool`＋`admit_reading_room`）／`servers`（`mcp_tools`）／`engine`（`execution_engine` 两臂＋`host_shell`）＋`tests.rs`。四个跨 `assembly` 调用的方法由 `pub(super)` 改 `pub(in crate::assembly)`——同一可见范围的精确拼写，不是放宽；子模块读 `Site.branch`／`Desks.waiting`／`Situation` 私有字段走"子读父"规则，**无字段开放**。`engine` 两函数只有 `tools` 与 `tests` 用，不再经父文件转出口。
- `credentials.rs` 930→130：值与读法留父文件（`Entered`／`Chosen`／`Ceilings`＋四常量＋`dialect_headers`／`poisoned_vault`／`dialect_of`／`local_model_facts`），方法按"签入"与"可调用"归 `signing`（`renew_if_stale`／`login`／`login_with`／`put_secret`／`resolver`）与 `endpoints`（`probe_endpoint`／`endpoint_of`／`attach_endpoint`／`probe`／`select_model`／`seed_from_environment`／`open_for_service`）＋`tests.rs`。八个跨 `assembly` 调用的方法改 `pub(in crate::assembly)`；**无字段开放**。
- **`attach_endpoint` 不再以探测为准入条件（card-1.2；gateway-SPEC §8-10 是权威，这里只记装配侧的落地）**：`endpoint_of` 的鉴权头改由 `gateway::AuthSpec::for_dialect` 产出（`Entered.auth_header` 仍恒优先），于是 Anthropic 兼容端点拿到的是 `x-api-key` 而不是必然 401 的 `Authorization: Bearer`；`attach_endpoint` 在探测失败时，若 `admit` 非空则按人报的型号登记（`probed=false`，另写一条 `effect` 级诊断点名探测的错），若 `admit` 为空才拒，恢复语是「把要用的 model id 报上来，再登记一次」。落选的是「探测失败即拒、让人先修好 `/models`」：多数兼容端点根本不服务这个接口，那条路等于让人去修一个对端从未承诺过的东西。
- **`Entered.secret`＋`Entered.auth_header` 合并为 `Credential` 枚举（card-1.1 的必然后果）**：`Absent`／`Key{reference, header}`／`Subscription{reference}`。因为「按兼容格式选头」只对 **API key** 成立：登录挣来的订阅令牌在 Anthropic 那里恒走 `Authorization: Bearer`，若也拿 `x-api-key` 发就是 401。两个 `Option` 拼不出这个区别，于是把它写成穷举枚举：**「订阅令牌装在 key 的头里」现在拼不出来**。`Credential::entered` 是线上命令的唯一入口（线上从不携订阅令牌），`signing` 自己造 `Subscription`。改动面：`credentials.rs` 加类型、`commanding/routing.rs` 两个构造点、`credentials/signing.rs` 一个、`assembly.rs` 一行 `use`。
- **`assembly/folds.rs`**（子代理切，主 agent 复核）：`folds.rs` 957→286。刀法沿 §8-31：先迁测试，再按缝切一簇，字段不为跨文件而开。六条测试按「问的是哪一次折叠」分两份：`folds/tests/standing.rs` 收 `Standing::fold` 的三条（活城与重启折出同一份 governance／collaboration、探针填出的 endpoint book、停摆与放行经账本活过重启），`folds/tests/history.rs` 收 `rebuild_views` 的三条（整城回翻、单会话自取、停在上限的那一页说从哪续）；两份不共用夹具，故无 `helpers.rs`，`folds/tests.rs` 只留 `mod history; mod standing;`。`history.rs` 不再 `use crate::assembly::fixture::*`——那三条测试从未用过夹具，内联 `mod tests` 时它被另外三条借着。迁测后仍 495 行，再切一簇：`Collaboration`／`CollaborationFold`／`artifact_of`／`new_inbox`／`INBOX_CAPACITY`／`SIGNAL_BANDWIDTH` 归 `folds/collaboration.rs`（房间里等着什么、哪块地已被认领），`BlockedJob`／`Sent`／`Governance`／`HALTED`／`RELEASED`／`Standing`／`rebuild_views` 留父文件。原先误挂在 `BlockedJob` 上的两段文档（讲「两个投影」与「筛法重建信号」）随它们描述的类型迁为 `collaboration` 的模块文档，`BlockedJob` 自己那段逐字未动。可见性：`assembly::lifetime` 读 `Collaboration` 的 `inboxes`／`joins`／`goals`／`requests`／`plan_holders` 并调 `pursuits`，`assembly.rs`／`dispatching::running`／`settling::landing`／`workbench::desks` 用 `artifact_of`／`new_inbox`，这七项由 `pub(super)` 改 `pub(in crate::assembly)`——同一可见范围的精确拼写，不是放宽；`CollaborationFold` 与其 `absorb`／`settle` 只有父文件用，`pub(super)` 现指 `folds`。`Collaboration.pursuits`（私有字段）与 `CollaborationFold` 的五个私有字段随 `settle` 同迁，**无字段开放**。父文件因此卸下 `Locator`／`effect`／`pursuit_from`／`building_of`／`plan_node_of` 五个 import，`standing.rs` 自引 `kernel::Locator`。`sprawling` 158 全绿（切前切后同为 6 条 `#[test]`），apisync 基线零漂移，未重写。
- **`assembly/freezing.rs`**（子代理切，主 agent 复核）：`freezing.rs` 778→214：非测试部分（`NEWLINE`／`building_segment`／`run_segment`／`task_line`＋`RunWorker::freeze_plan`）一行未动地留在原文件，573 行的内联 `mod tests` 整体迁到 `freezing/tests.rs`，父文件尾部只余原样保留的 `#[cfg(test)] #[allow(unwrap_used, expect_used, panic, indexing_slicing, reason = "test code")] mod tests;`。测试自身超 400，故 `tests.rs` 退为纯路由（`mod ceilings; mod dispatches;`），八条测试按"问的是什么"分两处：`tests/dispatches.rs` 收一次 dispatch 冻下什么、留下什么——读不动的 handoff 必须点名拒绝、up 模式无自测的改动不落地、job 字节与 must-read 三项进历史、prefix 直接携带楼规与任务而非指路、fork 记血缘并拒非母亲的节点；`tests/ceilings.rs` 收一次 run 在什么之下跑——转派下去的活沿用发它的 ceiling、审批答复续上的活沿用同一 ceiling、config 层解析出的 effort 就是上线的那个，二者共用的夹具 `ceiling_read_by` 只被 `ceilings` 里两条用，随它们同住一文件，故**不设 `helpers.rs`**。两个主题文件按仓内先例用 `use super::super::*;` 回到 `freezing`，再补 `use crate::assembly::fixture::*;` 与 `use crate::assembly::*;`。**无字段开放**，无可见性改动（迁出的只有测试，`freeze_plan` 等仍是 `pub(super)`），函数签名与公共面逐字节不变，apisync 未重写基线。`sprawling` 158 条测试全绿，测试计数切前切后同为 8。
- **`assembly/plans.rs`**（子代理切，主 agent 复核）：`plans.rs` 740→297：一刀即够，只迁测试，生产代码一行未动。`Reporter`／`tell_whoever_is_behind`／`holders_in`／`ready_in`／`plan_item`／`plan_of`／`set_pursuit`／`pursue` 全部留在父文件——它们回答的是同一个问题（一栋楼的计划树：谁占着哪个节点、什么现在可开工、红色能传到哪个房间），拆开只会把「读计划」与「按计划派活」隔到两个文件里，而后者每一步都要问前者。452 行的 `mod tests` 迁至 `plans/tests.rs`，因其自身逾 400 而按主题扁平化为两份：`plans/tests/rows.rs`（计划的行：读不出来的计划按名字拒绝而不赖给邻居、账本拒了那一行则磁盘上的计划分毫不动、一行只能被一次 run 占住、Done 带得回证据）与 `plans/tests/goals.rs`（撞上已占路径的常驻目标按判定它的层级被拒、workshop 图按依赖次序逐房间跑完且结果回汇）。`tests.rs` 只剩 `mod goals; mod rows;`，沿用 `console/tests.rs` 的先例；两份测试各自写 `use super::super::*;` 直取父模块，夹具仍走 `crate::assembly::fixture::*`，**无夹具复制，故无 `helpers.rs`**。原 `mod tests` 上的四条 `#[allow]` 原样搬到父文件的 `mod tests;` 声明上，lint 沿模块树下传覆盖两份子文件。**无字段开放**，无可见性变更（父文件里 `pub(super)` 的两项本就只被 `assembly` 内同级调用，位置未动）；6 条测试切前切后相等，`sprawling` 158 全绿，基线零漂移，apisync 未重写。钉行 1 删（`crates/sprawling/src/assembly/plans.rs`）。
- **`assembly/genesis.rs`**（子代理切，主 agent 复核）：`genesis.rs` 677→329：刀法止于第一刀。文件里只有一簇——「一个目录如何成为城，以及重启看见了什么」：`InitReport`／`Adopt`／`CITY_MD`、读盘的 `standing_of`／`has_history`／`city_address`／`city_segment`，以及 `RunWorker` 上的 `configure_building`／`create_building`／`adopt_building`／`startup_scan`。这四个方法是 §8-31 之前就从动词表挪进来的（本 SPEC 第 1175 行），它们与 `form_city` 共读 `self.city_root` 与 `self.ledger`，拆开只会把一次开城分给两个文件叙述；347 行的 `#[cfg(test)] mod tests` 迁出后父文件 329 行，已在 400 之内，再切一刀就是为切而切。
    `genesis/tests.rs` 352 行，八条测试逐字未动，`use super::*` 与 `use crate::assembly::fixture::*`／`use crate::assembly::*` 原样保留——父文件是 `genesis.rs`，`super` 仍指 `genesis`。父文件尾部留 `mod tests;`，原 `mod tests` 上的四个 `#[allow]` 与 `reason = "test code"` 整份搬到声明上。
    **无字段开放**，可见性一处未改：`pub(super)` 的 `CITY_MD`／`standing_of`／`city_segment` 只被 `assembly` 内同层兄弟读，方法上的 `pub`／`pub(super)` 是 `RunWorker` 的 inherent impl 面，位置不动即路径不动。`apisync` 未重写基线（公开项定义位置未移），`sprawling` 158 条测试全绿。钉行删 1（`crates/sprawling/src/assembly/genesis.rs` = 677）。
- **`assembly/reviewing.rs`**（子代理切，主 agent 复核）：`reviewing.rs` 567→168：`settle_requests` 这一个方法（连同它对 `PrEffect::Opened`／`Merged`／`Rejected` 三臂的处理）整体留在父文件，可见性与签名逐字节不变（`pub(super) fn settle_requests`，仍只被 `assembly` 同层调用，故无须改写为 `pub(in crate::assembly)`）。399 行内联测试迁出为 `reviewing/tests.rs`，原 `#[allow(unwrap_used, expect_used, panic, indexing_slicing)]` 属性列表原样搬到父文件的 `mod tests;` 声明上。
    迁出后测试文件本身 403 行仍越线，故按 D7 的「测试扁平化」再切一刀，缝落在**通过的评审**与**被历史拒绝的合并**之间：`tests/landing.rs` 收 `a_run_under_review_puts_nothing_on_the_shelf_before_it_is_checked` 与 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it`，二者共用的夹具 `branch_opened`（从账本读回请求所在分支）随它们同住，不复制；`tests/refusal.rs` 收 `a_merge_the_history_refused_leaves_the_building_where_it_was`（§8-30 那条红），它走 `open_faulty` + `cut_on_write: Some("pr_merged")`，与前两条一份夹具也不共享。`tests.rs` 只剩抬头与 `mod landing; mod refusal;`。两个子文件的导入沿用 `console/tests/*.rs` 的先例，把原来的 `use super::*` 写成 `use super::super::*`，另两行 `use crate::assembly::fixture::*` 与 `use crate::assembly::*` 原样保留。
    **无字段开放**，无可见性变更，测试计数 3→3 不变，`sprawling` 158 条全绿；apisync 基线零漂移（未重写），钉行 `crates/sprawling/src/assembly/reviewing.rs = 567` 删除。
- **`install.rs`**（子代理切，主 agent 复核）：`install.rs` 556→268：值与判断留父文件（`INSTALLED_STEM`／`SEPARATOR`／`PathEdit`／`PathRemoval`／`Report`／`PathOutcome`＋`program_dir`／`installed_name`／`same_directory`／`plan_append`／`plan_remove`／`on_search_path`／`place`／`displace`／`no_home`／`dirs`／`install`），两条按平台分岔的落地路径各成一个文件：`install/search_path_windows.rs`（`HKCU\Environment\Path` 的原样读写与 `WM_SETTINGCHANGE` 广播，两段 PowerShell 常量、`Raw`／`carrier_path`／`powershell`／`read`／`write` 与 `extend`／`retract`）与 `install/search_path_elsewhere.rs`（不写任何 shell 启动文件，把 `export PATH=…` 那一行随 outcome 交还本人）。原来一个名字 `mod search_path` 带 `#[cfg]` 双身，现在是两个各自 `#[cfg]` 的文件名，`use search_path_windows::{extend, retract}` 与 `use search_path_elsewhere::{extend, retract}` 同样带 cfg；两个子模块仍以 `pub(super)` 向父文件交出 `extend`／`retract`，可见范围逐字节不变，crate 内其它文件的 `use` 一行未改。测试搬 `install/tests.rs`，`#[cfg(target_os = "windows")] mod plan` 原样保留其 `use crate::install::{…}` 绝对路径，故 `mod tests` 内联时的导入一字未动（原来就不是 `use super::*`，保持 `use super::program_dir`）。**无字段开放**；apisync 未重写基线，公共面未动。
- **`mcp_http.rs`**（子代理切，主 agent 复核）：- `mcp_http.rs` 543→322：一刀即够，只迁测试。内联 `#[cfg(test)] mod tests`（含 `vault`／`fake_server`／`sessioned_server` 三个夹具与八个 `#[test]`）整块搬 `mcp_http/tests.rs`，父文件尾部只留带原 `#[allow(unwrap_used, expect_used, panic, indexing_slicing)]` 属性的 `mod tests;` 声明。传输层的值与方法（`HeaderValue`／`Session`／`HttpServer`／`Exchange`／`one_message`＋`Outbound` 实现）全部留在父文件，因为 `HttpServer` 是「主类型＋核心 impl」的形状，把 `post`／`learn`／`forget`／`refused` 拆到兄弟文件只会让读者多跳一次而不减一分复杂度。`tests.rs` 直读 `held.session` 与 `Session.id` 两个私有字段走「子模块读父模块私有项」的 Rust 规则，**无字段开放**；`use super::*` 与 `use protocol::Outbound as _` 原样保留，八个测试的名字与断言一字未动。`sprawling` 158 测试全绿，公开面不变，故 apisync 基线未重写。
- **`plan_view.rs`**（子代理切，主 agent 复核）：`plan_view.rs` 413→241：只用了 D7 的第一刀就够——`#[cfg(test)] mod tests`（174 行）整体搬 `plan_view/tests.rs`，父文件尾部只留带原样 `#[allow(unwrap_used／expect_used／panic／indexing_slicing, reason = "test code")]` 的 `mod tests;` 声明。投影本身不切：`PlanView`／`PlanReading`／`Reading` 三个值、折账的 `apply`、读文件的 `of`、成表的 `describe`／`blockages` 与四个自由函数（`unplanned`／`building_of`／`node_of`／`cause_of`）回答的是同一个问题——「计划上一次读到的样子，以及什么记录能让它作废」——按缝再切只会把一条折叠链拆成两处权威。父文件是 `plan_view.rs`，故 `tests.rs` 里的 `use super::*;` 仍指 `plan_view`，六条测试的断言与名字逐字未改，`crate::plan_view::PlanView`／`PlanReading` 的三处外部引用（`assembly::building_page`、`views::holding`）一行未动。**无字段开放**，可见性一处未变（`PlanView`／`PlanReading` 原本就是 `pub(crate)`，`Reading` 与四个自由函数留在父文件里保持私有）。apisync 基线未重写：公共面逐字节不变。`sprawling` 158 条全绿，`modmap`／`length`／`header`／`apisync` 四门绿。

## 8-40 这台机器有什么，这座城要什么（V4；`bin::doctor`）

**病灶**：README 说「别 `cargo install` 这个东西」，`just check` 要 `just` 与 `cargo-nextest`，客户端要一个版本与 crate 版本相等的 `wasm-bindgen` CLI，exec 工具的 python 臂要一个 CPython-WASI 组件，而这些要求今天散在四份文档里。一个人装不全的时候，得到的是某一条命令的失败信息，而不是一句「这台机器缺什么」。

```rust
// bin::doctor（形状 1 decision：表与判定；驱动只读写它拿到的那两个句柄）
pub(crate) enum Tier { Use, Develop }                  // 两层：用得起来，改得动
pub(crate) enum Need { Required, Optional }
pub(crate) enum Platform { Windows, MacOs, Linux }
pub(crate) struct PerPlatform<T> { windows: T, macos: T, linux: T }
pub(crate) enum Detection { Program { program, version_arg, places }, Environment { variable } }
pub(crate) enum Recipe { Command { program, args }, Print(&'static str), Manual(&'static str) }
pub(crate) struct Requirement { name, tier, need, enables, detect, recipe }
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

// bin::doctor::probe（形状 4 adapter：这台机器）
pub(crate) trait Machine { fn look(&self, r: &Requirement) -> Presence; fn install(&self, name: &str, recipe: &Recipe) -> Result<(), AxError>; }
pub(crate) struct ThisMachine { platform: Option<Platform>, patience: Duration }
pub(super) fn names_of(program: &str) -> Vec<String>;         // Windows 上 .exe／.cmd／.bat 在先，无扩展名在后
```

- **两层而不是一张清单**：一个只想让这座城跑起来的人与一个要改这份代码的人，缺的不是同一批东西。把 Firefox 与 `cargo-nextest` 摆进同一张「缺失」清单，等于告诉前者他缺一个他永远不会用的测试跑器——**判定因此按层给两句话**（`ready to use`／`ready to develop`），而不是一句总分。
- **逐项征求同意，而不是一次总同意**：`--install` 对每一个缺项先印出**这台机器上要跑的那条命令**，再在 stdin 上问 `y/N`，默认是 N。一次总同意会让人对一串他没读过的命令点头，而这些命令改的是他自己的机器。**没被问到的东西恒不安装**。
- **恒不提权**：这里的每条命令都是用户级的（`winget`／`brew`／`cargo install`／`rustup`），`sudo`／`apt` 那一支落在 `Recipe::Print`，人自己贴。一个默认会请求管理员权限的 doctor，是把「检查」变成了「让我动你的系统」，与 §8-9 的 install 同一条理由：**只碰这个人 profile 里的东西**。
- **curl 脚本只印不跑（被否决的备选：`curl | sh` 自动安装）**：bun 在没有包管理器的平台上的官方装法是把一段脚本管进 shell。跑它意味着这座城代替人接受了一份它没读过、也无法在此刻校验的远端代码——**被否决**。那一支是 `Recipe::Print`：命令印在屏幕上，人自己决定。
- **探测是「在不在 PATH 上」加「`--version` 说什么」，且带时限**：一个装坏了的工具会挂在启动上，而 doctor 挂住等于比不装还糟。子进程的读法沿用 `bin::mcp_stdio` 的形状——读在一个线程里，等在一个带 deadline 的 channel 上，超时就杀掉子进程；`patience` 是参数，**不在这里采时钟**。Firefox 另加平台标准安装路径，因为 Windows 与 macOS 上它常常不在 PATH 上。
- **四个文件而不是两个，理由是尺寸与形状**：`doctor.rs` 只留判定（表的形状、`finding_line`、`verdict`），表落 `table.rs`，屏幕与那一问落 `screen.rs`，跑子进程的落 `probe.rs`。判定与驱动同住一个文件时 `doctor.rs` 是 399 行——`xtask length` 的 400 之下一行，即下一次编辑必红。**这不是把文件切碎，是把「判断」与「跟人说话」分开**，两者本就不是一件事。
- **Windows 上先找带扩展名的那个文件**：`bun` 在本机由 npm 装出来，同一目录下既有无扩展名的 shell 脚本 `bun`（Windows 起不动）又有 `bun.cmd`。先取无扩展名的那个，报出来的是「装了但不说版本」——一个装好的工具被报成半坏的。故 `names_of` 在 Windows 上按 `.exe`／`.cmd`／`.bat`／无扩展名的次序找，这条次序有它自己的测试。
- **一项是环境变量而不是程序**：exec 工具 python 臂要的 CPython-WASI 组件由 `PYTHON_WASM_ENV` 指路（`bin::assembly::workbench::tools`），故它的探测是「那个变量指的文件在不在」，安装那一栏是 `Manual`——没有包管理器发它。它是 `Optional`，行尾说明它开启的是什么。
- **终端里的词是英文，这不违反 wording 门**：AGENTS.md 的语言表把 `web::lang` 的管辖写在客户端上，`xtask wording` 扫的目录是 `crates/web/src`（见 `xtask/src/wording.rs` 的 `CLIENT` 常量）。控制台是操作者的，与 `install`／`console`／`firstrun` 同一口径。
- **机器面是一条缝，而不是一个假想缝**：`Machine` 有两个实现——`ThisMachine`（真跑子进程）与测试里的 `ScriptedMachine`（一张 name→Presence 的表，外加它记下的安装请求）。判定因此不需要这台机器上真装着什么就能被咬。

**本章测试**：表的完整性（每一项都有探测方法，且三个平台各自要么给出命令要么明说 `manual`；每一个 `Optional` 项都说出它开启什么）；`verdict` 对「全在」「缺一个必需项」「只缺一个可选项」三类输入给出正确的穷尽枚举（可选项缺失不拖垮该层）；`finding_line` 的三种写法；`--install` 在答 `n` 时**一件也不装**、答 `y` 时只装被问的那一件（由 `ScriptedMachine` 记账）。

**本章验收**：本机 `cargo run -p sprawling -- doctor`，输出逐项与两句判定，必需项有缺则退 1。

**本章未做且已知**：`PYTHON_WASM_ENV` 是 `assembly::mcp` 的私有常量，bin 的别处够不着，故表里第二次拼出了 `SPRAWLING_PYTHON_WASM` 这个名字。**两处拼写由一条测试钉住**（读 `assembly/mcp.rs` 的那一行），但两份权威仍是两份：把它提成 `sprawling` lib 的 `pub` 常量、让 doctor 直接引用，是本卡不动 `assembly/*` 的边界之外的下一刀。

## 8-41 门说的话与门做的事：静默有自己的退出码，重放的命令只做一次（card-4.10.1；`bin::wire_client`、`bin::assembly::commanding::entrance`）

仓外的对抗性检验器（`adversary/adversary-SPEC.md` §4）留了两条未修的发现。两条都只在**门外**可观测，
两条都伤同一类调用方——一个拿退出码分支、拿重试兜底的 agent。本节一次答完，因为它们是同一个承诺的两半：
**门说出口的话必须等于门做的事**。

### 发现一：静默不是接受，故它不是 0

**病灶**：`main/data.rs` 的 `call` 在 `refusals == 0` 时退 0，而它自己的 rustdoc 写着退 1 意为「城拒绝了」。
于是「拒绝没赶上静默窗口」与「城照办了」在退出码上是同一个字。实测见 adversary-SPEC §4 第一个发现：
`AttachEndpoint` 指向一个连不上的 base URL，产品侧探测 15 s，客户端默认窗口 2 s，退出码 0。

**判据**：`call` 有三种结局，不是两种。

```rust
// bin::wire_client（形状 1 decision：三支穷尽枚举，壳只做映射）
pub(crate) enum Spoken { Refused, Answered, Quiet }
pub(crate) struct Heard { frames: u32, refusals: u32, answers: u32 }
impl Heard { pub(crate) fn spoken(&self) -> Spoken; }
```

| 结局 | 退出码 | 它断言的事实 |
|---|---|---|
| `Refused` | 1 | 城在窗口内拒绝了这一帧 |
| `Answered` | 0 | 城在窗口内答了话，且没有拒绝 |
| `Quiet` | 3 | 帧发出去了，窗口内**一帧都没回来**——城是否受理，此处无法断言 |
| （用法错误） | 2 | 参数不是这条命令能读的东西；与城无关 |

- **`answers` 与 `frames` 是两件事**。握手的 `Welcome` 也是一帧，故 `frames` 恒 ≥ 1；能区分静默的只有
  「命令发出**之后**回来的帧数」。把这一个数放进 `Heard` 而不是在壳里减一，是因为「减一」会把握手协议的形状
  抄到第二个地方。
- **3 而不是复用 1**（被否决的备选：静默即拒绝）。静默不是拒绝：城可能已经受理，只是答案比窗口慢。
  把它读成拒绝，会让一个 agent 在城正在照办的时候重试——而重试的无害性正是发现二在修的东西。
- **0 与 1 的含义一个字不改**，故已有的脚本只在原本被误读为成功的那一档上改变行为，这正是本卡要改的那一档。
- **窗口不变长**（被否决的备选：把默认 `--quiet-ms` 提到 20000）。窗口多长是调用方的事；把它调大只是把同一个
  歧义推后 18 秒，而 `Quiet` 让调用方**知道自己撞上了窗口**，于是加窗口重试是它能做的一个决定。

**本节测试**：`a_city_that_says_nothing_inside_the_window_is_not_a_success`——一个脚本化的 WebSocket 服务端
答完 `Welcome` 后闭口不言；`call` 返回的 `Heard` 的 `spoken()` 必须是 `Quiet`，`answers` 为 0。

### 发现二：一把必须带而无人读的钥匙

**病灶**：23 个状态变更命令每一个都带 `IdemKey`，`kernel::gate::dedup` 把这道门实现成纯函数，
而它在自身模块之外**没有调用者**。同一条 `Halt` 发两次，账本里两条 `city_halted`。
`serving::desk` 只合并**还在队列上或正在被执行**的同键命令（`clockwork.rs` 那条测试钉的就是它），
一旦第一条跑完，重放就是第二次副作用。

**判据：判在命令入口，判在任何副作用之前**（`kernel-SPEC.md` §8.2 的原话）。

```rust
// bin::assembly::commanding::entrance（形状 1 decision：状态是集合，判定借 kernel::gate::dedup）
pub(in crate::assembly) struct Entrance { /* seen: BTreeSet<IdemKey>, refused: BTreeMap<…>, carrying: Option<IdemKey> */ }
impl Entrance {
    pub(in crate::assembly) fn answered(&self, key: &IdemKey) -> Option<Result<(), AxError>>;
    pub(in crate::assembly) fn begin(&mut self, key: IdemKey);
    pub(in crate::assembly) fn settle(&mut self, outcome: &Result<(), AxError>);
    pub(in crate::assembly) fn absorb(&mut self, data: &Payload);   // 账本回放
    pub(in crate::assembly) fn stamp(&self, data: Payload) -> Result<Payload, AxError>;
}
pub(in crate::assembly) fn repeated(name: &str) -> String;   // 重复命令留下的那行诊断
pub(in crate::assembly) const IDEM_FIELD: &str = "idem";
```

- **门是 `serve_one`，不是 `handle`**。`serve_one` 是「一个人发出的命令变成什么」的唯一权威
  （`bin::assembly` 的 rustdoc 原话），也是 wire、控制台与 ACP 三条路唯一的汇合点——`serving::worker`
  是它在产品里的唯一调用方。`handle` 是执行者，留给夹具与内部调用方按顺序驱动一座城；
  **门与执行者分开，是因为「判过了吗」与「怎么做」是两个问题**，而把它们合成一个方法会让
  每一个内部调用方都被迫带一把它并没有从人那里收到的钥匙。
- **判定借 `kernel::gate::dedup`，不在这里重写**。`Entrance` 持有那个 `BTreeSet`，kernel 只回答成员关系——
  这正是那个纯函数的 SPEC 说的「seen 集合是调用方的状态」。本卡因此不改 kernel 的立面。
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
- **被否决的备选一：在 `serving::desk` 上记住所有见过的钥匙**。桌子没有账本，重启即失忆；且第一次的结果
  在桌子上不可得，它只能沉默地丢弃重放，而不是回答。
- **被否决的备选二：把 `IdemKey` 从线格式上撤掉**。那是把承诺删掉而不是兑现它，且 23 个命令的重试语义会
  一起消失。

**本节测试**：`the_same_dispatch_twice_under_one_key_opens_one_room_and_starts_one_run`——同一条 `Dispatch`
经 `serve_one` 送两次，钥匙相同：账本里恰有一条 `run_started`，房间恰有一个（此前是 `["one", "one-2"]`）；
`a_repeat_is_answered_with_what_the_first_ask_was_answered`——被拒的命令重发收到逐字相同的那份拒绝，
成功的命令重发不被拒也不再落账；`a_key_already_in_the_history_is_recognised_after_a_restart`——
一座重新打开的城认得账本里那把钥匙。

### 文档同步

本节；`ARCHITECTURE.md` §12 增 `bin::assembly::commanding::entrance` 与两个测试文件的行；
`kernel-SPEC.md` 的 `gate::dedup` 一节记下它的承兑人；`adversary/adversary-SPEC.md` §4 两条发现标注已修。
`docs/operating.md` 增退出码表。公开面：`bin::wire_client` 与 `bin::assembly` 都是二进制内部（`pub(crate)`
以下），`RunWorker::handle` 的签名不变，故 `api-baselines` 不动。

## 8-42 并发地板：一条记账线程，一个驾驶池（v0.0.4 轨道 3，卡 3.1–3.4）

这一节回答一个问题：**这座城怎样同时跑两轮活，而账本上的字节仍然逐字节可重放。**
答案是把今天那一条 `sprawling-runs` 线程一分为二，两半各自持有互不相交的东西。

### 8-42-1 两半各持有什么

**记账线程（accounting thread）**，也就是今天那条唯一写入者，独占下列全部状态，一件不外借：

| 它独占的 | 今天住在哪 |
|---|---|
| `JsonlLedger` | `RunWorker.ledger` |
| 端点书 `EndpointBook` | `RunWorker.book` |
| 计划（`plan_holders`／`bin::plan_view`） | `RunWorker.plan_holders` |
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
就是账本上 settle 各行的顺序**——这是 ARCH §10 规则 5「并行执行，串行记账」在本卡里的具体形状。

**`Interrupt` 按 run 注册**。今天 `RunWorker.interrupts` 是一个 `FnMut(RunId) -> Interrupt` 的钩子，
一次派活借走、结束还回（`driving.rs` 的 `self.interrupts.take()`）。一个池意味着同时有 N 轮活在问
「有人打断我吗」，于是这个钩子从「借走一个」变成「每轮活各注册一份」：`CommandDesk::interrupt_for`
本来就按 `RunId` 挑命令，本卡只是让 N 份 `Relay` 各自带一份指向同一张桌子的注册。

### 8-42-2 `bin::serving::relay`——`kernel::Ledger` 的第三个适配器（卡 3.2）

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

/// 记账那一侧的脸。
pub(crate) struct RelayGate { /* Receiver ＋ 一个用来发牌的 Sender */ }
impl RelayGate {
    pub(crate) fn open() -> RelayGate;
    pub(crate) fn issue(&self) -> Relay;
    /// 把此刻已经在等的请求全部服务掉，一件不留。返回服务了几件。
    pub(crate) fn serve_waiting(&self, ledger: &mut impl kernel::Ledger) -> usize;
}
```

**为什么 `append` 阻塞**：`kernel::Ledger` 的契约写着「`Ok(ref)` 意味着这条记录在那个适配器的介质里已经耐久」。
一个不阻塞的 relay 会在记录还没落盘时就回 `Ok`，那是把契约改写成「已经排队」——
于是 `run_started` 可能排在它自己那轮活的 `model_called` 后面。阻塞是这条契约的价钱，也是它的全部内容。

**回信通道是 `sync_channel(0)`**（会合信道）：一次 `append` 一个回信地址，不留缓冲，
所以「记账线程写完了」与「池线程知道写完了」之间没有第三种状态。

**签名与卡面的一处出入，如实记在这里**：卡 3.2 的说法是「阻塞等 `Result<EventRecord>`」，
而 `kernel::Ledger::append` 的返回类型是 `Result<EventRef, AxError>`。**以端口为准**——
一致性套件是契约（卡面自己这么说），而 `EventRef` 是那个套件检验的东西。
`EventRecord` 会把整条记录复制过河，`EventRef` 不会，且 `EventRef` 无法伪造（ARCH §9）。

**一致性套件原样通过**：`kernel::ledger::conformance::assert_ledger_conformance` 一个字不改地跑在 relay 上。
套件要求的 `LedgerInspect` 是「只为验证而设」的读回面（`ledger.rs` 的模块文档），
本节因此把它实现在测试里的一层包装上，而不是在 relay 的生产面上开一个读洞：
套件检验的是 relay 的 `append` 路径（seq、prev、字节、两次新实例产生同样的字节），那正是契约。

**服务顺序：relay 请求排在 desk 命令之前**。理由是一个已经在跑、已经花了钱的活，不该排在一条还没开始的命令后面；
反过来排会让一次 `Dispatch` 命令挡住三轮正在写 `tool_result` 的活。

### 8-42-3 `bin::serving::pool`（卡 3.3）

形状：**adapter**。N 条 `std::thread`，从 `bin` 里那唯一的 spawn 点起（ARCH §10 规则 3）。
入口 `(RunId, Driving)`，出口 `(RunId, Driven)`。

**线程 panic 不是一种情况**：发布档是 `panic = "abort"`（ARCH §2），没有可以接住的东西；
一次失败作为 `Driven::Failed` 走回来，而不是作为一个 join 出来的 `Err(Box<dyn Any>)`。

**池大小 = `min(准入天花板, 配置值)`**。准入天花板住在记账线程上（`gateway::admission` 已经持有
provider 的并发上限），配置值是人写的。取小的那个：比天花板大的池只会让线程停在 admission 上排队，
那是把排队从一个会算数的地方搬到一个不会算数的地方。**citysim 跑池大小 1**——
六个场景必须逐字节重放同样的账本，而池大小 1 时 `Driven` 的到达顺序就是发起顺序，
于是确定性不依赖调度器。

### 8-42-4 `dispatch_in` 一分为二（卡 3.4）

`dispatch_in` 今天从「城答应」一路跑到「run 冻结」再到 `conclude`。本卡让它在把 `Driving` 交给池之后就返回
`Dispatched::Started`，于是记账线程的主循环有三张嘴：

1. **relay 请求**（先服务，理由见 8-42-2）
2. **desk 命令**（`CommandDesk::wait`）
3. **`Driven` 到达**（池的出口）

`settling` 一个字不改：它本来就只在记账线程上跑，本卡只是让它的触发点从「`drive_dispatch` 返回」
变成「`Driven` 到达」。**敲门仍在 settle 之后发出**，与今天一样。

准入计数住在记账线程上：它是「同时有几轮活在跑」的唯一权威，而唯一权威必须在唯一写者那一侧，
否则两条线程各数各的，就有了两个答案。

### 8-42-5 被否决的备选

**备选一：多进程。** 一轮活一个进程，各自持有自己的一份状态，用管道汇总。
**否决理由有三条，任何一条都够。**
第一，账本的 `seq` 与 `prev` 是一条链，`kernel::ledger` 的契约把 seq/prev 的分配交给实现；
多进程要么共享一个写者进程（那就是本卡的 relay，只是把 `mpsc` 换成了一个需要序列化、需要重连、
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

1. **红转绿（卡 3.2）**：`the_relay_passes_the_ledger_conformance_suite`——
   `kernel::ledger::conformance::assert_ledger_conformance` 原样跑在 relay 上，
   记账那一侧是另一条线程持有的账本。
2. **红转绿（卡 3.4）**：`driving/tests` 里两次派活交错——账本 `seq` 保持单调，
   每一轮活的 `run_started` 排在它自己的 `model_called` 之前。
3. citysim 六个场景在池大小 1 下逐字节重放同样的账本。
4. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings` 与
   `cargo nextest run -p sprawling --locked --all-features` 绿。

### 8-42-7 本会话没做完的部分，以及挡住它的那件事（如实记录）

**落地的**：8-42-1 的判据、8-42-2 的 relay（含一致性套件）、本节其余部分作为设计。
**没落地的**：卡 3.3 的池与卡 3.4 的 `dispatch_in` 拆分。

**挡住它们的是一件量出来的事实，不是时间**：`Driving` 今天跨不过线程边界，而且它跨不过去的理由
不在 `bin` 里。逐字段看：

| `Driving` 的字段 | 能不能 `Send` | 为什么 |
|---|---|---|
| `adapter: Box<dyn Model + Send>` | 能 | 已经带 `Send` |
| `bench: &mut ToolBench` | **不能** | `ToolBench.tools: BTreeMap<String, Box<dyn Tool>>`，而 `kernel::Tool` 没有 `Send` 上界（`crates/kernel/src/tool.rs:254`） |
| `signals: &Rc<RefCell<SignalDesk>>` | **不能** | `Rc` 不是 `Send`；五张桌子（signals／goals／plan／shelf／pr）全是 `Rc<RefCell<_>>`，且工具持有它们的克隆（`workbench/desks.rs`） |
| `write_root: &Path`／`fence_scope`／`who`／`run_id`／`of` | 能 | — |

于是卡 3.3 有一张**必须排在它前面的卡**：把 `kernel::Tool` 的上界加上 `Send`，
把 `Desks` 的五个 `Rc<RefCell<_>>` 换成 `Arc<Mutex<_>>`，并让 `drive_dispatch` 从
「借走 `&mut self.ledger`」改成「拿着一个 `Relay`」。这张卡横跨 kernel／runtime／collab／city／
protocol／browser 六个 crate 的工具立面，不是 `bin` 里的一次改动。
**在它落地之前把池写出来，只能写成一个泛型的、没有第二个实现的空壳**——
那正是 AGENTS.md「一个只有一种实现的接口是装饰」要挡住的东西，所以本会话不写它。

### 8-42-8 C 波的进展，与 3.3／3.4 仍未动的原因（如实记录）

**落地的**：§8-44——`kernel::Tool: Send`、`runtime::Sandbox: Send`、`protocol::Outbound: Send`、`memory::vfs::Vfs: Send`，
七张 collab 桌子、`ReadTool` 的 catalog、`SucceedTool` 的桌子、`mcp_stdio` 的连接、装配层的 `Desks`／`Workbench`／`Reach` 全部换成 `Arc<Mutex<_>>`；
`Driving` 现在拥有自己的 `signals` 句柄、一份 `Cas` 第二句柄（§8-43）与 backlog 成员号（runtime-SPEC §8-28-2）。
`driving/tests/turns::a_drive_can_be_handed_to_another_thread` 由编译失败转绿：`Driving<'static>: Send` 成立。

**仍未动的**：卡 3.3 的池与卡 3.4 的 `dispatch_in` 拆分。
挡住它们的这次不是类型，是**同一波里另一条线正在重写同一段代码**：card-11.5（succession）把「后继 run 的 `dispatch_in`」放进了 `conclude`，
与 `delegate` 子 run 的递归派活并列；3.4 要把「drive 之后的一切」（`settle_desks`／`settle_requests`／`conclude`）从 `dispatch_in` 的尾部搬到「`Driven` 到达」那张嘴，
恰恰是那一段。两条线同时改一个函数体，谁后写谁赢，那比没有池更糟。

**下一会话的第一刀，已经量好**：`drive_dispatch` 今天还是 `&mut self` 的方法，用到工人的四样东西——
`self.ledger`（改经 `Relay`）、`self.watching`（`Arc<dyn Fn + Send + Sync>`，可克隆）、`self.interrupts`（改为按 run 注册，§8-42-1）、`self.backlog`（`Clone`）。
把这四样收成一个 `DriveContext { ledger: Relay, watching, person, backlog }` 值，`drive_dispatch` 变成 `Driving` 上的自由函数 `drive(driving, context) -> Driven`；
这一刀不改任何账本字节，落地后池就是「N 条线程各拿一份 `DriveContext`」，而 3.4 只剩把 `dispatch_in` 在交出 `Driving` 之后 `return Dispatched::Started`，
并把尾部搬进一个 `fn land(&mut self, Continuation, Driven)`——`Continuation` 就是今天 `dispatch_in` 尾部用到的那几个局部：`site`、`at`、`desks`、`workbench`、`job_locator`、`fence_scope`。
`serving/relay.rs` 的四处 `#[cfg_attr(not(test), expect(dead_code, …))]` 因此还在：池仍无生产调用方，它们仍在期待。

## 8-41 一次提交出自哪次运行，从账本回答（card-2.4；`bin::views::commits`、`sprawling whose`）

card-2.1–2.3 让这座城作出的每一个提交都带上五条 git trailers 与一个
`<actor>@<city 前 12 位>.sprawling` 的署名。那是**给城外读者的投影**：拿着 `git log`
的人看得见谁写了这一行。反方向还欠着——**手里只有一个 oid 的人，问不出它出自哪次会话**，
而这正是一个人在 `git blame` 之后会问的下一句话。

### 折叠，不是去读 git

```rust
// bin::views::commits（形状 7 projection）
pub(super) struct CommitFacts { /* run、seq、actor、model、effort —— 私有 */ }
pub(super) fn commit_facts(record: &EventRecord) -> Option<(GitOid, CommitFacts)>;
impl CommitFacts { pub(super) fn answer(&self, oid: GitOid) -> channels::CommitAnswer; }
```

- **两种记录进这张表**：`checkpoint_committed`（键 `oid`，工具波的栅栏与基线提交）与
  `pr_merged`（键 `commit`，一次评审落地时 trunk 收到的那个合并提交）。这两种是
  这座城**唯一**会造出提交的两处。
- **派活开场那条 `checkpoint_committed` 不进表**：它是「活钉在这里了」的销钉，载荷只有
  `job`，没有 oid。判据因此是「读得出一个 oid」而不是「是不是这个 kind」——
  一个不带 oid 的检查点行不是一次提交。
- **`run`、`seq` 与 `actor` 取自记录自己的身份**（`EventRecord::run`／`seq`／`addr`），
  `model` 与 `effort` 取自载荷（memory-SPEC §8-18）。**没有一个字段是从 git 读的**：
  投影读权威，不读另一份投影。
- **重复的 oid 后写覆盖前写**：同一个 oid 只可能由同一次提交产生，两条记录说同一件事时
  它们说的是同一件事。
- **`session` 由地址算出而不是另存一份**：房间是地址的最后一段，`city::open_room` 当初
  就是拿人给的名字开的它；派到楼根的运行没有房间，故答 `None`。

### 城外那扇门：`sprawling::ask`（住 `bin::views::holding`）

```rust
pub fn ask(city_root: &Path, query: &channels::Query) -> Result<channels::Answer, AxError>;
```

**住 `views` 而不住 `assembly`**：它的全部内容就是「造一份 `Views`、问一句、丢掉」，
而 `Views` 的一生是 `holding` 的；另一个理由是 `assembly.rs` 已站在 400 行预算上（本卡开工时
401 行，wave A 遗留），而为一行重导出把一道门推得更红是拿门当对手。

一次性折叠这座城的账本并回答一个 Query，然后把视图扔掉。**它与被端上来的城答的是同一个
`Views::answer`**——若 CLI 自己另写一份读法，同一个问题在这座城里就有两个答案，
而漂开的总是没人看的那一个。链先被 `runtime::replay::verify_ledger_dir` 验过：
历史不成立的城，它的视图不该被端出来。

### `sprawling whose <city> <oid>`（`bin::main::whose`）

四行输出：run、actor（带 session）、model 与 effort、以及账本位置 seq。
**自己一个文件而不是 `main::data` 多一个臂**：`data` 里每一个动词不是搬字节就是验链，
而这一个是从城的历史里读出一个答案并把它渲染给人看；且 `data` 已到 360 行，
把四十行放进去得拿别处的行数去换。
- **不接受短 oid**：`GitOid::parse` 只认 40 位小写十六进制，长度不对即拒而不是补齐去猜。
- **退出码**：0 答上了；1 这座城没写过这个提交（`Unavailable`）；2 命令行读不了。
  「城没写过它」是 1 而不是 0，因为一个脚本据此判断「这一行是机器写的吗」时，
  把「不知道」读成「不是」会把审计写成假话。

### 红转绿

`a_commit_the_city_made_says_which_run_wrote_it`（`bin::assembly::driving::tests::ledger`）：
一次真派活跑完，从账本里取出那条带 oid 的 `checkpoint_committed`，用它的 oid 问
`Query::Commit`；答必须给出这次运行的 run、房间地址、模型 id 与档位，以及那一行的 seq。
未落地时这条查询答 `Unavailable`，红就红在这里。

### 文档同步

本节；`channels-SPEC.md` §8-17 与 §2 的 golden（WIRE_V 13→14，Query 15 个）；
`memory-SPEC.md` §8-18；`ARCHITECTURE.md` §7 的两个数与 §12 的 `bin::views::commits` 一行；
`README.md` 的 History 一节；`adversary/adversary-SPEC.md` §2 的线面计数。
公开面：`channels` 增 `Query::Commit`／`Answer::Commit`／`CommitAnswer`，
`memory` 增 `Provenance::model_fields`／`model_choice_of`／`effort_word`，
`sprawling` 增 `ask`，故 `api-baselines` 三份随本卡重算。

**接线到哪一步了，以及四处 `#[expect]` 的账**：记账那一侧已经接上——`spawn_worker` 在循环外开一个 `RelayGate`，
循环里第一件事是 `worker.serve_relay(&relay)`，然后才 `worker_desk.wait(...)`，
于是「relay 请求排在 desk 命令之前」这条规矩现在由代码持有而不是由一段文字持有。
池那一侧还没有生产调用方，于是 `Relay`、`RelayGate::issuing`、`RelayGate::issue` 与 `gone` 在非测试构建里是死代码。
本卡的处理是四处 `#[cfg_attr(not(test), expect(dead_code, reason = …))]`——
**用 `expect` 而不是 `allow`，正是因为它会自己清掉**：卡 3.3 一落地，这四条期待就变成「未兑现的期待」而编译失败，
删掉它们是那张卡必须做的事，而不是某个人必须记得的事。这是本会话唯一一处压制，也是我请人过目的那一处。

### 城市立起来时就有市政厅，和一条写下来的代答（v0.0.4 card-5.1／5.2）

**`form_city` 在 line zero 之后多做三件事**，顺序固定：

1. 追加一条 `autonomy_changed`，值 `delegate:hall/clerk`。**不改 `AUTONOMY_DEFAULT`**：缺省值说的是「没人说过话时怎么办」，而这里是这座城市作出的一个决定，人可以改它，改它要有一行历史可改。`folds` 读回这条线，重启后 clerk 依旧是代答者，无需第二处记忆。
2. 用 `city::CityPlan::new(None).hall()` 拿到那栋楼，走 `create_building` 落 `BUILDING.md` 与脊柱文档，并记 `building_created`。走这扇门而不是另写一段，是为了让市政厅与任何一栋楼在历史里长得一样。
3. `city::lay_out_hall_identities` 把 `MAYOR.md` 与 `CLERK.md` 写进 `<city>/.sprawling/`，已存在的不覆盖。

**影响面（一处真实回归，已改）**：从此每座城市至少有两栋楼。`views::tests` 里两处按 `buildings[0]` 取楼的断言改成按地址找 `lab`——它们原本靠「城里只有一栋楼」这个此后不再成立的前提。改的是测试对现实的假设，不是把判据放宽。

**留给后续卡**：`hall` 的居民目前拿到的仍是 `workbench::tools` 给所有人的同一套工具表，`exec`／`delegate`／`workshop` 都在里面。card-5.3 才按地址裁这张表；在那之前，市政厅「不建造」只由写域挡住（`Documents` 拒非 `.md`），不由工具表挡住。

## 8-43 筛子接进产品：`driving` 把 `exec` 结果经 `pipeline::package` 交给模型（card-11.4 收尾；`bin::assembly::driving`、`runtime::pipeline::exec`）

**量出来的现状**：runtime-SPEC §8-27 的筛子完整落地，`pipeline::package` 也已带 `SieveRequest` 臂，但它在产品里没有调用方——`bin::assembly::driving` 的 `invoke` 钩子把 `BenchOutcome::Ran` 的结果原样交回 `runtime::turn`，于是压缩器只在 citysim 跑，城里的模型读的是 `cargo check` 的一千两百行原文。§8-27-9 末尾的「已知未接」说的就是这一处。

### 一个门，两个调用方

citysim 的 `citysim::sieving::package_exec` 是「一份 `exec` 结果怎样变成模型读到的东西」的第二份定义，产品接线若再写一份就是第三份。本卡把它搬进 runtime，一个权威、两个调用方：

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

**`stamp` 传 `None`**：时钟戳由 `runtime::turn` 既有路径打在结果尾部（`FrozenConfig.clock_stamp`），本卡不在第二处打。

### 验收

1. **红转绿（`driving/tests/sieving`）**：一次真实派活，`exec` 打印一份超过 2 KiB 的输出；模型收到的工具结果含 `[sieve:` 页脚且短于原文；账本 `tool_result` 载荷的 `sieve[0].original` 是 `cas:b3-` 且能从 CAS 读回原文，`rest_path` 在磁盘上。
2. `citysim::sieve::the_window_holds_the_diagnostics_and_the_way_back_and_only_the_news_the_second_time` 绿。
3. 同种子 citysim 逐字节重放不变（`the_same_seed_and_table_replay_a_byte_identical_window`）。

**留给 gitignore 的一句**：`.rest/` 住房间里，楼的 `.gitignore`（city-SPEC §8-21）应忽略它，否则围栏提交会把一份 rest 文件收进历史。本卡不改那份文件，它归 card-5.x 的作者。

## 8-44 `Driving` 跨过线程：`kernel::Tool` 加 `Send`，五张桌子从 `Rc<RefCell<_>>` 到 `Arc<Mutex<_>>`（§8-42-7 量出来的那张前置卡）

**问题**：§8-42-7 逐字段量过——`Driving` 今天跨不过线程边界，卡住它的是两件不在 `bin` 里的事实：
`kernel::Tool` 没有 `Send` 上界，于是 `ToolBench.tools: BTreeMap<String, Box<dyn Tool>>` 不是 `Send`；
五张桌子（signals／goals／plan／shelf／pr，加 delegates 与 workshop 两张只在 workbench 里的）全是 `Rc<RefCell<_>>`，工具持有它们的克隆。
没有这一刀，卡 3.3 的池只能写成没有第二实现的空壳。

### 判据：什么要变、什么不变

| 东西 | 今天 | 本卡后 | 理由 |
|---|---|---|---|
| `kernel::Tool` | `pub trait Tool` | `pub trait Tool: Send` | `Box<dyn Tool>` 由此自动 `Send`；一个不能跨线程的工具在这座城里没有位置——它会被池线程调用 |
| `protocol::mcp::Outbound` | 无上界 | `: Send` | `McpTool` 持 `Box<dyn Outbound>`；两个适配器（stdio 子进程、HTTP 客户端）本来都是 `Send` |
| collab 七张桌子的句柄 | `Rc<RefCell<Desk>>` | `Arc<Mutex<Desk>>` | 桌子本身没有 `Rc`，换句柄不换桌子 |
| `runtime::ReadTool.catalog` | `Rc<RefCell<Catalog>>` | `Arc<Mutex<Catalog>>` | 同上 |
| `runtime::StatusTool.children` | `Box<dyn Fn() -> Vec<ChildStatus>>` | `+ Send` | 闭包持派生台句柄 |
| `bin::mcp_stdio` 连接 | `Rc<RefCell<Connection>>` | `Arc<Mutex<Connection>>` | 同上 |
| `bin::assembly::workbench::{Desks, Workbench, Reach}` | `Rc<RefCell<_>>` | `Arc<Mutex<_>>` | 出借与收回的地方 |
| `Driving.signals` | `&Rc<RefCell<SignalDesk>>` | `Arc<Mutex<SignalDesk>>`（拥有） | 池线程不借工人的东西 |
| `memory::vfs::Vfs`（内缝） | 无上界 | `: Send` | 红测试量出的第七处：`Cas` 持 `Box<dyn Vfs>`，而 §8-43 让 `Driving` 带一份 `Cas`；`RealFs` 本来就是 `Send`，`FaultFs` 的 `Rc<RefCell<State>>` 换 `Arc<Mutex<State>>` |

**`try_borrow_mut` 失败 → 锁中毒**：`RefCell` 的「桌子在用」拒绝换成 `Mutex::lock` 的阻塞——那正是要的语义：两条线程同时到一张桌子前，后到的等，不是被拒。
`lock()` 的 `Err` 只有一种含义——持锁线程 panic 了——而发布档 `panic = "abort"` 下它不会发生；映射成 `E_STORAGE_FATAL`「桌子被一条死掉的线程留在锁里」，与 `runtime::backlog::hold` 同一句话。

**不变的**：桌子的内容、每张桌子的 `take_effects`／`take` 语义、`settle_desks` 的顺序、账本上的每一个字节。
这是一次句柄类型的迁移，不是一次行为变更；citysim 六个场景逐字节重放不变是它的验收。

**红测试**：`bin::assembly::driving::tests::turns::a_drive_can_be_handed_to_another_thread`——
`fn crosses_threads<T: Send>()` 对 `Driving<'static>`；今天这一行不编译（`Rc<RefCell<SignalDesk>>` cannot be sent between threads safely），本卡后编译并通过。
类型层面的红转绿正是 ARCH §9「unrepresentable 本身是需要测试的断言」那一条的用法。

**迁移一次做完**（AGENTS.md「完成每一次迁移」）：每个读者与写者一起搬，旧形状删除，不留 `Rc` 版本的构造函数。

### 8-40 花费闸删除后的装配面（card-11.7）

`Assignment`／`Given`／`Knock`／`Sent`／`BlockedJob` 五个结构各去掉一个 `budget` 字段，`DISPATCH_TURN_BUDGET` 与 `RunPlan.budget_turns`／`RunPlan.budget` 一并删除，`run_started` 载荷不再写 `usd_micros` 与 `tokens`，`JobBrief` 不再有 `budget` 一节，`StatusTool` 的十三字段变十二。

- **一条派活不再有回合上限**，`runtime::run::drive` 循环到这次跑自己结束为止：一回合作出结论、一次带 carrier 的失败、或一个安全点送到的中断。停一件正在跑的事仍是 `Cancel`，停一片仍是 `Halt`——后者现在真的会终止那片里的后台成员（card-11.2）。
- **`assembly/freezing/tests/ceilings.rs` 删去两条断言**（派下去的活与被批准接着跑的活各自「在派它的上限下」跑）。它们检验的性质不存在了，留着就是在检验一个没有主语的句子；文件保留 effort 那一条，模块头写明删了什么、为什么。
- **golden-p0 账本随之重生**（`GOLDEN_WRITE=1`）：`run_started` 少两个整数键。V8 跨版本字节夹具本来就为这种形状变更而存在。
- **未做（本卡之外，交给关波的 agent）**：`xtask/api-baselines/` 下 kernel／channels／web／sprawling 四份基线需 `just api-baseline` 重生——kernel 去掉 `BudgetCap` 一族、增 `GovernedDocumentWritten`，channels 去掉 `BudgetCap` 再导出、增本线三帧与三个答面类型，web 增 `put_document_command`。

### 8-41 治理两帧的执行与答（card-5.4）

- `RunWorker::put_document` 写 `<city>/.sprawling/` 下三份文件之一（经 `city::write_governed`，路径由 `city::Governed` 决定而不由帧决定），随后记一行 `governed_document_written`，载荷携文件名与字节数、**恒不携正文**——正文在盘上可读，抄进账本就是同一段话有了两个权威。
- `Views` 新增两个字段：`autonomy`（折自 `autonomy_changed`）与 `decided`（折自 `approval_resolved`，旧在前）。两者一起答 `Query::Governance`。`decided` 收下每一条被答过的审批，包括人自己答的——只列代答的清单会让「我答过」与「从没人答」在界面上长得一样。
- **`views::apply` 迁入 `views::holding`**：`answering.rs` 加上本卡与 card-2.6 的两臂后越过 400 行，而折叠本来就是 `holding` 自称拥有的东西（「what the views hold and how one record folds in」）。切完 `holding` 318、`answering` 335，无新模块行。

### 8-42 `Query::Hunks` 的答（card-2.6）

`views::answer` 的新臂调 `memory::of_file`，把 `memory::PatchLine`／`Withheld` 逐字段搬成线上的同名形状。这座城没写过的 oid 答 `Unavailable`，与 `Changes`／`Commit` 同口径：「没有变化」与「我看不了」是两个答案，读的人对它们的下一步不同。

## 8-45 一个能看见自己造出来的东西的居民（card-4.2／4.3／4.4；`bin::browser_bidi`、`bin::browser_tool`）

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

**本模块是三个文件**（400 行的价目表逼出来的一刀，切在三件事之间而不是切在行数上）：`engine.rs` 判「哪个引擎、命令行长什么样」，`lazy.rs` 持「还没起的那个引擎」与端口推导，`socket.rs` 只运字节。`browser_bidi.rs` 因此是索引，一行逻辑也没有。

`plan` 是纯函数，端口由调用方给，于是「参数长什么样」这件事在没有浏览器的机器上也逐字可断言；`launch` 只是 `Command::spawn` 加一个「进程死了就报出来」。**时间与随机都不在这里取**（ARCHITECTURE §10 第 2、4 条）：端口由 `port_for(city_root)` 从城目录的 BLAKE3 摘要推出，落在 40000–59999。这既不采时钟也不取熵，而同一台机器上的两座城本来就在不同目录里——用已经把它们区分开的那件事去区分端口，比再引入一个随机源更少一处不确定性。等待浏览器起来靠**敲门次数**而不是截止时刻，因为读时钟的地方只有 `bin::assembly` 一处。

### 8-45-2 `bin::browser_tool`——八个动作，一个会话

工具住这里而不是 `browser` crate，理由是卡 4.3：截图要落 `memory::cas`，而 `browser` 依赖图里没有 `memory`，也不该有。把 CAS 塞进 browser 会多一条本可不存在的依赖边；把工具放在装配层，`browser::verb` 的判定与 `memory::cas` 的字节各自留在自己那侧，中间只有一个 `Shot` 值。

工具持有：一个 `Box<dyn BrowserPort>`、一个 `Session`、当前 `ContextId`、上一次 `PageSnapshot`（快照的 generation 由它递增）、一个 CAS 句柄。八个动作即 `browser::verb::Verb` 的八个变体，一个不多一个不少。

`effect` 是 `Effect::Egress`：浏览器打开的每个 URL 都离开这台机器，所以它过出网门，confidential 楼因此天然拿不到它。

### 8-45-3 截图成为证据（卡 4.3）

一次 `screenshot` 的落点有三处，缺一处这张图就不是证据：

1. 字节进 `memory::cas`，得到一个 `cas:b3-…` 定位符——历史里恒不出现图片字节；
2. 结果载荷带上定位符与两个整数尺寸，于是模型即使不看图也知道它有多大；
3. `ToolOutcome.attachments` 带上 `ImageRef`，`runtime::turn::wave` 把它原样放进 `ContentBlock::ToolResult.attachments`，于是这张图真的到得了模型眼前。

**`kernel::ToolOutcome` 因此加一个字段** `attachments: Vec<ImageRef>`，`#[serde(default)]`，旧历史读成空列表。这是本卡唯一一处跨 crate 的形状变更，波及每一个构造 `ToolOutcome` 的工具（全部改为显式空列表），不改任何一个的行为。卡 4.1 已经把 `ContentBlock::ToolResult.attachments` 与两条 dialect 备好，`wave.rs` 里那句「a tool that produces a picture fills this in where it runs」等的就是这一步。

### 8-45-4 `BUILDING.md` 的 `browser: true|false`（卡 4.2、4.4）

`city::policy` 多读一个键。默认 **false**：一栋楼不写这行，它的居民就没有浏览器。这与 `confidential` 的「不写即报错」不同，理由是两者的失败方向相反——隐私设置读成宽松的一侧是事故，而工具没给到只是少一件工具。confidential 楼恒为 false，写了 `browser: true` 即拒，因为一个能开任意 URL 的浏览器就是一条出网路径，而「数据不出去」是那栋楼的全部意思。

### 8-45-5 验收

| 单元 | 完成的定义 |
|---|---|
| browser_bidi | Firefox 与 Chromium 的参数各自逐字断言；驱动不在即点名拒绝；两次 plan 的端口来自参数而非采样 |
| browser_tool | 录制适配器上重放 open→snapshot→act→screenshot 一整条；截图后 CAS 里有字节、载荷里有定位符与尺寸、attachments 里有一个 `ImageRef` |
| BUILDING.md | 不写 `browser:` 即没有；confidential 楼写 `browser: true` 即拒 |

## 8-46 同一栋楼里的并发：驾驶池、`dispatch_in` 一分为二，与拿走整个 ready set 的 `pursue`（v0.0.4 轨道 3，卡 3.3／3.5）

**开工时量出来的现状，与卡面不符，如实记在最前面**：卡 3.3（池）与卡 3.4（`dispatch_in` 一分为二）在 C 波**没有落地**——
`crates/sprawling/src/serving/` 下没有 `pool.rs`，`dispatch_in` 仍是「城答应 → 冻结 → 驾驶 → 归位 → conclude」一条直路，
`serving/relay.rs` 里那四处 `#[cfg_attr(not(test), expect(dead_code, …))]` 仍然挂着。§8-42-8 自己写下了这件事。
C 波真正落地的是 §8-44 的 `Send` 迁移，也就是这张卡的前置条件。**本节因此把 3.3 与 3.5 一起做完，并把 3.4 做窄**，
窄在哪里、为什么，写在 8-46-2。

### 8-46-1 `Driving` 拥有它驾驶所需的一切，`drive_dispatch` 变成自由函数

§8-44 让 `Driving<'static>: Send` 成立，但今天构造出来的 `Driving<'a>` 仍借着三样东西：
`&mut workbench.bench`、`&site.write_root`、`&site.who`，而 `drive_dispatch` 还是 `&mut self` 的方法，
用着工人的 `ledger`／`watching`／`interrupts`／`backlog`。一个借着调用栈上局部变量的值送不进线程。

| 字段 | 今天 | 本卡后 | 理由 |
|---|---|---|---|
| `bench` | `&'a mut ToolBench` | `ToolBench`（拥有） | `Workbench.bench` 改为 `Option<ToolBench>`，由 `take_bench` 取走一次。与 `Site.adapter` 同一手法：`Option` 是搬运的车，不是新状态 |
| `write_root` | `&'a Path` | `PathBuf` | 一次 clone，一次驾驶 |
| `who` | `&'a str` | `String` | 同上 |
| `plan`／`handoff` | `drive_dispatch` 的两个参数 | `Driving` 的两个字段 | 驾驶要的东西在一个值里，池的入口才是一个值 |

`drive_dispatch` 拆成两半：

```rust
/// 一次驾驶从工人那里拿走的四样东西，克隆而不借。
pub(super) struct DriveContext {
    watching: Option<Arc<dyn Fn(channels::Delta) + Send + Sync>>,
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

### 8-46-2 `dispatch_in` 一分为二，但续段留在调用者手里（卡 3.4 的窄形）

```rust
/// 驾驶之前城已经做完的一切，与驾驶之后要用到的一切。
pub(super) struct Continuation { at: Assignment, site: Site, desks: Desks, workbench: Workbench, job_locator: Locator, member: Option<runtime::BacklogId> }

fn prepare_dispatch(&mut self, at: Assignment, task: String, goal: String) -> Result<(Driving, Continuation), AxError>;
fn land(&mut self, continuation: Continuation, driven: Result<Driven, AxError>) -> Result<Dispatched, AxError>;
```

`dispatch_in` 于是等于 `prepare_dispatch` ＋ 在本线程 `drive_run` ＋ `land`，**行为一个字节不变**，
所以它今天的每一个调用方（人的 `Dispatch`、敲门、排程、`delegate` 的递归派活、succession 的后继）都不必知道这次切割。

**窄在哪里**：卡 3.4 要把 `Continuation` 交给工人，让记账线程的主循环长出第三张嘴（`Driven` 到达）。
本卡不做那件事，续段是 `pursue` 的一个局部 `BTreeMap<RunId, Continuation>`。
**理由是可以说清的一条**：主循环长出第三张嘴，意味着 `serve_one` 之后一条命令可能还没做完，
而 `commanding::entrance` 的幂等门是在 `serve_one` 里「开始—落定」一次答完的（§8-41）——
一个还在跑的 `Dispatch` 要怎样落定它的键，是一张属于那张门的卡，不是这张卡顺手能改对的东西。
把并发先给 `pursue`，是因为 `pursue` 是这座城里唯一一处**自己就是一个循环**的派活点：
它已经在等，等的时候顺手服务 relay 与 `Driven`，不需要任何一张门改变它的语义。

### 8-46-3 `bin::serving::pool`（卡 3.3）

形状：**adapter**（ARCH §9 第 4 种）。文件 `crates/sprawling/src/serving/pool.rs`。

```rust
/// 一轮跑完的活回到记账线程时带的两样东西。它们成对，因为
/// 一个 `Driven` 不说自己属于哪一轮，一个 run id 也不说要归位什么。
pub(crate) struct Arrival { run: RunId, driven: Result<Driven, AxError> }

pub(crate) struct DrivingPool { /* lanes、一个 mpsc 的两头、每条车道的 JoinHandle */ }
impl DrivingPool {
    pub(crate) fn open(lanes: u32) -> DrivingPool;
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
admission 上排队**——§8-42-3 早就写下这句话，本卡把它从设计变成一个带理由的常量。

### 8-46-4 `pursue` 拿走整个 ready set（卡 3.5；`bin::assembly::plans::pursuing`）

今天的 `pursue` 是「取一个 → 跑完 → 再取一个」，一次只有一轮活。本卡把它搬进 `plans` 下的一个自己的模块
（`plans.rs` 留 `set_pursuit` 与读计划的那几个私有方法；子模块看得见父模块的私有项，所以这一刀没有把任何字段变公开），
并改成：

1. 读整个 ready set；对其中每一个节点，只要车道没满就 `prepare_dispatch` 并交给池。
2. 车道满了就不再取：`kernel::observe(state, &ready, in_flight)` 的 `in_flight` 参数**第一次有真值**，
   于是「没什么可取但有人在跑」如实答 `Waiting { in_flight }`，而不是像今天那样恒传 0。
   **已经在别人手上的节点不在问它的那个 ready set 里**：ready 的意思是「现在可以有人接手」，
   而一个已经被接手的节点不能被接手两次。过滤在调用点做，判据仍然是 `kernel::pursuit` 的。
3. 等：先 `gate.serve_waiting(&mut self.ledger)`（relay 请求排在一切之前，§8-42-2），再看有没有 `Driven` 到达。
4. 到达即 `land`，**按到达顺序**（§8-42-1），落完账再回到第 1 步——一轮活结束可能让新的节点变 ready。
   一轮活回来时它那个节点仍然 ready，说明这一轮什么也没认领：记一条 `Refuse` 诊断，并停止再取新活
   （在跑的活照样等回来落账），这与本卡之前那条「不再派它一次」的规矩是同一条。
5. ready 空且没有人在跑 ＝ `Finished`，退出。

**账本上的写者仍然只有一个**：车道线程手上唯一的 `kernel::Ledger` 是 `Relay`，`JsonlLedger` 一步不离记账线程。
`RelayGate` 由 `pursue` 自己开一扇，发出去的 `Relay` 与它一一对应；`serving/worker.rs` 主循环里那一扇仍在，
仍然一件不服务——它属于卡 3.4，而不属于这一张。`relay.rs` 里四处 `#[expect(dead_code)]` 随本卡删除，
正如 §8-42-8 所预言的：它们是会自己清掉的期待。

**评审楼一轮活一个 worktree 是既有事实，本卡只验证不重做**：`stand_up` 用 `WorktreeName::parse(&run_id.to_string())`
认领工作树，名字是 run id，所以三轮活就是三棵树。同理，卡 11.6 已把 Handoff 从楼搬进房间，
三轮活分属三个房间时各写各的 `Handoff.md`；这也是本卡只读不改的东西。

### 8-46-5 citysim：一条账本、两轮活、逐字节重放（卡 3.6）

池大小 1 是模拟器的确定性条件（§8-42-3），所以 citysim 不跑池。它要证的是另一半：
**两轮活的记录交错在同一条账本上时，同一批脚本重放出同样的字节**。
`run_scenario` 今天每次自己造一个 `MemLedger`，于是「两轮活一条账本」在 citysim 里根本拼不出来。
本卡加一个入口：

```rust
pub fn run_scenario_on(ledger: &mut MemLedger, scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

`run_scenario` 变成「造一条账本，调它」——一个权威，两个调用方。新场景
`two_runs_interleaved_on_one_ledger_replay_byte_identically`：两个 run id、两个房间、各自的脚本模型，
跑在同一条 `MemLedger` 上，`seq` 单调、`prev` 成链（`check_chain`），再跑一次逐字节相同。

### 8-46-6 文档停止过度承诺（卡 3.7）

`README.md` 两处写着「多个 agent 同时工作」。本卡之前这句话在产品里没有主语：`pursue` 一次只跑一轮活。
本卡之后它成立，但只在一个位置成立，文档必须把那个位置说出来，而不是继续说一句听上去更大的话。
`ARCHITECTURE.md` §11 的性能记录增一行**并发墙**：一座城同时驾驶的轮数由车道数决定，
而车道之外的第一堵墙是 provider 的 admission 天花板；账本仍然是串行的，那是 §10 规则 5 的价钱。

### 8-46-7 验收

1. **红转绿（卡 3.5）**：`bin::assembly::plans::tests::goals::three_ready_nodes_drive_three_runs_at_once`——
   三个 ready 节点、一个 pursuit，假 provider 记下同时在飞的请求峰值。串行时峰值为 1，红就红在这里。
2. **红转绿（卡 3.6）**：`citysim::scenario::two_runs_interleaved_on_one_ledger_replay_byte_identically`。
3. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings`、
   `cargo nextest run -p sprawling --locked --all-features`、`cargo nextest run -p citysim --locked --all-features` 绿。
4. `cargo xtask modmap`、`length`、`header` 绿。

### 8-46-8 本卡之外仍然欠着的（如实记录）

- **卡 3.4 的完整形**：记账线程主循环的第三张嘴，与 `commanding::entrance` 幂等门在「一条命令还没做完」时的落定语义。
  在它落地之前，人从界面派的活仍然是一次一轮，只有 `pursue` 是并发的。
- **provider 天花板的可读形**：`gateway` 增一个公开的读法，`bin` 于是可以取 `min(天花板, 配置)`，
  而不是像本卡这样让两个数字碰巧相等。这张卡要重算 api-baseline。

## 8-50 三种读法的答：回合、证据、一个节点的花费（card-6.5；`bin::views::rounds`、`views::evidence`、`views::cost_of`）

`channels` 定形状（channels-SPEC §8-21），这里折出答。三个新模块都是形状 7 投影：把记录折成页要的读法，删掉重折逐字节相同。

### 8-50-1 `bin::views::rounds`——一次会话折成回合

`records_of(run)` 用 `LedgerIndex::run_seqs_before` 取这次跑最新的 `HISTORY_MAX` 条 seq，逐条读行、逐条解析，旧在前。**上界与客户端原来问的那一段等宽**：`web::live::page` 一直是 `RunHistory { limit: HISTORY_MAX }`，所以搬到服务端之后一个会话能被读到的范围一字未变——搬家不该顺手改答案。读不回来的一行**截断而不清空**：读到的那些仍然是真的（同 `Views::history`）。

折叠本身逐字从 `web::turn::rounds` 搬来，一条不改：`model_called` 开一个回合；`tool_called` 挂进当前回合并按 runtime 给的 id 记下等答；`model_returned` 落到最近一个回合上；`tool_result` 按 id 找回它自己的那次调用——**恒不按位置配对**，因为两个调用可以先后发出而后发的先答；其余记录按 `channels::reading::note_of` 判是否成为一条 `Note`。`opened_at` 是本会话第一条 `Fenced` 的 oid：那是这份活开始时的树，取最新的一道栅栏答的是另一个问题（「上一波动了什么」）。

### 8-50-2 `bin::views::evidence`——写下来的证据，不携字节

同一份记录再走一遍，只认两种：`tool_result` 载荷 `result.image` 是 `cas:` 定位符的，成 `EvidenceKind::Screenshot`（连同 `width`／`height`／`media_type` 三项，缺一则 `picture` 为 `None` 而行仍在）；`roadmap_finished` 载荷 `evidence` 能读成定位符的，成 `EvidenceKind::Finished`。定位符读不回来的记录**不成行**——发明一条指不到任何东西的证据比少一行糟。

### 8-50-3 `bin::views::cost_of`——节点到跑，跑到钱

`Views` 多一张 `claims: BTreeMap<NodeId, BTreeSet<RunId>>`，由 `roadmap_claimed` 折出（载荷 `node` 加记录自己的 `run`）。`BTreeMap` 而非 hash：这是答一个查询的路径，顺序必须是确定的。答时把这些跑在 `memory::attribution` 的 `by_run` 里各自的数取出来相加——**这里不计价**，计价是 `gateway::cost` 的，归因是 `memory::attribution` 的，本模块只做一次求和。

### 8-50-4 验收

- `views::rounds::tests` 与 `views::rounds::reading_tests` 把 `web::turn` 的两份测试逐字搬来，跑在服务端的折叠上：**服务端算出来的回合等于视图层对同一批记录算出来的**。红是在真账本上取的——`Query::Rounds` 先答 `Unavailable`，`asking_for_rounds_answers_the_fold_the_view_layer_ran` 在 `init_city` 铺出的城上写三条记录再问，失败于「Rounds answers with rounds」。
- `views::evidence::tests`：一张截图与一条完成证据各成一行，读不回定位符的载荷不成行。
- `views::cost_of::tests`：认领过的节点报出那次跑的钱；没人认领过的节点报 0 与空明细，而不是 `Unavailable`。
