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
pub(crate) struct Lasting(u64);                                   // 毫秒；ms()；of_ms：一分钟到七天，与 read 同一条界
#[derive(Clone)] pub(crate) struct Remote { pub(crate) doorway: Doorway, pub(crate) reaching: Reaching }
pub(crate) fn parse(tail: &str) -> RemoteLine;                    // 纯
pub(crate) fn perform(remote: &Result<Remote, AxError>, line: RemoteLine) -> Result<String, AxError>;
pub(crate) fn carry(remote: &Result<Remote, AxError>, line: RemoteLine) -> String;   // perform，拒绝印成三部分
// bin::outside::asking：远程门在线上的四个动词
#[derive(Clone, Default)] pub(crate) struct Front(/* Arc<OnceLock<…>> */);
impl Front {
    pub(crate) fn commands(&self, desk: impl Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync + 'static) -> Commands;
    pub(crate) fn attend(&self, remote: &Result<Remote, AxError>, show: Show);
}
// bin::console::terminal：控制台够进城的三样东西
pub(crate) struct Inside { pub(crate) desk: Arc<CommandDesk>, pub(crate) answering: Answering,
                           pub(crate) remote: Result<Remote, AxError> }
```

- **六个子动词**：`/remote open [--for <时长>]`、`/remote pair <名字> [--watch]`、`/remote close`、`/remote devices`、`/remote revoke <名字>|--all`、`/remote replace-key`。`remote` 是控制台自己的第六个动词（§8-11 的 `CONTROL`），不在线上，所以浏览器、远程设备与居民手里的任何工具都发不出它。设置页的门开关与「更换城钥匙」按钮走线上的四个命令（下一条）；配对与撤销仍只在这里（remote_access D4）。
- **线上的四个动词**（`bin::outside::asking`，`crates/wire/spec/Command/Kind.lean` 远程门的四行）：城自己的监听把每个 Command 交给 `Front::commands`，四个门的动词在这里答，其余照旧进命令桌，所以 worker 的 `run_command` 对它们只有一臂兜底的拒绝。`OpenRemoteDoor`（`lasting_ms` 不在 `Lasting::of_ms` 的界内时以 `E_INVALID_ARGS` 拒）与 `ReplaceCityKey` 经 `remote_access::confirm::Confirm::request` 取一个码，两分钟到期，在控制台印一行：页面在请求什么（开门连同开多久）、分组的码、两分钟；答页面 `E_APPROVAL_PENDING`，恢复语说码在跑 `sprawling serve` 的终端上。`ConfirmRemoteDoor` 经 `Confirm::confirm` 取回动词，再把 `RemoteLine::Open` 或 `RemoteLine::ReplaceKey` 交给 `perform`，与控制台打出那一行做的是同一件事；`CloseRemoteDoor` 不要码，直接交 `RemoteLine::Close`。这两种在一条自己的线程上做，因为开门要等通路就绪（至多 30 秒），不能占住监听的任务；做成时控制台印出与 `/remote` 相同的那几行，拒绝经 `Reply` 回到发帧的页面，页面已经走了就印在控制台上。
- **没有控制台**：门只由带控制台的 serve 持有（`Listening::serve` 在启动控制台时 `Front::attend`），没有控制台就没有人读得到码，所以开门、换钥匙与确认都以 `E_TOOL_UNAVAILABLE` 拒，恢复语是在城自己的终端里跑 `sprawling serve`；关一扇从未开过的门答成功。三个平台相同：码印在 serve 的标准输出上，Windows 的控制台窗口、macOS 与 Linux 的终端都是它。
- **时长**：一个整数加单位 `m`、`h`、`d`，至少一分钟，至多七天；不写 `--for` 是 12 小时。读不成的一行打印用法。
- **名字可以有空格**：`pair` 与 `revoke` 后面除 `--watch` 之外的词连起来就是名字，所以「客厅的 iPad」不必加引号。`--watch` 写在名字前后都行；不写就是 `act`。
- **`/remote pair` 印二维码**：邀请链接（`https://<主机名>/#pair=<配对码>&city=<指纹>`，crates/remote_access/Spec.lean §8-6）用 `qrcodegen` 以低纠错级编码，两行模块合一行字符，亮模块印成方块、暗模块印成空格，四周留两格亮边：终端是浅字深底，扫码器要深码浅底。码下面再印链接与分组的配对码，说明它十分钟内有效、只用一次。
- **`/remote open`** 印出门开多久、设备打开的地址；通路是 `PerStart` 时说明每次重启要重新配对；再照 vault 的 `Persistence` 说城的钥匙能留多久，也就是配对的设备在什么之后要重新配对：`AcrossReboots` 与 `AcrossRebootsWithPassphrase` 跨城与电脑的重启，`ThisBoot` 到这台电脑重启为止，`ThisProcess` 到城重启为止（§8-139）。这四句只说配对的后果，钥匙存在哪由 doctor 报。城没有配 `[remote]` 时印出 `crates/city/Spec.lean` §8-39 的那句拒绝：缺哪张表、两种通路各要哪几个键，去 `docs/operating.md` 看写法（§8-151）。**`/remote devices`** 每台一行：名字、权限、设备 id。**`/remote revoke`** 印出被撤销的名字，它们的会话随之结束。**`/remote replace-key`** 印出被撤销、要重新配对的名字（没有设备时说没有），并照上面的四句说新钥匙能留多久；门开着时印出那句拒绝。
- **控制台的循环拿一个 `Inside`**：发命令的桌子、问问题的那一个函数、远程门，三样合成一个值，`drive` 的参数从五个回到四个。

**测试**：`crates/sprawling/tests/remote_door.rs` 的 `a_page_opens_the_door_only_with_the_code_the_console_printed`（请求答 `E_APPROVAL_PENDING`、错码与错码之后的对码都被拒、新请求的码开门、关门不要码，Ledger 里是 `remote_opened` 与 `remote_closed`；到期由 `remote_access::confirm` 的迹向量判）；`outside::tests::a_remote_line_reads_as_the_verb_it_names`（时长、带空格的中文名与 `--watch`、`--all`、`replace-key`、多出的词与读不成的时长）；`console::tests::parsing::the_control_verbs_are_the_six_it_owns`。
-/
