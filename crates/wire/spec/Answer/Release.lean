-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::release

规定 `answer::release`（`crates/wire/src/` 下同名的文件）。这座城是哪一版、npm 上是哪一版，或为什么比不了。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-36 这是哪一版，npm 上是哪一版：`Query::NewestRelease`

```rust
pub enum Query { /* … */ NewestRelease }

pub struct ReleaseLine { pub version: String, pub released: String }
pub enum ReleaseAnswer {
    Stands { mine: ReleaseLine, registries: Vec<RegistryNewest>, verdict: ReleaseVerdict, update: UpdateHint },
    Unreleased { registries: Vec<RegistryNewest>, update: UpdateHint },
    Refused { refusal: AxError },
}
pub struct RegistryNewest { pub registry: Registry, pub reading: RegistryReading }
pub enum Registry { Npm, CratesIo }
pub enum RegistryReading { Read { newest: ReleaseLine }, Refused { refusal: AxError } }  // 每个注册表都问，所以只有读到与读不到两臂
pub enum InstallChannel { Npm, Cargo, Archive, Source }
pub struct UpdateHint { pub channel: InstallChannel, pub command: Option<String> }
```

`verdict` 是本城对这份二进制安装渠道的那个注册表的判定：`update.channel` 是 `Cargo` 时取 crates.io 那一行，其余取 npm 那一行——npm 的 `latest` 是发布流程最后写的那一处，也是口径 2 说的「真正解析的东西」，而 cargo 装的人只取得到 crates.io 上有的版本。所取的那个注册表读不到时整条答 `Refused`（口径 3）。crates.io 那一行是裸版本号（`0.0.9`），经 kernel 的 `Version::from_crates_version` 读、`stands_on_crates` 判：只比版本号，同一版本号恒答 `Current`，与 npm 的判定在每一对版本号不同的发布上相同（`crates/kernel/Spec.lean` §8-54-1、D35）；它的 `ReleaseLine.released` 是空串，因为注册表的版本串里没有日期，城不替它补一个。`update` 由 `release::channel` 按这份二进制自己的路径判（D24）：源码构建答 `Source`、命令为 `None`；发布版按路径分四种，路径里有 `node_modules`、`.bun` 或 `_npx` 一节是 `Npm`，所在目录等于 cargo 的 bin 目录是 `Cargo`（`cargo binstall` 装在同一目录，路径分不出，答同一条命令），旁边有发行归档的 `skills/` 是 `Archive`，都不是答 `Source`；读不出自己路径的二进制也答 `Source`，宁可不印命令，不印一条错渠道的命令。终端 `status --check` 与页面印同一个 `command`，命令只有这一处。

**五条口径：**

1. **人按下才发生，此外一律不发生。** 不在连上时问，不在定时器上问，也不搭另一个查询的车。`QUICKSTART.md` 的开场承诺是「什么都没装、没注册服务、删掉文件夹就干净」，一座按自己的时间表去够注册表的城，等于拿那句承诺去换一个没人问过的问题；§8-35 口径 6 对外包目录立的是同一条规矩。客户端因此在**按钮的处理函数里**调 `asking.ask`——Solid 在那里不给响应式 owner，于是这条答复没有 watcher，重连与事件都不会替人重问。它是这一面最慢的一条读，也是唯一一条由人选择何时付这个代价的读。
2. **问注册表（npm 与 crates.io），不问 GitHub。** 本项目每一次发布都是 pre-release，而 `GET /repos/{owner}/{repo}/releases/latest` 按设计排除 pre-release——它对本仓库答 404。看上去最像的那个端点恰是错的那个；npm 的 `latest` dist-tag 才是 `bunx sprawling` 真正解析的东西。
3. **三态穷尽，而第三态携拒绝。** 「你跑的是某个发布版，它站在这里」「你自己从源码构建的，没有可比的对象」「注册表读不到」是人接下来要做的三件不同的事。第三态不走 `Answer::Unavailable`：城是可用的、注册表不可用，页面必须能说清是哪一个，而拒绝里带的是 `kernel::reach` 已经定义的分阶段读数——名字没解析、连不上、握手失败、对方答了什么状态。
4. **判定在 Rust，页面只画字符串。** `ReleaseLine` 携两个已渲染好的串，谁比谁新由 `kernel::Release` 的 `Ord` 判——版本在前、日期在后，与 npm 对同样两个串的排序一致。客户端因此没有第二套排序规则可以漂掉。**败给的方案**：把六个数字发给页面自己比——那是把一条领域规则复制到另一门语言里。
5. **什么都不更新。** 归档路径归 `sprawling install`，npm 路径归 npm，第三方去覆写其中任何一条，就是「这个二进制住在哪」有了第二个权威（`tools/xtask/src/channel/shim.js` 已立此规）。因此终端与页面都只把该跑的命令印出来就停。
-/

