-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::budget

规定 `kernel::budget`（`crates/kernel/src/budget.rs`）：钱与量的整数化。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`kernel::budget` 旁还没有测试。
-/

/-!
### 8-12 kernel::budget（钱与量的整数化）

```rust
pub struct UsdMicros(u64);  pub struct Tokens(u64);  pub struct ByteLen(u64);   // 钱与量整数化，三新型同家
// 各：pub const fn new(value: u64) / pub const fn get(self) -> u64 / pub fn checked_add(self, other) -> Option<Self>
// checked_add 取 Option 而非 Result：溢出怎么算归调用点定（读作 E_INVALID_ARGS），
// 在原语层预先选一个错误故事会迫使调用方反封 AxError。

pub struct BudgetUse { pub usd: UsdMicros, pub tokens: Tokens }    // serde（Progress::Unplanned 载荷、Evidence.budget）
```

**kernel 不设花费闸**：没有预算上限、花费判定或上下文锁的类型，也没有花费门。

- **理由是刹车只留一个**：`Halt` 停一个范围并终止该范围内的后台成员（`runtime::backlog::halt` 是承兑点）。一座必须停下的城由人说停，而不是由一个没人能在事前算准的上限替他说停。
- **留下的是记账而不是闸**：`BudgetUse` 与 `storage::attribution` 的五路归因、成本页原样保留。**报告花了多少**与**事前不许花**是两件事，kernel 只做前者。
- **不在此列**：`tools/xtask/budgets.toml`（门的价目册，同名异物）与 `Fuel`（wasm 客的停机保证）。
- `BudgetUse` 保留 serde，因为它是 `Progress::Unplanned` 与 `Completion::Evidence` 的载荷字段，账本里已有历史行读得回去。
- 本模块没有 kani harness：三个新型只有构造、取值与 `checked_add`，没有判定可证；树上有几条 harness、CI 证哪几条，以 `cargo xtask proof --list` 为准。
-/
