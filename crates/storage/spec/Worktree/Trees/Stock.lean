-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::worktree::trees::stock

规定 `worktree::trees::stock`（`crates/storage/src/` 下同名的文件）。备树：检出在人等之前做完，放置只改名。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-35 `storage::worktree::trees::stock`：备树——检出在人等之前做完，放置只改名（形状 4 适配器）

```rust
impl Worktrees {
    pub fn stock(&self) -> Result<FileWork, StorageError>;
}
// 私有：`place` 在上限预检之后先试接管
fn adopt(&self, name: &WorktreeName, city: &Weight) -> Result<Option<WorktreeLease>, StorageError>;
```

- **为什么不能在领树时做完。** 放置一棵 512 个 16 KB 文件的树，等的是实时扫描对每个新建文件的放行（8-31）。在开着 Defender 的 NTFS 上（Windows x86_64，16 核，debug 构建）：直接写 512 个 16 KB 文件 0.47–0.53 s，512 个空文件 0.14 s，`CopyFile` 0.36–0.39 s，libgit2 全量检出 1.2 s（512 个相同的文件）到 2.2 s（512 个各不相同的文件）；把装着 512 个文件的目录改名 2.4–4.0 ms。扫描按新建的文件计价，所以任何在领树那一刻新建 N 个文件的做法都随 N 计价，缩小 libgit2 自己的开销也只能把 1.2 s 压向 0.5 s。降到毫秒只有一条路：检出挪到没人等的时候，领树时只改名。
- **备树是什么。** 一棵登记在城仓库里、id 为 `+spare` 的 git worktree：目录 `<city>/.sprawling/worktrees/+spare`，分支 `+spare`，检出在干线上。`+` 不在 `WorktreeName` 的字符集里（8-9），所以没有节点能叫这个名字，`claim`、`release`、`plan_merge`、`claim_at` 都够不到它；`live` 不列它，所以开城的清扫（8-9）留着它——它是库存，不是崩溃留下的树，下一次服务的第一次放置照样接管它。git 的 refname 规则容许 `+`。
- **`stock` 做什么。** 没有备树：按上限预检（与放置同一个 `refuse_oversized`），清掉一个没有登记的 `+spare` 目录（上一次备树中途失败留下的），分支 `+spare` 强制指向干线，加锁全量检出（与放置同一个 `check_out_tree`），把索引从分支头整份重读一遍再写下，然后解锁；检出失败时把登记收回，失败照原样交出。锁着的备树就是「还没检出完」，接管只接没锁的。已有、没锁：加锁，分支移到干线，整棵强制检出到分支头（与再领同一段 `restore`），同样重读、写下索引，解锁——只写干线自上次备树以来改过的文件。重读索引是为了让它带上树缓存：重读保留 blob 与模式没变的每一项的 stat，而检出之后才写下的索引让每一项都不再「racily clean」，于是接管时「索引写成的是不是分支头的树」不必哈希一棵树就答得出（debug 下这一问从 50 ms 降到 4 µs）。这一段失败也先解锁，因为一棵检出到一半的备树会在接管时被那一次 `restore` 补全，而留下的锁会让本次服务里再也没有备树可接管。已有而锁着：别的调用正在备，答零。登记在而目录或链接文件不是本模块写下的样子：收回登记再新备一棵。返回 `FileWork`（8-31）：新备时 `created` 是树的文件数、`walked` 是城的目录项；带到干线时三个数是干线改过的文件。
- **放置先接管。** `place` 在上限预检之后先清掉同名的、没有登记的目录（`standing` 已答 `Absent`，那个目录只能是一次中途失败的放置留下的），再试 `adopt`；备树不在、锁着、已被别的放置拿走、或链接文件认不出时答 `None`，放置照旧全量检出。接管的步骤，次序是有意的：①节点没有分支就在干线上建一条（有就用它，那是节点的工作线）；②把目录 `<home>/+spare` 改名为 `<home>/<name>`——这是「拿走」，两条 lane 同时放置时只有一条改得成，另一条见 `NotFound` 便全量检出；③把目录里的 `.git` 文件改指 `.git/worktrees/<name>`；④把登记目录 `.git/worktrees/+spare` 改名为 `.git/worktrees/<name>`；⑤把登记里的 `gitdir` 改指 `<home>/<name>/.git`；⑥走再领那一段（`reattach`，scope 取整棵）：HEAD 指到节点分支、跟上干线、索引不是分支头的树就整份重读、整棵强制检出、加锁、称重。所以备树之后干线动过的文件在⑥里补写，节点已有分支时写的是分支与备树之差。分支 `+spare` 留着，下一次 `stock` 强制把它指到干线：删它要 libgit2 逐棵打开城里的每一棵树查它有没有被检出，代价随树的数目长（debug 下九棵树 33–48 ms）。HEAD 用写一个符号引用改指（`reference_symbolic`），不用 `set_head`，理由相同：`set_head` 也逐棵查分支有没有被检出（debug 下 34 ms）。
- **两份链接文件是 git 写下的布局，本模块只换末尾那一段 id。** git-worktree(1) 的 DETAILS 一节记着这两份：工作树里的 `.git` 文件（`gitdir: <公共 git 目录>/worktrees/<id>/`）与登记目录里的 `gitdir`（`<工作树>/.git`）；`git worktree move` 改的也是它们。接管读出 libgit2 写下的原文，认得出末尾的 `worktrees/+spare/` 与 `/+spare/.git` 才接管，只把这一段换成节点的名字，其余字样（盘符、斜杠方向）原样保留；认不出就不接管，备树留给 `stock` 收回重备。
- **每一步之后断掉都收得回来。** ②之后：`<home>/<name>` 没有登记，下一次放置先清掉它再放；备树的登记指向一个不存在的目录，下一次 `stock` 收回它重备。④之后：登记 `<name>` 指向不存在的 `<home>/+spare`，`standing` 收回登记（8-9 的「登记在册但目录不存在」），放置清掉目录再放。⑤之后、⑥之前：树与登记已一致，HEAD 仍指 `+spare`；`standing` 答「留着」，再领的 `restore` 先把 HEAD 指到节点分支，所以节点不会在别的分支上工作。
- **读数。** 同一台机器、debug 构建、512 个 16 KB 文件的城：放置一次全量检出 1.18–1.30 s；`stock` 新备一棵 1.19–1.35 s（就是那次全量检出，挪到了没人等的地方）；放置接管备树 30–37 ms，其中拿走（读备树、建节点分支、两次改名、两份链接文件）约 15 ms，`restore`（打开、改指 HEAD、读索引、整棵检出时对 512 个文件各一次 stat）约 19 ms，称重约 1.5 ms。发行构建的读数由 `just bench` 的 `large_worktree_placement` 给出（`tools/citysim/Spec.lean` §8-12）。
- **计数（8-31）。** 接管的 `created`、`rewritten`、`removed` 是⑥写下的：干线自备树以来改过的文件，与树里有多少文件无关；`walked` 是城的目录项（上限预检）加新树的目录项（称重）。断言：`a_placement_from_the_stock_creates_no_file_at_either_size` 在 32 与 64 个文件的城上各备一棵、放置一次，整值比较两份 `FileWork`：备树的 `created` 是 N，放置的三个数都是 0；`a_stock_behind_the_trunk_is_placed_at_the_trunk_and_merges` 备树之后干线改一个文件，放置 `rewritten` 为 1、HEAD 在节点分支上、节点献出的改动照常合进干线；`the_stock_is_no_nodes_tree_and_outlives_the_sweep` 开城清扫之后备树还在，下一次放置不新建文件。
- **谁来调 `stock`。** 它的代价就是一次全量检出，所以它只能放在没人等的地方：一条 lane 在它的 run 落地之后（`crates/sprawling/Spec.lean` §8-145）。本 crate 只提供这扇门，不自己起线程（ARCHITECTURE §10 第 3 条：库 crate 起线程的地方是点名的）。没有调用者时每一次放置都全量检出，与没有这一节时相同。
- **两个并发的窗口，结局都正确。** 两次放置争一棵备树：改名只成一次（见②）。`stock` 刷新备树与一次放置接管它：接管先看锁再改名，二者之间刷新可能刚加锁；那时刷新的检出写进一个已经改了名的路径而失败，接管的⑥把树补全，下一次 `stock` 清掉刷新留下的残目录重备。
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

