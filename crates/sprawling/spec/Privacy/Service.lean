-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy 页面的服务：答复、操作与结果
规定 `bin::privacy::service`、`bin::privacy::answer`、`bin::privacy::system` 与装配层的
`bin::assembly::privacy`。本文件是接口说明，不是形式证明：写入次序与它的性质由
crates.sprawling.spec.Privacy 证明，服务只把页面的一次询问或一个命令交给同一个
`bin::privacy::coordinator`；线上的形状归 `crates/wire/spec/Privacy.lean` §8-86、§8-87。

## 装配层的拦截（`bin::assembly::privacy`）
城的监听收到的 `Query::Privacy` 由服务作答，其余询问照旧由视图作答；`Command::PrivacyOperation`
由服务接下，其余命令照旧交给远程门的前台与命令台。视图与 run worker 不读写主机设置：它们对这两帧
各答一句「由城的监听作答」（accounting 的 answering 与 routing），只在跳过了拦截的路径上出现。
控制台与监听共用同一个 answering，所以终端里的 `Query::Privacy` 也由服务作答。
远程设备的 `Ask { query: Privacy }` 与 `PrivacyOperation` 在中继按 `LocalOnly` 拒绝，不到城
（crates/sprawling/spec/Outside/Conduit.lean）。

## 答复（`Service::answer`，组装在 `bin::privacy::answer`）
次序：主机事实 → 历史 → 每个控制的当前值 → 组装。
- 主机不是 Windows：`host` 为 `NotWindows`，不读历史之外的任何主机值，每个控制 `current` 为 `NotRead`。
- 历史用 `journal::read` 读，不取写锁、不建文件；读不成（被另一个操作占着、损坏）答
  `PrivacyHistory::Unreadable`，答复的其余部分照常给出。
- 空历史没有 owner，答 `Empty`，不取样身份、不问 Vault。非空历史先取样实时身份，再经 Host 的 owner
  核对它与记录的 owner 引用，通过后才从 `History::standing` 取出每个控制的拥有栈栈顶与未结操作，
  答 `Disclosed`；核对拒绝或做不成答 `Withheld`，答复里没有任何记录下的值（Privacy.Cli D53 的同一边界）。
- 每个控制读一次目标：读到值、访问拒绝或其他失败；`written` 是控制表的写入值在当前值之上会留下的值。
- 组装只读控制表（`bin::privacy::controls`、`bin::privacy::originals`），无 IO：版本适用由
  `Editions::fit` 与主机的版本一处算出（Privacy.Controls D64），快照与线上的值互换由
  `bin::privacy::target` 一处定义（wire D49），换回后再换出不等于原拼写的值拒绝。

## 命令（`Service::accept` 与 `Service::perform`）
- `accept` 在监听的任务上同步完成：`expected` 换不回快照即以 `E_INVALID_ARGS` 拒绝，不记结果；
  idem 已接过则什么也不做；否则记一条 `Running` 并交出一个待执行的操作。
- `perform` 在一个专为这次操作起的线程上执行（ARCHITECTURE.md §10 第 3 条的线程清单），先取服务的操作锁，
  所以操作一个接一个：每次开一个 `LockedJournal`，经 coordinator 运行，结束后把该 idem 的结果改为
  终态（`Applied`、`Restored`、`AlreadyWritten`、`Reconciled` 或带稳定码与 AxError 的 `Refused`）。
-/

/-! D67 结果按 idem 留在服务里，页面经下一次询问取回；同一 idem 只执行一次
接下命令时记 `Running`，结束时改为终态；答复带最近 64 条结果，最旧的先让出。接过的 idem 全部记住
（只记键），所以一个结果已让出的 idem 再到也不再执行，以 `E_INVALID_ARGS` 拒绝并说它已执行过。
**理由**：一次操作可能等管理员批准几分钟，命令的答复通道（`Reply`）只送拒绝，页面不能挂在它上面等；
按 idem 取回让每个页面只认自己发出的操作。只记键的集合每次点击长一个键，城一次运行里人能点的次数
让它保持很小；结果本身带 AxError 与值，所以有界。
**被否**：①写进城的 Ledger——主机隐私不进入城市重放（Privacy §7），而且 Ledger 是城的历史，不是
主机的；②结果只按 64 条记、过了界就当新 idem——过了界的重放会对一个已经变了的主机再执行一次；
③页面读全局拒绝帧——别的命令的拒绝会改写隐私页的状态。
**重开参数**：页面需要跨城重启取回结果时，改从隐私日志的结论读，而不是在服务里留得更久。
-/

/-! D68 生产主机现状：只有身份、owner 核对与时钟是真的
`bin::privacy::system` 是服务与 CLI 写入动词共用的生产 Host：时钟是 `bin::assembly` 的 SystemClock，
身份由 `bin::privacy::identity` 取样，记录的 owner 由 `gateway::verify_platform_identity` 核对。
目标的读写与主机版本事实经 Windows 适配器（Privacy.Windows），它们接到这里之前，读与写以
`ToolUnavailable` 拒绝、版本事实答 `Unreadable`，页面如实显示每个控制读不成；空历史的第一次写入
要为实时身份建立 owner 绑定，gateway 还没有这个写入者，所以它以 `ToolUnavailable` 拒绝，什么也不写。
非 Windows 主机答 `NotWindows`，读写以「仅 Windows」拒绝。
**理由**：服务与装配在适配器之前就能接上并验收（远程拒绝、身份不符时不披露）；拒绝带着动作与恢复，
不会被读成「主机没有这个值」。
**被否**：适配器到位前不接装配——页面与 CLI 各自要再接一次，接线的验收也要再做一次。
**重开参数**：Windows 适配器与 owner 绑定的写入者落地时，本条改写为它们的路由，拒绝随之删除。
-/
