-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::console

规定 `crates/sprawling/src/console.rs` 与 `crates/sprawling/src/console/`：服务中的那个终端（`bin::console`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格。生命周期的状态机与它的性质在 `crates/sprawling/spec/Console/Lifecycle.lean`，那里的转移向量由 `bin::console::lifecycle` 的测试逐行回放；报告给终端的程序状态（OSC 7501）的模型在 `crates/sprawling/spec/Console/ProgramStatus.lean`，由 `bin::console::program_status` 的测试回放；其余接口形状与取舍由 Rust 的类型与 `bin::console::tests::helpers`、`bin::console::tests::parsing`、`bin::console::tests::terminal` 守住。
-/

/-!
## 8-11 控制台：CLI 与安静宿主

**原因**：一座服务中的城有两个面给人：浏览器，和起城的那个终端。终端这一面要同时满足两类人：终端爱好者要一个能直接派活、读答复、批准请求的 CLI；开了浏览器的人要终端安静，只留地址和配对码，免得浏览器误关以后找不回去，也免得事件行和诊断行一直刷屏。两者是同一个控制台的两个面（**console**：CLI 与 **quiet host**），由一个生命周期状态机决定此刻是哪一面。

**三扇门落到哪一面**：

| 起法 | 开城后的面 |
|---|---|
| `sprawling`（无参，§8-8） | `Cli` |
| `sprawling up` 与发行包里的启动器 | `QuietHost`，并打开浏览器 |
| `sprawling serve`，带 `--console` | `Cli` |
| `sprawling serve`，不带 `--console` 或带 `--no-console` | `Headless` |

标准输入或标准输出不是终端时（管道、服务、CI、测试工装），任何起法都落到 `Headless`；带了 `--console` 时 `Headless` 仍读标准输入的整行，逐行答在标准输出上（**行控制台**），供工装驱动，`remote_door` 的端到端测试就是这样驱动 `/remote` 的。行控制台读到 EOF 只关控制台，城照跑。

**生命周期**（`bin::console::lifecycle`，形状 2 state machine，模型与性质见 `Console/Lifecycle.lean`）：

```rust
pub(crate) enum Face { Opening, Cli, QuietHost, Headless, Stopping { mode: wire::CloseMode, sinks: Sinks }, Gone(Handoff) }
pub(crate) enum Sinks { Held, Cut }
pub(crate) const INTERRUPT_GRACE: Duration = Duration::from_secs(4);
pub(crate) enum Handoff { Written, Skipped }
pub(crate) enum Surface { Cli, QuietHost, Headless }
pub(crate) enum Asker { Console, Page }
pub(crate) enum Event { Ready(Surface), Web, Back, Quit(Asker, wire::CloseMode), InterruptSignal, BreakSignal, Terminate, TerminalLost, Failed, Landed, Deadline }
pub(crate) enum Cause { Closed(accounting::worker::ClosedBy), Failed }
pub(crate) fn step(face: Face, event: Event) -> (Face, Option<Cause>);
```

