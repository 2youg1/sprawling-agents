# storage-SPEC.md

> crate：`storage`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：十七节；按模块分章，每个模块的章号由 `architecture.toml` 模块图的 `spec` 列给出。
> 章号被那一列锚住，所以不重排：8-6 是空号。

## 1 需求分解

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

## 2 验收标准

- `JsonlLedger` 过 kernel conformance 六断言（含确定性双灌对拍）。
- proptest：任意 draft 序列落盘后，尾部任意字节级破坏（截断/追加垃圾）→ 重开＝最长合法前缀＋一条 `log_truncated`，链续可验，续写不断链。
- 读 `tools/fixtures/ledger-ahead/` 高版本夹具 → 方向感知拒绝（报「由更新版本写成」＋原始路径），恒不部分解读。
- 断电于 EventRecord 落账与断电于 CAS rename 各恢复一次（FaultFs 点阵驱动）。
- CAS：put→get 往返；范围取回（L/B 两式）；去重（同内容二次 put 不二次物化）；断电只留 `tmp/` 半成品，已命名对象恒完好。
- 形状 7 的 proptest 骨架一次两实例化：index 损坏即重建且查询结果不变；hot 折同一条流两次得同一份读数。
- 各维度归因之和恒等于同期权威计费额之和（逐维度断言，整数精确无余无溢）。
- 检查点：波前 checkpoint_committed 携 oid；波后删除逐条补记 file_discarded{restoration=Tracked(波前 oid)}；含 secret shape 的 staged diff 拒提交（E_SECRET_EGRESS，恒不回显字节）；queue 去重先于副作用＋shed 不丢已入队项；digest_cache 同哈希二次 put 幂等。

## 3 假设与歧义

1. **组提交的形态**：「批＝上一次 fsync 期间到达的 append 量」这条预设并发到达；城只有一个写者——记账线程——所以批＝一次 `append_all` 交付的入账波：`accounting::worker::relay` 把各驱动线程送来的 draft 攒成一波，一波一屏障。trait 的单条 `append` ＝单元素波。
2. **断尾扫描范围**：append-only 事故只伤尾部——只完整校验最后一段，并以前一段末行验跨段链续；更早段的全链校验归 `chain_audit`（8-27）。断链处只有在断链行本身不能解析、且它后面没有任何能解析的完整行时，才算撕裂的尾巴而截断；断链行能解析，或它后面还有能解析的完整行，就是分叉或段中损坏（例如两个写者各自续了同一个 `prev`），拒开并指出行号，不截断——截断会静默删掉另一个写者完整、合法的历史。撕裂只留下不带换行的半行或不成记录的垃圾，所以这条线不会把真正的断电尾巴误判成分叉。
3. **目录 fsync**：POSIX 上新建/rename 后同步父目录；Windows 无目录句柄同步原语，`sync_dir` 为显式 no-op。断电点阵在 FaultFs 模型层保持严格语义（未 sync_dir 的目录项不存活），使代码纪律跨平台一致；「fsync 返回但未落盘」只能在真机断电下复验，本 crate 的测试不覆盖。
4. **存储写失败码**：`append` 的 Io 失败映射 `E_STORAGE_FATAL`（装载期码，进程级 fatal）。
5. **硬链接的判定按平台分两半，两半都写在这里。** junction 与 symlink 都是重解析点，`file_type().is_symlink()` 对两者同真，故合为 `AliasKind::Link`，在每一扇写门字面拒绝。硬链接在 Unix 由 `std::os::unix::fs::MetadataExt::nlink` 判定：`nlink>1` 的普通文件即 `AliasKind::HardLink`，同样在构造点字面拒绝；Windows 上 `std` 的稳定面读不到链接计数（`MetadataExt::number_of_links` 属未稳定特性 `windows_by_handle`），本 crate 今天也不读它，所以 Windows 上硬链接臂由**写入恒落新 entry** 兜住（bundle 经 `bundle::landing::land` 写同目录暂存文件再 `rename` 覆盖，`runtime::tools::edit` 写前移除该名再建；两者都换掉目录项而不写穿旧 inode）——穿透在结构上不可能，只是不报拒。**未定：Windows 上是否改为字面拒绝。** 读链接计数有一条不写 unsafe 的路：`winapi-util` 的 `file::information(&File)?.number_of_links()`，它已经在锁里（经 `same-file`），采用它只多一条依赖边、不多一个包。代价是判定要先打开句柄：没有读权限、被别的进程独占打开、重解析点（须以 `FILE_FLAG_OPEN_REPARSE_POINT` 打开，免得跟进链接）都会让判定本身失败，判定与落盘之间也仍有替换窗口。判定它的证据是 Windows 上 `kind_at` 对硬链接、只读文件、被独占打开的文件三种情形的实测读数。被否：为读链接计数手写 FFI（本 workspace 的 `unsafe_code` 是 forbid）；夜间特性（撞稳定工具链定规）。**重开参数**：Windows `std` 出现稳定的 `number_of_links`；Unix `nlink` 语义变化时改 `kind_at` 一处。
6. **`first_seen` 的那一次折叠放在哪里未定。** 现在它在本进程第一次遇到缺文件的切片时整段读取各段（8-24），所以启动后的第一次新 room 派活仍付一遍全账本读与一段大小的瞬时内存。候选是并进启动扫描已有的那一遍流式验链（`runtime::replay::fold_ledger_dir`），或放到 lane 上预先折好；判定它的证据是 94 MB 账本上启动时长与首次派活时延的同仪表读数。
7. **合并的检出先于干线的比较并交换。** `worktree::trees` 的 `apply` 先把节点的树写进城的工作目录，再用 `reference_matching` 移动干线；两步之间干线若被别处移动，比较并交换失败，工作目录却已是节点的树，下一次 checkpoint 会把这份差读成人的编辑。开城时账本的独占锁（8-1）使同一座城只有一个写者，所以窗口只在一个进程内的两次合并之间打开。候选是先做比较并交换、再以旧干线为显式基线检出（`CheckoutBuilder` 的 baseline），或比较并交换失败时撤回检出；判定它的证据是一个在两次合并之间移动干线的 citysim 场景。
8. **取回一个文件抄的是被替换文件的权限，不是 point 上那一项的 `filemode`。** `worktree::back` 的 `restore_file` 经 `bundle::landing::land`（`Bits::OfReplaced`）落盘，所以在 Unix 上，point 上可执行、树里此刻不可执行的文件取回后仍不可执行；被删后再取回的文件取新建默认值。Windows 没有可执行位，不受影响。改法是由 `entry.filemode()` 推出权限，需要给 `landing::Bits` 添一臂；判定它的证据是一个 Unix 上取回脚本的场景。
9. **`checkpoint::commit` 的 family 1 只做了一部分。** `file_discarded` 的载荷仍由 checkpoint 手工拼成 `Map`，`taint_promoted`、`cross_building_transfer`、`secret_egress_blocked` 三种还没有各自的结构体。未定的是这些结构体住在 kernel 的事件表旁边还是 storage 里；判定它的证据是它们的第二个写者出现在哪个 crate。

## 4 现状分析

热路径＝append（chain_hash＋write＋fsync）；fsync 主导，BLAKE3 与 serde 开销可忽略。

## 5 权威信源

落盘形态/断尾/版本方向/组提交/分段；CAS 三工程事实；FaultFs 三故障与断电四注入点；确定性；断电与存储边界；ARCHITECTURE.md §4（内缝 Vfs 不升真缝）与 §12 的 storage 段；kernel-SPEC §8-4/§8-9。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。**本 crate 同例**：`storage` 六面切目录后，同一类型的 inherent impl 若住不同簇文件，基线为每个 impl 块各记一行 `impl`（如 `Checkpoint` 两行），公共面不变。

Vfs、RealFs、FaultFs、FaultPlan、power cut、tail-truncation recovery（断尾恢复）、direction-aware refusal（方向感知拒绝）、segment（分段）、group commit（组提交）、CAS、dedup。crate 根错误 `StorageError`（每 crate 一根，跨界映射 AxError 不透传）。

## 7 模块边界

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
  - 账本只在记账线程里打开且从不离开；驱动线程手里能写账本的只有 `accounting::worker::relay` 的 `Relay`，它把 draft 送到记账线程并等回执，所以「一座城一个写者」由类型持有（sprawling-SPEC 8-42，ARCHITECTURE §10）。探针量的是许多线程共享一个 `Mutex<JsonlLedger>`，那是本仓刻意不采用的形状。
  - 屏障的代价与骑在它上面的记录条数无关，所以一波一屏障：`kernel::Ledger` 端口有 `append_all`（kernel-SPEC 8-51），`JsonlLedger` 覆写它为整波一次写一道屏障，`relay` 按波交付。
  - 否决的是给端口加一个显式屏障动作、让 `append` 只写不同步。**它会改掉本模块的一条必要前提**：`Ok` 即已落盘，观察者也只在落盘后才听到一条——「架上不会有历史里没有的东西」靠的就是这个，而 `EventRef` 一旦在同步前发出去，它就不再指向一条已存在的历史。
- 不判定任何语义——kind 二分、载荷校验、规范字节全部来自 kernel；jsonl 只定 seq/prev 与介质。
- 不采样时钟（clippy disallowed 已看守）——`log_truncated` 的 `t` 由 open 的调用方注入；checkpoint 的提交时间同规入参。
- 不向调用方暴露分段——segment 边界、滚动阈值、文件名全为内部事务；对外只有目录（index 的 seq 寻址经 jsonl 的 pub(crate) 读面，不破此墙）。
- 派生视图族恒不成为第二历史——两视图（hot／attribution）与 queue 状态全部可删可重建，恢复逻辑恒不读它们做判定。**本 crate 不再持有任何落盘视图**：城的读面由 `accounting::views` 在内存里折，删了就重放。
- 不解读语义载荷之外的字段——各派生视图只消费已入账事件的声明字段，不反推、不补齐、不修复历史。

## 8 接口先行（按模块分章）

### 8-1 storage::jsonl

```rust
// Vfs 的声明住 §8-15，RealFs 住 §8-16，StorageError 住 §8-14。
// 本章只管账本本身：seq/prev 与介质。

// 公开面不带泛型：Vfs 是 pub(crate) 内缝，若 JsonlLedger<V: Vfs> 公开即私有 trait 漏入公开签名（E0445）。
// 故 Box<dyn Vfs> 内藏；生产构造子 open(dir, now) 恒用 RealFs，注入点为 pub(crate) open_with（本 crate 测试）
// 与 feature="fault" 构造子（取具体 FaultFs，trait 不出门）。
pub struct JsonlLedger { /* Box<dyn Vfs>、dir、当前段路径、段内字节数、next_seq、prev、roll_bytes、写者锁 */ }
pub struct OpenReport { pub recovered: Option<TailTruncation> }   // 断尾发生与否
pub struct TailTruncation { pub dropped_bytes: u64 }

impl JsonlLedger {
    /// Opens (or initializes) the ledger directory. `now` feeds the
    /// `log_truncated` record when tail recovery fires — time is a
    /// parameter here, never sampled (determinism rule 2).
    /// A city's ledger opens only while this process holds the city's
    /// writer lock; the lock lives as long as the returned ledger.
    pub fn open(dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), StorageError>;
    pub(crate) fn open_with(vfs: Box<dyn Vfs>, dir: &Path, now: TimeMs) -> …;  // 测试注入点
    /// Group commit: one durability barrier for the whole wave.
    /// Ok ⇒ every line of the wave is on its segment and synced.
    /// 一波的写或 sync 失败过，此后每一波都答 `LedgerBroken`，直到重开。
    pub fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, StorageError>;
    pub fn read_raw_lines(&self) -> Result<Vec<Vec<u8>>, StorageError>;   // 实例读面
    /// 写路径观察者（至多一个，后装者取代前装者）。只在**整波持久化完成后**逐条回调：
    /// 观察者因此看不到一条未落盘的事件，而推给界面的事实就是历史里的那一行。
    pub fn observe(&mut self, sink: WriteObserver);   // WriteObserver = Box<dyn FnMut(&EventRecord) + Send>
    pub fn position(&self) -> Seq;    // 现在写一条会落在哪；只给位置不给内容
}
/// 只读读面，给要原始行的读者（分叉、夹具与测试）：不走 open、不触发断尾与任何写——验证恒不修盘（runtime-SPEC §8-1）。
/// 只要结论的读者不经它：`sprawling replay` 走 `audit_chain`（8-27），折叠走 `LedgerIndex::folding`（8-4）；两者都一次只持一段字节。
pub fn read_raw_lines_at(dir: &Path) -> Result<Vec<Vec<u8>>, StorageError>;
/// 目录里的账本段，按应读顺序（`list` 已排序，段名零填充故字典序即时序）。
/// 空结果的意思是「这里没有账本」，与「账本里没有事件」不是同一件事；
/// `read_raw_lines_at` 对两者都答 `Ok([])`，故需要区分的调用方问这一面。
/// 现在只有一个：`sprawling replay`，它的路径是人敲的（sprawling-SPEC §12）。
/// 段名规则因此只住 `is_segment` 一处，不被谁再拼一遍。
pub fn ledger_segments_at(dir: &Path) -> Result<Vec<PathBuf>, StorageError>;
/// 一段的字节，只读、不走 open；`lines()` 给出该段完整且非空的行（撕裂尾不是行，留给 open 判）。
/// crate 内的面：`read_raw_lines_at` 是它唯一的调用者，完整行的规则因此只住 `complete_lines` 一处。
/// 库外的流式读者不逐段读，走 `LedgerIndex::folding`（8-4），那一遍同时建索引。三者住 `jsonl::reading`。
pub(crate) fn read_segment(segment: &Path) -> Result<SegmentBytes, StorageError>;
impl SegmentBytes { pub(crate) fn lines(&self) -> impl Iterator<Item = &[u8]>; }
/// 从账本尾部倒着读：最新的一行先出，逐段往前，段内从段尾往回按窗口读（窗口从一页起倍增，
/// 与 `first_line` 同一条规则，故一行无论多长，读到的字节至多是它自身的两倍）。
/// 只要最后 N 条的读者（`sprawling view` 的首屏）因此只付这 N 条的字节，而不是整本账本。
/// 每行过的检查与正向读者相同：信封、版本方向、kind 的有类型／可忽略之分、写者规范回显，
/// 这些住 `LineCheck::judge` 一处；链改成倒着接：这一行的 `chain_hash` 必须等于较新那行的 `prev`，
/// 它的 `seq.next()` 必须等于较新那行的 `seq`，seq 为 `FIRST` 的行的 `prev` 必须是 `GENESIS_PREV`。
/// 不合格的行以 `StorageError::Envelope` 报出，行号是链给它的位置（较新那行的 seq 值，即 1 起的行号；最新一行没有较新者，记作第 0 行），之后迭代结束。撕裂尾（最后一个 `\n` 之后的字节）
/// 不是行，跳过，与 `lines()` 同一规则。住 `jsonl::tail`。
/// 被否掉的：先 `read_raw_lines_at` 再取末尾——首屏要付整本账本的读取，正是这个读者要去掉的。
pub struct TailLines { /* 段路径（倒序）、当前段的未读偏移与缓冲、较新一行的 seq 与 prev */ }
pub struct TailLine { pub raw: Vec<u8>, pub checked: CheckedLine }
impl TailLines { pub fn at(dir: &Path) -> Result<Self, StorageError>; }
impl Iterator for TailLines { type Item = Result<TailLine, StorageError>; }
impl kernel::Ledger for JsonlLedger { /* append = append_all(vec![d]) */ }
#[cfg(feature = "conformance")] impl LedgerInspect for JsonlLedger { … }
// 测试可调滚动阈：roll_bytes 字段＋#[cfg(test)] 设定器；生产恒为 SEGMENT_ROLL_BYTES。

// 创世行撑裂且仅此一段：回到空城，OpenReport 报告但账上无 log_truncated——账本身不存在，
// 且首行必须是 city_initialized；首行可解析但链根不对＝Envelope，不静默清场。
```

