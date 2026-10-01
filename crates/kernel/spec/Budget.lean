-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::budget

规定 `kernel::budget`（`crates/kernel/src/budget.rs`）：钱与量的整数化。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-12 kernel::budget（钱与量的整数化）

```rust
pub struct UsdMicros(u64);  pub struct Tokens(u64);  pub struct ByteLen(u64);   // 钱与量整数化，三新型同家
// 各：pub const fn new(u64) / pub const fn get() / pub fn checked_add(self, o) -> Option<Self>
// checked_add 取 Option 而非 Result：溢出怎么算归调用点定（读作 E_INVALID_ARGS），
// 在原语层预先选一个错误故事会迫使调用方反封 AxError。

pub struct BudgetUse { pub usd: UsdMicros, pub tokens: Tokens }    // serde（Progress::Unplanned 载荷、Evidence.budget）
```

**kernel 不设花费闸**：没有预算上限、花费判定或上下文锁的类型，也没有花费门。

- **理由是刹车只留一个**：`Halt` 停一个范围并终止该范围内的后台成员（`runtime::backlog::halt` 是承兑点）。一座必须停下的城由人说停，而不是由一个没人能在事前算准的上限替他说停。
- **留下的是记账而不是闸**：`BudgetUse` 与 `storage::attribution` 的五路归因、成本页原样保留。**报告花了多少**与**事前不许花**是两件事，kernel 只做前者。
- **不在此列**：`tools/xtask/budgets.toml`（门的价目册，同名异物）与 `Fuel`（wasm 客的停机保证）。
- `BudgetUse` 保留 serde，因为它是 `Progress::Unplanned` 与 `Completion::Evidence` 的载荷字段，账本里已有历史行读得回去。
- kani：`admit_spend` 的 harness 随函数删除；`crates/kernel` 的 harness 总数与 CI 所证条数因此各少一条，被证的 `budget` 一条随函数一起消失（ARCHITECTURE §11 的数字同集更新）。**这里记的是那次变更当时的读数，不是今天的基数**；今天树上有几条、CI 证哪几条，以 `cargo xtask proof --list` 为准。
-/
