-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 让二进制成为一个词：搜索路径的两个判定

规定 `crates/sprawling/src/install.rs` 里的 `plan_append` 与 `plan_remove`（`bin::install`，形状 4 adapter：决定纯，落地薄）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（§8-9）。拷贝、注册表读写与广播是适配器的事，这里不建模。

一条搜索路径在模型里是它按分隔符切开的各段，按原来的次序。段的类型 `α` 是参数，因为判定只需要两件事：两段是不是同一个目录（`same`，Rust 里是 `same_directory`：去掉首尾空白与尾随的斜杠，Windows 上再不分大小写），以及一段是不是空段（`blank`）。空白的整串在模型里是空表，于是「空串变成那一个目录」不是一个特例。

四组性质：

* **追加是幂等的**——追加之后再判一次，答「已经在了」；
* **追加不遮挡**——原来的各段（去掉尾随的空段）原样在前，新目录在最后，所以装进去的目录不会压过系统已经解析到的东西；
* **追加再移除回到原值**——原值里没有这个目录、也没有尾随空段时，撤销恰好撤销那一次追加；
* **移除只动那一个目录**——其余每段按原次序留下，空段也留下；没有可移除的就答 `absent`，一字不写。
-/

namespace Sprawling.Install

variable {α : Type} (same : α → α → Bool) (blank : α → Bool)

/-- 搜索路径上已经有这个目录（Rust：`on_search_path`）。 -/
def onPath (entries : List α) (dir : α) : Bool :=
  entries.any (same · dir)

/-- 去掉尾随的空段：Rust 在拼接之前 `trim_end_matches(SEPARATOR)`，于是 `a;b;` 追加之后不会多出一个空段。 -/
def trimEnd (entries : List α) : List α :=
  (entries.reverse.dropWhile blank).reverse

/-- 安装对搜索路径做的事（Rust：`PathEdit`）。 -/
inductive Edit (α : Type) where
  | alreadyPresent
  | append (next : List α)
  deriving Repr, DecidableEq

/-- 卸载对搜索路径做的事（Rust：`PathRemoval`）。 -/
inductive Removal (α : Type) where
  | absent
  | rewrite (next : List α)
  deriving Repr, DecidableEq

/-- `plan_append`：已经在就不动；否则接在最后，而不是放在最前。 -/
def planAppend (entries : List α) (dir : α) : Edit α :=
  if onPath same entries dir then .alreadyPresent
  else .append (trimEnd blank entries ++ [dir])

/-- `plan_remove`：滤掉与这个目录相同的每一段；一段也没滤掉就是 `absent`。 -/
def planRemove (entries : List α) (dir : α) : Removal α :=
  if (entries.filter (fun e => !same e dir)).length = entries.length then .absent
  else .rewrite (entries.filter (fun e => !same e dir))

/-! ## 追加是幂等的 -/

theorem append_then_append_is_already_present (entries next : List α) (dir : α)
    (reflexive : same dir dir = true)
    (h : planAppend same blank entries dir = .append next) :
    planAppend same blank next dir = .alreadyPresent := by
  unfold planAppend at h
  split at h
  · cases h
  · cases h
    simp [planAppend, onPath, reflexive]

theorem present_is_left_alone (entries : List α) (dir : α)
    (h : onPath same entries dir = true) :
    planAppend same blank entries dir = .alreadyPresent := by
  simp [planAppend, h]

/-! ## 追加不遮挡 -/

theorem append_keeps_what_was_first (entries next : List α) (dir : α)
    (h : planAppend same blank entries dir = .append next) :
    next = trimEnd blank entries ++ [dir] := by
  unfold planAppend at h
  split at h
  · cases h
  · cases h; rfl

/-! ## 移除只动那一个目录 -/

theorem removal_is_the_filter (entries next : List α) (dir : α)
    (h : planRemove same entries dir = .rewrite next) :
    next = entries.filter (fun e => !same e dir) := by
  unfold planRemove at h
  split at h
  · cases h
  · cases h; rfl

