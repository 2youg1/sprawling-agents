-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::server

规定 `server`、`server::config`、`server::config::enrolment`、`server::socket`、`server::bundle`、`server::uploads`、`assets`、`answer::cost`（`crates/wire/src/` 下同名的文件）。监听的一端：判定是纯函数，套接字一个也不做；客户端资产、几扇 HTTP 门与 Query 的答面。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `wire::assets` 旁的测试守住。
-/

/-!
### 8-2 wire::server（形状 4 薄壳）＋reception（形状 1）＋assets（形状 4）

**Humble Object 在此的切法**（ARCHITECTURE §7 末段，理由只写一次）：难测的一端（tokio＋axum 监听）剥到最薄，厄的一端（绑定面判定、握手判定）是纯函数，无需跑服务即可穷尽测。

**策略不住 `server`。** 模块表给 `server` 的形状是 `adapter`——ARCHITECTURE §9 定义为「薄、无策略：换第二个实现不改变任何策略」——而判定函数就是策略。所以它们住 `wire::reception`（绑定／录凭证／握手／帧，HTTP 门的配对判定在 `reception::admission`），客户端资产的判定住 `wire::assets`，`server` 是一个目录：`server.rs` 只有声明与重导出，`config`（`ServeConfig` 与路由表）、`listener`（§8-46）、`socket`（一条 WS 会话）、`committed`（§8-47）、`bundle`（送页面的两条路由）、`uploads`（`/acp`／`/transcribe`／`/drop` 三个处理器）与 `config::enrolment`（`/enroll`）各一文件；其中每一条分支不是一次发送、一次接收，就是一次会话的结束。公开名经 `lib.rs` 重导出，调用方写 `wire::ClientAssets`、`wire::router`，不写定义模块。

```rust
// 面里携着它索要的凭证：暴露面因此不能“要求空”。
pub enum BindFace { Loopback { token: Option<B3Hash> }, Exposed { token: B3Hash } }
pub enum BindVerdict { Serve(BindFace), Refuse(AxError) }
pub fn decide_bind(addr: &SocketAddr, token: Option<B3Hash>) -> BindVerdict;
impl BindFace { pub fn token_digest(&self) -> Option<&B3Hash>; }

pub enum EnrollVerdict { Accept, Refuse(AxError) }
pub fn decide_enroll(peer: &SocketAddr) -> EnrollVerdict;   // 只认回环调用方
pub type SecretSink =
    Arc<dyn Fn(Command<Sealed<String>>, Reply) -> Result<(), AxError> + Send + Sync>;
pub struct EnrollBody { pub realm: String, pub name: String, pub value: String }

pub enum Door { Transcribe, Enroll, Acp, Drop }      // 四扇 HTTP 门，穷尽
pub enum Pairing { Held, Absent }                    // 不是 bool；住 `wire::auth`，不带 `server` 也在（D2）
pub enum Admission { Admit(Pairing), Refuse(AxError) }
pub fn decide_admission(door: Door, offered: Option<&str>, face: &BindFace)
    -> Admission;                                    // HTTP 侧唯一的令牌判定
pub fn offered_pairing(header: Option<&str>) -> Option<&str>;  // `Authorization: Bearer`

pub enum HandshakeVerdict { Accept, Reject(AxError) }
pub fn decide_handshake(hello: &Hello, expected: &Welcome, face: &BindFace) -> HandshakeVerdict;
// 路由表拿到的也是这个面：壳自己不再读配置里的令牌，因为那样“要求什么”就有两个家。
pub fn router(config: &ServeConfig, face: BindFace) -> Router;

// 先占住端口，再交出城：绑定判定与 bind 在任何 sink 存在之前做完（§8-46）。
pub struct Bound { /* 已绑定的监听器、它的地址与它的 BindFace —— 私有 */ }
pub async fn bind(addr: SocketAddr, token_digest: Option<B3Hash>) -> Result<Bound, AxError>;
impl Bound { pub fn local_addr(&self) -> SocketAddr; }
pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError>;

// 壳自己不能决定的一切，由装配层递入；每个 sink 是闭包而不是 trait（wire 不在缝清单上）。
pub struct ServeConfig {
    pub client: Arc<ClientAssets>,                       // 客户端资产源
    pub transcribe_sink: TranscribeSink,                 // §8-27
    pub drop_sink: DropSink,                             // §8-49
    pub acp: AcpSink,                                    // 外来编辑器
    pub commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
    pub events: broadcast::Sender<Committed>,            // §8-47
    pub deltas: broadcast::Sender<Delta>,                // 可丢的增量
    pub logs: broadcast::Sender<LogLine>,                // 可丢的进程日志
    pub outputs: broadcast::Sender<LiveOutput>,          // 跑着的命令写出的字节
    pub outputs_so_far: Arc<dyn Fn() -> Vec<LiveOutput> + Send + Sync>,
    pub monitor: MonitorFeed,
    pub queries: Answering,
    pub secrets: SecretSink,
    pub city: Option<Address>,
    pub head: Arc<LedgerHead>,                           // 最后一条广播记录的 seq，Welcome 读它
    pub epoch: Option<B3Hash>,                           // 账本第一行的链哈希
}

// 客户端资产面。
// 判定纯函数化（Humble Object 同款切法）：哪个路径答哪些字节、要不要
// Content-Encoding、为什么 miss，全部离线可测；handler 是三行壳。
pub struct EmbeddedFile { pub path: &'static str, pub gz: &'static [u8] }
pub enum ClientAssets {
    Embedded(&'static [EmbeddedFile]),  // 发布形：二进制内的 gzip 文件表
    Disk(PathBuf),                      // 开发形：--web-dir 逐请求读盘
}
pub enum AssetReply {
    Found { bytes: Vec<u8>, content_type: &'static str, gzipped: bool },
    Miss(AxError),
}
impl ClientAssets { pub fn lookup(&self, request_path: &str) -> AssetReply; }
// 送页面的两条路由：`/` 与同一张表里没有别的路由认领的每一条路径。
pub fn bundle_routes<S: Clone + Send + Sync + 'static>(client: Arc<ClientAssets>, headers: &PageHeaders) -> Router<S>;
```

