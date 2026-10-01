-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::digest_cache

规定 `digest_cache`（`crates/storage/src/` 下同名的文件）。内容哈希→摘要 Artifact；同哈希终生一次。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
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

- 消费者是 runtime::digest；本模块只交存储面。写入经 tmp＋rename（复用 cas 的 Vfs 纪律）。
-/
