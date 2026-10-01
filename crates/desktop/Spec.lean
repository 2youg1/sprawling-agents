-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.desktop.spec.Outline
import crates.desktop.spec.Platform.Windows.Strokes
import crates.desktop.spec.Platform.Windows.Target
import crates.desktop.spec.Platform.Windows.Views
import crates.desktop.spec.Scope
import crates.desktop.spec.Scope.Pattern
import crates.desktop.spec.Session

/-! # desktop 的规格

`sprawling-desktop`（库名 `desktop`，目录 `crates/desktop`）是一台按行说话的 MCP server：它给 agent 一双眼睛和一双手，落在这台 Windows 桌面上，并且这双手从第一天起就受一份 allowlist 约束。它是根工作区的成员，继承工作区的 lint 表，作为库链进 `sprawling`，由 `sprawling desktop` 这个动词起成子进程；它唯一的 FFI 缝 `sprawling-desktop-ffi`（`crates/desktop/ffi`）是另一个成员，规格是 `crates/desktop/ffi/Spec.lean`。

本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `desktop D<n>`。D1 到 D14 沿用这份规格在 Markdown 时 §12 的条目号，§10 的设计一到设计十是它原来 §8.5 的四对与 §8.6 的六对。
-/

/-! ## 1 需求分解

一件事：**给 agent 一双眼睛和一双手，落在这台 Windows 桌面上**，并且这双手从第一天起就受一份 allowlist 约束。

拆成三件可独立验收的事：

- **协议壳（`rpc` ＋ `session`）**：一台按行说话的 MCP server，握手与 `tools/call` 的形状与 `crates/agent_protocols/src/mcp/*` 所写的客户端逐字对齐，于是城把它当一台普通的 stdio MCP server 连：一栋楼的 `RULES.toml` 写 `desktop = true`，城就以 `sprawling desktop <DESKTOP.toml>` 起自己这个二进制（sprawling-SPEC §8-4d），之后走的是与任何 `[[mcp]]` 相同的那条路。
- **工具表（`tools`）**：六件工具的名字、说明与入参 schema **定死**。每条说明都写明这件工具**不做**什么。
- **拒绝故事（`scope` ＋ `platform`）**：工具名认出之后的拒绝——越界、未实现、平台做不到——都以 MCP `CallToolResult` 的 `isError` 结果回答，文字里是带稳定错误码的三段拒词；握手未完、方法或工具名不认识、行读不出，这几类协议层的错误才回 JSON-RPC error。没有 scope 文件＝全拒。

第三件事的后半句是**真的实现**，且它换掉的只有 `platform` 一个模块：协议壳与工具表一行不改，这正是把形状定死所买到的东西。

分部与单元的对应：`scope`（`spec/Scope.lean`）与 `scope::pattern`（`spec/Scope/Pattern.lean`）、`session`（`spec/Session.lean`）、`outline`（`spec/Outline.lean`），以及 Windows 臂里不碰 Win32 的三处判定：`strokes`（`spec/Platform/Windows/Strokes.lean`）、`views`（`spec/Platform/Windows/Views.lean`）、`target`（`spec/Platform/Windows/Target.lean`）。
-/

/-! ## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| rpc | 一条消息恒是单行且不含换行；答案恒携原 `id`；读不出的行回 `-32700` 而不是沉默 |
| answer | 做成了的调用恒答 `{ content: [...] }`；截图恒是一块 image content 在前、一块写 title／width／height／lossless 的文字在后；答复里恒不出现 `base64` 字段 |
| refusal | `isError` 结果的文字恒含码、拒了什么、为什么、还能做什么；效果未知的拒绝恒在 `_meta` 带 `"sprawling/effect-unknown": true`，其余拒绝恒不带 `_meta` |
| session | 未握手完成前 `tools/list`／`tools/call` 恒被拒；`notifications/initialized` 恒无答案；`ping` 恒答空对象；未知方法回 `-32601`；`tools/call` 在工具名认出之后的拒绝恒以 `isError` 结果回答，恒不以 JSON-RPC error 回答 |
| tools | 六个名字恒在 `tools/list` 里；每条 description 恒含一句「不做什么」；每张 `inputSchema` 恒是 `type: object` |
| outline | 同一串行恒折成同样的字节；每行恒是 `e<n> <role> "<name>"`，深度每深一层多缩进两格，没有名字的行恒不带引号；名字里的控制字符恒换成空格、空白恒压成一个、超过 80 个字符恒截断并以 `…` 收尾；role 恒是封闭词表里的一个词；遍历提前停下时，最后一行恒说出停在哪里、为什么 |
| scope | 缺文件与坏文件恒全拒；空 allowlist 恒不容许任何窗口；`record`／`clipboard` 未开则该工具恒被拒；没写 `sound` 则要声音的录制恒以 `E_GATE_DENIED` 被拒；不指名窗口的整屏截取恒被拒；allowlist 没列的窗口恒不出现在 `desktop.windows` 的答复里 |
| platform | 非 Windows 上恒回 `E_TOOL_UNAVAILABLE` 并报出平台名；Windows 上六件工具皆真的落到这台桌面上 |
| windows::target | 名字命中零个窗口恒被拒并指向 `desktop.windows`；命中两个以上恒被拒并列出各自的 title，**恒不**在其中挑一个 |
| windows::tree | 兄弟元素恒按窗口列出它们的次序铸 ref（文档序：先父后子，子按次序）；role 恒取自 control type 的编号，恒不取本地化的字符串；walker 的故障恒让遍历停下并被说出，恒不被读成「到头了」；ref 恒不超过 500 个 |
| windows::views | 快照恒推进这扇窗口（按窗口句柄，不按标题）的 generation；对着旧 generation 做的动作恒被拒；对着快照之后挪动过或改过尺寸的窗口做的动作恒被拒；快照没铸过的 ref 恒被拒 |
| windows::encode | 三种格式各自解得回原尺寸；`scale` 恒按百分比缩，且缩到 0 像素恒被拒而不是产出空图；`scale` 与 `quality` 域外的值**在解析点被拒**，不是钳位也不是静默换默认值 |
| windows::focus | 键盘在别的窗口手里时恒不发事件而回 `E_TOOL_UNAVAILABLE`；指针动作落点被别的窗口盖住时同样恒被拒；两条拒词恒写明「什么都没发出去」 |
| windows::keys | 表里每个键名恒映到一个虚拟键码；表外的键名恒被拒并列出可用的键名 |
| windows::strokes | 一批事件只被收下前 k 个（0 < k < n）时恒报出 k／n 并标效果未知；恒只补发前缀里按下而未抬起的键与鼠标键的抬起，恒不补发按下，恒不重发整批；一个都没收下时恒说「什么都没发出去」 |
| windows::dpi | 连接器答第一次调用之前，本进程恒已按显示器感知 DPI；做不到时每一次调用恒被拒，恒不在两种坐标之间混算 |
| windows::clipboard | 读恒以 `GlobalSize` 为上界；锁不住恒是拒绝而不是「没有文本」；写恒先备好整块内存再清空剪贴板，交不出时恒释放那块内存，拒词恒说明剪贴板已被清空 |
| windows::record | 同一窗口重复 start 恒被拒；未 start 就 stop 恒被拒；每一帧恒取自 `capture::window`；抓帧失败恒让录制停下，`stop` 恒交出落盘路径并说出写了多少帧、为什么提前停；`audio: true` 恒只录 scope 文件点名的那一个设备；`stop` 恒把不超过 `SOUND_CARRIED_MOST` 的声音作为一块 audio content 放在文字之前交回，更长的只交路径并说出为什么 |
| unsafe | 生产代码的 `unsafe` 恒只在 `crates/desktop/ffi/src/` 之下，恒是一次对 Zig 叶子的调用或那一处声明（§8-12 的表）；每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是**使它成立、并且可能为假的前提**，而不是把这次调用换句话再说一遍 |

分部里的定理是模型对性质的证明：scope 缺与坏都关成全拒、两个标识都要中、空 allowlist 不准任何窗口、没写 `sound` 不录声音；`*` 匹配任何名字、不带 `*` 的 pattern 只匹配等长的名字；握手完成之前不出工具；名字无控制字符且有上界；部分输入只补发前缀里按下过的东西的抬起；旧的一代与挪动过的窗口都被拒；两扇以上中了恒不挑。每个分部各有一条「拿掉守卫即反例」的定理。表里其余的行由各模块旁的测试经生产入口断言（§16）。
-/

/-! ## 3 假设与歧义

- **假设**：运行中的机器上的桌面是操作者自己的桌面。本 package 不做远程桌面、不做跨机器、不做无人值守的持续录制。
- **歧义已定**：scope 文件只能表达 allowlist（window title patterns ＋ process names）、`record`／`clipboard` 两位开关与 `sound` 一个设备名，**没有**「允许整屏」这一项。故**整屏截取在本版恒被拒**（见 §10 设计三），而不是被默许——一张全屏图会显示 allowlist 没有列出的一切。
-/

/-! ## 4 现状分析

`crates/agent_protocols` 已经是这套协议的**客户端**权威：`Rpc::initialize`／`initialized`／`list_tools`／`call_tool`／`read` 定死了城里说出去的每一行，`agent_protocols::mcp::stdio` 定死了字节怎么走（子进程、按行、消息内无换行、超时即回收子进程）。本 package 是那一端的**对侧**，因此它的形状不是设计出来的，是**读出来的**。

城这一侧读 `tools/call` 答复的是 `agent_protocols::McpTool`：`isError: true` 的结果是一次失败，拒词全文进它的 subject；`_meta` 带 `sprawling/effect-unknown` 时它标 `Retry::Unknown`（`crates/agent_protocols/Spec.lean` §8-1c）。
-/

/-! ## 5 权威信源

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
-/

/-! ## 6 命名统一

沿用词汇表（`docs/glossary.md`）：**three-part refusal**（拒了什么／为什么／还能做什么）、**reference**（快照铸的句柄，形如 `e1`）、**generation**（快照的世代号，随动作一起走）。

- **scope** 在本 package 里专指 `DESKTOP.toml` 里那份 allowlist，与 `kernel` 的 halted scope 不同层，故恒不缩写成裸词 `policy`。
- **恒不**把窗口叫作 page，**恒不**把桌面叫作 browser：`browser::act` 是被借鉴的形状，不是被复用的名字。
- 错误码是 `kernel::AxCode` 的六个变体（`E_TOOL_UNAVAILABLE`／`E_GATE_DENIED`／`E_INVALID_ARGS`／`E_TOOL_UNKNOWN`／`E_CONFIG_INVALID`／`E_WIRE_MISMATCH`），拼写只取 `AxCode::as_str`，本 package 不写第二份（§10 设计一）。
- **效果未知（effect unknown）**：请求已经交出一部分，桌面上是否生效不知道。城里对应 kernel 的 `Retry::Unknown`；本 package 用 `_meta` 的 `sprawling/effect-unknown` 说出它（D3）。

Lean 里的名字与 Rust 的对应：`Desktop.Scope.Tool` ↔ `tools::ToolName`，`namesAWindow` ↔ `names_a_window`，`Allowance.admits` ↔ `Allowance::admits`，`Desktop.Scope.Pattern.fits` ↔ `Pattern::matches`（`matches` 在 Lean 里是关键字），`Desktop.Session.Phase` ↔ `session::Phase`，`Method.opening`／`Method.opened` ↔ `initialize`／`notifications/initialized`（`initialize` 在 Lean 里是关键字），`Desktop.Outline.LABEL_MOST` ↔ `outline::LABEL_MOST`，`Strokes.lifts` ↔ `strokes::left_held`，`Views.mint`／`Views.resolve` ↔ `Views::mint`／`Views::resolve`，`Target.choose` ↔ `target::choose`。
-/

/-! ## 7 模块边界

