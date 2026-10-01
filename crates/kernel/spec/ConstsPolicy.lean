-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::consts_policy

规定 `kernel::consts_policy`（`crates/kernel/src/consts_policy.rs`）：政策常量。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-8 kernel::consts_policy

政策常量（改它须 EVAL 证据）。比值以整数对表示（kernel 判定路径禁浮点）：

```rust
pub struct Ratio { pub num: u32, pub den: u32 }   // 分子/分母；恒不约简
pub const STARTUP_BUDGET_TOKENS: u64 = 2000;
pub const CTX_REMINDER_FIRST_PERCENT: u64 = 25;                     // 上下文提醒第一道阈值（窗口百分比，恒不可调）
pub const CTX_REMINDER_SECOND_DEFAULT: u64 = 65;                     // 第二道阈值的缺省（窗口百分比）
pub const CTX_REMINDER_SECOND_MIN: u64 = 30;                         // 第二道阈值合法域下端（含）
pub const CTX_REMINDER_SECOND_MAX: u64 = 90;                         // 第二道阈值合法域上端（含）
pub const LOOP_REPEAT_THRESHOLD: u32 = 3;
pub const OFFLOAD_MIN_BYTES: u64 = 16_384;
pub const INTERVAL_CAP_BYTES: usize = 65_536;                        // 一次区间读／检索的窗口预算
pub const EXCHANGE_BUDGET_BYTES: u64 = OUTPUT_CEILING_DEFAULT * BYTES_PER_TOKEN;  // 一回合 exchange 的窗口预算（`crates/runtime/Spec.lean` §8-44）
pub const DRAFT_HELD_ESCALATE: u32 = 3;
pub const EDIT_WAR_FREEZE: u32 = 2;
pub const SECRET_ENTROPY_MIN: Ratio = Ratio { num: 7, den: 2 };      // 3.5 bits/char
pub const DISCARD_FILES_MAX: u32 = 16;
pub const DISCARD_RETENTION_DAYS: u32 = 30;
pub const CLOCK_ZONES_MAX: ClockZonesMax = ClockZonesMax::new(4);    // §8-73：拒因从类型给出
pub const WORKTREE_MAX_BYTES: u64 = 2_147_483_648;                   // 2 GiB
```

各章就近说明的常量也住本模块（`HALL_*` 三个见 §8-47）：

```rust
pub const BYTES_PER_TOKEN: u64 = 4;                                  // 字节到 token 的估算比
pub const PREFIX_SLOTS: NonZeroU64 = 4;                              // 整份 prefix 预算均分的槽数
pub const SANDBOX_FUEL_DEFAULT: u64 = 200_000_000;                   // §8-22 沙箱限额的缺省燃料
pub const CREDENTIAL_NAME_MARKERS: [&str; 11];                       // §8-22 凭据形状名字的标记词
pub const OUTPUT_CEILING_DEFAULT: u64 = 8_192;                       // messages 面输出上限梯的最后一档（token；gateway-SPEC §8-17）
pub const CLOCK_STAMP_DEFAULT: ClockStampGranularity = ClockStampGranularity::Minute;   // runtime D8
pub const AUTONOMY_DEFAULT: Autonomy = Autonomy::Owner;
pub const DEFAULT_AT: &str = "127.0.0.1:8787";                       // 服务缺省监听地址
```

`INTERVAL_CAP_BYTES` 是 `usize` 而不是 `u64`：它的读者只有 `runtime::tools::read` 与 `search`，两者都拿它比内存里一段文本的字节长度，换算因此不存在，也就没有一处可以把换算失败读成「无上限」。

**两项图片政策**：

```rust
pub const IMAGE_MAX_BYTES: ImageMaxBytes = ImageMaxBytes::new(2_097_152); // 2 MiB：一张图的字节上限
pub const IMAGES_PER_TURN: ImagesPerTurn = ImagesPerTurn::new(4);          // 一回合最多几张图
pub const IMAGE_QUALITY: ImageQuality = ImageQuality::new(100);            // 有损编码的质量域：0..=100，域外即拒
```

三项上限带 `kernel::policy_limit` 类型（§8-73）：合法域判定与四段拒因文案由类型一处给出，调用方只递观测值，拼不出第二种拒因；数的唯一家仍是本节。

两个数都是「一句拒绝说得出、一个人改得动」的上限，与 `WORKTREE_MAX_BYTES` 同口径。2 MiB 取自两家 provider 都能收下的 base64 体量（base64 膨胀 4/3，2 MiB 上线约 2.7 MiB），4 张取自一回合窗口预算：再多就是把窗口花在像素上而不是任务上。

`WORKTREE_MAX_BYTES` 是上限而非磁盘余量探测：余量是一台机器当下的事实，上限则是一句拒绝说得出、一个人改得动的数；建树前校，故一座过大的城是被拒而不是被拷到一半（`storage::worktree`）。

`AUTONOMY_DEFAULT` 与 `CLOCK_STAMP_DEFAULT` 带类型（分别是 `Autonomy` 与 `ClockStampGranularity`）；`IMAGE_MAX_BYTES`／`IMAGES_PER_TURN`／`IMAGE_QUALITY`／`CLOCK_ZONES_MAX` 带 `policy_limit` 类型（§8-73），其余为数。`SUBAGENT_CTX_LOCK_DEFAULT` 永不落地——子代理上下文锁不存在。

按 D1「默认 YOLO」，规模不改变删除的判决（§8-26），所以没有字节上限常量；`DISCARD_FILES_MAX` 的读者是 `sprawling` 的清扫阈值。
-/
