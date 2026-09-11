# client-SPEC — the browser client (Solid + Effect, outside the cargo workspace)

> 权威顺序同 AGENTS.md：人的裁决 → ARCHITECTURE.md → 本文件 → 代码与测试。本文件记接口与裁决；视图层（`src/*.tsx`、`src/theme.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。

## 1 定位与边界（card 6.1）

- `client/` 在 cargo workspace **之外**，由 bun 驱动；产物落 `target/web-dist/`（`index.html` 在该目录根，其余在 `assets/`），`crates/sprawling/build.rs` 递归嵌入该目录。
- 与 `crates/web`（Dioxus wasm）**并存到 card 6.11**。在此之前 `just dist` 仍走 `build-web-wasm`；`build.rs` 判「完整」仍看 `web.js`＋`web_bg.wasm`，故新客户端在 6.11 之前不会被二进制嵌入为完整客户端——这是有意的，不是缺陷。
- **并存期内本包冻在脚手架（裁决 D38）。** 前端整段后置到波次 E，并存期因而从几周拉长到整个版本的剩余时间。在此期间：wire 变更**只改 `crates/web` 一侧**；`src/wire.ts` 允许陈旧；card 6.2 的同步门**明确不登记**（它是故意不做，不是没做完）；card 6.3 开工那一刻跑一次 `cargo xtask wire-ts --write` 一次性追平。理由：`wire.ts` 是**生成物**，陈旧成本为零、追平成本是一条命令；本包 23 个测试中只有 `wire.test.ts` 的 6 个依赖 wire 形状，重生即复绿。**代价写在这里，以免有人当成 bug 去修**：并存期内本包不是「可用但落后」，而是「已知不可用」。并存期的代价已经付过一次：波次 A 的 card 1.2 因它交付不完整（`channels::EndpointSummary` 未带 `probed`，因为加字段要同时动两个客户端）。
- **两种范式不叠**（Memo D9）：Effect 只住效应核——socket 阶梯、enrol、asking、pace、wire 解码（6.2 生成的 Schema）、`Match` 穷尽帧；视图是纯 Solid（signal）。二者之间唯一的缝是 `src/core/bridge.ts`。
- 运行时依赖恰两个：`solid-js`、`effect`。hash 路由手写，不引路由库；不引 UI kit。
- Firefox 是第一浏览器：每个屏幕先在 Firefox 里验收。
- `trustedDependencies` 留空：bun 默认不跑生命周期脚本，任何包的 postinstall 都不执行。

## 2 工具链

| 包 | 版本 | 角色 |
|---|---|---|
| solid-js | 1.9.15 | 视图 |
| effect | 3.22.1 | 效应核、Schema、Brand |
| vite / vite-plugin-solid / @tailwindcss/vite / tailwindcss | 8.2.2 / 2.11.14 / 4.3.3 / 4.3.3 | 构建 |
| typescript（名下） | 6.0.3 | typescript-eslint 的 JS 编译器 API（见裁决 4-1） |
| typescript-native（别名 `npm:typescript@7.0.2`） | 7.0.2 | `bun run typecheck`（Go 版编译器） |
| eslint / @eslint/js / typescript-eslint / eslint-plugin-solid | 10.10.0 / 10.0.1 / 8.70.0 / 0.18.0 | lint |
| @types/bun | 1.4.2 | `bun:test` 的类型，仅测试文件用 |

**card-9.3 的一次依赖刷新**：`bun outdated` 只报两处落后。`eslint-plugin-solid` 0.17.0 → **0.18.0** 已取，`typecheck`／`lint`／`bun test`（29 条）三样全绿，裁决 4-5 的那条带理由的 `eslint-disable-next-line` 仍然被用到（未用即红，故这是实测而非推断），说明那道缝还在。**`typescript` 名下不动，留在 6.0.3**：7.0.2 是本机能装到的最新，但裁决 4-1 已经写明 typescript-eslint 8.70.0 的 peer 范围是 `>=4.8.4 <6.1.0` 且 7.0.2 不带 JS 编译器 API——把这个名升到 7.0.2 就是拿掉全部类型感知 lint。要升它，先改裁决 4-1，不是先改版本号。

脚本：`dev`、`build`（Vite，`base: './'`）、`typecheck`、`lint`（`eslint --max-warnings 0`：警告即红）、`test`（`bun test --conditions=browser`，见裁决 4-2；因此 justfile 的 `check-client` 写 `bun run test` 而不是 `bun test`）。`src/vite-env.d.ts` 只引 `vite/client` 的类型，让 `import "./theme.css"` 过 TS 7 的副作用导入检查。

## 3 接口

### 3-1 `src/core/lang.ts`（形状 6 数据面 ＋ 一个查表函数）

```ts
export type Lang = "en" | "zh";
export const LANGS: readonly Lang[];              // 开关的顺序：en, zh
export type Key = keyof typeof table;             // lang.json 的键，snake_case 的 Msg 名
export function say(lang: Lang, key: Key): string;
export function langOf(tag: string): Lang;        // 前缀匹配 zh*，其余 en（同 web::lang::Lang::of）
export function endonym(lang: Lang): string;      // 语言自称，永不翻译
export function fill(pattern: string, slots: Readonly<Record<string, string>>): string;
```

- `src/lang.json` 是全部对人的字句：每键一条 `{ en, zh }`，键为 `crates/web/src/lang.rs` 的 `Msg` 变体名转 snake_case，顺序同枚举。**本卡是一次复制**（430 条，2026-09-08）；在 6.11 之前 `lang.rs` 仍是权威，同步靠人；6.11 起 `lang.json` 成唯一权威。
- 漏译不可表示：`Key` 由 JSON 的类型推出，`say` 对不存在的键在编译期拒绝；测试另拒「中文栏无汉字」。

### 3-2 `src/core/route.ts`（形状 1 判定）＋ `address.ts`、`run_id.ts`（形状 2 值）

```ts
export type Lens = "ledger" | "archive" | "bin";
export type View =
  | { kind: "sessions" } | { kind: "session"; address: Address } | { kind: "waiting" }
  | { kind: "record"; lens: Lens } | { kind: "cost" } | { kind: "setup" }
  | { kind: "building"; address: Address } | { kind: "run"; run: RunId };
