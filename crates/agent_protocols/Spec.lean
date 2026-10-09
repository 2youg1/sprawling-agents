-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.agent_protocols.spec.Acp
import crates.agent_protocols.spec.Harness.Consent
import crates.agent_protocols.spec.Harness.Session
import crates.agent_protocols.spec.Mcp.Link
import crates.agent_protocols.spec.Mcp.Reading
import crates.agent_protocols.spec.Mcp.Tools

/-! # agent_protocols 的规格

`agent_protocols` 是这座城与外面说话的两种协议：MCP 出站、ACP 入站，以及把 ACP 反过来用的 harness 出站。本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `agent_protocols D<n>`。§8 的各条保留 `8-1`、`8-15` 这样的编号，别处引作 `crates/agent_protocols/Spec.lean` §8-15。
-/

/-! ## 1 需求分解

三件事，各自可独立验收：

| 模块 | 回答的问题 |
|---|---|
| `mcp`（`handshake`、`tools`、`outbound`） | 一个 Resident 调用外部服务的工具，调用落 Ledger 且可离线重演（§8-1、§8-1c、`spec/Mcp/Tools.lean`） |
| `mcp::reading` | 一条外部消息能有多大，读端怎样在拒绝之后停下（§8-15、`spec/Mcp/Reading.lean`） |
| `mcp::stdio`、`mcp::http`、`mcp::sse`、`mcp::link`、`mcp::redeeming` | 字节经哪种传输到一台 server，丢了答时能不能再问（§8-17、`spec/Mcp/Link.lean`） |
| `mcp::broker` | 哪家 broker 替人持外部应用的 OAuth（§8-18） |
| `acp` | 一个外部编辑器把这座城当 agent 驱动，请求变成一次普通 Dispatch（§8-2、`spec/Acp.lean`） |
| `harness`（`entry`、`catalog`、`roster`、`paste`、`environment`、`process`、`session`） | 城作为 ACP 的 client 驱动任何一个人同意过的 ACP agent，五家官方 harness 是内置条目（§8-19、`spec/Harness/Session.lean`、`spec/Harness/Consent.lean`） |
-/

/-! ## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| mcp | 请求恒是单行且不含换行；两台 server 的同名工具恒是两个工具；浮点入参拒该次调用并报出位置；confidential 楼恒不构造该工具；录制的调用重放得同一答案 |
| mcp::reading | 恰在上限内的消息照常读，超限的整条拒且拒词报出上限与是哪台 server；拒绝之后不再从同一个 source 读；HTTP 答复的 body 受同一个上限 |
| 传输 | 请求交出之后丢了答，效果未知、不自动再发；交出之前的失败不可重试；带会话 id 的 404 返回可重试拒词但不自动重发，下一次派活完整重握手与 listing，克隆共享失效；无 id 的 404 不重开 |
| acp | 已配对请求变成 Dispatch 三字段；`Incoming::parse` 是入站文法的唯一入口（字段私有，本 crate 之外无第二种造法）；配对令牌恒不进入 `Incoming`；持有效令牌也够不到 reserved prefix；回给编辑器的只有 progress 三字段 |
| harness | 没有同意就起不了进程，同意过的摘要与重算的不等即拒；停摆先变成一次 `session/cancel`，第二次什么也不发；停止原因之后不再读；读到输入结束而没有答，效果未知；`-32000` 读成 `E_AUTH_REQUIRED` 并列出登录方法；随版本附带的 registry 索引读成目录 |

分部里的定理是模型对这些性质的证明，每个分部各有一条「拿掉守卫即反例」的定理（§16）。Rust 实现对模型的一致性由测试检查，不由证明：每个模块旁的 `#[cfg(test)]` 经生产入口断言它在本文件 §8 或分部里的规则（§16 列出）。
-/

/-! ## 3 假设与歧义

- **假设**：用户自己拥有外部服务的账号。本库不内置任何一家的 key、不代付、不做代理。
- **歧义已定**：`2026-07-28` 修订版删除了协议级 session（本客户端协商的是 `2025-06-18`，那一版的会话住传输层，见 §8-3），`tools/list` 恒不因连接而异。因此工具表随 Run 冻结与它的规则同向，本库不实现任何会话恢复；需要跨调用状态的 server 自铸句柄，当普通入参传。
- 模型的边界写在各分部的文件头：`spec/Mcp/Reading.lean` 逐字节读而 Rust 按块读（判决相同，内存多一块），不是 UTF-8 与读不出不在模型里；`spec/Mcp/Tools.lean` 不表示 JSON 怎样摊平成叶子；地址文法与 reserved 判定写成 `spec/Acp.lean` 的参数，权威在 `kernel::address`。

**现状：登录与检测的一部分还没有执行器。** 本版已建的是开放名单、同意、目录快照、粘贴文法、环境白名单、`initialize`／`-32000` 的读法，以及 `AddAgent` 的执行（`crates/sprawling/spec/Serving.lean` 的同意一条）；以下几件的接口已在 `wire` 与 IF-0 定下，代码还没有，`AgentLogin` 今天答 `not_built`，ACP 页也不画登录键：
- 登录执行器：收到 `-32000` 之后，`agent` 类型发 `authenticate`，`terminal` 类型在 CLI 里交出终端或经 `sprawling acp-login <ticket>` 跳板开新窗口（Windows `CREATE_NEW_CONSOLE`，macOS `.command` 加 `open`，Linux 终端列表），成功后重新 `initialize` 并重发原请求；
- 检测的另两种线索：npm 与 bun 的全局 shim 反查包目录、PATH 上的名字（只作线索）；
- CLI 的 `/acp` 列表；
- url elicitation 的声明（D14）。
能判定它们完成的证据是：一次派活收到 `-32000` 之后不经人重打就重发成功。`run_started` 已经带着 agent 的 id、它在 `initialize` 里报的版本与同意的摘要（会话在 `HarnessRun::open` 之前打开，所以版本在落账时已知）。
-/

/-! ## 4 现状分析

三组模块：`mcp`（出站：握手、工具表、三种传输、消息上限、broker）、`acp`（入站：外部编辑器的请求文法）与 `harness`（ACP 出站：任何一个人同意过的 ACP agent）。生产消费者是 `crates/sprawling` 与 `crates/accounting`：`accounting::worker::mcp` 按楼的配置连 server 并把连接留给后来的派活（常驻连接，§8-16），`accounting::worker::workbench::servers` 把工具注册进 bench，`views::mcp_health` 用同一个 `McpLink` 探一台 server 的健康，`accounting::worker::driving::harness` 起一个同意过的 agent 并开会话，`accounting::views` 回答 agent 目录与粘贴（§8-19）。

stdio、HTTP、SSE 三种传输与 harness 会话读外部输入都受 `MESSAGE_CEILING` 约束：stdio 与 harness 经同一个 `Lines`，SSE 的每一行经 `read_one_message`，HTTP 的整段 body 经 `read_whole_message`（§8-15）。一次连接的开销由 `mcp::handshake` 的计数测试钉住（§8-16）。
-/

/-! ## 5 权威信源

| 事实 | 出处 |
|---|---|
| 规范总纲与当前修订 | <https://modelcontextprotocol.io/specification/2026-07-28> |
| `tools/list` 恒不因连接而异 | <https://modelcontextprotocol.io/specification/2026-07-28/server/tools> |
| stdio 传输：子进程、按行、消息内无换行 | <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio> |
| 删除协议级 session、新增 `server/discover` | <https://modelcontextprotocol.io/specification/2026-07-28/changelog> |
| 会话住传输层、`Mcp-Session-Id`、404 重开、`CallToolResult` 与 `isError` | 2025-06-18 修订版的 Transports、Lifecycle 与 Tools 三篇（§8-3、§8-1c） |
| ACP 第 1 版的线：请求、`session/update` 的变体、许可的四种选项、停止原因（§8-19） | `agentclientprotocol/agent-client-protocol` 的 `schema/v1/` 目录里稳定的那份 `schema.json`（不是旁边的 unstable 那份），读到的提交记在 docs/third-party.md §1 |
| ACP 里工具由 agent 自己执行，许可请求 agent 可以不发 | <https://agentclientprotocol.com/protocol/tool-calls> |
| 一个 registry 条目怎么起、随版本附带的目录快照 | ACP registry 的索引（<https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json>），条目格式是 `agentclientprotocol/registry` 的 `FORMAT.md` 与 `agent.schema.json`，被看路径见 docs/third-party.md §1 |
| 认证方法只有 `agent` 与 `terminal` 两种，`-32000` 是「要登录」，`auth.terminal` 能力 | 同一份稳定 `schema.json` 的 `AuthMethod`、`ErrorCode`、`AuthCapabilities`；<https://agentclientprotocol.com/protocol/v1/authentication> |
-/