- **字节从哪里来**归 `serve_stdio` 的调用方，城里的 `bin::main::desktop`：一个位置参数（`DESKTOP.toml` 的路径）与一对管道。stdin／stdout 的锁与刷新住 `serve_stdio`；`session` 只收一行、出一行。
- **准不准做**归 `scope`：一次 `tools/call` 在碰到平台之前先过它，于是「越界」这件事在任何 Win32 调用之前就已判完。
- **做得成做不成**归 `platform`：`cfg(windows)` 两个文件各自完整，**无 trait**——一个只有一个实现的接口是装饰（ARCHITECTURE §4）。没有准入安全接口的四组 Win32 调用由 `desktop_ffi` 整段做完（§8-12）；`platform` 只调它的安全函数，把它答的 step 与错误码写成拒词。
- **一行 allowlist 匹配什么**归 `scope::pattern`：glob 语义与文件解析是两件会各自变的事，且前者要被单独证明会终止（§10 第 6 条）。
- **说什么**归 `tools`：六张卡片是数据，改它就是改行为（形状 6）。
- **一扇窗口读起来是什么样**归 `outline`：role 的封闭词表、名字的清洗与截断、一串节点折成的文字。它平台无关，因为 macOS 臂以后把 AX 的 role 映到同一张词表；control type 编号到词表的映射是 Windows 的事实，住 `windows::tree`。

**恒不**引入：async runtime、HTTP 客户端、任何 GUI 框架。workspace crate 只依赖三个：`kernel`（错误码与质量的域）、`agent_protocols`（协议修订与效果未知的键）、`desktop_ffi`（只为本 package 存在的 FFI 缝），每一条边都是为了读一个事实的唯一定义，而不是为了它的行为（D14）。

D14 并回 workspace：本 package 继承 `forbid`，FFI 叶子自带一张只差一行的 lint 表。

- **决定**：server 与它的 FFI 缝住在 `crates/desktop` 与 `crates/desktop/ffi`，两个都是根工作区的成员；它们不再自成一个工作区，没有第二份锁，根 `Cargo.toml` 不 exclude 任何包。本 package 以 `[lints] workspace = true` 继承工作区的 lint 表（`unsafe_code = "forbid"`），包元数据与共享依赖都经 `workspace = true` 继承；lib 名改为 `desktop`，与 depmap 的行名、与其余 crate 的「lib 名即目录名」一致。`desktop_ffi` 写一张自己的 `[lints]`：与根 `[workspace.lints]` 逐键相同，只有 `unsafe_code` 是 `deny`。`xtask guard` 判这一张表（逐键等于根表，例外只有记下理由的 `unsafe_code`），并判其余每个成员的 `[lints]` 恰是 `workspace = true`。本 package 依赖 `kernel` 与 `agent_protocols`：错误码的拼写取 `kernel::AxCode::as_str`，质量的域取 `kernel::consts_policy::IMAGE_QUALITY`，协议修订取 `agent_protocols::PROTOCOL_VERSION`，效果未知的键取 `agent_protocols::EFFECT_META_KEY`；`refusal.rs`、`rpc.rs`、`encode.rs` 里的抄件、`xtask guard` 的 `quoted` 比对与 `kernel::error::code` 里读 `refusal.rs` 的那条测试一起删去。测试自己建窗口要写的 `unsafe` 搬进 `desktop_ffi::fixture`，只在 `fixture` feature 下编译；本 package 只在 Windows 的 dev 依赖里打开它，`xtask artifact` 把 `fixture` 列为测试 feature。
- **理由**：本 package 坐在墙外的理由只有一条（Win32 边界要放开 `unsafe_code`，§10 设计二）。X3 把那些调用收进 Zig 叶子之后（D12），本 package 的生产代码已不写 `unsafe`，这条理由只剩叶子那一层。墙外的代价却都还在：一份抄来的 lint 表、包元数据与依赖版本，第二份锁，六个错误码、质量域、协议修订与 `_meta` 键各一份第二拼写，`depmap` 只能陈述那一行而守不住，`cargo clippy --workspace` 判不到它。并回之后这些抄件都换成引用，剩下的唯一一处差异是叶子那张表的 `unsafe_code` 一行，guard 判的正是它。叶子留在工作区里，而不是另起一个工作区：放在外面它就要自己的锁与 `[workspace.package]`，guard 又要比对一整堵墙；在里面，它与别的成员共享锁、元数据与依赖版本，只有一张表是自己的。依赖 `agent_protocols` 只为两个常量，代价是本 package 单独编译时要先编 `agent_protocols` 与 `gateway`；二进制不变，因为 `sprawling` 本来就链接它们，而这是两个常量各只剩一个定义的唯一做法。
- **击败的备选**：①叶子也继承 `forbid`，把 `extern` 声明交给一个生成的或第三方的安全绑定——今天没有合格的绑定（D9），而 Rust 2024 的 `unsafe extern` 块在 `forbid` 下编不过；②叶子留在一个墙外工作区，只把本 package 并回——墙还在，只是变小，第二份锁与抄写核对都留着；③测试建窗口的 `unsafe` 留在本 package，给它另开一张 lint 表——cargo 的 lint 表不分 profile，给测试开就是给生产代码开；④不依赖 `agent_protocols`，两个常量留着由 guard 比对——一个事实两个定义，再加一道只为它们存在的门；⑤lib 名保持 `sprawling_desktop`——`depmap` 按 lib 名判边，那一行就要改名，而其余每个 crate 的 lib 名都是它的目录名。
- **重开的参数**：一个合格的安全绑定覆盖了叶子的四组调用，叶子离开，那张自己的表与 guard 一起删去；或 cargo 允许一个成员继承工作区 lint 表而只改一行，那时叶子写 `workspace = true` 加一行覆盖，guard 的比对随之删去。
-/

/-! ## 8 接口先行

```rust
// 8-1 refusal（形状 2 值类型）
pub(crate) enum RefusalCode { ToolUnknown, ToolUnavailable, InvalidArgs, GateDenied, ConfigInvalid, WireMismatch }
impl RefusalCode {
    pub(crate) fn code(self) -> kernel::AxCode;   // 拼写只在 AxCode::as_str
    pub(crate) fn json_rpc(self) -> i64;          // 保留区间或 -32000 起
}
pub(crate) struct Refusal { /* 私有：code／action／subject／recovery／aftermath */ }
enum Aftermath { Known, Unknown }                 // 私有：拒词说的就是桌面上发生的，或者不知道
use agent_protocols::EFFECT_META_KEY;             // 唯一定义在 agent_protocols::mcp::tools
impl Refusal {
    pub(crate) fn new(code: RefusalCode, action: &str, subject: impl Into<String>, recovery: &str) -> Refusal; // Aftermath::Known
    pub(crate) fn effect_unknown(self) -> Refusal; // 请求已交出一部分，效果不知道
    // 出口按答法分两个，没有专为测试读字段而开的第三个：
    pub(crate) fn as_error(&self) -> Value;        // 协议层：{ code, message, data: { code, action, subject, recovery } }
    pub(crate) fn as_tool_result(&self) -> Value;  // 工具层：{ content: [{ type: "text", text }], isError: true, _meta? }
}

// 8-2 rpc（形状 4 适配器）
use agent_protocols::PROTOCOL_VERSION;            // 两端谈的是同一个修订，只有一个定义
pub(crate) struct Request { pub(crate) id: Option<Value>, pub(crate) method: String, pub(crate) params: Value }
pub(crate) fn read(line: &str) -> Result<Request, Refusal>;
pub(crate) fn result_line(id: &Value, result: Value) -> String;
pub(crate) fn error_line(id: Option<&Value>, refusal: &Refusal) -> String;

// 8-2b answer（形状 2 值类型）：一次做成了的 tools/call 怎么答
pub(crate) struct Answer { /* 私有：Vec<Block> */ }
enum Block { Text(String), Image { bytes: Vec<u8>, mime: &'static str }, Sound { bytes: Vec<u8>, mime: &'static str } } // 私有
impl Answer {
    pub(crate) fn facts(facts: Value) -> Answer;                                        // 一块文字：facts 的单行 JSON
    pub(crate) fn picture(bytes: Vec<u8>, mime: &'static str, facts: Value) -> Answer;  // 一块图片在前，一块文字在后
    pub(crate) fn sound(bytes: Vec<u8>, mime: &'static str, facts: Value) -> Answer;    // 一块 audio 在前，一块文字在后（D13）
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
pub(crate) struct Allowance { /* 私有：windows／processes／record／clipboard／sound */ }
// 准入是一个值：它带着准入了这次调用的那份 allowlist，
// 于是「答复本身受 scope 约束」的那一件工具读的是同两张 Pattern 表。
pub(crate) struct Admitted<'a> { /* 私有：&Allowance；非 Windows 的非测试构建里没有读者，字段带 expect(dead_code)（D1） */ }
impl Admitted<'_> {
    #[cfg(any(windows, test))]   // 只有 Windows 臂报窗口；测试在每个平台上都判它（§16.2）
    pub(crate) fn visible(&self, title: &str, process: &str) -> bool;
    #[cfg(any(windows, test))]   // 只有 Windows 臂录声音
    pub(crate) fn sound(&self) -> Result<&str, Refusal>;   // scope 文件点名的那一个设备；没写 `sound` 即 E_GATE_DENIED（D11）
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

// 8-4c outline（形状 6 数据＋形状 1 判定）：一扇窗口的树怎么读给模型
pub(crate) enum Role { Button, Calendar, CheckBox, ComboBox, Edit, Hyperlink, Image, ListItem, List, Menu, MenuBar,
    MenuItem, ProgressBar, RadioButton, ScrollBar, Slider, Spinner, StatusBar, Tab, TabItem, Text, ToolBar, ToolTip,
    Tree, TreeItem, Custom, Group, Thumb, DataGrid, DataItem, Document, SplitButton, Window, Pane, Header,
    HeaderItem, Table, TitleBar, Separator, SemanticZoom, AppBar }   // UIA 的四十一种 control type，一一对应
impl Role {
    pub(crate) const fn word(self) -> &'static str;   // UIA 的类型名，小写：`button`、`edit`、`menuitem` …
    pub(crate) const fn frames(self) -> bool;         // pane／group／custom／separator：没有名字时不值一行
}
pub(crate) struct Line<'a> { pub(crate) reference: &'a str, pub(crate) role: Role, pub(crate) name: &'a str, pub(crate) depth: u32 }
pub(crate) enum Ending { Whole, RefLimit { most: usize }, Fault(String) } // 遍历为什么停下
pub(crate) fn label(raw: &str) -> String;                  // 控制字符换空格、压空白、截到 LABEL_MOST 个字符
pub(crate) fn fold<'a>(lines: impl IntoIterator<Item = Line<'a>>, ending: &Ending) -> String;

// 8-5 session（形状 4 适配器；Phase 是形状 5 typestate 的运行时投影）
// 本 package 的全部公开面就这一个函数：一台 server 的其余一切都经协议抵达。
pub fn serve_stdio(scope_path: Option<&Path>) -> std::io::Result<()>;
pub(crate) enum Phase { Fresh, Initializing, Ready }
impl Phase {
    fn opened(self) -> Result<Phase, Refusal>;   // notifications/initialized 之后的阶段；Fresh 即 E_GATE_DENIED，阶段不动（D15）
}
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
| `desktop.snapshot` | 一个窗口的 accessibility tree，折成一段文字（`outline`）：每个元素一行，写它的 ref、role 与 name，按深度缩进；另一块文字写 title、process 与 generation；bounds 只留在 `Views` 里，由 `desktop.act` 按 ref 取 | 不给像素、不给控件的内部句柄、不读被遮挡的内容 |
| `desktop.act` | ref 或 point ＋ 动作（click／double／right／drag／scroll／type／key，带 modifiers），携 snapshot 的 generation；**发事件前先核对前台窗口**；桌面只收下一部分时如实报数，并补发它留下按住的键与鼠标键的抬起 | 不合成整段脚本、不重试、不在 generation 过期或窗口挪动后改打别处、不在别的窗口拿着键盘时把按键发出去 |
| `desktop.screenshot` | window／region，format png\|jpeg\|webp，quality、scale；答一块 image content（`data` 是 base64，`mimeType`）和一块写着 title／width／height／lossless 的文字 | 不做 OCR、不做比对、不落盘 |
| `desktop.record` | start／stop：本 package 唯一那条线程经 `capture::window` 抓帧，PATH 上有 ffmpeg 就经它的 stdin 编成 mp4，否则写成一个 PNG 序列目录；`audio: true` 另起一个 ffmpeg 录 scope 文件点名的那一个声音设备（D11），`stop` 把录下的声音作为一块 audio content 交回（D13） | 不做剪辑、不做转码、不替人选声音设备、不在没说 stop 时自己停（十分钟上限除外） |
| `desktop.clipboard` | get／set 文本 | 不碰图片与文件列表、不保留历史 |

`desktop.act` 携 generation 是照抄 `browser::act` 的那一条：**对着一份快照做的决定，恒不落到另一份快照上**——过期就拒，而不是打到那时挪过去的东西上。

**`drag`／`scroll` 的形状只有一份**：桌面侧与浏览器侧读同一组字段——起点是 `ref` 或 `point`，拖拽终点与滚动增量都用 `to`，`steps` 是中间移动次数。浏览器侧由 `browser::input` 实现（`crates/browser/Spec.lean` 的 D9），桌面侧由这张表实现；两侧的 `Action`／动词名逐字对齐，不会各自演化出一套。

### 8-8 Windows 这条胳膊的内部

`platform::perform` 这个自由函数是一张**桌子**：

```rust
// 8-6 platform（形状 4 适配器；cfg 二选一，无 trait）
pub(crate) struct Desk { /* 私有：views／recordings／pixels／reader */ }
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
| `windows::enumerate` | 这台桌面上有哪些顶层窗口（经 `desktop_ffi::top_level`），各自的 title／process／bounds | 4 适配器 | **是**（窗口事实经 `winsafe`）|
| `windows::target` | 从一串窗口里按 title／process 挑出**恰好一个** | 1 判定 | 否 |
| `windows::views` | 一次快照铸了哪些 ref、一扇窗口（按句柄）现在是第几代、快照时它的矩形，以及一个动作该不该被这一代接受 | 1 判定 | 否 |
| `windows::tree` | UIA 树：role／name／ref／bounds，经 `uiautomation`；一张桌子读树用的公寓与 automation 对象（`Reader`） | 4 适配器 | **是** |
| `windows::keys` | 键名到虚拟键码的那张表 | 6 数据 | 否 |
| `windows::strokes` | 一个动作是哪几个事件（键、Unicode 单元、指针）；一批只被收下前 k 个时哪些键与鼠标键还按着，以及那句拒词 | 1 判定 | 否 |
| `windows::focus` | 键盘现在在谁手里、一个屏幕点下面是哪个窗口，以及两者都不是它时的那句拒词 | 1 判定 | **是**（两次只读，加一次置前）|
| `windows::act` | `SendInput`：一批事件交给桌面；只收下一部分时补发一次抬起 | 4 适配器 | **是** |
| `windows::capture` | 一个窗口变成一片 RGBA 像素（GDI 那一段经 `desktop_ffi::capture`），以及全黑判为失败 | 4 适配器 | 经 `desktop_ffi` |
| `windows::encode` | 像素按 `scale` 缩、按 `format` 编码、按 base64 出门 | 1 判定 | 否 |
| `windows::dpi` | 本进程按哪种 DPI 感知读桌面：连接器开张时声明按显示器感知，失败时读回判定 | 4 适配器 | 经 `desktop_ffi` |
| `windows::record` | 这条连接正在录哪些窗口、每一份由谁在写 | 4 适配器 | 否 |
| `windows::record::sink` | 一份录制的字节由谁写、落到哪里：本 package 唯一那条线程抓帧，交给 ffmpeg 的 stdin 或写成 PNG 序列 | 4 适配器 | 间接 |
| `windows::record::hearing` | 一份录制的声音：第二个 ffmpeg 读 scope 文件点名的 DirectShow 设备，写成录制目录里的一个 wav | 4 适配器 | 否（ffmpeg 读设备）|
| `windows::clipboard` | 运行中的机器的剪贴板，作为文本（经 `desktop_ffi::clipboard`），以及每个失败的那一步怎么说 | 4 适配器 | 经 `desktop_ffi` |

