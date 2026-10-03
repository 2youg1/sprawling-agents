-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::mem_ledger 与 citysim::checker

规定 `citysim::mem_ledger`（`tools/citysim/src/mem_ledger.rs`）与 `citysim::checker`（`tools/citysim/src/checker.rs`）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-1，别处引作 `tools/citysim/Spec.lean §8-1`。

能写成定理的是内存 Ledger 的写法：一串 draft 写下去，seq 从第一号起逐条连续，每一行的 `prev` 是上一行的摘要，第一行的 `prev` 是创世值。行怎样从 draft 拼出来（`EventRecord::from_draft` 与 `canonical_line`）、摘要怎样算（`chain_hash`）是 kernel 的，模型把它们当参数，所以这里说的是「接法」而不是「字节」；字节与 `storage::JsonlLedger` 逐字节相同由 conformance 与对拍测试检查（§16）。链与 seq 的验证规则本身由 runtime 的规格陈述（`crates/runtime/spec/Replay.lean`），检查器只调用它（D16）。
-/

/-!
### 8-1 mem_ledger 与 checker（形状 4 适配器＋形状 1 判定）

```rust
pub struct MemLedger { /* lines: Vec<Vec<u8>>, next_seq, prev */ }
impl MemLedger { pub fn new() -> Self;  pub fn raw_lines(&self) -> &[Vec<u8>]; }
impl Default for MemLedger { … }   // 等于 new()
impl kernel::Ledger for MemLedger { … }
impl kernel::conformance::LedgerInspect for MemLedger { … }   // citysim 恒开 conformance feature

/// Invariant: chain intact, seq contiguous.
pub fn check_chain(lines: Vec<Vec<u8>>) -> Result<(), AxError>;   // replay::verify_lines 薄封
```

`MemLedger` 的 append 是 from_draft→canonical_line→chain_hash 推进，没有别的逻辑；它不落盘，也不采时钟（t 由执行器的计数时钟给出）。`raw_lines` 是它固有的读面：执行器的报告与字节对拍从这里读，`LedgerInspect` 的实现留给 conformance 套件。模型的 seq 是 `Nat`；Rust 的 `Seq::next` 到顶时返回错误，那一档模型不写（一本内存账走不到 2⁶⁴ 行）。append 把每一步可失败的操作（拼行、推进 seq）放在第一次写之前，所以被拒的 draft 不动账本：行、seq、链摘要都留在原处（`an_append_past_the_last_seq_leaves_the_ledger_unchanged`）。

平台：Windows、macOS、Linux 上逐字节相同，`golden_fixture_pins_cross_os_bytes` 在三个平台上判同一份夹具。

D16 **检查器复用 `runtime::replay`，不自写链验证。** `check_chain` 是 `replay::verify_lines` 的薄封：验证语义只有一处，citysim 只加「检查器」这个角色名。被否：检查器自写链验证——citysim 独立性更强（不依赖 runtime），但即刻成为第二验证权威，与 replay 漂移时两边都对不上夹具。「重放与分叉共用重建器」的同一论证在此适用；citysim 依赖任何产品 crate 本就合法（第二 Main）。
-/

namespace Citysim.MemLedger

/-- 内存 Ledger 的状态（`mem_ledger::MemLedger`）：已写的行、下一个 seq、上一行的摘要。行的类型 `Line` 与摘要的类型 `Hash` 都是 kernel 的。 -/
structure MemLedger (Line Hash : Type) where
  lines : List Line
  next_seq : Nat
  prev : Hash

variable {Line Hash Draft : Type}

/-- 一本空账（`MemLedger::new`）：从 kernel 的第一号 seq 与创世 `prev` 起。 -/
def MemLedger.new (first : Nat) (genesis : Hash) : MemLedger Line Hash :=
  ⟨[], first, genesis⟩

/-- 写一条（`Ledger::append`）：`record` 是 `EventRecord::from_draft` 接 `canonical_line`，`chain_hash` 是 kernel 的链摘要。 -/
def MemLedger.append (record : Nat → Hash → Draft → Line) (chain_hash : Line → Hash)
    (ledger : MemLedger Line Hash) (draft : Draft) : MemLedger Line Hash :=
  let line := record ledger.next_seq ledger.prev draft
  ⟨ledger.lines ++ [line], ledger.next_seq + 1, chain_hash line⟩