**客户端是一张资产表，不是一个文件**：`ServeConfig` 携 `client: Arc<ClientAssets>`，因为 `client/` 的构建产物是 `index.html` 加它引用的脚本、样式与字体，只携一个文件的形状会让页面壳引用一条服务端没有的路由。资产表是封闭清单：路径穿越（`..`、空段、盘符、点头文件）在判定层拒，miss 报文件名并给出重建口令。`Disk` 臂逐请求读盘，专供开发回路（改前端刷新即见），发布路径恒不构造它。

**送页面的路由只有一张表**（D20）：城自己的端口把 `bundle_routes` 并进 `router`，远程监听（`crates/remote_access/Spec.lean` §8-10）也并进它自己的那张表，所以设备打开远程地址拿到的字节与这台电脑上的浏览器拿到的相同；响应头按监听器生成、作为参数交进来（§8-94 末节）。

**公开签名不携传输层的类型**：sink 收 `Vec<u8>` 而不是 `axum::body::Bytes`。**一个泄露自己传输层的公开签名，会把「换掉 HTTP 库」变成对每一个从未选过它的调用方的破坏性变更**。

**令牌只以摘要形式进入本 crate**：拿令牌的一方自己摘一次，边界只比摘要；`expose` 只得出现在兑付点（`xtask secret`）。代价为零（常数时间比较本来就要先摘），收益是 `wire` 在类型上根本拿不到配对令牌的明文。常数时间比较因此退化为定长 32 字节的无早退异或，**既无内容侧道也无长度侧道**。

`decide_bind` 的四格真值表是全部行为：回环×有钥匙＝`Serve(Loopback)`；非回环×有钥匙＝`Serve(Exposed)`；**两面×无钥匙＝`Refuse(E_CONFIG_INVALID)`**。拒绝发生在**启动时**，不是启动后拒连——它是配置判定。

**三个平台上同一条规则**：「回环」由 `IpAddr::is_loopback` 判（IPv4 的 `127.0.0.0/8` 与 IPv6 的 `::1`），标准库在 Windows、macOS、Linux 上给同一个答案，所以 `decide_bind` 与 `decide_enroll` 不分平台。一个 IPv4 映射地址（`::ffff:127.0.0.1`）不算回环：绑定在这样的地址上按暴露面判，比需要的更严而不更松。对端地址不同：监听在 `[::]` 上时，Linux 与 macOS 缺省接收 IPv4 连接并把对端报成映射地址，Windows 缺省不接收（三者 `IPV6_V6ONLY` 的系统缺省值不同，本 crate 不设它）；于是在 Linux 与 macOS 上，同机经 IPv4 连到 `[::]` 监听器的 `/enroll` 报来的对端是 `::ffff:127.0.0.1`。规则的本意是「只认同一台机器」，所以 `decide_enroll` 先取 `IpAddr::to_canonical` 再判 `is_loopback`（`crates/wire/src/reception.rs` 的 `decide_enroll`）：映射回环被接受，映射的外部地址（`::ffff:203.0.113.7`）仍被拒，三个平台给同一个答案。**被否掉的做法**：在监听器上设 `IPV6_V6ONLY`——那改的是哪些连接进得来，不是谁算同一台机器，且要在每个平台上各设一次。

