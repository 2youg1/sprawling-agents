-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::frames

规定 `frames`、`frames::query`、`frames::tests`、`answer::building`（`crates/wire/src/` 下同名的文件）。线上的信封：五种帧、`WIRE_V` 与 schema 哈希，线的另一端由这一端生成，以及只在线上活着的三类帧（增量、日志、命令输出）。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
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
// resume_from：账本头（最后广播的记录）的 seq，读自 ServeConfig.head: Arc<LedgerHead>；客户端据此把断线期间的缺口经 HistoryRange 补齐（client/Spec.lean §4-40）。
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

5. **wire 携 git 的 oid 时携 `kernel::GitOid` 本体**（`Query::Commit`／`Query::Hunks` 的 oid，与紧邻的 `B3Hash` 同形：40 位小写 hex，长度不对即拒），故 `GitOid` 带 serde。**被否**：在 wire 里自建 `CheckpointRef(String)` 并自校 40 hex——那是 git oid 形状的第二个权威。该变更属 kernel 公开面，已与 `crates/kernel/Spec.lean` §8-2 同集提交。
-/

/-!
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
-/

/-!
### 8-16 线的另一端由这一端生成（`wire_schema`，feature `schema`）

**需求**：`client/`（TypeScript）要与 `crates/wire` 说同一门语言，而 §8 从头到尾只承认一个权威——Rust 的类型声明。手写一份 TS 类型就是第二个权威，它会在握手通过之后才被发现漂了。故 TS 面由这一端**生成**：`cargo xtask wire-ts` 读本 crate 的 JSON Schema，写出 `client/src/wire.ts`（每个类型一条 TS `type` 加一条 Effect `Schema` 值，外加 `WIRE_V` 与 `WIRE_HASH`）；不带 `--write` 时只比对盘上文件，第一处不同的行即门红。

**接口**（feature `schema`，缺省关；`web` 以 `default-features = false` 依赖本 crate，产品二进制不开它）：

```rust
#[cfg(feature = "schema")]
pub fn wire_schema() -> serde_json::Value;   // 一份文档：`$defs` 里是信封两端可及的每一个具名类型，
                                             // 含 `ClientFrame` 与 `ServerFrame` 两个根
```

- **哈希的素材是整个线上名字面**：`schema_hash()` 吃 `WIRE_V`、命令名表、查询名表，以及 `kernel::EventKind::ALL` 按表序的每个种类名——事件种类随每个事件帧到达页面，改名、增删一个种类与改名一个帧一样会让旧页面误读，所以它移动握手哈希，不靠有人记得去升 `WIRE_V`。生成的 `WIRE_HASH` 常量就是这个函数的输出，客户端在握手处送回它，服务端按原样校验——两端校验的是同一个值，而不是一个「schema 文档的摘要」；后者会把每一条 doc 注释的改动都变成一次拒配。
- **每个入帧的类型都派生 `schemars::JsonSchema`**（`#[cfg_attr(feature = "schema", derive(...))]`），派生宏读的是 serde 已经在读的属性，故形状与编码同源。kernel 侧的值经 kernel 自己的 `schema` feature 派生（`crates/kernel/Spec.lean` §8-45）；`NoSecret` 手写 `impl JsonSchema` 为 `false`（任何值都不满足），于是 `PutSecret` 臂在 TS 里是 `value: never`——线上拼不出它，这一句在两端各说一次、意思相同。`Command<Secret>` 以 `schemars(rename = "Command")` 命名，因为线上只有 `Command<NoSecret>` 一种实例。
- **生成器只认 serde 会产出的那个子集**：对象（`properties`／`required`／`additionalProperties`）、`string`／`integer`／`number`／`boolean`／`null`、`array`（`items`）与元组（`prefixItems`）、`enum` 字符串表、`const`、`oneOf`／`anyOf`、`$ref` 指向 `#/$defs/…`、`type: [T, "null"]`、`true`／`false` 两种布尔 schema。其余一律拒绝并点名关键字与所在类型——一个会猜的生成器就是一个会静默产出错类型的生成器。具名的裸 `string`／`integer` 即 newtype，TS 侧打上 `Schema.brand(名)`。
- **文件确定**：`$defs` 按名排序后按依赖拓扑输出（Effect 的 `Schema` 值必须先定义后引用；环即拒绝），对象键排序，LF 行尾，生成头注明来源。

