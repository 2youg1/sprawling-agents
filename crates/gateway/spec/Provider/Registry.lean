-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::registry

规定 `provider::registry`（`crates/gateway/src/provider/registry.rs`）：一个端点是怎么连的，attach 时解析一次；会话之外的两张脸。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-18 `gateway::provider::registry`：一个端点是怎么连的，attach 时解析一次（形状 1 判定 ＋ 形状 6 数据面）

```rust
// provider::registry —— 连接解析，attach 后不重算
pub enum ConnectionKind { OpenAiCompat, Responses, AnthropicNative }
impl ConnectionKind {
    pub const fn wire(self) -> DialectKind;      // 由哪支写请求的笔作答
    pub const fn as_str(self) -> &'static str;   // 三个扁平词，账本／wire／配置文件共用
    pub fn parse(word: &str) -> Result<ConnectionKind, AxError>;   // 另读旧版本写下的四个 harness 词（§8-5）
}
pub fn resolve(shape: DialectHint) -> Result<ConnectionKind, AxError>;

// provider::modality —— 会话之外的两张脸（14.4 第一批）
pub enum Modality { Embedding, Rerank }          // as_str(): embedding|rerank；ALL: [Modality; 2]
impl ConnectionKind {
    pub const fn path_for(self, modality: Modality) -> Option<&'static str>;
    pub fn url_for(self, base_url: &str, modality: Modality) -> Option<String>;
}

// provider::modality::embedding —— 请求与回答的真实形状
pub struct EmbeddingRequest;                     // new(model, inputs) -> Result；with_dimensions；texts()；dimensions()；body()
pub struct Embeddings;                           // parse(&Value, &EmbeddingRequest) -> Result
                                                 // vectors() -> &[Vec<f64>]；model()；prompt_tokens()

// provider::modality::rerank —— 同上
pub struct RerankRequest;                        // new(query, passages) -> Result；passages()；body()
pub struct Rank { pub passage: usize, pub score: f64 }
pub struct Ranking;                              // parse(&Value, &RerankRequest) -> Result；ranks()；best()

// provider::modality::call —— 一次调用从头到尾
pub struct Vectors;                              // of(&AttachedEndpoint, model) -> Result<Vectors, AxError>
                                                 // url()；embed(&EmbeddingRequest, Redemption) -> Result<(Embeddings, EmbeddingCalled), AxError>
pub struct Ranks;                                // of(&AttachedEndpoint, model) -> Result<Ranks, AxError>
                                                 // url()；rank(&RerankRequest, Redemption) -> Result<(Ranking, RerankCalled), AxError>
```

