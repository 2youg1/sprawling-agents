-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::worktree::trees::stock

规定 `worktree::trees::stock`（`crates/storage/src/` 下同名的文件）。备树：检出在人等之前做完，放置只改名。Markdown 规格 `crates/storage/storage-SPEC.md` 仍是 storage 唯一生效的规格；本分部陈述并证明它相应各节写下的性质，切换到 `crates/storage/Spec.lean` 时收下那些节。
-/

/-!
## 模型：接管的每一步之后断掉，都收得回来

接管备树动的是盘上的四样东西：两个目录 `<home>/+spare` 与 `<home>/<name>`（各自里面的 `.git` 文件指向一个登记），两个登记 `.git/worktrees/+spare` 与 `.git/worktrees/<name>`（各自的 `gitdir` 指向一个目录），外加城仓库里节点那棵树的 HEAD 在不在节点分支上。一棵树「一致」，是说它的目录与登记互相指着对方。

接管的步骤是 §8-35 的 ②–⑥（① 建分支不改这四样）：改名目录、改写 `.git`、改名登记、改写 `gitdir`、再领。进程可能在任何一步之后死掉。下一次放置（`standing` 收回指向不存在目录的登记，`place` 清掉没有登记的同名目录，然后接管或全量检出，再领把 HEAD 指到节点分支）与下一次 `stock`（收回认不出的备树、清掉没有登记的 `+spare` 目录，再备一棵）把盘收回到这样的样子：节点的树一致且 HEAD 在节点分支上，备树一致（`every_cut_is_recovered`）。备树被拿走只成一次：改名之后 `+spare` 目录不在了，第二次接管见不到它，转去全量检出（`the_stock_is_taken_once`）。
-/

namespace Storage.Worktree.Trees.Stock

/-- 一个目录或登记属于谁：备树，或节点。 -/
inductive Owner where
  | spare
  | node
  deriving DecidableEq, Repr

/-- 接管动到的盘：每个目录在不在、它的 `.git` 指向哪个登记；每个登记在不在、它的 `gitdir` 指向哪个目录；节点那棵树的 HEAD 在不在节点分支上。 -/
structure Disk where
  dirSpare : Option Owner
  dirNode : Option Owner
  regSpare : Option Owner
  regNode : Option Owner
  headOnNode : Bool
  deriving DecidableEq, Repr

/-- 目录与登记互相指着对方。 -/
def Disk.consistent (d : Disk) : Owner → Bool
  | .spare => d.dirSpare == some .spare && d.regSpare == some .spare
  | .node => d.dirNode == some .node && d.regNode == some .node

/-- 接管之前：备树备好，节点没有树。 -/
def stocked : Disk := ⟨some .spare, none, some .spare, none, false⟩

/-- ②：把目录 `<home>/+spare` 改名为 `<home>/<name>`，这就是「拿走」；目录不在即 `NotFound`。 -/
def take (d : Disk) : Option Disk :=
  match d.dirSpare with
  | some t => some { d with dirSpare := none, dirNode := some t }
  | none => none

/-- ③–⑥，各一步。 -/
def relink (d : Disk) : Disk := { d with dirNode := d.dirNode.map fun _ => .node }
def moveRegistration (d : Disk) : Disk := { d with regNode := d.regSpare, regSpare := none }
def repoint (d : Disk) : Disk := { d with regNode := d.regNode.map fun _ => .node }
def reattach (d : Disk) : Disk := { d with headOnNode := true }

/-- 接管走到第 `k` 步之后断掉时盘的样子（`k = 0` 是还没动）。 -/
def cutAfter (k : Nat) : Disk :=
  let steps : List (Disk → Disk) :=
    [fun d => (take d).getD d, relink, moveRegistration, repoint, reattach]
  (steps.take k).foldl (fun d f => f d) stocked

/-- 一个登记指向的目录在不在。 -/
def Disk.dirOf (d : Disk) : Owner → Option Owner
  | .spare => d.dirSpare
  | .node => d.dirNode

/-- 下一次放置：`standing` 收回指向不存在目录的登记；`place` 清掉没有登记的同名目录；留着的树再领，否则接管一致的备树、接不了就全量检出；再领把 HEAD 指到节点分支。 -/
def place (d : Disk) : Disk :=
  let d := match d.regNode with
    | some t => if (d.dirOf t).isSome && t == Owner.node then d else { d with regNode := none }
    | none => d
  let d := if d.regNode.isNone then { d with dirNode := none } else d
  if d.consistent .node then reattach d
  else if d.consistent .spare then
    reattach (repoint (moveRegistration (relink ((take d).getD d))))
  else { d with dirNode := some .node, regNode := some .node, headOnNode := true }

/-- 下一次 `stock`：认不出的备树收回，没有登记的 `+spare` 目录清掉，再备一棵；一致的备树留着。 -/
def stock (d : Disk) : Disk :=
  if d.consistent .spare then d else { d with dirSpare := some .spare, regSpare := some .spare }

/-- **每一步之后断掉都收得回来。** 接管在 ②–⑥ 的任何一步之后断掉，下一次放置与下一次 `stock` 之后，节点的树一致、HEAD 在节点分支上，备树一致。 -/
theorem every_cut_is_recovered :
    ∀ k ∈ List.range 6,
      let d := stock (place (cutAfter k))
      d.consistent .node = true ∧ d.headOnNode = true ∧ d.consistent .spare = true := by
  decide

/-- 不是空话：没有断掉的接管本身就留下一致的节点树，备树被拿走。 -/
theorem a_whole_adoption_leaves_the_nodes_tree :
    (cutAfter 5).consistent .node = true ∧ (cutAfter 5).headOnNode = true ∧
      (cutAfter 5).dirSpare = none := by
  decide

/-- **备树被拿走只成一次。** 改名成了之后，第二次改名见不到 `+spare` 目录。 -/
theorem the_stock_is_taken_once (d d' : Disk) (taken : take d = some d') : take d' = none := by
  unfold take at taken
  split at taken
  · simp only [Option.some.injEq] at taken
    subst taken
    rfl
  · simp at taken

end Storage.Worktree.Trees.Stock
