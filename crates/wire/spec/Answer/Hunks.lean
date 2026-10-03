-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::hunks

规定 `answer::hunks`（`crates/wire/src/` 下同名的文件）。两个检查点之间一个文件的补丁文本，与凭证扫描不回显的行。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-20 一段补丁是它自己的一次请求

```rust
Hunks { oid_a: GitOid, oid_b: GitOid, path: String },   // → Answer::Hunks(HunksAnswer)

pub struct HunksAnswer {
    pub oid_a: GitOid,
    pub oid_b: GitOid,
    pub path: String,
    pub lines: Vec<PatchLine>,
    pub withheld: Vec<Withheld>,
}
pub struct PatchLine { pub number: u32, pub text: String }
pub struct Withheld { pub number: u32, pub reason: String }
```

**这与 `storage::changes` 的模块头不矛盾，它就是那句话说的那次请求。** 那段头写着「计数，永不补丁文本……一段补丁必须是它自己的一次请求……而不是这个模块」。`Query::Changes` 答的是哪些文件动了、动了多少行；本查询答的是**一个文件**的补丁文本。两者不是同一个答的详略两版：前者的代价与改动文件数同阶，后者与一个文件的大小同阶，把它们并成一个答会让「看看这次改了哪些文件」付上整批补丁的代价。

- **没有它，本版开篇承诺的那件事在浏览器里做不到**：审一个 PR 得开终端敲 `git diff`。
- **同一次凭证扫描，不是第二份**：补丁文本经 `storage::checkpoint::scan::scan_staged` 的同一个判定过一遍。命中凭证形状的那一行**不回显**，答里只留它的行号与原因（`Withheld`）。第二份扫描器就是同一条规则的第二个权威，而漂掉的那个总是没人读的那个。
- **一次一个文件**：`path` 是必填的，没有「整批补丁」这个形状。
- **两个 oid 都不可变，所以这个答任何人都可以永久缓存**（同 `Changes` 的理由）。
- **这座城没写过的 oid 答 `Unavailable`**，与 `Changes`／`Commit` 同口径。
-/
