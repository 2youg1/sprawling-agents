-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::approval

规定 `kernel::approval`（`crates/kernel/src/approval.rs`）：Approval Inbox 的条目、谁来答与 City Hall 的常量。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-21 kernel::approval

```rust
pub struct ApprovalId(String);               // 非空；铸造只此一处，与时钟无关
impl ApprovalId {
    pub fn of(run: &RunId, seq: Seq) -> ApprovalId;   // 唯一铸口：`ap-{run}-{seq:020}`
    pub fn of_sweep(run: &RunId) -> ApprovalId;       // = of(run, Seq::new(u64::MAX))
    pub fn new(raw: impl Into<String>) -> Option<ApprovalId>;   // 只收既存 id（线上回执、账本、夹具），空串拒
    pub fn as_str(&self) -> &str;
}
pub enum ApprovalClass { Question }          // Inbox只装设计问题
pub struct ClusterKey { pub class: ApprovalClass, pub detail: String }
pub struct ApprovalItem { pub id: ApprovalId, pub actor: String, pub action_desc: String,
                          pub artifact: Locator, pub cluster_key: ClusterKey,
                          pub created: TimeMs, pub tainted: bool }
pub enum Ruling { Allow, Deny }              // 人给一条问题的答复
pub enum Autonomy { Owner, Delegate(ResidentId) }
pub enum Answerer { Human, Resident(ResidentId) }
pub enum AnswerVerdict { May, NotTheDelegate, SelfApprovalBarred }
pub fn may_answer(autonomy: &Autonomy, item: &ApprovalItem, answerer: &Answerer) -> AnswerVerdict;
```

- **身份取自 run 与位次，不取自时钟**：车道不设上限（`crates/sprawling/Spec.lean` D34），几条车道常在同一毫秒各提第一个问题；毫秒形状的 id 会让两条成为一个键，先那条从只增账本里消失，而事后无人能把「丢了」与「从未发生」分开。`seq` 是该 run 自己的单调位次——run 已经用来 derive `IdemKey` 的那一个计数器，在一个 run 内计数、从不跨 run 传递。两个输入都不是采样值，重放逐字节重算出同一 id。位次按 `u64::MAX` 的宽度补零书写，于是同一 run 两条 id 的派生 `Ord` 读出的就是它们被提出的先后。
- **清扫槽位**：一次 drive 至多一条，占最高位次（`of_sweep`），调用计数器永远数不到那里。位次空间的这条划分只有 kernel 这一个家；调用方不自拼字符串。
- **`ApprovalClass` 只剩一个臂而枚举留下**：cluster key 是线上数据，类别写在载荷里；第二种问题出现的那天要在每一个读者处编译失败，而不是让一个字段悄悄改变含义。
- **`ApprovalItem.tainted` 是给人看的出处，不是判决位**：污染改变的是效果的判决（`gate::undoable` 与 `gate::discard` 的 Deny），而一个问题不是效果。
- `may_answer`：Human 恒 May；Resident r 仅当 autonomy==Delegate(r)（否则 NotTheDelegate）且 item.actor ≠ r（否则 SelfApprovalBarred）。
- **真值表由测试遍历**：两个 autonomy × 三个应答者 × 两个提问者＝十二行，逐行断言；表里加一行就是断言加一行。
- verdict 先落账再生效：效果层顺序约束，kernel 只出判定。
- `AUTONOMY_DEFAULT: Autonomy = Owner` 落 consts_policy。
-/

/-!
### 8-47 kernel::approval：City Hall 的两个常量与 clerk 的默认代答

```rust
// consts_policy
pub const HALL_BUILDING: &str = "hall";
pub const HALL_MAYOR: &str = "hall/mayor";
pub const HALL_CLERK: &str = "hall/clerk";
```

- `Autonomy` **不加变体**：`Owner | Delegate(ResidentId)` 已经能说出「clerk 代答」——`Delegate(ResidentId::new(HALL_CLERK))`。新增一个 `Clerk` 变体会让同一件事有两种写法，而 `may_answer` 要为两种都作答。
- `may_answer` 逻辑对 clerk 无特例：clerk 之所以能答，是因为它就是被任命的 delegate；clerk 自己发起的条目依旧 `SelfApprovalBarred`。kernel 侧只加常量与一条断言 clerk 走通全路的测试。
- genesis 侧（`bin::assembly`）在 `city_initialized` 之后写一条 `autonomy_changed`，值为 `delegate:hall/clerk`——记录在账上而不是写死在缺省值里，因为「谁来答」是这座城市的一个决定，人可以改它，改动要有一行历史。
-/

namespace Kernel.Approval

/-- 谁来答一条问题，与 `kernel::Autonomy` 逐变体同名；居民身份 `ResidentId` 在模型里是它的拼写。 -/
inductive Autonomy where
  | Owner
  | Delegate (resident : String)
  deriving DecidableEq, Repr

/-- 来答的是谁，与 `kernel::Answerer` 逐变体同名。 -/
inductive Answerer where
  | Human
  | Resident (resident : String)
  deriving DecidableEq, Repr

/-- `may_answer` 的答案，与 `kernel::AnswerVerdict` 逐变体同名。 -/
inductive AnswerVerdict where
  | May
  | NotTheDelegate
  | SelfApprovalBarred
  deriving DecidableEq, Repr

/-- `may_answer`：人恒可答；居民只在它就是被任命的 delegate、且这条问题不是它自己提的时可答。`actor` 是 `ApprovalItem.actor`。clerk 没有特例：它能答，是因为它就是被任命的 delegate（8-47）。 -/
def may_answer (autonomy : Autonomy) (actor : String) : Answerer → AnswerVerdict
  | .Human => .May
  | .Resident resident =>
    if autonomy ≠ .Delegate resident then .NotTheDelegate
    else if actor = resident then .SelfApprovalBarred
    else .May

/-- 人恒可答。 -/
theorem a_person_may_always_answer (autonomy : Autonomy) (actor : String) :
    may_answer autonomy actor .Human = .May :=
  rfl

/-- **居民可答，当且仅当它是被任命的 delegate、且问题不是它自己提的。** 两个 autonomy、三个应答者、两个提问者的十二行真值表（`approval::tests`）是这一条的逐值检查。 -/
theorem a_resident_answers_only_as_the_delegate_and_never_its_own (autonomy : Autonomy)
    (actor resident : String) :
    may_answer autonomy actor (.Resident resident) = .May ↔
      autonomy = .Delegate resident ∧ actor ≠ resident := by
  simp only [may_answer]
  split
  · rename_i other
    simp [other]
  · rename_i appointed
    split <;> simp_all

/-- 一条问题的提问者恒不能批准它自己，被任命的 delegate 也一样。 -/
theorem nobody_approves_their_own_question (resident : String) :
    may_answer (.Delegate resident) resident (.Resident resident) = .SelfApprovalBarred := by
  simp [may_answer]

end Kernel.Approval
