-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.gateway.spec.Adviser
import crates.gateway.spec.Concurrency
import crates.gateway.spec.Cost
import crates.gateway.spec.Credential
import crates.gateway.spec.Credential.Vault.File
import crates.gateway.spec.Dialect
import crates.gateway.spec.Dialect.Responses
import crates.gateway.spec.Endpoint
import crates.gateway.spec.Endpoint.Failure
import crates.gateway.spec.Endpoint.Models
import crates.gateway.spec.Endpoint.Stream
import crates.gateway.spec.Endpoint.Transport
import crates.gateway.spec.Market
import crates.gateway.spec.Ocr
import crates.gateway.spec.Provider
import crates.gateway.spec.Provider.Ceiling
import crates.gateway.spec.Provider.Input
import crates.gateway.spec.Provider.Preset
import crates.gateway.spec.Provider.Registry
import crates.gateway.spec.Provider.Stability
import crates.gateway.spec.Reach
import crates.gateway.spec.Reach.Resolve
import crates.gateway.spec.Router
import crates.gateway.spec.Router.Tuning
import crates.gateway.spec.Transcribe
import crates.gateway.spec.Transcribe.Recording

/-! # gateway 的规格

`sprawling-gateway`（库名 `gateway`，目录 `crates/gateway`，依赖 kernel）是模型调用的一切：模型路由；provider 客户端自写，加上认证的两半；Custody；市场快照与成本；`kernel::model` 缝的生产适配器。模块：dialect（及 anthropic／openai／mismatch）／endpoint／credential（内缝 Vault）／market／cost／router／provider／reach／transcribe／ocr／adviser。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部，并写下四节不属于任何一个模块的 crate 级接口。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部里，别处引作 `gateway D<n>`；D1 到 D15 是这份规格在 Markdown 时 §12 各段按出现顺序的编号，D16 起是迁到 Lean 之后的决定，§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：输出上限的事实梯（`spec/Provider/Ceiling.lean`）、一个模型收得下什么（`spec/Provider/Input.lean`）、预置表里哪一行为一个模型 id 作答（`spec/Provider/Preset.lean`）、一次失败能不能再试（`spec/Endpoint/Failure.lean`）、一次调用的结算（`spec/Cost.lean`）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

| 模块 | 一句话 |
|---|---|
| `dialect` | 城内规范 Anthropic Messages 与 OpenAI Chat 的双向纯函数翻译；保断点位／工具形状／usage |
| `endpoint` | 自写线格式 HTTP 客户端（reqwest blocking＋rustls）实现 `kernel::Model`；逐字段请求覆盖；回环与远端同一适配器（§8-3） |
| `credential` | Custody 效果半（scan 命中→入 Vault→原位替换 SecretRef）；兑付（组请求末格 expose，credential_lent）；describe；持久性探测。凭证只有人交出的 API key，订阅额度不经本 crate（§8-5） |
| `market`＋`cost` | 模型目录快照＋钉版回滚；per-call 入账（权威计费额优先）。本 crate 不设 provider 侧准入与备用端点（§8-6、§8-11） |
| `router` | Endpoint 簿：从 Ledger 重建的已登记端点与每个标签的选择；一次取模型只答一问（§8-9） |
| `provider` | 厂商文档写下来一次：host 预设、输出上限的事实梯、一个模型收得下什么的梯子、一个端点怎么连（§8-17、§8-37、§8-18） |
| `reach` | 哪些调用走这台电脑的代理，与一次分段读数（§8-15） |
| `transcribe` | 把一段录音变成一行字的可选设施（§8-12） |
| `ocr` | 把一张图变成一行字的可选设施（§8-34） |
| `adviser` | 一次顾问咨询，走既有登记面（§8-23） |

每一行的模块由 `spec/` 下同名的分部规定（§8 的表）。
-/

/-! ## 2 验收标准

- dialect：golden（两 Dialect 各一请求一响应）＋proptest 往返（响应侧 wire→canonical→重渲染 wire 逐字段等值；请求侧断点位／工具形状／文本字节无失）。
- endpoint：对回环假 provider 服务的半流中断（SSE 截断→E_PROVIDER 且不产伪 ModelReturn）＋幂等重试（同 IdemKey 重发，对外恰一次效果——由调用方 dedup 看守，endpoint 自身无重试暗策略）。
- credential：A13 链——人交出的值进金库（`set`，名字由调用方给）→`secret_captured` 入账（无明文无哈希前缀，写者是装配层 `credentials::signing`）→配置只见 `secret:`→resolve 兑付产 `credential_lent`→`describe` 恒不返回值。**外来字节的扫描与就地替换不在本 crate**：那是 `runtime::redact`，本 crate 不留第二份。持久性四档（§8-21）可注入验证。
- 调优：一个端点从表单与从导入两条路径附着后，`Query::Config` 回答的超时与重试相同；未调过的端点读到的三个默认值来自 `EndpointTuning::DEFAULTS` 而非任何第二处。
- cost：权威计费额在场则恒胜价目推算；两源不一致时以权威为准并记差额；A20 的取材面（对账断言住 storage::attribution）。

分部里的定理是模型对性质的证明：

