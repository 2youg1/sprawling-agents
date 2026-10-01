-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::worktree

规定 `worktree`、`worktree::name`、`worktree::lease`、`worktree::sweep`、`worktree::trees`、`worktree::trees::kept`、`worktree::landing`、`worktree::weight`、`reserved`（`crates/storage/src/` 下同名的文件）。一个节点一棵工作树，对象共享而文件不共享：领、还、再领、开城清扫、合并，以及领树的文件操作计数；`reserved` 是这里与 checkpoint 共用的「哪些字节属于人」的唯一谓词。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-9 storage::worktree（形状 4 适配器＋形状 2 值类型；git2）

```rust
pub struct WorktreeName(String);        // 文件系统安全；无分隔符、无点开头
pub struct Worktrees { /* repo、home、ceiling —— 私有 */ }
pub struct WorktreeLease { /* name、path、disk —— 私有 */ }
impl Worktrees {
    pub fn open(city_root: &Path) -> Result<Worktrees, StorageError>;
    /// `scopes` 是这棵树的写域（与检查点同一组 pathspec，空即整棵）：再领时只检出它。
    pub fn claim(&self, name: &WorktreeName, scopes: &[String]) -> Result<WorktreeLease, StorageError>;
    /// 备一棵树给下一次放置接管（8-35）：没有就全量检出一棵，有就带到干线。返回这次备树花了文件系统什么。
    pub fn stock(&self) -> Result<FileWork, StorageError>;
    pub fn release(&self, lease: WorktreeLease) -> Result<(), StorageError>;   // 解锁，不删树
    /// 城的唯一写者（借出的 `JsonlLedger` 即凭证）打开时调用：之前的写者没还的锁全部解开。无仓库即无事可做。
    pub fn lift_abandoned_leases(city_root: &Path, writer: &JsonlLedger) -> Result<(), StorageError>;
    pub fn live(&self) -> Result<Vec<WorktreeName>, StorageError>;
    /// 开城时收走崩溃留下的树：`held` 之外、住在 `<city>/.sprawling/worktrees/` 下的登记、目录与租约分支。
    /// 城没有仓库时答空。返回收走的名字，排序。
    pub fn sweep_abandoned(city_root: &Path, held: &[WorktreeName]) -> Result<Vec<WorktreeName>, StorageError>;
    /// 把一个节点已提交的活带进城的 trunk，返回落地的 commit。
    pub fn plan_merge(&self, name: &WorktreeName) -> Result<PlannedMerge<'_>, StorageError>;
}
/// 一次合并要写下的东西，四个恒同行的值合成一个。
pub struct Landing<'a> {
    pub t: TimeMs,
    pub of: &'a Provenance,
    pub subject: &'a str,
    /// 人亲自看过。真且仓库 git config 里有 user.name 与 user.email 时，
    /// 消息多一行 `Reviewed-by: Name <email>`。
    pub reviewed_by_person: bool,
}
impl PlannedMerge<'_> {
    pub fn commit(&self) -> String;
    pub fn apply(self, landing: &Landing<'_>) -> Result<(), StorageError>;
}
impl WorktreeLease {
    pub fn name(&self) -> &WorktreeName;  pub fn path(&self) -> &Path;  pub fn disk(&self) -> ByteLen;
    pub fn work(&self) -> FileWork;   // 这次领树的文件操作计数（8-31）
    pub fn opened_payload(&self) -> Result<Payload, StorageError>;   // worktree_opened，形状是 kernel::event::record::WorktreeOpened
}
```

- **一节点一棵，且它是 git worktree**：对象共享、工作树不共享，于是两个 Agent 看不见对方的中间态，而合入只走 PR 流。它从 `storage::checkpoint` 已在管的那个仓库分枝——城里不开第二个仓库。
- **建树前预检，不是建到一半失败**：工作树字节数 > `WORKTREE_MAX_BYTES` 即拒，拒词带当前上限与实测值。reflink 今天不尝试（无 unsafe FFI 或新依赖就没有 CoW 接口），故设计里「CoW 则 reflink，否则按上限拒」在每个平台上都只走后一臂——这是当前口径，不是已实现的 CoW。可用磁盘余量未探（std 无该接口），同样写在明处。
- **上限只称人的字节**：`measure` 跳 `.git` 与 `RESERVED_PREFIX` 子树（谓词住 `storage::reserved`，与 checkpoint 的 `stage_tree` 同一个）。
  账本、CAS、投影与别人的工作树都住 reserved 之下；把它们算进来，跑了一个月的城会因为自己的簿记长大而拒绝派活，
  并用一句「城的工作树有 N 字节」说这件事。断言：账本 4 KB、产品文件不到 1 KB 的城仍可领树。
