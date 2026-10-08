-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.sprawling.spec.Accounting.Landing

/-!
# accounting::worker：写在 sprawling 规格里的那些节

本分部收着写 accounting crate 里 `accounting::worker` 的节。模块从 `sprawling` 搬进 `accounting` 时，写它的那一节留在本 crate 的规格里（accounting D15），标签不变；为什么暂住这里、何时搬走，见 sprawling D30。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；记账线程上落 run 必须守住的性质证在 `crates/sprawling/spec/Accounting/Landing.lean`，其余由 Rust 的类型与 `accounting::worker` 旁的测试守住。
-/

/-!
## 8-2 派活回路

```rust
// bin::assembly —— 字段按所持状态分成六组（§8-110、§8-111），与整城共用的账本、CAS、日志
pub struct RunWorker { /* 私有 */ }
impl RunWorker {
    pub fn new(city_root: &Path, vault: gateway::Custodian, log: runtime::diagnostics::Diagnostics) -> Result<Self, AxError>;
    pub(crate) fn observe(&mut self, sink: Box<dyn FnMut(&EventRecord) + Send>);
}
// bin::assembly::listening —— 先占端口，再开写者（§8-88）
pub async fn listen(serving: Serving) -> Result<Listening, AxError>;
impl Listening { pub async fn serve(self) -> Result<(), AxError>; }
```

- **单写者**：账本只在写者线程（`assembly::attending`）上写；lane 写的行经 `accounting::worker::relay` 交给它（§8-42-2），所以一座城的历史只有一个追加者。
- **命令受理与命令执行分开**：socket 任务只把命令放上命令台（`accounting::worker::desk`），命令在写者线程上执行。刷新页面不会杀掉工作，而进展从事件流回流——「关掉界面再打开」与「从未关过」在服务端看来因此无差别。
- **事件流的源头是 Ledger 本人**：`JsonlLedger::observe` 在**持久化之后**逐条回调，回调把记录扔进 broadcast。服务端因此推不出一条历史里没有的事实。
- **工具名录与工具台同源**：`Catalog::admit_tool` 产 `ToolDef`（模型看到的），`ToolBench::register` 负责路由（实际跑的），一次登记喂两边——否则“模型以为存在的工具”与“真能跑的工具”会成为两份名单。
- **RunId 是推导而非抽取**：`dispatching::agreeing::run_id_for(job, addr, now)`，`b3(job|addr|now)` 前 16 字节。同一毫秒对同一地址派同一件活就是同一个 Run，且标识符里不进随机数（确定性第 7 条的同一条理由）。
- **一跑没有回合上限**：刹车是 `Cancel` 与 `Halt`（先判定后动手那一节 §8-40）。
- **`init` 写 `City.md` 入城**：二进制携默认本，城里那份是用户可改的权威；每次组装 prefix 读城里的那一份，代码里恒不长第二份副本。
-/

/-!
## 8-4 MCP 接线

三种传输、兑付与 `McpLink` 住 `agent_protocols::mcp`（签名见 `crates/agent_protocols/Spec.lean` §8-17）；本 crate 只把一栋楼的 `[[mcp]]` 表接成工具表。

```rust
// accounting::worker::workbench::servers —— impl Laying（lane 准备派活时持有，§8-113）
fn mcp_tools(&self, config: &FrozenConfig, write_root: &Path, confidential: bool)
    -> Vec<agent_protocols::McpTool>;   // 起不来的 server 缺席并留下诊断，恒不拒整次 dispatch

// accounting::worker::mcp（形状 3 常驻表）：worker 持有，一台 server 一项，每项一把锁
pub(crate) struct Residents { /* 私有：Mutex<Vec<Arc<Keyed>>>，键 (McpServer, write_root)，每个 Keyed 自带 Mutex<Option<Resident>> */ }
impl Residents {
    pub(crate) fn tools(&self, server: &kernel::McpServer, write_root: &Path, confidential: bool,
                        resolve: &gateway::SecretResolver)
        -> Result<(Vec<agent_protocols::McpTool>, Reached), AxError>;   // Reached::{Connected(Handshake), Resident}
}
```

- **兑付只有一处，三种 transport 共用**（`agent_protocols::mcp::redeeming`）：一个子进程要的环境变量与一台主机要的请求头是同一件事——一个名、一个值、以及一个可能是凭证的值。兑付发生在读配置的那一刻而不是第一次调用时：金库里没有的引用是一个配置错误，而能处理它的人正在编辑那份文件，不是一小时后工具不应答的那个模型。`Redeemed` 的 `Debug` 只报名字，明文的唯一出口是 `expose`。
- **交给子进程的名字收不回来**，故 `start` 收到的 `env` 已经是兑付好的值，明文只在这一次调用里存在；`env` 是**添加**到本进程已有的环境上，否则一台 server 会找不到 PATH 与 HOME。
- **一台 server 一个子进程，寿命是 worker 的寿命**（`accounting::worker::mcp::Residents`，常驻连接表）。每次 dispatch 都起子进程、握手、list，是 `[prepare_dispatch]` 里 servers 那一半的全部开销；表把它降到一次。键是整条 `McpServer` 声明加上运行根 `write_root`：声明任一字段变了就是另一台 server，起在另一个目录里的子进程也不是同一台，因为子进程的工作目录在启动时定下、之后改不了。命中且子进程仍在运行时，这次 dispatch 用表里记下的清单（`ToolMeta` 与远端名成对）和同一条连接构造 `McpTool`，不握手也不 list；子进程已退出或问不出状态，就丢掉这一项并重连，这就是 server 死后的重连。HTTP 与 SSE 的表项恒视为可用：连接逐次请求，一台死掉的主机在那次调用里失败并留名。表按键上锁：表自己的锁只在找键、添键的那一刻持有，连接、握手与 list 在这一键自己的锁里做，于是同一台 server 的两次查询排在同一把键锁上，先到的连接、后到的命中，不同 server 的连接互不等待；`tools` 取 `&self`，表是 `Sync`，可以被几条线程同时问。几条 lane 同时用一条 stdio 连接时由连接自己的锁排队。代价有两件：清单在连接那一刻读定，server 在两次 run 之间改了清单，要等子进程重启才看得见；配置里删掉的 server 的子进程活到 worker 落地。worker 落地时表落地，最后一个句柄落地时 `Drop` 杀子进程，于是「谁来回收」仍不需要第二份名单。缺表的那一次连接在准备这个 run 的 lane 里建立，lane 借的是表的 `Arc`（§8-113）。
- **读取线程是句柄的一部分，spawn 点仍在 bin**（确定性七条③的口径：并发归装配层）。同步读一根管道没有期限，而一个不回答的 server 会把整个 Run 挂死。故 `start` 起一条只读 stdout 的线程，`call` 用 `recv_timeout` 等它；超时即杀子进程并三段式拒。**线程恒不泄漏**：杀子进程关掉管道，读到 EOF 即结束。
- **一台 server 一个子进程，一次 dispatch 一条命**：工具表随 Run 冻结，子进程的寿命因此就是 Run 的寿命。最后一个 `McpTool` 落地时 `Drop` 杀子进程，于是「谁来回收」不需要第二份名单。
- **读取线程是句柄的一部分**（ARCHITECTURE 确定性规则 3 为 `agent_protocols::mcp::stdio` 与 `agent_protocols::mcp::sse` 各留一个每连接一条的 reader）。同步读一根管道没有期限，而一个不回答的 server 会把整个 Run 挂死。故 `start` 起一条只读 stdout 的线程，`call` 用 `recv_timeout` 等它；超时即杀子进程并三段式拒。**线程恒不泄漏**：杀子进程关掉管道，读到 EOF 即结束。
- **期限从声明里来，不在适配器里另写一个数**：`ToolMeta.timeout`（`tools_from` 写的 `TimeoutMs(60_000)`）既是对模型的承诺，就应当是真正被执行的那一个；否则该字段只是装饰。故期限随 `Outbound::call` 入参。
- **起不来的 server 缺席而不拒 dispatch**：与 `city::library` 对「楼里点名却不在架上的 SKILL」同形——模型看到的名录（工具表加休眠索引）恒等于真能跑的工具，缺席的那一件在诊断里留名。一个外部服务今天起不来，不是这栋楼今天不能干活。
- **confidential 楼：一条规则两层后果，不是两份判定**。工具能不能存在归 `agent_protocols::McpTool::new`（构造点拒，恒是权威）；**进程该不该被拉起归装配层**，因为进程寿命本来就是这一层的职责，而一台 MCP server 可能在启动那一刻就出网。故 confidential 楼在拉起任何子进程之前就跳过整张 `[[mcp]]` 表并留一条诊断；工具层的拒仍在，它是那一层失守时的兵底。两层同向，因此不会出现「只改一处」的漂移。
- **`Effect::Connector` 是这次接线带来的 kernel 变更**（语义住 `crates/kernel/Spec.lean` §8-23）：接线前 `tools_from` 写的是 `Effect::Egress`，而出站门从 **调用参数**里读 `host`——外部工具的参数表由 server 的 `inputSchema` 决定，里面恒没有 `host`。第一次真调用当场拿到 `E_INVALID_ARGS: declares Egress but named no host`：这就是「一个适配器是假想缝」的同一条道理在工具面上的实例——没有调用方的声明从未被那道门验过。
- **discover 先于 list，但今天不据它分支**：它当下的作用是在把任何工具交给模型之前，先证明对侧真的会应答；版本协商要有第二个版本才成立，而字段名本库今天无法从一台真的 server 上核对。读到什么写进诊断，不写进判定。
- **口径不变的那两件**：外部工具与 L0 工具同落 `kernel::tool` 缝，故结果恒自动进污染环，装配层无解包面；调用由 `ToolBench` 路由，故 `tool_called`／`tool_result` 两行自动落 Ledger，不为它另写一条入账路径。
-/

/-!
## 8-4b MCP 的第二条传输

`agent_protocols::mcp::http` 是 `Outbound` 缝的第三个适配器（stdio 子进程、ScriptedOutbound、HTTP），也是这条缝第一次真正被两条生产路径共用。`McpServer.transport` 从两个裸字段改成穷尽枚举 `McpTransport { Stdio{command,args}, Http{url,header} }`：**一行既写 command 又写 url 就是一行要读者去猜的配置**，故配置层当场三段式拒。差异全部花在 `agent_protocols::McpLink` 这一个枚举里，其上的接线仍是一条路。

两条口径：①**事件流只取第一条 `data:`**，读不出就拒，恒不把两条答案拼成一条工具结果；②**拒词不引用对侧正文**（服务端的错误页是别人写的字），只说状态码与该查什么。
-/

/-!
## 8-4c 会话、凭据与报错地址

- **`HttpServer` 持会话，且克隆共享它**（规范详 `crates/agent_protocols/Spec.lean` §8-3）。一台 server 就是一个会话，不论一次 Run 拿了它几件工具；两个克隆各发一个 session id 就是与同一台 server 开两场对话，而它只开过一场。协商版本从**携 `result.protocolVersion` 的那一条答案**学得——按生命周期，那就是 `initialize` 的答案，因为它是一条连接的第一句话，没有更早的答案能持有该字段。
- **`header` 兑付 `secret:` 引用**。`redeem_header` 把 `Name: secret:realm/name` 在最后一刻换成真值，与 endpoint 凭据同一条路。不这么做，一把付费档的 key 会明文躺在楼里的 `CONFIG.toml`，而 `xtask secret` 看不见它（城市配置不在仓库内）。不是引用的值原样通过：头里是个账号名或固定标记的 server 无物可兑。
- **报错地址指向真正出事的传输**。`agent_protocols::McpLink::site()` 按 `McpTransport` 分派；把所有 MCP 失败都挂在一个传输名下，会把跟着它去查的人引到错的文件。成功时同样留一行：对侧叫什么、说哪个版本、给了几件工具。

**真机验收**：一座真城接一台托管 server，诊断行为 `exa is exa-search-server speaking 2025-06-18, offering 2 tool(s)`；模型自主调用其搜索工具、读回真实结果、一个回合内给出答案并 `run_frozen{completion: done}`。
-/

/-!
## 8-4e harness 居民的一次 run（性质已证明）

一个房间的居民可以是任何一个人同意过的 ACP agent，五家官方 harness 是其中的内置条目（`crates/agent_protocols/Spec.lean` §8-19）。派活到这样的房间时，城起那一个 agent 的进程，在房间自己的 worktree 里开一场 ACP 会话，把它汇报的东西记进账本，在它答出停止原因时结束这次 run。

**性质的权威是 `crates/agent_protocols/spec/Harness/Session.lean`**，六组定理：

- **汇报恒不是准入历史**（`a_report_is_never_admitted`）：一次 harness run 的准入记录只有它的开始、检查点、它对城那次 prompt 的回答与冻结，中间汇报多少、汇报什么都不改变这一点。
- **截断先变成取消**（`a_halt_is_a_cancel_before_anything_else`、`a_second_halt_sends_nothing`）：城观察到一个罩住这个房间的停摆、或楼规的墙钟上限到了之后，run 发出的下一件事就是 `session/cancel`，此后的汇报排在它后面；第二次截断什么也不发，结局读头一次的那个。
- **树先提交，冻结的 run 是历史**（`nothing_follows_the_stop_reason`）：停止原因到了，run 先把树提交成检查点，再记下回答、冻结，此后什么都不记、什么都不发。
- **会话断了就不记回答**（`a_lost_session_freezes_cancelled_with_no_answer`）：没答出停止原因的会话冻成 `Cancelled`，账本里没有一条 harness 没给过的回答。
- **Done 要一次说了话的 `end_turn`**（`done_needs_an_end_turn_that_spoke`、`a_deadline_never_freezes_cancelled`）：其余停止原因与空回答都冻成 `Limit` 或 `Cancelled`，墙钟上限恒不读作 `Cancelled`。
- **只有准入的记录能作证**（`a_cited_record_is_admitted`、`no_report_is_admitted`）：Done 引的那一行是 harness 对城那次 prompt 的回答；汇报恒不在准入历史里，所以恒不作证。

**它守的是 ARCHITECTURE §5 第 4 步的弱形**。harness 自己执行工具，城准不了也拒不了它做的事，只能记它选择汇报的东西。所以「每个效果先成为事件」在这里分成两半：城自己决定的事（run 开始了、停摆变成了取消、城那次 prompt 得到了什么回答、run 怎么结束）是准入历史；harness 一路上说它做了什么是汇报，在它说了之后才落账，恒不被读回来当作一个判定。

**已定的十条**：

1. **汇报是一个新的 record-only 种类 `harness_reported`**：载荷是 `kernel::event::record::HarnessReported`（`crates/kernel/Spec.lean` §8-4），一条汇报一行：回答或推理的一段、harness 开始的一次工具调用与它的状态、一种本城没有读法的汇报（只记名字），以及 permission 的问与城的答。种类名进握手哈希，旧页面在握手处被拒，`WIRE_V` 不为此进位（wire D1）。回答「城做了什么」的 fold 恒不读它，只有回答「harness 说了什么」的视图读它。
2. **停摆到取消**：城在把下一条汇报落账之前查一次这个房间所在的停摆；罩住了，就先落 `cancel_received`，再发 `session/cancel`，然后照常读到 `stopReason: cancelled` 为止。复用已有的 `cancel_received`，因为它记的正是「这个 run 收到了一次取消」。
3. **停止原因到结局**：`cancelled` 冻成 `Completion::Cancelled`，截断它的是墙钟上限时冻成 `Completion::Limit`；`max_tokens`、`max_turn_requests` 与 `refusal` 冻成 `Completion::Limit`，因为 run 是撞上了什么而停，不是做完了；`end_turn` 见第 8 条。每一种停止原因都先提交检查点、写一条 `harness_answered`，再冻结：`Session.lean` 里 `checkpoint`、`answer`、`freeze` 依次发出，对五种停止原因都一样。这张表只在 `runtime::run::harness` 一处判（`crates/runtime/Spec.lean` §8-52）。
4. **confidential 楼拒绝 harness 居民**：`agree_to_work` 在写任何东西之前答 `E_GATE_DENIED`。harness 执行自己的工具，并把房间的内容送到它自己厂商的服务器，confidential 楼「数据进来不出去」的承诺对它不成立。
5. **harness 恒在房间自己的 worktree 里跑**：用评审楼的同一种租约，不论楼的 `review` 设了什么。城管不了它写什么，但管得了它写在哪：它的写入只经已有的评审合并进入城的主树，而合并是准入的。harness 够不着城的协作工具（第 7 条），所以一次冻成 `Done` 的 run 落地时由城替它开请求（§8-124）。
6. **进程归 `agent_protocols::harness`**：起 `Launch` 的程序与参数，工作目录是那棵 worktree，走管道，落地即杀。理由与 `agent_protocols::mcp::stdio` 相同：字节怎么走归协议那个 crate（`crates/agent_protocols/Spec.lean` §7）。
7. **`session/new` 的 `mcpServers` 这一版给空表**：城的工具要经一台城自己的 MCP server 交出去，那是另一件活。楼里 `[[mcp]]` 的 server 与城自带的桌面（§8-4d）也不转交：harness 直接调它们，`kernel::gate::undoable` 那道升给人的门就够不着了。

8. **`end_turn` 由城自己的记录作证**：`session/prompt` 答出停止原因之后，城先把这棵 worktree 提交成自己的检查点（`checkpoint_committed`，harness 改了什么由城记下；五种停止原因都提交，一次被截断的 run 改了什么同样要记），再写一条 record-only 种类 `harness_answered`：载荷携停止原因，与这一回合 harness 回答城的那段文字（`agent_message_chunk` 依次拼起来）。run 冻成 `Completion::Done`，证据引这一行，`kernel::completion::Evidence` 因此在 `tool_result`、`model_returned` 之外多收一种。这与模型 run 的证据同一个标准：模型的 run 以它最后那条 `model_returned` 作证（`runtime::run::lifecycle::concluded`），harness 的 run 以它对城那次请求的回答作证。回答是空的，冻成 `Limit`，与 `concluded` 对空回复的判法相同。**被否**：给 `Completion` 加一个「harness 自称完成」的变体（动 kernel 的公开面与每一个按结局分支的读者）；在 bench 上立一件「harness」工具，把 prompt 包成 `tool_called`／`tool_result`（那是一件没有模型调用过、也没有门判过的工具）。
9. **permission 一律答它给出的第一个 `allow_once`（定规）**：没有 `allow_once` 就答第一个 `reject_once`，两者都没有就答 `cancelled`。恒不答 `allow_always`：那是替以后的调用做决定。问与答各记成一条 `harness_reported`。理由：城不能按工具名授权（ACP 规格），能兜住的是这棵 worktree（第 5 条）；一律拒绝会让默认要问的 harness 什么都做不成；交给人则要让 Approval Inbox 收动作，而它今天只收设计问题，且一次挂起的 prompt 占着一条车道。**重开参数**：Approval Inbox 开始收动作。
10. **房间的 harness 由 `CONFIG.toml` 的 `[resident] harness` 点名（定规）**：值是城 `CONFIG.toml` 里一行 `[[agent]]` 的 `id`，或一个内置条目的词（`agent_protocols::OFFICIAL`），在城／楼／房间的梯子上取最近一级（`city::settled_harness`，`crates/city/Spec.lean` §8-4）。房间在派活时才开，所以这个键实际写在楼层或城层。`city` 只把它当字符串读进来，`[[agent]]` 的行也由 `city` 读（`city::agent_rows`）：`city` 只见 `kernel`，一个词点名的是哪个条目，权威在 `agent_protocols::Roster::seat`。`agree_to_work` 在写任何东西之前用它判，不认识的词答 `E_CONFIG_INVALID` 并列出认得的词；一行 `[[agent]]` 的 `launch_digest` 与重算的摘要不等，同样答 `E_CONFIG_INVALID`，那一行在人同意之后被改过。`[model] name` 不在梯子上，它是房间自己那一层的会话记录（`crates/city/Spec.lean` §8-14）；房间有这条记录时，这段会话以模型走完，harness 从 `/new` 开的下一段会话起生效。同一层同时写 `[model] name` 与 `[resident] harness` 在解析时即拒：一个房间的居民是模型还是 harness，要读者去猜就是配置写错了；城自己的写路径也写不出这样一份文件（`crates/city/Spec.lean` §8-4b）。**被否**：派活帧上加一个字段，那要动 wire，而居民是一个站着的身份（词汇表 Resident），不是每次派活现选的；梯子上的 harness 压过会话记录，那会让一段会话中途换居民（city D6）。

**接线四段**：①`agent_protocols`：`Roster::seat` 与 `Consented`、会话的取消与起进程的那一半（`crates/agent_protocols/Spec.lean` §8-19）；②`kernel`：`harness_reported`、`harness_answered` 两个种类与 `Evidence` 多收的一种；③`city`：`[resident] harness`（`crates/city/Spec.lean` §8-4）；④派活路径上的 harness run：`runtime::run::harness` 写它的每一行（`crates/runtime/Spec.lean` §8-52），`accounting::worker` 判居民、借树、起 harness、落地（§8-124）。
-/

/-!
## 8-124 harness 居民的派活路径（`accounting::worker::dispatching::harness`、`accounting::worker::driving::harness`）

§8-4e 定了一次 harness run 必须守的性质，本节是它在派活路径上的接线：判一次居民，借树，起 harness，驱动一个回合，落地。模型 run 的路径一字不动，两条路在 `agree_to_work` 分开，在池与 `land` 的入口汇合。

```rust
// dispatching —— agree_to_work 判一次居民，只有派活路径用
pub(super) enum Seat { Model(Agreed), Harness(HarnessSeat) }
pub(super) struct HarnessSeat { building: city::Building, rules: city::BuildingRules, harness: agent_protocols::Harness }
impl RunWorker { pub(super) fn agree_to_work(&mut self, at: &Assignment) -> Result<Seat, AxError>; }

// driving::harness —— 第二个驱动函数，与 drive_run 并列
pub(crate) type Prompting = Box<dyn FnMut(&str, &mut agent_protocols::Listener<'_>)
    -> Result<agent_protocols::Answer, AxError> + Send>;             // 一场开好的会话：一次 prompt，读到它结束
pub(crate) type StartHarness = Arc<dyn Fn(Harness, &Path) -> Result<Prompting, AxError> + Send + Sync>;
pub(crate) fn drive_harness<L: Ledger>(half: HarnessHalf, ledger: &mut L, context: DriveContext) -> HarnessDriven;
impl RunWorker { pub(crate) fn with_harnesses(self, start: StartHarness) -> RunWorker; }   // 测试的门
```

- **居民只判一次**：`agree_to_work` 在写任何东西之前得出 `Seat`。`Seat::Model` 带着原来的 `Agreed` 走原来的路；`Seat::Harness` 不选模型、不造适配器、不调 `choose_shape`，所以城恒不把 `[model] name` 写进一个点名了 harness 的层（`crates/city/Spec.lean` §8-4b 的复读仍在，那是第二道）。`Staged`、`Flown` 与 `Continuation` 各是两臂的枚举，池只调 `Staged::fly` 与 `land`，不看居民是谁。
- **判哪一个地址**：这次派活会开一间新房间时（楼地址，或带会话名），判它所在的楼；否则判地址本身。新房间自己那一层是空的，梯子的答案就是楼的答案；旧房间自己那一层的会话记录说的是那间旧房间，不是新开的这一间。「会不会开新房间」由 `dispatching::session` 一处回答，`session_for` 与这里读同一句。
- **harness 一臂的三条拒绝**，次序即调用方有权先听到的次序：词不点名任何条目答 `E_CONFIG_INVALID`，subject 是 `<地址>: <词>`，`nearby` 是 `Roster::words` 的词，恢复语给出点名它的那份 `CONFIG.toml`；楼是 confidential 答 `E_GATE_DENIED`（§8-4e 第 4 条）；派活点名了模型（`-m`）答 `E_CONFIG_INVALID`：居民是 harness 的房间不接一个模型，改点名要去 `CONFIG.toml`。三条都在写之前，拒绝之后磁盘与账本一字不动。
- **程序装没装，不在答应时判**：找程序的权威是 `bin::doctor::host::find_program`，accounting 看不见它，worker 接触宿主只经 `StartHarness`。起不来的 agent 由 `HarnessProcess::start` 在车道里答 `E_TOOL_UNAVAILABLE`，恢复语给出它的那一行命令；它发生在 `run_started` 之前，账本上只多一条 `worktree_opened`，与模型 run 的准备在车道里失败同形（见本节未决）。
- **车道里的次序**：先借房间的树（与评审楼同一个 `workbench::standing::lend_tree`，不论 `review`），再在那棵树里起 harness 并开会话，然后 `HarnessRun::open` 写开篇两行（`run_started.agent` 是 agent 的 id、它在 `initialize` 里报的版本与同意的摘要，`crates/kernel/spec/Event/Record.lean` §8-87），最后以房间的 brief 发出唯一一次 prompt：有 `JOB.md` 时是它的原文，没有目标的派活是任务本身（harness 没有前缀可放 `PRINCIPAL_BRIEF`）。
- **`Listener` 的四个回调**：`halted` 读这个 run 的截断：罩住房间的停摆（`driving::lane::scope_stopping`，与模型 run 同一条规则）或人对这个 run 的 `Cancel` 是 `Cut::Halt`，墙钟上限到了是 `Cut::Deadline`，先查墙钟；`cancelling` 调 `HarnessRun::cancel`；`report` 把 `Update` 穷尽映射成 `HarnessReported` 交给 `HarnessRun::report`；`permit` 照 §8-4e 第 9 条选，问与答各记一条 `harness_reported`。记不进账本的问答让这一问答 `cancelled`，失败在回合结束后照会话失败处理。
- **回合结束**：答出停止原因后，在城的那把 checkpoint 闸下把树 `wave_pre` 成检查点，交给 `HarnessRun::conclude`（`checkpoint_committed`、`harness_answered`、冻结）。会话在答出停止原因之前断了（`E_PROVIDER`、`E_WIRE_MISMATCH`），或检查点提交不了，`HarnessRun::abandon` 冻成 `Cancelled`，原错误照模型 run 的做法向上抛：账本得到判决，调用方得到诊断。会话与子进程在驱动返回时一起丢掉，丢掉即杀（`crates/agent_protocols/Spec.lean` §8-19）。
- **落地**（账本线程）：离开 backlog；冻成 `Done` 时城替它开请求：把树落成房间分支上的一个 commit，写 `pr_opened`，进请求簿，与居民自己 `pr open` 的那一臂是同一段代码（`reviewing::offer`），核与合并走已有的评审流程；这条分支已有一份请求在等时不开第二份，只记一行诊断；然后归还树，按 `Owing` 付账。模型 run 落地要放回的适配器、工作台与转录，harness run 一样都没有。
- **墙钟上限**：楼规 `RULES.toml` 的 `harness_minutes`（`crates/city/Spec.lean` §8-2），缺省 60 分钟。车道从 `run_started` 那一刻起算，到时按停摆同一条路走：先 `cancel_received`，再 `session/cancel`，读到 harness 答出 `cancelled`，冻成 `Limit`。一个既不说话也不结束的 harness 占着一条车道与一份内存（§8-46-3），上限是它自己让出车道的唯一一条路。
- **缝**：`StartHarness` 的生产值是 `HarnessProcess::start` 在那棵树里起那一家；测试在管道另一头用一条线程扮演 agent，经 `Lines::over` 与 `AcpSession::open` 开会话，与 `agent_protocols` 的会话测试同法，不起真的 harness。用闭包而不立 trait：第二个实现是测试的，只有一个调用方。

**未决**：

- **程序的预查**：要在答应时就拒一个不在搜索路径上的 harness，worker 需要一只找程序的手，生产接线在 `bin::assembly`（`listening`／`production`）里交 `doctor::host::find_program`。要证明的是：`StartHarness` 在车道里的拒绝晚于开房间与写 `JOB.md`，是人能看到的一间空房间。
- **截断的缘由**：`cancel_received` 的载荷是空对象，一次冻成 `Limit` 的取消是墙钟上限，冻成 `Cancelled` 的是停摆或人。要在那一行上直接读出缘由，kernel 的事件表要给它加键，归线协议的车道。
- **steer**：ACP 在一个回合中间没有给 agent 的消息，人发给 harness run 的 steer 被取走后只记一行诊断。可选的答案是回合结束后把它当下一次 prompt 发出，那要一个 run 有第二个回合。
- **分支会话**：从另一段会话分出来的房间，第一次 run 若是 harness，它继承的那段对话没有地方放：ACP 的 `session/new` 不收历史。今天这段对话被丢下，`run_forked` 仍在账本上。
- **进程树**：四家经 `npx`，Windows 上是 `npx.cmd`，丢掉句柄杀的是直接子进程；一家不读 stdin 结束的 harness 会留下 node 进程。证据是起一次真的 `npx.cmd -y pi-acp@0.0.34`、丢掉句柄后看进程表；Windows 上收整棵树要 Job Object，按 AGENTS 的平台调用顺序另开一张卡。
- **城开请求用的门**：`collab::PrDesk` 没有让城放一个 `Opened` 效果的公开面，所以落地调 `reviewing::offer`，不经 desk。collab 的 SPEC 迁到 Lean 之后，要不要给 desk 一扇「城替居民开」的门，由它的 SPEC 定。
-/

