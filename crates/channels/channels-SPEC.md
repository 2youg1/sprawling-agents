# channels-SPEC.md

> crate：`channels`（lib，依赖 kernel）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：十七节；按模块分章、每章自足。
> 五模块：wire／server／control／auth／aggregate。
> 本 crate 覆盖的语义：wire 面（Command／Query／Event、编码与握手、绑定面）、干预五动词、多台机器一个界面、Autonomy 应答者、三队列。

## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `wire` | Command 17／Query 9／Event 三分的类型与 JSON 编码；版本＋schema 哈希握手帧 |
| `server` | WebSocket 服务；静态资源与上传端点；绑定面判定（默认只绑回环） |
| `auth` | 回环零摩擦；非回环要求配对令牌且常数时间比较；未配置令牌即拒绝启动 |
| `control` | 人的五动词入口；自持鉴权与幂等（做不到则并入 `server`——ARCHITECTURE §6 已写明这条退路） |
| `aggregate` | 多 City 只读聚合：只转发 Query 与 Event，恒不转发 Command |

**本 crate 是进程外边界的唯一守卫**。它不实现任何业务判定：Command 的执行、Query 的求值、Event 的产生全在上游（runtime／memory／city），本 crate 只负责「让非法的帧在类型层或握手层就不存在」。

## 2 验收标准

- **wire**：Command 恰 24 个 variant、Query 恰 24 个（计数断言，对本 SPEC §8-1 两表逐名核对）；每个改状态 Command 携 `IdemKey`（类型强制，无可省字段）；`PutSecret` 的 `value: Sealed<String>` 不实现 `Serialize`——**「远程录凭证」这条帧编译不出来**，以 trybuild 反例钉死。
- **握手**：版本＋schema 哈希不配即断连并回 `E_WIRE_MISMATCH`（装载期码，无 carrier）；schema 哈希由 wire 类型集派生，改一个 variant 即变。golden 钉住当前哈希，改哈希必须与本 SPEC 同集变更。
  **当前 golden**：`2dfe19d4ec612563bf5a79958d7c06e6b77b95d44d886697f6b9c77719ac28be`；**WIRE_V ＝ 28**（帧表与查询表的当前内容见本节以下各章；端点带 `EndpointTuning` 见 §8-29；工具服务器的三种 transport 与 `McpHealth` 见 §8-34；日志帧 `ServerFrame::Log` → §8-32；机器上的两个动词 `DoctorInstall`／`DoctorRefresh` → §8-33）。
  `PutSecret` 无线格式——它经 `/enroll` 路由在进程内成形，见 §8-2 录入口。

**`Query::RunHistory { run, before, limit }` → `Answer::History`，WIRE_V 9→10。**

补的是一个**页面成立的前提**而不是一项便利：`Query::History` 不带 run 过滤，客户端在连上时问一次城全局的最后 500 条，然后在本地按 run 过滤。于是四个会话分这 500 条，而**昨天的会话根本不在里面——打开它是一张空白页**。`Query::RunView` 只答 5 个字段，它回答「这个 run 在不在、走到哪」，不回答「这个会话是什么」。

**答面复用 `HistoryAnswer` 而不新增一个。** 它已经带 `earlier` 游标，形状正是「往回翻」；再造一个只会让「一页历史长什么样」有两个答案。

**扫描量没有第二个上限。** `memory::index` 持了 run 索引之后，一次 `RunHistory` 只取属于这个 run 的 seq，**扫描量与 `limit` 同阶**，而 `limit` 已由 `HISTORY_MAX` 封顶。再设一个不防任何事的上限，只会让下一个读代码的人以为它还在防什么。

**`earlier` 的含义**：「从这条之前接着问」，`None` 即「到头了」；「答是空的而 `earlier` 是 `Some`」这一态**不存在**——服务端不需要把「我这一段没扫到」告诉客户端。

**转出 `kernel::{Span, Token, markdown}`，线格式未动。** 客户端只看得见
本 crate，而文档的词法器在 kernel（无 I/O、无时钟、输出穷举枚）。转出一个纯函数而不是
新增一个 Query：分词在浏览器里跑就行，为一个几 KB 的词法器先把线格式撑大，
是替尚不存在的第二实现造抽象。真需要语法引擎时它去服务端，线格式那时再长。

**建 run→seq 的索引，理由是两组实测数字。** 其一，一次 `RunHistory` 在 5 万条账本上实测为 **2823 ms**，索引之后压到 **22.4 ms**，而这是「打开昨天的会话」这个动作的全部延迟。其二，那份要随账本同步的派生状态已经存在：`memory::LedgerIndex` 常驻于 `Views` 并每次查询 `refresh`，run 表只是它多一个字段，搭同一趟刷新、同一份 cache、同一条「存疑即重建」的反射，不新增同步义务。至于「第二个权威」：索引回答的是「在哪」，从不回答「是什么」，它可弃且存疑即重建；账本仍是唯一权威。接面与内存代价见 memory-SPEC §8-4。
- **server**：默认绑定回环；绑非回环且 `auth` 未配置令牌时**拒绝启动**并回 `E_CONFIG_INVALID`（不是启动后再拒连——这是绑定面判定，不是请求面判定）。
- **auth**：令牌比较恒为常数时间（不早退）；比较函数以「逐字节差异位置不影响耗时」的性质测试看守。地基是 `server::constant_time_eq` 与 `decide_handshake`；`auth` 模块接令牌的生成、展示与持久化。
- **aggregate**：**类型化保证**——聚合上游连接的发送面在类型上只接受 `Query`，没有一个能塞进 `Command` 的方法（不是运行时 `if`，是类型上不存在该入口）；以 trybuild 反例钉死。
- **上传端点**：`Attach` 的字节走 HTTP，不走 WebSocket 帧；命令语义不变（明写这是传输细节）。

## 3 假设与歧义

- **本 crate 是全库首个引 tokio 的地方**。gateway 不引 tokio，endpoint 用 `reqwest::blocking`，tokio 行推迟到 channels——首个真异步消费者（gateway-SPEC §3／§13）。引入 tokio **须携 `Verdict:` 尾注**（触碰根 `Cargo.toml`，guard 辖区）。
- **没有第二条传输路径**：即使浏览器与服务在同一台机器上也是网络连接，故不存在「同进程内存通道」这条优惠。唯一例外是 `PutSecret`——它不是靠运行时判断走内存通道，而是**类型上不可序列化**，因此远程连接根本编不出这条帧。
- **编码是 JSON，且编码本身不进冻结面**。选 JSON 的理由是不对称：浏览器原生支持，且开发者能在网络面板直接读帧。
- **`control` 自持鉴权与幂等，独立成模块**。ARCHITECTURE §6 预留了「做不到则并入 server」的退路，这里不需要它：`control` 持有一条 `server` 不知道也不该知道的策略——**哪些 Command 是干预，以及一次干预必须留下什么**（「任何中断都以 Handoff 收尾，下一位拿得到完整现场」）。那是判定，不是转调。
- **`auth` 收回了一块放错位置的逻辑**：常数时间比较曾写在 `server` 里，那是因为 `auth` 尚未建。令牌的**整个生命周期**（铸造、展示形、摘要、比对）收进 `auth`，`server::decide_handshake` 改为调用它。这不是重构的赔罪，是模块建成后把属于它的东西放回去。
- **Signal 不在 Command 面**：`Attach{notify}` 产 Signal，但 Signal 的投递与消费住 `collab::inbox`。channels 只是产地。

## 4 现状分析

空壳 lib（`src/lib.rs` 仅 crate 文档）。无既有公开面，api-baseline 从零起算。下游唯一消费者是 `web`（ARCHITECTURE §2 depmap：`web: channels`），装配消费者是 `crates/sprawling`（bin `serve`）。

## 5 权威信源

wire 面全节（Command 表、Query 表、编码与握手、绑定面三段）；聚合层硬约束「聚合层只转发 Query 与 Event，恒不转发 Command」；五动词语义表；`Sealed<T>` 的不可序列化性质；`kernel::error` 的装载期五码白名单（kernel-SPEC §8-1，封闭且不得增长）。外部：axum 0.8.9（2026-04-14，内含 tokio-tungstenite 0.29）与 tokio。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。

Command／Query／Event（三分的原名，不译）；Dispatch／Login／Fork／Attach／CreateBuilding／PutSecret／Steer／Cancel／Takeover／Rollback／Halt／Release／BatchByBuilding／Approve／CreatePolicy／SetAutonomy／Auth／Wake（命令原名，逐字取本 SPEC §8-1 表）；RunView／CityView／ApprovalQueue／InboxView／Metrics／CostView／ArchiveSearch／RegistryView／DiscardView（9 查询原名）；control surface（不译）；配对令牌＝pairing token；握手＝handshake。

## 7 模块边界

```
                    ┌── wire（类型与编码；无 I/O，纯数据与纯函数）
server（tokio＋axum）┤
  ├ WS 端点         ├── auth（令牌判定；常数时间比较）
  ├ 静态资源         └── control（五动词入口；鉴权与幂等）
  └ 上传端点（HTTP）
aggregate ──▶ 上游 City 的 WS 连接（发送面类型上只收 Query）
```

**不做什么**：不执行 Command（只解码并交给装配层注入的处理器）；不求值 Query（同上）；不产 Event（Event 载荷即 `EventRecord`，产地在各效果模块）；不做业务鉴权之外的策略；不声明 `pub` trait（不在 ARCHITECTURE §3 缝清单内——**处理器以函数指针或具体类型注入，不是端口**）；不内置任何穿透或中继（明拒：「内置一种就是替用户做了一个安全决定」）。

## 8 接口先行（按模块分章）

三条不可动摇的形状约定，它们决定接口而非被接口决定：

1. **`Command` 是穷尽枚举，每个改状态臂携 `IdemKey`**——「双击两下不开两个 Run」由类型保证，不由服务端去重表保证（去重表是第二道，`kernel::gate::dedup` 已有）。
2. **`PutSecret::value: Sealed<String>`**——`Sealed<T>` 无 `Serialize`（kernel-SPEC §8-25），故含它的枚举也无法整体派生 `Serialize`。这迫使 `Command` 的序列化实现**手写并对该臂显式拒绝**，而不是让宏悄悄地把它序列化出去。手写点即唯一权威，`E_WIRE_MISMATCH` 在此产出。
3. **`aggregate` 的上游发送面签名只接受 `Query`**——不是 `fn send(&self, frame: Frame)` 再运行时判断，是 `fn query(&self, q: Query)` 且没有第二个发送方法。

### 8-0 跨层名字的携带法（先于一切接口的决定）

`PlanRow.status` 携 `kernel::RoadmapStatus`（经 `kernel` 重导出，住 `spine::row`，公共拼写不变）。
`Dispatch` 携 `mode`、`Login` 携 `provider`、`CreateBuilding` 携 `template`——**这三个集合的权威分别住 `runtime::Mode`、`gateway`、`city`，而 channels 只依赖 kernel**（ARCHITECTURE §2 depmap）。

取法：wire 携**无封闭列表的 newtype**（`ModeTag`、`ProviderName`、`TemplateName`），只断言「非空且无控制字符」，**不断言合法值集**。合法值集恒由上游单一权威回答，映射点在装配层（`bin::assembly`），未知值即报错不猜。

理由：若 channels 自建一份 `enum Mode`，就产生了**同一规则的第二个权威**（AGENTS.md 明拒），且两份枚举会静默地漂开。channels **确实不知道** mode 集合是什么，假装知道才是谎言。**被否**：（a）channels 内镜像三个枚举——两个权威；（b）把 `Mode` 上移入 kernel——它不在任何缝上，上移只为了让 wire 好看，且删 `runtime::mode` 行需 verdict。

### 8-1 channels::wire（形状 2 值类型 ＋ 形状 1 编解码）＋command／answer／carried_name

**一个文件装着四样东西，现在装四样的四个文件。** 1,278 → 310（wire）＋576（command）＋381（answer）＋88（carried_name）。
切法取自依赖方向而不是行数，**因为方向是无环的**：`wire`（信封：`Query`、五种帧、`WIRE_V`、`schema_hash`）→ `command`／`answer` → `carried_name`。
谁都不回头指，所以加一个 Command 不碰答面，加一个答面不碰命令，而 `carried_name` 的四个新类型谁也不依赖。
- `command`：`Command`／`WireCommand`／`NoSecret`／`COMMAND_NAMES` 与三个只服务于命令的步骤枚举（`LoginStep`／`HaltScope`／`PursuitStep`）。两条不可拼写的性质随类型走，反例仍在 `tests/trybuild.rs`。
- `answer`：二十余个答面结构与 `Answer` 枚举、`HISTORY_MAX`。它们是读形状，一处判定也不做。
- `carried_name`：`carried_name!` 宏与 `ModeTag`／`ProviderName`／`TemplateName`／`UploadId`——**本 crate 不拥有的名字**，只在唯一构造点拒空与控制字符，合法值集恒属上游。
**公开名一字未改**（`lib.rs` 重导出）；变的只有 `cargo public-api` 记的定义模块，`sprawling` 与 `web` 两份基线同变更集重生，各自 SPEC 记一行。

