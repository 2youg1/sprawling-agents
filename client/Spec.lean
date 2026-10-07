-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Core.Workbench
import client.spec.Views.Door
import client.spec.Views.Fold
import client.spec.Views.Guide
import client.spec.Views.Inspect.Open
import client.spec.Views.Parts
import client.spec.Views.Parts.Combobox
import client.spec.Views.Parts.Decide
import client.spec.Views.Parts.Popover
import client.spec.Views.Parts.Row
import client.spec.Views.Parts.Segmented
import client.spec.Views.Parts.Tabs
import client.spec.Views.Workspace

/-! # client 的规格

浏览器客户端（`client/`，Svelte 5 加 Effect，在 cargo workspace 之外，由 bun 驱动）的规格入口；分部在 `client/spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」，分部按 `client/src/` 下的模块路径命名（`views/parts/segmented.ts` 由 `client/spec/Views/Parts/Segmented.lean` 规定）。

权威顺序同 AGENTS.md：人的决定 → ARCHITECTURE.md → 本文件 → 代码与测试。本规格记接口与设计；视图层（`src/**/*.svelte`、`src/theme.css` 与 `src/theme/*.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。

**免的是画法，不是键盘。** 一个部件遵循哪个 WAI-ARIA 模式、每个键做什么、焦点还给谁、`aria-*` 取什么值，是这一层对使用者的承诺，不是一次视觉迭代；`docs/frontend-method.md` 的豁免因此只覆盖排布、间距、色调与动效，`views/parts/` 的交互契约由 §9 独家规定。

**形式化的是交互契约。** 每个收键部件的键表、焦点落点与 `aria-*` 写成状态机，在分部里证明（§9）：方向键的两种走法、焦点还给打开者、分段控件的落点与 Tab 站、页签、行列表、组合框与弹层的游标、模态的取焦、请决定卡的三个键、图层键与硬币键、检视面的页签带、工作台的分隔线。其余各节是注释：效应核（`client/src/core/`）的接口与它们的设计，由各模块旁的 `bun test` 守住（§16）。

**标签。** 带连字符的标签——§3-n、§4-n、§7-n、§7C、§7G、§7K、§7L、§7N——是条目的名字，沿用它们一直以来的写法，与本文件十七节的节号无关，别处引作 `client/Spec.lean §4-26`。决定写作 `D<n>`，别处引作 `client D<n>`；D34 是空号。§1 末尾列出每个标签住在哪个文件。说屏幕长什么样的条目——§3-3、§4-33、§4-34、§4-37、§4-43、§7A 至 §7B、§7D 至 §7F、§7H 至 §7J——不在本规格，在 `docs/frontend-method.md` 的「The design, entry by entry」一节，用英文写，标签不变，别处引作 `docs/frontend-method.md` §7D；它们说的键与焦点仍由本规格的 §9 规定。
-/

/-! ## 1 需求分解

- `client/` 在 cargo workspace **之外**，由 bun 驱动；产物落 `sprawling` 包里的 `crates/sprawling/web-dist/`（crates.io 的包只装包目录，`crates/sprawling/Spec.lean` §8-83），不随 `CARGO_TARGET_DIR` 移动（`index.html` 在该目录根，其余在 `assets/`），`crates/sprawling/build.rs` 递归嵌入该目录，并以 `index.html` 与 `assets/` 的存在判「完整」。
- **两种范式不叠**：Effect 给的是值，不是运行时——生成的 `Schema` 读帧与读城外来的 JSON，`Option` 与 `Result` 是可缺与可败的值（4-6）。socket 阶梯、asking、belief 都是纯 TS 状态机加 `svelte/store`，视图只见 Svelte。
- 运行时依赖的名单只有一个家：`tools/xtask/src/npm.rs` 的 `RUNTIME`，本文件不抄它的条目与数目。名单上除了 `svelte` 与 `effect`，还有 `@lezer/highlight` 与各语言的 `@lezer` 语法，因为代码视图按语法上色，而高亮器与每种语法都是按需加载的分块（4-26），不进首屏；以及图标集 `@lucide/svelte`，只经 `parts/glyph.svelte` 一处出口、按图标单独导入（`docs/frontend-method.md` §4-34）；以及 RefRain 的五个 `@codemirror` 包，只由 `views/refrain/` 读，是懒加载的一块（7N、D23）；以及 `pdfjs-dist` 与 `docx-preview`，只由 `views/refrain/formats/` 读，各是一块懒加载的分块，pdf.js 的 worker、CMap、两种符号字体与三个 WebAssembly 解码器是 bundle 里单独的文件（4-54、D32）。hash 路由手写，不引路由库；组件库按 §9 的判定逐个引入。`xtask npm` 门守三件事：锁文件与清单逐条同、运行时依赖恰为 `RUNTIME`、许可证在 `deny.toml` 的清单上。
- Firefox 是第一浏览器：每个屏幕先在 Firefox 里验收。
- `trustedDependencies` 留空：bun 默认不跑生命周期脚本，任何包的 postinstall 都不执行。

| 分部 | 规定什么 | 标签 |
|---|---|---|
| `client/spec/Views/Parts.lean` | `views/parts/` 的交互契约：方向键的两种走法、焦点还原、模态 | §7、§7-1 至 §7-7、§7-10 |
| `client/spec/Views/Parts/Segmented.lean` | 分段控件的落点与 Tab 站 | （§7-4） |
| `client/spec/Views/Parts/Tabs.lean` | 自动激活的页签 | （§7-4） |
| `client/spec/Views/Parts/Row.lean` | `RowList` 的走法 | （§7-4） |
| `client/spec/Views/Parts/Combobox.lean` | 组合框的游标、过滤与关闭 | （§7-5） |
| `client/spec/Views/Parts/Popover.lean` | 多列弹层的换列与游标 | （§7-5） |
| `client/spec/Views/Parts/Decide.lean` | 请决定卡的三个答复键 | （§7-11、§7C） |
| `client/spec/Views/Workspace.lean` | 外壳的控件：图层键、硬币键、信箱的走法 | §7-11 |
| `client/spec/Views/Inspect/Open.lean` | 检视面的页签带与它至多留几项 | （§7-11、§4-45） |
| `client/spec/Core/Workbench.lean` | 工作台分隔线的宽度 | （§7-11、D24） |
| `client/spec/Views/Fold.lean` | 设置树的枝与上手指南的步骤一次只展开一项 | （§7L、§7G、D53、D55） |
| `client/spec/Views/Guide.lean` | 启动时进不进上手指南，跳过之后落在哪 | （§7G、D54） |
| `client/spec/Views/Door.lean` | 远程组的门开关、「更换城钥匙」与确认码输入框：焦点、Escape 与拒绝 | （4-57） |

其余标签都在本文件：§3-1、§3-2 在 §8，§3-4 在 §3，其余 §4-n 与 §7C、§7G、§7K、§7L、§7N 在 §10，§7-8 在 §4，§7-9 在 §13；决定 D1 至 D48（含 D42a）、D52 至 D55、D72、D78、D80、D81、D82、D83、D85、D86、D88、D89、D90、D91、D92、D93 与 D94 在 §10 之后，D60 与 D73 在 `client/spec/Views/Workspace.lean`。
-/

/-! ## 2 验收标准

`bun run lint`、`bun run typecheck`、`bun run test` 三样绿，`cargo xtask npm`、`cargo xtask wire-ts`、`cargo xtask color`、`cargo xtask wording`、`cargo xtask render` 绿；`just check-client` 是这三条脚本的一条线。

在一座真城加一个说 OpenAI 形的假供应方上走得通的：连接与握手、上手指南的五步与「开始对话」（§7G）、从 composer 派活、工具调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与文件原文、run 页的统计栏与透镜、记录的三种读法（时间线与它的三个来源、归档、回收站；`#/record/log` 打开只留日志的时间线）、成本页、设置页 attach 与 select、`#/mcp` 三扇门加删（写进 `CONFIG.toml`）、楼页提交列表与 session 跳转、草稿留存。

**未验的**：`Changes`／`Hunks` 有内容时的样子、Firefox 与 Zen 的无头截图（`-screenshot` 不出图，须走 BiDi）、`Tip` 两条定位分支各自的 `#/gallery` 夹具（`xtask render` 只读 `#/gallery`，所以这两条分支在真引擎里的落点尚无机器读者）、**§9 的键表**（`xtask render` 今天只量盒子，没有一次按键进过真引擎，所以每一行键表今天的读者只有人）。

分部里的定理是模型对交互契约的证明：

- `spec/Views/Parts.lean`：环绕一步可逆、走满一圈回到原处、每一格都走得到（`wrap_back_undoes_forward`、`a_full_turn_comes_home`、`every_cell_is_reached`）；钳住不出界、两端是不动点（`clamp_stays`、`clamp_holds_the_last`、`clamp_holds_the_first`）；关上一层把焦点还给打开者，层层关上回到最初（`closing_returns_to_the_opener`、`closing_every_layer_restores_the_page`）；模态外面的事件——Escape 与点背景——从不确认（`escape_never_confirms`）。
- `spec/Views/Parts/Segmented.lean`：方向键从不落在不能选的格上（`an_arrow_never_lands_on_a_refused_cell`），落点在控件之内（`an_arrow_stays_inside`），Tab 站是一格或没有（`the_stop_is_a_cell`、`an_empty_control_offers_no_stop`、`a_refused_control_offers_its_first_cell`）。
- `spec/Views/Parts/Tabs.lean`、`Row.lean`、`Combobox.lean`、`Popover.lean`：每个键留在部件之内；行列表两端不环绕、文本框留住自己的键；游标走动不改生效的值；打字复位游标；Escape 关上并清空过滤词而不改值；换列复位游标。
- `spec/Views/Parts/Decide.lean`：门只收「知道了」、过期的提案只能拒绝、不带 accel 的一下按键不是答复。
- `spec/Views/Workspace.lean`：图层键三下回原档、看一眼不改选定的档；变淡的发送面什么都不做、停止面不发字；信箱的数字只落到画出来的条目。
- `spec/Views/Inspect/Open.lean`：至多 `KEPT` 项、刚打开的那一项在、Delete 之后焦点落在一个页签上。
- `spec/Core/Workbench.lean`：分隔线不改一对栏的总宽、两栏都不窄于两栏。
- `spec/Views/Door.lean`：任意一段按键与回答之后，Escape 把焦点还给按下的那个控件；码只从等码的输入框发出、且不空；在途时的拒绝清空输入框；没离开页面的命令回到按下的控件、不编一个拒绝；焦点在框里、框里有字，只在码被要着的时候。

这些是模型的证明，不是 TypeScript 的证明：实现与模型的对应由各模块旁的 `bun test`（`segmented.test.ts`、`workbench.test.ts` 等）与 `#/gallery` 的夹具承担（§16）。
-/

/-! ## 3 假设与歧义

### §3-4 未决

- **Solid 一臂的两个读数**（D12）。树上只有 Svelte 一臂的读数。能定下 D12 的证据是两个在同一台机器上交错测的读数，并写明机器类属：一是构建产物大小，按 `frontend_artifact` 的称法（`just build-web` 之后 gzip 称整个 dist），两臂各称一次；二是每 token 开销，用 `belief/fold_cost.test.ts` 的仪表（一帧 50 个 delta、R = 1e4、一个读全表的订阅者），Solid 一臂把 `belief/runs.svelte.ts` 换成 Solid 的 store。D2 的 30–40 µs 没有记下机器类属，所以交错测时 Svelte 一臂也要重测。

- **重连续传的阈值**（4-40）。`RESUME_PAGES = 2` 是估计。能定下它的读数是同一座城上一次快照重问（`asking.reconnected()` 发出的全部问题的回答字节）与一页 `HistoryRange` 的字节之比；缺口超过阈值时回退到快照的那条路今天也没有测试。
-/

/-! ## 4 现状分析

规格已定、实现未到的地方逐条点名在下面；每一条的规格在 §9 的分部里。

### §7-8 今天与模式不符的十一处

每一条都是「规格已定、实现未到」，不是待议的设计问题。

1. `combobox.svelte:92` — 过滤框没有 `role="combobox"`、`aria-controls`、`aria-activedescendant`，游标只有底色；读屏用户按方向键时听不到任何变化。
2. `combobox.svelte:106`–`127` — 缺 Home／End；Tab 不关闭弹层；全文件没有点外面关闭或失焦关闭的路，一个开着的弹层可以留在页面上。
3. `combobox.svelte:138` 与 `popover.svelte:176` — 同一个 `aria-selected` 两种读法：前者标已选中的值（对），后者标游标（错），而后者真正生效的那一项只由 `popover.svelte:190` 的圆点承担。
4. `popover.svelte:163` — `bind` 模式下 `aria-activedescendant` 写在不持焦点的 `<ul>` 上，因此 composer 里按方向键时读屏什么都不报。
5. `segmented.svelte:182`–`195` — 缺 ↓／↑ 与 Space，APG Radio Group 的键表只实现了一半。
6. `tabs.svelte:58` — `id="tab-<id>"` 今天没有读者：三个使用者（`run`／`record`／`mcp`）都没有 `role="tabpanel"` ＋ `aria-labelledby`，`<button role="tab">` 也没有 `aria-controls`，所以「这块面板属于哪个页签」在页面上说不出来。
7. `table.svelte:108` — 不可排序的列也写 `aria-sort="none"`，那是一句关于一个排不了序的列的排序陈述。
8. `table.svelte:98` — 表头勾选框只有选中与未选中两态，部分选中时说「一个都没选」。
9. `tip.svelte` — 全文件没有 Escape 撤下提示，而那是 APG Tooltip 的唯一一个键，也是 WCAG 1.4.13「可撤下」要的那一件。
10. `field.svelte:94` 与 `button.svelte:87` — 「一个人不能用的控件」两套写法：前者用原生 `disabled`（离开 Tab 序列，原因读不出来），后者用 `aria-disabled` 加一次点击判定。规格取后者。
11. `client/src/views/parts/row.svelte` 的 `RowList` — `<ul>` 直接收调用方的 `<Row>`，而 `Row` 画的是 `<div>`：一个子元素不是 `<li>` 的列表，读屏报得出「一个列表」却报不出「几项」。该文件本波正在改写，所以这一条只记事实、不钉行号。
-/

/-! ## 5 权威信源

权威顺序同 `AGENTS.md`：人的决定 → `ARCHITECTURE.md` → 本规格 → 代码与测试。键表的模式名取自 WAI-ARIA Authoring Practices（APG），借鉴自哪几份文档、为什么不产生许可证义务记在 `docs/third-party.md` §6。线上的形状由生成的 `client/src/wire.ts` 说，它的权威是 `crates/wire/Spec.lean`；页面不另写一份。运行时依赖的名单只有一个家，`tools/xtask/src/npm.rs` 的 `RUNTIME`。
-/

/-! ## 6 命名统一

概念名取自 `docs/glossary.md`，由 `xtask lexicon` 守住；对人的每一个字取自 `client/src/lang.json`（§3-1），由 `xtask wording` 守住。Lean 声明名是英文：`Tier` 的三个值就是 client D17 定下的 `zen`、`blend`、`panorama`，`Reply` 的三个值就是请决定卡的同意、改后同意与不同意。
-/

/-! ## 7 模块边界

### 效应核 `src/core/`（形状按 ARCHITECTURE §9）

| 文件 | 形状 | 接口 |
|---|---|---|
| `link.ts` | 1 判定 | `newLink(token, lang)`, `connect(link) -> [Link, LinkAction]`, `advance(link, LinkEvent) -> [Link, LinkAction]`；`LinkAction` 穷尽（open／send／welcomed／deliver／answered／saying／wait／report／close）；`isLive(link)`、`isRefused(link)`；`backoffMs(attempt)` 读阶梯 `[250,500,1000,2000,5000,10000]`；`unreadableRecord(lang, at)` 与 `unsentCommand(lang, verb)` 是链路自己写出的两种 `AxError` |
| `unsent.ts` | 2 值 | `isSpeech(command)`、`createUnsent() -> { count: Readable<number>, hold(command), release(send) }`：断线时人说的话按序排队，`welcomed` 时 `release` 逐条发出，某条发不出就停在那条、其余留着；`count` 是断线横幅写出的条数。**排着的话只在内存里**：断线期间关掉或重新载入这一页，它们就没了，而输入框在交给 `unsent` 时已经清空了草稿；要让它们活过重新载入，得经 `prefs.ts` 的 `draft` 门存下，今天没有这样做 |
| `socket.ts` | 4 适配器 | `openConnection(dial, token, lang) -> Connection { state, belief, asking, unsent, command, retry, dismissRefusal }`（`state`／`belief`／`unsent` 是 `Readable`，其余是函数）；链路在 `opening`／`handshaking`／`backoff` 时 `command` 把人说的话（`dispatch`／`steer`）交给 `unsent` 并答 `true`，其余命令仍答 `false`——停下、释放、授权这类动作等到重连后才生效，可能已不是人按下时想要的；链路在 `refused` 时一律答 `false`，因为阶梯不会自己走到 `welcome`，而 `E_WIRE_MISMATCH` 唯一的动作是重新载入，它会丢掉只在内存里的 `unsent`——答 `false` 让输入框留住草稿；`retry` 先取消阶梯排好的尝试再重连；`tokenIn(search)`, `bearing(token)`——POST 递配对码的唯一拼写（`Authorization: Bearer`，服务端读者是 `wire::reception::offered_pairing`）；`dial` 是链路说话经过的那条线（`line.ts`） |
| `line.ts` | 4 适配器 | `Line { send(text) -> boolean, close() }`、`Hearing { opened, heard(text), closed }`、`Dial = (hearing) -> Line`：链路说话经过的那一条线，`send` 在线没开时答 `false`，`closed` 每条线只报一次；`plainDial(url)` 是浏览器的 `WebSocket`，另一种是 `remote/session.ts` 的会话（4-64）；`socketUrl(location)` 是本源 `/ws` 的地址 |
| `gap_walk.ts` | 4 适配器 | `createGapWalk(ask, store, asking, lang) -> GapWalk { fold(record), lagged(from, to), welcomed(epoch, head), answered(askId, outcome) -> boolean }`：缺口补拉与水位（设计 4-40）。`socket.ts` 折的每条记录都经 `fold`，所以水位总是重连续传的起点；`answered` 只收它自己那一问（按 id 认），答 `false` 的交给 `asking`；重连落在阈值内时，尚待补拉的 `lagged` 缺口改为逐条失效，因为看过它的答案随旧 socket 一起作废 |
| `answered.ts` | 1 判定 | `readAnswer(answer, pick) -> Answered<T>`，`Answered = asking \| held { value } \| unavailable { query }`：一个视图只画一种变体，槽里的其余答案仍是答案——`Answer::Unavailable` 带回城拼写的那一问，别的变体以自己的键名为 `query`；视图用 `views/parts/unanswered.svelte` 画它，恢复是 `asking.refresh` 再问一次。不折成「还在问」或「空」，因为那两种读法把「城没能看」说成「城在忙」或「城是空的」 |
| `frames.ts` | 4 适配器 | `decodeFrame(text) -> ServerFrame \| null`（event／delta 帧走窄校验快路，其余经 `Schema.decodeUnknownResult`）, `encodeFrame(ClientFrame)` |
| `staleness.ts` | 1 判定 | `reachOf(name, kind) -> Reach`（`every`／`none`／`same_run`／`newest_page`）与 `reaches(reach, key, run)`：事件到查询的失效表，按问题名判，不按每条答案判。**一次写的回执就是它让哪个答案失效**：`governed_document_written` 让 `identity` 失效（「你与主 Agent」两张卡的保存回执是城重答的新版本，4-36），`rules_changed` 让 `config` 与楼里的 `document` 失效（`RULES.toml` 与城层 `CONFIG.toml` 的保存经 `rules_changed` 入账），`building_configured` 让 `config` 失效；`automation` 不失效，因为 `SCHEDULE.toml`／`WATCH.toml` 由人手改、城不为它们写记录；`sessions` 被 `session_opened`、`run_started`、`run_frozen` 置旧，作用面是 `every`，因为一段的最后一行与 run 数在这三个时刻移动，信箱与会话栏才看得到新开的一段 |
| `asking.ts` | 1 判定 | `createAsking(send) -> Asking { ask(query) -> Readable<Answer\|undefined>, refresh, answered, invalidate(record), reconnected, resumed }`（`reconnected` 标旧重问全部，`resumed` 只重发在途的；`Readable` 皆为 `svelte/store` 面，模板以 `$` 订阅）；`QUERIES` 是每个问题名的唯一拼写，`COMMITS_PAGE` 与 `commitsQuery(building, before)` 见 4-15，`HELD_CAP` 是页面保留的答案数上界（超过时丢掉最久没用、也没人看着的那一个，所以开了一周的标签页与第一天持有一样多），`keyOf` 是问题的合并键，`askedIn(subject)` 从本页自己写出的拒绝（如 `E_TIMEOUT`）的 subject 读回那个问题的线上名字；答案按内容匹配问题，无名者按到达序；失效按 `staleness.ts` 的表 |
| `belief.ts` | 7 投影 | `createBelief(now) -> BeliefStore { belief, adoptCity, apply(record) -> string \| null, say(delta), logged(line), refused(error), named(city), noticesSeen, forget, batch(folds) }`——`now` 是取时刻的那一个入口；`batch` 里的折叠只在最外层结束时 `set` 一次，`socket.ts` 的 `drain`（连同其中 `filled` 折的缺口页）与 welcome 各是一批，所以两帧之间的一串记录是一次更新、一次重绘；`forget` 在账本换了（welcome 的 `epoch` 变了）时丢弃全部折叠。**`apply` 答的是它读不出的字段名**（如 `tool_called.name`），`null` 才是读全了：一份形状不对的载荷仍然推进位置，但不静默当作缺席 |
| `belief/shape.ts` | 6 数据 | `Belief { runs, live, rooms, cancelled, halted, haltedAt, refusal, notices, city, sessions, policies, probed, logs }`（`policies` 是每个房间最新的一次 `run_policy_changed`，按 seq 只向前写：会话菜单从它起改，对话在它落下的位置画一行「从下一步起：<模式>，<写入限制>」，它不在当前一段里时两处都读 run 开场的 `Opening.policy`）；`RunBelief { run, addr, started, task, lastSeq, doing, model, pr, ask, local, saying, thinking }`；`Notice { error, seen, at: TimeMs, key, count, about: Address \| RunId \| null }`；`merged(notices, error, at)` 按 `key`（`code + subject`）合并同文并计 `count`，`at` 取首见时刻；`LOG_WINDOW` 与 `NOTICE_WINDOW` 是页面持有多少日志行与通知的唯一答案 |
| `belief/fold.ts` | 7 投影 | `unseen(run, at) -> RunBelief`（页面第一次见到的 run）、`fold(held, record) -> [RunBelief, string \| null]`：一条记录对一个 run 做了什么；记录属于哪个 run、是不是新的，由 `belief.ts` 判 |
| `belief/adopted.ts` | 7 投影 | `adopted(summary, held) -> RunBelief`：一行 `city_view` 读成 belief，流已经知道而行没带的字段留着；`adoptCity` 与 run 页（经链接到达、从没流过的 run 只有这一行）共读 |
| `belief/runs.svelte.ts` | 7 投影 | `runTable(held) -> Record<RunId, RunBelief>`：run 表是 `$state`，每个 run 是一个响应式对象。`say(delta)` 对已持有的 run 就地追加 `saying`／`thinking`，不重发 `belief`，只唤醒读这个 run 这个字段的读者；只有 delta 带来新 run（表的形状变了）才发布一次。记录的折叠与 `adoptCity` 仍整值写入并发布。测试经 `client/bunfig.toml` 预载的 `scripts/runes.ts` 用 `compileModule` 编 `.svelte.ts`（含 `*.svelte.test.ts`），与 vite 进产物同一编译器 |
| `belief/live.ts` | 7 投影 | `Belief.live: readonly RunBelief[]`——没冻结的 run，按 `started` 从旧到新，是「哪些 run 在干活」的唯一权威；`livened(live, run) -> readonly RunBelief[]` 在每次折叠里按这一个 run 改写它，O(L)（L 为在干活的 run 数），`liveOf(runs)` 只在 `adoptCity` 整表换写时 O(R) 重建；`within(run, room) -> boolean` 与它读的 `inside(addr, room) -> boolean` 是「在这个房间或其下」的唯一拼写；`newestWorking(belief, room) -> RunBelief | undefined` 从 `live` 取这个房间最新的在干活的 run，是对话页的 steer 与停止键、`in_front.ts` 在对话页的答案与房间页「正在进行」共读的唯一答案，O(L)；`onceFrozen(belief, run, then)`：belief 里这个 run 冻结时调一次 `then`，然后不再听（`/compact`，D46）。视图读 `$belief.live` 而不再各自 `Object.values(runs).filter(…)`：R = 1e4 时一次记录折叠加一次读「在干活的 run」≤ 20 µs（`belief/live.test.ts`） |
| `belief/rooms.ts` | 7 投影 | `Belief.rooms: ReadonlyMap<string, readonly RunId[]>`——每个房间持有过的 run（在干活的与已冻结的），按 `started` 从旧到新，只存 `RunId`，所以一次折叠只在 run 进表、换房间或换开始时刻时改写那一个房间的列表，O(k)（k 为该房间的 run 数），其余折叠不动索引；`adoptCity` 整表换写时 O(R log R) 重建。`heldIn(belief, room) -> RunBelief[]`（恰在这个房间）与 `heldWithin(belief, room) -> RunBelief[]`（这个房间及其下，按 `inside`）是房间页、目录、城市面板与天际线共读的答案，读者付 O(房间数 + k)：R = 1e4、百个房间时一次记录折叠加一次读一个房间 ≤ 500 µs（`belief/live.test.ts`）。同一开始时刻的两个 run 按进表先后排 |
| `belief/cancelled.ts` | 7 投影 | `Belief.cancelled: number`——冻结原因是 `cancelled` 的 run 数（线协议不带一次停机冻结了多少，停机写的正是这个原因）。`recounted(count, was, run) -> number` 在每次折叠里只看这一个 run 的前后两次读数，O(1)；`cancelledOf(runs)` 只在 `adoptCity` 整表换写时 O(R) 重数。页面的停机行读它：R = 1e4 时一次记录折叠加一次读 ≤ 500 µs（`belief/live.test.ts`） |

| `doing.ts` | 2 值 | `Doing = unknown \| thinking \| calling { tool: string \| null, subject } \| waiting \| awaiting_reply { wait: Waiting } \| frozen { completion }`、`Sending = dispatch \| steer \| queued`、`sendingInto(doing)`、`afterWait(doing, end) -> Doing`（一次回信等待的结束把姿态送到哪里，D88）；`MOVING`／`moves(kind)`／`PHASES` 是「哪些 kind 单凭 kind 就陈述姿态、各自陈述什么」的独家表，流与答案两条路都读它；`signal_wait_started`／`signal_wait_ended` 要读载荷才说得出等谁，所以不在表里，流的一路由 `belief/fold.ts` 折，答案的一路读 `RunSummary.waiting` |
| `landing.ts` | 1 判定 | `sentFrom(from, task, runs) -> Sent`（发出那一刻记下房间、任务原文与已知的 run）、`landingOf(sent, runs) -> Landing`，`Landing` 穷尽（`pending`／`here`／`elsewhere { run, addr }`）：`run_started` 之后第一个「发出时不认识、`task` 与原文相同、`addr` 是发出的房间或它下面任一层的房间」的 run 就是这次派的活；落在原房间时线程已经画出它，落在别处时 `talk/landed.svelte` 在原地留一行可点的去处（见 D6） |
| `reading.ts` | 4 适配器 | `taskOf(record)`、`toolCall(record)`、`modelOf(record)`、`completionOf(record)`、`askOf(record)`、`branchOf(record)`、`haltOf(record)`，各答 `[值, 读不出的字段名 \| null]`；`sessionStart(record) -> SessionStart \| null`（不是会话开头、或没说房间的记录答 `null`）；`policyChange(record) -> PolicyChange \| null`（`run_policy_changed` 的房间、seq、时刻与按生成的 `RunPolicy` 读出的策略，读不出答 `null`）；`offeredOn(record) -> [Address \| null, string \| null]`（`proposal_offered` 的 `doc`，按生成的 `Address` 文法读）；`kernel::event::record` 的字段名与 serde 属性（`Option` 与 `#[serde(default)]` 各是什么意思）在客户端只有这一处拼写 |
| `scope.ts` | 4 适配器 | `scopeOf(spelled) -> HaltScope \| null`（Ledger 拼法→frame 拼法，唯一相遇点）、`sameScope`、`buildingIsShut`、`cityIsShut`、`CITY`；`CITY` 是两套拼法共同的那一个词，五个视图改读它，不再手写 `"city"` |
| `run_id.ts` | 4 适配器 | `readRunId(raw) -> Option<RunId>`：`Schema.decodeOption` 于生成的 `RunId`，地址栏与转写文件名的唯一文法 |
| `commands.ts` | 2 值 | 每个命令帧一个构造函数，自铸 `IdemKey`；`selectModel(endpoint, model, tag, stated)` 的 `Stated { contextTokens, maxOutputTokens, input }` 是人对一个模型在名字之外说的三件事：两个上限与它收什么输入（`InputKinds`，`text`／`text_image`，即 `SelectModel.input`，`gateway::accepted_input` 的第一档）；`input` 为 `null` 是没说，由城往下一档问 |
| `enrol.ts` | 4 适配器 | `enrol(Enrolling { origin, token, realm, name, value, lang }) -> Promise<Enrolment>`；`referenceFor(provider)` 是 realm／name 唯一的选词处（realm 是本页选的词，城只判字母表），`referenceText(at)` 是页面预演用的唯一拼法，`keyField`／`secretFor` 保证存的引用只用在它被归档的那个 id 上。**引用来自城**：201 正文是 `kernel::SecretRef` 读回后写出的那一句，本页不自己拼一份存起来 |
| `speaking.ts` | 4 适配器 | `canRecord()`, `record(origin, token) -> Promise<Recording \| null>`；`Recording.stop() -> Promise<Heard>`，`Heard` 穷尽（text／refused／silent）；`dictation(origin, pairing, into) -> Dictation`：composer 的麦克风，听到的一句交给 `into`，落进输入框而不直接发出（4-16） |
| `idem.ts` | 2 值 | `mintIdem()` |
| `mark.ts` | 4 适配器 | `markOf({ waiting, working, link })` 判定四态：链路不在 `live` 时为 `untold`（页面此刻没被告知，城里的事可能已经变了，图标不替没人说过的话作证），否则有待人决定的事为 `waiting`、有 run 在动为 `live`、其余为 `quiet`；`paintMark(document, mark)` 把令牌解算成引擎实际会画的颜色，由 `markSvg` 拼成 SVG data URL 写进 `<link rel="icon">`。四态各是一个形状——`quiet` 空心环、`live` 实心圆、`waiting` 菱形、`untold` 一道横杠——因为标签栏很小，分不清ACCENT与警示色的人仍分得清环与菱形；零颜色字面量 |
| `rows.ts` | 4 适配器 | `Rows { getItem, setItem, removeItem, unkept: Readable<ReadonlySet<string>> }`、`memory()`、`keptRows(reach: () => Rows)`、`browserRows()`：浏览器存储那一扇门，三处会抛的拒绝（禁用存储、配额为零、写时配额满）在这里各变成一个值。**被拒的写不静默**：一行浏览器不肯收时这一页替它记着，直到标签页关掉，`unkept` 是此刻只有这一页记着的行名——写时配额满的那一行，或浏览器根本不给存储时写下的每一行；同一行后来写成了、或被删了，就从里面出去。`memory()` 是一张普通的表（测试与画廊拿它当浏览器的存储），它没拒绝过什么，所以它的 `unkept` 恒空；`keptRows` 把一个可能够不着的存储包成这扇门，`browserRows()` 是它对 `localStorage` 的那一个（§3-14 第 2 行，4-63） |
| `prefs.ts` | 6 数据 | `Preferences { lang, welcomed, panel, tier, appearance, proxying, notifying, showing }`、`Tier`（线上的 `wire::Tier`）、`TIERS`（线上的顺序，也是图层键循环的顺序；每次启动是 `zen`，存下的档不在启动时读回，D47）、`PROXYING_RULES`、`NOTIFYINGS`、`Keeper = "browser" \| "city"`、`PreferenceDoor { held, keeper, adopt, tell, setLang, setWelcomed, setPanel, setTier, setAppearance, setProxying, setNotifying, setShowing, chord(action), setChord, draft(at), setDraft, draftUnkept(at) -> Readable<boolean>, editor() -> { editor, folder }, setEditor, workbench, setWorkbench }`、`loadPreferences(rows, browserLang)`、`preferences()`；**全客户端每一个存储键的拼写都只在这个文件的 `ROWS` 里**（草稿键 `sprawling.draft.<房间或 run>`、快捷键 `sprawling.key.<action>`、工作台 `sprawling.workbench`）。`workbench` 是 `Readable<Workbench>`，`setWorkbench` 只写这个浏览器、不发 `PutPreferences`：栏序与栏宽是这块屏幕的事实，不是这个人的（D24）；`setAppearance` 发一件补丁，玻璃与混合档透明度因此随偏好进城（4-60）；`setTier` 只改这个标签页里的档，不写存储、不出城，城的回答也不改它（D47） |
| `workbench.ts` | 1 判定 | `Pane = "sessions" \| "session" \| "commits"`、`Column { pane, span }`、`Workbench`（三栏，从左到右）、`WORKBENCH`（默认从左到右是会话栏、所选会话栏、地方栏，宽 3、6、3 栏，即外壳的白银切分）、`NARROWEST = 2`、`Divider = 0 \| 1`（第一栏与第二栏之间、第二栏与第三栏之间）、`resized(bench, divider, span)`：分隔线前一栏取 `span` 栏宽，钳在 `NARROWEST` 与 `widest(bench, divider)`（这一对栏的总宽减 `NARROWEST`）之间，后一栏取余数，所以三栏之和恒为 12、每栏恒不窄于两栏；`moved(bench, pane, side)`：与左邻或右邻换位，宽度随栏走，在边上的栏不动；`readWorkbench(raw)` 与 `spelledWorkbench(bench)` 互为逆（`sessions:3 session:6 commits:3`），读不懂的一行——缺栏、重栏、宽度不是整数、窄于 `NARROWEST`、总和不是 12——读作 `WORKBENCH`，不修补；浏览器里已存的 `3 5 4` 仍是合法的一行，照读不改。检阅档工作台的栏序与栏宽怎样才算一个值的唯一判定（7K、D24） |
| `appearance.ts` | 2 值 | `Appearance { lighting, sans, mono, sansStack, monoStack, body, density, chroma, motion, glass, blend }` 与各项的词表（`LIGHTINGS`、`FACES`、`DENSITIES`、`CHROMAS`、`MOTIONS`、`GLASSES`，按选择器画出的顺序）、`STACK_SHAPE`、`BLEND_PERCENT`（混合档滑条的下限、上限与步长，`docs/frontend-method.md` §4-43）与 `blendOf(raw)`（一行存储读成一个在域内的百分数，读不出或越界答 `null`）：一条外观记录怎样才算合法；`prefs.ts` 负责存取，`prefs_city.ts` 负责带到城。`Glass` 是线上的 `wire::Glass`；`BLEND_PERCENT` 是透明度之域的唯一声明，城照存不判（wire D14） |
| `theme_override.ts` | 1 判定 | `Theme { tokens, css }`、`BUILT_IN_THEME`、`themeOf(stated: ThemeOverride) -> Theme`、`readTheme(raw) -> Theme`：人盖在内置主题上的颜色（按 `@theme` 变量的 CSS 颜色，加一段自己的样式表，`crates/wire/spec/Preference.lean` D29），以及本浏览器缓存它的那一行的唯一读法；字段取完整形状而不是线上的可缺字段，读的人不必问缺席是不是空 |
| `sizing.ts` | 1 判定 | `BODY_PX`、`sizingOf(text) -> Sizing`（`cleared` \| `sized { px }` \| `refused`）：人写进字号框的一串字读成什么；偏好的读取与外观组共用这一处。 |
| `editor.ts` | 1 判定 | `Editor = "none" \| "vscode" \| "vscode-insiders" \| "vscodium" \| "cursor" \| "windsurf" \| "zed"`、`EDITORS`、`editorLink(Opening { editor, folder, path, line }) -> string \| null`：「在我的编辑器里打开 文件:行」的唯一拼法，各编辑器的链接前缀由同文件的 `fileUrl` 给出（4-39），监视器的改动块拿它当 `href`，由浏览器把链接交给浏览器所在机器上注册了该协议的编辑器，服务端不启动任何程序。`path` 用生成的 `Address` 判（城内相对路径，无 `..`、无盘符、无反斜杠），`folder` 须是绝对路径且无 `.`／`..` 段，`line` 须是正整数；任何一条不成立答 `null`，页面不画这个链接。编辑器与城在浏览器所在机器上的文件夹由 `prefs.ts` 的 `editor()` 与 `setEditor` 保管。`reachOf(Opening, Shown) -> Reach` 是检视面问的那一句：`Shown = "current" \| "past"` 说行号读自工作树此刻的文本，还是读自工作树已不再持有的一个版本；`Reach` 是 `{ kind: "link", href }` 或 `{ kind: "copy", text }`。`past` 一律答 `copy`，`current` 在 `editorLink` 答得出时答 `link`，答不出（没选编辑器、没填文件夹、路径不在城里）时也答 `copy`；`text` 是 `路径:行`，所以人总有一个能拿走的定位，而页面从不声称已在编辑器里打开（4-39） |
| `document_pos.ts` | 1 判定 | `EditorChange { from, to, insert }`、`Place { line, column }`、`Positions { version, encoding, editor, lineBreak, bytes(at), editorAt(byte), place(at) }`、`positionsOf(version, encoding, text) -> Positions`、`textEdits(positions, changes) -> TextEdit[]`、`utf16At(text, byte) -> number`：一个版本的三种坐标——版本里的字节、解码出的字符（`place` 的列按码点数）、编辑器里的 UTF-16 码元——只在这里换算（refrain 路线图 §4-8），每个 `Positions` 带它所属的版本，所以一个坐标不会被拿去量另一版。编辑器的文本（`editor`）是这一版的文本去掉开头的字节顺序标记、每个换行（`\r\n`、`\r`、`\n`）读成一个 `\n`；`bytes` 与 `editorAt` 把这两处折叠还原，`textEdits` 把编辑器里的改动写成基线那一版的字节区间加文本，插入的换行写成这一版第一个换行的写法（`lineBreak`，没有换行时是 `\n`），所以没碰过的字节——标记、混合换行、尾随空格——一个不动（4-46）。落在一个字符中间的位置（UTF-16 代理对的两半、多字节字符的中间、`\r\n` 的两半）退到那个字符的开头。一次换算 O(`STRIDE`)：`positionsOf` 走一遍文本，每 `STRIDE`（1024）个编辑器位置记一个检查点。`utf16At` 是没有版本的一段文字（一段回复）里同一种换算：UTF-8 的第几个字节落在文字的第几个 UTF-16 码元，落在字符中间的退到那个字符的开头，超出文字的答文字的长度 |
| `document_windows.ts` | 1 判定 | `Opened`（`missing`／`unreadable { reason }`／`opaque { version, bytes }`／`text { gathering }`）、`opened(answer: DocumentAnswer) -> Opened`、`recorded(version, format) -> Gathering`、`Gathering { version, format, encoding, bytes: number \| null, text, through }`、`nextSpan(gathering) -> Span \| null`、`joined(gathering, answer: RangeAnswer) -> Gathering`、`whole(gathering) -> boolean`、`EDITABLE_BYTES_MAX`：一个版本的文本由第一窗与其后按版本读的 `Range` 窗口接成（`crates/wire/Spec.lean` §8-69、§8-70）；`nextSpan` 只在没接齐、且这一版不超过 `EDITABLE_BYTES_MAX`（4 MiB）时答下一段，接不上 `through` 的窗口（别的版本、重复、乱序到达）不改变 `Gathering`。空文件是零字节、已接齐的 `Gathering`，可以写出第一版；接齐之前与超过上界的版本只读（4-46）。`recorded` 是一个只知道摘要的版本（打开的不是城此刻的那一版，或草稿立在一个旧版本上）：长度未知（`bytes` 为 `null`），从第 0 个字节读起，答回一个空窗口时长度才定下来；编码按 `utf8` 记，因为线上只在 `Document` 的答复里说编码，而这样的版本只读，带标记的 UTF-8 的标记在这种读法下仍占三个字节，只有 UTF-16 的版本位置会偏，它们本来就没有预览 |
| `document.ts` | 1 判定 | `DocumentText { text, bytes, binary, truncated }`、`readDocument(answer) -> Answered<DocumentText>`：城的一份文件按显示它的页面怎么画读出（第一窗的文字、文件多长、是不是文本、窗口有没有漏掉一部分），三个读文件的页面经这一处读，所以它们一致 |
| `document_save.ts` | 1 判定 | 草稿：`Draft { version, changes }`、`draftPlace(doc) -> string`（`prefs.ts` 草稿门的地点 `document:<地址>`，冒号不在地址文法里，所以它与房间的草稿不会同名）、`writeDraft(draft) -> string`、`readDraft(stored, length) -> Draft \| null`（存的值读不出、改动乱序、重叠或越过基线的长度时答 `null`）。回执：`Receipt`（`clean`／`draft`／`saving { sent, edited }`／`pending { sent, edited }`／`saved { version }`／`conflict { current }`／`refused { error }`）、`Sent { doc, baseline, command }`、`saveOf(doc, positions, changes) -> Sent`、`receiptIn(records, sent) -> B3Hash \| null`、`RECEIPT_QUERY`、`advance(receipt, Happened) -> Receipt`，`Happened` 穷尽：`edited { empty }`、`based { empty }`、`sent { sent }`、`lost`、`relinked`、`landed { version }`、`refusal { error }`、`moved { version }`。只有带着这次保存的 `idem` 的 `document_written` 行能把 `saving`／`pending` 变成 `saved`（`crates/wire/Spec.lean` §8-72）；拒绝不带 `idem`，所以保存途中动作是 `save a document` 的拒绝归这次保存，`E_VERSION_CONFLICT` 变 `conflict`，其余变 `refused`，草稿都留着；链路断在保存途中是 `pending`（结果未知），重连后用同一个 `idem` 再发，城答它第一次的结果，不会落第二次；城换了版本而页面有草稿是 `conflict`，只有 `based`（草稿已改立在城此刻的版本上，或已丢弃）离开它 |
| `document_bytes.ts` | 1 判定 | `Fetching { version, size, through, parts }`、`fetching(version) -> Fetching`、`nextBytes(fetching) -> Span \| null`、`joinedBytes(fetching, answer: BytesAnswer) -> Fetching`、`Fetched`（`fetching { through, size }`／`whole { bytes }`／`too_large { size }`）、`fetchedOf(fetching) -> Fetched`、`storedObject(locator) -> B3Hash \| null`、`DRAWN_BYTES_MAX`：一个存在内容库里的对象的字节由 `Query::Bytes` 的窗口接成（`crates/wire/Spec.lean` §8-80）；`nextBytes` 从已接到的地方问到对象末尾，长度未知时问到 `Number.MAX_SAFE_INTEGER`，城每窗至多答 1 MiB；接不上 `through` 的窗口（别的版本、重复、乱序）与读不出的 base64 不改变 `Fetching`；对象长过 `DRAWN_BYTES_MAX`（64 MiB）时停在第一问之后，`fetchedOf` 答 `too_large`，因为画它的工具要整份字节都在内存里。`storedObject` 把一个整对象的 `cas:` 定位读成它在内容库里的地址，带区间的、`file:` 的定位答 `null`（4-45）|
| `results.ts` | 1 判定 | `Showing = whole \| results`、`drawsCalls(showing)`（房间在 `results` 下不挂载 `calls.svelte` 与推理折叠）、`Outcome = waiting \| failed \| done \| ended`、`outcomeOf(run)`、`resultsOf(runs, first) -> Group { outcome, first, total }[]`（一遍分四类，每类按 `started` 新到旧只留前 `first` 条，`FIRST = 5`）；`bandsOf(runs, now) -> Band { recency, runs }[]`（城在 `results` 下按时间读：新到旧，切成最近十分钟、这一小时、更早三段，空段不画；没有 `started` 的 run 落在「更早」末尾）；`producedOf(files) -> Produced { files, added, removed }`（房间在 `results` 下结局分隔线之下的产出一行：改了几个文件、共加减几行；二进制文件计入文件数、不计行数，因为 `Lines::binary` 没有行数可加）；只看结果模式「画什么」的唯一判定处。200 个 run 的夹具城分类耗时由 `results.test.ts` 判定并打印 `city_results` 行，登记于 `tools/xtask/budgets.toml` |
| `replying.ts` | 1 判定 | `Laying { blocks, reached, judged, refused }`、`UNLAID`、`question(text, laying, state) -> string \| null`、`answered(laying, asked, answer, state) -> Laying`：一段回复问 `Query::Reply` 的那一半（4-26、D31）。`reached` 是已画成块的那一段在文字里的 UTF-16 长度；`question` 答下一问带的文字——还在说时是未画部分里的完整行，结算时是整个未画部分——没有新的可问时答 `null`；`judged` 是城已经判过、不必再问的那一段，`refused` 是城答过 `Unavailable` |
| `route.ts` | 1 判定 | 见 §3-2 |
| `lang.ts` | 6 数据 | 见 §3-1 |
| `time.ts` | 1 判定 | `ago`, `clock`, `hhmm`, `hhmmss`, `isoInstant`, `isoDay`, `isoTime`, `lasted`, `count`, `kilo`, `usd`, `kib`；`isoDay(at)`／`isoTime(at)` 是一个账本时刻按 UTC 的两半——`2026-10-02` 与 `03:04:05.678Z`——时间轴的轴头写一次日期、每行写到毫秒的时刻（`docs/frontend-method.md` §7D），所以「这个时刻在 UTC 里怎么写」只有这一处，`isoInstant(at)` 是两者相接、`<time datetime>` 读的整个时刻，视图不自己相接；`kilo(n)` 是一眼比较两个 token 数时的写法：千记 `k`、百万记 `M`、三位有效数字、不留尾零，千以下照写（上下文环的 `82.4k / 200k`） |
| `notify.ts` | 1 判定 | `notices(heard, items, scene) -> [Heard, ApprovalItem[]]`：哪些待批事项变成一条浏览器通知。`Heard` 是「快照未到」或「已算过的 `ApprovalId` 集」；`Scene { notifying, focus, elapsed, watching }`。只对需要人决定的事（`approval_queue` 的答）发，四道闸全过才发：窗口失焦、过了预热期 `WARMUP_MS`、不在首个快照里也不在已算过的集里、不是正在看的那个地址（`item.actor`）。每个见过的 id 都记进 `Heard`，所以一件事在任何一道闸下被放过一次就永远不再弹。适配器是 `views/notifier.svelte`（权限为 `granted` 才 `new Notification`），开关是 `prefs.ts` 的 `notifying`，默认 `off` |
| `deferral.ts` | 1 判定 | `Urgency = "needs_you" \| "ordinary"`、`urgencyOf(error) -> Urgency`（按 `AxCode` 穷尽的一张表：哪些拒绝不等人就动不了）；`Box = absent \| holding \| emptied { at, by: "send" \| "hand" }`（对话框此刻的状况：页面没有对话框、框里有字、从某一刻起是空的以及是发送还是人手清空的）、`Attention { box, visible, returnedAt }`、`Moment = "sent" \| "idle" \| "returned"`、`momentOf(attention, now) -> Moment \| null`、`deliverable(urgency, attention, now) -> boolean`、`wakeAt(attention, now) -> number \| null`（不再发生任何事时下一个时刻在哪一毫秒开始，只有一个事件能打开时刻时为 `null`）、`IDLE_MS = 5000`、`RETURNED_MS = 1000`：普通通知何时主动冒头的唯一判定（4-35、D26）；`needs_you` 恒可投递，`ordinary` 只在一个时刻里投递，其余时候只动信箱键上的标记。适配器是 `views/mailbox/attention.ts`（从页面读出 `Attention`）与 toast 座位 `views/refusal.svelte` |
| `keys.ts` | 1 判定 | `ACTIONS`、`Action`、`Chord`、`DEFAULTS`、`LABELS`：外壳听的每一个键在这一张表里；`readChord(text)`、`spell(chord)`、`marks(chord, platform)`、`platformOf(userAgent)`、`matches(chord, pressed)`、`reserved(chord)`、`conflictsOf(bound)`；`loadKeys(door, userAgent) -> Keymap` 把人的覆写（`prefs.ts` 的 `chord`）叠在默认上，`keymap()` 是页面的那一份。左下三键的名字、按住即现的提示、外壳与设置页都读它，没有一处自己拼一个键；会改变状态或打开浮层的动作只用带 accel 的组合键，单键只剩 `composer.focus`（`/`，只把焦点移进输入框，D92）；外壳自己的动作是 `tier.cycle`（Accel-\，图层键）、`mailbox`（Accel-B，信箱键）、`inspect`（Accel-J，右侧的开合）、`help`（Accel-`/`，快捷键速查）与 `go.setup`（Accel-,，设置键），`fork.here`（Accel-Shift-F）只由线程听，`decide.yes`／`decide.edit`／`decide.no`（Accel-Shift-Y／E／X）只由持焦点的请决定卡听，外壳不答它们，默认表里没有两个动作共用一个键；`run.stop` 的标签是 `run_cancel`（`/stop`），它发的是对眼前的 run 的 `cancel`，不是整城的 `halt`（D16）；`finder`（Accel-P）打开找文件的面（4-62）。`folded(key)`（不分大小写的唯一定义）与 `face(key)`（一个键画成什么）也由它交出，`lines.ts` 读这两个 |
| `press.ts` | 2 值 | `Pressed`：两张键表判的同一条按下记录；`pressedOf(event)` 是读一次 `KeyboardEvent` 的唯一一处：落在文本框里或正在组合输入的按下算作 `field`；`HOLD_MS`（300）是「按住」的唯一定义，按住看一眼与按住即现都读它 |
| `lines.ts` | 1 判定 | 行间走动的表 `LINE_KEYS`（`LineMove` → 键序列：`line.next` ↓／j、`line.previous` ↑／k、`line.first` Home／gg、`line.last` End／G、`line.open` Enter、`line.close` Esc；每一步的第一个键是行尾画的那一个），`lineWalker()` 交出一个读者 `(pressed, at) -> LineMove \| null`：只认文本框之外、不带修饰键的按下，gg 是两次 g 相隔不超过 `SEQUENCE_MS`（1000 ms），其间任何别的键都把第一个 g 作废；`lineFaces(move)` 是每个键画出来的样子，字母照打出的大小写画（g 与 G 是两个键）。`initialOf(name, slug)` 是一行按首字母到达时用的那个字母：名字的第一个字是一个键打得出的拉丁字母或数字时取它，否则取 `slug` 的第一个字母（D40）；`initialTyped(pressed)` 读出一次按下是不是这样一个字母。工具行、信箱的条目、run 板与设置树读它，没有一处自己拼一个走动的键 |
| `in_front.ts` | 1 判定 | `runInFront(belief, view) -> RunBelief \| undefined`：「眼前的 run」的唯一答案。对话页是这个房间最新的在干活的 run（`newestWorking`），run 页是地址里那个仍在干活的 run，城页、楼页、monitor 与其余页面没有——它们同时画着很多 run，替人挑一个就是替人决定停哪个。Accel-.、palette 与输入框的 `/stop`、`/steer`、`/diff` 都读它（4-41、D16），O(L) |
| `slash.ts` | 1 判定 | `SLASH`：页面动词的唯一表（4-41）；`parse(line) -> SlashCall \| null`、`find(verb)`、`offered(line)`（`/` 菜单与 palette 按输入过滤）。每个动词经 `parse`、`find` 与 `run` 发出的帧由 `slash_frames.test.ts` 逐条整体比较，并按线上的 `ClientFrame` 解码一遍；用例表以 `SLASH` 的拼写为键，表里多一个动词而没有用例时那张表自己变红（Roadmap G1c）。`STOP` 与 `RELEASE_ALL` 是按钮上也写的两个动词拼写，读它们的按钮不在 `lang.json` 里另拼一份；`/admit [standing\|tested\|contract\|double]` 改下一次派发的准入要求，`/room <addr>` 去那个房间的对话（4-60） |
| `slash_session.ts` | 1 判定 | `fresh`、`compact`、`retag`：会话动词 `/new`、`/compact`、`/tag`／`/untag` 做什么，经 `SlashHands` 开会话、改标签；动词表与拼写仍只在 `slash.ts` |
| `slash_hands.ts` | 2 值 | `Slash { spelling, grammar, about, section, run }`、`Section`、`SECTIONS`、`SlashCall`、`SlashHands`（视图用手上已有的东西填它，动词从不点视图的名）、`Reached`／`reached(run)`（动词能作用的那个 run 与它走到的位置）、`Offered`；`SessionHands { tagged, retag(next), whenFrozen(run, then) }` 是 `SlashHands` 扩展的那一半：`tagged` 是主区里那一段按标签的叫法连同它现有的标签，城没有名字或这一段还没被 `Query::Sessions` 答出时为 `null`，`whenFrozen` 在 belief 里这个 run 冻结时调一次 `then` |
| `tags.ts` | 1 判定 ＋ 3 存取 | `PIN: Tag`（页面的保留词 `pin`，线上它只是一个合法标签，`crates/wire/Spec.lean` §8-84）；`Named { city, room, began }`（一段 session 按标签的叫法）与 `namedIn(city, room, began)`（握手没给城名时为 `null`，标签无处可存）；`readTag(raw) -> Tag \| null`：去两端空白、折成小写、再按生成的 `Tag` 文法读，读不出为 `null`，**大小写折叠只在这一处**；`tagsOf(held, named)`；`given` 与 `stripped` 答这一段新的**整组**标签（`SessionTags`），即 `PreferencePatch::Tags` 要带的那一组；`retagged(held, next)` 把一组落到本页持有的表上，与城的 `wire::preference::tag::retagged` 同义（换掉这一段的整组，空组删条目），本页据此先画，城的下一个答案整表取代它；`inUse(held, city)`：这座城里用过的标签（不含 `pin`），字典序；`Tagging { held, retag(next) -> boolean }` 与 `keepTags(conn)`：每个 `Query::Preferences` 的答案整表取代 `held`，`retag` 先落本页、再发 `PutPreferences { tags }`，连接不在时不落、答 `false`（D45） |
| `stretches.ts` | 1 判定 | `Stretch { room, line: SessionLine, current, runs }`：一段 session，`current` 是房间最新的一段（城只接着最新的一段说下去），`runs` 是本页持有的、最后一行落在 `[began, 下一段 began)` 里的 run，旧的在前；`stretchesOf(answers, runsIn)`：每个房间的 `SessionsAnswer` 摊成行，按 `line.at` 新的在前；`lineIn(answer, asked)`：路由点名的那一段（没点名或点的就是最新一段时答最新一段），答复里没有它时为 `null`；`Pinning = "mayor" \| "tagged" \| "none"` 与 `pinningOf(stretch, tags)`：`hall/mayor` 的当前一段恒为 `mayor`（推出，不存储），带 `pin` 标签的为 `tagged`；`grouped(stretches, tagsFor, filter)`：置顶组在前，其余按楼分组，楼的次序取它最新一段的次序，组内新的在前，`filter` 非空时只留带这个标签的行，空组不画；`tailOf(stretch) -> Origin \| null`：信箱「最近」段从这一段分叉时的位置（最后一个 run 的最后一行），本页没有它的 run 时为 `null`（D44） |
| `completion.ts` | 1 判定 | `completed(line) -> string`：Tab 把一行按动词表补到所有匹配共有的最长前缀 |
| `forking.ts` | 2 值 | `forkAsked`（计数的 store）与 `askFork()`：`/fork` 不带地址时向对话页要选行器，对话页看着这个计数打开它；计数而不是布尔，因为选行器开着时的第二次请求也要被听见 |
| `recovering.ts` | 1 判定 | `Recovery`（命令／`reconnect`／`settings`／`reload`／`form`）、`recoveryFor(error) -> readonly Recovery[]`、`linkRecovery(code)`、`formOf(room, subject, words) -> Option<Form>`：拒绝到出路的唯一表，toast 与信箱的通知段都读它（4-35、4-35a） |
| `probed.ts` | 4 适配器 | `readProbed(data) -> Probed \| null`：`endpoint_probed` 记录的载荷在这里收窄一次，离开这个文件的是类型；`normalisedFrom(probed, typed) -> Normalised \| null`（城把人打的地址补成了什么）、`stoppedAt(reach) -> Key`（探测停在哪一步的说法） |
| `share.ts` | 1 判定 | `fraction(ppb)`、`percent(ppb)`：城以十亿分之一计份额（`kernel::share::WHOLE_PPB`），这是客户端唯一拼写那个整体的地方 |
| `pursuit.ts` | 1 判定 | `pursuitClause(lang, verdict)`：城给出常设目标的判词种类，词由 `lang.json` 给 |
| `provider_failure.ts` | 1 判定 | `providerClause(lang, failure, retry)`：模型调用失败时人读的出路；失败的种类与值不值得再试由城判，词由 `lang.json` 给 |
| `removal.ts` | 1 判定 | `Removal = offered \| hall \| busy`、`removalOf(building, living)`：楼页给不给「移除这栋楼」；城自己拒两种情况，页面先问，免得给人一个只会答拒绝的按钮 |
| `live_output.ts` | 7 投影 | `Tail`、`NO_TAIL`、`LIVE_LINES`、`appended(tail, piece, cap)`：正在跑的命令已经写出的行，按行封顶、最旧的先丢并计数；该调用的 `tool_result` 一到就丢掉，因为账本里的结果才是那段输出的权威 |
| `monitor.ts` | 1 判定 | `rows(samples, width) -> Row[]`（`Row { label, reading, p50, p99, plot }`）、`plot(values) -> Plot`（`Plot { bars, p50, p99 }`，高度以 `FULL` = 1000 的千分计）、`summary(latest)`：性能面板读监视器历史。读数按 `sprawling top` 的单位规则写，所以页面与终端对同一个样本读出同一个数；柱子是页面自己的画法，按窗口自己的最小值到最大值分级，标出窗口的 p50 到 p99 一带（D82） |
| `timing.ts` | 4 适配器 | `markStart(interaction, at?)`、`markEnd(interaction)`、`markEndAtFrame(interaction)`、`measured(interaction) -> number[]`、`forget(interaction)`、`resolution() -> "isolated" \| "coarsened"`、`timeAtRoot(root) -> stop`：M0 第 4 条的关键交互（`Interaction`：`keystroke_echo`、`send_shown`、`layer_drawn`、`panel_frame`、`session_switch`、`mailbox_open`、`settings_switch`、`scroll_frame`）各是一对 User Timing mark 与其间的一个 measure（名前缀 `sprawling:`），headless 测试、真浏览器的性能面板与读 `performance` 的测量台读同一批条目；本模块不取读数、不存数。没有开着的起点的终点不记；每种交互至多留 `KEPT`（10,000）个 measure，满了从头再记。时长按浏览器 `performance.now` 给的分辨率取：没有跨源隔离时各浏览器把它放粗到 100 µs 或更粗，每个 measure 的 `detail.resolution` 写明是哪一种，Windows、macOS、Linux 上同一浏览器行为相同。八种都已接上：`main.ts` 的 `timeAtRoot(document)` 记按键到回显与滚动到帧；其余六种在各自的视图里，起点是人的动作，终点是其后的下一帧（`markEndAtFrame`）——`layer_drawn` 起于 `app.svelte` 的 `cycleTier`，终于换档或开合世界面之后的一帧，从别的页回到对话再换档的，终于 `settle` 落档之后；`session_switch` 起止都在 `app.svelte` 的 `settle`，地址栏从对话的一段换到另一段（别的房间或同房间的另一段）时记；`send_shown` 起于 `talk/composer.svelte` 的 `submit`，连接收下这句话时终于框清空、回执画出的那一帧；`panel_frame` 起止在 `right.svelte` 的 `pick`（点右侧的一个标签）；`mailbox_open` 起止在 `mailbox/mailbox.svelte` 的 `feed`，信箱由合变开时记；`settings_switch` 起于 `settings/hosted.svelte.ts` 的 `pickGroup`，终于 `hostSettled` 读到组变了之后的一帧，所以后退键换组不记（没有开着的起点） |
| `watching.ts` | 4 适配器 | `createWatching(sendText) -> Watching`：本页对监视器说的最大需要（面板开着 `watch`，只有设置树「性能」条目旁那一行摘要时 `watch_summary`，都没有时 `release`），新连接上再说一次；城只在有人看时采样 |
| `prefs_city.ts` | 4 适配器 | `keepWithCity(door, conn)`：偏好门与连接唯一的接点（4-29）；`adopted(held, answer)` 把城的回答盖在浏览器此刻的记录上，回答缺席或答「没说过」（`null`）的字段留浏览器的值；`blend_percent` 不在 `BLEND_PERCENT` 的域里时读作没有透明度；`appearanceOnWire(next)` 是外观记录的线上拼写（含 `glass` 与 `blend_percent`，4-60） |
| `commands/endpoint.ts` | 2 值 | `Endpoint`、`Pair`、`Tuning`、`providerName(name)`、`probeEndpoint(e) -> Command`、`attachEndpoint(e, admit) -> Command`：挂一个端点要说的一张表单与它的两个帧；单列，因为只有这一族是人跨几屏填完才发的表单 |
| `commands/building.ts` | 2 值 | `configureMcp(addr, mcp)`、`configureDesktop(addr, allowlist)`、`configureContext(addr, percent)`、`configureSandbox(addr, limits)`：`ConfigureBuilding` 带的一栋楼的四个面各一个构造函数，各填自己的面、其余三面写 `null`（城把 `null` 读作「这一面不动」）；因 `commands.ts` 的长度预算与它分开 |
| `commands/document.ts` | 2 值 | `putRange(doc, baseline, edits)`、`decideProposals(doc, decisions)`：写一份文档的两个命令；版本已经移动时城拒绝保存，草稿留在页面上，回执是带这个命令键的 `document_written` 行（`crates/wire/Spec.lean` §8-72） |
| `commands/door.ts` | 2 值 | `openRemoteDoor(lastingMs)`、`replaceCityKey()`、`confirmRemoteDoor(code)`、`closeRemoteDoor()`：远程门的四个命令；开门与换钥匙只是请求，城在自己的控制台印一个码并答 `E_APPROVAL_PENDING`，码经 `confirmRemoteDoor` 回来（`crates/remote_access/Spec.lean` D4）；关门不要码 |
| `commands/guide.ts` | 2 值 | `putGuide(progress)`：这座城上手指南的进度整份写入，两次写入以后到的为准（`crates/wire/Spec.lean` §8-68） |
| `remote/base32.ts` | 1 判定 | `encode(bytes) -> string`、`decode(text) -> Uint8Array \| null`、`canonical(typed) -> string`（去掉空白与连字符、转小写）：RFC 4648 base32，小写、不补 `=`，与 `remote_access::pairing` 同一个字母表；配对码、城的指纹与设备种子在页面上只有这一种写法 |
| `remote/invitation.ts` | 1 判定 | `isInvitation(hash) -> boolean`（片段带 `pair` 这一键）、`invitationIn(hash) -> Invitation \| null`，`Invitation { code, city }`：码规范化后须是 26 个 base32 符号，指纹须是 52 个符号并且是某 32 字节的规范写法，与 `CityFingerprint::read` 同一条判断（`crates/remote_access/Spec.lean` §8-6）；读不成答 `null`，页面说「重新扫码」 |
| `remote/keys.ts` | 2 值 | `SEED_BYTES`、`PUBLIC_BYTES`、`SIGNATURE_BYTES`；`halves(seed) -> Promise<[ed25519, mlDsa]>`（HKDF-SHA256，盐 `sprawling remote key v1`，§8-3）；`keyFrom(seed) -> Promise<DeviceKey \| null>`，`DeviceKey { public, ed25519, mlDsa }`：`ed25519` 是 WebCrypto 的不可导出私钥，`mlDsa` 是 ML-DSA-44 的私钥字节；`sign(key, message) -> Promise<Uint8Array \| null>`、`verifies(public, message, signature) -> Promise<boolean>`：两半都成立才成立，不说是哪一半没过。浏览器不给 Ed25519 时 `keyFrom` 答 `null` |
| `remote/agreement.ts` | 1 判定 | `Ephemeral { x25519, mlKem }`、`ephemeral() -> Promise<{ ephemeral, public } \| null>`（临时 X25519 由 WebCrypto 生成，ML-KEM-768 由 `@noble/post-quantum`）；`record(label, opening, unsignedAnswer)`（握手记录，SHA-256）、`signed(label, role, transcript)`、`derive(transcript, xSecret, kemSecret) -> Promise<SessionKeys>`、`agreed(ephemeral, transcript, xPeer, ciphertext) -> Promise<SessionKeys \| null>`：§8-4 的会话密钥，两种握手共用 |
| `remote/seal.ts` | 5 状态 | `Direction = "device_to_city" \| "city_to_device"`、`sealer(key, direction)`、`opener(key, direction)`：AES-256-GCM，随机数是方向标签的四个字节接 8 字节大端计数，计数在调用的那一刻占下，所以并发的两次 `seal` 不会用同一个随机数；打不开答 `null`，会话随之结束（§8-5）。`Payload = frame { text } \| lock`、`payloadBytes(payload)`、`payloadOf(bytes) -> Payload \| null` |
| `remote/handshake.ts` | 1 判定＋5 状态 | 配对：`pairHello(nonce) -> Promise<DevicePairing \| null>`、`claim(pairing, reply, invitation, key) -> Promise<Claimed \| Refused>`；会话：`sessionHello(device, nonce) -> Promise<DeviceWaiting \| null>`、`finish(waiting, reply, city, key) -> Promise<Finished \| Refused>`。`Refused = "length" \| "fingerprint" \| "signature" \| "agreement" \| "key"`：消息长度不对；城出示的公钥与邀请里的指纹不符；城的签名不对；密钥交换不成；这台设备自己的密钥签不出或封不上。指纹或签名不对时设备在发出认领之前停下，配对码没有离开设备（§8-6） |
| `remote/connect.ts` | 4 适配器 | `remoteUrl(location, path)`（`wss://<主机>/remote/pair` 与 `/remote/session`，`https:` 之外答 `null`）；`pairOver(url, invitation, key) -> Promise<Paired \| Refusal>`：一条配对连接上三条消息一条回执，答出城的公钥与门给的 `DeviceId`；`sessionOver(url, device) -> Promise<Opened \| Refusal>`：一次会话握手，答出这条连接（`Door`：逐条读二进制消息的 `next`、`send`、`close` 与关闭帧的原因 `closed`）与这次会话的封与开；`lockOver(url, device) -> Promise<Locked \| Refusal>`：`sessionOver`，然后一帧封好的 `Lock`（§8-5、D5）。`Refusal = refused { why: Refused } \| closed { code } \| unreachable`：`closed` 的 `code` 是城在关闭帧原因里写的那个稳定码（§8-10），没有时为 `null` |
| `remote/session.ts` | 4 适配器 | `dialFor(location) -> Dial`：每次连接判一次走哪条线——`https:` 且这个源存着配对过的设备时是远程会话，否则是 `plainDial(socketUrl(location))`；`talk(door, session, hearing) -> Line`：握手之后的会话，发出的帧按调用的次序一帧封完再封下一帧，收到的帧一帧开完再开下一帧，打不开、读不成或收到锁门时关上这条线（4-64） |
| `remote/device.ts` | 4 适配器 | `kept() -> Promise<Device \| null>`、`keep(device) -> Promise<boolean>`、`forget() -> Promise<boolean>`：这台设备配对的结果存在这个源的 IndexedDB 里，`Device { city, fingerprint, id, key, at }`；`key.ed25519` 以 `CryptoKey` 原样存入，浏览器不让任何脚本导出它。种子不存 |

`src/ui.ts` 是视图拿到的一切，一个上下文、一个取法：`setUi(value)` 由 `app.svelte` 挂载时调一次；后代组件在初始化期调 `ui(): Ui` 拿到 `Ui { conn, prefs, tags, lang, effort, policy, conversing, approvals, bar, origin, pairing, now, chooseEffort, choosePolicy, go, send, hearing }`。旧的 `useUi`／`useSay`／`useGo`／`useCommand`／`useHearing`／`useApprovals` 等透传壳收敛成这一个门（AGENTS：不做只改名的壳）。**词不是上下文**：`core/lang.ts` 的 `say(lang, key)` 保持纯函数，插槽由 `fill(pattern, slots)` 填，模板写 `say($lang, key)`，`$lang` 的订阅就是换语言时重画的来源。`pairing` 是开这一页的地址栏上的配对码，两扇会动作的 HTTP 门要它。

### 视图（免 SPEC，列出以便定位）

`views/parts/tip.svelte` 提示（见设计 4-18）；`views/banners.svelte`（每页之上的那一格：断线与停城两条横幅，`link_banner.svelte` 是前一条）；`views/talk/drop_refused.svelte`（拖到框上没收下的文件，各带城给的原因）；`views/parts/unkept.svelte`（浏览器没存下的草稿：一句话与复制，4-63）；`views/parts/unanswered.svelte`（城没能回答的那一问，见 §7 `answered.ts`）；`views/parts/notice_title.ts` 通知的标题（见设计 4-35）；`views/parts/code.svelte` 只读代码视图（面包屑＋行号＋语法着色，见设计 4-26）＋ `parts/inked.svelte`（按语法上色的文字，文件视图与 Markdown 代码块共用）＋ `parts/code.ts`（五种 `Ink` 与取高亮器的入口）＋ `parts/paint.ts`（懒加载的高亮器块）；`views/edge.svelte` 左下三键（`docs/frontend-method.md` §7E）；`views/talk.svelte` ＋ `talk/{thread,turn,calls,call_line,head,sparkline,scroller,wear,delivered,composer,waiting}.svelte` 对话（线程的读数与阅读位置见设计 4-44；`talk/{timing,call_kind,rhythm,delivery,naming,frozen,around}.ts` 与 `talk/arrivals.svelte.ts` 是它们读的判定（`around` 是信件只画发信那一回合与前后各一回合，D91）；`naming` 是对话怎么称呼它的居民与 `Opening.dispatched_by` 读成谁派来的活（`dispatcherOf`，D83），`frozen` 是首条消息头与检阅档「模型」格同写的强度与模式，见 4-44、4-59） ＋ `talk/person.svelte`（人说的一句，画成一个气泡）＋ `talk/note_line.svelte` 与 `talk/note_line.ts`（线程里的一条注记——到达、拒绝、等人、等回信、丢弃、读不回的行——按它在账本里的行排；检查点不画，它是 run 页的事实）＋ `talk/{letter_note,handback_note,reply_wait_note}.svelte`（居民的来信、子会话的交回与一次等回信，D86）＋ `talk/inbox.svelte` 与 `talk/inbox.ts`（这个房间排队未读的信：种类、首行、去发信房间的链接与单复数的条数，D86）＋ `talk/{record,send}.svelte`（composer 的麦克风与发送键）＋ `talk/handed.svelte`（发出后给读屏器的一句回执）＋ `talk/drop_zone.svelte.ts`（框作为拖放目标：拖过时的底色与没收下的文件）＋ `talk/standing.ts`（这个标签页里每个地方的选区与阅读位置，与 ↑ 取回的原文，见设计 4-63）＋ `talk/quoting.ts`（一行引文交给开着的框或草稿行，4-63）＋ `talk/copying.ts`（复制整段排好版的回复时放进剪贴板的是模型写的 Markdown，只复制一部分时交给浏览器）＋ `talk/listening.svelte`（房间芯片菜单头上的「谁在听」，见 `docs/frontend-method.md` §7I、D36）＋ `world/context_bar.svelte`（会话栏一行的上下文细条，见 7K）＋ `world/speed.ts`（检阅档两个速度格——首块用时与输出速度——的数：首块用时与 t/s 的中位数与分布、各计入几个回合，见 7K）＋ `world/{sessions,session_menu}.svelte` 与 `world/stretches.svelte.ts`（会话栏、它的行菜单与它读的各段，见 7K）＋ `talk/past.svelte`（主区里过去的一段，见 7K）＋ `talk/asked.svelte`（审批卡问的内容：经 `Query::Content` 读 `ApprovalItem.artifact`） ＋ `talk/produced.svelte`（一个 run 的产出短语「n 个文件 · +a −b」，由 `opened_at` 到最后一次检查点的 `Changes` 答案求和；结果房间的块与结果城的行共用）＋ `talk/earlier.ts`（更早的一段画不画、折不折，两种模式共读，D80）＋ `talk/{stream,result}.svelte`（只看结果的房间：`stream` 按会话分组——开着的这段在前、以它怎么开始命名，其前的并为「更早的会话」，组内新的在前，「更早的会话」与全部模式的分隔线同样折叠，规则在 `talk/earlier.ts`，D80；`result` 是一个 run 一块：开始时刻、结局记号、任务，引出它最后说的话，末行写做完了与产出和 PR、失败或停下的原因与「没有提交」、等你与 `ask`、或在干活与打开监视）；`views/checkpoints.ts`（`lastCheckpointIn` 是 run 最后一次检查点的树，run 页与产出一行都量到这里；`bracketOf` 是一次调用前后的两个检查点，检视面的 diff 读它）；`views/right.svelte` 检视面（见设计 4-45 与 `docs/frontend-method.md` §7F；楼页装同一个，4-50 其三）＋ `inspect/open.svelte.ts`（右侧打开了什么，唯一的家）＋ `inspect/reading.ts`（一次调用按什么读、右侧跟着什么）＋ `inspect/{called,diff,patch,terminal,file,shot,strip,split,crumb,reach}.svelte` ＋ `inspect/terminal.ts`（终端印什么：在跑读尾巴，有结局读账本）；`views/refrain/formats/{label,frame,html,pdf,docx,compare,opaque}.svelte` ＋ `formats/{format,framed,pdf,docx,zip,compared}.ts`（RefRain 画的三种格式，见 4-54；`compared.ts` 把两版的比较写成 Markdown，见 4-61）；`views/refrain/fetched.svelte.ts`（一个对象的字节逐窗取回，见 4-61）＋ `views/refrain/saved_file.ts`（交给浏览器下载的一份文件）；`views/city.svelte` ＋ `city/{bar,panel,skyline,marks,results,produced}.svelte`（`results` 是只看结果的城：按结局过滤的分段控件带各类总数，其下是 `bandsOf` 的时间段，一行写开始时刻、结局、房间、任务与产出或停下的原因；`produced` 是一行「刚完成」末尾的产出，向这个 run 的 `Rounds` 要 `opened_at` 与最后一次检查点，再交给 `talk/produced.svelte` 的同一次 `Changes` 求和——`RunSummary` 不带产出数字，所以只有画出来的前 `FIRST` 行在问；「刚完成」一行另写 `RunBelief.pr`，「等你」一行写 `RunBelief.ask`） ＋ `city/shape.ts`（超椭圆路径）＋ `city/table.svelte` 楼表（城页默认的第一视图，画是一键可切的第二视图：账本树的第一层，一楼一行，等你／在干活／完成／最后开始四个数，加一条折叠刻度上的时间条，每个 run 的开始是一个按阶段着色的刻点）＋ `city/table.ts`（`tableOf`：楼到行，行是定长摘要，只存计数与各 run 的开始，不存历史；楼的归属、排序与树线取自 `runs/lineage.ts`）；`views/runs/board.svelte` 全城 run 板（城页画在楼表或图之下：楼是这棵树的根）＋ `runs/lineage.ts`（run 表到账本树的行、只画可见行的窗口，`boardRuns` 只读板要画的五个字段，一个流式 token 不重建整块板）＋ `runs/fold.ts`（时间条的刻度）＋ `runs/phase.ts`（阶段的颜色与说法，run 板与楼表的时间条共用一份图例）；`views/registry.svelte`（`Query::RegistryView`：这座城决定留下来的东西，一行一件，见设计 4-24）；`views/building.svelte` ＋ `building/{tree,rooms,directory,file,skills,plan,node_cost,commits,commit,commit_line,commit_facts,whose,status,take_back,sandbox,goal,opened}.svelte` ＋ `building/transcript.ts`（楼页的各节，见设计 4-50）；`views/changes.svelte`（`Changes`／`Hunks` 的一份读法，run 页与楼页共用；打开的一行画 `inspect/patch.svelte`）；`views/run.svelte` ＋ `run/{head,river}.svelte`（顶部统计栏与时间透镜）＋ `run/lanes.ts`（时间透镜画什么：每个回合一段、每列一个盒子、只画视口内的调用行，见设计 4-42）；`views/setup.svelte` ＋ `setup/{providers,models,skills,appearance,colours,keys}.svelte` ＋ `setup/providers/{account_editor.ts,accounts.ts,rosters.ts,accounts.svelte,accounts.look.svelte}`（一个供应方的有序账号：编辑器状态、外观收到的值、端点与搜索供应方两种名册、座位与外观，见 D93）＋ `setup/search/{search_editor.ts,search.ts,search.svelte,search.look.svelte}`（网络搜索卡：状态、外观收到的值、座位与外观，见 D94）（skills 组见设计 4-31；`colours` 是配色页）＋ `setup/colours.ts`（配色页读的判定：令牌表取自正在生效的样式表，覆写的穿上与脱下，APCA 可读性警告按 `theme/colour.css` 的 `--tier-*`、`--tier-slack` 与 `--surface-ceiling` 判）＋ `setup/kept.svelte`（一个组的答案由谁保管，见设计 4-29）＋ `setup/decided.svelte`（代为答复的记录：`GovernanceAnswer.decided`，画在审批控件之下）＋ `setup/groups.ts`（各组的标题、提示与宽度，见 `docs/frontend-method.md` §4-33 与设计 4-36）；`views/settings/{panel,sheet,tree,hosted.svelte}`（设置面、它装的两栏、设置树与外壳里它站在哪，见 7L）＋ `settings/tree.ts`（树的枝与条目）＋ `settings/saving.ts`（一张卡的保存走到哪一步，见 4-36）＋ `settings/{you,import,card,city_layer,rules,automation,admission,remote,remote_state}.svelte` 与 `settings/files.ts`（4-48）；`views/sheets.svelte.ts`（一栏外壳的两张面与它们的历史格，见 4-52）＋ `views/shared/frame.ts`（外壳几栏、可视视口，见 4-52、D30）；`views/shared/{provider,effort,buildings}.svelte` ＋ `shared/buildings.ts`（`buildingsOf`：hall 在首、其余按名排，楼列与规则组的选楼共用这一个次序）（从设置页的组里拆出的三件：`provider` 是接供应方的那扇门，`effort` 说强度住在哪一层，`buildings` 是楼列，它的第二个座位是 `#/mcp`）；`views/shared/outcome.ts`（结局的记号与墨色，结果城与结果房间共用）；`views/shared/showing.svelte`（只看结果开关，三个座位：房间、城、外观设置，都写同一条偏好，所以页上的选择就是下一页的默认）；`views/machine.svelte`（doctor 的答，这一页只管问）＋ `machine/{report,skeleton,unchecked,versions,pack,copy}.svelte`（`report`、`skeleton`、`unchecked` 是城答了、正在问、还没答三种状态；报告每行写状态、状态说不清时的原因、版本（`versions`：已装的、仓库钉住的、上游最新）、缺时的安装命令，判定是 `setup/dependencies.ts` 的 `rowOf`；`pack` 是 Rust 工具包一行一个安装键；`copy` 是复制一段文字的控件，doctor 页的命令、远程组的步骤、版本回答的更新命令与提交事实单的 oid 都用它，见 4-65）＋ `machine/scanning.{ts,svelte}`（报告末尾是城目录前的扫描：Windows 上读盘与排除项，macOS 与 Linux 写为什么不适用，判定是 `scanningOf`）；`views/release.svelte` ＋ `release/{reading.ts,answer.svelte}`（这一版是哪一版：每个问过的注册表一行，按装它的渠道写更新命令与复制键；`reading.ts` 是读法，`answer.svelte` 是画法，画廊从夹具画它）；`setup/{harnesses.ts,harness_cards.svelte}`（官方 harness 的卡：启动器缺、启动器在而 harness 没装或没登录、就绪三种状态，判定是 `harnessOf`）；`views/notifier.svelte`（不画任何东西，`core/notify.ts` 的适配器）＋ `setup/notifying.svelte`（外观组里的通知开关）；`views/desktop.svelte`（一栋楼的桌面白名单）；`views/mcp.svelte`；`views/settings/context_rung.svelte`（运行组里一栋楼的第二级提醒，4-66）；`views/welcome.svelte` ＋ `welcome/{guide.ts,body.svelte}`（上手指南：两种状态的判定与每一步挂的门，见 7G）；`views/record.svelte` ＋ `record/{timeline.ts,timeline.svelte,archive,bin}`（一条时间线与两种读法，见 4-51）；`views/cost.svelte`；`views/palette.svelte`；`views/refusal.svelte`；`views/prose.svelte`（结算的回复与楼页的 Markdown 文件，4-53）＋ `views/reply.svelte.ts`（一段回复问 `Reply` 的状态，与结算时从流式接过来的块）；`views/gallery.svelte` ＋ `gallery/doc.svelte`（doctor 页三种读法的每一种状态：harness 卡、版本回答、扫描）。

**`#/gallery` 是一条路由而不是一个构建开关**，因为量它的那道门应当打开一个人真正跑的 bundle；夹具不需要城（偏好走 `core/rows.ts` 那扇门，没有 localStorage 时是一张只活一次会话的表）。每个能进入多种状态的屏幕在那里各有一份夹具，`cargo xtask render` 打开真引擎读它。`app.svelte` 用动态 `import()` 取 `views/gallery.svelte`，所以画廊与它的夹具表是 bundle 里单独的一块，只在打开 `#/gallery` 时下载：其余路由首屏不再为它付字节，而 `frontend_artifact` 称的是整个 dist，这一块仍在其中。


**本节只说哪个屏用哪个部件；部件欠使用者什么写在 §9。**

**`views/parts/` 的五个复合部件各有生产座位**，`#/gallery` 只是它们的第二个读者：`combobox` 在 `setup/models.svelte`、`setup/model_table.svelte`、`talk/composer.ts` 与 `talk/pill.svelte`（一个端点答两百个模型时，下拉正是它替换的那个控件）；`notice` 在 `views/mailbox/{deciding,notices}.svelte` 与 `views/refusal.svelte`，AxError 的三段式因此只有它一个画法（4-35）；`row` 在 `building/commits.svelte`、`mcp/servers.svelte` 与 `record/ledger.svelte`；`skeleton` 在 `machine/skeleton.svelte`。`dialog` 的座位是撤不回来的删除，它们在发帧之前先问（D1）：删除 MCP 服务器（`mcp/servers.svelte`）与移除一栋楼（`building.svelte`，楼的文件随之搬出城）。设置页没有移除端点的控件，所以这一族里没有第三个座位；`part_remove_endpoint` 只由 `#/gallery` 的 dialog 夹具读。
-/

/-! ## 8 接口先行

### §3-1 `src/core/lang.ts`（形状 6 数据面 ＋ 一个查表函数）

```ts
export type Lang = "en" | "zh";
export const LANGS: readonly Lang[];              // 开关的顺序：en, zh
export type Key = keyof typeof table;             // lang.json 的键，snake_case
export function isKey(text: string): text is Key;         // 一串拼出来的文字是不是表里的键
export function say(lang: Lang, key: Key): string;
export function langOf(tag: string): Lang;        // 前缀匹配 zh*，其余 en
export function endonym(lang: Lang): string;      // 语言自称，永不翻译
export function fill(pattern: string, slots: Readonly<Record<string, string>>): string;
```

- `src/lang.json` 是全部对人的字句，且是唯一权威：每键一条 `{ en, zh }`。
- 漏译不可表示：`Key` 由 JSON 的类型推出，`say` 对不存在的键在编译期拒绝；测试另拒「中文栏与英文栏逐字相同」，例外是术语（以 `/` 或 `{` 起头，或单 token ≤12 字）。

### §3-2 `src/core/route.ts`（形状 1 判定）＋ `src/core/run_id.ts`（形状 4 适配器）

```ts
export type Lens = "ledger" | "archive" | "bin" | "log";
export const LENSES: readonly Lens[];
export const MAYOR: Address;                              // hall/mayor
export const SETUP_GROUPS: readonly SetupGroup[];         // 设置面画在正文里的组，地址栏认得的全部拼写
export type SetupGroup = "you" | "accounts" | "harnesses" | "network" | "remote" | "run" | "rules"
  | "automation" | "skills" | "tools" | "appearance" | "keys" | "advanced" | "about";
export type RunLens = "time" | "turns" | "monitor" | "prompt" | "context" | "changes" | "evidence";
export const RUN_LENSES: readonly RunLens[];              // run 页的七个透镜，按页签的次序
export type ItemLink =                                    // 链接点名的右侧项（4-63），与 `inspect/open.svelte.ts` 的两种项同形
  | { kind: "call"; run: RunId; at: Seq }
  | { kind: "document"; building: Address; path: string; version: B3Hash | null };
export type View =
  | { kind: "talk"; address: Address; item?: ItemLink; session?: Seq } | { kind: "city" } | { kind: "building"; address: Address }
  | { kind: "run"; run: RunId; lens?: RunLens } | { kind: "setup"; group?: SetupGroup } | { kind: "mcp" } | { kind: "record"; lens: Lens }
  | { kind: "cost" } | { kind: "registry" } | { kind: "welcome"; step?: GuideStep } | { kind: "monitor" } | { kind: "gallery" };
export const DEFAULT_VIEW: View;                          // 与 MAYOR 的对话
export function toFragment(view: View): string;           // 恒以 `#/` 开头，每个 View 恰一种写法
export function fromFragment(raw: string): Option<View>;  // 认不出答 None，不悄悄回首页
export const PAGES: readonly string[];                    // 人能输入的页名，由写法表导出
export function page(name: string): Option<View>;         // 一个裸页名到达的页，菜单经它取片段
export function unresolved(hash: string): Option<string>; // 空片段答 None；认不出答 `#/<named>`
export interface AddressBar { hash: string }              // `window.location` 是一个，测试给普通对象
export function current(bar: Readonly<AddressBar>): Option<View>;
export function go(bar: AddressBar, view: View): void;    // 写地址栏；hashchange 才动 store
export function buildingOf(address: Address): Address;    // 地址的第一段
export function roomOf(address: Address): string;
export function roomIn(building: Address, name: string): Option<Address>;

export function readRunId(raw: string): Option.Option<RunId>;  // 地址栏与转写文件名唯一的读法
```

- **设置面开着是一个地址**（7L）：`#/setup` 是设置面开在它自己选的组上，`#/setup/<组>` 开在那一组上，刷新与外部链接都回到同一组；组名不在 `SETUP_GROUPS` 里的片段答 `None`，与认不出的页一样不悄悄落到别处。`group` 缺席而不是一个「默认组」值，是因为「开在哪一组」在缺席时由设置面自己判（它上次画的组），地址栏不替它说。
- **展开面有可复制的定位（4-63）**：`#/talk/<地址>?call=<run>&at=<seq>` 是那段对话、右侧开着那次调用；`#/talk/<地址>?building=<楼>&path=<路径>[&version=<b3>]` 是那段对话、右侧开着那份文档的那一版（没有 `version` 是工作树此刻的文本）；`#/run/<id>/<透镜>` 是 run 页停在那个透镜；`#/welcome/<步>` 是上手指南展开那一步。查询串用 `URLSearchParams` 写读，只有对话页带它；键不多不少、每个值都过生成的 `Schema`（`RunId`、`Seq`、`Address`、`B3Hash`）才读得出，否则整条答 `None`，与认不出的页一样。市长的对话带项时写 `#/talk/hall/mayor?…`，不带时仍是 `#/`，所以每个 View 仍恰一种写法。地址的 `?` 在段里写成 `%3F`（`escaped`），所以第一个裸 `?` 就是查询串的起点。缺席而不是一个默认值，理由同 `group`：透镜缺席时由 run 页按 run 的状态判（D28），步缺席时由指南判当前那一步。
- **配对邀请读作远程组**：`/remote pair` 印出的链接是 `https://<主机>/#pair=<码>&city=<指纹>`（`crates/remote_access/Spec.lean` §8-6），这个片段不以 `#/` 开头，`fromFragment` 经 `core/remote/invitation.ts` 的 `isInvitation` 认出它，读作 `{ kind: "setup", group: "remote" }`。只读不写：远程组自己的写法仍是 `#/setup/remote`，配对一结束页面就把地址栏换成它，邀请里的码不留在历史里（4-57）。
- **路由点名一段 session**（7K、D44）：`talk` 的 `session` 缺省是这个房间的当前一段，有值是 `SessionLine::began` 为它的那一段，写作 `#/talk/<地址>:<seq>`；`:` 是地址文法拒绝的字符，而每一段经 `encodeURIComponent` 写出，所以分隔符不会与地址混淆。Mayor 的当前一段仍写作 `#/`。
- 写一种、读全部旧写法：`overview`／`city`／`live`／空片段都读作与 `hall/mayor` 的对话，`approvals` 读作对话（等人的事插在流里），`ledger`／`archive`／`recycle-bin` 读作 record 三透镜，`dashboard` 读作 cost，`settings` 读作 setup。
- **两个身份值都由 `client/src/wire.ts` 生成，且都带 pattern 精炼**：`Address` 是 `kernel::Address::parse` 的路径文法，`RunId` 只收连字符小写 uuid（两者都是 `client/src/wire.ts` 里的同名导出）。客户端不再自带文法：`core/run_id.ts` 的 `readRunId` 是 `Schema.decodeOption` 于生成的 `RunId`，`route.ts` 的片段读法与 `building/tree.svelte` 的转写文件名读法都调它，认不出答 `None`。**裸 32 位十六进制、`{…}`、`urn:uuid:` 与全大写由此都读不出**，与城收窄后的 `kernel::RunId::parse` 一致；地址栏与目录树的 `Address` 直接来自 wire.ts。
-/

/-! ## 9 工作流程

收键部件的状态机是本规格形式化的那一部分，各在自己的分部：`views/parts/` 的键表与焦点还原在 `client/spec/Views/Parts.lean`（§7、§7-1 至 §7-7、§7-10），每个复合部件的模型在 `client/spec/Views/Parts/` 下，外壳的控件在 `client/spec/Views/Workspace.lean`（§7-11），检视面的页签带在 `client/spec/Views/Inspect/Open.lean`，工作台的分隔线在 `client/spec/Core/Workbench.lean`，远程组的门开关与确认码输入框在 `client/spec/Views/Door.lean`。模型只保留键与焦点的次序，不规定一帧里先画什么：画法由视图决定，`cargo xtask render` 量它落在哪。
-/

/-! ## 10 实现逻辑

### §4 设计

- **4-1 两个 `typescript` 名，一条类型车道：`@typescript/native`（TS 7，Go 版）是检查器，`typescript`（TS 6）是 JS 编译器 API。** `typecheck` 跑 `svelte-check --tsconfig ./tsconfig.json --tsgo`：svelte-check 实测把 `.svelte` 与 `.ts` 同一车道查完（两类文件的错都报、`noUncheckedIndexedAccess` 等严格旗标都从 tsconfig 读到），`--tsgo` 车道用 TS 7 的检查器，同树实测比普通车道快约一倍，两万行级的客户端上差距只会更大。**别名必须叫 `@typescript/native`**（svelte-check 只认这个名字，`typescript-native` 无人读）；`typescript`（TS 6）仍有岗位——typescript-eslint 的类型感知 lint 要 JS 编译器 API，而 TS 7 的包只导出版本号，upstream 的报错也要求两个名字并存。**`--tsgo` 车道的已测缺陷是安装缺失时打印错误却退出 0**，损坏时仍绿的门不满足「绿 run 即证据」，所以 `typecheck` 不裸跑它：一层薄封装（`client/scripts/typecheck.ts`）先清掉 `.svelte-check/`（车道写盘的转译产物，清掉即无状态，目录进 `.gitignore`），再断言三件——退出码为 0、总结行是 0 errors and 0 warnings、输出里没有 setup 失败的那句 Error；封装把上游缺陷变成响亮的红。重开参数：上游把退出码修对，封装即可删。
- **4-2 `bun test` 必须带 `--conditions=browser`。** `svelte` 的 `.`／`svelte/store`／`svelte/reactivity` 各带 browser／worker／default 三个条件，缺省解析到 server 构建——**那不是不响应，而是假的 SUT**：`SvelteMap === Map` 为真、`mount` 是抛错桩（均实测），而假构建上的测试是全绿的。测试套件因此留一条构建保真断言（`SvelteMap !== Map` 且 `mount.length > 0`）把这个条件钉住；`bunfig.toml` 仍没有能改条件的键。否决「不带旗标跑」：无声测错对象比报错更糟。
- **4-3 hash 路由，不用 path 路由。** 片段不发给服务端，书签、后退、深链成立，且不动 `ClientAssets::lookup` 那道安全判定。
- **4-4 身份值只由生成的 `Schema` 产出，`as` 全库禁用。** `Address` 与 `RunId` 是 `wire.ts` 里带 pattern 的 brand，判合法只有一条路：`Schema.decodeOption`，非法值答 `None`，与 Rust 的 `Result` 同形（`core/run_id.ts` 是 run id 那条判定的唯一家）。`as` 全库禁用（`as const` 除外），所以「新类型」只能由构造器产出；`make` 是 brand 的构造器，供本文已经写对的字面量用（`MAYOR` 与夹具），它不查 pattern。
- **4-5 eslint 配置用 ESLint 自己的 `defineConfig()`，三条补充各有一个原因。** eslint-plugin-svelte 在 `defineConfig` 里定型通过，所以不需要 typescript-eslint 的 `tseslint.config` 定型桥。补充一：typescript-eslint 的 `eslint-recommended` 块（以类型检查器代管 `no-undef`／`no-unused-vars`）扩到 `**/*.svelte`——Svelte 的 script 就是 TS，在那里手写禁用是同一规则的第二个家。补充二：模板表达式无处写类型，`settings.svelte.ignoreWarnings` 只在模板里静默 `no-unsafe-assignment`／`no-unsafe-member-access`，script 里的同一规则照报。补充三：被 lint 的每个文件都必须在 tsconfig 工程里，故 `svelte.config.ts` 取 `.ts` 而非 `.js` 并进 `include`；`.svelte-check/`（svelte-check 写盘的生成物）进 `ignores` 与 `.gitignore`。否决「把配置文件排除在 typecheck 与 lint 之外」：那会让全库唯一不受 `as` 禁令保护的文件恰好是定义禁令的文件。
- **4-6 Effect 给值，不给运行时；线上的帧由生成的 `Schema` 读。** 客户端用 Effect 4 的三样东西，都是值：生成的 `Schema` 读每一帧（`core/frames.ts`，`Schema.decodeUnknownResult(ServerFrame)`），页面手写的几个 `Schema` 读城外来的 JSON（一段贴进来的 MCP 配置、`exec` 的参数）；`Option` 与 `Result` 是可缺与可败的值；`Effect.runSync` 只在 `core/belief.ts` 里包一次同步的折叠，取它的收尾（`ensuring`）；一次可能抛出的浏览器调用读成值用 `Result.try`（`core/rows.ts`）。热帧例外：`event` 帧（每条 ledger 记录一帧）与 `delta` 帧（每个 token 一帧）先走 `JSON.parse` 加窄校验，窄校验的每条规则都取自生成的 schema——事件种类集合取自 `EventKind` 的字面量，`RunId`／`B3Hash`／`Address` 直接跑各自字符串节点上的 `checks`；窄校验不收的帧仍交给 Effect，所以它只能让帧变快，不能放进 schema 拒绝的帧。理由：完整的 Schema 解码一帧的开销大半在解析器机器本身（`client/scripts/frame_cost.ts` 交错测两条路径的下限，读数记在 `tools/xtask/budgets.toml` 的 `client_frame_decode` 行）。这个读数不是测试：墙钟读数属于机器，一台满载的机器不是缺陷，`frames.test.ts` 只判热路径与 schema 读出同样的帧、拒绝同样的帧。**为 4.0 重新论证（D29）**：4.0 能多给的是 `Schedule`、`Effect.retry` 与纤程的取消，去处是链路的退避与问答的重试。链路的退避是 `core/link.ts` 里一张阶梯表加一个纯的 `step`，交出 `{ kind: "wait", ms }` 由 `socket.ts` 去等；问答不重试，没有认领的问题照常超时（D10）。用 `Schedule` 重写退避，就要让一个纤程带着时钟跑这台状态机，测试也要推一只测试时钟，而它今天按一次 `step` 一个断言就测完了；所以 4.0 不扩大 Effect 的边界。
- **4-7 首屏即对话。** `#/` ＝ 与 `hall/mayor` 的对话；同一房间的每次 dispatch 是一段线程；live 时 Enter 是 `steer`，冻结后 Enter 是新的 `dispatch { addr: room, session: null }`（`room_for` 对含 `/` 的地址不再开子房间）。等人的事以卡片插进对话流，不另开一页。
- **4-8 页面上常驻的按钮只在两处：左下三键与对话框。** 左下三键（`docs/frontend-method.md` §7E）是图层、信箱与设置；对话框（`docs/frontend-method.md` §7I）带发送与停止合一的硬币键，和它横线下的设置行。城、楼、记录、成本、登记簿、MCP 与性能这些页没有各自的常驻入口，由设置面里的设置树到达（7L）；Ctrl-K 与 `core/keys.ts` 的 `go.*` 键是去同一批地方的快路。页面其余部分只有内容自己的控件——一行的展开、一张卡的答复。理由：常驻入口每天被扫视一次，八个页面八个字形，是让人每天读八次他一周才去几次的东西；去这些页是一次查找，查找该有分组，而分组是一棵树说得出、一列平铺的字形说不出的（D19）。
- **4-9 两层可视化。** `#/city` 开在楼表上，SVG 画的城是一键之隔的第二个读法；`#/building/<addr>` 是左边的索引（计划、提交、改动、技能、沙箱）与目录树（`Query::Listing` 逐层）＋中间的正文（所选的一节，或文件原文 `Query::Document`）＋本楼一份文件开在右侧时的第三块（4-50）；`#/run/<id>` 有七个透镜——time、turns、monitor、prompt、context、changes、evidence，打开时停在哪一个由这次 run 的状态定（D28）。
- **4-10 页面上没有句子，但零数据的屏必须说下一步。** `lang.json` 全是标签，说明段不写。**动词一律拼成命令**：`city_stop` ＝ `/halt --all`、`bld_halt` ＝ `/halt {addr}`、`talk_stop`／`run_cancel` ＝ `/stop`，`release` 与 `halt` 同形；两语同一拼写。理由：人会从别的软件迁移用法，一条命令的拼写自己说明自己。**图例与空态提示不算说明段**：`city/bar.svelte` 的图例是五个字形唯一的名字，去掉它城就是一张没人读得懂的画；一个零数据的屏只画一个灰词时，人分不清这屏是空的还是坏的。因此**空态一律走 `parts/empty.svelte`**——一个形状、一句说缺什么的话、一个离开这个状态的动作，动作能省而那句话不能。这不放宽「不写说明段」：空态那句话说的是这一屏此刻没有什么，不是这一屏是干什么的。**对话页的空态是对话框本身**：空房间里对话框立在页面的竖直中线上，上方一行写收件人，框内有任何字符就隐藏，不重复房间地址，没有说明句；第一次发出后同一个元素沉到底（`docs/frontend-method.md` §7I）。说明句只出现在展开的面（信箱、设置面、右侧）与欢迎页。
- **4-11 性能纪律。** 帧按动画帧合并（`socket.ts` 的 `queue` ＋ `requestAnimationFrame`）；页面隐藏时浏览器不再调用动画帧，此时改用计时器排空（`visibilityState === "hidden"` 下用 `setTimeout(drain, 0)`），否则一条审批请求要等人切回标签页才到；`visibilitychange` 转为可见时立即排空，链路若在 backoff 就取消已排的尝试、立即重试一次，因为人此刻在看，这一级剩下的秒数只是一页空白。事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` 的 `PACE_MS`），stale-while-revalidate，动画只用 `transform`／`opacity`。
- **4-12 SVG 的规则：id 只有一个家。** Svelte 的模板解析器按命名空间处理 svg 子树，所以 `<a>` 在 SVG 里仍是 SVG 元素。规则只有一条：`<defs>` 只在最外层绘图组件里写，渐变／滤镜／裁剪的 id 在那里声明一次，引用者拿 id、不自己拼第二遍。城市插画（skyline／marks）是画不是图标，导航与动作类图标一律 `parts/glyph.svelte`（`docs/frontend-method.md` §4-34）。
- **4-13 composer 说出消息落点。** `core/doing.ts` 的 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`；硬币键发送面的名字与提示（`docs/frontend-method.md` §7I）是 `/dispatch`／`/steer`／`/steer · after the tool call`；三者发的都是 `/dispatch` 或 `/steer <text>`，`queued` 只是名字说出的落点，不是另一种送法（真正的排队送达——等 run 冻结后再送——要改 wire，不在本客户端的语法里）。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻。落点也画在线程里：run 在调用工具时，一枚 accent 楔形钉在在跑的那条工具行上；在想或在说时，钉在正在说的话或姿态行的末尾（refrain §3-5）。钉子重复硬币键名字里已经说出的落点，所以只给眼睛（`aria-hidden`），没有自己的提示。**零 wire 变更**：信息全在 belief 里。**抖动缓冲不做**：抖动多大是一个未测量的量，为一个未测量的量先建队列是这座城禁的那条。
- **4-14 从 diff 到 session 是一次路由跳转**，不新增命令帧、不加「继续」按钮。`dispatch{addr: room, session: null}` 与在该房间对话页按 Enter 是同一个动作，加按钮就是给一个已有机制起第二个名字。
- **4-15 提交列表按页问、按页存。** `core/asking.ts` 的 `COMMITS_PAGE = 40` 与 `commitsQuery(building, before)`——问题只有一种拼写，因为 `CommitsAnswer` 回带 `building` 与 `before` 而不带 `limit`，键若拼法不一，答案永远落不回槽里。每页是一个独立的问题：只有 `before: null` 的首页会因 `checkpoint_committed`／`pr_merged` 失效重问；旧页上界是已写下的 seq，且 `lineage` 从写提交的 run 向前走，后来的接替者改不了它。
- **4-16 转写结果落进输入框，不直接发出去。** 机器听错的那一句必须能改，否则它会花掉一次 run。没有为 `transcribe` 选过模型的城不画那个按钮：一个只可能答拒绝的控件，是一个没人该遇见的控件。
- **4-17 焦点环只有一个家，所以 `outline-none` 是删掉而不是换掉。** `theme/base.css` 的 base 层写着 `*:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px }`，而 utilities 层优先级更高：26 处 `outline-none` 把它吃掉，键盘用户在设置页看不见自己在哪。做法是把这 26 处全部删除、不补任何替代类——`:focus-visible` 本来就只在键盘取焦时匹配（文本框被点击时也匹配，因为它确实要收键盘输入，这是对的）。否决「改写成 `focus:outline-hidden focus-visible:outline-2`」：Tailwind v4 的 `outline-hidden` 设 `--tw-outline-style: none`，而 `outline-2` 展开成 `outline-style: var(--tw-outline-style); outline-width: 2px`，两条规则在 `:focus-visible` 时同时命中，算出来是 `outline-style: none`——那对组合会删掉它本想保住的那个环。**任何一处需要压制焦点环的地方写 `outline-hidden` 而不是 `outline-none`**：v4 把「画 2px 透明描边、在强制色模式下仍可见」这层语义改名到了 `outline-hidden`，照旧写 `outline-none` 会在 Windows 高对比下丢掉焦点环。闸门条件是 `client/src` 里 `outline-none` 出现 0 次。
- **4-18 `parts/tip.svelte` 的提示有两条定位路，调用方说关系。** `title` 对三种读者都失效：指针要悬停约一秒，键盘根本到不了，触屏永远不画。`Tip` 用 `:hover` 与 `:focus-within` 显示一个 `role="tooltip"` 的兄弟节点，`transition-delay: 300ms` 挡住指针扫过一行时的连环点亮，未显示时是 `display: none`——既不画也不被 `xtask render` 量到。**关系由调用方写**：控件自己有名字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`；组件不猜，因为只有调用点知道控件有没有可见文字。**定位两条路都要在**：`@supports (anchor-name: --a)` 内用 `position-area` 锚定并 `fixed`，脱开一切裁剪祖先；Safari 与 Firefox 今天不支持，落到对 wrapper 的 `absolute` 分支（`AGAINST_WRAPPER`）。只写前一条是静默失效：不支持 anchor 的引擎里弹层退回静态位置，可能溢出视口。锚点名按实例生成，经继承的自定义属性 `--tip-anchor` 传给提示节点，所以同一行上的两个提示各锚各的控件。
- **4-19 `Field` 管有标签的表单格，不管搜索框与命令框。** 23 处裸 `<input class="rounded-control bg-g2 …">` 里，供应方表单、模型表的两个上限格、键名值对与登录码改走 `Field`，因而一次拿到标签、说明、错误态、`aria-describedby` 与 `:user-invalid`（失焦后才红，不是每次按键）。命令面板、combobox 过滤框、composer、楼页目标框与记录搜索框**不改**：它们要 `ref`、`autofocus`、逐键的 `onKeyDown` 与自定义补全，塞进 `Field` 会把它变成十个透传参数的 `<input>` 壳子，正是 AGENTS.md 禁的那种壳。`Figure`（原 `providers.svelte`）删除，三个调优数字改 `Field kind="number" step=…`，键盘上下键因此能用；**不给 `ms` 后缀**——标签本身就是 Codex 的 `timeout_ms` 与 `stream_idle_timeout_ms`，再加一个后缀就是同一个单位的第二个家。
- **4-20 模态是平台的事。** `parts/dialog.svelte` 是原生 `<dialog>` 加 `showModal()`：top layer、焦点陷阱、Esc、其余页面 `inert`，四件都由引擎承担，文件里因此没有遮罩层、没有 keydown、没有取焦调用，也没有 z-index——top layer 之上排不进任何数字。取焦由文档顺序决定：平台取对话框里第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。Esc 到达时先 `preventDefault` 再回调 `onCancel`，否则默认行为绕过调用方关掉元素，`open` 还说着「开着」。开与合是同一条 `transition` 的两个读法，`display` 与 `overlay` 是离散属性，少了 `transition-discrete` 关门第一帧盒子就消失、连同它的淡出。**遮罩是 `::backdrop` 上的 `backdrop-filter: brightness()`，不是一层 `bg-g0/80`**：`::backdrop` 只在实现了「从原始元素继承」的引擎里读得到页面的颜色令牌，令牌解析不出时 `background-color` 回到初始值——一扇没有遮罩的模态，且没有任何一道闸会报出来；亮度滤镜不向平台要颜色，深浅两套灯光下都变暗，Tailwind 又把 `--tw-backdrop-*` 直接声明在 `::backdrop` 上，所以这条路不经过继承。
- **4-21 `views/parts/` 不写 z-index。** 定位过的盒子在未定位的盒子之后绘制：下拉、弹层、粘性表头与提示压住它们打开时盖住的那些行，这是绘制顺序本来就给的，不需要一个数字。给了数字反而要维护一张谁比谁大的表，而那张表没有家。`parts/kbd.svelte` 的快捷键表必须盖住左下三键，而三键的 `<nav>`（`views/edge.svelte`）是全客户端允许保留的那一个 `z-10`；表因此是 `showModal()` 打开的原生 `<dialog>`，站在 top layer，同样不写数字。`#/gallery` 里的那一份是 `seat="specimen"`：`<dialog open>` 留在所在折页的流里，不进 top layer，否则它会盖住这条路由上的每一个折页（见 4-23 的第二个参数）。
- **4-22 按钮的三态是一个 `data-state`。** `idle`／`loading`／`stopped` 由一处判定给出，属性本身、`aria-disabled`、`aria-busy` 与点击闸是它的四个读者。色调表只写静息态与静息态的 hover，`not-data-[state=idle]` 一条规则管住其余两态——「正在等回执」与「你按不了」对手的回答是同一句，不该有两套写法；hover 写进 `data-[state=idle]:` 里，因而一个不能按的控件在指针下不会亮起来假装这一按会落地。`background-color` 进了 transition 列表，hover 因此是到达而不是切换。
- **4-23 Popover API 今天进不了 `parts/popover.svelte`，两个参数卡着它。** 其一：`popover` 元素开着时在 top layer，包含块是视口，`absolute` 不再相对 composer 的 `<form>` 解算，而把弹层钉回 composer 的唯一机制是 CSS anchor positioning，本项目的第一浏览器（Firefox）没有——所以「不支持 anchor 时走今天的 `absolute` 分支」这条降级对一个 popover 不成立，它只在元素还留在流里时成立。其二：`#/gallery` 的两个弹层夹具靠一个 `relative` 祖先与两层 padding 把面板留在自己的 Case 里，元素一进 top layer 就逸出，`xtask render` 的「没有盒子画在容纳它的盒子之外」立刻红。两个参数任一移动都应重新论证本条；在那之前弹层是 `absolute`，Esc 与取回焦点由这个文件自己管。
- **4-24 后端已经答了的，客户端必须问，问到了必须画。** 一个城答得出而没人问的 `Query`，与一个上了 wire 而没有屏幕读的字段，是同一个缺陷的两半：它们让「这件事做完了」在两侧各有一个说法。三处落点：**其一**，城页顶栏的六个数字全部出自 `Query::Metrics` 一问——事件、在干活的 run、已收尾的 run、等人批的、排队的信号、回收站里的。`MetricsAnswer` 的第七个字段 `buildings` 故意不画：顶栏下面的天际线与旁边的楼列表本身就是楼的数目，再写一个数字就是同一个事实的第二个家。原先的状态徽标一并去掉——它说的 `runs_active` 就是那六个里的一个，而「城停了」由外壳的横幅在每一页说一次（`app.svelte`），顶栏只留那一个停/放的控件。**其二**，`Query::RegistryView` 得到 `views/registry.svelte` 一屏，四列（登记于、类别、所属楼、是什么），走 `parts/table.svelte` 因而每列可排序，默认最新在上。**其三**，run 页直接问 `Query::RunView` 而不再只靠流折出来的 belief：从别人发来的链接打开 `#/run/<id>` 的那一页没见过任何记录，`city_view` 又只列它还在列的 run，所以这条 run 属于哪个房间、是否已经结束，恰好在最需要的那一类 run 上是空的。**两个读法按 seq 判定**：流说的是此刻，摘要说的是城写下来的，两边都带账本位置，所以谁更新是一次比较而不是一次偏好。**其四**，`Query::Commit` 由楼页提交的父提交与提交列表头部的 oid 框问，`Query::CostOf` 由楼页计划行问（4-50）：两问上线已久而没有提问者，它们回答的事——这个提交是谁写的、这个节点花了多少——在页面上因此只能靠猜。
- **4-25 base URL 的形状只拼写一次。** `setup/providers.svelte` 的 `BASE_URL` 同时喂两个读者：框子自己的 `pattern`（浏览器在人还在填表时判，失焦后才红）与 `hostOf`（「看看」和「接上」两个控件的开关）。**`type="url"` 不是那条规则**——它收 `mailto:somebody` 和任何别的 scheme，于是一个这张表单会拒的值可以坐在一个浏览器称为合法的框里，而人是按下一个始终发灰、不说为什么的控件才知道的。形状是：scheme、ASCII 主机、可选端口、可选路径，之后什么都不许有；查询串拒掉，因为供应方陈述的是 API 的根，每个 face 自己在后面挂路径。**客户端只做最宽的判断，永不比城更严。** 归一化今天有调用者了：`accounting::worker::credentials::Entered::resolved` 是打字地址变成被调用地址的唯一一处，probe 与 attach 都经过它。`gateway::router::normalise` 的第三条规则把缺席的 scheme 读成 `https://`，运行这座城的机器地址读成 `http://`，所以 scheme 在这张表单里是可选的——一个要求写 scheme 的框会拒掉厂商文档印出来的 `api.openai.com/v1` 与 `127.0.0.1:11434`，而城收得下。这张表单因此只判「有没有一个能调用的主机」，scheme 由城补，路径由城按预设表补。

- **4-26 「这次调用算哪一类」由工具的注册回答，客户端不留表。** `kernel::ToolMeta` 为每个注册工具声明了 `effect`（`Read`／`Write`／`Egress`／`Connector`／`Spawn`／`Govern`／`Spend`）与 `render`（`Generic`／`Terminal`／`Diff { locations }`），两者随 `Call` 上 wire（`crates/wire/Spec.lean` §8-47）。对话页的折叠摘要按它们数「探索／写入／运行／其他」，检视面按它们选读法（4-45）；客户端没有一张以工具名为键的表，一个新注册的工具不需要页面改一行就落进正确的一类，城认不出的工具两个字段都缺席，读作「其他」。`render` 为 `signal` 或 `delegate` 的一行不读主体，读这一次调用的参数：`signal` 的 `send` 读作「发给 <to>：<text 开头>」，`pull` 读作「取信」，`delegate` 读作「交给 <room>：<task>」，参数读不出时退回主体（D85）。`parts/code.svelte` 的行号从 1 起算：检视面的文件视图读的是 `Query::Document` 给的文件开头，所以第 27 行就是文件的第 27 行；一段输出的头部（`Output.head`）不带它在文件里从第几行开始，画输出时的行号仍只是这段输出的行号。 **同一个代码视图的颜色按语法给出，高亮器不进首屏。** `parts/paint.ts` 是唯一知道语法存在的文件：它用 `@lezer/highlight` 与十种 `@lezer` 语法（cpp、css、go、java、javascript 及其 ts／jsx／tsx 变体、json、python、rust、yaml）把文本切成 `parts/code.ts` 的五种 `Ink`；它本身是懒加载块，每种语法又各是一块，所以只看 Rust 的页面只下载 Rust 的语法，一段代码都不显示的页面一块都不下载。文件扩展名与 Markdown 围栏的语言词查同一张表（`rs` 与 `rust` 不可能指向两种语法），表里没有的名字整段画成 plain；`toml`、`sh` 今天落在这里，因为 `@lezer` 没有这两种语法，而 `@codemirror/legacy-modes` 要连带 `@codemirror/language` 与编辑器状态进来。`parts/inked.svelte` 是文件视图与 Markdown 代码块共用的画法：文字第一帧就以 plain 画出，答案到了才上色，晚到的旧答案丢弃；块取不到时仍是 plain，因为文字不上色也值得读。选 lezer 不选 shiki（JavaScript 正则引擎）的参数：同样十种语言、1,000 行 TypeScript，在一台满载的 16 线程机器上 lezer 解析加上色约 70 ms，shiki 约 1,240 ms 且首次建立要 600 ms；十种语法压缩后两者都约 166 KB，而 lezer 按语言分块、给的是真正的语法树。1,000 行 ≤ 16 ms 与各块的 gzip 体积由测量那一轮读出并写进 budgets；当 lezer 在空载机器上仍超过 16 ms，或出现同体积下更快的语法高亮器时，重开这条选择。

  **Markdown 只有一个文法，它在城里（D14）。** 文档预览问 `Query::Preview { version, viewport }`，答复是 `documents::Preview` 的块树（`crates/wire/Spec.lean` §8-74）：页面把每一种块与行内画成元素，不经 `innerHTML`；`Unsupported` 画成等宽的原文（HTML、前置元数据、过深的嵌套），公式由 KaTeX 画、它不认的公式仍是原文（4-64a），`Preview::Unsupported` 说这一版不按 Markdown 读、留在源码视图，零个块是空文档的空态（4-10），三者是三种画法。代码块的 `info` 与文件扩展名查同一张语言表，经 `parts/inked.svelte` 上色。链接与图片的去处城已判过（`crates/documents/Spec.lean` D23），页面不再判协议，相对地址相对文档所在的目录。块的 `span` 是版本里的字节，与源码视图的位置对应经 `core/document_pos.ts` 换算。一个版本的预览不会过时，`staleness.ts` 把 `preview` 与 `range` 放在同一行。从 `Document` 打开的每一份 Markdown 文件都在内容库里，不论长短都能按版本预览（`crates/accounting/Spec.lean` §8-23）。**对话流问 `Query::Reply { text, state }`**（`crates/wire/Spec.lean` §8-75，D15）：还在说的回复由 `talk/saying.svelte` 以 `streaming` 问，结算的回复由 `views/prose.svelte` 以 `settled` 问；答复是同一棵块树 `documents::Laid`，结算的回复与同一段字节的预览读出的树相同，两处都经 `refrain/laid.svelte` 画，所以页面上的 Markdown 只有一种画法（4-53）。什么时候问、送哪一段、答复怎样接上，只在 `core/replying.ts` 判定（D31）：页面只送它还没画成块的那一段（第一问是整段），答复的块接在已画的块之后，答复的 `span.end` 是这一段里的字节，经 `core/document_pos.ts` 的 `utf16At` 换成这一段里的位置，之后的文字照旧画原文。还在说的一问只带那一段里以换行结束的完整行，因为收束点只落在完整行之后（`crates/documents/Spec.lean` D31）：没有新的完整行就不问，所以问的次数不超过到达的行数，一个字一个字到达的增量不各引出一问；一段回复同时只有一问在途，答复到了而又多了完整行才问下一次。还在说的答复止于收束点，它之后、问过的那几行城已经判过，不再为它们问；结算的答复止于文字末尾，止在之前只因为读满了一个窗口（`crates/documents/Spec.lean` D7），页面接着送其余的部分。收束点只进不退，所以已经画出的块不会被收回；块与块之间隔着空行的列表在流式期间一段一段画，结算的那一问从头读整段，把它读成一个列表（D31）。结算的回复在自己的答复到达之前，先画同一段文字流式时最后画出的块（`views/reply.svelte.ts` 的交接），所以回复结算的那一刻正在读的段落不跳回原文再跳回块。答 `Unavailable` 的文字（含 NUL）从还没画的地方起画成原文，这段回复不再问。块的区间对话流不读，因为对话流里的块不与任何源码对应。楼页的文件视图（`building/file.svelte`）手里有文件第一窗的文字，经 `views/prose.svelte` 同样按结算的回复问，读出的树与这些字节的预览相同（D30）。`staleness.ts` 把 `reply` 与 `preview` 放在同一行。`RUNTIME` 不为 Markdown 加任何一项。

- **4-27 右侧由账本决定要不要画，由网格决定画在哪。** `talk.svelte` 问的 `Query::Rounds` 与 `Thread` 问的是同一句，`core/asking.ts` 按内容合并，因此「现在显示的是哪个 run」只有一个答案；右侧开着什么不占路由，理由是一条新路由会让这个答案有第二处判定，而两处判定第一次分歧就发生在有人打开别人发来的链接时。地址栏可以带一个项的定位（§3-2、4-63）：它点名的是一项（一次调用、一份文档的一版），不是右侧此刻显示哪个 run；到达时经 `openCall`／`openDocument` 打开一次，此后开着什么仍只由 `inspect/open.svelte.ts` 说，所以定位不会与右侧的状态分歧，只会在人关掉那一项后描述一件过去打开过的事。**右侧是网格上的一块，不是对话列旁边挤出来的第二栏**：打开时它占第 8–12 栏（线 8 到 13：右边一份加中间一份的后两栏），上、右、下三边越过页边距贴到窗口边；对话让出那两栏、左移到第 2–7 栏（线 2 到 8），第 1 栏仍留给边缘键（`docs/frontend-method.md` §4-33、§7F，D72）；禅档与混合档都是这样，混合档的会话栏与地方栏一起收起，因为文字栏不压在对话下面；检阅档会话栏占 2 栏、所选的会话占 5 栏，地方栏收起（`docs/frontend-method.md` §7H）。**开合由两件事决定，各有一个家**：人打开的项（工具行、时间轴行、文件树里的文件）住在 `views/inspect/open.svelte.ts`，有一项就开着，不管偏好怎么说；没有项时由 `core/prefs.ts` 的 `panel` 决定要不要跟着眼前的 run 开（4-45），它是人的偏好，归宿是 `~/.sprawling/config.toml`，由那扇门后面的浏览器缓存记住，城的回答到了由 `adopt` 顶掉（4-29）。Accel-J 与检视面的收起键关掉两者：清空打开的项，偏好记为收起。

- **4-28 强度只有一个权威，客户端不留副本。** 城层 `CONFIG.toml` 的 `[model] effort` 经配置梯子冻结进每一次 run（`crates/city/src/config_layers.rs`），所以浏览器里不再存 `sprawling.effort` 这一行。**缺席不是 `"medium"`，缺席是不说**：帧里不写这个字段，城的文件回答；文件也没写时供应方回答，而 `Effort::None` 是「尽量不要想」，是另一件事。一次派活仍可为它开的那一场单独说一个档位（`Dispatch.effort`，城把它写进房间层，`crates/city/Spec.lean` §8-14），所以选择器只在 composer 上——那里的作用域与控件的位置一致。**欢迎页没有强度选择器**：它承诺的是「从此以后」，而浏览器里的设置控件是第二个权威的开始；设置面运行组的城层卡写的是城的 `CONFIG.toml` 本身（`ConfigureCity`，4-48），所以那一处不是。六句 `effort_note_*` 随选择器搬到 composer 的那一列，做每一格的 `hint`，人在决定的那一刻读到它。
- **4-29 偏好有两层，城赢。** 权威是人层 `~/.sprawling/config.toml`，浏览器存储是它前面的缓存：缓存只负责首帧不闪，城的回答一到就整条顶掉它。`core/prefs.ts` 独家拥有每一个存储键的拼写（`ROWS`）与读写，`core/rows.ts` 独家拥有对 `localStorage` 的触碰；`keys.ts` 问 `chord(action)`／`setChord`，外观屏问 `held().appearance`／`setAppearance`，网络屏问 `held().proxying`，没有第二处拼一个键名。`core/prefs_city.ts` 的 `keepWithCity(door, conn)` 是这扇门与连接唯一的接点，`main.ts` 开页时调它一次。

  **一进一出两个方向，形状因此不同。** 出城的方向是具名改动：`setLang`／`setWelcomed`／`setPanel`／`setAppearance`／`setProxying`／`setChord` 各发一条 `Command::PutPreferences { patch }`，与 `wire::PreferencePatch` 的变体一一对应；链路不在 `live` 时改动只留在这个浏览器，重连后城的回答为准。`setTier`、`setNotifying`、`setShowing` 不出城，因为城的记录没有这三个字段；`tier` 等 `PreferencePatch` 有了它那一臂再入城（`docs/frontend-method.md` §7H）。进城的方向是 `adopt(stated, chords)` 一整条：一次回答陈述每一个值，按字段贴回去会贴出半新半旧的一条；回答缺席的字段由浏览器此刻的值补上（`prefs_city.ts` 的 `adopted`），回答带来的快捷键覆写这个浏览器里同一动作的行。`keeper()` 说此刻是哪一层在保管（`"browser"`／`"city"`），第一次 `adopt` 之后答 `"city"`；设置页把它画出来（`setup/kept.svelte`），因为「清掉浏览器数据会不会丢」是人有权知道的事。

  **`readPreferences` 与 `writePreferences` 互为逆，这才使缓存是缓存**：城上次答的就是下一次首帧画的。
- **4-30 设置面的 `config.toml` 折叠区读城的文件，不自己拼。** 原先这一栏用 `[model_providers.<name>]` 拼出一段文字，而城自己的读法只认 `[model]`／`[sandbox]`／`[[mcp]]` 三节——一个事实在前端与后端各有一个家且已经分歧。现在折叠区里是城层 `CONFIG.toml` 本身（`.sprawling/CONFIG.toml`，经 `Query::Document` 读，`views/settings/files.ts` 是这个地址在页面上的唯一拼写），页面一行都不拼；文件还没写任何东西时是 `setup_toml_unread` 的空态。**一个值来自哪一层由拥有它的控件旁边说**：`Query::Config { addr }` 的 `ConfigAnswer` 逐值带层（`SettledEffort.from: ConfigLayer`），强度一节（`shared/effort.svelte`）与城层卡（`settings/city_layer.svelte`）都读它；折叠区只是校对工具，读的是城自己会读的那份字节，所以它不会与城说出两个版本。城写这份文件经 `rules_changed` 入账，`staleness.ts` 让这一问随之失效，所以保存之后折叠区与卡脚读到的是新的字节。

- **4-31 设置树的「接入」枝里有 MCP，skills 组是「列表 ＋ 只读源文」。** MCP 是设置树里一个指向 `#/mcp` 的页条目，与「账户与供应方」同在「接入」枝下，因为两者是同一类事（接什么进来）；MCP 页本身、它的三扇门与删除动作不在设置面里，选中这一项是离开设置面去那一页（7L）。skills 组由 `setup/skills.svelte` 承担三件：放技能的两个文件夹——城库 `.sprawling/library/` 与所选楼的楼架 `<楼>/.sprawling/skills/`，各带复制键，路径相对城的文件夹、一律用 `/` 写——楼列（`shared/buildings.svelte`，与 `#/mcp` 同一份）、那栋楼**三个书架**的清单（`Query::Skills`，`SkillShelf` 三臂：城库／楼架／外部架）与打开一条后的原文（`Query::Document`，走楼页那一个 `FileView`；外部架没有城内地址，那一行因此只报名不打开）。**这里没有编辑器，因为城的那扇门答「未建」**：library 在保留前缀下，居民可读不可放（`crates/city/src/library.rs`），`Command::PutShelved` 已在 wire 上而 `crates/accounting/src/worker/commanding/routing.rs` 以 `not_built` 拒它，一个存不下去的 `<textarea>` 会把「改了」说成两件事（写面未建）。

  **那两级文件夹路径今天由页面拼写，这是记下的欠账。** `Shelves` 画的是 `${城名}/.sprawling/library/`，而这条路径的家是 `kernel::layout`（`RESERVED_PREFIX` 与 `LIBRARY_DIR`）——客户端与城各拼一次，城改了前缀或目录名，这一行会静默指错地方。退休它需要一个跨 wire 的字段：`SkillsAnswer`（或它的邻居）带一个由 `CityLayout::library()` 相对城根算出的 `Address`，页面改读它。今天没有任何回答携带布局，所以这一处保留并在此记录。
- **4-32 一个状态药丸只有 `parts/badge.svelte` 一个画法。** `building/plan.svelte` 原先手画五种漆色（`done` 灰、`blocked` 实心 alert、`in_progress` 实心 accent、ready 的 `bg-g3`、其余无底色），那是同一件事的第二个家，且那串嵌套三元没有 `awaiting_approval` 的臂——等人批的一行被画成没人开工的一行。现在一个穷尽 `RoadmapStatus` 的 `weightOf(row)` 给出 `quiet`／`live`／`alert` 三档，`Badge` 画它。**两个状态共用一档是对的**：`ready` 与 `in_progress` 都是城在动，`blocked` 与 `awaiting_approval` 都是城停下来等人，而分辨它们的是词，不是颜色（7-1 的 badge 行）。代价是 done 不再比 not_started 更暗；这不是损失，因为那两个词本来就不同，而颜色按 7-1 只许重复词。

- **4-35 通知是三个座位、一个组件。** 一切拒绝与提示都由 `parts/notice.svelte` 画，座位由调用方给：**inline**（有归属表单的拒绝，紧贴出错的字段，随字段编辑清除）、**toast**（无归属页面的拒绝，以及页面对一次落空按键的回答（D16）；立在对话框之上、对话列的宽度之内，右侧打开时随对话列左移，至多三条，拒绝是 `role="alert"`，按键的回答是 `role="status"`，8 秒自动收起、悬停暂停；按键的回答不进抽屉）、**drawer**（信箱的「通知」段，4-49）。三个座位都画同一个 `parts/notice.svelte`，AxError 的三段式因此只有一个画法。**toast 什么时候出现由 `core/deferral.ts` 一处判定（D26）**：不等人就动不了的拒绝一到就出现，普通的拒绝一到只在信箱键上点一个标记，等人刚发出一句、框空满 5 s 或刚切回标签页时才出现；toast 不取焦点。**toast 的位置由对话框给**：对话页上它贴在对话框的上沿、宽度取对话框所在那一列的宽度，所以右侧打开、对话列左移时它跟着走（`views/refusal.svelte` 量对话框的 `<form>`，那一列的边由网格定，本文件不另拼一份）；没有对话框的页上它居中立在页底。通知段按天分组，每条是标题（`lang.json` 的 `err_<code>`，如 `err_E_CONFIG_INVALID`；页面自己等不到回答的问题是例外，`E_TIMEOUT` 的 subject 读得出一个 `Query` 时标题取 `ask_late_title`，写出那个问题的线上名字，因为一页同时问好几个问题，同一句「等得太久」看两遍的人分不出城漏答的是哪一个；规则在 `parts/notice_title.ts`）、时间、同 `code+subject` 的计数徽标，英文原句折叠进等宽详情。**动作由 `core/recovering.ts` 的一张表从 `code` 映射到动词**，toast 与通知段都读它——两个读者各写一张表就是同一个事实的两个家。动作有四种臂：命令（按自身拼写）、`reconnect`（让链路再试）、`settings`（去设置页选模型）、`reload`（`location.reload()`，取这座城构建时的客户端）。`E_WIRE_MISMATCH` 只给 `reload`：两端对线上格式意见不一，重连只会再撞上同一处分歧；草稿按地点存在 `localStorage`（`prefs.ts` 的 `draft`），重新载入后仍在。**动作只作用于 composer 所在的房间**（`views/notice_recovery.ts`）：`/new` 与 `/fork` 的房间取自地址栏——对话页的地址，或 run 页那个 run 的房间——从不取自拒绝的 subject，因为地址语法接受一句带空格和反引号的话，把 subject 当地址读会在一句错误原文上开出一栋楼；`/stop` 只在 subject 是 run id 时出现，停的也只是那个 run（run id 的语法窄到装不下一句话）；subject 只指房间的拒绝不给 `/stop`（`recoveryFor(error)` 按 subject 判），因为停房间是 `/halt`，一个永远跑不起来的控件不是动作；没有 composer 的页面上动作置灰（`act_no_target`）。**toast 在对话框之上**：人刚按下发送或停止，眼睛就在那里；右下角会压在右侧的编辑器上，左下是三键。**链路丢失不是通知，是页面所处的状况**：`views/link_banner.svelte` 用 `parts/banner.svelte` 画在「城已暂停」的同一位置（主区之上，两者同时成立时断线在上），写出第几次重连与 `unsent` 里等着的条数，唯一的动作是「现在重试」（取消阶梯的等待、立即重连）；横幅从 `backoff` 出现，到 `live` 或 `refused` 才撤，其间每次 `opening` 不闪掉。
- **4-35a 恢复动作可以打开一张预填表单，由人提交。** `core/recovering.ts` 的 `Recovery` 在 4-35 的四种臂之外还有 `form`——`{ kind: "form", label, words, room }`：`label` 是按钮上的 `lang.json` 键，`words` 是预填正文的键（槽 `{building}` 与 `{name}`），`room` 是 `"mayor"` 或 `"building"`，指表单开在哪个房间。按下它把填好的正文写进那个房间的草稿门（`PreferenceDoor.setDraft`，与欢迎页 `assign work`、`/fork` 回填同一扇门），再把地址栏移到那个房间；发出去的仍是人按下发送的那一次 `dispatch`。**不进审批队列，也不绕开门**：表单只替人写好字，city D1 一字不改。**`form` 臂只服务 `subject` 读作 `<楼地址>: <缺失的名字>` 的拒绝码**——地址文法不含 `:`，所以第一个冒号就是分界，冒号后去空白非空才算读到；读不出这两样时按钮置灰（`act_no_target`），不猜。逐码核对的结果：只有 `E_PLAN_MISSING` 的 `subject` 全程是这个形状（`<楼地址>: <常设目标>`，由设常设目标的那一处唯一抛出），它的行是 `{ kind: "form", label: "act_ask_plan", words: "form_ask_plan", room: "mayor" }`：按钮「让市长写计划」把「为 {building} 写一份计划，让它的就绪步骤朝向：{name}」填进市长的草稿。`E_CREDENTIAL_MISSING` 的 subject 依出处是 URL、provider 名、`secret:` 引用、MCP server 标签或一句话，都不带楼地址，所以它没有 `form` 行。
- **4-36 设置面是左边一棵树、右边一组；`config.toml` 折进每组的底部；每个设置项是一张卡。** 左边是设置树（7L，`views/settings/tree.svelte`），宽 `index`（232）；右边是这一组的正文（`views/setup.svelte`），一个 `<h2>`（设置面开在页面之上，页面自己的 `<h1>` 仍在下面）与一行说明（4-10 的例外：这行说的是这一组此刻管什么），两栏各自滚动，所以长组不会把树滚出手边。`config.toml` 是每组底部的折叠区，默认收起——**它是校对工具而不是设置项**（4-30）。**卡片语法**：标题（label 600）＋一句说明（note faint）＋控件＋卡脚（左：保存处在哪一步，或一句约束；右：需要按一下才保存的卡才有按钮）。**保存只认城的回执**（refrain 路线图 §3-14）：发出帧时卡脚写「保存中」；城再答一次、版本变了，才写「已保存」，并写出改动何时生效；城拒绝这次写入时写「未保存」与城给的出路，草稿留在框里；等过页面的耐心（15 s）仍没有回执时写「待核对」，之后任一个移动了版本的回答仍会把它判成已保存。这一步一步的判定在 `views/settings/saving.ts` 一处。一屏一个 `title` 级标题，其下只用 `label`。

- **4-39 「用我的编辑器打开」只列厂商自己的文档或源码读得懂 `文件:行` 链接的编辑器，编辑器与城的文件夹由这个浏览器保管。** 猜来的协议会静默失败——浏览器去找一个没人装的程序，或者把文件开在第一行——所以每一项都要有出处。VS Code 的 <https://code.visualstudio.com/docs/configure/command-line>（“Opening VS Code with URLs”一节）写明 `vscode://file/{full path to file}:line:column`，并写明 Insiders 版的前缀是 `vscode-insiders://`。这个处理器在 VS Code 源码 `src/vs/code/electron-main/app.ts` 的 `getWindowOpenableFromProtocolUrl` 里，按 URL 的 authority 是 `file` 来认，协议名是构建在 `product.json` 的 `urlProtocol` 里注册的那个；所以继承它的构建用同一形状、换自己的协议名：VSCodium 的 `prepare_vscode.sh` 把 `urlProtocol` 设为 `vscodium`；Cursor 的工作人员在论坛帖 <https://forum.cursor.com/t/remote-uri-opens-a-new-window-every-time/166093> 里称本地的 `cursor://file/...` 链接照常工作；Windsurf 没有公开这一处的文档，它注册 `windsurf` 协议，LocatorJS 等工具按同一形状生成 `windsurf://file/...:行:列`；这是七项里出处最弱的一项，一旦证明它不按这个形状打开就删掉。Zed 的文档（<https://zed.dev/docs/reference/cli>）只写了命令行的 `文件:行`，读 `zed://file` 的是源码 `crates/zed/src/zed/open_listener.rs`：去掉 `zed://file` 前缀、解码后按 `路径:行:列` 打开。**Zed 在 Windows 上要换一种写法**：去掉前缀后剩下 `/C:/...`，Windows 拒收这个名字（os error 123）；`//?/C:/...` 是同一路径的设备形式，Zed 能打开，其中 `?` 写成 `%3F`，免得浏览器把它读成查询串。这一条在 Windows 上的 Zed Preview 1.22 实测过。JetBrains 系不列：`idea://open?file=` 这类协议只在 macOS 上注册，Toolbox 的 `jetbrains://<工具>/navigate/reference` 要项目名而不是路径；Sublime Text 没有自带协议。哪天这些编辑器自己支持「文件:行」链接，在 `EDITORS` 加一项、在这里补上出处。选择控件是原生下拉列表而不是分段控件：七个选项放不进设置卡片里一条等宽的轨道。**保管在浏览器而不是城的 `config.toml`**：装了哪个编辑器、城在那台机器的哪个文件夹，是浏览器所在机器的事实，同一座城从另一台机器打开时这两个值不同；而线协议不带任何机器上的绝对路径（`wire.ts` 里 `Address` 之外不传路径）。代价是人要在设置里填一次城的文件夹。重开参数：线协议给页面城在浏览器所在机器上的根路径时，`cityFolder` 改读它并删掉这一行存储。**历史版本不冒充工作树**：编辑器打开的永远是工作树此刻的文件，所以一个读自过去版本的行号交给它，会落在另一行上而不报错。检视面的 diff 画的是两次检查点之间的那段，它的行号属于后一次检查点；页面先问 `Changes { base: 后一次检查点, head: null }`（`head` 缺席即工作树），这个文件在答里出现，就说明工作树已经不是检查点里的那份，`reachOf` 收到 `past`，画可复制的 `路径:行` 而不画链接；不在答里时收到 `current`。文件视图读的是 `Query::Document` 给的工作树此刻文本，永远是 `current`。没有可用链接时（没选编辑器、远程浏览器、文件夹没填）也画可复制的 `路径:行`，不声称已打开。**只有城里的路径得到链接，而这条判断是字面的**：路径按生成的 `Address` 语法判，文件夹须是绝对路径、不是 UNC 共享（`//host/share` 的主机不是文件夹，当成文件夹会把链接指到另一个文件）、不含 `.` 与 `..`；磁盘上指向城外的符号链接不跟随，因为浏览器看不到磁盘，城也不启动任何东西。
- **4-40 重连按水位续传，差距大才整页重问。** `socket.ts` 记下本页折过的最大 `seq`（全城水位）。welcome 的 `resume_from` 是服务端账本头的 `seq`：水位已知、头在水位之后、差距不超过 `RESUME_PAGES × GAP_PAGE`（两页，400 条）时，缺口 `水位+1..头` 排进 `gaps`，走 `HistoryRange` 逐页补拉，补回的每条记录经 `asking.invalidate` 只失效它影响的答案，`asking.resumed()` 只重发断线时在途的问题；水位未知、`resume_from` 缺席或差距超过两页时回退到快照，即 `asking.reconnected()` 把每个答案标旧重问。阈值两页（`RESUME_PAGES`）是估计，不是读数：两页以内补拉的字节估计少于把一页上所有被看着的问题重问一遍，超过两页时一次快照估计比逐页补拉更快到达当前状态。能定下它的读数是同一座城上一次快照重问的字节数与一页 `HistoryRange` 的字节数之比（§3-4）。welcome 的 `epoch`（创世记录的链哈希）与本页上次见到的不同时，水位属于另一份账本：`belief.forget()` 丢弃全部折叠、水位与缺口清空，再按快照重问，因为旧水位在新账本里指向的是别的记录。

- **4-41 斜杠动词是一张表，每条自带分组。** `core/slash.ts` 的 `SLASH` 是页面动词的唯一权威，`/` 菜单与 Ctrl-K palette 都读它；`Slash` 记录有五个字段：`spelling`、`grammar`、`about`、`section`（`actions`／`navigation`／`sessions`，类型 `Section`，类型检查强制每条填写）、`run`。palette 按 `section` 分组，不另立以拼写为键的表——另一张表会让改名的动词静默落进 actions。glossary 的 Halt 与 Cancel 是两个动词，页面随之分开拼写：`/stop` 无参，只对眼前的 run 发 `cancel`，眼前的 run 由 `core/in_front.ts` 一处判定（D16）；Accel-.（`run.stop`）是同一个动作的键，眼前没有 run 时不发任何帧，toast 座位出一条提示，说这一页没有在跑的 run，并写出停整座城的拼写 `/halt --all`；palette 里同一情形 `/stop` 与 `/steer` 置灰，理由是 `no_run_in_front`；`/halt [addr|--all]` 与 `/release [addr|--all]` 成对，带地址的作用于那栋楼，`--all` 与无参作用于整座城（`halt`／`release` 帧）；第一个词既不是 `--all` 也不是合法地址时不发任何帧、行留在输入框里待改，因为把打错的地址放大成整座城是这个人不可能想要的读法。Ctrl-K palette 同理：动词既没发出帧、没换页、也没写回一行时 palette 不关，行留着待改；只有做了事或清空了行才关。notice 上的 `/stop` 恢复同样只对 refusal 所指的 run 发 `cancel`，只指房间的 refusal 不给这个恢复（4-35），不升级为 `halt`。没有 `/clear`：在此地址开一个什么都不带的新 session 只拼作 `/new`（D44）。`/steer <text>` 的语法里没有 `--queued`。`/diff` 打开眼前 run（没有时取此房间最新的 run）的 run 页，changes 视图是那一页的一个 lens。**会话动词**（`core/slash_session.ts`）作用于主区那一段所在的房间（`hands.here`），无论主区是当前一段还是过去的一段：`/new [--carry]` 在那个房间开新的一段；`/compact` 等于 `/new --carry`，有 run 在跑时先发 `cancel`，belief 里它冻结后再发 `OpenSession { carry: handoff }`，没有单独的 `/handoff`（D46）；两者发出之后主区回到这个房间的当前一段，因为从一段过去的 session 发出时，新的一段才是人要去的地方。`/tag <name>`、`/untag <name>` 给主区那一段加、减一个标签，`<name>` 经 `readTag` 读，读不出时什么也不发、行留着待改（D45）。palette 里这四个动词在没有房间时说「需要一个房间」（`palette_needs_room`）。
- **4-42 run 页是时间透镜加顶部统计栏，不展开账本树。** 统计栏（`run/head.svelte`）是一张事实表：每个事实是网格里的一格，名字小字在上、值在下，排法同检阅档所选会话的仪表——结局、用时（开始时刻写整个 ISO 时刻，所以这一格占两栏）、token 入/出（缓存另记）、花费（回合数另记）、模型、来处（楼 / 房间）、由谁派来；每个数都由页面已经问到的 `RoundsAnswer` 求和，所以表与下面的透镜说不出两个数，花费也只在这一格写，透镜里不再写第二遍（`docs/frontend-method.md` §7D）。时间透镜（`run/river.svelte`，在跑的 run 打开时的页签）按份额画三条并行轨道——模型在说、工具在跑、在等人——下面是这次 run 的全部工具调用。**工具调用带自己的时钟**：`Call.called`／`Call.answered` 是账本写下调用与结果的时刻（`crates/wire/Spec.lean` §8-47），所以调过工具的回合在量到的地方切开——`Turn.t` 到第一次调用归模型，到最后一个结果归工具，其后归模型；调用列表每行写它答复的时刻（UTC 的 `HH:MM:SS.mmmZ`，`core/time.ts` 的 `isoTime`）与量到的毫秒用时，`Call.timing` 为 `unmeasured` 的调用两个时刻不是一次量出来的时长，不写用时，在跑的调用写「在跑」；**等人从量到的那一刻开始**：`Note::waiting.t` 是账本写下批准请求的时刻，`Note::waiting.answered` 是答复记下的时刻，等过人的回合在这两处切开——请求之前照调过工具的回合切法，请求到答复归人，答复之后按那之后发出的调用再切；答复时刻为 `null`（窗口外或读不回来）时请求之后整段归人，而不是按没人量过的长度再切一刀。**页面持有的盒子有上界**：`run/lanes.ts` 的 `columnsOf` 每列二分取样一次再按份额合并，一条轨道最多一列一个盒子；调用列表经 `windowOf` 只画视口内的行加两侧各 `OVERSCAN`（8）行，一万次调用的 run 与四十次的 run 在页面里是同一个量级。**透镜读的尺寸不回头喂自己**：轨道宽与调用列表高由同一个 `ResizeObserver` 报出，在下一帧才写进状态；行高是 `--spacing-control-sm` 这条令牌，未画行的占位也按这条令牌计，所以列表的总高只随调用数变，画出哪几行都不改它。备选是量画出的行再除以行数：那个高度又决定画哪几行，浏览器在同一轮报告里看到尺寸再变，就报 `ResizeObserver loop completed with undelivered notifications`，`#/gallery` 在 `xtask render` 里因此没法量。备选是逐段画、逐行画：一万个节点在 390 px 宽的轨道上大多窄于一个像素，却要付全部的布局与内存。统计栏的「模型」一格读 `Turn.model`（`crates/wire/Spec.lean` §8-47）：最后一个回合问的模型，run 中途换过模型时按出现顺序列出各个名字。统计栏的「由谁派来」一格读 `Opening.dispatched_by`（`crates/wire/Spec.lean` §8-48）：`person`、`city` 或派活居民的地址，原样以等宽字画出；为 `null`（旧账本、窗口外）时不画这一格。
- **4-44 线程的读数：每个数读自账本的两个时刻，缺一个就不画；在跑的数读页面的时钟，并从事件时刻重算。** 线程照模板排：消息头、正文、其下一条一条工具行（refrain §3-2、§3-3、§3-4；`talk/turn.svelte`）。
  **消息头**（`talk/head.svelte`）是一行注释字：谁 · 模型 · 强度 · 模式 · 时刻 · TTFT · t/s · 到达节奏。模型是会话的事实，只写在这一段的第一个消息头上（`Thread` 的 `opens`），之后只在某个回合换了模型时再写一次（`Turn.model`）。强度与完整 run 策略同是冻结事实，只写在第一个消息头上：首条消息头经 `talk/frozen.ts` 的 `firstHeadSaid` 读取写入限制、准入与落地，措辞由 `talk/policy.ts` 的 `policyFace` 读 `Opening.policy`，不借页面当前策略；仪表模型格仍经 `frozenSaid` 仅读强度与模式，策略限制留在边界与准入格；强度读 `Opening.effort`（`run_started.effort`，`crates/wire/Spec.lean` §8-79），模式读 `Opening.policy.mode`；没说强度的派活不写强度。「谁」见 4-59。时刻来自 `Turn.t`，写到秒，毫秒留给工具行。花费不画：专注与混合两档由人定不画花费（`docs/frontend-method.md` §7D），检阅档的对话栏不画线程。只调工具、不说话的回合没有消息头，它的工具行接在上一段话下面。**TTFT** ＝ `Turn.first_at − Turn.t`，只在 `Turn.timing` 为 `measured` 且差不为负时画（`talk/timing.ts` 的 `ttftOf`）。**t/s** ＝ 供应方报告的输出 token 数（`Turn.used.output`）÷（`Turn.returned − Turn.first_at`）秒（`timing.ts` 的 `tpsOf`）；`returned` 是答复那一行自己量下的时刻，没量过的行没有它。缺返回时刻、缺首字时刻、没有 `used`、输出数不为正或两刻之差不为正时不画，不写 0（refrain `docs/frontend-method.md` §3-3）。写到整数，单位 `t/s`。**到达节奏**（`talk/sparkline.svelte`，40 px）只画本页亲眼看着流进来的回复：正文每次变长记一个样本（页面时钟，已到字数），一帧合并的几个增量只算一个样本，所以它说的是节奏，不是逐 token 时序；按 run 收集，回合在回答里出现后按 `Turn.opened` 归档（`talk/arrivals.svelte.ts`，页面寿命内最多 200 条，最旧的先丢），没认出是哪个回合就丢掉；切成 20 段，每段画它占最忙那段的比例（`talk/rhythm.ts`）。从历史打开的回复没有节奏。
  **工具行**（`talk/call_line.svelte`）：`种类 主体 用时 结果`，网格四列（7ch、余下、9ch、自适应），整行是一个按钮。**种类读注册，不读工具名**（`talk/call_kind.ts`）：先读 `Call.render`（`terminal` → exec，`diff` → edit），再读 `Call.effect`（read、write、net、mcp、spawn、govern、spend、browser），两者都缺（工作台不认识的工具、键出现之前写下的行）时写工具名本身。**用时**：落账后是 `answered − called` 的毫秒，不到一秒写 `31 ms`，以上写 `3.412 s`；`Call.timing` 为 `unmeasured` 或两个时刻倒序时不画。在跑时由一个计时器重画：每个时钟一个（`ticker(now)`），每 100 ms 一次，没有在跑的行订阅或页面隐藏时停下，恢复可见时立即从时钟重算——读数始终是 `now − called`，不是累加的次数；不满一秒只画在跑的点，满一秒画十分之一秒（Nielsen 的 1 s）。完成的 ISO 时刻（在跑时是开始的）在这一行的提示里（`docs/frontend-method.md` §7D）。过 10 s 的行只在知道时写它在等什么：run 在等人时写「等你回答」，其余不编原因。失败写「失败」，用 alert。按下经 `openCall` 在右侧打开这次调用（`inspect/open.svelte.ts`）；右侧正显示的那一行 `aria-pressed` 并带 2 px accent 前缘条（`docs/frontend-method.md` §7B）。键见 7-11。
  **阅读位置**（`talk/scroller.svelte`）：人在末尾时跟随，离开末尾或线程里有选区时不动；离开末尾之后新到的块（带 `data-wear` 的每个回合与结局线）计数，计数是一个回到末尾的按钮，回到末尾即恢复跟随。字体载入、代码上色、Markdown 闭合改变的是已有文字的高度：不跟随时由引擎的滚动锚定保住眼前那段，跟随时视口本来就在末尾。**读痕滚动条**（`talk/wear.svelte`）是指针的滚动条：轨道上按 `runs/phase.ts` 的阶段色标出每块的位置（等人、调工具、只说话、结局），视口显示过的段落加深一档，拇指是当前视口；按下跳到那里，拖动跟随。滚轮、触屏与键盘仍由对话列本身承担，所以它对读屏器隐藏；窄窗口不画它，留系统的滚动条。测量合并到下一帧，一阵增量只读一次布局。
  **送达**（`talk/delivery.ts`、`talk/delivered.svelte`）：刚发出的话在发出的同一帧画在线程底部（带 `data-local-feedback`，≤ 100 ms），并写出它的状态。草稿是框里的字，执行中是 run 自己的线程，两者都不归这里；之间三态加一态：**held**（按下时链路不在，话在本页的队列里，`core/unsent.ts`）、**pending**（帧已发出，城此后什么也没写）、**accepted**（城在发出之后写了记录；线上的回执不带帧的身份，所以同 `talk/handing.ts` 一样按「发出之后有新记录」读）、**unknown**（等待时链路断了）。unknown 不重发：城若已接下，再发一次就是第二个 run；重连后城告诉页面的记录把它推到 accepted。发出之后到来的拒绝结束回显，拒绝画在回复本该在的地方。第一次发送就让空房间的对话框沉底，因为回显的字已经在那里。
  **播报**：流式正文不进实时区域，逐 token 念出来只是噪音；结局线与送达状态是 `role="status"`，各在事件发生时说一次。

- **4-45 检视面按调用：一次调用一点就在右侧打开它的完整结果，读法由调用自己的注册决定。** 右侧的状态只有一个家，`views/inspect/open.svelte.ts`：打开的项是一组页签，按打开的先后排；最后碰过的那一项在前，编辑器区与终端区各显示本区最后碰过的那一项，所以打开一条命令不会把它上面的文件盖掉。每个打开的项一直挂载到关掉为止，滚动位置、选区与 RefRain 的草稿因此在别的页签到前面时不丢（roadmap §3-14）；代价是一个页签一个视图，所以至多 `KEPT`（8）项，多开一项就关掉最久没碰的那一项。**没有人打开任何项时，右侧跟着眼前的 run**（`reading.ts` 的 `followingIn`）：编辑器区是它最后读过或改过的文件，终端区是它最后跑的命令，还在跑的命令也跟；选其中一个页签就把两项都收下，成为人自己持有的项。**读法**（`reading.ts` 的 `readingOf`，`views/inspect/called.svelte` 按它分派）：`render = terminal` 是终端；`render = diff` 是 diff；其余的调用按它在账本里留下的形状读——结果是一张存在内容库里的图（`{ image, width, height, media_type }`）就是截图，`effect = read` 且参数恰好是 `{ path, offset?, limit? }` 就是读文件（搜索也是只读，但它带 `pattern`，多出的键让解码不成立）——都不是的是 `printed`，在终端区画工具、主体与它答的话。**diff**（`inspect/diff.svelte`）问这次调用前后两个检查点之间这个文件的 `Hunks`（`views/checkpoints.ts` 的 `bracketOf`：调用之前最新的检查点，没有时是 run 开始的树；调用之后第一个检查点），所以 diff 覆盖调用所在的那一波，编辑器上方一行写出两个提交；之后还没有检查点时说「下一个检查点后」而不猜。行号栏照 7-2 把「路径:行」接进对话的草稿，被选的行带 2 px accent 左边条；画法是 `inspect/patch.svelte`，`changes.svelte` 打开的那一行也用它，一个 diff 只有一种画法。**终端**（`inspect/terminal.svelte`）的头两行是命令（exec 的命令行与 monitor 读的是同一个解析，`monitor/trace.ts` 的 `commandOf`），以及毫秒用时、退出码、完成时刻（`HH:MM:SS.mmmZ`，`datetime` 是完整的 ISO）与被裁的行数；在跑时读 `core/live_output.ts` 的尾巴，调用一有结局就只读账本的结果（`terminal.ts` 的 `printedOf`，`terminal.test.ts` 判）。`Output.pinned` 有定位时头的右端有「原文」，经 `Query::Content { locator }` 读，不以 `cut > 0` 为条件，因为 sieve 可以不裁一行而缩短内容；没有定位而有裁剪时头里写「没有留下原文」，不把头部称为全文。**文件**（`inspect/file.svelte`）经 `Query::Document` 读工作树此刻的文本，交给 `parts/code.svelte` 只读画出，滚到调用读的那一行（`offset + 1`）并标出；那一行在城发来的开头之外时说出来，不标在别的行上。**截图**（`inspect/shot.svelte`）：图在内容库里的定位（`Picture.image`）经 `core/document_bytes.ts` 的 `storedObject` 读成对象的地址，字节经 `Query::Bytes` 逐窗取回（4-61），拼成一张 `blob:` 地址的图，按原图比例放进这一区；取回之前画同比例的框，取不回（城不再有这个对象、定位不是整个对象）时框里说出来——空白会读成一张什么都没有的图。框上一行写尺寸与格式，框下写定位。**到编辑器的路**按 4-39 的 `reachOf`：diff 的行属于检查点，工作树已离开它时只给可复制的「路径:行」。键见 7-11 的检视面几行；Esc 关上检视面，焦点回到打开它的那个控件（7-7）。
- **4-46 RefRain 编辑的是一个版本：读整份、改在浏览器、存成那一版的字节区间。** 右侧的文档项画 `views/refrain/refrain.svelte`（7N），它问 `Query::Document { at }`，`Coverage::Head` 的版本再按版本问 `Query::Range` 逐窗接齐（`core/document_windows.ts`）；接齐之前编辑器只读，因为对半份文本的改动会把另一半说成删掉了。**超过 `EDITABLE_BYTES_MAX`（4 MiB）的版本只给第一窗、只读，并说出这是第一窗、全文多大**：附录 D 要求超能力时明确降级、不静默截断保存，而一窗一个往返，4 MiB 是六十四个往返，再大的文件不是人在这一栏里逐字改的稿子；重开参数是 A11 在固定语料上量出的首个可读视口与输入至绘制时间。
  **坐标只在 `core/document_pos.ts` 换算。** 编辑器（CodeMirror 6，D23）的位置是 UTF-16 码元，而且它把 `\r\n`、`\r`、`\n` 都读成一个换行；线上的位置是版本里的字节（`crates/documents/Spec.lean` D2）。所以编辑器拿到的是去掉开头字节顺序标记、换行折成 `\n` 的文本，保存时 `textEdits` 把改动集（基线到此刻，`ChangeSet` 的 `iterChanges`）换成基线那一版的字节区间，插入的换行写成这一版第一个换行的写法。**被否决的**：让编辑器只按 `\n` 分行、把 `\r` 留在行里——`End` 键会停在 `\r` 之后，在那里打字就把字写进 `\r` 与 `\n` 之间；把标记交给编辑器——全选删除会删掉标记，城按 D11 拒绝这次保存（换了编码）。两处折叠因此都在换算里还原，没碰过的字节一个不动（A1）。
  **草稿是相对基线的改动，不是全文。** `core/document_save.ts` 把 `{ version, changes }` 写进 `prefs.ts` 的草稿门（地点 `document:<地址>`），每次改动后 300 ms 写一次：改动集与文件多大无关，全文却会在一份几 MB 的文件上撞到浏览器存储的配额。重开时草稿的版本就是城此刻的版本，改动照原样恢复；不是时进冲突态，草稿留着。**保存**是 `PutRange { doc, baseline, edits, idem }`；回执只认带着这个 `idem` 的 `document_written` 行，页面在保存途中问最新一页账本（`RECEIPT_QUERY`），因为账本行不经视图的门，而 `history` 本来就随每一条记录重问（`staleness.ts`）。拒绝不带 `idem`（§8-2），所以保存途中动作是 `save a document` 的拒绝归这次保存；同一页同时在存两份文档、其中一份被拒时，另一份会误读成被拒，草稿仍在，回执晚到仍把它改成已保存，这是记下的代价。**冲突**不自动解决：页面给出对照（草稿对城此刻的版本）、移到现版（把基线到草稿的改动经 `@codemirror/merge` 的 `diff` 求出的「基线到现版」改动集映射过去，结果仍是草稿、不自动保存，人看过对照再存）与丢弃草稿三个动作；附录 E 说的「重读与三方比较」就是前两个。
  **版本读城的列表**（4-61）：`Query::Versions { at }` 答这份文档的各版，新的在前；城读到或写下的每一版都在内容库里（`crates/wire/Spec.lean` §8-69），所以任意两版都能按版本经 `Query::Range` 接齐全文再比较，本页自己拿过全文的那几版（打开的、自己存下的、城换掉的）直接用手里的文本，不再问。
- **4-47 世界层跟随工作区，工作台只读页面已经在问的回答。** 三栏要的每一个数都出自一个已有的问题：会话栏读 `belief.rooms`，所选会话读它那个 run 的 `Query::Rounds`（与对话问的是同一句，`asking` 按内容合并）、端点表、`Query::Config` 的第二级提醒与 `CostAnswer`，地方栏读 `commitsQuery` 的第一页、`city_view` 与逐层的 `Query::Listing`；所以打开检阅档不让城多答一种问题，一个数在工作台与它的老家（环、成本页、楼页）之间也说不出两个值。**提交的范围由房间定**（`views/world/chosen.svelte.ts` 的 `commitsIn`）：市长的房间是城自己的地方，旁边是全城的提交与城的天际线；楼里的房间旁边是这栋楼的提交与文件——所选会话一栏与地方栏都问这一句，检查点的时刻与父提交因此和提交栏读同一页。**摆放是栏线而不是类名**：`workspace.svelte` 的 `layoutOf(tier, right, bench)` 答每块区域站在哪两条栏线之间（`Lines`），由内联的 `grid-column` 写出；Tailwind 只认源码里写死的类名，而人拖出来的宽度是运行时的数，写成类名就得把全部组合预先抄一遍。外壳只有一栏时（4-52）不写栏线，区域按各自的 `narrow:` 类占满一栏。
- **4-48 设置面补齐后端已经答得出的设置，答不出的留在命令行并写明为什么。** 「你与主 Agent」组（`settings/you.svelte`，refrain 路线图 §3-13）是两张各自保存的卡：用户 ID、导入来源与「关于你」写进 `PREFERENCES.md` 的身份区与正文，主 Agent 的名字写进 `MAYOR.md` 的身份区；两张卡都从 `Query::Identity` 的回答开始，经 `PutIdentity` 以回答给出的那份原文为 `base` 保存，所以旧表单写不过较新的文件（城以 `E_VERSION_CONFLICT` 拒绝，草稿留着，再存一次就是对现在的文件写）；身份区读不出时回答是 `unreadable`，两张卡让位，页面写出哪份文件第几行、为什么，而不画默认名——画默认名会让下一次保存盖掉人写错的那一行。名字在新会话生效，卡脚在已保存之后说这一句。**GitHub 导入是一次显式的读**：按下才问 `Query::GithubLogin`，在运行城的那台机器上执行，卡上先说清这一点；读到的登录名与主机是候选，填进框里，由这张卡的保存使它成为设置；`gh` 不在、没登录、主机不认识、失败或卡住各有一句，框里的字不动，手填始终可用。**城层配置**（`settings/city_layer.svelte`）经 `ConfigureCity` 一次写一个事实：常设强度（`[model] effort`）与是否保持 prompt 缓存不过期（`keep_warm`）；强度的现值读 `Query::Config` 对 hall 的回答（`from` 是 `city` 时才算城层的值），`keep_warm` 不在任何回答里，所以它的控件起初什么都不选，现值在组底部的 `CONFIG.toml` 里读。4-28 说设置页没有强度选择器，是因为当时没有写城层强度的命令帧；`ConfigureCity` 写的就是城的 `CONFIG.toml` 本身，所以这里的选择器不是第二个权威。**规则组**（`settings/rules.svelte`）用一个原生 `<select>` 选楼（`buildingsOf` 的次序，D78），经 `PutRules` 整份写一栋楼的 `RULES.toml`，城先读过再落盘，`base` 守住 Mayor 的并发写；页面不写也不查 TOML。远程门上这扇门是 LocalOnly，城的拒绝就是页面读到的。**自动化组**（`settings/automation.svelte`）只读：`Query::Automation` 答出 `SCHEDULE.toml` 的任务与 `WATCH.toml` 的来源，两份文件由人在城的根目录手改，这里不画表单，因为表单会成为城独自解析的文法的第二个写者。**模型表**的「输入」一列（X6）让人说出一个模型收什么输入：`text` 或 `text_image`，经 `SelectModel.input` 送到城，是 `gateway::accepted_input` 的第一档；不说是「由城判断」，供应方自己列出的输入种类在选择下面作为证据。**开发者组**（高级）里一张说明卡（`settings/admission.svelte`，refrain 路线图 Q6）解释三个分开选、可组合的值：写入限制（`full`／`create`）回答能改什么，准入要求（`standing`／`tested`／`contract_kept`／`double_validated`）回答合入前要带什么证据——选了要求不等于有了证据，楼自己的校验照跑——落地策略（`ordinary`／`experiment`）回答改动是否进入项目；它是说明不是控件，因为三者每次派活时选，城把选择记在它开的 run 上，常设控件会成为第二个决定处。**`adopt`、`export`、`restore` 只在命令行**：`adopt` 把城所在机器上一个已有的目录收进城，`export` 把整座城打包写到一个目录，两者都要一个城外的机器路径，而线协议除 `Address` 之外不传路径（4-39），一个可能在远程的浏览器替主机点名目录，是线协议有意没有的一种触达；`restore` 在另一台机器上把包解成一座城，那时还没有一座城可以连。重开参数：线协议给出城外路径的一种受控拼法时，`adopt` 与 `export` 可以进诊断枝。`[core] priority`（`PreferencePatch.core_priority`）可写而 `PreferencesAnswer` 不答它，所以设置面不画这个开关：一个读不到现值的开关只能猜；它进回答时加在高级组。

- **4-49 信箱是一栏四段，按「需要你」排序；它的键把「需要你」「有新消息」与链路分成三个记号。** 信箱键与 Accel-B 从窗口左缘推出一栏 transient 面（`views/mailbox/`），画在三个边缘键之下（`client/spec/Views/Workspace.lean` D60）；点外面与 Esc 关闭、焦点回到信箱键由 `layer.ts` 判；手机上它是一整屏、从左进来的 sheet，返回在左上。面里自上而下是：**待决**——一个门在等人亲自动手（`E_APPROVAL_PENDING`）与居民提的设计问题（`ApprovalItem`），两者是 7C 的请决定卡，其后是停住工作的故障（`core/deferral.ts` 判为 `needs_you` 的其余拒绝），画成带出路的通知；**在跑**——每个房间最新的、没冻结的 run，读 `belief.live`，一行是阶段的字形与词、跑了多久、房间与任务，点开是那个房间的对话；**最近**——本页见过的每个房间各问一次 `Query::Sessions`，按每段最后一行的时刻合并，新的在前，只挂载视口内的行（行高是 `control` 令牌，`run/lanes.ts` 的 `windowOf` 定窗），每行右端常显「从最后一轮分叉」（4-14：继续一段会话就是分叉，不加恢复命令），母 run 是这段会话里最后一个 run；那个 run 早于本页持有的范围时控件仍在、按不动并说为什么；**通知**——原抽屉（4-35）里普通的拒绝，按天分组。一条拒绝只在待决与通知之一出现，不两处画。**键的三个记号互不借用**：数字只数需要人的事（设计问题、未读的 `needs_you` 拒绝，加上全城开着的提案卡，4-55）；只有普通的未读拒绝时是一个不带数的点；链路不在 `live` 时键脚有一道横杠（与 `core/mark.ts` 在标签页上给「没被告知」的形状相同），连接中会呼吸，被拒是 alert 色，它的词进键的可读名字。打开信箱就是读过：未读一并标为已读。面里的段只在开着时挂载，所以关着的信箱只问一种事：全城开着的提案卡（`Query::OpenProposals`），因为键上的数要数它们（4-55）。键表在 7-11。
- **4-50 城、楼、登记簿、成本、机器与桌面六屏站在外壳的栏线上；楼页的 git 面说出自己是什么，并给出口。**
  **其一，栏线。** 这些屏在 `<main>` 里，`<main>` 占外壳第 2–11 栏（线 2 到 12）、自身没有内边距，与对话一样以屏幕中线对称：第 1 栏留给左下三键，第 12 栏与它对称；`parts/page.svelte` 的框也不加左右内边距，所以页头、它下面那条 1 px 的线与正文的左缘就是第 2 栏的栏线（`docs/frontend-method.md` §4-33）。页面要按外壳的白银线分栏时写 `silver-columns`：中栏恰为外壳中间一份的宽，两侧均分其余；页面居中，所以中栏两缘就是外壳的两条白银线（D24）。MCP 页三栏为楼列（左）、服务器与添加表单（中，即对话所在的一份）、Composio 与桌面（右），容器宽 ≥ 64rem 时排成 `silver-columns`，否则上下叠放；上手指南（welcome）是一栏，≥ 64rem 时站在中间一份，步骤行、分隔线与步骤里的字段共用这一栏的两缘，说明文字不另设 `max-w-measure`。外壳的栏不等宽，所以页内的 `grid-cols-11` 不再与外壳栏线重合；城页与楼页仍用自己的 `grid-cols-11 gap-x-gutter`，只在自己内部对齐，窄于 768 px 时一栏（`narrow:`）。备选是 `subgrid`，它要 `<main>` 本身是网格，而 `<main>` 是每一页共用的容器（`views/pages.svelte`），改它会改每一页的排法，不归这六屏决定。各屏的分栏：城页的楼表或城画占第 1–8 栏，所选楼的面板第 9–11 栏，没选时楼表占满；楼页的索引与目录树第 1–3 栏，正文第 4–11 栏，右侧开着本楼一份文件时正文第 4–7 栏、右侧第 8–11 栏；成本页五个切面各取一格，格宽 `grid-fit`；登记簿是满宽的表。**字号**：一屏一个 `title`（页名），区块名是 `text-note text-text-faint`（与世界层各栏的栏名同一写法），行是 `text-note`，数字带 `figure`；层级只靠字号、字重与灰阶（D51）。
  **其二，提交行说明自己（UC2）。** 一行是短 oid、提交说明的第一行（`CommitAnswer.message`；为 `null` 的是这个字段上线之前写下的提交，行上说「没有说明」而不是留空）、写它的房间与 ISO 时刻（`toISOString` 截到秒）。展开后是一张事实单：run（链到 run 页）、session（链到那个房间的对话）、模型与强度、这次 run 的花费、接替链（`lineage`，glossary 的 succession）、父提交，以及复制 oid——回执等 `clipboard.writeText` 兑现才出现，被拒时说没复制上。事实单之下是这次提交改了的文件（`views/changes.svelte`）。父提交是按钮：它在已读的页里就展开那一行，不在就问 `Query::Commit { oid }` 并在原处画出谁写的它（UG 的 Commit 页，即 CLI `whose` 的那一问）；提交列表的头部另有一个 oid 框，粘入一个 oid 按 Enter 问同一句。
  **其三，改动行有出口（UC3）。** 楼页的「改动」列出上一个检查点之后的工作树（`Query::GitStatus`），一行按下就在右侧打开这个文件的工作树版本：`openDocument({ building, path, version: null })`，即 `views/inspect/open.svelte.ts` 的那扇门。楼页因此也有右侧：有打开的项时第 8–11 栏画 `views/right.svelte`，与对话页是同一份画法、同一条页签带、同一组键（`docs/frontend-method.md` §7F、4-45），收起键与 Esc 关上它；没有项时不画，楼页上没有眼前的 run 可跟（D27）。
  **其四，从检查点取回一个文件（UC8）。** 「改动」的每一行与一次提交下的每个文件各有一个「取回」，发 `Command::RestoreFile { at: <楼>/<路径>, point }`：改动行的 point 是上一个检查点，提交下的文件取这次提交。它改写工作树里的那个文件，而那时的内容不在任何检查点里，所以发帧之前经 `parts/dialog.svelte` 问一次（D1 那一族）。城在这栋楼有 run 在干活时拒绝（`E_BUSY`，恢复句说先停那个 run），拒绝走 toast。取回写下 `file_restored`，`core/staleness.ts` 让 `git_status`、`document` 与 `listing` 随它重问：否则取回之后「改动」仍列着那个文件，人会以为没有取回。
  **其五，历史的限度说出来（UC7a）。** 页面只认得热视图里的 run：活跃的，加上全城最近冻结的 `RECENT_FROZEN` 个。房间目录数列表里的 `<run>.jsonl` 转写，其中 belief 不认得的 run 不再被说成「这个房间还没有 run」，而是一行「更早的 run，不在此页 · n」，其下每个链到它的 run 页（run 页按 `Query::RunView` 自己问，4-24）。提交按页问（4-15），列表末尾说这是这栋楼的第一个提交，或说正在问下一页。
  **其六，计划的节点答它花了多少（UG 的 CostOf 页）。** 计划行可以展开，展开时问 `Query::CostOf { node }`，画这个节点的花费与认领过它的每个 run。`NodeId` 只是计划表第一列的编号，城回答时不分楼（`accounting::views::cost_of` 按编号汇总），所以页面只画 belief 认得、地址在本楼之内的 run，这一格的数字是它们的和；其余的 run 数成一句「另有 n 个 run 认领过同号的节点，在别的楼或早于此页」。
  **其七，沙箱有控件（UG，`ConfigureBuilding.sandbox`）。** 楼页索引的「沙箱」一节是一张卡：shell 开关（分段控件）、执行边界（原生 `<select>`，Tab 进入与离开，方向键和展开／关闭沿用平台行为，选择后焦点留在控件）、燃料、可读的挂载（每行一个地址）、透传的环境变量名与可信的连接器（各每行一个）；选择 container 时同卡输入本地 immutable image ID、非零 user、millicpu CPU、memory bytes 与 pids，保存时保留 interpreter 与各列表，发 `core/commands.ts` 的 `configureSandbox(addr, limits)`。它只填 `sandbox`，其余三面写 `null`，因为 `null` 在城那边是「这一面不动」（`assembly/commanding/configure.rs`）。沙箱整值写入，因为城整值解析它（`kernel::config::SandboxLimits`：一层说到沙箱就说全部）。`BuildingAnswer.sandbox` 为 `null` 时这栋楼没有自己的沙箱、由城那一层管，卡说这一点，燃料框空着且必填：默认燃料是 `kernel::consts_policy::SANDBOX_FUEL_DEFAULT`，线上没有它，页面不抄。名字的语法（`EnvVarName`、`ServerLabel`、保留子树不可挂载）归城，拒绝原样走 toast。城的回答回来之前（`building_configured` 让 `building_view` 重问）卡不说「已保存」。
- **4-51 记录、run、性能、MCP 与上手页是网格上的一张纸：分区靠 1 px 的线与小号的名字，不靠抬起的卡。** 这五页各自站在页框（`parts/page.svelte`）之下：一节是一条线、线下一行灰色的 `text-note` 名字，再是内容；行是按列对齐的网格，时刻、序号与数字各占一个等宽的列，数字右对齐，所以一列数字读成一列而不是一串句子。窄窗口的判断问容器而不是窗口（`@container`），因为同一页在画廊的 390 px 样本里与在手机上是同一个宽度问题。**记录**是三种读法：时间线、归档、回收站。时间线把账本记录与进程日志排在同一根轴上——账本位置（`seq`）：日志行带着它写下时账本的位置，所以两者不必比较两台机器的时钟；同一位置上日志行在记录之上，因为它写在那条记录之后。轴头每天写一次 UTC 日期，每行写到毫秒的时刻、位置、发生了什么与在哪；账本行可展开成写下时的原样。日志从一个透镜改成时间线的来源筛选（账本与日志、账本、日志），`#/record/log` 仍打开时间线并只留日志，选来源时地址栏随之移动，所以 `core/route.ts` 的四个记录透镜一个不改。两种来源的保存方式不同，页面照实说：账本按页读，日志是这一页开着以来收到的窗口，读较早的一页账本时只画那一页记录写下期间的日志行（`record/timeline.ts`）。**run 页**的事实表、调用行与打开时的透镜见 4-42、D28。**性能页**一行一个计数：名字、最新读数（右对齐在数字列上）、曲线到行尾，第一条曲线量出每条曲线有多宽；窄于 40rem 时曲线自占一行。**MCP 页**三栏：楼、服务器与添加服务器的三扇门、Composio 与桌面；窄于 48rem 时楼列在上，窄于 64rem 时右栏落到下面。**run 板**是 APG Tree：j／k 与 ↑／↓ 在行间走，h／l 与 ←／→ 收起与展开，Enter 或点击打开这一行所指的页——楼的行开楼页、房间的行开它的对话、run 的行开 run 页（`runs/lineage.ts` 的 `viewOf`）；行底下的键提示写着这三组键。**上手页**是 refrain §3-15 的五步清单，见 7G。
- **4-52 一栏的外壳：对话占满宽，世界层与右侧各是一张全屏的面，每张面是浏览器历史里的一格；外壳站在人看得见的那块视口上。** 何时只有一栏由 `theme/surface.css` 的 `frame` 一处决定（`narrow` 变体，问的是名为 `shell` 的容器，D30），它同时写下 `--shell-columns`，需要知道的脚本读这个值（`views/shared/frame.ts` 的 `watchColumns`），768 只写一次。
  **其一，一栏里三档只有两种画法。** 一栏的旁边没有地方，所以专注与混合都只画对话，检阅是世界层的面、对话在面下当它的带（与宽屏的检阅带是同一个元素：草稿、选区、输入法的组合都不重建，仍能发送与 steer）。图层键在一栏上打开或关上世界层的面，不改存下的 `tier`：档是宽屏上的排法，默认是混合，若一栏照偏好画，手机一打开看到的是世界层而不是对话。按住照旧是临时看一眼，看的是那张面；键下的刻度标出此刻画的是专注还是检阅。
  **其二，世界层的面**从左进来（`.sheet`，`duration-page` 的 `--ease-arrive`），返回键在左上（44 px 见方，`world_back`），右边是 APG Tabs 的页签，按人排的栏序一次显示一栏；先显示地方栏，因为在手机上打开世界层多半为了 git 图与文件（refrain §3-2 的手机草图）。**右侧的面**只在有人打开一项时出现（`openCall`、`openDocument`），偏好 `panel` 在一栏上不打开它：一张盖住整页的面不该因为 run 有了产出就自己弹出来。它从右进来，盖住对话与三键，关闭键在右上（页签带右端那一个）；一栏上工作区伸到外壳的左右与下边，面盖满工作区，所以它站在 frame 留出的安全区之内，横幅（断线、城已暂停）仍在它上方。两张面进来有动画、离开没有，理由同设置面（7L）：面在它的历史格被离开时撤下，多停一刻的面会替一个已经不在的状态接住点击。
  **其三，一张面是一格历史**（`views/sheets.svelte.ts`）。开一张面压入一格，格的 state 的 `sheets` 自底向上列出开着的面；返回键调 `history.back()`，所以面上的返回键、浏览器的返回键与边缘返回手势走同一个栈，先退出眼前的面，再回到来源（refrain §3-14「焦点与返回」）。离开的面被关上：右侧放掉它的项（`closeRight`，焦点回到打开它的控件），世界层的面撤下时焦点若落在 `body` 上，就回到开面时持焦的控件（7-7）。在面里换房间（点一个会话）是一次地址跳转，新的一格不带面，所以落到那个房间的对话；退一格又回到面。刷新保留浏览器为这一格存下的 state，世界层的面照原样打开；右侧的项不跨刷新，所以一格写着右侧而右侧已经没有项时（刷新之后，或从右侧里的链接去了别页再退回来），这一格被改写成不带右侧，而不是再退一格——再退一格会把人带到比他要的更早的一页。画廊的标本不碰历史。
  **其四，三键在芯片行。** 一栏上对话框设置行的芯片组在开头留出三键的宽度（`edge-slot`），三键以 CSS anchor positioning 钉在那里，所以三键与房间芯片同在一行、折行的芯片仍在三键旁边，模型与强度折到下一行时靠右，跟着对话框走——空房间里在竖直中线上，检阅的带里在世界层的面下——软键盘打开时仍在拇指区（refrain §3-6）。`anchor-scope` 把这个名字限在一个 frame 里；不支持锚定的引擎上，与没有对话框的页上，三键留在外壳底部自己的一行。
  **其五，视口。** `index.html` 的 viewport 写 `viewport-fit=cover`，页面画到刘海与 home 指示条之下，frame 的内边距取页边距与 `env(safe-area-inset-*)` 中较大的一个；也写 `interactive-widget=resizes-content`，能随软键盘缩小布局视口的引擎就这样做。不能的引擎（移动 Safari）上，键盘只缩可视视口，`100dvh` 不动，所以 `followViewport` 把可视视口的高与它在布局视口里的偏移写在 frame 上（`visual-viewport`），外壳正好站在看得见的那一块上，对话框、硬币键与三键都在键盘之上。捏合缩放不跟：高按页面自身的比例算，平移留给人，否则页面会随每一次捏合重排，人永远放大不了。图层键按下时不取焦点，所以换档与临时看一眼既不唤起也不收起软键盘（§3-14「手机软键盘与 IME」）。200 % 缩放下 1440 的窗口是 720 CSS px，外壳按一栏画，每个动作仍可到达。
  **验收**：`#/gallery` 的 `mob ·` 六个夹具——390×844 的工作中房间、空房间、世界层的面、右侧的面，键盘占去下方 344 px 的同一部手机，与 200 % 缩放的 1440×900 窗口。真实手机上的软键盘、输入法、旋转与分屏要人在设备上走一遍，headless 的几何不代替它（refrain S7.3 验收）。
- **4-53 对话里的回复：闭合的块随闭合画出，开着的末尾是原文，读屏器不逐字听。** 回复的块由 `refrain/laid.svelte` 画，与 RefRain 的预览同一个画法（4-26）：标题仍是页面字号里的段落，一屏只有一个 `h1`；代码块按 `info` 上色；城读得出而不画的（HTML、前置元数据、过深的嵌套）画成等宽的原文，公式由 KaTeX 画（4-64a），所以「这里有东西没画」不会读成「这里什么都没有」。还在说的回复（`talk/saying.svelte`）：已经闭合的块画在上面，开着的末尾画成原文，最后 10 个字淡出、其后一个 accent 的点，没有计时器、没有逐字的节点；这段文字不带 `aria-live`，读屏器在 run 结束时听那一行结局，只听一次。城没有答（链路不通、超时）时文字一直是原文，不画空态，也不在线程里另说一句：那一问的沉默由 `core/asking.ts` 的超时（`E_TIMEOUT`）报告一次。**链接**：`http(s)` 与 `mailto` 的链接在新标签页打开；相对地址在一段回复里没有一份文档可以相对，所以画成链接的文字，不是一个按下去什么都不做的控件（RefRain 的预览里相对地址相对文档所在的目录，那里 `onOpen` 在）。回复的块不带区间、不接焦点，它们是阅读的文字，键盘只经过其中的链接。
- **4-54 RefRain 画三种格式——PDF、DOCX、HTML；每张预览写出工具、参数与版本，两版之间比较的是各自读出的文字。** 哪种画法由文件名定（`views/refrain/formats/format.ts` 的 `drawnAs`：`.pdf`、`.docx`、`.html` 与 `.htm`），因为 wire 的 `Format` 只说是不是 Markdown；标签上工具的版本读 `client/package.json` 钉住的那一个，所以标签说不出 bundle 里没有的版本。**每张预览上方一行**（`formats/label.svelte`）：格式名 · 工具与版本 · 参数 · 版本（RefRain 头一行已写出版本时不再写），其下每行一条「这张图没画出的东西」。三种格式各自的比较对象与保真等级：
  - **HTML** 是文本，所以它在 RefRain 里多一个「预览」读法，由本浏览器画在一个 `sandbox=""` 的 `iframe` 里（`formats/frame.svelte`）：脚本不跑、表单不交、窗口不开，页面的来源与客户端不同，所以它读不到这一页、城的 socket 与浏览器的存储。框里的页面先经 `formats/framed.ts`：浏览器自己的解析器读一遍（读时什么都不执行），开头注入一条 CSP，只许页面自己的样式与 `data:` 的图片、字体和媒体；删掉自动刷新与 `<base>`，每个链接指向沙箱不开的新窗口；原来的 doctype 原样写回，所以没写 doctype 的页面仍按怪异模式画。标签写「脚本关闭 · 不读外部资源 · 链接不打开」。画的是读法打开时编辑器里的文字，有草稿时标签多一行「按未保存的草稿画出」——这与 Markdown 的预览不同，因为 HTML 不需要城来读。**比较对象**是源文：RefRain 自己的 diff 与版本读法（7N）。**保真**：引擎本身画，除脚本与外部资源之外与浏览器直接打开它相同。
  - **PDF** 由 pdf.js 画（`formats/pdf.ts` 是一块懒加载的分块，worker 是单独的文件）。每页一开始就按第一页的比例占位，滚动条因此是整份文件的长度；一页进入视口一屏之内才画，栏宽按 32 px 一档变动时重画，所以一百页的文件只付屏上那几页的代价。worker 按名字要的资源——没有自带编码表的字体要的 Adobe CMap、两种机器未必有的符号字体（FoxitSymbol、FoxitDingbats）、三个 WebAssembly 图像解码器（OpenJPEG、JBIG2、qcms）——是 bundle 里的文件，经页面一侧的资源门交给 worker（`useWorkerFetch: false`），所以除了送出这一页的城，什么都不向别处取；文件没带的其余字体用设备自带的字体画（`useSystemFonts` 的默认，与 Firefox 自带的阅读器相同）。有密码的 PDF 不打开，说这一句；读不出的说出 pdf.js 给的原因；两种情形都说原文件不动。**比较对象**是每页按阅读顺序的文字（`getTextContent`），页与页之间一行页标；版式、图片与字体不比较。没有文字层的页（扫描件或画出来的字）在标签里逐页点名，不做 OCR，所以一页扫描件的沉默不会读成「没有改动」（refrain 附录 D）。**保真**：页面像素是 pdf.js 画的；比较只到文字。
  - **DOCX** 先由 `formats/zip.ts` 读压缩包自己的目录：声明的解压总量超过 `UNPACKED_MAX`（64 MiB）或部件多于 `PARTS_MAX`（4096）就不打开；然后用平台的 `DecompressionStream` 逐部件解压，一个部件长过目录承诺的长度就停下、判为读不出。只有守住了每一个承诺的包才交给 docx-preview，所以它自己再解压一遍也超不出这个上界（附录 D「解压有界」）。docx-preview 把页排进页面从不显示的元素，再整份写成一份 HTML 交给同一个沙箱框（页与页之间透出框后面的面板）；页比框宽时按 40 px 一档缩到框宽，所以一行仍在文件断行的地方断、两边不被裁掉。图片与嵌入字体以 `data:` 地址随页走，那是框唯一肯读的一种；`altChunk`（文件里夹带的外来 HTML）不画，因为 docx-preview 会把它画进一个不带 sandbox 的框。标签写「近似版式 · 修订按接受后的结果显示 · 不显示批注」，并从文件本身数出修订、批注与嵌入对象各有几处，逐行说出，所以没画出来的东西不被读成没有（附录 D）。**比较对象**是正文的段落按修订接受后的文字（只读 `w:t`；删去的字写作 `w:delText`），一段一行，文本框里的段落各占一行；页眉、页脚、批注与对象不比较。**保真**：近似，字体、分页与浮动对象可能与 Word 不同。
  - **比较**（`formats/compare.svelte`）把两版读出的文字交给 RefRain 自己的只读对照（`refrain/editing.ts` 的 `openComparison`），所以一处改动的样子与 Markdown 两版之间的一样；标签写比较的是什么、从哪一版到哪一版。
  - **不是文本、也不是这两种格式的文件**仍只写「二进制」与长度，RefRain 头一行不给读法（`formats/opaque.svelte`）。
  - **城里的文件**（4-61）：`formats/opaque.svelte` 经 `Query::Bytes` 逐窗取回这一版的字节，交给 `pdf.svelte` 或 `docx.svelte`；超过 `DRAWN_BYTES_MAX`（64 MiB）的不取，说出它多大。两版比较从这份文档的版本列表里选另一版，两版的字节都取回之后交给 `compare.svelte`。
- **4-55 修改提案是请决定卡的第三种：正文是逐句的 diff，决定逐句，基线过期的卡只能拒绝。** 一张卡（`ProposalCard`，`crates/wire/Spec.lean` §8-73）画在两处，都是 `views/refrain/proposals_card.svelte`：右侧 RefRain 的文稿上方（`views/refrain/proposals.svelte`，问 `Query::Proposals(这份文稿)`），与信箱的待决段（`views/mailbox/deciding_document.svelte`，一份文稿一个）。**正文是 diff**：`Slice` 按序接成一段，`same` 是原样的字，`delete` 是 `<del>`（alert 淡底加删除线），`insert` 是 `<ins>`（accent 淡底），每句前后的空白照原样（`lead`、`text`、`trail`），所以人读到的就是城会落下的字。**三个答复**：y 接受整张（每一句改动 `accept`）；e 进入改后接受——每一句改动一行，带一个「取」的勾选，插入的句子是可改的文本框，再按 y 发出（改过的插入句 `amend`，没改的 `accept`，没勾的不点名，即拒绝；一句都没取时 y 按不动并说这等于拒绝），再按 e 回到 diff；n 拒绝整张（`verdicts` 为空）。判词由 `views/refrain/proposals.ts` 的 `verdictsOf` 一处拼出：只点名改动过的句子、每句至多一次、`amend` 只给插入句，因为城对另外三种点名一律以 `E_INVALID_ARGS` 拒绝（documents D16）。**过期的卡说出来，不替人去试**：卡的 `baseline` 不等于回答的 `version`（或文稿此刻没有版本）时，卡头下一行写出它基于哪一版、文稿现在是哪一版，y 与 e 置灰并说为什么，n 仍可按——城对有接受的过期决定整次拒绝 `E_VERSION_CONFLICT`，拒绝一张过期的卡却不看基线。页面判得比城早（另一个页面刚存了一版、回答还没重来）时城的拒绝照样会回来：卡读 `belief.refusal`，动作是 `decide proposals`、subject 点名这张卡或没点名任何一张时，把城写的出路画在卡上，并重问这份文稿的提案。**发出之后**卡上的答复都置灰，卡头下写「决定中」；城落下 `proposal_decided`，`staleness.ts` 让这一问失效，重答里没有这张卡，卡就消失——卡消失就是回执。链路在途中断开时写「待核对」，重连后以同一个 `idem` 再发一次：城按键认出重发、只答第一次的结果（§8-73 的落下顺序），所以重发不会决定两次。这一步一步的判定是同一文件的 `advance`。**哪些文稿进信箱**：城答全城开着的每一张卡在哪份文稿上、何时提出（`Query::OpenProposals`，wire D15，最新的在前）；信箱按这张表列出文稿，一份一次，按它最新那张卡的先后，卡的正文仍按文稿问 `Query::Proposals`。键上的数与待决段的数读同一处（`views/mailbox/deciding_proposals.ts` 的 `openCards`，即这张表的长度），所以信箱关着时也只问这一问（4-49），本页打开之前提出的卡与之后提出的一样进信箱。**卡头**：谁在问写提出它的 run 所在的房间（本页不认识那个 run 时写 run id 的前八位）；卡本身不带提出的时刻，所以头右端写它基于的版本前七位。在信箱里，卡的正文上方多一行文稿的路径、这份文稿最新那张卡提出的时刻（全城那一问答的）、「读这封信」与「在文稿旁打开」（`openDocument`）。**读这封信**（`openLetter`，`views/inspect/letter.svelte`）在右侧只打开这一张卡，一个页签就是一封信：四种读法——卡自己的 diff、发信 run 的对话（run 页的同一个 `talk/thread.svelte`，不给分叉与重试，所以只读；run 经 `Query::RunView` 的摘要并上本页折叠的读数，本页没见过的 run 也画得出）、卡写成时所依的那一版的全文（RefRain 按 `baseline` 打开）、文稿现在的全文——diff 是其中一种，信打开时停在它；头上一行是文稿的路径与去发信 run 页面的链接。信件打开时信箱收起，因为右侧在面外；关上信件时信箱重新打开、焦点回到打开它的那一行，行按卡的 `id` 认出（`mailbox/layer.ts` 的 `stepLetter`，`client/spec/Views/Workspace.lean` D73）；在 RefRain 里多一个「在原文中显示」，基线就是编辑器此刻那一版时把光标放到这张卡改的那一段的开头并滚进视野。提案一栏在 RefRain 里至多占右侧高度的五分之二、自己滚动，编辑器不被挤没。
- **4-57 远程设备的配对页是设置面的「远程」组；页面做设备那一半，再加门开关与「更换城钥匙」，配对与撤销仍只在城的控制台。** 设置树「城够得到什么」一枝在「网络」之后有一条「远程」（`views/settings/remote.svelte`），城所在的机器与一台设备打开的是同一组，画法按这个浏览器手里有什么分开：
  **没有邀请、也没有配对过**（存储读不出、浏览器缺一样能力时也是这一画法，前面多一句缺的是什么）：一段说明，然后是一次配对的编号步骤，再是门的控件，再是控制台上的六个动词（`/remote open`、`/remote pair <名字>`、`/remote devices`、`/remote revoke`、`/remote close`、`/remote replace-key`，以 `docs/operating.md` 为准）。**步骤**是 `views/settings/remote.ts` 的 `STEPS`，一处定序：门要一条写在城层 `CONFIG.toml` 的 `[remote]` 表里的 `https://` 通路；在控制台 `/remote open`；在控制台 `/remote pair <名字>`，它印出二维码与一条只用一次、十分钟内有效的链接；在另一台设备上扫码或打开那条链接（画出链接的形状 `https://<host>/#pair=<code>&city=<fingerprint>`）；在设备上按「配对」、抄下只显示一次的种子。在控制台上做的那几步带它的拼写与复制键（`machine/copy.svelte`），不带按钮：配对不上线协议（`crates/remote_access/Spec.lean` D4 ③：邀请显示在页面上，驱动页面的工具就读得到它），所以这一组没有配对的按钮。**门的控件**（`views/settings/remote_door.ts`，状态机是 `client/spec/Views/Door.lean`）：「开门」旁一个开多久的选择（`LASTINGS`，与 `/remote open --for` 同一写法，最长一周，缺省 12h），按下发 `OpenRemoteDoor{lasting_ms}`；「更换城钥匙」按下发 `ReplaceCityKey`；城答 `E_APPROVAL_PENDING` 时两者都画一个确认码输入框，说明码印在城的控制台上，焦点进框，Esc 取消并回到按下的那个控件，提交发 `ConfirmRemoteDoor{code}`；城的拒绝清空输入框、焦点回到那个控件，链路不在、命令没离开页面时状态机收到的是 `unsent` 而不是拒绝：回到按下的那个控件，状态行说链路断了（`remote_door_unsent`），不替城编一个码；`E_GATE_DENIED` 说再按一次、照控制台上新印的码输入，`E_TOOL_UNAVAILABLE` 说要在城自己的终端里跑 `sprawling serve`。确认发出后没有拒绝、过了 `RECEIPT_MS` 即算做成：门开没开着今天没有一个可问的回答，所以这里不画一个开或关的状态，开与关是两个控件（D4 ②，开门要码）。「关门」按下发 `CloseRemoteDoor`，不要码（D4 ①）。页面上只有一条全局的拒绝，所以只在请求或确认在途时一条拒绝才算门的回答。**末一步说城所在的电脑重启时配对会怎样**，读 doctor 答的 `DoctorAnswer.custody.keeps`（`remote.ts` 的 `restartOf`，一个 `DoctorCustodyLifetime` 一句）：跨重启保管（Windows 凭据管理器、macOS 钥匙串）时配对仍在；用加密的 vault 文件时启动输入口令后配对仍在；Linux keyutils 只到这次开机结束；只在城的内存里时城一重启就要重新配对；doctor 还没答时一句话写出三个平台各是哪一种。所以这一句按城真有的保管方式说，不按平台猜（D94）。
  **带着邀请打开**（`#pair=…&city=…`，§3-2）：先画城的指纹的前 12 个符号，再画「配对这台设备」。按下时页面取 32 字节熵作种子，`keyFrom` 派生两半密钥，在 `/remote/pair` 上走配对握手（`core/remote/connect.ts`）。成功时把结果存进这个源的 IndexedDB（`core/remote/device.ts`），请浏览器把这个源的存储标为持久（`navigator.storage.persist()`，被拒时照实说一句：浏览器可能在存储吃紧时清掉它），把地址栏换成 `#/setup/remote`（`history.replaceState`，配对码不留在历史里），然后把种子按四个一组画出来，只这一次，旁边一句「不保存；重装这个页面就要重新配对」，人按「我记下了」它就从页面上消失。种子在配对成功之后才画：先画的种子若配对失败，对应的是一把谁也没钉住的密钥。
  **配对过**：画城的指纹、这台设备的 id、配对的时刻，其下是同一句重启说明（`restartOf`），两个按钮。「锁上门」在 `/remote/session` 上握手一次，发一帧封好的 `Lock`（`crates/remote_access/Spec.lean` D5：任何设备都可以锁门离开）。「忘掉这台设备」经 `parts/dialog.svelte` 确认后删掉本地记录，并说城那边仍记着它，要在控制台 `/remote revoke` 才算撤销。
  **拒绝按原因各说一句**：指纹不符或城的签名不对（设备在发出认领之前就停下，码没有离开设备）→ 重新扫码，反复出现说明通路在改动它转发的内容；城以关闭帧给出的码（`E_GATE_DENIED` 是码已用过、过期或门已关）→ 在控制台重新 `/remote pair`；连不上 → 门是否开着、地址是否是控制台印的那个；不是安全上下文、浏览器不给 Ed25519 或 X25519、没有 IndexedDB → 说缺的是哪一样，因为这三样缺一样就做不成配对（D8）。
  **只在用到时付字节**：ML-KEM 与 ML-DSA 来自 `@noble/post-quantum`，`core/remote/` 用动态 `import()` 取它，第一次按下配对或锁门时才下载；X25519、Ed25519、HKDF、SHA-256 与 AES-GCM 都是 WebCrypto。
  **装到主屏**：`src/public/manifest.json` 与一个 SVG 图标由 Vite 原样拷进产物，`index.html` 以 `<link rel="manifest">` 指向它（名字、`start_url: "./"`、`display: standalone`）。不带 service worker：它买不到「钉住页面字节」（D10），而今天的主流引擎装 PWA 已不要求它。
  **页面怎样到设备、配对之后怎样说话**见 4-64。
- **4-64 页面从远程地址到达设备；配对过的源经封好的会话说线协议，其余照旧连 `/ws`。** 远程监听对一个不是 WebSocket 升级的 `GET` 答出这份 bundle，与城的端口同一张路由表（`crates/remote_access/Spec.lean` §8-10，wire D20），所以二维码里的地址打开的就是配对的那一页。链路说话经一条线（`core/socket.ts` 的 `Line`）：浏览器的 `WebSocket` 连本源的 `/ws`（`plainDial`），或一次远程会话（`core/remote/session.ts`）。哪一条由 `dialFor(location)` 在**每一次连接**时判：本页是 `https:`、这个源的 IndexedDB 存着一台配对过的设备，走会话；否则走 `/ws`。每次连接都判，所以在这一页上刚配对完，链路下一次重连就走会话，不必重新载入。会话一条线是：`/remote/session` 上一次会话握手（与锁门同一段，`connect.ts` 的 `sessionOver`），此后链路发的每一帧文本封成 `Payload` 的 `frame` 发出，城发回的每一帧依次打开、读成文本交给链路；一帧打不开、读不成或是锁门，线就关上，链路照它的阶梯重连。**封与开都排成一列**：WebCrypto 的加密与解密是异步的，两次 `seal` 的结果可能不按调用的次序回来，而城按计数开帧，乱了序的第二帧就打不开；所以发出的帧一帧封完、发出，下一帧才开始封，收到的帧也一帧开完才开下一帧（`talk` 的不变量，`core/remote/session.test.ts`）。设备的问候里不带配对令牌：中继把城的令牌替它放进去（§8-10）。远程源上不在线协议里的几扇 HTTP 门（`/transcribe`、`/drop`）答 404，所以设备上的录音与拖入文件按「城拒绝了」说出来。没配对过的浏览器打开远程地址而片段里没有邀请时，链路连 `/ws` 连不上，外壳照常画「连不上」，设置树里的「远程」组说该怎样配对；不另做一个外壳之外的首屏（D35 的重开参数已看过：配对之后用的仍是这个外壳）。
- **4-64a 公式由 KaTeX 排成 MathML，在一块单独的产物里；它不认的公式仍是原文。** 城把 `$…$` 与 `$$…$$` 读成 `construct: math` 的 `Unsupported`，原文带着定界符（`crates/documents/Spec.lean` D22），所以页面分得出公式与文字。`refrain/laid.svelte` 把这样一个行内交给 `refrain/formula.svelte`：两边各两个美元号是独占一行的公式，各一个是行内公式。页面第一次画公式时以动态 `import()` 取 `katex`，`katex.render` 以 `output: "mathml"` 把 MathML 作为元素建进这个组件自己的一个节点，不经 `innerHTML`（4-26）；排版由浏览器自己的 MathML 引擎做，所以不随包 KaTeX 的样式表与字体。KaTeX 在 `throwOnError` 下拒绝的源（一个它不认的命令、一个不成对的括号），以及它还没到的时候，画的都是等宽的原文，与其余不画的构造同一个样子；`trust` 关着，`\href` 一类会造出链接或取外部资源的命令被拒，同样画成原文。读屏器读 MathML 本身；KaTeX 在 MathML 里附上原文的 `annotation`，复制时带出的是 TeX。对话流与 RefRain 的预览经同一个 `laid.svelte`，所以两处的公式是同一种画法。
- **4-64b 说话可以只用键盘：Accel-K 打开 palette，选「说话，转写进输入框」。** 这一行只在对话页、城能转写（`ui.hearing()`，即答 `endpoints` 时选了 `transcribe` 的模型）且浏览器能录音（`core/speaking.ts` 的 `canRecord`）时出现，与输入框旁的麦克风出现的条件相同；行尾写这句话会落进哪个房间。选它就是按一次那个麦克风（`talk/speak_asked.ts` 的 `askToSpeak`，`talk/record.svelte` 听它）：录着时再选一次就停下，城听到的话照旧落进输入框，不直接发出（4-16）。palette 随之关上，焦点回到它打开之前的地方。
- **4-65 复制键是一个控件：字形、名字与说明一次给齐（D55）。** 页面上每一个按下就复制一段文字的键都是 `views/machine/copy.svelte`（doctor 页的命令、远程组的步骤、版本回答的更新命令、提交事实单的 oid），所以 D55 对按钮的要求——认得出的字形、名字、悬停说明——只在这一处兑现：面上是 `parts/glyph` 的 `copy`（lucide 的两页纸）加看得见的名字「复制」，名字就是它的可访问名；说明经 `parts/tip.svelte` 画出、以 `aria-describedby` 挂在键上，写的是会进剪贴板的那段原文（`copy_note`），所以一行里两个复制键各说各的。说明在悬停与键盘焦点时都出现，因为触屏没有悬停（4-18）。写入兑现之后字形换成 `check`、名字换成「已复制」，`RECEIPT_MS`（1200 ms）后还原；写入被拒时面不变，不画一个说谎的勾。浏览器、操作系统都不改变这一画法：Chromium、Firefox 与 Safari 在 Windows、macOS 与 Linux 上走同一条 `navigator.clipboard.writeText`，提示的两条定位路见 4-18。
- **4-66 第二级提醒是运行组的一张卡，不在 MCP 页。** 第二级提醒（`views/settings/context_rung.svelte`）回答一栋楼里的 run 在上下文窗口的哪一处说交接那句话，经 `ConfigureBuilding.context_second_threshold` 写进这栋楼，现值读 `Query::Config` 的 `second`（4-58）。它站在设置树「运行」枝的运行组里，在自主权卡之后，卡头一个原生 `<select>` 按 `buildingsOf` 的次序选楼，与规则组同一种选法（D78）。理由：人找这个数时问的是「run 怎样跑」，与常设审批、自主权是同一类问题；MCP 页的右栏是另外两条接工具的路（Composio 与桌面），一张关于上下文的卡放在那里只因为它与那两件同走 `ConfigureBuilding` 一帧，那是线协议的分组，不是人的分组。否决的两处：留在 MCP 右栏（帧的分组冒充了页面的分组，人在 MCP 页上找不到理由去找它）；楼页的沙箱一节（楼页是读一栋楼的地方，常设的运行设置从设置树到达，4-8，且沙箱一节说的是边界，不是上下文）。重开参数：城把第二级提醒改成城层或房间层的一个值时，卡跟着那一层走，楼的选择随之去掉。
- **4-58 run 的冻结事实与城的两级提醒，各从线上的一个字段读，页面不抄一个数。** 两级上下文提醒都读这个房间的 `ConfigAnswer`：第一级是 `first`（城的 `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`），第二级是 `second.percent`，由 `talk/gauge.ts` 的 `remindersOf` 一处读出，上下文环、所选会话仪表的「上下文」格与会话栏的细条三处读它；城不答 `first` 时环上不画绿点，提示与格里也不写那一段，不拿页面里的数补。缓存读与缓存写分开读 `Used.cached` 与 `Used.cache_write`，各回合相加；没有一个回合报了缓存写时，「缓存」格不写「写」那一段，而不是写成零。命令的退出码读 `Call.exit_code`：城从完整的结果里读出它，而线上的输出头可能被裁到不成 JSON，所以监视器、检视面的终端（`monitor/trace.ts` 的 `endingOf`）与时间轴都不再从输出里解析退出码；没有退出码的调用（被信号停下、还没有结果）不画数字。一个 run 的写入限制、准入要求、落地策略与模式读它开场的 `Opening.policy`：「边界」格写写入限制，「准入」格写准入要求与落地策略，模式写进「模型」格与首条消息头。工作树名读 `RoundsAnswer.worktree`，写在所选会话标题下的地址行。提交的 B3 读 `CommitAnswer.b3`，在提交栏选中的提交下与 oid 各占一行；它是宣告这个提交的 `checkpoint_committed` 那一行的 BLAKE3，账本里能对得上的就是它。强度读开场的 `Opening.effort`（`run_started` 记下的、这次 run 的请求冻下的那一个），所以还没有提交的 run 也写得出强度；没说强度的派活不写，不拿房间现在的设定充数。提交的时刻不在提交栏里重写一遍：选中提交即选中它的会话，时间轴上它的检查点行写着同一刻（`docs/frontend-method.md` §7D 的「时刻」一行）；时间轴跨过 UTC 的午夜时，新的一天在它的第一行之上写一次日期，所以每一行的时刻都对得上头上那个日期。
- **4-59 名字、用时、时刻与时钟各有一个家。** **主 Agent 的名字**（refrain §3-13）：还没开始的会话——空房间的标题、对话框的占位符——读 `Query::Identity` 的 `stated.mayor`，那是下一个会话会冻下的名字。开始了的会话读它冻下的名字：`Opening.names.mayor`（`crates/wire/Spec.lean` §8-79、D17），市长回合的消息头、在说的那一行与全景档对话带的末句写它，回答到达之前也不借今天的名字。没起名、身份区读不出、`names` 缺席（早于身份入账的账本、内容库里没了那一版）时都写语言表的角色名（`nav_mayor`，refrain §3-13「回退到本地化角色」）；`hall/mayor` 这个地址仍留在详情里。别的居民一直写地址末段。判定在 `talk/naming.ts` 的 `called` 一处，调用方只决定交给它哪一个名字。**一次调用用了多久**只有 `talk/timing.ts` 的 `lastedOf` 一个读法（两个时刻都量过、差不为负）：对话的工具行、检阅档时间轴、run 页的调用列表、监视器与检视面的终端都读它，所以一次调用在两屏上说不出两个数，倒序的两刻在哪里都不画。**落账的用时**的字只有一对：`talk_took_ms`／`talk_took_s`，经 `landedWords(ms, lang)` 写出，对话工具行、消息头的 TTFT 与时间轴都读它。**ISO 时刻**只由 `core/time.ts` 写：`isoInstant` 是 `<time datetime>` 的整个时刻，`isoDay`／`isoTime` 是它的两半；视图不调 `toISOString`。**走的时钟**只有 `talk/timing.ts` 的 `ticker(now)`：每 100 ms 一拍、没人订阅或页面隐藏时停、恢复可见时立即从时钟重算；工具行、信箱「在跑」段与会话栏的「跑了多久」都订阅它，读数始终是 `now − 事件时刻`，所以隐藏一分钟的标签页一露面就是对的数。
- **4-60 人选的东西有一个家：档位与外观随偏好进城，run 策略在派发前的设置行里选，提案读全城一问。** **偏好**：`glass` 与混合档透明度（`blend_percent`）随 `PutPreferences` 进人的 `config.toml`，城的回答盖过浏览器的缓存（`core/prefs_city.ts`），二者在城里都可缺：缺席即这个人没说过，页面画它自己的姿态（wire D14）。档位不随偏好进出城：每次启动是专注档（D47）。工作台的栏序与栏宽仍只在这个浏览器（D24）。外观组多一张「档位」卡（`views/setup/tier.svelte`），三档一组单选，与图层键改同一个档，触屏上不靠键就能选档。**run 策略**：会话开始之前，设置行只给工作区、模型设置与权限设置三个入口。模型设置沿用 `parts/popover.svelte` 的键表：provider 列选择父项，model 列只列该 provider 的模型，thinking 列在模型旁；换 provider 只换子列表并清空搜索，选模型只发 `SelectModel`，不发 `OpenSession`。model 子列表超过 `talk/composer.ts` 的 `FILTER_AFTER` 时，面上给搜索框，只筛当前 provider 的 model 子列表而不改选中值；搜索框取焦或输入时，游标指向 model 列；搜索框持焦点，Home/End 留给文本光标，其余菜单键转交既有 Popover 三列键表（与 composer 的 slash 输入框同一个 bind 模式），`PopoverBinding` 给按键处理器与同一组件生成的 listbox DOM id；输入框的 `aria-controls` 读这些 id，`aria-activedescendant` 指向当前列游标，关闭后焦点还给模型入口。没有模型可选时入口隐藏。权限入口展开一面，默认映射由 `talk/policy.ts` 的 `POLICY_SWITCHES` 决定，用两个原生 checkbox（`role="switch"`），分别是工作（chat/work）与常规写入（create/full），这两个映射是用户尚未覆盖的默认选择，其余档位是准入要求（tested／double_validated／contract_kept），与落地值用同一面内的原生 select；Tab 按文档顺序经过它们，Space 改 switch，select 用平台键表，Escape 关闭并还焦点给入口，焦点走出面时关闭而不抢回焦点。两个开关只改自己的字段，其他字段留在同一个 `Ui.policy`。权限面打开时焦点到第一个开关；gallery 预先打开的面不取焦。会话开始的判定是当前段已有 run 或首条发送已经被连接接受，不等第一次模型调用；开始后设置行不画控件或边界提示，冻结事实在首条消息头。`/admit` 改准入要求，`/room` 去另一个房间。**提案**：信箱读全城开着的卡（4-55）。**动词拼写**：「停止」与「全城放行」按钮上写的是 `/stop` 与 `/release --all`，读 `core/slash.ts` 的 `STOP`、`RELEASE_ALL`，`lang.json` 不再另拼。
- **4-61 一份文档的版本、字节与导出都按版本问城：列出各版、逐窗取字节、导出成一份文件。** 三问都以内容库里的版本为身份（`crates/wire/Spec.lean` §8-80、§8-81、§8-83），所以页面读到的永远是它点名的那一版，盘上的文件此后怎样变都不影响。**版本**（`refrain/versions.svelte`）：`Query::Versions { at }` 的每一行写版本的前七位、来源与长度——经页面写下的写「已保存」与它的时刻，城只在盘上或作为一次保存的基线见过的写「页面之外写下」，不写时刻、不写是谁，因为城不知道；内容库里没有的一版列出但不能选；答复说更早的版本没列出时，列表末尾说一句。两边各选一版（另有一项是草稿），全文由 `refrain/gathered.svelte.ts` 按版本接齐，接齐之前比较处说在读，读不了时说这一版城读不出。PDF 与 DOCX 的两版比较在同一个读法里，只是两边各取字节、交给 `formats/compare.svelte`。**字节**（`core/document_bytes.ts`）：一窗至多 1 MiB，下一问从上一窗的末尾起，长度由第一问答出；别的版本、对不上末尾、读不出 base64 的一窗不改变已接的字节；超过 `DRAWN_BYTES_MAX` 的对象不再往下取。视图里取字节的是 `refrain/fetched.svelte.ts`，一个组件一份，组件卸下就不再问。**导出**：Markdown 的一版在 RefRain 头一行有「导出 HTML」，按下才问 `Query::Export { at, version }`，答复存成 `<文件名>.html` 下载；城答不了时头一行说「城没能导出这一版」。PDF、DOCX 与 HTML 两版的比较在比较读法的标签行有「导出比较」，存成一份 Markdown（`formats/compared.ts` 的 `comparedMarkdown`）：文件名、从哪一版到哪一版、比较的是什么与工具和参数（就是标签上的那几行）、没有文字的页，然后每一处改动一节，删去的与加入的各在一个代码块里，所以一个读 Markdown 的工具（或一个会改 Word 的 run）拿到的是可以照着改的意见，写回那份文件由有这种格式能力的工具去做（refrain 4-12）。
- **4-62 键与查找：行间走动一张表，第三层的键画在行尾，Accel-P 找文件，会话按任务找。** 工具行、信箱的条目与 run 板读 `core/lines.ts` 的 `LINE_KEYS`，各自只答自己用得上的那几步：工具行走上下与首尾、Esc 收起右侧（Enter 是按钮自己的）；信箱的条目走上下与首尾（Enter 是链接自己的）；run 板走上下与首尾、Enter 打开这一行。设置树走 ↓／↑ 与 Home／End，**字母在那里是首字母**：按一个字母，焦点到下一个以它开头的组（到尾绕回），所以 j／k／g／G 在设置树里不走动。**第三层画在行尾**（refrain §3-12）：打开着或持焦点的工具行在行尾画 ↑ ↓ Enter Esc；设置树的每个组在行尾画它的首字母（D40）；两处都经 `parts/kbd.svelte` 从 `core/lines.ts` 读，没有第二处拼写。**Accel-P** 打开 `views/finder.svelte`：一个模态 `<dialog>`，里面是 APG Combobox（输入框带 `aria-controls` 与 `aria-activedescendant`，列表是 listbox），问 `Query::Find { under, text }`（`crates/wire/Spec.lean` §8-82），`under` 是眼前那栋楼：对话页是这个房间所在的楼，楼页是这栋楼，别的页是 Mayor 所在的 `hall`；面的标题写出是哪栋楼。↓／↑ 在结果间走，Enter 经 `openDocument` 在右侧打开这个文件（`version: null`，即工作树此刻的文字）并关上面，Esc 关上面、焦点回到打开它之前的地方。城答 `Walked::Cut` 时列表下写一行「只看了这棵树的一部分，没列出的文件可能也匹配」，不让人把没列出读成没有。浏览器把 Ctrl-P 留给自己时（打印），命令面板里同名的一条是它的后备。**会话查找**：信箱「最近」段的过滤框按任务原文过滤这一页问回来的全部会话，不只是虚拟列表挂上的那几行，并写出没问过的房间不在查找之内。**设置树「诊断」一枝末尾有「上手指南」**，开 `#/welcome`。
- **4-63 右侧与对话守住自己的位置。** 八件事，各有一个家。
  **定位**（§3-2）：一次调用与一份文档的一版各有一个写在对话地址后面的定位，run 页的透镜与上手指南的一步各有一段路径。`app.svelte` 把 `View.item` 交给 `workspace.svelte`，它在到达时、以及地址栏换成另一个带项的定位时经 `openCall`／`openDocument` 打开那一项，只此一次；定位从不发送、从不保存，因为发一句话与存一份稿是动作，动作不住在地址里，刷新与别人发来的链接都只会打开东西。**页签本身是链接**（`inspect/strip.svelte` 的 `<a role="tab" href>`）：地址是这一项连同它旁边那段对话的定位，浏览器的「复制链接」与「在新标签页打开」都读它；按下只把它带到前面，不改地址栏（D41）。没有定位写法的项（一次提交的改动）的页签不带 `href`。不另放一个复制键，因为对话页的常驻控件由 `xtask render` 按 `talk_controls` 数着（P11），一个复制键多一个常驻控件，而页签已经在那里。
  **一份右侧**：楼页与对话页装的是同一个 `views/right.svelte`（4-50 其三、D27）。
  **提交的改动开在右侧**：`RightItem` 多一种 `changes { base, head }`，经 `openChanges` 打开，`views/changes.svelte` 画它，属编辑器区，页签名是 head 的前七位；工作台的提交栏点一行时提交的事实单（oid、B3、父提交）仍在那一行下面，改动开在右侧，而不是在一栏 3 列宽的栏里展开一份补丁（refrain §3-9）。
  **页签的草稿点**：RefRain 的会话在回执不是 `clean`／`saved` 时把自己登记为「有没存的草稿」（`refrain/session.svelte.ts` 的 `holdsDraft`，以文档的地址与版本为键），页签带给这样的文档页签画一个 alert 圆点，可读名字里加「未保存」（7F）。
  **引一行进开着的框**：diff 的行号经 `talk/quoting.ts` 的 `quoteInto` 送出：那个地方有一个挂着的框时，引文接在框里的字后面另起一行，光标与选区不动；没有时接进那个地方的草稿行，框挂载时读到它。框挂着时不写草稿行，因为框里还没落盘的几个键会被草稿行里旧的那份盖掉。
  **↑ 取回**：框空、不在输入法组合中时，↑ 把这个房间里人最近一次派出的任务原文写回框里（`talk/standing.ts` 的 `recalled`，读 belief 里这个房间的 run 的 `task`）。来源是账本而不是这个标签页的记忆，所以刷新之后、在另一台设备上发的话也取得回来；steer 的原文不在 belief 里，取不回。
  **切回来恢复**：框跟着它的地方走：换房间时先把旧地方的字写进草稿行、记下选区，再读新地方的字与选区；对话的阅读位置按地方记，回到一个停在中途的房间时回到那一行，回到一个跟着底部的房间时仍跟着底部。选区与阅读位置只记在这个标签页的内存里（`talk/standing.ts`），字仍在草稿行里，所以重新载入恢复字而不恢复选区。
  **存不下的草稿说出来**：浏览器拒绝一次草稿的写入时，那一行由这一页记着（`core/rows.ts` 的 `unkept`），`prefs.ts` 的 `draftUnkept(at)` 答这个地方的草稿是不是只在这一页；框下与 RefRain 的头一行下各画一行 `parts/unkept.svelte`：草稿只存在这个标签页、关掉就没了，旁边一个「复制」，回执等 `clipboard.writeText` 兑现才出现（§3-14 第 2 行）。
-/

/-!
### §7C 需要人同意的东西，长什么样

会停下来问人的东西**共用一套语言**，人学一次，之后每一次都是认出来而不是读出来。

- **前缘一条 2 px 的条，加一个字形。**
- **字形是编码，颜色只是加强。** `forced-colors` 会把每一处填充与边框颜色换成系统色，所以只用颜色说「请决定」的标记，恰好在最需要它的那些机器上什么都不说。
- 条画在**前缘**而不是左边，因为这一页也会用从右往左的语言排。

实现是 `theme/surface.css` 的 `@utility asks`，连同 `theme/preference.css` 的 `@media (forced-colors: active)` 里把它的边换成 `CanvasText` 的那一条。读者：检阅档仪表「边界」格里的门、请决定卡。

**请决定卡是一个部件**（`views/parts/decide.svelte`）：前缘条与字形，一行头写谁在问与何时，正文按种类换，答复至多三个，各在一个键上——y 同意、e 改后同意、n 不。种类与字形是一张表：`question`（居民提的设计问题，手的字形；正文是它问的事、它问到的内容，与「拒绝不会还原文件」的那一句，y 允许、n 拒绝，同一组问题一次答完）、`ask`（一个门在等人亲自动手，门的字形；正文是那次被挡下的动作、主体与城写的出路，城那边没有可替人发的帧，所以只有 n「知道了」）。`proposal`（一次 run 对一份文稿的修改提案，文稿加笔的字形；正文是逐句的 diff，y 接受整张、e 改后接受、n 拒绝整张，基线已过期的卡只给 n，4-55）。卡在对话里（等人的事以卡片插进对话流，4-7）、信箱的待决段里与 RefRain 的文稿上方是同一个部件。信箱里一份文稿的提案卡上方多一行：文稿的路径、这份文稿最新那张卡提出的时刻（读全城一问 `Query::OpenProposals` 的答案）与「在文稿旁打开」（4-55）。
-/

/-!
### §7G 从零到第一次对话：上手指南

欢迎页（`views/welcome.svelte`）是 refrain §3-15 的五步清单，一栏、一步一行：序号、步名、这一步此刻的状态。五步依次是连接 provider 并选定 `main` 模型、安装依赖项、文本与称呼、导入 skill、连接 MCP，只有第一步必做。一步展开时挂的是城为这件事已有的那扇门——供应方表单与 `main` 的模型选择、doctor 的报告与安装、治理文档、书架、去 `#/mcp` 的链接（`welcome/body.svelte`）——所以在这里做完与在设置里做完是同一个动作、同一张回执。

**完成与进度是两个来源，互不代替**（`welcome/guide.ts`）。一步「已完成」只由城的配置说：选了 `main` 模型、doctor 的使用层没有缺项、身份区写了用户 ID 或主 Agent 名称；skills 与 MCP 没有按城可读的完成判定，所以只画「看过」或「稍后再做」，从不画完成。人对一步做了什么——看过、稍后、指南下次从哪一步打开、是否已离开——是指南的进度，按城保存（`Query::Guide`、`Command::PutGuide`，`crates/wire/Spec.lean` §8-68）。展开一步记为看过；点开不算完成，稍后不画成已配置，后来在别处做完的一步按完成画。进度不写账本，没有事件让它的答案变旧，所以页面每写一次就再问一次，答案到之前画它刚发出的那一份。

**收尾只做一次选择。** 「开始对话」在第一步完成后可用，之前置灰并说出原因；按下它记下指南已离开，打开与主 Agent 的空房间，不发任何东西，也不往输入框里写字。「跳过全部可选项」把还没人看过的可选步记为稍后，记下指南已离开，再打开与主 Agent 的房间，与「开始对话」落在同一处（D54）。**启动时进不进指南由外壳判一次**（`welcome/guide.ts` 的 `opensGuide`，模型 `client/spec/Views/Guide.lean`）：城第一次答出端点表时，若停在对话页，而城里一个 provider 端点都没有，就进指南，不看城记着的「已离开」与 `welcomed`；有端点而没有 `main` 模型时，只在这个浏览器没走过指南（`welcomed` 偏好）时进。此后的回答不再换页，所以跳过之后进的对话页不被送回来。**一步做完，指南前进**（D55）：展开着的那一步由城的配置变成已完成时，指南展开下一个还没完成、也没人推迟的步，原来那一步收起；一次只展开一步（`client/spec/Views/Fold.lean`），展开的正文用 `drop` 进来，收起没有动画，减少动效时静止。

| 起点 | 键 | 落点 |
|---|---|---|
| `#/welcome`，无主模型 | 第一步已展开；Tab 进供应方表单 | 表单第一格 |
| 供应方表单 | 填写，Enter 提交 | 端点出现在表单上方，`main` 的选择框出现在表单之下 |
| `main` 的选择框 | 方向键选定 | 第一步画「已完成」，「开始对话」可用 |
| 任一步的标题 | Enter／Space | 展开这一步、收起原来那一步；进度记下这一步 |
| 一步里的「稍后设置」 | Enter | 这一步记为稍后，指南移到下一步，焦点落到下一步的标题 |
| 「开始对话」 | Enter | `#/talk/hall/mayor`，composer 取焦 |
| 「跳过全部可选项」 | Enter | `#/talk/hall/mayor`，指南记为已离开 |
| 展开着的一步被城判为完成 | — | 下一个未完成的步展开，原来那一步收起 |

**交互契约**：步骤列表是 APG Accordion。每一步的标题是 `<h2>` 里的按钮，带 `aria-expanded` 与指向正文的 `aria-controls`；正文是以标题为名（`aria-labelledby`）的 `section`。同一时刻只展开一步，再按展开着的那一步把它收起。「稍后设置」收起正文之后，焦点移到指南前进到的那一步的标题，不落回文档顶部。`#/gallery` 的 `guide ·` 夹具画三态（新城、有主模型而第三步展开、走到最后一步），各在 390 与 1440 两个宽度。

**拒绝框画城给的出路。** 同一个码覆盖几种原因（`E_CONFIG_INVALID` 既是「没选模型」也是「会话中途换了模型」），所以 `err_<code>` 的标题只说拒绝的种类，不说原因；`parts/notice.svelte` 把城写的 `recovery` 句子不折叠地放在标题下，动作与主体留在折叠里（D5）。
-/

/-!
### §7K 检阅档的工作台

检阅档里世界层是工作区，分三栏：会话、所选会话、这份工作所在的地方。会话、提交与文件在城里本来绑在一起——一个 run 在它的工作树里写文件，检查点把它们提交成一个提交——工作台把这层绑定画出来：**选一个提交即选中它的会话，并把时间轴定位到产生它的检查点**；选一个会话，提交栏标出它的那条泳道。「所选会话是哪一个 run」只在 `views/world/chosen.svelte.ts` 判定一次：选过的提交所指的 run（只在选它的那个房间里算数），否则是这个房间还在干活的 run，再否则是它最后持有的 run；所选会话一栏、提交栏的泳道与时间轴都读这一个答案。选另一个房间的提交时，对话经地址栏移到那个房间，与点一行会话是同一条路。

- **会话栏一行一段 session**（D44）：每个房间的每一段（`Query::Sessions` 答出的，含过去的段，`views/world/stretches.svelte.ts` 一处问、信箱的「最近」段读同一份）各一行，新的在前；置顶的在最上面一组，其余按楼分组，楼的次序取它最新一段的次序（`core/stretches.ts` 的 `grouped`）。一行是一个链接：点它或焦点在它上面按 Enter，让这一段成为主区里的那一段——对话区与所选会话栏都画它；当前一段的行链接到房间本身（`#/talk/<地址>`），所以 `/new` 之后它跟到新的一段，过去一段的行链接到 `#/talk/<地址>:<seq>`，前进、后退与重载都留在它上面。一行画状态点（在跑 accent、等你 alert、完成是空心圈；只有当前一段会在跑或等你，过去的一段总是完成）、标题（`SessionLine.name` 是人给这一段起的名字，空时是房间名；过去一段用 `text-quiet`）、置顶时一个图钉字形（`Glyph name="pin"`）与时间（在跑的写已跑多久，等你的写「等你」，其余写最后一行距今多久）；第二行是城给的预览 `SessionLine.preview`，没有时是这一段最后一个 run 的任务，本页也没有它的 run 时是它的起法：派活、新开、带交接、分叉；其下一行写这一段跑在什么上——模型 · 强度 · 工作区，城没说的那一项不写；派活开始的一段由居民交下时，标题之下另起一行写「由 <居民> 派来」，读这一行自己的 `SessionStart::Dispatched { by }`（D83）；再下一行是标签小签（不含 `pin`），当前一段再画上下文细条，带交接刻度。窄（右侧打开）时只留第一行。**混合档里会话栏收输入**：世界层其余各栏仍 `inert`，会话栏照样按 `--blend-opacity` 变淡，指针悬停或焦点在内时回到全不透明，因为人要在混合与检阅两档都能点侧边的一段切换主区。
- **行菜单**（`views/world/session_menu.svelte`，APG Menu Button，与栏标签的菜单同一模式，键在 7-11）：每行右侧一个按钮，可访问名是「`<房间>，<多久前>` 的操作」；悬停、焦点在行内或这一行在主区时可见，Tab 总能到达它。城没有名字（握手的 `Welcome::city` 为空）时按钮禁用，提示说标签由城保管而城没有连上。项依次是 `pin` 或 `unpin`（Mayor 的当前一段没有这一项：它的置顶是推出的）、每个由城存着的非 `pin` 标签一项「去掉标签」（工作区的默认标签不存，所以没有这一项）、「加一个标签」、「改名」，当前一段在菜单打开时读到它开场的运行策略（`Opening.policy`）后再加「从下一步起 <模式>」与「从下一步起 <写入限制>」两项（`ChangeRunPolicy` 带整份策略，所以读到之前不给这两项，不猜一个字段替人发出去；准入与落地照开场的不动）；「加一个标签」与「改名」把菜单换成一个输入框（标签框的可访问名「Tag」，`aria-describedby` 指向文法提示，读不出时 `aria-invalid` 且提示变 alert 色；改名框至多 `NAME_MAX` = 80 个字，留空交回房间名）。
- **标签筛选行**：栏里列出的行带着标签（不含 `pin`，含工作区的默认标签）时，栏标题下一行 `role="group"` 的切换按钮，第一个是「全部」，其后每个标签一个，都是 `aria-pressed`；一次只选一个，再按已选的那个回到全部，被筛空的组不画。筛选是这块屏幕上的状态，不存储。
- **过去的一段在主区**（`views/talk/past.svelte`）：这一段本页持有的 run 按对话的样子画（`Thread`，每一轮的分叉键照常），其下一条横线、一句「较早的一段对话，只读：城只接着这个房间当前的一段说下去」与一个回到当前对话的链接。从这一段接着说就是分叉：每一轮的分叉键，或信箱「最近」段的分叉入口（起点是这一段的末尾）；这里不另放一个「从末尾接着说」的按钮，因为它只是分叉的第二个入口，人从一行另起一条线时本来就在这一行上。一栏外壳的世界层面板下只画横线以下那一部分；分叉键发出之后，主区回到这个房间的当前一段。
- **所选会话**：标题与状态，状态旁是去 run 页的链接；一行地址——房间、工作树、基于哪个提交（`RoundsAnswer.opened_at`）；一张仪表：模型（与强度、模式，读 run 开场的 `Opening.effort` 与 `Opening.policy`，与首条消息头同写一句，`talk/frozen.ts`）、上下文（确切数字与两级提醒，读 `talk/gauge.ts` 的 `contextOf`，与上下文环是同一对数）、Token（各回合供应方报告的入与出之和）与单价（端点为这个模型报的价，原样）、缓存（命中率是缓存读的 token 占输入的份额，`Used.input` 含缓存读的部分；读与写）、成本（本会话读 `views/pricing.ts` 的 `runSpend`，全城读同一个 `CostAnswer.total`）、速度分两格，各一个主数在数字行、第二个数与回合数在注行，所以这一行恒为两行高（首块用时格：中位数为主，平均与计入几个回合在注；输出速度格：t/s 的 p50 为主，p99 与计入几个回合在注；一个回合的 t/s 是供应方报告的输出 token 数除以回复返回的时刻减首块的时刻，`views/world/speed.ts` 的 `speedOf` 读 `talk/timing.ts` 的 `ttftOf` 与 `tpsOf`；回合的时刻不是量出来的、没有首块、没有返回时刻或时长不为正的回合不计入，而不是计作零）、边界（写入限制、门、沙箱；门与沙箱读 `talk/bounds.svelte`，沙箱只在真有限制时画，判定在 `talk/sandbox.ts`，与设置行和首条消息头是同一个判定）、准入（准入证据与落地策略）。读不到的数画成一道破折号，不猜。仪表之下是时间轴：轴头一次写日期与「UTC」，每行 `HH:MM:SS.mmmZ`——回合写它被问的时刻，调用写结果落账的时刻，检查点写它的提交的时刻；回合行写首块用时与本回合 token，调用行写用时（十毫秒以下写整微秒，一秒以下写毫秒，一秒以上写到毫秒的秒数，`talk/timing.ts`），在跑的调用写「运行中」，exec 另写退出码，检查点行写提交的短 oid 与改了几个文件。调用行一点在右侧打开这次调用（`inspect/open.svelte.ts` 的 `openCall`）；检查点行一点选中它的提交。时间轴只在账本事件上重画，token 不碰它。
- **地方栏**：栏名是这份工作所在的地方——市长房间旁是城的名字，楼里的房间旁是楼的地址；栏里两个页签（APG Tabs）。第一个页签是提交：市长房间旁是全城的提交，楼里的房间旁是这栋楼的，都是 `commitsQuery` 的第一页，画成从 `CommitAnswer.parents` 走出的泳道图（`views/world/lanes.ts`，git `log --graph` 的走法：每条泳道等一个 oid，第一个父提交继承泳道，其余父提交另开泳道；页外的父提交让泳道一直画到页底，线上没带父提交的提交让它的泳道结束）。每行写 oid、消息与房间；节点按做它的 run 的阶段着色（`runs/phase.ts`），所选会话的提交与它的泳道取 accent。选中的提交在行下展开完整 oid、父提交与改动的文件，文件一点展开它的补丁，补丁的行号把「路径:行」接进对话的草稿（`views/changes.svelte`，与楼页同一份读法）。页里还有更早的提交时，栏底一条链接到楼页。第二个页签跟着工作区走（refrain 路线图 Q10）：市长房间旁是城的天际线（`city/skyline.svelte`，按城页的尺寸画、横向滚动、打开时停在市政厅，因为缩到一栏宽时每栋楼的名字小于七像素），点一栋楼，工作区移到那栋楼里最近跑过活的房间，没有就移到楼本身；楼里的房间旁是这栋楼的文件树（`building/tree.svelte`，一层一问 `Query::Listing`），点一个文件，它在右侧以工作树此刻的文字打开（`openDocument`）。

**栏的顺序与宽度归人**：两条分隔线可拖、可用键盘调（7-11），宽度以栏为单位、拖动吸附到栏线，每栏至少两栏宽，默认 3、6、3 栏；栏线由外壳排出的实际栏宽读出（`frame` 的 `gridTemplateColumns` 已解析成像素，子网格只读得到 `subgrid`），因为栏不等宽，按 12 等分去算会落错线；白银线就是第 4、10 条栏线，所以吸附到白银点与吸附到栏线是同一件事。每栏的标签是一个菜单按钮，菜单里的「移到左边／移到右边」换栏序，宽度随栏走。顺序与宽度存在浏览器（`core/workbench.ts` 判值，`prefs.ts` 存取，D24），在分隔线上按 Enter 或双击，整个工作台复位到默认的顺序与宽度。对话总在所选会话一栏之下，两行按 1 : √2 切，对话为高的一份（`grid-rows-[minmax(0,calc(100%/(1+var(--silver))))_minmax(0,1fr)]`），画完整的线程与输入框；`talk.svelte` 的 `band` 只在一栏外壳的世界层面板下面为真。所选会话排在最左时，对话从第 2 栏起，给左下三键留出第 1 栏。右侧打开时，会话与所选会话按人的顺序各占 2 与 5 栏，地方栏收起，右侧占第 8–12 栏（D72），分隔线与菜单不出现。

仪表、时间轴、提交栏与会话栏读线上的哪个字段见 4-58；会话栏的上下文细条是每行一个 `world/context_bar.svelte`，向那一行的 run 问它的回合，与上下文环读同一对数。

**当前状态**：选中提交的文件在行下展开补丁，而不是在右侧打开：右侧的条目（`inspect/open.svelte.ts`）有调用与文档两种，还没有「两个提交之间的一个文件」这一种。
-/

/-!
### §7L 设置面与设置树

设置键与 Accel-, 打开设置面：从左缘到达的原生模态 `<dialog>`（4-20 的做法：top layer、焦点陷阱、页面其余部分 `inert` 与 Esc 都由平台承担，`views/settings/panel.svelte`），宽度从窗口左缘到右白银线（`--silver-side × (1 + --silver)` 减半道栏距），一栏外壳上占满窗口；左边是设置树，宽一份侧宽减半道栏距，右边是正文（4-36），所以树站在会话栏站的位置、组站在对话站的位置。点外面、按 Esc 或再按一次 Accel-, 关闭，焦点回到打开它的那个控件（7-7）。**城、楼、记录、成本、登记簿、MCP 与性能这些页都从这棵树到达**（4-8，D19）。

**开着是一个地址**（D25）：`#/setup` 是设置面开在它上次画的组上，`#/setup/<组>` 开在那一组上（`core/route.ts` 的 `SETUP_GROUPS`），刷新、后退与外部链接都回到同一组。设置面开在一页之上而不是取代它：页面里按设置键时是那一页，从链接或刷新打开时是与市长的对话（`views/settings/hosted.svelte.ts`）。在树里选另一组是**替换**地址而不是压一条历史，所以后退键离开设置面，而不是倒着走过看过的每一组；关上时若是这一页自己压的地址就后退一步，否则用下面那一页替换它。

**树最多三级**：枝、条目、子条目。枝是一个按钮，按下展开或收起它的条目，同一时刻只展开一枝（D53）：设置面打开时展开的是当前页所在的那一枝——下面那一页在树里时是它的枝，否则是正文画着的那一组的枝；按下另一枝，原来那一枝收起；在树里选一组，那一组的枝本就开着。条目有四种：一种是设置组，选中后画在面板的正文里，面板不关；一种是页，选中后地址栏移到那一页、面板随之关上，条目尾部带一个箭头字形，说明它会离开面板；一种是有子条目的条目，按下展开或收起，下面那一页所在的那一项打开时就展开；一种是枝末的「更多」，按下展开这一枝的二级设置，当前组或下面那一页在其中时它开着。「更多」下只放组与页，不放有子条目的条目，所以树在类型上就不超过三级（`views/settings/tree.ts` 的 `Branch`）。展开的那一层用 `drop` 进来，收起没有动画（主题的动效表），减少动效时静止。折叠的状态机是 `client/spec/Views/Fold.lean`。

| 枝 | 条目 | 更多 | 子条目 |
|---|---|---|---|
| 城 | 你与主 Agent、总览（`#/city`）、楼 | 登记簿（`#/registry`）、成本（`#/cost`） | 楼：每栋楼一项（`#/building/<楼>`），按城的回答列出，名字是楼的地址，只有一栋也列出 |
| 接入 | 账户与供应方（含默认模型与思考强度）、官方 harness | 网络、远程（4-57） | — |
| 运行 | 运行（权限与运行策略的默认）、规则 | 自动化 | — |
| 扩展 | 技能、MCP（`#/mcp`） | — | — |
| 偏好 | 外观、配色、快捷键 | — | — |
| 诊断 | 记录、性能（`#/monitor`）、依赖 | 关于这一版、上手指南（`#/welcome`）、高级 | 记录：每个透镜一项（`#/record/<lens>`） |

**分组按对象分**（D52）：城是这座城自己的事实，接入是城能够到什么，运行是一个 run 被允许做什么，扩展是城装进来的技能与 MCP，偏好只属于这个人（外观与配色随城的偏好文件走，D47 的档位与工作台留在这个标签页与这个浏览器），诊断是出了事去看的地方。还没有页的条目不进树，等它的页落地再加：运行枝的沙箱与工具、扩展枝技能下的审核与调用记录、MCP 下的调用记录；树上不放按不动的条目。楼的子条目今天开的是楼页，楼页里有这栋楼的规则、技能、MCP 与文件各节（4-50），等它们各有地址时再成为楼下的第三级。**「你与主 Agent」是城枝的第一项，也是设置面第一次打开时画的组**：身份按城保存（refrain 路线图 §3-13），所以它是这座城的事实，而它是人第一次打开设置时最该先看见的那一组。从对话页到任何一页最多是「设置键、枝、条目」三步，楼与记录的子项与「更多」里的项多一步。**性能条目带一行进程读数**（处理器与工作集，`core/monitor.ts` 的 `summary`）：设置树画着时页面向监视器要摘要（`watchSummary`），城只在有人看时采样（`docs/frontend-method.md` §7D）。

**外观组有一张「档位」卡**（`views/setup/tier.svelte`）：三档一组单选，改的是图层键改的同一个档，只在这个标签页里有效（D47），所以触屏上不靠键也能选档。

各组的内容与它们读写的门见 4-48；树的键盘契约见 7-11。
-/

/-!
### §7N RefRain：右侧的文档编辑器

右侧的文档项（`views/inspect/open.svelte.ts` 的 `DocumentItem`）画 `views/refrain/refrain.svelte`，它只收三个参数：楼、楼内路径、版本（`null` 是城此刻的文本）。名字取自它的来处 RefRain（由人定）。它编辑 Markdown 与纯文本，读法有四种，状态有一行，行为由 4-46 规定。

**头一行**（32 px，与 `docs/frontend-method.md` §7F 编辑器上方那一行同一个座位）：左边是路径，楼名淡、文件名实；接着是版本的前七位与保存回执；右端是读法的分段控件「源码／预览／diff／版本」与保存键。读法只有 Markdown 与 HTML 才有「预览」（HTML 的预览见 4-54）；不是文本的文件没有读法。**回执**一个词加一个形状：草稿（alert 圆点）、保存中、已保存、待核对（链路断在保存途中，重连后用同一个键再发）、冲突、被拒；没有改动时不画。待核对与被拒在头一行下面多一行：前者说为什么还不知道结果，后者是城写的出路（D5）。

**四种读法**：

- **源码**：CodeMirror 6 的编辑器，Markdown 与纯文本都按字面显示，纯文本没有预览（A5）。撤销与重做只在本页的草稿里；查找与替换是编辑器自己的面板，字句取自 `lang.json`。接齐之前、超过上界的版本、历史版本都只读，只读的原因写在头一行下面的一行里。
- **预览**：`Query::Preview` 逐窗画基线那一版的块（4-26），滚到底再要下一窗；草稿不在预览里，有草稿时预览上方一行这样说。`Preview::Unsupported` 说这一版不按 Markdown 读，零个块是空态。
- **diff**：还是那个编辑器，加上与基线的对照（删去的行是 alert 淡底、加上的行是 accent 淡底），仍可编辑；没有改动时是「与基线相同」的空态。
- **版本**：城列出的这份文档的各版（`Query::Versions`，4-46、4-61），新的在前，每行写版本前七位、来处与时刻；本页手里已有全文的那几版直接用手里的文本，其余按版本逐窗接齐；选两行，下面是两版之间只读的 diff，默认是最近的两版。

**切换读法不卸载编辑器**：源码与 diff 是同一个编辑器，换读法只换它的一个扩展；预览与版本画在它旁边，编辑器只是隐藏，所以光标、选区、撤销栈与输入法的组合都留着。位置跨读法对应的是读者看到的最上面一行：从源码到预览，编辑器最上面一行所在的块成为预览的第一块；从预览回源码，预览最上面一块的起点成为编辑器的第一行，光标不动；编辑器显示出来之后才滚动，隐藏的编辑器没有高度可滚（`views/refrain/carry.ts`）。右侧的开合、换档与窄窗口下的重排都不重建编辑器：只有 `building`、`path` 或 `version` 换了才换一份文档。

**草稿与恢复**：每次改动后 300 ms 把草稿写进浏览器（4-46）；关掉右侧、换一份文档、重新载入都不丢。重开时草稿基于的版本仍是城此刻的版本就照原样恢复，否则进冲突。**冲突**是编辑器上方一条 `asks` 标记（7C）的横条：一句话说城里的文件已经换了一版、草稿留着，三个动作「对照」「移到现版」「丢弃草稿」，丢弃先经 `parts/dialog.svelte` 确认（D1）。

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| 编辑区 | 多行文本框（CodeMirror 的 `role="textbox"`、`aria-multiline="true"`） | Accel-Z／Accel-Shift-Z（Accel-Y） | 撤销／重做本页草稿里的一步 |
| | | Accel-F | 打开查找面板，焦点进查找框；面板里 Enter 是下一处，Shift-Enter 是上一处 |
| | | Accel-S | 保存；组合输入期间不保存，只读时不发帧 |
| | | Escape | 先结束输入法的组合，再关查找面板，焦点回编辑区；不关右侧 |
| | | Tab | 离开编辑区（不插入制表符），所以键盘用户出得去；缩进用 Accel-] 与 Accel-[ |
| 读法 | `parts/segmented.svelte`（7-4） | 同 7-4 | 换读法，焦点留在控件上 |
| 保存键 | APG Button（`parts/button.svelte`） | Enter／Space | 同 Accel-S；没有草稿或只读时 `aria-disabled="true"`，提示说为什么 |
| 版本列表 | 两个原生 `<select>`（「从」与「到」），各有可读名字 | 平台的下拉键（↑／↓、Alt-↓ 展开） | 换比较的两版；版本的名字、来处与时刻放不进等宽的分段格 |
| 冲突条 | 三个 APG Button | Enter／Space | 各做按钮上的事；「丢弃草稿」打开确认，关上后焦点回到这个按钮 |
| PDF 的页列（4-54） | 可滚动的组（`role="group"`，可读名字是文件名，`tabindex="0"`）；每页 `role="img"`，名字是「第 n 页，共 m 页」 | 取焦后平台的滚动键（↑／↓、PageUp／PageDown、Home／End、空格） | 滚动页列；页进入视口一屏之内才画 |
| HTML 与 DOCX 的框（4-54） | `iframe`，`title` 是文件名 | Tab 进出框；框内是那份页面自己的焦点顺序 | 框里的链接不打开，表单不提交 |

`aria-*`：编辑区的可读名字是文件名（`aria-label`）；只读时 `aria-readonly="true"`。回执是 `role="status"`，只在它换了词时播报，打字不播报（§3-14：流式与逐键都不逐字播报）；冲突条是 `role="alert"`，出现时播报一次，不取焦点。

**当前状态**：右侧的页签带（`docs/frontend-method.md` §7F）由检视面画，四个读法因此暂在 RefRain 自己的头一行；页签带接管读法时只搬这个控件。
-/

/-! D1 删除动作的形态是 dialog 确认，不是撤销 toast

- **决策**：删 MCP（及同族删除动作）经 `parts/dialog` 确认后才发帧——取消答案写在确认之下，撤不回来的那一问把安全的答案放在手下（4-20）。不引入 6s 撤销 toast。
- **理由**：撤销 toast 只在删除可逆时成立，而删除在今天的树上不可逆。删 MCP 是整表换写 `CONFIG.toml` 的 `mcp` 数组（`crates/city/src/config_layers/write.rs` 的 `write_mcp`），被删行不留副本；配置写不入账（`RulesChanged` 未落）；技能侧没有删除动词——library 只读，`Command::PutShelved` 以 `not_built` 拒，技能安装预检、来源记录与内容寻址存储都还没有。一个会自己落下的删除把不可恢复的内容交给无人看着的计时器。
- **被击败的备选**：6s 撤销 toast，撤销窗口过后才真正落删除——窗口内不发帧，撤销即取消，账上没有「删了又还」的一对。被败因只是今日删除不可逆；被删内容一旦可恢复，该形态即为首选。
- **重开参数**：被删内容留下可恢复副本——library 入内容寻址存储、来源有记录、同哈希重装幂等，或配置写留痕可还原。参数移动后按被击败备选的形态实现：撤销窗口过后才真正落删除，删除事件于落删除时入账，先落账再生效的口径不变。
-/

/-! D2 run 表是 `$state` 记录，不是 `SvelteMap`

- **决策**：`Belief.runs` 保持 `Record<RunId, RunBelief>` 的读法，由 `runTable` 包成 `$state`；token delta 就地写进那个 run。
- **理由**：两者给同样的逐键粒度——读 `runs[id].saying` 的 effect 只因这个 run 这个字段重跑，遍历表的读者只因 run 的增删重跑（`belief/grain.svelte.test.ts` 判定）。记录的写法让十余个按 `runs[id]`、`Object.values(runs)` 读表的视图一行不改。在 R = 1e4、一帧 50 个 delta、一个读全表的订阅者下，每帧折叠从约 470–540 µs 降到约 30–40 µs（`belief/fold_cost.test.ts`，同一仪表前后交错测），因为 delta 不再让订阅者走一遍表。
- **被击败的备选**：`SvelteMap<RunId, RunBelief>`。粒度相同，但每个读者都得改成 `get`／`values()`，且对已有键 `set` 新值时，有遍历读者就会连带推进迭代版本。
- **重开参数**：视图改由 belief 暴露的派生索引读表（不再直接下标）时，表的容器可以换，读者迁移的成本就不再存在。
-/

/-! D3 「在干活的 run」是 belief 维护的索引，不是每个视图的过滤

- **决策**：belief 在折叠时维护 `Belief.live`（没冻结的 run，按开始时刻排），视图读它；「run 在房间或其下」只有 `within` 一个拼写。
- **理由**：七个视图各自对整张 run 表过滤、排序来回答同一个问题，每次发布都付 O(R)，而且「在干活」与「在房间下」各有四份拼写，改一处不会带动其余。在干活的 run 受并发上限约束，远少于表里的 run，所以一次折叠只按那一个 run 改写 O(L) 的索引，读者付 O(L)。
- **被击败的备选**：读时计算的 `$derived`（按表派生）。它仍在每次发布时走一遍表，只是把重复的代码收成一份，代价不变。
- **重开参数**：在干活的 run 数与表的大小同阶时（L ≈ R），维护索引不再比读时过滤便宜。
-/

/-! D4 房间的 run 是 belief 维护的按房间索引，存 `RunId`

- **决策**：belief 按房间维护 run 的 `RunId` 列表（`Belief.rooms`），房间页、目录、城市面板与天际线经 `heldIn`／`heldWithin` 读它。
- **理由**：四个视图各自对整张 run 表过滤、排序，每次发布都付 O(R)；房间的历史含已冻结的 run，`live` 答不了。表里的 run 每次折叠都换成新对象，索引若存对象就得每次折叠改写列表；存 `RunId` 时，只有 run 进表、换房间或换开始时刻才动索引。
- **被击败的备选**：存 `RunBelief` 对象的索引。每次折叠都要在列表里替换那个 run，O(k) 的复制发生在每个 token 上。
- **重开参数**：run 表不再每次折叠换对象（就地改写）时，存对象的索引不再多付复制。
-/

/-! D5 拒绝框的正文是城写的出路，不是按码查的原因

**决定：** `parts/notice.svelte` 把 `AxError.recovery` 画在标题下、折叠之外；`err_<code>` 的标题只说拒绝的种类。**理由：** 一个码在城里有多种原因，按码写死的标题（如「模型已冻结」）在「没选模型」时说错了原因，而把城的原话折起来，人会先去按那些与这次拒绝无关的按钮。`recovery` 是唯一知道原因的句子。**胜过的方案：** 给每个原因一个客户端文案——做不到，客户端只看得见码；按原因分码是服务端改线协议的事，那之后出路表才能按码给出「去设置」。
-/

/-! D6 派出去的活落在哪，由 `run_started` 的地址与任务原文认出

- **决策**：页面在发出 `dispatch` 时记下房间、任务原文和当时已知的 run；随后第一个满足三条的 run——发出时不认识、`RunStarted::task` 等于原文、记录的 `addr` 是发出的房间或它下面任一层的房间（按整段比较，`shopfront` 不在 `shop` 下面）——就是这次派的活（`core/landing.ts`）。落在别的房间时，原地留一行「这次 run 去了 `<地址>`」，链到那间房（`talk/landed.svelte`），不自动跳走。
- **理由**：`runtime::run::lifecycle` 写 `run_started` 时带上房间地址（`addr`）和原样的 `task`，所以不动线协议就能认出；在楼的地址上派活时，城按规则开 `<楼>/<房间>`，人若留在楼的对话页就会对着一间空房。留一行而不是跳走，是因为人可能正要在原房间里接着写，一次不请自来的跳转会把焦点和草稿带走。
- **被击败的备选**：`run_started` 带上派活帧的 `IdemKey`，按键精确认领。它更严：两个页面同时往同一栋楼派同一句话时，今天的认法两边都会认第一个起来的 run（两个都是这个人自己派的，所以链接不会指向别人的活）。它要改线协议、`WIRE_V` 进位，归到改线协议的那一组。
- **重开参数**：同一栋楼里同一句任务的并发派活成为常态（例如一个页面批量派活），或城开始改写任务原文（去空白、加前缀），就按被击败的备选改为按 `IdemKey` 认领。
-/

/-! D7 浏览器通知只报需要人决定的事，默认关闭

- **决策**：`core/notify.ts` 的纯函数只从 `approval_queue` 的答里挑新到的待批事项；四道闸（失焦、预热期、快照里的旧事不算新、正在看的地址不弹）全过才交给 `views/notifier.svelte` 发出。设置页「外观」组里的开关默认 `off`，打开时才向浏览器要权限，开关只存在这个浏览器（`sprawling.notify`），因为通知权限本身就是每个浏览器各自授予的。
- **理由**：通知打断的是人在别处做的事，所以只配给「没有人就停下」的那一类——run 的进度、完成与拒绝都不需要人回答，已经由标签页的标题与图标承担。预热期与快照闸挡住的是同一个错：页面刚打开或重连时，答里的每一件事对这一页都是「第一次见」，却不是新发生的。
- **被击败的备选**：对每个完成、每个拒绝也发通知，或默认打开。前者把需要回答的那一条淹在不需要回答的里面；后者让浏览器在人还没理解这一页时就弹出权限请求。
- **与页面内投递的分工**：浏览器通知只管人不在这一页时（窗口失焦）；人在这一页时，什么在什么时刻冒头由 D26 判定。两者不重叠：审批在页面内从不延迟，在页面外只经这里的四道闸。
- **重开参数**：城开始产出第二类必须由人回答、却不经 `approval_queue` 的事（例如一个问句），这张表就要把它也读进来；或者通知改由城经 Web Push 发出（页面关闭时也要报），开关就该跟着偏好一起存进城。
-/

/-! D8 对话里没有 `/goal`

- **决策**（由人选定）：页面的斜杠表不设 `/goal`。
- **理由**：目标已经有两处权威——楼的常设目标（`set_pursuit`／`pursue`，`crates/sprawling/Spec.lean` §8-35）与每次派活的 `run_started.goal`。对话里再开一个入口，就是同一事实的第三处定义，三处之间没有东西把它们绑在一起。
- **被击败的备选**：`/goal <text>|pause|resume|clear`，把 `/goal x` 翻成 `pursue{step:{set:{goal:"x"}}}`。
- **重开参数**：常设目标与派活目标合并成一处权威时，对话入口可以指向那一处而不增加定义。
-/

/-! D9 只看结果模式里，人取消的 run 不进任何一类

- **决策**：`outcomeOf` 把 `completion = done` 记为刚完成，`cancelled` 不列，未名的结局（`completion = null`）记为「已结束」，其余冻结（`limit` 与此后新增的词）记为失败；在跑的 run 不列。
- **理由**：前三类是人要处理的东西。取消是人自己做的，结果他已知道。`belief.adopted` 把 `RunSummary.completion` 填进冻结的 run，所以重载后结局照旧；「已结束」只剩流与答都没看到 `run_frozen` 的 run（热视图只见一段尾巴时），把它们记为失败会把完成了的 run 说成失败，所以单列一类，不声称不知道的结局。
- **重开参数（已结束）**：服务端的热视图总能看到每个 run 的 `run_frozen` 时，「已结束」永远为空，可以删去这一类。
- **一行末尾写什么**：刚完成写产出与 `RunBelief.pr`（PR 以分支为名，城里的 PR 没有编号）；等你写 `RunBelief.ask`（`approval_requested` 的 `action_desc`，下一条记录即清空，与 `RunSummary.ask` 同一规则）；失败写 completion。
- **被击败的备选**：把取消也记为失败——会把人自己的动作当成要他处理的事，挤掉真正失败的前 N 条。
- **重开参数**：取消可以由人以外的一方发起（例如预算或上级 run 撤回）时，那部分取消应当按失败列出。
-/

/-! D10 没有问题认领的回答不报错，等着的问题照常以 `E_TIMEOUT` 超时

- **决定**：`asking.answered` 两轮匹配（在途表与迟到表）都认不出的回答直接丢弃，不给人任何提示；仍在队里的问题由超时扫描报一次 `ask_late`／`E_TIMEOUT`，出路是「重连」。
- **理由**：同一次构建的页面与城在连接期间也收到过认不出的回答，把它报成 `E_WIRE_MISMATCH` 会对一个人无法处理的事说「版本不同」。
- **代价**：页面与城真的漂移时，人看到的是超时与「重连」，而重连修不好漂移；能修好它的「重新加载页面以取得客户端」这时不出现。
- **被击败的备选**：超时扫描发现等待期间来过认不出的回答时，改报重新加载的出路——同一次构建里认不出的回答也会触发它，把一次普通的超时说成需要重新加载。
- **重开参数**：线协议让回答带上发问方的构建标识时，认不出且构建不同的回答直接报 `E_WIRE_MISMATCH`，这条取舍随之删去。
-/

/-! D11 `src/` 里的组件一律按 runes 模式编译

- **决策**：`client/svelte.config.ts` 的 `vitePlugin.dynamicCompileOptions` 调 `runesFor(filename)`：文件在 `client/src/` 下时答 `{ runes: true }`，其余文件（`node_modules` 里的组件）答 `undefined`，由编译器照默认按组件推断。`svelte.config.test.ts` 判这条分界。
- **理由**：推断模式下，一个没用到任何 rune 的组件按旧语法编译——`export let` 是 prop、顶层 `let` 是响应式——所以一个组件删掉最后一个 `$state` 就会悄悄换一套语义，而两套语义在同一棵树里并存。按路径打开 runes 让本包的每个组件只有一种语义：旧语法在 `src/` 里是编译错误，不是另一种读法。
- **被击败的备选**：`compilerOptions.runes: true`。它也作用于 `node_modules` 里的 Svelte 组件（`svelte/types/index.d.ts` 的 `runes` 选项说明），一个仍用旧语法的依赖会编译失败；本包今天没有这样的依赖，但运行时依赖由 `RUNTIME` 管，是否引入它不该取决于它用哪套语法。维持推断（不设）则留着上面那个静默换语义的口子。
- **重开参数**：Svelte 把 runes 设为默认、不再推断时，这个函数与它的测试一起删掉。
-/

/-! D12 视图栈是 Svelte 5，打包器是 Vite；对手是 Solid

- **决策**（换栈由人选定）：视图写成 Svelte 5 组件，由 Vite 与 `@sveltejs/vite-plugin-svelte` 打包。
- **理由**：参数是构建产物的大小与每个 token 的更新开销，因为一个流式对话客户端在每个 token 上都要更新页面。Svelte 5 与 Solid 在这两件事上是同一类设计——响应式编译进产物、没有虚拟 DOM、一次更新只重算碰过的信号——所以两者只能由读数分高下，分类分不出来。仓库里的读数都是 Svelte 一臂：整个客户端（含随包字体与 `#/gallery` 夹具）gzip 后 <!-- xtask:begin budget_reading:frontend_artifact -->677,073 B<!-- xtask:end -->（`tools/xtask/budgets.toml` 的 `frontend_artifact`，`just build-web` 之后称整个 dist 目录）；R = 1e4、一帧 50 个 delta、一个读全表的订阅者时，每帧折叠约 30–40 µs（D2，`belief/fold_cost.test.ts`）。Solid 一臂的两个读数还没有（§3-4）。在读数出来之前，选择由两件已有的事定：`.svelte` 组件与 `core/` 的 `$state` 模块（`belief/runs.svelte.ts`）由同一个编译器处理，测试经 `scripts/runes.ts` 走同一条编译路径；4-1 的类型车道与 4-5 的 eslint 配置都按 `.svelte` 定型，换成 Solid 的 JSX 要换掉这两条车道。**组件生态不是参数**：`parts/` 的控件全部自绘（§9 判定），平台已给 `<dialog>`、Popover 与提示语义。Vite 保留的理由是产物：`@sveltejs/vite-plugin-svelte` 支持当前的 Vite 主版本，换打包器不改变一个产物字节，却要动 `crates/sprawling/build.rs` 读取的输出契约。
- **被击败的备选**：Solid（`solid-js` 与 `vite-plugin-solid`）。它输在上面两件编译与车道的事上，不是输在一个读数上。SvelteKit（仍是 Vite，外加路由、SSR、`load`、服务端 endpoint 与 adapter）。它的主要能力都假定有一个 JS 进程在服务端跑页面，而这里的服务端是 Rust：产物在构建时嵌进二进制，城原样答它，数据全走 `/ws`，能用的只剩 adapter-static 加 hash 路由。代价有三：URL 文法从 `core/route.ts` 的穷尽 `View` 搬进目录名与 param matcher，成为同一事实的第二个家；十几个同名 `+page.svelte` 违反「一个模块一个文件、按它拥有的东西命名」；`$app/*` 虚拟模块在 `bun test --conditions=browser`（4-2）下跑不起来。它能给的按路由拆包由动态 `import()` 给出（`#/gallery` 已是这样），PWA 的 service worker 读 Vite 的 build manifest 手写。
- **重开参数**：§3-4 的两个读数量出来以后，同一仪表、同一机器上 Solid 一臂的产物不到 Svelte 一臂的一半，或每帧折叠开销不到一半，就重新论证本条。出现第一个真正需要组件库的需求时，先过 §9 的 `RUNTIME` 判定与 7-9。SvelteKit 一条在以下任一情况出现时重开：客户端改由一个 JS 服务端托管（例如远程门另起一个 Node 或 Bun 站点）；需要服务端渲染或预渲染的页面；页面数多到手写的 `route.ts` 本身成了负担。
-/

/-! D13 随包的第三方许可文本由打包器从产物里认出

- **决策**：`client/scripts/notices.ts` 给 Vite 一个 plugin，在 `generateBundle` 时从每个 chunk 的 `moduleIds` 里认出 `node_modules/<包>`（带 scope 的取两段，嵌套的 `node_modules` 取最后一段），读该包目录下的 `package.json`（版本、`license`）与包目录顶层的许可文件（文件名以 `LICENSE`、`LICENCE`、`COPYING` 或 `NOTICE` 开头，不分大小写），写成产物根下的 `THIRD-PARTY-NOTICES.txt`：按包名排序，同一个包的多个模块只出现一次，字节只取决于输入。二进制嵌入整个产物，所以这份文件随二进制分发，城在 `/THIRD-PARTY-NOTICES.txt` 答它。字体的许可仍是 `fonts/OFL.txt`，本文件开头指向它。一个进了产物却没有许可文件的包让构建失败，并点名它。
- **理由**：`svelte`、`effect`、`@lezer/*` 与 Svelte 运行时带进来的包是 MIT 或 Apache-2.0，两者都要求版权与许可声明随副本分发，压缩后的 bundle 也是副本；物料清单（`xtask sbom`）只列 cargo 包。从产物认包，而不从 `package.json` 或 `bun.lock` 认：前者漏掉传递进来的运行时包，后者把 devDependencies 与 tree-shaking 删掉的模块也算进去。包目录取自模块路径本身，而不是按包名到 `client/node_modules` 下去找：打包器读的是哪一份，声明就写哪一份。缺许可文件即失败而不是跳过：悄悄少了一个包的声明，与没有声明是同一个缺口。
- **被击败的备选**：一个现成的 rollup 许可证 plugin——多一个 devDependency 做几十行就能做完的事；把 npm 包写进物料清单——清单是 cargo 的 CycloneDX，且不在二进制里。
- **重开参数**：产物里出现一个许可要求别的形式的包（例如要求在界面上署名），或这份文件让 `frontend_artifact` 的读数增长超过 8 KiB。
-/

/-! D14 Markdown 不在浏览器里读，`RUNTIME` 不为它加一项

- **决策**：refrain 路线图 S7.9 要求二选一写进条目的那一项，选城侧：Markdown 由 `documents::markdown`（comrak）在城里读成块，页面经 `Query::Preview` 拿到块再画（4-26）；comrak 不编成 wasm，`RUNTIME` 与包体不因它多任何东西。
- **理由**：文档的版本在城里，页面只持有窗口，预览随一次往返就到，与它读 `Range` 是同一种代价；导出在 Rust 里读同一个函数，一个文法就只有一份构建。删掉 `prose.ts` 之后包体还少 1–2 KB，而 `frontend_artifact` 的余量本来就只有几 KB。
- **被击败的备选**：comrak 编成 wasm、由页面加载。它省掉每一窗的往返，经远程门的设备上这一点更明显；但 wasm 的导出要 `unsafe`，工作区里只有 `crates/desktop/ffi` 可以有自己的 lint 表，多一个 crate 就要人的定规，工具链多一个目标，包体推断多 100 KB 以上，同一个 comrak 还要在两条构建路上各编一次（`crates/documents/Spec.lean` D20）。
- **重开参数**：远程门上一次往返量出来超过 100 ms（refrain 路线图 §5 的 `client_send_feedback`）而 D15 的做法消不掉它；或允许第二个带自己 lint 表的 crate 的定规出现。
-/

/-! D15 对话流按文字问城，页面不再自己读 Markdown

- **决策**：对话流的回复由城读成块，页面经 `Query::Reply { text, state }` 问（4-26，`crates/wire/Spec.lean` §8-75）；「还在说的回复里哪些块已经不会再变」由城判（`layout::closed`，`crates/documents/Spec.lean` D31），页面只决定什么时候问。`core/prose.ts` 与它的 `closedUpTo` 在页面接上这个入口时删去。
- **理由**：一个文法管文档、对话流与导出（refrain 路线图 §4-9）；页面手里本来就有回复的文字，流式时来自增量，从历史打开时来自 `model_returned`，所以一条带文字的查询两处都用，删掉 `prose.ts` 之后页面里没有第二个 Markdown 读法。收束的规则也搬进城里，因为它判的是文法里的块，留在页面就是规则的第二个家。
- **被击败的备选**：按版本问——回复要先另存进内容库（`crates/documents/Spec.lean` D30 的 (a)），从历史打开的旧回复没有版本；城在增量旁边带上块——增量可丢，漏一帧就少一块，历史也还要另一个入口（D30 的 (b)）；页面留着 `closedUpTo` 只把闭合的那一段送去——收束的规则在页面与城各写一份，围栏的认法已经不一样（D31 的③）。
- **重开参数**：远程门上量出的一次往返超过 100 ms，使流式期间块的出现明显晚于文字，那时流式这一半另加 D30 的 (b)。
-/

/-! D16 停止键只停眼前的 run，眼前没有 run 时只提示

- **决策**：Accel-.（`run.stop`）、palette 与输入框里的 `/stop` 都只对眼前的 run 发 `cancel`。眼前的 run 由 `core/in_front.ts` 的 `runInFront(belief, view)` 判定：对话页取这个房间最新的在干活的 run，run 页取地址里那个仍在干活的 run，其余页面没有。眼前没有 run 时，Accel-. 不发任何帧，toast 座位出一条 `info` 提示，标题 `no_run_in_front`，正文 `stop_whole_city` 写出 `/halt --all`；这条提示不进抽屉。
- **理由**：glossary 把 Halt 与 Cancel 定为两个动词：Halt 关闭一个范围、终止其中积压的活，Cancel 只停一次 run。这个键原先发整城 `halt`，palette 的 `/stop` 又取全城最新的在跑 run，所以人按文档按下 Ctrl+. 想停眼前这一次，得到的是整座城停摆，或是另一个房间的 run 被取消；run 页输入框里打的 `/stop` 则什么都不做。三个入口给出三种结果，所以「眼前」只在一个函数里判定，三个入口都读它。眼前没有 run 时，页面不替人猜要停哪一个，也不把一次按键放大到整座城，只写出整城的拼写，由人自己打。提示不进抽屉：抽屉存的是城对人说过的话，一次落空的按键是人自己刚做的事，记进去会让栏顶圆点显示未读。
- **被击败的备选**：一，眼前没有 run 时停全城最新的在跑 run，即 palette 原来的读法；它可能停掉人看不见的另一个房间的 run。二，打开一个在跑 run 的选择面；它多出一个只为这个键存在的部件，而人按停止键是要停，不是要选。三，提示里放一个 `/halt --all` 按钮；人刚凭反射按了停止键，角落里再给一个一按就停整城的控件，正是本条要去掉的放大。四，把提示铸成一条 `E_INVALID_ARGS` 拒绝，照页面自铸 `E_TIMEOUT` 的先例走拒绝通路；标题会说城看不懂这个请求，而城什么都没收到，`recover_e_invalid_args` 还会把写着 `/halt --all` 的那句挤进折叠。
- **重开参数**：一页上出现第二种「眼前」时（例如检视面展开的一次调用属于另一个 run，或一页并排两个 run），`runInFront` 改为读那一面自己的选择；`halt` 有了确认或撤销时，停整城的控件可以回到这条提示里。
-/

/-! D17 三档叫 zen、blend、panorama，偏好叫 tier

- **决策**（改名由人定）：三档的存储与线上拼写是 `zen`／`blend`／`panorama`，中文是专注、混合、检阅；偏好的名字是 `tier`。
- **理由**：一个名字一个概念。`focus` 在页面代码里已经是 DOM 焦点（`focus()`、`:focus-visible`、`composer.focus`），`tier === "focus"` 与 `box.focus()` 会在同一个文件里相遇；`survey` 已经是 `browser::survey`，render 门量几何的那个读者。`zen` 是编辑器里「只留正在写的东西」那一档的通行叫法（VS Code 的 Zen Mode）；`panorama` 在仓库里没有别的意思。`layer` 已是世界层、对话层、边缘层的名字，档说的是世界层画多少，是另一个概念。
- **被击败的备选**：`focus`／`blend`／`survey`；第三档叫 `overview`——`route.ts` 已把 `#/overview` 读作城页。
- **重开参数**：`zen` 或 `panorama` 在仓库里有了第二个意思。
-/

/-! D18 发送与停止是一个键的两面

- **决策**（由人选定）：对话框只有一个动作键，朝上的一面由「眼前有没有 run」与「框里有没有字」判定（`docs/frontend-method.md` §7I）。
- **理由**：发与停从来不同时有意思——框里有字时这一按是说话（run 在跑就是 steer），框空而有 run 时这一按只能是停。两个并排的键让慌乱中的一按有一半机会落错；一个键在手下永远只做脸上写着的那件事，手不用换地方，翻面的动画让人看见它刚换了意思。
- **被击败的备选**：独立的停止键，run 在跑时出现在发送键旁边。它让「停」不依赖框空不空；这一点由 Accel-. 承担，它在任何时候都停眼前的 run（D16），框里写着半句也一样。
- **重开参数**：出现框里有字时也常要一键停的用法，例如边写下一句边看着 run 跑偏。
-/

/-! D19 其余页面由设置树到达

- **决策**（由人选定）：城、楼、记录、成本、登记簿、MCP 与性能这些页没有常驻入口，从设置面的设置树到达，树最多三级（7L）。
- **理由**：对话页常驻的只有工作内容、参数与事实（4-8）。一列八个页面字形是每天扫视的成本，换来的是一周几次的到达；这些页的到达是一次查找，查找该有分组——城的事实、接入、运行、偏好、诊断各是一类，树说得出这几类，平铺的一列说不出。
- **被击败的备选**：外壳原先那一列页面字形，三态展开；只靠 Ctrl-K——键盘之外到不了，触屏没有路。
- **重开参数**：某一页成了每天多次打开的页；那时它该在对话页或世界层里得到一个位置。
-/

/-! D20 缓动按值抄自 Open Props，时长取 Tailwind 的命名空间

- **决策**：`--ease-arrive`／`--ease-leave` 是 Open Props 的 `--ease-out-4`／`--ease-in-4` 与 `--ease-spring-1`，按值写进 `theme/tokens-motion.css`，出处与版本写在 `docs/frontend-method.md` §4-43；三档时长拼作 `--transition-duration-short|panel|page`，类名 `duration-short|panel|page`；原先的第三条曲线 `--ease-standard` 删除，它的每个读者按方向改读 arrive 或 leave。
- **理由**：Open Props 是 refrain 1-3 筛过的候选里唯一形状相符的一类，而它与主题不重叠的只有缓动这一片；为二十行 CSS 装一个包，`RUNTIME` 要多一项、`xtask motion` 要认第二个家，而这些值装进来之后也不会再变。时长取 `--transition-duration-*` 是因为 Tailwind 的 `duration-*` 类只在这个命名空间里找令牌：路线图写的 `--duration-*` 拼法让 `duration-short` 解析不出任何 CSS，又不报错。`ease-standard` 回答的是「在原地换状态」，而 Q1 已经说了到达一律减速，它是同一个答案的第二个家。
- **被击败的备选**：一，装 `open-props` 只导入 `props.easing.css`——它把四十多条曲线写进 `:where(html)`，页面多出四十个没有读者的名字；二，留 `--ease-standard` 给悬停与按下——三条曲线让每个写过渡的人多一次选择，而 Q1 的读法里那一次选择的答案永远是 arrive；三，每个读者照旧写自己的毫秒数——同一种位移今天有 90、100、120、150、200 ms 五个答案。
- **重开参数**：出现一个只含缓动与动画、可以单独导入、愿意被 `xtask motion` 当作第二个家的库（refrain 1-3）；或 Tailwind 换掉 `--transition-duration-*` 这个命名空间。
-/

/-! D21 线程里一次调用是一行，整行打开右侧；不折叠，不在线程里画参数与输出

- **决策**：对话线程把每次工具调用画成一行（种类、主体、用时、结果），排在那个回合的正文之下；整行是打开右侧检视面的按钮（4-44、7-11）。调用的参数与输出不再画在线程里，回合的调用也不再收进一个按类计数的折叠摘要。
- **理由**：一次调用的完整内容属于右侧，那里按调用组织（refrain Q2、§3-4）；线程只说工作的形状，所以一行要说出种类、对象、用了多久、成没成，而这四样一眼读完。按类计数的摘要（「读了 7 个文件，跑了 4 条命令」）要一张从工具名到类别的表，那张表是 `kernel::ToolMeta` 的替身（4-26）；一行一调用直接读每次调用自带的 `render` 与 `effect`，客户端不再需要类别表。行在跑时画计时器、落账后画账本的毫秒（refrain U12），折叠起来的调用看不到这两样。
- **被击败的备选**：一，保留折叠、展开后列出调用并内联参数与输出：同一份输出在线程与右侧各画一次，一个事实两个家，长输出还把正文推出视口。二，折叠摘要加逐行列表两层：多一次点击才看得到计时器，而在跑的那一行正是人最想看的。三，每个回合只画最后一次调用：跳过的行让 ↑／↓ 走不到它们，右侧也就打不开它们。
- **重开参数**：一个回合的调用常常多到把正文推出一屏（例如一次读几十个文件）时，按回合给超过某个行数的调用加一个「其余 n 条」的收起，收起的行仍可由 ↑／↓ 与右侧到达。
-/

/-! D22 检视面的 diff 读两个检查点之间的 `Hunks`，不读 edit 结果里自带的 diff

- **决策**：一次改文件的调用在检视面里画成它前后两个检查点之间这个文件的 `Query::Hunks`（4-45），不解析 edit 工具写进结果的 `{ path, base_version, new_version, diff }`。
- **理由**：两个提交 id 永不改变，答复可以一直留着；行号属于城持有的一棵树，所以「路径:行」与删去行的「路径@旧提交:行」（7-2）都指向一个人能再打开的地方；工作树有没有离开那棵树由一次 `Changes { base: 后一个检查点, head: null }` 判定，`reachOf` 因此知道该给链接还是给可复制的位置（4-39）。结果里的 diff 受 `Output` 的裁剪约束，长的改动会被切掉，而它的旧侧是一个 B3 版本，不是一个提交。
- **代价**：检查点跟在一波之后，所以 diff 覆盖调用所在的整波，同一波里改同一个文件的另一次调用也在里面；那一波还没有检查点时 diff 画不出来，只说「下一个检查点后」。
- **被击败的备选**：解析结果里的 diff（`monitor/trace.ts` 已这样读，监视器要的正是每次调用自己的改动）：精确到这一次调用、检查点之前就有，但被裁时不完整，行号对着一个不在任何提交里的版本。
- **重开参数**：城按调用写检查点，或 `Call` 带上这次调用自己的 `Hunks` 定位（两端是内容库里的版本而不是提交）时，diff 改读这一次调用自己的改动。
-/

/-! D23 RefRain 的编辑器是 CodeMirror 6 的最小组合

- **决策**：RefRain（7N）的编辑区是 CodeMirror 6，只取五个包：`@codemirror/state`（文档与改动集）、`@codemirror/view`（视图、输入法、选区）、`@codemirror/commands`（撤销历史与键表）、`@codemirror/search`（查找与替换）、`@codemirror/merge`（两段文本之间的 diff 与对照）。不取语言包与 `basicSetup`。五个包只由 `views/refrain/` 读，整块懒加载：第一次打开一份文档时才下载。编辑器的颜色写在 `theme/refrain.css` 末尾 RefRain 的那一块，盖过 CodeMirror 自带的基础主题，所以颜色仍只有一个家；编辑器自带的字句（查找面板、对照的提示）经 `EditorState.phrases` 取自 `lang.json`。
- **理由**：4-46 要的四件事平台给不了：一份可编辑、按视口画的长文本（一个 4 MiB 的 `<textarea>` 在输入时整份重排）；一个能精确说出「从基线到此刻改了哪几段」的改动集（`<textarea>` 只有整份的值，改动要事后比对，而比对给出的区间不一定是人做的那一下）；不被程序改动打断的撤销栈（给 `<textarea>` 赋值会清掉浏览器的撤销）；在长文本里查找与替换（浏览器的查找不进 `<textarea>`，也不能替换）。CodeMirror 的改动集按 UTF-16 位置给出、带 `mapPos`，正是 `core/document_pos.ts` 换算的另一头；它的输入法处理在三家引擎上有自己的测试。它替换的是本客户端原本要手写的这一套编辑面与一张行级 diff（`@codemirror/merge` 的 `diff` 同时给「移到现版」用，4-46）。许可证都是 MIT，在 `deny.toml` 的清单上。
- **被击败的备选**：①`<textarea>`——上面四件事各缺一件；②`contenteditable` 加自写的模型——输入法、选区与撤销要自己在三家引擎上重做一遍，那正是 CodeMirror 已经做完的；③Monaco——体积大一个数量级，要 worker，按 refrain 路线图附录 F 落选；④ProseMirror——富文本的文档模型，Markdown 要先解析成树再序列化回去，源文字节保不住（A1）。
- **读数**：五个包与它们带进来的 `@codemirror/language`、`@lezer/common`、`@lezer/lr`、`style-mod`、`w3c-keyname`、`crelt` 在一块懒加载的分块里，首屏不付；`frontend_artifact` 称整个 dist，引入前后的读数由整合记进 `tools/xtask/budgets.toml`。
- **重开参数**：Markdown 源码要语法着色时，加 `@codemirror/lang-markdown` 还是复用 `parts/paint.ts` 的 lezer 块，先过 7-9；或者出现一个同样给出改动集与输入法保证、体积小一半的编辑器。
-/

/-! D24 外壳按白银比切分；工作台的栏宽以栏计，存在浏览器，不入城

- **决策**：外壳按窗口宽切一刀，侧 : 主 : 侧 = 1 : √2 : 1，十二栏按 3、6、3 放进三份，第 4、10 条栏线即白银线；每一层只切一刀，主为 √2、次为 1：混合档会话 : 对话 : 提交 = 1 : √2 : 1，检阅档默认会话 : 所选会话 : 提交同为 1 : √2 : 1，所选会话 : 其下的对话（高）= 1 : √2，设置面与分栏的页面站在同两条白银线上。比例的唯一权威是 `theme/colour.css` 的 `--silver`。工作台的三栏顺序与宽度仍是 `core/workbench.ts` 的 `Workbench`——三栏各占几栏、之和恒为 12、每栏不窄于两栏（`NARROWEST`）——由 `prefs.ts` 的 `workbench`／`setWorkbench` 存在这个浏览器的 `sprawling.workbench`，`setWorkbench` 不发 `PutPreferences`；拖动吸附到栏线，键盘一次一栏；默认 3、6、3。
- **理由**：这是人定下的：主区用白银比，而不是平均分布的网格，三个方向在 1920×1080 下对照之后选定这一个。把十二栏放进白银三份而不是另立一套比例，栏线仍是唯一的对齐对象（`docs/frontend-method.md` §4-33）：栏边落在栏线上由构造保证，宽度仍是整数栏数，`Workbench` 的不变式一字不改，白银点就是栏线；切口量在窗口上（连页边距），中间一份因此正好以屏幕中线居中，侧栏均分剩余，页面在任何页边距下都对称。存在浏览器而不入城：栏宽是这块屏幕的事实——同一个人在笔记本与外接屏上要的宽度不同——而城的偏好记录说的是这个人，`PreferencePatch` 也没有这一臂。
- **被击败的备选**：一，B「白银轨道」——把 12 条等宽轨道换成 8 条按 √2 递进、左右镜像的轨道，最统一，但每一页、每块已定的屏的每条边都会移动。二，C「12 栏取最近整数」——混合 2 | 7 | 3、检阅 3 | 5 | 4 不变，√2 只用在纵向，横向只是近似，栏仍等宽，而人要的是不平均的网格。三，以像素、`fr` 或分数存工作台宽度（不吸附栏线）：栏边会落在两条栏线之间，世界层、对话与右侧不再共用栏线。四，随偏好入城：要给 `PreferencePatch` 加一臂、让 `WIRE_V` 进位，换来的是在另一块屏幕上打开一个不合那块屏幕的排布。
- **重开参数**：人改变比例或方向（改 `--silver` 一处即可）；外壳不再是 12 栏；或人要求工作台排布跨浏览器一致（那时给 `PreferencePatch` 加 `workbench` 一臂，`setWorkbench` 像 `setPanel` 一样出城）。右侧打开时的排法不归本条，见 D72。
-/

/-! D72 右侧打开时占第 8–12 栏，对话左移两栏

- **决策**：右侧打开时从第 8 条栏线排到第 13 条（右边一份加中间一份的后两栏），对话让出这两栏、站在第 2–7 栏（线 2 到 8），第 1 栏仍是边缘键的；混合档的会话栏随地方栏收起，检阅档会话栏与所选会话按人的顺序取 2 与 5 栏。这条线只在 `views/workspace.svelte` 的 `RIGHT_FROM` 写一次，三档都读它。右侧关着时对话仍在中间一份（D24）。
- **理由**：右侧装的是文件、diff 与信件，读的是等宽字；右边一份在 1920 px 下约 518 px，只容约 60 个等宽字，一行代码或一段 diff 常被折断。加上中间两栏约 90 个字。对话在第 2–7 栏仍宽于它 760 px 的阅读栏，所以对话左移而不变窄。人要的是视觉协调，不是栏线本身（roadmap SR）；选第 8 条线是因为在 1440 与 1920 上右侧与对话按同一比例随窗口缩放，两块的边又与世界层的栏边对齐，两种宽度看着是同一个排布。
- **被击败的备选**：一，对话不动、右侧只占右边一份（对话 : 右侧 = √2 : 1）——60 字装不下代码。二，右侧占中间一份的一半（线 7 到 13），对话只剩第 4–6 栏或要跨到左边三栏——前者窄于阅读栏，后者压到边缘键的第 1 栏。三，以主题里的像素令牌给右侧定宽——1920 上合适的宽度在 1440 上把对话压到阅读栏以下，换窗口时两块的比例漂移；而且对话的线也要由它推出，一个 CSS 令牌与 `layoutOf` 的栏线就成了同一件事的两处判定。
- **重开参数**：人要对话在右侧打开时不动；或右侧的主要内容不再是等宽字；或外壳不再是 12 栏。
-/

/-! D78 规则组用原生 `<select>` 选楼，不用楼列

- **决策**：规则组（`settings/rules.svelte`）选楼用一个原生 `<select>`，项是 `shared/buildings.ts` 的 `buildingsOf`：hall 在首、其余按名排。
- **理由**：一行的楼列读作标签而不是控件，一栋楼的城就打不开选择；`<select>` 一项或五十项都是同一个控件，它的键与读屏行为由平台承担。
- **被击败的备选**：技能组与 `#/mcp` 用的楼列（`shared/buildings.svelte`）——在只有 hall 的城里只有一行，看不出可选。
- **重开参数**：规则组要同时比较两栋楼的规则；或楼多到需要搜索（那时换成 `parts/combobox`）。
-/

/-! D80 更早的一段在两种模式下同一条折叠规则

- **决策**：房间里最新一段 session 之前的 run（`talk.svelte` 的 `earlier`）在「全部」与「只看结果」两种模式下按同一个答案画（`views/talk/earlier.ts` 的 `earlierDrawn`）：没有更早的 run 时不画；开着的这一段已经有自己的 run 时折叠——全部模式折在分隔线后，只看结果折在「更早的会话」标题后；开着的这一段还没有 run 时展开。人按下折叠行之后，这个房间里的这一次以人的选择为准。
- **理由**：新开的一段把前一段放进「更早的」；全部模式与只看结果必须给出同一个展开判定。一段刚开、还没说话时，人要看的正是上一段；一旦这一段有了 run，上一段就让位。
- **被击败的备选**：一，两种模式都一律折叠——新开的一段还没有 run 时看不见上一段；二，两种模式都一律展开——长房间每一段都摊开，分隔线失去意义。
- **重开参数**：过去的段（`talk/past.svelte`）成为看上一段的主要入口时，重新评估自动展开。
-/

/-! D81 面的第二层在第一层走到一半时开始

- **决策**：一张面进来时，它里面的第二层晚半个时长开始，两层读作一个动作的两步。右侧（`views/right.svelte`）是 `side-in` 与 `side-in-then`（`page` 时长，延迟 `page / 2`）；设置面（`views/settings/panel.svelte` 的 `.settings-panel`，`panel` 时长的平移）里的组一栏（`settings/sheet.svelte` 的 section）是 `slide-then`（`slide` 关键帧，`panel` 时长，延迟 `panel / 2`）。时长、曲线与延迟只写在 `client/src/theme/motion.css`，视图只写类名，`xtask motion` 守住。只在面挂载时走一次：在设置树里换组是 `shift`，不是第二层再进来一次。
- **平台**：这些是普通的 CSS 动画加 `calc()` 延迟，Windows、macOS 与 Linux 上的 Chromium、Firefox 与 Safari 都支持，画法相同；`prefers-reduced-motion: reduce`（除非 `data-motion="on"`）与 `data-motion="off"` 在每个平台上都把它们关掉，面直接到位。
- **理由**：两层同时进来时，组的文字与面的边一起滑动，眼睛没有先落的地方；先让面立住一半，再让内容跟上，人先看见面从哪来，再读里面是什么。
- **被击败的备选**：一，用 JS 定时器在面的 `transitionend` 之后挂载第二层——第二层晚一整个时长才有，键盘焦点与读屏器在空面上停一拍；二，第二层不动——面的边与内容一起平移，与右侧的两步读法不一致。
- **重开参数**：设置面不再从左边平移进来（例如成为一页），或右侧与设置面的时长令牌合并成一个。
-/

/-! D82 性能页的柱子按窗口自己的量程画，读数仍与 `sprawling top` 同一规则

- **决策**：性能页（`views/monitor.svelte`）每个计数器画一排柱子（SVG，一个点 3 px），由 `core/monitor.ts` 的 `plot` 定高：窗口里最小的值站在 `FLOOR`（80‰，最低的柱子也看得见），最大的站满 `FULL`（1000‰），中间线性；全部相等时站在一半；柱子后面是窗口 p50 到 p99 的一带，读数旁写出这两个数（最近秩，与 `bin::monitor::spread::Spread` 同一取法）。读数本身（单位、截断、千分比）仍按 `crates/sprawling/spec/Monitor.lean` §8-95 的规则写，`monitor.test.ts` 与 `monitor::top::tests` 用同一组样本对照；终端的八级字符曲线只留在终端。
- **理由**：页面原先把终端的八级字符照搬过来，一条曲线只有一个字形高，监视器要看的变化挤在几个像素里。字符是终端的限制，页面没有这个限制；读数要两边一致，因为人与 agent 读同一个样本要得到同一个数，画法不必一致，因为高度从来不能跨行比较（§8-95 决定 1）。
- **被击败的备选**：一，对数刻度：它把相差几个数量级的值放进一张图，可监视器的一行只画一个计数器，要看的是它自己在一个窄带里的起伏，在 3.1 GiB 到 3.2 GiB 之间抖动的工作集在对数刻度上与从 0 起的线性刻度上都是一条平线；二，继续用字符、只放大字号：高度仍只有八级，柱子之间的差别仍是一级一级的。
- **重开参数**：一行要同时画几个计数器而需要彼此比较高度；或终端改用非字符的画法，两边的画法可以再合成一个。
-/

/-! D85 发信与派活的一行读参数，不读主体

- **决策**：`views/talk/call_kind.ts` 的 `kindOf` 对 `render` 为 `signal` 的调用给「发信」或「取信」（按参数 `action`），对 `delegate` 给「派活」；`lineOf` 从 `Call.arguments.head`（参数的 JSON，`wire::arguments_in` 排好的）读出 `to`／`text` 或 `room`／`task`，一行画成「发给 <to>：<text 开头>」与「交给 <room>：<task>」；结果一栏读工具的答复：`send` 答了 `waiting` 读作「等回信」，否则「已发出」，`delegate` 读作「本回合后开始」。参数被截断或读不成 JSON 时退回 `subject`，与没有登记的旧行一样不编造。正在跑的调用把时长放在自己的一格里，主体一格 `min-w-0` 截断，两者不再叠在一起。三个平台上是同一段 TypeScript 与同一份 lang.json，没有平台分支。
- **理由**：登记为 `Generic` 时这两行读作「写入 send」「派生 <goal>」，人看不出信给了谁、活交给了哪个房间（kernel D37，collab D17）。地址与任务就在参数里，读参数是唯一不需要第二个权威的办法。
- **被击败的备选**：一，按工具名分支——§4-26 拒掉的第二张表；二，让线上 `Call` 多带 `to`／`room` 字段——同一个事实在参数与新字段里各写一遍，且要进位 `WIRE_V`。
- **重开参数**：投递结果（落进正在跑的 run、进队列、敲门开新 run）有了自己的账本行（collab D17），那时结果一栏改读那一行。
-/

/-! D83 委派来的活在线程与会话栏里写出是哪个居民交下的

- **决策**：线程开头那一句任务（`talk/thread.svelte`）的署名按 `Opening.dispatched_by` 定，判定是 `talk/naming.ts` 的 `dispatcherOf` 一处：`person` 与缺席（旧账本）写「你」，`city` 写「由城派来」，居民地址写「由 <居民> 派来」，居民名经 `called` 取；署名链接到派活的那个 run（`Opening.parent`，`#/run/<parent>`，那一页就是父会话的那一段），`parent` 缺席的旧账本链接到居民的房间（`#/talk/<地址>`）。会话栏里一段以 `dispatched` 开始、且由居民交下的会话，在标题下写同一句（`talk_dispatched_by`），读这一行自己的 `SessionStart::Dispatched { by }`，同一个 `dispatcherOf` 判定；会话栏不为此问任何 `Rounds`。三个平台上是同一段 TypeScript，没有平台分支。
- **理由**：委派来的子房间以前把父 Agent 写的任务署成「你」，人会以为那是自己说过的话。派活者与父 run 都已在线上（wire D40），所以链接能落到派活的那一段，而不是只到房间；会话栏读行上的 `by`，就不必为每个派活开始的会话再问一次 `Rounds`——那样一页有几十段时就是几十个问题，且在回答到之前那一行什么都不说。`kernel::event::Who` 不许居民用 `person` 或 `city` 作地址，所以先认这两个词不会把居民认成另一方。
- **被击败的备选**：一，会话栏仍按行问第一个 run 的 `Rounds`——同一个事实有了第二条读路，且依赖本页恰好持有那个 run；二，把派活者折进 `RunBelief`——那是效应核 `core/belief` 的接口，重载后要从 `RunSummary` 再读一次，而 `RunSummary` 不带它；三，链接到父房间的当前一段——父房间之后可能已经 `/new`，那一段不是派活的那一段。
- **重开参数**：线上能从 run 直接给出它所在那一段的 `Seq` 时，署名链接改到 `#/talk/<地址>:<seq>`，在对话里而不是 run 页读父会话。
-/

/-! D86 居民的话画成左侧的信，等回信与交回各有自己的一行

- **决策**：`Note::Arrived` 的 `by` 为 `user` 时仍画 `talk/person.svelte` 的气泡，署名「你」；为 `resident` 时画 `talk/letter_note.svelte`：左对齐、不填底、左缘一道 accent 的卡，头一行是「来信」、发信居民的名字（`naming.ts` 的 `called`，链接到 `#/talk/<地址>`）与时刻，其下是原文。带 `handback` 的到达画 `talk/handback_note.svelte`：「<子房间> 交回：完成（由 … 验证）」或「停下，因为 …」，子房间的名字链接到那一段的 run 页（`#/run/<session>`）。`Note::AwaitingReply` 画 `talk/reply_wait_note.svelte`：在等时写等的房间与剩下的时间（`timing.ts` 的 `ticker` 每秒走一次，过了期限写「已到期限」），结束时按 `ReplyEnded.by` 三种各一句、各一种墨色：回信 accent、超时 alert、对方离开 text-faint。`talk/inbox.svelte` 一行写种类（`signal_kind_<kind>`，表里没有的种类原样写出）、首行（`SignalLine.first_line`，缺时不写）、发信房间与距今多久，整行链接到发信房间；条数一条时用单数句。三个平台上是同一段 TypeScript 与同一份 lang.json，没有平台分支。
- **理由**：居民的话以前画成人的气泡，人会以为那是自己说过的话（UC3）；形状不同才不靠标签区分。`Note::Arrived` 不带 `SignalKind`，所以信卡只写「来信」，不猜是哪一种；发信 run 也不在线上，所以链接到房间而不是那一段，与 D83 在 `Opening.parent` 缺席时的取法相同。
- **被击败的备选**：一，在气泡上换一个标签——同一形状的两种话仍要读字才分得开；二，在页面里按 `from` 是不是 `user` 判断——`by` 已在线上，第二处判断就是第二个权威。
- **重开参数**：`Note::Arrived` 带上信的种类或发信 run 时，信卡头改写种类、链接改到那一段。
-/

/-! D88 停在同步 `send` 上的 run 有自己的姿态：在等谁回信、还剩多久

- **决策**：`core/doing.ts` 的 `Doing` 多一臂 `awaiting_reply { wait: Waiting }`，`Waiting { on, until }` 就是线上 `RunSummary.waiting` 的那个类型（wire D34），客户端不另拼一份。流的一路：`belief/fold.ts` 读到 `signal_wait_started` 时按载荷（`on`、`deadline_ms`，经 `core/reading.ts` 的 `waitOf`）进入这一臂；读到 `signal_wait_ended` 时由 `afterWait` 一处决定离开到哪里——`reply` 与 `timeout` 回到 `thinking`（run 从安全点接着走，下一次模型调用会再说一次），`left` 到 `unknown`（run 离开了房间，说它怎样结束的是随后的 `run_frozen`）；姿态已不是这一臂时（例如 `run_frozen` 先到）结束行不改它。答案的一路：`belief/adopted.ts` 在 `RunSummary.waiting` 在场时给这一臂，缺席而本页仍以为在等时给 `unknown`，因为答案比本页新、又不陈述别的姿态。等待中发出的话是 `queued`：run 停在 `send` 的工具调用里，与 `calling` 同理。画法：run 板的阶段多一个 `reply`（`send` 字形、live 墨色、accent 填色），信箱「在跑」一行与 run 板的一行写「在等 <房间> 回信，还剩 <时长>」，过了时限写「已过时限」，词都来自 `lang.json`，时长按页面的同一只钟（`talk/timing.ts` 的 `ticker`）。三个平台上是同一段 TypeScript，没有平台分支。
- **理由**：一个停着等回信的 run 以前在信箱与 run 板上读作「在想」，人分不出它是在推理、卡住了，还是在等另一个居民；等谁与时限都已在账本那一行与线上的摘要里，不需要新的线协议。
- **被击败的备选**：一，借用 `waiting`——那是「等你批准」，信箱会把它算进需要你的事；二，只在线程里画（`Note::AwaitingReply`）——信箱与 run 板不问 `Rounds`，那样它们照旧说「在想」；三，把结束的方式留在 `RunBelief` 上画一行「已回信／超时」——那是线程注记已经说的事，在两处画就是两个说法。
- **重开参数**：一个 run 可以同时等两个房间时（wire D34 的重开参数），`wait` 改成一张表，这一臂跟着改。
-/

/-! D89 性能页的采样节拍是一个五档的分段控件，选中即发，读数到了才换档

- **决策**：性能页（`views/monitor.svelte`）页头右侧一个分段控件，五档 50、100、250、500、1000 ms，选中即经 `core/watching.ts` 的 `setBeat` 发 `{"monitor":{"beat":<n>}}`（wire D46）。控件停在哪一档由最近一个读数的 `Sample.beat_ms` 决定：发出之后、城按新节拍采到的读数到来之前，控件停在原档；城当前的节拍不在五档里时（另一台设备设了别的数）没有一档选中。所有看这座城的人共用一个节拍，城记住它（`crates/sprawling/spec/Monitor.lean` D48），看的人都走开也不回到默认。键盘与读屏照分段控件的契约（§7-4）。三个平台上是同一段 TypeScript。
- **理由**：节拍是城的状态，不是本页的：控件读城说的值，就不会出现「控件说 250、城还在 100」的样子；五档盖住要看短峰（50 ms）、默认（100 ms，M0 第 7 条）与长时间看趋势（1 s）三种用法。
- **被击败的备选**：一，一个输入框，可填 10–1000 之间任何数——多数人不知道该填多少，而线上的范围已经挡住了填错；二，控件自己记住所选的档——城没答应之前就显示成已改，另一台设备改了节拍时两边各说各的；三，最后一个看的人走开时回到默认——User 要的是「采样频率可调」，每次打开性能页都要再调一次，就等于不可调。
- **重开参数**：User 要求在五档之外的节拍时，改成档位加一个自定义值。-/

/-! D90 发信一行画信落在哪里；信件卡写种类、链接发信的那一段

- **决策**：`views/talk/call_kind.ts` 的 `outcomeOf` 对 `render` 为 `signal` 的调用先读 `Call.landing`（wire D42）：`delivered` 写「已投进正在跑的 run」，`queued` 写「进了队列」，`knocked` 写「敲门开了新 run」，同步 `send` 还在等回信时也写，因为那时信已经投下；调用失败时不写落点；`landing` 缺席（落点行之前的账本、落点在窗口外）时仍写工具自己答的「已发出」或「等回信」（D85）。`talk/letter_note.svelte` 的头一行在 `Note::Arrived.kind` 在场时写种类（`talk/inbox.ts` 的 `kindSaid`，与信箱一行同一个读法），缺席时写「来信」；`Note::Arrived.session` 在场时多一个「发信的会话」链接到 `#/run/<session>`，发信居民的名字仍链接到房间。`talk/handback_note.svelte` 的子房间名链接到 `Arrived.session`，缺席时链接到子房间。三个平台上是同一段 TypeScript 与同一份 lang.json，没有平台分支。
- **理由**：D85 与 D86 的重开参数都已成立：投递结果有了自己的账本行（kernel D38），种类与发信 run 也随到达上了线（wire D43）。落点是投递那一刻城的判定，工具答复时还不知道；页面读那一行，而不是从 `run_started` 反推。
- **被击败的备选**：一，落点与工具的答复并排写两样——同一件事（信去了哪里）两个说法，`delivered: true` 读起来像 `delivered` 那一臂；二，信件卡的名字改链接到会话——人点名字通常是要去那个居民的房间，会话是另一个去处，所以各占一个链接。
- **重开参数**：一封信可以同时落进几个房间（wire D42 的重开参数）时，结果一栏改成按房间的一列。
-/

/-! D91 信件的「对话」读法只画提出这张卡的那一回合与前后各一回合

- **决策**：`views/inspect/letter.svelte` 的「对话」读法把 `ProposalCard.offered`（wire D44）交给 `talk/thread.svelte` 的 `around`；`thread.svelte` 有 `around` 时只画 `talk/around.ts` 的 `turnsAround` 选出的回合：含这一行的那一回合（`opened` 不大于这一行的最后一回合）与它前后各一回合，开场的任务照画，因为它是这个 run 被要求做什么；被略去的回合不画，画一行去发信 run 页面的链接（`letter_whole_talk`），整段会话在那里。`offered` 缺席、或这一行早于第一回合时画整段。三个平台上是同一段 TypeScript，没有平台分支。
- **理由**：A25 要的是发信 run 的相关对话：卡是在哪一回合提出的、之前读了什么、之后说了什么。整段画出来，一段几十回合的会话要人自己找那一处；前后各一回合够说清「为什么」，再多就回到整段的问题。
- **被击败的备选**：一，画整段、滚到那一回合——右侧的读法是一个面，滚动位置在换读法时丢掉，回来又是从头；二，只画那一回合——看不到它为什么提这张卡。
- **重开参数**：人常常要在信里读到更远的回合时，`AROUND` 改成一个可调的数。
-/

/-! D93 一个供应方的账号编辑器分成接线、座位与外观，顺序就是优先级，每次改动发出整张列表

- **决策**：Provider 卡每一行折叠一个账号编辑器，摘要写账号数；网络搜索卡的每一家供应方折叠同一个编辑器（D94）。编辑器不认识端点或供应方，只认一份 `Roster`（`views/setup/providers/rosters.ts`）：拥有这张列表的是谁、城答复的列表与各账号的 Key 状态、一把 Key 存进 vault 的哪里、header 名是否站在表单里，以及载着整张列表的那一帧。`rosters.ts` 造两种：端点的（`endpointRoster`，帧是 `reattachEndpoint`，Key 存在 `referenceFor(provider, account)`）与搜索供应方的（`supplierRoster`，帧是 `configureSearch` 发出城那一层的整个 `[search]`、只换这一家的 `accounts`，Key 存在 `searchReferenceFor(supplier, account)`，即 `secret:search/<supplier>.<account>`；搜索的 Key 只经账号点名的 header 送出，城拒绝有 Key 而没有 header 的账号，所以这一种的 header 站在表单里而不折叠）。接线是两份不碰 DOM、不用 rune 的文件：`views/setup/providers/account_editor.ts` 定义编辑器状态，判定每一次按下改什么、发什么；`accounts.ts` 判定哪个控件现在不可用及原因，并造出外观要画的整份值 `AccountsLook`；`accounts.svelte` 是座位，持有编辑器状态，把套接字、vault 的 `/enroll` 路由、语言和两个焦点落点借给接线；`accounts.look.svelte` 是外观，只收 `AccountsLook`，所有文字都已译好、所有按下都已接好，所以换一套外观（例如 UI 库组件写的）画出的是同一个编辑器。**交互契约**：列表是 `<ol>`，每行依次是序号、账号 id、Key 状态（`EndpointSummary.account_status`，wire D49：已存入、缺失、由环境变量提供（只读）、环境变量已设但不可用、不用 Key、还没查看）、上移、下移、编辑、移除，四个都是原生 `button`（`parts/button.svelte`）；第一行的上移、最后一行的下移、只剩一个账号时的移除、Key 正在存入时的每个控件都不可用，并由 `why` 说出原因（4-18 的提示，而不是不写原因的 `disabled`）。上移或下移之后焦点留在被移动那一行的同一个控件上；移除之后焦点回到编辑器标题（`tabindex="-1"` 的 `h4`）。添加或编辑表单：账号 id 只收 `ServerLabel`（编辑时只读），Key 是密码框，经 `core/enrol.ts` 存进 vault，名字只由 `referenceFor(provider, account)` 拼成 `<provider>.<account>`（`ServerLabel` 不含 `.`，按最后一个 `.` 拆回两个 id 没有歧义），账号带回城答复的引用；已有引用与 header 名折叠在「引用与 header」之下。存入在途时有一张票：编辑器关闭、供应方、城或配对换了之后才回来的答复不落到任何地方；存入被拒或命令没能离开页面时草稿保留并说明原因。移除一个 Key 已存入 vault 的账号之后，编辑器多一行：说这把 Key 还留在 vault 里，旁边一个按钮「同时删除 Key」；按下发出 `ForgetSecret`（`core/enrol.ts` 的 `forgetSecret`，带账号原来的引用），这一行在下一次按下任何控件时消失。它排在移除那一帧之后从同一条连接发出，城按到达次序执行，所以城看到它时列表里已经没有这个账号；列表被拒时它也因「还有登记在用」被拒，拒绝照样写在列表旁边。Key 由环境变量提供的账号不给这一行，因为城会拒（gateway D33）。不在移除时一并删：一把 Key 可能还被别的供应方、别的账号或手写的配置引用，删不删要人在看到这一行时决定。每一次改动——保存、移动、移除——都经 `core/commands/endpoint.ts` 的 `reattachEndpoint` 发出完整列表：tuning 按城读回的样子（`EndpointSummary.tuning`）原样发回，只换 `accounts`，已服务的模型重新 admit，旧的单 Key 与 header 不发（gateway Router D29）。发出的列表在城对这个端点的下一个答复到来之前一直画着，所以第二次移动从第一次的结果出发；城拒绝时画回城的列表，拒绝写在列表旁边。一行说明：新 Session 取列表里第一个可用账号、成功后一直用它，顺序只决定新 Session 和出错后的换号（搜索供应方的说明另写：搜索不保持账号，每次都从第一个可用账号起）。端点有两个以上账号时，编辑器多一个分段控件「同一账号的重试次数」（1／2，`EndpointTuning.account_retries`）：选中的是城读回的值，没有值时两格都不选、旁边写城的缺省（`Query::Config` 在 hall 答的 `TuningDefaults.account_retries`，页面不写死这个数）；按下发出同一个 `reattachEndpoint`，列表不变，只换这一项。只有一个账号时这个数不被读（gateway Router D28 之后的 kernel D54），所以控件不画。端点还只有接入时的那一个 Key 时多一行，说保存账号后列表会取代它。`client/swap/views/setup/providers/accounts.look.svelte` 是第二套外观（原生元素、不用工具类、样式全在自己的 `<style>` 里并只读主题令牌），只收同一个 `AccountsLook`：把它换进 bundle 之后接线测试与 `cargo xtask render` 仍须通过，这是这个编辑器的替换测试。三个平台上是同一段 TypeScript，没有平台分支。
- **理由**：列表位置就是优先级（gateway Router D28），整数优先级会多出「并列」这个没人要的状态；`AttachEndpoint` 整体替换 tuning，只发 `accounts` 会清掉人填过的超时、header 与覆盖，所以发回的必须是城读回的整份。接线不碰 DOM，接线测试（`accounts.test.ts`）不 import 任何外观，就能对两套外观都成立；焦点落点由座位持有，因为只有它拿得到元素。
- **被击败的备选**：一，拖拽排序——键盘与读屏要另写一套，上移、下移两个原生按钮已经覆盖；二，账号放进一张独立的页——人在 Provider 卡上读到的「这个供应方有几个账号、哪个缺 Key」就要跳页才看得到；三，外观直接调命令与 `enrol`——换一套外观就要把接线再写一遍，两份接线第一次分歧就是一个缺陷。
- **重开参数**：人要在账号之间分摊用量（那时顺序不再是唯一的选择规则），或者账号多到上移、下移要按很多次（那时加「移到第一」）。
-/

/-! D94 网络搜索卡：城那一层的 `[search]` 一次整份发出，自定义只接选中的那一家

- **决策**：Provider 页的 accounts 组在 Provider 卡下方画一张网络搜索卡，分成接线、座位与外观，与 D93 同一种切法：`views/setup/search/search_editor.ts` 是卡的状态与每次按下发什么，`search.ts` 造出外观要画的整份值 `SearchLook`，`search.svelte` 是座位（在 hall 问 `Query::Config`，读 `ConfigAnswer.search`；调用方可以递进一份 `SettledSearch`，画廊这样用），`search.look.svelte` 只画。卡编辑的是城那一层（`SettledSearch.city`），不是这个地址生效的值：一栋楼写了自己的 `[search]` 时（`from` 是 `building` 或 `resident`），卡上多一行只读说明，写出生效的是哪一种、来自哪一层，卡上的改动不改它。**交互契约**：三格分段控件「默认／自定义／关闭」（`parts/segmented.svelte`，键盘契约见 `spec/Views/Parts/Segmented.lean`）；默认格旁写城答复的缺省服务地址（`SettledSearch.default_url`，页面不写死）。选默认或关闭立即发出 `ConfigureCity { search }`；选自定义时，城那一层已经是自定义就什么也不发，列表为空就只打开添加表单，第一家保存时发出 `Custom { selected: 这一家, suppliers: [这一家] }`，因为 `Custom` 的列表不能为空（city D25）。从自定义切到默认或关闭之后，这一页记着刚才的自定义值，再选自定义就把它整份发回，而不是让人重填。自定义时有一行说明：选中的服务失败时不回落到 Exa。供应方是一张列表，每行依次是 id、地址、remote 工具名、「使用这一家」、编辑、移除，都是原生 `button`；选中的那一行的「使用这一家」与只剩一家时的移除不可用，并由 `why` 说出原因；移除选中的那一家也不可用，原因是先选另一家。每行折叠这一家的账号编辑器（D93 的 `supplierRoster`）。添加或编辑表单：id 只收 `ServerLabel`（编辑时只读），MCP 地址、remote 工具名、query 参数名必填，count 与 objective 参数名可空；保存发出整个自定义值，这一家原来的账号原样带上。每一次发出——选档、选家、保存、移除——都是城那一层的整个 `[search]`，经 `core/commands.ts` 的 `configureSearch`；发出的值在城的下一个答复到来之前一直画着；城拒绝时画回城的值，拒绝写在卡上；命令没能离开页面时草稿保留并说明。另有一行说明 confidential 楼不提供网络搜索。卡不碰 Key：账号的 Key 由折叠的账号编辑器经 `/enroll` 存进 vault。
- **理由**：`ConfigureCity.search` 是整值替换（wire §8-61），只发改动的那一家就要城去合并，城就有了第二份「列表长什么样」的判断；城那一层与生效值分开答（wire `SettledSearch`），卡就不会把楼的覆盖当作城的值改写。切走自定义时记住上一份值，是因为 `Default` 与 `Off` 在文件里不留供应方，回到自定义时要人重填一遍地址与参数名，代价落在最常见的来回试上。
- **被击败的备选**：一，用单选按钮组画三档——分段控件已是本页的三选一部件（`city_layer.svelte`），第二种画法要第二套键盘契约；二，供应方表单里直接写 Key——Key 属于账号，同一家有几个账号就有几把 Key，表单里一格写不下；三，卡也编辑楼的覆盖——楼的 `[search]` 是那栋楼的文件，在 accounts 组里改它要一个选楼控件，而楼的覆盖本来就少见。
- **重开参数**：楼的覆盖变得常见（那时卡按楼读写），或接入方式超出 MCP streamable HTTP（那时表单按接入方式分栏）。
-/

/-! D25 设置面开着是一个地址，开在一页之上

- **决策**：设置面开与关由地址栏说：`#/setup[/<组>]` 时外壳画下面那一页加设置面，别的地址时没有设置面；在树里换组替换地址，不压历史；关上时若这一页压过这个地址就后退一步，否则替换成下面那一页（7L）。
- **理由**：refrain 路线图 §3-14 要求浏览器历史与界面不各自维护一套栈，刷新恢复设置分组。地址是两者共有的唯一状态：书签、外部链接、刷新、后退键与 `<a href>` 都经同一条 `hashchange` 进来，所以「面板开着」不可能与地址栏说的不一致。换组替换而不压历史，是因为人在设置里来回看几组之后按后退，要的是离开设置，而不是倒着重看。
- **被击败的备选**：一，面板开合只是外壳的一个状态，不进地址——刷新就丢，外部链接打不开某一组，后退键关不了它而去了上一页，界面与历史各有一套栈。二，设置仍是一个页面（`#/setup` 取代下面那一页）——那是 D19 已经否掉的读法：设置面打开时人还在原来那一页上，关上要回到那里。三，每次换组都压一条历史——后退键要按很多次才离开设置面。
- **重开参数**：移动端的 sheet 需要「后退先退出眼前的面」以外的栈语义（MOB 的 U9），或者设置面里出现了需要单独可后退的第二层（例如一组里打开一份文档）。
-/

/-! D26 普通通知只在三个时刻主动冒头，等人的事与停住工作的故障不等

- **决策**：城回来的一条拒绝先按码分成两类（`core/deferral.ts` 的 `urgencyOf`，一张按 `AxCode` 穷尽的表）：`needs_you` 是不等人就动不了的那一类——一个门在等人的动作（`E_APPROVAL_PENDING`）、没有模型或凭据、一个供应方的每个账号都失败了、配置读不通、账本与内容库的完整性坏了、两端的线上格式不一、预算用尽、常设目标缺计划——它一到就以 toast 出现，并进信箱的「待决」段，计入信箱键上的数；其余是 `ordinary`，它一到只在信箱键上点一个不带数的标记，toast 等到三个时刻之一才出现：人刚发出一句（框是被发送清空的，且之后一直空着）、框空着已满 5 s（`IDLE_MS`）、刚切回这个标签页（`RETURNED_MS` 1 s 之内）。标签页藏着时没有时刻。页面上没有对话框时（对话页之外的各页）人不在框里写字，任何时候都是时刻。人自己按下的键落了空（D16 的停止键）不是通知，是对那一按的回答，从不延迟。一条拒绝若由打开的对话自己画在回复处（`talk/handing.ts` 的 `claims`），它不进 toast，与延迟无关。
- **理由**：通知打断的是人手上那句话。Horvitz、Apacible 与 Subramani 2005 的 bounded deferral 把不紧急的提醒压到下一个断点再给；Iqbal 与 Bailey 2008 在真实任务里量到断点处的打断代价最低，而写完一句、按下发送正是一个断点，停笔几秒也是。框里的字就是本页对「人在不在写」能读到的唯一事实，所以时刻按框读：发送清空的框立刻算断点，因为那一句已经交出去了；手删空的框等 5 s，因为删空常是重写的开头。切回标签页是人重新看这一页的那一刻，1 s 足够让藏着期间排下的帧在可见后的第一帧折完（4-11）。等人的事不延迟，因为它们停着的正是人的那份工作：延迟它们只是让人晚几秒发现自己才是那个卡点；它们也不抢焦点，只出现在人看得到的地方。
- **被击败的备选**：一，按严重性三档延迟——严重性是码的种类，延迟只问一件事：不等人这件事会不会自己动，两档就答完了；二，把延迟做进 `socket.ts` 或 belief——那会让信箱与记录也晚知道，而延迟只该管主动冒头，不该管事实何时被记下；三，用 `document.hasFocus()` 代替框的状态——焦点在框里的人可能正在读，焦点不在框里的人可能正在别处打字，焦点说不出断点。
- **重开参数**：框之外出现第二个人写字的地方（右侧的 RefRain 编辑器有了未存的草稿时，那里也是一个「正在写」），`Box` 要读那一处；或者测得人在 5 s 内常常重新开始写字、被弹出的 toast 打断，`IDLE_MS` 随读数移动。
-/

/-! D27 楼页的改动行开在右侧，读对话页的同一份右侧状态

- **决策**：楼页「改动」的一行按下就调 `openDocument`；楼页读 `views/inspect/open.svelte.ts` 的同一份右侧状态，有打开的项时在第 8–11 栏装 `views/right.svelte`，与对话页同一份画法（4-50 其三）。
- **理由**：一个文件从哪里打开都是同一个右侧项：人从楼页回到对话页时它仍开着，从对话页来到楼页时也看得到它。右侧开着什么只有那一份状态说了算，楼页是它的第二个读者，不是第二个家；画法也只有一份，楼页另画一份只带关闭键、不带页签带的右侧，草稿圆点、键与焦点的规矩就得写两遍，第一次只改了一处时两页就说出两种右侧。楼页画全部打开的项而不只画本楼的文件：只画一部分是对「开着什么」的第二种回答。
- **被击败的备选**：其一，按下就跳到对话页再开右侧——人离开了他正在看的楼，回来还要再找一次那一行。其二，在正文里就地展开这个文件自上一个检查点以来的补丁——`Query::Hunks` 要两个提交，工作树不是提交，所以这条路今天走不通。
- **重开参数**：`Query::Hunks` 接受工作树作为一端；那时改动行可以就地展开补丁，右侧只留给要读写整份文件的时候。
-/

/-! D28 run 页打开时停在哪个透镜，由 run 的状态定

- **决策**：不带透镜的 `#/run/<id>` 打开时，在跑的 run 停在 time；已结束、并在开场之后提交过检查点的 run 停在 changes；已结束而没有改动的 run 停在 turns。人选过一个透镜之后，这一页留在他选的那个。带透镜的 `#/run/<id>/<lens>`（§3-2）停在它写的那个；提交行与 `/diff` 写的是 changes。
- **理由**：人从三处来到一个已结束的 run——对话里 run 结束的那一行、提交行与 `/diff`——三处要看的都是它改了什么，原来一律停在 time，人要再找到第六个页签（S07 的 E9）；一个只回答了问题的 run，changes 只会说没有改动，它说了什么在 turns 里；在跑的 run 要看的是时间花在了哪。判定只读页面已经问到的 `RoundsAnswer`（开场的检查点与最后一个检查点），不多问一次。
- **被击败的备选**：一律停在 time，即原来的读法；记住这个浏览器上次选的透镜，它把上一个 run 的问题带给下一个 run。
- **重开参数**：人常常从一个已结束的 run 的链接再换到另一个透镜（读数是透镜切换紧跟在到达之后的比例）时，重看不带透镜的那几处入口该写哪个透镜。
-/

/-! D29 客户端升到 Effect 4，Effect 的边界不随之扩大

- **决策**：`effect` 钉在 4.0.0；`cargo xtask wire-ts` 发出 Effect 4 的 `Schema`（`tools/xtask/Spec.lean` §8-52）；`Either` 换成 `Result`，`parseJson` 换成 `fromJsonString`，热帧的窄校验直接跑字符串节点上的 `checks`（4-6）。链路的退避与问答仍是纯状态机，不改用 `Schedule` 与 `Effect.retry`。
- **理由**：人要求在 Effect 4 发布后积极使用它，用在它确实买到东西的地方。4.0 的 `Schema` 把检查挂在节点的 `checks` 上，热路径因此不必再拆 refinement 节点；`Result` 与 `Option` 的形状在 4.0 统一了命名。退避与重试那一半见 4-6 的论证：两者今天都是不带时钟的纯函数，按一次 `step` 一个断言就测完了，换成纤程买到的只有一层运行时。
- **被击败的备选**：①留在 3.x，只生成 4.0 的 schema——一个包两个大版本不能同处一份清单；②用 `Schedule` 重写 `core/link.ts` 的阶梯——见理由；③生成器发 `Schema.Literal` 多参数的写法——4.0 里多值字面量是 `Schema.Literals([...])`，单值才是 `Literal`。
- **重开参数**：客户端里出现第二条要组合超时、取消与重试的异步流程（例如远程配对的握手），那时纤程买得到东西。
-/

/-! D30 「一栏」问外壳的容器，不问窗口

- **决策**：`narrow` 变体与 `frame` 的一栏规则是 `@container shell (width < 768px)`。`shell` 是页面的 `body`；画廊里外壳的标本各自以 `@container/shell` 当自己的 `shell`。脚本读 `frame` 写下的 `--shell-columns`，不再用 `MediaQuery("width < 768px")` 写第二遍。
- **理由**：外壳自己的页面上 `body` 就是窗口的宽，两种问法答案相同；差别在画廊。媒体查询问窗口，一个 390 宽的标本在 1440 的窗口里画成十二栏，手机的页面只能在 390 的真窗口里看；而 `xtask render` 在 768、1280、2560 三个宽度打开画廊，没有一个小于 768，一栏的页面从来不被量到。问容器之后，390 的标本在任何窗口里都是一栏，门的每一次打开都量它。脚本读 CSS 的判定，不自己写 768，因为两处 768 一旦不一致，网格是一栏而脚本按十二栏摆区域，正是 `docs/frontend-method.md` §4-33 要靠构造避免的不对齐。
- **被击败的备选**：留媒体查询、只在 390 的窗口里截图——门量不到一栏的页面；给 `render` 加一个 390 的宽度——门因此多一次整页打开，而容器问法不加一次就覆盖了。
- **代价**：`body` 带 `container-type: inline-size`，即带布局约束，成为 fixed 后代的包含块；外壳的 frame 已经是（`visual-viewport` 的 translate），所以没有一样东西换了位置。
- **重开参数**：外壳需要按设备而不是按宽度改变的东西（打印、`hover: none` 之类）时，那一项另用媒体查询，这一条不动。
-/

/-! D31 页面只送还没画成块的那一段，答复接在后面

- **决策**：一段回复的下一问只带页面还没画成块的那一段（`core/replying.ts`，4-26）；答复的块接在已画的块之后，不替换它们。还在说的一问只带那一段里的完整行。结算的回复先画流式时最后画出的块，等自己的答复到了再换成它（`views/reply.svelte.ts`）。
- **理由**：收束点只进不退（`crates/documents/Spec.lean` D31），所以收束点之前画出的块不必再问，接着问其余的部分就够了；每一问的字节因此只随新到的文字增长，而不随整段回复增长。收束点只落在完整行之后，带上没写完的最后一行读出的块也一样，所以还在说时只送完整行，没有新的一行就没有新的一问。结算时城从头读整段，把流式期间一段一段读的列表与晚到的引用定义读对（D31 的两处例外）；在它答到之前画流式的块，回复结算的那一刻就不会先跳回原文再跳回块。
- **被击败的备选**：①每一问都送整段、用答复替换已画的块——每一问的字节随整段回复增长，而且一个列表在流式期间画成一个、下一问又读成另一个，已经画出的块会变，违背「不收回」；②每到一个增量就问——绝大多数增量不带换行，城只会答出同一个收束点；③结算时直接沿用流式的块、不再问——两处例外在结算之后就永远读错了。
- **重开参数**：城在 `Delta` 帧旁边带上块（D30 的 (b)，D15 的重开参数）时，流式这一半不再问；或回复的典型长度超过一个窗口，使结算的那一问要多次往返才画完。
-/

/-! D32 PDF 用 pdf.js，DOCX 用 docx-preview，HTML 用浏览器加一个空 sandbox 的框

- **决策**：refrain S7.12 的三种格式：PDF 用 `pdfjs-dist`（Apache-2.0），DOCX 用 `docx-preview`（Apache-2.0，带进 jszip，许可证 `MIT OR GPL-3.0-or-later` 取 MIT 一支），HTML 不引库，用本浏览器与一个 `sandbox=""` 的 `iframe` 加注入的 CSP（4-54）。两个库只由 `views/refrain/formats/` 读，各自懒加载。pdf.js 的资源随 bundle 走、经页面一侧的资源门交给 worker：全部 Adobe CMap、FoxitSymbol 与 FoxitDingbats 两种符号字体、OpenJPEG、JBIG2 与 qcms 三个 WebAssembly 解码器；它们各自的许可证文件（BSD-3-Clause、BSD-2-Clause、MIT）由 `client/scripts/notices.ts` 从资源所在的目录读进 `THIRD-PARTY-NOTICES.txt`（D13）。不引 Mermaid（路线图 S05 Q2）与 Mammoth；KaTeX 由公式引入（4-64a、D42a）。
- **理由**：平台给不了 PDF 与 DOCX 的页：浏览器自带的 PDF 阅读器在 `iframe` 里不受页面控制，拿不到文字来比较，三家引擎的行为也各不相同（refrain 附录 F：原生 iframe 的内部控制不宜成为 diff 契约）；DOCX 没有任何引擎会画。HTML 正是平台本身会画的东西，库只会成为第二个画法。DOCX 画进沙箱框而不是客户端页面本身，因为文件里的样式会进入客户端的层叠，而 docx-preview 画 `altChunk` 用的是一个不带 sandbox 的框。资源经资源门而不是 `cMapUrl` 目录交给 worker，因为 Vite 只发出被导入的文件、不复制目录，而走 CDN 要向城外的主机发请求。标准 14 种字体里只随包两种符号字体：`useSystemFonts` 在浏览器里默认打开时，pdf.js 只为 Symbol 与 ZapfDingbats 取字体，其余用设备自带的字体；随 pdfjs-dist 发来的 Liberation 字体的许可证（GPL-2.0 加字体例外）也不在 `deny.toml` 的清单上。
- **被击败的备选**：①`<iframe src>` 指向 PDF、用浏览器自带的阅读器——见理由；②Mammoth 抽 DOCX——只给语义 HTML，不排页，「预览」名不副实；③docx-preview 直接画进客户端页面——文件里的 CSS 进入客户端的层叠，`altChunk` 开一个不带 sandbox 的框；④自写 PDF 或 DOCX 排版——这两个库已经做完的事，自写的那一份要在三家引擎上重测一遍；⑤pdf.js 资源走 CDN——城外的主机会知道有一座城打开了一份 PDF，没有外网的机器上什么都画不出（与 `theme/fonts.css` 不从字体主机取 Geist Mono 的理由相同）。
- **读数**：`frontend_artifact`（`just build-web` 之后每个文件 `gzip -9` 后相加）引入前 676,051 B，引入后 2,416,686 B，超过 `tools/xtask/budgets.toml` 记的 2 MiB；那一行是读数，不是门。最大的一块是 CMap（约 1.4 MB，`.bcmap` 本身已压缩，gzip 几乎不再缩小它），其次是 pdf.js 的 worker（约 1.27 MB，gzip 后约 0.35 MB）。分块与资源都懒加载，首屏一个字节都不多付。
- **重开参数**：整份 dist 的字节成为要守的数时，第一步是把 CMap 做成可选的资源包，由城从自己的目录提供，只在一份 PDF 要某一张 CMap 时才取；其次是只留中日韩常用的 UCS2／UTF16 那几族。一份文件没带标准字体、设备上又没有度量相近的字体而画错时，重新考虑随包 Liberation 字体，那要先在 `deny.toml` 上论证它的许可证。
-/

/-! D33 修改提案卡逐句决定，过期的卡在页面上先说出来，信箱读全城开着的卡

- **决策**：修改提案画成请决定卡的第三种（7C），正文是城切好的逐句 diff；e 打开的改后接受让人逐句取舍、改写插入句，判词在 `views/refrain/proposals.ts` 一处拼出；基线不等于文稿此刻版本的卡，y 与 e 置灰并写出两个版本，n 仍可按；信箱的待决段读城答的全城开着的卡（`Query::OpenProposals`），再按文稿问卡的正文（4-55）。
- **理由**：线上的决定本来就是逐句的（documents D16），一张只有「整张接受／整张拒绝」的卡会让人为了改一个词去拒绝整段，再请 run 重提一遍。过期是城对每张卡都会给出的同一个判定，而页面手上已经有判定所需的两个值（卡的基线与回答的版本）：先说出来，人不必按一次、读一条拒绝、再明白为什么。哪些卡还开着是城的折叠的事实（关掉一张卡的记录不带文稿，页面自己折数不出哪张关了），所以信箱问城要这张表，而不是从本页听到的记录拼。
- **被击败的备选**：一，卡上只放「整张接受／拒绝」两个答复，改写留给 RefRain 的编辑器——人改完字再存，存下的是一次普通的保存，账本里没有这张卡被改后接受的记录（A10 要它入账）。二，过期的卡照常给 y，让城拒绝——同一个事实（这张卡不能再接受）由城说一遍、由 toast 再说一遍，按键的人却是最后知道的。三，信箱逐页读账本找 `proposal_offered`——为了一个数把整份历史搬进页面，且 `proposal_decided` 读不回文稿，依旧不知道哪张已关。四，页面折自己听到的 `proposal_offered` 记下文稿——本页打开之前提出的卡永远不进信箱。
- **重开参数**：线上的卡带上提出的时刻时，卡头右端改写时刻。
-/

/-! D35 配对页是设置面的一组，不是一张单独的页

**决定**：设备一侧的配对、锁门与忘掉这台设备都画在设置面的「远程」组里（4-57），配对邀请的片段读作这一组（§3-2）；种子在配对成功之后画一次。**理由**：其余页面都由设置树到达（D19），城所在的机器与设备打开的是同一个组，一处说清远程门在这座城里是什么、这个浏览器手里有什么；一张单独的页（`#/pair`，或 `pair.html` 这样第二个入口）要么在 `View` 里多一种只在扫码时出现的页，要么多一个外壳，而设备配对之后用的终究是这个外壳。**被否**：①单独的轻量入口，给手机少下几十 KB——配对是一次性的，而第二个入口要自己的路由与样式；②种子在配对之前画——配对失败时那份种子对应一把谁也没钉住的密钥，人却已经抄下了它。**重开参数**：没配对过的设备打开远程地址成了常见的路（例如邀请改成先开页面、再在页面上输入配对码）时，重看设备一侧是否要一个在外壳之外的首屏；种子有了恢复的用处（设备 id 与城的公钥有了第二个家）时，重看种子的画法。
-/

/-! D36 「谁在听」并进房间芯片的菜单，不另开一个面

- **决策**：城、楼、居民与这次 run 被告知的四段，画在房间芯片菜单的头上、房间列表之上（`talk/listening.svelte`，由 `talk/pill.svelte` 的 `told` 插槽画进去）；每段写大小、读自哪些文件与被裁掉的字节，全文留在 run 页的 prompt 透镜。
- **理由**：「谁在听」回答的是「这句话发到哪」，房间芯片回答的也是它，人换房间之前正是要看谁在听的时候；放进同一个菜单，设置行不多一个控件（7-11 的常驻控件数由 `xtask render` 数着），也不在对话框下多一行常驻的字（P14）。菜单里只读不按，因为菜单的键归房间列表；四段全文是四份文件，菜单装不下，run 页已有它的读法。
- **被击败的备选**：①对话框下常驻一行「谁在听」——那是 `docs/frontend-method.md` §7I 撤下的旧样子，一行常驻介绍违反 P14；②芯片旁另放一个「i」按钮开一个弹层——多一个常驻控件，且与房间芯片说的是同一件事；③在菜单里展开四段全文——把一个选房间的菜单变成阅读器。
- **重开参数**：人要在不离开对话的情况下读某一段全文时（例如检视面加了「run 被告知的内容」这种条目），这一段的行变成在右侧打开它的入口，菜单本身不变。
-/

/-! D37 冻下的名字由城读成类型交给页面，页面不取内容库的字节

- **决策**：开始了的会话的名字读 `Opening.names`（城在作答时从内容库读回 `run_started.naming` 那一版，`crates/wire/Spec.lean` D17）；还没开始的会话读 `Query::Identity`。页面不经 `Query::Content` 自己取那一版的字节。
- **理由**：那份字节是 `city::Naming` 的编码，页面解析它就是同一种编码的第二个读者；内容库的回答是给人读的截断文字。名字在新会话生效（4-48），所以已经开始的会话写今天的名字就是在改一段历史。
- **被击败的备选**：①所有消息头都读 `Query::Identity`——改名之后旧会话跟着改名，与模型请求里冻下的名字不符；②线上带 `naming` 摘要，页面再问 `Query::Content`——多一次往返，页面多一个无类型的解析。
- **重开参数**：会话栏或信箱也要写冻下的名字时，城把名字折进视图按 run 缓存，页面的读法不变。
-/

/-! D38 run 策略是页面上的一个值，设置行上一个权限入口

- **决策**：页面把下一次派发的四个值作为一个 `RunPolicy` 持有（`Ui.policy`，模式从它的 `mode` 字段读取），设置行在会话开始之前用一个权限入口持有模式与写入限制两个开关，另两项在同一面内选择（4-60）。`/admit` 只改准入要求。
- **理由**：线上派发带的就是这一个值（wire D4），页面另持四个散的值会让 `/dispatch` 与发送键各拼一次。写入限制是一次派发最常改的边界（「只读可新建」），准入与试验是少数派发才动的（refrain 4-2「试验与准入选择放展开面」）；合着的键写出被改过的值，是因为一个看不见的试验会让人以为工作已经落地。
- **被击败的备选**：①模式与写入限制分开常驻——设置行多一个常驻控件（7-11 的常驻数由 `xtask render` 数着、`budgets.toml` 的 `talk_controls` 没有余量），且大多数时候它们说的是「什么也没加」；②准入与落地放进设置面——那是一个人的长期偏好，而这三个值按派发选（`settings/admission.svelte` 只解释它们）；③每次派发后把准入复位——模式与强度都留在本页，单这一项复位会让同一行里的选择有两种寿命。
- **重开参数**：人要一种派发后自动复位的选择时，复位规则写在 `Ui.policy` 一处，四个值一起定。
-/

/-! D39 文件的字节经 `Query::Bytes` 到页面，比较的导出由页面写成 Markdown

- **决策**：PDF、DOCX 与截图的字节由 `core/document_bytes.ts` 经 `Query::Bytes` 逐窗取回（wire D18），页面拼成一份 `Uint8Array` 交给画它的工具；一份 Markdown 的导出由城写（`Query::Export`，documents D33），两版比较的导出由页面写成一份 Markdown（4-61）。
- **理由**：字节走与其他回答同一条 socket，远程设备经封好的会话也拿得到，页面不需要第二种取法。比较是页面做的：两版的文字由 pdf.js 与 docx-preview 在浏览器里读出（D32），城不读 PDF 与 DOCX，所以把比较送回城去导出就是把页面已有的文字再传一遍、再由城拼一遍；Markdown 是一个会改 Word 的工具或一个 run 最容易照着做的形状，代码块保住改动的原文不被当成标记读。
- **被击败的备选**：①比较导出成 HTML，与 Markdown 的导出同形——HTML 给人看，意见要给工具照着改，工具读 Markdown 比读带样式的 HTML 省事；②城读 PDF 与 DOCX 的文字、在城里比较并导出——城要多两个解析器，与页面的两个读出两种文字，比较就有了两个答案；③截图另开一扇按定位取图的门——截图也是内容库里的一个对象，同一扇门按地址就够了，定位里只有整个对象的那一种拼法被读成地址。
- **重开参数**：一份文件经 `Query::Bytes` 送到页面的时间成为人能察觉的等待（wire D18 的重开参数）；或者导出的比较要带版式（页面的截图），那时比较导出改为一个带图片的包。
-/

/-! D40 设置组的首字母取它看得见的名字，名字的第一个字打不出时取它在地址栏里的名字

- **决策**：设置树里一个组的键是它的名字（`setup/groups.ts` 的 `HEADING`，随语言）的第一个字，这个字是一个键打得出的拉丁字母或数字时取它；否则取这个组在地址栏里的名字（`#/setup/<group>` 的 `<group>`）的第一个字母。按下一个字母，焦点到下一个以它开头的组，到尾绕回，几个组同一个字母时连按就挨个走过（APG 的 type-ahead）。行间走动的 gg 两次 g 之间至多 1000 ms（`SEQUENCE_MS`），与 vim 的 `timeoutlen` 默认相同。
- **理由**：APG 说首字母是名字的首字母，一个人按下的是他看见的那个字；英文里看得见的名字就是答案（「official harnesses」画 O，不画地址里的 H）。中文的组名第一个字要经输入法，按下时浏览器只报 `Process`，按不出来；地址栏的名字在每种语言里都一样，也是每个键盘都打得出的。
- **被击败的备选**：①一律取地址栏的名字——英文里「dependencies」旁边画 T、「which release this is」旁边画 A，画出的键与读到的字对不上；②中文里取拼音首字母——要在页面里带一张拼音表，而它只为十四个组服务；③给每个组在 `lang.json` 里另写一个键字母——每种语言一份第二拼写，换一个组名就要记得换它。
- **重开参数**：组名改成以一个打不出的字开头的英文（例如以引号开头），或加入第三种语言而它的组名用另一种拉丁之外的文字时，看这条规则是否还画出人读到的那个字。
-/

/-! D41 定位点名一项，到达时打开一次；地址栏不跟着右侧走

- **决策**：对话页的地址可以带一个右侧项的定位（§3-2），到达时经 `openCall`／`openDocument` 打开它；此后开着什么只由 `inspect/open.svelte.ts` 说，地址栏不随页签改写。一个项的定位就是它页签的 `href`，复制它用浏览器自己的「复制链接」。
- **理由**：§3-14 第 10 行要的是可复制、刷新能回来、不重放动作的定位。让定位只说「打开这一项」，右侧的状态就仍只有一个家（4-27）：地址栏不会因为人点了另一个页签而与右侧说出两样东西。到达时打开一次，人随后关掉它，刷新会再打开——这是链接的意思，与浏览器对一个片段锚点的做法相同。
- **被击败的备选**：其一，地址栏跟着前台项改写——地址成了「开着什么」的第二个家，每点一个页签就多一格浏览器历史，后退键变成在页签之间走。其二，一条单独的 `#/inspect/…` 路由——它离开了对话，而 diff 的引行要写进旁边那段对话的草稿，一个调用离开它的对话也说不清是哪段会话的。
- **重开参数**：人要用后退键关上右侧项，或一个链接要交出几个页签时，重看地址栏是否要写出整组打开的项。
-/

/-! D42 链路每次连接时选一条线：配对过的 `https:` 源走远程会话，其余走 `/ws`

- **决策**：`core/socket.ts` 不再自己 `new WebSocket(url)`，而是向 `Dial` 要一条 `Line`；`main.ts` 交给它 `core/remote/session.ts` 的 `dialFor(location)`，后者每次连接读一次这个源的设备记录，决定走远程会话还是 `/ws`（4-64）。会话的封与开各排成一列。
- **理由**：链路的状态机（`link.ts`）只关心「开了、收到一帧、关了」，这三件在两条线上是同一回事，所以缝开在线上，而不是让 `socket.ts` 认识远程门；两种实现都是生产路径，这条缝有它的第二个实现。每次连接都判，是因为配对发生在这一页上：开页时判一次的话，刚配对完的设备要重新载入才用得上会话，而链路本来就会在连不上 `/ws` 时按阶梯重连。
- **被击败的备选**：①开页时判一次（`main.ts` 先等 `kept()` 再开链路）——见理由，而且第一次绘制要等一次 IndexedDB；②远程监听把 `/ws` 也原样转给城、页面不封帧——设备的帧就只剩通路的 TLS，Cloudflare 在边缘解开 TLS，门的逐帧授权也落空（`crates/remote_access/Spec.lean` D2、§8-10）；③让 `link.ts` 的状态机多一个「握手中」的远程态——会话握手失败与 `/ws` 连不上对链路是同一件事，都走阶梯，多一个态只多一组转换。
- **重开参数**：远程源上需要线协议之外的 HTTP 门（录音、拖入文件）时，那几扇门要么也封进会话，要么由中继按动词类判后转发，`Line` 不变；一个源要同时连两座城时，设备记录从「每个源一条」改成按城的指纹存，`dialFor` 随之按城选设备。
-/

/-! D42a 公式输出 MathML、建成元素，不输出 HTML，不随包字体

- **决策**：`katex` 进 `RUNTIME`，只由 `views/refrain/formula.svelte` 懒加载；`katex.render(tex, 节点, { output: "mathml", throwOnError: true, trust: false })`，失败与未到时画原文（4-64a）。
- **理由**：KaTeX 的 HTML 输出要它自己的样式表与几十个字体文件，并且以字符串的形式最省事地用起来，而 4-26 不许任何东西经 `innerHTML` 进页面；`render` 以元素建出它的树，MathML 输出交给浏览器排版，三家引擎都已实现 MathML Core，字体用系统的数学字体。`throwOnError` 让一个不认的命令变成一个可判断的失败，页面据此留原文，而不是画出 KaTeX 自己的红字。
- **被击败的备选**：①`renderToString` 加 `{@html}`——一条 `innerHTML` 的路，4-26 与 documents D21 都排除；②HTML 输出加样式表与字体——首次画公式要多下载数百 KB 的字体，一份城里的文档就能让页面向资源门要几十个文件；③MathJax——更大，同样以字符串或它自己的 DOM 适配器输出；④页面自己把 TeX 翻成 MathML——那是第二个 TeX 文法。
- **重开参数**：某个引擎的 MathML 排版让常见的公式读不清（分式、上下标挤在一起）时，改用 HTML 输出并随包字体，那时字体经资源门懒加载；documents 在城里排公式（把 MathML 放进块树）时，页面不再需要 KaTeX，`RUNTIME` 删去它。
-/

/-! D43 中文断行交给浏览器：`theme/base.css` 的 `line-break: strict`，不移植 RefRain 的 `typeset.rs`

- **决策**：中文的禁则与标点处理写在 `client/src/theme/base.css` 的一条规则里：`:root:lang(zh)` 设 `line-break: strict`（行首不出现闭括号、句号一类的标点）、`hanging-punctuation: first allow-end`（开头的标点可悬进页边，句末标点可悬出行尾）与 `text-spacing-trim: space-first`（行首全角标点左侧的空白去掉）；根元素的 `lang` 就是人选的语言，所以一条选择器管住整页。refrain 路线图附录 B 列的 `typeset.rs`（字符分类、禁则、贪心断行）不移植，Rust 与客户端都不另写一个断行器。三个平台上都是这一条 CSS；不认识其中某个属性的引擎跳过那一行，照它今天的排法排，字体是设备自己的中文字体（`docs/frontend-method.md` §4-37）。
- **理由**：人看见的每一行都是浏览器用此刻装上的字体、此刻的栏宽排出来的；一个在 Rust 或 TypeScript 里算断点的贪心断行器拿不到这两样，它算出的断点到不了人看见的那几行，要生效只能往文字里插零宽断点或换行，而那些字符会随复制带走、也会与引擎自己的断行打架。CSS Text 的 `strict` 档就是禁则表，由引擎按 Unicode 换行算法维护，是这一事实唯一的权威。
- **被击败的备选**：移植 `typeset.rs`，在页面或城里按它的字符表与禁则预先断行——第二份断行规则，栏宽或字体一变就与引擎不一致，且只能经插入字符起作用。
- **重开参数**：出现一个浏览器不排版的中文表面（城在服务端排出的 PDF、画布、终端输出），或某个引擎的 `strict` 档在常见文本上让闭标点落到行首。
-/

/-! D44 会话栏一行一段 session，过去的一段只读；替换 session 只靠 `/new`，分叉是另一项功能

- **决策**：会话栏列每个房间的每一段，点一行把它放进主区；过去的一段只读，其下是回到当前一段的链接。对话里**替换** session——丢掉眼前的对话、在同一地址从空白开新的一段——只有一条路：打 `/new`（`/compact` 是 `/new --carry`，带上交接）；没有同义的 `/clear`，选模型也不顺带开新会话。**分叉**是另一项功能，不是替换：线程里每条消息、回合与工具行上的分叉键（`talk/fork_button.svelte`，键 Accel-Shift-F）、`/fork` 与信箱「最近」段的分叉入口（起点是那一段的末尾，`tailOf`），都从对话里的一行开出一条新的线，原来的对话一字不丢地留在账本与会话栏里。路由以 `#/talk/<地址>:<began>` 点名一段（§3-2、7K）。
- **理由**：这是人定下的规则：替换会把眼前的对话换走，一个顺手就能按到的控件或一个与它同义的第二个拼写，会在人没打算丢掉对话时把它丢掉，所以替换只留给明说的 `/new`；分叉不丢任何东西，人点分叉键正是要从那一行另起一条线，所以它的控件留在对话里。在过去的一段上「继续」只能是分叉，因为城只接着一个房间最新的一段说下去（glossary：Session），开新的一段即结束旧的。`:` 是地址文法拒绝的字符，作分隔不会与地址混淆；`began` 是 `Query::Sessions` 已答出的稳定序号。城在有 run 工作的房间里拒开新段（`E_BUSY`），所以一个地址上不会同时活着两段。
- **被击败的备选**：①保留 `/clear` 作 `/new` 的同义词——同一件事的第二个拼写，人读到两个动词会以为它们做的事不同；②去掉对话里的分叉键——把分叉当成了替换，人要从一行另起一条线时只剩打字；③在过去的一段上直接派活——城会把它接到当前一段上，人看到的不是他点的那一段；④一行一个房间、过去的段折在房间下——多一次点击，键盘路径更长；⑤过去的一段下再放一个「从它的末尾接着说」按钮——它只是分叉的第二个入口，人要从旧的一段接着说时，逐条的分叉键与信箱的分叉入口已经在手边，多一个按钮只多占一处常驻控件。
- **重开参数**：城开始允许一个房间有不止一段同时活着，或人要求替换 session 有第二条路。
-/

/-! D45 标签是人的偏好，存在城里；`pin` 是页面的保留标签

- **决策**：标签经 `PutPreferences { tags }` 存进人的 `config.toml`，按 `(city, room, began)` 存（`crates/wire/Spec.lean` §8-84、wire D21）；页面先画自己的副本，城的每个答案整表取代它。`pin` 置顶；Mayor 的当前一段由页面推出为置顶，不存储，菜单不提供取消。标签的文法只在城里写一次（`wire::preference::tag`，经生成的 `Tag` 到页面），页面在输入时折成小写（`core/tags.ts` 的 `readTag` 一处）。
- **理由**：标签是人的分类，不改变任何 run 能观察到的东西；从手机连同一座城读到同一个答案。Mayor 的当前一段永远是人回来时第一个要找的，而「当前」每次 `/new` 都会换一段——存下来的置顶会留在旧的一段上。
- **被击败的备选**：①账本事件——人的分类成了城的历史；②存在浏览器——手机看不到；③每个标签一种颜色、一个标签管理页——没有人要，且颜色要另立一套角色。
- **重开参数**：同一台机器上两座同名的城共享标签（键是城的名字）；那时按城的指纹存，wire D21 记着同一个参数。
-/

/-! D46 `/compact` 在页面上组合已有的动词

- **决策**：`/compact` 等于 `/new --carry`：有 run 在跑时先发 `cancel`，belief 里它冻结后（`belief/live.ts` 的 `onceFrozen`）再发 `OpenSession { carry: handoff }`；不加内核动词，不加 `/handoff`。
- **理由**：每个 run 冻结时各写一次 `handoff_written` 与 `run_frozen`，冻结即已写好交接；城在有 run 工作的房间里拒开新段（`E_BUSY`），所以必须等冻结的那一行到达再开。
- **被击败的备选**：一个内核 `Compact` 动词——它做的事页面用两帧已能表达，多一个动词多一份权威。
- **已知边界**：冻结的记录到达页面与 worker 交还房间之间有一个很短的窗口；城在这个窗口里答 `E_BUSY` 时，拒绝照常出现在角落，再发一次即可。
- **重开参数**：这个窗口在常见的机器上让 `/compact` 经常第一次就被拒。
-/

/-! D47 每次启动都进专注档，存下的档不在启动时读回

- **决策**（User 定）：页面每次启动画 `zen`；`prefs.ts` 的 `setTier` 只改这个标签页里的档，不写 `sprawling.tier`、不发 `PutPreferences`，`prefs_city.ts` 的 `adopted` 不拿城回答里的 `tier`。启动在 `client/spec/Views/Workspace.lean` 的 `launch` 里，`a_launch_opens_in_zen` 对任何存下的档成立。
- **理由**：打开 WebUI 是来说话的；上次离开时停在检阅档，下次一打开就是满屏的世界层，对话被压成底下一条带。
- **被击败的备选**：照旧读回存下的档、只把首访档从 `blend` 改成 `zen`——第一次之后的每次启动仍不是专注档，与 User 的要求不符。
- **重开参数**：`wire::PreferencePatch` 与城的 `config.toml` 仍有 `tier` 一臂；页面不再读写它，这一臂在城那边是否删去由 wire 的拥有者决定。
-/

/-! D48 图层键在任何时候都回到对话

- **决策**（User 定）：宽屏上按图层键（键或 Accel-\），开着设置面或别的一页时先回到对话（设置面下面是对话时回到那段对话，否则回到市长的房间），再换到下一档；换页与换档是同一次视图过渡，焦点落进对话框。状态机是 `client/spec/Views/Workspace.lean` 的 `pressLayers`。一栏上的图层键照旧开关世界层的面（4-52）。
- **理由**：档是对话页的排法；在设置面或成本页上按图层键只改一个看不见的值，人看不出键有没有反应。
- **被击败的备选**：在别的页上让图层键什么都不做——人按了没有反应，比换页更难懂。
-/

/-! D52 设置树按对象分组（方案 A）

- **决策**（User 定）：设置树六枝——城、接入、运行、扩展、偏好、诊断——按设置所关于的对象分（§7L 的表）。
- **理由**：刚上手的人找一项设置时，先想到的是它关于什么——城、接进来的供应方、run、装进来的技能——而不是自己多久用一次它；按对象分的树在人第一次打开时就读得懂。
- **被击败的备选**：方案 B，按使用频率分——常用（模型与思考强度、权限、外观、远程）、城与楼、扩展、诊断与关于，其余收进「高级」。它让常用的几项少一步，但「常用」是谁的常用要靠猜，一项设置在哪一枝随人的习惯而变，新手读不出规律。
- **重开参数**：树上的条目多到一枝的条目在 1440 宽的设置面里放不下一屏。
-/

/-! D53 设置树的枝折叠，一次只展开一枝；二级设置收进「更多」

- **决策**（User 定方向，本规格定细节）：枝是按钮，一次只展开一枝，打开时展开当前页所在的那一枝；每枝的二级设置收进枝末的「更多」，「更多」下不放有子条目的条目，树因此至多三级（D19）。状态机是 `client/spec/Views/Fold.lean`，与上手指南的步骤共用。
- **理由**：五枝二十余项一齐摊开时，人要扫过整棵树才找得到一项；只开当前那一枝，树在打开时就指着人所在的地方。一次只开一枝，是因为两枝同时开着时「当前在哪」又要靠读。
- **被击败的备选**：各枝各自记开合（每枝一个布尔值）——人按开第二枝后两枝都开着，树又回到摊开的样子；枝永不折叠（此前的做法）——条目一多就是一面墙。
- **重开参数**：人常在两枝之间来回，每次都要重开一枝。
-/

/-! D54 没有 provider 端点时启动总是进快速开始；跳过即离开指南

- **决策**（User 定）：启动时城里一个 provider 端点都没有，外壳就进上手指南，不看城的 `GUIDE.toml` 记着的 `state = "left"`，也不看 `welcomed`；「跳过全部可选项」记下指南已离开，打开与主 Agent 的房间。判定只在城第一次答出端点表时做一次（`client/spec/Views/Guide.lean`）。
- **理由**：没有端点的城什么也派不出去，第一步是唯一能让它工作的事；离开过一次的指南不该把一座仍然不能工作的城藏起来。跳过之后人要的是对话，而判定每个回答都做一次时，跳过之后的下一个回答就把人送回指南。
- **被击败的备选**：在城那边把 `left` 在没有端点时读成 `open`——要改 `crates/accounting/src/guide.rs` 与线上的回答，而进不进指南是页面在启动时的一次选择，城不必为它改答案。
-/

/-! D55 上手指南一步做完就前进，一次只展开一步

- **决策**（User 定行为，手感由前端定）：展开着的那一步由城的配置变成已完成时，指南展开下一个还没完成也没人推迟的步，原来那一步收起；一次只展开一步；展开的正文用 `theme/motion.css` 的 `drop` 进来，收起没有动画；`prefers-reduced-motion` 或外观里关掉动效时静止。
- **理由**：做完一步之后人的下一个动作是下一步；让做完的那一步继续开着，人要先收起它再找下一步。收起不做动画，是动效表的通则：离开的东西不该多停一刻。
- **被击败的备选**：用高度过渡做展开与收起——要动 `height`，动效表只动 transform 与 opacity，而且 Firefox 没有 `interpolate-size`，三家引擎读出两种手感。
-/

/-! D92 会改变状态或打开浮层的动作只用组合键

- **决策**（User 定）：`core/keys.ts` 的默认表里，除 `composer.focus`（`/`）之外每个动作都带 accel：图层键 Accel-\，快捷键速查 Accel-`/`，从手下那一条分叉 Accel-Shift-F，请决定卡的三个答复 Accel-Shift-Y／E／X。浏览器自己留着的组合（`RESERVED`：N、T、W，带不带 Shift）不用，默认表里没有两个动作共用一个组合；`keys.test.ts` 判这三条。人在设置的快捷键组里仍可把任何动作改回单键，那是人自己的选择。
- **理由**：任何不带修饰键的键都有打字误触的风险：焦点停在消息、卡片或页面上而不在输入框里时，打出的一个字母会分叉对话、换档、开出速查面，`y`／`n` 会直接替人作决定。`matches` 只在文本框里不认单键，挡不住焦点不在文本框时的误触。`/` 留作单键，因为它只把焦点移进输入框，误按的后果是接下来的字落进输入框，正好接住误打的字。分叉与三个答复带 Shift，是因为 Accel-F 是浏览器的查找，Accel-Y、Accel-E 在一些浏览器里是历史与搜索；不同意取 X，是因为 Accel-N 带不带 Shift 都在页面听到之前开出新窗口。
- **被击败的备选**：①保留单键、只在焦点处于消息或卡片时才认——误触正是发生在焦点停在那里的时候；②用 Alt 组合——`matches` 不认 Alt，因为 Alt 在 macOS 上用来打出别的字符，在 Windows 上会把焦点交给浏览器的菜单栏。
- **重开参数**：浏览器开始允许页面接住 Accel-N，或人要求某个动作改回单键。
-/

/-! ## 11 边界枚举

交互契约的边界在模型里逐个成立：空控件没有 Tab 站、全部被拒的控件给第 0 格（`Segmented`）；列表的两端（`Row`、`Combobox`、`Popover`、信箱的 j/k）；一圈的回绕（`Parts.wrap`）；第 `KEPT` 加一项与最后一个页签被关掉（`Inspect.Open`）；分隔线推到一栏之外（`Workbench`）；打开者已不在页面上时不还焦点（`Parts.closeLayer`）。效应核的边界——断线、重连的缺口、陈旧的回答、拒绝——写在 §7 各模块的那一行与 §10 的设计条目里，由各模块旁的测试判。
-/

/-! ## 12 错误处理

客户端不抛出：eslint 禁 `throw` 与 `try`，可败的值是 Effect 的 `Result`，可缺的值是 `Option`（§4-6）。城的拒绝由 `parts/notice.svelte` 画在三个座位之一（§4-35），出路由 `core/recovering.ts` 一张表从拒绝码映射到动作；页面自己等不到回答的问题以 `E_TIMEOUT` 超时（D10）；链路丢失不是通知，是页面所处的状况（§4-35 的横幅）。失败之后保留什么：草稿留在框里或草稿门里（§4-35、§4-46），被拒的保存不清掉草稿（§4-36、§4-46）。
-/

/-! ## 13 依赖选型

版本号的权威是 `client/package.json` 与 `client/bun.lock`，本表只写每个包的角色。

| 包 | 角色 |
|---|---|
| svelte | 视图（runes 编译进产物，无虚拟 DOM、无框架运行时 diff；`src/` 一律 runes 模式，见 D11） |
| effect | Schema、Brand |
| @lezer/highlight 与各语言的 @lezer 语法 | 代码视图的语法着色，按语言懒加载（4-26） |
| @lucide/svelte | 图标；只有 `parts/glyph.svelte` 导入它，每个图标单独导入，产物只带用到的那些（`docs/frontend-method.md` §4-34） |
| @noble/post-quantum | 远程设备一侧的 ML-KEM-768 与 ML-DSA-44，WebCrypto 没有的两种算法；只由 `core/remote/` 用动态 `import()` 取，只有远程组打开时才下载（4-57，`crates/remote_access/Spec.lean` D21） |
| @codemirror/state、view、commands、search、merge | RefRain 的编辑器：文档与改动集、视图与输入法、撤销与键表、查找替换、两版之间的 diff；只由 `views/refrain/` 读，懒加载（7N、D23） |
| pdfjs-dist | RefRain 画 PDF 的页、读出每页的文字供两版比较；worker 在自己的线程里解析；只由 `views/refrain/formats/` 读，懒加载（4-54、D32） |
| docx-preview | RefRain 把 DOCX 排成页（近似版式），画进沙箱框；带进 jszip；只由 `views/refrain/formats/` 读，懒加载（4-54、D32） |
| katex | 把城读出而不画的 `$…$` 与 `$$…$$` 公式排成 MathML，由浏览器自己的 MathML 引擎画；只由 `views/refrain/formula.svelte` 用动态 `import()` 取，页面第一次画公式时才下载；不带它的样式表与字体（4-64a、D42a） |
| vite / @sveltejs/vite-plugin-svelte / @tailwindcss/vite / tailwindcss | 构建 |
| svelte-check | `bun run typecheck`：`.svelte` 与 `.ts` 同一车道（见设计 4-1） |
| @typescript/native（别名，指向 TS 7 的 `typescript` 包） | `--tsgo` 车道的检查器（Go 版） |
| typescript | typescript-eslint 的 JS 编译器 API；svelte-check 的版本闸 |
| eslint / @eslint/js / typescript-eslint / eslint-plugin-svelte | lint |
| @types/bun | `bun:test` 的类型，仅测试文件用 |

脚本：`dev`、`build`（Vite，`base: './'`）、`typecheck`（`client/scripts/typecheck.ts` 封 `svelte-check --tsgo`，连 `.ts` 一起查，见设计 4-1）、`lint`（`eslint --max-warnings 0`：警告即红）、`test`（`bun test --conditions=browser`，见设计 4-2；因此 justfile 的 `check-client` 写 `bun run test` 而不是 `bun test`）。`svelte.config.ts` 与 `vite.config.ts` 同在 `client/` 根：vite-plugin-svelte 按 Vite root 找配置而 svelte-check 逐文件向上找，`svelte({ configFile: "../svelte.config.ts" })` 是让两者共用一个家的那根线（设计 4-5）。`src/vite-env.d.ts` 只引 `vite/client` 的类型，让 `import "./theme.css"` 的副作用导入有类型。

### §7-9 一个库进来要满足的条件

四条，一条都不能省：**它替换掉一样东西**——一段自有代码、一个手画的部件，或平台在三家引擎上都给不了的一种无障碍行为；**同一变更集里有生产读者**，没有读者的库是一个有体积没人用的名字；**许可证在 `deny.toml` 的清单上**；**本节写下它替换了什么，以及引入前后 `frontend_artifact` 的读数**。门机制的那一次提交带 `Verdict: user-approved`，因为放宽 `RUNTIME` 是放宽一道门。

| 库 | 替换了什么 | 读数 |
|---|---|---|
| `@lucide/svelte` | `parts/glyph.ts` 的手画路径表（17 个图标） | 引入时在构建产物上读出，记进 `tools/xtask/budgets.toml` 的 `frontend_artifact` 行 |
| `@codemirror/state`、`view`、`commands`、`search`、`merge` | RefRain 要手写的编辑面（改动集、撤销、输入法、查找替换）与行级 diff（D23） | 懒加载的一块，首屏不付；`frontend_artifact` 引入前后的读数由整合记进 `tools/xtask/budgets.toml` |
| `pdfjs-dist` | 本客户端画不出的 PDF 页与读不出的 PDF 文字；落选的是浏览器自带的阅读器（D32） | 与 `docx-preview` 一起，`frontend_artifact`（每个文件 `gzip -9` 后相加）从 676,051 B 到 2,416,686 B，其中 CMap 约占一半；分块与资源都懒加载，首屏不付（D32） |
| `docx-preview` | 一个自写的 DOCX 排版器；落选的是只抽语义不排页的 Mammoth（D32） | 见上一行 |
| `@noble/post-quantum` | 页面要自己实现的 ML-KEM-768 与 ML-DSA-44：WebCrypto 在三家引擎上都没有这两种算法（4-57、`crates/remote_access/Spec.lean` D21） | 懒加载的一块，只在远程组按下配对或锁门时下载；`frontend_artifact` 的读数由整合记进 `tools/xtask/budgets.toml` |

**版本范围**：`client/package.json` 的每一项写插入号范围（`^x.y.z`），装进来的树由 `client/bun.lock` 定，`just build-web` 以 `bun install --frozen-lockfile` 安装，所以范围放宽不让两台机器装出两棵树；锁文件不算钉子。精确版本防的是锁文件已经防住的事，因此不写。留下的上限只有一条，各写明它防什么、什么时候拿掉：

| 包 | 上限 | 防的是什么 | 拿掉的条件 |
|---|---|---|---|
| `typescript` | `^6.0.3`，即低于 7 | typescript-eslint 8 读 TypeScript 的 JS 编译器 API，遇到 TS 7 在加载配置时就拒绝运行（“typescript-eslint does not support TS 7.0”），`bun run lint` 因此整个不跑；TS 7 的检查器另由 `@typescript/native` 别名供 `--tsgo` 车道用 | typescript-eslint 的某个版本支持 TS 7 时，把 `typescript` 升到 7，并删去 `@typescript/native` 别名，由 `typescript` 一项同时供两条车道；typescript-eslint 8.71.0 的 peer 范围是 `typescript >=4.8.4 <6.1.0`，所以今天这个条件不成立 |

`@noble/post-quantum` 是 0.x 版本，插入号范围本身只放补丁号（`^0.7.1` 即 0.7.x），这正是要的界：ML-KEM-768 与 ML-DSA-44 的算法由 FIPS 203 与 FIPS 204 定，补丁不改线上的字节，次版本号可以改接口。它在最终产物里是 `core/remote/` 动态 `import()` 的两块懒加载分块（`ml-kem`、`ml-dsa`，共用一块 `_crystals`），首屏不带。
-/

/-! ## 14 硬编码声明

固定的数各在定义它的模块里只写一次，理由写在用它的设计条目旁：`KEPT`（8，§4-45，模型 `client/spec/Views/Inspect/Open.lean`）、`NARROWEST` 与 `COLUMNS`（2 与 12，D24，模型 `client/spec/Core/Workbench.lean`）、`COMMITS_PAGE`（40，§4-15）、`PACE_MS`（250，§4-11）、`RESUME_PAGES`（2，§4-40）、`EDITABLE_BYTES_MAX`（4 MiB，§4-46）、提示的 300 ms 意图阈值（§4-18）。主题令牌的数只住 主题（`client/src/theme.css` 与它 import 的部分）（`docs/frontend-method.md` §3-3）。
-/

/-! ## 15 影响面

改一个部件的键表，改的是它在 `#/gallery` 的夹具、`docs/frontend-method.md` 的走查，以及调用它的每个视图：调用者由 §7 的视图一节列出。改效应核的一个接口，读者是 §7 那张表里列出的模块与它们的测试；改线上的形状，先改 `crates/wire/Spec.lean`，再由 `cargo xtask wire-ts` 重新生成 `client/src/wire.ts`。
-/

/-! ## 16 测试与约束

模型由 `just models`（`lake build`，无 `sorry`、`admit` 或 `axiom`）证明。实现与模型的对应由客户端自己的测试判：`just check-client` 跑 `bun run lint`、`bun run typecheck` 与 `bun run test`。画出来的页面由 `cargo xtask render` 在真引擎里量 `#/gallery`（`tools/xtask/Spec.lean` §8-13），颜色、字句、动效与体积各有自己的门（`xtask color`、`wording`、`motion`、`budget`）。键表今天没有一次按键进过真引擎（§7-10）：模型证明的是规格自洽，`bun test` 判的是实现里不碰 DOM 的那一半，其余由人按 `docs/frontend-method.md` 的走查判。
-/

/-! ## 17 文档关系

- `docs/frontend-method.md`：一个屏怎样在这里建成、人走查运行窗口的清单，以及屏幕长什么样的条目（§3-3、§4-33、§4-34、§4-37、§4-43、§7A、§7B、§7D、§7E、§7F、§7H、§7I、§7J，英文）；它引用本规格的交互契约（§9）。
- `ARCHITECTURE.md` §9（七种形状，§7 的模块表按它标形状）、§11（Lean 规格的布局，以及本规格进 `lakefile.toml` 的两个 glob）。
- `docs/glossary.md`：概念名；`client/src/lang.json`：对人的字。
- `docs/third-party.md` §4（随包字体的许可）、§6（键表借鉴的文档）。
- `crates/wire/Spec.lean`：线上的帧与回答，`client/src/wire.ts` 由它生成。
- `tools/xtask/Spec.lean`：判客户端的门（`render`、`color`、`motion`、`wording`、`npm`、`budget`）。
-/

/-! 性能组的交互契约

设置树的 performance 叶子绘制 core 的 placement、priority 与可缺席 memory_bytes，
只以 Preferences 查询答复为读回 authority。placement 与 priority 复用 Segmented
的 Radio Group 键表；内存复用 Field 的 label/help/error 契约，空值发送 None，
正整数字节才可提交。保存只发送相对读取基线已编辑的字段，收到新的回答且读回的已发送字段全部符合目标才报告 saved，
以免一张卡的未编辑字段覆写另一张卡的变化；在等待回执时继续编辑也保留新草稿，
拒绝保留草稿，
超出 receipt patience 时报告 unverified。保存后提醒重启 serving；运行中的 Shares
不被改写。gallery/settings 的 820 与 390 两种宽度绘制真实控件，无真实配置写入。
-/
