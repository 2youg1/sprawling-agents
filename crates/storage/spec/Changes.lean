-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::changes

规定 `changes`（`crates/storage/src/` 下同名的文件）。两个 checkpoint 之间动了什么：路径与计数，不含补丁文本。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-13 storage::changes（形状 4 适配器；git2）

```rust
// 这三个值类型住在 `kernel::change`（`kernel` 的根重导出它们），因为 wire 的答复也携带它们；本模块只产出它们。
pub enum Lines { Counted { added: u32, removed: u32 }, Binary }
pub enum How    { Added, Modified, Deleted, Renamed { from: String } }
pub struct FileChange { pub path: String, pub how: How, pub lines: Lines }
pub enum Head   { Commit(GitOid), WorkingTree }
pub fn between(city_root: &Path, base: GitOid, head: Head)
    -> Result<Vec<FileChange>, StorageError>;
/// 每个 oid 的父提交，按提交对象自己记的次序；仓库里没有的 oid 答 None。仓库只开一次。
pub fn parents_of(city_root: &Path, oids: &[GitOid])
    -> Result<Vec<Option<Vec<GitOid>>>, StorageError>;
```

**写入侧早就是 git 原生的，缺的是整个读出侧。** 每一次工具浪前 `wave_pre` 都落一个真 commit，
而仓库里没有任何一处读得出两个 commit 之间变了什么。人要的是「这个 agent 动过哪些文件」，
而那个事实已经在盘上。

**它天然只含写域。** `stage_scopes` 为写域的**每一个**前缀各暂存 `<prefix>/*`，所以两个检查点之间的差异不可能包含
会话只读过的文件——其他 harness 正在为这件事头痛（一个会话的 diff 把读过的文件也算进去），
而这个设计因为检查点就是写域而白得。

**`Lines` 是穷举枚而不是两个数。** 二进制文件没有行数，把它画成 `+0 −0` 是界面在说假话；
同理 `How::Renamed` 与「删一个加一个」是两件事。

**只算数，不搬补丁文本。** `scan_staged` 拒绝回显命中的字节（「回显字节本身即泄漏」），
而补丁文本就是文件内容过 socket——同一个出口问题。路径与计数是一个查询；单文件的 hunk
得是另一个显式查询，并且必须过同一道 `kernel::secret::scan`（§8-19）。

**不缓存。** 两个 oid 都不可变，所以结果可以永久缓存（`digest_cache` 是现成先例）；
但先测再调，未测到慢之前多一张表就是多一份要同步的状态。

`wave_pre` 不写 `HEAD`，而是写一个
dangling commit 并由 `refs/sprawling/runs/<run>/<oid>` 指住（见 8-8）。本模块只读不写，
从 oid 工作。
-/