```rust
pub struct ModeTag(String);      // 三个携带 newtype：parse 拒空串与控制字符，不拒未知值
pub struct ProviderName(String);
pub struct TemplateName(String);
pub enum HaltScope { City, Building(Address), Workshop(Address) }  // 协议自有，无外部权威

pub enum Command { /* 逐名取本 SPEC §8-1 表 */ }
pub enum Query   { /* 息 9 */ }

impl Command {
    pub fn name(&self) -> &'static str;   // 穷尽 match：加一个 variant 即编译不过
    pub fn idem(&self) -> Option<&IdemKey>; // 改状态臂恒 Some；唯一例外 Auth
}
impl Query { pub fn name(&self) -> &'static str; }

pub const WIRE_V: u32;
pub const COMMAND_NAMES: [&str; 17];   // 形状 6 数据面：名字权威
pub const QUERY_NAMES:   [&str; 9];
pub fn schema_hash() -> B3Hash;        // blake3("sprawling/wire/" || WIRE_V 小端 || 'C'+名… || 'Q'+名…)

pub enum ClientFrame { Hello(Hello), Command(Box<Command>), Query(Query) }
pub enum ServerFrame { Welcome(Welcome), Event(Box<EventRecord>), Reply(Box<Reply>), Refusal(Box<AxError>) }
pub struct Hello   { pub wire_v: u32, pub schema: B3Hash, pub token: Option<Sealed<String>> }
pub struct Welcome { pub wire_v: u32, pub schema: B3Hash, pub resume_from: Option<Seq> }
```

**三个形状决定及其理由**：

1. **`Command`／`Query` 不标 `#[non_exhaustive]`**。它们的版城所在的机器制是 schema 哈希（加一个 variant 即改哈希，旧客户端在握手处被拒），不是通配臂。不标它使装配层必须**穷尽处理 18 条命令**——新增一条而无人处理在编译期即红。这是想要的约束，不是疏漏。
- **`Query::History` 有界且向后翻页**：服务端只广播「接下来发生什么」，于是今天打开的页面把一座运行了一个月的城看成空城。答复恒**旧在前**——那是账本写它们的顺序，也是折叠期待的顺序；要新在前的读者自己倒一下手上的表，而服务端倒序会把折叠变成调用方的问题。上限 `HISTORY_MAX = 500` 由服务端钳，客户端要不到更多：一个调用方钳不动的上限，少一条让服务端做无界工作的路。`Answer` 因此失去 `Eq`（`EventRecord` 的载荷是任意 JSON，JSON 没有全序相等），crate 外无人比较过两个 `Answer`。
- **`/enroll` 答的是凭据的下场，不是请求的下场**：先前命令一进桌就回 201，于是 vault 拒收谁也不知道，人盯着一条成功消息而密钥根本没存。现在路由**先订阅事件流再投递命令**（顺序是承载的：先投递会让完成得快的 worker 把那一行写进没人在读的流里），然后等三选一——`secret_captured` 且 `ref` 相符即 **201**，`Reply` 回来的拒绝即 **422**，两者都没有即 **202 并在正文里说明为什么是 202**。`SecretSink` 因此长出 `Reply` 参数：worker 在另一条线程上几分钟后拒绝，没有地址的拒绝到不了任何人。
- **回复通道关闭不等于成功**：worker 成功时不调用 `reply.refuse`，`Reply` 随之析构，于是 `recv()` 立即返回 `None`。若把它读成一个答案，它会与 `secret_captured` 赛跑并经常赢——所以关闭只熄灭那条分支，201 仍然只由事件给出。
- **`ConfigureBuilding` 写的是楼自己那一级**：`[sandbox]` 与 `[mcp]` 沿城→楼→房间解析，先前无任何写面，于是一个人读得到自己被什么治理却改不动它。答复里回的是**楼自己那一级的值**而不是解析后的值——用解析值填表，第一次按保存就会把城一级的设置抄进楼里。两个字段各自可缺省，缺省即不动那一节；`mcp` 为空表是「这栋楼一个服务器都不够到」，与「没说」不是一回事。
- **`ProbeEndpoint` 与 `AttachEndpoint` 是两条命令而不是一个开关**：先前模型清单只作为 attach 的副产物到达，于是「看看这把 key 买到了什么」必须先注册。两者的 `IdemKey` 由不同素材派生（`probe:` 前缀），因为问与登记是两件事，一件不得把另一件去重掉。`AttachEndpoint.admit` 空表即全部准入——没看过清单的人本就是这个意思；表里有而 endpoint 不供应的名字被略去而不是被承诺，与阅览室对不在书架上的 skill 的答复同形。

2. **`name()` 的穷尽 match 是计数断言的真机制**。光有 `COMMAND_NAMES.len() == 17` 拦不住「加 variant 但不改表」；`name()` 穷尽后，新 variant 必须在 `name()` 里现身，测试再断言它必在名表内。三道连环：编译 → 名表 → schema 哈希 golden → SPEC 同集变更。
3. **`Command` 泛型于 secret 携带者，远端实例把它钉成不可居住类型**（强于「手写 Serialize 在该臂报错」的写法）：

```rust
pub enum NoSecret {}                                   // 无值可造
pub enum Command<Secret = Sealed<String>> { … PutSecret { value: Secret } … }
pub type WireCommand = Command<NoSecret>;              // 套接字所能携的全部
impl From<WireCommand> for Command                     // 总函数；PutSecret 臂写作 `match value {}`
```

它同时关死两个方向，**且两边都是编译期**：出——`Sealed<String>` 无 `Serialize`，故 derive 生成的 `impl<S: Serialize> Serialize for Command<S>` 对 `Command<Sealed<String>>` 不成立，`serde_json::to_string` 对它是编译错误；入——`WireCommand::PutSecret` 的 `value` 字段无值可填，任何类型都不匹配。运行期只剩一道兑底：字节写着 `put_secret` 时 `Deserialize` 拒收，**拒绝文案恒不回显它正在保护的字节**。两个编译期反例住 `tests/ui/put_secret_onto_the_wire.rs`（stderr 快照分别钉在 `Sealed: !Serialize` 与类型不匹配两个正因）。

**被否**：另写一个 16 variant 的 `WireCommand` 枚举——它把 16 条命令的声明拄成两份，是同一规则的第二个权威。

4. **`Auth`／`Hello` 的令牌是明文 `String`，而 `PutSecret` 的值是 `Sealed`**。不对称是故意的：配对令牌**必须跨线**才能完成配对，在传输中密封它只是自欺；它在**落地一刻**被封（`decide_handshake` 只接受 `&Sealed<String>` 作为已配置值）。凭证则相反：它本就不应跨线。

5. **`Rollback{checkpoint}` 携 `kernel::GitOid`，为此给 `GitOid` 补 serde**（与紧邻的 `B3Hash` 同形：40 位小写 hex，长度不对即拒）。**被否**：在 wire 里自建 `CheckpointRef(String)` 并自校 40 hex——那是 git oid 形状的第二个权威。该变更属 kernel 公开面，已与 kernel-SPEC §8-2 同集提交（apisync 门）。

### 8-2 channels::server（形状 4 薄壳）＋reception（形状 1）＋assets（形状 4）

**Humble Object 在此的切法**（ARCHITECTURE §7 末段，理由只写一次）：难测的一端（tokio＋axum 监听）剥到最薄，厄的一端（绑定面判定、握手判定）是纯函数，无需跑服务即可穷尽测。

**切法写在这里，而两半一直在同一个文件里。** 模块表给 `server` 的形状是 `adapter`——ARCHITECTURE §9 定义为「薄、无策略：换第二个实现不改变任何策略」——而四个判定函数就是策略。于是它们搬进 `channels::reception`（绑定／录凭证／握手／帧），客户端资产搬进 `channels::assets`，`server` 剩下的每一条分支不是一次发送、一次接收，就是一次会话的结束。三个文件 1,235 → 649＋385＋271。
**公开名一字未改**（`lib.rs` 重导出）；变的只有 `cargo public-api` 记的**定义模块**，所以 `sprawling` 基线里 `Serving::client` 的类型路径从 `channels::server::ClientAssets` 变成 `channels::assets::ClientAssets`，两份 SPEC 同变更集各记一行。

```rust
pub enum BindFace { Loopback, Exposed }              // 穷尽，不是 bool
pub enum BindVerdict { Serve(BindFace), Refuse(AxError) }
pub fn decide_bind(addr: &SocketAddr, token_configured: bool) -> BindVerdict;

pub enum EnrollVerdict { Accept, Refuse(AxError) }
pub fn decide_enroll(peer: &SocketAddr) -> EnrollVerdict;   // 只认回环调用方
pub type SecretSink = Arc<dyn Fn(Command<Sealed<String>>) -> Result<(), AxError> + Send + Sync>;
pub struct EnrollBody { pub realm: String, pub name: String, pub value: String }

pub enum HandshakeVerdict { Accept, Reject(AxError) }
pub fn decide_handshake(hello: &Hello, expected: &Welcome, configured: Option<&B3Hash>) -> HandshakeVerdict;

pub struct ServeConfig {
    pub addr: SocketAddr,
    pub token_digest: Option<B3Hash>,   // 摘要，不是令牌
    pub client: Arc<ClientAssets>,      // 客户端资产源由装配层递入
    pub upload_sink: Arc<dyn Fn(Bytes) -> Result<UploadId, AxError> + Send + Sync>,
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

**为什么 `index_html: Arc<[u8]>` 改成 `client: Arc<ClientAssets>`**：原形状只能携带一个文件，而真实客户端是 `index.html` ＋ `web.js` ＋ `web_bg.wasm` ＋ wasm-bindgen snippets——单文件形状使「单二进制交付」从未真的成立（页面壳引用 `./web.js`，服务端却没有那条路由，浏览器拿到的是空页）。资产表是封闭清单：路径穿越（`..`、空段、盘符、点头文件）在判定层拒，miss 报文件名并给出重建口令。`Disk` 臂逐请求读盘，专供开发回路（改前端刷新即见），发布路径恒不构造它。

**`upload_sink` 收 `Vec<u8>`，不收传输层的缓冲类型**（接 bin `serve` 时发现）。初版写的是 `axum::body::Bytes`，于是装配层为了递一个 sink 就必须直接命名 axum。**一个泄露自己传输层的公开签名，会把「换掉 HTTP 库」变成对每一个从未选过它的调用方的破坏性变更**。

**令牌只以摘要形式进入本 crate**（由 `xtask secret` 门逆推出的修正）。初版写的是 `Option<&Sealed<String>>` 加一次 `.expose()`，门当场咬住——expose 只得出现在兑付点。**修因不修门**：拿令牌的一方自己摘一次，边界只比摘要。代价为零（常数时间比较本来就要先摘），收益是 `channels` 在类型上根本拿不到配对令牌的明文。常数时间比较因此退化为定长 32 字节的无早退异或，**既无内容侧道也无长度侧道**。

`decide_bind` 的四格真值表是全部行为：回环×无令牌＝`Serve(Loopback)`；回环×有令牌＝`Serve(Loopback)`；非回环×有令牌＝`Serve(Exposed)`；**非回环×无令牌＝`Refuse(E_CONFIG_INVALID)`**。拒绝发生在**启动时**，不是启动后拒连——它是配置判定。

薄壳的职责恒为三件：静态资源（`include_bytes!` 的前端产物）｜WS 升级｜上传端点（`Attach` 的字节，不走 WS 帧）。它不持业务状态，不做策略判断。

**WS 路由与两条沿途缝**。升级后的会话只做三件事：先收 `Hello` 并交 `decide_handshake` 判（拒即关，不降级）；收到 `ClientFrame::Command` 交给 sink；把订阅到的 `EventRecord` 以 `ServerFrame::Event` 推给客户端。

```rust
pub struct ServeConfig {
    /* …前四项不变… */
    /// 命令受理面：**只受理，不执行**。同步、不阻塞；真正的回合循环在装配层自己的任务里跑。
    pub commands: Arc<dyn Fn(WireCommand) -> Result<(), AxError> + Send + Sync>,
    /// 事件广播源。本 crate 只 `subscribe`，恒不发送——写入方是 Ledger。
    pub events: broadcast::Sender<EventRecord>,
}
```

- **为什么 sink 只受理不执行**：一个 Dispatch 会跑几分钟到几小时。把它做成 `async` 并在 socket 任务里 await，等于把一条连接的寿命绑在一次派活上；刷新页面就会杀掉工作。**受理后立即返回，进展从 Ledger 的事件流回流**——这同时使「关掉界面再打开」与「从未关过」在服务端看来无差别。
- **为什么广播的是 `EventRecord` 而不是自定义推送体**：客户端要重建的正是那一行历史。另造一个推送类型等于为同一件事立第二个形状权威，而两者一旦漂开，界面会显示一个历史里没有的事实。
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

pub commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
```

- **拒绝属于发问者，不广播**。把它做成一条事件会告诉所有在看的人「别人打错了一个字」，而事件流是这座城的历史，不是某个人的错字簿。故每条会话自持一个无界队列，`Deliver` 时把写入该队列的闭包随命令交给工人；会话的 `select!` 因此从两臂变三臂。
- **`Delivered` 是三态而不是 `Result`**，因为「没有人问过」与「问的人走了」是两件不同的事：前者是排程的正常形态，后者值一行诊断。这也是**不得重新引入 `let _ =`** 的落法——`SendError` 被穷尽消解成一个领域枚举，而不是被丢掉。
- **无界队列而非 `broadcast`**：一条拒绝丢不得，而它的量级是「人点错的次数」，不是事件流量。
- **未做且已知**：`/enroll` 路由同病。它同步答 201，而 `PutSecret` 是投递到同一张桌子的，工人的拒绝到不了 HTTP 响应。此处**不顺手改**，因为桌子在一次 dispatch 期间不被读取，把 HTTP 请求做成同步等待会让它挂上几分钟；正确的形状是有界等待加 202，随 `sprawling enrol` 一并落。