**这张表的分法就是 Humble Object**（ARCHITECTURE §9）：难测的那一端（`enumerate`／`tree`／`act`／`capture`／`clipboard`／`dpi`／`focus` 的三次平台调用）薄到几乎没有判断，判断都搬进了 `target`／`views`／`strokes`／`encode`／`keys`／`geometry`／`focus::settled` 七处纯代码——它们一行 Win32 都不跑，因而可以被逐条证明。一台没有桌面的机器上，本 package 仍然能证明「哪个窗口被选中」「过期的动作被拒」「一张图缩成什么尺寸」「键盘不在这个窗口手里时什么都不发」「一批输入被截断时还按着哪些键」这几件最容易错的事。

### 8-9 `unsafe` 的那一条规矩

本 package 继承工作区的 `unsafe_code = "forbid"`，生产与测试代码都写不出 `unsafe`。唯一放开它的一层是 `desktop_ffi`：它的 lint 表与工作区逐键相同，只有 `unsafe_code` 是 `deny`（§10 设计二、D14）。既然是花了代价换来的，代价就要花在明处：

**每一个 `unsafe` 块恒带一行 `SAFETY:`，写的是使这次调用成立的前提。**「我们调用 `EnumWindows`」不是前提，那只是把调用换句话再说一遍；「`into` 有 `into.len()` 个已初始化的槽、只借给这一次调用，叶子按同一个长度写」才是前提。审这一条的办法是逐个 `SAFETY:` 问一句：它说的东西**能不能是假的**？不能为假的句子不是前提，是复述。

生产代码的 `unsafe` 恒只在 `crates/desktop/ffi/src/` 之下，且恒只包住一次对 Zig 叶子的调用——不包住随后的判断，因为把安全代码收进 `unsafe` 块只会让下一个读者多审几行。本 package 自己的 `src/` 一行 `unsafe` 也不写；测试为了建自己的窗口要写的 `unsafe` 在 `desktop_ffi::fixture` 里，各带 `SAFETY:`，只在 `fixture` feature 下编译（§8-11）。哪些生产调用写 `unsafe`，只看 §8-12 那张表；本节不另列一份。

几条最容易写成复述的前提，在这里点名。剪贴板的两次调用要求本进程没有别的线程正处在一次打开与关闭之间：`desktop_ffi::clipboard` 的进程内轮次锁就是这条前提，所以 SAFETY 行写的是「锁在手里」这件可能为假的事。句柄写进 `winsafe::HWND` 的槽里，前提是槽数与交给叶子的容量是同一个数。叶子里那几条前提——位图解除选择之后才读回、`ReleaseDC` 传取得 DC 时的那扇窗口、剪贴板的块以 `GlobalSize` 为界——不再是 Rust 的 `SAFETY:`，而是 `crates/desktop/ffi/Spec.lean` 证明的模型性质与 `leaf.zig` 里写在取得旁边的 `defer`。

### 8-10 进程的两端住城里

本 package 没有自己的可执行文件，公开面只有 `serve_stdio` 一个函数。

- **进程从哪里起**：`sprawling desktop [scope]`（`crates/sprawling/src/main/desktop.rs`，sprawling-SPEC §8-4d）。一个位置参数是 `DESKTOP.toml` 的路径，缺席即 `Scope::Closed`。它不判定任何事：作用域归 `scope`，应答归 `session`。
- **唯一真的把进程拉起来的测试**也住城里（`crates/sprawling/tests/desktop.rs`）：城按一栋楼的规则起 `sprawling desktop`，握手、list，模型收到六件工具。其余测试只证明库里的判断。

### 8-11 按操作准入的安全接口

Windows 臂的每一次平台调用都落在下表的一行。「实现」一栏是它今天经过的接口；写 `desktop_ffi` 的行由 Zig 叶子整段做完（§8-12），本 package 只调它的安全函数。「准入」一栏是审过、可以换上的安全接口，写「无」的行理由在 D9、D10。换一行的次序是固定的：先写它的契约测试，在旧实现上跑绿，证明契约不依赖实现；再换实现，并在同一提交里把这一行的「实现」改成新的接口。

| 操作 | 模块 | 调用 | 实现 | 准入 | 契约测试 |
|---|---|---|---|---|---|
| 枚举顶层窗口，铸出句柄 | `enumerate` | `EnumWindows` 与它的回调；回调把系统交来的值写进 `winsafe::HWND` 的槽 | `desktop_ffi`（Zig 叶子） | 无 | `a_window_this_process_opens_is_listed_by_its_title_process_and_bounds` |
| 一扇窗口的事实：可见、标题、进程映像名、外框 | `enumerate` | `IsWindowVisible`、`GetWindowText`、`GetWindowThreadProcessId`、`OpenProcess`＋`QueryFullProcessImageName`、`GetWindowRect` | `winsafe` | `winsafe` | 同上 |
| 前台与落点 | `focus` | `GetForegroundWindow`、`WindowFromPoint`、`GetAncestor`、`SetForegroundWindow` | `winsafe` | `winsafe` | `the_window_under_a_point_is_the_window_drawn_there` |
| 输入 | `act` | `SendInput`、`GetSystemMetrics` | `winsafe` | `winsafe` | `each_stroke_becomes_the_event_it_names` |
| 可访问性树 | `tree` | UIA 的 automation 对象、control view walker、元素属性；COM 公寓 | `uiautomation`，公寓经 `winsafe` | `uiautomation`，公寓经 `winsafe` | `a_windows_tree_names_the_control_inside_it` |
| 按窗口捕获 | `capture` | `GetDC`／`ReleaseDC`、`CreateCompatibleDC`、`CreateCompatibleBitmap`、`SelectObject`、`PrintWindow`、`GetDIBits` | `desktop_ffi`（Zig 叶子） | 无 | `a_window_this_process_opens_is_read_back_whole`、`capturing_a_window_gives_back_every_gdi_object_it_took` |
| 剪贴板文本 | `clipboard` | owner 窗口、`OpenClipboard`、`GetClipboardData`、`GlobalSize`／`GlobalLock`、`GlobalAlloc`、`EmptyClipboard`、`SetClipboardData` | `desktop_ffi`（Zig 叶子） | 无 | `text_written_to_the_clipboard_reads_back_as_itself`；锁不住的块由 `leaf.zig` 的测试 `a block that will not lock is a refusal, not an empty clipboard` 判 |
| DPI 感知 | `dpi` | `SetProcessDpiAwareness`、`GetProcessDpiAwareness` | `desktop_ffi`（Zig 叶子） | 无 | `a_desk_reads_this_desktop_in_physical_pixels` |

**句柄只在一处铸出。** `winsafe::HWND` 从裸指针构造（`from_ptr`）要写 `unsafe`，反方向（`ptr()` 交给叶子或 `uiautomation`）不要。所以窗口句柄只在边界的另一侧铸出：叶子把 `EnumWindows` 交来的值写进 Rust 借出的 `winsafe::HWND` 槽里（它是 `#[repr(transparent)]` 的指针），`desktop_ffi::top_level::windows` 答的就是这些槽，本 package 一处 `from_ptr` 也不写；其余模块只借用它，需要 `uiautomation` 的类型时就地转过去。

**契约测试在真窗口上跑，只碰测试自己建的窗口。** 枚举、落点、树与捕获这几行的契约，由测试在本进程里建一扇不抢焦点的窗口（`desktop_ffi::fixture`，只在 `fixture` feature 下编译，本 package 只在 Windows 的 dev 依赖里打开它；`platform/windows/fixture.rs` 只把它的外框换成本 package 的 `Bounds`）再读回它来判；它们不读、不点、不改人桌面上别的窗口。输入这一行的旧实现把事件写进 `INPUT` 联合体，不写 `unsafe` 读不回来，所以它的契约测试随替换一起写，判的是每个 stroke 变成了哪个 `HwKbMouse` 值；真的把事件送到窗口上仍是 §16.2 的操作者检查。

### 8-12 Zig 缝：`crates/desktop/ffi`

没有准入安全接口的四组调用（§8-11 里「实现」写 `desktop_ffi` 的四行）由一个包做完：

