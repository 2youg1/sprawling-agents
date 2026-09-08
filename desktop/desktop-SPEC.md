# desktop-SPEC.md

> package：`sprawling-desktop`（out-of-tree，**不是** workspace member）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> card-7.1 只造壳、协议、拒绝故事与 scope 文件；Windows 实现属 card-7.2。

## 1 需求拆解

一件事：**给 agent 一双眼睛和一双手，落在这台 Windows 桌面上**，并且这双手从第一天起就受一份 allowlist 约束。

拆成三件可独立验收的事：

- **协议壳（`rpc` ＋ `session`）**：一台按行说话的 MCP server，握手与 `tools/call` 的形状与 `crates/protocol/src/mcp/*` 所写的客户端逐字对齐，于是城里一栋楼的 `CONFIG.toml` 用一条普通 `command` 就能接上它，装配层无需为它开任何特例。
- **工具表（`tools`）**：六件工具的名字、说明与入参 schema **此刻定死**，实现留给 card-7.2。每条说明都写明这件工具**不做**什么。
- **拒绝故事（`scope` ＋ `platform`）**：越界与未实现都回一个带稳定错误码与恢复句的 JSON-RPC error。没有 scope 文件＝全拒。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| rpc | 一条消息恒是单行且不含换行；答案恒携原 `id`；读不出的行回 `-32700` 而不是沉默 |
| session | 未握手完成前 `tools/list`／`tools/call` 恒被拒；`notifications/initialized` 恒无答案；`ping` 恒答空对象；未知方法回 `-32601` |
| tools | 六个名字恒在 `tools/list` 里；每条 description 恒含一句「不做什么」；每张 `inputSchema` 恒是 `type: object` |
| scope | 缺文件与坏文件恒全拒；空 allowlist 恒不容许任何窗口；`record`／`clipboard` 未开则该工具恒被拒；不指名窗口的整屏截取恒被拒 |
| platform | 非 Windows 上恒回 `E_TOOL_UNAVAILABLE` 并报出平台名；Windows 上本 build 回同码并说明「尚未在本 build 中实现」 |

## 3 假设与歧义

- **假设**：这台机器上的桌面是操作者自己的桌面。本 package 不做远程桌面、不做跨机器、不做无人值守的持续录制。
- **歧义已定**：card 只给了 allowlist（window title patterns ＋ process names）与 `record`／`clipboard` 两位开关，**没有**给「允许整屏」的表达。故**整屏截取在本版恒被拒**（见 §8.5 第三对），而不是被默许——一张全屏图会显示 allowlist 没有列出的一切。

## 4 现状分析

`crates/protocol` 已经是这套协议的**客户端**权威：`Rpc::initialize`／`initialized`／`list_tools`／`call_tool`／`read` 定死了城里说出去的每一行，`bin::mcp_stdio` 定死了字节怎么走（子进程、按行、消息内无换行、超时即回收子进程）。本 package 是那一端的**对侧**，因此它的形状不是设计出来的，是**读出来的**。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 握手：`initialize` → `notifications/initialized` → 其它 | <https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle> |
| stdio 传输：子进程、按行、消息内无换行 | <https://modelcontextprotocol.io/specification/2025-06-18/basic/transports> |
| `tools/list`／`tools/call` 的 `name`／`description`／`inputSchema` 三字段 | <https://modelcontextprotocol.io/specification/2025-06-18/server/tools> |
| JSON-RPC 2.0 的保留错误码区间与 `-32000` 起的实现自定义区 | <https://www.jsonrpc.org/specification#error_object> |
| 城里客户端实际发出的行 | `crates/protocol/src/mcp/handshake.rs`、`crates/protocol/src/mcp/tools.rs` |
| 城里字节怎么走、超时怎么算 | `crates/sprawling/src/mcp_stdio.rs` |
| `CONFIG.toml` 的 `[[mcp]]` 与 `McpTransport::Stdio { command, args }` | `crates/kernel/src/config.rs` |

## 6 命名统一

