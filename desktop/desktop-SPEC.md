# desktop-SPEC.md

> package：`sprawling-desktop`（out-of-tree，**不是** workspace member）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。

## 1 需求分解

一件事：**给 agent 一双眼睛和一双手，落在这台 Windows 桌面上**，并且这双手从第一天起就受一份 allowlist 约束。

拆成三件可独立验收的事：

- **协议壳（`rpc` ＋ `session`）**：一台按行说话的 MCP server，握手与 `tools/call` 的形状与 `crates/protocol/src/mcp/*` 所写的客户端逐字对齐，于是城里一栋楼的 `CONFIG.toml` 用一条普通 `command` 就能接上它，装配层无需为它开任何特例。
- **工具表（`tools`）**：六件工具的名字、说明与入参 schema **定死**。每条说明都写明这件工具**不做**什么。
- **拒绝故事（`scope` ＋ `platform`）**：越界与未实现都回一个带稳定错误码与恢复句的 JSON-RPC error。没有 scope 文件＝全拒。

第三件事的后半句是**真的实现**，且它换掉的只有 `platform` 一个模块：协议壳与工具表一行不改，这正是把形状定死所买到的东西。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| rpc | 一条消息恒是单行且不含换行；答案恒携原 `id`；读不出的行回 `-32700` 而不是沉默 |
| session | 未握手完成前 `tools/list`／`tools/call` 恒被拒；`notifications/initialized` 恒无答案；`ping` 恒答空对象；未知方法回 `-32601` |
| tools | 六个名字恒在 `tools/list` 里；每条 description 恒含一句「不做什么」；每张 `inputSchema` 恒是 `type: object` |
| scope | 缺文件与坏文件恒全拒；空 allowlist 恒不容许任何窗口；`record`／`clipboard` 未开则该工具恒被拒；不指名窗口的整屏截取恒被拒 |
| platform | 非 Windows 上恒回 `E_TOOL_UNAVAILABLE` 并报出平台名；Windows 上六件工具皆真的落到这台桌面上 |
| windows::target | 名字命中零个窗口恒被拒并指向 `desktop.windows`；命中两个以上恒被拒并列出各自的 title，**恒不**在其中挑一个 |
| windows::views | 快照恒推进该窗口的 generation；对着旧 generation 做的动作恒被拒；快照没铸过的 ref 恒被拒 |
| windows::encode | 三种格式各自解得回原尺寸；`scale` 恒按百分比缩，且缩到 0 像素恒被拒而不是产出空图 |
| windows::keys | 表里每个键名恒映到一个虚拟键码；表外的键名恒被拒并列出可用的键名 |
| windows::record | 同一窗口重复 start 恒被拒；未 start 就 stop 恒被拒；stop 恒交出一条落盘路径 |
| unsafe | 每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是**使它成立的前提**，而不是把这次调用换句话再说一遍 |

## 3 假设与歧义

- **假设**：运行中的机器上的桌面是操作者自己的桌面。本 package 不做远程桌面、不做跨机器、不做无人值守的持续录制。
- **歧义已定**：scope 文件只能表达 allowlist（window title patterns ＋ process names）与 `record`／`clipboard` 两位开关，**没有**「允许整屏」这一项。故**整屏截取在本版恒被拒**（见 §8.5 第三对），而不是被默许——一张全屏图会显示 allowlist 没有列出的一切。

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

### 8-8 Windows 这条胳膊的内部

`platform::perform` 这个自由函数是一张**桌子**：

```rust
// 8-6 platform（形状 4 适配器；cfg 二选一，无 trait）
pub(crate) struct Desk { /* 私有：views／recordings */ }
impl Desk {
    pub(crate) fn new() -> Desk;
    pub(crate) fn perform(&mut self, tool: &str, arguments: &Value) -> Result<Value, Refusal>;
}
```

改成有状态是被两件事逼出来的，不是为了好看：**generation** 要跨调用记住（`desktop.snapshot` 铸、`desktop.act` 核对），**录制**要跨调用记住（`start` 开、`stop` 关）。把它们放进进程级全局，等于给同一份状态开第二道门；放进 `Desk`，`Server` 持有一张桌子，生命周期就是这条连接的生命周期——连接断了，录制随之收摊。`Server::call` 因此从 `&self` 变成 `&mut self`。

十二个文件，切口都落在语义上：

