-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::dialect

规定 `dialect`（`crates/gateway/src/dialect.rs`）：canonical 会话类型与各兼容格式之间的纯函数翻译，及 anthropic、openai、mismatch、images 四个同层模块。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::dialect::request`、`gateway::dialect::response`、`gateway::anthropic::stream`等 旁的测试守住。
-/

/-!
### 8-1 gateway::dialect（形状 1 判定函数族）＋anthropic／openai／mismatch

城内规范会话类型住 `kernel::model`（缝上类型）；本模块只做 canonical↔wire 翻译，纯函数、无 I/O、无状态。

**两家 provider 的字段知识各住各家。** 切缝是**变化的理由**：一家 provider 改了它的形状，只有它那一个文件动；而本模块顶上那句「改之前先读 provider 自己的文档」只有跟它指的那堆字段同居一处才真的被读到，所以两张文档链接表各自跟着它的 dialect 走。
- `dialect` 的每个入口是一条 `match kind`，对 `DialectKind` 穷尽；不认得的 dialect 恒拒而不拿较近的那一家近似。跨 dialect 的断言（两向往返、两种强度拼写、float 拒收）留在这里，因为它们测的就是路由的契约。
- `mismatch` 是两家共用的四个读取器（`require`／`as_str`／`tokens_or_zero`／`payload_from`）与拒词（`mismatch`／`mismatch_found`／`stream_cut`）。**依赖是单向的**：dialect → 两家 → mismatch，谁都不回头指。
**两家各自把流的拼接放进自己的 `stream.rs`。** 切的仍是变化的理由：`increment_of`／`settled` 回答的是「一串 SSE 帧如何合成一份完整答案」，与「一个请求如何写上线」是两件事，且两家的流帧形状各自变。归 `anthropic/stream.rs` 与 `openai/stream.rs`，父文件各以一行 `pub(crate) use stream::{increment_of, settled};` 再导出。

- 只属一家的东西跟着它：`empty_answer`（空答案）、`joined_text`与 `effort_field` 入 openai；`role_str`、`stop_from`／`stop_str`、`block_wire`／`block_from`、`effort_fields` 入 anthropic。

**每一条形状不匹配都带出路，而空答案不是形状不匹配**。`AxError` 的契约写着 `recovery` 必须是可直接执行的信息，所以：

- `mismatch()` 统一携一句出路（对一句 dialect 选错与 base url 写错），每个调用点共用这一句。
- **`choices` 为 `null` 不是形状问题**，单独报 `E_PROVIDER`：信封是对的、字段都在、只是答案被丢了。OpenAI 兼容的托管端点可能在 `max_tokens` 高于所选模型的上限时**既不拒也不答**，回 HTTP 200 携 `"choices": null` 与全零 usage；同一家端点的上限因模型而异，故**恒不把某个上限写进代码**：那是对侧的数字，写下来就是第二个会漂的权威。拒词只指向该改的那一项（max output tokens），并标 `retriable`。

```rust
pub enum DialectKind { Anthropic, OpenAi, OpenAiResponses }   // 定义在 kernel::model::wire；第三支见 §8-20
pub fn request_wire(kind: DialectKind, req: &ChatRequest, images: &ImageBytes, spelling: ChatSpelling) -> Result<serde_json::Value, AxError>;
                                    // spelling 只有 chat 面读；另两面各只有一种拼法（§8-17 厂商拼法列）
pub fn response_from_wire(kind: DialectKind, wire: &serde_json::Value) -> Result<ChatResponse, AxError>;
#[cfg(test)]
pub(crate) fn response_wire(kind: DialectKind, resp: &ChatResponse) -> Result<serde_json::Value, AxError>;
                                    // 响应侧的反方向只供测试：造 provider 回复、证往返（wire→canonical→wire 等值）；生产里没有调用者，所以不编进发行的二进制
