-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::checkpoint 的并发写者

规定 `crates/storage/src/checkpoint.rs`、`crates/storage/src/checkpoint/base.rs` 与 `crates/storage/src/checkpoint/commit.rs` 在多个写者同时写一座城的检查点时必须守住的性质。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（`crates/storage/Spec.lean` §8-8、§8-39）。

一次检查点分三步：写者把自己的写域暂存进**自己的** index（或不经盘上 index，直接在 mempack 里写树对象），把树对象写进城共享的对象库，再把引用 `refs/sprawling/runs/<run>/<oid>` 钉到它上面。对象库按内容寻址、只增不减；检查点引用以提交自己的 oid 为名，只建不改；唯一会被改写的引用是 HEAD（`ensure_base` 在没有 HEAD 时建它，`land` 移动它），它的改写是一次比较后交换。

模型的性质分五组，每组一串定理，全部对任意交错的步骤序列成立：
* **谁也不碰别人的 index**——别的写者的任何一串步骤之后，一个写者的 index 原样不动；它钉下的恰是它暂存的那棵树（`others_never_touch_an_index`、`a_pin_holds_what_its_writer_staged`）；
* **不同写者的步骤可交换**——两个写者相邻的两步换个次序，城一样；所以任何交错都等于某个串行次序，不需要一把全城的锁（`independent_steps_commute`）；
* **引用更新的冲突被发现，恢复后等于一个串行次序**——HEAD 的比较后交换只在读到的值仍在时成功，两个写者从同一个读数出发至多一个改得动；`ensure_base` 输了之后重读，结果等于赢家先做、输家后做（`a_swap_moves_only_what_it_read`、`no_update_is_lost`、`a_lost_base_race_is_the_serial_order`）；
* **失败的检查点不动已有的引用**——被拒的一步什么也不改，钉引用之前的每一步都不改引用，钉下的引用只增不减（`a_refused_step_changes_nothing`、`before_the_pin_no_reference_moves`、`pins_only_grow`）；
* **崩溃的写者只留下垃圾**——每个引用都指向对象库里的对象，任何一步都保持这一点；崩溃不让任何新对象变得可达（`references_stay_inside_the_store`、`a_crash_reaches_nothing_new`）。

另一组性质管增量检查点：edit 与 write 知道自己写了哪些路径，只暂存这些路径；只要这一波在写域里只碰了这些路径，得到的树与暂存整个写域相同（`staging_the_touched_paths_is_staging_the_scope`）。exec 不知道写了什么，按修改时间与尺寸筛出的路径是同一个定理的另一个实例：筛子的前提是「修改时间与尺寸都没变的文件内容也没变」。
-/

/-!
### 8-39 storage::checkpoint 的并发写者：每个写者一个 index，共享的只有对象库与引用（形状 4 适配器；git2）

```rust
// crates/storage/src/checkpoint/opening.rs
impl Checkpoint {
    /// 城的那一份 index 上的句柄：仓库不在时建它，并把行尾钉成原样（`core.autocrlf = false`）。
    /// 配置只在值不同时写，所以城开过一次之后，再开只读不写，不与别的句柄争配置的锁。
    pub fn open(city_root: &Path) -> Result<Checkpoint, StorageError>;
    /// 一个写者的检查点句柄：仓库与对象库是城的那一个，index 是这个写者自己的
    /// `<root>/.sprawling/index/<run>`（`git2::Index::open` 加 `Repository::set_index`）。
    /// 文件不在时从仓库的 index 复制一份，于是第一道检查点仍按 stat 跳过没变的文件，
    /// 而不是为写域里的每个文件求一次哈希。没有仓库即 `StorageError::Checkpoint`：
    /// 写者不建仓库，仓库在开城时由 `Checkpoint::open` 建一次。
    pub fn open_writer(root: &Path, writer: RunId) -> Result<Checkpoint, StorageError>;
    /// 写者的 run 落地后删掉它的 index 文件；不在即成功；`open` 开的句柄什么也不删。
    pub fn close_writer(self) -> Result<(), StorageError>;
    /// 开城时删掉 `.sprawling/index/` 下的每一个文件，答删了几个：开城时没有活的 run。
    pub fn sweep_writers(root: &Path) -> Result<usize, StorageError>;
}
```