**Query 的答面**。`ServeConfig.queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>`，同步；`ServerFrame` 增 `Answer(Box<Answer>)` 变体。答面类型住 `wire`：`Answer`（City／Run／Approvals／Cost／Unavailable）、`RunSummary`、`CityAnswer`、`ApprovalsAnswer`、`CostAnswer`。

**`Query::BuildingView { addr }` → `Answer::Building(Box<BuildingAnswer>)`**（`BuildingDoc`／`ArchiveLine` 随之入 wire）。楼里的文件是楼的记忆，服务端在被问的那一刻读盘——**文件是权威**，另存一份索引就是第二个权威。`QUERY_NAMES` 因此从 10 增到 11，schema 哈希随之从 `238f11b2…` 变为 `85705c03…`：客户端与服务端同批发布，旧页面会在握手期被明确拒绝并提示刷新。

**`Welcome` 携 `city: Option<Address>`，`decide_frame` 增一个 `city` 入参。** 事件流只送连接之后发生的事，而城市的名字写在 Ledger 的第一条记录里——一个今天打开的浏览器永远等不到它。握手是「这是哪座城」的自然回答处；服务端从同一条创世记录读它，故两边不构成第二个权威。同批：`init` 把城市名写进创世记录的 `addr`（此前是 `None`，城市名只活在目录项里）。

**`ApprovalsAnswer.items` 携 `kernel::ApprovalItem` 全项，`ApprovalSummary` 删除。** 旧摘要类型丢掉了 `cluster_key` 与 `created`，于是界面无法按类聚合、也排不出「谁等得最久」；服务端为了填它还要从事件载荷里猜一个 `summary` 字段——那个字段从来没被写过，故每一条待批项都渲染成「(no summary recorded)」。载荷本身就是 `ApprovalItem` 的序列化，原样送过去既少一次有损转换，也让「什么算一类」只有 `web::approval::inbox` 一处答案。

- **为什么是强类型答面而不是一团 `Payload`**：`web` 只依赖本 crate，故发帧的边界 crate 欠对方一套读帧的词汇（同 kernel 再导出的理由）。一个无类型载荷会把解析责任推给每一个视图模块，每一个都得自己猜一遍形状。
- **`Answer::Unavailable { query }` 是一个真答案**：不求值的视图报自己的名字，而不是返回空结果——空城与未实现在界面上必须长得不一样。
- **`CityAnswer.buildings: Vec<BuildingProgress>`**：每栋楼一行，携 `Progress` 与 `problems`。解析不出的行进 `problems` 并照显——悄悄丢掉读不懂的行，等于按一个没人选过的分母报进度。
- **五维成本携权威总额**：`CostAnswer.total` 与五个维度各自求和相等；界面按 `total` 算占比而不自己归一，未归因余额因此看得见。

- **採用 `broadcast` 而非每连接一个队列**：多个标签页是常态；慢客户端被 `Lagged` 拉下而不拖住写入方，它重连时从 `Welcome.resume_from` 补齐（接前端时兑现）。

## 8.5 两个设计

**握手的失败处置：断连 vs 降级协商。** 取断连。理由是这条错配的真实来源早已写明——「浏览器可能缓存了旧前端而服务端已经升级」，而降级协商要求服务端同时维护两套 wire 语义，那是两个权威。断连＋提示刷新把一个协议问题还原成一个刷新动作。**被否**：版本协商（多版本共存）——它的成本在每次改 wire 时都要付，而收益只在「用户不肯刷新」这一个场景里兑现。

**上传通路：WebSocket 分帧 vs 独立 HTTP 端点。** 取独立 HTTP 端点。理由是「WebSocket 不适合携带数百 MB 的帧」。**被否**：在 WS 上自制分片协议——那是重新实现 HTTP 已经做好的事（范围请求、断点续传、进度），且会让 `Attach` 的传输失败与命令失败混在同一条通路上难以区分。

**`POST /enroll`，唯一携凭证字节的路由**

它是 HTTP 而非 socket 帧，因为 socket 的 `WireCommand` **拼不出** `PutSecret`（`NoSecret` 无值）。两半合起来才是完整保证：类型层管住帧，`decide_enroll` 管住字节——因为字节总可以被 POST 到一个路由上。

- **只认回环对端，配对令牌也不算数**：令牌认的是人，而这条规则管的是**字节走到哪里**。拒绝的第三段指向宙主机，于是它是约束而非死路。
- 壳里零策略：判定在 `decide_enroll`，壳只搬字节——同 `decide_bind`／`decide_frame` 的切法，故无需跑服务即可穷尽测。
- 应答返回 `secret:<realm>/<name>`；值不回声、不入事件载荷。入金库由 `Sealed::into_vault_value`（住 kernel::secret，即 expose 白名单三文件之一）完成，开封因此**不发生在装配层**。

**`Login` 携 `LoginStep { Begin, Code { code } }`，WIRE_V 1→2**。两步之间站着一个人：provider 在它自己的页面上把 code 显示给他，他再带回来。**用穷尽枚举而不是 `Option<String>`**——「开始登录」与「兑付这个 code」是两个动作、两种失败，一个页面表达其一时恒不该被读作另一个。`COMMAND_NAMES` 不变，故 schema 哈希单靠名字表不会动；这正是 `WIRE_V` 存在的那种情形（语法换形而名字没换），于是版本进位、旧页面在握手期被明确拒绝。**恒不开回环监听端口**：该 provider 的 redirect 就是它自己的页面，多一个监听口就是多一条没人走的入口。

**`Login` 携穷尽枚举 `LoginStep { Begin, Code { code } }`，WIRE_V 1→2**。两步之间站着一个人：provider 在它自己的页面上把 code 显示给他，他再带回来。用枚举而不是 `Option<String>`——「开始登录」与「兑付这个 code」是两个动作、两种失败。`COMMAND_NAMES` 未变故 schema 哈希单靠名字表不会动，这正是 `WIRE_V` 存在的那种情形。**恒不开回环监听端口**：该 provider 的 redirect 就是它自己的页面。

**五个查询各有自己的答**（`InboxView`／`DiscardView`／`RegistryView`／`ArchiveSearch`／`Metrics`），WIRE_V 2→3。此前它们一律答 `Unavailable`，界面据此什么都画不出来。三条口径：①**队列折叠着看不消费着看**（`Inbox::pull` 要拿走才给内容，看一眼就取走的视图会改变它所报告的对象）；②归档在被问的那一刻读盘（同 `BuildingView`，文件是权威）；③**`Metrics` 恒不携钱**——钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。

**`DiscardLine.restoration` 携 `Option<Restoration>` 而非一个句子，WIRE_V 3→4**。回收站那一行的「怎么拿回来」原本在服务端被拼成 `"tracked: file:…"`，而客户端早已持有它的唯一措辞处（`web::approval::ReturnPath::sentence`）——**一件事两个渲染权威**，而服务端那个还拼不出可执行的那句话。现在计划以它自己的形状上线（载荷本来就是 `Restoration` 序列化出来的，故读得回去）；`None` 的意思是**这一条记录用了本构建读不懂的方案**，界面据此画一行而不给动作（`ReturnPath::Undescribed`）——行恒不隐藏，因为藏起一件被删的东西比承认读不懂它的方案更糟。`QUERY_NAMES` 与 `COMMAND_NAMES` 未动，故哈希只因 `WIRE_V` 而变——又一例「语法换形而名字没换」。

**`POST /acp` 与 `AcpSink`**。外来编辑器的请求走自己的路由，不挤 Command 面：它自带鉴权、要一个当场的回答，而 Command 面的回答是事件流。三条口径：①**令牌在本 crate 判**（配对令牌住这里，常数时间比对也就住这里），只把 `authentic` 一位传进去——拒词由 `protocol::admit` 措辞，「未配对者只学到一位」因此只有一个权威；②回给编辑器的只有 `AcpProgress` 三字段，run id 是工人接单时才铸的，故受理那一刻诚实的答案是「已受理、未完成」；③没配对令牌的城即回环独占，与 control surface 同一条规矩。

**三帧登记面**（§8-1 golden 同集更新）——`AttachEndpoint`（人刚输入的 URL＋兼容格式＋`secret:` 引用；**引用有字节形，凭证没有**）、`SelectModel`（标签→模型＋两个探不到的 token 数；输出上限是 `Option<Ceiling>`，缺席即「没人登记过」，零在类型上不存在）、`EndpointView`（设置页的读；`EndpointsAnswer` 里 `has_credential` 是关于凭证能回答的全部）。

**三个 kernel 类型的再导出**（`DialectKind`／`Effort`／`ModelTag`）。`web` 只依赖 `channels`（拓扑图），而设置页要拼写这三个词；再导出而非镜像定义，因为镜像就是同一规则的第二个权威——同 §8-0 对 `Mode` 的口径。

### 8-3 channels::auth（形状 2 值类型 ＋ 形状 1 判定）

```rust
pub struct PairingToken(B3Hash);               // 只是摘要，不持明文
impl PairingToken {
    pub fn mint(entropy: [u8; 32]) -> (Self, String);  // 右侧即只展示一次的配对码
    pub fn from_configured(raw: &str) -> Result<Self, AxError>;
    pub fn digest(&self) -> B3Hash;                    // 交给 ServeConfig 的全部
}
pub fn verify(presented: Option<&str>, expected: &B3Hash) -> bool;  // 常数时间
```

四条决定：

（a）**熵入参不采样**——使铸造可重演、可测，与「种子 RNG 单点发放」一致。

（b）**`PairingToken` 不持明文**（由 `xtask secret` 门逆推出的修正）。初版写的是 `Sealed<String>` 加一个 `display_form()` 里的 `.expose()`，门咬住。候选的应对是把该点加进兑付点白名单——那是修门。**实际修的是因**：本模块真的不需要明文，`mint` 把配对码直接交给调用方去展示，自己只留摘要。封一个值再在下一行解封是表演；**根本不持才是我们想要的性质**。

（c）**字母表剔除可混淆字符**（0／O／1／l／I／5／S），29 符号×四组五位≈ 97 位熵。理由不是审美：配对码要被人读出来、在另一台机器上手敲进去，一个口述会错的码，代价由用户在另一台机器前承担。

（d）**分组形兼顾了 secret 门**：每片 5 字节远低于 20 字节阈值，测试里的配对码字面量不会被熵侦测器咬住。

### 8-4 channels::control（形状 1 判定函数）

```rust
pub enum Intervention { Steer, Cancel, Takeover, Rollback, Halt, Release }
pub enum ControlVerdict {
    Intervene { verb: Intervention, run: Option<RunId>, must_write_handoff: bool },
    NotAnIntervention,
    Refuse(AxError),
}
pub fn classify(command: &Command) -> ControlVerdict;
```

**本模块持有的唯一规则**：中断一个活着的 Run 的三个动词（Steer／Cancel／Rollback）**恒以 Handoff 收尾**，使下一位（人或 Agent）拿得到完整现场。`Takeover` 同理。`Halt`／`Release` 按 scope 停一片，不针对单个 Run，故 `run` 为 `None` 且不强制 Handoff。

**`must_write_handoff` 为什么是返回值而不是副作用**：本 crate 不持 Ledger 句柄（§7 已写）。它只能**说出义务**，履行义务的是装配层。把它做成返回值的代价是装配层可能忽略它——故同变更集交付一条断言：一次干预的事件序里若无 `handoff_written`，即失败。

**为什么不并入 `server`**：`server` 的职责是「这个字节流能不能变成一个帧」，`control` 的职责是「这个帧是不是干预、干预要留下什么」。后者在无网络的 citysim 里也成立，前者不。两个职责的变化率也不同：加一个动词不应该碰监听器。

### 8-5 channels::aggregate（形状 1 判定 ＋ 形状 2 值类型）

```rust
pub struct CityLabel(String);                     // parse 拒空与控制字符
pub struct Upstream { label, address: String, token_digest: Option<B3Hash> }
pub struct Sighting { city: CityLabel, event: EventRecord }
pub struct Forwarded { address: String, query: Query }   // 无 Command 字段
pub struct Aggregate { /* BTreeMap<CityLabel, Upstream> */ }
impl Aggregate {
    pub fn attach(&mut self, Upstream);  pub fn detach(&mut self, &CityLabel) -> bool;
    pub fn cities(&self) -> impl Iterator<Item = &Upstream>;
    pub fn ask(&self, &CityLabel, Query) -> Result<Forwarded, AxError>;  // 唯一发送面
    pub fn merge(Vec<(CityLabel, Vec<EventRecord>)>) -> Vec<Sighting>;
}
```

**硬约束的存放形式**：它只有一句「聚合层只转发 Query 与 Event，恒不转发 Command」。它在这里**不是一个判断**，是一个缺席：`ask` 接 `Query`，而没有第二个发送方法。连 `Forwarded` 也不带 Command 字段，使下游无处升格。理由：一个可代发命令的聚合层是**不在任何一本账上的跨城权威**——目标城无法把命令与发起人对应。

**合流序为什么不能用 seq**：两座 City 各有各的 Ledger，各自从 1 编号。故排序键取 `(t, city, seq)`：时间先行，label 入键而非做最后的破平——因为两城同一毫秒是常态，而合流视图必须两次一样。