**逐行检查 `jsonl::verify`（形状 2 值）**：账本的每个读者都经同一个 `LineCheck` 走一行——`open` 的尾段扫描与前段末行、`audit_chain`、`runtime::replay` 同一份，故一方收下的行另一方拒不了。

```rust
pub struct LineCheck { /* prev 与下一个期望 seq——只有链状态，不持行 */ }
pub enum CheckedLine { Known(EventRecord), IgnoredUnknown(Seq) }
pub enum LineFault { NotALine(String), VersionAhead(u64), NotAVersion(u64), ChainBreak,
                     SeqGap { found: Seq, expected: Seq }, UnknownKind(String),
                     NotCanonical(String), SeqExhausted(AxError) }
impl LineCheck {
    pub fn at_genesis() -> Self;
    pub fn expected(&self) -> Seq;
    /// 判一行（不带 `
`）；过则链前进，错则状态不动。
    pub fn advance(&mut self, raw: &[u8]) -> Result<CheckedLine, LineFault>;
}
impl LineFault { pub fn into_ax(self, line_no: u64) -> AxError; }  // 整本读者的拒词：更新的写者＝E_LOG_VERSION_UNSUPPORTED，其余＝E_CAS_CORRUPT
```

- 判定顺序：信封（`v` 经 `readable_log_v`）→ `prev` → `seq` → kind 二分（借用判定，不 clone）→ 已知 kind 的类型解析与规范回写比对；`ig:true` 的未知 kind 不解析但照样入链。
- `open` 对故障的处置：`VersionAhead`／`NotAVersion` 按版本拒；`NotALine` 且其后无带信封的行＝撕裂，截断；其余一律拒而不截——撕裂不会留下带信封的行，截掉它等于删掉合法历史。
- 被否：两个读者各持一份检查。只做类型解析的 `open` 会把一条链续正确的 `ig:true` 行在尾段当撕裂截掉，而 `replay` 收下同一行。
- 被否：前段末行只做类型解析。`EventRecord` 的 kind 没有「未知」这一臂，一条 `ig:true` 的新 kind 行若恰是前段的最后一行，类型解析拒它，城就打不开，而尾段扫描与 `replay` 都收下同一行（`jsonl/boundary/tests.rs` 的 `a_prior_segment_ending_in_an_ignorable_line_opens`）。代价：前段末行若不是写者规范的字节，`open` 现在也拒；那样的行 `replay` 本来就拒。

**落盘形态**：目录内 `ledger-<first_seq 20 位零填>.jsonl` 若干段；行＝`canonical_line`＋`\n`；链与 seq 跨段连续。滚动：当前段字节数 ≥ `SEGMENT_ROLL_BYTES` 时下一波起新段（新段创建后 `sync_dir`）。
**写者锁：一个账本目录同一时刻只有一个 `JsonlLedger`，跨进程成立。** `open` 在列段之前，对账本目录的同级文件 `<目录名>.lock`（城的账本即 `<city>/.sprawling/ledger.lock`）取 `std::fs::File::try_lock` 独占锁；`JsonlLedger` 持着那个 `File`，锁与账本同寿命，drop 即放。拿不到锁就是别的 `JsonlLedger`（这个进程的或另一个进程的）正持着这座城的账本：`StorageError::LedgerHeld { dir }`，映射装载期码 `E_LEDGER_HELD`。拒绝发生在任何读写之前，所以被拒的一方不修盘，也不写 `log_truncated`。

```rust
pub(crate) struct WriterLock { /* 持锁的 File；只为它的 Drop 而存在 */ }
impl WriterLock {
    /// 锁文件＝`dir` 的同级 `<目录名>.lock`，由 jsonl 自己命名；`dir` 没有目录名（根、`..`）即 `Io`。
    pub(crate) fn take(dir: &Path) -> Result<WriterLock, StorageError>;
}
```

- **锁文件在账本目录旁边，不在里面。** 账本目录的读者把目录里的每一项都当历史：`has_history` 问「有没有条目」，`bundle` 逐文件拷贝。锁文件放进去，就会被当成一段历史拷进 bundle，而且 Windows 上另一个句柄读一个被锁住的文件会失败。`.sprawling/` 根是保留子树，本来就不随 bundle 走。
- **每个账本目录都锁，锁路径只由 `dir` 推出。** `open_faulty` 不取锁：FaultFs 的盘只存在于这个进程里。理由见 §12 `LedgerHeld`。
- **同一进程里的第二个句柄同样被拒。** `try_lock` 在 Windows 上是 `LockFileEx`，在 Unix 上是 `flock`，两者都按打开的句柄算，不按进程算。所以「一个进程里开两个 `JsonlLedger`」也被拒，这就是跨进程性质在单进程里可测的形式（`open/tests.rs` 的 `a_second_writer_of_a_city_is_refused_until_the_first_lets_go`）。
- **锁文件不删。** 放锁时删文件，会留下一个窗口：先到者还持着旧文件上的锁，后到者已经在同名的新文件上拿到了锁。一个空文件不花任何代价。
- **锁文件不在就建，连同 `.sprawling/`。** 建不了这个文件，或者操作系统答不了这次取锁（答的不是「已被持有」），都是 `Io`→`E_STORAGE_FATAL`，subject 是锁文件的路径：连锁文件都写不了的城，也写不了账本。
- **被否：把锁放在 `Vfs` 缝上。** 那要给 `FaultFs` 造一个进程模型，而它的盘本来只有一个进程到得了。锁是真实文件系统上的事实，只在生产入口 `open` 上取。
- **被否：把 PID 写进锁文件。** Windows 上别的进程读不了被锁住的文件，拒词因此报不出 PID；recovery 改为说明怎样停下持锁的那个进程。
- **重开参数**：出现不经 `JsonlLedger::open` 写账本的生产路径；或者 `std` 的 `try_lock` 改为按进程而不是按句柄判定。

**open 六步**：①列段排序；②空目录＝新 Ledger（next_seq=FIRST、prev=GENESIS_PREV）；③读首段首行验 `v`（只读这一行：`jsonl::first_line` 用 `Vfs::read_at` 从 4 KiB 的窗口读起、每次加倍，见到第一个 `\n` 或文件尽头即停；不见 `\n` 的首行是撕裂行，答「无行」，交给尾部恢复。整段读进来再切第一行，会让单段账本在 open 时被读两遍）——判定一律经 `kernel::consts_external::readable_log_v`：`Ahead` 即 `VersionAhead`（先于一切链检，恒不部分解读），`NotAVersion`（低于任何构建写过的首版本，含 v0）即 `Envelope` 且拒词说版本，`Current` 与 `Older` 放行；④校验最后一段：逐行 parse＋段内链续，本段任一可解析行的 `v` 同样经 `readable_log_v` 判定——`Ahead` 与 `NotAVersion` 在此**拒**而不作尾损截断（截掉它等于删掉更新构建的历史），首个非法字节起截断（`truncate`＋`sync_data`）；末段的入口取自前段末行（`jsonl::boundary`）：这一行过 `LineCheck::judge`，与正向检查同一条规则——已知 kind 类型解析并比对规范回显，`ig:true` 的未知 kind 只读信封——入口的 `prev` 是它的 `chain_hash`，下一个 seq 是它的 seq 加一；判不过即拒，`VersionAhead` 按版本拒，其余为 `Envelope`，行号是它在前段里的行号；⑤若截掉字节>0（含「截空整段即删段文件」的退化情形），append 一条 `log_truncated`（run=CITY、who=`Who::City`——开账本是城自己的活，"system" 这第四种写法已删、data 由 `kernel::event::record::LogTruncated` 拼写为 `{"dropped_bytes":n}`）；⑥恢复 next_seq/prev 内存态。
**append_all 五步**：逐 draft：seq=next、`EventRecord::from_draft`、`canonical_line`、必要时滚段；写段；单次 `sync_data`（跨段波对每个触及段各一次）；更新 prev/next_seq；铸 refs。任何 Io 错误⇒整波失败，内存态不前进，盘上的段也退回本波之前（`jsonl::unwind`，形状：adapter）：当前段截回 `seg_len`，本波新建的段删去。退回本身失败时留作待办，下一波写之前先完成它；完成不了，那一波以退回的 Io 错误失败而不写一个字节。于是写失败留下的半行永远不会成为下一波的前缀，进程活着也好、重启也好，账本都是一条不断的链；open 的断尾只剩掉电一种来源。
- 被否：写失败后把账本置为只读、直到重启。它同样不写坏账本，但盘满时腾出空间以后城仍要重启才能继续记账；退回只多一次 `truncate`，而且只发生在失败的那一波。重新考虑的条件：出现一个 `truncate` 不可靠、而 open 的断尾可靠的平台。

**屏障状态 `jsonl::barrier`（形状 2 值）**：内存里的位置（`seg_len`、`next_seq`、`prev`）必须就是段的末尾，账本答出的 `Ok` 才能都在重开后的账本里找到。

```rust
pub(crate) enum Barrier { Whole, Broken }
impl Barrier {
    /// Whole ⇒ Ok；Broken ⇒ StorageError::LedgerBroken { dir, at: next_seq }。
    pub(crate) fn admit(&self, dir: &Path, at: Seq) -> Result<(), StorageError>;
}
```

- 一波在写出第一个字节之前把屏障置为 `Broken`，在它触及的每一段都 sync 完（以及新段的 `sync_dir`）之后才置回 `Whole`；`append_all` 进门先 `admit`，空波也一样。
- 原因：写到一半死掉的波会在段尾留下位置不认识的字节——撕裂的半行，或 sync 失败的整行。在它后面再写的一波，会接在下次 open 要截掉的那段字节之后，它的 `Ok` 就说了一条盘上没有的记录（`barrier.rs` 的 `no_append_after_a_failed_barrier_claims_a_record_the_disk_loses`）。能判断段尾有什么的只有 open，所以坏了的句柄拒绝之后的每一波。
- 性质「句柄答过 `Ok` 的每个 seq 重开后都在」由 `crates/storage/spec/Jsonl/Barrier.lean` 对任意一串波证明（`answered_survives_reopen`），并给出不守屏障时的反例（`withoutBarrier`）；`just models` 证明它。
- 被否：失败后把位置退回或前推到盘上真实的末尾。写失败时句柄不知道落下了多少字节，sync 失败后页缓存里的字节是否还会落盘也不知道；猜一个位置，就是用猜测替 open 的断尾恢复作答。
- 重开参数：出现后台组提交（记账线程发布「已持久到 seq N」的水位线）之后，sync 失败不再发生在 `append_all` 里，屏障状态随水位线一起搬到记账线程。

### 8-2 storage::fault_fs

```rust
#[cfg(any(test, feature = "fault"))]        // 测试与 citysim 的故障面两个消费者
#[derive(Clone)]                            // 句柄共享状态（Rc<RefCell>）：断电后以同一实例重开
pub struct FaultFs { /* files: BTreeMap<路径, FileState{durable, live, durable_entry}>、op 计数、FaultPlan */ }
pub struct FaultPlan { pub cut_at_op: Option<u64>, pub cut_on_write: Option<&'static str>, pub torn_tail: TornTail }
                                            // cut_on_write：首个字节含该串的 append 断电，一次即消费
pub enum TornTail { None, KeepBytes(u64) }  // 撕裂写：每文件未同步增量保留前 k 字节；全显式即全确定，不需种子

impl FaultFs {
    pub fn new(plan: FaultPlan) -> Self;
    /// Simulates power loss now: live falls back to durable (+ torn
    /// prefix); files whose dir entry was never synced vanish.
    pub fn power_cut(&self);
    pub fn op_count(&self) -> u64;
}
impl Vfs for FaultFs { /* 每 op 自增计数；append 先落 live 再判 cut（撕裂可咬本次写），其余 op 先判；命中即 power_cut 并报 io::Error，plan 消费后后续 op 照常（重开阶段） */ }
```

**模型三则**（比真实平台严格，故纪律跨平台成立）：①`sync_data` 前的字节不存活：durable/live 两平面，断电即 live 回落 durable，撕裂按 `TornTail` 多留未同步增量前缀；②新建文件在 `sync_dir` 前目录项不存活，断电即消失（含已 sync_data 者——比 POSIX 更严，使建段后必 sync_dir 的纪律跨平台成立）；③rename 自身原子——恒不出现半个目标文件。
**第二个旋钮 `cut_on_write`，与两个入口**：

```rust
impl JsonlLedger {
    #[cfg(any(test, feature = "fault"))]
    /// 收具体 FaultFs，故 Vfs 缝不出门（缝表不动，depmap 不动）
    pub fn open_faulty(fs: FaultFs, dir: &Path, now: TimeMs) -> Result<(Self, OpenReport), StorageError>;
}
```

`open_faulty` 返回的就是生产用的同一个 `JsonlLedger`，于是 crate 之外的调用方跑的是真代码，只丢掉它点名的那一次写。

**为什么按内容而不只按序号**：`cut_at_op` 在本 crate 内部好用，因为操作序就在眼前。**在装配层它是一个注定碎掉的数字**：要正好落在 `roadmap_claimed` 那一次 append 上得数一个魔术数，而上游任何一处多读一个文件就全盘失效。`cut_on_write` 让调用方用自己的词汇点名那一行——**它仍然完全显式、完全确定**（本模块自述「Everything is explicit… there is no randomness」，这条不破它），并且直接表达要问的那件事：假如这一行没落下。取 `&'static str` 是为了让 `FaultPlan` 保持 `Copy`；点名一条账本行用的是字面量。

**断电点阵**：以 `cut_at_op` 扫描 1..=N 全部注入点各跑一遍「写入→断电→重开→断言」；断言两条：链恒可验，**已返回 Ok 的波恒存活**（append_all 耐久契约的机器面）。断电于 EventRecord 落账与断电于 CAS rename 在此点阵上断言；checkpoint 经 git2 落盘、不经 `Vfs`（8-15），不在点阵上。

### 8-3 storage::cas

```rust
pub struct Cas { /* Box<dyn Vfs>、dir —— 非泛型，理由同 jsonl（Vfs 不得漏入公开签名） */ }
impl Cas {
    pub fn open(dir: &Path) -> Result<Self, StorageError>;           // RealFs；建目录＋清别的进程留下的 tmp/*.part
    pub(crate) fn open_with(vfs: Box<dyn Vfs>, dir: &Path) -> …;    // 测试注入点
    /// Content-addressed put: tmp + rename, dedup by existence.
    pub fn put(&mut self, bytes: &[u8]) -> Result<B3Hash, StorageError>;
    /// put＋记下这块是为哪个 run、哪栋楼存的；两者都落盘才返回 Ok。
    pub fn put_for(&mut self, bytes: &[u8], origin: &BlockOrigin) -> Result<B3Hash, StorageError>;
    /// 这块的全部来源，先存先列；没记过来源的块与没见过的哈希都是空表。
    pub fn origins(&self, hash: &B3Hash) -> Result<Vec<BlockOrigin>, StorageError>;
    pub fn contains(&self, hash: &B3Hash) -> bool;                  // 存在判定无可失败面，不包 Result
    /// Full read re-verifies the hash (cheap: BLAKE3 GB/s); mismatch ⇒ CasCorrupt.
    pub fn get(&self, hash: &B3Hash) -> Result<Vec<u8>, StorageError>;
    /// Range read per Locator semantics (L: 1-based closed; B: 0-based closed).
    /// Reads only the named bytes; trusts the object as verified at put.
    pub fn get_range(&self, hash: &B3Hash, range: &Range) -> Result<Vec<u8>, StorageError>;
}
```

