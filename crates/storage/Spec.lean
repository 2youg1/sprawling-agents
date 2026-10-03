-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.storage.spec.Alias
import crates.storage.spec.Attribution
import crates.storage.spec.Blob
import crates.storage.spec.Bundle
import crates.storage.spec.Cas
import crates.storage.spec.Cas.Ranges
import crates.storage.spec.ChainAudit
import crates.storage.spec.Changes
import crates.storage.spec.Checkpoint
import crates.storage.spec.Checkpoint.Provenance
import crates.storage.spec.DigestCache
import crates.storage.spec.Error
import crates.storage.spec.FaultFs
import crates.storage.spec.Hot
import crates.storage.spec.Hunks
import crates.storage.spec.Index
import crates.storage.spec.Jsonl
import crates.storage.spec.Jsonl.Barrier
import crates.storage.spec.Jsonl.Preallocate
import crates.storage.spec.Jsonl.Verify
import crates.storage.spec.Queue
import crates.storage.spec.RealFs
import crates.storage.spec.Sessions
import crates.storage.spec.Snapshot
import crates.storage.spec.Snapshot.Start
import crates.storage.spec.Status
import crates.storage.spec.Vfs
import crates.storage.spec.Worktree
import crates.storage.spec.Worktree.Back
import crates.storage.spec.Worktree.Trees.Stock

/-! # storage 的规格