薄壳的职责恒为三件：静态资源（前端产物，`bundle_routes`）｜WS 升级（`/ws`）｜四条 HTTP 路由（`/enroll`、`/transcribe`、`/drop`、`/acp`）。它不持业务状态，不做策略判断：`/enroll`、`/transcribe`、`/drop` 由 `decide_admission` 在门前判配对，`/acp` 在处理器里经同一个函数判，因为编辑器把令牌放在正文的一个键里而不是请求头里；送页面的两条路由不设配对，因为还没拿到配对码的浏览器也得先载入输入配对码的那张表单。

**WS 路由与两条沿途缝**。升级后的会话只做三件事：先收 `Hello` 并交 `decide_handshake` 判（拒即关，不降级）；收到 `ClientFrame::Command` 交给 sink；把订阅到的 `EventRecord` 以 `ServerFrame::Event` 推给客户端，醒来时已经到了的记录作为一帧写出、刷写一次（`crates/wire/spec/Server/Socket.lean` §8-47h）。

```rust
// ServeConfig 的两项（全表见上）：
pub struct ServeConfig {
    /// 命令受理面：**只受理，不执行**。同步、不阻塞；真正的回合循环在装配层自己的任务里跑。
    pub commands: Arc<dyn Fn(WireCommand) -> Result<(), AxError> + Send + Sync>,
    /// 事件广播源。本 crate 只 `subscribe`，恒不发送——写入方是 Ledger。
    /// 每条携记录与它已拼好的 `Event` 帧，见 §8-47。
    pub events: broadcast::Sender<Committed>,
}
```

- **为什么 sink 只受理不执行**：一个 Dispatch 会跑几分钟到几小时。把它做成 `async` 并在 socket 任务里 await，等于把一条连接的寿命绑在一次派活上；刷新页面就会杀掉工作。**受理后立即返回，进展从 Ledger 的事件流回流**——这同时使「关掉界面再打开」与「从未关过」在服务端看来无差别。
- **为什么广播的是 `EventRecord`（裹在 `Committed` 里）而不是自定义推送体**：客户端要重建的正是那一行历史。另造一个推送类型等于为同一件事立第二个形状权威，而两者一旦漂开，界面会显示一个历史里没有的事实。
**回信地址（`Reply`／`Delivered`）**。受理与执行分开之后，工人的拒绝没有任何通道回到发问的那个 peer——回程只有 `EventRecord` 广播。真机派活验出的后果是：**一个人在设置页点 attach，base_url 少了 `/v1`，页面一个字都不说**，那条拒绝只躺在服务端自己的日志里。

```rust
/// 一条拒绝的去向，三态穷尽。
pub enum Delivered { ToThePeer, NobodyAsked, PeerGone }

/// 回信地址。`Fn` 而非 tokio 通道，故本类型不把传输层写进签名。
pub struct Reply(/* private */);
impl Reply {
    pub fn to(sink: impl Fn(AxError) -> Delivered + Send + Sync + 'static) -> Reply;
    pub fn nowhere() -> Reply;                    // 排程自己发起的活，没有发问者
    #[must_use] pub fn refuse(&self, error: AxError) -> Delivered;
}

/// 受理之后回给外来编辑器的全部内容（`POST /acp`）。
pub struct AcpProgress { pub run: String, pub turns: u32, pub finished: bool }

pub commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
```