沿用词汇表（`docs/glossary.md`）：**three-part refusal**（拒了什么／为什么／还能做什么）、**reference**（快照铸的句柄，形如 `e1`）、**generation**（快照的世代号，随动作一起走）。

- **scope** 在本 package 里专指 `DESKTOP.toml` 里那份 allowlist，与 `kernel` 的 halted scope 不同层，故恒不缩写成裸词 `policy`。
- **恒不**把窗口叫作 page，**恒不**把桌面叫作 browser：`browser::act` 是被借鉴的形状，不是被复用的名字。
- 错误码沿用 `kernel::AxCode` 的**拼写**（`E_TOOL_UNAVAILABLE`／`E_GATE_DENIED`／`E_INVALID_ARGS`／`E_TOOL_UNKNOWN`／`E_CONFIG_INVALID`／`E_WIRE_MISMATCH`），但**不依赖** `kernel`：本 package 在 workspace 之外，一条 `use kernel::…` 会把它拉回墙内。同一拼写、两处定义，是本 SPEC 明知并接受的一处重复，理由见 §8.5 第一对。

## 7 模块边界

- **字节怎么走**归 `main`：argv、环境变量、stdin／stdout 的锁与刷新住那里；`session` 只收一行、出一行。
- **准不准做**归 `scope`：一次 `tools/call` 在碰到平台之前先过它，于是「越界」这件事在任何 Win32 调用之前就已判完。
- **做得成做不成**归 `platform`：`cfg(windows)` 两个文件各自完整，**无 trait**——一个只有一个实现的接口是装饰（ARCHITECTURE §4）。
- **一行 allowlist 匹配什么**归 `scope::pattern`：glob 语义与文件解析是两件会各自变的事，且前者要被单独证明会终止（§10 第 6 条）。
- **说什么**归 `tools`：六张卡片是数据，改它就是改行为（形状 6）。

**恒不**引入：async runtime、HTTP 客户端、任何 GUI 框架、任何 workspace crate。

## 8 接口先行

```rust
// 8-1 refusal（形状 2 值类型）
pub(crate) enum RefusalCode { ToolUnknown, ToolUnavailable, InvalidArgs, GateDenied, ConfigInvalid, WireMismatch }
impl RefusalCode {
    pub(crate) fn as_str(self) -> &'static str;   // E_… 与 kernel 同拼写
    pub(crate) fn json_rpc(self) -> i64;          // 保留区间或 -32000 起
}
pub(crate) struct Refusal { /* 私有：code／action／subject／recovery */ }
impl Refusal {
    pub(crate) fn new(code: RefusalCode, action: &str, subject: impl Into<String>, recovery: &str) -> Refusal;
    // 出口只有这一个：一个只为让测试读字段而存在的 accessor，就是同一个值的第二道门。
    pub(crate) fn as_error(&self) -> Value;       // { code, message, data: { code, action, subject, recovery } }
}

// 8-2 rpc（形状 4 适配器）
pub(crate) const PROTOCOL_VERSION: &str = "2025-06-18";
pub(crate) struct Request { pub(crate) id: Option<Value>, pub(crate) method: String, pub(crate) params: Value }
pub(crate) fn read(line: &str) -> Result<Request, Refusal>;
pub(crate) fn result_line(id: &Value, result: Value) -> String;
pub(crate) fn error_line(id: Option<&Value>, refusal: &Refusal) -> String;

// 8-3 tools（形状 6 数据）
pub(crate) struct ToolCard { pub(crate) name: &'static str, pub(crate) description: String, pub(crate) schema: Value }
pub(crate) fn table() -> Vec<ToolCard>;
pub(crate) fn card(name: &str) -> Option<ToolCard>;

// 8-4 scope（形状 1 判定）
pub(crate) struct Reach<'a> { pub(crate) tool: &'a str, pub(crate) title: Option<&'a str>, pub(crate) process: Option<&'a str> }
// Closed 携码：没人写过的文件是没人给过的许可（E_GATE_DENIED），
// 读不出来的文件是这份文件本身有缺陷（E_CONFIG_INVALID）。两件事，两个码。
pub(crate) enum Scope { Closed { code: RefusalCode, because: String }, Open(Allowance) }
pub(crate) struct Allowance { /* 私有：windows／processes／record／clipboard */ }
impl Scope {
    pub(crate) fn read(path: Option<&Path>) -> Scope;       // 恒不失败：缺文件与坏文件都关成 Closed
    pub(crate) fn parse(text: &str) -> Scope;
    pub(crate) fn admits(&self, reach: &Reach<'_>) -> Result<(), Refusal>;
}

// 8-4b scope::pattern（形状 2 值类型）
pub(crate) struct Pattern { /* 私有：Vec<char> */ }
impl Pattern {
    pub(crate) fn new(line: &str) -> Pattern;
    pub(crate) fn matches(&self, named: &str) -> bool;
}

// 8-5 session（形状 4 适配器；Phase 是形状 5 typestate 的运行时投影）
// 本 package 的全部公开面就这一个函数：一台 server 的其余一切都经协议抵达。
pub fn serve_stdio(scope_path: Option<&Path>) -> std::io::Result<()>;
pub(crate) enum Phase { Fresh, Initializing, Ready }
pub(crate) struct Server { /* 私有：scope／phase */ }
impl Server {
    pub(crate) fn new(scope: Scope) -> Server;
    pub(crate) fn answer(&mut self, line: &str) -> Option<String>;   // None＝这是一条通知
    pub(crate) fn serve(&mut self, input: impl BufRead, output: &mut impl Write) -> std::io::Result<()>;
}

// 8-6 platform（形状 4 适配器；cfg 二选一，无 trait）
pub(crate) fn perform(tool: &str, arguments: &Value) -> Result<Value, Refusal>;
```