**本模块不含传输**：需要确定性与可测性的是合流序与转发面，两者都不需要 socket。实际连接属装配层（界面接入时）。

### 8-6 crate 面的两项（随 `web` 接入时定下）

**一、`server` feature**（默认开）。`web → channels` 是 depmap 冻结边，而 tokio 的 mio **编译不到 wasm32**。故监听器进 feature：`server = ["dep:tokio", "dep:axum"]`，`web` 取 `default-features = false`，只得 wire／control／aggregate 三模块。浏览器里本来也没有可供监听的 socket，这条分割与现实同形。

**被否**：把 wire 拆成第六个 crate。那要改 ARCHITECTURE §2 冻结拓扑（需 verdict），而 feature 边界已足以表达「词汇与监听器分开」这一件事。

**代价与看守**：`cargo clippy --workspace --all-features` 在 host 上恒开 `server`，故关掉它的构建不在 `just check` 覆盖面内；补以 `just check-web`（wasm32 目标上跑 clippy，路径上必然关掉 `server`）。

**二、kernel 类型再导出**。本 crate 的公开签名上出现的 kernel 类型（`EventRecord`、`AxError`、`RunId`、`Seq`、`Address`、`BudgetCap` 等）一律从 `lib.rs` 再导出。理由是拓扑硬约束：`web` 只能依赖 `channels`，一个拿不到 `EventRecord` 的客户端读不了自己收到的帧——**发帧的边界 crate 欠对方一套读帧的词汇**（C-REEXPORT）。

**被否**：给 depmap 加 `web: channels, kernel`。那是改冻结面去适应一个本就有标准解法的问题。

**再导出集随 `web` 的需要生长**，当前含：标识与度量（`Address`、`RunId`、`Seq`、`TimeMs`、`UsdMicros`、`Tokens`、`B3Hash`、`GitOid`、`IdemKey`）｜事件（`EventRecord`、`EventDraft`、`EventKind`、`Payload`）｜判定结果（`AxError`、`AxCode`、`Progress`与两系、`BudgetCap`、`BudgetUse`、`PolicyVerdict`、`Autonomy`）｜待批与删除（`ApprovalItem`、`ApprovalId`、`ApprovalClass`、`ApprovalSource`、`ClusterKey`、`Restoration`、`Locator`）。

> **施工纪律（被 apisync 门咬两次后写下）**：动本 crate 的再导出列表就是动公开面。本节必须与该变更**同一提交**更新——它很容易被当成「只是多写一行 `pub use`」而漏掉，而门不接受这个理由。

### 8-7 一次会话有名字，而名字就是它干活的那个房间（未实现，设计已定）


```rust
pub const WIRE_V: u32 = 5;                     // 4 → 5：一个字段，一次握手拒绝
WireCommand::Dispatch { addr, task, goal, mode, budget, idem,
                        session: Option<SessionName> }
pub struct SessionName(String);                // 形状 2；一个构造点，内容即一个地址段
```

**问题不在线格式上，在于没有人给新会话开房间。** ARCHITECTURE §6 自己写着 `JOB.md — the task for this session`：房间本来就是会话的工作区。今天派活要人手打一个 `building/room`，于是所有派活撞进同一个地址，模板文件互相覆盖。

- **不加第四层**：每个 Run 一个目录会切断 Handoff 的连续性，而连续性正是一个 session 之所以是 session 的东西；`collab` 整套（draft／PR／fanin）也都建立在「几个居民在同一栋楼里不互相踩」上。
- **地址给楼，名字给会话**：`addr` 可以只是一栋楼；`session` 在它底下开一个房间。重名加数字后缀（`refactor`、`refactor-2`），而不是拒绝——一个人连着开两次同名会话是常事。
- **向已有房间派活即继续那条会话**：它的 `Handoff.md` 与 `JOB.md` 就是连续性。「回复某个 session」因此不需要新概念。
- **`RunId` 不变**：账上的身份仍是 `b3(job|addr|now)`；名字是房间的标签，不是 Run 的。一个房间一生中的多次 Run 合起来才是一条会话。
- **`SessionName` 是值类型而非 `String`**：它必须能当一个地址段（无 `/`、无 `.`／`..`、无控制字符、非空、不叫 `.sprawling`），否则一个人输入的字会变成一条路径。构造点一个，拒绝携 recovery。
- **`WIRE_V` 4 → 5**：握手处的 schema hash 因此变，旧页面拒绝而不是误读——这正是那个机制存在的理由。
- **服务端开房间落在 `city`**（新一节，同期写）：已存在名字→加后缀，目录建在楼下；`.sprawling` 不得为会话名（`city` 的保留名谓词直接答这件事）。
- **客户端（同期写在 web-SPEC）**：派活条第一格从「你猜 building/room」变成「选一栋楼 ＋ 这次叫什么」；直播页与楼页显示名字而不是 `short_run` 的十六进制。

## 9 工作流程

（待填：从监听、绑定面判定、握手、鉴权，到帧解码、处理器分派、Event 推送的完整通路。）

## 10 实现逻辑

（待填。）

## 11 边界枚举

（待填：握手哈希不配／令牌错／绑定非回环无令牌／上传中断／客户端半关连接／聚合上游断线／Event 推送背压。）

## 12 错误处理（逐码答「能否定义掉」）

- **`E_WIRE_MISMATCH`**：不可——它是装载期五码之一（封闭白名单），且它的存在理由就是「浏览器缓存旧前端」这一 WebUI 特有错配。类型无法定义掉跨版本的字节。
- **`E_CONFIG_INVALID`**：不可——绑定非回环而无令牌必须在**启动时**拒绝，这是配置判定不是请求判定。
- **`E_SIGNAL_UNKNOWN`**（ARCHITECTURE §11 点名的三条待消解之一，本 crate 须作答）：**部分定义掉**。形状未知的一半可以定义掉——握手的 schema 哈希保证同一连接的两端共享同一份 Signal 枚举，故「收到一个不认识的 Signal 种类」在单个连接内不可表示。语义未知的一半不可定义掉——一个 Resident 收到语法合法但自己不处理的 Signal 类别，这是真实结局，判定位置在消费端 `collab::inbox`，channels 只是 `Attach{notify}` 的产地。**结论：本码不在 channels 消解，其生产消费者住 `collab::inbox`；本条即 ARCHITECTURE §11 要求的作答，不是默认保留。**

## 13 依赖选型

| 依赖 | 用途 | 依据与替代 |
|---|---|---|
| `tokio` | 异步运行时；本 crate 是全库首个真异步消费者 | B.7 已钉；gateway 明确把它推迟至此。替代（自写 reactor）被 C12 与「复用既有机制」双拒 |
| `axum` 0.8.x | HTTP 静态资源、上传端点、WS 升级 | B.7 写「axum 或同类」。取 axum：tokio 官方序列、7978 下游 crate、2026-04-14 仍在发版，且其 WS 支持内含 tokio-tungstenite，省掉一层版本对齐 |
| `tokio-tungstenite` | WS 协议 | 经 axum 传递依赖（axum 0.8.9 已升至 0.29）；**不直接依赖**，避免两处版本权威 |
| `serde`／`serde_json` | 帧编码 | 已在 workspace |
| `kernel` | AxError／EventRecord／IdemKey／Sealed／Address | 唯一上游 |

**不引**：任何通用 RPC 框架（wire 是 17＋9 个具名 variant，不是一个可扩展的服务定义）；任何 session 中间件（鉴权面只有配对令牌一件）；任何穿透／中继库。

**依赖面代价须实测并回填**：引 tokio＋axum 后 `channels` 的依赖 crate 数，按先例（wasmtime 使 runtime 从 71 涨到 257）在收口时记录。

## 14 硬编码声明

- **默认绑定地址恒为回环**——它不是配置的默认值那么软，而是「非回环需要额外条件才允许」的判定起点。
- **schema 哈希的派生框架**（哪些类型入哈希、以什么序）一旦定下即是冻结面：改框架＝旧客户端全部拒配。故派生框架与 `IDEM_DERIVE_V` 同规，携版本字节。

## 15 影响面

- 根 `Cargo.toml`：`workspace.dependencies` 增 tokio／axum（guard 辖区，须 `Verdict:` 尾注）。
- `crates/sprawling/assembly.rs`：`serve` 装配点——gateway 的 Custodian 生产装配、memory 三视图的界面查询、`runtime::replay` 补写面的启动扫描都在此接线（ARCHITECTURE §6 接线台账到期项）。
- `xtask/api-baselines/channels.txt`：从零起算。
- ARCHITECTURE §6 模块表 channels 五行：从「未建」翻「已建」。

## 16 测试与约束

- 计数断言：Command 17、Query 9，逐名对本 SPEC §8-1 两表（与 kernel 的 specalign 同规——**若 wire 表也值得机器看守，评估扩 specalign 覆盖面**）。
- trybuild 两反例：远程 `PutSecret`（含 `Sealed` 的帧不可序列化）；`aggregate` 发 Command（发送面无该入口）。
- 常数时间比较的性质测试：差异位置不影响比较耗时。
- 握手 golden：schema 哈希入快照；改 wire 类型必须同时改快照与本 SPEC。
- 绑定面判定的单元测试：回环／非回环×有令牌／无令牌四格，只有「非回环＋无令牌」拒绝启动。
- 约束：非测试代码遵守 C3 硬化全条；全库禁裸 spawn（确定性第 3 条）——**本 crate 的并发必须是结构化的，带取消令牌**，这是引入 tokio 后第一条要守住的线。

## 17 模型体验

零字节，因为 wire 面是人与服务端之间的协议，不进入任何 prefix。间接影响有一条：`Steer` 经 control surface 送达后，落点是**结果信封**（追加在下一次工具调用结果末尾，前缀 `user`），那几个字节由 `runtime::pipeline` 计入，不由本 crate 计入。

## 18 文档同步

- ARCHITECTURE §6 模块表 channels 五行状态；§6 接线台账中到期的五行（channels 全部、gateway Custodian 生产装配、memory 三视图界面消费者、attribution→CostView、replay 补写面→resume 启动扫描）。
- AGENTS.md：若新增命令面配方则同步命令表。
- 依赖钉版表「axum 或同类 / tokio-tungstenite」行：回填实际选定版本。
- `crates/sprawling/sprawling-SPEC.md`：`serve` 子命令的装配面。

### 8-8 第三类帧：模型还在说的时候（形状 2 值类型）

**一个 token 增量不是效果，所以它不是事件。** 事件是发生过的事：有序号、进账本、可重放、可离线验。增量没有序号、永不落盘、无法重放，而且一个客户端漏掉一条什么也没丢。把它折进事件流，等于给「模型说了什么」造第二份、不可验证的历史——而这座城全部的主张就是那份历史可验。

于是它是 `ServerFrame` 的第四个取值：

```rust
pub enum ServerFrame { Welcome(..), Event(..), Answer(..), Refusal(..), Delta(Delta) }
pub struct Delta { pub run: RunId, pub text: String }
```

`WIRE_V` 11 → 12，schema 哈希随之变（`ServerFrame` 不进名字表，故这是「语法换形而名字没换」那一类，版本进位、旧页面在握手期被明确拒绝）。

**三条口径，各自都是必要前提：**

1. **携 `RunId` 而不携序号。** 客户端据此把缓冲挂在一个 run 名下，并在该 run 的 `model_returned` 到达时整个丢掉。这就是「结算文本赢」的全部机制——一条断言钉住它（`web::session`）。
2. **两条广播通道而不是一条。** 增量与事件的丢弃语义相反：漏掉的增量什么都不是，漏掉的事件是必须从账本补回的历史。共用一条通道会让一次话多的模型把记录挤出慢读者的窗口。
3. **没人看时不开流。** 装配层只在有客户端时装 sink；`RunHooks.deltas` 为 `None` 的 run 走原本的阻塞调用，字节不差。于是 citysim 与离线重放的路径一字未改。

**服务端半边在 `gateway`：** `kernel::Model` 多一个 `call_streaming(req, onto)`，默认实现就是 `call` 并且不报告任何增量——一个没有流的适配器因此是诚实的而不是坏的。`gateway::endpoint` 覆盖它：请求带 `stream: true`，逐行读 SSE，`dialect::increment_of` 只认各 dialect 用于助手散文的那个字段（工具参数与 thinking 块一律不报——半个工具参数不是短一点的工具参数），最后 `dialect::settled_from_stream` 把帧重装成**非流式的那个形状**，交给同一个 `response_from_wire`。**结算答案因此只有一个解析器**：流式调用与阻塞调用不可能对同一个回复得出两个结论。流被切断仍然表现为读取错误，永不表现为一个变短的回答。

## 19 每个动词从哪里够得到（`xtask wiring` 的数据面）

**这张表存在的理由，是一次已经发生过的失效。** v0.0.3 的审计发现 `assembly::run_command` 只匹配 22 个 Command 里的 14 个，
六个动词落进 catch-all——其中 Takeover／Rollback／CreatePolicy **在线上、画在客户端、由任何东西执行不了**，
而 Cancel 与 Steer 在 run 不处于安全点时失败，恰好是人最需要它们的那一刻。
`not_built` 的 rustdoc 当时就写着「**在这里被回绝的动词不得作为控件出现在客户端**」——那是一条**没有任何机器在看的规矩**。

