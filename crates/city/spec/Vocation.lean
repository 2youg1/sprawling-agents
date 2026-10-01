-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address

/-!
# city::vocation

规定 `vocation`（`crates/city/src/` 下同名的文件）。一个地址上的居民是来建造的还是来规划的。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-22 city::vocation：一个地址上的居民是来建造的还是来规划的

**接口**：

```rust
// city::vocation（形状 1 判定；无 I/O、无时钟）
pub enum Vocation { Builds, Plans }   // 穷尽；第三种职分＝第三个臂
pub fn vocation_of(building: &Address) -> Vocation;
```

- **按地址判，不按提示词判**：City Hall 的居民写 Markdown 和计划，不建造。装配层据此为它组工具台：没有 `exec`、没有 `delegate`、没有 `workshop`。把这件事交给 `MAYOR.md` 的措辞，就是把不变量交给提示词。
- **返回穷尽枚举而不是 bool**：`is_hall()` 只答得出「是不是市政厅」，而调用点要问的是「这个地址上的人是干什么的」。第三种职分出现时，缺臂是编译错误，不是一个悄悄落到 `else` 里的新楼。
- **依据是首段等于 `kernel::consts_policy::HALL_BUILDING`**：`hall` 这个名字只有一处权威，本模块不重抄字面量。
-/

/-!
## 模型：只有 City Hall 规划

`vocation_of` 拿楼的地址与 `kernel::consts_policy::HALL_BUILDING` 比；这个名字只有那一个家，模型里它是参数 `hall`。

* 规划的恰是 City Hall 那一栋（`only_the_hall_plans`）；名字以 `hall` 开头的别的楼照样建造（`example`），比较的是整个地址而不是前缀。
-/

namespace City.Vocation

open Kernel.Address

/-- `Vocation`：建造，或规划。 -/
inductive Vocation where
  | Builds
  | Plans
  deriving DecidableEq, Repr

/-- `vocation_of`。 -/
def vocation_of (hall : String) (building : Address) : Vocation :=
  if building = ⟨[hall]⟩ then .Plans else .Builds

theorem only_the_hall_plans (hall : String) (building : Address) :
    vocation_of hall building = .Plans ↔ building = ⟨[hall]⟩ := by
  unfold vocation_of
  split <;> simp_all

example : vocation_of "hall" ⟨["hallway"]⟩ = .Builds := by decide

end City.Vocation