### 8-7 六张工具卡片

名字用点号分段（`desktop.act`），城里 `tools_from` 会把它 sanitise 成 `{label}_desktop_act`——分段在两侧都读得出来。

| 名字 | 做什么 | 说明里写明**不做**什么 |
|---|---|---|
| `desktop.windows` | 列顶层窗口：title、process、bounds、ref | 不激活、不移动、不改变任何窗口 |
| `desktop.snapshot` | 一个窗口或整屏的 accessibility tree：role、name、ref、bounds | 不给像素、不给控件的内部句柄、不读被遮挡的内容 |
| `desktop.act` | ref 或 point ＋ 动作（click／double／right／drag／scroll／type／key，带 modifiers），携 snapshot 的 generation | 不合成整段脚本、不重试、不在 generation 过期时改打别处 |
| `desktop.screenshot` | window／screen／region，format png\|jpeg\|webp，quality、scale，回 base64 ＋ width／height／mime | 不做 OCR、不做比对、不落盘 |
| `desktop.record` | start／stop：PATH 上有 ffmpeg 则 mp4，否则一个 PNG 序列目录；audio 可选 | 不做剪辑、不做转码、不在没说 stop 时自己停 |
| `desktop.clipboard` | get／set 文本 | 不碰图片与文件列表、不保留历史 |

`desktop.act` 携 generation 是照抄 `browser::act` 的那一条：**对着一份快照做的决定，恒不落到另一份快照上**——过期就拒，而不是打到那时挪过去的东西上。

## 8.5 四个设计

**第一对（错误码住哪）**：`use kernel::AxCode`（落选）vs 在本 package 重新定义同拼写的一小组（选中）。前者把这个 package 拉回 workspace 的墙内，而它坐在墙外的**理由**就是 card-7.2 要在 Win32 边界上写 `unsafe`；为了六个字符串常量把这个理由作废是本末倒置。选中方案付的代价是同一拼写有两处定义，边界是：本 package 恒只**引用**已有拼写，恒不铸造新的 `E_` 码——新码要先进 `kernel::error::code`。