theorem removal_leaves_no_copy (entries next : List α) (dir : α)
    (h : planRemove same entries dir = .rewrite next) :
    onPath same next dir = false := by
  rw [removal_is_the_filter same entries next dir h]
  simp [onPath, List.any_eq_false]

theorem absent_exactly_when_not_on_path (entries : List α) (dir : α) :
    planRemove same entries dir = .absent ↔ onPath same entries dir = false := by
  unfold planRemove onPath
  constructor
  · intro h
    split at h
    · rename_i hlen
      have hall := List.length_filter_eq_length_iff.mp hlen
      simpa [List.any_eq_false] using hall
    · cases h
  · intro h
    have hall : ∀ e ∈ entries, (!same e dir) = true := by
      simpa [List.any_eq_false] using h
    simp [List.filter_eq_self.mpr hall]

/-! ## 追加再移除回到原值 -/

theorem append_then_remove_restores (entries next : List α) (dir : α)
    (reflexive : same dir dir = true)
    (trimmed : trimEnd blank entries = entries)
    (h : planAppend same blank entries dir = .append next) :
    planRemove same next dir = .rewrite entries := by
  have absent : onPath same entries dir = false := by
    unfold planAppend at h
    split at h
    · cases h
    · simpa using ‹¬onPath same entries dir = true›
  have keep : entries.filter (fun e => !same e dir) = entries := by
    apply List.filter_eq_self.mpr
    simpa [onPath, List.any_eq_false] using absent
  rw [append_keeps_what_was_first same blank entries next dir h, trimmed]
  simp [planRemove, List.filter_append, keep, reflexive]

/-! ## 可实现性：每一支都有输入走到 -/

example : planAppend (· == ·) (· == "") ([] : List String) "bin" = .append ["bin"] := by decide
example : planAppend (· == ·) (· == "") ["a", "bin"] "bin" = .alreadyPresent := by decide
example : planAppend (· == ·) (· == "") ["a", ""] "bin" = .append ["a", "bin"] := by decide
example : planRemove (· == ·) ["a", "", "bin", "c"] "bin" = .rewrite ["a", "", "c"] := by decide
example : planRemove (· == ·) ["a", "c"] "bin" = .absent := by decide

end Sprawling.Install

/-!
## 8-9 让二进制成为一个词

必须守住的性质的权威是本文件上面的模型：追加幂等、追加不遮挡、追加再移除回到原值、移除只动那一个目录；本节是接口与做法。

**原因**：解压之后，那个 exe 不在任何搜索路径上。唯一的入口是找到那个文件夹再双击 `start.cmd`——找一个脚本比敲一条命令难，而桌面快捷方式比两者都难。`sprawling` 今天不是一个可以敲出来的词。

```rust
// bin::install（形状 4 adapter；决定纯，落地薄）
// 判定四项只在 Windows 编译：非 Windows 不改搜索路径（见下「非 Windows 拷贝照做」一条），
// 于是它们在别的平台没有调用方，dead_code 在 `-D warnings` 下即是错误。
#[cfg(target_os = "windows")] pub(crate) enum PathEdit { AlreadyPresent, Append(String) }
#[cfg(target_os = "windows")] pub(crate) enum PathRemoval { Absent, Rewrite(String) }

pub(crate) fn program_dir(local_app_data: Option<&Path>, home: Option<&Path>) -> Option<PathBuf>;
pub(crate) fn installed_name() -> String;                    // 恒为 sprawling + EXE_SUFFIX
#[cfg(target_os = "windows")] pub(crate) fn plan_append(current: &str, dir: &str) -> PathEdit;
#[cfg(target_os = "windows")] pub(crate) fn plan_remove(current: &str, dir: &str) -> PathRemoval;
pub(crate) fn install(uninstall: bool) -> Result<Report, AxError>;
```