`sprawling-storage`（库名 `storage`，目录 `crates/storage`）是城的持久化：kernel Ledger 的落盘实现与它的逐行检查、内容库、旁挂索引与热视图、git 检查点与工作树、导出与恢复、链的证明与快照。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。接口一节一节写在规定它的那个模块的分部里，每一节保留它的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`；本文件 §8 列出每个标签住在哪个分部。标签被 `architecture.toml` 模块图的旧锚点与别的规格里的引用锚住，所以不重排：8-6 是空号。决定写作 `D<n>`，放在它所管的声明正上方，或它所管主题的那个分部的末尾，别处引作 `storage D<n>`；§12 末尾列出每条住在哪里。

能写成定理的规则在分部里证明，Lean 模型是「必须守住哪些性质」的权威，Rust 代码是「怎样守住」的权威：逐行检查与链（`spec/Jsonl/Verify.lean`）、屏障与重开（`spec/Jsonl/Barrier.lean`）、预分配段的段尾零（`spec/Jsonl/Preallocate.lean`）、已命名对象恒不腐蚀（`spec/Cas.lean`）与范围读（`spec/Cas/Ranges.lean`）、快照与已验证前缀（`spec/Snapshot.lean`）、停机值与证明的波（`spec/ChainAudit.lean`）、工作树的租约（`spec/Worktree.lean`）、接管备树的断点（`spec/Worktree/Trees/Stock.lean`）、回到过去（`spec/Worktree/Back.lean`）、每个错误答哪个码（`spec/Error.lean`）、按 seq 往后读索引（`spec/Index.lean` §8-38）。其余分部只有节注释：它们写的是接口的形状、取舍与被否的备选，由 Rust 的类型与各模块旁的测试守住（§16）。
-/

/-! ## 1 需求分解

| 模块 | 形状 | 一句话 |
|---|---|---|
| `jsonl` | 4 适配器 | kernel Ledger 的落盘实现：组提交＋断尾＋版本方向判定＋分段滚动 |
| `vfs` | 3 端口 | 内缝 `trait Vfs`：本 crate 触碰文件系统的唯一一张脸 |
| `real_fs` | 4 适配器 | Vfs 的生产适配器：std::fs，持住正在追写的那个句柄 |
| `error` | 2 值 | `StorageError` 与 `into_ax`：本 crate 失败词汇的唯一定义点 |
| `fault_fs` | 4 适配器 | Vfs 第二适配器：撕裂写／乱序持久化／rename 中断；断电点阵的驱动器 |
| `cas` | 4 适配器 | BLAKE3 寻址存储：范围取回＋临时文件 rename＋去重 |
| `index` | 7 projection | Ledger 旁挂索引 seq→（段，偏移）；可弃，损坏即重建；持 `Box<dyn Vfs>`，只读不写 |
| `reserved` | 2 值 | `outside_reserved(relative)`：哪些字节属于人；checkpoint 与 worktree 共用的唯一谓词 |
| `hot` | 7 projection | 内存热视图：界面查询在此命中，不读盘 |
| `attribution` | 7 projection | 成本归因：逐维度精确分割同一总额，各维度之和对账 |
| `checkpoint` | 4 适配器 | git2 波前 add -A＋波后补记＋staged diff secret 扫描 |
| `queue` | 7 projection | 一份实现服务三队列；admit 先于入队，去重先于副作用 |
| `digest_cache` | 4 适配器 | 内容哈希→摘要 Artifact；同哈希终生一次 |
| `alias` | 2 值 | 别名族（junction／symlink／硬链接）与每扇写门收的已清空写目标 |
| `sessions` | 7 projection | 每个 room 一份可弃的投影文件，在 Ledger 落盘之后写 |
| `bundle` | 4 适配器 | 导出与恢复；清单即完整性检验 |
| `chain_audit` | 7 projection | 从创世逐段走完整条链，断链时让写者停机 |
| `snapshot` | 7 projection | 一次折叠在某一行之后持有的东西，以那一行的链哈希命名 |
| `worktree` | 4 适配器 | 一个节点一棵工作树，对象共享而文件不共享 |
| `blob` | 4 适配器 | 一次提交里一个文件的字节，从不读工作树 |
| `changes` | 4 适配器 | 两个 checkpoint 之间动了什么：路径与计数，不含补丁文本 |
| `hunks` | 4 适配器 | 两个 checkpoint 之间一个文件的补丁文本，凭证形状的行被扣下并点名 |
| `status` | 4 适配器 | 还没被检查点收走的改动：分支、它与上游的差距、与某次提交不同的文件 |

每一行的模块由 `spec/` 下规定它的分部写明（§8 的表）；`reserved` 由 `spec/Worktree.lean` 规定，它是工作树与检查点共用的谓词。
-/

/-! ## 2 验收标准

- `JsonlLedger` 过 kernel conformance 六断言（含确定性双灌对拍）。
- proptest：任意 draft 序列落盘后，尾部任意字节级破坏（截断/追加垃圾）→ 重开＝最长合法前缀＋一条 `log_truncated`，链续可验，续写不断链。
- 读 `tools/fixtures/ledger-ahead/` 高版本夹具 → 方向感知拒绝（报「由更新版本写成」＋原始路径），恒不部分解读。
- 断电于 EventRecord 落账与断电于 CAS rename 各恢复一次（FaultFs 点阵驱动）。
- CAS：put→get 往返；范围取回（L/B 两式）；去重（同内容二次 put 不二次物化）；断电只留 `tmp/` 半成品，已命名对象恒完好。
- 形状 7 的 proptest 骨架一次两实例化：index 损坏即重建且查询结果不变；hot 折同一条流两次得同一份读数。
- 各维度归因之和恒等于同期权威计费额之和（逐维度断言，整数精确无余无溢）。
- 检查点：波前 checkpoint_committed 携 oid；波后删除逐条补记 file_discarded{restoration=Tracked(波前 oid)}；含 secret shape 的 staged diff 拒提交（E_SECRET_EGRESS，恒不回显字节）；queue 去重先于副作用＋shed 不丢已入队项；digest_cache 同哈希二次 put 幂等。

分部里的定理是模型对性质的证明：

- `spec/Jsonl/Verify.lean`：`advance` 收下的一行恰好接上链（`an_accepted_line_extends_the_chain`）；版本超前在一切链检之前答出（`a_line_from_a_newer_writer_is_refused_before_any_chain_check`）；`ig:true` 的未知 kind 照样入链（`an_ignorable_line_joins_the_chain`）；从创世走完而不拒的账本满足 kernel 的 `Chained`（`a_walked_ledger_is_chained`）；`open` 只截掉不带信封的字节，留下的是逐行收下的前缀（`truncation_drops_no_enveloped_line`、`truncation_keeps_a_walked_prefix`）。
- `spec/Jsonl/Barrier.lean`：句柄答过 `Ok` 的每个 seq，在任意一串波与退回（`jsonl::unwind`）之后重开都在（`answered_survives_reopen`），不守屏障有反例（`withoutBarrier`）；从已验证前缀起重开与从头重开相同（`reopenFromVerifiedPrefix`）。
- `spec/Jsonl/Preallocate.lean`：预分配段尾的零不改变尾段扫描的结论（`trailing_zeros_change_no_scan`），撕裂照样被截（`a_tear_before_zeros_is_still_truncated`），夹在中间的零读作撕裂（`zeros_before_a_line_are_not_the_end`）。
- `spec/Cas.lean`：临时件名对写者单射时，任何交错的 `put` 与崩溃之后已命名的对象恒不腐蚀（`named_objects_never_corrupt`），只按内容哈希命名有反例（`shared_temporary_names_corrupt_an_object`）；一次 `put` 名下恰是它的字节，二次 `put` 不写（`a_put_names_its_bytes`、`a_second_put_of_the_same_bytes_writes_nothing`）。
- `spec/Cas/Ranges.lean`：短答即越界的实现与区间规格相同（`a_short_answer_is_out_of_bounds`），答出来的不夹取（`never_clamps`）。
- `spec/Snapshot.lean`：快照加尾部就是全量折叠（`snapshotPlusTailIsWhole`、`resumeIsWhole`、`twoCutsOnePass`）；带记录的证明等于逐行核对（`cachedVerifyIsStrict`），入口与版本缺一不可（`withoutLinkAcceptsSplice`、`withoutVersionAcceptsStale`）；按波先算摘要，判定不变（`wavesAreCached`、`wavesAreStrict`）。
- `spec/ChainAudit.lean`：第一个判定有效（`the_first_verdict_stands`）；写者的准入随判定与停机值的造法（`admission_follows_the_verdict`）；波数只取决于段数（`waves_depend_only_on_the_segments`）。
- `spec/Worktree.lean`：锁着的树拒第二次领（`a_held_tree_refuses_a_second_claim`）；还了的树再领不重新放置（`a_released_tree_is_claimed_again_without_a_placement`）；开城之后只剩备树（`opening_a_city_leaves_only_the_stock`）。
- `spec/Worktree/Trees/Stock.lean`：接管在任何一步之后断掉都收得回来（`every_cut_is_recovered`）；备树被拿走只成一次（`the_stock_is_taken_once`）。
- `spec/Worktree/Back.lean`：回到过去的树恰在那一点、拒绝活树、只动自己的树、账本只增不减（§8-27）。
- `spec/Index.lean`：从某个 seq 往后读，答的是索引里不小于它的每一条（`seqs_from_answers_the_held_seqs_at_or_after`），与从头读再跳过前面的相同（`seqs_from_is_the_walk_past_the_smaller`），答案不取决于它之前的任何一格（`seqs_from_reads_no_slot_before_its_start`）。
- `spec/Error.lean`：账本介质的失败答 `E_STORAGE_FATAL`（`ledger_failures_stop_the_writer`），只有两个变体原样交出它们带着的错误（`only_two_variants_carry`）。

每个模型都带一个可实现的正常路径（走得通的两行账本、一次没有打扰的 `put`、恰触界的范围、没有断掉的接管、Lean 里的 `example`），所以这些保证不是从一个无法满足的前提推出来的。生产实现与模型的对应由 §16 列出的 Rust 测试检查；一条 Lean 定理证明的是模型，不是 Rust。
-/

/-! ## 3 假设与歧义

1. **组提交的形态**：「批＝上一次 fsync 期间到达的 append 量」这条预设并发到达；城只有一个写者——记账线程——所以批＝一次 `append_all` 交付的入账波：`accounting::worker::relay` 把各驱动线程送来的 draft 攒成一波，一波一屏障。trait 的单条 `append` ＝单元素波。
2. **断尾扫描范围**：append-only 事故只伤尾部——只完整校验最后一段，并以前一段末行验跨段链续；更早段的全链校验归 `chain_audit` 的证明（8-30）。断链处只有在断链行本身不能解析、且它后面没有任何能解析的完整行时，才算撕裂的尾巴而截断；断链行能解析，或它后面还有能解析的完整行，就是分叉或段中损坏（例如两个写者各自续了同一个 `prev`），拒开并指出行号，不截断——截断会静默删掉另一个写者完整、合法的历史。撕裂只留下不带换行的半行或不成记录的垃圾，所以这条线不会把真正的断电尾巴误判成分叉。
3. **目录 fsync**：POSIX 上新建/rename 后同步父目录；Windows 无目录句柄同步原语，`sync_dir` 为显式 no-op。断电点阵在 FaultFs 模型层保持严格语义（未 sync_dir 的目录项不存活），使代码纪律跨平台一致；「fsync 返回但未落盘」只能在真机断电下复验，本 crate 的测试不覆盖。
4. **存储写失败码**：`append` 的 Io 失败映射 `E_STORAGE_FATAL`（装载期码，进程级 fatal）。
5. **硬链接在两个平台上都由链接计数判定，都在构造点字面拒绝。** junction 与 symlink 都是重解析点，`file_type().is_symlink()` 对两者同真，故合为 `AliasKind::Link`。硬链接是链接计数大于 1 的普通文件（`AliasKind::HardLink`）：Unix 读 `std::os::unix::fs::MetadataExt::nlink`；Windows 的 `std` 稳定面读不到链接计数（`MetadataExt::number_of_links` 属未稳定特性 `windows_by_handle`），本 crate 打开一个不申请任何数据访问（`access_mode(0)`）、带 `FILE_FLAG_OPEN_REPARSE_POINT` 与 `FILE_FLAG_BACKUP_SEMANTICS` 的句柄，再调 `winapi-util` 的 `file::information(&File)?.number_of_links()`；unsafe 在那个 crate 内部，它本已在锁里（经 `same-file`），采用它只多一条依赖边、不多一个包。不申请数据访问的句柄不与别的句柄的共享模式冲突，所以只读文件与被别的进程独占打开的文件照样读得到计数（8-25 的读数）；带 `FILE_FLAG_OPEN_REPARSE_POINT` 是为了不跟进链接，而链接在读计数之前已由 `symlink_metadata` 判成 `Link`。判定与落盘之间的替换窗口仍在，由落盘纪律（8-12 的暂存文件再 `rename`）罩住。被否：为读链接计数手写 FFI（本 workspace 的 `unsafe_code` 是 forbid）；夜间特性（撞稳定工具链定规）；Windows 上维持不报拒、只靠写入落新 entry（D13）。**重开参数**：Windows `std` 出现稳定的 `number_of_links` 时换用它并删掉这条依赖；Unix `nlink` 语义变化时改 `kind_at` 一处。
6. **服务之外的进程里，`first_seen` 的那一次折叠仍在写者线程上。** 服务中的城把切片交给视图线程（8-24），那一次整段读取不再占记账线程；`resume`、`fork`、`adopt` 与一次性命令的写者没有视图线程，第一次遇到缺文件的切片时仍在自己的线程上整段读取各段。候选是由 `LedgerIndex` 给出每个地址的第一条 seq（索引已按段折过每一行）；判定它的证据是一次 `resume` 在 40 万行账本上首次写新 room 的时延。
7. **合并的检出先于干线的比较并交换。** `worktree::trees` 的 `apply` 先把节点的树写进城的工作目录，再用 `reference_matching` 移动干线；两步之间干线若被别处移动，比较并交换失败，工作目录却已是节点的树，下一次 checkpoint 会把这份差读成人的编辑。开城时账本的独占锁（8-1）使同一座城只有一个写者，所以窗口只在一个进程内的两次合并之间打开。候选是先做比较并交换、再以旧干线为显式基线检出（`CheckoutBuilder` 的 baseline），或比较并交换失败时撤回检出；判定它的证据是一个在两次合并之间移动干线的 citysim 场景。
8. **取回一个文件抄的是被替换文件的权限，不是 point 上那一项的 `filemode`。** `worktree::back` 的 `restore_file` 经 `bundle::landing::land`（`Bits::OfReplaced`）落盘，所以在 Unix 上，point 上可执行、树里此刻不可执行的文件取回后仍不可执行；被删后再取回的文件取新建默认值。Windows 没有可执行位，不受影响。改法是由 `entry.filemode()` 推出权限，需要给 `landing::Bits` 添一臂；判定它的证据是一个 Unix 上取回脚本的场景。
9. **`checkpoint::commit` 的 family 1 只做了一部分。** `file_discarded` 的载荷仍由 checkpoint 手工拼成 `Map`，`taint_promoted`、`cross_building_transfer`、`secret_egress_blocked` 三种还没有各自的结构体。未定的是这些结构体住在 kernel 的事件表旁边还是 storage 里；判定它的证据是它们的第二个写者出现在哪个 crate。

模型自己的假设写在各分部的定理假设里，不写成公理：摘要函数是参数，单射性归 `crates/kernel/spec/Ledger.lean`；临时件名对写者单射是 `named_objects_never_corrupt` 的前提（D1 的命名给出它）；已验证前缀的记录只由逐行核对写下是 `cachedVerifyIsStrict` 的前提（§8-30 的「只有持写者锁的进程写记录」给出它）。
-/

/-! ## 4 现状分析

热路径＝append（chain_hash＋write＋fsync）；fsync 主导，BLAKE3 与 serde 开销可忽略。
-/

/-! ## 5 权威信源

落盘形态/断尾/版本方向/组提交/分段；CAS 三工程事实；FaultFs 三故障与断电四注入点；确定性；断电与存储边界；ARCHITECTURE.md §4（内缝 Vfs 不升真缝）与 §12 的 storage 段；`crates/kernel/Spec.lean` §8-4/§8-9。
-/

/-! ## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，类型定义住在簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。**本 crate 同例**：`storage` 六面切目录后，同一类型的 inherent impl 可以住不同簇文件（如 `Checkpoint` 的两个 impl 块），公共面不变。

Vfs、RealFs、FaultFs、FaultPlan、power cut、tail-truncation recovery（断尾恢复）、direction-aware refusal（方向感知拒绝）、segment（分段）、group commit（组提交）、CAS、dedup。crate 根错误 `StorageError`（每 crate 一根，跨界映射 AxError 不透传）。

Lean 里的名字与 Rust 的对应：

- `Storage.Jsonl.Verify.LineCheck`／`advance`／`LineFault`／`CheckedLine` ↔ `jsonl::verify` 的同名类型与 `LineCheck::advance`；`walk` ↔ 读者逐行调 `advance` 的那一遍；`scan` 与 `dispose` ↔ `open` 第 ④ 步的尾段扫描与它对故障的处置；`Raw.enveloped` ↔ `LineCheck::carries_envelope`。
- `Storage.Cas.step` 的四步 ↔ `Cas::put` 的「已存在即去重返回 → 写临时件＋`sync_data` → `rename`」与开句柄时清别的进程的临时件；`Storage.Cas.Ranges.byRead` ↔ `cas::ranges::of_object` 的 `B` 式，`readAt` ↔ `Vfs::read_at`。
- `Storage.ChainAudit.ChainHalt`／`settle`／`admit`／`Verdict`／`Before` ↔ `chain_audit` 的同名类型与方法；`waves` ↔ `ProofCount.waves`；`inWaves` ↔ `walk` 每次取至多 `PROOF_WAVE` 段的那一步。
- `Storage.Worktree.claim`／`release`／`lift`／`sweep` ↔ `Worktrees::claim`、`release`、`lift_abandoned_leases`、`sweep_abandoned`；`Standing` ↔ `trees::kept::Standing`。
- `Storage.Worktree.Trees.Stock.take` ↔ `adopt` 里的目录改名（②），`place`、`stock` ↔ 下一次放置与下一次 `Worktrees::stock` 的收回与重备。
- `Storage.Error.StorageError`／`code` ↔ `StorageError` 与 `StorageError::into_ax` 的码；`AxCode` 的构造子 ↔ kernel `AxCode` 的同名变体。
-/

/-! ## 7 模块边界

```
vfs ──声明──▶ pub(crate) trait Vfs（内缝，形状 3；不出对外接口，不升真缝）
real_fs ──实现──▶ Vfs（生产适配器；持住追写句柄）
fault_fs ──实现──▶ Vfs（第二适配器；#[cfg(any(test, feature = "fault"))]）
jsonl / cas / bundle / digest_cache ──使用──▶ vfs::Vfs ＋ real_fs::RealFs
error ◀──使用── 其余模块（StorageError 与 into_ax 的唯一定义点）
```

**`StorageError` 住独立模块 `storage::error`**：全 crate 的模块都用它，而它的多数变体描述的失败 jsonl 永远不会产生（`CasCorrupt`、`Checkpoint`、`Worktree`、`MergeStale` 等），放在 jsonl 里就是让 jsonl 替别人的失败命名。`io_err`（`StorageError::Io` 的构造子）随它住。

**Vfs 与 RealFs 分家的依据是形状而不是行数**：一条缝上的 trait 是形状 3，std::fs 的直译是形状 4，而第二适配器 `fault_fs` 是自己的文件。
一个文件同时装端口、适配器与账本，正是 ARCHITECTURE §9 说的「说不出自己形状的模块通常装着两件想分开的东西」。

**本 crate 不做什么（否定式）**：
- **`RealFs` 持住它正在追写的那个文件**。
  - `append` 与 `sync_data` **各自重开一次文件**时，每一条记录下面压着两次 `CreateFile`；持住句柄后，每条记录的代价只剩写与屏障，而写只占其中很小一部分——剩下的是屏障本身，那是盘的物理。
  - **句柄命名的是文件，不是路径**，所以 `rename`／`remove_file`／`truncate` 之前必须松手：不松手的话，重命名之后向旧路径的追写会写进**已经改了名的那个文件**，删除之后的追写会写进**一个已不存在的文件**，两者都静默。CAS 正是靠「写 tmp 再重命名」给对象命名的，断尾修复正是靠截断与删除修段的。**Windows 不会拦住你**（Rust 开文件带共享删除与重命名），所以看着的断言问的不是「这个操作能不能做」而是「之后字节落在哪个文件里」；去掉 `release` 它当场报错。
  - **剩下的是 fsync 本身**，要再压就必须跨记录合屏障，而那是契约问题不是实现问题（下两段）。
  - 否决「跨会话组提交：一个序列化写入者收集同一时间窗内各会话的 drafts」，它针对的是多个线程争一把账本锁时的车队延迟。**这座城没有那个形状可优化**：
  - 账本只在记账线程里打开且从不离开；驱动线程手里能写账本的只有 `accounting::worker::relay` 的 `Relay`，它把 draft 送到记账线程并等回执，所以「一座城一个写者」由类型持有（`crates/sprawling/Spec.lean` §8-42，ARCHITECTURE §10）。探针量的是许多线程共享一个 `Mutex<JsonlLedger>`，那是本仓刻意不采用的形状。
  - 屏障的代价与骑在它上面的记录条数无关，所以一波一屏障：`kernel::Ledger` 端口有 `append_all`（`crates/kernel/Spec.lean` §8-51），`JsonlLedger` 覆写它为整波一次写一道屏障，`relay` 按波交付。
  - 否决的是给端口加一个显式屏障动作、让 `append` 只写不同步。**它会改掉本模块的一条必要前提**：`Ok` 即已落盘，观察者也只在落盘后才听到一条——「架上不会有历史里没有的东西」靠的就是这个，而 `EventRef` 一旦在同步前发出去，它就不再指向一条已存在的历史。
- 不判定任何语义——kind 二分、载荷校验、规范字节全部来自 kernel；jsonl 只定 seq/prev 与介质。
- 不采样时钟（clippy disallowed 已看守）——`log_truncated` 的 `t` 由 open 的调用方注入；checkpoint 的提交时间同规入参。
- 不向调用方暴露分段——segment 边界、滚动阈值、文件名全为内部事务；对外只有目录（index 的 seq 寻址经 jsonl 的 pub(crate) 读面，不破此墙）。
- 派生视图族恒不成为第二历史——两视图（hot／attribution）与 queue 状态全部可删可重建，恢复逻辑恒不读它们做判定。**本 crate 不再持有任何落盘视图**：城的读面由 `accounting::views` 在内存里折，删了就重放。
- 不解读语义载荷之外的字段——各派生视图只消费已入账事件的声明字段，不反推、不补齐、不修复历史。
-/

/-! ## 8 接口先行

每一节的接口写在规定它的模块的分部里。标签照旧，按标签找分部：

| 标签 | 分部 |
|---|---|
| 8-1 | `crates/storage/spec/Jsonl.lean`、`crates/storage/spec/Jsonl/Verify.lean`、`crates/storage/spec/Jsonl/Barrier.lean`、`crates/storage/spec/Jsonl/Preallocate.lean` |
| 8-2 | `crates/storage/spec/FaultFs.lean` |
| 8-3 | `crates/storage/spec/Cas.lean` |
| 8-36 | `crates/storage/spec/Cas.lean` |
| 8-4 | `crates/storage/spec/Index.lean` |
| 8-5 | `crates/storage/spec/Hot.lean` |
| 8-7 | `crates/storage/spec/Attribution.lean` |
| 8-17 | `crates/storage/spec/Checkpoint/Provenance.lean` |
| 8-18 | `crates/storage/spec/Checkpoint/Provenance.lean` |
| 8-8 | `crates/storage/spec/Checkpoint.lean` |
| 8-13 | `crates/storage/spec/Changes.lean` |
| 8-9 | `crates/storage/spec/Worktree.lean` |
| 8-10 | `crates/storage/spec/Queue.lean` |
| 8-11 | `crates/storage/spec/DigestCache.lean` |
| 8-12 | `crates/storage/spec/Bundle.lean` |
| 8-14 | `crates/storage/spec/Error.lean` |
| 8-15 | `crates/storage/spec/Vfs.lean` |
| 8-16 | `crates/storage/spec/RealFs.lean` |
| 8-19 | `crates/storage/spec/Hunks.lean` |
| 8-20 | `crates/storage/spec/Checkpoint.lean` |
| 8-21 | `crates/storage/spec/Bundle.lean` |
| 8-22 | `crates/storage/spec/Status.lean` |
| 8-23 | `crates/storage/spec/Cas/Ranges.lean` |
| 8-24 | `crates/storage/spec/Sessions.lean` |
| 8-25 | `crates/storage/spec/Alias.lean` |
| 8-26 | `crates/storage/spec/Snapshot.lean` |
| 8-30 | `crates/storage/spec/ChainAudit.lean` |
| 8-28 | `crates/storage/spec/Snapshot/Start.lean` |
| 8-27 | `crates/storage/spec/Worktree/Back.lean` |
| 8-31 | `crates/storage/spec/Worktree.lean` |
| 8-35 | `crates/storage/spec/Worktree/Trees/Stock.lean` |
| 8-32 | `crates/storage/spec/Alias.lean` |
| 8-29 | `crates/storage/spec/Blob.lean` |
| 8-33 | `crates/storage/spec/Checkpoint.lean` |
| 8-34 | `crates/storage/spec/Jsonl.lean` |
| 8-37 | `crates/storage/spec/ChainAudit.lean` |
| 8-38 | `crates/storage/spec/Index.lean` |
| 8-39 | `crates/storage/spec/Checkpoint/Concurrent.lean` |
-/

/-! ## 9 工作流程

装配（消费者＝测试与 citysim）：构造 Vfs → `JsonlLedger::open`（断尾自愈）→ 作为 `&mut dyn kernel::Ledger` 交记录方 → 波到即 `append_all`。CAS 旁路：offload/attach 字节 `put`，Locator 携 hash 跨会话，`get/get_range` 取回。
-/

/-! ## 10 实现逻辑

1. 行终止符恒 `\n`（含末行）；chain_hash 对不含 `\n` 的行字节计算（`crates/kernel/Spec.lean` §8-9）。`.gitattributes` 的 `* text=auto eol=lf` 令索引与每个平台的检出都是 LF，夹具字节因此跨平台一致。
2. open 的段校验用 `EventRecord::parse_line`＋`chain_hash` 复算，无独立解析器（一个权威）。
3. 段内偏移不建索引（storage::index 的事）；`read_raw_lines` 全量读，消费者只有 replay/夹具/conformance。
4. `list` 排序返回＋段名零填宽度 20：字典序＝数值序，跨平台遍历确定。
5. `truncate` 后同步；整段截空直接 `remove_file`，重开容忍残留空段文件（崩溃窗口的两态都可解析，比 rename 舞步少一个中间态）。另：最后一段首行即损坏且仅此一段时，截至 0 字节＝回到新 Ledger（append 未曾返回 Ok 即无可观察效果）；非尾段损坏才是 Envelope 错误。
6. FaultFs 的 io::Error 用 `ErrorKind::Other`＋自述文本；jsonl/cas 对错误只透传包裹为 `Io{op,path}`，不吞不换。

### 两个设计

**A（选中）：Vfs 内缝＋FaultFs 注入**——故障面在文件系统语义层注入，jsonl/cas 的产品代码零测试钩子。杠杆：一套故障模型服务两模块；断电点阵是 Vfs 语义的性质，不是某模块的分支。
**B（落选）：jsonl 内置故障开关**（`#[cfg(test)]` 的注入点散布各写步）——不需要 Vfs 抽象，但故障语义与产品逻辑同居一文件，点阵无法复用给 cas，且「测试钩子进产品代码」违背「测试与产品走同一道门」。落选理由：内缝的存在证明是第二适配器，不是一句声明。