**第二对（unsafe 怎么关）**：照抄 workspace 的 `unsafe_code = "forbid"`（落选）vs 本 package 用 `deny`（选中）。`forbid` 在文件内无法就地放开，而 card-7.2 要在 Win32 调用点上就地放开、并在那一处写明理由；`deny` 让放开成为**一个带理由的、看得见的、最窄作用域的例外**，而不是把整堵墙推倒。clippy 那张表逐行照抄，一条不减。card-7.1 本身**一行 unsafe 也不写**。

**第三对（整屏怎么办）**：默许整屏截取（落选）vs 无表达即拒（选中）。scope 文件能表达的只有「哪些窗口」，一张全屏图会显示 allowlist 没有列出的一切；把没写下来的东西当成允许，正是 fail closed 要防的那件事。拒词里给的替代是「指名一个窗口」，可执行。等 scope 文件长出一位 `screen` 开关，这条再改，改时先改本节。

**第四对（未实现怎么回答）**：先回一个假的成功形状让上游先接线（落选）vs 回 `E_TOOL_UNAVAILABLE` 并说明这个 build 里没有它（选中）。一个假的成功会让模型据此往下推理，而错误的答案比没有答案贵得多；本 card 的全部价值就是**形状已经定死、拒绝是诚实的**。

## 9 工作流程

进程起来 → `main` 取 scope 路径（argv[1]，否则 `SPRAWLING_DESKTOP_SCOPE`，否则无）→ `Scope::read`（缺文件即 `Closed`）→ `Server::serve` 阻塞读 stdin。

每收到一行：`rpc::read` → 按 method 分派 → `initialize` 答能力与自我介绍并进 `Initializing` → `notifications/initialized` 无答案并进 `Ready` → `ping` 答 `{}` → `tools/list` 答六张卡片 → `tools/call` 先 `Scope::admits`，过了再 `platform::perform` → 出一行 → flush。

## 10 实现逻辑

1. **一条消息一行**：`serde_json::to_string` 不产生换行，且写出前不做美化；这与 `bin::mcp_stdio::Connection::framed` 的检查是同一条契约的两端。
2. **握手顺序被强制**：`Fresh` 只答 `initialize` 与 `ping`，`Initializing` 收到通知才进 `Ready`。规范就是这么写的，而一个不强制它的 server 会让客户端的顺序错误在别处以别的形状爆出来。
3. **`id` 原样回**：不解析、不重编号——`id` 是对侧的东西。通知（无 `id`）恒无答案，否则会在管道里留下一行，此后每次调用读到的都是上一条的答案。
4. **glob 只认 `*`**：title pattern 是给人写的，一整套正则会让「我到底放开了什么」变成一个需要推演的问题。匹配按 ASCII 大小写不敏感，因为 Windows 的进程名就是这样比的。
5. **越界判定在平台之前**：`admits` 是纯判定，无 I/O、无时钟，于是它可以被逐条测，而 Win32 一行都还没跑。
6. **glob 一定终止**：回溯时文本下标只增不减，且到达文本末尾即失败，故循环恒有界；`scope::pattern` 里有一条专门打这一点的测试（一个「几乎匹配很多次」的名字）。
7. **`deny_unknown_fields`**：拼错的键若被默默忽略，写它的人会把它读成一个生效了的键。故坏键＝坏文件＝全拒。

## 11 边界枚举

非 JSON 行／非对象／无 `method`／`params` 非对象／未知方法／握手未完成就调用／`tools/call` 无 `name`／`name` 不在表里／scope 文件缺失／scope 文件语法坏／scope 文件有拼错的键／allowlist 两张表皆空／title 不匹配／process 不匹配／同时给 title 与 process 而只中一个／既不给 title 也不给 process／`record` 未开／`clipboard` 未开／整屏截取／非 Windows 平台／Windows 但本 build 未实现。

**同时给两个标识则两个都要中**：任何一个给出的标识都要落在它自己那张表里，这是 fail closed 的一致读法。