反方向同样会失效，而且更安静：**一个城做得到、却没有任何控件够得到的能力，没有人会收到抱怨**，
因为不存在的按钮不会有人去点。`Pursue`（「城自己走」）与 `SetAutonomy` 就是这样漏掉的，
这也正是验收标准第 2 条一直没过的机制原因——**在浏览器里打不开它**。

### 19-1 reach 的四个取值

| 取值 | 含义 | 门要求什么 |
|---|---|---|
| `client` | 人用的动词，客户端必须画得出 | `client/src` 里有发出点，且 `run_command` 不以 `not_built` 作答 |
| `push` | 由外部服务推进来，不是人点的 | 只要求 `run_command` 能执行；客户端有没有它都不看 |
| `handshake` | 在握手层被吃掉，进不到 `run_command` | 两侧都不要求 |
| `sealed` | 线上不可拼写 | 两侧都不要求；客户端**若**出现即为红 |

### 19-2 表

| Command | reach | 说明 |
|---|---|---|
| `Dispatch` | client | 派活，产品的正面 |
| `Login` | client | 登录一个 provider |
| `ProbeEndpoint` | client | 问一个端点它供应什么 |
| `ConfigureBuilding` | client | 改一栋楼的规矩 |
| `AttachEndpoint` | client | 把一个端点挂上 |
| `SelectModel` | client | 选一个模型 |
| `Fork` | client | 从一条线上分出第二条 |
| `Reveal` | client | 在人自己的文件管理器里指出一个地址 |
| `DoctorInstall` | client | 按需求表里的名字装一件机器缺的东西 |
| `DoctorRefresh` | client | 重新探一遍机器，取代开城时的快照 |
| `CreateBuilding` | client | 起一栋楼 |
| `Steer` | client | 中途换方向 |
| `Cancel` | client | 停下这一个 |
| `Halt` | client | 停下一个范围 |
| `Release` | client | 放开一个范围 |
| `Approve` | client | 答一条审批 |
| `SetAutonomy` | client | 定一栋楼的 Autonomy（三态） |
| `Pursue` | client | 设一个持续追的目标，以及暂停／恢复／清除 |
| `PutDocument` | client | 写治理这座城的三份文件之一 |
| `Attach` | client | 传一份附件 |
| `Takeover` | client | 人接管一条在跑的线 |
| `Rollback` | client | 回到一个检查点 |
| `CreatePolicy` | client | 把一条判定变成一条常规 |
| `BatchByBuilding` | client | 按楼成批派活 |
| `Wake` | push | 外面发生了一件事；地址由 watch 表与 triage 决定，调用方说不出房间 |
| `Auth` | handshake | 出示配对令牌，`server::decide_handshake` 吃掉它 |
| `PutSecret` | sealed | 唯一没有字节形式的 Command；`Sealed<String>` 在线上不可居留 |

**`client` 而尚未落地的五个**（`Attach`／`Takeover`／`Rollback`／`CreatePolicy`／`BatchByBuilding`）今天由 `not_built` 作答，
所以门对它们要求的是**客户端不画**——`not_built` 的 rustdoc 说的就是这件事，现在有机器看着了。
它们的 reach 仍写 `client`，因为那是它们做完之后该去的地方；写成别的取值等于把「还没做」记成「不该做」。

## 附记：`NodeId` 的定义模块变了，接口没变

本 crate 的 API 基线里 `kernel::plan::NodeId` 变成 `kernel::node_id::NodeId`。
**这不是一次接口变更**：公开路径仍是 `kernel::NodeId`，字段与签名一字未动，
变的只是 `cargo public-api` 记录的定义模块——`NodeId` 从 `kernel::plan` 搬进了自己的文件（kernel-SPEC §8-N）。
记在这里是因为 `apisync` 判的是「基线动了就要有一份 SPEC 同行」，而基线确实动了。

### 8-14 channels 目录化（与 protocol 同形）

`command.rs`（576）→ `command/kind.rs`（`COMMAND_NAMES`／`NoSecret`／`LoginStep`／`HaltScope`／
`PursuitStep`／`Command`）／`command/wire.rs`（`WireCommand`＋`impl`＋`From`，单测试住此）；
`server.rs`（652）→ `server/config.rs`（`ServeConfig`／路由／`ShellState` 字段开 `pub(crate)`）／
`server/reply.rs`（`Delivered`／`Reply`）／`server/socket.rs`（`upgrade` 由 reply 迁入此，
handlers 开 `pub(crate)` 供 config 挂载，单测试住此）。
跨文件私有项开 `pub(crate)`，对外签名逐字节不变。

**跨 crate 记法**：下游 `sprawling`／`web` 基线记 `channels` 内定义位簇路径
（如 `command::kind::Command`、`command::wire::WireCommand`），公共拼写不变。

### 8-15 `/enroll` 的三结局测试进程内驱动（`tests/enrolment.rs`）

`tests/enrolment.rs` 原以 `axum::serve` 端起本 crate 的路由、手写 HTTP 字节去问它，因而是 `xtask boundary` 在册的唯一越线文件。它检验的是 §8-2 的三选一（`secret_captured` 相符→201／`Reply` 拒绝→422／有界等待到期→202），三者由测试替身的工人（存／拒／沉默）分出——这是白盒问题：真二进制上 vault 只有一种下场，且 `serve` 经 `Custodian::probe` 写平台凭据服务、线格式无收回凭据的动词，黑盒重写既不可判也不可回收。故改为进程内驱动：`channels::router(&config).layer(MockConnectInfo(peer))` 后 `tower::ServiceExt::oneshot` 一发一收，peer 以 axum 给测试的那条路供给，不起 socket、不写字节。三断言原文不动；`[boundary.predating]` 归空。`tower`（`util`）只作 dev-dependency，已在 axum 之下的依赖图里，锁文件不增包。

### 8-17 `Query::Commit`：一次提交出自哪次运行（WIRE_V 13→14）

```rust
Commit { oid: GitOid },        // → Answer::Commit(CommitAnswer)，或 Answer::Unavailable

pub struct CommitAnswer {
    pub oid: GitOid,
    pub run: RunId,
    pub actor: Address,                // Sprawling-Actor：这次运行工作的那个地址
    pub model: String,                 // Sprawling-Model；空串＝那条记录没说
    pub effort: Option<kernel::Effort>,// Sprawling-Effort
    pub seq: Seq,                      // 宣告这次提交的那一行在账本里的位置
    pub at: TimeMs,                    // 那一行写下的时刻
    pub session: Option<SessionName>,  // 房间那一段：人给这条活起的名字
}
```

- **四个字段与 trailers 逐一对上，少 `Sprawling-City`**：问这个问题的人手里已经拿着城，
  把城的身份再答一遍是在回答它自己的问题。`seq` 是 trailers 携不了的那一个：
  从哪一行接着读去。
- **`session` 不是 `actor` 的复述**：派到楼根的运行没有房间，故它是 `Option`；
  有房间时它就是 `city::open_room` 当初拿这个名字开的那一段（重名时带 `-2` 后缀）。
- **一条这座城没写过的 oid 答 `Unavailable`**，与 `Changes` 同口径：「没有变化」与
  「我看不了」是两个答案，而读的人对它们的下一步不同。
- **答案从账本来，不从 git 来**（memory-SPEC §8-18）。一座导出后在别处恢复、
  `.git` 不在身边的城，照样答得出自己的历史。
- **客户端欠的（前端冻结，此处不画界面）**：`client/src/wire.ts` 需重新生成
  （`cargo xtask wire-ts --write`）；只把新变体接进
  `mount::frame` 那条「有答案而暂无页面问它」的臂，使其仍能编译。

### 8-16 线的另一端由这一端生成（`wire_schema`，feature `schema`）

**需求**：`client/`（TypeScript）要与 `crates/channels` 说同一门语言，而 §8 从头到尾只承认一个权威——Rust 的类型声明。手写一份 TS 类型就是第二个权威，它会在握手通过之后才被发现漂了。故 TS 面由这一端**生成**：`cargo xtask wire-ts` 读本 crate 的 JSON Schema，写出 `client/src/wire.ts`（每个类型一条 TS `type` 加一条 Effect `Schema` 值，外加 `WIRE_V` 与 `WIRE_HASH`）；不带 `--write` 时只比对盘上文件，第一处不同的行即门红。

**接口**（feature `schema`，缺省关；`web` 以 `default-features = false` 依赖本 crate，产品二进制不开它）：

```rust
#[cfg(feature = "schema")]
pub fn wire_schema() -> serde_json::Value;   // 一份文档：`$defs` 里是信封两端可及的每一个具名类型，
                                             // 含 `ClientFrame` 与 `ServerFrame` 两个根
```

- **哈希的素材一字不动**：`schema_hash()` 仍只吃 `WIRE_V`、命令名表、查询名表。生成的 `WIRE_HASH` 常量就是这个函数的输出，客户端在握手处送回它，服务端按原样校验——两端校验的是同一个值，而不是一个「schema 文档的摘要」；后者会把每一条 doc 注释的改动都变成一次拒配。
- **每个入帧的类型都派生 `schemars::JsonSchema`**（`#[cfg_attr(feature = "schema", derive(...))]`），派生宏读的是 serde 已经在读的属性，故形状与编码同源。kernel 侧的值经 kernel 自己的 `schema` feature 派生（kernel-SPEC §8-45）；`NoSecret` 手写 `impl JsonSchema` 为 `false`（任何值都不满足），于是 `PutSecret` 臂在 TS 里是 `value: never`——线上拼不出它，这一句在两端各说一次、意思相同。`Command<Secret>` 以 `schemars(rename = "Command")` 命名，因为线上只有 `Command<NoSecret>` 一种实例。
- **生成器只认 serde 会产出的那个子集**：对象（`properties`／`required`／`additionalProperties`）、`string`／`integer`／`number`／`boolean`／`null`、`array`（`items`）与元组（`prefixItems`）、`enum` 字符串表、`const`、`oneOf`／`anyOf`、`$ref` 指向 `#/$defs/…`、`type: [T, "null"]`、`true`／`false` 两种布尔 schema。其余一律拒绝并点名关键字与所在类型——一个会猜的生成器就是一个会静默产出错类型的生成器。具名的裸 `string`／`integer` 即 newtype，TS 侧打上 `Schema.brand(名)`。
- **文件确定**：`$defs` 按名排序后按依赖拓扑输出（Effect 的 `Schema` 值必须先定义后引用；环即拒绝），对象键排序，LF 行尾，生成头注明来源。

**被否**：（a）在 channels 用 schemars 的 remote derive 镜像 kernel 的四十个类型——每个镜像是同一形状的第二个权威，而 §8-1 第 5 条早已为 `GitOid` 拒过同一形状的提案；（b）把 schema 文档的摘要作为握手哈希——doc 注释入哈希，改一句注释即旧页面全拒；（c）手写 `wire.ts`——正是本节要关掉的那扇门。

**`answer.rs` 随之切出 `answer/building.rs`**：二十六条 `cfg_attr` 派生行把 381 行推到 407 行，越过 400 行预算，故一栋楼说自己的七个读形状（`BuildingProgress`／`BlockedLine`／`PlanRow`／`PursuitLine`／`BuildingDoc`／`ArchiveLine`／`BuildingAnswer`）迁入 `crates/channels/src/answer/building.rs`，`answer.rs` 以 `pub use` 引回，公开拼写不变；文字逐字节照搬，无字段开放。**记法同 §8-14**：下游基线里定义位路径从 `channels::answer::BuildingAnswer` 变为 `channels::answer::building::BuildingAnswer`（`web` 基线一行），那是 `cargo public-api` 记录的定义模块，不是接口变更。

**本节的公开面变更**：channels 多出 `wire_schema`（仅 feature `schema`，缺省基线不见它）；kernel 在 `--all-features` 下多出四十余条 `JsonSchema` 实现（缺省基线不见）；`web` 基线因上述路径变动重生。

### 8-18 `CommitAnswer.lineage`：一次提交背后的接替链（WIRE_V 14→15）

`CommitAnswer` 增 `lineage: Vec<RunId>`：本跑在前，逐级向前到第一任；没接替过谁的跑是长度 1 的链。名字表没动，语法换了形——正是 `WIRE_V` 存在的那种情形，于是 14→15，golden 由 `730e9d0b…` 变为 `24b7e8ff3727cad505653c951a6733b748cb294f9eec418adefb3c4a7e7223b9`。服务端从 `run_started` 的 `predecessor` 键折出 `predecessors` 表（`bin::views::commits`），答时沿表走链。客户端读它的页尚未画，`crates/web` 只需编译通过；新客户端欠一行「replaced <run>」。

### 8-18 派活帧不再携上限（WIRE_V 的第一笔，随本线一次进位）

```rust
Dispatch { addr, task, goal, mode, idem, session, effort }   // 删去 budget: BudgetCap
```

**没有人能在一件事跑之前给它定价**，所以说出「跑这件事」的那条帧不带上限。刹车只留一个：`Halt` 关掉一个范围并终止该范围里已经起来的后台成员（`runtime::backlog` 使这句话为真）。`kernel::BudgetCap` 及其判定面随之删除（kernel-SPEC §8-12），`channels` 的 kernel 再导出列表因此少一项 `BudgetCap`——**这是公开面变更**，`web` 与 `sprawling` 两份基线同变更集重生。

- **`BudgetUse` 留在再导出列表里**：成本页读它，五路归因报它。**报告花了多少**与**事前不许花**是两件事，此处只删后者。
- **`Dispatch` 的 reach 不变**（§19-2 仍是 `client`）：删的是一个字段，不是一个动词。
- **旧客户端**：`WIRE_V` 进位后在握手期被明确拒绝，所以一条仍然写着 `budget` 的帧到不了服务端；服务端也不再有那个字段可读。

