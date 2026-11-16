-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::server::listener

规定 `server::listener`（`crates/wire/src/` 下同名的文件）。城在开门之前先占住的端口，与在它上面服务。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-46 先占住端口，再交出城：`bind` 与 `serve` 分成两步

```rust
pub struct Bound { /* listener: tokio::net::TcpListener, local: SocketAddr, face: BindFace —— 私有 */ }
/// 判定绑定面，再绑定监听器。判定拒绝时不碰网络。
pub async fn bind(addr: SocketAddr, token_digest: Option<B3Hash>) -> Result<Bound, AxError>;
impl Bound {
    /// 监听器实际占住的地址：`addr` 的端口是 0 时，这里是系统给的那个端口。
    pub fn local_addr(&self) -> SocketAddr;
}
/// 在已经绑定的监听器上服务，直到 future 被丢弃。
pub async fn serve(bound: Bound, config: ServeConfig) -> Result<(), AxError>;
```

- **次序是装配层要的**：一座城先占住它的端口，然后才打开写者、写下第一行（`crates/sprawling/Spec.lean` §8-88）。`ServeConfig` 里的 sink 要等写者线程开好才造得出来（转写 sink 借的是写者打开的那个金库），所以「绑定」必须能在 sink 存在之前单独做完；若两件事合在一个 `serve(config)` 里，装配层只能先开写者、再在 `serve` 里发现端口已被占用。
- **`addr` 与 `token_digest` 离开 `ServeConfig`**，成为 `bind` 的两个入参。它们只被绑定判定读过；留在 `ServeConfig` 里，同一个地址就会有 `bind` 的入参和配置字段两个家。
- **失败码**：判定拒绝是 `decide_bind` 的 `E_CONFIG_INVALID`；操作系统拒绝绑定也是 `E_CONFIG_INVALID`，recovery 是「换一个空闲端口，或者停掉占着它的进程」。读不出绑定之后的地址同样是 `E_CONFIG_INVALID`，recovery 相同：那个监听器已经不可用。`serve` 在 accept 循环失败时答 `E_STORAGE_FATAL`，recovery 是「重启进程，监听器已经没了」。
- **三个平台**：监听器是 tokio 的 `TcpListener`。端口被另一个进程占着时，Windows、macOS、Linux 都拒绝绑定，`bind` 都答上面那个 `E_CONFIG_INVALID`；在 macOS 与 Linux 上 mio 给监听器设 `SO_REUSEADDR`，只让重启的城能立刻重新绑上一个还处于 `TIME_WAIT` 的端口，不让两个进程同时监听一个端口；Windows 上不设它。端口 0 在三个平台上都由系统给一个临时端口，`local_addr` 读出的就是它。
- **绑定之后的地址由 `Bound` 说出**（D16）：`serve` 被给的地址可以是 `:0`，那时只有监听器知道系统给了哪个端口；装配层要把城的地址交给别处（远程中继连回城的 `/ws`、启动横幅、控制台的 `/serving`、打开浏览器之前的探测），每一处都读 `local_addr`，不读 `serve` 被给的那个。
- **被否：先试绑一次再放掉，然后在 `serve` 里真绑**。试绑与真绑之间，别的进程可以把端口拿走，那样原来的缺陷只是窗口变窄了，并没有消失。
- **两者住 `server::listener`**，不住 `server::socket`：它们管的是监听器本身，先占、再服务；`server::socket` 管的是连上之后的一条 WS 会话；资产与上传各住 `server::bundle` 与 `server::uploads`。`Bound` 带 `#[must_use]`：占住端口而不服务，得到的是一个谁也不应答的端口。
-/

/-! D16 `Bound` 在绑定时读下监听器的地址，`local_addr` 只是读出它

**决定**：`bind` 绑好监听器之后立刻问操作系统它占住的地址，存进 `Bound`；`Bound::local_addr` 不会失败，读出的就是那一次的答案。

**理由**：装配层把城的地址交给四个读者（远程中继、启动横幅、控制台 `/serving`、打开浏览器之前的探测），它们拿到的若是 `serve` 被给的地址，在 `:0` 上就指向 0 号端口，连不到城。监听器的地址在它活着的时候不会变，所以在绑定时读一次就够；读不出时 `bind` 本身就失败，调用方不必在四处各自处理一个读不出的地址。

**被否**：①每次调用 `local_addr` 都去问 tokio 监听器，答 `Result`——同一个不会变的事实问四次，每个读者都多一条永远走不到的失败分支；②装配层先在 `:0` 上试绑一次拿到端口、放掉，再把具体端口交给 `bind`——试绑与真绑之间别的进程可以把端口拿走（见上）。

**重开参数**：一个 `Bound` 在服务期间换绑地址（今天没有这条路）。
-/
