# browser-SPEC.md

> crate：`browser`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

一个 Agent 要能驱动运行中的机器上的浏览器：看一个页面、点一个按钮、填一个输入框、改完代码再看一眼、并且下次还认得同一个账号。拆成六个可独立验收的单元，与模块一一对应：缝（port）、会话（session）、可见面（snapshot）、动作（act）、开发回路（devloop）、登录态（profile）。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| port | 帧的字节不依赖 map 迭代序；拒绝与读不懂的回复各有一条断言。**不含** conformance 断言——理由见 §8-1 决定 |
| session | 一次会话的帧序可在无浏览器下逐帧断言；Recording 重放同一问题得同一答案，答不出即报出问不出口的那条 |
| snapshot | 原始 DOM 恒不入窗（断言）；同一棵树两次快照字节相同；label 超长即截断且不引入换行 |
| act | 陈旧 generation 恒拒；页面文本进表达式后，字面量内除定界符外无未转义引号 |
| devloop | 任意观察序列在预算内到达一个结局；结局枚举穷尽 |
| survey | 同一页经 `--dump-dom` 与经 `script.evaluate` 读出同一个 `probe::Read`；`xtask render` 在本仓画廊上恒零发现（量具自身的假阳性已清到 0）；住户路径的 `Sources` 为空时每条发现只点名盒子、不点名行 |
| profile | 两栋楼的 profile 互不包含；路径住 reserved prefix；**confidential 楼的拒绝只有一个家**：`city::policy::evaluate` 读到 `confidential = true` 与 `browser = true`／`usersbrowser` 并存即 `E_CONFIG_INVALID`，本 crate 不再有 `Ephemeral` 臂（那是同一个规则的不可达第二家） |

## 3 假设与歧义

- **假设**：起进程归 `bin::browser_bidi`；本库恒不拉起浏览器进程、恒不持套接字、恒不下载驱动。
- **假设（附着）**：连一个已经开着的浏览器也归装配层——本库只把帧交给缝。装配层的 `AttachedBrowser` 与 `LazyEngine` 是同一缝上的第二个实现：前者只连、不启动、不结束进程；后者的 `running`／`Drop` 假定进程归自己，两者因此不能合成一个类型。
- **协议形状**：`input.performActions` 的 `pointer`／`wheel` 源动作字段与元素 origin 的 `SharedReference` 据 W3C 草案写成；`script.evaluate` 回复里 `sharedId` 的位置、空能力集、`image/png` 拼写已对 Gecko 真会话核过（§19-7）。`input::shared_id_of` 在回复里按有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`。
- **未定**：`-headless` 这一位由哪一面提供（楼的 `CONFIG.toml` 还是派活帧的一个字段）；某个具体 Firefox fork 是否接受本 crate 的启动参数与会话形态；行容器的交叉轴怎么扫（§19-10）；画出来的一对颜色是否可读：接进 `xtask::color` 的对比度模型要把它开放给本 crate，另写一条对比度公式就是第二个权威（`survey::legibility`）。
- **歧义已定**：BiDi 的 `session.new` 能力集合本版本只请求空能力＋按需 `network` 事件；更多能力等到有消费者再加，因为每一项能力都是远端因此获得的一项许可。

## 4 现状分析

纯判定与值类型：`session`、`snapshot`、`act`、`verb`、`input`、`shot`、`diff`、`devloop`、`profile`、`geometry`、`survey` 与缝 `port`。生产消费者是 `crates/sprawling`：`browser_bidi` 持套接字并实现 `BrowserPort`，`browser_tool` 把动作面接成 `browser` 与 `usersbrowser` 两件工具；`xtask render` 读 `survey`。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 命令形 `{id, CommandData, Extensible}`、模块划分 | <https://www.w3.org/TR/webdriver-bidi/> |
| `browsingContext` 语义（context 即可载入文档的 navigable） | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/browsingContext> |
| `script` 模块 | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/script> |
| `input` 模块（指针、滚轮；`performActions` 的源动作与元素 origin） | <https://w3c.github.io/webdriver-bidi/#module-input> |

规范是 W3C 工作草案；本库用 `session`／`browsingContext`／`script`／`input` 四个模块，`network` 仅作为可选订阅出现。

## 6 命名统一

`BrowserPort`｜`PageSnapshot`｜login state per Building——三者均取自词汇表，恒不自造同义词。「快照」在本 crate 恒指 `PageSnapshot`，与客户端的界面 fold 不同物，故跨 crate 引用时写全名。

## 7 模块边界

**三件邻居的活，及它们各自的主人**：

- **字节怎么走**归 `bin::assembly`：WebSocket、重连、超时住装配层，本 crate 恒不持套接字，也恒不依赖异步运行时。
- **这栋楼准不准出网**归 `city::policy`：`Profile::of` 只收楼的 `Address`，本 crate 读不到 policy；confidential 的判定在 city（§2 profile 行）。
- **页面带回来的内容算什么**归 `kernel::taint`：快照文本与工具结果同落污染环，本 crate 不另设解包面。
- **哪一页该被勘察、勘察完谁去改**归调用方：`xtask render` 是门，判的是本仓画廊；`browser` 工具的 `survey` 判的是住户自己打开的那一页。本 crate 只回答「这一页哪里不对」，不回答「该不该红」——`Standing` 由读者解释（§19-10）。

## 8 接口先行

```rust
// 8-1 port（形状 3 端口＋形状 2 值类型）
pub struct Frame { /* id、method、params 私有 */ }
impl Frame {
    pub fn new(id: u64, method: &str, params: Value) -> Result<Frame, AxError>;
    pub fn to_wire(&self) -> String;                       // 字段序固定，不随 map 迭代序
}
pub enum Reply { Success { id: u64, result: Value }, Error { id: u64, code: String, message: String } }
impl Reply {
    pub fn parse(line: &str) -> Result<Reply, AxError>;
    pub fn into_result(self) -> Result<Value, AxError>;    // 远端的拒绝带着它自己的词过来
}
pub trait BrowserPort { fn send(&mut self, frame: &Frame) -> Result<Reply, AxError>; }

