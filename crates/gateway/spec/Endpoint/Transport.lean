-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::endpoint::transport

规定 `endpoint::transport`（`crates/gateway/src/endpoint/transport.rs`）：每个端点一个 HTTP 客户端；本地推理不另设适配器；第一次调用为连接付多少。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::endpoint::transport` 旁的测试守住。
-/

/-!
### 8-3 本地推理（无独立适配器）

本地模型与远端模型走同一个 `Endpoint`：`adapter_for` 对每个 chosen 只造一种适配器。回环与否在 `Endpoint::new` 里判定一次——它经 `client_for(proxying, base_url)` 建客户端，回环地址默认绕开代理，人给这个端点另定的 `proxying` 照样生效。

- **本地模型也流式输出**：`call_streaming` 是 `Endpoint` 的门，回环端点因此请求里带 `stream: true`，delta 帧逐帧交给调用方。另设一个只实现 `call` 的本地适配器，等于让最朴素的那类端点（回环、无凭证、无覆盖）的回答在模型写完后才一次涌到页面上。
- **机密楼的拒绝不因回环而放宽**：`Endpoint` 的两扇门对 `confidential` 一律拒绝（§8-2），回环端点走的也是这两扇门，所以没有哪条路能让机密楼绕过它。
- **每个 endpoint 一个 HTTP 客户端**（`endpoint/transport.rs`，形状 4 适配器）：`EndpointBook` 为每个登记的 endpoint 持有一个 `Transport`——一个在第一次调用时才建、此后被每次调用克隆共用的 `reqwest::blocking::Client`。`Chosen` 带着它（`pub(crate) transport`），`adapter_for` 经 `Endpoint::over(transport, config, redemption)` 取客户端，于是同一 endpoint 上的每个 run、给工作起名的调用和顾问调用共用一个客户端。理由是进程内的线程数与稳定性：每个 blocking 客户端各起一条 `reqwest-internal-sync-runtime` 线程和自己的连接池，按调用建客户端时，同时跑 N 个 run 就有 N 条这样的线程，每次调用还要重新握手。`EndpointAttached` 重新登记同名 endpoint 时换一个新的 `Transport`（tuning 里的超时与 `proxying` 可能变了），`EndpointLost` 连同它一起删除。`Endpoint::new` 仍为探针和测试建一个只属于自己的客户端，建法与 `Transport` 是同一个函数。同一端点的调用怎样复用连接、第一次调用为连接付多少，见 §8-35。
  - 否决的方案：在折叠 `EndpointAttached` 时立即建客户端。折叠会因传输原因失败，重放整座城时还会为每个 endpoint 各起一条线程，而这些 endpoint 可能一次都不被调用。
- 否决的方案：保留一个包着 `Endpoint` 的本地类型只为在构造时断言回环。它唯一的读者是 `adapter_for` 里那道 `is_local()` 判断——同一个判定两处写，而它什么都不多拦：凭证、头覆盖、体覆盖本来就让回环端点走通用路径。
-/

/-!
### 8-35 一个端点的第一次调用为连接付多少：仪器、读数，与不在派活时预热（`endpoint::transport`，形状 4 适配器）

```rust
// endpoint::transport —— #[cfg(test)]
#[test] fn a_second_call_to_one_endpoint_goes_out_over_the_first_calls_connection();
#[test] #[ignore] fn instrument_first_call_connect();   // 读数；SPRAWLING_CONNECT_PROBE=<base URL>,... 换成真主机
// reach::resolve —— #[cfg(test)]
pub(crate) struct KeptOpen { /* 回环 TLS 替身：每个请求都答 500，连接留着；记下每个请求走的是第几条连接 */ }
impl KeptOpen {
    pub(crate) fn listening(host: &str) -> KeptOpen;
    pub(crate) fn toward(&self) -> impl Fn(ClientBuilder) -> ClientBuilder + Send + Sync + 'static;   // 与 StandIn::toward 同一步
    pub(crate) fn served_on(&self) -> Vec<usize>;
}
#[test] fn a_warm_up_opens_the_connection_the_first_call_goes_out_over();