### 8-19 治理两帧：写身份文件，读被代答的事（WIRE_V 的第二笔）

```rust
// Command（第 24 条）
PutDocument { which: GovernedDocument, body: String, idem: IdemKey }
pub enum GovernedDocument { Mayor, Clerk, Preferences }

// Query（第 16 条）
Governance,                       // → Answer::Governance(GovernanceAnswer)

pub struct GovernanceAnswer {
    pub autonomy: Autonomy,          // 谁来答：Owner／Delegate(resident)／Deferred
    pub decided: Vec<Decision>,      // 替这个人做掉的事，旧在前
}
pub struct Decision {
    pub item: String,                // ApprovalId
    pub verdict: PolicyVerdict,
    pub cluster: ClusterKey,
    pub at: TimeMs,
}
```

- **三份文件一条命令，不是三条**：`MAYOR.md`／`CLERK.md`／`PREFERENCES.md` 都住 `<city>/.sprawling/`（没有任何写域够得到的地方），三者的写法逐字节相同，差别只在文件名。用穷尽枚举而不是路径串：**路径由城决定，不由发帧的人决定**，否则这条命令就成了往保留子树里写任意文件的入口。
- **`Preferences` 是第三份**：市长与文书各有身份文件，而「这个人怎么喜欢这座城办事」不属于其中任何一个居民，它属于城。它与前两者同住一处、同一条命令写，因为它们被同一条规则治理：住在保留子树里，居民读得到、改不了。
- **`decided` 回答的是「你不在的时候，有谁替你答了什么」**：`approval_resolved` 折出来的流，旧在前——与 `HistoryAnswer` 同口径，因为折叠期待这个顺序。它**不筛掉人自己答的那些**：一份只列代答的清单，会让「我答过」与「从没人答」在界面上长得一样。谁答的写在 `autonomy` 里，那是同一次读的另一半。
- **`PutDocument` 不携版本**：这三份文件没有并发写入者——只有人写，而人一次只按一次保存。`edit` 工具的乐观并发管的是居民之间抢同一个文件，这里没有那回事。
- **被否**：（a）三条命令 `PutMayor`／`PutClerk`／`PutPreferences`——同一条规则三个入口，加第四份文件要改三处；（b）复用 `edit` 工具——`edit` 走写域，而写域恒不含保留子树，让它开一个例外就是把「居民改不了治自己的东西」这条最老的规矩打穿。

### 8-20 一段补丁是它自己的一次请求（WIRE_V 的第三笔）

```rust
Hunks { oid_a: GitOid, oid_b: GitOid, path: String },   // → Answer::Hunks(HunksAnswer)

pub struct HunksAnswer {
    pub oid_a: GitOid,
    pub oid_b: GitOid,
    pub path: String,
    pub lines: Vec<PatchLine>,
    pub withheld: Vec<Withheld>,
}
pub struct PatchLine { pub number: u32, pub text: String }
pub struct Withheld { pub number: u32, pub reason: String }
```

**这与 `memory::changes` 的模块头不矛盾，它就是那句话说的那次请求。** 那段头写着「计数，永不补丁文本……一段补丁必须是它自己的一次请求……而不是这个模块」。`Query::Changes` 答的是哪些文件动了、动了多少行；本查询答的是**一个文件**的补丁文本。两者不是同一个答的详略两版：前者的代价与改动文件数同阶，后者与一个文件的大小同阶，把它们并成一个答会让「看看这次改了哪些文件」付上整批补丁的代价。

- **没有它，本版开篇承诺的那件事在浏览器里做不到**：审一个 PR 得开终端敲 `git diff`。
- **同一次凭证扫描，不是第二份**：补丁文本经 `memory::checkpoint::scan_staged` 的同一个判定过一遍。命中凭证形状的那一行**不回显**，答里只留它的行号与原因（`Withheld`）。第二份扫描器就是同一条规则的第二个权威，而漂掉的那个总是没人读的那个。
- **一次一个文件**：`path` 是必填的，没有「整批补丁」这个形状。
- **两个 oid 都不可变，所以这个答任何人都可以永久缓存**（同 `Changes` 的理由）。
- **这座城没写过的 oid 答 `Unavailable`**，与 `Changes`／`Commit` 同口径。

### 8-21 四种读法回到服务端（WIRE_V 15→16）

```rust
// Query 第 18、19、20 条（声明序，QUERY_NAMES 同序追加）
Rounds   { run: RunId },      // → Answer::Rounds(Box<RoundsAnswer>)
Evidence { run: RunId },      // → Answer::Evidence(EvidenceAnswer)
CostOf   { node: NodeId },    // → Answer::CostOf(CostOfAnswer)

pub struct RoundsAnswer {
    pub run: RunId,
    pub turns: Vec<Turn>,
    pub opened_at: Option<GitOid>,   // 本会话的第一道栅栏
}
pub struct Turn {
    pub number: u32, pub opened: Seq,
    pub said: Option<String>, pub spent: Option<UsdMicros>,
    pub used: Option<Used>, pub stopped: Option<String>,
    pub calls: Vec<Call>, pub notes: Vec<Note>,
}
pub struct Call { pub tool: String, pub subject: Option<String>,
                  pub outcome: Outcome, pub at: Seq, pub output: Option<Output> }
pub enum Outcome { Waiting, Answered, Failed }
pub struct Output { pub head: String, pub cut: usize }
pub struct Used { pub input: Tokens, pub output: Tokens, pub cached: Tokens }
pub enum Note { Refused { error: AxError, at: Seq }, Fenced { oid: GitOid, at: Seq },
                Waiting { at: Seq }, Arrived { from: String, said: String, at: Seq },
                Discarded { count: usize, at: Seq } }

pub struct EvidenceAnswer { pub run: RunId, pub items: Vec<EvidenceItem> }
pub struct EvidenceItem { pub at: Seq, pub kind: EvidenceKind,
                          pub locator: Locator, pub picture: Option<Picture> }
pub enum EvidenceKind { Screenshot, Finished }
pub struct Picture { pub media_type: String, pub width: u32, pub height: u32 }

pub struct CostOfAnswer { pub node: NodeId, pub spent: UsdMicros,
                          pub runs: Vec<(RunId, UsdMicros)> }
```

**这里搬的是读法而不是接口。** 一个会话被读成回合、一次跑留下什么证据、一个计划节点花了多少钱——这三件事此前只有 `crates/web` 会算，于是「线就是全部 API」（ARCHITECTURE §8）在这三处是假的：另写一个客户端就得把折叠逻辑照抄一遍，而照抄出来的那一份迟早与这一份不一致。现在三者各是一次查询，答由 `bin::views` 折出（sprawling-SPEC §8-47）。

- **`Rounds` 的值类型住 `channels`，折叠住 `bin::views`。** 值要上线，故必须可序列化；折叠要读账本，故必须在能读账本的那一层。两者切分开来，正是 ARCHITECTURE §9 的形状 2 与形状 7 的分界。
- **`channels::reading` 是第三块**：把一条账本载荷读成上面这些值的那些纯函数（`said_in`／`used_in`／`output_in`／`subject_of`／`note_of`）。它住在线这一层而不是服务端，因为**两端都要读**：服务端答 `Rounds` 要它，客户端把推来的 `model_returned` 折进自己的快照也要它（ARCHITECTURE §5 第 12 步：同一个折叠，线的两边）。一份权威，两个调用者。
- **`Changes` 早已在线上**（§8-20），`memory::changes` 一直是它唯一的权威；查过之后不动它——把一件已经做完的事再做一遍就是造第二个权威。
- **`Evidence` 只认写下来的东西**：截图是 `tool_result` 载荷里的 `image` 定位符（`bin::browser_tool::stored` 写的那三项：定位符、两条边、media type），完成证据是 `roadmap_finished` 载荷里的 `evidence` 定位符。**答里恒不携字节**：一张图是一个 `cas:` 定位符，取它是资产端点的事，把 base64 塞进查询答会让「看一眼这次跑干了什么」付上整批像素的代价——与 §8-20 拒绝整批补丁同一条理由。
- **`CostOf` 的分母不在这里**：答只报这个节点上归到的绝对金额与逐跑明细，不报占比。占比需要一个这一端没有的分母（整城总额是 `CostView` 的），而没有分母的百分比正是 `UnplannedProgress` 拒绝拼出来的那种东西。节点到跑的映射由 `roadmap_claimed` 折出（载荷里的 `node` 与记录的 `addr`），钱由 `memory::attribution` 的 `by_run` 给——**不新增任何计价处**。
- **一个本城没认领过的节点答 `CostOf { spent: 0, runs: [] }` 而不是 `Unavailable`**：与 `Changes` 那一条相反，因为这里「没人认领过它」是一个真答案而不是「我读不了」；节点地址本身经 `NodeId` 的手写 `Deserialize` 把过关，读不了的形状根本上不了线。
- **`WIRE_V` 15→16，一次进位管三条查询**：名字表长了三项（17→20），故 schema 哈希无论如何都要变。旧页面在握手期被明确拒绝，这正是该机制存在的理由。
- **被否**：（a）把 `Rounds` 并进 `RunHistory` 的答——前者是折叠后的读法，后者是原始记录页，一个答两副形状会让翻页与折叠互相牵制；（b）让 `Evidence` 直接回字节——见上一条；（c）把折叠留在 `channels` 里由客户端调用——那样新客户端仍要自己跑一遍折叠，而这一节整件事就是不要它这么做。

### 8-22 没有监听器的那份构建，测试也不许提它（`tests/enrolment.rs`、`tests/wire_contract.rs`）

`cargo clippy -p channels --no-default-features --all-targets` 是红的：`tests/enrolment.rs` 整份都在驱动 `channels::router`，`tests/wire_contract.rs` 有三条断言在问 `decide_bind`／`decide_handshake`，而这三样连同 `axum`、`tokio` 都由 feature `server` 带进来。**这份构建正是给 `web` 用的那一份**——它需要本 crate 的词汇而不许把 TCP 栈拖进 WebAssembly；一个在这里名词都拼不出来的测试文件，把它自己的红判在了产品的一条真路径上。

**按测试真正需要的东西设门，而不是把 feature 打开**：`tests/enrolment.rs` 首行 `#![cfg(feature = "server")]`（整份文件都是路由的事）；`tests/wire_contract.rs` 只给那三条断言与它们的两个辅助函数、以及 `Hello`／`Welcome`／`AxCode`／`SocketAddr` 这几个只被它们用到的名字加 `#[cfg(feature = "server")]`——命令表、查询表、schema 哈希与那两个不可拼写的形状**在两份构建里都被判**，因为它们在两份构建里都成立。

### 8-29 一个端点带着人给它定的规矩上线（WIRE_V 24→25）

```rust
pub struct EndpointTuning {
    pub label: Option<String>,               // 显示名，缺省即 id
    pub timeout_ms: Option<u64>,             // 一次已结请求的期限
    pub request_max_retries: Option<u32>,    // 值得再问一次的失败再问几次
    pub stream_idle_timeout_ms: Option<u64>, // 一次流式请求的期限（命名同 Codex）
    pub headers: Vec<HeaderPair>,            // { name, value }，value 可为 `secret:` 引用
    pub overrides: Vec<BodyOverride>,        // { pointer, value }，JSON pointer → 值的文本
    pub proxying: Option<Proxying>,          // 这个端点的调用走不走这台电脑的代理，缺席即城自己的规则
}

ProbeEndpoint  { name, base_url, dialect, secret, auth_header, tuning: EndpointTuning, idem }
AttachEndpoint { name, base_url, dialect, secret, auth_header, admit, tuning: EndpointTuning, idem }
EndpointSummary { name, label, base_url, dialect, models, local, has_credential }
```

- **一个值而不是六个字段**。它们在同一张表单上被填，被同一次调用一起读；分开传就给了 probe 与它之后的 attach 三次机会对「我们在跟什么说话」产生分歧——`Entered` 当初收成一个值正是这个理由。
- **probe 也带 tuning**。一个需要自定义请求头的网关，在 probe 不带那个头时答 401；人于是读到「密钥无效」，而那把密钥是好的。probe 与 call 因此按同一套头、同一个期限发出。
- **覆盖的值走文本，不走 `serde_json::Value`**。`Command` 派生 `Eq`，而 JSON 没有全序相等；更要紧的是 `/temperature` → `0.2` 一旦成为值就是一个浮点，而它随 `endpoint_attached` 进账本——这座城把浮点挡在账本之外。文本原样往返，「这段文本作为 JSON 是什么」只有一个权威：`gateway::EndpointTuning::applied_overrides`，规则是「解析得出就是那个 JSON，解析不出就是它看上去的那个字符串」，于是 `/reasoning/effort` → `high` 不必要求人自己加引号。**败给的方案**：帧上直接放 `Value`——那要求 `Command` 放弃 `Eq`，并把浮点写进账本。
- **零即缺省**。清空一个数字框到达线上是 `Some(0)`，而没有请求能在 0 ms 内完成；装配层把零读成「没说」（`bin::assembly::credentials::tuning_of`），于是清空一个框等于回到城自己的值，而不是让此后每一次调用立刻失败。
- **`stream_idle_timeout_ms` 在线上保留 Codex 的名字，在 gateway 里叫 `stream_deadline_ms`**：阻塞传输交回的是一个没有分块钩子的 body reader，城因此能限定一次应答总共多久，限定不了其中某一次沉默多久。线上用人在自己 `config.toml` 里写熟的那个词，gateway 用它真正做到的那件事命名，装配层是唯一的翻译点。**败给的方案**：在 gateway 里也叫 idle——那会让一个读代码的人以为分块之间有计时器。
- **`EndpointSummary` 长出 `label`**：缺省即 `name`，所以页面永远不必替一个没写显示名的端点决定显示什么。
- **`proxying` 跟着 tuning 走，因而探测与调用恒用同一个决定**（WIRE_V 27→28）。一个只在调用时生效的代理设置，会让表单上那份分段读数描述一条真正的调用不会走的路，而那份读数存在的全部意义就是告诉人调用停在了哪一段。`Option` 而非值：线上的缺席是「没人定过」，装配层把它翻成城的默认值（`ExceptLocal`），于是 `gateway` 一侧拿到的是一个已经定下来的值，没有第三种状态要每一个调用方再答一次。

