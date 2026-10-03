-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.remote_access.spec.Door
import crates.remote_access.spec.Confirm
import crates.remote_access.spec.Handshake

/-! # remote_access 的规格

`sprawling-remote-access`（库名 `remote_access`，目录 `crates/remote_access`）是远程接入：一个人离开这台电脑时，从外面够到自己这座城——谁能进、进来能做什么、何时失效、如何一键关上。模块：door（门的状态）／confirm（等控制台确认的那一次请求）／pairing（一次性配对码）／keys（混合签名密钥）／handshake（每次连接的握手，以及设备第一次连接时的配对握手）／seal（帧封装）／route（通路缝与它的三个实现）；尚未落地的部分写在 §3。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」：`spec/Door.lean` 证明远程门的六条性质，`spec/Handshake.lean` 证明配对握手的三条性质，`spec/Confirm.lean` 证明控制台确认的三条性质（D4，§8-13）。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定。决定写作 `D<n>`，别处引作 `remote_access D<n>`；D1 到 D21 沿用这份规格在 Markdown 时 §12 的条目号，所以旧的引用改写之后号不变；D22 起是迁移之后的决定。§8 的各条保留 `8-1` 到 `8-12` 的编号，`8-13` 是迁移之后加的，别处引作 `crates/remote_access/Spec.lean` §8-n。

本文件本身是描述，不是被证明的规格：它不含 Lean 定义与定理，被证明的只有三个分部里的性质；其余每一条由 §16 列出的测试判，`cloudflare` 通路由 §8-8 的操作者检查判。

**三个平台**：门、配对码、密钥、两种握手与封装零 I/O，只读参数，在 Windows、macOS、Linux 上逐字节相同（§8-12 的向量在三个平台的 CI 上都读回）。各平台不同的只有两处：城钥匙的种子存在哪里、能留多久，由 vault 的平台表决定（`crates/gateway/spec/Credential.lean` §8-4，D23）；通路起的子进程是各平台自己的 `cloudflared` 或人写的命令，结束它用标准库的 `Child::kill`（Windows 上是 `TerminateProcess`，macOS 与 Linux 上是 `SIGKILL`），三处语义相同。
-/

/-! ## 1 需求分解

一个人把城留在家里的电脑上出门，要能用平板或手机继续看城、回答提问、叫停。今天城只听回环地址，或者在局域网上凭一把配对令牌（`crates/wire/Spec.lean` §8-41）；出了局域网就够不到。本 crate 是 remote access（远程接入），其中的安全内核是远程门（`door::Door`，它的性质由 Lean 模型 `RemoteDoor` 规定）：

| 单元 | 一句话 | 阶段 |
|---|---|---|
| `door` | 门开着还是关着、在哪个纪元；还在等的配对；配过的设备；它们持有的会话 | 1a（已落） |
| `pairing` | 一次性配对码的铸造、给人看的写法、按人抄回来的写法读回 | 1a（已落） |
| `keys` | 城的身份密钥与设备密钥：ML-DSA-44 与 Ed25519 的混合签名 | 1b（已落） |
| `handshake` | 每次连接的混合密钥交换（X25519＋ML-KEM-768）与双方对握手记录的混合签名 | 1b（已落） |
| `handshake`（配对） | 设备第一次连上城：一次只由城认证的握手，配对码封在它里面走 | 2（已落） |
| `seal` | 会话内每一帧的 AES-256-GCM 封装，计数器防重放；负载的首字节分开线协议帧与锁门 | 1b、2（已落） |
| `route`（缝） | 让外面够得到这台电脑回环上的远程监听：开、关；开的时候答出外面用的地址，以及这个地址跨重启是否不变。三个实现：Cloudflare 命名隧道、人写的一条命令、测试用的脚本化通路 | 2（已落） |

**安全与通路分成两层，只有通路可换。** 门、配对、密钥、握手、封装、权限是安全内核，不可换、不可关；通路只负责可达性，默认是 Cloudflare 命名隧道（D3、§8-8），人可以换成一条自己的命令（§8-9）。握手认证的是配对时钉住的密钥而不是地址，配对本身也走一次只由城认证的握手（§8-6），帧又是端到端加密的，所以只搬运字节的通路看到的只有握手消息与密文，冒充不了任何一方，通路选错的最坏结果是连不上。页面本身也经通路送到设备，改写页面的通路能读到页面读到的一切；这一件由选择通路的人承担，密码学替不了（D10）。

D2 后量子放在三处：TLS、设备认证、帧封装。TLS 负责传输层机密性，但 Cloudflare 隧道总在边缘解开 TLS；帧封装让边缘只看到密文，配对码也只在封装里走（§8-6），所以「现在截获、以后解密」对这一层也不成立。
-/

/-! ## 2 验收标准

- **door**：`crates/remote_access/spec/Door.lean` 证明的六条性质在 Rust 门上各有一个场景测试：关着的门什么都不放；关门结束每一个会话，再开也带不回来；配对码只用一次、只在自己的纪元、只在过期之前；撤销的设备不持有会话也开不了新的；会话不比门活得久；远程会话永远够不到只限本地的动词。另加一条：门拒绝在它所在的纪元里重开。`cargo nextest run -p sprawling-remote-access` 全绿，且每条性质的测试在对应实现被故意改坏时转红。
- **pairing**：铸出的码按人重新抄写（大小写、空格、连字符）读回同一个码；码的正文是 16 字节熵的 RFC 4648 base32（26 个符号）；两份熵给出两个码。
- **keys**：同一个种子给出同一把公钥，两个种子给出两把；签名只在两半都成立时成立，换掉任一半即不成立；错误长度的线形式被拒。`written` 写出的正文经 `from_sealed` 读回的公钥等于 `from_seed` 的那一把；长度不对、不在字母表里的正文以 `E_CONFIG_INVALID` 拒。
- **handshake**：两端握手后，一端封的帧另一端打得开，两个方向都是；设备拒绝一个它没有钉住的城；城拒绝一台持有别的密钥的设备；途中被改过的回复被拒；一次握手的 Finish 完不成另一次握手。
- **handshake（配对）**：诚实的一次配对两侧对上：设备拿到城的公钥与会话，城拿到配对码、设备公钥与会话。回答里出示的公钥与邀请里的指纹不符，或回答的签名不是那把公钥签的，设备在发出认领之前停下。被改过的认领、另一次配对的认领、在回答之前到达的字节，城都打不开；设备签名不对的认领，城不交给门。`crates/remote_access/spec/Handshake.lean` 证明三条性质：配对码只封给邀请钉住的城；城只从封好的认领里兑码；诚实的一次配对走得通。无 `sorry`、`admit`、`axiom`。
- **route**：`PublicUrl` 收 `https://` 的地址，拒 `http://`、其他 scheme、没有主机、带用户信息或片段的写法。`command` 通路对一个脚本化子进程答出的 `Opened` 与脚本化通路答出的相同，子进程从环境变量读到回环地址，前面的噪声行被跳过；在印出地址之前就结束的命令、印错地址那一行的命令各得一句带恢复语的拒绝；三个实现关一个没开的通路都答成功。`cloudflare` 通路不进自动测试，它要一个 Cloudflare 账号、一个域名与能连上边缘的网络，由操作者按 §8-8 的检查跑一次。
- **与浏览器互通**：按 D11，组件级已知答案向量在 Rust 与 TS 两侧都过；一条由 Rust 城生成的单向夹具，TS 设备解出约定的明文。文件与写法见 §8-12：Rust 一侧的测试读回每一份，确定的三份逐字节等于重算的结果，签名在 Rust 里验过，夹具的第一帧在 Rust 设备一侧用同样固定的临时私钥打开、得出约定的明文；文件不在时测试在断言上失败。
- **正文写法**：设备 id、公钥与权限各自的正文读回原值；短了的 id 与不认识的权限字被拒（§8-11）。
- **seal**：帧按封的次序打开；被重放、被丢掉前一帧、被改过、方向不对的帧都打不开。负载按首字节读回原样；未知的首字节、锁门后面多出的字节、不是 UTF-8 的帧、空负载都以 `E_WIRE_MISMATCH` 拒。
- **控制台确认**（D4）：`crates/remote_access/spec/Confirm.lean` 证明三条性质：一个动词只在回答紧挨着它的那次请求时才做，答的是那次请求印出的码、同一个动词、在到期之前；控制台从未印出的回答什么也做不成；按时答回的码做成那个动词。Rust 一侧随模块落地（§3）。
- **Lean**：`just models` 构建 `Spec`；本文件、`crates/remote_access/spec/Door.lean`、`crates/remote_access/spec/Confirm.lean` 与 `crates/remote_access/spec/Handshake.lean` 无 `sorry`、无 `admit`、无 `axiom`。
-/

/-! ## 3 假设与歧义

以下是这个接口今天尚未落地的部分，按阶段写成当前状态；每一段落地时，从这里删掉，写进它所属的 §8 章节。

- **`cloudflare` 通路没有在真的隧道上开过。** 自动测试不跑它（§2）。缺的证据有两件：在一台装了 `cloudflared`、能连上 Cloudflare 边缘的电脑上，按 §8-8 的操作者检查开、关一次；在一台出网受限（经代理、7844 端口被拦）的电脑上，`cloudflared` 在就绪之前退出时，能否给出比「它在就绪之前退出」更具体的一句拒绝。今天的拒绝给出手动跑的那一行命令，人从 `cloudflared` 自己的输出里读原因。
- **没有控制台时的恢复语说的命令给不出控制台。** `bin::outside::asking` 对开门、换钥匙与确认的拒绝（`E_TOOL_UNAVAILABLE`）写「在城自己的终端里跑 `sprawling serve`」，而 `serve` 只在带 `--console`、或由 `sprawling up` 启动时才有控制台（`crates/sprawling/src/main/city.rs` 的 `wanted`）；照这句恢复语做的人仍然没有控制台。改法是让恢复语写出 `sprawling up`，或 `sprawling serve --console`；它住在装配层的那一个函数里，改它时把 D4 里的同一句一起改。
- **二进制更新之后，macOS 钥匙串是否再问一次读取 `remote` realm 的权限，还没有答案。** 设置页的门（开关、开多久、更换城钥匙，交互契约在 `client/spec/Views/Door.lean`）与线上的四个命令都已落地（§8-13，`crates/sprawling/spec/Outside.lean` §8-140）。核实它的是 `.github/workflows/on-demand.yml` 的 `keychain` 作业：同一棵树的 debug 与 release 两个构建先后在同一路径上 serve 同一座城，比较 `sprawling/remote` 服务下那一项的 `mdat`。门只由带控制台的 serve 持有（`crates/sprawling/spec/Outside.lean` 的「没有控制台」），城钥匙也只在持有门时读写，所以作业的两次 serve 都带 `--console`、标准输入为空：门与钥匙在启动时持有，控制台读到输入结束就停，城照常服务。vault 选了哪个存储，作业从城的 Ledger 读：启动探测被钥匙串拒绝时写一行 `provider_degraded`，其 `fallback` 是 `session-memory`；没有这一行，存储就是登录钥匙串。终端上的 `sprawling doctor` 不印存储（存储只在线上的 `DoctorAnswer.custody` 里），所以作业不再拿它作证据。判词点名存储，并给出四种结果之一：没写（第一次 serve 之后没有那一项）、对话框（第二次 serve 没能按序关闭，因为门在 Ctrl-C 处理器装好之前持有，等一个没人答的对话框就停在启动里）、更新后没留住（`mdat` 变了或那一项没了，城造了新钥匙，每台设备要重新配对）、更新后留住（那一项没动，静默读到，或以平台服务的原话拒绝，访问列表说是哪一种）。能定下这一条的证据是在 wave 分支上派发 `keychain` 作业得到的判词：存储是登录钥匙串且结果是「留住」或「对话框」，这一条就有了答案，写进 §8-3 的城钥匙一段与 `crates/gateway/spec/Credential.lean` §8-4 的 macOS 一行，并从这里删掉；存储是会话内存时，托管 runner 上测不到，要换一台登录着的 Mac 跑同一个作业。macOS 之外不需要这一条：Windows 的凭据管理器与 Linux 的内核 keyring 不按二进制的签名给读取权限。
-/

