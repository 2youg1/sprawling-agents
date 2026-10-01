-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::chain_audit

规定 `chain_audit`、`chain_audit::wave`、`verified_prefix`（`crates/storage/src/` 下同名的文件）。从创世逐段证明整条链、已验证前缀按摘要复用、一波至多八段并行读，以及断链时让写者停下的停机值。已验证前缀与按波读段的性质住 `crates/storage/spec/Snapshot.lean`，本分部引用而不复写。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-30 `storage::chain_audit` 与 `storage::verified_prefix`：证明整条链，已验证前缀不重算（形状 7 投影：把整条链折成一个判定）

```rust
// storage::chain_audit
pub enum ChainAudit { Whole { lines: u64 }, Broken(AxError) }
/// 一次证明读了什么、算了什么：读数，不是时间，所以在 N 与 2N 两种规模上都能断言。
pub struct ProofCount { pub lines_checked: u64, pub segments_by_digest: u64, pub bytes_read: u64, pub bytes_hashed: u64, pub waves: u64 }   // waves 见 8-37
pub struct Proven { pub audit: ChainAudit, pub counted: ProofCount, pub unkept: Option<AxError> }
pub fn prove_chain(dir: &Path, records: &ProofRecords) -> Result<Proven, StorageError>;   // 带记录走整条链
pub fn audit_chain(dir: &Path) -> Result<ChainAudit, StorageError>;                        // 不带记录：每一行都逐行核对
#[derive(Clone, Default)]
pub struct ChainHalt { /* Arc<(Mutex<Option<Verdict>>, Condvar)>、判定之前放不放行 */ }
impl ChainHalt {
    pub fn awaiting_proof() -> ChainHalt;          // 判定之前拒绝每一次追加（服务中的城）
    pub fn prove(&self);                           // 判定为完好；第一个判定有效
    pub fn trip(&self, reason: AxError);           // 判定为断链；第一个判定有效
    pub fn reason(&self) -> Option<AxError>;
    pub fn proved(&self) -> bool;
    pub fn await_verdict(&self);                   // 阻塞到有判定为止；默认的停机值立刻返回
}
impl JsonlLedger {
    pub fn halt_on(&mut self, halt: ChainHalt);
    pub fn await_verdict(&self);
}

// storage::verified_prefix
pub struct ProofRecords { /* 记录目录；能不能写 —— 私有 */ }
impl ProofRecords {
    pub fn read_only(dir: &Path) -> ProofRecords;  // ask、replay、resume、fork：读记录，从不写
}
impl JsonlLedger {
    pub fn proof_records(&self, dir: &Path) -> ProofRecords;   // 持写者锁的 handle 给出能写的，其余给出只读的
}
pub fn line_check_version() -> u32;                // blake3(CARGO_PKG_VERSION ‖ LINE_CHECK_RULES) 的前四字节（LE）
```

