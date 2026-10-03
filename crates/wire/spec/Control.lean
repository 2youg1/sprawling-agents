-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.wire.spec.Command.Kind

/-!
# wire::control

规定 `control`（`crates/wire/src/` 下同名的文件）。人的干预动词，与其中哪些欠一份 Handoff。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-4 wire::control（形状 1 判定函数）

```rust
pub enum Intervention { Steer, Cancel, Halt, Release }
pub enum ControlVerdict {
    Intervene { verb: Intervention, run: Option<RunId>, must_write_handoff: bool },
    NotAnIntervention,
    Refuse(AxError),
}
pub fn classify(command: &Command) -> ControlVerdict;
```

**本模块持有的唯一规则**：中断一个活着的 Run 的两个动词（Steer／Cancel）**恒以 Handoff 收尾**，使下一位（人或 Agent）拿得到完整现场。`Halt`／`Release` 按 scope 停一片，不针对单个 Run，故 `run` 为 `None` 且不强制 Handoff。

**`must_write_handoff` 为什么是返回值而不是副作用**：本 crate 不持 Ledger 句柄（§7 已写）。它只能**说出义务**，履行义务的是装配层。把它做成返回值的代价是装配层可能忽略它——故同变更集交付一条断言：一次干预的事件序里若无 `handoff_written`，即失败。

**为什么不并入 `server`**：`server` 的职责是「这个字节流能不能变成一个帧」，`control` 的职责是「这个帧是不是干预、干预要留下什么」。后者在无网络的 citysim 里也成立，前者不。两个职责的变化率也不同：加一个动词不应该碰监听器。
-/

namespace Wire.Control

open Wire.Command.Kind

/-- 人的干预动词（`control::Intervention`）。 -/
inductive Intervention where
  | Steer
  | Cancel
  | Halt
  | Release
  deriving DecidableEq, Repr

/-- `ControlVerdict`：一次干预、它针对不针对一个 Run、它欠不欠一份 Handoff；或不是干预。`Steer` 的文字为空时的 `Refuse` 是对一个字段的判定，命令在模型里不带字段，所以这一臂不写。 -/
inductive ControlVerdict where
  | Intervene (verb : Intervention) (run : Bool) (mustWriteHandoff : Bool)
  | NotAnIntervention
  deriving DecidableEq, Repr

/-- `classify`：中断一个活着的 Run 的两个动词恒以 Handoff 收尾；按 scope 停一片与放开一片不针对单个 Run。 -/
def classify : Command → ControlVerdict
  | .Steer => .Intervene .Steer true true
  | .Cancel => .Intervene .Cancel true true
  | .Halt => .Intervene .Halt false false
  | .Release => .Intervene .Release false false
  | _ => .NotAnIntervention

/-- 一个判定欠不欠一份 Handoff。 -/
def ControlVerdict.owesHandoff : ControlVerdict → Bool
  | .Intervene _ _ owes => owes
  | .NotAnIntervention => false

/-- **欠 Handoff 的恰是中断一个活着的 Run 的两个动词。** -/
theorem a_handoff_is_owed_exactly_by_steer_and_cancel (c : Command) :
    (classify c).owesHandoff = true ↔ c = .Steer ∨ c = .Cancel := by
  cases c <;> decide

/-! 按范围停与放开不针对单个 Run。 -/

end Wire.Control