`root` 是 run 写的那棵树：城自己，或借给房间的 worktree。worktree 本来就有自己的 index，私有 index 放在它自己的 `.sprawling/index/` 下，随 worktree 一起被收走；城的私有 index 由 `sweep_writers` 在开城时收。

- **`wave_pre` 的引用只建不冲突。** 引用名是提交自己的 oid（§8-8），建的时候 `force = true`：名字就是值，覆盖只可能写下同一个值。`force = false` 反而会让同一刻、同一棵树、同一父提交的第二次检查点（同一个 oid）被「引用已在」拒掉。所以检查点引用之间没有冲突，`pins_only_grow` 与 `independent_steps_commute` 说的就是这一点。
- **HEAD 的改写是比较后交换。** `ensure_base` 与 `land` 以 `Repository::commit(Some("HEAD"), …, parents)` 提交，libgit2 在当前 HEAD 不是第一个父提交时拒绝（无父提交时，HEAD 已在即拒）。`ensure_base` 被拒即重读 HEAD：HEAD 已在就返回 `None`，与赢家先做的串行次序相同（`a_lost_base_race_is_the_serial_order`）。`land` 被拒是 `StorageError::Checkpoint { op: "move HEAD" }`（→ `E_WORKTREE_BUSY`，恢复：重试这一次落地），因为它的树是对旧 HEAD 暂存的，静默重做会把别人刚落下的改动当成这次要退回的。引用的锁文件被别人占着（libgit2 的 `Locked`；Windows 上改名撞上一个开着的文件也报在这里）答的是同一个错误与同一个恢复。
- **失败不动引用。** 暂存与写对象都不改引用（`before_the_pin_no_reference_moves`），所以在钉引用之前失败的检查点留下的引用与开始时相同；崩溃写下的对象没有引用够得着，`git gc` 收走（`a_crash_reaches_nothing_new`）。私有 index 在崩溃后留在盘上：worker 开城时 `sweep_writers` 删掉它们，与 `sweep_abandoned` 收走崩溃留下的树同一个时机。
- **增量检查点等于整个写域的检查点。** 只暂存这一波写过的路径由 `crates/runtime/Spec.lean` §8-45 决定（lane 把各调用的 `Writes::Paths` 并起来，`Domain` 与失败的调用暂存整个写域）；它依赖的性质在这里证明：这一波在写域里只碰了这些路径时，两种暂存得到同一棵树（`staging_the_touched_paths_is_staging_the_scope`）。exec 声明 `Domain`，它的检查点暂存整个写域，libgit2 按修改时间与尺寸跳过没变的文件，是同一个定理以「stat 变了的路径」为 `touched` 的实例。
- **验收**：从本模型导出的 Rust 检查在 `crates/storage/src/checkpoint/opening/tests.rs`——两个写者各开 `open_writer`，在同一座城里的两个线程上交错 `wave_pre`，各自的提交的树等于各自暂存的写域，两条引用都在（`others_never_touch_an_index`、`a_pin_holds_what_its_writer_staged`）；一个写者暂存并写下树对象之后、提交与钉引用之前被丢掉，城的引用集合不变（`before_the_pin_no_reference_moves`、`a_crash_reaches_nothing_new`）；两个写者在两个线程上同时 `ensure_base` 一座没有提交的城，HEAD 恰是其中一个的提交，另一个答 `None`（`a_lost_base_race_is_the_serial_order`）。
-/

