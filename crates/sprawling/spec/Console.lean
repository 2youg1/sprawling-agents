-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::console

规定 `crates/sprawling/src/console.rs` 与 `crates/sprawling/src/console/`：服务中的那个终端（`bin::console`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::console::tests::helpers`、`bin::console::tests::parsing`、`bin::console::tests::terminal` 守住。
-/

/-!
## 8-11 控制台：服务中的那个终端不再是死胡同

**已有工作区的人**：`form_city(root, Adopt)` 取代 `init_city` 成为唯一的成城路径（后者是 `Adopt::Nothing` 的别名）。`Adopt` 是穷尽枚举而不是布尔：在既有工作旁边形成一座城、与把那些工作放到规则之下，是两件事，一个布尔会把它们说成一件事的一个设置。采纳走的是 `sprawling adopt` 的同一道门，于是创世时收进来的文件夹与一个月后收进来的受同一套规则治理。首屏因此长出第三个答案 `FirstScreen::Use(path)`——回车之外、`q` 之外的任何输入都是一个路径（去掉文件管理器加的引号）；**路径存不存在由调用方查并报**，屏幕自己去猜要么把真文件夹当错字拒了，要么在没人看过的位置造一座城。

**原因**：`sprawling up` 打四行字然后阻塞到 Ctrl-C。那块屏幕是产品白白扔掉的一个面，也是一台没有浏览器的机器**仅有的**那一个面。

**写不出去的一行归一处**（G-23）：`console::terminal::say` 是这个文件里唯一决定「控制台写失败怎么办」的地方——写不进去的控制台是没人在读的控制台，而城不归控制台停，故失败止于此。十三处 `let _ = writeln!(out, …)` 因此不再各自决定一次。同章的两个布尔入参改为枚举（G-25）：`serve_city` 的 `open: bool` 成 `Open::{Browser, Nothing}`，`install::install` 的 `uninstall: bool` 成 `Direction::{Install, Uninstall}`——`install(true)` 在调用点说不出它做了什么。

```rust
// bin::console（形状 1 decision；壳在一条线程里，判定全在纯函数）
pub(crate) enum Line {
    Nothing,
    Help,
    OpenWeb,
    Select(Address),
    Quit,
    Frame(Box<wire::ClientFrame>),   // 一个 wire 动词
    Work(String),                        // 普通一行：派给当前选中的 room
    Unknown { verb: String, nearest: Vec<String> },
    Malformed { verb: String, reason: String },  // 动词认得，JSON 体读不出：reason 是 serde 的原话，写出缺的字段名
}
pub(crate) fn parse(line: &str, selected: Option<&Address>, idem: IdemKey) -> Line;
struct LineKeys { origin: [u8; 16], lines: Seq }   // 每行一把键
impl LineKeys { fn drawn() -> Result<LineKeys, AxError>; fn next(&mut self) -> IdemKey; }
pub(crate) fn verbs() -> Vec<String>;      // 控制动词 ⊕ wire 动词
pub(crate) fn snake(camel: &str) -> String;
```

- **wire 动词表是投影，不是第二份手写清单**。`verbs()` 从 `wire::COMMAND_NAMES` 与 `QUERY_NAMES` 逐个转 snake_case 得来；一份手写清单就是第二套词汇，而它漂开时没有任何东西会发出声音。一条断言钉住这件事：每个 wire 名字都在动词表里。
- **控制动词另成一个穷尽枚举**（`/help`、`/web`、`/at`、`/quit`）。它们是**控制台自己的**动词，不在 wire 上，故不属于那张投影。两张表合并后仍不得重名，一条断言钉住。
- **控制台不做任何判定**。一行变成 `Command` 之后，走的是人在页面上点按钮走的**同一张桌子**（`CommandDesk`）与同一个 `Reply`。拒绝因此自动回到控制台，不需要为它另写一条回程——这正是那条回信地址的第二个消费者。
- **每一行一把幂等键，由控制台铸**。`LineKeys::drawn` 在控制台启动时取 16 字节 OS 熵作 `origin`，`next` 按行计数：键 = `IdemKey::derive(RunId::CITY, 行号, origin)`。城把见过的键连同第一次的答复一起记住，且跨重启记住；故键只由 `地址+任务` 派生时，同一房间同一句话第二次被吞，先被拒（例如还没配模型）、配好模型后再打同一行仍拿到那次拒绝。行号使同一进程内的两行不同，`origin` 使两次启动的同一行号不同。熵取不到时控制台说出原因并关闭，城照跑——一把可预测的键会被上一个进程的答复吞掉。**被否掉的做法**：把时间放进键——时间只在 `bin::assembly` 取样，且同一毫秒内两行仍撞。
- **wire 动词的 JSON 体缺 `idem` 时由控制台补上这一行的键**；人在没有浏览器的机器上刹住整座城只需 `/halt {"scope":"city"}`。体读不出时返回 `Malformed`，打印 serde 说出的原因（含缺的字段名），而不是「没有这个动词」。
- **投影只取 socket 能带的 Command**：`put_secret`（`WireCommand` 里没有这个值）与 `auth`（配对令牌在握手里证明，不在命令里）是 `Command::idem()` 返回 `None` 的恰好那两个，控制台既不列它们也不认它们。
- **`/quit` 只关控制台，城继续服务**；帮助说的是同一句话。停城是 Ctrl-C，那是只有起城的进程按得动的收口（下文）。
- **普通一行就是派活**。要人为一件活敲 `/dispatch {"addr":…}` 是把 JSON 当人机界面；选中一个 room（`/at`）后直接写任务，才是终端本来的手势。未选中任何 room 时拒，并说该敲什么。
- **不是 TTY 就不进控制台**。stdin 读到 EOF（管道、服务、CI）即退出控制台循环而**城照跑**：一座因为没人敲键盘而停止服务的城是一个以交互换服务的回归。
- **拒长表与图**。查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形；表格与图归浏览器。一个同时伺候两个主人的 CLI 是 CLI 文献里的反面教材。`sprawling view` 是另一个动词、另一个进程，按 stdout 是不是终端把两个主人分开（§8-105、§8-117）。
- **事件流默认每条记录一行，不印正文**（sprawling D44）：`start` 订阅 Committed 事件流，按 `Terminal.records: Records` 打印，`console::stream::printed(record, records)` 是唯一的渲染。`Records::Summary`（默认）印 `  seq <n>  <kind>  <addr>`——kind 用账本与 wire 共用的 serde 名，没有地址的记录写 `city`；`Records::Whole` 印整条记录，与 `sprawling call` 同形，只在 `up`／`serve` 带 `--whole-records` 时取。**原因**：一条记录里有 User 打的字与模型的回复，终端会被旁人看到、被录屏、留在滚动缓冲里，所以默认只说发生了什么、在哪里。**被否掉的做法**：默认整条、加一个 `--quiet` 去掉——泄露成了默认，要人知道去关。三个平台行为相同。
- **`/web` 携配对令牌**，故没有人需要手拷一串东西。令牌在 `serve` 里只被读一次，控制台拿到的是那一次的副本，不重新读环境。
- **Ctrl-C 是有序收口**：`serve` 在 `wire::serve` 与 `tokio::signal::ctrl_c` 之间 `select!`。收到信号后先停止接受连接，再 `CommandDesk::close(Closing::Chosen)` 告诉 worker，worker **在读队列的同一处**读到它，于是正在跑的那条命令先跑完，`handoff_written` 是最后一行而不是某一行的中间。主线程 join worker 线程再返回——先返回的 main 会在那一行写出来之前结束进程。
  - **`DeskWait::Close` 与 `Gone` 不是一回事**：前者是城要停了，值一份 Handoff；后者是桌子自己坏了，那座城已经写不出 Handoff 了。
  - **收口带着它的缘由**：`CommandDesk::close(Closing)`，`Closing` 是穷尽枚举（`accounting::worker::lifetime`）：`Chosen`——人按了 Ctrl-C；`Broken { cause }`——`wire::serve` 返回了错误。serve 的结果只有一处读法：`Closing::of(&served)`，`Ok` 读作 `Chosen`，`Err` 读作 `Broken`。信号处理器装不上不是 `Broken`：`serve` 在标准错误上说一句，然后继续等 `wire::serve`，城照常服务，只是 Ctrl-C 回到不写 handoff 的硬停；装不上信号处理器只拿走了有序收口，没有坏掉服务，为它关掉一座正常服务的城是把小故障放大成停城。`serve` 只从 `select!` 的结果里判这一次：`Ok` 即 `Chosen`，`Err` 即 `Broken`，错误的文字就是 `cause`。`close_city(&Closing)` 按缘由写 handoff：只有 `Chosen` 写「the city was closed by the User running it」；`Broken` 写「the city stopped because serving failed」并带上 `cause`，下一步是先修 `cause` 点名的东西。**原因**：没有人做过的事不能记成人做的；一份把失败写成人主动关城的交接件，会让下一任以为什么都没坏。**否决的方案**：失败时不写 handoff——那样失败与崩溃在记录里又成了同一种沉默，而 worker 此时仍然写得出这一行。
  - **收口不是一条 Command**：能被拼出来的线上帧就是陌生人停掉别人城市的一条路。`closing` 是台子上的一个 `OnceLock<Closing>`，只有起城的那个进程按得动；先到的缘由作数，第二次 `close` 不改写它。
  - **Windows 交两个信号，本城两个都收**：控制台会发 Ctrl-C 与 Ctrl-Break。一座在其中一个上有序收口、在另一个上暴死的城，等于同一个手势有两种行为，而决定用哪一种的是人碰巧按了哪个键。其他平台只有一个。
  - **钥匙在旁读，信号随即装上**：控制台要的远程门会读城钥匙，macOS 上二进制更新后的第一次读会等一次钥匙串对话框（`crates/remote_access/Spec.lean` §8-3 D23）。`Listening::serve` 不在 reactor 线程上读它：`Outdoors::keep_aside` 在一条自己的线程上读钥匙、建门，再在那里起控制台（`console::start`），`serve` 随即进入 `select!`，第一次轮询就装上 Ctrl-C 的处理器（Windows 上 `ctrl_c` 与 `ctrl_break`，macOS 与 Linux 上 `SIGINT`）。于是没人答对话框时（经 ssh 起的城、无人值守的 Mac、CI 里以 `&` 起的进程，那里 `SIGINT` 在处理器装上之前是被忽略的），Ctrl-C 照样有序收口；那条线程停在对话框上，随进程结束。Windows 的凭据管理器与 Linux 的 Secret Service 不弹对话框，次序在三个平台相同。**否决的方案**：给读钥匙加一个时限、到时把门记为「钥匙读不到」——多一个常数与一个第三态，而读钥匙离开 reactor 线程已让关城不再依赖这一读；只把处理器提前装上、仍在 reactor 线程上读钥匙——信号被接住了，可 `select!` 在读钥匙返回之前轮不到它，城照样关不掉。
  - **代价**：根 `Cargo.toml` 给 tokio 开 `signal` feature。unix 上它引入 `signal-hook-registry`（Apache-2.0/MIT，deny 表内）。

**本章测试**：每个 `COMMAND_NAMES`／`QUERY_NAMES` 都在 `verbs()` 里（这就是「投影而非第二份清单」的可执行形式）；控制动词与 wire 动词不重名；`snake` 对 `AttachEndpoint`／`RunView` 给出预期串；空行、`/quit`、`/at <addr>`、普通文本（选中与未选中两情形）、未知动词（携最接近的几个）各得正确枚举。
-/

/-!
## 8-21 控制台读得到它身处的那座城

**原因（两处，同一个不对称）**：控制台与 socket 拿的是同一张桌子（`CommandDesk`）与同一条事件流，唯独**读**这一路没接上。

1. `console::post` 对 `ClientFrame::Ask` 只印一句「a question is answered over the wire: `sprawling call '…'`」——它请人开第二个终端，去问一座人已经身处其中的城。而 `accounting::worker::serve` 早已构造出 `queries: wire::Answering` 并只交给 socket。§8-11 自述「查询的答案在控制台以 JSONL 逐行输出，与 `sprawling call` 同形」——**这句话今天是假的**，这次改动使它为真。
2. `serve_city` 起城时印的四行（city／WebUI／client）随事件流滚走。一个远程盯着城的人于是再也看不到自己开在哪个端口、有几条 run 在跑。

```rust
// bin::console（形状仍为 1 decision；渲染是纯函数，I/O 仍在壳里）
pub struct Terminal {
    pub url: String,
    pub token: Option<String>,
    pub city: String,     // 新增：城在磁盘上的位置
    pub client: String,   // 新增：客户端从哪来（嵌入／目录）
    pub bind: SocketAddr, // 新增：真正绑住的那个地址
}

