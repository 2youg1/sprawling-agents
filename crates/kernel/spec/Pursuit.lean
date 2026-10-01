-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::pursuit

规定 `kernel::pursuit`（`crates/kernel/src/pursuit.rs`）：城的常设目标与它何时停下。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-35 kernel::pursuit（形状 1 判定）

```rust
pub enum PursuitState { Running, Paused }        // 携 serde
pub struct Pursuit { /* 私有；无 serde */ }
impl Pursuit {
    pub fn declare(at: &Delegator, goal: String) -> Result<Pursuit, AxError>;
    pub fn goal(&self) -> &str;
    pub fn state(&self) -> PursuitState;
    pub fn pause(&mut self);
    pub fn resume(&mut self);
}
pub enum PursuitVerdict { Work { next: NodeId }, Waiting { in_flight: u32 }, Paused, Finished }  // 携 serde，内标签 kind
pub fn observe(state: PursuitState, ready: &[NodeId], in_flight: u32) -> PursuitVerdict;
```

- **不叫 Endless，按它是什么命名**：本仓已有好几处叫 standing 的类型（如 `city::wizard::Standing`、`accounting::worker::folds::Standing`、`GoalEntry.standing`），再加一个会让这个词再多一个含义。
- **一个社会停下来不是因为有人喊停，是因为没有就绪的活了。** 依据只此一条：就绪集为空**且**没有在途的 run。两半都要——就绪集空而四个 run 在跑，意思是活在别人手上，不是活干完了。
- **钱明确不是停机条件**。本仓的成本面受众是 Agent（给它优化的材料），不是刹车；一个读预算的停机条件回答的是一个这里没人问的问题。
- **`observe` 收状态而不收 `Pursuit`**：判定不依赖目标说了什么，而一个必须先持有 `Pursuit` 才能发问的读者，等于要拿深度零位才能**读**这座城。**声明是被守的动作，看不是。**
- **子代理拼不出来**：`declare` 收 `&Delegator`，而 `Delegate` 造不出一个（trybuild `delegate_declares_pursuit`）。与 `delegation` 同一个两层守卫，理由也同一个：一个能让全城通宵干活的子代理，就是一个能替你决定通宵干什么的子代理。
- **`PursuitVerdict` 原样上线**：线上是 `{"kind":"work","next":"2.3"}` 这类内标签形状，`PursuitLine.verdict` 就是它。人读的那句话由客户端按 `kind` 从 `lang.json` 取词；城若在线上给一句英文，中文界面就只能照抄英文，而两边各写一份措辞就是同一句话的两个权威。
- **pause 与 clear 是两件事，都要**：暂停留着目标，清除把它丢掉（丢掉值本身，于是不会被误恢复）。取消一个 **run** 是第三件事，住在 run 那边。
-/

namespace Kernel.Pursuit

/-- 与 `kernel::PursuitState` 逐变体同名。 -/
inductive PursuitState where
  | Running
  | Paused
  deriving DecidableEq, Repr

/-- 与 `kernel::PursuitVerdict` 逐变体同名；`NodeId` 在模型里是它的拼写。 -/
inductive PursuitVerdict where
  | Work (next : String)
  | Waiting (in_flight : Nat)
  | Paused
  | Finished
  deriving DecidableEq, Repr

/-- `pursuit::observe`：暂停着就答暂停；否则就绪集的第一个是下一步；就绪集空而有在途的 run 就等；两者都空才是完成。钱不是参数。 -/
def observe (state : PursuitState) (ready : List String) (in_flight : Nat) : PursuitVerdict :=
  match state, ready with
  | .Paused, _ => .Paused
  | .Running, next :: _ => .Work next
  | .Running, [] => if 0 < in_flight then .Waiting in_flight else .Finished

/-- **一个社会停下来，是因为没有就绪的活、也没有在途的 run。** 两半都要：就绪集空而四个 run 在跑，意思是活在别人手上，不是干完了。 -/
theorem finished_exactly_when_nothing_is_ready_or_in_flight (state : PursuitState)
    (ready : List String) (in_flight : Nat) :
    observe state ready in_flight = .Finished ↔ state = .Running ∧ ready = [] ∧ in_flight = 0 := by
  cases state <;> cases ready <;> simp [observe] <;> split <;> simp_all <;> omega

end Kernel.Pursuit
