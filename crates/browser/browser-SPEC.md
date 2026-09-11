# browser-SPEC.md

> crate：`browser`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

一个 Agent 要能驱动运行中的机器上的浏览器：看一个页面、点一个按钮、填一个输入框、改完代码再看一眼、并且下次还认得同一个账号。拆成六个可独立验收的单元，与模块一一对应：缝（port）、会话（session）、可见面（snapshot）、动作（act）、开发回路（devloop）、登录态（profile）。

## 2 验收标准

| 单元 | 完成的定义 |
|---|---|
| port | 两个适配器过同一套 conformance 断言；帧的字节不依赖 map 迭代序 |
| session | 一次会话的帧序可在无浏览器下逐帧断言；Recording 重放同一问题得同一答案，答不出即报出问不出口的那条 |
| snapshot | 原始 DOM 恒不入窗（断言）；同一棵树两次快照字节相同；label 超长即截断且不引入换行 |
| act | 陈旧 generation 恒拒；页面文本进表达式后，字面量内除定界符外无未转义引号 |
| devloop | 任意观察序列在预算内到达一个结局；结局枚举穷尽 |
| profile | 两栋楼的 profile 互不包含；路径住 reserved prefix；confidential 楼恒 Ephemeral |

## 3 假设与歧义

- **假设**：起进程归 `bin::browser_bidi`；本库恒不拉起浏览器进程、恒不持套接字、恒不下载驱动。
- **歧义已定**：BiDi 的 `session.new` 能力集合本版本只请求空能力＋按需 `network` 事件；更多能力等到有消费者再加，因为每一项能力都是远端因此获得的一项许可。

## 4 现状分析

P4 之前 `crates/browser/src/` 只有 `lib.rs` 一行文档。无既有代码要迁移。

## 5 权威信源

| 事实 | 出处 |
|---|---|
| 命令形 `{id, CommandData, Extensible}`、模块划分 | <https://www.w3.org/TR/webdriver-bidi/> |
| `browsingContext` 语义（context 即可载入文档的 navigable） | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/browsingContext> |
| `script` 模块 | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/script> |

2026-08-22 复核：规范仍是 W3C 工作草案；本库只用 `session`／`browsingContext`／`script` 三个模块，`network` 仅作为可选订阅出现。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。

`BrowserPort`｜`PageSnapshot`｜login state per Building——三者均取自词汇表，恒不自造同义词。「快照」在本 crate 恒指 `PageSnapshot`，与 `web::Snapshot`（界面前进式 fold）不同物，故跨 crate 引用时写全名。

## 7 模块边界

**三件邻居的活，及它们各自的主人**：

- **字节怎么走**归 `bin::assembly`：WebSocket、重连、超时住装配层，本 crate 恒不持套接字，也恒不依赖异步运行时。
- **这栋楼准不准出网**归 `city::policy`：`profile` 只读 confidential 这一位，判定本身在 city。
- **页面带回来的内容算什么**归 `kernel::taint`：快照文本与工具结果同落污染环，本 crate 不另设解包面。

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
#[cfg(feature = "conformance")]                            // 恒不进构建物
pub fn assert_port_conformance<P: BrowserPort>(port: &mut P, known: &Frame);

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
pub enum Action { Click { reference: String }, Type { reference: String, text: String }, Read { reference: String } }
pub fn frame_for(session: &mut Session, context: &ContextId, snapshot: &PageSnapshot,
                 generation: u64, action: &Action) -> Result<Frame, AxError>;

// 8-5 devloop（形状 1 判定）
pub struct Observation { pub text: String, pub complained: bool }
pub enum Step { Settled { looks: u32 }, LookAgain { looks: u32 }, Complained { looks: u32 }, GaveUp { looks: u32, why: String } }
pub const LOOKS_MAX: u32 = 8;
pub const QUIET_LOOKS: u32 = 2;
pub struct DevLoop { /* 私有 */ }
impl DevLoop { pub fn observe(&mut self, observation: &Observation) -> Result<Step, AxError>; }

// 8-6 profile（形状 1 判定）
pub enum Profile { At { path: Address }, Ephemeral }
pub const PROFILES_DIR: &str = "browser-profiles";
impl Profile { pub fn of(building: &Address, confidential: bool) -> Result<Profile, AxError>; pub fn persists(&self) -> bool; }
```

## 8.5 两个设计

**第一对（缝画在哪）**：把 WebSocket 会话整体放进本 crate（落选）vs 缝只运帧、套接字归装配层（选中）。前者读起来更像「一个浏览器客户端」，但它把异步运行时拖进一个本可纯的 crate，于是所有断言都要一个 runtime，而「第二适配器」只能是一个假服务器。后者让整段会话在无浏览器、无异步的条件下逐帧断言，录制回放因此是**真的第二适配器**而不是测试替身——一台没有 WebDriver 的机器因此不再是缺口。代价：装配层多一段连接管理，且帧的 id 必须由 `Session` 铸而不能由传输层铸（否则重放会重新编号）。

**第二对（ref 是什么）**：ref ＝ 页面里的稳定标识（落选）vs ref ＝ 本次快照里的位置（选中）。前者要求页面配合（`id` 属性、`data-testid`），而页面是别人写的；后者把「页面动过了」变成一个可判定事实——ref 携 generation，陈旧即拒。代价：每次动作前必须先看一眼，这正是我们要的顺序。

## 9 工作流程

装配层连上 WebSocket → `Session::begin` → `tree` → 取一个 `ContextId` → `navigate` → `script.evaluate` 取无障碍树 → `PageSnapshot::read`（generation ＋1）→ 文本入窗 → 模型给出 `Action` → `act::frame_for` → 帧出网 → `Reply` → 若在开发回路中则 `DevLoop::observe` 决定是否再看。

## 10 实现逻辑

1. **帧先于传输**：`Frame::to_wire` 手写字段序而不用 `serde_json::to_string`，因为录制回放要按字节比对，而 map 的迭代序不是契约。
2. **回复分两层**：传输失败是 `Err`，远端拒绝是 `Reply::Error`——「这个节点没了」是答案，不是故障；把两者混同会让调用方对着一个错误码猜是谁的问题。
3. **快照用白名单不用黑名单**：`ROLES` 十四项闭合。「除了 X 都放行」会在平台新增角色时静默变宽，而它变宽的终点就是原始 DOM。
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

`ROLES` 十四项、`LABEL_MAX = 120`、`LOOKS_MAX = 8`、`QUIET_LOOKS = 2`、`PROFILES_DIR`。前两项改动即改变模型看见什么，属 15.2 的行为变更，改需证据；后三项是回路与落盘位置的约定。

## 15 影响面

新增 crate，无既有调用方。装配层将来接线时波及：连接管理、`kernel::tool` 缝上的浏览器工具、`city::policy` 的 confidential 读取。

## 16 测试与约束

逐模块 `#[cfg(test)]`；`Recording` 过 `assert_port_conformance`；「原始 DOM 恒不入窗」「字节确定性」「陈旧 generation 恒拒」「回路必有终点」四条各有一条断言。**约束**：本 crate 恒不出现 `async`、恒不依赖 `tokio`、恒不持有文件句柄。

