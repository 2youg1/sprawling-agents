-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::hunks

规定 `hunks`（`crates/storage/src/` 下同名的文件）。两个 checkpoint 之间一个文件的补丁文本，凭证形状的行被扣下并点名。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-19 storage::hunks：一个文件的补丁文本（形状 4 adapter）

```rust
pub struct PatchLine { pub number: u32, pub text: String }
pub struct Withheld  { pub number: u32, pub reason: String }
pub struct FilePatch { pub lines: Vec<PatchLine>, pub withheld: Vec<Withheld> }

pub fn of_file(city_root: &Path, base: GitOid, head: Head, path: &str)
    -> Result<FilePatch, StorageError>;
```

**补丁文本是 `storage::changes` 之外的一次独立请求。** `storage::changes` 只计数、不搬补丁文本；一段补丁必须是它自己的一次请求，经同一次扫描作答。本模块就是那次请求，逐字兑现它开出的三个条件：

1. **一次一个文件**，`path` 必填，没有「整批补丁」这个形状。理由是代价：`changes` 的代价与改动文件数同阶，本函数与一个文件的大小同阶，合成一个答会让「这次改了哪些文件」付上整批补丁的钱。
2. **同一次凭证判定**，不是第二份。`checkpoint::scan_staged` 用 `kernel::secret::scan::scan` 判一个 staged blob，本模块判每一行补丁文本用的是同一个函数。命中的行**不回显**，只报行号与命中原因（provider 名，或熵判定）——理由与 `scan_staged` 对自己的命中说的同一句：把字节打出来以证明泄漏，本身就是泄漏。
3. **两端都是 commit 时，答可永久缓存**（同 `changes` 的理由）；`Head::WorkingTree` 答的是此刻的工作树，那是一波还没提交完的样子，也正是审阅进行中的改动时人看的那一份。

- **一个两次 checkpoint 之间没动过的文件答空补丁**，而不是报错：「它没动」是一个答案。「这座城没写过这个 oid」是另一个答案，由调用方（`accounting::views`）答 `Unavailable`——只有调用方知道人问的是什么。
- **测试用工作树而不是第二次 checkpoint**：checkpoint 根本不肯提交带凭证的 blob（`scan_staged` 拒），所以那一行只可能存在于盘上的树里。这条约束本身就是本模块的扫描不是多余的一层的证据：字节到不了 commit，但到得了 socket。
-/
