-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::attribution

规定 `attribution`、`attribution::report`、`attribution::split`（`crates/storage/src/` 下同名的文件）。成本归因：逐维度精确分割同一总额，各维度之和对账。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-7 storage::attribution（形状 7）

```rust
pub struct Attribution { /* by_run、by_actor、by_segment、by_tool、by_skill: BTreeMap<String, UsdMicros>、
                            total: UsdMicros、pending_wave: … —— 私有 */ }
impl Attribution {
    pub fn new() -> Attribution;
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError>;
    pub fn report(&self) -> AttributionReport;
}
pub struct AttributionReport { pub total: UsdMicros, pub by_run: Vec<(String, UsdMicros)>,
    pub by_actor: Vec<(String, UsdMicros)>, pub by_segment: Vec<(String, UsdMicros)>,
    pub by_tool: Vec<(String, UsdMicros)>, pub by_skill: Vec<(String, UsdMicros)>,
    pub unpriced: Unpriced }
pub struct Unpriced { pub calls: u64, pub tokens: u64 }
```

- **五维度**（prefix 段位／SKILL／工具／子 Run／Building・Resident）。映射：`by_segment`＝prefix 段位（四段＋window 桶）；`by_skill`＝SKILL；`by_tool`＝工具；`by_run`＝子 Run；`by_actor`＝Building／Resident。
- 现状与待接点（不造假数据）：SKILL 机制属 Library，故 `by_skill` 恒入兵底桶 `no_skill`，取材契约定为 `tool_result.data.skill`（字串，权重同字节数）；派生执行面属 collab，故 `by_run` 为“每 Run 自身花费”，`run_started.parent` 链的父子归并待派生落地后接。两处均不影响 A20：兵底桶仍参与求和，五维各自恒等 total。
- 读取契约定死三条——`prompt_assembled` 携 `segments:[{slot,len}]`（两种载荷形均有此二字段）与可选 `window_bytes`（缺即不设 window 桶）；`tool_result` 携 `name` 与 `bytes`；`model_returned` 携 `billed_usd_micros`。**无权威计费额即归因零**（估算等于臆造钱，宁不报），但这次调用记入 `unpriced`：`calls` 加一，`tokens` 加上它 `usage` 的四项 token 之和（缺 `usage` 即加零）。没有它，一座只用订阅登录或本地模型的城跑了多少次都是 total 0，读者分不出「没跑」与「跑了没有报价」；token 是这时唯一量得到的用量。无权重基础即入诚实桶 `unattributed`／`no_tool`（不静默丢）。工具权重只属一波：结算即清，下一调用不继承上波。A20 除四断言外另以 256 例 proptest 钉（任意金额×权重组合均恒等）。
- 取材：`model_returned.data.billed_usd_micros`（权威计费额）；`prompt_assembled` 逐段 len；`tool_result` 的 name。每维度独立分割同一总额：by_run/by_actor 按事件归属；by_segment 按该 model_returned 所属 run 最近一条 prompt_assembled 的段 len 最大余额法分割（四段＋window 桶：入窗历史份额）——段权重按 run 分键，因为 runtime 每个 run 只写一条 prompt_assembled（其后的回合载荷不变即不再写），账本上交错的另一个 run 的 prompt_assembled 不是这次调用的基础；by_tool 按前一波 tool_result 字节最大余额法（无波则 no_tool 桶）。最大余额法使每维度和恒精确＝total（A20 的整数保证）。
- **段位基准按 run 保存，run 冻结即丢。** 一次整账本折叠会遇到城里有过的每一个 run；`run_frozen` 是终态，其后不再有该 run 的调用，所以它最近一次 `prompt_assembled` 的段位基准随之移除，常驻量只随在跑的 run 数增长，不随城的历史增长。
-/