/-! ## 6 命名统一

`Connector` 是词汇表里这层的统称；代码里出现的是它的两个具体面 `McpTool` 与 `Incoming`。恒不把 MCP server 叫作 endpoint：`Endpoint` 在本库专指 external provider 网关。

`ServerLabel` 不住本 crate：它住 `kernel::tool`（理由与文法见 `crates/kernel/Spec.lean` §8-23）。本 crate 继续用它，但不再拥有它：配置层要在文件边界解析标签，而 `city` 只见 `kernel`。

模型的命名空间按 Rust 模块路径取：`AgentProtocols.Mcp.Reading`、`AgentProtocols.Mcp.Tools`、`AgentProtocols.Mcp.Link`、`AgentProtocols.Acp`。`spec/Harness/Session.lean` 的命名空间是 `HarnessRun`，因为它规定的是跨三个 crate 的那一次 harness run，本 crate 只守其中的会话半（§8-19）。
-/

/-! ## 7 模块边界

- **字节怎么走**归本 crate 的三种传输（`mcp::stdio`、`mcp::http`、`mcp::sse`，见 §8-17）：子进程的拉起、期限与回收，HTTP 会话与事件流都住这里。装配层只决定一栋楼按配置连哪几台 server。
- **哪家 broker 替人持外部应用的 OAuth**也归本 crate（`mcp::broker`，见 §8-18）：出网政策仍只在 `gateway::client_for`。
- **一条消息能有多大**归本 crate（`mcp::reading`）：上限是 MCP 这个协议的事实，不是某一种传输的事实；两个传输各写一个数字就是两条会漂的上限。按行读一条连接的线程与无队列的交接也住那里（`Lines`，D7）；SSE 自留循环，每一行仍交给 `read_one_message` 并接受它的拒绝；HTTP 的整段 body 经 `read_whole_message`（D8）。
- **准不准出网**归 `kernel::gate` 的 egress 门：外部工具声明 `Effect::Connector`，路由到那道门；本 crate 只在 confidential 一位上做构造点拒（更早、更硬，D5）。
- **回来的东西算什么**归 `kernel::taint`：与 L0 工具同落 `kernel::tool` 缝，故自动进污染环，本 crate 无解包面。
- **配对判定不住本 crate**：令牌住 `wire`，判定住入站中间件，本 crate 因此既看不见密钥也不持有它的副本；`admit` 只判 reserved prefix 这一条本 crate 独有的规则。
-/

/-! ## 8 接口先行

签名的权威是 Rust 源码；这里记每个模块的接口、它为什么是这个形状，以及分部没有表示的要求。

### 8-1 mcp（形状 3 端口＋形状 4 适配器＋形状 1 判定）

```rust
// call 携期限。声明即承诺可协作取消（`crates/kernel/Spec.lean` §8-23 的 TimeoutMs），
// 而一个不回答的 server 是把整个 Run 挂死的最短路径。
// `notify`：一条通知没有答案。HTTP 上它被 202 加空体应答，
// 把它当请求读的客户端会因为对侧「什么都没说」而拒掉一台正确的 server。
pub trait Outbound: Send {
    fn call(&mut self, line: &str, patience: TimeoutMs) -> Result<String, AxError>;
    fn notify(&mut self, line: &str, patience: TimeoutMs) -> Result<(), AxError>;
}
pub const EXTERNAL_CALL_PATIENCE: TimeoutMs = TimeoutMs(60_000);
pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub struct Handshake { pub protocol_version: String, pub server: String }
pub fn handshake(out: &mut dyn Outbound, rpc: &mut Rpc, patience: TimeoutMs)
    -> Result<Handshake, AxError>;
pub fn digits_for_floats(value: Value) -> Value;
pub struct Rpc { /* next: u64 私有 */ }
impl Rpc {
    pub fn new() -> Rpc;
    pub fn initialize(&mut self) -> String;
    pub fn initialized() -> String;              // 通知，无 id
    pub fn list_tools(&mut self) -> String;
    pub fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<String, AxError>;
    pub fn read(line: &str) -> Result<Value, AxError>;
}
pub struct Listed { pub remote: String, pub meta: ToolMeta } // 一件工具的两个名字，一起走
pub fn tools_from(server: &ServerLabel, result: &Value) -> Result<Vec<Listed>, AxError>;
pub struct McpTool { /* 私有：meta、remote、patience、Mutex<Link>（Rpc 与 Outbound） */ }
impl McpTool {
    pub fn new(meta: ToolMeta, remote: String, outbound: Box<dyn Outbound>, confidential: bool)
        -> Result<McpTool, AxError>;
    pub fn remote(&self) -> &str;
}
pub(crate) const EFFECT_META_KEY: &str = "sprawling/effect-unknown";
pub(crate) const ERROR_TEXT_CAP_BYTES: usize = 4_096;
pub struct ScriptedOutbound { /* 私有 */ }                                          // 第二适配器
```

- `ServerLabel` 住 `kernel::tool`（§6）；本 crate 不再转导它，一个类型两条导入路径就是一个类型两个住址。
- 无期限的 meta 在 `McpTool::new` 即拒：一件没有期限的出站工具就是一件可以挂死 Run 的工具（`spec/Mcp/Tools.lean` 的 `construct`）。
- `Tool::invoke` 取 `&self`：`Rpc` 的 id 与它编号的连接是同一把锁，答案从请求出去的那条连接上读回，所以同一台 server 的两次调用轮流走。

### 8-1b 外部输入的消息上限（形状 1 判定；见 §8-15）

```rust
pub const MESSAGE_CEILING: usize = 8_388_608;
pub enum Received { Message(String), EndOfInput }
pub fn read_one_message(source: &mut dyn BufRead, server: &str) -> Result<Received, AxError>;
pub(crate) fn read_whole_message(source: &mut dyn Read, server: &str) -> Result<String, AxError>; // HTTP 的一整段 body
// 一条线程读一条按行分帧的连接，交接无队列（D6 (d)）；stdio 与 harness 共用（D7）
pub struct Lines { /* 私有：Receiver<Result<Received, AxError>> */ }
impl Lines {
    pub fn over<R: BufRead + Send + 'static>(reader: R, name: &str) -> Result<Lines, AxError>;
    pub(crate) fn next(&self, wait: Duration) -> Result<Heard, AxError>; // Heard::{Message, Ended, Silent}
}
```

### 8-1c 一次 `tools/call` 的答复

MCP 2025-06-18 把工具的答复定为 `CallToolResult`：`content` 是内容块数组；`isError` 为真时，这是工具自己报的错。规格要求工具的错误放进结果、置 `isError`，不回协议层的 JSON-RPC error。`McpTool::invoke` 按这条读，判决在 `spec/Mcp/Tools.lean` 的 `read`（D1）：

- `isError` 不为真：`result` 经 `digits_for_floats` 成为 `ToolOutcome.result`。窗口怎么装它，归 `runtime::pipeline::connector`（`crates/runtime/Spec.lean` §8-27-10）。
- `isError` 为真：这是一次失败，`invoke` 回 `Err`。subject 是 `<remote> reported a failure: <文字>`，文字是全部 `type: "text"` 块按原顺序以换行连起；超过 `ERROR_TEXT_CAP_BYTES` 时在字符边界截断，并写明截掉了多少字节；非文字块不进 subject，只报个数。码是 `E_TOOL_UNAVAILABLE`，`Retry::No`。
- `isError` 为真且 `_meta` 里有 `sprawling/effect-unknown`、值不是 `false`：码是 `E_TOOL_OUTCOME_UNKNOWN`，标 `effect_unknown`（`Retry::Unknown`）。这个键说的是「这次调用交出去了一部分，桌面或别处是否已经生效不知道」。值写错也按「不知道」读。
- 协议层的 JSON-RPC error 照旧由 `Rpc::read` 读成 `E_TOOL_UNAVAILABLE`，只取 `code` 与 `message`，不读 `data`：`data` 的形状各家自定，城只认规格定过的东西。

