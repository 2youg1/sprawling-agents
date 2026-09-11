# gateway-SPEC.md

> crate：`gateway`（lib，依赖 kernel）。本 SPEC 先于代码存在（十七节）。
> Stage 3 八模块：dialect／endpoint／native／credential（内缝 Vault）／oauth_profiles／admission／market／cost；router 属 P1 不在本版。
> 本 crate 覆盖的语义：模型路由；provider 客户端自写＋认证两半；Custody；市场快照与成本；model 缝；provider 侧准入。

## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `dialect` | 城内规范 Anthropic Messages 与 OpenAI Chat 的双向纯函数翻译；保断点位／工具形状／usage |
| `endpoint`＋`native` | 自写线格式 HTTP 客户端（reqwest blocking＋rustls）实现 `kernel::Model`；逐字段请求覆盖；native＝回环 OpenAI 兼容服务的固定形 |
| `credential`＋`oauth_profiles` | Custody 效果半（scan 命中→入 Vault→原位替换 SecretRef）；兑付（组请求末格 expose，credential_lent）；describe；持久性探测；OAuth 流程（代码）＋情报表（数据） |
| `admission`＋`market`＋`cost` | provider 侧并发上限＋确定性最小发起间隔；模型目录快照＋钉版回滚；per-call 入账（权威计费额优先） |

## 2 验收标准

- dialect：golden（两 Dialect 各一请求一响应）＋proptest 往返（响应侧 wire→canonical→重渲染 wire 逐字段等值；请求侧断点位／工具形状／文本字节无失）。
- endpoint：对回环假 provider 服务的半流中断（SSE 截断→E_PROVIDER 且不产伪 ModelReturn）＋幂等重试（同 IdemKey 重发，对外恰一次效果——由调用方 dedup 看守，endpoint 自身无重试暗策略）。
- credential：A13 链——外来字节命中 shape→`secret_captured`（无明文无哈希前缀）→配置只见 `secret:`→resolve 兑付产 `credential_lent`→`describe` 恒不返回值。探测三态（跨重启／仅本次开机／仅本进程）可注入验证。
- admission：同一到达序列两次求值，放行时刻序逐条相同（无时钟采样，`now` 入参）。
- cost：权威计费额在场则恒胜价目推算；两源不一致时以权威为准并记差额；A20 的取材面（对账断言住 memory::attribution）。

## 3 假设与歧义

- **native 的 S3 形**＝指向回环地址的 OpenAI 兼容服务（llama.cpp/ollama 一类）的固定客户端：无凭证要求、禁非回环 base_url。本地引擎进程管理不属本 crate（P4 产品化再议）。
- **tokio 不引入**：turn 是同步函数面，endpoint 用 `reqwest::blocking`（内部自管运行时线程，不出接口）。B.7 的 tokio 行推迟到 S4 channels（首个真异步消费者），偏离已记（§13）。
- 线格式 JSON 允许浮点（temperature 等 provider 字段）：dialect 是翻译面不是判定路径；判定路径（cost／admission／market 价目）恒整数。
- OAuth 活体流程不可在 CI 验证：流程状态机与请求构造以形状测试看守，端到端属人工清单。

## 4 现状分析

空壳 lib。无既有公开面（api-baseline 自此起算）。

## 5 权威信源

自写客户端的四条被逼理由与「流程是代码、情报是数据」；Custody 全节；model 缝（native／endpoint 两生产适配器）；Anthropic Messages API 与 OpenAI Chat Completions API 官方文档（线格式字段名以官方为准）；keyring crate 文档（平台凭证服务绑定）。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。**本 crate 同例**：同一类型的 inherent impl 住不同簇文件时基线为每块各记一行 `impl`（`Endpoint` 两行）；下游 `sprawling` 基线记 `gateway` 内定义位簇路径（如 `credential::custodian::Custodian`），公共拼写不变。

Dialect／Endpoint／Custody／Vault／SecretRef／Sealed／admission／market snapshot／UsdMicros／权威计费额（authoritative billed amount）。概念名英文原词；「兑付」＝resolve+expose 的合称。

## 7 模块边界