### 8-28 `endpoint_probed` 答的是一次读数，不是一次成败

```jsonc
{ "name": …, "base_url": …,
  "reach": { "host": …, "named": …, "connected": …, "answered": …, "through": …, "elapsed_ms": … },
  "models": ["id", …],
  "facts":  [{ "id", "context_tokens"?, "max_output_tokens"?, "input_modalities", "input_price"?, "output_price"? }, …],
  "failed": { "code": …, "subject": … }   // 仅当模型表读不出来
}
```

- **读不出模型表的 probe 照样作答**。名字解析不了、端口没人应、证书不受信、供应方答 401，对填表的人是四个不同的下一步；作为一次拒绝返回，它们在界面上塌成传输库的一句话。记录带上停在哪一段（`kernel::Reach`，由 `gateway::reach` 量出），再把拒绝自己的 code 与 subject 放在旁边。
- **`models` 与 `facts` 同时在**。只要 id 的客户端不必读 facts；要显示上下文窗口与输出上限的表格不必第二次发问。读不出来时两者都是空数组而 `failed` 在场，于是「表是空的」与「表读不出来」在形状上可分。
- **没有被应答说出来的数字，记录里也没有**。`/models` 的一行里有什么由供应方决定；城不替它补零、补默认、补猜测——补出来的数字会盖过真正计费的那个。

### 8-27 说出来的那句话：`/transcribe` 与 `ModelTag::Transcribe`（WIRE_V 20→21）

```rust
pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;
// POST /transcribe，body 是录音字节，content-type 是浏览器录进的容器；200 的 body 就是那行文字。
```

- **是一条路由，不是一条 Command，也不是一条 Query**。Command 被接下之后经事件流作答，而「我刚说的那句话是什么」必须回到录它的那个标签页；Query 是另一种会作答的形状，而一条在供应方那里花掉数秒的查询就是一条装成读的命令。`/enroll` 与 `/upload` 早已是同一类旁门：帧的文法装不下的那几件事各有一扇 HTTP 门。
- **容器从请求头读，不从字节猜**：浏览器录进它手上有的容器，而只有它知道是哪一个。没有 content-type 即按名拒绝——一个没人声明的容器发不出去。`; codecs=opus` 这类参数说的是容器里的编解码器，而音频线路由的是容器，故取分号前那一段。
- **`ModelTag` 增第三个 `Transcribe`**（kernel-SPEC 的枚举表同步）：**「哪个 endpoint、哪个 model 答这一类活」本来就有机制**——人登记一个 endpoint，再为一个 tag 选一个 model。第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。人填 URL 与 key 因而走的是既有的 attach 表单。
- **服务端**：`gateway::transcriber_for(chosen, secrets)` 与 `adapter_for` 同形——把一个选择变成一件可调用的东西这件事只在一处发生。`Views::transcriber` 在锁内读出选择、锁外发请求。

### 8-26 `ConfigureBuilding` 长出 `desktop`：一栋楼的桌面白名单走同一条帧（WIRE_V 19→20）

```rust
ConfigureBuilding { addr: Address, sandbox: Option<SandboxLimits>, mcp: Option<Vec<McpServer>>,
                    desktop: Option<String>, idem: IdemKey },
```

- **不新起一条命令，也不给 `GovernedDocument` 加变体**。这条帧问的本来就是「这栋楼的 runs 够得到什么」——沙箱、外部服务器、运行中的机器上的哪些窗口，是同一个问题的三面；各自可缺省，缺省即不动那一面。而 `GovernedDocument` 是**城**的三份文件（`<city>/.sprawling/`），桌面白名单是**楼**的（`<building>/.sprawling/`）：把楼级路径塞进一个按 city_root 取路径的枚举里，会让那个枚举需要一个只有部分变体用得上的参数（city-SPEC §8-26 已写下这条）。
- **`desktop` 是文本而不是解析过的值**。读它的那台 server 是它语法的权威，且 fail closed——读不出来的文件关成全拒。城这一侧再抄一份解析器就是第二个权威，而两个权威里迟早有一个把某份文件读成另一种意思。城只保证「写进去的字节就是人给的字节」。
- **`building_configured` 载荷第三个布尔位 `desktop`**：与 `sandbox`／`mcp` 同形，说的是「这一面被写过」而不是写了什么。`city::Written` 把三个布尔收成一个值——一个调用点写 `(true, false, true)` 说不出哪一位是哪一面。
- **页面读它用 `Query::Document`**：`<building>/.sprawling/DESKTOP.toml`。那条查询本来就明说保留子树在这里可读，理由是「治理一栋楼的东西正是这个视图要给人看的」。

### 8-25 `Query::Doctor`：运行中的机器有什么（WIRE_V 18→19）

```rust
// Query 第 24 条（声明序，QUERY_NAMES 同序追加）
Doctor,                                   // → Answer::Doctor(Box<DoctorAnswer>)

pub struct DoctorAnswer { pub items: Vec<DoctorItem>, pub tiers: Vec<DoctorVerdict> }
pub struct DoctorItem { pub name: String, pub tier: DoctorTier, pub need: DoctorNeed,
                        pub enables: String, pub state: DoctorState, pub install: DoctorInstall }
pub enum DoctorTier { Use, Develop }
pub enum DoctorNeed { Required, Optional }
pub enum DoctorState { Present { at, version }, Broken { at, fault }, Absent { absence } }
pub enum DoctorVersion { Said { text }, Silent, Unreadable, Late }
pub enum DoctorFault { WillNotStart { said }, HalfWritten, Unreadable { said } }
pub enum DoctorAbsence { NotOnSearchPath, VariableNamesNothing { variable, path },
                         NoComponent { dir }, NoHome, NotInThisBuild }
pub enum DoctorInstall { Command { spelled }, Print { spelled }, Manual { how }, UnknownPlatform }
pub struct DoctorVerdict { pub tier: DoctorTier, pub missing: Vec<String> }
```

- **每一种状态都是枚举，不是句子**。终端那份报告是一台机器的散文，而浏览器说两种语言；线上若携措辞，页面的用词就成了服务端的选择。唯一的例外是 `enables`——那是需求表自己关于「有了它能做什么」的一句话，读者推不出来，这条答案里也没有别的字段装得下它。
- **答的是城启动时看到的那一眼，不是现问现看**。每一项都是起一个进程问版本；一次查询若这么做，会把答一切读的那条线程按住数秒。城若没看过（一次一条命令驱动的工人就是），答 `Unavailable`——与「一栋没人盖过的楼」同口径：**「我没看」是它自己的答案**，而一台空机器会让页面告诉人他手上每件工具都缺。
- **`install` 把平台不明单列一支**。三个平台之外的机器上，本项目没有任何配方；此时拼一条别的平台的命令是错的，沉默也是错的。
- **服务端**：`bin::doctor::report` 把 findings 折成本形状，`Views` 存一份（sprawling-SPEC §8-54）。

### 8-24 `Query::Commits`：一座楼做过的提交，倒序分页（WIRE_V 17→18）

```rust
// Query 第 23 条（声明序，QUERY_NAMES 同序追加）
Commits { building: Option<Address>, before: Option<Seq>, limit: u32 },   // → Answer::Commits(CommitsAnswer)

pub struct CommitsAnswer {
    pub building: Option<Address>,   // 问题里的那个，原样回带
    pub before: Option<Seq>,         // 同上
    pub commits: Vec<CommitAnswer>,  // seq 递减
    pub more: bool,                  // 末条之前还有没有符合条件的提交
}
```

- **答案复用 `CommitAnswer`**：一个提交是什么只有一处权威，列举与反查因而不可能给出两套字段；`lineage` 逐条照答，楼页据此画接替标记。
- **倒序分页同 `History`**：`before` 是独占上界（`None` 从尾读起），`limit` 夹到 `1..=HISTORY_MAX`。分页游标是 `commits.last().seq`：下一页问 `before: Some(那个 seq)`。`more` 而不是 `earlier: Option<Seq>`——`History` 按 seq 连续扫描，所以「从哪接着读」是它自然算出的量；提交在账本里是稀疏的，下一条在哪只有再走一步才知道，而客户端手里已经有末条的 seq，答一个游标就是把它已有的东西再给一遍。
- **`building` 按 `actor` 地址前缀过滤，不按 session**：`actor == building` 或 `actor` 以 `<building>/` 起头；run 的 actor 是权威，session 是它的投影（`session_of`），反过来过滤会丢掉派到楼根、没开过会话的 run。`None` 列全城。
- **答案回带 `building` 与 `before`**：线上没有请求 id，客户端按内容把答案配回问题（`ChangesAnswer` 回带 `base`／`head` 是同一个理由）；缺了这两个字段，两座楼的两页同时在飞时无法分辨谁是谁的。
- **失联如实**：只列城自己写过的提交；人 rebase／squash 之后 trunk 上的 oid 不在其中，`Commit` 对它仍答 `Unavailable`。不读提交体的 trailer 回填——那会让投影成为第二权威（`views/commits.rs` 模块头）。
- **服务端**：`views::holding` 给按 oid 键的 `commits` 表加一条按 `seq` 的索引（`commit_seqs: BTreeMap<Seq, GitOid>`），`fold_commit` 两表同写；sprawling-SPEC §8-53。
- **`CommitAnswer` 长出 `at: TimeMs`**（同版内，第二条提交）：宣告这次提交的那条记录自己的 `t`。理由来自第一张截图——一列 seq 没法扫读，而一列时间可以。与 `Opening.at`／`Closing.at` 同源、同型。
- **客户端**：`cargo xtask wire-ts --write` 重生。

### 8-23 新客户端第一次真正用这条线，线上缺的四件事（WIRE_V 16→17）

```rust
// RunSummary 多两个字段（memory::RunHot 从 run_started 记下，memory-SPEC §8-5）
pub struct RunSummary { pub run: RunId, pub who: String, pub frozen: bool,
                        pub last_seq: Seq, pub last_kind: EventKind,
                        pub addr: Option<Address>, pub started: Option<TimeMs> }

// CityAnswer 多一个字段：被 halt 的 scope 名（`city`、`<building>`、`<workshop>`），BTreeSet 序
pub struct CityAnswer { ..., pub halted: Vec<String> }

// RoundsAnswer 多开场与收场
pub struct RoundsAnswer { pub run: RunId, pub turns: Vec<Turn>, pub opened_at: Option<GitOid>,
                          pub opening: Option<Opening>, pub closing: Option<Closing> }
pub struct Opening { pub task: String, pub goal: String, pub at: TimeMs }
pub struct Closing { pub completion: String, pub at: TimeMs }

// Query 第 21、22 条（声明序，QUERY_NAMES 同序追加）
Listing  { at: Option<Address> },   // → Answer::Listing(ListingAnswer)；None 是城根
Document { at: Address },           // → Answer::Document(Box<DocumentAnswer>)

pub struct ListingAnswer { pub at: Option<Address>, pub entries: Vec<Entry> }
pub struct Entry { pub name: String, pub kind: EntryKind }
pub enum EntryKind { Directory, File { bytes: u64 } }          // 目录在前、文件在后，各按名字序
pub struct DocumentAnswer { pub at: Address, pub text: String, pub bytes: u64,
                            pub truncated: bool, pub binary: bool }
```

**这四件事都是同一个发现**：ARCHITECTURE §8 说「线就是全部 API」，而旧客户端从没把这句话当真——它在浏览器里折叠 `history` 的原始记录，所以从来没问过线「这次跑在哪个房间」。新客户端只问线不折历史，四处空白一次全露出来。