export const DEFAULT_VIEW: View;                          // sessions
export function toFragment(view: View): string;           // 恒以 `#/` 开头，每个 View 恰一种写法
export function fromFragment(raw: string): Option<View>;  // 认不出答 None，不悄悄回首页
export function unresolved(hash: string): Option<string>; // 空片段答 None；认不出答 `#/<named>`
export function current(location): Option<View>;         // 读地址栏（薄壳）
export function go(location, view): void;                 // 写地址栏；hashchange 才动 signal

export type Address = string & Brand<"Address">;  export const Address: Brand.Constructor<Address>;
export type RunId   = string & Brand<"RunId">;    export const RunId:   Brand.Constructor<RunId>;
```

- 片段表**逐字移植** `crates/web/src/route/fragments.rs`：写一种、读全部旧写法（overview/city/live→sessions；approvals→waiting；ledger/archive/recycle-bin→record 三透镜；dashboard→cost；settings→setup；building/<addr>→building）。
- `Address` 文法移植 `kernel::address::Address::parse`（非空、非绝对、无 `\`、无 `:`、无控制字符、无空段、无 `.`/`..` 段）。`RunId` 读连字符 8-4-4-4-12 与 32 位纯十六进制两种，写连字符小写（同 `RunId` 的 `Display`）；`urn:uuid:` 与花括号形式不读。6.2 的 `wire.ts` 生成 Schema 后，这两个 Brand 由生成物替代。

### 3-3 `src/core/bridge.ts`（形状 4 适配器：Effect → Solid 的唯一缝）

```ts
export function signalFromStream<A>(stream: Stream<A>, initial: A): Accessor<A>;
export function resourceFromEffect<A, E>(effect: Effect<A, E>): Resource<Exit<A, E>>;
```

- 流与效应的类型参数 `R = never`、流的 `E = never`：**失败在过缝之前必须已经变成数据**。`resourceFromEffect` 把结果交给视图时是 `Exit<A, E>`，视图读 `Exit.match`，永远拿不到一个 throw 出来的东西。
- `signalFromStream` 在当前 owner 上 `onCleanup` 中断 fiber；组件卸载即停流。

### 3-4 `src/theme.css`

`@import "tailwindcss"` 之后 `@theme` 把默认调色板、字体、字号、字重、圆角、间距、容器宽度全部置 `initial`，再用 `crates/web/src/theme.rs` 的令牌表重新声明（名字沿用：`g0…g10`、`accent`、`alert`、`accent-hover`、`alert-hover`、`accent-solid`、`text`、`text-quiet`、`text-faint`、`text-disabled`；字体 `sans`/`mono`；字号与字重 `figure/title/heading/label/body/note`；间距 `tight/snug/base/pane/wide/section`；宽度 `measure/page`；圆角 `panel/card/control/pill`）。彩色令牌的 chroma 值取自 `theme` 测试写出的 `target/screens/tokens.css`（比例解算的结果），并保留 `calc(<chroma> * var(--chroma))`，去色仍是一个系数置零。**全客户端只有这一个文件可以出现颜色字面量。**

## 4 裁决

- **4-1 TypeScript 7.0.2 与 typescript-eslint 8.70.0 不能共用一个 `typescript` 名。** 7.0.2 是 Go 版编译器，包的 `.` 导出只有 `{version, versionMajorMinor}`，没有 JS 编译器 API；typescript-eslint 的 peer 范围是 `>=4.8.4 <6.1.0`，实测 `require('typescript')` 拿不到 `createProgram`。取法：`typescript` 名给 6.0.3（该 API 的最后一条线），7.0.2 以别名 `typescript-native` 安装并承担 `typecheck`。否决「只装 7.0.2、放弃类型感知 lint」：禁 `any`/`as`/非穷尽 switch 都是类型感知规则，没有它们 D9 的守卫就不存在。否决「只装 6.0.3」：违背 D18「新引入依赖钉最新精确版」且放弃原生编译器的速度。代价：两个编译器读同一份 tsconfig，lint 用 6.0.3 的类型信息，typecheck 用 7.0.2；二者分歧时以 typecheck 为准。
- **4-2 `bun test` 必须带 `--conditions=browser`。** `solid-js` 的 exports 在 `node` 条件下给 `dist/server.js`，其中 `createEffect` 不跑；`bunfig.toml` 没有能改条件的键（实测 `[test] conditions`、`[run] conditions`、顶层 `conditions` 皆无效）。否决「测试只用不依赖响应性的 API」：bridge 的全部价值就是响应性。
- **4-3 hash 路由，不用 path 路由。** 同 web-SPEC §8-14：片段不发给服务端，书签、后退、深链成立，且不动 `ClientAssets::lookup` 那道安全判定。
- **4-5 eslint 配置用 `tseslint.config()` 而不是 ESLint 的 `defineConfig()`。** eslint-plugin-solid 0.17.0 经 `@typescript-eslint/utils` 8.70.0 给 plugin 定型，其 `RuleContext` 仍声明 ESLint 10 已删的成员（`getAncestors`、`parserPath` 等），故该 plugin 对象不可赋给 ESLint 自己的 `Plugin` 类型，`defineConfig` 在 typecheck 下红。`tseslint.config` 是 typescript-eslint 为这道缝留的定型桥，已标 deprecated；配置文件上用一条带理由的 `eslint-disable-next-line @typescript-eslint/no-deprecated`，并开 `reportUnusedDisableDirectives: "error"`——上游修好、桥不再需要的那天，这条指令变「未用」即红（实测未用指令确实红），与 Rust 的 `#[expect(reason)]` 同义。否决「把配置文件排除在 tsc 与 lint 之外」：那会让全库唯一不受 `as` 禁令保护的文件恰好是定义禁令的文件。
- **4-4 `Address`/`RunId` 用 Effect `Brand.refined`，不用 `as`。** Brand 的构造器提供 `.option()`，非法值答 `None`，与 Rust 的 `Result` 同形；`as` 全库禁用（`as const` 除外），所以「新类型」只能由构造器产出。