- **按段走，一段一次读。** `prove_chain` 按 `ledger_segments_at` 的顺序逐段读进整段字节，这一份字节既交给逐行核对，也交给 BLAKE3：记录里的摘要就是被核对过的那些字节的摘要，核对与摘要之间没有两次读取可能读到两份内容的窗口（`ProofCount.bytes_read` 等于各段长度之和）。段按波读（8-37），常驻的是一波的段字节。文件末尾没有 `\n` 的撕裂字节不是一行，留给 `open` 判。第一行不过就停，答 `Broken(fault.into_ax(行号))`；I/O 本身失败才是 `StorageError`。
- **每段一条记录，各自判定。** 记录是 `<记录目录>/<段文件名>.proof`，内容为 `SPRPRF01` 八字节魔数｜规则版本 u32｜前缀长度 `L` u64｜前缀里的行数 u64｜入口（`prev` 32 字节、下一 seq u64）｜出口（同形）｜前 `L` 字节的 BLAKE3｜以上各字段的 BLAKE3。走到一段时，记录只在四个条件都成立时代替逐行核对：版本等于本构建的 `line_check_version()`、段长不短于 `L`、记录的入口等于走到这里的链状态、前 `L` 字节的摘要等于记录的摘要；这时链状态直接取记录的出口，`L` 之后的完整行仍逐行核对。任一条件不成立，这一段整段逐行核对，后面的段照样各自判定，因为入口状态是重新走出来的，失配不能绕过段间的连接。记录的字节不是记录（魔数、长度、体摘要不符）即当作没有。性质由 `crates/storage/spec/Snapshot.lean` 的 `cachedVerifyIsStrict` 定：记录都由逐行核对写下、摘要在段上单射时，带记录的判定就是逐行核对的判定；`withoutLinkAcceptsSplice` 与 `withoutVersionAcceptsStale` 说明入口与版本缺一不可。
- **记录对任何一段的前缀有效，不对「封好的段」有效。** 崩溃后被删掉的末段会让前一段重新成为尾段并接着长（8-1 的尾部恢复），所以记录记的是「前 `L` 字节」，不是「这一段」：段变长，前 `L` 字节照旧按摘要证明，之后的字节逐行核对，证明完再写一条覆盖新长度的记录。`Barrier.lean` 的 `reopenFromVerifiedPrefix` 陈述从这样一个前缀起重开与从头重开相同。
- **只有持写者锁的进程写记录。** 能写的 `ProofRecords` 只能从一个持着写者锁的 `JsonlLedger` 取得（`proof_records`）；`ProofRecords::read_only` 读记录、从不创建。所以 `ask`、`replay` 与一次性命令从不写盘（sprawling-SPEC 8-91「读命令不写盘」）。写记录走「临时文件 → sync → rename → sync 目录」，与快照同一写法；写不成的记录不改变判定，第一条失败放进 `Proven.unkept` 交调用方报告。
- **规则版本不靠人记得改。** `line_check_version()` 取 blake3(`CARGO_PKG_VERSION` ‖ `LINE_CHECK_RULES`) 的前四字节。常量写成 `line-check-<16 位十六进制>`，后缀是一份固定夹具（经写者写下的几行，加上改坏的、`ig:true` 的与版本超前的几行）逐行判定结果的 blake3 前缀，`chain_audit::tests` 的 `line_check_rules_pin_the_verdicts_of_a_fixed_fixture` 在判定变了时给出新值并失败，所以同一版本内改了逐行规则（事件表、规范回显）而忘了改常量，测试变红而不是沿用旧判定。
- **读数是计数。** `ProofCount` 在证明里累计：逐行核对的行数、按摘要复用的段数、读过的字节、哈希过的字节。记录全部命中时，逐行核对的行数等于记录之后长出的行数，与历史长度无关（`chain_audit::tests` 在两种规模上断言）；读与哈希的字节等于整条链的长度，这是这份保证的代价。
- **判定是查询，停机值是命令。** `prove_chain` 不改停机值；调用方（服务中的城，唯一的起点在 `bin::assembly::chain_watch`，sprawling-SPEC 8-122）拿到 `Whole` 调 `prove`，拿到 `Broken`、或证明没读完（`StorageError`）调 `trip`。停机值在写者与证明线程之间共享：写者经 `halt_on` 接上它，之后每次 `append_all` 在组帧之前先问它——已跳闸答 `StorageError::ChainHalted`，`into_ax` 原样交出那条原因；还在等判定而这个停机值是 `awaiting_proof()` 造的，答 `StorageError::Unproven`，映射 `E_HISTORY_UNPROVEN`（kernel-SPEC §8-1）。`ChainHalt::default()` 在判定之前放行：`resume`、`fork`、`adopt` 与 citysim 的写者在打开时已经同步证明过历史（sprawling-SPEC 8-101），不需要等后台。
- **判定只有一个，不能复位。** 判定在锁里从「无」写成 `Whole` 或 `Broken` 就不再变：先证明完好、后又被叫停的停机值仍是完好，先跳闸的仍是断链；复位要人修好账本后重开这座城，那时是一个新的 `ChainHalt`。`await_verdict` 在条件变量上等判定，关城的写者用它：人在证明结束之前关城，交接那一行等证明有了结局再写，而不是被拒。**被否：`AtomicBool`**——它能被写回，而且带不出原因。**被否：审计一失败就 panic 退出进程。** 进程没了，页面也就收不到原因；停写不停读，人还能看见城停在哪、为什么停。
- **按波并行读段。** 各段的读与记录前缀的哈希在一波里并行，链状态仍按段序走；判定、计数与常驻内存见 8-37。
-/

/-!
### 8-37 证明按波读段：一波至多 8 段，各段的读与记录前缀的哈希并行，链仍按段序走（`storage::chain_audit`、`storage::verified_prefix`，形状 7 投影）

```rust
// storage::chain_audit
pub struct ProofCount {
    pub lines_checked: u64, pub segments_by_digest: u64, pub bytes_read: u64, pub bytes_hashed: u64,
    pub waves: u64,                // 读段的波数：段数除以 PROOF_WAVE 向上取整
}
const PROOF_WAVE: usize = 8;       // 一波的段数，也是一波的线程数（调用线程算一条）
// storage::verified_prefix
impl SegmentRecord {
    /// 记录的版本等于 `version`、`bytes` 不短于记录的 L 时，`bytes` 的前 L 字节。
    pub(crate) fn prefix<'a>(&self, bytes: &'a [u8], version: u32) -> Option<&'a [u8]>;
    /// 入口等于 `entry`、`hashed` 恰好哈希过那段前缀且摘要等于记录的摘要时，前缀里的行数与出口。
    pub(crate) fn stands(&self, entry: LineCheck, hashed: &blake3::Hasher) -> Option<(u64, LineCheck)>;
}
```