- **一次安装做两件事，撤销就撤销这两件**：把正在运行的这个二进制拷进用户级程序目录，并把该目录写进用户级搜索路径。`--uninstall` 删掉它拷过去的那个文件、删掉它追加过的那一段，别的一概不碰。**恒不要管理员权限**，因为这两件事都在用户自己的 profile 里。
- **装进去的名字是推导的，不是抄来的**：`installed_name()` 恒给 `sprawling` 加平台后缀，不取当前 exe 的文件名。归档里的文件被改过名字，敲出来的那个词也仍然是 `sprawling`——否则「让它成为一个词」这件事取决于谁解压的。
- **搜索路径的判定住 Rust，落地住 PowerShell**：`plan_append`／`plan_remove` 是两个纯函数，输入是那条字符串本身，输出是穷尽枚举。适配器只负责取回原值、写回新值、广播。**幂等因此是一条可单测的判定**，而不是一次要在真注册表上观察的行为。
- **判定的编译面等于它的调用面**：这四项与 `PathOutcome::Rewritten` 原本无条件编译，而只有 Windows 那一支调用它们，故 macOS 的 clippy 以五条 `dead_code` 报错——而推送门只跑 Windows。修法是让平台条件跟着调用方走（`#[cfg(target_os = "windows")]`），而不是加一条 `allow`：**平台不同不是要压制的告警，是要写进类型里的事实**。`PathOutcome::Rewritten` 是枚举变体、两平台共用同一枚举，故取同文件已有的先例——`#[cfg_attr(not(target_os = "windows"), expect(dead_code, …))]`，与 `SelfService` 对称。
- **Windows 必须直接改注册表，且必须保住值类型**。`[Environment]::SetEnvironmentVariable(..., 'User')` 是所有教程里的写法，也是错的：它**恒写 REG_SZ**，把 `HKCU\Environment\Path` 的 `REG_EXPAND_SZ` 降级，其中的 `%VAR%` 从此不再展开（dotnet/runtime#1442、chocolatey/choco#699）。实测该值确为 `ExpandString`，故适配器读原值时用 `DoNotExpandEnvironmentNames`、写回时用读到的那个 `RegistryValueKind`——**读到什么类型就写回什么类型**，键不存在时才取 `ExpandString`（Path 在 Windows 上的默认类型）。
- **值经临时文件进出，不经命令行**：用户名含非 ASCII 字符时，命令行要穿过控制台代码页（例如 936），而 `PATH` 的整条值也可能逼近命令行长度上限。故 Rust 与 PowerShell 之间用一个 UTF-8 临时文件传值，文件路径经环境变量交接，两侧都不需要引号规则。
- **改完必须广播 `WM_SETTINGCHANGE`，否则新窗口也读不到**：Explorer 缓存环境块，从它启动的新控制台继承的是缓存。`#![forbid(unsafe_code)]` 关掉了在 Rust 里调 `SendMessageTimeout` 这条路，故广播由 PowerShell 的 `Add-Type` P/Invoke 完成（`HWND_BROADCAST=0xffff`、`WM_SETTINGCHANGE=0x1A`、`SMTO_ABORTIFHUNG=2`、5 秒上限）。实测一次约 1.1 秒。**广播失败不致命**：路径已经写下了，报一行提示说「注销后生效」，而不是把已经成功的一半说成失败。
- **非 Windows 拷贝照做，改 shell rc 不做**：装进 `~/.local` 下的 `bin`（该目录在现代发行版上默认已在 PATH 上）。**不写 shell rc**，理由记在这里而不是留一个静默的空分支：rc 文件有 bash／zsh／fish 三套语法与 `.profile`／`.bashrc`／`.zshrc` 多个候选，选错就是往人的登录脚本里写一行没有作用却要人自己删的东西；而从 Windows 交叉编译到 Linux 已知走不通（`aws-lc-sys` 需 C 交叉工具链），故这一支只能由 CI 编译与 lint，不能由我运行验收——**该 job 是 `platforms.yml` 的 macOS job，不再是 ubuntu**（ubuntu 已被裁出流水线）。**没有跑过的写入动作不写**。目录不在 PATH 上时，报告里给出该加的那一行，人自己贴。

  **Linux 进流水线**：上段论证的是「从无 Linux 的开发机器**交叉编译**走不通（`aws-lc-sys` 需 C 交叉工具链）」，不是「Linux 构建不成立」；GitHub 的原生 runner 在 Linux 上原生构建，根本不碰交叉工具链。落法两件，分开决策：①`release.yml` 矩阵加 `x86_64-unknown-linux-musl` **静态**一行——NixOS 没有 `/lib64/ld-linux-x86-64.so.2`，一份动态链接的 ubuntu 产物在 NixOS 上起不来，而静态一份同时覆盖 NixOS／Alpine／老发行版／容器。**待探项已探明，后端不换**：`aws-lc-sys` 由 `reqwest → hyper-rustls → rustls → aws-lc-rs` 引入（`cargo tree -i aws-lc-sys`，实测），而 aws-lc-sys 0.44.0 的 crate 源码内自带 `src/x86_64_unknown_linux_musl_crypto.rs` 且 README 的 Pregenerated Bindings 表列出该三元组——**该目标在预生成绑定名单上**，故不需要 bindgen，rustls 后端保持 `aws-lc-rs`，不切 `ring`（少一个 crypto 后端就少一处与 Windows／macOS 产物不同的实现）。它仍需一套 musl 的 C 工具链编译 AWS-LC 源码，故该 job 装 `musl-tools` 并令 `CC_x86_64_unknown_linux_musl=musl-gcc`；cmake 已在 runner 镜像上。**这一条只在 CI 上成立**（Windows 开发机上没有 Linux，也没有 musl 工具链），证据是依赖树与 crate 自带文件，不是一次绿色构建。**那次构建已经发生，两半各自有了答案。** aws-lc 那一半站住：`aws-lc-sys 0.44.0` 与 `aws-lc-rs 1.18.0` 在 musl 上开编且未报错，预生成绑定与 `musl-tools` 这套安排没有被证伪。构建停在另一处，而这一处上段根本没有论到：`keyring` 的 Linux 后端是 secret-service，树因此另到 `libdbus-sys`，它的 build script 跨目标边界问 pkg-config，而 pkg-config 默认拒答跨编译查询。**装 `libdbus-1-dev` 不是解法**：那会把宿主的 glibc D-Bus 递给一次静态 musl 链接，那是一个矛盾而不是一项配置——**一份静态 Linux 二进制与一个 D-Bus 凭据库不能同时为真**。故**那个先于构建的问题已经有答案，该行随之进矩阵。** 问题是：**Linux 装上之后，一把 API key 存在哪里**。答：内核自带的 keyring——凭证库的 Linux store 取 keyutils（`linux-keyutils-keyring-store`，`crates/gateway/Spec.lean` §8-4），那是一组 syscall，不需要会话总线、不需要动态库，静态 musl 与容器里同样成立，`libdbus-sys` 随之离树（实测：`Cargo.lock` 删 `dbus`／`dbus-secret-service`／`libdbus-sys`，增 `linux-keyutils`）。代价写在类型上：`KeyringVault::PERSISTENCE` 在 Linux 上恒为 `Persistence::ThisBoot`，`Persistence::consequence()` 是那句话的唯一权威，`resolve` 未命中时把它接在 recovery 后面，故重启吃掉的 key 自己会说话而不是静默失败（`crates/gateway/Spec.lean` §8-4）。仍欠的只剩一项：**真机验收必须在 Linux 上跑**，开发机器是 Windows，故 keyutils 的写—读—删探针只能到 CI 或一台 Linux 上才算数。该行除此之外所需的都已建成并保留：`just package` 收三元组、归档名带三元组、`install.sh` 认得那个名字。**那一处债已清**：`binary_path` 认了三元组并迁进 `xtask::package`，`just package <triple>` 一条配方打三行矩阵，`release.yml` 里重抄的步骤随之删除，musl 归档的名字带三元组（tools/xtask/Spec.lean §8-11）；②`flake.nix` 提供 devshell 与完整应用，Rust 从 `rust-toolchain.toml` 派生，应用版本读工作区 Cargo 清单；Nix 构建不接管 Windows／macOS 发布路径，验证由 `platforms.yml` 持有。Nix 打包接口与验收见本文件 D50。
