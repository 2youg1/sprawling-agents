-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::blob

规定 `blob`（`crates/storage/src/` 下同名的文件）。一次提交里一个文件的字节，从不读工作树。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-29 `storage::blob`：一次提交里一个文件的字节（形状 4 adapter）

```rust
pub fn blob_at(city_root: &Path, oid: GitOid, addr: &Address) -> Result<Option<Vec<u8>>, StorageError>;
```

- 读城仓库里 `oid` 那次提交的树上 `addr` 处的 blob；那里不是文件（目录、子模块、不存在）＝`Ok(None)`，由调用方说出拒因——它知道是谁问的。仓库打不开、提交找不到＝`StorageError::Checkpoint`。
- 不碰工作区与索引：`file:<addr>@<oid>` 指的是提交里的字节，工作区此刻的文件可能已经改过。
-/