/-! ## 4 现状分析

- `wire::auth::PairingToken` 与 `wire::reception::decide_bind` 已经守住局域网这一面：非回环绑定必须带令牌，令牌只存摘要、常数时间比较。远程门不改它，也不把它当作通路（D8）：局域网地址是 `http://`，浏览器不把它当安全上下文，页面在那里做不了握手，也装不成 PWA。
- 城的线协议帧由 `wire::frames` 定义，远程门不改帧，只在帧外加一层封装，所以本 crate 不让 `WIRE_V` 进位。远程门的 Ledger 事件随装配落地时，事件种类名进 schema 哈希，哈希随之变（wire D1）。
- 门、配对码、密钥、握手与封装零 I/O：时间与熵都是参数，与 `wire::auth` 的做法相同。通路是例外，两个生产实现起子进程、在回环上问就绪（§7、D14）。

各单元的落地情形：门、配对码、密钥、两种握手、封装与三个通路实现都已落地，装配层 `bin::outside` 与按城的 `[remote]` 造通路的 `bin::assembly::remote_door`（`crates/sprawling/Spec.lean` §8-139、§8-140、§8-151）是唯一调用方；§3 列出的是还没有落地的部分。

已知的出入：`spec/Handshake.lean` 规定的是 `handshake::pairing`，按 ARCHITECTURE.md §11 的路径规则它的名字应是 `spec/Handshake/Pairing.lean`。它沿用迁移之前的名字，因为两个分部的名字已被 Rust 的模块文档与 `architecture.toml` 的锚点引用；它的内容下一次改动时一并改名，改名不改任何性质。
-/

/-! ## 5 权威信源

- 门的性质：`crates/remote_access/spec/Door.lean`（本 crate 是它的实现，它是性质的权威）。
- base32：RFC 4648 §6，测试向量取自 §10（`"foobar"` → `MZXW6YTBOI======`）。
- ML-KEM：FIPS 203；ML-DSA：FIPS 204；X25519：RFC 7748；Ed25519：RFC 8032；HKDF：RFC 5869；AES-GCM：NIST SP 800-38D。
- 后量子混合密钥交换 `X25519MLKEM768` 已是 TLS 的默认候选：`cloudflared` 到 Cloudflare 边缘的连接在日志里报出这一曲线偏好（2026.9.3），Cloudflare 的 1.1.1.1 以 ML-DSA-44 验证 DNSSEC。
- Cloudflare Tunnel：命名隧道需要账号与托管在 Cloudflare 上的域名，主机名固定，可在边缘加 Access、WAF 与 Bot 规则；Quick Tunnel 每次启动换一个 `trycloudflare.com` 子域，不保证 SLA，不支持 SSE，并发 200 个请求（developers.cloudflare.com 的 Quick Tunnels 页）。
- 配对握手的性质：`crates/remote_access/spec/Handshake.lean`（本 crate 的配对握手是它的实现，它是性质的权威）。
- `cloudflared`：`tunnel run` 收 `--url`（隧道的源）、`--metrics`（指标服务的地址）与 `--no-autoupdate`；指标服务的 `/ready` 在至少有一条到边缘的连接时答 200，否则答 503（cloudflare/cloudflared 仓库 `metrics/readiness.go`）。
- 安全上下文：WebCrypto 的 `SubtleCrypto`、service worker 与 `StorageManager.persist()` 只在安全上下文里可用，`http://localhost` 之外的 `http://` 地址不算（MDN 的 SubtleCrypto 与 Service Worker API 页）。URL 片段在请求发出之前就被分离出去，不送给服务器（RFC 3986 §3.5）。
-/

/-! ## 6 命名统一

门（remote door）、纪元（epoch）、配对码（pairing code）、邀请（invitation：二维码里城的指纹与配对码那一对）、城的指纹（city fingerprint）、配对握手（pairing handshake）、认领（claim）、设备（device）、远程会话（remote session）、权限（authority：`Watch`／`Act`）、动词类（verb class：`Read`／`Act`／`LocalOnly`）、通路（route）、主机名的持久性（permanence：`Fixed`／`PerStart`）。五个词与城里别处的词同名或相近而不同义，本 crate 的文档这样区分：

- 「门」写全为「远程门」。glossary 的 door 是 Gate 的判定点，列在 `kernel::gate::DOORS`，判的是城里的一个动作；远程门判的是谁能从外面进城。
- 「会话」在本 crate 指远程会话：一台设备一次握手之后持有的那一段，`SessionId` 与 `handshake::Session` 都指它。glossary 的 Session 是房间的一段，与它无关。
- 「纪元」指远程门的纪元。wire 的 `Welcome.epoch` 是 Ledger 首行的链哈希，是另一件事。
- 通路打开的答案叫 `Opened`，不叫 reach：`kernel::Reach` 是一次到 provider 的分段可达性读数，`crates/wire/Spec.lean` §19 的 reach 说一个动词从哪里够得到（D12）。
- 「配对令牌」（pairing token）是 wire 局域网那一面的词，与本 crate 的「配对码」不是一物：令牌在一次服务期间反复出示，配对码只兑一次。
-/

/-! ## 7 模块边界

**三件邻居的活，及它们各自的主人**：

- 帧的类型、编码与握手版本归 `wire::frames`；本 crate 只在帧外封一层，不认识任何一个帧。
- 一帧属于哪个动词类（`Read`／`Act`／`LocalOnly`）的对照表住在 `crates/wire/Spec.lean` §19-2，是 reach 旁边的 `class` 一列，`xtask wiring` 读那一张表并对照代码；逐帧查表、再问 `door::permits` 的中继在装配层，因为只有 `sprawling` 同时依赖 wire 与本 crate。本 crate 只给出 `door::permits(Authority, VerbClass)` 这条判定。
- 随机字节、时钟、远程监听与它的路径、设备表的落盘与城钥匙种子的存取（何时生成、写进 vault、取回、更换）归装配层，种子的解封与派生归本 crate 的 `keys`（D23）（`bin::assembly` 取时钟与熵，`bin::outside` 持有门、远程监听与设备表，`crates/sprawling/Spec.lean` §8-139）；本 crate 只收参数、只给判定。通路是唯一的例外：通路缝（§8-7）与它的三个实现都在本 crate，两个生产实现在这里起子进程（`cloudflared`、人写的命令）、读它们的输出、在回环上问就绪（D14）。它们只用标准库的 `std::process` 与 `std::net`，本 crate 仍只依赖 kernel、aws-lc-rs 与清零用的 zeroize。装配层选哪一条通路、把配置读成类型化的参数交给它。

依赖：`remote_access: kernel`（ARCHITECTURE §3 的 depmap）。`sprawling` 是唯一消费者。

D8 局域网那一面不是通路。wire 的局域网面（绑定非回环地址，凭配对令牌）照旧，远程门不经过它。局域网地址是 `http://`，浏览器不把它当安全上下文，WebCrypto、service worker 与 `storage.persist()` 在那里都不可用，页面做不了握手，也装不成 PWA；那一面又是 `serve` 启动时一次定下的绑定，不能随门开关。落选的两种：给局域网配自签名的 HTTPS，浏览器会警告，iOS 上 service worker 装不上，还要多一份生成证书的依赖；把它照旧放进缝里，它既不经过门也不经过封装，名字却让人以为它受门保护。在局域网里要用远程门，跑一条带有效证书的 `command` 通路。

D10 页面由通路送达，这是通路上剩下的一件信任。页面脚本经通路到达设备，改写页面的通路能读到页面读到的一切：片段里的配对码、设备的种子、解开的帧。浏览器目前没有办法让一个页面钉住自己下一次加载的字节：service worker 的脚本变了，浏览器就装上新版本，旧版本拦不住。落选的做法：只在这台电脑旁边配对（手机要先到电脑边上，而局域网不是安全上下文，D8）；另做一个原生外壳（多一种发行件和一条签名链）。浏览器提供钉住顶层页面字节的办法时，重新考虑这一条。

D14 通路缝的三个实现与缝同在本 crate。一条缝要带着它的第二个实现落地才是真的（ARCHITECTURE §4），而装配层要到接上中继时才链接本 crate；通路的策略——哪种地址算安全上下文、通路何时算就绪、人写的命令要印什么——又都属于声明它的接口。两个生产实现只用标准库起子进程、读一个管道、在回环上问一次 HTTP，所以本 crate 的依赖仍是 kernel、aws-lc-rs 与 zeroize，装配层只选一条通路、把配置读成类型化的参数交给它。落选的做法是把 `cloudflare` 与 `command` 放进装配层、脚本化通路放进 `crates/sprawling/tests/`：缝在链接之前就无从证明，装配层还要多管两个与组装无关的子进程。
-/

/-! ## 8 接口先行

签名的权威是 Rust 源码；这里记每个模块的接口、它为什么是这个形状，以及分部没有表示的要求。

### 8-1 remote_access::door（形状 1 判定＋形状 5 状态）

```rust
pub struct Epoch([u8; 16]);        // from_entropy
pub struct DeviceId([u8; 16]);     // from_entropy；as_bytes
pub struct SessionId([u8; 16]);    // from_entropy
pub struct DeviceKey(Vec<u8>);     // from_bytes／as_bytes；设备公钥的原始字节，由 keys 解释
pub struct DeviceName(String);     // parse：去首尾空白后 1..=MAX_CHARS（64）个字符；as_str
pub enum Authority { Watch, Act }
pub enum VerbClass { Read, Act, LocalOnly }
pub fn permits(authority: Authority, class: VerbClass) -> bool;
pub struct Device { pub id, pub name, pub authority, pub key }
pub struct Door { /* epoch、phase、pending、devices、sessions —— 私有 */ }
impl Door {
    pub fn start(devices: Vec<Device>, epoch: Epoch) -> Door;
    pub fn open(&mut self, epoch: Epoch, closes_at: TimeMs) -> Result<(), AxError>;
    pub fn close(&mut self);
    pub fn tick(&mut self, now: TimeMs);
    pub fn expect_pairing(&mut self, code: PairingCode, name: DeviceName, authority: Authority, expires: TimeMs) -> Result<(), AxError>;
    pub fn pair(&mut self, code: PairingCode, id: DeviceId, key: DeviceKey, now: TimeMs) -> Result<(), AxError>;
    pub fn revoke(&mut self, id: DeviceId) -> Result<(), AxError>;
    pub fn admit(&mut self, session: SessionId, device: DeviceId, expires: TimeMs) -> Result<(), AxError>;
    pub fn authority(&self, session: SessionId, now: TimeMs) -> Option<Authority>;
    pub fn device(&self, id: DeviceId) -> Option<&Device>;
    pub fn devices(&self) -> &[Device];
    pub fn closes_at(&self) -> Option<TimeMs>;
}
```

