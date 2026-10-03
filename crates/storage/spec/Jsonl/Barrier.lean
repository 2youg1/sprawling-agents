-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 账本的回答与重开后磁盘上的内容

规定 `crates/storage/src/jsonl/barrier.rs`：`append_all` 在一波之前查询、在该波最后一次 sync 之后修补的状态 `Barrier`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪条性质」的权威（§8-1）。

磁盘是一列格子：一格要么是一条带 seq 的完整记录，要么是一波写到一半中断时留下的字节。开账本时保留到第一个不是完整记录的格子为止，这就是尾部恢复。一个 handle 记着一个位置，以及它答过 `Ok` 的那些 seq。

一条性质：**handle 答过 `Ok` 的每个 seq，重开后的账本里都有**。它成立，是因为一个有一波失败过的 handle 拒绝此后的每一波，直到退回（`jsonl::unwind`）把磁盘截回到它的位置；退回失败时句柄照旧拒绝。`withoutBarrier` 给出 handle 不守屏障、继续写时打破这条性质的轨迹。

这里一波就是一条记录。一波带几条记录，不改变撕裂相对于此前已答记录能落在哪里。
-/

/-!
### 8-1 storage::jsonl：屏障状态

**屏障状态 `jsonl::barrier`（形状 2 值）**：内存里的位置（`seg_len`、`next_seq`、`prev`）必须就是段的末尾，账本答出的 `Ok` 才能都在重开后的账本里找到。

```rust
pub(crate) enum Barrier { Whole, Broken }
impl Barrier {
    /// Whole ⇒ Ok；Broken ⇒ StorageError::LedgerBroken { dir, at: next_seq }。
    pub(crate) fn admit(&self, dir: &Path, at: Seq) -> Result<(), StorageError>;
}
```

- 一波在写出第一个字节之前把屏障置为 `Broken`，在它触及的每一段都 sync 完（以及新段的 `sync_dir`）之后才置回 `Whole`；`append_all` 进门先完成上一波留下的退回，再 `admit`，空波也一样。
- 置回 `Whole` 的另一条路是退回（`jsonl::unwind`）：删掉失败那一波新建的段、把当前段截回 `seg_len`，于是段又恰好结束在内存位置上。模型里这是 `Step.unwind`：成功时磁盘变成位置之下的那些记录，失败时什么都不变。
- 原因：写到一半死掉的波会在段尾留下位置不认识的字节——撕裂的半行，或 sync 失败的整行。在它后面再写的一波，会接在下次 open 要截掉的那段字节之后，它的 `Ok` 就说了一条盘上没有的记录（`barrier.rs` 的 `no_append_after_a_failed_barrier_claims_a_record_the_disk_loses`）。句柄不知道段尾有什么，所以它不去猜：要么把段截回自己知道的位置（退回），要么拒绝之后的每一波，留给 open 判段尾。
- 性质「句柄答过 `Ok` 的每个 seq 重开后都在」由 `crates/storage/spec/Jsonl/Barrier.lean` 对任意一串波证明（`answered_survives_reopen`），并给出不守屏障时的反例（`withoutBarrier`）；`just models` 证明它。
- 被否：失败后把位置退回或前推到盘上真实的末尾。写失败时句柄不知道落下了多少字节，sync 失败后页缓存里的字节是否还会落盘也不知道；猜一个位置，就是用猜测替 open 的断尾恢复作答。
- 重开参数：出现后台组提交（记账线程发布「已持久到 seq N」的水位线）之后，sync 失败不再发生在 `append_all` 里，屏障状态随水位线一起搬到记账线程。
-/

namespace Storage.Jsonl.Barrier

/-- 段里一个位置上放着什么。 -/
inductive Cell where
  | record (seq : Nat)
  | torn
  deriving DecidableEq, Repr

def Cell.isRecord : Cell → Bool
  | .record _ => true
  | .torn => false

/-- 尾部恢复：保留到第一个不是记录的格子为止。 -/
def reopen (disk : List Cell) : List Cell :=
  disk.takeWhile Cell.isRecord

inductive Barrier where
  | whole
  | broken
  deriving DecidableEq, Repr

structure Ledger where
  disk : List Cell
  next : Nat
  barrier : Barrier
  claimed : List Nat