- **`Report` 说的是已经发生的事**：拷到哪、搜索路径改没改（`AlreadyPresent` 与 `Append` 是两句不同的话）、广播成不成、以及「PATH 变更不会进已经开着的窗口」。**恒不说「安装成功」四个字**——人要知道的是下一步该开一个新窗口。

**本章测试**：`program_dir` 在两个平台各取本平台约定；`plan_append` 对空串、已含该目录（含大小写不同与带尾分隔符两形）、含其它目录三类输入分别给出正确的穷尽枚举；`plan_remove` 删得干净且保住其余段（含空段）；`plan_append` 之后 `plan_remove` 回到原值——**幂等与可逆是一对性质测试，不是一次手工观察**。判定既然只在 Windows 编译，这组测试也只在 Windows 编译：**测一个在本平台不存在的函数，测的是空**；推送门跑的正是 Windows，故这组测试每次推送都跑。

**本章验收（必须真做）**：`install` 之后**开一个新的 PowerShell 窗口**敲 `sprawling`；随后 `--uninstall`，再开新窗口确认 `Get-Command sprawling` 为空。
-/

/-!
## 8-118 发布前在回环地址上跑一遍 `install.sh`（`install.sh`、`.github/install-loopback.sh`、`release.yml` 的 `archive`）

