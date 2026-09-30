# remote_access-SPEC.md

> crate：`remote_access`（lib，依赖 kernel）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 模块：door（门的状态）／pairing（一次性配对码）／keys（混合签名密钥）／handshake（每次连接的握手）／seal（帧封装）；通路缝是下文 §3 列出的后续阶段。
> 本 crate 覆盖的语义：一个人离开这台电脑时，从外面够到自己这座城——谁能进、进来能做什么、何时失效、如何一键关上。

## 1 需求分解

一个人把城留在家里的电脑上出门，要能用平板或手机继续看城、回答提问、叫停。今天城只听回环地址，或者在局域网上凭一把配对令牌（wire-SPEC §8-41）；出了局域网就够不到。本 crate 给出「远程门」：

| 单元 | 一句话 | 阶段 |
|---|---|---|
| `door` | 门开着还是关着、在哪个纪元；还在等的配对；配过的设备；它们持有的会话 | 1a（已落） |
| `pairing` | 一次性配对码的铸造、给人看的写法、按人抄回来的写法读回 | 1a（已落） |
| `keys` | 城的身份密钥与设备密钥：ML-DSA-44 与 Ed25519 的混合签名 | 1b（已落） |
| `handshake` | 每次连接的混合密钥交换（X25519＋ML-KEM-768）与双方对握手记录的混合签名 | 1b（已落） |
| `seal` | 会话内每一帧的 AES-256-GCM 封装，计数器防重放 | 1b（已落） |
| `route`（缝） | 把 Mac 上的远程端口变成一个外面够得着的地址：开、关、自报能力 | 2 |

**安全与通路分成两层，只有通路可换。** 门、配对、密钥、握手、封装、权限是安全内核，不可换、不可关；通路只负责可达性，默认是 Cloudflare 命名隧道（§12-3），用户可以换成 Tailscale、局域网或一条自己的命令。因为握手认证的是配对时钉住的密钥而不是地址，帧又是端到端加密的，所以任何通路都只能看到密文、冒充不了任何一方：通路选错的最坏结果是连不上。

## 2 验收标准

- **door**：`adversary/design/RemoteDoor.lean` 证明的六条性质在 Rust 门上各有一个场景测试：关着的门什么都不放；关门结束每一个会话，再开也带不回来；配对码只用一次、只在自己的纪元、只在过期之前；撤销的设备不持有会话也开不了新的；会话不比门活得久；远程会话永远够不到只限本地的动词。另加一条：门拒绝在它所在的纪元里重开。`cargo nextest run -p sprawling-remote-access` 全绿，且每条性质的测试在对应实现被故意改坏时转红。
- **pairing**：铸出的码按人重新抄写（大小写、空格、连字符）读回同一个码；码的正文是 16 字节熵的 RFC 4648 base32（26 个符号）；两份熵给出两个码。
- **keys**：同一个种子给出同一把公钥，两个种子给出两把；签名只在两半都成立时成立，换掉任一半即不成立；错误长度的线形式被拒。
- **handshake**：两端握手后，一端封的帧另一端打得开，两个方向都是；设备拒绝一个它没有钉住的城；城拒绝一台持有别的密钥的设备；途中被改过的回复被拒；一次握手的 Finish 完不成另一次握手。
- **seal**：帧按封的次序打开；被重放、被丢掉前一帧、被改过、方向不对的帧都打不开。
- **Lean**：`just models` 构建 `Design`，`RemoteDoor` 无 `sorry`、无 `admit`、无 `axiom`。

## 3 假设与歧义

以下是这个接口今天尚未落地的部分，按阶段写成当前状态；每一段落地时，从这里删掉，写进它所属的 §8 章节。

- **浏览器一侧的互通未证。** Rust 一侧的握手与封装已落（§8-3 至 §8-5），两端都由 Rust 驱动的测试成立；缺的证据是：浏览器侧的 ML-KEM／ML-DSA 实现（§13）与 aws-lc-rs 的已知答案互通测试。它随客户端阶段落地。
- **2 通路缝未落。** 接口已定（§8.5）：`open(local) -> Reach { url }`、`close()`、`capabilities()`；第一批实现 `cloudflare-named`（默认）、`tailscale`、`lan`、`command`。缺的证据是：在一台出网受限（经代理、7844 端口被拦）的机器上，`cloudflared` 的预检输出能否被读成一句可执行的拒绝。
- **3 客户端未落。** 页面的配对页、握手、PWA 与窄屏布局属于 `client/`，不在本 crate；它们的交互契约写进 `client/client-SPEC.md` §7。
- **装配未落。** 控制台动词 `/remote open [--for <时长>]`、`/remote pair <名字> [--watch]`、`/remote close`、`/remote devices`、`/remote revoke <名字>|--all` 属于 `sprawling` 的控制台；它们不上线协议（§12-4）。设备表跨重启的持久化、Ledger 事件（门开、门关、设备配对、设备撤销、会话开始）属于装配层与 kernel 的事件表，落地时在 kernel-SPEC §8-4 增列。