| 模块 | 它拥有什么 | 形状 | 碰 Win32 吗 |
|---|---|---|---|
| `windows` | 一次已获准的调用路由到哪一件；`Desk` 的状态 | 4 适配器 | 否 |
| `windows::fault` | 一次 Win32 失败**怎么变成一句三段式拒词**——全 package 唯一一处 | 4 适配器 | 只读错误码 |
| `windows::geometry` | 矩形、窗口内坐标与屏幕坐标、缩放后的整数尺寸 | 2 值类型 | 否 |
| `windows::enumerate` | `EnumWindows`：这台桌面上有哪些顶层窗口，各自的 title／process／bounds | 4 适配器 | **是** |
| `windows::target` | 从一串窗口里按 title／process 挑出**恰好一个** | 1 判定 | 否 |
| `windows::views` | 一次快照铸了哪些 ref、该窗口现在是第几代、一个动作该不该被这一代接受 | 1 判定 | 否 |
| `windows::tree` | UIA `IUIAutomation` 树：role／name／ref／bounds | 4 适配器 | **是** |
| `windows::keys` | 键名到虚拟键码的那张表 | 6 数据 | 否 |
| `windows::act` | `SendInput`：一个动作落到一个窗口上 | 4 适配器 | **是** |
| `windows::capture` | `PrintWindow`：一个窗口变成一片 BGRA 像素 | 4 适配器 | **是** |
| `windows::encode` | 像素按 `scale` 缩、按 `format` 编码、按 base64 出门 | 1 判定 | 否 |
| `windows::record` | start／stop：PATH 上有 ffmpeg 则 mp4，否则一个 PNG 序列目录 | 4 适配器 | 间接 |
| `windows::clipboard` | 运行中的机器的剪贴板，作为文本 | 4 适配器 | **是** |

**这张表的分法就是 Humble Object**（ARCHITECTURE §9）：难测的那一端（`enumerate`／`tree`／`act`／`capture`／`clipboard`）薄到几乎没有判断，判断都搬进了 `target`／`views`／`encode`／`keys`／`geometry` 五个纯模块——它们一行 Win32 都不跑，因而可以被逐条证明。一台没有桌面的机器上，本 package 仍然能证明「哪个窗口被选中」「过期的动作被拒」「一张图缩成什么尺寸」这四件最容易错的事。

### 8-9 `unsafe` 的那一条规矩

本 package 坐在 workspace 之外，**理由只有一个**：Win32 边界要写 `unsafe`（§8.5 第二对）。既然是花了代价换来的，代价就要花在明处：

**每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是使这次调用成立的前提。**「我们调用 `EnumWindows`」不是前提，那只是把调用换句话再说一遍；「回调是本模块里的 `extern "system" fn`，`lparam` 指向的 `Vec` 在本次调用期间恒存活且无第二个别名」才是前提。审这一条的办法是逐个 `SAFETY:` 问一句：它说的东西**能不能是假的**？不能为假的句子不是前提，是复述。

`unsafe` 恒只出现在 `platform/windows/` 之下，且恒只包住 FFI 调用本身——不包住随后的判断，因为把安全代码收进 `unsafe` 块只会让下一个读者多审几行。

### 8-10 `main` 与 `smoke`：进程的两端

模块表的 `Spec` 列要求每个在册模块指向定义它的那一节。

- **`desktop::main`（形状 4 适配器）**：这个服务器从哪里被启动——一个参数（`DESKTOP.toml` 的路径）、一个环境变量、一对管道。它不判定任何事：作用域归 `scope`，应答归 `session`。
- **`desktop::smoke`（形状 4 适配器；`desktop/tests/smoke.rs`）**：唯一一处真的把二进制拉起来的测试（§16 已记其理由）——从管道灌一次 `initialize` 加一次 `tools/list`，断言六个名字。其余测试只证明库里的判断。

## 8.5 四个设计

**第一对（错误码住哪）**：`use kernel::AxCode`（落选）vs 在本 package 重新定义同拼写的一小组（选中）。前者把这个 package 拉回 workspace 的墙内，而它坐在墙外的**理由**就是要在 Win32 边界上写 `unsafe`；为了六个字符串常量把这个理由作废是本末倒置。选中方案付的代价是同一拼写有两处定义，边界是：本 package 恒只**引用**已有拼写，恒不铸造新的 `E_` 码——新码要先进 `kernel::error::code`。

**第二对（unsafe 怎么关）**：照抄 workspace 的 `unsafe_code = "forbid"`（落选）vs 本 package 用 `deny`（选中）。`forbid` 在文件内无法就地放开，而 Win32 调用点要就地放开、并在那一处写明理由；`deny` 让放开成为**一个带理由的、看得见的、最窄作用域的例外**，而不是把整堵墙推倒。clippy 那张表逐行照抄，一条不减。协议壳**一行 unsafe 也不写**。

