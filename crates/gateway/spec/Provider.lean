-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider

规定 `provider`（`crates/gateway/src/provider/mod.rs`）：本城在问一个 provider 之前就知道的事：厂商文档写下来一次，与输出上限的事实梯。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`gateway::provider` 旁还没有测试。
-/

/-!
### 8-17 `gateway::provider`：厂商文档写下来一次，与输出上限的事实梯（形状 6 数据面 ＋ 形状 1 判定）

```rust
// provider::preset —— 数据面，逐行注出处
pub struct HostPreset { pub host: &'static str, pub faces: &'static [Face],
                        pub models: &'static [ModelPreset], pub source: &'static str }
pub struct Face { pub dialect: DialectKind, pub path: &'static str }   // 厂商文档写的一面，与它挂在哪条路径下
impl HostPreset {
    pub fn default_dialect(&self) -> Option<DialectKind>;   // 恰一面时是那一面，否则 None
    pub fn path_for(&self, dialect: Option<DialectKind>) -> Option<&'static str>;   // 那一面的路径；未定面或本表不列的面取第一面
}
pub fn known_hosts() -> Result<Vec<KnownHost>, AxError>;    // 设置页的厂商表；归一化拒掉某一行即是本表的缺陷
pub struct KnownHost { pub host: &'static str, pub faces: Vec<(DialectKind, String)> }   // 每面一个 base URL，经 normalise_entered 算出
pub struct ModelPreset { pub id_prefix: &'static str, pub context_tokens: u64,
                         pub max_output_tokens: u64, pub input: InputKinds,
                         pub source: &'static str }
pub const PRESETS: [HostPreset; 13];
pub struct ChatSpelling { pub ceiling: CeilingField, pub effort: EffortField, pub reasoning: ReasoningReturn }
impl ChatSpelling { pub const DOCUMENTED: ChatSpelling; }       // 未登记的主机：max_tokens、reasoning_effort、丢弃思考
pub enum CeilingField { MaxTokens, MaxCompletionTokens }
pub enum EffortField { ReasoningEffort, ReasoningObject }
pub enum ReasoningReturn { Dropped, AsReasoningContent }
// HostPreset 另有两列：chat: ChatSpelling、session_header: Option<&'static str>
pub fn chat_spelling(base_url: &str) -> ChatSpelling;             // 未登记即 DOCUMENTED
pub fn session_header(base_url: &str) -> Option<&'static str>;    // 要一个会话标识头的主机说出头名
pub fn for_host(host: &str) -> Option<&'static HostPreset>;
pub fn model_for(base_url: &str, id: &str) -> Option<&'static ModelPreset>;   // 这个 host 自己的行；没有自己模型行、`reach::is_local` 答否的 host 再查发布这个 id 的厂商的行
pub fn ceiling_for(base_url: &str, id: &str) -> Option<Ceiling>;
pub fn window_for(base_url: &str, id: &str) -> Option<Window>;               // 同一行的上下文窗口
pub(crate) fn input_for(base_url: &str, id: &str) -> Option<InputKinds>;     // 同一行的 input，只由 §8-37 的梯子读

// provider::ceiling —— 判定，先命中者胜；哪几档作答取决于这一面要不要这个字段
pub enum CeilingSource { Person, Upstream, Preset, Policy }   // as_str(): person|upstream|preset|policy
pub struct Stated { pub person: Option<Ceiling>, pub upstream: Option<Ceiling> }
pub struct Target<'a> { pub base_url: &'a str, pub id: &'a str, pub wire: DialectKind }
pub enum OutputCeiling {
    Sent { tokens: Ceiling, from: CeilingSource },   // 每个请求都写出的数字，与说出它的那一档
    ProviderDefault,                                 // 无人陈述且这一面不要求：不写该字段，由供应方按模型取缺省
}
impl OutputCeiling {
    pub fn resolve(stated: Stated, pinned: Option<Ceiling>, target: Target<'_>) -> Option<OutputCeiling>;
    pub const fn tokens(self) -> Option<Ceiling>;
    pub const fn word(self) -> &'static str;     // person|upstream|preset|policy|provider，进 model_selected.ceiling_from
}
```

