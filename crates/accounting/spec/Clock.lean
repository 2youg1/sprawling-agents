-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::clock

规定 `crates/accounting/src/clock.rs` 的端口 `Clock`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状由 Rust 的类型守住，`accounting::clock` 旁还没有测试。
-/

/-!
### 8-3 accounting::clock（形状 3 端口）

```rust
pub trait Clock {
    /// # Errors
    /// A clock that cannot be read, such as a wall clock set before the
    /// unix epoch.
    fn now(&self) -> Result<kernel::TimeMs, AxError>;
}
```

```rust
// bin::assembly::production（形状 4 适配器）
pub struct SystemClock;                   // 生产：墙钟，唯一被许可的采样点（clippy.toml disallowed-methods）
impl RunWorker {
    pub fn with_clock(self, clock: Arc<dyn accounting::Clock + Send + Sync>) -> RunWorker;
}
```

- **worker 读的每一个时刻都经 `RunWorker.clock`**：它写的行、它排的期限。它量的用时（派活准备、`mcp_tools`、probe 的 `elapsed_ms`）读另一只手 `Hands.monotonic`：城钟会被拨、会跳，一段时长要的是只往前走的钟（`crates/sprawling/Spec.lean` §8-129-2）。lane 线程从 `DriveContext` 拿到同一个时钟的克隆，所以一个 run 的行与 worker 自己的行读的是同一个钟。worker 调用的自由函数（`captured_until`、`reach_of`）把时刻或钟当参数收下，不自己采样。
- **worker 之外只有三个读点用 `SystemClock`**：`bin::assembly::production::hands` 把它装进 `Hands`；`bin::assembly::listening` 用它取 serve 打开账本的时刻，交给 `folds::fold_city`；`bin::serving::journal` 用它标诊断日志行的时间（那不是城的记录）。worker 的构造器打开账本、`holding` 为 `last_tick` 取起点、`worker::genesis::form` 写创世两行，读的都是交进来的 `hands.clock`，所以一个脚本钟从第一行起就生效。
- **固定值**：生产的 `Hands` 装上 `SystemClock`，测试的 `Hands`（`fixture::hands`）装上读墙钟的测试钟；构造之后 `with_clock` 是唯一换掉它的门。`Send + Sync` 与 `Arc`，是因为 lane 线程与 worker 同时读它。
- **等待也读这个钟**：lane 等 provider 的退避时，一片一片地睡，直到这个钟过了期限。所以一个永远不走的脚本钟，会让遇上退避的 run 一直等下去；脚本要让钟往前走。
-/
