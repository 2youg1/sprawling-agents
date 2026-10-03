-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::jsonl

规定 `jsonl`、`jsonl::ledger`、`jsonl::open`、`jsonl::boundary`、`jsonl::first_line`、`jsonl::reading`、`jsonl::append`、`jsonl::unwind`、`jsonl::tail`（`crates/storage/src/` 下同名的文件）。kernel Ledger 的落盘实现：组提交、断尾、版本方向判定、分段滚动、写者锁，以及开账本时按摘要证明末段的已验证前缀。逐行检查住 `crates/storage/spec/Jsonl/Verify.lean`，屏障状态住 `crates/storage/spec/Jsonl/Barrier.lean`。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。逐行检查、屏障与预分配段的性质是被证明的，住 `crates/storage/spec/Jsonl/Verify.lean`、`crates/storage/spec/Jsonl/Barrier.lean` 与 `crates/storage/spec/Jsonl/Preallocate.lean`。
-/

/-!
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
    /// 一波的写或 sync 失败过，此后每一波都答 `LedgerBroken`，直到 `jsonl::unwind` 把段退回到那一波之前，或者重开。
    /// 链证明判出断链之后（`chain_audit::ChainHalt`，8-27），每一波都答 `ChainHalted`。
    pub fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, StorageError>;
    pub fn read_raw_lines(&self) -> Result<Vec<Vec<u8>>, StorageError>;   // 实例读面
    /// 写路径观察者（至多一个，后装者取代前装者）。只在**整波持久化完成后**逐条回调：
    /// 观察者因此看不到一条未落盘的事件，而推给界面的事实就是历史里的那一行。
    pub fn observe(&mut self, sink: WriteObserver);   // WriteObserver = Box<dyn FnMut(&EventRecord) + Send>
    pub fn position(&self) -> Seq;    // 现在写一条会落在哪；只给位置不给内容
}
/// 只读读面，给要原始行的读者（分叉、夹具与测试）：不走 open、不触发断尾与任何写——验证恒不修盘（`crates/runtime/Spec.lean` §8-1）。
/// 只要结论的读者不经它：`sprawling replay` 走 `audit_chain`（8-27），折叠走 `LedgerIndex::folding`（8-4）；两者都一次只持一段字节。
pub fn read_raw_lines_at(dir: &Path) -> Result<Vec<Vec<u8>>, StorageError>;
/// 目录里的账本段，按应读顺序（`list` 已排序，段名零填充故字典序即时序）。
/// 空结果的意思是「这里没有账本」，与「账本里没有事件」不是同一件事；
/// `read_raw_lines_at` 对两者都答 `Ok([])`，故需要区分的调用方问这一面。
/// 现在只有一个：`sprawling replay`，它的路径是人敲的（sprawling D2）。
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
-/

/-!
### 8-1 storage::jsonl：落盘形态、写者锁、open 与 append_all

**落盘形态**：目录内 `ledger-<first_seq 20 位零填>.jsonl` 若干段；行＝`canonical_line`＋`\n`；链与 seq 跨段连续。滚动按行判，不按波判：当前段非空、且再写这一行会让它超过 `SEGMENT_ROLL_BYTES` 时，这一行起新段，所以一波可以跨两段（新段创建后 `sync_dir`）。按波判的写法会让一个大波把段撑过上限任意多；按行判，一段至多比上限多出一行，而首行本身超过上限时它独占一段。
**各平台的屏障**：`sync_data` 在 Windows 上是 `FlushFileBuffers`，在 Linux 上是 `fdatasync`，在 macOS 上是 `fcntl(F_FULLFSYNC)`（storage D24，`crates/storage/spec/Jsonl/Barrier.lean`）。`sync_dir` 在 Linux 与 macOS 上对目录做 `File::sync_all`；Windows 上是显式的 no-op（`crates/storage/Spec.lean` §3 第 3 条），新段的目录项在那里不经这一步落盘。
**写者锁：一个账本目录同一时刻只有一个 `JsonlLedger`，跨进程成立。** `open` 在列段之前，对账本目录的同级文件 `<目录名>.lock`（城的账本即 `<city>/.sprawling/ledger.lock`）取 `std::fs::File::try_lock` 独占锁；`JsonlLedger` 持着那个 `File`，锁与账本同寿命，drop 即放。拿不到锁就是别的 `JsonlLedger`（这个进程的或另一个进程的）正持着这座城的账本：`StorageError::LedgerHeld { dir }`，映射装载期码 `E_LEDGER_HELD`。拒绝发生在任何读写之前，所以被拒的一方不修盘，也不写 `log_truncated`。

