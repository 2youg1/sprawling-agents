-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::endpoint

规定 `endpoint`（`crates/gateway/src/endpoint.rs`）：自写线格式的 provider 客户端，`kernel::Model` 的生产适配器。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
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
- **半流中断**：SSE 流截断（连接断／不完整事件）＝`E_PROVIDER`，恒不产部分 ModelReturn；流式读法见 §8-13。
- **无暗重试**：重试是 watchdog 的决策（上限的唯一表示是 `Retries`，§8-16），endpoint 一次调用恰一次 HTTP 往返；幂等由调用方 IdemKey dedup 看守。
- base_url＝完整端点 URL（逐字段哲学，不拼路径）；`EndpointConfig.pricing: Option<ModelEntry>` 让结算在适配器内完成，`ModelReturn` 因此携 usage／stop／billed 入账（kernel-SPEC §8-24；权威额线上无标准槽位，现行恒 PriceSheet 源）；TLS 取 rustls，加密后端由 `reach::tls` 在造客户端前装好（§8-15）。
- `.expose(` 白名单（`tools/xtask/src/secret.rs` 的 `EXPOSE_WHITELIST`，全表以它为准）：gateway 侧的合法出现点只有 `endpoint/call.rs`（端点调用）。**两种凭证在同一句里写上线**：`authorize` 既写注册带的那条，也写人在自定义头里放的 `HeaderValue::Redeemed`；三个请求写入点（`call`、`stream`、`list_models`）都只调它，谁都不自己遍历 `extra_headers`。
- **`Endpoint` 的字段不出 `endpoint/`**：`config`／`client`／`redemption` 是 `pub(super)`；`transcribe` 走 `post_bytes` 与 `model()`。于是「一次 POST 如何发出、非 2xx 如何变成 `AxError`、对侧正文如何不被回显」在本 crate 里只有一份答案。转写面仍保留它自己那句恢复语（`rewrite_recovery`）：线上出了什么事是端点的事，人接下来能做什么是设施的事。

**图的兑付面与两句拒绝**

两型一构造面住 `endpoint/redemption.rs`（切的理由是职责：「一个端点在线上兑什么」与「一个端点配成什么样」各自变化）：

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
-/