- **`Reply`、`Delivered`、`AcpProgress` 住 `wire::reply`，`Pairing` 住 `wire::auth`，四个都不在 `server` feature 之后**：城的写者 `accounting::worker` 点名它们，却从不监听端口，而它依赖本 crate 时关掉 `server`（D2）。`server` 只管 axum 那一半；把拒绝写成 HTTP 响应体的 `refusal_text` 仍在 `server::config`。
- **拒绝属于发问者，不广播**。把它做成一条事件会告诉所有在看的人「别人打错了一个字」，而事件流是这座城的历史，不是某个人的错字簿。故每条会话自持一个无界队列，`Deliver` 时把写入该队列的闭包随命令交给工人；会话的 `select!` 因此从两臂变三臂。
- **`Delivered` 是三态而不是 `Result`**，因为「没有人问过」与「问的人走了」是两件不同的事：前者是排程的正常形态，后者值一行诊断。这也是**不得重新引入 `let _ =`** 的落法——`SendError` 被穷尽消解成一个领域枚举，而不是被丢掉。
- **无界队列而非 `broadcast`**：一条拒绝丢不得，而它的量级是「人点错的次数」，不是事件流量。

**Query 的答面**。`ServeConfig.queries: Answering`，即 `Arc<dyn Fn(Query) -> (Seq, Result<Answer, AxError>) + Send + Sync>`，同步；`Seq` 是答案读出时的账本位置，随答案以 `ServerFrame::Answered(Box<Answered>)` 回给发问的会话。答面类型住 `answer`：`Answer` 每个查询一个变体（如 `City`／`Run`／`Approvals`／`Cost`／`Building`），连同 `RunSummary`、`CityAnswer`、`ApprovalsAnswer`、`CostAnswer` 等各自的答面。

**`Query::BuildingView { addr }` → `Answer::Building(Box<BuildingAnswer>)`**（`BuildingDoc`／`ArchiveLine` 随之入 wire）。楼里的文件是楼的记忆，服务端在被问的那一刻读盘——**文件是权威**，另存一份索引就是第二个权威。`QUERY_NAMES` 因此多这一项，schema 哈希随名字表而变：客户端与服务端同批发布，旧页面在握手期被明确拒绝并提示刷新。

**`Welcome` 携 `city: Option<Address>`，`decide_frame` 经 `WelcomeFacts { city, head, epoch }` 收到它。** 事件流只送连接之后发生的事，而城市的名字写在 Ledger 的第一条记录里——一个今天打开的浏览器永远等不到它。握手是「这是哪座城」的自然回答处；服务端从同一条创世记录读它，故两边不构成第二个权威。`init` 把城市名写进创世记录的 `addr`，城市名因此不只活在目录项里。

**`ApprovalsAnswer.items` 携 `kernel::ApprovalItem` 全项，不另设摘要类型。** 摘要会丢掉 `cluster_key` 与 `created`，于是界面无法按类聚合、也排不出「谁等得最久」，服务端为了填它还要从事件载荷里猜一个从来没被写过的字段。载荷本身就是 `ApprovalItem` 的序列化，原样送过去既少一次有损转换，也让「什么算一类」只由客户端读 `cluster_key` 这一处回答。

- **为什么是强类型答面而不是一团 `Payload`**：客户端读帧的词汇是 `client/src/wire.ts`，由 `cargo xtask wire-ts` 从本 crate 的 schema 生成，故发帧的边界 crate 欠对方一套有类型的读帧词汇（同 kernel 再导出的理由）。一个无类型载荷会把解析责任推给每一个视图模块，每一个都得自己猜一遍形状。
- **`Answer::Unavailable { query, reason }` 是一个真答案**：不求值的视图报自己的名字，而不是返回空结果——空城与未实现在界面上必须长得不一样。看了却没看成的视图在 `reason` 里说出没看成的原因（D47）；`reason` 缺席时视图只报名字：这份构建不求值这个查询，或这个读面还没有说出原因。
- **`CityAnswer.buildings: Vec<BuildingProgress>`**：每栋楼一行，携 `Progress` 与 `problems`。解析不出的行进 `problems` 并照显——悄悄丢掉读不懂的行，等于按一个没人选过的分母报进度。
- **五维成本携权威总额**：`CostAnswer.total` 与 actor、segment、tool、skill 四个维度各自求和相等；`by_run` 只带活跃的跑与花得最多的前几个（`crates/sprawling/Spec.lean` §8-90），和可以小于 `total`。界面按 `total` 算占比而不自己归一，未归因余额与列表之外的跑因此都看得见。
- **无报价的调用单独报数**：`CostAnswer.unpriced: UnpricedCalls { calls, tokens }` 是账本上没有权威计费额的模型调用次数与它们的 token 总数（`storage::Attribution` 的 `unpriced` 原样上线）。它们不进 `total`，所以缺了这一项，一座只用订阅登录或本地模型的城跑了多少次都读作「没花钱」；界面据 `calls > 0` 说「有调用没有报价」并给出 token 数，而不是把 `$0.00` 当作量出来的数。

