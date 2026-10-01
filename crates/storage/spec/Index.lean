-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::index

规定 `index`、`index::ledger`、`index::reader`、`index::fold`、`index::fold::entries`（`crates/storage/src/` 下同名的文件）。Ledger 旁挂索引 seq→（段，偏移）与 run 表；可弃，存疑即重建。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
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

- **索引随视图快照落盘，只作为投影**：建表只有一遍扫描（`index/ledger.rs` 的 `walk`），两个入口共用它：`rebuild` 自己定位每一行，`folding` 让折叠交回它已读出的位置；`Views` 持有它并在每次查询前 `refresh`。`LedgerIndex` 实现 `serde::Serialize`／`Deserialize`，只写 `folded`（列、列外行、段名字典、run 表与 `scanned`），读回时配一条新的 `RealFs` 缝；视图快照（sprawling-SPEC 8-91）连同它一起编码，所以从快照起步的城，第一次历史查询只 `refresh` 快照之后长出来的字节，而不是整本扫描一遍。它仍是投影：`scanned` 记着切快照那一刻每段已折入的长度，段变短或消失就整份重建（下文 `refresh` 的规则），快照删掉就从段重建。**被否：单独一个索引文件。** 那是第二个可失效的家，要自己的版本、自己的切点与自己的损坏判定；放进视图快照，它与视图共用 `fold_version`、体摘要与原子写。**被否：从快照起步后不带索引。** 第一次历史查询会整本扫描、逐行解析（40 万行的城是秒级），把开城省下的读取挪到第一个问历史的人身上。
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
-/

/-!
### 8-38 storage::index 按 seq 往后读（`LedgerIndex::seqs_from`）

```rust
impl LedgerIndex {
    /// 索引里 seq 不小于 `from` 的每一条，升序；两端都可取。`seqs()` 就是 `seqs_from(Seq::FIRST)`。
    pub fn seqs_from(&self, from: Seq) -> impl DoubleEndedIterator<Item = Seq> + '_;
}
```

- **答什么**：`seqs()` 里不小于 `from` 的那一段，次序不变（`seqs_from_answers_the_held_seqs_at_or_after`）；也就是调用方原来写的 `seqs().skip_while(|seq| *seq < from)` 交出的那一串（`seqs_from_is_the_walk_past_the_smaller`）。`from` 之后没有行时答空，`from` 不必是写过的 seq。
- **不看 `from` 之前的格**：seq 是隐式的（8-4），`from` 所在那一格就是 `from - base`，读者从那一格起走；列外行是有序表，从 `BTreeMap::range(from..)` 起走。起步的代价是一次减法与一次 O(log 列外行数) 的查找，之后每交出一个值走一格（空洞不交出，但要走过）。答案不取决于 `from` 之前的任何一格（`seqs_from_reads_no_slot_before_its_start`），所以从某个 seq 往后读一段的代价只与那一段同阶，而与它前面的账本长度无关。
- **消费者**：`accounting::trace` 判同楼的别人时，从区间的下界读到上界（accounting-SPEC.md 8-25）；每个提交一次，有了这个读者，一次导出在这一项上的代价是各区间长度之和，而不是提交数乘行数。
- **列外行与归并**：与 `seqs()` 同一个归并，列与列外行各从 `from` 起；`index/fold/tests.rs` 的 map 形 oracle 对每个探测的 `from` 判等（正反两向）。本模型只写列，列外行是有序表的区间查询，没有自己的算术。

D23 从某个 seq 往后读由索引给，而不由调用方跳过前面的。理由：列里一行的位置就是它的 seq 减 `base`，索引知道 `from` 在哪一格，调用方只能从第一格数过去；`skip_while` 在每个提交上都把区间之前的全部 seq 走一遍，一次导出就是提交数乘行数。被否决的做法：①给一个闭开区间的读者 `seqs_in(Range<Seq>)`——上界是调用方的条件（`take_while` 在第一个越界的值上停），放进索引不省一格，只多一个参数；②在 playback 里留一份 walk 走过的 seq 表，按下界二分——那是同一批行的第二份索引，要与 `LedgerIndex` 各自维护；③按 run 读（`run_seqs_before`）再合并——同楼的别人是全部 run，按 run 读要先知道有哪些 run。重开参数：有调用方要从某个 seq 往前读（例如 `view --follow` 从尾部倒着取新长出来的行）时，再看要不要一个从上界往前的读者；今天的 `seqs().rev().take_while(..)` 只走新长出来的那几行，不随账本长度增长。
-/

namespace Storage.Index

/-- seq 列：从 `base` 起每格一个槽，`true` 是有行，`false` 是空洞（Rust 的 `HOLE`）。第 `i` 格的 seq 是 `base + i`，seq 本身不存。 -/
structure Column where
  base : Nat
  slots : List Bool

/-- 从 seq `low` 那一格起逐格读，有行的格交出它的 seq。 -/
def heldFrom : Nat → List Bool → List Nat
  | _, [] => []
  | low, true :: rest => low :: heldFrom (low + 1) rest
  | low, false :: rest => heldFrom (low + 1) rest

/-- `seqs()`：列里的每一条，升序。 -/
def Column.seqs (c : Column) : List Nat := heldFrom c.base c.slots

/-- `seqs_from(start)`：直接到 `start` 所在那一格（`start` 在 `base` 之前时就是第一格），它之前的格一格也不读。 -/
def Column.seqsFrom (c : Column) (start : Nat) : List Nat :=
  heldFrom (c.base + (start - c.base)) (c.slots.drop (start - c.base))