// 8-2 session（形状 4 适配器＋形状 2）
pub struct ContextId(/* 私有 */);
pub struct SessionRequest { pub network: bool }            // 默认全关
pub struct Session { /* next: u64 私有 */ }
impl Session {
    pub fn begin(&mut self, request: SessionRequest) -> Result<Frame, AxError>;
    pub fn tree(&mut self) -> Result<Frame, AxError>;
    pub fn navigate(&mut self, context: &ContextId, url: &str) -> Result<Frame, AxError>;
    pub fn evaluate(&mut self, context: &ContextId, expression: &str) -> Result<Frame, AxError>;
    pub fn end(&mut self) -> Result<Frame, AxError>;
    pub fn read_tree(result: &Value) -> Result<Vec<ContextId>, AxError>;
}
pub struct Recording { /* 私有 */ }                        // 第二适配器
impl Recording { pub fn answer(&mut self, frame: &Frame, result: Value); pub fn missed(&self) -> &[String]; }

// 8-3 snapshot（形状 1 判定＋形状 2）
pub struct Node { pub reference: String, pub role: String, pub name: String }
pub struct PageSnapshot { /* generation、nodes 私有 */ }
impl PageSnapshot {
    pub fn read(generation: u64, tree: &Value) -> Result<PageSnapshot, AxError>;
    pub fn to_text(&self) -> String;
    pub fn resolve(&self, reference: &str) -> Result<&Node, AxError>;
}

// 8-4 act（形状 1 判定）
pub struct Point { pub x: i64, pub y: i64 }                       // 视口坐标，CSS 像素；滚轮增量可为负
pub enum Origin { Reference(String), Point(Point) }               // 拖拽从哪开始：快照的 ref 或视口点
pub enum Action { Click { reference: String }, Type { reference: String, text: String },
                  Read { reference: String },
                  Drag { from: Origin, to: Point, steps: u32 },   // input.performActions
                  Scroll { at: Option<Point>, by: Point } }       // 滚轮，by 为 CSS 像素增量
pub const STEPS_MAX: u32 = 32;
impl Action { pub fn reference(&self) -> Option<&str>; pub fn resolves_element(&self) -> bool; }
pub fn frame_for(session: &mut Session, context: &ContextId, snapshot: &PageSnapshot,
                 generation: u64, action: &Action) -> Result<Frame, AxError>;   // 仅 script 三臂
pub fn resolve_frame(session: &mut Session, context: &ContextId, snapshot: &PageSnapshot,
                     reference: &str) -> Result<Frame, AxError>;   // 元素 origin 的第一帧；generation 由 Verb::frames 先判
pub enum ResolvedOrigin { Element(String), Viewport(Point) }       // 已解析的 origin（input::Origin 经根重导出）
pub fn shared_id_of(value: &Value) -> Result<String, AxError>;     // 从第一帧的回复里读出元素 id