def Ledger.empty : Ledger := ⟨[], 0, .whole, []⟩

/-- 一波在设备上的结局。 -/
inductive Outcome where
  /-- 写完并 sync 过：这一波答 `Ok`。 -/
  | synced
  /-- 写到一半中断，留下不成记录的字节。 -/
  | tore
  /-- 记录已进文件，但 sync 失败。 -/
  | unsynced

/-- 守屏障的 handle 写一波。 -/
def Ledger.append (l : Ledger) : Outcome → Ledger
  | o => match l.barrier with
    | .broken => l
    | .whole => match o with
      | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
      | .tore => { l with disk := l.disk ++ [.torn], barrier := .broken }
      | .unsynced => { l with disk := l.disk ++ [.record l.next], barrier := .broken }

/-- 依次排好的记录 `0 .. n`：一块完好的磁盘就是这样。 -/
def records (n : Nat) : List Cell :=
  (List.range n).map Cell.record

/-- 每个可达的 handle 都守着两件事：`Barrier` 为 `whole` 时，磁盘恰好是位置之下的那些记录；每个答过的 seq 都在位置之下，并且重开后还在。 -/
def Ledger.Sound (l : Ledger) : Prop :=
  (l.barrier = .whole → l.disk = records l.next) ∧
  (∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk) ∧
  ∀ s ∈ l.claimed, s < l.next

theorem takeWhile_records_append (n : Nat) (rest : List Cell) :
    (records n ++ rest).takeWhile Cell.isRecord
      = records n ++ rest.takeWhile Cell.isRecord := by
  induction n generalizing rest with
  | zero => simp [records]
  | succ k ih =>
    have split : records (k + 1) = records k ++ [Cell.record k] := by
      simp [records, List.range_succ]
    rw [split, List.append_assoc, ih]
    simp [List.takeWhile_cons, Cell.isRecord]

theorem reopen_records (n : Nat) : reopen (records n) = records n := by
  have h := takeWhile_records_append n []
  simpa [reopen] using h

theorem mem_records {s n : Nat} : Cell.record s ∈ records n ↔ s < n := by
  simp [records]

theorem empty_sound : Ledger.empty.Sound := by
  refine ⟨fun _ => rfl, ?_, ?_⟩ <;> simp [Ledger.empty]

/-- 无论这一波结局如何，守得住的 handle 写完之后仍守得住。 -/
theorem append_sound (l : Ledger) (o : Outcome) (h : l.Sound) : (l.append o).Sound := by
  obtain ⟨shape, kept, below⟩ := h
  cases hb : l.barrier with
  | broken =>
    simp only [Ledger.append, hb]
    exact ⟨shape, kept, below⟩
  | whole =>
    have disk := shape hb
    cases o with
    | synced =>
      have grown : l.disk ++ [Cell.record l.next] = records (l.next + 1) := by
        rw [disk]; simp [records, List.range_succ]
      refine ⟨fun _ => by simp [Ledger.append, hb, grown], ?_, ?_⟩
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        rw [grown, reopen_records, mem_records]
        simp only [List.mem_append, List.mem_singleton] at hs
        rcases hs with hs | hs
        · exact Nat.lt_succ_of_lt (below s hs)
        · omega
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        simp only [List.mem_append, List.mem_singleton] at hs
        rcases hs with hs | hs
        · exact Nat.lt_succ_of_lt (below s hs)
        · omega
    | tore =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_, ?_⟩
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        rw [disk, reopen, takeWhile_records_append]
        simp [mem_records, below s hs]
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        exact below s hs
    | unsynced =>
      refine ⟨fun hw => by simp [Ledger.append, hb] at hw, ?_, ?_⟩
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        rw [disk, reopen, takeWhile_records_append]
        simp [mem_records, below s hs]
      · intro s hs
        simp only [Ledger.append, hb] at hs ⊢
        exact below s hs

/-- 退回在设备上的结局：删段与截断都做成了，或者有一步被拒。 -/
inductive Restore where
  | done
  | refused

/-- 退回（`jsonl::unwind`）：做成时磁盘恰好是位置之下的记录，屏障修好；被拒时什么都不变，屏障照旧是坏的。位置与答过的 seq 都不动。 -/
def Ledger.unwind (l : Ledger) : Restore → Ledger
  | .done => { l with disk := records l.next, barrier := .whole }
  | .refused => l

