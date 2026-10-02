-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::plan_view

规定 `crates/accounting/src/plan_view.rs`：每栋楼的计划是一次投影。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-6 accounting::plan_view：计划从每问一次重解析，变成一次投影（形状 7 投影）

`CityView` 与 `Metrics` 每被问一次都要每栋楼的计划。页面是轮询的，而一份计划一小时改不了几次；每问一次就把 `Roadmap.md` 从盘上读出来重新解析，是**为一个几乎不变的答案，按提问频率付钱**。

```rust
pub struct PlanView { /* read、causes —— 私有 */ }
pub struct PlanReading {
    pub progress: Progress,
    pub problems: Vec<String>,
    pub rows: Vec<wire::PlanRow>,
    pub blocked: Vec<wire::BlockedLine>,
    pub ready: Vec<NodeId>,
}
impl PlanView {
    pub fn apply(&mut self, record: &EventRecord);
    pub fn of(&mut self, city_root: &Path, addr: &Address) -> PlanReading;
}
```

- **文件仍然是计划**。变的只是谁去读：`kernel::WriteMoment` 说这张表只在三个时刻被写，而每一个时刻都是一条记录，于是折叠记录、只在有记录点到那栋楼时才回去读文件。
- **失效由两类记录触发，理由不同**。`roadmap_*` 说一个 run 动了计划——既是忘掉已解析副本的理由，也是一件本身值得留着的事实（红的原因）。`checkpoint_committed` 只说一波工具写过文件——**用 edit 工具改了表的 agent 不留 `roadmap_*` 记录**，一个忽略工具波的缓存会继续报改动之前的计划。
- **它是投影不是副本**：这里不存计划说了什么，只存**上一次读到的时候它是什么**，并在任何可能改变它的事情发生时丢掉。删掉整个它、把同一批记录再折一遍，得到同样的字节——因为它做的全部事情就是折叠。
- **它折的唯一一件文件装不下的事，是节点为什么红**。表格有位置说 `Blocked`；人需要的那句话在 `roadmap_blocked` 的记录里，在表里再放一份就是同一句话的第二个权威。没有记录撑着的 `Blocked` 行仍然算红，措辞退回状态词本身——一个人手改的行仍然是一行说着活停了的行。
- **`BuildingView` 也走这一份**：楼的对象页从这里拿计划，只有文档、房间与档案仍在被问的那一刻读盘。让对象页自己再解析一次，就是「什么卡住了、为什么」有两个答案，而只有一个在折记录。
-/

/-! ## 模型：投影只记上一次读到的样子，并在可能改变它的记录到来时忘掉

`View` 是 `PlanView` 的读法：每栋楼至多记一份上一次读到的计划。`apply` 收一行记录，丢掉它点到的楼（`plan_view::reach` 的判定，这里是参数 `moved`）；`of` 有记的就答记的，没有就读盘并记下。性质只有一条，分三步证：读盘的那份只在记录点到它的时刻变（`kernel::WriteMoment` 的三个时刻，前提 `hd`）时，记下的每一份都等于盘上那份，所以 `of` 答的就是盘上此刻的计划。反例钉住为什么 `checkpoint_committed` 也让一栋楼失效：只认 `roadmap_*` 的投影，遇上用 edit 工具改了表的 run，会继续答改动之前的计划。本模型不是 Rust 实现的证明，对应由 `accounting::plan_view::tests` 检查（§16）。
-/

namespace Accounting.PlanView

/-- 每栋楼上一次读到的计划；`none` 是没记或已忘掉。 -/
structure View (B R : Type) where
  read : B → Option R

/-- 一行记录到来：它点到的楼忘掉。 -/
def View.apply {B R : Type} (v : View B R) (moved : B → Bool) : View B R :=
  ⟨fun b => if moved b then none else v.read b⟩

/-- 问一栋楼的计划：记着就答记的，否则读盘并记下。交回答复与之后的投影。 -/
def View.of {B R : Type} [DecidableEq B] (v : View B R) (disk : B → R) (b : B) : R × View B R :=
  match v.read b with
  | some r => (r, v)
  | none => (disk b, ⟨fun b' => if b' = b then some (disk b) else v.read b'⟩)

/-- 记下的每一份都是盘上此刻的那份。 -/
def Coherent {B R : Type} (v : View B R) (disk : B → R) : Prop :=
  ∀ b r, v.read b = some r → r = disk b

theorem a_new_view_is_coherent {B R : Type} (disk : B → R) : Coherent ⟨fun _ => none⟩ disk := by
  intro b r h; simp at h

/-- 盘上只在被点到的楼变时，忘掉被点到的楼就仍与盘一致。 -/
theorem apply_keeps_coherence {B R : Type} (v : View B R) (moved : B → Bool) (disk disk' : B → R)
    (h : Coherent v disk) (hd : ∀ b, moved b = false → disk' b = disk b) :
    Coherent (v.apply moved) disk' := by
  intro b r hr
  by_cases hm : moved b = true
  · simp [View.apply, hm] at hr
  · have hf : moved b = false := by simpa using hm
    simp [View.apply, hf] at hr
    rw [hd b hf]; exact h b r hr

/-- 一致的投影答的就是盘上此刻的计划。 -/
theorem of_answers_the_disk {B R : Type} [DecidableEq B] (v : View B R) (disk : B → R) (b : B)
    (h : Coherent v disk) : (v.of disk b).1 = disk b := by
  unfold View.of
  cases hr : v.read b with
  | none => rfl
  | some r => exact h b r hr

/-- 答过之后仍一致：读盘记下的那份就是盘上的那份。 -/
theorem of_keeps_coherence {B R : Type} [DecidableEq B] (v : View B R) (disk : B → R) (b : B)
    (h : Coherent v disk) : Coherent (v.of disk b).2 disk := by
  unfold View.of
  cases hr : v.read b with
  | some r => exact h
  | none =>
    intro b' r' hr'
    by_cases he : b' = b
    · subst he; simp at hr'; exact hr'.symm
    · simp [he] at hr'; exact h b' r' hr'

/-- 反例：楼 0 的计划被一波工具改成 2，而投影只认计划记录、没把这一行当作点到楼 0，于是继续答 1。 -/
theorem a_view_that_ignores_tool_waves_answers_the_old_plan :
    let v : View Nat Nat := ⟨fun b => if b = 0 then some 1 else none⟩
    let roadmapOnly : Nat → Bool := fun _ => false
    ((v.apply roadmapOnly).of (fun _ => 2) 0).1 = 1 := by
  decide

end Accounting.PlanView

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/sprawling-SPEC.md` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-76 | `accounting::plan_view::reach` |
-/
