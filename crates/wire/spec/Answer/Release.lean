-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::release

规定 `answer::release`（`crates/wire/src/` 下同名的文件）。这座城是哪一版、npm 上是哪一版，或为什么比不了。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-36 这是哪一版，npm 上是哪一版：`Query::NewestRelease`

```rust
pub enum Query { /* … */ NewestRelease }

pub struct ReleaseLine { pub version: String, pub released: String }
pub enum ReleaseAnswer {
    Stands { mine: ReleaseLine, newest: ReleaseLine, verdict: ReleaseVerdict },
    Unreleased { newest: ReleaseLine },
    Refused { refusal: AxError },
}
```

**五条口径：**

1. **人按下才发生，此外一律不发生。** 不在连上时问，不在定时器上问，也不搭另一个查询的车。`QUICKSTART.md` 的开场承诺是「什么都没装、没注册服务、删掉文件夹就干净」，一座按自己的时间表去够注册表的城，等于拿那句承诺去换一个没人问过的问题；§8-35 口径 6 对外包目录立的是同一条规矩。客户端因此在**按钮的处理函数里**调 `asking.ask`——Solid 在那里不给响应式 owner，于是这条答复没有 watcher，重连与事件都不会替人重问。
2. **问 npm，不问 GitHub。** 本项目每一次发布都是 pre-release，而 `GET /repos/{owner}/{repo}/releases/latest` 按设计排除 pre-release——它对本仓库答 404。看上去最像的那个端点恰是错的那个；npm 的 `latest` dist-tag 才是 `bunx sprawling` 真正解析的东西。
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
