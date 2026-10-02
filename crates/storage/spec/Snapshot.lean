-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 快照加上它之后的尾部，就是全量折叠

`storage::snapshot` 保存一次折叠在 Ledger 某一行之后的状态，连同那一行的 seq 与它的链哈希。开城时找到快照，只核对那一行，再只折叠它之后的行。这样开城靠得住，全凭一条性质：从某个切点上折叠已到达的状态出发，走完切点之后的行，到达的状态与在整本账本上折叠到达的状态相同。

**本模块对每一种折叠、每一个切点陈述这条性质。** 折叠是一个不透明的步进函数，作用在不透明的行上，所以产品折叠的任何视图都不会与它漂移：`Views`、`Standing` 以及链状态 `storage::LineCheck` 本身各是它的一个实例。Rust 一侧由 `proptest` 在随机账本与随机切点上守住同一性质（`storage::snapshot` 的测试），那里的折叠就是链检查：从快照恢复的一遍走，接受的行与终态都必须和从创世走的一遍相同。

**行哈希买到什么、买不到什么。** 快照只经它最后一行的哈希指明自己切自哪本账本。当这个哈希与盘上同一 seq 的那一行相符，并且摘要函数在被覆盖的行上是单射（这条假设归 `Kernel.Ledger`，见 `crates/kernel/spec/Ledger.lean`），快照所折的行就是盘上账本的前缀；`resumeIsWhole` 就陈述在这个前缀事实之上，所以它根本不需要摘要函数。

**两份快照，一遍读。** 服务中的城有两份快照（视图与 Standing），各切在自己的行上。开城从较早的切点读一遍，每一行只交给切点在它之前的那一份折叠；`twoCutsOnePass` 陈述这一遍的结果等于两份各自的全量折叠（`crates/sprawling/Spec.lean` §8-122，`accounting::views::snapshot::start::start_both`）。

**已验证前缀：不重算，也不少核对。** 快照之前的行由后台的证明走一遍（§8-30，`storage::verified_prefix`）。每段有一条记录：版本、入口状态、这段前缀字节的摘要、出口状态。记录的入口等于当前状态、版本相同、摘要相符时，证明不再逐行核对这段前缀，直接取记录的出口；否则逐行核对。`cachedVerifyIsStrict` 陈述：只要记录都由严格核对写下、摘要在段上单射，这样得到的判定与逐行核对整条链的判定相同。两个反例说明三个条件缺一不可：不比入口，删掉中间一段也被接受（`withoutLinkAcceptsSplice`）；不比版本，旧规则放过的行被沿用（`withoutVersionAcceptsStale`）。记录没有密钥：能同时改段与记录的人绕得过它，与今天无密钥的链一样（sprawling D4）。

**按波读段：先算摘要，再按段序走。** 证明把至多八段并成一波，各段的读与前缀哈希同时做，join 之后再按段序判入口、逐行核对（§8-37，`storage::chain_audit`）。能先算的只有摘要判定，因为它只看这一段的字节；`wavesAreCached` 陈述先算好的判定与边走边算的判定相同，`wavesAreStrict` 把它接到逐行核对上。
-/

/-!
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
- `Views` 与 `Standing` 都从这里起步（`crates/sprawling/Spec.lean` §8-91、§8-101）；服务中的城让两者从较早的切点一遍读起（`crates/sprawling/Spec.lean` §8-122）。
-/

namespace Storage.Snapshot

/-- 从 `init` 出发折叠完 `lines` 之后的状态。 -/
def fold {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) : σ :=
  lines.foldl step init

/-- 切在 `k` 的快照：折叠完前 `k` 行之后的状态。 -/
def cut {σ α : Type} (step : σ → α → σ) (init : σ) (lines : List α) (k : Nat) : σ :=
  fold step init (lines.take k)

/-- 从快照的状态出发折叠切点之后的行，结果就是全量折叠。对每一个切点都成立，包括 `0`（没有快照）和越过末尾的切点（整本账本的快照，尾部为空）。 -/
theorem snapshotPlusTailIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (lines : List α) (k : Nat) :
    fold step (cut step init lines k) (lines.drop k) = fold step init lines := by
  unfold cut fold
  rw [← List.foldl_append, List.take_append_drop]

/-- 在 `pre` 上取的快照，对以 `pre` 为前缀的任何账本都能正确恢复：快照写下之后账本变长了，尾部就是它长出来的部分。 -/
theorem resumeIsWhole {σ α : Type} (step : σ → α → σ) (init : σ)
    (pre lines : List α) (h : pre <+: lines) :
    fold step (fold step init pre) (lines.drop pre.length) = fold step init lines := by
  obtain ⟨tail, rfl⟩ := h
  unfold fold
  rw [List.drop_left, List.foldl_append]