### 入窗的字节与它的代价

零字节：本 crate 恒不进 prefix；模型可见面只有经 tool_result 携带的 AxError（如 E_CAS_CORRUPT 的 three-part 拒绝），其余全部是落盘内部事务。
-/

/-! ## 11 边界枚举

空目录首开（新 Ledger）；末段恰好整段损坏（截空删段、退至前段）；首段首行即损坏（Envelope 错误——创世行不可断尾，宁停不脏）；波跨滚动边界（两段各一次 sync）；`append_all(vec![])`（no-op，Ok(空)）；同内容并发 put（各写各的 tmp，rename 到同一目标）；本进程另开句柄时别的句柄正有 put 在途（不清本进程的 tmp）；get_range 恰触界（`B` 端点＝len-1 合法）；`L` 范围起于超出总行数（越界拒）；v 低于当前（v<1 不存在，按 Envelope 拒）；夹具目录只读（A16 只读不写回）。
-/

/-! ## 12 错误处理

每个 `StorageError` 变体答哪个码、为什么它不能被定义掉，写在 `spec/Error.lean` 的 `code` 上方（D3–D12、D17）；账本被另一个进程持着（D18）、一波写坏之后的拒绝（D16）、证明之前的拒绝（D19）各住它们主题的分部。失败之后什么保持不变是各接口的一部分：`append_all` 失败时内存态不前进、盘上的段退回本波之前（§8-1）；`claim_at`、`plan_merge` 的拒绝不改城内文件、干线与分支（§8-27、§8-9）；`put` 失败只留临时件，已命名的对象不变（§8-3，`spec/Cas.lean`）。