/-!
## 8-133 一次派活的运行策略在派活路径上走到哪里（`accounting::worker::dispatching`、`accounting::worker::workbench`、`accounting::worker::reviewing`）

`crates/kernel/Spec.lean` §8-77 定了运行策略的四个值，本节是它们在派活路径上的接线：从帧到账，到写门，到这次 run 写在哪棵树里，到合并。

```rust
pub(super) struct Assignment { /* …既有字段… */ pub(super) policy: kernel::RunPolicy }
pub(super) struct Knock { /* …既有字段… */ pub(super) policy: kernel::RunPolicy }
impl Site {
    pub(in crate::worker) fn name_tree(&mut self, at: &Assignment) -> Result<(), AxError>;
    pub(in crate::worker) fn place_tree<L: Ledger>(&mut self, at: &Assignment, placing: &Placing<'_>, lines: &mut Stamping<'_, L>) -> Result<(), AxError>;
}
```

- **帧到账**：`Command::Dispatch.policy` 原样进 `Assignment.policy`，经 `RunPlan.run_policy`（harness run 经 `Chartered.policy`）由 `runtime::run::Charter::open` 写进 `run_started.policy`。城自己派的活——计划节点、追的目标、编辑器经 ACP、控制台与 `sprawling dispatch` 这个动词——取 `RunPolicy::of(Mode::Work)`；委派、交回与敲门叫醒继承说话那一方的整份策略，所以一次只读可新建的试验不会因为交给了别的居民就放宽或落地。
- **写门**：`workbench::tools` 用 `Assignment.policy` 开一个 `PolicyCell`（`crates/runtime/src/mode.rs`），把它的 `PolicyReader` 交给 `EditTool` 与 `ExecSetup`，楼的写域照旧取自 `RULES.toml`，两道判定在每次写时各问一次（`crates/city/Spec.lean` §8-32、`crates/runtime/Spec.lean` §8-55、§8-62）。开始时的 `policy.mode` 决定目录里那一行（`Catalog::set_mode`），会话中换 mode 不改它。
- **会话中换策略**：`Command::ChangeRunPolicy` 让城经 `record_at` 写一行 `run_policy_changed`（`crates/kernel/Spec.lean` D21）；这一行落账之后，`change_run_policy` 经 `RoomQueues::post_policy` 把它放进那个房间的策略槽 `PolicySlot`（`crates/accounting/src/worker/rooms.rs`，与房间的 `Mailslot` 一同借给正在跑的 run，只留最后一条）；lane 的中断源 `ask()` 在问过人的那张台子之后、问邻居的 steer 之前取它，交给 run 作 `Interrupt::Policy`，run 在下一个 `SafePoint::BeforeWave` 取用（`crates/runtime/Spec.lean` D28）。同一房间的第二个 run 拿的是备用队列，它的策略槽没有人投；房间空着就只留在账上，由派活来读：`accounting::worker::folds::session::SessionOrigins` 记下每个房间这一 session 里最后一行 `run_policy_changed`（`session_opened` 清掉它；`record_at` 把城自己写的每一行也交给这个 fold，所以 `change_run_policy` 落账即记下），人派的活（`Command::Dispatch`）在这一份存在时用它作 `Assignment.policy`，没有时才用帧里的 `Command::Dispatch.policy`。权威是账上房间的这一行，不是页面的那一份：会话菜单（`client/src/views/world/session_menu.svelte`）与对话页把「session 开始之后最后一次改动，否则开场策略」当作房间的状态给人看，而 composer 发出的是这一页自己持有的策略（`client/src/ui.ts` 的 `policy`，从 `FIRST_POLICY` 起），它不知道账上有什么，也没有时间可比；让 client 去比要在页面上再算一遍账已经算过的事，是同一事实的第二个定义。代价是：人改过房间策略之后再在 composer 里换一个，这一 session 里仍是房间那一份说了算，要换就再改一次房间策略。城自己派的活不读这一份，仍按上一条取策略。一个 run 读到改动而在下一个 `BeforeWave` 之前离开，改动同样留给下一次 run，不另敲门：换策略不是要人回话的事。
- **试验写在自己的树里**：一次 run 在不在自己的工作树里写，由 `Site::works_apart` 一处回答——`landing` 为 `Experiment` 时恒是，`Ordinary` 时照楼的 `review`。所以没有评审的楼里，试验也借房间的树（与评审楼同一个 `lend_tree`，写 `worktree_opened`），它写的东西不出现在楼里；树与分支留着，人与居民都读得到。
- **合并时的准入**：`reviewing::settle_requests` 在 `PrEffect::Merged` 写 `pr_merged` 之前问 `runtime::admits(&ended, produced)`，`ended` 是那个 run 冻结时策略格里的值（`crates/runtime/Spec.lean` §8-54）：试验恒拒，常规落地按准入证据要求判；拒时写 `pr_rejected`，理由是拒词的两句。没有任何一条路把试验的树自动合并，所以「没有隐式落地」不靠一条额外的检查。
- 验收：`dispatching` 测试 `an_experiment_writes_in_a_tree_of_its_own_in_a_building_without_review`（没有评审的楼里，一次试验新建的文件不出现在楼里，账上有它的 `worktree_opened` 与带 `"landing":"experiment"` 的 `run_started`）；`commanding` 测试 `a_policy_changed_while_the_room_is_idle_rules_the_next_dispatch`（房间空着时改成 chat，下一次帧里带 work 的派活，`run_started.policy` 是 chat）。
-/

/-!
## 8-5 订阅额度走 harness，不走登录

本城没有登录命令，也不持订阅令牌（`crates/gateway/Spec.lean` §8-5）。一个人要用自己的 Codex、Claude Code、Grok Build、Kimi Code 或 Pi 订阅，是在那个 harness 里自己登录，再把它作为 harness 居民接进城；provider 页只收 API key。

- **旧账本照样折得回**：旧版本写过的 `login_started` 仍是一个事件种类，没有读者；`secret_captured` 行里的 `expires_at` 与 `<provider>-subscription`／`<provider>-renewal` 来源读入时照收，本城不再据此续期。
- **旧快照不接**：`StandingFolds` 少了到期表，`STANDING_FOLD_RULES` 随之换值，旧快照按版本不符从创世重折（§8-101）。
- **harness 页读名单与这台电脑**：`accounting::views::lines::harnesses_answer` 把 `agent_protocols::OFFICIAL` 的每个内置条目按随版本附带的目录快照抄成 `wire::HarnessLine`，`found` 经 `Views.programs` 问服务中的城交进来的查找（`crates/accounting/Spec.lean` §8-10）；生产的查找是 `bin::doctor::host::find_program`，与 doctor 找程序读同一条搜索路径（`crates/wire/Spec.lean` §8-52）。
- **provider 页先给厂商表**：`views::lines::known_hosts_answer` 把 `gateway::known_hosts` 逐行抄成 `wire::KnownHostsAnswer`，不加不减（`crates/wire/Spec.lean` §8-51）。
-/

/-!
## 8-12 prefix 自己带上它要求模型读的东西

**原因**（把四个段拼出来才看得见）：Building 段是 12 字节的地址，Run 段是 71 字节的 `cas:b3-…` 内容哈希。而 City.md 要求模型「read `RULES.toml`」「`FULL READ:` 给出你的 `JOB.md` 的路径」——**两句话指的东西一个都不在 prompt 里，而城里八个工具没有一个解析 `cas:`**。第三处：City.md 无条件说「你的第一条消息正好有三行」，这对没有 `JOB.md` 的主 Agent 是假的。

```rust
// bin::assembly（形状 4 适配器；四个段的填充点）
fn building_segment(city_root: &Path, addr: &Address, building: &Address) -> Vec<u8>;
// Building 段 = 地址 + `RULES.toml` + （若在）`<city>/AGENTS.md` + （若在）
// `<building>/AGENTS.md`。后两者是**项目自带的约定**，按同一句理由直接给而不让居民去取：
// 要先 fetch 才能遵守的规则，要么晚一回合遵守，要么不遵守。**只看两个固定地址**：城根
// 与楼根。城根是这座城围起来的那个工作区，它要么就是项目本身（楼是项目的一级子目录），
// 要么是装着几个项目的父目录（楼就是那几个项目），所以项目的约定只可能在这两层；
// 不向城根之上找、不认其它拼法、不做搜索——一个读者无法从规则推出来的地址，没人核得了。
// 次序由泛到专：工作区那份在前，楼那份在后，最后读到的是离活最近的那份。两份都排在
// `RULES.toml` **之后**并自述位次：二者会冲突（一份叫你跑测试的 AGENTS.md 遇上一栋
// 没有 `exec` 的楼），而城执行的是楼规。文件不在就不写标题：空标题会让居民去
// 遵守一份不存在的约定。
fn run_segment(city_root: &Path, building: &Address, brief: &city::RunBrief) -> Vec<u8>;
```

| 槽 | 装什么 | 稳定性依据 |
|---|---|---|
| City | 城里那份 `City.md` | 整座城不变 |
| Building | 地址 ＋ `.sprawling/RULES.toml` 全文 | 人写、任何写域够不到、整个 Run 不变 |
| Resident | `URBANITE.md`（或无身份那 106 字节）＋ catalog | 同一个 Resident 每次 Run 同样的字节 |
| Run | `Handoff.md`（写过的话）＋ 本次 brief | 每次 Run 一份 |

- **注入的依据是稳定性，不是重要性**。`Roadmap.md` 与 `Memo.md` **故意不注入**：Run 自己会改它们（`plan` 工具持有 roadmap 全文），冻进 prefix 就是第二个权威——模型会读到自己刚改过的旧副本。它们的正路是工具，不是 prefix。
- **Run 段的次序是「上一场留下的」在前、「这一次要做的」在后**：最后读到的东西是被执行的东西。
- **空白表单不占 prefix 字节**：`city::handoff` 认出还是模板原样的 `Handoff.md` 并答 `None`。依据是模板自己的括号提示行——写过的会话会把它们换掉。
- **内容哈希整个退出 prompt**。它在 Ledger 里记了两遍（`run.rs:126` 的 pin 与 `:141` 的 started），溯源不依赖模型看见它；`FULL READ:` 那一行随之消失。
- **CAS 的 pin 改钉 brief 的正文**，两条臂都钉：一次没人派任务的会话，pin 里是「说明没有人派」的那几句，于是 Ledger 的 `job` locator 恒解析得到 Run 段真正携带过的字节，而不是一个从未被写出的文件。

**本章测试**：一次真派活后，provider 收到的请求里含楼规原文（`confidential = false`）、含上一场的 Handoff 正文、含本次 Goal，且**不含** `FULL READ` 与 `cas:b3-`；一次无 Goal 的派活不写 `JOB.md`，请求里说出「working with the User directly」且不把人那句话包成 `Task:` 表单。
-/

/-!
## 8-13 一封信与一次敲门

**原因**：两位居民在同一栋楼里谈价，发信的那一跑连续五次 `signal pull` 等一封**在它自己那一跑里物理上不可能到达**的回信，最后以 `limit` 冻结；而收信人根本没有在跑。证据：同一条链上 `signal_enqueued` 落在 `run_frozen` 之后两行。

```rust
// bin::assembly
struct Knock { addr: Address, from: String, mode: kernel::Mode, chain: KnockChain }
impl RunWorker {
    fn knock(&mut self, signal: &Signal, speaker: &Address, mode, chain: &KnockChain) -> Result<(), AxError>;
    fn answer_knocks(&mut self);   // 成波排干，循环而非递归
}
```

**两种送达，分法是收信人在不在**：

| 收信人的状态 | 机制 | 落点 |
|---|---|---|
| 正在跑 | 信从门缝塞进去——发出时 `signal_enqueued` 经 relay 落账，记账线程展示这一行时把信投进他那份借出队列的 `Mailslot`，`SignalDesk::take_steer` 在下一个安全点取走 | 追在下一次工具结果末尾，前缀 `@发件人地址`，其后是投递那一刻发信 run 的状态（collab D10） |
| 没在跑 | 敲门——发出时投进房间队列，记账线程投递时就入 `knocks`（敲门携发信方的房间、run policy 与 `KnockChain`，`Flight` 在那一跑开着时记着它们），同一次展示之后 `answer_knocks` 为他开一跑，发信方还在跑；队列已空的敲门跳过，信已被读过 | 新 Run 的 brief，同样写明 `@发件人地址` |

- **取走不是消费**（collab D8）：安全点取走的信被那一跑拿着，下一次 `SafePoint::BeforeWave`（这一回合的 `model_returned` 已落盘）才写 `signal_consumed`；那一跑先离开，信回到队列最前，落地时为本房间敲一次门（`RunWorker::knock_for_returned`）。

- **人压过居民**：中断源先问人的命令队列（Cancel 再 Steer），空手才问本屋信箱。
- **属名不是装饰，是回信地址**：另一个 agent 的话氒不得以人的身份进窗口。类型已经把它变成判定（只有 `Steer::from_person` 写得出 `user`）；同一条规则延到敲门路上——被叫醒的一跑，其 brief 第一句就是「@X signalled you. This run exists because that signal arrived: nobody else asked for it.」。一份读起来像人写的 brief 会让每一封回信寄错地方。
- **敲门敲的是 Resident，不是一段已封存的对话**：冻结的 Run 是历史，历史只读而不叫醒；被开出来的是那个地址上住户的**一跑新的 Run**，它靠 `Handoff.md` 接住上一场——那正是为穿过一次冻结而造的那件东西。没有 `URBANITE.md` 的地址因此不敲：它是一间房而不是一个人，信就在那儿等到人派个住户过去。
- **叫醒有一个上限，但不是一份预算**：一条敲门链与一条继任链各有一个接力上限，住
  `accounting::worker::driving::owing`（`CONVERSATION_HOPS_MAX` / `SUCCESSION_HOPS_MAX`，§8-46-12）。
  上限管的是「没有人在里面的链条不许无限长」，不给一轮活定价，也不规定居民之间能谈多少轮；
  花多少仍事后从 Ledger 报出来，停一片仍是 `Halt`。§8-13 早先「不设叫醒预算」的口径
  以「刹车就在被链条堵住的线程上」为前提，H-04 之后这个前提不再成立。
- **一次对话一道底都没有**：**这座城没有金额上限，也没有回合上限**，那是决定而不是遗漏：什么时候停下来归对话里的居民，花了多少事后从 Ledger 报出来。从无调用方的 spend 门连同它判的 ladder 、以及 `DISPATCH_TURN_BUDGET` 一并删除（kernel 一侧见 `crates/kernel/Spec.lean` §8-12、本文 §8-40），刹车此后只剩 `Cancel`（停一件）、`Halt`（停一片），以及两条接力链各自的上限（§8-46-12）。
- **一个敲不成不连坐发件人**：叫不醒的人进诊断日志，不把发件那一跑的 dispatch 弄成失败。

**本章测试**：一位居民向另一位发信，无人再派活而收信人自己跑了一跑，且其 brief 里带着发件人的地址；向一个无 `URBANITE.md` 的房间发信不开任何 Run，信仍在队里。
-/

/-!
## 8-14 幂等键里的那个时钟

**原因**：同一跑里两次 `read` 被拒为 `this call was already made`，下一回合同一路径又读得干净。原因在一行里：`IdemKey::derive(&run_id, Seq::new(t.value()), call.name.as_bytes())`——

一、**它取了一个时钟**（回合的毫秒戳），而确定性第七条写着「IdemKey 恒不得源于时钟或随机数」；二、**它不含参数**，于是同一回合内对同一件工具的任两次调用归为一键——两次 `edit` 也会，而那是丢写。

现形：`(run_id, 本跑内的调用序号, 工具名＋参数 JSON)`。序号由闭包自己的计数器给，重放同一段历史得同一串键。两次不同的调用是两个键，都跑；同一个位置被重放是同一个键，去重正是为此而存在。
-/

/-!
## 8-16 读不了的计划不再被报成被人改过的计划

**原因**：`dispatch_in` 里三处把失败抹平成默认值。

```rust
let plan_text = std::fs::read_to_string(&plan_path).unwrap_or_default();          // 驱动前：喂给 ClaimDesk
let shelf = city::archive_index(&self.city_root, building.addr()).unwrap_or_default();  // 驱动前：喂给 ArchiveDesk
let on_disk = std::fs::read_to_string(&plan_path).unwrap_or_default();           // 驱动后：落盘前的 compare-and-swap
```

第三处最重。那一段的注释自述它存在的理由——「each effect is checked against the file **as it stands now** … the losing claim is dropped with a diagnostic instead of overwriting somebody's row」。但读失败使 `on_disk` 成为空串，`still_true` 对空文档恒为 `false`，于是每一条 claim 都落入 stale 分支，人收到的诊断是「row … moved before this run's claim landed」——**一个从未发生的并发冲突**。他们会去查另一个居民，而真正要修的是一个读不开的文件。

（更重的那个后果——读失败→`stale` 为空→`write_plan` 覆盖真实计划——不成立：空文档下 `check_roadmap_shape` 不产 `WellFormed`，`still_true` 因此恒返 `false`。不存在数据丢失，只存在误报。）

第二处：`city::archive::index` 自己已经实现了正确契约（目录不在 → `Ok(空)`，真失败 → `Err`），所以 `.unwrap_or_default()` 恰好只扯掉真失败；换成 `?` 即可，不需新机制。

**现形**：

- accounting worker 里的 `plan_path` 函数删除。它在 `city` 之外拼了一遍 `city_root/<addr>/Roadmap.md`，而 `ROADMAP_FILE` 住在 `city::spine_files`——两份「计划在哪里」的权威。改走新增的 `city::roadmap_path`。
- 两处读全走 `city::roadmap`：仅 `NotFound` 答空串，其余以 `E_STORAGE_FATAL` 上报并带路径。一栋还没铺计划的楼确实没有计划，那不是失败；其余一切都是。
- `archive_index(…).unwrap_or_default()` → `?`。

**拒而不是降级**：计划是共享地面。读不到它就开跑，会花掉一次模型调用去产生一批注定被丢弃的 claim。在派活口上拒，人拿到的是路径和修法。

**红**：向一栋 `Roadmap.md` 是**目录**的楼派活（`read_to_string` 因此以非 `NotFound` 失败，无需权限把戏）。改动之前：派活成功，诊断行说「row moved」。改动之后：派活被拒，错误点名那个文件。

**影响面**：`city` 公开面增两项（基线同提交更新）。正常楼不受影响——`spine_files::lay_out` 给每栋新楼都铺了 `Roadmap.md`，而未铺的情形仍走 `NotFound` 答空串这条。`assembly.rs:219`（楼页读 Roadmap）同属一族但爆炸半径不同——那里读不到只是页上少一块，不会变成误报——此处不动。
-/

/-!
## 8-17 一次验证遍历，三个折叠

```rust
pub(crate) struct Standing { pub(crate) book: gateway::EndpointBook, governance: views::Governance, collaboration: Collaboration }
impl Standing { pub(crate) fn fold(ledger_dir: &Path) -> Result<Standing, AxError>; }
// serve 的那一遍：同一份已验证记录，逐条先给 Views 再给 Standing
pub(crate) fn fold_city(ledger_dir: &Path, cost: &mut OpeningCost) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // cost 见 §8-121
impl RunWorker { pub(crate) fn holding(city_root: &Path, vault: Custodian, log: Diagnostics, held: (JsonlLedger, Standing)) -> Result<RunWorker, AxError>; }

// views/governance.rs —— 判定面与读面共用的那一个定义
impl Governance { pub(crate) fn empty() -> Governance; fn absorb(&mut self, record: &EventRecord); }
struct CollaborationFold { … }   // 暂存 enqueued／consumed，`settle` 产 Collaboration
```

`rebuild_book`／`rebuild_governance`／`rebuild_collaboration` 三个函数删除。

- `Standing::fold` 与视图的起步（`Views::rebuild`，经 `views::snapshot::start`）折叠的是 `runtime::replay::fold_ledger_dir` 逐行借出的已解析记录，不对原始行调第二次 `EventRecord::parse_line`。理由有二：同一行只解析一次；更要紧的是，逐行检查放行的 `ig: true` 未知种类行（`VerifiedLine::IgnoredUnknown`）在第二次解析时会失败，于是一份能通过验证的历史却起不了城。两个折叠对 `IgnoredUnknown` 都跳过，与 `runtime::fork` 的读法一致。由 `a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary` 判定。
- serve 只验证并折叠历史一遍：`fold_city` 在 serve 线程上先 `JsonlLedger::open`（取得写锁、修复撕裂尾），再经 `runtime::replay::fold_ledger_dir` 逐段流式读一次，每条已知记录先给 `Views::apply`、再给 `Standing` 的折叠；打开的账本与折好的 `Standing` 作为一个值经 `assembly::attending::Opening` 交给 `RunWorker::holding`，worker 线程既不打开账本、也不读史。两个折叠对同一份记录作答，所以 worker 判定用的治理与页面读到的治理出自同一遍，不会因两遍之间账本变了而不等。`RunWorker::new`／`over` 自己折，它们的调用者（genesis、命令行、测试）手里没有现成的 `Standing`。`Standing` 是 worker 判定的依据，所以它必须在写锁之下折：锁先于读史取得，别的进程在折叠与打开之间追加的一行（例如 `CityHalted`）就无从落在 worker 的治理之外。serve 要求账本目录已存在（`init` 建好的城市），`JsonlLedger::open` 对缺失目录报错。
- `Standing::fold` 与视图的起步同走 `fold_ledger_dir`：三处启动折叠都不同时持有全部原始行与全部记录，常驻的只是一段字节与一条记录。
- `LedgerIndex` 在 `fold_ledger_dir` 的同一遍里建：`fold_city` 与视图的起步把那一遍返回的索引交给 `Views::hold_index(index, ledger_dir)`，`Views::new` 与 `Views::over` 只放一个空索引，不另扫一遍历史；没经过折叠的 `Views`（测试、命令行）在第一次查询前由 `refresh` 补齐。`Standing::fold` 不要索引，丢弃它。
- 尚未落地的部分：`Governance` 与 `EndpointBook` 仍在 `Views` 与 worker 各有一份。验收是 5 万条记录时首字节 ≤ 500 ms、l100k 启动峰值 ≤ 稳定值 + 8 MiB，由 `just bench-startup` 量。
- `Standing::fold` 与 `Views::rebuild` 折叠的是 `runtime::replay::VerifiedLedger::lines()` 里那份已解析的记录，不再对原始行调第二次 `EventRecord::parse_line`。理由有二：同一行只解析一次；更要紧的是，逐行检查放行的 `ig: true` 未知种类行（`VerifiedLine::IgnoredUnknown`）在第二次解析时会失败，于是一份能通过验证的历史却起不了城。两个折叠对 `IgnoredUnknown` 都跳过，与 `runtime::fork` 的读法一致。由 `a_city_opens_past_an_ignorable_line_from_a_newer_vocabulary` 判定。
- 尚未落地的部分：`Views`、`Standing` 与 `LedgerIndex` 合成一个 `CityFold`，在同一遍里建立，并让 `Governance` 与 `EndpointBook` 各只留一份；验收是 5 万条记录时首字节 ≤ 500 ms、l100k 启动峰值 ≤ 稳定值 + 8 MiB。这需要 `assembly::attending::spawn_worker` 把 serve 线程上折好的 `Standing` 交给 `RunWorker`，而不是让 `RunWorker::new` 再读一遍。

- **这不是缺陷修复**。三处实现漂移的假设（`rebuild_governance` 管 `granted` 与 `CityHalted`，`govern` 不管，`answer_approval`／`set_admission` 各自直改字段）不成立：新测试 `what_a_worker_holds_is_what_a_restart_rebuilds` 否定了它——派一次活、发一条信号之后，活 worker 与重建结果逐项相等。那条测试因此不是这次的红，而是让合并安全的护栏；它同时把一条四处代码都依赖、却从未被断言过的形状-7 性质变成了可红的。
- **以测量收口而非以红转绿收口**，理由写在上一条：没有可咬的红，因为没有缺陷。实测（windows-x86_64、16 核，release，外部探针经 `sprawling` 的 lib 门驱动 `RunWorker::new`）：

  | 记录数 | 改前 | 改后 |
  |---:|---:|---:|
  | 5,000 | 136.6 ms | 44.6 ms |
  | 20,000 | 538.7 ms | 175.6 ms |
  | 50,000 | 1856.1 ms | 436.2 ms |

  这是每一次 `serve`、`resume`、`fork`、`adopt` 都要付的钱。
- **为何不是 4 → 1 而是 4 → 2**：`RunWorker::new` 自己还要 `JsonlLedger::open`（尾部恢复）读一遍，而 `serve` 另走 `Views::rebuild` 一遍。把 `Views` 也并进来要改 `serve` 的所有权形状（它住在 `Arc<Mutex<_>>` 里与查询侧共享，而 worker 在自己线程上）——那属于 `dispatch_in` 的拆分，不在此顺手做。
- **暂存只给真需要的一项**：`CollaborationFold` 只暂存 signals，因为队列是 `enqueued` 减 `consumed` 而两者到达顺序任意；book、governance、goals、requests 都是逐条即结的，所以不暂存。
- **验证不动位**：链验仍在折叠之前。一部不能自证的历史，不是这三个视图中任何一个可以建在上面的历史。
-/

/-!
## 8-18 审查中的 run 不再把自己的决策直接放上楼的书架

**原因**：`dispatch_in` 的档案回收段写的是 `city::file_archive(&self.city_root, …)`，而不是 `&write_root`。ARCHITECTURE.md §5 把输线设计写在明处——「A building under review gives every run its own tree … Nothing it writes is visible until somebody else checks it — the losing line of the design made physical rather than promised」。档案不在那棵树里。

具体危害：书架回头喂给 `ArchiveDesk`，成为模型看到的「这栋楼已经知道什么」。一个**被驳回**的 PR 里的决策因此会留在架上，变成后续 run 的前提。

**现形两步，缺一不可**：

1. `city::file_archive(&write_root, …)`——落进检查点。
2. 持有租约时，`checkpoint_scope` 从**房间**改为**楼**。否则第一步把泄漏换成了静默丢失：`wave_pre` 只暂存 `<scope>/*`，而档案在 `<building>/Archive/…`，在房间作用域之外，不会进提交，租约一释放就没了。在**自己独占的** worktree 里暂存整栋楼是安全的：那棵树里变化过的东西全是这个 run 的。无租约时检查点仍在房间——那才是一个 run 在城里唯一可写的地方。

**与计划的不对称是故意的**：`Roadmap.md` 恒写回城里，因为它是共享地面且带着对当前文件的 compare-and-swap（那段注释自述了理由）。档案没有这样的声明，也没有守卫，所以它是漏而不是决定。

**四处检查点调用点**：（`workbench/standing.rs` 的 `ensure_base`、`workbench/tools.rs` 的 `with_checkpoint`、`driving.rs` 的逐波检查点、`reviewing.rs` 的 `PrEffect::Opened`）都从 `Site` 与 `RunWorker` 手上凑齐一个 `storage::Provenance`（run id、resident 地址、选中的模型 id 与思考档位、城的创世哈希）交给 storage；城的创世哈希由 `storage::Provenance::city_of(ledger_dir)` 只读账本首段第一行得出。

**红**：在 `review = true` 的楼里派一次带 `archive` 工具调用的活，断言书架仍空。改动之前它拿到 `[Entry { kind: Decision, … lab\Archive\decision\… }]`。同一条测试接着让第二位居民检查并合入，断言书架变为 1 条——**两半同一条测试**，因为只测前半的修法可以是「干脆不写」。
-/

/-!
## 8-19 沙箱接上

**原因**：`dispatch_in` 把 `Box::new(runtime::AbsentSandbox)` 写成字面量，而 `crates/sprawling/Cargo.toml` 没有任何 feature 到达 `runtime/wasm`。于是 `runtime::WasmtimeSandbox` 在 `runtime/tests/sandbox_a10.rs` 之外**没有调用方**，任何 sprawling 构建都到不了执行引擎。

`AbsentSandbox` 的三段式拒绝里写着 recovery：「use the program arm, or install a build with the `wasm` feature」。**那样的构建不存在。** 只写在文里、无人执行的 recovery 等于没有 recovery——这次是只写在错误消息里、无人可安装的构建。

ARCHITECTURE.md §2 把 wasmtime 列进技术栈并声明了代价（「Cost: an optional feature; a build without it refuses tool execution in three parts rather than pretending」），措辞预设了存在带该 feature 的构建。

**现形**：
- `crates/sprawling/Cargo.toml` 增 `[features] sandbox = ["runtime/wasm"]`。
- 引擎的选择收进一个函数 `execution_engine()`，两条 `#[cfg]` 臂各一个实现，`dispatch_in` 的构造点因此不随 feature 改变形状。
- **带引擎的构建起不来引擎就拒派活，不回落**。回落是「人以为跑在沙箱里、其实没有」的由来。