/-! ## 两个切点，一遍读 -/

/-- 带位置的一步：位置在切点 `k` 之前的行不交给折叠，位置照样加一。 -/
def gatedStep {σ α : Type} (step : σ → α → σ) (k : Nat) (p : Nat × σ) (x : α) : Nat × σ :=
  (p.1 + 1, if k ≤ p.1 then step p.2 x else p.2)

/-- 从位置 `i` 起带门地折完 `xs`，等于跳过切点之前的那几行再折。 -/
theorem gatedFold {σ α : Type} (step : σ → α → σ) (k : Nat) (xs : List α) :
    ∀ (i : Nat) (s : σ),
      xs.foldl (gatedStep step k) (i, s) = (i + xs.length, fold step s (xs.drop (k - i))) := by
  induction xs with
  | nil => intro i s; simp [fold]
  | cons x rest ih =>
    intro i s
    rw [List.foldl_cons]
    by_cases hk : k ≤ i
    · have h1 : k - i = 0 := by omega
      have h2 : k - (i + 1) = 0 := by omega
      simp only [gatedStep, if_pos hk]
      rw [ih, h1, h2]
      simp [fold, List.length_cons]
      omega
    · have h1 : k - i = (k - (i + 1)) + 1 := by omega
      simp only [gatedStep, if_neg hk]
      rw [ih, h1]
      simp [List.length_cons]
      omega

/-- 两份折叠同走一遍：各带自己的门。 -/
def bothStep {σ τ α : Type} (stepA : σ → α → σ) (stepB : τ → α → τ) (ka kb : Nat)
    (p : Nat × σ × τ) (x : α) : Nat × σ × τ :=
  (p.1 + 1, if ka ≤ p.1 then stepA p.2.1 x else p.2.1,
    if kb ≤ p.1 then stepB p.2.2 x else p.2.2)

/-- 同走的一遍，两个分量各自就是带门的一遍。 -/
theorem bothFold {σ τ α : Type} (stepA : σ → α → σ) (stepB : τ → α → τ) (ka kb : Nat)
    (xs : List α) : ∀ (i : Nat) (a : σ) (b : τ),
      xs.foldl (bothStep stepA stepB ka kb) (i, a, b)
        = ((xs.foldl (gatedStep stepA ka) (i, a)).1,
           (xs.foldl (gatedStep stepA ka) (i, a)).2,
           (xs.foldl (gatedStep stepB kb) (i, b)).2) := by
  induction xs with
  | nil => intro i a b; rfl
  | cons x rest ih =>
    intro i a b
    simp only [List.foldl_cons, bothStep, gatedStep]
    rw [ih]

/-- 视图切在 `ka`、Standing 切在 `kb`，各自从快照的状态出发；从较早的切点起读一遍，每行只交给切点在它之前的那一份，结果就是两份各自的全量折叠。 -/
theorem twoCutsOnePass {σ τ α : Type} (stepA : σ → α → σ) (stepB : τ → α → τ)
    (a0 : σ) (b0 : τ) (lines : List α) (ka kb : Nat) :
    let m := min ka kb
    let pass := (lines.drop m).foldl (bothStep stepA stepB ka kb)
      (m, cut stepA a0 lines ka, cut stepB b0 lines kb)
    (pass.2.1, pass.2.2) = (fold stepA a0 lines, fold stepB b0 lines) := by
  intro m pass
  have ha : ka - m + m = ka := by omega
  have hb : kb - m + m = kb := by omega
  simp only [pass, bothFold, gatedFold, List.drop_drop]
  rw [Nat.add_comm (ka - m) m, Nat.add_comm (kb - m) m] at *
  have ea : m + (ka - m) = ka := by omega
  have eb : m + (kb - m) = kb := by omega
  rw [ea, eb, snapshotPlusTailIsWhole, snapshotPlusTailIsWhole]

/-! ## 已验证前缀 -/

/-- 一段的记录：写下它时的规则版本、它的入口状态、这段前缀的摘要、走完之后的出口状态。 -/
structure Record (σ D : Type) where
  version : Nat
  entry : σ
  digest : D
  exit : σ

/-- 逐行核对一段：`check` 答 `none` 即这一行不过，整段不过。 -/
def strictSeg {σ α : Type} (check : σ → α → Option σ) (s : σ) (seg : List α) : Option σ :=
  seg.foldlM check s

/-- 逐行核对整条链，一段接一段。 -/
def strictRun {σ α : Type} (check : σ → α → Option σ) (s : σ) : List (List α) → Option σ
  | [] => some s
  | seg :: rest => (strictSeg check s seg).bind (fun t => strictRun check t rest)