**范围读只读要答的那一段，并且不校验。** 取回走 `Vfs::read_at`：`B` 式一次定位读取 `to-from+1` 字节，**短答即越界**（文件到头了，拒而不夹取）；`L` 式自对象开头按 64 KiB 块扫换行，扫到第 `to` 行的终止符即止，付的是答案**之前**的字节，从不付答案之后的字节。语义仍归 `storage::cas::ranges` 一处（`of_object`），`Cas::get_range` 只做存在判定与路径解析。

**两条路里选了「不校验，调用方明示接受」，另一条（分块哈希）落选。** 一个对象的地址覆盖整份内容，拿它校验一个片段就必须把整份读回来重算 BLAKE3——那正是本条要去掉的代价。要让片段可校验就得改写入面：put 时另存一棵分块哈希树（BLAKE3 的可验证流式形态），于是每个对象多一份旁挂物、多一条要与对象保持同步的事实、并且旧对象无树可用。买到的是「范围读能发现位腐烂」，而位腐烂**已经**由 `get` 的全读复算发现，且 §12 已把 `CasCorrupt` 记为不可定义掉的外部事故。因此：**范围读信任 put 时的校验，需要地址被证明的调用方走 `get`**；这句话在 `cas/ranges.rs` 的模块文档里逐字重复一遍，因为改那段代码的人先读的是它。

```rust
pub struct BlockOrigin { pub run: RunId, pub building: Address }
```

**块的来源在存块时记下，不事后推断。** 为一个 run 存块的调用方（转录、卸载、截图）走 `put_for`；上架的技能包是全城的，走 `put`，不带来源。来源记录在 `<dir>/from/<hex 前 2>/<hex64>`，一行一个 `<run> <address>`：同一份字节可能为两栋楼各存一次，所以一个块可以有多个来源，同一来源再存不重复记。记录追加后 `sync_data`＋`sync_dir`。读回来不成形的行（崩溃截断的追加尾）不授予任何东西——读取界往关的一侧失败。已有字节不以换行结尾时（一次崩溃截断的追加尾），新记录前先补一个换行：否则新行接在残尾后面，读回时与残尾一起被丢，而 `put_for` 已经返回了 `Ok`。另一条路是按账本里哪一行写了这个哈希来推断来源，落选：模型写的文字会落进带地址的行，那样的归属可以伪造。

布局：`<dir>/b3/<hex 前 2>/<hex64>`；临时件 `<dir>/tmp/<hex64>.<pid>.<本进程第几次 put>.part`（每次 put 一个自己的名字：同内容并发写者各自写满、各自 rename 到同一目标，无随机源；开句柄只清 pid 不是本进程的 tmp，理由见 §12；同名残留先 truncate 再写）。put 四步：hash→已存在即去重返回→写 tmp＋`sync_data`→`rename`＋`sync_dir`（分片目录）。范围取回越界＝`RangeOutOfBounds`（fail-closed，不静默夹取）；`L` 式行切分按 `\n`，末行无终止符同计一行；返回字节含行间 `\n`、不含末行终止符；`B` 式按 0 起闭区间直切。
rename 入 Vfs；FaultFs 模型：rename 原子；新目标目录项在 `sync_dir` 前不存活，断电即整体消失（源已移除）——看似比真实更损，但 put 尚未返回 Ok，无可观察效果被丢失，A3 点 2 的断言面（已命名对象恒不腐蚀）不受影响。

### 8-4 storage::index（形状 7）

```rust
pub struct LedgerIndex { /* folded: Folded —— entries（base: Seq 起点、column: Vec<u64> 隐式 seq 列，每行一字＝高 16 位段名字典 id＋低 48 位行首字节偏移、
                            outliers: BTreeMap<Seq, (字典 id, 偏移)> 列外行、segs: Vec<String> 段名字典）、runs: Vec<RunSpans>（run→连续 seq 值区间表，按 run 排序）、scanned；
                            vfs: Mutex<Box<dyn Vfs>> —— 内缝，私有（谁在缝外见 8-15） */ }
impl LedgerIndex {
    /// 扫描账本目录建索引（本就是唯一建表入口，无库外旁挂物可信）。
    pub fn rebuild(dir: &Path) -> Result<LedgerIndex, StorageError>;
    /// 一遍读史，两件事：每条完整行按账本序借给 `each`，同时按它在段里的偏移入索引。
    /// `each` 已读出这一行的 seq 与 run 时交回 `Some(Located)`，索引就不再解析它；交回 `None`
    /// 则由索引自己 `locate`（与 `rebuild` 同一规则）。常驻的只有一段字节。`each` 报错即停，
    /// 返回它的错，读盘错经 `StorageError::into_ax`。`rebuild` 就是 `each` 恒答 `None` 的这一遍。
    pub fn folding(dir: &Path, each: impl FnMut(&[u8]) -> Result<Option<Located>, AxError>)
        -> Result<LedgerIndex, AxError>;
    pub fn empty() -> LedgerIndex;                                                 // 账本目录读不出时先要一个：可弃，refresh 会填上
    pub fn refresh(&mut self, dir: &Path) -> Result<Refreshed, StorageError>;       // 只读长出来的字节
}
/// 一行在史中的位置：它的 seq，以及它读回来的 run（读不出 run 时仍按 seq 入索引）。
pub struct Located { pub seq: Seq, pub run: Option<RunId> }
/// 一次 refresh 做了什么，以及为此从盘上抬起了多少字节。
pub enum Refreshed { Unchanged, Appended { bytes_read: u64 }, Rebuilt }
impl LedgerIndex {
    pub fn reader(&self, dir: &Path) -> LineReader<'_>;                            // 取行的唯一入口
    pub fn run_seqs_before(&self, run: RunId, before: Option<Seq>)                 // 一个 run 写过的 seq，新的在前
        -> impl Iterator<Item = Seq> + '_;
    pub fn len(&self) -> usize;  pub fn is_empty(&self) -> bool;                   // 重建后自证条数
    pub fn tail_seq(&self) -> Option<Seq>;
    pub fn seqs(&self) -> impl DoubleEndedIterator<Item = Seq> + '_;               // 全部 seq，升序；两端都可取
}
/// 一次查询期间的取行游标：持有当前段的句柄与它的逻辑位置。私有字段。
pub struct LineReader<'index> { /* index、dir、Option<段名＋BufReader<File>＋下一行偏移> */ }
impl LineReader<'_> {
    pub fn line_at(&mut self, seq: Seq) -> Result<Vec<u8>, StorageError>;           // 取一条（顺序读零 syscall 开销）
}
```

- **索引不落盘，只有一段常驻内存**：建表只有一遍扫描（`index/ledger.rs` 的 `walk`），两个入口共用它：`rebuild` 自己定位每一行，`folding` 让折叠交回它已读出的位置；`Views` 持有它并在每次查询前 `refresh`。不设落盘的索引文件：它会是第二个可失效的「答案来源」，而从段重建（`rebuild`）与增量折入（`refresh`）已经够用。
- **seq 是隐式的，每行只存一个 `u64`**：内核给行连续编号，所以一行的 seq 就是它在列里的位置加 `base`，列里只存段名字典 id 与偏移拼成的一个字。五万行的账本在索引里占 400 KB，显式 seq 列加 (id, 偏移) 对的布局要 1.2 MB（每行 24 B，`a_contiguous_ledger_costs_eight_bytes_per_record` 钉住 8 B）。损坏的账本仍要能索引，因为修复路径靠它：低于 `base` 的 seq、远到要让空洞多于行数才够得着的 seq（拉长后列里的空洞数超过 `max(列里的行数, 64)`；界按行数而不按列长算，因为按列长算时每个被接受的 seq 都能让列翻倍）、段 id 超过 16 位或偏移超过 48 位的位置，都进 `outliers`；一个 seq 只在两处之一，重复写保留最后的位置，`seqs()` 把两处按序归并。**被否：只留列、把列外行丢掉**——被丢的行对每个读者都不可见；**被否：空洞无上限地拉长列**——一个被写坏成 2^60 的 seq 会让索引去分配 2^63 字节。`doubling_seqs_leave_the_column_bounded_by_its_lines` 钉住这条界；`fold/tests.rs` 的 map 形 oracle 对全部查询（含 `seqs()` 正反两向与两端交替）判等。
- **`base` 由第一条插入的行定下，之后不再移动。** 这第一条行的 seq 若已损坏且很大，其后每一条健康的行都低于 `base`，全部落进 `outliers`：答案仍然正确，每行 8 B 的布局却失去了，一行按 `BTreeMap` 的节点计价。接受这笔代价，因为它只出现在首行损坏的账本上，而那样的账本本来走修复路径；列外行多于列内行时重定 `base` 是候选的改法，判定它的证据是一个首行 seq 损坏的真实账本在索引里的常驻字节。
- **取行走游标，而不是每行一次 open ＋逐字节 read**：一次 `History`／`RunHistory` 查询要取一段连续的 seq，而每一行重开段文件、再一次一个字节 `read` 到换行的读法，系统调用数与行长同阶。句柄因此住进 `LineReader`：段名不变即不重开，读用 `BufReader::read_until(b'\n')`，一次填充服务多行。
  - **位置自持**：游标记住下一行的偏移。所求偏移与它相同即不 seek，顺序读全程零 seek；位置已知而不同，就按差值相对移动（`BufReader::seek_relative`），目标落在已读入的缓冲里时缓冲原样保留，落在缓冲外时才去底层 seek 并弃缓冲；位置未知时绝对 seek。差值换不成 `i64` 时按位置未知处理。`run_history` 由旧到新读一个 run 的行，相邻两行之间隔着别的 run 的几行，这段间隔常在同一次缓冲填充之内，于是这样的一行既不付 seek 也不付填充。**被否：位置不同一律绝对 seek**——每读一行都丢掉刚读进来、多半已经含着下一行的那块缓冲。
  - **位置用 `Option<u64>` 表达，算不出来就作废**：seek 前、读前各置 `None`，只在一次成功的读之后写回 `offset + 读长`；长度换 `u64` 失败或相加溢出则继续为 `None`，下一次调用必 seek。一个可能错的位置会让游标把别的行的字节交在调用方要的 seq 名下，而旁挂物宁可重做不可误信（与「存疑即重建」同一条反射）。
  - **读一行只此一条路**（`LineReader::line_at`），不留转发壳：否则「怎样读一行」有两个权威，而慢的那个还在原地招手。
  - **段不因此出门**：`LineReader` 的公开面只有 `line_at(seq)`，段名与偏移仍是内部事务（§7 第三条）。
- **索引常驻，不每次查询重建**：每一次 `History`／`RunHistory` 都从头 `rebuild` 一遍时，查询代价与账本长度同阶：逐段重读、逐行取一个 `String`。
  - **增量面是 `refresh`，不是“追加时告知索引”**：调用方手里只有 `EventRecord`，段名与偏移是 `jsonl` 的内部事务（§7 第三条）。让观察者携偏移会把分段泄给调用方，而 `refresh` 把那个知识留在本模块。
  - 索引因此多记一张 `scanned: BTreeMap<段名, 已折入字节数>`。`refresh` 逐段比对：**变大就只读新那一段字节**；**变小或消失就全重建**（断尾修复截过段，旧偏移不再可信）；一字未动就什么也不做。「消失」按计数判：本次列出的段名逐个在 `scanned` 里查，命中的个数少于 `scanned` 的条数，就是有已折入的段不见了。每段一次有序表查找，不按段数的平方比对名字（`index/ledger/tests.rs` 的 `a_segment_that_vanished_rebuilds_the_index`）。
  - **「上次读到哪」只有这一个家，而且它恒落在整行边界上**：`fold_segment` 只把带终止符的行计入，所以 `scanned` 记的永远是某条记录的结尾，下一次 refresh 从一条记录的开头起读，**取不到半条**。
  - **定位读，而不是整读再切尾**：`Vfs::read_at(path, from, size-from)`，读进来的缓冲只装增量。长度以本次 `Vfs::size` 读到的值封顶——stat 与 read 之间落的那条账留给下一次 refresh，而不是折在一个本次没有确认过的偏移上。
  - **`scanned` 不落盘，所以「偏移写坏」不是一个状态**：它是常驻状态，建表时取当下段大小，故「已折入」与「盘上长度」在那一刻是同一个数。进程内若出现 `scanned > 段长`，走的是「变小」那一支，同样整份重建。任何一条可疑路径的答案都是重建，没有一条会读到半条记录。
  - **`refresh` 报出它读了多少字节**（`Refreshed`）：整读与定位读折出的索引逐条相同，唯一的区别就是抬起的字节数，所以那个数必须能被断言，也正是 `index_refresh_read` 这条预算的读数来源。
  - 消费者：`accounting::views` 的 `Views` 持一份，每次查询先 `refresh`。刷新代价是一次 `read_dir` 加逐段 `metadata`，与账本大小无关。
- **run 索引与 seq 索引同住一张旁挂物**：不建 run 表时，`run_history` 只能从账本尾部逐行往回读、读完再过滤，不属于这个 run 的行也要完整取出再丢掉。索引因此多记一张 run 表（`RunTable`，按区间记每个 run 写过的 seq），查询只取属于它的 seq，代价与**答的条数**同阶而不与账本长度同阶。
  - **为什么不放进一份落盘冷投影**：要让 `run_history` 读一份落盘投影，就得在事件路径上开一个写事务，每条事件多一道磁盘屏障。为一个不需要持久化的答案给每一条落账加一道屏障，方向是反的；本 crate 因此不持有落盘投影。
  - **为什么可以放进 `index`**：这张旁挂物**已经常驻**（`Views` 持有并每查询 `refresh`），**已经逐行解析过每一条新行**（`locate` 一次 `serde_json` 解析读两个字段），**已经带着「存疑即重建」的可弃性**。run 表搭的是同一趟 refresh、同一条重建反射，不新增任何同步义务。
  - **内存代价**：run 表每 run 一段或几段区间，不是每条 seq 一个节点；seq 列不为每条带一个段名 `String`，段名进字典一行一个。计入 `resident_empty_idle` 预算（`tools/xtask/budgets.toml` 该行记录读数与上限）。
  - **run 表是区间，不是 id 集**：run 是连续突发，写入即尾部并段；真交错的 run 就是几段区间。只有值域首尾相接才并段，而并段时缺口里的值必然都写过（两端点各自由一次写入造出），故区间从不声称没写过的 seq。成员查询＝区间定位后按值下探，答的顺序仍由新到旧。它是惰性的：按区间由新到旧逐值产出，调用方 `take(n)` 取多少才走多少，所以代价与取出的条数同阶，不先把整个 run 的 seq 展开成一张表。等价仍由 `index/fold/tests.rs` 的 oracle 判定，oracle 那一侧照旧是展开的表。**被否**：`BTreeSet<Seq>` 每条一个 id，长 run 不塌缩。
  - **等价锁在 `index/fold/tests.rs`**：oracle 是 map 形实现，proptest 喂随机行流（乱序与重复 seq、不可解析的 run 名、非文档行）断言全部公开查询恒同解。
  - **`before` 取开区间**，与线格式 `HistoryAnswer.earlier` 的含义（「从这条之前接着问」）同字同义，调用方不做 `before - 1` 这条减法，也就碰不到它的 `Seq::FIRST` 边界。
  - **答的顺序**：`run_seqs_before` 由新到旧，因为调用方要的是会话的**末尾**；调用方取够条数后翻转成由旧到新再取行，于是 `LineReader` 全程向前走，不付逆序读每行一次的 seek。
