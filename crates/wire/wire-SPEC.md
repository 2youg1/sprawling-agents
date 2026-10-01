# wire-SPEC.md

> crate：`wire`（lib，依赖 kernel）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：十七节；按模块分章、每章自足。
> 模块：wire（含 command／answer／carried_name／named_frames）／server（含 reception／assets）／control／auth／aggregate／preference／reading。
> 本 crate 覆盖的语义：wire 面（Command／Query／Event、编码与握手、绑定面）、干预动词、多台机器一个界面、Autonomy 应答者、三队列。

## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `frames` | Command／Query／Event 三分的类型与 JSON 编码；版本＋schema 哈希握手帧 |
| `server` | WebSocket 服务；静态资源；绑定面判定（默认只绑回环） |
| `auth` | 回环零摩擦；非回环要求配对令牌且常数时间比较；未配置令牌即拒绝启动 |
| `control` | 人的干预动词入口；自持鉴权与幂等（做不到则并入 `server`——ARCHITECTURE §6 已写明这条退路） |
| `aggregate` | 多 City 只读聚合：只转发 Query 与 Event，恒不转发 Command |
| `reception` | 一帧进来之后的判定：读不出的帧、会动作的门先问配对（§8-37、§8-40） |
| `preference` | 客户端读的那几张偏好枚举，值集在这里生成 |
| `reading` | 一次回合的读法回到服务端（§8-21） |

**本 crate 是进程外边界的唯一守卫**。它不实现任何业务判定：Command 的执行、Query 的求值、Event 的产生全在上游（runtime／storage／city），本 crate 只负责「让非法的帧在类型层或握手层就不存在」。

## 2 验收标准

- **wire**：Command 恰 33 个 variant、Query 恰 40 个（计数断言；两张名表由 `named_frames!` 从变体表生成，故计数断言核的是「变体数没被无声改动」，不再是「两张手写表与枚举是否一致」——见 §8-38）；每个改状态 Command 携 `IdemKey`（类型强制，无可省字段）；`PutSecret` 的 `value: Sealed<String>` 不实现 `Serialize`——**「远程录凭证」这条帧编译不出来**，以 trybuild 反例钉死。
- **握手**：版本＋schema 哈希不配即断连并回 `E_WIRE_MISMATCH`（装载期码，无 carrier）；schema 哈希由 wire 类型集派生，改一个 variant 即变。golden 钉住当前哈希，改哈希必须与本 SPEC 同集变更。
  **当前 golden**：`85ca5ecc871408f3ced9c72cfe37bc1deec31fcbfceade25e314a0b8662c0004`；**WIRE_V ＝ 45**（帧表与查询表的当前内容见 §8 各章）。
  `PutSecret` 无线格式——它经 `/enroll` 路由在进程内成形，见 §8-2 录入口。

**`Query::RunHistory { run, before, limit }` → `Answer::History`**：一个会话的历史按 run 取。`Query::History` 是城全局的最后一页，按它在客户端过滤，一个较早的会话就不在那一页里；`Query::RunView` 回答「这个 run 在不在、走到哪」，不回答「这个会话是什么」。

**答面复用 `HistoryAnswer` 而不新增一个。** 它已经带 `earlier` 游标，形状正是「往回翻」；再造一个只会让「一页历史长什么样」有两个答案。

**扫描量没有第二个上限。** `storage::index` 持了 run 索引之后，一次 `RunHistory` 只取属于这个 run 的 seq，**扫描量与 `limit` 同阶**，而 `limit` 已由 `HISTORY_MAX` 封顶。再设一个不防任何事的上限，只会让下一个读代码的人以为它还在防什么。

**`earlier` 的含义**：「从这条之前接着问」，`None` 即「到头了」；「答是空的而 `earlier` 是 `Some`」这一态**不存在**——服务端不需要把「我这一段没扫到」告诉客户端。

**建 run→seq 的索引**：不建索引，一次 `RunHistory` 要扫整本账，扫描量与账本长度同阶，而这是「打开一个较早的会话」这个动作的全部延迟。那份要随账本同步的派生状态已经存在：`storage::LedgerIndex` 常驻于 `Views` 并每次查询 `refresh`，run 表只是它多一个字段，搭同一趟刷新、同一份 cache、同一条「存疑即重建」的反射，不新增同步义务。至于「第二个权威」：索引回答的是「在哪」，从不回答「是什么」，它可弃且存疑即重建；账本仍是唯一权威。接面与内存代价见 storage-SPEC §8-4。
- **server**：默认绑定回环；绑非回环且 `auth` 未配置令牌时**拒绝启动**并回 `E_CONFIG_INVALID`（不是启动后再拒连——这是绑定面判定，不是请求面判定）。**暴露面必须有凭证是一条不变量，不是一句注释**：绑定判定把凭证本身装进 `BindFace::Exposed`，壳只持有这个面，于是「暴露着却不要求任何东西」是一个类型上不存在的状态（§8-41）。
- **auth**：令牌比较恒为常数时间（不早退）；比较函数以「逐字节差异位置不影响耗时」的性质测试看守。地基是 `server::constant_time_eq` 与 `decide_handshake`；`auth` 模块接令牌的生成、展示与持久化。
- **aggregate**：**类型化保证**——聚合上游连接的发送面在类型上只接受 `Query`，没有一个能塞进 `Command` 的方法（不是运行时 `if`，是类型上不存在该入口）；以 trybuild 反例钉死。

## 3 假设与歧义

- **本 crate 是 tokio 的异步消费者**：套接字服务跑在 tokio＋axum 上；gateway 与 endpoint 用 `reqwest::blocking`，不引 tokio（gateway-SPEC §3／§13）。
- **没有第二条传输路径**：即使浏览器与服务在同一台机器上也是网络连接，故不存在「同进程内存通道」这条优惠。唯一例外是 `PutSecret`——它不是靠运行时判断走内存通道，而是**类型上不可序列化**，因此远程连接根本编不出这条帧。
- **编码是 JSON，且编码本身不进冻结面**。选 JSON 的理由是不对称：浏览器原生支持，且开发者能在网络面板直接读帧。
- **`control` 自持鉴权与幂等，独立成模块**。ARCHITECTURE §6 预留了「做不到则并入 server」的退路，这里不需要它：`control` 持有一条 `server` 不知道也不该知道的策略——**哪些 Command 是干预，以及一次干预必须留下什么**（「任何中断都以 Handoff 收尾，下一位拿得到完整现场」）。那是判定，不是转调。
- **令牌的整个生命周期住 `auth`**（铸造、展示形、摘要、常数时间比对），`server::decide_handshake` 调用它：握手是令牌的一个读者，不是它的第二个家。
- **Signal 不在 Command 面**：Signal 的投递与消费住 `collab::inbox`。
- **未定：`Command::PutRange { doc, baseline, idem, edits }`。** 页面对任意一份文档的保存（refrain 路线图 §4-8）要在账本上如实记下一次写：哪份文档、从哪一版到哪一版、多少字节。现有的三种写文档的事件各只说一类文件（`spine_document_written` 四份 spine 文档、`governed_document_written` 三份治理文档、`rules_changed` 两份规矩文件），用它们记一份任意文档的写就是凑，所以 `PutRange` 与它的回执（新版本）等下一次加事件种类的那一轮一起落，§19-2 那时多一行。规则已经在 `crates/documents/Spec.lean`（D8、D9）：基线是 §8-69 答出的 `version`，编辑是那一版的 `Span`，后到者得 `E_VERSION_CONFLICT`。在那之前文档答复里没有写的操作。
- **`Welcome.resume_from` 读自 `LedgerHead`，`Welcome.epoch` 是创世记录的链哈希**：`decide_frame` 的第四个参数是 `WelcomeFacts { city, head, epoch }`——城名、账本头与 epoch 合成一个值，因为 `decide_frame` 已占满 4 个参数。`LedgerHead` 是 `ServeConfig.head` 递进来的一个 `AtomicU64`：装配层以重建视图时读到的最后一条记录的 `seq` 起头，折叠线程在每条记录**广播之前**把头推到它的 `seq`，socket 在 hello 时读一次。头放在原子量里而不放在视图锁里，因为读者可能长时间持有视图，而 hello 跑在 tokio 任务上，读头只是一次 Acquire load。先推头、后广播，加上会话在 hello 之前已订阅事件流，保证 `resume_from` 之后的记录必在流上：头之前而在订阅之后广播的记录会同时出现在流上与补拉里，所以边界上只可能重复、不可能缺失。`epoch` 是 `kernel::ledger::chain_hash(创世行)`，装配层在重建视图时读一次，由 `ServeConfig.epoch` 递进来：同一份账本的 epoch 永不改变，换了账本（重新 init、换了城目录）epoch 必变，所以客户端见到与上次不同的 epoch 就丢弃 belief、按快照重建，而不是拿旧水位去新账本里补拉。

## 4 现状分析

公开面见 `tools/xtask/api-baselines/wire.txt`。装配消费者是 `crates/sprawling`（`serve` 把处理器注入 `ServeConfig`）；客户端 `client/` 读的 `client/src/wire.ts` 由 `cargo xtask wire-ts` 从本 crate 的 schema 生成（§8-16）。

**已定而未落的改形。** 下列改形与 §8-53 起各节共用 `WIRE_V` 45（§12.1）：`CommitAnswer` 带出提交说明。下列新名字只动名字表，哈希随之变，不另进位：按房间分页列出 session 的查询（UC7b：起点 seq、起点方式——新开、`--carry`、分叉及其 `Origin`——、run 数、最近一次活动）。后者未落的原因是读法：今天视图只持热视图里最近冻结的 32 个 run，账本索引只按 run 建表，所以要么视图多折一张按房间的 session 表（视图快照的 `fold_version` 随之进一位），要么 `storage::index` 多建一张按地址的表；判定它的证据是两种做法在 40 万行城上开一次「最近」段的读数。每落一项删一项。

## 5 权威信源

wire 面全节（Command 表、Query 表、编码与握手、绑定面三段）；聚合层硬约束「聚合层只转发 Query 与 Event，恒不转发 Command」；干预动词语义表；`Sealed<T>` 的不可序列化性质；`kernel::error` 的装载期六码白名单（kernel-SPEC §8-1，封闭）。外部：axum（内含 tokio-tungstenite）与 tokio，版本钉在根 `Cargo.toml`。

## 6 命名统一

Command／Query／Event（三分的原名，不译）；命令与查询的原名逐字取 `COMMAND_NAMES`／`QUERY_NAMES`，两张表由 `named_frames!` 从变体表生成（§8-38），本文不另列；control surface（不译）；配对令牌＝pairing token；握手＝handshake。

## 7 模块边界

```
                    ┌── wire（类型与编码；无 I/O，纯数据与纯函数）
server（tokio＋axum）┤
  ├ WS 端点         ├── auth（令牌判定；常数时间比较）
  ├ 静态资源         └── control（干预动词入口；鉴权与幂等）
  └ /enroll、/transcribe、/drop（HTTP）
aggregate ──▶ 上游 City 的 WS 连接（发送面类型上只收 Query）
```

**不做什么**：不执行 Command（只解码并交给装配层注入的处理器）；不求值 Query（同上）；不产 Event（Event 载荷即 `EventRecord`，产地在各效果模块）；不做业务鉴权之外的策略；不声明 `pub` trait（不在 ARCHITECTURE §3 缝清单内——**处理器以函数指针或具体类型注入，不是端口**）；不内置任何穿透或中继（明拒：「内置一种就是替用户做了一个安全决定」）。

## 8 接口先行（按模块分章）

客户端是 TypeScript 的 `client/`，它的 SPEC 是 `client/client-SPEC.md`。

三条不可动摇的形状约定，它们决定接口而非被接口决定：

1. **`Command` 是穷尽枚举，每个改状态臂携 `IdemKey`**——「双击两下不开两个 Run」由类型保证，不由服务端去重表保证（去重表是第二道，`kernel::gate::dedup` 已有）。
2. **`PutSecret::value: Sealed<String>`**——`Sealed<T>` 无 `Serialize`（kernel-SPEC §8-25），故含它的枚举也无法整体派生 `Serialize`。这迫使 `Command` 的序列化实现**手写并对该臂显式拒绝**，而不是让宏悄悄地把它序列化出去。手写点即唯一权威，`E_WIRE_MISMATCH` 在此产出。
3. **`aggregate` 的上游发送面签名只接受 `Query`**——不是 `fn send(&self, frame: Frame)` 再运行时判断，是 `fn query(&self, q: Query)` 且没有第二个发送方法。

### 8-0 跨层名字的携带法（先于一切接口的决定）

`PlanRow.status` 携 `kernel::RoadmapStatus`（经 `kernel` 重导出，住 `spine::row`，公共拼写不变）。
`Dispatch` 携 `policy`，值集住 kernel（`kernel::RunPolicy` 与它的四个枚举，§8-57），wire 依赖 kernel，所以帧直接携它。`CreateBuilding` 携 `template`、`ConnectToolkit` 携 toolkit 的 slug——**这两个集合的权威分别住 `city` 与 broker 的目录，而 wire 只依赖 kernel**（ARCHITECTURE §2 depmap）。

取法：wire 携**无封闭列表的 newtype**（`ProviderName`、`TemplateName`、`ToolkitSlug`），只断言「非空且无控制字符」，**不断言合法值集**。合法值集恒由上游单一权威回答，映射点在装配层（`bin::assembly`），未知值即报错不猜。

理由：若 wire 自建一份 `enum Mode`，就产生了**同一规则的第二个权威**（AGENTS.md 明拒），且两份枚举会静默地漂开。wire **确实不知道** mode 集合是什么，假装知道才是谎言。**被否**：wire 内镜像这几个枚举——两个权威。

### 8-1 wire::frames（形状 2 值类型 ＋ 形状 1 编解码）＋command／answer／carried_name

**四样东西，四个文件**，切法取自依赖方向而不是行数，**因为方向是无环的**：`frames`（信封：`Query`、五种帧、`WIRE_V`、`schema_hash`）→ `command`／`answer` → `carried_name`。
谁都不回头指，所以加一个 Command 不碰答面，加一个答面不碰命令，而 `carried_name` 的四个新类型谁也不依赖。
- `command`：`Command`／`WireCommand`／`NoSecret`／`COMMAND_NAMES` 与两个只服务于命令的步骤枚举（`HaltScope`／`PursuitStep`）。两条不可拼写的性质随类型走，反例仍在 `tests/trybuild.rs`。
- `answer`：二十余个答面结构与 `Answer` 枚举、`HISTORY_MAX`。它们是读形状，一处判定也不做。
- `carried_name`：`carried_name!` 宏与 `ProviderName`／`TemplateName`／`ToolkitSlug`——**本 crate 不拥有的名字**，只在唯一构造点拒空与控制字符，合法值集恒属上游。
公开名经 `lib.rs` 重导出，定义住哪个文件是本 crate 的内政。

```rust
pub struct ProviderName(String); // 三个携带 newtype：parse 拒空串与控制字符，不拒未知值
pub struct TemplateName(String);
pub struct ToolkitSlug(String);
pub enum HaltScope { City, Building(Address), Workshop(Address) }  // 协议自有，无外部权威

// 两个枚举都由 named_frames! 声明：变体表一处，enum、name()、名表三样由它生成。
named_frames! { pub enum Command<Secret = Sealed<String>> { /* 变体表 */ } pub const COMMAND_NAMES; }
named_frames! { pub enum Query { /* 变体表 */ } pub const QUERY_NAMES; }

impl Command {
    pub fn name(&self) -> &'static str;   // 生成：恒等于本变体在名表里的那一条
    pub fn idem(&self) -> Option<&IdemKey>; // 改状态臂恒 Some；唯一例外 Auth
}
impl Query { pub fn name(&self) -> &'static str; }

pub const WIRE_V: u32;
pub const COMMAND_NAMES: [&str; /* 长度由变体数生成 */];   // 形状 6 数据面：名字权威，计数断言见 §2
pub const QUERY_NAMES:   [&str; /* 长度由变体数生成 */];   // 同上（§8-38）
pub fn schema_hash() -> B3Hash;        // blake3("sprawling/wire/" || WIRE_V 小端 || 'C'+名… || 'Q'+名… || 'E'+事件种类名…)

pub enum ClientFrame { Hello(Hello), Command(Box<Command>), Query(Query), /* … Monitor（§8-47g） */ }
pub enum ServerFrame {
    Welcome(Welcome), Event(Box<EventRecord>), Answered(Box<Answered>), Refusal(Box<AxError>),
    Delta(Delta),       // §8-8
    Log(LogLine),       // §8-32
    Lagged(Lagged),     // §8-41
    Output(LiveOutput), // §8-48d
    Monitor(Sample),    // §8-47g
}
pub struct Hello   { pub wire_v: u32, pub schema: B3Hash, pub token: Option<Sealed<String>> }
pub struct Welcome { pub wire_v: u32, pub schema: B3Hash, pub resume_from: Option<Seq>, pub city: Option<Address>, pub epoch: Option<B3Hash> }
// resume_from：账本头（最后广播的记录）的 seq，读自 ServeConfig.head: Arc<LedgerHead>；客户端据此把断线期间的缺口经 HistoryRange 补齐（client-SPEC 4-40）。
```

**三个形状决定及其理由**：

1. **`Command`／`Query` 不标 `#[non_exhaustive]`**（全库如此）。它们的版城所在的机器制是 schema 哈希（加一个 variant 即改哈希，旧客户端在握手处被拒），不是通配臂。不标它使装配层必须**穷尽处理每一条命令**——新增一条而无人处理在编译期即红。这是想要的约束，不是疏漏。
- **`Query::History` 有界且向后翻页**：服务端只广播「接下来发生什么」，只听广播的页面会把一座运行了一个月的城看成空城。答复恒**旧在前**——那是账本写它们的顺序，也是折叠期待的顺序；要新在前的读者自己倒一下手上的表，而服务端倒序会把折叠变成调用方的问题。上限 `HISTORY_MAX = 500` 由服务端钳，客户端要不到更多：一个调用方钳不动的上限，少一条让服务端做无界工作的路。`Answer` 因此失去 `Eq`（`EventRecord` 的载荷是任意 JSON，JSON 没有全序相等），crate 外无人比较过两个 `Answer`。
- **`/enroll` 答的是凭据的下场，不是请求的下场**：命令一进桌就回 201，vault 拒收就谁也不知道，人盯着一条成功消息而密钥根本没存。所以路由**先订阅事件流再投递命令**（顺序是承载的：先投递会让完成得快的 worker 把那一行写进没人在读的流里），然后等三选一——`secret_captured` 且 `ref` 与所投的引用相符即 **201**，正文就是那条记录里的 `ref`（答出去的就是存进去的那句），`Reply` 回来的拒绝即 **422**，两者都没有即 **202 并在正文里说明为什么是 202**。`SecretSink` 因此长出 `Reply` 参数：worker 在另一条线程上几分钟后拒绝，没有地址的拒绝到不了任何人。
- **回复通道关闭不等于成功**：worker 成功时不调用 `reply.refuse`，`Reply` 随之析构，于是 `recv()` 立即返回 `None`。若把它读成一个答案，它会与 `secret_captured` 赛跑并经常赢——所以关闭只熄灭那条分支，201 仍然只由事件给出。
- **`ConfigureBuilding` 写的是楼自己那一级**：`[sandbox]` 与 `[mcp]` 沿城→楼→房间解析；没有写面，一个人就读得到自己被什么治理却改不动它。答复里回的是**楼自己那一级的值**而不是解析后的值——用解析值填表，第一次按保存就会把城一级的设置抄进楼里。两个字段各自可缺省，缺省即不动那一节；`mcp` 为空表是「这栋楼一个服务器都不够到」，与「没说」不是一回事。
- **`ProbeEndpoint` 与 `AttachEndpoint` 是两条命令而不是一个开关**：模型清单若只作为 attach 的副产物到达，「看看这把 key 买到了什么」就必须先注册。两者的 `IdemKey` 由不同素材派生（`probe:` 前缀），因为问与登记是两件事，一件不得把另一件去重掉。`AttachEndpoint.admit` 空表即全部准入——没看过清单的人本就是这个意思；表里有而 endpoint 不供应的名字被略去而不是被承诺，与阅览室对不在书架上的 skill 的答复同形。

2. **`name()` 的穷尽 match 是计数断言的真机制**。光有一条计数断言拦不住「加 variant 但不改表」；`name()` 穷尽后，新 variant 必须在 `name()` 里现身，测试再断言它必在名表内。三道连环：编译 → 名表 → schema 哈希 golden → SPEC 同集变更。
3. **`Command` 泛型于 secret 携带者，远端实例把它钉成不可居住类型**（强于「手写 Serialize 在该臂报错」的写法）：

```rust
pub enum NoSecret {}                                   // 无值可造
pub enum Command<Secret = Sealed<String>> { … PutSecret { value: Secret } … }
pub type WireCommand = Command<NoSecret>;              // 套接字所能携的全部
impl From<WireCommand> for Command                     // 总函数；PutSecret 臂写作 `match value {}`
```

它同时关死两个方向，**且两边都是编译期**：出——`Sealed<String>` 无 `Serialize`，故 derive 生成的 `impl<S: Serialize> Serialize for Command<S>` 对 `Command<Sealed<String>>` 不成立，`serde_json::to_string` 对它是编译错误；入——`WireCommand::PutSecret` 的 `value` 字段无值可填，任何类型都不匹配。运行期只剩一道兑底：字节写着 `put_secret` 时 `Deserialize` 拒收，**拒绝文案恒不回显它正在保护的字节**。两个编译期反例住 `tests/ui/put_secret_onto_the_wire.rs`（stderr 快照分别钉在 `Sealed: !Serialize` 与类型不匹配两个正因）。

**被否**：另写一个少 `PutSecret` 一臂的 `WireCommand` 枚举——它把命令的声明拆成两份，是同一规则的第二个权威。

4. **`Auth`／`Hello` 的令牌是明文 `String`，而 `PutSecret` 的值是 `Sealed`**。不对称是故意的：配对令牌**必须跨线**才能完成配对，在传输中密封它只是自欺；它在**落地一刻**被封（`decide_handshake` 只接受 `&Sealed<String>` 作为已配置值）。凭证则相反：它本就不应跨线。

5. **wire 携 git 的 oid 时携 `kernel::GitOid` 本体**（`Query::Commit`／`Query::Hunks` 的 oid，与紧邻的 `B3Hash` 同形：40 位小写 hex，长度不对即拒），故 `GitOid` 带 serde。**被否**：在 wire 里自建 `CheckpointRef(String)` 并自校 40 hex——那是 git oid 形状的第二个权威。该变更属 kernel 公开面，已与 kernel-SPEC §8-2 同集提交（apisync 门）。

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
pub enum Pairing { Held, Absent }                    // 不是 bool；住 `wire::auth`，不带 `server` 也在（§12.2）
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

- **`Reply`、`Delivered`、`AcpProgress` 住 `wire::reply`，`Pairing` 住 `wire::auth`，四个都不在 `server` feature 之后**：城的写者 `accounting::worker` 点名它们，却从不监听端口，而它依赖本 crate 时关掉 `server`（§12.2）。`server` 只管 axum 那一半；把拒绝写成 HTTP 响应体的 `refusal_text` 仍在 `server::config`。
- **拒绝属于发问者，不广播**。把它做成一条事件会告诉所有在看的人「别人打错了一个字」，而事件流是这座城的历史，不是某个人的错字簿。故每条会话自持一个无界队列，`Deliver` 时把写入该队列的闭包随命令交给工人；会话的 `select!` 因此从两臂变三臂。
- **`Delivered` 是三态而不是 `Result`**，因为「没有人问过」与「问的人走了」是两件不同的事：前者是排程的正常形态，后者值一行诊断。这也是**不得重新引入 `let _ =`** 的落法——`SendError` 被穷尽消解成一个领域枚举，而不是被丢掉。
- **无界队列而非 `broadcast`**：一条拒绝丢不得，而它的量级是「人点错的次数」，不是事件流量。
- **未做且已知**：`/enroll` 路由同病。它同步答 201，而 `PutSecret` 是投递到同一张桌子的，工人的拒绝到不了 HTTP 响应。此处**不顺手改**，因为桌子在一次 dispatch 期间不被读取，把 HTTP 请求做成同步等待会让它挂上几分钟；正确的形状是有界等待加 202，随 `sprawling enrol` 一并落。

**Query 的答面**。`ServeConfig.queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>`，同步；`ServerFrame` 增 `Answer(Box<Answer>)` 变体。答面类型住 `answer`：`Answer`（City／Run／Approvals／Cost／Unavailable）、`RunSummary`、`CityAnswer`、`ApprovalsAnswer`、`CostAnswer`。

**`Query::BuildingView { addr }` → `Answer::Building(Box<BuildingAnswer>)`**（`BuildingDoc`／`ArchiveLine` 随之入 wire）。楼里的文件是楼的记忆，服务端在被问的那一刻读盘——**文件是权威**，另存一份索引就是第二个权威。`QUERY_NAMES` 因此从 10 增到 11，schema 哈希随之从 `238f11b2…` 变为 `85705c03…`：客户端与服务端同批发布，旧页面会在握手期被明确拒绝并提示刷新。

