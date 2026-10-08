-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::endpoint::models

规定 `endpoint::models`（`crates/gateway/src/endpoint/models.rs`）：探测面：一行 `/models` 说了什么，没说什么。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::endpoint::models` 旁的测试守住。
-/

/-!
### 8-10 gateway::endpoint 探测面

```rust
impl Endpoint { pub fn list_models(&self, url: &str) -> Result<Vec<ModelFacts>, AxError>; }
```

两家一方都在 `GET .../models` 返回 `{"data":[{"id":..}]}`，**都不返回价目**——探测取对端报出的事实（`ModelFacts`，§8-16），对端没报的 token 数字由人在登记时确认。探测与正式调用共用同一个兑付路径（`authorize`）：两套认证拼法就是两个权威，而漂开的总是没人看的那个。

**探测不是登记的前提**。上一段的「两家都返回」只对那两家为真：网关与 Anthropic 兼容格式的第三方多半根本不服务 `/models`，于是一条本可用的线路被一个它从未承诺过的接口挡在城外。三行规则：探测成功→按对端报的 id 收窄（`probed=true`）；探测失败且人报了型号→按人报的登记（`probed=false`，并按 `effect` 级写一条诊断，点名探测的错——**登记确实发生了，被拒的只是探测**，用 `refuse` 级会说成这次登记被门拒了，那是假话）；探测失败且人没报型号→仍然拒绝，恢复语改为「把要用的 model id 报上来，再登记一次」，因为此时城手里一个可调用的名字都没有。

**四个参照实现一致**：pi、codex、opencode、Claude Code 都让人**声明**模型清单，发现是可选的（Claude Code 默认关闭、3 秒超时、失败静默）。把可选的发现当成必选的准入，是本仓自己加的限制，不是外部事实。

**「路径归兼容格式」也管凭证头**：Anthropic 的 API key 走 `x-api-key`（`Authorization: Bearer` 只发给短时联邦令牌），OpenAI 兼容格式走 `Bearer`；人显式填的头名恒优先。见 §8-9 `AuthSpec::for_dialect`。落选的是「让登记页必填头名」：那把一个兼容格式自己就知道的事推给了人，而人填错的代价是一个 401。

**一行还读两件事：思考档的陈述与模型的规范 id**，读法与窗口、上限一样是「问题→键」，没有键作答即缺席（`ModelFacts.thinking`、`ModelFacts.canonical`，kernel 的 `crates/kernel/spec/Event/Record.lean`）：

| 上游 | 键 | 读成 |
|---|---|---|
| OpenRouter | `reasoning.{supported_efforts,default_effort,default_enabled}` | `supported_efforts` 为数组时是那几档；为 `null` 时是全部六档（原文 "When `null`, all gateway effort values are accepted"）；缺这个键时只有开关（"the model does not expose effort selection"）；`reasoning` 整个缺席即没说 |
| Anthropic | `capabilities.effort.<level>.supported`、`capabilities.thinking.types.adaptive.supported` | `supported` 为真的那几档；`adaptive` 为真即能开启 |
| DeepSeek | `effort.{supported_levels,default_level}` | 那几档与默认档 |
| xAI | `capabilities.{reasoning_effort,default_reasoning_effort}` | 那几档与默认档；同名两键也在行的顶层读 |
| Moonshot | `supports_reasoning` | 只有开关：真即能开启，假即不能 |
| OpenRouter | `canonical_slug`，再是 `hugging_face_id` | 规范 id（`provider::identity`，§8-38） |

- **上游的词照 `Effort` 的 serde 拼写读**：七个词以外的词（`ultra`、Ollama 由模型自定的名字）不入集合，`none` 也不入，因为关闭思考不是一档；一个说了档位却一个都读不懂的陈述读作只有开关。
- **Ollama 的 `/v1/models` 不带这些信息**：它的行只有 `id/object/created/owned_by`，读成没说，梯子落到下一档；`/api/show` 的 `thinking` 对象要逐模型另发一次请求，今天不读（§3）。
- **Anthropic 的列表要翻页**：`GET /v1/models` 每页缺省 20 行（<https://platform.claude.com/docs/en/api/models/list>），所以 messages 面的探测带 `limit=1000`，并在答复的 `has_more` 为真时以 `after_id=<last_id>` 接着读，至多 `MODEL_LIST_PAGES`（16）页；第 21 个以后的模型因此也有陈述。别的两面不翻页：OpenAI 兼容的列表没有分页参数，带上一个对端不认的查询参数可能被拒。
- **读到的事实跨过重启**：登记把这一次读到的 `Vec<ModelFacts>` 以 JSON 写进 CAS，`endpoint_attached.facts_blob` 记它的摘要（kernel D57）；编码与解码只在 `router::attached` 一处（`AttachedEndpoint::facts_bytes`、`EndpointBook::learn`），写 CAS 与读 CAS 的是持有 CAS 的 accounting。簿的折叠不做 I/O：它只记下摘要，由持有 CAS 的一方读出字节交给 `learn`，`learn` 只填进摘要相同的那一行，所以一次更晚的登记不会被较早的事实覆盖。

**`adapter_for` 住 `endpoint/adapter.rs`**：装配线（每个 chosen 造一个 Endpoint，§8-3）读的只有 chosen 与赎回闭包，故它是自由函数而非 `RunWorker` 方法——挂在 worker 上等于说装配需要整座城。调用期限是 `EndpointTuning::DEFAULTS.timeout_ms`（§8-16），`adapter_for` 读 `call_timeout_ms()`——一个默认值该住在它所默认的那个设置旁边。凭据簇只出 `resolver`（赎回闭包是凭据的形状）与 `dialect_headers`（它住装配层的凭据簇 `crates/accounting/src/worker/credentials.rs`；兼容格式要的头按理是兼容格式的事，是否搬进本 crate 未定）。
-/
