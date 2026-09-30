# agent_protocols-SPEC.md

> crate：`agent_protocols`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

两件方向相反的事，各自可独立验收：

- **出站（`mcp`）**：一个 Resident 调用外部服务的工具，调用落 Ledger 且可离线重演。
- **入站（`acp`）**：一个外部编辑器把这座城当 agent 驱动，请求变成一次普通 Dispatch。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| mcp | 请求恒是单行且不含换行；两台 server 的同名工具恒是两个工具；浮点入参拒该次调用并报出位置；confidential 楼恒不构造该工具；录制的调用重放得同一答案 |
| acp | 已配对请求变成 Dispatch 三字段；`Incoming::parse` 是入站文法的唯一入口（字段私有，本 crate 之外无第二种造法）；配对令牌恒不进入 `Incoming`；持有效令牌也够不到 reserved prefix；回给编辑器的只有 progress 三字段 |

## 3 假设与歧义

- **假设**：用户自己拥有外部服务的账号。本库不内置任何一家的 key、不代付、不做代理。
- **歧义已定**：`2026-07-28` 修订版**删除了协议级 session**（本客户端协商的是 `2025-06-18`，那一版的会话住传输层，见 §8-3），`tools/list` 恒不因连接而异。因此工具表随 Run 冻结与它的规则同向，本库不实现任何会话恢复；需要跨调用状态的 server 自铸句柄，当普通入参传。

## 4 现状分析

两个模块：`mcp`（出站：握手、工具表、三种传输、消息上限、broker）与 `acp`（入站：外部编辑器的请求文法）。生产消费者是 `crates/sprawling`：`assembly::mcp` 按楼的配置连 server 并把连接留给后来的派活（常驻连接），`assembly::workbench::servers` 把工具注册进 bench，`views::mcp_health` 用同一个 `McpLink` 探一台 server 的健康。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 规范总纲与当前修订 | <https://modelcontextprotocol.io/specification/2026-07-28> |
| `tools/list` 恒不因连接而异 | <https://modelcontextprotocol.io/specification/2026-07-28/server/tools> |
| stdio 传输：子进程、按行、消息内无换行 | <https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio> |
| 删除协议级 session、新增 `server/discover` | <https://modelcontextprotocol.io/specification/2026-07-28/changelog> |
| ACP 第 1 版的线：请求、`session/update` 的变体、许可的四种选项、停止原因（§8-19） | `agentclientprotocol/agent-client-protocol` 的 `schema/v1/` 目录里稳定的那份 `schema.json`（不是旁边的 unstable 那份），读到的提交记在 docs/third-party.md §1 |

## 6 命名统一

`Connector` 是词汇表里这层的统称；代码里出现的是它的两个具体面 `McpTool` 与 `Incoming`。**恒不**把 MCP server 叫作 endpoint——`Endpoint` 在本库专指 external provider 网关。

**`ServerLabel` 不住本 crate**：它住 `kernel::tool`（理由与文法见 kernel-SPEC §8-23）。本 crate 继续用它，但不再拥有它：配置层要在**文件边界**解析标签，而 `city` 只见 `kernel`。

## 7 模块边界

- **字节怎么走**归本 crate 的三种传输（`mcp::stdio`、`mcp::http`、`mcp::sse`，见 §8-17）：子进程的拉起、期限与回收，HTTP 会话与事件流都住这里。装配层只决定一栋楼按配置连哪几台 server。
- **哪家 broker 替人持外部应用的 OAuth**也归本 crate（`mcp::broker`，见 §8-18）：出网政策仍只在 `gateway::client_for`。
- **一条消息能有多大**归本 crate（`mcp::reading`）：上限是 MCP 这个协议的事实，不是某一种传输的事实；两个传输各写一个数字就是两条会漂的上限。每种传输拥有自己 reader 的形式（线程、管道、`sync_channel`），它把字节交给 `read_one_message` 并接受它的拒绝。
- **准不准出网**归 `kernel::gate` 的 egress 门：外部工具声明 `Effect::Egress`，路由到那道门；本 crate 只在 confidential 一位上做构造点拒（更早、更硬）。
- **回来的东西算什么**归 `kernel::taint`：与 L0 工具同落 `kernel::tool` 缝，故自动进污染环，本 crate 无解包面。

## 8 接口先行