决定的条目与它们住的地方：

| 决定 | 标题 | 分部 |
|---|---|---|
| D1 | 临时件按「内容哈希＋写者进程＋本进程的 put 序号」命名，开句柄只清别的进程写的临时件 | `crates/storage/spec/Cas.lean` |
| D2 | 文档的版本不另设存放处，进的就是内容库 | `crates/storage/spec/Cas.lean` |
| D3 | `VersionAhead`→`E_LOG_VERSION_UNSUPPORTED` | `crates/storage/spec/Error.lean` |
| D4 | `CasCorrupt`→`E_CAS_CORRUPT` | `crates/storage/spec/Error.lean` |
| D5 | `CasMissing`→`E_PATH_NOT_FOUND` | `crates/storage/spec/Error.lean` |
| D6 | `RangeOutOfBounds`→`E_INVALID_ARGS` | `crates/storage/spec/Error.lean` |
| D7 | `Io`→`E_STORAGE_FATAL` | `crates/storage/spec/Error.lean` |
| D8 | `WorktreeBusy`→`E_WORKTREE_BUSY` | `crates/storage/spec/Error.lean` |
| D9 | `MergeStale`→`E_VERSION_CONFLICT` | `crates/storage/spec/Error.lean` |
| D10 | `MergeWouldDiscard`→`E_VERSION_CONFLICT` | `crates/storage/spec/Error.lean` |
| D11 | `Worktree`→`E_STORAGE_FATAL` | `crates/storage/spec/Error.lean` |
| D12 | `Alias`→`E_OUTSIDE_WRITE_DOMAIN` | `crates/storage/spec/Error.lean` |
| D13 | Windows 上硬链接按链接计数字面拒绝，计数经 `winapi-util` 的句柄读取 | `crates/storage/spec/Alias.lean` |
| D14 | 放置接管一棵事先检出好的备树，备树在没人等的时候检出（8-35） | `crates/storage/spec/Worktree/Trees/Stock.lean` |
| D15 | runtime 的 edit 经本 crate 落盘，新建由文件系统原子地占名（8-32） | `crates/storage/spec/Alias.lean` |
| D16 | `LedgerBroken`→`E_STORAGE_FATAL` | `crates/storage/spec/Jsonl/Barrier.lean` |
| D17 | `Envelope`→`E_LOG_VERSION_UNSUPPORTED` 同族拒读 | `crates/storage/spec/Error.lean` |
| D18 | `LedgerHeld`→`E_LEDGER_HELD` | `crates/storage/spec/Jsonl.lean` |
| D19 | `Unproven`→`E_HISTORY_UNPROVEN` | `crates/storage/spec/ChainAudit.lean` |
| D20 | 已验证前缀的记录不带密钥 | `crates/storage/spec/ChainAudit.lean` |
| D21 | 证明并行的只是各段前缀的摘要，链仍按段序判 | `crates/storage/spec/ChainAudit.lean` |
| D22 | 开账本借用证明写下的记录，不另起一种记录 | `crates/storage/spec/Jsonl.lean` |
| D23 | 从某个 seq 往后读由索引给，而不由调用方跳过前面的（8-38） | `crates/storage/spec/Index.lean` |
| D24 | 屏障本身按平台各有一臂，每一臂守同一条持久语义 | `crates/storage/spec/Jsonl/Barrier.lean` |
| D25 | 一座城的检查点由多个写者同时做，每个写者一个自己的 index，共享的只有对象库与引用更新 | `crates/storage/spec/Checkpoint/Concurrent.lean` |
| D26 | 移动 HEAD 的两步在每一座城上都经同一进程里一把只护 HEAD 的锁，不按盘的种类分路 | `crates/storage/spec/Checkpoint/Concurrent.lean` |
| D27 | 城的第一个提交与收楼的基线同一种写法：对象进 mempack、一个 pack 落盘，之后才写 index、以比较后交换建分支 | `crates/storage/spec/Checkpoint.lean` |
-/

