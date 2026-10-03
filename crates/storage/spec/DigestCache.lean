-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::digest_cache

规定 `digest_cache`（`crates/storage/src/` 下同名的文件）。内容哈希→摘要 Artifact；同哈希终生一次。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-11 storage::digest_cache（形状 4）

```rust
pub struct DigestCache { /* dir —— 私有；文件名＝内容哈希 hex64.json */ }
impl DigestCache {
    pub fn open(dir: &Path) -> Result<DigestCache, StorageError>;
    /// Same content hash digests once for life: a second put with the same
    /// hash is a no-op returning the stored artifact.
    pub fn put(&mut self, content: &B3Hash, tree_json: &[u8]) -> Result<(), StorageError>;
    pub fn get(&self, content: &B3Hash) -> Result<Option<Vec<u8>>, StorageError>;
    /// Invalidation produces the digest_invalidated payload; the entry is
    /// removed so the next digest re-runs.
    pub fn invalidate(&mut self, content: &B3Hash, reason: &str) -> Result<Payload, StorageError>;
}
```

- 本模块只交存储面。今天它没有生产调用方：runtime 里没有 digest 模块，`DigestCache` 只在本文件旁的测试里被调；接上一个读者还是删掉本模块，是 `crates/storage/Spec.lean` §3 第 10 条的待决项。
- 写入经 tmp＋rename（复用 cas 的 Vfs 纪律）：`open` 先清掉 `tmp/` 里上一进程留下的半写条目，`put` 写 `tmp/<hash>.part`、`sync_data`、再改名成 `<hash>.json`。
- `Vfs::append` 是**追加**，残留 `.part` 未清即发布出「残骸＋新内容」的拼接体；修法取 cas 既有两层纪律（open 清扫 tmp 残骸＋put 前 `truncate(0)`）而非另立新机制。`invalidate` 对不存在项不报错（末态即调用者所求），载荷携 `existed` 实报。
-/
