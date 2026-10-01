-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::checkpoint

规定 `checkpoint`、`checkpoint::base`、`checkpoint::commit`、`checkpoint::scan`、`checkpoint::scan::pathspec`、`checkpoint::scan::stage_filter`（`crates/storage/src/` 下同名的文件）。git2 波前 add -A、波后补记、staged diff 的凭证扫描、重启后的比较基准，以及从检查点取回一个文件。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-8 storage::checkpoint（形状 4；git2）

```rust
pub struct Checkpoint { /* repo: git2::Repository、last: Option<git2::Oid> —— 私有 */ }
impl Checkpoint {
    pub fn open(city_root: &Path) -> Result<Checkpoint, StorageError>;      // 无仓即 init（创世提交由 ensure_base 产）
    /// Commits once when the repository has no HEAD, and never otherwise.
    /// A worktree branches from a commit, so a city that was never
    /// checkpointed cannot lend a tree; committing on every dispatch instead
    /// would move the trunk under every request already waiting.
    pub fn ensure_base(&mut self, scope: &str, t: TimeMs, of: &Provenance) -> Result<Option<Payload>, StorageError>;
    /// Pre-wave checkpoint: add -A within scope, then a **dangling** commit
    /// pointed at by refs/sprawling/runs/<run>/<oid>. HEAD does not move.
    /// Returns the checkpoint_committed payload
    /// {oid, scope, files, model, effort} (the last two: §8-18).
    /// `files` 只列这一次 checkpoint 改动了的路径（新增、修改、删除），按字节序：
    /// 比的是暂存前后两份 index 的 (路径, blob) 对。列全部已跟踪路径会让
    /// 5,000 个文件的楼每一波往账本里写 5,000 条路径，而读者要的是
    /// 这一波碰过什么。首个 checkpoint 与 `ensure_base`
    /// 面对空 index，所以照旧列出它们提交的每一个文件。
    /// `scopes` 的每一项可以是目录前缀，也可以是一个文件：lane 在只知道
    /// 这一波写了哪些文件时只把它们交进来（`crates/runtime/Spec.lean` §8-45）。工作区里是
    /// 文件、或工作区没有而 index 记为文件的项，按字面路径 `add_path`／
    /// `remove_path`，过同一道 `StageFilter` 与同一条 ignore 规则；其余的项
    /// 生成两条 pathspec，它本身与 `<它>/*`，交给一次 `add_all`。只拿 pathspec
    /// 走两个文件时 `add_all` 要匹配整个写域，比字面暂存慢，所以文件走字面路径。
    /// pathspec 里每项仍是**字面路径**：地址文法允许 `[` `]` `*` `?`，所以这些
    /// 字节各自包进单字符类（`[[]`）再交给 libgit2，否则 `notes[1]` 会匹配 `notes1`。
    pub fn wave_pre(&mut self, scopes: &[String], t: TimeMs, of: &Provenance) -> Result<Payload, StorageError>;
    /// 一栋楼的基线 checkpoint（`checkpoint::base`）：暂存 `scopes`、扫描、提交，
    /// 全部对象先写进内存里的对象库（mempack），最后作为**一个 pack** 落盘。
    /// 无 HEAD 时提交移动 HEAD（等于 `ensure_base`），否则与 wave checkpoint 同样
    /// 悬空并挂在 `refs/sprawling/runs/<run>/<oid>`。`progress` 依次收到
    /// `Staged { files }` 与 `Packed { bytes }`。取 `self` 的所有权：mempack 装在
    /// 这个 handle 的对象库上，handle 一放下它就跟着消失，之后的 checkpoint 不会
    /// 把对象写进一个再也不落盘的内存库。index 与 HEAD（或 checkpoint ref）只在
    /// pack 落盘之后才写：之前任何一步失败（staged secret、pack 被拒、崩溃），
    /// 盘上的 index 与引用都保持原样，不会指向从未落盘的对象。
    pub fn base_checkpoint(self, scopes: &[String], t: TimeMs, of: &Provenance, progress: &mut dyn FnMut(BaseProgress)) -> Result<Payload, StorageError>;
    /// Post-wave sweep: deletions since pre_oid, each as a file_discarded
    /// payload with restoration=Tracked(file:<addr>@<pre_oid>).
    pub fn wave_post(&mut self, pre_oid: &str) -> Result<Vec<Payload>, StorageError>;
    /// The way back a `file_discarded` names: the blob at `address` in commit
    /// `oid`, written to the same path under the working tree, parents created.
    /// 不移动 HEAD，不碰 index。oid 不是提交、提交里没有这条路径（或它不是
    /// blob）、地址落在受保护的元数据子树（`Address::is_reserved`）、写盘失败，
    /// 都是 `StorageError::Checkpoint`（→ `E_WORKTREE_BUSY`）；路径上有链接是
    /// `StorageError::Alias`。
    pub fn restore(&self, address: &Address, oid: &GitOid) -> Result<(), StorageError>;
    /// 把 `scopes` 之下的工作树提交到**当前分支**（HEAD 移动），供一次评审运行
    /// 在自己的 worktree 里献出成果时使用；scope 之外的索引项原样进树。返回落地的 oid。
    pub fn land(&mut self, scopes: &[String], t: TimeMs, of: &Provenance, subject: &str) -> Result<String, StorageError>;
    /// Staged-diff secret scan; a hit refuses the commit (E_SECRET_EGRESS,
    /// positions only, never the bytes). 只扫这一次会新提交进去的
    /// blob——基线那一次仍然全扫。
    pub fn scan_staged(&mut self) -> Result<(), StorageError>;
}
```

- **工具波的检查点离开 HEAD。**
  城是围绕人已有的文件夹形成的，而每一次工具波往人自己的分支历史里写一个
  `checkpoint:` 提交时，一个被采纳的仓库每波长一格。`wave_pre` 因此写一个
  **dangling commit**（`update_ref = None`，父为当前 HEAD 提交，无 HEAD 时无父），
  再把引用 `refs/sprawling/runs/<run>/<oid>` 指向它。`git log HEAD` 因此跨波不增长，
  而 oid 可 checkout、`wave_post` 与 `storage::changes` 从 oid 工作。
- **`restore` 只写工作区那一个文件。** 还原是把人丢掉的东西放回原处，不是一次提交：
  写 index 或移动 HEAD 会让「人还原了一个文件」在他自己的分支历史里长出一格。
  地址是 `Address::is_reserved` 的（`.sprawling/`、`.git/` 等受保护子树）即拒绝，先于任何盘上动作：还原的 `restoration` 来自线上，城不拿它比对账本，而受保护子树只经 spine 与治理写门写入——与 `sessions` 跳过保留地址是同一条规则。
  `Address` 只管路径的拼写，不管盘上把它解析到哪（kernel 把链接解析交给效应层），所以路径上工作区根以下已存在的任一段是符号链接或 junction 就拒绝（`StorageError::Alias`），不跟随——这条问的是 `alias::WriteTarget::within`，写落在它放行的那个值上，与其它写门同一条链接规则；
  目标处已有文件且字节与 blob 不同时拒绝（恢复：把现有文件挪开再还原），字节相同即视为已还原；写用 `create_new`，不覆盖在检查之后出现的文件。
  还原不持 `checkpoint_gate`：与同一栋楼里正在跑的波并发时，由上面的 `create_new` 拒绝而不是覆盖。
  提交能被找到，靠的是上一条的引用；没有它，`git gc` 之后 `restore` 答「找不到提交」。
- **被否的另一条路：把检查点留在 HEAD。** 它让人的历史被机器的簿记淹没——一天的工作里
  几百个 `checkpoint:` 行，人自己的提交夹在中间找不到。留在 HEAD 唯一买到的是
  「不用写引用」，而写一个引用是一行。
- **引用名是提交自己的 oid：`refs/sprawling/runs/<run>/<oid>`，不设计数器。** 一个 run 经
  不止一个 handle 升检查点——lane 自己的检查点句柄与 `runtime::bench` 的预测网各开一个 `Checkpoint`，
  而 `file_discarded` 的还原指向哪个 handle 升起的那一道都可能。引用是这道提交**唯一的钉子**：
  账本里的 oid 不让 git 保留任何对象，`git gc` 回收一切没有引用可达的提交，回收站随之失效。
  以 oid 为名，名字的唯一性由提交本身担保，不再有第二个计数权威；同一个检查点（同树、同父、同刻）
  就是同一个提交、同一个引用，重写它是幂等的。被否：每个 handle 各自计数（两个 handle 同从 0 起，
  后写的引用强制覆盖前一个，前一道提交从此无钉）；跨 handle 共享一个计数器或借用账本 seq（都要把
  一个位置穿进拿不到它的闭包，多一条耦合，而名字只需要唯一，不需要有序——顺序由账本给）。
  **翻案条件**：哪一天引用需要表达顺序（例如按先后清理旧检查点），顺序仍应读账本，而不是回到计数器。
- **基线 checkpoint 在收楼时做，写成一个 pack**：一栋 5,000 个文件的楼，第一次派活的第一道 checkpoint 要给每个文件求哈希并写 5,000 个松散对象，每个都是一次建文件；这笔钱挪到 `sprawling adopt`（以及开城时的 `Adopt::EveryFolder`）付，写法是 mempack：对象库换成「内存库在前、盘上的对象目录作只读备用」，暂存与提交产生的对象全进内存，`Mempack::dump` 出一个 pack，经仓库自己的对象库 `packwriter` 落盘（它同时写 `.idx`）。之后 run 的第一道 checkpoint 面对的是一份暖 index：libgit2 按 stat 跳过没变的文件，只剩一次目录遍历。**被否**：收楼时直接调 `wave_pre`——每个 blob 一个松散对象，在旋转盘上是 5,000 次随机写，而 pack 是一次顺序写。**被否**：把 mempack 长期装在城的 handle 上——装上就卸不掉，之后每道 checkpoint 的对象都会留在一个没人 dump 的内存库里，进程一退就丢。
- **`ensure_base` 仍然移动 HEAD**：worktree 从一个提交分枝，城必须先有第一个提交。
- **「无 HEAD」只有两种读法**：`head()` 报 `UnbornBranch`（空仓库）或 `NotFound`（HEAD 指向的引用不存在）时才算「这座城还没有提交」，提交无父、扫描全扫。其他任何读不出 HEAD 的情形——引用文件损坏、HEAD 指向一个剥不出提交的对象——都是 `Checkpoint { op: "read HEAD" }` 错误，检查点不立。被否：把一切失败读成「无 HEAD」。那样一次读不出的 HEAD 会让检查点静默地变成一个无父的根提交，账本记下的 oid 与之前的历史断开，而没有人被告知。判定只有一处（`Checkpoint::head_commit`），提交与扫描都问它。
- **提交时间是注入时刻的整秒**：`TimeMs` 是 `u64` 毫秒，除以 1000 后恒落在 `i64` 内，换算仍走 `i64::try_from` 且失败时报 `Checkpoint { op: "stamp the commit" }`，而不是写成 1970。

- **扫改动过的 blob，不扫整棵树**：遍历 git index 里的**每一个** blob、对**整份内容**跑 `kernel::secret::scan` 且**每一次工具波都跑一遍**时，每波都付整棵树的扫描与 zlib 解压——**改了一个文件的波，付整棵树的钱**。
  - 扫描因此走 `diff_tree_to_index(HEAD 树, index)`：git 自己报出这一次提交会新写进去的条目，只有它们被读出内容并扫描。它问的是 git 而不是自己逐文件推，于是「什么会进树」由暂存与扫描共用的一个机制回答。
  - **无 HEAD 时全扫**：基线那一次没有「上一棵树」可比，扫的就是全部。
  - **守的性质不变，且这是它成立的归纳证明**：`commit` 只从 `wave_pre` 出来，而 `wave_pre` 恒先扫后提交。基例——第一次提交全扫；归纳步——第 N 次提交里未变的 blob 在它进树的那一次已被扫过，变了的这一次扫。于是**每一个进过树的字节都被扫过一次**，而「模型刚写下的密钥不会变成永久」正是这句话：密钥只能随改动进入。
  - **一个被收窄的东西，明写在此**：`kernel::secret::scan` 将来多认一种形状时，**已经提交进树的内容不会被回头重扫**——新形状从此刻起作用于所有到来的改动，但不追溯。追溯要的是一次全树重扫，而那正是这里不再每波支付的开销；真要重扫时，删掉 `.git` 让下一次波成为基线是现成的路。

- **`wave_post` 只问存在性，不问内容**：sweep 要的是「pre 提交树里的哪个 blob 从工作区消失了」，而这是一个存在问题——对树里的每个 blob 做一次 `symlink_metadata`，`NotFound` 即删除。**不碰 `diff_tree_to_workdir`**：它为每一个与树对上号的路径求哈希（libgit2 的 `git_diff__oid_for_entry`），而工作区里有城自己刚写完又改动的文件（session 投影，以及任何还开着写句柄的文件）；Windows 的目录枚举尺寸对这样的文件可以落后于句柄里的真实长度，libgit2 拿这个过期尺寸去 `git_odb__hashfd`，读到比声明尺寸多的字节使剩余计数下溢，最后把整次 sweep 拒成 `E_WORKTREE_BUSY`——写路径完全正确，读路径却对城市自己的写入过敏。
  - 被否：每波走 `diff_tree_to_workdir`。它省的是「每一波付整棵树的钱」，而那棵树是**这次检查点自己的写域**（`wave_pre` 刚逐文件走过一遍），不是全城；sweep 只报 `Deleted`，而 `Deleted` 是存在问题不是内容问题。被否：`Path::exists()`——它跟随软链，一个悬空软链会被当成删除，`symlink_metadata` 不会。
  - 输出仍然在本模块排序而不信 walk 的顺序：**这批行落账的顺序是重放要复现的东西**。
- `open` 无仓即 `init` 但**不造创世提交**（空仓是合法态；在此臆造历史会使首个 checkpoint 无法归属）。暂存只用一次 `add_all`：libgit2 把 index 与工作区比一遍（按 stat 跳过没变的文件），新增、修改、删除都在这一遍里暂存；再跑一遍 `update_all` 是把同一个写域重走一次，5,000 个文件的写域上稳态 checkpoint 的中位数因此从 104 ms 降到 71–78 ms（未优化构建，16 核、SSD，同一仪表交错测三次）。暂存规则只写在 `wave_pre` 的文档里（点名文件的 scope 走字面 `add_path`/`remove_path`，点名前缀的 scope 走字面 glob 加 `<glob>/*`）；**session 切片永不进 add**（`sessions::is_session_projection`）：它是账务线程在波中持续追加的可弃投影，一旦被暂存，git 下一次就会去读一个自己以为已经知道的文件，而一个还在长的工作区文件会让那一次读把整波拒掉（`E_WORKTREE_BUSY`）。`wave_post` 走 pre 提交树的 `TreeWalk` 比对工作区存在性，输出按路径排序（确定性）。secret 扫描在**提交之前**扫 index blob，命中即拒且只报 `path:start+len`——回显字节本身即泄漏。新增 `StorageError::Checkpoint{op,detail}`（→ `E_WORKTREE_BUSY`）与 `SecretEgress{locations}`（→ `E_SECRET_EGRESS`）。
- `open` 逐次钉仓库局部 `core.autocrlf=false`。城里的文件必须逐字节往返，而运行中的机器的 git 有可能被配成在检出时重写行尾；被重写的文件与 Ledger 里它的哈希不符，而那看起来像损坏不像设置。
- 提交身份见 8-17（而不是一个固定的 `sprawling <sprawling@local>`）；时间恒入参（git 签名时间＝t，确定性 2）；scope 外文件恒不入 add（WriteDomain 即边界，全树扫描被明拒）。**`scopes` 是一组前缀而非一个**，因为写域是一个集合：楼自己的子树，加上 `RULES.toml` 另外声明的每一条。调用方传房间而门判整栋楼时，两者之间的文件进不了任何检查点——`Changes` 因此恒空，`file_discarded` 也无处恢复；权威在本节。无变化波：wave_pre 产空提交（同树 oid，仍记 payload——链可重建优于省一次提交）。
- **写域拒绝链接穿透（junction／symlink 字面拒；硬链接臂见 8-25 与 §3.5）。** 暂存回调对每个命中路径问 `storage::alias`：任一链接使**整波拒绝**（`StorageError::Alias`），绝不跳过继续——跳过即部分捕获，`file_discarded` 的恢复地址会指向一份与自己不符的树。git 交回调的是相对仓根的路径，判别名前必须先拼上工作树根（否则问的是进程自己的目录）。保护元数据在检查点侧是**跳过**而非拒绝（`storage::reserved::outside_reserved`）：那些字节另有家（城或楼的治理、git 的对象库），与 `stage_tree` 跳过保留子树同口径；被拒的 run 拿到的 recovery 是「把链接换成普通文件后重试」，故不会卡死在自己的目录上。
-/

/-!
### 8-20 重启后凭证扫描的比较基准

进程重启后 `Checkpoint::last` 为空，`scan_staged` 退回按 HEAD 比较，而这**漏扫不了重启之前的改动**：wave checkpoint 恒不移动 HEAD，故 HEAD 只可能是 `ensure_base` 或 `land` 写下的提交——它必是最后一次 checkpoint 的**祖先**。拿祖先做基准，diff 出来的路径集是拿 checkpoint 做基准那一集的**超集**：读得更多，不会更少。代价是把已经放行过的 blob 再读一遍，安全上一分不让。

**一条断言守着它**：`a_change_made_before_a_restart_is_still_read_after_it`——立城、checkpoint 一次、写入一份带凭证的文件、丢掉 handle、重开 `Checkpoint`、再 checkpoint，第二次 checkpoint 必须以 `SecretEgress` 拒绝并报出路径而不回显字节。**翻案条件**：哪一天有一条路径能在不经扫描的情况下移动 HEAD（今天 `ensure_base`／`land` 都先扫后提交，合并提交用的是节点已扫过的树），这条推理的前提就没了，届时基准必须改回记住的 checkpoint。
-/

/-!
### 8-33 从检查点取回一个文件进城自己的工作树（`storage::checkpoint::commit`，形状 4 适配器）

```rust
impl Checkpoint {
    pub fn take_back(&self, address: &Address, point: &GitOid) -> Result<FileRestored, StorageError>;
}
```

- **换回，或删去。** `point` 在 `address` 有一个文件：经 `bundle::landing::land`（暂存再 `rename`，取被替换文件的权限）把工作树里这一处换成那一份；没有：删去工作树里这一处（本来就没有也算成功）。目录或别的东西在那里拒。与回收站那一扇门（`restore`）不同：那一扇只放回一个不在的文件，见到已有的就拒；这一扇的意思就是「不要现在这一份」。
- **门与 `restore` 相同。** 保留子树里的地址拒；`WriteTarget::within` 字面拒路径上的链接与 junction；不动索引与 HEAD——取回一个文件不是一次提交。
- **回的那一行。** 返回 `FileRestored { name: "", path, point }`，由调用方写进账本；空的 `name` 指城自己的工作树，与 `Worktrees::restore_file` 写的 run 树的名字区分。
- 验收：`checkpoint::commit::restore_tests` 的 `taking_a_file_back_replaces_what_the_tree_holds_and_removes_what_the_point_did_not_hold`。
-/
