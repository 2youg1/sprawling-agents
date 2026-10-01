-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::router

规定 `router`（`crates/gateway/src/router.rs`）：已登记端点的簿与每个标签的选择；一次取模型只答一问。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-9 gateway::router（形状 7 projection）

```rust
pub struct AttachedEndpoint { pub name, pub base_url, pub dialect: DialectKind,
                              pub auth: AuthSpec, pub models: Vec<String>,
                              pub probed: bool }   // 这份 models 是问出来的（true）还是人报的（false）
impl AttachedEndpoint {
    pub fn is_local(&self) -> bool;          // 与 client_for 绕开代理同一依据（reach::is_local）
    pub fn has_credential(&self) -> bool;    // 关于凭证，金库外只能回答这一问
    pub fn chat_url(&self) -> String;        // base_url ＋ 该兼容格式自己的路径
    pub fn models_url(&self) -> String;
}
pub struct EndpointBook { /* 私有：endpoints（各带自己的 Transport）、chosen */ }
pub struct Chosen<'b> { pub endpoint: &'b AttachedEndpoint, pub entry: &'b ModelEntry,
                         pub(crate) transport: &'b Transport }   // 这个 endpoint 共用的客户端（§8-3）
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

- **一次取模型只答一问**：用哪个模型、在哪个端点上。「不答时怎么办」不是本 crate 的问题（§8-11）。
- **三种不答，三个码**：这一类标签没人选过＝`E_MODEL_UNCHOSEN`（出路：去设置页接供应方、选模型）；选过的端点已不在＝`E_CONFIG_INVALID`；confidential 楼的选择会让字节离开运行中的机器＝`E_GATE_DENIED`。「没选」单独成码，因为它是一座新城的第一个状态，客户端要按码给「去设置」，而 `E_CONFIG_INVALID` 在别处还答「会话中途换了模型」，那里的出路是开新对话。

- **为何不是 duty pool**：多 Agent 功能未成形之前，职责池没有消费者，而没人读的权威只会漂。降为 `ModelTag` 两值枚举（`Main`／`Digest`）：**标签因为有人按它取模型而存在**，新增一个标签的前提是先有调用方。
- **两个入口一个读者**：`apply`（重建路径，手里是 record）与 `apply_payload`（写入路径，手里是刚要写的 payload）共用同一套载荷读取，于是「写者以为的」与「重建得到的」不可能分岔。
- **confidential 在选型点再守一次**：非回环 endpoint 对 confidential 楼恒拒（`E_GATE_DENIED`）。`gateway::endpoint` 的兜底拒同期改为**按本地性判定**（而非一律拒）：规则是「字节不出运行中的机器」，不是「不准用这个类型」；否则一个回环的 Anthropic 服务器会被误拒。
- **路径归兼容格式**：人输入 base URL（provider 文档就是那么印的），`messages`／`chat/completions`／`models` 由兼容格式拼。这与 `EndpointConfig.base_url`「完整端点 URL、不拼路径」并不矛盾：适配器保持字面，拼路径的是上层登记面。
- **`probed` 是这份 models 的来源，不是端点的健康度**：`true` ＝ `GET .../models` 答了，登记的 id 是对端自己说的；`false` ＝ 探测失败而人自己报了型号，城照登。载荷里缺 `probed` 键读作 `true`，于是没有这个键的 `endpoint_attached` 重放不变。
- **`AuthSpec::for_dialect` 是凭证头的唯一产地**：`AuthSpec::for_dialect(dialect: DialectKind, reference: SecretRef, header: Option<String>) -> AuthSpec`，纯函数，住 `endpoint/auth.rs`。人显式填的头名恒胜（`Header`）；否则 Anthropic → `Header{name:"x-api-key"}`，OpenAI 及其余 → `Bearer`。**它不住 `endpoint/config.rs`**：「凭证头归兼容格式」是一个可以自己站着的概念；`AuthSpec` 类型本体留在 config.rs，因为搬它会让同一个名字在 crate 内多出一条 `pub(crate) use` 路径。登记面（`accounting::worker::credentials::endpoints::endpoint_of`）不自己在 Bearer 与具名头之间选，否则「Anthropic 用哪个头」在城里有两个权威，而漂开的总是没人看的那个。
-/

/-!
### 8-11 不设备用端点

一个标签只选一个端点与一个模型；`selected_payload` 直接收 `ceiling_from: Option<CeilingSource>`。线上没有设备用端点的字段，而一个在生产里只有一臂的「两臂的值」，另一臂就是给一个还不存在的设置面预留的权威。**带 `fallback_endpoint`／`fallback_model` 键的记录照样读得回**：`read_choice` 不取这两个键，与它对待任何本书不拥有的键同一口径，测试钉住「带着这两个键的 `model_selected` 仍读成它所述的那次选择」。

**重开的参数**：设置面上有人能为一个标签指定备用端点与备用模型，且 `runtime` 侧有一处在失败时读它。那时备用端点与 §8-6 的准入一起设计，因为何时改投备用端点只能由退避节奏回答。
-/