- **上限称一次检出，不称留着的树之和**：`WORKTREE_MAX_BYTES` 只在 `place`（新建一棵：接管备树或全量检出，8-35）与 `stock` 之前量城的工作树；再领一棵留着的树不量，留着的各棵也不相加。
  上限挡的是「一次检出要复制多少字节」，只有量在复制之前才挡得住；再领只把树重置到分支头，写的是差量。
  否决「领树时把留着的树加起来比上限」：那要在每次领树时走遍每一棵留着的树，正是保留树省下的那次全量遍历；而且它量的其实是磁盘占用，该比的是磁盘余量，std 读不到。
  盘上的总量因此大致是评审房间数乘以单棵上限（再领不量，所以干线长过上限之后，留着的树也跟着长过去）；留着的树在下一次开城时由 `sweep_abandoned` 收走（见下）。重开的条件：能廉价读到磁盘余量时，改为按余量拒；评审房间数不再有界时，先做回收。
- **问不出祖先关系不是「不是祖先」**：`graph_descendant_of` 的失败按 `Worktree{op:"judge a fast-forward"}` 上报，
  不说成 `MergeStale`——把一次 git 失败说成「trunk 动过了」，会让一个 trunk 没动的人回去重做不需要重做的活。
- **租约是 git 的 worktree 锁，树在一次服务期间留在盘上**：`claim` 以加锁的方式建树（`WorktreeAddOptions::lock`），`release` 只解锁，登记与目录都留下。
  同名再领时树已在：不量城的工作树、不重新检出整棵树；干线已含该节点分支的全部提交时先把分支快进到干线（分支上还有未合入的活则不动它，那份活仍在等合并），
  索引写成的树不是分支头的树时再把索引整份重读成分支头的树，然后只在 `scopes` 之下强制检出到分支头并删掉未跟踪文件（`CheckoutBuilder::path`，每个 scope 一条 pathspec），最后加锁——
  一个节点在一次服务里付一次全量检出，而不是每次 run 付一次、再付一次整目录删除；再领写的是 scope 里的差量，scope 之外的文件一个也不写。
  scope 里未提交的改动与未跟踪文件在再领时消失：没提交的从来不是这个节点的活。scope 之外盘上的文件停在上一次放置或上一次 run 留下的样子，读它的 run 读到的可能落后于干线；
  它们不进提交，因为 `land` 与检查点只按 `scopes` 暂存，scope 之外的索引项就是分支头，提交的树在 scope 之外与分支头逐项相同。
  索引与分支头一致是这条的关键：路径收窄的检出只改 scope 内的索引项，scope 外的索引项若留在上一次 run 的提交上（分支刚随干线快进），下一次按 scope 暂存的提交会把干线在别处的改动悄悄退回去；若是某次 run 在 scope 外暂存过、却没有献出的路径，它会搭下一次献出进提交。所以重读的条件是「索引写成的树不是分支头的树」，而不是「分支动过」：后者漏掉第二种。先比后读，因为整份重读要走整棵树，而分支与索引都停在上一次 run 留下之处的再领最常见；比较用 `Index::write_tree`，索引的树缓存完整时它不必哈希就给出根。
  否决 skip-worktree 位：所用 git2 版本所带的 libgit2 在 `Index::update_all` 里不认这一位，置了位、盘上又没有的文件会被记成删除。
  否决首次放置也只检出 scope：`Worktree::add` 总做一次全量检出，所用 git2 版本没有把 `checkout_options` 暴露成安全接口，而本 crate 禁 `unsafe`；重开的条件是 git2 暴露它。首次放置不在人等的时候检出，靠的是备树（8-35），不是缩小检出。
  进程在 run 中途死掉会留下锁：一个城只有一个写者，所以新写者一拿到 `JsonlLedger` 就由 `lift_abandoned_leases` 解开全部锁——此时任何锁都不可能属于活着的 run。
  `E_WORKTREE_BUSY` 因此恒表示「锁着」，也就是有人正在用；登记在册但目录不存在即 prune 后重建，与 index 的「存疑即重建」同一反射。
  否决「释放即 prune 并删目录」：它让同一节点的下一次 run 重新量整个城并全量检出，代价随城的大小涨，而节点的分支本来就留着。