**形状：适配器的验收。** `install.sh` 是 `curl | sh` 那条通道：它向发布 API 要最新一版，按平台后缀挑归档，下载、比对 sha256、解包、问一句 `status`，再把二进制交给 `sprawling install` 落位。这一串里每一步都只在真的发布之后才被执行，所以它坏了，第一个知道的是下载的人。

**接口。**
- `install.sh` 读 `SPRAWLING_API`：发布列表的地址，缺省是 `https://api.github.com/repos/${SPRAWLING_REPO}/releases`。设了它，列表（`?per_page=1`）与单个 tag（`/tags/<tag>`）都从这个地址问；归档的下载地址仍然取自列表里每个资产的 `browser_download_url`，不由脚本拼。这个变量同样服务镜像与 GitHub Enterprise，它不是只为测试开的口子。
- `.github/install-loopback.sh <archive-dir>`：`<archive-dir>` 里恰好一份 `.zip`（`just package` 在一个 matrix 行上产出的那份）。脚本在 `127.0.0.1` 上起 `python3 -m http.server`（端口取 0，由内核分配，从服务的第一行读出），写一份与 GitHub 同形的发布列表（`tag_name`；每个资产的 `name`、`size`、`digest: sha256:<hex>`、`browser_download_url`），然后以沙箱 `HOME` 跑 `install.sh`，最后执行 `sprawling install` 放进沙箱 `HOME` 的 `.local` 下 `bin` 里的那个文件的 `status`，打印第一行。
- 判定，三条都成立才绿：`install.sh` 退出 0；落位的文件与归档里的 `sprawling` 逐字节相同（`cmp`）；落位的二进制的 `status` 第一行以 `sprawling ` 开头。任何一条不成立，脚本以非 0 退出并说出是哪一条。
- 离机的请求一律失败：脚本给 `install.sh` 设 `https_proxy`／`http_proxy` 指向 `127.0.0.1:9`，`no_proxy=127.0.0.1`。所以一个不认 `SPRAWLING_API` 的 `install.sh` 不会偷偷装上 GitHub 上已发布的那一版，而是在第一次请求就红。
- `release.yml` 的 `archive` job 在 `just package` 之后、上传之前对 macOS 与 Linux 两行调它；Windows 那一行走 `install.ps1`，不在本节。

**决定。**
1. **比字节，不比版本行。** 版本行只说版本与发布日，两次构建同一个 tag 的二进制说同一句话，GitHub 上已发布的那一版也可能说同一句话；落位的文件与本次归档逐字节相同，才证明装上的是本次构建的这一份。败给的方案：断言版本行含 tag——`status` 的第一行根本不印 tag。
2. **端口由内核分配。** 固定端口在并行的 runner 或开发机上会撞；端口 0 在每台机器上都成立，不需要按机器调。
3. **归档与列表都经 HTTP 提供，而不是 `file://`。** `install.sh` 真实走的是 `curl -fsSL` 的 HTTP 路径，`file://` 会绕开状态码与重定向的处理，测到的不是人跑的那条路。
-/