- **位置**：`crates/desktop/ffi/`，包名 `sprawling-desktop-ffi`，库名 `desktop_ffi`，规格是 `crates/desktop/ffi/Spec.lean`（边界规则与资源配对的定理在那里）。Zig 源码在 `crates/desktop/ffi/zig/`：`leaf.zig`（四组操作与 export）、`boundary.zig`（叶子往借来的缓冲里写什么）、`step.zig`（step 的 Zig 拼写）。
- **一张自己的表**：两个包都是根工作区的成员，包元数据与 `winsafe` 的版本行都以 `workspace = true` 继承。本 package 的 `[lints]` 是 `workspace = true`；`desktop_ffi` 的 `[lints]` 是它自己的一张，与根 `[workspace.lints]` 逐键相同，只有 `unsafe_code` 是 `deny`，`xtask guard` 判这一张表，例外只有那一行（tools/xtask/Spec.lean §8-46、D14）。
- **构建**：`crates/desktop/ffi/build.rs` 只用标准库起 `zig build-lib`（`ReleaseSafe`，目标取自 cargo 的目标），只在 Windows 目标上；别的目标上本包只剩 `step`，本 package 也只在 Windows 上依赖它。Zig 的版本只写在 `crates/desktop/ffi/zig-version`：构建脚本、doctor 的 `zig` 一行（sprawling-SPEC 8-146）与 CI 的安装步骤都读它。
- **对拍与 fuzz 的配方**：`desktop_ffi::boundary` 的测试以种子化输入比对叶子与 Rust 参考（`crates/desktop/ffi/src/reference.rs`），每条规则两万个；`just fuzz-desktop <rounds> <seed>` 把同一比对按给定的轮数与种子跑下去（Rust 一侧的 fuzz）；`just check-desktop` 里的 `zig test crates/desktop/ffi/zig/leaf.zig` 跑 Zig 侧的单测、种子化性质测试与 `std.testing.fuzz` 测试（Zig 一侧的 fuzz）。

生产代码的每一个 `unsafe` 都在这张表里，各是一次叶子调用，`SAFETY:` 写在调用旁：

| 位置 | 调用 | 前提（`SAFETY:` 的要点） |
|---|---|---|
| `desktop_ffi::leaf` | `unsafe extern "C"` 声明块本身 | 声明不承诺任何事；每次调用各写自己的前提 |
| `desktop_ffi::top_level::windows` | `sprawling_desktop_windows` | 槽数即容量；写进槽的值都是 `EnumWindows` 交来的 |
| `desktop_ffi::capture::pixels` | `sprawling_desktop_capture` | 缓冲长度是叶子自己的位图规则给的字节数 |
| `desktop_ffi::clipboard::text` | `sprawling_desktop_clipboard_read` | 缓冲长度即容量；进程内轮次锁在手里 |
| `desktop_ffi::clipboard::put_text` | `sprawling_desktop_clipboard_write` | 文本切片只读；进程内轮次锁在手里 |
| `desktop_ffi::dpi::declare` | `sprawling_desktop_dpi_declare` | 只传一个整数 |
| `desktop_ffi::dpi::awareness` | `sprawling_desktop_dpi_awareness` | 出参是一个活着的局部变量 |
| `desktop_ffi::boundary::keep` | `sprawling_desktop_keep` | 两个切片，长度随指针走 |
| `desktop_ffi::boundary::text_copy` | `sprawling_desktop_text_copy` | 同上 |
| `desktop_ffi::boundary::text_fill` | `sprawling_desktop_text_fill` | 同上 |
| `desktop_ffi::boundary::bitmap_bytes` | `sprawling_desktop_bitmap_bytes` | 两个整数与一个出参 |

叶子答 step（`desktop_ffi::step::Step`）与调用线程当时的 Win32 错误码；本 package 的 `platform::windows::fault` 是把它们写成三段式拒词的唯一一处，每个 step 对应一句「拒了什么」，码只在停在 Win32 调用上的 step 里读出。

D1 定规：`visible` 只在 Windows 与测试里编译。

- **决定**：`Admitted::visible` 与它所读的 `Allowance::visible` 带 `#[cfg(any(windows, test))]`；`Admitted` 的 `allowance` 字段带 `#[cfg_attr(not(any(windows, test)), expect(dead_code, reason = "…"))]`。类型与唯一的构造点 `Admitted { allowance: self }` 在每个平台上相同。
- **理由**：非 Windows 臂（`platform/elsewhere.rs`）拒绝一切调用，不报任何窗口，所以那里的非测试构建里这两个方法与这个字段没有读者，编译器报三条 `dead_code`。`visible` 是纯判定、不碰 Win32，§16.2 要它在任何机器上受测，所以条件里带 `test`。字段上用 `expect` 而不是 `allow`：哪天非 Windows 臂开始报窗口，字段有了读者，这条 `expect` 落空，构建变红，它就跟着被删掉。
- **击败的备选**：①按平台把字段拼成 `&'a Allowance` 与 `PhantomData<&'a Allowance>`——不需要任何 lint 抑制，但同一个值有两种拼写，构造点也要跟着分叉，而「是不是 Windows」按 `platform.rs` 的约定只写在一处；②等本 package 并入 workspace 时再处理——并入之后，workspace 的 `-D warnings` 会在 macOS 上把这三条警告变成错误。
- **重开的参数**：非 Windows 臂开始报告窗口。那时 `visible` 在每个平台上都有读者，本条与那条 `expect` 一起删除。

D12 没有准入安全接口的四组调用经一片 Zig 叶子，Rust 面零业务 `unsafe`。

- **决定**：枚举、捕获、剪贴板、DPI 四组调用由 `crates/desktop/ffi` 的 Zig 叶子整段做完（§8-12）：一个 export 一个完整操作，Rust 借出缓冲、读回 step 与错误码，句柄、DC 与全局块恒不跨边界。Rust 面每次调用一个 `unsafe` 块，加那一处 `unsafe extern` 声明，全部在 §8-12 的表里；本 package 的 `src/` 生产代码不写 `unsafe`。两个包都是根工作区的成员，共享锁、包元数据与 `winsafe` 的一条版本行；lint 表只有 `desktop_ffi` 那一张与工作区差一行（D14）。叶子往缓冲里写什么、资源怎么配对，由 `crates/desktop/ffi/Spec.lean` 证明模型性质，由对拍、fuzz 与真窗口上的契约测试检查实现。
- **理由**：AGENTS.md「Rust」一节给平台调用定了次序：先找安全接口，没有合格的就用 Zig 叶子，`unsafe` Rust 只在测量表明它整体最好时才准入。这四组在 X2 的审查里没有合格的安全接口（D9），口径 ① 要的正是业务代码零 `unsafe`、只留一个经审的 FFI 缝。整段操作放进叶子，是因为这四组的风险不在某一次调用，而在调用之间：位图选进之后要选回、DC 要随取得它的窗口还回去、全局块要么交出要么释放。这些次序在叶子里写成取得旁边的 `defer`，在 Lean 里写成可穷举的模型；若每个 Win32 调用各包一个 export，次序就又回到 Rust 的 `Drop` 与 `unsafe` 里（`desktop_ffi` D1）。
- **剩余限制**（写明，不当作已解决）：两侧的 fuzz 都是抽样而不是覆盖引导。Zig 的覆盖引导 fuzz（`--fuzz`）今天不在 Windows 上实现；cargo-fuzz 在 windows-msvc 上链接不出 sancov 的节区符号，nightly 也不带 msvc 的 ASan 运行时，而叶子只在 Windows 上编，所以 libFuzzer 没有一个能跑它的平台。叶子以 `ReleaseSafe` 编，抽样到的越界即 trap。Lean 证明的是叶子对操作系统回答的处理，操作系统本身是假设。
- **击败的备选**：①留在 `windows` 绑定上继续手写 `unsafe`（33 个块，与口径 ① 相反）；②一个独立的 Zig 可执行程序、经进程边界说话（多一个交付物与它的监管，而本 server 本来就是一个子进程，多一层隔离买不到新东西）；③每个 Win32 调用一个 export（见上）；④把叶子的调用留在本 package 里、本 package 整个用 `deny`（`unsafe` 的许可就落到了协议壳与平台臂上，那里一行也不需要它）。
- **重开的参数**：一个安全 crate 修好了其中一组（例如 `winsafe` 修好 `EnumWindows`），那一行先写契约测试、再离开叶子；Zig 的 `--fuzz` 或 cargo-fuzz 在 Windows 上落地，`just fuzz-desktop` 换成覆盖引导的那一种。

D13 录下的声音作为一块 audio content 交回城里。

- **决定**：`desktop.record` 的 `stop` 在这份录制录了声音时，答复的第一块是 MCP 的 audio content（`type: "audio"`、`data` 是 base64、`mimeType: "audio/wav"`），第二块是照旧的那段文字，多写 `sound`（wav 的路径）。wav 超过 `SOUND_CARRIED_MOST` 时不带 audio 块，文字里写 `sound_left_out` 说明多长、在哪里；声音那个 ffmpeg 提前退出时写 `sound_ended_early`，有多少交多少。
- **理由**：wav 落在临时目录（§14），城里没有一件工具读得到那里；交回城里的唯一一条不新增依赖、不新增写权限的路，是 MCP 答复本身。连接器已经把答复里的图片存进 CAS（`crates/runtime/Spec.lean` §8-27-10），声音走同一条路，模型读到的是一行带 locator 的字，base64 恒不进窗口也恒不进账本。截图恒是一块 image 在前（§8-2b），声音照同一个形状，模型读这两种答复用的是同一种读法。
- **击败的备选**：①把录音写进这座楼（scope 文件说的是能碰哪些窗口，没说能往城里哪里写，§14；而且本 server 不知道城根在哪）；②只交路径（城里没有工具读得到临时目录）；③把整段声音不论多长都塞进答复（十分钟的 wav 是 19 MB，base64 之后超过城的 MCP 单行上限，整个答复会被拒，连画面的路径都到不了）。
- **重开的参数**：城的 MCP 客户端改了单行上限；或者 `transcribe` 能直接读运行中的机器上的一个文件。
-/

/-! ## 9 工作流程

进程起来 → `sprawling desktop` 取 scope 路径（第一个位置参数，否则无）→ `serve_stdio` → `Scope::read`（缺文件即 `Closed`）→ `Server::serve` 阻塞读 stdin。

每收到一行：`rpc::read` → 按 method 分派 → `initialize` 答能力与自我介绍并进 `Initializing` → `notifications/initialized` 在 `initialize` 之后收到时无答案并进 `Ready`，在它之前收到时被拒、阶段不动（D15）→ `ping` 答 `{}` → `tools/list` 答六张卡片 → `tools/call` 先 `Scope::admits`（过了交出一个 `Admitted`），再把它随参数一起交给 `platform::perform` → 出一行 → flush。
-/

/-! ## 10 实现逻辑

1. **一条消息一行**：`serde_json::to_string` 不产生换行，且写出前不做美化；这与 `agent_protocols::mcp::stdio::Connection::framed` 的检查是同一条契约的两端。
2. **握手顺序被强制**：`Fresh` 只答 `initialize` 与 `ping`，`Initializing` 收到通知才进 `Ready`，`Fresh` 收到通知留在 `Fresh`（D15）。规范就是这么写的，而一个不强制它的 server 会让客户端的顺序错误在别处以别的形状爆出来。
3. **`id` 原样回**：不解析、不重编号——`id` 是对侧的东西。通知（无 `id`）恒无答案，否则会在管道里留下一行，此后每次调用读到的都是上一条的答案。
4. **glob 只认 `*`**：title pattern 是给人写的，一整套正则会让「我到底放开了什么」变成一个需要推演的问题。匹配按 ASCII 大小写不敏感，因为 Windows 的进程名就是这样比的。
5. **越界判定在平台之前**：`admits` 是纯判定，无 I/O、无时钟，于是它可以被逐条测，而 Win32 一行都还没跑。
6. **glob 一定终止**：回溯时文本下标只增不减，且到达文本末尾即失败，故循环恒有界；`scope::pattern` 里有一条专门打这一点的测试（一个「几乎匹配很多次」的名字）。
7. **`deny_unknown_fields`**：拼错的键若被默默忽略，写它的人会把它读成一个生效了的键。故坏键＝坏文件＝全拒。
8. **GDI 的三条前提写在取得旁边**：叶子的 `draw` 把位图选进 memory DC，`defer` 在它返回之前选回旧对象，`readBack` 在 `draw` 返回之后才调 `GetDIBits`；`ReleaseDC` 的 `defer` 传的是 `GetDC` 时的那扇窗口；`GetDIBits` 读回的行数不等于高就答 `ShortRows`，不交出半张图。空句柄在 `GetDC` 之前就答 `NoWindow`，因为 `GetDC(NULL)` 取的是整屏。三条的模型性质在 `crates/desktop/ffi/Spec.lean`（`every_gdi_object_is_given_back_once`、`read_back_follows_unselect`）。
9. **每个常量只写一处**：滚轮一格的 `WHEEL_DELTA` 用 `windows` 绑定里的定义；`PW_RENDERFULLCONTENT`、`CF_UNICODETEXT`、`GMEM_MOVEABLE`、`HWND_MESSAGE` 只写在 `leaf.zig`，Rust 一侧不再拼写它们；DPI 感知值由本 package 从 `windows` 绑定的 `PROCESS_PER_MONITOR_DPI_AWARE` 读出、传给叶子。