- **门在城启动时关着**，开门换一个新纪元，旧纪元里铸的码与开的会话一律作废；`open` 拒绝在当前纪元里重开，这是旧东西回来的唯一一条路。
- **关门只留下设备**：配对与会话全部清空。设备留下，是因为「出门前锁门」不应让人回家后重新配对每一台设备；要让设备也失效，用 `revoke`。
- **配对码兑一次**：`pair` 先把码从等待表里取走，再判它是否仍在本纪元、是否未过期，所以一个码无论兑成与否都只答一次。
- **会话的寿命取 `min(请求的到期, 门的关闭时刻)`**：会话永远不比门活得久。`authority` 在门关着、纪元不符、会话过期或设备已撤销时答 `None`。
- 失败码：门关着、码不对、设备未配对 → `E_GATE_DENIED`；重用纪元、撤销未知设备、设备名不合 → `E_INVALID_ARGS`。每一条都带一句可执行的恢复语，指向控制台上的哪条命令。

D4 门的动词按它们给出什么分三种走法。**威胁**：城的回环端口上，任何本地客户端都能连 `/ws`、发任何一帧，其中包括居民的两种浏览器工具——`browser` 开的 headless 浏览器能打开回环上的页面（`crates/browser/Spec.lean` 的 `fetch` 一段），`usersbrowser` 经 User 批准后驱动 User 自己的浏览器与 profile（browser D10）——以及居民的 `exec` 跑的命令；所以页面上一个开关能做的，这些工具都能做，执行者从一帧本身分不出发它的是 User 还是工具。**走法**：①**关门**是线上的 `CloseRemoteDoor`，任何本地客户端发来都照做，不要证明，因为关门只减少访问（D5）；远程设备锁门仍走封装的锁门字节（D13）。②**开门与更换城钥匙**是线上的 `OpenRemoteDoor` 与 `ReplaceCityKey`，一帧请求本身什么也不做：城取一个确认码，印在城的控制台上，旁边写明它要做的那个动词，回给页面的只是「有一个码在等」，不带码；只有紧接着的一帧 `ConfirmRemoteDoor` 带回这个码、且在它到期之前，城才做那个动词（`spec/Confirm.lean` 的三条性质）。控制台是 User 自己的终端，它的输出不进任何一帧、不进任何页面，所以一个只能驱动页面或只能说线协议的客户端拿不到码。三个平台上码都印在 `sprawling serve` 的标准输出上（Windows 的控制台窗口，macOS 与 Linux 的终端）；城没有控制台时没有人读得到码，开门、换钥匙与确认以 `E_TOOL_UNAVAILABLE` 拒，恢复语是在城自己的终端里跑 `sprawling serve`（今天 `serve` 只在带 `--console` 或经 `sprawling up` 启动时才有控制台，见本文件 §3）。更换城钥匙在三个平台上做的事相同（D23）：新种子盖过旧的，每台已配对的设备被撤销、要重新配对，已撤销的设备仍是撤销的；新钥匙能留多久照 D23 的平台表。③**配对与撤销**只在控制台。配对不上线，是因为邀请是持有即用的秘密：它一旦显示在页面上，驱动这个页面的工具就能读到它，替自己配上一台设备，那是一份比 run、比楼的规则都活得久的访问；页面给出配对的分步说明与可复制的 `/remote pair` 一行（client 的 `remote.ts`）。撤销只减少访问，可以上线，今天没有页面需要它，所以留在控制台。**逐平台的保证**（以 `crates/runtime/spec/Tools/Exec.lean` 今天的 exec 臂为准）：三个平台上，两种浏览器工具都开不了门、换不了钥匙、配不了设备，能关门。`exec` 在 Linux 上 `PATH` 里有 `bwrap` 时跑在 `LinuxNamespaces` 臂里，它够不够得到回环与控制台由那一臂给的命名空间决定；Linux 上没有 `bwrap`、以及 Windows 与 macOS 上，`exec` 恒在 `CopiedTree` 臂里，命令带着这个用户的权限与网络，够得到 `/ws`，也读得到这个用户的凭据存放处（城钥匙的种子在那里，D23），推断在 Windows 上与城共用一个控制台的进程还能读控制台的屏幕缓冲区，所以对这一臂的 `exec`，门不构成边界，与上一版 D4 写明的缺口相同；那个缺口由 exec 沙箱补，不由这扇门补。落选的两种：只留控制台、页面只给可复制的命令与状态——对浏览器工具的保证与选中的一样，但 Roadmap 的 A7 要设置页上的开关，而确认只多要 User 看一眼控制台；线上的门动词对任何本地客户端都照做——浏览器工具就能开门。**重开参数**：控制台的输出不再只给 User 看（例如页面镜像控制台，或某个居民工具读得到它），确认码就失去保证，这一条要重写；三个平台都有切断回环与凭据存放处的 exec 沙箱时，把逐平台的保证改写为对 `exec` 也成立；要让设置页也能配对时，重新论证邀请怎样不经过一个工具驱动得到的页面。

D5 关门可以由任何人做，开门只能由人做。关门只会减少访问，所以远程设备自己也可以「锁门离开」。

D19 会话的寿命就是门的寿命。`Door::admit` 取门关上的时刻作会话的到期，会话不另设时长；设备锁门、人关门、门到时，会话都随之结束。落选的是给会话一个比门短的时长：设备每过一段就要重新握手，而握手证明的事——它持有配对时的密钥——在门开着的这段时间里不会变。

### 8-2 remote_access::pairing（形状 2 值类型）

```rust
pub const CODE_BYTES: usize = 16;
pub struct PairingCode(B3Hash);
impl PairingCode {
    pub fn mint(entropy: [u8; CODE_BYTES]) -> (PairingCode, String); // 给人看的写法：五个一组，连字符分隔
    pub fn read(typed: &str) -> PairingCode;                          // 忽略大小写、空白与连字符
}
```

- **门只存摘要**：`mint` 把正文交还给调用方去展示一次，本 crate 之后不再持有它，与 `wire::auth::PairingToken` 同一个理由。
- **比较不做常数时间**：等待表里比的是摘要，攻击者选不了摘要的字节，时序只泄露「哪一个摘要前缀相同」，而这推不出任何一个码的正文。
- **base32 小写**：二维码扫描器与剪贴板都原样携带，URL 里无需转义；给人看时五个一组。

### 8-3 remote_access::keys（形状 2 值类型）

```rust
pub const SEED_BYTES: usize = 32;
pub const ED25519_PUBLIC_BYTES: usize = 32;
pub const ML_DSA_44_PUBLIC_BYTES: usize = 1312;          // FIPS 204 表 2
pub const ED25519_SIGNATURE_BYTES: usize = 64;
pub const ML_DSA_44_SIGNATURE_BYTES: usize = 2420;       // FIPS 204 表 2
pub const PUBLIC_BYTES: usize = ED25519_PUBLIC_BYTES + ML_DSA_44_PUBLIC_BYTES;            // Ed25519 在前
pub const SIGNATURE_BYTES: usize = ED25519_SIGNATURE_BYTES + ML_DSA_44_SIGNATURE_BYTES;   // Ed25519 在前
pub struct SigningKey { /* 两半密钥对 —— 私有 */ }
impl SigningKey {
    pub fn from_seed(seed: &[u8; SEED_BYTES]) -> Result<SigningKey, AxError>;
    pub fn from_sealed(seed: &Sealed<String>) -> Result<SigningKey, AxError>;   // D23
    pub fn public(&self) -> VerifyingKey;
    pub fn sign(&self, message: &[u8]) -> Result<Signature, AxError>;
}
pub fn written(seed: &[u8; SEED_BYTES]) -> Zeroizing<String>;                // 种子交给 vault 的正文
pub struct VerifyingKey(Box<[u8; PUBLIC_BYTES]>);   // from_bytes／as_bytes／verify
pub struct Signature(Box<[u8; SIGNATURE_BYTES]>);   // from_bytes／as_bytes
```

- **一个种子派生两半**：HKDF-SHA256，盐 `sprawling remote key v1`，两半各用自己的标签（`ed25519`、`ml-dsa-44`），所以两半不共享任何密钥材料；盐里的版本号保证以后的派生不会产出以前的密钥。人要保存的只是这 32 字节。
- **验证两半都要成立，并且只给一个答案**：调用方从拒绝里得不出是哪一半没过。

- **城钥匙**（D23）：城自己的 `SigningKey` 由 vault 里的一份种子派生。`written` 把装配层新取的种子写成 vault 收的正文：§8-2 的小写 base32，不带填充，52 个字符，交出时已是 `Zeroizing`。`from_sealed` 读回这份正文；它是本 crate 唯一一处 `.expose(`，明文种子只活在这个函数里，派生完即随 `Zeroizing` 清掉。只认 `written` 写得出的那一种正文（解码后再编码要逐字相同），所以多一个字符、少一个字符、末位多出的位不为零、不在字母表里的字符都读不成 → `E_CONFIG_INVALID`，恢复语是在城的控制台上用 `/remote replace-key` 更换城钥匙、再重新配对设备。

D23 城钥匙的种子存进城已有的 vault，兑现点在本 crate 的 `keys`（定规）。引用的 realm 是 `remote`，name 是 `city-key.` 接这座城的创世 id，即 Ledger 创世行的链哈希（wire 的 `Welcome.epoch` 读的同一个值）的小写十六进制，由 `SecretRef::new` 在装配层一处造出，所以两座城不共用一把，`sprawling export` 打的包里没有 vault 条目，也就带不走它。「更换城钥匙」（今天是城的控制台上的 `/remote replace-key`，设置里的按钮随它的 wire 命令到来）生成新种子盖过旧的，再撤销每一台已配对的设备、各写一行 `device_revoked`：设备钉住的是旧钥匙，留在表里的设备既连不上，又占着名字让同名的重新配对被拒，所以每台设备都要重新配对；已撤销的设备不在表里，换钥匙也不把它带回来。门开着时拒绝更换，因为开着的会话是用旧钥匙握的手。每个平台上种子能留多久，就是 vault 在那个平台上能留多久（`crates/gateway/spec/Credential.lean` §8-4 的平台表）：Windows 凭据管理器与 macOS 钥匙串跨重启保留；Linux 的内核 keyutils 只留到这次开机结束，重启电脑后设备要重新配对，除非这座城用加密的 vault 文件（`crates/gateway/Spec.lean` §8-21）；vault 退回进程内时每次城重启都要重新配对，`/remote open` 照实说出这一句。局限：同一个系统用户下运行的程序都能读这个用户的凭据存放处，与 provider key 相同。落选的两种：兑现点放在装配层（`bin::outside::keeper`），明文种子会出现在组装根里，而 `EXPOSE_WHITELIST` 的注释写明它存在就是为了不让明文出现在那里；种子写进保留子树里的一份文件，明文凭据落在 vault 之外，`sprawling export` 也会把它带走，一份拷贝的城就能冒充原来那座。vault 有了跨机同步时重新考虑这一条，因为那时「两座城不共用一把」要由同步来守。

