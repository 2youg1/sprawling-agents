-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::plan_view：写在 sprawling 规格里的那些节

本分部收着写 accounting crate 里 `accounting::plan_view` 的节。模块从 `sprawling` 搬进 `accounting` 时，写它的那一节留在本 crate 的规格里（accounting D15），标签不变；为什么暂住这里、何时搬走，见 sprawling D30。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-34 计划的投影（`accounting::plan_view`）

`PlanView` 与 `PlanReading` 住在 `accounting`，接口、理由与测试见 `crates/accounting/Spec.lean` §8-6，这里不留第二份；`views` 与 `RunWorker.plan_holders` 从那里读。哪些记录会动计划见 §8-76，锁外读盘见 §8-100。
-/