/-! D25 一座城的检查点由多个写者同时做，每个写者一个自己的 index，共享的只有对象库与引用更新；`accounting` 那把全城的检查点锁因此拆掉。
**为什么。** 那把锁让一座城的每一次检查点排成一列，理由是「一个仓库只有一个 index」：两条 lane 同时取 `.git/index.lock`，后者被拒。可那把锁护的是 index 这一个文件，而 index 不必共享：一个写者用自己的 index（或像 `base_checkpoint` 那样在 mempack 里直接写树），它的暂存、写树、建提交就不碰任何别的写者能看见的东西。剩下共享的两样各自是原子的：对象按内容寻址，同名即同字节，先写后写一样；检查点引用以 oid 为名，同名即同值。本模型证明这两样加上 HEAD 的比较后交换，在任何交错下都等于某个串行次序（`independent_steps_commute`、`a_lost_base_race_is_the_serial_order`）。
**被否：保留一把全城的检查点锁。** 它的代价随并发的 run 数线性涨：测试城里一个 run 独占这把锁时，写入波前的检查点已占 p50 18 ms、最大 64 ms，N 条 lane 同时有写入波时，排在最后的一条要等 N−1 次检查点。**被否：每个 run 一个仓库。** 对象不再共享，worktree 无从分枝，合并要跨仓库搬对象。
**文件系统的前提。** 对象：libgit2 先写临时文件，再改名到 `objects/<xx>/<rest>`（或一个 pack 与它的 `.idx`）；目标已在时，后来者丢掉自己的临时文件。引用：libgit2 以 `O_EXCL` 创建 `<ref>.lock` 取得排他，写完改名覆盖。这两步要的是「独占创建」与「同卷改名是原子的」：Windows 的 NTFS 上是 `CreateFileW(CREATE_NEW)` 与 `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`，macOS 的 APFS 与 Linux 的 ext4 上是 `open(O_CREAT|O_EXCL)` 与 `rename(2)`，三者都满足。不满足的地方是城放在网络盘（SMB、NFS）上：两个写者可能都以为拿到了 `<ref>.lock`。检查点引用以 oid 为名，两个写者写的是同一个值，所以仍然不坏；HEAD 的比较后交换失去保证，所以 `ensure_base` 与 `land` 还经同一进程里的一把锁，这把锁只护 HEAD，不护检查点（D26）。私有 index 在保留子树之下，不进任何树。
**重开参数。** libgit2 不再以内容寻址写对象，或检查点引用的名字不再由 oid 决定。
-/

/-! D26 移动 HEAD 的两步（`ensure_base`、`land`）在每一座城上都经同一进程里的一把只护 HEAD 的锁（`opening::HEAD_MOVES`），不按城是否在网络盘上分两条路。
**为什么。** 认出网络盘要问平台：Windows 是 `GetDriveTypeW`，macOS 与 Linux 是 `statfs` 的文件系统类型，三者都只有 `unsafe` 的 FFI 或一个新依赖，标准库没有安全接口（AGENTS.md 平台调用的次序）。这把锁护的两步一个 run 至多各走一次，从不在写入波的路上：检查点（`wave_pre`）不取它，所以它不是被拆掉的那把全城的锁。本地盘上它多余而无害，网络盘上它是同一进程里唯一的保证。
**被否：按盘的种类分路。** 多一个平台调用，少一把几乎不被争用的锁。
**重开参数。** 移动 HEAD 的步骤进了写入波，或标准库有了认出网络盘的安全接口。
-/

namespace Storage.Checkpoint.Concurrent

/-! ## 城仓库与写者 -/

/-- 一个写者：一条 lane 上的一个 run，或 harness run 的一次提交。 -/
abbrev Writer := Nat

/-- 一个对象的名字。对象按内容寻址，所以一棵树就是它的 oid。 -/
abbrev Oid := Nat

/-- 城仓库：每个写者自己的 index（暂存出的树）、共享的对象库、每个写者钉下的检查点引用（名字就是 oid，所以是一个集合），与 HEAD。 -/
structure Repo where
  index : Writer → Oid
  store : Oid → Bool
  pins : Writer → Oid → Bool
  head : Option Oid

