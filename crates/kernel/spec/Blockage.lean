-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::blockage

规定 `kernel::blockage`（`crates/kernel/src/blockage.rs`）：红从哪里来、波及什么。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::blockage` 旁的测试守住。
-/

/-!
### 8-34 kernel::blockage（形状 1 判定）

```rust
pub struct RedNode { pub at: NodeId, pub why: StopCause }
pub struct Blockage { pub source: NodeId, pub why: StopCause, pub reaches: Vec<NodeId> }
impl Blockage { pub fn line(&self) -> String; }
pub struct Notice { pub to: String, pub about: NodeId, pub line: String }
pub fn spread(tree: &PlanTree, red: &[RedNode]) -> Vec<Blockage>;
pub fn notices(blocked: &[Blockage], holders: &BTreeMap<NodeId, String>) -> Vec<Notice>;
```

- **红不是新机制**：来源全是这座城已经记着的事实——冻结而无证据、门升给人超期、`kernel::stall` 判定原地打转、居民自己说卡住了。本模块只答那些事实答不了的一问：**既然 2.3.1 红了，还有什么动不了。**
- **答案指名源头而不是罗列症状**：一份计划只有一个真问题时，产出是一条「整条 2.3 支线卡在 2.3.1」，而不是十七个红点让人自己往回找。首屏放得下一个原因，放不下一串后果。
- **红走两条路，而它们是同一关系的两面**：沿树向上（子节点卡住则枝卡住），沿依赖边向前（等一个红节点就是等一件不会来的事）。`reaches` 恒不含 `source`，且**已自带原因的节点不算别人的后果**——顺着列表读下来，每个问题只遇到一次。
- **`notices` 让交流由事实触发而不是由人触发**：每个 blockage 对每个持有者只发一条（一个信箱里四份同一个问题就是一个没人读的信箱），报告者不会收到自己那条。持有者是一个字符串而不是地址——kernel 不假设「谁持有」是一个地址，装配层用房间地址填它。
-/
