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
- **假设（未核验的协议形状，离线无法查）**：`input.performActions` 的 `pointer`／`wheel` 源动作字段、元素 origin 的 `SharedReference`、以及 `script.evaluate` 返回节点时 `sharedId` 的嵌套位置，据 W3C 草案写成；`input::shared_id_of` 在回复里按有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`。第一次对着真浏览器跑时要先核这三处。
- **歧义已定**：BiDi 的 `session.new` 能力集合本版本只请求空能力＋按需 `network` 事件；更多能力等到有消费者再加，因为每一项能力都是远端因此获得的一项许可。

## 4 现状分析

P4 之前 `crates/browser/src/` 只有 `lib.rs` 一行文档。无既有代码要迁移。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 命令形 `{id, CommandData, Extensible}`、模块划分 | <https://www.w3.org/TR/webdriver-bidi/> |
| `browsingContext` 语义（context 即可载入文档的 navigable） | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/browsingContext> |
| `script` 模块 | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/script> |
| `input` 模块（指针、滚轮；`performActions` 的源动作与元素 origin） | <https://w3c.github.io/webdriver-bidi/#module-input> |

2026-08-22 复核：规范仍是 W3C 工作草案；本库只用 `session`／`browsingContext`／`script` 三个模块，`network` 仅作为可选订阅出现。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。

`BrowserPort`｜`PageSnapshot`｜login state per Building——三者均取自词汇表，恒不自造同义词。「快照」在本 crate 恒指 `PageSnapshot`，与 `web::Snapshot`（界面前进式 fold）不同物，故跨 crate 引用时写全名。

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

## 8.6 port 缝的决定（推翻 §8.5 第一对里「录制回放是真的第二适配器」一句）

**决定（§8-1，推翻旧决定「两个适配器过同一套 conformance 断言」）**：`assert_port_conformance` 删除，`BrowserPort` 保留。

- trait 保留的理由是 AGENTS.md「一个 trait 只在已有第二个实现的缝上引入」：缝上有两个生产实现——`crates/sprawling/src/browser_bidi/socket.rs:61` 的 `BidiSocket` 与 `lazy.rs:113` 的 `LazyEngine`（按需起引擎、首帧才连），加上本 crate 的 `Recording`。撤 trait 会让懒起与直连两条路合成一个类型。
- 套件删除的理由是它的证据为零：唯一调用点在 `session.rs` 的测试里，被测者 `Recording` 按构造就回 `frame.id()` 且回答不消费条目，两条断言恒真；两个生产适配器从不跑它，因为 CI 没有浏览器驱动（ARCHITECTURE.md §11 已具名的四个缺口之一）。留着它，读者会把「过了 conformance」读成「传输层被验过」。
- **重开参数**：当回复路由规则（读过无 id 的事件、按 id 认领答案）从 `socket.rs` 的 async 循环搬进本 crate 成为纯函数，且 `crates/sprawling` 的测试能用一对本地 socket 驱动它时，套件与它的调用方在同一次改动里回来。那是一次跨两个 crate 的改动，不属于本叶子。

## 9 工作流程

装配层连上 WebSocket → `Session::begin` → `tree` → 取一个 `ContextId` → `navigate` → `script.evaluate` 取无障碍树 → `PageSnapshot::read`（generation ＋1）→ 文本入窗 → 模型给出 `Action` → `act::frame_for` → 帧出网 → `Reply` → 若在开发回路中则 `DevLoop::observe` 决定是否再看。

## 10 实现逻辑

1. **帧先于传输**：`Frame::to_wire` 手写字段序而不用 `serde_json::to_string`，因为录制回放要按字节比对，而 map 的迭代序不是契约。
2. **回复分两层**：传输失败是 `Err`，远端拒绝是 `Reply::Error`——「这个节点没了」是答案，不是故障；把两者混同会让调用方对着一个错误码猜是谁的问题。
3. **快照用白名单不用黑名单**：角色词汇十四项闭合。「除了 X 都放行」会在平台新增角色时静默变宽，而它变宽的终点就是原始 DOM。
   - **词汇只有一个家（B-27）**：`snapshot::ROLE_MAP`（`(标签, 可选 type) → 角色`，27 行）是权威；采树脚本的映射表由 `role_lookup_js()` 从它生成，过滤器 `shown()` 也从它取值。
     此前脚本把角色定义为「显式 role 属性，否则小写标签名」，而过滤器查的是 ARIA 角色表，两者只在 button/table/form/option/dialog 上重合：
     不手写 `role=` 的页面快照里**没有链接也没有输入框**，`to_text()` 几乎是空的，act/measure 拿不到 ref。夹具手填角色绕过了脚本，所以测试曾经全绿。
   - **两条派生断言**：`ROLE_MAP` 产得出的每个角色都在词汇里；词汇里除 `tab` 与 `alert`（无元素隐含，只能由页面显式声明）之外的每一项都至少有一个标签映射到它。
   - **`<input>` 的 type 缺席读作 `text`**（HTML 默认值）；`hidden`／`file` 等无行可查的 type 不得角色，因而不过河。
4. **动作里页面文本恒是数据**：`quote` 是页面内容成为代码的唯一位置，逐字符转义，含 U+2028／U+2029（JS 里它们是行终止符）。
5. **回路必有终点**：`LOOKS_MAX` 与 `QUIET_LOOKS` 两个常量把「不收敛」变成一个结局而不是一段时间。

## 11 边界枚举

空 method／非对象 params／无 id 的回复／`type` 未知／`contexts` 缺失／节点无 role／label 超长／label 含控制符／ref 非 `e<n>`／`e0`／陈旧 generation／回路超预算／房间地址当楼名／reserved prefix 当楼名。

## 12 错误处理

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_INVALID_ARGS` | 帧构造、ref 解析、楼名不是楼 | 部分能：ref 已由快照铸造，非法 ref 只能来自模型自造的字符串 |
| `E_WIRE_MISMATCH` | 回复或树的形状读不出 | 不能：对侧版本不由本库决定，故 fail closed |
| `E_BROWSER_UNAVAILABLE` | 远端拒绝、重放缺答案 | 不能：这是外部世界的事实 |
| `E_LOOP_SUSPECTED` | 结局之后继续观察 | 能：调用方持 `Step`，越过结局是它的错，故报出调用方 |