- `spec/Provider/Ceiling.lean`：人填的数压过其余三档（`the_person_outranks_every_other_rung`），人没说时上游压过本城钉下的数（`upstream_outranks_what_this_city_pinned`）；不要这个字段的一面上没人说就不写（`a_face_that_takes_no_figure_is_sent_none_nobody_stated`），要它的一面上恒有一个数（`a_face_that_needs_a_figure_always_gets_one`）；目录在预置表之前（`the_catalogue_outranks_the_preset_table`）；本城钉下的数与策略缺省只在这一面要一个数时作答（`the_city_states_a_figure_only_where_the_face_needs_one`）。
- `spec/Provider/Input.lean`：目录在预置表之前、目录沉默时预置表作答、两者都沉默时答 `Text`；答案要么是一个说过的事实，要么是 `Text`（`every_answer_is_a_stated_fact_or_text`）；只读目录的被否设计把一个文档写明读图的模型登记成 `Text`（`the_catalogue_alone_registers_a_documented_reader_as_text`）。
- `spec/Provider/Preset.lean`：这台电脑上的服务不借厂商的行（`a_server_on_this_machine_borrows_no_vendor_row`）；host 有自己的行时只由它们作答（`a_host_with_rows_of_its_own_answers_from_them`）；作答的行的前缀是这个 id 的前缀（`the_answer_is_a_prefix_of_the_id`）。
- `spec/Endpoint/Failure.lean`：没发出去的请求再发，发出去丢了回答的效果不明；非 2xx 恰在 408、429 与 5xx 时再问（`a_refusal_is_asked_again_exactly_when_the_provider_is_busy`、`the_status_table`）；溢出恰是 400 或 413 且拒词说窗口满了（`a_refusal_is_an_overflow_exactly_when_it_names_the_window`），从不再试。
- `spec/Cost.lean`：权威计费额在场恒胜（`the_authoritative_amount_always_wins`）；价目推算答出的数是精确的和、装得进 `u64`（`a_settled_sheet_is_the_exact_sum`）。

每个模型都带一个可实现的正常路径（`example`：一个两表都不认识的模型在 messages 面上以策略缺省叫得通、转发厂商 id 的中转站读到厂商那一行、`cost` 测试里那一次结算），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

- **本地推理**＝指向回环地址的 OpenAI 兼容服务（llama.cpp/ollama 一类），与远端同走 `Endpoint`（§8-3）。本地引擎进程管理不属本 crate。
- **tokio 不引入**：turn 是同步函数面，endpoint 用 `reqwest::blocking`（内部自管运行时线程，不出接口），为一个 HTTP 调用把 async 传染到全库不值。
- 线格式 JSON 允许浮点（temperature 等 provider 字段）：dialect 是翻译面不是判定路径；判定路径（cost／market 价目）恒整数。
- **只有 Anthropic 兼容格式在结算前交出调用。** `Endpoint` 经 `Model::call_speculating`（kernel SPEC「模型端口第三扇门」）在 `content_block_stop` 到达时交出完整调用，消费方是 `runtime::turn::speculation`（`crates/runtime/Spec.lean` §8-3），它守的性质由 `crates/runtime/spec/Turn/Speculation.lean` 证明。OpenAI 两种兼容格式在结算前不交出调用：它们的调用在哪一帧完整，线上没有一帧明说。
- **加密金库文件还没有探针选它**：`Custodian::probe` 只试平台服务，何时向人要口令、口令从哪里来未定（§8-21）。
- **上游 `/models` 说了 `image` 算不算「收得下什么」的一档，未定。** 探测把每一行的 `input_modalities` 原样记进 `ModelFacts`（§8-16），`AttachedEndpoint.models` 带着它，而 `accepted_input`（§8-37）不读它：那是供应方自己的词（`image`、`audio`），一个中转站可能为一个厂商自己拒图的 id 写上 `image`。若定为一档，它排在人之后、目录之前，与上限梯的 `Upstream` 同位；要定下它，需要一份真实中转站的 `/models` 记录，其中写着 `image` 的模型在那条线上确实收图。
- **responses 面答 `usage: null` 时算不算「供应方没报用量」，未定。** `openai-openapi` 把 `Response.usage` 写作 `ResponseUsage` 或 `null`。`dialect::responses::reply` 在 `usage` 缺席时报 `E_WIRE_MISMATCH`，为 `null` 时经 `mismatch::tokens_or_zero` 把输入与输出都读成 0 token：两种缺法得到相反的结论，后一种让这次调用的用量从账上静默消失。规格没有说 `status` 为 `completed` 的 response 会不会带 `null`，所以读法暂不改；要定下它，需要一个真实供应方在已完成的 response 里答出 `usage: null` 的记录。

模型自己的假设写在各分部的定义与定理假设里，不写成公理：`InputKinds`、`DialectKind`、`Ceiling` 是 kernel 的类型，kernel 的规格还没有迁到 Lean，所以分部照它们的变体与不变量各写一份模型里的类型，拼写与 Rust 相同；预置表按 host 与前缀查到的答案、`reach::is_local` 的判断、拒词里有没有窗口标记，都当作参数交给模型，它们各自的读法由 Rust 的测试检查。
-/

/-! ## 4 现状分析

公开面就是 `lib.rs` 的再导出；crate 根只再导出别的 crate 叫得出名字的项（D8）。
-/

/-! ## 5 权威信源

自写客户端的四条被逼理由与「流程是代码、情报是数据」；Custody 全节；model 缝（endpoint 生产适配器）；Anthropic Messages API 与 OpenAI Chat Completions API 官方文档（线格式字段名以官方为准）；`keyring-core` 与三个平台 store crate 的文档（平台凭证服务绑定）。
-/

/-! ## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。**本 crate 同例**：同一类型的 inherent impl 住不同簇文件时基线为每块各记一行 `impl`（`Endpoint` 两行）；下游 `sprawling` 基线记 `gateway` 内定义位簇路径（如 `credential::custodian::Custodian`），公共拼写不变。

Dialect／Endpoint／Custody／Vault／SecretRef／Sealed／Retries／HeaderValue／market snapshot／UsdMicros／权威计费额（authoritative billed amount）。概念名英文原词；「兑付」＝resolve+expose 的合称。

Lean 里的名字与 Rust 的对应：