**被否**：（a）在 wire 用 schemars 的 remote derive 镜像 kernel 的四十个类型——每个镜像是同一形状的第二个权威，而 §8-1 第 5 条早已为 `GitOid` 拒过同一形状的提案；（b）把 schema 文档的摘要作为握手哈希——doc 注释入哈希，改一句注释即旧页面全拒；（c）手写 `wire.ts`——正是本节要关掉的那扇门。

**`answer.rs` 随之切出 `answer/building.rs`**：二十六条 `cfg_attr` 派生行把 381 行推到 407 行，越过 400 行预算，故一栋楼说自己的七个读形状（`BuildingProgress`／`BlockedLine`／`PlanRow`／`PursuitLine`／`BuildingDoc`／`ArchiveLine`／`BuildingAnswer`）迁入 `crates/wire/src/answer/building.rs`，`answer.rs` 以 `pub use` 引回，公开拼写不变；文字逐字节照搬，无字段开放。定义模块从 `wire::answer` 变为 `wire::answer::building`，那是住处，不是接口变更。

**本节的公开面变更**：wire 多出 `wire_schema`（仅 feature `schema`，缺省 feature 下不存在）；kernel 在 `--all-features` 下多出四十余条 `JsonSchema` 实现（缺省 feature 下不存在）。
-/

/-!
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
-/

/-!
### 8-48d `ServerFrame::Output`，一条还在跑的命令写出的字节

```rust
pub enum ServerFrame { …, Delta(Delta), Log(LogLine), Lagged(Lagged), Output(LiveOutput) }
pub enum OutputStream { Out, Err }        // 恒不是 bool
pub struct LiveOutput { pub run: RunId, pub stream: OutputStream, pub text: String }
```

- **与 `Delta` 同一条规则**：可丢弃，不带账本序号，不进账本；调用的结果以 `tool_returned` 落账时页面扔掉它画的这段，两者不一致时账本赢。
- **第四条广播通道 `ServeConfig::outputs`**：一条刷屏的命令不该把模型的增量或日志行挤出慢读者的窗口；`RecvError::Lagged` 一言不发地略过，理由与增量相同——漏掉的字节在调用落账时整段到达。
- **`text` 是 UTF-8 有损解码**，因为一块在字节上界处切开，可能切在一个多字节字符中间；切口处画成替换字符只影响预览，结果以账本为准。**被否**：线上带字节数组——JSON 里一个字节要三四个字符，而页面最终画的是文本。
- **`ServeConfig::outputs_so_far: Arc<dyn Fn() -> Vec<LiveOutput> + Send + Sync>`**：一个会话在送出 `Welcome` 之后、接实时帧之前调它一次，把还在跑的命令已经写出的字节按原来的次序作为 `Output` 帧送出，所以在命令跑到一半时打开页面的人先看到已经写出的部分。会话先订阅第四条通道再调它，所以一块可能送两遍而不会漏；预览里重复一块无害，漏一块则要等调用落账才补上。缓冲住在装配层（`crates/sprawling/Spec.lean` §8-90），因为清空它要看账本里的 `tool_result`，而本 crate 不折叠账本。
- **`OutputStream` 是 `runtime::Stream` 的第二处拼写而不是第二处权威**：本 crate 的依赖图够不到 `runtime`，映射住在装配层（`sprawling::assembly` 的 `serve`），与 `LogLevel`（§8-32）同一个安排。
-/

namespace Wire.Frames

/-- 一份构建在线上说的语法：它的 `WIRE_V`、进 `schema_hash()` 的名字（命令名、查询名、事件种类名，按表序），与名字之外的形状（答面与帧的字段、一个枚举的值、`ServerFrame` 的臂）。形状是模型参数，模型只需要分得清两份形状同不同。 -/
structure Grammar (Shape : Type) where
  version : Nat
  names : List String
  shape : Shape

