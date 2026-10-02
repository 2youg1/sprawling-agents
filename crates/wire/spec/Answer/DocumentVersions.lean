-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::document_versions

规定 `answer::document_versions`（`crates/wire/src/` 下同名的文件）。一份文档的各个版本，新的在前。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-79 一份文档的版本：`Query::Versions`

```rust
// Query
Versions { at: Address },                    // → Answer::Versions(Box<VersionsAnswer>)
pub struct VersionsAnswer { pub at: Address, pub versions: Vec<DocumentVersion>, pub more: bool }
pub struct DocumentVersion {
    pub version: B3Hash,
    pub bytes: Option<u64>,                  // 账本行或内容库说得出时才有
    pub kept: bool,                          // 内容库里有这一版的字节，Range／Bytes／Export 读得到
    pub source: VersionSource,
}
pub enum VersionSource {                     // 线上 "on_disk" | { saved } | { before }
    OnDisk,                                  // 盘上此刻的这一版，没有一次记下的保存写出它
    Saved { seq: Seq, at: TimeMs },          // 经页面写下：一次保存，或人接受的提案卡；记下它的那一行
    Before { seq: Seq },                     // seq 那次保存所基于的一版，而它之前没有一次记下的保存写出它
}
pub const VERSIONS_MAX: usize = 100;
```

- **来源只有两种，账本说得出的与说不出的。** 经页面写下的每一版都有一行 `document_written`（kernel §8-83），行上有时刻、基线与新版本，写它的是人（一次保存，或一次提案的决定）。居民的 `edit`、`exec`，人自己的编辑器，都不经过城的文档门，它们写下的版本账本里没有一行：答复只在两处见到它们——盘上此刻的一版与最新一次保存的结果不同时（`OnDisk`），以及一次保存所基于的那一版不是前一次保存的结果时（`Before`）。这两种不写时刻、不写谁，因为城不知道；页面写「页面之外写下」，而不猜一个居民。
- **新的在前，至多 `VERSIONS_MAX`（100）版。** 盘上的那一版（若有）第一，然后每一次保存从新到旧，每一次保存的结果之后紧跟它所基于、却不是前一次保存写出的那一版。更早的版本截掉时 `more` 为真；页面写出「更早的版本没有列出」。100 是一屏能翻完的列表，不是读数；重开参数：一份文档真的有上百次保存、而人要翻到更早的那一版时，按 `seq` 分页。
- **`kept` 说的是内容库此刻有没有这一版**（`storage::Cas::contains`）。城读到或写下的每一版都进内容库（§8-69、accounting 8-21），所以 `kept` 为假的只有城从没读过的那几版：一次保存之前、页面之外写下、又在任何一次读之前被再改掉的那一版。页面对 `kept` 为假的一版不给比较与导出。
- **问的那一刻读一次盘**：盘上的一版要说出来，所以这一问在视图的锁之外读文件（与 `Document` 同一扇门，读到的一版同样进内容库）。文件不在、读不了时没有 `OnDisk` 一行，账本里的保存照列。
- 验收：accounting 的 `views::versions::tests`——两次保存之间盘上被改过一次，答复是盘上的一版、第二次保存、它所基于的那一版（`Before`）、第一次保存、第一次所基于的那一版，各带 `kept`；没有任何保存的文件只答盘上的一版。
-/