D6 设备密钥在设备上生成。城只存公钥，城的存储泄露不让任何人登录；人要保存的恢复种子是设备自己的，城从未见过它。

### 8-4 remote_access::handshake（形状 1 判定＋形状 5 状态）

```rust
pub const NONCE_BYTES: usize = 32;
pub const HELLO_BYTES: usize = 16 + 32 + 1184 + 32;         // 设备 id、X25519、ML-KEM-768 封装密钥、nonce
pub const REPLY_BYTES: usize = 32 + 1088 + 32 + 2484;        // X25519、ML-KEM 密文、nonce、城的签名
pub struct Hello; pub struct Reply; pub struct Finish;       // from_bytes／as_bytes，定长；Hello::device() 读出设备 id
pub struct Session { pub sealer: Sealer, pub opener: Opener }
pub fn device_hello(device: DeviceId, nonce: [u8; 32]) -> Result<DeviceWaiting, AxError>;
impl DeviceWaiting {
    pub fn hello(&self) -> &Hello;
    pub fn finish(self, reply: &Reply, city: &VerifyingKey, device_key: &SigningKey) -> Result<(Finish, Session), AxError>;
}
pub fn city_reply(hello: &Hello, city: &SigningKey, nonce: [u8; 32]) -> Result<(Reply, CityWaiting), AxError>;
impl CityWaiting {
    pub fn device(&self) -> DeviceId;
    pub fn accept(self, finish: &Finish, device_key: &VerifyingKey) -> Result<Session, AxError>;
}
```

- **三条消息**：设备发 Hello；城回 Reply，并用城的密钥签「协议标签‖`city`‖握手记录」；设备用配对时钉住的城公钥验它，再用自己的密钥签「协议标签‖`device`‖握手记录」作为 Finish；城用配对时存下的设备公钥验它。两个角色标签让一方的签名永远不能冒充另一方的。
- **握手记录**：SHA-256（协议标签‖Hello‖不含签名的 Reply）。用 SHA-256 而不用城的 BLAKE3，因为另一端是浏览器，它的 WebCrypto 有 SHA-256 与 HKDF、没有 BLAKE3。
- **会话密钥**：HKDF-SHA256，盐是握手记录，输入是 X25519 共享秘密‖ML-KEM 共享秘密，展开出「设备到城」「城到设备」两把。中间人要同时持有两个秘密才能算出它们。
- **状态用类型表达**：`DeviceWaiting` 与 `CityWaiting` 只能各用一次（`finish`、`accept` 取走 `self`），握手记录与临时私钥随之消失。
- **临时密钥的随机性由 aws-lc-rs 在内部抽取**：ML-KEM 的封装不接受外部熵，这是唯一一处本 crate 不从参数拿随机性的地方；门的纪元、设备 id、会话 id、配对码与 nonce 仍由调用方给。

### 8-5 remote_access::seal（形状 5 状态）

```rust
pub enum Direction { DeviceToCity, CityToDevice }
pub struct Sealer; pub struct Opener;
impl Sealer { pub fn seal(&mut self, frame: &[u8]) -> Result<Vec<u8>, AxError>; }
impl Opener { pub fn open(&mut self, sealed: &[u8]) -> Result<Vec<u8>, AxError>; }
```

- AES-256-GCM；随机数是 4 字节方向标签加 8 字节大端计数器；计数器不上线，因为套接字按序送达，双方各自知道下一个编号。被重放、被丢掉前一帧、被改过、方向不对的帧都打不开，打不开就结束会话：没有哪一帧值得跳过。
- 计数器用尽（2^64 帧）以 `E_BUDGET_EXHAUSTED` 拒，恢复语是重连：新会话换新密钥、从零计数。
- **会话里封装的负载，第一个字节说它是什么**（D13）：

  ```rust
  pub enum Payload { Frame(String), Lock }   // to_bytes／from_bytes：0 后接一帧线协议文本（UTF-8）；1 是锁门，其后没有字节
  ```

  `Frame` 两个方向都用；中继把设备发来的 `Frame` 原样转给回环上的 `/ws`，不解析后再序列化。`Lock` 只由设备发，装配层收到后调用 `Door::close`，任何权限的设备都可以发（D5）。未知的首字节、`Lock` 后面多出的字节、不是 UTF-8 的帧，`from_bytes` 以 `E_WIRE_MISMATCH` 拒，连接结束。配对连接里的负载不带这个字节，它们是 §8-6 的定长消息。

D13 会话里封装的负载用第一个字节说它是线协议文本帧还是锁门。D5 许诺远程设备可以「锁门离开」，而门的动词不上线协议（D4），所以锁门只能在封装这一层说。落选的做法是给 wire 加一个锁门命令：它要动线协议的 schema 哈希，而且让城的线协议认识一扇它本不认识的门（§7）。

### 8-6 remote_access::handshake 的配对一半（形状 1 判定＋形状 5 状态）

```rust
pub const FINGERPRINT_BYTES: usize = 32;
pub struct CityFingerprint([u8; FINGERPRINT_BYTES]);
impl CityFingerprint {
    pub fn of(city: &VerifyingKey) -> CityFingerprint;         // 公钥字节的 SHA-256；指纹只在这里算
    pub fn text(&self) -> String;                               // 与配对码同一种 base32（§8-2），52 个字符
    pub fn read(text: &str) -> Result<CityFingerprint, AxError>;
}
pub struct Invitation { pub city: CityFingerprint, pub code: String }   // 二维码带给设备的那一对
pub const CODE_TEXT_BYTES: usize = 26;                          // 配对码正文：小写、不分组
pub const PAIR_HELLO_BYTES: usize = 32 + 1184 + 32;             // X25519、ML-KEM-768 封装密钥、nonce；没有设备 id
pub const PAIR_REPLY_BYTES: usize = PUBLIC_BYTES + 32 + 1088 + 32 + SIGNATURE_BYTES; // 城的公钥、X25519、ML-KEM 密文、nonce、城的签名
pub const CLAIM_BYTES: usize = CODE_TEXT_BYTES + PUBLIC_BYTES + SIGNATURE_BYTES;     // 封装之前的认领：配对码正文、设备公钥、设备的签名
pub struct PairHello; pub struct PairReply;                     // from_bytes／as_bytes，定长
pub struct Claim;                                               // code() -> PairingCode；device_key() -> &VerifyingKey
pub struct Claimed { pub sealed: Vec<u8>, pub city: VerifyingKey, pub session: Session }
pub fn device_pair_hello(nonce: [u8; NONCE_BYTES]) -> Result<DevicePairing, AxError>;
impl DevicePairing {
    pub fn hello(&self) -> &PairHello;
    pub fn claim(self, reply: &PairReply, invitation: &Invitation, device_key: &SigningKey) -> Result<Claimed, AxError>;
}
pub fn city_pair_reply(hello: &PairHello, city: &SigningKey, nonce: [u8; NONCE_BYTES]) -> Result<(PairReply, CityPairing), AxError>;
impl CityPairing {
    pub fn open_claim(self, sealed: &[u8]) -> Result<(Claim, Session), AxError>;
}
```

- **为什么要这一半**：设备第一次连上城时，城还没有它的公钥，§8-4 的握手无从认证设备。如果这时把配对码与设备公钥明文发出，解开 TLS 的边缘在码的十分钟里就能抢先配上自己的密钥（D9）。所以配对先走一次只由城认证的握手，认领在它派生的会话密钥下封好再发。
- **邀请不经通路**：`/remote pair` 在控制台印出二维码，内容是 `https://<主机名>/#pair=<配对码正文>&city=<指纹正文>`。配对码与指纹放在 URL 片段里，浏览器不把片段随请求发出（RFC 3986 §3.5），所以只转发请求的通路见不到它们；页面脚本从 `location.hash` 读出它们。二维码里放指纹而不放整把公钥：公钥 1344 字节，编进 URL 后二维码密得难扫；指纹 32 字节，城在回答里出示整把公钥，设备核对它的 SHA-256。
- **三条消息，一条回执**：设备发 `PairHello`。城回 `PairReply`，出示自己的公钥，并用城的密钥签「配对标签‖`city`‖握手记录」。设备先核对 `CityFingerprint::of(出示的公钥)` 等于邀请里的指纹，再验签名，两样都过才派生会话密钥，把「配对码正文‖设备公钥‖设备对『配对标签‖`device`‖握手记录』的签名」封成第一帧发出。城用 `open_claim` 解开它、验设备的签名，交回 `Claim`；装配层随后 `PairingCode::read`、`Door::pair`，门给出的 `DeviceId`（16 字节）由城封成第一帧回给设备，连接随即结束。设备留下城的公钥、自己的种子与 `DeviceId`，此后每次连接走 §8-4。
- **握手记录与会话密钥**：配对标签是 `sprawling remote pairing v1`，与 §8-4 的协议标签不同，所以一次配对的签名不能冒充一次会话握手的签名，反过来也一样。记录是 SHA-256（配对标签‖`PairHello`‖不含签名的 `PairReply`）；会话密钥的派生与 §8-4 相同，盐换成这份记录。
- **设备也签一次**：认领里的签名证明设备持有它交出的那把公钥。没有它，一把设备其实不持有的公钥（例如页面生成密钥时出了错）要到码已经用掉、第一次 §8-4 握手时才暴露，那时只能重新配对。
- **失败**：消息长度不对 → `E_WIRE_MISMATCH`；邀请里的配对码规范化后不是 26 个 base32 符号 → `E_INVALID_ARGS`，恢复语是重新扫码；指纹不符、城的签名不对 → `E_GATE_DENIED`，恢复语是「在城的控制台上重新扫码；反复出现说明通路在改动它转发的内容」；认领打不开、设备签名不对 → `E_GATE_DENIED`，连接结束，码仍在门里等到到期；码本身不对由 `Door::pair` 回答（§8-1）。
- **指纹的读法**：`CityFingerprint::read` 去掉首尾空白、不论大小写；正文须是 52 个 base32 符号，并且是某 32 字节的规范写法（最后一个符号的低 4 位为 0），否则 `E_INVALID_ARGS`，恢复语是重新扫码。一个指纹只有一种写法，所以两个指纹相等当且仅当它们的正文相等。
- **一条配对连接只做配对**：它不调 `Door::admit`，不转发任何帧，回执之后就结束。`DevicePairing` 与 `CityPairing` 各只能用一次（`claim`、`open_claim` 取走 `self`）。
- **临时密钥的随机性**：与 §8-4 相同，由 aws-lc-rs 在内部抽取；nonce 由调用方给。