/-- 握手比对的那一对：`(WIRE_V, schema_hash())`。哈希是参数 `hash`，它吃 `WIRE_V` 与名字表（`frames::schema_hash`）。 -/
def key {Shape Digest : Type} (hash : Nat → List String → Digest) (g : Grammar Shape) :
    Nat × Digest :=
  (g.version, hash g.version g.names)

/-- 两份构建握手成功：两端的那一对相等（`server::decide_handshake` 的版本与哈希两格）。 -/
def Handshakes {Shape Digest : Type} (hash : Nat → List String → Digest)
    (a b : Grammar Shape) : Prop :=
  key hash a = key hash b

/-! D1 `WIRE_V` 在两次推送之间最多进一位，进在第一个改形的提交

**决定**：握手要保证的只有一件事：两份可能相遇的构建，只要线上语法不同，`(WIRE_V, schema_hash())` 就不同。`schema_hash()` 已经吃进命令名、查询名与事件种类名（§8-1、§8-16），所以增删、改名、调序一个 `Command`／`Query` 变体或一个 `EventKind`，哈希自己会变，`WIRE_V` 不为此进位。名字都不变而形状变了——答面或既有帧加字段、一个枚举换词或加值（`Mode`、`ModelTag`、`Note` 这类）、`ServerFrame` 加一臂——哈希不变，`WIRE_V` 必须进位。进位按推送计：上一次推送之后，第一个名字不变而改形的提交把 `WIRE_V` 加一，此后到下一次推送之前的改形共用这个值；推送之后再遇到改形，再进一位。

**理由**：会相遇的构建在人手上：发布版与推送过的 `main`。一个由构建 X 送出的页面连到构建 Y 的服务端，只发生在人换了二进制而页面还开着或缓存着的时候；没推送的中间提交只在开发机上跑，彼此不会各自服务一个人的页面。进在第一个改形的提交而不是推送前收尾：收尾才进，中间各树会以上一次推送的号放行语法不同的旧页面。改形只由一个写者按次序落地：两条并行分支各自进位，曾让两份语法不同的构建共用一个号、互相通过握手。

**被否**：①每个改形的提交各进一位：更严，但一次推送带出几个号只取决于提交怎么切，读者从号上读不出任何东西；②推送前统一进位：理由见上；③改名也进位：哈希已经拒掉旧页面，多进一位不多拒任何页面。

**守护**：`tests/wire_contract.rs` 钉住去掉文档文字之后的 `wire_schema()` 摘要。改形即红，失败信息写出本条；改的人据此判断这一次要不要进位，再更新摘要。它不判断该不该进位，那取决于上一次推送之后是否已经进过一位。
-/

/-- **语法不同的两份构建握不上手。** 哈希对名字表单射（同一个 `WIRE_V` 下），且名字都不变而形状变了的两份构建 `WIRE_V` 不同（D1 的进位规则）时，名字或形状不同的两份构建，握手那一对必不相等。 -/
theorem distinct_grammars_never_handshake {Shape Digest : Type}
    (hash : Nat → List String → Digest)
    (injective : ∀ v n m, hash v n = hash v m → n = m)
    (a b : Grammar Shape)
    (bumped : a.names = b.names → a.shape ≠ b.shape → a.version ≠ b.version)
    (differ : a.names ≠ b.names ∨ a.shape ≠ b.shape) :
    ¬ Handshakes hash a b := by
  intro meet
  have versions : a.version = b.version := congrArg Prod.fst meet
  have digests : hash a.version a.names = hash b.version b.names := congrArg Prod.snd meet
  rw [versions] at digests
  have names : a.names = b.names := injective _ _ _ digests
  cases differ with
  | inl other => exact other names
  | inr reshaped => exact bumped names reshaped versions

/-- 同一份构建与自己握得上手：这条保证不是从一个满足不了的前提推出来的。 -/
theorem a_grammar_meets_itself {Shape Digest : Type} (hash : Nat → List String → Digest)
    (g : Grammar Shape) : Handshakes hash g g :=
  rfl

