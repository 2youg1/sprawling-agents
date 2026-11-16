-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::retries

规定 `kernel::retries`（`crates/kernel/src/retries.rs`）：失败的调用再试几次。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::retries` 旁的测试守住。
-/

/-!
### 8-72 `kernel::retries`：失败的调用再试几次（形状 2 值类型）

**两个读者，谁也不依赖谁。** gateway 判断要不要再发一次请求，runtime 的 `Watchdog` 判断要不要冻结这次运行；两个 crate 之间没有依赖边，所以这个事实只能住在它们共同的下层。

**缺席是一个答案而不是一个空位。** `UntilHalted` 是没有填这个数的人要的东西——停下它的是 `Halt`，这座城唯一的刹车。`AtMost(0)` 是另一个答案，意思是「跑一次然后报告」。用一个带哨兵的整数，这两件事就会被同一个 `0` 说出来。

**看着设置页等一个探测的人手里没有刹车**，所以 `without_a_brake` 把 `UntilHalted` 读成一次重试，理由写在那个方法上，而不是散在每个调用点。

**线上与账本里的那个数只有一种读法**：`stated()` 对 `UntilHalted` 答 `None`、对 `AtMost(n)` 答 `Some(n)`；线上缺键与账本缺键是同一个事实，所以两边都经它写。
-/
