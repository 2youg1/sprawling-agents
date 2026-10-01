# client-SPEC — the browser client (Svelte + Effect, outside the cargo workspace)

> 权威顺序同 AGENTS.md：人的决定 → ARCHITECTURE.md → 本文件 → 代码与测试。本文件记接口与设计；视图层（`src/**/*.svelte`、`src/theme.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。
>
> **免的是画法，不是键盘。** 一个部件遵循哪个 WAI-ARIA 模式、每个键做什么、焦点还给谁、`aria-*` 取什么值，是这一层对使用者的承诺，不是一次视觉迭代；`docs/frontend-method.md` 的豁免因此只覆盖排布、间距、色调与动效，`views/parts/` 的交互契约由 §7 独家规定。

## 1 定位与边界

- `client/` 在 cargo workspace **之外**，由 bun 驱动；产物落 `sprawling` 包里的 `crates/sprawling/web-dist/`（crates.io 的包只装包目录，sprawling-SPEC 8-83），不随 `CARGO_TARGET_DIR` 移动（`index.html` 在该目录根，其余在 `assets/`），`crates/sprawling/build.rs` 递归嵌入该目录，并以 `index.html` 与 `assets/` 的存在判「完整」。
- **两种范式不叠**：Effect 只做一件事——用生成的 `Schema` 读帧（`core/frames.ts`；`event` 与 `delta` 两种热帧先走由同一份 schema 导出的窄校验，见 4-6）。socket 阶梯、asking、belief 都是纯 TS 状态机加 `svelte/store`，视图只见 Svelte。
- 运行时依赖的名单只有一个家：`tools/xtask/src/npm.rs` 的 `RUNTIME`，本文件不抄它的条目与数目。名单上除了 `svelte` 与 `effect`，还有 `@lezer/highlight` 与各语言的 `@lezer` 语法，因为代码视图按语法上色，而高亮器与每种语法都是按需加载的分块（4-26），不进首屏。hash 路由手写，不引路由库；不引 UI kit（§7 判定）。`xtask npm` 门守三件事：锁文件与清单逐条同、运行时依赖恰为 `RUNTIME`、许可证在 `deny.toml` 的清单上。
- Firefox 是第一浏览器：每个屏幕先在 Firefox 里验收。
- `trustedDependencies` 留空：bun 默认不跑生命周期脚本，任何包的 postinstall 都不执行。

## 2 工具链

版本号的权威是 `client/package.json` 与 `client/bun.lock`，本表只写每个包的角色。

| 包 | 角色 |
|---|---|
| svelte | 视图（runes 编译进产物，无虚拟 DOM、无框架运行时 diff；`src/` 一律 runes 模式，见 12-11） |
| effect | Schema、Brand |
| @lezer/highlight 与各语言的 @lezer 语法 | 代码视图的语法着色，按语言懒加载（4-26） |
| vite / @sveltejs/vite-plugin-svelte / @tailwindcss/vite / tailwindcss | 构建 |
| svelte-check | `bun run typecheck`：`.svelte` 与 `.ts` 同一车道（见设计 4-1） |
| @typescript/native（别名，指向 TS 7 的 `typescript` 包） | `--tsgo` 车道的检查器（Go 版） |
| typescript | typescript-eslint 的 JS 编译器 API；svelte-check 的版本闸 |
| eslint / @eslint/js / typescript-eslint / eslint-plugin-svelte | lint |
| @types/bun | `bun:test` 的类型，仅测试文件用 |

脚本：`dev`、`build`（Vite，`base: './'`）、`typecheck`（`client/scripts/typecheck.ts` 封 `svelte-check --tsgo`，连 `.ts` 一起查，见设计 4-1）、`lint`（`eslint --max-warnings 0`：警告即红）、`test`（`bun test --conditions=browser`，见设计 4-2；因此 justfile 的 `check-client` 写 `bun run test` 而不是 `bun test`）。`svelte.config.ts` 与 `vite.config.ts` 同在 `client/` 根：vite-plugin-svelte 按 Vite root 找配置而 svelte-check 逐文件向上找，`svelte({ configFile: "../svelte.config.ts" })` 是让两者共用一个家的那根线（设计 4-5）。`src/vite-env.d.ts` 只引 `vite/client` 的类型，让 `import "./theme.css"` 的副作用导入有类型。

## 3 接口

### 3-1 `src/core/lang.ts`（形状 6 数据面 ＋ 一个查表函数）

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

### 3-2 `src/core/route.ts`（形状 1 判定）＋ `src/core/run_id.ts`（形状 4 适配器）

```ts
export type Lens = "ledger" | "archive" | "bin" | "log";
export const LENSES: readonly Lens[];
export const MAYOR: Address;                              // hall/mayor
export type View =
  | { kind: "talk"; address: Address } | { kind: "city" } | { kind: "building"; address: Address }
  | { kind: "run"; run: RunId } | { kind: "setup" } | { kind: "mcp" } | { kind: "record"; lens: Lens }
  | { kind: "cost" } | { kind: "registry" } | { kind: "welcome" } | { kind: "monitor" } | { kind: "gallery" };
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

- 写一种、读全部旧写法：`overview`／`city`／`live`／空片段都读作与 `hall/mayor` 的对话，`approvals` 读作对话（等人的事插在流里），`ledger`／`archive`／`recycle-bin` 读作 record 三透镜，`dashboard` 读作 cost，`settings` 读作 setup。
- **两个身份值都由 `client/src/wire.ts` 生成，且都带 pattern 精炼**：`Address` 是 `kernel::Address::parse` 的路径文法，`RunId` 只收连字符小写 uuid（两者都是 `client/src/wire.ts` 里的同名导出）。客户端不再自带文法：`core/run_id.ts` 的 `readRunId` 是 `Schema.decodeOption` 于生成的 `RunId`，`route.ts` 的片段读法与 `building/tree.svelte` 的转写文件名读法都调它，认不出答 `None`。**裸 32 位十六进制、`{…}`、`urn:uuid:` 与全大写由此都读不出**，与城收窄后的 `kernel::RunId::parse` 一致；地址栏与目录树的 `Address` 直接来自 wire.ts。

### 3-3 `src/theme.css`

`@import "tailwindcss"` 之后 `@theme` 把默认调色板、字体、字号、字重、圆角、间距、容器宽度全部置 `initial`，再声明这套令牌（`g0…g10`、`accent`、`alert`、`accent-hover`、`alert-hover`、`accent-solid`、`text`、`text-quiet`、`text-faint`、`text-disabled`；字体 `sans`／`mono`；字号与字重 `figure/title/heading/label/body/note`；间距 `tight/snug/base/pane/wide/section`；宽度 `measure/page`；圆角 `panel/card/control/pill`）。彩色令牌保留 `calc(<chroma> * var(--chroma))`，去色仍是一个系数置零。**全客户端只有这一个文件可以出现颜色字面量**，`xtask color` 守它。

### 3-4 未决

- **Solid 一臂的两个读数**（12-12）。树上只有 Svelte 一臂的读数。能定下 12-12 的证据是两个在同一台机器上交错测的读数，并写明机器类属：一是构建产物大小，按 `frontend_artifact` 的称法（`just build-web` 之后 gzip 称整个 dist），两臂各称一次；二是每 token 开销，用 `belief/fold_cost.test.ts` 的仪表（一帧 50 个 delta、R = 1e4、一个读全表的订阅者），Solid 一臂把 `belief/runs.svelte.ts` 换成 Solid 的 store。12-2 的 30–40 µs 没有记下机器类属，所以交错测时 Svelte 一臂也要重测。
- **重连续传的阈值**（4-40）。`RESUME_PAGES = 2` 是估计。能定下它的读数是同一座城上一次快照重问（`asking.reconnected()` 发出的全部问题的回答字节）与一页 `HistoryRange` 的字节之比；缺口超过阈值时回退到快照的那条路今天也没有测试。

## 4 设计

