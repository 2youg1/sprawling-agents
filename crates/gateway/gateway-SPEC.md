# gateway-SPEC.md

> crate：`gateway`（lib，依赖 kernel）。本 SPEC 先于代码存在（十七节）。
> Stage 3 模块：dialect／endpoint／native／credential（内缝 Vault）／oauth_profiles／market／cost；`admission` 与 `fallback` 已按 H-02 删除（§8-6、§8-11）；router 属 P1 不在本版。
> 本 crate 覆盖的语义：模型路由；provider 客户端自写＋认证两半；Custody；市场快照与成本；model 缝；provider 侧准入。

## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `dialect` | 城内规范 Anthropic Messages 与 OpenAI Chat 的双向纯函数翻译；保断点位／工具形状／usage |
| `endpoint`＋`native` | 自写线格式 HTTP 客户端（reqwest blocking＋rustls）实现 `kernel::Model`；逐字段请求覆盖；native＝回环 OpenAI 兼容服务的固定形 |
| `credential`＋`oauth_profiles` | Custody 效果半（scan 命中→入 Vault→原位替换 SecretRef）；兑付（组请求末格 expose，credential_lent）；describe；持久性探测；OAuth 流程（代码）＋情报表（数据） |
| `market`＋`cost` | 模型目录快照＋钉版回滚；per-call 入账（权威计费额优先）。provider 侧并发上限曾住 `admission`，因零调用者删除，重新长出来的条件见 §8-6 |

## 2 验收标准

- dialect：golden（两 Dialect 各一请求一响应）＋proptest 往返（响应侧 wire→canonical→重渲染 wire 逐字段等值；请求侧断点位／工具形状／文本字节无失）。
- endpoint：对回环假 provider 服务的半流中断（SSE 截断→E_PROVIDER 且不产伪 ModelReturn）＋幂等重试（同 IdemKey 重发，对外恰一次效果——由调用方 dedup 看守，endpoint 自身无重试暗策略）。
- credential：A13 链——人交出的值进金库（`set`，名字由调用方给）→`secret_captured` 入账（无明文无哈希前缀，写者是装配层 `credentials::signing`）→配置只见 `secret:`→resolve 兑付产 `credential_lent`→`describe` 恒不返回值。**外来字节的扫描与就地替换不在本 crate**：那是 `runtime::redact`，本 crate 不留第二份。探测三态（跨重启／仅本次开机／仅本进程）可注入验证。
- 调优：一个端点从表单与从导入两条路径附着后，`Query::Config` 回答的超时与重试相同；未调过的端点读到的三个默认值来自 `EndpointTuning::DEFAULTS` 而非任何第二处。
- cost：权威计费额在场则恒胜价目推算；两源不一致时以权威为准并记差额；A20 的取材面（对账断言住 memory::attribution）。

## 3 假设与歧义

- **native 的 S3 形**＝指向回环地址的 OpenAI 兼容服务（llama.cpp/ollama 一类）的固定客户端：无凭证要求、禁非回环 base_url。本地引擎进程管理不属本 crate（P4 产品化再议）。
- **tokio 不引入**：turn 是同步函数面，endpoint 用 `reqwest::blocking`（内部自管运行时线程，不出接口）。B.7 的 tokio 行推迟到 S4 channels（首个真异步消费者），偏离已记（§13）。
- 线格式 JSON 允许浮点（temperature 等 provider 字段）：dialect 是翻译面不是判定路径；判定路径（cost／market 价目）恒整数。
- OAuth 活体流程不可在 CI 验证：流程状态机与请求构造以形状测试看守，端到端属人工清单。

## 4 现状分析

空壳 lib。无既有公开面（api-baseline 自此起算）。

## 5 权威信源

自写客户端的四条被逼理由与「流程是代码、情报是数据」；Custody 全节；model 缝（native／endpoint 两生产适配器）；Anthropic Messages API 与 OpenAI Chat Completions API 官方文档（线格式字段名以官方为准）；keyring crate 文档（平台凭证服务绑定）。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。**本 crate 同例**：同一类型的 inherent impl 住不同簇文件时基线为每块各记一行 `impl`（`Endpoint` 两行）；下游 `sprawling` 基线记 `gateway` 内定义位簇路径（如 `credential::custodian::Custodian`），公共拼写不变。

Dialect／Endpoint／Custody／Vault／SecretRef／Sealed／Retries／HeaderValue／market snapshot／UsdMicros／权威计费额（authoritative billed amount）。概念名英文原词；「兑付」＝resolve+expose 的合称。

## 7 模块边界

```
dialect（路由，纯函数）◀── endpoint／native（I/O 适配器，impl kernel::Model）
dialect ───────────────▶ anthropic／openai（各自一种线格式的两向翻译）
anthropic／openai ──────▶ mismatch（共用的读取器与拒词；单向，无环）
credential ──▶ 内缝 Vault（pub(crate) trait：keyring 生产适配器＋会话内存第二适配器）
oauth_profiles（数据面，零分支）◀── credential（流程消费情报）
market／cost：纯判定与数据面，被 endpoint 与 S3 回合层消费
```

**market**：`ModelEntry.max_output_tokens: Option<Ceiling>`——一次回答能吐多少字是**模型的事实**，不是调用处的选择；探测接口不返回它，故它随模型登记入目录行。**这一列为空时由谁来答，现在写在 §8-17 的事实梯里，而不再是各兼容格式各自的默认**：`openai` 不写 `max_tokens`、`anthropic` 写不出请求于是拒（`E_CONFIG_INVALID`），是同一个缺口在两条线上的两种后果，而登记时梯子已经把它补上。尚未登记的本地模型沿用 `local` 行的保守上限，登记面（§8-9）接管后改为人确认过的行。

**router**：本 crate 持有 Endpoint 簿——它是**值不是库**，从 Ledger 重建（同 `kernel::registry` 的口径）；本 crate 仍不持 Ledger 句柄，写入由装配层做。

**不做什么**：不做 duty pool 与三轴定档（多 Agent 功能未成形之前，职责池是给一个还不存在的消费者建权威；现为标签选择，见 §8-9）；不持有 Ledger 句柄（事件载荷由调用方入账，本 crate 只产载荷值）；不缓存已解封凭证（每次操作解析一次）；不实现通用 provider 抽象层（被明拒）；dialect 不开缝（纯函数不 trait 化）。

## 8 接口先行（按模块分章）

### 8-1 gateway::dialect（形状 1 判定函数族）＋anthropic／openai／mismatch

城内规范会话类型住 `kernel::model`（缝上类型，S3 只加）；本模块只做 canonical↔wire 翻译，纯函数、无 I/O、无状态。

**一个文件里两家 provider 的字段知识交错排列，现在各住各家。** 1,280 → 512＋352（anthropic）＋393（openai）＋88（mismatch）。
切缝不是行数而是**变化的理由**：一家 provider 改了它的形状，只有它那一个文件动；而本模块顶上那句「改之前先读 provider 自己的文档」只有跟它指的那堆字段同居一处才真的被读到，所以两张文档链接表各自跟着它的 dialect 走。
- `dialect` 剩五个入口，每个一条 `match kind`，封闭集为二；不认得的 dialect 恒拒而不拿较近的那一家近似。跨 dialect 的断言（两向往返、两种强度拼写、float 拒收）留在这里，因为它们测的就是路由的契约。
- `mismatch` 是两家共用的四个读取器（`require`／`as_str`／`tokens_or_zero`／`payload_from`）与三句拒词（`mismatch`／`stream_cut`／`unspelled_effort`）。**依赖是单向的**：dialect → 两家 → mismatch，谁都不回头指。
**两家各自再切分一次，把流的拼接搬出去。** 图的翻译把 `openai.rs` 顶到 443 行、`anthropic.rs` 顶到 398 行，而文件上限是 400。切的仍是变化的理由：`increment_of`／`settled` 回答的是「一串 SSE 帧如何合成一份完整答案」，与「一个请求如何写上线」是两件事，且两家的流帧形状各自变。归 `anthropic/stream.rs`（110）与 `openai/stream.rs`（116），父文件各以一行 `pub(crate) use stream::{increment_of, settled};` 保住路径，`dialect` 一字未改。

- 只属一家的东西跟着它：`empty_answer`（空答案）、`joined_text`与 `effort_field` 入 openai；`role_str`、`stop_from`／`stop_str`、`block_wire`／`block_from`、`effort_fields` 入 anthropic。

**每一条形状不匹配都带出路，而空答案不再被当成形状不匹配**。真机派活时拿到 `E_WIRE_MISMATCH: translate wire on response.choices: expected array`，**`recovery` 为空串**——一条让人无从下手的拒绝，而 `AxError` 的契约写着 `recovery` 必须是可直接执行的信息。两处修正：

- `mismatch()` 统一携一句出路（对一句 dialect 选错与 base url 写错）。这是 25 个调用点共用的那一句。
- **`choices` 为 `null` 不是形状问题**，单独报 `E_PROVIDER`：信封是对的、字段都在、只是答案被丢了。实测来源：一家 OpenAI 兼容的托管端点在 `max_tokens` 高于所选模型的上限时，**既不拒也不答**，回 HTTP 200 携 `"choices": null` 与全零 usage。同一家端点上限因模型而异（实测：一个模型在 1024 与 2048 之间就翻，另一个 8192 仍然正常），故**恒不把某个上限写进代码**：那是对侧的数字，写下来就是第二个会漂的权威。拒词只指向该改的那一项（max output tokens），并标 `retriable`。

```rust
pub enum DialectKind { Anthropic, OpenAi }
pub fn request_wire(kind: DialectKind, req: &ChatRequest) -> Result<serde_json::Value, AxError>;
pub fn response_from_wire(kind: DialectKind, wire: &serde_json::Value) -> Result<ChatResponse, AxError>;
pub fn response_wire(kind: DialectKind, resp: &ChatResponse) -> Result<serde_json::Value, AxError>;
                                    // 响应侧双向：往返性质可测（wire→canonical→wire 等值）；重放剧本也要它造假响应
```

- **保三样**：①断点位——canonical 的 `cache: true` 标记翻到 Anthropic 侧＝`cache_control{type:"ephemeral"}`，逐块原位；OpenAI 侧无显式断点（供应商缓存是隐式前缀匹配），翻译**记录性丢弃**（文档声明，不静默）；②工具形状——`ToolDef{name, description, input_schema}`↔Anthropic `tools[]`／OpenAI `tools[{type:"function",function:{…}}]`，逐字段；tool_use↔tool_calls（id/name/args 无失）；③usage——Anthropic `usage{input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens}`／OpenAI `usage{prompt_tokens,completion_tokens,prompt_tokens_details.cached_tokens}`→`ModelUsage` 四整数字段，缺失字段取 0。
- 未知 wire 字段：请求侧不产（我们只写自己声明的字段＋overrides）；响应侧忽略未知键、缺必需键报 `E_WIRE_MISMATCH`（subject 写键路径）。
- canonical 枚举（Role／StopReason／ContentBlock）是闭的，本模块对每一支写出真实的臂；新增一支即在两种兼容格式里同时编译失败，这正是要的（G-22）。原先的 fail-closed 通配臂连同 `mismatch::unspelled_effort` 一并删除：它们恒不可达；wire JSON 键序＝serde_json BTreeMap 字典序（确定性，对端语义无关）。
- `E_ENDPOINT_DIALECT_UNSUPPORTED`：DialectKind 之外的兼容格式请求（wire 探查失败）；本模块两码之外不新增。

**思考块与思考强度的两侧翻译**

保的第四样：**思考块。**Anthropic 侧两向逐字，`thinking`（携 `signature`）与 `redacted_thinking`（携 `data`）各自原位往返；canonical→wire→canonical 与 wire→canonical→wire 两条往返均逐字节相等。理由是 provider 官方规定而非我们的偏好：改动即 400，报文指名这两个块 cannot be modified（kernel-SPEC §8-24 引原文）。

OpenAI 侧无对应物：出向翻译**记录性丢弃**思考块（同断点位的现行口径：文档声明，不静默），因为 Chat Completions 拼不出该形状且它也不要求回传；canonical 记录不受影响，重放仍能从 canonical 重推出当时实发字节（dialect 是纯函数）。入向则相反：OpenAI 回的推理摘要不伪造成 `Thinking`（它没有可回传的 signature）。

强度映射表（`ChatRequest.effort`，缺席即不写字段）：

| `Effort` | Anthropic wire | OpenAI wire |
|---|---|---|
| 缺席（`Option::None`） | 不写（等价于 `high`，官方明言） | 不写 |
| `None` | `thinking:{type:"disabled"}` | `reasoning:{effort:"none"}` |
| `Low`／`Medium`／`High`／`XHigh`／`Max` | `effort:"…"` | `reasoning:{effort:"…"}` |