- **事实梯只有一架，权威从高到低：人填 → 上游陈述 → 本地预设 → 策略缺省**。人填不是推断，故压过其余三档；上游 `/v1/models` 说过的话压过本城钉下的任何数字（Anthropic 自己的 Models API 逐模型给出 `max_tokens` 与 `max_input_tokens`，`endpoint::models` 读这两个键）。**后两档只在这一面要求这个字段时作答**：messages 兼容格式的每个请求都必须带 `max_tokens`，于是那里梯子恒有答案，`kernel::consts_policy::OUTPUT_CEILING_DEFAULT = 8_192` 只在前三档全数沉默时作答；chat 与 responses 两面的上限字段是可选的，人与上游都没说时答 `ProviderDefault`，请求里不写这个字段，由供应方按模型取它自己的缺省。
- **为什么 OpenAI 兼容两面不写本城的数字**：本城写下的任何数字，要么低于厂商按模型给的缺省（把一个思考模型截在它还在想的时候，跑以 `limit` 结束，人读到的是体验问题而不是设置问题），要么在对话变长后被拒——Kimi 的文档写明「input plus max_completion_tokens exceeds the model context window」时答 `invalid_request_error`，vLLM 对 `max_tokens` 加输入超过 `max_model_len` 的请求同样答 400。厂商的缺省总被厂商自己接受，并且随模型走：DeepSeek 不写 `max_tokens` 时思考模式缺省 64K（<https://api-docs.deepseek.com/api/create-chat-completion>），Kimi K3 缺省 131072（<https://platform.moonshot.ai/docs/api/chat>）。落选的是「抬高策略缺省」：一个对所有模型都更大的数字正是上面第二种失败，而且它仍是本城替厂商猜的数字。**重开参数**：当某个登记过的主机的文档缺省低到截断一次普通的派活（今天没有一家），在本表给它的模型写行，并让这一面的预设档作答。
- **`resolve` 的返回值带着是谁答的**（`CeilingSource`），因为只拿到数字的调用方说不出一次跑为什么停在那里。这是对 `anthropic.rs` 那句自我反对的回答——「a ceiling invented at the call site truncates runs for a reason that appears nowhere in the account」：它出现在账里（`model_selected.ceiling_from`，见 `crates/sprawling/Spec.lean` §8-52）。
- **预设表的上下文窗口也是一档**：`window_for` 与 `ceiling_for` 读同一行，装配层的窗口梯（`crates/sprawling/Spec.lean` §8-71）在钉版目录之后读它。窗口不上线，只用于上下文提醒与会话冻下的形状，所以它在三个兼容格式上一样作答。今天登记模型行的是 `api.anthropic.com`（当前四个家族与更早的四行，读自厂商的 models overview）与 `api.deepseek.com`（`deepseek-flash` 与 `deepseek-v4` 两个家族：1M 窗口，上限 393216——API 参考写作「between 1 and 384K (393216)」）。厂商页上写作 `1M`、`128K` 的数按一千进位读：它恒不高于厂商所指的那个数。
- **钉版目录与预设表是同一档的两个索引**：目录按精确 id 查，预设表按 host ＋ id 前缀查，一条测试钉住两个索引不为同一个 id 作答。`MarketSnapshot::lookup` 因此保持精确匹配，前缀匹配只发生在预设表内。
- **预设表逐行注出处**，每行带厂商文档地址；查不到的行不发明数字，而是让梯子落到下一档——这就是「不补零、不补默认、不补猜测」在登记面上的样子。价目列不在本表：厂商价目随时在动，一个没有复核日期的价目行就是第二个会漂的权威，价目继续住钉版目录（§8-7）。
- **chat 面的三处拼法随主机走，住本表的 `chat` 列**：输出上限写进 `max_tokens` 还是 `max_completion_tokens`、强度写成 `reasoning_effort` 还是 `reasoning:{effort}`、上一轮的推理是否作为 `reasoning_content` 放回 assistant 消息。三件事都是「同一个 OpenAI 兼容格式，各家文档各写各的」，dialect 本身不带主机名，所以由 `Endpoint::wire_request` 按 base URL 查一次本表、把拼法作为参数交给纯函数。**未登记的主机取 `ChatSpelling::DOCUMENTED`**：`max_tokens`（OpenAI 规格已标 deprecated，但本地推理服务与多数兼容服务只认它，而 `max_completion_tokens` 落在一个不认它的服务上是被静默忽略的上限）、`reasoning_effort`（OpenAI 规格的拼法）、丢弃思考块（OpenAI 规格的 assistant 消息没有这个字段）。`api.openai.com` 必须是 `max_completion_tokens`：规格写明 `max_tokens`「is not compatible with o-series models」，推理模型对它答 400。落选的是「一律 `max_completion_tokens`」：DeepSeek、通义与智谱的文档只写 `max_tokens`，对它们换名就是把人定的上限丢掉。**重开参数**：当本地推理服务普遍改收 `max_completion_tokens` 时，`DOCUMENTED` 的上限列随之改。
- **会话标识头**：`opencode.ai`（OpenCode Zen 与 Go 共用这个 host）的文档要求每个会话带一个稳定的 `x-opencode-session`，用于路由与前缀缓存。值由 `endpoint` 从请求本身推出：冻结前缀的四段 hash 加第一条消息的字节，经 `kernel::B3Hash::digest` 取十六进制。同一会话的每一轮前缀与首条消息都不变，所以值不变；不同会话首条不同，值就不同；推出来的值不含任何原文，也不需要本城另存一份状态。人在 `extra_headers` 里自己写了同名头时让位于人。本城不冒充 OpenCode 客户端：只收 OpenCode 客户端的免费模型不在本城要走的路上。所有经 `client_for` 出去的请求都带 `sprawling/<版本>` 的 User-Agent——OpenCode Go 的文档要求客户端以自己的名字自报，而不是 HTTP 库的名字。
- **一个 host 说几面，各挂在哪，是本表的一列（`faces`）**。它取代原来的 `base_path` 与 `dialect` 两列：一列只能说一条路径与一个默认面，而厂商文档常把两种兼容格式挂在同一 host 的两条路径下（DeepSeek 的 OpenAI 兼容面在 `/`、Anthropic 兼容面在 `/anthropic`；Kimi Code 的 OpenAI 兼容面在 `/coding/v1`、Anthropic 兼容面在 `/coding/`）。**默认面只在恰一面时存在**：两面以上时替人挑一面就是替人猜，归一化照旧让人选。人粘了裸 host 时，路径取他所选那一面的路径；他还没选时取第一面的路径，第一面因此按厂商文档的主推面排。
- **设置页的厂商表读本表，客户端不另持一份**（`known_hosts`）。每一面的 base URL 由 `normalise_entered(host, 那一面)` 算出，所以页上填进框里的地址，就是登记时这座城会算出的同一个地址；客户端据同一答案决定哪几面可选。落选的是保留客户端的 `presets.ts`：它与本表已经分歧（它说 DeepSeek 只有 chat 面，本表与厂商文档都说 chat 与 responses 两面都在），第二份表只会继续分歧。
- **本表登记的 host**（逐行注出处）：`api.anthropic.com`、`api.openai.com`、`api.deepseek.com`（`/`：文档印的 base URL 不带路径，补 `/v1` 就离开了文档）、`api.x.ai`、`openrouter.ai`、`generativelanguage.googleapis.com`（`/v1beta/openai`：OpenAI 兼容面挂在这里，`/v1beta` 之下是 Gemini 自己的形状，本城没有那支笔）、`open.bigmodel.cn` 与 `api.z.ai`（`/api/paas/v4`，智谱国内与海外两站）、`opencode.ai`（`/zen/v1`；OpenCode Go 挂在同一 host 的 `/zen/go/v1`，人粘的路径恒不被改写，所以 Go 的人粘带路径的 URL），加 `api.kimi.com`（`/coding/v1`）、`api.moonshot.cn`（`/v1`）、`api.moonshot.ai`（`/v1`），后三行读自 `MoonshotAI/kimi-cli` 的 `src/kimi_cli/auth/platforms.py`（docs/third-party.md §1 已列为被看路径），兼容格式为 OpenAI 兼容——同仓 `kosong/chat_provider/openai_common.py` 以这三个 base URL 构造 OpenAI 客户端。**`api.kimi.com` 与 `openrouter.ai` 是同一类缺陷的两个实例**：一律补 `/v1` 会把 Kimi Code 会员端点指到不存在的路径。
- **`label` 与「价格」两列不进本表——这是一条决定**。`label`：一个端点在人眼前叫什么，已有唯一的家，即人自己填的 `EndpointTuning.label`（§8-16）；人没填时该显示什么，从 summary 已经携带的 base URL 里按 `reach::split` 读一次 host 即得。厂商展示名再落一列，就是把 URL 已经携带的事实重拼一遍，而两处一旦不一致，界面上那个名字与实际调用的主机会指向两家厂商。「价格」：一次调用按什么价结算，也已有唯一的家，且是一架有序的梯——厂商在 `/v1/models` 里陈述的原文（`ModelFacts.input_price`／`output_price`，§8-16）在上，钉版目录按精确 id（§8-7）在下，而 `CostSource::Authoritative` 在两者之上；按 host ＋ id 前缀再加一个索引，就是同一批厂商数字的第三个家。**结算读的是登记那一刻写进 `model_selected` 的那份价目**，故表里改一个数也追不回已登记的模型，第三个家只会静默地与前两个分叉。**重开参数**：当一次真实调用在钉版目录无行、上游又不陈述价目而必须结算出非零金额时，价目以**迁移**而非新增索引的方式进本表——把价目事实从 `MarketSnapshot` 整体搬到 host ＋ id 前缀索引下，钉版目录同期删去价目列，每格带复核日期。`label` 的重开参数同理：`wire::EndpointSummary` 决定展示名不再由人填时，那一列进表且 `EndpointTuning.label` 同期降为覆盖值。
- **中转站转发厂商的 id 时，厂商的行作答，但排在中转站自己的陈述之后**：`model_for` 先查 base URL 所在 host 自己的行；这个 host 在本表没有模型行、且 `reach::is_local` 答否时，再在全表里按最长前缀查发布这个 id 的厂商的行。理由：中转站的模型列表几乎从不陈述上限（上游档因此空着），而 messages 兼容格式非写一个数不可——落到策略缺省的 8192 与中转站的事实毫无关系，比它所转发的那个模型的文档上限更远；一个把上限压得更低的中转站会以一次拒绝说出来，而拒绝的恢复语指向设置页上那一格，这比一次被静默截断、以 `limit` 结束的跑更容易被人看懂。`reach::is_local` 答是的 host 不查厂商的行：本地推理服务上一个同名的量化模型，窗口是那个服务的配置，套用厂商图表就是把一个它放不下的数字发给它。**重开参数**：当中转站普遍在 `/v1/models` 里陈述上限时，这一档对它们沉默也不失什么，可以撤回。
- **主机表只有这一张**：`router::normalise` 的路径与形状缺省从本表取（`openrouter.ai` 是 `/api/v1`、Gemini 的兼容面是 `/v1beta/openai`），`Endpoint::wire_request` 的 chat 面拼法与会话标识头也从本表取，归一化算法与 dialect 自身都不带任何主机名。
-/