- **4-1 两个 `typescript` 名，一条类型车道：`@typescript/native`（TS 7，Go 版）是检查器，`typescript`（TS 6）是 JS 编译器 API。** `typecheck` 跑 `svelte-check --tsconfig ./tsconfig.json --tsgo`：svelte-check 实测把 `.svelte` 与 `.ts` 同一车道查完（两类文件的错都报、`noUncheckedIndexedAccess` 等严格旗标都从 tsconfig 读到），`--tsgo` 车道用 TS 7 的检查器，同树实测比普通车道快约一倍，两万行级的客户端上差距只会更大。**别名必须叫 `@typescript/native`**（svelte-check 只认这个名字，`typescript-native` 无人读）；`typescript`（TS 6）仍有岗位——typescript-eslint 的类型感知 lint 要 JS 编译器 API，而 TS 7 的包只导出版本号，upstream 的报错也要求两个名字并存。**`--tsgo` 车道的已测缺陷是安装缺失时打印错误却退出 0**，损坏时仍绿的门不满足「绿 run 即证据」，所以 `typecheck` 不裸跑它：一层薄封装（`client/scripts/typecheck.ts`）先清掉 `.svelte-check/`（车道写盘的转译产物，清掉即无状态，目录进 `.gitignore`），再断言三件——退出码为 0、总结行是 0 errors and 0 warnings、输出里没有 setup 失败的那句 Error；封装把上游缺陷变成响亮的红。重开参数：上游把退出码修对，封装即可删。
- **4-2 `bun test` 必须带 `--conditions=browser`。** `svelte` 的 `.`／`svelte/store`／`svelte/reactivity` 各带 browser／worker／default 三个条件，缺省解析到 server 构建——**那不是不响应，而是假的 SUT**：`SvelteMap === Map` 为真、`mount` 是抛错桩（均实测），而假构建上的测试是全绿的。测试套件因此留一条构建保真断言（`SvelteMap !== Map` 且 `mount.length > 0`）把这个条件钉住；`bunfig.toml` 仍没有能改条件的键。否决「不带旗标跑」：无声测错对象比报错更糟。
- **4-3 hash 路由，不用 path 路由。** 片段不发给服务端，书签、后退、深链成立，且不动 `ClientAssets::lookup` 那道安全判定。
- **4-4 身份值只由生成的 `Schema` 产出，`as` 全库禁用。** `Address` 与 `RunId` 是 `wire.ts` 里带 pattern 的 brand，判合法只有一条路：`Schema.decodeOption`，非法值答 `None`，与 Rust 的 `Result` 同形（`core/run_id.ts` 是 run id 那条判定的唯一家）。`as` 全库禁用（`as const` 除外），所以「新类型」只能由构造器产出；`make` 是 brand 的构造器，供本文已经写对的字面量用（`MAYOR` 与夹具），它不查 pattern。
- **4-5 eslint 配置用 ESLint 自己的 `defineConfig()`，三条补充各有一个原因。** eslint-plugin-svelte 在 `defineConfig` 里定型通过，所以不需要 typescript-eslint 的 `tseslint.config` 定型桥。补充一：typescript-eslint 的 `eslint-recommended` 块（以类型检查器代管 `no-undef`／`no-unused-vars`）扩到 `**/*.svelte`——Svelte 的 script 就是 TS，在那里手写禁用是同一规则的第二个家。补充二：模板表达式无处写类型，`settings.svelte.ignoreWarnings` 只在模板里静默 `no-unsafe-assignment`／`no-unsafe-member-access`，script 里的同一规则照报。补充三：被 lint 的每个文件都必须在 tsconfig 工程里，故 `svelte.config.ts` 取 `.ts` 而非 `.js` 并进 `include`；`.svelte-check/`（svelte-check 写盘的生成物）进 `ignores` 与 `.gitignore`。否决「把配置文件排除在 typecheck 与 lint 之外」：那会让全库唯一不受 `as` 禁令保护的文件恰好是定义禁令的文件。
- **4-6 Effect 只做 wire 解码。** `core/frames.ts` 用生成的 `Schema` 读每一帧（`Schema.decodeUnknownEither(ServerFrame)`），这是 Effect 在运行时唯一出现的地方。热帧例外：`event` 帧（每条 ledger 记录一帧）与 `delta` 帧（每个 token 一帧）先走 `JSON.parse` 加窄校验，窄校验的每条规则都取自生成的 schema——事件种类集合取自 `EventKind` 的字面量，`RunId`／`B3Hash`／`Address` 直接调各自 refinement 的 filter；窄校验不收的帧仍交给 Effect，所以它只能让帧变快，不能放进 schema 拒绝的帧。理由：完整 Effect 解码的开销大半在解析器机器本身，event 帧一帧要 16–28 µs，热路径 2–4 µs（`client/scripts/frame_cost.ts` 交错测两条路径的下限，读数记在 `tools/xtask/budgets.toml` 的 `client_frame_decode` 行）。这个读数不是测试：墙钟读数属于机器，一台满载的机器不是缺陷，`frames.test.ts` 只判热路径与 schema 读出同样的帧、拒绝同样的帧。理由：`Link` 是一个纯状态机，用 Stream／Fiber 包它买不到任何东西，却让每个视图多一层范式。
- **4-7 首屏即对话。** `#/` ＝ 与 `hall/mayor` 的对话；同一房间的每次 dispatch 是一段线程；live 时 Enter 是 `steer`，冻结后 Enter 是新的 `dispatch { addr: room, session: null }`（`room_for` 对含 `/` 的地址不再开子房间）。等人的事以卡片插进对话流，不另开一页。
- **4-8 按钮全在左栏。** `views/rail.svelte` 收起时只有字形，展开（hover／`[`／`?`）才出现名字与快捷键——`g m`／`g c`／`g s`／`g w`／`g r`／`g $`、Ctrl-K。页面其余部分没有按钮。
- **4-9 两层可视化。** `#/city` 是 SVG 画的城；`#/building/<addr>` 是目录树（`Query::Listing` 逐层）＋文件原文（`Query::Document`）＋计划表＋提交列表；`#/run/<id>` 四透镜。
- **4-10 页面上没有句子，但零数据的屏必须说下一步。** `lang.json` 全是标签，说明段不写。**动词一律拼成命令**：`city_stop` ＝ `/halt --all`、`bld_halt` ＝ `/halt {addr}`、`talk_stop`／`run_cancel` ＝ `/stop`，`release` 与 `halt` 同形；两语同一拼写。理由：人会从别的软件迁移用法，一条命令的拼写自己说明自己。**图例与空态提示不算说明段**：`city/bar.svelte` 的图例是五个字形唯一的名字，去掉它城就是一张没人读得懂的画；一个零数据的屏只画一个灰词时，人分不清这屏是空的还是坏的。因此**空态一律走 `parts/empty.svelte`**——一个形状、一句说缺什么的话、一个离开这个状态的动作，动作能省而那句话不能。这不放宽「不写说明段」：空态那句话说的是这一屏此刻没有什么，不是这一屏是干什么的。
- **4-11 性能纪律。** 帧按动画帧合并（`socket.ts` 的 `queue` ＋ `requestAnimationFrame`）；页面隐藏时浏览器不再调用动画帧，此时改用计时器排空（`visibilityState === "hidden"` 下用 `setTimeout(drain, 0)`），否则一条审批请求要等人切回标签页才到；`visibilitychange` 转为可见时立即排空，链路若在 backoff 就取消已排的尝试、立即重试一次，因为人此刻在看，这一级剩下的秒数只是一页空白。事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` 的 `PACE_MS`），stale-while-revalidate，动画只用 `transform`／`opacity`。
- **4-12 SVG 的规则：id 只有一个家。** Svelte 的模板解析器按命名空间处理 svg 子树，所以 `<a>` 在 SVG 里仍是 SVG 元素。规则只有一条：`<defs>` 只在最外层绘图组件里写，渐变／滤镜／裁剪的 id 在那里声明一次，引用者拿 id、不自己拼第二遍。城市插画（skyline／marks）是画不是图标，导航与动作类图标一律 `parts/glyph.svelte`（4-34）。
- **4-13 composer 说出消息落点。** `core/doing.ts` 的 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`；标签 `/dispatch`／`/steer`／`/steer · after the tool call`；三者发的都是 `/dispatch` 或 `/steer <text>`，`queued` 只是标签说出的落点，不是另一种送法（真正的排队送达——等 run 冻结后再送——要改 wire，不在本客户端的语法里）。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻。**零 wire 变更**：信息全在 belief 里。**抖动缓冲不做**：抖动多大是一个未测量的量，为一个未测量的量先建队列是这座城禁的那条。
- **4-14 从 diff 到 session 是一次路由跳转**，不新增命令帧、不加「继续」按钮。`dispatch{addr: room, session: null}` 与在该房间对话页按 Enter 是同一个动作，加按钮就是给一个已有机制起第二个名字。
- **4-15 提交列表按页问、按页存。** `core/asking.ts` 的 `COMMITS_PAGE = 40` 与 `commitsQuery(building, before)`——问题只有一种拼写，因为 `CommitsAnswer` 回带 `building` 与 `before` 而不带 `limit`，键若拼法不一，答案永远落不回槽里。每页是一个独立的问题：只有 `before: null` 的首页会因 `checkpoint_committed`／`pr_merged` 失效重问；旧页上界是已写下的 seq，且 `lineage` 从写提交的 run 向前走，后来的接替者改不了它。
- **4-16 转写结果落进输入框，不直接发出去。** 机器听错的那一句必须能改，否则它会花掉一次 run。没有为 `transcribe` 选过模型的城不画那个按钮：一个只可能答拒绝的控件，是一个没人该遇见的控件。
- **4-17 焦点环只有一个家，所以 `outline-none` 是删掉而不是换掉。** `theme.css` 的 base 层写着 `*:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px }`，而 utilities 层优先级更高：26 处 `outline-none` 把它吃掉，键盘用户在设置页看不见自己在哪。做法是把这 26 处全部删除、不补任何替代类——`:focus-visible` 本来就只在键盘取焦时匹配（文本框被点击时也匹配，因为它确实要收键盘输入，这是对的）。否决「改写成 `focus:outline-hidden focus-visible:outline-2`」：Tailwind v4 的 `outline-hidden` 设 `--tw-outline-style: none`，而 `outline-2` 展开成 `outline-style: var(--tw-outline-style); outline-width: 2px`，两条规则在 `:focus-visible` 时同时命中，算出来是 `outline-style: none`——那对组合会删掉它本想保住的那个环。**任何一处需要压制焦点环的地方写 `outline-hidden` 而不是 `outline-none`**：v4 把「画 2px 透明描边、在强制色模式下仍可见」这层语义改名到了 `outline-hidden`，照旧写 `outline-none` 会在 Windows 高对比下丢掉焦点环。闸门条件是 `client/src` 里 `outline-none` 出现 0 次。
- **4-18 `parts/tip.svelte` 的提示有两条定位路，调用方说关系。** `title` 对三种读者都失效：指针要悬停约一秒，键盘根本到不了，触屏永远不画。`Tip` 用 `:hover` 与 `:focus-within` 显示一个 `role="tooltip"` 的兄弟节点，`transition-delay: 300ms` 挡住指针扫过一行时的连环点亮，未显示时是 `display: none`——既不画也不被 `xtask render` 量到。**关系由调用方写**：控件自己有名字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`；组件不猜，因为只有调用点知道控件有没有可见文字。**定位两条路都要在**：`@supports (anchor-name: --a)` 内用 `position-area` 锚定并 `fixed`，脱开一切裁剪祖先；Safari 与 Firefox 今天不支持，落到对 wrapper 的 `absolute` 分支（`AGAINST_WRAPPER`）。只写前一条是静默失效：不支持 anchor 的引擎里弹层退回静态位置，可能溢出视口。锚点名按实例生成，经继承的自定义属性 `--tip-anchor` 传给提示节点，所以同一行上的两个提示各锚各的控件。
- **4-19 `Field` 管有标签的表单格，不管搜索框与命令框。** 23 处裸 `<input class="rounded-control bg-g2 …">` 里，供应方表单、模型表的两个上限格、键名值对与登录码改走 `Field`，因而一次拿到标签、说明、错误态、`aria-describedby` 与 `:user-invalid`（失焦后才红，不是每次按键）。命令面板、combobox 过滤框、composer、楼页目标框与记录搜索框**不改**：它们要 `ref`、`autofocus`、逐键的 `onKeyDown` 与自定义补全，塞进 `Field` 会把它变成十个透传参数的 `<input>` 壳子，正是 AGENTS.md 禁的那种壳。`Figure`（原 `providers.svelte`）删除，三个调优数字改 `Field kind="number" step=…`，键盘上下键因此能用；**不给 `ms` 后缀**——标签本身就是 Codex 的 `timeout_ms` 与 `stream_idle_timeout_ms`，再加一个后缀就是同一个单位的第二个家。
- **4-20 模态是平台的事。** `parts/dialog.svelte` 是原生 `<dialog>` 加 `showModal()`：top layer、焦点陷阱、Esc、其余页面 `inert`，四件都由引擎承担，文件里因此没有遮罩层、没有 keydown、没有取焦调用，也没有 z-index——top layer 之上排不进任何数字。取焦由文档顺序决定：平台取对话框里第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。Esc 到达时先 `preventDefault` 再回调 `onCancel`，否则默认行为绕过调用方关掉元素，`open` 还说着「开着」。开与合是同一条 `transition` 的两个读法，`display` 与 `overlay` 是离散属性，少了 `transition-discrete` 关门第一帧盒子就消失、连同它的淡出。**遮罩是 `::backdrop` 上的 `backdrop-filter: brightness()`，不是一层 `bg-g0/80`**：`::backdrop` 只在实现了「从原始元素继承」的引擎里读得到页面的颜色令牌，令牌解析不出时 `background-color` 回到初始值——一扇没有遮罩的模态，且没有任何一道闸会报出来；亮度滤镜不向平台要颜色，深浅两套灯光下都变暗，Tailwind 又把 `--tw-backdrop-*` 直接声明在 `::backdrop` 上，所以这条路不经过继承。
- **4-21 `views/parts/` 不写 z-index。** 定位过的盒子在未定位的盒子之后绘制：下拉、弹层、粘性表头与提示压住它们打开时盖住的那些行，这是绘制顺序本来就给的，不需要一个数字。给了数字反而要维护一张谁比谁大的表，而那张表没有家。唯一还留着数字的部件是 `parts/kbd.svelte` 的快捷键表：它必须盖住左栏，而左栏是全客户端允许保留的那一个 `z-10`，所以只有 top layer 能让它交出数字（见 4-23 的第二个参数）。
- **4-22 按钮的三态是一个 `data-state`。** `idle`／`loading`／`stopped` 由一处判定给出，属性本身、`aria-disabled`、`aria-busy` 与点击闸是它的四个读者。色调表只写静息态与静息态的 hover，`not-data-[state=idle]` 一条规则管住其余两态——「正在等回执」与「你按不了」对手的回答是同一句，不该有两套写法；hover 写进 `data-[state=idle]:` 里，因而一个不能按的控件在指针下不会亮起来假装这一按会落地。`background-color` 进了 transition 列表，hover 因此是到达而不是切换。
- **4-23 Popover API 今天进不了 `parts/popover.svelte`，两个参数卡着它。** 其一：`popover` 元素开着时在 top layer，包含块是视口，`absolute` 不再相对 composer 的 `<form>` 解算，而把弹层钉回 composer 的唯一机制是 CSS anchor positioning，本项目的第一浏览器（Firefox）没有——所以「不支持 anchor 时走今天的 `absolute` 分支」这条降级对一个 popover 不成立，它只在元素还留在流里时成立。其二：`#/gallery` 的两个弹层夹具靠一个 `relative` 祖先与两层 padding 把面板留在自己的 Case 里，元素一进 top layer 就逸出，`xtask render` 的「没有盒子画在容纳它的盒子之外」立刻红。两个参数任一移动都应重新论证本条；在那之前弹层是 `absolute`，Esc 与取回焦点由这个文件自己管。
- **4-24 后端已经答了的，客户端必须问，问到了必须画。** 一个城答得出而没人问的 `Query`，与一个上了 wire 而没有屏幕读的字段，是同一个缺陷的两半：它们让「这件事做完了」在两侧各有一个说法。三处落点：**其一**，城页顶栏的六个数字全部出自 `Query::Metrics` 一问——事件、在干活的 run、已收尾的 run、等人批的、排队的信号、回收站里的。`MetricsAnswer` 的第七个字段 `buildings` 故意不画：顶栏下面的天际线与旁边的楼列表本身就是楼的数目，再写一个数字就是同一个事实的第二个家。原先的状态徽标一并去掉——它说的 `runs_active` 就是那六个里的一个，而「城停了」由外壳的横幅在每一页说一次（`app.svelte`），顶栏只留那一个停/放的控件。**其二**，`Query::RegistryView` 得到 `views/registry.svelte` 一屏，四列（登记于、类别、所属楼、是什么），走 `parts/table.svelte` 因而每列可排序，默认最新在上。**其三**，run 页直接问 `Query::RunView` 而不再只靠流折出来的 belief：从别人发来的链接打开 `#/run/<id>` 的那一页没见过任何记录，`city_view` 又只列它还在列的 run，所以这条 run 属于哪个房间、是否已经结束，恰好在最需要的那一类 run 上是空的。**两个读法按 seq 判定**：流说的是此刻，摘要说的是城写下来的，两边都带账本位置，所以谁更新是一次比较而不是一次偏好。
- **4-25 base URL 的形状只拼写一次。** `setup/providers.svelte` 的 `BASE_URL` 同时喂两个读者：框子自己的 `pattern`（浏览器在人还在填表时判，失焦后才红）与 `hostOf`（「看看」和「接上」两个控件的开关）。**`type="url"` 不是那条规则**——它收 `mailto:somebody` 和任何别的 scheme，于是一个这张表单会拒的值可以坐在一个浏览器称为合法的框里，而人是按下一个始终发灰、不说为什么的控件才知道的。形状是：scheme、ASCII 主机、可选端口、可选路径，之后什么都不许有；查询串拒掉，因为供应方陈述的是 API 的根，每个 face 自己在后面挂路径。**客户端只做最宽的判断，永不比城更严。** 归一化今天有调用者了：`accounting::worker::credentials::Entered::resolved` 是打字地址变成被调用地址的唯一一处，probe 与 attach 都经过它。`gateway::normalise` 的第三条规则把缺席的 scheme 读成 `https://`，运行这座城的机器地址读成 `http://`，所以 scheme 在这张表单里是可选的——一个要求写 scheme 的框会拒掉厂商文档印出来的 `api.openai.com/v1` 与 `127.0.0.1:11434`，而城收得下。这张表单因此只判「有没有一个能调用的主机」，scheme 由城补，路径由城按预设表补。

- **4-26 「这次调用算哪一类」在客户端只有一张表，而那张表是替身。** `views/talk/trace.ts` 把工具名读成 `Deed`（`explored`／`wrote`／`ran`／`other`），对话页的折叠摘要与制品面板两个读者都问它，所以客户端内部没有第二张表。**权威不在这里**：`kernel::ToolMeta` 为每个注册工具声明了 `effect`（`Read`／`Write`／`Egress`／`Connector`／`Spawn`／`Govern`／`Spend`）与 `render`（`Generic`／`Terminal`／`Diff { locations }`），`runtime` 的 exec 工具声明的 `RenderIntent::Terminal` 与 edit 工具声明的 `RenderIntent::Diff` 正是制品面板要分的那两半；两个字段今天都不上 wire（`RoundsAnswer` 的 `Call` 只有 `tool`／`subject`／`arguments`／`outcome`／`at`／`output`）。**迁移一次做完**：`Call` 增补 `effect` 与 `render`、`WIRE_V` 随之进位、`trace.ts` 的 `DEEDS` 与 `deedOf` 删除、两个读者改问新字段。在那之前每加一个工具，这张表与工具注册处会各自演化一次，而只有注册处是对的。`parts/code.svelte` 的行号从 1 起算同属这笔债：`Output` 只带 `head` 与 `cut`，不带这段头部在文件里从第几行开始，所以「第 27 行」今天指的是这段输出的第 27 行。 **同一个代码视图的颜色按语法给出，高亮器不进首屏。** `parts/paint.ts` 是唯一知道语法存在的文件：它用 `@lezer/highlight` 与十种 `@lezer` 语法（cpp、css、go、java、javascript 及其 ts／jsx／tsx 变体、json、python、rust、yaml）把文本切成 `parts/code.ts` 的五种 `Ink`；它本身是懒加载块，每种语法又各是一块，所以只看 Rust 的页面只下载 Rust 的语法，一段代码都不显示的页面一块都不下载。文件扩展名与 Markdown 围栏的语言词查同一张表（`rs` 与 `rust` 不可能指向两种语法），表里没有的名字整段画成 plain；`toml`、`sh` 今天落在这里，因为 `@lezer` 没有这两种语法，而 `@codemirror/legacy-modes` 要连带 `@codemirror/language` 与编辑器状态进来。`parts/inked.svelte` 是文件视图与 Markdown 代码块共用的画法：文字第一帧就以 plain 画出，答案到了才上色，晚到的旧答案丢弃；块取不到时仍是 plain，因为文字不上色也值得读。选 lezer 不选 shiki（JavaScript 正则引擎）的参数：同样十种语言、1,000 行 TypeScript，在一台满载的 16 线程机器上 lezer 解析加上色约 70 ms，shiki 约 1,240 ms 且首次建立要 600 ms；十种语法压缩后两者都约 166 KB，而 lezer 按语言分块、给的是真正的语法树。1,000 行 ≤ 16 ms 与各块的 gzip 体积由测量那一轮读出并写进 budgets；当 lezer 在空载机器上仍超过 16 ms，或出现同体积下更快的语法高亮器时，重开这条选择。

  **Markdown 只有一个文法，它在城里（12-14）。** 文档预览问 `Query::Preview { version, viewport }`，答复是 `documents::Preview` 的块树（wire-SPEC §8-74）：页面把每一种块与行内画成元素，不经 `innerHTML`；`Unsupported` 画成等宽的原文（HTML、公式、前置元数据、过深的嵌套），`Preview::Unsupported` 说这一版不按 Markdown 读、留在源码视图，零个块是空文档的空态（4-10），三者是三种画法。代码块的 `info` 与文件扩展名查同一张语言表，经 `parts/inked.svelte` 上色。链接与图片的去处城已判过（`crates/documents/Spec.lean` D23），页面不再判协议，相对地址相对文档所在的目录。块的 `span` 是版本里的字节，与源码视图的位置对应经 `core/document_pos.ts` 换算。一个版本的预览不会过时，`staleness.ts` 把 `preview` 与 `range` 放在同一行。**当前状态**：`core/prose.ts` 仍是对话流的读法——模型说的话没有版本，`Preview` 读不到它，候选与决定它的证据在 `crates/documents/Spec.lean` §3，那一条定下之前 `prose.ts` 与 `closedUpTo` 留着。整份放得下 64 KiB 的 Markdown 文件今天不在内容库里，`Preview` 对它答 `Unavailable`（accounting-SPEC §8-23）；页面接入预览之前，城那一侧先补上。`RUNTIME` 不为 Markdown 加任何一项。

- **4-27 对话页的第二栏由账本决定要不要画，由容器宽度决定画在哪。** `talk.svelte` 问的 `Query::Rounds` 与 `Thread` 问的是同一句，`core/asking.ts` 按内容合并，因此「现在显示的是哪个 run」只有一个答案；面板不占路由，理由是一条新路由会让这个答案有第二处判定，而两处判定第一次分歧就发生在有人打开别人发来的链接时。**断点问 `main`（具名容器 `page`）而不是问窗口**：左栏钉开时吃掉 232px，按窗口宽算会在 1440px 把两栏挤坏，所以 `@lg/page:`（820px）转成两栏、`@wide/page:`（1120px）让面板长到 `max-w-measure`。**开合状态走 `core/prefs.ts` 的 `panel`，不在 `talk.svelte` 里开第二扇门**：它是人的偏好，归宿是 `~/.sprawling/config.toml`，由那扇门后面的浏览器缓存记住，城的回答到了由 `adopt` 顶掉（4-29）。