// endpoint::transport —— 开城时预热（D26）
pub(crate) struct ClientShape { url: String, proxying: Proxying, timeout_ms: u64 }   // 一个端点的共用客户端由它建
pub struct WarmUp { /* 端点 Transport 的克隆、它的 ClientShape、它的 models_url；没有凭据，也没有兑付 */ }
impl WarmUp { pub fn open(self); }
// router::attached
impl AttachedEndpoint { pub(crate) fn client_shape(&self) -> ClientShape; }   // adapter_for 与预热读同一个
// router::book
impl EndpointBook { pub fn warm_ups(&self) -> impl Iterator<Item = WarmUp> + '_; }
```

- **要回答的问题。** 人看到的 TTFT 是 `first_at − model_called.t`（`model_returned` 的首块时刻减调用时刻），它包着这次调用为连接付的钱：解名、TCP、TLS 握手。连接只在一个端点的客户端里没有可用的空闲连接时才付：这个进程里对这个端点的第一次调用，或者上一次调用已经过去了 reqwest 的空闲上限（`pool_idle_timeout` 的默认值 90 s）。fx 研究 R8 要先量这一段，再决定要不要在派活时预热。
- **复用由一条测试守住。** `a_second_call_to_one_endpoint_goes_out_over_the_first_calls_connection`：回环 TLS 替身 `KeptOpen` 在主机名 `api.anthropic.com` 下答每个请求 500、连接留着，记下每个请求走的是第几条连接；同一个 `Transport` 上的两次 `Endpoint::call` 与另一个 `Transport` 上的一次，走的连接是「第二次同第一次、第三次另开」。第三次是对照：它证明替身数得出第二条连接，所以前两次同一条不是数漏了。断言比的是「同不同一条」而不是连接总数：hyper 在旧连接回池之前可能先开一条备用的新连接，那条连接不承载请求，不改变这次调用付不付握手。
- **仪器。** `instrument_first_call_connect`（`#[ignore]`，`cargo nextest run -p sprawling-gateway --run-ignored only -E 'test(instrument_first_call_connect)' --no-capture`）：20 轮，每轮用 `client_for` 新建一个客户端，同一个 URL 连问两次 `GET`、读完正文，两次各计时，两者之差就是第一次为连接付的钱；印出这个进程的第一次、两次各自的中位数与差的中位数（µs）。默认问回环替身；环境变量 `SPRAWLING_CONNECT_PROBE` 给出逗号分隔的 base URL 时改问它们，不带任何凭据（D40：真 key 只由人在测试时填，这一段不需要它）。时钟是测试自己读的，不进任何产品路径，读数不进 Ledger。
- **读数**（Windows x86_64，16 核，debug 构建，20 轮）：

  | 对端 | 第一次 p50 | 第二次 p50 | 差 p50 | 进程第一次 |
  |---|---|---|---|---|
  | 回环替身 | 7.7 ms | 0.6 ms | 7.0 ms | 50.8 ms |
  | `api.anthropic.com/v1/models` | 731 ms | 299 ms | 417 ms | 734 ms |
  | `api.openai.com/v1/models` | 643 ms | 286 ms | 345 ms | 667 ms |
  | `openrouter.ai/api/v1/models` | 1005 ms | 334 ms | 671 ms | 949 ms |