/-- 写者能走的一步。 -/
inductive Step where
  /-- 把写域暂存进自己的 index；`tree` 是它这一刻从工作区读到的树。 -/
  | stage (by_ : Writer) (tree : Oid)
  /-- 把自己 index 的树对象写进共享的对象库。 -/
  | put (by_ : Writer)
  /-- 把引用 `refs/sprawling/runs/<run>/<oid>` 建到自己 index 的树上；对象不在库里即拒。 -/
  | pin (by_ : Writer)
  /-- HEAD 的比较后交换：HEAD 仍是 `read` 时换成自己 index 的树，否则拒。 -/
  | swap (by_ : Writer) (read : Option Oid)
  /-- 写者死掉：私有的 index 随它消失，写进库里的对象留下。 -/
  | crash (by_ : Writer)

/-- 这一步是谁走的。 -/
def Step.writer : Step → Writer
  | .stage w _ | .put w | .pin w | .swap w _ | .crash w => w

/-- 让 `w` 的 index 取 `t`，别的写者不变。 -/
def Repo.setIndex (s : Repo) (w : Writer) (t : Oid) : Repo :=
  { s with index := fun u => if u = w then t else s.index u }

/-- 这一步此刻会不会被拒。 -/
def Step.admitted (s : Repo) : Step → Bool
  | .stage _ _ | .put _ | .crash _ => true
  | .pin w => s.store (s.index w)
  | .swap w read => s.head == read && s.store (s.index w)

/-- 走一步；被拒的一步什么也不改（`a_refused_step_changes_nothing`）。 -/
def Step.apply : Step → Repo → Repo
  | .stage w t, s => s.setIndex w t
  | .put w, s => { s with store := fun o => o == s.index w || s.store o }
  | .pin w, s =>
    if s.store (s.index w) then
      { s with pins := fun u o => (u == w && o == s.index w) || s.pins u o }
    else s
  | .swap w read, s =>
    if s.head == read && s.store (s.index w) then { s with head := some (s.index w) } else s
  | .crash w, s => s.setIndex w 0

/-- 按次序走一串步骤，交错由这串的次序给出。 -/
def run (s : Repo) : List Step → Repo
  | [] => s
  | st :: rest => run (st.apply s) rest

theorem run_append (s : Repo) (xs ys : List Step) :
    run s (xs ++ ys) = run (run s xs) ys := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => exact ih (x.apply s)

/-! ## 谁也不碰别人的 index -/

theorem a_step_keeps_other_indexes (st : Step) (s : Repo) {w : Writer}
    (other : st.writer ≠ w) : (st.apply s).index w = s.index w := by
  cases st <;> simp only [Step.apply, Step.writer] at other ⊢ <;> (try split) <;>
    simp [Repo.setIndex, Ne.symm other]

theorem others_never_touch_an_index (xs : List Step) (s : Repo) {w : Writer}
    (others : ∀ st ∈ xs, st.writer ≠ w) : (run s xs).index w = s.index w := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih =>
    simp only [run]
    rw [ih (x.apply s) (fun st h => others st (List.mem_cons_of_mem x h))]
    exact a_step_keeps_other_indexes x s (others x List.mem_cons_self)

theorem a_step_keeps_the_store (st : Step) (s : Repo) {o : Oid}
    (held : s.store o = true) : (st.apply s).store o = true := by
  cases st <;> simp only [Step.apply] <;> (try split) <;> simp [Repo.setIndex, held]

theorem the_store_only_grows (xs : List Step) (s : Repo) {o : Oid}
    (held : s.store o = true) : (run s xs).store o = true := by
  induction xs generalizing s with
  | nil => exact held
  | cons x xs ih => exact ih (x.apply s) (a_step_keeps_the_store x s held)

theorem a_step_keeps_the_pins (st : Step) (s : Repo) {w : Writer} {o : Oid}
    (held : s.pins w o = true) : (st.apply s).pins w o = true := by
  cases st <;> simp only [Step.apply] <;> (try split) <;> simp [Repo.setIndex, held]