## 4 现状分析

- `wire::auth::PairingToken` 与 `wire::server::decide_bind` 已经守住局域网这一面：非回环绑定必须带令牌，令牌只存摘要、常数时间比较。远程门不改它：`lan` 通路就是这一面，它仍用自己的令牌守握手之前的那一步。
- 城的线协议帧由 `wire::frames` 定义，远程门不改帧，只在帧外加一层封装（1b），所以 `WIRE_V` 与 schema 哈希不因本 crate 变化。
- 本 crate 1a 只依赖 kernel，零 I/O：时间与熵都是参数，与 `wire::auth` 的做法相同。

## 5 权威信源

- 门的性质：`adversary/design/RemoteDoor.lean`（本 crate 是它的实现，它是性质的权威）。
- base32：RFC 4648 §6，测试向量取自 §10（`"foobar"` → `MZXW6YTBOI======`）。
- ML-KEM：FIPS 203；ML-DSA：FIPS 204；X25519：RFC 7748；Ed25519：RFC 8032；HKDF：RFC 5869；AES-GCM：NIST SP 800-38D。
- 后量子混合密钥交换 `X25519MLKEM768` 已是 TLS 的默认候选：`cloudflared` 到 Cloudflare 边缘的连接在日志里报出这一曲线偏好（2026.9.3），Cloudflare 的 1.1.1.1 以 ML-DSA-44 验证 DNSSEC。
- Cloudflare Tunnel：命名隧道需要账号与托管在 Cloudflare 上的域名，主机名固定，可在边缘加 Access、WAF 与 Bot 规则；Quick Tunnel 每次启动换一个 `trycloudflare.com` 子域，不保证 SLA，不支持 SSE，并发 200 个请求（developers.cloudflare.com 的 Quick Tunnels 页）。

## 6 命名统一

门（door）、纪元（epoch）、配对码（pairing code）、设备（device）、会话（session）、权限（authority：`Watch`／`Act`）、动词类（verb class：`Read`／`Act`／`LocalOnly`）、通路（route）。「配对令牌」（pairing token）是 wire 局域网那一面的词，与本 crate 的「配对码」不是一物：令牌是一次服务期间反复出示的，配对码只兑一次。

## 7 模块边界

**三件邻居的活，及它们各自的主人**：

- 帧的类型、编码与握手版本归 `wire::frames`；本 crate 只在帧外封一层，不认识任何一个帧。
- 一帧属于哪个动词类（`Read`／`Act`／`LocalOnly`）的对照表归 `wire`，因为动词表住在那里；本 crate 只给出 `door::permits(Authority, VerbClass)` 这条判定。
- 随机字节、时钟、进程（`cloudflared`）与设备表的落盘归装配层 `bin::assembly`；本 crate 只收参数、只给判定。

依赖：`remote: kernel`（ARCHITECTURE §3 的 depmap）。`sprawling` 是唯一消费者。

## 8 接口先行

### 8-1 remote_access::door（形状 1 判定＋形状 5 状态）

```rust
pub struct Epoch([u8; 16]);        // from_entropy
pub struct DeviceId([u8; 16]);     // from_entropy
pub struct SessionId([u8; 16]);    // from_entropy
pub struct DeviceKey(Vec<u8>);     // 设备公钥的原始字节；1b 起由 keys 解释
pub struct DeviceName(String);     // parse：去首尾空白后 1..=64 个字符
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
pub const PUBLIC_BYTES: usize = 32 + 1312;      // Ed25519，然后 ML-DSA-44
pub const SIGNATURE_BYTES: usize = 64 + 2420;   // Ed25519，然后 ML-DSA-44
pub struct SigningKey { /* 两半密钥对 —— 私有 */ }
impl SigningKey {
    pub fn from_seed(seed: &[u8; SEED_BYTES]) -> Result<SigningKey, AxError>;
    pub fn public(&self) -> VerifyingKey;
    pub fn sign(&self, message: &[u8]) -> Result<Signature, AxError>;
}
pub struct VerifyingKey(Box<[u8; PUBLIC_BYTES]>);   // from_bytes／as_bytes／verify
pub struct Signature(Box<[u8; SIGNATURE_BYTES]>);   // from_bytes／as_bytes
```