```rust
// 8-1 mcp（形状 3 端口＋形状 4 适配器＋形状 1 判定）
// call 携期限。声明即承诺可协作取消（kernel-SPEC §8-23 的 TimeoutMs），
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
// ServerLabel 住 kernel::tool（见 §6）；本 crate 不再转导它，
// 一个类型两条导入路径就是一个类型两个住址。
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
    // 无期限的 meta 在构造点即拒——一件没有期限的出站工具就是一件可以挂死 Run 的工具。
    pub fn new(meta: ToolMeta, remote: String, outbound: Box<dyn Outbound>, confidential: bool)
        -> Result<McpTool, AxError>;
    pub fn remote(&self) -> &str;
}
// Tool::invoke 取 &self：Rpc 的 id 与它编号的连接是同一把锁，
// 答案从请求出去的那条连接上读回，所以同一台 server 的两次调用轮流走。
// tools/call 的答复（§8-1c）：`isError: true` 的结果是一次失败，不是一份答案
pub(crate) const EFFECT_META_KEY: &str = "sprawling/effect-unknown";
pub(crate) const ERROR_TEXT_CAP_BYTES: usize = 4_096;
pub struct ScriptedOutbound { /* 私有 */ }                                          // 第二适配器

// 8-1b 外部输入的消息上限（形状 1 判定；见 §8-15）
pub const MESSAGE_CEILING: usize = 8_388_608;
pub enum Received { Message(String), EndOfInput }
pub fn read_one_message(source: &mut dyn BufRead, server: &str) -> Result<Received, AxError>;

// 8-2 acp（形状 1 判定＋形状 2 值类型）
pub struct Incoming { /* 私有：addr／task／goal；唯一构造者是 parse */ }
impl Incoming { pub fn parse(body: &Value) -> Result<Incoming, AxError>; }
pub enum Admitted { Dispatch { addr: Address, task: String, goal: String } }
pub fn admit(request: Incoming) -> Result<Admitted, AxError>;
pub struct Progress { pub run: String, pub turns: u32, pub finished: bool }
```

## 8-3 生命周期与会话

依据是 2025-06-18 修订版 Transports 与 Lifecycle 两篇的规范句：

- **开场必须是 `initialize`**，携 `protocolVersion`、`capabilities`、`clientInfo` 三项；随后必须发 `notifications/initialized`，之后才能问别的。故 `handshake()` 是这条生命周期的**唯一权威**，坐在两个传输之上——两个传输各写一遍就是两份会漂的生命周期。
- **`capabilities` 故意为空**：roots／sampling／elicitation 是 server 反过来向**我们**要的能力；声明一项本城没实现的能力，等于招来一个随后只能拒的请求。
- **会话住传输层，不住本 crate**，因为规范把它写在 Transports 而不是 Lifecycle：server **可选**在 `initialize` 应答的头里发 `Mcp-Session-Id`；一旦发了，客户端 **MUST** 在此后每一次请求带回。404 意味着 server 结束了会话，**MUST** 重开一个——故 `agent_protocols::mcp::http` 遇 404 丢掉 id，而不是拿一个已死的 id 永远碰下去。带着会话 id 的 404 标 `retriable`：按规范 server 对已结束会话的请求一律答 404，调用没有被执行，下一次派遣先重开会话；不带会话 id 的 404 标不可重试，它说的是地址不对，再问只得到同一个答。
- **已知的向前变化**：更新的修订正在把会话去掉（SEP-2575）。本客户端协商的是 `2025-06-18` 并按那一版行事；一台忽略该头的 server 不会因此变得不可用。
- **HTTP 传输带自己的 User-Agent**：CDN 后面的托管 server 可能对不报名的客户端回 403 `browser_signature_banned`，早于任何 MCP 消息。

### 入向浮点：治我们发的，适应我们收的

`digits_for_floats` 把对侧答案里的小数**原样写成字符串**。搜索类 server 的答案常带相关度分之类的小数；Ledger 不收浮点，照搬就会让每一次调用都以 `E_INVALID_ARGS` 失败。三条理由：① 禁浮点是 **Ledger 的**规矩（确定性第 6 条），不得放松；② 拒掉整个答案等于声明本城接不了任何真实的搜索 server；③ 丢掉该字段是隐形地删别人的数据。写成字符串**不丢一位数字、不做任何算术**，且在 Ledger 里看得见（带引号的数）。与 `call_tool` 拒掉携浮点的**入参**并不矛盾：本城治自己发出去的，适应自己收回来的。

### 8-1c 一次 `tools/call` 的答复

MCP 2025-06-18 把工具的答复定为 `CallToolResult`：`content` 是内容块数组；`isError` 为真时，这是工具自己报的错。规格要求工具的错误放进结果、置 `isError`，不回协议层的 JSON-RPC error。`McpTool::invoke` 按这条读：