- **为什么。** 40 万行夹具城（376,383,853 B，6 段）上，后台证明 6 段全按记录命中、一行不逐行核对，仍要 390 ms：读一遍与 BLAKE3 一遍都在 `chain_watch` 的那一条线程上，接受命令（M3）因此落在开城起点之后 650 ms（sprawling-SPEC 8-154）。各段的读与哈希互不依赖：一段前缀的摘要只取决于这一段的字节。
- **两步。** 一波取段序上接下来的至多 `PROOF_WAVE` 段。第一步并行：每段读进整段字节、读它的记录；`prefix` 给出前缀时，把它哈希进一个 `blake3::Hasher`。第一段在调用线程上做，其余各段在 `std::thread::scope` 里各一条线程上做，一波全部 join 之后才走第二步。第二步按段序串行，与 8-30 的走法相同：`stands` 给出行数与出口时取出口，用同一个 hasher 接着哈希前缀之后的字节并逐行核对；不给就整段逐行核对、从头哈希。一段坏只让这一段退回逐行核对。
- **判定不变。** 第一步算的只是前缀的摘要，它是段字节的纯函数；版本、段长、入口、摘要四个条件仍由 `prefix` 与 `stands` 两个方法定，开账本（8-34）也经这两个方法判末段，判定只写一次。`crates/storage/spec/Snapshot.lean` 的 `wavesAreCached` 陈述：先把每段的摘要判定算好、再按段序走，与边走边算的 `cachedRun` 是同一个判定，所以经 `cachedVerifyIsStrict` 等于逐行核对。
- **核对与哈希仍是同一次读。** 每段只读一次，第二步核对的字节就是第一步哈希的那一份。
- **计数是确定的。** `waves` 只取决于段数，不取决于核数或线程完成的次序；`bytes_read` 等于各段长度之和；`bytes_hashed` 在 `prefix` 给出前缀时总含这段前缀，入口接不上时也含，因为第一步已经哈希过它。`chain_audit::tests` 在 N 与 2N 段上断言全部五个计数。
- **常驻内存。** 一波的段字节同时在内存里，至多 `PROOF_WAVE × SEGMENT_ROLL_BYTES`（8 × 64 MiB）；历史短于一波时就是整条账本。
- **读数。** 40 万行夹具城（6 段，windows-x86_64、16 核、NVMe、release）上证明从 390–395 ms 落到 119–121 ms；一波 4 段（两波）时是 200 ms（sprawling-SPEC 8-154）。一波 8 段因此胜过 4 段：多出的常驻内存只在段数超过 4 的城上付，换来的是这些城的证明少一波。
- **线程。** 这是库 crate 起线程的又一处（ARCHITECTURE.md §10 第 3 条）：作用域线程，一波 join 完才往下走，寿命不超过一次 `prove_chain`。起不了线程是 `StorageError::Io`（op `start a thread to read a segment`）；线程没有交回结果也是 `StorageError::Io`（op `read a segment on its own thread`）。两者都让证明没有走完，调用方照 8-30 跳闸。
- **被否：按块流式读前缀，每条线程只持一个小缓冲。** 常驻内存会小得多，但 `Vfs` 没有读进调用方缓冲的门：`read_at` 每块开一次文件、新分配一次；加一个 `Vfs` 方法要四个适配器各写一份。**被否：在一段之内并行哈希（blake3 的 `rayon` 特性，或 `hazmat` 的子树拼接）。** `rayon` 带一个线程池，是第二个起线程的地方，还要改锁文件；子树拼接要本 crate 自己维护 BLAKE3 的树形偏移。段间并行已经让 6 段的城一波读完。**重开参数**：一波的常驻内存成为问题（机器内存小于一波的字节），或一座城的段数常常少于核数、证明仍是 M3 的大头时。
-/

/-!
## 模型：停机值只有一个判定，判定之前按它的造法放行或拒绝

证明一条链之后的判定（`Whole` 或 `Broken`）由 `ChainHalt` 在写者与证明线程之间传递。性质：