- 只有停城请求进入 `Stopping`：CLI 的 `/quit`、同一台电脑上的页面发来的 `CloseCity`、信号、终端没了、`wire::serve` 失败。进入的那一步给出缘由，`CommandDesk::close` 记下它；先到的缘由作数。
- **键与信号是两件事**。CLI 与安静宿主在 raw 模式里读键，敲出来的 Ctrl+C 是一个键，不是信号，城只在每个会话里第一次收到它时印一行「复制请先选中文字；关闭城市用 /quit」，此外什么都不做；Ctrl+C 与 Ctrl+V 归终端（有选区时复制、粘贴）。信号形式的 `SIGINT`（Windows 的 `CTRL_C_EVENT`）在交互面上面不变，在 `Opening` 与 `Headless` 里是 `Stopping(Drain)`，工装与服务管理器靠它。Windows 的 Ctrl+Break 始终是信号，处处是 `Stopping(Drain)`；`SIGTERM` 处处是 `Stopping(Interrupt)`，与其他 interrupt 一样带时限。
- **每一次 interrupt 都有时限**（`Console/Lifecycle.lean` D75）：面进入 `Stopping(Interrupt)` 的那一步，外壳装上 `INTERRUPT_GRACE`（4 s），不论这次 interrupt 来自 `/quit` 后的 n、页面的「立刻停下并关闭」、`SIGTERM`、收口中的第二次请求还是终端没了。进行中的 provider 调用不是安全点，没有时限的 interrupt 要等到调用超时；时限到了，进程不写交接退出，留下的 run 由下一次 `serve` 的启动扫描冻结（`crates/accounting/Spec.lean` §8-18-1）。
- **终端没了**（Windows 的 `CTRL_CLOSE_EVENT`，Unix 的 `SIGHUP`，raw 模式下读键出错）：切断所有终端出口（`Sinks::Cut`），进入 `Stopping(Interrupt)`，于是带着同一个时限。Windows 在关窗 5 s 后结束进程，4 s 给交接留出余量；Unix 取同一个数，三个平台行为相同。
- **升档**：收口中再来一次显式停止（`/quit`、页面的 `CloseCity`、`SIGINT`、Ctrl+Break）从 `Drain` 升到 `Interrupt`，时限从这一步起算；`Interrupt` 中再来一次，或第二个 `SIGTERM`，或时限到了，进程不写交接立刻退出，并说一行「the city exits without a handoff; `sprawling resume` will close what was open」。终端没了不算一次升档，只切断出口。
- `Interrupt` 怎样停：台子记下 `interrupting`，每条 lane 在下一个安全点从 `CommandDesk::interrupt_for` 读到 `Cancel`，后台命令经 `Backlog::halt(None)` 停下，不写 `city_halted`（否则下次开城这座城仍冻着）；然后照常 `land_the_rest` 并写交接；时限到时还没落地的 run，进程不等它。

**`/quit` 是一条线上命令**：`CloseCity { mode, idem }`（`crates/wire/spec/Command/Kind.lean`，`LocalOnly`）。执行者在 `bin::assembly`：`Listening::serve` 的 `select!` 里与信号同一条事件通道，`Front::commands` 在命令到达台子之前截下它，变成 `Event::Quit(Asker::Page, mode)`；worker 那一臂答「城在别处关闭」。只在监听绑在回环上时接受，绑在别的地址上一律以 `E_GATE_DENIED` 拒，因为 `/ws` 的命令出口不带对端地址，而回环绑定是「对端是回环」唯一不必另取对端地址就成立的条件；会话已认证由本地门的入场判定负责（`crates/wire/spec/Reception.lean`）。**推翻的记录**：此前「收口不是一条 Command，能被拼出来的线上帧就是陌生人停掉别人城市的一条路」。本地门要求每个调用者都带凭据之后，回环上不再有陌生人；剩下的本地调用者（居民的 `exec`、浏览器工具）本来就能发 `PutRules`、`RemoveBuilding` 这类更重的命令，停城对它们不是新增的能力。远程设备由 class 列的 `LocalOnly` 挡在中继上。

**CLI（`Cli`）**：主屏、raw 模式、内联，终端自己的回滚、搜索、复制照常可用。屏上分两块：**转写**写一次就进回滚；**活动区**在转写下面，每一帧重画。两块都由控制台的渲染器画（`console_ffi`，`crates/console_ffi/Spec.lean`）：控制台说屏上有什么，渲染器决定它长什么样，颜色只取终端自己的调色板，所以跟着人选的主题走。