**`Welcome` 携 `city: Option<Address>`，`decide_frame` 增一个 `city` 入参。** 事件流只送连接之后发生的事，而城市的名字写在 Ledger 的第一条记录里——一个今天打开的浏览器永远等不到它。握手是「这是哪座城」的自然回答处；服务端从同一条创世记录读它，故两边不构成第二个权威。同批：`init` 把城市名写进创世记录的 `addr`（此前是 `None`，城市名只活在目录项里）。

**`ApprovalsAnswer.items` 携 `kernel::ApprovalItem` 全项，`ApprovalSummary` 删除。** 旧摘要类型丢掉了 `cluster_key` 与 `created`，于是界面无法按类聚合、也排不出「谁等得最久」；服务端为了填它还要从事件载荷里猜一个 `summary` 字段——那个字段从来没被写过，故每一条待批项都渲染成「(no summary recorded)」。载荷本身就是 `ApprovalItem` 的序列化，原样送过去既少一次有损转换，也让「什么算一类」只有 `web::approval::inbox` 一处答案。

- **为什么是强类型答面而不是一团 `Payload`**：`web` 只依赖本 crate，故发帧的边界 crate 欠对方一套读帧的词汇（同 kernel 再导出的理由）。一个无类型载荷会把解析责任推给每一个视图模块，每一个都得自己猜一遍形状。
- **`Answer::Unavailable { query }` 是一个真答案**：不求值的视图报自己的名字，而不是返回空结果——空城与未实现在界面上必须长得不一样。
- **`CityAnswer.buildings: Vec<BuildingProgress>`**：每栋楼一行，携 `Progress` 与 `problems`。解析不出的行进 `problems` 并照显——悄悄丢掉读不懂的行，等于按一个没人选过的分母报进度。
- **五维成本携权威总额**：`CostAnswer.total` 与 actor、segment、tool、skill 四个维度各自求和相等；`by_run` 只带活跃的跑与花得最多的前几个（sprawling-SPEC §8-90），和可以小于 `total`。界面按 `total` 算占比而不自己归一，未归因余额与列表之外的跑因此都看得见。
- **五维成本携权威总额**：`CostAnswer.total` 与五个维度各自求和相等；界面按 `total` 算占比而不自己归一，未归因余额因此看得见。
- **无报价的调用单独报数**：`CostAnswer.unpriced: UnpricedCalls { calls, tokens }` 是账本上没有权威计费额的模型调用次数与它们的 token 总数（`storage::Attribution` 的 `unpriced` 原样上线）。它们不进 `total`，所以缺了这一项，一座只用订阅登录或本地模型的城跑了多少次都读作「没花钱」；界面据 `calls > 0` 说「有调用没有报价」并给出 token 数，而不是把 `$0.00` 当作量出来的数。

- **採用 `broadcast` 而非每连接一个队列**：多个标签页是常态；慢客户端被拉下而不拖住写入方。**丢下的那一段不再静默**：事件流慢过城的会话收到 `ServerFrame::Lagged { from, to }`，按这个区间向账本补拉（§8-41）。三路语义不同，故这三节分开陈述：事件可补、增量与日志恒不可补、会话自己的拒绝根本不走广播。

## 8.5 两个设计

**握手的失败处置：断连 vs 降级协商。** 取断连。理由是这条错配的真实来源早已写明——「浏览器可能缓存了旧前端而服务端已经升级」，而降级协商要求服务端同时维护两套 wire 语义，那是两个权威。断连＋提示刷新把一个协议问题还原成一个刷新动作。**被否**：版本协商（多版本共存）——它的成本在每次改 wire 时都要付，而收益只在「用户不肯刷新」这一个场景里兑现。

**携字节的通路：WebSocket 帧 vs 独立 HTTP 路由。** 取独立 HTTP 路由（`/enroll` 的凭证、`/transcribe` 的录音）。**被否**：在 WS 上自制分片协议——那是重新实现 HTTP 已经做好的事，且会让字节的传输失败与命令失败混在同一条通路上难以区分。

**`POST /enroll`，唯一携凭证字节的路由**

它是 HTTP 而非 socket 帧，因为 socket 的 `WireCommand` **拼不出** `PutSecret`（`NoSecret` 无值）。两半合起来才是完整保证：类型层管住帧，`decide_enroll` 管住字节——因为字节总可以被 POST 到一个路由上。

- **只认回环对端，配对令牌也不算数**：令牌认的是人，而这条规则管的是**字节走到哪里**。拒绝的第三段指向宙主机，于是它是约束而非死路。
- 壳里零策略：判定在 `decide_enroll`，壳只搬字节——同 `decide_bind`／`decide_frame` 的切法，故无需跑服务即可穷尽测。
- 应答返回那条 `secret_captured` 记录写下的 `ref`——金库键的那句文本，不是路由再拼一次的一句；值不回声、不入事件载荷。入金库由 `Sealed::into_vault_value`（住 kernel::secret，即 expose 白名单三文件之一）完成，开封因此**不发生在装配层**。

**线上没有登录命令**：订阅额度由厂商自己的 harness 带进城，人在 harness 里自己登录，本城不以任何厂商客户端的身份登录（gateway-SPEC §8-5）。旧版本的 `Login` 命令与 `LoginStep` 随之删去；`COMMAND_NAMES` 少一项，schema 哈希因名字表而变，旧页面在握手期被明确拒绝，所以 `WIRE_V` 不为此进位。

**五个查询各有自己的答**（`InboxView`／`DiscardView`／`RegistryView`／`ArchiveSearch`／`Metrics`）。三条口径：①**队列折叠着看不消费着看**（`Inbox::pull` 要拿走才给内容，看一眼就取走的视图会改变它所报告的对象）；②归档在被问的那一刻读盘（同 `BuildingView`，文件是权威）；③**`Metrics` 恒不携钱**——钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。

**`DiscardLine.restoration` 携 `Option<Restoration>` 而非一个句子，WIRE_V 3→4**。回收站那一行的「怎么拿回来」原本在服务端被拼成 `"tracked: file:…"`，而客户端早已持有它的唯一措辞处（`web::approval::ReturnPath::sentence`）——**一件事两个渲染权威**，而服务端那个还拼不出可执行的那句话。现在计划以它自己的形状上线（载荷本来就是 `Restoration` 序列化出来的，故读得回去）；`None` 的意思是**这一条记录用了本构建读不懂的方案**，界面据此画一行而不给动作（`ReturnPath::Undescribed`）——行恒不隐藏，因为藏起一件被删的东西比承认读不懂它的方案更糟。`QUERY_NAMES` 与 `COMMAND_NAMES` 未动，故哈希只因 `WIRE_V` 而变——又一例「语法换形而名字没换」。

**`POST /acp` 与 `AcpSink`**。外来编辑器的请求走自己的路由，不挤 Command 面：它自带鉴权、要一个当场的回答，而 Command 面的回答是事件流。三条口径：①**令牌在本 crate 判**（配对令牌住这里，常数时间比对也就住这里），只把 `authentic` 一位传进去——拒词由 `agent_protocols::admit` 措辞，「未配对者只学到一位」因此只有一个权威；②回给编辑器的只有 `AcpProgress` 三字段，run id 是工人接单时才铸的，故受理那一刻诚实的答案是「已受理、未完成」；③没配对令牌的城即回环独占，与 control surface 同一条规矩。

**三帧登记面**（§8-1 golden 同集更新）——`AttachEndpoint`（人刚输入的 URL＋兼容格式＋`secret:` 引用；**引用有字节形，凭证没有**）、`SelectModel`（标签→模型＋两个探不到的 token 数；输出上限是 `Option<Ceiling>`，缺席即「没人登记过」，零在类型上不存在）、`EndpointView`（设置页的读；`EndpointsAnswer` 里 `has_credential` 是关于凭证能回答的全部）。

**三个 kernel 类型的再导出**（`DialectKind`／`Effort`／`ModelTag`）。`web` 只依赖 `wire`（拓扑图），而设置页要拼写这三个词；再导出而非镜像定义，因为镜像就是同一规则的第二个权威——同 §8-0 对 `Mode` 的口径。

### 8-3 wire::auth（形状 2 值类型 ＋ 形状 1 判定）

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

（b）**`PairingToken` 不持明文**：`expose` 只得出现在兑付点（`xtask secret`），而把展示点加进兑付点白名单是修门不修因。本模块不需要明文，`mint` 把配对码直接交给调用方去展示，自己只留摘要。封一个值再在下一行解封是表演；**根本不持才是我们想要的性质**。

（c）**字母表剔除可混淆字符**（0／O／1／l／I／5／S），29 符号×四组五位≈ 97 位熵。理由不是审美：配对码要被人读出来、在另一台机器上手敲进去，一个口述会错的码，代价由用户在另一台机器前承担。

（d）**分组形兼顾了 secret 门**：每片 5 字节远低于 20 字节阈值，测试里的配对码字面量不会被熵侦测器咬住。

### 8-4 wire::control（形状 1 判定函数）

```rust
pub enum Intervention { Steer, Cancel, Halt, Release }
pub enum ControlVerdict {
    Intervene { verb: Intervention, run: Option<RunId>, must_write_handoff: bool },
    NotAnIntervention,
    Refuse(AxError),
}
pub fn classify(command: &Command) -> ControlVerdict;
```

**本模块持有的唯一规则**：中断一个活着的 Run 的两个动词（Steer／Cancel）**恒以 Handoff 收尾**，使下一位（人或 Agent）拿得到完整现场。`Halt`／`Release` 按 scope 停一片，不针对单个 Run，故 `run` 为 `None` 且不强制 Handoff。

**`must_write_handoff` 为什么是返回值而不是副作用**：本 crate 不持 Ledger 句柄（§7 已写）。它只能**说出义务**，履行义务的是装配层。把它做成返回值的代价是装配层可能忽略它——故同变更集交付一条断言：一次干预的事件序里若无 `handoff_written`，即失败。

**为什么不并入 `server`**：`server` 的职责是「这个字节流能不能变成一个帧」，`control` 的职责是「这个帧是不是干预、干预要留下什么」。后者在无网络的 citysim 里也成立，前者不。两个职责的变化率也不同：加一个动词不应该碰监听器。

### 8-5 wire::aggregate（形状 1 判定 ＋ 形状 2 值类型）

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

### 8-6 crate 面的两项

**一、`server` feature**（默认开）：`server = ["dep:tokio", "dep:axum"]`。`accounting`、`xtask` 与 `fuzz` 要本 crate 的词汇而不要监听器，取 `default-features = false`；`just features` 编译一次关掉它的构建，`cargo clippy --all-features` 编译开着它的。

**被否**：把 wire 拆成另一个 crate。那要改 ARCHITECTURE §2 的拓扑，而 feature 边界已足以表达「词汇与监听器分开」这一件事。

**二、kernel 类型再导出**：本 crate 的公开签名上出现的 kernel 类型一律从 `lib.rs` 再导出——**发帧的边界 crate 欠对方一套读帧的词汇**（C-REEXPORT）。再导出集就是 `lib.rs` 的 `pub use kernel::…` 那几行；动它就是动公开面，与本节同一提交更新。

### 8-7 一次会话有名字，而名字就是它干活的那个房间

```rust
WireCommand::Dispatch { addr, task, goal, policy, idem, session: Option<SessionName>, effort, … }
// SessionName 住 kernel：一个构造点，内容即一个地址段
```

- **不加第四层**：每个 Run 一个目录会切断 Handoff 的连续性，而连续性正是一个 session 之所以是 session 的东西；`collab` 整套也都建立在「几个居民在同一栋楼里不互相踩」上。
- **地址给楼，名字给会话**：`addr` 可以只是一栋楼；`session` 在它底下开一个房间（`city::room::open`）。重名加数字后缀（`refactor`、`refactor-2`），而不是拒绝——一个人连着开两次同名会话是常事。
- **向已有房间派活即继续那条会话**：它的 `Handoff.md` 与 `JOB.md` 就是连续性。「回复某个 session」因此不需要新概念。
- **`RunId` 不变**：名字是房间的标签，不是 Run 的。一个房间一生中的多次 Run 合起来才是一条会话。
- **`SessionName` 是值类型而非 `String`**：它必须能当一个地址段（无 `/`、无 `.`／`..`、无控制字符、非空、不叫 `.sprawling`），否则一个人输入的字会变成一条路径。

## 9 工作流程

启动时 `decide_bind` 判绑定面（非回环无令牌即拒启动）→ 监听 → 一条连接先收 `Hello`，`decide_handshake` 判版本、schema 哈希与配对 → 回 `Welcome`（`resume_from`、`city`、`epoch`）→ 之后每帧经 `reception` 判定：`Command` 交装配层注入的 sink，`Query` 交视图求值并以 `Answered` 回，订阅到的记录以 `Event` 推送，落后的订阅者收到 `Lagged` 并经 `HistoryRange` 补拉（§8-41）。

## 10 实现逻辑

各模块的实现规则写在它的 §8 小节里。

## 11 边界枚举

握手哈希不配／令牌错／绑定非回环无令牌／读不出的帧（§8-37）／客户端半关连接／聚合上游断线／Event 推送背压（`Lagged`，§8-41）。

## 12 Decisions

- **`E_WIRE_MISMATCH`**：不可——它是装载期六码之一（封闭白名单），且它的存在理由就是「浏览器缓存旧前端」这一 WebUI 特有错配。类型无法定义掉跨版本的字节。握手之后解不出的帧同归此码、两侧的处置见 §8-37。
- **`E_CONFIG_INVALID`**：不可——绑定非回环而无令牌必须在**启动时**拒绝，这是配置判定不是请求判定。
- **没有 signal-unknown 码**：握手的 schema 哈希保证同一连接的两端共享同一份词汇，一个本版本不认的 Signal 种类只能来自更新的二进制写的 Ledger，而那已由版本方向门拒在外面（`crates/collab/Spec.lean` §8 的 `collab::inbox` 一条）。

### 12.1 `WIRE_V` 在两次推送之间最多进一位，进在第一个改形的提交

**决定**：握手要保证的只有一件事：两份可能相遇的构建，只要线上语法不同，`(WIRE_V, schema_hash())` 就不同。`schema_hash()` 已经吃进命令名、查询名与事件种类名（§8-1、§8-16），所以增删、改名、调序一个 `Command`／`Query` 变体或一个 `EventKind`，哈希自己会变，`WIRE_V` 不为此进位。名字都不变而形状变了——答面或既有帧加字段、一个枚举换词或加值（`Mode`、`ModelTag`、`Note` 这类）、`ServerFrame` 加一臂——哈希不变，`WIRE_V` 必须进位。进位按推送计：上一次推送之后，第一个名字不变而改形的提交把 `WIRE_V` 加一，此后到下一次推送之前的改形共用这个值；推送之后再遇到改形，再进一位。

**理由**：会相遇的构建在人手上：发布版与推送过的 `main`。一个由构建 X 送出的页面连到构建 Y 的服务端，只发生在人换了二进制而页面还开着或缓存着的时候；没推送的中间提交只在开发机上跑，彼此不会各自服务一个人的页面。进在第一个改形的提交而不是推送前收尾：收尾才进，中间各树会以上一次推送的号放行语法不同的旧页面。改形只由一个写者按次序落地：两条并行分支各自进位，曾让两份语法不同的构建共用一个号、互相通过握手。

**被否**：①每个改形的提交各进一位：更严，但一次推送带出几个号只取决于提交怎么切，读者从号上读不出任何东西；②推送前统一进位：理由见上；③改名也进位：哈希已经拒掉旧页面，多进一位不多拒任何页面。

**守护**：`tests/wire_contract.rs` 钉住去掉文档文字之后的 `wire_schema()` 摘要。改形即红，失败信息写出本条；改的人据此判断这一次要不要进位，再更新摘要。它不判断该不该进位，那取决于上一次推送之后是否已经进过一位。

### 12.2 写者点名的四个类型不在 `server` feature 之后

**决定**：`Reply`、`Delivered`、`AcpProgress`（`wire::reply`）与 `Pairing`（`wire::auth`）在任何 feature 组合下都编译；crate 根的名字与再导出不变。`server` feature 只带监听器：axum 的路由、WebSocket 会话、HTTP 门与把拒绝写成响应体的 `refusal_text`。

**理由**：城的唯一写者搬进 `accounting` 之后（accounting-SPEC.md 8-11），它的命令入口、桌子、欠账与 ACP 受理点名这四个类型，而 `accounting` 依赖本 crate 时关掉默认 feature，因为它要的是词汇，不是 TCP 栈。四个类型本来就是词汇：`Reply` 包一个 `Fn(AxError) -> Delivered`，正是为了不把传输层写进签名（§8-2）；另外三个是普通的枚举与结构体，不持有 tokio 或 axum 的任何东西。`Pairing` 是一次配对判定的结论，与配对令牌同住 `auth`；判定本身 `decide_admission` 仍在 `reception`。

**被否**：①给 `accounting` 的 `wire` 依赖打开 `server`：它把 tokio 与 axum 拉进一个从不监听的 crate，`wire` 不带 `server` 的那份构建（`just features`）也就不再守住「词汇不需要监听器」；②在 `accounting` 里另写一份同形的类型再在装配根互转：同一个拒绝去向有两个定义，互转是第二个权威。

**守护**：`just features` 编译不带 `server` 的 `wire`，`accounting` 的每一次编译也是。四个类型之一若被挪回 `server` 之后，`accounting` 编不过。

### 12.3 本批上线的字段各读自一个权威，缺席即没有

**决定**：(a) `Turn.first_at` 照录 `model_returned` 的键；`Timing` 由 `EventRecord::moment` 与答复的错误码判出。线上不带首字耗时，也不从相邻行推断一个时刻量没量过。

**理由**：一个事实一个家。时刻语义的权威是 kernel-SPEC §8-4 与 `EventRecord::moment`；首字耗时是两个时刻之差，页面手里已有这两个数。

**被否**：①线上带一个 `ttft` 时长：派生值在线上有了第二个家，而且 `t` 未量时它要答一个答不了的数；②`Timing` 三值（量过、回合时间戳、城补的）：页面对后两种做同一件事——不画用时——第三个值只会逼每个读者多写一臂；要分辨时，`Call.outcome` 与答复内容已经说明那是城补的。

(b) `CommitAnswer.previous` 由账本折出；`CommitAnswer.parents` 在答问时读 git。

**理由**：「同一次 run 的上一个提交」是账本上两行的关系，折叠已经按 `seq` 看过每一行。父提交是提交对象的一部分，oid 就是对它的哈希，账本从未记过它。

**被否**：①在 `checkpoint_committed` 里记下父提交：检查点的写方要多记一个字段，评审落地的合并提交由另一处写，这个键出现之前的每一行都没有它，而这三种情形 git 都答得出；②`previous` 只带 oid：一段的另一端还要一个 `seq`，才能不扫账本就往回读调用；③读 git 在锁内做：一页五百个提交各开一次仓库，别的问题都在等这把锁。

(c) `Call.effect`、`Call.render` 照录 `tool_called` 在调用那一刻记下的登记。

**理由**：登记只在 run 的工具台上存在，读面够不到；记在调用那一行，读面读的是那一刻的事实（kernel-SPEC §12.11）。

**被否**：读面按工具名匹配出呈现：每加一件工具都要改这个匹配，楼的 MCP 工具读面不认识。

(d) `Output.pinned` 读自结果里第一笔离窗账目，账目不另记调用 id。

**理由**：账目住在它裁掉的那个结果里，那个结果写在带着 `tool_use_id` 的 `tool_result` 行上，所以调用身份已经由行给出。

**被否**：给 `ResultOffloaded` 加 `tool_use_id`：同一个 id 的第二个家，而这份账目随结果整份进模型的请求字节，多出的键会让每一次被裁的调用多付这些 token。

### 12.4 `Dispatch` 带一个 `policy`，而不是四个平铺的字段

**决定**：运行策略在帧上是一个嵌套的值 `policy`，形状就是 `kernel::RunPolicy`；`mode` 不再是 `Dispatch` 的顶层字段（§8-57）。

**理由**：四个值总是一起走——线上、账本的 `run_started`、装配层的 `Assignment` 都是整份地拿、整份地传——一起走的值是一个值。帧上的形状与账本上的形状相同，页面、回放与 playback 读的是同一个东西，没有一层把四个字段重新拼成一个结构。四个键都必填：选择要求不等于拿到证据，而一个缺省成「免检」的键正是这条规则要挡住的。

**被否**：①保留顶层 `mode`、平铺加 `write`／`admit`／`landing` 三个带缺省的字段：旧页面可以不改，但缺省值会替没选过的人做一个选择，而帧本来就因改形要换一次版本；②`policy` 里的键带缺省：同一条理由。

**重开参数**：运行策略多出一个只有部分派活才需要的值时，重议那个值要不要缺省。

### 12.5 设置页上的卡片的改写在城里做，页面只交值

**决定**：设置页上的卡片发 `PutIdentity { card, base }`，由 `city::Naming` 在 `base` 上改写身份区并整份落盘；原文编辑器发 `PutDocument { which, base, body }`，城在落盘之前用同一个解析器读一遍身份区（§8-59）。

**理由**：身份区是 TOML，`PREFERENCES.md` 与 `MAYOR.md` 里还有人写的未知键与正文。页面若自己拼整份文件，「身份区怎么写、保留什么、名字合法与否」在 TypeScript 与 Rust 各有一份，两份迟早不一致，而写坏的一份会让下一次开城读不出名字。卡片只交值，拼法就只有城一处；两条命令携同一种 `base`，所以并发保存只有一条规则：后到的那一个被拒。

**被否**：①页面拼出整份文本，只用 `PutDocument`：拼法有第二个家；②卡片带一个版本号而不是全文作 `base`：城要另存版本到全文的对应，而 `PutSpine` 已经用全文作基线，两种基线会让同一个页面写两种守卫。

**重开参数**：身份区长出页面要分块编辑的结构（例如多个账号各一张卡）时，重议卡片是否改为按键寻址。

### 12.6 动词类是 §19-2 的一列，由中继的穷尽匹配实现、门机器对照

**决定**：一帧由远程设备发来时属于 `Read`／`Act`／`LocalOnly` 哪一类，写在 §19-2 reach 旁边的 `class` 列；中继在 `bin::outside::verbs` 以穷尽匹配实现它，`xtask wiring` 逐行对照两者（§8-65）。

**理由**：这一列回答的问题与 reach 同族——一个动词从哪里够得到——所以与 reach 住在同一张表上，读者在一处看到两件事。实现必须在 `sprawling`，因为 `VerbClass` 归 `remote_access`，本 crate 不依赖它；穷尽匹配保证一个新 Command 在有人决定它的类之前编译不过，表格对照保证决定写下来了。

**被否**：①给 `wire::Command` 加一个 `class()` 方法：要么本 crate 依赖 `remote_access`，要么另立一个同值的枚举，两处定义同一组值；②在表里写类、中继在启动时解析 SPEC：二进制读一份 Markdown 做授权，文档的排版错误就成了门的漏洞；③不写表、只留匹配：一个动词能不能从城外做，是人要读到、要决定的事，藏在代码里没人看。

**重开参数**：wire 的规格迁成 `Spec.lean` 时，`class` 改写成 `def Command.verbClass` 的一臂一行，`xtask wiring` 按 xtask-SPEC §8-45 读它。

### 12.7 GitHub 导入是一条只读查询，由二进制跑 gh；指南进度是一个按城的文件，整份写

**决定**：(a) 从 gh 读用户 ID 是 `Query::GithubLogin`，答一个候选与它来自的主机，不写任何东西；跑 gh 的函数住二进制（`bin::doctor::github`），由服务中的城交给视图（§8-67）。(b) 上手指南的进度是 `Query::Guide` 与 `Command::PutGuide`，存进城保留子树里的 `GUIDE.toml`，整份写、后到者为准、不写账本行（§8-68）。

**理由**：(a) 导入只给出一个候选，保存仍经 `PutIdentity` 与它的基线守卫，所以「谁改了称呼」只有一条路；gh 的当前身份会随多账号、环境令牌而变，一条把它直接写进文件的命令会让一次按键悄悄改掉已保存的名字。起子进程是碰主机的事，按 accounting-SPEC §12 第 9、10 条它住 `sprawling`，经一个 `fn` 指针交进来；视图在放开快照之后才调用它，所以一次慢的网络请求不挡折叠。(b) 进度要跨浏览器、跨重开，所以在城里；它治理的是这座城给人看什么，所以在保留子树里、没有写域够得到；文件就是线上值的序列化，键名只有一处。它是光标而不是正文，基线守卫只会把一次普通的翻页变成冲突。