// 8-5 devloop（形状 1 判定）
pub struct Observation { pub text: String, pub complained: bool }
pub enum Step { Settled { looks: u32 }, LookAgain { looks: u32 }, Complained { looks: u32 }, GaveUp { looks: u32, why: String } }
pub const LOOKS_MAX: u32 = 8;
pub const QUIET_LOOKS: u32 = 2;
pub struct DevLoop { /* 私有 */ }
impl DevLoop { pub fn observe(&mut self, observation: &Observation) -> Result<Step, AxError>; }

// 8-6 profile（形状 1 判定）
pub struct Profile { /* path 私有 */ }
pub const PROFILES_DIR: &str = "browser-profiles";
impl Profile { pub fn of(building: &Address) -> Result<Profile, AxError>; pub fn path(&self) -> &Address; }
```

## 8.5 两个设计

**第一对（缝画在哪）**：把 WebSocket 会话整体放进本 crate（落选）vs 缝只运帧、套接字归装配层（选中）。前者读起来更像「一个浏览器客户端」，但它把异步运行时拖进一个本可纯的 crate，于是所有断言都要一个 runtime，而「第二适配器」只能是一个假服务器。后者让整段会话在无浏览器、无异步的条件下逐帧断言，录制回放因此能在一台没有 WebDriver 的机器上重放一次真实会话；它替代不了传输层的证据（§8.6）。代价：装配层多一段连接管理，且帧的 id 必须由 `Session` 铸而不能由传输层铸（否则重放会重新编号）。

**第二对（ref 是什么）**：ref ＝ 页面里的稳定标识（落选）vs ref ＝ 本次快照里的位置（选中）。前者要求页面配合（`id` 属性、`data-testid`），而页面是别人写的；后者把「页面动过了」变成一个可判定事实——ref 携 generation，陈旧即拒。代价：每次动作前必须先看一眼，这正是我们要的顺序。

## 8.6 port 缝的决定

**决定**：`BrowserPort` 是 trait，缝上没有 conformance 套件。录制回放（`Recording`）是重放证据，不是传输层的证据。

- trait 的理由是 AGENTS.md「一个 trait 只在已有第二个实现的缝上引入」：缝上有三个生产实现——`crates/sprawling/src/browser_bidi` 的 `BidiSocket`（直连）、`LazyEngine`（按需起引擎、首帧才连）与 `AttachedBrowser`（连人自己的浏览器），加上本 crate 的 `Recording`。撤 trait 会让这几条路合成一个类型。
- 不设套件的理由是它的证据为零：被测者 `Recording` 按构造就回 `frame.id()` 且回答不消费条目，断言恒真；生产适配器跑不了它，因为 CI 没有浏览器驱动（ARCHITECTURE.md §11 具名的缺口之一）。这样的套件会让读者把「过了 conformance」读成「传输层被验过」。
- **重开参数**：回复路由规则（读过无 id 的事件、按 id 认领答案）从 `socket.rs` 的 async 循环搬进本 crate 成为纯函数，且 `crates/sprawling` 的测试能用一对本地 socket 驱动它。那时套件与它的调用方在同一次改动里出现。

## 9 工作流程

装配层连上 WebSocket → `Session::begin` → `tree` → 取一个 `ContextId` → `navigate` → `script.evaluate` 取无障碍树 → `PageSnapshot::read`（generation ＋1）→ 文本入窗 → 模型给出 `Action` → `act::frame_for` → 帧出网 → `Reply` → 若在开发回路中则 `DevLoop::observe` 决定是否再看。

## 10 实现逻辑

1. **帧先于传输**：`Frame::to_wire` 手写字段序而不用 `serde_json::to_string`，因为录制回放要按字节比对，而 map 的迭代序不是契约。
2. **回复分两层**：传输失败是 `Err`，远端拒绝是 `Reply::Error`——「这个节点没了」是答案，不是故障；把两者混同会让调用方对着一个错误码猜是谁的问题。
3. **快照用白名单不用黑名单**：角色词汇十四项闭合。「除了 X 都放行」会在平台新增角色时静默变宽，而它变宽的终点就是原始 DOM。
   - **词汇只有一个家**：`snapshot::ROLE_MAP`（`(标签, 可选 type) → 角色`，27 行）是权威；采树脚本的映射表由 `role_lookup_js()` 从它生成，过滤器 `shown()` 也从它取值。
     脚本与过滤器若各持一张表，两张表只要不重合，不手写 `role=` 的页面快照里就**没有链接也没有输入框**，act/measure 拿不到 ref；手填角色的夹具绕过脚本，测不出这一点，所以表只有一张。
   - **两条派生断言**：`ROLE_MAP` 产得出的每个角色都在词汇里；词汇里除 `tab` 与 `alert`（无元素隐含，只能由页面显式声明）之外的每一项都至少有一个标签映射到它。
   - **`<input>` 的 type 缺席读作 `text`**（HTML 默认值）；`hidden`／`file` 等无行可查的 type 不得角色，因而不过河。
4. **动作里页面文本恒是数据**：`quote` 是页面内容成为代码的唯一位置，逐字符转义，含 U+2028／U+2029（JS 里它们是行终止符）。
5. **回路必有终点**：`LOOKS_MAX` 与 `QUIET_LOOKS` 两个常量把「不收敛」变成一个结局而不是一段时间。

## 11 边界枚举

空 method／非对象 params／无 id 的回复／`type` 未知／`contexts` 缺失／节点无 role／label 超长／label 含控制符／ref 非 `e<n>`／`e0`／陈旧 generation／回路超预算／房间地址当楼名／reserved prefix 当楼名。

## 12 Decisions

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_INVALID_ARGS` | 帧构造、ref 解析、楼名不是楼 | 部分能：ref 已由快照铸造，非法 ref 只能来自模型自造的字符串 |
| `E_WIRE_MISMATCH` | 回复或树的形状读不出 | 不能：对侧版本不由本库决定，故 fail closed |
| `E_BROWSER_UNAVAILABLE` | 远端拒绝、重放缺答案 | 不能：这是外部世界的事实 |
| `E_LOOP_SUSPECTED` | 结局之后继续观察 | 能：调用方持 `Step`，越过结局是它的错，故报出调用方 |