theorem held_from_starts_at {low x : Nat} {slots : List Bool} (h : x ∈ heldFrom low slots) :
    low ≤ x := by
  induction slots generalizing low with
  | nil => simp [heldFrom] at h
  | cons s rest ih =>
    cases s with
    | true =>
      simp only [heldFrom, List.mem_cons] at h
      cases h with
      | inl here => omega
      | inr later => have := ih later; omega
    | false =>
      simp only [heldFrom] at h
      have := ih h
      omega

/-- 一串从 `low` 起的值，丢掉小于 `m` 的，`m ≤ low` 时什么也不丢。 -/
theorem held_from_drops_nothing_below_its_start (slots : List Bool) :
    ∀ (low m : Nat), m ≤ low →
      (heldFrom low slots).dropWhile (fun x => decide (x < m)) = heldFrom low slots := by
  induction slots with
  | nil => intro low m _; simp [heldFrom]
  | cons s rest ih =>
    intro low m below
    cases s with
    | true =>
      simp only [heldFrom]
      rw [List.dropWhile_cons_of_neg (by simp only [decide_eq_true_eq]; omega)]
    | false =>
      simp only [heldFrom]
      exact ih (low + 1) m (by omega)

/-- 跳过前 `k` 格再读，与读完再丢掉小于 `low + k` 的值相同。 -/
theorem held_from_after_dropping (slots : List Bool) :
    ∀ (low k : Nat), heldFrom (low + k) (slots.drop k)
      = (heldFrom low slots).dropWhile (fun x => decide (x < low + k)) := by
  induction slots with
  | nil => intro low k; simp [heldFrom]
  | cons s rest ih =>
    intro low k
    cases k with
    | zero =>
      simp only [Nat.add_zero, List.drop_zero]
      exact (held_from_drops_nothing_below_its_start (s :: rest) low low (Nat.le_refl low)).symm
    | succ k =>
      have shifted : low + (k + 1) = (low + 1) + k := by omega
      cases s with
      | true =>
        simp only [List.drop_succ_cons, heldFrom]
        rw [List.dropWhile_cons_of_pos (by simp only [decide_eq_true_eq]; omega), shifted]
        exact ih (low + 1) k
      | false =>
        simp only [List.drop_succ_cons, heldFrom]
        rw [shifted]
        exact ih (low + 1) k

/-- `seqs_from(start)` 交出的，就是 `seqs()` 跳过小于 `start` 的那些之后剩下的：调用方原来的 `skip_while` 一字不差。 -/
theorem seqs_from_is_the_walk_past_the_smaller (c : Column) (start : Nat) :
    c.seqsFrom start = c.seqs.dropWhile (fun x => decide (x < start)) := by
  unfold Column.seqsFrom Column.seqs
  rw [held_from_after_dropping c.slots c.base (start - c.base)]
  by_cases before : start ≤ c.base
  · have none_skipped : start - c.base = 0 := by omega
    rw [none_skipped, Nat.add_zero,
      held_from_drops_nothing_below_its_start c.slots c.base c.base (Nat.le_refl _),
      held_from_drops_nothing_below_its_start c.slots c.base start before]
  · have reaches : c.base + (start - c.base) = start := by omega
    rw [reaches]

/-- 答的是索引里不小于 `start` 的每一条，升序。 -/
theorem seqs_from_answers_the_held_seqs_at_or_after (c : Column) (start : Nat) :
    c.seqsFrom start = c.seqs.filter (fun x => decide (start ≤ x)) := by
  unfold Column.seqsFrom Column.seqs
  have answer : ∀ (slots : List Bool) (low k : Nat),
      heldFrom (low + k) (slots.drop k) = (heldFrom low slots).filter (fun x => decide (low + k ≤ x)) := by
    intro slots
    induction slots with
    | nil => intro low k; simp [heldFrom]
    | cons s rest ih =>
      intro low k
      cases k with
      | zero =>
        simp only [Nat.add_zero, List.drop_zero]
        refine (List.filter_eq_self.mpr ?_).symm
        intro x held
        simpa using held_from_starts_at held
      | succ k =>
        have shifted : low + (k + 1) = (low + 1) + k := by omega
        cases s with
        | true =>
          simp only [List.drop_succ_cons, heldFrom]
          rw [List.filter_cons_of_neg (by simp only [decide_eq_true_eq]; omega), shifted]
          exact ih (low + 1) k
        | false =>
          simp only [List.drop_succ_cons, heldFrom]
          rw [shifted]
          exact ih (low + 1) k
  rw [answer c.slots c.base (start - c.base)]
  apply List.filter_congr
  intro x held
  have := held_from_starts_at held
  simp only [decide_eq_decide]
  constructor <;> intro <;> omega

/-- 改动 `start` 之前的任何一格都不改变答案：读者从 `start` 那一格起走，前面的格它不读。 -/
theorem seqs_from_reads_no_slot_before_its_start (c c' : Column) (start : Nat)
    (sameBase : c.base = c'.base)
    (sameFromStart : c.slots.drop (start - c.base) = c'.slots.drop (start - c'.base)) :
    c.seqsFrom start = c'.seqsFrom start := by
  unfold Column.seqsFrom
  rw [sameFromStart, sameBase]

/-- 一列：seq 3 有行，4 是空洞，5、6 有行。从 5 起读得 5、6；从 1 起读得全部。 -/
example : (Column.mk 3 [true, false, true, true]).seqsFrom 5 = [5, 6] := by decide
example : (Column.mk 3 [true, false, true, true]).seqsFrom 1 = [3, 5, 6] := by decide

end Storage.Index
