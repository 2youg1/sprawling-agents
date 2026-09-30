-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 回到过去，与并行 run 的写域

规定 `crates/storage/src/worktree/back.rs`：一个人回到城历史中某个 point 时 run 打开的那棵树，以及与它共用写域的两步——run 在自己的树里写一个文件（`crates/storage/src/worktree/trees.rs`），和从某个 point 把一个文件取回到那棵树里（`crates/storage/src/worktree/back.rs`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威（storage-SPEC.md 8-27）。

城的历史只由两样已有的东西构成：只增不减的那一本 Ledger，以及城仓库里写下后就不再改变的 git 对象。一个 point 就是一次提交。回到某个 point，是开一个新 run，给它一棵自己的树，树的分支从那次提交起；城所在的干线不动，因为此刻可能有别的 run 正在写，干线是它的工作落地的地方。撤销不是删除：它从更早的 point 取回一个文件，Ledger 把这次取回记成又一条记录。

模型有四条性质，每条一组定理：

* **回到过去打开的树恰在那个 point**——新 run 的文件恰好是它回到的那次提交里的文件；
* **回到过去拒绝一棵活着的树**——已被占用的名字不会被替换，所以回到过去从不覆盖任何 run 的树，调用者自己的也不例外；
* **一步只动自己那个 run 的树**——回到过去、写、取回，只改变走这一步的 run 的树，不动别的树，也从不动干线；在任何一串都不属于某个 run 的步骤之后，那个 run 的树与开始时相同，所以它写下的内容经得起别的 run 做的一切；
* **Ledger 只增不减**——每一步恰好追加一条记录，取回（即撤销）也是其中之一。

把一个 run 的工作合进干线走已有的快进路径（storage-SPEC.md 8-9），这里不建模。
-/

namespace Storage.Worktree.Back

/-- 城仓库里的一次提交：历史上的一个 point。 -/
abbrev Commit := Nat
/-- 一个 run 的树的名字（`WorktreeName`）；一个 run 一个名字。 -/
abbrev Name := Nat
/-- 树里的一条文件路径。 -/
abbrev Path := Nat
/-- 文件内容；Rust 代码存的是 git blob。 -/
abbrev Content := Nat

/-- 一个 run 的树：它的分支起于哪次提交，以及此刻的文件。 -/
structure Tree where
  base : Commit
  files : Path → Option Content

/-- 每一步追加到 Ledger 的记录。 -/
inductive Record where
  | forked (run : Name) (point : Commit)
  | wrote (run : Name) (path : Path)
  | restored (run : Name) (path : Path) (point : Commit)
  deriving Repr, DecidableEq

/-- 城：干线所在的提交、已有的提交、每次提交里有什么（git 对象不可变，所以这是一个函数）、活着的树，以及按追加次序排列的 Ledger。 -/
structure City where
  trunk : Commit
  commits : List Commit
  history : Commit → Path → Option Content
  trees : Name → Option Tree
  ledger : List Record

/-- 让 `n` 持有 `t`，其余名字不变。 -/
def put (trees : Name → Option Tree) (n : Name) (t : Tree) : Name → Option Tree :=
  fun m => if m = n then some t else trees m

/-- 让文件 `p` 取值 `v`，其余文件不变。 -/
def Tree.set (t : Tree) (p : Path) (v : Option Content) : Tree :=
  { t with files := fun q => if q = p then v else t.files q }

/-- 以新 run `n` 回到 `c`：`n` 已持有一棵树，或 `c` 不是城里的提交时，拒绝。 -/
def goBack (s : City) (n : Name) (c : Commit) : Option City :=
  if (s.trees n).isNone ∧ c ∈ s.commits then
    some { s with
      trees := put s.trees n ⟨c, s.history c⟩
      ledger := s.ledger ++ [.forked n c] }
  else none

/-- run `n` 在自己的树里把 `p` 写成 `v`；没有树的 run 什么也写不了。 -/
def write (s : City) (n : Name) (p : Path) (v : Content) : Option City :=
  match s.trees n with
  | none => none
  | some t => some { s with
      trees := put s.trees n (t.set p (some v))
      ledger := s.ledger ++ [.wrote n p] }

/-- run `n` 从 point `c` 把文件 `p` 取回到自己的树里。那个 point 上没有的文件被删掉，取回它就是这个意思。 -/
def restore (s : City) (n : Name) (p : Path) (c : Commit) : Option City :=
  match s.trees n with
  | none => none
  | some t =>
    if c ∈ s.commits then
      some { s with
        trees := put s.trees n (t.set p (s.history c p))
        ledger := s.ledger ++ [.restored n p c] }
    else none

/-- 任何 run 都可以走的一步。 -/
inductive Step where
  | goBack (run : Name) (point : Commit)
  | write (run : Name) (path : Path) (content : Content)
  | restore (run : Name) (path : Path) (point : Commit)

/-- 这一步关乎哪个 run 的树。 -/
def Step.run : Step → Name
  | .goBack n _ | .write n _ _ | .restore n _ _ => n

def Step.apply : Step → City → Option City
  | .goBack n c, s => Storage.Worktree.Back.goBack s n c
  | .write n p v, s => Storage.Worktree.Back.write s n p v
  | .restore n p c, s => Storage.Worktree.Back.restore s n p c