D9 配对走一次只由城认证的握手，配对码只在封装里走。边缘解开 TLS，明文的配对消息会把配对码交给边缘，边缘在码的十分钟有效期里可以抢先配上自己的密钥。落选的做法是在这十分钟里信任边缘并把这一点写明：它省下 §8-6 的一组消息，代价是 §1「通路冒充不了任何一方」在配对这一步不成立。指纹与配对码经二维码从控制台屏幕直接到设备，放在 URL 片段里，只转发请求的通路见不到它们。

### 8-7 remote_access::route（缝：通路）

```rust
pub trait Route {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError>;
    fn close(&mut self) -> Result<(), AxError>;
}
pub struct Opened { pub url: PublicUrl, pub permanence: Permanence }
pub struct PublicUrl(String);          // parse(&str) -> Result<PublicUrl, AxError>；as_str
pub enum Permanence { Fixed, PerStart }
```

- **开**：`local` 是装配层的远程监听在回环上的地址。`open` 在外面够得到它之后才返回，答出外面用的地址，以及这个地址的主机名在通路重启之后是否不变。一条已开的通路再 `open`，先关掉原来那一条，再开新的。
- **关**：之后外面再够不到 `local`。`close` 可以重复调用，关一个没开的通路答成功，所以门到时与人输入 `/remote close` 同时发生也无妨。
- **两个方法是同步的**（D15）：`open` 阻塞到通路就绪，最长到实现构造时收下的耐心（`kernel::TimeoutMs`）用完；装配层在自己起的阻塞线程上调用它们。
- **`PublicUrl` 只收 `https://`**：页面要做握手、要装成 PWA，都需要浏览器的安全上下文，`http://` 的地址上两样都做不成（D8）。类型挡住它，调用方就不必再问一个通路「能不能用」。`parse` 不论 scheme 的大小写，存成小写；拒 `http://` 与其他 scheme、没有主机的地址、主机前带用户信息（`user@`）的地址、带片段（`#`）的地址、含空白或控制字符的地址，报 `E_CONFIG_INVALID`，恢复语指向产生这个地址的通路配置。片段被拒，是因为邀请要在这个地址后面接上 `#pair=…&city=…`（§8-6）；用户信息被拒，是因为这个地址要印进二维码。
- **`Permanence`**：设备上的密钥与 PWA 按域名隔离，主机名一换，已配对的设备就读不到自己的密钥。在 `PerStart` 的通路上，装配层在配对之前向人说明这一点。Cloudflare 命名隧道答 `Fixed`；`command` 通路由人在配置里写明。
- **实现**（D14）：`route::cloudflare::NamedTunnel`（默认，D3、§8-8）、`route::command::CommandRoute`（§8-9）与 `route::scripted::ScriptedRoute`。脚本化通路答出构造时给它的 `Opened`，并按次序记下每一次 `open(local)` 与 `close()`（`ScriptedRoute::new(Opened)`、`calls() -> &[RouteCall]`、`enum RouteCall { Open(SocketAddr), Close }`），装配层的测试从中看出自己按什么次序开关通路。缝登记在 ARCHITECTURE §4。局域网那一面不是通路（D8）。
- **子进程的寿命跟着通路**：两个生产实现各起一个子进程，`close` 结束它并等它退出；一条通路没有关就被丢弃时，`Drop` 同样结束它，所以城退出时不留下一条还开着的隧道。
- **没有能力查询**：一个通路要让调用方知道的两件事，一件由 `PublicUrl` 的类型挡住，一件放在 `Opened.permanence` 里（D12）。

D12 通路的接口只有开与关；早先设想的 `Reach` 与 `Capabilities` 两个类型不存在。开通路答 `Opened`：外面用的地址，以及这个地址的主机名是否跨重启不变。`Reach` 已是 kernel 公开的分段可达性读数（`kernel::Reach`），reach 又是 `crates/wire/Spec.lean` §19 的一列，第三个同名物会让读者把打开通路读成一次探测；`Capabilities` 在 ACP 里是握手字段名，glossary 里的 capability bits 是楼的能力位。能力查询整个消失，还因为调用方据以行动的事实只有两件：地址是否是安全上下文，由 `PublicUrl` 的类型挡住；主机名会不会变，通路打开之后才可靠地知道，于是放进 `open` 的答案。落选的做法是保留一个改了名的 `capabilities()`：它多一个查询面，还允许通路自报的能力与它打开之后的实际不一致。

D15 `Route` 的两个方法是同步的。三个实现里，两个要等一个子进程（印出地址，或者连上边缘），一个什么也不等；等子进程要么阻塞一个线程，要么要 tokio 的 `process` 特性，而本 crate 不依赖任何执行器。同步的签名把「这一步会阻塞，最长到耐心用完」写在类型上，调用方在装配层自己的阻塞线程上调用它（装配层是起线程的地方，ARCHITECTURE §10 第 3 条）。落选的做法是保留 `async` 签名：实现要么在 future 里阻塞执行器的一个工作线程，要么让本 crate 带上 tokio 与它的 `process` 特性。一个通路能不占线程地等待（例如只和本地守护进程说 HTTP）、并且装配层同时持有很多条通路时，重新考虑这一条。

### 8-8 remote_access::route::cloudflare（适配器：Cloudflare 命名隧道）

```rust
pub struct Tunnel { pub program: PathBuf, pub name: TunnelName, pub url: PublicUrl }
pub struct TunnelName(String);         // parse(&str) -> Result<TunnelName, AxError>；as_str
pub struct NamedTunnel { /* 私有 */ }  // new(tunnel: Tunnel, patience: TimeoutMs)；impl Route
```

- **跑的是本地管理的命名隧道**（D16）。人事先建好它一次：`cloudflared tunnel login`、`cloudflared tunnel create <名字>`、`cloudflared tunnel route dns <名字> <主机名>`。隧道的凭据留在 `cloudflared` 自己的目录里，从不进城；`url` 是 `https://<主机名>`，由人写在配置里。
- **开**：跑 `<program> tunnel --no-autoupdate --metrics 127.0.0.1:<空闲端口> --url http://<local> run <名字>`。`--url` 把隧道的源指到这一次的远程监听，所以监听的端口不必写进配置；`--no-autoupdate` 让 `cloudflared` 不在通路开着的时候替换自己再重启。空闲端口的取法：在回环上绑一个 0 端口，读出系统给的端口，再放开。子进程的 stdin、stdout、stderr 都接空：它的日志只写给它自己，读它就要一个线程替它排空管道。
- **就绪**：每 200 ms 问一次 `http://127.0.0.1:<端口>/ready`，答 200 表示 `cloudflared` 至少有一条到边缘的连接；这时答 `Opened { url, permanence: Fixed }`。问的次数是耐心除以 200 ms，所以等待按次数计，不读时钟。
- **`TunnelName`**：1 到 64 个字符，只含 ASCII 字母、数字、`.`、`_`、`-`，不以 `-` 开头。名字作为一个参数交给 `cloudflared`，以 `-` 开头就会被读成一个选项。
- **失败**：程序起不来 → `E_TOOL_UNAVAILABLE`，恢复语是装上 `cloudflared` 或给出它的完整路径；在就绪之前退出 → `E_TOOL_UNAVAILABLE`，主题带退出状态，恢复语给出手动跑的那一行命令与建隧道的三条命令；到时仍未就绪 → 结束子进程，`E_TIMEOUT`；回环上拿不到空闲端口 → `E_TOOL_UNAVAILABLE`；名字不合 → `E_CONFIG_INVALID`。
- **操作者检查**（不作门）：在装了 `cloudflared`、建好隧道、能连上边缘的电脑上，经 `NamedTunnel::open` 开一次（装配层接上之后就是 `/remote open`），在另一台设备上打开答出的地址，看到远程监听的回答；再 `close`，确认 `cloudflared` 进程已经退出，地址不再有回答。

D16 `cloudflare` 通路跑本地管理的命名隧道，凭据留在 `cloudflared` 自己手里。隧道由人用 `cloudflared` 的三条命令建好一次，`cloudflared` 凭它自己目录里的凭据连上边缘，城只把 `--url http://<local>` 交给它。好处有两件：隧道的源随每次打开给出，远程监听的端口不必固定，也不必与 Cloudflare 面板上的配置保持一致；城里从不出现隧道凭据的明文。落选的做法是面板管理的隧道：令牌存进 vault、经 `TUNNEL_TOKEN` 交给子进程。它的源写在面板上，监听端口要写死在配置里并手工与面板对齐；令牌的明文要在本 crate 里从 `Sealed` 取出，那是 `xtask secret` 名单之外的又一个兑现点，加进名单是放宽一道门。人要用面板管理的隧道时，重新考虑这一条，那时它是 `cloudflare` 里一个带兑现点与固定端口的变体。

### 8-9 remote_access::route::command（适配器：人写的一条命令）

```rust
pub const LOCAL_ENV: &str = "SPRAWLING_REMOTE_LOCAL";
pub struct RouteCommand { pub program: PathBuf, pub args: Vec<String> }
pub struct CommandRoute { /* 私有 */ }  // new(command: RouteCommand, permanence: Permanence, patience: TimeoutMs)；impl Route
```

- **开**：起这条命令，环境变量 `SPRAWLING_REMOTE_LOCAL` 的值是 `local`（写法如 `127.0.0.1:4000`），stdin 与 stderr 接空。命令让外面够得到这个地址之后，往 stdout 印一行 `{"url": "https://…"}`，然后一直跑着，直到通路关上时被结束。本 crate 读 stdout 上第一行这样的 JSON 对象，它之前的行当作噪声跳过；地址按 `PublicUrl::parse` 判。答 `Opened { url, permanence }`，`permanence` 由人写在配置里，因为命令无从证明自己的主机名跨重启不变。
- **那一行的写法**：去掉首尾空白后，整行是只有 `url` 一个成员的 JSON 对象，值是不含反斜杠的字符串；`{` 之后、冒号两侧、`}` 之前允许空白。它是 JSON 的一个子集，所以本 crate 不为一行输出引入 JSON 解析器；地址里本不需要转义，写了转义的行被拒，恢复语给出写法。以 `{` 开头却不合这一写法的行被拒而不是跳过：那一行想给出地址却写错了，跳过它只会一直等到超时。
- **读 stdout 的是一个线程**：管道上的阻塞读没有期限，线程把它换成一个能带期限等待的通道，与 `agent_protocols::mcp::stdio` 同一个理由。读到地址之后它继续读、丢弃，直到命令结束，免得管道写满后命令卡在写上；命令结束时管道关上，线程随之返回。
- **失败**：起不来 → `E_TOOL_UNAVAILABLE`；在印出地址之前结束 → `E_TOOL_UNAVAILABLE`，恢复语让人手动跑这条命令看它为何停下；地址那一行写错、地址不合 `PublicUrl` → 结束命令，`E_CONFIG_INVALID`；到时仍未印出 → 结束命令，`E_TIMEOUT`。
- **Tailscale 的写法**：用 Tailscale 的人让这条命令包一层 `tailscale serve`，tailnet 里的设备就能打开页面。一个 sh 示例：

  ```sh
  host=$(tailscale status --json | jq -r '.Self.DNSName | rtrimstr(".")')
  echo "{\"url\": \"https://$host\"}"
  exec tailscale serve --https=443 "http://$SPRAWLING_REMOTE_LOCAL"
  ```

  城那一层的 `[remote]` 写 `route = "command"`、`command = "sh"`、`args = ["<这份脚本的路径>"]`、`permanence = "fixed"`（`crates/city/Spec.lean` §8-39）：MagicDNS 名字跟着这台电脑，不随重启变。`exec` 让通路关上时被结束的正是前台的 `tailscale serve`，它的配置随进程一起撤下。这一行地址印在 serve 生效之前片刻，设备扫码总在其后。

