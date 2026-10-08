-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::browser_bidi

规定 `crates/sprawling/src/browser_bidi.rs` 与 `crates/sprawling/src/browser_tool.rs`：居民经 BiDi 驱动的浏览器，与按楼的规则交给 run 的浏览器工具（`bin::browser_bidi`、`bin::browser_tool`）。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::browser_tool::tests`、`bin::browser_tool::tests::recordings` 守住。
-/

/-!
## 8-45 一个能看见自己造出来的东西的居民（`bin::browser_bidi`、`bin::browser_tool`）

### 8-45-1 谁按启动键

`browser` crate 是纯的：帧进帧出，无套接字、无进程、无异步。启动一个引擎与端着一条 WebSocket 因此落在装配层，这正是 ARCHITECTURE §3 的「装配边」——运行期存在、只在 `bin` 里存在的那一类边。

**Firefox 是第一引擎**：它原生说 BiDi，不需要任何驱动，所以一台只装了 Firefox 的机器就是一台能用的机器。命令行三件：`--remote-debugging-port <随机端口>`、`-profile <这栋楼的 profile 目录>`、按需 `-headless`。端口随机是因为同一台机器上可能有第二座城在开着第二个浏览器，而一个固定端口会让第二座城静默连到第一座城的浏览器上。

**Chromium 是第二条路**，且只在 `chromedriver` 已在 PATH 上时存在。这不是并列的两个后端：Chromium 的 BiDi 要经 `chromedriver` 转，驱动不在就是不在，此时回一句点名的拒绝而不是沉默降级。

```rust
pub(crate) enum Engine { Firefox { program: String }, Chromium { driver: String } }
pub(crate) struct LaunchPlan { pub(crate) program: String, pub(crate) args: Vec<String>, pub(crate) port: u16 }
impl Engine {
    pub(crate) fn plan(&self, profile: &Path, port: u16, headless: bool) -> LaunchPlan;
}
```

**本模块是三个文件**（400 行的价目表逼出来的一次切分，切在三件事之间而不是切在行数上）：`engine.rs` 判「哪个引擎、命令行长什么样」，`lazy.rs` 持「还没起的那个引擎」与端口推导，`socket.rs` 只运字节。`browser_bidi.rs` 因此是索引，一行逻辑也没有。

`plan` 是纯函数，端口由调用方给，于是「参数长什么样」这件事在没有浏览器的机器上也逐字可断言；`launch` 只是 `Command::spawn` 加一个「进程死了就报出来」。**时间与随机都不在这里取**（ARCHITECTURE §10 第 2、4 条）：端口由 `port_for(city_root)` 从城目录的 BLAKE3 摘要推出，落在 40000–59999。这既不采时钟也不取熵，而同一台机器上的两座城本来就在不同目录里——用已经把它们区分开的那件事去区分端口，比再引入一个随机源更少一处不确定性。等待浏览器起来靠**敲门次数**而不是截止时刻，因为读时钟的地方只有 `bin::assembly` 一处。

### 8-45-2 `bin::browser_tool`——一张动作表，一个会话

工具住这里而不是 `browser` crate，理由是截图要落 `storage::cas`，而 `browser` 依赖图里没有 `storage`，也不该有。把 CAS 塞进 browser 会多一条本可不存在的依赖边；把工具放在装配层，`browser::verb` 的判定与 `storage::cas` 的字节各自留在自己那侧，中间只有一个 `Shot` 值。

工具持有：一个 `Box<dyn BrowserPort>`、一个 `Session`、当前 `ContextId`、上一次 `PageSnapshot`（快照的 generation 由它递增）、一个 CAS 句柄。动作即 `browser::verb::Verb` 的变体（`crates/browser/Spec.lean` 的 D5），一个不多一个不少；`fetch` 的回复在这里变成 `status`、`type`、`url`、`text`、`cut`、`chars` 六个字段的结果，非文本的回复变成 `binary` 与 `bytes`，页面自己的拒绝变成 `error`。

`effect` 是 `Effect::Egress`：浏览器打开的每个 URL 都离开运行中的机器，所以它过出网门，confidential 楼因此天然拿不到它。

**`act` 的结果说出它落在哪里**（`browser_tool::answering`）：`acted` 一格是快照给的 ref；指针动作没有 ref 时是 `viewport`；按键（`Action::Press`）是 `focus`，因为一次按键落在页面此刻的焦点上，不带 ref，也不碰视口（`crates/browser/Spec.lean` 的 D12）。三个词按 `Action` 的臂穷尽给出，不从「有没有 ref」反推——那样按键会被记成 `viewport`，读账本的人看到的是一次没发生过的指针动作。

**`usersbrowser` 披露的参数表与 `Verb::read` 读的参数一一对应**（`browser_tool::person`）：`kind` 的枚举含 `press`，`key` 与 `modifiers` 各有一格；键名与修饰键名的清单不抄进描述，描述只说它们取 DOM `KeyboardEvent.key` 的名字，未知的名字由 `Key::parse`、`Modifier::parse` 拒，拒词列出能用的名字——清单的家仍是 `browser::keyboard` 一处。同一个参数名只写一格：`ref` 与 `refs` 的说明把 `act`、`measure` 与 `screenshot` 写在同一句里，因为 JSON 对象里后写的同名键会静默盖掉先写的，模型只会读到后一种用法。

**worker 经它被交到的 `fn` 指针拿这些工具**：

```rust
/// 楼的规则要的浏览器工具：先是楼自己的，再是这个人声明过的那一个。
pub type Browsers = fn(&Path, &storage::BlockOrigin, &city::BuildingRules) -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;
impl RunWorker {
    pub fn with_browsers(self, browsers: Browsers) -> RunWorker;   // 生产装 browser_tool::for_rules
}
```

`lay_out_workbench` 调的是 `RunWorker.browsers`，不直接调 `browser_tool::for_rules`：本模块起浏览器、经 BiDi 说话，worker 搬进 `accounting` 时它留在 `sprawling`（`crates/accounting/Spec.lean` §7、accounting D10）。换掉它放不宽机密：`city::policy` 在机密楼上拒绝 `browser` 与 `usersbrowser` 两项设置，楼的规则里就没有要浏览器的那一句。钉住它的测试是 `crates/sprawling/tests/browsers.rs` 的 `a_run_is_offered_the_browser_the_worker_was_handed`。

### 8-45-3 截图成为证据

一次 `screenshot` 的落点有三处，缺一处这张图就不是证据：

1. 字节进 `storage::cas`，得到一个 `cas:b3-…` 定位符——历史里恒不出现图片字节；
2. 结果载荷带上定位符与两个整数尺寸，于是模型即使不看图也知道它有多大；
3. `ToolOutcome.attachments` 带上 `ImageRef`，`runtime::turn::wave` 把它原样放进 `ContentBlock::ToolResult.attachments`，于是这张图真的到得了模型眼前。

**`kernel::ToolOutcome` 因此加一个字段** `attachments: Vec<ImageRef>`，`#[serde(default)]`，旧历史读成空列表。这是唯一一处跨 crate 的形状变更，波及每一个构造 `ToolOutcome` 的工具（全部改为显式空列表），不改任何一个的行为。`ContentBlock::ToolResult.attachments` 与两条 dialect 备好，`wave.rs` 里那句「a tool that produces a picture fills this in where it runs」等的就是这一步。