- `Gateway.Provider.Ceiling.resolve` ↔ `OutputCeiling::resolve`，其参数 `preset` ↔ `preset::ceiling_for(target.base_url, target.id)`，`policy` ↔ `OUTPUT_CEILING_DEFAULT`，`wire` ↔ `Target.wire`；`field_on`、`Field` ↔ 同名的私有函数与枚举；`Ceiling.new` ↔ `kernel::Ceiling::new`。
- `Gateway.Provider.Input.accepted_input` ↔ `provider::input::accepted_input`，其参数 `preset` ↔ `preset::input_for(base_url, id)`。
- `Gateway.Provider.Preset.model_for` ↔ `preset::model_for`，`own` ↔ host 自己那一行的 `models`，`vendors` ↔ `PRESETS` 全部的模型行，`isLocal` ↔ `reach::is_local(base_url)`；`longest` ↔ `max_by_key(|row| row.id_prefix.len())`。
- `Gateway.Endpoint.Failure.retry`、`refusal` ↔ `ProviderFailure::retry`、`ProviderFailure::refusal`；`busy` ↔ `retry` 里 408、429、`is_server_error` 那一臂的条件；`RETRIABLE_KINDS` ↔ `Reported` 那一臂的 `matches!` 列表；`Exchange` 的参数 ↔ `reqwest::Error::is_connect`。
- `Gateway.Cost.settle`、`share` ↔ `cost::settle`、`cost::share`；`settle_from` ↔ `settle` 里逐项 `checked_add` 的循环；`checked` ↔ `u64` 的 `checked_mul`／`checked_add`；`exact` 是模型里不回绕的份额，Rust 没有对应。
-/

/-! ## 7 模块边界

```
dialect（路由，纯函数）◀── endpoint（I/O 适配器，impl kernel::Model）
dialect ───────────────▶ anthropic／openai（各自一种线格式的两向翻译）
anthropic／openai ──────▶ mismatch（共用的读取器与拒词；单向，无环）
credential ──▶ 内缝 Vault（pub(crate) trait：平台凭证库生产适配器＋会话内存第二适配器）
market／cost：纯判定与数据面，被 endpoint 与 runtime 回合层消费
```

**market**：`ModelEntry.max_output_tokens: Option<Ceiling>`——一次回答能吐多少字是**模型的事实**，不是调用处的选择；探测接口不返回它，故它随模型登记入目录行。**这一列为空时由谁来答，写在 §8-17 的事实梯里，而不是各兼容格式各自的默认**：chat 与 responses 两面不写上限字段是梯子的答案（`ProviderDefault`，供应方按模型取缺省）；`anthropic` 写不出请求时拒（`E_CONFIG_INVALID`）只是后盾，因为登记时梯子已为 messages 面给出一个数字。尚未登记的本地模型沿用 `local` 行的保守上限，登记面（§8-9）接管后改为人确认过的行。

**router**：本 crate 持有 Endpoint 簿——它是**值不是库**，从 Ledger 重建（同 `kernel::registry` 的口径）；本 crate 仍不持 Ledger 句柄，写入由装配层做。

**不做什么**：不做 duty pool 与三轴定档（多 Agent 功能未成形之前，职责池是给一个还不存在的消费者建权威；现为标签选择，见 §8-9）；不持有 Ledger 句柄（事件载荷由调用方入账，本 crate 只产载荷值）；不缓存已解封凭证（每次操作解析一次）；不实现通用 provider 抽象层（被明拒）；dialect 不开缝（纯函数不 trait 化）。
-/

/-! D8 crate 根只再导出有别的 crate 叫得出名字的项