- **4-28 强度只有一个权威，客户端不留副本。** 城层 `CONFIG.toml` 的 `[model] effort` 经配置梯子冻结进每一次 run（`crates/city/src/config_layers.rs`），所以浏览器里不再存 `sprawling.effort` 这一行。**缺席不是 `"medium"`，缺席是不说**：帧里不写这个字段，城的文件回答；文件也没写时供应方回答，而 `Effort::None` 是「尽量不要想」，是另一件事。一次派活仍可为它开的那一场单独说一个档位（`Dispatch.effort`，城把它写进房间层，city-SPEC 8-14），所以选择器只在 composer 上——那里的作用域与控件的位置一致。**设置页与欢迎页因此不再有强度选择器**：它们承诺的是「从此以后」，而没有写城层强度的命令帧，一个刷新就忘的设置控件是第二个权威的开始。六句 `effort_note_*` 随选择器搬到 composer 的那一列，做每一格的 `hint`，人在决定的那一刻读到它。
- **4-29 偏好有两层，城赢。** 权威是人层 `~/.sprawling/config.toml`，浏览器存储是它前面的缓存：缓存只负责首帧不闪，城的回答一到就整条顶掉它。`core/prefs.ts` 独家拥有每一个存储键的拼写（`ROWS`）与读写，`core/rows.ts` 独家拥有对 `localStorage` 的触碰；`keys.ts` 问 `chord(action)`／`setChord`，外观屏问 `held().appearance`／`setAppearance`，网络屏问 `held().proxying`，没有第二处拼一个键名。`core/prefs_city.ts` 的 `keepWithCity(door, conn)` 是这扇门与连接唯一的接点，`main.ts` 开页时调它一次。

  **一进一出两个方向，形状因此不同。** 出城的方向是具名改动：`setLang`／`setWelcomed`／`setPanel`／`setAppearance`／`setProxying`／`setChord` 各发一条 `Command::PutPreferences { patch }`，与 `wire::PreferencePatch` 的变体一一对应；链路不在 `live` 时改动只留在这个浏览器，重连后城的回答为准。`setRail`、`setNotifying`、`setShowing` 不出城，因为城的记录没有这三个字段。进城的方向是 `adopt(stated, chords)` 一整条：一次回答陈述每一个值，按字段贴回去会贴出半新半旧的一条；回答缺席的字段由浏览器此刻的值补上（`prefs_city.ts` 的 `adopted`），回答带来的快捷键覆写这个浏览器里同一动作的行。`keeper()` 说此刻是哪一层在保管（`"browser"`／`"city"`），第一次 `adopt` 之后答 `"city"`；设置页把它画出来（`setup/kept.svelte`），因为「清掉浏览器数据会不会丢」是人有权知道的事。

  **`readPreferences` 与 `writePreferences` 互为逆，这才使缓存是缓存**：城上次答的就是下一次首帧画的。
- **4-30 设置页的 `config.toml` 侧栏引用文件，不自己拼。** 原先这一栏用 `[model_providers.<name>]` 拼出一段文字，而城自己的读法只认 `[model]`／`[sandbox]`／`[[mcp]]` 三节——把它抄进 `CONFIG.toml` 的人会拿到一句列出三节的拒绝，一个事实在前端与后端各有一个家且已经分歧。这一版把那段字符串删掉，侧栏只说「这一页不逐行列出 config.toml」（`setup_toml_unread`），因为 `Query::Config` 已由上下文梯级那一处控件在问，说「读不到」是假话。填进去的是 `Query::Config { addr }` 的答案。**它逐值带层**：`wire::ConfigAnswer` 是 `{ addr, effort: Option<SettledEffort>, tuning: TuningDefaults }`，`SettledEffort` 带 `from: ConfigLayer`，因而侧栏每一行画的是「值 ＋ 它来自哪一层」，`parts/badge.svelte` 画那一层的名字。**今天画不出来**：`Query::Config` 与 `ConfigAnswer` 都在（`client/src/wire.ts` 已带 `ConfigAnswer`），而 `setup.svelte` 那一栏尚未问它，所以侧栏仍是 `setup_toml_unread` 的空态；接上之后这一栏是唯一的填入点，不需要先退休任何一处手拼。
- **4-31 设置页有 MCP 组，skills 组是「列表 ＋ 只读源文」。** MCP 组仍是一个指向 `#/mcp` 的链接，但它搬回了设置页：左栏顶层六项里，MCP 与「账户与供应方」是同一类事（接什么进来），分成两处等于让左栏说两套分类。**反转的理由是项目数，不是「到不了」**——「左栏已经到得了」曾经是删掉它的理由，而人每天扫视左栏的成本当时没有计入；MCP 页本身、它的三扇门与删除动作一个字不动，只是入口从顶层挪进这一组。skills 组由 `setup/skills.svelte` 承担三件：放技能的文件夹、楼列（`shared/buildings.svelte`，与 `#/mcp` 同一份）、那栋楼**三个书架**的清单（`Query::Skills`，`SkillShelf` 三臂：城库／楼架／外部架）与打开一条后的原文（`Query::Document`，走楼页那一个 `FileView`；外部架没有城内地址，那一行因此只报名不打开）。**这里没有编辑器，因为城的那扇门答「未建」**：library 在保留前缀下，居民可读不可放（`crates/city/src/library.rs`），`Command::PutShelved` 已在 wire 上而 `crates/accounting/src/worker/commanding/routing.rs` 以 `not_built` 拒它，一个存不下去的 `<textarea>` 会把「改了」说成两件事（写面未建）。

  **那两级文件夹路径今天由页面拼写，这是记下的欠账。** `Shelves` 画的是 `${城名}/.sprawling/library/`，而这条路径的家是 `kernel::layout`（`RESERVED_PREFIX` 与 `LIBRARY_DIR`）——客户端与城各拼一次，城改了前缀或目录名，这一行会静默指错地方。退休它需要一个跨 wire 的字段：`SkillsAnswer`（或它的邻居）带一个由 `CityLayout::library()` 相对城根算出的 `Address`，页面改读它。今天没有任何回答携带布局，所以这一处保留并在此记录。
- **4-32 一个状态药丸只有 `parts/badge.svelte` 一个画法。** `building/plan.svelte` 原先手画五种漆色（`done` 灰、`blocked` 实心 alert、`in_progress` 实心 accent、ready 的 `bg-g3`、其余无底色），那是同一件事的第二个家，且那串嵌套三元没有 `awaiting_approval` 的臂——等人批的一行被画成没人开工的一行。现在一个穷尽 `RoadmapStatus` 的 `weightOf(row)` 给出 `quiet`／`live`／`alert` 三档，`Badge` 画它。**两个状态共用一档是对的**：`ready` 与 `in_progress` 都是城在动，`blocked` 与 `awaiting_approval` 都是城停下来等人，而分辨它们的是词，不是颜色（7-1 的 badge 行）。代价是 done 不再比 not_started 更暗；这不是损失，因为那两个词本来就不同，而颜色按 7-1 只许重复词。

- **4-33 版式三则：内容对齐外壳，宽度按内容种类，网格列有最小宽。**
  **其一，正文列的左缘是左栏右缘加 `pane`，`mx-auto` 不再出现在任何页面容器上。** 左栏是每一页唯一的固定参照物，一个居中的岛在 1920 宽的窗口里离它三百多像素，人的眼睛要在两个坐标系之间来回跳；`mx-auto` 此后只允许出现在阅读列内部（对话线程、长文档）。**其二，宽度分三档按内容封顶，不按页面封顶**：`measure`（520）给段落、`talk`（760）给对话、`page`（1040）给带表格的表单，表格与代码块不封顶、随容器长到 `wide`（1120）。同一页的不同区块各取各的档——整页取最窄的那一档，正是设置页正文被挤成 360 px 的原因。**其三，一个网格列的最小宽度是 320 px**：`grid-cols-[repeat(auto-fit,minmax(320px,1fr))]`，不用断点，因为断点问的是容器而列宽问的是内容。表格的列另行规定：文本列 `min-w-[12ch]`、数字列 `w-figure`、id 列 `min-w-[24ch]`，超出容器就横向滚动——**永不逐字折行**，`wrap-anywhere` 从 `setup/models.svelte` 删除。
- **4-34 度量令牌：控件高度、图标网格、圆角、阴影两级、触达面。** 此前没有控件高度这条令牌，于是每个视图自己拼 `py-tight`／`py-snug`，同一行里三个按钮高 26、28、30 px。令牌是 `--spacing-control-sm|control|control-lg`（28／32／36）、`--spacing-glyph-sm|glyph`（16／20，图标画在哪个方格里就住哪个方格）、圆角 `control 6｜card 8｜panel 12`、阴影两级（`shadow-float` 弹层、`shadow-sheet` 抽屉与 dialog；`shadow-composer` 与 `shadow-float` 是两个名字一个值，前者在本条落地时删除）。**有影的面不画边，有边的面不画影**，弹层例外。触达面：桌面 ≥ 28×28、触屏 ≥ 44×44，小于这个的图标按钮用 `::before` 扩热区。图标收进 `parts/glyph.svelte` 一个 `Glyph name=…` 与一张路径表——六个文件各画各的 `<svg>` 是同一套图形的六个家；城市插画不是图标，留在原处。
- **4-35 通知是三个座位、一个组件。** 一切拒绝与提示都由 `parts/notice.svelte` 画，座位由调用方给：**inline**（有归属表单的拒绝，紧贴出错的字段，随字段编辑清除）、**toast**（无归属页面的拒绝，右下角，宽 `min(480px, 100vw − rail − 2·pane)`，至多三条，`role="alert"`，8 秒自动收起、悬停暂停）、**drawer**（栏顶圆点点开，贴左栏右缘的全高抽屉，宽 440，Esc 与点外部关闭）。**今天 AxError 的三段式在三处各手写一遍**（`views/notices.svelte`、`views/refusal.svelte`、`parts/notice.svelte`），本条把它收成一处。抽屉按天分组，每条是标题（`lang.json` 的 `err_<code>`，如 `err_E_CONFIG_INVALID`；页面自己等不到回答的问题是例外，`E_TIMEOUT` 的 subject 读得出一个 `Query` 时标题取 `ask_late_title`，写出那个问题的线上名字，因为一页同时问好几个问题，同一句「等得太久」看两遍的人分不出城漏答的是哪一个；规则在 `parts/notice_title.ts`）、时间、同 `code+subject` 的计数徽标，英文原句折叠进等宽详情。**动作由 `core/recovering.ts` 的一张表从 `code` 映射到动词**，toast 与抽屉都读它——两个读者各写一张表就是同一个事实的两个家。动作有四种臂：命令（按自身拼写）、`reconnect`（让链路再试）、`settings`（去设置页选模型）、`reload`（`location.reload()`，取这座城构建时的客户端）。`E_WIRE_MISMATCH` 只给 `reload`：两端对线上格式意见不一，重连只会再撞上同一处分歧；草稿按地点存在 `localStorage`（`prefs.ts` 的 `draft`），重新载入后仍在。**动作只作用于 composer 所在的房间**（`views/notice_recovery.ts`）：`/new` 与 `/fork` 的房间取自地址栏——对话页的地址，或 run 页那个 run 的房间——从不取自拒绝的 subject，因为地址语法接受一句带空格和反引号的话，把 subject 当地址读会在一句错误原文上开出一栋楼；`/stop` 只在 subject 是 run id 时出现，停的也只是那个 run（run id 的语法窄到装不下一句话）；subject 只指房间的拒绝不给 `/stop`（`recoveryFor(error)` 按 subject 判），因为停房间是 `/halt`，一个永远跑不起来的控件不是动作；没有 composer 的页面上动作置灰（`act_no_target`）。**toast 在右下而不是左下**：左下压着左栏展开后的悬停区。**链路丢失不是通知，是页面所处的状况**：`views/link_banner.svelte` 用 `parts/banner.svelte` 画在「城已暂停」的同一位置（主区之上，两者同时成立时断线在上），写出第几次重连与 `unsent` 里等着的条数，唯一的动作是「现在重试」（取消阶梯的等待、立即重连）；横幅从 `backoff` 出现，到 `live` 或 `refused` 才撤，其间每次 `opening` 不闪掉。
- **4-35a 恢复动作可以打开一张预填表单，由人提交。** `core/recovering.ts` 的 `Recovery` 在 4-35 的四种臂之外还有 `form`——`{ kind: "form", label, words, room }`：`label` 是按钮上的 `lang.json` 键，`words` 是预填正文的键（槽 `{building}` 与 `{name}`），`room` 是 `"mayor"` 或 `"building"`，指表单开在哪个房间。按下它把填好的正文写进那个房间的草稿门（`PreferenceDoor.setDraft`，与欢迎页 `assign work`、`/fork` 回填同一扇门），再把地址栏移到那个房间；发出去的仍是人按下发送的那一次 `dispatch`。**不进审批队列，也不绕开门**：表单只替人写好字，city-SPEC §12.1 一字不改。**`form` 臂只服务 `subject` 读作 `<楼地址>: <缺失的名字>` 的拒绝码**——地址文法不含 `:`，所以第一个冒号就是分界，冒号后去空白非空才算读到；读不出这两样时按钮置灰（`act_no_target`），不猜。逐码核对的结果：只有 `E_PLAN_MISSING` 的 `subject` 全程是这个形状（`<楼地址>: <常设目标>`，由设常设目标的那一处唯一抛出），它的行是 `{ kind: "form", label: "act_ask_plan", words: "form_ask_plan", room: "mayor" }`：按钮「让市长写计划」把「为 {building} 写一份计划，让它的就绪步骤朝向：{name}」填进市长的草稿。`E_CREDENTIAL_MISSING` 的 subject 依出处是 URL、provider 名、`secret:` 引用、MCP server 标签或一句话，都不带楼地址，所以它没有 `form` 行。
- **4-36 设置页是左锚定的两栏，`config.toml` 折进正文底部；每个设置项是一张卡。** 分组导航 200 px 竖排、当前组 `aria-current="page"` 加左侧 2 px accent 条（7B 允许的第二处）；正文 `flex-1 min-w-0` 左对齐，每组一个 `<h1>` 与一行说明（4-10 的例外：这行说的是这一组此刻管什么，不是这一屏是干什么）。`config.toml` 从常驻第三栏改为正文底部的可折叠区块，默认收起——**它是校对工具而不是设置项**，常驻占 300 px 是正文被挤到 360 px 的直接原因。**卡片语法**：标题（label 600）＋一句说明（note faint）＋控件＋卡脚（左：一句约束或状态；右：需要提交的才有按钮），立即生效的控件没有按钮，改动后卡脚出现「已保存 ✓」。今天同一屏里 `title`／`heading`／`note` 三级标题叠在 80 px 内的写法随之取消：**一屏一个 `title`，其下只用 `label`**。


