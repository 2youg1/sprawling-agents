-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::credential

规定 `credential`（`crates/gateway/src/credential.rs`）：Custody：人交出的值进金库、以引用替换、在线上那一格兑付；凭证库选哪一家。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-4 gateway::credential（形状 4＋内缝 Vault）

```rust
pub(crate) trait Vault {                        // 内缝：两句话接口
    fn put(&mut self, reference: &SecretRef, value: Sealed<String>) -> Result<(), AxError>;
    fn get(&self, reference: &SecretRef) -> Result<Option<Sealed<String>>, AxError>;
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
    pub fn custody(&self) -> Custody;                          // 探测的结论：哪一家、能留多久、平台服务的原话
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError>;
                                    // 遮蔽即拒；空值即未配置；入参取 Zeroizing 非 Sealed：
                                    // `.expose(` 只在 EXPOSE_WHITELIST 所列的解封点，Custody 是库不是 sink——
                                    // 持 Sealed 者恒密封直至线上；PutSecret 命令在自己边界内转 Zeroizing。
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError>;   // 未命中→E_CREDENTIAL_MISSING；恒不跨操作缓存
    pub fn describe(&self, reference: &SecretRef) -> Described;                        // 恒不返回值
}
```

- **生产适配器＝操作系统的凭证库（Windows Credential Manager／macOS Keychain／Linux 内核 keyring），经 keyring 生态第 4 代接入：`keyring-core` 给出 `Entry` 与错误类型，每个目标平台各一个 store crate（`windows-native-keyring-store`、`apple-native-keyring-store` 的 `keychain` 模块、`linux-keyutils-keyring-store`）；兜底＝会话内存 BTreeMap（测试面同用）**；一个城把秘密写进哪一家，由 `Store` 一处命名。探针只试平台服务，失败即落到内存。**加密文件是第三家，带在类型里而尚无探针选它**：打开它的口令要在启动时向人要，那条接线是另一件事（§8-21）。
- **一个引用在凭证库里叫什么，只在 `credential::vault::platform` 定一次，且沿用第 3 代写下的名字。** 引用 `secret:<realm>/<name>` 取 service＝`sprawling/<realm>`、user＝`<name>`。Windows 上凭证的 target name 是 `<name>.sprawling/<realm>`，Linux 上 keyutils 的 description 是 `keyring-rs:<name>@sprawling/<realm>`，macOS 上就是 service 与 account 两个字段。前两者由 store 的显式修饰（`target`、`description`）给出，不交给 store 的默认拼法：第 4 代的 Linux store 默认拼 `keyring:<user>@<service>`，前缀与第 3 代不同，升级一次就会让人存过的 key 读成「从来没配过」；Windows store 的默认今天与第 3 代相同，写成显式修饰是为了让名字只有这一处权威，而不是跟着上游的默认走。代价：带显式 `target` 新写的 Windows 凭证不填 UserName 字段；查找只按 target name，故读写不受影响。store 每次调用时构造，不设进程级默认 store（`keyring_core::set_default_store`），于是 vault 之外没有代码能在同一进程里改变 key 落在哪里。
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
