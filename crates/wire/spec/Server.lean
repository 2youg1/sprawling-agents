-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::server

规定 `server`、`server::config`、`server::config::enrolment`、`server::socket`、`server::bundle`、`server::uploads`、`assets`、`answer::cost`（`crates/wire/src/` 下同名的文件）。监听的一端：判定是纯函数，套接字一个也不做；客户端资产、几扇 HTTP 门与 Query 的答面。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-2 wire::server（形状 4 薄壳）＋reception（形状 1）＋assets（形状 4）

**Humble Object 在此的切法**（ARCHITECTURE §7 末段，理由只写一次）：难测的一端（tokio＋axum 监听）剥到最薄，厄的一端（绑定面判定、握手判定）是纯函数，无需跑服务即可穷尽测。

**切法写在这里，而两半一直在同一个文件里。** 模块表给 `server` 的形状是 `adapter`——ARCHITECTURE §9 定义为「薄、无策略：换第二个实现不改变任何策略」——而四个判定函数就是策略。于是它们搬进 `wire::reception`（绑定／录凭证／握手／帧），客户端资产搬进 `wire::assets`，`server` 剩下的每一条分支不是一次发送、一次接收，就是一次会话的结束。三个文件 1,235 → 649＋385＋271。
**公开名一字未改**（`lib.rs` 重导出）；变的只有 `cargo public-api` 记的**定义模块**，所以 `sprawling` 基线里 `Serving::client` 的类型路径从 `wire::server::ClientAssets` 变成 `wire::assets::ClientAssets`，两份 SPEC 同变更集各记一行。

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
pub struct Bound { /* 已绑定的监听器与它的 BindFace —— 私有 */ }
pub async fn bind(addr: SocketAddr, token_digest: Option<B3Hash>) -> Result<Bound, AxError>;
pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError>;