下面十个设计各记一个被选中的做法与它击败的备选。

**设计一（错误码住哪）**：在本 package 重新定义同拼写的一小组（落选）vs `RefusalCode` 映到 `kernel::AxCode`、拼写只取 `AxCode::as_str`（选中）。`RefusalCode` 留着，因为它是本 server 能答的那六个码的封闭子集，并且带着 JSON-RPC 的数；它不再带字符串。落选方案是一个事实的两处定义，靠一道门盯着才不漂；本 package 在工作区里，锁只有一份，依赖 `kernel` 不再多解一遍任何东西（D14）。新码仍要先进 `kernel::error::code`，因为那是唯一能写下它的地方。

**设计二（unsafe 怎么关）**：叶子也继承 `unsafe_code = "forbid"`（落选）vs 本 package 继承 `forbid`、只有 `desktop_ffi` 用 `deny`（选中）。`forbid` 在文件内无法就地放开，而叶子的每次调用要就地放开、并在那一处写明理由；`deny` 让放开成为**一个带理由的、看得见的、最窄作用域的例外**，而且只在叶子那一层。clippy 那张表逐行相同，一条不减。协议壳与平台臂**一行 unsafe 也不写**。

**设计三（整屏怎么办）**：默许整屏截取（落选）vs 无表达即拒（选中）。scope 文件能表达的只有「哪些窗口」，一张全屏图会显示 allowlist 没有列出的一切；把没写下来的东西当成允许，正是 fail closed 要防的那件事。拒词里给的替代是「指名一个窗口」，可执行。等 scope 文件长出一位 `screen` 开关，这条再改，改时先改本节。

**设计四（未实现怎么回答）**：先回一个假的成功形状让上游先接线（落选）vs 回 `E_TOOL_UNAVAILABLE` 并说明这个 build 里没有它（选中）。一个假的成功会让模型据此往下推理，而错误的答案比没有答案贵得多；全部价值就是**形状已经定死、拒绝是诚实的**。

**设计五（截图怎么取）**：DXGI Desktop Duplication（落选）vs `PrintWindow`（选中）。DXGI 复制的是**整个输出**，而这台 server 的 scope 文件说的是「哪些窗口」；用一个整屏机制去实现一件按窗口授权的事，等于把 §10 设计三刚关上的门从背面打开。`PrintWindow` 带 `PW_RENDERFULLCONTENT` 直接向一个 `HWND` 要它自己的像素，授权单位与机制单位因此是同一个。代价写在明处：某些用 DirectComposition 独立合成的窗口会回一片黑，那时的答案是**拒绝并说出来**（`E_TOOL_UNAVAILABLE`，全黑像素是可判的），恒不把一片黑当成截图交出去。录制走同一个入口（D7）：每一帧都取自 `capture::window`，再经管道交给 ffmpeg，所以「授权单位与机制单位是同一个」这句话对录制也成立。全黑判为失败是一条产品策略，不是对捕获失败的完美识别：一扇真的全黑的窗口也会被拒，拒词给出不经像素的 `desktop.snapshot`。

**设计六（快照的 ref 拿什么撑住）**：跨调用持有 `IUIAutomationElement` 这个 COM 指针（落选）vs 只留下快照当时的**屏幕矩形**（选中）。前者让 COM 对象的生存期缠上连接的生存期，而一次 `desktop.act` 需要的其实只有「点哪里」。选中方案让 COM 完整地关在 `tree` 一次调用之内，`act` 只面对整数坐标；generation 这一条的确切含义见 D5：一代 ref 属于一扇窗口（按句柄，不按标题），并且只在这扇窗口的矩形与快照时相同的前提下有效；窗口挪动或改了尺寸，旧的一代被拒。窗口内部重排而外框不动时，这一代仍被接受——这是剩余限制，不是已经解决的事。

**设计七（`desktop.windows` 报的 ref 是什么）**：让它成为 `snapshot`／`act` 也接受的第二种指名方式（落选）vs 只作为这条连接内一个窗口的**稳定叫法**（选中）。scope 判定读的是 `title` 与 `process`（`Reach`），一个绕过它们的 ref 就是同一份许可的第二道门——而两道门里一定有一道最后没人看。工具表是定死的，`snapshot`／`act` 的 schema 里本来也没有窗口 ref 这一项；本节记下的是**为什么不去加它**。ref 里恒不含 `HWND` 的数值：句柄是运行中的机器的内部事实，模型没有一处用得上它。

**设计八（没有 ffmpeg 时录什么）**：宣告录制不可用（落选）vs 自己抓一列 PNG 帧（选中）。工具卡片明写了「有 ffmpeg 出 mp4，没有则出帧序列」，而帧序列要一个**在读循环之外**跑的东西——本 package 因此有且只有一个 `std::thread::spawn`，在 `record::sink`：这条线程抓帧，交给 ffmpeg 的 stdin，或写成 PNG，由一个 `AtomicBool` 停下，`stop` 恒 join 它。这是本 package 唯一一处并发，写在这里是为了下一个读者不必去找第二处。声音（`audio: true`）恒被拒：选一个录音设备要知道运行中的机器上它叫什么，而这台 server 没有任何一处知道；假装录了而没录，比拒绝贵。能知道它的只有人，所以设备名只从 scope 文件来（D11）。

**设计九（城里的事实怎么读）**：抄一份、由 `xtask guard` 比对（落选）vs 直接引用唯一定义（选中）。错误码、质量的域、协议修订与 `_meta` 的键 `sprawling/effect-unknown` 都是城里定下的事实，本 package 读 `kernel::AxCode`、`kernel::consts_policy::IMAGE_QUALITY`、`agent_protocols::PROTOCOL_VERSION` 与 `agent_protocols::EFFECT_META_KEY`。抄件加比对只能让漂移**可见**；引用让它**不可能**，而且 `RefusalCode::code` 是一个对两侧穷尽的 match，每个变体映到哪个 kernel 码由编译器读出，不再只是拼写存在（D14）。

**设计十（一次 `act` 落在哪个窗口上）**：信任 `scope` 已经判过的那个窗口（落选）vs 发事件之前核对前台窗口，不是它就拒（选中）。`SendInput` 不带窗口：一次按键落在**那一刻**持有键盘的窗口上，而 scope、allowlist 与拒词判的是 title 与 process，这些没有一样跟着事件走。决定动作与发出动作之间隔着一段时间，操作者按一次 Alt+Tab、一个提权对话框弹出来，键盘就在别人手里了——`type` 会把整段文字打进那个窗口，密码框也包括在内；今天全仓 grep `SetForegroundWindow`／`GetForegroundWindow`／`WindowFromPoint` 零命中，故 scope 实际约束住的只有坐标的算法。选中方案是：`windows::focus` 读一次前台窗口，不是它就请求置前并在 200 ms 内有界地重读，仍不是就以 `E_TOOL_UNAVAILABLE` 拒，拒词写明**什么都没有发出去**；指针动作另问第二句——落点下面的顶层窗口也得是它，因为一个窗口可以持有键盘而另一个盖在点击处。比较的是句柄地址这一个纯值，于是这条规则在一台没有桌面的机器上也能逐条证明（§16.2）。付的代价写在明处：置前是一次**副作用**，而它是这台 server 唯一一处主动改变桌面的排布；把它藏起来的做法是不置前直接拒，那会让每一次正常的连续操作都要操作者手动切窗口。

D4 部分输入：如实报数，只补抬起。

- **决定**：`SendInput` 只收下前 k 个事件（0 < k < n）时，按已收下的前缀算出仍按住的键、Unicode 单元与鼠标键，按按下的相反次序补发一次它们的抬起；从不补发按下，从不重发整批。答复是 `E_TOOL_UNAVAILABLE` 的拒绝，标效果未知，写出 k／n 以及补发的抬起是否全部被收下。k = 0 时什么都没发出去，拒词照实说，不标效果未知。
- **理由**：微软只承诺按序插入、返回插入的个数，不承诺回滚；前缀可能已经落在窗口上，修饰键可能停在按下状态，人的桌面随后每一次按键都会带着 Ctrl。补发抬起只会松开本次按下的东西，不会让动作多做一步；重发整批会把已经落地的点击或文字再做一遍。
- **击败的备选**：只报告、不补发（桌面可能停在 Ctrl 按下，人要自己找原因）；重发整批（重复点击、重复输入）。
- **重开的参数**：换成能按元素投递的接口（UIA patterns）之后，部分投递不再是这个形状。

D5 generation 按窗口身份与快照时的矩形失效。

- **决定**：`Views` 以窗口句柄（`focus::Aim`）为键，不以标题为键；一份快照记下这扇窗口当时的外框矩形。`resolve` 同时比对句柄、generation 与当前外框，外框不同即按过期拒绝。
- **理由**：按标题记，两个进程的同名窗口共用一份记录，对 A 做的快照会让对 B 的动作取 A 的坐标；窗口挪动而不重新快照时，ref 的屏幕坐标指向挪动前的位置。外框是本 package 每次调用本来就读的值，比对它不多一次 Win32 调用。
- **剩余限制**（写明，不当作已解决）：窗口内部重排而外框不动时，旧的一代仍被接受；句柄在窗口关闭后可能被新窗口复用，新窗口恰好外框相同时同样检测不到。
- **击败的备选**：持有 UIA 元素跨调用（§10 设计六）；每次动作前重读整棵树比对（一次动作的代价变成一次快照）。
- **重开的参数**：换成按元素投递（UIA patterns），ref 改指元素而不是矩形。

D6 坐标约定：虚拟桌面上的物理像素。

- **决定**：连接器建 `Desk` 时以 `SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE)` 声明按显示器感知；声明失败时读回 `GetProcessDpiAwareness`，已是按显示器感知就接受，不是就让每一次调用以 `E_TOOL_UNAVAILABLE` 被拒。此后枚举的外框、UIA 的矩形、落点检查、截图尺寸与输入的归一化都是虚拟桌面上的物理像素。
- **理由**：不声明时，缩放显示器上 `GetWindowRect` 与 `GetSystemMetrics` 给的是虚拟化后的坐标，UIA 给的是物理坐标，一次按 ref 的点击会落在别处。声明只作用于 `sprawling desktop` 这个子进程，城的进程不受影响。用 Windows 8.1 起就有的 shcore 接口而不是 1703 起的 per-monitor v2 context 接口：本 package 链在 `sprawling` 里，一个缺失的 user32 导出会让整个二进制在更老的 Windows 上起不来；本 server 不画窗口，v2 多出的子窗口与非客户区缩放用不上。
- **击败的备选**：在 `sprawling.exe` 的 manifest 里声明（城的进程也被改，本 package 自己的测试进程享受不到）；不声明（两种坐标混算）。
- **重开的参数**：本 package 开始画自己的窗口，或最低支持的 Windows 版本升到 1703 以上。

D7 录制经同一个捕获入口。

- **决定**：录制的唯一那条线程按 `FRAME_EVERY` 调 `capture::window` 抓帧。PATH 上有 ffmpeg 时，帧以 `-f rawvideo -pixel_format rgba -video_size <宽>x<高> -framerate 10 -i -` 从 stdin 交给它；`spawn` 报「找不到程序」时写 PNG 序列；`spawn` 报别的错时 `start` 被拒。抓帧失败、写帧失败或 ffmpeg 提前退出，录制停下，`stop` 照常交出路径并说出写了多少帧、为什么停。结束 ffmpeg 靠关掉它的 stdin，它读到结尾自己写完索引退出。
- **理由**：按标题交给 gdigrab 时，ffmpeg 以 `FindWindowW(NULL, title)` 重新找窗口，不看进程，像素来自 `BitBlt` 而不是 `PrintWindow`，也不过全黑判定；一个按 title 加 process 选中的窗口可能在这里换成另一扇同名窗口。
- **击败的备选**：gdigrab 的 `hwnd=`（要 FFmpeg 7.0 以上，像素仍来自 `BitBlt`，版本过老时本 package 还探测不出来）；保持按标题（与 §10 设计五矛盾）。
- **重开的参数**：`PrintWindow` 每帧的代价让 10 fps 做不到，届时再比较按句柄的 gdigrab。

