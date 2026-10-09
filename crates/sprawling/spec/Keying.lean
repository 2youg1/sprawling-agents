-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::keying

规定 `crates/sprawling/src/keying.rs` 与 `crates/sprawling/src/serving/key_file.rs`：每次服务的钥匙从哪来（`bin::keying`），以及原生客户端读它的钥匙文件（`bin::serving::key_file`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::keying` 旁的测试守住。
-/

/-!
## 8-22 每次服务都有一把钥匙：回环也铸，只有面向网络的那一把给人看

**原因**：回环端口不是凭据。跨站页面、同站另一个端口的页面、同机的另一个 OS 用户与居民的工具都到得了它（`crates/wire/spec/Reception/Entry.lean` wire D54）。所以 `wire::decide_bind` 不再收「什么都不要」的面，每一次服务都带一把钥匙，原生客户端出示它，浏览器配对之后出示会话令牌。

```rust
// bin::keying（形状 1 decision；纯，穷尽，无 I/O、无熵）
pub(crate) enum Keying {
    /// 人配置过的：我们没见过它被铸出来，故不显示。
    Adopt,
    /// 回环、没配置：当场铸一把native key，不显示，只写进钥匙文件。
    MintUnshown,
    /// 面向网络、没配置：当场铸一把，显示一次，也写进钥匙文件。
    Mint,
}
impl Keying { pub(crate) fn decide(bind: SocketAddr, configured: bool) -> Self; }

// bin::serving::door（形状 4 适配器；熵在这里取）
pub enum Keyed { Unshown(String), Adopted(String), Minted(String) }
impl Keyed { pub fn code(&self) -> &str; pub fn shown(&self) -> Option<&str>; }
pub fn key_for(bind: SocketAddr, configured: Option<String>) -> Result<Keyed, AxError>;

// bin::serving::key_file（形状 4 适配器）
pub(crate) struct KeyFile { … }
impl KeyFile {
    pub(crate) fn write(port: u16, key: &str) -> Result<Self, AxError>;   // 先写临时文件再改名
    pub(crate) fn remove(self) -> Result<(), AxError>;                     // 收口时
}
pub fn read_key(port: u16) -> Result<Option<String>, AxError>;             // 原生客户端没给 --token 时读（二进制里的 `wire_client` 经库的公开面读它）
pub(crate) fn runtime_dir() -> Result<PathBuf, AxError>;                   // <每用户运行目录>/sprawling
```

- **四格**：配置过（任何面）＝`Adopt`；回环×没配置＝`MintUnshown`；面向网络×没配置＝`Mint`。钥匙总是有的，所以 `decide_bind` 的「没有钥匙即拒」一格在产品里到不了，只给第三方 embedder。
- **人配置过的优先**：`SPRAWLING_PAIRING_TOKEN` 在场就采纳，不覆盖、不显示。一个配置过的城在回环上照样要它，人可以在这台电脑演练暴露的部署。
- **只有 `Minted` 给人看**：面向网络的城要让另一台机器上的原生客户端进来（`sprawling call --at <host:port> --token <key>`），所以没有控制台的城在横幅里把那把钥匙单独印一行，只印一次。它从不进地址：横幅、`/web` 与打开的浏览器都不带 `?token=`，因为地址会进浏览器历史、书签与截图，页面也不读它（`client/Spec.lean` §4-57b）。`Unshown` 只进钥匙文件与进程内的远程中继，从不进终端、URL、Ledger 或 diagnostics。
- **钥匙文件**：`<每用户运行目录>/sprawling/<port>.key`。Linux 用 `$XDG_RUNTIME_DIR`（没有时依次用 `$XDG_CACHE_HOME` 与 `$HOME/.cache`，不用所有账户共用的 `/tmp`），macOS 用 `$TMPDIR`，两者目录建成 0700、文件 0600；Windows 用 `%LOCALAPPDATA%`，目录建好后用 `icacls` 去掉继承、只授当前用户完全控制，失败即拒绝写钥匙。先写同目录的临时文件再改名，所以读者看不到写了一半的钥匙。临时文件以 `create_new` 新建：Unix 上 0600 只在新建时生效，所以先删掉同名的旧临时文件再新建，留在那里的文件不会带着它原来的权限被沿用；删掉之后又有人抢先建了它，新建就失败，城拒绝写钥匙。收口时删掉钥匙文件。不进环境变量：子进程会继承环境，`child::command` 也只清它知道的名字。读它的是 `sprawling call`、`dispatch`、`gauge`、`enrol` 在没给 `--token` 时按 `--at` 的端口找，而且只在 `--at` 的主机是回环字面量或 `localhost` 时找（`crates/sprawling/spec/WireClient.lean`）；别的主机要 `--token`。远程中继在进程内直接拿，不读文件。读者只按端口找文件，不验证端口上答话的是不是写文件的那座城：城被强制结束后文件留着，这台电脑上后来占了这个端口的别的程序会收到它。铸出的钥匙随那座城一起失效；人配置的 `SPRAWLING_PAIRING_TOKEN` 不会，SECURITY.md 写明这一点。
- **为什么不放城目录**：`sprawling call --at <addr>` 不知道城目录；城目录可能在权限宽松的盘或共享上；`LinuxNamespaces` 臂把宿主文件系统只读地暴露给居民。
- **Windows 的 DACL 为什么用 `icacls`**：AGENTS 的平台调用次序要先用标准库的安全接口；直接调 `SetNamedSecurityInfoW` 要 `unsafe`，而 `unsafe` 只许在 `crates/desktop/ffi`。`icacls` 随 Windows 自带，经 `child::command` 起，不开窗口。`%LOCALAPPDATA%` 继承来的 DACL 已只给本用户、SYSTEM 与 Administrators，`icacls` 把后两者也去掉。

D75 `icacls` 按绝对路径起：`<SystemRoot>\System32\icacls.exe`，`SystemRoot` 取 HKLM 的安装记录（`bin::privacy::windows::system32`，与 Windows PowerShell 同一处解析，Privacy.Cli D55），不按裸名经搜索顺序找。原因：搜索顺序的第一个位置是二进制所在的目录，城的默认位置就在二进制旁边，那个目录本用户可写，放进一个 `icacls.exe` 就能替掉这次授权。授权对象仍是 `USERDOMAIN\USERNAME`，不是进程令牌里的 SID：标准库没有读 SID 的接口，`windows-sys` 的接口都要 `unsafe`，唯一安全的读法是起一次 Windows PowerShell（`bin::privacy::identity`），冷启动要五到十几秒，每次起城都付不起。这两个变量同一用户的进程改得了，而改得了它们的进程本来就读得到这个目录，所以 SID 换来的只是纵深。重新打开它的条件：标准库或某个公开面安全的 crate 给出读当前令牌 SID 的接口。
- **配对码与开页码不在这里**：它们是浏览器的事，住在 `wire::reception::pairing`（`crates/wire/spec/Reception/Pairing.lean` §8-95）。`/web` 与首次开浏览器经 `bin::firstrun::open_paired` 把开页码写进只给本用户读的跳转文件（`crates/sprawling/spec/Firstrun.lean` §8-8）。
-/