- **采用 `broadcast` 而非每连接一个队列**：多个标签页是常态；慢客户端被拉下而不拖住写入方。**丢下的那一段不再静默**：事件流慢过城的会话收到 `ServerFrame::Lagged { from, to }`，按这个区间向账本补拉（§8-41）。三路语义不同，故这三节分开陈述：事件可补、增量与日志恒不可补、会话自己的拒绝根本不走广播。

**`POST /enroll`，唯一携凭证字节的路由**

它是 HTTP 而非 socket 帧，因为 socket 的 `WireCommand` **拼不出** `PutSecret`（`NoSecret` 无值）。两半合起来才是完整保证：类型层管住帧，`decide_enroll` 管住字节——因为字节总可以被 POST 到一个路由上。

- **只认回环对端，配对令牌也不算数**：令牌认的是人，而这条规则管的是**字节走到哪里**。拒绝的第三段（recovery）指向宿主机，于是它是约束而非死路。
- 壳里零策略：判定在 `decide_enroll`，壳只搬字节——同 `decide_bind`／`decide_frame` 的切法，故无需跑服务即可穷尽测。
- 应答返回那条 `secret_captured` 记录写下的 `ref`——金库键的那句文本，不是路由再拼一次的一句；值不回声、不入事件载荷。入金库由 `Sealed::into_vault_value`（住 kernel::secret，即 expose 白名单三文件之一）完成，开封因此**不发生在装配层**。

**线上没有登录命令**：订阅额度由厂商自己的 harness 带进城，人在 harness 里自己登录，本城不以任何厂商客户端的身份登录（`crates/gateway/Spec.lean` §8-5）。`WireCommand` 没有登录命令，`COMMAND_NAMES` 里也没有。

**五个查询各有自己的答**（`InboxView`／`DiscardView`／`RegistryView`／`ArchiveSearch`／`Metrics`）。三条口径：①**队列折叠着看不消费着看**（`Inbox::pull` 要拿走才给内容，看一眼就取走的视图会改变它所报告的对象）；②归档在被问的那一刻读盘（同 `BuildingView`，文件是权威）；③**`Metrics` 恒不携钱**——钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。

**`DiscardLine.restoration` 携 `Option<Restoration>` 而非一个句子**。回收站那一行的「怎么拿回来」的唯一措辞处在客户端（`client/src/views/record/bin.svelte` 按 `tracked`／`interred`／`rebuildable` 三臂措辞）；服务端若把它拼成一句话，就是**一件事两个渲染权威**，而服务端那句还拼不出可执行的那句话。所以计划以它自己的形状上线（载荷本来就是 `Restoration` 序列化出来的，故读得回去）；`None` 的意思是**这一条记录用了本构建读不懂的方案**，界面据此画一行而不给动作——行恒不隐藏，因为藏起一件被删的东西比承认读不懂它的方案更糟。这类「语法换形而名字没换」的改动不动 `QUERY_NAMES` 与 `COMMAND_NAMES`，故只能由 `WIRE_V` 进位让旧页面在握手期被拒。

**`POST /acp` 与 `AcpSink`**。外来编辑器的请求走自己的路由，不挤 Command 面：它自带鉴权、要一个当场的回答，而 Command 面的回答是事件流。三条口径：①**令牌在本 crate 判**（配对令牌住这里，常数时间比对也就住这里），只把 `authentic` 一位传进去——拒词由 `agent_protocols::admit` 措辞，「未配对者只学到一位」因此只有一个权威；②回给编辑器的只有 `AcpProgress` 三字段，run id 是工人接单时才铸的，故受理那一刻诚实的答案是「已受理、未完成」；③编辑器出示本机钥匙（读钥匙文件，`crates/sprawling/spec/Keying.lean` §8-22）或一个会话令牌，与 control surface 同一条规矩。

