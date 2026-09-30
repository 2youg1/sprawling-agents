# desktop-SPEC.md

> package：`sprawling-desktop`（out-of-tree，**不是** workspace member；作为库链进 `sprawling`，由 `sprawling desktop` 这个动词起成子进程）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。

## 1 需求分解

一件事：**给 agent 一双眼睛和一双手，落在这台 Windows 桌面上**，并且这双手从第一天起就受一份 allowlist 约束。

拆成三件可独立验收的事：

- **协议壳（`rpc` ＋ `session`）**：一台按行说话的 MCP server，握手与 `tools/call` 的形状与 `crates/agent_protocols/src/mcp/*` 所写的客户端逐字对齐，于是城把它当一台普通的 stdio MCP server 连：一栋楼的 `RULES.toml` 写 `desktop = true`，城就以 `sprawling desktop <DESKTOP.toml>` 起自己这个二进制（sprawling-SPEC §8-4d），之后走的是与任何 `[[mcp]]` 相同的那条路。
- **工具表（`tools`）**：六件工具的名字、说明与入参 schema **定死**。每条说明都写明这件工具**不做**什么。
- **拒绝故事（`scope` ＋ `platform`）**：工具名认出之后的拒绝——越界、未实现、平台做不到——都以 MCP `CallToolResult` 的 `isError` 结果回答，文字里是带稳定错误码的三段拒词；握手未完、方法或工具名不认识、行读不出，这几类协议层的错误才回 JSON-RPC error。没有 scope 文件＝全拒。

第三件事的后半句是**真的实现**，且它换掉的只有 `platform` 一个模块：协议壳与工具表一行不改，这正是把形状定死所买到的东西。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| rpc | 一条消息恒是单行且不含换行；答案恒携原 `id`；读不出的行回 `-32700` 而不是沉默 |
| answer | 做成了的调用恒答 `{ content: [...] }`；截图恒是一块 image content 在前、一块写 title／width／height／lossless 的文字在后；答复里恒不出现 `base64` 字段 |
| refusal | `isError` 结果的文字恒含码、拒了什么、为什么、还能做什么；效果未知的拒绝恒在 `_meta` 带 `"sprawling/effect-unknown": true`，其余拒绝恒不带 `_meta` |
| session | 未握手完成前 `tools/list`／`tools/call` 恒被拒；`notifications/initialized` 恒无答案；`ping` 恒答空对象；未知方法回 `-32601`；`tools/call` 在工具名认出之后的拒绝恒以 `isError` 结果回答，恒不以 JSON-RPC error 回答 |
| tools | 六个名字恒在 `tools/list` 里；每条 description 恒含一句「不做什么」；每张 `inputSchema` 恒是 `type: object` |
| scope | 缺文件与坏文件恒全拒；空 allowlist 恒不容许任何窗口；`record`／`clipboard` 未开则该工具恒被拒；不指名窗口的整屏截取恒被拒；allowlist 没列的窗口恒不出现在 `desktop.windows` 的答复里 |
| platform | 非 Windows 上恒回 `E_TOOL_UNAVAILABLE` 并报出平台名；Windows 上六件工具皆真的落到这台桌面上 |
| windows::target | 名字命中零个窗口恒被拒并指向 `desktop.windows`；命中两个以上恒被拒并列出各自的 title，**恒不**在其中挑一个 |
| windows::views | 快照恒推进这扇窗口（按窗口句柄，不按标题）的 generation；对着旧 generation 做的动作恒被拒；对着快照之后挪动过或改过尺寸的窗口做的动作恒被拒；快照没铸过的 ref 恒被拒 |
| windows::encode | 三种格式各自解得回原尺寸；`scale` 恒按百分比缩，且缩到 0 像素恒被拒而不是产出空图；`scale` 与 `quality` 域外的值**在解析点被拒**，不是钳位也不是静默换默认值 |
| windows::focus | 键盘在别的窗口手里时恒不发事件而回 `E_TOOL_UNAVAILABLE`；指针动作落点被别的窗口盖住时同样恒被拒；两条拒词恒写明「什么都没发出去」 |
| windows::keys | 表里每个键名恒映到一个虚拟键码；表外的键名恒被拒并列出可用的键名 |
| windows::strokes | 一批事件只被收下前 k 个（0 < k < n）时恒报出 k／n 并标效果未知；恒只补发前缀里按下而未抬起的键与鼠标键的抬起，恒不补发按下，恒不重发整批；一个都没收下时恒说「什么都没发出去」 |
| windows::dpi | 连接器答第一次调用之前，本进程恒已按显示器感知 DPI；做不到时每一次调用恒被拒，恒不在两种坐标之间混算 |
| windows::clipboard | 读恒以 `GlobalSize` 为上界；锁不住恒是拒绝而不是「没有文本」；写恒先备好整块内存再清空剪贴板，交不出时恒释放那块内存，拒词恒说明剪贴板已被清空 |
| windows::record | 同一窗口重复 start 恒被拒；未 start 就 stop 恒被拒；每一帧恒取自 `capture::window`；抓帧失败恒让录制停下，`stop` 恒交出落盘路径并说出写了多少帧、为什么提前停 |
| unsafe | 每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是**使它成立、并且可能为假的前提**，而不是把这次调用换句话再说一遍 |

## 3 假设与歧义

- **假设**：运行中的机器上的桌面是操作者自己的桌面。本 package 不做远程桌面、不做跨机器、不做无人值守的持续录制。
- **歧义已定**：scope 文件只能表达 allowlist（window title patterns ＋ process names）与 `record`／`clipboard` 两位开关，**没有**「允许整屏」这一项。故**整屏截取在本版恒被拒**（见 §8.5 第三对），而不是被默许——一张全屏图会显示 allowlist 没有列出的一切。

## 4 现状分析

`crates/agent_protocols` 已经是这套协议的**客户端**权威：`Rpc::initialize`／`initialized`／`list_tools`／`call_tool`／`read` 定死了城里说出去的每一行，`agent_protocols::mcp::stdio` 定死了字节怎么走（子进程、按行、消息内无换行、超时即回收子进程）。本 package 是那一端的**对侧**，因此它的形状不是设计出来的，是**读出来的**。

城这一侧读 `tools/call` 答复的是 `agent_protocols::McpTool`：`isError: true` 的结果是一次失败，拒词全文进它的 subject；`_meta` 带 `sprawling/effect-unknown` 时它标 `Retry::Unknown`（agent_protocols-SPEC §8-1c）。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 握手：`initialize` → `notifications/initialized` → 其它 | <https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle> |
| stdio 传输：子进程、按行、消息内无换行 | <https://modelcontextprotocol.io/specification/2025-06-18/basic/transports> |
| `tools/list`／`tools/call` 的 `name`／`description`／`inputSchema` 三字段 | <https://modelcontextprotocol.io/specification/2025-06-18/server/tools> |
| JSON-RPC 2.0 的保留错误码区间与 `-32000` 起的实现自定义区 | <https://www.jsonrpc.org/specification#error_object> |
| 城里客户端实际发出的行 | `crates/agent_protocols/src/mcp/handshake.rs`、`crates/agent_protocols/src/mcp/tools.rs` |
| 城里字节怎么走、超时怎么算 | `crates/agent_protocols/src/mcp/stdio.rs` |
| `CONFIG.toml` 的 `[[mcp]]` 与 `McpTransport::Stdio { command, args }` | `crates/kernel/src/config.rs` |
| `CallToolResult`、`isError`、内容块；「工具自己的错误放进结果、置 `isError`，不回协议层错误」 | <https://modelcontextprotocol.io/specification/2025-06-18/server/tools>，`schema/2025-06-18/schema.ts` 的 `CallToolResult` |
| `_meta` 键名的格式与保留前缀 | <https://modelcontextprotocol.io/specification/2025-06-18/basic> 的 `_meta` 一节 |
| `GetDIBits`、`ReleaseDC`、`OpenClipboard`、`GetClipboardData`、`SendInput` 的调用前提 | learn.microsoft.com 上各自的函数页（`wingdi/nf-wingdi-getdibits`、`winuser/nf-winuser-releasedc`、`winuser/nf-winuser-openclipboard`、`winuser/nf-winuser-getclipboarddata`、`winuser/nf-winuser-sendinput`） |
| 按显示器感知 DPI 时各 API 给的是物理像素 | <https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-screenscaling> |

## 6 命名统一

沿用词汇表（`docs/glossary.md`）：**three-part refusal**（拒了什么／为什么／还能做什么）、**reference**（快照铸的句柄，形如 `e1`）、**generation**（快照的世代号，随动作一起走）。

