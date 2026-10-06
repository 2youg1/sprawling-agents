-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::router::tuning

规定 `router::tuning`（`crates/gateway/src/router/tuning.rs`）：端点带着人给它定的规矩，与对端报出的模型事实。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::router::tuning`、`gateway::endpoint::header` 旁的测试守住。
-/

/-!
### 8-16 端点带着人给它定的规矩：`EndpointTuning` 与 `ModelFacts`（形状 7 值 ＋ 形状 4 适配器）

```rust
// Retries 即 kernel::Retries，缺席即 UntilHalted；本 crate 不导出别名
pub struct TuningDefaults { pub timeout_ms: u64, pub retries: Retries, pub stream_idle_timeout_ms: Option<u64> }
pub enum HeaderValue { Plain(String), Redeemed(SecretRef) }

pub struct EndpointTuning {
    pub accounts: Option<Vec<kernel::event::record::ProviderAccount>>, // 顺序决定优先级
    pub label: Option<String>,
    pub timeout_ms: Option<u64>,
    pub request_max_retries: Retries,
    pub stream_idle_timeout_ms: Option<u64>,
    pub extra_headers: Vec<(String, HeaderValue)>,
    pub overrides: Vec<(String, String)>,   // JSON pointer → 值的文本
    pub proxying: Proxying,
    pub max_in_flight: Option<MaxInFlight>,   // 缺席取这一类连接的默认（D21，§8-6）
}
impl EndpointTuning {
    pub const DEFAULTS: TuningDefaults;                        // 三个默认值的唯一住处
    pub fn call_timeout_ms(&self) -> u64;                      // 人设的，否则 DEFAULTS
    pub fn idle_timeout_ms(&self) -> Option<u64>;
    pub fn applied_overrides(&self) -> Vec<(String, Value)>;   // 文本读成 JSON 的唯一权威
}
pub struct AttachedEndpoint { …, pub tuning: EndpointTuning }
impl AttachedEndpoint { pub fn label(&self) -> &str }          // 缺省即 name

pub use kernel::event::record::ModelFacts;   // endpoint_probed 行里的值，结构归 kernel
pub(crate) fn facts_of(row: &Value) -> Option<ModelFacts>;   // endpoint::models：一行 /models 读成事实，无 id 即 None
impl Endpoint { pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError> }
```

- **人填的设置接的是 `EndpointConfig` 既有的 `extra_headers` 与 `overrides`**，不是第二套：「人填的那份设置」与「一周以后发出的那次调用」之间的存放处在 `AttachedEndpoint` 上，随 `endpoint_attached` 进账本、随重放回到书里。
- **快照中的账号字段保留 Option 标记**：`EndpointTuning.accounts` 为 None 时仍编码该字段，postcard 按字段顺序读取；账本的缺席由 `AttachedTuning` 保持，记录的旧 JSON 形状不变。
- **覆盖以文本入账**：账本不收浮点。`applied_overrides` 是文本变 JSON 的唯一一处，规则为「解析得出即那个 JSON，否则即它看上去的字符串」。
- **人写的头顶掉兼容格式自己的同名头**（按 ASCII 大小写不敏感比较），不是并列两行：两条 `anthropic-version` 是一条没有供应方承诺按谁的意思读的请求。
- **`stream_idle_timeout_ms` 是一次沉默的上限，不是整次应答的期限**。三层一个名字：线上、本 crate 与文案都叫 `stream_idle_timeout_ms`，装配层不翻译它。**实现与名字一致**：`reqwest::blocking` 把一个请求的 timeout 当整体期限执行（异步层的 total timeout 覆盖整个 body），所以流式请求发出前把 `timeout` 清成 `None`，正文在一条自己的线程上逐行读，调用侧用 `recv_timeout(idle)` 计时，每收到一行重置。缺席则取 `timeout_ms`：一条没单独设过界的流也不允许永远安静。**代价写在这里而不是藏着**：对侧不说话又不断连时，那条读线程阻到对侧断连为止；结束这次调用是人要的，结束那条连接是对侧的。拒词报出越过的那个界（`no byte arrived for N ms`）而不是传输的原句。**败给的方案**：把线上字段改名 `stream_deadline_ms`——那会让一段写了六分钟的长回答在五分钟整被切，而那正是人抱怨的那件事。
- **重试只有一个家，就是 `Retries`**。`Endpoint::call` 一次调用一个来回；「人没填」只有一个意思：缺席就是 `Retries::UntilHalted`，读者拼不出第二种答案。没有刹车的地方（设置页上的探测，人正等着，`Halt` 按不下去）读 `Retries::without_a_brake()`，它把 `UntilHalted` 兑成 **1 次**重试——这个读法住在设置自身上，而不是调用点的第二个默认值。账本里仍然只写已设的数字，缺键即无上限（`Retries::stated()`／`Retries::of()` 一对）。
- **三个默认值住 `EndpointTuning::DEFAULTS`**：`timeout_ms = 120_000`、`retries = UntilHalted`、`stream_idle_timeout_ms = 无`。`adapter_for` 读 `call_timeout_ms()`；表单的占位符由 `Query::Config` 带回这三个值，客户端不手抄一份。
- **自定义头的值是一个类型，不是一段依前缀猜的文本**。`HeaderValue::Plain` 逐字发出，`HeaderValue::Redeemed` 走与 `authorize` 同一格的兑付；写入侧（`HeaderValue::parse`）对 `Plain` 跑一次 `kernel::secret::scan`，命中即拒，拒词不复述那个值。**一条规则**：本城读得出的金库引用就是引用（`SecretRef::parse` 是该语法的唯一读者），其余一律是字面量。账本里写的是 `spelled()`，对 `Redeemed` 即那条引用，所以 `endpoint_attached` 仍然可导出。重放一条没有经过这项检查的记录时，读不出引用的值仍读作 `Plain`：键已在账本里，在重放处丢掉它只会让人的端点不声不响地换一种叫法。
- **`list_models` 读出每一行真正说了的东西**。OpenAI 形的 `/models` 一行里除 id 之外有什么由供应方决定：`context_length` 与 `max_completion_tokens`、同样两项嵌在 `top_provider` 之下、模态写在 `architecture` 里、绝大多数什么都不写。读法因此是「一组问题，各自由第一个带着它的键作答，没有键带着它就缺席」。**城不补零、不补默认、不补猜测**：补出来的数字会盖过真正计费的那个。
- **价格按供应方自己的文本原样携带**。单位也是供应方的——按 token 还是按百万 token，按美元还是按美分——而一个没人能拿去对账单的换算值，比供应方印出来的那串字符更糟。
-/
