-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::attribution

规定 `attribution`、`attribution::report`、`attribution::split`（`crates/storage/src/` 下同名的文件）。成本归因：逐维度精确分割同一总额，各维度之和对账。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与模块旁的测试守住（`crates/storage/Spec.lean` §16）。
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
    pub unpriced: Unpriced, pub renewals: Renewals }
pub struct Unpriced { pub calls: u64, pub tokens: u64 }
/// 账本上每一行 cache_renewed，按 `CacheRenewed` 的 `outcome` 分到四格之一（D35）。
pub struct Renewals { pub read: u64, pub late: u64, pub unreported: u64, pub refused: u64 }
```

- **五维度**（prefix 段位／SKILL／工具／子 Run／Building・Resident）。映射：`by_segment`＝prefix 段位（四段＋window 桶）；`by_skill`＝SKILL；`by_tool`＝工具；`by_run`＝子 Run；`by_actor`＝Building／Resident。
- 现状与待接点（不造假数据）：SKILL 机制属 Library，故 `by_skill` 恒入兵底桶 `no_skill`，取材契约定为 `tool_result.data.skill`（字串，权重同字节数）；派生执行面属 collab，故 `by_run` 为“每 Run 自身花费”，`run_started.parent` 链的父子归并待派生落地后接。两处均不影响 A20：兵底桶仍参与求和，五维各自恒等 total。
- 读取契约定死三条——`prompt_assembled` 携 `segments:[{slot,len}]`（两种载荷形均有此二字段）与可选 `window_bytes`（缺即不设 window 桶）；`tool_result` 携 `name` 与 `bytes`；`model_returned` 携 `billed_usd_micros`。**无权威计费额即归因零**（估算等于臆造钱，宁不报），但这次调用记入 `unpriced`：`calls` 加一，`tokens` 加上它 `usage` 的四项 token 之和（缺 `usage` 即加零）。没有它，一座只用订阅登录或本地模型的城跑了多少次都是 total 0，读者分不出「没跑」与「跑了没有报价」；token 是这时唯一量得到的用量。无权重基础即入诚实桶 `unattributed`／`no_tool`（不静默丢）。工具权重只属一波：结算即清，下一调用不继承上波。A20 除四断言外另以 256 例 proptest 钉（任意金额×权重组合均恒等）。
- 取材：`model_returned.data.billed_usd_micros`（权威计费额）；`prompt_assembled` 逐段 len；`tool_result` 的 name。每维度独立分割同一总额：by_run/by_actor 按事件归属；by_segment 按该 model_returned 所属 run 最近一条 prompt_assembled 的段 len 最大余额法分割（四段＋window 桶：入窗历史份额）——段权重按 run 分键，因为 runtime 每个 run 只写一条 prompt_assembled（其后的回合载荷不变即不再写），账本上交错的另一个 run 的 prompt_assembled 不是这次调用的基础；by_tool 按前一波 tool_result 字节最大余额法（无波则 no_tool 桶）。最大余额法使每维度和恒精确＝total（A20 的整数保证）。
- **保温续期单独报数**（D35）：每一行 `cache_renewed` 按 kernel 里 `CacheRenewed` 的 `outcome` 落进 `renewals` 的一格——`read`（provider 读到了缓存）、`late`（usage 报出非零的 `cache_write_tokens`：续期到得太晚，前缀被重写）、`unreported`（没报写入数，或这一行读不成 `CacheRenewed`）、`refused`（provider 拒了）。它不动 `total` 与五个维度：续期的花费怎样进总额是另一件事，这里只让成本视图读得到续了几次、几次迟到。
- **段位基准按 run 保存，run 冻结即丢。** 一次整账本折叠会遇到城里有过的每一个 run；`run_frozen` 是终态，其后不再有该 run 的调用，所以它最近一次 `prompt_assembled` 的段位基准随之移除，常驻量只随在跑的 run 数增长，不随城的历史增长。
-/

/-! D35 保温续期在归因里单独报数，按 kernel 的一条读法分四格

**决定**：`Attribution` 折叠 `cache_renewed`，把每一行按 `kernel::event::record::CacheRenewed` 的 `outcome` 计进 `Renewals { read, late, unreported, refused }`，随 `AttributionReport` 交给成本视图；`total` 与五个维度不动。读不成 `CacheRenewed` 的行计进 `unreported`：续期确实发生过，只是它的用量这个构建读不出，这正是「没报」。

**理由**：迟到的续期不报错，只多花钱——按写价重建前缀而不是按读价刷新；在这之前城里没有任何地方把它浮现出来，只能去读 `cache_renewed` 的 usage 字段。「什么算迟到」只在 kernel 定义一处，归因只数格子；成本视图已经从 `AttributionReport` 读 `unpriced`，续期的次数放在同一份报告里，视图不必再折一遍账本。

**被否**：在归因里按 JSON 键自己判 `cache_write_tokens`——那是「迟到」的第二份定义；读不成的行让 `apply` 失败——一行坏掉的续期会让整个成本视图答不出来；只数迟到、不数总次数——没有分母，人读不出迟到是常态还是偶发。

**重开参数**：续期花费进入房间总额时（单列一行），重议这四格是否随它一起按房间分。
-/
