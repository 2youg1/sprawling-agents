-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::error

规定 `error`（`crates/storage/src/` 下同名的文件）。`StorageError` 与 `into_ax`：本 crate 失败词汇的唯一定义点，以及每个变体答哪个码。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
-/

/-!
## 每个变体答哪个码

`StorageError::into_ax` 是本 crate 失败越过 crate 边界的唯一出口。下面的 `code` 列出每个变体答的 `AxCode`；`Draft` 与 `ChainHalted` 不造码，原样交出它们带着的那个 `AxError`（`carried`）。码本身（拼写、语义、装载期白名单）的权威是 kernel 的错误码表（kernel-SPEC §8-4），这里只说 storage 的哪个失败落在哪个码上，以及为什么它不能被定义掉。

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