**宽度按内容种类分档，不按页面封顶。** `setup/groups.ts` 的 `WIDTH` 是这张表的唯一权威：`accounts`、`tools`、`skills` 是 `page`（表与卡片网格），其余七组是 `measure`（段落与分段控件）。**账户组有两个例外，两个都是内容种类给的**：供应商表单占 `talk`（760）而不是 `measure`，因为那里贴的是 base URL 与密钥、常常六十多字符，520 会把它们截断；`等价的 config.toml` 在 `@wide/page:`（≥1120）下挪到表单右侧的 `w-tree` 列，因为校对材料该在被校对的东西旁边，窄容器下它回到正文之后。**一个名字隔阱记在 `theme.css` 里**：`max-w-wide` 取的是间距档 `--spacing-wide`（24 px）而不是 `--container-wide`（1120），页面上任何 `max-w-wide` 都会把整列压成 24 px；能安全指名的只有 `page`、`measure`、`talk`。
- **4-37 字栈不指名任何 CJK 面，也不随包发一个。** 汉字落到这台设备自己有的面上，因为那正是引擎对一个指名面都没有的字形会做的事，而平台自己的选择是唯一按本项目能接受的条件拿得到的一个。**两条条件各自单独就足以定下来**：许可上，这条字栈只能指名客户端可以再分发的面（`fonts/OFL.txt`、`docs/third-party.md` 第 4 节），而 Windows 与 macOS 上人真正有的中文面是它们厂商的；尺寸上，一个值得指名的面按厂商原样是 17,773,244 B、过 `gzip -9` 是 11,266,972 B，是 `tools/xtask/budgets.toml` 给整个前端产物那一档（`frontend_artifact`）的数倍。**代价很小**：回退面的基线与 x-height 与随包面不同，而这只在一行里同时出现拉丁字与汉字时看得出来；指名一个 CJK 面并不能取消这件事——它只会让结果取决于那台机器恰好装了哪些字体。**中文的尺寸与行高照旧另计**（`theme.css` 的 `:root:lang(zh)`）：注释步不再减 1 px、行高 1.6、字距归零——那三条说的是同一个字号下汉字比拉丁字密得多，与用哪个面无关。
- **4-39 「用我的编辑器打开」只列厂商自己的文档或源码读得懂 `文件:行` 链接的编辑器，编辑器与城的文件夹由这个浏览器保管。** 猜来的协议会静默失败——浏览器去找一个没人装的程序，或者把文件开在第一行——所以每一项都要有出处。VS Code 的 <https://code.visualstudio.com/docs/configure/command-line>（“Opening VS Code with URLs”一节）写明 `vscode://file/{full path to file}:line:column`，并写明 Insiders 版的前缀是 `vscode-insiders://`。这个处理器在 VS Code 源码 `src/vs/code/electron-main/app.ts` 的 `getWindowOpenableFromProtocolUrl` 里，按 URL 的 authority 是 `file` 来认，协议名是构建在 `product.json` 的 `urlProtocol` 里注册的那个；所以继承它的构建用同一形状、换自己的协议名：VSCodium 的 `prepare_vscode.sh` 把 `urlProtocol` 设为 `vscodium`；Cursor 的工作人员在论坛帖 <https://forum.cursor.com/t/remote-uri-opens-a-new-window-every-time/166093> 里称本地的 `cursor://file/...` 链接照常工作；Windsurf 没有公开这一处的文档，它注册 `windsurf` 协议，LocatorJS 等工具按同一形状生成 `windsurf://file/...:行:列`；这是七项里出处最弱的一项，一旦证明它不按这个形状打开就删掉。Zed 的文档（<https://zed.dev/docs/reference/cli>）只写了命令行的 `文件:行`，读 `zed://file` 的是源码 `crates/zed/src/zed/open_listener.rs`：去掉 `zed://file` 前缀、解码后按 `路径:行:列` 打开。**Zed 在 Windows 上要换一种写法**：去掉前缀后剩下 `/C:/...`，Windows 拒收这个名字（os error 123）；`//?/C:/...` 是同一路径的设备形式，Zed 能打开，其中 `?` 写成 `%3F`，免得浏览器把它读成查询串。这一条在 Windows 上的 Zed Preview 1.22 实测过。JetBrains 系不列：`idea://open?file=` 这类协议只在 macOS 上注册，Toolbox 的 `jetbrains://<工具>/navigate/reference` 要项目名而不是路径；Sublime Text 没有自带协议。哪天这些编辑器自己支持「文件:行」链接，在 `EDITORS` 加一项、在这里补上出处。选择控件是原生下拉列表而不是分段控件：七个选项放不进设置卡片里一条等宽的轨道。**保管在浏览器而不是城的 `config.toml`**：装了哪个编辑器、城在那台机器的哪个文件夹，是浏览器所在机器的事实，同一座城从另一台机器打开时这两个值不同；而线协议不带任何机器上的绝对路径（`wire.ts` 里 `Address` 之外不传路径）。代价是人要在设置里填一次城的文件夹。重开参数：线协议给页面城在浏览器所在机器上的根路径时，`cityFolder` 改读它并删掉这一行存储。**只有城里的路径得到链接，而这条判断是字面的**：路径按生成的 `Address` 语法判，文件夹须是绝对路径、不是 UNC 共享（`//host/share` 的主机不是文件夹，当成文件夹会把链接指到另一个文件）、不含 `.` 与 `..`；磁盘上指向城外的符号链接不跟随，因为浏览器看不到磁盘，城也不启动任何东西。
- **4-40 重连按水位续传，差距大才整页重问。** `socket.ts` 记下本页折过的最大 `seq`（全城水位）。welcome 的 `resume_from` 是服务端账本头的 `seq`：水位已知、头在水位之后、差距不超过 `RESUME_PAGES × GAP_PAGE`（两页，400 条）时，缺口 `水位+1..头` 排进 `gaps`，走 `HistoryRange` 逐页补拉，补回的每条记录经 `asking.invalidate` 只失效它影响的答案，`asking.resumed()` 只重发断线时在途的问题；水位未知、`resume_from` 缺席或差距超过两页时回退到快照，即 `asking.reconnected()` 把每个答案标旧重问。阈值两页（`RESUME_PAGES`）是估计，不是读数：两页以内补拉的字节估计少于把一页上所有被看着的问题重问一遍，超过两页时一次快照估计比逐页补拉更快到达当前状态。能定下它的读数是同一座城上一次快照重问的字节数与一页 `HistoryRange` 的字节数之比（§3-4）。welcome 的 `epoch`（创世记录的链哈希）与本页上次见到的不同时，水位属于另一份账本：`belief.forget()` 丢弃全部折叠、水位与缺口清空，再按快照重问，因为旧水位在新账本里指向的是别的记录。

- **4-41 斜杠动词是一张表，每条自带分组。** `core/slash.ts` 的 `SLASH` 是页面动词的唯一权威，`/` 菜单与 Ctrl-K palette 都读它；`Slash` 记录有五个字段：`spelling`、`grammar`、`about`、`section`（`actions`／`navigation`／`sessions`，类型 `Section`，类型检查强制每条填写）、`run`。palette 按 `section` 分组，不另立以拼写为键的表——另一张表会让改名的动词静默落进 actions。glossary 的 Halt 与 Cancel 是两个动词，页面随之分开拼写：`/stop` 无参，只对眼前的 run 发 `cancel`；`/halt [addr|--all]` 与 `/release [addr|--all]` 成对，带地址的作用于那栋楼，`--all` 与无参作用于整座城（`halt`／`release` 帧）；第一个词既不是 `--all` 也不是合法地址时不发任何帧、行留在输入框里待改，因为把打错的地址放大成整座城是这个人不可能想要的读法。Ctrl-K palette 同理：动词既没发出帧、没换页、也没写回一行时 palette 不关，行留着待改；只有做了事或清空了行才关。notice 上的 `/stop` 恢复同样只对 refusal 所指的 run 发 `cancel`，只指房间的 refusal 不给这个恢复（4-35），不升级为 `halt`。`/clear` 等于 `/new`：丢掉对话、在此地址开一个什么都不带的新 session，与各家 harness 的 `/clear` 同义，不再只清空输入框。`/steer <text>` 的语法里没有 `--queued`。`/diff` 打开眼前 run（没有时取此房间最新的 run）的 run 页，changes 视图是那一页的一个 lens。
- **4-42 run 页是时间透镜加顶部统计栏，不展开账本树。** 统计栏（`run/head.svelte`）是一行带标签的事实：结局、用时、token 入/出（缓存另记）、花费（回合数另记）、来处（楼 / 房间）；每个数都由页面已经问到的 `RoundsAnswer` 求和，所以栏与下面的透镜说不出两个数。时间透镜（`run/river.svelte`，默认页签）按份额画三条并行轨道——模型在说、工具在跑、在等人——下面是这次 run 的全部工具调用。**工具调用带自己的时钟**：`Call.called`／`Call.answered` 是账本写下调用与结果的时刻（wire-SPEC §8-47），所以调过工具的回合在量到的地方切开——`Turn.t` 到第一次调用归模型，到最后一个结果归工具，其后归模型，调用列表每行带它量到的用时；**等人从量到的那一刻开始**：`Note::waiting.t` 是账本写下批准请求的时刻，`Note::waiting.answered` 是答复记下的时刻，等过人的回合在这两处切开——请求之前照调过工具的回合切法，请求到答复归人，答复之后按那之后发出的调用再切；答复时刻为 `null`（窗口外或读不回来）时请求之后整段归人，而不是按没人量过的长度再切一刀。**页面持有的盒子有上界**：`run/lanes.ts` 的 `columnsOf` 每列二分取样一次再按份额合并，一条轨道最多一列一个盒子；调用列表经 `windowOf` 只画视口内的行加两侧各 `OVERSCAN`（8）行，一万次调用的 run 与四十次的 run 在页面里是同一个量级。**透镜读的尺寸不回头喂自己**：轨道宽与调用列表高由同一个 `ResizeObserver` 报出，在下一帧才写进状态；行高是 `--spacing-control-sm` 这条令牌，未画行的占位也按这条令牌计，所以列表的总高只随调用数变，画出哪几行都不改它。备选是量画出的行再除以行数：那个高度又决定画哪几行，浏览器在同一轮报告里看到尺寸再变，就报 `ResizeObserver loop completed with undelivered notifications`，`#/gallery` 在 `xtask render` 里因此没法量。备选是逐段画、逐行画：一万个节点在 390 px 宽的轨道上大多窄于一个像素，却要付全部的布局与内存。统计栏的「模型」一格读 `Turn.model`（wire-SPEC §8-47）：最后一个回合问的模型，run 中途换过模型时按出现顺序列出各个名字。统计栏的「由谁派来」一格读 `Opening.dispatched_by`（wire-SPEC §8-48）：`person`、`city` 或派活居民的地址，原样以等宽字画出；为 `null`（旧账本、窗口外）时不画这一格。

## 5 `src/core/`（形状按 ARCHITECTURE §9）

