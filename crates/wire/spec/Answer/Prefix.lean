-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::prefix

规定 `answer::prefix`、`answer::skills`、`answer::git_status`（`crates/wire/src/` 下同名的文件）。一个 run 冻下的 system prompt、一栋楼会做什么、它那里还有什么没提交。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-35 四条读：一个 agent 被告知了什么、一栋楼会做什么、它那里还有什么没提交

这四条读回答三个问题：「这个 agent 收到的 system prompt 是什么」（`prompt_assembled` 只记四段的哈希与来源注记，拿着哈希读不回原文）；「这栋楼会做什么」（`Query::RegistryView` 的书里没有 skill）；「此刻工作树在哪」（`Query::Changes` 比的是两个检查点，答不出分支名、与上游的距离、以及哪些文件还没进检查点）。

```rust
// Query 第 25–28 条（声明序，QUERY_NAMES 同序追加）
Prefix    { run: RunId },           // → Answer::Prefix(Box<PrefixAnswer>)
Content   { locator: Locator },     // → Answer::Content(Box<ContentAnswer>)
Skills    { building: Address },    // → Answer::Skills(Box<SkillsAnswer>)
GitStatus { building: Address },    // → Answer::GitStatus(Box<GitStatusAnswer>)

pub enum PrefixSlot { City, Building, Resident, Run }
pub struct PrefixSource  { pub addr: Address, pub kept: u64, pub dropped: u64 }
pub struct PrefixSegment { pub slot: PrefixSlot, pub hash: B3Hash, pub bytes: u64,
                           pub text: String, pub stored: bool,
                           pub sources: Vec<PrefixSource> }
pub struct PrefixAnswer  { pub run: RunId, pub segments: Vec<PrefixSegment> }
pub struct ContentAnswer { pub locator: Locator, pub text: String, pub bytes: u64,
                           pub truncated: bool, pub binary: bool }

pub enum SkillShelf { Library(Address), Building(Address),
                      External { index: u32, path: String } }
pub struct SkillLine  { pub name: String, pub section: String, pub shelf: SkillShelf,
                        pub disclosure: String, pub hash: B3Hash,
                        pub admitted: bool, pub pinned_by: Vec<RunId> }
pub struct SkillsAnswer { pub building: Address, pub skills: Vec<SkillLine>,
                          pub missing: Vec<String> }

pub struct Drift { pub ahead: u64, pub behind: u64 }
pub struct GitStatusAnswer { pub building: Address, pub branch: Option<String>,
                             pub drift: Option<Drift>, pub files: Vec<FileChange>,
                             pub checkpoint: Option<CommitAnswer> }
```

**六条口径：**

1. **`stored` 与空文本是两件事。** 仓库被清理过与这一段本来就没有内容，读者下一步做的事不同；用空串同时表示两者，会让一次数据丢失看起来像一次正常的装配。
2. **`Content` 只答 `cas:` 一种方案。** `file:` 指的是树上的一条路径，那是 `Query::Document` 的问题；在这里再答一次就是同一条规则的第二个权威。**被否**：让 `Content` 按方案分流兼收两种——它会把「读一个对象」和「读一个文件」的失败面合成一个，而两者恢复动作不同。
3. **`PrefixSource.dropped` 恒上线，即使是零。** 「一个字节都没裁」是一次测量，缺省的字段不是。
4. **技能一行而三类架子，`shelf` 同时说它来自哪一格、落在哪里**：按架子各造一张表会让「近的架子压过远的架子」这条既有规则在客户端被重写一遍。两个城内臂携地址，城外臂携第几条挂载与相对该架根的路径——城外那份没有地址，编一个会送读者去开一个不存在的文件（布局与优先级见 city-SPEC §8-8）。`SkillLine.at` 随之删除：架子与落点分成两个字段，就有「说 library 却指向城外」这一态可写。`pinned_by` 按**名字加哈希**成对匹配——两次 run 之间被编辑过的 skill 是同一个名字下的两份文档，只按名字匹配会宣称早先那次 run 读到了后来才写的字。
5. **`Drift` 整个可缺席，而不是两个零。** 没有上游的分支与和上游齐平的分支不是一回事，读成 `0/0` 的页面会告诉人「你的工作已经推上去了」。
6. **`GitStatusAnswer.checkpoint` 携整条 `CommitAnswer`。** 变更栏旁边那一行要说出 run、房间、模型与花费，而这四样已经有了唯一形状；另造一个摘要类型就是第二个「一次提交是什么」。

**`CommitAnswer` 携 `spent: UsdMicros`**：一行提交画得出 run、房间与模型，也要画得出钱，「这次改动花了多少」才不必另开一页去查。携的是**那次 run 的总额**而不是这条提交的份额——检查点不被计价，把一次 run 的钱按检查点分摊会得到一个没有人测量过的数字。

**被否**：把 skill 与工作树的状态折进 `BuildingView`。楼页的那一帧是在每一次记录之后都会失效的读，而扫书架要走盘、读工作树要开仓库；合成一帧会让这两件慢事按城里的心跳重复发生，而它们各自只在有人打开那一栏时才需要一次。
-/