- **scope** 在本 package 里专指 `DESKTOP.toml` 里那份 allowlist，与 `kernel` 的 halted scope 不同层，故恒不缩写成裸词 `policy`。
- **恒不**把窗口叫作 page，**恒不**把桌面叫作 browser：`browser::act` 是被借鉴的形状，不是被复用的名字。
- 错误码沿用 `kernel::AxCode` 的**拼写**（`E_TOOL_UNAVAILABLE`／`E_GATE_DENIED`／`E_INVALID_ARGS`／`E_TOOL_UNKNOWN`／`E_CONFIG_INVALID`／`E_WIRE_MISMATCH`），但**不依赖** `kernel`，理由见 §8.5 第一对。同一拼写、两处定义，是本 SPEC 明知并接受的一处重复；`xtask guard` 的 wall 检查本 package 写出的每个 `E_` 都是 kernel 定义过的。
- **效果未知（effect unknown）**：请求已经交出一部分，桌面上是否生效不知道。城里对应 kernel 的 `Retry::Unknown`；本 package 用 `_meta` 的 `sprawling/effect-unknown` 说出它（§12.3）。

## 7 模块边界

- **字节从哪里来**归 `serve_stdio` 的调用方，城里的 `bin::main::desktop`：一个位置参数（`DESKTOP.toml` 的路径）与一对管道。stdin／stdout 的锁与刷新住 `serve_stdio`；`session` 只收一行、出一行。
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
pub(crate) struct Refusal { /* 私有：code／action／subject／recovery／aftermath */ }
enum Aftermath { Known, Unknown }                 // 私有：拒词说的就是桌面上发生的，或者不知道
const EFFECT_META_KEY: &str = "sprawling/effect-unknown"; // 抄自 agent_protocols::mcp::tools，xtask guard 比对两份
impl Refusal {
    pub(crate) fn new(code: RefusalCode, action: &str, subject: impl Into<String>, recovery: &str) -> Refusal; // Aftermath::Known
    pub(crate) fn effect_unknown(self) -> Refusal; // 请求已交出一部分，效果不知道
    // 出口按答法分两个，没有专为测试读字段而开的第三个：
    pub(crate) fn as_error(&self) -> Value;        // 协议层：{ code, message, data: { code, action, subject, recovery } }
    pub(crate) fn as_tool_result(&self) -> Value;  // 工具层：{ content: [{ type: "text", text }], isError: true, _meta? }
}

// 8-2 rpc（形状 4 适配器）
pub(crate) const PROTOCOL_VERSION: &str = "2025-06-18";
pub(crate) struct Request { pub(crate) id: Option<Value>, pub(crate) method: String, pub(crate) params: Value }
pub(crate) fn read(line: &str) -> Result<Request, Refusal>;
pub(crate) fn result_line(id: &Value, result: Value) -> String;
pub(crate) fn error_line(id: Option<&Value>, refusal: &Refusal) -> String;