D22 本 crate 不读配置。通路的三个参数（选哪一个实现、隧道名与地址或命令、`permanence`）写在城那一层的 `[remote]` 表里，由 `city::config_layers` 读成字符串（`crates/city/Spec.lean` §8-39），由装配层在每次 `/remote open` 时经 `TunnelName::parse`、`PublicUrl::parse` 造成这里的类型（`crates/sprawling/Spec.lean` §8-151）。配置的层次、缺键与错键的拒法、哪一层能写哪张表，在 city 有唯一的家；本 crate 若给 `Tunnel`、`RouteCommand` 加上 serde 的读法，就是第二个配置读者，梯子与拒法要再写一遍。落选的另一条路是 city 依赖本 crate、在解析点直接造出类型化的值：它让写错的地址在读文件时就被拒，代价是 city 的拓扑多一条边（city 今天只依赖 kernel）。city 被允许依赖本 crate 时，重新考虑这一条。

### 8-10 远程监听：两条路径、页面与它们上面的消息（装配层 `bin::outside::listener`；`crates/sprawling/Spec.lean` §8-139）

```text
/remote/pair     设备 → PairHello；城 → PairReply；设备 → 封好的认领；城 → 封好的 DeviceId（16 字节）；城关闭连接
/remote/session  设备 → Hello；城 → Reply；设备 → Finish；其后两向都是封好的 Payload（§8-5）
/ 与其下的路径    不是 WebSocket 升级的 GET：客户端的 bundle，与城自己的端口同一份（`wire::bundle_routes`，`crates/wire/Spec.lean` §8-2、wire D20）
```

- **页面经同一个端口到达设备**：设备打开二维码里的 `https://<外面的地址>/#pair=…`，远程监听答出客户端的 bundle，页面在这个源上配对、存下设备密钥（`client/Spec.lean` §4-57）。页面这一半不经过门：没配对的浏览器得先拿到页面，才有地方兑配对码，这与城的端口上 `/` 不要令牌是同一个理由（`crates/wire/spec/Reception/Admission.lean`）。页面之外的 HTTP 门（`/transcribe`、`/drop`、`/enroll`、`/acp`）不在远程监听上：它们不是线协议帧，中继判不了它们的动词类，所以在远程源上答 404。
- **配对之后整个客户端经 `/remote/session` 说线协议**：页面在 `https:` 源上、这个源的 IndexedDB 存着一台配对过的设备时，链路的每一次连接都是一次会话握手，此后每一帧线协议文本封成 `Payload::Frame` 发出，城发回的每一帧打开后交给链路；没有配对过的源照旧连 `/ws`（`client/Spec.lean` §4-64）。中继把设备的 `Hello` 帧连同城的配对令牌一起交给城的 `/ws`，所以设备不持有令牌。端到端的证据是 `crates/sprawling/tests/remote_door.rs`：从远程地址取到的 `/` 与城的端口上的 `/` 逐字节相同，设备经 `/remote/session` 收到城的 `Welcome`。

- **监听在回环上，随门开关**：`/remote open` 在 `127.0.0.1` 上绑一个系统给的端口，通路把外面引到它（§8-7）；门关上（控制台、设备锁门或到时）监听随之停止。城自己的端口不经通路，仍由它的配对令牌守着。
- **每条消息都是二进制 WebSocket 消息**，正文就是 §8-4、§8-6、§8-5 的定长消息或封装；文本消息被拒（`E_WIRE_MISMATCH`），连接结束。
- **连接结束时，关闭帧的原因只写一个稳定的错误码**（如 `E_GATE_DENIED`）：通路上的陌生人也读得到它，所以不写主题与恢复语；页面按码给人一句话。
- **会话里每一帧都逐帧授权**：中继打开封装，按首字节分开线协议帧与锁门；线协议帧先按 `crates/wire/Spec.lean` §19-2 的 `class` 列判类，再问 `Door::authority(会话, 此刻)` 与 `permits`。放行的帧原样发给城的 `/ws`；拒绝的帧不到城，设备收到一帧封好的 `Refusal`；会话已不被门持有（门关了、设备被撤销、到时）时连接结束，而不是一帧一帧地拒（`crates/wire/Spec.lean` §8-66）。
- **门的一次判定、它写的那一行与设备表的落盘在同一把锁下**，所以账本上的次序就是门里发生的次序（`crates/sprawling/Spec.lean` §8-139）。

D17 远程监听是自己的一个回环端口，不是城的端口上的一条路由。通路把这个端口引到外面，城自己的端口留在 `serve` 绑定它的地方、由它自己的令牌守着；两件事一个开一个不开，所以是两个端口。中继在这里以一个线协议客户端的身份连城的 `/ws`，与浏览器同一扇门进城。落选的做法是在城的端口上加 `/remote` 路由：那要让通路指向城的端口，局域网那一面的配对令牌与远程门的设备密钥守同一个端口，门关着时外面仍够得到城的端口。

### 8-11 remote_access::door 的正文写法（形状 2 值类型）

```rust
impl DeviceId  { pub fn text(&self) -> String; pub fn read(text: &str) -> Result<DeviceId, AxError>; }   // 26 个 base32 符号
impl DeviceKey { pub fn text(&self) -> String; pub fn read(text: &str) -> Result<DeviceKey, AxError>; }
impl Authority { pub fn word(self) -> &'static str; pub fn from_word(word: &str) -> Result<Authority, AxError>; }  // "watch" | "act"
```

- **设备表与账本读同一种写法**：设备 id 与公钥用配对码与城的指纹已经用的那一种 base32（§8-2），一个人可能看到的每一串字节都是同一个字母表。`DeviceId::read` 只收 16 字节的规范写法。
- **失败**：读不成 → `E_INVALID_ARGS`，主题截到 64 个字符，恢复语是撤销那台设备再重新配对（表被人手改过）。

D18 设备 id 与公钥写成配对码那一种 base32 （§8-11）。一个字母表，人无论在设备表里、在账本里、还是在二维码的片段里看到的字节串都长一个样；落选的十六进制让同一个 id 有两种外形。

### 8-12 与浏览器互通的向量与单向夹具（`tools/fixtures/remote-handshake/`；形状 6 数据面）

```text
keys.txt              seed, ed25519_seed, ml_dsa_44_seed, public
session-keys.txt      transcript, x25519_secret, ml_kem_secret, device_to_city, city_to_device
seal.txt              key, frame, frame_nonce, frame_sealed, lock, lock_nonce, lock_sealed
signature-rust.txt    public, message, signature
fixture.txt           city_public, hello, x25519_private, ml_kem_private, reply, first_frame, plaintext
```

- **写法**：UTF-8 文本，LF 换行，每行一个字段：字段名（小写字母、数字与 `_`）、一个空格、小写十六进制的字节；没有空行、注释或别的行，字段按上表的次序。TS 一侧不需要任何解析库：`Object.fromEntries(text.trimEnd().split("\n").map((line) => line.split(" ")))`，再把每个值两个字符一组读成字节。
- **`keys.txt`**：§8-3 的派生。`seed` 是 32 字节种子；`ed25519_seed`、`ml_dsa_44_seed` 是 HKDF-SHA256（盐 `sprawling remote key v1`，`info` 分别是 `ed25519`、`ml-dsa-44`）展开出的两份 32 字节子种子；`public` 是两半公钥相接，Ed25519（RFC 8032 的私钥就是这份子种子）在前，ML-DSA-44（FIPS 204 的 ξ 就是这份子种子）在后。
- **`session-keys.txt`**：§8-4 的会话密钥派生。`transcript` 是盐，`x25519_secret` 接 `ml_kem_secret` 是输入，`info` 为 `device to city`、`city to device` 的两次 HKDF-SHA256 展开各 32 字节。三个输入是固定的字节，不来自一次握手，所以这份文件只测 HKDF。
- **`seal.txt`**：§8-5 的封装。同一把城到设备的密钥 `key` 依次封两个负载：`frame` 是 `Payload::Frame` 的字节（首字节 `00`，后接 UTF-8 文字），计数 0；`lock` 是 `Payload::Lock`（一个字节 `01`），计数 1。nonce 是方向标签 `c->d` 的四个 ASCII 字节接 8 字节大端计数，计数不上线；AAD 为空；`*_sealed` 是密文接 16 字节的 GCM 标签。
- **`signature-*.txt`**：一把混合公钥、一条消息与它的混合签名（§8-3）。`signature-rust.txt` 由 Rust 用 `keys.txt` 的那把密钥签；aws-lc-rs 的 ML-DSA-44 签名带随机（hedged），所以只有 Ed25519 一半能逐字节复现，浏览器验整份。浏览器一侧签的那一份由客户端提交为 `signature-browser.txt`，字段相同；Rust 的测试验目录里每一个 `signature-*.txt`，所以它落地时 Rust 一侧不改代码。
- **`fixture.txt`**：单向夹具（D11）。`hello` 是设备的 `Hello`（设备 id、X25519 公钥、ML-KEM-768 封装密钥、nonce），`x25519_private` 与 `ml_kem_private`（FIPS 203 的 2400 字节解封装密钥）是它的两把临时私钥；`reply` 是 Rust 城对它的回答，`city_public` 是签回答的城公钥；`first_frame` 是握手之后城封给设备的第一帧（城到设备，计数 0），`plaintext` 是它打开后的负载字节（`Payload::Frame`，约定的文字）。设备走 §8-4 的设备一半：从 `hello` 与不含签名的 `reply` 算握手记录，用 `city_public` 验回答的签名，派生会话密钥，打开 `first_frame`，得出 `plaintext`。
- **浏览器一侧**：`client/src/core/remote/interop.test.ts` 读同一组文件，以页面的实现（WebCrypto 加 `@noble/post-quantum`，D21）重算确定的三份、验 `signature-rust.txt`、用固定的临时私钥打开 `fixture.txt` 的第一帧；它还用 `keys.txt` 的种子签 `signature-rust.txt` 的那条消息，写成 `signature-browser.txt`，Rust 一侧的 `every_signature_vector_verifies` 验它。这一份只在树上那一份在浏览器一侧验不过时由 `GOLDEN_WRITE=1 bun test --conditions=browser src/core/remote/interop.test.ts`（在 `client/` 里跑）重写。
- **重生成**：`GOLDEN_WRITE=1 cargo nextest run -p sprawling-remote-access -E 'test(/vector|fixture/)'`。确定的三份（`keys.txt`、`session-keys.txt`、`seal.txt`）每次重写，字节不变；`signature-rust.txt` 与 `fixture.txt` 带着随机（ML-DSA 的签名、城的临时密钥、新抽的 ML-KEM 密钥），只在提交入库的那一份在 Rust 一侧验不过或打不开时重写，所以协议没变时重跑一次不改一个字节，而标签、握手记录、派生或封装的任何改动都让它们打不开、随之重写。不带这个变量时，测试只读、只比，不写。
- **固定的临时密钥只经测试的缝进来**：读写这些文件的是 `#[cfg(test)]` 的 `remote_access::handshake::agreement::interop`，它从两把私钥造出设备的临时密钥；发行二进制里设备的临时密钥只有 `generate` 一个来处，`handshake` 没有可注入随机源的接口（D20）。