/-- 依次写一串 draft。 -/
def MemLedger.appendAll (record : Nat → Hash → Draft → Line) (chain_hash : Line → Hash)
    (ledger : MemLedger Line Hash) (drafts : List Draft) : MemLedger Line Hash :=
  drafts.foldl (MemLedger.append record chain_hash) ledger

/-- 从 seq `seq`、上一行摘要 `prev` 起写下 `drafts` 得到的行：每一行接在上一行的摘要上。 -/
def written (record : Nat → Hash → Draft → Line) (chain_hash : Line → Hash) :
    Nat → Hash → List Draft → List Line
  | _, _, [] => []
  | seq, prev, draft :: rest =>
    let line := record seq prev draft
    line :: written record chain_hash (seq + 1) (chain_hash line) rest

theorem append_all_writes (record : Nat → Hash → Draft → Line) (chain_hash : Line → Hash)
    (drafts : List Draft) (ledger : MemLedger Line Hash) :
    (ledger.appendAll record chain_hash drafts).lines
        = ledger.lines ++ written record chain_hash ledger.next_seq ledger.prev drafts
      ∧ (ledger.appendAll record chain_hash drafts).next_seq = ledger.next_seq + drafts.length := by
  induction drafts generalizing ledger with
  | nil => simp [MemLedger.appendAll, written]
  | cons draft rest ih =>
    have step := ih (ledger.append record chain_hash draft)
    simp only [MemLedger.appendAll, List.foldl_cons] at step ⊢
    refine ⟨?_, ?_⟩
    · rw [step.1]
      simp [MemLedger.append, written]
    · rw [step.2]
      simp [MemLedger.append]
      omega

/-- 一本空账写下一串 draft 之后，账上恰是 `written` 从第一号与创世值起写出的那些行，下一个 seq 是第一号加行数：seq 连续，不跳不重。 -/
theorem a_fresh_ledger_holds_what_was_written (record : Nat → Hash → Draft → Line)
    (chain_hash : Line → Hash) (first : Nat) (genesis : Hash) (drafts : List Draft) :
    ((MemLedger.new first genesis).appendAll record chain_hash drafts).lines
        = written record chain_hash first genesis drafts
      ∧ ((MemLedger.new first genesis).appendAll record chain_hash drafts).next_seq
        = first + drafts.length := by
  have := append_all_writes record chain_hash drafts (MemLedger.new first genesis)
  simpa [MemLedger.new] using this

/-- 写下的第 `index` 行带着 seq `seq + index`，接在上一行的摘要上；第一行接在起点的 `prev` 上。链完整、seq 连续就是这一条。 -/
theorem every_line_follows_the_one_before (record : Nat → Hash → Draft → Line)
    (chain_hash : Line → Hash) (drafts : List Draft) :
    ∀ (seq : Nat) (prev : Hash) (index : Nat) (line : Line),
      (written record chain_hash seq prev drafts)[index + 1]? = some line →
      ∃ before draft,
        (written record chain_hash seq prev drafts)[index]? = some before
          ∧ line = record (seq + index + 1) (chain_hash before) draft := by
  induction drafts with
  | nil => intro seq prev index line at_; simp [written] at at_
  | cons first rest ih =>
    intro seq prev index line at_
    cases index with
    | zero =>
      cases rest with
      | nil => simp [written] at at_
      | cons second more =>
        simp only [written, List.getElem?_cons_succ, List.getElem?_cons_zero,
          Option.some.injEq] at at_
        exact ⟨record seq prev first, second, by simp [written], by rw [← at_]⟩
    | succ index =>
      simp only [written, List.getElem?_cons_succ] at at_
      obtain ⟨before, draft, found, made⟩ := ih (seq + 1) _ index line at_
      refine ⟨before, draft, by simpa [written] using found, ?_⟩
      rw [made]
      congr 1
      omega

end Citysim.MemLedger