/-!
### 8-47f 查询按它回答的东西命名：`Query::NewestRelease`

```rust
pub enum Query { /* … */ NewestRelease }   // 线上拼作 "newest_release"
```

- **名字说出答的是什么**：这条查询回答「这座城是哪一版、npm 上最新的是哪一版」（§8-36 的五条口径不变）。它原来叫 `Release`，与释放一个关停范围的 `Command::Release` 同名；控制台把 socket 能带的动词与查询列在一处，一个人看到两个 `release`，分不清哪个放开关停、哪个去问注册表。
- **只改查询，不改答复**：`Answer::Release` 与 `ReleaseAnswer` 保持原名。答复不出现在人能输入的地方，没有同名的歧义；改它只会多一次无人受益的线上换形。
- **`WIRE_V` 不动**：变体名进 `QUERY_NAMES`，名字一变 §8-1 的 golden 就变，旧页面在握手时被拒，而不是发出一条城不认识的查询；`WIRE_V` 只为「名字没换而语法换形」而升（D1），改名不属于那一类。新名字登记在 `docs/glossary.md` §6；旧拼法不进 `tools/xtask/lexicon.toml`，因为已发布版本的 `CHANGELOG.md` 如实记着它当时的名字，子串禁令会误伤那段历史。
- **被否：保留 `release` 作别名**。一条查询两个拼法，就是同一个名字有两个家；握手已经把旧页面挡在门外，别名没有读者。
-/

/-! D24 更新检查同时问 npm 与 crates.io，按这份二进制的安装方式给出更新命令

**决定**：`ReleaseAnswer::Stands` 与 `Unreleased` 的 `newest` 换成 `registries: Vec<RegistryNewest>`，并多一件 `update: UpdateHint`：

```rust
pub struct RegistryNewest { pub registry: Registry, pub reading: RegistryReading }
pub enum RegistryReading { Read { newest: ReleaseLine }, Refused { refusal: AxError } }  // 每个注册表都问，所以只有读到与读不到两臂
pub enum Registry { Npm, CratesIo }
pub enum InstallChannel { Npm, Cargo, Archive, Source }
pub struct UpdateHint { pub channel: InstallChannel, pub command: Option<String> }
```

安装方式从这份二进制自己的路径判：落在 npm 的全局包目录（经 `bunx`／`npx` 解出的缓存也算）是 `Npm`，命令 `npm install -g sprawling@latest`；落在 cargo 的 bin 目录（`$CARGO_HOME/bin`，缺省 `~/.cargo/bin`，Windows 上是 `%USERPROFILE%\.cargo\bin`）是 `Cargo`，命令 `cargo install sprawling --locked`；带着发行归档的兄弟文件（`skills/` 与物料清单）是 `Archive`，`command` 是一句「从发布页下载最新归档，再从里面跑 `sprawling install`」——归档没有能自己更新的包管理器；都不是则 `Source`，`command` 为 `None`。三个平台用同一套规则，只有路径的展开不同。仍然只在人按下时问（§8-36 口径 1），仍然什么都不更新（口径 5）；口径 2「问 npm、不问 GitHub」扩成「问 npm 与 crates.io、不问 GitHub」。

**理由**：页面只去 npm 查，而 crates.io 上也有发布（roadmap A10）；用 cargo 装的人照 npm 的命令更新，会装出第二份二进制。两边各自可能读不到，所以每个注册表各带自己的结果。

每一行的读数是两臂的 `RegistryReading` 而不是 `Result`：每个注册表都问，所以没有「还没有问」这一臂；serde 给 `Result` 的 `Ok`／`Err` 外壳不是线上其余各处的拼法。

**被否**：①只问与安装方式对应的那一个注册表：源码构建的人看不到任何一边的最新版；②让页面按路径猜命令：一条领域规则抄进另一门语言。

**重开参数**：出现第三个发布渠道（例如系统包管理器）时，`Registry` 与 `InstallChannel` 各加一臂。
-/