- `locate` 只探 `seq` 与 `run` 两个字段——索引不要求整条记录可解析，破损日志上的索引正是修复路径所需；`run` 缺失或解析不出的行**照样入 seq 表，只是不属于任何 run**，残尾（无换行结尾）跳过不入索引，其修复归 jsonl。段名排序由本模块自持（不信文件系统枚举序）。新增 `StorageError::SeqMissing{seq}`（→ `E_INVALID_ARGS`）：问一条从未写过的 seq 是调用者错，不是损坏。

### 8-5 storage::hot（形状 7）

```rust
pub struct HotView { /* runs: BTreeMap<RunId, RunHot>、evicted: BTreeSet<RunId> —— 私有 */ }
pub struct RunHot { pub phase: RunPhase, pub last_seq: Seq, pub last_kind: EventKind, pub who: String,
                    pub addr: Option<Address>, pub started: Option<TimeMs>,     // 房间与开始时刻
                    pub completion: Option<String>, pub pr: Option<String>, pub ask: Option<String>,  // 结局、PR、所等之事
                    pub task: Option<String>, pub goal: Option<String> }                              // 人交给它的任务与目标
pub enum RunPhase { Active, Frozen }
impl HotView {
    pub fn new() -> HotView;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError>;   // 增量；重复 seq 幂等（只前进）
    pub fn runs(&self) -> impl Iterator<Item = (&RunId, &RunHot)>;              // BTreeMap 序
    pub fn get(&self, run: &RunId) -> Option<&RunHot>;
    pub fn was_evicted(&self, run: &RunId) -> bool;                            // 墓碑：这次跑冻结后被逐出
    pub fn active_count(&self) -> u64;  pub fn frozen_count(&self) -> u64;
}
pub const RECENT_FROZEN: usize = 32;
```

- **热视图只留活跃的跑和最近冻结的 `RECENT_FROZEN` 个**：一次跑冻结后，若留着的冻结跑超过 `RECENT_FROZEN`，`last_seq` 最小的那个被逐出，只留一块墓碑（它的 RunId）。所以 `runs()` 本身就是一页城景该带的那几次跑，城景的大小只随活跃数增长，热视图的内存也一样（墓碑每次跑 16 字节）；更早的冻结跑经分页的 `History`／`RunHistory` 读，`frozen_count` 把墓碑也数进去，页面知道列表之外还有多少。每次冻结至多逐出一个，找最小 `last_seq` 扫一遍留着的跑，O(活跃＋N)，不另建按 seq 排的索引。`RECENT_FROZEN` 是线上答复的大小上界，不随机器变，所以是常量；它只在这里定义一次。
- **落在墓碑上的记录归冷的一侧**：冻结在热视图里是终态，被逐出的跑不会再活跃，所以一条记录的 RunId 在墓碑里时，`apply` 什么也不改——那条记录在账本里，分页的历史读得到它。没有墓碑的话，一条没有开场的尾巴会被当成一次新跑的检查点，把旧跑重新记成活跃的（活跃数多一，城景多一行）。

- 界面查询在此命中不读盘；run_started→Active，run_frozen→Frozen；其余事件只推进 last_seq/last_kind。
- **`addr` 与 `started` 从 `run_started` 记下**：`record.addr()` 是这次跑的房间，`record.t()` 是它开始的时刻；二者只在这一种记录上赋值，其余记录不动它们，所以一次跑的房间不会被后来的城市级记录改写。`Option`，因为热视图可能在 `run_started` 之前先看到同一次跑的 `checkpoint_committed`（检查点先于开场落账），也可能只看到一段没有开场的尾巴——**看不到的事不猜**。理由：`RunSummary.who` 是首条记录的作者、恒为 `city`，单靠它无法把一次跑归到 `hall/mayor` 这个房间，「与 Mayor 的对话」就在线上拼不出来。
- **`completion`、`pr`、`ask` 各从一种记录记下**：`run_frozen` 的 `completion` 字段；`pr_opened` 的 `branch` 字段（后一条覆盖前一条）；`approval_requested` 的 `action_desc` 字段，且只活到这次跑的下一条记录——任何别的记录清掉它，所以 `ask` 有值当且仅当 `last_kind` 是 `approval_requested`，页面据 `last_kind` 判「在等」，据 `ask` 写「等什么」，两者同源。按键读字段而不整条 `Payload::read`：热视图每条记录都折，整条反序列化要复制整个 map；字段缺失记 `None`，同 `addr` 的口径，看不到的事不猜。
- **`task` 与 `goal` 从 `run_started` 记下**：那条记录的同名两个字段，与 `addr`、`started` 同一处赋值、同一个口径——只在这一种记录上写，其余记录不动它们；字段缺失或是空串记 `None`，因为一句空的任务不是一个名字。理由：run 板以它们给一行 run 起名，而重载后的页面只有 `RunSummary`（wire-SPEC §8-48e）。
- **城市级记录不进 run 表**：`RunId::CITY`（nil）标记的是属于城而不属于任何 Run 的记录——创世记录、`building_created`。把它们折进 run 表会让 `active_count()` 在一座**从未派过活的城**里返回 1：城市页读服务端的这个数、写「1 run in flight」，而总览页折同一条流写「什么都没在跑」——**一个问题两个答案，而错的那个是服务端的**。

### 8-7 storage::attribution（形状 7）

```rust
pub struct Attribution { /* by_run、by_actor、by_segment、by_tool、by_skill: BTreeMap<String, UsdMicros>、
                            total: UsdMicros、pending_wave: … —— 私有 */ }
impl Attribution {
    pub fn new() -> Attribution;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError>;
    pub fn report(&self) -> AttributionReport;
}
pub struct AttributionReport { pub total: UsdMicros, pub by_run: Vec<(String, UsdMicros)>,
    pub by_actor: Vec<(String, UsdMicros)>, pub by_segment: Vec<(String, UsdMicros)>,
    pub by_tool: Vec<(String, UsdMicros)>, pub by_skill: Vec<(String, UsdMicros)>,
    pub unpriced: Unpriced }
pub struct Unpriced { pub calls: u64, pub tokens: u64 }
```

- **五维度**（prefix 段位／SKILL／工具／子 Run／Building・Resident）。映射：`by_segment`＝prefix 段位（四段＋window 桶）；`by_skill`＝SKILL；`by_tool`＝工具；`by_run`＝子 Run；`by_actor`＝Building／Resident。
- 现状与待接点（不造假数据）：SKILL 机制属 Library，故 `by_skill` 恒入兵底桶 `no_skill`，取材契约定为 `tool_result.data.skill`（字串，权重同字节数）；派生执行面属 collab，故 `by_run` 为“每 Run 自身花费”，`run_started.parent` 链的父子归并待派生落地后接。两处均不影响 A20：兵底桶仍参与求和，五维各自恒等 total。
- 读取契约定死三条——`prompt_assembled` 携 `segments:[{slot,len}]`（两种载荷形均有此二字段）与可选 `window_bytes`（缺即不设 window 桶）；`tool_result` 携 `name` 与 `bytes`；`model_returned` 携 `billed_usd_micros`。**无权威计费额即归因零**（估算等于臆造钱，宁不报），但这次调用记入 `unpriced`：`calls` 加一，`tokens` 加上它 `usage` 的四项 token 之和（缺 `usage` 即加零）。没有它，一座只用订阅登录或本地模型的城跑了多少次都是 total 0，读者分不出「没跑」与「跑了没有报价」；token 是这时唯一量得到的用量。无权重基础即入诚实桶 `unattributed`／`no_tool`（不静默丢）。工具权重只属一波：结算即清，下一调用不继承上波。A20 除四断言外另以 256 例 proptest 钉（任意金额×权重组合均恒等）。
- 取材：`model_returned.data.billed_usd_micros`（权威计费额）；`prompt_assembled` 逐段 len；`tool_result` 的 name。每维度独立分割同一总额：by_run/by_actor 按事件归属；by_segment 按该 model_returned 所属 run 最近一条 prompt_assembled 的段 len 最大余额法分割（四段＋window 桶：入窗历史份额）——段权重按 run 分键，因为 runtime 每个 run 只写一条 prompt_assembled（其后的回合载荷不变即不再写），账本上交错的另一个 run 的 prompt_assembled 不是这次调用的基础；by_tool 按前一波 tool_result 字节最大余额法（无波则 no_tool 桶）。最大余额法使每维度和恒精确＝total（A20 的整数保证）。
- **段位基准按 run 保存，run 冻结即丢。** 一次整账本折叠会遇到城里有过的每一个 run；`run_frozen` 是终态，其后不再有该 run 的调用，所以它最近一次 `prompt_assembled` 的段位基准随之移除，常驻量只随在跑的 run 数增长，不随城的历史增长。

### 8-17 storage::checkpoint::provenance（形状 2 值）

```rust
/// 一次运行选定的模型，两者恒同行：模型 id 与它被要求的思考档位。
pub struct ModelChoice { pub id: String, pub effort: Option<kernel::Effort> }

/// 一次提交出自谁：唯一构造点在 `Provenance::new`，字段私有。
pub struct Provenance { /* run、actor、model、effort、city —— 私有 */ }
impl Provenance {
    pub fn new(run: RunId, actor: Address, city: B3Hash, chosen: ModelChoice) -> Provenance;
    /// 城的身份＝创世行的链哈希，从账本首段的第一行读出（只读一行）。
    pub fn city_of(ledger_dir: &Path) -> Result<B3Hash, StorageError>;
    /// git trailers 块，顺序与拼写恒为下列五行，有前任时加第六行，末尾带换行。
    pub fn succeeding(self, predecessor: RunId) -> Provenance;   // 写前任的唯一入口
    pub fn predecessor(&self) -> Option<RunId>;
    pub fn trailers(&self) -> String;
    pub fn actor(&self) -> &Address;
    pub fn run(&self) -> RunId;
}
```

```
Sprawling-Run: <run>
Sprawling-Actor: <actor>
Sprawling-Model: <model>
Sprawling-Effort: <effort or none>
Sprawling-City: <hex>
Sprawling-Predecessor: <run-id>      （只在有前任时）
```

前任经消费式的 `succeeding` 写入（`new` 已到四参数上限，而前任与 run 并不总是同行），`attribution()` 同时多出 `predecessor` 键（8-18）；缺席即 `None`——一条缺席的世系比一条编出来的好。没有前任的提交，trailers 只有上面五行。

- **每一个由城作出的提交都带上作出它的会话。** 提交署名为
  `<actor> <<actor>@<city 前 12 位 hex>.sprawling>`，
  正文为 `<subject>\n\n<trailers>\n`。一个人 `git log` 一眼看得出这一行出自哪个居民、
  哪次运行、哪个模型；`git interpret-trailers --parse` 读得出结构。
- **被否的另一条路：把这些事实塞进 subject。** subject 是给人读的一行，塞五个字段就没人读它了；
  trailers 是 git 自己就有的机制（`interpret-trailers`），复用它比发明一种前缀语法更省。
- **账本仍是权威。** trailers 是**给城外读者的投影**，不是第二个事实来源：谁做了什么由 Ledger
  回答，两边靠 oid 对上（`checkpoint_committed.oid` 与 `file_discarded.restoration`）。
  trailers 与账本不一致时以账本为准，trailers 是要修的那一侧。
- **effort 的字面来自 serde 的名字**（`kernel::Effort` 的 `snake_case`），所以「档位怎么拼」在这个仓库里只有一处权威；缺档位写 `none`。
  模型 id 未知时写空串——写一个假的 id 比写空更糟。
- **`city_of` 只读第一行**：整本账本可以有几十兆，而创世行是第一段文件的第一行；它与 open 的版本探测共用 `jsonl::first_line`，所以「第一行是什么」只有一处权威——以 `\n` 结尾的第一行，撕裂的首行不算。

### 8-18 trailers 的账本一侧

```rust
impl Provenance {
    /// 这次提交出自谁，按每一条指名提交的记录都携的形状给出。
    /// run 与 actor 已经是记录自己的身份（`EventRecord::run` 与 `addr`），
    /// 在载荷里重复它们等于给同一个事实立第二个权威。
    pub fn attribution(&self) -> kernel::event::record::CommitAttribution;
}

/// 没人点过的档位怎么记：记成 `Effort::None`。
/// `Option` 划出的那条线（由提供方决定 / 要它别想）从未上过 trailer 或账本，
/// 两边一向都写 `none`；这个坍缩全仓只在这里发生一次。
pub fn recorded_effort(effort: Option<kernel::Effort>) -> Effort;

/// 档位怎么拼的唯一权威（取自 `kernel::Effort` 的 serde 名）。
pub fn effort_word(effort: kernel::Effort) -> String;
```

- **键的权威在 kernel。** `model` / `effort` / `predecessor` 三个键由
  `kernel::event::record::CommitAttribution` 拼写，本 crate 不另持键名常量：
  两个 crate 各手写一次同一个键，就是两个会各说各话的权威。
- **`checkpoint_committed` 与 `pr_merged` 携同一份归属。** 8-17 写着「trailers 是投影，
  账本是权威」，而这两个事实否则只存在于 git 提交上；带上它们，`sprawling whose`
  才能只读账本作答（`accounting::views`，sprawling-SPEC §8-41）。两条记录 flatten 同一个结构，
  于是「一次提交出自谁」不会在两个 kind 上长成两种说法。
- **缺键的记录读得回**：不带这两项的 `checkpoint_committed` 没有这两个键，
  `CommitAttribution` 的 `#[serde(default)]` 对它答空 id 与 `None`——
  **投影说不知道，好过投影猜一个**。
- **被否的另一条路：让 `sprawling whose` 去读 git trailers。** 那是把投影当成权威，正是 8-17
  明确拒绝的方向；而且一座导出后在别处恢复、`.git` 并不在身边的城将答不出自己的历史。

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
    /// 这一波写了哪些文件时只把它们交进来（runtime-SPEC §8-45）。工作区里是
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

### 8-13 storage::changes（形状 4 适配器；git2）

```rust
pub enum Lines { Counted { added: u32, removed: u32 }, Binary }
pub enum How    { Added, Modified, Deleted, Renamed { from: String } }
pub struct FileChange { pub path: String, pub how: How, pub lines: Lines }
pub enum Head   { Commit(GitOid), WorkingTree }
pub fn between(city_root: &Path, base: GitOid, head: Head)
    -> Result<Vec<FileChange>, StorageError>;
```

**写入侧早就是 git 原生的，缺的是整个读出侧。** 每一次工具浪前 `wave_pre` 都落一个真 commit，
而仓库里没有任何一处读得出两个 commit 之间变了什么。人要的是「这个 agent 动过哪些文件」，
而那个事实已经在盘上。

**它天然只含写域。** `stage_scopes` 为写域的**每一个**前缀各暂存 `<prefix>/*`，所以两个检查点之间的差异不可能包含
会话只读过的文件——其他 harness 正在为这件事头痛（一个会话的 diff 把读过的文件也算进去），
而这个设计因为检查点就是写域而白得。

**`Lines` 是穷举枚而不是两个数。** 二进制文件没有行数，把它画成 `+0 −0` 是界面在说假话；
同理 `How::Renamed` 与「删一个加一个」是两件事。