- 第一行是横幅：`sprawling  <城目录>`，右边是地址与「/web opens the page」；窗口窄时先省掉提示，再省掉地址。默认房间是 `kernel::consts_policy::HALL_MAYOR`，房间写在 composer 下面那一行。
- 普通一行派给当前房间（`Dispatch`，`Mode::Work`），带 `/model` 与 `/effort` 选下的模型与档位。人提交的一行进转写，前面是本地时刻与 `›`。选中房间的一个 run 照 WebUI 的会话页进转写：第一次请求模型时一行头（住户、模型、模式、档位），每一轮推理的长度，每次工具调用在它答复时一行（名字、对象、用时，失败的标 `failed`），答复的正文（反引号里的代码原样、反引号淡色，`**` 之间加粗，围栏里的行保留空白），最后一行结局与总用时。时刻是这台电脑所在时区的 `HH:MM:SS`（`bin::console::local_time`），由记录的 `t` 换算，不另取时钟。一条记录属于这个房间，当它的地址是这个房间、写它的是这个房间的住户、或它是这个房间里正在工作的那个 run 的一行：run 的请求、调用、答复与冻结不带地址。别的房间与城级的记录不进转写。拒绝与斜杠动词的回答作为控制台自己的一行进转写。
- 活动区自上而下：最早的待批请求（`?`、它的说明、`y approve   n deny`）；正在工作的 run（`●`、住户、开始的时刻、`esc stops it`）；进行中的调用（至多最新三个，`running`）；`/quit` 问的那一句；然后是 composer——字在上，下一行一条线跟着字走，再下一行左边是房间、右边是平线问的模型与档位。活动区只在有事发生时重画：一条记录、一个键、一个粘贴、一次缩放，从不自己动（`crates/console_ffi/Spec.lean` D4）。
- 键（全部不用 Ctrl 组合键：Ctrl+C 与 Ctrl+V 是几乎所有人都会的两个键，归终端；Alt 在 macOS 默认终端里输入特殊字符、在 Windows Terminal 里是全屏与窗格）：Enter 发送；行尾的 `\` 加 Enter，或终端分得清时的 Shift+Enter，是换行；Esc 依次关菜单、清空输入、中断当前房间的 run（发 `Cancel`）；↑/↓ 是本次会话的历史；←/→、Home、End 移动；Alt+Backspace 删一个词，Alt+←/→ 按词移动；输入为空且有待批请求时 y、n 答它。
- **斜杠菜单**（`bin::console::ui::menu`）：Tab 补全斜杠动词，以及 `/model` 与 `/effort` 的参数；只有一个合适的就直接换上，几个合适的就打开菜单，光标在第一个——按 `wire::Slash` 的次序，所以 `/w` 加 Tab 加 Enter 是 `/web`。菜单开着时 Tab 与 ↓ 往下，Shift+Tab 与 ↑ 往上，Enter 取光标所在的那一项：不带参数的动词与参数取了就发送，带参数的动词写成「动词加空格」等参数；再打字时菜单跟着筛，什么都不合适就关上。菜单至多显示五项，下面一行说还有几项。
- 在 Unix 上，控制台向终端要两样东西：不含糊的键（kitty 键盘协议的第一级，Shift+Enter 因此与 Enter 分得开）与整段粘贴（bracketed paste）；不认得的终端忽略这两个请求。Windows 上读键走控制台 API，两样都没有。交还终端之前两样都撤回，光标形状也还原。
- `/quit` 是关闭城市唯一的键盘路径。没有 run 在跑：`Stopping(Drain)`。有 run 在跑：同一行问 `N runs are going   Enter wait for them   n stop them now   Esc keep serving`，Enter 是 `Drain`，n 是 `Interrupt`，Esc 取消。
- **参数补全从当前的 offer 取**（`Session::arguments`）：每按一次 Tab 问一次 `Query::EndpointView`，不另列一份模型表或档位表，也不缓存，因为接上或卸下一个 endpoint 之后，下一次 Tab 就该看见。`/model` 补全每个 endpoint 所服务的模型 id，去重后按字母排；`/effort` 补全当前（Endpoint，模型）的 `thinking.levels`。当前模型是 `/model` 选下的那个，没选就是 `main` 标签指向的那个；`/model` 选下的 id 先在 `main` 所在的 endpoint 上找，找不到才取第一个服务它的 endpoint。模型没有思考控制、或答询答不出来时，`/effort` 什么也不补。
- 斜杠词表是 `wire::Slash`（§8-21），控制台不再有自己的一份；`/wire <verb> [<json>]` 收起其余 wire 动词，它们是 `wire::COMMAND_NAMES` 与 `QUERY_NAMES` 的投影，JSON 体缺 `idem` 时由控制台补上这一行的键。
- **每一行一把幂等键，由控制台铸**：`LineKeys::drawn` 取 16 字节 OS 熵作 `origin`，`next` 按行计数，键 = `IdemKey::derive(RunId::CITY, 行号, origin)`。城把见过的键连同第一次的答复跨重启记住，所以只由文字派生的键会吞掉第二次敲的同一行。熵取不到时控制台说出原因并关闭，城照跑。

**安静宿主（`QuietHost`）**：备用屏、raw 模式，恰好两行，作为一块放在屏幕正中：地址，与配对码（本地门给出时；没有就只有地址），外加至多一行临时行，在下方隔一行：远程门的确认码，或收口进度。监听超出这台电脑、城为这次 serve 现铸了 key 时（`Terminal.token`），key 跟在第二行的配对码后面：备用屏是它唯一出现的地方，CLI 里要看它就 `/web`。主屏、回滚、地址栏与行控制台都不出现 key，`/web` 印出的地址不带 `?token=`，浏览器经 open code 配对（`firstrun::open_paired`）。Enter 再开一次浏览器，Esc 回到 `Cli`，主屏的回滚原样都在。离开备用屏之前先擦掉它（`Clear(All)` 再 `LeaveAlternateScreen`），因为用户设置可以让备用屏进回滚；真正的保证是配对码寿命短。窗口标题写「sprawling · closing this window stops it」，标题里不放配对码。安静宿主不印事件流；它显示期间进转写的行留给 CLI（至多 512 行），人按 Esc 回来时一并写进回滚。

**程序状态**（`bin::console::program_status`，模型、性质与取舍见 `Console/ProgramStatus.lean` D77）：CLI 与安静宿主两个面以 Program Status Protocol（OSC 7501）向终端报告整座城一条根记录：有请求待批是 `blocked`，有 run 在跑是 `working`，都没有时由最后一个结束的 run 决定 `done`、`error` 或 `idle`。UI 线程在开控制台、换面、读到 `run_started`、`run_frozen`、`approval_requested`、`approval_resolved` 时问一次 `Query::Metrics`，与终端存着的那条相同就不写，交还终端之前写 `state=clear`。报告只带计数，记录里的字到不了终端。`SPRAWLING_PROGRAM_STATUS=never` 关掉它；`Headless` 没有终端，不报告。

**终端只有一个写者**（`bin::console::ui`）：一条 UI 线程拥有终端，每一帧经渲染器写成字节，包在一次同步更新（mode 2026）里——先擦掉上一帧的活动区，写下新的转写行，再画活动区并把光标放回 composer；不认 2026 的终端照样画出同样的字节。别的线程经一个有界通道（`UI_DEPTH`，256 条）交给它一行；满了就丢，与诊断可丢同义，worker 与记账线程因此从不在终端上阻塞（旧 conhost 在选中文字时会暂停输出）。读键在另一条线程上，经同一个通道交进来。生命周期的面经 `tokio::sync::watch` 告诉 UI 线程，UI 线程按面切屏；面到了 `Stopping(_, Cut)` 时终端已经没了，UI 线程不再写任何东西。进入 raw 模式时装一个 panic hook：先擦掉备用屏、退出备用屏与 raw 模式，再交给原来的 hook，然后照常 abort。

**事件流与诊断行不再印到终端**（sprawling D44）。服务中的城每提交一行账本、每写一条诊断，终端零写入：诊断只进页面的 log 透镜（`serving::Journal` 的终端出口在控制台拥有终端时关掉），CLI 只印选中房间的记录。标准输入输出不是终端时（`Headless`），诊断照旧写标准错误，工装读它。**原因**：一条记录里有人打的字与模型的回复，终端会被旁人看到、被录屏、留在回滚里；CLI 印正文是因为人选了 CLI，这正是「正文只在人要时才印」本来的条件。**被否掉的做法**：保留默认每条记录一行的事件流——它在 CLI 里与答复抢屏，在安静宿主里违背「只有两行」，而 1024 条的广播通道订阅了却不读会让带正文的记录一直驻留。

**写不出去的一行归一处**：`console::ui` 是唯一决定「终端写失败怎么办」的地方——写不进去的终端是没人在读的终端，UI 线程把它当作终端没了，交给生命周期。

**本节接口的当前状态。**

- 进程级的一次性通知（`serving::standing` 的优先级、`serving::placement` 的座位与节流、`monitor::beat` 的节拍文件）走 `serving::journal::notice`，是 `Diagnostics` 的 `refuse` 级一行：控制台拥有终端时只进 log 透镜。`assembly::listen` 装上这个出口之前（这时还没有控制台）它们写标准错误。`supervising::children` 的几行不在其中：它们在子进程退出之后、由监护进程写在终端上，「按 Enter 再试」要人读到，那时没有控制台占着终端。
- 在 Windows 上粘贴多行文本时每个换行仍是一次 Enter，粘贴会被拆成几次发送：控制台 API 不报告粘贴，要统一处理需要另一种判断。Unix 上终端把粘贴整段交来，换行留在输入里。

**本章测试**：`bin::console::lifecycle` 逐行回放 `Console/Lifecycle.lean` §5 的转移向量；`/wire` 的投影含每个可经 socket 带的 Command 与每个 Query；行控制台把一行答在那一行之后，同一行敲两次是两次派活；`bin::console::tests::stream` 把一个 run 的记录逐条读成转写行，不带地址的调用与答复也算这个房间的；`bin::console::ui::menu` 的测试守住 `/w` 加 Tab 加 Enter 是 `/web`，以及菜单的窗口总含着光标所在的那一项。
-/

/-!
## 8-21 控制台读得到它身处的那座城

**原因**：控制台与 socket 拿的是同一张桌子（`CommandDesk`）与同一个答询函数（`wire::Answering`），一个答出来的数字与浏览器看到的不可能不同，因为它们是同一次调用。安静宿主只有两行，表头也只有一行，所以「这个进程开在哪、门朝谁开」要能随时问出来。

```rust
// bin::console::terminal（渲染是纯函数，I/O 在 UI 线程）
pub struct Terminal {
    pub url: String,
    pub token: Option<String>,    // 为超出这台电脑的 serve 现铸的 key，只印在安静宿主的第二行
    pub city: String,
    pub client: String,
    pub bind: SocketAddr,
    pub surface: Surface,         // 开城后进入的面
}
pub(crate) fn serving(terminal: &Terminal, vitals: Option<&wire::MetricsAnswer>, pid: u32) -> String;
```

- **`/serving` 是渲染，不是来源**。城侧的数全部来自一次 `Query::Metrics`；`/wire metrics` 仍印它的 JSONL 原样，与 `sprawling call` 同形。两个动词，两个问题：`/serving` 答「这个进程开在哪、门朝谁开」，`metrics` 答「这座城里有多少什么」。
- **动词名不与既有概念撞车**：`status` 在 `docs/glossary.md` 里是工具的名字，故控制台这个动词叫 `serving`。
- **常驻内存不进这一屏**：「resident 在本平台叫什么」的唯一权威是 `xtask::mem`；`/serving` 印出 pid，`cargo xtask mem <pid>` 只差一次粘贴。**翻案条件**：新增一个 unit 承载这一个计数器，届时两个调用方共用一份定义。
- **斜杠词表一张**（`wire::slash`，`crates/wire/spec/Slash.lean`）：`/help`、`/room <addr>`、`/new`、`/stop`、`/halt [--all]`、`/release [--all]`、`/model [<id>]`、`/effort [<level>]`、`/approve [<id>]`、`/deny [<id>]`、`/web`、`/quit`、`/serving`、`/remote <verb>`、`/acp [<text>]`、`/wire <verb> [<json>]`，每行写明在 CLI、WebUI 还是两处都提供。CLI 读它，客户端经 `cargo xtask wire-ts` 生成的 `SLASH` 读它；`/at` 不再存在，选房间是 `/room`。
-/