pub struct ServeConfig {
    pub client: Arc<ClientAssets>,      // 客户端资产源由装配层递入
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
```

**客户端是一张资产表，不是一个文件**：`ServeConfig` 携 `client: Arc<ClientAssets>`，因为 `client/` 的构建产物是 `index.html` 加它引用的脚本、样式与字体，只携一个文件的形状会让页面壳引用一条服务端没有的路由。资产表是封闭清单：路径穿越（`..`、空段、盘符、点头文件）在判定层拒，miss 报文件名并给出重建口令。`Disk` 臂逐请求读盘，专供开发回路（改前端刷新即见），发布路径恒不构造它。

**公开签名不携传输层的类型**：sink 收 `Vec<u8>` 而不是 `axum::body::Bytes`。**一个泄露自己传输层的公开签名，会把「换掉 HTTP 库」变成对每一个从未选过它的调用方的破坏性变更**。

**令牌只以摘要形式进入本 crate**：拿令牌的一方自己摘一次，边界只比摘要；`expose` 只得出现在兑付点（`xtask secret`）。代价为零（常数时间比较本来就要先摘），收益是 `wire` 在类型上根本拿不到配对令牌的明文。常数时间比较因此退化为定长 32 字节的无早退异或，**既无内容侧道也无长度侧道**。

`decide_bind` 的四格真值表是全部行为：回环×无令牌＝`Serve(Loopback)`；回环×有令牌＝`Serve(Loopback)`；非回环×有令牌＝`Serve(Exposed)`；**非回环×无令牌＝`Refuse(E_CONFIG_INVALID)`**。拒绝发生在**启动时**，不是启动后拒连——它是配置判定。

薄壳的职责恒为三件：静态资源（前端产物）｜WS 升级｜几条 HTTP 路由（`/enroll`、`/transcribe`、`/acp`）。它不持业务状态，不做策略判断。

**WS 路由与两条沿途缝**。升级后的会话只做三件事：先收 `Hello` 并交 `decide_handshake` 判（拒即关，不降级）；收到 `ClientFrame::Command` 交给 sink；把订阅到的 `EventRecord` 以 `ServerFrame::Event` 推给客户端。

```rust
pub struct ServeConfig {
    /* …前四项不变… */
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

**Query 的答面**。`ServeConfig.queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>`，同步；`ServerFrame` 增 `Answer(Box<Answer>)` 变体。答面类型住 `answer`：`Answer`（City／Run／Approvals／Cost／Unavailable）、`RunSummary`、`CityAnswer`、`ApprovalsAnswer`、`CostAnswer`。

**`Query::BuildingView { addr }` → `Answer::Building(Box<BuildingAnswer>)`**（`BuildingDoc`／`ArchiveLine` 随之入 wire）。楼里的文件是楼的记忆，服务端在被问的那一刻读盘——**文件是权威**，另存一份索引就是第二个权威。`QUERY_NAMES` 因此从 10 增到 11，schema 哈希随之从 `238f11b2…` 变为 `85705c03…`：客户端与服务端同批发布，旧页面会在握手期被明确拒绝并提示刷新。

**`Welcome` 携 `city: Option<Address>`，`decide_frame` 增一个 `city` 入参。** 事件流只送连接之后发生的事，而城市的名字写在 Ledger 的第一条记录里——一个今天打开的浏览器永远等不到它。握手是「这是哪座城」的自然回答处；服务端从同一条创世记录读它，故两边不构成第二个权威。同批：`init` 把城市名写进创世记录的 `addr`（此前是 `None`，城市名只活在目录项里）。

**`ApprovalsAnswer.items` 携 `kernel::ApprovalItem` 全项，`ApprovalSummary` 删除。** 旧摘要类型丢掉了 `cluster_key` 与 `created`，于是界面无法按类聚合、也排不出「谁等得最久」；服务端为了填它还要从事件载荷里猜一个 `summary` 字段——那个字段从来没被写过，故每一条待批项都渲染成「(no summary recorded)」。载荷本身就是 `ApprovalItem` 的序列化，原样送过去既少一次有损转换，也让「什么算一类」只有 `web::approval::inbox` 一处答案。

- **为什么是强类型答面而不是一团 `Payload`**：`web` 只依赖本 crate，故发帧的边界 crate 欠对方一套读帧的词汇（同 kernel 再导出的理由）。一个无类型载荷会把解析责任推给每一个视图模块，每一个都得自己猜一遍形状。
- **`Answer::Unavailable { query }` 是一个真答案**：不求值的视图报自己的名字，而不是返回空结果——空城与未实现在界面上必须长得不一样。
- **`CityAnswer.buildings: Vec<BuildingProgress>`**：每栋楼一行，携 `Progress` 与 `problems`。解析不出的行进 `problems` 并照显——悄悄丢掉读不懂的行，等于按一个没人选过的分母报进度。
- **五维成本携权威总额**：`CostAnswer.total` 与 actor、segment、tool、skill 四个维度各自求和相等；`by_run` 只带活跃的跑与花得最多的前几个（`crates/sprawling/Spec.lean` §8-90），和可以小于 `total`。界面按 `total` 算占比而不自己归一，未归因余额与列表之外的跑因此都看得见。
- **无报价的调用单独报数**：`CostAnswer.unpriced: UnpricedCalls { calls, tokens }` 是账本上没有权威计费额的模型调用次数与它们的 token 总数（`storage::Attribution` 的 `unpriced` 原样上线）。它们不进 `total`，所以缺了这一项，一座只用订阅登录或本地模型的城跑了多少次都读作「没花钱」；界面据 `calls > 0` 说「有调用没有报价」并给出 token 数，而不是把 `$0.00` 当作量出来的数。

- **採用 `broadcast` 而非每连接一个队列**：多个标签页是常态；慢客户端被拉下而不拖住写入方。**丢下的那一段不再静默**：事件流慢过城的会话收到 `ServerFrame::Lagged { from, to }`，按这个区间向账本补拉（§8-41）。三路语义不同，故这三节分开陈述：事件可补、增量与日志恒不可补、会话自己的拒绝根本不走广播。

**`POST /enroll`，唯一携凭证字节的路由**

它是 HTTP 而非 socket 帧，因为 socket 的 `WireCommand` **拼不出** `PutSecret`（`NoSecret` 无值）。两半合起来才是完整保证：类型层管住帧，`decide_enroll` 管住字节——因为字节总可以被 POST 到一个路由上。

- **只认回环对端，配对令牌也不算数**：令牌认的是人，而这条规则管的是**字节走到哪里**。拒绝的第三段指向宙主机，于是它是约束而非死路。
- 壳里零策略：判定在 `decide_enroll`，壳只搬字节——同 `decide_bind`／`decide_frame` 的切法，故无需跑服务即可穷尽测。
- 应答返回那条 `secret_captured` 记录写下的 `ref`——金库键的那句文本，不是路由再拼一次的一句；值不回声、不入事件载荷。入金库由 `Sealed::into_vault_value`（住 kernel::secret，即 expose 白名单三文件之一）完成，开封因此**不发生在装配层**。

**线上没有登录命令**：订阅额度由厂商自己的 harness 带进城，人在 harness 里自己登录，本城不以任何厂商客户端的身份登录（`crates/gateway/Spec.lean` §8-5）。旧版本的 `Login` 命令与 `LoginStep` 随之删去；`COMMAND_NAMES` 少一项，schema 哈希因名字表而变，旧页面在握手期被明确拒绝，所以 `WIRE_V` 不为此进位。

**五个查询各有自己的答**（`InboxView`／`DiscardView`／`RegistryView`／`ArchiveSearch`／`Metrics`）。三条口径：①**队列折叠着看不消费着看**（`Inbox::pull` 要拿走才给内容，看一眼就取走的视图会改变它所报告的对象）；②归档在被问的那一刻读盘（同 `BuildingView`，文件是权威）；③**`Metrics` 恒不携钱**——钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。

**`DiscardLine.restoration` 携 `Option<Restoration>` 而非一个句子，WIRE_V 3→4**。回收站那一行的「怎么拿回来」原本在服务端被拼成 `"tracked: file:…"`，而客户端早已持有它的唯一措辞处（`web::approval::ReturnPath::sentence`）——**一件事两个渲染权威**，而服务端那个还拼不出可执行的那句话。现在计划以它自己的形状上线（载荷本来就是 `Restoration` 序列化出来的，故读得回去）；`None` 的意思是**这一条记录用了本构建读不懂的方案**，界面据此画一行而不给动作（`ReturnPath::Undescribed`）——行恒不隐藏，因为藏起一件被删的东西比承认读不懂它的方案更糟。`QUERY_NAMES` 与 `COMMAND_NAMES` 未动，故哈希只因 `WIRE_V` 而变——又一例「语法换形而名字没换」。

**`POST /acp` 与 `AcpSink`**。外来编辑器的请求走自己的路由，不挤 Command 面：它自带鉴权、要一个当场的回答，而 Command 面的回答是事件流。三条口径：①**令牌在本 crate 判**（配对令牌住这里，常数时间比对也就住这里），只把 `authentic` 一位传进去——拒词由 `agent_protocols::admit` 措辞，「未配对者只学到一位」因此只有一个权威；②回给编辑器的只有 `AcpProgress` 三字段，run id 是工人接单时才铸的，故受理那一刻诚实的答案是「已受理、未完成」；③没配对令牌的城即回环独占，与 control surface 同一条规矩。

**三帧登记面**（§8-1 golden 同集更新）——`AttachEndpoint`（人刚输入的 URL＋兼容格式＋`secret:` 引用；**引用有字节形，凭证没有**）、`SelectModel`（标签→模型＋两个探不到的 token 数＋人说的「收得下什么」；输出上限是 `Option<Ceiling>`，缺席即「没人登记过」，零在类型上不存在；`input: Option<kernel::InputKinds>` 紧接在 `max_output_tokens` 之后，出现时是 `gateway::accepted_input` 的第一档，缺席时梯子从目录开始，`crates/gateway/Spec.lean` §8-37、gateway D16）、`EndpointView`（设置页的读；`EndpointsAnswer` 里 `has_credential` 是关于凭证能回答的全部）。

**三个 kernel 类型的再导出**（`DialectKind`／`Effort`／`ModelTag`）。`web` 只依赖 `wire`（拓扑图），而设置页要拼写这三个词；再导出而非镜像定义，因为镜像就是同一规则的第二个权威——同 §8-0 对 `Mode` 的口径。
-/

/-!
### 8-15 `/enroll` 的三结局测试进程内驱动（`tests/enrolment.rs`）

`tests/enrolment.rs` 原以 `axum::serve` 端起本 crate 的路由、手写 HTTP 字节去问它，因而是 `xtask boundary` 在册的唯一越线文件。它检验的是 §8-2 的三选一（`secret_captured` 相符→201／`Reply` 拒绝→422／有界等待到期→202），三者由测试替身的工人（存／拒／沉默）分出——这是白盒问题：真二进制上 vault 只有一种下场，且 `serve` 经 `Custodian::probe` 写平台凭据服务、线格式无收回凭据的动词，黑盒重写既不可判也不可回收。故改为进程内驱动：`wire::router(&config).layer(MockConnectInfo(peer))` 后 `tower::ServiceExt::oneshot` 一发一收，peer 以 axum 给测试的那条路供给，不起 socket、不写字节。三断言原文不动；`[boundary.predating]` 归空。`tower`（`util`）只作 dev-dependency，已在 axum 之下的依赖图里，锁文件不增包。
-/

/-!
### 8-22 没有监听器的那份构建，测试也不许提它（`tests/enrolment.rs`、`tests/wire_contract.rs`）

`cargo clippy -p sprawling-wire --no-default-features --all-targets` 是红的：`tests/enrolment.rs` 整份都在驱动 `wire::router`，`tests/wire_contract.rs` 有三条断言在问 `decide_bind`／`decide_handshake`，而这三样连同 `axum`、`tokio` 都由 feature `server` 带进来。**这份构建正是给 `web` 用的那一份**——它需要本 crate 的词汇而不许把 TCP 栈拖进 WebAssembly；一个在这里名词都拼不出来的测试文件，把它自己的红判在了产品的一条真路径上。

**按测试真正需要的东西设门，而不是把 feature 打开**：`tests/enrolment.rs` 首行 `#![cfg(feature = "server")]`（整份文件都是路由的事）；`tests/wire_contract.rs` 只给那三条断言与它们的两个辅助函数、以及 `Hello`／`Welcome`／`AxCode`／`SocketAddr` 这几个只被它们用到的名字加 `#[cfg(feature = "server")]`——命令表、查询表、schema 哈希与那两个不可拼写的形状**在两份构建里都被判**，因为它们在两份构建里都成立。
-/

/-!
### 8-27 说出来的那句话：`/transcribe` 与 `ModelTag::Transcribe`

```rust
pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;
// POST /transcribe，body 是录音字节，content-type 是浏览器录进的容器；200 的 body 就是那行文字。
```

- **是一条路由，不是一条 Command，也不是一条 Query**。Command 被接下之后经事件流作答，而「我刚说的那句话是什么」必须回到录它的那个标签页；Query 是另一种会作答的形状，而一条在供应方那里花掉数秒的查询就是一条装成读的命令。`/enroll` 与 `/upload` 早已是同一类旁门：帧的文法装不下的那几件事各有一扇 HTTP 门。
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