/-- 记录只由逐行核对写下：在当前版本 `v` 下，它的入口经某段字节走到它的出口，摘要就是那段字节的摘要。别的版本写下的记录不作任何假设：那是别的规则。 -/
def Sound {σ α D : Type} (v : Nat) (dig : List α → D) (check : σ → α → Option σ)
    (r : Record σ D) : Prop :=
  r.version = v → ∃ seg, dig seg = r.digest ∧ strictSeg check r.entry seg = some r.exit

/-- 用记录代替逐行核对的三个条件：版本相同、入口接得上、摘要相符。 -/
def reuses {σ α D : Type} [DecidableEq σ] [DecidableEq D] (v : Nat) (dig : List α → D)
    (s : σ) (seg : List α) (r : Record σ D) : Bool :=
  decide (r.version = v) && decide (r.entry = s) && decide (r.digest = dig seg)

/-- 有记录且三个条件都满足就取记录的出口，否则逐行核对这一段。 -/
def cachedSeg {σ α D : Type} [DecidableEq σ] [DecidableEq D] (check : σ → α → Option σ)
    (v : Nat) (dig : List α → D) (s : σ) (seg : List α) : Option (Record σ D) → Option σ
  | some r => if reuses v dig s seg r then some r.exit else strictSeg check s seg
  | none => strictSeg check s seg

/-- 带记录走整条链：每段各自判定，一段的记录失配只让这一段退回逐行核对。 -/
def cachedRun {σ α D : Type} [DecidableEq σ] [DecidableEq D] (check : σ → α → Option σ)
    (v : Nat) (dig : List α → D) (s : σ) : List (List α × Option (Record σ D)) → Option σ
  | [] => some s
  | (seg, r) :: rest => (cachedSeg check v dig s seg r).bind (fun t => cachedRun check v dig t rest)

/-- 记录都由逐行核对写下、摘要在段上单射时，带记录的判定就是逐行核对整条链的判定。 -/
theorem cachedVerifyIsStrict {σ α D : Type} [DecidableEq σ] [DecidableEq D]
    (check : σ → α → Option σ) (v : Nat) (dig : List α → D)
    (injective : ∀ a b, dig a = dig b → a = b)
    (segs : List (List α × Option (Record σ D)))
    (sound : ∀ p ∈ segs, ∀ r, p.2 = some r → Sound v dig check r) :
    ∀ s, cachedRun check v dig s segs = strictRun check s (segs.map Prod.fst) := by
  induction segs with
  | nil => intro s; rfl
  | cons p rest ih =>
    intro s
    obtain ⟨seg, rec⟩ := p
    have tail : ∀ q ∈ rest, ∀ r, q.2 = some r → Sound v dig check r :=
      fun q hq r hr => sound q (List.mem_cons_of_mem _ hq) r hr
    have same : cachedSeg check v dig s seg rec = strictSeg check s seg := by
      cases rec with
      | none => rfl
      | some r =>
        simp only [cachedSeg]
        split
        · rename_i hr
          simp only [reuses, Bool.and_eq_true, decide_eq_true_eq] at hr
          obtain ⟨⟨hv, he⟩, hd⟩ := hr
          obtain ⟨seg', hdig, hrun⟩ := sound (seg, some r) List.mem_cons_self r rfl hv
          have : seg' = seg := injective _ _ (hdig.trans hd)
          rw [← he, ← this, hrun]
        · rfl
    simp only [cachedRun, strictRun, List.map_cons, same]
    cases strictSeg check s seg with
    | none => rfl
    | some t => exact ih tail t

/-- 反例用的链：状态是下一行应有的 seq，一行就是它的 seq。 -/
def nextSeq (s x : Nat) : Option Nat := if x = s then some (s + 1) else none

/-- 不比入口的复用：删掉中间一段，后一段的记录照样被接受。 -/
def reusesNoLink (v : Nat) (seg : List Nat) (r : Record Nat (List Nat)) : Bool :=
  decide (r.version = v) && decide (r.digest = seg)

def cachedNoLink (v : Nat) (s : Nat) : List (List Nat × Option (Record Nat (List Nat))) → Option Nat
  | [] => some s
  | (seg, some r) :: rest =>
    (if reusesNoLink v seg r then some r.exit else strictSeg nextSeq s seg).bind
      (fun t => cachedNoLink v t rest)
  | (seg, none) :: rest => (strictSeg nextSeq s seg).bind (fun t => cachedNoLink v t rest)

/-- 三段账本删掉中间一段：不比入口就接受，逐行核对与带入口的记录都拒绝。 -/
theorem withoutLinkAcceptsSplice :
    let spliced := [([0], some ⟨1, 0, [0], 1⟩), ([2], some ⟨1, 2, [2], 3⟩)]
    cachedNoLink 1 0 spliced = some 3 ∧
      strictRun nextSeq 0 (spliced.map Prod.fst) = none ∧
      cachedRun nextSeq 1 id 0 spliced = none := by
  decide

