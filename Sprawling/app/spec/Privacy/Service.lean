-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# privacy 页面的服务：答复、操作与结果
规定 `bin::privacy::service`、`bin::privacy::answer`、`bin::privacy::system` 与装配层的
`bin::assembly::privacy`。本文件是接口说明，不是形式证明：写入次序与它的性质由
crates.sprawling.spec.Privacy 证明，服务只把页面的一次询问或一个命令交给同一个
`bin::privacy::coordinator`；线上的形状归 `crates/wire/spec/Privacy.lean` §8-88、§8-89。

## 装配层的拦截（`bin::assembly::privacy`）
城的监听收到的 `Query::Privacy` 由服务作答，其余询问照旧由视图作答；`Command::PrivacyOperation`
由服务接下，其余命令照旧交给远程门的前台与命令台。视图与 run worker 不读写主机设置：它们对这两帧
各答一句「由城的监听作答」（accounting 的 answering 与 routing），只在跳过了拦截的路径上出现。
控制台与监听共用同一个 answering，所以终端里的 `Query::Privacy` 也由服务作答。
远程设备的 `Ask { query: Privacy }` 与 `PrivacyOperation` 在中继按 `LocalOnly` 拒绝，不到城
（crates/sprawling/spec/Outside/Conduit.lean）。

## 答复（`Service::answer`，组装在 `bin::privacy::answer`）
次序：结果表 → 主机事实 → 历史 → 每个控制的当前值 → 组装（D72）。
- 主机不是 Windows：`host` 为 `NotWindows`，不读历史之外的任何主机值，每个控制 `current` 为 `NotRead`。
- 历史用 `journal::read` 读，不取写锁、不建文件；读不成（被另一个操作占着、损坏）答
  `PrivacyHistory::Unreadable`，答复的其余部分照常给出。
- 空历史没有 owner，答空的 `Disclosed`，不取样身份、不问 Vault。非空历史先取样实时身份，再经 Host 的
  owner 核对它与记录的 owner 引用，通过后 `History::holdings` 才交出每个控制的拥有栈栈顶与未结操作
  （读者形式，owner 为 `()`，Privacy.State），答 `Disclosed`；核对拒绝或做不成答 `Withheld`，答复里没有任何记录下的值（Privacy.Cli D54 的同一边界）。
- 每个控制读一次目标：读到值、访问拒绝或其他失败；`written` 是控制表的写入值在当前值之上会留下的值。
- 组装只读控制表（`bin::privacy::controls`、`bin::privacy::originals`），无 IO：版本适用由
  `Editions::fit` 与主机的版本一处算出（Privacy.Controls D67），快照与线上的值互换由
  `bin::privacy::target` 一处定义（wire D50），换回后再换出不等于原拼写的值拒绝。

## 命令（`Service::accept` 与 `Service::perform`）
- `accept` 在监听的任务上同步完成：`expected` 换不回快照即以 `E_INVALID_ARGS` 拒绝，不记结果；
  idem 已接过则什么也不做；否则记一条 `Running` 并交出一个待执行的操作。
- `perform` 在一个专为这次操作起的线程上执行（ARCHITECTURE.md §10 第 3 条的线程清单），先取服务的操作锁，
  所以操作一个接一个：每次开一个 `LockedJournal`，经 coordinator 运行，结束后把该 idem 的结果改为
  终态（`Applied`、`Restored`、`AlreadyWritten`、`Reconciled` 或带稳定码与 AxError 的 `Refused`）。
-/

/-! D70 结果按 idem 留在服务里，页面经下一次询问取回；同一 idem 只执行一次
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

/-! D71 页面与 CLI 写入动词经同一个生产 Host；不是 Windows 的主机没有隐私控制
`bin::privacy::system` 选出进程所在的主机：Windows 上是 `bin::privacy::windows::host` 的生产 Host（身份、owner 的
核对与绑定、按控制路由到适配器、机器作用域经提升子进程，Privacy.Cli 与 Privacy.Windows），时钟是
`bin::assembly` 的 SystemClock，主机事实由 `bin::privacy::windows` 读版本记录（读不成答 `Unreadable`）；
其他平台是一台没有隐私控制的主机：答 `NotWindows`，不读任何目标，每个操作以 `ToolUnavailable`
「隐私控制是 Windows 的设置」拒绝，这句拒绝由 `bin::privacy::system` 一处给出，CLI 也用它。
**理由**：页面与 CLI 走同一个 coordinator，也走同一个 Host，所以一条控制在两处的读写、拒绝与恢复相同；
平台的选择只在一处。
**被否**：服务自建一个 Host——同一台机器有两种读写路径，验收过的那一种（一次性 runner 上的 CLI）
不是页面用的那一种。
**重开参数**：macOS 或 Linux 有了隐私控制时，`bin::privacy::system` 为该平台选它的 Host。
-/

/-! D72 答复先取结果表，再读历史与当前值，所以答复里的结论不早于它的历史
一次操作先把结论写进隐私日志，释放日志锁，才把该 idem 的结果改为终态；答复先抄下结果表，再读历史、
再读每个控制，所以答复里一个结果是终态时，同一答复的历史已含它的结论、当前值已是它之后的值。结果表
仍是 `Running` 而历史已含结论的答复无害：页面看到自己的 idem 仍在运行，会再问一次。
**理由**：页面只在自己的 idem 有了终态的那一次答复上停止再问，并按这同一次答复画出拥有与按钮；结果表
若比历史新，页面会停在一份还没有这次 apply 的历史上——显示 applied，却说「本应用未改过」、不给 restore，
直到人自己刷新。非空历史要先核对身份，核对要几秒，一次只写本账户设置的 apply 足以在这段时间里做完。
**被否**：①把整次答复放在服务的操作锁里——答复读每个控制要二三十秒，操作会等着答复，确认后的写入被
无关的一次读拖慢；②页面在结论到达后再多问一次——修的只是一个读者，答复本身仍可自相矛盾，
每个读者都得各自知道要再问。
**重开参数**：结果表改从隐私日志的结论读（D70 的重开参数）时，结果与历史出自同一次读，这条次序随之取消。
-/