## 12 错误处理

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_WIRE_MISMATCH` | 行不是 JSON、不是对象、无 `method` | 不能：对侧发什么不由本 package 决定，fail closed |
| `E_TOOL_UNKNOWN` | 未知方法、`name` 不在六张卡片里 | 不能：模型会试不存在的名字 |
| `E_INVALID_ARGS` | `params` 形状读不出、`tools/call` 无 `name` | 部分能：schema 已给出，拒词指到那一处 |
| `E_GATE_DENIED` | 越界、`record`／`clipboard` 未开、整屏截取、握手未完成 | **能**（对 scope 而言）：缺文件即 `Closed`，于是「默许」这件事不成立 |
| `E_CONFIG_INVALID` | scope 文件语法坏 | 不能：文件是人写的。坏文件恒关成全拒，恒不退回默认允许 |
| `E_TOOL_UNAVAILABLE` | 非 Windows 平台；Windows 但本 build 未实现 | 不能：这是这台机器与这个 build 的事实 |

拒词恒是三段（three-part refusal）：拒了什么（action）、为什么（subject）、还能做什么（recovery），装进 JSON-RPC error 的 `data` 里；`message` 是人读的一句摘要。

## 13 依赖选型

`serde`＋`serde_json`＋`toml` 三个，与 workspace 同版本线。**恒不引入**：workspace 内任何 crate（理由见 §8.5 第一对）、async runtime、HTTP 客户端、glob crate（§10 第 4 条）。card-7.2 会引入 `windows` crate，那时在本节增一行并写清它买到了什么。

## 14 硬编码声明

`PROTOCOL_VERSION = "2025-06-18"`：与 `protocol::PROTOCOL_VERSION` 同值，理由是两端要谈得拢；它变了，本 package 要在同一次改动里跟着变，故本节是它的第二处台账。服务器自称 `sprawling-desktop`，版本取 `CARGO_PKG_VERSION`。

## 15 影响面

新增 out-of-tree package，无既有调用方。波及两处：根 `Cargo.toml` 的 `[workspace]` 增一行 `exclude = ["desktop"]`（属 gate machinery，单独提交）；`ARCHITECTURE.md` §12 增一节十一行。城里接上它时只需一栋楼的 `CONFIG.toml` 写一条 `[[mcp]]` ＋ `command`，装配层**零改动**——这正是本 card 要验的那一条。

## 16 测试与约束

逐模块 `#[cfg(test)]`，外加 `tests/smoke.rs`：**真的把二进制拉起来**，从管道里灌一次 `initialize` ＋ 一次 `tools/list`，断言六个名字。它是唯一一处证明「城里那条 `command` 真的能接上」的测试，其余测试都只证明库里的判断。

**约束**：本 card 恒不出现 `unsafe`、`unwrap`、`expect`、`panic!`、`todo!`、裸下标、`as`；算术走 `checked_*`／`saturating_*`；每个文件 ≤400 行、每个函数 ≤200 行且 ≤4 参数。`scope.rs` 因这条尺子而在 446 行处切出 `scope/pattern.rs`——切口落在「一行 allowlist 匹配什么」与「这份文件许可什么」之间，是语义的，不是为了凑行数。

验收命令（在 `desktop/` 内）：`cargo fmt`／`cargo clippy --all-targets -- -D warnings`／`cargo nextest run`。根目录的 `just check` 够不到本 package，因为它不是 workspace member。

## 17 模型体验

工具名恒是 `desktop.<动词>`，模型一眼看得出这一件事发生在桌面上而不是页面里。每条说明的后半句写的是**不做什么**，因为模型下一步最贵的错误是把一件工具当成它旁边那件。拒词恒给一个可执行的下一步：越界给「把这个窗口写进 `DESKTOP.toml`」，未实现给「这个 build 里没有它」。

## 18 文档同步

`ARCHITECTURE.md` §12 新增 desktop 一节｜`desktop/README.md`（英文，讲清它为什么住在 workspace 外）｜card-7.2 落地时同步本 SPEC §13 与 §8-7 的实现状态。