## 17 模型体验

`to_text` 的每行是 `ref role "name"`——三个字段一行，因为模型要做的下一件事是把 ref 抄回来。拒绝词恒报出可用 ref 的数量与起点（`e1` 起），这样「我编了一个 ref」和「页面变了」在读者那里是两句不同的话。

## 18 文档同步

`ARCHITECTURE.md` §6 browser 六行与 §3 缝清单｜`docs/glossary.md` 若新增词汇｜装配层接线时同步 §6 末接线台账。

**`conformance` feature（test 恒不进构建物）**：`assert_port_conformance` 住 `#[cfg(feature = "conformance")]` 之后，否则它随发行二进制出厂，而「dev-only by contract」这句话没有任何机器持有。本工作区另外四套 conformance 同形，此规则由 `cargo xtask artifact` 持有；`crates/browser/src/session.rs` 中调用它的那条断言同样带 `#[cfg(feature = "conformance")]`，故它在 `--all-features` 下运行——那正是 `just check` 与 `just test` 所用的构建。

## 19 八个动作，与截图成为证据

### 19-1 谁起浏览器进程

这座城自己起 Firefox：随机远程调试端口、`-profile <这栋楼的 profile>`、按需 `-headless`。本库与 `docs/glossary.md` 的 **browser** 行一致——

> **假设**：起进程这件事归 `bin::browser_bidi`，本 crate 仍恒不起进程、恒不持套接字、恒不下载驱动。Firefox 是**第一引擎**（原生 BiDi，无需驱动）；Chromium 只在 `chromedriver` 已在 PATH 上时才走得通，因此是第二条路而非并列的一条。

这不放宽本 crate 的任何约束：纯的那一半仍然纯，「谁按下启动键」住在装配层（§7 第一条）。

### 19-2 `browser::verb`（新模块，形状 1 判定）

工具 `browser` 的八个动作，读成一个穷尽枚举，再变成帧。**一个动作可能要一帧以上**，所以出口是 `Vec<Frame>` 而不是 `Frame`：`open` 要先导航再装上控制台录音器，`screenshot` 带 `scale` 时要先改 devicePixelRatio。

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

依赖 `png` 0.18（MIT OR Apache-2.0，`deny.toml` 的 allow 列表已含两者）：产品路径只解码，测试用它的编码器造夹具，于是断言比的是真 PNG 字节而不是一份没人能复核的固定串。

### 19-5 `browser::devloop` 消费 `look` 的判定

`DevLoop::observe` 已经吃 `Observation { text, complained }`；要的是**接线而非新判定**：`browser` 工具的 `snapshot` 动作产出的那段文本就是 `text`，`console` 里出现过 error 级别的条目就是 `complained`，于是「改一处、看一眼、再决定」在工具层闭合，`Step` 作为工具结果回给模型。判定本身一个字不改——已有机制复用是这里的正解。

### 19-6 验收（追加到 §2）

| 单元 | 完成的定义 |
|---|---|
| verb | 八个动作各自的帧可在无浏览器下逐帧断言；`act` 无快照即拒；`measure` 的假 ref 在出网前被拒 |
| shot | 同一段 PNG 字节两次读出同一尺寸；非 PNG 不猜尺寸；quality 不引入浮点变量 |
| diff | 尺寸不同即拒；全同两图为 0；一个像素变化的框恰好含那个像素 |


### 19-7 尚未验证的部分

- **`bin::browser_bidi::BidiSocket` 没有对着真浏览器跑过**。逐帧逻辑（发一帧、读到 id 相同的那条、跳过无 id 的事件）由阅读 W3C 草案得出而非由一次真实会话验证。工具那一侧的整条 open→snapshot→act→screenshot 由 `Recording` 逐帧断言，缝的另一个适配器因此是可信的；**这一侧不是**。第一次真跑要看的是三件事：`session.new` 的能力集合是否被 Firefox 接受、`script.evaluate` 的返回值是否真是 `result.value` 的字符串形状、`browsingContext.captureScreenshot` 的 `format.type` 是否收 `image/png` 这一拼写。
- **`-headless` 有开关没有问的人**：`LaunchPlan` 带这一位并逐字断言，但 `for_building` 恒传 `false`。这一位由哪一面提供尚未定：候选是楼的 `CONFIG.toml` 与派活帧的一个字段。
