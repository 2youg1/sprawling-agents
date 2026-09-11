# client-SPEC — the browser client (Solid + Effect, outside the cargo workspace)

> 权威顺序同 AGENTS.md：人的裁决 → ARCHITECTURE.md → 本文件 → 代码与测试。本文件记接口与裁决；视图层（`src/*.tsx`、`src/theme.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。

## 1 定位与边界

- `client/` 在 cargo workspace **之外**，由 bun 驱动；产物落 `target/web-dist/`（`index.html` 在该目录根，其余在 `assets/`），`crates/sprawling/build.rs` 递归嵌入该目录，并以 `index.html` 与 `assets/` 的存在判「完整」。
- **两种范式不叠**：Effect 只做一件事——用生成的 `Schema` 读每一帧（`core/frames.ts`）。socket 阶梯、asking、belief 都是纯 TS 状态机加 Solid signal／store，视图只见 Solid。
- 运行时依赖恰两个：`solid-js`、`effect`。hash 路由手写，不引路由库；不引 UI kit。`xtask npm` 门守这三件事：锁文件与清单逐条同、运行时依赖白名单、许可证清单。
- Firefox 是第一浏览器：每个屏幕先在 Firefox 里验收。
- `trustedDependencies` 留空：bun 默认不跑生命周期脚本，任何包的 postinstall 都不执行。

## 2 工具链

| 包 | 版本 | 角色 |
|---|---|---|
| solid-js | 1.9.15 | 视图 |
| effect | 3.22.1 | Schema、Brand |
| vite / vite-plugin-solid / @tailwindcss/vite / tailwindcss | 8.2.2 / 2.11.14 / 4.3.3 / 4.3.3 | 构建 |
| typescript（名下） | 6.0.3 | typescript-eslint 的 JS 编译器 API（见裁决 4-1） |
| typescript-native（别名 `npm:typescript@7.0.2`） | 7.0.2 | `bun run typecheck`（Go 版编译器） |
| eslint / @eslint/js / typescript-eslint / eslint-plugin-solid | 10.10.0 / 10.0.1 / 8.70.0 / 0.18.0 | lint |
| @types/bun | 1.4.2 | `bun:test` 的类型，仅测试文件用 |

脚本：`dev`、`build`（Vite，`base: './'`）、`typecheck`、`lint`（`eslint --max-warnings 0`：警告即红）、`test`（`bun test --conditions=browser`，见裁决 4-2；因此 justfile 的 `check-client` 写 `bun run test` 而不是 `bun test`）。`src/vite-env.d.ts` 只引 `vite/client` 的类型，让 `import "./theme.css"` 过 TS 7 的副作用导入检查。

## 3 接口

### 3-1 `src/core/lang.ts`（形状 6 数据面 ＋ 一个查表函数）

```ts
export type Lang = "en" | "zh";
export const LANGS: readonly Lang[];              // 开关的顺序：en, zh
export type Key = keyof typeof table;             // lang.json 的键，snake_case
export function say(lang: Lang, key: Key): string;
export function langOf(tag: string): Lang;        // 前缀匹配 zh*，其余 en
export function endonym(lang: Lang): string;      // 语言自称，永不翻译
export function fill(pattern: string, slots: Readonly<Record<string, string>>): string;
```

- `src/lang.json` 是全部对人的字句，且是唯一权威：每键一条 `{ en, zh }`。
- 漏译不可表示：`Key` 由 JSON 的类型推出，`say` 对不存在的键在编译期拒绝；测试另拒「中文栏与英文栏逐字相同」，例外是术语（以 `/` 或 `{` 起头，或单 token ≤12 字）。

### 3-2 `src/core/route.ts`（形状 1 判定）＋ `address.ts`、`run_id.ts`（形状 2 值）

```ts
export type View =
  | { kind: "talk"; address: Address } | { kind: "city" } | { kind: "building"; address: Address }
  | { kind: "run"; run: RunId } | { kind: "setup" } | { kind: "mcp" }
  | { kind: "record"; lens: Lens } | { kind: "cost" } | { kind: "welcome" } | { kind: "gallery" };
export function toFragment(view: View): string;           // 恒以 `#/` 开头，每个 View 恰一种写法
export function fromFragment(raw: string): Option<View>;  // 认不出答 None，不悄悄回首页
export function unresolved(hash: string): Option<string>; // 空片段答 None；认不出答 `#/<named>`
export function current(location): Option<View>;          // 读地址栏（薄壳）
export function go(location, view): void;                 // 写地址栏；hashchange 才动 signal

export type Address = string & Brand<"Address">;  export const Address: Brand.Constructor<Address>;
export type RunId   = string & Brand<"RunId">;    export const RunId:   Brand.Constructor<RunId>;
```

- 写一种、读全部旧写法：`overview`／`city`／`live`／空片段都读作与 `hall/mayor` 的对话，`approvals` 读作对话（等人的事插在流里），`ledger`／`archive`／`recycle-bin` 读作 record 三透镜，`dashboard` 读作 cost，`settings` 读作 setup。
- `address.ts`／`run_id.ts` 是**地址栏那道缝的文法解析器**，与 `wire.ts` 的同名 brand 不重复：生成物的 brand 没有精炼，而这两个文件按 `kernel::address::Address::parse` 与 `RunId` 的文法判合法（非空、非绝对、无 `\`、无 `:`、无控制字符、无空段、无 `.`／`..` 段；8-4-4-4-12 与 32 位纯十六进制两读一写）。`route.ts`／`palette.tsx`／`building/tree.tsx` 依赖它们的 `.option()`。

### 3-3 `src/theme.css`

`@import "tailwindcss"` 之后 `@theme` 把默认调色板、字体、字号、字重、圆角、间距、容器宽度全部置 `initial`，再声明这套令牌（`g0…g10`、`accent`、`alert`、`accent-hover`、`alert-hover`、`accent-solid`、`text`、`text-quiet`、`text-faint`、`text-disabled`；字体 `sans`／`mono`；字号与字重 `figure/title/heading/label/body/note`；间距 `tight/snug/base/pane/wide/section`；宽度 `measure/page`；圆角 `panel/card/control/pill`）。彩色令牌保留 `calc(<chroma> * var(--chroma))`，去色仍是一个系数置零。**全客户端只有这一个文件可以出现颜色字面量**，`xtask color` 守它。

## 4 裁决

- **4-1 TypeScript 7.0.2 与 typescript-eslint 8.70.0 不能共用一个 `typescript` 名。** 7.0.2 是 Go 版编译器，包的 `.` 导出只有 `{version, versionMajorMinor}`，没有 JS 编译器 API；typescript-eslint 的 peer 范围是 `>=4.8.4 <6.1.0`，实测 `require('typescript')` 拿不到 `createProgram`。取法：`typescript` 名给 6.0.3（该 API 的最后一条线），7.0.2 以别名 `typescript-native` 安装并承担 `typecheck`。否决「只装 7.0.2、放弃类型感知 lint」：禁 `any`／`as`／非穷尽 switch 都是类型感知规则，没有它们守卫就不存在。否决「只装 6.0.3」：违背「新引入依赖钉最新精确版」且放弃原生编译器的速度。代价：两个编译器读同一份 tsconfig，lint 用 6.0.3 的类型信息，typecheck 用 7.0.2；二者分歧时以 typecheck 为准。**升 `typescript` 这个名要先改本条，不是先改版本号。**
- **4-2 `bun test` 必须带 `--conditions=browser`。** `solid-js` 的 exports 在 `node` 条件下给 `dist/server.js`，其中 `createEffect` 不跑；`bunfig.toml` 没有能改条件的键（实测 `[test] conditions`、`[run] conditions`、顶层 `conditions` 皆无效）。否决「测试只用不依赖响应性的 API」：效应核的全部价值就是响应性。
- **4-3 hash 路由，不用 path 路由。** 片段不发给服务端，书签、后退、深链成立，且不动 `ClientAssets::lookup` 那道安全判定。
- **4-4 `Address`／`RunId` 用 Effect `Brand.refined`，不用 `as`。** Brand 的构造器提供 `.option()`，非法值答 `None`，与 Rust 的 `Result` 同形；`as` 全库禁用（`as const` 除外），所以「新类型」只能由构造器产出。
- **4-5 eslint 配置用 `tseslint.config()` 而不是 ESLint 的 `defineConfig()`。** eslint-plugin-solid 经 `@typescript-eslint/utils` 给 plugin 定型，其 `RuleContext` 仍声明 ESLint 10 已删的成员（`getAncestors`、`parserPath` 等），故该 plugin 对象不可赋给 ESLint 自己的 `Plugin` 类型，`defineConfig` 在 typecheck 下红。`tseslint.config` 是 typescript-eslint 为这道缝留的定型桥，已标 deprecated；配置文件上用一条带理由的 `eslint-disable-next-line @typescript-eslint/no-deprecated`，并开 `reportUnusedDisableDirectives: "error"`——上游修好、桥不再需要的那天，这条指令变「未用」即红，与 Rust 的 `#[expect(reason)]` 同义。否决「把配置文件排除在 tsc 与 lint 之外」：那会让全库唯一不受 `as` 禁令保护的文件恰好是定义禁令的文件。
- **4-6 Effect 只做 wire 解码。** `core/frames.ts` 用生成的 `Schema` 读每一帧（`Schema.parseJson(ServerFrame)`），这是 Effect 在运行时唯一出现的地方。理由：`Link` 是一个纯状态机，用 Stream／Fiber 包它买不到任何东西，却让每个视图多一层范式。
- **4-7 首屏即对话。** `#/` ＝ 与 `hall/mayor` 的对话；同一房间的每次 dispatch 是一段线程；live 时 Enter 是 `steer`，冻结后 Enter 是新的 `dispatch { addr: room, session: null }`（`room_for` 对含 `/` 的地址不再开子房间）。等人的事以卡片插进对话流，不另开一页。
- **4-8 按钮全在左栏。** `views/rail.tsx` 收起时只有字形，展开（hover／`[`／`?`）才出现名字与快捷键——`g m`／`g c`／`g s`／`g w`／`g r`／`g $`、Ctrl-K。页面其余部分没有按钮。
- **4-9 两层可视化。** `#/city` 是 SVG 画的城；`#/building/<addr>` 是目录树（`Query::Listing` 逐层）＋文件原文（`Query::Document`）＋计划表＋提交列表；`#/run/<id>` 四透镜。
- **4-10 页面上没有句子。** `lang.json` 全是标签；说明段、图例、空态提示不写。**动词一律拼成命令**：`city_stop` ＝ `/stop --all`、`bld_halt` ＝ `/stop {addr}`、`talk_stop`／`run_cancel` ＝ `/stop`，`release` 同形；两语同一拼写。理由：人会从别的软件迁移用法，一条命令的拼写自己说明自己。
- **4-11 性能纪律。** 帧按动画帧合并（`socket.ts` 的 `queue` ＋ `requestAnimationFrame`），事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` 的 `PACE_MS`），stale-while-revalidate，动画只用 `transform`／`opacity`。
- **4-12 Solid 里画 SVG 的一条规则。** 组件内部的 SVG 元素只能用编译器按名字认得的标签（`g`／`path`／`rect`／`circle`／`text`／`line`），链接用 `<g role="link">` 加事件，不用 `<a>`——`<a>` 会被建成 HTML 元素，其 SVG 子树不渲染。`<defs>` 只在最外层组件里写。
- **4-13 composer 说出消息落点。** `core/belief.ts` 的 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`；拼写 `/dispatch`／`/steer`／`/steer --queued`。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻。**零 wire 变更**：信息全在 belief 里。**抖动缓冲不做**：抖动多大是一个未测量的量，为一个未测量的量先建队列是这座城禁的那条。
- **4-14 从 diff 到 session 是一次路由跳转**，不新增命令帧、不加「继续」按钮。`dispatch{addr: room, session: null}` 与在该房间对话页按 Enter 是同一个动作，加按钮就是给一个已有机制起第二个名字。
- **4-15 提交列表按页问、按页存。** `core/asking.ts` 的 `COMMITS_PAGE = 40` 与 `commitsQuery(building, before)`——问题只有一种拼写，因为 `CommitsAnswer` 回带 `building` 与 `before` 而不带 `limit`，键若拼法不一，答案永远落不回槽里。每页是一个独立的问题：只有 `before: null` 的首页会因 `checkpoint_committed`／`pr_merged` 失效重问；旧页上界是已写下的 seq，且 `lineage` 从写提交的 run 向前走，后来的接替者改不了它。
- **4-16 转写结果落进输入框，不直接发出去。** 机器听错的那一句必须能改，否则它会花掉一次 run。没有为 `transcribe` 选过模型的城不画那个按钮：一个只可能答拒绝的控件，是一个没人该遇见的控件。

## 5 `src/core/`（形状按 ARCHITECTURE §9）

| 文件 | 形状 | 接口 |
|---|---|---|
| `link.ts` | 1 判定 | `newLink(token)`, `connect`, `advance(link, LinkEvent) -> [Link, LinkAction]`；`LinkAction` 穷尽（open／send／welcomed／deliver／answered／saying／wait／report／close）；阶梯 `[250,500,1000,2000,5000,10000]` |
| `frames.ts` | 4 适配器 | `decodeFrame(text) -> ServerFrame \| null`, `encodeFrame(ClientFrame)` |
| `socket.ts` | 4 适配器 | `openConnection(url, token) -> Connection { state, belief, asking, command, retry, dismissRefusal }`；`tokenIn(search)`, `socketUrl(location)` |
| `asking.ts` | 1 判定 | `createAsking(send) -> { ask(query) -> Accessor<Answer\|undefined>, refresh, answered, invalidate(record), reconnected }`；答案按内容匹配问题，无名者按到达序；`staleBy` 是事件到查询的失效表 |
| `belief.ts` | 7 投影 | `createBelief() -> { belief: {runs, halted, refusal, city, probed}, adoptCity, apply, say, refused, named }`；`RunBelief { addr, started, task, doing: thinking\|calling\|waiting\|frozen, saying }`；`sendingInto(doing)` |
| `commands.ts` | 2 值 | 每个命令帧一个构造函数，自铸 `IdemKey` |
| `enrol.ts` | 4 适配器 | `enrol(origin, realm, name, value) -> Promise<Enrolment>`；`referenceFor(provider)` |
| `speaking.ts` | 4 适配器 | `canRecord()`, `record(origin) -> Promise<Recording \| null>`；`Recording.stop() -> Promise<Heard>`，`Heard` 穷尽（text／refused／silent） |
| `idem.ts` | 2 值 | `mintIdem()` |
| `mark.ts` | 4 适配器 | `paintMark(document, quiet \| live \| waiting)`：把令牌解算成引擎实际会画的颜色，拼成 SVG data URL 写进 `<link rel="icon">`；零颜色字面量 |
| `prefs.ts` | 6 数据 | `loadPrefs(storage, browserLang) -> { lang, effort, welcomed, draft(at), setDraft(at, text) }`（localStorage，草稿键 `sprawling.draft.<房间或 run>`） |
| `prose.ts` | 1 判定 | `blocks(text) -> Block[]`, `inline(text) -> Inline[]`：Markdown 读成数据，永不 innerHTML |
| `route.ts` | 1 判定 | 见 §3-2 |
| `time.ts` | 1 判定 | `ago`, `clock`, `count`, `usd`, `kib` |

`src/ui.tsx` 是视图拿到的一切：`UiProvider`／`useUi`（conn、prefs、bar、origin、now）、`useSay`（键→词，填槽）、`useGo`、`useCommand`、`useHearing`。

## 6 视图（免 SPEC，列出以便定位）

`views/rail.tsx` 左栏；`views/talk.tsx` ＋ `talk/{thread,composer,waiting}.tsx` 对话；`views/city.tsx` ＋ `city/shape.ts`（超椭圆路径）；`views/building.tsx` ＋ `building/{tree,commits}.tsx`；`views/changes.tsx`（`Changes`／`Hunks` 的一份读法，run 页与楼页共用）；`views/run.tsx`；`views/setup.tsx` ＋ `setup/{providers,models}.tsx`；`views/machine.tsx`（doctor 的答）；`views/desktop.tsx`（一栋楼的桌面白名单）；`views/mcp.tsx`；`views/welcome.tsx`；`views/record.tsx`；`views/cost.tsx`；`views/palette.tsx`；`views/refusal.tsx`；`views/prose.tsx`；`views/gallery.tsx`。

**`#/gallery` 是一条路由而不是一个构建开关**，因为量它的那道门应当打开一个人真正跑的 bundle；夹具不需要城（`prefs` 走 localStorage）。每个能进入多种状态的屏幕在那里各有一份夹具，`cargo xtask render` 打开真引擎读它。

**左栏是覆盖而不是推挤**：外层 `<div>` 只在钉开（`[`）时取 `w-rail-open`，`<nav>` 绝对定位、hover 时自宽并加投影；正文的左边因此不随指针越过左缘而重排。

## 7 验收

`bun run lint`、`bun run typecheck`、`bun run test`（29 条）三样绿，`cargo xtask npm`、`cargo xtask wire-ts`、`cargo xtask color`、`cargo xtask wording`、`cargo xtask render` 绿；`just check-client` 是这三条脚本的一条线。

在一座真城加一个说 OpenAI 形的假供应方上走得通的：连接与握手、引导五步、从 composer 派活、工具调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与文件原文、run 页四透镜、记录三透镜、成本页、设置页 attach 与 select、`#/mcp` 三扇门加删（写进 `CONFIG.toml`）、楼页提交列表与 session 跳转、左栏展开、草稿留存。

**未验的**：`Changes`／`Hunks` 有内容时的样子、Firefox 与 Zen 的无头截图（`-screenshot` 不出图，须走 BiDi）。
