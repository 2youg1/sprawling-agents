-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::outside

规定 `crates/sprawling/src/outside.rs` 与 `crates/sprawling/src/outside/`：城外的门与控制台的 `/remote`（`bin::outside`）；逐帧授权在 `spec/Outside/Conduit.lean`。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-140 控制台的 `/remote`（`bin::outside::console`、`bin::console::language`；remote_access D4）

```rust
pub(crate) enum RemoteLine { Open(Lasting), Pair { name: String, authority: Authority }, Close, Devices,
                             Revoke(Revoking), ReplaceKey, Unreadable }
pub(crate) struct Lasting(u64);                                   // 毫秒；ms()
pub(crate) struct Remote { pub(crate) doorway: Doorway, pub(crate) reaching: Reaching }
pub(crate) fn parse(tail: &str) -> RemoteLine;                    // 纯
pub(crate) fn carry(remote: &Result<Remote, AxError>, line: RemoteLine) -> String;
// bin::console::terminal：控制台够进城的三样东西
pub(crate) struct Inside { pub(crate) desk: Arc<CommandDesk>, pub(crate) answering: Answering,
                           pub(crate) remote: Result<Remote, AxError> }
```

- **六个子动词**：`/remote open [--for <时长>]`、`/remote pair <名字> [--watch]`、`/remote close`、`/remote devices`、`/remote revoke <名字>|--all`、`/remote replace-key`。`remote` 是控制台自己的第六个动词（§8-11 的 `CONTROL`），不在线上，所以浏览器、远程设备与居民手里的任何工具都发不出它。设置页的门开关与「更换城钥匙」按钮随 `remote_access::confirm` 到来（crates/remote_access/Spec.lean §3）：页面发的开门与换钥匙只在控制台上印出一个确认码，User 把它输回页面才做，做的是这里的同一条路；配对与撤销仍只在这里（remote_access D4）。
- **时长**：一个整数加单位 `m`、`h`、`d`，至少一分钟，至多七天；不写 `--for` 是 12 小时。读不成的一行打印用法。
- **名字可以有空格**：`pair` 与 `revoke` 后面除 `--watch` 之外的词连起来就是名字，所以「客厅的 iPad」不必加引号。`--watch` 写在名字前后都行；不写就是 `act`。
- **`/remote pair` 印二维码**：邀请链接（`https://<主机名>/#pair=<配对码>&city=<指纹>`，crates/remote_access/Spec.lean §8-6）用 `qrcodegen` 以低纠错级编码，两行模块合一行字符，亮模块印成方块、暗模块印成空格，四周留两格亮边：终端是浅字深底，扫码器要深码浅底。码下面再印链接与分组的配对码，说明它十分钟内有效、只用一次。
- **`/remote open`** 印出门开多久、设备打开的地址；通路是 `PerStart` 时说明每次重启要重新配对；再照 vault 的 `Persistence` 说城的钥匙能留多久，也就是配对的设备在什么之后要重新配对：`AcrossReboots` 与 `AcrossRebootsWithPassphrase` 跨城与电脑的重启，`ThisBoot` 到这台电脑重启为止，`ThisProcess` 到城重启为止（§8-139）。这四句只说配对的后果，钥匙存在哪由 doctor 报。城没有配 `[remote]` 时印出 `crates/city/Spec.lean` §8-39 的那句拒绝：缺哪张表、两种通路各要哪几个键，去 `docs/operating.md` 看写法（§8-151）。**`/remote devices`** 每台一行：名字、权限、设备 id。**`/remote revoke`** 印出被撤销的名字，它们的会话随之结束。**`/remote replace-key`** 印出被撤销、要重新配对的名字（没有设备时说没有），并照上面的四句说新钥匙能留多久；门开着时印出那句拒绝。
- **控制台的循环拿一个 `Inside`**：发命令的桌子、问问题的那一个函数、远程门，三样合成一个值，`drive` 的参数从五个回到四个。

**测试**：`outside::tests::a_remote_line_reads_as_the_verb_it_names`（时长、带空格的中文名与 `--watch`、`--all`、`replace-key`、多出的词与读不成的时长）；`console::tests::parsing::the_control_verbs_are_the_six_it_owns`。
-/