**第三对（整屏怎么办）**：默许整屏截取（落选）vs 无表达即拒（选中）。scope 文件能表达的只有「哪些窗口」，一张全屏图会显示 allowlist 没有列出的一切；把没写下来的东西当成允许，正是 fail closed 要防的那件事。拒词里给的替代是「指名一个窗口」，可执行。等 scope 文件长出一位 `screen` 开关，这条再改，改时先改本节。

**第四对（未实现怎么回答）**：先回一个假的成功形状让上游先接线（落选）vs 回 `E_TOOL_UNAVAILABLE` 并说明这个 build 里没有它（选中）。一个假的成功会让模型据此往下推理，而错误的答案比没有答案贵得多；全部价值就是**形状已经定死、拒绝是诚实的**。

## 8.6 五个设计

**第一对（截图怎么取）**：DXGI Desktop Duplication（落选）vs `PrintWindow`（选中）。DXGI 复制的是**整个输出**，而这台 server 的 scope 文件说的是「哪些窗口」；用一个整屏机制去实现一件按窗口授权的事，等于把 §8.5 第三对刚关上的门从背面打开。`PrintWindow` 带 `PW_RENDERFULLCONTENT` 直接向一个 `HWND` 要它自己的像素，授权单位与机制单位因此是同一个。代价写在明处：某些用 DirectComposition 独立合成的窗口会回一片黑，那时的答案是**拒绝并说出来**（`E_TOOL_UNAVAILABLE`，全黑像素是可判的），恒不把一片黑当成截图交出去。

**第二对（快照的 ref 拿什么撑住）**：跨调用持有 `IUIAutomationElement` 这个 COM 指针（落选）vs 只留下快照当时的**屏幕矩形**（选中）。前者让 COM 对象的生存期缠上连接的生存期，而一次 `desktop.act` 需要的其实只有「点哪里」。选中方案让 COM 完整地关在 `tree` 一次调用之内，`act` 只面对整数坐标；generation 这一条也因此有了确切含义——**这一代的 ref 指的是那一刻它在屏幕上的位置**，窗口一动，重新快照，旧的一代作废。

**第三对（`desktop.windows` 报的 ref 是什么）**：让它成为 `snapshot`／`act` 也接受的第二种指名方式（落选）vs 只作为这条连接内一个窗口的**稳定叫法**（选中）。scope 判定读的是 `title` 与 `process`（`Reach`），一个绕过它们的 ref 就是同一份许可的第二道门——而两道门里一定有一道最后没人看。工具表是定死的，`snapshot`／`act` 的 schema 里本来也没有窗口 ref 这一项；本节记下的是**为什么不去加它**。ref 里恒不含 `HWND` 的数值：句柄是运行中的机器的内部事实，模型没有一处用得上它。

**第四对（没有 ffmpeg 时录什么）**：宣告录制不可用（落选）vs 自己抓一列 PNG 帧（选中）。工具卡片明写了「有 ffmpeg 出 mp4，没有则出帧序列」，而帧序列要一个**在读循环之外**跑的东西——本 package 因此有且只有一个 `std::thread::spawn`，就在 `record::start`，由一个 `AtomicBool` 停下，`stop` 恒 join 它。这是本 package 唯一一处并发，写在这里是为了下一个读者不必去找第二处。声音（`audio: true`）恒被拒：选一个录音设备要知道运行中的机器上它叫什么，而这台 server 没有任何一处知道；假装录了而没录，比拒绝贵。

**第五对（错误码的第二份拼写怎么收）**：§8.5 第一对接受了「同一拼写、两处定义」，而这里**收成一处可检查的引用**：`refusal.rs` 里每个 `E_` 码旁写明它引自 `kernel::error::code` 的哪一个，并在城里那一侧加一条测试，逐字比对两张表——测试住在 workspace 内（它可以 `use kernel`），比对的对象是本 package 的 `README.md` 与 SPEC 记下的那六个字符串。**结论是不能靠共享依赖消除这份重复**：让 `desktop` 依赖 `kernel`，就把它拉回墙内，而它坐在墙外的唯一理由是 Win32 要 `unsafe`；六个字符串常量换掉这个理由是本末倒置。能做到的是让漂移**可见**——两处定义，一处权威，一条测试在城里那侧盯着。

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
| `E_TOOL_UNAVAILABLE` | 非 Windows 平台；Windows 但本 build 未实现 | 不能：这是运行中的机器与这个 build 的事实 |

