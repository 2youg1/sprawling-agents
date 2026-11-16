-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::history

规定 `history`（`crates/city/src/` 下同名的文件）。这个目录有没有城的历史。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`city::history` 旁还没有测试。
-/

/-!
### 8-30 这个目录有没有城的历史（`city::history`，形状 1 判定）

**接口**：`pub enum History { Absent, Present }`；`pub fn has_history(city_root: &Path) -> Result<History, AxError>`。账本目录由 `kernel::layout::CityLayout::ledger` 回答；目录不存在或为空是 `Absent`，有一个条目是 `Present`；目录在却列不出来是 `StorageFatal`，恢复提示「让账本目录可读，或换一个城目录」——把列不出来当 `Absent` 会让 `init` 在一座只是读不到的城上再写一次创世。

**决定**：这件事住在 `city`，与其余读城在盘上布局的函数（`buildings`、`survey`）同层。`init` 与 `up` 在装配点问它，doctor 也问它；doctor 是读面，读面不依赖装配点（`crates/sprawling/Spec.lean` §8-92），而 `city` 是 doctor 本来就依赖的一层。**败给的方案**：放进 `kernel::layout`——kernel 不做文件 I/O；放进 `storage`——`storage` 读的是账本的行，这里只问目录里有没有东西，而问它的三个地方都已依赖 `city`。
-/