## 13 依赖选型

`kernel`（错误、地址）＋`serde`／`serde_json`（帧与探针的三串）＋`base64`／`png`（截图解码与量尺寸，§19-4）。**恒不引入** WebSocket 客户端、异步运行时、HTML 解析器：前两者归装配层，第三者会把原始 DOM 请回本 crate。

## 14 硬编码声明

`ROLE_MAP` 二十七行与 `ARIA_ONLY` 两项（合为十四个角色）、`NAME_MAX_JS = 200`（脚本回传的名字上限，与展示上限不同事）、`LABEL_MAX = 120`、`LOOKS_MAX = 8`、`QUIET_LOOKS = 2`、`PROFILES_DIR`。前两项改动即改变模型看见什么，属 15.2 的行为变更，改需证据；后三项是回路与落盘位置的约定。

## 15 影响面

改 `BrowserPort` 或 `Verb` 波及 `crates/sprawling` 的 `browser_bidi` 与 `browser_tool`；改 `survey` 波及 `xtask render`；`city::policy` 的 confidential 读取决定一栋楼有没有这两件工具。

## 16 测试与约束

逐模块 `#[cfg(test)]`；「原始 DOM 恒不入窗」「字节确定性」「陈旧 generation 恒拒」「回路必有终点」四条各有一条断言。**约束**：本 crate 恒不出现 `async`、恒不依赖 `tokio`、恒不持有文件句柄。

## 17 模型体验

`to_text` 的每行是 `ref role "name"`——三个字段一行，因为模型要做的下一件事是把 ref 抄回来。拒绝词恒报出可用 ref 的数量与起点（`e1` 起），这样「我编了一个 ref」和「页面变了」在读者那里是两句不同的话。

## 18 文档同步

`ARCHITECTURE.md` 模块表的 browser 各行与 §3 缝清单｜`docs/glossary.md` 若新增词汇。本 crate 没有 `conformance` feature（§8.6）。

## 19 工具的动作面，与截图成为证据

### 19-1 谁起浏览器进程

这座城自己起 Firefox：随机远程调试端口、`-profile <这栋楼的 profile>`、按需 `-headless`。本库与 `docs/glossary.md` 的 **browser** 行一致——

> **假设**：起进程这件事归 `bin::browser_bidi`，本 crate 仍恒不起进程、恒不持套接字、恒不下载驱动。Firefox 是**第一引擎**（原生 BiDi，无需驱动）；Chromium 只在 `chromedriver` 已在 PATH 上时才走得通，因此是第二条路而非并列的一条。

这不放宽本 crate 的任何约束：纯的那一半仍然纯，「谁按下启动键」住在装配层（§7 第一条）。

### 19-2 `browser::verb`（形状 1 判定）