一个只在 gateway 内部用、或只在测试里用的项，挂在 crate 根上就是一个没人读的公开面：编译器不再为它报 dead code，于是它在生产里还有没有调用者就没有东西在看。只供测试的项（`response_wire` 一族）放在 `#[cfg(test)]` 后面，不编进发行的二进制。另一种做法是保留再导出、靠审查记住它们没有调用者——那正是让这些项积下来的原因。
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/gateway/spec/Dialect.lean` |
| 8-2 | `crates/gateway/spec/Endpoint.lean` |
| 8-3 | `crates/gateway/spec/Endpoint/Transport.lean` |
| 8-4 | `crates/gateway/spec/Credential.lean` |
| 8-5 | 本文件 §8 |
| 8-6 | 本文件 §8 |
| 8-7 | `crates/gateway/spec/Market.lean` |
| 8-8 | `crates/gateway/spec/Cost.lean` |
| 8-9 | `crates/gateway/spec/Router.lean` |
| 8-10 | `crates/gateway/spec/Endpoint/Models.lean` |
| 8-11 | `crates/gateway/spec/Router.lean` |
| 8-12 | `crates/gateway/spec/Transcribe.lean` |
| 8-13 | `crates/gateway/spec/Endpoint/Stream.lean` |
| 8-14 | 本文件 §8 |
| 8-15 | `crates/gateway/spec/Reach.lean` |
| 8-16 | `crates/gateway/spec/Router/Tuning.lean` |
| 8-17 | `crates/gateway/spec/Provider.lean` |
| 8-18 | `crates/gateway/spec/Provider/Registry.lean` |
| 8-19 | `crates/gateway/spec/Provider/Stability.lean` |
| 8-20 | `crates/gateway/spec/Dialect/Responses.lean` |
| 8-21 | `crates/gateway/spec/Credential/Vault/File.lean` |
| 8-23 | `crates/gateway/spec/Adviser.lean` |
| 8-31 | 本文件 §8 |
| 8-32 | `crates/gateway/spec/Reach/Resolve.lean` |
| 8-33 | `crates/gateway/spec/Transcribe/Recording.lean` |
| 8-34 | `crates/gateway/spec/Ocr.lean` |
| 8-35 | `crates/gateway/spec/Endpoint/Transport.lean` |
| 8-37 | `crates/gateway/spec/Provider/Input.lean` |

§8-36 不用。下面四节不属于任何一个模块：两件本 crate 不做的事，目录化的形状，以及重试上限住在哪里。
-/

/-!
### 8-5 订阅额度不经本 crate

本 crate 只接两种兼容格式的端点：OpenAI 兼容（chat 与 responses 两面）与 Anthropic 兼容，凭证只有人交出的 API key。**订阅额度由厂商自己的 harness 带进城**（Codex、Claude Code、Grok Build、Kimi Code、Pi 五家，人在 harness 里自己登录，见 `crates/sprawling/Spec.lean` 的 harness 一节），本 crate 不以任何厂商客户端的身份登录，不持订阅令牌，也不续期。

- **理由是厂商原文，不是口味。** Anthropic 的 Claude Code 合规页（<https://code.claude.com/docs/en/legal-and-compliance>，「Authentication and credential use」一节）写明 "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications"，并且 "developers may not collect, store, or intermediate Claude.ai credentials or session tokens"。Kimi Code 的会员指南（<https://www.kimi.com/en/help/kimi-code/membership-guide>）把 OAuth 留给官方客户端，第三方工具用 API key。OpenAI 没有给第三方用 ChatGPT 凭据调用订阅后端的公开契约，它给出的嵌入方式是 `codex app-server`。借用某家官方客户端的 client id 登录、把令牌存进金库，正是前两家禁止、第三家不承诺的做法。
- **落选**：保留四家登录表并逐家标注风险。那是替人拿他的账号去赌厂商不执行条款，执行的后果落在人身上，而不是本城。
- **旧账本照样读得回。** 旧版本写下的 `endpoint_attached` 可能带四个 harness 词之一（`codex`／`claude_code`／`grok_build`／`kimi_cli`），`ConnectionKind::parse` 把它读成那家实际答话的那一面（§8-18）。它的凭证引用和载荷里记下的认证头原样保留，但本城不再续期，令牌过期后对侧以 401 拒绝，人换一把 API key 重新登记即可。
- **重开的参数**：某家厂商公开一份允许第三方应用以订阅身份调用其 API 的契约，而且这份契约要求应用自己注册，而不是借用官方客户端的 client id。
-/

/-!
### 8-6 provider 侧准入：每次模型调用先在端点的门前取名额

一个端点的并发名额是纯判定 `gateway::concurrency`（D17，性质在 `spec/Concurrency.lean`）：`Permits::take` 答三臂——取到、名额已满、等到某一刻（那一刻总晚于取的时刻，调用方不会忙等）；`rate_limited` 把名额减半（至少 1）并记下 `Retry-After` 给出的时刻；`succeeded` 连续 `WIDEN_AFTER` 次成功后、且过了那一刻，才把名额加 1，直到配置的上限。判定只读传给它的时刻与每次调用的结果，三个平台相同。

现状：名额状态是 `endpoint::permit::Gate`，随端点住 `endpoint::transport::Transport`（所有克隆共享同一个 `Arc`）；`adapter_for` 交出的模型是 `permit::Gated`，它的 `call`、`call_streaming` 与 `call_speculating` 先 `Gate::admit` 取得守卫 `Admitted`，再走 `Endpoint` 自己的往返，结果出来后守卫 `settle`（成功调 `succeeded`，`E_PROVIDER` 且状态 429 调 `rate_limited` 并带上 `retry_after_ms`），守卫丢弃时还名额并唤醒排队者。`Endpoint` 的三扇门因此是本 crate 内的固有方法，不再是 `kernel::Model` 的实现：crate 外拿到的每一个模型都走过门。门的上限是端点 tuning 的 `max_in_flight`，缺席时取 `vendor_in_flight(base_url)`（D21）；路由簿在折叠 `endpoint_attached` 时按它建端点的 `Transport`。

**要接时接在哪**（D20 定接线，D21 定配置与仪表）：名额状态随端点住 `endpoint::transport::Transport`（每个端点一份、所有克隆共享，与 HTTP client 同一个槽），`kernel::Model` 的门（`endpoint/model.rs` 的 `call`、`call_streaming` 与 `call_speculating`）在发出请求前取、得到结果后还，429 时把 `ProviderFailure::retry_after_ms` 交给 `rate_limited`；时刻来自 `bin::assembly` 注入、经路由簿交给 `Transport` 的单调时钟（D20），`max_in_flight` 经端点的 tuning 与 `endpoint_attached` 的载荷到达（D21）。

**代价**：在接上之前，429 的等待仍只由对端的 `retry-after` 提示与 watchdog 的退避表给出（D2），每条 run 各自退避；`provider_degraded` 事件由 `E_PROVIDER` 的 carrier 产出。
-/

/-! D17 每个端点一个并发上限：可配置，缺省取厂商文档的值，遇 429 收窄、恢复后放宽

**决定**：形状如下；§8-6 写判定之外还没接上的部分。端点的 tuning（`router/tuning.rs`）多一把 `max_in_flight: Option<MaxInFlight>`，配置里拼作 `max_in_flight`，合法域 1 到 `IN_FLIGHT_MAX`（256）；缺席时取这一类连接的厂商文档给出的并发值，厂商不给并发值的连接取 `IN_FLIGHT_DEFAULT`（16）。收窄与放宽是一个纯判定 `gateway::concurrency`：收到 429 或 `Retry-After` 时把当下的名额减半（至少 1），在 `Retry-After` 给出的时刻之前不再放宽；之后每连续 `WIDEN_AFTER`（8）次成功加 1，直到配置的上限。判定答两臂：名额已满（排队并计数）与等到某一刻（带时刻），不让调用方忙等（§8-6「要接时接在哪」）。状态随端点住它的 `Transport`，名额在 `kernel::Model` 的门里每次调用前取、后还（§8-6），所以 runtime 不改一行；排队数与等待时长是仪表的读数之一，页面在 provider 一处显示。

**理由**：车道不设上限之后（`crates/sprawling/Spec.lean` D34），排队只该发生在 provider 一处，因为那是对方限流与计费的地方；不设闸，429 就由每条 lane 各自撞墙、各自退避。缺省取厂商的值而不是城内一个常数，是因为不同端点的上限差一两个数量级。判定与平台无关，三个平台相同。

**被否**：①一个城级的总车道数（V0.0.8 写死的四条）：第五个准备好的 run 等一个与任何 provider 都无关的名额；②只靠 watchdog 的退避（D2）：退避是一条 run 自己的事，挡不住其余 run 继续撞同一个端点。

**重开参数**：厂商在响应头里给出当下余量时，缺省改读响应头，`IN_FLIGHT_DEFAULT` 只作没有头时的退路。256、16 与 8 是推断值，各是一个常量，改它们不改形状。
-/

/-! D20 名额的接线：一个注入的单调时钟，取在门里、按先来后到排队，等待有界

**决定**：
- **时钟是一个参数**：单调时钟是 `fn() -> Instant`，与 `reach_of` 已收的那一个同型，由 `bin::assembly` 经 `RunWorker` 的 `monotonic` 交给 `adapter_for(chosen, redemption, dialect_headers, monotonic)`，再进 `Gated`；本 crate 仍不调用 `Instant::now`（clippy 的 `disallowed-methods` 照旧拦它）。测试给一个读计数的函数，这就是这条接缝的第二个实现；函数指针而不是闭包或 trait，因为生产与测试的两个实现都不带状态，而 worker 早已用这个型把同一个时钟交给 `reach_of` 与 `Flight`。时钟进模型而不进路由簿，因为簿是从账本重建、可序列化的值，而时钟是进程的。
- **取与还在门里**：`Gated` 的三扇门的第一步是 `Gate::admit`，它返回一个持有名额的守卫 `Admitted`，守卫被丢弃时 `give_back` 并唤醒排队者——于是任何一条返回路径（成功、失败、`?` 提前返回、流读到一半出错）都还名额，不靠每条路径记得还。结果是成功时守卫先调 `succeeded(now)`；结果是 429（`E_PROVIDER` 且 `ProviderFailureKind::Refused { status: 429 }`）时先调 `rate_limited(now, retry_after_ms)`。流式调用在流收齐或放弃时才还，因为在那之前它在对端仍占一个并发；`call_speculating` 在流收齐之后才返回，守卫覆盖整条流。
- **排队按先来后到**：每次调用拿一张号（端点上一个单调递增的计数）排到队尾，在 `Condvar` 上等，只有号在队首的那一个重新 `take`，取到即出队并 `notify_all`，让下一张号看自己是否已到队首；`WaitUntil(t)` 时队首按 `t - now()` 定时等，其余照排。这样一个端点上的等待按到达次序得到名额，不靠操作系统唤醒谁。
- **等待有界**：一次排队至多等 `QUEUE_WAIT_MAX`（10 分钟，从拿号时的单调读数算），超过即以 `E_BACKPRESSURE_SHED` 拒这一次调用，主体是端点名与排了多久，恢复语「这个端点的并发上限比同时想调它的 run 少：在端点设置里调大 `max_in_flight`，或少派几条」；拒后号出队，下一个顶上。排队的时间不计入 `timeout_ms`：那个期限说的是一次请求对端多久答，不是城里排了多久。
- **取消**：门的签名没有取消信号，一个排着队的 run 在它拿到名额、调用返回之后的那个安全点才看见取消；`QUEUE_WAIT_MAX` 封住这段延迟的上限。

**理由**：本 crate 的每个判定只读传给它的时刻（D17），而名额要按时刻放宽，所以时刻必须有一个来处；放在路由簿的构造参数里，生产与测试各给一个，门的签名（`kernel::Model`）一行不改，runtime 也就一行不改。守卫还名额，是因为门里有七八条提前返回的路，漏还一次就让这个端点永远少一个名额。先来后到，是因为车道不设上限之后（`crates/sprawling/Spec.lean` D34）几十条 run 可能同时排在一个端点上，没有次序时一条 run 可以一直被后来者插队。

**被否**：①门的签名里加一个 `now`：`kernel::Model` 的每个实现与每个调用方都要改，而只有这一个实现用它；②`Condvar::notify_one` 交给操作系统挑：Windows、macOS 与 Linux 的唤醒次序不同，同一个场景在三个平台上排出三种次序；③无界等待：一个被收窄到 1 的端点后面排着的 run 会停到人发现为止；④把名额放在 runtime 的 `run::drive`：每条 run 各持一份状态，正是 D17 要去掉的那件事。

**重开参数**：`kernel::Model` 的门长出取消信号时，排队改为同时等它；`QUEUE_WAIT_MAX` 是推断值，一个常量。

**三个平台**：时钟在 Windows、macOS 与 Linux 上都是注入的 `Instant`（单调时钟：Windows 的 QueryPerformanceCounter，macOS 的 `mach_absolute_time`，Linux 的 `CLOCK_MONOTONIC`，由标准库选），排队次序由号决定，与平台无关。
-/

/-! D21 `max_in_flight` 进端点的 tuning 与 `endpoint_attached`；排队数与等待时长是每个端点的一份读数

**决定**：
- **配置**：wire 的 `EndpointTuning` 多 `max_in_flight: Option<u32>`（`crates/wire/spec/Command/Tuning.lean` §8-29），装配层的 `tuning_of` 把零与缺席读成 `None`、把 1 到 `IN_FLIGHT_MAX` 的值经 `MaxInFlight::try_from` 收下、其余拒 `E_CONFIG_INVALID`（与同一张表单上读作凭据的请求头同一个码，一张表单的拒绝不分两种码）；gateway 的 `EndpointTuning::max_in_flight: Option<MaxInFlight>` 是 D17 那把键，住在 tuning 而不是每次调用的 `EndpointConfig`，因为门是端点的、一次建成，调用只取名额。`router/payload.rs` 的 `endpoint_attached` 载荷多同一把键，`#[serde(default, skip_serializing_if = "Option::is_none")]`：旧账本没有它的行读作 `None`，重开的城取缺省，从没被定过的端点写出的字节与今天相同。
- **缺省**：`None` 时取 `vendor_in_flight(base_url)`，一张住在 `gateway::concurrency` 的小表；今天没有厂商在文档里给出并发数（厂商给的是每分钟请求数与 token 数），所以表里只有一行：`reach::is_local` 认作这台电脑上的服务器取 `IN_FLIGHT_LOCAL`（4，本地推理服务器的并行槽位常见的缺省），其余取 `IN_FLIGHT_DEFAULT`（16）。
- **读数**（Roadmap M2）：每个 `Transport` 记三样，`EndpointBook::queue_readings()` 按端点名一次读出 `QueueReading { endpoint, limit, in_use, queued, waited: WaitTally }`——`queued` 是此刻排队的号数，`waited` 是自启动以来每次排队等了多少微秒的计数直方图（与 `throughput` 台的 `relay_queue` 同一个分桶，p50／p99／p999 与 max 由它读出），没排队就拿到名额的调用记作 0 µs 一次，于是「等待为 0」可以被读出而不是被推断。读数是内存里的计数，不进账本：它说的是这一个进程的排队，不是城的历史。
- **预算行**：`tools/xtask/budgets.toml` 加一行 `[provider_queue]`（测量、不设门），`what` 写「一次模型调用在端点名额前从拿号到取到的等待，注入的单调时钟，p50／p99／p999 与 max」，读数来自 TP1 吞吐台在 N = 16 与 64 时的 `queue_readings()`。