/-- 钉下的引用只增不减：别的写者怎么交错，也拿不走一个写者已经钉下的检查点。 -/
theorem pins_only_grow (xs : List Step) (s : Repo) {w : Writer} {o : Oid}
    (held : s.pins w o = true) : (run s xs).pins w o = true := by
  induction xs generalizing s with
  | nil => exact held
  | cons x xs ih => exact ih (x.apply s) (a_step_keeps_the_pins x s held)

/-- 一个写者暂存 `t`、写进库、再钉下，中间夹着别的写者的任何步骤：它钉下的恰是 `t`。 -/
theorem a_pin_holds_what_its_writer_staged (s : Repo) (w : Writer) (t : Oid)
    (xs ys : List Step) (others : ∀ st ∈ xs ++ ys, st.writer ≠ w) :
    (run s ([.stage w t] ++ xs ++ [.put w] ++ ys ++ [.pin w])).pins w t = true := by
  have hx : ∀ st ∈ xs, st.writer ≠ w := fun st h => others st (List.mem_append_left ys h)
  have hy : ∀ st ∈ ys, st.writer ≠ w := fun st h => others st (List.mem_append_right xs h)
  simp only [run_append]
  generalize hs1 : run s [.stage w t] = s1
  have i1 : s1.index w = t := by
    subst hs1; simp [run, Step.apply, Repo.setIndex]
  generalize hs2 : run s1 xs = s2
  have i2 : s2.index w = t := by rw [← hs2, others_never_touch_an_index xs s1 hx, i1]
  generalize hs3 : run s2 [.put w] = s3
  have i3 : s3.index w = t := by
    subst hs3; simp [run, Step.apply, i2]
  have st3 : s3.store t = true := by
    subst hs3; simp [run, Step.apply, i2]
  generalize hs4 : run s3 ys = s4
  have i4 : s4.index w = t := by rw [← hs4, others_never_touch_an_index ys s3 hy, i3]
  have st4 : s4.store t = true := by rw [← hs4]; exact the_store_only_grows ys s3 st3
  simp [run, Step.apply, i4, st4]

/-! ## 不同写者的步骤可交换 -/

/-- HEAD 之外的四种步骤：暂存、写对象、钉检查点引用、崩溃。 -/
def Step.local_ : Step → Bool
  | .swap _ _ => false
  | .stage _ _ | .put _ | .pin _ | .crash _ => true

/-- 一步的前提与次序无关：钉引用的那一步，它的对象已经在库里。写者按「暂存、写对象、钉」的次序走，所以轮到它钉时这一条恒成立。 -/
def Step.ready (s : Repo) : Step → Prop
  | .pin w => s.store (s.index w) = true
  | .stage _ _ | .put _ | .swap _ _ | .crash _ => True

@[simp] theorem Repo.setIndex_index (s : Repo) (w : Writer) (t : Oid) (u : Writer) :
    (s.setIndex w t).index u = if u = w then t else s.index u := rfl

@[simp] theorem Repo.setIndex_store (s : Repo) (w : Writer) (t : Oid) :
    (s.setIndex w t).store = s.store := rfl

@[simp] theorem Repo.setIndex_pins (s : Repo) (w : Writer) (t : Oid) :
    (s.setIndex w t).pins = s.pins := rfl

@[simp] theorem Repo.setIndex_head (s : Repo) (w : Writer) (t : Oid) :
    (s.setIndex w t).head = s.head := rfl

theorem Repo.setIndex_comm (s : Repo) {w v : Writer} (t t' : Oid) (apart : w ≠ v) :
    (s.setIndex w t).setIndex v t' = (s.setIndex v t').setIndex w t := by
  simp only [Repo.setIndex]
  congr 1
  funext u
  by_cases h1 : u = v <;> by_cases h2 : u = w <;> simp_all