工具 `browser` 的每一个动作，读成一个穷尽枚举，再变成帧。**一个动作可能要一帧以上**，所以出口是 `Vec<Frame>` 而不是 `Frame`：`open` 要先导航再装上控制台录音器，`screenshot` 带 `scale` 时要先改 devicePixelRatio。

```rust
pub enum Verb {
    Open { url: String },
    Snapshot,
    Act { generation: u64, action: Action },
    Screenshot(ShotRequest),
    Measure { references: Vec<String> },
    Console,
    Viewport { width: u32, height: u32 },
    Close,
}
impl Verb {
    pub fn read(args: &Payload) -> Result<Verb, AxError>;
    pub fn frames(&self, session: &mut Session, context: &ContextId,
                  snapshot: Option<&PageSnapshot>) -> Result<Vec<Frame>, AxError>;
    pub fn destination(&self) -> Result<Option<String>, AxError>;   // Open 的主机，门据此判出网
    pub fn input_frame(&self, session: &mut Session, context: &ContextId,
                       origin: Option<ResolvedOrigin>) -> Result<Frame, AxError>;
}
```

三条判定写在这里而不是调用方：`act` 没有快照即拒（对没看过的页面动手不可拼写，§8.5 第二对的直接后果）；`measure` 的每个 ref 都过 `PageSnapshot::resolve`，因此「我编了一个 ref」在出网前就被报出；`console` 读的是 `open` 时装上的录音数组，因为本 crate 的缝只运请求-应答，而 BiDi 的 `log.entryAdded` 是无 id 的事件，归装配层路由——用一个页面内数组换一条事件订阅，是拿已有机制复用而非新开一条通路。

### 19-3 `browser::shot`（形状 2 值类型）

截图的选项与回来的字节。`quality` 是 **0..=100 的整数**而不是浮点：浮点不进判定路径，而 BiDi 要的 `0.85` 只在最后一刻由 `format!("0.{q:02}")` 解析成 JSON 数，于是本仓库里没有一个 f64 变量。`scale` 同样是百分比整数，`100` 表示不改。

**一张截图裁哪一块，三个入口，互斥**：`clip` 是四个整数（x、y、width、height，CSS 像素）；`ref` ＋ `generation` 是快照铸出、世代守卫的引用；`refs` ＋ `generation` 是同样守卫的一组引用，取它们的包围矩形。**三臂而不是三个可空字段**：一块区域只有一个来处，三个 `Option` 会允许调用方同时点名，而同时点名没有答案。

```rust
pub enum Clip { Rect(Rect), Element(Element), Union(Union) }
pub struct Rect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
pub struct Element { /* reference、generation 私有 */ }
pub struct Union { /* references、generation 私有 */ }
pub struct ShotRequest { pub clip: Option<Clip>, pub format: ImageType,
                         pub quality: Option<u8>, pub scale: Option<u32> }
pub struct Shot { /* bytes、width、height 私有 */ }
impl Clip { pub fn waits_for_page(&self) -> bool; }
impl ShotRequest {
    pub fn waits_for_page(&self) -> bool;
    /// 先发的一帧之后：`reply` 是它自己的回复。
    pub fn capture_frame(&self, session: &mut Session, context: &ContextId,
                         reply: &Value) -> Result<Frame, AxError>;
}
impl Shot { pub fn read(reply: &Value, media: ImageType) -> Result<Shot, AxError>; }
```

**并集是协议装不下的那一臂**：BiDi 没有多元素裁剪，所以它先量后拍——`frames` 发的正是 `measure` 用的那段取框脚本（同一个 `measure_script`、同一个 `Math.round`），`capture_frame` 把回复里的盒子在 Rust 里求并（`checked` 算术，越界即拒），再以矩形臂拍。**框因此只有一个来源**：截图覆盖的区域与 `measure` 报出的区域是同一次读数，不可能两个说法。并集是全库唯一一个「多个元素 → 一个区域」的地方，也是这一臂存在的理由。

**`ref` 走的是一帧换一帧的两段路**，与 §19-8 的元素起点同形：`Verb::frames` 只发让页面报出元素的那一帧（`act::resolve_frame`，世代守卫仍是 `act::ensure_fresh` 一个权威），`input::shared_id_of` 从回复里取出驱动的句柄，`ShotRequest::capture_frame` 再发捕获帧，`clip` 拼成 `{"type":"element","element":{"sharedId":…}}`。**元素裁剪的矩形因此不由本仓计算**：它由页面报给驱动，没有四舍五入，也没有第二次测量，于是不存在「模型看到的框」与「裁出来的图」两个家。矩形臂上传入句柄、或元素臂上没有句柄，都回 `E_INVALID_ARGS`：这条路上没有可解的句柄，是正确的拒绝而不是不可能的状态。