**三帧登记面**（§8-1 golden 同集更新）——`AttachEndpoint`（人刚输入的 URL＋兼容格式＋`secret:` 引用；**引用有字节形，凭证没有**）、`SelectModel`（标签→模型＋两个探不到的 token 数＋人说的「收得下什么」；输出上限是 `Option<Ceiling>`，缺席即「没人登记过」，零在类型上不存在；`input: Option<kernel::InputKinds>` 紧接在 `max_output_tokens` 之后，出现时是 `gateway::accepted_input` 的第一档，缺席时梯子从目录开始，`crates/gateway/Spec.lean` §8-37、gateway D16）、`EndpointView`（设置页的读；`EndpointsAnswer` 里 `has_credential` 是关于凭证能回答的全部）。

**三个 kernel 类型的再导出**（`DialectKind`／`Effort`／`ModelTag`）。客户端只读 `wire` 的 schema，而设置页要拼写这三个词；再导出而非镜像定义，因为镜像就是同一规则的第二个权威——同 §8-0 对 `Mode` 的口径。
-/

/-! D47 `Answer::Unavailable` 带上没看成的原因

```rust
Unavailable {
    query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,   // 读面遇到的错误的文字（`kernel::AxError` 的 Display）；缺席即只报名字
}
```

**决定**：一个视图去读账本、盘或子进程而没有读成时，答 `Unavailable` 并把它遇到的错误的文字放进 `reason`；不去读的视图（这份构建不求值这个查询）不带 `reason`。第一个带原因的读面是被热视图逐出的 run 的回读（`LedgerAsk::recalled`、`recalled_bill`，accounting `views::answering::history`）：它们返回 `Result<_, AxError>`，答 `Run`、`CostOf`、`Commit` 的视图把错误映射进 `reason`。其余以 `views::prepared::unavailable` 作答的读面先保持没有 `reason`，各自改成带原因时只改它自己那一处。

**理由**：只有 `query` 时，「账本这一行读不回」与「这份构建不答这个查询」在线上长得一样，读面只好把错误丢掉（`.ok()?`）；而页面和读日志的人要据此决定是等一会儿、修盘，还是换一份构建。

**被否**：①`reason` 必填：四十多处不读的视图只能填一句「不求值」，那是 `query` 已经说过的话；②一个原因的枚举：错误本身已经带稳定的码与恢复（`AxError`），线上再列一份码表就是第二个权威；③为读不成另开一个 `Answer` 变体：页面对两者都画「城没能回答」，区别只是要不要多给一句原因。

**三个平台**：相同；`reason` 里的路径按所在平台拼写。

与 D48 同一次 `WIRE_V` 进位。
-/

/-! D20 送页面的两条路由是一个公开函数，城的端口与远程监听各把它并进自己的路由表

**决定**：`/` 与 `/{*asset}` 两条路由连同它们的响应头由 `bundle_routes` 造出，对路由表的状态类型泛型，自带 `Arc<ClientAssets>` 作状态；`router` 把它并进来，装配层的远程监听也把它并进来。

**理由**：设备从二维码打开的是远程监听的地址，那里要答出与城的端口同一份页面（`crates/remote_access/Spec.lean` §8-10）。哪个路径答哪些字节已经只在 `ClientAssets::lookup` 一处判；路由的拼写与「gzip 的字节要带 `Content-Encoding`」这两件事若在装配层再写一份，两个端口送页面的方式就有两个家，一边加了一个响应头，另一边不会知道。

**被否**：①远程监听把非升级的 `GET` 反向代理到城的端口——多一跳 HTTP 客户端，且城的端口在局域网面上要配对令牌，中继还得替设备出示它；②`wire` 只公开 `asset_response`，装配层自己拼两条路由——路径的拼写成了两份。

**重开参数**：两个端口送页面的方式需要不同（例如远程地址要加一条只对外的 CSP 或缓存头）时，在这里加一个参数，而不是在装配层另写一张表。
-/

/-!
### 8-15 `/enroll` 的三结局测试进程内驱动（`tests/enrolment.rs`）

`tests/enrolment.rs` 检验的是 §8-2 的三选一（`secret_captured` 相符→201／`Reply` 拒绝→422／有界等待到期→202），三者由测试替身的工人（存／拒／沉默）分出——这是白盒问题：真二进制上 vault 只有一种下场，且 `serve` 经 `Custodian::probe` 写平台凭据服务、线格式无收回凭据的动词，黑盒重写既不可判也不可回收。故它进程内驱动：`wire::router(&config, face).layer(MockConnectInfo(peer))` 后 `tower::ServiceExt::oneshot` 一发一收，peer 以 axum 给测试的那条路供给，不起 socket、不写字节，`xtask boundary` 因此不把它算作从外面进来的检查。`tower`（`util`）只作 dev-dependency，已在 axum 之下的依赖图里，锁文件不增包。
-/