键名 `sprawling/effect-unknown` 合 `_meta` 的键名格式：前缀是一个以字母开头的标签加斜杠，不落在 `mcp`／`modelcontextprotocol` 的保留前缀里。它的唯一定义是 `mcp::tools::EFFECT_META_KEY`，经 `agent_protocols::EFFECT_META_KEY` 对外给出；桌面 server 引用它，不写第二份。

### 8-2 acp（形状 1 判定＋形状 2 值类型）

```rust
pub struct Incoming { /* 私有：addr／task／goal；唯一构造者是 parse */ }
impl Incoming { pub fn parse(body: &Value) -> Result<Incoming, AxError>; }
pub enum Admitted { Dispatch { addr: Address, task: String, goal: String } }
pub fn admit(request: Incoming) -> Result<Admitted, AxError>;
pub struct Progress { pub run: String, pub turns: u32, pub finished: bool }
```

性质在 `spec/Acp.lean`：`parse` 造出的值三个字段都在，`admit` 恒不放过 reserved prefix，准入不增不减。

### 8-3 生命周期与会话

依据是 2025-06-18 修订版 Transports 与 Lifecycle 两篇的规范句：

- **开场必须是 `initialize`**，携 `protocolVersion`、`capabilities`、`clientInfo` 三项；随后必须发 `notifications/initialized`，之后才能问别的。故 `handshake()` 是这条生命周期的唯一权威，坐在三个传输之上：各传输各写一遍就是几份会漂的生命周期。
- **`capabilities` 故意为空**：roots／sampling／elicitation 是 server 反过来向我们要的能力；声明一项本城没实现的能力，等于招来一个随后只能拒的请求。
- **已知的向前变化**：更新的修订正在把会话去掉（SEP-2575）。本客户端协商的是 `2025-06-18` 并按那一版行事；一台忽略该头的 server 不会因此变得不可用。
- **HTTP 传输带自己的 User-Agent**：CDN 后面的托管 server 可能对不报名的客户端回 403 `browser_signature_banned`，早于任何 MCP 消息。

D2 会话住传输层，不住本 crate 的协议层，因为规范把它写在 Transports 而不是 Lifecycle：server 可选在 `initialize` 应答的头里发 `Mcp-Session-Id`；一旦发了，客户端 MUST 在此后每一次请求带回。404 意味着 server 结束了会话，MUST 重开一个，故 `mcp::http` 遇带 id 的 404 清除会话与版本并记录失效，`McpLink::has_ended` 把这个共享状态交给唯一 Residents 持有者，由下一次派遣新开链接、完整握手与 listing，而不是拿一个已死的 id 永远碰下去。带着会话 id 的 404 标 `retriable`：按规范 server 对已结束会话的请求一律答 404，调用没有被执行，下一次派遣先重开会话；不带会话 id 的 404 标不可重试，它说的是地址不对，再问只得到同一个答（`spec/Mcp/Link.lean` 的 `sessionEnded`）。被否：把会话放进 `handshake` 与 `Rpc`，那样 stdio 与 SSE 要为一个它们没有的头背一份状态。

### 8-4 入向浮点：治我们发的，适应我们收的

D3 `digits_for_floats` 把对侧答案里的小数原样写成字符串。搜索类 server 的答案常带相关度分之类的小数；Ledger 不收浮点，照搬就会让每一次调用都以 `E_INVALID_ARGS` 失败。三条理由：① 禁浮点是 Ledger 的规矩（确定性第 6 条），不得放松；② 拒掉整个答案（被否）等于声明本城接不了任何真实的搜索 server；③ 丢掉该字段（被否）是隐形地删别人的数据。写成字符串不丢一位数字、不做任何算术，且在 Ledger 里看得见（带引号的数）。与 `call_tool` 拒掉携浮点的入参（D4）并不矛盾：本城治自己发出去的，适应自己收回来的。

### 8-15 外部输入通道的消息上限

stdio 与 SSE 都按换行分帧，一台不写换行的 server 会让读端在遇到换行前无限增长一块缓冲；读端与调用方之间的队列若无界，读端又会在调用方慢的时候把读到的消息全攒在内存里。MCP server 是 `CONFIG.toml` 指进来的外部方，其输出是不受信任的输入，而读它的进程同时是这座城唯一的写者：内存耗尽等于历史中断，不是一次工具调用失败。性质在 `spec/Mcp/Reading.lean`，四条决定是 D6，写在 `readFrom` 的上方。

**两条按行读的传输怎么用它**：reader 读到 `Received::EndOfInput` 就停，调用方看见的是连接断开；读到拒词（超限、不是 UTF-8、流在消息中途断掉、读不出）就把它交给正在等的那次调用，然后停读（D6 (c)）。这次调用拿到的拒词带 `Retry::Unknown`（`mcp::reading::answer_unread`）：请求已经交出，server 也答了，只是这份答案本城不读，它做没做事与 §8-17 丢了答的情形一样不可知。stdio 随即回收子进程，`has_ended` 为真，持有者下一次派活重开；SSE 的 reader 停读即放下这条流，之后的调用读到流已断。SSE 的一行是一个 `data:` 帧、一个事件名或一行注释，`read_one_message` 按换行分帧，所以每一行受同一个上限。

D7 一个读端，两条连接：stdio 与 harness 的读端是同一个 `mcp::reading::Lines`：一条线程循环调 `read_one_message`，经 `sync_channel(0)` 交出，读到 `EndOfInput` 或拒词即停；调用方用 `Lines::next(wait)` 带期限地等，得到 `Heard::{Message, Ended, Silent}` 或那条拒词。期限各归调用方：stdio 等一次调用的 patience，harness 每 `HALT_TICK_MS` 回头问一次停摆。被否：两份逐字相同的循环，改其中一份（比如 D6 (c) 的停读）另一份不会跟上。SSE 不用它：它的线程先开流、把打不开的拒词当作第一条也是唯一一条交出，读到的行只留 `data:` 那些，每一行照样经 `read_one_message`。

### 8-16 常驻连接

一次连接的开销是每台 server 两次有应答的请求（`initialize`、`tools/list`）加一条通知（`notifications/initialized`），由 `mcp::handshake` 的 `opening_one_connection_costs_two_round_trips_and_one_notification` 用一个计数 `Outbound` 钉住。断言的是条数而不是时间：条数在每台机器上相同。本 crate 为这三条消息花的 CPU 远小于子进程启动与两次往返，而后两者属于传输。

D9 本 crate 不持连接表：`McpLink` 可 clone，寿命由持有者决定。持有者是装配层的 `accounting::worker::mcp::Residents`，它把一台 server 的连接与工具表留给后来的派活，子进程退出或 HTTP 带会话 id 的 404 使连接失效（`McpLink::has_ended`）时重开并重取工具表。派活在这一步花的时间记在 `tools/xtask/budgets.toml` 的 `[prepare_dispatch]`。HTTP 会话失效不在传输内重发：原调用的失败仍归调用者，下一 dispatch 的恢复与工具表更新归 Residents，模型与派生回归在 `spec/Mcp/Link.lean` 与 `accounting::worker::mcp::tests`。被否：本 crate 自持一张按配置索引的连接表，那样同一台 server 的寿命有两个主人，装配层重开时本 crate 还握着旧的那条。

### 8-17 三种传输与 `McpLink`（形状 4 适配器；实现 `Outbound`）

```rust
// mcp::link：一台可达的 server，不论经哪种传输
#[derive(Clone)]
pub struct McpLink(Reach); // 私有：enum Reach { Stdio, Http, Sse }；克隆即同一条连接的第二个句柄
impl McpLink {
    pub fn open(transport: &kernel::McpTransport, write_root: &Path,
                resolve: &gateway::SecretResolver) -> Result<McpLink, AxError>;
    pub fn site(transport: &kernel::McpTransport) -> &'static str; // 出事时该打开的模块
    pub fn has_ended(&self) -> bool; // 子进程已退出或 HTTP 会话已失效：持有者据此重开
}
impl Outbound for McpLink { /* 逐传输转发 call／notify */ }
// mcp::stdio::StdioServer、mcp::http::HttpServer、mcp::sse::SseServer：pub(crate)
// mcp::redeeming：一栋楼写在 server 旁边的成对表，引用已兑付；三种传输共用
#[cfg(any(test, feature = "conformance"))]
pub fn echoing(answer: &str) -> (String, Vec<String>); // 对每行都回同一个结果的子进程
#[cfg(feature = "conformance")] // 只有装配层的测试用它
pub fn gated(answer: &str, starts: &Path, gate: &Path) -> (String, Vec<String>); // 握手在 gate 文件出现前不作答
#[cfg(feature = "conformance")]
pub fn counting_starts(answer: &str, starts: &Path) -> (String, Vec<String>); // 每次启动在 starts 里记一笔
```