```

- **保三样**：①断点位——canonical 的 `cache: true` 标记翻到 Anthropic 侧＝`cache_control{type:"ephemeral"}`，system 块逐块原位，被标记的消息落在它的最后一块上（缓存区到这一块结束为止）；断点放在哪由 `runtime::prefix::BreakpointPlan` 决定，兼容格式只拼写；OpenAI 侧无显式断点（供应商缓存是隐式前缀匹配），翻译**记录性丢弃**（文档声明，不静默）；②工具形状——`ToolDef{name, description, input_schema}`↔Anthropic `tools[]`／OpenAI `tools[{type:"function",function:{…}}]`，逐字段；tool_use↔tool_calls（id/name/args 无失）；③usage——Anthropic `usage{input_tokens,output_tokens,cache_read_input_tokens,cache_creation_input_tokens}`／OpenAI `usage{prompt_tokens,completion_tokens,prompt_tokens_details.cached_tokens}`→`ModelUsage` 四整数字段加 `dialect`，缺失字段取 0。`input_tokens` 在每个兼容格式下都是整个 prompt（`crates/kernel/Spec.lean` 的 `ModelUsage`）：OpenAI 两面的 `prompt_tokens`／`input_tokens` 本就含缓存部分，照抄；Anthropic 的 `input_tokens` 只数未命中缓存的部分，解析时加上 `cache_read_input_tokens` 与 `cache_creation_input_tokens`，写回线上时减回去。`cost::settle` 按输入价计的是 `input_tokens` 减去两个缓存数。
- 未知 wire 字段：请求侧不产（我们只写自己声明的字段＋overrides）；响应侧忽略未知键、缺必需键报 `E_WIRE_MISMATCH`（subject 写键路径）。
- canonical 枚举（Role／StopReason／ContentBlock）是闭的，本模块对每一支写出真实的臂；新增一支即在两种兼容格式里同时编译失败，这正是要的，所以没有 fail-closed 通配臂；wire JSON 键序＝serde_json BTreeMap 字典序（确定性，对端语义无关）。
- `E_ENDPOINT_DIALECT_UNSUPPORTED`：DialectKind 之外的兼容格式请求（wire 探查失败）；本模块两码之外不新增。

**思考块与思考强度的两侧翻译**

保的第四样：**思考块。**Anthropic 侧两向逐字，`thinking`（携 `signature`）与 `redacted_thinking`（携 `data`）各自原位往返；canonical→wire→canonical 与 wire→canonical→wire 两条往返均逐字节相等。理由是 provider 官方规定而非我们的偏好：改动即 400，报文指名这两个块 cannot be modified（`crates/kernel/Spec.lean` §8-24 引原文）。

Chat 面的思考块由主机的拼法决定（`ChatSpelling.reasoning`，§8-17）。OpenAI 自己的规格里 assistant 消息没有推理字段，未登记的主机因此**记录性丢弃**思考块（与断点位同一规则：文档声明，不静默）。DeepSeek、Moonshot／Kimi、智谱与 OpenRouter 的文档则要求或接受把上一轮的推理原样放回 assistant 消息的 `reasoning_content`：DeepSeek 的思考模式默认开启，文档写明带 `tools` 的请求必须回传全部历史 `reasoning_content`，否则答 400——本城的每一次派活都带工具，所以在这几家主机上丢弃思考块就是第二轮必然失败。回传的只有思考文本，`signature` 恒不上这条线。canonical 记录不受影响，重放仍能从 canonical 重推出当时实发字节（dialect 是纯函数，拼法是它的参数）。入向：`reasoning` 与 `reasoning_content` 两种拼法都读成签名为空的 `Thinking`。

强度映射表（`ChatRequest.effort`，缺席即不写字段）：

| `Effort` | Anthropic wire | OpenAI wire |
|---|---|---|
| 缺席（`Option::None`） | 不写（等价于 `high`，官方明言） | 不写 |
| `None` | `thinking:{type:"disabled"}` | `reasoning_effort:"none"` |
| `Low`／`Medium`／`High`／`XHigh`／`Max` | `output_config:{effort:"…"}` | `reasoning_effort:"…"` |

两种兼容格式都拼得出全部六级，**差别是「不思考」写在哪个字段**：Anthropic 的 `output_config.effort` 只收五级，无 `none`。Messages API 参考页里 `effort` 只挂在 `output_config` 之下，顶层写 `effort` 是一个对侧不认的字段。Chat 面的 `reasoning_effort` 是 OpenAI 规格（`openai-openapi` 的 `CreateChatCompletionRequest`）的拼法，DeepSeek、Gemini 的兼容面、xAI、Moonshot 与 OpenRouter 的文档都收它；`reasoning:{effort}` 是 OpenRouter 自己的统一参数，也是只有它的文档写着收 `max` 的拼法，所以只有 `openrouter.ai` 一行按它拼。responses 面的 `reasoning:{effort}` 不变。`Effort` 是闭的，映射对每一级写出真实的臂，新增一级即在两种兼容格式里同时编译失败。

**缓存后果写在这里，因为它是选型理由**：官方排错文档记明「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」。故强度住 `FrozenConfig`（`crates/kernel/Spec.lean` §8-22），Run 内不可变；本模块只负责把已冻结的值翻上线。

**两种缓存失效是两件事，不要合成一件。** ① **供应商侧的 message cache breakpoints**：同一个前缀字节不变，仅因请求形状变了（思考模式、effort、思考预算），对方就把已缓存的**消息**断点作废——本模块与 §8-19 守的是这一条，故强度与模型在 Run 内不得改；② **本城冻结的 system 前缀本身**：四段字节任一段变了，后续请求读到的就是另一份前缀，缓存自然落空——守它的是 runtime 侧的重算对拍（`FrozenPrefix::verified_segment_hashes`，本 crate 不参与）。一句区分：**effort 变了前缀没变，缓存仍会失效；前缀变了哪怕形状一字未改，缓存也已不同。** 顾问之所以可以动窗口而不能动模型与 effort，正是因为窗口属易变半，而这两件属①。

**两条 wire 上的图**

```rust
// gateway::dialect::images（形状 2 value）
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
- **本兼容格式多一项记录性丢失：tool 消息拿不了图**。Chat Completions 的 `role:"tool"` 只收字符串 `content`，所以 tool_result 的 `attachments` 不随它走，而是落到紧跟其后的一条 user 消息里。图没丢，丢的是「这张图是那次工具调用的结果」这层归属；与断点位、思考块同一规则，**文档声明，不静默**，写在 `openai.rs` 顶上那张丢失清单里并由测试钉住。
- **base64 依赖**：`base64 0.22` 本就在 `Cargo.lock` 的图里（reqwest／git2 一系已携），直接命名不向锁里添包；自己写一份编码器才是新权威。
-/