两种兼容格式都拼得出全部六级，**唯一差别是「不思考」写在哪个字段**：Anthropic 的 `effort` 只收五级，无 `none`。通配臂仍 fail-closed，但它现在只能被「日后新增且尚未教会写的级别」触发（同 `role_str`／`stop_str` 的既有习惯）。

**缓存后果写在这里，因为它是选型理由**：官方排错文档记明「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」。故强度住 `FrozenConfig`（kernel-SPEC §8-22），Run 内不可变；本模块只负责把已冻结的值翻上线。

**两条 wire 上的图**

```rust
// gateway::dialect::images（新文件；形状 2 value）
pub struct ImageBytes(BTreeMap<String, Vec<u8>>);   // 键＝Locator 的规范拼写
impl ImageBytes {
    pub fn insert(&mut self, at: &Locator, bytes: Vec<u8>);
    pub fn len(&self) -> usize;  pub fn is_empty(&self) -> bool;
    pub(crate) fn encoded(&self, at: &Locator) -> Result<String, AxError>;  // 标准 base64
}
pub fn request_wire(kind: DialectKind, req: &ChatRequest, images: &ImageBytes)
    -> Result<serde_json::Value, AxError>;
```

- **dialect 仍是数据的纯函数**。字节不在这里取：`ImageBytes` 是参数，不是一个 store 句柄。同一份 `(ChatRequest, ImageBytes)` 永远翻出同一串字节，重放因此仍能重推当时实发的请求。
- **为何不是 `BTreeMap<Locator, Vec<u8>>`**：`Locator` 不实现 `Ord`（它内含的 `Range` 也不），而给 `kernel::locator` 加一对 derive 是在另一个模块的文件上改公共面。改成以规范拼写为键的 newtype：接口更窄（三个方法），排序仍然只取决于内容，且「解不出字节」这条失败路径归值本身拿着，而不是散在两家 dialect 里。
- **位置**：`dialect/images.rs` 与 `mismatch` 同层——两家都读、谁都不回头指，所以依赖仍是单向的。
- **locator 解不出字节即 `E_WIRE_MISMATCH`**（fail-closed）：一张描述存在、字节不在的图，发上线就是一个模型看不见的空位。
- **Anthropic 侧**：Image 块→`{type:"image", source:{type:"base64", media_type:…, data:…}}`；带 `attachments` 的 tool_result 的 `content` 由字符串改写为块数组 `[{type:"text",text},{type:"image",…}…]`（无附件时仍写字符串，既有 golden 字节不变）。两处形状都是 provider 自己的：<https://platform.claude.com/docs/en/build-with-claude/vision>。
- **OpenAI 侧**：user 消息的 `content` 由字符串改写为部件数组 `[{type:"text",text},{type:"image_url",image_url:{url:"data:<mime>;base64,…"}}]`。
- **本兼容格式多一项记录性丢失：tool 消息拿不了图**。Chat Completions 的 `role:"tool"` 只收字符串 `content`，所以 tool_result 的 `attachments` 不随它走，而是落到紧跟其后的一条 user 消息里。图没丢，丢的是「这张图是那次工具调用的结果」这层归属；同断点位与思考块的现行口径，**文档声明，不静默**，写在 `openai.rs` 顶上那张丢失清单里并由测试钉住。
- **base64 依赖**：`base64 0.22` 本就在 `Cargo.lock` 的图里（reqwest／git2 一系已携），直接命名不向锁里添包；自己写一份编码器才是新权威。

### 8-2 gateway::endpoint（形状 4 适配器）

```rust
pub struct EndpointConfig {
    pub base_url: String,                       // 恒 https 或回环 http；尾斜线归一
    pub dialect: DialectKind,
    pub model: String,                          // provider 侧模型名
    pub max_tokens: u64,
    pub auth: AuthSpec,                         // 认证头模板；SecretRef 兑付在组请求末格
    pub extra_headers: Vec<(String, HeaderValue)>,    // 字面量或金库引用，见 §8-16
    pub overrides: Vec<(String, serde_json::Value)>,  // 逐字段请求覆盖：JSON Pointer→值，最后应用
    pub timeout_ms: u64,                        // 一次已结请求的总期限
    pub stream_idle_timeout_ms: Option<u64>,    // 一次沉默的上限，缺席取 timeout_ms
}
pub enum AuthSpec { Bearer(SecretRef), Header { name: String, value: SecretRef }, None }
pub struct Endpoint { /* config、reqwest::blocking::Client、redemption —— 三字段 pub(super)，出了 endpoint/ 就取不到 */ }
impl Endpoint {
    pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError>;
    pub(crate) fn model(&self) -> &str;                       // 唯一对外可读的配置项：一个名字
    pub(crate) fn post_bytes(&self, content_type: &str, body: Vec<u8>) -> Result<Value, AxError>;
}
impl Endpoint { pub fn new(config: EndpointConfig, resolver: /* 兑付闭包，由 credential 提供 */) -> Result<Endpoint, AxError>; }
impl kernel::Model for Endpoint { /* call：ChatRequest（req.chat）→dialect→HTTP→ChatResponse→ModelReturn */ }
```

- **组请求五步**：canonical→`request_wire`→逐条应用 overrides（JSON Pointer，后者胜）→认证头兑付（`resolver` 取 `Sealed`，`expose()` 只在写头那一格，写完即 drop 零化）→POST。响应四步：状态码判定（429/5xx→E_PROVIDER 携 retry 语义；4xx→E_PROVIDER 携 provider 错误体摘要）→`response_from_wire`→usage 抽取→`ModelReturn`。
- **半流中断**：SSE 流截断（连接断／不完整事件）＝`E_PROVIDER`，恒不产部分 ModelReturn；S3 先落非流式全量路径，流式属只加（接口不变，config 增 `stream: bool` 字段即可）。
- **无暗重试**：重试是 watchdog 的决策（上限的唯一表示是 `Retries`，§8-16），endpoint 一次调用恰一次 HTTP 往返；幂等由调用方 IdemKey dedup 看守。
- base_url＝完整端点 URL（逐字段哲学，不拼路径）；EndpointConfig 增 pricing: Option<ModelEntry>（结算在适配器内以便 ModelReturn 携 billed 入账；权威额线上无标准槽位，现行恒 PriceSheet 源）；reqwest 0.13 的 rustls feature 名＝`rustls`（非 0.12 的 rustls-tls）；非流式先行，半流中断以截断 body 实测（E_PROVIDER，恒不产部分 ModelReturn）；kernel::ModelReturn 增 usage/stop/billed 三字段＋bare()/from_response() 两构造面（kernel-SPEC §8-24 同集）。
- `.expose(` 白名单（xtask secret）：`endpoint/call.rs` 与 `native.rs` 是 gateway 侧仅有的两个合法出现点（另加 `credential/oauth/flow.rs` 的刷新）。**两种凭证在同一句里写上线**：`authorize` 既写注册带的那条，也写人在自定义头里放的 `HeaderValue::Redeemed`；三个请求写入点（`call`、`stream`、`list_models`）都只调它，谁都不再自己遍历 `extra_headers`。
- **越过接口读字段的那一处收回来了**（H-08，叶子 8.8）。`Endpoint` 的 `config`／`client`／`redemption` 由 `pub(crate)` 降为 `pub(super)`，出了 `endpoint/` 取不到；`transcribe` 改走 `post_bytes` 与 `model()`。于是「一次 POST 如何发出、非 2xx 如何变成 `AxError`、对侧正文如何不被回显」在本 crate 里只有一份答案。转写面仍保留它自己那句恢复语（`rewrite_recovery`）：线上出了什么事是端点的事，人接下来能做什么是设施的事。

**图的兑付面与两句拒绝**

两型一构造面住新文件 `endpoint/redemption.rs`（`config.rs` 加完为 457 行，越了 400 行上限；切的理由不是行数而是职责：「一个端点在线上兑什么」与「一个端点配成什么样」各自变化）：

```rust
pub type ImageResolver = Arc<dyn Fn(&Locator) -> Result<Vec<u8>, AxError> + Send + Sync>;
pub struct Redemption { /* secrets: SecretResolver、images: ImageResolver —— 私有 */ }
impl Redemption {
    pub fn new(secrets: SecretResolver, images: ImageResolver) -> Redemption;
    pub fn without_images(secrets: SecretResolver) -> Redemption;   // 探针用：每一张图都拒
}
impl Endpoint { pub fn new(config: EndpointConfig, redemption: Redemption) -> Result<Endpoint, AxError>; }
```

- **两个兑付闭包总是同行，所以它们是一个值**（`Redemption`），而不是构造子上多出来的第二个参数；`adapter_for` 的参数个数因此不变。探针用 `without_images`：一个只问「你服务哪些模型」的调用本就不该有图，它拿到图就是一个 bug，而不是一张要传的图。
- **`wire_request` 三件事**：收齐本次请求的 Image 块与 tool_result 附件 → 数量超 `IMAGES_PER_TURN` 即 `E_INVALID_ARGS`（三段式，recovery 报出上限数）→ 逐张 `images(locator)` 取字节，单张超 `IMAGE_MAX_BYTES` 同样 `E_INVALID_ARGS`，然后交给 `request_wire`。上限住 `kernel::consts_policy`，不在这里手写。
- **看不见图的模型恒拒而不静默丢图**：`config.pricing`（即选型点定下的 `ModelEntry`）的 `input` 为 `Text` 而请求带图，则 `E_INVALID_ARGS`，recovery 指名 `text_image` 这个口径。拒绝住 `endpoint/model.rs`（`call` 与 `call_streaming` 两扇门共用一句），与 confidential 那句拒词同位同形：拒绝写在泄漏会发生的那一格，才能活过一次路由失误。

### 8-3 gateway::native（形状 4 适配器）

```rust
pub struct NativeConfig { pub base_url: String /* 恒回环 */, pub model: String, pub max_tokens: u64, pub timeout_ms: u64 }
pub struct Native { /* Endpoint 复用 —— 私有 */ }
impl Native { pub fn new(config: NativeConfig) -> Result<Native, AxError>; }   // 非回环 base_url→E_CONFIG_INVALID
impl kernel::Model for Native { /* 委托内部 Endpoint（OpenAi dialect、AuthSpec::None） */ }
```

- 本地推理恒走此路：S3 形＝回环 OpenAI 兼容服务的固定客户端；「恒回环」由构造子强制（fail-closed），出网面因此在类型上不存在。
- 不是 pass-through 豁免：Native 的策略＝回环强制＋无凭证＋兼容格式固定，三条都是 Endpoint 不持有的判定。

### 8-4 gateway::credential（形状 4＋内缝 Vault）