/-- 旧规则（版本 0）放过了以 seq 7 开头的一段并写下记录；新规则 `nextSeq` 拒绝它。规则变了而版本键没动（`cachedNoLink 0`），旧记录照样被沿用；版本键随规则进到 1，这条记录就不再复用，逐行核对拒绝它。 -/
theorem withoutVersionAcceptsStale :
    let stale : List (List Nat × Option (Record Nat (List Nat))) := [([7], some ⟨0, 0, [7], 1⟩)]
    cachedNoLink 0 0 stale = some 1 ∧
      strictRun nextSeq 0 (stale.map Prod.fst) = none ∧
      cachedRun nextSeq 1 id 0 stale = none := by
  decide

/-! ## 按波读段 -/

/-- 一段的摘要判定：只看这一段的字节与它的记录，不看走到它时的链状态，所以能在走到这一段之前、与同一波的别的段同时算（§8-37）。 -/
def digestMatches {σ α D : Type} [DecidableEq D] (dig : List α → D) (seg : List α) :
    Option (Record σ D) → Bool
  | some r => decide (r.digest = dig seg)
  | none => false

/-- 摘要判定已经算好时走一段：版本与入口仍在这里按段序判，摘要取算好的那一位。 -/
def wavedSeg {σ α D : Type} [DecidableEq σ] (check : σ → α → Option σ) (v : Nat)
    (s : σ) (seg : List α) (matched : Bool) : Option (Record σ D) → Option σ
  | some r =>
    if decide (r.version = v) && decide (r.entry = s) && matched then some r.exit
    else strictSeg check s seg
  | none => strictSeg check s seg

/-- 带着每段算好的摘要判定，按段序走整条链。 -/
def wavedRun {σ α D : Type} [DecidableEq σ] (check : σ → α → Option σ) (v : Nat) (s : σ) :
    List (List α × Option (Record σ D) × Bool) → Option σ
  | [] => some s
  | (seg, r, m) :: rest => (wavedSeg check v s seg m r).bind (fun t => wavedRun check v t rest)

/-- 每段的摘要判定先算好（对各段逐一作用的纯函数，哪一段先算完都一样）、再按段序走，与边走边算的 `cachedRun` 是同一个判定。 -/
theorem wavesAreCached {σ α D : Type} [DecidableEq σ] [DecidableEq D]
    (check : σ → α → Option σ) (v : Nat) (dig : List α → D)
    (segs : List (List α × Option (Record σ D))) :
    ∀ s, wavedRun check v s (segs.map fun p => (p.1, p.2, digestMatches dig p.1 p.2)) =
      cachedRun check v dig s segs := by
  induction segs with
  | nil => intro s; rfl
  | cons p rest ih =>
    intro s
    obtain ⟨seg, rec⟩ := p
    have same : wavedSeg check v s seg (digestMatches dig seg rec) rec =
        cachedSeg check v dig s seg rec := by
      cases rec with
      | none => rfl
      | some r => rfl
    simp only [List.map_cons, wavedRun, cachedRun, same, ih]

/-- 记录都由逐行核对写下、摘要在段上单射时，按波读段的证明给出逐行核对整条链的判定。 -/
theorem wavesAreStrict {σ α D : Type} [DecidableEq σ] [DecidableEq D]
    (check : σ → α → Option σ) (v : Nat) (dig : List α → D)
    (injective : ∀ a b, dig a = dig b → a = b)
    (segs : List (List α × Option (Record σ D)))
    (sound : ∀ p ∈ segs, ∀ r, p.2 = some r → Sound v dig check r) (s : σ) :
    wavedRun check v s (segs.map fun p => (p.1, p.2, digestMatches dig p.1 p.2)) =
      strictRun check s (segs.map Prod.fst) :=
  (wavesAreCached check v dig segs s).trans (cachedVerifyIsStrict check v dig injective segs sound s)

/-- 不是空话：一段的记录站得住时，按波读段取记录的出口；入口接不上时退回逐行核对，与 `withoutLinkAcceptsSplice` 的拼接账本一样被拒。 -/
theorem wavesTakeARecordAndRefuseASplice :
    let whole : List (List Nat × Option (Record Nat (List Nat))) := [([0], some ⟨1, 0, [0], 1⟩)]
    let spliced := [([0], some ⟨1, 0, [0], 1⟩), ([2], some ⟨1, 2, [2], 3⟩)]
    wavedRun nextSeq 1 0 (whole.map fun p => (p.1, p.2, digestMatches id p.1 p.2)) = some 1 ∧
      wavedRun nextSeq 1 0 (spliced.map fun p => (p.1, p.2, digestMatches id p.1 p.2)) = none := by
  decide

end Storage.Snapshot
