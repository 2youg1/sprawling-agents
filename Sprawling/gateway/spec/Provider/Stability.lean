-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::stability

规定 `provider::stability`（`crates/gateway/src/provider/stability.rs`）：本城发出的 system prefix 两次派活逐字节相等。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::provider::stability` 旁的测试守住。
-/

/-!
### 8-19 system prefix 的稳定性是一条被守护的性质

- **性质**：同一配置下连续两次派活，发往端点的 system prefix 逐字节相等。守在 `provider::stability`，不走网络、不需要端点。
- **为什么它值一道闸**：兼容中转按整段 prompt 做缓存。**一个逐轮变化的字节就把 cache key 挪走一次**，于是一段语义上从未变过的前缀每回合重付全价。情报（使用者观察，本仓未独立复核）：某上游客户端自某版起在发往 `/v1/messages` 的每条请求的 system prompt 最前面插入一行带 `cch=` 参数的文本，每条不同；该字段其官方服务端能识别，中转不能。
- **供应层不得照抄这个字段**：它是对方服务端的计费旁路，本城既不需要也不该发。断言因此有两条——写出的请求两次逐字节相等；**上线的 system 文本与交给供应层的那几块逐字节相等，前面不多任何东西**，且不含 `cch=`。三种连接各测一遍，新增一种连接若未想过缓存，红在这里而不是红在别人的账单上。
- **本仓的前缀不带这种病。** 前缀由 `runtime::prefix` 在 Run 开始时冻结一次，四块顺序为 city／building／resident／run，最稳定的在前；`build_prefix` 与 `system_blocks()` 都是对字节的纯函数，**无时钟、无随机、无 run id 进前三块**。
- **任何新增前缀成分必须说明它为什么可以逐轮变化。** 不能说明的，就得挪到 run 块之后或根本不进前缀。
- **两种失效要分开写，否则下一个读者会把闸关掉**：
  - **设计上正确的失效**：改 effort 使缓存断点失效（官方排错文档：「switching thinking modes, changing the effort value, and changing `budget_tokens` all invalidate message cache breakpoints」）。前缀真的变了，人也真的改了配置。effort 本身并不住在 prefix 里，它是 `ChatRequest.effort` 独立字段（`accounting::worker::freezing` 冻进 `RunPlan.shape`）；工具卡片同理住 `ChatRequest.tools`。住在 prefix 里的是技能清单（resident 块的 catalog 渲染）。
  - **本条要防的意外失效**：没人决定过、也没人看得见的逐轮变化——时间戳、run id、随机序、每次请求重排的集合，以及照抄来的计费字段。
-/