- **`RunSummary.addr`／`started`**：`who` 是这次跑第一条记录的作者，恒为 `city`，不是房间。房间是 `run_started` 记录自己的 `addr`，热视图在那一条上记下它（memory-SPEC §8-5）。没有它，页面无法把 `city_view` 列出的 run 归到 `hall/mayor`，「与 Mayor 的对话」拼不出来。`Option`：热视图可能只看到没有开场的一段尾巴，看不到的事不猜。
- **`CityAnswer.halted`**：`city_halted` 是记录，`halted_by` 是 `bin::assembly` 工作线程的判定，而页面刷新后两者都够不到——它只收此后的事件。答里带上被 halt 的 scope 名，一个刚打开的页面才知道城是不是停着的，而不是等下一次 dispatch 被拒才发现。名字与 `HaltScope` 的 `scope_name` 同拼法，页面按名字画。
- **`RoundsAnswer.opening`／`closing`**：回合的折叠从第一条 `model_called` 开始，所以人说的第一句（`run_started.task`）与这次跑怎么结束的（`run_frozen.completion`）都不在答里；一段对话缺开头与结尾就不是对话。两个都是 `Option`，理由同 `addr`：`HISTORY_MAX` 那段窗口可能不含开场。
- **`Listing`／`Document`**：这座城是一棵目录树，而目录树本身就是产品（glossary：「那个层级就是目录树——不是它的模型，是树本身」）；`building_view` 只回楼根的 `.md` 与房间名，房间里的 `URBANITE.md`／`JOB.md`／`Handoff.md`／`<run>.jsonl` 页面看不到，于是这个设计在界面上是不可见的。两条查询让页面能走完整棵树。**路径经 `Address` 文法把关**（非绝对、无 `..`、无 `\`、无 `:`），所以走不出城根；`.sprawling/` **允许读**——它正是要展示的那部分，且这条线只答回环（或持配对 token 的）人，与工具层对居民的拒绝不是一个门。`Document` 上限 64 KiB 与 `BuildingDoc` 同（`DOC_BYTES_MAX`），截断必说；头 8 KiB 里出现 NUL 字节判 `binary`，`text` 留空——把 redb 或 CAS 的字节当文本喷到页面上是撒谎。文件不存在答 `Unavailable { query: "Document(<at>)" }`，与 `BuildingView`（没人立过的楼）、`Changes`（本城没写过的 oid）同口径：「我读不了」是一个真答案，与空文件不同。
- **`WIRE_V` 16→17，一次进位管四件事**：四件事同一提交同一哈希。
- **被否**：（a）让客户端自己折 `history` 找 `run_started`——那是旧客户端的做法，也是这四处空白存在的原因；（b）`Document` 直接回任意大小——同 §8-20／§8-21 拒绝整批的理由；（c）`Listing` 排除 `.sprawling/`——排除了要展示的东西。

### 8-34 一台工具服务器带着 `claude mcp add` 允许写的东西上线，`McpHealth` 答它站在哪（WIRE_V 26→27）

`McpTransport` 先前只拼得出「命令＋参数」与「地址＋至多一个 header」，于是 MCP 页上环境变量表、多行请求头、`sse` 三样控件各挂一句「线上没有这个字段」。三支取值补齐，且**关掉 `#[non_exhaustive]`**：本枚举的读者全在这一个二进制里，通配臂什么也换不来，却会把下一种 transport 从必须表态的三个模块面前藏起来。

```rust
pub enum McpTransport {
    Stdio { command: String, args: Vec<String>, env: Vec<(String, String)> },
    Http  { url: String, headers: Vec<(String, String)> },
    Sse   { url: String, headers: Vec<(String, String)> },
}

// Query 第 29 条（声明序，QUERY_NAMES 同序追加）
McpHealth { addr: Address },              // → Answer::McpHealth(Box<McpHealthAnswer>)

pub struct McpHealthAnswer { pub addr: Address, pub servers: Vec<McpServerHealth> }
pub struct McpServerHealth { pub label: ServerLabel, pub transport: String,
                             pub target: String, pub state: McpState }
pub enum McpState {
    Connected { protocol_version: String, server: String, tools: Vec<McpToolLine> },
    Authenticating { recovery: String },
    Failed { refusal: Box<AxError> },
}
pub struct McpToolLine { pub remote: String, pub name: String,
                         pub disclosure: String, pub input_schema: Payload }
```

**五条口径：**

1. **名与值成对而不是一行 `"Name: value"`**。旧写法把「怎么切这一行」留给了读者，而一个值里合法地含冒号；成对之后拆分这件事不存在，写的人与读的人也不必约定同一条切法。值可以是 `secret:realm/name` 引用，兑付点在离线最近的那一格（sprawling-SPEC §8-4／§8-15）。
2. **`Sse` 是自己的一支而不是 `Http` 的一个开关**。两者开法不同、败法也不同：http 服务器拒绝一次请求，而流式服务器可以接下每一次请求却一次也不答。
3. **三种状态穷尽，而不是一个标志加一句可选的理由**。「在答」「要登录」「失败了，原因在此」是人接下来要做的三件不同的事；理由可选的形状会让一次失败不带理由地上线。`Authenticating` 的判断只有一个来源——传输层对 401／403 抬的 `E_CREDENTIAL_MISSING`，故这里没有什么要猜。
4. **失败携整条三段式拒绝**，界面逐字画出：措辞的权威在城里，页面再写一遍就是第二个权威。
5. **这是唯一一条按秒计的读**，每台服务器一次 `initialize` ＋ `tools/list`，且用的就是 Run 起点那一次握手（`protocol::mcp`）。它恒不由记录触发、也不上定时器——人打开 MCP 页或加完一台服务器时问一次。**被否**：把握手结果写进账本再折出来——一台服务器此刻通不通是关于此刻的事实，记下来的那一份会在它停掉一小时后仍说它在。

### 8-32 第三类之外的第三类：`ServerFrame::Log`（WIRE_V 的一笔）

**日志不是历史，而「不是历史」不等于「不给人看」。** `docs/logging.md` 把账本与日志分得很干净：账本答「发生了什么」、权威、进重放；日志答「当时在想什么」、不权威、随时可丢。记录页的第四个透镜先前是一个空态，理由写在页面上——没有任何帧携得动一行日志。

于是 `ServerFrame` 多一个取值，形状口径与 `Delta`（§8-8）逐条相同：

```rust
pub enum ServerFrame { Welcome(..), Event(..), Answer(..), Refusal(..), Delta(..), Log(LogLine) }
pub enum LogLevel { Refuse, Effect, Decide, Trace, Wire }   // 五个名字取自 docs/logging.md，不另起
pub struct LogLine {
    pub seq: Seq,             // 写这行时账本站在哪；不是本流自己的序号
    pub t: Option<TimeMs>,    // 机器写它的时刻；读不到钟即缺席
    pub level: LogLevel,
    pub module: String,
    pub run: Option<RunId>,   // 城自己说话时缺席
    pub line: String,
}
```

**四条口径：**

1. **`seq` 不是序号而是锚**。两行可以共用一个 `seq`，客户端据此把日志摆到账本旁边，而不是据此发现自己漏了一条。漏一条日志什么也没丢——这正是它可以有自己一类帧的理由，与增量同源。
2. **第三条广播通道**。丢弃语义与事件相反、与增量相同：`wire` 层底的一座城写得比人读得快，共用事件通道会把历史挤出慢读者的窗口。`RecvError::Lagged` 一言不发地略过。
3. **`t` 可缺席而行不可丢**。采样在装配层（`docs/logging.md` §8 认可的唯一处），而写日志的库不许有第二个时间源。钟读不出来时，锚仍是 `seq`，为了一个时间戳丢掉整条诊断是把代价付错了地方。
4. **五个等级的名字只有一个权威**。本 crate 依赖图上够不到 `runtime`，所以 `LogLevel` 是第二处拼写而不是第二处**权威**：两者的映射住在装配层（`sprawling::serving::journal`），一条测试把五个 serde 名与 `Level::as_str()` 逐个钉成相等。**被否**：把 `Level` 上移进 `kernel`——那会让 `docs/logging.md` 说的「`runtime::diagnostics` 实现本文」不再成立，为一个枚举搬走一条设计权威。

### 8-33 `DoctorInstall` 与 `DoctorRefresh`：机器上的两个动词（WIRE_V 的同一笔）

`Query::Doctor` 答的是开城那一刻的快照（§8-25），于是机器页只能复制一行命令去终端跑，跑完还得重启城才看得见结果。两条命令补上这段：

```rust
Command::DoctorInstall { item: String, idem: IdemKey }   // 按需求表里的名字装一件
Command::DoctorRefresh { idem: IdemKey }                 // 重新探一遍，取代启动快照
```

- **只跑 `Recipe::Command`**。`Print` 与 `Manual` 各自带着「人自己去做什么」被拒——管道进 shell 的脚本是没人读过的代码，这条纪律不因为请求来自页面而不是终端就松一格。执行走的是终端那条 `Machine::install`，不是第二个安装器。
- **进度就是日志行**。安装是本城起的一个进程并等它，值得报告的两件事——将要跑什么、怎么结束的——正好是一行日志的形状。第二条进度通道会是同一件事的第二个权威。
- **`DoctorInstall` 装完自己再探一遍**，而不是让页面记得补一帧：装完仍答启动快照的城，会告诉人他刚装的东西还是没有。
- **两者都不入账本**。机器有什么不是这座城里发生的事：它在本进程之外被改变，写进历史就是写进一份会错的历史。答案沿 `RunWorker::examine` 交给服务层的 views，与开城那一次的写法同一条。
- **为什么不是 `Query::Doctor { fresh: true }`**：读要拿着 views 的锁答，而探测是十几个进程各被起一次的几秒钟；那会让一次读把其他每一次读都堵住。命令在写线程上跑，那里本来就是本城把工作排成一列的地方。

### 8-35 四条读：一个 agent 被告知了什么、一栋楼会做什么、它那里还有什么没提交（WIRE_V 23→24）

先前线上没有任何一帧答得出「这个 agent 收到的 system prompt 是什么」。`prompt_assembled` 只记四段的哈希与来源注记，而其中两段根本没有落进内容仓库，于是哈希指向不存在的字节——一个人拿着哈希也读不回原文。技能同理：`Query::RegistryView` 的书里没有 skill，楼页没有一栏画得出「这栋楼会做什么」。工作树同理：`Query::Changes` 比的是两个检查点，而没有一帧答得出分支名、与上游的距离、以及此刻哪些文件还没进检查点。

```rust
// Query 第 25–28 条（声明序，QUERY_NAMES 同序追加）
Prefix    { run: RunId },           // → Answer::Prefix(Box<PrefixAnswer>)
Content   { locator: Locator },     // → Answer::Content(Box<ContentAnswer>)
Skills    { building: Address },    // → Answer::Skills(Box<SkillsAnswer>)
GitStatus { building: Address },    // → Answer::GitStatus(Box<GitStatusAnswer>)

pub enum PrefixSlot { City, Building, Resident, Run }
pub struct PrefixSource  { pub addr: Address, pub kept: u64, pub dropped: u64 }
pub struct PrefixSegment { pub slot: PrefixSlot, pub hash: B3Hash, pub bytes: u64,
                           pub text: String, pub stored: bool,
                           pub sources: Vec<PrefixSource> }
pub struct PrefixAnswer  { pub run: RunId, pub segments: Vec<PrefixSegment> }
pub struct ContentAnswer { pub locator: Locator, pub text: String, pub bytes: u64,
                           pub truncated: bool, pub binary: bool }

pub enum SkillShelf { Library, Building }
pub struct SkillLine  { pub name: String, pub section: String, pub shelf: SkillShelf,
                        pub at: Address, pub disclosure: String, pub hash: B3Hash,
                        pub admitted: bool, pub pinned_by: Vec<RunId> }
pub struct SkillsAnswer { pub building: Address, pub skills: Vec<SkillLine>,
                          pub missing: Vec<String> }

pub struct Drift { pub ahead: u64, pub behind: u64 }
pub struct GitStatusAnswer { pub building: Address, pub branch: Option<String>,
                             pub drift: Option<Drift>, pub files: Vec<FileChange>,
                             pub checkpoint: Option<CommitAnswer> }
```

**六条口径：**

1. **`stored` 与空文本是两件事。** 仓库被清理过与这一段本来就没有内容，读者下一步做的事不同；用空串同时表示两者，会让一次数据丢失看起来像一次正常的装配。
2. **`Content` 只答 `cas:` 一种方案。** `file:` 指的是树上的一条路径，那是 `Query::Document` 的问题；在这里再答一次就是同一条规则的第二个权威。**被否**：让 `Content` 按方案分流兼收两种——它会把「读一个对象」和「读一个文件」的失败面合成一个，而两者恢复动作不同。
3. **`PrefixSource.dropped` 恒上线，即使是零。** 「一个字节都没裁」是一次测量，缺省的字段不是。
4. **技能一行而两个书架**，`shelf` 说它来自哪一格：两张表会让「近的架子压过远的架子」这条既有规则在客户端被重写一遍。`pinned_by` 按**名字加哈希**成对匹配——两次 run 之间被编辑过的 skill 是同一个名字下的两份文档，只按名字匹配会宣称早先那次 run 读到了后来才写的字。
5. **`Drift` 整个可缺席，而不是两个零。** 没有上游的分支与和上游齐平的分支不是一回事，读成 `0/0` 的页面会告诉人「你的工作已经推上去了」。
6. **`GitStatusAnswer.checkpoint` 携整条 `CommitAnswer`。** 变更栏旁边那一行要说出 run、房间、模型与花费，而这四样已经有了唯一形状；另造一个摘要类型就是第二个「一次提交是什么」。

**`CommitAnswer` 随本线长出 `spent: UsdMicros`**：一行提交上先前画得出 run、房间与模型，独独没有钱，于是「这次改动花了多少」必须另开一页去查。携的是**那次 run 的总额**而不是这条提交的份额——栅栏不被计价，把一次 run 的钱按栅栏分摊会得到一个没有人测量过的数字。

**被否**：把 skill 与工作树的状态折进 `BuildingView`。楼页的那一帧是在每一次记录之后都会失效的读，而扫书架要走盘、读工作树要开仓库；合成一帧会让这两件慢事按城里的心跳重复发生，而它们各自只在有人打开那一栏时才需要一次。