D8 剪贴板：自己的 owner，有界的读，先备好再清空。

- **决定**：每次打开剪贴板时建一扇仅消息窗口（预定义类 `STATIC`，父窗口 `HWND_MESSAGE`）作为 owner，交给 `OpenClipboard`；`Held` 结束时先关剪贴板，再销毁这扇窗口，最后放掉进程内的轮次锁。读：以 `GlobalSize` 求出块的上界，在界内找终止符，找不到就取整块；`GlobalLock` 失败是拒绝。写：先分配并填好整块，再打开、清空、交出；交出失败时本进程释放这块内存，拒词说明剪贴板已被清空。
- **理由**：微软写明以空窗口打开时 `EmptyClipboard` 把 owner 置空，随后的 `SetClipboardData` 会失败；块是别的程序写的，「它会以 0 结尾」可能为假；把锁失败当成「没有文本」，调用方会去试别的而不是重试；先清空再分配，分配失败时剪贴板已空，拒词却只说内存不够。这扇窗口只活一个剪贴板轮次、在同一线程建和毁，本 package 不依赖它收到任何消息，所以不需要消息泵。
- **击败的备选**：`OpenClipboard(None)`（与文档冲突）；遇到无终止符的块就拒（对一个格式不规范的程序，整块文字仍然可读，界已经由 `GlobalSize` 守住）；WinRT Clipboard（要求前台与 UI 线程，后台子进程是否适用未证）。
- **重开的参数**：出现一个能证明 owner、界与所有权移交的安全封装。X2 审过的 `winsafe` 0.0.29 还不是，理由见 D9。

D11 声音：ffmpeg 的 `dshow` 录 scope 文件点名的那一个设备。

- **决定**：`desktop.record` 的 `audio: true` 录的是人写在 scope 文件里的那一个 DirectShow 音频设备，键为 `sound = "<设备名>"`，与 `record`、`clipboard` 同样缺省不写。设备名只从 scope 文件来：调用方的参数里没有它，本 server 也不枚举设备。scope 文件写了 `record = true` 而没写 `sound` 时，`audio: true` 以 `E_GATE_DENIED` 拒，恢复语说出去哪里查设备名（`ffmpeg -hide_banner -list_devices true -f dshow -i dummy`）与写进哪一键。声音由第二个 ffmpeg 进程录，`-f dshow -i audio=<设备名>`，写成录制目录里的一个 16 kHz 单声道 wav，与画面各成一个文件；没有 ffmpeg 时，有声音的录制以 `E_TOOL_UNAVAILABLE` 拒，因为帧序列那一支录不了声音。
- **理由**：主线 ffmpeg 在 Windows 上的音频输入是 `dshow`（`ffmpeg -devices` 列得出），不加依赖、不加 `unsafe`。dshow 设备的名字是驱动起的，随机器而变，城里没有一处知道；替人挑一个，就是替人授权录下一个他没点名的声音源，而麦克风是比一扇窗口更重的授权。人写下名字，就是人授权了这一个设备，与 `windows` 列表授权窗口是同一种读法。画面与声音分成两个进程、两个文件：画面从 stdin 进（D7），声音是 ffmpeg 自己的输入，放进一个进程就要对齐两条时钟；wav 是 `gateway::AudioType` 认得、城的 `transcribe` 工具直接收得下的容器（sprawling-SPEC 8-131），16 kHz 单声道一分钟约 1.9 MB。
- **击败的备选**：WASAPI 回环（能录机器正在放的任何声音，但要新依赖或新 `unsafe`）；枚举设备取第一个（替人选了授权对象）；由调用方在参数里给设备名（模型给出授权对象，scope 文件就不再是授权的唯一处）；把声音混进 mp4（两条时钟，而没有 ffmpeg 的那一支本来就没有声音可混）。
- **重开的参数**：主线 ffmpeg 在 Windows 上有了 WASAPI 输入；或者 scope 文件要按窗口给不同的设备。

D15 `initialize` 之前的 `notifications/initialized` 被拒，阶段不动。

- **决定**：`Fresh` 收到 `notifications/initialized` 留在 `Fresh`；作为通知发来时照旧无答案，带 `id` 时答 `E_GATE_DENIED`，与握手未完成时 `tools/list` 得到的是同一句拒词。`Initializing` 与 `Ready` 收到它进 `Ready`。这一步转移只写在 `Phase::opened`（`session.rs`），`spec/Session.lean` 的 `afterOpened` 与它同形，`fresh_until_initialize` 证明一个没发过 `initialize` 的连接恒在 `Fresh`。
- **理由**：MCP 的生命周期是 `initialize` 在先、这条通知在后（§5 第一行）。一个只发通知的客户端没有协商协议修订，也没读到能力表，它拿到的工具表按一个它不知道的修订写成；旧的转移让这条通知在任何阶段都开门，§10 第 2 条写下的次序就只强制了一半。
- **击败的备选**：①照旧放行（次序只强制一半）；②`Fresh` 收到它就进 `Initializing`（替客户端补了一次它没发的 `initialize`）；③断开连接（一次次序错误就断线，而之后正确的 `initialize` 与任何时候的 `ping` 仍该得到回答）。
- **重开的参数**：MCP 的新修订允许不经 `initialize` 开始一个连接。

**成本：模型读到的是什么。** 工具名恒是 `desktop.<动词>`，模型一眼看得出这一件事发生在桌面上而不是页面里。一扇窗口读起来是一段缩进的大纲，每行一个元素和它的 ref，模型读完就能指名要点的那一个。每条说明的后半句写的是**不做什么**，因为模型下一步最贵的错误是把一件工具当成它旁边那件。拒词恒给一个可执行的下一步：越界给「把这个窗口写进 `DESKTOP.toml`」，未实现给「这个 build 里没有它」。拒词全文作为文字到达模型：城这一侧把 `isError` 结果读成一次失败，文字原样进它的 subject；部分输入另带效果未知，模型读到的是「先看一眼再动」，不是「可以重试」。
-/

/-! ## 11 边界枚举

非 JSON 行／非对象／无 `method`／`params` 非对象／未知方法／握手未完成就调用／`tools/call` 无 `name`／`name` 不在表里／scope 文件缺失／scope 文件语法坏／scope 文件有拼错的键／allowlist 两张表皆空／title 不匹配／process 不匹配／同时给 title 与 process 而只中一个／既不给 title 也不给 process／`record` 未开／`clipboard` 未开／整屏截取／非 Windows 平台／Windows 但本 build 未实现／键盘在别的窗口手里且置前请求没有生效／指针动作的落点被别的窗口盖住。一批输入只被收下一部分／补发的抬起也被挡／窗口快照后挪动或改了尺寸／两扇同名窗口／进程已被设成别的 DPI 感知／剪贴板块没有终止符／剪贴板块锁不住／剪贴板清空之后交不出新文本／ffmpeg 起不来或中途退出／录制中窗口不再能抓帧／要声音而 scope 文件没写 `sound`／要声音而运行中的机器上没有 ffmpeg／声音设备打不开或录到一半退出／录下的声音比一次答复带得下的长。

**同时给两个标识则两个都要中**：任何一个给出的标识都要落在它自己那张表里，这是 fail closed 的一致读法。
-/

