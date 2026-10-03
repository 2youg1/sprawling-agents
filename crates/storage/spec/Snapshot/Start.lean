-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::snapshot::start

规定 `snapshot::start`（`crates/storage/src/` 下同名的文件）。开城从快照起步还是从创世起步，以及为什么。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
-/

/-!
### 8-28 `storage::snapshot::start`：从快照起步还是从创世起步（形状 7 投影：决定折叠从哪一行起）

```rust
pub enum SnapshotStart {
    Resume { snapshot: ChainSnapshot, tail: Vec<Vec<u8>> },   // 快照合身：它的 views，再折 seq 之后的这些行
    Whole(WholeFold),                                          // 从创世折，并说明为什么没用快照；行不在这里
}
pub enum WholeFold { NoSnapshot, Damaged(String), OtherFoldVersion { found: u32 }, Stale, Missing }
pub fn start_from_snapshot(ledger_dir: &Path, snapshot_dir: &Path, fold_version: u32) -> Result<SnapshotStart, StorageError>;
```

- **一次读既核对又取尾部。** 快照的 `seq` 按段名（`segment_first_seq`）定位到含它的那一段：它之前的段一个字节也不读；这一段从段尾往回找：末行自己写着的 `seq` 减去快照的 `seq`，就是往回数的行数，数到的那一行交给 `ChainSnapshot::fit`，它之后的行（连同之后各段的完整行）就是尾部。末行读不出 `seq`、比快照的 `seq` 小、或这一段没有那么多行时，从段首数第 `seq - 段首 seq` 行，结果与往回找相同或给出 `Missing`。往回找只扫尾部的字节，不把整段切成行（40 万行夹具城的末段 40 MB，从段首切要 35 ms，`crates/sprawling/Spec.lean` §8-144），也不让快照那一行之前同一段里的损坏改变起点：那些行与更早的段一样，归 `prove_chain` 核对（8-30）。所以起步的读量是「尾部加一段的前缀」，与账本总长无关。
- **不能用快照时一律退回全量折叠，并带上原因。** 没有快照（`NoSnapshot`）、字节不是快照（`Damaged`，原样带出 8-26 的原因）、`fold_version` 不等（`OtherFoldVersion`）、那一行的链哈希不同（`Stale`）、账本里根本没有那一行（`Missing`，账本比快照短）——都是 `Whole`。`Whole` 只带原因：从创世折叠的调用方经 `runtime::replay::fold_ledger_dir` 一段一段地读、每行过 `LineCheck`（`crates/runtime/Spec.lean` §8-1），常驻的是一段字节与一条记录，与没有快照时的起步完全相同。原因给调用方写诊断用；它们都不是错误，因为快照只是投影。I/O 本身失败才是 `StorageError`。
- **被否：`Whole` 带回账本的全部行。** 调用方不读它们：它要的是流式折叠，那些行一到手就被丢掉，账本却已经整本进过一次内存，峰值随历史长度增长。
- **被否：在快照里再存该行的字节偏移，做真正的单次 `pread`。** 偏移指向的是字节，不是链：换过的账本在同一偏移可能正好有一行，核对仍要看链哈希；而尾部本来就要从那一段读，省下的只是一段内的前缀扫描，却让 8-26 的格式多一个会随段滚动失效的字段。
-/
