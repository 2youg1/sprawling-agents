-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::prefix

规定 `answer::prefix`、`answer::layers`、`answer::skills`、`answer::git_status`（`crates/wire/src/` 下同名的文件）。一个 run 冻下的 system prompt、下一个 run 将被告知什么、一栋楼会做什么、它那里还有什么没提交。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
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
4. **技能一行而三类架子，`shelf` 同时说它来自哪一格、落在哪里**：按架子各造一张表会让「近的架子压过远的架子」这条既有规则在客户端被重写一遍。两个城内臂携地址，城外臂携第几条挂载与相对该架根的路径——城外那份没有地址，编一个会送读者去开一个不存在的文件（布局与优先级见 `crates/city/Spec.lean` §8-8）。`SkillLine.at` 随之删除：架子与落点分成两个字段，就有「说 library 却指向城外」这一态可写。`pinned_by` 按**名字加哈希**成对匹配——两次 run 之间被编辑过的 skill 是同一个名字下的两份文档，只按名字匹配会宣称早先那次 run 读到了后来才写的字。
5. **`Drift` 整个可缺席，而不是两个零。** 没有上游的分支与和上游齐平的分支不是一回事，读成 `0/0` 的页面会告诉人「你的工作已经推上去了」。
6. **`GitStatusAnswer.checkpoint` 携整条 `CommitAnswer`。** 变更栏旁边那一行要说出 run、房间、模型与花费，而这四样已经有了唯一形状；另造一个摘要类型就是第二个「一次提交是什么」。

**`CommitAnswer` 携 `spent: UsdMicros`**：一行提交画得出 run、房间与模型，也要画得出钱，「这次改动花了多少」才不必另开一页去查。携的是**那次 run 的总额**而不是这条提交的份额——检查点不被计价，把一次 run 的钱按检查点分摊会得到一个没有人测量过的数字。

**被否**：把 skill 与工作树的状态折进 `BuildingView`。楼页的那一帧是在每一次记录之后都会失效的读，而扫书架要走盘、读工作树要开仓库；合成一帧会让这两件慢事按城里的心跳重复发生，而它们各自只在有人打开那一栏时才需要一次。
-/

/-!
### 8-98 一个地址的提示层：`Query::Layers` 与 `Command::PutPrompt`

`Query::Prefix` 答一个 run 已经被告知了什么；本节答下一个在这个地址冻结的 run 将被告知什么，每份文档从哪一层来，并给人一扇改它们的门。

```rust
// Query
Layers { addr: Address },                    // → Answer::Layers(Box<LayersAnswer>)
pub struct LayersAnswer { pub addr: Address, pub documents: Vec<LayerDocument> }
pub struct LayerDocument {
    pub at: Address,                         // 文件在哪，或不在时会在哪
    pub version: B3Hash,                     // 盘上这一版的哈希，不在时是空字节的哈希：`PutPrompt` 的 `base`
    pub slot: PrefixSlot,                    // 它进前缀的哪一段
    pub kind: PromptKind,
    pub layer: PromptLayer,
    pub state: LayerState,
    pub shipped: Option<ShippedDefault>,     // 本版带默认文本的三份才有
}
pub enum PromptKind { City, Conventions, Identity, ModelNote { provider: String, model: String } }
pub enum PromptLayer { City, Building, Room }
pub enum LayerState {                        // 线上 { sent } | { replaced } | "absent" | { withheld }
    Sent { bytes: u64 },                     // 下一个 run 收到它
    Replaced { by: Address },                // 在，但近的一层替换了它
    Absent,                                  // 不在
    Withheld { why: String },                // 在，但不送：读界拒了它指向的地方，或它不是文件
}
pub struct ShippedDefault { pub version: B3Hash, pub standing: DefaultStanding }
pub enum DefaultStanding { Absent, Current, Earlier, Edited }

// Command
PutPrompt(PromptWrite),                      // 与 `PutRange(RangeWrite)` 同形
pub struct PromptWrite { pub at: Address, pub base: B3Hash, pub step: PromptStep, pub idem: IdemKey }
pub enum PromptStep { Own, Inherit, Restore }

// §8-83 的来源多一臂
pub enum VersionSource { OnDisk, Saved { .. }, Before { .. }, Shipped }
```

- **次序就是发送的次序**：City 段（城的 `City.md`、楼的 `City.md`），Building 段（工作区与楼根的 `AGENTS.md`），Resident 段（这个地址的身份文件、楼根以下每一级房间的 `AGENTS.md`），Run 段（城与楼两层的每一份模型提示）。同一段里远的在前。哪份文档送不送、谁替换谁由 accounting 读盘时判，判法与冻结前缀时同一个函数（accounting §8-21），页面不另算。
- **`slot` 上线而不由页面推**：哪份文档进哪一段是冻结的事，页面再写一张表就是第二个权威。复用 §8-35 的 `PrefixSlot`。
- **模型提示列出盘上的每一份**，而不只是某一个模型的：一栋楼的 run 用哪个模型由每次派活决定，问这一问时还不知道。同一个模型在楼里有一份时，城那份是 `Replaced`；没有时楼那一格也列出、为 `Absent`，与楼的 `City.md` 一样，页面因此能在那里「起一份」而不必自己拼楼层的地址。
- **`Withheld` 带原因**：房间里一个叫 `AGENTS.md` 的目录，或一个链接指向读界之外（城外、机密楼、保留子树），下一个 run 不收到它；冻结照常进行，页面在这里说出原因，因为这是唯一说得出它没被送的地方。
- **`shipped` 只给本版带默认文本的三份**（城根的 `City.md`、`MAYOR.md`、`CLERK.md`）：`version` 是默认文本的哈希，`standing` 是 city 的 `CopyStanding`（city §8-43）。页面据此写「当前默认／改过／旧版默认」。
- **`VersionSource::Shipped`**：`Query::Versions` 对这三份文档在列表末尾多给一版，它是本版的默认文本，答之前放进内容库；放进去了 `kept` 为真，内容库不收时为假，这一版照样列出。「与默认比较」因此就是 RefRain 已有的两版比较，不另写比较界面。
- **`PutPrompt` 是整份动作，逐字的修改走 `PutRange`**（§8-72）。`Own`：这一层还没有自己的一份时，从此刻管着这里的那一份抄一份起手（楼的 `City.md` 抄城的；楼的模型提示抄城的，城没有就是空的）；`Inherit`：删掉这一层的一份，远的一层重新管（只对楼的 `City.md` 与两层的模型提示成立，城根的三份只能写）；`Restore`：写回本版默认（只对带默认的三份成立，`MAYOR.md` 的身份区保留），副本回到 `Current`，以后随二进制的默认走。`base` 是页面看到的那一版的哈希，不在时是空字节的哈希；文件已经不是 `base` 时拒（`E_VERSION_CONFLICT`），人看到的才是被替换或删掉的那一版。哪些地址是提示文档、每一步对哪一份成立，由 city 的一张名单判（city §8-44），`PutRange` 读同一张名单放行保留子树里的这几份。
- **账本行沿用 `governed_document_written`**（kernel §8-79），多两个可缺省的键：`at`（不是城保留子树里那三份时的地址）与 `removed`（`Inherit` 删掉了文件）。
- 验收：accounting 的 `views::layers::tests` 与 `worker::commanding::prompting::tests`——楼有自己的 `City.md` 时城那份是 `Replaced`；`Inherit` 之后城那份重新 `Sent`；RefRain 经 `PutRange` 存一份楼的 `City.md`；`Versions` 对城根 `City.md` 列出本版默认。
-/