/-! ## 12 错误处理

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_WIRE_MISMATCH` | 行不是 JSON、不是对象、无 `method` | 不能：对侧发什么不由本 package 决定，fail closed |
| `E_TOOL_UNKNOWN` | 未知方法、`name` 不在六张卡片里 | 不能：模型会试不存在的名字 |
| `E_INVALID_ARGS` | `params` 形状读不出、`tools/call` 无 `name` | 部分能：schema 已给出，拒词指到那一处 |
| `E_GATE_DENIED` | 越界、`record`／`clipboard` 未开、整屏截取、握手未完成 | **能**（对 scope 而言）：缺文件即 `Closed`，于是「默许」这件事不成立 |
| `E_CONFIG_INVALID` | scope 文件语法坏 | 不能：文件是人写的。坏文件恒关成全拒，恒不退回默认允许 |
| `E_TOOL_UNAVAILABLE` | 非 Windows 平台；Windows 但本 build 未实现；桌面不收输入、只收下一部分（后者效果未知）；DPI 感知声明不成 | 不能：这是运行中的机器与这个 build 的事实 |

拒词恒是三段（three-part refusal）：拒了什么（action）、为什么（subject）、还能做什么（recovery）。工具名认出之后的拒绝写进 `isError` 结果的文字：第一行 `E_…: cannot <action> — <subject>`，第二行 `instead: <recovery>`；协议层的拒绝装进 JSON-RPC error 的 `data`，`message` 是第一行那句摘要。

D2 工具自己的拒绝以 `isError` 结果回答。

- **决定**：`tools/call` 在工具名认出之后的一切拒绝——scope 不准、参数读不了、平台做不到——都答 `CallToolResult`，`isError: true`，`content` 是一块文字，写三段拒词。握手未完、方法或工具名不认识、行读不出、`params` 不是对象，这几类才回 JSON-RPC error。
- **理由**：MCP 2025-06-18 规定工具自己的错误放进结果、置 `isError`，协议层错误留给协议本身。城的客户端只读 JSON-RPC error 的 `code` 与 `message`，拒词里的恢复句到不了模型；放进结果，全文作为文字到达。
- **击败的备选**：继续回 JSON-RPC error，让城读 `data.code`、`data.recovery`：那是规格外的约定，城要为每一家 server 猜一次 `data` 的形状。
- **重开的参数**：MCP 的新修订改了工具错误的答法。

D3 效果未知写在 `_meta` 的一个命名空间键里。

- **决定**：一次拒绝若发生在请求已交出一部分之后（今天只有部分输入，D4），`isError` 结果带 `_meta: { "sprawling/effect-unknown": true }`。键的唯一定义在 `agent_protocols::mcp::tools::EFFECT_META_KEY`，本 package 引用它（D14）。
- **理由**：「效果未知」要落成城里的 `Retry::Unknown`，它决定模型与人会不会把一次可能已经点下去的动作再做一遍；写进文字，城只能把它当一句话，读不出这个三态。`_meta` 是规格留给实现附加元数据的位置，键名按规格带自己的前缀，不占 `mcp` 的保留前缀。
- **击败的备选**：放进 JSON-RPC error 的 `data.retry`（与 D2 冲突，也是规格外）；只写进拒词文字（城标不出 `Retry::Unknown`）。
- **重开的参数**：MCP 规格为「副作用未知」定了自己的字段。
-/

/-! ## 13 依赖选型

三个 workspace crate（`kernel`、`agent_protocols`、`desktop_ffi`，§7、D14），加八个外部依赖。`serde`、`serde_json`、`toml`、`base64` 经 `workspace = true` 取工作区的版本行，只做协议、scope 文件与 image content 的读写；另外四个各自买到什么，写在下表，后三个与 `desktop_ffi` 只在 Windows 上链接。**恒不引入**：async runtime、HTTP 客户端、glob crate（§10 第 4 条）。

| crate | 买到什么 | 为什么不是别的 |
|---|---|---|
| `desktop_ffi`（同一工作区，§8-12） | 枚举、捕获、剪贴板、DPI 四组调用的安全函数，由 Zig 叶子做完 | 手写 `windows` 绑定的 `unsafe`（口径 ① 要的正是业务代码零 `unsafe`，D12） |
| `windows` 0.62 | 绑定里的常量（`WHEEL_DELTA`、DPI 感知值）、交给 `uiautomation` 的 `HWND`、Win32 错误的文字；生产代码不调它的任何函数，测试用它建自己的窗口 | `windows-sys` 只有裸函数；`uiautomation` 本身也链接同一版 `windows`，锁里不多一个包 |
| `winsafe` 0.0.29（只开 `user`、`ole`） | 输入与窗口事实的安全接口、COM 公寓的守卫（D9、D10），以及缝上的 `HWND`、`co::ERROR`、`co::HRESULT` | 没有 Cargo 依赖；它的 `EnumWindows` 与剪贴板写法不准入，理由在 D9 |
| `uiautomation` 0.25.1（`default-features = false`，只开 `input`） | UIA 树的安全封装：automation 对象、walker、元素属性（D10） | 关掉默认特性，于是它的截图、剪贴板与控件匹配都不进来；`input` 只因它的 core 模块不开就编不过而开着，本 package 不调它；手写 COM 调用是被它换掉的那九个 `unsafe` 块 |
| `image` 0.25（`default-features = false`，只开 `png`／`jpeg`／`webp`） | `desktop.screenshot` 点名的三种编码，以及缩放 | 关掉默认特性是因为本 package 只编码、从不解码，也不碰另外十种格式 |
| `base64` 0.23 | image content 的 `data` 那一层编码，只在 `answer` 里编 | 与 workspace 的 `gateway::dialect::images` 同一条版本行（`workspace = true`） |

`windows` 与 `winsafe` 的 feature 列表本身就是一份**够得着范围的声明**：一个本 package 从不调用的 API，在这里连名字都拼不出来。

D9 输入与窗口事实经 `winsafe`；枚举、捕获、剪贴板、DPI 仍走 FFI。

- **决定**：输入（`SendInput`、`GetSystemMetrics`）与窗口事实（可见、标题、进程映像名、外框、前台、落点、置前）改经 `winsafe` 0.0.29 的安全接口，只开 `user` 与 `ole` 两个 feature。键码以 `co::VK` 常量写在 `keys` 表里，一个动作的事件在 Rust 里构造成 `HwKbMouse` 值，整批一次交给 `winsafe::SendInput`，它回的收下个数照 D4 读。窗口句柄只在 `enumerate` 的 `EnumWindows` 回调里铸成 `winsafe::HWND`（§8-11），那是这一组唯一新写的 `unsafe`。
- **理由**：这几个调用的前提（出参缓冲的长度、句柄的借用、`INPUT` 数组的元素大小）由 `winsafe` 的签名承担，本 package 只剩值；`winsafe` 没有 Cargo 依赖，`user` 不拉 GUI 那一层。
- **不准入的四组与各自的理由**（它们今天经 §8-12 的 Zig 缝）：`winsafe::EnumWindows` 把 `&func` 当作地址交出去，回调里再从它造出 `&mut F`（`src/user/funcs.rs` 与 `src/user/callbacks.rs`），这是从共享引用造可变引用，属未定义行为，所以枚举留在 `windows` 绑定上；以 `FindWindowEx` 或 `GetWindow` 逐个取顶层窗口可以不写 `unsafe`，但 z 序在两次调用之间变化时会漏掉窗口或兜圈，而 `EnumWindows` 先取快照再回调，故落选。剪贴板：`HCLIPBOARD::SetClipboardData` 在调用方已经清空剪贴板之后才分配内存，交出失败时那块内存被 `leak` 而不释放，与 D8「先备好再清空、交不出就释放」相反；建 owner 窗口的 `CreateWindowEx` 在 `winsafe` 里本身是 `unsafe`。捕获：`winsafe` 没有 `PrintWindow`，`GetDIBits` 是 `unsafe`。DPI：`SetProcessDpiAwareness` 在 shcore 里，`winsafe` 没有它。这四组由 §8-12 的 Zig 缝做完（D12）。
- **剩余限制**（写明，不当作已解决）：`SendInput` 仍只报收下几个；换接口不改变 §10 设计十的前台检查只是一次采样这件事。
- **击败的备选**：把这几组一起搬进 Zig 缝（有合格安全接口的调用不需要一道新的 FFI 缝）；留在 `windows` 绑定上继续手写 `unsafe`（口径 ①要的正是业务代码零 `unsafe`）。
- **重开的参数**：`winsafe` 修好 `EnumWindows`，枚举这一行随之换过去；或者新版本改了这里准入的签名，那一行的契约测试先红。

D10 树经 `uiautomation` 读、按文档序铸 ref、折成文字；COM 公寓每张桌子进一次。

- **决定**：`tree` 经 `uiautomation` 0.25.1 读树，默认 feature 关掉，只开 `input`（它的 core 模块不开 `input` 编不过），只用 `UIAutomation::new_direct`、`element_from_handle`、control view walker 与元素的三项属性（role、name、外框）。COM 公寓由 `Desk` 在第一次 snapshot 时进入一次，用多线程公寓（MTA），经 `winsafe::CoInitializeEx` 的守卫；automation 对象与 walker 同这个守卫一起住在桌子里，字段的析构次序保证先放 COM 对象、后退出公寓。元素仍只活在一次 snapshot 之内（§10 设计六）。walker 答「没有这个元素」（错误码 0）是这一层到头；答别的错误是 provider 出了故障，遍历在那里停下，而不是当作到头。
- **理由**：微软对不开窗口的 UIA 工作线程推荐 MTA；进一次、配对退出，公寓的生存期就是连接的生存期。旧写法每次 snapshot 都以 STA 进入而从不退出，并且每次新建 automation 对象；`uiautomation::UIAutomation::new()` 同样每次进入而不退出，故不用它。把 provider 的故障读成「到头了」，模型拿到的是一棵看起来完整、其实缺了一块的树。
- **剩余限制**（写明，不当作已解决）：`winsafe` 的 `CoUninitializeGuard` 在进入公寓答 `RPC_E_CHANGED_MODE` 时也会调 `CoUninitialize`；本 package 的读循环线程不进入任何别的公寓，这条路走不到。剪贴板走的是 Win32 剪贴板而不是 OLE 剪贴板，同一线程上的 MTA 与它无关。
- **击败的备选**：保留经 `windows` 绑定手写的 COM 调用（九个 `unsafe` 块）；让 UIA 也回答窗口事实（UIA 根的子元素、`IsOffscreen` 与 `EnumWindows`、`WS_VISIBLE` 不是同一个定义，安全判断会跟着 provider 的实现走）。
- **重开的参数**：需要 UIA patterns（按元素投递动作）时，那是另一项能力，另写一节；或 `uiautomation` 的新版本改了这里用到的签名，`a_windows_tree_names_the_control_inside_it` 先红。
- **(b) 读给模型的是文字**：`desktop.snapshot` 的答复是 `outline` 折成的一段文字，不再是每个节点一个带 bounds 的 JSON 对象。形状照 `browser::snapshot` 的折法（名字不照抄）：一行一个元素，`e<n> <role> "<name>"`，按深度缩进两格；名字去控制字符、压空白、截到 80 个字符。role 取 control type 的编号，映到一张封闭词表，词就是 UIA 自己的类型名的小写；本地化字符串随机器的语言变，同一个按钮在两台机器上会是两个词。没有名字的 pane、group、custom、separator 不占一行也不铸 ref，它们的子元素照读。遍历按文档序：先父后子，兄弟按窗口列出的次序；旧写法用栈，同一层倒着铸 ref。遍历提前停下（ref 到 500 个，或 provider 出了故障）时，文字最后一行说出停在哪里、为什么。
- **(b) 理由**：模型读一行一个元素的文字比读嵌套 JSON 快，bounds 对模型没有用处，`desktop.act` 按 ref 从 `Views` 取；一个可读的词表比一串编号或一个随语言变的字符串更好对照。
- **(b) 击败的备选**：继续回每个节点的 JSON（答复是现在的四五倍长，bounds 占去一半）；另造一套平台无关的 role 词（在只有一个平台的今天，它只是一层没有第二个读者的翻译）。
- **(b) 重开的参数**：macOS 臂落地时 AX 的 role 映不进这张词表。
-/

/-! ## 14 硬编码声明

协议修订是 `agent_protocols::PROTOCOL_VERSION`，本 package 引用它而不写第二份，因为两端要谈得拢。服务器自称 `sprawling-desktop`，版本取 `CARGO_PKG_VERSION`。

常数，每条都写清它是谁的事实：

| 常数 | 值 | 谁的事实 |
|---|---|---|
| 快照默认深度 | 8 层 | 我们的选择：再深一层的 UIA 树，模型读到的东西开始多过它用得上的 |
| 一次快照最多铸的 ref 数 | 500 | 我们的选择：一份读不完的树等于没读 |
| 大纲里一个名字最多的字符数（`outline::LABEL_MOST`） | 80 | 我们的选择：够分辨两个控件，一扇窗口的大纲仍是一页；再长的名字多半是一段正文，模型要读它就截图或读剪贴板 |
| 大纲每深一层的缩进 | 两个空格 | 我们的选择：最省字符、仍一眼看得出层次的缩进 |
| 截图默认格式／`scale` | `png`／100 | 我们的选择：默认不损、不缩，缩放是调用方明说才发生的事 |
| `quality` 的域 | 0..=100 | 城里的域（`kernel::consts_policy::IMAGE_QUALITY`），本 package 以它的 `admit` 判，截图 schema 的 `maximum` 也经 `admit` 读出（D16），不写第二个数。域外即拒：一个要求 120 的调用方以为自己要多好就有多好，而替它填默认值是在回答另一个问题 |
| `scale` 与城里那个同名量的区别 | 本包的 `scale` 是窗口自身像素的百分数 | 城里 `browser` 的 `scale` 是设备像素比（devicePixelRatio）的百分数——同一个词、两个量，**不是同一个事实**，所以两边的域也不必相同 |
| `webp` 忽略 `quality` | —— | 外面的事实：`image` 的 WebP 编码器是**无损**的，故 `quality` 对它无意义。schema 允许同时给出，本 server 恒不因此报错，而在答复里写明这一次的编码是无损的 |
| 帧序列的抓帧间隔 | 100 ms（10 fps） | 我们的选择：`PrintWindow` 一帧的代价决定了上限，而 10 fps 足够看清一次交互 |
| 声音的格式 | 16 kHz、单声道、16 位 PCM 的 wav，一分钟约 1.9 MB | 外面的事实与我们的选择：wav 是 `gateway::AudioType` 认得的容器；转写端点要的是语音，16 kHz 单声道足够，再高只多字节（D11） |
| 一次答复里最多带多少声音（`SOUND_CARRIED_MOST`） | 4 MiB，约两分钟 | 我们的选择：base64 让它长三分之一，整行仍在城的 MCP 客户端那 8 MiB 的单行上限之内；更长的录音只交路径（D13） |
| 声音的收尾 | 往第二个 ffmpeg 的 stdin 写 `q`，再等它自己退出 | 外面的事实：ffmpeg 读到 `q` 才写完 wav 的头；直接杀掉会留下一个长度字段为零的文件 |
| 录制落盘的去处 | `std::env::temp_dir()/sprawling-desktop/<窗口名安全化>-<序号>` | 我们的选择：scope 文件说的是「可以碰哪些窗口」，没说「可以往哪写文件」，故恒不写进城里，也恒不写进操作者的家目录 |
| ffmpeg 的收尾 | 关掉它的 stdin，再等它自己退出 | 外面的事实：帧从 stdin 进，读到结尾时 ffmpeg 写完 mp4 的尾部索引；直接杀掉会留下一个播放不了的文件 |
| 全黑像素判为失败 | —— | 外面的事实：`PrintWindow` 对某些独立合成的窗口回全黑。依据是「每一个像素的 RGB 三通道皆为 0」 |
| 效果未知的 `_meta` 键 | `sprawling/effect-unknown`，值 `true` | 我们的约定；唯一定义在 `agent_protocols::mcp::tools::EFFECT_META_KEY`，本 package 引用它 |
| 工具层拒词的文字 | 第一行 `E_…: cannot <action> — <subject>`，第二行 `instead: <recovery>` | 我们的选择：第一行与 JSON-RPC error 的 `message` 同一句，第二行让恢复句在模型读到的文字里有自己的位置 |
| DPI 感知 | 按显示器感知（`PROCESS_PER_MONITOR_DPI_AWARE`，取自 `windows` 绑定，交给叶子） | 外面的事实：Windows 8.1 起的 shcore 接口；理由见 D6 |
| Zig 的版本 | `crates/desktop/ffi/zig-version` 里的那一行 | 我们的选择：叶子只对一个 Zig 版本编过、测过；换版本只改那一个文件（`crates/desktop/ffi/Spec.lean` D3） |

模型里的 `LABEL_MOST` 与 Rust 的 `outline::LABEL_MOST` 取同一个值，由 `outline` 模块旁的测试守着。

D16 截图 schema 的 `quality` 上限经 `IMAGE_QUALITY.admit` 读出。

- **决定**：`desktop.screenshot` 的 `inputSchema` 里 `quality.maximum` 是 `kernel::consts_policy::IMAGE_QUALITY` 收下的最大值，由 `tools::largest_quality` 在 `admit` 上二分找出：`admit` 的域是 `0..=max`（`crates/kernel/spec/PolicyLimit.lean` §8-73），所以「收下」对质量单调，三十二次判定之内找到边界。一次调用的质量仍在解析点经 `admit` 判（`platform::windows::reading`）。
- **理由**：schema 是模型读到的域，`admit` 是本 server 执行的域。schema 里写死的 100 是同一个事实的第二份定义：城里的域一挪，模型照旧按旧域发，server 按新域拒。kernel 的 `ImageQuality` 没有 getter，那是 kernel D3 的定规（拒因只由类型说出，调用方读不出裸数去拼自己的句子）；schema 的上界不是一句拒因，经类型唯一的门读出它，定规不动。
- **击败的备选**：①在 `consts_policy` 加一个公开常量 `IMAGE_QUALITY_MAX`，`IMAGE_QUALITY` 由它构造——一个数一个家，代码最短，但它正是 12.3 这条定规关上的那扇门；②留 100、由评审盯着；③由 `xtask guard` 比对两处（设计九已否：抄件加比对只让漂移看得见）。
- **重开的参数**：kernel D3 改为准许读出上限类政策值的数；那时 schema 直接读那个数，`largest_quality` 删去。
-/

/-! ## 15 影响面

工作区成员，唯一的调用方是 `sprawling`：`crates/sprawling/Cargo.toml` 以 `desktop = { workspace = true }` 依赖它，`ARCHITECTURE.md` §3 的 depmap 块有它与 `desktop_ffi` 各一行。改 `kernel::AxCode` 那六个变体、`IMAGE_QUALITY`、`agent_protocols::PROTOCOL_VERSION` 或 `EFFECT_META_KEY` 波及本 package。它在 Windows 上按路径依赖 `desktop_ffi`，所以 Windows 上构建 `sprawling` 要有 `zig-version` 钉住的 Zig。改 `serve_stdio` 的签名波及 `bin::main::desktop`；改工具名或 `tools/list` 的形状波及 `kernel::gate::undoable`（按远端名前缀 `desktop.` 判）与 `crates/sprawling/tests/desktop.rs`。

### 15.2 城里那一侧

城里认这台 server 的四件事与各自的落点：

1. **`RULES.toml` 增一键 `desktop`**：住在 `city::policy`，与 `confidential`／`write`／`review` 同一处解析。缺省是**关**——一栋楼默认不把桌面交出去，理由与 `record`／`clipboard` 默认关是同一条。
2. **`desktop.` 前缀的 MCP 工具归到「撤不回」那道门**：一次点击没有 restoration，`kernel::discard` 那套「拿得回来才准删」在这里无从谈起，故它该走的是**升给人**（Escalate），不是 Deny。落点是 `runtime::bench::admit` 里 `Effect::Connector` 那一支。
3. **设置页写 `DESKTOP.toml`**：它是**治理文件**，不是产物——它说的是这栋楼的 runs 能碰什么。故它落在这栋楼的 reserved subtree（`<building>/.sprawling/DESKTOP.toml`），与 `RULES.toml`／`CONFIG.toml` 同处，**任何 write domain 都够不着**；写它的那一点照 `city::governed` 的形状办（一道门、整份写、不拼路径），而不是让设置页自己拼一个路径出来。这就是 `DomainReach` 立下的那条读法：一份决定「residents 能写什么」的文件，恒不由 resident 写。

4. **城自己起这台 server**：`RULES.toml` 写 `desktop = true` 的楼，城以 `sprawling desktop <DESKTOP.toml>` 起它，不要人手写 `[[mcp]]`（sprawling-SPEC §8-4d）。

#### 落到哪一步，还欠什么

四件都已落地并各自有测试：`city::policy` 读 `desktop:`（缺省关、机密楼即拒、打字错误即拒）；`kernel::gate::undoable` 判「远端名前缀 `desktop.`」并升给人，`runtime::bench::admit` 在出网门之后叫它；设置页的框经 `ConfigureBuilding` 的 `desktop` 字段整份写 `DESKTOP.toml`，写之前先落账本一行（sprawling-SPEC 里桌面 allowlist 那一节）；城按规则起 `sprawling desktop`。

**不内置任何模型（定规）**：本二进制不带 OCR、ASR 或任何视觉模型的权重，这些都经人接入的端点。模型拿到的桌面文字先是 accessibility tree；OCR 端点与本地 ASR 端点都是人接进来的。

**录音到 `transcribe`**：`desktop.record` 停下时把声音作为一块 audio content 交回（D13），连接器把它存进 CAS，模型读到的是一行带 `cas:` locator 的字（`crates/runtime/Spec.lean` §8-27-10、runtime D15）；`transcribe` 经 `runtime::BoundReader` 按这个 locator 读这一块，容器从开头的字节认（sprawling-SPEC 8-131，`crates/runtime/Spec.lean` §8-59，`crates/gateway/Spec.lean` §8-34）。

**还欠的**，都是这条接口的当前状态：

1. **OCR**：一张截图变成文字由城工具 `ocr` 承担（sprawling-SPEC 8-142），它读连接器存进 CAS 的截图，经人为 `ModelTag::Ocr` 选的端点；本 package 不做 OCR。
2. **macOS 这条胳膊**：`platform/elsewhere.rs` 对 macOS 答 `E_TOOL_UNAVAILABLE`。它要在一台 Mac 或夜间的 `platforms.yml` 上验，Windows 上验不了。
-/

/-! ## 16 测试与约束

逐模块 `#[cfg(test)]`。真的把进程拉起来的那一条住城里（`crates/sprawling/tests/desktop.rs`，§8-10）：它是唯一一处证明「城起的 `sprawling desktop` 真的接得上」的测试，其余测试都只证明库里的判断。

