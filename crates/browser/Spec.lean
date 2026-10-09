-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.browser.spec.Act
import crates.browser.spec.Devloop
import crates.browser.spec.Keyboard
import crates.browser.spec.Profile
import crates.browser.spec.Snapshot

/-! # browser 的规格

`browser` 让一个居民驱动运行中的机器上的浏览器：看一个页面、点一个按钮、填一个输入框、按一个键、改完代码再看一眼，并且下次还认得同一个账号。本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `browser D<n>`。
-/

/-! ## 1 需求分解

七个可独立验收的单元，与模块一一对应：缝（`port`）、会话（`session`）、可见面（`snapshot`，`spec/Snapshot.lean`）、动作（`act`、`input`、`keyboard`，`spec/Act.lean`、`spec/Keyboard.lean`）、开发回路（`devloop`，`spec/Devloop.lean`）、登录态（`profile`，`spec/Profile.lean`），以及 `browser` 工具的动作面（`verb`、`shot`、`diff`、`geometry`）与量具（`survey`）。
-/

/-! ## 2 验收标准

分部里的定理是模型对性质的证明：陈旧 generation 恒拒、没看过的页面不动手；一次按键里修饰键嵌套着按下与放开；回路在 `LOOKS_MAX` 眼之内到达结局；label 无控制字符且有上界；两栋楼的 profile 互不包含。每个分部各有一条「拿掉守卫即反例」的定理。

其余单元的完成定义，由各模块旁的测试在无浏览器下经生产入口断言：