/-- 退回保住同一个不变式：截回的正是答过的那些记录。 -/
theorem unwind_sound (l : Ledger) (r : Restore) (h : l.Sound) : (l.unwind r).Sound := by
  obtain ⟨shape, kept, below⟩ := h
  cases r with
  | refused => exact ⟨shape, kept, below⟩
  | done =>
    refine ⟨fun _ => rfl, ?_, below⟩
    intro s hs
    simp only [Ledger.unwind] at hs ⊢
    rw [reopen_records, mem_records]
    exact below s hs

/-- 句柄上的一步：写一波，或者完成上一波留下的退回。 -/
inductive Step where
  | wave (o : Outcome)
  | unwind (r : Restore)

def Ledger.step (l : Ledger) : Step → Ledger
  | .wave o => l.append o
  | .unwind r => l.unwind r

theorem step_sound (l : Ledger) (s : Step) (h : l.Sound) : (l.step s).Sound := by
  cases s with
  | wave o => exact append_sound l o h
  | unwind r => exact unwind_sound l r h

/-- 从空账本起，任意一串波与退回之后，答过的每个 seq 在重开后的磁盘上都还在。 -/
theorem answered_survives_reopen (steps : List Step) :
    (steps.foldl Ledger.step Ledger.empty).Sound := by
  suffices ∀ l : Ledger, l.Sound → (steps.foldl Ledger.step l).Sound from
    this _ empty_sound
  induction steps with
  | nil => exact fun _ h => h
  | cons s rest ih => exact fun l h => ih _ (step_sound l s h)

/-- 不守屏障的 handle：一波失败后位置不动，下一波照写。 -/
def Ledger.appendAnyway (l : Ledger) : Outcome → Ledger
  | .synced => ⟨l.disk ++ [.record l.next], l.next + 1, .whole, l.claimed ++ [l.next]⟩
  | .tore => { l with disk := l.disk ++ [.torn] }
  | .unsynced => { l with disk := l.disk ++ [.record l.next] }

/-- `Barrier` 要挡住的反例：一波撕裂，下一波答了 `Ok`，尾部恢复却把它截掉。 -/
theorem withoutBarrier :
    let l := [Outcome.synced, .tore, .synced].foldl Ledger.appendAnyway Ledger.empty
    ¬ ∀ s ∈ l.claimed, Cell.record s ∈ reopen l.disk := by
  decide

/-! ## 从已验证前缀起重开

一段的前 `L` 个格子已由记录证明是完整记录（§8-30）时，尾部恢复不必从第 0 格看起：前缀原样保留，只看 `L` 之后。记录只写在完整记录组成的前缀上，这是 `verified` 这个前提；Rust 一侧由 `storage::verified_prefix` 只为核对过的完整行写记录守住它。 -/

/-- 前 `L` 格都是记录时，重开等于前缀加上从 `L` 起的重开。 -/
theorem reopenFromVerifiedPrefix (disk : List Cell) (L : Nat)
    (verified : (disk.take L).all Cell.isRecord = true) :
    reopen disk = disk.take L ++ reopen (disk.drop L) := by
  unfold reopen
  induction L generalizing disk with
  | zero => simp
  | succ k ih =>
    cases disk with
    | nil => simp
    | cons c rest =>
      simp only [List.take_succ_cons, List.all_cons, Bool.and_eq_true] at verified
      obtain ⟨hc, hrest⟩ := verified
      simp only [List.takeWhile_cons, hc, if_true, List.take_succ_cons, List.drop_succ_cons,
        List.cons_append]
      rw [ih rest hrest]

/-- 于是守屏障的 handle 答过的每个 seq，从任何已验证前缀起重开之后也都还在。 -/
theorem answered_survives_reopen_from_a_verified_prefix (steps : List Step) (L : Nat)
    (verified :
      ((steps.foldl Ledger.step Ledger.empty).disk.take L).all Cell.isRecord = true) :
    ∀ s ∈ (steps.foldl Ledger.step Ledger.empty).claimed,
      Cell.record s ∈ (steps.foldl Ledger.step Ledger.empty).disk.take L
        ++ reopen ((steps.foldl Ledger.step Ledger.empty).disk.drop L) := by
  intro s hs
  rw [← reopenFromVerifiedPrefix _ L verified]
  exact (answered_survives_reopen steps).2.1 s hs