/-!
### 8-22 没有监听器的那份构建，测试也不许提它（`tests/enrolment.rs`、`tests/wire_contract.rs`）

`cargo clippy -p sprawling-wire --no-default-features --all-targets` 要绿：`tests/enrolment.rs` 整份都在驱动 `wire::router`，`tests/wire_contract.rs` 有三条断言在问 `decide_bind`／`decide_handshake`，而这三样连同 `axum`、`tokio` 都由 feature `server` 带进来。**这份构建正是 `accounting` 用的那一份**（`crates/accounting/Cargo.toml` 依赖本 crate 而不开 `server`，D2）——它需要本 crate 的词汇而不要 TCP 栈；一个在这里名词都拼不出来的测试文件，把它自己的红判在了产品的一条真路径上。

**按测试真正需要的东西设门，而不是把 feature 打开**：`tests/enrolment.rs` 首行 `#![cfg(feature = "server")]`（整份文件都是路由的事）；`tests/wire_contract.rs` 只给那三条断言与它们的两个辅助函数、以及 `Hello`／`Welcome`／`AxCode`／`SocketAddr` 这几个只被它们用到的名字加 `#[cfg(feature = "server")]`——命令表、查询表、schema 哈希与那两个不可拼写的形状**在两份构建里都被判**，因为它们在两份构建里都成立。
-/

/-!
### 8-27 说出来的那句话：`/transcribe` 与 `ModelTag::Transcribe`

```rust
pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;
// POST /transcribe，body 是录音字节，content-type 是浏览器录进的容器；200 的 body 就是那行文字。
```

- **是一条路由，不是一条 Command，也不是一条 Query**。Command 被接下之后经事件流作答，而「我刚说的那句话是什么」必须回到录它的那个标签页；Query 是另一种会作答的形状，而一条在供应方那里花掉数秒的查询就是一条装成读的命令。`/enroll` 与 `/drop` 是同一类旁门：帧的文法装不下的那几件事各有一扇 HTTP 门。
- **容器从请求头读，不从字节猜**：浏览器录进它手上有的容器，而只有它知道是哪一个。没有 content-type 即按名拒绝——一个没人声明的容器发不出去。`; codecs=opus` 这类参数说的是容器里的编解码器，而音频线路由的是容器，故取分号前那一段；那一段修剪后为空（头里只有参数）同样是没声明容器，与缺头同一句拒绝。
- **`ModelTag` 增第三个 `Transcribe`**（`crates/kernel/Spec.lean` §8-24 的枚举同步）：**「哪个 endpoint、哪个 model 答这一类活」本来就有机制**——人登记一个 endpoint，再为一个 tag 选一个 model。第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。人填 URL 与 key 因而走的是既有的 attach 表单。
- **服务端**：`gateway::transcriber_for(chosen, secrets)` 与 `adapter_for` 同形——把一个选择变成一件可调用的东西这件事只在一处发生。`Views::transcriber` 在锁内读出选择、锁外发请求。
-/

/-!
### 8-49 拖进对话框的文件：`/drop`

```rust
pub type DropSink = Arc<dyn Fn(&str, &[u8]) -> Result<String, AxError> + Send + Sync>;
pub const DROP_BYTES_MAX: usize = 64 * 1024 * 1024;
// POST /drop?name=<百分号编码的文件名>，body 是文件的字节；200 的 body 是城存下它的绝对路径。
pub enum Door { Transcribe, Enroll, Acp, Drop }
```

- **是一条路由，不是一条 Command**：与 `/transcribe` 同一个理由，字节不进帧的文法，而答案（那条路径）必须回到拖文件的那个标签页。
- **文件名走查询参数，不走请求头**：请求头的值只能可靠地携带 ASCII，而人的文件名常常不是。缺 `name` 即 422。
- **`Door::Drop` 未配对即拒**：它往城的磁盘上写字节，与另外两扇会动作的门同一个判定（8-40）。
- **正文上限 `DROP_BYTES_MAX`（64 MiB），只加在这一条路由上**：axum 的缺省上限是 2 MiB，一张截图或一份 PDF 就会超过；更大的正文答 413。上限不放宽到其余路由，因为它们收的是一行文字或一份录音。
- 城怎么存、存在哪里、答出哪条路径是 `crates/sprawling/Spec.lean` §8-119 的事；这里只把名字与字节交进去，把答案或拒绝原样交回来（拒绝是 422 加 `refusal_text`）。
-/