/-! D50 Nix 包从同一源码构建完整应用（人的决定）

本节说明打包接口，不是形式证明；构建工具与网络沙箱属于环境，由实际构建与验收检查。

`flake.nix` 的 `packages.default` 与 `apps.default` 交出同一份完整应用，devshell 保留。
选择 release tag 的 flake 引用固定源码，`flake.lock` 固定构建依赖，Cargo 与 Bun 依赖分别只由
`Cargo.lock` 与 `client/bun.lock` 决定；Rust 工具链只读 `rust-toolchain.toml`，应用版本读工作区清单。
没有可用的发行 ref 时，二进制如实说 built from source，不能从版本号猜日期或制造 tag。
发行 ref 的解释仍由 `kernel::Release` 决定。

参数与理由：应用交付要求浏览器能打开真实客户端，因而占位页不满足 Nix 包接口。
`bun2nix` 的转换器在构建中从原 Bun 锁派生 Nix 表达式，其 `fetchBunDeps` 使用锁中的 integrity
下载并生成离线缓存；不在仓库维护第二份依赖表。nixpkgs 本身没有对应的通用 Bun 锁构建器，
因此转换器作为 flake 构建依赖加入，版本由 flake 锁持有。现有锁为 version 2，转换器仅接受 version 1，
但 npm 包的四元组仍为名称版本、下载地址、元数据与 integrity；转换输入只投影 packages，
保留每个四元组不变，并以转换器接受的 schema 标记生成临时输入，原锁仍交给 Bun 安装。
投影只接受 version 2、configVersion 1 与全部带 sha512 integrity 的 npm 四元组，其他形状
必须失败并要求更新转换适配，不能猜测新锁格式。临时投影和 Nix 表达式均在 store 构建，
不维护第二份依赖清单。代价是求值需要先构建转换结果
（import from derivation），首次求值可能编译转换器；它不进入应用运行闭包。

客户端经 `just build-web` 构建后由现有 `build.rs` 嵌入；Nix 从该文件的 `BUNDLE_DIR` 读取位置，
不声明第二个目录。Cargo 用同一钉住的 Rust 的 `makeRustPlatform.buildRustPackage`，默认 features
包含 sandbox。打包前要求生成的嵌入表声明 `CLIENT_COMPLETE = true`，否则构建退出非零并要求恢复
客户端构建步骤。安装后的 skills 与许可证在包的 share 目录，skills 来源与目录名读既有归档权威
`tools/xtask/src/package/contents.rs`，首次起城的模板与 skills 仍由 city 的构建脚本嵌入。
下载 integrity 不符、锁解析失败、客户端构建失败或 Rust 编译失败均让 derivation 失败，不能交出
占位页作为完整包；修复原锁、源文件或构建依赖后重建，不改变依赖版本来掩盖失败。

验收：`nix build .#default --print-build-logs` 在网络沙箱内构建完整包；`nix flake check`
保留工具链与 devshell 检查。`platforms.yml` 的 Nix 作业用包内二进制及 skills 驱动既有 Lean
acceptance，检查首次起城、真实客户端 HTTP 与其脚本和样式、skill 读取、派活、历史恢复。
`status` 核对清单版本，仅帮助文字或文件存在不构成应用验收。缺客户端的编译嵌入表必须被打包步骤拒绝。

被否：继续只交出占位页的开发运行；另写 Bun 下载器或手工维护 Nix 依赖锁；每次 release 自建
跨仓库更新机器人。自有 flake 随 tag 可取用不等于 nixpkgs 已收录，上游更新允许延迟并复用其通用
更新工具。重开条件：nixpkgs 提供原 Bun 锁构建器时移除转换器依赖，或交付改为由 Cargo 构建客户端时
重新评估客户端步骤。README 与上手文档的 Nix 段只陈述实际验收所支持的平台。
-/