| 文件 | 形状 | 接口 |
|---|---|---|
| `link.ts` | 1 判定 | `newLink(token, lang)`, `connect(link) -> [Link, LinkAction]`, `advance(link, LinkEvent) -> [Link, LinkAction]`；`LinkAction` 穷尽（open／send／welcomed／deliver／answered／saying／wait／report／close）；`isLive(link)`、`isRefused(link)`；`backoffMs(attempt)` 读阶梯 `[250,500,1000,2000,5000,10000]`；`unreadableRecord(lang, at)` 与 `unsentCommand(lang, verb)` 是链路自己写出的两种 `AxError` |
| `unsent.ts` | 2 值 | `isSpeech(command)`、`createUnsent() -> { count: Readable<number>, hold(command), release(send) }`：断线时人说的话按序排队，`welcomed` 时 `release` 逐条发出，某条发不出就停在那条、其余留着；`count` 是断线横幅写出的条数。**排着的话只在内存里**：断线期间关掉或重新载入这一页，它们就没了，而输入框在交给 `unsent` 时已经清空了草稿；要让它们活过重新载入，得经 `prefs.ts` 的 `draft` 门存下，今天没有这样做 |
| `socket.ts` | 4 适配器 | `openConnection(url, token) -> Connection { state, belief, asking, unsent, command, retry, dismissRefusal }`（`state`／`belief`／`unsent` 是 `Readable`，其余是函数）；链路在 `opening`／`handshaking`／`backoff` 时 `command` 把人说的话（`dispatch`／`steer`）交给 `unsent` 并答 `true`，其余命令仍答 `false`——停下、释放、授权这类动作等到重连后才生效，可能已不是人按下时想要的；链路在 `refused` 时一律答 `false`，因为阶梯不会自己走到 `welcome`，而 `E_WIRE_MISMATCH` 唯一的动作是重新载入，它会丢掉只在内存里的 `unsent`——答 `false` 让输入框留住草稿；`retry` 先取消阶梯排好的尝试再重连；`tokenIn(search)`, `socketUrl(location)`, `bearing(token)`——POST 递配对码的唯一拼写（`Authorization: Bearer`，服务端读者是 `wire::reception::offered_pairing`） |
| `gap_walk.ts` | 4 适配器 | `createGapWalk(ask, store, asking, lang) -> GapWalk { fold(record), lagged(from, to), welcomed(epoch, head), answered(askId, outcome) -> boolean }`：缺口补拉与水位（设计 4-40）。`socket.ts` 折的每条记录都经 `fold`，所以水位总是重连续传的起点；`answered` 只收它自己那一问（按 id 认），答 `false` 的交给 `asking`；重连落在阈值内时，尚待补拉的 `lagged` 缺口改为逐条失效，因为看过它的答案随旧 socket 一起作废 |
| `answered.ts` | 1 判定 | `readAnswer(answer, pick) -> Answered<T>`，`Answered = asking \| held { value } \| unavailable { query }`：一个视图只画一种变体，槽里的其余答案仍是答案——`Answer::Unavailable` 带回城拼写的那一问，别的变体以自己的键名为 `query`；视图用 `views/parts/unanswered.svelte` 画它，恢复是 `asking.refresh` 再问一次。不折成「还在问」或「空」，因为那两种读法把「城没能看」说成「城在忙」或「城是空的」 |
| `frames.ts` | 4 适配器 | `decodeFrame(text) -> ServerFrame \| null`（event／delta 帧走窄校验快路）, `encodeFrame(ClientFrame)` |
| `staleness.ts` | 1 判定 | `reachOf(name, kind) -> Reach`（`every`／`none`／`same_run`／`newest_page`）与 `reaches(reach, key, run)`：事件到查询的失效表，按问题名判，不按每条答案判 |
| `asking.ts` | 1 判定 | `createAsking(send) -> Asking { ask(query) -> Readable<Answer\|undefined>, refresh, answered, invalidate(record), reconnected, resumed }`（`reconnected` 标旧重问全部，`resumed` 只重发在途的；`Readable` 皆为 `svelte/store` 面，模板以 `$` 订阅）；`QUERIES` 是每个问题名的唯一拼写，`COMMITS_PAGE` 与 `commitsQuery(building, before)` 见 4-15，`HELD_CAP` 是页面保留的答案数上界（超过时丢掉最久没用、也没人看着的那一个，所以开了一周的标签页与第一天持有一样多），`keyOf` 是问题的合并键，`askedIn(subject)` 从本页自己写出的拒绝（如 `E_TIMEOUT`）的 subject 读回那个问题的线上名字；答案按内容匹配问题，无名者按到达序；失效按 `staleness.ts` 的表 |
| `belief.ts` | 7 投影 | `createBelief(now) -> BeliefStore { belief, adoptCity, apply(record) -> string \| null, say(delta), logged(line), refused(error), named(city), noticesSeen, forget, batch(folds) }`——`now` 是取时刻的那一个入口；`batch` 里的折叠只在最外层结束时 `set` 一次，`socket.ts` 的 `drain`（连同其中 `filled` 折的缺口页）与 welcome 各是一批，所以两帧之间的一串记录是一次更新、一次重绘；`forget` 在账本换了（welcome 的 `epoch` 变了）时丢弃全部折叠。**`apply` 答的是它读不出的字段名**（如 `tool_called.name`），`null` 才是读全了：一份形状不对的载荷仍然推进位置，但不静默当作缺席 |
| `belief/shape.ts` | 6 数据 | `Belief { runs, live, rooms, cancelled, halted, haltedAt, refusal, notices, city, sessions, probed, logs }`；`RunBelief { run, addr, started, task, lastSeq, doing, model, pr, ask, local, saying, thinking }`；`Notice { error, seen, at: TimeMs, key, count, about: Address \| RunId \| null }`；`merged(notices, error, at)` 按 `key`（`code + subject`）合并同文并计 `count`，`at` 取首见时刻；`LOG_WINDOW` 与 `NOTICE_WINDOW` 是页面持有多少日志行与通知的唯一答案 |
| `belief/fold.ts` | 7 投影 | `unseen(run, at) -> RunBelief`（页面第一次见到的 run）、`fold(held, record) -> [RunBelief, string \| null]`：一条记录对一个 run 做了什么；记录属于哪个 run、是不是新的，由 `belief.ts` 判 |
| `belief/adopted.ts` | 7 投影 | `adopted(summary, held) -> RunBelief`：一行 `city_view` 读成 belief，流已经知道而行没带的字段留着；`adoptCity` 与 run 页（经链接到达、从没流过的 run 只有这一行）共读 |
| `belief/runs.svelte.ts` | 7 投影 | `runTable(held) -> Record<RunId, RunBelief>`：run 表是 `$state`，每个 run 是一个响应式对象。`say(delta)` 对已持有的 run 就地追加 `saying`／`thinking`，不重发 `belief`，只唤醒读这个 run 这个字段的读者；只有 delta 带来新 run（表的形状变了）才发布一次。记录的折叠与 `adoptCity` 仍整值写入并发布。测试经 `client/bunfig.toml` 预载的 `scripts/runes.ts` 用 `compileModule` 编 `.svelte.ts`（含 `*.svelte.test.ts`），与 vite 进产物同一编译器 |
| `belief/live.ts` | 7 投影 | `Belief.live: readonly RunBelief[]`——没冻结的 run，按 `started` 从旧到新，是「哪些 run 在干活」的唯一权威；`livened(live, run) -> readonly RunBelief[]` 在每次折叠里按这一个 run 改写它，O(L)（L 为在干活的 run 数），`liveOf(runs)` 只在 `adoptCity` 整表换写时 O(R) 重建；`within(run, room) -> boolean` 与它读的 `inside(addr, room) -> boolean` 是「在这个房间或其下」的唯一拼写；`newestWorking(belief, room) -> RunBelief | undefined` 从 `live` 取这个房间最新的在干活的 run，是输入框 steer、`/stop` 与房间页「正在进行」三处共读的唯一答案，O(L)。视图读 `$belief.live` 而不再各自 `Object.values(runs).filter(…)`：R = 1e4 时一次记录折叠加一次读「在干活的 run」≤ 20 µs（`belief/live.test.ts`） |
| `belief/rooms.ts` | 7 投影 | `Belief.rooms: ReadonlyMap<string, readonly RunId[]>`——每个房间持有过的 run（在干活的与已冻结的），按 `started` 从旧到新，只存 `RunId`，所以一次折叠只在 run 进表、换房间或换开始时刻时改写那一个房间的列表，O(k)（k 为该房间的 run 数），其余折叠不动索引；`adoptCity` 整表换写时 O(R log R) 重建。`heldIn(belief, room) -> RunBelief[]`（恰在这个房间）与 `heldWithin(belief, room) -> RunBelief[]`（这个房间及其下，按 `inside`）是房间页、目录、城市面板与天际线共读的答案，读者付 O(房间数 + k)：R = 1e4、百个房间时一次记录折叠加一次读一个房间 ≤ 500 µs（`belief/live.test.ts`）。同一开始时刻的两个 run 按进表先后排 |
| `belief/cancelled.ts` | 7 投影 | `Belief.cancelled: number`——冻结原因是 `cancelled` 的 run 数（线协议不带一次停机冻结了多少，停机写的正是这个原因）。`recounted(count, was, run) -> number` 在每次折叠里只看这一个 run 的前后两次读数，O(1)；`cancelledOf(runs)` 只在 `adoptCity` 整表换写时 O(R) 重数。页面的停机行读它：R = 1e4 时一次记录折叠加一次读 ≤ 500 µs（`belief/live.test.ts`） |
| `doing.ts` | 2 值 | `Doing = unknown \| thinking \| calling { tool: string \| null, subject } \| waiting \| frozen { completion }`、`Sending = dispatch \| steer \| queued`、`sendingInto(doing)`；`MOVING`／`moves(kind)`／`PHASES` 是「哪些 kind 陈述姿态、各自陈述什么」的独家表，流与答案两条路都读它 |
| `landing.ts` | 1 判定 | `sentFrom(from, task, runs) -> Sent`（发出那一刻记下房间、任务原文与已知的 run）、`landingOf(sent, runs) -> Landing`，`Landing` 穷尽（`pending`／`here`／`elsewhere { run, addr }`）：`run_started` 之后第一个「发出时不认识、`task` 与原文相同、`addr` 是发出的房间或它下面任一层的房间」的 run 就是这次派的活；落在原房间时线程已经画出它，落在别处时 `talk/landed.svelte` 在原地留一行可点的去处（见 12-6） |
| `reading.ts` | 4 适配器 | `taskOf(record)`、`toolCall(record)`、`modelOf(record)`、`completionOf(record)`、`askOf(record)`、`branchOf(record)`、`haltOf(record)`，各答 `[值, 读不出的字段名 \| null]`；`sessionStart(record) -> SessionStart \| null`（不是会话开头、或没说房间的记录答 `null`）；`kernel::event::record` 的字段名与 serde 属性（`Option` 与 `#[serde(default)]` 各是什么意思）在客户端只有这一处拼写 |
| `scope.ts` | 4 适配器 | `scopeOf(spelled) -> HaltScope \| null`（Ledger 拼法→frame 拼法，唯一相遇点）、`sameScope`、`buildingIsShut`、`cityIsShut`、`CITY`；`CITY` 是两套拼法共同的那一个词，五个视图改读它，不再手写 `"city"` |
| `run_id.ts` | 4 适配器 | `readRunId(raw) -> Option<RunId>`：`Schema.decodeOption` 于生成的 `RunId`，地址栏与转写文件名的唯一文法 |
| `commands.ts` | 2 值 | 每个命令帧一个构造函数，自铸 `IdemKey` |
| `enrol.ts` | 4 适配器 | `enrol(Enrolling { origin, token, realm, name, value, lang }) -> Promise<Enrolment>`；`referenceFor(provider)` 是 realm／name 唯一的选词处（realm 是本页选的词，城只判字母表），`referenceText(at)` 是页面预演用的唯一拼法，`keyField`／`secretFor` 保证存的引用只用在它被归档的那个 id 上。**引用来自城**：201 正文是 `kernel::SecretRef` 读回后写出的那一句，本页不自己拼一份存起来 |
| `speaking.ts` | 4 适配器 | `canRecord()`, `record(origin, token) -> Promise<Recording \| null>`；`Recording.stop() -> Promise<Heard>`，`Heard` 穷尽（text／refused／silent）；`dictation(origin, pairing, into) -> Dictation`：composer 的麦克风，听到的一句交给 `into`，落进输入框而不直接发出（4-16） |
| `idem.ts` | 2 值 | `mintIdem()` |
| `mark.ts` | 4 适配器 | `markOf({ waiting, working, link })` 判定四态：链路不在 `live` 时为 `untold`（页面此刻没被告知，城里的事可能已经变了，图标不替没人说过的话作证），否则有待人决定的事为 `waiting`、有 run 在动为 `live`、其余为 `quiet`；`paintMark(document, mark)` 把令牌解算成引擎实际会画的颜色，由 `markSvg` 拼成 SVG data URL 写进 `<link rel="icon">`。四态各是一个形状——`quiet` 空心环、`live` 实心圆、`waiting` 菱形、`untold` 一道横杠——因为标签栏很小，分不清ACCENT与警示色的人仍分得清环与菱形；零颜色字面量 |
| `rows.ts` | 4 适配器 | `Rows { getItem, setItem, removeItem }`、`memory()`、`browserRows()`：浏览器存储那一扇门，三处会抛的拒绝（禁用存储、配额为零、写时配额满）在这里各变成一个值 |
| `prefs.ts` | 6 数据 | `Preferences { lang, welcomed, panel, rail, appearance, proxying, notifying, showing }`、`Rail`、`RAILS`、`PROXYING_RULES`、`NOTIFYINGS`、`Keeper = "browser" \| "city"`、`PreferenceDoor { held, keeper, adopt, tell, setLang, setWelcomed, setPanel, setRail, setAppearance, setProxying, setNotifying, setShowing, chord(action), setChord, draft(at), setDraft, editor() -> { editor, folder }, setEditor }`、`loadPreferences(rows, browserLang)`、`preferences()`；**全客户端每一个存储键的拼写都只在这个文件的 `ROWS` 里**（草稿键 `sprawling.draft.<房间或 run>`、快捷键 `sprawling.key.<action>`） |
| `appearance.ts` | 2 值 | `Appearance { lighting, sans, mono, sansStack, monoStack, body, density, chroma, motion }` 与各项的词表（`LIGHTINGS`、`FACES`、`DENSITIES`、`CHROMAS`、`MOTIONS`，按选择器画出的顺序）、`STACK_SHAPE`：一条外观记录怎样才算合法；`prefs.ts` 负责存取，`prefs_city.ts` 负责带到城 |
| `sizing.ts` | 1 判定 | `BODY_PX`、`sizingOf(text) -> Sizing`（`cleared` \| `sized { px }` \| `refused`）：人写进字号框的一串字读成什么；偏好的读取与外观组共用这一处。 |
| `editor.ts` | 1 判定 | `Editor = "none" \| "vscode" \| "vscode-insiders" \| "vscodium" \| "cursor" \| "windsurf" \| "zed"`、`EDITORS`、`editorLink(Opening { editor, folder, path, line }) -> string \| null`：「在我的编辑器里打开 文件:行」的唯一拼法，各编辑器的链接前缀由同文件的 `fileUrl` 给出（4-39），监视器的改动块拿它当 `href`，由浏览器把链接交给浏览器所在机器上注册了该协议的编辑器，服务端不启动任何程序。`path` 用生成的 `Address` 判（城内相对路径，无 `..`、无盘符、无反斜杠），`folder` 须是绝对路径且无 `.`／`..` 段，`line` 须是正整数；任何一条不成立答 `null`，页面不画这个链接。编辑器与城在浏览器所在机器上的文件夹由 `prefs.ts` 的 `editor()` 与 `setEditor` 保管 |
| `results.ts` | 1 判定 | `Showing = whole \| results`、`drawsCalls(showing)`（房间在 `results` 下不挂载 `calls.svelte` 与推理折叠）、`Outcome = waiting \| failed \| done \| ended`、`outcomeOf(run)`、`resultsOf(runs, first) -> Group { outcome, first, total }[]`（一遍分四类，每类按 `started` 新到旧只留前 `first` 条，`FIRST = 5`）；`bandsOf(runs, now) -> Band { recency, runs }[]`（城在 `results` 下按时间读：新到旧，切成最近十分钟、这一小时、更早三段，空段不画；没有 `started` 的 run 落在「更早」末尾）；`producedOf(files) -> Produced { files, added, removed }`（房间在 `results` 下结局分隔线之下的产出一行：改了几个文件、共加减几行；二进制文件计入文件数、不计行数，因为 `Lines::binary` 没有行数可加）；只看结果模式「画什么」的唯一判定处。200 个 run 的夹具城分类耗时由 `results.test.ts` 判定并打印 `city_results` 行，登记于 `tools/xtask/budgets.toml` |
| `prose.ts` | 1 判定 | `blocks(text) -> Block[]`, `inline(text) -> Inline[]`：Markdown 读成数据，永不 innerHTML；`closedUpTo(text) -> number`：流式文字里已经闭合、可以按块画出的前缀长度 |
| `route.ts` | 1 判定 | 见 §3-2 |
| `lang.ts` | 6 数据 | 见 §3-1 |
| `time.ts` | 1 判定 | `ago`, `clock`, `hhmm`, `hhmmss`, `lasted`, `count`, `usd`, `kib` |
| `notify.ts` | 1 判定 | `notices(heard, items, scene) -> [Heard, ApprovalItem[]]`：哪些待批事项变成一条浏览器通知。`Heard` 是「快照未到」或「已算过的 `ApprovalId` 集」；`Scene { notifying, focus, elapsed, watching }`。只对需要人决定的事（`approval_queue` 的答）发，四道闸全过才发：窗口失焦、过了预热期 `WARMUP_MS`、不在首个快照里也不在已算过的集里、不是正在看的那个地址（`item.actor`）。每个见过的 id 都记进 `Heard`，所以一件事在任何一道闸下被放过一次就永远不再弹。适配器是 `views/notifier.svelte`（权限为 `granted` 才 `new Notification`），开关是 `prefs.ts` 的 `notifying`，默认 `off` |
| `keys.ts` | 1 判定 | `ACTIONS`、`Action`、`Chord`、`DEFAULTS`、`LABELS`：外壳听的每一个键在这一张表里；`readChord(text)`、`spell(chord)`、`marks(chord, platform)`、`platformOf(userAgent)`、`matches(chord, pressed)`、`reserved(chord)`、`conflictsOf(bound)`；`loadKeys(door, userAgent) -> Keymap` 把人的覆写（`prefs.ts` 的 `chord`）叠在默认上，`keymap()` 是页面的那一份。左栏、外壳与设置页都读它，没有一处自己拼一个键 |
| `slash.ts` | 1 判定 | `SLASH`：页面动词的唯一表（4-41）；`parse(line) -> SlashCall \| null`、`find(verb)`、`offered(line)`（`/` 菜单与 palette 按输入过滤） |
| `slash_hands.ts` | 2 值 | `Slash { spelling, grammar, about, section, run }`、`Section`、`SECTIONS`、`SlashCall`、`SlashHands`（视图用手上已有的东西填它，动词从不点视图的名）、`Reached`／`reached(run)`（动词能作用的那个 run 与它走到的位置）、`Offered` |
| `completion.ts` | 1 判定 | `completed(line) -> string`：Tab 把一行按动词表补到所有匹配共有的最长前缀 |
| `forking.ts` | 2 值 | `forkAsked`（计数的 store）与 `askFork()`：`/fork` 不带地址时向对话页要选行器，对话页看着这个计数打开它；计数而不是布尔，因为选行器开着时的第二次请求也要被听见 |
| `recovering.ts` | 1 判定 | `Recovery`（命令／`reconnect`／`settings`／`reload`／`form`）、`recoveryFor(error) -> readonly Recovery[]`、`linkRecovery(code)`、`formOf(room, subject, words) -> Option<Form>`：拒绝到出路的唯一表，toast 与抽屉都读它（4-35、4-35a） |
| `probed.ts` | 4 适配器 | `readProbed(data) -> Probed \| null`：`endpoint_probed` 记录的载荷在这里收窄一次，离开这个文件的是类型；`normalisedFrom(probed, typed) -> Normalised \| null`（城把人打的地址补成了什么）、`stoppedAt(reach) -> Key`（探测停在哪一步的说法） |
| `share.ts` | 1 判定 | `fraction(ppb)`、`percent(ppb)`：城以十亿分之一计份额（`kernel::share::WHOLE_PPB`），这是客户端唯一拼写那个整体的地方 |
| `pursuit.ts` | 1 判定 | `pursuitClause(lang, verdict)`：城给出常设目标的判词种类，词由 `lang.json` 给 |
| `provider_failure.ts` | 1 判定 | `providerClause(lang, failure, retry)`：模型调用失败时人读的出路；失败的种类与值不值得再试由城判，词由 `lang.json` 给 |
| `removal.ts` | 1 判定 | `Removal = offered \| hall \| busy`、`removalOf(building, living)`：楼页给不给「移除这栋楼」；城自己拒两种情况，页面先问，免得给人一个只会答拒绝的按钮 |
| `live_output.ts` | 7 投影 | `Tail`、`NO_TAIL`、`LIVE_LINES`、`appended(tail, piece, cap)`：正在跑的命令已经写出的行，按行封顶、最旧的先丢并计数；该调用的 `tool_result` 一到就丢掉，因为账本里的结果才是那段输出的权威 |
| `monitor.ts` | 1 判定 | `rows(samples, width)`、`summary(latest)`、`sparkline(values, width)`：性能面板读监视器历史，画法与 `sprawling top` 相同，所以页面与终端对同一个样本读出同一个数 |
| `watching.ts` | 4 适配器 | `createWatching(sendText) -> Watching`：本页对监视器说的最大需要（面板开着 `watch`，只有事实条的摘要时 `watch_summary`，都没有时 `release`），新连接上再说一次；城只在有人看时采样 |
| `prefs_city.ts` | 4 适配器 | `keepWithCity(door, conn)`：偏好门与连接唯一的接点（4-29）；`adopted(held, answer)` 把城的回答盖在浏览器此刻的记录上，回答缺席的字段留浏览器的值；`appearanceOnWire(next)` 是外观记录的线上拼写 |
| `commands/endpoint.ts` | 2 值 | `Endpoint`、`Pair`、`Tuning`、`providerName(name)`、`probeEndpoint(e) -> Command`、`attachEndpoint(e, admit) -> Command`：挂一个端点要说的一张表单与它的两个帧；单列，因为只有这一族是人跨几屏填完才发的表单 |

`src/ui.ts` 是视图拿到的一切，一个上下文、一个取法：`setUi(value)` 由 `app.svelte` 挂载时调一次；后代组件在初始化期调 `ui(): Ui` 拿到 `Ui { conn, prefs, lang, effort, mode, approvals, bar, origin, pairing, now, chooseEffort, chooseMode, go, send, hearing }`。旧的 `useUi`／`useSay`／`useGo`／`useCommand`／`useHearing`／`useApprovals` 等透传壳收敛成这一个门（AGENTS：不做只改名的壳）。**词不是上下文**：`core/lang.ts` 的 `say(lang, key)` 保持纯函数，插槽由 `fill(pattern, slots)` 填，模板写 `say($lang, key)`，`$lang` 的订阅就是换语言时重画的来源。`pairing` 是开这一页的地址栏上的配对码，两扇会动作的 HTTP 门要它。

## 6 视图（免 SPEC，列出以便定位）