```
dialect（路由，纯函数）◀── endpoint／native（I/O 适配器，impl kernel::Model）
dialect ───────────────▶ anthropic／openai（各自一种线格式的两向翻译）
anthropic／openai ──────▶ mismatch（共用的读取器与拒词；单向，无环）
credential ──▶ 内缝 Vault（pub(crate) trait：keyring 生产适配器＋会话内存第二适配器）
oauth_profiles（数据面，零分支）◀── credential（流程消费情报）
admission／market／cost：纯判定与数据面，被 endpoint 与 S3 回合层消费
```

**market**：`ModelEntry` 增 `max_output_tokens`——一次回答能吐多少字是**模型的事实**，不是调用处的选择；探测接口不返回它，故它随模型登记入目录行。尚未登记的本地模型沿用 `local` 行的保守上限，登记面（§8-9）接管后改为人确认过的行。

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
#[non_exhaustive] pub enum DialectKind { Anthropic, OpenAi }
pub fn request_wire(kind: DialectKind, req: &ChatRequest) -> Result<serde_json::Value, AxError>;
pub fn response_from_wire(kind: DialectKind, wire: &serde_json::Value) -> Result<ChatResponse, AxError>;
pub fn response_wire(kind: DialectKind, resp: &ChatResponse) -> Result<serde_json::Value, AxError>;
                                    // 响应侧双向：往返性质可测（wire→canonical→wire 等值）；重放剧本也要它造假响应
```

- **保三样**：①断点位——canonical 的 `cache: true` 标记翻到 Anthropic 侧＝`cache_control{type:"ephemeral"}`，逐块原位；OpenAI 侧无显式断点（供应商缓存是隐式前缀匹配），翻译**记录性丢弃**（文档声明，不静默）；②工具形状——`ToolDef{name, description, input_schema}`↔Anthropic `tools[]`／OpenAI `tools[{type:"function",function:{…}}]`，逐字段；tool_use↔tool_calls（id/name/args 无失）；③usage——Anthropic `usage{input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens}`／OpenAI `usage{prompt_tokens,completion_tokens,prompt_tokens_details.cached_tokens}`→`ModelUsage` 四整数字段，缺失字段取 0。
- 未知 wire 字段：请求侧不产（我们只写自己声明的字段＋overrides）；响应侧忽略未知键、缺必需键报 `E_WIRE_MISMATCH`（subject 写键路径）。
- canonical 枚举（Role／StopReason／ContentBlock）在 crate 外属 non_exhaustive，本模块通配臂恒 fail-closed（未知变体＝E_WIRE_MISMATCH，不猜不默）；wire JSON 键序＝serde_json BTreeMap 字典序（确定性，对端语义无关）。
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
    pub extra_headers: Vec<(String, String)>,   // 明文头（非凭证）；凭证恒走 auth
    pub overrides: Vec<(String, serde_json::Value)>,  // 逐字段请求覆盖：JSON Pointer→值，最后应用
    pub timeout_ms: u64,
}
#[non_exhaustive] pub enum AuthSpec { Bearer(SecretRef), Header { name: String, value: SecretRef }, None }
pub struct Endpoint { /* config、reqwest::blocking::Client、resolver: Box<dyn Fn(&SecretRef)->Result<Sealed<String>,AxError>> —— 私有 */ }
impl Endpoint { pub fn new(config: EndpointConfig, resolver: /* 兑付闭包，由 credential 提供 */) -> Result<Endpoint, AxError>; }
impl kernel::Model for Endpoint { /* call：ChatRequest（req.chat）→dialect→HTTP→ChatResponse→ModelReturn */ }
```