  回环的差是握手在本地的计算（debug 下的 rustls 与 aws-lc），真主机的差几乎全是往返：同一类机器上 curl 量到 TLS 握手完成于发起后 0.26–0.37 s。所以按这组读数，一个端点在 90 s 没被调用之后的第一次调用，TTFT 里有 0.35–0.67 s 是连接。发行构建的回环读数由同一条命令加 `--release` 给出。
- **决定：不在派活时预热。** 派活时开始的预热，最多藏住派活到第一次调用之间的那一段：那一段是 lane 的准备（`crates/sprawling/Spec.lean` §8-113），`[prepare_dispatch]` 的下限 58 ms，工作树放置接管备树之后以毫秒计（`crates/storage/Spec.lean` §8-35）；而要藏的是 0.35–0.67 s。预热还要一次真实请求才能让连接进池（reqwest 没有只建连接的接口），那是每次派活多一次发给 provider 的、不带凭据的请求；在记账线程上发它要等一次往返（8-113 不许），在 lane 里发它与准备串行，什么也藏不住，另起线程又只为藏 58 ms。理由与被否的备选写在 D11。
- **池按端点保持，TCP 一层的保活取 reqwest 的缺省。** reqwest 0.13 给每条连接开 TCP keepalive（15 s 起、每 15 s 一探、3 次；Windows 上探测次数由系统定），三个平台都由 `socket2` 设到套接字上，所以一条对端已断的空闲连接在池里活不过约一分钟，下一次调用不会拿它去等。开城时预热见下一条与 D26。
- **开城时预热**（D26）。`EndpointBook::warm_ups` 为簿里每个已登记端点交出一个 `WarmUp`：那个端点 `Transport` 的克隆（与簿共用同一个客户端槽与名额）、`AttachedEndpoint::client_shape` 读出的建客户端的三样（`chat_url`、tuning 的 `proxying`、`call_timeout_ms`，`adapter_for` 填 `EndpointConfig` 时读的是同一个），以及 `models_url`。`WarmUp::open` 在槽里建这个端点的客户端，再对 `models_url` 发一次 `GET`、把回答读完，于是解名、TCP 与 TLS 握手都已付过，连接回到池里等第一次调用。这次请求不带凭据，这由类型担保而不是由一处判断担保：`WarmUp` 里没有 `AuthSpec`、没有 `Redemption`，也没有人填的额外头，它根本拿不到 key。它不取名额（它不是模型调用，不该挤占 `max_in_flight`），不写账本，也不报告结果：建不出客户端、连不上、回答是任何状态码、正文读不完，都只让这一次预热停下，城照常开，第一次真实调用照旧自己付连接并报告它遇到的错。谁也不等它：`bin::assembly::attending` 在写者报告就绪之后为每个 `WarmUp` 起一条分离的线程 `sprawling-warm-up`，线程在这一次请求结束时结束；预热与第一次调用撞在一起时，两边各建一个客户端，槽留先到的那个（§8-3），什么都不错，只是那一次调用没藏住握手。`a_warm_up_opens_the_connection_the_first_call_goes_out_over` 用 `KeptOpen` 守住：预热之后替身已在第 0 条连接上收到一个请求，随后同一 `Transport` 上的第一次 `Endpoint::call` 走的还是第 0 条。
- **给 U7 前端的话。** TTFT 的读法不变；一次明显偏大的 TTFT，若它是这个端点 90 s 以来的第一次调用，多出来的那一段就是这里的连接，量级见上表。
-/

/-! D11 一个端点的连接不在派活时预热（§8-35）

决定：派活路径上不加预热；同一端点的调用照旧共用一个客户端的连接池，空闲上限取 reqwest 的默认值。理由：派活时开始的预热最多藏住派活到第一次调用之间那一段（lane 的准备，下限 58 ms，放置接管备树之后以毫秒计），而要藏的连接是 0.35–0.67 s（§8-35 的读数）；而且预热要发一次真实请求才能让连接进池，每次派活多一次不带凭据、发给 provider 的请求。被否：①在派活时于记账线程上发一个轻请求——记账线程等一次往返（`crates/sprawling/Spec.lean` §8-113）；②在 lane 的准备开头发——与准备串行，藏不住任何东西；③另起一条线程在准备期间发——为最多 58 ms 加一条每次派活的线程；④把空闲上限拉长并开 HTTP/2 保活 ping，让连接跨过人读答案、打下一句的那段时间——它藏得住整段连接，但一条被中间设备静默丢掉的 HTTP/1.1 连接会让下一次流式调用一直等到静默上限，这一臂没有故障注入测试之前不做。**重开参数**：①城收到「人要开始说话了」这样比派活更早的信号（例如输入框获得焦点经线协议到达），那时在它到达时预热，能藏的是人打字的时间；②有了覆盖静默断连的故障注入测试（发送失败的连接不回池、静默的连接在上限内被认出），那时考虑④；③准备的那一段长到与连接同一个量级。
-/

/-! D26 开城时为每个已登记端点预热一次连接：一次不带凭据的 `GET models_url`，失败只停这一次预热，谁也不等它（§8-35）

决定：城开好（写者报告就绪）之后，装配层为路由簿里的每个端点起一条线程，经 `WarmUp::open` 在那个端点共用的客户端上对 `models_url` 发一次不带凭据的 `GET` 并读完回答；结果不进账本、不报给人，失败只让这一次预热停下。理由：一个端点在 90 s 空闲上限之后的第一次调用，TTFT 里有 0.35–0.67 s 是连接（§8-35 的读数），而开城到人说出第一句之间往往有人读页面、打字的时间，这段时间里连接可以先付掉；reqwest 没有只建连接的接口，所以要一次真实请求，而 `/models` 是三种兼容格式共有、不花 token、也是 §8-35 的仪器量过的那一条路径。不带凭据由类型担保（`WarmUp` 拿不到 `AuthSpec` 与 `Redemption`），因为预热不该在人没开口时动用 key，也不该在供应方那边记一次带身份的调用。被否：①带凭据请求 `/models`——一次 200 能确认 key 还活着，但那是探针（§8-21）的事，预热把它做了就成了第二处判断凭据的地方，而且开城时无声地兑付每个端点的 key；②对 `base_url` 发 `HEAD`——有的供应方对根路径答 404 或改写到别处，连接未必留在同一主机上，`/models` 是量过的路径；③在写者线程上依次预热——写者要等每一次往返；④只预热有已选模型的端点——人刚登记、还没选模型的端点正是下一次调用最可能去的地方，省下的请求不值这条分支。**重开参数**：①量到开城到第一次调用的间隔多数超过 90 s（那时预热的连接在调用前已被空闲上限收回，这批请求白发）；②有供应方把这批不带凭据的请求计入限流或封禁来源地址；③D11 的第一条重开参数成立（城收到「人要开始说话了」的信号），那时在信号到达时预热比在开城时更准。
-/
