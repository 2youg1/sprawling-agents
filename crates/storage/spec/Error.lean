-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::error

规定 `error`（`crates/storage/src/` 下同名的文件）。`StorageError` 与 `into_ax`：本 crate 失败词汇的唯一定义点，以及每个变体答哪个码。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-14 storage::error（形状 2 值）

```rust
pub enum StorageError {                      // thiserror；crate 根
    Io { op: &'static str, path: PathBuf, source: io::Error },
    VersionAhead { path: PathBuf, v: u64 }, // 方向感知拒绝的机器面
    Envelope { path: PathBuf, line: u64, source: AxError },  // 段中损坏（断尾候选之外）
    Draft { source: AxError },              // 组 log_truncated 草稿失败（不以死变体粉饰）
    CasMissing / CasCorrupt / RangeOutOfBounds,               // cas
    SeqMissing,                                              // index
    Checkpoint / SecretEgress / Bundle,                      // 各自的模块
    Worktree / WorktreeBusy / MergeStale / MergeWouldDiscard, // worktree
    Alias { op: &'static str, path: PathBuf, kind: alias::AliasKind },  // → E_OUTSIDE_WRITE_DOMAIN（8-25）
    LedgerHeld { dir: PathBuf },            // 另一个 JsonlLedger 持着这座城的账本（8-1）→ E_LEDGER_HELD
    LedgerBroken { dir: PathBuf, at: Seq }, // 一波的写或 sync 失败过，重开前拒绝之后每一波（8-1）→ E_STORAGE_FATAL
    ChainHalted { source: AxError },        // 全链审计发现断链，写者停止接新行（8-27）；码取审计自己的 source
    Snapshot { op: &'static str, path: PathBuf, source: io::Error },  // 快照读写被盘拒绝（8-26）→ E_STORAGE_FATAL，恢复说的是快照
}
impl StorageError { pub fn into_ax(self) -> AxError; }   // 跨 crate 边界的唯一出口
pub(crate) fn io_err(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> StorageError;
```

- **为什么是独立模块**：见 §7，全 crate 的模块都用它。
- **为什么带着 `io_err` 走**：它是 `StorageError::Io` 的构造子，而一个值的构造子与它的定义同住。四个模块（cas／bundle／digest_cache／index）只为取它而 import jsonl，那是一条指错了方向的依赖。
- **快照的 I/O 失败有自己的变体**：`Io` 的恢复建议说的是账本（停机，重开会截掉撕裂的尾巴），对快照是错的——快照是账本随时能重建的缓存。`Snapshot` 与 `Io` 同码（盘拒绝了写，多半账本也写不进），恢复则说：删掉这份快照、腾出盘，下次启动从创世折叠。被否：让 `Io` 的恢复按 `op` 分支——一个变体两种建议，读恢复的人得先知道 `op` 的全集。
- **公开名是 `storage::StorageError`**（`lib.rs` 重导出）：模块住处不进公共拼写，下游的引用不随它动。
-/

/-!
## 每个变体答哪个码

`StorageError::into_ax` 是本 crate 失败越过 crate 边界的唯一出口。下面的 `code` 列出每个变体答的 `AxCode`；`Draft` 与 `ChainHalted` 不造码，原样交出它们带着的那个 `AxError`（`carried`）。码本身（拼写、语义、装载期白名单）的权威是 kernel 的错误码表（`crates/kernel/Spec.lean` §8-4），这里只说 storage 的哪个失败落在哪个码上，以及为什么它不能被定义掉。

`ledger_failures_stop_the_writer` 陈述：账本介质的失败（`Io`、`LedgerBroken`）与快照的盘拒绝都答 `E_STORAGE_FATAL`，宁停不脏；`only_two_variants_carry` 陈述只有 `Draft` 与 `ChainHalted` 不造码，所以别的变体的码都在这张表里写死。
-/

namespace Storage.Error

/-- `StorageError` 的变体，拼写与 Rust 相同，去掉各自携带的字段。 -/
inductive StorageError where
  | Io
  | VersionAhead
  | Envelope
  | Draft
  | CasMissing
  | CasCorrupt
  | RangeOutOfBounds
  | SeqMissing
  | Checkpoint
  | SecretEgress
  | Bundle
  | Worktree
  | WorktreeBusy
  | MergeStale
  | MergeWouldDiscard
  | Alias
  | NameTaken
  | LedgerHeld
  | LedgerBroken
  | ChainHalted
  | Unproven
  | Snapshot
  deriving DecidableEq, Repr

/-- storage 答出的 kernel `AxCode`，拼写与 Rust 的变体相同。 -/
inductive AxCode where
  | StorageFatal
  | LogVersionUnsupported
  | HistoryUnproven
  | WorktreeBusy
  | VersionConflict
  | PathNotFound
  | CasCorrupt
  | InvalidArgs
  | ConfigInvalid
  | SecretEgress
  | LedgerHeld
  | OutsideWriteDomain
  deriving DecidableEq, Repr

/-- 一个变体越过边界时的码：自己的码，或原样交出它带着的那个错误的码。 -/
inductive Answer where
  | own (code : AxCode)
  | carried
  deriving DecidableEq, Repr

/-! D3 `VersionAhead`→`E_LOG_VERSION_UNSUPPORTED`

`VersionAhead`→`E_LOG_VERSION_UNSUPPORTED`：不可定义掉——二进制升级与数据寿命天然错位；方向感知拒绝即其最小语义。
-/

