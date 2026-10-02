-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::reminder

规定 `reminder`（`crates/runtime/src/` 下同名的文件）。窗口满到什么程度，按 provider 自己的计数提醒。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-34 runtime::reminder（形状 1 判定）


**两道阈值**，以 provider 报回的 `input_tokens`（事实）对模型的 `context_tokens`（`CallShape::context_tokens`，来自 endpoint 簿）计算；**恒不用窗口字节数**（那是估计）。

| 阈值 | 说的话 |
|---|---|
| 30%（`CTX_REMINDER_FIRST_PERCENT`，恒不可调；`crates/kernel/Spec.lean` D19） | 只报用量：`[context] 30% of the window used (N of M input tokens).` |
| 第二道：缺省 65%（`CTX_REMINDER_SECOND_DEFAULT`），合法域 31–90、可由配置梯子调（`crates/kernel/Spec.lean` §8-22） | 报用量，并说明剩余预算仍够写 handoff 并 `succeed`，过了这一点就不够了 |

**每道阈值一跑恰响一次**。状态是穷尽枚举 `Sounded { Nothing, First, Handover }` 而不是两个布尔；一跳越过两道（0→70%）时只响高的那一道，低的一并作废——两句话叠在一起是噪声。

```rust
pub struct ContextGauge { window: Tokens, second_at: u64, sounded: Sounded }
pub struct ContextReading(Arc<AtomicU64>);   // Clone＋Default；Run 写、status 读的同一格
impl ContextReading { pub fn tokens(&self) -> Tokens; pub(crate) fn record(&self, used: Tokens); }
impl ContextGauge { pub fn new(window: Tokens, second: Option<SecondThreshold>) -> ContextGauge; pub fn observe(&mut self, used: Tokens) -> Option<ContextReminder>; }
pub enum ContextReminder { Usage { used: Tokens, window: Tokens }, HandoverWindow { used: Tokens, window: Tokens } }
impl ContextReminder { pub fn render(&self) -> String; }
```

`second` 来自 `RunPlan.second_threshold`：配置梯子冻结的值，`None`＝没有一层说话，取 `CTX_REMINDER_SECOND_DEFAULT`，「缺席取默认」只在这一个构造点判定。`window == 0`（簿上没写）恒不响：没有分母就没有百分比，与 `UnplannedProgress` 同一条理。整数算术：`used * 100 / window` 用 checked 乘法。

**接线**：`TurnReport` 增 `usage: Option<ModelUsage>`；`RunPlan` 增 `second_threshold: Option<SecondThreshold>`（Run 起点冻结，理由住 `crates/kernel/Spec.lean` §8-22）；`RunPlan` 增 `context: ContextReading`，Run 在每回合观察之前把同一个 `input_tokens` 记进去，装配层把同一格交给 `StatusTool::metering`——只有一处写，窗口提醒与 `status` 读的是同一个数；`Run<Active>` 持 `ContextGauge`，每回合以 `usage.input_tokens` 观察，响则以 `Conversation::push_reminder` 落在该回合工具结果之后——与 steer 同一扇门，所以它「落在下一次工具结果的尾部」。`pipeline::PackContext` 同时增 `reminder: Option<ContextReminder>` 作第四个附件，句子只在 `ContextReminder::render` 一处定义。

**改这一格的入口**：`wire::Command::ConfigureBuilding` 的 `context_second_threshold`（`crates/wire/Spec.lean` §8-45）写的就是 `RunPlan.second_threshold` 读的那一格——写入落那一级的 `[context] second_threshold`，下一个 Run 起点冻结时读到；正在跑的那个 Run 不受影响（冻结的理由见 `crates/kernel/Spec.lean` §8-22）。
-/