- **`site()` 由传输的拥有者给出**：报错地址是「哪个模块到达了这台 server」，只有拥有这三个模块的 crate 能不漂地说出它。
- **子进程回收**：期限到即杀子进程，理由见 `mcp::stdio` 的模块文档：一个迟到的答案会被读成下一次调用的答案。
- **`echoing` 在 `conformance` 后面**：装配层的测试要起同一个假 server；产品二进制不带它（`xtask artifact`）。
- **请求交出之后丢了答，效果未知，不可重试**：性质在 `spec/Mcp/Link.lean`。请求已经完整交给对侧之后（stdio 是那一行写完并 flush，HTTP 与 SSE 是 POST 得到回应），期限内没有答案（`E_TIMEOUT`）、对侧在作答前关了输出或流断了（`E_TOOL_UNAVAILABLE`）、答复被读端拒（`E_WIRE_MISMATCH`），都标 `effect_unknown`（`Retry::Unknown`，`crates/kernel/spec/Error.lean` 的三态）：server 可能已经做了，再发一次同一调用可能把一次写做两遍，由看得见这次调用的人决定要不要再问。请求还没交出去时的失败（stdio 写管道失败、POST 本身失败）按对侧没收下读，照旧 `Retry::No`。
- 失败码：各传输沿用 §12 的 `E_TIMEOUT`／`E_WIRE_MISMATCH`／`E_TOOL_UNAVAILABLE`，HTTP 与 SSE 在 401／403 抬 `E_CREDENTIAL_MISSING`，客户端构造不成抬 `E_CONFIG_INVALID`。
- **换不换账号、能不能再发，由这里标在错误上**（`AxError::account` 与 `retry`，`crates/kernel/spec/Error.lean` D53）：401 是这个账号的 Key 被拒，标 `Advance`（`ErrorDraft::account_unusable`），下一个账号可能被收下；403 不标，它可能是权限、地域或这台 server 的规则，换一个账号不见得修得好。408、429、502、503、504 是对端这次没有处理这个请求，标可重试（`Retry::Yes`），这五个状态码只在 HTTP 与 SSE 的拒词这一处判；其余非 2xx 照旧 `No`。读者是 `web_search` 的一轮（`crates/accounting/spec/Connectors.lean` §8-35）；其它 MCP 工具不自动重发，这两格只是交给模型的事实。

HTTP 的答复 body 经 `read_whole_message` 受同一个上限，非 2xx 的答复不读 body：D8，写在 `spec/Mcp/Reading.lean` 的 `readWhole` 上方。

D10 传输住协议旁边，不住组合根：三种传输与握手说同一个协议，差别只在字节去哪。放在装配层时，`agent_protocols` 定义了 `Outbound` 缝却看不见它的生产实现；组合根只剩「一栋楼按配置连哪几台 server」（`accounting::worker::mcp`）。被否的另一方案是把传输留在 `sprawling`、只搬 `McpLink`：那样 `McpLink` 的三个分支仍指向另一个 crate 的私有类型，搬不动。(b) HTTP 与 SSE 共用一个客户端构造（`mcp::http::client_for`）：代理规则、user agent 与构造失败的拒词只写一处。两者唯一的差别是整请求时限，作为参数 `WholeRequest` 递进去：HTTP 取 `DefaultTimeout`（一次 post 与它的回答），SSE 取 `Unbounded`，因为 SSE 的 body 就是整段对话，reqwest 的整请求时限会把流掐断。被否：SSE 自留一份构造只为多一行 `timeout(None)`，两份构造一旦有一份改了代理规则，另一份就静默地走另一条出网路径。

### 8-18 外包 OAuth 的 broker（形状 4 适配器；三个调用，别无其它）

```rust
// mcp::broker：唯一知道某台 server 属于 Composio 的模块
pub struct Broker { /* 私有：client、Sealed<String> 项目密钥、base url */ }
impl Broker {
    pub fn new(key: kernel::Sealed<String>, rule: kernel::Proxying) -> Result<Broker, AxError>;
    pub fn shelf(&self, user: &str) -> Result<Vec<Toolkit>, AxError>;   // 目录 × 此人的状态
    pub fn connect(&self, slug: &str, user: &str) -> Result<String, AxError>; // 同意页地址
}
pub struct Toolkit { pub slug: String, pub name: String, pub auth: String, pub standing: Connection }
pub enum Connection { Absent, Awaiting { consent_url: String }, Connected { alias: String }, Refused { refusal: AxError } }
```

- **返回自己的词汇，不返回线上形状**：装配层把 `Toolkit`／`Connection` 映成 `wire::ToolkitLine`，与它把握手映成 `McpState` 同一做法。
- 失败码：401／403 抬 `E_CREDENTIAL_MISSING`；408／429／5xx 抬可重试的 `E_PROVIDER`；连接阶段超时抬可重试的 `E_PROVIDER`（请求还没离开这台电脑）；请求发出之后等答超时抬 `effect_unknown` 的 `E_PROVIDER`，因为 `connect` 会在 broker 那边建一份 auth config，重发可能建出第二份；2xx 之后 body 读不完（连接在答案中途断开）同样抬 `effect_unknown` 的 `E_PROVIDER`，subject 带读不出的原因：broker 已经照做了，丢的只是答案；非 2xx 的 body 读不出时，读不出的原因代替 body 作附近文字；其余状态、读不出的答案与接不上 base 的路径抬 `E_PROVIDER`。接不上的路径在 recovery 里报出本模块的路径（`module_path!()`），模块再搬家也不漂。
- 字段按防御方式读：缺一个字段少一行，不毁整张答案；测试里的假 server 是本 crate 对 broker 所发内容的陈述。

D11 broker 住 `agent_protocols::mcp`，不住 `gateway`：它回答的是「连哪台 MCP server 上的哪个应用」，与三种传输同属一件事；出网的两条政策（代理规则、HTTP 客户端的构造）仍只在 `gateway::client_for` 一处，broker 经它取客户端，测试的 `Broker::at` 也一样（`Proxying::ExceptLocal` 对回环地址不走代理），所以本库之内没有第二个构造点。被否的另一方案是留在 `gateway`：那样 MCP 一分为二，`gateway` 要知道一家 MCP 服务的目录形状。(b) 只有一家 broker，所以没有 trait：第二家外包服务才是这条缝的第二个实现。

### 8-19 ACP agent 出站：开放名单、同意与一场 ACP 会话（`agent_protocols::harness`；`entry`、`roster` 形状 6 数据面，`catalog`、`paste` 形状 2 文法，`environment` 形状 1 判定，`process`、`session` 形状 4 适配器）

订阅额度由厂商自己的 agent 带进城（`crates/gateway/Spec.lean` §8-5）。本节是这条路的传输半：一个 agent 条目是什么、人怎样同意它、怎么把它起成一个说 ACP 的子进程、怎么跟它开一场会话并把它说的话读回来。本城是 ACP 的 client，与 §8-2 的入站方向相反。

