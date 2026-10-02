-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::effect

规定 `crates/accounting/src/effect.rs`：一张桌子留下的效应先成为账本行、再成为这座城，以及计划的效应怎样重放。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-5 accounting::effect：一条效应先成为账本行，再成为这座城（形状 2 值类型）

```rust
// crates/accounting/src/effect.rs —— `architecture.toml` 的 accounting::effect，形状 2（值类型）
pub struct Line { pub who: String, pub addr: Address, pub kind: EventKind, pub data: Payload }
pub struct Filing { pub entry: city::ArchiveEntry, pub body: String }

/// 一张桌子留下的全部效应：它们成为的行，以及行之后才允许发生的变化。
pub struct Landing<L = Line> { lines: Vec<L>, then: Then }        // 两个字段都是私有的
/// 计划落地的一行，与它关掉的那个认领；拆分关掉它的父节点。
pub struct Closing { pub line: Line, pub closes: Option<NodeId> }

pub enum Then { Nothing, Deliver(Vec<collab::Signal>),
                Roadmap { path: PathBuf, base: String, text: String }, Shelf(Vec<Filing>) }

impl<L> Landing<L> {
    /// 先走完每一行，再把变化交出去。这是 `Then` 唯一的出口。
    pub fn record(self, append: &mut impl FnMut(L) -> Result<(), AxError>) -> Result<Then, AxError>;
}
impl Landing {
    pub fn signals(Vec<SignalEffect>, room: &Address, who: &str) -> Result<Landing, AxError>;
    pub fn discards(Vec<Payload>, room: &Address, who: &str) -> Landing;
    pub fn shelf(Vec<ArchiveEffect>, write_root: &Path, building: &Address, at: TimeMs, room: &Address, who: &str) -> Result<Landing, AxError>;
}

/// 一跑对共享计划做的事。两种而无第三种：效应按次序重放到盘上那份、每条都还对得上，或有一条认领对不上、一个字也不写（§8-27）。
pub enum Claims {
    Landed(Box<Landing<Closing>>),
    Stale { node: NodeId, released: Box<Landing<Closing>> },   // 第一条对不上的认领报给人；released 关掉本跑已落账的认领
}
impl Claims { pub fn of(effects: &[ClaimEffect], on_disk: &str, path: PathBuf, room: &Address, who: &str) -> Result<Claims, AxError>; }

// 装配层那一扇门（accounting::worker::settling::landing）：每张桌子都走它，`Then` 的 match 穷尽
impl RunWorker { fn settle(&mut self, at: &Assignment, run: RunId, landing: effect::Landing, chain: &KnockChain) -> Result<(), AxError>; }
```

**原因**：先把效应变成账本行、再变成状态，是 Ledger 的定义（`docs/glossary.md`：「Every effect becomes an EventRecord first」；ARCHITECTURE.md §5 步 4）。写反的代价是具体的：先上书架后落账，落账失败就在架上留下一条历史没有的记录；先改共享计划后落账，而 `roadmap_claimed` 是 `storage::hot` 与 `storage::projection` 判断谁拿着哪一行的依据，写进了文件而没落账的认领是一行看上去有人占着、历史里却无人占着的行。

**形状**：先后是类型的性质，而不是写桌子的人的纪律。`Then` 只能从 `Landing::record` 里拿到，而 `record` 先把所有行送进去才返回它；要把顺序写反，得先拿到一个拿不到的值。

