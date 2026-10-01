-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::share

规定 `kernel::share`（`crates/kernel/src/share.rs`）：守恒的份额。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-32 kernel::share（形状 2 value）

```rust
pub struct Share(/* u64 私有：十亿分之一 */);
pub const WHOLE_PPB: u64 = 1_000_000_000;
impl Share {
    pub const WHOLE: Share;                 // 唯一原点
    pub const NONE:  Share;                 // 加不出来，故可公开
    pub fn ppb(self) -> u64;
    pub fn split(self, weights: &[u32]) -> Result<Vec<Share>, AxError>;   // 取走自己，分出恰好等于自己的诸份
}
pub fn gather(parts: &[Share]) -> Share;    // 自由函数，不是 Add
```

- **守恒不是被检查的规则，而是类型唯一能表达的事。** 份额只有两种来路：整份计划，或者**分掉另一份得到的一片**。`split` 按值取走输入，于是「凭空多出一份」拼不出来（trybuild `forge_share`）。没有构造器、没有算术、没有 `Deserialize`——**从文件里读回来的数字必须先从整体里分出来才能成为份额**，这正是 `PlanTree::build` 每次自根重分的理由：守恒是重新推导出来的，不是被信任的。
- **这样就解掉了自评偏差**：一个 Agent 怎么切自己那一支都行，但它切不出比父节点更多的份。**封顶取代仲裁**，也就不需要「权重从哪来」的第二权威。
- **十亿分之一而不是分数**：两份份额在任何机器上以同样方式比较与相加，反复细分不会把分母撑大，判定路径上没有浮点（ARCHITECTURE §10 规则 6）。整除余数按**先来先得**发给靠前的几份——这是一条规则而不是一次舍入，于是同一份计划在任何机器上分法相同。
- **`gather` 是自由函数而不是 `Add`**：它是「一根枝的叶子加起来是多少」这件事，不是谁都可以随手做的算术。混了两份计划的调用方会得到饱和在整份上的结果，那是诚实答案。
-/