| 单元 | 完成的定义 |
|---|---|
| port | 帧的字节不依赖 map 迭代序；拒绝与读不懂的回复各有一条断言；不含 conformance 断言（D1） |
| session | 一次会话的帧序可逐帧断言；`Recording` 重放同一问题得同一答案，答不出即报出问不出口的那条 |
| snapshot | 原始 DOM 恒不入窗；同一棵树两次快照字节相同；label 超长即截断且不引入换行 |
| act | 陈旧 generation 恒拒；页面文本进表达式后，字面量内除定界符外无未转义引号 |
| devloop | 任意观察序列在预算内到达一个结局；结局枚举穷尽 |
| verb | 每个动作各自的帧可逐帧断言；`act` 无快照即拒；`measure` 的假 ref 在出网前被拒；`fetch` 是一帧、带凭据、跳转不跟，只收带主机的 http(s) 地址，`destination()` 答它的主机 |
| shot | 同一段 PNG 字节两次读出同一尺寸；非 PNG 不猜尺寸；quality 不引入浮点变量；`clip` 与 `ref` 同时给出即拒，`ref` 没有世代即拒；两张捕获帧都带上界，上界按字节读出的两侧判；矩形臂传入句柄即拒 |
| diff | 尺寸不同即拒；全同两图为 0；一个像素变化的框恰好含那个像素；解码字节短于头部时拒绝语点名是哪一张 |
| input | 指针拖拽恒是 pointerMove、pointerDown、若干 pointerMove、pointerUp；元素 origin 有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`；滚轮增量可为负 |
| keyboard | 一次按键恒是一帧 `input.performActions` 的 `key` 源，修饰键按固定次序按下、键按下又放开、修饰键逆序放开；键名与修饰键名只在 `keyboard` 的表里拼写；未知的名字在读参时即拒 |
| survey | 同一页经 `--dump-dom` 与经 `script.evaluate` 读出同一个 `probe::Read`；`xtask render` 在本仓画廊上恒零发现；住户路径的 `Sources` 为空时每条发现只点名盒子、不点名行 |
| usersbrowser | 工具名取 `ToolName::USER_BROWSER` 一个权威；未声明地址的楼每次调用都得到门的问题；声明了地址的楼其 effect 带该主机；`usersbrowser` 与 `browser` 是两个设置；confidential 楼在 `city::policy` 即拒 |
| profile | 两栋楼的 profile 互不包含；路径住保留区；confidential 楼的拒绝只有一个家（`spec/Profile.lean`） |

usersbrowser 披露给模型的参数表（`crates/sprawling/src/browser_tool/person.rs`）与 `Verb::read` 同一个变更集改：解析器是接受什么的权威，参数表是披露什么。
-/

/-! ## 3 假设与歧义

- **假设**：起进程归 `bin::browser_bidi`；本库恒不拉起浏览器进程、恒不持套接字、恒不下载驱动（D2、D4）。
- **假设（附着）**：连一个已经开着的浏览器也归装配层，本库只把帧交给缝。装配层的 `AttachedBrowser` 与 `LazyEngine` 是同一缝上的两个实现：前者只连、不启动、不结束进程；后者的 `running` 与 `Drop` 假定进程归自己，两者因此不能合成一个类型。
- **协议形状**：`input.performActions` 的 `pointer`、`wheel`、`key` 源动作字段与元素 origin 的 `SharedReference` 据 W3C 草案写成；`script.evaluate` 回复里 `sharedId` 的位置、空能力集、`image/png` 拼写已对 Gecko 真会话核过（§4）。`input::shared_id_of` 在回复里按有界深度找 `sharedId`，找不到即 `E_WIRE_MISMATCH`。
- **未定**：`-headless` 这一位由哪一面提供（楼的 `CONFIG.toml` 还是派活帧的一个字段）；某个具体 Firefox fork 是否接受本 crate 的启动参数与会话形态；行容器的交叉轴怎么扫（D11）；`fetch` 的脚本在 Gecko 上是否读出与 Chromium 相同的文字；画出来的一对颜色是否可读，接进 `xtask::color` 的对比度模型要把它开放给本 crate，另写一条对比度公式就是第二个权威（`survey::legibility`）。
- **已定**：BiDi 的 `session.new` 本版本只请求空能力加按需 `network` 事件；更多能力等到有消费者再加，每一项能力都是远端因此获得的一项许可。
-/

/-! ## 4 现状分析

纯判定与值类型：`session`、`snapshot`、`act`、`verb`、`input`、`keyboard`、`shot`、`diff`、`devloop`、`profile`、`geometry`、`survey` 与缝 `port`。生产消费者是 `crates/sprawling`：`browser_bidi` 持套接字并实现 `BrowserPort`，`browser_tool` 把动作面接成 `browser` 与 `usersbrowser` 两件工具；`xtask render` 读 `survey`。

**对真浏览器核过的部分。** 对 Gecko 的一次真会话核过了 §3 的协议形状：`session.new` 接受空能力集；`script.evaluate` 的回复是 `result.result = { type, handle, sharedId, value }`，`sharedId` 就在这一层；`browsingContext.captureScreenshot` 的 `format.type` 收 `image/png`。元素裁剪随之落地：`clip.type = "element"` 收由 `script.evaluate` 回复里取出的 `sharedId`，回来的图正好是该元素的框。同一次会话量到驱动原样忽略 `imageSize`：2000×1500 的视口带 `maxWidth:1920` 仍回 2000×1500，400×300 的 clip 带 `maxWidth:200` 仍回 400×300，所以上界靠重拍生效（D7）。

`fetch` 的脚本对 Chromium 引擎核过：headless Chromium 打开一个回环地址上的页面，页面里跑 `fetch_script` 取同源的一份 HTML，读回 `status` 200、`content-type`、最终地址，正文是「标题、空行、各块一行」的文字，`script`、`style`、`noscript` 的内容不在其中，段落里的源码换行收成空格，`pre` 里的换行与缩进原样保留。同一段脚本在 Bun 的 `fetch` 下读 JSON、纯文本（超上限时 `cut` 为真并报原长）、图片（只报 `binary` 与字节数）、302（停在跳转本身）与连不上的地址（`error` 带页面的原话）。

`-headless` 有开关没有问的人：`LaunchPlan` 带这一位并逐字断言，但 `for_building` 恒传 `false`。引擎的名字已经是查表：`host::firefox` 走 `doctor` 的 `gecko` 条目（`Need::OneOf(Group::BrowserEngine)`），家族表有 firefox、zen、librewolf、waterfox、floorp、firefox-developer、firefox-nightly、tor-browser 八行，`SPRAWLING_BROWSER` 可压过其一，所以只有 fork、没有 Firefox 的机器也起得来。
-/

/-! ## 5 权威信源

| 事实 | 出处 |
|---|---|
| 命令形 `{id, CommandData, Extensible}`、模块划分 | <https://www.w3.org/TR/webdriver-bidi/> |
| `browsingContext` 语义（context 即可载入文档的 navigable） | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/browsingContext> |
| `script` 模块 | <https://developer.mozilla.org/en-US/docs/Web/WebDriver/Reference/BiDi/Modules/script> |
| `input` 模块（指针、滚轮、按键；`performActions` 的源动作与元素 origin） | <https://w3c.github.io/webdriver-bidi/#module-input> |

规范是 W3C 工作草案；本库用 `session`、`browsingContext`、`script`、`input` 四个模块，`network` 只作为可选订阅出现。键名取 DOM `KeyboardEvent.key` 的值，码位取 WebDriver 的键码表。
-/

/-! ## 6 命名统一

`BrowserPort`、`PageSnapshot`、login state per Building，三者取自词汇表，恒不自造同义词。「快照」在本 crate 恒指 `PageSnapshot`，与客户端的界面 fold 不同物，跨 crate 引用时写全名。模型里的声明名取同样的词：`Browser.Act`、`Browser.Keyboard.press`、`Browser.Devloop.Loop`、`Browser.Profile.of`、`Browser.Snapshot.label`。
-/

/-! ## 7 模块边界

三件邻居的活，及它们各自的主人：

- **字节怎么走**归 `bin::assembly`：WebSocket、重连、超时住装配层，本 crate 恒不持套接字，也恒不依赖异步运行时（D2）。
- **这栋楼准不准出网**归 `city::policy`：`Profile::of` 只收楼的 `Address`，本 crate 读不到 policy；confidential 的判定在 city（`spec/Profile.lean`）。
- **页面带回来的内容算什么**归 `kernel::taint`：快照文本与工具结果同落污染环，本 crate 不另设解包面。
- **哪一页该被勘察、勘察完谁去改**归调用方：`xtask render` 是门，判的是本仓画廊；`browser` 工具的 `survey` 判的是住户自己打开的那一页。本 crate 只回答「这一页哪里不对」，不回答「该不该红」，`Standing` 由读者解释（D11）。

约束：本 crate 恒不出现 `async`、恒不依赖 `tokio`、恒不持有文件句柄。
-/

/-! ## 8 接口先行

签名的权威是 Rust 源码；这里记每个模块的形状与它为什么是这个形状。

- **`port`**（形状 3 端口、形状 2 值类型）：`Frame::new(id, method, params)`、`to_wire`（字段序固定）；`Reply::{Success, Error}`、`parse`、`into_result`（远端的拒绝带着它自己的词过来）；`trait BrowserPort { fn send(&mut self, frame: &Frame) -> Result<Reply, AxError>; }`（D1）。
- **`session`**（形状 4 适配器、形状 2）：`ContextId`、`SessionRequest { network }`（默认全关）、`Session::{begin, tree, navigate, evaluate, frame, end, read_tree}`；`Recording` 是第二适配器，`answer` 与 `missed`。帧的 id 由 `Session` 铸，不由传输层铸，否则重放会重新编号。
- **`snapshot`**（形状 1 判定、形状 2）：`Node { reference, role, name }`、`PageSnapshot::{read(generation, tree), to_text, resolve}`。`to_text` 的每行是 `ref role "name"`；拒词恒报出可用 ref 的数量与起点（`e1` 起），于是「我编了一个 ref」与「页面变了」在读者那里是两句不同的话。
- **`act`**（形状 1 判定）：`Point { x, y }`（视口坐标，CSS 像素，滚轮增量可为负）、`Origin::{Reference, Point}`、`Action::{Click, Type, Read, Drag { from, to, steps }, Scroll { at, by }, Press { key, modifiers }}`、`STEPS_MAX`；`Action::reference` 答 `Option`，`resolves_element` 说明哪一臂要多一帧；`frame_for` 只管 script 的三臂，`resolve_frame` 是元素 origin 的第一帧（`spec/Act.lean`）。
- **`input`**（形状 1 判定）：`DragPath`、`Origin::{Element, Viewport}`（经根重导出为 `ResolvedOrigin`）、`pointer_frame`、`wheel_frame`、`shared_id_of`（D9）。
- **`keyboard`**（形状 1 判定）：`NamedKey`、`Key::{Named, Char}`、`Modifier::{Control, Shift, Alt, Meta}`、`Key::parse`、`Modifier::parse`、`key_frame`（D12，`spec/Keyboard.lean`）。
- **`devloop`**（形状 1 判定）：`Observation { text, complained }`、`Step::{Settled, LookAgain, Complained, GaveUp { why }}`、`LOOKS_MAX`、`QUIET_LOOKS`、`DevLoop::observe`（`spec/Devloop.lean`）。
- **`profile`**（形状 1 判定）：`Profile::of(building)`、`path`、`PROFILES_DIR`（`spec/Profile.lean`）。
- **`own`**（形状 1 判定）：`OwnListeners::{new, holds}`、`page_address(tree, context)`；一个地址是不是城自己的某个监听（D13）。`Verb::address` 给出 `Open` 与 `Fetch` 的整个地址，守卫据此连端口一起判。
- **`verb`**（形状 1 判定）：`Verb::{Open, Snapshot, Act { generation, action }, Screenshot, Measure { references }, Survey, Fetch { url }, Console, Viewport { width, height }, Close}`；`read`、`frames`（出口是 `Vec<Frame>`，一个动作可能要一帧以上）、`destination`（`Open` 与 `Fetch` 的主机，门据此判出网）、`input_frame`（D5、D6）。
- **`shot`**（形状 2 值类型）：`Clip::{Rect, Element, Union}`、`Rect`、`ShotRequest { clip, format, quality, scale }`、`Shot::read`、`ShotRequest::{waits_for_page, capture_frame, refit_frame}`、`ShotMaxEdge::admit`、`SHOT_MAX_EDGE_PX`（D7）。
- **`diff`**（形状 1 判定）：`diff(a, b) -> Difference`，变了万分之几（`changed_ppm`、`ratio_q4`）与变在哪几个框里（D8）。
- **`geometry`**（形状 2 值）：一个元素的框只有一种读法，`measure`、`refs` 截图与 `survey` 都拼进同一个脚本函数。
- **`survey`**（量具，形状 1 判定）：`probe::body`、`probe::evaluated`、`probe::Read`、`Sources`、`Standing`（D11）。
-/

/-! ## 9 工作流程

装配层连上 WebSocket，`Session::begin`，`tree`，取一个 `ContextId`，`navigate`，`script.evaluate` 取无障碍树，`PageSnapshot::read`（generation 加一），文本入窗；模型给出 `Action`，`Verb::frames` 给出帧，帧出网，`Reply` 回来；元素 origin 的拖拽与按引用的截图在第一帧的回复之后补第二帧；若在开发回路中，则 `DevLoop::observe` 决定是否再看。
-/

/-! ## 10 实现逻辑

1. **帧先于传输**：`Frame::to_wire` 手写字段序而不用 `serde_json::to_string`，因为录制回放要按字节比对，map 的迭代序不是契约。
2. **回复分两层**：传输失败是 `Err`，远端拒绝是 `Reply::Error`。「这个节点没了」是答案，不是故障；混同两者会让调用方对着一个错误码猜是谁的问题。
3. **快照用白名单**：角色词汇十四项闭合；「除了 X 都放行」会在平台新增角色时静默变宽，而它变宽的终点就是原始 DOM。词汇只有一个家：`snapshot::ROLE_MAP`（`(标签, 可选 type) → 角色`，二十七行）是权威，采树脚本的映射表由 `role_lookup_js()` 从它生成，过滤器 `shown()` 也从它取值。脚本与过滤器若各持一张表，只要两张不重合，不手写 `role=` 的页面快照里就没有链接也没有输入框；手填角色的夹具绕过脚本，测不出这一点。两条派生断言：`ROLE_MAP` 产得出的每个角色都在词汇里；词汇里除 `tab` 与 `alert`（无元素隐含，只能由页面显式声明）之外的每一项都至少有一个标签映射到它。`<input>` 的 type 缺席读作 `text`（HTML 默认值）；`hidden`、`file` 等无行可查的 type 不得角色。
4. **动作里页面文本恒是数据**：`quote` 是页面内容成为代码的唯一位置（`spec/Act.lean`）。
5. **回路必有终点**：`LOOKS_MAX` 与 `QUIET_LOOKS` 把「不收敛」变成一个结局（`spec/Devloop.lean`）。

D1 缝上有 trait，没有 conformance 套件。`BrowserPort` 是 trait，因为缝上有多个生产实现：`crates/sprawling/src/browser_bidi` 的 `BidiSocket`（直连）、`LazyEngine`（按需起引擎、首帧才连）与 `AttachedBrowser`（连人自己的浏览器），加上本 crate 的 `Recording`；撤 trait 会让这几条路合成一个类型。不设套件，因为它的证据为零：被测者 `Recording` 按构造就回 `frame.id()` 且回答不消费条目，断言恒真；生产适配器跑不了它，CI 没有浏览器驱动（ARCHITECTURE.md §11 具名的缺口之一）。这样的套件会让读者把「过了 conformance」读成「传输层被验过」。录制回放是重放证据，不是传输层的证据。重开参数：回复路由规则（读过无 id 的事件、按 id 认领答案）从 `socket.rs` 的 async 循环搬进本 crate 成为纯函数，且 `crates/sprawling` 的测试能用一对本地 socket 驱动它；那时套件与它的调用方在同一次改动里出现。

D2 缝只运帧，套接字归装配层。被否决的是把 WebSocket 会话整体放进本 crate：读起来更像「一个浏览器客户端」，但它把异步运行时拖进一个本可纯的 crate，所有断言都要一个 runtime，第二适配器只能是一个假服务器。选中的做法让整段会话在无浏览器、无异步的条件下逐帧断言，录制回放能在一台没有 WebDriver 的机器上重放一次真实会话；它替代不了传输层的证据（D1）。代价：装配层多一段连接管理，帧的 id 必须由 `Session` 铸。

D3 ref 是本次快照里的位置。被否决的是 ref 等于页面里的稳定标识：它要求页面配合，而页面是别人写的。位置让「页面动过了」变成可判定的事实，ref 携 generation，陈旧即拒（`spec/Act.lean`）；代价是每次动作前必须先看一眼。

D4 这座城自己起 Firefox：随机远程调试端口、`-profile <这栋楼的 profile>`、按需 `-headless`。Firefox 是第一引擎（原生 BiDi，无需驱动）；Chromium 只在 `chromedriver` 已在 PATH 上时才走得通，所以是第二条路而非并列的一条。这不放宽本 crate 的任何约束：谁按下启动键住在装配层（`bin::browser_bidi`），本 crate 仍不起进程、不持套接字、不下载驱动；与 `docs/glossary.md` 的 **browser** 行一致。

D5 一个动作可能要一帧以上，所以 `Verb::frames` 的出口是 `Vec<Frame>`：`open` 先导航再装上控制台录音器，`screenshot` 带 `scale` 时先改 devicePixelRatio。三条判定写在 `verb` 而不是调用方：`act` 没有快照即拒（对没看过的页面动手不可拼写，D3 的直接后果）；`measure` 的每个 ref 都过 `PageSnapshot::resolve`，编出来的 ref 在出网前就被报出；`console` 读的是 `open` 时装上的页面内数组，因为本 crate 的缝只运请求与应答，而 BiDi 的 `log.entryAdded` 是无 id 的事件，归装配层路由，用一个页面内数组换一条事件订阅是复用已有机制而非新开通路。`Verb::Survey` 是新的一臂，不是 `Measure` 的扩展：`Measure` 回答「我点名的这几个节点在哪」，survey 不接引用、读整页、返回判决；合成一臂会让 `references` 在一半调用里恒为空。

D6 `fetch` 用页面自己的 Fetch API 取一份文字。它是一帧 `script.evaluate`（`awaitPromise`），在已打开的页面里跑 `fetch(url, { credentials: "include", redirect: "manual" })`：请求从这个页面发出，带着它的 cookie，守它的同源与 CORS 规则；跨源而对方不许带凭据时，页面自己的 `TypeError` 原样回来，本城不另开一条绕过 CORS 的路。回来的是一个 JSON 串：`status`、`type`、`url`（最终地址）、`text`、`cut` 与 `chars`。HTML 由 `DOMParser` 解析、不挂进活文档（挂进去就会取图、跑脚本），删掉 `script`、`style`、`noscript`、`template`，按块级元素换行、收拢空白，`<title>` 放第一行；JSON、XML 与 `text/*` 原样；其余类型只报 `binary` 与字节数。每个收到请求的主机都被门判过：`destination()` 答 `url` 的主机；`redirect: "manual"` 让跨主机跳转停在跳转本身（`opaqueredirect`，状态 0），模型要跟就用新地址再 `fetch`，那一次同样过门。被否决的是 `follow`：跳转目标是门从没见过的主机。只收带主机的 http(s) 地址：相对地址、`file:` 与 `data:` 在 `Verb::read` 即拒，因为相对地址按页面的 `<base>` 解析，`<base>` 可以指向任何主机，门看不见。正文在页面里截到 `FETCH_TEXT_MAX_CHARS`（65 536 个 UTF-16 单位）并报出原长；它与 `snapshot` 不重复：`snapshot` 读渲染后的无障碍树，`fetch` 读一个地址的正文，不渲染、不执行、不改页面。

D7 截图的区域三臂互斥，框只有一个来源，上界由两端说住。`quality` 与 `scale` 是整数（0 到 100 的百分比），BiDi 要的小数只在最后一刻由 `format!("0.{q:02}")` 写成 JSON 数，本仓库没有一个 f64 变量。`clip` 是四个整数，`ref` 加 `generation` 是快照铸出、世代守卫的引用，`refs` 加 `generation` 取一组引用的包围矩形；三臂而不是三个可空字段，因为一块区域只有一个来处。并集是协议装不下的那一臂：BiDi 没有多元素裁剪，所以先发 `measure` 用的那段取框脚本，`capture_frame` 在 Rust 里求并（checked 算术，越界即拒），再以矩形臂拍，截图覆盖的区域与 `measure` 报出的区域是同一次读数。`ref` 走一帧换一帧：`resolve_frame` 让页面报出元素，`shared_id_of` 取出句柄，`capture_frame` 发捕获帧，元素裁剪的矩形由页面报给驱动，不由本仓计算；矩形臂传入句柄、或元素臂没有句柄，都回 `E_INVALID_ARGS`。每张截图带上界：捕获帧恒带 `imageSize {maxWidth, maxHeight}`，两侧同为 `SHOT_MAX_EDGE_PX`（1920），但驱动不执行它（§4），所以收下后按字节读出的两侧判，超界就按实测长边算出比例重拍一次（`refit_frame`，比例是上界除以长边乘调用方要的密度，永不放大），重拍后仍超界才拒（`ShotMaxEdge::admit`）。重拍而不是自缩字节：自己重采样等于新增依赖与自己的 CPU；devicePixelRatio 改的是页面可见的事实，可能让页面换一套样式重排，所以只在与「拒绝」二选一时付。重开参数：驱动开始执行 `imageSize`。上界不住 `kernel::policy_limit`：它的两个读者都在本 crate；第二个 crate 要这个数时（桌面侧截图并入同一份词汇即是），它搬进 `consts_policy`。不静默降级：驱动不接受元素裁剪时以驱动自身的拒绝失败，本库不偷偷退回自己算框。`Shot::read` 只在 PNG 上给出尺寸，其他格式回 `E_WIRE_MISMATCH` 而不是猜：`ImageRef` 的宽高是模型看图前唯一的尺度，猜错的尺寸比没有尺寸更坏。

D8 `diff` 的百分比是万分比整数（`changed_ppm`、`ratio_q4`），框是像素坐标的整数矩形，因为两者会进账本载荷。尺寸不同的两张图不比较，回 `E_INVALID_ARGS`：缩放后再比，比出来的是缩放算法的差异。解码后的字节短于头部声明的尺寸时回 `E_WIRE_MISMATCH` 并说出是哪一张短了（先拍的、后拍的、两张都短），因为三种情况的下一步不同；判定对前后两个像素取值穷尽匹配。全 crate 只有一个矩形类型 `shot::Rect`（`covers` 与 `covering` 是它的方法），`diff` 从 `shot` 读它。`png` 依赖：产品路径只解码，测试用它的编码器造夹具，断言比的是真 PNG 字节。

D9 拖拽与滚动与 `desktop.act` 同一份词汇：`drag` 从 ref 或 point 到 point，`scroll` 用 `to` 表示滚多远，`steps` 为中间移动次数，两侧字段名与含义逐字相同（`crates/desktop/Spec.lean` §8-7 指向这里）。BiDi 的元素 origin 用页面自己的 shared id 而不是选择器，所以元素起点的拖拽在线上是两帧：`act::resolve_frame` 让页面报出元素，`input::shared_id_of` 读出 id，`input::pointer_frame` 发 `input.performActions`；`Verb::frames` 只发第一帧，工具在 `invoke` 里补第二帧，判定仍在纯代码里，`Recording` 能逐帧重放。`Scroll` 与 point 起点的 `Drag` 没有元素，所以 `Action::reference()` 是 `Option`。

D10 `usersbrowser` 驱动人已经开着的那个浏览器，用的是那个人的真 profile；与 `browser` 的差别是安全模型而不是动作集合。地址是人的声明：`RULES.toml` 的 `usersbrowser` 一键，值为 `ws://127.0.0.1:<port>/session` 时启用该工具并把地址交给 attach 门；值为 `true` 时启用而地址未定，每次调用都得到门的问题（`E_APPROVAL_PENDING`）；absent 或 `false` 即无此工具；confidential 楼写这一键即在规则读取处被拒。恒不关人的浏览器：`AttachedBrowser` 只连、不启动、不结束进程，`Verb::Close` 结束的是一次会话；附着是会话级、绑一个 run。入账：附着是工具的第一次调用，与之后每个动作一样写 `tool_called` 与 `tool_result`；`disclosure` 写明它需要人先批准并引导先用 `browser`。平台的门就是授权：Firefox 走 `--remote-debugging-port`、Chromium 走驱动，两者都要求人的动作；地址是否 loopback 由 `gate::attach` 判，非 loopback 的声明被拒，因为那会把登录态读过一个网络。

D11 一页哪里画错了，由一份测量、一套判决回答，门（`xtask render`）与工具（`survey` 动作）读同一份；它住产品 crate，门与工具各留一份就会分叉到「门说干净、住户说坏」而二者各自诚实。`probe::body` 是注入页面的 ES5，返回三个字符串（元素、声明的词、绘制条件）：门渲染一次并 dump DOM，三串写进三个 `<pre>`；住户勘察的是人自己打开的页面，不许往上加任何东西，所以 `probe::evaluated` 把同一段包进一个 promise，三串作为一次 `script.evaluate` 的值回来。主题由调用方决定：门为每个 pass 强制一个主题，住户传 `None`，因为强制主题报的是没人看过的那一页；`PaintSource` 同理。源码索引在有源码树的那一侧：`Sources` 的查找随判决进产品 crate，走源码树的那一步留在 `xtask`，住户得到的每条发现只点名盒子，这是诚实的空状态（`Sources::default()`）。结果答一个 tagged 字符串（一个 `<edit>` 一处修复、一个 `<at>` 一个落点），拆成结构化载荷等于第二种渲染。量具的两条规则：容差 `SLACK`（1 px）在每一次缘比较上都加；群体先从几何读出容器的堆叠方向，只比容器不分发的那一轴。行容器的交叉轴不扫：带里的位置由 `align-items` 决定，本库到处用居中，不同行高的子元素按设计就有不同顶缘；正确读它要比较顶、中、底里多数实际持有的那一个，这把尺子还没有这个读数（§3）。

D12 一个 run 能按键：`Action::Press { key, modifiers }`，一帧 `input.performActions` 的 `key` 源（`spec/Keyboard.lean`）。客户端的键表（`client/Spec.lean` §9）因此有了机器读者，验收可以由本产品驱动本产品。键名与修饰键名取 DOM `KeyboardEvent.key` 的值，一个字符键就写那个字符；名字、WebDriver 码位与枚举在 `keyboard` 的一处对应，`Key::parse`、`Modifier::parse` 与 `key_frame` 都读它，`verb::read` 经这两个 `parse` 取值。按键落在焦点上：`Press` 不带 ref，不需要第二帧；要先让某个元素得到焦点，是先做一次 `click`。没有快照即拒，没有 ref 所以不核 generation。不换算大小写：`K` 送出的就是 `K`，`Shift` 另是一个修饰键。读参：`kind` 为 `press`，`key` 必填，`modifiers` 可选、是字符串数组；未知的名字各以 `E_INVALID_ARGS` 拒绝，恢复语列出能用的名字。被否决的备选：在 `Type` 的文字里夹转义（同一个字符串既是文字又是键）；`"Ctrl+K"` 形式的小语言（键名之外的第二套文法）。

D13 「这个地址是不是城自己的一个监听」是本 crate 的一个纯判定：`OwnListeners::new(listeners)` 收城此刻监听的 `SocketAddr`，`OwnListeners::holds(url)` 答一个 `http`、`https`、`ws` 或 `wss` 地址是否落在其中之一上：端口相同（不写端口时取 scheme 的默认端口），并且主机是回环（`localhost`、`*.localhost`、回环地址，`::ffff:127.0.0.1` 也算），或者等于那个监听绑定的地址，或者那个监听绑定在未指定地址（`0.0.0.0`、`::`）上而主机是一个 IP 字面量。`page_address(tree, context)` 从 `browsingContext.getTree` 的答复里读出一个 tab 此刻的地址。装配层持登记、按它拒（`crates/sprawling/spec/BrowserBidi.lean` §8-45-6，sprawling D74）。不解析名字：一次 DNS 查询会让判定依赖网络。被否：按主机名单判（`127.0.0.1`、`localhost`）不看端口——居民在回环上起的开发服务器也会被拒。

成本：`to_text` 一行三个字段，因为模型下一件事是把 ref 抄回来；拒词报出可用 ref 的数量与起点。
-/

/-! ## 11 边界枚举

空 method；非对象 params；无 id 的回复；`type` 未知；`contexts` 缺失；节点无 role；label 超长；label 含控制符；ref 不是 `e<n>`；`e0`；陈旧 generation；回路超预算；房间地址当楼名；保留区当楼名；drag 同时给 ref 与 point 或都不给；`steps` 越界；`fetch` 的相对地址、`file:` 与 `data:`；`clip` 与 `ref` 同给；非 PNG 要尺寸；两张图尺寸不同；未知的键名与修饰键名。
-/

/-! ## 12 错误处理

| 码 | 何时 | 能否让它不可能发生 |
|---|---|---|
| `E_INVALID_ARGS` | 帧构造、ref 解析、楼名不是楼、动作参数读不成 | 部分能：ref 已由快照铸造，非法 ref 只能来自模型自造的字符串 |
| `E_WIRE_MISMATCH` | 回复或树的形状读不出、元素 id 找不到、非 PNG 要尺寸、解码字节短于头部 | 不能：对侧版本不由本库决定，故 fail closed |
| `E_BROWSER_UNAVAILABLE` | 远端拒绝、重放缺答案 | 不能：这是外部世界的事实 |
| `E_LOOP_SUSPECTED` | 结局之后继续观察 | 能：调用方持 `Step`，越过结局是它的错，故报出调用方 |

每个拒绝是三段式的 `AxError`；本 crate 的判定不改变任何状态，拒绝之后会话、快照与回路都原样留着。传输失败与远端拒绝分两层（§10 第 2 条）。
-/

/-! ## 13 依赖选型

`kernel`（错误、地址）、`serde` 与 `serde_json`（帧与探针的三串）、`base64` 与 `png`（截图解码与量尺寸）。恒不引入 WebSocket 客户端、异步运行时、HTML 解析器：前两者归装配层，第三者会把原始 DOM 请回本 crate。规格的分部不 import 任何别的 crate 的规格。
-/

/-! ## 14 硬编码声明

`ROLE_MAP` 二十七行与 `ARIA_ONLY` 两项（合为十四个角色）、`NAME_MAX_JS = 200`（脚本回传的名字上限，与展示上限不同事）、`LABEL_MAX = 120`、`LOOKS_MAX = 8`、`QUIET_LOOKS = 2`、`STEPS_MAX = 32`、`SHOT_MAX_EDGE_PX = 1920`、`FETCH_TEXT_MAX_CHARS = 65536`、`PROFILES_DIR`、十三个命名键与四个修饰键的 WebDriver 码位。前两项改动即改变模型看见什么，改需证据；其余是回路、帧与落盘位置的约定。模型里的 `LABEL_MAX`、`LOOKS_MAX`、`QUIET_LOOKS` 与 Rust 的同名常量取同一个值，由各自模块旁的测试守着。
-/

/-! ## 15 影响面

改 `BrowserPort` 或 `Verb` 波及 `crates/sprawling` 的 `browser_bidi` 与 `browser_tool`；改 `survey` 波及 `xtask render`；`city::policy` 的 confidential 读取决定一栋楼有没有这两件工具；改键名或修饰键名波及客户端键表的读者与 `browser_tool` 披露的参数表。
-/

/-! ## 16 测试与约束

证明：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`。咬得动的演示：`Browser.Act.withoutGuard_acts_on_a_stale_page`、`Browser.Keyboard.withoutReverse_does_not_nest`、`Browser.Devloop.withoutGiveUp_looks_again_at_the_limit`、`Browser.Snapshot.withoutCleaning_keeps_a_newline`、`Browser.Profile.rooms_would_nest`。

实现一致性：逐模块 `#[cfg(test)]`；「原始 DOM 恒不入窗」「字节确定性」「陈旧 generation 恒拒」「回路必有终点」「按键嵌套」各有一条断言；`cargo nextest run -p sprawling-browser`。模型的证明不是 Rust 实现的证明。
-/

/-! ## 17 文档关系

- `architecture.toml` 里 browser 各行（锚点指向本文件与分部）与 ARCHITECTURE.md §4 缝清单（`BrowserPort`）；ARCHITECTURE.md §11 的 V3 一行引 D1。
- `docs/glossary.md` 的 **browser** 行（D4）与新增词汇。
- `crates/desktop/Spec.lean` §8-7（拖拽与滚动的同一份词汇，D9）；client/Spec.lean §9（键表，D12）；`crates/city/Spec.lean` §8-2 的 policy（confidential 与 `usersbrowser`，D10）；`tools/xtask/Spec.lean` 的 `render` 一节与 §8-26 `survey`（D11）；`crates/sprawling/Spec.lean` 的 `browser_tool` 一节。这些节改了，重读本文件对应的决定。
- 本 crate 没有 `conformance` feature（D1）。
-/
