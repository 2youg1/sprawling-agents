-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::effect：写在 sprawling 规格里的那些节

本分部收着写 accounting crate 里 `accounting::effect` 的节。模块从 `sprawling` 搬进 `accounting` 时，写它的那一节留在本 crate 的规格里（accounting D15），标签不变；为什么暂住这里、何时搬走，见 sprawling D30。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `accounting::effect` 旁的测试守住。
-/

/-!
## 8-24 一条效应先成为账本行，再成为这座城（`accounting::effect`）

`accounting::effect` 的接口、理由与测试住 `crates/accounting/Spec.lean` §8-5，这里不留第二份。装配层那一扇门是 `RunWorker::settle(&mut self, at: &Assignment, run: RunId, landing: effect::Landing, chain: &KnockChain)`（`accounting::worker::settling::landing`）：每张桌子交出的 `Landing` 都走它，`record` 先落完行，再把 `Then` 交给一个穷尽的 match。
-/

/-!
## 8-30 合并也排到它那条行后面

§8-24 把五张桌子搬进 `accounting::effect` 时，把 `PrEffect::Merged` 留在原地，理由写得很清楚：`trees.merge` 确实先动世界，但「先落账在这里更坏」——`merge` 有一条可达的失败臂 `MergeStale`，先落账就是把一句谎写进历史里的可达路径。它同时写下了解法：`storage::Worktrees` 得先能答「这一合并会落在哪个 commit」且能先验干线。这里做的就是那一条（`crates/storage/Spec.lean` §8-2），于是两头不再互斥：

```rust
let planned = trees.plan_merge(&name)?;    // 全部拒绝在此，世界未动
record_for(…, EventKind::PrMerged, … planned.commit() …)?;   // 行
planned.apply()?;                           // 才是变化
```

于是：一个会被拒的合并永远不会先得到一条行（`MergeStale` 早于落账）；一条没落下的行也永远不会已经改了干线（`apply` 需要一个只能从 `plan_merge` 拿到的值，而行写在它之前）。

**红**：`a_merge_the_history_refused_leaves_the_building_where_it_was`。一个 `review = true` 的楼，一跑改文并提交请求，第二跑去检——而第二跑的账本是 `open_faulty`（§8-29 的工具）且 `cut_on_write: Some("pr_merged")`。断言：楼里那份文件仍是 `before`。**改动之前它是 `after`**：干线已经移了，而宣布它的那一行从未落地——一座楼站在它自己的历史说从来没有并入过的工作上。

**影面**：`storage` 公开面去 `Worktrees::merge`、增 `plan_merge` 与 `PlannedMerge`（基线与 storage 的规格同提交）；四个读写方全部迁完后旧入口删除，不留适配。`sprawling` 公开面不变。
-/