- `isError` 不为真：`result` 经 `digits_for_floats` 成为 `ToolOutcome.result`。窗口怎么装它，归 `runtime::pipeline::connector`（runtime-SPEC 8-27-10）。
- `isError` 为真：这是一次失败，`invoke` 回 `Err`。subject 是 `<remote> reported a failure: <文字>`，文字是全部 `type: "text"` 块按原顺序以换行连起；超过 `ERROR_TEXT_CAP_BYTES` 时在字符边界截断，并写明截掉了多少字节；非文字块不进 subject，只报个数。码是 `E_TOOL_UNAVAILABLE`，`Retry::No`。
- `isError` 为真且 `_meta` 里有 `sprawling/effect-unknown`、值不是 `false`：码是 `E_TOOL_OUTCOME_UNKNOWN`，标 `effect_unknown`（`Retry::Unknown`）。这个键说的是「这次调用交出去了一部分，桌面或别处是否已经生效不知道」。值写错也按「不知道」读：错读成「没生效」，模型会把一次可能已经落地的动作再做一遍。
- 协议层的 JSON-RPC error 照旧由 `Rpc::read` 读成 `E_TOOL_UNAVAILABLE`，只取 `code` 与 `message`，不读 `data`：`data` 的形状各家自定，城只认规格定过的东西。

键名 `sprawling/effect-unknown` 合 `_meta` 的键名格式：前缀是一个以字母开头的标签加斜杠，不落在 `mcp`／`modelcontextprotocol` 的保留前缀里。它的唯一定义是 `mcp::tools::EFFECT_META_KEY`；墙外的 `desktop/` 抄一份，由 `xtask guard` 比对。

**决定**：工具报的错成为 `Err`（选中）vs 原样当成一份答案交给窗口（落选）。落选那条让模型读到的 `is_error` 为假，账本记成 `Answered`，一次失败看起来像一个古怪的结果。4 KiB 的上限是我们的选择：一句拒词加一句恢复写得下；更长的错误文字多半是 server 把堆栈或整页内容塞了进来，而这条路上没有 CAS 可以把它存下再分窗，窗口与账本为它付的代价比它能告诉模型的多。**被否**：读 JSON-RPC error 的 `data.code`、`data.recovery`、`data.retry`——规格外的约定，城要为每一家 server 猜一次形状。

## 8.5 两个设计

**第一对（浮点入参怎么处置）**：拒掉整个工具（落选）vs 拒掉这一次调用（选中）。禁浮点的真实来源是 Ledger 载荷——一次记不下来的调用就是一次重演不出来的调用。而工具的 schema 里有一个数值字段，并不说明这个工具不可用；拿一次坏参数把整件能力下架，惩罚的是下一次本来正确的调用。故按**调用**拒，并在拒词里报出那个值的路径（`a.b[1]`），让模型改的是那一处而不是猜整张表。

**第二对（confidential 怎么落）**：调用时由 egress 门拒（落选）vs 构造时就不存在（选中）。前者依赖每条路径都记得问那道门；后者让「这栋楼有一个出站工具」这件事本身不成立——那时还没有任何东西可泄。两者不冲突：egress 门仍在，这只是把同一条判断挪到更早、更便宜、更难绕的位置。

## 9 工作流程

**出站**：装配层按楼的配置取一条 `McpLink`（先找常驻的，子进程已退出时 `McpLink::open` 新开）→ `handshake`（`initialize` → `notifications/initialized`）→ `Rpc::list_tools` → `tools_from` → catalog 与 bench 各注册一次（工具表随 Run 冻结）→ 模型调用 → `McpTool::invoke` → `call_tool`（浮点检查）→ `Outbound::call` → `Rpc::read` → `ToolOutcome`（污染态）→ 装配层落 `tool_called`／`tool_result`。

**入站**：`wire` 的入站中间件先与这座城的配对令牌常数时间比对（`wire::auth`，未配对即在路由层被拒）→ 装配层收 HTTP／stdio 请求 → `Incoming::parse` → `admit` → `Admitted::Dispatch` → 走与人相同的 `Command::Dispatch` 路径 → 期间回 `Progress`。

**配对判定不住本 crate**：令牌住 `wire`，判定住那一层中间件，本 crate 因此既看不见密钥也不持有它的副本；`admit` 只判 reserved prefix 这一条本 crate 独有的规则。

## 10 实现逻辑