**只算数，不搬补丁文本。** `scan_staged` 拒绝回显命中的字节（「回显字节本身即泄漏」），
而补丁文本就是文件内容过 socket——同一个出口问题。路径与计数是一个查询；单文件的 hunk
得是另一个显式查询，并且必须过同一道 `kernel::secret::scan`（§8-19）。

**不缓存。** 两个 oid 都不可变，所以结果可以永久缓存（`digest_cache` 是现成先例）；
但先测再调，未测到慢之前多一张表就是多一份要同步的状态。

`wave_pre` 不写 `HEAD`，而是写一个
dangling commit 并由 `refs/sprawling/runs/<run>/<oid>` 指住（见 8-8）。本模块只读不写，
从 oid 工作。

### 8-9 storage::worktree（形状 4 适配器＋形状 2 值类型；git2）

```rust
pub struct WorktreeName(String);        // 文件系统安全；无分隔符、无点开头
pub struct Worktrees { /* repo、home、ceiling —— 私有 */ }
pub struct WorktreeLease { /* name、path、disk —— 私有 */ }
impl Worktrees {
    pub fn open(city_root: &Path) -> Result<Worktrees, StorageError>;
    /// `scopes` 是这棵树的写域（与检查点同一组 pathspec，空即整棵）：再领时只检出它。
    pub fn claim(&self, name: &WorktreeName, scopes: &[String]) -> Result<WorktreeLease, StorageError>;
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
    pub fn opened_payload(&self) -> Result<Payload, StorageError>;   // worktree_opened，形状是 kernel::event::record::WorktreeOpened
}
```

- **一节点一棵，且它是 git worktree**：对象共享、工作树不共享，于是两个 Agent 看不见对方的中间态，而合入只走 PR 流。它从 `storage::checkpoint` 已在管的那个仓库分枝——城里不开第二个仓库。
- **建树前预检，不是建到一半失败**：工作树字节数 > `WORKTREE_MAX_BYTES` 即拒，拒词带当前上限与实测值。reflink 今天不尝试（无 unsafe FFI 或新依赖就没有 CoW 接口），故设计里「CoW 则 reflink，否则按上限拒」在每个平台上都只走后一臂——这是当前口径，不是已实现的 CoW。可用磁盘余量未探（std 无该接口），同样写在明处。
- **上限只称人的字节**：`measure` 跳 `.git` 与 `RESERVED_PREFIX` 子树（谓词住 `storage::reserved`，与 checkpoint 的 `stage_tree` 同一个）。
  账本、CAS、投影与别人的工作树都住 reserved 之下；把它们算进来，跑了一个月的城会因为自己的簿记长大而拒绝派活，
  并用一句「城的工作树有 N 字节」说这件事。断言：账本 4 KB、产品文件不到 1 KB 的城仍可领树。
- **上限称一次检出，不称留着的树之和**：`WORKTREE_MAX_BYTES` 只在 `place`（新建一棵、全量检出）之前量城的工作树；再领一棵留着的树不量，留着的各棵也不相加。
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
  否决首次放置也只检出 scope：`Worktree::add` 总做一次全量检出，所用 git2 版本没有把 `checkout_options` 暴露成安全接口，而本 crate 禁 `unsafe`；重开的条件是 git2 暴露它。
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

### 8-10 storage::queue（形状 7）

```rust
pub struct EventQueue { /* items: BTreeMap<u64, QueueItem>、next_id、seen: BTreeMap<IdemKey, TimeMs>、stats —— 私有 */ }
pub struct QueueItem { pub id: u64, pub key: IdemKey, pub payload: Payload }
impl EventQueue {
    pub fn new(lane: QueueLane, capacity: u64) -> EventQueue;
    /// Admission first (kernel::backpressure), then enqueue; Shed returns
    /// the verdict to the caller (who accounts backpressure_shed).
    pub fn enqueue(&mut self, key: IdemKey, payload: Payload, now: TimeMs) -> Result<Admission, StorageError>;
    /// Dedup before side effects: a key already consumed is Duplicate and
    /// must not reach the consumer twice.
    pub fn consume(&mut self) -> Option<QueueItem>;
    pub fn stats(&self) -> QueueStats;   pub fn len(&self) -> u64;   pub fn is_empty(&self) -> bool;
}
pub enum QueueLane { Signal, Approval, Repair }   // 一份实现三队列
```

- 容量入构造子（`new(lane, capacity)`）而非写死常量——三 lane 容量不同是装配事。**重复键返回 `Admit` 而非 `Shed`**：发送方已尽职，告知失败只会招致无效重试；`seen` 持久于队列寿命（消费后仍认得出重复，因为副作用已跑过一次）。被 shed 项不入 `seen`，故重试不算重复。
- **去重记忆按时间有界**：`seen` 记下每个键的入队时刻，`enqueue` 先按 `now` 驱逐早于 `IDEM_WINDOW_MS`（六小时）的键。
  只增不减的集合会让长跑的城为它曾经入过队的每一条事件各留一个键，而重试发生在一次投递的窗口内，不发生在一天之后。
  时钟倒退时一个键也不驱逐——队列对时钟不持观点，而记得太久只会少跑一次副作用。
  **常量住 `storage::queue`**：它只有这一个读者，放进 `kernel::consts_policy` 会让 kernel 的公开面多一个只有 storage 读的数。
- 重建性：队列状态＝（signal_enqueued − signal_consumed）的 projection；持久性不在本模块（Ledger 已是历史）。lane 只定账目名字段，三队列零分支差异——差异出现之日即分模块之日（反推式合并的退出条件写在明处）。

### 8-11 storage::digest_cache（形状 4）

```rust
pub struct DigestCache { /* dir —— 私有；文件名＝内容哈希 hex64.json */ }
impl DigestCache {
    pub fn open(dir: &Path) -> Result<DigestCache, StorageError>;
    /// Same content hash digests once for life: a second put with the same
    /// hash is a no-op returning the stored artifact.
    pub fn put(&mut self, content: &B3Hash, tree_json: &[u8]) -> Result<(), StorageError>;
    pub fn get(&self, content: &B3Hash) -> Result<Option<Vec<u8>>, StorageError>;
    /// Invalidation produces the digest_invalidated payload; the entry is
    /// removed so the next digest re-runs.
    pub fn invalidate(&mut self, content: &B3Hash, reason: &str) -> Result<Payload, StorageError>;
}
```

- 消费者是 runtime::digest；本模块只交存储面。写入经 tmp＋rename（复用 cas 的 Vfs 纪律）。

### 8-12 storage::bundle（形状 4 适配器＋形状 2 值类型）

```rust
pub struct Manifest { /* 私有；records、head、cas_objects、files、history（包数与引用数） */ }
impl Manifest {
    pub fn records(&self) -> u64;         pub fn head(&self) -> &str;   // 链头哈希
    pub fn cas_objects(&self) -> u64;     pub fn files(&self) -> u64;
}
pub struct Bundle;
impl Bundle {
    pub fn export(city_root: &Path, dest: &Path) -> Result<Manifest, StorageError>;
    pub fn restore(bundle: &Path, city_root: &Path) -> Result<Manifest, StorageError>;
    pub fn read_manifest(bundle: &Path) -> Result<Manifest, StorageError>;
}
pub const MANIFEST: &str = "MANIFEST.json";
pub fn open_restored(city_root: &Path, now: TimeMs) -> Result<PathBuf, StorageError>;  // 恢复后可继续写
// StorageError 增一臂：Bundle { op, detail }——I/O 正常但不是一座城（目的地已占、清单对不上、链有缺口）
// Vfs 内缝增 `list_dirs`：两个适配器同改；list 与 list_dirs 都是浅层，遍树用显式工作表（不递归，栈溢出接不住）
```

- **带走什么**：`ledger/`（唯一历史，必带）、`cas/`（Locator 指进去，不带就断链）、城里的产品文件（`City.md`、各楼的 `RULES.toml`／`Roadmap.md`／`URBANITE.md` 与房间内容）。**不带**：索引与任何派生视图（可弃，恢复后由 Ledger 重建，带了就是第二份历史）；凭证（**它从不在城里**，在宙主机金库——导出一份能拷走凭证的备份会把隐私保证一次性作废）。
- **为何是目录而非单文件**：单文件要么自造容器格式（多一个要养的格式），要么引 tar／zip 依赖。目录两者都不要，且任何备份工具都能再打包一层——压缩不是本模块的职责。
- **清单是完整性的依据**：`MANIFEST.json` 记下记录数、链头哈希、CAS 对象数与文件数；`restore` 恢复后重算并比对。不对即拒，而不是“恢复了但少了几条”——后者是历史失真。
- **四个数由 `Manifest::of(vfs, ledger_dir, cas_dir, files_root)` 一处算出**：导出量目的地、恢复量城本身、比较的两侧因此是同一种测量。
- **清单也数随行的历史：`history_packs` 与 `history_refs`**，由 `history::Carried::of(vfs, bundle)` 一处量出 bundle 的 `history/` 里有几个包、几条引用。导出端量目的地写进清单；恢复端在复制任何东西之前量 bundle 本身再比，不等即整次拒绝（`StorageError::Bundle { op: "restore" }`）——删掉包的 bundle 若照常恢复，文件与账本已落、引用却指向不存在的对象，全成或全不成就破了。清单没有这两个键时读作 0：v0.0.6 的导出不写 `history/`，于是一份被剥掉这两个键、却仍带 `history/` 的 bundle 照样被拒。被否：恢复后再从新仓库反查（那时城文件已经落下，拒绝只能留下半座城）。
  `walk` 返回 `Result`：不存在的目录算空（尚无 CAS 的城），读不动的目录停下并带路径上报——一个子目录静默贡献零个文件，正是一份短了的备份与它自己的清单相符的来路。
  `restore` 比四个字段而不是两个：丢了 CAS 对象的 bundle 链校全绿、Locator 全部指空，只有对象数说得出这件事。
- **清单的四个数全部读自导出结果，因而不能自证**：一次把半座城丢掉的拷贝与它自己的清单完全相符。故 `export` **另取源侧的五个数**——账本文件数、CAS 对象数、城内文件数、账本记录数、链头——与目的地逐项比，任一项不等即拒。源侧的数从 `copy_tree`／`copy_city_files` 的返回值来。
- **链验在 restore 内**：恢复完即走一遍 Ledger 开启与链校（jsonl 已有的那一道）。交给调用方去验等于把一个必须成立的性质变成约定。
- **文件顺序确定**：遍历走 `Vfs::list`（已排序），清单用 BTreeMap；同一座城导两次，`MANIFEST.json` 逐字节相同。
- `Vfs::append` 是**追加**，残留 `.part` 未清即发布出「残骸＋新内容」的拼接体；修法取 cas 既有两层纪律（open 清扫 tmp 残骸＋put 前 `truncate(0)`）而非另立新机制。`invalidate` 对不存在项不报错（末态即调用者所求），载荷携 `existed` 实报。
- **什么算城里的文件：`travels` 一个家（形状 1 判定）。** 导出端与恢复端问同一个谓词：根层 `.sprawling`（其内容经 ledger／cas 两张门各走各的）与任何深度的 `.git`（受保护元数据，写下即提权）不算城文件；楼自己的 `.sprawling`（`RULES.toml` 等治理字节）**随行**，它是城的一部分。名字引自 kernel 的名单（12.3 定规），不重拼字符串。两侧的策略不同且必须不同：导出端**跳过**（选择带走什么，源城里的簿记本来就不走），恢复端**拒绝**（全成或全拒——一份夹带 `.git` 的 bundle 是伪造品，恢复它就是在落 hooks）。
- **git 历史随行（`bundle::history`，形状 4 适配器；git2）。** 城根若是 git 仓库，`export` 用 `git2::PackBuilder` 把每条引用所达的全部对象（沿 revwalk 的提交与树，外加每个引用目标的递归闭包，覆盖附注标签）打成 `history/history.pack`，把引用写成 `history/refs`（每行 `<oid> <name>` 或 `ref:<target> <name>`，`HEAD` 在内）；城根不是仓库时 bundle 没有 `history/`，恢复端也不建仓库。`restore` 在复制任何东西之前拒绝已有 `.git` 的城根（`StorageError::Bundle { op: "restore" }`），复制后 `git2::Repository::init` 一个新仓库，经 `Odb::packwriter` 写入包（libgit2 边写边建索引并校验每个对象），再逐条立引用、按 `HEAD` 读出树写入索引，使 `git status` 只报导出时未提交的改动。**`hooks/` 与 `config` 永不复制**：bundle 里只有包与引用两样，仓库的配置与钩子来自新 init，于是一份伪造的 bundle 没有落钩子的路。引用名只收 `HEAD` 与 `refs/` 之下、且过 `git2::Reference::is_valid_name` 的名字，其余整次拒绝。被否：直接复制 `.git` 目录（会带上 hooks 与 config，正是 8-12 要拒的东西）；`git bundle` 子进程（引外部二进制，且 bundle 格式的解析不在本进程的校验之内）。
- **v0.0.6 的 bundle 导入其仓库而不是整体拒绝。** v0.0.6 的导出把城根的 `.git` 整个复制进 `city/.git`，清单的 `files` 也数了它的文件。恢复端见到 `city/.git` 是目录、而 bundle 没有 `history/` 时，把它当作一份旧式历史：以 `git2::Repository::open_bare` 只读打开，按导出端同一段打包逻辑（同一个引用名准入）得到包与引用，再走同一个 `History::land`；`city/.git` 下的文件既不拒也不复制，清单的 `files` 减去它们的个数再比。`hooks/` 与 `config` 依旧只来自新 init。仍整次拒绝：`history/` 与 `city/.git` 同在（v0.0.7 的导出从不写后者，二者同在即伪造）、`city/.git` 不是能打开的仓库、`city/.git/objects/info/alternates` 或 `city/.git/commondir` 存在（`open_bare` 会顺着前者把 bundle 之外任一仓库的对象读进包，顺着后者把 `objects`、`refs` 与 `packed-refs` 整个换成它所写路径下的那一份，伪造的 bundle 借此把 bundle 之外的对象与引用带进新城；v0.0.6 的导出从不写二者，因为城根的仓库既不借用别处的对象库，也不是别处仓库的 worktree），以及任何更深处的 `.git`（那是别的仓库，导入它不在本契约内）。被否：继续整体拒绝（v0.0.6 用户的备份因此恢复不了，而其中的对象与引用正是本节已有的导入路径能安全接住的）。
- **别名永不落盘（8-25）。** `walk` 见到任一链接拒绝整次操作；bundle 的每一次落盘只走 `bundle::landing::land`（形状 4 适配器）：它按值收下 `alias::WriteTarget`，让清过的目标被消费而不是查完即丢。bundle 的目标由 `WriteTarget::within(op, root, path)` 清出：`root` 是人给的根（导出时是 bundle 目录，恢复时是城根），只查 `path` 与它和 `root` 之间的每一级目录，不查 `root` 本身及其上——城根放在哪里是人选的，`/home` 指向 `var/home`、macOS 的 `$TMPDIR` 经 `/var` 这样的链接不是 run 能造出来的写路径。不在 `root` 之下的路径退回到查到文件系统根，所以界只会放宽人选的那一段。被否：沿用 `WriteTarget::at` 查到文件系统根（城只要放在链接下面，导出与恢复就整次失败）。runtime 的 edit 工具同一条理由：它以城根为界调 `within`，因为 macOS 的临时目录都在 `/var` 这个链接之下，查到文件系统根的写门在那里拒绝每一次编辑（`crates/runtime/src/tools/edit/tests.rs` 里放在链接下面的城那一条）。字节先写同目录的暂存文件 `.<name>.part`（这个拼写只在 `landing` 定义一处；导出不把合这个拼写的名字当城文件带走，因为恢复中途崩溃会把暂存文件留在城根下）、`sync_data`，再把原权限抄到暂存文件上，最后 `rename` 覆盖并 `sync_dir`。原权限按 `landing::Bits` 取：复制城文件时取源文件的（可执行位、只读位随文件走，导出再恢复后不丢），写清单与历史时取被覆盖文件的（没有就用新建默认值）。`rename` 换的是目录项，所以硬链接臂同样由此保证，且不存在「名已删、字节未落」的丢文件窗口。权限在 `sync_data` 之后才抄：只读位一旦落上，Windows 不再允许以写句柄打开该文件。被否：写前移除该名再建（失败即丢文件，且按 umask 新建，丢可执行位）；就地截断重写（写穿硬链接，崩溃留半个文件）。链接臂是拒绝不是跳过——对照组的 “skipped N files” 就是宽容部分还原。悬空链接在列举中不可见（`is_file`／`is_dir` 走不到它），但落盘必须先过 `WriteTarget`，故恒拒仍然成立。