- **组请求五步**：canonical→`request_wire`→逐条应用 overrides（JSON Pointer，后者胜）→认证头兑付（`resolver` 取 `Sealed`，`expose()` 只在写头那一格，写完即 drop 零化）→POST。响应四步：状态码判定（429/5xx→E_PROVIDER 携 retry 语义；4xx→E_PROVIDER 携 provider 错误体摘要）→`response_from_wire`→usage 抽取→`ModelReturn`。
- **半流中断**：SSE 流截断（连接断／不完整事件）＝`E_PROVIDER`，恒不产部分 ModelReturn；S3 先落非流式全量路径，流式属只加（接口不变，config 增 `stream: bool` 字段即可）。
- **无暗重试**：重试／转移是 watchdog 与 admission 的决策，endpoint 一次调用恰一次 HTTP 往返；幂等由调用方 IdemKey dedup 看守。
- base_url＝完整端点 URL（逐字段哲学，不拼路径）；EndpointConfig 增 pricing: Option<ModelEntry>（结算在适配器内以便 ModelReturn 携 billed 入账；权威额线上无标准槽位，现行恒 PriceSheet 源）；reqwest 0.13 的 rustls feature 名＝`rustls`（非 0.12 的 rustls-tls）；非流式先行，半流中断以截断 body 实测（E_PROVIDER，恒不产部分 ModelReturn）；kernel::ModelReturn 增 usage/stop/billed 三字段＋bare()/from_response() 两构造面（kernel-SPEC §8-24 同集）。
- `.expose(` 白名单（xtask secret）：本文件与 native.rs 是 gateway 侧仅有的两个合法出现点。

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
#[non_exhaustive] pub enum Persistence { AcrossReboots, ThisBoot, ThisProcess }
pub struct Described { pub configured: bool, pub source: String, pub persistence: Persistence, pub writable: bool }