```rust
// harness::entry —— 一个 agent 条目（数据面）
pub struct AgentId(/* 私有 String */);
impl AgentId { pub fn parse(text: &str) -> Result<AgentId, AxError>; pub fn as_str(&self) -> &str; }
pub enum AgentSource { Registry, Detected, Pasted }
pub struct AgentEntry {
    pub id: AgentId, pub name: String, pub source: AgentSource, pub launch: Launch,
    pub version: Option<String>, pub licence: Option<String>,
}
pub struct Launch { pub program: String, pub args: Vec<String>, pub env: Vec<(String, String)> }
impl Launch {
    pub fn digest(&self) -> B3Hash;    // 同意绑定的摘要（D16）
    pub fn preview(&self) -> String;   // 同意卡片上那一行命令，原样
    pub fn pin(&self) -> Pin;          // 版本钉没钉住
}
pub enum Pin { Exact, Floating, Unknown }
pub struct Consented { /* 私有：AgentEntry */ }
impl Consented {
    pub fn given(entry: AgentEntry, digest: &B3Hash) -> Result<Consented, AxError>;   // 摘要不等即拒
    pub fn written_by_hand(entry: AgentEntry) -> Consented;                           // 人亲手写进 CONFIG.toml 的那一行
    pub fn entry(&self) -> &AgentEntry;
}

// harness::catalog —— ACP registry 的索引，与随版本附带的快照（文法）
pub struct Catalog { pub entries: Vec<AgentEntry>, pub date: String, pub etag: String }
impl Catalog {
    pub fn bundled() -> Result<Catalog, AxError>;                                  // 随版本附带的快照
    pub fn read(index: &str, date: String, etag: String) -> Result<Catalog, AxError>;  // 一份 registry.json
}
pub fn registry_entry(value: &serde_json::Value) -> Result<Option<AgentEntry>, AxError>;   // 一份 agent.json；本平台起不了的答 None

// harness::roster —— 内置条目：五家官方 harness（数据面）
pub struct Official { pub word: &'static str, pub registry_id: &'static str, pub set_up: &'static [SetUpDir], pub docs: &'static str }
pub const OFFICIAL: [Official; 5];
pub struct Roster { /* 私有：人加的行、快照 */ }
impl Roster {
    pub fn new(rows: Vec<(AgentEntry, Option<B3Hash>)>, catalog: Catalog) -> Roster;
    pub fn builtin(&self, official: &Official) -> Option<AgentEntry>;
    pub fn seat(&self, word: &str) -> Result<Consented, Unseated>;   // [resident] harness 的值
    pub fn words(&self) -> Vec<String>;                              // 拒词的 nearby
}
pub enum Unseated { Unknown, Refused(AxError) }
pub struct SetUpDir { pub variable: Option<&'static str>, pub under_home: &'static [&'static str], pub source: &'static str }
impl SetUpDir { pub fn on(&self, home: Option<&Path>, variable: Option<OsString>) -> Option<PathBuf>; }

// harness::paste —— 粘贴文法（文法）
pub fn pasted(text: &str) -> Result<AgentEntry, AxError>;

// harness::detect —— 不执行任何东西的检测（判定）
pub const CLIENT_CONFIGS: [SetUpDir; 2];                                   // 别的 ACP 客户端写下的配置文件
pub fn configured(text: &str) -> Vec<AgentEntry>;                          // 一份 agent_servers 配置里的每一个 agent，env 不带
pub fn detected(roster: &Roster, set_up: impl Fn(&SetUpDir) -> bool, configs: &[String]) -> Vec<AgentEntry>;

// harness::environment —— 子进程见到的环境（判定）
pub fn passed(city: impl Iterator<Item = (OsString, OsString)>) -> Vec<(OsString, OsString)>;

// harness::process —— 把一个同意过的 agent 起成子进程（适配器）；落地即杀
pub struct HarnessProcess { /* 私有：Child */ }
impl HarnessProcess {
    pub fn start(agent: &Consented, cwd: &Path) -> Result<(HarnessProcess, AcpSession<ChildStdin>), AxError>;
}

// harness::session —— 一场 ACP 会话，JSON-RPC 2.0，按行分帧（适配器）
impl<W: Write> AcpSession<W> {
    pub fn open(lines: Lines, writer: W, name: &str, cwd: &Path) -> Result<Self, AxError>;   // initialize ＋ session/new
    pub fn introduced(&self) -> &Introduced;
    pub fn prompt(&mut self, text: &str, listener: &mut Listener<'_>) -> Result<Answer, AxError>;
}
pub struct Introduced { pub version: Option<String>, pub auth_methods: Vec<AuthMethod> }
pub struct AuthMethod { pub id: String, pub name: String, pub kind: LoginKind }
pub enum LoginKind { Agent, Terminal }
// Listener、Answer、Update、PermissionAsk、PermitOption、PermitKind、Permit、StopReason 不变
```

- **名单是开放的**：任何说 ACP 的 agent 都可以当居民。`[resident] harness` 的值点名城 `CONFIG.toml` 里的一行 `[[agent]]`（`crates/city/spec/ConfigLayers.lean` §8-4），或者一个内置条目。五家官方 harness（Claude Code、Codex、Grok Build、Kimi Code、Pi）是内置条目：`OFFICIAL` 只记它们的词（0.0.10 写下的 `[resident] harness` 原样读得回来）、registry id、装好或登录后写下的目录（D19）和厂商的登录说明；怎么起它们取自快照里同一个 registry id 的那一行（D12）。`Roster::seat` 先找人加的行，再找内置条目：一行与内置条目同名时，人加的那一行胜，它是人为这台电脑写下的那一条。
- **同意先于任何执行**（D16，性质在 `spec/Harness/Consent.lean`）：`HarnessProcess::start` 只收 `Consented`，`Consented` 的字段私有，只有两扇门：`given` 收一份条目和同意时记下的摘要，摘要与 `Launch::digest` 重算的不等就拒（`E_CONFIG_INVALID`）；`written_by_hand` 收一行人亲手写下、没有摘要的条目，以及内置条目，那份文件就是人的同意，与 `[[mcp]]`、`[remote]` 同一条理由。目录、检测与粘贴只产出 `AgentEntry`，产出不了 `Consented`，所以页面上看见的东西在人同意之前起不了任何进程。
- **快照随版本附带，只在人按下时刷新**（D13）：`Catalog::bundled` 读 `crates/agent_protocols/catalog/registry.json`，这份文件由 `cargo xtask acp-catalog` 从 registry 的 CDN 索引生成并提交，带索引的 `Date` 与 `ETag`；生成时只保留城读的字段（`id`、`name`、`version`、`license`、`distribution`，`binary` 目标只留 `cmd`、`args`、`env`），作者、图标、链接与下载地址不进快照。`Catalog::read` 读的是同一种形状，所以一份完整的 CDN 索引与快照经同一个读者。读法：`npx` 起 `npx -y <package> <args…>`，`uvx` 起 `uvx <package> <args…>`，`env` 原样带上（它们会关掉 agent 的自更新）；`binary` 本版不下载（推迟，见 §3），读成本平台那一项 `cmd` 的文件名经搜索路径起，参数照抄，钉不住版本（`Pin::Unknown`）；本平台没有一项能起的条目不进目录。
- **粘贴文法只在 Rust 里**（`paste::pasted`，WebUI 与 CLI 经 `Query::ParseAgentSpec` 共用）：接受一行命令、Zed 或 JetBrains 的 `agent_servers` 块（恰好一项）、registry 的 `agent.json`。一行命令按空白分词，双引号括住的一段是一个参数，不解释任何 shell 元字符；出现 `|`、`&`、`;`、`<`、`>` 中的任何一个就拒，恢复语说「只粘贴一个程序和它的参数」。条目的 id 取 `agent_servers` 的键、`agent.json` 的 `id`，或程序的文件名，经 `AgentId::parse` 规范成小写。
- **检测不执行任何东西**（`detect`）：两种证据，都只读文件。一是别的 ACP 客户端里人亲手写下的配置：JetBrains 的 `~/.jetbrains/acp.json` 与 Zed 的 `~/.config/zed/settings.json`（`CLIENT_CONFIGS`，各引厂商文档），其中 `agent_servers` 的每一项读成一个 `Detected` 条目，`env` 不带：别处的 `env` 可能有密钥，值要人另行写进 Vault；读不成 JSON 的文件（Zed 的文件允许注释）不算证据，不拒整页。二是内置条目的厂商目录（`Official::set_up`，D19）存在：那个内置条目以 `Detected` 列出。检测只决定排序与提示，证明它能用的只有同意之后的 `initialize`。npm 与 bun 的 shim、PATH 上的名字本版不读（§3）。查哪个路径由服务中的城经 `Views` 的 `places` 入口交进来（`crates/accounting/spec/Views.lean`），本模块只判。
- **钉住**：`Pin::Exact` 是包串带一个确切版本（`pkg@1.2.3`、`pkg==1.2.3`）；包串不带版本或带 `latest` 是 `Floating`；不是 `npx`、`uvx` 起的程序是 `Unknown`。同意卡片照实显示这一格，不替人拒一个浮动的版本：粘贴的那一行是人自己写的。
- **子进程只见白名单里的环境**（`environment::passed`，D17）：先清空，再放行白名单里的名字，最后放行条目自己的 `env`。白名单是系统与终端需要的那些（`PATH`、家目录、临时目录、语言、Windows 的系统目录与 `PATHEXT`、XDG 目录、显示、代理），加上 `SSH_CONNECTION`、`SSH_CLIENT`、`SSH_TTY` 与 `NO_BROWSER`：agent 靠它们判断自己是否在远程主机上、该给哪种登录；再加上 `OFFICIAL` 里每家的目录变量，它们的名字只写在 `SetUpDir::variable` 一处。Windows 上名字不分大小写。
- **`initialize`**（D14）：发 `protocolVersion: 1`、`clientInfo {name: "sprawling", version}`、`fs` 两项与 `terminal` 为 `false`、`auth.terminal: true`。对方答的 `protocolVersion` 不是 1 就拒（`E_WIRE_MISMATCH`，恢复语说这家说的是哪一版），规范要求客户端这时断开并告诉人。`agentInfo.version` 与 `authMethods` 读进 `Introduced`；方法的 `type` 缺省是 `agent`，`terminal` 是另一种，其余拒而不猜。
- **`-32000` 是「要登录」**：对方对任何请求答 JSON-RPC `-32000`，拒词是 `E_AUTH_REQUIRED`，`nearby` 列出可用的登录方法 id，恢复语逐个说出方法名。claude.ai 订阅登录（`claude-agent-acp` 声明的方法 `claude-ai-login`）从 `Introduced` 与这份列表里拿掉：本城不替人启动订阅登录（D18）。其余错误码仍是 `E_PROVIDER`。
- **登录推迟到 `-32000`**：添加 agent 时不登录；派活时 `session/new` 或 `prompt` 答 `-32000`，才轮到登录（§3 记着登录执行器的现状）。
- **一条消息的上限与 MCP 同一个**：`read_one_message` 与 `MESSAGE_CEILING`（§8-15）。ACP 与 MCP 同是按行分帧的 JSON-RPC，一个图片加信封的上限对两者是同一个事实。
- **`prompt` 在对方答出 `StopReason` 时返回**；读到输入结束而没有答，是 `E_PROVIDER` 并标 `Retry::Unknown`：对方也许已经做了事。`StopReason` 未知的词拒而不猜。
- **停摆在对侧沉默时也要变成取消**：`BufRead` 的读没有期限，一家在跑长命令的 agent 可以几分钟一行不写，而一次停摆不能等它开口。所以读端在自己的线程上（`Lines::over`），把行交进通道；`prompt` 每次最多等 `HALT_TICK_MS` 就回头问一次 `halted`。线程在对侧关闭输出（子进程被杀）或会话丢掉通道时结束，不会泄漏。ARCHITECTURE §10 规则 3 把它列为库 crate 起线程的一处。
- **取消的次序是 `spec/Harness/Session.lean` 定的**：`halted` 头一次答真，先调 `cancelling`（调用方在这里把 `cancel_received` 落账），再发 `session/cancel`，此后的汇报排在它后面；第二次答真什么也不发。取消之后 agent 再问 permission，一律答 `cancelled`，不再问调用方。
- **`Answer.text` 是 agent 这一回合对城说的话**：`agent_message_chunk` 依次拼起来，每一块同时照常交给 `report`。它是城那次请求的回答，汇报是一路上的事，两者由调用方分别记（`crates/sprawling/Spec.lean` §8-4e 第 8 条）。
- **`HarnessProcess::start` 起同意过的那条 `Launch`**：经 `child::command`（不挂城的控制台、自成进程组），程序名由搜索路径补，工作目录是调用方给的那棵 worktree，stderr 丢弃（与 `mcp::stdio` 同理）。起不来答 `E_TOOL_UNAVAILABLE`，恢复语给出预览的那一行命令。进程句柄落地时杀掉并收尸。
- **测试走同一扇门**：测试用 `Lines::over` 读一条内存管道，在另一头用一条线程扮演 agent，与生产读子进程的输出是同一段代码，不另立 trait。