1. **请求行手工拼装**：`format!` 而不是 `to_string(&map)`，因为 stdio 传输按行分隔且消息内恒不得含换行，而序列化器的换行策略不是本库的契约。
2. **id 由 `Rpc` 铸**：重放按「方法＋参数」建索引、恒不看 id——重放会重新编号，把 id 计入键就等于永远匹配不上。
3. **工具名加服务器前缀**：`{label}_{sanitised}`。两台 server 都提供 `search` 时，不加前缀就会有一个工具在不同的楼里做不同的事。
4. **外部工具声明 `Effect::Egress`＋`Temporal::Timestamped`**：前者把它路由到能说不的那道门，后者是事实——对侧是活的服务，答案是关于此刻的。
5. **入站拒词只泄一位**：未配对的拒绝由中间件出词，不提地址、不提楼、不提令牌像不像。
6. **`Incoming` 字段私有**：外部编辑器的请求只能经 `parse` 成形，于是「空 task 被拒」这条规则没有第二条入口可绕；同理令牌不是它的字段，一个 `derive(Debug)` 的值因此不可能把明文令牌打进日志。

## 11 边界枚举

一条消息超过 `MESSAGE_CEILING`／流在消息中途断掉／消息不是 UTF-8／非 JSON 行／非对象／既无 result 又无 error／`tools` 缺失／工具无名／标签非法／浮点在顶层、数组内、深层对象内／confidential 楼／空 addr／空 task／空 goal／地址落 reserved prefix／重放缺答案。

## 12 Decisions

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_INVALID_ARGS` | 浮点入参、入站字段缺失、路由错工具 | 部分能：浮点由模型给出，故在出口拒并指位置 |
| `E_WIRE_MISMATCH` | 答案或列表形状读不出；一条消息超过 `MESSAGE_CEILING`；流在消息中途断掉；消息不是 UTF-8 | 不能：对侧写多少字节不由本库决定，fail closed；超限恒是整条拒，恒不截断后解析 |
| `E_TOOL_UNAVAILABLE` | server 返回 JSON-RPC error；`tools/call` 答 `isError: true`；stdio 对侧在作答前关了输出、SSE 流在作答前断了（后两者带 `Retry::Unknown`）；重放缺答案 | 不能：外部世界的事实 |
| `E_TIMEOUT` | 期限内未答；请求已交出，带 `Retry::Unknown` | 不能：对侧多久回答不由本库决定；拒词同时是子进程被回收的那一刻 |
| `E_TOOL_OUTCOME_UNKNOWN` | `tools/call` 答 `isError: true`，且 `_meta` 带 `sprawling/effect-unknown` | 不能：server 自己说不知道 |
| `E_GATE_DENIED` | confidential 楼构造出站工具 | **能**：构造点即拒，于是「它存在过」这件事不成立 |
| `E_OUTSIDE_WRITE_DOMAIN` | 入站地址落 reserved prefix | 能：判定在 `admit`，无第二条入口 |

**五家的版本钉子只有一个家：`Harness::launch`（§8-19）。** 本 SPEC 写怎么选版本（读 registry、钉死、跟正式版），不写五家的包名与版本号。两处都写时没有门把它们绑在一起，上游每发一次版要改两个文件，漏改的那一份读起来仍然像真的。被否的方案是在 §8-19 保留五个版本号作说明，这正是会漏改的那一份。

**跟 `agent.json` 的 `distribution`，不跟 `preview`（§8-19）。** registry 为同一家同时给出正式版（`distribution`）与预览版（`preview`），本城钉正式版：预览版比正式版发得勤，每发一次看守就开一个 issue，而它还没有被发布方当作正式版交出。被否的方案是跟 `preview`，它只让本城更早拿到发布方自己还没定稿的适配器。

## 13 依赖选型

`kernel`＋`serde_json`＋`gateway`＋`reqwest`。HTTP 客户端只在 `gateway::client_for` 一处构造，本 crate 取它构造好的 builder，只加 user agent 与整请求时限（`mcp::http::WholeRequest`）；`secret:realm/name` 引用经 `gateway::SecretResolver` 兑付。**恒不引入**异步运行时与任何一家服务商的 SDK：前者会让一次同步的工具调用变成异步库，后者会把「本体不认识任何一家」这条承诺作废。

## 14 硬编码声明

外部工具的 `TimeoutMs(60_000)` 与 `CostTier::Heavy`：外部服务比本地工具慢一个量级，且计费。改动即改变调度与预算行为，属 15.2 行为变更。

`HALT_TICK_MS = 200`（`harness::session`）：我们的选择。它是人按下停摆到 harness 收到 `session/cancel` 的上限；比一次按键的反应慢不了多少，又不至于让一条等着 harness 的车道每秒醒几十次。

`EFFECT_META_KEY = "sprawling/effect-unknown"`：我们的约定，理由见 §8-1c；改它要同时改 `desktop/src/refusal.rs` 的抄本，`xtask guard` 会指出没跟上的那一边。`ERROR_TEXT_CAP_BYTES = 4_096`：我们的选择，理由见 §8-1c。

## 15 影响面

改 `Outbound`、`McpLink` 或 `tools_from` 的签名，波及 `crates/sprawling` 的 `assembly::mcp`、`assembly::workbench::servers` 与 `views::mcp_health`；改 `Incoming`／`admit` 波及入站路由与 `wire::auth` 的配对比对。

## 16 测试与约束

逐模块 `#[cfg(test)]`；「单行无换行」「同名不合并」「浮点按位置拒」「confidential 构造即拒」「未配对只泄一位」「重放同答案」六条各有一条断言，再加「恰在上限内的消息照常解析」「超限的消息被整条拒且拒词报出上限与是哪台 server」两条（§8-15），以及「`isError` 读成失败」「`_meta` 效果未知」「stdio 超时与断流、SSE 断流标效果未知」三条。**约束**：本 crate 恒不出现 `async`、恒不持文件句柄、恒不内置任何服务商名字。

