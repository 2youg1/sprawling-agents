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

城内规范会话类型住 `kernel::model`（缝上类型）；canonical↔wire 翻译入口是纯函数，无 I/O。crate 内的 `StreamFrames` 持有流帧，保留与 EOF 结算契约由 `crates/gateway/spec/Endpoint/Stream.lean` §8-13 规定。

**两家 provider 的字段知识各住各家。** 切缝是**变化的理由**：一家 provider 改了它的形状，只有它那一个文件动；而本模块顶上那句「改之前先读 provider 自己的文档」只有跟它指的那堆字段同居一处才真的被读到，所以两张文档链接表各自跟着它的 dialect 走。
- `dialect` 的每个入口是一条 `match kind`，对 `DialectKind` 穷尽；不认得的 dialect 恒拒而不拿较近的那一家近似。跨 dialect 的断言（两向往返、float 拒收）留在这里，因为它们测的就是路由的契约。
- `mismatch` 是两家共用的四个读取器（`require`／`as_str`／`tokens_or_zero`／`payload_from`）与拒词（`mismatch`／`mismatch_found`／`stream_cut`）。**依赖是单向的**：dialect → 两家 → mismatch，谁都不回头指。
**两家各自把流的拼接放进自己的 `stream.rs`。** 切的仍是变化的理由：`increment_of`／`settled` 回答的是「一串 SSE 帧如何合成一份完整答案」，与「一个请求如何写上线」是两件事，且两家的流帧形状各自变。归 `anthropic/stream.rs` 与 `openai/stream.rs`，父文件各以一行 `pub(crate) use stream::{increment_of, settled};` 再导出。

- 只属一家的东西跟着它：`empty_answer`（空答案）、`joined_text` 入 openai；`role_str`、`stop_from`／`stop_str`、`block_wire`／`block_from` 入 anthropic。思考字段两家都不写（下文「思考块与思考强度的两侧翻译」）。

**每一条形状不匹配都带出路，而空答案不是形状不匹配**。`AxError` 的契约写着 `recovery` 必须是可直接执行的信息，所以：

- `mismatch()` 统一携一句出路（对一句 dialect 选错与 base url 写错），每个调用点共用这一句。
- **`choices` 为 `null` 不是形状问题**，单独报 `E_PROVIDER`：信封是对的、字段都在、只是答案被丢了。OpenAI 兼容的托管端点可能在 `max_tokens` 高于所选模型的上限时**既不拒也不答**，回 HTTP 200 携 `"choices": null` 与全零 usage；同一家端点的上限因模型而异，故**恒不把某个上限写进代码**：那是对侧的数字，写下来就是第二个会漂的权威。拒词只指向该改的那一项（max output tokens），并标 `retriable`。