- **批而不是逐条**：一张桌子的行全部落完，才轮到它的变化。signal 一支因此先落完所有 `signal_enqueued` 再投递；`deliver` 与 `knock` 都不写账，所以账本字节与逐条交错时相同。
- **计划那一支是全有全无的**，所以它自己一个穷尽枚举 `Claims`：效应按次序重放、只有认领核盘上的状态（§8-27），任一条认领对不上，就一字不写，把那个节点报给人，并用 `released` 里的 `roadmap_released` 行关掉本跑已经落账的认领（`crates/sprawling/Spec.lean` §8-16 的形制）。效应重放到 `on_disk` 上，而不是写回派活时的副本，所以别的 run 在此期间落下的行保留。记账线程在模型认领时已经拒绝了另一个在飞 run 持有的节点（`accounting::worker::booking`，`crates/sprawling/Spec.lean` §8-42-8）；`Claims::of` 是后盾，接住从旧副本认领了已被别人落地的节点的 run。
- **归档行不需要先写盘**：账本行要的 `kind`／`day`／`subject` 由 `city::archive_entry` 从入参算出（`crates/city/Spec.lean` §8-9）。不在装配层另算 `day_of`，因为那会是「一条归档记录长什么样」的第二个权威。
- **`raised`（待批项）不进本模块**：它不是桌子交出来的效应，而是驱动期间暂存的项，本身就先落账后改状态。

**pr 那两支不走 `Landing`，理由记在这里**：

- `PrEffect::Opened` 里的 `storage::Checkpoint::land` 先于 `pr_opened` 落账，**但它不是「先动世界」**。它铸出的是那条账本行所指向的 commit，与 `run_started` 之前把 brief 放进 CAS 同形：没有任何记录指向的 git commit 不改变任何人读到的东西。
- `PrEffect::Merged` 先经 `Worktrees::plan_merge` 定下这次合并会落在哪个 commit，干线已经动过的拒绝（`MergeStale`）在这一步就报出，然后才写 `pr_merged`。所以不会有一条 `pr_merged` 是替一次注定被拒的合并写的（`crates/sprawling/Spec.lean`「合并也排到它那条行后面」）。

**测试**：`what_a_run_changes_is_changed_after_the_line_that_announces_it`（`accounting::worker::driving::tests::ledger`）。一跑归档一条决定、又从共享计划里拿一行；`RunWorker::observe` 的 sink 在一行耐久之后才跑，所以它正是看得见「先」的位置。断言：`asset_archived` 落时书架上还没有它，`roadmap_claimed` 落时盘上那一行还没被拿走；跑完两者都在位。

**影响面**：`accounting` 公开面有 `effect` 模块，因为写它的桌子在 `bin::assembly`；`city` 的 `archive_entry` 与 `collab` 的效应类型是它的入参，所以本 crate 依赖 `city` 与 `collab`（ARCHITECTURE.md §3 的 `depmap`）。
-/

/-!
### 8-27 accounting::effect：计划的效应按次序重放，只有认领核盘上的状态（形状 2 值类型；collab D6）

```rust
// crates/accounting/src/effect.rs
pub enum Claims {
    Landed(Box<Landing<Closing>>),
    Stale { node: NodeId, released: Box<Landing<Closing>> },
}
impl Claims { pub fn of(effects: &[ClaimEffect], on_disk: &str, path: PathBuf, room: &Address, who: &str) -> Result<Claims, AxError>; }
```

- **按次序重放，每条在前面几条留下的文本上核。** `Claims::of` 从盘上此刻的那份出发，对每条效应先问 `collab::still_true`，再用 `ClaimEffect::apply` 改文本；本 run 自己的前几条效应因此是后几条的前提，不需要「每个节点只核第一条」的例外。本 run 分了自己握着的一行、再认领其中一片叶子，那条认领核的是拆完的文本，两条都落下。
- **只有认领会对不上。** 认领要那一行仍是 `Not started`；放下与拆分只作用于本 run 握着的那一行（collab D6），握持之前的那条认领已经在重放里核过，落地不为它们另判一个期待状态。「分一行要不要握着它」于是只有桌子那一处判定（`crates/collab/spec/Claim.lean` 的 `land` 与 `admitted_lands`）。
- **过时报第一条对不上的认领，重放就停在那里。** `Stale.node` 是那一行；`released` 照旧关掉本跑每一条已落账的认领。停下而不是接着核：被丢下的那条认领之后可能跟着它拆出的子行，接着核会把还不存在的子行报成「被动过」，而接着重放就得吞掉 `apply` 对一行已被别人改掉的拒绝。
- **装配层**：`accounting::worker::settling::desks` 的 `Stale` 一臂为那一行留一条诊断（`collab::claim_tool`，Refuse 级），其余不变。

