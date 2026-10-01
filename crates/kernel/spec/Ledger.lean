-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 账本的链证明了什么，没有证明什么

规定 `crates/kernel/src/ledger.rs` 的 `chain_hash` 与 `GENESIS_PREV` 立下的链规则。

Ledger 的每一行都带一个 `prev`：它上一行的摘要。决定这个值的只有一个 Rust 函数，`kernel::ledger::chain_hash`，即对该行规范字节做的 `B3Hash::digest`；消费它的只有一个 Rust 步骤，`storage::jsonl::open` 检查每一行的 `prev` 等于上一行的摘要。其余一切信任 Ledger 的代码，信任的都是这两处。

**本模块从不计算摘要。** 摘要函数始终是一个不透明的参数，所以这里没有哪个值会与 `chain_hash` 漂移：明天把 blake3 换成别的函数，本文件的每一条陈述一字不变。本模块持有的是那两行代码作出却从不说出的假设——摘要函数是单射——以及链能覆盖到哪里：它比整个文件短一行。读者对这两件事往往按自己方便的方向去假设，所以这里证明它们，而不只是描述：

* 一本账本欠下的声索，由它被覆盖的行决定。这个方向是**免费**的：它就是同余，对摘要函数一无所求。检查拿一个记下的摘要去比另一个记下的摘要时，用的就是这个方向。
* 两本账本是否写下了相同的被覆盖的行，由它们携带的声索决定。这个方向要**花钱买**：它需要摘要是单射，这是关于 blake3 的陈述，而不是关于本产品的陈述。这条假设写在定理自己的语句里，不写在注释里；`aCoveredLineHidesWithoutInjectivity` 给出一个让同一结论为假的摘要函数，所以这条假设不是摆设。
* **最后一行**在这一切之外。它之后没有谁对它取过摘要，所以一本成链账本的头就是磁盘说的样子；`theLastLineIsNotCertified` 给出一串 prev，它与两本只在最后一行不同的账本都相符。没有哪条单射假设能补上这个缺口，这条定理也不作任何假设。由此得出两个推论，都关乎代码而不是算术：一个要测检测能力的检查必须瞄准一条被覆盖的行，因为头上的损坏走的是产品的恢复路径而不是拒绝路径；任何读者都不得把 Ledger 自己的链当作关于它的头的证据。

**为什么用 Lean 而不是再写一个 Rust 测试。** Rust 测试断言的是它能构造出来的账本，`proptest` 抽样的是一个策略能抽到的账本。这里的陈述针对每一串行、每一个摘要函数，其中两条是否定的——它们说某个声索*得不到*——而这是任何样本都显示不了的。`cargo xtask proof` 拿真实 MIR 检验几条 kernel 命题；这里关乎的是一条规则能管到多远。

**为什么这不会成为第二个权威。** 它不重述产品的任何规则：没有代码查询下面的谓词，里面不出现任何摘要值，它的载体都是树里已有的，按磁盘上的样子读——文件的行作为文本，以及每条记录携带的 `prev`，按读者解析出来的次序。需要判定时，检验器 `tools/adversary/src/Sprawling/Door.lean` 从门外问产品（`Door.verify` 跑产品自己的离线校验），而不问本模块；检验器也不 import 本模块。每条陈述都写明它对应的 Rust 代码行。
-/

/-!
### 8-9 kernel::ledger（缝清单文件，全库五真缝之一）

```rust
/// The only write entrance to history (ARCHITECTURE §1-2). Implementations
/// own seq/prev assignment and byte production; callers never serialize.
/// Contract: Ok(ref) ⇒ the record is durable in that adapter's medium and
/// `ref` points at it; Err ⇒ nothing observable was appended (torn bytes
/// are the reopen path's business, not the caller's).
pub trait Ledger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
}

pub const GENESIS_PREV: B3Hash;                       // 32 个零字节（hex64 全 0）
/// Chain rule: prev of line k+1 = blake3(raw bytes of
/// line k, excluding the line terminator). One hash function, one home.
pub fn chain_hash(raw_line: &[u8]) -> B3Hash;

#[cfg(feature = "conformance")]
pub mod conformance {
    /// Read-back surface for verification only; production callers never read
    /// through the Ledger handle (projections do). Lives behind the feature
    /// so the production port stays write-only.
    pub trait LedgerInspect { fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError>; }
    /// One assertion suite for every implementation (V3). `fresh` must yield
    /// an empty ledger each call.
    pub fn assert_ledger_conformance<L: Ledger + LedgerInspect>(fresh: impl FnMut() -> L);
}
```