/-! ## 13 依赖选型

kernel（workspace 内层）；`thiserror`；`blake3`（经 kernel 的 chain_hash／cas 自身 hash——直接依赖，B.7 钉版）；`serde_json`（envelope 探查）。dev：`proptest`、`tempfile`（RealFs 测试隔离目录）。不引 walkdir（Vfs::list 一层足矣）。
形状 7 的重建骨架住 `tests/derived_views.rs`（`tests/` 不受 modmap 辖，与 kernel/runtime 既有集成测试同例），一次定义两次实例化（index／hot）。
`git2`（checkpoint、worktree 与读仓库的各模块；libgit2 vendored，链接例外在 `deny.toml` 登记；版本由根 `Cargo.toml` 与 `Cargo.lock` 给出）。git2 不入缝：崩溃安全委托它的事务，只测重建与孤儿清扫。

规格本身只 import 工具链的库、本 crate 的分部与 kernel 的分部（`spec/Jsonl/Verify.lean` import `crates/kernel/spec/Ledger.lean`，ARCHITECTURE.md §3 的 `depmap` 允许 storage 依赖 kernel）。
-/

/-! ## 14 硬编码声明

备树的 id `+spare`（8-35；`storage::worktree::trees::stock::STOCK`，私有常量：登记名、目录名、分支名都是它，选 `+` 是因为它不在 `WorktreeName` 的字符集里而 git 容许它；改它只需同时改本规格）；`SEGMENT_ROLL_BYTES = 64 MiB`（内部事务，非 consts_policy——对上层不可见，改它不改任何行为语义，只改文件切法）；`PROOF_WAVE = 8`（`storage::chain_audit`，证明一波读的段数，也是这一波的线程数，调用线程算一条；它与 `SEGMENT_ROLL_BYTES` 的积是证明常驻内存的上限，8 × 64 MiB；改它只改证明的墙钟、常驻内存与 `ProofCount.waves`，判定不变，8-37）；段名前缀 `ledger-`＋20 位零填；CAS 分片取 hex 前 2；tmp 后缀 `.part`。均为 pub(crate) 常量，改动随本规格。
-/

