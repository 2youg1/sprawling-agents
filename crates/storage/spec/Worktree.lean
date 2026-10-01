-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::worktree

规定 `worktree`、`worktree::name`、`worktree::lease`、`worktree::sweep`、`worktree::trees`、`worktree::trees::kept`、`worktree::landing`、`worktree::weight`、`reserved`（`crates/storage/src/` 下同名的文件）。一个节点一棵工作树，对象共享而文件不共享：领、还、再领、开城清扫、合并，以及领树的文件操作计数；`reserved` 是这里与 checkpoint 共用的「哪些字节属于人」的唯一谓词。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
-/

/-!
## 模型：租约是锁，树留在盘上，开城解锁再清扫

一个名字下的树有三种处境（Rust 的 `trees::kept::Standing`）：`Held`（锁着，有 run 在用）、`Kept`（登记与目录都在、没锁，同名的下一次领用它）、`Absent`。备树的 id `+spare` 不是 `WorktreeName`，所以下面每一个按名字的操作都够不到它（§8-35）。

* `claim` 遇到锁着的树即 `WorktreeBusy`，什么都不改（`a_held_tree_refuses_a_second_claim`）；`release` 只解锁，树留下，同名再领是再领而不是新放置（`a_released_tree_is_claimed_again_without_a_placement`）；一次领或还只动它自己那个名字，也不动备树（`a_claim_touches_only_its_name`）。
* 城的唯一写者打开时 `lift_abandoned_leases` 解开全部锁，之后没有一棵树是锁着的，所以 `E_WORKTREE_BUSY` 恒表示有活着的 run 正在用（`after_lifting_no_tree_is_held`）。
* `sweep_abandoned` 收走 `held` 之外的每一棵，`held` 里的一概不动，备树不在清扫范围里（`the_sweep_keeps_what_is_held_and_the_stock`）；`bin::assembly` 开城时先解锁、再以空的 `held` 清扫，之后没有节点的树留下，备树还在（`opening_a_city_leaves_only_the_stock`）。
-/

namespace Storage.Worktree

/-- 一个节点的树的名字。 -/
abbrev WorktreeName := Nat

/-- 一个名字下的树的处境（`trees::kept::Standing`）。 -/
inductive Standing where
  | Held
  | Kept
  | Absent
  deriving DecidableEq, Repr

/-- 领一棵树时做了什么：全量放置（或接管备树），还是再领一棵留着的树。 -/
inductive Claimed where
  | Placed
  | Reattached
  deriving DecidableEq, Repr

/-- 城的树：每个名字的处境，与备树在不在。 -/
structure Home where
  standing : WorktreeName → Standing
  stock : Bool

/-- 改一个名字的处境。 -/
def Home.set (h : Home) (n : WorktreeName) (s : Standing) : Home :=
  { h with standing := fun m => if m = n then s else h.standing m }

/-- `Worktrees::claim`：锁着即拒；留着的再领；没有就放置。领到的树都锁上。 -/
def claim (h : Home) (n : WorktreeName) : Option (Claimed × Home) :=
  match h.standing n with
  | .Held => none
  | .Kept => some (.Reattached, h.set n .Held)
  | .Absent => some (.Placed, h.set n .Held)

/-- `Worktrees::release`：只解锁，登记与目录都留下。 -/
def release (h : Home) (n : WorktreeName) : Home :=
  match h.standing n with
  | .Held => h.set n .Kept
  | .Kept | .Absent => h

/-- `Worktrees::lift_abandoned_leases`：之前的写者没还的锁全部解开。 -/
def lift (h : Home) : Home :=
  { h with standing := fun n => match h.standing n with
      | .Held => .Kept
      | .Kept => .Kept
      | .Absent => .Absent }

/-- `Worktrees::sweep_abandoned`：`held` 之外的树全部收走；备树不是 `WorktreeName`，不在其中。 -/
def sweep (h : Home) (held : List WorktreeName) : Home :=
  { h with standing := fun n => if n ∈ held then h.standing n else .Absent }

theorem a_held_tree_refuses_a_second_claim (h : Home) (n : WorktreeName)
    (held : h.standing n = .Held) : claim h n = none := by
  simp [claim, held]

theorem a_released_tree_is_claimed_again_without_a_placement (h h' : Home) (n : WorktreeName)
    (c : Claimed) (claimed : claim h n = some (c, h')) :
    (claim (release h' n) n).map Prod.fst = some .Reattached := by
  unfold claim at claimed
  split at claimed <;> simp at claimed <;> obtain ⟨-, rfl⟩ := claimed <;>
    simp [claim, release, Home.set]

theorem a_claim_touches_only_its_name (h h' : Home) (n m : WorktreeName) (c : Claimed)
    (claimed : claim h n = some (c, h')) (other : m ≠ n) :
    h'.standing m = h.standing m ∧ h'.stock = h.stock := by
  unfold claim at claimed
  split at claimed <;> simp at claimed <;> obtain ⟨-, rfl⟩ := claimed <;>
    simp [Home.set, other]

theorem after_lifting_no_tree_is_held (h : Home) (n : WorktreeName) :
    (lift h).standing n ≠ .Held ∧ (claim (lift h) n).isSome := by
  cases hs : h.standing n <;> simp [lift, claim, hs]

theorem the_sweep_keeps_what_is_held_and_the_stock (h : Home) (held : List WorktreeName) :
    (∀ n ∈ held, (sweep h held).standing n = h.standing n) ∧
      (∀ n, n ∉ held → (sweep h held).standing n = .Absent) ∧ (sweep h held).stock = h.stock := by
  refine ⟨fun n hn => by simp [sweep, hn], fun n hn => by simp [sweep, hn], rfl⟩

theorem opening_a_city_leaves_only_the_stock (h : Home) :
    (∀ n, (sweep (lift h) []).standing n = .Absent) ∧ (sweep (lift h) []).stock = h.stock := by
  refine ⟨fun n => by simp [sweep], rfl⟩

end Storage.Worktree