pub struct Custodian { /* backend: Box<dyn Vault>、source 名、persistence —— 私有 */ }
impl Custodian {
    /// Startup probe: write-read-delete on each candidate backend, first
    /// pass wins; all failed -> session-memory fallback + provider_degraded
    /// payload returned to the caller for ledger append.
    pub fn probe() -> (Custodian, Option<Payload>);
    /// Custody effect half: spans from kernel::secret::scan, plaintext into
    /// the vault, `secret:` literals back in place. Payloads carry
    /// realm/name/origin/span length only - no plaintext, no hash prefix.
    pub fn capture(&mut self, bytes: &[u8], origin: &str) -> Result<Captured, AxError>;
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
                                    // 遮蔽即拒；空值即未配置；入参取 Zeroizing 非 Sealed：
                                    // `.expose(` 白名单恒三文件（定义处＋两解封点），Custody 是库不是 sink——
                                    // 持 Sealed 者恒密封直至线上；S4 PutSecret 在自己边界内转 Zeroizing。
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError>;   // 未命中→E_CREDENTIAL_MISSING；恒不跨操作缓存
    pub fn describe(&self, reference: &SecretRef) -> Described;                        // 恒不返回值
}
pub struct Captured { pub replaced: Vec<u8>, pub events: Vec<Payload> }   // secret_captured 载荷（入账归调用方）
```

- 生产适配器＝keyring crate（Windows Credential Manager／macOS Keychain／Linux secret service）；第二适配器＝会话内存 BTreeMap（探测全败的兜底＋测试面）。**恒不自写加密文件**。
- realm/name 派生：capture 时 realm=形状表 provider（无则 "detected"）、name=定长计数器 `cap-<n>`（确定性，无随机）；用户改名属 S4 命令面。
- OAuth 两流程（PKCE／设备码）＝代码：`pub fn oauth_begin(profile, …) -> OauthPending`＋`pub fn oauth_redeem(pending, …) -> Sealed<String>` 的纯构造（HTTP 往返由调用方经 endpoint 的 Client 执行或 S4 命令面驱动；此处交付构造与校验，不交付活体登录）。续期＝到期前 resolve 触发 refresh 构造。
- 环境变量是只读来源（键形 `SPRAWLING_SECRET_<REALM>_<NAME>`）：`describe.writable=false`；`set` 撞遮蔽即拒并指名遮蔽者；读取器可注入（edition 2024 的 set_var 不安全，测试恒不改进程环境）。
- A13 值正确性经真 Endpoint＋回环假服务在线断言（capture→vault→resolve→写头，服务侧见原值）——credential 自身零 `.expose(`；PKCE 以 RFC 7636 Appendix B 向量钉实（S256，sha2 为外部协议事实非第二哈希权威）；base64url／percent-encode 自写纯函数；probe() 对真平台服务的验证属装配期人工清单（测试不擅动开发者凭证库）。

### 8-5 gateway::oauth_profiles（形状 6 数据面）

```rust
pub struct OauthProfile { pub provider: &'static str, pub auth_endpoint: &'static str,
                          pub token_endpoint: &'static str, pub scopes: &'static [&'static str],
                          pub client_id: &'static str, pub headers: &'static [(&'static str, &'static str)] }
pub const OAUTH_PROFILES: [OauthProfile; N] = [ /* 各 provider 一行；只有数据零分支 */ ];
```

- 内容自活跃维护的开源 harness 情报汇集（跟情报不跟代码）；上游变更检测是周任务不是 CI 门。S3 初版收录 Anthropic／OpenAI 两行（可空流程字段留空串——宁缺毋错，缺项在 oauth_begin 处 fail-closed）。
- **情报源两家**：codex 管 OpenAI 一侧，pi 管 Anthropic 与其余订阅 provider；名单与复核办法住 `docs/third-party.md`。
- **兑付真的发出去（credential＋oauth_profiles）**：`pub fn oauth_redeem(profile, pending, code, timeout_ms) -> Result<OauthTokens, AxError>` 真正把 POST 发出去，`OauthTokens { access, refresh, expires_in_s }` 两个密文恒裹在 `Zeroizing` 里且 `Debug` 手写成 `<redacted>`——`Zeroizing` 自己的 `Debug` 会打印明文，派生一个就等于把活令牌交给第一条格式化它的 panic 信息。**发送住 credential 而不住 endpoint**：本模块就是整条 OAuth 流程，把「造请求」与「发请求」分到两个模块，就是让一次兑付有两个权威。**拒词恒不引用对侧正文**：令牌端点的错误页里可能带着刚用过的 code。`OauthProfile` 增 `api_base`（该 provider 的 API 根，登录完成后据它自动 attach；空串＝fail-closed，与空端点同口径）。
- **续期与兑付共用一次发送（credential）**：`oauth_refresh(profile, refresh: &Sealed<String>, timeout_ms)` 与兑付**共用一次发送**（`send_token_request`），故「拒词不引用对侧正文」只写一次、也只可能对一次。入参是 `Sealed<String>` 而不是 `&str`：明文只在**线前最后一格**出现，这与 endpoint／native 是同一类兑付点，故本文件同期进 `xtask secret` 的 expose 白名单——**放宽白名单而不是在调用点绕开它**，因为绕开的写法会让装配层持明文，而那正是这张名单存在的理由。
- **`state != code_verifier`（在 `credential::oauth_begin` 执行）**：两个值答的是不同的问题——verifier 证明「来兑的就是当初请求的那个客户端」，state 证明「这次回跳对应本进程发起的那次请求」。互用就是把一个证明做两遍、另一个一遍不做，且已有 provider 直接以 `400 invalid_grant` 拒。**在构造点拒而不交给接线的人记住**：这正是一份照上游抄来的实现会具有的形状。
- **迁出为独立 crate：未做，理由写在这里**。迁出的前提是一个**仓库外**的、持自有许可的上游存在；它今天不存在。在本仓库里建一个「另一个许可的目录」只会同时得到两件坏事：MPL 头门要么被改得认不出它、要么给它戟上一顶不属于它的帽子，而两者都不是迁出。因此只做两件真实可做的：表缺失时恒三段式拒（已在），以及 NOTICE 义务归 `xtask` 的 `release` 门。

### 8-6 gateway::admission（形状 1 判定函数）

```rust
pub struct AdmissionState { /* in_flight: u32、interval_ms: u64、consecutive_ok: u32、next_allowed_at: TimeMs —— 字段私有，构造子给初值 */ }
#[non_exhaustive] pub enum AdmissionVerdict { Admit, Hold { until: TimeMs } }
pub fn admit(state: &AdmissionState, now: TimeMs) -> AdmissionVerdict;      // 纯判定：不改 state
pub fn on_dispatch(state: &mut AdmissionState, now: TimeMs) -> Result<(), AxError>;
pub fn on_outcome(state: &mut AdmissionState, outcome: ProviderOutcome, now: TimeMs) -> Result<(), AxError>;
#[non_exhaustive] pub enum ProviderOutcome { Ok, RateLimited { retry_after_ms: Option<u64> }, Failed }
```

- 确定性 AIMD：RateLimited→interval 加倍（上限封顶；retry_after 在场则取其大者）；连续 `ADMISSION_OK_STREAK=8` 次 Ok→interval 减半（下限 `ADMISSION_MIN_INTERVAL_MS=250`）；并发上限 `ADMISSION_MAX_IN_FLIGHT=4`。三常量为 pub(crate) 数据面（provider 侧工程参数，非城策口径，不入 consts_policy；改须本 SPEC 同集）。
- `on_dispatch` 对越帽调用 fail-closed 后拦（E_BUDGET_EXHAUSTED 形）；`ADMISSION_MAX_INTERVAL_MS=60000` 封顶；快照回滚＝持前值（值语义，测试钉实）；结算溢出恒拒不回绕。
- 无时钟采样：`now` 恒入参；同一到达序列重演逐条同 verdict（验收 §2）。

### 8-7 gateway::market（形状 6 数据面＋快照）

```rust
#[non_exhaustive] #[derive(Default)] #[serde(rename_all = "snake_case")]
pub enum InputKinds { #[default] Text, TextImage }      // 这个模型收得下什么
pub struct ModelEntry { pub id: String, pub context_tokens: u64, pub input: InputKinds,
                        pub input_price: UsdMicros /* per 1M tokens */, pub output_price: UsdMicros,
                        pub cache_read_price: UsdMicros, pub cache_write_price: UsdMicros }
pub struct MarketSnapshot { /* version: u32、entries: BTreeMap<String, ModelEntry> —— 私有 */ }
impl MarketSnapshot {
    pub fn builtin() -> MarketSnapshot;                                  // 内置钉版目录（数据面）
    pub fn from_entries(version: u32, entries: Vec<ModelEntry>) -> Result<MarketSnapshot, AxError>;
    pub fn lookup(&self, id: &str) -> Option<&ModelEntry>;  pub fn version(&self) -> u32;
}
```

- **`input` 默认 `Text`，宽容读。** `selected_payload` 写 `input` 键，`read_choice` 读不到就当 `Text`——旧 Ledger 里的 `model_selected` 没有这个键，而重放一份旧历史不应该报错；默认取「只收文字」而非「收图」，因为猜错方向的代价不同：猜小了是一句拒绝，猜大了是 provider 的 400。内置目录里收图的行（现为 `claude-sonnet`）标 `TextImage`，`local` 保持 `Text`。
- 钉版回滚＝持前一快照即回滚（值语义，无 I/O）；快照落盘属 projection／config 面，本模块只管形与查询。价目恒整数微美元（判定路径禁浮点）。

### 8-8 gateway::cost（形状 1 判定函数）

```rust
pub struct CallCost { pub billed: UsdMicros, pub source: CostSource, pub usage: ModelUsage }
#[non_exhaustive] pub enum CostSource { Authoritative, PriceSheet }
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
pub struct Chosen<'b> { pub endpoint: &'b AttachedEndpoint, pub entry: &'b ModelEntry,
                        pub fallback: &'b Fallback }   // 一次取模型答的是「用哪个」与「不答时怎么办」两问
impl EndpointBook {
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError>;
    pub fn apply_payload(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError>;
    pub fn select(&self, tag: ModelTag, policy: &BuildingPolicy) -> Result<Chosen<'_>, AxError>;
    pub fn endpoints(&self) -> impl Iterator<Item = &AttachedEndpoint>;
    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry, &Fallback)>;
}
pub fn attached_payload(&AttachedEndpoint) -> Result<Payload, AxError>;      // endpoint_attached 唯一成形处
pub fn selected_payload(ModelTag, &str, &ModelEntry, &Fallback) -> Result<Payload, AxError>;  // model_selected 同上
```

- **`Fallback` 随选择入簿，缺键读作 `Fallback::None`。** `selected_payload` 写 `fallback_endpoint` 与 `fallback_model` 两键，且只在 `Then` 时写；`read_choice` 读不到就取 `None`。本键之前写下的每一条 `model_selected` 因此重放不变，而且重放出的是**默认值本身该是的那个**：一份旧历史不会因此获得一条谁都没设过的退路。

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

**`adapter_for` 住 `endpoint/adapter.rs`**：装配线（哪个 chosen 走 Native、哪个走 Endpoint）读的只有 chosen 与赎回闭包，故它是自由函数而非 `RunWorker` 方法——挂在 worker 上等于说装配需要整座城。`CALL_TIMEOUT_MS` 随它搬家（没人读的数字没人能辩护）。凭据簇只出 `resolver`（赎回闭包是凭据的形状）与 `dialect_headers`（兼容格式要的头是兼容格式的事，搬家留待日后：`dialect_headers` 住凭据是历史位置，此处只动 adapter 线）。

### 8-11 gateway::fallback（形状 2 值）

```rust
#[non_exhaustive] pub enum Fallback { None, #[non_exhaustive] Then { endpoint: String, model: String } }
impl Default for Fallback { fn default() -> Fallback { Fallback::None } }
impl Fallback {
    pub fn then(endpoint: &str, model: &str) -> Result<Fallback, AxError>;  // 空串在构造点拒（E_CONFIG_INVALID）
    pub fn endpoint(&self) -> Option<&str>;
    pub fn model(&self) -> Option<&str>;
    pub fn retreat(&self, now: TimeMs, admission: &AdmissionState) -> Retreat;
}
#[non_exhaustive] pub enum Retreat {
    Freeze,                                                     // None：这次 Run 就此冻住，理由入帐
    #[non_exhaustive] MoveTo { endpoint: String, model: String, not_before: TimeMs },  // Then：退避到此刻之后再动
}
pub fn retreat_payload(tag: ModelTag, from: &str, retreat: &Retreat, because: &AxError)
    -> Result<Payload, AxError>;   // provider_degraded 载荷，两臂各一句
```

**枚举逐变体（`Fallback`）**：`None` ——标签的 endpoint 不答时不另找人，Run 冻住，理由写进 Ledger；`Then` ——按 `admission` 退避之后把活挪到具名的 endpoint 与 model 上。**默认是 `None`**：在无人过问的情况下替一个人换掉他的模型，是一个默认值最不该做的那个决定——换模型改的是答案的质量、价钱与数据去处三件事，而这三件事恰好都是人自己挑 endpoint 时在挑的东西。

**枚举逐变体（`Retreat`）**：`Freeze` 不带理由，因为理由是调用方手里那个 `AxError`，抄一份就是给同一个事实立第二个权威；`MoveTo` 带 `not_before`，它就是 `AdmissionState::admit` 给出的 `Hold { until }`——退避的算法（AIMD、provider 自己的 retry-after 取大者）已经住在 §8-6，这里不重算一遍。`admit` 答 `Admit` 时 `not_before` 取 `now`。

**挪窝本身是一件事，不是一次静默的重试。** 两臂都产 `provider_degraded` 载荷（`E_PROVIDER` 的 carrier，record-only）：`action` 为 `freeze` 或 `move_to`，`tag`／`from`／`code`／`subject` 恒在，`move_to` 另带 `to_endpoint`／`to_model`／`not_before_ms`。载荷全整数，无浮点。**本 crate 不持 Ledger 句柄**（§7），故成形在此、入帐在装配层——这与 `attached_payload`／`selected_payload` 同一口径。

**为什么是值不是判定**：`Fallback` 的不变量（`Then` 的两个名字都非空）只在一个构造点上守，无 setter，这正是形状 2 的依据；`retreat` 是它身上的一个纯查询，不改自身、不采样时钟、不做 I/O。

**枚举变体的字段没有「私有」这一档。** Rust 里 `Then { endpoint, model }` 的两个字段随枚举一同公开，于是一个结构体字面量就能绕过 `then` 写出两个空串。封住它的是**变体上的 `#[non_exhaustive]`**：crate 之外写不出该字面量，只能走构造函数；`Retreat::MoveTo` 同理。枚举上的 `#[non_exhaustive]` 只管匹配不管构造，两道都要标。

**线上还没有设它的字段。** 设置面在设置页，经 `AttachEndpoint`／`SelectModel` 两帧；在那个字段上线之前，城里写下的每一条 `model_selected` 都带 `Fallback::None`。

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
#[non_exhaustive] pub enum AudioType { Webm, Ogg, Mpeg, Mp4, Wav }
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

**凭据只有一条路。** `Transcriber` 内部持一个真的 `Endpoint`（`DialectKind::OpenAi`、`Redemption::without_images`、`pricing: None`），认证头由 `Endpoint::authorize` 写——与聊天调用、与 `list_models` 探测是同一格兑付。头名由 `AuthSpec::for_dialect` 定（§8-9），登记面不再自己在 Bearer 与具名头之间选。**恒不为转写开第二个持凭据的地方**：两处持凭据就是两处会漏。

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

回合层组 `ChatRequest`（prefix 四段＋窗口历史）→admission.admit→endpoint.call（dialect 翻译＋兑付＋HTTP）→cost.settle→model_returned 载荷（usage＋billed）→attribution（memory 侧）摊回。credential 独立线：启动 probe→capture（Tainted::new 构造点驱动）→resolve（组请求末格）。

## 10 实现逻辑

dialect 先行（纯函数零依赖，golden 钉形）→endpoint 骨架（假 provider 服务回环测试）→credential（Vault 两适配器＋探测）→admission/market/cost（纯判定）。native 最后（复用 endpoint）。每步红先行：golden 未落前不写翻译分支。

## 11 边界枚举

空 messages（合法：首轮）；空 tools（不写 tools 键）；SSE 半流（S3 非流式先行，流式只加）；429 携 retry-after；usage 缺席（取 0，CostSource=PriceSheet）；权威计费额为 0（合法，免费档）；base_url 尾斜线；overrides 指向不存在的路径（创建）；OAuth profile 字段空串（oauth_begin fail-closed）；Vault 探测三候选全败（会话内存＋provider_degraded）；遮蔽写入；空串凭证（视同未配置）。

## 12 错误处理（逐码答「能否定义掉」）

- `E_PROVIDER`：不可定义掉——网络与对端是本 crate 的本质失败面；subject 写状态码与端点名，恒不含请求体。
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

`transcribe::recording::RECORDING_MAX_BYTES = 25 MiB`（§8-12：OpenAI 音频面自己印的上限，provider 侧工程参数，非城策口径，不入 `consts_policy`；改须本 SPEC 同集）与 `wire::BOUNDARY_SEED`（同上，分界线种子）；admission 三常量（§8-6）；`Fallback` 的默认值 `None`（§8-11，它是一条策略而非一个数字，改它须本 SPEC 同集）；oauth_profiles 表（§8-5，数据面即定义处）；market 内置目录（`builtin()`，S3 收录城内实际使用的模型行，价目随 Stage 复核）。三者全 pub(crate) 数据面，改动须本 SPEC 同集变更。

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

**认不出的帧跳过，缺失的结算帧不跳过。** provider 会加新的事件类型，一个人不该因为其中一个是新的就丢掉整次调用；但流在说明「为什么停」的那一帧之前结束，是 `Provider` 失败并且可重试——它和一个被截断的 body 是同一种失败，刻意不允许「保留已收到的增量」来补救：把不完整的回复当成完整的呈现出去，是这里唯一不能有的结局。

**机密楼宇的拒绝写一次。** 两扇门（`call` 与 `call_streaming`）都说同一句话，出自同一个 `confidential_refusal`——一条安全拒绝有两份拷贝，就是两个各自变软的机会。

### 8-14 gateway 目录化（形状：主类型居索引，方法按簇归文件）

`credential.rs`（988）→ `credential/vault.rs`（`Vault` 缝＋双后端＋`Persistence`／`Described`／`EnvReader`）／
`custodian.rs`（`Custodian`／`Captured`）／`oauth/`（`codec.rs` 编码、`flow.rs` 往返、`types.rs` 类型，
测试住 `flow/tests.rs`）；`dialect.rs`（512）→ `dialect/request.rs`（`sample_request` 提
`#[cfg(test)] pub(crate)` 供 response 面复用）／`response.rs`（strategies＋`proptest!` 住此，
快照搬 `dialect/snapshots/`）；`endpoint.rs`（639）→ `endpoint/config.rs`（类型＋`new`＋
` SecretResolver`＋loopback helpers 提 `#[cfg(test)] pub(crate)`）／`call.rs`／`model.rs`；
`router.rs`（559）→ `router/attached.rs`／`book.rs`／`payload.rs`（`Choice` 字段开
`pub(crate)`，`payload.rs` 无专属测试故无 tests 模）。
跨文件私有项开 `pub(crate)`，对外签名逐字节不变（`cargo public-api` 基线记定义位簇路径，
`memory` 同例）。