**默认仍为关**：这是 ARCHITECTURE.md 记录过的取舍（wasmtime 是一大块二进制），不改默认，只让开关存在。`just check` 走 `--all-features`，所以带 feature 的那条臂进门禁；`just dist` 不带，所以交付形态与体积预算不变。

**红是编译红而非行为红，这里说明白**：改动前 `execution_engine` 不存在，测试连编译都过不去。行为面的红需要一个真的 `python.wasm` 与 `SPRAWLING_PYTHON_WASM`，那是交付形态的事，不在此范围内。新测试只在 `cfg(feature = "sandbox")` 下存在，断言引擎给出的不是「this build carries no execution engine」那句话。
-/

/-!
## 8-20 交接件读不了不再等于没有交接件

`city::handoff` 的 `.ok()?` 与 §8-16 修掉的那三处同族，且它喂的是 **prefix 的 run 段**——下一次会话读到的第一样东西。三件事（文件不在／读不了／仍是空白表单）原先并为一个 `None`。

现形与 `roadmap` 同：`Result<Option<String>, AxError>`，`None` 只说「没有值得带走的东西」，`NotFound` 归入其中，其余上报并带路径。同时补 `handoff_path` 与把 `HANDOFF_FILE` 转 `pub`——红测要点名那个文件，而在别处拼一遍文件名就是第二份权威。

`run_segment` 因此转为 `Result<Vec<u8>, AxError>`；它只有一个调用方（prefix 的四段装配），所以波及面就是那一处 `?`。

**红**：向一栋 `Handoff.md` 是目录的楼派活，`expect_err` 撞上 `Ok(())`。`city` 侧另加一条单测，把三件事排成三行断言。
-/

/-!
## 8-25 一个答复接上的活，不靠重读全部历史找到，也不丢掉它的天花板

```rust
// Governance —— 现在是 RunWorker 的一个字段，而不是四个散字段加一份重写
struct Governance {
    pending: BTreeMap<String, ApprovalItem>, autonomy: Autonomy,
    granted: Vec<ClusterKey>, halted: BTreeSet<kernel::event::Scope>,
    rules: BTreeMap<(kernel::event::Scope, GoverningDocument), B3Hash>, // 从 rules_changed 折；每份治理文档上次记下的摘要（§8-40）
    origins: BTreeMap<String, BlockedJob>, // 从 approval_requested 折；答复时 O(log n)
}
impl Governance {
    fn absorb(&mut self, EventKind, RunId, Option<&Address>, &Payload);
}
/// 一项待批阻着的活：提它的房间与 run。那一跑被派去做什么，答复时从它的 `run_started` 读回。
struct BlockedJob { addr: Address, run: RunId }
```

**三个原因，一个改动**（§8-23 留下的那一半）：

1. **`blocked_job` 扫全史**。每次审批应答都 `verify_ledger_dir` 一遍再解析两遍，只为找 `(addr, task, goal)`，随历史线性增长。（量级取自 §8-17 留下的同类读数：`verify_ledger_dir` 约 215k 记录/秒，于是 50k 的历史光验链就是百毫秒量级；没有重测。）
2. **天花板归零**。`fn dispatch` 写死 `BudgetCap::default()`，而它正是审批应答后续活走的那条路。一跑带着天花板派出、因待批停下、被批准后续上的那一跑，向模型报 `0 usd_micros, 0 tokens`。
3. **`self.pending.remove(item)` 先于落账**，与 `set_admission` 相反，且是冗余的——`record → govern(ApprovalResolved)` 本就移除它。`self.granted.push(…)` 同理。

**为什么三件一起改**：它们是同一个结构问题的三个面。治理状态本来有两份实现：`Governance::absorb`（重启折）与 `RunWorker::govern` 加上 `set_admission`／`answer_approval` 里直改字段的几行（活折）。§8-17 测过两者不漂移，但那只是当时恰好相等；**再加一份 origins 折就是第三份**。这里把四个散字段换成 `RunWorker.governance`，`govern` 就是 `absorb`，于是新的两张表只有一个折法。

**选甲而不选乙**。乙案是让 `approval_requested` 的 payload 自述所阻之活；但那份 payload 就是 `ApprovalItem` 本体，改它得改 `kernel::ApprovalItem` 的公开面与每一个构造点。更重要的是：**账本已经说得出一项是哪一跑提的**（envelope 的 `run`），它没说的是那一跑被派去做什么、在什么天花板下。那是 `run_started` 的事，不是每一项待批的事。

**一个未决问题，用测试回答了**：甲案是进程内存，重启后的 worker 从账本重建，那「重启前提出、重启后才批」的项接不接得上活？答：接得上，因为 `origins` 就在 `Governance` 里，而 `Standing::fold` 对每一行调的正是 `absorb`——与 `pending` 同一折、同一遍。`what_a_worker_holds_is_what_a_restart_rebuilds` 增一条断言盯住它。

**进程里不留每个 run 被派去做什么，答复时从账本读回**。`origins` 只记一项待批由哪个房间、哪个 run 提出；人答「允许」时，`answer_approval` 先刷新账本旁索引，按索引从这个 run 最早的一行往后读，读到它的 `run_started` 就取出 `task` 与 `goal`，再在原房间续上这件活。读回走 `storage::LineReader` 的定位读，读到的页留在操作系统的文件缓存里，进程里不另存一份（`crates/sprawling/spec/Serving/Memory.lean` §8-173）。这个 run 的 `run_started` 不在账本里时（只有一段尾巴的历史），答复照常落账，不续活，与这项待批从来没有记下来源时相同。`origins` 在 `ApprovalResolved` 上裁，因为答过的项不再阻着任何东西。

理由：一张按 run 记 `task` 与 `goal` 的表每派一跑长一条，从不裁剪，而 `task` 是人写给这一跑的整段话，长度没有上界；冻结的 run 也一直占着它。它不能按 `RunFrozen` 裁：账本次序是 `RunStarted … RunFrozen … ApprovalRequested`，装配层在 drive 之后才记下待批项。被否的做法：①照旧留这张表——随 run 数无界增长，与「冻结的 run 不留任何东西」相反；②表里只留 `run_started` 的 seq——每跑仍留一条，只是短一些；③经 `storage::resident` 缓存读回——答复是人点一下的事，频率低，一份按字节计的缓存省下的只是一次定位读，文件缓存已经替它做了。代价：一次答复多一次索引刷新与几次定位读，与这个 run 的行数无关（读到 `run_started` 即停，它在 run 的头几行里）。三个平台相同：索引与读回都经 `storage` 的 `Vfs::read_at`，那是同一段 std 代码。

**读在落账之前，派活在落账之后**：`answer_approval` 先取一份 `origins`（读，不是变化），再落 `approval_resolved`（它自身就是关闭动作，`absorb` 随之丢掉 pending 与 origin），最后才派活。与 §8-24 同一条规矩。

**红（两条）**：

- `work_resumed_by_an_answer_is_done_under_the_ceiling_that_sent_it`：以 `BudgetCap { usd: 250_000, tokens: 4_000 }` 派一跑，它读一次 `status`（对照组），然后提一项待批；批准后续上的那一跑再读一次。**两跑同地址**，所以依据不是 `addr:` 而是该地址上读到的天花板去重后的**集合**：改动之前是两个值（`0 …` 与 `250000 …`），之后是一个。数请求体不能作依据（§8-23 已记）。
- `what_a_worker_holds_is_what_a_restart_rebuilds` 增一条：活 worker 的 `origins` 与 `Standing::fold` 重建的逐项相等。改动之前 `origins` 不存在，是编译红。

**性能以结构收口而不以计时收口**：一次审批应答从「验链一遍加解析两遍全史」变为一次 `BTreeMap` 查找；`blocked_job` 连同它的两个循环一并删除，因此这不是一个快了多少的问题——那条路径不存在了。

**影面**：`runtime` 公开面增一字段（`RunPlan.budget`），基线与 `crates/runtime/Spec.lean` 同提交；`RunPlan` 的三个构造点（assembly、citysim、runtime 集成测）各加一行；`tools/fixtures/golden-p0` 重生。
-/

/-!
## 8-26 读不到一份文件不等于那份文件写错了

```rust
fn city_segment(city_root: &Path) -> Result<Vec<u8>, AxError>;  // NotFound → 内置副本；其余 → Err
```

§8-20（交接件）与 §8-16（计划）定下的形制是：**「还没有」答默认值，「读不了」带着路径上报**。这里收尾同族剩下的两处。

**一、楼页把「读不开」报成「表写错了」**。「读不到只是页面少一块，不会变成误报」不成立。`read_building` 把读失败抹成空串，而 `check_roadmap_shape("")` 并不返回空结果：`header_seen` 为假使它推出 `Malformed { problems: ["no four-column table found"] }`。于是页面向人断言一件它无从得知的事：那张表的形状不对。人于是去修表格，而要修的是一个打不开的文件。

**现形**：改走 `city::roadmap`（§8-16 立的那扇门），读失败时**把失败本身放进 `problems`**——那正是这个字段的用途，也是页面已经会画的东西。`read_building` 不改返回类型：`None` 的意思是「没这栋楼」，把「计划读不了」塑成那个形状会让一栋存在的楼从城里消失。

**二、关城时把零字节当成城的规范**。`close_city` 的 `std::fs::read(&city_file).unwrap_or_default()` 使 must-read 指向空字节的 CAS 哈希：下一任被告知「先读这份」，读到的是什么都没有。

**现形**：不新建读法，改用同文件已有的 `city_segment`——「这座城的规范是什么」应当只有一个答案，而 prefix 装配已经在问同一个问题。同时把 `city_segment` 自己改成同一形制：它原本的 `unwrap_or_else(|_| CITY_MD)` 注释自述为「falling back to the built-in copy **when a city predates it**」，而那只描述了 `NotFound`；其余失败下它静默地拿内置副本冗作人编过的那份，而两份可以完全不同。修后：`NotFound` 仍答内置副本（那是已记录的契约），其余一律带路径上报，于是一跑在读不了的城规范下开跑这件事也一并没了。

**关城于是会失败，这是有意的**。一次说不出下一任该读什么的关闭不是一次有序关闭；`serve` 的循环已经写着 `eprintln!("the city could not write its handoff: {err}")`，于是人在终端上拿到路径与修法，而不是一条指向空白的交接件。

**红（两条，各咬一处）**：把 `Roadmap.md`／`City.md` 各做成**同名目录**（§8-20／§8-16 用过的手法，不碰权限，在 Windows 上稳定）。一：楼页的 `problems` 必须点名 `Roadmap.md`——改动之前它说的是 `no four-column table found`。二：`close_city` 必须以点名 `City.md` 的错误拒绝——改动之前它返回 `Ok` 并写下一条指向空字节的 must-read。
-/

/-!
## 8-27 一次登记喂到两处，于是只写一遍

**原因不是缺陷，是两个权威**。`dispatch_in` 里目录准入与工作台注册是两份各十三行的名单，而同一段的注释自述「one registration feeds both」。两份名单今天相等，但相等是人维护出来的：只上工作台的工具是没人能叫的工具，只上目录的工具是告诉了模型、叫下去却不存在的工具。

**现形**：一个 `Vec<Box<dyn kernel::Tool>>`，一个循环里先 `admit_tool(tool.meta())` 再 `bench.register(tool)`。順序取**目录的**那一份：`Catalog::render` 按准入顺序把工具摆在模型面前，而 resident 段是要算哈希的，所以这个顺序是缓存面的一部分。十三件的次序逐字照旧代码排（archive、exec、claim、edit、status、signal、goal、pr、delegate、workshop、rules、neighbours、read，然后 MCP），故字节不变。

**它以什么收口**（照 §8-17 的写法）：**没有可咬的红**，因为两份名单今天并未漂移，十三对十三逐项相等。一条「两集合相等」的断言今天就绿，而且改完之后它恒绿（不可能不相等），那不是测试而是装饰。收口在于：变化后两份名单不可能不相等，且 141 条现有测试（包括多条断言工具名与 prefix 内容的）全绿。**不为了凑一个红而补一条前后都绿的测试。**

**尺寸不是这次改动的理由**：`dispatch_in` 983 → 977。五十行准入换成四十五行名单加循环，净值接近零；换来的是一个权威而不是两个。

**一个曾被推翻、现已了结的假设**：`invoke` 里 `match bench.invoke(…)` 的 `_ =>` 臂当时看似死代码，删掉却得 `E0004`——`BenchOutcome` 带 `#[non_exhaustive]`，本 crate 在它定义的 crate 之外，穷尽匹配不可写。那一臂因此保留并注明不可达。

——**该接口问题已结**（G-22 / 叶子 7.10）：`runtime` 不发布，工作区之外没有第三方，`#[non_exhaustive]` 只换来每个下游一条永不执行的分支。全库枚举现已撤下该属性，这一臂与同类的四十余条一并删除，穷尽性回到编译器手里。
-/

/-!
## 8-28 一次调用的键，只有一份读法

```rust
// driving::placing::Placing::admit 内：六行手写的动作字节换成一个问句
let key = match call.action() { Ok(action) => IdemKey::derive(&self.run, Seq::new(at), &action), Err(e) => return Admitted::Answered(Err(e)) };
```

**这里不修 `bin::assembly` 的缺陷，因为这里没有缺陷**。被修的是 citysim（`tools/citysim/Spec.lean` §8-4、citysim D20）；本文件变的是「谁来回答动作字节」。原先这六行把 name 与 `serde_json::to_string(&call.args)` 拼起来，是全库两份实现中对的那一份；对的那一份待在装配层，正是另一份能静静漂走的原因。`crates/kernel/Spec.lean` §8-6 早写着这条规则「属 S2 工具面」，而它一处也不在那里。现在它在（`ToolCall::action`，`crates/kernel/Spec.lean` §8-23），本文件改为问它。

**字节逐字不变**：`action()` 内部就是搬过去的同一句（name 字节接 args 的 JSON 字节），位次是 `Placing` 在放行时按调用序数出的计数器。唯一的行为差异是 `unwrap_or_default()` 换成 `?`：一个序列化失败以前产空串（于是两次参数不同的调用得同一把键），现在上报。`Payload` 拒浮点且键恒为字符串，故这一臂今天不可达。

**一个被推翻的假设**：「让 `ToolBench` 自己持 run 与位次、`invoke` 内部铸键」看上去是更好的形——传钟进来这件事就没有参数可传。它不成立：`ToolBench::seen` 恒从空集起，且键在过门之后才记入（`crates/runtime/Spec.lean` §8-14），所以一个恒递增的内部位次会让键在一次驱动内永不重复，`BenchOutcome::Duplicate` 随之变成**任何门都达不到的变体**，`turn.rs` 那条 `dedup_runs_before_the_side_effect`（同键调两次、断言文件未再变）连同它守的不变量一起写不出来。**把一个可测的防御换成不可测的死代码，不是加固。** 位次因此留在调用方。

**顺手记下、这里不动的一件事**：dedup 是一道**今天接不到任何东西的防御**。`kernel::idem` 自述它存在是为了「resume 与 replay 重派出同一把键」的双付防御，而 `seen` 从不从历史播种，`sprawling resume` 也不重跑一跑（ARCH §5 末：它只验链、把丢了结果的调用关成 unknown、并报告等人的事）。此后，`Duplicate` 在两个驱动器里都不会再出现，而这是**对的**：它本就是重放路径上的结果。要让它真正接上，得让 `seen` 从账本重建——那是另一件事，它自己的红在「重建后的 worker 不会把已经付过的钱再付一遍」上。
-/

/-!
## 8-29 行没落下，城就没动

```rust
impl RunWorker {
    pub fn new(city_root, vault, log) -> Result<Self, AxError>;              // = open ➕ over
    pub(crate) fn over(city_root, vault, log, ledger: JsonlLedger) -> Result<Self, AxError>;
}
```

这是 §8-24 那条性质的另一半。§8-24 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）；这里问的是「**行没落下，城就没动**」。

**选甲而不选乙**。一种说法是甲案（`RunWorker::over`）「只有一个生产调用方，近乎为测试拓宽」，而乙案（倒置 `kernel::Ledger`）才是 ARCH 点名的那类动作。两头都不对：

- **甲不是测试拓宽，是 ARCH §3 自己提的那条批评**。§3 末段写着 `RunWorker`「builds its model adapter **instead of receiving one**」，并把它列为 V6 停在装配层下方的原因。同一句逐字适用于账本：一个自己 `open` 账本的 worker 同样无法被驱动到第二份实现上。把「账本从哪来」从构造子里取出去，是把一个不属于它的决定交回给调用方。
- **乙今天买不起**。`RunWorker` 对账本用的不只 `append`，还有 `position()`（两处）与 `observe()`。把 `observe` 推上 `kernel::Ledger` 等于让最内层去定义什么是「耐久后通知」——那是持久化适配器的事，不是「一个 Ledger 是什么」的事；代价是全库 **七个 `impl Ledger`** 各长出一个它们不需要的方法，加 conformance 套件。而这里根本不需要第二个类型：两条路上都是具体的 `JsonlLedger`，**不同的是它下面的 `Vfs`**。既然缝不必动，就不动。
- 丙（只在 `storage` 内写红）**已经存在**：`power_cut_matrix_over_every_op_keeps_acknowledged_waves`。它证的是账本自己的耐久契约，不是装配层的不变量，所以它不替代这一条。

**它以什么收口：没有红，照 §8-17／§8-27 的写法说清楚**。`a_line_the_history_refused_is_a_change_the_city_never_made` **首跑即绿**，因为这条性质 §8-24 已经用类型持住了：`record` 遇拒即 `?` 返回，`Then` 随之丢弃，改变无从发生。**未止步于一条前后都绿的测试**：把 `Landing::record` 的 `append(line)?` 改成 `let _ = append(line);` 后重跑，它当场红，且红在实质那条断言上——盘上的计划被写成了 `| 1 | wire the kiln | In progress |  |`，而宣布它的那一行从未落地。恢复后又绿。这条测试因此是一张网，不是一条红，而它能咬是量出来的不是声明出来的。

**测试里两个世界各归各位**：账本在 `FaultFs` 的内存平面上，城的文件（`Roadmap.md`）在真盘上。这正是要问的形状：被断言的东西是一份人事后真能去打开的文件。`Standing::fold` 仍读真目录（那是「城到目前为止知道什么」），而本跑新落的行进虚拟账本——两者不相干，因为断言不靠账本内容，只靠盘上那份计划。

**影面**：`storage` 公开面在 `fault` 下增 `open_faulty`、`FaultPlan` 增一字段（`crates/storage/Spec.lean` §8-2 同提交）；`sprawling` 公开面**不变**（`over` 是 `pub(crate)`）；`crates/sprawling/Cargo.toml` 的 dev-dependencies 打开 `storage/fault`，发行构建不含它。
-/

/-!
## 8-31 dispatch_in 向 ARCH §5 的十二步靠拢（逐次拆分）

**目标不是「把某一段搬走」，是「让 `dispatch_in` 成为 ARCHITECTURE.md §5 已经写好的那个序列」**。两者的区别是形状问题的生死：按行号切出来的一块叫不出 §9 的名字，而一个相位叫得出来——§5 已经给了它名字。

**第一次拆分：驱动（§5 步 7–11）。**

```rust
struct Driven {
    outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,   // 仍是 Result：跑败也要结桌子
    checkpointed: Vec<String>, ran: (u32, u32), raised: Vec<ApprovalItem>,
}
impl RunWorker {
    fn drive_dispatch(&mut self, plan, handoff, adapter, bench, signals,
                      write_root, checkpoint_scope, who, run_id) -> Result<Driven, AxError>;
}
```

三个钩子住在一起，理由不是它们相邻，而是**它们是唯一在驱动器持有账本期间碰账本的代码**（`invoke` 里那句自述：「the ledger is the driver's for the length of the run」）。它们收集的三样东西也只在那段时间里可写，所以一并作为 `Driven` 返回，而不是留四个 `Rc<RefCell<…>>` 让调用方自己保持同步——四个单元格是四个可以忘记读的东西，一个值不是。

工具面是 `accounting::worker::driving::placing::Placing`（形状：adapter），`crates/runtime/Spec.lean` §8-3 的 `ConcurrentInvoke` 在装配层的实现：

```rust
pub(super) struct Placing<'f> { /* bench、sieving、run、next、ran —— 私有；checkpointing: &'f Checkpointing */ }
impl<'f> Placing<'f> {
    pub(super) fn new(bench: ToolBench, sieving: Sieving, run: RunId, checkpointing: &'f Checkpointing) -> Placing<'f>;
    pub(super) fn ran(&self) -> (u32, u32);   // 本跑命令的 (通过, 失败)
}
impl ConcurrentInvoke for Placing<'_> { /* admit：定位次、派生键、bench.clear；tool：bench.tool_for；account：bench.account，再记 checkpoint 与命令计数、exec 结果过筛 */ }
```

键的位次在 `admit` 里定、checkpoint 记录与命令计数在 `account` 里记，两段都按调用序跑，所以一波开头的只读调用并行跑完之后，键、checkpoint 列表与计数与串行波逐字相同。位次与计数是 `Placing` 的普通字段：先前它们是闭包捕获的 `Cell` 与 `Rc<RefCell<…>>`，因为闭包只能借不能拥有。`checkpointed` 仍是一个 `RefCell`，借给 `Placing` 与波前 checkpoint 钩子两处写，因为清扫读的是**最早**立起的那道 checkpoint（`settling::desks`），两份各自的列表合不出时间次序。失败码不新增：`admit` 与 `account` 的失败原样是 bench 与工具的失败，回给模型。

`outcome` 刻意仍是 `Result` 而不在方法里 `?`：一跑失败了它的桌子照样要结，而结桌子正是把它最后几行放上历史的动作。把它提到方法边界上会静静跳过它们。

**尺寸**：`dispatch_in` 975 → **833**；`drive_dispatch` 171。尺寸不是这次拆分的理由（照 §8-27 的写法），但它是尺寸门的前提，而那道门的门限是量出来的 200。

**它以什么收口**：纯结构，无可咬的红——行为逐字不变（钩子体原样搬迁，`checkpoint_scope` 由计算改为传入）。143 条 `sprawling` 测试全绿，其中包括直接盯驱动行为的 §8-24／§8-29／§8-30 三条。

### 剩下的五次拆分

`dispatch_in` 今为 **833** 行（@3611），相位实测如下。目标 <200；每次拆分都是同一个形制：相位成为 `RunWorker` 的一个方法，多个活值归并为一个归位值类型（如 `Driven`），而不是一排得保持同步的局部变量。

已切九次，**975 → 158**，产出的方法均在阀值内：`drive_dispatch` 171、`settle_desks` 124、`settle_requests` 122、`conclude` 104、`stand_up` 92、`admit_reading_room` 32。三个归位值类型：`Driven`（驱动期间写、驱动之后读的四样东西）、`Desks`（一起出借、一起收回的五张桌子）与 `Site`（一次跑站在哪儿）。

**第七次拆分：桌子（§5 步 3–4）。**

```rust
struct Desks { signals, goals, plan, shelf, pr, plan_path: PathBuf, waiting: u32 }
impl RunWorker {
    fn open_desks(&mut self, site: &Site, addr: &Address) -> Result<Desks, AxError>;
}
```

**`pr` 与 `waiting` 入伙，`Desks` 的理由随之改写**。建 `Desks` 时写的理由是「一起结算」，而 `pr` 不与它们一起结（它等 `produced`，在 `settle_requests` 里）。但五张桌子**一起出借、一起收回**，而这正是它自己标题已经写着的那一句。理由换成出借，`pr` 于是入伙；否则它就是唯一一个被抛在值外面、靠人记得的桌子。`waiting`（`lent.pending()`，`u32`）同理：它只能在队列交给桌子**之前**数，数不到就永远数不到了。

**一个名字在相位内改了**：原来的局部 `shelf`（`Vec<Held>`）与 `memory_desk` 在归位值里叫 `shelf`，于是前者改叫 `held`——一个名字对一个东西，而“书架”指的是那张桌子。

**尺寸**：`dispatch_in` 495 → **423**；`open_desks` 76。十行的 `Desks` 手工构造（原在驱动之前）随之消失：归位值由相位自己交出来，不再由调用方拼。

**它以什么收口**：纯结构，无可咬的红。五张桌子的构造顺序、采钟的位置、`inboxes.remove` 与 `pending()` 的先后均逐字不变。143 条 `sprawling` 测试全绿，其中 `a_signal_one_run_sends_is_read_by_the_run_that_pulls_it` 与 `a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke` 走的就是“队列借出去、再收回”这一支。

**第八次拆分：工作台（§5 步 6）。**

```rust
struct Workbench {
    catalog: Rc<RefCell<runtime::Catalog>>, bench: ToolBench,
    delegates: Rc<RefCell<collab::DelegateDesk>>,
}
impl RunWorker {
    fn lay_out_workbench(&mut self, site, desks, addr, depth, mode, budget, job_locator)
        -> Result<Workbench, AxError>;
    fn status_tool(&self, site, desks, addr, mode, budget, seen, delegates)
        -> Result<StatusTool, AxError>;
}
```

**`Workbench` 持目录而不持它渲出的 `tools`**：后者是前者的投影，两份都存就是同一件事的两个权威。`ToolBench` 路由一次调用，`Workbench` 是一次跑工作的那张台子——名字相邻而职责不同，差别写在 rustdoc 第一句。

**工具块需要的不是两个方法而是三个，这是量出来的**。原先写「工具块需要两个」（阅览室一个、剩下一个）；切完一量，`lay_out_workbench` **206 行**，越过将要执行的 200。纪律是「不为通过而放宽门」，于是再切一次。切在 `status`：它是十三件里**唯一一件要读 worker 治理状态（`governance.autonomy`）与目标登记册（`self.goals`）的工具**，也是唯一一件带活闭包的（`children` 读委派桌，因为一跑边跑边派活）。它回答的那个问题与周围不同：**这一跑对自己怎么交代**。

**两个局部变量随它走了**：`writable` 与 `neighbours` 原本只为 `status` 而算（`write_domain()` 本就在同一方法里被叫三次，多一次不改变任何东西），现在各自在 `status_tool` 内部算。

**尺寸**：`dispatch_in` 423 → **237**；`lay_out_workbench` 164；`status_tool` 51。

**它以什么收口**：纯结构，无可咬的红。十三件工具的**构造顺序与登记顺序逐字不变**——而登记顺序是缓存面的一部分（§8-27），prefix 字节一变即有测试当场发作。

**第九次拆分：冻结（§5 步 5）。**

```rust
impl Freezing<'_> {
    pub(super) fn freeze_plan(self, site: &Site, workbench: &Workbench, at: &Assignment, given: Given)
        -> Result<(RunPlan, runtime::handoff::Handoff), AxError>;
}
```

**两个值而不是一个新类型**：`RunPlan` 与 `Handoff` 类型不同、谁也不会认错，再包一层只是给元组取个名字。它们同属一相位的理由是读一遍就看得见的：prefix 为这份 plan 而装配并与它一同冻结，handoff 引的是 plan 自己的 `task_line`，而 job locator 两边都在。

**冻结的交接就是房间里那份**：房间的 `Handoff.md` 填过时（`city::handoff` 答 `Some`），它的原字节入 CAS 并排在 must-read 末尾，`overview`／`progress`／`next_step` 取 `city::handoff_sections` 读出的对应节，`context` 在派活来源与 transcript 地址之后接上该节。一节没写就写明「没有记录」（`NOT_RECORDED`），不再写「see the city roadmap」「resume from the job locator」这类指向别处的占位：那两句读起来像交接，其实一个字的信息也不带，而下一个读者会照着去找一份交接里从未提过的路线图。`overview` 在文件没写时仍取 plan 的 `task_line`，因为那是真事实。

**它以什么收口**：纯结构，无可咬的红。prefix 四段的装配顺序、must-read 的入列顺序（先 norms 后 job）均逐字不变；两者一变即有多条盯 prefix 字节与交接件内容的测试发作。143 条全绿。

**剩下四次拆分**（目标 <200，预计落在 ~160）：四个相位都还在 `dispatch_in` 里，四个都要切。

| 拆分 | 相位（§5 步） | 长度 | 归位值 |
|---|---|---|---|
| e | 规则／配置／选型／身份／租约（步 3） | ≈70 | `Site` |
| f | 五张 desk 的构造（步 3–4） | ≈75 | `Desks`（扩 `pr` 与 `waiting`） |
| g | catalog＋十三件工具＋bench（步 6） | ≈175 | `Workbench`（catalog、bench；delegates 与 workshop 两张桌子在 `Desks` 里，记账线程在调用时读它们） |
| h | prefix＋RunPlan＋handoff（步 5） | ≈90 | `(RunPlan, Handoff)` |