- **「这个端点是怎么连的」分在三处答，没有一处说得全**：dialect 说由哪支笔写请求，credential 说哪把 key 在付账，上限梯说这次调用能写多少。三处各答一半，一致到其中一处被改为止。`ConnectionKind` 是那一个答案，**attach 时在归一化之后解析一次**，写进 `endpoint_attached`、由 `Query::Config` 读回，调用路径不猜——一件事算两遍就是两遍可以算出不同结果。
- **登记面留住人粘贴的是哪一种 URL**：人粘贴 responses URL，归一化判出 `DialectHint::Responses`，`ConnectionKind::Responses` 把它记进账本，`wire()` 对它答 `DialectKind::OpenAiResponses`（§8-20）。哪支笔写请求只由 `wire()` 的一条臂决定，改它不需要任何人重新 attach 端点。
- **旧的 harness 词读成它答话的那一面**：`codex` 读作 `Responses`，`claude_code` 读作 `AnthropicNative`，`grok_build` 与 `kimi_cli` 读作 `OpenAiCompat`。这四个词只在读入时出现，本 crate 不再写它们（§8-5）。落选的是把这样的端点在重放时当作丢失：那会让人登记过的东西在列表里无声消失，而读成它的面至少让下一次调用以对侧自己的 401 说明凭证已不可用。
- **`DialectHint::Unset` 是真状态而不是缺失值**：没人说过形状时解析拒绝，恢复语指向「把供应方文档印出来的完整 URL 粘进来」，因为带 `chat/completions`／`responses`／`messages` 尾段的 URL 自己就说了。
- **base URL 里不住凭据**：`router::normalise` 在读任何别的东西之前先拒两种写法——authority 里带 `userinfo@`，或 query 里有一个值被 `kernel::secret::scan` 认出、或参数名被 `kernel::secret::names_a_credential` 认作凭据名（Gemini 的 `?key=` 属此类）。码 `E_CONFIG_INVALID`，恢复语指向「把 key 存进金库，URL 只留地址」。拒绝的 action 与 subject 都不回显所输入的文本，因为这个错误会原样进 socket。理由：base URL 进 `endpoint_attached`／`endpoint_probed` 的载荷并随事件广播，账本只追加、`export` 会带走它，写进去就撤不回；把判定放在归一化这唯一的门上，登记与探测都经过它（`Entered::resolved`），不必在每个出口各扫一次。不带凭据的 query（如 `?api-version=`）照旧原样保留，因为少了它就是在调另一个端点。
- **词只有一套**：三个扁平词（`openai_compat`／`responses`／`anthropic_native`），`as_str` 写、`parse` 读，一条测试钉住往返与不重词。
- **模态只有形状与那条路**：`Modality::{Embedding, Rerank}` ＋「哪种连接在哪条路径上服务它」一张表，加上 `provider::modality::call` 里一次调用的全程。Anthropic 不发布这两张脸，故答 `None`——**没有路径就是不服务**，调用方连 URL 都拼不出来，而拒绝语报出的是**这个连接是怎么连的**（人手里能改的那个事实），不是一条他不认识的路径。路径与 base URL 的拼接复用全城唯一那个 `router::join`，并且只发生在 `url_for` 一处：`Face` 存的是 `url_for` 拼好的地址，不自持半成品路径。两处各自拼一次时，一致只是因为两条路恰好都走同一个 `join` 与同一张表，改一处不会让另一处红。
- **一次调用不自己写传输**（`call`）：向外的 POST、非 2xx 怎么折、凭据写在哪一个头上、超时与代理怎么算，都还是 `endpoint` 那一处的答案；本模块只多给两样本张脸独有的东西：**这次问的是哪张脸**与**请求体**。自称一个传输层就是第二份「什么叫失败」的定义。
- **人设的 body override 不到向量面**：override 是指向对话请求体的 JSON 指针（`endpoint::config::apply_override` 会替它造出缺失的路径），同一指针落到 embeddings 体上会多出一个没人读的字段——那是本城在替人回答一个他没问过的关于对话的问题。**额外的头会到**：头是关于端点的事实，不是关于一个体的。
- **凭据每次调用赎回一次**（`Redemption` 作为调用参数而不是字段）：赎回按设计不缓存，它只为一次操作存在，把 `Secrets` 挂在长寿命对象上就是让它活到一整批检索跑完。
- **每次调用把手写的那一行还回调用方**：`embed` 返 `(Embeddings, EmbeddingCalled)`、`rank` 返 `(Ranking, RerankCalled)`。本模块没有账本可写，而那一行的归宿是 run 的历史：拿得住账本的人 append `Payload::of(&record)`。载荷的字段类型住 `kernel::event::record::modality`，本模块不另写一份键名。**rerank 那一行没有用量列**：本城写入的那两张脸不报用量，填一个派生数就是把本城的估算放进人读账单的那一列，而留一个永远写不进 `Some` 的字段同样是给读账本的人一句填不上的承诺。`RerankCalled` 因此只有 `model`／`passages`／`ranks` 三键；用量只在 embedding 面上有，且只在供应方真的报了它时出现（`EmbeddingCalled::prompt_tokens`）。
- **两张脸的字节形状也各只有一处**：`embedding` 的请求与回答照 `openai/openai-openapi` 的 `CreateEmbeddingRequest`／`CreateEmbeddingResponse`／`Embedding` 三个 schema 写，`rerank` 照 `huggingface/text-embeddings-inference` 仓库 docs 目录下的 `openapi.json` 的 `/rerank` 路径与 `RerankRequest`／`Rank` 写；两处出处（仓库、文件与 schema 名）写在模块头，读到的提交只记在 docs/third-party.md §1。**请求显式写 `encoding_format`**，因为读答案的那段只认一种编码，而厂商可改的默认值不是可以照着解析的依据。**回答按 `index` 归位而不按到达顺序**：把第三段文字的向量配给第一段，是一个不报错的检索错误。**名次不在本城重排**：分数只在一次回答内可比，服务端排好的序就是答案，再排一次就是本城对自己付钱问来的名次有第二个意见。
- **chat 不是本枚举的成员**：会话路径归已经在调用它的那处（`router::attached::chat_path`），在这里再写一次就是每回合都在发的那条路径有了第二个家。
-/