/-! D14 放置接管一棵事先检出好的备树，备树在没人等的时候检出（8-35）

决定：`Worktrees::stock` 在城仓库里留一棵检出在干线上的 worktree；`place` 先把它改名成节点的树、改写 git 的两份链接文件、再按再领那一段补写干线自备树以来的改动，接管不成才全量检出。理由：放置的时间花在实时扫描对每个新建文件的放行上（8-31、8-35 的读数），按新建文件计价；不在领树时新建文件，是让放置随文件数不变的唯一办法，而一次目录改名是毫秒级。被否：①只检出 scope——run 会读 scope 之外的文件，而且 git2 不给安全的检出选项（8-9）；②硬链接——两棵树共享可写字节，`alias` 对链接计数大于 1 的文件一律拒写（8-25），run 在自己的树里就写不了任何文件；③CoW 克隆——NTFS 没有，ReFS／Dev Drive 的块克隆要 `unsafe` 的 FSCTL 调用或一个 Zig 叶子，而且 Dev Drive 要管理员权限建；④压低 libgit2 的检出开销——最好也只到直接写文件的 0.5 s，仍以秒计；⑤在 `release` 里补备树——`release` 在记账线程上（`crates/sprawling/Spec.lean` §8-113），补备树要等一次全量检出。代价：盘上多一棵树（不超过 `WORKTREE_MAX_BYTES`）；两份链接文件按 git 文档里的布局由本模块改写，换的只是末尾的 id。**重开参数**：git2 提供移动或改名 worktree 的接口（届时改用它）；城所在的卷支持块克隆而进程不需要额外权限（届时克隆比备树便宜，也不占一棵树的盘）。
-/