D12 内置条目怎么起只有一个家：快照里同一个 registry id 的那一行。`OFFICIAL` 不写包名与版本，`docs/third-party.md` §1 也不写；新版本随下一次 `cargo xtask acp-catalog` 进来。被否的方案是在 `roster` 里保留五个钉死的启动命令：那是快照之外的第二份版本号，上游每发一次版要改两处，漏改的那一份读起来仍然像真的。

D13 目录随版本附带快照，不在打开页面时拉取。快照同时是离线首次运行时的目录、内置条目的启动命令和默认钉住的版本，首次运行不发任何请求；刷新只在人按下时发生，与 `crates/wire/spec/Answer/Release.lean` 的「人按下才联网」同一条规则。添加时用快照里的版本，不顺带刷新：刷新出的新版本要人再同意一次，这违背最少操作。快照只记 `distribution`，registry 在已发布的索引里本来就剥掉了 `preview`。被否：像 Zed 那样打开页面就拉取并缓存一小时，它让每次打开 ACP 页都向 CDN 发一次请求。

D14 本城不向 agent 提供文件与终端：`initialize` 声明 `fs.readTextFile`、`fs.writeTextFile`、`terminal` 全为 `false`，agent 用它自己的工具。ACP 规格里工具由 agent 自己执行，本城因此只能记录一家 agent 做了什么，不能管辖它。`auth.terminal: true` 与 `terminal` 是两回事：它只说本城能在一个交互终端里重现这条启动命令，好让 agent 列出 terminal 类型的登录；不声明它时 `claude-agent-acp` 答一份空的方法列表。`elicitation` 本版不声明：声明了而答不了 `elicitation/create` 的请求，会让 agent 给出一种本城完不成的登录（codex 的设备码登录依赖 url elicitation），能把 url 交给人的页面落地时再声明。agent 发来的其余请求以 JSON-RPC `-32601` 回答，不静默：不答的请求会把 agent 挂住。

D16 同意绑定启动规格的摘要，而不是绑定条目的 id。摘要是 `Launch` 的程序、每个参数、每个 env 的名与值依次写成「长度＋字节」再取 BLAKE3（`kernel::B3Hash::digest`，城唯一的内容散列）；带长度前缀，所以 `["a b"]` 与 `["a", "b"]` 不会撞成同一个摘要。同意卡片显示的那一行与城执行的那一行由同一个摘要绑定：目录刷新之后同一个 id 换了命令，旧的同意不再适用。被否：绑 id 加版本号，它让 `env` 或参数的改动不经同意就生效。

D17 子进程的环境是白名单，不是黑名单。开放名单之后，起的是任何人写的程序；它继承城的全部环境时，城自己的秘密（`SPRAWLING_SECRET_*`）、别的厂商的 key 与任何恰好在环境里的凭据都交给了它。`child::command` 拿掉的是城自己的秘密，这里再清空其余的，只放行 agent 运行与判断登录方式所需的名字。被否：只靠 `child::command` 的黑名单，它挡不住人自己 shell 里导出的其他厂商的 key。

D18 claude.ai 订阅登录不由本城启动，Console 登录可以。`claude-agent-acp` 的注释原文是 "this integration must never bill a claude.ai subscription"，Anthropic 也不允许第三方应用提供 Claude.ai 登录；本城替人启动它就是在提供它。拿掉的地方是读 `authMethods` 的那一处，所以页面、拒词与将来的登录执行器都看不见它。被否：保留它并在页面上警告，那仍然是本城在提供这种登录。

D19 内置条目装好或登录过，看的是厂商文档写的那个目录在不在（`Official::set_up`）。当前的表，每行的出处就是行里的那页：

| 内置条目 | 变量 | 家目录下 | 出处 |
|---|---|---|---|
| `claude_code` | `CLAUDE_CONFIG_DIR` | `.claude` | https://code.claude.com/docs/en/settings |
| `codex` | `CODEX_HOME` | `.codex` | https://developers.openai.com/codex/auth |
| `pi` | `PI_CODING_AGENT_DIR` | `.pi/agent` | https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md |
| `grok_build` | `GROK_HOME` | `.grok` | https://docs.x.ai/build/settings/reference |
| `kimi_code` | `KIMI_CODE_HOME` | `.kimi-code` | https://moonshotai.github.io/kimi-code/en/configuration/data-locations |