## 13 依赖选型

`kernel`（错误、地址）＋`serde_json`。**恒不引入** WebSocket 客户端、异步运行时、HTML 解析器：前两者归装配层，第三者会把原始 DOM 请回本 crate。

## 14 硬编码声明

`ROLE_MAP` 二十七行与 `ARIA_ONLY` 两项（合为十四个角色）、`NAME_MAX_JS = 200`（脚本回传的名字上限，与展示上限不同事）、`LABEL_MAX = 120`、`LOOKS_MAX = 8`、`QUIET_LOOKS = 2`、`PROFILES_DIR`。前两项改动即改变模型看见什么，属 15.2 的行为变更，改需证据；后三项是回路与落盘位置的约定。

## 15 影响面

新增 crate，无既有调用方。装配层将来接线时波及：连接管理、`kernel::tool` 缝上的浏览器工具、`city::policy` 的 confidential 读取。

## 16 测试与约束

逐模块 `#[cfg(test)]`；「原始 DOM 恒不入窗」「字节确定性」「陈旧 generation 恒拒」「回路必有终点」四条各有一条断言。**约束**：本 crate 恒不出现 `async`、恒不依赖 `tokio`、恒不持有文件句柄。

## 17 模型体验

`to_text` 的每行是 `ref role "name"`——三个字段一行，因为模型要做的下一件事是把 ref 抄回来。拒绝词恒报出可用 ref 的数量与起点（`e1` 起），这样「我编了一个 ref」和「页面变了」在读者那里是两句不同的话。

## 18 文档同步

`ARCHITECTURE.md` §6 browser 六行与 §3 缝清单｜`docs/glossary.md` 若新增词汇｜装配层接线时同步 §6 末接线台账。