/// socket 用的那一个答询函数，控制台拿到的是它的副本。
pub(crate) type Answering =
    Arc<dyn Fn(wire::Query) -> Result<wire::Answer, AxError> + Send + Sync>;

pub(crate) enum Line { …, Serving }   // 控制动词从四个变五个

/// 进程自己知道的事实 ⊕ 一次 Metrics 的答案 → 一屏。纯。
pub(crate) fn serving(terminal: &Terminal, vitals: &wire::MetricsAnswer, pid: u32) -> String;
```

- **答询走同一个函数，不是第二个权威**。`post` 的 Query 臂改调 `Answering`，与 `wire::server` 的 `SessionStep::Answer` 是同一个 `Arc`；控制台答出来的数字与浏览器看到的数字不可能不同，因为它们是同一次调用。
- **`/serving` 是渲染，不是来源**。城侧那几个数（几条 run 在跑、几件事等人、几栋楼）全部来自一次 `Query::Metrics`；`/metrics` 仍印它的 JSONL 原样，与 `sprawling call` 同形。两个动词，两个问题，无重叠：`/serving` 答「这个进程开在哪、门朝谁开」，`/metrics` 答「这座城里有多少什么」。
- **动词名不与既有概念撞车**。`status` 在 `docs/glossary.md` 里已经是**工具**的名字（「答一次 run 自己的处境」），一名一义是门禁事项，故控制台这个动词叫 `serving`——它印的正是 `accounting::worker::Serving` 持有的那几样东西，沿用已在库内的词。
- **常驻内存不进这一屏，理由记在这里**。「resident 在本平台叫什么」的唯一权威是 `xtask::mem`（它读哪三个计数器、各平台叫什么，见 `tools/xtask/Spec.lean` §8-30），而 `xtask` 只依赖 `kernel`——让它依赖产品会使每次门禁编译整个 workspace。在 bin 里再抄一张三平台表，正是那个模块自己的 doc comment 警告的「三份权威」。`/serving` 因此印出本进程 **pid**，`cargo xtask mem <pid>` 只差一次粘贴。**翻案条件**：新增第十三个 unit 承载这一个计数器（ARCHITECTURE.md §3 的拓扑是 add-only），届时两个调用方共用一份定义。

**红**：一条测试把 `Line::Serving` 之外的路径全部钉住不动，另一条驱动 `drive` 读入 `/metrics`，断言输出里有 `MetricsAnswer` 的 JSON 而**不含** `sprawling call`——改动之前它撞上那句转介。第三条断言 `serving()` 的那一屏同时含端口、`runs`、与 pid。
-/