/-- 两个不同写者的相邻两步，只要都不改 HEAD、钉引用的那一步对象已在库里，换个次序城一样。任何交错都能经这样的相邻交换排成按写者分段的串行次序，所以每一个交错都等于一个串行次序。 -/
theorem independent_steps_commute (a b : Step) (s : Repo)
    (apart : a.writer ≠ b.writer) (la : a.local_ = true) (lb : b.local_ = true)
    (ra : a.ready s) (rb : b.ready s) :
    b.apply (a.apply s) = a.apply (b.apply s) := by
  have ne' := Ne.symm apart
  cases a <;> cases b <;> simp only [Step.local_] at la lb <;> (try contradiction) <;>
    simp only [Step.writer, Step.ready] at ra rb apart ne' <;>
    simp [Step.apply, ra, rb, apart, ne', Repo.setIndex_comm _ _ _ apart, Bool.or_left_comm] <;>
    rfl

/-! ## 引用更新的冲突被发现，恢复后等于一个串行次序 -/

/-- 比较后交换只在 HEAD 仍是它读到的值时改动 HEAD。 -/
theorem a_swap_moves_only_what_it_read (s : Repo) (w : Writer) (read : Option Oid)
    (moved : ((Step.swap w read).apply s).head ≠ s.head) : s.head = read := by
  simp only [Step.apply] at moved
  by_cases h : (s.head == read && s.store (s.index w)) = true
  · simp only [Bool.and_eq_true, beq_iff_eq] at h
    exact h.1
  · rw [if_neg h] at moved
    exact absurd rfl moved

/-- 两个写者从同一个读数 `read` 出发：先到的那个改了 HEAD，后到的那个被拒，先到者的更新不会丢。 -/
theorem no_update_is_lost (s : Repo) (w v : Writer) (read : Option Oid)
    (first : ((Step.swap w read).apply s).head ≠ s.head) :
    ((Step.swap v read).apply ((Step.swap w read).apply s)).head
      = ((Step.swap w read).apply s).head := by
  have was := a_swap_moves_only_what_it_read s w read first
  by_cases h : s.store (s.index w) = true
  · have after : (Step.swap w read).apply s = { s with head := some (s.index w) } := by
      simp [Step.apply, was, h]
    rw [after] at first ⊢
    have hne : some (s.index w) ≠ read := by
      rw [← was]
      exact first
    simp [Step.apply, hne]
  · have after : (Step.swap w read).apply s = s := by
      simp [Step.apply, h]
    rw [after] at first
    exact absurd rfl first

/-- `ensure_base`：读 HEAD，没有就以自己的树建它；HEAD 已在就什么也不做。 -/
def ensureBase (w : Writer) (s : Repo) : Repo :=
  match s.head with
  | none => (Step.swap w none).apply s
  | some _ => s

/-- 两个写者都读到「没有 HEAD」，各自交换；输的那个重读后什么也不做。结果等于赢家先 `ensure_base`、输家后 `ensure_base` 的串行次序。 -/
theorem a_lost_base_race_is_the_serial_order (s : Repo) (w v : Writer)
    (unborn : s.head = none) (written : s.store (s.index w) = true) :
    ensureBase v ((Step.swap v none).apply ((Step.swap w none).apply s))
      = ensureBase v (ensureBase w s) := by
  simp [ensureBase, Step.apply, unborn, written]

/-! ## 失败的检查点不动已有的引用 -/

theorem a_refused_step_changes_nothing (st : Step) (s : Repo)
    (refused : st.admitted s = false) : st.apply s = s := by
  cases st <;> simp only [Step.admitted] at refused <;> (try contradiction) <;>
    simp [Step.apply, refused]

/-- 钉引用之前的步骤（暂存、写对象）与崩溃都不改任何引用：一次在钉之前失败的检查点，留下的引用与开始时相同。 -/
theorem before_the_pin_no_reference_moves (st : Step) (s : Repo)
    (early : st = .put st.writer ∨ (∃ t, st = .stage st.writer t) ∨ st = .crash st.writer) :
    (st.apply s).pins = s.pins ∧ (st.apply s).head = s.head := by
  rcases early with h | ⟨t, h⟩ | h <;> rw [h] <;>
    simp [Step.apply, Repo.setIndex]

/-! ## 崩溃的写者只留下垃圾 -/

/-- 每个引用都指向对象库里的对象。 -/
def Repo.sound (s : Repo) : Prop :=
  (∀ w o, s.pins w o = true → s.store o = true) ∧ (∀ o, s.head = some o → s.store o = true)

theorem a_step_keeps_references_sound (st : Step) (s : Repo) (ok : s.sound) :
    (st.apply s).sound := by
  obtain ⟨hp, hh⟩ := ok
  cases st with
  | stage w t =>
    simp only [Step.apply, Repo.setIndex]; exact ⟨hp, hh⟩
  | crash w =>
    simp only [Step.apply, Repo.setIndex]; exact ⟨hp, hh⟩
  | put w =>
    simp only [Step.apply]
    exact ⟨fun u o h => by simp [hp u o h], fun o h => by simp [hh o h]⟩
  | pin w =>
    simp only [Step.apply]
    by_cases held : s.store (s.index w) = true
    · rw [if_pos held]
      refine ⟨fun u o h => ?_, hh⟩
      simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at h
      rcases h with ⟨_, rfl⟩ | h
      · exact held
      · exact hp u o h
    · rw [if_neg held]
      exact ⟨hp, hh⟩
  | swap w read =>
    simp only [Step.apply]
    by_cases held : (s.head == read && s.store (s.index w)) = true
    · rw [if_pos held]
      simp only [Bool.and_eq_true] at held
      exact ⟨hp, fun o h => by cases h; exact held.2⟩
    · rw [if_neg held]
      exact ⟨hp, hh⟩

/-- 任何交错之后，每个引用仍指向库里的对象：没有一个引用是坏的。 -/
theorem references_stay_inside_the_store (xs : List Step) (s : Repo) (ok : s.sound) :
    (run s xs).sound := by
  induction xs generalizing s with
  | nil => exact ok
  | cons x xs ih => exact ih (x.apply s) (a_step_keeps_references_sound x s ok)

/-- 从某个引用够得着。 -/
def Repo.reachable (s : Repo) (o : Oid) : Prop :=
  s.head = some o ∨ ∃ w, s.pins w o = true

/-- 崩溃不让任何对象变得可达：崩溃的写者写进库里而没钉下的对象，恰是够不着的垃圾。 -/
theorem a_crash_reaches_nothing_new (s : Repo) (w : Writer) (o : Oid) :
    ((Step.crash w).apply s).reachable o ↔ s.reachable o := by
  simp [Repo.reachable, Step.apply, Repo.setIndex]

/-! ## 增量检查点等于整个写域的检查点 -/

/-- 树里的一条路径。 -/
abbrev Path := Nat
/-- 一个文件的内容；Rust 代码存的是 git blob。 -/
abbrev Blob := Nat
/-- 一棵树，或一份 index：每条路径上有没有文件、是什么。 -/
abbrev Tree := Path → Option Blob

/-- 暂存 `chosen` 选中的路径：选中的取工作区里的样子，没选中的留 index 原样。 -/
def stage (chosen : Path → Bool) (index work : Tree) : Tree :=
  fun p => if chosen p then work p else index p

/-- 这一波碰过的路径都在写域里，写域里没碰过的路径工作区与 index 相同：只暂存碰过的路径，得到的树与暂存整个写域相同。edit 与 write 以它们写下的路径作 `touched`；exec 以修改时间或尺寸变了的路径作 `touched`，前提是两者都没变的文件内容也没变。 -/
theorem staging_the_touched_paths_is_staging_the_scope (scope touched : Path → Bool)
    (index work : Tree)
    (inside : ∀ p, touched p = true → scope p = true)
    (untouched : ∀ p, scope p = true → touched p = false → work p = index p) :
    stage touched index work = stage scope index work := by
  funext p
  simp only [stage]
  cases ht : touched p <;> cases hs : scope p <;> simp_all

end Storage.Checkpoint.Concurrent
