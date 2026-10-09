-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::reach

规定 `reach`（`crates/gateway/src/reach.rs`）：一次分段读数；哪些调用走这台电脑的代理；进程唯一的 TLS 后端。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::reach`、`gateway::reach::proxy` 旁的测试守住。
-/

/-!
### 8-15 `gateway::reach`：一次分段读数，以及「哪些调用走这台电脑的代理」（形状 4 适配器）

```rust
pub fn reach(client: &reqwest::blocking::Client, rule: Proxying, base_url: &str, elapsed_ms: u64) -> kernel::Reach;
pub fn client_for(rule: Proxying, base_url: &str) -> reqwest::blocking::ClientBuilder;   // reach::proxy
pub fn through(rule: Proxying, base_url: &str) -> kernel::Through;                       // reach::proxy
pub(crate) enum Locality { Machine, Private, Public }                                     // reach::proxy
pub(crate) fn locality(base_url: &str) -> Locality;                                      // reach::proxy
pub fn is_local(base_url: &str) -> bool;                                                 // reach::proxy：locality == Machine
```

- **分段怎么测**：没有代理适用时，先用 `to_socket_addrs` 解名、再用 `TcpStream::connect_timeout` 开一个套接字，两段各自成一个读数；随后无论如何都发一次真实请求，把它的错误链摊平成一行，按其中出现的字样归到「主机名不能进握手」「握手失败」「压根没到握手」三类之一，或者归到它答的状态码。**分类读的是链而不是 reqwest 的 `is_connect`**：一个被拒的套接字与一张不受信的证书在那个判断下是同一个答案。
- **`socks` 不花钱**：reqwest 0.13 的 `socks = []` 是空 feature，实现就在它自己的 `connect.rs` 里，锁文件不多一个包。`system-proxy` 只在 Windows 与 macOS 各拉一个读系统设置的包。
- **全城的 HTTP 客户端都在 `reach::proxy` 里造**（`client_for`）。同一条规则写在五处就是五条规则，它们一直一致到其中一处被改为止；更要紧的是，分段读数若自己再判一次，它报出的就是一条请求不会走的路——而那正是看报告的人唯一无法自己核实的东西：客户端已经 `no_proxy` 了，读数却按环境变量报 `Environment`，就是这种分叉。
- **全进程只有一个 TLS 加密后端：aws-lc-rs，由 `reach::tls` 装一次。** reqwest 取 `rustls-no-provider`，rustls 作为工作区依赖只开 `std`、`tls12`、`aws_lc_rs`；`reach::tls::install_provider()` 用 `std::sync::Once` 把 `rustls::crypto::aws_lc_rs::default_provider()` 装成 rustls 的进程默认，`client_for` 在交出 builder 之前调它。于是用哪一个后端是一次显式的调用，不是 reqwest 某个 feature 的副作用；远程门（`remote_access`）的握手与封装原语走工作区里同一行 `aws-lc-rs`、同一组 feature，编出来是同一份 AWS-LC；进程里任何别的 rustls 使用者读 `CryptoProvider::get_default()`，得到的也是它。**绕开 `client_for` 造客户端被机械地挡住**：`clippy.toml` 的 `disallowed-methods` 列出 reqwest 的构造入口与 `CryptoProvider::install_default`，唯一的例外是 `client_for` 与 `install_provider` 各自那一处 `#[expect]`。挡的理由是具体的：`rustls-no-provider` 下，一个没经过 `client_for` 的 builder 在 `build()` 时找不到进程默认，reqwest 会 panic，而发行 profile 是 `panic = "abort"`。
- **默认把打到这台电脑的调用摘出代理，但那是默认而不是定理**（`Proxying::ExceptLocal`）：开了 system-proxy 之后，一台配了代理的机器会把回环也送进代理，本地推理服务器由别人的网关代答 502。但把它写死就是替所有人做了一个只对大多数人成立的决定，而这一类决定失效时没有任何一屏能告诉人到底发生了什么。它是 `EndpointTuning.proxying` 的默认值，另两个值各自对应一类真实的机器（`crates/kernel/Spec.lean` §8-50），而无论哪一个，读数都会把结论写在 `through` 那一格里。
- **工具服务器用默认值，且是显式地用**（`agent_protocols::mcp::http`、`agent_protocols::mcp::sse`）：它没有一份属于自己的设置可携。**会重新打开这一条的参数**：出现一个必须经代理才能够到的回环 MCP 服务器——到那时 `McpServer` 也要长出这一字段，而不是在这里改常量。
- **`locality` 是全城唯一的那一条判断**（D36）：一个地址落在这台电脑（`Machine`）、它所在的私网（`Private`）还是公网（`Public`），只在 `reach::proxy::locality` 判一次。`client_for` 的代理豁免（`Through::LocalAddress`）读 `Machine` 与 `Private` 两格；地址规整时缺省的 scheme 与兼容格式提示、设置页上那个 `local` 标记、并发与机密楼的判断读 `is_local`，它就是 `locality == Machine`，因为它们问的是「在不在这台电脑上」，不是「出不出这个网」。`Machine`：回环（`IpAddr::is_loopback`，先经 `to_canonical` 把 `::ffff:127.0.0.1` 读成 IPv4），外加 `localhost` 与 `*.localhost`，所以 `127.0.0.2` 在每一处都算这台电脑。`Private`：IPv4 的私网段（`10/8`、`172.16/12`、`192.168/16`）与链路本地段（`169.254/16`），IPv6 的唯一本地段（`fc00::/7`）与链路本地段（`fe80::/10`），名字以 `.local`（mDNS，RFC 6762）或 `.home.arpa`（RFC 8375）结尾。其余都是 `Public`；名字不解析，因为一次 DNS 查询会让判定依赖网络，并把每一个端点的名字送给解析器。两份判断只会在某一处先被改掉时各说各的。
- **每个出站请求报出这座城的名字**：`client_for` 给 builder 设 user agent `sprawling/<CARGO_PKG_VERSION>`，一处写给全城。服务端会拒绝不报名字的请求：有的 provider 要求客户端报自己的名字而不是 HTTP 库的名字，CDN 后面的 MCP 服务器对没有 user agent 的请求答 403。
- **三个平台读代理的方式不同，读数只报本侧看得见的那一部分。** 环境变量（`HTTPS_PROXY`／`HTTP_PROXY`／`ALL_PROXY`，大小写两种，`NO_PROXY` 排除）在 Windows、macOS、Linux 上一样读；reqwest 的 `system-proxy` 另在 Windows 读系统代理设置（注册表里的 WinINet 设置），在 macOS 读 System Configuration，在 Linux 不读别的来源，只剩环境变量。系统设置由 HTTP 客户端自己读，`through` 看不见，所以一格 `direct` 只说「本侧看得见的东西里没有点名代理」，不是说请求一定直连。
- **`NO_PROXY` 照客户端的规则读**（D23）：`through` 读的名单与判定一个主机在不在名单里的规则，都与真正发请求的客户端（reqwest 之下 hyper-util 的代理匹配器）相同，所以读数报的 `Excluded` 就是请求不走代理的那一条路。
- **5 秒一段**：设置页上有人在等，一个在这个时间里答不出来的主机，人要的是知道，而不是继续等。
-/