**被否**：(a) ①一条命令读 gh 并直接写进 `PREFERENCES.md`：跳过了人对候选的确认与基线守卫；②在开城时读一次 gh：违背「只在人要求时跑」，并让每次开城都碰一次网络；③在 accounting 里直接起 gh：本 crate 的端口规矩（accounting-SPEC §12 第 9 条）就是为了让 citysim 与测试换得掉碰主机的那一步。(b) ①存进浏览器：换一个浏览器就从头再来；②存进人的 `~/.sprawling/config.toml`：一个人有几座城，每座城的配置不同，进度也不同；③每一步一条命令：进度的形状一变就多几条帧，而整份写的唯一代价是两个浏览器同时翻页时少一个「看过」；④写一行账本：要一个新的事件种类，而界面的位置不是这座城做过的事。

**重开参数**：(a) 页面要列出 gh 已登录的全部主机供人选时，加一条读 `gh auth status` 的查询；(b) 同一座城常有几个人各开一个浏览器、各走各的指南时，进度改为按人存。

### 12.8 文档答复带版本，三种「没有文本」各是一种答复，范围按版本读

**决定**：`Query::Document` 答 `DocumentAnswer { at, state }`：缺失、读不了、空、有内容四种状态；有内容时带版本、格式、大小与第一个窗口，文本之外的是 `Opaque`（§8-69）。其余的经 `Query::Range { version, range }` 从内容库里那一版读（§8-70）。

**理由**：文档工作区要编辑、要保存、要知道自己改的是哪一版（refrain 路线图 §4-8），旧答复只有一个被截断、被有损解码的头，没有版本。把版本做成内容库的地址，「这一版的第三屏」就有一个不随文件变动的读法，而下一次保存的基线是 32 字节的摘要而不是整份正文。

**被否**：①仍用 `Unavailable` 答缺失：页面分不出「可以新建」与「读不了」，`Unavailable` 的 `query` 串也不是给人读的理由；②范围读文件此刻的字节、版本不对就拒：文件每动一次，读到一半的页面就要从头重读，而一个居民在写的文件每几秒动一次；③每一版都存进内容库：一份整份已经在答复里的小文件，页面不会再按版本要它，存下它只是让内容库替每一次打开付一份拷贝。

**重开参数**：页面要读一份整份放得下的文件的旧版本时（例如对比保存前后），小文件也存版本；内容库长出回收时，按版本读的窗口要说「这一版已经回收」，与 `Unavailable` 分开。

## 13 依赖选型

| 依赖 | 用途 | 依据与替代 |
|---|---|---|
| `tokio` | 异步运行时 | 替代（自写 reactor）重做一件现成的事 |
| `axum` | HTTP 静态资源、几条 HTTP 路由、WS 升级 | tokio 官方序列，且其 WS 支持内含 tokio-tungstenite，省掉一层版本对齐 |
| `tokio-tungstenite` | WS 协议 | 经 axum 传递依赖；**不直接依赖**，避免两处版本权威 |
| `serde`／`serde_json` | 帧编码 | 已在 workspace |
| `kernel` | AxError／EventRecord／IdemKey／Sealed／Address | 唯一上游 |

**不引**：任何通用 RPC 框架（wire 是一组具名 variant，不是一个可扩展的服务定义）；任何 session 中间件（鉴权面只有配对令牌一件）；任何穿透／中继库。**WebTransport／QUIC 同此**：重开条件写在 §8-41 末节（城真的在回环之外且实测有队头阻塞，两条都成立才重开），此前它不因「需要第二种协议」而回来。


## 14 硬编码声明

- **默认绑定地址恒为回环**——它不是配置的默认值那么软，而是「非回环需要额外条件才允许」的判定起点。
- **schema 哈希的派生框架**（哪些类型入哈希、以什么序）一旦定下即是冻结面：改框架＝旧客户端全部拒配。故派生框架与 `IDEM_DERIVE_V` 同规，携版本字节。

## 15 影响面

- 改 `Command`／`Query`／帧：名字变了，schema golden 随之变；名字不变而形状变了，按 §12.1 进 `WIRE_V`；`client/src/wire.ts` 重新生成（`cargo xtask wire-ts`），`crates/sprawling` 的处理器穷尽匹配随之改，§19-2 的 reach 表增删一行（`xtask wiring`）。
- 改 `ServeConfig`：波及 `crates/sprawling` 的 `serve` 装配点。

## 16 测试与约束

- 计数断言：`tests/wire_contract.rs` 逐名核两张名表与枚举变体数是否同步；**数字不写在这里**——一个被抄进本文的计数就是同一条规则的第二个家（同 §8-1 对 golden 的处置：本文只记哈希值本身，因为那是一个不可推导的输出而不是一条可重算的规则）。
- trybuild 两反例：远程 `PutSecret`（含 `Sealed` 的帧不可序列化）；`aggregate` 发 Command（发送面无该入口）。
- 常数时间比较的性质测试：差异位置不影响比较耗时。
- 握手 golden：schema 哈希入快照；改 wire 类型必须同时改快照与本 SPEC。
- 绑定面判定的单元测试：回环／非回环×有令牌／无令牌四格，只有「非回环＋无令牌」拒绝启动；并断言判定给出 `BindFace::Exposed` 时里面的摘要就是判定的那一个（凭证属于面的这一条因此有测试，而不只有形状）。
- 约束：非测试代码遵守 C3 硬化全条；全库禁裸 spawn（确定性第 3 条）——**本 crate 的并发必须是结构化的，带取消令牌**，这是引入 tokio 后第一条要守住的线。

## 17 模型体验

零字节，因为 wire 面是人与服务端之间的协议，不进入任何 prefix。间接影响有一条：`Steer` 经 control surface 送达后，落点是**结果信封**（追加在下一次工具调用结果末尾，前缀 `user`），那几个字节由 `runtime::pipeline` 计入，不由本 crate 计入。

## 18 文档同步

- ARCHITECTURE 模块表的 wire 各行。
- `client/client-SPEC.md`：线上形状变了的那一侧。
- `crates/sprawling/sprawling-SPEC.md`：`serve` 子命令的装配面。

### 8-8 第三类帧：模型还在说的时候（形状 2 值类型）

**一个 token 增量不是效果，所以它不是事件。** 事件是发生过的事：有序号、进账本、可重放、可离线验。增量没有序号、永不落盘、无法重放，而且一个客户端漏掉一条什么也没丢。把它折进事件流，等于给「模型说了什么」造第二份、不可验证的历史——而这座城全部的主张就是那份历史可验。

于是它是 `ServerFrame` 的第四个取值：

```rust
pub enum ServerFrame { Welcome(..), Event(..), Answer(..), Refusal(..), Delta(Delta) }
pub struct Delta { pub run: RunId, pub increment: kernel::Increment }
```

`WIRE_V` 11 → 12，schema 哈希随之变（`ServerFrame` 不进名字表，故这是「语法换形而名字没换」那一类，版本进位、旧页面在握手期被明确拒绝）。

**三条口径，各自都是必要前提：**

1. **携 `RunId` 而不携序号。** 客户端据此把缓冲挂在一个 run 名下，并在该 run 的 `model_returned` 到达时整个丢掉。这就是「结算文本赢」的全部机制——一条断言钉住它（`web::session`）。
2. **两条广播通道而不是一条。** 增量与事件的丢弃语义相反：漏掉的增量什么都不是，漏掉的事件是必须从账本补回的历史。共用一条通道会让一次话多的模型把记录挤出慢读者的窗口。
3. **没人看时不开流。** 装配层只在有客户端时装 sink；`RunHooks.deltas` 为 `None` 的 run 走原本的阻塞调用，字节不差。于是 citysim 与离线重放的路径一字未改。

**一条增量说清自己来自哪一路**（`kernel::Increment`，WIRE_V 28→29）。推理与散文是两路而不是一路：一个把八成以上输出花在推理上的模型几乎不往散文那一路发东西，而把两路并进一个缓冲区的页面，要么把模型的草稿当答案给人看，要么让人对着空线程等三分钟。**败给的方案**：帧上加一个布尔——那要求每一个读者自己记住 true 是哪一路。

**服务端半边在 `gateway`：** `kernel::Model` 多一个 `call_streaming(req, onto)`，默认实现就是 `call` 并且不报告任何增量——一个没有流的适配器因此是诚实的而不是坏的。`gateway::endpoint` 覆盖它：请求带 `stream: true`，逐行读 SSE，`dialect::increment_of` 认两路——助手散文与推理各自的那个字段，工具参数一律不报（半个工具参数不是短一点的工具参数），最后 `dialect::settled_from_stream` 把帧重装成**非流式的那个形状**，交给同一个 `response_from_wire`。**结算答案因此只有一个解析器**：流式调用与阻塞调用不可能对同一个回复得出两个结论。流被切断仍然表现为读取错误，永不表现为一个变短的回答。

### 8-15 `/enroll` 的三结局测试进程内驱动（`tests/enrolment.rs`）

`tests/enrolment.rs` 原以 `axum::serve` 端起本 crate 的路由、手写 HTTP 字节去问它，因而是 `xtask boundary` 在册的唯一越线文件。它检验的是 §8-2 的三选一（`secret_captured` 相符→201／`Reply` 拒绝→422／有界等待到期→202），三者由测试替身的工人（存／拒／沉默）分出——这是白盒问题：真二进制上 vault 只有一种下场，且 `serve` 经 `Custodian::probe` 写平台凭据服务、线格式无收回凭据的动词，黑盒重写既不可判也不可回收。故改为进程内驱动：`wire::router(&config).layer(MockConnectInfo(peer))` 后 `tower::ServiceExt::oneshot` 一发一收，peer 以 axum 给测试的那条路供给，不起 socket、不写字节。三断言原文不动；`[boundary.predating]` 归空。`tower`（`util`）只作 dev-dependency，已在 axum 之下的依赖图里，锁文件不增包。

### 8-16 线的另一端由这一端生成（`wire_schema`，feature `schema`）

**需求**：`client/`（TypeScript）要与 `crates/wire` 说同一门语言，而 §8 从头到尾只承认一个权威——Rust 的类型声明。手写一份 TS 类型就是第二个权威，它会在握手通过之后才被发现漂了。故 TS 面由这一端**生成**：`cargo xtask wire-ts` 读本 crate 的 JSON Schema，写出 `client/src/wire.ts`（每个类型一条 TS `type` 加一条 Effect `Schema` 值，外加 `WIRE_V` 与 `WIRE_HASH`）；不带 `--write` 时只比对盘上文件，第一处不同的行即门红。

**接口**（feature `schema`，缺省关；`web` 以 `default-features = false` 依赖本 crate，产品二进制不开它）：

```rust
#[cfg(feature = "schema")]
pub fn wire_schema() -> serde_json::Value;   // 一份文档：`$defs` 里是信封两端可及的每一个具名类型，
                                             // 含 `ClientFrame` 与 `ServerFrame` 两个根
```

- **哈希的素材是整个线上名字面**：`schema_hash()` 吃 `WIRE_V`、命令名表、查询名表，以及 `kernel::EventKind::ALL` 按表序的每个种类名——事件种类随每个事件帧到达页面，改名、增删一个种类与改名一个帧一样会让旧页面误读，所以它移动握手哈希，不靠有人记得去升 `WIRE_V`。生成的 `WIRE_HASH` 常量就是这个函数的输出，客户端在握手处送回它，服务端按原样校验——两端校验的是同一个值，而不是一个「schema 文档的摘要」；后者会把每一条 doc 注释的改动都变成一次拒配。
- **每个入帧的类型都派生 `schemars::JsonSchema`**（`#[cfg_attr(feature = "schema", derive(...))]`），派生宏读的是 serde 已经在读的属性，故形状与编码同源。kernel 侧的值经 kernel 自己的 `schema` feature 派生（kernel-SPEC §8-45）；`NoSecret` 手写 `impl JsonSchema` 为 `false`（任何值都不满足），于是 `PutSecret` 臂在 TS 里是 `value: never`——线上拼不出它，这一句在两端各说一次、意思相同。`Command<Secret>` 以 `schemars(rename = "Command")` 命名，因为线上只有 `Command<NoSecret>` 一种实例。
- **生成器只认 serde 会产出的那个子集**：对象（`properties`／`required`／`additionalProperties`）、`string`／`integer`／`number`／`boolean`／`null`、`array`（`items`）与元组（`prefixItems`）、`enum` 字符串表、`const`、`oneOf`／`anyOf`、`$ref` 指向 `#/$defs/…`、`type: [T, "null"]`、`true`／`false` 两种布尔 schema。其余一律拒绝并点名关键字与所在类型——一个会猜的生成器就是一个会静默产出错类型的生成器。具名的裸 `string`／`integer` 即 newtype，TS 侧打上 `Schema.brand(名)`。
- **文件确定**：`$defs` 按名排序后按依赖拓扑输出（Effect 的 `Schema` 值必须先定义后引用；环即拒绝），对象键排序，LF 行尾，生成头注明来源。

**被否**：（a）在 wire 用 schemars 的 remote derive 镜像 kernel 的四十个类型——每个镜像是同一形状的第二个权威，而 §8-1 第 5 条早已为 `GitOid` 拒过同一形状的提案；（b）把 schema 文档的摘要作为握手哈希——doc 注释入哈希，改一句注释即旧页面全拒；（c）手写 `wire.ts`——正是本节要关掉的那扇门。

**`answer.rs` 随之切出 `answer/building.rs`**：二十六条 `cfg_attr` 派生行把 381 行推到 407 行，越过 400 行预算，故一栋楼说自己的七个读形状（`BuildingProgress`／`BlockedLine`／`PlanRow`／`PursuitLine`／`BuildingDoc`／`ArchiveLine`／`BuildingAnswer`）迁入 `crates/wire/src/answer/building.rs`，`answer.rs` 以 `pub use` 引回，公开拼写不变；文字逐字节照搬，无字段开放。**记法同 §8-14**：下游基线里定义位路径从 `wire::answer::BuildingAnswer` 变为 `wire::answer::building::BuildingAnswer`（`web` 基线一行），那是 `cargo public-api` 记录的定义模块，不是接口变更。

**本节的公开面变更**：wire 多出 `wire_schema`（仅 feature `schema`，缺省基线不见它）；kernel 在 `--all-features` 下多出四十余条 `JsonSchema` 实现（缺省基线不见）；`web` 基线因上述路径变动重生。

### 8-17 `Query::Commit`：一次提交出自哪次运行

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
- **答案从账本来，不从 git 来**（storage-SPEC §8-18）。一座导出后在别处恢复、
  `.git` 不在身边的城，照样答得出自己的历史。
- **客户端欠的（前端冻结，此处不画界面）**：`client/src/wire.ts` 需重新生成
  （`cargo xtask wire-ts --write`）；只把新变体接进
  `mount::frame` 那条「有答案而暂无页面问它」的臂，使其仍能编译。

### 8-18 `CommitAnswer.lineage`：一次提交背后的接替链

`CommitAnswer` 增 `lineage: Vec<RunId>`：本跑在前，逐级向前到第一任；没接替过谁的跑是长度 1 的链。名字表没动，语法换了形——正是 `WIRE_V` 存在的那种情形，于是 14→15，golden 由 `730e9d0b…` 变为 `24b7e8ff3727cad505653c951a6733b748cb294f9eec418adefb3c4a7e7223b9`。服务端从 `run_started` 的 `predecessor` 键折出 `predecessors` 表（`accounting::views::commits`），答时沿表走链。客户端读它的页尚未画，`crates/web` 只需编译通过；新客户端欠一行「replaced <run>」。

### 8-18b 派活帧不再携上限

```rust
Dispatch { addr, task, goal, policy, idem, session, effort }   // 删去 budget: BudgetCap
```

**没有人能在一件事跑之前给它定价**，所以说出「跑这件事」的那条帧不带上限。刹车只留一个：`Halt` 关掉一个范围并终止该范围里已经起来的后台成员（`runtime::backlog` 使这句话为真）。`kernel::BudgetCap` 及其判定面随之删除（kernel-SPEC §8-12），`wire` 的 kernel 再导出列表因此少一项 `BudgetCap`——**这是公开面变更**，`web` 与 `sprawling` 两份基线同变更集重生。

- **`BudgetUse` 留在再导出列表里**：成本页读它，五路归因报它。**报告花了多少**与**事前不许花**是两件事，此处只删后者。
- **`Dispatch` 的 reach 不变**（§19-2 仍是 `client`）：删的是一个字段，不是一个动词。
- **旧客户端**：`WIRE_V` 进位后在握手期被明确拒绝，所以一条仍然写着 `budget` 的帧到不了服务端；服务端也不再有那个字段可读。

### 8-19 治理两帧：写身份文件，读被代答的事

```rust
// Command（第 24 条）
PutDocument { which: GovernedDocument, base: String, body: String, idem: IdemKey }
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
- **`PutDocument` 携 `base`**：三份文件有两个写者——原文编辑器与设置页上的卡片，或者开在两处的两个页面——所以一次保存说明它起手时的全文，文件已经变了就拒，什么都不写（§8-59）。`edit` 工具碰不到这三份文件，居民不是它们的写者。
- **被否**：（a）三条命令 `PutMayor`／`PutClerk`／`PutPreferences`——同一条规则三个入口，加第四份文件要改三处；（b）复用 `edit` 工具——`edit` 走写域，而写域恒不含保留子树，让它开一个例外就是把「居民改不了治自己的东西」这条最老的规矩打穿。

### 8-20 一段补丁是它自己的一次请求

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

**这与 `storage::changes` 的模块头不矛盾，它就是那句话说的那次请求。** 那段头写着「计数，永不补丁文本……一段补丁必须是它自己的一次请求……而不是这个模块」。`Query::Changes` 答的是哪些文件动了、动了多少行；本查询答的是**一个文件**的补丁文本。两者不是同一个答的详略两版：前者的代价与改动文件数同阶，后者与一个文件的大小同阶，把它们并成一个答会让「看看这次改了哪些文件」付上整批补丁的代价。

- **没有它，本版开篇承诺的那件事在浏览器里做不到**：审一个 PR 得开终端敲 `git diff`。
- **同一次凭证扫描，不是第二份**：补丁文本经 `storage::checkpoint::scan_staged` 的同一个判定过一遍。命中凭证形状的那一行**不回显**，答里只留它的行号与原因（`Withheld`）。第二份扫描器就是同一条规则的第二个权威，而漂掉的那个总是没人读的那个。
- **一次一个文件**：`path` 是必填的，没有「整批补丁」这个形状。
- **两个 oid 都不可变，所以这个答任何人都可以永久缓存**（同 `Changes` 的理由）。
- **这座城没写过的 oid 答 `Unavailable`**，与 `Changes`／`Commit` 同口径。

### 8-21 四种读法回到服务端

```rust
// Query 第 18、19、20 条（声明序，QUERY_NAMES 同序追加）
Rounds   { run: RunId },      // → Answer::Rounds(Box<RoundsAnswer>)
Evidence { run: RunId },      // → Answer::Evidence(EvidenceAnswer)
CostOf   { node: NodeId },    // → Answer::CostOf(CostOfAnswer)

