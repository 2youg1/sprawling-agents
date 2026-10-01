-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::node_id

规定 `kernel::node_id`（`crates/kernel/src/node_id.rs`）：计划节点的地址。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-48 `kernel::node_id`：计划节点的地址（形状 2 值）

`kernel::plan` 的模块表行是 `decision`：树判定什么可以开工、一个枝值多少、一个持有节点走两个出口里的哪一个。
`NodeId` 不判定任何事——它只说清「一个良构地址长什么样」，并在**唯一的构造点** `parse` 把别的一律拒掉，
好让下游没有一处需要再问一遍。那是 `value`，`crates/kernel/Spec.lean` §9 的形状 2，所以它住自己的模块 `kernel::node_id`，公开路径是 `kernel::NodeId`。

**`Deserialize` 是手写的，走 `parse`**，这也是这个类型存在的理由：
derive 出来的那个会把线上任意字符串收下、交回一个从没过过 `parse` 的 `NodeId`。
-/
