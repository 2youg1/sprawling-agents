-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::frames::ask

规定 `frames::ask`（`crates/wire/src/` 下同名的文件）。问与答按问的一方铸造的编号配对，答带它反映到哪一条。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-47d 问与答按 `ask_id` 配对，答带 `as_of`

```rust
pub struct AskId(pub u32);                      // 形状 2；页面按连接铸造，服务端只回显、不判定
pub struct Ask { pub ask_id: AskId, pub query: Query }
ClientFrame::Ask(Ask)
pub struct Answered { pub ask_id: AskId, pub as_of: Seq, pub outcome: AskOutcome }
pub enum AskOutcome { Answer(Answer), Refusal(AxError) }
ServerFrame::Answered(Box<Answered>)            // 对一问的拒绝也走这里
```

**一个答复回到哪一问，由问的一方铸造的编号决定，而不是从答复的内容反推。** 从内容反推需要一张「答复字段 → 问题键」的表，那是「问什么」的第二个权威：`Query` 每加一条，表就得跟着加一行，漏一行的后果是一个永远不落地的答；两个同种、参数不同的问题同时在途时，内容也分不出它们。

- **配对的键由问的一方铸造**：`AskId` 在页面上按连接递增（到 `u32` 上限回到 1），服务端原样回显，不检查唯一——配对是页面的事，服务端再判一次就是第二个家。重连后页面清空在途表，旧连接的 id 不会再来。区间补拉（§8-41）与视图的问共用这一个计数器，两者的 id 不会相撞。
- **拒绝也带 `ask_id`**：`AskOutcome::Refusal` 让对一问的拒绝落到那一问上，那一问回到陈旧状态，由下一个观看者再问；拒绝本身仍交给页面的拒绝角落。拒绝码是 `E_WIRE_MISMATCH` 时整条连接停下，与 `ServerFrame::Refusal` 同一规则。命令的拒绝仍走 `ServerFrame::Refusal`。
- **`as_of` 是答复尚未反映的第一个 `seq`**：`sprawling` 在视图锁内先取视图下一条待折叠记录的 `seq`，再读答复，二者在同一把锁下，所以答复恰好反映 `seq < as_of` 的全部记录、不含其余。什么都没折叠的视图答 `Seq::FIRST`：`Seq::FIRST` 是创世那一行的编号，若 `as_of` 取「最后折叠的一条」，「什么都没折叠」与「已折叠创世」就拼成同一个值，创世之前问出的答复会把随后到来的创世当成已含，从此不再重问。视图锁中毒时 `as_of` 为 `Seq::FIRST`，结果是拒绝。页面据此判陈旧：`seq < as_of` 的事件已在答复里，不再触发重问；问在途时到达的事件记下它的 `seq`，答复的 `as_of` 大于它时，这次陈旧随答复一起消掉。
- **帧名随之换了**（`query`→`ask`，`answer`→`answered`），`WIRE_V` 加一；旧页面在握手处被拒，而不是发出服务端读不懂的帧。
- **`sprawling console` 只有一问在途**，所以它的问一律用 `AskId(0)`，并且不读回 id。
-/

namespace Wire.Frames.Ask

/-- 视图下一条待折叠记录的 `seq`：折过的最后一条之后那一条；什么都没折时是 `Seq::FIRST`，即 1。`folded` 是视图按次序折过的记录的 `seq`，从 1 起连续。 -/
def asOf (folded : List Nat) : Nat :=
  folded.length + 1

/-- 页面判陈旧：`seq < as_of` 的记录已经在答复里。 -/
def Contains (answerAsOf seq : Nat) : Prop :=
  seq < answerAsOf

/-- 被否的读法：`as_of` 取折过的最后一条，什么都没折时也只能写 `Seq::FIRST`。 -/
def lastFolded (folded : List Nat) : Nat :=
  max folded.length 1

/-- **创世之前问出的答复不含创世**：什么都没折的视图答 `Seq::FIRST`，随后到来的创世那一行（`seq` 1）对页面是新闻，问题会被重问。 -/
theorem an_answer_before_genesis_does_not_contain_genesis : ¬ Contains (asOf []) 1 := by
  simp [Contains, asOf]

/-- 折过创世之后的答复含创世。 -/
theorem an_answer_after_genesis_contains_it : Contains (asOf [1]) 1 := by
  simp [Contains, asOf]

/-- **被否的读法分不开两种视图**：什么都没折与只折了创世，取「最后折过的一条」时拼成同一个值，创世之前问出的答复会把随后到来的创世当成已含。 -/
theorem the_last_folded_reading_confuses_nothing_with_genesis : lastFolded [] = lastFolded [1] := by
  decide

end Wire.Frames.Ask