/-- **不进位就有反例。** 名字一个没改而形状变了，哈希不变；`WIRE_V` 也不动的话，两份语法不同的构建互相通过握手。这就是 D1 要 `WIRE_V` 为改形进位的原因。 -/
theorem a_reshaped_grammar_meets_the_old_one_without_a_bump {Digest : Type}
    (hash : Nat → List String → Digest) :
    Handshakes hash ({ version := 45, names := ["Dispatch"], shape := 0 } : Grammar Nat)
      { version := 45, names := ["Dispatch"], shape := 1 } :=
  rfl

/-- 两次推送之间的提交，逐个说自己是不是名字不变而改形的那一类；从上一次推送时的号 `v` 起，答每个提交的 `WIRE_V`：第一个改形的提交进一位，此后到下一次推送共用这个号。 -/
def numbered (v : Nat) : List Bool → List Nat
  | [] => []
  | false :: rest => v :: numbered v rest
  | true :: rest => (v + 1) :: rest.map (fun _ => v + 1)

/-- 两次推送之间 `WIRE_V` 至多进一位。 -/
theorem numbered_moves_at_most_once (v : Nat) (commits : List Bool) :
    ∀ n ∈ numbered v commits, n = v ∨ n = v + 1 := by
  induction commits with
  | nil => intro n member; simp [numbered] at member
  | cons head rest step =>
    cases head with
    | false =>
      intro n member
      simp only [numbered, List.mem_cons] at member
      rcases member with equal | inner
      · exact Or.inl equal
      · exact step n inner
    | true =>
      intro n member
      right
      simp only [numbered, List.mem_cons, List.mem_map] at member
      rcases member with equal | ⟨_, _, equal⟩
      · exact equal
      · exact equal.symm

/-- 一次推送之间有过改形，就有一个提交带着进过位的号。 -/
theorem a_reshaping_commit_is_numbered_past_the_push (v : Nat) (commits : List Bool)
    (reshaped : true ∈ commits) : v + 1 ∈ numbered v commits := by
  induction commits with
  | nil => cases reshaped
  | cons head rest step =>
    cases head with
    | true => simp [numbered]
    | false =>
      simp only [List.mem_cons] at reshaped
      rcases reshaped with wrong | inner
      · cases wrong
      · simp only [numbered]
        exact List.mem_cons_of_mem _ (step inner)

/-- 没有改形的推送，号不动。 -/
theorem no_reshaping_keeps_the_number (v : Nat) (commits : List Bool)
    (unchanged : ∀ c ∈ commits, c = false) :
    numbered v commits = commits.map (fun _ => v) := by
  induction commits with
  | nil => rfl
  | cons head rest step =>
    have first : head = false := unchanged head (List.mem_cons_self ..)
    subst first
    simp only [numbered, List.map_cons]
    rw [step (fun c member => unchanged c (List.mem_cons_of_mem _ member))]

end Wire.Frames

/-! D22 V0.0.9 的线上改形一次进位，由一个变更集落地

**决定**：V0.0.9 的全部线上改形——`Call`／`Used` 的微秒耗时（D28）、`SessionLine` 的五件与 `NameSession`、`ChangeRunPolicy` 两帧（D27）、`HarnessLine.state`（D23）、`ReleaseAnswer` 的注册表与更新命令（D24）、`DoctorAnswer.scanning`（D25）、沙箱臂名（D26）、skill 与 MCP 的三个查询（D28）、`theme` 偏好（D29）——在同一个变更集里落地，`WIRE_V` 按 D1 只进一位；同一个变更集重生 schema golden、`client/src/wire.ts`（`cargo xtask wire-ts`）、docnum，并改 `tools/adversary/` 的 Door 与 Regression。之后的车道不再改线；确有一处必须改时，由整合者在合并时重生上面这几样，`WIRE_V` 在两次推送之间仍至多进一位。

**理由**：升版的代价是 `wire.ts` 重生与客户端同改，与改动的数量无关（§8-39），分十次就付十次，而且十条车道同时改帧表会在合并时十次冲突。先把形状写定、再一次实现，是 V0.0.9 的执行规则（接口先定）。

**被否**：每个功能车道各自改自己的帧：帧表、golden 与 `wire.ts` 是共享的热点文件，每次合并都要重生，`WIRE_V` 还会在一次推送里进好几位。

**重开参数**：本版之后的下一版若只有一两处线上改动，按 D1 照常各自进位即可，不必再集中。
-/