## 17 模型体验

工具名恒是 `{server}_{action}`，模型一眼看得出它在跟谁说话。浮点拒词报出 JSON 路径而不是「参数非法」，因为模型下一步要改的是那一个字段。

## 18 文档同步

`ARCHITECTURE.md` §3 缝清单（`Outbound` 一行）与模块表的 agent_protocols 各行｜`docs/third-party.md` §二（服务外挂的四条边界）。

### 8-15 外部输入通道的消息上限

**缺陷所在**：stdio reader 用 `BufReader::lines()`，遇到换行前无限增长一个 `String`；承载答案的 `mpsc::channel()` 又是无界的。MCP server 是 `CONFIG.toml` 指进来的外部方，其输出是不受信任的输入，而读它的进程同时是这座城**唯一的写者**——内存耗尽等于历史中断，不是一次工具调用失败。

三条决定：

1. **上限不是参数。** 签名是 `read_one_message(source, server)`，上限取自 `MESSAGE_CEILING`，不由调用方传入。理由：一个可以由调用方选的上限，就是每个传输各选一个的上限，而拒词必须对每一台 server 报出同一个数字。测试用真实大小的输入压两边边界，不靠注入一个小上限。
2. **数值是推导来的，不是拍的。** `IMAGE_MAX_BYTES` 是 2 MiB，base64 后 2,796,203 B；8 MiB 让一次工具答案装得下这样一张图、它的文字与 JSON-RPC 信封，对本城已接受的最大载荷留 3 倍余量。读数与推导记在 `tools/xtask/budgets.toml` 的 `[mcp_message_ceiling]`，值本身只有 `MESSAGE_CEILING` 一个家。
3. **拒绝是终止性的，不是跳过一条。** 超限消息未读完的尾巴与下一条消息在字节上无从分辨，所以拒绝之后调用方**恒不**再从同一个 source 读；传输回收子进程。`Received` 是穷尽枚举而非 `Option<String>`：「对侧关了输出」是调用方要据以停读的状态，用缺席表示它就等于让每个调用点各自重推一遍。

**同集的另一半住传输**（`mcp::stdio`、`mcp::http`、`mcp::sse`）：reader 线程改调 `read_one_message`，`mpsc::channel()` 换 `sync_channel(N)`——无界队列在读端慢时把内存吃光，而「慢」正是一个被工具卡住的 Run 的常态；HTTP 那条用 `take(MESSAGE_CEILING)` 包住响应体。

### 8-16 常驻连接

一次连接的开销是每台 server 两次有应答的请求（`initialize`、`tools/list`）加一条通知（`notifications/initialized`），由 `mcp::handshake` 的 `opening_one_connection_costs_two_round_trips_and_one_notification` 用一个计数 `Outbound` 钉住。断言的是条数而不是时间：条数在每台机器上相同。本 crate 为这三条消息花的 CPU 远小于子进程启动与两次往返，而后两者属于传输。

所以本 crate 不持连接表：`McpLink` 可 clone，寿命由持有者决定。持有者是装配层的 `assembly::mcp::Residents`，它把一台 server 的连接与工具表留给后来的派活，子进程退出（`McpLink::has_ended`）时才重开。派活在这一步花的时间记在 `tools/xtask/budgets.toml` 的 `[prepare_dispatch_ms]`。

### 8-17 三种传输与 `McpLink`（形状 4 适配器；实现 `Outbound`）