拒词恒是三段（three-part refusal）：拒了什么（action）、为什么（subject）、还能做什么（recovery），装进 JSON-RPC error 的 `data` 里；`message` 是人读的一句摘要。

## 13 依赖选型

`serde`＋`serde_json`＋`toml` 三个，与 workspace 同版本线。**恒不引入**：workspace 内任何 crate（理由见 §8.5 第一对）、async runtime、HTTP 客户端、glob crate（§10 第 4 条）。

三个依赖，各自买到什么写在这里：

| crate | 买到什么 | 为什么不是别的 |
|---|---|---|
| `windows` 0.62 | Win32 与 UI Automation 的绑定，只开六件工具真正够得着的那几个 namespace | `windows-sys` 只有裸函数，而 `IUIAutomation` 是 COM：手写 vtable 会把本该由绑定承担的正确性搬到本 package 里 |
| `image` 0.25（`default-features = false`，只开 `png`／`jpeg`／`webp`） | `desktop.screenshot` 点名的三种编码，以及缩放 | 关掉默认特性是因为本 package 只编码、从不解码，也不碰另外十种格式 |
| `base64` 0.22 | 图片出门的那一层编码 | 与 workspace 的 `gateway::dialect::images` 同一条版本线，两侧读同一种 base64 |

`windows` 的 feature 列表本身就是一份**够得着范围的声明**：一个本 package 从不调用的 API，在这里连名字都拼不出来。

## 14 硬编码声明

`PROTOCOL_VERSION = "2025-06-18"`：与 `protocol::PROTOCOL_VERSION` 同值，理由是两端要谈得拢；它变了，本 package 要在同一次改动里跟着变，故本节是它的第二处台账。服务器自称 `sprawling-desktop`，版本取 `CARGO_PKG_VERSION`。

常数，每条都写清它是谁的事实：

| 常数 | 值 | 谁的事实 |
|---|---|---|
| 快照默认深度 | 8 层 | 我们的选择：再深一层的 UIA 树，模型读到的东西开始多过它用得上的 |
| 一次快照最多铸的 ref 数 | 500 | 我们的选择：一份读不完的树等于没读 |
| 截图默认格式／`scale` | `png`／100 | 我们的选择：默认不损、不缩，缩放是调用方明说才发生的事 |
| `webp` 忽略 `quality` | —— | 外面的事实：`image` 的 WebP 编码器是**无损**的，故 `quality` 对它无意义。schema 允许同时给出，本 server 恒不因此报错，而在答复里写明这一次的编码是无损的 |
| 帧序列的抓帧间隔 | 100 ms（10 fps） | 我们的选择：`PrintWindow` 一帧的代价决定了上限，而 10 fps 足够看清一次交互 |
| 录制落盘的去处 | `std::env::temp_dir()/sprawling-desktop/<窗口名安全化>-<序号>` | 我们的选择：scope 文件说的是「可以碰哪些窗口」，没说「可以往哪写文件」，故恒不写进城里，也恒不写进操作者的家目录 |
| ffmpeg 的收尾 | 向其 stdin 写一个 `q`，再等它自己退出 | 外面的事实：这是 ffmpeg 写完 mp4 尾部索引的办法；直接杀掉会留下一个播放不了的文件 |
| 全黑像素判为失败 | —— | 外面的事实：`PrintWindow` 对某些独立合成的窗口回全黑。依据是「每一个像素的 RGB 三通道皆为 0」 |

## 15 影响面

新增 out-of-tree package，无既有调用方。波及两处：根 `Cargo.toml` 的 `[workspace]` 增一行 `exclude = ["desktop"]`（属 gate machinery，单独提交）；`ARCHITECTURE.md` §12 增一节十一行。城里接上它时只需一栋楼的 `CONFIG.toml` 写一条 `[[mcp]]` ＋ `command`，装配层**零改动**——这正是要验的那一条。

## 15.2 城里那一侧欠的东西

城里要认这台 server，三件事的落点写在这里：