**依赖序即执行序，而且依赖是真的**：工具块读十五个局部（`write_root`／`rules`／`config`／`building`／`who`／`depth`／`model` …），先有 `Site` 才能让 g 收得下参数，而不是把十五个形参排成一列。

**第六次拆分：站位（§5 步 3）。**

```rust
struct Site {
    building: city::Building, rules: city::BuildingRules, config: kernel::FrozenConfig,
    model: gateway::ModelEntry, adapter: Box<dyn Model + Send>,
    identity: city::Identity, who: String, run_id: RunId,
    lease: Option<storage::WorktreeLease>, write_root: PathBuf, branch: Option<String>,
}
impl RunWorker {
    fn stand_up(&mut self, addr: &Address, job: &Locator,
                task: &str, goal: &str, budget: kernel::BudgetCap) -> Result<Site, AxError>;
}
```

**一个值而不是三个，理由是时钟而不是口味**。这一相位读上去是三件事（规则与选型、身份与登记、检查点与租约），而它们在代码里互相咀合：租约要 `run_id` 与 `who`，而 `run_id` 在选型**之后**采钟。拆成三个方法就得把身份块提到选型之前，那会改变 `run_id` 的时间戳落在哪一步之后——而这是纯结构改动，行为需逐字不变。**一个采钟点的先后不是重构可以顺手改的东西**（ARCH §10：全库只有一个采样点，它采到的值进了账本）。于是相位按原序整体搬迁，归位值一个。

**`Site` 不收 `addr`**：`Address` 是 `dispatch_in` 的形参，它在相位之前就在，放进去就是同一个值的第二份，故它不在字段里。

**归位值在调用点以整值接住（`let mut site = …`）而不拆开**，理由是量出来的：拆开之后，剩下三个相位要从调用点接过去的名字共 **102 处引用、十一个名字**（`building` 23、`model` 18、`who` 17、`write_root` 11 …），即每个相位方法都得排一列十五个形参——而那正是归位值要消掉的东西。`Driven` 可以拆，因为它四个字段只在驱动之后被读一次；`Site` 不行，因为它要穿过剩下每一个相位。

**尺寸**：`dispatch_in` 548 → **495**；`stand_up` 92。

**它以什么收口**（照 §8-17／§8-27 的写法）：纯结构，无可咬的红。搬迁逐字，唯一的改动是 `&addr`／`&job` 从局部变成形参，且两者在相位内部的用法不变；采钟点的个数与先后不变（`run_id_for` 一次、`ensure_base` 一次）。143 条 `sprawling` 测试全绿，其中 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯租约这一支。不补前后都绿的测试冒充红转绿。

工具那一块原为 214 行，先把阅览室（`admit_reading_room`）切出去，余下 ≈175 才能装进一个合格方法。**这正是阀值取 200 的一个副作用**：它不允许把一堆东西搬到另一处冒充分解。

**尺寸门须等这五次拆分完成**：纪律是「不为通过而放宽门」，所以门不能先落地再给自己开例外。门限与单位由全库测量定下，数字就写在这里：单位是**生产函数**（以首个 `#[cfg(test)]` 截断），门限 **200 行**。依据：1646 个生产函数中位数 9、p90 为 37、p99 为 114；超过 200 的只有六个，而其中五个是数据与标记（`web::lang::phrase` 是译文表，属 §9 形状 6；`Settings`／`CityView`／`BuildingView`／`Root` 是 Dioxus 组件，函数体即标记），故这两类需在门里声明为数据。排掉它们，全库超阀的生产函数只剩 `dispatch_in` 一个，第二名 `serve` 为 233——它也在网内，这是故意的，把门开到 240 去放它过就是为通过而放宽门。而按**文件**计不行：任何诚实阀值都会在四个 crate 里同时点燃八处（800 行阀 → 8 个文件），那是工程而不是一道门。提交须带 `Verdict: user-approved`。
-/

/-!
## 8-32 一座城的那一个写者，自己有个名字

```rust
fn spawn_worker(city_root, vault, vault_notice, log, views, to_clients, worker_desk)
    -> Result<std::thread::JoinHandle<()>, AxError>;
```

**为什么是它**：`serve` 233 行，是那道尺寸门报出的两个对象之一。三个相位里（存储与视图、写者线程、socket 配置与关城），线程那一段是唯一一段带着**自己的契约**的：账本在线程**里面**打开且永不离开（一座城只有一个写者，而这件事不靠约定靠类型），并且它带着一次**握手**：`ready_rx.recv()` 回来之前，没人能把 socket 架在一座没打开的城上。把握手包进方法里，返回的 `JoinHandle` 于是自带一句断言：拿到它，就意味着那个写者已经在跑。

**尺寸**：`serve` 233 → **162**；`spawn_worker` 91。

**它以什么收口**：纯结构，无可咬的红。线程体逐字搬迁；原先在 `serve` 里各自 `Arc::clone` 的 `worker_desk` 与 `to_clients` 改为在调用点克隆后传入，克隆的**个数与时机不变**。143 条 `sprawling` 测试全绿。
-/

/-!
## 8-33 同一把键的第二次，不是第二件活

```rust
pub(crate) struct CommandDesk { waiting: Mutex<Waiting>, arrived: Condvar, closing: AtomicBool }
struct Waiting { queue: VecDeque<Posted>, keys: BTreeSet<IdemKey> }   // 队列与在途键同一把锁

impl CommandDesk {
    pub(crate) fn post(&self, command: wire::Command, reply: wire::Reply);
    fn wait(&self, patience: Duration) -> DeskWait<'_>;               // Command(Posted, Underway<'_>)
}

/// 正在被办的那一条命令的键，办完即释放（Drop）。
struct Underway<'desk> { desk: &'desk CommandDesk, key: Option<IdemKey> }
```

**「让 `ToolBench::seen` 从账本重建」量完是错的**。键由 `(run_id, 本次驱动内的位次, action)` 铸成（§8-28），而 `run_id_for(job, addr, clock.now())` 把采钟拌进了身份，**没有任何生产路径会用同一个 `run_id` 再驱动一次**：审批放行后接着干的那段活，是 `answer_approval` 重新 `dispatch_in` 出来的一次**新 run**（§8-25），`resume` 只验链并把丢了结果的调用关成 unknown（ARCH §5 末）。往 `ToolBench::seen` 里播种历史，播进去的键在那一层永远比不中——那是把可测的防御换成不可测的死代码，正是 §8-28 拒绝过的那件事。

**没人守的那道门在上一层，而它今天就在漏钱**。`wire::frames` 的模块文档写着「每一条改状态的 Command 都带 `IdemKey`……『双击两次开出两个 Run』在这个类型里拼不出来」，而 `run_command` 的每一条臂都用 `..` 把 `idem` 丢掉：全库没有一处读 `Command::idem()`（只有 `wire/tests/wire_contract.rs` 与 `web::reach` 的两条测试读它）。四个发送端却都是照「服务端会去重」写的——`web::app::dispatch_command` 铸 `addr|task`、`web::city_view::create_command` 铸 `addr`、`console::dispatch` 铸 `console:addr:task`、`acp_dispatch` 铸 `acp:addr:task`，同一次提交两次就是同一把键；`web::reach` 甚至有一条测试叫 `saving_twice_configures_once`，它断言的却只是两条命令的键相等，**「只配置一次」这半句今天由谁兑现，答案是没有人**。于是双击一次、编辑器超时重发一次、控制台重敲一行，都是两次全款的模型账单。

**规则住在桌子上，而不是住在 `handle` 里**。`CommandDesk` 是每一条命令在 socket 与写者之间必经的那一处，它本来就按键之外的理由扫过自己的队列（`interrupt_for` 找 Cancel／Steer）。判定复用 `kernel::idem::claim`——`kernel::gate` 自述「seen 集合是调用方的状态，kernel 只判成员关系」，这里就是那个调用方的第二个实例（第一个是 `ToolBench`）。两个集合不是两处权威：一个管**工具调用**，一个管**人递进来的命令**，主体不同。

**在途，而不是永远**。一把键从 `post` 起在途，到那条命令**办完**为止：`wait` 交出 `Posted` 时一并交出 `Underway`，写者循环让它活到那一条命令服务完毕，Drop 释放键。于是——

- 双击、传输重发、编辑器超时重试：第二帧在第一件活还没办完时到达，被丢掉，**一次派活一次账单**；
- 人看完结果、想再跑一遍同一件活：键早已不在途，第二次照常受理，**不会静默吞掉**；
- 集合大小由队列深度加一封顶，不随城的寿命增长，也不需要时钟或任何窗口常数。

**`interrupt_for` 消耗掉的那条命令也要释放键**，否则同一个 run 的第二次 Cancel（`web::live` 铸的是 `cancel-from-the-control-surface`，每个 run 一把定键）会被永远丢掉。删队列与删键在同一把锁里完成。

**被丢掉的那一帧不回话**：发送者要的那件事正在办，`Reply` 只承载拒绝，而这里没有拒绝可言。**留下的残余**：控制台里同一行敲两遍，第二遍在第一遍办完前无声消失。真要给它一句话，得在 `AxCode` 上开一个「已在办」的码并让 `post` 交回受理与否——那是另一件事，这里不动，条件是这件事真的绊到人。

**它以什么收口**：一条会咬的红。`a_repeat_of_a_command_already_underway_is_not_a_second_piece_of_work` 首跑即红——今天两帧都进队列，第二次 `wait` 交出第二条命令而不是 `Idle`；实现后转绿，并在同一条测试里证明另一半：办完之后同一把键再来照常受理。既有测试 `a_cancel_reaches_the_run_it_cancels_without_waiting_for_it_to_end` 原先给三条不同命令共用一把 `b"i"` 键（图省事的夹具，真实客户端不会这么铸），改为一条一把——它测的路由与优先级不变。
-/

/-!
## 8-35 谁在追一个目标，谁替它派活（`RunWorker.pursuits`）

- **值住在工人身上，事实住在账本里。** `kernel::Pursuit` 由 `Delegator::root()` 铸出，而这座城里**唯一一处 `Delegator::root()` 就在 `RunWorker::over`**——于是「子代理不能让全城通宵干活」是一件关于代码的事实，而不是一条谁去遵守的规则。设置／暂停／恢复／清除各落一条 `pursuit_changed`，`Views` 折它来画，重启后工人从同一批记录把值重新铸出来。两处折叠都经 `Payload::read::<kernel::event::record::PursuitChanged>` 与它的 `held` 读这一行，读不回的一行让折叠报错而不是被跳过：跳过它，一座被清除目标的楼在重启后会继续追下去。
- **`Views` 不持 `Pursuit`，只持文本与状态**：一个能铸出 `Pursuit` 的视图，就是那道守卫上的第二扇门。判定仍由 `kernel::pursuit::observe` 给出，措辞由 `verdict_line` 一处写出——页面、控制台与日志说同一句话。
- **`pursue` 会终止，理由在集合上而不在计数器上**：认领把节点移出就绪集，而一个结束时还持有节点的 run 会把它留成 Blocked（`ClaimDesk::abandon`），所以就绪集严格变小；唯一让它变大的是拆分，而那是这座城找到了更多活，不是在打转。派活之后若该节点仍在就绪集里，追求暂停并留一条诊断——**看的是集合本身，不是一个凭空定的上限。**
- **值住在工人身上，事实住在账本里。** `kernel::Pursuit` 由 `Delegator::root()` 铸出，而这座城里**唯一一处 `Delegator::root()` 就在 `RunWorker::over`**——于是「子代理不能让全城通宵干活」是一件关于代码的事实，而不是一条谁去遵守的规则。设置／暂停／恢复／清除各落一条 `pursuit_changed`，`Views` 折它来画，重启后工人从同一批记录把值重新铸出来。
- **`Views` 不持 `Pursuit`，只持文本与状态**：一个能铸出 `Pursuit` 的视图，就是那道守卫上的第二扇门。判定仍由 `kernel::pursuit::observe` 给出，并以 `kernel::PursuitVerdict` 原样放进 `PursuitLine.verdict`；城不替它写句子，人读的措辞只在客户端的 `lang.json` 里按种类取。
- **`pursue` 会终止，理由在集合上而不在计数器上**：认领把节点移出就绪集，而一个结束时还持有节点的 run 会把它留成 Blocked（`ClaimDesk::abandon`），所以就绪集严格变小；唯一让它变大的是拆分，而那是这座城找到了更多活，不是在打转。派活之后若该节点仍在就绪集里，循环停下并留一条诊断——**看的是集合本身，不是一个凭空定的上限。**
-/

/-!
## 8-36 一个节点红了，站在它后面的人会知道（`tell_whoever_is_behind`）

- **由事实触发的交流。** 这座城里居民互相够到彼此的每一种方式，都从「有人决定要说话」开始；这一种从「一个节点红了」开始，并且恰好够到那些手上的活现在动不了的房间。
- **信号 id 由楼与节点推出**（`blocked-<building>-<node>`）：同一处卡住宣布两次是同一条信号，信箱按 id 去重。一个房间为一个问题被通知四次，是一个会停止读信箱的房间。
- **「谁持有哪个节点」只有一份**，折在 `CollaborationFold.plan_holders` 里：认领那条记录的 `addr` 就是房间，所以这里不推导任何别人已经写下的东西。`plan_view` 不再持第二份——它只画，不派信。
-/

/-!
## 8-40 先判定后动手：一次派活在城答应之前不写任何东西

`stage_dispatch` 的开篇注释一字不差地写着这条规矩——「Nothing is written before the city agrees to take
the work: a halted city that laid a job file down would leave a task in a room no run ever opened」——
而代码只守住了停摆那一道。**本节把那句注释变成代码的形状。**

### 量出来的现状

一次派活从人发出的动词到第一次写，走过三段，其中两段先写后判：

| 顺序 | 在哪 | 做什么 | 能不能拒绝 | 写不写 |
|---|---|---|---|---|
| 1 | `commanding::run_command` Dispatch 臂 | `session_for` | `E_INVALID_ARGS`（取不到名字） | 否，但**要花一次 Digest 模型调用** |
| 2 | 同上 | `room_for` → `city::open_room` | 存储错 | **写：房间目录**（`create_dir`） |
| 3 | 同上 | `choose_shape` → `city::write_session` | `E_CONFIG_INVALID`（形状已在会话里冻下） | **写：房间的 CONFIG.toml**（只在会话的第一个 Run） |
| 4 | `dispatching::stage_dispatch` | `halted_by` | `E_GATE_DENIED` | 否 |
| 5 | 同上 | `city::write_brief` | 存储错 | **写：`JOB.md`** |
| 6 | 同上 | `cas.put` | 存储错 | 写：CAS 对象（`.sprawling/` 内，内容寻址） |
| 7 | `workbench::stand_up` | `Building::of`／`city::load`／`load_config`／`Router::select`／`adapter_for`／`Identity::load` | `E_INVALID_ARGS`／`E_CONFIG_INVALID`／`E_MODEL_UNCHOSEN`／`E_GATE_DENIED` | 否 |
| 8 | 同上 | `run_id_for`／worktree 租约 | 存储错 | 写 |

实测（`sprawling call` 打到一座刚 init 的城）：派活到从没立过的楼 `gamma`，得到
`E_MODEL_UNCHOSEN「no model is chosen for this tag」`——**来自第 7 段的 `Router::select`**——
而磁盘上留下 `<city>/gamma/one/JOB.md`，账本只有 `seq 0 city_initialized`。
**第 2 段与第 5 段跑在第 7 段之前，这就是全部的病因。** 不是写域逃逸：`Work ".sprawling/evil"`
得到 `E_INVALID_ARGS` 且一字节未落，保留子树守得住。

### 判定的依据

**一次派活在城答应之前不写任何东西；城一答应，第一件被写下的就是房间。**

「城答应」由一处回答，穷尽如下，且每一条都只读不写：保留子树（`Building::of`）、停摆
（`halted_by`）、楼的规矩读得出（`city::load`）、房间的居民是 agent 时它的词认得、楼不是 confidential、派活没点名模型（`city::settled_harness` 与 `agent_protocols::Roster::seat`，§8-124；居民是 harness 就不再往下判模型）、tag 后面有模型且端点还在且不违反 confidential
（`Router::select`）、适配器造得出（`adapter_for`）。

**留在答应之后的两条拒绝，各有其理由，写在这里而不是被含糊过去**：

- `city::load_config` 读城／楼／居民三层。它**必须**在 `choose_shape` 之后，因为同一次派活写下的
  模型与 effort 要被这一次跑读到（「Chosen once, it holds for every later run in that room」）。把它提前
  会让这次派活看不见自己刚写下的那一层——那是行为改变，不是顺序整理。
- `city::Identity::load` 只在**文件权限**上拒绝；文件不存在读作 ephemeral。那是机器的故障而不是
  这座城的依据，与「派活到没立过的楼」不是一类。

两者拒绝时留下的是**一间空房间**，而那间房间是城已经答应之后开的：人要的房间开出来了，然后他自己写坏的
配置挡住了这次跑。这与「城拒绝了却留下一间没人开过的屋子」不是同一件事。

### 接口

```rust
/// 城在写下任何东西之前答应的那些事。
pub(super) struct Standing {
    building: city::Building,
    rules: city::BuildingRules,
    model: gateway::ModelEntry,
    adapter: Box<dyn Model + Send>,
}

impl RunWorker {
    /// 每一条这次派活可能欠下的拒绝，在它花掉任何东西之前。
    pub(super) fn agree_to_work(&mut self, addr: &Address) -> Result<Standing, AxError>;
    /// 站位，接过城已经答应的那部分。
    pub(super) fn stand_up(&mut self, agreed: Standing, at: &Assignment, given: &Given)
        -> Result<Site, AxError>;
}
```

`Standing` 的四样东西在 `stand_up` 里原样进 `Site`，**不留第二份**：`Site` 的字段一个不增一个不减，
拆开归位即可。`agree_to_work` 住在 `dispatching.rs` 而不是 `workbench.rs`，理由是尺寸也是位置：
`workbench.rs` 997 行、离 1000 行的文件门只剩三行，而这条规矩的那句注释本来就写在 `dispatching.rs`。

### `Assignment` 多两个字段，理由是规矩不能有两个家

`session_for`／`room_for`／`write_effort` 必须搬进 `stage_dispatch`，**否则这条规矩就有两个家**：
`stage_dispatch` 是唯一被七个派活入口共用的地方（人发的动词、批准后续跑的活、`wake`／`tick`／`knock`／
委派子活、继任），而房间是在人发的那条臂上开的。把答应放进 `stage_dispatch` 而把开房间留在臂上，
等于让敲门那条路照旧先写后判；把答应也放到臂上，就要在三个调用点各算一次。

于是 `Assignment` 从四个字段变六个：

```rust
pub(super) struct Assignment {
    /// 活被送到哪儿。还要开房间时它是一座楼，已经有房间时它就是那个房间。
    addr: Address,
    /// 要在那座楼下开的房间，当调用方点了名。序幕用掉它，而序幕正是 `addr`
    /// 从「人要的地方」变成「跑干活的地方」的那一行。
    session: Option<kernel::SessionName>,
    /// 那间房里的跑从此想多久，当调用方说了。写进房间自己的配置层，
    /// 所以它答不到房间存在之前去。
    effort: Option<kernel::Effort>,
    mode: kernel::Mode,
    budget: kernel::BudgetCap,
    parent: Option<RunId>,
}
```

`session` 与 `effort` 只活到序幕结束，而 `addr`／`mode`／`budget`／`parent` 穿过每一个相位——
一个值里两种寿命，这是自认的代价。**换来的是这条规矩只有一处能被违反**，而另一种形状是
它有两处、且其中一处没人看着。四个已有字段的语义逐字不变；三个不开房间的调用点写 `session: None,
effort: None`，那正是 `session_for`／`room_for` 对它们本来就有的答案。

### 序幕的顺序，以及为什么是这个顺序

```rust
let agreed = self.agree_to_work(&at.addr)?;        // 只读；第一条拒绝在这里
self.book_rules(&agreed.building)?;                // 答应之后的第一句：rules_changed 先落账再生效
let session = self.session_for(&at.addr, at.session.take(), &task, agreed.rules.policy())?;
// ↑ 可能花一次 Digest 调用，而那次调用带着这座楼的策略
at.addr = self.room_for(at.addr, session.as_ref())?;                  // ← 第一次写
// 会话冻下的形状写在这里，而已经开着的会话在这里被拒：两件事一个决定。
self.choose_shape(&at, &agreed.model)?;                              // §8-79
let brief = city::write_brief(...)?;
...
let mut site = self.stand_up(agreed, &at, &given)?;
```

**`session_for` 排在答应之后**：它可能向 Digest 模型要一个名字，而为一件城不会接的活付一次模型调用，
是这条规矩的钱那一面。

**命名这一次调用带着楼的策略走，与跑自己的调用同一格**（S-04）。`agree_to_work` 刚读出的 `rules`
就在手边，`session_for` 与 `name_the_work` 因此收一个 `&kernel::BuildingPolicy` 参数，由 `EndpointBook::select`
据它拒绝跑在城之外的端点——判定只有 `select` 一处，这里不复述规则。confidential 楼上的 Digest 端点跑在城之外时，
这条拒绝**原样上抛**（`E_GATE_DENIED`）而不是塌成「取不到名字」：模型在哪台机器上是人要处理的事，
恢复语因此同时给出两条出路（自己写 `building/name`，或选一个与城同在一台机器上的 digest 模型）。
取不到名字的其余失败仍是 `E_INVALID_ARGS`。

**验收**：dispatching 测试里的 `a_confidential_building_will_not_name_a_room_with_a_model_off_this_machine`（今天已无此名的测试，机密楼的拒绝由 `crates/accounting/src/worker/driving/tests/confidential.rs` 与 `crates/accounting/src/worker/dispatching/harness/tests.rs` 守住）——
confidential 楼、主模型与城同机、Digest 端点在机器之外，往楼名派活以 `E_GATE_DENIED` 告终，任务原文不上任何一条线。

**`book_rules` 是答应之后的第一句：改规则先落账再生效**。派工是规则的生效点：人手改 `RULES.toml`／`CONFIG.toml`
之后，城在下一次派工时才把新规则交给一个 run。`RunWorker::book_rules(&Building)` 把 city 层的 `CONFIG.toml`，
以及这栋楼**真实存在**时它自己的 `CONFIG.toml` 与 `RULES.toml`，逐份对比治理 fold 上次记下的摘要（`Governance.rules`，§8-25），
动了就先落一行 `rules_changed` 再让 run 起步。载荷携 scope、`which: GoverningDocument`（一栋楼两份，scope 单独说不清是哪份）、
before、after 与字节数，恒不携正文；`before` 缺席即开账行，此后本行的 `after` 等于下一行的 `before`，链断本身就说明有人绕过一切门改了文件。
摘要状态只由 fold 从 `rules_changed` 行得出，重启重建同一本账。缺席的文件按零字节记摘要，与 `city::load` 把缺席读成普通楼是同一个判断。
读不了的文件以 `E_STORAGE_FATAL` 拒绝这次派工：一个 run 不能站在城说不清的规则之下。

- **记在 `agree_to_work` 之后，不在它之前**：本节的规矩是城答应之前不写任何东西，一次被拒的派工因此不留行。
  被否：`stage_dispatch` 的第一句——停城、被拒的模型也会先落一行账，拒绝不再是「什么都没写」。
- **只为存在的楼记账**：楼的根目录不存在时，只记 city 层那一份。
  理由：往一个打错的楼名派活会给不存在的楼开账，账上就多出一座城没有的楼。存在与否用 `try_exists` 问，问不出来按读不了拒绝，而不是当作不存在。
- **记账点在派工（生效点），不在写入门**：这两份文件有一个在编辑器里动笔的写者，写入门看不见那次写入；
  生效点看见的是文档本身，两种写者一视同仁。每一个入口——人的派工、城自发的活、计划自己的一行——都经 `stage_dispatch` 起步。

验收：`accounting::worker::driving::tests::flight` 的三条 `rules_changed` 测试——首见开账、链上前后相接并先于它治理的 run 的每一行、
重启 fold 出同一本账；往不存在的楼派活只记 city 层一行。

**`halted_by` 并入 `agree_to_work`**：它本来就是唯一守住的那道门，
现在与其余五道站在一起，于是「城答应什么」读一处就够。

**采钟点不动**（ARCH §10）：`run_id_for` 读的 `clock.now()` 仍在选型之后，
采样次数与相对先后逐字不变；变的只是两者之间多了几次文件写，而那不是任何账本值的输入。

### 谁答哪一个错误码，逐字不变

`agree_to_work` 里依据的先后就是今天的先后，因此**没有一个调用方会看到与今天不同的码**：
停摆答 `E_GATE_DENIED` 而不是 `E_CONFIG_INVALID`（否则会把一个能自己解除的停摆说成要去接 provider），
保留地址答 `E_INVALID_ARGS`。派活到没立过的楼仍答 `E_CONFIG_INVALID`——
**这一条是刻意保下来的**：轨道二的模型把「楼在不在这里不问」记为一件量出来的产品事实
（`tools/adversary/src/Sprawling/Model.lean` 的 `refusal`），改码等于要那份模型跟着改，
而这里不碰 `tools/adversary/`。加一道「楼必须存在」的前置判断会正好破坏它——这是不选那个修法的第二个理由，
第一个理由是它只修一半（`acme` 真在而没挂 provider 时 `<city>/acme/one/JOB.md` 照旧留下）。

### CAS 那一次 `put` 留在原位，理由写在这里

`cas.put(brief.segment_text())` 仍在 `stand_up` 之前，所以严格地说「答应之后」并非一个字节都不写：
`.sprawling/cas/` 会多一个对象。**留它的理由**：CAS 是内容寻址且去重的，同样的字节写第二次就是同一份，
它不可能在城里留下一间没人开过的屋子；而把它挪到答应之后，`run_id_for` 就得从一个此刻还没入库的
摘要拼出 `cas:b3-…`，于是「locator 钉住的是库里的那些字节」这条权威会有第二处。
本节的依据因此写作「城里人看得见的东西」，而不是含糊的「任何东西」。

### 验收

1. **红转绿（Rust，本仓）**：`a_dispatch_the_city_will_not_take_leaves_no_room_behind`——
   一座刚 init、没挂任何 provider 的城，派活到 `gamma/one`，必须得到 `E_CONFIG_INVALID`，
   且 `gamma` 目录不存在。今天它红在第二条断言上。
2. **红转绿（轨道二的检验器，仓外）**：`a refusal costs nothing` 那一组两条转绿——`nothingBehind`
   （被拒的派活不改变城的目录树）与 `listsOnlyRaised`（`city_view` 只列被立起来过的楼）。
   这两条断言是验收标准，不许为了让它绿而改动它们。
3. **不回归**：`sprawling` 全部单测绿；`a_dispatch_with_no_goal_leaves_no_job_file_and_says_the_person_is_here`
   与 `work_in_a_review_building_reaches_it_only_after_someone_else_checks_it` 直接盯着序幕与租约这两支。
4. `just check` 绿。

### 文档同步

本节；`ARCHITECTURE.md` §5 第 3–4 步（派活先答应再写，房间是第一件被写下的东西）；
§8-31 的相位表（`stand_up` 多收一个归位值）。
`city` 与 `wire` 的公开面不变。
-/

/-!
## 8-163 落点一 · 逆携带：变化带着它的撤销值来

§8-24 把「行在变化之前」变成了类型的性质（`Then` 只能从 `Landing::record` 里拿到）。
这里问的是下一句：**变化落到一半失败，城是什么形状**。今天 `settle` 里五个臂都是
「推过去，错了就把错抛上去」——`Deliver` 循环里第三个 signal 投递失败，前两个已经在
Inbox里；`Shelf` 循环里第二个 filing 写盘失败，第一个已经在架上；`Roadmap` 写半截，
`Hold` 无失败面所以没事。账本是对的（行全落了），城是撕裂的：历史说五个都到了，
城里只到了两个。

**逆携带**：`Then` 的每个臂与其同构造子的撤销值一起走。`Deliver` 带着「这一轮推进了
几个、knocks 推了几个」回来，失败时调用方把没投递的留下（它们本来就在调用方的
`Vec` 里，没丢）、把已推进的 knocks 截回进入时的长度；`Shelf` 带着已写下的路径回来，
失败时把它们删掉（架上无历史，这是 §8-24 那句话的另一半）；`Roadmap` 经 `city::edit_against`
以落地时读到的文本为基线替换，替换是原子的，失败时文件仍是基线，撤销值是空；`Hold` 无失败面，
撤销值是空。撤销值不是第二个权威：它只在 `settle` 的一次调用里活着，调用结束就丢掉。

**为什么不是两阶段提交**：两阶段要一个所有参与者都认的准备态，inbox 的队列、
goal 登记、书架的文件三者没有公共的准备态。逆携带要的只是一个调用里「做了什么」
的记录，失败时按记录往回走。往回走本身也可能失败（删文件时盘掉了）——那时返回
原始错误并把回滚失败写进 recovery：两个事实都得说，丢哪一个都是撒谎。