- **开城清扫上一次服务留下的树（`storage::worktree::sweep`）**：租约的树在一次服务期间留在盘上给同名的下一次领用；开城时没有任何 run 持有树（账本的独占锁），
  所以此刻城自己造的每棵树都是上一次服务留下的。`bin::assembly` 开城时先 `lift_abandoned_leases`、再以空的 `held` 调 `sweep_abandoned`，所以保留的树只省下同一次服务里的再次检出，跨一次重启就重新全量检出；
  清扫的理由是崩溃：进程死在一轮中间时，`.git/worktrees/<name>` 的登记、`.sprawling/worktrees/<name>` 目录（最多 `WORKTREE_MAX_BYTES`）与分支 `<name>` 永远留着。
  `sweep_abandoned` 收三样，每样只收城自己造的：登记的路径在盘上解析后恰是本城的 `.sprawling/worktrees/<name>`（公共 git 目录列出共用它的每座城与每个链接检出的树，只比路径尾部会把别处 `.sprawling/worktrees/<name>` 下的树连文件一起删掉；人用 `git worktree add` 加的树在别处，不碰；树目录已被删时解析它的父目录）；
  `.sprawling/worktrees/` 下没有登记的目录（整个子树是城的机器）；与被收登记同名、且尖端已被 HEAD 包含的分支——尖端带着 HEAD 没有的提交时，
  那是一轮已经 land 的活（PR 的 commit 就在它上面），分支留下；人把它检出成当前分支时，它已是人的，也留下。`held` 里的名字一概不动，那是活着的 run 手里的树。
  `refs/sprawling/runs/` 下的检查点引用不在清扫范围里：回收站靠它们让被删文件的提交躲过 `git gc`（§8-8）。
  备树（`+spare`，8-35）也不在清扫范围里：它不是任何节点的树，`live` 不列它，没有登记的 `+spare` 目录也不是 `WorktreeName`；它是给下一次放置的库存，留到下一次服务照样接管。
  清扫在开城时做，因为账本的独占锁（§8-1）保证那一刻没有别的进程在用这座城——这把锁只罩本城，所以别城的树靠上面的精确路径比较排除，不靠锁；被否：在 `claim` 里顺手清——`claim` 只遇得到它要领的那个名字，别的节点留下的树它碰不到。
- **同名再领即 `E_WORKTREE_BUSY`**；能否定义掉：能，但尚未做——当「领节点」本身变成取租约（`storage::queue` 已有队列），busy 就从错误变成排队。在那之前它是一条拒，不是一个静默的第二棵树。
- **路径不入历史**：`worktree_opened` 载荷只携 name 与字节数。绝对路径是一台机器自己的事实，写进账本会使一本能搬到另一台机器的历史带上搬不走的东西。
- **merge 只走 fast-forward**：trunk 在节点分枝之后动过即 `MergeStale`（→`E_VERSION_CONFLICT`），不由机器把一份活重放到别人的活上面——能说出「这份活是否仍然适用」的是做它的人。拒后城内文件逐字节不变（一条断言）。
- **合并不覆盖城市目录**：`plan_merge` 以 libgit2 SAFE 策略对节点 tree 做一次 dry-run 检出，基线是当前干线；人改过未提交、且合并要改或删的被跟踪文件，以及挡在新路径上的未跟踪文件，都是冲突，合并以 `MergeWouldDiscard { paths }`（→`E_VERSION_CONFLICT`，恢复：提交或挪开这些改动再合）拒绝，列出全部路径。`apply` 先写不挪指针的 merge commit，再以 SAFE 检出，最后用 compare-and-swap 把干线移到它上面；从不强制检出。不选「只移分支、不碰目录」：那样城市目录与干线不一致，下一次 checkpoint 会把节点的改动当成人撤销了它们。
- **落地的形状是真合并提交**：`apply` 造一个
  **双亲**提交（trunk 当前提交在前、节点提交在后），树取节点的树，消息为
  `<subject>\n\n<trailers>`。**判定不受此影响**——仍然只在 fast-forward 时才允许，
  refusal 仍在 `plan_merge` 里发生；合并因此在
  `git log` 里是一件事，而不是一次无声的指针移动。否决纯指针移动，因为
  那样合并没有自己的消息，也就没有地方挂 trailers 与 `Reviewed-by:`。
- **`Reviewed-by:` 只在两件事同时成立时出现**：调用方传了 `reviewed_by_person: true`，
  且运行中的机器的仓库 git config 里同时有 `user.name` 与 `user.email`。人的名字是人的，
  城不替人编一个。
- **空仓即拒并说出原因**：worktree 从一个提交分枝，而新城在首次 checkpoint 之前没有提交；本模块恒不自建创世提交（那是 `checkpoint` 的职责，两个写入者就是两个权威）。

**合并拆成决定与动作**：