/-! ## 15 影响面

runtime::replay 读 `read_raw_lines`；citysim 夹具对拍与断电点阵消费 FaultFs；index 挂账本目录布局。trait 边界 Io 映射为 Io→E_STORAGE_FATAL。
-/

/-! ## 16 测试与约束

单测：open 六步各分支；滚动边界；append_all 原子性（注入 Io 后内存态不前进）；cas put/get/get_range/dedup。proptest：链续与断尾（任意截断点/垃圾尾）；FaultFs 点阵（cut_at_op 扫描）。夹具：A16 高版本拒读。conformance：JsonlLedger 过 kernel 六断言。约束：clippy 零告警；fault_fs 在非 test/fault 构建中零字节。

形式化的义务由证明清偿：`lake build crates.storage.Spec`（`just models` 在 `just check` 里构建全部规格），不留 `sorry`、`admit` 与 `axiom`，`cargo xtask gates spec` 检查这一点。模型与生产实现的对应由这些 Rust 测试检查，它们是行为比对，不是精化证明：

- 逐行检查与尾部恢复：`jsonl::append::tests` 的 `any_tail_damage_recovers_to_a_valid_chain`、`jsonl/open/tests.rs`、`jsonl/boundary/tests.rs` 的 `a_prior_segment_ending_in_an_ignorable_line_opens`、`jsonl/tail/tests.rs`。
- 屏障：`jsonl/barrier.rs` 的 `no_append_after_a_failed_barrier_claims_a_record_the_disk_loses`、`jsonl/unwind/tests.rs`。
- 内容库：`cas` 的测试（往返、去重、范围、同内容并发 put、别的句柄在途时开句柄）与 `fault_fs/fs/tests.rs` 的断电点阵。
- 已验证前缀与证明：`chain_audit::tests`（两种规模上的计数、`line_check_rules_pin_the_verdicts_of_a_fixed_fixture`）、`jsonl::open::tests` 的 `open_reusing`、`snapshot::tests` 与 `snapshot::start::tests`。
- 工作树与备树：`worktree::trees::tests`（`one_node_holds_one_tree_and_the_second_claim_is_refused_by_name` 等）、`trees::kept::tests`、`trees::stock::tests`（`a_placement_from_the_stock_creates_no_file_at_either_size`、`the_stock_is_no_nodes_tree_and_outlives_the_sweep`）、`worktree::back::tests`。
- 错误码：没有逐臂比对整张表的测试；`jsonl/open/tests.rs`（`E_LEDGER_HELD`、`E_LOG_VERSION_UNSUPPORTED`）、`chain_audit::tests`（`E_HISTORY_UNPROVEN`）、`snapshot::tests` 与 `worktree::trees::tests`（`E_STORAGE_FATAL`）各比对它们走到的那一臂，其余各臂由 `into_ax` 的穷尽 `match` 与 `spec/Error.lean` 的 `code` 对照着读。