**理由**：上限是 User 对一个端点定的规矩，与 `timeout_ms` 同属 tuning，随端点上线、随 `endpoint_attached` 进账本，重放才能说出某次调用是在什么上限下排的队。缺省写成一张表而不是一个常数，是因为本地服务器与云端的可承受并发差一个数量级；表今天几乎是空的，因为编一个厂商没写的数比给一个保守的缺省更糟。排队读数放在 gateway，是因为排队只发生在这里（D17）；不入账，是因为每次调用一行等待会让账本随调用数线性长，而它回答的是「这个进程现在堵不堵」。

**被否**：①上限住在城的 `CONFIG.toml`：一个端点的规矩分在两处，attach 的那一刻看不见；②每次排队写一行事件：账本行数翻倍，回答的问题只在进程活着时有意义；③缺省按厂商的 RPM 换算：RPM 说的是速率不是并发，换算要假设每次调用多长，而那正是不知道的量。

**重开参数**：厂商开始在文档或响应头里给出并发余量时，`vendor_in_flight` 加行或改读响应头（D17 的重开参数）；`IN_FLIGHT_LOCAL` 是推断值，一个常量。页面在 provider 一处显示排队，要一件线上字段，由接线之后、读数第一次有读者的那一次改动加（wire 的下一条决定）。

