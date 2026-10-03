-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::credential::vault::file

规定 `credential::vault::file`（`crates/gateway/src/credential/vault/file.rs`）：口令解开的加密金库文件。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

本文件是描述，不是被证明的规格：它不含 Lean 定义与定理，它说的每一条由 `crates/gateway/src/credential/vault/file/tests.rs` 里的测试判。
-/

/-!
### 8-21 `gateway::credential::vault::file`：口令解开的加密金库文件（形状 4 适配器）

```rust
pub(crate) struct FileVault { /* path、派生密钥、salt、密文条目 —— 全私有 */ }
impl FileVault {
    pub(crate) const SOURCE: &'static str = "encrypted-file";
    pub(crate) const PERSISTENCE: Persistence = Persistence::AcrossRebootsWithPassphrase;
    /// 打开或创建；口令不对即 `E_CONFIG_INVALID` 具名拒绝，恒不 panic、恒不当作空金库
    pub(crate) fn open(path: PathBuf, passphrase: &Zeroizing<String>) -> Result<Self, AxError>;
}
impl Vault for FileVault { /* put / get / delete，写即整文件替换 */ }
pub enum Persistence { AcrossReboots, AcrossRebootsWithPassphrase, ThisBoot, ThisProcess }
```

**它答的是哪个威胁**：本城的工具调用能 grep 城所运行的电脑。明文躺在配置旁边的凭据，就是 Agent 能读进窗口的凭据；密文对这个读者有效。磁盘被拿走是另一个问题，本节不声称答它——持有口令者本来就不被挡在外面。

**条目逐条封装**：ChaCha20-Poly1305（RFC 8439），每条每次写盘取新的 96 位 nonce；AAD 绑格式版本与 `secret:realm/name`，故一条密文被挪到另一个名下就打不开（有测试钉住）。KEK 只来自口令 + Argon2id（RFC 9106 第二推荐档：64 MiB / 3 轮 / 1 道）。

**文件里有什么**：一个 JSON 对象，`version`、`salt`（base64）、`verifier`、`entries`（`secret:realm/name` → base64 的 nonce‖密文）。`verifier` 是一条不存凭据的条目，主题是 `verifier`，明文是一句固定的标记；`open` 先打开它，所以口令不对在门口就是一句具名拒绝，而不是第一次 `get` 时才失败，也不会被读成一个空金库。

**成本参数只有一个家**：文件只记 salt 与密文，不记三个 cost 数字——读盘时不听文件的，`FORMAT_VERSION` 变更才是改它们的方式。密钥派生一次、进程内持有：Argon2id 是故意慢的，每次读都派生等于给每次模型调用加一秒。

**原子替换归本模块**：`city::document` 对城文档做同一支舞，但它是 `city` 的 `pub(crate)`，而 gateway 不依赖 `city`、依赖方向上也不该依赖（ARCHITECTURE §2）。因此这一个文件的替换住在这里：整份写到旁边的 `<文件名>.staged`，落盘（`sync_all`）后 `rename` 到原名，留下的要么是旧文件、要么是新文件。各平台上谁能读它：macOS 与 Linux 上文件在写第一个字节之前就以 `0o600` 创建，只有本账户可读；Windows 上它继承所在目录的 ACL，城的目录在本账户的用户目录下时效果相同。把这支舞提升为两边都能调的权威，是一次连 `city` 两个调用点一起搬的迁移，归拥有 `kernel` 的那条路。**这一条是记录，不是遗漏。**

**等级句仍只有一处**：`Persistence::consequence()` 新增一档 `AcrossRebootsWithPassphrase`，句子是「密钥住在城所运行电脑的一个加密文件里，每次启动城要输一次口令」。`describe` 与 `resolve` 照旧读它。

**后端选择未接线**：`Custodian::probe` 只在 keyring 与内存之间选；何时向人要口令、口令从哪里来，接线落在 `custodian.rs`，是未定的一件（§3）。

**两个重开参数**：ML-KEM-1024 在「多机同步金库」立项时进——单机无对象可封；FN-DSA-1024 在「账本需要对第三方可验证的出处证明」立项时进——防篡改由哈希链承担。对称 AEAD 本身已抗量子，故这两条不是安全缺口。

**闸同步扩一条**（`xtask secret`）：城的保留子树（`kernel::RESERVED_PREFIX`）里出现凭据明文，按「城的记录」这条规则报，recovery 指向 `Custodian` 而不是手改；子树内非 UTF-8 的对象（CAS 产物）不按文本规则判，否则第一屏全是内容存储、真正那一行被埋掉。
-/