**约束**：`unsafe` 只在 `platform/windows/` 之下且每块携一行 `SAFETY:`（§8-9）；其余处恒不出现 `unsafe`、`unwrap`、`expect`、`panic!`、`todo!`、裸下标、`as`；算术走 `checked_*`／`saturating_*`；每个文件 ≤400 行、每个函数 ≤200 行且 ≤4 参数。`scope.rs` 因这条尺子而在 446 行处切出 `scope/pattern.rs`——切口落在「一行 allowlist 匹配什么」与「这份文件许可什么」之间，是语义的，不是为了凑行数；`Admitted` 落地时它再次抵线，这一次切出的是 `scope/tests.rs`（形状同 `session/tests.rs`），判定与对判定的断言各占一个文件。

验收命令：本 package 与 `desktop_ffi` 是工作区成员，`just clippy`、`just test` 与 `just fmt-check` 判它们，与判其余 crate 是同一条命令；Windows 上 `just check-desktop` 另跑 `zig test` 判叶子自己的测试（`zig fmt --check` 在 `just fmt-check` 里）。`xtask guard` 判 `desktop_ffi` 那一张自己的 lint 表，例外只有记下理由的 `unsafe_code` 一行（D14）。叶子的三份 `.zig` 与 `.rs` 受同样的 `xtask header`、`length`、`modmap`：MPL 头、函数 200 行与文件 400 行、模块表里各一行（tools/xtask/Spec.lean §8-48，ARCHITECTURE §2 条件 5）。`record.rs` 因 `xtask length` 的 400 行文件尺子在 443 行处切出 `record/sink.rs`——切口落在「可不可以开始录」与「由谁写、写到哪」之间。

`platforms.yml` 的 macOS job 跑工作区的 clippy 与 nextest，所以非 Windows 臂与只在非 Windows 上编译的测试（`session/tests.rs` 里 `#[cfg(not(windows))]` 的那一条）每晚在 macOS 上过一次。

### 16.2 怎么测一件需要桌面的事

本 card 的测试分两层，分界线就是 §8-8 那张表的最后一列：

- **不碰 Win32 的那几处逐条测**（`target`／`views`／`strokes`／`encode`／`keys`／`geometry`／`focus::settled`／`Admitted::visible`）。最容易错的几件事——选中了哪个窗口、过期或挪动过的窗口上的动作有没有被拒、一批输入被截断时还按着什么、一张图缩成什么尺寸、一个键名映到什么、键盘不在这个窗口手里时发不发、allowlist 没列的窗口报不报——全在这一层。它们住在 `platform/windows/` 之下，只在 Windows 上编译，所以在**任何一台 Windows** 机器上都跑得起来，不需要桌面；`Admitted::visible`、`outline` 与协议壳（`session`、`refusal`、`answer`）的测试在每个平台上都跑。DPI 感知在测试进程里读回，不需要窗口：nextest 每个测试一个进程，进程级的声明不会串到别的测试。
- **碰 Win32 的五个模块只测「拒绝是诚实的」**：一个不存在的窗口名恒得到一句指向 `desktop.windows` 的拒词，而不是一次崩溃。CI 里没有一张桌面可供点击，故「点下去真的点中了」这件事**恒不**被写成一条会在没有桌面时假装通过的测试；它由操作者在真机上验，本节记下这是一处**具名的空缺**，不是一处被忽略的覆盖率。

- **换过接口的调用各有一条契约测试**（§8-11）：测试在本进程里建一扇不抢焦点的窗口，读回它的标题、进程、外框、落点与树；这几条要一张桌面，不碰人桌面上别的窗口。

这条分界线是诚实的代价：写一条「在没有窗口时也返回 ok」的测试会比现在好看，但它证明的是这条测试自己，不是这台 server。

**操作者检查**（真机上由人做，不是门）：一台按 150% 缩放的显示器上，`desktop.snapshot` 之后按 ref 点击落在控件中心；对记事本 `type` 一段文字，文字进了指名的那扇窗口，动作之后 Ctrl、Shift 没有停在按下；两扇同名窗口按 `process` 指名录制，录到的是那一扇；写剪贴板期间 `GetClipboardOwner` 不为空；一张截图经 MCP 到模型，模型能说出图里的内容。

**证明**：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`。咬得动的演示：`Desktop.Scope.withoutBoth_admits_an_unlisted_process`、`Desktop.Scope.Pattern.withoutTheEnd_matches_a_longer_name`、`Desktop.Session.withoutTheGate_lists_tools_on_a_fresh_connection`、`Desktop.Session.withoutTheOrder_opens_the_tools_on_the_notification_alone`、`Desktop.Outline.withoutCleaning_keeps_a_newline`、`Desktop.Platform.Windows.Strokes.withoutUp_lifts_a_finished_click`、`Desktop.Platform.Windows.Views.withoutBounds_acts_on_a_moved_window`、`Desktop.Platform.Windows.Target.withoutUniqueness_picks_the_first_of_two`。模型的证明不是 Rust 实现的证明：实现与模型的一致由各模块旁的测试判（`cargo nextest run -p sprawling-desktop`），与上面的操作者检查分开记。
-/

/-! ## 17 文档关系

`ARCHITECTURE.md` §3 depmap 块的 `desktop` 与 `desktop_ffi` 两行｜`crates/desktop/README.md`（英文，讲清城怎么起它、叶子为什么有自己的一张 lint 表）｜tools/xtask/Spec.lean §8-46（guard 判叶子那张表）｜sprawling-SPEC §8-4d｜同步本 SPEC §13 与 §8-7 的实现状态｜`crates/desktop/README.md` 的 The honest-refusal rule 一节｜`crates/agent_protocols/Spec.lean` §8-1c（城怎么读 `isError` 与 `_meta`）。

`docs/glossary.md` 的 **desktop connector** 一行；`crates/browser/Spec.lean` D9（拖拽与滚动的同一份词汇，§8-7）；`crates/agent_protocols/Spec.lean` §8-1c（城怎么读 `isError` 与 `_meta`，`EFFECT_META_KEY` 与 `PROTOCOL_VERSION` 的唯一定义）；`crates/desktop/ffi/Spec.lean`（缝本身的规格）；sprawling-SPEC §8-4d 与 8-146；`architecture.toml` 里 desktop 各行的锚点指向本文件与分部。这些改了，重读本文件对应的节与决定。
-/