**每张截图都带上界**：捕获帧恒带 `imageSize {maxWidth, maxHeight}`，两侧同为 `SHOT_MAX_EDGE_PX`（1920）。这是协议的一等参数（`browsingContext.ImageSize`），请求侧只声明意图；**它不构成保证**：对真 Gecko 会话的实测是 2000×1500 的视口带 `maxWidth:1920` 仍回 2000×1500，400×300 的 clip 带 `maxWidth:200` 仍回 400×300，即驱动原样不理这个参数。所以上界由两端一起说住：请求声明意图，**收下后按字节读出的两侧判**，超界就**按实测长边算出比例重拍一次**（`ShotRequest::refit_frame`，比例＝上界／长边×调用方要的密度，且永不放大），重拍后再判；仍然超界才拒（`ShotMaxEdge::admit`）。一次超界的截图因此是“变小”而不是“被拒”，而变小这件事只在重拍那一步发生。`scale` 与上界是两个事实：前者要「以多高密度渲染」，后者是这座城的成本纪律，**模型没有要多大就多大的旋钮**。上界只此一处定义，两个读它的人是同一件事的两端：请求侧的 `imageSize` 与收下后的 `admit`。

- **为什么重拍而不是自缩字节**：自己重采样等于新增依赖与自己的 CPU，而协议给的两个杆杆里 `imageSize` 不被执行，就只剩 `devicePixelRatio`。那个代价是明确的——它改的是页面可见的事实（`@media (resolution)`、`window.devicePixelRatio`），可能让页面按另一套样式重排——所以它**只在与“拒绝”二选一时付**：一张本来就在上界内的图，一次也不会碰到它。**重新考虑的参数**：驱动开始执行 `imageSize`（那时这一支永远不走）。
- **上界为什么不住 `kernel::policy_limit`**：那个模块的约定是一个极限没有公开取值，调用方交出观察到的事实、由极限说出唯一那句拒绝。而这个上界的两个读者都在本 crate（请求与收下），定义在此处就是一处。**重新考虑的参数**：若第二个 crate 要这个数（桌面侧截图并入同一份词汇即是），它搬进 `consts_policy`，并让 `policy_limit` 长出那一刻需要的取值方法。
- **不静默降级**：驱动若不接受元素裁剪，这一调用以驱动自身的拒绝失败，本库**不偷偷退回自己算框**——退回就是给「这块在哪」添第二个家。退回的路存在且是公开动作（让页面回矩形、走矩形臂），要不要走由 §19-7 的真机核对决定。

`Shot::read` 解 base64 并把字节交给 `png` 读出尺寸：**本版本只在 PNG 上给出尺寸**，其他格式回 `E_WIRE_MISMATCH` 而不是猜。理由是 `ImageRef` 的 width／height 是模型看图前唯一的尺度，猜错的尺寸比没有尺寸更坏；而默认格式本就是 PNG，所以这条拒绝挡的是有人显式要了别的格式又要尺寸。

### 19-4 `browser::diff`（形状 1 判定）

`diff(a, b)` 回答两件事：变了百分之几，以及变的地方在哪几个框里。百分比是**万分比整数**（`changed_ppm`／`ratio_q4`），框是像素坐标的整数矩形，因为这两个数会进账本载荷。尺寸不同的两张图不比较，回 `E_INVALID_ARGS`——把一张缩放到另一张上再比，比出来的差异是缩放算法的，不是页面的。

解码后的字节短于自己头部声明的尺寸时回 `E_WIRE_MISMATCH`，并且**说出是哪一张短了**：先拍的、后拍的、还是两张都短。三种情况的下一步动作不同——要重拍的是哪一张，拒绝语直接给出，读的人不必两张都重来。判定对 `(前, 后)` 两个像素取值穷尽匹配，没有兜底臂。

**一个矩形类型**：差异的框与截图覆盖的区域、以及元素报出的框，是同一个四整数形状，所以全 crate 只有一个 `shot::Rect`（`covers` 与 `covering` 是它的方法），`diff` 从 `shot` 读它：同一个形状两个名字，就是两个会漂开的定义。

依赖 `png`（MIT OR Apache-2.0，`deny.toml` 的 allow 列表已含两者）：产品路径只解码，测试用它的编码器造夹具，于是断言比的是真 PNG 字节而不是一份没人能复核的固定串。