没有 Lean 模型的分部，其要求由类型与 `cargo nextest run -p sprawling-storage` 的各模块测试守住：索引的列与列外行，以及 `seqs_from` 的列外行与归并（`index/fold/tests.rs` 的 map 形 oracle）、热视图与归因（`tests/derived_views.rs` 与各自的测试）、检查点与它的凭证扫描、`changes`、`hunks`、`status`、`blob`、`bundle`、`sessions`、`alias`、`queue`、`digest_cache`、`real_fs`、`fault_fs`。`TailLines` 倒着接链与正着接是同一条链，由 `jsonl/tail/tests.rs` 比对，没有在 Lean 里证明。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md §11「Specifications in Lean」：本规格的布局；它改了，分部的路径与 `architecture.toml` 里 storage 各行的 `spec` 锚点一起重看。ARCHITECTURE.md §4（内缝 Vfs 不升真缝）与 §10 第 3 条（库 crate 起线程的地方是点名的，§8-37 是其中一处）约束本 crate 的形状。
- `architecture.toml` 的模块图：storage 每一行的 `spec` 指向规定它的分部，`cargo xtask gates specalign` 检查锚点在盘上。
- `docs/glossary.md`：本规格用的词，`cargo xtask gates lexicon` 检查。
- kernel 的规格（`crates/kernel/Spec.lean`）：事件表、错误码、`Ledger` 端口与 `chain_hash` 的权威；`crates/kernel/spec/Ledger.lean`：链规则与摘要单射的假设，`spec/Jsonl/Verify.lean` import 它。它们改了，这里的逐行检查与 §8-1、§8-14 一起重看。
- sprawling 的规格（`crates/sprawling/Spec.lean`）8-91、8-101、8-122、8-144、8-145、8-154：服务中的城怎样开账本、起证明、调 `stock`，以及证明与开城的读数；`tools/xtask/budgets.toml`：本 crate 的读数与预算。
- runtime 的规格（`crates/runtime/Spec.lean` §8-1、§8-13-2）：`replay` 与本 crate 共用 `LineCheck`，沙箱副本与本 crate 共用 `FileWork`。
- 引本规格的其他规格与 rustdoc 写 `crates/storage/Spec.lean §8-n` 或 `storage D<n>`；一节换了分部，它的标签不变，引用不必改。
-/