```rust
// mcp::link：一台可达的 server，不论经哪种传输
#[derive(Clone)]
pub struct McpLink(Reach); // 私有：enum Reach { Stdio, Http, Sse }；克隆即同一条连接的第二个句柄
impl McpLink {
    pub fn open(transport: &kernel::McpTransport, write_root: &Path,
                resolve: &gateway::SecretResolver) -> Result<McpLink, AxError>;
    pub fn site(transport: &kernel::McpTransport) -> &'static str; // 出事时该打开的模块
    pub fn has_ended(&self) -> bool; // 子进程已退出：持有者据此重开
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

- **传输住协议旁边，不住组合根**：三种传输与握手说同一个协议，差别只在字节去哪。放在装配层时，`agent_protocols` 定义了 `Outbound` 缝却看不见它的生产实现；组合根只剩「一栋楼按配置连哪几台 server」（`bin::assembly::mcp`）。拒绝的另一方案是把传输留在 `sprawling`、只搬 `McpLink`：那样 `McpLink` 的三个分支仍指向另一个 crate 的私有类型，搬不动。
- **`site()` 由传输的拥有者给出**：报错地址是「哪个模块到达了这台 server」，只有拥有这三个模块的 crate 能不漂地说出它。
- **HTTP 与 SSE 共用一个客户端构造**（`mcp::http::client_for`）：代理规则、user agent 与构造失败的拒词只写一处。两者唯一的差别是整请求时限，作为参数 `WholeRequest` 递进去：HTTP 取 `DefaultTimeout`（一次 post 与它的回答），SSE 取 `Unbounded`，因为 SSE 的 body 就是整段对话，reqwest 的整请求时限会把流掐断。被否：SSE 自留一份构造只为多一行 `timeout(None)`——两份构造一旦有一份改了代理规则，另一份就静默地走另一条出网路径。
- **子进程回收逻辑原样搬移**：期限到即杀子进程，理由见 `mcp::stdio` 的模块文档。
- **`echoing` 在 `conformance` 后面**：装配层的测试要起同一个假 server；产品二进制不带它（`xtask artifact`）。
- **请求交出之后丢了答，效果未知，不可重试**：请求已经完整交给对侧之后——stdio 是那一行写完并 flush，SSE 是 POST 得到 2xx——期限内没有答案（`E_TIMEOUT`），或对侧在作答前关了输出、流断了（`E_TOOL_UNAVAILABLE`），都标 `effect_unknown`（`Retry::Unknown`，kernel-SPEC 的三态）：server 可能已经做了，再发一次同一调用可能把一次写做两遍，由看得见这次调用的人决定要不要再问。请求还没交出去时的失败（stdio 写管道失败、POST 本身失败）按对侧没收下读，照旧 `Retry::No`。剩余情形：stdio 的行体写完、换行没写出去时管道断了，对侧可能在输入结束时把没有换行的最后一行当作一条消息读；这一刻本 crate 分不出，按没收下报。
- 失败码不变：各传输沿用 §12 的 `E_TIMEOUT`／`E_WIRE_MISMATCH`／`E_TOOL_UNAVAILABLE`，HTTP 与 SSE 在 401／403 抬 `E_CREDENTIAL_MISSING`，客户端构造不成抬 `E_CONFIG_INVALID`。

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

- **住 `agent_protocols::mcp`，不住 `gateway`**：它回答的是「连哪台 MCP server 上的哪个应用」，与三种传输同属一件事；出网的两条政策（代理规则、HTTP 客户端的构造）仍只在 `gateway::client_for` 一处，broker 经它取客户端，测试的 `Broker::at` 也一样（`Proxying::ExceptLocal` 对回环地址不走代理），所以本库之内没有第二个构造点。拒绝的另一方案是留在 `gateway`：那样 MCP 一分为二，`gateway` 要知道一家 MCP 服务的目录形状。
- **返回自己的词汇，不返回线上形状**：装配层把 `Toolkit`／`Connection` 映成 `wire::ToolkitLine`，与它把握手映成 `McpState` 同一做法。
- **只有一家 broker，所以没有 trait**：第二家外包服务才是这条缝的第二个实现。
- 失败码：401／403 抬 `E_CREDENTIAL_MISSING`；408／429／5xx 抬可重试的 `E_PROVIDER`；连接阶段超时抬可重试的 `E_PROVIDER`（请求还没离开这台电脑）；请求发出之后等答超时抬 `effect_unknown` 的 `E_PROVIDER`，因为 `connect` 会在 broker 那边建一份 auth config，重发可能建出第二份；2xx 之后 body 读不完（连接在答案中途断开）同样抬 `effect_unknown` 的 `E_PROVIDER`，subject 带读不出的原因——broker 已经照做了，丢的只是答案；非 2xx 的 body 读不出时，读不出的原因代替 body 作附近文字；其余状态、读不出的答案与接不上 base 的路径抬 `E_PROVIDER`。接不上的路径在 recovery 里报出本模块的路径（`module_path!()`），模块再搬家也不漂。
- 字段按防御方式读：缺一个字段少一行，不毁整张答案；测试里的假 server 是本 crate 对 broker 所发内容的陈述。

### 8-19 官方 harness 出站：五家与一场 ACP 会话（`agent_protocols::harness`；`roster` 形状 6 数据面，`session` 形状 4 适配器）

订阅额度由厂商自己的 harness 带进城（gateway-SPEC §8-5）。本节是这条路的传输半：认得哪几家、怎么把一家起成一个说 ACP 的子进程、怎么跟它开一场会话并把它说的话读回来。**本城是 ACP 的 client**，与 §8-2 的入站方向相反。

```rust
// harness::roster —— 数据面，定规：只有这五家
pub enum Harness { Codex, ClaudeCode, GrokBuild, KimiCode, Pi }   // as_str(): codex|claude_code|grok_build|kimi_code|pi
impl Harness {
    pub const ALL: [Harness; 5];
    pub const fn launch(self) -> Launch;          // 起一个说 ACP 的进程：程序与参数
    pub const fn registry_id(self) -> &'static str;   // ACP registry 里的 id
    pub const fn docs(self) -> &'static str;      // 这家自己写的登录说明
    pub fn parse(word: &str) -> Option<Harness>;  // as_str 的逆；不认识的词答 None，拒词由调用方按它的场合写
}
pub struct Launch { pub program: Program, pub args: &'static [&'static str] }
pub enum Program { Npx, Kimi }
impl Program { pub const fn name(self) -> &'static str; }   // Windows 上 npx 是 npx.cmd，kimi 是 kimi.exe 由搜索路径补

// harness::reading —— 读端：一条线程把对侧的行交进通道，会话带期限地等
pub struct Lines { /* 私有：Receiver<Result<Received, AxError>> */ }
impl Lines { pub fn over<R: BufRead + Send + 'static>(reader: R, name: &str) -> Result<Lines, AxError>; }

// harness::process —— 把一家 harness 起成子进程（形状 4 适配器）；落地即杀
pub struct HarnessProcess { /* 私有：Child */ }
impl HarnessProcess {
    pub fn start(harness: Harness, cwd: &Path) -> Result<(HarnessProcess, AcpSession<ChildStdin>), AxError>;
}

// harness::session —— 一场 ACP 会话，JSON-RPC 2.0，按行分帧
pub struct AcpSession<W: Write> { /* 私有：lines、writer、下一个请求 id、session id、harness 名、是否已取消 */ }
impl<W: Write> AcpSession<W> {
    pub fn open(lines: Lines, writer: W, name: &str, cwd: &Path) -> Result<Self, AxError>;   // initialize ＋ session/new
    pub fn prompt(&mut self, text: &str, listener: &mut Listener<'_>) -> Result<Answer, AxError>;
}
pub struct Listener<'a> {
    pub halted: &'a mut dyn FnMut() -> bool,                    // 每条消息到达前、对侧每沉默满 HALT_TICK_MS 时各问一次
    pub cancelling: &'a mut dyn FnMut() -> Result<(), AxError>, // 头一次答「停了」时调一次，在 session/cancel 发出之前
    pub report: &'a mut dyn FnMut(Update) -> Result<(), AxError>,
    pub permit: &'a mut dyn FnMut(&PermissionAsk) -> Permit,    // 取消之后不再问它，一律答 cancelled
}
pub struct Answer { pub stop: StopReason, pub text: String }    // text：这一回合的 agent_message_chunk 依次拼起来
pub enum Update { Text(String), Thought(String), ToolCall { id: String, title: String, kind: String },
                  ToolCallStatus { id: String, status: String }, Other { variant: String } }
pub struct PermissionAsk { pub title: String, pub options: Vec<PermitOption> }
pub struct PermitOption { pub id: String, pub name: String, pub kind: PermitKind }
pub enum PermitKind { AllowOnce, AllowAlways, RejectOnce, RejectAlways }
pub enum Permit { Chosen(String), Cancelled }
pub enum StopReason { EndTurn, MaxTokens, MaxTurnRequests, Refusal, Cancelled }
```

- **名单是定规**：Codex、Claude Code、Grok Build、Kimi Code、Pi 五家是人认可的全部 harness。增一家要人另定，不因为 ACP registry 里多了一行就跟着加。
- **怎么起一家，读 ACP registry**（<https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json>，逐家的 `agent.json` 在 `agentclientprotocol/registry`，被看路径见 docs/third-party.md §1）：Claude Code 与 Codex 各经 registry 组织发布的适配器包，Pi 经第三方的适配器包，Grok Build 经它自己的包并带 `agent stdio` 两个参数，这四家都由 `npx -y <包>@<版本>` 起；Kimi Code 是人装好的 `kimi acp`。包名、版本与参数只写在 `Harness::launch`，取 `agent.json` 里 `distribution` 那一项（§12）。**版本钉死**：`npx` 不带版本会在每次起进程时向 npm 取最新的包，一个没人看过的版本就进了城；`every_package_a_harness_is_fetched_as_is_pinned_to_a_version` 要求每个 `npx` 包都带版本。
- **登录是人在 harness 里做的**：本城不起登录流程、不读 harness 的凭据文件。`docs` 是每家自己写的登录说明，页面只把它交给人。
- **本城不向 harness 提供文件与终端**：`initialize` 声明 `fs.readTextFile`、`fs.writeTextFile`、`terminal` 全为 `false`，harness 用它自己的工具。ACP 规格里工具由 agent 自己执行，`session/request_permission` 是 agent 可以不发的请求，工具名 "do not advertise a capability or grant authorization"（<https://agentclientprotocol.com/protocol/tool-calls>）；本城因此只能**记录**一家 harness 做了什么，不能**管辖**它。harness 发来的其余请求（`fs/*`、`terminal/*`）以 JSON-RPC `-32601` 回答，不静默。
- **一条消息的上限与 MCP 同一个**：`read_one_message` 与 `MESSAGE_CEILING`（§8-15）。ACP 与 MCP 同是按行分帧的 JSON-RPC，一个图片加信封的上限对两者是同一个事实。
- **`prompt` 在对方答出 `StopReason` 时返回**；读到输入结束而没有答，是 `E_PROVIDER` 并标 `Retry::Unknown`：对方也许已经做了事。`StopReason` 未知的词拒而不猜。
- **停摆在对侧沉默时也要变成取消**：`BufRead` 的读没有期限，一家在跑长命令的 harness 可以几分钟一行不写，而一次停摆不能等它开口。所以读端在自己的线程上（`Lines::over`），把行交进通道；`prompt` 每次最多等 `HALT_TICK_MS` 就回头问一次 `halted`。线程在对侧关闭输出（子进程被杀）或会话丢掉通道时结束，不会泄漏。ARCHITECTURE §10 规则 3 把它列为库 crate 起线程的一处。
- **取消的次序是 `crates/agent_protocols/spec/Harness/Session.lean` 定的**：`halted` 头一次答真，先调 `cancelling`（调用方在这里把 `cancel_received` 落账），再发 `session/cancel`，此后的汇报排在它后面；第二次答真什么也不发。取消之后 agent 再问 permission，一律答 `cancelled`（ACP 要求客户端这样答取消后的每一个 permission 请求），不再问调用方。
- **`Answer.text` 是 agent 这一回合对城说的话**：`agent_message_chunk` 依次拼起来，每一块同时照常交给 `report`。它是城那次请求的回答，汇报是一路上的事，两者由调用方分别记（sprawling-SPEC §8-4e 第 8 条）。
- **`HarnessProcess::start` 起 `Launch` 的程序与参数**：程序名由搜索路径补（`Program::name`），工作目录是调用方给的那棵 worktree，stderr 丢弃（与 `mcp::stdio` 同理：那是它的诊断，不是答案）。起不来答 `E_TOOL_UNAVAILABLE`，恢复语给出这家自己的 `docs()`。进程句柄落地时杀掉并收尸，所以「谁回收它」不需要第二份名单。
- **测试走同一扇门**：测试用 `Lines::over` 读一条内存管道，在另一头用一条线程扮演 agent，与生产读子进程的输出是同一段代码，不另立 trait。

**尚未做到的（本节接口的当前状态）**：一个 harness 居民的 run 还没有接线：派活到这样的居民时起它的进程、在房间的 worktree 里开会话、把 `Update` 写进账本、把停摆译成 `session/cancel`、confidential 楼拒绝构造它。它必须守住的性质已在 `crates/agent_protocols/spec/Harness/Session.lean` 证明，设计写在 sprawling-SPEC §8-4e；本 crate 这一侧（`Harness::parse`、`harness::process`、`harness::reading` 与 `session/cancel`）已落地，接线欠在 kernel、city 与 sprawling 三处。在接线之前，设置页的 harness 页只说明五家在这台电脑上够不够得着、怎么起、去哪里登录。