### 8-14 storage::error（形状 2 值）

```rust
pub enum StorageError {                      // thiserror；crate 根
    Io { op: &'static str, path: PathBuf, source: io::Error },
    VersionAhead { path: PathBuf, v: u64 }, // 方向感知拒绝的机器面
    Envelope { path: PathBuf, line: u64, source: AxError },  // 段中损坏（断尾候选之外）
    Draft { source: AxError },              // 组 log_truncated 草稿失败（不以死变体粉饰）
    CasMissing / CasCorrupt / RangeOutOfBounds,               // cas
    SeqMissing,                                              // index
    Checkpoint / SecretEgress / Bundle,                      // 各自的模块
    Worktree / WorktreeBusy / MergeStale / MergeWouldDiscard, // worktree
    Alias { op: &'static str, path: PathBuf, kind: alias::AliasKind },  // → E_OUTSIDE_WRITE_DOMAIN（8-25）
    LedgerHeld { dir: PathBuf },            // 另一个 JsonlLedger 持着这座城的账本（8-1）→ E_LEDGER_HELD
    LedgerBroken { dir: PathBuf, at: Seq }, // 一波的写或 sync 失败过，重开前拒绝之后每一波（8-1）→ E_STORAGE_FATAL
    ChainHalted { source: AxError },        // 全链审计发现断链，写者停止接新行（8-27）；码取审计自己的 source
    Snapshot { op: &'static str, path: PathBuf, source: io::Error },  // 快照读写被盘拒绝（8-26）→ E_STORAGE_FATAL，恢复说的是快照
}
impl StorageError { pub fn into_ax(self) -> AxError; }   // 跨 crate 边界的唯一出口
pub(crate) fn io_err(op: &'static str, path: &Path) -> impl FnOnce(io::Error) -> StorageError;
```

- **为什么是独立模块**：见 §7，全 crate 的模块都用它。
- **为什么带着 `io_err` 走**：它是 `StorageError::Io` 的构造子，而一个值的构造子与它的定义同住。四个模块（cas／bundle／digest_cache／index）只为取它而 import jsonl，那是一条指错了方向的依赖。
- **快照的 I/O 失败有自己的变体**：`Io` 的恢复建议说的是账本（停机，重开会截掉撕裂的尾巴），对快照是错的——快照是账本随时能重建的缓存。`Snapshot` 与 `Io` 同码（盘拒绝了写，多半账本也写不进），恢复则说：删掉这份快照、腾出盘，下次启动从创世折叠。被否：让 `Io` 的恢复按 `op` 分支——一个变体两种建议，读恢复的人得先知道 `op` 的全集。
- **公开名是 `storage::StorageError`**（`lib.rs` 重导出）：模块住处不进公共面，api-baseline 不随它动。

### 8-15 storage::vfs（形状 3 端口）

```rust
pub(crate) trait Vfs {                      // 内缝：不出对外接口，不升真缝
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()>;
    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>>;     // 排序后返回：遍历确定性
    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>>; // 同为浅层：遍树用显式工作表
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn size(&self, path: &Path) -> io::Result<u64>;             // 段长；index 比长度而不抬字节
    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>>;  // 定位读；短答＝文件到头
    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()>;
    fn sync_data(&mut self, path: &Path) -> io::Result<()>;
    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;
    fn sync_dir(&mut self, dir: &Path) -> io::Result<()>;       // Windows no-op（§3-3）
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;                      // cas 去重与重开容忍需要
}
```

- **`read_at` 只报事实，不判越界**：短答表示文件在那里结束，「要的范围超出了对象」这句话归提出范围的模块（§8-3）。端口若自己拒绝，Locator 文法就有了第二个权威。
- **`FaultFs` 另记 `bytes_read`**：op 计数答「碰了几次盘」，字节计数答「抬回来多少」，而范围读与整读的唯一区别就是后者。`FaultFs::bytes_read()` 是范围读不整读这条断言的观测面。
- **端口的符合性套件就是断电点阵**（§8-2）：两个适配器对同一组语义负责，`fault_fs` 存在本身就是这条缝的存在证明（§8.5 设计 A）。
- **仍然不升真缝**：`pub(crate)`，`JsonlLedger` 把它藏在 `Box<dyn Vfs>` 后面，公开签名里一次不出现（否则 E0445）。
- **谁在缝外，以及为什么**：不经 `Vfs` 的读写属下列几类，每一类的理由写在它下面。
  - `index::reader::OpenSegment`——按 seq 取单行时持住段句柄。`Vfs::read_at` 每次调用开一次文件，
    按 seq 逐行取时，每行重开一次文件的代价远大于顺着持住的句柄读。
    它只读不写，而缝要建模的是崩溃语义，对一次定位读无话可说。
  - `worktree`——树由 git2 建、由 git2 prune，落盘不经本 crate；`release` 只解 git 的锁，不删目录；开城的 `sweep_abandoned` 用 `std::fs` 删城自己造的树目录（§8-9）。
    缝拦不住 git2，声称拦得住才是第二个权威。释放顺序与自愈见 §8-9。
  - `checkpoint`、`changes`、`hunks`、`status`、`blob`——同样经 git2 提交、检出与读对象。
  - `chain_audit`——逐段流式读整条链，只读不写（§8-27）。
  - `jsonl::ledger::WriterLock`——文件锁是操作系统对打开句柄的事实，Vfs 的崩溃语义模型对它无话可说（§8-1）。
  - `alias`——重解析点与链接计数是 `std::fs` 自己的事实（`file_type().is_symlink()` 对 symlink 与 junction 同真，Unix `MetadataExt::nlink` 判硬链接），Vfs 的崩溃语义模型对它们无话可说；它只问元数据、不读不写字节。
  - **`index` 不在例外之列**：`LedgerIndex` 持 `Box<dyn Vfs>`，段列举、段长、整段读（建表）与尾部增量读（刷新）全部经缝，
    而它没有一步是写；按 seq 取行走 `LineReader`，那正是上一条的例外。

### 8-16 storage::real_fs（形状 4 适配器）

```rust
pub(crate) struct RealFs { open: Option<OpenAppend> }   // std::fs 直译，零策略
impl RealFs { pub(crate) fn new() -> RealFs; }
impl Vfs for RealFs { … }
```

- **唯一的状态是那个句柄**：追写与 sync 走同一个句柄，所以被做持久的就是刚写的那些字节，也省掉每条记录两次重开（§7）。
- **句柄命名的是文件不是路径**：`rename`／`remove_file`／`truncate` 前必须 `release`。两条断言随这个模块走，它们问的是「之后字节落在哪个文件里」。
- **`rename` 是原子替换，在 Windows 上也是**：Windows 拒绝把文件改名到一个只读文件上（拒绝访问），而 `Vfs::rename` 的契约是替换。所以在 Windows 上，改名因无权被拒且目标是只读文件时，先清掉目标的只读位再改名一次；别的原因的拒绝原样返回。成功路径不多花一次系统调用。两步之间崩溃，目标内容不变、只丢只读位；暂存文件已带原权限（8-25），重做一次落盘即复原。被否：在 `bundle::landing` 里先清目标的位——那要给端口加一个方法，而规则属于「替换」本身，所有经 `rename` 的替换都该守它。

## 8.5 两个设计

**A（选中）：Vfs 内缝＋FaultFs 注入**——故障面在文件系统语义层注入，jsonl/cas 的产品代码零测试钩子。杠杆：一套故障模型服务两模块；断电点阵是 Vfs 语义的性质，不是某模块的分支。
**B（落选）：jsonl 内置故障开关**（`#[cfg(test)]` 的注入点散布各写步）——不需要 Vfs 抽象，但故障语义与产品逻辑同居一文件，点阵无法复用给 cas，且「测试钩子进产品代码」违背「测试与产品走同一道门」。落选理由：内缝的存在证明是第二适配器，不是一句声明。

## 9 工作流程

装配（消费者＝测试与 citysim）：构造 Vfs → `JsonlLedger::open`（断尾自愈）→ 作为 `&mut dyn kernel::Ledger` 交记录方 → 波到即 `append_all`。CAS 旁路：offload/attach 字节 `put`，Locator 携 hash 跨会话，`get/get_range` 取回。

## 10 实现逻辑

1. 行终止符恒 `\n`（含末行）；chain_hash 对不含 `\n` 的行字节计算（kernel-SPEC §8-9）。`.gitattributes` 的 `* text=auto eol=lf` 令索引与每个平台的检出都是 LF，夹具字节因此跨平台一致。
2. open 的段校验用 `EventRecord::parse_line`＋`chain_hash` 复算，无独立解析器（一个权威）。
3. 段内偏移不建索引（storage::index 的事）；`read_raw_lines` 全量读，消费者只有 replay/夹具/conformance。
4. `list` 排序返回＋段名零填宽度 20：字典序＝数值序，跨平台遍历确定。
5. `truncate` 后同步；整段截空直接 `remove_file`，重开容忍残留空段文件（崩溃窗口的两态都可解析，比 rename 舞步少一个中间态）。另：最后一段首行即损坏且仅此一段时，截至 0 字节＝回到新 Ledger（append 未曾返回 Ok 即无可观察效果）；非尾段损坏才是 Envelope 错误。
6. FaultFs 的 io::Error 用 `ErrorKind::Other`＋自述文本；jsonl/cas 对错误只透传包裹为 `Io{op,path}`，不吞不换。

## 11 边界枚举

空目录首开（新 Ledger）；末段恰好整段损坏（截空删段、退至前段）；首段首行即损坏（Envelope 错误——创世行不可断尾，宁停不脏）；波跨滚动边界（两段各一次 sync）；`append_all(vec![])`（no-op，Ok(空)）；同内容并发 put（各写各的 tmp，rename 到同一目标）；本进程另开句柄时别的句柄正有 put 在途（不清本进程的 tmp）；get_range 恰触界（`B` 端点＝len-1 合法）；`L` 范围起于超出总行数（越界拒）；v 低于当前（v<1 不存在，按 Envelope 拒）；夹具目录只读（A16 只读不写回）。

## 12 Decisions

- **临时件按「内容哈希＋写者进程＋本进程的 put 序号」命名，开句柄只清别的进程写的临时件。** 一个进程同时开着多个 `Cas` 句柄（驱动 run 的各条 lane、读图、浏览器工具、前缀视图各开各的），若临时件只以内容哈希命名、且每次开句柄都清空 `tmp/`，则后开的句柄会删掉另一个句柄写好尚未 rename 的临时件（rename 报 `NotFound`，run 以 `E_STORAGE_FATAL` 停下），两个句柄同写一份字节时一方的 `truncate` 还会截掉另一方正要 rename 的文件、给对象留下错误内容。按写者分名后两种撞车都不存在；残留只可能来自已经退出的进程，而同一座城同时只有一个进程（`LedgerHeld`），所以 pid 不同即残留。代价：本进程里 put 失败留下的临时件要到下次启动才清。被否：全进程只开一个句柄（每个开句柄的调用方都得改，且下一个新调用方仍会踩中）；在 put 里遇 `NotFound` 重写一次（仍不防 `truncate` 截断别人的文件）。**重开参数**：若允许两个进程同时写同一座城的 CAS，改为按锁或租约判定残留。
- `VersionAhead`→`E_LOG_VERSION_UNSUPPORTED`：不可定义掉——二进制升级与数据寿命天然错位；方向感知拒绝即其最小语义。
- `CasCorrupt`→`E_CAS_CORRUPT`：不可定义掉——位腐烂与外部改动在本设计边界外；能定义掉的部分（写路径半成品）已由 tmp+rename 定义掉。
- `CasMissing`→`E_PATH_NOT_FOUND`：不可定义掉——Locator 是跨会话引用，对象可被更早的介质事故清除；nearby 给同前缀既存对象。
- `RangeOutOfBounds`→`E_INVALID_ARGS`：可部分定义掉——`Range` 构造已保 `from<=to`；对象长度只在读时可知，读时校验是剩余的不可消部分。
- `Io`→`E_STORAGE_FATAL`（宁停不脏路径；不可定义掉——介质失败在设计边界外）。
- `WorktreeBusy`→`E_WORKTREE_BUSY`：可定义掉但尚未做——当「领节点」本身变成取租约（`storage::queue` 已有队列），busy 就从错误变成排队。它同时承担「该节点的树被占」与「再开一棵就越上限」两个情形：两者的可执行替代同为「先归还一棵」，而区分它们的是 subject 不是码。
- `MergeStale`→`E_VERSION_CONFLICT`：不可定义掉——两个节点同时开工就会有一个后到；能定义掉的那部分（“合到一半失败”）已由 fast-forward 判定在动手之前定义掉。
- `MergeWouldDiscard`→`E_VERSION_CONFLICT`：不可定义掉——人的未提交改动在城市目录里，机器无权决定它与节点的活谁留下；能定义掉的「静默覆盖」已由 SAFE 检出定义掉。
- `Worktree`→`E_STORAGE_FATAL`：不可定义掉——仓库与文件系统是外部世界；能定义掉的那部分（名字走出目录）已由 `WorktreeName` 在构造点定义掉。
- `Alias`→`E_OUTSIDE_WRITE_DOMAIN`：不可定义掉——名字与它指向的文件之间隔着一个链接是外部文件系统的事实；能定义掉的那部分（一次写入经链接穿透）已由 `WriteTarget` 在构造点定义掉，recovery 恒为「换成普通文件后重试」，故被拒的 run 不会卡死。Unix 上硬链接臂就在这个码下（`nlink>1` 即拒）；Windows 上链接计数不可判定（§3.5），由落盘纪律拆别名而非报拒。
- `LedgerBroken`→`E_STORAGE_FATAL`：不可定义掉——写与 sync 的失败来自介质；能定义掉的那部分（失败之后再写的一波被下次 open 截掉，却已答了 `Ok`）已由 `Barrier` 定义掉。recovery 是修好盘之后重启，由 open 修段尾。
- `Envelope`→`E_LOG_VERSION_UNSUPPORTED` 同族拒读（段中损坏非尾部＝不可自动修复，指出路径交人决定）。
- `LedgerHeld`→`E_LEDGER_HELD`（装载期）：不可定义掉——两个进程打开同一座城，是人的两个普通动作（双击两次、两个终端各跑一次 `up`／`serve`／`resume`）。能定义掉的那部分已经定义掉：锁先于一切读写，被拒的一方不会先写下任何东西。recovery 说明持锁的是另一个 sprawling 进程，以及怎样停下它。
  锁路径的权威是 jsonl：`WriterLock::take` 从账本目录自己推出同级的 `<目录名>.lock`，不问 `CityLayout`。原因：会话切片路径只有 `storage::sessions` 一个权威，而从账本目录反推城根是 `of_ledger` 的活；另一种做法——由调用方传入锁路径——要改 `open` 的签名和它的每个调用方，却只换来同一个文件名。代价是夹具、bench、fuzz 的账本目录也各多一个锁文件，而「一个目录一个写者」对它们同样成立。