```rust
pub enum DialectKind { Anthropic, OpenAi, OpenAiResponses }   // 定义在 kernel::model::wire；第三支见 §8-20
pub fn request_wire(kind: DialectKind, req: &ChatRequest, images: &ImageBytes, spelling: ChatSpelling) -> Result<serde_json::Value, AxError>;
                                    // chat 面读整行；responses 面读 responses_tool_pictures；messages 面不读（§8-17）
pub fn response_from_wire(kind: DialectKind, wire: &serde_json::Value) -> Result<ChatResponse, AxError>;
pub fn increment_of(kind: DialectKind, frame: &serde_json::Value) -> Option<Increment>;          // §8-13
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

思考字段不由 dialect 写。`ChatRequest.effort` 是城存下或这一次明说的档，实发什么由思考档的梯子（`crates/gateway/spec/Provider/Thinking.lean` §8-39）按这个（Endpoint，模型）的 offer 算出一个 `Ask`（一档，或「开启思考」，或缺席），再由 `provider::thinking::encoding` 按这一面与主机的 `EffortField`（§8-17）写进正文顶层（gateway D35）：

| 这一面（`EffortField`） | `Ask::Level(l)` | `Ask::On` | 主机 |
|---|---|---|---|
| messages | `output_config:{effort:l}` | `thinking:{type:"adaptive"}` | Anthropic 与各家 Anthropic 兼容面 |
| responses | `reasoning:{effort:l}` | 不写 | OpenAI、xAI、DeepSeek 的 responses 面 |
| chat，`ReasoningEffort` | `reasoning_effort:l` | 不写 | OpenAI、xAI、Gemini 兼容面、未登记主机 |
| chat，`ReasoningObject` | `reasoning:{effort:l}` | `reasoning:{enabled:true}` | OpenRouter |
| chat，`ThinkingToggle` | `thinking:{type:"enabled"}` 与 `reasoning_effort:l` | `thinking:{type:"enabled"}` | DeepSeek、Moonshot／Kimi、智谱 |
| chat，`EnableThinking` | `enable_thinking:true` 与 `reasoning_effort:l` | `enable_thinking:true` | 阿里 DashScope |

- **缺席即不写**：offer 什么都不提供时梯子答缺席，正文里没有任何思考字段，由供应方按模型取它自己的缺省；这不再等价于某一档，因为 Anthropic 各模型的缺省已经不同（多数为 `high`，Opus 5.5 与 Haiku 5.5 为 `medium`，<https://platform.claude.com/docs/en/build-with-claude/effort>）。
- **「关闭思考」恒不写**：`Effort::None` 只为读回旧账保留，梯子恒不提供它，于是它走默认规则；DeepSeek、Kimi、智谱与 DashScope 的文档里关闭思考写在 `thinking.type` 或 `enable_thinking`，从前在 chat 面上写的 `reasoning_effort:"none"` 是这几家文档都不认的拼法。
- **一档写成哪个词只有 `Effort::as_str` 一处**：七个词与 serde 拼写相同，`minimal` 照原词写；发不发某一档给某个模型归 offer，不归兼容格式。
- Messages API 参考页里 `effort` 只挂在 `output_config` 之下，顶层写 `effort` 是一个对侧不认的字段。Chat 面的 `reasoning_effort` 是 OpenAI 规格（`openai-openapi` 的 `CreateChatCompletionRequest`）的拼法；`reasoning:{effort}` 与 `reasoning:{enabled}` 是 OpenRouter 自己的统一参数（<https://openrouter.ai/docs/guides/best-practices/reasoning-tokens>）。

**缓存后果写在这里，因为它是选型理由**：官方排错文档记明「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」。故强度住 `FrozenConfig`（`crates/kernel/Spec.lean` §8-22），Run 内不可变；实发的档由 §8-39 从已冻结的值与登记的 offer 算出，两者在 Run 内都不变。

**两种缓存失效是两件事，不要合成一件。** ① **供应商侧的 message cache breakpoints**：同一个前缀字节不变，仅因请求形状变了（思考模式、effort、思考预算），对方就把已缓存的**消息**断点作废——本模块与 §8-19 守的是这一条，故强度与模型在 Run 内不得改；② **本城冻结的 system 前缀本身**：四段字节任一段变了，后续请求读到的就是另一份前缀，缓存自然落空——守它的是 runtime 侧的重算对拍（`FrozenPrefix::verified_segment_hashes`，本 crate 不参与）。一句区分：**effort 变了前缀没变，缓存仍会失效；前缀变了哪怕形状一字未改，缓存也已不同。** 顾问之所以可以动窗口而不能动模型与 effort，正是因为窗口属易变半，而这两件属①。

**两条 wire 上的图**

```rust
// gateway::dialect::images（形状 2 value）
pub struct ImageBytes(BTreeMap<String, String>);   // 键＝Locator 的规范拼写，值＝标准 base64
impl ImageBytes {
    pub fn insert(&mut self, picture: &ImageRef, bytes: Vec<u8>, max_side: Option<NonZeroU32>) -> Result<(), AxError>;
                                    // 仅超限图片先转换；插入时编码一次，原始字节随即释放
    pub fn len(&self) -> usize;  pub fn is_empty(&self) -> bool;
    pub(crate) fn encoded(&self, at: &Locator) -> Result<&str, AxError>;  // 借用已编码文本
}
// request_wire 收 &ImageBytes，签名见上文 §8-1 那一块（第四个参数是 ChatSpelling）
```

- **dialect 仍是数据的纯函数**。字节不在这里取：`ImageBytes` 是参数，不是一个 store 句柄。同一份 `(ChatRequest, ImageBytes)` 永远翻出同一串字节，重放因此仍能重推当时实发的请求。`insert` 的转换同样是纯计算：它只读调用方交来的字节，不写 CAS，也不访问文件或网络。
- **为何以规范拼写和编码文本为键值**：`Locator` 不实现 `Ord`（它内含的 `Range` 也不），而给 `kernel::locator` 加一对 derive 是在另一个模块的文件上改公共面。以规范拼写为键让排序只取决于内容；值在插入时编码，使原始字节不必留到翻译结束。「解不出字节」这条失败路径归值本身拿着，而不是散在两家 dialect 里。
- **位置**：`dialect/images.rs` 与 `mismatch` 同层——两家都读、谁都不回头指，所以依赖仍是单向的。
- **locator 解不出字节即 `E_WIRE_MISMATCH`**（fail-closed）：一张描述存在、字节不在的图，发上线就是一个模型看不见的空位。
- **Anthropic 侧**：Image 块→`{type:"image", source:{type:"base64", media_type:…, data:…}}`；带 `attachments` 的 tool_result 的 `content` 由字符串改写为块数组 `[{type:"text",text},{type:"image",…}…]`（无附件时仍写字符串，既有 golden 字节不变）。两处形状都是 provider 自己的：<https://platform.claude.com/docs/en/build-with-claude/vision>。
- **OpenAI 侧**：user 消息的 `content` 由字符串改写为部件数组 `[{type:"text",text},{type:"image_url",image_url:{url:"data:<mime>;base64,…"}}]`。
- **工具产出的图随主机与接口面走**（`ChatSpelling.chat_tool_pictures`／`responses_tool_pictures`，§8-17）。`FollowingUser` 把 tool_result 的 `attachments` 落到紧跟其后的一条 user 消息里——图没丢，丢的是「这张图是那次工具调用的结果」这层归属；与断点位、思考块同一规则，**文档声明，不静默**，由测试钉住。`InToolContent` 把图留在 tool 结果上：chat 面写 `role:"tool"` 的部件数组（含 `image_url`），responses 面写 `function_call_output.output` 的部件列表（含 `input_image`）。DeepSeek 的 chat 面是 `FollowingUser`，responses 面是 `InToolContent`；Kimi 的 chat 面是 `InToolContent`；未登记主机两面都取 `FollowingUser`。
- **base64 依赖**：`base64 0.22` 本就在 `Cargo.lock` 的图里（reqwest／git2 一系已携），直接命名不向锁里添包；自己写一份编码器才是新权威。

/-! D66 工具产出的图按主机与接口面选择位置，DeepSeek 的 chat 与 responses 不共用一档

决定：`ChatSpelling` 以 `chat_tool_pictures` 与 `responses_tool_pictures` 分别回答两面。`FollowingUser` 把 tool_result 的附件写进紧随其后的 user 消息；`InToolContent` 把附件写进 chat 面的 `role:"tool"` 部件数组，或 responses 面的 `function_call_output.output` 部件列表。`DOCUMENTED` 与未登记主机两面都取 `FollowingUser`。`api.deepseek.com` 的 chat 面取 `FollowingUser`、responses 面取 `InToolContent`；Moonshot／Kimi 三台主机 `api.moonshot.ai`／`api.moonshot.cn`／`api.kimi.com` 的两列都取 `InToolContent`；本表没有为它们登记 responses 面，responses 列不是对该面上游能力的新声明。人自己附上的 `Image` 块仍写在 user／developer 消息里，与拼法无关。Anthropic Messages 面不读这两列：图本来就写在 `tool_result` 的内容块里（见上文 Anthropic 侧）。

理由：DeepSeek 的 Vision 指南明确限制「Images are supported in `user` messages only」，所以 chat 面的 tool 消息不能携带图片；其 Responses 指南另写明 `input_image` 可出现在 `function_call_output`／`custom_tool_call_output` 的 `output` 中（<https://api-docs.deepseek.com/guides/vision>、<https://api-docs.deepseek.com/guides/responses_api#image-input>）。这是同一主机两个接口面的两种事实，不能合成一档。Kimi K2.6 的多模态工具示例把 `image_url`／`video_url` 部件直接作为 `role:"tool"` 的 `content` 数组交回（<https://platform.kimi.ai/docs/guide/kimi-k2-6-quickstart>），三台 Moonshot／Kimi 主机共用这条 chat 拼法；它们没有在本表登记 responses 面，responses 列保持同一常量原有的值，不构成一次上游能力调查。OpenAI chat 的 tool content 只收文本，xAI 的图片文档只示范 user 消息，二者保持 `FollowingUser`；Anthropic 的 vision 页写明 `tool_result` 内可嵌 `image` 块，由 Anthropic dialect 直接写。

被否：①整个主机只设一档——DeepSeek 两面支持的图片位置不同；②一律 `InToolContent`——DeepSeek chat、OpenAI chat 与未登记主机会拒绝带 `image_url` 的 tool content；③图一律剥成占位文字——能看见的模型被弄瞎。重开参数：厂商文档改变某一面的图片位置时，只改对应面的那一列，并以该面的真实请求回归测试守住。
-/

/-! D67 供应方的单边像素上限在请求出口执行，CAS 与 canonical 图片仍指原图

决定：`ModelPreset.image_max_side` 是厂商文档陈述的单边像素上限，经 `model_for` 的同一选行规则作答。`Endpoint::pictures_for` 把这个可选值交给 `ImageBytes::insert`；没有上限，或 `ImageRef` 的两边都不超限，原始字节直接编码。超限图片先完整解码，应用 EXIF 方向，再保持宽高比把最长边缩到上限，以原格式编码。PNG／WebP／GIF 动画不以丢弃后续帧的方式通过这条路径：超限动画以 `E_INVALID_ARGS` 拒绝，恢复语让人选择静态帧或已经合规的图片。解码、方向读取或编码失败同样以 `E_INVALID_ARGS` 报出 locator 与上限，不交给供应方猜。

理由：DeepSeek Vision 把单边上限写为 8192 px（本城一次最多四张图，因此文档在十五张图起使用的 4096 px 档不可达）；只移动图片在消息里的位置不能让一张超限图片变得可读。转换留在请求出口，城市的历史、CAS 内容地址与其他 provider 的原图路径都不变；供应方随后仍会按自己的规则缩放用于推理，因此这里只移除它拒绝输入的那个条件。被否：①在 `read` 时改写成缩图——所有 provider 被迫共用一家的限制，CAS 也不再是原图；②超限即拒而不给转换——一张内容完整、供应方本来就会缩放的图片被无谓地排除；③动画静默取首帧——模型看到的是另一份内容。重开参数：动画保真转换有生产场景与测试时，可让动画逐帧走同一上限；其他模型文档给出不同上限时，在对应模型行填值。
-/

/-! D65 同一 locator 在一次请求里只兑付并编码一次，原始字节不随整次翻译驻留

决定：`pictures_in` 按 locator 规范拼写去重并保留第一次出现的顺序，`pictures_for` 对不同图片的数量执行 `IMAGES_PER_TURN` 准入，同一 locator 只兑付一次；`ImageBytes::insert` 立即写成标准 base64，`encoded` 借用这段文本。重复出现的图片在 wire 上仍各守原位置，去重的是请求上限的计数对象、CAS 读取与编码工作，不改变图片块的数量或顺序。

理由：同一张图重复引用不应重复读盘；四张图的原始字节也不必等所有图片都读完后仍与全部 base64 同时驻留。被否：①把原始字节保留到 `encoded` 每次现编——重复块重复付 CPU，峰值同时持有两族字节；②跨请求 LRU——单次请求的不同图片数量已有上限，缓存读取又要求已发送 Conversation 不变（runtime D129），没有测量证明值得再引进淘汰状态与容量规则。
-/
-/