## 5 未完成与交接

- `lang.json` 是复制件，`lang.rs` 仍在被其他卡改动；6.3 前重跑一次抽取或改由 xtask 生成。
- `build.rs` 的「完整」判据与 `crates/web/assets/index.html` 的注入在 6.11 改指新产物（不在本卡范围，`crates/` 未触）。
- `xtask npm` 门（锁文件、运行时依赖白名单、许可证清单）Roadmap 6.1 列有，属 xtask，本卡未做。

## 6 重建后的形状（card-6.3／6.4／6.6／6.7，2026-09-10；本节起为权威，§1 的「冻在脚手架」与 §3-3 的 `bridge.ts` 作废）

### 6-1 裁决（接 Memo D40–D47）

- **D40 Effect 只做 wire 解码。** `core/frames.ts` 用生成的 `Schema` 读每一帧（`Schema.parseJson(ServerFrame)`），这是 Effect 在运行时唯一出现的地方；socket 阶梯、asking、belief 都是纯 TS 状态机加 Solid signal／store。`bridge.ts` 删除。D9「两种范式不叠」的原意保住：视图只见 Solid。理由：`Link` 本是 392 行的纯状态机，用 Stream/Fiber 包它买不到任何东西，却让每个视图多一层范式。
- **D41 首屏即对话**：`#/` = 与 `hall/mayor` 的对话；同一房间的每次 dispatch 是一段线程；live 时 Enter 是 `steer`，冻结后 Enter 是新的 `dispatch { addr: room, session: null }`（`room_for` 对含 `/` 的地址不再开子房间）。等人的事以卡片插进对话流。
- **D42 按钮全在左栏**（用户提议）：`views/rail.tsx` 收起时只有字形，展开（hover／`[`／`?`）才出现名字与快捷键——`g m`／`g c`／`g s`／`g w`／`g r`／`g $`、Ctrl-K。页面其余部分没有按钮。
- **D43 两层可视化**：`#/city` 是 SVG 画的城（一栋楼一块，窗＝run，门口小人＝活动 run 的姿态，旗＝pursuit，牌＝blocked，条＝进度，唯一控件是刹车）；`#/building/<addr>` 是目录树（`Query::Listing` 逐层）＋文件原文（`Query::Document`）＋计划表；`#/run/<id>` 四透镜。
- **D44 页面上没有句子**（用户裁）：`lang.json` 195 条，全是标签；引导页的说明段、图例、空态提示全部删除。
- **D45 性能纪律**：帧按动画帧合并（`socket.ts` 的 `queue`＋`requestAnimationFrame`），事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` `PACE_MS`），stale-while-revalidate，动画只用 transform／opacity。
- **D46 视觉**：令牌与 `theme.rs` 同值；四个 `rounded-*` 工具类带 `corner-shape`（面板 `superellipse(2)`＝G3，卡片／控件 1.5＝G2，胶囊真圆）；`--spacing-0` 命名为零，因为 `--spacing` 置 `initial` 后 `min-h-0` 会消失。
- **D48 刹车与动词一律拼成命令**（前端会话 2，用户裁「停下这座城」应改成 `/stop` 形）：`city_stop`＝`/stop --all`、`bld_halt`＝`/stop {addr}`、`talk_stop`／`run_cancel`＝`/stop`，`release` 同形；两语同一拼写，`lang.test.ts` 因此承认「术语」可以两语相同（以 `/` 或 `{` 起头，或单 token ≤12 字）。图例、`rail_hint`、`talk_empty_hint` 一类解释句删除：人会从别的软件迁移用法。
- **D49 城是一条大道上的天际线**：`views/city.tsx` 重画——市政厅居中带穹顶与柱廊，其余楼按名字左右交替；楼高＝2 层＋每 4 次 run 一层（上限 8）；一扇窗一次 run（frozen＝g3、thinking＝accent-solid、calling＝闪、waiting＝alert）；门口小人＝活动 run（≤3）；旗＝pursuit（running 时 `wave`）；门旁灯＝blocked；基座一条进度带；星空由固定散列布点。`<defs>` 只在 `City()` 里写（编译器看得见 `<svg>`），Building 组件内仍守 D47。
- **D50 `#/mcp` 页与 `belief.probed`**：MCP 按楼管理（`configure_building.mcp`），三扇门：Composio（server id＋user id＋api key→`/enroll` 入库为 `secret:mcp/composio-<label>`，header 写 `x-api-key: <ref>`，url `https://backend.composio.dev/v3/mcp/<id>?user_id=<u>`）、http、stdio；`configure_building` 不落账本事件，页面在命令发出 300 ms 后自问一次 `building_view`。`endpoint_probed` 改由 belief 折叠为 `probed{name,models}`，不再翻历史尾巴。被拒的 `steer`（`action` 以 `steer` 起头）让 belief 把该 run 标 frozen——服务端曾把死在 `model_called` 之后的 run 留成活的。
- **D47 Solid 里画 SVG 的一条规则**：组件内部的 SVG 元素只能用编译器按名字认得的标签（`g`／`path`／`rect`／`circle`／`text`／`line`），链接用 `<g role="link">` 加事件，不用 `<a>`——`<a>` 会被建成 HTML 元素，其 SVG 子树不渲染（card-6.6 实测）。

### 6-2 `src/core/`（形状按 ARCHITECTURE §9）

| 文件 | 形状 | 接口 |
|---|---|---|
| `link.ts` | 1 判定 | `newLink(token)`, `connect`, `advance(link, LinkEvent) -> [Link, LinkAction]`；`LinkAction` 穷尽（open／send／welcomed／deliver／answered／saying／wait／report／close）；阶梯 `[250,500,1000,2000,5000,10000]` |
| `frames.ts` | 4 适配器 | `decodeFrame(text) -> ServerFrame \| null`, `encodeFrame(ClientFrame)` |
| `socket.ts` | 4 适配器 | `openConnection(url, token) -> Connection { state, belief, asking, command, retry, dismissRefusal }`；`tokenIn(search)`, `socketUrl(location)` |
| `asking.ts` | 1 判定 | `createAsking(send) -> { ask(query) -> Accessor<Answer\|undefined>, refresh, answered, invalidate(record), reconnected }`；答案按内容匹配问题，无名者按到达序；`staleBy` 是事件到查询的失效表 |
| `belief.ts` | 7 投影 | `createBelief() -> { belief: {runs, halted, refusal, city}, adoptCity, apply, say, refused, named }`；`RunBelief { addr, started, task, doing: thinking\|calling\|waiting\|frozen, saying }` |
| `commands.ts` | 2 值 | 每个命令帧一个构造函数，自铸 `IdemKey` |
| `enrol.ts` | 4 适配器 | `enrol(origin, realm, name, value) -> Promise<Enrolment>`；`referenceFor(provider)` |
| `idem.ts` | 2 值 | `mintIdem()` |
| `prefs.ts` | 6 数据 | `loadPrefs(storage, browserLang) -> { lang, effort, welcomed }`（localStorage） |
| `prose.ts` | 1 判定 | `blocks(text) -> Block[]`, `inline(text) -> Inline[]`：Markdown 读成数据，永不 innerHTML |
| `route.ts` | 1 判定 | `View = talk\|city\|building\|run\|setup\|record\|cost\|welcome`；旧片段全读；`MAYOR`, `buildingOf`, `roomOf` |
| `time.ts` | 1 判定 | `ago`, `clock`, `count`, `usd`, `kib` |

`src/ui.tsx` 是视图拿到的一切：`UiProvider`／`useUi`（conn、prefs、bar、origin、now）、`useSay`（键→词，填槽）、`useGo`、`useCommand`。

### 6-3 视图（免 SPEC，列出以便定位）

`views/rail.tsx` 左栏；`views/talk.tsx`＋`talk/{thread,composer,waiting}.tsx` 对话；`views/city.tsx`＋`city/shape.ts`（超椭圆路径）；`views/building.tsx`＋`building/tree.tsx`；`views/run.tsx`；`views/setup.tsx`＋`setup/{providers,models}.tsx`；`views/welcome.tsx`；`views/record.tsx`；`views/cost.tsx`；`views/palette.tsx`；`views/refusal.tsx`；`views/prose.tsx`。

### 6-5 会话 3（2026-09-11）：相位、浮现、画廊

- **D57 composer 说出消息落点。** `core/belief.ts` 新增 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`。拼写按 D48 成命令：`/dispatch`／`/steer`／`/steer --queued`，两语同形。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻；三种情形拼成一个词，恰好是流式文字最容易误导的那一点。**零 wire 变更**：信息全在 belief 里。`belief.test.ts` 三条钉住它。
- **文字末端浮现。** 一次活的回合里，`saying` 的末 `EDGE = 10` 字画 `text-faint`。**纯由文本推出**：无计时器、无队列、无 per-character 节点；`model_returned` 落账时 belief 本就清空 `saying`，所以「何时不再是边缘」无人需要决定。
- **抖动缓冲故意不做**（D57）。服务端刚刚才变成真流，抖动多大是未测量的量；不够顺再开卡，那时它有一个量过的理由。
- **`#/gallery`**（card-6.9）：路由而非构建开关，因为量它的那道门应当打开一个人真正跑的 bundle。夹具不需要城（`prefs` 走 localStorage）。**实测**：起静态服务＋无头 Edge `--dump-dom`，四种姿态各自映到 `/steer`／`/steer --queued`／`/steer --queued`／`/dispatch`。

### 6-4 验收记录（2026-09-10，前端会话 1＋2）

`bun run lint`／`typecheck`／`test`（26 条）绿；`cargo xtask npm`／`wire-ts` 绿；`cargo nextest -p gateway`（84）绿。在真城（`sprawling serve --web-dir target/web-dist`）＋假扮 OpenAI 形供应方上实测：连接与握手、引导页触发（无 main 时）、从 composer 派活、`read` 调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与 transcript 链接、面板。**会话 2 补实测**（Edge headless，CDP 驱动）：引导第 1–5 步在 ModelScope 真供应方上走通（enrol→probe 出 46 个模型→attach→select_model main/digest）；设置页 attach 与 select；对话页派活到 GLM-5.2 出 `E_PROVIDER 400/429` 卡片与结局线（免费 key 额度耗尽，多 Agent 场景未跑）；`#/mcp` 三扇门加／删（CONFIG.toml 实写）；楼页目录树、文件原文（编号行）、transcript 直达 run；run 页四透镜；记录三透镜；成本无价态；左栏展开。**仍未实测**：等人卡片、`Changes`／`Hunks` 有内容时、Firefox／Zen（headless `-screenshot` 不出图，需走 BiDi）。**流式**：远端 endpoint 走 `Endpoint::call_streaming`（`stream: true` 实测），但 delta 帧在 run 结束时一次涌到（`tmpscripts/delta-probe.ts` 计时：全部 delta 同一毫秒到达），归服务端转发路径；本机无 key 的 endpoint 走 `Native` 适配器，无 `call_streaming`。