## 13 依赖选型

kernel（workspace 内层）；`thiserror`；`blake3`（经 kernel 的 chain_hash／cas 自身 hash——直接依赖，B.7 钉版）；`serde_json`（envelope 探查）。dev：`proptest`、`tempfile`（RealFs 测试隔离目录）。不引 walkdir（Vfs::list 一层足矣）。
形状 7 的重建骨架住 `tests/derived_views.rs`（`tests/` 不受 modmap 辖，与 kernel/runtime 既有集成测试同例），一次定义两次实例化（index／hot）。
`git2`（checkpoint、worktree 与读仓库的各模块；libgit2 vendored，链接例外在 `deny.toml` 登记；版本由根 `Cargo.toml` 与 `Cargo.lock` 给出）。git2 不入缝：崩溃安全委托它的事务，只测重建与孤儿清扫。

## 14 硬编码声明

`SEGMENT_ROLL_BYTES = 64 MiB`（内部事务，非 consts_policy——对上层不可见，改它不改任何行为语义，只改文件切法）；段名前缀 `ledger-`＋20 位零填；CAS 分片取 hex 前 2；tmp 后缀 `.part`。均为 pub(crate) 常量，改动随本 SPEC。

## 15 影响面

runtime::replay 读 `read_raw_lines`；citysim 夹具对拍与断电点阵消费 FaultFs；index 挂账本目录布局。trait 边界 Io 映射为 Io→E_STORAGE_FATAL。

## 16 测试与约束

单测：open 六步各分支；滚动边界；append_all 原子性（注入 Io 后内存态不前进）；cas put/get/get_range/dedup。proptest：链续与断尾（任意截断点/垃圾尾）；FaultFs 点阵（cut_at_op 扫描）。夹具：A16 高版本拒读。conformance：JsonlLedger 过 kernel 六断言。约束：clippy 零告警；fault_fs 在非 test/fault 构建中零字节。

## 17 模型体验

零字节：本 crate 恒不进 prefix；模型可见面只有经 tool_result 携带的 AxError（如 E_CAS_CORRUPT 的 three-part 拒绝），其余全部是落盘内部事务。

### 8-19 storage::hunks：一个文件的补丁文本（形状 4 adapter）

```rust
pub struct PatchLine { pub number: u32, pub text: String }
pub struct Withheld  { pub number: u32, pub reason: String }
pub struct FilePatch { pub lines: Vec<PatchLine>, pub withheld: Vec<Withheld> }

pub fn of_file(city_root: &Path, base: GitOid, head: Head, path: &str)
    -> Result<FilePatch, StorageError>;
```

**补丁文本是 `storage::changes` 之外的一次独立请求。** `storage::changes` 只计数、不搬补丁文本；一段补丁必须是它自己的一次请求，经同一次扫描作答。本模块就是那次请求，逐字兑现它开出的三个条件：

1. **一次一个文件**，`path` 必填，没有「整批补丁」这个形状。理由是代价：`changes` 的代价与改动文件数同阶，本函数与一个文件的大小同阶，合成一个答会让「这次改了哪些文件」付上整批补丁的钱。
2. **同一次凭证判定**，不是第二份。`checkpoint::scan_staged` 用 `kernel::scan` 判一个 staged blob，本模块判每一行补丁文本用的是同一个函数。命中的行**不回显**，只报行号与命中原因（provider 名，或熵判定）——理由与 `scan_staged` 对自己的命中说的同一句：把字节打出来以证明泄漏，本身就是泄漏。
3. **两端都是 commit 时，答可永久缓存**（同 `changes` 的理由）；`Head::WorkingTree` 答的是此刻的工作树，那是一波还没提交完的样子，也正是审阅进行中的改动时人看的那一份。

- **一个两次 checkpoint 之间没动过的文件答空补丁**，而不是报错：「它没动」是一个答案。「这座城没写过这个 oid」是另一个答案，由调用方（`accounting::views`）答 `Unavailable`——只有调用方知道人问的是什么。
- **测试用工作树而不是第二次 checkpoint**：checkpoint 根本不肯提交带凭证的 blob（`scan_staged` 拒），所以那一行只可能存在于盘上的树里。这条约束本身就是本模块的扫描不是多余的一层的证据：字节到不了 commit，但到得了 socket。

### 8-20 重启后凭证扫描的比较基准

进程重启后 `Checkpoint::last` 为空，`scan_staged` 退回按 HEAD 比较，而这**漏扫不了重启之前的改动**：wave checkpoint 恒不移动 HEAD，故 HEAD 只可能是 `ensure_base` 或 `land` 写下的提交——它必是最后一次 checkpoint 的**祖先**。拿祖先做基准，diff 出来的路径集是拿 checkpoint 做基准那一集的**超集**：读得更多，不会更少。代价是把已经放行过的 blob 再读一遍，安全上一分不让。

**一条断言守着它**：`a_change_made_before_a_restart_is_still_read_after_it`——立城、checkpoint 一次、写入一份带凭证的文件、丢掉 handle、重开 `Checkpoint`、再 checkpoint，第二次 checkpoint 必须以 `SecretEgress` 拒绝并报出路径而不回显字节。**翻案条件**：哪一天有一条路径能在不经扫描的情况下移动 HEAD（今天 `ensure_base`／`land` 都先扫后提交，合并提交用的是节点已扫过的树），这条推理的前提就没了，届时基准必须改回记住的 checkpoint。

### 8-21 `bundle` 的城夹具住一处（`storage::bundle::fixture`）

`city_with(records, root)`——立一座有 N 条记录的城、写两个文件、开一次 CAS——在 `bundle/export.rs`、`bundle/files.rs`、`bundle/manifest.rs` 的测试模块里**逐字节重复三遍**。三份拷贝就是三个「一座城长什么样」的权威：改其中一份，另外两份的断言仍在对着旧形状作证。

**一个夹具一处**：`crates/storage/src/bundle/fixture.rs`，`#[cfg(test)]` 编译，由 `bundle.rs` 以 `#[cfg(test)] mod fixture;` 挂上，三个测试模块 `use super::super::fixture::city_with;`。形状 4 适配器（它造的是被测代码之外的一个真实环境）。**不放进 `bundle.rs` 自身**：索引文件不持逻辑，而夹具是逻辑。

### 8-22 `storage::status`：还没被检查点收走的那些改动，以及仓库此刻站在哪

`between` 比的是调用方已经握着的两个点，答不出仓库自己站在哪：哪个分支被检出、它有没有上游、它跑出上游多远——这三件是人在问「哪些文件动了」之前先问的。

```rust
pub struct Drift { pub ahead: u64, pub behind: u64 }
pub struct WorkingStatus { pub branch: Option<String>, pub drift: Option<Drift>,
                           pub files: Vec<FileChange> }
pub fn working_status(city_root: &Path, scope: Option<&str>, base: Option<GitOid>)
    -> Result<WorkingStatus, StorageError>;
```

**四条口径：**

1. **`base` 由调用方点名，因为检查点不动 HEAD。** `checkpoint::wave_pre` 把提交挂在 `refs/sprawling/` 之下，HEAD 停在基提交或上一次落地处；照 HEAD 比会把这座城跑过的每一次 wave 都报成「未提交」。`None` 退回 HEAD，那是一座还没立过检查点的城所拥有的全部。
2. **未跟踪文件照样成行。** 一个 agent 写下又从未入暂存的文件，恰恰是人要找的那个;只列已跟踪改动的清单会把一个新模块报成什么都没发生。为此 `changes::collect` 的 `Untracked` 归入 `How::Added`——工作树有而没有任何提交有的文件，就是这次加出来的。
3. **`scope` 是一条 pathspec 而不是事后过滤。** 楼页问的是它自己那些文件，让 git 在走差异时就收窄，比走完全城再筛一遍少一趟盘。
4. **`drift` 整个可缺席。** 没有上游、上游被删、以及处在游离头上，对读者而言是同一件可做的事（没有可比的对象），而与「和上游齐平」不是一回事。

### 8-23 `storage::cas::ranges`：Locator 范围文法住一处（形状 4 适配器）

`get_range` 的文法——`B` 0 起闭区间、`L` 1 起闭区间、行间 `\n` 保留、末行终止符不返回、越界拒不夹取——是一套读法，不是 CAS 的存取。它因此住 `crates/storage/src/cas/ranges.rs`，`Cas::get_range` 只解析对象路径、判定存在，再把 `&dyn Vfs` 与路径交给 `of_object`。

**块长 64 KiB 是本模块的内部事务**（`SCAN_CHUNK_BYTES`）：`L` 式要知道第几行从哪开始，只能从对象开头扫换行，于是代价与**答案之前**的字节同阶，而与对象大小无关——一份 200 MB 的 offload 取第二行，读的是 64 KiB。`B` 式一次定位读即可，不必扫。

**这套文法没有第二个家**：`Cas::get` 仍是唯一会复算 BLAKE3 的读法，`ranges` 一次也不哈希。两条断言守着这件事——`byte_and_line_ranges_follow_locator_semantics` 守文法，`a_range_read_lifts_the_range_rather_than_the_object` 守代价（用 `FaultFs::bytes_read()` 数字节：一个五字节范围移动的字节数必须以十计，而不是以对象长度计）。

### 8-24 `storage::sessions`：账本投影到各楼的 sessions（形状 7 投影）

```rust
pub(crate) struct Sessions { /* layout、ledger、vfs、open、first_seen —— 私有 */ }
impl Sessions {
    pub(crate) fn for_ledger(ledger_dir: &Path) -> Option<Sessions>;   // 非城账本 → None
    pub(crate) fn absorb(&mut self, record: &EventRecord) -> Result<(), StorageError>;
}
pub(crate) const SLICE_MAGIC: &str = "slices v1";
```

**账本不拆链。** 一条链、一个写者、一本完整历史（`docs/glossary.md` 的 one Ledger 与 one writer）一个字不改；工作区里出现的是一份**从账本投影出来、可删可重建的切片**：一个 room 一个文件，落在它所属 Building 的 `.sprawling/sessions/` 下，地址的嵌套就是文件的嵌套。**城自己的那条记录（地址即城名）不落任何切片**：那个地址是城，不是城里的 Building，给它落一份就会在城根里造一个与城同名的目录（kernel-SPEC 8-56 的 `city_address`）。路径由本模块的私有函数 `session_slice` 从 `CityLayout::root` 推出，目录名 `SESSIONS_DIR` 也是私有常量；crate 内唯一对外的是谓词 `pub(crate) fn is_session_projection(relative: &Path) -> bool`，供检查点判断一条路径是不是切片。可见性就是这句话的守卫：别的模块写不出切片的路径，也就写不出切片的读者。被击败的替代是把路径留在 `kernel::layout` 再用文本扫描拦住别的调用点——扫描只认拼写，绕开一次别名就放行。

**只有账务线程写，而且写在账本落盘之后。** 入口是 `JsonlLedger::append_all`：一段 wave 全部 `sync_data` 之后才逐条 `absorb`。投影写失败不改变账本的返回值——历史已经在盘上，可弃物不得让它的调用方吃到一次假失败——但拒绝被报出（一行 `eprintln!`）而不是被吞掉。**投影永不进检查点。** `checkpoint::stage_scopes` 跳过任何 `sessions` 下的路径（`is_session_projection`）：它还在被账务线程追加，而 git 的暂存要读它以为已经知道的文件；一个仍在长的文件会把整波拒成 `E_WORKTREE_BUSY`（8-8）。可弃物因此从不进历史，也不进任何 diff 的读者面。

**头部是唯一的疑点。** 每份切片首行是 `slices v1 <first_seq>`，即该文件第一条记录的 seq。魔数不符、或首行 seq 与头部不符 → 整份重建：扫账本、按地址过滤、按账本序写下，不写迁移代码。文件不存在时先问常驻的 `first_seen`（地址 → 账本里它的第一条 seq）：第一条就是正在 absorb 的这条，账本里没有更早的行，切片就从这一条落下，一个段也不读；否则按上一句整份重建。进程重启后接上已有文件时，先校验头部与内容，再从账本把漏掉的尾巴补齐；段名带首 seq，故不可能含新记录的段不打开。断在半行的尾巴先截掉再续，与账本自己的 tail recovery 同一条读法。

**新 room 不扫段。** `first_seen` 在本进程第一次遇到缺文件的切片时从各段折叠一次，此后由每次 `absorb` 增量写入，所以派往新 room 的活只付一次 `exists` 与一次追加，而不是每个新 room 都把整本账本读一遍再解析——那样的答案永远是「除了这一条什么都没有」。被否：打开账本时就折叠——每个打开城账本的进程（包括只追加一行的命令）都要多读一遍全部段，而多数进程从不遇到缺文件的切片。剩下的一次折叠仍是整段读取，把它搬到 lane 上或并进启动时已有的 verify 那一遍，是 §3 的开放项。

**重建逐字节相同。** 每一行都是账本 `canonical_line` 的副本，头部由内容决定（首条记录的 seq），所以删掉整个 `sessions/` 再放一遍得到同样的字节——`deleting_the_sessions_directory_and_replaying_the_ledger_restores_the_bytes` 钉住它。

**产品不读它做判断。** 它只给人的眼睛与城外 agent 的 `read`；`runtime` 的 `read` 工具照旧拒绝 `.sprawling`，城里别的模块连它的路径都拼不出（路径推导是 `storage::sessions` 的私有项）。

**自带的 `RealFs`，不借账本那条缝。** `RealFs` 同一时刻只持一个追加句柄，写投影会把账本的热句柄挤掉；投影可弃，不该让主干为它付句柄开销。代价是投影不参与 `FaultFs` 的断电模型：断电后它可能落后，下一次 attach 补齐，这正是它可重建的含义。

**没有第二份 session 索引。** 「全城有哪些 session」走目录遍历；切片在追加时写，写者当场就知道 room 地址，索引只会在两者之间造出一个可失效的家。

### 8-25 `storage::alias`：别名族与被清空的写目标（形状 2 值）

```rust
pub enum AliasKind { Link, HardLink }   // Link＝symlink 与 junction（Win32 重解析点族）
pub struct WriteTarget(PathBuf);        // 字段私有；构造点是 at 与 within
impl WriteTarget {
    pub fn at(op: &'static str, path: &Path) -> Result<WriteTarget, StorageError>;   // 查目标与每一级父目录
    pub fn within(op: &'static str, root: &Path, path: &Path) -> Result<WriteTarget, StorageError>;  // 只查到 root 为止；bundle 与 runtime 的 edit 工具用它
    pub fn as_path(&self) -> &Path;
}
pub(crate) fn kind_at(path: &Path) -> Result<Option<AliasKind>, StorageError>;   // 叶级分类，walk 用
```