* 第一个判定有效，之后的 `prove`／`trip` 都不改变它（`the_first_verdict_stands`），所以先证明完好、后又被叫停的停机值仍是完好，先跳闸的仍是断链；复位只能是一个新的停机值。
* `append_all` 在组帧之前问 `admit`：已跳闸答 `ChainHalted`（带第一次跳闸的原因），`awaiting_proof()` 造的停机值在判定之前答 `Unproven`，`default()` 造的在判定之前放行，判定为完好之后放行（`admission_follows_the_verdict`）。
* 一次证明的波数只取决于段数：段数除以 `PROOF_WAVE` 向上取整（`waves_depend_only_on_the_segments`），所以 `ProofCount.waves` 不随核数或线程完成的次序变（§8-37）。

已验证前缀的记录怎样代替逐行核对、按波先算摘要为什么不改变判定，由 `crates/storage/spec/Snapshot.lean` 的 `cachedVerifyIsStrict`、`wavesAreCached`、`wavesAreStrict` 证明；从记录覆盖的前缀起尾部恢复与从头恢复相同，由 `crates/storage/spec/Jsonl/Barrier.lean` 的 `reopenFromVerifiedPrefix` 证明。本分部引用它们，不复写。
-/

namespace Storage.ChainAudit

/-- 证明给出的判定；`Broken` 带着原因（Rust 里是一个 `AxError`）。 -/
inductive Verdict (Reason : Type) where
  | Whole
  | Broken (reason : Reason)
  deriving DecidableEq, Repr

/-- 判定之前写者怎么做：`Admit`（`ChainHalt::default()`，历史在打开时已同步证明过）或 `Refuse`（`ChainHalt::awaiting_proof()`，服务中的城）。 -/
inductive Before where
  | Admit
  | Refuse
  deriving DecidableEq, Repr

structure ChainHalt (Reason : Type) where
  verdict : Option (Verdict Reason)
  before : Before
  deriving DecidableEq, Repr

/-- 对停机值的一次调用。 -/
inductive Call (Reason : Type) where
  | prove
  | trip (reason : Reason)
  deriving DecidableEq, Repr

variable {Reason : Type}

/-- `ChainHalt::settle`：还没有判定才写下这一个。 -/
def settle (h : ChainHalt Reason) (v : Verdict Reason) : ChainHalt Reason :=
  match h.verdict with
  | none => { h with verdict := some v }
  | some _ => h

def Call.verdict : Call Reason → Verdict Reason
  | .prove => .Whole
  | .trip r => .Broken r

/-- 依次调用之后的停机值。 -/
def calls (h : ChainHalt Reason) (cs : List (Call Reason)) : ChainHalt Reason :=
  cs.foldl (fun h c => settle h c.verdict) h

/-- 写者问停机值时的三种回答（`ChainHalt::admit`）。 -/
inductive Admission (Reason : Type) where
  | Admitted
  | ChainHalted (reason : Reason)
  | Unproven
  deriving DecidableEq, Repr

/-! D19 `Unproven`→`E_HISTORY_UNPROVEN`

`Unproven`→`E_HISTORY_UNPROVEN`（装载期）：服务中的城从快照起步，快照之前的历史由后台的证明走一遍（8-30），这之前写者不往一段没证明过的历史后面写任何一行。不可定义掉：证明要读完整条链，而开城不等它（sprawling-SPEC 8-122）。它住装载期白名单，因为它恰好只在本进程此刻写不了账本时出现。recovery 让人等证明那一行出现后再发一次；证明本身失败时停机值跳闸，之后答的是断链的原因，不再是这个码。**被否：命令先扣住、证明完成再执行。** 扣住的命令要一个新的唤醒理由才能不忙等（`crates/accounting/spec/Worker/Attend.lean`），而人以为已发出的命令可能在几秒之后才执行；拒绝让这件事当场可见。**重开参数**：F1 在 40 万行城上量出的证明墙钟（M3）到了毫秒级以外、人一再撞上这个码时，改成扣住，证明到达作为一次唤醒。
-/

def admit (h : ChainHalt Reason) : Admission Reason :=
  match h.verdict, h.before with
  | some .Whole, _ => .Admitted
  | none, .Admit => .Admitted
  | some (.Broken r), _ => .ChainHalted r
  | none, .Refuse => .Unproven

theorem settled_stays (h : ChainHalt Reason) (v : Verdict Reason) (cs : List (Call Reason))
    (settled : h.verdict = some v) : (calls h cs).verdict = some v := by
  induction cs generalizing h with
  | nil => exact settled
  | cons c rest ih =>
    apply ih
    simp [settle, settled]