```rust
pub(crate) trait Vault {                        // 内缝：两句话接口
    fn put(&mut self, reference: &SecretRef, value: Sealed<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Sealed<String>>, AxError>;
    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError>;   // 探针与轮换用；不出对外接口
}
pub enum Persistence { AcrossReboots, ThisBoot, ThisProcess }
pub struct Described { pub configured: bool, pub source: String, pub persistence: Persistence, pub writable: bool }

pub struct Custodian { /* backend: Box<dyn Vault>、source 名、persistence —— 私有 */ }
impl Custodian {
    /// Startup probe: write-read-delete on each candidate backend, first
    /// pass wins; all failed -> session-memory fallback + provider_degraded
    /// payload returned to the caller for ledger append.
    pub fn probe() -> (Custodian, Option<Payload>);
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
                                    // 遮蔽即拒；空值即未配置；入参取 Zeroizing 非 Sealed：
                                    // `.expose(` 白名单恒三文件（定义处＋两解封点），Custody 是库不是 sink——
                                    // 持 Sealed 者恒密封直至线上；S4 PutSecret 在自己边界内转 Zeroizing。
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError>;   // 未命中→E_CREDENTIAL_MISSING；恒不跨操作缓存
    pub fn describe(&self, reference: &SecretRef) -> Described;                        // 恒不返回值
}
```

- 生产适配器＝keyring crate（Windows Credential Manager／macOS Keychain／Linux 内核 keyring）；第二适配器＝会话内存 BTreeMap（探测全败的兜底＋测试面）。**恒不自写加密文件**。
- **Linux 存在内核 keyring，不存在 secret service，理由是发布产物。** 发布的 Linux 归档是一份静态 musl 二进制（`x86_64-unknown-linux-musl`），而 secret service 走 D-Bus，树因此另到 `libdbus-sys`，那要求链接宿主的 glibc D-Bus——一份静态二进制与一个 D-Bus 凭据库不能同时为真。故 keyring 的 Linux 特性取 `linux-native`（keyutils，纯 syscall，不需要会话总线、不需要动态库、容器里同样成立），不取 `sync-secret-service`。代价写在类型上而不是写在注释里：内核 keyring 是内存，重启即清，故 `KeyringVault::PERSISTENCE` 在 Linux 上恒为 `Persistence::ThisBoot`，`SOURCE` 恒为 `kernel-keyring`。
- **等级只说一次，两处读它。** `Persistence::consequence()` 是「这个等级让人付出什么」的唯一权威句；`describe` 报状态（`source`＋`persistence`），`resolve` 未命中时把这句接在 recovery 后面——重启吃掉的 key 因此读作「重启清了内核 keyring，请再输一次」，而不是读作「你从来没配过」。探测成功不产 `provider_degraded`：Linux 上那是健康路径，每次启动报一条假警报只会让这个 kind 没人再读。
- realm/name 由调用方给定，本模块不生成名字：订阅线走 `credentials::subscription` 的逐供应方定名，API key 走 S4 命令面的 `PutSecret`。**恒无计数器命名**——按捕获次序编号会让同一个值每场会话换一个名、两个值跨会话撞同一个名，于是旧账本里的引用兑到别人的凭据。一个值的短名若要由内容导出，口径只有 `runtime::redact::fingerprint` 一处。
- OAuth 两流程（PKCE／设备码）＝代码：`pub fn oauth_begin(profile, …) -> RedirectPending` 与 `pub fn device_login_begin(profile, now_ms, timeout_ms) -> OauthPending`（§8-22），兑付分别是 `oauth_redeem` 与 `DeviceLogin::ask`。两条路的第二步都由 S4 命令面驱动，恒不在本 crate 里取时钟、取随机或替人等待。续期＝到期前 resolve 触发 refresh 构造。
- 环境变量是只读来源（键形 `SPRAWLING_SECRET_<REALM>_<NAME>`）：`describe.writable=false`；`set` 撞遮蔽即拒并指名遮蔽者；读取器可注入（edition 2024 的 set_var 不安全，测试恒不改进程环境）。
- A13 值正确性经真 Endpoint＋回环假服务在线断言（set→vault→resolve→写头，服务侧见原值）——credential 自身零 `.expose(`；PKCE 以 RFC 7636 Appendix B 向量钉实（S256，sha2 为外部协议事实非第二哈希权威）；base64url／percent-encode 自写纯函数；probe() 对真平台服务的验证属装配期人工清单（测试不擅动开发者凭证库）。

### 8-5 gateway::oauth_profiles（形状 6 数据面）

```rust
pub enum Grant {
    AuthorizationCode { redirect_uri: &'static str },        // RFC 6749 §4.1 ＋ RFC 7636 PKCE
    DeviceCode { authorization_endpoint: &'static str },     // RFC 8628
}
pub struct OauthProfile {
    pub family: Family,                     // 以哪家自己的客户端身份登录（§8-18 的那个枚举）
    pub provider: &'static str,             // 金库 realm 与线上词；一家厂商两个 harness 共用一个 realm
    pub api_base: &'static str,
    pub auth_endpoint: &'static str, pub token_endpoint: &'static str,
    pub scopes: &'static [&'static str], pub client_id: &'static str,
    pub grant: Grant,
    pub headers: &'static [(&'static str, &'static str)],
}
pub const OAUTH_PROFILES: [OauthProfile; 4] = [ /* 一家一行；只有数据零分支 */ ];
pub fn profile(provider: &str) -> Option<&'static OauthProfile>;   // 按 realm 词
pub fn profile_for(family: Family) -> Option<&'static OauthProfile>;
```

- 内容自 `docs/third-party.md` §1 逐行指名并每日监视的上游路径（跟情报不跟代码）；上游变更检测是周任务不是 CI 门。缺项留空串——宁缺毋错，在 `oauth_begin` 处 fail-closed。
- **四家一张表，键是 family**（叶子 14.3 / F-25 · §20.3 的定规：不 spawn、不 vendored、不装外部 CLI）。测试钉住「每个 `Family` 恰一行」与「每个 realm 恰一家」：少一行的那一家在界面上就是登不进去，两行共用一个 realm 就是把两个人的凭据存进同一格。
- **流程读 grant 而不假定 grant**：`oauth_begin` 对 `DeviceCode` 一行三段式拒并指名它答的那种授权，因为按重定向的路子给设备码流程拼一个 URL，是在向厂商要一个它从未承诺的回跳。
- **今天填到什么程度，逐行说清楚**（读于 2026-09-21，源见 `docs/third-party.md` §1 的监视路径）：
  - `Codex`／realm `openai`：issuer `https://auth.openai.com`、`/oauth/authorize`、`/oauth/token`、六个 scope、client id 与回环回跳 `http://localhost:1455/auth/callback` 均取自被监视的 `codex-rs/login/`（`server.rs` 与 上游登录管理器那个文件）。`api_base` 为 `https://chatgpt.com/backend-api/codex`，取自被监视的 `codex-rs/model-provider-info/`：订阅态的每一种 auth mode 都选这个 base，`https://api.openai.com/v1` 只服务 API key。这个 base 答的是 responses 面，而 responses 笔已在 §8-20 落地，故本行填实——**一次登录完成即 attach，attach 用哪支笔由 `Family::Codex` 说了算**（§8-18），不再由 realm 词二次推断。
  - `ClaudeCode`／realm `anthropic`：照旧，本城自己实现的那一份。
  - `GrokBuild`／realm `xai`：**设备码那条路已填实，浏览器回跳那条仍空着**。被监视的 `crates/codegen/xai-acp-lib` 的 `device_code.rs` 与它的配置模块陈述了 issuer、client id、十个 scope、`https://auth.x.ai/oauth2/device/code` 与 `https://auth.x.ai/oauth2/token`（读于 2026-09-21），这几个常量不经 discovery 即可驱动，故本行以 `Grant::DeviceCode` 填实。`auth_endpoint` 留空是因为浏览器回跳那条路的 issuer 与回跳地址由运行时读到的 OIDC discovery 文档陈述，钉在这里就是给那份文档造第二个家。`api_base` 是厂商文档上的 `https://api.x.ai/v1`。
  - `KimiCli`／realm `kimi`：client id、`https://auth.kimi.com/api/oauth/device_authorization`、`https://auth.kimi.com/api/oauth/token` 与设备码授权取自被监视的 `src/kimi_cli/auth/oauth.py`；`api_base` 为 `https://api.kimi.com/coding/v1`，取自 `platforms.py` 的 Kimi Code 一行——**不是** `api.moonshot.cn`／`.ai`，那两个是按 key 计费的开放平台而不是订阅面。
- **设备码登录已整条打通（叶子 14.3，本次落地），记法见 §8-22**；`xai` 与 `kimi` 两家因此与另外两家走同一条命令面，不再只能以 API key attach。**此处更正本节上一版记下的那条决定**：它要求 `channels::LoginStep` 增第三步，理由是「设备码登录的第二步人什么也粘不回来」。这条前提不成立——人手里有一样东西可以粘回来，就是城刚给他看的那个 user code。于是第二步不必新增：`Code { code }` 在设备码这家读作「我已在厂商页上同意，这是你给我看的那个码」，两种授权因此共用同一对命令步，客户端与 wire 一字未改。**没有跟着改的另一件要说清楚**：城不自行轮询，人按一次就问一次——在城的工作线程里替一个人等上半小时，会把整座城停在那里。RFC 8628 §3.5 的两条硬规则并没有因此松掉，它们由 `DeviceLogin` 在发送之前执行（见 §8-22）。**仍然缺的一件**：xAI 浏览器回跳那条路的 discovery 读取，它不改本表的形状。
- **兑付真的发出去（credential＋oauth_profiles）**：`pub fn oauth_redeem(profile, pending, code, timeout_ms) -> Result<OauthTokens, AxError>` 真正把 POST 发出去，`OauthTokens { access, refresh, expires_in_s }` 两个密文恒裹在 `Zeroizing` 里且 `Debug` 手写成 `<redacted>`——`Zeroizing` 自己的 `Debug` 会打印明文，派生一个就等于把活令牌交给第一条格式化它的 panic 信息。**发送住 credential 而不住 endpoint**：本模块就是整条 OAuth 流程，把「造请求」与「发请求」分到两个模块，就是让一次兑付有两个权威。**拒词恒不引用对侧正文**：令牌端点的错误页里可能带着刚用过的 code。`OauthProfile` 增 `api_base`（该 provider 的 API 根，登录完成后据它自动 attach；空串＝fail-closed，与空端点同口径）。
- **续期与兑付共用一次发送（credential）**：`oauth_refresh(profile, refresh: &Sealed<String>, timeout_ms)` 与兑付**共用一次发送**（`exchange::post_json`；设备码那条走同一模块的 `post_form`），故「拒词不引用对侧正文」只写一次、也只可能对一次。入参是 `Sealed<String>` 而不是 `&str`：明文只在**线前最后一格**出现，这与 endpoint／native 是同一类兑付点，故本文件同期进 `xtask secret` 的 expose 白名单——**放宽白名单而不是在调用点绕开它**，因为绕开的写法会让装配层持明文，而那正是这张名单存在的理由。
- **回跳带回的是 `code#state`，两半各有各的去处（在 `credential::oauth_redeem_request` 执行）**：`redeemed_code(pasted, pending)` 按 `#` 拆开人粘回来的那一行，左半是去兑付的 code，右半在场即必须与 `pending.state` 逐字节相等，不等即 `E_INVALID_ARGS`。不拆就把整行当 code 发出去，provider 回一个 `invalid_grant`；不核对 state 就等于本进程接受了一次它没有发起过的回跳，而 OAuth 2.0 要求发过 state 的客户端必须核对它。右半缺席按「人只复制了 code」处理而不拒——这是回跳落在人自己浏览器里时的常态。**两句拒词恒不回显粘贴内容**：那一行里带着一个活的授权码。
- **`state != code_verifier`（在 `credential::oauth_begin` 执行）**：两个值答的是不同的问题——verifier 证明「来兑的就是当初请求的那个客户端」，state 证明「这次回跳对应本进程发起的那次请求」。互用就是把一个证明做两遍、另一个一遍不做，且已有 provider 直接以 `400 invalid_grant` 拒。**在构造点拒而不交给接线的人记住**：这正是一份照上游抄来的实现会具有的形状。
- **迁出为独立 crate：未做，理由写在这里**。迁出的前提是一个**仓库外**的、持自有许可的上游存在；它今天不存在。在本仓库里建一个「另一个许可的目录」只会同时得到两件坏事：MPL 头门要么被改得认不出它、要么给它戟上一顶不属于它的帽子，而两者都不是迁出。因此只做两件真实可做的：表缺失时恒三段式拒（已在），以及 NOTICE 义务归 `xtask` 的 `release` 门。

### 8-6 gateway::admission —— 已删除（H-02，叶子 8.2）

本模块连同 `AdmissionState`／`AdmissionVerdict`／`ProviderOutcome` 与四个 `ADMISSION_*` 常量一并删除，`lib.rs` 的三行导出同删。

**判定依据**：它在生产路径上一个调用者也没有——全树只有 `lib.rs` 的重导出与 `fallback` 对它的引用，而 `fallback` 自身同样无人调用。一条没有调用者的缝不是缝，是一份将来会与真路径分叉的第二权威：它写着「并发上限 4、最小间隔 250 ms」，而线上每一次模型调用都不问它。

**它本该接在哪，写在这里，因为删除不等于这个问题没有了**：并发与最小发起间隔是对方计费的闸，要接就接在 `runtime::run::drive` 的每次模型调用前后（`admit` / `on_dispatch` / `on_outcome`），状态随端点名住 `RunWorker`。接的时候要一并修掉删除前就带着的缺陷：名额占满时 `admit` 返回的 `Hold { until: next_allowed_at }` 那个时刻可能已经过去，照它等待即忙等，所以 `AdmissionVerdict` 要把 `Saturated` 与 `Hold { until }` 分开。这份接线跨 `gateway`／`runtime`／`sprawling` 三个 crate，不在本片的文件边界内，故先删后接：git 里留着的实现比树上留着的空缝更诚实。

**代价**：429 不再加宽间隔，`provider_degraded` 事件由 `E_PROVIDER` 的 carrier 产出而不再由退避判定产出。

### 8-7 gateway::market（形状 6 数据面＋快照）

```rust
#[derive(Default)] #[serde(rename_all = "snake_case")]
pub enum InputKinds { #[default] Text, TextImage }      // 这个模型收得下什么
pub struct ModelEntry { pub id: String, pub context_tokens: u64, pub input: InputKinds,
                        pub input_price: UsdMicros /* per 1M tokens */, pub output_price: UsdMicros,
                        pub cache_read_price: UsdMicros, pub cache_write_price: UsdMicros }
pub struct MarketSnapshot { /* version: u32、entries: BTreeMap<String, ModelEntry> —— 私有 */ }
impl MarketSnapshot {
    pub fn builtin() -> Result<MarketSnapshot, AxError>;                 // 内置钉版目录（数据面）
    pub fn from_entries(version: u32, entries: Vec<ModelEntry>) -> Result<MarketSnapshot, AxError>;
    pub fn lookup(&self, id: &str) -> Option<&ModelEntry>;  pub fn version(&self) -> u32;
}
```

- **`input` 默认 `Text`，宽容读。** `selected_payload` 写 `input` 键，`read_choice` 读不到就当 `Text`——旧 Ledger 里的 `model_selected` 没有这个键，而重放一份旧历史不应该报错；默认取「只收文字」而非「收图」，因为猜错方向的代价不同：猜小了是一句拒绝，猜大了是 provider 的 400。内置目录里收图的行（现为 `claude-sonnet`）标 `TextImage`，`local` 保持 `Text`。
- 钉版回滚＝持前一快照即回滚（值语义，无 I/O）；快照落盘属 projection／config 面，本模块只管形与查询。价目恒整数微美元（判定路径禁浮点）。
- **`builtin()` 上抛 `from_entries` 的拒绝，不顶一份空目录。** 两行同 id 是编程错误，而把它顶成空目录的后果是：城里每一个模型都查不到价目行，拒词一个也不点名那张表。`Result` 让这件事在第一个调用点就说出来（`E_INVALID_ARGS`，主题为撞车的 id）。落选的是「rows 改 const 数组加一条去重测试」：那把不变量交给一条可以被删掉的测试，而类型能一直拿着它。

### 8-8 gateway::cost（形状 1 判定函数）

```rust
pub struct CallCost { pub billed: UsdMicros, pub source: CostSource, pub usage: ModelUsage }
pub enum CostSource { Authoritative, PriceSheet }
pub fn settle(usage: &ModelUsage, authoritative: Option<UsdMicros>, entry: &ModelEntry) -> Result<CallCost, AxError>;
```

- 权威计费额在场恒胜（`CostSource::Authoritative`）；缺席则按价目推算：`input×input_price/1M + output×output_price/1M + cache 两项`，全程 checked 整数（溢出→E_INVALID_ARGS 报「结算溢出」）。model_returned 载荷含 `billed_usd_micros`＋usage 四整数，A20 对账消费之。

### 8-9 gateway::router（形状 7 projection）

```rust
pub struct AttachedEndpoint { pub name, pub base_url, pub dialect: DialectKind,
                              pub auth: AuthSpec, pub models: Vec<String>,
                              pub probed: bool }   // 这份 models 是问出来的（true）还是人报的（false）
impl AttachedEndpoint {
    pub fn is_local(&self) -> bool;          // 与本地适配器同一依据（native::is_loopback）
    pub fn has_credential(&self) -> bool;    // 关于凭证，金库外只能回答这一问
    pub fn chat_url(&self) -> String;        // base_url ＋ 该兼容格式自己的路径
    pub fn models_url(&self) -> String;
}
pub struct EndpointBook { /* 私有：endpoints、chosen */ }
pub struct Chosen<'b> { pub endpoint: &'b AttachedEndpoint, pub entry: &'b ModelEntry }
impl EndpointBook {
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError>;
    pub fn apply_payload(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError>;
    pub fn select(&self, tag: ModelTag, policy: &BuildingPolicy) -> Result<Chosen<'_>, AxError>;
    pub fn endpoints(&self) -> impl Iterator<Item = &AttachedEndpoint>;
    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry)>;
}
pub fn attached_payload(&AttachedEndpoint) -> Result<Payload, AxError>;      // endpoint_attached 唯一成形处
pub fn selected_payload(ModelTag, &str, &ModelEntry, Option<CeilingSource>) -> Result<Payload, AxError>;  // model_selected 同上
```

- **一次取模型只答一问**：用哪个模型、在哪个端点上。「不答时怎么办」曾是第二问，随 §8-11 一并删除。

- **为何不是 duty pool**：多 Agent 功能未成形之前，职责池没有消费者，而没人读的权威只会漂。降为 `ModelTag` 两值枚举（`Main`／`Digest`）：**标签因为有人按它取模型而存在**，新增一个标签的前提是先有调用方。
- **两个入口一个读者**：`apply`（重建路径，手里是 record）与 `apply_payload`（写入路径，手里是刚要写的 payload）共用同一套载荷读取，于是「写者以为的」与「重建得到的」不可能分岔。
- **confidential 在选型点再守一次**：非回环 endpoint 对 confidential 楼恒拒（`E_GATE_DENIED`）。`gateway::endpoint` 的兜底拒同期改为**按本地性判定**（而非一律拒）：规则是「字节不出运行中的机器」，不是「不准用这个类型」；否则一个回环的 Anthropic 服务器会被误拒。
- **路径归兼容格式**：人输入 base URL（provider 文档就是那么印的），`messages`／`chat/completions`／`models` 由兼容格式拼。这与 `EndpointConfig.base_url`「完整端点 URL、不拼路径」并不矛盾：适配器保持字面，拼路径的是上层登记面。
- **`probed` 是这份 models 的来源，不是端点的健康度**：`true` ＝ `GET .../models` 答了，登记的 id 是对端自己说的；`false` ＝ 探测失败而人自己报了型号，城照登。载荷里缺 `probed` 键读作 `true`，于是本键之前写下的每一条 `endpoint_attached` 重放不变——**旧记录的含义没有改，改的是新记录能多说一句**。
- **`AuthSpec::for_dialect` 是凭证头的唯一产地**：`AuthSpec::for_dialect(dialect: DialectKind, reference: SecretRef, header: Option<String>) -> AuthSpec`，纯函数，住 `endpoint/auth.rs`。人显式填的头名恒胜（`Header`）；否则 Anthropic → `Header{name:"x-api-key"}`，OpenAI 及其余 → `Bearer`。**它不住 `endpoint/config.rs`，因为 config.rs 已 397 行、行数门限 400**：为了过门而把测试删短是拿门当对手，而「凭证头归兼容格式」本来就是一个可以自己站着的概念；`AuthSpec` 类型本体留在 config.rs，因为搬它会让同一个名字在 crate 内多出一条 `pub(crate) use` 路径。登记面（`bin::assembly::credentials::endpoints::endpoint_of`）不再自己在 Bearer 与具名头之间选，否则「Anthropic 用哪个头」在城里有两个权威，而漂开的总是没人看的那个。

### 8-10 gateway::endpoint 探测面

```rust
impl Endpoint { pub fn list_models(&self, url: &str) -> Result<Vec<String>, AxError>; }
```

两家一方都在 `GET .../models` 返回 `{"data":[{"id":..}]}`，**都不返回价目与 token 上限**——所以探测只取 id，两个 token 数字由人在登记时确认。探测与正式调用共用同一个兑付路径（`authorize`）：两套认证拼法就是两个权威，而漂开的总是没人看的那个。

**探测不是登记的前提**。上一段的「两家都返回」只对那两家为真：网关与 Anthropic 兼容格式的第三方多半根本不服务 `/models`，于是一条本可用的线路被一个它从未承诺过的接口挡在城外。新规则三行：探测成功→按对端报的 id 收窄（旧行为不动，`probed=true`）；探测失败且人报了型号→按人报的登记（`probed=false`，并按 `effect` 级写一条诊断，点名探测的错——**登记确实发生了，被拒的只是探测**，用 `refuse` 级会说成这次登记被门拒了，那是假话）；探测失败且人没报型号→仍然拒绝，恢复语改为「把要用的 model id 报上来，再登记一次」，因为此时城手里一个可调用的名字都没有。

**四个参照实现一致**：pi、codex、opencode、Claude Code 都让人**声明**模型清单，发现是可选的（Claude Code 默认关闭、3 秒超时、失败静默）。把可选的发现当成必选的准入，是本仓自己加的限制，不是外部事实。

**「路径归兼容格式」现在也管凭证头**：Anthropic 的 API key 走 `x-api-key`（`Authorization: Bearer` 只发给短时联邦令牌），OpenAI 兼容格式走 `Bearer`；人显式填的头名恒优先。见 §8-9 `AuthSpec::for_dialect`。落选的是「让登记页必填头名」：那把一个兼容格式自己就知道的事推给了人，而人填错的代价是一个 401。

**`adapter_for` 住 `endpoint/adapter.rs`**：装配线（哪个 chosen 走 Native、哪个走 Endpoint）读的只有 chosen 与赎回闭包，故它是自由函数而非 `RunWorker` 方法——挂在 worker 上等于说装配需要整座城。调用期限不再是这里的常量：它是 `EndpointTuning::DEFAULTS.timeout_ms`（§8-16），`adapter_for` 读 `call_timeout_ms()`——一个默认值该住在它所默认的那个设置旁边。凭据簇只出 `resolver`（赎回闭包是凭据的形状）与 `dialect_headers`（兼容格式要的头是兼容格式的事，搬家留待日后：`dialect_headers` 住凭据是历史位置，此处只动 adapter 线）。

### 8-11 gateway::fallback —— 已删除（H-02，叶子 8.2）

本模块连同 `Fallback`／`Retreat`／`retreat_payload`、`Chosen.fallback`、`Choice.fallback` 与 `selected_payload` 的 `fallback_endpoint`／`fallback_model` 两个键一并删除；`Settled` 随之消失，`selected_payload` 直接收 `ceiling_from: Option<CeilingSource>`。

**判定依据**：`Fallback::Then` 在生产里写不出来——线上没有设它的字段，装配层恒写 `Fallback::None`，`retreat` 与 `retreat_payload` 零调用者。于是一个「两臂的值」在生产里只有一臂，另一臂是给一个还不存在的设置面预留的权威。**旧记录仍读得回**：`read_choice` 对这两个键不再取值，与它对待任何本书不拥有的键同一口径，测试钉住「带着退避键的 `model_selected` 仍读成它所述的那次选择」。

**要重新长出来的条件**：设置面上真有人能为一个标签指定备用端点与备用模型，且 `runtime` 侧有一处在失败时读它。那一天它连同 §8-6 的退避一起回来，而不是单独回来——`MoveTo.not_before` 的唯一来源是退避阶梯。

### 8-12 gateway::transcribe（`transcriber` 形状 4 适配器，`recording` 形状 2 值，`wire` 形状 1 判定）

把一段录音变成一行字的那个 provider 端点。**它是可选设施**：没配的城照常跑完每一件事，只是在有人开口说话时明说自己听不见，而不是在兑付那一格炸开或回一个空串。

```rust
// gateway::transcribe（索引，无逻辑）
pub use chosen::transcriber_for;
pub use recording::{AudioType, Recording};
pub use transcriber::{Transcriber, TranscriberConfig};

// transcribe/chosen.rs（形状 1 装配，`adapter_for` 的孪生）
pub fn transcriber_for(chosen: &Chosen<'_>, secrets: SecretResolver)
    -> Result<Transcriber, AxError>;    // TRANSCRIBE_TIMEOUT_MS = 120_000

// transcribe/recording.rs（形状 2 值）
pub enum AudioType { Webm, Ogg, Mpeg, Mp4, Wav }
impl AudioType {
    pub fn media_type(self) -> &'static str;   pub fn file_name(self) -> &'static str;
    pub fn of_media_type(raw: &str) -> Result<AudioType, AxError>;  // 认不得的容器＝E_INVALID_ARGS
}
pub struct Recording { /* bytes、kind —— 私有 */ }
impl Recording {
    pub fn new(bytes: Vec<u8>, kind: AudioType) -> Result<Recording, AxError>;  // 空／越顶＝E_INVALID_ARGS
    pub fn kind(&self) -> AudioType;   pub fn len(&self) -> usize;
}

// transcribe/transcriber.rs（形状 4 适配器）
pub struct TranscriberConfig { pub base_url: String, pub model: String,
                               pub auth: AuthSpec, pub timeout_ms: u64 }
pub struct Transcriber { /* attached: Option<Endpoint> —— 私有 */ }
impl Transcriber {
    pub fn absent() -> Transcriber;                       // 这座城没有这项设施
    pub fn attach(config: TranscriberConfig, secrets: SecretResolver) -> Result<Transcriber, AxError>;
    pub fn is_attached(&self) -> bool;
    pub fn transcribe(&self, recording: &Recording) -> Result<String, AxError>;
}
```

**没配就是一句具名的拒绝，不是一个空串。** `Transcriber::absent()` 上的 `transcribe` 恒返回三段式 `E_TOOL_UNAVAILABLE`：action ＝ `transcribe a recording`，subject ＝ `this city has no transcription endpoint attached`，recovery 指出两条人能立刻做的路（登记一个服务 `audio/transcriptions` 的 endpoint，或者改用打字）。**为何复用 `E_TOOL_UNAVAILABLE` 而不新增一码**：基表里这一码的语义正是「这次部署里没有这项设施」，而 `E_CONFIG_INVALID` 会说成人填错了什么——什么都没填错，这项设施本就是可选的；`E_BROWSER_UNAVAILABLE` 那样的专码属于模型会调用的 tool，转写不是 tool 而是界面设施。码表是 kernel 全城权威且按「能否定义掉」逐码守着，为一件已有码能如实表达的事把它撑大，就是给同一个事实立第二个名字。

**凭据只有一条路，请求也只有一条。** `Transcriber` 内部持一个真的 `Endpoint`（`DialectKind::OpenAi`、`Redemption::without_images`、`pricing: None`），整次 POST 由 `Endpoint::post_bytes` 发（§8-2），认证头由 `Endpoint::authorize` 写——与聊天调用、与 `list_models` 探测是同一格兑付。头名由 `AuthSpec::for_dialect` 定（§8-9），登记面不再自己在 Bearer 与具名头之间选。**恒不为转写开第二个持凭据的地方**：两处持凭据就是两处会漏。

**哪个端点答这类活，由账本说了算，装配只有一处。** `transcriber_for` 与 `adapter_for` 是同一句话的两半：`EndpointBook::select` 给出 `Chosen`，这两个自由函数各自把那个选择变成一件可调用的设施。调用方自己拼一个 `TranscriberConfig`，就是给「base URL 与凭据在哪里合流」立第二个地点。**容器从 content-type 读而不从文件名猜**：`AudioType::of_media_type` 是那一步的唯一入口，认不得的容器当场拒。

**路径归兼容格式。** 人填 base URL（provider 文档就那么印），`audio/transcriptions` 由 `transcribe::wire` 拼，拼法复用 `router::attached::join`（该函数因此升为 `pub(crate)`）——「base URL 加上兼容格式自己的路径」在城里只有一个算法。

**多部分请求体自写，不引 reqwest 的 `multipart` feature。** 本 crate 自写线格式是既定选型（§8.5），而 `multipart/form-data` 只是两个字段加一条分界线；引 feature 要动根清单与 `Cargo.lock` 两个共享权威，换来的代码比自写的还多。分界线是**确定性的**：常量种子起头，只要它作为子串出现在录音里就加一个 `-` 再试，录音有限故必然终止；同一份 `(model, Recording)` 因此永远拼出同一串字节，重放能重推当时实发的请求。

```rust
// transcribe/wire.rs（形状 1 判定，纯函数）
pub(crate) struct FormBody { pub(crate) content_type: String, pub(crate) bytes: Vec<u8> }
pub(crate) fn form_body(model: &str, recording: &Recording) -> FormBody;
pub(crate) fn transcription_path() -> &'static str;                       // "audio/transcriptions"
pub(crate) fn transcription_of(wire: &serde_json::Value) -> Result<String, AxError>;
```

- 请求体两个字段：`model`（纯文本）与 `file`（`filename` 取自 `AudioType`，`Content-Type` 取自同一处——**扩展名与 media type 是同一个事实的两面，故住同一个枚举**，provider 两边都看）。`response_format` 不写：默认就是 `{"text": …}`，而写一个与默认相同的字段是给同一件事立第二个权威。
- 读答案只认一个键：`text` 缺席或非字符串＝`E_WIRE_MISMATCH`（走 `mismatch::require`／`as_str`，与两家 dialect 同一批读取器）；非 2xx＝`E_PROVIDER`，subject 只写 URL 与状态码，恒不回显对侧正文。
- **空转写是合法答案**：一段静音本来就该转出空串。空串只在「没有端点」那条路上被禁止，而那条路根本不返回 `Ok`。

**为何 `Recording` 是值而不是一对参数**：字节与它的格式永远同行，且两条不变量（非空、不超 `RECORDING_MAX_BYTES`）只在 `new` 一处守；无 setter。**为何 `wire` 与 `transcriber` 分家**：「这段多部分请求体长什么样」是纯数据的判定、可逐字节断言，「怎么把它发出去并兑付凭据」要一个 socket——两件事变化的理由不同。

**线上还没有这个动词。** 语音输入是前端功能而前端已冻结；本节只交付服务端半，故 `channels` 无新帧、`Command` 无新变体、wiring 门辖区不变。客户端要照着建的三件事是端点名、请求形与拒绝码。

## 8.5 两个设计（crate 级）

**A（选中）：canonical 会话类型住 kernel::model 缝上，dialect 只做翻译**——ScriptModel（citysim）与真适配器消费同一请求形，重放重建的入窗字节有唯一权威；代价是 kernel 公开面变大（约十个纯数据类型）。
**B（落选）：canonical 类型住 gateway，Model::call 只收哈希、真会话经旁道传递**——kernel 面最小，但旁道即第二权威：sim 与真适配器走不同请求形，dialect 往返与 A15 重建无从对同一对象断言；缝的意义（「同一批产品 crate，换适配器整城可跑」）被掏空。落选。
**endpoint 客户端选型**：reqwest::blocking（选中）vs 自写 hyper 直连 vs ureq。自写 hyper＝维护整个连接池与 TLS 面，收益为零（我们自写的是**线格式**不是传输层）；ureq 更小但 rustls 集成与代理面弱于 reqwest；blocking 而非 async＝与同步 turn 面同构，避免为一个 HTTP 调用引入全库 async 传染。

## 9 工作流程

回合层组 `ChatRequest`（prefix 四段＋窗口历史）→endpoint.call（dialect 翻译＋兑付＋HTTP）→cost.settle→model_returned 载荷（usage＋billed）→attribution（memory 侧）摊回。credential 独立线：启动 probe→set（S4 命令面或订阅登录写入）→resolve（组请求末格）。

## 10 实现逻辑

dialect 先行（纯函数零依赖，golden 钉形）→endpoint 骨架（假 provider 服务回环测试）→credential（Vault 两适配器＋探测）→market/cost（纯判定）。native 最后（复用 endpoint）。每步红先行：golden 未落前不写翻译分支。

## 11 边界枚举

空 messages（合法：首轮）；空 tools（不写 tools 键）；SSE 半流（S3 非流式先行，流式只加）；429 携 retry-after；usage 缺席（取 0，CostSource=PriceSheet）；权威计费额为 0（合法，免费档）；base_url 尾斜线；overrides 指向不存在的路径（创建）；OAuth profile 字段空串（oauth_begin fail-closed）；Vault 探测三候选全败（会话内存＋provider_degraded）；遮蔽写入；空串凭证（视同未配置）。

## 12 错误处理（逐码答「能否定义掉」）

- `E_PROVIDER`：不可定义掉——网络与对端是本 crate 的本质失败面；subject 写状态码与端点名，恒不含请求体。
- **「能否再试一次」只有一个家：`endpoint::config::ProviderFailure`**。调用点只说它看见了哪一种失败，retriable 与恢复语由该枚举一处给出，恒不在调用点第二次判定。往返未完成（send／execute／读体／读帧／静默超时）＝`Exchange`／`Cut`／`Silence`，标 `retriable`；对端已答而本城拒绝（非 2xx＝`Refused`，body 形状不可读＝`Unreadable`）与本侧 `.build()` 失败（`Unbuilt`，确定性重演同一失败）不标。此前十五处调用点各自造错、无一 opt-in，使默认 `Retries::UntilHalted` 实际等于零次重试：一次瞬时断连即静默废掉一个 handdown 子运行。
- `E_WIRE_MISMATCH`：不可定义掉——对端响应形状漂移是外部事实；subject 写键路径。
- `E_ENDPOINT_DIALECT_UNSUPPORTED`：不可定义掉——用户可配任意 external provider，兼容格式探查失败必须可报。
- `E_CREDENTIAL_MISSING`／`E_CONFIG_INVALID`：kernel 已有码，语义照 Custody 一节；不新增码。
- `E_SECRET_EGRESS`：本 crate 不产（出口扫描住 gate::egress 与 checkpoint 面）；endpoint 组请求不做二次扫描（Custody 在入口已换引用，纵深由门守）。

## 13 依赖选型

- `reqwest = { version = "0.13", default-features = false, features = ["blocking", "rustls-tls", "json"] }`——钉版表钉 rustls；blocking 理由见 §8.5。2026-08 复核：0.13.4 现行。
- `keyring = "3"`——平台凭证服务绑定（B.7「平台凭证服务绑定」行）；MIT/Apache。
- `secrecy`／`zeroize`：workspace 既钉。serde_json：wire 面。
- **tokio 不引**（偏离 B.7 引入期列，理由 §3；S4 channels 首个真异步消费者时引入）。

## 14 硬编码声明

`transcribe::recording::RECORDING_MAX_BYTES = 25 MiB`（§8-12：OpenAI 音频面自己印的上限，provider 侧工程参数，非城策口径，不入 `consts_policy`；改须本 SPEC 同集）与 `wire::BOUNDARY_SEED`（同上，分界线种子）；`EndpointTuning::DEFAULTS` 三个默认值（§8-16，端点调优默认值的唯一住处，改须本 SPEC 同集）；oauth_profiles 表（§8-5，数据面即定义处）；market 内置目录（`builtin()`，S3 收录城内实际使用的模型行，价目随 Stage 复核）。三者全 pub(crate) 数据面，改动须本 SPEC 同集变更。

## 15 影响面

kernel::model 增 canonical 会话类型（kernel-SPEC §8-24 同集改；specalign 若辖新枚举则表同集落）；runtime 回合层消费 ChatRequest；memory::attribution 消费 model_returned 新载荷字段；citysim ScriptModel 改收 ChatRequest（同一缝）。

## 16 测试与约束

golden：两 Dialect 各一请求一响应（insta）；proptest：响应往返、usage 保值、断点位保序；endpoint 对回环假服务的状态码矩阵（200/429/500/截断体）；credential：内存 Vault 全流程＋探测注入；admission 重演等值；cost 权威胜出＋溢出拒绝。约束：clippy 零告警；无 `unwrap`；`.expose(` 只在 endpoint.rs／native.rs。

## 17 模型体验

零字节：gateway 全体不产 prefix 字节。间接贡献：dialect 保断点位使 city-wide 缓存在真 provider 上成立（省的是每回合重付的 prefix 费）；cost/attribution 使「钱花在哪」可答而不占模型上下文（成本归因是给人看的）。

## 18 文档同步

ARCHITECTURE §6 gateway 表逐行状态翻转；§6 接线台账登记（endpoint/native 生产消费者＝S3 回合层与 assembly；credential 消费者＝endpoint＋S4 PutSecret）；kernel-SPEC §8-24 同集改（落 canonical 类型时）；api-baseline 含 gateway 起算（SPEC 既存，apisync 自动入集）。

### 逐字读一次调用（形状 3 适配器）

`Endpoint` 覆盖 `Model::call_streaming`：请求带 `stream: true`，逐行读 `data:`，把每一帧交给 `dialect::increment_of`，最后 `dialect::settled_from_stream` 把收集到的帧重装成**这个 dialect 非流式的那个形状**，再交给同一个 `response_from_wire`。

**「逐行」指的是响应体到达的节奏，而不是一个已经读完的字符串的行。** 先前的实现先 `response.text()` 把整个 body 读完，再在 `body.lines()` 的循环里逐条 `onto`——于是请求确实带了 `stream: true`、provider 确实分次答，而全部增量在模型已经停下之后的同一毫秒里一起发出。对一个真供应方的计时：`model_called` 在 1.9 s，25 条 delta 在 11.9 s 的同一毫秒，随后 `model_returned`。**改成把 `Response` 当 `std::io::Read` 包进 `BufReader` 逐行读**：一帧到达即交一帧。否决「把转发搬到 socket 任务那一层去查锁」：`to_watchers` 是非阻塞广播，帧压根没有到达那里，往下游找只会找到一个不存在的原因。**验收形式**：假供应方先写开头几帧并 flush，**然后等调用方回报「第一条增量已转出」才写剩下的**；读完再回放的实现永远回报不了，服务器自己就是断言，测试侧不读时钟。

**结算答案只有一个解析器。** 直接把流读成 `ChatResponse` 会立刻长出第二个权威：同一个回复，流式路径与阻塞路径可能得出两个结论。重装成非流式形状是这条口径的全部实现。

**`increment_of` 只认散文。** 各 dialect 各读各的：Anthropic 读 `delta.type == "text_delta"` 的 `delta.text`；OpenAI 读 `choices[0].delta.content`。**工具参数与 thinking 块一律不报**：半个工具参数不是短一点的工具参数，而 thinking 块是替 provider 转交签名用的、不是拿来发表的。它不返回 `Result`——一个读不出来的增量就是不显示的增量，一个显示细节不得有能力弄失败一次本来正常的调用。

**思考块的 signature 与文本走两条 delta，两条都要收。** Anthropic 把一个 thinking 块拆成 `thinking_delta`（正文）与 `signature_delta`（签名）两串增量，而 `content_block_start` 给出的那份 signature 恒为空串。`settled` 因此按 index 累积 signature 并在重装时写回，与 `partial_json` 同形。**空 signature 在 `block_from` 升为 `E_WIRE_MISMATCH`**：provider 拿签名去核验它自己发出的那段推理，空的那份带进下一回合就是一个 400，而这座城此刻还说不出为什么——拒在产生它的那一回合，报的才是「流把签名丢了」。两条往返（settled→`response_from_wire`→`request_wire`）逐字节相等由 `anthropic/stream.rs` 的测试钉住。

**半个工具参数恒拒，两家同码同形。** 流中断时 `partial_json`／`arguments` 停在一个值的中间；把它读成 `{}` 就是把半次调用变成一次真的「无参数调用」，而 `exec {}`／`write {}` 会落账、过门、真执行。判定住 `mismatch::settled_tool_arguments(tool, at, raw)`——两家 dialect 都从碎片拼参数，故拼完即校验的那一句只有一处，拒词点名工具、index、已收字符数与解析停在哪里，码取 `E_PROVIDER`（`stream_cut`，retriable：截断的流与截断的 body 是同一种失败）。落选的是「在 OpenAI 侧沿用 `response_from` 的 `E_WIRE_MISMATCH`」：形状没有漂，漂的是这次传输，而 `E_WIRE_MISMATCH` 的恢复语会让人去改 dialect 与 base url 两个没有错的设置。

**认不出的帧跳过，缺失的结算帧不跳过。** provider 会加新的事件类型，一个人不该因为其中一个是新的就丢掉整次调用；但流在说明「为什么停」的那一帧之前结束，是 `Provider` 失败并且可重试——它和一个被截断的 body 是同一种失败，刻意不允许「保留已收到的增量」来补救：把不完整的回复当成完整的呈现出去，是这里唯一不能有的结局。

**机密楼宇的拒绝写一次。** 两扇门（`call` 与 `call_streaming`）都说同一句话，出自同一个 `confidential_refusal`——一条安全拒绝有两份拷贝，就是两个各自变软的机会。

### 8-14 gateway 目录化（形状：主类型居索引，方法按簇归文件）

`credential.rs`（988）→ `credential/vault.rs`（`Vault` 缝＋双后端＋`Persistence`／`Described`／`EnvReader`）／
`custodian.rs`（`Custodian`）／`oauth/`（`codec.rs` 编码、`flow.rs` 往返、`types.rs` 类型，
测试住 `flow/tests.rs`）；`dialect.rs`（512）→ `dialect/request.rs`（`sample_request` 提
`#[cfg(test)] pub(crate)` 供 response 面复用）／`response.rs`（strategies＋`proptest!` 住此，
快照搬 `dialect/snapshots/`）；`endpoint.rs`（639）→ `endpoint/config.rs`（类型＋`new`＋
` SecretResolver`＋loopback helpers 提 `#[cfg(test)] pub(crate)`）／`call.rs`／`model.rs`；
`router.rs`（559）→ `router/attached.rs`／`book.rs`／`payload.rs`（`Choice` 字段开
`pub(crate)`，`payload.rs` 无专属测试故无 tests 模）。
跨文件私有项开 `pub(crate)`，对外签名逐字节不变（`cargo public-api` 基线记定义位簇路径，
`memory` 同例）。

### 8-16 端点带着人给它定的规矩：`EndpointTuning` 与 `ModelFacts`（形状 7 值 ＋ 形状 4 适配器）

```rust
pub enum Retries { UntilHalted, AtMost(u32) }          // 缺席即 UntilHalted，就这一处
pub struct TuningDefaults { pub timeout_ms: u64, pub retries: Retries, pub stream_idle_timeout_ms: Option<u64> }
pub enum HeaderValue { Plain(String), Redeemed(SecretRef) }

pub struct EndpointTuning {
    pub label: Option<String>,
    pub timeout_ms: Option<u64>,
    pub request_max_retries: Retries,
    pub stream_idle_timeout_ms: Option<u64>,
    pub extra_headers: Vec<(String, HeaderValue)>,
    pub overrides: Vec<(String, String)>,   // JSON pointer → 值的文本
    pub proxying: Proxying,
}
impl EndpointTuning {
    pub const DEFAULTS: TuningDefaults;                        // 三个默认值的唯一住处
    pub fn call_timeout_ms(&self) -> u64;                      // 人设的，否则 DEFAULTS
    pub fn idle_timeout_ms(&self) -> Option<u64>;
    pub fn is_plain(&self) -> bool;                            // 无自定义头也无覆盖
    pub fn applied_overrides(&self) -> Vec<(String, Value)>;   // 文本读成 JSON 的唯一权威
}
pub struct AttachedEndpoint { …, pub tuning: EndpointTuning }
impl AttachedEndpoint { pub fn label(&self) -> &str }          // 缺省即 name

pub struct ModelFacts {
    pub id: String,
    pub context_tokens: Option<u64>,
    pub max_output_tokens: Option<Ceiling>,
    pub input_modalities: Vec<String>,
    pub input_price: Option<String>,
    pub output_price: Option<String>,
}
impl Endpoint { pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError> }
```

- **`EndpointConfig` 已有 `extra_headers` 与 `overrides`，`apply_override` 已实现并有测试**，所以这一条接的是既有机制而不是第二套：缺的只是「人填的那份设置」与「一周以后发出的那次调用」之间的存放处，它现在在 `AttachedEndpoint` 上，随 `endpoint_attached` 进账本、随重放回到书里。
- **覆盖以文本入账**：账本不收浮点。`applied_overrides` 是文本变 JSON 的唯一一处，规则为「解析得出即那个 JSON，否则即它看上去的字符串」。
- **自定义头或覆盖存在时，回环端点也走通用适配器**（`is_plain`）：本地适配器发不出自定义头，也写不进覆盖；悄悄丢掉它们就是用另一种方式去调用那个端点，而表单刚刚给人看的是这一种。
- **人写的头顶掉兼容格式自己的同名头**（按 ASCII 大小写不敏感比较），不是并列两行：两条 `anthropic-version` 是一条没有供应方承诺按谁的意思读的请求。
- **`stream_idle_timeout_ms` 是一次沉默的上限，不是整次应答的期限**（叶子 7.2 / G-02）。三层一个名字：线上、本 crate 与文案都叫 `stream_idle_timeout_ms`，装配层不再翻译它。**实现与名字一致**：`reqwest::blocking` 把一个请求的 timeout 当整体期限执行（异步层的 total timeout 覆盖整个 body），所以流式请求发出前把 `timeout` 清成 `None`，正文在一条自己的线程上逐行读，调用侧用 `recv_timeout(idle)` 计时，每收到一行重置。缺席则取 `timeout_ms`：一条没单独设过界的流也不允许永远安静。**代价写在这里而不是藏着**：对侧不说话又不断连时，那条读线程阻到对侧断连为止；结束这次调用是人要的，结束那条连接是对侧的。拒词报出越过的那个界（`no byte arrived for N ms`）而不是传输的原句。**败给的方案**：把线上字段改名 `stream_deadline_ms`——那会让一段写了六分钟的长回答在五分钟整被切，而那正是人抱怨的那件事。
- **重试只有一个家，就是 `Retries`**（叶子 7.1 / G-01）。`Endpoint::call` 仍然一次调用一个来回，这一条没变；变的是「人没填」的意思：字段不再是 `Option<u32>`，缺席就是 `Retries::UntilHalted`，读者拼不出第二种答案。没有刹车的地方（设置页上的探测，人正等着，`Halt` 按不下去）读 `Retries::without_a_brake()`，它把 `UntilHalted` 兑成 **1 次**重试——这个读法住在设置自身上，而不是调用点的第二个默认值。账本里仍然只写已设的数字，缺键即无上限（`Retries::stated()`／`Retries::of()` 一对）。
- **三个默认值住 `EndpointTuning::DEFAULTS`**（叶子 7.3 / G-03）：`timeout_ms = 120_000`、`retries = UntilHalted`、`stream_idle_timeout_ms = 无`。`adapter_for` 读 `call_timeout_ms()`，旧的 `CALL_TIMEOUT_MS` 常量删除；表单的占位符由 `Query::Config` 带回这三个值，不再在客户端手抄一份。
- **自定义头的值是一个类型，不是一段依前缀猜的文本**（叶子 10.6 / S-06）。`HeaderValue::Plain` 逐字发出，`HeaderValue::Redeemed` 走与 `authorize` 同一格的兑付；写入侧（`HeaderValue::parse`）对 `Plain` 跑一次 `kernel::secret::scan`，命中即拒，拒词不复述那个值。**一条规则**：本城读得出的金库引用就是引用（`SecretRef::parse` 是该语法的唯一读者），其余一律是字面量。账本里写的是 `spelled()`，对 `Redeemed` 即那条引用，所以 `endpoint_attached` 仍然可导出。重放一条早于本检查写下的记录时，读不出引用的值仍读作 `Plain`：键已在账本里，在重放处丢掉它只会让人的端点不声不响地换一种叫法。
- **`list_models` 读出每一行真正说了的东西**。OpenAI 形的 `/models` 一行里除 id 之外有什么由供应方决定：`context_length` 与 `max_completion_tokens`、同样两项嵌在 `top_provider` 之下、模态写在 `architecture` 里、绝大多数什么都不写。读法因此是「一组问题，各自由第一个带着它的键作答，没有键带着它就缺席」。**城不补零、不补默认、不补猜测**：补出来的数字会盖过真正计费的那个。
- **价格按供应方自己的文本原样携带**。单位也是供应方的——按 token 还是按百万 token，按美元还是按美分——而一个没人能拿去对账单的换算值，比供应方印出来的那串字符更糟。

### 8-15 `gateway::reach`：一次分段读数，以及「哪些调用走这台电脑的代理」（形状 4 适配器）

```rust
pub fn reach(client: &reqwest::blocking::Client, rule: Proxying, base_url: &str, elapsed_ms: u64) -> kernel::Reach;
pub fn client_for(rule: Proxying, base_url: &str) -> reqwest::blocking::ClientBuilder;   // reach::proxy
pub fn through(rule: Proxying, base_url: &str) -> kernel::Through;                       // reach::proxy
pub fn is_local(base_url: &str) -> bool;                                                 // reach::proxy
```

- **分段怎么测**：没有代理适用时，先用 `to_socket_addrs` 解名、再用 `TcpStream::connect_timeout` 开一个套接字，两段各自成一个读数；随后无论如何都发一次真实请求，把它的错误链摊平成一行，按其中出现的字样归到「主机名不能进握手」「握手失败」「压根没到握手」三类之一，或者归到它答的状态码。**分类读的是链而不是 reqwest 的 `is_connect`**：一个被拒的套接字与一张不受信的证书在那个判断下是同一个答案。
- **`socks` 不花钱**：reqwest 0.13 的 `socks = []` 是空 feature，实现就在它自己的 `connect.rs` 里，锁文件不多一个包。`system-proxy` 只在 Windows 与 macOS 各拉一个读系统设置的包。
- **全城的 HTTP 客户端都在 `reach::proxy` 里造**（`client_for`）。同一条规则写在五处就是五条规则，它们一直一致到其中一处被改为止；更要紧的是，分段读数若自己再判一次，它报出的就是一条请求不会走的路——而那正是看报告的人唯一无法自己核实的东西。**这个缺陷真实存在过**：客户端已经 `no_proxy` 了，而读数还在按环境变量报 `Environment` 并把解名与套接字两段记成 `ProxiedAway`／`Skipped`。
- **默认把打到这台电脑的调用摘出代理，但那是默认而不是定理**（`Proxying::ExceptLocal`）：跑起来才现形——开了 system-proxy 之后，一台配了代理的机器把回环也送进代理，本地推理服务器由别人的网关代答 502。但把它写死就是替所有人做了一个只对大多数人成立的决定，而这一类决定失效时没有任何一屏能告诉人到底发生了什么。现在它是 `EndpointTuning.proxying` 的默认值，另两个值各自对应一类真实的机器（kernel-SPEC.md 8-50），而无论哪一个，读数都会把结论写在 `through` 那一格里。
- **工具服务器与订阅登录用默认值，且是显式地用**（`bin::mcp_http`、`bin::mcp_sse`、`credential::oauth::flow`）：两者都没有一份属于自己的设置可携。**会重新打开这一条的参数**：出现一个必须经代理才能够到的回环 MCP 服务器——到那时 `McpServer` 也要长出这一字段，而不是在这里改常量。
- **`is_local` 是全城唯一的那一条判断**：本地适配器拒一个不属于这台电脑的 URL、代理决定、设置页上那个 `local` 标记，读的是同一个函数。它曾经是两个（`native::is_loopback` 只认 `localhost`／`127.0.0.1`／`::1`，而 `reach::is_local` 按 `IpAddr::is_loopback` 判），于是 `127.0.0.2` 在一处算这台电脑、在另一处不算。
- **5 秒一段**：设置页上有人在等，一个在这个时间里答不出来的主机，人要的是知道，而不是继续等。

### 8-17 `gateway::provider`：厂商文档写下来一次，与输出上限的事实梯（形状 6 数据面 ＋ 形状 1 判定）

```rust
// provider::preset —— 数据面，逐行注出处
pub struct HostPreset { pub host: &'static str, pub base_path: &'static str,
                        pub dialect: Option<DialectKind>, pub models: &'static [ModelPreset],
                        pub source: &'static str }
pub struct ModelPreset { pub id_prefix: &'static str, pub context_tokens: u64,
                         pub max_output_tokens: u64, pub input: InputKinds,
                         pub source: &'static str }
pub const PRESETS: [HostPreset; 7];
pub fn for_host(host: &str) -> Option<&'static HostPreset>;
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset>;
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling>;

// provider::ceiling —— 判定，四档先命中者胜
pub enum CeilingSource { Person, Upstream, Preset, Policy }   // as_str(): person|upstream|preset|policy
pub struct Stated { pub person: Option<Ceiling>, pub upstream: Option<Ceiling> }
pub struct OutputCeiling { /* 私有：tokens、from */ }
impl OutputCeiling {
    pub fn resolve(stated: Stated, pinned: Option<Ceiling>, at: &str, id: &str) -> Option<OutputCeiling>;
    pub const fn tokens(self) -> Ceiling;  pub const fn source(self) -> CeilingSource;
}
```

- **事实梯只有一架，权威从高到低：人填 → 上游陈述 → 本地预设 → 策略缺省**（路线图 §20.2）。人填不是推断，故压过其余三档；上游 `/v1/models` 说过的话压过本城钉下的任何数字；`kernel::consts_policy::OUTPUT_CEILING_DEFAULT = 8_192` 只在前三档全数沉默时作答。**梯子恒有答案**，于是「没有任何行登记过的模型」不再是 messages 兼容格式写不出请求的那个缺口（A 章 B-01）。
- **`resolve` 的返回值带着是谁答的**（`CeilingSource`），因为只拿到数字的调用方说不出一次跑为什么停在那里。这是对 `anthropic.rs` 那句自我反对的回答——「a ceiling invented at the call site truncates runs for a reason that appears nowhere in the account」：现在它出现在账里（`model_selected.ceiling_from`，见 sprawling-SPEC §8-52）。
- **钉版目录与预设表是同一档的两个索引**：目录按精确 id 查，预设表按 host ＋ id 前缀查，一条测试钉住两个索引不为同一个 id 作答。`MarketSnapshot::lookup` 因此保持精确匹配，前缀匹配只发生在预设表内。
- **预设表逐行注出处**，每行带厂商文档地址；查不到的行不发明数字，而是让梯子落到下一档——这就是「不补零、不补默认、不补猜测」在登记面上的样子。价目列本次不落：厂商价目随时在动，一个没有复核日期的价目行就是第二个会漂的权威，价目继续住钉版目录（§8-7），由 Stage 复核。
- **本表登记七个 host**（2026-09-21 补三行，逐行注出处）：`api.anthropic.com`、`api.openai.com`、`openrouter.ai`、`generativelanguage.googleapis.com`，加 `api.kimi.com`（`/coding/v1`）、`api.moonshot.cn`（`/v1`）、`api.moonshot.ai`（`/v1`），后三行读自 `MoonshotAI/kimi-cli` 的 `src/kimi_cli/auth/platforms.py`（docs/third-party.md §1 已列为被看路径），兼容格式为 OpenAI 兼容——同仓 `kosong/chat_provider/openai_common.py` 以这三个 base URL 构造 OpenAI 客户端。**`api.kimi.com` 与 `openrouter.ai` 是同一类缺陷的两个实例**：一律补 `/v1` 会把订阅端点指到不存在的路径。
- **§20.2 的列集里，`label` 与「价格」两列不进本表——这是一条决定，不是一次推迟**（叶子 4.6）。`label`：一个端点在人眼前叫什么，今天已有唯一的家，即人自己填的 `EndpointTuning.label`（§8-16）；人没填时该显示什么，从 summary 已经携带的 base URL 里按 `reach::split` 读一次 host 即得。厂商展示名再落一列，就是把 URL 已经携带的事实重拼一遍，而两处一旦不一致，界面上那个名字与实际调用的主机会指向两家厂商。「价格」：一次调用按什么价结算，今天也已有唯一的家，且是一架有序的梯——厂商在 `/v1/models` 里陈述的原文（`ModelFacts.input_price`／`output_price`，§8-16）在上，钉版目录按精确 id（§8-7）在下，而 `CostSource::Authoritative` 在两者之上；按 host ＋ id 前缀再加一个索引，就是同一批厂商数字的第三个家。**结算读的是登记那一刻写进 `model_selected` 的那份价目**，故表里改一个数也追不回已登记的模型，第三个家只会静默地与前两个分叉。**重开参数**：当一次真实调用在钉版目录无行、上游又不陈述价目而必须结算出非零金额时，价目以**迁移**而非新增索引的方式进本表——把价目事实从 `MarketSnapshot` 整体搬到 host ＋ id 前缀索引下，钉版目录同期删去价目列，每格带复核日期。`label` 的重开参数同理：`channels::EndpointSummary` 决定展示名不再由人填时，那一列进表且 `EndpointTuning.label` 同期降为覆盖值。
- **登记站点的 base URL 有两个家，今天不冲突，且这不是长久之计**：`oauth_profiles.api_base` 写「一次订阅登录完成后去连哪里」，`preset.base_path` 写「人粘一个裸 host 时补什么路径」。`api.kimi.com` 两处都在（`https://api.kimi.com/coding/v1` 与 `/coding/v1`），两处一致但无人断言其一致。**迁移方向**：订阅登录完成后的 attach 走 `normalise_entered`，`api_base` 退成裸 host，路径只由本表作答。
- **厂商的 id 属于厂商的 host**：中转站以同名 id 转发时不套用厂商图表，它截在哪里是它自己的事实，而它的模型列表就是它陈述这件事的地方。
- **主机表只有这一张**：`router::normalise` 的路径与形状缺省从本表取（`openrouter.ai` 是 `/api/v1`、Gemini 形是 `/v1beta`），归一化算法自身不带任何主机名。两者是一件事的两半（路线图 §3.2 审阅第 13 条）。

### 8-18 `gateway::provider::registry`：一个端点是怎么连的，attach 时解析一次（形状 1 判定 ＋ 形状 6 数据面）

```rust
// provider::registry —— 连接解析，attach 后不再重算
pub enum Family { Codex, ClaudeCode, GrokBuild, KimiCli }      // as_str(): codex|claude_code|grok_build|kimi_cli
impl Family { pub const fn shape(self) -> DialectHint; pub const ALL: [Family; 4]; }
pub enum ConnectionKind { OpenAiCompat, Responses, AnthropicNative, Harness(Family) }
impl ConnectionKind {
    pub const fn wire(self) -> DialectKind;      // 今天由哪支写请求的笔作答
    pub const fn as_str(self) -> &'static str;   // 七个扁平词，账本／wire／配置文件共用
    pub fn parse(word: &str) -> Result<ConnectionKind, AxError>;
}
pub fn resolve(shape: DialectHint, subscription: Option<Family>) -> Result<ConnectionKind, AxError>;

// provider::modality —— 会话之外的两张脸（14.4 第一批）
pub enum Modality { Embedding, Rerank }          // as_str(): embedding|rerank；ALL: [Modality; 2]
impl ConnectionKind {
    pub const fn path_for(self, modality: Modality) -> Option<&'static str>;
    pub fn url_for(self, base_url: &str, modality: Modality) -> Option<String>;
}

// provider::modality::embedding —— 请求与回答的真实形状
pub struct EmbeddingRequest;                     // new(model, inputs) -> Result；with_dimensions；texts()；body()
pub struct Embeddings;                           // parse(&Value, &EmbeddingRequest) -> Result
                                                 // vectors() -> &[Vec<f64>]；model()；prompt_tokens()

// provider::modality::rerank —— 同上
pub struct RerankRequest;                        // new(query, passages) -> Result；passages()；body()
pub struct Rank { pub passage: usize, pub score: f64 }
pub struct Ranking;                              // parse(&Value, &RerankRequest) -> Result；ranks()；best()
```

- **「这个端点是怎么连的」今天散在三处，没有一处说得出**：dialect 说由哪支笔写请求，credential 说是 key 还是订阅登录在付账，上限梯说这次调用能写多少。三处各答一半，一致到其中一处被改为止。`ConnectionKind` 是那一个答案，**attach 时在归一化之后解析一次**，写进 `endpoint_attached`、由 `Query::Config` 读回，调用路径不再猜——一件事算两遍就是两遍可以算出不同结果。
- **登记面今天在丢一个事实**：人粘贴 responses URL，归一化正确地判出 `DialectHint::Responses`，而存进去的 `DialectKind` 只有两个变体，于是它被折成 `OpenAi`（`bin::assembly::credentials::Entered::resolved`）。`ConnectionKind::Responses` 把这句话留住；`wire()` 今天仍答 `OpenAi`，因为本仓只写了一种 OpenAI 形的请求体（路线图 4.5）。**那一天到来时改的是 `wire()` 的一条臂，没有人需要重新 attach 端点。**
- **订阅决定连接，冲突当场拒绝**：harness 的线属于厂商而不属于表单，故签了某家订阅即 `Harness(family)`；人粘贴的形状与该家的 `shape()` 不一致时报 `E_CONFIG_INVALID` 并给出两条出路（换该厂商自己的 base URL，或改用 API key 登记）。**既不静默丢弃参数，也不替人猜**——两者都会在第一次调用处变成 404。
- **`DialectHint::Unset` 是真状态而不是缺失值**：没人说过形状时解析拒绝，恢复语指向「把供应方文档印出来的完整 URL 粘进来」，因为带 `chat/completions`／`responses`／`messages` 尾段的 URL 自己就说了。
- **词只有一套**：七个扁平词（`openai_compat`／`responses`／`anthropic_native` ＋ 四个家名），`as_str` 写、`parse` 读，一条测试钉住往返与不重词。harness 不拼复合串，读者无需切分。
- **模态第一批只落形状**：`Modality::{Embedding, Rerank}` ＋「哪种连接在哪条路径上服务它」一张表。Anthropic 不发布这两张脸，四家订阅 harness 卖的是会话，故都答 `None`——**没有路径就是不服务**，调用方连 URL 都拼不出来。路径与 base URL 的拼接复用全城唯一那个 `router::join`。**入账（`embedding_called`／`rerank_called`）等 payload 类型化（G-15）落地后再接**，本节不预先写键名。
- **两张脸的字节形状也各只有一处**（2026-09-21 落）：`embedding` 的请求与回答照 `openai/openai-openapi` 的 `CreateEmbeddingRequest`／`CreateEmbeddingResponse`／`Embedding` 三个 schema 写，`rerank` 照 `huggingface/text-embeddings-inference` 的 `docs/openapi.json` 的 `/rerank` 路径与 `RerankRequest`／`Rank` 写；两处出处与读取日期写在模块头。**请求显式写 `encoding_format`**，因为读答案的那段只认一种编码，而厂商可改的默认值不是可以照着解析的依据。**回答按 `index` 归位而不按到达顺序**：把第三段文字的向量配给第一段，是一个不报错的检索错误。**名次不在本城重排**：分数只在一次回答内可比，服务端排好的序就是答案，再排一次就是本城对自己付钱问来的名次有第二个意见。
- **chat 不是本枚举的成员**：会话路径归已经在调用它的那处（`router::attached::chat_path`），在这里再写一次就是每回合都在发的那条路径有了第二个家。

### 8-19 system prefix 的稳定性是一条被守护的性质（路线图 17.2）

- **性质**：同一配置下连续两次派活，发往端点的 system prefix 逐字节相等。守在 `provider::stability`，不走网络、不需要端点。
- **为什么它值一道闸**：兼容中转按整段 prompt 做缓存。**一个逐轮变化的字节就把 cache key 挪走一次**，于是一段语义上从未变过的前缀每回合重付全价。情报（使用者观察，本仓未独立复核）：某上游客户端自某版起在发往 `/v1/messages` 的每条请求的 system prompt 最前面插入一行带 `cch=` 参数的文本，每条不同；该字段其官方服务端能识别，中转不能。
- **供应层不得照抄这个字段**：它是对方服务端的计费旁路，本城既不需要也不该发。断言因此有两条——写出的请求两次逐字节相等；**上线的 system 文本与交给供应层的那几块逐字节相等，前面不多任何东西**，且不含 `cch=`。七种连接各测一遍，新增一种连接若未想过缓存，红在这里而不是红在别人的账单上。
- **本仓的证伪结论（2026-09-21，读码而非跑网）：没有复现这个病，闸今天是绿的。** 前缀由 `runtime::prefix` 在 Run 开始时冻结一次，四块顺序为 city／building／resident／run，最稳定的在前；`build_prefix` 与 `system_blocks()` 都是对字节的纯函数，**无时钟、无随机、无 run id 进前三块**。
- **任何新增前缀成分必须说明它为什么可以逐轮变化。** 不能说明的，就得挪到 run 块之后或根本不进前缀。
- **两种失效要分开写，否则下一个读者会把闸关掉**：
  - **设计上正确的失效**：改 effort 使缓存断点失效（§110 引官方排错文档：「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」）。前缀真的变了，人也真的改了配置。**顺带更正一处口口相传的说法**：effort 本身并不住在 prefix 里，它是 `ChatRequest.effort` 独立字段（`bin::assembly::freezing` 冻进 `RunPlan.shape`）；工具卡片同理住 `ChatRequest.tools`。住在 prefix 里的是技能清单（resident 块的 catalog 渲染）。
  - **本条要防的意外失效**：没人决定过、也没人看得见的逐轮变化——时间戳、run id、随机序、每次请求重排的集合，以及照抄来的计费字段。

### 8-31 重试上限住 kernel

`gateway::Retries` 现在是 `kernel::Retries` 的再导出。缺席的含义（`UntilHalted`）、探测在无人可停时读成一次（`without_a_brake`）、以及记进账本时写不写这个数（`stated`），三条都由那一处定义，本 crate 不再自持一份。

### 8-20 第三支笔：OpenAI responses 面（叶子 4.5 · F-05）

`dialect/responses/` 三个文件——`request.rs`（规范请求 → `input` 数组）、`reply.rs`（`output` 数组 ↔ `ChatResponse`）、`stream.rs`（具名事件 → `Increment`，终帧 → 已定答案）。`dialect` 的五个入口各多一条臂，闭集由二变三。

**形状取自供应方自己的规格**：`openai/openai-openapi`，`openapi.yaml` 自述 `info.version` ＝ 2.3.0，提交 `ddface9b`（2026-09-19）。不取自任何客户端库，也不取自记忆。看着不对的字段通常是搬过家的字段——改这里之前先读那份文档。

**它不是第二支笔的一个开关。** chat 面发 `messages`、读 `choices`；这面发 `input`、读 `output`——一个数组，成员是消息、函数调用与推理项，而不是一条挂着若干字段的消息。流是第三套文法：具名事件（`response.output_text.delta`）而不是匿名 chunk。折在一起就是一个在每一步上分支的写入器。

**这面载得动而 chat 面载不动的两件事：**

- **显式缓存断点**。标了 `cache` 的 `SystemBlock` 成为带 `prompt_cache_breakpoint` 的 `input_text` 片段。本城本来就在算段边界，于是把它说出来，而不是交给前缀匹配去猜。system 段因此走 `input[0]`（`role: developer`，一段一片）而非 `instructions`——后者是一个字符串，装不下四段与它们的边界。
- **流自带已定答案**。终帧（`response.completed`／`incomplete`／`failed`）携整个 response 对象，重组读它而不是缝合碎片。**一个解析器因此得以保持**：流式调用与阻塞调用交给 `response_from` 的是同样的字节，两者得不出不同的结论。

**它主动放弃的一件事**：上一轮的 `Thinking` 块不回发。这面只接受带供应方自己签发的标识与密文的 `reasoning` 项，本城两样都不存；拼一个出来会在第一次调用处被拒。规范记录仍留着那个块，历史不缺东西。

**读不认识的输出项不算失败**：`Item::Unread`——内建工具调用本城从没要过，本 build 之后新增的项也一样；本城要的字段就在它们旁边，全都在。判定写成具名枚举而不是 `_ =>`，于是「放过去」是一个有名字的决定。

**停止原因来自整体而非某条消息**：这面报的是 response 的 `status`，以及 `incomplete` 时的 `reason`。产出了调用就是 `ToolUse`（无论 status 说什么——调用方两种情况下都有一个调用要跑），`incomplete` ＋ `max_output_tokens` 是 `MaxTokens`，`completed` 是 `EndTurn`，其余拒绝而不猜。

**`ConnectionKind::wire()` 从此指向真的那一支**（§8-18 那条「那一天到来时改的是 `wire()` 的一条臂」兑现）：`Responses` → `OpenAiResponses`，`Harness(Codex)` → `OpenAiResponses`（Codex 的订阅答在 responses 面，这是 `Family::shape()` 早已声明的事，此处读它而不是重说一遍）。一条遍历式断言钉住：每一家被调用的脸，与它自己声明的脸相同。

**`store: false` 每条请求都写**：本城自持历史，上游留一份副本就是第二份，且它比本城「决定忘记」这个动作活得更久。

**`provider::stability` 的守卫因此长出第三条读法**：system 文本在这面是 `input[0].content[*].text`。那道闸的两条断言（两次派活逐字节相等、上线文本与交给供应层的几块逐字节相等且前面不多任何东西）现在覆盖全部七种连接。

### 8-21 `gateway::credential::vault::file`：口令解开的加密金库文件（叶子 14.5 · F-28 · 形状 4 适配器）

```rust
pub(crate) struct FileVault { /* path、派生密钥、salt、密文条目 —— 全私有 */ }
impl FileVault {
    pub(crate) const SOURCE: &'static str = "encrypted-file";
    pub(crate) const PERSISTENCE: Persistence = Persistence::AcrossRebootsWithPassphrase;
    /// 打开或创建；口令不对即 `E_CONFIG_INVALID` 具名拒绝，恒不 panic、恒不当作空金库
    pub(crate) fn open(path: PathBuf, passphrase: &Zeroizing<String>) -> Result<Self, AxError>;
}
impl Vault for FileVault { /* put / get / delete，写即整文件替换 */ }
pub enum Persistence { AcrossReboots, AcrossRebootsWithPassphrase, ThisBoot, ThisProcess }
```

**它答的是哪个威胁**：本城的工具调用能 grep 城所运行的电脑。明文躺在配置旁边的凭据，就是 Agent 能读进窗口的凭据；密文对这个读者有效。磁盘被拿走是另一个问题，本节不声称答它——持有口令者本来就不被挡在外面。

**条目逐条封装**：ChaCha20-Poly1305（RFC 8439），每条每次写盘取新的 96 位 nonce；AAD 绑格式版本与 `secret:realm/name`，故一条密文被挪到另一个名下就打不开（有测试钉住）。KEK 只来自口令 + Argon2id（RFC 9106 第二推荐档：64 MiB / 3 轮 / 1 道）。

**成本参数只有一个家**：文件只记 salt 与密文，不记三个 cost 数字——读盘时不听文件的，`FORMAT_VERSION` 变更才是改它们的方式。密钥派生一次、进程内持有：Argon2id 是故意慢的，每次读都派生等于给每次模型调用加一秒。

**原子替换归本模块**（E-6 ④ 的决定）：`city::document` 对城文档做同一支舞，但它是 `city` 的 `pub(crate)`，而 gateway 不依赖 `city`、依赖方向上也不该依赖（ARCHITECTURE §2）。因此这一个文件的替换住在这里；把这支舞提升为两边都能调的权威，是一次连 `city` 两个调用点一起搬的迁移，归拥有 `kernel` 的那条路。**这一条是记录，不是遗漏。**

**等级句仍只有一处**：`Persistence::consequence()` 新增一档 `AcrossRebootsWithPassphrase`，句子是「密钥住在城所运行电脑的一个加密文件里，每次启动城要输一次口令」。`describe` 与 `resolve` 照旧读它。

**后端选择尚未接线**：`Custodian::probe` 今天在 keyring 与内存之间选，接上这一档（何时提示口令、口令从哪里来）落在 `custodian.rs`，本轮不属本路可写集，见交付说明。

**两个重开参数照抄 §20.5.3，本版不进树**：ML-KEM-1024 在「多机同步金库」立项那天进——今天单机无对象可封；FN-DSA-1024 在「账本需要对第三方可验证的出处证明」立项那天进——今天防篡改由哈希链承担。对称 AEAD 本身已抗量子，故这两条不是安全缺口。

**闸同步扩一条**（`xtask secret`）：城的保留子树（`kernel::RESERVED_PREFIX`）里出现凭据明文，按「城的记录」这条规则报，recovery 指向 `Custodian` 而不是手改；子树内非 UTF-8 的对象（CAS 产物）不按文本规则判，否则第一屏全是内容存储、真正那一行被埋掉。

### 8-22 `gateway::credential::oauth::device::login`：一次在途的设备码登录（叶子 14.3 · F-25 · 形状 2 适配器）

```rust
pub enum OauthPending {                 // 在途登录的两种形状，穷尽
    Redirect(RedirectPending),          // 回跳：auth_url ＋ state ＋ code_verifier
    Device(DeviceLogin),                // 设备码：厂商答的那份 ＋ 轮询表 ＋ 两个时刻
}
impl OauthPending {
    pub fn open_url(&self) -> &str;          // 人要打开的那一页，两种形状同一个问题
    pub fn user_code(&self) -> Option<&str>; // 只有设备码这家有一个要人念出来的短码
}
pub enum DeviceStep { Signed(OauthTokens), NotYet { seconds: u64 } }
pub fn device_login_begin(profile, now_ms, timeout_ms) -> Result<OauthPending, AxError>;
impl DeviceLogin {
    pub fn ask(&mut self, profile, user_code: &str, now_ms: u64) -> Result<DeviceStep, AxError>;
}
```

**人就是那个轮询循环。** RFC 8628 让客户端在人于另一台设备上同意期间反复问令牌端点；本城改为「人说一次自己已同意，城就问一次」，因为另一种写法是把跑整座城的那条工作线程停在那里等最多半小时。**这不是把 RFC 的规则让掉**：`ask` 在开 socket 之前先判两件事——粘回来的码不是本次登录给人看的那个就拒（`E_INVALID_ARGS`，与回跳路上 `state` 同一个职责：别人的一次登录不能收尾我这一次），以及厂商声明的 `interval` 还没过就答 `NotYet` 而不发送（§3.5 的 MUST；忽略它的客户端会被厂商限流）。`slow_down` 与过期两条仍由 `DevicePoll::step` 判，`ask` 只负责把新的「最早下次」记在城的时钟上。

**时钟在外，判定在内。** 本 crate 不取时钟也不取随机（`oauth_random` 同理收外部熵）；`now_ms` 由装配层读城的时钟递进来，于是「四秒后才许再问」这条规则在一个不花时间的测试里就能跑完。装配层那张在途登录表因此类型未变——它装的仍是 `OauthPending`，只是这个类型现在穷尽两种形状。

**一次兑付只有一个发送**（`credential::oauth::exchange`）：回跳的 JSON 体与设备码的表单体在内容类型之后就是同一次交换——同样的 `access_token`／`refresh_token`／`expires_in`，同样的「拒词不引用对侧正文」（正文里有刚用过的 code）。故 `post_json`／`post_form`／`tokens` 住一处，`FormPost::CONTENT_TYPE` 也读这里的那一个串。状态码交回调用方而不在此判：设备码那条路要从非 2xx 的正文里读 `error` 才知道该不该再问（§3.5），回跳那条路除了停下没有别的事可做。

**账本与界面**：`login_started` 的 `auth_url` 在设备码这家是厂商的验证页（有 `verification_uri_complete` 就用它，那一页已经把码填好了），并多一个 `user_code` 键——**缺席而不是空串**，回跳登录没有这样一个码，空串在页面上会读成「码没取到」。设备码本身恒不进账本、恒不进 `Debug`：它是被偷了就能冒充的那一半。