- **别名族全不穿透（junction／symlink／硬链接）。** junction 与 symlink 都是重解析点、`file_type().is_symlink()` 对两者同真，故合为 `Link`，在每一扇门**字面拒绝**。硬链接在 Unix 由 `nlink>1` 判定、同样字面拒绝；在 Windows 本 crate 今天不读链接计数（§3.5：有一条安全的读法，是否采用未定），由**写入恒落新 entry** 的落盘纪律兜住：`bundle::landing::land` 把同目录的暂存文件 `rename` 覆盖该名，`runtime::tools::edit` 写前移除该名再建；两者都换掉目录项，其它名字保有旧字节，穿透在结构上不可能。**被否：跳过并报数**（对照组 “skipped N files”）——部分落盘破坏全成/全拒，且一行计数无法让重放方复现跳过了哪几个。
- **`WriteTarget` 是形状 2 值：不变量在唯一构造点，字段私有。** 「未经检查的写目标拼不出来」由 trybuild 编译失败反例钉住（`tests/ui/`）。它的证明范围是「检查那一刻这个名与它的父级都不是链接」；检查与落盘之间的替换窗口属效果面，故两个采样点（walk 与写入）都过同一判定。Unix 上硬链接在判定内被拒；Windows 上判定今天不读链接计数，落盘纪律另兜该臂。
- **消费面是三个写域加一个工具写面**：checkpoint 暂存回调（8-8）、bundle 的 `landing::land`（8-12；`worktree::back::restore_file` 也经它落盘，8-27）、worktree 放置，加上 `runtime::tools::edit` 的物理写入（运行的写域）——经链接写保留路径在每一扇门恒拒。
- **proptest 族「别名永不落盘」**：对别名种类 × 目标（受保护／普通）× 落点（名上／父目录）的组合，凡该平台造得出的别名（junction 无需特权即可创建；symlink 需特权；硬链接随处可造）：链接臂与 Unix 硬链接臂写入被拒，Windows 硬链接臂写入落新 entry；各臂同一断言——目标字节不变、别名带不出新字节；该平台造不出的退化为断言「放置失败时盘上无任何变化」，各臂同性质。

### 8-26 `storage::snapshot`：链哈希快照（形状 7 投影）

```rust
pub struct ChainSnapshot { /* fold_version, seq, line_hash, views：字段私有 */ }
impl ChainSnapshot {
    pub fn cut(fold_version: u32, seq: Seq, line: &[u8], views: Vec<u8>) -> ChainSnapshot;  // line_hash = chain_hash(line)
    pub fn fold_version(&self) -> u32;
    pub fn seq(&self) -> Seq;
    pub fn views(&self) -> &[u8];
    pub fn fit(&self, line_at_seq: &[u8]) -> SnapshotFit;              // 定位读到的那一行是否就是切点那一行
    pub fn resume(&self) -> Result<LineCheck, AxError>;               // 切点之后的链状态：prev = line_hash，expected = seq + 1；seq 已是最后一个时是 Seq::next 的错误
}
pub enum SnapshotFit { Fits, Stale }
pub enum StoredSnapshot { Absent, Damaged(String), Present(ChainSnapshot) }
pub fn write_snapshot(dir: &Path, snapshot: &ChainSnapshot) -> Result<(), StorageError>;
pub fn read_snapshot(dir: &Path) -> Result<StoredSnapshot, StorageError>;
```

- **快照是投影，不是历史。** 它放在 `<city>/.sprawling/snapshot/`，内容是 `(fold_version, seq, line_hash, views_bytes)`：折叠在第 `seq` 行之后持有的状态，连同那一行的链哈希。删掉它，城照样起得来，只是回到全量折叠；所以文件损坏（魔数不对、长度不够、体摘要不符）读成 `Damaged(原因)` 交给调用方丢弃，只有 I/O 本身失败才是 `StorageError`。
- **文件格式**：`SPRSNAP1` 八字节魔数｜`fold_version` u32 LE｜`seq` u64 LE｜`line_hash` 32 字节｜`views` 的 blake3 32 字节｜`views` 到文件尾。体摘要让位翻转读成 `Damaged` 而不是一份错的视图；写入走「临时文件 → sync → rename → sync 目录」，所以撕裂的写不会留下半个快照。
- **核对只看一行。** 启动时按 `seq` 做一次定位读，`fit` 比较那一行的 `chain_hash` 与 `line_hash`：相等时，在摘要单射的前提下（`crates/kernel/spec/Ledger.lean` 持有这条假设），快照所折的行就是盘上账本的前缀；`Stale` 时调用方丢弃快照，全量折叠。`fold_version` 不等同样丢弃：折叠规则变了，旧状态不再是新规则折出来的。
- **「快照加尾部 ≡ 全量折叠」** 由 `crates/storage/spec/Snapshot.lean` 对任意折叠、任意切点证明（`snapshotPlusTailIsWhole`、`resumeIsWhole`）；Rust 侧由 proptest 在随机账本与随机切点上持有同一性质，折叠取链检查本身：从 `resume()` 出发走尾部，接受的行与终态都等于从创世走全程。
- **被否：只存 `seq` 不存 `line_hash`。** 账本被换成另一条同长的链时，只比 `seq` 会把别人的视图接到这条链的尾部上；多 32 字节换来的是一次定位读就能拒绝。
- `Views` 从这里起步并由服务起步时切快照（sprawling-SPEC 8-91）；`Standing` 仍从创世折叠。

### 8-27 `storage::chain_audit`：全链审计与写者停机（形状 7 投影：把整条链折成一个判定）

```rust
pub enum ChainAudit { Whole { lines: u64 }, Broken(AxError) }
pub fn audit_chain(dir: &Path) -> Result<ChainAudit, StorageError>;   // 只读，从创世走到最后一个完整行
#[derive(Clone, Default)]
pub struct ChainHalt { /* Arc<OnceLock<AxError>> */ }
impl ChainHalt {
    pub fn trip(&self, reason: AxError);          // 第一次有效，之后的原因丢弃
    pub fn reason(&self) -> Option<&AxError>;
}
impl JsonlLedger { pub fn halt_on(&mut self, halt: ChainHalt); }
```

- **按段流式，一次只持一行。** `audit_chain` 按 `ledger_segments_at` 的顺序逐段打开，用一个复用的行缓冲逐行喂给 `LineCheck`（8-1 的同一个逐行检查，没有第二份规则），所以内存与账本长度无关；文件末尾没有 `
` 的撕裂字节不是一行，留给 `open` 判（与 `read_raw_lines_at` 同一口径）。第一行不过就停，答 `Broken(fault.into_ax(行号))`：原因、行号与恢复办法都是人读得懂的，页面把它原样作为诊断显示。I/O 本身失败才是 `StorageError`。
- **审计是查询，停机是命令。** `audit_chain` 不改任何状态；调用方（后台线程，唯一的起点在 `bin::assembly`）拿到 `Broken`、或审计没读完（`StorageError`）时调 `ChainHalt::trip`：从快照起步的视图只有这次审计会看快照之前的行，没被证明的链与断链一样不能再往后写。停机值在写者与视图之间共享：写者经 `halt_on` 接上同一个值，之后每次 `append_all` 在组帧之前先问它，已跳闸就返回 `StorageError::ChainHalted`，`into_ax` 原样交出那条审计原因，所以被拒的写与页面诊断说的是同一句话。
- **只能跳闸，不能复位。** `OnceLock` 让「停机后又恢复写」在类型上不可表达：链断了，接在断链后面的每一行都是在错的历史上写的；复位要人修好账本后重开这座城，那时是一个新的 `ChainHalt`。**被否：`AtomicBool`**——它能被写回 `false`，而且带不出原因。
- **被否：审计一失败就 panic 退出进程。** 进程没了，页面也就收不到原因；停写不停读，人还能看见城停在哪、为什么停。
- 服务中的城由 `bin::assembly::chain_watch`（sprawling-SPEC 8-90）起这条后台线程并接上停机值；视图只折写者写下的记录，所以不另接停机值。

### 8-28 `storage::snapshot::start`：从快照起步还是从创世起步（形状 7 投影：决定折叠从哪一行起）

```rust
pub enum SnapshotStart {
    Resume { snapshot: ChainSnapshot, tail: Vec<Vec<u8>> },   // 快照合身：它的 views，再折 seq 之后的这些行
    Whole(WholeFold),                                          // 从创世折，并说明为什么没用快照；行不在这里
}
pub enum WholeFold { NoSnapshot, Damaged(String), OtherFoldVersion { found: u32 }, Stale, Missing }
pub fn start_from_snapshot(ledger_dir: &Path, snapshot_dir: &Path, fold_version: u32) -> Result<SnapshotStart, StorageError>;
```

- **一次读既核对又取尾部。** 快照的 `seq` 按段名（`segment_first_seq`）定位到含它的那一段：它之前的段一个字节也不读；从这一段起逐段取完整行，第 `seq - 段首 seq` 行交给 `ChainSnapshot::fit`，它之后的行就是尾部。所以起步的读量是「尾部加一段的前缀」，与账本总长无关；整条链的核对留给 `audit_chain`（8-27）。
- **不能用快照时一律退回全量折叠，并带上原因。** 没有快照（`NoSnapshot`）、字节不是快照（`Damaged`，原样带出 8-26 的原因）、`fold_version` 不等（`OtherFoldVersion`）、那一行的链哈希不同（`Stale`）、账本里根本没有那一行（`Missing`，账本比快照短）——都是 `Whole`。`Whole` 只带原因：从创世折叠的调用方经 `runtime::replay::fold_ledger_dir` 一段一段地读、每行过 `LineCheck`（runtime-SPEC §8-1），常驻的是一段字节与一条记录，与没有快照时的起步完全相同。原因给调用方写诊断用；它们都不是错误，因为快照只是投影。I/O 本身失败才是 `StorageError`。
- **被否：`Whole` 带回账本的全部行。** 调用方不读它们：它要的是流式折叠，那些行一到手就被丢掉，账本却已经整本进过一次内存，峰值随历史长度增长。
- **被否：在快照里再存该行的字节偏移，做真正的单次 `pread`。** 偏移指向的是字节，不是链：换过的账本在同一偏移可能正好有一行，核对仍要看链哈希；而尾部本来就要从那一段读，省下的只是一段内的前缀扫描，却让 8-26 的格式多一个会随段滚动失效的字段。

### 8-27 `storage::worktree::back`：回到过去，与从某一点取回一个文件（形状 4 适配器；git2）

```rust
impl Worktrees {
    /// 从 point 分叉：一棵新树，分支 `name` 起于 point；城的 HEAD 与干线不动。
    pub fn claim_at(&self, name: &WorktreeName, point: &GitOid) -> Result<WorktreeLease, StorageError>;
    /// 把 point 上的 path 取回 lease 这棵树；point 上没有这个文件即删掉它。
    /// 返回调用者要追加进账本的 `file_restored` 记录。
    pub fn restore_file(&self, lease: &WorktreeLease, point: &GitOid, path: &Path) -> Result<FileRestored, StorageError>;
}
```

**统一历史不另建存储。** 城的历史只有两份已有的东西：只追加的账本，与城仓库里写下就不再变的 git 对象。log 是血缘树，diff 是两点之间对话与文件一起的差别，blame 是 `whose`，合并走已有的 PR 流（8-9）；本节只管其中两个会写盘的动作。

**性质由 Lean 模型定。** `crates/storage/spec/Worktree/Back.lean`（`just models`）规定的性质分两处守。本模块守树的四条：回到过去得到的树恰是那一点的文件（`goBack_opens_at_the_point`）；名字已被一棵活树占着即拒，调用者自己的树也不被替换（`goBack_refuses_a_live_tree`）；回到过去、写、取回都只动发起它的那个 run 的树，从不动干线，于是一个 run 写下的内容在任何只含别的 run 的步骤序列之后原样还在（`others_never_touch_a_tree`、`a_write_survives_other_runs`）。账本的两条——每一步给账本追加恰好一条记录，撤销即取回、也是追加（`the_ledger_only_grows`、`undo_is_an_appended_restore`）——归调用者：本模块不写账本，但 `restore_file` 把它那一步的记录作为返回值交出（kernel 的 `FileRestored`，path 取 git 树的写法，所以同一次取回在任何机器上记成同样的字节），调用者追加它即是 `restored` 那一条；`claim_at` 那一步的记录是 kernel 的 `WentBack`（名字加 point）。

**回到过去不移动 HEAD。** 选「新 session ＋ 一棵停在那一点提交上的独立工作区」，否决「把城的 HEAD 检出到那一点」：后者会在别的 run 正写着的时候改掉它们落地的基线，而 Lean 模型里干线恒不被任何一步改动，正是这条的形式化。分支在那一点上新建，名字就是 `WorktreeName`，与 `claim` 同一套名字。

**拒绝，不覆盖。** `claim_at` 在三种情况下拒：名字登记着一棵活树，或者已有同名分支（那是一条已有的工作线，覆盖它就是冲掉别人的写入）→ `WorktreeBusy`（`E_WORKTREE_BUSY`，恢复：换一个名字）；point 不是城仓库里的提交 → `Worktree{op:"find the point to go back to"}`。城的工作树超过上限 → `WorktreeBusy`，与 `claim` 同一个预检（`refuse_oversized`）。拒后城内文件、干线与所有分支都不变；分支建好而检出失败时，这条专为它新建的分支随即删掉，否则它会以一条没人开始的工作线占住这个名字。这次删除本身失败时，分支留下，返回的 `Worktree{op:"remove the branch of a tree that did not open"}` 同时带着检出失败的原因：调用方知道有一条孤儿分支占着这个名字，换一个名字即可继续。检出与 `claim` 走同一个 `add_tree`，放树的位置与别名判定只有一处。它不像 `claim` 那样自愈一个目录已丢的登记：回到过去总取新名字，碰上旧名字就是调用方的错。

**取回只写自己的树。** `restore_file` 只接受相对路径且不含 `..`，不接受 `RESERVED_PREFIX` 之下的路径，也不接受任何以 `.` 或空格结尾、含 `:`、或含 `~` 后跟数字的段——Win32 把这些拼写折叠到另一个名字上（`.git.`、`.git `、`.git::$DATA` 都指向树的 `.git` 链接，卷生成 8.3 短名时 `GIT~1` 也是），逐段比较挡不住它们；写入目标是 `lease.path()` 下的那个文件，经 `alias::WriteTarget` 判定（8-25）。point 上是 blob 即按原字节经 `bundle::landing::land` 落盘（同目录暂存、`sync_data`、抄原权限、`rename` 覆盖、`sync_dir`，8-25），所以经硬链接指向别的树或干线的名字只被换掉目录项，那一头的字节不动，崩溃也不留半个文件；point 上没有即删除，这就是「恢复到那一点」的含义；目录与子模块不是一个文件，拒。

**现状。** 本模块是统一历史的第一段。「分叉」与「取回」的事件种类（`went_back`、`file_restored`）已在 kernel 事件表里。其余几段尚不存在：服务端把账本加 git 投影成一棵血缘树的读者面，以及网页上把楼页的提交、改动、回收站与对话页的分叉合成一页的「历史」页。它们到来之前，`claim_at` 与 `restore_file` 没有生产调用者。
### 8-29 `storage::blob`：一次提交里一个文件的字节（形状 4 adapter）

```rust
pub fn blob_at(city_root: &Path, oid: GitOid, addr: &Address) -> Result<Option<Vec<u8>>, StorageError>;
```

- 读城仓库里 `oid` 那次提交的树上 `addr` 处的 blob；那里不是文件（目录、子模块、不存在）＝`Ok(None)`，由调用方说出拒因——它知道是谁问的。仓库打不开、提交找不到＝`StorageError::Checkpoint`。
- 不碰工作区与索引：`file:<addr>@<oid>` 指的是提交里的字节，工作区此刻的文件可能已经改过。