/-! D10 TLS 后端的权威是一次调用，不是一个 feature

决定：reqwest 取 `rustls-no-provider`，由 `reach::tls::install_provider` 显式安装 aws-lc-rs（§8-15）。理由：reqwest 的 `rustls` feature 顺带打开 `quinn?/rustls-aws-lc-rs`，锁里多出一族从不编译的 quinn；更要紧的是后端藏在一个 feature 名里，远程门与 HTTP 用的是不是同一个 aws-lc-rs，要去读 reqwest 的清单才知道。被否的备选：自建一份 `rustls::ClientConfig`，经 `tls_backend_preconfigured` 交给 reqwest——那要在本 crate 重写 reqwest 已有的平台证书校验与 ALPN 选择，且 reqwest 自己的文档说这条路要求两边 rustls 版本逐一同步，版本一错就是运行期的 unknown TLS backend。
-/

/-! D23 `NO_PROXY` 的读法与匹配规则跟着客户端走：域名按点为界，非 Unicode 的值读作没设

决定：`reach::proxy` 先按 `NO_PROXY`、`no_proxy` 的次序取第一个装着 Unicode 的值（`Exclusions::Listed`）；一个拼法没设就看下一个，一个拼法的值不是 Unicode 也看下一个，两个都没有可读的值时，读数分出「都没设」（`Unset`）与「设了却读不出」（`Unreadable`），两者都不排除任何主机。名单按逗号切开、去掉空白；主机是地址时只比地址项：相等的地址，或装着它的网段（`10.0.0.0/8`、`fd00::/8`）；主机是名字时只比名字项：`*` 盖住一切，一项盖住与它相等的名字，以及以「`.`＋这一项」结尾的名字，项开头的 `.` 意思相同，ASCII 大小写不计。所以 `example.com` 盖住 `api.example.com`，不盖 `badexample.com`。