```rust
impl Worktrees {
    pub fn plan_merge(&self, name: &WorktreeName) -> Result<PlannedMerge<'_>, StorageError>;  // 只读；全部拒绝在此
}
pub struct PlannedMerge<'a> { /* 私有：trees、target */ }
impl PlannedMerge<'_> {
    pub fn commit(&self) -> String;               // 干线将指向的 commit，供那条行写
    pub fn apply(self, landing: &Landing<'_>) -> Result<(), StorageError>; // 移动干线并检出；`PlannedMerge` 无第二来源
}
```

`plan_merge` 只读，全部拒绝都在它里面；`apply` 只能从它拿到。**判定的输入在动世界之前就全部齐了**——`theirs` 就是分支尖，`ours` 就是干线尖，两者都读得到，所以「这一合并会落在哪个 commit」与「它可不可以合」都能先答，装配层得以按 §8-24 的规矩落行在前、动世界在后。

形制与 `accounting::effect` 的 `Landing`／`Then` 同源：动作只能从决定里拿到，写反顺序等于去取一个取不到的值。
-/

/-!
### 8-31 `storage::worktree` 的文件操作计数：放置与再领各动了多少文件（形状 2 值）

```rust
pub struct FileWork { pub created: u64, pub rewritten: u64, pub removed: u64, pub walked: u64 }
impl WorktreeLease { pub fn work(&self) -> FileWork; }   // 领这棵树花了文件系统什么
```

- **四个数各数一件事。** `created`：在没有文件的名字上写下的文件；`rewritten`：因为与该有的内容不同而改写的文件；`removed`：删掉的目录项，一个目录连同里面的东西算一项；`walked`：读过的目录项。新建目录不计，因为实时扫描等的是文件。`FileWork` 也是 runtime 沙箱副本的计数（`crates/runtime/Spec.lean` §8-13-2），一个值、一处定义。
- **门是计数，墙钟只是读数。** 放置一棵 512 个 16 KB 文件的树 p50 约 0.9 s，等的是实时扫描对新建文件的放行，不是磁盘，也不是本 crate 的计算；墙钟随扫描器与机器变，计数不变。`trees::kept::tests` 的 `claiming_a_tree_costs_what_it_places_and_restores` 在 N 与 2N 个文件的城上各领一次、再领一次，断言四个数；墙钟与 RSS 以毫秒、MiB 记进 `budgets.toml`，不设门。
- **放置（新建一棵）。** 称城的工作树一遍（`walked` 是城的目录项数，`.git` 与 reserved 两个不下探的目录各算一项），全量检出（`created` 是树里的文件数，子模块不算），`rewritten` 与 `removed` 为 0。树的 `disk` 取检出刚写下的索引：libgit2 每写一个文件就 lstat 它，把大小记进索引项，所以各项大小之和就是量出来的字节，不必再走一遍新树。被否：检出后再 `measure` 新树——多读一遍 N 个目录项，而新树里只有检出写下的文件。索引项的大小是 32 位（git 索引格式），4 GiB 及以上的单个文件记成取模后的值，`disk` 随之偏小；`disk` 只进 `worktree_opened` 的读数，不参与任何判定。
- **再领（留着的树）。** `created`、`rewritten`、`removed` 取 scope 内检出的通知（`CheckoutNotificationType::UPDATED` 与 `UNTRACKED`，libgit2 在改盘之前逐项告知）：目标没有的已跟踪文件算 `removed`，盘上没有的算 `created`，其余算 `rewritten`，被删的未跟踪项算 `removed`。所以一次什么都没变的再领这三个数都是 0，一个 run 在 scope 里改了一个文件、多留了一个文件，下一次再领就是 1、0、1。`walked` 是 `disk` 那一遍全树称重，随树的大小长：树里可能有 scope 之外、上一次 run 留下的未跟踪文件（构建产物），只有走一遍才量得到。取消这一遍要么让 `disk` 只报已跟踪的字节，要么把它改成估计值，两者都改了 `worktree_opened` 的含义。**重开参数**：`disk` 不再需要是量出来的值时。
- **首次放置的树是整棵的。** git2 不把 `checkout_options` 暴露成安全接口（8-9）；而且 run 会读 scope 之外的文件，`exec` 里的编译器经真实文件系统读依赖，按需放置必须覆盖这些读取，缩到 scope 会让它们读不到。整棵树的文件在备树时写下（8-35），放置接管备树时只写干线自备树以来改过的文件；没有备树可接管时放置全量检出，`created` 是树里的文件数。沙箱副本的缩减另见 `crates/runtime/Spec.lean` §8-13-2、runtime D10。不用硬链接把树「放」成共享文件：共享可写文件的两棵树不是彼此隔离的两棵树，一边的写入会出现在另一边，而且链接计数大于 1 的文件在 `alias` 那里一律拒写（8-25）。
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