**三个平台**：配置、载荷与读数都与平台无关；直方图按注入的单调时钟计时，三个平台相同。
-/

/-!
### 8-14 gateway 目录化（形状：主类型居索引，方法按簇归文件）

`credential`、`dialect`、`endpoint`、`router` 各是一个目录：主类型居索引文件，方法按职责簇归文件（例如 `endpoint/config.rs` 管类型与构造，`call.rs` 管往返，`model.rs` 管 `kernel::Model` 的门）。跨文件的私有项开 `pub(crate)`，对外拼写不变；文件清单以 ARCHITECTURE 的模块图为准。
-/

/-!
### 8-31 重试上限住 kernel

本 crate 直接用 `kernel::Retries`，不导出别名。缺席的含义（`UntilHalted`）、探测在无人可停时读成一次（`without_a_brake`）、以及记进账本时写不写这个数（`stated`），三条都由那一处定义，本 crate 不自持一份。
-/

/-! ## 9 工作流程

回合层组 `ChatRequest`（prefix 四段＋窗口历史）→endpoint.call（dialect 翻译＋兑付＋HTTP）→cost.settle→model_returned 载荷（usage＋billed）→attribution（storage 侧）摊回。credential 独立线：启动 probe→set（`PutSecret` 命令）→resolve（组请求末格）。
-/

/-! ## 10 实现逻辑

dialect 先行（纯函数零依赖，golden 钉形）→endpoint 骨架（假 provider 服务回环测试）→credential（Vault 两适配器＋探测）→market/cost（纯判定）。每步红先行：golden 未落前不写翻译分支。

**两个设计（crate 级）**