`views/parts/tip.svelte` 提示（见设计 4-18）；`views/parts/unanswered.svelte`（城没能回答的那一问，见 §5 `answered.ts`）；`views/parts/notice_title.ts` 通知的标题（见设计 4-35）；`views/parts/code.svelte` 只读代码视图（面包屑＋行号＋语法着色，见设计 4-26）＋ `parts/inked.svelte`（按语法上色的文字，文件视图与 Markdown 代码块共用）＋ `parts/code.ts`（五种 `Ink` 与取高亮器的入口）＋ `parts/paint.ts`（懒加载的高亮器块）；`views/rail.svelte` 左栏；`views/talk.svelte` ＋ `talk/{thread,calls,composer,waiting}.svelte` 对话 ＋ `talk/person.svelte`（人说的一句，画成一个气泡）＋ `talk/note_line.svelte` 与 `talk/note_line.ts`（线程里的一条注记——到达、拒绝、检查点、等人——按它在账本里的行排）＋ `talk/{record,send}.svelte`（composer 的麦克风与发送键）＋ `talk/asked.svelte`（审批卡问的内容：经 `Query::Content` 读 `ApprovalItem.artifact`） ＋ `talk/artifact.svelte` 制品面板 ＋ `talk/produced.svelte`（一个 run 的产出短语「n 个文件 · +a −b」，由 `opened_at` 到最后一次检查点的 `Changes` 答案求和；结果房间的块与结果城的行共用）＋ `talk/{stream,result}.svelte`（只看结果的房间：`stream` 按会话分组——开着的这段在前、以它怎么开始命名，其前的并为「更早的会话」，组内新的在前；`result` 是一个 run 一块：开始时刻、结局记号、任务，引出它最后说的话，末行写做完了与产出和 PR、失败或停下的原因与「没有提交」、等你与 `ask`、或在干活与打开监视）＋ `talk/trace.ts`（工具调用的分类，两个读者共用，见设计 4-26；`lastCheckpointIn` 是 run 最后一次检查点的树，run 页与产出一行都量到这里）；`views/city.svelte` ＋ `city/{bar,panel,skyline,marks,results,produced}.svelte`（`results` 是只看结果的城：按结局过滤的分段控件带各类总数，其下是 `bandsOf` 的时间段，一行写开始时刻、结局、房间、任务与产出或停下的原因；`produced` 是一行「刚完成」末尾的产出，向这个 run 的 `Rounds` 要 `opened_at` 与最后一次检查点，再交给 `talk/produced.svelte` 的同一次 `Changes` 求和——`RunSummary` 不带产出数字，所以只有画出来的前 `FIRST` 行在问；「刚完成」一行另写 `RunBelief.pr`，「等你」一行写 `RunBelief.ask`） ＋ `city/shape.ts`（超椭圆路径）＋ `city/table.svelte` 楼表（城页默认的第一视图，画是一键可切的第二视图：账本树的第一层，一楼一行，等你／在干活／完成／最后开始四个数，加一条折叠刻度上的时间条，每个 run 的开始是一个按阶段着色的刻点）＋ `city/table.ts`（`tableOf`：楼到行，行是定长摘要，只存计数与各 run 的开始，不存历史；楼的归属、排序与树线取自 `runs/lineage.ts`）；`views/runs/board.svelte` 全城 run 板（城页画在楼表或图之下：楼是这棵树的根）＋ `runs/lineage.ts`（run 表到账本树的行、只画可见行的窗口，`boardRuns` 只读板要画的五个字段，一个流式 token 不重建整块板）＋ `runs/fold.ts`（时间条的刻度）＋ `runs/phase.ts`（阶段的颜色与说法，run 板与楼表的时间条共用一份图例）；`views/registry.svelte`（`Query::RegistryView`：这座城决定留下来的东西，一行一件，见设计 4-24）；`views/building.svelte` ＋ `building/{tree,commits,rooms}.svelte`；`views/changes.svelte`（`Changes`／`Hunks` 的一份读法，run 页与楼页共用）；`views/run.svelte` ＋ `run/{head,river}.svelte`（顶部统计栏与时间透镜）＋ `run/lanes.ts`（时间透镜画什么：每个回合一段、每列一个盒子、只画视口内的调用行，见设计 4-42）；`views/setup.svelte` ＋ `setup/{providers,models,skills,appearance,keys}.svelte`（skills 组见设计 4-31）＋ `setup/kept.svelte`（一个组的答案由谁保管，见设计 4-29）＋ `setup/decided.svelte`（代为答复的记录：`GovernanceAnswer.decided`，画在审批控件之下）＋ `setup/groups.ts`（各组的顺序、标题、提示、宽度与导航，见设计 4-33、4-36）；`views/shared/{provider,effort,buildings}.svelte`（从设置页的组里拆出的三件：`provider` 是接供应方的那扇门，`effort` 说强度住在哪一层，`buildings` 是楼列，它的第二个座位是 `#/mcp`）；`views/shared/outcome.ts`（结局的记号与墨色，结果城与结果房间共用）；`views/shared/showing.svelte`（只看结果开关，三个座位：房间、城、外观设置，都写同一条偏好，所以页上的选择就是下一页的默认）；`views/machine.svelte`（doctor 的答）；`views/notifier.svelte`（不画任何东西，`core/notify.ts` 的适配器）＋ `setup/notifying.svelte`（外观组里的通知开关）；`views/desktop.svelte`（一栋楼的桌面白名单）；`views/mcp.svelte`；`views/welcome.svelte`；`views/record.svelte`；`views/cost.svelte`；`views/palette.svelte`；`views/refusal.svelte`；`views/prose.svelte`；`views/gallery.svelte`。

**`#/gallery` 是一条路由而不是一个构建开关**，因为量它的那道门应当打开一个人真正跑的 bundle；夹具不需要城（偏好走 `core/rows.ts` 那扇门，没有 localStorage 时是一张只活一次会话的表）。每个能进入多种状态的屏幕在那里各有一份夹具，`cargo xtask render` 打开真引擎读它。`app.svelte` 用动态 `import()` 取 `views/gallery.svelte`，所以画廊与它的夹具表是 bundle 里单独的一块，只在打开 `#/gallery` 时下载：其余路由首屏不再为它付字节，而 `frontend_artifact` 称的是整个 dist，这一块仍在其中。

**左栏是覆盖而不是推挤**：外层 `<div>` 只在钉开（`[`）时取 `w-rail-open`，`<nav>` 绝对定位、hover 时自宽并加投影；正文的左边因此不随指针越过左缘而重排。

**本节只说哪个屏用哪个部件；部件欠使用者什么写在 §7。**

**`views/parts/` 的五个复合部件各有生产座位**，`#/gallery` 只是它们的第二个读者：`combobox` 在 `setup/models.svelte`、`setup/model_table.svelte`、`talk/composer.ts` 与 `talk/pill.svelte`（一个端点答两百个模型时，下拉正是它替换的那个控件）；`notice` 在 `views/notices.svelte` 与 `views/refusal.svelte`，AxError 的三段式因此只有它一个画法（4-35）；`row` 在 `building/commits.svelte`、`mcp/servers.svelte` 与 `record/ledger.svelte`；`skeleton` 在 `machine/skeleton.svelte`。`dialog` 的座位是撤不回来的删除，它们在发帧之前先问（12-1）：删除 MCP 服务器（`mcp/servers.svelte`）与移除一栋楼（`building.svelte`，楼的文件随之搬出城）。设置页没有移除端点的控件，所以这一族里没有第三个座位；`part_remove_endpoint` 只由 `#/gallery` 的 dialog 夹具读。

## 7 `views/parts/` 的交互契约

> **这是规格，不是描述。** 表里写的是部件欠使用者什么；今天的代码欠而未还的十二处，逐条点名在 7-8。模式名与键表借鉴自哪几份文档、为什么不产生许可证义务，一处记在 `docs/third-party.md` §6，本节不复述。

**判定：不引入任何 UI 库依赖。** `tools/xtask/src/npm.rs` 的 `RUNTIME` 是这条判定的机器面——运行时依赖恰为那张表所列（今天是 `effect`、`svelte` 与画代码颜色的 `@lezer` 高亮器，名单以表为准；高亮器不是控件，见 4-26），要加一个组件库就得先改那一行，而**一道专为阻止依赖蔓延而设的闸，第一次例外就是它失效的开始**。理由不是保守：`parts/dialog.svelte` 已经把模态整个交给原生 `<dialog>`（top layer、焦点陷阱、Esc、其余页面 `inert`，四件都是平台承担的，见设计 4-20），`parts/tip.svelte` 已经把 `title` 换成一个 `role="tooltip"` 的兄弟节点（设计 4-18）。**这些正是一个组件库存在的理由，而平台现在自己提供了**；引一个库会让同一件事有两个提供者。判定失效的条件写在 7-9，一个字都不留给临时判断。

### 7-1 不收键的部件

这些部件不进 Tab 序列、不读键，只欠一个角色和一组确切的 `aria-*`。

| 部件 | 角色 | `aria-*` 的确切取值 |
|---|---|---|
| `badge.svelte` | 无（行内文本） | 圆点 `aria-hidden="true"`；状态由词承担，颜色只重复那个词 |
| `banner.svelte` | 实时区域 | `weight="alert"` → `role="alert"`；其余 → `role="status"` |
| `notice.svelte` | 实时区域 | 同上；`seat` 只改画法（浮起或列在中心），不改角色 |
| `empty.svelte` | 无 | 形状 `aria-hidden="true"`；那句话与那个动作是它全部的可读内容 |
| `progress.svelte` | `role="progressbar"` | `aria-label` 取调用方给的名字；`aria-valuemin="0"` 恒在；`total > 0` 时 `aria-valuemax="<total>"`、`aria-valuenow="<done>"`、`aria-busy="false"`，`total ≤ 0` 时这两个值一个都不写并 `aria-busy="true"` |
| `skeleton.svelte` | `role="status"` | `aria-label`、`aria-busy="true"`；每根条 `aria-hidden="true"` |
| `row.svelte` 的 `Row` | 无 | `onOpen` 在场时两段文字合成一个 `<button>`，右侧动作各自是独立的一站；行本身不收键，走动由 `RowList` 承担（7-4）|
| `kbd.svelte` 的 `Kbd` | 无 | 一个字形一个 `<kbd>`，不取焦、不收键 |

### 7-2 一次一个动作的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `button.svelte` | APG Button | Enter | 激活；`state() !== "idle"` 时 `onClick` 原地返回 |
| | | Space | 同 Enter：平台把两个键都送进同一个 `onClick`，所以一次判定挡住指针、Enter 与 Space 三种输入 |
| `path.svelte` 的显示控件 | APG Button | Enter／Space | 地址解析得出时发 `reveal`；解析不出时 `aria-disabled="true"`，点击落进一个空操作 |
| `field.svelte` | 有标签的文本框（无复合模式） | 平台的单行编辑键 | 由浏览器实现，本部件不截获 |
| | | ↑／↓（`kind="number"`） | 按 `step` 增减，由平台实现 |
| `changes.svelte` 的补丁行号栏（调用方给了 `talk` 地址时） | 链接 | Enter | 把「路径:新行号」（删去的行是「路径@旧提交:旧行号」）和该行的引文接到那个地址的草稿之后，再打开那段对话；composer 挂载时从草稿门读出它。每行一站 Tab，不给 `talk` 的页面行号栏不取焦 |

`button.svelte` 的 `aria-*`：`aria-disabled` 在 `loading` 或 `why` 在场时为 `"true"`，`aria-busy` 只在 `loading` 时为 `"true"`，`why` 在场时 `aria-describedby` 指向 `Tip` 的 id。**用 `aria-disabled` 而不是 `disabled`**：控件因此留在 Tab 序列里，键盘到得了它，读屏也读得到它为什么按不动。

`field.svelte` 的 `aria-*`：`<label for>` 给名字（`labelling="hidden"` 只把标签移出视线，名字仍在）；`aria-invalid` 恒等于 `error !== undefined`；`aria-describedby` 是调用方的 `describedBy` 与本格说明段 id 的并集，**错误替换说明而不是叠在它上面**，错误段自己带 `role="alert"`。红边有两条权威且说的是两件事：`error` 是城的回答，一到就红；`:user-invalid` 是浏览器读 `type` 与 `pattern` 的结果，失焦后才红。

### 7-3 提示与模态

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `tip.svelte` | APG Tooltip | Escape | **规格要求撤下提示；今天没有实现**（7-8 第 9 条）|
| `dialog.svelte` | APG Modal Dialog | Tab／Shift+Tab | 在对话框内循环，由平台实现 |
| | | Escape | 平台发 `cancel`，本部件 `preventDefault()` 后回调 `onCancel`——默认行为会绕过调用方关掉元素，而 `open` 还说着开着 |
| `kbd.svelte` 的 `Cheatsheet` | 手写的 dialog | Escape | 由外壳 `app.svelte` 的按键处理器答，不在部件里（7-8 第 12 条）|

`tip.svelte` 的 `aria-*`：提示节点是 `role="tooltip"`，id 交给调用方——控件自己有可见文字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`。**组件不猜**，因为只有调用点知道控件有没有名字。显示由 `:hover` 与 `:focus-within` 触发，延迟 300 ms；提示自己 `pointer-events-none`，永不取焦。

`dialog.svelte` 的 `aria-*`：`aria-labelledby` 指向标题，`aria-describedby` 指向说明段**且仅在 `detail` 在场时才写**。取焦由文档顺序决定：平台取对话框内第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。**点 `::backdrop` 不关闭**：撤不回来的那一问不该被一次落在外面的点击答掉。

### 7-4 在几件之间走动的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `segmented.svelte` | APG Radio Group（roving tabindex） | Tab／Shift+Tab | 进出控件；控件在 Tab 序列里只占一站 |
| | | → | 移到下一个可选格并选中它，走到末端回到开头 |
| | | ← | 移到上一个可选格并选中它，走到开头回到末端 |
| | | ↓／↑ | **规格要求同 →／←；今天没有实现**（7-8 第 5 条）|
| | | Space | **规格要求选中当前聚焦格；今天没有实现**（7-8 第 5 条）|
| `tabs.svelte` | APG Tabs（自动激活） | → | 下一个透镜并立即切换，走到末端回到开头 |
| | | ← | 上一个透镜并立即切换，走到开头回到末端 |
| | | Home／End | 第一个／最后一个透镜并立即切换 |
| | | Space／Enter | 与点击同一条路；因为切换已跟随焦点，它们不额外做事 |
| `row.svelte` 的 `RowList` | 无 APG 部件模式：一串各自可达的行，加上方向键 | ↓／↑ | 走到下一／上一行，焦点落在那一行第一个可达控件上；**两端不环绕**——一本上千行的账本从末行跳回首行，是把人移到了他看不出自己去过的地方 |
| | | Home／End | 第一／最后一行的第一个可达控件 |
| | | 落在文本框、`<select>` 或可编辑区域上的同一批键 | 不接管：那些键在那个控件里已经有意思了 |
| | | Tab | 照旧逐行走——**每一行仍是一个 Tab 站，方向键是加法不是替换** |

`segmented.svelte` 的 `aria-*`：轨道 `role="radiogroup"` ＋ `aria-label`；每格 `role="radio"`、`aria-checked` 等于「这一格就是 `held`」、`why` 在场时 `aria-disabled="true"` 并 `aria-describedby` 指向 `Tip`。**Tab 序列里的那一站由 `tabStop` 独家决定**：选中格；无选中时第一个可选格；全部被拒时第 0 格（那格的原因还得读得到）；空控件一站都没有。选择跟随焦点，所以不可选的格被 `nextStop` 跳过——落在上面就等于选中它。

`tabs.svelte` 的 `aria-*`：`role="tablist"` ＋ `aria-label`；每个 `role="tab"`、`aria-selected` 等于「这就是 `current`」、`tabindex` 只给当前那个 `0`。自动激活是 APG 对「面板内容已在本地、切换无可察延迟」的推荐读法，本客户端三个使用者都满足它。

### 7-5 开一层列表的复合部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `combobox.svelte` | APG Combobox（listbox 弹层） | ↓／↑ | 游标下移／上移一行，钳在列表两端 |
| | | Home／End | **规格要求到首行／末行；今天没有实现**（7-8 第 2 条）|
| | | Enter | 采纳游标行，关闭弹层，焦点回触发器 |
| | | Escape | 关闭弹层，清空过滤词，焦点回触发器 |
| | | Tab | **规格要求关闭弹层并让焦点正常离开；今天弹层留着**（7-8 第 2 条）|
| | | 可打印字符 | 过滤，并把游标复位到第 0 行 |
| `popover.svelte` | 多列 listbox，装在 `role="dialog"` 里 | ↓／↑ | 当前列的游标下移／上移，钳在两端 |
| | | Home／End | 当前列的首行／末行 |
| | | Tab／Shift+Tab | **换列**（环绕）并把游标复位到第 0 行——本部件在此覆盖平台的 Tab |
| | | Enter | 应用当前列的游标行，回调 `onApply` |
| | | Escape | 回调 `onClose` |

`combobox.svelte` 的 `aria-*`（规格）：文本框是 `role="combobox"`，带 `aria-expanded`、`aria-controls` 指向列表、`aria-activedescendant` 指向游标行；列表 `role="listbox"` ＋ `aria-label`；每行 `role="option"`，`aria-selected` 只标**已选中的那个值**，不标游标。今天的实现把 `aria-haspopup="listbox"` ＋ `aria-expanded` 放在触发按钮上、过滤框没有角色、游标只有底色——见 7-8 第 1 条。

`popover.svelte` 的 `aria-*`（规格）：外层 `role="dialog"` ＋ `aria-label`；每列 `<ul role="listbox">` ＋ `aria-label`；每行 `role="option"`。**`aria-selected` 在两个部件里必须说同一件事——「这是当前生效的值」**，游标一律由持焦元素的 `aria-activedescendant` 承担；今天 `popover.svelte` 用 `aria-selected` 标游标，而真正生效的那一项只有一个圆点（7-8 第 3 条）。两种触发各有一条焦点路：按钮触发时列表自己取焦（`tabindex` 只给当前列 `0`）；文本框触发时调用方经 `bind` 拿走键表，焦点留在文本框里，此时 `aria-activedescendant` 必须写在那个文本框上（7-8 第 4 条）。

### 7-6 表格

`table.svelte` 是一张数据表，**不是 APG Grid**：它不做二维方向键导航，Tab 依次走过排序按钮、勾选框与可改单元格，Enter／Space 在排序按钮上切换方向。

`aria-*`：`<caption class="sr-only">` 给表名；**`aria-sort` 只写在可排序的列上**，取 `"ascending"`／`"descending"`／`"none"`；表头勾选框 `aria-label` 取 `allLabel`，行勾选框取 `keyOf(row)`，可改单元格取 `"<列名> <keyOf(row)>"`。表头勾选框在部分选中时必须是 `indeterminate`——说「一个都没选」是一句假话（7-8 第 7、8 条）。

### 7-7 焦点还原：一条规则，三种实现

**一个把焦点拿走的部件必须把它还给打开它的那个元素**，在它关闭的那一刻。三种实现今天并存，它们的差别只在谁记住了那个元素：

| 谁还 | 部件 | 怎么还 |
|---|---|---|
| 平台 | `dialog.svelte` | `close()` 按 HTML 标准把焦点还给 `showModal()` 之前持焦点的元素，本文件因此没有一行取焦代码 |
| 部件自己 | `combobox.svelte`、`popover.svelte` | 前者记住触发按钮的 ref，`shut()` 时还；后者在 `onMount` 记下当时的 `document.activeElement`，`onCleanup` 还（`bind` 模式下焦点从未离开文本框，因此不还）|
| 外壳 | `kbd.svelte` 的 `Cheatsheet` | `app.svelte` 在打开前记 `opener`，`closeSheet` 时还——**全客户端唯一一处还原权威不在部件里**，它随 7-8 第 12 条的搬家一起消失 |

### 7-8 今天与模式不符的十二处

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
12. `kbd.svelte:38`–`60` — `Cheatsheet` 是今天唯一一个没走 `parts/dialog.svelte` 的模态：没有焦点陷阱、没有 `aria-modal`、Esc 在外壳里、还留着全客户端仅剩的层号之一（`kbd.svelte:46` 的 `z-20`，设计 4-21 记的那个例外）。原生 `<dialog>` 三家引擎都支持，所以这一条是搬家而不是取舍。

### 7-9 重开参数：判定在什么条件下失效

**当某个部件的正确无障碍行为在 Chromium、Firefox、WebKit 三家上都无法用平台能力加百行以内的自有代码达成**，才回到「`RUNTIME` 放宽到三项」，并在同一变更集里写明是哪一个部件逼出了这次例外。三条限定一个都不能省：三家都试过（不是一家不支持就算数）、百行算的是自有代码的行数、例外记进本节而不是只躺在一条提交信息里。

今天没有任何部件触发它：`<dialog>`、`::backdrop`、`@starting-style` 与 Popover API 三家都有，而三家之间确实缺的两件（CSS anchor positioning、`field-sizing: content`）都不是无障碍行为，它们各自的降级分支已经在 `tip.svelte` 与 composer 里。

### 7-10 这张表的机器读者

**`#/gallery` 的夹具断言本节的键表**，这是让规格不止有人类读者的那一步：每个收键部件在那条路由上有一份夹具，夹具按「初始焦点 ＋ 一串按键 → 焦点落点、`aria-*` 取值、回调是否发生」逐行断言 7-2 至 7-6。夹具与断言的实现属于 `client/src/views/gallery.svelte` 与 `tools/xtask/src/render/`，本节只定内容。