/-!
### 8-93 本地门的配对与会话：三条路由的形状

```rust
// POST /pair                  { "open": "<开页码>" | "code": "<配对码>", "public_key": "<设备公钥>", "label": "<名字>" }
pub struct PairBody { #[serde(flatten)] pub proof: PairProof, pub public_key: String, pub label: String }
pub enum PairProof { Open(String), Code(String) }            // 线上 snake_case：二者恰有其一
pub struct PairAnswer { pub device: DeviceId }               // §8-91

// POST /session/challenge     无请求体
pub struct ChallengeAnswer { pub nonce: String }

// POST /session               { "device", "nonce", "signature" }
pub struct SessionBody { pub device: DeviceId, pub nonce: String, pub signature: String }
pub struct SessionAnswer { pub token: String }               // 页面只把它放在内存里
```

- **会话令牌是凭据**：页面以 hello 的 `token` 或 POST 的 `Authorization: Bearer` 出示它；`/ws`、`/transcribe`、`/enroll`、`/drop`、`/acp` 对每一个调用方都要求一份凭据——浏览器的会话令牌，或同一台机器上的原生客户端的本机钥匙（§8-40）。
- **两种码，一个入口**：`open` 是 `/web` 经只给本用户读的跳转文件交给浏览器的开页码，`code` 是终端上显示的配对码；错、用过或太早一律 `E_PAIRING_REFUSED`，403。状态与它的规则在 `spec/Reception/Pairing.lean` §8-95。
- **字节怎么写**：`public_key` 是 Ed25519 公钥的 32 字节，`nonce` 是 32 字节，`signature` 是 64 字节，`token` 是 32 字节，线上一律写成小写十六进制；`label` 至多 `LABEL_MAX`（64）个字符，空或含控制字符即 422。页面用 `crypto.subtle.exportKey("raw", publicKey)` 得到公钥。签的字节是 `sprawling local session v1
<nonce>
<origin>`，`origin` 是页面自己的 `location.origin`，服务端取这次请求过了入口判定的 Origin 头去验。
- **答复**：`/pair` 200 `{ device }`；`/session/challenge` 200 `{ nonce }`；`/session` 200 `{ token }`，nonce 过期或用过、设备不认识、签名不对都是 403 `E_GATE_DENIED`。三条路只收浏览器（§8-94 第 6 步），不问凭据——它们就是换凭据的地方。

```rust
// 本机这扇门的句柄：装配层建一次，交给 ServeConfig，也交给控制台与开浏览器的那一方。
pub struct LocalDoor { … }                                        // Clone，内部一把锁
pub struct DoorSenses { pub clock: Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>,
                        pub entropy: Arc<dyn Fn(&mut [u8]) -> Result<(), AxError> + Send + Sync> }
pub type KeepBrowsers = Arc<dyn Fn(&[PairedBrowser]) -> Result<(), AxError> + Send + Sync>;
impl LocalDoor {
    pub fn new(browsers: Vec<PairedBrowser>, senses: DoorSenses, keep: KeepBrowsers) -> Result<Self, AxError>;
    pub fn pairing_code(&self) -> tokio::sync::watch::Receiver<String>;   // 换码时推一次，控制台据此原地刷新
    pub fn issue_open_code(&self) -> Result<OpenCode, AxError>;           // OpenCode { code, redeemed: mpsc::Receiver<()> }
    pub fn devices(&self) -> DevicesAnswer;
    pub fn forget(&self, device: &DeviceId) -> Result<bool, AxError>;
}
```

- **熵与时间都是入参**：`LocalDoor` 不采样，装配层（`bin::assembly`）交进来的 `DoorSenses` 是它唯一的时钟与随机源，所以状态机（§8-95）可以在测试里逐步驱动。
- **设备表的落盘是一个闭包**：配对与忘掉之后，整张表交给 `keep`；装配层把它写进城的 `.sprawling/browsers.toml`。wire 不知道城目录在哪。写失败时这次配对答 500 并保持未登记，因为一把重启就丢的设备钥会让人以为配上了。
- **兑掉的开页码通知交出它的人**：`OpenCode.redeemed` 在兑掉时收到一次，交出它的那一方据此删跳转文件；过期没兑也删（`OPEN_CODE_LIFETIME_MS`）。
-/