```rust
pub(crate) struct WriterLock { /* 持锁的 File；只为它的 Drop 而存在 */ }
impl WriterLock {
    /// 锁文件＝`dir` 的同级 `<目录名>.lock`，由 jsonl 自己命名；`dir` 没有目录名（根、`..`）即 `Io`。
    pub(crate) fn take(dir: &Path) -> Result<WriterLock, StorageError>;
}
```

- **锁文件在账本目录旁边，不在里面。** 账本目录的读者把目录里的每一项都当历史：`has_history` 问「有没有条目」，`bundle` 逐文件拷贝。锁文件放进去，就会被当成一段历史拷进 bundle，而且 Windows 上另一个句柄读一个被锁住的文件会失败。`.sprawling/` 根是保留子树，本来就不随 bundle 走。
- **每个账本目录都锁，锁路径只由 `dir` 推出。** `open_faulty` 不取锁：FaultFs 的盘只存在于这个进程里。理由见 D18。
- **同一进程里的第二个句柄同样被拒。** `try_lock` 在 Windows 上是 `LockFileEx`，在 Unix 上是 `flock`，两者都按打开的句柄算，不按进程算。所以「一个进程里开两个 `JsonlLedger`」也被拒，这就是跨进程性质在单进程里可测的形式（`open/tests.rs` 的 `a_second_writer_of_a_city_is_refused_until_the_first_lets_go`）。
- **锁文件不删。** 放锁时删文件，会留下一个窗口：先到者还持着旧文件上的锁，后到者已经在同名的新文件上拿到了锁。一个空文件不花任何代价。
- **锁文件不在就建，连同 `.sprawling/`。** 建不了这个文件，或者操作系统答不了这次取锁（答的不是「已被持有」），都是 `Io`→`E_STORAGE_FATAL`，subject 是锁文件的路径：连锁文件都写不了的城，也写不了账本。
- **被否：把锁放在 `Vfs` 缝上。** 那要给 `FaultFs` 造一个进程模型，而它的盘本来只有一个进程到得了。锁是真实文件系统上的事实，只在生产入口 `open` 上取。
- **被否：把 PID 写进锁文件。** Windows 上别的进程读不了被锁住的文件，拒词因此报不出 PID；recovery 改为说明怎样停下持锁的那个进程。
- **重开参数**：出现不经 `JsonlLedger::open` 写账本的生产路径；或者 `std` 的 `try_lock` 改为按进程而不是按句柄判定。