### 19-5 `browser::devloop` 消费 `look` 的判定

`DevLoop::observe` 已经吃 `Observation { text, complained }`；要的是**接线而非新判定**：`browser` 工具的 `snapshot` 动作产出的那段文本就是 `text`，`console` 里出现过 error 级别的条目就是 `complained`，于是「改一处、看一眼、再决定」在工具层闭合，`Step` 作为工具结果回给模型。判定本身一个字不改——已有机制复用是这里的正解。

### 19-6 动作面的验收

| 单元 | 完成的定义 |
|---|---|
| verb | 每个动作各自的帧可在无浏览器下逐帧断言；`act` 无快照即拒；`measure` 的假 ref 在出网前被拒 |
| shot | 同一段 PNG 字节两次读出同一尺寸；非 PNG 不猜尺寸；quality 不引入浮点变量；`clip` 与 `ref` 同时给出即拒，`ref` 没有世代即拒；两张捕获帧都带上界，且上界按字节读出的两侧判；矩形臂传入句柄即拒 |
| diff | 尺寸不同即拒；全同两图为 0；一个像素变化的框恰好含那个像素；解码字节短于头部时拒绝语点名是哪一张 |
| input | 指针拖拽恒是 pointerMove→pointerDown→pointerMove×n→pointerUp；元素 origin 有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`；滚轮增量可为负 |
| usersbrowser | 工具名取 `ToolName::USER_BROWSER` 一个权威；未声明地址的楼每次调用都得到门的问题；声明了地址的楼其 effect 带该主机；`usersbrowser` 与 `browser` 是两个设置；confidential 楼在 `city::policy` 即拒 |


### 19-7 对真浏览器核过的部分

**对 Gecko 的一次真会话核过了 §3 的协议形状**：`session.new` 接受空能力集；`script.evaluate` 的回复是 `result.result = { type, handle, sharedId, value }`，`sharedId` 就在这一层，`shared_id_of` 的有界查找找到它；`browsingContext.captureScreenshot` 的 `format.type` 收 `image/png` 这一拼写。元素裁剪也随之落地：`clip.type = "element"` 收由 `script.evaluate` 回复里取出的 `sharedId`，回来的图正好是该元素的框。

同一次会话量到驱动**原样忽略** `imageSize`，所以上界靠 §19-3 的重拍生效：超界时按实测长边算比例重拍一次，仍超界才拒。

- **`-headless` 有开关没有问的人**：`LaunchPlan` 带这一位并逐字断言，但 `for_building` 恒传 `false`；由哪一面提供见 §3。
- **引擎的名字不止一个，而且已经是查表**：`host::firefox` 走的是 `doctor` 的 `gecko` 条目（`Need::OneOf(Group::BrowserEngine)`），家族表里有 firefox、zen、librewolf、waterfox、floorp、firefox-developer、firefox-nightly、tor-browser 八行，`SPRAWLING_BROWSER` 可压过其一；`Engine::choose` 的参数只是叫 `firefox`，取的是这条答案的路径——所以一个只有 fork、没有 Firefox 的机器是可起的。**仍未定的只是：某个具体 fork 是否接受本 crate 的启动参数与会话形态**，而那要在那个 fork 上真的起一次会话才算数。

### 19-8 `browser::input`（形状 1 判定）

BiDi 的 `input` 是 `script` 之外的另一个协议模块，本 crate 之前全走 `script.evaluate`。指针动作要说明它从哪开始，而 BiDi 的元素 origin 用页面自己的 shared id 而不是选择器，于是元素起点的拖拽在线上是**两帧**：`act::resolve_frame` 让页面报出元素，`input::shared_id_of` 从回复里读出 id，`input::pointer_frame` 再发 `input.performActions`。`Verb::frames` 只发第一帧，工具在 `invoke` 里补第二帧——判定仍在纯代码里，`Recording` 能逐帧重放。

`Scroll` 与 point 起点的 `Drag` 都没有元素，不进 `Action::reference()` 的形状；那个方法因此是 `Option<&str>`，`resolves_element()` 说明哪一臂要多一帧。

**与 `desktop.act` 同一份词汇**：`drag` 从 ref 或 point 到 point、`scroll` 用 `to` 表示滚多远、`steps` 为中间移动次数；两侧字段名与含义逐字相同（`desktop-SPEC.md` §8-4 指向本节）。同一个动作在浏览器侧与桌面侧各有一个家会立刻漂开，所以拖拽的形状只有一份。

### 19-9 `usersbrowser`（工具，装配层）

驱动**人已经开着的那个浏览器**，用的是那个人的真 profile。与 `browser` 的差别是安全模型而不是动作集合：`browser` 那台由城拉起、profile 按楼隔离、confidential 楼恒无；`usersbrowser` 连的是人自己的进程，楼与楼的登录态隔离在附着那一刻不再成立，所以：

- **地址是人的声明**：`RULES.toml` 的 `usersbrowser` 一键，值为 `ws://127.0.0.1:<port>/session` 时启用该工具并把地址交给 attach 门；值为 `true` 时启用而地址未定，于是每次调用都得到门的问题（`E_APPROVAL_PENDING`）与那句要人做的事；absent 或 `false` 即无此工具。confidential 楼写这一键即在规则读取处被拒。
- **恒不关人的浏览器**：附着的端口（`bin::browser_bidi::attach::AttachedBrowser`）只连、不启动、不结束进程；`Verb::Close` 结束的是一次会话，进程还在。附着是会话级、绑一个 run，run 一结束套接字随工具一起 drop。
- **入账**：附着是工具的第一次调用，与之后每个动作一样写 `tool_called`／`tool_result`；`disclosure` 写明它需要人先批准并引导先用 `browser`，`params` 给出动作的读法（§19-6 的 usersbrowser 行）。
- **平台的门就是授权**：Firefox 走 `--remote-debugging-port`、Chromium 走驱动，两者都要求人的动作；这不是我们加的仪式，是平台留下的授权面。地址是否 loopback 由 `gate::attach` 判：非 loopback 的声明被拒，因为那会把登录态读过一个网络。