/-- 按次序走的步骤；第一次拒绝就让整串停下。 -/
def steps (s : City) : List Step → Option City
  | [] => some s
  | st :: rest =>
    match st.apply s with
    | none => none
    | some s' => steps s' rest

/-! ## 回到过去打开的树恰在那个 point -/

theorem goBack_opens_at_the_point {s s' : City} {n : Name} {c : Commit}
    (h : goBack s n c = some s') :
    ∃ t, s'.trees n = some t ∧ t.base = c ∧ t.files = s.history c := by
  unfold goBack at h
  split at h
  · cases h
    exact ⟨⟨c, s.history c⟩, by simp [put], rfl, rfl⟩
  · contradiction

/-! ## 回到过去拒绝一棵活着的树 -/

theorem goBack_refuses_a_live_tree {s : City} {n : Name} (c : Commit)
    (live : (s.trees n).isSome) : goBack s n c = none := by
  unfold goBack
  cases hn : s.trees n with
  | none => simp [hn] at live
  | some _ => simp

/-! ## 一步只动自己那个 run 的树 -/

theorem step_leaves_other_trees {st : Step} {s s' : City} {m : Name}
    (h : st.apply s = some s') (other : m ≠ st.run) :
    s'.trees m = s.trees m := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h
      simp [put, show m ≠ n from other]
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h
      simp [put, show m ≠ n from other]
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h
        simp [put, show m ≠ n from other]
      · contradiction

/-- 没有哪一步移动干线或改写一次提交；所以回到过去扰动不了一个工作正要进干线的 run。 -/
theorem step_keeps_trunk_and_history {st : Step} {s s' : City}
    (h : st.apply s = some s') :
    s'.trunk = s.trunk ∧ s'.commits = s.commits ∧ s'.history = s.history := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h; exact ⟨rfl, rfl, rfl⟩
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h; exact ⟨rfl, rfl, rfl⟩
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h; exact ⟨rfl, rfl, rfl⟩
      · contradiction

theorem others_never_touch_a_tree {s s' : City} {m : Name} :
    ∀ {trace : List Step}, steps s trace = some s' →
      (∀ st ∈ trace, st.run ≠ m) → s'.trees m = s.trees m := by
  intro trace
  induction trace generalizing s with
  | nil =>
    intro h _
    cases h
    rfl
  | cons st rest ih =>
    intro h others
    simp only [steps] at h
    split at h
    · contradiction
    · rename_i mid hmid
      have rest_same := ih h (fun st' mem => others st' (List.mem_cons_of_mem _ mem))
      have first_same := step_leaves_other_trees hmid
        (Ne.symm (others st List.mem_cons_self))
      rw [rest_same, first_same]

/-- 一个 run 写下的内容，经得起别的 run 走的任意一串步骤，回到过去也在其中。 -/
theorem a_write_survives_other_runs {s s₁ s₂ : City} {m : Name} {p : Path}
    {v : Content} {trace : List Step}
    (wrote : write s m p v = some s₁) (ran : steps s₁ trace = some s₂)
    (others : ∀ st ∈ trace, st.run ≠ m) :
    (s₂.trees m).bind (·.files p) = some v := by
  rw [others_never_touch_a_tree ran others]
  unfold write at wrote
  split at wrote
  · contradiction
  · cases wrote
    simp [put, Tree.set]

/-! ## Ledger 只增不减 -/

theorem step_appends_one_record {st : Step} {s s' : City}
    (h : st.apply s = some s') : ∃ r, s'.ledger = s.ledger ++ [r] := by
  cases st with
  | goBack n c =>
    simp only [Step.apply, goBack] at h
    split at h
    · cases h; exact ⟨_, rfl⟩
    · contradiction
  | write n p v =>
    simp only [Step.apply, write] at h
    split at h
    · contradiction
    · cases h; exact ⟨_, rfl⟩
  | restore n p c =>
    simp only [Step.apply, restore] at h
    split at h
    · contradiction
    · split at h
      · cases h; exact ⟨_, rfl⟩
      · contradiction

theorem the_ledger_only_grows {s s' : City} :
    ∀ {trace : List Step}, steps s trace = some s' → s.ledger <+: s'.ledger := by
  intro trace
  induction trace generalizing s with
  | nil =>
    intro h
    cases h
    exact List.prefix_refl _
  | cons st rest ih =>
    intro h
    simp only [steps] at h
    split at h
    · contradiction
    · rename_i mid hmid
      obtain ⟨r, grown⟩ := step_appends_one_record hmid
      exact List.IsPrefix.trans ⟨[r], grown.symm⟩ (ih h)

/-- 撤销就是取回，而取回追加自己的记录，不删掉它撤销的那一条。 -/
theorem undo_is_an_appended_restore {s s' : City} {n : Name} {p : Path}
    {c : Commit} (h : restore s n p c = some s') :
    s'.ledger = s.ledger ++ [.restored n p c] := by
  unfold restore at h
  split at h
  · contradiction
  · split at h
    · cases h; rfl
    · contradiction

end Storage.Worktree.Back