// 8-2b answer（形状 2 值类型）：一次做成了的 tools/call 怎么答
pub(crate) struct Answer { /* 私有：Vec<Block> */ }
enum Block { Text(String), Image { bytes: Vec<u8>, mime: &'static str } } // 私有
impl Answer {
    pub(crate) fn facts(facts: Value) -> Answer;                                        // 一块文字：facts 的单行 JSON
    pub(crate) fn picture(bytes: Vec<u8>, mime: &'static str, facts: Value) -> Answer;  // 一块图片在前，一块文字在后
    pub(crate) fn as_result(&self) -> Value;      // { content: [...] }；图片的 base64 只在这里编
}

// 8-3 tools（形状 6 数据）
// 六个工具名唯一的家（M-23）：工具表、scope 判定与 platform 路由此前各写一遍字面量。
pub(crate) enum ToolName { Windows, Snapshot, Act, Screenshot, Record, Clipboard }
impl ToolName {
    pub(crate) const ALL: [ToolName; 6];
    pub(crate) const fn as_str(self) -> &'static str;   // `desktop.windows` …
    pub(crate) fn parse(name: &str) -> Option<ToolName>; // 未知名字在这里止步，拒词由 session 写
}
pub(crate) struct ToolCard { pub(crate) name: ToolName, pub(crate) description: String, pub(crate) schema: Value }
pub(crate) fn table() -> Vec<ToolCard>;

// 8-4 scope（形状 1 判定）
pub(crate) struct Reach<'a> { pub(crate) tool: ToolName, pub(crate) title: Option<&'a str>, pub(crate) process: Option<&'a str> }
// 「哪些工具必须指名窗口」由一个对 ToolName 穷尽的 const fn 判定，第七件工具在此处是编译错误。
// Closed 携码：没人写过的文件是没人给过的许可（E_GATE_DENIED），
// 读不出来的文件是这份文件本身有缺陷（E_CONFIG_INVALID）。两件事，两个码。
pub(crate) enum Scope { Closed { code: RefusalCode, because: String }, Open(Allowance) }
pub(crate) struct Allowance { /* 私有：windows／processes／record／clipboard */ }
// 准入是一个值：它带着准入了这次调用的那份 allowlist，
// 于是「答复本身受 scope 约束」的那一件工具读的是同两张 Pattern 表。
pub(crate) struct Admitted<'a> { /* 私有：&Allowance；非 Windows 的非测试构建里没有读者，字段带 expect(dead_code)（§12.1） */ }
impl Admitted<'_> {
    #[cfg(any(windows, test))]   // 只有 Windows 臂报窗口；测试在每个平台上都判它（§16.2）
    pub(crate) fn visible(&self, title: &str, process: &str) -> bool;
}
impl Scope {
    pub(crate) fn read(path: Option<&Path>) -> Scope;       // 恒不失败：缺文件与坏文件都关成 Closed
    pub(crate) fn parse(text: &str) -> Scope;
    pub(crate) fn admits(&self, reach: &Reach<'_>) -> Result<Admitted<'_>, Refusal>;
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
pub(crate) fn perform(tool: ToolName, arguments: &Value, admitted: &Admitted<'_>) -> Result<Answer, Refusal>;
```

### 8-7 六张工具卡片

名字用点号分段（`desktop.act`），城里 `tools_from` 会把它 sanitise 成 `{label}_desktop_act`——分段在两侧都读得出来。

| 名字 | 做什么 | 说明里写明**不做**什么 |
|---|---|---|
| `desktop.windows` | 列顶层窗口：title、process、bounds、ref，**只列 scope 列得出的那些** | 不激活、不移动、不改变任何窗口，不报 allowlist 之外的窗口 |
| `desktop.snapshot` | 一个窗口或整屏的 accessibility tree：role、name、ref、bounds | 不给像素、不给控件的内部句柄、不读被遮挡的内容 |
| `desktop.act` | ref 或 point ＋ 动作（click／double／right／drag／scroll／type／key，带 modifiers），携 snapshot 的 generation；**发事件前先核对前台窗口**；桌面只收下一部分时如实报数，并补发它留下按住的键与鼠标键的抬起 | 不合成整段脚本、不重试、不在 generation 过期或窗口挪动后改打别处、不在别的窗口拿着键盘时把按键发出去 |
| `desktop.screenshot` | window／region，format png\|jpeg\|webp，quality、scale；答一块 image content（`data` 是 base64，`mimeType`）和一块写着 title／width／height／lossless 的文字 | 不做 OCR、不做比对、不落盘 |
| `desktop.record` | start／stop：本 package 唯一那条线程经 `capture::window` 抓帧，PATH 上有 ffmpeg 就经它的 stdin 编成 mp4，否则写成一个 PNG 序列目录；audio 可选 | 不做剪辑、不做转码、不在没说 stop 时自己停（十分钟上限除外） |
| `desktop.clipboard` | get／set 文本 | 不碰图片与文件列表、不保留历史 |

`desktop.act` 携 generation 是照抄 `browser::act` 的那一条：**对着一份快照做的决定，恒不落到另一份快照上**——过期就拒，而不是打到那时挪过去的东西上。

**`drag`／`scroll` 的形状只有一份**：桌面侧与浏览器侧读同一组字段——起点是 `ref` 或 `point`，拖拽终点与滚动增量都用 `to`，`steps` 是中间移动次数。浏览器侧由 `browser::input` 实现（`crates/browser/Spec.lean` 的 D9），桌面侧由这张表实现；两侧的 `Action`／动词名逐字对齐，不会各自演化出一套。

### 8-8 Windows 这条胳膊的内部

`platform::perform` 这个自由函数是一张**桌子**：

```rust
// 8-6 platform（形状 4 适配器；cfg 二选一，无 trait）
pub(crate) struct Desk { /* 私有：views／recordings／pixels */ }
impl Desk {
    pub(crate) fn new() -> Desk;
    // 路由对 ToolName 穷尽，unknown 臂已删：未知名字在 ToolName::parse 止步。
    pub(crate) fn perform(&mut self, tool: ToolName, arguments: &Value, admitted: &Admitted<'_>) -> Result<Answer, Refusal>;
}
```

改成有状态是被两件事逼出来的，不是为了好看：**generation** 要跨调用记住（`desktop.snapshot` 铸、`desktop.act` 核对），**录制**要跨调用记住（`start` 开、`stop` 关）。把它们放进进程级全局，等于给同一份状态开第二道门；放进 `Desk`，`Server` 持有一张桌子，生命周期就是这条连接的生命周期——连接断了，录制随之收摊。`Server::call` 因此从 `&self` 变成 `&mut self`。

十八个文件，切口都落在语义上：

| 模块 | 它拥有什么 | 形状 | 碰 Win32 吗 |
|---|---|---|---|
| `windows` | 一次已获准的调用路由到哪一件；`Desk` 的状态 | 4 适配器 | 否 |
| `windows::fault` | 一次 Win32 失败**怎么变成一句三段式拒词**——全 package 唯一一处 | 4 适配器 | 只读错误码 |
| `windows::reading` | 一次调用的参数各说了什么，每个字段按名拒，不填默认值 | 1 判定 | 否 |
| `windows::geometry` | 矩形、窗口内坐标与屏幕坐标、缩放后的整数尺寸 | 2 值类型 | 否 |
| `windows::enumerate` | `EnumWindows`：这台桌面上有哪些顶层窗口，各自的 title／process／bounds | 4 适配器 | **是** |
| `windows::target` | 从一串窗口里按 title／process 挑出**恰好一个** | 1 判定 | 否 |
| `windows::views` | 一次快照铸了哪些 ref、一扇窗口（按句柄）现在是第几代、快照时它的矩形，以及一个动作该不该被这一代接受 | 1 判定 | 否 |
| `windows::tree` | UIA `IUIAutomation` 树：role／name／ref／bounds | 4 适配器 | **是** |
| `windows::keys` | 键名到虚拟键码的那张表 | 6 数据 | 否 |
| `windows::strokes` | 一个动作是哪几个事件（键、Unicode 单元、指针）；一批只被收下前 k 个时哪些键与鼠标键还按着，以及那句拒词 | 1 判定 | 否 |
| `windows::focus` | 键盘现在在谁手里、一个屏幕点下面是哪个窗口，以及两者都不是它时的那句拒词 | 1 判定 | **是**（两次只读，加一次置前）|
| `windows::act` | `SendInput`：一批事件交给桌面；只收下一部分时补发一次抬起 | 4 适配器 | **是** |
| `windows::capture` | `PrintWindow`：一个窗口变成一片 BGRA 像素；位图解除选择之后才读回 | 4 适配器 | **是** |
| `windows::encode` | 像素按 `scale` 缩、按 `format` 编码、按 base64 出门 | 1 判定 | 否 |
| `windows::dpi` | 本进程按哪种 DPI 感知读桌面：连接器开张时声明按显示器感知，失败时读回判定 | 4 适配器 | **是** |
| `windows::record` | 这条连接正在录哪些窗口、每一份由谁在写 | 4 适配器 | 否 |
| `windows::record::sink` | 一份录制的字节由谁写、落到哪里：本 package 唯一那条线程抓帧，交给 ffmpeg 的 stdin 或写成 PNG 序列 | 4 适配器 | 间接 |
| `windows::clipboard` | 运行中的机器的剪贴板，作为文本 | 4 适配器 | **是** |

**这张表的分法就是 Humble Object**（ARCHITECTURE §9）：难测的那一端（`enumerate`／`tree`／`act`／`capture`／`clipboard`／`dpi`／`focus` 的三次 FFI）薄到几乎没有判断，判断都搬进了 `target`／`views`／`strokes`／`encode`／`keys`／`geometry`／`focus::settled` 七处纯代码——它们一行 Win32 都不跑，因而可以被逐条证明。一台没有桌面的机器上，本 package 仍然能证明「哪个窗口被选中」「过期的动作被拒」「一张图缩成什么尺寸」「键盘不在这个窗口手里时什么都不发」「一批输入被截断时还按着哪些键」这几件最容易错的事。

### 8-9 `unsafe` 的那一条规矩

本 package 坐在 workspace 之外，**理由只有一个**：Win32 边界要写 `unsafe`（§8.5 第二对）。既然是花了代价换来的，代价就要花在明处：

**每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是使这次调用成立的前提。**「我们调用 `EnumWindows`」不是前提，那只是把调用换句话再说一遍；「回调是本模块里的 `extern "system" fn`，`lparam` 指向的 `Vec` 在本次调用期间恒存活且无第二个别名」才是前提。审这一条的办法是逐个 `SAFETY:` 问一句：它说的东西**能不能是假的**？不能为假的句子不是前提，是复述。

`unsafe` 恒只出现在 `platform/windows/` 之下，且恒只包住 FFI 调用本身——不包住随后的判断，因为把安全代码收进 `unsafe` 块只会让下一个读者多审几行。哪些调用今天仍写 `unsafe`，只看 §8-11 那张表「实现」一栏写 `windows`（FFI）的行；本节不另列一份。

几条最容易写成复述的前提，在这里点名。`GetDIBits` 要求位图此刻没有选入任何 DC：`Selected` 守卫离开作用域时把旧对象选回，读回在那之后，所以 SAFETY 行写的是「守卫已经结束」这件可能为假的事。`ReleaseDC` 要传取得 DC 时的那扇窗口，`Surface` 因此存着它。剪贴板的块是别的程序写的，不可信：读以 `GlobalSize` 为界，而不是以「它会以 0 结尾」为前提。

### 8-10 进程的两端住城里

本 package 没有自己的可执行文件，公开面只有 `serve_stdio` 一个函数。

- **进程从哪里起**：`sprawling desktop [scope]`（`crates/sprawling/src/main/desktop.rs`，sprawling-SPEC §8-4d）。一个位置参数是 `DESKTOP.toml` 的路径，缺席即 `Scope::Closed`。它不判定任何事：作用域归 `scope`，应答归 `session`。
- **唯一真的把进程拉起来的测试**也住城里（`crates/sprawling/tests/desktop.rs`）：城按一栋楼的规则起 `sprawling desktop`，握手、list，模型收到六件工具。其余测试只证明库里的判断。

### 8-11 按操作准入的安全接口

Windows 臂的每一次平台调用都落在下表的一行。「实现」一栏是它今天经过的接口；写 `windows`（FFI）的行仍在本 package 里写 `unsafe`，这几行合起来就是 X3（Zig 缝）的输入，别处不另记。「准入」一栏是审过、可以换上的安全接口，写「无」的行理由在 §12.9、§12.10。换一行的次序是固定的：先写它的契约测试，在旧实现上跑绿，证明契约不依赖实现；再换实现，并在同一提交里把这一行的「实现」改成新的接口。

| 操作 | 模块 | 调用 | 实现 | 准入 | 契约测试 |
|---|---|---|---|---|---|
| 枚举顶层窗口，铸出句柄 | `enumerate` | `EnumWindows` 与它的回调；回调里把系统交来的值包成 `winsafe::HWND` | `windows`（FFI） | 无 | `a_window_this_process_opens_is_listed_by_its_title_process_and_bounds` |
| 一扇窗口的事实：可见、标题、进程映像名、外框 | `enumerate` | `IsWindowVisible`、`GetWindowText`、`GetWindowThreadProcessId`、`OpenProcess`＋`QueryFullProcessImageName`、`GetWindowRect` | `winsafe` | `winsafe` | 同上 |
| 前台与落点 | `focus` | `GetForegroundWindow`、`WindowFromPoint`、`GetAncestor`、`SetForegroundWindow` | `winsafe` | `winsafe` | `the_window_under_a_point_is_the_window_drawn_there` |
| 输入 | `act` | `SendInput`、`GetSystemMetrics` | `winsafe` | `winsafe` | `each_stroke_becomes_the_event_it_names` |
| 可访问性树 | `tree` | UIA 的 automation 对象、control view walker、元素属性；COM 公寓 | `windows`（FFI） | `uiautomation`，公寓经 `winsafe` | `a_windows_tree_names_the_control_inside_it` |
| 按窗口捕获 | `capture` | `GetDC`／`ReleaseDC`、`CreateCompatibleDC`、`CreateCompatibleBitmap`、`SelectObject`、`PrintWindow`、`GetDIBits` | `windows`（FFI） | 无 | `the_failing_path_releases_what_it_took` |
| 剪贴板文本 | `clipboard` | owner 窗口、`OpenClipboard`、`GetClipboardData`、`GlobalSize`／`GlobalLock`、`GlobalAlloc`、`EmptyClipboard`、`SetClipboardData` | `windows`（FFI） | 无 | `a_clipboard_block_that_will_not_lock_is_a_refusal_not_an_empty_clipboard` |
| DPI 感知 | `dpi` | `SetProcessDpiAwareness`、`GetProcessDpiAwareness` | `windows`（FFI） | 无 | `a_desk_reads_this_desktop_in_physical_pixels` |

**句柄只在一处铸出。** `winsafe::HWND` 从裸指针构造（`from_ptr`）要写 `unsafe`，反方向（`ptr()` 交给 `windows` 绑定或 `uiautomation`）不要。所以窗口句柄只在 `enumerate` 的 `EnumWindows` 回调里由系统交来的值包成 `winsafe::HWND`，那一处是枚举这一行的一部分；其余模块只借用它，需要 `windows` 或 `uiautomation` 的类型时就地转过去。

**契约测试在真窗口上跑，只碰测试自己建的窗口。** 枚举、落点与树这三行的契约，由测试在本进程里建一扇不抢焦点的窗口（`platform/windows/fixture.rs`，只在测试里编译）再读回它来判；它们不读、不点、不改人桌面上别的窗口。输入这一行的旧实现把事件写进 `INPUT` 联合体，不写 `unsafe` 读不回来，所以它的契约测试随替换一起写，判的是每个 stroke 变成了哪个 `HwKbMouse` 值；真的把事件送到窗口上仍是 §16.2 的操作者检查。

## 8.5 四个设计

**第一对（错误码住哪）**：`use kernel::AxCode`（落选）vs 在本 package 重新定义同拼写的一小组（选中）。依赖边不带来 lint：一个 crate 是否继承 workspace 的 lint 表，只看它自己的清单写没写 `[lints] workspace = true`，本 package 依赖 `kernel` 也照样用自己的 `deny`。落选的理由在锁上：本 package 有自己的 `Cargo.lock`，依赖 `kernel` 就要在这份锁里再解一遍 kernel 的依赖（`thiserror`、`uuid`、`blake3`、`secrecy`、`zeroize`），而 `xtask guard` 只比对两份清单直接声明的依赖，这几个包的版本会在两份锁里各走各的，没有人看。为六个字符串常量付这个价不值。选中方案付的代价是同一拼写两处定义，边界是：本 package 恒只**引用**已有拼写，恒不铸造新的 `E_` 码——新码要先进 `kernel::error::code`。**重开参数**：本 package 并回 workspace，锁只剩一份，这一对作废，直接 `use kernel::AxCode`。

**第二对（unsafe 怎么关）**：照抄 workspace 的 `unsafe_code = "forbid"`（落选）vs 本 package 用 `deny`（选中）。`forbid` 在文件内无法就地放开，而 Win32 调用点要就地放开、并在那一处写明理由；`deny` 让放开成为**一个带理由的、看得见的、最窄作用域的例外**，而不是把整堵墙推倒。clippy 那张表逐行照抄，一条不减。协议壳**一行 unsafe 也不写**。

**第三对（整屏怎么办）**：默许整屏截取（落选）vs 无表达即拒（选中）。scope 文件能表达的只有「哪些窗口」，一张全屏图会显示 allowlist 没有列出的一切；把没写下来的东西当成允许，正是 fail closed 要防的那件事。拒词里给的替代是「指名一个窗口」，可执行。等 scope 文件长出一位 `screen` 开关，这条再改，改时先改本节。

**第四对（未实现怎么回答）**：先回一个假的成功形状让上游先接线（落选）vs 回 `E_TOOL_UNAVAILABLE` 并说明这个 build 里没有它（选中）。一个假的成功会让模型据此往下推理，而错误的答案比没有答案贵得多；全部价值就是**形状已经定死、拒绝是诚实的**。

## 8.6 六个设计

**第一对（截图怎么取）**：DXGI Desktop Duplication（落选）vs `PrintWindow`（选中）。DXGI 复制的是**整个输出**，而这台 server 的 scope 文件说的是「哪些窗口」；用一个整屏机制去实现一件按窗口授权的事，等于把 §8.5 第三对刚关上的门从背面打开。`PrintWindow` 带 `PW_RENDERFULLCONTENT` 直接向一个 `HWND` 要它自己的像素，授权单位与机制单位因此是同一个。代价写在明处：某些用 DirectComposition 独立合成的窗口会回一片黑，那时的答案是**拒绝并说出来**（`E_TOOL_UNAVAILABLE`，全黑像素是可判的），恒不把一片黑当成截图交出去。录制走同一个入口（§12.7）：每一帧都取自 `capture::window`，再经管道交给 ffmpeg，所以「授权单位与机制单位是同一个」这句话对录制也成立。全黑判为失败是一条产品策略，不是对捕获失败的完美识别：一扇真的全黑的窗口也会被拒，拒词给出不经像素的 `desktop.snapshot`。

**第二对（快照的 ref 拿什么撑住）**：跨调用持有 `IUIAutomationElement` 这个 COM 指针（落选）vs 只留下快照当时的**屏幕矩形**（选中）。前者让 COM 对象的生存期缠上连接的生存期，而一次 `desktop.act` 需要的其实只有「点哪里」。选中方案让 COM 完整地关在 `tree` 一次调用之内，`act` 只面对整数坐标；generation 这一条的确切含义见 §12.5：一代 ref 属于一扇窗口（按句柄，不按标题），并且只在这扇窗口的矩形与快照时相同的前提下有效；窗口挪动或改了尺寸，旧的一代被拒。窗口内部重排而外框不动时，这一代仍被接受——这是剩余限制，不是已经解决的事。

**第三对（`desktop.windows` 报的 ref 是什么）**：让它成为 `snapshot`／`act` 也接受的第二种指名方式（落选）vs 只作为这条连接内一个窗口的**稳定叫法**（选中）。scope 判定读的是 `title` 与 `process`（`Reach`），一个绕过它们的 ref 就是同一份许可的第二道门——而两道门里一定有一道最后没人看。工具表是定死的，`snapshot`／`act` 的 schema 里本来也没有窗口 ref 这一项；本节记下的是**为什么不去加它**。ref 里恒不含 `HWND` 的数值：句柄是运行中的机器的内部事实，模型没有一处用得上它。

**第四对（没有 ffmpeg 时录什么）**：宣告录制不可用（落选）vs 自己抓一列 PNG 帧（选中）。工具卡片明写了「有 ffmpeg 出 mp4，没有则出帧序列」，而帧序列要一个**在读循环之外**跑的东西——本 package 因此有且只有一个 `std::thread::spawn`，在 `record::sink`：这条线程抓帧，交给 ffmpeg 的 stdin，或写成 PNG，由一个 `AtomicBool` 停下，`stop` 恒 join 它。这是本 package 唯一一处并发，写在这里是为了下一个读者不必去找第二处。声音（`audio: true`）恒被拒：选一个录音设备要知道运行中的机器上它叫什么，而这台 server 没有任何一处知道；假装录了而没录，比拒绝贵。

**第五对（错误码的第二份拼写怎么收）**：§8.5 第一对接受了「同一拼写、两处定义」，而这里收成一处可检查的引用：`xtask guard` 的 wall 读 `refusal.rs` 里每个 `E_` 字符串，要求它是 `kernel::error::code` 定义过的拼写；`_meta` 的键 `sprawling/effect-unknown` 也由它与 `agent_protocols::mcp::tools` 的那一份比对。集合包含只证明拼写存在，不证明本 package 的每个变体映到了语义对的那个 kernel 码；后一件靠 §12 的码表与评审。结论不变：不能靠共享依赖消除这份重复（§8.5 第一对），能做到的是让漂移**可见**——两处定义，一处权威，一道门在城里那侧盯着。

**第六对（一次 `act` 落在哪个窗口上）**：信任 `scope` 已经判过的那个窗口（落选）vs 发事件之前核对前台窗口，不是它就拒（选中）。`SendInput` 不带窗口：一次按键落在**那一刻**持有键盘的窗口上，而 scope、allowlist 与拒词判的是 title 与 process，这些没有一样跟着事件走。决定动作与发出动作之间隔着一段时间，操作者按一次 Alt+Tab、一个提权对话框弹出来，键盘就在别人手里了——`type` 会把整段文字打进那个窗口，密码框也包括在内；今天全仓 grep `SetForegroundWindow`／`GetForegroundWindow`／`WindowFromPoint` 零命中，故 scope 实际约束住的只有坐标的算法。选中方案是：`windows::focus` 读一次前台窗口，不是它就请求置前并在 200 ms 内有界地重读，仍不是就以 `E_TOOL_UNAVAILABLE` 拒，拒词写明**什么都没有发出去**；指针动作另问第二句——落点下面的顶层窗口也得是它，因为一个窗口可以持有键盘而另一个盖在点击处。比较的是句柄地址这一个纯值，于是这条规则在一台没有桌面的机器上也能逐条证明（§16.2）。付的代价写在明处：置前是一次**副作用**，而它是这台 server 唯一一处主动改变桌面的排布；把它藏起来的做法是不置前直接拒，那会让每一次正常的连续操作都要操作者手动切窗口。

## 9 工作流程

进程起来 → `sprawling desktop` 取 scope 路径（第一个位置参数，否则无）→ `serve_stdio` → `Scope::read`（缺文件即 `Closed`）→ `Server::serve` 阻塞读 stdin。

每收到一行：`rpc::read` → 按 method 分派 → `initialize` 答能力与自我介绍并进 `Initializing` → `notifications/initialized` 无答案并进 `Ready` → `ping` 答 `{}` → `tools/list` 答六张卡片 → `tools/call` 先 `Scope::admits`（过了交出一个 `Admitted`），再把它随参数一起交给 `platform::perform` → 出一行 → flush。

## 10 实现逻辑

1. **一条消息一行**：`serde_json::to_string` 不产生换行，且写出前不做美化；这与 `agent_protocols::mcp::stdio::Connection::framed` 的检查是同一条契约的两端。
2. **握手顺序被强制**：`Fresh` 只答 `initialize` 与 `ping`，`Initializing` 收到通知才进 `Ready`。规范就是这么写的，而一个不强制它的 server 会让客户端的顺序错误在别处以别的形状爆出来。
3. **`id` 原样回**：不解析、不重编号——`id` 是对侧的东西。通知（无 `id`）恒无答案，否则会在管道里留下一行，此后每次调用读到的都是上一条的答案。
4. **glob 只认 `*`**：title pattern 是给人写的，一整套正则会让「我到底放开了什么」变成一个需要推演的问题。匹配按 ASCII 大小写不敏感，因为 Windows 的进程名就是这样比的。
5. **越界判定在平台之前**：`admits` 是纯判定，无 I/O、无时钟，于是它可以被逐条测，而 Win32 一行都还没跑。
6. **glob 一定终止**：回溯时文本下标只增不减，且到达文本末尾即失败，故循环恒有界；`scope::pattern` 里有一条专门打这一点的测试（一个「几乎匹配很多次」的名字）。
7. **`deny_unknown_fields`**：拼错的键若被默默忽略，写它的人会把它读成一个生效了的键。故坏键＝坏文件＝全拒。
8. **GDI 的三条前提写进类型**：`PrintWindow` 之前把位图选进 memory DC 的是一个 `Selected` 守卫，守卫结束时选回旧对象，`GetDIBits` 在守卫结束之后才调；`Surface` 存着取得 DC 的那扇窗口，`Drop` 以它调 `ReleaseDC`；`GetDIBits` 读回的行数不等于高就拒，不交出半张图。空句柄在 `GetDC` 之前就拒，因为 `GetDC(NULL)` 取的是整屏。
9. **两个常量取自绑定**：滚轮一格的 `WHEEL_DELTA` 与 `PW_RENDERFULLCONTENT` 用 `windows` 绑定里的定义，本 package 不再写第二份数值。

## 11 边界枚举

非 JSON 行／非对象／无 `method`／`params` 非对象／未知方法／握手未完成就调用／`tools/call` 无 `name`／`name` 不在表里／scope 文件缺失／scope 文件语法坏／scope 文件有拼错的键／allowlist 两张表皆空／title 不匹配／process 不匹配／同时给 title 与 process 而只中一个／既不给 title 也不给 process／`record` 未开／`clipboard` 未开／整屏截取／非 Windows 平台／Windows 但本 build 未实现／键盘在别的窗口手里且置前请求没有生效／指针动作的落点被别的窗口盖住。一批输入只被收下一部分／补发的抬起也被挡／窗口快照后挪动或改了尺寸／两扇同名窗口／进程已被设成别的 DPI 感知／剪贴板块没有终止符／剪贴板块锁不住／剪贴板清空之后交不出新文本／ffmpeg 起不来或中途退出／录制中窗口不再能抓帧。

**同时给两个标识则两个都要中**：任何一个给出的标识都要落在它自己那张表里，这是 fail closed 的一致读法。

## 12 Decisions

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_WIRE_MISMATCH` | 行不是 JSON、不是对象、无 `method` | 不能：对侧发什么不由本 package 决定，fail closed |
| `E_TOOL_UNKNOWN` | 未知方法、`name` 不在六张卡片里 | 不能：模型会试不存在的名字 |
| `E_INVALID_ARGS` | `params` 形状读不出、`tools/call` 无 `name` | 部分能：schema 已给出，拒词指到那一处 |
| `E_GATE_DENIED` | 越界、`record`／`clipboard` 未开、整屏截取、握手未完成 | **能**（对 scope 而言）：缺文件即 `Closed`，于是「默许」这件事不成立 |
| `E_CONFIG_INVALID` | scope 文件语法坏 | 不能：文件是人写的。坏文件恒关成全拒，恒不退回默认允许 |
| `E_TOOL_UNAVAILABLE` | 非 Windows 平台；Windows 但本 build 未实现；桌面不收输入、只收下一部分（后者效果未知）；DPI 感知声明不成 | 不能：这是运行中的机器与这个 build 的事实 |

拒词恒是三段（three-part refusal）：拒了什么（action）、为什么（subject）、还能做什么（recovery）。工具名认出之后的拒绝写进 `isError` 结果的文字：第一行 `E_…: cannot <action> — <subject>`，第二行 `instead: <recovery>`；协议层的拒绝装进 JSON-RPC error 的 `data`，`message` 是第一行那句摘要。

### 12.1 定规：`visible` 只在 Windows 与测试里编译

- **决定**：`Admitted::visible` 与它所读的 `Allowance::visible` 带 `#[cfg(any(windows, test))]`；`Admitted` 的 `allowance` 字段带 `#[cfg_attr(not(any(windows, test)), expect(dead_code, reason = "…"))]`。类型与唯一的构造点 `Admitted { allowance: self }` 在每个平台上相同。
- **理由**：非 Windows 臂（`platform/elsewhere.rs`）拒绝一切调用，不报任何窗口，所以那里的非测试构建里这两个方法与这个字段没有读者，编译器报三条 `dead_code`。`visible` 是纯判定、不碰 Win32，§16.2 要它在任何机器上受测，所以条件里带 `test`。字段上用 `expect` 而不是 `allow`：哪天非 Windows 臂开始报窗口，字段有了读者，这条 `expect` 落空，构建变红，它就跟着被删掉。
- **击败的备选**：①按平台把字段拼成 `&'a Allowance` 与 `PhantomData<&'a Allowance>`——不需要任何 lint 抑制，但同一个值有两种拼写，构造点也要跟着分叉，而「是不是 Windows」按 `platform.rs` 的约定只写在一处；②等本 package 并入 workspace 时再处理——并入之后，workspace 的 `-D warnings` 会在 macOS 上把这三条警告变成错误。
- **重开的参数**：非 Windows 臂开始报告窗口。那时 `visible` 在每个平台上都有读者，本条与那条 `expect` 一起删除。

### 12.2 工具自己的拒绝以 `isError` 结果回答

- **决定**：`tools/call` 在工具名认出之后的一切拒绝——scope 不准、参数读不了、平台做不到——都答 `CallToolResult`，`isError: true`，`content` 是一块文字，写三段拒词。握手未完、方法或工具名不认识、行读不出、`params` 不是对象，这几类才回 JSON-RPC error。
- **理由**：MCP 2025-06-18 规定工具自己的错误放进结果、置 `isError`，协议层错误留给协议本身。城的客户端只读 JSON-RPC error 的 `code` 与 `message`，拒词里的恢复句到不了模型；放进结果，全文作为文字到达。
- **击败的备选**：继续回 JSON-RPC error，让城读 `data.code`、`data.recovery`：那是规格外的约定，城要为每一家 server 猜一次 `data` 的形状。
- **重开的参数**：MCP 的新修订改了工具错误的答法。

### 12.3 效果未知写在 `_meta` 的一个命名空间键里

- **决定**：一次拒绝若发生在请求已交出一部分之后（今天只有部分输入，§12.4），`isError` 结果带 `_meta: { "sprawling/effect-unknown": true }`。键的唯一定义在 `agent_protocols::mcp::tools::EFFECT_META_KEY`，本 package 在 `refusal.rs` 抄一份，`xtask guard` 比对。
- **理由**：「效果未知」要落成城里的 `Retry::Unknown`，它决定模型与人会不会把一次可能已经点下去的动作再做一遍；写进文字，城只能把它当一句话，读不出这个三态。`_meta` 是规格留给实现附加元数据的位置，键名按规格带自己的前缀，不占 `mcp` 的保留前缀。
- **击败的备选**：放进 JSON-RPC error 的 `data.retry`（与 §12.2 冲突，也是规格外）；只写进拒词文字（城标不出 `Retry::Unknown`）。
- **重开的参数**：MCP 规格为「副作用未知」定了自己的字段；或本 package 并回 workspace，键的拼写能直接引用。

### 12.4 部分输入：如实报数，只补抬起

- **决定**：`SendInput` 只收下前 k 个事件（0 < k < n）时，按已收下的前缀算出仍按住的键、Unicode 单元与鼠标键，按按下的相反次序补发一次它们的抬起；从不补发按下，从不重发整批。答复是 `E_TOOL_UNAVAILABLE` 的拒绝，标效果未知，写出 k／n 以及补发的抬起是否全部被收下。k = 0 时什么都没发出去，拒词照实说，不标效果未知。
- **理由**：微软只承诺按序插入、返回插入的个数，不承诺回滚；前缀可能已经落在窗口上，修饰键可能停在按下状态，人的桌面随后每一次按键都会带着 Ctrl。补发抬起只会松开本次按下的东西，不会让动作多做一步；重发整批会把已经落地的点击或文字再做一遍。
- **击败的备选**：只报告、不补发（桌面可能停在 Ctrl 按下，人要自己找原因）；重发整批（重复点击、重复输入）。
- **重开的参数**：换成能按元素投递的接口（UIA patterns）之后，部分投递不再是这个形状。

### 12.5 generation 按窗口身份与快照时的矩形失效

- **决定**：`Views` 以窗口句柄（`focus::Aim`）为键，不以标题为键；一份快照记下这扇窗口当时的外框矩形。`resolve` 同时比对句柄、generation 与当前外框，外框不同即按过期拒绝。
- **理由**：按标题记，两个进程的同名窗口共用一份记录，对 A 做的快照会让对 B 的动作取 A 的坐标；窗口挪动而不重新快照时，ref 的屏幕坐标指向挪动前的位置。外框是本 package 每次调用本来就读的值，比对它不多一次 Win32 调用。
- **剩余限制**（写明，不当作已解决）：窗口内部重排而外框不动时，旧的一代仍被接受；句柄在窗口关闭后可能被新窗口复用，新窗口恰好外框相同时同样检测不到。
- **击败的备选**：持有 UIA 元素跨调用（§8.6 第二对）；每次动作前重读整棵树比对（一次动作的代价变成一次快照）。
- **重开的参数**：换成按元素投递（UIA patterns），ref 改指元素而不是矩形。

### 12.6 坐标约定：虚拟桌面上的物理像素

- **决定**：连接器建 `Desk` 时以 `SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE)` 声明按显示器感知；声明失败时读回 `GetProcessDpiAwareness`，已是按显示器感知就接受，不是就让每一次调用以 `E_TOOL_UNAVAILABLE` 被拒。此后枚举的外框、UIA 的矩形、落点检查、截图尺寸与输入的归一化都是虚拟桌面上的物理像素。
- **理由**：不声明时，缩放显示器上 `GetWindowRect` 与 `GetSystemMetrics` 给的是虚拟化后的坐标，UIA 给的是物理坐标，一次按 ref 的点击会落在别处。声明只作用于 `sprawling desktop` 这个子进程，城的进程不受影响。用 Windows 8.1 起就有的 shcore 接口而不是 1703 起的 per-monitor v2 context 接口：本 package 链在 `sprawling` 里，一个缺失的 user32 导出会让整个二进制在更老的 Windows 上起不来；本 server 不画窗口，v2 多出的子窗口与非客户区缩放用不上。
- **击败的备选**：在 `sprawling.exe` 的 manifest 里声明（城的进程也被改，本 package 自己的测试进程享受不到）；不声明（两种坐标混算）。
- **重开的参数**：本 package 开始画自己的窗口，或最低支持的 Windows 版本升到 1703 以上。

### 12.7 录制经同一个捕获入口

- **决定**：录制的唯一那条线程按 `FRAME_EVERY` 调 `capture::window` 抓帧。PATH 上有 ffmpeg 时，帧以 `-f rawvideo -pixel_format rgba -video_size <宽>x<高> -framerate 10 -i -` 从 stdin 交给它；`spawn` 报「找不到程序」时写 PNG 序列；`spawn` 报别的错时 `start` 被拒。抓帧失败、写帧失败或 ffmpeg 提前退出，录制停下，`stop` 照常交出路径并说出写了多少帧、为什么停。结束 ffmpeg 靠关掉它的 stdin，它读到结尾自己写完索引退出。
- **理由**：按标题交给 gdigrab 时，ffmpeg 以 `FindWindowW(NULL, title)` 重新找窗口，不看进程，像素来自 `BitBlt` 而不是 `PrintWindow`，也不过全黑判定；一个按 title 加 process 选中的窗口可能在这里换成另一扇同名窗口。
- **击败的备选**：gdigrab 的 `hwnd=`（要 FFmpeg 7.0 以上，像素仍来自 `BitBlt`，版本过老时本 package 还探测不出来）；保持按标题（与 §8.6 第一对矛盾）。
- **重开的参数**：`PrintWindow` 每帧的代价让 10 fps 做不到，届时再比较按句柄的 gdigrab。

### 12.8 剪贴板：自己的 owner，有界的读，先备好再清空

- **决定**：每次打开剪贴板时建一扇仅消息窗口（预定义类 `STATIC`，父窗口 `HWND_MESSAGE`）作为 owner，交给 `OpenClipboard`；`Held` 结束时先关剪贴板，再销毁这扇窗口，最后放掉进程内的轮次锁。读：以 `GlobalSize` 求出块的上界，在界内找终止符，找不到就取整块；`GlobalLock` 失败是拒绝。写：先分配并填好整块，再打开、清空、交出；交出失败时本进程释放这块内存，拒词说明剪贴板已被清空。
- **理由**：微软写明以空窗口打开时 `EmptyClipboard` 把 owner 置空，随后的 `SetClipboardData` 会失败；块是别的程序写的，「它会以 0 结尾」可能为假；把锁失败当成「没有文本」，调用方会去试别的而不是重试；先清空再分配，分配失败时剪贴板已空，拒词却只说内存不够。这扇窗口只活一个剪贴板轮次、在同一线程建和毁，本 package 不依赖它收到任何消息，所以不需要消息泵。
- **击败的备选**：`OpenClipboard(None)`（与文档冲突）；遇到无终止符的块就拒（对一个格式不规范的程序，整块文字仍然可读，界已经由 `GlobalSize` 守住）；WinRT Clipboard（要求前台与 UI 线程，后台子进程是否适用未证）。
- **重开的参数**：出现一个能证明 owner、界与所有权移交的安全封装。X2 审过的 `winsafe` 0.0.29 还不是，理由见 §12.9。

### 12.9 输入与窗口事实经 `winsafe`；枚举、捕获、剪贴板、DPI 仍走 FFI

- **决定**：输入（`SendInput`、`GetSystemMetrics`）与窗口事实（可见、标题、进程映像名、外框、前台、落点、置前）改经 `winsafe` 0.0.29 的安全接口，只开 `user` 与 `ole` 两个 feature。键码以 `co::VK` 常量写在 `keys` 表里，一个动作的事件在 Rust 里构造成 `HwKbMouse` 值，整批一次交给 `winsafe::SendInput`，它回的收下个数照 §12.4 读。窗口句柄只在 `enumerate` 的 `EnumWindows` 回调里铸成 `winsafe::HWND`（§8-11），那是这一组唯一新写的 `unsafe`。
- **理由**：这几个调用的前提（出参缓冲的长度、句柄的借用、`INPUT` 数组的元素大小）由 `winsafe` 的签名承担，本 package 只剩值；`winsafe` 没有 Cargo 依赖，`user` 不拉 GUI 那一层。
- **不准入的四组与各自的理由**：`winsafe::EnumWindows` 把 `&func` 当作地址交出去，回调里再从它造出 `&mut F`（`src/user/funcs.rs` 与 `src/user/callbacks.rs`），这是从共享引用造可变引用，属未定义行为，所以枚举留在 `windows` 绑定上；以 `FindWindowEx` 或 `GetWindow` 逐个取顶层窗口可以不写 `unsafe`，但 z 序在两次调用之间变化时会漏掉窗口或兜圈，而 `EnumWindows` 先取快照再回调，故落选。剪贴板：`HCLIPBOARD::SetClipboardData` 在调用方已经清空剪贴板之后才分配内存，交出失败时那块内存被 `leak` 而不释放，与 §12.8「先备好再清空、交不出就释放」相反；建 owner 窗口的 `CreateWindowEx` 在 `winsafe` 里本身是 `unsafe`。捕获：`winsafe` 没有 `PrintWindow`，`GetDIBits` 是 `unsafe`。DPI：`SetProcessDpiAwareness` 在 shcore 里，`winsafe` 没有它。这四组是 X3 的输入（§8-11）。
- **剩余限制**（写明，不当作已解决）：`SendInput` 仍只报收下几个；换接口不改变 §8.6 第六对的前台检查只是一次采样这件事。
- **击败的备选**：把这几组一起搬进 Zig 缝（有合格安全接口的调用不需要一道新的 FFI 缝）；留在 `windows` 绑定上继续手写 `unsafe`（口径 ①要的正是业务代码零 `unsafe`）。
- **重开的参数**：`winsafe` 修好 `EnumWindows`，枚举这一行随之换过去；或者新版本改了这里准入的签名，那一行的契约测试先红。

### 12.10 树经 `uiautomation`，COM 公寓每张桌子进一次

- **决定**：`tree` 经 `uiautomation` 0.25.1（关默认 feature）读树，只用 `UIAutomation::new_direct`、`element_from_handle`、control view walker 与元素的三项属性（role、name、外框）。COM 公寓由 `Desk` 在第一次 snapshot 时进入一次，用多线程公寓（MTA），经 `winsafe::CoInitializeEx` 的守卫；automation 对象与 walker 同这个守卫一起住在桌子里，字段的析构次序保证先放 COM 对象、后退出公寓。元素仍只活在一次 snapshot 之内（§8.6 第二对）。walker 答「没有这个元素」（错误码 0）是这一层到头；答别的错误是 provider 出了故障，遍历在那里停下，而不是当作到头。
- **理由**：微软对不开窗口的 UIA 工作线程推荐 MTA；进一次、配对退出，公寓的生存期就是连接的生存期。旧写法每次 snapshot 都以 STA 进入而从不退出，并且每次新建 automation 对象；`uiautomation::UIAutomation::new()` 同样每次进入而不退出，故不用它。把 provider 的故障读成「到头了」，模型拿到的是一棵看起来完整、其实缺了一块的树。
- **剩余限制**（写明，不当作已解决）：`winsafe` 的 `CoUninitializeGuard` 在进入公寓答 `RPC_E_CHANGED_MODE` 时也会调 `CoUninitialize`；本 package 的读循环线程不进入任何别的公寓，这条路走不到。剪贴板走的是 Win32 剪贴板而不是 OLE 剪贴板，同一线程上的 MTA 与它无关。
- **击败的备选**：保留经 `windows` 绑定手写的 COM 调用（九个 `unsafe` 块）；让 UIA 也回答窗口事实（UIA 根的子元素、`IsOffscreen` 与 `EnumWindows`、`WS_VISIBLE` 不是同一个定义，安全判断会跟着 provider 的实现走）。
- **重开的参数**：需要 UIA patterns（按元素投递动作）时，那是另一项能力，另写一节；或 `uiautomation` 的新版本改了这里用到的签名，`a_windows_tree_names_the_control_inside_it` 先红。

## 13 依赖选型

八个依赖。`serde`、`serde_json`、`toml` 三个与 workspace 同版本线，只做协议与 scope 文件的读写；另外五个各自买到什么，写在下表，后三个只在 Windows 上链接。**恒不引入**：workspace 内任何 crate（理由见 §8.5 第一对）、async runtime、HTTP 客户端、glob crate（§10 第 4 条）。

| crate | 买到什么 | 为什么不是别的 |
|---|---|---|
| `windows` 0.62 | 还没有合格安全接口的那几组 Win32 调用（枚举、捕获、剪贴板、DPI，§8-11）与绑定里的常量 | `windows-sys` 只有裸函数；`uiautomation` 本身也链接同一版 `windows`，锁里不多一个包 |
| `winsafe` 0.0.29（只开 `user`、`ole`） | 输入与窗口事实的安全接口，以及 COM 公寓的守卫（§12.9、§12.10） | 没有 Cargo 依赖；它的 `EnumWindows` 与剪贴板写法不准入，理由在 §12.9 |
| `uiautomation` 0.25.1（`default-features = false`） | UIA 树的安全封装：automation 对象、walker、元素属性（§12.10） | 关掉默认特性，于是它的输入、截图、剪贴板与控件匹配都不进来；手写 COM 调用是被它换掉的那九个 `unsafe` 块 |
| `image` 0.25（`default-features = false`，只开 `png`／`jpeg`／`webp`） | `desktop.screenshot` 点名的三种编码，以及缩放 | 关掉默认特性是因为本 package 只编码、从不解码，也不碰另外十种格式 |
| `base64` 0.23 | image content 的 `data` 那一层编码，只在 `answer` 里编 | 与 workspace 的 `gateway::dialect::images` 同一条版本线，`xtask guard` 比对版本 |

`windows` 与 `winsafe` 的 feature 列表本身就是一份**够得着范围的声明**：一个本 package 从不调用的 API，在这里连名字都拼不出来。

## 14 硬编码声明

`PROTOCOL_VERSION = "2025-06-18"`：与 `agent_protocols::PROTOCOL_VERSION` 同值，理由是两端要谈得拢；它变了，本 package 要在同一次改动里跟着变，故本节是它的第二处台账。服务器自称 `sprawling-desktop`，版本取 `CARGO_PKG_VERSION`。

常数，每条都写清它是谁的事实：

| 常数 | 值 | 谁的事实 |
|---|---|---|
| 快照默认深度 | 8 层 | 我们的选择：再深一层的 UIA 树，模型读到的东西开始多过它用得上的 |
| 一次快照最多铸的 ref 数 | 500 | 我们的选择：一份读不完的树等于没读 |
| 截图默认格式／`scale` | `png`／100 | 我们的选择：默认不损、不缩，缩放是调用方明说才发生的事 |
| `quality` 的域 | 0..=100 | 与城里同一域（`kernel::consts_policy::IMAGE_QUALITY`）；本包在 workspace 之外读不到它，所以两个数由 `xtask guard` 的墙对账（`wall::quality_domain`）。域外即拒：一个要求 120 的调用方以为自己要多好就有多好，而替它填默认值是在回答另一个问题 |
| `scale` 与城里那个同名量的区别 | 本包的 `scale` 是窗口自身像素的百分数 | 城里 `browser` 的 `scale` 是设备像素比（devicePixelRatio）的百分数——同一个词、两个量，**不是同一个事实**，所以两边的域也不必相同 |
| `webp` 忽略 `quality` | —— | 外面的事实：`image` 的 WebP 编码器是**无损**的，故 `quality` 对它无意义。schema 允许同时给出，本 server 恒不因此报错，而在答复里写明这一次的编码是无损的 |
| 帧序列的抓帧间隔 | 100 ms（10 fps） | 我们的选择：`PrintWindow` 一帧的代价决定了上限，而 10 fps 足够看清一次交互 |
| 录制落盘的去处 | `std::env::temp_dir()/sprawling-desktop/<窗口名安全化>-<序号>` | 我们的选择：scope 文件说的是「可以碰哪些窗口」，没说「可以往哪写文件」，故恒不写进城里，也恒不写进操作者的家目录 |
| ffmpeg 的收尾 | 关掉它的 stdin，再等它自己退出 | 外面的事实：帧从 stdin 进，读到结尾时 ffmpeg 写完 mp4 的尾部索引；直接杀掉会留下一个播放不了的文件 |
| 全黑像素判为失败 | —— | 外面的事实：`PrintWindow` 对某些独立合成的窗口回全黑。依据是「每一个像素的 RGB 三通道皆为 0」 |
| 效果未知的 `_meta` 键 | `sprawling/effect-unknown`，值 `true` | 我们的约定；唯一定义在 `agent_protocols::mcp::tools::EFFECT_META_KEY`，本 package 的抄本由 `xtask guard` 比对 |
| 工具层拒词的文字 | 第一行 `E_…: cannot <action> — <subject>`，第二行 `instead: <recovery>` | 我们的选择：第一行与 JSON-RPC error 的 `message` 同一句，第二行让恢复句在模型读到的文字里有自己的位置 |
| DPI 感知 | 按显示器感知（`PROCESS_PER_MONITOR_DPI_AWARE`） | 外面的事实：Windows 8.1 起的 shcore 接口；理由见 §12.6 |

## 15 影响面

out-of-tree package，唯一的调用方是 `sprawling`：根 `Cargo.toml` 的 `[workspace]` 有一行 `exclude = ["desktop"]`，`crates/sprawling/Cargo.toml` 按路径依赖它，`ARCHITECTURE.md` §3 的 depmap 块有它一行。改 `serve_stdio` 的签名波及 `bin::main::desktop`；改工具名或 `tools/list` 的形状波及 `kernel::gate::undoable`（按远端名前缀 `desktop.` 判）与 `crates/sprawling/tests/desktop.rs`。

## 15.2 城里那一侧

城里认这台 server 的四件事与各自的落点：

1. **`RULES.toml` 增一键 `desktop`**：住在 `city::policy`，与 `confidential`／`write`／`review` 同一处解析。缺省是**关**——一栋楼默认不把桌面交出去，理由与 `record`／`clipboard` 默认关是同一条。
2. **`desktop.` 前缀的 MCP 工具归到「撤不回」那道门**：一次点击没有 restoration，`kernel::discard` 那套「拿得回来才准删」在这里无从谈起，故它该走的是**升给人**（Escalate），不是 Deny。落点是 `runtime::bench::admit` 里 `Effect::Connector` 那一支。
3. **设置页写 `DESKTOP.toml`**：它是**治理文件**，不是产物——它说的是这栋楼的 runs 能碰什么。故它落在这栋楼的 reserved subtree（`<building>/.sprawling/DESKTOP.toml`），与 `RULES.toml`／`CONFIG.toml` 同处，**任何 write domain 都够不着**；写它的那一点照 `city::governed` 的形状办（一道门、整份写、不拼路径），而不是让设置页自己拼一个路径出来。这就是 `DomainReach` 立下的那条读法：一份决定「residents 能写什么」的文件，恒不由 resident 写。

4. **城自己起这台 server**：`RULES.toml` 写 `desktop = true` 的楼，城以 `sprawling desktop <DESKTOP.toml>` 起它，不要人手写 `[[mcp]]`（sprawling-SPEC §8-4d）。

#### 落到哪一步，还欠什么

四件都已落地并各自有测试：`city::policy` 读 `desktop:`（缺省关、机密楼即拒、打字错误即拒）；`kernel::gate::undoable` 判「远端名前缀 `desktop.`」并升给人，`runtime::bench::admit` 在出网门之后叫它；设置页的框经 `ConfigureBuilding` 的 `desktop` 字段整份写 `DESKTOP.toml`，写之前先落账本一行（sprawling-SPEC 里桌面 allowlist 那一节）；城按规则起 `sprawling desktop`。

**不内置任何模型（定规）**：本二进制不带 OCR、ASR 或任何视觉模型的权重，这些都经人接入的端点。模型拿到的桌面文字先是 accessibility tree；OCR 端点与本地 ASR 端点都是人接进来的。

**还欠的**，都是这条接口的当前状态：

1. **给模型的文字反馈**：`desktop.snapshot` 回的是 role／name／ref／bounds 的树，还没有像 `browser::snapshot` 那样把一扇窗口折成模型读得快的纯文本。
2. **OCR**：一张截图经人接入的 OCR 端点变成文字，给 accessibility tree 读不到字的窗口（画在画布上的界面、远程桌面）。端点的形状与 `gateway` 里的哪一面还没有定。
3. **ASR**：`gateway` 的 `transcribe`（OpenAI 兼容的 `audio/transcriptions`）已经在；桌面这一侧还没有把录下的声音交给它，`desktop.record` 的 `audio: true` 仍被拒。
4. **macOS 这条胳膊**：`platform/elsewhere.rs` 对 macOS 答 `E_TOOL_UNAVAILABLE`。它要在一台 Mac 或夜间的 `platforms.yml` 上验，Windows 上验不了。


## 16 测试与约束

逐模块 `#[cfg(test)]`。真的把进程拉起来的那一条住城里（`crates/sprawling/tests/desktop.rs`，§8-10）：它是唯一一处证明「城起的 `sprawling desktop` 真的接得上」的测试，其余测试都只证明库里的判断。

**约束**：`unsafe` 只在 `platform/windows/` 之下且每块携一行 `SAFETY:`（§8-9）；其余处恒不出现 `unsafe`、`unwrap`、`expect`、`panic!`、`todo!`、裸下标、`as`；算术走 `checked_*`／`saturating_*`；每个文件 ≤400 行、每个函数 ≤200 行且 ≤4 参数。`scope.rs` 因这条尺子而在 446 行处切出 `scope/pattern.rs`——切口落在「一行 allowlist 匹配什么」与「这份文件许可什么」之间，是语义的，不是为了凑行数；`Admitted` 落地时它再次抵线，这一次切出的是 `scope/tests.rs`（形状同 `session/tests.rs`），判定与对判定的断言各占一个文件。

验收命令（在 `desktop/` 内）：`cargo fmt`／`cargo clippy --all-targets -- -D warnings`／`cargo nextest run`；根目录一条 `just check-desktop` 把这三条加上 `cargo deny check` 一起跑，并挂在 `just check` 的依赖链上——`cargo clippy --workspace` 够不到本 package，因为它不是 workspace member。**两道工作区的闸门从外面伸进来**：`xtask length` 把 400 行的文件尺子量到 `desktop/src`（它读文件而不是读 crate，故够得着 `cargo` 够不着的地方），`xtask guard` 的 `wall` 把本 package 的 lint 表、包元数据与共享依赖版本逐键比回根 `Cargo.toml`，例外只有记下理由的那两条（§8.5 第二对，以及本 package 没有 kani harness）。`record.rs` 因那把尺子在 443 行处切出 `record/sink.rs`——切口落在「可不可以开始录」与「由谁写、写到哪」之间。

`platforms.yml` 的 macOS job 在 workspace 测试之后跑同一条 `just check-desktop`，所以非 Windows 臂与只在非 Windows 上编译的测试（`session/tests.rs` 里 `#[cfg(not(windows))]` 的那一条）每晚在 macOS 上过一次 clippy 与 nextest。那台 runner 没装 `cargo-deny`，配方里的许可证检查会自行跳过；许可证只在 `ci.yml` 的 Windows desktop job 里判。

### 16.2 怎么测一件需要桌面的事

本 card 的测试分两层，分界线就是 §8-8 那张表的最后一列：

- **不碰 Win32 的那几处逐条测**（`target`／`views`／`strokes`／`encode`／`keys`／`geometry`／`focus::settled`／`Admitted::visible`）。最容易错的几件事——选中了哪个窗口、过期或挪动过的窗口上的动作有没有被拒、一批输入被截断时还按着什么、一张图缩成什么尺寸、一个键名映到什么、键盘不在这个窗口手里时发不发、allowlist 没列的窗口报不报——全在这一层。它们住在 `platform/windows/` 之下，只在 Windows 上编译，所以在**任何一台 Windows** 机器上都跑得起来，不需要桌面；`Admitted::visible` 与协议壳（`session`、`refusal`、`answer`）的测试在每个平台上都跑。DPI 感知在测试进程里读回，不需要窗口：nextest 每个测试一个进程，进程级的声明不会串到别的测试。
- **碰 Win32 的五个模块只测「拒绝是诚实的」**：一个不存在的窗口名恒得到一句指向 `desktop.windows` 的拒词，而不是一次崩溃。CI 里没有一张桌面可供点击，故「点下去真的点中了」这件事**恒不**被写成一条会在没有桌面时假装通过的测试；它由操作者在真机上验，本节记下这是一处**具名的空缺**，不是一处被忽略的覆盖率。

- **换过接口的调用各有一条契约测试**（§8-11）：测试在本进程里建一扇不抢焦点的窗口，读回它的标题、进程、外框、落点与树；这几条要一张桌面，不碰人桌面上别的窗口。

这条分界线是诚实的代价：写一条「在没有窗口时也返回 ok」的测试会比现在好看，但它证明的是这条测试自己，不是这台 server。

**操作者检查**（真机上由人做，不是门）：一台按 150% 缩放的显示器上，`desktop.snapshot` 之后按 ref 点击落在控件中心；对记事本 `type` 一段文字，文字进了指名的那扇窗口，动作之后 Ctrl、Shift 没有停在按下；两扇同名窗口按 `process` 指名录制，录到的是那一扇；写剪贴板期间 `GetClipboardOwner` 不为空；一张截图经 MCP 到模型，模型能说出图里的内容。

## 17 模型体验

工具名恒是 `desktop.<动词>`，模型一眼看得出这一件事发生在桌面上而不是页面里。每条说明的后半句写的是**不做什么**，因为模型下一步最贵的错误是把一件工具当成它旁边那件。拒词恒给一个可执行的下一步：越界给「把这个窗口写进 `DESKTOP.toml`」，未实现给「这个 build 里没有它」。拒词全文作为文字到达模型：城这一侧把 `isError` 结果读成一次失败，文字原样进它的 subject；部分输入另带效果未知，模型读到的是「先看一眼再动」，不是「可以重试」。

## 18 文档同步

`ARCHITECTURE.md` §3 depmap 块的 `desktop` 一行｜`desktop/README.md`（英文，讲清它为什么住在 workspace 外、城怎么起它）｜sprawling-SPEC §8-4d｜同步本 SPEC §13 与 §8-7 的实现状态｜`desktop/README.md` 的 The honest-refusal rule 一节｜agent_protocols-SPEC §8-1c（城怎么读 `isError` 与 `_meta`）。