**`conformance` feature 作废**：本 crate 不再有 conformance 套件（§8-1 决定），`crates/browser/Cargo.toml` 的 `conformance = []` 应随之删除（跨文件，见交付报告）。`cargo xtask artifact` 的规则不变，它辖的另外四套 conformance 与本 crate 无关。

## 19 工具的动作面，与截图成为证据

### 19-1 谁起浏览器进程

这座城自己起 Firefox：随机远程调试端口、`-profile <这栋楼的 profile>`、按需 `-headless`。本库与 `docs/glossary.md` 的 **browser** 行一致——

> **假设**：起进程这件事归 `bin::browser_bidi`，本 crate 仍恒不起进程、恒不持套接字、恒不下载驱动。Firefox 是**第一引擎**（原生 BiDi，无需驱动）；Chromium 只在 `chromedriver` 已在 PATH 上时才走得通，因此是第二条路而非并列的一条。

这不放宽本 crate 的任何约束：纯的那一半仍然纯，「谁按下启动键」住在装配层（§7 第一条）。

### 19-2 `browser::verb`（新模块，形状 1 判定）

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

### 19-3 `browser::shot`（新模块，形状 2 值类型）

截图的四个可选项与回来的字节。`quality` 是 **0..=100 的整数**而不是浮点：浮点不进判定路径，而 BiDi 要的 `0.85` 只在最后一刻由 `format!("0.{q:02}")` 解析成 JSON 数，于是本仓库里没有一个 f64 变量。`clip` 是四个整数（x、y、width、height）。`scale` 同样是百分比整数，`100` 表示不改。

```rust
pub struct ShotRequest { pub clip: Option<Clip>, pub format: ImageType,
                         pub quality: Option<u8>, pub scale: Option<u32> }
pub struct Clip { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
pub struct Shot { /* bytes、width、height 私有 */ }
impl Shot { pub fn read(reply: &Value, media: ImageType) -> Result<Shot, AxError>; }
```

`Shot::read` 解 base64 并把字节交给 `png` 读出尺寸：**本版本只在 PNG 上给出尺寸**，其他格式回 `E_WIRE_MISMATCH` 而不是猜。理由是 `ImageRef` 的 width／height 是模型看图前唯一的尺度，猜错的尺寸比没有尺寸更坏；而默认格式本就是 PNG，所以这条拒绝挡的是有人显式要了别的格式又要尺寸。

### 19-4 `browser::diff`（新模块，形状 1 判定）

`diff(a, b)` 回答两件事：变了百分之几，以及变的地方在哪几个框里。百分比是**万分比整数**（`changed_ppm`／`ratio_q4`），框是像素坐标的整数矩形，因为这两个数会进账本载荷。尺寸不同的两张图不比较，回 `E_INVALID_ARGS`——把一张缩放到另一张上再比，比出来的差异是缩放算法的，不是页面的。

解码后的字节短于自己头部声明的尺寸时回 `E_WIRE_MISMATCH`，并且**说出是哪一张短了**：先拍的、后拍的、还是两张都短。三种情况的下一步动作不同——要重拍的是哪一张，拒绝语直接给出，读的人不必两张都重来。判定对 `(前, 后)` 两个像素取值穷尽匹配，没有兜底臂。

依赖 `png` 0.18（MIT OR Apache-2.0，`deny.toml` 的 allow 列表已含两者）：产品路径只解码，测试用它的编码器造夹具，于是断言比的是真 PNG 字节而不是一份没人能复核的固定串。

### 19-5 `browser::devloop` 消费 `look` 的判定

`DevLoop::observe` 已经吃 `Observation { text, complained }`；要的是**接线而非新判定**：`browser` 工具的 `snapshot` 动作产出的那段文本就是 `text`，`console` 里出现过 error 级别的条目就是 `complained`，于是「改一处、看一眼、再决定」在工具层闭合，`Step` 作为工具结果回给模型。判定本身一个字不改——已有机制复用是这里的正解。