### 8-45-4 `RULES.toml` 的 `browser` 的真假两值

`city::policy` 多读一个键。默认 **false**：一栋楼不写这行，它的居民就没有浏览器。这与 `confidential` 的「不写即报错」不同，理由是两者的失败方向相反——隐私设置读成宽松的一侧是事故，而工具没给到只是少一件工具。confidential 楼恒为 false，写了 `browser = true` 即拒，因为一个能开任意 URL 的浏览器就是一条出网路径，而「数据不出去」是那栋楼的全部意思。

### 8-45-5 验收

| 单元 | 完成的定义 |
|---|---|
| browser_bidi | Firefox 与 Chromium 的参数各自逐字断言；驱动不在即点名拒绝；两次 plan 的端口来自参数而非采样 |
| browser_tool | 录制适配器上重放 open→snapshot→act→screenshot 一整条；截图后 CAS 里有字节、载荷里有定位符与尺寸、attachments 里有一个 `ImageRef` |
| RULES.toml | 不写 `browser` 即没有；confidential 楼写 `browser = true` 即拒 |
-/

/-!
### 8-45-6 居民的浏览器不碰城自己的源（`bin::browser_tool::guarding`）

城在哪些地址上监听，由装配层在监听绑定之后登记：城自己的监听在 `bin::assembly::listening` 绑定之后登记一次，远程监听在 `bin::outside::listener` 开门时登记、关门时撤销。两件浏览器工具（`browser` 与 `usersbrowser`）每一次调用都先读这份登记，得到一个 `browser::OwnListeners`，再按它判：

```rust
pub(crate) fn serve(at: SocketAddr) -> Served;        // bin::browser_tool::guarding；Served 被丢掉时撤销
pub(crate) fn own() -> Result<OwnListeners, AxError>;  // 此刻的登记
```

- `open` 与 `fetch` 的地址是城自己的源，拒（`E_GATE_DENIED`），不发任何一帧；
- 每一个动作之前读这个 tab 此刻的地址（一帧 `browsingContext.getTree`），它是城自己的源，拒；
- `open` 之后再读一次：一次跳转把 tab 带到了城自己的源上，这一次也拒，下一次动作照样被前一条拒住。

拒词说的是动作与地址，恢复语是「城自己的页面是 User 的，打开工作所服务的那个地址」。
-/

/-! D74 居民的两种浏览器都不碰城自己的源；守卫读 tab 的地址，不装 BiDi 拦截

决定：§8-45-6 的三条判断落在 `bin::browser_tool`，对 `browser` 与 `usersbrowser` 一样，判定本身（一个地址是不是城的某个监听）在 `browser::OwnListeners`。理由：`usersbrowser` 驱动的是人自己的 profile，城的页面在那个 profile 里存着人的设备钥，一个居民把它带到城的源上，就能以人的身份操作这座城；`browser` 的 profile 是楼自己的，没有设备钥，可它照样能打开配对页。两件工具一条规则，比只守一件少一处要记住的差别。出网门不改：`kernel::gate` 的出网判定只按主机，回环一律放行（居民要看自己起的开发服务器），它不知道城监听在哪个端口，而登记只有装配层知道。被否：①让出网门长出「本城的监听」这一判定——要把端口带进 kernel 的门，而门的输入今天只有主机；②拦所有回环——居民的开发循环就是在回环上看自己造的东西。

现状：页面自己发起的子资源请求与脚本里的 `fetch` 不经这条守卫。sec.md E.6 要的 BiDi `network.addIntercept` 需要 `BidiSocket` 在等答复的循环里答 `network.beforeRequestSent` 事件（`network.failRequest`），并用一段不与 `Session` 相撞的帧 id；今天的端口跳过一切事件，装上拦截而不答，被拦的请求就挂到下一次调用为止。挡住这类请求的是本机端口的入口判定：Origin 不在名单里的请求在升级之前就被拒（`crates/wire/Spec.lean` 的入口判定）。重开参数：BiDi 端口开始处理事件时，加上这一道拦截。
-/

