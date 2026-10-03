-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::reach::resolve

规定 `reach::resolve`（`crates/gateway/src/reach/resolve.rs`）：把一个预置主机的名字解析到回环替身，按「主机 × face」证请求形状（只编进测试）。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::reach::resolve::tests` 守住。
-/

/-!
### 8-32 `gateway::reach::resolve`：把一个预置主机的名字解析到回环替身，按「主机 × face」证请求形状（只编进测试）

```rust
// reach::resolve —— #[cfg(test)]，整个模块不进发行的二进制
pub(crate) struct StandIn { /* 回环监听、为一个主机名签的自签证书、记下听到的请求 */ }
impl StandIn {
    pub(crate) fn listening(host: &str) -> StandIn;          // 为 host 签一张证书，在 127.0.0.1 上听一次 TLS 连接
    pub(crate) fn toward(&self) -> impl Fn(ClientBuilder) -> ClientBuilder + Send + Sync + 'static;
                                                             // 构造后的一步：resolve(host, 回环地址)＋只信这张证书＋不走代理
    pub(crate) fn heard(self) -> Heard;                      // 请求行的路径、每个头、正文
}
// endpoint::transport —— #[cfg(test)]
impl Transport { pub(crate) fn detoured(step: impl Fn(ClientBuilder) -> ClientBuilder + Send + Sync + 'static) -> Transport; }
```

- **请求形状随主机名而变，所以证明它的替身必须在主机名下被够到。** 会话头（`session_header`）与 chat 面的拼法（`chat_spelling`）按 base URL 的主机查 `PRESETS`；一个以 `127.0.0.1` 为地址的替身永远触发不了这两条，`x-opencode-session` 写到线上的那一步因此从未被任何测试看见。`StandIn::toward` 给 `client_for` 交出的 builder 加三步：reqwest 的 `resolve` 把主机名指到替身的回环地址；`tls_certs_only` 只信替身那张证书；`no_proxy`，因为城所在电脑的代理设置（环境变量或系统设置）会把请求连同主机名一起交给代理，`resolve` 就不再起作用。base URL 一个字节不改，于是主机、路径、TLS 的 SNI 与 `Host` 头都是真请求的。
- **`client_for` 的签名不变，覆写是构造之后的一步。** `client_for` 是全工作区唯一放行的 reqwest 构造点（§8-15），它的调用者在别的 crate；覆写做成一个 `Fn(ClientBuilder) -> ClientBuilder`，由 `Transport::detoured` 带着；`Transport` 第一次为某个端点建客户端时，把它接在端点自己的配置（`client_for` 加上超时）之后、`build()` 之前。这一步与带着它的字段都只编进测试。端点的配置只在 `transport` 里写一次，生产路径与测试路径走同一个函数。
- **轮次从 `PRESETS` 算出，不另抄名单。** 测试对 `PRESETS` 的每一行、行里的每一个 face 各跑一轮：`router::normalise_entered` 按 face 规整出存下的 base URL，`provider::registry::resolve` 给出连接种类，凭据头由 `AuthSpec::for_dialect` 定，经 `adapter_for` 造出模型并 `call` 一次——与 `accounting` 的生产路径走同一串函数，只差 `Transport`。每一轮比一个整值：请求路径（face 的路径加上兼容格式自己的那一段）、`Host`、凭据头的名字与值、会话头（该主机要求的那一个在，别的主机的都不在）、正文的兼容格式（`input` 是 Responses，顶层 `system` 是 Anthropic，其余是 chat）、chat 面的上限字段名。期望值的三段兼容格式路径与上限字段名写在测试里，它们是厂商文档的说法，是测试拿来比的期望，而不是第二份实现。
- **替身是 TLS 的，证书自签、只为那一个主机名。** 证书由测试按 DER 拼出（Ed25519，签名用 rustls 的 aws-lc-rs 后端），不引新包：锁里没有生成证书的 crate，而一张 v3、带 `subjectAltName`、不带扩展用途的叶证书只有几十个字节的结构。私钥是一个固定的 32 字节种子，因为这张证书除了这一次回环握手什么也不保护。
- **它不跑真 provider。** 真 provider 的一轮要人在测试时填 key（D40），由 `just e2e` 按同一张表跑（`crates/sprawling/Spec.lean` §8-69）。
- **失败**：本模块只在测试里；替身的读写失败按测试的失败处理。改了道的 `Transport` 建客户端失败时，与没改道的同一个码（`E_CONFIG_INVALID`）。
-/

/-! D9 请求形状在主机名下白盒证明，不靠截获代理

决定：测试经 `reach::resolve` 把预置主机解析到回环上的 TLS 替身，按「主机 × face」逐轮比请求的路径、头与兼容格式（§8-32）。理由：会话头与 chat 拼法按主机名决定，回环地址触发不了它们；一个在城旁边截获真请求的代理要拿着真 key 过手，而 key 只在人测试时才有（D40），截获的记录本身又是一份要保护的东西。被否的备选：①截获代理（见上）；②给 `client_for` 加一个解析参数——那会改一个跨 crate 的签名，让生产调用者为一个只在测试里用的能力多传一个值；③把查表从主机名改成可注入——那是为测试改生产的判定，替身证明的就不再是生产那条路。重开参数：生产里出现第二个需要按主机名改道的调用者（例如一台要把某个主机钉到固定地址的机器），那时把这一步提成 `client_for` 之外的一个公开入口并写进 §8-15。
-/