### 8-13 remote_access::confirm（形状 5 状态）

```rust
pub const CONFIRM_BYTES: usize = 5;
pub struct Confirm<V> { /* 至多一个等待中的请求 —— 私有 */ }   // Default：没有请求
impl<V> Confirm<V> {
    pub fn request(&mut self, verb: V, entropy: [u8; CONFIRM_BYTES], expires: TimeMs) -> String;
    pub fn confirm(&mut self, answer: &str, now: TimeMs) -> Result<V, AxError>;
}
```

- `request` 换下先前的请求，交回印在控制台上的正文（8 个 base32 符号，四个一组，`abcd-efgh`）；本模块之后只存正文的摘要，与 §8-2 的配对码同一个理由。
- `confirm` 不论对错都结束等待中的请求：答错、没有请求、`now` 不在到期之前，都以 `E_GATE_DENIED` 拒，恢复语是在页面上再按一次、照控制台上新印的码输入。答案的读法与配对码相同，大小写、空白与连字符不论。
- 对动词泛型，是因为开门的载荷（开多久）是装配层的类型。确认码 5 字节熵：一次请求只给一次猜的机会，40 位够。两分钟的期限由装配层给出（`bin::outside::asking`），与配对码的十分钟同一种做法。
- 迹向量：模块里的测试把 `spec/Confirm.lean` 的 `step` 逐字抄成参照，在十七个事件的字母表上穷举四个事件以内的每一条迹，逐条对照 `Confirm` 的结果；三条性质各另有一条具名向量。

D20 互通的证据是 §8-12 的五份文本文件，由 Rust 的测试写、两侧的测试读。写法是一行一个字段的十六进制，而不是 JSON：本 crate 不依赖 JSON 解析器，为测试加一条 `serde_json` 的开发依赖会让锁文件多一条边，手写一个 JSON 读写器又是为测试多一份文法；TS 一侧两种写法都只要一行代码。固定的设备临时密钥经 `#[cfg(test)]` 的缝进来，生产的 `handshake` 不加可注入随机源的接口：§8-4 的城一侧随机性在 aws-lc-rs 内部，加一个注入点也做不出双向固定的整次握手，只会让发行二进制多一条测试才用的路。带随机的两份只在验不过时重写：每次重写都改字节的夹具，会让与协议无关的重跑在评审里看起来像协议改了。落选的做法是跑测试时起 bun 现场比对（D11 已排除）。
-/

/-! ## 9 工作流程

1. 人在城的控制台输入 `/remote open --for 12h`：装配层取一个新纪元，`Door::open`，并启动所选通路。
2. `/remote pair phone`：装配层取 16 字节熵，`PairingCode::mint`，`Door::expect_pairing`（10 分钟到期），把地址、配对码正文与 `CityFingerprint::of(城公钥)` 按 §8-6 的片段写法印成二维码。
3. 手机扫码打开页面，页面在设备上生成设备密钥，走配对握手（§8-6）：`device_pair_hello` → `city_pair_reply` → `DevicePairing::claim` → `CityPairing::open_claim` → `PairingCode::read`、`Door::pair` → 城把 `DeviceId` 封好回给设备，连接结束。
4. 此后每次连接：握手（§8-4）证明设备持有已配对的私钥，`Door::admit` 开会话；每一帧进来先 `Opener::open`，再按首字节分开线协议帧与锁门（§8-5）；线协议帧先问 `Door::authority`，再按 `permits` 判它所属的动词类。
5. `/remote close`、设备发来的锁门或门到时：`Door::close` 或 `Door::tick`，装配层同时 `Route::close`；`/remote revoke phone`：`Door::revoke`。
-/

/-! ## 10 实现逻辑

- 门是一个值加 `&mut self` 方法，而不是 Lean 模型里的纯函数：装配层持有唯一一扇门，就地改比每步重建一次整张表便宜，性质不变。
- 三张表都是 `Vec`：一座城配对的设备与同时持有的会话以个位数计，线性查找比哈希表更短也更快；按设备数上界（§14）封顶之后复杂度无关紧要。
- `pair` 用 `swap_remove` 取走码：表内次序无意义，O(1)。

**签名算法：FN-DSA 还是 ML-DSA-44。** 选 ML-DSA-44 与 Ed25519 的混合（D1）。落选的 FN-DSA 小在公钥与签名，而一次会话只做一次握手，这点大小不影响任何体验；人需要抄写或保存的是私钥种子，两种算法都能从 32 字节种子确定地派生整对密钥，写成 base32 都是 52 个字符。FN-DSA（FIPS 206）仍是草案，签名依赖浮点高斯采样，NIST 自己说难以做成常数时间，而手机浏览器里的 JS 只有双精度浮点、没有审计过的实现。FIPS 206 定稿且有审计过的实现时，换进来是 `keys` 里多一个枚举分支。

D1 签名用 ML-DSA-44＋Ed25519 的混合，不用 FN-DSA。理由见上一段。混合而非单用 ML-DSA：攻击者必须同时攻破两者；Ed25519 那一半在浏览器里可用 WebCrypto 的不可导出密钥，页面脚本能用它签名却导不出它。

**默认通路：Quick Tunnel 还是命名隧道。** 选命名隧道（D3）。Quick Tunnel 不要账号，但每次启动换一个子域：人在外面无从得知新地址，而手机上的密钥与 PWA 按域名隔离，地址一换就全读不到，只能再加一个静态站与一个签名会合点去补。命名隧道要一个 Cloudflare 账号与一个域名，换来固定主机名，还能在边缘开 Access、WAF 与 Bot 规则，在机器流量到达这台电脑之前挡掉它们，页面里不加一行第三方脚本。

D3 默认通路是 Cloudflare 命名隧道；通路是一条缝，人可以换。理由见上一段。README 原有的决定是「本仓库不附带隧道，替你选一种就是替你做安全决定」。端到端的配对密钥把这件决定缩成一件事：通路是否原样送达页面（D10）。只看、只转发、甚至改动帧的通路什么也得不到，能读到内容的只有改写页面的通路，那是运营方的主动攻击。所以附带一个默认通路仍是替人做一件信任决定，只是范围小了，文档照这个范围写。

**成本与模型体验**：零字节。本 crate 不进任何 run 的上下文，城的居民不知道这扇门存在；远程门的五个 Ledger 事件由装配层写（`crates/kernel/Spec.lean` §8-81），它们怎样进窗口归 kernel。
-/

/-! ## 11 边界枚举

- 同一个码被两个连接同时出示：门在装配层只有一个持有者，两次 `pair` 串行，第二次找不到码。
- 时钟回拨：`tick` 与判定都只比较 `now` 与到期时刻；回拨只会让会话多活一会儿，不会让门在关着时打开，也不会让旧纪元的东西回来。
- 进程在门开着时崩溃：重启后 `Door::start` 关着，设备表由装配层读回，会话与配对全部作废——这就是期望的样子。
- 设备名为空或超长：`DeviceName::parse` 拒。
- 回答里出示的公钥与邀请里的指纹不符，或签名不是那把公钥签的：设备停在这一步，配对码没有离开设备。
- 认领在回答之前到达，或解不开：城结束连接，不兑任何码；码仍在门里，直到到期。
- 认领解开了，门却拒了码（过期、别的纪元、已兑过）：城结束连接；按 §8-1，码已从等待表取走。
- 同一个名字配对两次：门里是两台设备。名字是否唯一由装配层在 `expect_pairing` 之前判定，门不看名字。
- `command` 通路的命令在通路开着时自己结束了：外面再够不到 `local`，本 crate 不察觉；之后的 `close` 仍答成功，下一次 `open` 起一条新命令。
- 通路重启后主机名变了（`PerStart`）：已配对设备的页面读不到自己的密钥，只能重新配对；这正是 `Permanence` 要说出的事。
-/

/-! ## 12 错误处理

一个拒绝是三段式的 `AxError`：动作、主体、恢复。下表按码列出本 crate 何时答出它；每一条的恢复语写在 §8 对应的一节。

| 码 | 何时 | 之后的状态 |
|---|---|---|
| `E_GATE_DENIED` | 门关着、码不对、设备未配对（§8-1）；混合签名有一半验不过（§8-3）；对方的密钥材料被库拒（§8-4）；回答的指纹不符、城的签名不对、认领打不开或设备签名不对（§8-6）；一帧打不开（§8-5） | 门不变；码已从等待表取走的仍算用掉（§8-1）；握手的等待状态已被取走，连接结束；会话里一帧打不开即结束会话 |
| `E_INVALID_ARGS` | 重用纪元、撤销未知设备、设备名不合（§8-1）；公钥或签名的线形式长度不对（§8-3）；邀请里的配对码或指纹读不成（§8-6）；设备 id、公钥或权限的正文读不成（§8-11） | 门不变；读不成的设备表行由装配层拒整张表 |
| `E_WIRE_MISMATCH` | 握手消息长度不对（§8-4、§8-6）；负载的首字节未知、锁门后面多出字节、帧不是 UTF-8、负载为空（§8-5） | 连接结束 |
| `E_BUDGET_EXHAUSTED` | 一个方向的帧计数用尽（§8-5） | 会话结束，重连得到新密钥与从零开始的计数 |
| `E_CONFIG_INVALID` | `PublicUrl` 不合、隧道名不合、命令印错地址那一行（§8-7 到 §8-9）；vault 里城钥匙的正文读不成（§8-3） | 命令已被结束，通路没有开；读不成的城钥匙不派生任何密钥，换一把之前门不发邀请、不答握手 |
| `E_TOOL_UNAVAILABLE` | 程序起不来、在就绪或印出地址之前退出、回环上拿不到空闲端口（§8-8、§8-9） | 通路没有开 |
| `E_TIMEOUT` | 耐心用完仍未就绪或仍未印出地址（§8-8、§8-9） | 子进程已被结束，通路没有开 |
| `E_STORAGE_FATAL` | 密码库拒绝一次本该成功的派生、签名或封装（`keys::crypto_failure`） | 这一步没有产出；恢复语是重启城并报告平台与版本 |