1. **`BUILDING.md` 增一位 `desktop:`**：住在 `city::policy`，与 `confidential:`／`write:`／`review:` 同一处解析。缺省是**关**——一栋楼默认不把桌面交出去，理由与 `record`／`clipboard` 默认关是同一条。
2. **`desktop.` 前缀的 MCP 工具归到「撤不回」那道门**：一次点击没有 restoration，`kernel::discard` 那套「拿得回来才准删」在这里无从谈起，故它该走的是**升给人**（Escalate），不是 Deny。落点是 `runtime::bench::admit` 里 `Effect::Connector` 那一支。
3. **设置页写 `DESKTOP.toml`**：它是**治理文件**，不是产物——它说的是这栋楼的 runs 能碰什么。故它落在这栋楼的 reserved subtree（`<building>/.sprawling/DESKTOP.toml`），与 `BUILDING.md`／`CONFIG.toml` 同处，**任何 write domain 都够不着**；写它的那一点照 `city::governed` 的形状办（一道门、整份写、不拼路径），而不是让设置页自己拼一个路径出来。这就是 `DomainReach` 立下的那条读法：一份决定「residents 能写什么」的文件，恒不由 resident 写。

#### 落到哪一步，还欠什么

前两件已落地并各自有测试：`city::policy` 读 `desktop:`（缺省关、机密楼即拒、打字错误即拒），`kernel::gate::undoable` 判「远端名前缀 `desktop.`」并升给人，`runtime::bench::admit` 在出网门之后叫它。

第三件**只落了服务端那一半**：`city::write_desktop_scope` 与 `city::desktop_scope_path` 已经在，落点与理由如上，并有测试断言它落在 reserved subtree 之内。**还欠三样，且都在这条线之外的地方**：

1. `channels` 上一条命令帧，携「哪一栋楼」与「整份文本」——形状与 `Govern` 那一类同，因为它改的是治理文件。
2. `bin::assembly` 上接这条帧的一格，写之前先上账本一行（每一次效果先成为事件）。
3. 设置页上的那个框。

**它们尚未做**：加一条命令帧会同时动 `channels::command`、`WIRE_V` 的 schema hash 与 `xtask wiring` 认的那张表，而前端此刻是冻结的。第 3 条欠客户端一个整份文本的编辑框加一次保存——没有分段编辑，理由与 `city::governed` 同：写一半会让这台 server 被半行 allowlist 约束。


## 16 测试与约束

逐模块 `#[cfg(test)]`，外加 `tests/smoke.rs`：**真的把二进制拉起来**，从管道里灌一次 `initialize` ＋ 一次 `tools/list`，断言六个名字。它是唯一一处证明「城里那条 `command` 真的能接上」的测试，其余测试都只证明库里的判断。

**约束**：本 card 恒不出现 `unsafe`、`unwrap`、`expect`、`panic!`、`todo!`、裸下标、`as`；算术走 `checked_*`／`saturating_*`；每个文件 ≤400 行、每个函数 ≤200 行且 ≤4 参数。`scope.rs` 因这条尺子而在 446 行处切出 `scope/pattern.rs`——切口落在「一行 allowlist 匹配什么」与「这份文件许可什么」之间，是语义的，不是为了凑行数。

验收命令（在 `desktop/` 内）：`cargo fmt`／`cargo clippy --all-targets -- -D warnings`／`cargo nextest run`。根目录的 `just check` 够不到本 package，因为它不是 workspace member。

### 16.2 怎么测一件需要桌面的事

本 card 的测试分两层，分界线就是 §8-8 那张表的最后一列：

- **不碰 Win32 的五个模块逐条测**（`target`／`views`／`encode`／`keys`／`geometry`）。最容易错的四件事——选中了哪个窗口、过期的动作有没有被拒、一张图缩成什么尺寸、一个键名映到什么——全在这一层，且在**任何**机器上都跑得起来。
- **碰 Win32 的五个模块只测「拒绝是诚实的」**：一个不存在的窗口名恒得到一句指向 `desktop.windows` 的拒词，而不是一次崩溃。CI 里没有一张桌面可供点击，故「点下去真的点中了」这件事**恒不**被写成一条会在没有桌面时假装通过的测试；它由操作者在真机上验，本节记下这是一处**具名的空缺**，不是一处被忽略的覆盖率。

这条分界线是诚实的代价：写一条「在没有窗口时也返回 ok」的测试会比现在好看，但它证明的是这条测试自己，不是这台 server。

## 17 模型体验

工具名恒是 `desktop.<动词>`，模型一眼看得出这一件事发生在桌面上而不是页面里。每条说明的后半句写的是**不做什么**，因为模型下一步最贵的错误是把一件工具当成它旁边那件。拒词恒给一个可执行的下一步：越界给「把这个窗口写进 `DESKTOP.toml`」，未实现给「这个 build 里没有它」。

## 18 文档同步

`ARCHITECTURE.md` §12 新增 desktop 一节｜`desktop/README.md`（英文，讲清它为什么住在 workspace 外）｜同步本 SPEC §13 与 §8-7 的实现状态。

