-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::plan

规定 `kernel::plan`（`crates/kernel/src/plan.rs` 与 `crates/kernel/src/plan/` 下的 `node`、`tree`、`share`、`blocking`）：计划树、就绪集与持有节点的两个出口。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::plan::tree::tests` 守住。
-/

/-!
### 8-33 kernel::plan（形状 1 判定）

```rust
pub struct NodeId(/* String 私有：点分十进制 */);   // 携 Serialize；Deserialize 走 parse
impl NodeId {
    pub fn parse(raw: &str) -> Result<NodeId, AxError>;
    pub fn parent(&self) -> Option<NodeId>;
    pub fn ordinal(&self) -> u32;
    pub fn depth(&self) -> usize;
    pub fn is_ancestor_of(&self, other: &NodeId) -> bool;
    pub fn ancestors(&self) -> Vec<NodeId>;
    pub fn child(&self, ordinal: u32) -> Result<NodeId, AxError>;
    pub fn as_str(&self) -> &str;
}
pub const NODE_DEPTH_MAX: usize = 10;

pub enum StopCause { Blocked { note }, HandedBack { note }, FrozeWithoutEvidence,
                     Stalled { repeats }, GateOverdue { waited_ms } }
impl StopCause { pub fn status(&self) -> RoadmapStatus; pub fn is_red(&self) -> bool; pub fn line(&self) -> String; }

pub struct Held(/* NodeId 私有 */);         // #[must_use]
impl Held {
    pub fn id(&self) -> &NodeId;
    pub fn finish(self, evidence: Locator) -> PlanExit;   // 绿
    pub fn stop(self, why: StopCause) -> PlanExit;        // 红，或回到就绪集
}
pub enum PlanExit { Finished { id, evidence }, Stopped { id, why } }

pub struct PlanNode { pub row: RoadmapRow, pub share: Share, pub children: Vec<NodeId> }
pub struct PlanTree { /* BTreeMap<NodeId, PlanNode> 私有 */ }
impl PlanTree {
    pub fn build(rows: Vec<RoadmapRow>) -> Result<PlanTree, AxError>;
    pub fn get(&self, id: &NodeId) -> Option<&PlanNode>;
    pub fn nodes(&self) -> impl Iterator<Item = &PlanNode>;
    pub fn needs_of(&self, id: &NodeId) -> BTreeSet<NodeId>;
    pub fn ready(&self) -> Vec<NodeId>;
    pub fn claim(&self, id: &NodeId) -> Result<Held, AxError>;
    pub fn progress(&self) -> Progress;
}
```

- **构造点拒五种形状**（fail-closed，与 `Locator` 同）：索引重复、父行缺失、依赖指向不存在的行、依赖自指、依赖成环。第五种用 Kahn 剥层，剥不掉的就在环上，**报成一条走法而不是一个集合**——修的人需要看见该剪哪条边。拿到 `PlanTree` 的调用方因此永远不必再问「这份计划讲不讲得通」。
- **一根枝把自己的份额整份分给子节点**，`build` 自根向下重分一次，于是**总量恒为整份计划**，不需要给分母编版本（否决「分母版本化」，因为守恒之下它在解一个不存在的问题）。
- **只有叶子进分子**。一根枝的活就是它的子节点，两边都算等于把同一份力气数两遍；`build` 因此拒绝「枝说 Done 而子节点没说」，于是枝的状态列是一句读者可信的摘要而不是第二种意见。
- **进度两个数一起走**（`PlannedProgress` 增 `done_ppb`／`blocked_ppb`）：份额说走了多少路，叶子数说这份计划最后原来有多少片。**只看份额会被慷慨的拆分骗**，只看叶子数不知道轻重；两个一起看，「先挑软柿子」的形状会自己显出来。
- **就绪集是纯函数**：叶子、无人认领、且自己与**每一层祖先**的依赖都已 Done。祖先那一条是必须的——否则 `2.3.1` 会在 `2.3` 等的那件事还没好时就开工。
- **`claim` 是拿到 `Held` 的唯一路径**，且三种拒绝各说各的：不在表里、是枝或已被拿走（`E_GOAL_CONFLICT`——两个 run 想要同一个节点就是目标冲突）、还在等什么（**点名等谁**）。第三段永远给出一个真能拿的节点。
- **计划门禁就是 `Held` 的形状**：一个私有字段使它只能由 `claim` 铸出，两个按值取走的方法使它只能花在两扇门上。**没有第三个出口**；一个只是结束了的 run 把它花在 `FrozeWithoutEvidence` 上，那正是 `blockage` 里红色的来处。`HandedBack` 是唯一不红的停：它把节点放回就绪集，而把它和「卡住」合成一个取值，要么让没人拒绝过的活搁浅，要么在有人只是没预算了的时候把计划涂红。
-/
