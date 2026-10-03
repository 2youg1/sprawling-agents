-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::keying

规定 `crates/sprawling/src/keying.rs`：面向网络的那扇门自己铸的钥匙（`bin::keying`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::keying` 旁的测试守住。
-/

/-!
## 8-22 面向网络的那扇门自己铸钥匙，页面把钥匙递上去

**原因（一条端到端全断的链，四段里断三段）**：把 WebUI 暴露到回环之外这件事，今天**做不到**。

| 段 | 今天 | 依据 |
|---|---|---|
| 铸 | `PairingToken::mint` **在产品里没有调用方**，只有测试用它 | `grep mint(` 只命中 `wire/tests` |
| 拒 | `decide_bind` 在没有 `SPRAWLING_PAIRING_TOKEN` 时拒绝任何非回环地址 | `server.rs:63` |
| 携 | `console::web_url` 把 `?token=…` 挂到 URL 上 | `console.rs` |
| 递 | `web::app` 写死 `Link::new(None)`，且 `socket_url()` 只取 `host`，**查询串整段丢掉** | `app.rs:1679`、`socket.rs:311` |

于是：不配置令牌起不来；配置了令牌，页面握手时不出示任何东西，`decide_handshake` 照 `server.rs:306` 拒之。**一座暴露出去的城连自己的 WebUI 都进不来。** 这不是两个缺陷，是一条链，所以一次修完，不留「钥匙铸出来了但没人能用」的中间态。

**对用户提案的更正，记为落选方案**：「BLAKE3 随机抽一个文件的 hex 值当口令」的熵是 `log2(候选文件数)`，不是摘要宽度——十万个文件约 17 bit，可当场穷举；且被抽中的文件内容常常是公开的（仓库里的源文件、依赖的许可证）。攻击者只要知道文件集合就把 256 bit 的外观还原成一次目录枚举。`PairingToken::mint` 收 32 字节 OS 熵、经 29 符号字母表给出四组五位（约 97 bit），**且它的 doc 明写就是为「显示一次」而设**。这里用它，不另造。

```rust
// bin::keying（形状 1 decision；纯，穷尽，无 I/O、无熵）
pub(crate) enum Keying {
    /// 回环：运行中的机器自己，什么都不用出示。
    NothingToPresent,
    /// 人配置过的：我们没见过它被铸出来，故不显示。
    Adopt,
    /// 这次服务当场铸一把，显示一次，不落任何地方。
    Mint,
}
impl Keying { pub(crate) fn decide(bind: SocketAddr, configured: bool) -> Self; }

// bin::assembly（形状 4 适配器；熵在这里取，与 `random_token` 同一处出身）
pub enum Keyed { NothingToPresent, Adopted(String), Minted(String) }
pub fn key_for(bind: SocketAddr, configured: Option<String>) -> Result<Keyed, AxError>;
```

- **`decide_bind` 不动**。它仍是那条守卫；assembly 只是**在 socket 存在之前先把它满足了**，所以它 doc 里那句「there is no window in which the port is open and unauthenticated」原样成立。拒绝臂在 `wire` 自己的测试与任何第三方 embedder 处仍可达。
- **人配置过的优先**。`SPRAWLING_PAIRING_TOKEN` 在场就 `from_configured` 采纳，不覆盖、不显示——我们没见过它被铸出来，印它就是把一个长期口令又抄进一处日志。
- **铸出来的 code 不落盘、不进 Ledger、不进 diagnostics**。进程结束即失效，这就是「一次性」。它只经两处：显示一次的那一行，与 `Terminal.token`（`/web` 据此拼出带钥匙的 URL）——后者今天已经持有明文，不扩大它的存放面。
- **页面这一段是纯函数加一次读**（Humble Object，ARCHITECTURE.md §9）：`web::socket::token_in(search) -> Option<String>` 对查询串取值，可在非 wasm 目标上测；`pairing_token()` 只在 wasm 下多一次 `location.search()`，不含判定。`app.rs` 的 `Link::new(None)` 改为 `Link::new(crate::socket::pairing_token())`。
- **令牌留在查询串里是既有取舍的延续**：`console::web_url` 的 doc 已写明这一取舍（「A token in a query string is a token in the browser's history, and that is the trade this makes deliberately」），替代方案是人在两个窗口之间手抄一个秘密，然后抄错并粘到更糟的地方。改存 `sessionStorage` 会把钥匙放进同源 JS 读得到的地方——对一座**公网暴露**的城，那比浏览器历史更坏，故不改。

**红（三条，每条咬住一段）**：`Keying::decide` 对四格（回环／暴露 × 配置过／没有）给出的枚举——改动之前 `keying` 不存在，是编译红；`token_in` 对 `?token=abc`、`?a=1&token=abc`、`?token=`、空串的四个答案；以及 `web::socket` 那条握手测试，断言 `Link::new(token_in(...))` 发出的 `Hello.token` 非空——改动之前 `Link::new(None)` 使它恒 `None`。端到端那一段（真浏览器对真暴露端口）落在 V9，是人跑的命令而非门禁，如 ARCHITECTURE.md §11 所记。
-/