- **一个种子派生两半**：HKDF-SHA256，盐 `sprawling remote key v1`，两半各用自己的标签（`ed25519`、`ml-dsa-44`），所以两半不共享任何密钥材料；盐里的版本号保证以后的派生不会产出以前的密钥。人要保存的只是这 32 字节。
- **验证两半都要成立，并且只给一个答案**：调用方从拒绝里得不出是哪一半没过。

### 8-4 remote_access::handshake（形状 1 判定＋形状 5 状态）

```rust
pub const HELLO_BYTES: usize = 16 + 32 + 1184 + 32;         // 设备 id、X25519、ML-KEM-768 封装密钥、nonce
pub const REPLY_BYTES: usize = 32 + 1088 + 32 + 2484;        // X25519、ML-KEM 密文、nonce、城的签名
pub struct Hello; pub struct Reply; pub struct Finish;       // from_bytes／as_bytes，定长
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

## 8.5 两个设计

**签名算法：FN-DSA 还是 ML-DSA-44。** 选 ML-DSA-44 与 Ed25519 的混合（§12-1）。落选的 FN-DSA 小在公钥与签名，而一次会话只做一次握手，这点大小不影响任何体验；人需要抄写或保存的是私钥种子，两种算法都能从 32 字节种子确定地派生整对密钥，写成 base32 都是 52 个字符。FN-DSA（FIPS 206）仍是草案，签名依赖浮点高斯采样，NIST 自己说难以做成常数时间，而手机浏览器里的 JS 只有双精度浮点、没有审计过的实现。FIPS 206 定稿且有审计过的实现时，换进来是 `keys` 里多一个枚举分支。

**默认通路：Quick Tunnel 还是命名隧道。** 选命名隧道（§12-3）。Quick Tunnel 不要账号，但每次启动换一个子域：人在外面无从得知新地址，而手机上的密钥与 PWA 按域名隔离，地址一换就全读不到，只能再加一个静态站与一个签名会合点去补。命名隧道要一个 Cloudflare 账号与一个域名，换来固定主机名，还能在边缘开 Access、WAF 与 Bot 规则，在机器流量到达这台电脑之前挡掉它们，页面里不加一行第三方脚本。

## 9 工作流程

1. 人在城的控制台输入 `/remote open --for 12h`：装配层取一个新纪元，`Door::open`，并启动所选通路。
2. `/remote pair phone`：装配层取 16 字节熵，`PairingCode::mint`，`Door::expect_pairing`（10 分钟到期），把正文与城公钥指纹、地址一起印成二维码。
3. 手机扫码，在页面里生成设备密钥，出示配对码与设备公钥：`PairingCode::read`，`Door::pair`。
4. 此后每次连接：握手（1b）证明设备持有已配对的私钥，`Door::admit` 开会话；每一帧进来先问 `Door::authority`，再按 `permits` 判它所属的动词类。
5. `/remote close` 或门到时：`Door::close` 或 `Door::tick`，装配层同时关掉通路；`/remote revoke phone`：`Door::revoke`。

## 10 实现逻辑

- 门是一个值加 `&mut self` 方法，而不是 Lean 模型里的纯函数：装配层持有唯一一扇门，就地改比每步重建一次整张表便宜，性质不变。
- 三张表都是 `Vec`：一座城配对的设备与同时持有的会话以个位数计，线性查找比哈希表更短也更快；按设备数上界（§14）封顶之后复杂度无关紧要。
- `pair` 用 `swap_remove` 取走码：表内次序无意义，O(1)。

## 11 边界枚举

- 同一个码被两个连接同时出示：门在装配层只有一个持有者，两次 `pair` 串行，第二次找不到码。
- 时钟回拨：`tick` 与判定都只比较 `now` 与到期时刻；回拨只会让会话多活一会儿，不会让门在关着时打开，也不会让旧纪元的东西回来。
- 进程在门开着时崩溃：重启后 `Door::start` 关着，设备表由装配层读回，会话与配对全部作废——这就是期望的样子。
- 设备名为空或超长：`DeviceName::parse` 拒。

## 12 Decisions

1. **签名用 ML-DSA-44＋Ed25519 的混合，不用 FN-DSA。** 理由见 §8.5。混合而非单用 ML-DSA：攻击者必须同时攻破两者；Ed25519 那一半在浏览器里可用 WebCrypto 的不可导出密钥，页面脚本能用它签名却导不出它。
2. **后量子放在三处：TLS、设备认证、帧封装。** TLS 负责传输层机密性，但 Cloudflare 隧道总在边缘解开 TLS；帧封装（1b）让边缘只看到密文，所以「现在截获、以后解密」对这一层也不成立。
3. **默认通路是 Cloudflare 命名隧道；通路是一条缝，用户可换。** 理由见 §8.5。README 原有的决定是「本仓库不附带隧道，替你选一种就是替你做安全决定」；它被这一条取代，因为安全由端到端的配对密钥保证，隧道只负责可达性，附带一个默认通路不再替人做安全决定。
4. **开门、配对、撤销只在控制台，不上线协议。** agent 拿到的任何工具（包括驱动本地页面的浏览器工具）都够不到它们。限制：Windows 与 macOS 上 `exec` 尚无操作系统级沙箱，agent 跑的命令拥有这个用户的权限；那个缺口由 exec 沙箱补，不由这扇门补。
5. **关门可以由任何人做，开门只能由人做。** 关门只会减少访问，所以远程设备自己也可以「锁门离开」。
6. **设备密钥在设备上生成。** 城只存公钥，城的存储泄露不让任何人登录；人要保存的恢复种子是设备自己的，城从未见过它。
7. 每个错误码能否不可能发生：`E_GATE_DENIED` 不能——它报的是外面来的东西不合格，这是门存在的理由；`E_INVALID_ARGS` 的重用纪元一条，由装配层每次开门都取新熵而在实践中不会发生，保留检查是因为它是旧纪元回来的唯一一条路。

## 13 依赖选型

- 1a 只依赖 kernel（`B3Hash`、`TimeMs`、`AxError`）。
- 1b 选 **aws-lc-rs**（已在依赖图里，经 rustls 引入）：它在稳定接口上提供 ML-KEM-768、ML-DSA-44、X25519、Ed25519、HKDF 与 AES-GCM，是一个带 FIPS 验证历史的库，比拼接几个各自维护的 RustCrypto crate 少一个信任面。浏览器侧：X25519、Ed25519、HKDF、AES-GCM 用 WebCrypto；ML-KEM 与 ML-DSA 需要一个纯 JS 实现，候选 `@noble/post-quantum`（MIT），加入它要改 `xtask/src/npm.rs` 的 `RUNTIME` 表，是一次门的改动，按 AGENTS.md 单独提交。

## 14 硬编码声明

- 配对码 16 字节熵（128 位）、10 分钟到期：到期由装配层给出，本 crate 不写死它。
- 设备名最长 64 个字符（`DeviceName::MAX_CHARS`）：够写「客厅的 iPad」，又不让一个名字撑坏设备列表。

## 15 影响面

- 新增 crate `remote_access`：根 `Cargo.toml` 的 members、`ARCHITECTURE.md` §3 的 depmap、`architecture.toml` 的模块图与 family 表。
- `adversary/lakefile.toml` 的 `Design` 库多一个根 `RemoteDoor`。
- 1a 没有调用方，产品二进制行为不变。

## 16 测试与约束

- `crates/remote_access/src/door/tests.rs`：§2 的七个场景，每个对应 Lean 模型的一组定理。
- `crates/remote_access/src/pairing.rs` 内的测试：读回、RFC 4648 向量、两份熵两个码。
- `crates/remote_access/src/keys.rs`、`crates/remote_access/src/seal.rs` 内的测试与 `crates/remote_access/src/handshake/tests.rs`：§2 对 keys、handshake、seal 的各条；每条验证步骤被故意拿掉时，对应测试转红。
- 约束：零 I/O，零 `unsafe`；时间与熵从参数来，唯一例外是握手的临时密钥（§8-4）。

## 17 模型体验

零字节：本 crate 不进任何 run 的上下文，城的居民不知道这扇门存在。

## 18 文档同步

- 1a：`ARCHITECTURE.md` §3 depmap、`architecture.toml`。
- 装配落地时：`README.md` 与 `README.zh-CN.md` 的「它在哪里监听」一节（改写 §12-3 取代的那句话）、`docs/getting-started.md` 与中文版的「另一台机器」一节、`docs/operating.md`、`docs/glossary.md`（门、纪元、配对码、设备、通路）、kernel-SPEC §8-4 的事件表。