conformance 六断言（对任意实现同一套）：
1. 首条 append 得 `Seq::FIRST`，记录 prev＝`GENESIS_PREV`；
2. seq 连续无洞（逐条 +1）；
3. 链续：第 k 行 prev＝`chain_hash(第 k-1 行原始字节)`；
4. 写方规范：每行 `parse_line` 后 `canonical_line` 与原始字节逐字节相等；
5. `v` 恒＝`EVENT_LOG_V`，`ref.kind`＝draft.kind；
6. 确定性：同一 draft 序列灌两个 fresh 实例，raw_lines 逐字节相同。
-/

/-!
### 8-51 `Ledger` 端口的第二个方法：一波一屏障

```rust
pub trait Ledger {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError>;  // 默认逐条 append
}
```

- **为什么端口要长这一只手**：一次持久写的代价是一道磁盘屏障，而屏障的价钱与骑在它上面的记录条数无关，真正的写只占其中一小部分。手上已经攥着一波的调用方按条交付，就为每一条付一道屏障。`storage::JsonlLedger` 覆写它为一波一屏障，`accounting::worker::relay` 按波调它。
- **默认实现是诚实的**：逐条 `append`，任何没有批量能力的存储照此就是正确的，不必为了满足端口去假装合并。
- **契约逐元素成立**：答 `Ok` 即整波已落盘，refs 按给入顺序回来。第一条拒绝结束整波，其前的记录可能已经落盘——这与单条 `append` 在它后面那条失败时给出的承诺完全一样。
- **否决「显式屏障动作」**：让 `append` 只写不同步、另给一个 flush 动作，会让一条已经发出的 `EventRef` 指向一条可能还不存在的历史，而那正是这个类型存在的全部意义。
-/

namespace Kernel.Ledger

/-- 一本账本欠下的 `prev`，从最早的起：第一行欠创世摘要，此后每一行欠它上一行的摘要。

这就是 `storage::jsonl::open` 每次开账本都要校验的规则，写成整本账本欠下的序列，而不是对一个文件的循环。`lines.dropLast` 是除头之外的每一行，也就是有后继把它哈希进去的那些行；所以没有行的账本不欠声索，只有一行的账本恰好欠创世摘要。

追加一行，只在这个序列末尾加一项，已有的项一个都不动；下面的 `appendingALineMovesNoClaim` 把这件事写成了定理。

`hash` 就是 `kernel::ledger::chain_hash`，只是它所依赖的摘要算法留作开放：进一个 `String`，出一个 `String`，这里不计算任何值。 -/
def required (hash : String → String) (genesis : String) (lines : List String) : List String :=
  match lines with
  | [] => []
  | _ :: _ => genesis :: lines.dropLast.map hash

/-- 一本账本成链，是说它的记录携带的声索，正是它的行欠下的声索。

两个参数是同一个文件的两半：按次序排列的行文本，以及每条记录携带的 `prev`，按读者解析出来的次序。两者并不独立——下面的等式迫使两个列表一样长——一串与行对不上的 prev 不是成链的账本，而是一次与文件不一致的解析。

任何一串 prev 都可以，所以下面的陈述对读者把记录解析成什么都成立；检验器的 `Record` 是这样的读者之一，本模块不 import 它。

摘要函数与创世摘要是参数，因为两者都不是这里的事实：`chain_hash` 住在 `crates/kernel/src/ledger.rs`，它的值与 `GENESIS_PREV` 从产品读出。在这里抄一份，就是 `tools/adversary/Spec.lean` 第 5 节禁止的第二个家。 -/
def Chained (hash : String → String) (genesis : String) (lines : List String)
    (prevs : List String) : Prop :=
  prevs = required hash genesis lines

/-- **被覆盖的行决定声索，这个方向免费。**

