-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::reach

规定 `kernel::reach`（`crates/kernel/src/reach.rs`）：一次调用停在哪一段。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-50 `kernel::reach`：一次调用停在哪一段（形状 2 值）

```rust
pub enum Named { Resolved(u32), NotFound, Refused(String), ProxiedAway }
pub enum Connected { Open, Refused, Silent, Failed(String), Skipped }
pub enum Answered { Status(u16), NameNotUsable(String), HandshakeFailed(String), Unreachable(String) }
pub enum Through { Direct, Environment(String), Excluded, LocalAddress, Disabled }
pub enum Proxying { ExceptLocal, Always, Never }
pub struct Reach { host, named, connected, answered, through, elapsed_ms }
```

- **为什么值在 kernel 而读数在 gateway**：这套词汇要同时被 gateway（做测量）与 wire（往线上送）叫出名字，而 wire 不依赖 gateway。与 `DialectKind` 同一条依赖倒置：**定义住在这里，求值住在拿得到套接字的那一层**。
- **四段各有各的下一步**：名字解不出（检查拼写或代理）、连不上或没人应（防火墙、端口、没起来的代理）、握手失败（主机名不是合法 DNS 名、证书不受信）、供应方答了状态（401 是密钥，404 是 base_url 末尾多了路径）。**一条 `error sending request for url (...): operation timed out` 里这四种全长一个样**，而人对着它无事可做。
- **`Resolved(0)` 不可表达**：解出零个地址就是 `NotFound`，不是「解出了，零个」。
- **`ProxiedAway` 是一段诚实的缺席**：有代理时名字与套接字都由代理去做，城自己再解一次名，报的是一条请求不会走的路。
- **`Through` 的五格里有四格都是「没走代理」，分开是因为下一步不同**：没人指定（`Direct`）、这台电脑自己的 `NO_PROXY` 排除了它（`Excluded`）、城按规则把打到这台电脑的调用摘了出来（`LocalAddress`）、人为这个端点定下了 `Never`（`Disabled`）。一个人对着 `direct` 无事可做，对着这四句里的任何一句都有。
- **`Proxying` 是一条设置而不是一个常量**：「打到这台电脑的调用不走代理」对常见的那一类机器是对的——代理拦回环会让本地推理服务器由别人的网关代答 502——但它对每一台机器都成立这件事从来没有被证明过。另外两格各自对应一类真实的机器：把出网一律送进回环上的审计中继、因而要求连回环也走代理的组织（`Always`），以及虚拟网卡已经在路由层接管全部流量、于是一条过期的代理变量只会弄坏调用的机器（`Never`）。**败给的方案**：把它做成城一级的开关——那需要一条命令、一个折叠、一份投影和「改设置要重建哪些客户端」的答案，而端点的 `EndpointTuning` 本来就是「人关于这个端点还定下了什么」的存放处，探测与调用共用它因而恒不会各说各话。
- **时间是参数**：`elapsed_ms` 由调用方盖戳，因为全城只有 Main 采样时钟。
-/