今天的 `xtask render` 读的是画出来的盒子与它们的名字（`xtask-SPEC.md` 8-13），**一次按键都没有进过真引擎**——在这第二个读数落地之前，本节的键表没有机器读者，这一点如实记在 §8 的「未验的」里。

## 7A 表面角色：一个面「是干什么的」只有一个家

**十一档灰阶是值的权威，角色是用途的权威，两层不重叠。** `--color-g0…g10` 与 `tools/xtask/src/color.rs:164` 的「十一档」硬断言一个字不改；本节新增的是它们之上的一层**角色**。

改前的缺陷不是缺档位，是**「什么样的面算一个抬起的控件」这个事实有两百三十六个家**——`client/src/views/` 里每一处手写的 `bg-g2` 与 `border-g3` 都是一个家，没有一个是权威，两个家不一致时谁也看不见。

### 7A-1 角色是单跳，不是值

```css
--color-raised: var(--color-g2);
```

**不许写字面量。** 一个抄在档位旁边的 `oklch()` 立刻成为那个值的第二个家，档位一动就要手工重调；一个十六进制别名更糟——`tools/xtask/src/color/tables.rs` 只保留值能解析成 `oklch()` 的声明，所以它**会被静默忽略而不是被拒绝**。单跳还让一份声明同时服务两种打光：浅色块重述每一档，指向档位的角色跟着走，不必在那里再声明一次。

### 7A-2 封闭词汇

角色名住 `tools/xtask/src/color/roles.rs` 的 `ROLES`，共 21 个：四档表面（`page` / `chrome` / `raised` / `raised-hover`）、三种非导航填充（`speech` / `track` / `disabled`）、一种标记填充（`mark`）、三档边（`edge` / `edge-panel` / `edge-input`）、一种覆在彩色实心上的墨（`on-accent`），以及城市插画自己的九档（`drawn-*`）。

**加一行是一次设计决定。** 只有当一个人能用一句不提档位的话说出它回答什么问题时，这个角色才配有名字——草稿里 `inert` 与 `resting` 相隔一档，没有读者说得出某个圆点是哪一个，它们现在是一个 `mark`。

### 7A-3 闸判四条（`cargo xtask color`）

1. 每个既非档位、非文字 token、值又不是 `oklch()` 的 `--color-*`，必须是 `ROLES` 里的名字，且是到一个已声明档位的**单跳**；
2. `ROLES` 里的每个名字，样式表里**恰好声明一次**；
3. 每个角色在 `client/src` 里**至少有一个读者**——没有读者的角色是一个有值没人读的名字，删掉而不是留着；
4. `theme.css` 之外的任何文件**不得拼出档位工具类**（`bg-g2`、`border-g3`、`fill-g9` …）。

**闸因此多一条规则而不是少一条**：十六进制别名既解析不成 `oklch()`，也不是单跳，两道都拦。

### 7A-4 图底关系：外壳上浮，不是内容下沉

参考图把代码窗格画得比页面更暗，本仓不能照做——`tools/xtask/src/color.rs:60-64` 有「g0 是页」的契约，ramp 两端被硬断言。**同一个读数从另一侧取到**：内容留在 `page`，而框住工作的东西（左栏、事实条、视图头）升到 `chrome`。屏幕上最深的一片仍然是工作，契约一个字没改。

要紧的距离是 `page` → `raised`，深色页上 100 个千分点：一个控件必须不靠边框就看得出可以按。`page` → `chrome` 只有一半，是有意的——一个宣告自己的框会跟它框住的工作抢注意力，而且它另有一条 `edge`。

## 7B ACCENT 是预算，不是装饰

**accent 只许出现在两处**：焦点环，以及选中行的 2 px 左边条（`parts/tabs.svelte` 的当前页签就是后者）。

其余一切「当前 / 已选 / 激活」，一律用表面差抬一档表达。本轮收窄的四处：`parts/segmented.svelte` 的滑块默认色从 accent 改为 `raised-hover`（`Tone` 的缺省从 `accent` 改名为 `plain`）、`rail.svelte` 的在跑计数药丸改为 `raised-hover`、焦点环用 `color-mix` 削到六成、composer 的聚焦边框从「整条 accent」改为「虚线转实线」。

规则是一句可核的话：**全屏对比度最高的元素应当是「停」**，因为那是人需要在慌乱中一次点中的东西。

## 7C 需要人同意的东西，长什么样

三样东西会停下来问人：附着到人自己登录着的浏览器、在沙箱外跑一条命令、队列里等签字的审批。它们**共用一套语言**，人学一次，之后每一次都是认出来而不是读出来。

- **前缘一条 2 px 的条，加一个字形。**
- **字形是编码，颜色只是加强。** `forced-colors` 会把每一处填充与边框颜色换成系统色，所以只用颜色说「请决定」的标记，恰好在最需要它的那些机器上什么都不说。
- 条画在**前缘**而不是左边，因为这一页也会用从右往左的语言排。

实现是 `theme.css` 的 `@utility asks`，连同 `@media (forced-colors: active)` 里把它的边换成 `CanvasText` 的那一条。读者：常驻事实条的告警格、审批卡。

## 7D 常驻事实条

窗口底部一条 28 px 的条（`--spacing-facts`），**七个事实唯一的家**：模型 · effort · 本次用量 · 全城用量 · 连接 · 门 · 沙箱。

**四个是风险读数**——花了多少钱、门开到什么程度、沙箱是哪一臂、还连不连得上。风险读数必须在人**不去找**的时候就看得见；藏在一个要打开的页面后面，它只在人已经起疑之后才起作用，而那时钱已经花了、文件已经写了。

**这同时是一次单一权威修正**：改前模型与 effort 是 composer 第二行的活控件，连接是左栏顶端的圆点，用量是两个页面上两个语义不同的数，门在设置页。谁也没法和谁比，而两个用量数长期被读成同一个数。

- **两个用量格不合并**：一个是正在跑的这一次花了多少，一个是这座城一共花了多少，是两个问题；它们来自同一个 `CostAnswer`（`by_run` 与 `total`），所以线上仍然只是一个问题。
- **条里没有控件**：一格只读不按，这才使它只要 28 px。模型与 effort 画成文字还有第二个理由——session 开始后它们应当是冻结的，而一个按不动的控件比一个词更糟。
- 左栏顶端的圆点因此**只剩一个意思**：这里有东西等你。连接不再是它变黄的第三个理由。

## 7E 左栏三态

图标（44 px）／ 图标加名字（232 px）／ **收起（0）**。姿态住 `core/prefs.ts` 的 `rail` 行，不是内存信号——一个刷新就撤销的决定，人每天早上都要重做一次。

**只有图标态在悬停时展开。** 把栏收起来的人要的是整个窗口；一条宽度为零的栏若在指针经过时弹出来，等于每次他去够行首的字都把这个决定收回一次。收起态只由那个快捷键离开。

`--breakpoint-wide` 随本节删除：`src/views/` 里最后一个 `wide:` 已改为 `@wide/page:`，`lg:` 一并改为 `@lg/page:`。**容器才是诚实的尺子**——展开的左栏吃掉 232 px 窗口宽度，按窗口断点排的三栏会被挤进只放得下两栏的地方。

## 7F 制品卡：一张卡，三个窗格

工具结果是三样可读的东西中的一样，而它们**不是对等的**：有人读过的文件是主题，占卡体；改了什么、命令打印了什么是同一次 run 的两个读数，占卡脚的一条页签带，一次一个。三张卡会让眼睛先挑一张看；一张卡带头、体、带页签的脚，说清楚谁是主题，另外两个只差一次点击。

**三个就是 `trace.ts` 已经命名的三种 deed**（`explored` / `wrote` / `ran`，由 `deedOf` 分开，也是折叠句计数用的同一个权威）。改前 `explored` 与 `wrote` 被折进同一个 `file`，谁后发生谁赢——一次 run 重写了一个模块然后读了一个头文件，卡上显示的是那个头文件。

**开关卡的控件不在卡里**：它在对话旁边，卡关着的时候也够得到。头里再放一个就是同一份状态的第二个查看处，而关着的那一种情况仍然需要外面那一个。参考图在头里放叉，是因为它的面板没有别的地方可关。

## 7G 从零到第一次派活：一条按键路径

欢迎页（`views/welcome.svelte`）是三张卡，每张卡是一个链接，离开这一页就是做了卡上说的那件事，所以没有另一个「下一步」要按；按下任何一张都记下 `welcomed`，外壳因此不会立刻把人送回这里。

| 卡 | 何时出现 | 落点 |
|---|---|---|
| provider（接上一家供应商） | 只在还没有 `main` 模型时出现，排第一并标「先做这一步」；有了主模型就消失 | `#/setup` |
| work（派活） | 一直在；没有主模型时提示改为「接上供应商后可用」 | `#/talk/hall/mayor`，先经草稿门（`PreferenceDoor.setDraft`）把一句预填的任务写进市长的输入框 |
| city（看城） | 一直在 | `#/city` |

「有没有主模型」读的是 `endpoint_view` 这一问（`QUERIES.endpoints`）的答里，已选的模型有没有一个标着 `main`。`#/gallery` 的 `welcome ·` 夹具画三态（无供应方、有供应方无主模型、有主模型），各在 390 与 1440 两个页宽，由 `cargo xtask render` 量。

一座没有主模型的城里，从零到第一次派活是下表这一条按键路径：

| 起点 | 键 | 落点 |
|---|---|---|
| `#/welcome`，无主模型 | Tab 到第一张卡，Enter | `#/setup` 的 accounts 组 |
| accounts 组的挂载表单 | 填写，Enter 提交 | 端点卡出现在列表顶部 |
| 端点卡之后 | Tab 到 `main` 的模型选择框，方向键选定 | 事实条的「模型」格显示所选模型 |
| 左栏或 `#/welcome` 的「派活」卡 | Enter | `#/talk/hall/mayor`，composer 取焦 |
| composer | Enter | 派活帧发出 |

**这条路今天有两处还不是一步：** 第三行的选择框在设置页的末尾，1440×900 的窗口里要滚动才看得到，挂上第一家供应方后它既不紧跟在卡下，也不取焦；最后一行在无主模型时仍发出派活帧，由城拒绝，composer 的发送按钮尚未换成去 `#/setup` 的链接。两处都改完时，上表每一行的落点都不需要一次滚动或一次无效的按键。

**拒绝框画城给的出路。** 同一个码覆盖几种原因（`E_CONFIG_INVALID` 既是「没选模型」也是「会话中途换了模型」），所以 `err_<code>` 的标题只说拒绝的种类，不说原因；`parts/notice.svelte` 把城写的 `recovery` 句子不折叠地放在标题下，动作与主体留在折叠里（12-5）。

## 8 验收

`bun run lint`、`bun run typecheck`、`bun run test` 三样绿，`cargo xtask npm`、`cargo xtask wire-ts`、`cargo xtask color`、`cargo xtask wording`、`cargo xtask render` 绿；`just check-client` 是这三条脚本的一条线。

在一座真城加一个说 OpenAI 形的假供应方上走得通的：连接与握手、欢迎页三张卡各自的落点（§7G）、从 composer 派活、工具调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与文件原文、run 页的统计栏与透镜、记录四透镜（ledger／archive／bin／log）、成本页、设置页 attach 与 select、`#/mcp` 三扇门加删（写进 `CONFIG.toml`）、楼页提交列表与 session 跳转、左栏展开、草稿留存。

**未验的**：`Changes`／`Hunks` 有内容时的样子、Firefox 与 Zen 的无头截图（`-screenshot` 不出图，须走 BiDi）、`Tip` 两条定位分支各自的 `#/gallery` 夹具（`xtask render` 只读 `#/gallery`，所以这两条分支在真引擎里的落点尚无机器读者）、**§7 的键表**（`xtask render` 今天只量盒子，没有一次按键进过真引擎，所以每一行键表今天的读者只有人）。

## 12 Decisions

### 12-1 删除动作的形态是 dialog 确认，不是撤销 toast

- **决策**：删 MCP（及同族删除动作）经 `parts/dialog` 确认后才发帧——取消答案写在确认之下，撤不回来的那一问把安全的答案放在手下（4-20）。不引入 6s 撤销 toast。
- **理由**：撤销 toast 只在删除可逆时成立，而删除在今天的树上不可逆。删 MCP 是整表换写 `CONFIG.toml` 的 `mcp` 数组（`crates/city/src/config_layers/write.rs` 的 `write_mcp`），被删行不留副本；配置写不入账（`RulesChanged` 未落）；技能侧没有删除动词——library 只读，`Command::PutShelved` 以 `not_built` 拒，技能安装预检、来源记录与内容寻址存储都还没有。一个会自己落下的删除把不可恢复的内容交给无人看着的计时器。
- **被击败的备选**：6s 撤销 toast，撤销窗口过后才真正落删除——窗口内不发帧，撤销即取消，账上没有「删了又还」的一对。被败因只是今日删除不可逆；被删内容一旦可恢复，该形态即为首选。
- **重开参数**：被删内容留下可恢复副本——library 入内容寻址存储、来源有记录、同哈希重装幂等，或配置写留痕可还原。参数移动后按被击败备选的形态实现：撤销窗口过后才真正落删除，删除事件于落删除时入账，先落账再生效的口径不变。