/-- **第一个判定有效。** 从没有判定的停机值起，一串调用之后的判定就是第一次调用的判定。 -/
theorem the_first_verdict_stands (h : ChainHalt Reason) (c : Call Reason) (rest : List (Call Reason))
    (fresh : h.verdict = none) : (calls h (c :: rest)).verdict = some c.verdict := by
  have first : calls h (c :: rest) = calls (settle h c.verdict) rest := rfl
  rw [first]
  apply settled_stays
  simp [settle, fresh]

theorem admission_follows_the_verdict (h : ChainHalt Reason) :
    (h.verdict = none → admit h = (match h.before with | .Admit => .Admitted | .Refuse => .Unproven)) ∧
      (h.verdict = some .Whole → admit h = .Admitted) ∧
      (∀ r, h.verdict = some (.Broken r) → admit h = .ChainHalted r) := by
  obtain ⟨v, b⟩ := h
  refine ⟨fun e => ?_, fun e => ?_, fun r e => ?_⟩ <;> subst e <;> cases b <;> rfl

/-- 一次服务：先证明完好、后来又有一条跳闸，写者照旧放行。 -/
example : admit (calls (⟨none, .Refuse⟩ : ChainHalt String) [.prove, .trip "late"]) = .Admitted := by
  decide

/-- 证明一波读的段数（`chain_audit::PROOF_WAVE`）。 -/
def PROOF_WAVE : Nat := 8

/-- `ProofCount.waves`：段数除以 `PROOF_WAVE` 向上取整。 -/
def waves (segments : Nat) : Nat := (segments + PROOF_WAVE - 1) / PROOF_WAVE

/-- 一波取段序上接下来的至多 `PROOF_WAVE` 段，直到段取完；这样分出来的波数就是 `waves`。 -/
def inWaves (segments : List Nat) : List (List Nat) :=
  if _h : segments.length ≤ PROOF_WAVE then (if segments = [] then [] else [segments])
  else segments.take PROOF_WAVE :: inWaves (segments.drop PROOF_WAVE)
termination_by segments.length
decreasing_by simp only [List.length_drop, PROOF_WAVE] at _h ⊢; omega

theorem waves_depend_only_on_the_segments :
    ∀ (n : Nat) (segments : List Nat), segments.length = n → (inWaves segments).length = waves n := by
  intro n
  induction n using Nat.strongRecOn with
  | ind n ih =>
    intro segments hl
    unfold inWaves
    split
    · rename_i small
      split
      · rename_i empty
        subst empty
        simp at hl
        subst hl
        rfl
      · rename_i nonempty
        have pos : 0 < segments.length := List.length_pos_iff.mpr nonempty
        simp only [List.length_singleton, waves, PROOF_WAVE] at small ⊢
        omega
    · rename_i big
      simp only [List.length_cons]
      have shorter : (segments.drop PROOF_WAVE).length < n := by
        simp [PROOF_WAVE] at big ⊢; omega
      rw [ih _ shorter _ rfl]
      simp only [List.length_drop, waves, PROOF_WAVE] at big ⊢
      omega

end Storage.ChainAudit

/-! D20 已验证前缀的记录不带密钥

记录挡的是意外：段在两次开城之间被截短、被换成另一份、某一行被外部改写，摘要都不再相符，这一段退回逐行核对。能同时改段与记录的人绕得过它，与今天没有密钥的链本身一样：能改段的人也能重算整条链。**被否：用城密钥派生的 MAC 签记录。** 开城就得先打开 vault，密钥轮换让全部记录失效，而它挡住的那个人今天就能绕过链。**被否：不要记录，每次开城逐行核对全量一遍。** 保证相同，而 40 万行城每次开城都要付秒级的解析；记录让这笔钱只在升级之后第一次开城时付一次。**重开参数**：远程访问的城要对外证明自己的历史时（remote_access 的城密钥设计），给记录加 MAC。
-/

/-! D21 证明并行的只是各段前缀的摘要，链仍按段序判

一段前缀的摘要只取决于这一段的字节，在知道走到它时的链状态之前就能算；入口接不接得上、逐行核对，只能按段序做。所以一波的段先并行读、并行哈希，join 之后再按段序走（8-37），判定与单线程的走法相同（`crates/storage/spec/Snapshot.lean` 的 `wavesAreCached`）。**被否：每段一条线程、不分波。** 线程数与常驻内存都随账本长度增长，一座几百段的城会一次起几百条线程、把整条账本读进内存。**被否：一条线程上流水（哈希第 i 段时读第 i+1 段）。** 只能藏住读与哈希中较小的那一份，40 万行城上约三分之一。**重开参数**：一波的常驻内存成为问题，或段数常常少于核数、证明仍是接受命令（M3）的大头时，见 8-37 的两条被否。
-/