**红**：`a_half_settled_landing_leaves_no_torn_city`。一跑投递三个 signal，
第二个投递失败（inbox 满）；断言：第一个 signal 不在Inbox里（调用方截回），
knocks 长度与进入时相等，账本三行都在（行不受牵连）。`Shelf` 同形：第二个 filing
写盘失败，第一个 filing 的路径不存在，账本两行都在。

**影面**：`sprawling` 公开面不变（`effect` 不是 `pub mod`）。`settle` 签名不变，
行为变（失败时回滚），红钉住它。
-/

/-!
## 8-164 落点二 · epoch 机器：「依赖快照」回答、「轮询」不回答

哲学一句话的后半句是「每个依赖驱动其激活」。这里先回答它的一半：
**快照是依赖的形状，轮询是依赖的反形状。**「散装 notify 轮询」不是指某一个
timer——`SCHEDULE_TICK`（20s，`serving.rs`）本身留着——而是说：今天「什么该醒」
这个问题的答案散在五处，每处各读一遍磁盘，各用各的「上次」：

| 谁问 | 在哪 | 读什么 | 记住什么 |
|---|---|---|---|
| `tick` | `commanding/routing.rs` | `Schedule::load` 全表 | `last_tick`（worker 字段） |
| `wake` | `waking.rs` | `Watch::load` 全表＋`buildings` 全量 | 无（每次全算） |
| `knock` | `waking.rs` | `Identity::load`（逐 room） | 无 |
| `answer_knocks` | `waking.rs` | 无（只 drain） | `knocks`（worker 字段） |
| `stage_dispatch` | `dispatching.rs` | `halted_by`（治理折叠） | 无 |

**三处可量**：`wake` 一次读两遍磁盘（watch 表＋全部楼目录）而只为投递一个 arrival；
`tick` 一次读全表而只为问「自上次以来谁到期」；`Watch::listening` 的「楼还在」
每次现算，而楼的生死是账本里变化最慢的事实之一。

### 动的与不动的

**动的只有一处**：`commanding/routing.rs` 的 `tick` 不再读全表，而是读
`city::Schedule::due_after(path, last_tick, now)`——到期判断（`last_firing`
区间比较）下沉到 `city`，`assembly` 只剩 dispatch 循环。`Schedule::due`
（返回 `Vec<&Entry>` 全量引用）保留：它是 `city` 自己的公共面，删它是
`city` 的 breaking，故不碰。

**不动的三处，理由各写一条**：

- `SCHEDULE_TICK` 不动：tick 间隔是 serve 面的节奏（§8-38），到期判断是 city
  面的语义，两者不在同一层，换一处不动另一处。
- `wake` 的双读不动：watch 表是人的文件（`listening` 语义含「楼已拆即失聪」），
  到达即读即算正是「文件是人的」这个归属的形状；快照它等于替人记住，归属错。
- `knocks` 不动：它是逆携带的截回点（§8-27），形状已钉，动它等于重开逆携带。

### 接口

```rust
impl Schedule {
    /// 自 `after` 以来到 `now` 之间到期的条目。`due` 的区间判断原样下沉，
    /// 返回拥有权的 `(Address, String, String)` 三元组：调用方只剩 dispatch。
    pub fn due_after(&self, after: TimeMs, now: TimeMs) -> Vec<(Address, String, String)>;
}
```

`tick` 此后三行：`load` → `due_after(last_tick, now)` → 逐条 `dispatch`。
`last_tick = now` 的位置不变（先推进再跑：到期判断的输入是读到的那一刻）。

### epoch / LOADING / UNLOADING 在哪

epoch／LOADING／UNLOADING 是**「Assembly 显式化」那一步的
主题**，不在这里。这里只把「到期判断」这一处依赖收成快照的形状
（`due_after` 即运行级依赖快照的最小形态：调用方拿着「到期了什么」，
而不是「全表＋上次」），并为那一步留下一句依据：**凡调用方仍在做区间比较、
仍在记 `last_*` 的，皆是 epoch 机器要收走的东西**（`knocks` 除外，它是
§8-27 的逆携带点）。

### 验收

1. `tick` 的三行与 `due_after` 的区间语义由既有红守着：
   `a_scheduled_job_starts_by_itself_and_only_once_per_firing`
   （三调用：到期 1、期内 0、一小时宕机仍 1）逐字绿，不改一字。
2. `commanding` 一分为二（`routing`：动词路由＋tick；`governing`：halt／
   autonomy／approval／fork）与 `settling` 一分为二（`desks` 四桌顺序；
   `landing` 单落点＋结论）只搬家：跨文件调用的可见性收成
   `pub(in crate::assembly)`，行为零变，`sprawling` 158 全绿。
3. `just check` 绿；`city` 公开面只增一函数。

### 文档同步

本节；`ARCHITECTURE.md` §6 新增六行（commanding::routing／governing、
settling::desks／landing、commanding::tests::answering／clockwork、
settling::tests 已有、waking::tests）；`crates/city/Spec.lean` 的 schedule 节（§8-6）记
`due_after` 与 `due` 并存的理由（删 `due` 是 breaking）。
-/

/-!
## 8-42 并发地板：一条记账线程，一个驾驶池

这一节回答一个问题：**这座城怎样同时跑两轮活，而账本上的字节仍然逐字节可重放。**
答案是把今天那一条 `sprawling-runs` 线程一分为二，两半各自持有互不相交的东西。

### 8-42-1 两半各持有什么

**记账线程（accounting thread）**，也就是今天那条唯一写入者，独占下列全部状态，一件不外借：

| 它独占的 | 今天住在哪 |
|---|---|
| `JsonlLedger` | `RunWorker.ledger` |
| 端点书 `EndpointBook` | `RunWorker.book` |
| 计划（`plan_holders`／`accounting::plan_view`） | `RunWorker.plan_holders` |
| 追求 `pursuits` | `RunWorker.pursuits` |
| 治理 `Governance`（待批、放行、停摆） | `RunWorker.governance` |
| 五张桌子（inbox／join／pr／goal／shelf 的**归位**那一半） | `RunWorker.inboxes`／`joins`／`requests`／`goals` |
| 准入计数（同时在跑几轮活） | 新增，见 §8-42-4 |

**驾驶池（driving pool）**的线程只持有一个东西：一次 `Driving`。它没有账本、没有书、没有桌子的所有权，
也没有工人。它把 `Driven` 送回来，然后什么都不记得。

**从池里发出的每一次写，都走 relay**（§8-42-2）。池线程手上唯一的 `kernel::Ledger` 实现是 `Relay`，
于是「一座城只有一个写者」这条性质由类型持有而不是由纪律持有：池线程根本拿不到 `JsonlLedger`。

**`settle` 永远在记账线程上跑，且按 `Driven` 到达顺序跑**。到达顺序而不是发起顺序：发起顺序会要求
记账线程为一轮还没回来的活留位置，那就是一个隐式的队头阻塞，而它挡住的正是已经跑完、正等着落账的那一轮。
到达顺序由 `mpsc` 的接收顺序给出，接收顺序由记账线程单线程读取，因此**同一批 `Driven` 的到达顺序
就是账本上 settle 各行的顺序**——这是 ARCH §10 规则 5「并行执行，串行记账」在这里的具体形状。

**`Interrupt` 按 run 注册**。今天 `RunWorker.interrupts` 是一个 `FnMut(RunId) -> Interrupt` 的钩子，
一次派活借走、结束还回（`driving.rs` 的 `self.interrupts.take()`）。一个池意味着同时有 N 轮活在问
「有人打断我吗」，于是这个钩子从「借走一个」变成「每轮活各注册一份」：`CommandDesk::interrupt_for`
本来就按 `RunId` 挑命令，这里只是让 N 份 `Relay` 各自带一份指向同一张桌子的注册。

**车道在驾驶的那轮活，它的 Cancel／Steer 留在桌上等车道来取**。记账线程每次被敲醒都读桌子，车道只在安全点读，
所以两个读者抢同一条命令时几乎总是记账线程先到——它若照 FIFO 取走，路由只能答「没有在飞的 run 认这个 id」，
而那轮活其实正在跑。于是 `CommandDesk::next(driving)` 带一个「这个 run 此刻在车道上吗」的判定（由
`RunWorker::drives` 回答，它读 `Flight.driving` 那一张表）：命中的 Cancel／Steer 原位留下，只归
`interrupt_for` 取；排在它后面的命令照常往下走。挑哪一条、Cancel 压过 Steer 仍只由 `interrupt_for` 一处决定，
桌子上的这个判定只决定「谁有资格取」。那轮活落地后 `drives` 转为否，还留在桌上的那条才交给路由，
拒绝的措辞此时是真的。**另一条路被否决**：记账线程先取走、再塞进车道的一个信箱，会让「Cancel 压过 Steer」
在桌子与信箱两处各判一次。判定与派活都在记账线程上，派活把 run 登记进 `Flight.driving` 之前不会读下一条命令，
所以「刚派出、还没登记」的窗口不存在。

### 8-42-2 `accounting::worker::relay`——`kernel::Ledger` 的第三个适配器

形状：**adapter**（ARCH §9 第 4 种）。它不做任何判断，判断全在记账线程那一侧。

```rust
/// 一批追加（一个回合在下一个对外效果之前攒下的记录，runtime D24），以及它的回信地址。
pub(crate) struct RelayRequest {
    drafts: Vec<EventDraft>,
    back: std::sync::mpsc::SyncSender<Result<Vec<EventRef>, AxError>>,
    /// lane 放它进队那一刻的单调钟读数；记账线程取到它时减出排队等待（§8-98）。
    queued: std::time::Instant,
}

/// 池那一侧的脸：唯一的 `kernel::Ledger` 实现，池线程只有它。
pub(crate) struct Relay { /* 一个 Sender */ }
impl kernel::Ledger for Relay {
    /// 一条 draft 是一批只有一条的 `append_all`。
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError>;
    /// 整批作为**一个**请求发下去，然后**阻塞**等一封回信：一次往返，与同时排队的别的请求共用一道屏障。
    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError>;
}

/// 唤醒记账线程的一切，同在一条队列上：每张嘴一个变体，再加关门。
pub(crate) enum Wake { Relay(RelayRequest), Claim(ClaimAsk), Goal(GoalAsk), Home(Box<Arrival>), Command, Close }

/// 一次看队列最多等多久。
pub(crate) enum Patience { Now, For(Duration), Unbounded }

/// 记账那一侧的脸：那一条队列的 Receiver ＋ 一个用来发牌的 Sender。
pub(crate) struct RelayGate { /* … */ }
impl RelayGate {
    pub(crate) fn open() -> RelayGate;
    pub(crate) fn issue(&self) -> Relay;
    /// 车道回家与 desk 用的 Sender。
    pub(crate) fn bell(&self) -> mpsc::Sender<Wake>;
    /// 按 `patience` 等第一条消息，再用 `try_recv` 把排在后面的一次取尽：relay 请求**合成一道屏障**
    /// 写下去，回家的活按到达次序放进 `homes`，Command 与 Close 只负责唤醒。
    /// 认领按队列次序当场答复；目标登记不在这里答，按到达次序交回。
    pub(crate) fn serve(&mut self, patience: Patience, ledger: &mut impl kernel::Ledger,
                        homes: &mut VecDeque<Arrival>) -> Drained;
}
/// 一次看队列留给记账线程的东西：账本收下的每一行（按账本次序，交给各份折叠），
/// 与等着目标表答复的登记（按到达次序，交给 `RunWorker::answer_goal`）。
pub(crate) struct Drained { pub(crate) written: Vec<EventDraft>, pub(crate) goals: Vec<GoalAsk> }
```

**为什么 `append` 阻塞**：`kernel::Ledger` 的契约写着「`Ok(ref)` 意味着这条记录在那个适配器的介质里已经耐久」。
一个不阻塞的 relay 会在记录还没落盘时就回 `Ok`，那是把契约改写成「已经排队」——
于是 `run_started` 可能排在它自己那轮活的 `model_called` 后面。阻塞是这条契约的价钱，也是它的全部内容。

**回信通道是 `sync_channel(0)`**（会合信道）：一次 `append_all` 一个回信地址，不留缓冲，
所以「记账线程写完了」与「池线程知道写完了」之间没有第三种状态。

**签名的一处出入，如实记在这里**：另一种说法是「阻塞等 `Result<EventRecord>`」，
而 `kernel::Ledger::append` 的返回类型是 `Result<EventRef, AxError>`。**以端口为准**——
一致性套件是契约，而 `EventRef` 是那个套件检验的东西。
`EventRecord` 会把整条记录复制过河，`EventRef` 不会，且 `EventRef` 无法伪造（ARCH §9）。

**一致性套件原样通过**：`kernel::ledger::conformance::assert_ledger_conformance` 一个字不改地跑在 relay 上。
套件要求的 `LedgerInspect` 是「只为验证而设」的读回面（`ledger.rs` 的模块文档），
本节因此把它实现在测试里的一层包装上，而不是在 relay 的生产面上开一个读洞：
套件检验的是 relay 的 `append` 路径（seq、prev、字节、两次新实例产生同样的字节），那正是契约。

**服务顺序：relay 请求排在 desk 命令之前**。理由是一个已经在跑、已经花了钱的活，不该排在一条还没开始的命令后面；
反过来排会让一次 `Dispatch` 命令挡住三轮正在写 `tool_result` 的活。

**一道屏障带一回合攒下的记录（runtime D24）**。回合把只读调用的 `tool_called`／`tool_result` 与 `model_returned` 留在回合自己的账本门里，到下一个对外可见的效果之前用一次 `append_all` 交下来（`crates/runtime/Spec.lean` §8-3，性质在 `crates/runtime/spec/Turn/Durability.lean`）。relay 的那一半是：`Relay` 覆写 `append_all`，把整批 draft 装进**一个** `RelayRequest`，记账线程把它与同时排队的别的请求摊平后一起写进同一道屏障，再按各请求的条数把 ref 按位置切回去；ref 少于 draft 时那个请求得到拒绝，不会得到一份短的答复。`Health` 的两个计数按请求计，一批是一次追加。契约一字不改：回信仍在整批落盘之后才发，`Ok(refs)` 里每一条都已耐久；一批中途被拒时整批都得到拒绝，拒绝之前的那些可能已经落盘，与 `append_all` 在端口上的承诺相同。`relay::tests::tf1_relay_answers_a_batch_in_one_round_trip` 守着「三条 draft 一次往返、一道屏障、按序三个 ref」。落选的是「relay 收下就回信、回合另发一个只等屏障的请求」：那等于在 relay 上开一个「已排队」的回信，`EventRef` 就不再指向一段存在的历史。

### 8-42-3 `accounting::worker::pool`

形状：**adapter**。N 条 `std::thread`，从 `bin` 里那唯一的 spawn 点起（ARCH §10 规则 3）。
入口 `(RunId, Driving)`，出口 `(RunId, Driven)`。

**线程 panic 不是一种情况**：发布档是 `panic = "abort"`（ARCH §2），没有可以接住的东西；
一次失败作为 `Driven::Failed` 走回来，而不是作为一个 join 出来的 `Err(Box<dyn Any>)`。

**池不设车道数**（`crates/sprawling/Spec.lean` D34）：准备好的 run 立即得到一条车道，放行只问内存（§8-46-3）；
并发的闸是每个端点的名额（`crates/gateway/Spec.lean` D17），排队只发生在 provider 那一处，在那里计数。
**citysim 不起车道**——场景是一份在一条线程上重放的固定脚本，`Driven` 的到达顺序就是发起顺序，
于是确定性不依赖调度器。

### 8-42-4 记账线程的三张嘴

人从界面派的活在把 `Driving` 交给一条车道之后就回到主循环，于是记账线程有三张嘴，按这个次序：

1. **relay 请求**（先服务，理由见 §8-42-2）
2. **`Driven` 到达**（车道的出口）
3. **desk 命令**（`CommandDesk::wait`）

`settling` 一个字不改：它本来就只在记账线程上跑，触发点从「drive 返回」换成「`Driven` 到达」。
**敲门仍在 settle 之后发出**。落地的类型、幂等键的落定语义与主循环的等待节奏写在 §8-46-2。

准入计数住在记账线程上：它是「同时有几轮活在跑」的唯一权威，而唯一权威必须在唯一写者那一侧，
否则两条线程各数各的，就有了两个答案。

**这条循环必须成立的三条性质由 Lean 模型定**：`crates/accounting/spec/Worker/Attend.lean`（`just models`
证明它）。三张嘴与关门都送进同一条队列，线程阻塞在第一条消息上，醒来后把已经排在后面的
一次取尽，按到达次序服务。模型证明：醒来时在等的每条消息都在这一次醒来里被服务（于是到达的消息在下一次
醒来里被服务）；追加的次序就是 seq 的次序；没有消息、也没有到期的排程截止时刻时线程不醒，而每次醒来都
消耗掉至少一件工作，所以醒来的次数以消息数加截止时刻数为上界。模型只管追加了几条记录，一条消息落地时
做的其余事（结账、回话）归 Rust。

**`attend` 就是模型里那一条队列**：`Wake` 每张嘴一个变体再加关门，`RelayGate::serve` 先阻塞在
第一条上，醒来后 `try_recv` 取尽，relay 请求合成一次 `append_all`。desk 仍然自己存命令（运行中的活在
安全点上要从 desk 里找 Cancel），`post` 与 `close` 只往队列里送一条 `Command` 或 `Close` 去唤醒；
线程每次醒来都先服务口子，再看 desk，desk 空时才再睡。一条被 `pursue` 的内层循环吃掉的 `Command`
不会丢命令：命令在 desk 上，外层循环回来时先看 desk，再睡。这个枚举不叫 `Inbox`：词汇表里 Inbox 是
Approval Inbox，人的待答队列。

**落 run 的重活不在记账线程上。** 第二张嘴取到一个 `Arrival` 之后在记账线程上调 `RunWorker::land`；transcript 的物化
与检查点清扫在这之前已经在这个 run 的 lane 上做完（`Staged::fly` 在调 `home` 之前做），清扫结果随 `Flown::Model` 的
`swept` 回家，于是 `land` 在记账线程上只把结果折成记录追加，并做改写折叠的那些步骤。切线与每一步留在哪一边的理由是
`crates/sprawling/spec/Accounting/Landing.lean` 的 D37。三张嘴的次序与 ARCHITECTURE §13.4 一致（relay 先、再至多一个回家、
再 desk）。模型证明 relay 请求在账本里的位置与重活多重无关、记录仍只有一个追加者与一个全局次序、run 被看见冻结时它
落 run 的记录已在账上。由模型导出的 Rust 检查是
`dispatching::preparing::tests::tp4_a_lane_keeps_the_transcript_before_its_run_comes_home`：run 回家的那一刻它的
transcript 已经在盘上，所以记账线程的 `land` 里没有这一步。视图广播的按帧合并在每个会话的事件臂上做：醒来时已经到了的
记录逐条成为各自的 `Event` 帧，刷写一次，模型与理由在 `crates/wire/spec/Server/Socket.lean` §8-47h（wire D45）；那里也说明
为什么不让正在看的 session 先走：服务端不知道 User 在看哪个 session，按 session 重排又会让 `decide_lag` 发出假的 `Lagged`。

### 8-42-5 被否决的备选

**备选一：多进程。** 一轮活一个进程，各自持有自己的一份状态，用管道汇总。
**否决理由有三条，任何一条都够。**
第一，账本的 `seq` 与 `prev` 是一条链，`kernel::ledger` 的契约把 seq/prev 的分配交给实现；
多进程要么共享一个写者进程（那就是这里的 relay，只是把 `mpsc` 换成了一个需要序列化、需要重连、
需要处理半个写入的 socket），要么让每个进程自己发 seq（那就有 N 个权威，链断）。
第二，ARCH §1 写的是「一个进程，一个页面」，§6 写的是「一座城是一个目录」——
多进程要给每个子进程一份 git worktree 之外的东西（金库句柄、redb 句柄），而 redb 与 keyring 都不是
可以被两个进程同时打开的东西。
第三，确定性：citysim 用一个种子重放一次跑（ARCH §11 V6），而进程边界会把「谁先写」交给操作系统的调度器，
且这件事在崩溃恢复时不可重现。

**备选二：把一轮活改成 async。** 让 `turn` 与 `drive` 变成 `async fn`，用 tokio 的多线程 runtime 跑 N 轮。
**否决理由**写在 ARCH §2 那张表里，而且是这份设计已经付过钱的一条：
「回合循环是刻意同步的——一个会 await 的决策就是一个会交错的决策。async 停在进程边界。」
把 `drive` 改成 async 会让四个取消安全点（`runtime::turn` 的 typestate）从「四个可以枚举的点」
变成「每一个 `.await` 都是一个点」，而 typestate 之所以能说「相位内部的中断拼不出来」，
靠的正是那四个点是可以数清的。第二条理由是钱：async 会把 `reqwest` 的阻塞客户端换成异步客户端，
而 ARCH §2 说这个 workspace 只有一个 HTTP 客户端，两个客户端就是一个二进制里两套 TLS。
第三条理由是这次并发要的东西 async 给不了：我们要的是**并行驾驶、串行记账**，
而 async 在一个 runtime 上给的是并发交错——交错的是同一条线程上的决策，那恰恰是被禁止的那件事。

### 8-42-6 验收

1. **红转绿**：`the_relay_passes_the_ledger_conformance_suite`——
   `kernel::ledger::conformance::assert_ledger_conformance` 原样跑在 relay 上，
   记账那一侧是另一条线程持有的账本。
2. **红转绿**：`driving/tests` 里两次派活交错——账本 `seq` 保持单调，
   每一轮活的 `run_started` 排在它自己的 `model_called` 之前。
