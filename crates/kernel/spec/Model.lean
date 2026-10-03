-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::model

规定 `kernel::model`（`crates/kernel/src/model.rs` 与 `crates/kernel/src/model/` 下的 `seam`、`wire`、`usage`、`image`、`mode`、`window`、`conformance`）：模型端口、canonical 会话类型、增量与提前交出的调用、运行策略。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::model::tests` 守住。
-/

/-!
### 8-24 kernel::model（缝清单文件）

```rust
pub struct BuildingPolicy { pub confidential: bool }      // 构造子 new(confidential)
pub struct ModelReturn { pub message: Payload, pub calls: Vec<ToolCall>,
                         pub usage: Option<ModelUsage>, pub stop: Option<StopReason>,
                         pub billed_usd_micros: Option<UsdMicros> }
                                    // message＝助手内容（入窗载荷）；calls＝请求的工具波（空＝本回合无工具，回合层据此收束）
pub trait Model {
    /// One provider call; adapters never sample clocks or read globals.
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;
}
#[cfg(feature = "conformance")]
pub fn assert_model_conformance<M: Model>(model: &mut M, benign: &ModelRequest);
```

**canonical 会话类型族**（城内规范 Dialect 的缝上定义；gateway::dialect 只做翻译，两适配器与剧本模型消费同一形）：

```rust
pub enum Role { User, Assistant }                      // wire 枚举，封闭：认不出的词在反序列化处即拒
pub enum StopReason { EndTurn, ToolUse, MaxTokens }
pub enum ModelTag { Main, Digest, Transcribe, Ocr }    // ALL: [ModelTag; 4]（Ocr 见 §8-80）
pub struct SystemBlock { pub text: String, pub cache: bool }             // cache＝显式断点标记
pub enum ContentBlock { Text{text} | Thinking{thinking, signature}
                                        | RedactedThinking{data}
                                        | ToolUse{id, name: ToolName, input: Payload}
                                        | ToolResult{tool_use_id, content, is_error} }
pub struct ChatMessage { pub role: Role, pub content: Vec<ContentBlock> }
pub struct ToolDef { pub name: ToolName, pub description: String, pub input_schema: Payload }
pub struct Ceiling(NonZeroU64);  // 零不可表达：new(0) 即 None
pub enum MessageBreakpoint { Unmarked, Tail }        // 请求侧注记，不进 serde；缺省 Unmarked
pub struct ChatRequest<'a> { pub model: String, pub max_tokens: Option<Ceiling>, pub system: Vec<SystemBlock>,
                         pub messages: Cow<'a, [ChatMessage]>, pub tools: Cow<'a, [ToolDef]>,
                         pub breakpoint: MessageBreakpoint, pub effort: Option<Effort> }
impl ChatRequest<'_> { pub fn carries_breakpoint(&self, index: usize) -> bool; }  // 该下标的消息是否带断点：Tail 且为末条
pub struct ModelRequest<'a> { pub policy: BuildingPolicy, pub segments: [B3Hash; 4], pub chat: ChatRequest<'a> }
                                    // segments＝冻结 prefix 分段哈希（与 prompt_assembled 同源）
pub enum CacheCount { Reported(Tokens), Unreported }  // 缓存读／写的一个数：报了多少，或没报（D36）
impl CacheCount { pub fn reported(self) -> Option<Tokens>; pub fn or_zero(self) -> Tokens; }
pub struct ModelUsage { pub input_tokens: Tokens, pub output_tokens: Tokens,
                        pub cache_read_tokens: CacheCount, pub cache_write_tokens: CacheCount,
                        pub dialect: Option<DialectKind> }