D7 每个错误码能否不可能发生：`E_GATE_DENIED` 不能——它报的是外面来的东西不合格，这是门存在的理由；`E_INVALID_ARGS` 的重用纪元一条，由装配层每次开门都取新熵而在实践中不会发生，保留检查是因为它是旧纪元回来的唯一一条路。
-/

/-! ## 13 依赖选型

- 1a 只依赖 kernel（`B3Hash`、`TimeMs`、`AxError`）。
- 通路（2）不加依赖：子进程用 `std::process`，就绪用 `std::net::TcpStream` 上的一次 HTTP/1.0 请求，命令的那一行按 §8-9 的子集读，不引入 JSON 解析器与执行器（D15）。
- 1b 选 **aws-lc-rs**（已在依赖图里，经 rustls 引入）：它在稳定接口上提供 ML-KEM-768、ML-DSA-44、X25519、Ed25519、HKDF 与 AES-GCM，是一个带 FIPS 验证历史的库，比拼接几个各自维护的 RustCrypto crate 少一个信任面。浏览器侧：X25519、Ed25519、HKDF、AES-GCM 用 WebCrypto；ML-KEM 与 ML-DSA 需要一个纯 JS 实现，用 `@noble/post-quantum`（MIT），信任口径见 D21；加入它要改 `tools/xtask/src/npm.rs` 的 `RUNTIME` 表，是一次门的改动，按 AGENTS.md 单独提交。
- 互通向量（§8-12）不加依赖：十六进制的读写在测试里，不引入 JSON 解析器（D20）。

D21 浏览器一侧的 ML-KEM 与 ML-DSA 用 `@noble/post-quantum`，精确钉版本，只由远程页面懒加载；种子只在配对时显示一次（定规）。它是一个自审、未经独立审计的 0.x 库，而 Rust 一侧选 aws-lc-rs 的理由之一是它的 FIPS 验证历史，两侧的信任口径因此不同；接受它的条件是它的 ML-KEM-768 与 ML-DSA-44 过 §8-12 的向量与夹具。版本号只写在 `client/package.json`，`tools/xtask/src/npm.rs` 的 `RUNTIME` 表收它一项，两处都归客户端的那次改动。懒加载让没有打开远程页面的人不为它付字节。种子：页面在配对时把 52 个字符的种子给人看一次，不保存；两半密钥分开保存，Ed25519 那一半是 WebCrypto 的不可导出密钥（D1）。代价是重装 PWA 就要重新配对。落选的三种：把 aws-lc 或 RustCrypto 编成 wasm（两端同源，体积推断大一个数量级，aws-lc 编 wasm 未验证）；等浏览器原生支持 ML-KEM 与 ML-DSA（远程门推迟到不定的时候）；种子加密后存进 IndexedDB（页面持有种子，D1 的不可导出就不成立）。出现独立审计，或主流引擎的 WebCrypto 原生提供这两种算法时，重新考虑这一条。
-/

/-! ## 14 硬编码声明

- 配对码 16 字节熵（128 位）、10 分钟到期：到期由装配层给出，本 crate 不写死它。
- 设备名最长 64 个字符（`DeviceName::MAX_CHARS`）：够写「客厅的 iPad」，又不让一个名字撑坏设备列表。
- 城的指纹 32 字节（SHA-256），写成 base32 是 52 个字符；配对码正文 26 个字符。两者都进二维码里的 URL 片段，它们的长度决定二维码的密度。
- 配对标签 `sprawling remote pairing v1` 与握手的协议标签各一个，版本号在标签里，消息形状一改就换标签。
- 负载首字节：`0` 是线协议文本帧，`1` 是锁门。
- `cloudflare` 通路每 200 ms 问一次就绪，每一问连接与读写各限 200 ms：`cloudflared` 连上边缘通常要一到数秒，200 ms 的间隔让就绪之后的等待不超过一次间隔，又不让回环上的轮询成为负担。
- `command` 通路交给命令的环境变量名是 `SPRAWLING_REMOTE_LOCAL`（`route::command::LOCAL_ENV`），与城给子进程的其他变量同一个前缀。
- 互通向量里的种子、nonce、固定的 X25519 临时私钥与约定的明文是 `handshake::agreement::interop` 里的测试常量，只有这一处；它们改了，§8-12 的文件随之重生成。
- 隧道名最长 64 个字符：`cloudflared` 的名字写进命令行，这个长度足够任何人起的名字。
-/

/-! ## 15 影响面

- 新增 crate `remote_access`：根 `Cargo.toml` 的 members、`ARCHITECTURE.md` §3 的 depmap、`architecture.toml` 的模块图与 family 表。
- 本文件与 `spec/` 下的三个分部由根 `lakefile.toml` 的 `Spec` 库按 glob 收进构建，不必登记。
- 通路缝登记在 ARCHITECTURE §4 的缝表，`architecture.toml` 的 family 表说本 crate 也持有通路。
- 装配层（`bin::outside` 与 `bin::assembly::remote_door`，`crates/sprawling/Spec.lean` §8-139、§8-140、§8-151）是唯一调用方；通路的参数从城的 `[remote]` 来（`crates/city/Spec.lean` §8-39，D22），`Tunnel`、`RouteCommand`、`Permanence` 的字段改了，装配层的 `chosen` 与那张表一起改；depmap 的 `sprawling` 一行带着本 crate。二维码的片段写法由装配层印、由客户端读，两边以 §8-6 为准；两条监听路径与上面的消息以 §8-10 为准。
- `tools/fixtures/remote-handshake/`（§8-12）由本 crate 的测试写、由客户端的 TS 测试读：改 §8-3 的派生、§8-4 的标签与握手记录、§8-5 的 nonce 布局与负载首字节，这些文件随之重生成，客户端的测试须仍过。
-/

/-! ## 16 测试与约束

- `crates/remote_access/src/door/tests.rs`：§2 的七个场景，每个对应 Lean 模型的一组定理。
- `crates/remote_access/src/pairing.rs` 内的测试：读回、RFC 4648 向量、两份熵两个码。
- `crates/remote_access/src/door/spelling.rs` 内的测试：设备 id、公钥与权限的正文各读回原值；短了的 id 与不认识的权限字以 `E_INVALID_ARGS` 拒（§8-11）。
- `crates/remote_access/src/keys.rs`、`crates/remote_access/src/seal.rs` 内的测试与 `crates/remote_access/src/handshake/tests.rs`：§2 对 keys、handshake、seal 的各条；每条验证步骤被故意拿掉时，对应测试转红。
- `crates/remote_access/src/handshake/pairing/tests.rs`：§2 对配对握手的各条，每条对应 `Handshake.lean` 的一组定理；`crates/remote_access/src/route/tests.rs`：§2 对通路的各条，`command` 通路对一个脚本化子进程（Windows 上是 `cmd`，其余平台是 `sh`）答出的 `Opened` 与脚本化通路答出的相同。
- `crates/remote_access/src/handshake/agreement/interop.rs`：§8-12 的五份文件各有一条读回测试，名字里带 `vector` 或 `fixture`；带 `GOLDEN_WRITE=1` 时按 §8-12 重写。浏览器一侧的同一组检查在 `client/src/core/remote/interop.test.ts`，由 `just check-client` 跑。
- 约束：零 `unsafe`；门、配对码、密钥、握手与封装零 I/O，时间与熵从参数来，唯一例外是两种握手的临时密钥（§8-4、§8-6）；通路的 I/O 只经标准库（D14）。
- `crates/remote_access/spec/Handshake.lean`：配对握手的三条性质，由 `just models` 证明。它不驱动 Rust 代码；Rust 一侧与它对应的是 §2 里配对的各条。

D11 与浏览器的互通只做组件级已知答案向量，加一条从 Rust 城到 JS 设备的单向夹具。城一侧的临时 X25519 与 ML-KEM 随机性在 aws-lc-rs 内部（§8-4），每次的回答都不同，两端都固定的整次握手向量做不出来。组件向量覆盖：同一个种子在两侧得到同样的两半公钥；两个方向的签名互验；HKDF 展开；给定密钥下的 AES-GCM 封装与 §8-5 的负载首字节。单向夹具：Rust 城对一个固定的设备 `Hello` 生成一次回答与第一帧密文，提交入库，JS 设备用同样固定的临时密钥解出约定的明文；协议改动时重新生成。落选的是两个运行时同时在线的跨进程测试：它让 `just check` 依赖 bun，还要先按 `xtask boundary` 判定它放在哪里。JS 设备发往 Rust 城的那一半，由组件向量与 Rust 设备端的测试间接覆盖。
-/

/-! ## 17 文档关系

- `ARCHITECTURE.md`：§3 的 `depmap` 块（`remote_access: kernel`，`sprawling` 一行带着本 crate）、§4 的缝表（`remote_access::route` 一行）、§10 规则 3（`route::command` 的读线程与 `bin::outside::listener` 的任务）。这些改了，重读本文件 §7 与 §8-7。
- `architecture.toml`：本 crate 各行与 `[family.remote_access]`，锚点指向本文件或它的分部。新模块先在那里登记。
- `docs/glossary.md`：远程门、纪元、配对码、邀请、设备、通路、远程会话，并写明远程门与 Gate 的 door、远程会话与房间的 Session 不是一物（§6）。§6 的词改了，两处一起改。
- `crates/kernel/Spec.lean` §8-76（设备表的路径）与 §8-81（远程门的五个事件）、`crates/wire/Spec.lean` §19-2 的 `class` 列与 §8-66（中继与 `Refusal`）、`crates/sprawling/Spec.lean` §8-139 与 §8-140（门的看守、远程监听、控制台的 `/remote`）、`client/Spec.lean` §9（配对页的交互契约落地时写在那里，§3）。§8-1 的动词类、§8-5 的负载、§8-6 的邀请写法或 §8-10 的两条路径改了，重读这几节。
- `tools/fixtures/remote-handshake/` 与 `tools/README.md` 的 fixtures 一行（§8-12）：客户端的互通测试读这些文件；§8-3 到 §8-5 的任何字节改了，文件重生成，客户端的测试须仍过。
- `docs/operating.md` 里控制台 `/remote` 的那一段与 `[remote]` 的写法（两种通路、`tailscale serve` 的示例）：控制台的动词以 `crates/sprawling/Spec.lean` §8-140 为准，表的键以 `crates/city/Spec.lean` §8-39 为准，§8-8、§8-9 改了参数时三处一起改。
- `README.md` 与 `README.zh-CN.md` 的「它在哪里监听」一节照 D3 与 D10 写远程门与通路上剩下的那一件信任，`docs/getting-started.md` 与中文版的「另一台机器」一节指向 `docs/operating.md`；D3、D10 改了，这四处一起改，中英两份在同一个提交里。
-/