### 19-10 `browser::survey`（量具，形状 1 判定）

一页哪里画错了，由**一份测量、一套判决**回答，而这一份同时是 `xtask render` 这道门和 `browser` 工具的 `survey` 动作所读的东西。它住产品 crate，因为门与工具各留一份就会分叉到「门说页面是干净的、住户说页面是坏的」而二者各自诚实。

- **一份测量，两种取回**。`survey::probe::body` 是那段注入页面的 ES5，返回三个字符串（元素、声明的词、绘制条件）。门渲染一次并 dump 整个 DOM，所以它把三串写进三个 `<pre>`；住户勘察的是人自己打开的页面，**不许往那页面上加任何东西**，所以 `probe::evaluated` 把同一段包进一个 promise，三串作为一次 `script.evaluate` 的值回来（`awaitPromise` 本就是开着的）。两条路读的是同一个 `probe::Read`。
- **`Verb::Survey` 是新的一臂，不是 `Measure` 的扩展**。`Measure { references }` 回答「我点名的这几个节点在哪」，取的是调用方给的引用；survey 不接引用、读整页、返回判决。合成一个臂会让 `references` 在一半调用里恒为空，那是「一个参数被接受然后丢掉」。
- **主题由调用方决定，而住户不决定**。`body` 收 `Option<&str>`：门为每一个 pass 强制一个主题并据此断言页面照办，住户传 `None`——勘察一个别人打开的页面时强制主题，报的就是没人看过的那一页。同理 `PaintSource`：门按它要求的 pass 给，住户按页面自陈的 `forced-colors` 给。
- **源码索引在有源码树的那一侧**。`Sources` 的查找（最长字面量匹配）随判决进产品 crate，**走一遍源码树的那一步留在 `xtask`**——产品二进制身边没有仓库。住户得到的每条发现因此只点名盒子、不点名行号，而这是诚实的空状态，不是缺陷：`Sources::default()` 正是为这一天实现的。
- **答一个字符串，不拆成载荷**。`survey` 动作的结果是 tagged 形态的整份报告（一个 `<edit>` 一处修复、一个 `<at>` 一个落点）。把它拆成结构化载荷等于给同一份报告第二种渲染。
- **量具的两条规则**：容差 `SLACK`（1 px）在每一次缘比较上都加；群体先从几何读出容器的堆叠方向，只比容器不分发的那一轴，因为一行里并排的盒子右缘近似相等纯属巧合。
- **行容器的交叉轴不扫**：一行把子元素约束在一条带里，但带里的位置由 `align-items` 决定，本库到处用居中，于是两个不同行高的子元素**按设计**就有不同顶缘；按左右缘的读法去扫会报出一批假阳性。要正确读它得比较顶／中／底里多数实际持有的那一个，这把尺子还没有这个读数（§3）。
