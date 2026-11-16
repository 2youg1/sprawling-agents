-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::watch

规定 `watch`（`crates/city/src/` 下同名的文件）。城在听什么，哪栋楼应答。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::watch` 旁的测试守住。
-/

/-!
### 8-7 city::watch（形状 6 数据面＋形状 1 判定）

```rust
pub const WATCH_FILE: &str = "WATCH.toml";
pub struct Source { /* name、matches、addr、starts_work 私有 */ }
impl Source { pub fn name(&self) -> &str; pub fn matches(&self) -> &str; pub fn addr(&self) -> &Address;
              pub fn starts_work(&self) -> bool; pub fn building(&self) -> &str; }
pub enum Link { Live { since: TimeMs }, Down { since: TimeMs } }
pub struct Watch { /* sources 私有 */ }
impl Watch {
    pub fn parse(text: &str) -> Result<Watch, AxError>;
    pub fn load(city_root: &Path) -> Result<Watch, AxError>;
    pub fn listening(&self, standing: &[Address]) -> Vec<&Source>;
}
pub fn watch_path(city_root: &Path) -> PathBuf;
```

- **形状同 `schedule`**：盘上一张表、一个纯问题、答案是派活。差别在触发源：日程因时间流逝而响，城自己看得见；watch 因别处发生了事而响，城看不见。
- **本地恒不轮询**：持连接的服务推过来，城只负责接。轮询是一份无人阅读的定时流量，也是第二份「谁先到」的权威。
- **`starts_work` 默认 false**：外来事到达本身不是花一次模型的理由。它为 true 时表示**人事先核过这个来源**——这与 `collab::triage` 的「污染件不自行开工」不冲突，两者答的是不同问题（详见 ARCHITECTURE.md §10）。
- **楼拆即不再听**：`listening` 按现存楼过滤，而不是去改用户写的文件——文件是人的，城替人改文件就是在回答一个没人问的问题。
- **两条边都入 Ledger**：`Link` 的 Live 与 Down 各携时刻，于是「这栋楼何时没在听」是可读事实而非猜测。不做断线补投。
-/