pub struct ChatResponse { pub content: Vec<ContentBlock>, pub stop: StopReason, pub usage: ModelUsage }
pub fn message_payload(content: &[ContentBlock]) -> Result<Payload, AxError>;  // model_returned 载荷的唯一成形处
```

`ModelUsage.input_tokens` 在每种兼容格式下都是**这次请求的全部输入 token，含缓存读与缓存写**。OpenAI 两个兼容格式本来就这样报；Anthropic 的 `input_tokens` 只数未缓存的部分，由它的解析器加上两个缓存数。选这个口径是因为上下文量表读的正是它（一次请求占了多大的窗口），而按价单结算时用 `input_tokens - cache_read_tokens - cache_write_tokens` 求未缓存部分只需一次减法。`dialect` 记下是哪个兼容格式报的；脚本模型与测试不经兼容格式，记 `None`。

账本只追加：`model_returned.usage` 在写时带 `"v": 1`（本口径）与 `dialect`，旧行字节不改。读者一律经 `ModelUsage` 的 `Deserialize` 读这一格，版本换算只在那里做：没有 `v` 的旧行没有兼容格式可查，当 `cache_read_tokens + cache_write_tokens > input_tokens` 时它只可能是 Anthropic 的旧口径（全部输入不会小于其中的缓存部分），读成三者之和；否则照写的读。两种旧口径在没有缓存时一致，所以照读只会把「有缓存、且缓存部分不超过未缓存部分」的 Anthropic 旧行读小，这种行在带长前缀的会话里少见。缓存两数在行里是可缺的键：`Unreported` 不写这个键，读时缺键与 `null` 都读作 `Unreported`，在场的 `0` 读作 `Reported(0)`（D36）；换算旧口径时 `Unreported` 按 0 计，因为这样的行在本格式出现之前都写着数。

浮点禁令只有一个家：`Payload::new`（及其 `Deserialize`）。wire 面把 `serde_json::Value` 转成
`Payload` 即受判，故 seam 不再另设判定原语，拒绝理由与错误码也只有一处。

- 工具入参／schema 用 Payload：浮点禁令在缝上即成立（这些字节逐字进 Ledger 载荷）；provider 送浮点工具入参＝E_WIRE_MISMATCH（fail-closed，城规优先）。
- ModelRequest 携 chat 字段（turn 的 Assembling 相组 ChatRequest 入请求）；ModelReturn 携 usage/stop/billed 三字段，另有 `bare()`（脚本最小构造）与 `from_response(resp, billed)`（tool_use 块→波，全量入账）两构造面；turn 的 model_returned 载荷随之增 usage／stop／billed_usd_micros（在场才写）。
- BuildingPolicy 住本缝而非 city：kernel 不能依赖外层，city::policy（P1）是它的**求值器**不是定义处（依赖反转，同 ledger 缝）。
- **一种活一个标签，不是一种活一个存储**：`ModelTag` 答的是「哪个端点、哪个模型接这类活」，而这件事已有一套机制——人登记一个 endpoint，再为一个标签选一个模型。因此转写进的是 `Transcribe` 这个 variant，而不是第二张表单与第二个凭据入口；多一个存储就是给同一个问题立第二个答案。`ALL` 是界面枚举标签时走的那条路，新增一个 variant 即改它的长度。
- conformance 两断言，都用调用方递进来的良性请求 `benign`：①连调两次都返回（Ok 或带码的 Err），不 panic；②Err 后适配器不中毒（再调仍得应答）。确定性不入 conformance（真 model 非确定），剑本适配器的确定性由 citysim 自证。

**思考记录与思考强度**（思考记录原样保留，消息往返恒按 provider 官方规定处理）

```rust
pub enum Effort { None, Low, Medium, High, XHigh, Max }   // 全序；Ord 按声明序
pub fn content_from_message(message: &Payload) -> Result<Vec<ContentBlock>, AxError>;  // 契约变更，见下
```

- **两个思考块，逐字保留**。provider 官方规定：「During tool use, you must pass thinking blocks back to the API for the last assistant message. Include the complete unmodified block back」；改动即 400 `invalid_request_error`，报文为「`thinking` or `redacted_thinking` blocks in the latest assistant message cannot be modified」。故 canonical 侧两个变体缺一不可，字段名与线上同名（`thinking`／`signature`／`data`），使翻译无重命名、使 Ledger 载荷可直接对照官方文档校读。`signature` 是「an encrypted copy of the full reasoning」，由 provider 验签，城内恒不解析、不截断、不重排。
- **为何不是「可选保留」**：两条独立理由各自足以定案。其一，丢弃即违约（上一条）。其二，`message_payload` 是 `model_returned` 载荷的唯一成形处，脱机重建窗口靠它；入账前剥掉思考块，重建出的窗口就是一个从未发送过的窗口——那是判负条件三（历史失真），而它一旦成立，本设计的一切保证同时作废。**第三条理由是缓存**：改写助手消息即换缓存前缀。
- **`content_from_message` 返回 `Result<Vec<ContentBlock>, AxError>`**：`content` 键缺席折为空块表（脚本载荷的行为）；`content` 键在场就必须解出，否则 `E_WIRE_MISMATCH`。静默折为空会让一个未知块类型把整条助手消息从窗口里抹掉，而 Ledger 里它还在。
- **`Effort` 六级**：两家实际在用的就是 `none/low/medium/high/xhigh/max`，不另列其他方案。一处差别写清楚：**Anthropic 的 `effort` 只收五级**（官方 SDK 类型 `Literal["low","medium","high","xhigh","max"]`），`none` 不是它的取值，关思考在另一个字段 `thinking:{type:"disabled"}`；官方另记「Setting `effort` to `"high"` produces exactly the same behavior as omitting the `effort` parameter entirely」。OpenAI 侧六级同名（其 `minimal` 属 gpt-5 旧拼写，不入城内梯子）。故**两种兼容格式都拼得出全部六级**，否决「兼容格式拼不出就拒」这条路径；dialect 里只留 fail-closed 通配臂，含义改为「日后新增的级别尚未教会写」，恒不夹取到邻级。
- **不建每模型强度支持表**：任何 provider API 都不返回「本模型支持哪几级」。造一张我们填不满的表，就是给 provider 的真实行为立第二个权威；模型自己拒的原样透出。
- **`max_tokens` 是模型的事实，不是调用方的偏好**：Anthropic 要求每请求必带 `max_tokens`，且开思考时它是「思考＋回答」的总上限；OpenAI 则可缺席。两家的 `GET /v1/models` 都不返回该上限，所以它探不到，只能随模型登记。权威定在 `gateway::market::ModelEntry.max_output_tokens`，`CallShape.max_tokens` 由选型点从那一行解出；**任何调用处手写数字即错**——截断会发生在一个账上找不到理由的地方。
- **没人登记过的上限，载为「没人登记过」**：`Ceiling` 包 `NonZeroU64`，`ChatRequest.max_tokens` 是 `Option<Ceiling>`，于是「零」在类型上不存在，「缺席」也不等于零。缺席时 OpenAI 形不写该字段、取供应方自己的默认；Anthropic 形写不出请求，于是**拒**（`E_CONFIG_INVALID`，恢复语指向模型登记处），绝不在兼容格式那一层现编一个数。理由是实测：一个目录不认识的模型曾以 `max_tokens: 0` 上线，供应方答空、`stop` 记 `end_turn`、那次 run 冻结为「做完了」——**一个零上限造出的是一条看起来完成了的假历史**。

**模型看得见图**（`kernel::model::image`；形状 2 value）

```rust
#[derive(Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum ImageType { Png, Jpeg, Webp, Gif }        // 封闭枚举：城内认得的四种图
impl ImageType { pub fn mime(&self) -> &'static str; }   // "image/png" 等，两条 wire 共用
pub struct ImageRef { pub locator: Locator, pub media_type: ImageType,
                      pub width: u32, pub height: u32 }
pub enum ContentBlock { /* …既有五变体… */
    Image(ImageRef),
    ToolResult { tool_use_id, content, is_error, #[serde(default)] attachments: Vec<ImageRef> } }
```

- **账上存 locator 与整数尺寸，恒不存字节**。`ImageRef.locator` 是一条 `cas:` Locator，字节住 `storage::cas`；Ledger 载荷因而仍只有整数与短字符串，浮点禁令与载荷体量规则两条同时成立。宽高用 `u32` 而非比例：像素数是整数事实。
- **`ImageRef` 只定义一次**。Image 块与 ToolResult 的 `attachments` 是同一个值——四个字段一字不差——所以块写成 `Image(ImageRef)`（serde 内部标签，线上仍是 `{"kind":"image", "locator":…, "media_type":…, "width":…, "height":…}` 的扁平形），而不是把四个字段抄两遍。抄两遍就是一个概念两个权威，日后加一个字段要改两处。
- **`attachments` 带 `#[serde(default)]`**：既有 Ledger 里每一条 `tool_result` 都没有这个键，缺席即空表，所以每一份历史照旧重放。这是「只加不改」在 wire 面的具体形式。
- **`ImageType` 封闭而非 `non_exhaustive`**：它不是 provider 送来的开放词汇，而是城内决定收哪几种图；封闭枚举让 `mime()` 的 `match` 穷尽，加一种图必须同时回答「它的 MIME 是什么」。
- **conformance 增一条**：良性请求之外再发一次「含一个 Image 块」的请求，适配器同样必须返回而不是 panic。剧本模型（citysim）据此照旧通过——它不解释块，只按剧本作答。
-/

/-!
### 模型端口多一扇门：说到一半的话

```rust
pub type Increments<'a> = &'a mut dyn FnMut(&Increment);   // Increment 见 8-53

pub trait Model {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError>;
    fn call_streaming(&mut self, req: &ModelRequest, onto: Increments<'_>)
        -> Result<ModelReturn, AxError> { self.call(req) }
}
```

**默认实现是必要前提。** 它让「没有流的适配器」成为诚实的而不是坏的：调用方在同一时刻拿到同一个 `ModelReturn`，只是没看到任何增量。citysim 的脚本模型与离线重放不覆盖它。

**`Increments` 一个参数、无返回值，是刻意的。** 增量不是判断：下游任何东西都不得据它分支，而一个能拒绝的 sink 会让一个显示细节有能力弄失败一次调用。

**覆盖它的适配器欠同一个 `ModelReturn`，包括同样的失败。** 流被切断是一次读取错误，永远不是一个变短的回答——`ModelReturn` 恒不由增量拼出来。写进账本的那句话只从 `ModelReturn` 来，一次，在调用结算之后。

### 模型端口第三扇门：提前交出的调用

```rust
pub type EarlyCalls<'a> = &'a mut dyn FnMut(&ToolCall);

fn call_speculating(&mut self, req: &ModelRequest, onto: Increments<'_>, early: EarlyCalls<'_>)
    -> Result<ModelReturn, AxError> { self.call_streaming(req, onto) }
```

**一个工具调用的块一结束它就是完整的，这扇门在那一刻把它交给调用方**，好让只读工具在模型还在生成时就开跑。交出的调用与结算后 `ModelReturn` 里那一条逐字段相等（gateway §8）。

**它与 `Increments` 分开，因为它是要据以执行的。** 增量只供人看，谁都不得据它分支；提前交出的调用恰恰要据以启动工具。并进同一个 sink 就是让「看的东西」变成「决定的东西」。

**提前交出的调用不是历史。** 账本仍只从结算后的 `ModelReturn` 记 `tool_called`；回答被截断或取消，这扇门返回失败，调用方把据提前交出的调用得出的结果一并丢弃。调用方能据它做什么由 `crates/runtime/spec/Turn/Speculation.lean` 定：只提前启动排在第一个写调用之前的只读调用，结果按调用位置缓存，结算后按发出顺序记账。

**默认实现落回 `call_streaming`、什么也不提前交出。** 这对没有流、或其兼容格式在结算前说不出一个调用何时完整的适配器是诚实的：调用方只是没有提前量，拿到的 `ModelReturn` 不变。落选的是给 `call_streaming` 加第三个参数：那会让每个适配器与每个调用点都改签名，而只有一个兼容格式说得出块何时结束。
-/

/-!
### 8-53 `kernel::Increment`：模型正在产出的一小块，以及它来自哪一路（形状 2 值）

```rust
pub enum Increment { Said(String), Thought(String) }
pub type Increments<'a> = &'a mut dyn FnMut(&Increment);
```

- **两路而不是一路**：散文是答案，推理是得到答案的过程。把两者并进一个缓冲区，对一个把大部分输出花在推理上的模型，等于把草稿当答案给人看。
- **哪一路是这一小块自己的一部分**，于是没有任何下游读者需要猜。`ContentBlock::Thinking` 是它结算之后的落点，两处说的是同一件事的两个阶段。
- **同一块里两路都有时散文优先**：没有供应方这样发；真发了，它是在同一瞬间既回答又推理，而人在等的是答案。
-/

/-!
### 8-40 两个闭集，因为默认值该有名字：`model::Mode` 与 `model::Window`

`model/` 下多两个文件，各收一个「没人说过」的编码。

**`model::Mode`**——`Chat｜Work`，wire 词 `chat｜work`，`as_str` 写、serde 读，一条遍历式断言钉住往返同词。`Mode::ALL` 的次序就是控件列出的次序，`Chat` 排第一，因为一个人在对话框里打的一句话首先是在说话，而不是在派一件活。它是运行策略 `RunPolicy` 的四个值之一（§8-77）；写什么、要什么证据、落不落地由另外三个值回答。

为什么定义在这里而不在 `runtime`：**与 `DialectKind` 同一条依赖倒置**——线上携带它，`runtime::mode` 求值它，而 wire 不依赖 runtime，runtime 也不依赖 wire。两个外层 crate 都要叫出这个名字，谁都不得指名对方，所以名字住在这里。本枚举只说**有哪几种**；每一种怎样介绍给模型、准落什么，仍旧只有 `runtime::mode` 一处回答。

**认不出的词在进程边界反序列化失败，不落成默认值**：读成默认值的词让拼错它的人得到一个他没要的 run 和零句反馈。闭集把那句反馈还给发送方，落在它还能改的地方。运行策略的另外三个枚举同此。

**`model::Window`**——非零 `u64` 新类型，`Window::new(0) == None`，线上是裸数字、缺席是 `null`，`0` 在反序列化处被拒。

与 `Ceiling` 同形而**不合并**：一个界定模型一次能读多少，一个界定它能写多少，两者互换后仍然能编译，所以它们是两个类型。这不是「相似文本各写一遍」——共用一个名字买到的是让调用处把输入上限传进输出上限的那一天。

**`DialectKind` 有三支**：`Anthropic｜OpenAi｜OpenAiResponses`。responses 面与 chat 面请求体不同、回复形状不同、流式事件名不同，是第三支笔而不是第二支笔的开关；折在一起就成了一个在每一步上分支的写入器，那正是「给一张脸改的东西够得到另一张脸」的形状。求值仍全在 `gateway::dialect`。
-/

/-!
### 8-77 运行策略：一次 run 在什么纪律下工作（`kernel::model::mode`，形状 2 值类型）

```rust
pub enum Mode { Chat, Work }                                        // "chat" | "work"
pub enum AdmissionRequirement { Standing, Tested, ContractKept, DoubleValidated }
                                                                    // "standing" | "tested" | "contract_kept" | "double_validated"
pub enum LandingPolicy { Ordinary, Experiment }                     // "ordinary" | "experiment"
pub struct RunPolicy {
    pub mode: Mode,
    pub write: WriteLimit,              // §8-78
    pub admit: AdmissionRequirement,
    pub landing: LandingPolicy,
}
impl RunPolicy {
    pub const fn of(mode: Mode) -> RunPolicy;   // write Full、admit Standing、landing Ordinary
}
impl Mode { pub const ALL: [Mode; 2]; pub const fn as_str(self) -> &'static str; }
impl AdmissionRequirement { pub const ALL: [AdmissionRequirement; 4]; pub const fn as_str(self) -> &'static str; }
impl LandingPolicy { pub const ALL: [LandingPolicy; 2]; pub const fn as_str(self) -> &'static str; }
```

- **四件事，四个值。** `Mode` 回答这次 run 是在交谈还是在干活；`WriteLimit` 回答它能改什么；`AdmissionRequirement` 回答它的产出要带什么证据才准合并；`LandingPolicy` 回答产出走常规的路，还是留在一棵试验的树里不落地。四者独立取值、可以任意组合：只读可新建的试验、要测试的交谈都拼得出来，而且都有确定的意思。它们总是一起走（线上的 `Dispatch`、账本的 `run_started`、装配层的派活），所以是一个值 `RunPolicy`。
- **`Standing` 是一个有名字的值，不是「没有要求」。** 它说的是「楼自己的规矩已经要的检查，不多加一项」：楼要评审，评审照旧；楼不要，就只有这次 run 自己的检查点。它不取消任何既有的强制校验。把它写成 `Option<AdmissionRequirement>` 的 `None`，读者会把缺席读成免检。
- **选了要求不等于拿到了证据。** `RunPolicy` 只记要求；证据由 `runtime::mode::admits` 在合并那一刻对照（`crates/runtime/Spec.lean` §8-54）。
- **`RunPolicy::of(mode)` 是「只选了 mode」的策略**：写入不额外收窄、准入按楼的规矩、常规落地。派活的入口里只有人能选另外三项；城自己派的活（计划、排程、来信、编辑器经 ACP 派来的活）都走 `of(Mode::Work)`，一次委派或敲门继承说话那一方的整份策略。
- **拼法只有一处**：每个枚举的 `as_str` 写出的词就是 serde 读的词，各有一条遍历式断言钉住往返同词；未知词在反序列化处即拒（§8-40）。
- **入账**：`RunStarted.policy: Option<RunPolicy>`（§8-4 记录围栏），由 `runtime::run::Charter::open` 写，`#[serde(default, skip_serializing_if = "Option::is_none")]`。旧账本里没有这个键，读作「这一行早于策略入账」；旧的六个 mode 词从未以类型化载荷进过账本，所以没有要映射的历史读法（D12）。
- 验收：`kernel::event::record::run` 的 `a_run_started_line_records_its_policy`（四个值写出、原样读回、键名与词都是本节的拼法）；`model::mode` 的往返断言。
-/

/-!
### 8-80 `ModelTag::Ocr`：一个能读图的模型的登记位（`kernel::model::wire`，形状 2 值类型）

```rust
pub enum ModelTag { Main, Digest, Transcribe, Ocr }   // 线上 "main" | "digest" | "transcribe" | "ocr"
```

- **一个标签，不是一个模型。** 二进制里不带任何模型（D18）：人接一个端点、为 `Ocr` 选一个能读图的模型，城的 OCR 工具（X6）按这一次选择调用它，与转写读 `Transcribe` 是同一个机制（`crates/wire/Spec.lean` §8-27）。没有选时（或选中的端点已经摘下、机密楼的端点不在运行这座城的机器上），这栋楼的 run 的工具表里没有 `ocr`，模型看不到一件用不了的工具；`gateway::Recogniser::absent()` 是这种城里的识别器，每次识别都答 `E_TOOL_UNAVAILABLE`。两条路都不回落到 `Main`——一个只会读字的模型被递上一张图，答的是它猜的东西。
- **先有登记位，调用方随 X6 来。** 这条与本枚举「有人问才长」的规矩相违，理由是线上形状：`ModelTag` 在线上，加一个值要进位，本批（`WIRE_V` 45）一次带上，X6 落地时不再为它另进一位（`crates/wire/Spec.lean` D1）。在 X6 落地之前，页面不画这一行（client/Spec.lean 的模型表照旧三行）。
- `ALL` 的顺序就是设置页给出它们的顺序，`Ocr` 在最后。
-/

/-! D36 没报的缓存数是「没报」，不是 0

**决定**：`ModelUsage` 的 `cache_read_tokens` 与 `cache_write_tokens` 是 `CacheCount`：`Reported(n)` 或 `Unreported`。兼容格式的解析器在 provider 没给这个字段（或给了 `null`）时记 `Unreported`；给了 `0` 记 `Reported(0)`。OpenAI chat 格式没有缓存写的位置，恒记 `Unreported`。账本行里 `Unreported` 不写这个键，旧行一律写着数，所以读旧行不变。按价单结算与上下文量表只做减法，经 `or_zero` 把没报读成没有缓存，结算口径不变；线上 `Used.cached`／`cache_write` 与页面经 `reported` 把没报读成「不知道」，页面写 lang.json 的「未知」字样而不是 0%。三个平台上记录、折叠、页面的行为相同，没有平台分支。

**理由**：一个从不报缓存的 provider 与一个报了「零命中」的 provider 对用户是两件不同的事：前者的命中率不知道，后者的命中率是 0。把没报记成 0，页面就把「不知道」说成「全没命中」，用户会去查一个不存在的缓存问题。

**被否**：①照旧记 0——就是上面的错；②在 `ModelUsage` 上另加一个「缓存报了没有」的布尔——读和写两个数时都得记得先看旗标，忘了就又读出 0，而枚举让没看旗标的读法写不出来；③`Option<Tokens>`——`None` 不说出它的意思，调用处会写成 `unwrap_or_default`。

**重开参数**：某个兼容格式报了缓存读却不报总输入，使「缓存部分不超过总输入」不再成立。
-/

/-! D6 定规：请求借用会话与工具表，断点是请求的注记

**决定**：`ChatRequest<'a>` 的 `messages` 与 `tools` 是 `Cow<'a, [_]>`；回合组请求时借用 `Conversation` 与 catalog 的工具表，不复制。消息断点从 `ChatMessage` 挪到请求上的 `MessageBreakpoint`，兼容格式经 `ChatRequest::carries_breakpoint(index)` 问某条消息是否带断点。要跨调用留住请求的地方（保温续约）持 `ModelRequest<'static>`，经 `ModelRequest::into_owned`（它调 `ChatRequest::into_owned`）自己付一次拷贝。

**理由**：请求若持有会话与工具表，每一回合就把整段会话与整张工具表各复制一次，会话越长复制越多，而请求活不过这一次调用。断点标在消息上时，标记就得先拿到一份可写的拷贝；标在请求上，借用才成立。计划只锚尾消息，所以一个两值枚举就够，也写不出越界的下标。

**被否**：①`Arc<Vec<ChatMessage>>` 共享——保温持有它时，下一次追加消息就要复制整段会话，只是把拷贝挪了地方；②`&'a [ChatMessage]` 纯借用——保温与测试构造的自有请求就写不出来；③在请求里记 `Option<usize>` 下标——能写出一个不存在的消息下标。

**重开参数**：某个兼容格式需要把断点放在非末条消息上。
-/

/-! D12 运行策略是四个值，一次以新形状入账

**决定**：`Mode` 收成 `Chat｜Work`；原来六个词各自携带的三件事拆成三个独立的值——写入限制 `WriteLimit`、准入证据要求 `AdmissionRequirement`、落地策略 `LandingPolicy`——与 `Mode` 合成 `RunPolicy`，以这个形状第一次写进 `run_started`（§8-77）。旧六词的映射：`plan_goal`、`up`、`sc`、`ud` 都是 `work`，后三者的证据要求依次是 `tested`、`contract_kept`、`double_validated`；`experiment` 是 `work` 加 `experiment` 落地；计划由固定的 SDD 工作流入口进入，不再是一个 mode。

**理由**：一个词同时说「能写什么」「要什么证据」「落不落地」，三件事就只能按六种固定搭配出现：「只读可新建的试验」拼不出来，「要测试的计划」也拼不出来。拆开之后每件事有一个值、一个判定点：写入限制在写门上判（§8-78），证据在合并时判，落地策略决定 run 写在哪棵树里。旧六词从未以类型化载荷进过账本（它们只出现在线上、状态工具的结果文本与提示字节里），所以第一次入账就用新形状，不需要一段只为把旧词翻成新值而存在的读法。

**被否**：①保留六词、加一个「只新建」旗标：写入限制与 mode 的组合仍是固定的，而布尔旗标不是穷尽枚举；②先以六词入账、再写一段有版本的历史读法：那段读法只为本批未推送的开发账本服务，没有别的读者；③证据要求用 `Option`：缺席会被读成免检，而缺省的意思是「按楼的规矩」，它值得一个名字。

**重开参数**：出现第三种落地方式（例如合并到别的分支）时，`LandingPolicy` 加一臂；出现一种 mode 需要自己的证据规则时，重议「mode 不参与准入」。
-/