理由：§8-15 要求读数说的是请求真走的路。客户端（reqwest 0.13 的 `system-proxy` 与环境读取都落在 hyper-util 的 `Matcher`）正是这样读：`get_first_env` 对 `NotUnicode` 与 `NotPresent` 一样跳过，`DomainMatcher::contains` 以点为界、不计大小写，`IpMatcher` 比地址与网段。旧的 `host.ends_with(entry)` 让 `example.com` 排除了 `badexample.com`，读数就报一条请求不走的路；`unwrap_or_default` 把读不出的值和没设混成一个空串，读数与客户端碰巧一致，却说不出这是一个读不出的值。非 Unicode 的值是一次被记下的拒绝：它在类型里是 `Unreadable` 这一臂，`through` 的那一臂注明客户端照样走代理。被否：①读不出时报错——`through` 的答案是 kernel 的 `Through`，它说的是请求走哪条路，而客户端此时照样走代理变量，一个错误会让读数说出一条不存在的路；②把非 Unicode 的值有损地转成字符串再匹配——客户端不这样做，读数又会与请求分叉。重开参数：客户端换了 `NO_PROXY` 的读法或匹配规则（升级 reqwest 或 hyper-util 时读它们的 `NoProxy` 文档与测试），或 kernel 的 `Through` 长出一臂能带「名单读不出」的原因。

三个平台：环境变量在 Windows、macOS、Linux 上一样读；不是 Unicode 的值在 macOS 与 Linux 上是不合法的 UTF-8 字节，在 Windows 上是落单的代理项（UTF-16 的 unpaired surrogate），`std::env::var` 在三处都答 `VarError::NotUnicode`，所以三处读数相同。
-/

/-! D36 私网上的地址与这台电脑一样不走代理：`locality` 分三格，代理判定把 `Private` 当作 `Machine`

决定：`reach::proxy::locality` 返回 `Locality { Machine, Private, Public }`，`through` 在 `Proxying::ExceptLocal` 下对 `Machine` 与 `Private` 都答 `Through::LocalAddress`，`client_for` 据此 `no_proxy`。`Through` 不长新的一臂：kernel 的 `Through` 是线上的读数，`LocalAddress` 的意思是「城按规则把这次调用摘出了代理」，摘的理由在本节写明。理由：局域网上的模型服务器（例如 `http://192.168.1.20:11434`）原来按 `is_local` 判作不在这台电脑上，于是走环境变量里的代理，提示词以明文经过代理的运营方，而这个请求本来不必离开这个网；一个公司代理也到不了一台家里的私网地址，代答的只会是 502。判定仍只在这一处，读数（`through`）与客户端（`client_for`）读的是同一个函数。被否：①把私网地址并进 `is_local`——设置页的 `local` 标记、机密楼只许这台电脑上的端点的判断与并发的缺省会把一台别人的电脑当成这台电脑；②靠人自己写 `NO_PROXY`——默认就把提示词送进代理，人要先知道这件事才能关掉它。重开参数：一个人需要让私网地址也走代理（例如审计每一次调用的公司网关就在私网上）时，他设 `Proxying::Always`，这一条不必改；出现运营商级 NAT 段（`100.64/10`，Tailscale 用它）算不算私网的真实需求时，再在这里加一格判断。
-/
