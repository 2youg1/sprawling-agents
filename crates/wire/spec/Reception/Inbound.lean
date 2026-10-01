-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception::inbound

规定 `reception::inbound`（`crates/wire/src/` 下同名的文件）。读一帧文本，以及读不出的那一帧带计数的拒绝。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-37 读不出的那一帧不是一次断线（`reception::inbound`）

**规则**：一帧 JSON 在任一侧解不出，就是两端对 wire 的分歧，判 `E_WIRE_MISMATCH`；**恒不把它表达为一次连接断开**。

- **服务端**（`wire::reception::inbound`，形状 1 判定）：`Inbound::read` 是这一侧唯一的入站解码点；解不出即计数一次并产 `SessionStep::Refuse { close: false }`，subject 写「这是本会话第几帧读不出」与本服务端说的 `WIRE_V`，恒不回显对端字节（一个 peer 的帧是它自己的内容，抄进日志就带走了它携的东西）。壳（`server::socket`）因此对「读不出」与「读得出后判出的拒绝」走同一条分支，两种拒绝只有一条送达路径。
- **客户端**（`client/src/core/link.ts`）：`LinkEvent::undecodable` 与「服务端送来的 `E_WIRE_MISMATCH` 拒绝帧」都把链路置为 `refused`，梯子停摆；`core/socket.ts` 据 `isRefused` 取消已排期的那次重连并丢掉这条 socket。措辞随 `AxError` 的三段走（action／subject／recovery），与握手期的版本不匹配同一渲染处（`views/refusal.svelte`），故人看到的是「刷新页面取这台服务端配的客户端」加一个重试按钮。
- **退避计数的清零点是「收到一帧数据」，不是「握手成功」**。握手完成即清零时，一条「连上、打招呼、还没说话就断」的 socket 让梯子永远停在第一级 250 ms——一个对人不可见、也长不大的热循环。

**为什么服务端不关连接，而握手期仍然关**（§10「握手的失败处置取断连」不变）：握手期页面还没上来，断连就是它唯一能读到的信号，而它读到的正确；握手之后页面**已经把断连读成网络故障**并按梯子重连，于是同一帧在下一条 socket 上重现，人得到一张永远在重连的空白页。告诉它一次，它就停一次。

**计数的用途**：让「同一个旧页面反复撞同一堵墙」在人看到的那条拒绝里可见。它不改变行为——不设阈值、到点不关连接：阈值只会把刚被移除的那条静默断线路径原样请回来。

**被否**：①继续以关连接表达解码失败——这正是 `WIRE_V` 31→32 时旧页面会撞上的路径；②给不可读帧设上限、超限即关——参见上一条的理由；③把解码失败降级为「忽略这一帧」——两端的分歧不会因为丢掉一帧而消失，而下一帧照样读不出。
-/