**测试**：`a_run_that_splits_its_row_and_claims_a_leaf_lands_both`（`accounting::effect::tests`）；`a_split_of_a_row_this_run_does_not_hold_is_refused_at_the_call`（`collab::claim_tool::split_tests`）。
-/

/-! ## 模型：先落行，后交出变化；计划的效应按次序重放

`record` 是 `Landing::record` 的读法：按次序追加每一行，全部落下之后才交出 `Then`；第一行被拒就停下，`Then` 不交出，所以没有一种走法能在某一行落下之前拿到它之后才允许的变化。`replay` 是 `Claims::of` 的读法：效应按次序作用在前面几条留下的文本上，只有认领核状态，第一条对不上的认领让整次重放停下并点名那一行。文本、节点与「还对得上」都是参数：`collab::still_true` 与 `ClaimEffect::apply` 的文法是 collab 的（`crates/collab/spec/Claim.lean`），这里只陈述次序。本模型不是 Rust 实现的证明，对应由 `accounting::worker::driving::tests::ledger` 与 `accounting::effect::tests` 检查（§16）。
-/

namespace Accounting.Effect

/-- `Landing::record`：`append` 答这一行是否耐久地落下（Rust 的 `Ok(())`）。交回落下的行与交出的 `Then`；`none` 是 Rust 的 `Err`。 -/
def record {L T : Type} (append : L → Bool) : List L → T → List L × Option T
  | [], t => ([], some t)
  | l :: ls, t =>
    if append l then ((l :: (record append ls t).1), (record append ls t).2) else ([], none)