### 19-6 验收（追加到 §2）

| 单元 | 完成的定义 |
|---|---|
| verb | 每个动作各自的帧可在无浏览器下逐帧断言；`act` 无快照即拒；`measure` 的假 ref 在出网前被拒 |
| shot | 同一段 PNG 字节两次读出同一尺寸；非 PNG 不猜尺寸；quality 不引入浮点变量 |
| diff | 尺寸不同即拒；全同两图为 0；一个像素变化的框恰好含那个像素；解码字节短于头部时拒绝语点名是哪一张 |
| input | 指针拖拽恒是 pointerMove→pointerDown→pointerMove×n→pointerUp；元素 origin 有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`；滚轮增量可为负 |
| usersbrowser | 工具名取 `ToolName::USER_BROWSER` 一个权威；未声明地址的楼每次调用都得到门的问题；声明了地址的楼其 effect 带该主机；`usersbrowser` 与 `browser` 是两个设置；confidential 楼在 `city::policy` 即拒 |


### 19-7 尚未验证的部分

- **`bin::browser_bidi::BidiSocket` 没有对着真浏览器跑过**。逐帧逻辑（发一帧、读到 id 相同的那条、跳过无 id 的事件）由阅读 W3C 草案得出而非由一次真实会话验证。工具那一侧的整条 open→snapshot→act→screenshot 由 `Recording` 逐帧断言，缝的另一个适配器因此是可信的；**这一侧不是**。第一次真跑要看的是三件事：`session.new` 的能力集合是否被 Firefox 接受、`script.evaluate` 的返回值是否真是 `result.value` 的字符串形状、`browsingContext.captureScreenshot` 的 `format.type` 是否收 `image/png` 这一拼写。
- **`-headless` 有开关没有问的人**：`LaunchPlan` 带这一位并逐字断言，但 `for_building` 恒传 `false`。这一位由哪一面提供尚未定：候选是楼的 `CONFIG.toml` 与派活帧的一个字段。

### 19-8 `browser::input`（新模块，形状 1 判定）

BiDi 的 `input` 是 `script` 之外的另一个协议模块，本 crate 之前全走 `script.evaluate`。指针动作要说明它从哪开始，而 BiDi 的元素 origin 用页面自己的 shared id 而不是选择器，于是元素起点的拖拽在线上是**两帧**：`act::resolve_frame` 让页面报出元素，`input::shared_id_of` 从回复里读出 id，`input::pointer_frame` 再发 `input.performActions`。`Verb::frames` 只发第一帧，工具在 `invoke` 里补第二帧——判定仍在纯代码里，`Recording` 能逐帧重放。

`Scroll` 与 point 起点的 `Drag` 都没有元素，不进 `Action::reference()` 的形状；那个方法因此是 `Option<&str>`，`resolves_element()` 说明哪一臂要多一帧。

**与 `desktop.act` 同一份词汇**：`drag` 从 ref 或 point 到 point、`scroll` 用 `to` 表示滚多远、`steps` 为中间移动次数；两侧字段名与含义逐字相同（`desktop-SPEC.md` §8-4 指向本节）。同一个动作在浏览器侧与桌面侧各有一个家会立刻漂开，所以拖拽的形状只有一份。

### 19-10 `browser::survey`（量具毕业进工具，形状 1 判定）

一页哪里画错了，由**一份测量、一套判决**回答，而这一份同时是 `xtask render` 这道门和 `browser` 工具的 `survey` 动作所读的东西。它先在 `xtask` 里长成，在那里把自身的假阳性从 24 条清到 0；毕业进产品 crate 的理由不是它变好了，而是**门与工具原本会各留一份**，而两份会分叉到「门说页面是干净的、住户说页面是坏的」而二者各自诚实。

- **一份测量，两种取回**。`survey::probe::body` 是那段注入页面的 ES5，返回三个字符串（元素、声明的词、绘制条件）。门渲染一次并 dump 整个 DOM，所以它把三串写进三个 `<pre>`；住户勘察的是人自己打开的页面，**不许往那页面上加任何东西**，所以 `probe::evaluated` 把同一段包进一个 promise，三串作为一次 `script.evaluate` 的值回来（`awaitPromise` 本就是开着的）。两条路读的是同一个 `probe::Read`。
- **`Verb::Survey` 是新的一臂，不是 `Measure` 的扩展**。`Measure { references }` 回答「我点名的这几个节点在哪」，取的是调用方给的引用；survey 不接引用、读整页、返回判决。合成一个臂会让 `references` 在一半调用里恒为空，那是「一个参数被接受然后丢掉」。
- **主题由调用方决定，而住户不决定**。`body` 收 `Option<&str>`：门为每一个 pass 强制一个主题并据此断言页面照办，住户传 `None`——勘察一个别人打开的页面时强制主题，报的就是没人看过的那一页。同理 `PaintSource`：门按它要求的 pass 给，住户按页面自陈的 `forced-colors` 给。
- **源码索引在有源码树的那一侧**。`Sources` 的查找（最长字面量匹配）随判决进产品 crate，**走一遍源码树的那一步留在 `xtask`**——产品二进制身边没有仓库。住户得到的每条发现因此只点名盒子、不点名行号，而这是诚实的空状态，不是缺陷：`Sources::default()` 正是为这一天实现的。
- **答一个字符串，不拆成载荷**。`survey` 动作的结果是 tagged 形态的整份报告（一个 `<edit>` 一处修复、一个 `<at>` 一个落点）。把它拆成结构化载荷等于给同一份报告第二种渲染。
- **量具自身的两条真缺陷记在这里**，免得下一个人重新发现：`SLACK` 在 observed 路径上从未被应用（十一条假阳性出自这一个 off-by-one）；群体定义有范畴错误——它对所有 holder 都比左右缘，而一行里并排的盒子右缘近似相等纯属巧合，现在先从几何读出容器的堆叠方向，只比容器不分发的那一轴。
- **一条没做的，明写**：行容器的**交叉轴没有扫**。一行把子元素约束在一条带里，但带里的位置由 `align-items` 决定，本库到处用居中，于是两个不同行高的子元素**按设计**就有不同顶缘。扫过一次，5 条假阳性变 15 条。要正确读它得比较顶／中／底里多数实际持有的那一个，**那是这把尺子还没有的读数**。

### 19-9 `usersbrowser`（工具，装配层）

驱动**人已经开着的那个浏览器**，用的是那个人的真 profile。与 `browser` 的差别是安全模型而不是动作集合：`browser` 那台由城拉起、profile 按楼隔离、confidential 楼恒无；`usersbrowser` 连的是人自己的进程，楼与楼的登录态隔离在附着那一刻不再成立，所以：

- **地址是人的声明**：`RULES.toml` 的 `usersbrowser` 一键，值为 `ws://127.0.0.1:<port>/session` 时启用该工具并把地址交给 attach 门；值为 `true` 时启用而地址未定，于是每次调用都得到门的问题（`E_APPROVAL_PENDING`）与那句要人做的事；absent 或 `false` 即无此工具。confidential 楼写这一键即在规则读取处被拒。
- **恒不关人的浏览器**：附着的端口（`bin::browser_bidi::attach::AttachedBrowser`）只连、不启动、不结束进程；`Verb::Close` 结束的是一次会话，进程还在。附着是会话级、绑一个 run，run 一结束套接字随工具一起 drop。
- **入账**：附着是工具的第一次调用，与之后每个动作一样写 `tool_called`／`tool_result`；`disclosure` 写明它需要人先批准并引导先用 `browser`，`params` 给出动作的读法（§19-6 的 usersbrowser 行）。
- **平台的门就是授权**：Firefox 走 `--remote-debugging-port`、Chromium 走驱动，两者都要求人的动作；这不是我们加的仪式，是平台留下的授权面。地址是否 loopback 由 `gate::attach` 判：非 loopback 的声明被拒，因为那会把登录态读过一个网络。
