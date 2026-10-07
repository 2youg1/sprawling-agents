-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::credential

规定 `credential`（`crates/gateway/src/credential.rs`）：Custody：人交出的值进金库、以引用替换、在线上那一格兑付；凭证库选哪一家。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

本文件是描述，不是被证明的规格：它不含 Lean 定义与定理，它说的每一条由 `crates/gateway/src/credential/custodian/tests.rs` 与 `crates/gateway/src/credential/vault/platform.rs` 里的测试判，探针对真平台服务的那一半由装配期的人工清单判。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::credential::custodian::tests` 守住。
-/

/-!
### 8-4 gateway::credential（形状 4＋内缝 Vault）

```rust
pub(crate) trait Vault {                        // 内缝：两句话接口；值在库里是 Zeroizing，不是 Sealed
    fn put(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Zeroizing<String>>, AxError>;
    fn delete(&mut self, reference: &SecretRef) -> Result<(), AxError>;   // 探针与轮换用；不出对外接口
}
pub enum Persistence { /* 全部档位与 consequence() 见 §8-21 */ }
pub struct Described { pub configured: bool, pub source: String, pub persistence: Persistence, pub writable: bool }

pub enum Store { PlatformService, EncryptedFile, SessionMemory }   // 一个城把秘密写进哪一家
pub struct Custody { pub store: Store, pub persistence: Persistence,
                     pub refusal: Option<String> }   // 平台服务自己的拒词；它成事时与本城自选时皆 None

pub struct Custodian { /* backend: Box<dyn Vault>、store、refusal、env —— 私有 */ }
impl Custodian {
    /// Startup probe: write-read-delete against the platform service; any
    /// answer but the value it was asked to keep falls back to session
    /// memory and returns the provider_degraded payload to the caller for
    /// ledger append.
    pub fn probe() -> (Custodian, Option<Payload>);
    pub fn in_memory() -> Custodian;                           // 会话内存；测试与「本城自选内存」用
    pub fn with_env_reader(self, env: EnvReader) -> Custodian; // 测试的缝：换掉只读来源的读取器
    pub fn custody(&self) -> Custody;                          // 探测的结论：哪一家、能留多久、平台服务的原话
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
                                    // 遮蔽即拒；空值即未配置；入参取 Zeroizing 非 Sealed：
                                    // `.expose(` 只在 EXPOSE_WHITELIST 所列的解封点，Custody 是库不是 sink——
                                    // 持 Sealed 者恒密封直至线上；PutSecret 命令在自己边界内转 Zeroizing。
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError>;   // 未命中→E_CREDENTIAL_MISSING；恒不跨操作缓存
    pub fn describe(&self, reference: &SecretRef) -> Described;                        // 恒不返回值
    pub fn forget(&mut self, reference: &SecretRef) -> Result<(), AxError>;            // 遮蔽即拒（D33）；库里本就没有不算错
    pub fn persistence(&self) -> Persistence;                                          // 等于 custody().persistence
}
```

- **`Vault` 里的值是 `Zeroizing<String>`，不是 `Sealed<String>`**：`Sealed` 只在线上那一格解封（`xtask secret` 的 `EXPOSE_WHITELIST`），库是存放处不是 sink；`resolve` 在交出之前把库里的值重新包成 `Sealed`，所以 `Custodian` 之外没有人拿到 `Zeroizing` 的那一份。
- **探针用的引用是 `secret:sprawling/startup-probe`**：写进、读回、删掉，读回的值不等于写进的值也算失败；探针成功不留下这个条目。

- **生产适配器＝操作系统的凭证库（Windows Credential Manager／macOS Keychain／Linux 内核 keyring），经 keyring 生态第 4 代接入：`keyring-core` 给出 `Entry` 与错误类型，每个目标平台各一个 store crate（`windows-native-keyring-store`、`apple-native-keyring-store` 的 `keychain` 模块、`linux-keyutils-keyring-store`）；兜底＝会话内存 BTreeMap（测试面同用）**；一个城把秘密写进哪一家，由 `Store` 一处命名。探针只试平台服务，失败即落到内存。**加密文件是第三家，带在类型里而尚无探针选它**：打开它的口令要在启动时向人要，那条接线是另一件事（§8-21）。
- **一个引用在凭证库里叫什么，只在 `credential::vault::platform` 定一次，且沿用第 3 代写下的名字。** 引用 `secret:<realm>/<name>` 取 service＝`sprawling/<realm>`、user＝`<name>`。Windows 上凭证的 target name 是 `<name>.sprawling/<realm>`，Linux 上 keyutils 的 description 是 `keyring-rs:<name>@sprawling/<realm>`，macOS 上就是 service 与 account 两个字段。前两者由 store 的显式修饰（`target`、`description`）给出，不交给 store 的默认拼法：第 4 代的 Linux store 默认拼 `keyring:<user>@<service>`，前缀与第 3 代不同，升级一次就会让人存过的 key 读成「从来没配过」；Windows store 的默认今天与第 3 代相同，写成显式修饰是为了让名字只有这一处权威，而不是跟着上游的默认走。代价：带显式 `target` 新写的 Windows 凭证不填 UserName 字段；查找只按 target name，故读写不受影响。store 每次调用时构造，不设进程级默认 store（`keyring_core::set_default_store`），于是 vault 之外没有代码能在同一进程里改变 key 落在哪里。
- **每个平台上 vault 做什么**（D94 要求每一项写明三个平台）。同一张表也是城钥匙的表：城钥匙的种子是 vault 里的一个条目（`crates/remote_access/Spec.lean` §8-3、remote_access D23），它与 provider key 同一个存放处、同一个持久性等级、同一句 `consequence()`。

  | 平台 | 生产存放处（`Store::PlatformService`） | `Persistence` | 对 provider key 与城钥匙意味着什么 |
  |---|---|---|---|
  | Windows | 凭据管理器（`windows-native-keyring-store`），target name `<name>.sprawling/<realm>` | `AcrossReboots` | 跨重启保留；城钥匙不变，已配对设备跨重启保持配对 |
  | macOS | 钥匙串（`apple-native-keyring-store` 的 `keychain`），service 与 account 两个字段 | `AcrossReboots` | 跨重启保留；二进制更新后，钥匙串对新二进制的第一次读取弹出一次对话框，因为每一项的访问列表只认写它的二进制的 cdhash，而发行件只有 ad-hoc 签名，每次构建 cdhash 都变（`on-demand.yml` 的 `keychain` 作业，run 37120203124：存储是登录钥匙串，城钥匙那一项更新后原样留着）；User 允许之后 key 与城钥匙照旧读到，已配对的设备不必重新配对。推断：provider key 的每一项也这样各问一次，因为它们与城钥匙由同一个二进制写进同一个钥匙串。证据与无人应答时城停住的那一面在 `crates/remote_access/Spec.lean` §8-3 的 D23 与 §3 |
  | Linux | 内核 keyutils（`linux-keyutils-keyring-store`），description `keyring-rs:<name>@sprawling/<realm>` | `ThisBoot` | 活到这次开机结束；重启电脑后 key 要重新输入，城钥匙换新，设备要重新配对。用加密的 vault 文件（§8-21，`AcrossRebootsWithPassphrase`）就跨重启保留，代价是每次启动输一次口令；那条接线尚未落地（§8-21） |
  | 其他目标，或探针失败 | 会话内存（`MemoryVault`），探针写一条 `provider_degraded` | `ThisProcess` | 每次城重启都要重新输入 key、重新配对设备，`describe` 与 `resolve` 的恢复语照实说出这一句 |

  局限，三个平台相同：同一个系统用户下运行的程序都能读这个用户的凭据存放处；本城的工具调用也在这个用户下运行。加密文件答的正是其中「grep 得到明文」那一面（§8-21）。
- **一次探测的结论是一个值，不是三个读取口**：哪一家、能留多久、平台服务自己说了什么，三答出自同一次往返——分成三处报就可能说出「平台服务能用」与「重启什么都没了」这一对让人无从下手的答案。`custody()` 从 `describe` 与 `resolve` 所读的那批字段铸出，故它说不出一个两个读口都没在用的 store；`refusal` 是平台服务自己的拒词，它成事时与本城自选时皆 `None`。
- **Linux 存在内核 keyring，不存在 secret service，理由是发布产物。** 发布的 Linux 归档是一份静态 musl 二进制（`x86_64-unknown-linux-musl`），而 secret service 走 D-Bus，树因此另到 `libdbus-sys`，那要求链接宿主的 glibc D-Bus——一份静态二进制与一个 D-Bus 凭据库不能同时为真。故 Linux 的 store 取 `linux-keyutils-keyring-store`（keyutils，纯 syscall，不需要会话总线、不需要动态库、容器里同样成立），不取 secret service 的两个 store；`keyring` 包自己的默认 feature `v1` 在 Linux 上选的正是 zbus 的 secret service，这是本 crate 直接依赖 `keyring-core` 与各 store、而不依赖 `keyring` 的原因之一（D15）。代价写在类型上而不是写在注释里：内核 keyring 是内存，重启即清，故 `KeyringVault::PERSISTENCE` 在 Linux 上恒为 `Persistence::ThisBoot`，`SOURCE` 恒为 `kernel-keyring`。
- **等级只说一次，凡报它的地方都读同一处。** `Persistence::consequence()` 是「这个等级让人付出什么」的唯一权威句；`describe` 报状态（`source`＋`persistence`），`resolve` 未命中时把这句接在 recovery 后面——重启吃掉的 key 因此读作「重启清了内核 keyring，请再输一次」，而不是读作「你从来没配过」。探测成功不产 `provider_degraded`：Linux 上那是健康路径，每次启动报一条假警报只会让这个 kind 没人再读。
- realm/name 由调用方给定，本模块不生成名字：API key 走 `PutSecret` 命令，名字是人在登记时给的。**恒无计数器命名**——按捕获次序编号会让同一个值每场会话换一个名、两个值跨会话撞同一个名，于是旧账本里的引用兑到别人的凭据。一个值的短名若要由内容导出，口径只有 `runtime::redact::fingerprint` 一处。
- 环境变量是只读来源（键形 `SPRAWLING_SECRET_<REALM>_<NAME>`）：`describe.writable=false`；`set` 撞遮蔽即拒并指名遮蔽者；读取器可注入（edition 2024 的 set_var 不安全，测试恒不改进程环境）。
- A13 值正确性经真 Endpoint＋回环假服务在线断言（set→vault→resolve→写头，服务侧见原值）——credential 自身零 `.expose(`；probe() 对真平台服务的验证属装配期人工清单（测试不擅动开发者凭证库）。
-/

/-! D15 凭证库经 `keyring-core` 与各平台 store 接入，不经 `keyring`

决定见 §8-4。理由：`keyring` 第 4 代的默认 feature 在 Linux 上选 secret service，与静态 musl 发行件矛盾；关掉默认 feature 的 `keyring` 只剩一层转发，上游 README 建议应用直接依赖 `keyring-core` 与所需的 store。被否的备选：停在 `keyring` 3——它是锁里 `security-framework` 2 与 `windows-sys` 0.60 两族的来源之一，也不再是上游维护的那条线。
-/

/-! D22 探针落到会话内存时，编不出的 `provider_degraded` 通告并进 `custody()` 的拒词，不被丢掉

决定：`degraded_payload` 返回 `Result<Payload, AxError>`；探针三条落到内存的路都经 `Custodian::fell_back`，它把这个结果交给 `in_session_memory`。编码成功时通告照旧交给调用方入账，`refusal` 是平台服务的原话；编码失败时没有一行账能说出这次降级，于是 `refusal` 写成「平台服务的原话; the provider_degraded notice for it could not be encoded: 那个 `AxError`（码、动作、主题）」，通告为 `None`，doctor 的报告（`custody()`）把两件事一起读出。`probe()` 的签名 `(Custodian, Option<Payload>)` 不变。理由：`VaultFellBack` 只有字符串字段，`Payload::of` 今天不会失败，但 `.ok()` 让「这次降级没人知道」在它失败的那一天静默成立，而 §8-4 写的是降级恒不静默。被否：①`probe()` 返回 `Result<Option<Payload>, AxError>`——两个调用方（装配层开金库与 doctor）都只能把它记下或丢掉，而城照样落在会话内存上跑，返回值多一层却不多一个可做的决定；②编码失败时 panic——发行 profile 是 `panic = "abort"`，城因为一句通告写不出而起不来。三个平台：平台服务各不相同（§8-4 的表），落到会话内存之后的这条路与平台无关，三个平台报同一个失败。
-/

/-! D24 一个凭证环境变量的值不是 Unicode 时，它是一个点名变量的配置错，不是「没配过」

决定：`EnvReader` 答 `std::env::var` 自己的 `Result<String, VarError>`。值不是 Unicode 的变量照样算设了：它遮住金库，所以 `set` 照遮蔽拒绝；`resolve` 以 `E_CONFIG_INVALID` 拒，主题是「`<变量名>` is set to a value that is not Unicode」，恢复语叫人把这个变量设成 key 的文本，或取消它让金库作答；`describe` 报来源是这个环境变量、`configured` 为假、不可写，与 `resolve` 的答案一致。空值与没设照旧让金库作答。理由：读取器原来是 `std::env::var(key).ok()`，把「设了却读不出」与「没设」并成一个 `None`，于是金库里的旧 key 静默作答，或者人读到「请存这个凭证」，而他明明设了变量。被否：①读不出时当没设——就是旧的读法；②有损地转成字符串再用——发给 provider 的就不是人给的 key。三个平台：不是 Unicode 的值在 macOS 与 Linux 上是不合法的 UTF-8 字节，在 Windows 上是落单的代理项，`std::env::var` 在三处都答 `VarError::NotUnicode`，所以三处拒得一样。
-/

/-! D33 删一把 Key 只删库里那一份，环境变量提供的 Key 拒删并说要 unset 哪个变量

决定：`Custodian::forget(reference)` 先问与 `set` 同一个遮蔽判定（`shaded`，一处定义）：这个引用的环境变量设了（非空，或不是 Unicode，D24），就以 `E_CONFIG_INVALID` 拒，动作是 `forget credential`，主题点名引用与变量名，恢复语叫人在城启动的环境里 unset 这个变量；城不改自己的环境。没有遮蔽时交给后端的 `delete`：三家后端对一个本就没有的条目都答成功（平台服务的 `NoEntry`、内存表、加密文件），所以连按两次「删除」与删一个从没存过的 Key 都不是错。「还有登记在用这把 Key」不在本模块判：本模块不知道城里谁在用哪个引用，那是持有端点簿与配置梯子的执行者的事（`crates/accounting/spec/Worker.lean` §8-37）；端点这一侧的答案是 `AttachedEndpoint::references()`——列了账号时是每个账号的引用，没列时是旧的单个凭据的引用，`has_credential` 读的也是它，「这个端点用哪些 Key」只有这一处。理由：删掉库里那一份而环境变量照旧作答，人读到「已删除」而 provider 照收这个 Key，比拒绝更糟。被否：①遮蔽时照删库里那一份并报成功——同上；②删一个不存在的条目报 `E_CREDENTIAL_MISSING`——页面在删之前已经读过 Key 的状态，两次之间别处删掉了它，人要的结果已经成立。三个平台：遮蔽判定读进程环境，与平台无关；`delete` 在 Windows 凭据管理器、macOS 钥匙串、Linux 内核 keyring 上都把「没有这一项」读成 `NoEntry` 并答成功。
-/

/-! D31 隐私 owner 的只读核对在 Vault 内完成
`credential::identity::verify_platform_identity(reference, observed)` 由 gateway 根接口
重导出为 `gateway::verify_platform_identity`，只读取
平台 Vault，并在模块内部比较 stored 与 observed；返回 Result<(), AxError>，
不返回原值，不调用 Sealed::expose，不使用 provider 的环境遮蔽，不运行 probe。
身份比较的唯一 authority 是 gateway 根接口重导出的
`verify_identity_binding(reference, observed, read)`；read 是 FnOnce(&SecretRef)
到 Result<Option<Zeroizing<String>>, AxError> 的读取缝，生产端调用现有
KeyringVault::get，派生检查提供内存条目，函数只交出 Result<(), AxError>。
此缝不新增 Vault、不改变凭据位置、不允许写入，也不把 stored identity 返回调用方。
拒绝是三种之一，各带一句只回答它自己的 recovery：读取被拒（保留来源的错误码，
移除可能复述私密输入的诊断文字，recovery 叫人解锁平台凭据服务后再问）、
缺失（CredentialMissing，recovery 叫人登录持有该绑定的账户）、
不匹配或观察到的身份为空（ConfigInvalid，recovery 叫人以拥有该历史的
Windows 账户运行）。任何失败都不写、删除或重建引用；读取缝只交出值，
函数拿不到可写的 Vault，所以零写由类型持有，测试不再复述。
此入口核对真实身份与既有引用，不建立 journal 的随机身份绑定，不授予写权限。
测试经 MemoryVault 检查缺失、匹配、不匹配与空身份，经拒绝的读取缝检查
错误码保留而诊断文字不出现；都比较整个 AxError。
被否：三种拒绝共用一句 recovery——不匹配时叫人「解锁凭据服务」，读者照做也无用。
-/