/-- 交出 `Then` 时，每一行都已按原来的次序落下，交出的就是桌子给的那一个。 -/
theorem then_comes_after_every_line {L T : Type} (append : L → Bool) (ls : List L) (t t' : T)
    (h : (record append ls t).2 = some t') : (record append ls t).1 = ls ∧ t' = t := by
  induction ls with
  | nil => simp [record] at h ⊢; exact h.symm
  | cons l ls ih =>
    by_cases ha : append l = true
    · simp [record, ha] at h ⊢; exact ih h
    · simp [record, ha] at h

/-- 有一行没落下，`Then` 就不交出：先上架后落账、先改计划后落账都写不出来。 -/
theorem a_refused_line_withholds_then {L T : Type} (append : L → Bool) (ls : List L) (t : T)
    (h : ∃ l ∈ ls, append l = false) : (record append ls t).2 = none := by
  induction ls with
  | nil => simp at h
  | cons x xs ih =>
    by_cases ha : append x = true
    · simp [record, ha]
      apply ih
      obtain ⟨l, hl, hf⟩ := h
      rcases List.mem_cons.mp hl with rfl | hl'
      · simp [ha] at hf
      · exact ⟨l, hl', hf⟩
    · simp [record, ha]

/-- 正常路径可实现：两行都落下，变化在它们之后交出。 -/
theorem two_lines_land_then_the_change : record (fun (_ : Nat) => true) [1, 2] "shelf" = ([1, 2], some "shelf") := by
  rfl

/-- 一跑对共享计划做的事的读法：哪些效应是认领、点名哪一行；一条效应在一份文本上是否还对得上；它怎样改文本。 -/
structure Plan (S E N : Type) where
  claimOf : E → Option N
  holds : E → S → Bool
  apply : E → S → S

/-- `Claims` 的两臂：全部重放得上，交回重放之后的文本；或第一条对不上的认领点名的那一行。 -/
inductive Claims (S N : Type) where
  | Landed (text : S)
  | Stale (node : N)
  deriving Repr, DecidableEq

/-- `Claims::of`：从盘上此刻的那份出发按次序重放，只有认领核对状态。 -/
def replay {S E N : Type} (p : Plan S E N) : List E → S → Claims S N
  | [], s => .Landed s
  | e :: es, s =>
    match p.claimOf e with
    | some n => if p.holds e s then replay p es (p.apply e s) else .Stale n
    | none => replay p es (p.apply e s)

/-- 落下的文本就是按次序把每条效应作用上去的结果：别的 run 在盘上留下的行保留，因为起点是盘上那份。 -/
theorem landed_text_is_the_ordered_replay {S E N : Type} (p : Plan S E N) (es : List E) (s t : S)
    (h : replay p es s = .Landed t) : t = es.foldl (fun s e => p.apply e s) s := by
  induction es generalizing s with
  | nil => simp [replay] at h; simp [h]
  | cons e es ih =>
    cases hc : p.claimOf e with
    | none => simp [replay, hc] at h; exact ih _ h
    | some n =>
      by_cases hh : p.holds e s = true
      · simp [replay, hc, hh] at h; exact ih _ h
      · simp [replay, hc, hh] at h

/-- 只有认领会对不上：一串没有认领的效应总是落下。 -/
theorem only_a_claim_is_stale {S E N : Type} (p : Plan S E N) (es : List E) (s : S)
    (h : ∀ e ∈ es, p.claimOf e = none) : ∃ t, replay p es s = .Landed t := by
  induction es generalizing s with
  | nil => exact ⟨s, rfl⟩
  | cons e es ih =>
    have he : p.claimOf e = none := h e (List.mem_cons_self ..)
    simp only [replay, he]
    exact ih (p.apply e s) (fun x hx => h x (List.mem_cons_of_mem _ hx))

/-- 一个具体的计划：每行是（节点，是否 `Not started`）。拆分给本 run 握着的一行加一个空着的子行；认领要那一行仍空着，并把它拿走。 -/
inductive Step where
  | split (row child : Nat)
  | claim (row : Nat)
  deriving Repr, DecidableEq

def rows : Plan (List (Nat × Bool)) Step Nat where
  claimOf := fun e => match e with
    | .claim n => some n
    | .split _ _ => none
  holds := fun e s => match e with
    | .claim n => s.contains (n, true)
    | .split _ _ => true
  apply := fun e s => match e with
    | .claim n => s.map (fun r => if r = (n, true) then (n, false) else r)
    | .split _ c => s ++ [(c, true)]

/-- 本 run 分了自己握着的一行、再认领其中一片叶子：认领核的是拆完的文本，两条都落下。 -/
theorem a_run_that_splits_its_row_and_claims_a_leaf_lands_both :
    replay rows [.split 1 11, .claim 11] [(1, false)] = .Landed [(1, false), (11, false)] := by
  decide

/-- 只对盘上原文判认领时，那片叶子在盘上还不存在，同一次落地会被判过时：这正是按次序重放要免掉的那一种。 -/
theorem the_disk_alone_would_call_the_leaf_stale :
    rows.holds (.claim 11) [(1, false)] = false := by
  decide

/-- 别人先拿走了那一行：第一条对不上的认领被点名，重放停在那里。 -/
theorem a_row_taken_meanwhile_is_stale :
    replay rows [.claim 2, .claim 3] [(2, false), (3, true)] = .Stale 2 := by
  decide

end Accounting.Effect

/-! D39 计划的效应按次序重放、每条在前面几条留下的文本上核，只有认领核盘上的状态；过时报第一条对不上的认领（§8-27）

理由：「分一行要不要握着它」只由桌子判（collab D6），落地若再为拆分、放下各判一个期待状态，就是同一条规则的第二份拼写，`tools/adversary/Spec.lean` §4 的第八个发现正是两份拼写不一致的样子。按次序重放让本 run 的前几条效应成为后几条的前提，「每个节点只核第一条效应」那条例外随之没有了，本 run 拆出又认领的子行也不再被判过时。被否决的做法：①照旧只核每个节点的第一条效应、对盘上原文判——拆出的子行在盘上还不存在，认领它的那条被判过时，工具答了成功、落地一字不写，与第八个发现同一类；②过时之后接着重放，把每一条对不上的认领都报出来——被丢下的拆分之后的子行认领会被误报，而接着重放就得吞掉 `apply` 的拒绝。重开参数：一个 run 能同时握多行时，一行过时不该挡住别的行，那时按行分组判。
-/