**A（选中）：canonical 会话类型住 kernel::model 缝上，dialect 只做翻译**——ScriptModel（citysim）与真适配器消费同一请求形，重放重建的入窗字节有唯一权威；代价是 kernel 公开面变大（约十个纯数据类型）。
**B（落选）：canonical 类型住 gateway，Model::call 只收哈希、真会话经旁道传递**——kernel 面最小，但旁道即第二权威：sim 与真适配器走不同请求形，dialect 往返与 A15 重建无从对同一对象断言；缝的意义（「同一批产品 crate，换适配器整城可跑」）被掏空。落选。
**endpoint 客户端选型**：reqwest::blocking（选中）vs 自写 hyper 直连 vs ureq。自写 hyper＝维护整个连接池与 TLS 面，收益为零（我们自写的是**线格式**不是传输层）；ureq 更小但 rustls 集成与代理面弱于 reqwest；blocking 而非 async＝与同步 turn 面同构，避免为一个 HTTP 调用引入全库 async 传染。

**代价：模型体验**

零字节：gateway 全体不产 prefix 字节。间接贡献：dialect 保断点位使 city-wide 缓存在真 provider 上成立（省的是每回合重付的 prefix 费）；cost/attribution 使「钱花在哪」可答而不占模型上下文（成本归因是给人看的）。
-/

/-! ## 11 边界枚举

空 messages（合法：首轮）；空 tools（不写 tools 键）；SSE 半流（E_PROVIDER，不产部分 ModelReturn）；429 携 retry-after；usage 缺席（取 0，CostSource=PriceSheet）；权威计费额为 0（合法，免费档）；base_url 尾斜线；overrides 指向不存在的路径（创建）；平台服务探测失败（会话内存＋provider_degraded）；遮蔽写入；空串凭证（视同未配置）。
-/

/-! ## 12 错误处理

一次失败的调用方看到的是一个 `AxError`：动作、主题、稳定的码与恢复语。provider 侧的失败经 `endpoint::failure::ProviderFailure` 一处成形（D2、D3，`crates/gateway/spec/Endpoint/Failure.lean`），本 crate 用到的码各有一条决定说明它为什么不可定义掉：
-/

/-! D1 `E_PROVIDER`

不可定义掉——网络与对端是本 crate 的本质失败面；subject 写状态码与端点名，恒不含请求体。
-/

/-! D4 `E_WIRE_MISMATCH`

不可定义掉——对端响应形状漂移是外部事实；subject 写键路径。
-/

/-! D5 `E_ENDPOINT_DIALECT_UNSUPPORTED`

不可定义掉——用户可配任意 external provider，兼容格式探查失败必须可报。
-/

/-! D6 `E_CREDENTIAL_MISSING`／`E_CONFIG_INVALID`

kernel 已有码，语义照 Custody 一节；不新增码。
-/

/-! D7 `E_SECRET_EGRESS`

本 crate 不产（出口扫描住 gate::egress 与 checkpoint 面）；endpoint 组请求不做二次扫描（Custody 在入口已换引用，纵深由门守）。
-/

/-! 每一条决定住在哪里：

- D1 `E_PROVIDER`：本文件 §12
- D2 「能否再试一次」只有一个家：`endpoint::failure::ProviderFailure`：`crates/gateway/spec/Endpoint/Failure.lean`，在 `retry` 与 `refusal` 正上方
- D3 窗口溢出是自己的一族：`Overflow`：`crates/gateway/spec/Endpoint/Failure.lean`，在 `retry` 与 `refusal` 正上方
- D4 `E_WIRE_MISMATCH`：本文件 §12
- D5 `E_ENDPOINT_DIALECT_UNSUPPORTED`：本文件 §12
- D6 `E_CREDENTIAL_MISSING`／`E_CONFIG_INVALID`：本文件 §12
- D7 `E_SECRET_EGRESS`：本文件 §12
- D8 crate 根只再导出有别的 crate 叫得出名字的项：本文件 §7
- D9 请求形状在主机名下白盒证明，不靠截获代理：`crates/gateway/spec/Reach/Resolve.lean`
- D10 TLS 后端的权威是一次调用，不是一个 feature：`crates/gateway/spec/Reach.lean`
- D11 一个端点的连接不在派活时预热（§8-35）：`crates/gateway/spec/Endpoint/Transport.lean`
- D12 转写是一项设施，两条路用它：`crates/gateway/spec/Transcribe.lean`
- D13 OCR 是一项设施，形状照转写：`crates/gateway/spec/Ocr.lean`
- D14 一个模型收得下什么只在 `provider::input` 判一次（§8-37）：`crates/gateway/spec/Provider/Input.lean`，在 `accepted_input` 正上方
- D16 人那一档是 `SelectModel` 的一个可选字段，出现即作答，缺席即「这一次没人说」（§8-37）：`crates/gateway/spec/Provider/Input.lean`，在 D14 之后
- D15 凭证库经 `keyring-core` 与各平台 store 接入，不经 `keyring`：`crates/gateway/spec/Credential.lean`
- D17 每个端点一个并发上限：可配置，缺省取厂商文档的值，遇 429 收窄、恢复后放宽：本文件 §8-6 之后
- D20 名额的接线：一个注入的单调时钟，取在门里、按先来后到排队，等待有界：本文件 §8-6 之后
- D21 `max_in_flight` 进端点的 tuning 与 `endpoint_attached`；排队数与等待时长是每个端点的一份读数：本文件 §8-6 之后
-/

/-! ## 13 依赖选型