/-! D4 `CasCorrupt`→`E_CAS_CORRUPT`

`CasCorrupt`→`E_CAS_CORRUPT`：不可定义掉——位腐烂与外部改动在本设计边界外；能定义掉的部分（写路径半成品）已由 tmp+rename 定义掉。
-/

/-! D5 `CasMissing`→`E_PATH_NOT_FOUND`

`CasMissing`→`E_PATH_NOT_FOUND`：不可定义掉——Locator 是跨会话引用，对象可被更早的介质事故清除；nearby 给同前缀既存对象。
-/

/-! D6 `RangeOutOfBounds`→`E_INVALID_ARGS`

`RangeOutOfBounds`→`E_INVALID_ARGS`：可部分定义掉——`Range` 构造已保 `from<=to`；对象长度只在读时可知，读时校验是剩余的不可消部分。
-/

/-! D7 `Io`→`E_STORAGE_FATAL`

`Io`→`E_STORAGE_FATAL`（宁停不脏路径；不可定义掉——介质失败在设计边界外）。
-/

/-! D8 `WorktreeBusy`→`E_WORKTREE_BUSY`

`WorktreeBusy`→`E_WORKTREE_BUSY`：可定义掉但尚未做——当「领节点」本身变成取租约（`storage::queue` 已有队列），busy 就从错误变成排队。它同时承担「该节点的树被占」与「再开一棵就越上限」两个情形：两者的可执行替代同为「先归还一棵」，而区分它们的是 subject 不是码。
-/

/-! D9 `MergeStale`→`E_VERSION_CONFLICT`

`MergeStale`→`E_VERSION_CONFLICT`：不可定义掉——两个节点同时开工就会有一个后到；能定义掉的那部分（“合到一半失败”）已由 fast-forward 判定在动手之前定义掉。
-/

/-! D10 `MergeWouldDiscard`→`E_VERSION_CONFLICT`

`MergeWouldDiscard`→`E_VERSION_CONFLICT`：不可定义掉——人的未提交改动在城市目录里，机器无权决定它与节点的活谁留下；能定义掉的「静默覆盖」已由 SAFE 检出定义掉。
-/

/-! D11 `Worktree`→`E_STORAGE_FATAL`

`Worktree`→`E_STORAGE_FATAL`：不可定义掉——仓库与文件系统是外部世界；能定义掉的那部分（名字走出目录）已由 `WorktreeName` 在构造点定义掉。
-/

/-! D12 `Alias`→`E_OUTSIDE_WRITE_DOMAIN`

`Alias`→`E_OUTSIDE_WRITE_DOMAIN`：不可定义掉——名字与它指向的文件之间隔着一个链接是外部文件系统的事实；能定义掉的那部分（一次写入经链接穿透）已由 `WriteTarget` 在构造点定义掉，recovery 恒为「换成普通文件后重试」，故被拒的 run 不会卡死。硬链接臂在两个平台上都在这个码下（链接计数大于 1 即拒，§3.5）。
-/

/-! D17 `Envelope`→`E_LOG_VERSION_UNSUPPORTED` 同族拒读

`Envelope`→`E_LOG_VERSION_UNSUPPORTED` 同族拒读（段中损坏非尾部＝不可自动修复，指出路径交人决定）。
-/

/-- `StorageError::into_ax` 的码。`Checkpoint` 答 `E_WORKTREE_BUSY`（恢复：理顺仓库的状态，再跑这一波），`Bundle` 答 `E_CONFIG_INVALID`（I/O 本身无误、得到的却不是一座城：目标目录被占、清单与到达的东西不符、链有缺口；恢复：恢复进空目录，bundle 短了就再拷一次），`NameTaken` 答 `E_VERSION_CONFLICT`（§8-32），`LedgerHeld` 见 storage D18，`LedgerBroken` 见 storage D16，`Unproven` 见 storage D19。 -/
def code : StorageError → Answer
  | .Io => .own .StorageFatal
  | .VersionAhead => .own .LogVersionUnsupported
  | .Envelope => .own .LogVersionUnsupported
  | .Draft => .carried
  | .CasMissing => .own .PathNotFound
  | .CasCorrupt => .own .CasCorrupt
  | .RangeOutOfBounds => .own .InvalidArgs
  | .SeqMissing => .own .InvalidArgs
  | .Checkpoint => .own .WorktreeBusy
  | .SecretEgress => .own .SecretEgress
  | .Bundle => .own .ConfigInvalid
  | .Worktree => .own .StorageFatal
  | .WorktreeBusy => .own .WorktreeBusy
  | .MergeStale => .own .VersionConflict
  | .MergeWouldDiscard => .own .VersionConflict
  | .Alias => .own .OutsideWriteDomain
  | .NameTaken => .own .VersionConflict
  | .LedgerHeld => .own .LedgerHeld
  | .LedgerBroken => .own .StorageFatal
  | .ChainHalted => .carried
  | .Unproven => .own .HistoryUnproven
  | .Snapshot => .own .StorageFatal

theorem ledger_failures_stop_the_writer :
    code .Io = .own .StorageFatal ∧ code .LedgerBroken = .own .StorageFatal ∧
      code .Snapshot = .own .StorageFatal := by
  decide

theorem only_two_variants_carry (e : StorageError) :
    code e = .carried ↔ e = .Draft ∨ e = .ChainHalted := by
  cases e <;> decide

end Storage.Error