3. citysim 六个场景在池大小 1 下逐字节重放同样的账本。
4. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings` 与
   `cargo nextest run -p sprawling --locked --all-features` 绿。

### 8-42-7 `Driving` 为什么必须拥有它驾驶所需的一切

一条车道是一条线程，所以 `Driving` 的每一个字段都要 `Send`，而其中两个曾经不是：
`bench` 借的是 `ToolBench`，`kernel::Tool` 当时没有 `Send` 上界；`signals` 借的是 `Rc<RefCell<SignalDesk>>`。
两者都不是 `bin` 能单独修好的——工具立面横跨 kernel／runtime／collab／city／agent_protocols／browser 六个 crate。
落地形状写在 §8-44：这六个 crate 的 trait 各加 `Send` 上界，七张桌子换成 `Arc<Mutex<_>>`，
`Driving` 于是拥有自己的 signals 句柄、一份 `Cas` 第二句柄（§8-43）与 backlog 成员号（`crates/runtime/Spec.lean` §8-28-2）。
`driving/tests/turns::a_drive_can_be_handed_to_another_thread` 钉住这条性质：`Driving: Send` 是编译期事实。

### 8-42-8 `accounting::worker::booking`——计划认领在调用时由记账线程判定（形状 4 适配器）

每条车道的 `ClaimDesk` 持有派活那一刻的 `Roadmap.md`，并排派出的两轮活读的是同一份文件，所以只凭桌子自己的副本，
两轮活都会认领同一个节点、都把活做完，第二个到落地时才被 `still_true` 丢掉。认领因此在模型调用 `plan claim` 的那一刻
交给记账线程判定：它按队列次序看见每一个认领，先问的拿到节点，后问的当场被拒，一次模型调用都不白花。

```rust
pub(crate) struct PlanRead(u64);  // 桌子读计划时 ClaimBook 已看见的停止行数
pub(crate) struct Claimant { pub(crate) building: Address, pub(crate) room: Address, pub(crate) run: RunId, pub(crate) who: String, pub(crate) clock: Arc<dyn Clock>, pub(crate) read: PlanRead }
pub(crate) struct ClaimAsk { /* building、node、roadmap_claimed 那一行的 EventDraft、放回行 effect::Line、回信的 SyncSender —— 私有 */ }
#[derive(Default)]
pub(crate) struct ClaimBook { /* (building, node) → 在飞的 RunId 与放回行 —— 私有 */ }
impl ClaimBook {
    pub(crate) fn answer(&mut self, ask: ClaimAsk, ledger: &mut impl Ledger); // 入账并登记，或拒绝；然后回信
    pub(crate) fn release(&mut self, run: RunId) -> OpenClaims; // 这轮活回家时放开它持有的节点，交出它们的放回行
    pub(crate) fn read_mark(&self) -> PlanRead;                 // 现在开的桌子带的计数
    pub(crate) fn absorb(&mut self, kind: EventKind, addr: Option<&Address>, data: &Payload) -> Result<(), AxError>; // 停止行加计数，放回行让节点重新就绪
}
#[must_use] pub(crate) struct OpenClaims { /* run 与 node → 放回行 —— 私有 */ }
impl OpenClaims {
    pub(crate) fn close(&mut self, node: &NodeId);           // 这个节点的一条收尾行已在账上：不再放回它
    pub(crate) fn owed(self) -> (RunId, Vec<effect::Line>);  // 尚未合上的放回行，归在哪一轮活名下
}
// effect.rs：计划那一步的每一行带着它合上的节点；split 行合上它的父节点
pub(crate) struct Closing { pub(crate) line: Line, pub(crate) closes: Option<NodeId> }
// effect.rs：放回行的唯一拼法，陈旧落地的 released() 与车道的认领共用
pub(crate) fn handed_back(claim: &ClaimEffect, note: &str, room: &Address, who: &str) -> Result<Option<Line>, AxError>;
pub(crate) fn booking(bell: mpsc::Sender<Wake>, claimant: Claimant) -> collab::Booking;
```

- **走同一条队列**：认领是 `Wake::Claim`，与 relay 的 append 同在记账线程那一条队列上（§8-42-4），而不是第二条通道；
  `RelayGate` 持有 `ClaimBook`，`serve` 在排空队列时逐个答复。车道阻塞在一个 rendezvous 通道上等回信，与 append 一样。
  入账的认领行随 `serve` 的返回值交回，经 `RunWorker::absorb` 进活的 `PlanHolders`，所以那轮活还没回家时，记账线程读到的持有者已经与重启折出的一致。
- **拒词**：`InvalidArgs`，动作 `claim a plan node`，说出节点与持有它的 run，恢复是「list the plan and claim a node that is ready」。
  记账线程已经不在时是 `StorageFatal`，与 relay 的 `gone` 同一形状。
- **先入账再登记**：`answer` 接受认领时，在记账线程上把 `roadmap_claimed` 追加进账本，追加成功才登记节点并回 `Ok`；
  账本拒绝那一行时不登记，拒绝原样回给模型——于是没有哪轮活持有一个历史上看不出它持有的节点。那一行由车道在调用那一刻
  用 `ClaimEffect::kind`／`payload` 拼好（时刻取自 `Claimant.clock`，即 worker 自己的 `accounting::Clock`），归在那轮活的房间下；`Claimant` 是派活时就定下、
  随每次认领一起走的六个值。被拒的认领不留任何一行。
- **登记持续到那轮活回家**：`Flight::arrived` 放开它，而它的落地在同一线程上、在下一次 `serve` 之前跑完，
  所以没有任何认领会在「放开」与「盘上的计划写明节点结局」之间被答复。
- **桌子的副本比节点的结局旧时，账本的停止行作答**：一轮活的桌子可能在另一轮活落地、把节点写成完成／拆分／阻塞之前读了计划，
  副本里那个节点仍是就绪。`ClaimBook` 经 `RunWorker::absorb` 看见 worker 写下的每一行：`roadmap_finished`、`roadmap_split`、
  `roadmap_blocked` 把计数 `PlanRead` 加一并记下这个节点停在哪个计数上，`roadmap_released` 让节点重新就绪、把它移出。
  `open_desks` 读计划的同时把当时的计数放进 `Claimant.read`；落地先写文件再写行，两者与开桌子同在记账线程上，所以计数不超过 `read`
  的停止都已在那份副本里。认领的节点停在比 `read` 更大的计数上时当场拒绝，拒词同上一条的形状，说节点在这轮活读计划之后已经结束。
  于是认领读的是账本上的结局，不是渲染出来的文件；人在编辑器里把节点改回就绪之后开的桌子读到的是新文件，计数也已越过那次停止，不受影响。
  重启时计数从零开始：那时没有在飞的桌子持有旧副本。被否：认领时在记账线程上重读 `Roadmap.md`——文件是账本的渲染，
  它与账本之间隔着落地的写盘；以及让 `ClaimBook` 永久记住被阻塞的节点——人改回就绪的节点会被永远拒绝。只有类型与内存里的表，
  Windows、macOS、Linux 上一样。
- **落地时的 `still_true` 比对保留为兜底**（`effect::Claims::of`）：上一条只拦停止过的节点；文件在落地前被城外的写者改动时，
  落地仍丢弃并告诉人。那条认领已在账本上，所以 `Claims::Stale` 带着给每条认领补的 `roadmap_released`
  （`StopCause::HandedBack`，说明是计划在落地前变了）一起落账；只丢弃不补，`folds::collaboration` 会把那一行读成永远有人占着。
  落地成功时 `Claims::of` 照样重放 `Claimed` 改文本，但不再写它的行，否则同一次认领在历史里数成两次。
- **落地重放效应，不写桌子的副本**：`Claims::of` 把本轮的效应按次序经 `ClaimEffect::apply` 重放到落地时读到的盘上文本，
  `Then::Roadmap` 带着那份文本作基线，经 `Planning::write_plan` 替换；生产里它就是 `city::edit_against`，测试换上一个拒绝的写者，
因为只读文件拦不住 Unix 上目录可写时的 rename，「改写被拒」要在每个 OS 上都成立才能钉住下面那条规则。桌子的副本是派活那一刻的文件，写它会把别的轮在这期间落下的行
  改回派活时的状态；重放只动本轮碰过的行。基线与读盘之间只隔同一线程上的落账，能在这里改动文件的只有城外的写者（人的编辑器），
  那时替换以 `E_VERSION_CONFLICT` 拒绝，行已在账本上而文件未动，错误原样交给 `settle` 的调用方。
- **回家的每一条路都要合上本轮开着的认领**：`roadmap_claimed` 在调用时入账，所以每一条认领都得有一条 `roadmap_released`
  或完成行把它合上，否则重启后 `folds::collaboration` 把那个节点读成永远被那间房占着，`plans` 还会给它发阻塞通知。
  车道在调用那一刻把认领行与它的放回行（`PutDown`，`StopCause::HandedBack`，说明这轮活回家却没有落地）一起拼好交给记账线程，
  `ClaimBook` 登记节点时连放回行一起记下；`Flight::arrived` 放开这轮活时 `release` 交出仍登记着的放回行（`OpenClaims`），
  随 `Home` 交给落地。`settle_desks` 的计划那一步每把一条收尾行写上账本（`Claims::Landed` 的放下／完成行，或 `Claims::Stale` 的
  `released()`），就对那一行的 `Closing::closes` 调 `OpenClaims::close`：`RunWorker::record_closing` 在追加成功之后、各份折叠看见那一行之前合上节点，因为折叠随后拒绝那一行也撤不回账上的收尾行，再补放回行会让历史说一个已完成的节点又被放回。`folds::collaboration` 按每个节点的最后一行判定持有，
  任何一条收尾行都让节点空出来，所以账本在其中一条上拒绝时，已写上收尾行的节点不再放回，只有最后一行仍是 `roadmap_claimed`
  的节点还欠放回行；一次清空整份 `OpenClaims` 会给已完成或已阻塞的节点再补一条放回行。落地结束后，不论成败，`serve_flight` 经 `record_for` 追加余下的放回行（时刻在追加时取，
  各份折叠照常看见它们）：落地在计划那一步之前的任何一个 `?` 上失败——驱动本身返回 `Err`（`driven?`）、目标行、清扫、
  `city::roadmap`、`Claims::of` 拒绝重放——余下的就是本轮全部的认领。计划那一步之后的失败（书架、请求、租约、结论）不再补行，
  因为那时认领已经合上，补一条放回行会让历史说一个已完成的节点又被放回。落地成功而追加失败时返回追加的错误；落地已经失败时
  返回落地的原错误，追加的失败记进诊断——账本已经拒绝过一行，第二次拒绝不改变人要做的事。认领在它的收尾行（finished、blocked、released 或拆分父节点的 split）写上账本时就算关闭——拆分之后这一轮什么也不持有，split 行就是父节点的去向，不等 `Roadmap.md` 改写返回：改写被拒（`E_VERSION_CONFLICT` 或文件不可写）时收尾行已在账上，不再补放回行。
- **目标登记同一条路**（`accounting::worker::registering`，形状 4 适配器）：`goal` 工具的桌子不留目标表的副本，只铸 id、拼 `GoalEntry`，
  经 `registering::booking` 把它作为 `Wake::Goal` 送上同一条队列并阻塞等回信。`serve` 不自己答它，而是按到达次序交回；
  记账线程在给各份折叠看过这一次写下的行之后逐个答复（`RunWorker::answer_goal`）：对着全城的目标表
  `collab::arbitrate`，不撞就先把 `goal_registered` 写上账本，撞了就先写 `goal_conflict`（载荷 `collab::conflict_payload`），
  两种都经 `record_for` 写、经 `absorb` 进活的目标表，然后才回 `Ok` 或 `collab::conflict_refusal`。
  于是并排的第二轮活在调用那一刻被拒，同一轮活的第二次登记也撞上第一次；落地不再写任何目标行（`Landing` 没有目标那一扇门）。
  目标表只由 `goal_registered` 折出，活的与重启的走同一个定义 `collaborating::register_goal`。
  **不在 `serve` 里答**：目标表住在 `RunWorker` 上，`serve` 只借得到账本；在那里答就得给门一份目标表的副本，
  那是第二个「这片地归谁」的答案，同一次排空里的第二个登记还会读不到第一个。
- 验收：`cargo nextest run -p sprawling -E 'test(/second_run_to_ask_for_a_node|two_runs_claiming_one_node_through_the_served_gate|two_runs_landing_different_nodes|a_claim_whose_landing_failed|a_claim_closed_on_the_ledger|a_landing_refused_part_way|a_split_closes_the_claim_on_its_parent|a_claim_booked_through_the_gate|two_runs_registering_one_ground/)'`；
  `cargo nextest run -p sprawling-collab -E 'test(/two_runs_read_as_ready/)'` 在桌子一侧钉住「第二个认领当场被拒、什么都不留」。
-/

/-!
## 8-43 筛子接进产品：`driving` 把 `exec` 结果经 `pipeline::package` 交给模型（`accounting::worker::driving`、`runtime::pipeline::exec`）

**量出来的现状**：`crates/runtime/Spec.lean` §8-27 的筛子完整落地，`pipeline::package` 也已带 `SieveRequest` 臂，但它在产品里没有调用方——`accounting::worker::driving` 的 `invoke` 钩子把 `BenchOutcome::Ran` 的结果原样交回 `runtime::turn`，于是压缩器只在 citysim 跑，城里的模型读的是 `cargo check` 的一千两百行原文。§8-27-9 末尾的「已知未接」说的就是这一处。

### 一个门，两个调用方

citysim 的 `citysim::sieving::package_exec` 是「一份 `exec` 结果怎样变成模型读到的东西」的第二份定义，产品接线若再写一份就是第三份。这里把它搬进 runtime，一个权威、两个调用方：

```rust
// runtime::pipeline::exec（形状 1 判定；文件 crates/runtime/src/pipeline/exec.rs）
pub struct SieveSite<'a> { pub offload: OffloadSite<'a>, pub table: &'a FilterTable, pub history: &'a mut SieveHistory }
/// 一份 exec 结果：stdout＋stderr 合成一段文本，按命令键过 package；
/// 结果里 stdout／stderr 换成 content，其余字段（arm、exit_code、env、background、handle……）原样留着，
/// 再加 sieve:[result_offloaded 载荷]。没有 stdout 也没有 stderr 的结果（backgrounded 形）原样返回。
pub fn package_exec(call: &ToolCall, outcome: ToolOutcome, site: SieveSite<'_>, stamp: Option<ClockStamp>) -> Result<ToolOutcome, AxError>;
pub const EXEC_CAP_BYTES: u64 = 16_384;   // citysim 一直用的那个值，现在只写一次
```

citysim 的 `sieving.rs` 改为调它；旧函数删除（迁移做完，不留适配层）。

### 装配层持有的三样东西

| 东西 | 谁持有 | 何时定 |
|---|---|---|
| `FilterTable` | `Site.filters` | `stand_up` 读 `<city>/.sprawling/FILTERS.toml` 与 `<building>/.sprawling/FILTERS.toml`，走 `FilterTable::resolve`（楼＞城＞内建，整值覆盖）。文件不存在＝`None`；读不到＝错误（§8-26：读不到不等于写错） |
| `SieveHistory` | 一次 `drive_dispatch` 内的局部 | 每跑一份；跨调用差分只在本 run 内成立 |
| `OffloadSite` | `Driving.sieving`：一份 `Cas` 第二句柄＋rest 目录 | rest 目录是 `<write_root>/<addr>/.rest`——`read` 只放行模型选的非保留路径，rest 文件放在保留区里就是给模型一个它够不着的地址 |

`Cas` 开第二个句柄而不借工人的：CAS 按内容寻址、经临时文件写入，同一目录开两次是同一个库；而 §8-42 的池线程不能借工人的任何东西，这一份句柄正是它以后要带走的。

**`stamp` 由这一跑的 `StampGate` 给**：`Sieving` 持有它与这一跑的 `ClockReading`，打包前问一次（§8-125）；citysim 的闭包工具面按同一扇门自己问它的那一个。

### 验收

1. **红转绿（`driving/tests/sieving`）**：一次真实派活，`exec` 打印一份超过 2 KiB 的输出；模型收到的工具结果含 `[sieve:` 页脚且短于原文；账本 `tool_result` 载荷的 `sieve[0].original` 是 `cas:b3-` 且能从 CAS 读回原文，`rest_path` 在磁盘上。
2. `tools/citysim/tests/sieve.rs` 的 `the_window_holds_the_diagnostics_and_the_way_back_and_only_the_news_the_second_time` 绿。
3. 同种子 citysim 逐字节重放不变（`the_same_seed_and_table_replay_a_byte_identical_window`）。

**留给 gitignore 的一句**：`.rest/` 住房间里，楼的 `.gitignore`（`crates/city/Spec.lean` §8-21）应忽略它，否则检查点提交会把一份 rest 文件收进历史。那份文件不在本节范围内。
-/

/-!
## 8-44 `Driving` 跨过线程：`kernel::Tool` 加 `Send`，五张桌子从 `Rc<RefCell<_>>` 到 `Arc<Mutex<_>>`（§8-42-7 量出来的前置条件）

**问题**：§8-42-7 逐字段量过——`Driving` 今天跨不过线程边界，卡住它的是两件不在 `bin` 里的事实：
`kernel::Tool` 没有 `Send` 上界，于是 `ToolBench.tools: BTreeMap<String, Box<dyn Tool>>` 不是 `Send`；
五张桌子（signals／goals／plan／shelf／pr，加 delegates 与 workshop 两张）全是 `Rc<RefCell<_>>`，工具持有它们的克隆。
没有这次拆分，池只能写成没有第二实现的空壳。

### 依据：什么要变、什么不变

| 东西 | 今天 | 此后 | 理由 |
|---|---|---|---|
| `kernel::Tool` | `pub trait Tool` | `pub trait Tool: Send` | `Box<dyn Tool>` 由此自动 `Send`；一个不能跨线程的工具在这座城里没有位置——它会被池线程调用 |
| `agent_protocols::mcp::Outbound` | 无上界 | `: Send` | `McpTool` 持 `Box<dyn Outbound>`；两个适配器（stdio 子进程、HTTP 客户端）本来都是 `Send` |
| collab 七张桌子的句柄 | `Rc<RefCell<Desk>>` | `Arc<Mutex<Desk>>` | 桌子本身没有 `Rc`，换句柄不换桌子 |
| `runtime::ReadTool.catalog` | `Rc<RefCell<Catalog>>` | `Arc<Mutex<Catalog>>` | 同上 |
| `runtime::StatusTool.children` | `Box<dyn Fn() -> Vec<ChildStatus>>` | `+ Send` | 闭包持派生台句柄 |
| `agent_protocols::mcp::stdio` 连接 | `Rc<RefCell<Connection>>` | `Arc<Mutex<Connection>>` | 同上 |
| `accounting::worker::workbench::{Desks, Workbench, Reach}` | `Rc<RefCell<_>>` | `Arc<Mutex<_>>` | 出借与收回的地方 |
| `Driving.signals` | `&Rc<RefCell<SignalDesk>>` | `Arc<Mutex<SignalDesk>>`（拥有） | 池线程不借工人的东西 |
| `storage::vfs::Vfs`（内缝） | 无上界 | `: Send` | 红测试量出的第七处：`Cas` 持 `Box<dyn Vfs>`，而 §8-43 让 `Driving` 带一份 `Cas`；`RealFs` 本来就是 `Send`，`FaultFs` 的 `Rc<RefCell<State>>` 换 `Arc<Mutex<State>>` |

**`try_borrow_mut` 失败 → 锁中毒**：`RefCell` 的「桌子在用」拒绝换成 `Mutex::lock` 的阻塞——那正是要的语义：两条线程同时到一张桌子前，后到的等，不是被拒。
`lock()` 的 `Err` 只有一种含义——持锁线程 panic 了——而发布档 `panic = "abort"` 下它不会发生；映射成 `E_STORAGE_FATAL`「桌子被一条死掉的线程留在锁里」，与 `runtime::backlog::hold` 同一句话。

**不变的**：桌子的内容、每张桌子的 `take_effects`／`take` 语义、`settle_desks` 的顺序、账本上的每一个字节。
这是一次句柄类型的迁移，不是一次行为变更；citysim 六个场景逐字节重放不变是它的验收。

**红测试**：`accounting::worker::driving::tests::turns::a_drive_can_be_handed_to_another_thread`——
`fn crosses_threads<T: Send>()` 对 `Driving<'static>`；今天这一行不编译（`Rc<RefCell<SignalDesk>>` cannot be sent between threads safely），改动后编译并通过。
类型层面的红转绿正是 ARCH §9「unrepresentable 本身是需要测试的断言」那一条的用法。

**迁移一次做完**（AGENTS.md「完成每一次迁移」）：每个读者与写者一起搬，旧形状删除，不留 `Rc` 版本的构造函数。

### 8-168 花费闸删除后的装配面

`Assignment`／`Given`／`Knock`／`Sent`／`BlockedJob` 五个结构各去掉一个 `budget` 字段，`DISPATCH_TURN_BUDGET` 与 `RunPlan.budget_turns`／`RunPlan.budget` 一并删除，`run_started` 载荷不再写 `usd_micros` 与 `tokens`，`JobBrief` 不再有 `budget` 一节，`StatusTool` 的十三字段变十二。

- **一条派活不再有回合上限**，`runtime::run::drive` 循环到这次跑自己结束为止：一回合作出结论、一次带 carrier 的失败、或一个安全点送到的中断。停一件正在跑的事仍是 `Cancel`，停一片仍是 `Halt`——后者现在真的会终止那片里的后台成员。
- **`assembly/freezing/tests/ceilings.rs` 删去两条断言**（派下去的活与被批准接着跑的活各自「在派它的上限下」跑）。它们检验的性质不存在了，留着就是在检验一个没有主语的句子；文件保留 effort 那一条，模块头写明删了什么、为什么。
- **golden-p0 账本随之重生**（`GOLDEN_WRITE=1`）：`run_started` 少两个整数键。V8 跨版本字节夹具本来就为这种形状变更而存在。
- **公开面变动**：kernel 去掉 `BudgetCap` 一族、增 `GovernedDocumentWritten`，wire 去掉 `BudgetCap` 再导出、增本线三帧与三个答面类型，web 增 `put_document_command`；下游 crate 随之编译，是公开面变更唯一的守门（`tools/xtask/Spec.lean` §8-32）。

### 8-169 治理两帧的执行与答

- `RunWorker::put_document` 写 `<city>/.sprawling/` 下三份文件之一（经 `city::write_governed`，路径由 `city::Governed` 决定而不由帧决定），随后记一行 `governed_document_written`，载荷携文件名与字节数、**恒不携正文**——正文在盘上可读，抄进账本就是同一段话有了两个权威。
- `Views` 新增两个字段：`autonomy`（折自 `autonomy_changed`）与 `decided`（折自 `approval_resolved`，旧在前）。两者一起答 `Query::Governance`。`decided` 收下每一条被答过的审批，包括人自己答的——只列代答的清单会让「我答过」与「从没人答」在界面上长得一样。
- **`views::apply` 迁入 `views::holding`**：`answering.rs` 加上这两臂后越过 400 行，而折叠本来就是 `holding` 自称拥有的东西（「what the views hold and how one record folds in」）。切完 `holding` 318、`answering` 335，无新模块行。

### 8-170 `Query::Hunks` 的答

`views::answer` 的新臂调 `storage::of_file`，把 `storage::PatchLine`／`Withheld` 逐字段搬成线上的同名形状。这座城没写过的 oid 答 `Unavailable`，与 `Changes`／`Commit` 同口径：「没有变化」与「我看不了」是两个答案，读的人对它们的下一步不同。
-/

/-!
## 8-46 同一栋楼里的并发：驾驶池、一次派活切成三段，与拿走整个 ready set 的 `pursue`

这一节把 §8-42 的设计落成类型：一条车道是什么（§8-46-3）、一次派活怎么切（§8-46-2）、
一个追求怎样拿走整个 ready set（§8-46-4）。前置是 §8-44：`Driving` 要能跨过线程边界。

### 8-46-1 `Driving` 拥有它驾驶所需的一切，`drive_dispatch` 变成自由函数

§8-44 让 `Driving<'static>: Send` 成立，但今天构造出来的 `Driving<'a>` 仍借着三样东西：
`&mut workbench.bench`、`&site.write_root`、`&site.who`，而 `drive_dispatch` 还是 `&mut self` 的方法，
用着工人的 `ledger`／`watching`／`interrupts`／`backlog`。一个借着调用栈上局部变量的值送不进线程。

| 字段 | 今天 | 此后 | 理由 |
|---|---|---|---|
| `bench` | `&'a mut ToolBench` | `ToolBench`（拥有） | `Workbench.bench` 改为 `Option<ToolBench>`，由 `take_bench` 取走一次。与 `Site.adapter` 同一手法：`Option` 是搬运的车，不是新状态 |
| `write_root` | `&'a Path` | `PathBuf` | 一次 clone，一次驾驶 |
| `who` | `&'a str` | `String` | 同上 |
| `plan`／`handoff` | `drive_dispatch` 的两个参数 | `Driving` 的两个字段 | 驾驶要的东西在一个值里，池的入口才是一个值 |

`drive_dispatch` 拆成两半：

```rust
/// 一次驾驶从工人那里拿走的四样东西，克隆而不借。
pub(super) struct DriveContext {
    watching: Option<Arc<dyn Fn(wire::Delta) + Send + Sync>>,
    person: Option<Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    backlog: runtime::Backlog,
}

/// 一次驾驶，泛型于它写进哪个账本：记账线程上是 `JsonlLedger`，
/// 车道线程上是 `Relay`，而两条路上跑的是同一段代码。
pub(super) fn drive_run<L: Ledger>(driving: Driving, ledger: &mut L, context: DriveContext) -> Result<Driven, AxError>;
```

**`interrupts` 从「借走一个」变成「每轮活各持一份」**：`RunWorker.interrupts` 由
`Option<Box<dyn FnMut(RunId) -> Interrupt + Send>>` 改为 `Option<Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>`。
`CommandDesk::interrupt_for` 本来就取 `&self` 并按 `RunId` 挑命令，所以 N 份克隆指向同一张桌子，
`steer` 与 `cancel` 各自走自己那一轮活的 `Interrupt`，这是 §8-42-1 早就写下的形状。
`take()`／放回那一对动作随之删除：一个被借走的钩子在 N 轮活同时跑时只有一个借用人。

### 8-46-2 一次派活切成三段，在飞的那些收成一张表

```rust
/// 驾驶之前城已经做完的一切，与驾驶之后要用到的一切。
pub(in crate::assembly) struct Continuation { at: Assignment, site: Site, desks: Desks, workbench: Workbench, job_locator: Locator, member: Option<runtime::BacklogId> }

fn stage_dispatch(&mut self, at: Assignment, task: String, goal: String) -> Result<(Staged, Continuation), AxError>;
fn land(&mut self, continuation: Continuation, driven: Result<Driven, AxError>) -> Result<Dispatched, AxError>;
```

**在飞的活只有一张表**，住在 `RunWorker.flight`（`accounting::worker::driving::flight`）：车道（`DrivingPool`）、
车道写历史的那道口子（`RelayGate`）、以及每一轮在飞的活欠着什么。

```rust
pub(in crate::assembly) struct Flight { pool: DrivingPool, gate: RelayGate, driving: BTreeMap<RunId, InLane> }

/// 一轮活回家之后城还欠着什么。**七个派活入口只在这里不同。**
pub(in crate::assembly) enum Owed {
    /// 人派的活：它谈过话的邻居接着答。
    Asked,
    /// 追求认领的一行计划：它回家时那一行若仍然 ready，追求停下而不是再派一次。
    Row { addr: Address, node: NodeId },
    /// 城自己起的活：排程、外来到达、敲门、批准放行。没有人在等答案。
    Unasked(Unasked),
    /// 某轮活交下来的子活：干完之后由提问的房间收 handback。
    Child { parent: Address },
}

/// 城自己起一轮活的四个理由。起不来时诊断行按理由归因，故是穷尽枚举而不是一个 bool。
pub(in crate::assembly) enum Unasked { Schedule, Arrival, Knock, Unblocked }

/// 城欠这轮活什么、拒绝回哪儿去、这条链已经走了多远。三者同行，因为义务比承载它的
/// 那轮活活得久：继任者对的是「谁要了被它替换的那轮活」，而链条的上限要看得见它
/// 已经走了几跳。`reply` 用 `Arc` 共享而不是搬走，因为落地要在把义务交给继任者
/// 之后仍能回一句拒绝。两个计数器住 `Owing` 而不住 `Owed`：它们是义务的属性而不是
/// 欠着什么这件事的属性，`Owed` 的每个读者（`discharge`、`rows_of`）也不必为它们
/// 多改一处 match。
pub(in crate::assembly) struct Owing { owed: Owed, reply: Arc<wire::Reply>, relays: Relays }

/// 一次「服务口子＋接一轮活回家」做了什么。
pub(in crate::assembly) enum Landed { Nothing, Row { addr: Address, node: NodeId }, Elsewhere }

fn dispatch_into_lane(&mut self, at: Assignment, task: String, goal: String, owing: Owing) -> Result<RunId, AxError>;
fn start_unasked(&mut self, addr: Address, task: String, goal: String, because: Unasked) -> Option<RunId>;
fn land(&mut self, continuation: Continuation, driven: Result<Driven, AxError>, owing: Owing) -> Result<Landed, AxError>;
fn serve_flight(&mut self, wait: Duration) -> Result<Landed, AxError>;
```

**一张表而不是两张，改掉的是第二个权威**。窄形里 `pursue` 自带一个池与一道口子，主循环若再开一套，
池就有两个主语（两套各自的内存闸与等待队列），而主循环服务不到 `pursue` 那道口子——
一条车道在它的 append 上停多久，取决于另一条线正好在做什么。收成一张之后：池是一个，
口子是一道，`pursue` 与 desk 派的活在同一批车道里排队，谁回家由 `Owed` 决定接着做什么。

**幂等键在「活起飞」那一刻落定**（§8-41 欠的那条语义）。理由：`Dispatch` 这条命令做完的事就是
**让一轮活跑起来**，跑本身不是命令的一部分。落在起飞：同一个键的第二帧在活还没跑完时到达，
`Entrance` 认得它，于是不会有第二轮活——这正是那扇门存在的理由。落在落地则反过来：
键要在 `serve_one` 返回之后继续被 `carrying` 持有，而 `carrying` 是「此刻正在写的记录该盖谁的章」，
两轮活同时在飞时它答不出一个。

**主循环只在消息到达与排程截止时刻醒来**（§8-42-4）：车道的 append、回家的活、desk 上的命令都把线程
唤醒，所以一条车道不必等定时器才前进；desk 刚交出一条命令时下一次看队列不睡（desk 上可能还有），
其余时候最多睡到下一个排程截止时刻（`SCHEDULE_TICK_MS`，20 s），一座闲城就是闲的。排程因此自带节律
——有活在飞时 `DeskWait::Idle` 随每次 relay 往返而来，而一座每次都打开排程文件的城把时间花在开文件上。

**关城要把车道等回来**：`DeskWait::Close` 之后不再接新活，但口子照服务、回家的照落账，直到没有活在飞，
然后才写交接。一条停在 append 上的车道被丢下，丢掉的是城已经答应它耐久的那些行。

**七个入口全部进车道，`dispatch_in` 退出生产（H-04）**。此前只有 `Command::Dispatch` 走车道，
而排程（`tick`）、外部到达（`wake`）、敲门（`answer_knocks`）、委派子活与继任（`conclude`）、
批准放行续活（`answer_approval`）六个入口调同步的 `dispatch_in`，在记账线程上跑完整个 drive。
代价不是「有界的等待」而是命令台整段关闭：`serve_flight` 只在 attending 的主循环里被调用，
一次同步 drive 持续几分钟，期间在飞的车道全部停在 relay 追加上，`Halt` 与 `Cancel` 也读不进来。

改法是「谁在等它」由 `Owed` 回答，于是那个问题不再拦路：

| 入口 | `Owed` | 落地时做什么 |
|---|---|---|
| `Command::Dispatch` | `Asked` | 醒来的邻居接着答 |
| `pursue` 的一行 | `Row { addr, node }` | 该行仍 ready 则追求停下 |
| `tick`／`wake`／`answer_knocks`／`answer_approval` | `Unasked(_)` | 无人可答，起不来时记一条 `Refuse` 诊断 |
| 委派子活 | `Child { parent }` | 子活在父 run 调用 `delegate` 或 `workshop lay_out` 时就开，图在同一刻登记给父房间（`accounting::worker::waking::handing`，collab D7），所以子活可能在父 run 落地之前落地；落地时向提问的房间投 handback 并汇入它的 join，派出那个房间的图因此就绪的节点；房间没有图、或图已汇合删去时为它敲门（`crates/collab/Spec.lean` §8 `collab::handback`、`crates/collab/spec/Workshop.lean`） |
| 继任 | 继承前任的 `Owing` | 义务随活走：链条结束时才兑现 |

**义务跟着活走，不跟着轮次走**：继任者是同一件工作接着做，所以要 handback 或要回信的那一位，
等的是链条的末端而不是每一环。`Owed` 因此不需要 `Successor` 这一格——前任的 `Owing` 原值搬给继任者即可，
谁被替换由 `Assignment.succession.predecessor` 说，一个事实一个家。

**污点随派活走，不随工人走**：`wake` 从前置 `RunWorker.tainted_arrival`、派完再清；
活进车道之后清旗标的那一刻远在落地之前，被清掉的正是那轮外来活自己的 C15 标记。
污点因此成为 `Assignment` 的字段 `taint: kernel::TaintSet`，子活与继任者继承它。
写点给出真实来路：`waking` 把到达的来路记成标签 `arrival:<source>`，装进 `Unasked::Arrival(TaintSource)` 交给
`driving::flight::start_unasked`，由 `Unasked::taint` 一处换成 `TaintSet`；其余派活写 `TaintSet::empty()`。
标签与 `runtime::bench::outside` 的 `mcp:<server>`、`web` 同一种写法，拒绝文案逐个念出来路，
模型与人都看得见是哪一处外来内容挡住了命令。
`lay_out_workbench` 把 `Assignment.taint` 原样放进这张桌子的 `TaintSet`，不再另造标签，
于是桌子问的每一道门都看得见它，`gate::command` 据此拒掉 `exec`（`E_TAINTED_ACTION`）。
读点 `settling::landing` 转交 `at.taint`，待批项的 C15 标记取 `!taint.is_empty()`，一个事实一个家。

**车道满不是拒绝，是排队**：`DrivingPool::start` 对每一个派活入口都是同一道闸——人派的活、
城自己起的活、敲门、委派、workshop、继任、追求，谁都一样。车道满时，已经准备好的那一轮活
（`Driving` 连同它的写口与 `DriveContext`）进池里的先到先起队列，不开线程；`landed` 空出
一条车道之后，`serve_flight` 在落地这一轮之前调 `start_waiting`，按到达顺序把排着的活起到
车道满为止。排在前面的先起，所以落地时新派的子活、敲门排在已经在等的活后面。
`in_flight` 只数车道里的，排队的不算；因为只有车道满才会排队，「有活在排」必然意味着
「有车道在跑」，`land_the_rest` 的循环条件因此不变。起不来的那一轮（线程开不出）离开
在飞表，拒绝交给它欠着的那一位。队列放在池里而不是放在 provider 的 admission 上，因为
池数得清谁在等，admission 只让线程停着等，一个 20 节点的 workshop 就是 20 条线程。

```rust
impl DrivingPool {
    fn start(&mut self, driving: Driving, ledger: Relay, context: DriveContext) -> Result<(), AxError>;
    /// 车道空出之后，按到达顺序起排着的活，直到车道满；返回起不来的那几轮与各自的拒绝。
    fn start_waiting(&mut self) -> Vec<(RunId, AxError)>;
}
```

**`handle` 仍然是同步的一扇门**：`RunWorker::handle` = 一条命令 ＋ `land_the_rest`，
于是命令行与测试看到的仍是「调用返回即事情做完」，而落地过程中起的子活、敲门与继任
都在同一次 `land_the_rest` 里排空。

### 8-46-3 `accounting::worker::pool`

形状：**adapter**（ARCH §9 第 4 种）。文件 `crates/accounting/src/worker/pool.rs`。

```rust
/// 一轮跑完的活回到记账线程时带的两样东西。它们成对，因为
/// 一个 `Driven` 不说自己属于哪一轮，一个 run id 也不说要归位什么。
pub(crate) struct Arrival { run: RunId, flown: Flown }

pub(crate) struct DrivingPool { /* 回家的那一头、在驾驶与在补货的车道的 JoinHandle、内存紧时排着的活、read_memory、monotonic、seat_lane、health */ }
/// 池取自这台电脑的三只手，总是一起交进来。
pub(crate) struct LaneHands { pub(crate) read_memory: fn() -> Memory, pub(crate) monotonic: fn() -> Instant, pub(crate) seat_lane: SeatLane }
impl DrivingPool {
    /// `read_memory` 是池判断内存紧不紧时唯一的读数来源；生产交 `bin::monitor::memory::read`。
    /// `monotonic` 给每一轮等车道的时长计时，`health` 是记下它的地方（与 relay 排队同一份 `Health`）。
    /// `seat_lane` 在每条车道的线程开头调一次，答的座位活到车道结束（`spec/Serving/Placement.lean` D46）。
    pub(crate) fn open(home: mpsc::Sender<Wake>, lanes: LaneHands, health: Health) -> DrivingPool;
    pub(crate) fn full(&self) -> bool;
    pub(crate) fn in_flight(&self) -> u32;
    /// 交出一次驾驶：起一条车道，内存紧时排队。run id 取自 `staged` 自己，不另传一份——
    /// 两处说同一件事就有两处说错的机会。
    pub(crate) fn start(&mut self, staged: Staged, ledger: Relay, context: DriveContext) -> Result<(), AxError>;
    /// 一轮回家后按到达顺序起排着的活，直到内存又紧。
    pub(crate) fn start_waiting(&mut self) -> Vec<(RunId, AxError)>;
    /// 把回家的那一轮从驾驶表上取下，不等它的车道补完货。
    pub(crate) fn landed(&mut self, arrival: Arrival) -> Result<Arrival, AxError>;
}
```

**一条车道就是一条只活一轮活那么久的线程**，而不是常驻的 N 条。理由：一条常驻线程在没活时持有的东西是零，
而它要活着就得有一个入口通道与一次关闭协议；一轮活一条线程把「这条线程的一生就是这轮活的一生」写成事实，
`JoinHandle` 于是同时是「这轮活还在跑」的凭据。spawn 点仍在 `bin`（ARCH §10 规则 3）。

**线程 panic 不是一种情况**：发布档 `panic = "abort"`（ARCH §2）。车道把 `Result<Driven, AxError>` 送回来，
送不回来（`join` 报错）在测试档下也只是一次 `E_STORAGE_FATAL`，而不是一个 `Box<dyn Any>` 的分支。

**车道不设上限**（`crates/sprawling/Spec.lean` D34）：一个准备好的 run 立即得到一条车道，池里没有车道数。
并发的闸是这次模型调用要去的端点的名额（`crates/gateway/Spec.lean` D17）：lane 的时间大多阻塞在网络上，
每条只多占一个缺省大小的线程栈，排队只发生在 provider 那一处，在那里计数。`accounting::worker::driving::tests::flight` 的
`tp2_more_than_four_runs_start_at_once` 守着「五轮以上同时起，没有一轮等车道」。

**内存紧时计划的下一行排队**：`full` 只在已有 run 在跑、而整机可用内存低于物理内存的十分之一时为真（`admits(in_flight, memory)` 是这一条规则的唯一出处）。三个平台的读数都经 `sysinfo`：Windows 读 `GlobalMemoryStatusEx` 的总物理内存与可用物理内存，Linux 读 `/proc/meminfo` 的 `MemTotal` 与 `MemAvailable`，macOS 读 `hw.memsize` 与 Mach 的 `vm_statistics64`（可用量把 inactive 与 purgeable 页算在内，因此比另两个平台的口径宽）。读数来自 `open` 时交给池的 `read_memory`，池自己不碰主机：生产交 `bin::monitor::memory::read`，worker 搬进 `accounting` 时 `monitor` 留在 `sprawling`、经这个 `fn` 指针进来（`crates/accounting/Spec.lean` §7、accounting D10），脚本场景交一个自己的读数就能造出内存紧的机器。`full` 每次被问都读一次，所以跟着实时的可用内存走；问它的有三处：`DrivingPool::start`（每一轮进车道都经过的门，内存紧就排队）、`start_waiting`（一轮回家后按到达顺序起排着的活）与 `Flight::full`（计划推进循环 `accounting::worker::plans::pursuing` 每次决定是否起下一行）。读一次约 1.6 µs（Windows x86-64 桌面级机器、测试档构建），只发生在起一轮之前。没有 run 在跑时总放一轮进来：否则一台内存一直紧的机器上城永远不动，而一轮自己占的内存远小于它派出的构建。排着的计划行在下一轮回家时再判一次。取物理内存的十分之一而不是一个字节数，是因为一个字节数只适合某一类机器；十分之一留给人的其他程序与页缓存。**被否**：按「每轮估计占用」算出可同时驱动的轮数——一轮的边际内存还没有测过，估计值就是一个没有来源的常数。**重开参数**：测得一轮的边际内存之后，改成「可用内存 ≥ 留给别人的那一份 + 一轮的实测边际」。证据：`crates/accounting/src/worker/pool.rs` 的 `a_new_run_waits_while_memory_is_tight` 与 `a_pool_judges_memory_by_the_reader_it_was_handed`。

**纯车道等待是一份读数**（Roadmap M2）：一轮在 `start` 时排进 `waiting` 就记下 `monotonic()`，在 `start_waiting` 里出队、车道开起时把差值交给 `Health::lane_waited`；不排队就开车道的那一轮记 0 µs 一次，于是「没有一轮等车道」可以被读出而不是被推断。`Health::lane_wait_us` 按到达次序读出（至多 `RELAY_QUEUE_KEPT` 个，最旧的先丢），`instrument_throughput` 把它印成 `wait=lane_pure`。读数是内存里的计数，不进账本；时钟是 worker 的同一个单调时钟，Windows、macOS 与 Linux 上由标准库各选单调源，读法相同。

### 8-46-4 `pursue` 拿走整个 ready set（`accounting::worker::plans::pursuing`）

`pursue` 住 `plans` 下自己的模块（`plans.rs` 留 `set_pursuit` 与读计划的那几个私有方法；子模块看得见父模块的
私有项，所以没有字段因此变公开）。**它由事件推进，不是命令里的一个循环**：命令台只在主循环里读，
一个在 `Pursue` 命令里转到底的循环会把 `Pause`、`Clear`、`Halt`、新派活、审批与排程全都关在门外。

1. `set_pursuit` 记下 `pursuit_changed` 之后调一次 `pursue`：读整个 ready set，对其中每一个节点，只要车道没满就
   `stage_dispatch` 并交给池，然后**返回**，命令台随即空出来。
2. 车道满了就不再取：`kernel::observe(state, &ready, in_flight)` 的 `in_flight` 是真值，
   于是「没什么可取但有人在跑」如实答 `Waiting { in_flight }`。
   **已经在别人手上的节点不在问它的那个 ready set 里**：ready 的意思是「现在可以有人接手」，
   而一个已经被接手的节点不能被接手两次。过滤在调用点做，依据仍然是 `kernel::pursuit` 的。
3. 每一轮活落地之后，`serve_flight` 调 `advance_pursuits`：落地的是某个追求的一行（`Landed::Row`）而那个节点
   仍然 ready，说明这一轮什么也没认领——记一条 `Refuse` 诊断，并经 `set_pursuit` 把这个追求**暂停**，
   `pursuit_changed` 因此写下 `pause`，页面不再说「working on …」，人 `Resume` 即可再试；然后对每一个
   追求再取一次 ready work。任何一轮活落地都可能空出车道或让新节点变 ready，所以不只看自己的行。
4. 这一步取活失败（计划读不出、派活被拒）落一条 `Refuse` 诊断，不让刚落地的那一轮活的结局跟着失败：
   两件事没有因果。

**账本上的写者仍然只有一个**：车道线程手上唯一的 `kernel::Ledger` 是 `Relay`，`JsonlLedger` 一步不离记账线程。
`RelayGate` 由 `pursue` 自己开一扇，发出去的 `Relay` 与它一一对应；`assembly::attending` 主循环服务的那一扇仍在，
属于同一张 `Flight`（§8-46-2），于是 desk 派的活与追求派的活在同一批车道里排队。

**评审楼一个房间一棵 worktree 是既有事实，只验证不重做**：`stand_up` 按房间地址给工作树起名，
三轮活分属三个房间时就是三棵树。同理，Handoff 住房间而不住楼，
三轮活分属三个房间时各写各的 `Handoff.md`；这也是只读不改的东西。

### 8-46-5 citysim：一条账本、两轮活、逐字节重放

池大小 1 是模拟器的确定性条件（§8-42-3），所以 citysim 不跑池。它要证的是另一半：
**两轮活的记录交错在同一条账本上时，同一批脚本重放出同样的字节**。
`run_scenario` 今天每次自己造一个 `MemLedger`，于是「两轮活一条账本」在 citysim 里根本拼不出来。
这里加一个入口：

```rust
pub fn run_scenario_on(ledger: &mut MemLedger, scenario: Scenario) -> Result<ScenarioReport, AxError>;
```

`run_scenario` 变成「造一条账本，调它」——一个权威，两个调用方。新场景
`two_runs_interleaved_on_one_ledger_replay_byte_identically`：两个 run id、两个房间、各自的脚本模型，
跑在同一条 `MemLedger` 上，`seq` 单调、`prev` 成链（`check_chain`），再跑一次逐字节相同。

### 8-46-6 文档停止过度承诺

`README.md` 两处写着「多个 agent 同时工作」。此前这句话在产品里没有主语：`pursue` 一次只跑一轮活。
此后它成立，但只在一个位置成立，文档必须把那个位置说出来，而不是继续说一句听上去更大的话。
`ARCHITECTURE.md` §11 的性能记录增一行**并发墙**：一座城同时驾驶的轮数由车道数决定，
而车道之外的第一堵墙是 provider 的 admission 天花板；账本仍然是串行的，那是 §10 规则 5 的价钱。

### 8-46-7 验收

1. **红转绿**：`crates/accounting/src/worker/plans/tests/graph.rs` 的 `three_ready_nodes_drive_three_runs_at_once`——
   三个 ready 节点、一个 pursuit，假 provider 记下同时在飞的请求峰值。串行时峰值为 1，红就红在这里。
2. **红转绿**：`tools/citysim/tests/scenario.rs` 的 `two_runs_interleaved_on_one_ledger_replay_byte_identically`。
3. **红转绿**：`accounting::worker::driving::tests::flight::two_dispatches_from_the_desk_drive_at_once`——
   两次派活各自进一条车道，主循环接它们回家；账本 `seq` 单调、`prev` 成链，
   每一轮活的 `run_started` 排在它自己的 `model_called` 之前，且第一轮冻结之前两轮都已开始。
   串行时红在最后那条：第二轮要等第一轮冻结之后才开始。
4. `cargo clippy -p sprawling --all-targets --all-features --locked -- -D warnings`、
   `cargo nextest run -p sprawling --locked --all-features`、`cargo nextest run -p citysim --locked --all-features` 绿。
5. `cargo xtask modmap`、`length`、`header` 绿。

### 8-46-8 车道数不是一个常数

池不持有车道数（D34）。并发上限只有一个权威，是每个端点的名额 `gateway::concurrency`（`crates/gateway/Spec.lean` D17）；
池若再持有一个数，它就成了与任何 provider 都无关的第二道闸，队列会停在一个不知道为什么在等的地方。
-/

/-!
## 8-49 `Reviewed-by:` 说的是真话：这次合并没有人看过（`accounting::worker::reviewing`）

合并落成一个真正的合并提交，于是 `settle_requests` 给 `storage::Landing` 传的是写死的 `reviewed_by_person: true`。**那是一句写进永久历史的假话**：`PrEffect::Merged` 里的 `by` 是 `PrDesk::who`，也就是跑这次检查的那个 resident 的地址；评审楼的全部意思正是「另一个 resident 检查它」，而不是「一个人看过它」。运行中的机器的 git config 里若有 `user.name`／`user.email`，那句写死的 `true` 就会把仓库主人的名字挂到一份他从未读过的改动上。

**改为 `false`，并写下它为什么恒为假**：今天这条路径上不存在人的复核——`pr` 工具由模型调用，`who` 恒是城里的一个地址。`storage::Landing` 的这个字段不因此作废：它的两个取值在 `storage` 那侧各有一条断言（`a_person_who_looked_is_named_from_the_repositorys_own_config` 判真，`a_merge_lands_as_a_two_parent_commit_carrying_the_merging_runs_trailers` 判假），这里在 `bin` 这侧加第三条，从城外读回 trunk 的提交消息作证。**翻案条件**：当合并这一步真的经过一个人（例如合并成为一件需要 Approval 的事，由 `kernel::Answerer::Human` 答复），这里的取值就由那次答复得出，而不是再写一个字面量。
-/

/-!
## 8-171 一次回合一个内容库句柄（`accounting::worker::credentials::endpoints`）

`redemption()` 造的取图闭包里写着 `storage::Cas::open(&cas_dir)`：**每张图开一次库**。一次带四张图的回合就开四次，每次都要建目录、探路径；而内容库是按内容寻址的只读读取，一个句柄答得了整场对话。

**改法**：`redemption` 在造闭包之前开一次库，把它放进 `Arc<Mutex<storage::Cas>>` 让闭包捕获。`Mutex` 不是为了并发而是为了类型：`ImageResolver` 要求 `Send + Sync`，而 `storage::Cas` 的 `Vfs` 缝只承诺 `Send`——`Mutex<Cas>` 在 `Cas: Send` 时即是 `Sync`，这比把 `Vfs: Send + Sync` 拓宽给所有适配器要窄。

**签名随之带上失败**：`redemption(&self) -> Result<gateway::Redemption, AxError>`，因为开库会失败，而失败的时刻从「第一张图到达时」提前到「装配适配器时」——这正是想要的：一个读不了自己内容库的城，应当在造适配器时说出来，而不是在模型已经开口之后。两个调用点各改一处（`dispatching::agreeing` 用 `?`，`dispatching::session::name_the_work` 在 Option 语境里用 `.ok()?`）。

**本节的断言把 `credentials/tests.rs` 顶过 400 行，故它按责任一分为二**（`length` 门报的红，修的是因而不是门）：`credentials/tests/endpoints.rs`（一座城够得着哪些模型，以及适配器在线上兑现什么）与 `credentials/tests/signing.rs`（凭证怎么进城：录入），`tests.rs` 只剩两行 `mod`。切分对着源文件的两半（`endpoints.rs`／`signing.rs`）而不是对着行数切。

**未由机器作证的那一半，写在明处**：「只开一次」本身没有断言，因为 `storage::Cas` 不数自己被开过几次；加一个计数缝只为这一条断言，代价大于它买到的东西。作证的是行为面——一次 `redemption` 解得开对话里的每一张图。
-/

/-!
## 8-152 页面保存一份文档、决定修改提案、读提交说明，服务中的城怎么做（`accounting::worker::commanding::saving`、`accounting::views::proposals`、`accounting::views::commits`；`crates/wire/Spec.lean` §8-72、§8-73、§8-54）

- **两条写命令走 worker 的同一扇门。** `PutRange` 与 `DecideProposals` 经 desk 进 `run_command`，各有一臂，交给 `commanding::saving`（`crates/accounting/Spec.lean` §8-22）；`idem` 由 `commanding::entrance` 判，所以页面丢了答复再发一次，城答第一次的结果，不写第二行。
- **回执是事件流上的一行。** 成功的保存写 `document_written`，带这条命令的 `idem`；页面在 `core/belief` 的折叠里见到带自己 `idem` 的那一行才把「保存中」换成「已保存」，`version` 换成新的基线。`client/src/core/staleness.ts` 让这一行使 `document` 与 `proposals` 两种答复过期，页面再问一次就读到新版本与剩下的卡。
- **`Query::Proposals` 读视图里的 `Governance.proposals`**，锁外读盘取文件此刻的版本（同 §8-52 的 `document`，§8-100 的锁外规矩）。
- **提交说明随父提交一起读**：`Query::Commit` 与 `Query::Commits` 的 `CommitsAsk::read` 在锁外各补 `parents` 与 `message`（§8-128）。
- **客户端今天只有发出点，没有画面**：`client/src/core/commands/document.ts` 拼 `put_range` 与 `decide_proposals` 两种帧；编辑页与请决定卡还没有画出来，画它们的依据是 `crates/wire/Spec.lean` §8-72、§8-73 与 refrain 路线图 §4-8、§4-10。
- 验收：见 `crates/accounting/Spec.lean` §8-22 列的测试；远程门对两条命令的分类见 D26「写文档的两条命令在远程门外」。
-/

/-!
## 8-55 一栋楼的桌面白名单，走配置那条帧（`accounting::worker::commanding::configure`；`crates/wire/Spec.lean` §8-26、§8-45、`crates/city/Spec.lean` §8-26）

城这一侧的两件事（楼级 `desktop:` 与 `kernel::gate::undoable`）早已落地，缺的是**把那份 allowlist 从人手里送到盘上的那一段**。

- **不新起一条命令**：`ConfigureBuilding` 问的就是「这栋楼的 runs 够得到什么」，沙箱、外部服务器与运行中的机器上的窗口是同一个问题的三面，各自可缺省。
- **不解析**：`city::write_desktop_scope` 整份覆写，字节即人给的字节。语法的权威是读它的那台 server，且它 fail closed。
- **载荷是四个面**：`city::Written { sandbox, mcp, desktop, context }` 取代四个裸布尔——调用点写 `(true, false, true, false)` 说不出哪一位是哪一面；四个面进来时也已经是一个值（`commanding::configure::Reconfiguration`），所以 `configure_building` 收两个参数而不是五个。
- **页面**：`client/src/views/desktop.svelte` 一个框装整份文件，读用 `Query::Document`（`<building>/.sprawling/DESKTOP.toml`），写用 `configure_building`。一个「每个窗口一行」的表单会是这一侧对那份语法的第二次解读。
- **验收**：`crates/accounting/src/worker/building_page_tests.rs` 的 `the_desktop_allowlist_is_written_where_no_resident_reaches_it`——人写的字节落在 `desktop_scope_path` 上，且那条地址 `is_reserved` 为真（任何写域都够不到）。
-/

/-!
## 8-131 城给 run 的转写工具 `transcribe`（`accounting::worker::workbench::tools::transcribe`；`crates/gateway/Spec.lean` §8-12、§8-33、§8-34，`crates/runtime/Spec.lean` §8-59）

**原因**：computer use 要能把声音变成字，而二进制里不内置任何模型（定规），所以模型只能经人接入的转写端点转写。composer 的麦克风已经经这个端点把录音变成字（§8-56），run 却没有任何一条路用它。

```rust
// accounting::worker::workbench::tools::transcribe（形状 4 适配器）
impl Laying {
    pub(super) fn transcription_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<TranscribeTool>, AxError>;
}
pub(super) struct TranscribeTool { /* reader: runtime::BoundReader、transcriber: Mutex<Transcriber>、meta —— 私有 */ }
impl kernel::Tool for TranscribeTool { … }   // 名 `transcribe`；参数 `{ path }`：路径或 `cas:`／`file:` Locator；答 `{ path, text }`

// accounting::worker::workbench::Laying 多一格
book: gateway::EndpointBook,   // 派活立起那一刻的端点账本
```

- **有没有这件工具，与 composer 的麦克风读同一个选择。** `EndpointBook::select(ModelTag::Transcribe, 楼的 policy)` 成了，`gateway::transcriber_for` 把它变成设施，工具上表；拒了，工具不上表。没有第二个「这座城能不能转写」的开关。与 `views::hearing` 只差 policy：run 有楼，读楼规，于是机密楼只拿得到回环地址上的转写端点；口述没有楼，按普通楼。`select` 的三种拒绝——没选、端点已撤、机密楼而端点离机——对模型是同一件事：这座楼没有转写，一件叫了必败的工具不放上表。`transcriber_for` 的失败（HTTP 客户端造不起来）照常上抛，因为它对主模型的调用同样成立。设施放在一把锁后面：它里面的凭据解析器是 `Send` 而不是 `Sync`，而一件工具要在一波调用之间共享，于是同一个 run 的两次转写轮流进行。
- **账本在 `RunWorker::laying` 里复制一份进 `Laying`。** 工作台在驾驶这个 run 的 lane 里摆（§8-113），账本属于 accounting 线程；`Laying` 本来就带着「派活立起那一刻」的值（`waiting`、`locks`、`trust`），账本是同一种值，几个端点的复制是微秒量级。被否：在 `agree_to_work` 里造好设施随 `Agreed` 带过去——那样工具在一处造、在另一处登记，「这件工具有没有」就有两处答案；只把 `Transcriber` 放进 `Laying`——`laying()` 手里没有楼规，造不出对的 policy。
- **录音从读界之内的一个文件或一个 `cas:` 块进来，只经 `runtime::BoundReader`**（`crates/runtime/Spec.lean` §8-59）。参数与 `read` 同一种写法：相对城根的路径（审查楼里是这个 run 的树）、城内的绝对路径，或 Locator。连接器把 desktop 的录音存进 CAS，窗口里那一行 `[recording attached: …, cas:…]` 的 locator 原样交给它（runtime D15）。判定是 `read` 的那一份：文法、reserved subtree、读界，链接解开后再判一次，打开后核对没换过；拒绝与 `read` 同码。读界之内的别楼也收：模型能 `read` 的东西本来就进了它对主模型的对话，录音出门去的是按本楼的 policy 选出的端点，与那段对话同一种出门；机密楼的文件楼外读不到，机密楼里的 run 只拿得到回环上的转写端点。**容器**：文件（`file:` 在内）看扩展名（`AudioType::of_file_name`），字节经 `Recording::read_from` 读到上限为止（`crates/gateway/Spec.lean` §8-33）；块没有名字，看开头的字节，经 `Recording::read_unlabelled`（`crates/gateway/Spec.lean` §8-34）。
- **效果是 `Effect::Read`。** 它读一个文件，城里什么都不写；录音出门去的是人为这类活选定、已按本楼 policy 判过的端点，与 run 的对话去主模型端点是同一种出门，而模型调用不是工具效果。被否：`Effect::Egress`——那一类的定义是「目的地由这次调用指名」，出网门扫的是参数里的字节，而这里参数只有一条路径，出去的是录音，判它的是已经判过的 `select`。`CostTier::Heavy`：一次 provider 往返，数秒，可能计费。`timeout: None`：设施自己的 `TRANSCRIBE_TIMEOUT_MS` 已是上限，第二个期限会是第二个权威。`render: Generic`：它产出的是文字。
- **在目录里的位置**：它不在任何 mode 的常驻核心（`runtime::mode::core_tools`），所以不进工具表，只在休眠索引里按名字的字节序占一行，经 `describe` 与 `call` 用（`crates/runtime/Spec.lean` §8-60）；登记次序不再决定它在请求里的位置。
- **验收**：`crates/sprawling/tests/acceptance/episodes.rs` 末尾一段。选了转写端点的城，run 调 `transcribe` 拿回脚本端点转出的字，端点收到的模型名是人选的那一个；机密楼配离机转写端点时，工具不在表上。没选转写的城不上这件工具，由 catalogue 的两条覆盖测试守着：上了表而没被调用，会被点名。连接器存下的录音：一个为这座楼存进 CAS 的 wav 块，run 以它的 `cas:` 调 `transcribe`，拿回脚本端点转出的字。工具自己的拒绝（机密楼的路径、reserved subtree、认不得的容器、不存在的文件与块）由 `transcribe` 模块的测试守着。
-/

/-!
## 8-142 城给 run 的 OCR 工具 `ocr`（`accounting::worker::workbench::tools::ocr`；`crates/gateway/Spec.lean` §8-34，`crates/runtime/Spec.lean` §8-59）

**原因**：computer use 读不到字的窗口（画在画布上的界面、远程桌面）只剩截图，模型要文字反馈得有 OCR；二进制里不内置任何模型（定规），所以 OCR 与转写一样经人接入的端点。人在端点账本里为 `ModelTag::Ocr` 选一个能读图的模型，城给 run 一件 `ocr`，把 run 有权读的一张图交给那个模型，取回文字。desktop 那一侧不做 OCR，截图经连接器进 CAS（`crates/desktop/Spec.lean` §15.2「还欠的」第 1 条）。

```rust
// accounting::worker::workbench::tools::ocr（形状 4 适配器）
impl Laying {
    pub(super) fn ocr_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<OcrTool>, AxError>;
}
pub(super) struct OcrTool { /* reader、policy、recogniser: Mutex<gateway::Recogniser>、meta —— 私有 */ }
impl kernel::Tool for OcrTool { … }   // 名 `ocr`；参数 `{ path }`：PNG 的路径或 `cas:`／`file:` Locator；答 `{ path, text }`
```

- **有没有这件工具，读的是 `select(ModelTag::Ocr, 楼的 policy)`。** 成了，`gateway::recogniser_for` 把它变成设施（带上这个 face 的头，与主模型的适配器同一张），工具上表；拒了（没选、端点已撤、机密楼而端点离机），工具不上表，理由同 §8-131：一件叫了必败的工具不放上表。
- **图只经 `runtime::BoundReader` 读进来**，与 `transcribe` 同一扇门、同一份判定（§8-131）：楼里的一个 PNG、读界之内别楼的一个 PNG、连接器存进 CAS 的截图（窗口里那一行 `[picture attached: image/png WxH, cas:…]` 的 locator）都收。读进来的字节交 `runtime::pipeline::connector::png_picture`，与连接器存图同一种认法：只认 PNG，超过 `IMAGE_MAX_BYTES` 或头读不出，`E_INVALID_ARGS`。
- **发出去的是所选 face 的图片内容**（`crates/gateway/Spec.lean` §8-34）：一条 system、一条带图的 user 消息，没有工具；`policy` 是这座楼的。选中的模型登记为只读字时，端点在发出前拒绝，拒词让人把 `ocr` 指向一个 `text_image` 的模型。
- **效果照 8-131**：`Effect::Read`（读一个文件，城里什么都不写；图出门去的是按本楼 policy 选出的端点）、`CostTier::Heavy`（一次 provider 往返，可能计费）、`timeout: None`（端点自己的调用期限是上限）、`render: Generic`（产出的是文字）。
- **在目录里的位置**：它不在任何 mode 的常驻核心，所以不进工具表，只在休眠索引里按名字的字节序占一行，经 `describe` 与 `call` 用（`crates/runtime/Spec.lean` §8-60）。
- **验收**：`crates/sprawling/tests/acceptance/ocr.rs`。选了 OCR 模型的城（回环端点、chat 面），run 以楼里的一个 PNG 与一个为这座楼存进 CAS 的截图各调一次 `ocr`，拿回端点读出的字；端点收到的两次请求走 `chat/completions`、带人选的模型名与那张图的 base64。没选 OCR 的城不上这件工具，由 catalogue 的两条覆盖测试守着。工具自己的拒绝（不是 PNG、机密楼的路径、没有设施）由 `ocr` 模块的测试守着。
- **本节接口的当前状态**：线上已有的 `SelectModel` 能为 `Ocr` 选模型，登记的 `input` 是 `gateway::accepted_input` 那架梯子的答案（`crates/gateway/Spec.lean` §8-37）：钉版目录、预置表、`Text`，先说者胜。所以预置表写着 `text_image` 的模型（例如 `api.anthropic.com` 上的 `claude-sonnet-4-5`，或中转站转发的同一个 id）选为 `Ocr` 之后，`ocr` 在它上面读得出字；两张表都不认识的模型仍按 `Text` 登记，`ocr` 在它上面由端点拒绝。梯子最高的一档——人自己说「这个模型读图」——的线上字段已经落地：`SelectModel.input: Option<InputKinds>`，出现时排在目录之前，一个目录写着 `text` 的模型带 `input: TextImage` 选为 `Ocr` 之后，`ocr` 在它上面读得出字（gateway D16）。只差设置页的控件（client/Spec.lean 的模型表仍是三行，`crates/kernel/Spec.lean` §8-80），控件归前端；页面今天不带这一格（线上缺席即 `None`），梯子照旧。
-/

/-!
## 8-172 提示词语料的分层：哪类事实住哪一层（`crates/city/templates/City.md`＋`ToolMeta`＋`Catalog`）

**依据是成本的形状，不是篇幅的偏好。** 四段前缀与工具 schema 在 `runtime::turn` 的**每一趟请求**里全文重发（`turn.rs:108-117`），于是同一批字节有两种代价：**窗口**是进上下文的一次性入场费（请求累积，前缀不随 turn 增长），**钱**是每 turn 重付（`prompt_cache_breakpoint` 命中后按 `cache_read_price` 折价）。两种读法下窗口占用完全相同，所以「多写一句」永远是全城每次请求少一份工作空间。

由此定下分工，`ToolMeta` 的两个字段各管一半：

| 层 | 字段 | 落点 | 只写 | 执法者 |
|---|---|---|---|---|
| 目录行 | `disclosure` | 常驻核心的工具随 `tool_defs()`；其余工具的第一句截成休眠索引的提示进 Resident 段，全文只经 `describe`（`crates/runtime/Spec.lean` §8-60） | 它是什么、什么时候该用 | `claim_tool/tests.rs` 的 548 B 预算 |
| 说明书 | `params` 各字段的 `description` | 常驻核心的随 `tool_defs()`；其余只经 `describe` | 怎么用：字段、取值、默认、拒绝条件 | 同上（量的是两者之和） |
| 二级展开 | `CatalogEntry::expansion` | 按需 `read` | 整套纪律 | — |

`plan` 的六个动作就住在说明书里（`action` 那句），目录行只剩 84 B 余量而原文已占 83 B——**把动作抄进目录行既付两遍钱也放不下**，那条预算就是这个决定的执法者。`signal` 的四类 kind 同理。

**搬走的**：市长与书记的角色描述（`crates/city/templates/MAYOR.md`、`CLERK.md` 已逐条写着）、`signal`／`goal`／`pr` 的动词解释（各自 disclosure）、委托的一层上限（`delegate` 的 disclosure）、模式语义（`Mode::catalog_entry()`）、六份文档清单（各模板开头的自述已逐条重复）、浏览器语义（`BUILDING_DISCLOSURE`）、Python 子集（`exec` 的 `arm` 描述）。

**补进 City.md 的**：读全再动手、外置记忆（照模板写、按需读）、通信（先读别人留下的、加入前先问、方式由现场定）、隐私（**上下文本身是泄露面**，不问不找不需要的隐私与密钥值，拿到就叫人换）、环境探测、licence 与 copyright、引用保留完整上下文与出处、重要事实对第二来源交叉验证。这些都是四类身份都成立的话，才留在这个每跑都要付的段落里。

**一句话规则**：City.md 的每一句都必须对 `EPHEMERAL_SEGMENT` 扮下的工人成立——它只有 `JOB.md`、不共享楼、不写 Memo。不成立的降级到 `RULES.toml` 或模式的 catalog 条目，**不降级到 `URBANITE.md`**：那份文件坐在常驻自己的地址上，它改得动，把城级规则放进去等于让被约束者起草规则（`hall.rs` 把市长与书记的身份放在保留子树，理由同一条）。

**`citysim::ablation` 是这一章的尺**：它按段落切除文档、报出每段独占哪些能力。当前读数是 12 段、32 项能力、**全部独占**（无一 `Restated`），即每项能力恰好一个家。
-/

/-!
## 8-107 回收站的一行放回原处（`accounting::worker::commanding::restoring`，形状：适配器）

```rust
impl RunWorker {
    /// `Command::RestoreDiscard` 的执行者。
    pub(in crate::assembly) fn restore_discard(&mut self, restoration: &kernel::Restoration) -> Result<(), AxError>;
}
```

- **先写盘，后落账**：`Tracked(file:<addr>@<oid>)` 经 `storage::Checkpoint::restore` 把那个 blob 写回城根下同一路径，成功之后才追加 `discard_restored`。反过来的次序会让历史说一个文件回来了，而盘上没有它。载荷与被关掉的那条 `file_discarded` 同形（`paths: ["file:<addr>"]`、`restoration`），于是 `DiscardView` 用同一个 `discard_lines` 读两种记录，按路径关掉那一行（§8-6）。
- **写回的是城根，不是 lease 的 worktree**：检查点的对象在城的对象库里，一个评审运行的 worktree 与城共用它；人要回的是自己丢的文件，放回主干的那个位置。
- **拒绝**：`Interred` 与带 `range` 的定位符答 `E_INVALID_ARGS`（前者从内容仓库取回尚未接线，后者不是整个文件）；`Rebuildable` 答 `E_INVALID_ARGS`，recovery 就是那条重建的理由；提交找不到或路径不在提交里，是 storage 的 `E_WORKTREE_BUSY`，原样上抛。
- **被否：按路径在写线程上查回收站**。写线程不持有 `DiscardView`；为一次还原把整份历史再折一遍，是在人按下按钮的那一刻付一整次重放。页面手里的那一行已经带着路（`crates/wire/Spec.lean` §8-47）。
-/

/-!
## 8-108 开城时收走崩溃留下的树（`accounting::worker::lifetime`；`crates/storage/Spec.lean` §8-9）

**原因**：评审楼的树按房间保留（§3），归还后留在盘上等同一房间的下一轮活；开城时账本的写锁保证没有 run 持有任何树，所以城造过的每一棵树都是上一次服务（包括崩溃的那一次）留下的：git 的登记、保留子树下的目录与 git 为它建的分支，一棵最多 `WORKTREE_MAX_BYTES`。

- **在哪里做**：`RunWorker::over` 在开账之后调 `storage::Worktrees::sweep_abandoned(city_root, &[])`。`held` 为空，因为账本的独占锁保证此刻没有别的进程在用这座城，而这个 worker 还没有派出任何一轮；`over` 是 `serve`、`resume` 与 `form_city` 共同的构造点，所以每条开城路径都清扫。
- **失败不挡开城**：清扫失败（例如 Windows 上一个文件还被别的程序打开）以 `Level::Effect` 写进 `Diagnostics`，城照常打开；留下的树下一次开城再收。一棵收不走的树不该让人进不了自己的城。
- **收走了什么也说**：收走至少一棵时，同一级别写一行，列出名字；什么都没收时不写。
- **不碰的东西**：人加的 worktree、人的分支、带着未进 HEAD 的提交的租约分支、`refs/sprawling/runs/` 下的检查点引用（`crates/storage/Spec.lean` §8-9）。

**本节测试**：`storage::worktree::sweep::tests` 的四条——崩溃留下的租约被收走（`a_crash_left_lease_is_swept_when_the_city_opens`）；检查点引用留下；另一座城的家目录下的树留下；活着的 run 的树与人自己的树留下——以及 `accounting::worker::lifetime` 的 `a_crash_left_worktree_is_gone_once_the_city_opens`，它从开城这一侧看同一件事。
-/

/-!
## 8-127 进程死在写账的半途之后，城照规则重开（验收：`crates/sprawling/tests/acceptance/crash.rs`）

**要什么。** 服务进程在写一行账的半途被杀，或机器断了电，人再打开这座城时，城说出它截掉了什么，链照样验得过，丢了答复的调用关成「结果未知」而不是「失败」，视图从幸存的历史与补写的行折出，那间房照样派得出活。这一条把这几件事放在同一座真城上一次验收，作为 V0.1.0 崩溃验收（G5）的白盒证据。

**怎么造一次死亡。** 一座真城，脚本化的模型（`ModelFactory` 缝）让一次 run 调一次 `status`；run 写完后丢掉 worker——这释放写者锁，与被杀的进程释放它一样——然后在盘上把账截在回答那次调用的 `tool_result` 那一行的一半处，那一行之后的行与之后的段都不存在：它们正是一个死在那一刻的进程来不及写的东西。之后经 `RunWorker::new` 重开，跑 `startup_scan`（`sprawling resume` 的那一遍），再经一次性查询 `accounting::views::ask` 问城景，最后向同一间房再派一次活。

**验收的五件事，一个值比较**（`Reopened`）：

1. `startup_scan` 报 `TailDropped { bytes }`，字节数就是被截的那半行的长度（§8-102）；
2. 账里多一行 `log_truncated`，整条链经 `verify_ledger_dir` 验过；
3. 那次调用恰有一个答复，码是 `E_TOOL_OUTCOME_UNKNOWN`，`startup_scan` 报关了一个（ARCHITECTURE §5 末）；
4. 城景列出的那次 run 已冻结，结局是 `cancelled`，`last_kind` 是启动扫描写的那行 `run_frozen`、`last_seq` 是它的序号，账上那一行带 `cause: process_died`——视图从截过、补过的历史折出，没有拿一份比账长的快照作答（§8-91、§8-101），死掉的 run 按 ARCHITECTURE §13.7 的 `Lost --> Frozen` 冻结（`crates/kernel/Spec.lean` §8-82-2，`crates/accounting/Spec.lean` §8-18-1）；
5. 同一间房的下一个任务立起一次新 run 并冻结它：死掉的 run 没有把房间占住。

**能咬住，量过。** 把 `startup_scan` 里补写结果未知的那一步拿掉再跑，测试红在第 3、4 两项上（答复为空，城景的 `last_kind` 停在 `tool_called`）；不冻结死掉的 run 时，红在第 4 项上（城景把它列为未冻结，`last_kind` 停在补写的 `tool_result`）。

**断电那一半归 storage。** 断电比被杀多丢的，是平台还没落盘的字节；那是账本自己的耐久契约，由 `storage` 在它的故障文件系统上证（`power_cut_matrix_over_every_op_keeps_acknowledged_waves`，`crates/storage/Spec.lean` §8-2）。这里不重证它：故障文件系统只承载账本，城的其余文件与 `Standing::fold`、视图的折叠读的是真目录，在它上面重开的不是一座完整的城（D13「崩溃验收在盘上造死亡」）。

**另外三个死亡点，同一种造法、同一个结论（不丢一行、不重一行）。** 都在盘上造，所以 Windows、macOS、Linux 上是同一组字节：被杀的进程（Windows 的 `TerminateProcess`、别处的 `SIGKILL`）留下的正是这些。
- 派活的 job 已进内容存储、引它的 `checkpoint_committed` 行还没写（`a_city_killed_between_a_job_put_and_its_checkpoint_line_reopens_with_nothing_to_cut`）：没有要截的、没有要冻结的，房间照样派得出活。
- 一道 git 检查点的提交与它的引用已经进了仓库（比较后交换已经做完），带 `oid` 的那行还没写（`a_city_killed_after_a_checkpoint_swap_and_before_its_line_loses_and_doubles_nothing`）：检查点在它守的那一波之前，所以那一波的调用没写进账，它写的文件也从盘上拿掉；重开的城不替死掉的 run 记这道提交（账里没有一行提到那个 oid），也不替它重做那次写；引用留在仓库里，是崩溃的写者只留下的垃圾（storage D25）。
- 一波多行的屏障做到一半：两次模型调用之间那一波（头一次的回答、它要的读调用与那次调用的结果，回合在下一次模型调用之前一道屏障写下，runtime D36），第一行整行落了盘，第二行落了一半（`a_city_killed_inside_a_barrier_keeps_the_whole_lines_once_and_cuts_the_torn_one`）：这一波没有答过 `Ok`，谁也没被告知；重开的城截掉那半行并说出截了多少，整行的那一行恰出现一次，死掉的 run 冻结，房间照样派得出活。

**死掉的 run 由谁冻结。** `startup_scan` 补完悬空调用之后，为每一次有 `run_started`、没有 `run_frozen` 的 run 写一行 `RunFrozen::lost()`：结局 `cancelled`，载荷 `cause: process_died`，作者是那次 run 的居民（`crates/accounting/Spec.lean` §8-18-1；为什么不是第四种结局，见 kernel D14）。所以第 4 项比的是城景里那次 run 的整行状态：已冻结、结局、最后一行与它的序号。
-/

/-!
## 8-114 移走一栋楼（`accounting::worker::commanding::removing`，形状 3 决定；`crates/wire/Spec.lean` §19、`crates/city/Spec.lean` §8-3）

```rust
impl RunWorker {
    pub(in crate::assembly) fn remove_building(&mut self, addr: &Address) -> Result<(), AxError>;
}
impl RoomQueues {
    pub(in crate::assembly) fn worked_within(&self, building: &Address) -> Option<(Address, RunId)>;
}
```

- `Command::RemoveBuilding` 走这里。城拥有 `city::remove_building` 不知道的两件事：楼里有没有 run 正在跑，以及 Ledger 记什么。
- **一个拒绝**：楼里某个房间的队列正借给一个 run（`worked_within`），拒 `E_BUSY`，主语点名房间与 run，恢复是先停下它。房间队列是城里唯一说「这里有人在干活」的账，另开一本会与它分歧；逐个看全部房间，因为地址序把 `lab-2` 排在 `lab` 与 `lab/room1` 之间，按前缀截区间会漏。
- **先搬后记**：文件先搬进 `.sprawling/removed/`，再写 `building_removed`（载荷 addr、kept），因为这一行说的是「已经搬了」。楼以前写过的每一行都留在账里，什么都不删。
- 客户端（`client/src/core/removal.ts`）在 City Hall 上不画这个控件，楼里有 run 在跑时画成不可用并说明原因；确认走 `parts/dialog`。城仍自己拒这两种情况，页面只是不把一个只会被拒的按钮交给人。

### 8-115 还在跑的命令写出的字节，给后来打开页面的会话留一段（`bin::serving::output_ring`）

`keep` 必须守住的性质的权威是 `crates/sprawling/spec/Serving/OutputRing.lean`：最新的一块总留着，每个 run 不超过上界或只剩一块，只从最旧的整块丢起；本节是接口与理由。

```rust
// bin::serving::output_ring —— shape: value
pub(super) struct OutputRing { /* Mutex<BTreeMap<RunId, Kept>> */ }
impl OutputRing {
    pub(super) fn keep(&self, piece: &wire::LiveOutput); // 追加；超过上界时丢最旧的整块
    pub(super) fn settle(&self, record: &EventRecord);       // tool_result 落账：清空这个 run
    pub(super) fn so_far(&self) -> Vec<wire::LiveOutput>; // 每个 run 按到达次序
}
```

- **没有失败返回**：它存的是可丢弃的预览（`crates/runtime/Spec.lean` §8-28-3），账本里的结果才是权威；锁中毒时照样取出里面的表，因为每个操作都在一步之内让表保持一致，中毒只说明别的线程在锁外崩了。
- **上界 `KEPT_BYTES_PER_RUN = 64 KiB`**，等于 runtime 一秒钟最多读出的字节（`READ_BYTES_PER_MS = 64`），也就够页面按 `LIVE_LINES = 400` 行画满两条流（每行约八十字节）。超过就从最旧的一块丢起，但最新的一块总留着，哪怕它自己超过上界。每个 run 的内存因此有上界，没有 run 在跑命令时表是空的。
- **喂与清在记账线程上同步发生**：`attending` 装给 `Serving::outputs` 的闭包先 `keep` 再广播；装给 `worker.observe` 的观察者先 `settle` 再交给视图折叠。块在这次调用的结果落账之前读出，所以同一线程上的次序保证清空之后不会再收到这次调用的块。
- **决定**：缓冲住在装配层而不是 `wire`。清空要认出 `tool_result` 这一行，而记账线程的观察者就在这里；放进 `wire` 要让它为这件事再订阅一次事件流。被否的另一种是让每个会话自己记：那只能记它打开之后的块，正好漏掉这个缓冲要补的那一段。
-/

/-!
## 8-125 生产的命令结果带时钟行，`status` 报这一刻（`accounting::worker::driving`、`accounting::worker::workbench`；`crates/runtime/Spec.lean` §8-10、§8-53）

**接线**：

| 东西 | 谁持有 | 何时定 |
|---|---|---|
| `runtime::ClockReading` | `Site.clock` | `stand_up` 造一个空的；这一跑的 `status` 工具（`StatusTool::clocked`）与 `Sieving` 各拿一份克隆 |
| `runtime::StampGate` | `Sieving.stamps` | `Sieving::for_run` 按 `Site.config` 的 `clock_stamp` 与 `clock_zones` 造，一跑一个 |
| 写读数 | `drive_run` 交给 `RunHooks` 的 `now` | 每读一次墙钟，先 `keep` 进 `Sieving.clock` 再交给驱动 |

`Sieving::package` 打包一份 `exec` 结果之前，拿 `ClockReading::latest` 与这条调用的工具声明的 `Temporal` 问 `StampGate::observe`，得到的戳交给 `runtime::package_exec`，戳成为结果 `content` 的附件行。驱动在工具面打包之前读这条调用的答复时刻（`crates/runtime/Spec.lean` §8-15），所以一条命令的戳是它答复的那一秒，与账本里它那一行 `tool_result` 的 `t` 同一个读数（`crates/runtime/Spec.lean` §3 第 5 条）。

- **只有 `exec` 的结果挂戳**：`status` 自己有 `now:` 一栏，连接器与浏览器的结果不经 `package_exec`，它们的时钟行要一种挂在 JSON 结果上的形状，本节未定；它们不经 `StampGate`，所以「一跑的第一条结果带一次戳」在生产里是「第一条命令结果」。
- **不给工具面一个钟**：`Placing` 与 `Sieving` 手里只有读数。换成工具面自己读 `hands.clock` 会让一跑有第二个采样点，这是 runtime D8 否掉的那一条。

**验收**：`driving/tests/sieving` 的 `a_served_command_result_ends_with_the_second_its_call_answered`：一次真实派活，城里没写 `[clock]`，`exec` 的结果在账本上以 `clock: <ISO>;` 结尾，秒数等于这条调用 `tool_result` 的 `t`。戳等于答复时刻由 runtime 的 `a_stamp_the_face_reads_is_the_moment_its_answer_records` 钉住。
-/

/-!
## 8-98 核心健康：记账队列与持久水位线跨线程可读（`accounting::worker::health`，形状：数据）

`Sample` 里核心健康的两项由记账线程与 lane 在各自的线程上改，由采样线程（§8-96）读：`ledger_queue_depth` 是 lane 经 relay 交给记账线程、还在队列里没被取走的 append 条数；`durable_lag` 是记账线程已取进一批、这批的磁盘屏障还没返回、所以还没有答复的条数。两项都以记录条数计。它们的属主是 `accounting::worker::relay`：只有它知道一条 append 何时进队、何时被取、何时变得持久；本模块只给它一处能跨线程读的地方。

**接口。**

- `Health`：`Clone`，同一只 `Arc` 里的两个 `AtomicU64`；克隆是同一份计数的另一个句柄。
- 写（只由 `accounting::worker::relay` 调）：`asked(&self)`——一条 append 进队之前；`withdrawn(&self)`——进队失败（记账线程已不在），收回那一次 `asked`；`taken(&self, n: u64)`——记账线程取出 `n` 条放进一批，队列减 `n`、未持久加 `n`；`answered(&self, n: u64)`——这一批的屏障返回、答复发出，未持久减 `n`。
- 读：`read(&self, into: Sample) -> Sample`——把此刻的两项填进 `into`，其余字段原样。
- `RelayGate::open(monotonic)` 开一份新的 `Health`，`RelayGate::health()` 交出它的句柄；worker 起好时把 `Flight` 里那只 gate 的句柄经 `Started.health` 交给 `listening`，`listening` 交给采样线程：`spawn_sampler(monitor, samples, volume, health)`，每一拍的读数都经 `Health::read`，摘要与整页都有，因为读它只是两次原子读。
- 没有失败路径。减法饱和：计数是给人看的读数，两条线程各自的加减在某一刻读到的先后可以错开，饱和让一个瞬间的错位读成 0 而不是一个巨大的数。
- 两项等待（roadmap M2），都按整数微秒、都读 worker 注入的单调钟（`Hands.monotonic`，经 `Flight::open` 交给 `RelayGate::open(monotonic)`，再经 `issue` 交给每个 `Relay`）：`relay_queue_us(&self) -> Vec<u64>`——每个 relay 请求从 lane 把它放进队列（`Relay::append_all` 发送之前读一次钟）到记账线程把它取进一批（`RelayGate::serve` 取到它时读一次钟）之间的等待，只留最近的 `RELAY_QUEUE_KEPT` 个，先进先出；`idle_us(&self) -> u64`——记账线程开城以来睡在这一个队列上的累计时长，即 `serve` 里等第一个 wake 的那一段。记账线程的忙占比由读者算：两次读 `idle_us` 之差除以同一段墙钟之外的部分。两项都只由 `accounting::worker::relay` 在记账线程上写，样本表的锁因此只在读者读时有第二个人碰。

**决定。**

1. 计数放在 relay 进队、取出、答复这三处，而不是去数 `mpsc` 队列的长度：标准库的 `mpsc` 不报长度，而且队列里还有 claim、回家的 run 与命令，它们不是 append。被否：记账线程每一轮开头把自己看到的队列长度写进一个原子数——它只在记账线程醒着时更新，而队列最长的时候正是记账线程在写盘、没醒的时候。
2. 计数的类型放在 `bin::monitor`，由 assembly 持有并写入：依赖朝 assembly → monitor，与 `monitor::memory` 相同；monitor 不点名 assembly。
3. 用 `Relaxed` 次序：两项互不约束，读数不参与任何决定（与 §8-96 决定 3 同一个重开条件）。
4. 等待留原始样本而不是直方图：吞吐台要的是 n 与精确的 p50/p99/p999（M0 的单位），而只留最近 `RELAY_QUEUE_KEPT` 个让内存有界；样本由记账线程写、由读者复制一份再排序，排序的钱付在读者那边。被否：对数分桶的直方图——p999 落在一个桶里只能报桶的边，M0 要的是读数本身。重开参数：要把这项等待送上线（`Sample.relay_p50_nanos`）时，再议在写的一侧维护分位数。

**测试。** `accounting::worker::relay::tests` 的 `the_accounting_queue_counts_what_waits_and_what_is_not_yet_durable`：一条 lane 的 append 进队后，记账线程服务之前读到队列 1、未持久 0；服务之后两项都回到 0，append 拿到了答复。同一处的 `a_relay_request_waits_on_the_gates_clock`：钟是一个每读一次走一微秒的计数钟，一条 append 进队后再服务，`relay_queue_us` 恰好是一个样本，值等于两次读钟之间的步数；读数来自注入的钟而不是墙钟，所以每个平台的 CI 读到同一个数。吞吐台（`driving::tests::throughput` 的 `instrument_throughput`）把 `relay_queue` 与 `ledger_thread_busy` 两行和其余等待一起打出。

**本节接口的当前状态。** `Sample` 余下的核心健康字段仍读作 0：`relay_p50_nanos` 与 `event_to_screen_p50_nanos` 要一个按次记录往返时间的直方图，属主分别是 relay 与 socket 的发送一侧，都还没有；`queued_runs` 没有一处权威的计数——计划行在 `Flight::full` 为真时停在 `pursue` 的循环外，没有被数进任何队列，人派的 run 在满时的去向见 §8-46-3。S5.9M 的降级状态没有 `Sample` 字段。
-/

/-! D7 城的工具读别楼的文件与 `cas:` 块，只经 runtime 的一扇门（§8-131、§8-142，`crates/runtime/Spec.lean` §8-59、runtime D16）

`ocr` 与 `transcribe` 的字节都由 `runtime::BoundReader` 读：它把 `read` 的那一份判定（换成城里的拼写、文法、reserved subtree、读界、解开链接再判、打开后核对）公开成一个调用，交回一个 `Read` 与字节的来处。`transcribe` 原先只收本楼的文件，用 `Address::is_within`、`is_reserved` 与 `WriteTarget::within` 自己判，那是这份判定在 accounting 里的替身，门落地后删去。**被否掉的**：两件工具各自在 accounting 里判（两份判定，一份跟着 `read` 变，一份不跟）；只为 `cas:` 另开一条路、文件仍只收本楼（连接器存下的截图与录音能读，读界之内别楼的文件却不能，同一个读界有两个答案）。
-/

/-! D8 一条命令的戳从驱动最近的读数渲染，不给工具面一个钟（§8-125，runtime D8）

`drive_run` 把它交给驱动的 `now` 包一层，每个读数先记进这一跑的 `ClockReading`；`Sieving` 打包时读最新的那个。回合在调用工具面的 `account` 之前读答复时刻，所以最新的那个就是这条命令的答复时刻，戳上的秒数等于它 `tool_result` 的 `t`；一跑仍只有一个采样点，计数时钟下的剧本字节不变。**被否掉的**：`Sieving` 打包时自己读 `hands.clock`——一跑多了一个采样点，戳与它 `tool_result` 的 `t` 可以差一秒；把戳挪进回合的 `account`——那是 runtime 的 `turn` 的事，不是装配层的，而读数的位置已经给出同一个秒数。
-/

/-! D13 崩溃验收在盘上造死亡，不在进程里杀（§8-127）

一次 run 真跑完，丢掉 worker 释放写者锁，再把账截在一行的半途、删去其后的行；重开走 `RunWorker::new` 与 `startup_scan`，与 `sprawling resume` 同一条路。理由：被杀的进程留在盘上的就是这样一份账——锁已释放，最后一行写了一半——而盘上的截法可以精确指定死在哪一行的哪一个字节，每次重跑都是同一次死亡。被否：①起真二进制再杀掉它——那是从外面进城的检查，按边界规则归 `tools/adversary/` 的 Lean 黑盒（G1e），而且杀在哪一刻取决于调度，失败不能逐字节重演；②把账本放在 `storage` 的故障文件系统上断电——它只承载账本，`Standing::fold` 与视图读真目录，重开的不是一座完整的城，断电的耐久契约已由 storage 自己的测试证过。重开参数：账本之外的文件（检查点、快照、CAS）也要在一次死亡里与账本错开时，验收要能在同一刻截断它们，那时改为在 `Vfs` 缝之上承载整座城。
-/

/-! D14 死掉的 run 在重开时冻结，验收比它的整行状态（§8-127）

启动扫描补完悬空调用之后为死掉的 run 写冻结，验收第 4 项比城景里那次 run 的冻结、结局、最后一行与序号，并读账上那一行的 `cause`。理由：ARCHITECTURE §13.7 画的 `Lost --> Frozen` 是账本上的一行，服务中的城与重开的城读同一份账本，必须对这次 run 说同一个结局；只比最后一行的种类，看不出视图把它当成仍在跑。被否：验收只比 `last_kind` 与 `last_seq`、冻结留给以后——那是在把「死掉的 run 永远未冻结」钉成规则；在验收里自己补写冻结——验收就成了它要判的那段代码的第二份。重开参数：`serve` 不经 `resume` 也要收拾死亡时（accounting D30），验收改为经 `serve` 的起步路径重开。
-/

/-! D19 居民判一次，harness 走第二个驱动函数

`agree_to_work` 得出 `Seat` 枚举，模型一臂带着原来的 `Agreed`，harness 一臂带着楼、规则与那一家；`Staged::fly` 按它分到 `drive_run` 或 `drive_harness`，`land` 按它分到原来的落地或 harness 的落地。理由：`Driving` 的十二个字段里，适配器、工作台、sieve、计划与 bench 只有模型用得到，`Driven` 带回的适配器、工作台与冻结的 `Run<Frozen>` 也是；把居民枚举放进 `Driving`，`drive_run` 开头就要 match 一次，结果还是两条路，而每个只有模型才有的字段都要变成 `Option`。**被否**：`Driving` 里的居民枚举（同一个判断在 `Driving`、`Driven`、`Site`、`land` 四处各 match 一次）；在 bench 上立一件「harness」工具（§8-4e 第 8 条已否）。**重开参数**：harness 的 run 开始有第二个回合、或开始用城的工具（MCP 转交）时，两条路共享的部分会变大，那时重议。
-/

/-! D20 试验借一棵自己的树，而不是在合并时才拦

落地策略为 `experiment` 的 run 不论楼要不要评审都在房间的工作树里写（§8-133），合并时 `runtime::admits` 再拒一次。理由：没有评审的楼里一次 run 直接写进楼的文件，到合并那一步时已经没有东西可拦——试验的「什么都不落地」只有在写的位置上才守得住；把它放在树上，楼的文件从头到尾不变，而试验的产出仍在分支上，读得到、比得出。**被否**：只在合并时拒（没有评审的楼里试验照样改了楼）；试验 run 结束时把它改过的文件还原（还原之前别的 run 与人已经读到了改动，而且要为每一种写路径各写一种撤销）。**重开参数**：工作树放置的读数（§8-31 的 `[large_worktree_placement]`）大到让一次小试验的开销以秒计时，重议「小试验用副本、不借树」。
-/

/-! D26 写文档的两条命令在远程门外（§8-152，`crates/wire/Spec.lean` §19-2）

`PutRange` 与 `DecideProposals` 的 `class` 是 `LocalOnly`：一台远程设备即使带着 `Act` 权限也不能经它们写城里的文件。`PutSpine` 是 `Act`，因为它只写一栋楼自己的四份 spine 文档，那是人离开电脑时仍要改的计划；`PutRange` 能写城里保留子树之外的任何一份文件，包括另一栋楼的源码与人的笔记，它比 `Act` 那一组宽。新命令一律 `LocalOnly` 是 §19-3 的规矩，这里写下的是没有例外的理由。**被否掉的**：随 `PutSpine` 定为 `Act`——一台被借走的手机就能改写城里任意一份文件，而撤销配对之前写下的东西只能从检查点里找回。条件变了就重议：人决定在外面也要改文档时，先给 `PutRange` 一个只写某几栋楼的范围，再谈 `Act`。
-/