同一本账本的两次读取——行数相同，除头之外每一行都一致——欠下相同的声索。`chain_hash` 不参与：*任何*函数都有这个性质，所以检查可以比较两个记下的摘要，而不必知道它们各自怎样算出。

依赖它的代码拿先前记下的摘要与现在记下的摘要相比：`runtime::prefix` 在 run 开始时冻结一次、每个回合重申的四段哈希。行数写进假设，是因为一次读取包含行数——没有行的文件与只有一行的文件 `dropLast` 相同，只有行数能把它们分开。 -/
theorem theCoveredLinesDecideTheClaims (hash : String → String) (genesis : String)
    {lines lines' : List String} {prevs prevs' : List String}
    (chained : Chained hash genesis lines prevs) (chained' : Chained hash genesis lines' prevs')
    (same : lines.dropLast = lines'.dropLast) (counted : lines.length = lines'.length) :
    prevs = prevs' := by
  have owed : required hash genesis lines = required hash genesis lines' := by
    cases lines with
    | nil =>
      cases lines' with
      | nil => rfl
      | cons head rest =>
        exact absurd counted (by simp)
    | cons head rest =>
      cases lines' with
      | nil =>
        exact absurd counted (by simp)
      | cons other tail => simp [required, same]
  unfold Chained at chained chained'
  rw [chained, chained', owed]

/-- **声索决定被覆盖的行，这个方向要花钱买。**

要从两本账本携带的声索相等推出它们写下了相同的被覆盖的行，全部代价就是 `injective`。在产品里，这是 `runtime::replay::verify_lines` 每次离线检查都走的一步——它拿一行的 `prev` 与上一行的摘要相比，相符就读作「过去就是它说的样子」——也是城恢复时 `storage::jsonl::open::recover_tail` 走的一步，以及 `storage::bundle::files::head_of` 坚持只有一条链时走的一步。下面的 `aCoveredLineHidesWithoutInjectivity` 是反例，表明这条假设去不掉。

头不在结论之内，这是规则本身，不是证明留下的痕迹：`lines.dropLast` 恰好是有后继对它取过摘要的那些行。 -/
theorem theClaimsDecideTheCoveredLines (hash : String → String) (genesis : String)
    (injective : Function.Injective hash) {lines lines' : List String}
    (same : required hash genesis lines = required hash genesis lines') :
    lines.dropLast = lines'.dropLast := by
  cases lines with
  | nil =>
    cases lines' with
    | nil => rfl
    | cons other tail => exact absurd same (by simp [required])
  | cons head rest =>
    cases lines' with
    | nil => exact absurd same (by simp [required])
    | cons other tail =>
      have covered : (head :: rest).dropLast.map hash = (other :: tail).dropLast.map hash := by
        simpa only [required, List.map_cons, List.cons.injEq, true_and] using same
      exact (List.map_inj_right injective).mp covered

/-- 被覆盖的行一动，声索就跟着动。

磁盘的敌意动作瞄准的就是这条陈述：改掉除最后一行外任何一行的一个字节，账本携带的声索就不再是它自己的行欠下的声索，于是比较两者的读者拒绝它。`injective` 在这里的理由与上一条相同。

`Ground.corrupt` 翻转*最老*那一行的一个字节，还有一个本定理不涉及的理由：只有那个位置没有城进程可能在同时写。本定理说的是，最老那一行不是唯一值得攻击的位置——每一条有后继的行都被覆盖——所以只在一个位置测检测能力的检查，测到的只是那一个位置。 -/
theorem aCoveredLineCannotChangeUnnoticed (hash : String → String) (genesis : String)
    (injective : Function.Injective hash) {lines lines' : List String} {prevs : List String}
    (chained : Chained hash genesis lines prevs) (moved : lines.dropLast ≠ lines'.dropLast) :
    prevs ≠ required hash genesis lines' := by
  intro agrees
  unfold Chained at chained
  have same : required hash genesis lines = required hash genesis lines' := by
    rw [← chained, agrees]
  exact moved (theClaimsDecideTheCoveredLines hash genesis injective same)

/-- **最后一行在链之外，任何摘要函数都救不回来。**

一串 prev，两本账本，都成链，只在最后一行不同：对两个不同的字节串，再强的摘要函数也可以随意给值，因为它们之后没有谁对它们取过摘要。这里的 `hash` 是恒等函数，所以这件事在最强的摘要函数下也成立，而不只是在某个弱函数下。

敌意动作之所以这样分，原因就在这里。`Ground.tear` 去掉一段尾巴，产品*恢复*，这个承诺它守得住，因为没有声索被打破；`Ground.corrupt` 必须瞄准一条有后继覆盖的行，否则一个想测检测能力的检查测的是恢复路径。这也是为什么任何读者都不得把链当作关于头的证据：头说什么，就是磁盘说什么。 -/
theorem theLastLineIsNotCertified (genesis a b c : String) (different : b ≠ c) :
    ∃ lines lines' prevs,
      lines ≠ lines' ∧ Chained id genesis lines prevs ∧ Chained id genesis lines' prevs := by
  refine ⟨[a, b], [a, c], [genesis, a], ?_, ?_, ?_⟩
  · intro same
    injection same with _ rest
    injection rest with bytes _
    exact different bytes
  · simp [Chained, required, List.dropLast_eq_take]
  · simp [Chained, required, List.dropLast_eq_take]

/-- **没有单射，被覆盖的行也能像头一样藏起来。**

一串 prev，两本账本，都成链，在一条有后继对它取过摘要的行上不同——摘要函数把每一行都映到同一个字符串。`theClaimsDecideTheCoveredLines` 与 `aCoveredLineCannotChangeUnnoticed` 用假设排除的就是这种情形，而它不是边角情形：「假设不成立」就长这个样子。树的校验有多强，就等于这条假设有多强，别无其他。

要紧的是这条假设写在哪里。这里没有证明它，也证明不了：它是关于 blake3 的陈述。树能做的，是说清自己的哪些承诺建立在它上面，`tools/adversary/Spec.lean` 第 5 节做的就是这件事——下一个需要更强保证的读者因此知道自己在换什么，而不是事后发现一个洞。 -/
theorem aCoveredLineHidesWithoutInjectivity (genesis a z b : String) (different : a ≠ z) :
    ∃ lines lines' prevs,
      lines ≠ lines' ∧ Chained (fun _ => "") genesis lines prevs
        ∧ Chained (fun _ => "") genesis lines' prevs := by
  refine ⟨[a, b], [z, b], [genesis, ""], ?_, ?_, ?_⟩
  · intro same
    injection same with first _
    exact different first
  · simp [Chained, required, List.dropLast_eq_take]
  · simp [Chained, required, List.dropLast_eq_take]

/-- 账本的头在去掉最后一行之后仍在，形状恰是下面的证明唯一需要的那种：两个及以上元素的列表去掉最后一个，前部保持读者已经校验过的样子。

写在这里，是因为 `List` 没有带这条引理：单元素列表的 `dropLast` 会去掉那个元素，所以从三个元素往下，才是第一次从左边走起的一步。 -/
private theorem dropLastConsCons {α : Type} (first second : α) (rest : List α) :
    (first :: second :: rest).dropLast = first :: (second :: rest).dropLast := by
  simp [List.dropLast_eq_take]

/-- 城再写一行时，读者已经校验过的账本内容依然校验过。

`jsonl::append` 每写一行算一次摘要，从不回读身后的内容；这里把同一件事陈述在声索序列上，要紧的是已经欠下的声索不动。保住的是这些声索，而不是新一行的义务——新记录必须携带什么由 `Chained` 的定义本身给出，它就是 `required` 的下一项。 -/
theorem appendingALineMovesNoClaim (hash : String → String) (genesis : String)
    (lines : List String) (line : String) :
    required hash genesis lines <+: required hash genesis (lines ++ [line]) := by
  cases lines with
  | nil => simp [required]
  | cons first rest =>
    have covered : (first :: rest).dropLast.map hash <+: (first :: rest).map hash := by
      revert first
      induction rest with
      | nil => intro first; simp
      | cons second more ih =>
        intro first
        rw [dropLastConsCons]
        simp only [List.map_cons]
        exact List.cons_prefix_cons.mpr ⟨rfl, ih second⟩
    simp only [required, List.dropLast_concat]
    exact List.cons_prefix_cons.mpr ⟨rfl, covered⟩

end Kernel.Ledger
