-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::devices

规定 `answer::devices`（`crates/wire/src/` 下同名的文件）：在本地门上配对过的浏览器。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema 守住。
-/

/-!
### 8-91 配对过的浏览器：`Query::Devices` 与 `ForgetDevice`

```rust
Query::Devices                                       // 无参数
Answer::Devices(Box<DevicesAnswer>)
pub struct DevicesAnswer { pub devices: Vec<DeviceLine> }
pub struct DeviceLine {
    pub id: DeviceId,
    pub label: String,                 // 配对时页面报的名字，人用来认出它
    pub paired_at: TimeMs,
    pub last_seen: Option<TimeMs>,     // 最近一次凭它建会话的时刻；配对后还没用过就缺席
}
#[serde(transparent)] pub struct DeviceId(String);   // 配对时城给的 id 的正文
```

- **一台浏览器一行**：配对时页面在自己的源里生成一把不可导出的设备钥，城只存公钥（城的 `.sprawling/` 里），并给它一个 id；之后每次建会话都以这把钥签一次挑战（§8-93）。`ForgetDevice { device, idem }` 删掉那一行，那台浏览器下次只能重新配对。
- **`DeviceId` 只是正文**：线上的 id 是城在配对时给出的那串字，`wire` 不解析它；一个城不认识的 id 与一个拼错的 id 得到同一个拒绝，因为对 `ForgetDevice` 来说两者是同一件事：没有这台设备。
- **答复里没有公钥，也没有会话**：页面要的是认出与撤销，公钥与会话令牌对它没有用，放进答复只多一处可以被读走的地方。
- **现状**：`Devices` 在线上，城以 `Answer::Unavailable` 作答，`ForgetDevice` 以 `not_built` 作答，直到本地门的配对落地（`crates/sprawling/Spec.lean`）。
-/