**open 六步**：①列段排序；②空目录＝新 Ledger（next_seq=FIRST、prev=GENESIS_PREV）；③读首段首行验 `v`（只读这一行：`jsonl::first_line` 用 `Vfs::read_at` 从 4 KiB 的窗口读起、每次加倍，见到第一个 `\n` 或文件尽头即停；不见 `\n` 的首行是撕裂行，答「无行」，交给尾部恢复。整段读进来再切第一行，会让单段账本在 open 时被读两遍）——判定一律经 `kernel::consts_external::readable_log_v`：`Ahead` 即 `VersionAhead`（先于一切链检，恒不部分解读），`NotAVersion`（低于任何构建写过的首版本，含 v0）即 `Envelope` 且拒词说版本，`Current` 与 `Older` 放行；④校验最后一段：逐行 parse＋段内链续（服务中的城经 `open_reusing` 开账本，末段的已验证前缀按摘要证明、只逐行核对记录之后的行，8-34），本段任一可解析行的 `v` 同样经 `readable_log_v` 判定——`Ahead` 与 `NotAVersion` 在此**拒**而不作尾损截断（截掉它等于删掉更新构建的历史），首个非法字节起截断（`truncate`＋`sync_data`）；段尾连续的零字节是预分配的空间，不是撕裂（`crates/storage/spec/Jsonl/Preallocate.lean`）：扫描前先剥掉，不计入截掉的字节、不写 `log_truncated`，但同样截去，好让下一波接在最后一条记录之后（今天 `Vfs::append` 写在文件末尾）；零之后还有带信封的字节时那串零是段内的损伤，拒开而不截；末段的入口取自前段末行（`jsonl::boundary`）：从前段末尾按 `BOUNDARY_WINDOW`（16 KiB）一块向前读，见到这一行之前的换行（或段首）即停，所以读量与这一行同阶，不与段长同阶；这一行过 `LineCheck::judge`，与正向检查同一条规则——已知 kind 类型解析并比对规范回显，`ig:true` 的未知 kind 只读信封——入口的 `prev` 是它的 `chain_hash`，下一个 seq 是它的 seq 加一；判不过即拒，`VersionAhead` 按版本拒，其余为 `Envelope`，行号是它在前段里的行号；⑤若截掉字节>0（含「截空整段即删段文件」的退化情形），append 一条 `log_truncated`（run=CITY、who=`Who::City`——开账本是城自己的活，"system" 这第四种写法已删、data 由 `kernel::event::record::LogTruncated` 拼写为 `{"dropped_bytes":n}`）；⑥恢复 next_seq/prev 内存态。
**append_all 五步**：逐 draft：seq=next、`EventRecord::from_draft`、`canonical_line`、必要时滚段；写段；单次 `sync_data`（跨段波对每个触及段各一次）；更新 prev/next_seq；铸 refs。任何 Io 错误⇒整波失败，内存态不前进，盘上的段也退回本波之前（`jsonl::unwind`，形状：adapter）：当前段截回 `seg_len` 并 `sync_data`，本波新建的段删去并对目录 `sync_dir`。退回因此和一波一样落盘：退回之后、下一波之前掉电，删掉的段不会回来，截掉的字节也不会回来（`unwind/tests.rs` 的 `a_restored_tail_survives_a_power_cut`）。Windows 上截断之后是 `FlushFileBuffers`，删段之后的 `sync_dir` 是 no-op（目录项的持久由 NTFS 的元数据日志负责）；Linux 与 macOS 上是 `sync_data` 与对目录的 `sync_all`。退回本身失败时留作待办，下一波写之前先完成它；完成不了，那一波以退回的 Io 错误失败而不写一个字节。于是写失败留下的半行永远不会成为下一波的前缀，进程活着也好、重启也好，账本都是一条不断的链；open 的断尾只剩掉电一种来源。
- 被否：写失败后把账本置为只读、直到重启。它同样不写坏账本，但盘满时腾出空间以后城仍要重启才能继续记账；退回只多一次 `truncate`，而且只发生在失败的那一波。重新考虑的条件：出现一个 `truncate` 不可靠、而 open 的断尾可靠的平台。
-/

/-!
### 8-34 开账本时，末段的已验证前缀按摘要证明（`storage::jsonl::open`、`storage::verified_prefix`，形状 7 投影）

```rust
impl JsonlLedger {
    /// `open` 的尾部恢复，末段的前缀由 `records` 里那一段的记录按摘要证明，只逐行核对记录之后的完整行。
    pub fn open_reusing(dir: &Path, now: TimeMs, records: &ProofRecords) -> Result<(Self, OpenReport), StorageError>;
}
pub struct OpenReport {
    pub recovered: Option<TailTruncation>,
    pub counted: ProofCount,   // 尾部恢复读了什么、算了什么（8-30 的同一个计数）
}
```