end Storage.Jsonl.Barrier

/-! D16 `LedgerBroken`→`E_STORAGE_FATAL`

`LedgerBroken`→`E_STORAGE_FATAL`：不可定义掉——写与 sync 的失败来自介质；能定义掉的那部分（失败之后再写的一波被下次 open 截掉，却已答了 `Ok`）已由 `Barrier` 定义掉。recovery 是修好盘之后再写：下一波进门先完成退回，退回做成即照常写；退回仍被拒时，重启，由 open 修段尾。
-/

/-! D24 屏障本身按平台各有一臂，每一臂守同一条持久语义

**每一臂必须守的不变式**：`append_all` 答 `Ok` 时，这一波的每个字节在掉电之后仍在介质上，与今天 `Vfs::sync_data` 之后的保证相同；一臂若只保证「离开了进程」或「进了磁盘的易失缓存」，它就是一道更弱的屏障，不能入选（D94）。屏障之后才把 `Barrier` 置回 `whole`、才答 `Ok`，这一点对每一臂都不变，所以上面的 `answered_survives_reopen` 对每一臂成立；预分配的那一臂另须守 `crates/storage/spec/Jsonl/Preallocate.lean` 的三条性质（段尾零不改变扫描结论、撕裂照样被截、中间的零读作撕裂），并且写者的位置取剥零之后的长度。

**今天的臂**：三个平台都走 `std::fs::File::sync_data`（`crates/storage/src/real_fs.rs` 的 `Vfs::sync_data`）。标准库在三个平台上把它落到（读自 Rust 1.97.1 的 `library/std/src/sys/fs/unix.rs` 与 `library/std/src/sys/fs/windows.rs`，`File::datasync`）：
* Windows：`datasync` 调 `fsync`，即 `FlushFileBuffers`；
* Linux：`fdatasync`；
* macOS（`target_vendor = "apple"`）：`fcntl(fd, F_FULLFSYNC)`，不是普通的 `fsync`——所以今天在 macOS 上 `sync_data` 已经要求磁盘清空它的写缓存，不是一道更弱的屏障。

**候选臂**（TF2 在 W5 按测量选，每个平台选最快且守住上面不变式的一臂，或保留今天的臂并写明理由）：
* Windows：`FlushFileBuffers`（今天）；`FILE_FLAG_WRITE_THROUGH`，经 `std::os::windows::fs::OpenOptionsExt::custom_flags` 这个安全接口打开段文件，每次写直达介质，屏障本身变成空操作；两者都可加段文件预分配（`File::set_len`）。
* Linux：`fdatasync`（今天）；`O_DSYNC`，经 `std::os::unix::fs::OpenOptionsExt::custom_flags`，每次写带数据同步；两者都可加预分配，预分配让 `fdatasync` 不必再写文件长度这条元数据。
* macOS：`F_FULLFSYNC`（今天）；`F_BARRIERFSYNC` 只给次序不给持久，是更弱的屏障，不入选；预分配可选。

**实现**：写直达两臂是 `crates/storage/src/real_fs.rs` 的 `SegmentDurability::WriteThrough`，每个平台的选择是一个常量 `SEGMENT_DURABILITY`，今天三处都是 `SyncData`；写直达之后屏障照发，所以上面的不变式不随臂变。读源码的结论在 1.97.1 上成立；`rust-toolchain.toml` 现在钉的是 1.99.0，那一版的 `library/std/src/sys/fs/unix.rs` 与 `windows.rs` 的 `File::datasync` 还要重读一遍，读法同上。预分配那一臂要求写者按位置写（`Vfs` 今天只有写在文件末尾的 `append`），所以还没有实现；读者一侧已经守着 `crates/storage/spec/Jsonl/Preallocate.lean` 的三条性质。

**落选**：一个平台用一臂、其余平台跟着它——三个平台的系统调用与它们的持久语义各不相同，同一个名字在三处是三件事。重新打开它的参数：标准库改变 `File::sync_data` 在某个平台上落到的系统调用（W5 的实现者先在 `rust-toolchain.toml` 钉住的版本上重读上面两个源文件），或某一臂的测量在 p99 上不再胜出。
-/