这五家的文档对三个平台写的是同一个位置：家目录（Windows 的 `%USERPROFILE%`，macOS 与 Linux 的 `$HOME`）下的一个目录，设了变量就换成变量的值；某家给某个平台另写了位置时，这一行按平台分开。家目录与变量由 `bin::doctor::host::place_set_up` 读（`accounting::home::Home::detect`，先 `USERPROFILE` 后 `HOME`），`accounting::views` 只经服务中的城交进来的那个函数读这台电脑。判定是只读的文件存在检查，表里的每一行引这家的官方文档，不猜。`detect` 读它排序与提示（§8-19 检测一条），`environment::passed` 读它放行每家的目录变量。被否：真的起一次 agent 问它版本——慢，而且那是在人同意之前执行它的程序，违反 D16。重开参数：某家提供了稳定的「是否已登录」命令，而且执行它不需要先同意。

**谁用它**：派活路径上的 harness run（`crates/sprawling/Spec.lean` §8-4e、§8-124）。`accounting::worker::driving::harness` 在房间的 worktree 里经 `HarnessProcess::start` 起一个同意过的 agent、开会话，把 `Listener` 接到 `runtime::run::harness::HarnessRun`；会话与子进程在驱动返回时一起丢掉。`accounting::views` 用 `Catalog::bundled`、`Roster` 与 `paste::pasted` 回答 `Query::AgentCatalog`、`Query::ParseAgentSpec` 与 harness 页。
-/

/-! ## 9 工作流程

**出站**：装配层按楼的配置取一条 `McpLink`（先找常驻的，子进程已退出或 HTTP 会话已失效时 `McpLink::open` 新开）→ `handshake`（`initialize` → `notifications/initialized`）→ `Rpc::list_tools` → `tools_from` → catalog 与 bench 各注册一次（工具表随 Run 冻结）→ 模型调用 → `McpTool::invoke` → `call_tool`（浮点检查）→ `Outbound::call` → `Rpc::read` → `ToolOutcome`（污染态）→ 装配层落 `tool_called`／`tool_result`。

**入站**：`wire` 的入站中间件先与这座城的配对令牌常数时间比对（`wire::auth`，未配对即在路由层被拒）→ 装配层收 HTTP／stdio 请求 → `Incoming::parse` → `admit` → `Admitted::Dispatch` → 走与人相同的 `Command::Dispatch` 路径 → 期间回 `Progress`。

**harness 出站**：`HarnessProcess::start` → `AcpSession::open`（`initialize` → `session/new`）→ `prompt`（`session/prompt`，期间汇报、许可与停摆）→ 停止原因 → `Answer`。会话的状态机在 `spec/Harness/Session.lean`。
-/

/-! ## 10 实现逻辑

1. **请求行手工拼装**：`format!` 而不是 `to_string(&map)`，因为 stdio 传输按行分隔且消息内恒不得含换行，而序列化器的换行策略不是本库的契约（D15）。
2. **id 由 `Rpc` 铸**：重放按「方法＋参数」建索引、恒不看 id：重放会重新编号，把 id 计入键就等于永远匹配不上（D15）。
3. **工具名加服务器前缀**：`{label}_{sanitised}`。两台 server 都提供 `search` 时，不加前缀就会有一个工具在不同的楼里做不同的事（`spec/Mcp/Tools.lean` 的 `two_servers_offering_one_verb_stay_two_tools`）。
4. **外部工具声明 `Effect::Connector`＋`Temporal::Timestamped`**：前者把它路由到能说不的那道门，并告诉那道门它去哪；后者是事实：对侧是活的服务，答案是关于此刻的。
5. **入站拒词只泄一位**：未配对的拒绝由中间件出词，不提地址、不提楼、不提令牌像不像。
6. **`Incoming` 字段私有**：外部编辑器的请求只能经 `parse` 成形，于是「空 task 被拒」这条规则没有第二条入口可绕；同理令牌不是它的字段，一个 `derive(Debug)` 的值因此不可能把明文令牌打进日志。

D15 请求行与 id 是本 crate 的契约，不是序列化器的：行由 `format!` 拼、id 由 `Rpc` 铸。被否：交给 `serde_json::to_string` 并让传输层编号，那样「单行」取决于一个库的换行策略，重放的键里混进一个每次都变的数。

D16 MCP stdio 的 server 从 `child::command` 起（`crates/child/Spec.lean`）：Windows 上不附着在城的控制台上，Unix 上在自己的进程组里，`SPRAWLING_SECRET_*` 与 `SPRAWLING_PAIRING_TOKEN` 不进它的环境；`[[mcp]]` 里声明、经 vault 兑现的那几个名字在之后由 `env` 递给它，那是人交给这台 server 的值。其余的环境照旧继承，因为 server 要找得到 `PATH` 与家目录（`mcp::stdio::StdioServer::start`）。理由：stdio server 多半是 `npx` 拉下来的第三方包，继承整个环境就等于把城的全部密钥交给它的每一个传递依赖；附着在城的控制台上的进程能读屏上的配对码与人敲进 CLI 的键。被否：清空环境再放白名单——`[[mcp]]` 没有声明环境变量名的那一栏，一份白名单会让今天能起的 server 起不来，是一个配置面的变化而不是一处修补。钉住它的测试是 `mcp::stdio::tests::a_server_inherits_none_of_the_city_secret_keys`：测试进程带着一个秘密键把自己再起一次，那一次起的 server 读回自己的环境，那个键不在其中。

**两对设计各有被否的一方**，写作 D4、D5，住在 `spec/Mcp/Tools.lean` 的 `floatAt` 与 `construct` 上方：浮点入参拒这一次调用而不是拒整个工具；confidential 楼在构造时就没有这件工具，而不是等 egress 门在调用时拒。

**成本与模型体验**：工具名恒是 `{server}_{action}`，模型一眼看得出它在跟谁说话。浮点拒词报出 JSON 路径而不是「参数非法」，因为模型下一步要改的是那一个字段。工具自己报的错整段进 subject（4 KiB 以内，D1），模型读到的是 server 的原话与它给的恢复。
-/

/-! ## 11 边界枚举

一条消息超过 `MESSAGE_CEILING`／一段 HTTP body 超过它／流在消息中途断掉／消息不是 UTF-8／非 JSON 行／非对象／既无 result 又无 error／`tools` 缺失／工具无名／标签非法／浮点在顶层、数组内、深层对象内／confidential 楼／无期限的注册／空 addr／空 task／空 goal／地址落 reserved prefix／重放缺答案／`isError` 为真且 `_meta` 的值写错／stdio 期限到、对侧关输出、答复被拒／SSE 流在作答前断／HTTP 带会话 id 与不带会话 id 的 404／harness 的未知停止原因与输入在作答前结束／停摆在对侧沉默时到达。

剩余情形，本 crate 分不出：stdio 的行体写完、换行没写出去时管道断了，对侧可能在输入结束时把没有换行的最后一行当作一条消息读；这一刻按对侧没收下报（`Retry::No`）。
-/