- **为什么。** 尾部恢复（8-1 第 ④ 步）逐行核对整个末段，末段至多 64 MiB。40 万行夹具城的末段 40,843,081 B，逐行核对它是开城的 `open the ledger` 那一段的几乎全部（`crates/sprawling/Spec.lean` §8-144 的读数）；同一段读一遍、BLAKE3 一遍加起来不到它的十分之一。末段的前缀已经被后台的证明逐行核对过，并写下了记录（8-30）。
- **四个条件，与证明相同。** `open_reusing` 照 `open` 取锁、探版本、从前一段的末行取入口状态，再把末段读进内存一次。`records` 里有末段的记录，且版本等于 `line_check_version()`、段长不短于记录的 `L`、记录的入口等于前一段末行给出的链状态、前 `L` 字节的 BLAKE3 等于记录的摘要时，链状态取记录的出口，从第 `L` 字节起逐行核对；四个条件有一个不成立，整段逐行核对，与 `open` 相同。被哈希的字节与之后被核对的字节出自同一次读（8-30 第一条）。判定 `chain_audit` 的 `Walked::reuse` 已经写过一次，`open_reusing` 调同一个判定，不写第二份。
- **截断与拒开的判定不变。** 记录只覆盖证明时逐行核对过的完整行，所以前 `L` 字节里没有撕裂与断链，截断只可能落在 `L` 之后；`crates/storage/spec/Jsonl/Barrier.lean` 的 `reopenFromVerifiedPrefix` 陈述从这样一个前缀起重开与从头重开相同，`crates/storage/spec/Snapshot.lean` 的 `cachedVerifyIsStrict` 陈述出口状态与逐行核对相同（末段看作「记录覆盖的前缀」与「之后的字节」两段）。截断之后段比 `L` 短，记录就不再适用。
- **开账本只读记录。** `open_reusing` 不写记录、不删记录；记录只由持锁的证明写（8-30）。`records` 由调用方给出：记录目录的唯一拼法是 `accounting::views::snapshot::start::proof_dir`（`crates/sprawling/Spec.lean` §8-122），本 crate 不从城布局再推一次。服务中的城经 `open_reusing` 开账本（`crates/sprawling/Spec.lean` §8-144）；其余打开者照旧走 `open`。
- **读数是计数。** `OpenReport.counted` 与证明的 `ProofCount` 同形：逐行核对的行数、按摘要复用的段数（0 或 1）、尾部恢复读过的字节（末段的长度）、哈希过的字节。记录命中时逐行核对的行数等于证明之后写下的行数，与历史长度无关；`jsonl::open::tests` 在两种规模上断言它。
- **被否：命中记录时不哈希，只比段长。** 段在两次开城之间被截短再接着写、或被外部改写同样长度时，段长照样相符；不哈希就接受了 8-30 要挡的那种意外。
-/

/-! D18 `LedgerHeld`→`E_LEDGER_HELD`

`LedgerHeld`→`E_LEDGER_HELD`（装载期）：不可定义掉——两个进程打开同一座城，是人的两个普通动作（双击两次、两个终端各跑一次 `up`／`serve`／`resume`）。能定义掉的那部分已经定义掉：锁先于一切读写，被拒的一方不会先写下任何东西。recovery 说明持锁的是另一个 sprawling 进程，以及怎样停下它。

锁路径的权威是 jsonl：`WriterLock::take` 从账本目录自己推出同级的 `<目录名>.lock`，不问 `CityLayout`。原因：会话切片路径只有 `storage::sessions` 一个权威，而从账本目录反推城根是 `of_ledger` 的活；另一种做法——由调用方传入锁路径——要改 `open` 的签名和它的每个调用方，却只换来同一个文件名。代价是夹具、bench、fuzz 的账本目录也各多一个锁文件，而「一个目录一个写者」对它们同样成立。
-/

/-! D22 开账本借用证明写下的记录，不另起一种记录

尾部恢复要的事实——末段某个前缀已逐行核对、走完它的链状态是什么——正是证明为每一段写下的那一条（8-30），所以开账本读同一条记录、用同一个判定（8-34）。**被否：开账本时为末段另写一条「已恢复到此」的记录。** 那是第二个回答「这段字节核对过没有」的地方，两处的版本与失效规则要各自维护；而且开账本的进程在证明之前写它，会让一条没有被证明线程看过的前缀被当成证明过的。**重开参数**：服务之外的打开者（`resume`、`fork`、`adopt`）也要毫秒级开账本时，它们同样经 `open_reusing`，记录照旧只读。
-/