- `reqwest`（workspace 依赖，`default-features = false`，features 以根 `Cargo.toml` 为准：`blocking`／`rustls-no-provider`／`json`／`http2` 等）——TLS 钉 rustls；blocking 理由见 §10；后端由 `reach::tls` 装（§8-15）。
- `keyring-core = "1"`，与按目标平台声明的 `windows-native-keyring-store`（关默认 feature，不要 `search`）、`apple-native-keyring-store`（feature `keychain`）、`linux-keyutils-keyring-store`——平台凭证服务绑定；全部 MIT OR Apache-2.0。理由见 D15。
- `rustls`（workspace 依赖，`default-features = false`，feature `std`／`tls12`／`aws_lc_rs`）——只为点名进程的加密后端（§8-15）；它本就经 reqwest 在锁里。
- **`idna` 的 Unicode 后端留在 ICU4X，不钉 `idna_adapter`。** `url` 经 `idna` 规整主机名，`idna` 按锁里 `idna_adapter` 的版本线选后端：1.2 线是 ICU4X，1.1 线是 unicode-rs；本 workspace 不点名 `idna_adapter`，锁取 1.2 线。理由：发行二进制的体积在 64 KiB 以内才换后端，而实测换到 unicode-rs 使它增长 126,976 B（`just build-web` 后 `cargo build --release -p sprawling --features sprawling/sandbox --locked`，13,138,432 B 到 13,265,408 B，windows-x86_64）。被否的备选：`idna_adapter = "~1.1"`（unicode-rs）——锁里少 16 个包，一个 ASCII 主机名走 `idna` 的 ASCII 快路径不进后端，但二进制增长超过上限；1.0 的空后端——拒绝非 ASCII 域名，上游明说不推荐。**重开参数**：unicode-rs 一侧的体积差回到 64 KiB 以内，或 ICU4X 一侧出现一条只能靠换后端清掉的公告。
- `secrecy`／`zeroize`：workspace 既钉。serde_json：wire 面。
- **不引 tokio**：理由见 §3。

规格本身不加依赖：分部只 import 工具链的库与本 crate 的分部（ARCHITECTURE.md §3 的 `depmap` 允许 gateway 引 kernel 的分部，kernel 的规格迁到 Lean 之前没有可引的）。
-/

/-! ## 14 硬编码声明

`transcribe::recording::RECORDING_MAX_BYTES = 25 MiB`（§8-12：OpenAI 音频面自己印的上限，provider 侧工程参数，非城策口径，不入 `consts_policy`；改须本规格同集）与 `wire::BOUNDARY_SEED`（同上，分界线种子）；`EndpointTuning::DEFAULTS` 三个默认值（§8-16，端点调优默认值的唯一住处，改须本规格同集）；market 内置目录（`builtin()`，收录城内实际使用的模型行）。这些全是 pub(crate) 数据面，改动须本规格同集变更。
-/

/-! ## 15 影响面

kernel::model 持 canonical 会话类型（`crates/kernel/Spec.lean` §8-24）；runtime 回合层消费 ChatRequest；storage::attribution 消费 model_returned 的 usage 与 billed 字段；citysim ScriptModel 收同一个 ChatRequest（同一缝）。
-/

/-! ## 16 测试与约束

golden：两 Dialect 各一请求一响应（insta）；proptest：响应往返、usage 保值、断点位保序；endpoint 对回环假服务的状态码矩阵（200/429/500/截断体）；credential：内存 Vault 全流程＋探测注入；cost 权威胜出＋溢出拒绝。约束：clippy 零告警；无 `unwrap`；`.expose(` 只在 `EXPOSE_WHITELIST` 所列文件。

形式化的义务由证明清偿：`lake build crates.gateway.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查，它们是行为比对，不是精化证明：

- 输出上限的梯子：`provider::ceiling` 的测试（`the_higher_rung_wins_and_says_that_it_did`、`on_a_face_that_takes_no_figure_the_provider_picks_when_nobody_stated_one`、`a_relay_forwarding_a_vendors_id_is_called_at_the_vendors_ceiling`、`an_unknown_model_at_an_unknown_host_is_still_callable`）。
- 收得下什么：`provider::input` 的 `the_first_rung_that_states_a_fact_answers`，与 accounting 的 `a_preset_model_that_reads_pictures_is_registered_as_reading_them`（选型点的生产路径）。
- 预置表的行：`provider::preset` 的测试（`a_documented_model_is_matched_by_the_longest_prefix_that_fits`、`a_relay_forwarding_a_vendors_id_reads_the_vendors_row_and_a_local_server_does_not`、`no_pinned_catalogue_row_is_also_matched_by_this_table`）。
- 能否再试：`endpoint::failure` 的测试（`what_never_completed_is_asked_again_and_what_was_refused_is_not`、`a_provider_that_says_busy_or_broken_is_asked_again_and_one_that_refuses_is_not`、`a_request_that_outgrew_the_window_is_told_how_to_fit_again`）。
- 结算：`cost` 的测试（`the_authoritative_amount_always_wins`、`the_price_sheet_computes_integer_shares`、`overflowing_settlements_are_errors_not_wraps`）。

只有节注释的分部，其要求由类型与 `cargo nextest run -p sprawling-gateway` 的各模块测试守住。
-/

/-! ## 17 文档关系

模块登记在 ARCHITECTURE 的模块图（`xtask modmap`）；canonical 类型的改动与 `crates/kernel/Spec.lean` §8-24 同一变更集。endpoint 的生产消费者是 runtime 回合层与 `bin::assembly`；credential 的消费者是 endpoint 与 `PutSecret` 命令。

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 gateway 各行的 `spec` 锚点一起重看。
- `architecture.toml` 的模块图：gateway 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- kernel 的规格（`crates/kernel/Spec.lean`）：canonical 会话类型（§8-24）、`InputKinds`、`ModelFacts`、`Ceiling`、`Retries` 与码表的权威；它们改了，这里的模型与 §8 相应各节一起重看。
- sprawling 的规格（`crates/sprawling/Spec.lean`）：选型点的窗口梯（8-71）、城的工具 `transcribe`（8-131）与 `ocr`（8-142）；它们读本规格的 §8-12、§8-17、§8-34、§8-37。
- 引本规格的其他规格与 rustdoc 写 `crates/gateway/Spec.lean §8-n` 或 `gateway D<n>`；一节换了分部，它的标签不变，引用不必改。
-/