/-! ## 12 错误处理

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_INVALID_ARGS` | 浮点入参、入站字段缺失、路由错工具、无期限的注册 | 部分能：浮点由模型给出，故在出口拒并指位置 |
| `E_WIRE_MISMATCH` | 答案或列表形状读不出；一条消息或一段 HTTP 答复的 body 超过 `MESSAGE_CEILING`；流在消息中途断掉；消息不是 UTF-8（后三者发生在一次调用的答案上时带 `Retry::Unknown`，§8-15） | 不能：对侧写多少字节不由本库决定，fail closed；超限恒是整条拒，恒不截断后解析 |
| `E_TOOL_UNAVAILABLE` | server 返回 JSON-RPC error；`tools/call` 答 `isError: true`；stdio 对侧在作答前关了输出、SSE 流在作答前断了（后两者带 `Retry::Unknown`）；重放缺答案 | 不能：外部世界的事实 |
| `E_TIMEOUT` | 期限内未答；请求已交出，带 `Retry::Unknown` | 不能：对侧多久回答不由本库决定；拒词同时是子进程被回收的那一刻 |
| `E_TOOL_OUTCOME_UNKNOWN` | `tools/call` 答 `isError: true`，且 `_meta` 带 `sprawling/effect-unknown` | 不能：server 自己说不知道 |
| `E_GATE_DENIED` | confidential 楼构造出站工具 | 能：构造点即拒，于是「它存在过」这件事不成立 |
| `E_OUTSIDE_WRITE_DOMAIN` | 入站地址落 reserved prefix | 能：判定在 `admit`，无第二条入口 |
| `E_CREDENTIAL_MISSING` | HTTP、SSE 与 broker 答 401／403；一栋楼写的 `secret:realm/name` 引用 vault 里没有（vault 的拒词原样上抛） | 不能：账号在对侧，密钥由人存 |
| `E_CONFIG_INVALID` | HTTP 客户端构造不成；一个头或环境变量没有名字 | 能：在打开连接时拒，早于任何请求 |
| `E_PROVIDER` | broker 的失败（§8-18）；harness 拒了请求、写不进、在作答前关了输出 | 不能：外部世界的事实 |

一个拒绝是三段式的 `AxError`：动作、主体、恢复。失败之后的状态是契约的一部分：读端拒了一条消息就不再读那个 source（D6 (c)），stdio 随即回收子进程、`has_ended` 为真；HTTP 遇带会话 id 的 404 清除 id 与协商版本，并使所有克隆共享的连接失效；原调用返回拒词且不自动重发，旧工具句柄不再发请求，下一次派活由 Residents 新开连接、握手并重取工具表；构造被拒的工具从未存在；`admit` 拒绝时没有 Dispatch。请求交出之后的失败一律 `Retry::Unknown`（`spec/Mcp/Link.lean`）。
-/

/-! ## 13 依赖选型

`kernel`＋`serde_json`＋`gateway`＋`reqwest`。HTTP 客户端只在 `gateway::client_for` 一处构造，本 crate 取它构造好的 builder，只加 user agent 与整请求时限（`mcp::http::WholeRequest`）；`secret:realm/name` 引用经 `gateway::SecretResolver` 兑付。恒不引入异步运行时与任何一家服务商的 SDK：前者会让一次同步的工具调用变成异步库，后者会把「本体不认识任何一家」这条承诺作废。版本见根 `Cargo.toml` 的 `[workspace.dependencies]`。

规格的分部不 import 任何别的 crate 的规格：本 crate 要的 kernel 概念（`Retry` 的三态、地址文法）在模型里写成最小的归纳类型或参数。
-/

/-! ## 14 硬编码声明

外部工具的 `EXTERNAL_CALL_PATIENCE = TimeoutMs(60_000)` 与 `CostTier::Heavy`：外部服务比本地工具慢一个量级，且计费。改动即改变调度与预算行为。

`MESSAGE_CEILING = 8_388_608`：推导来的，不是拍的（D6 (b)）。读数与推导记在 `tools/xtask/budgets.toml` 的 `[mcp_message_ceiling]`，值本身只有这一个家。

`PROTOCOL_VERSION = "2025-06-18"`：本客户端协商的 MCP 修订（§8-3）；桌面 server 引用它，不写第二份。

`HALT_TICK_MS = 200`（`harness::session`）：我们的选择。它是人按下停摆到 harness 收到 `session/cancel` 的上限；比一次按键的反应慢不了多少，又不至于让一条等着 harness 的车道每秒醒几十次。

`EFFECT_META_KEY = "sprawling/effect-unknown"`：我们的约定，理由见 §8-1c；桌面 server 引用它，没有抄本。`ERROR_TEXT_CAP_BYTES = 4_096`：我们的选择，理由见 D1。

`read_whole_message` 每次读 8 KiB：与 `BufReader` 的缺省块同大，两种读法在拒绝前多握的量相同（D8）。
-/

/-! ## 15 影响面

改 `Outbound`、`McpLink` 或 `tools_from` 的签名，波及 `crates/accounting` 的 `accounting::worker::mcp`、`accounting::worker::workbench::servers` 与 `views::mcp_health`；改 `Incoming`／`admit` 波及入站路由与 `wire::auth` 的配对比对；改 `AgentEntry`、`Consented`、`Roster`、`Catalog`、`HarnessProcess`、`AcpSession`、`Listener` 或 `Lines` 波及 `accounting::worker::driving::harness`、`accounting::worker::dispatching::agreeing`、`accounting::views` 与它们的测试（`agent_protocols::Lines` 是它们扮演 agent 时读内存管道的门）；改 `PROTOCOL_VERSION` 或 `EFFECT_META_KEY` 波及 `crates/desktop/` 的抄本。改分部里的模型，先改本文件对应的要求，再改 Rust 与它的测试。
-/

/-! ## 16 测试与约束

证明：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`，`spec` 门判后两条的文本形状。咬得动的演示：`AgentProtocols.Mcp.Reading.withoutCeiling_reads_past_it`、`AgentProtocols.Mcp.Reading.withoutStopping_reads_a_tail_as_a_message`、`AgentProtocols.Mcp.Tools.withUnderscoredLabels_two_servers_collide`、`AgentProtocols.Mcp.Tools.strictReading_repeats_a_misspelled_flag`、`AgentProtocols.Mcp.Link.eagerRetry_performs_twice`、`AgentProtocols.Acp.withoutReserved_reaches_the_citys_own_subtree`，各自说明拿掉哪条守卫后对应的性质不再成立。

实现一致性：逐模块 `#[cfg(test)]`，`cargo nextest run -p sprawling-agent-protocols`。「单行无换行」「同名不合并」「浮点按位置拒」「confidential 构造即拒」「未配对只泄一位」「重放同答案」六条各有一条断言；「恰在上限内的消息照常解析」「超限的消息被整条拒且拒词报出上限与是哪台 server」「stdio 与 SSE 的 server 答出超限的一条时，那次调用被拒且标效果未知」「HTTP 答复的 body 恰在上限内照常读，超过即拒且标效果未知」四条（§8-15）；「`isError` 读成失败」「`_meta` 效果未知」「stdio 超时与断流、SSE 断流标效果未知」三条；harness 会话的取消次序、停止原因与未知词各有断言（`harness::session::tests`），同意的性质由 `harness::entry::tests` 里从 `spec/Harness/Consent.lean` 派生的 proptest 判（任一同意过的规格、任一改动），随版本附带的快照与一份录下的 CDN 索引由 `harness::catalog::tests` 读成目录，内置五家都在快照里且 `npx` 包都钉了确切版本。读端的交接无队列（D6 (d)）是并发的事实，由 `Lines::over` 用 `sync_channel(0)` 的构造守住，模型不表示线程。

模型的证明不是 Rust 实现的证明：两者之间由这些测试连着。

约束：本 crate 恒不出现 `async`、恒不持文件句柄、恒不内置任何服务商名字。
-/

/-! ## 17 文档关系

- `ARCHITECTURE.md` §4 缝清单（`Outbound` 一行）、§10 规则 3（库 crate 起线程的地方：`mcp::reading` 与 `mcp::sse` 的读端）与模块表的 agent_protocols 各行（`architecture.toml`，锚点指向本文件与分部）。这些改了，重读本文件 §7、§8-15 与 §8-19。
- `docs/third-party.md` §1（ACP schema 与 registry 被看的路径）与服务外挂的边界：上游改了线或包名，重读 §5 与 §8-19。
- `crates/kernel/Spec.lean` §8-23（`ServerLabel`、`TimeoutMs`）、`crates/gateway/Spec.lean` §8-5（订阅额度经 harness 进城）、`crates/runtime/Spec.lean` §8-27-10（窗口怎么装工具答复）与 §8-52（harness run 写的行）、`crates/sprawling/Spec.lean` §8-4d（桌面经 stdio 接进来）、§8-4e 与 §8-124（harness run 与它的派活路径）、`crates/wire/Spec.lean` 的配对中间件。这些节改了，重读本文件对应的条目。
- `crates/desktop/Spec.lean` §4 与 `crates/desktop/src/refusal.rs`：`isError` 与 `_meta` 的读法（§8-1c），以及本 crate 对外给出的 `EFFECT_META_KEY` 与 `PROTOCOL_VERSION`。
- `tools/xtask/budgets.toml` 的 `[mcp_message_ceiling]` 与 `[prepare_dispatch]`：上限的推导与常驻连接省下的时间。
-/