### 12-2 run 表是 `$state` 记录，不是 `SvelteMap`

- **决策**：`Belief.runs` 保持 `Record<RunId, RunBelief>` 的读法，由 `runTable` 包成 `$state`；token delta 就地写进那个 run。
- **理由**：两者给同样的逐键粒度——读 `runs[id].saying` 的 effect 只因这个 run 这个字段重跑，遍历表的读者只因 run 的增删重跑（`belief/grain.svelte.test.ts` 判定）。记录的写法让十余个按 `runs[id]`、`Object.values(runs)` 读表的视图一行不改。在 R = 1e4、一帧 50 个 delta、一个读全表的订阅者下，每帧折叠从约 470–540 µs 降到约 30–40 µs（`belief/fold_cost.test.ts`，同一仪表前后交错测），因为 delta 不再让订阅者走一遍表。
- **被击败的备选**：`SvelteMap<RunId, RunBelief>`。粒度相同，但每个读者都得改成 `get`／`values()`，且对已有键 `set` 新值时，有遍历读者就会连带推进迭代版本。
- **重开参数**：视图改由 belief 暴露的派生索引读表（不再直接下标）时，表的容器可以换，读者迁移的成本就不再存在。

### 12-3 「在干活的 run」是 belief 维护的索引，不是每个视图的过滤

- **决策**：belief 在折叠时维护 `Belief.live`（没冻结的 run，按开始时刻排），视图读它；「run 在房间或其下」只有 `within` 一个拼写。
- **理由**：七个视图各自对整张 run 表过滤、排序来回答同一个问题，每次发布都付 O(R)，而且「在干活」与「在房间下」各有四份拼写，改一处不会带动其余。在干活的 run 受并发上限约束，远少于表里的 run，所以一次折叠只按那一个 run 改写 O(L) 的索引，读者付 O(L)。
- **被击败的备选**：读时计算的 `$derived`（按表派生）。它仍在每次发布时走一遍表，只是把重复的代码收成一份，代价不变。
- **重开参数**：在干活的 run 数与表的大小同阶时（L ≈ R），维护索引不再比读时过滤便宜。

### 12-4 房间的 run 是 belief 维护的按房间索引，存 `RunId`

- **决策**：belief 按房间维护 run 的 `RunId` 列表（`Belief.rooms`），房间页、目录、城市面板与天际线经 `heldIn`／`heldWithin` 读它。
- **理由**：四个视图各自对整张 run 表过滤、排序，每次发布都付 O(R)；房间的历史含已冻结的 run，`live` 答不了。表里的 run 每次折叠都换成新对象，索引若存对象就得每次折叠改写列表；存 `RunId` 时，只有 run 进表、换房间或换开始时刻才动索引。
- **被击败的备选**：存 `RunBelief` 对象的索引。每次折叠都要在列表里替换那个 run，O(k) 的复制发生在每个 token 上。
- **重开参数**：run 表不再每次折叠换对象（就地改写）时，存对象的索引不再多付复制。

### 12-5 拒绝框的正文是城写的出路，不是按码查的原因

**决定：** `parts/notice.svelte` 把 `AxError.recovery` 画在标题下、折叠之外；`err_<code>` 的标题只说拒绝的种类。**理由：** 一个码在城里有多种原因，按码写死的标题（如「模型已冻结」）在「没选模型」时说错了原因，而把城的原话折起来，人会先去按那些与这次拒绝无关的按钮。`recovery` 是唯一知道原因的句子。**胜过的方案：** 给每个原因一个客户端文案——做不到，客户端只看得见码；按原因分码是服务端改线协议的事，那之后出路表才能按码给出「去设置」。

### 12-6 派出去的活落在哪，由 `run_started` 的地址与任务原文认出

- **决策**：页面在发出 `dispatch` 时记下房间、任务原文和当时已知的 run；随后第一个满足三条的 run——发出时不认识、`RunStarted::task` 等于原文、记录的 `addr` 是发出的房间或它下面任一层的房间（按整段比较，`shopfront` 不在 `shop` 下面）——就是这次派的活（`core/landing.ts`）。落在别的房间时，原地留一行「这次 run 去了 `<地址>`」，链到那间房（`talk/landed.svelte`），不自动跳走。
- **理由**：`runtime::run::lifecycle` 写 `run_started` 时带上房间地址（`addr`）和原样的 `task`，所以不动线协议就能认出；在楼的地址上派活时，城按规则开 `<楼>/<房间>`，人若留在楼的对话页就会对着一间空房。留一行而不是跳走，是因为人可能正要在原房间里接着写，一次不请自来的跳转会把焦点和草稿带走。
- **被击败的备选**：`run_started` 带上派活帧的 `IdemKey`，按键精确认领。它更严：两个页面同时往同一栋楼派同一句话时，今天的认法两边都会认第一个起来的 run（两个都是这个人自己派的，所以链接不会指向别人的活）。它要改线协议、`WIRE_V` 进位，归到改线协议的那一组。
- **重开参数**：同一栋楼里同一句任务的并发派活成为常态（例如一个页面批量派活），或城开始改写任务原文（去空白、加前缀），就按被击败的备选改为按 `IdemKey` 认领。

### 12-7 浏览器通知只报需要人决定的事，默认关闭

- **决策**：`core/notify.ts` 的纯函数只从 `approval_queue` 的答里挑新到的待批事项；四道闸（失焦、预热期、快照里的旧事不算新、正在看的地址不弹）全过才交给 `views/notifier.svelte` 发出。设置页「外观」组里的开关默认 `off`，打开时才向浏览器要权限，开关只存在这个浏览器（`sprawling.notify`），因为通知权限本身就是每个浏览器各自授予的。
- **理由**：通知打断的是人在别处做的事，所以只配给「没有人就停下」的那一类——run 的进度、完成与拒绝都不需要人回答，已经由标签页的标题与图标承担。预热期与快照闸挡住的是同一个错：页面刚打开或重连时，答里的每一件事对这一页都是「第一次见」，却不是新发生的。
- **被击败的备选**：对每个完成、每个拒绝也发通知，或默认打开。前者把需要回答的那一条淹在不需要回答的里面；后者让浏览器在人还没理解这一页时就弹出权限请求。
- **重开参数**：城开始产出第二类必须由人回答、却不经 `approval_queue` 的事（例如一个问句），这张表就要把它也读进来；或者通知改由城经 Web Push 发出（页面关闭时也要报），开关就该跟着偏好一起存进城。

### 12-8 对话里没有 `/goal`

- **决策**（由人选定）：页面的斜杠表不设 `/goal`。
- **理由**：目标已经有两处权威——楼的常设目标（`set_pursuit`／`pursue`，sprawling-SPEC §8-35）与每次派活的 `run_started.goal`。对话里再开一个入口，就是同一事实的第三处定义，三处之间没有东西把它们绑在一起。
- **被击败的备选**：`/goal <text>|pause|resume|clear`，把 `/goal x` 翻成 `pursue{step:{set:{goal:"x"}}}`。
- **重开参数**：常设目标与派活目标合并成一处权威时，对话入口可以指向那一处而不增加定义。

### 12-9 只看结果模式里，人取消的 run 不进任何一类

- **决策**：`outcomeOf` 把 `completion = done` 记为刚完成，`cancelled` 不列，未名的结局（`completion = null`）记为「已结束」，其余冻结（`limit` 与此后新增的词）记为失败；在跑的 run 不列。
- **理由**：前三类是人要处理的东西。取消是人自己做的，结果他已知道。`belief.adopted` 把 `RunSummary.completion` 填进冻结的 run，所以重载后结局照旧；「已结束」只剩流与答都没看到 `run_frozen` 的 run（热视图只见一段尾巴时），把它们记为失败会把完成了的 run 说成失败，所以单列一类，不声称不知道的结局。
- **重开参数（已结束）**：服务端的热视图总能看到每个 run 的 `run_frozen` 时，「已结束」永远为空，可以删去这一类。
- **一行末尾写什么**：刚完成写产出与 `RunBelief.pr`（PR 以分支为名，城里的 PR 没有编号）；等你写 `RunBelief.ask`（`approval_requested` 的 `action_desc`，下一条记录即清空，与 `RunSummary.ask` 同一规则）；失败写 completion。
- **被击败的备选**：把取消也记为失败——会把人自己的动作当成要他处理的事，挤掉真正失败的前 N 条。
- **重开参数**：取消可以由人以外的一方发起（例如预算或上级 run 撤回）时，那部分取消应当按失败列出。

### 12-10 没有问题认领的回答不报错，等着的问题照常以 `E_TIMEOUT` 超时

- **决定**：`asking.answered` 两轮匹配（在途表与迟到表）都认不出的回答直接丢弃，不给人任何提示；仍在队里的问题由超时扫描报一次 `ask_late`／`E_TIMEOUT`，出路是「重连」。
- **理由**：同一次构建的页面与城在连接期间也收到过认不出的回答，把它报成 `E_WIRE_MISMATCH` 会对一个人无法处理的事说「版本不同」。
- **代价**：页面与城真的漂移时，人看到的是超时与「重连」，而重连修不好漂移；能修好它的「重新加载页面以取得客户端」这时不出现。
- **被击败的备选**：超时扫描发现等待期间来过认不出的回答时，改报重新加载的出路——同一次构建里认不出的回答也会触发它，把一次普通的超时说成需要重新加载。
- **重开参数**：线协议让回答带上发问方的构建标识时，认不出且构建不同的回答直接报 `E_WIRE_MISMATCH`，这条取舍随之删去。

### 12-11 `src/` 里的组件一律按 runes 模式编译

- **决策**：`client/svelte.config.ts` 的 `vitePlugin.dynamicCompileOptions` 调 `runesFor(filename)`：文件在 `client/src/` 下时答 `{ runes: true }`，其余文件（`node_modules` 里的组件）答 `undefined`，由编译器照默认按组件推断。`svelte.config.test.ts` 判这条分界。
- **理由**：推断模式下，一个没用到任何 rune 的组件按旧语法编译——`export let` 是 prop、顶层 `let` 是响应式——所以一个组件删掉最后一个 `$state` 就会悄悄换一套语义，而两套语义在同一棵树里并存。按路径打开 runes 让本包的每个组件只有一种语义：旧语法在 `src/` 里是编译错误，不是另一种读法。
- **被击败的备选**：`compilerOptions.runes: true`。它也作用于 `node_modules` 里的 Svelte 组件（`svelte/types/index.d.ts` 的 `runes` 选项说明），一个仍用旧语法的依赖会编译失败；本包今天没有这样的依赖，但运行时依赖由 `RUNTIME` 管，是否引入它不该取决于它用哪套语法。维持推断（不设）则留着上面那个静默换语义的口子。
- **重开参数**：Svelte 把 runes 设为默认、不再推断时，这个函数与它的测试一起删掉。

### 12-12 视图栈是 Svelte 5，打包器是 Vite；对手是 Solid

- **决策**（换栈由人选定）：视图写成 Svelte 5 组件，由 Vite 与 `@sveltejs/vite-plugin-svelte` 打包。
- **理由**：参数是构建产物的大小与每个 token 的更新开销，因为一个流式对话客户端在每个 token 上都要更新页面。Svelte 5 与 Solid 在这两件事上是同一类设计——响应式编译进产物、没有虚拟 DOM、一次更新只重算碰过的信号——所以两者只能由读数分高下，分类分不出来。仓库里的读数都是 Svelte 一臂：整个客户端（含随包字体与 `#/gallery` 夹具）gzip 后 <!-- xtask:begin budget_reading:frontend_artifact -->599,102 B<!-- xtask:end -->（`tools/xtask/budgets.toml` 的 `frontend_artifact`，`just build-web` 之后称整个 dist 目录）；R = 1e4、一帧 50 个 delta、一个读全表的订阅者时，每帧折叠约 30–40 µs（12-2，`belief/fold_cost.test.ts`）。Solid 一臂的两个读数还没有（§3-4）。在读数出来之前，选择由两件已有的事定：`.svelte` 组件与 `core/` 的 `$state` 模块（`belief/runs.svelte.ts`）由同一个编译器处理，测试经 `scripts/runes.ts` 走同一条编译路径；4-1 的类型车道与 4-5 的 eslint 配置都按 `.svelte` 定型，换成 Solid 的 JSX 要换掉这两条车道。**组件生态不是参数**：`parts/` 的控件全部自绘（§7 判定），平台已给 `<dialog>`、Popover 与提示语义。Vite 保留的理由是产物：`@sveltejs/vite-plugin-svelte` 支持当前的 Vite 主版本，换打包器不改变一个产物字节，却要动 `crates/sprawling/build.rs` 读取的输出契约。
- **被击败的备选**：Solid（`solid-js` 与 `vite-plugin-solid`）。它输在上面两件编译与车道的事上，不是输在一个读数上。
- **重开参数**：§3-4 的两个读数量出来以后，同一仪表、同一机器上 Solid 一臂的产物不到 Svelte 一臂的一半，或每帧折叠开销不到一半，就重新论证本条。出现第一个真正需要组件库的需求时，先过 §7 的 `RUNTIME` 判定与 7-9。

### 12-13 随包的第三方许可文本由打包器从产物里认出

- **决策**：`client/scripts/notices.ts` 给 Vite 一个 plugin，在 `generateBundle` 时从每个 chunk 的 `moduleIds` 里认出 `node_modules/<包>`（带 scope 的取两段，嵌套的 `node_modules` 取最后一段），读该包目录下的 `package.json`（版本、`license`）与包目录顶层的许可文件（文件名以 `LICENSE`、`LICENCE`、`COPYING` 或 `NOTICE` 开头，不分大小写），写成产物根下的 `THIRD-PARTY-NOTICES.txt`：按包名排序，同一个包的多个模块只出现一次，字节只取决于输入。二进制嵌入整个产物，所以这份文件随二进制分发，城在 `/THIRD-PARTY-NOTICES.txt` 答它。字体的许可仍是 `fonts/OFL.txt`，本文件开头指向它。一个进了产物却没有许可文件的包让构建失败，并点名它。
- **理由**：`svelte`、`effect`、`@lezer/*` 与 Svelte 运行时带进来的包是 MIT 或 Apache-2.0，两者都要求版权与许可声明随副本分发，压缩后的 bundle 也是副本；物料清单（`xtask sbom`）只列 cargo 包。从产物认包，而不从 `package.json` 或 `bun.lock` 认：前者漏掉传递进来的运行时包，后者把 devDependencies 与 tree-shaking 删掉的模块也算进去。包目录取自模块路径本身，而不是按包名到 `client/node_modules` 下去找：打包器读的是哪一份，声明就写哪一份。缺许可文件即失败而不是跳过：悄悄少了一个包的声明，与没有声明是同一个缺口。
- **被击败的备选**：一个现成的 rollup 许可证 plugin——多一个 devDependency 做几十行就能做完的事；把 npm 包写进物料清单——清单是 cargo 的 CycloneDX，且不在二进制里。
- **重开参数**：产物里出现一个许可要求别的形式的包（例如要求在界面上署名），或这份文件让 `frontend_artifact` 的读数增长超过 8 KiB。

### 12-14 Markdown 不在浏览器里读，`RUNTIME` 不为它加一项

- **决策**：refrain 路线图 S7.9 要求二选一写进条目的那一项，选城侧：Markdown 由 `documents::markdown`（comrak）在城里读成块，页面经 `Query::Preview` 拿到块再画（4-26）；comrak 不编成 wasm，`RUNTIME` 与包体不因它多任何东西。
- **理由**：文档的版本在城里，页面只持有窗口，预览随一次往返就到，与它读 `Range` 是同一种代价；导出在 Rust 里读同一个函数，一个文法就只有一份构建。删掉 `prose.ts` 之后包体还少 1–2 KB，而 `frontend_artifact` 的余量本来就只有几 KB。
- **被击败的备选**：comrak 编成 wasm、由页面加载。它省掉每一窗的往返，经远程门的设备上这一点更明显；但 wasm 的导出要 `unsafe`，工作区里只有 `crates/desktop/ffi` 可以有自己的 lint 表，多一个 crate 就要人的定规，工具链多一个目标，包体推断多 100 KB 以上，同一个 comrak 还要在两条构建路上各编一次（`crates/documents/Spec.lean` D20）。
- **重开参数**：远程门上一次往返量出来超过 100 ms（refrain 路线图 §5 的 `client_send_feedback`）而对话流的候选都消不掉它；或允许第二个带自己 lint 表的 crate 的定规出现。
