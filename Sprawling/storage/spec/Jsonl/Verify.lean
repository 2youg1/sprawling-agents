-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Ledger

/-!
# storage::jsonl::verify

规定 `jsonl::verify`（`crates/storage/src/` 下同名的文件）。账本的每个读者走一行的同一份检查 `LineCheck`，以及 `open` 对一行故障的处置。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-1 storage::jsonl：逐行检查

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
/// 不经链判一行：与 `advance` 对这一行自身字节的回答相同，给经索引找到这一行的读者（索引的折叠已走过链）。
pub fn read_line(raw: &[u8]) -> Result<CheckedLine, LineFault>;
/// 一行信封里声称的 seq，只说读者去哪里找，不说这一行好不好。
pub(crate) fn claimed_seq(raw: &[u8]) -> Option<Seq>;
impl LineFault { pub fn into_ax(self, line_no: u64) -> AxError; }  // 整本读者的拒词：更新的写者＝E_LOG_VERSION_UNSUPPORTED，其余＝E_CAS_CORRUPT
```

- 判定顺序：信封（`v` 经 `readable_log_v`）→ `prev` → `seq` → kind 二分（借用判定，不 clone）→ 已知 kind 的类型解析与规范回写比对；`ig:true` 的未知 kind 不解析但照样入链。
- `open` 对故障的处置：`VersionAhead`／`NotAVersion` 按版本拒；`NotALine` 且其后无带信封的行＝撕裂，截断（`carries_envelope` 把一串零当作预分配的空间而不是行的一部分，只判最后一个零之后的字节，所以零之后的记录算作「其后带信封的行」，见 `crates/storage/spec/Jsonl/Preallocate.lean`）；其余一律拒而不截——撕裂不会留下带信封的行，截掉它等于删掉合法历史。
- 被否：两个读者各持一份检查。只做类型解析的 `open` 会把一条链续正确的 `ig:true` 行在尾段当撕裂截掉，而 `replay` 收下同一行。
- 被否：前段末行只做类型解析。`EventRecord` 的 kind 没有「未知」这一臂，一条 `ig:true` 的新 kind 行若恰是前段的最后一行，类型解析拒它，城就打不开，而尾段扫描与 `replay` 都收下同一行（`jsonl/boundary/tests.rs` 的 `a_prior_segment_ending_in_an_ignorable_line_opens`）。代价：前段末行若不是写者规范的字节，`open` 现在也拒；那样的行 `replay` 本来就拒。
-/

/-!
## 模型：一行的判定、一串行的链，与 `open` 对一行故障的处置

一行在判定里露出的只有信封的几个字段与它自己的字节：`v` 经 `readable_log_v` 的回答、`seq`、`prev`、kind 落在哪一类，以及这一行的文字（它的 `chain_hash` 由 `hash` 给出，`hash` 就是 `kernel::ledger::chain_hash`，与 `crates/kernel/spec/Ledger.lean` 一样留作参数，这里不计算任何摘要）。JSON 能不能解析、规范回显是不是逐字节相同，模型各缩成一位：`Raw.notALine` 与 `Kind.known false`。

四条性质：

* `advance` 过了的一行恰好接上链：它的 `prev` 是当前状态的 `prev`、它的 `seq` 是期望的 seq，之后的状态是它的摘要与下一个 seq（`an_accepted_line_extends_the_chain`）；版本超前在一切链检之前答出，与 `prev`、`seq` 无关（`a_line_from_a_newer_writer_is_refused_before_any_chain_check`）；`ig:true` 的未知 kind 不解析，但照样入链（`an_ignorable_line_joins_the_chain`）。
* 从创世走完一串行而不拒，这串行就是 `Kernel.Ledger.Chained`：每条记录携带的 `prev` 正是它的行欠下的声索，seq 从 `Seq::FIRST` 起逐一加一（`a_walked_ledger_is_chained`）。所以读者经同一个 `LineCheck` 收下的，就是 kernel 的链规则要求的那一串。
* `open` 截断的，只有不带信封的字节：截掉的每一项都是 `NotALine`（`truncation_drops_no_enveloped_line`），留下的前缀正是 `advance` 一行一行收下的（`truncation_keeps_a_walked_prefix`）。所以两个写者各续同一个 `prev` 时，`open` 拒开，而不静默删掉另一个写者完整的历史。

Rust 的 `Seq` 是 `u64`，`next` 到顶时答 `SeqExhausted`；模型的 seq 是自然数，不建这一臂。
-/

namespace Storage.Jsonl.Verify

/-- `kernel::consts_external::readable_log_v` 对一行的 `v` 的四种回答（Rust 的 `LogVersion`）。 -/
inductive LogVersion where
  | Current
  | Older
  | Ahead
  | NotAVersion
  deriving DecidableEq, Repr

/-- 一行的 kind 落在哪一类：已知 kind 带着「类型解析后的规范回显与原字节逐字节相同」这一位；未知 kind 带着它的 `ig`。 -/
inductive Kind where
  | known (canonical : Bool)
  | unknown (ig : Bool)
  deriving DecidableEq, Repr

/-- 读者从一行读出的信封，连同这一行的文字。 -/
structure Envelope where
  text : String
  v : LogVersion
  seq : Nat
  prev : String
  kind : Kind
  deriving DecidableEq, Repr

/-- 一行原始字节：连信封都读不出，或者一个信封。 -/
inductive Raw where
  | notALine
  | line (e : Envelope)
  deriving DecidableEq, Repr

/-- Rust 的 `LineFault`，去掉各臂携带的说明文字与 `SeqExhausted`。 -/
inductive LineFault where
  | NotALine
  | VersionAhead
  | NotAVersion
  | ChainBreak
  | SeqGap
  | UnknownKind
  | NotCanonical
  deriving DecidableEq, Repr

/-- Rust 的 `CheckedLine`；已知 kind 的记录在模型里只留它的 seq。 -/
inductive CheckedLine where
  | Known (seq : Nat)
  | IgnoredUnknown (seq : Nat)
  deriving DecidableEq, Repr

/-- 链状态：上一行的摘要与下一个期望的 seq，只有链状态，不持行。 -/
structure LineCheck where
  prev : String
  expected : Nat
  deriving DecidableEq, Repr

/-- `LineCheck::at_genesis`：`GENESIS_PREV` 与 `Seq::FIRST`（0）。 -/
def LineCheck.at_genesis (genesis : String) : LineCheck := ⟨genesis, 0⟩

/-- 判一行，次序与 `LineCheck::advance` 相同：信封与版本、`prev`、`seq`、kind 二分与规范回显。过了，链前进到这一行的摘要与下一个 seq；没过，状态不动（函数式模型里状态不是可变的，Rust 的「错则状态不动」就是这里不返回新状态）。 -/
def advance (hash : String → String) (s : LineCheck) : Raw → Except LineFault (CheckedLine × LineCheck)
  | .notALine => .error .NotALine
  | .line e =>
    match e.v with
    | .Ahead => .error .VersionAhead
    | .NotAVersion => .error .NotAVersion
    | .Current | .Older =>
      if e.prev = s.prev then
        if e.seq = s.expected then
          match e.kind with
          | .known true => .ok (.Known e.seq, ⟨hash e.text, s.expected + 1⟩)
          | .known false => .error .NotCanonical
          | .unknown true => .ok (.IgnoredUnknown e.seq, ⟨hash e.text, s.expected + 1⟩)
          | .unknown false => .error .UnknownKind
        else .error .SeqGap
      else .error .ChainBreak

/-- 一串行逐行走完：第一行不过就停，答它的故障。 -/
def walk (hash : String → String) : LineCheck → List Raw → Except LineFault LineCheck
  | s, [] => .ok s
  | s, r :: rest =>
    match advance hash s r with
    | .error f => .error f
    | .ok (_, s') => walk hash s' rest

theorem an_accepted_line_extends_the_chain {hash : String → String} {s s' : LineCheck}
    {r : Raw} {c : CheckedLine} (h : advance hash s r = .ok (c, s')) :
    ∃ e, r = .line e ∧ e.prev = s.prev ∧ e.seq = s.expected ∧
      s' = ⟨hash e.text, s.expected + 1⟩ := by
  cases r with
  | notALine => simp [advance] at h
  | line e =>
    refine ⟨e, rfl, ?_⟩
    simp only [advance] at h
    split at h <;> try simp at h
    all_goals
      split at h
      · split at h
        · split at h <;> simp_all
        · simp at h
      · simp at h

theorem a_line_from_a_newer_writer_is_refused_before_any_chain_check (hash : String → String)
    (s : LineCheck) (e : Envelope) (ahead : e.v = .Ahead) :
    advance hash s (.line e) = .error .VersionAhead := by
  simp [advance, ahead]

theorem an_ignorable_line_joins_the_chain (hash : String → String) (s : LineCheck) (e : Envelope)
    (readable : e.v = .Current ∨ e.v = .Older) (linked : e.prev = s.prev)
    (next : e.seq = s.expected) (ignorable : e.kind = .unknown true) :
    advance hash s (.line e) = .ok (.IgnoredUnknown e.seq, ⟨hash e.text, s.expected + 1⟩) := by
  rcases readable with h | h <;> simp [advance, h, linked, next, ignorable]

/-- 一串信封从 `p` 起欠下的 `prev`：第一行欠 `p`，此后每一行欠上一行的摘要。 -/
def owed (hash : String → String) (p : String) : List Envelope → List String
  | [] => []
  | e :: rest => p :: owed hash (hash e.text) rest

/-- `owed` 就是 kernel 的 `required` 从任意起点算起的样子。 -/
theorem owed_is_required (hash : String → String) (p : String) (es : List Envelope) :
    owed hash p es = Kernel.Ledger.required hash p (es.map (·.text)) := by
  induction es generalizing p with
  | nil => rfl
  | cons e rest ih =>
    rw [owed, ih]
    cases rest with
    | nil => rfl
    | cons e2 r2 => simp [Kernel.Ledger.required]

/-- 从任意链状态走完一串信封而不拒：每条记录携带的 `prev` 是这串行欠下的，seq 从期望的那一个起逐一加一。 -/
theorem walked_lines_carry_what_they_owe (hash : String → String) :
    ∀ (es : List Envelope) (s s' : LineCheck), walk hash s (es.map .line) = .ok s' →
      es.map (·.prev) = owed hash s.prev es ∧ es.map (·.seq) = List.range' s.expected es.length := by
  intro es
  induction es with
  | nil => intro s s' _; simp [owed]
  | cons e rest ih =>
    intro s s' h
    simp only [List.map_cons, walk] at h
    split at h
    · simp at h
    · rename_i c t ht
      obtain ⟨e', he, hp, hs, rfl⟩ := an_accepted_line_extends_the_chain ht
      cases he
      obtain ⟨ihp, ihs⟩ := ih _ _ h
      refine ⟨?_, ?_⟩
      · simp [owed, hp, ihp]
      · simp [List.range'_succ, hs, ihs]

/-- **读者收下的就是 kernel 的链。** 从创世走完一本账本的每一行而不拒，它的行与记录携带的 `prev` 就满足 `Kernel.Ledger.Chained`，seq 恰是 `0, 1, …`。 -/
theorem a_walked_ledger_is_chained (hash : String → String) (genesis : String)
    (es : List Envelope) (s' : LineCheck)
    (h : walk hash (LineCheck.at_genesis genesis) (es.map .line) = .ok s') :
    Kernel.Ledger.Chained hash genesis (es.map (·.text)) (es.map (·.prev)) ∧
      es.map (·.seq) = List.range es.length := by
  obtain ⟨hp, hs⟩ := walked_lines_carry_what_they_owe hash es _ _ h
  refine ⟨?_, ?_⟩
  · unfold Kernel.Ledger.Chained
    rw [hp, owed_is_required]
    rfl
  · rw [hs, List.range_eq_range']
    rfl

/-- 不是空话：创世行与它之后一条 `ig:true` 的新 kind 行，从创世走得通。 -/
example :
    walk id (LineCheck.at_genesis "g")
      [.line ⟨"a", .Current, 0, "g", .known true⟩, .line ⟨"b", .Older, 1, "a", .unknown true⟩]
      = .ok ⟨"b", 2⟩ := by
  rfl

/-! ### `open` 对尾段一行故障的处置 -/

/-- 尾段扫描遇到一行故障时的三种处置。 -/
inductive Disposition where
  | Truncate
  | RefuseVersion
  | RefuseEnvelope
  deriving DecidableEq, Repr

/-- 一行带不带信封：Rust 的 `LineCheck::carries_envelope`。 -/
def Raw.enveloped : Raw → Bool
  | .notALine => false
  | .line _ => true

/-- `VersionAhead`／`NotAVersion` 按版本拒；`NotALine` 且其后没有带信封的行是撕裂，截断；其余一律拒而不截。 -/
def dispose (f : LineFault) (after : List Raw) : Disposition :=
  match f with
  | .VersionAhead | .NotAVersion => .RefuseVersion
  | .NotALine => if after.any Raw.enveloped then .RefuseEnvelope else .Truncate
  | .ChainBreak | .SeqGap | .UnknownKind | .NotCanonical => .RefuseEnvelope

/-- 尾段扫描的结局：整段都过；截在第 `kept` 行之后；或拒开。 -/
inductive Scan where
  | whole (s : LineCheck)
  | truncated (kept : Nat) (s : LineCheck)
  | refused (d : Disposition)
  deriving DecidableEq, Repr

/-- 截断点之前的每一行都已收下，所以截在第几行要随走过的行数后移。 -/
def Scan.after : Scan → Scan
  | .whole s => .whole s
  | .truncated k s => .truncated (k + 1) s
  | .refused d => .refused d

/-- `open` 第 ④ 步的尾段扫描：逐行 `advance`，第一行故障按 `dispose` 处置。 -/
def scan (hash : String → String) : LineCheck → List Raw → Scan
  | s, [] => .whole s
  | s, r :: rest =>
    match advance hash s r with
    | .ok (_, s') => (scan hash s' rest).after
    | .error f =>
      match dispose f rest with
      | .Truncate => .truncated 0 s
      | .RefuseVersion => .refused .RefuseVersion
      | .RefuseEnvelope => .refused .RefuseEnvelope

theorem a_fault_without_an_envelope_is_not_a_line {hash : String → String} {s : LineCheck}
    {r : Raw} (h : advance hash s r = .error .NotALine) : r = .notALine := by
  cases r with
  | notALine => rfl
  | line e =>
    simp only [advance] at h
    split at h <;> try simp at h
    all_goals
      split at h
      · split at h
        · split at h <;> simp at h
        · simp at h
      · simp at h

theorem truncate_only_a_tear {f : LineFault} {after : List Raw} (h : dispose f after = .Truncate) :
    f = .NotALine ∧ after.any Raw.enveloped = false := by
  cases f with
  | NotALine =>
    refine ⟨rfl, ?_⟩
    cases ha : after.any Raw.enveloped with
    | false => rfl
    | true => simp [dispose, ha] at h
  | VersionAhead => simp [dispose] at h
  | NotAVersion => simp [dispose] at h
  | ChainBreak => simp [dispose] at h
  | SeqGap => simp [dispose] at h
  | UnknownKind => simp [dispose] at h
  | NotCanonical => simp [dispose] at h

theorem truncation_drops_no_enveloped_line (hash : String → String) :
    ∀ (seg : List Raw) (s s' : LineCheck) (k : Nat), scan hash s seg = .truncated k s' →
      ∀ r ∈ seg.drop k, r.enveloped = false := by
  intro seg
  induction seg with
  | nil => intro s s' k h; simp [scan] at h
  | cons r rest ih =>
    intro s s' k h
    simp only [scan] at h
    split at h
    · rename_i c t _
      cases hs : scan hash t rest with
      | whole _ => simp [hs, Scan.after] at h
      | refused _ => simp [hs, Scan.after] at h
      | truncated k' u =>
        simp only [hs, Scan.after, Scan.truncated.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        simpa using ih t u k' hs
    · rename_i f hf
      cases hd : dispose f rest with
      | RefuseVersion => simp [hd] at h
      | RefuseEnvelope => simp [hd] at h
      | Truncate =>
        simp only [hd, Scan.truncated.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        obtain ⟨rfl, none_after⟩ := truncate_only_a_tear hd
        have torn : r = .notALine := a_fault_without_an_envelope_is_not_a_line hf
        simp only [List.any_eq_false] at none_after
        intro x hx
        simp only [List.drop_zero, List.mem_cons] at hx
        rcases hx with rfl | hx
        · rw [torn]; rfl
        · simpa using none_after x hx

theorem truncation_keeps_a_walked_prefix (hash : String → String) :
    ∀ (seg : List Raw) (s s' : LineCheck) (k : Nat), scan hash s seg = .truncated k s' →
      walk hash s (seg.take k) = .ok s' := by
  intro seg
  induction seg with
  | nil => intro s s' k h; simp [scan] at h
  | cons r rest ih =>
    intro s s' k h
    simp only [scan] at h
    split at h
    · rename_i c t ht
      cases hs : scan hash t rest with
      | whole _ => simp [hs, Scan.after] at h
      | refused _ => simp [hs, Scan.after] at h
      | truncated k' u =>
        simp only [hs, Scan.after, Scan.truncated.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        simp only [List.take_succ_cons, walk, ht]
        exact ih t u k' hs
    · rename_i f _
      cases hd : dispose f rest with
      | RefuseVersion => simp [hd] at h
      | RefuseEnvelope => simp [hd] at h
      | Truncate =>
        simp only [hd, Scan.truncated.injEq] at h
        obtain ⟨rfl, rfl⟩ := h
        rfl

/-- 两个写者各续了同一个 `prev`：第二个写者的那一行带信封、链断，`open` 拒开而不截。 -/
example :
    scan id (LineCheck.at_genesis "g")
      [.line ⟨"a", .Current, 0, "g", .known true⟩, .line ⟨"b", .Current, 1, "a", .known true⟩,
       .line ⟨"c", .Current, 1, "a", .known true⟩] = .refused .RefuseEnvelope := by
  decide

/-- 撕裂的尾巴：最后一项读不出信封，`open` 截掉它，留下前两行。 -/
example :
    scan id (LineCheck.at_genesis "g")
      [.line ⟨"a", .Current, 0, "g", .known true⟩, .line ⟨"b", .Current, 1, "a", .known true⟩,
       .notALine] = .truncated 2 ⟨"b", 2⟩ := by
  decide

end Storage.Jsonl.Verify