pub struct RoundsAnswer {
    pub run: RunId,
    pub turns: Vec<Turn>,
    pub opened_at: Option<GitOid>,   // 本会话的第一个检查点
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
pub enum Note { Refused { error: AxError, at: Seq }, Checkpointed { oid: GitOid, at: Seq },
                Waiting { at: Seq }, Arrived { from: String, said: String, at: Seq },
                Discarded { count: usize, at: Seq }, Unreadable { cause: String, at: Seq } }

pub struct EvidenceAnswer { pub run: RunId, pub items: Vec<EvidenceItem> }
pub struct EvidenceItem { pub at: Seq, pub kind: EvidenceKind,
                          pub locator: Locator, pub picture: Option<Picture> }
pub enum EvidenceKind { Screenshot, Finished }
pub struct Picture { pub media_type: String, pub width: u32, pub height: u32 }

pub struct CostOfAnswer { pub node: NodeId, pub spent: UsdMicros,
                          pub runs: Vec<(RunId, UsdMicros)> }
```

**这里搬的是读法而不是接口。** 一个会话被读成回合、一次跑留下什么证据、一个计划节点花了多少钱——这三件事此前只有 `crates/web` 会算，于是「线就是全部 API」（ARCHITECTURE §8）在这三处是假的：另写一个客户端就得把折叠逻辑照抄一遍，而照抄出来的那一份迟早与这一份不一致。现在三者各是一次查询，答由 `accounting::views` 折出（sprawling-SPEC §8-47）。

- **`Rounds` 的值类型住 `wire`，折叠住 `accounting::views`。** 值要上线，故必须可序列化；折叠要读账本，故必须在能读账本的那一层。两者切分开来，正是 ARCHITECTURE §9 的形状 2 与形状 7 的分界。
- **`wire::reading` 是第三块**：把一条账本载荷读成上面这些值的那些纯函数（`said_in`／`used_in`／`output_in`／`note_of`）。它住在线这一层而不是服务端，因为**两端都要读**：服务端答 `Rounds` 要它，客户端把推来的 `model_returned` 折进自己的快照也要它（ARCHITECTURE §5 第 12 步：同一个折叠，线的两边）。一份权威，两个调用者。**`Call.subject` 不由这里算**：写方在写 `tool_called` 时把它定下（kernel-SPEC §8-4：`ToolCalled::subject_of`），折叠读记录里的 `subject` 键；两个读方各按自己的 map 序推一次，同一次调用已经出现过两个名字。
- **读不出的载荷是一条 `Note::Unreadable { cause, at }`，不是没有 note**：`note_of` 认下的种类（被拒、检查点）若载荷读不回它该有的形状，答里留一行，`cause` 说哪一种事件、读到哪一步失败，`at` 指向账本里那条记录。被否：返回 `None`——那样一次被拒在人眼里就是「什么都没发生」，而失败本身被这一层抹掉了。`CheckpointCommitted` 的 `JobPinned` 是一个真答案（派发钉住的是作业不是提交），仍然没有 note。
- **`Changes` 早已在线上**（§8-20），`storage::changes` 一直是它唯一的权威；查过之后不动它——把一件已经做完的事再做一遍就是造第二个权威。
- **`Evidence` 只认写下来的东西**：截图是 `tool_result` 载荷里的 `image` 定位符（`bin::browser_tool::stored` 写的那三项：定位符、两条边、media type），完成证据是 `roadmap_finished` 载荷里的 `evidence` 定位符。**答里恒不携字节**：一张图是一个 `cas:` 定位符，取它是资产端点的事，把 base64 塞进查询答会让「看一眼这次跑干了什么」付上整批像素的代价——与 §8-20 拒绝整批补丁同一条理由。
- **`CostOf` 的分母不在这里**：答只报这个节点上归到的绝对金额与逐跑明细，不报占比。占比需要一个这一端没有的分母（整城总额是 `CostView` 的），而没有分母的百分比正是 `UnplannedProgress` 拒绝拼出来的那种东西。节点到跑的映射由 `roadmap_claimed` 折出（载荷里的 `node` 与记录的 `addr`），钱由 `storage::attribution` 的 `by_run` 给——**不新增任何计价处**。
- **一个本城没认领过的节点答 `CostOf { spent: 0, runs: [] }` 而不是 `Unavailable`**：与 `Changes` 那一条相反，因为这里「没人认领过它」是一个真答案而不是「我读不了」；节点地址本身经 `NodeId` 的手写 `Deserialize` 把过关，读不了的形状根本上不了线。
- **`RunCosts { runs: Vec<RunId> }` → `Answer::RunCosts(RunCostsAnswer { runs: Vec<(RunId, UsdMicros)> })`**：`CostView.by_run` 之外的跑逐个按名字问，一次最多 `RUN_COSTS_MAX`（64）个，按问的顺序答；读不出的跑不出行，零是「没花钱」。钱仍只由 `storage::attribution` 折，冷的一侧从账本折（sprawling-SPEC §8-90）。
- **`WIRE_V` 15→16，一次进位管三条查询**：名字表长了三项（17→20），故 schema 哈希无论如何都要变。旧页面在握手期被明确拒绝，这正是该机制存在的理由。
- **被否**：（a）把 `Rounds` 并进 `RunHistory` 的答——前者是折叠后的读法，后者是原始记录页，一个答两副形状会让翻页与折叠互相牵制；（b）让 `Evidence` 直接回字节——见上一条；（c）把折叠留在 `wire` 里由客户端调用——那样新客户端仍要自己跑一遍折叠，而这一节整件事就是不要它这么做。

### 8-22 没有监听器的那份构建，测试也不许提它（`tests/enrolment.rs`、`tests/wire_contract.rs`）

`cargo clippy -p sprawling-wire --no-default-features --all-targets` 是红的：`tests/enrolment.rs` 整份都在驱动 `wire::router`，`tests/wire_contract.rs` 有三条断言在问 `decide_bind`／`decide_handshake`，而这三样连同 `axum`、`tokio` 都由 feature `server` 带进来。**这份构建正是给 `web` 用的那一份**——它需要本 crate 的词汇而不许把 TCP 栈拖进 WebAssembly；一个在这里名词都拼不出来的测试文件，把它自己的红判在了产品的一条真路径上。

**按测试真正需要的东西设门，而不是把 feature 打开**：`tests/enrolment.rs` 首行 `#![cfg(feature = "server")]`（整份文件都是路由的事）；`tests/wire_contract.rs` 只给那三条断言与它们的两个辅助函数、以及 `Hello`／`Welcome`／`AxCode`／`SocketAddr` 这几个只被它们用到的名字加 `#[cfg(feature = "server")]`——命令表、查询表、schema 哈希与那两个不可拼写的形状**在两份构建里都被判**，因为它们在两份构建里都成立。

### 8-23 新客户端第一次真正用这条线，线上缺的四件事

```rust
// RunSummary 多两个字段（storage::RunHot 从 run_started 记下，storage-SPEC §8-5）
pub struct RunSummary { pub run: RunId, pub who: String, pub frozen: bool,
                        pub last_seq: Seq, pub last_kind: EventKind,
                        pub addr: Option<Address>, pub started: Option<TimeMs> }

// CityAnswer 多一个字段：被 halt 的 scope 名（`city`、`<building>`、`<workshop>`），BTreeSet 序
pub struct CityAnswer { ..., pub halted: Vec<String> }

// RoundsAnswer 多开场与收场
pub struct RoundsAnswer { pub run: RunId, pub turns: Vec<Turn>, pub opened_at: Option<GitOid>,
                          pub opening: Option<Opening>, pub closing: Option<Closing> }
pub struct Opening { pub task: String, pub goal: String, pub at: TimeMs,
                     pub dispatched_by: Option<Who> }   // §8-48
pub struct Closing { pub completion: String, pub at: TimeMs }

// Query 第 21、22 条（声明序，QUERY_NAMES 同序追加）
Listing  { at: Option<Address> },   // → Answer::Listing(ListingAnswer)；None 是城根
Document { at: Address },           // → Answer::Document(Box<DocumentAnswer>)

pub struct ListingAnswer { pub at: Option<Address>, pub entries: Vec<Entry> }
pub struct Entry { pub name: String, pub kind: EntryKind }
pub enum EntryKind { Directory, File { bytes: u64 } }          // 目录在前、文件在后，各按名字序
// DocumentAnswer 的形状见 §8-69
```

**这四件事都是同一个发现**：ARCHITECTURE §8 说「线就是全部 API」，而旧客户端从没把这句话当真——它在浏览器里折叠 `history` 的原始记录，所以从来没问过线「这次跑在哪个房间」。新客户端只问线不折历史，四处空白一次全露出来。

- **`RunSummary.addr`／`started`**：`who` 是这次跑第一条记录的作者，恒为 `city`，不是房间。房间是 `run_started` 记录自己的 `addr`，热视图在那一条上记下它（storage-SPEC §8-5）。没有它，页面无法把 `city_view` 列出的 run 归到 `hall/mayor`，「与 Mayor 的对话」拼不出来。`Option`：热视图可能只看到没有开场的一段尾巴，看不到的事不猜。
- **`CityAnswer.halted`**：`city_halted` 是记录，`halted_by` 是 `bin::assembly` 工作线程的判定，而页面刷新后两者都够不到——它只收此后的事件。答里带上被 halt 的 scope 名，一个刚打开的页面才知道城是不是停着的，而不是等下一次 dispatch 被拒才发现。名字与 `HaltScope` 的 `scope_name` 同拼法，页面按名字画。
- **`RoundsAnswer.opening`／`closing`**：回合的折叠从第一条 `model_called` 开始，所以人说的第一句（`run_started.task`）与这次跑怎么结束的（`run_frozen.completion`）都不在答里；一段对话缺开头与结尾就不是对话。两个都是 `Option`，理由同 `addr`：`HISTORY_MAX` 那段窗口可能不含开场。
- **`Listing`／`Document`**：这座城是一棵目录树，而目录树本身就是产品（glossary：「那个层级就是目录树——不是它的模型，是树本身」）；`building_view` 只回楼根的 `.md` 与房间名，房间里的 `URBANITE.md`／`JOB.md`／`Handoff.md`／`<run>.jsonl` 页面看不到，于是这个设计在界面上是不可见的。两条查询让页面能走完整棵树。**路径经 `Address` 文法把关**（非绝对、无 `..`、无 `\`、无 `:`），所以走不出城根；`.sprawling/` **允许读**——它正是要展示的那部分，且这条线只答回环（或持配对 token 的）人，与工具层对居民的拒绝不是一个门。`Document` 答什么、在哪里切、怎样判文本，见 §8-69：答复带版本，缺失、读不了、空各是一种答复。
- **`WIRE_V` 16→17，一次进位管四件事**：四件事同一提交同一哈希。
- **被否**：（a）让客户端自己折 `history` 找 `run_started`——那是旧客户端的做法，也是这四处空白存在的原因；（b）`Document` 直接回任意大小——同 §8-20／§8-21 拒绝整批的理由；（c）`Listing` 排除 `.sprawling/`——排除了要展示的东西。

### 8-69 文档读取契约：`Query::Document` 答一个版本

```rust
// Query（形状不变）
Document { at: Address },                    // → Answer::Document(Box<DocumentAnswer>)
pub struct DocumentAnswer { pub at: Address, pub state: DocumentState }
pub enum DocumentState {                     // 线上 "missing" | { unreadable } | { empty } | { held }
    Missing,                                 // 这个地址上没有文件
    Unreadable { reason: String },           // 有东西而读不出：目录、无权限、读到一半出错；reason 是系统的原话
    Empty { version: B3Hash, format: documents::Format },
    Held(Box<HeldDocument>),
}
pub struct HeldDocument { pub version: B3Hash, pub format: documents::Format, pub bytes: u64, pub body: DocumentBody }
pub enum DocumentBody {
    Text { encoding: documents::Encoding, head: documents::Window, coverage: Coverage },
    Opaque,                                  // 不是任何一种本城读的编码的文本：只有版本与大小
}
pub enum Coverage { Whole, Head }            // Head：其余的经 Query::Range 按 version 读
// documents::{Span, Format, Encoding, Window} 由 documents crate 定义，线上直接携带（documents D1）
```

- **版本身份是整份字节的 `B3Hash`**，与内容库给同一份字节的地址相同（documents D3）。下一次保存拿它作基线（§3 的 `PutRange`），页面拿它判断两次读到的是不是同一版。
- **缺失、读不了、空是三种答复**，不再借 `Unavailable`：「这里没有文件」页面画成可以新建，「读不了」页面说出系统的原话，「空」是一份可以写的文件，三者页面采取的动作不同。空文件也有版本（空字节的摘要），因为它同样可以是一次保存的基线。`Unavailable { query: "Document(<at>)" }` 只剩视图本身答不了的情形。
- **文本的判定**（documents D4）：字节顺序标记先判，所以带标记的 UTF-16 是文本；没有标记时，不含 NUL 的合法 UTF-8 是文本；其余是 `Opaque`。不再有损解码，不再凭头 8 KiB 的 NUL 判二进制。
- **第一个窗口** `head`（documents D7）：整份放得下 `WINDOW_BYTES_MAX`（64 KiB）就是整份（`Coverage::Whole`）；放不下时止于放得下的最后一个块的末尾，一块都放不下时止于界内最后一个字符边界（`Coverage::Head`）。窗口从第 0 个字节数起，标记是文本的第一个字符（documents D5），所以页面把各窗口的文本接起来就是整份文本。
- **`Coverage::Head` 的版本在内容库里**：答复发出之前，读面把这一版的字节放进城的内容库（storage-SPEC §8-36），所以之后按版本取范围读的是这一版，而不是文件此刻的样子。整份已经在答复里的版本不存：页面没有理由再要它。
- 验收：accounting 的 `views::document::tests`——缺失、目录、空文件各得各的答复；带标记的 UTF-16 文件判成文本并解出原文；没有标记而含 NUL 的判成 `Opaque`；超过一个窗口的文件答 `Coverage::Head`，它的版本在内容库里。

### 8-70 按版本取范围：`Query::Range`

```rust
// Query
Range { version: B3Hash, range: documents::Span },   // → Answer::Range(Box<RangeAnswer>)
pub struct RangeAnswer { pub version: B3Hash, pub window: documents::Window }
```

- **读的是内容库里的那一版**，不是文件此刻：一个页面滚到第三屏时，文件可能已经被居民改过，而它要的是它开始读的那一版的第三屏。内容库里没有这一版答 `Unavailable { query: "Range(<version>)" }`。
- **请求的是半开字节区间，答的是切好的窗口**（documents D2、D7）：起点在字符中间时退到那个字符的第一个字节，终点往回退到字符边界，长度不过 `WINDOW_BYTES_MAX`，起点在版本末尾之后时答末尾处的空窗口。编码由这一版前三个字节里的标记定，切出的字节在这种编码下拼不出文本（`Opaque` 的版本）时答 `Unavailable`。答复里的 `window.span` 是实际切出的区间，页面从它的 `end` 接着要下一段。
- **只读内容库里要答的那几个字节**（`storage::Cas::size` 与 `Cas::get_range`），所以第三屏的代价是第三屏，与文件多大无关。
- 验收：accounting 的 `views::answering::range::tests`——从 `Document` 的 `head` 末尾起逐段读到末尾，窗口首尾相接就是整份字节；文件在第一次读之后被改写，按旧版本读出的仍是旧字节；内容库里没有的版本答 `Unavailable`。

### 8-24 `Query::Commits`：一座楼做过的提交，倒序分页

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

### 8-25 `Query::Doctor`：运行中的机器有什么

```rust
// Query 第 24 条（声明序，QUERY_NAMES 同序追加）
Doctor,                                   // → Answer::Doctor(Box<DoctorAnswer>)

pub struct DoctorAnswer { pub items: Vec<DoctorItem>, pub tiers: Vec<DoctorVerdict>,
                          pub sandbox: DoctorSandbox, pub custody: DoctorCustody,
                          pub core: DoctorCore }
pub struct DoctorItem { pub name: String, pub tier: DoctorTier, pub need: DoctorNeed,
                        pub homepage: Option<String>, pub state: DoctorState,
                        pub install: DoctorInstall }
pub enum DoctorTier { Use, Develop }
pub enum DoctorNeed { Required, Optional }
pub enum DoctorState { Present { at, version }, Broken { at, fault }, Absent { absence } }
pub enum DoctorVersion { Said { text }, Silent, Unreadable, Late }
pub enum DoctorFault { WillNotStart { said }, HalfWritten, Unreadable { said } }
pub enum DoctorAbsence { NotOnSearchPath, VariableNamesNothing { variable, path },
                         NoComponent { dir }, NoHome, NotInThisBuild }
pub enum DoctorInstall { Command { spelled }, Print { spelled }, Manual { how }, UnknownPlatform }
pub struct DoctorVerdict { pub tier: DoctorTier, pub missing: Vec<String> }

// 与逐件的 items 并列的两道整机读数：命令跑在什么盒子里、凭据住在哪里。
pub struct DoctorSandbox { pub arm: DoctorSandboxArm, pub coverage: Vec<DoctorGuarantee> }
pub enum DoctorSandboxArm { LinuxNamespaces, WindowsJobObject, CopiedTree,
                            Unavailable { missing: DoctorSandboxMissing } }
pub enum DoctorSandboxMissing { ScratchDirectory }
pub struct DoctorGuarantee { pub axis: DoctorGuaranteeAxis, pub kept: DoctorCoverage }
pub enum DoctorGuaranteeAxis { Filesystem, Network, ProcessTree, User, Resources }
pub enum DoctorCoverage { Kept, NotKept }
pub struct DoctorCustody { pub store: DoctorCustodyStore, pub keeps: DoctorCustodyLifetime,
                           pub refusal: Option<String> }
pub enum DoctorCustodyStore { PlatformService, EncryptedFile, SessionMemory }
pub enum DoctorCustodyLifetime { AcrossReboots, WithPassphrase, UntilReboot, ThisProcess }
pub enum DoctorCore { Raised, HeldBySetting, Refused { said }, LoweredByValve, Unasked { said } }
```

- **每一种状态都是枚举，不是句子**。终端那份报告是一台机器的散文，而浏览器说两种语言；线上若携措辞，页面的用词就成了服务端的选择。「有了它能做什么」那一句也不上线：`name` 就是这一项的 id（需求表里的名字，如 `cargo-nextest`），页面按它从 `lang.json` 的 `machine_enables_<name>`（连字符写成 `_`）取两种语言的那一句，终端的英文留在需求表（sprawling-SPEC §12「doctor 的「能做什么」各面自持」）。线上仍携的句子只有 `said`：平台或程序自己说的话，读者推不出来。
- **答的是城启动时看到的那一眼，不是现问现看**。每一项都是起一个进程问版本；一次查询若这么做，会把答一切读的那条线程按住数秒。城若没看过（一次一条命令驱动的工人就是），答 `Unavailable`——与「一栋没人盖过的楼」同口径：**「我没看」是它自己的答案**，而一台空机器会让页面告诉人他手上每件工具都缺。
- **`install` 把平台不明单列一支**。三个平台之外的机器上，本项目没有任何配方；此时拼一条别的平台的命令是错的，沉默也是错的。
- **沙箱的保证逐轴作答，不是一句「已隔离」**：`coverage` 逐轴一行，`Kept`／`NotKept` 两个字而不是布尔——页面两态都要有词，布尔会让每个读者自己给 `false` 选一个。它存在的理由，是 agent 在动手前要读得到哪几条保证没成立。臂与轴的定义住 `runtime-SPEC §8-13-2`（`Confinement` 与 `Guarantee`），线上重拼一份，逐臂对应只住 `sprawling::doctor::report` 的穷尽匹配——上游加一臂即编译红。
- **凭据的存放与寿命一起答，`refusal` 是平台服务自己的话**：三者同出 `gateway::Custodian::probe` 的一次往返（`gateway::Custody`，gateway-SPEC §8-4；寿命的全部档位见 §8-21），线上重拼 `Store` 与 `Persistence` 两套词，逐臂对应同住 `sprawling::doctor::report`；`refusal` 缺席读作服务没有拒——或该 store 由城自选，没有服务可拒。
- **核心线程站在哪一档，`said` 是平台自己的话**：`core` 是主机此刻会给核心线程的档位（sprawling-SPEC §8-93、§8-40）——升到正常档之上一级、按人的 `[core] priority` 留在正常档、平台拒绝（Unix 上没有 `CAP_SYS_NICE`）、被安全阀降回，或 doctor 没能问到。派出的命令不在这里：它们总是低一档，降档从不被拒（runtime-SPEC §8-13-3）。
- **服务端**：`sprawling::doctor::report` 把 findings 与这两道整机读数折成本形状，`Views` 存一份（sprawling-SPEC §8-54）。

### 8-26 `ConfigureBuilding` 长出 `desktop`：一栋楼的桌面白名单走同一条帧

```rust
ConfigureBuilding { addr: Address, sandbox: Option<SandboxLimits>, mcp: Option<Vec<McpServer>>,
                    desktop: Option<String>, idem: IdemKey },
```

- **不新起一条命令，也不给 `GovernedDocument` 加变体**。这条帧问的本来就是「这栋楼的 runs 够得到什么」——沙箱、外部服务器、运行中的机器上的哪些窗口，是同一个问题的三面；各自可缺省，缺省即不动那一面。而 `GovernedDocument` 是**城**的三份文件（`<city>/.sprawling/`），桌面白名单是**楼**的（`<building>/.sprawling/`）：把楼级路径塞进一个按 city_root 取路径的枚举里，会让那个枚举需要一个只有部分变体用得上的参数（city-SPEC §8-26 已写下这条）。
- **`desktop` 是文本而不是解析过的值**。读它的那台 server 是它语法的权威，且 fail closed——读不出来的文件关成全拒。城这一侧再抄一份解析器就是第二个权威，而两个权威里迟早有一个把某份文件读成另一种意思。城只保证「写进去的字节就是人给的字节」。
- **`building_configured` 载荷第三个布尔位 `desktop`**：与 `sandbox`／`mcp` 同形，说的是「这一面被写过」而不是写了什么。`city::Written` 把三个布尔收成一个值——一个调用点写 `(true, false, true)` 说不出哪一位是哪一面。
- **页面读它用 `Query::Document`**：`<building>/.sprawling/DESKTOP.toml`。那条查询本来就明说保留子树在这里可读，理由是「治理一栋楼的东西正是这个视图要给人看的」。

### 8-27 说出来的那句话：`/transcribe` 与 `ModelTag::Transcribe`

```rust
pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;
// POST /transcribe，body 是录音字节，content-type 是浏览器录进的容器；200 的 body 就是那行文字。
```

- **是一条路由，不是一条 Command，也不是一条 Query**。Command 被接下之后经事件流作答，而「我刚说的那句话是什么」必须回到录它的那个标签页；Query 是另一种会作答的形状，而一条在供应方那里花掉数秒的查询就是一条装成读的命令。`/enroll` 与 `/upload` 早已是同一类旁门：帧的文法装不下的那几件事各有一扇 HTTP 门。
- **容器从请求头读，不从字节猜**：浏览器录进它手上有的容器，而只有它知道是哪一个。没有 content-type 即按名拒绝——一个没人声明的容器发不出去。`; codecs=opus` 这类参数说的是容器里的编解码器，而音频线路由的是容器，故取分号前那一段；那一段修剪后为空（头里只有参数）同样是没声明容器，与缺头同一句拒绝。
- **`ModelTag` 增第三个 `Transcribe`**（kernel-SPEC 的枚举表同步）：**「哪个 endpoint、哪个 model 答这一类活」本来就有机制**——人登记一个 endpoint，再为一个 tag 选一个 model。第二张表单加第二份存储会是同一个问题的第二个答案，而那把 key 还要有第二条进金库的路。人填 URL 与 key 因而走的是既有的 attach 表单。
- **服务端**：`gateway::transcriber_for(chosen, secrets)` 与 `adapter_for` 同形——把一个选择变成一件可调用的东西这件事只在一处发生。`Views::transcriber` 在锁内读出选择、锁外发请求。

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

### 8-29 一个端点带着人给它定的规矩上线

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
- **零即缺省**。清空一个数字框到达线上是 `Some(0)`，而没有请求能在 0 ms 内完成；装配层把零读成「没说」（`accounting::worker::credentials::tuning_of`），于是清空一个框等于回到城自己的值，而不是让此后每一次调用立刻失败。
- **`stream_idle_timeout_ms` 在线上保留 Codex 的名字，在 gateway 里叫 `stream_deadline_ms`**：阻塞传输交回的是一个没有分块钩子的 body reader，城因此能限定一次应答总共多久，限定不了其中某一次沉默多久。线上用人在自己 `config.toml` 里写熟的那个词，gateway 用它真正做到的那件事命名，装配层是唯一的翻译点。**败给的方案**：在 gateway 里也叫 idle——那会让一个读代码的人以为分块之间有计时器。
- **`Turn` 携 `thought`**。thinking 块为供应方的签名校验端到端携带，看起来属于传输；但对一个把大部分调用花在推理上的模型，挡住它就是让人先对着空线程等几分钟，再读到两句话。所以推理以自己的字段作答，页面把它折起来放在散文旁边，两者永不混进同一个缓冲区。`RedactedThinking` 仍然不出现：它的载荷是加密的，里面没有人能读的东西。
- **`EndpointSummary` 长出 `label`**：缺省即 `name`，所以页面永远不必替一个没写显示名的端点决定显示什么。
- **`proxying` 跟着 tuning 走，因而探测与调用恒用同一个决定**（WIRE_V 27→28）。一个只在调用时生效的代理设置，会让表单上那份分段读数描述一条真正的调用不会走的路，而那份读数存在的全部意义就是告诉人调用停在了哪一段。`Option` 而非值：线上的缺席是「没人定过」，装配层把它翻成城的默认值（`ExceptLocal`），于是 `gateway` 一侧拿到的是一个已经定下来的值，没有第三种状态要每一个调用方再答一次。

### 8-32 第三类之外的第三类：`ServerFrame::Log`

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

### 8-33 `DoctorInstall` 与 `DoctorRefresh`：机器上的两个动词

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

### 8-34 一台工具服务器带着 `claude mcp add` 允许写的东西上线，`McpHealth` 答它站在哪

`McpTransport` 的三支携 MCP 页的环境变量表、多行请求头与 `sse`，且**不标 `#[non_exhaustive]`**（全库如此）：本枚举的读者全在这一个二进制里，通配臂什么也换不来，却会把下一种 transport 从必须表态的三个模块面前藏起来。

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
5. **这是唯一一条按秒计的读**，每台服务器一次 `initialize` ＋ `tools/list`，且用的就是 Run 起点那一次握手（`agent_protocols::mcp`）。它恒不由记录触发、也不上定时器——人打开 MCP 页或加完一台服务器时问一次。**被否**：把握手结果写进账本再折出来——一台服务器此刻通不通是关于此刻的事实，记下来的那一份会在它停掉一小时后仍说它在。

### 8-35 四条读：一个 agent 被告知了什么、一栋楼会做什么、它那里还有什么没提交

这四条读回答三个问题：「这个 agent 收到的 system prompt 是什么」（`prompt_assembled` 只记四段的哈希与来源注记，拿着哈希读不回原文）；「这栋楼会做什么」（`Query::RegistryView` 的书里没有 skill）；「此刻工作树在哪」（`Query::Changes` 比的是两个检查点，答不出分支名、与上游的距离、以及哪些文件还没进检查点）。

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

pub enum SkillShelf { Library(Address), Building(Address),
                      External { index: u32, path: String } }
pub struct SkillLine  { pub name: String, pub section: String, pub shelf: SkillShelf,
                        pub disclosure: String, pub hash: B3Hash,
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
4. **技能一行而三类架子，`shelf` 同时说它来自哪一格、落在哪里**：按架子各造一张表会让「近的架子压过远的架子」这条既有规则在客户端被重写一遍。两个城内臂携地址，城外臂携第几条挂载与相对该架根的路径——城外那份没有地址，编一个会送读者去开一个不存在的文件（布局与优先级见 city-SPEC §8-8）。`SkillLine.at` 随之删除：架子与落点分成两个字段，就有「说 library 却指向城外」这一态可写。`pinned_by` 按**名字加哈希**成对匹配——两次 run 之间被编辑过的 skill 是同一个名字下的两份文档，只按名字匹配会宣称早先那次 run 读到了后来才写的字。
5. **`Drift` 整个可缺席，而不是两个零。** 没有上游的分支与和上游齐平的分支不是一回事，读成 `0/0` 的页面会告诉人「你的工作已经推上去了」。
6. **`GitStatusAnswer.checkpoint` 携整条 `CommitAnswer`。** 变更栏旁边那一行要说出 run、房间、模型与花费，而这四样已经有了唯一形状；另造一个摘要类型就是第二个「一次提交是什么」。

**`CommitAnswer` 携 `spent: UsdMicros`**：一行提交画得出 run、房间与模型，也要画得出钱，「这次改动花了多少」才不必另开一页去查。携的是**那次 run 的总额**而不是这条提交的份额——检查点不被计价，把一次 run 的钱按检查点分摊会得到一个没有人测量过的数字。

**被否**：把 skill 与工作树的状态折进 `BuildingView`。楼页的那一帧是在每一次记录之后都会失效的读，而扫书架要走盘、读工作树要开仓库；合成一帧会让这两件慢事按城里的心跳重复发生，而它们各自只在有人打开那一栏时才需要一次。

### 8-35b 外包服务的目录与一键连接：`Query::Toolkits` 与 `Command::ConnectToolkit`

人给出一把 key，城替他列出目录并建连接，不要他去对方控制台取回 server id 与 user id：建 auth config 不必访问对方后台（`POST /api/v3/auth_configs` 可建托管配置），而正在退役的是 `initiate` 而非 `link`（`docs/third-party.md`）。

```rust
Query::Toolkits,                                        // 不带参数，也不带 key
Command::ConnectToolkit { toolkit: ToolkitSlug, idem: IdemKey },
Answer::Toolkits(Box<ToolkitsAnswer>),

pub enum ToolkitsAnswer {
    Unenrolled,                                  // 还没存 key，这是第一步而不是失败
    Shelf { toolkits: Vec<ToolkitLine> },
    Refused { refusal: Box<AxError> },
}
pub struct ToolkitLine { pub slug: ToolkitSlug, pub name: String,
                         pub auth: String, pub standing: Standing }
pub enum Standing {
    Absent,
    Awaiting { consent_url: String },
    Connected { alias: String },
    Refused { refusal: Box<AxError> },
}
```

**六条口径：**

1. **目录不由本仓库持有。** 目录由 `GET /api/v3/toolkits` 出；客户端里硬写一份应用清单就是一份保证会过期的第二权威。
2. **三态穷尽，而不是一个可能为空的列表。** 「你还没给这座城 key」「问不到对方」「对方确实没有可连的」是人接下来要做的三件不同的事，而一个空 `Vec` 把三句话说成了一句。
3. **`Standing` 四态，每一态对应一个不同的下一步**，且每一态都是对方此刻的读数，不是城记住的东西——昨天打开过一个同意页面，不构成今天已经连上的证据。
4. **不进账本的是状态，进账本的是请求。** `EventKind::ToolkitLinkOpened` 只记「谁在何时请求连接哪个应用」；站位属于 §8-31 判过的「关于此刻的事实」，而 consent URL 是一张能力凭证——记进可重放的账本就等于把它发给每一个重放的人。
5. **同意页面由客户端打开，不由城打开。** 人坐在客户端那一侧；城可能跑在另一个房间的机器上，在那里弹出的浏览器没有人在看。按钮在同一次点击手势里先开一个空白标签页，等答案回来再给它地址——浏览器会拦掉往返之后才发起的弹窗，而按了按钮却什么也没发生的人分不清是弹窗被拦还是城坏了。
6. **全程不轮询。** 页面在三个时刻重读：打开页面、按下连接、以及**从同意页面切回本窗口时**。最后一条是这条流程不需要任何定时器的原因——人离开去授权再回来，「回来」本身就是那个事件。`docs/third-party.md` 边界 2 禁的是「有什么新东西吗」的定时订阅，而带死线的一次握手收尾不是它。

**被否**：让 `ConnectToolkit` 直接把 consent URL 作为命令的答案回去。`Reply` 只运送拒绝（`wire::reply`），把一个成功结果塞进 `AxError` 是为一次往返伪造一条错误路径；URL 由随后的 `Query::Toolkits` 从对方这个「此刻」的权威取回，客户端因而只有一条渲染路径而不是两条需要互相对齐的。

### 8-36 这是哪一版，npm 上是哪一版：`Query::NewestRelease`

```rust
pub enum Query { /* … */ NewestRelease }

pub struct ReleaseLine { pub version: String, pub released: String }
pub enum ReleaseAnswer {
    Stands { mine: ReleaseLine, newest: ReleaseLine, verdict: ReleaseVerdict },
    Unreleased { newest: ReleaseLine },
    Refused { refusal: AxError },
}
```

**五条口径：**

1. **人按下才发生，此外一律不发生。** 不在连上时问，不在定时器上问，也不搭另一个查询的车。`QUICKSTART.md` 的开场承诺是「什么都没装、没注册服务、删掉文件夹就干净」，一座按自己的时间表去够注册表的城，等于拿那句承诺去换一个没人问过的问题；§8-35 口径 6 对外包目录立的是同一条规矩。客户端因此在**按钮的处理函数里**调 `asking.ask`——Solid 在那里不给响应式 owner，于是这条答复没有 watcher，重连与事件都不会替人重问。
2. **问 npm，不问 GitHub。** 本项目每一次发布都是 pre-release，而 `GET /repos/{owner}/{repo}/releases/latest` 按设计排除 pre-release——它对本仓库答 404。看上去最像的那个端点恰是错的那个；npm 的 `latest` dist-tag 才是 `bunx sprawling` 真正解析的东西。
3. **三态穷尽，而第三态携拒绝。** 「你跑的是某个发布版，它站在这里」「你自己从源码构建的，没有可比的对象」「注册表读不到」是人接下来要做的三件不同的事。第三态不走 `Answer::Unavailable`：城是可用的、注册表不可用，页面必须能说清是哪一个，而拒绝里带的是 `kernel::reach` 已经定义的分阶段读数——名字没解析、连不上、握手失败、对方答了什么状态。
4. **判定在 Rust，页面只画字符串。** `ReleaseLine` 携两个已渲染好的串，谁比谁新由 `kernel::Release` 的 `Ord` 判——版本在前、日期在后，与 npm 对同样两个串的排序一致。客户端因此没有第二套排序规则可以漂掉。**败给的方案**：把六个数字发给页面自己比——那是把一条领域规则复制到另一门语言里。
5. **什么都不更新。** 归档路径归 `sprawling install`，npm 路径归 npm，第三方去覆写其中任何一条，就是「这个二进制住在哪」有了第二个权威（`tools/xtask/src/channel/shim.js` 已立此规）。因此终端与页面都只把该跑的命令印出来就停。

### 8-37 读不出的那一帧不是一次断线（`reception::inbound`）

**规则**：一帧 JSON 在任一侧解不出，就是两端对 wire 的分歧，判 `E_WIRE_MISMATCH`；**恒不把它表达为一次连接断开**。

- **服务端**（`wire::reception::inbound`，形状 1 判定）：`Inbound::read` 是这一侧唯一的入站解码点；解不出即计数一次并产 `SessionStep::Refuse { close: false }`，subject 写「这是本会话第几帧读不出」与本服务端说的 `WIRE_V`，恒不回显对端字节（一个 peer 的帧是它自己的内容，抄进日志就带走了它携的东西）。壳（`server::socket`）因此对「读不出」与「读得出后判出的拒绝」走同一条分支，两种拒绝只有一条送达路径。
- **客户端**（`client/src/core/link.ts`）：`LinkEvent::undecodable` 与「服务端送来的 `E_WIRE_MISMATCH` 拒绝帧」都把链路置为 `refused`，梯子停摆；`core/socket.ts` 据 `isRefused` 取消已排期的那次重连并丢掉这条 socket。措辞随 `AxError` 的三段走（action／subject／recovery），与握手期的版本不匹配同一渲染处（`views/refusal.svelte`），故人看到的是「刷新页面取这台服务端配的客户端」加一个重试按钮。
- **退避计数的清零点是「收到一帧数据」，不是「握手成功」**。握手完成即清零时，一条「连上、打招呼、还没说话就断」的 socket 让梯子永远停在第一级 250 ms——一个对人不可见、也长不大的热循环。

**为什么服务端不关连接，而握手期仍然关**（§8.5「握手的失败处置取断连」不变）：握手期页面还没上来，断连就是它唯一能读到的信号，而它读到的正确；握手之后页面**已经把断连读成网络故障**并按梯子重连，于是同一帧在下一条 socket 上重现，人得到一张永远在重连的空白页。告诉它一次，它就停一次。

**计数的用途**：让「同一个旧页面反复撞同一堵墙」在人看到的那条拒绝里可见。它不改变行为——不设阈值、到点不关连接：阈值只会把刚被移除的那条静默断线路径原样请回来。

**被否**：①继续以关连接表达解码失败——这正是 `WIRE_V` 31→32 时旧页面会撞上的路径；②给不可读帧设上限、超限即关——参见上一条的理由；③把解码失败降级为「忽略这一帧」——两端的分歧不会因为丢掉一帧而消失，而下一帧照样读不出。

### 8-38 一帧的拼写只有一处：名表由枚举生成

**规则**：`Query` 与 `Command` 的变体表是这个 crate 里唯一拼写帧名的地方；`name()` 与 `QUERY_NAMES`／`COMMAND_NAMES` 由 `wire::named_frames!` 从同一份变体表生成。

**一个拼写，一个家**。手写时一个 `Query` 的拼写写在三处：变体自身、`name()` 的手写臂、`QUERY_NAMES` 数组；握手赖以成立的 `schema_hash()` 只读第三个。穷尽 `match` 挡得住「新变体不写 `name()`」，挡不住「新变体不进名表」。生成之后，名表就是变体表本身，它不能与枚举不一致，因为它没有第二份内容可以不一致。

**为什么它值一次升版**。名字一个没改、字段一个没动、语义一个没变，但生成出来的两张表按**声明顺序**排，而 `COMMAND_NAMES` 的手写顺序不是声明顺序（`Wake` 手写在第 2 位，声明在第 25 位）。`schema_hash()` 按表的顺序混入名字，于是哈希变了。**哈希变即旧页面必须被拒绝**，这正是 `WIRE_V` 存在的理由，故 31→32 与本次同集。客户端侧只有 `client/src/wire.ts` 由 `xtask wire-ts --write` 重生，没有手改。

**为什么是一个宏而不是两个**。`Query` 与 `Command` 的差别只有一个泛型载体（`Secret`），其余逐字相同；`carried_name!` 已经为四个 newtype 用过同一手法，这是复用既有机制而不是造相似物。

**被否**：①保留手写表、加一道 `xtask` 闸去比对——那是给两个家配一个裁判，而不是把它们合成一个；②用 `strum` 之类的派生宏——多一个依赖换一段本仓库五十行就写得出、且要按本仓库的文档口径读的代码。

### 8-39 七件线上的小事：闭集、缺席、两个文件与一个兼容格式

**升版的代价是 `wire.ts` 重生与客户端同改，与改动数量无关，分两次就是付两次**，所以能同时落地的线上改动放进同一次升版。以下各件与 §8-38 同属一次升版。

**一、`mode` 是 `kernel::model::Mode`，不是自由文本。** 自由文本的 mode 要由上游把认不出的词落到某个默认值上，于是拼错一个词得到一个没人要的 run 和零句话。`Mode` 是闭集（今天 `chat｜work`，§8-57），未知词在反序列化处即拒。**定义落在 kernel 而不是 wire**：与 `DialectKind` 同一条依赖倒置，wire 携带它、`runtime` 求值它，两边都不得指名对方。`carried_name` 因此只剩三个真正开放的名字（provider／template／toolkit）——**值集开放才进那个宏，闭集不进**。

**二、`context_tokens` 是 `Option<Window>`。** `Window` 与 `Ceiling` 同形（非零新类型）而**不是同一个类型**：一个界定模型能读多少，一个界定它能写多少，互换仍能编译的两个数不该共用一个名字。零在类型上不存在，缺席是 `null`；旧编码把「没人填」写成 `0`，于是上下文提醒拿一段对话去比对一个没人给过的数。

**三、`HandOff`；没有 `CreatePolicy`。** `HandOff { item, to, idem }` 把一条等待中的设计问题交给一位居民，写 `question_handed`；`SetAutonomy` 仍然回答「谁答全部」，两者是不同射程的两个决定，故是两条帧。`CreatePolicy` 随升级机制一起删——没有升级就没有可豁免的。

**四、没有上传链条。** 线上没有 `Attach`、`/upload` 与 `UploadId`：一条只写不读的链会把字节永久落进城目录，无保留期、无清理，而消费它的路由臂只能拒为 `not_built`。

**五、`agent_protocols::Incoming` 是唯一入站文法。** 若路由把请求反序列化成另一个结构再逐字段搬进 `Incoming`，`Incoming::parse`——那个拒绝空 task／空 goal 的构造器——就**只有测试在调**，rustdoc 承诺的「没有完成定义的 run 报不出自己完成了」在真实编辑器路径上不成立；同一次搬运还把明文令牌抄进一个 `derive(Debug)` 的结构。现在 `AcpSink` 收 `&serde_json::Value` 与 `Pairing`，门只读 `token` 一个键用于判定，其余的键由 `parse` 读——**一个文法一个家**。`Pairing` 是枚举而不是 `bool`：传反了不该还能编译。

**六、`EndpointSummary` 说得出自己是怎么连的，也说得出它服务的模型。** 它携 `connection_kind: String`，取 `ConnectionKind::as_str` 的七个扁平词之一；`models: Vec<String>` 升为 `Vec<ModelFactsSummary>`（上限、模态、价格原文）。`dialect` 留在旁边，它答的是更窄的一问——**哪支笔写请求**；两种连接可以共用一支笔而仍是两次不同的登记，只显示笔的页面说不出人当初设的是哪一个。`ModelFactsSummary` 每个字段都是**上游说过的话，不是本城的结论**：缺席就是那一行没说，不在此处补预设表，因为事实梯要在调用处爬一次，答案已经爬过的摘要就是第二个答案。

**七、人能编辑的两个文件各得一扇门。**

| 帧 | 形状 | 为什么是这个形状 |
|---|---|---|
| `Query::Preferences` → `PreferencesAnswer` | `lang: Option<Lang>`、welcomed、panel、`Appearance`、`proxying`、改过的和弦；`#[serde(default, deny_unknown_fields)]` | 浏览器曾按行缓存这些：十二个键、三个读取器，各自处理缺省与非法值。整表一扇门，允许值表由本 crate 声明一次并经 schema 传给客户端——**能画出来的选项就是这个 build 装得回的选项** |
| 同上：本类型兼任 `[ui]` 的文法 | `[ui]` 节就是 `PreferencesAnswer` 的序列化；缺席字段取 `PreferencesAnswer::default()`（`panel` 是唯一不同于类型默认的一个：没人关之前它开着）；不认的键即拒 | 文件能写的键与答案能说的字段是**同一份声明**，而不是一边一份的两张表。`lang` 缺席而不是填 `en`：没人选过之前，浏览器自己的语言标是唯一的证据，写死一种语言会在每一台从未打开过该设置的机器上盖掉它 |
| `Command::PutPreferences { patch, idem }` | `PreferencePatch` 闭集：`lang｜welcomed｜panel｜appearance｜proxying｜chord` | 一帧一件事，不是整表写回：两个屏幕各改一件，不得互相覆盖 |
| `Query::Config { addr }` → `ConfigAnswer` | `effort: Option<SettledEffort>` ＋ `second: SettledSecond` ＋ `TuningDefaults`；`SettledEffort` 携 `ConfigLayer`（`default｜city｜building｜resident`，后三个与 `city::Layer` 同拼写，`default` 是没有任何一级文件说过、城的内建值在生效）；`SettledSecond` 同携 `ConfigLayer`，`percent` 为已过 `SecondThreshold` 构造点的整百分数，没有一级说过时是 `kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`，`domain: SecondDomain { min, max }` 是 `SecondThreshold` 构造点的合法域（`CTX_REMINDER_SECOND_MIN`／`_MAX`）（§8-47）；`effort` 缺席仍是一句陈述：没有一级说过时回答的是提供方自己的缺省，城说不出那个值；`TuningDefaults` 为 `from: ConfigLayer`（今天恒为 `default`：梯上没有一级文件说得出这几个数，它们是 `gateway::EndpointTuning::DEFAULTS` 的读出；层随值一起答，页面才不必自己断定「这是内建的」）＋ `timeout_ms` ＋ `request_max_retries: Option<u32>`（`Retries::stated`，缺席即 `UntilHalted`）＋ `stream_idle_timeout_ms: Option<u64>`（缺席即与整通调用同界）＋ `proxying` | **层是答案的一半。** 只给解析值的页面说不出这是本层写的还是继承来的，于是要把三层再读一遍自己爬一次梯子——**一把梯子爬两次就是一个问题两个答案**。层名随 `city::Layer`：线上把楼的文件叫 `resident`、把房的文件叫 `room`，而梯子把房的文件叫 `resident`——读者与被治的那次 run 会对“这是哪一份文件”给出不同的答案；一处穷尽匹配（`accounting::views::lines::rung_of`）把两份拼写钉在一起。`TuningDefaults` 是 `gateway::EndpointTuning::DEFAULTS` 的读出，字段形状也随它：重试上限是 `Retries` 而不是一个数（“直到有人按停”没有数字拼得出来），流的那个界是**闲置界而非整答案的截止**，且可缺席 |
| `Command::PutShelved { shelf, name, text, idem }` | `Shelf { Library, Building(addr) }`；写 `shelved_document_written` | 与 `GovernedDocument` 同一条理由：两处货架都在保留子树里，任何写域都够不着，所以帧里没有路径可拼。`name` 允许子路径，脚本因此留得住自己的文件夹 |

**八、`DialectKind::OpenAiResponses`。** kernel 的兼容格式集由二变三，`gateway::dialect` 的五个入口各多一条臂。登记与调用从此说同一句话：人粘贴 responses URL，`ConnectionKind::Responses` 记住了，而 `wire()` 从前仍答 `OpenAi`——**记对了、调错了**。形状的出处归 gateway-SPEC §8-20。

### 8-40 会动作的两扇门先问配对：`decide_admission` 与一层 middleware

`decide_bind` 把「能从回环之外到达」换成配对令牌的要求，只在 `decide_handshake`（`/ws`）判令牌，**合法配置的暴露城就允许任何能到达端口的人写字节进金库路由、并驱动 `transcribe_sink` 花钱**。所以判定只有一个家。

```rust
// 三扇门共用的那一问，壳里零策略（同 decide_bind／decide_frame 的切法）。
decide_admission(Door::Transcribe | Door::Enroll | Door::Drop, offered, face)  // 未配对即 E_GATE_DENIED
decide_admission(Door::Acp,        offered, face)  // 未配对仍进，携 Pairing::Absent
```

三条口径：

- **`Door` 是枚举而不是路径字符串**。门烧进 `route_layer` 的状态里，路由表因此仍是全仓唯一拼出 `/transcribe`、`/enroll` 的地方；middleware 不回头读 `uri().path()`，否则路径就有了第二个家。
- **两扇会动作的门当场拒，`/acp` 不拒**。花钱与收凭证是动作，未配对者不该触发；而外来编辑器「只学到一位」是 `agent_protocols::admit` 的措辞权（`crates/agent_protocols/Spec.lean` §9），所以那扇门把 `Pairing` 传进去而不是自己写拒词。`/acp` 的令牌仍写在 body 的 `token` 键里（编辑器没有别的地方写），但**判定调的是同一个函数**——一个规则一个家，与令牌写在哪无关。
- **`/` 与 `/{*asset}` 保持开放**：人要先拿到页面，才有地方输入配对码。
- **令牌怎么递**：socket 写在 hello 帧里（帧类型给了它字段名），POST 没有帧，于是走标准的 `Authorization: Bearer`，`offered_pairing` 是这条拼写在服务端的唯一读者，`client/src/core/socket.ts` 的 `bearing()` 是浏览器侧唯一的写者。

- **`/enroll` 与保存凭据的那条路共用一个构造点**：realm 与 name 交给 `kernel::SecretRef::new` 建引用，路由不再手拼 `secret:<realm>/<name>`——手拼的文本本城可能解析不回来，而 201 会把它答出去，所以建不出来即 422。**201 正文引用的是 `secret_captured` 记录里那句 `ref`**：存进去的与答出去的原是同一个值，两处各拼一次今天相等只因没人归一，改一处就分岔。客户端同改：`enrol()` 用 201 正文里城说的那句引用，不再自己拼一份。


### 8-41 丢帧可见、区间补拉、暴露面凭证

三件事同一集落地，因为它们是同一条链上的三个断点：慢会话丢掉的记录没有人说、丢了之后也没有一句话能把那一段要回来、以及「暴露面必须有凭证」当时只是一句注释加一个 bool。

#### 一 丢掉的记录要说出来：`ServerFrame::Lagged { from, to }`

```rust
pub struct Lagged { pub from: Seq, pub to: Seq }   // 两端都含
pub enum ServerFrame { …, Lagged(Lagged) }
```

**两端都取自会话自己数得出的记录，不取自广播报的那个数。** `RecvError::Lagged(u64)` 只说跳过了多少条，一个端点也不给；一个只拿到跳过量的人无法把丢掉的那一段要回来。所以会话新增一份状态——**上一条已发的 `EventRecord.seq`**——`from` 是它之后的第一条；`to` 是**恢复后首条回退一位**，因为「这一段到哪结束」只有在下一条记录到达时才成为事实。同一个值同时装着「已发到哪」和「还欠不欠一段范围」，故它是 `Stream::{Even, Owed}` 而不是一个 bool 加一个 `Option<Seq>`。

**`Even` 里的 seq 断口同样是一段欠账。** 一条记录可能根本没进广播（§8-47：拼不出帧），这时订阅不报 `Lagged`，会话仍是 `Even`；已发过记录的 `Even(Some(last))` 遇到 `next > last + 1`，照样先发 `Lagged { last + 1, next - 1 }`。判定只有 `decide_lag` 一处；`Even(None)` 不算——会话的视图从欢迎开始，欢迎之前的记录是问题而不是欠账。

**四路语义不同，故四路分开陈述**（四路指一个会话的四个接收臂：自己的拒绝、事件、增量、日志）：

| 臂 | 缓冲 | 拉下时 | 原因 |
|---|---|---|---|
| 自己的拒绝 | unbounded mpsc | 不会发生 | 拒绝的速率是一个人犯错的速率，不是城的速率，且一条都不能丢 |
| 事件 | broadcast 1024 | 发 `Lagged { from, to }` | 记录在账本里，那一段可以要回来 |
| 增量 | broadcast 256 | 一言不发 | 增量不落账本、不可重放，为它报一个区间等于报一段不存在的记录；settled 文本随后作为记录到达 |
| 日志 | broadcast 深度见 `serving::journal` | 一言不发 | 日志是诊断不是历史，且它携的位置是账本的位置而不是自己的序号，所以任何区间都放不回读者原来的地方 |

**`Closed` 只有一种语义：会话结束。** 一个已关闭的 broadcast 接收端**立刻**返回 `Closed`，把它当空转处理的臂在 `select!` 里会变成热转。发端全没了就是城要走了，四条臂一律 `return`。

**「重连从账本补」只在事件一路成立**（`Delta` 的 doc 明写 never written to the Ledger, cannot be replayed，日志同理）；另两路的静默在表里各自有理由。

**还没被欢迎的会话不报区间。** 它没有给对端看过任何一条记录，它的视角从 welcome 开始——而欢迎之前发生的事，页面要的话是一个提问（`Query::History`），不是一帧。

**客户端的闭包臂先封掉。** `client/src/core/link.ts` 末尾原有一个 `return [link, { kind: "nothing" }]`：**忘记处理一帧因此是零编译错误的静默失败**，而新帧刚好要落在这个位置上。现在那条路径是一个参数类型为 `never` 的函数，穷尽时它的实参收敛成 `never`、漏一臂时保持原类型——于是「忘了处理」是编译错误而不是沉默。

#### 二 丢了之后要能要回来：`Query::HistoryRange { from, to, limit }`

`Query::History` 只有 `before: Option<Seq>` 的向后翻页，回答的是「这之前发生了什么」；区间两头都有，从尾巴走到区间的近端要为此付掉中间每一条记录的代价。故新增一条查询：**答面是 `HistoryRangeAnswer { from, to, records, next }`，不是复用 `HistoryAnswer`。**

- **两个端点回声**：提问没有游标可握（两个 seq 来自一帧，那一帧早已被丢掉），所以答必须说出自己切的是哪一段——否则同一页同时开着记录视图与补拉时，两条答无法分辨。
- **`next` 而不是 `more`**：服务端知道账本里这一段的下一条在哪，答一个游标就是把它已经知道的告诉你；客户端因此不做序号算术。
- **`HISTORY_MAX = 500` 由服务端钳，客户端要不到更多**；一次补拉是多页，`next` 说下一页从哪起。
- **走不到的那一步结束区间而不是指一个越界的游标**：一条读不回来的行，或者一条越过账本尾部的区间，都让 `next` 是 `None`。反过来会让客户端对着同一个 `from` 永远问下去。
- **`QUERY_NAMES` 从 33 增到 34**，schema 哈希随之变（33→34 名字表长了一项，故哈希无论如何都要动，版本因此同集进位）。旧页面在握手期被明确拒绝，这正是该机制存在的理由。
- **实现住 `sprawling::views::answering::history_range`**，与 `history` 并排、共用同一个 `LedgerIndex::reader`；本 crate 不读盘。

**客户端的补拉走的是「一次一页、答的游标推进」，不是「一次一个可替换的窗口」**：区间排成队列（两次丢失是两个区间），队首一页在飞，答回来了才前进或出队。可替换的窗口在「一页在飞时又来一次丢失」这一刻会丢掉中间那一段。补到的记录**像别的记录一样折进 belief**（`store.apply`），且**不使任何已答的问题变旧**：它们是旧记录而不是新闻，为它们把所有答案标旧等于把整座城重问一遍。视图要不要直接画这一段，是前端道的事（本波只有 socket 半边问它）。

**客户端的 socket 半边**因此是本层唯一会「问一句、等一句、再问」的地方：`client/src/core/socket.ts` 的 `askGap()` 与 `filled()` 两小段，判断仍全在 `link.ts`。

#### 三 暴露面必须有凭证：把凭证装进面里

```rust
pub enum BindFace {
    Loopback { token: Option<B3Hash> },   // 只从回环可达；配了令牌就照样要
    Exposed { token: B3Hash },            // 能从别处可达，且从不无凭证服务
}
pub fn decide_bind(addr: &SocketAddr, token: Option<B3Hash>) -> BindVerdict;
impl BindFace { pub fn token_digest(&self) -> Option<&B3Hash>; }
```

**强制点：`decide_bind` 是 `BindFace` 的唯一生产者，`bind` 是它的唯一调用者，`serve` 把 `Bound` 里的面经 `router(config, face)` 交给壳。** 壳（`ShellState.face`）此后是「这一面要求什么」的唯一读者：`decide_frame`、`decide_admission` 都拿 `&BindFace`，不再拿一个 `Option<&B3Hash>`。

- **以前是什么样**：`decide_bind` 收一个 `token_configured: bool`，`serve` 只 `match` 掉 `Refuse` 而把 `Serve(BindFace)` 丢掉；然后每一道门各自去读 `config.token_digest`。「暴露面必须有凭证」因此靠一句话与一个 bool 维持，而 `router()` 是 pub：第二个入口可以造出一个暴露着却不要求任何东西的壳。
- **现在是什么样**：`Exposed` 里**没有** `Option`——「暴露着却不要求任何东西」是一个类型上不存在的状态。这个不变量在测试里以四种格钉住（回环有无令牌、暴露有无令牌、以及 `BindingFace` 索要的摘要是不是判定它的那一个）。
- **令牌摘要是 `bind` 的入参**：它是配置说的话（谁配了令牌），面是绑定判定给出的判决。判决只在 `bind` 里产生一次，面随 `Bound` 走到壳里，故这不是同一个事实的两个家。
- **`decide_admission` 的那句注释同时兑现**：「没有配令牌的城只可能是回环城」由面的形状保证——它拿到的是一个不可能要求空的东西。

#### 四 `/enroll` 等待里的第四个静默臂

`/enroll` 等的是 `secret_captured`：这条记录在广播上，于是这个等待也可能被流拉下，而**被跳过的记录不会再发一次**。旧代码对 `Lagged` 一言不发、继续等，最后由超时给出一个 202，正文写着「它还没答复」——一句可能不真的话。现在等待的结局是一个枚举：`Settled`／`Overtaken`／`Ended`，被拉下时当场结束等待，202 的正文写成它真正的样子（「城的流走过了这次请求」），因为再等下去也等不到那条记录。

#### 五 WebTransport 的重开条件

**不因「需要第二种协议」回来。** 重开条件是**两个都要实测成立**：①这座城真的跑在回环之外；②队头阻塞实测存在（即一条大帧让同一条连接上的小帧延迟到人能察觉）。理由：`ws://` 只到回环、暴露面经终止器是已记录的部署判断（根 `Cargo.toml`，`connect` 不带 TLS），而换成 QUIC 会同时改掉握手、帧界与资产通路，代价落在每一次改 wire 时；收益只在②成立时出现。`Carrier` 这个抽象不在树里：本 crate 只有一个实现，抽出它就是穿透层。

### 8-42 关停范围在答案里是一个类型，不是一个地址串

**这一笔只改一处**：`CityAnswer.halted` 由 `Vec<String>` 改成 `Vec<HaltScope>`（`answer.rs`）。

```rust
pub struct CityAnswer { …, pub halted: Vec<HaltScope> }   // 原为 Vec<String>
```

**根缺陷是同一件事的两种拼法，而读到两种拼法的那个读者刚好把它们比错了。** 一个 scope 在这座城里有两处书写方式，这是 `kernel::event::scope` 记下的分工：**账本**持 `city`／`building:<addr>`／`workshop:<addr>`，因为每一份已写下的历史都是这样；而**命令帧**持 §8-40 的 `HaltScope` 那样的带标签形状。答案面此前把它降成账本的那个串，于是客户端拿到的是 `building:lab` 与裸 `lab` 两个东西，`views/building.svelte:115` 拿后者去比前者——**被停的楼永远读成未停，release 永远不出现**，而两侧各自诚实，没有任何测试会红。

**答案用命令面的类型，因为那是提问的词汇。** 一页宁可拿 `HaltScope` 去比 `HaltScope`，也不要为了知道看的是哪栋楼而把一个字符串拆开——拆开就是给文法安第二个家，而这正是这一笔在关的东西。转换只发生在服务端一处（`views::answering` 的 `named`），账本的书写方式一个字未动。

**客户端那侧的两处消费也归一处**：`core/scope.ts` 是两种拼法唯一的接缝（`scopeOf`／`sameScope`／`buildingIsShut`／`cityIsShut`），`city_halted` 记录折进 `halted` 时经它转一次；其余每个比较点都比较类型。

**与事件载荷不冲突**：`city_halted` 的载荷里 `scope` 是 `kernel::Scope`，经 `schemars(with = "String")` 在 schema 上呈现为字符串。答案面改用 `HaltScope` 与那条覆盖不冲突——两者是不同的帧，各写各的读者。

### 8-43 新的会话是一个动词，不是一个开关

```rust
pub enum Carry { Nothing, Handoff }   // Nothing 是第一个变体，即默认
Command::OpenSession { addr: Address, carry: Carry, from: Option<Origin>, idem: IdemKey }
// Origin { run: RunId, at_seq: Seq }  —— kernel::Origin，一个家
```

**`from` 把「分叉」收进了同一个动词**。从某句分出去与从此处重开是同一件事的两个起点：都是「在这个房间开新的一段」，只差新的一段要不要继承某条线的对话。所以线上没有第二个动词，没有 `Fork` 帧（它写下血统却没有任何 dispatch 路径消费它，`routing.rs` 的注释与 runtime §8-2 的 §186 早把这件事记成缺陷），`OpenSession { from: Some(..) }` 是它该在的地方。**血统仍然写在 `run_forked` 里**，由真正开始的那个 run 写：一个分支在「开」的时刻还没有 run，先写一条血统就得先编一个 run id，而那个 run 永远不会存在。

**它答的是一个死路。** 房间的第一个 run 把模型与强度冻进它自己的 `CONFIG.toml`（city-SPEC §8-14），此后形状不同的派活全被 `E_CONFIG_INVALID` 拒——这条规则本身是对的，前缀缓存不能中途换模型；错在被拒之后没有任何出口，换过主模型的人再也派不出去。新的一段会话就是那个出口，而它只能在城里发生（清掉房间自己写下的两行、清掉交接槽位、在账本写 `session_opened`），所以它是一个 Command 而不是页面自己做的几件事。

**`Carry` 是枚举而不是 `bool`。** 两个状态都是有名字的行为，而且落到磁盘上的结果不同：`Nothing` 连 `Handoff.md` 的槽位一起清空，`Handoff` 留着它。`carry: true` 在调用点读不出是哪一个，`Nothing` 也不是「没有值」而是一个答案——它是第一个变体，`Default` 因此不需要人再写一遍。

**本章测试**：`crates/wire/tests/wire_contract.rs` 的命令样本（表长、名表去重、golden 哈希）；`Carry` 的默认值在 `wire` 侧有一条断言，因为它是这份规格里唯一被写成「第一个变体」的默认。

### 8-44 删掉两个拼得出、执行不了的动词

`Command::Takeover` 与 `Command::Rollback` 删除；`COMMAND_NAMES` 从 30 到 28，schema 哈希随之变（golden 见 §2），故同集进位 35→36。

- **它们从来没有执行者**：装配层自上线起就对这两帧以 `not_built` 作答（§19 记的那次失效的三个动词之二），而客户端按 §19-2 的门要求不画它们。一条线上拼得出、任何东西都执行不了的命令，是对客户端的假承诺；删帧之后旧页面在握手期被明确拒绝（§8.5），而不是拿到一个永远失败的按钮。
- **规则的家在 kernel-SPEC §12.2**（回滚＝分支＋git 还原），含理由、被否方案与重开参数；本节只记线的形状，不复述第二份。
- **`control` 与 reach 表同集缩面**：`Intervention` 少两臂，只剩 Steer／Cancel／Halt 加 Release 返程（§8-4）；「中断一个活着的 Run 恒以 Handoff 收尾」的中断动词随之只剩 Steer／Cancel；§19-2 删两行。
- **事件词同集删二**（kernel-SPEC §8-4 表）：`rollback_applied`／`takeover_started` 无生产者，账本从未写下过携它们的行，故已写历史的字节与逐字节重放不受影响；携这两个词的行今天在读侧入口拒（`E_INVALID_ARGS`，kernel `parse_line` 的既有码）。
- **两帧的拒因不再是 `not_built`**：`not_built` 只留给仍在文法里、等待执行者的动词；对这两帧，字节在解码处就不再是命令（§8-37 的 `E_WIRE_MISMATCH` 口径不变）。

### 8-45 `ConfigureBuilding` 长出第二道阈值的面

`Command::ConfigureBuilding` 多一个可选字段 `context_second_threshold: Option<u64>`，`Query::Config` 的回答多一个 `second: Option<SettledSecond>`；schema 哈希随之变，故同集进位 36→37。

- **一个字段而不是一条新帧**：那条帧问的就是「这栋楼的 runs 按什么规矩来」——沙箱、外部服务器、桌面白名单与第二道阈值是同一个问题的四面；为它单立一帧会给同一个问题两个写入口。
- **线上传裸 `u64`，域的判定不在这条帧上**：合法域与拒因句式是 `kernel::config::SecondThreshold` 的一个构造点（30–90，拒因带域），在这条帧上再判一次就是同一个规则的第二个家；越界值在解析点拒，拒因随答复回到设置页。
- **回答带 `SettledSecond { percent, from }`**：`from` 是说出这个值的那一级文件，理由与 `SettledEffort` 同（`Query::Config` 回答表那一条）；缺省不是缺口，而是城一级默认值在生效，页面据此把一个空框画成默认值。
- **线的背面是同一件事**：写入经 `city::write_second_threshold` 落到那一级的 `CONFIG.toml` 的 `[context] second_threshold`，与 `write_effort` 同一扇门（读—改—写整份文件，别人的键原样保留）；`building_configured` 的载荷因此从三面到四面（`Written::context`）。
- **`WIRE_V` 的路不单独走**：36→37 记的是这一次面变——给既有命名帧加字段是「语法换形而名字没换」那一类（字段名不进 `COMMAND_NAMES`），与 §8-44 的 35→36 无关；两次都在 §8-1 的 golden 里看得见。

### 8-46 先占住端口，再交出城：`bind` 与 `serve` 分成两步

```rust
pub struct Bound { /* listener: tokio::net::TcpListener, face: BindFace —— 私有 */ }
/// 判定绑定面，再绑定监听器。判定拒绝时不碰网络。
pub async fn bind(addr: SocketAddr, token_digest: Option<B3Hash>) -> Result<Bound, AxError>;
/// 在已经绑定的监听器上服务，直到 future 被丢弃。
pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError>;
```

- **次序是装配层要的**：一座城先占住它的端口，然后才打开写者、写下第一行（sprawling-SPEC §8-88）。`ServeConfig` 里的 sink 要等写者线程开好才造得出来（转写 sink 借的是写者打开的那个金库），所以「绑定」必须能在 sink 存在之前单独做完。原先的单个 `serve(config)` 把两件事绑在一起，装配层只能先开写者、再在 `serve` 里发现端口已被占用。
- **`addr` 与 `token_digest` 离开 `ServeConfig`**，成为 `bind` 的两个入参。它们只被绑定判定读过；留在 `ServeConfig` 里，同一个地址就会有 `bind` 的入参和配置字段两个家。
- **失败码不变**：判定拒绝仍是 `decide_bind` 的 `E_CONFIG_INVALID`；操作系统拒绝绑定仍是 `E_CONFIG_INVALID`，recovery 仍是「换一个空闲端口，或者停掉占着它的进程」。
- **被否：先试绑一次再放掉，然后在 `serve` 里真绑**。试绑与真绑之间，别的进程可以把端口拿走，那样原来的缺陷只是窗口变窄了，并没有消失。
- **两者住 `server::listener`**，不住 `server::socket`：它们管的是监听器本身，先占、再服务；`server::socket` 管的是连上之后的会话、资产与上传。`Bound` 带 `#[must_use]`：占住端口而不服务，得到的是一个谁也不应答的端口。

### 8-47 一条记录只序列化一次：`Committed`

```rust
/// 一条已提交的记录，与它在线上的那一帧。克隆只加两个引用计数。
#[derive(Debug, Clone)]
pub struct Committed { /* record: Arc<EventRecord>, frame: Utf8Bytes */ }
impl Committed {
    /// 拼帧：`{"event":` ＋ 记录的 JSON ＋ `}`。
    /// # Errors  `E_WIRE_MISMATCH`：记录序列化不出来（恢复：该记录不推；下一条记录到达时 seq 断口使会话发出 `Lagged`，页面按它补拉，§8-41）。
    pub fn new(record: EventRecord) -> Result<Self, AxError>;
    pub fn record(&self) -> &EventRecord;
    pub(crate) fn frame(&self) -> Utf8Bytes;
}
```

- **seq 断口只能出自这一处**：广播的唯一写入者是 `sprawling` 的 folding 观察者，每条提交的记录都经它进这一个广播，一城一条全局 seq；所以 `Even(Some)` 时的 seq 断口只意味着某条记录没拼出帧，`decide_lag` 据此发 `Lagged`（§8-41）。在广播前过滤记录的任何改动都会让每个会话收到假 `Lagged`，须先改 `decide_lag`。
- **帧在广播之前拼好，socket 只写字节**。每个订阅者 `recv` 时广播要克隆一次载荷；载荷若是 `EventRecord`，16 个 socket 就是 16 次深拷贝加 16 次 `serde_json::to_string`。拼好的 `Utf8Bytes` 与 `Arc<EventRecord>` 让每个 socket 的代价降到两次引用计数和一次写。**被否**：socket 各自序列化——同一份字节算 N 遍，且 N 正是人开着的标签页数。
- **拼法的唯一权威是 `Committed::new`**，守护夹具断言它拼出的帧与 `serde_json::to_string(&ServerFrame::Event(..))` 逐字节相等：`ServerFrame` 是外部标签的 snake_case 枚举，`Event` 臂的形状恰是 `{"event":<记录>}`，两者一旦漂开页面读到的是另一条历史。
- **读数**：`instrument_fanout_cpu_per_event`（`--release --run-ignored only`）在 1／4／16 个订阅者上量每事件的 CPU，门槛是 16 个订阅者的每事件代价不超过 1 个订阅者的两倍——即序列化只算一次，不随标签页数增长。门槛取比值而不取绝对值，因为绝对值随机器档次变；现在的每事件代价由那一次 `EventRecord` 序列化主导（约 10 µs 量级），16 个订阅者时 ≤ 1 µs 是下一条的目标，不是现在成立的性质。
- 这份帧目前由 `EventRecord` 序列化而来；改为直接取 `append_all` 写下的账本行字节（使广播环只留一份字节，`Lagged` 补拉亦回账本字节）是本接口的下一步，前提是账本行与 `serde_json::to_vec(&EventRecord)` 逐字节相等——同一条守护夹具会判定它。

### 8-47b 一次工具调用带上它的起止时刻，一个回合带上它问的模型与它开始等人的时刻

```rust
pub struct Call {
    // …既有字段…
    pub called: TimeMs,            // tool_called 那条记录的 t
    pub answered: Option<TimeMs>,  // 配对上的 tool_result 那条记录的 t；未答为 None
}
pub struct Turn {
    // …既有字段…
    pub model: Option<String>,     // 开这个回合的 model_called 记下的 model
}
pub enum Note {
    // …
    Waiting { at: Seq, t: TimeMs, answered: Option<TimeMs> },
    // t：approval_requested 那条记录的 t；answered：按 approval id 配上的 approval_resolved 的 t
}
```

- **时刻读自账本记录，不读此处的时钟**：与 `Turn.t` 同理，重放的会话报它当初的时刻。`called` 不是 `Option`：一次调用由一条 `EventRecord` 折出，它总带读数。`answered` 与 `outcome` 同时写、同一次配对——`outcome` 为 `Waiting` 时它必为 `None`，窗口外答的调用也是 `None`，不猜。
- **为什么要上线**：run 页的时间透镜原本只能按回合着色，一个回合里模型说话与工具运行各占多久，线上没有数；有了这两个时刻，透镜画的是量出来的段，而不是按回合结局推断的整段。
- **被否：只带一个时长**。时长丢了起点，页面画不出调用在时间轴上的位置，也就排不出并发的两次调用。
- **模型名挂在回合上，不挂在答案上**：`model_called` 每问一次记一次 `model`，一次 run 中途换模型（降级、换端点）时，逐回合的名字才是账本写下的事实；run 页统计栏的「模型」格取最后一个回合的名字，前后不同时列出各个名字。读法与 `Call.subject` 同：取那一键的文本，读不出为 `None`，页面不画名字而不猜。
- **被否：`RoundsAnswer.model` 一个字段**。那得在折叠里挑一个回合的名字当整次 run 的名字，换过模型的 run 上它说错一半。
- **等人从哪一刻开始，读自请求记录**：`Note::Waiting.t` 是 `approval_requested` 那条记录的 `t`，与 `Call.called` 同理不是 `Option`。等到哪一刻结束是 `answered`：`approval_resolved` 记在城自己的 run 下，服务端按 approval id 把它配回请求（sprawling-SPEC §8-50-1），配不上为 `None`，不猜。

### 8-47c `RestoreDiscard`：回收站的一行按它自己的路回去

```rust
Command::RestoreDiscard { restoration: Restoration, idem: IdemKey }
```

- **帧里带的是那一行的 `restoration` 原样**，不是路径。`DiscardView` 的每一行已经带着它自己的回去的路（`Restoration`），页面把它交回来；城要是改成按路径去查，就得在写线程上为一次还原把整份历史再折一遍，而那一行本来就在页面手里。一个伪造的 `Tracked` 能做到的最多是把城自己历史里的某个文件写回城里它自己的路径——`Address` 爬不出城，`restore` 拒绝 `Address::is_reserved` 的地址，所以受保护的元数据子树（`.sprawling/`、`.git/`）不经这条路写入。
- **三种路，三个回答**：`Tracked(file:<addr>@<oid>)` 由装配层经 `storage::Checkpoint::restore` 写回，再追加 `discard_restored`（载荷与它关掉的那条 `file_discarded` 同形：`paths` 与 `restoration`），`DiscardView` 据此把那一行标成已还原；`Interred` 答 `E_INVALID_ARGS`，recovery 说从内容仓库取回尚未接线；`Rebuildable` 答 `E_INVALID_ARGS`，recovery 就是那条重建的理由——没有存着的字节可放回去。带 `range` 的定位符同样被拒：还原的是整个文件。
- **被否：`RestoreDiscard { path }`**。见第一条；另外，同一路径可以被丢两次，只给路径说不清要回到哪一次。

### 8-47d 问与答按 `ask_id` 配对，答带 `as_of`

```rust
pub struct AskId(pub u32);                      // 形状 2；页面按连接铸造，服务端只回显、不判定
pub struct Ask { pub ask_id: AskId, pub query: Query }
ClientFrame::Ask(Ask)
pub struct Answered { pub ask_id: AskId, pub as_of: Seq, pub outcome: AskOutcome }
pub enum AskOutcome { Answer(Answer), Refusal(AxError) }
ServerFrame::Answered(Box<Answered>)            // 对一问的拒绝也走这里
```

**一个答复回到哪一问，由问的一方铸造的编号决定，而不是从答复的内容反推。** 从内容反推需要一张「答复字段 → 问题键」的表，那是「问什么」的第二个权威：`Query` 每加一条，表就得跟着加一行，漏一行的后果是一个永远不落地的答；两个同种、参数不同的问题同时在途时，内容也分不出它们。

- **配对的键由问的一方铸造**：`AskId` 在页面上按连接递增（到 `u32` 上限回到 1），服务端原样回显，不检查唯一——配对是页面的事，服务端再判一次就是第二个家。重连后页面清空在途表，旧连接的 id 不会再来。区间补拉（§8-41）与视图的问共用这一个计数器，两者的 id 不会相撞。
- **拒绝也带 `ask_id`**：`AskOutcome::Refusal` 让对一问的拒绝落到那一问上，那一问回到陈旧状态，由下一个观看者再问；拒绝本身仍交给页面的拒绝角落。拒绝码是 `E_WIRE_MISMATCH` 时整条连接停下，与 `ServerFrame::Refusal` 同一规则。命令的拒绝仍走 `ServerFrame::Refusal`。
- **`as_of` 是答复尚未反映的第一个 `seq`**：`sprawling` 在视图锁内先取视图下一条待折叠记录的 `seq`，再读答复，二者在同一把锁下，所以答复恰好反映 `seq < as_of` 的全部记录、不含其余。什么都没折叠的视图答 `Seq::FIRST`：`Seq::FIRST` 是创世那一行的编号，若 `as_of` 取「最后折叠的一条」，「什么都没折叠」与「已折叠创世」就拼成同一个值，创世之前问出的答复会把随后到来的创世当成已含，从此不再重问。视图锁中毒时 `as_of` 为 `Seq::FIRST`，结果是拒绝。页面据此判陈旧：`seq < as_of` 的事件已在答复里，不再触发重问；问在途时到达的事件记下它的 `seq`，答复的 `as_of` 大于它时，这次陈旧随答复一起消掉。
- **帧名随之换了**（`query`→`ask`，`answer`→`answered`），`WIRE_V` 加一；旧页面在握手处被拒，而不是发出服务端读不懂的帧。
- **`sprawling console` 只有一问在途**，所以它的问一律用 `AskId(0)`，并且不读回 id。

### 8-47e 缺省也是一层

`ConfigLayer` 多一个 `Default`（线上拼 `default`）；`ConfigAnswer::second` 从 `Option<SettledSecond>` 变成 `SettledSecond`：没有一级文件说过时，它是 `CTX_REMINDER_SECOND_DEFAULT` 且 `from = default`。字段换了形而名字没换，故同集进位 38→39。

- **缺席不是一个值**：`None` 让页面自己补上缺省，于是客户端带着第二份 `65`，和 kernel 的那一份之间没有任何东西把它们拴住。回答里写明生效值和它的来源，页面画出城说的数，只有一份。
- **`from = default` 是页面判断「这一级有没有写过」的依据**：`default` 时框留空、把生效值画成提示；否则框里是那一级写下的数。
- **合法域随值一起回答**：`SettledSecond::domain` 是 `SecondThreshold` 构造点读的那两个常量。页面仍不判域（拒因由城带回），但它画给人看的「30 到 90」不再是客户端自己写的第二份。
- **`effort` 不跟着变**：没有一级说过时生效的是提供方的缺省，城不知道那个值；为它编一个级别就是说一句城说不出的话。重开条件：城自己开始为 effort 定一个缺省值。
- **被否的方案**：保留 `Option` 并在旁边加一个 `default_percent` 字段——那样一个值有两个字段，读者要自己拼出生效值，拼法又多一个家。

### 8-47f 查询按它回答的东西命名：`Query::NewestRelease`

```rust
pub enum Query { /* … */ NewestRelease }   // 线上拼作 "newest_release"
```

- **名字说出答的是什么**：这条查询回答「这座城是哪一版、npm 上最新的是哪一版」（§8-36 的五条口径不变）。它原来叫 `Release`，与释放一个关停范围的 `Command::Release` 同名；控制台把 socket 能带的动词与查询列在一处，一个人看到两个 `release`，分不清哪个放开关停、哪个去问注册表。
- **只改查询，不改答复**：`Answer::Release` 与 `ReleaseAnswer` 保持原名。答复不出现在人能输入的地方，没有同名的歧义；改它只会多一次无人受益的线上换形。
- **`WIRE_V` 不动**：变体名进 `QUERY_NAMES`，名字一变 §8-1 的 golden 就变，旧页面在握手时被拒，而不是发出一条城不认识的查询；`WIRE_V` 只为「名字没换而语法换形」而升（§298），改名不属于那一类。新名字登记在 `docs/glossary.md` §6；旧拼法不进 `tools/xtask/lexicon.toml`，因为已发布版本的 `CHANGELOG.md` 如实记着它当时的名字，子串禁令会误伤那段历史。
- **被否：保留 `release` 作别名**。一条查询两个拼法，就是同一个名字有两个家；握手已经把旧页面挡在门外，别名没有读者。

### 8-47g 性能监视器的一对帧（`wire::frames::monitor`，形状：值类型）

监视页和 `sprawling top` 读同一份历史（sprawling-SPEC.md 8-94），它们从线上拿到它。线协议为此加一种帧，两个方向各一个变体：

- `ClientFrame::Monitor(Monitoring)`：`Monitoring` 是 `Watch`、`WatchSummary` 或 `Release`。`Watch` 让这个会话算作一个看整页的人，`WatchSummary` 让它算作一个只看事实条摘要的人，`Release` 让它不再算；会话结束等于 `Release`。
- `ServerFrame::Monitor(Sample)`：一次读数，只发给正在看的会话。`Sample` 的字段即 sprawling-SPEC.md 8-94 列出的 13 个 `u64`，加上 §8-64 的 `view_backlog` 与 `read_nanos`；它定义在 `wire::frames::monitor`，`bin::monitor` 用的就是这一个类型，不再另写一份。
- `Watched`：看的人看什么，`Everything`（整页）或 `Summary`（事实条上的摘要）。它不上线，是 `MonitorFeed::watch` 的参数，`bin::monitor::Monitor` 按它分两类计数。
- `decide_frame` 把一个已打开会话的 `Monitor(Watch)` 答成 `SessionStep::Watch(Watched::Everything)`，`Monitor(WatchSummary)` 答成 `SessionStep::Watch(Watched::Summary)`，`Monitor(Release)` 答成 `SessionStep::Release`；未打开的会话照旧拒绝并关闭。外壳收到 `Watch(watched)` 时调用 `ServeConfig::monitor` 的 `watch(watched)` 拿一个看的凭据，已有凭据时换掉它、保留已有的 `samples` 订阅，没有时订阅 `samples`；收到 `Release` 时把两者都丢掉。重复的 `Watch` 不叠加计数：一个会话至多持有一个凭据。

**决定。**

1. 看与不看是会话里的两个帧，而不是一个 `Query`。`Query` 问一次答一次，而监视是一段持续的订阅：它的结束（`Release` 或断开）必须让城停止采样，这件事只有持有会话的外壳能保证。另一种做法是每秒一个 `Query`，它让每个看的人每秒多一次往返，且城无法知道人已经走了。
2. `watch` 是一个返回不透明凭据的函数，而不是把计数器交给本 crate。有没有人在看由 `bin::monitor::Monitor` 一处决定；外壳只持有凭据，丢掉它就是不看。
3. 读数经 `broadcast` 发出，与 `deltas`、`logs` 同形：错过的一次读数不必补，下一秒还有一个。
4. 摘要是第三个变体，而不是 `Watch` 带一个参数。事实条在每个页面上，所以它的看法必须比整页便宜得多：只看摘要时城只读本进程（sprawling-SPEC.md 8-96 的 `OwnProcess`，约 1 µs），不打开整机与卷的计数器。已有的 `"watch"` 拼写保持原义，新的 `"watch_summary"` 让 `WIRE_V` 加一。另一种做法是让 `Watch` 带上 `Watched`，它改掉已有帧的拼写，而得到的东西相同。

### 8-48 一次 run 的开头带上由谁派来

```rust
pub struct Opening {
    // …既有字段…
    pub dispatched_by: Option<Who>,  // run_started 的 dispatched_by；缺键为 None
}
```

- **派活者写在 `run_started` 的载荷里，不读那条记录的作者**：`run_started` 的作者恒为 `city`——是城的派活台写下这一行——所以作者说不出这次 run 是人派的、城按日程与计划派的，还是一个居民委派、接替或敲门派的。派活处各自知道答案：人下的 `Dispatch` 写 `person`；计划节点、日程、外来到达与人刚放行的活写 `city`；委派写委派者的地址，接替写前任的地址，敲门叫醒写敲门者的地址。`runtime::RunPlan.dispatched_by` 把它从派活处带到 `run_started`，线上的 `Opening` 原样转述。
- **旧账本里没有这个键**，读作 `None`，页面不画「由谁派来」而不猜。
- **被否：从 `parent`／`predecessor` 推断**。那两个键只说明委派与接替，人派的与城派的在账本里长得一样，推断在最常见的两种派活上答不出来。

### 8-48b 一行 run 带上它的结局、它开的 PR 与它等的事

```rust
pub struct RunSummary {
    // …既有字段…
    pub completion: Option<String>, // run_frozen.completion；未冻结或窗口外为 None
    pub pr: Option<String>,         // 最近一条 pr_opened 的 branch（城里的 PR 以分支为名）
    pub ask: Option<String>,        // 最后一条记录是 approval_requested 时，它的 action_desc
}
```

- **为什么要上线**：只看结果的城把冻结的 run 分成做完、失败与已结束，并在一行末尾写出 PR 或等你做的事。页面重载后它只有 `city_view` 的答，没有这三件，每个冻结 run 都落进「已结束」，等你的 run 只知道在等、不知道等什么。
- **三件都由 `storage::RunHot` 折出**（storage-SPEC §8-5），`summarize` 照抄，不读账本。
- **`ask` 只在 `last_kind` 为 `approval_requested` 时有值**：页面判断等待用的是 `last_kind`，`ask` 跟着同一条记录走，二者不会一个说在等、一个说不等。
- **`pr` 是分支名而不是数字**：城里的 PR 是 `collab::OpenRequest`，它的身份是分支（`node` 由分支解析而来），账本里没有别的编号；编一个序号就是给人一个线外查不到的名字。
- **被否：把 `Rounds.closing` 让城逐行去问**。那是每行一次查询，一座两百个 run 的城首屏要问两百次，而这三件热视图本来就在折。

### 8-48c `Dispatch` 可以点名这一次的模型

```rust
Dispatch { addr, task, goal, policy, idem, session, effort, model: Option<String> }
```

- **`model` 是城已登记的一个模型 id**，登记在哪个 tag 下都行；`None` 取房间自己那层已冻结的模型，房间尚未冻结时取 `main` tag 的模型。装配按这个 id 在簿子里找到那一条登记，连同它的端点与窗口一起用，保密楼「只用回环端点」的检查照旧由 `ModelBook::select` 做（sprawling-SPEC §8-10）。
- **被否：借 `SelectModel` 换 tag 再派活**。`SelectModel` 改的是整座城的配置，会在同时跑着的别人的 run 底下换模型；一次派活的选择只该属于这一次派活。
- **被否：接受任意 id，在 `main` 的端点上直接调用**。窗口与输出上限是登记时说出的，未登记的 id 没有这两个数，上下文提醒只能量一个没人给过的数。
- **`WIRE_V` 38→39**：给既有命名帧加字段是「语法换形而名字没换」那一类，§8-1 的 golden 随之变；`client/src/wire.ts` 由 `cargo xtask wire-ts --write` 同集重生成。

### 8-48d `ServerFrame::Output`，一条还在跑的命令写出的字节

```rust
pub enum ServerFrame { …, Delta(Delta), Log(LogLine), Lagged(Lagged), Output(LiveOutput) }
pub enum OutputStream { Out, Err }        // 恒不是 bool
pub struct LiveOutput { pub run: RunId, pub stream: OutputStream, pub text: String }
```

- **与 `Delta` 同一条规则**：可丢弃，不带账本序号，不进账本；调用的结果以 `tool_returned` 落账时页面扔掉它画的这段，两者不一致时账本赢。
- **第四条广播通道 `ServeConfig::outputs`**：一条刷屏的命令不该把模型的增量或日志行挤出慢读者的窗口；`RecvError::Lagged` 一言不发地略过，理由与增量相同——漏掉的字节在调用落账时整段到达。
- **`text` 是 UTF-8 有损解码**，因为一块在字节上界处切开，可能切在一个多字节字符中间；切口处画成替换字符只影响预览，结果以账本为准。**被否**：线上带字节数组——JSON 里一个字节要三四个字符，而页面最终画的是文本。
- **`ServeConfig::outputs_so_far: Arc<dyn Fn() -> Vec<LiveOutput> + Send + Sync>`**：一个会话在送出 `Welcome` 之后、接实时帧之前调它一次，把还在跑的命令已经写出的字节按原来的次序作为 `Output` 帧送出，所以在命令跑到一半时打开页面的人先看到已经写出的部分。会话先订阅第四条通道再调它，所以一块可能送两遍而不会漏；预览里重复一块无害，漏一块则要等调用落账才补上。缓冲住在装配层（sprawling-SPEC §8-90），因为清空它要看账本里的 `tool_result`，而本 crate 不折叠账本。
- **`OutputStream` 是 `runtime::Stream` 的第二处拼写而不是第二处权威**：本 crate 的依赖图够不到 `runtime`，映射住在装配层（`sprawling::assembly` 的 `serve`），与 `LogLevel`（§8-32）同一个安排。

### 8-48e 一行 run 带上人交给它的任务与目标

```rust
pub struct RunSummary {
    // …既有字段…
    pub task: Option<String>,       // run_started.task；空串或窗口外为 None
    pub goal: Option<String>,       // run_started.goal；同上
}
```

- **为什么要上线**：run 板以人说的第一句话给一行 run 起名，没有这句话就以目标起名。页面只从事件流里听到 `run_started` 时才知道这两句；重载之后它只有 `city_view` 的答，每一行都只能叫「某房间里的一次 run」，同一个房间里的几次 run 在板上没法分开。
- **两件都由 `storage::RunHot` 从 `run_started` 折出**（storage-SPEC §8-5），`summarize` 照抄，不读账本。空串记作 `None`：一句空的任务不是一个名字，页面对 `None` 与空串本来就得同样处理，线上只留一种拼法。
- **被否：让页面逐行问 `Rounds` 拿 `opening`**。与 §8-48b 否掉逐行问 `closing` 同一个理由：一座两百个 run 的城首屏要问两百次，而热视图本来就在折 `run_started`。
- **`WIRE_V` 42→43**：给既有答面类型加字段是「语法换形而名字没换」那一类，golden 随之变；`client/src/wire.ts` 由 `cargo xtask wire-ts --write` 同集重生成。

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
- 城怎么存、存在哪里、答出哪条路径是 sprawling-SPEC 8-119 的事；这里只把名字与字节交进去，把答案或拒绝原样交回来（拒绝是 422 加 `refusal_text`）。

### 8-50 依赖页的三个版本与一包：`DoctorItem.pinned`／`pack`、`Query::UpstreamVersion`

```rust
pub struct DoctorItem { …, pub pinned: Option<String>, pub pack: Option<DoctorPack> }
pub enum DoctorPack { RustTools }
// Query 追加在声明序末尾
UpstreamVersion { item: String },          // → Answer::Upstream(Box<DoctorUpstream>)
pub struct DoctorUpstream { pub item: String, pub newest: DoctorNewest }
pub enum DoctorNewest { Asking, Read { version: String }, Unread { why: DoctorUnread }, Refused { said: String } }
pub enum DoctorUnread { WithToolchain, ManyBrands, MatchesBrowser, ThisProject, NoSource, UnknownItem }
```

- **每项一问，而不是一份答案里的一个字段**：上游版本来自六个不同的站点，一个慢的站点不能拖住整页；页面对每一项各问一次，答一个填一个。塞进 `DoctorAnswer` 就得等最慢的那一个，或者要第二条推送通道。
- **`Asking` 让问题不等网络**：一个会话的问题按到达的次序一个一个答，一个要出网几秒的问题会挡住它后面的每一个；城先答 `Asking`，在后台去读，页面过一会儿再问。
- **`pack` 是一个枚举而不是一个字符串**：页面要给这一包起名字、写说明，所以它必须是页面认得的封闭集合；新的一包是每个读者处的编译错误。
- **`DoctorUnread` 是封闭的原因**：「读不到」有几种，每一种页面各有一句话，线上不带句子（§8-25 同一条理由）。`Refused.said` 带的是网络在哪一步停下，那是平台自己的话。
- **`pinned` 是版本号本身**，已从仓库的文件里读好；没有钉子的项为 `None`。
- 城那一侧从哪里读、怎么记住读数，见 sprawling-SPEC §8-120。

### 8-51 设置页的厂商表：`Query::KnownHosts`

```rust
Query::KnownHosts                                   // 无参数；答案不随城变
Answer::KnownHosts(KnownHostsAnswer)
pub struct KnownHostsAnswer { pub hosts: Vec<KnownHost> }
pub struct KnownHost { pub host: String, pub faces: Vec<KnownFace> }
pub struct KnownFace { pub dialect: DialectKind, pub base_url: String }
```

- **一个人挑厂商，而不是去厂商文档里复制一个地址。** 本城认得的 host 住 `gateway::provider::preset`（gateway-SPEC §8-17），设置页经这一问读它：每个 host 说几面、每面的 base URL 是什么。`base_url` 是这座城登记时自己会算出的那个地址（`normalise_entered`），所以页上填进框里的与登记下来的是同一串。
- **客户端据同一答案决定哪几面可选**：一个 host 不说的那一面在控件上拒点，理由写出它说的几面。客户端不再持自己的 host 表。
- **一问而不是塞进 `EndpointsAnswer`**：那个答案说的是这座城登记了什么，随账本变；这一问说的是本城认得哪些厂商，只随二进制变。合成一个答案，会让每一次登记都重发一份不变的表。
- 名字表多一项，schema 哈希因此而变，`WIRE_V` 不为此进位。

### 8-52 设置页的 harness 页：`Query::Harnesses`

```rust
Query::Harnesses
Answer::Harnesses(HarnessesAnswer)
pub struct HarnessesAnswer { pub harnesses: Vec<HarnessLine> }
pub struct HarnessLine { pub name: String, pub launch: Vec<String>, pub found: bool, pub docs: String }
```

- **provider 页与 harness 页分开**（定规）：provider 页收 API key，harness 页说明五家官方 harness（`crates/agent_protocols/Spec.lean` §8-19）。
- `name` 是 `agent_protocols::Harness::as_str` 的词；`launch` 是起它说 ACP 的那条命令，逐词；`found` 是那条命令的程序在这台电脑的搜索路径上找不找得到；`docs` 是这家自己写的登录说明。**登录是人在 harness 里做的**，这一问不答任何凭据的事。
- 名字表多一项，schema 哈希因此而变，`WIRE_V` 不为此进位。

### 8-53 一个回合带出首个内容几时到，每个时刻带出它是不是量出来的；检查点的 note 叫 `checkpointed`

```rust
pub struct Turn {
    // …既有字段…
    pub first_at: Option<TimeMs>,   // 开这个回合的回复记下的 first_at（kernel-SPEC §8-75）；缺席即没量到
    pub timing: Timing,             // `t` 是不是 model_called 自己那一刻
}
pub struct Call {
    // …既有字段…
    pub timing: Timing,             // called 与 answered（在场时）是不是各自那一刻
}
#[serde(rename_all = "snake_case")]
pub enum Timing { Measured, Unmeasured }
pub enum Note {
    // …
    Checkpointed { oid: GitOid, at: Seq },   // 线上 "checkpointed"
}
```

- **`first_at` 照录那一行的键。** 回合里最后一条 `model_returned` 写下的 `first_at`，读不出或缺席即 `None`。首字耗时是 `first_at − t`，线上不另带一个时长：页面手里已有这两个数。
- **`Timing` 答一个问题：两个时刻之差是不是一次测量。** `Measured`：这一行上的每个时刻都是它自己那条记录量下的那一刻（`EventRecord::moment` 答 `Some`）。`Unmeasured`：至少一个不是——账本版本 1 写下的行带的是回合时间戳，同一回合的行同值；或者答复是重启之后城补上的 `E_TOOL_OUTCOME_UNKNOWN`，它记的是城补上它的那一刻（kernel-SPEC §8-4「信封 `t` 记的是什么」）。时刻本身照旧带出，它仍给出次序；页面不从 `Unmeasured` 的行画用时。
- **`Turn.timing` 只说 `t`**：回合在线上只有这一个时刻；`first_at` 在场即量过，缺席即没有。`Call.timing` 说 `called` 与 `answered` 两个：一次调用的两条记录由同一个构建写下时两者同为量过或同为未量，城补上的答复例外，所以一个值够用。
- **`Checkpointed`**：fence 与 checkpoint 曾是一个概念的两个名字，checkpoint 留下（glossary）。`Note` 不进名字表，改它的标签不动 schema 哈希，所以它随本节的进位落地。
- **进位**：本节的提交是 §12.1 意义上上一次推送之后第一个名字不变而改形的提交，`WIRE_V` 44 → 45；§8-53 至 §8-58 共用 45。

### 8-54 一次提交带出同一次 run 的上一个提交，与它在 git 里的父提交

```rust
pub struct CommitAnswer {
    // …既有字段…
    pub previous: Option<CommitAt>,     // 同一次 run 在它之前宣告的最后一个提交；这是那次 run 的第一个时为 None
    pub parents: Option<Vec<GitOid>>,   // 提交对象自己记的父提交，按 git 的次序；读不到时为 None
}
pub struct CommitAt { pub oid: GitOid, pub seq: Seq }
```

- **`previous` 由账本折出。** 两条宣告提交的记录（`checkpoint_committed` 的提交一支与 `pr_merged`）按 `seq` 折进视图时，同一次 run 上一次宣告的那个提交就是它的 `previous`（sprawling-SPEC 8-128）。它与本提交围出一段：`Query::Changes { base: previous.oid, head: Some(oid) }` 答这次提交相对上一个检查点改了哪些文件，`Query::RunHistory { run, before: Some(seq) }` 往回读到 `previous.seq` 为止，答这一段里这次 run 发出的调用。这一段是候选，不是原因：同一栋楼里别的 run 与人也可能在这一段里写过文件。
- **`parents` 读自 git，在答问时读。** 提交对象自己记着它的父提交，账本记下的 oid 就是这个对象（连同父提交）的哈希，所以这里读的是权威本身，不是投影；五条 trailer 才是投影，本节不读它们。`Some(vec![])` 是根提交；`None` 是这座城没有仓库、仓库里没有这个对象，或者读失败——一座导出后在别处恢复、身边没有 `.git` 的城，其余各字段照答，只是画不出这一格。读 git 在快照的锁放开之后做（sprawling-SPEC 8-100），一页提交只开一次仓库。
- **提交说明另成一项**，仍在 §4 的清单里。

### 8-55 一次调用带出它的效果与呈现

```rust
pub struct Call {
    // …既有字段…
    pub effect: Option<kernel::Effect>,        // tool_called 记下的登记（kernel-SPEC §8-75(b)）
    pub render: Option<kernel::RenderIntent>,  // 同上：Generic、Terminal 或 Diff
}
```

- **照录 `tool_called` 的两个键**，读不出或缺席即 `None`：这件工具没登记，或这一行写在这两个键出现之前。页面据 `render` 选画法（终端、差异、通用），据 `effect` 说这次调用越过了哪一种边界；两者都是 `None` 时按通用画。
- **携 kernel 的类型本身**，不在线上另立枚举（§8-0）：`Effect::Write` 带它的写域地址，`Connector` 带服务器的标签，都是登记写下的事实。
- **`Diff.locations` 今天恒为空**：账上记的是登记层面的声明；一次编辑调用改的是哪个文件，读 `subject`。

### 8-56 被裁掉的输出指向它的原文

```rust
pub struct Output {
    // …既有字段…
    pub pinned: Option<Locator>,   // 这次调用的输出离窗时，原文存在哪；未离窗、旧行或读不出时为 None
}
```

- **只在调用的输出上有值**：`Call.output` 的 `pinned` 读自配对上的 `tool_result` 结果里第一笔离窗账目（`runtime::pipeline::pinned_original`，runtime-SPEC §8-51(b)）；`Call.arguments` 也是 `Output`，它的 `pinned` 恒为 `None`——参数从不离窗。
- **原文是命令原本写出的字节**：结果先被 sieve 裁、再被普通搬运存一次时，指的是第一笔账目的原文，不是 sieve 留下的替身。页面拿它经 `Query::Content` 读全文，`cut` 仍只说这个视图裁了几行。
- **旧行明确缺席**：结果里没有账目（没离窗、或写在账目进结果之前）即 `None`，不按相邻的行去猜。

### 8-57 派活带出整份运行策略：`Dispatch.policy`

```rust
Dispatch { addr, task, goal, policy: kernel::RunPolicy, idem, session, effort, model }
// RunPolicy { mode: Mode, write: WriteLimit, admit: AdmissionRequirement, landing: LandingPolicy }
// 线上：{"mode":"work","write":"create","admit":"tested","landing":"experiment"}
```

- **一个字段，四个必填的键**：`mode`（`chat｜work`）、`write`（`full｜create`）、`admit`（`standing｜tested｜contract_kept｜double_validated`）、`landing`（`ordinary｜experiment`）。值集与拼法只住 kernel（kernel-SPEC §8-77、§8-78），本 crate 再导出 `RunPolicy`、`Mode`、`WriteLimit`、`AdmissionRequirement`、`LandingPolicy` 五个名字，页面按 `wire.ts` 里生成的字面量拼。缺一个键、或一个认不出的词，帧在反序列化处即拒，不落成默认值。
- **页面怎么选**：写域选择给 `write` 的两项（普通 `full`、只读可新建 `create`）；`/admit tested|contract|double` 依次填 `tested`、`contract_kept`、`double_validated`，不说时填 `standing`；试验填 `landing: "experiment"`；计划经 `/plan` 进入固定的 SDD 工作流，帧上是 `mode: "work"`。这些控件归客户端的展开面（client-SPEC 4-41），本节只定帧。
- **城自己派的活不经这个帧**：计划、日程、来信与编辑器经 ACP 派来的活由装配层取 `RunPolicy::of(Mode::Work)`；委派与敲门继承说话那一方的整份策略（sprawling-SPEC 8-133）。
- **账上读得到**：装配层把收到的策略原样写进这次 run 的 `run_started.policy`，所以一次派活选了什么，回放与 playback 从账本读，不从线上猜。
- **`WIRE_V` 不另进位**：`Dispatch` 的名字没变而形状变了，这正是 §12.1 说的改形，与本批其余改形共用 45。

### 8-59 身份的线面：读名字，带基线写，读回当时冻下的那一版

```rust
// Command
PutDocument { which: GovernedDocument, base: String, body: String, idem: IdemKey }
PutIdentity { card: IdentityCard, base: String, idem: IdemKey }
pub enum IdentityCard {
    Person { user_id: Option<String>, imported_from: Option<String>, about: Option<String> }, // 写 PREFERENCES.md
    Mayor { name: Option<String> },                                                         // 写 MAYOR.md
}
// Query
Identity,                                  // → Answer::Identity(Box<IdentityAnswer>)
pub enum IdentityAnswer {
    Stated(StatedIdentity),
    Unreadable { document: GovernedDocument, line: u32, why: String },
}
pub struct StatedIdentity {
    pub user_id: Option<String>,           // 城怎么称呼这个人；缺席时页面用语言表里的「你」
    pub imported_from: Option<String>,     // user_id 是从哪台主机的 gh 导入的；手填为 None
    pub about: String,                     // PREFERENCES.md 身份区之后的正文
    pub mayor: Option<String>,             // 主 Agent 的显示名；缺席时页面用语言表里的默认名
    pub version: B3Hash,                   // 新 session 会冻下的那一版（city-SPEC §8-33）
    pub preferences_text: String,          // 两份文件此刻的全文：保存时的 base
    pub mayor_text: String,
}
```

- **两份文件，一个身份区。** 用户 ID、导入来源与「关于你」住 `PREFERENCES.md`，主 Agent 的名字住 `MAYOR.md`；身份区的语法、名字的合法域与「一个 session 冻下哪一版」都由 `city::Naming` 一处回答（city-SPEC §8-33），本 crate 只携字符串，不判它们合法与否（§8-0 同一条理由）。
- **卡片只写自己那几个键，其余原样。** `PutIdentity` 把卡上的值交给城，由城在 `base` 上改写身份区：卡上的键写入或删去（`None` 即删去，回到默认称呼），身份区里别的键、`about` 为 `None` 时的正文，都按 `base` 里的字节留着。页面不拼 TOML：拼身份区的只有城一处，表单与原文编辑器读到的是同一份解析结果（§12.5）。
- **两个写者，一道守卫。** 原文编辑器发 `PutDocument`，卡片发 `PutIdentity`，两者都携 `base`——发信方起手时那份全文——文件已经变了就拒 `E_VERSION_CONFLICT`，什么都不写；人的草稿留在页面上，页面重读之后再发。`MAYOR.md` 与 `PREFERENCES.md` 的身份区读不出时（重复的键、没有闭合的 `+++`、名字为空或带控制字符），`PutDocument` 在落盘之前拒 `E_CONFIG_INVALID`，拒因里有行号；`CLERK.md` 没有身份区，只经基线守卫。
- **回执是账本行。** 两条命令都写一行 `governed_document_written`，`naming` 键是写完之后城的身份版本（kernel-SPEC §8-79）；页面发出之后只显示「保存中」，见到这一行才显示「已保存」，并以它判断自己读到的 `version` 是否已经过时。
- **读的是此刻，跑的是冻下的那一版。** `Query::Identity` 每次从盘上读，所以页面显示的总是新 session 将要冻下的名字。已经开始的 session 用它第一次 run 冻下的版本（accounting-SPEC §8-15）；那一版记在每次 run 的 `run_started.naming` 上，页面拿它经 `Query::Content { locator: cas:<naming> }` 读回当时的名字。旧账本没有这个键，页面就显示地址或语言表里的角色名，不拿今天的名字冒充当时的。
- **读不出就说在哪一行。** 身份区读不出时答 `Unreadable`：哪一份文件、第几行（从文件第一行数起）、为什么。页面据此打开原文编辑器，而不是画一个默认名字再让下一次保存把人的正文盖掉。
- **`WIRE_V` 不另进位**：`PutDocument` 加 `base` 是名字不变的改形，与本批其余改形共用 45（§12.1）；`PutIdentity`、`Identity` 是新名字，哈希自己会变。
- 验收：city 的 `a_stale_identity_save_is_refused_and_the_draft_survives`（基线过期被拒、文件不变）；accounting 的 `a_new_session_freezes_the_name_the_page_shows`（实际发出的请求上下文与 `Query::Identity` 的答面同名同版本，旧 session 不改名，`/new` 之后两边一起换）。

### 8-67 从 GitHub CLI 读一个候选用户 ID：`Query::GithubLogin`

```rust
// Query
GithubLogin(Option<String>),                 // 问哪台主机 → Answer::GithubLogin(GithubLoginAnswer)；None 即 github.com
pub struct GithubLoginAnswer {
    pub host: String,                        // 问的是哪台主机：页面显示它，保存时写进 imported_from
    pub reading: GithubReading,
}
pub enum GithubReading {
    Found { login: String },                 // 这台主机当前认证身份的 login：一个候选，不是已保存的名字
    NoCli,                                   // 服务这座城的机器的搜索路径上没有 gh
    NotLoggedIn,                             // gh 在，这台主机没有登录（gh 的退出码 4）
    Failed { exit: Option<i32> },            // 别的失败：网络、gh 自己的错误；None 是没有退出码（没起来，或超时被停下）
    Stuck { why: String },                   // gh 超时，城停不下它：那个进程还在城的机器上跑，why 是停的时候出了什么错
    NotAHost,                                // host 不是一个主机名，gh 没有被启动
}
```

- **只在人按下时跑。** 这是一条查询，页面在人按「从 GitHub CLI 导入」时问一次；开城、刷新、换页都不问，城也不周期地问。读到的 login 只是候选：页面把它填进「你的 ID」卡，人保存时经 `PutIdentity` 写下（`imported_from` 记 `host`）。答复不改任何文件，所以多账号或环境令牌换了 gh 的当前身份，也不会悄悄改掉已保存的名字。
- **跑在服务这座城的那台机器上。** 城执行 `gh api --hostname <host> user --jq .login`，stdin 为空、`GH_PROMPT_DISABLED=1`，所以 gh 不会停下来等人登录；有上限地等它结束，超时就停下它，答 `Failed { exit: None }`；停不下时答 `Stuck`，因为一个城起了却收不回的进程是人要处理的事实，与「没读到」不同。不需要管理员权限，不读、不记令牌：stdout 只取 login 那一行，stderr 不读。远程设备上的页面问到的是城那台机器的 gh，页面在导入之前说明这一点。
- **主机由人选，默认 `github.com`。** 查询带的主机缺席即 `github.com`，这个默认只写在城一处（`accounting::views::answering::github`）。不是主机名的串（空、以 `-` 开头、带空白或 `/`）答 `NotAHost`，gh 不被启动，所以一个以 `-` 开头的串不会被 gh 读成一个选项。
- **每种失败各有一个名字，页面按它给出路。** `NoCli`：说明装 gh 或手填；`NotLoggedIn`：说明在那台机器上运行 `gh auth login --hostname <host>`，城不替人启动登录；`Failed`：照样留着手填的草稿；`Stuck`：说明那台机器上还有一个 gh 进程；`NotAHost`：说明主机名的写法。没有一种失败清掉卡上已有的值。
- **读的人是视图，跑的人是二进制。** 视图经 `Views::ask_github_through` 收下一个 `fn(&str) -> GithubReading`（与 `ask_upstream_through` 同形），在放开快照之后调用它；生产的那一个是 `bin::doctor::github::login`，找 gh 走 `doctor::host::find_program` 那一条搜索路径，起子进程、停子进程走 doctor 起程序的同一套规矩。没有交这个函数的视图（一次性的 `views::ask`）答 `Unavailable`。
- **`WIRE_V` 不另进位**：`GithubLogin` 是新名字，哈希自己会变（§12.1）。
- 验收：`bin::doctor::github` 的三条——搜索路径上没有 gh 答 `NoCli`、退出码 4 答 `NotLoggedIn`、退出码 0 加一行 login 答 `Found`；accounting 的 `a_github_login_is_asked_of_the_reader_the_views_were_handed`（缺席的 host 问的是 `github.com`，不是主机名的串不去问）。

### 8-68 上手指南的进度按城保存：`Query::Guide`、`Command::PutGuide`

```rust
// Query
Guide,                                       // → Answer::Guide(GuideProgress)
// Command
PutGuide { progress: GuideProgress, idem: IdemKey },
pub struct GuideProgress {
    pub at: Option<GuideStep>,               // 指南下次从哪一步打开；没走过时缺席
    pub state: GuideState,                   // Open：开城时仍给出指南；Left：人离开过，开城直接进对话
    pub dependencies: Option<GuideMark>,     // 第 2 步：依赖项
    pub texts: Option<GuideMark>,            // 第 3 步：文本与称呼
    pub skills: Option<GuideMark>,           // 第 4 步：导入 skill
    pub mcp: Option<GuideMark>,              // 第 5 步：连接 MCP
}
pub enum GuideStep { Provider, Dependencies, Texts, Skills, Mcp }   // "provider" | "dependencies" | "texts" | "skills" | "mcp"
pub enum GuideState { Open, Left }                                   // "open" | "left"；缺省 Open
pub enum GuideMark { Seen, Skipped }                                 // "seen" | "skipped"；缺席即还没看过
```

- **存的是「走到哪、看过什么、跳过什么」，不存「做完了没有」。** 每一步做完没有，由已保存的配置与检测结果推出：第 1 步看服务端已确认的端点与 `main` 的选择，第 2 步看 doctor，其余各看各自的文件与连接。这里存的是另一种进度：人看过哪一步、选了跳过哪一步、下次从哪一步继续、是否已经离开指南。点开一步不算完成，跳过不画成已配置。
- **第 1 步没有标记。** 它是唯一必做的一步，完成与否只由服务端的配置答，没有「跳过」可选；类型里于是没有它的标记格，`at` 仍可以停在它上面。
- **按城保存，换浏览器也在。** 进度写在城的保留子树里（`kernel::layout::CityLayout::guide`，`<城>/.sprawling/GUIDE.toml`），文件就是 `GuideProgress` 的 TOML 序列化，没有第二份键表；写经 `city::edit_document` 整份替换（与 `accounting::person` 写人的配置同一条路）。浏览器的副本只是缓存。没有文件即 `GuideProgress::default()`：从头开始、仍给出指南。读不出的文件答 `Unavailable`，`PutGuide` 拒 `E_CONFIG_INVALID` 并说出文件与原因，不拿缺省值盖掉它。
- **整份写，后到者为准。** `PutGuide` 带整份进度；两个浏览器同时移动指南时，后写的那一份留下。它是一个光标，不是人写的正文，不设基线守卫：被盖掉的最多是一个「看过」的标记，而守卫会让一次普通的翻页被拒。
- **不写账本行。** 进度是界面的位置，不是这座城做过的事；与 `PutPreferences` 一样只落盘。回执是命令的答复本身，页面再问一次 `Guide` 即得此刻的值。
- **`GuideProgress` 住 `wire::guide`，与 `wire::preference` 并列**：它与人的设置一样，是一份页面读、也整份写回的记录，不只是一种答复。
- **`WIRE_V` 不另进位**：三个都是新名字（§12.1）。
- 验收：accounting 的 `the_guide_keeps_its_progress_across_a_reopen`（`PutGuide` 写下的进度，在 worker 丢掉、城经 `views::ask` 重开读之后原样答回；没写过的城答缺省）。

### 8-60 `PutRules`：页面写一栋楼的 `RULES.toml`，整份、带基线、先求值后落盘

```rust
// Command
PutRules(RulesWrite)
pub struct RulesWrite { pub building: Address, pub base: String, pub body: String, pub idem: IdemKey }
```

- **整份文本，一道基线。** `body` 是新的整份 `RULES.toml`，`base` 是页面起手时读到的那份（文件还不存在时为空串）。楼规有两个写者——人在页面上，以及住在楼里的市长经 `rules` 工具提案——所以与 `PutSpine` 同一条守卫：文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`，什么都不写。
- **先求值，后落盘。** 城先用读楼规的同一个求值器（`city::evaluate`）读 `body`，读不出——不合 TOML、缺 `confidential`、机密楼列了出网域名——就拒，盘上不动；求值通过才经基线守卫整份换上去（city-SPEC §8-34）。所以盘上的楼规永远是这个构建读得懂的那一份，下一次派活不会因为一次保存而打不开这栋楼。
- **账上一行 `rules_changed`。** 写成之后城记一行 `rules_changed { scope: building, which: "RULES.toml", before, after, bytes }`，与派活前核对楼规的那一行同形（kernel-SPEC §8-4）；所以下一次派活看到的摘要与账上一致，不会把这次保存读成「有人绕过了门改了文件」。
- **载荷是一个值。** `PutRules` 与 `ConfigureCity` 各带一个结构体而不是平铺的字段，线上形状与平铺时相同（`{"put_rules":{…}}`）；理由是 `Command` 住的文件与 `From<WireCommand>` 那个函数都已经贴着长度上限，一个值占一行。
- **只经页面，不经远程设备之前先分类。** 楼规决定一栋楼能出网、能开浏览器与桌面，远程门打开之后这条帧走哪一类由 R2 定（§19-2 的 `class` 列落地时）。
- 验收：accounting 的 `a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing`（过期的 `base` 被拒、求值失败被拒，两次之后文件不变、账上没有 `rules_changed`；对的 `base` 与能求值的正文落盘并记一行）。

### 8-61 城一级的配置与核心优先级可写：`ConfigureCity`、`PreferencePatch::CorePriority`

```rust
// Command
ConfigureCity(CitySettings)
pub struct CitySettings { pub keep_warm: Option<KeepWarm>, pub effort: Option<Effort>, pub idem: IdemKey }
// PreferencePatch 多一臂
CorePriority(CorePriority)                 // {"core_priority":"raised"} | {"core_priority":"normal"}
pub enum CorePriority { Raised, Normal }   // 值集与拼法住这里，accounting::person 读写 `[core] priority` 用的就是它
```

- **城那一层，两个键。** `ConfigureCity` 写城自己那份 `CONFIG.toml`（`<city>/.sprawling/CONFIG.toml`）：`keep_warm` 写 `[cache] keep_warm`，`effort` 写 `[model] effort`；`None` 不动那一项。城那一层从梯子的最远一端说话，楼与房间各自的一层照旧压过它（city-SPEC §8-4）。`ConfigureBuilding` 不改：它的地址就是它写的楼，城那一层没有地址可写，所以是另一条帧，而不是 `ConfigureBuilding` 收一个特殊地址。
- **账上一行。** 写成之后城记一行 `rules_changed { scope: city, which: "CONFIG.toml", … }`，与派活前核对城配置的那一行同形。
- **核心优先级是这个人自己那一层。** `CorePriority` 进 `PreferencePatch`，所以经已有的 `PutPreferences` 写，落在 `~/.sprawling/config.toml` 的 `[core] priority`——那是它一直住的地方（sprawling-SPEC 8-93），不在 `[ui]` 里，所以 `PreferencesAnswer` 不带它；页面从 `Query::Doctor` 的核心一项读到它此刻的效果。写下之后，下一次 `serve` 起线程时读它。
- 验收：city 的 `a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it`（城层写 `keep_warm` 之后，一间没有说话的房间读到 `FiveMinute`；写 `effort` 之后梯子答它来自城那一层）；accounting 的 `the_core_priority_lands_in_its_own_section_and_reads_back`。

### 8-62 自动化只读组：`Query::Automation`

```rust
// Query
Automation,                                  // → Answer::Automation(Box<AutomationAnswer>)
pub struct AutomationAnswer {
    pub jobs: Vec<ScheduledJob>,             // SCHEDULE.toml，文件里的顺序
    pub sources: Vec<WatchedSource>,         // WATCH.toml，文件里的顺序
    pub unreadable: Vec<String>,             // 读不出的那份文件：文件名与拒因；它的行不列
}
pub struct ScheduledJob { pub name: String, pub addr: Address, pub task: String, pub goal: String, pub cadence: Cadence }
pub enum Cadence { EveryMinutes { minutes: u64 }, DailyAt { minute: u64 }, WeeklyAt { minute: u64 } }   // minute：UTC 的一天／一周里的第几分钟
pub struct WatchedSource { pub name: String, pub matches: String, pub addr: Address, pub starts_work: bool }
```

- **只读，读的是文件此刻。** 两份文件由人在城根上写，城每一拍读一次 `SCHEDULE.toml`，起服务时读一次 `WATCH.toml`；本查询在问的那一刻经 `city::Schedule::load` 与 `city::Watch::load` 各读一次，答的正是下一拍会读到的东西。页面上不写这两份文件（S06 Q4 (b)）：写它们要一套 cron 与路由的编辑器，而 CLI 与编辑器已经够得到。
- **一份读不出不挡另一份。** 读不出的文件进 `unreadable`，拒因就是派活那一拍会给的那一句；另一份照常列出。没有文件即空列表，那是没设自动化的城的常态。
- **`Cadence` 是线上自己的拼法。** `city::Cadence` 不派生 serde（它是城判定的值），本 crate 定一份线上形状，装配处一一映射，三个值一一对应、穷尽匹配，不另存规则。
- 验收：accounting 的 `the_automation_query_lists_both_tables_and_names_the_one_that_does_not_read`。

**(b) 从检查点取回单个文件：`Command::RestoreFile`**（S07 Q2 (c)）

```rust
RestoreFile { at: Address, point: GitOid, idem: IdemKey }
```

- **把城自己工作树里的一个文件换回检查点里的那一份。** `at` 是文件在城里的地址（`Address` 爬不出城、点不到保留子树），`point` 是一个检查点的 oid（页面从 `checkpoint_committed` 或 `Query::Commits` 读到）。检查点里有这个文件：工作树里这一处的字节换成检查点里的那一份（原子替换，取被替换文件的权限）；检查点里没有：工作树里这一处的文件删去。不回到过去开一棵树（S07 Q2 (c) 只做取回单个文件）。
- **有 run 在这栋楼里干活就拒。** 拒 `E_BUSY`，点名房间与 run：取回会改掉 run 正在写的那棵树，与 `RemoveBuilding` 同一条理由。
- **一步一行。** 写成之后记一行 `file_restored { name: "", path, point }`；`name` 为空串指城自己的工作树，非空时仍是一棵 run 的工作树的名字（kernel-SPEC §8-4，storage-SPEC §8-33）。
- 验收：storage 的 `taking_a_file_back_replaces_what_the_tree_holds_and_removes_what_the_point_did_not_hold`。

### 8-63 页面上「历史已证明到哪一条」：`CityAnswer.proved`

```rust
pub struct CityAnswer { /* …既有字段… */ pub proved: Option<Seq> }
```

- **意思。** `Some(n)`：账本到 `n` 为止整条链已经证明完好，写者在接受命令（sprawling-SPEC 8-122 的 M3）；证明之后写下的每一行都由这个已证明的写者接在链上，所以 `n` 就是视图此刻折到的最后一条。`None`：服务中的城还在后台证明（命令此时答 `E_HISTORY_UNPROVEN`），或者证明发现链断了；视图照常答查询（S10 Q2 (a)），页面据此标明「历史还在核对」。
- **读法。** 视图持有写者挂上的那个 `storage::ChainHalt` 的一份句柄（sprawling-SPEC 8-134）；`proved()` 为真时答视图的头。一次性查询（`views::ask`）与测试里的视图起步前已经同步证明过整条链，答它们的头。
- 名字不变而形状变，与本批共用 `WIRE_V` 45。

### 8-64 `Sample` 多两项，`ModelTag` 多一值

```rust
pub struct Sample { /* …既有 13 项… */ pub view_backlog: u64, pub read_nanos: u64 }
pub enum ModelTag { /* …既有… */ Ocr }      // 线上 "ocr"
```

- `view_backlog`：写者已经交给视图线程、还没折完广播的已提交记录条数，读 `bin::serving::folding::Backlog::records`（sprawling-SPEC 8-123）；采样线程每一拍读一次。
- `read_nanos`：上一拍读计数器花了多少纳秒，由采样线程用单调钟在读取前后各量一次；第一拍为 0（sprawling-SPEC 8-129-6）。
- `ModelTag::Ocr`：人登记的一个能读图的模型，城的 OCR 工具读这一次选择（gateway-SPEC §8-34）。二进制里不带任何模型（D18），这个值只是一个登记位。
- 三项都是名字不变的改形，共用 45。

### 8-65 动词类：§19-2 的 `class` 列

```rust
// bin::outside::verbs（sprawling）：中继读这一列的那一处
pub(super) fn command_class(command: &wire::WireCommand) -> remote_access::door::VerbClass;   // 穷尽匹配，无通配臂
```

- **`class` 是 §19-2 的一列，不是 wire 的一个方法。** 它说的是远程门放不放一帧进来，而远程门的权限与 `VerbClass` 归 `remote_access`（crates/remote_access/Spec.lean §8-1）；本 crate 不依赖它，也不为它另立一个同值的枚举。中继在 `sprawling`，那里同时看得见两者（crates/remote_access/Spec.lean §7）。
- **表与匹配由门机器对照**：`xtask wiring` 读表的第三格与 `command_class` 的每一臂，表里缺格、读不成三个取值之一、或与匹配说法不一，都点名那一个动词（xtask-SPEC §8-45）。匹配是穷尽的，所以一个新 Command 在有人定下它的类之前编译不过。
- 不改任何帧，不动 `WIRE_V`。

### 8-66 远程中继怎样用这条线

- **设备说的是同一条线。** 远程会话里封装的每一帧文本（crates/remote_access/Spec.lean §8-5 的 `Payload::Frame`）就是一帧 `ClientFrame` 或 `ServerFrame`；中继打开封装、按 §19-2 判类，放行的帧**原样**发给城自己在回环上的 `/ws`，不解析后再序列化。
- **`Hello` 换成城自己的**：设备不知道、也不该知道城的配对令牌（§8-41）。中继把设备的 `Hello` 里的 `token` 换成城的令牌（没有就是空），`wire_v` 与 `schema` 照设备说的发，所以设备上的页面与城说不说同一版线，仍由 `server::decide_handshake` 判。
- **拒绝是一帧 `Refusal`**：被门拒的帧不到城，中继封一帧 `ServerFrame::Refusal` 回给设备，码是 `E_GATE_DENIED`，恢复语说该在城自己的机器上做，或该重新配对为 `act`。读不成 `ClientFrame` 的文本同样封一帧 `E_WIRE_MISMATCH` 回去。
- 城发来的每一帧都封好送回设备，事件、答复、增量一视同仁：一台 `Watch` 设备能读的就是这座城的整条线，这正是「看」的意思。

## 19 每个动词从哪里够得到（`xtask wiring` 的数据面）

**这张表存在的理由，是一次已经发生过的失效。** v0.0.3 的审计发现 `accounting::worker::run_command` 只匹配 22 个 Command 里的 14 个，
六个动词落进 catch-all——其中 Takeover／Rollback／CreatePolicy **在线上、画在客户端、由任何东西执行不了**，
而 Cancel 与 Steer 在 run 不处于安全点时失败，恰好是人最需要它们的那一刻。
`not_built` 的 rustdoc 当时就写着「**在这里被回绝的动词不得作为控件出现在客户端**」——那是一条**没有任何机器在看的规矩**。

反方向同样会失效，而且更安静：**一个城做得到、却没有任何控件够得到的能力，没有人会收到抱怨**，
因为不存在的按钮不会有人去点。`Pursue`（「城自己走」）与 `SetAutonomy` 就是这样漏掉的，
这也正是验收标准第 2 条一直没过的机制原因——**在浏览器里打不开它**。

### 19-1 reach 的四个取值

表里的第二个事实是 `class`：这个 Command 由一台远程设备经远程门发来时属于哪一类动词（§19-3）。reach 说城里谁该够得到它，class 说城外的设备能不能带它进来，两件事互不推出，所以是两列。

| 取值 | 含义 | 门要求什么 |
|---|---|---|
| `client` | 人用的动词，客户端必须画得出 | `client/src` 里有发出点，且 `run_command` 不以 `not_built` 作答 |
| `push` | 由外部服务推进来，不是人点的 | 只要求 `run_command` 能执行；客户端有没有它都不看 |
| `handshake` | 在握手层被吃掉，进不到 `run_command` | 两侧都不要求 |
| `sealed` | 线上不可拼写 | 两侧都不要求；客户端**若**出现即为红 |

### 19-2 表

| Command | reach | class | 说明 |
|---|---|---|---|
| `Dispatch` | client | Act | 派活，产品的正面 |
| `ProbeEndpoint` | client | LocalOnly | 问一个端点它供应什么 |
| `ConfigureBuilding` | client | LocalOnly | 改一栋楼的规矩 |
| `AttachEndpoint` | client | LocalOnly | 把一个端点挂上 |
| `SelectModel` | client | LocalOnly | 选一个模型 |
| `OpenSession` | client | Act | 在同一个地址上开新的一段会话（`Carry` 说带不带上一段的交接，`from` 说从哪条线哪一行分出来） |
| `Reveal` | client | LocalOnly | 在人自己的文件管理器里指出一个地址 |
| `RestoreDiscard` | client | LocalOnly | 把回收站里一行按它自带的回去的路放回原处 |
| `DoctorInstall` | client | LocalOnly | 按需求表里的名字装一件机器缺的东西 |
| `DoctorRefresh` | client | LocalOnly | 重新探一遍机器，取代开城时的快照 |
| `ConnectToolkit` | client | LocalOnly | 请外包服务开一次同意会话，把一个外部应用接进来 |
| `PutSpine` | client | Act | 写一栋楼自己的 spine 文档（roadmap／memo／handoff／spec）。携 `base`（发信方起手时那份正文）与 `body`，文件已被人或居民改过即拒——**这几份有两个写者**，与 `PutDocument` 的单写者前提不同，故两道门的守卫不同 |
| `CreateBuilding` | client | LocalOnly | 起一栋楼 |
| `RemoveBuilding` | client | LocalOnly | 把一栋楼移出城：文件搬进 reserved subtree（city-SPEC §8-3），历史留在 Ledger，写 `building_removed`；有 run 正在其中某个房间里跑时拒 `E_BUSY`，点名房间与 run |
| `Steer` | client | Act | 中途换方向 |
| `Cancel` | client | Act | 停下这一个 |
| `Halt` | client | Act | 停下一个范围 |
| `Release` | client | Act | 放开一个范围 |
| `Approve` | client | Act | 答一条审批 |
| `SetAutonomy` | client | LocalOnly | 定一栋楼的 Autonomy（两态：本人或被任命的居民） |
| `HandOff` | client | Act | 把一条问题转给另一位居民去答 |
| `PutPreferences` | client | LocalOnly | 写这个人自己的 `~/.sprawling/config.toml` 的 `[ui]` |
| `PutShelved` | client | LocalOnly | 写一份上架的文档（技能或说明） |
| `Pursue` | client | Act | 设一个持续追的目标，以及暂停／恢复／清除 |
| `PutDocument` | client | LocalOnly | 写治理这座城的三份文件之一，携 `base`（§8-59） |
| `PutIdentity` | client | LocalOnly | 写设置页上的卡片：城在 `base` 上改写 `PREFERENCES.md` 或 `MAYOR.md` 的身份区（§8-59） |
| `PutRules` | client | LocalOnly | 写一栋楼的 `RULES.toml`：整份、带 `base`，先求值后落盘（§8-60） |
| `ConfigureCity` | client | LocalOnly | 写城自己那一层 `CONFIG.toml` 的 `keep_warm` 与 `effort`（§8-61） |
| `RestoreFile` | client | LocalOnly | 把城工作树里的一个文件换回一个检查点里的那一份，检查点里没有就删去（§8-62(b)） |
| `PutGuide` | client | LocalOnly | 写这座城的上手指南进度：走到哪一步、看过与跳过了哪几步、是否已离开指南（§8-68） |
| `BatchByBuilding` | client | Act | 按楼成批派活 |
| `Wake` | push | LocalOnly | 外面发生了一件事；地址由 watch 表与 triage 决定，调用方说不出房间 |
| `Auth` | handshake | LocalOnly | 出示配对令牌，`server::decide_handshake` 吃掉它 |
| `PutSecret` | sealed | LocalOnly | 唯一没有字节形式的 Command；`Sealed<String>` 在线上不可居留 |

### 19-3 class 的三个取值

| 取值 | 含义 | 远程设备 |
|---|---|---|
| `Read` | 读城，什么也不改 | `Watch` 与 `Act` 两种权限都带得进来 |
| `Act` | 人离开电脑时仍要做的活：派活、改方向、停下、叫停与放开、答审批、转交、按楼成批派活、追目标、开新一段会话、写楼自己的 spine 文档 | 只有 `Act` 权限带得进来 |
| `LocalOnly` | 放宽访问、够到凭证或城所在的宿主机、改变治理这座城的东西：接端点、选模型、建楼拆楼、写规则与配置、装东西、开文件管理器 | 恒不带进来，不论权限 |

- **新加的 Command 一律 `LocalOnly`**，除非人决定一台不在电脑旁的设备可以做它。`Act` 与 `Read` 是一次决定，不是默认值；表里一行缺 `class` 格，`xtask wiring` 点名那一行。
- `class` 只判 Command。`Ask` 与 `Monitor` 两种帧属 `Read`；设备发来的 `Hello` 由中继换成城自己的令牌再发（§8-66）；远程门自己的动词（开门、配对、撤销）不在线上，表里没有它们（remote_access D4）。
- 这一列是权威，中继按它判，`xtask wiring` 把表与中继的穷尽匹配（`bin::outside::verbs::command_class`）逐行对照（§8-65）。

**`client` 而尚未落地的三个**（`HandOff`／`PutShelved`／`BatchByBuilding`）今天由 `not_built` 作答，
所以门对它们要求的是**客户端不画**——`not_built` 的 rustdoc 说的就是这件事，现在有机器看着了。
它们的 reach 仍写 `client`，因为那是它们做完之后该去的地方；写成别的取值等于把「还没做」记成「不该做」。
