# client-SPEC — the browser client (Svelte + Effect, outside the cargo workspace)

> 权威顺序同 AGENTS.md：人的决定 → ARCHITECTURE.md → 本文件 → 代码与测试。本文件记接口与设计；视图层（`src/**/*.svelte`、`src/theme.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。
>
> **免的是画法，不是键盘。** 一个部件遵循哪个 WAI-ARIA 模式、每个键做什么、焦点还给谁、`aria-*` 取什么值，是这一层对使用者的承诺，不是一次视觉迭代；`docs/frontend-method.md` 的豁免因此只覆盖排布、间距、色调与动效，`views/parts/` 的交互契约由 §7 独家规定。

## 1 定位与边界

- `client/` 在 cargo workspace **之外**，由 bun 驱动；产物落 `sprawling` 包里的 `crates/sprawling/web-dist/`（crates.io 的包只装包目录，sprawling-SPEC 8-83），不随 `CARGO_TARGET_DIR` 移动（`index.html` 在该目录根，其余在 `assets/`），`crates/sprawling/build.rs` 递归嵌入该目录，并以 `index.html` 与 `assets/` 的存在判「完整」。
- **两种范式不叠**：Effect 只做一件事——用生成的 `Schema` 读帧（`core/frames.ts`；`event` 与 `delta` 两种热帧先走由同一份 schema 导出的窄校验，见 4-6）。socket 阶梯、asking、belief 都是纯 TS 状态机加 `svelte/store`，视图只见 Svelte。
- 运行时依赖的名单只有一个家：`tools/xtask/src/npm.rs` 的 `RUNTIME`，本文件不抄它的条目与数目。名单上除了 `svelte` 与 `effect`，还有 `@lezer/highlight` 与各语言的 `@lezer` 语法，因为代码视图按语法上色，而高亮器与每种语法都是按需加载的分块（4-26），不进首屏；以及图标集 `@lucide/svelte`，只经 `parts/glyph.svelte` 一处出口、按图标单独导入（4-34）；以及 RefRain 的五个 `@codemirror` 包，只由 `views/refrain/` 读，是懒加载的一块（7N、12-23）。hash 路由手写，不引路由库；组件库按 §7 的判定逐个引入。`xtask npm` 门守三件事：锁文件与清单逐条同、运行时依赖恰为 `RUNTIME`、许可证在 `deny.toml` 的清单上。
- Firefox 是第一浏览器：每个屏幕先在 Firefox 里验收。
- `trustedDependencies` 留空：bun 默认不跑生命周期脚本，任何包的 postinstall 都不执行。

## 2 工具链

版本号的权威是 `client/package.json` 与 `client/bun.lock`，本表只写每个包的角色。

| 包 | 角色 |
|---|---|
| svelte | 视图（runes 编译进产物，无虚拟 DOM、无框架运行时 diff；`src/` 一律 runes 模式，见 12-11） |
| effect | Schema、Brand |
| @lezer/highlight 与各语言的 @lezer 语法 | 代码视图的语法着色，按语言懒加载（4-26） |
| @lucide/svelte | 图标；只有 `parts/glyph.svelte` 导入它，每个图标单独导入，产物只带用到的那些（4-34） |
| @codemirror/state、view、commands、search、merge | RefRain 的编辑器：文档与改动集、视图与输入法、撤销与键表、查找替换、两版之间的 diff；只由 `views/refrain/` 读，懒加载（7N、12-23） |
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

`@import "tailwindcss"` 之后 `@theme` 把默认调色板、字体、字号、字重、圆角、间距、容器宽度全部置 `initial`，再声明这套令牌（`g0…g10`、`accent`、`alert`、`accent-hover`、`alert-hover`、`accent-solid`、`text`、`text-quiet`、`text-faint`、`text-disabled`；字体 `sans`／`mono`；字号与字重 `figure/title/heading/label/body/note`；间距 `tight/snug/base/pane/wide/section`；宽度 `measure/page`；圆角 `panel/card/control/pill`；缓动 `arrive/leave` 与时长 `short/panel/page`；玻璃 `glass` 与 `--glass-opacity`；弧边指数 `--corner-exponent`；混合档的 `--blend-opacity`，见 4-43）。彩色令牌保留 `calc(<chroma> * var(--chroma))`，去色仍是一个系数置零。**全客户端只有这一个文件可以出现颜色字面量**，`xtask color` 守它。

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
- **4-8 页面上常驻的按钮只在两处：左下三键与对话框。** 左下三键（7E）是图层、信箱与设置；对话框（7I）带发送与停止合一的硬币键，和它横线下的设置行。城、楼、记录、成本、登记簿、MCP 与性能这些页没有各自的常驻入口，由设置面里的设置树到达（7L）；Ctrl-K 与 `core/keys.ts` 的 `go.*` 键是去同一批地方的快路。页面其余部分只有内容自己的控件——一行的展开、一张卡的答复。理由：常驻入口每天被扫视一次，八个页面八个字形，是让人每天读八次他一周才去几次的东西；去这些页是一次查找，查找该有分组，而分组是一棵树说得出、一列平铺的字形说不出的（12-19）。
- **4-9 两层可视化。** `#/city` 是 SVG 画的城；`#/building/<addr>` 是目录树（`Query::Listing` 逐层）＋文件原文（`Query::Document`）＋计划表＋提交列表；`#/run/<id>` 有七个透镜——time、turns、monitor、prompt、context、changes、evidence，打开时停在 time（4-42）。
- **4-10 页面上没有句子，但零数据的屏必须说下一步。** `lang.json` 全是标签，说明段不写。**动词一律拼成命令**：`city_stop` ＝ `/halt --all`、`bld_halt` ＝ `/halt {addr}`、`talk_stop`／`run_cancel` ＝ `/stop`，`release` 与 `halt` 同形；两语同一拼写。理由：人会从别的软件迁移用法，一条命令的拼写自己说明自己。**图例与空态提示不算说明段**：`city/bar.svelte` 的图例是五个字形唯一的名字，去掉它城就是一张没人读得懂的画；一个零数据的屏只画一个灰词时，人分不清这屏是空的还是坏的。因此**空态一律走 `parts/empty.svelte`**——一个形状、一句说缺什么的话、一个离开这个状态的动作，动作能省而那句话不能。这不放宽「不写说明段」：空态那句话说的是这一屏此刻没有什么，不是这一屏是干什么的。**对话页的空态是对话框本身**：空房间里对话框立在页面的竖直中线上，上方一行写收件人与房间地址，没有说明句；第一次发出后同一个元素沉到底（7I）。说明句只出现在展开的面（信箱、设置面、右侧）与欢迎页。
- **4-11 性能纪律。** 帧按动画帧合并（`socket.ts` 的 `queue` ＋ `requestAnimationFrame`）；页面隐藏时浏览器不再调用动画帧，此时改用计时器排空（`visibilityState === "hidden"` 下用 `setTimeout(drain, 0)`），否则一条审批请求要等人切回标签页才到；`visibilitychange` 转为可见时立即排空，链路若在 backoff 就取消已排的尝试、立即重试一次，因为人此刻在看，这一级剩下的秒数只是一页空白。事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` 的 `PACE_MS`），stale-while-revalidate，动画只用 `transform`／`opacity`。
- **4-12 SVG 的规则：id 只有一个家。** Svelte 的模板解析器按命名空间处理 svg 子树，所以 `<a>` 在 SVG 里仍是 SVG 元素。规则只有一条：`<defs>` 只在最外层绘图组件里写，渐变／滤镜／裁剪的 id 在那里声明一次，引用者拿 id、不自己拼第二遍。城市插画（skyline／marks）是画不是图标，导航与动作类图标一律 `parts/glyph.svelte`（4-34）。
- **4-13 composer 说出消息落点。** `core/doing.ts` 的 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`；硬币键发送面的名字与提示（7I）是 `/dispatch`／`/steer`／`/steer · after the tool call`；三者发的都是 `/dispatch` 或 `/steer <text>`，`queued` 只是名字说出的落点，不是另一种送法（真正的排队送达——等 run 冻结后再送——要改 wire，不在本客户端的语法里）。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻。落点也画在线程里：run 在调用工具时，一枚 accent 楔形钉在在跑的那条工具行上；在想或在说时，钉在正在说的话或姿态行的末尾（refrain §3-5）。钉子重复硬币键名字里已经说出的落点，所以只给眼睛（`aria-hidden`），没有自己的提示。**零 wire 变更**：信息全在 belief 里。**抖动缓冲不做**：抖动多大是一个未测量的量，为一个未测量的量先建队列是这座城禁的那条。
- **4-14 从 diff 到 session 是一次路由跳转**，不新增命令帧、不加「继续」按钮。`dispatch{addr: room, session: null}` 与在该房间对话页按 Enter 是同一个动作，加按钮就是给一个已有机制起第二个名字。
- **4-15 提交列表按页问、按页存。** `core/asking.ts` 的 `COMMITS_PAGE = 40` 与 `commitsQuery(building, before)`——问题只有一种拼写，因为 `CommitsAnswer` 回带 `building` 与 `before` 而不带 `limit`，键若拼法不一，答案永远落不回槽里。每页是一个独立的问题：只有 `before: null` 的首页会因 `checkpoint_committed`／`pr_merged` 失效重问；旧页上界是已写下的 seq，且 `lineage` 从写提交的 run 向前走，后来的接替者改不了它。
- **4-16 转写结果落进输入框，不直接发出去。** 机器听错的那一句必须能改，否则它会花掉一次 run。没有为 `transcribe` 选过模型的城不画那个按钮：一个只可能答拒绝的控件，是一个没人该遇见的控件。
- **4-17 焦点环只有一个家，所以 `outline-none` 是删掉而不是换掉。** `theme.css` 的 base 层写着 `*:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px }`，而 utilities 层优先级更高：26 处 `outline-none` 把它吃掉，键盘用户在设置页看不见自己在哪。做法是把这 26 处全部删除、不补任何替代类——`:focus-visible` 本来就只在键盘取焦时匹配（文本框被点击时也匹配，因为它确实要收键盘输入，这是对的）。否决「改写成 `focus:outline-hidden focus-visible:outline-2`」：Tailwind v4 的 `outline-hidden` 设 `--tw-outline-style: none`，而 `outline-2` 展开成 `outline-style: var(--tw-outline-style); outline-width: 2px`，两条规则在 `:focus-visible` 时同时命中，算出来是 `outline-style: none`——那对组合会删掉它本想保住的那个环。**任何一处需要压制焦点环的地方写 `outline-hidden` 而不是 `outline-none`**：v4 把「画 2px 透明描边、在强制色模式下仍可见」这层语义改名到了 `outline-hidden`，照旧写 `outline-none` 会在 Windows 高对比下丢掉焦点环。闸门条件是 `client/src` 里 `outline-none` 出现 0 次。
- **4-18 `parts/tip.svelte` 的提示有两条定位路，调用方说关系。** `title` 对三种读者都失效：指针要悬停约一秒，键盘根本到不了，触屏永远不画。`Tip` 用 `:hover` 与 `:focus-within` 显示一个 `role="tooltip"` 的兄弟节点，`transition-delay: 300ms` 挡住指针扫过一行时的连环点亮，未显示时是 `display: none`——既不画也不被 `xtask render` 量到。**关系由调用方写**：控件自己有名字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`；组件不猜，因为只有调用点知道控件有没有可见文字。**定位两条路都要在**：`@supports (anchor-name: --a)` 内用 `position-area` 锚定并 `fixed`，脱开一切裁剪祖先；Safari 与 Firefox 今天不支持，落到对 wrapper 的 `absolute` 分支（`AGAINST_WRAPPER`）。只写前一条是静默失效：不支持 anchor 的引擎里弹层退回静态位置，可能溢出视口。锚点名按实例生成，经继承的自定义属性 `--tip-anchor` 传给提示节点，所以同一行上的两个提示各锚各的控件。
- **4-19 `Field` 管有标签的表单格，不管搜索框与命令框。** 23 处裸 `<input class="rounded-control bg-g2 …">` 里，供应方表单、模型表的两个上限格、键名值对与登录码改走 `Field`，因而一次拿到标签、说明、错误态、`aria-describedby` 与 `:user-invalid`（失焦后才红，不是每次按键）。命令面板、combobox 过滤框、composer、楼页目标框与记录搜索框**不改**：它们要 `ref`、`autofocus`、逐键的 `onKeyDown` 与自定义补全，塞进 `Field` 会把它变成十个透传参数的 `<input>` 壳子，正是 AGENTS.md 禁的那种壳。`Figure`（原 `providers.svelte`）删除，三个调优数字改 `Field kind="number" step=…`，键盘上下键因此能用；**不给 `ms` 后缀**——标签本身就是 Codex 的 `timeout_ms` 与 `stream_idle_timeout_ms`，再加一个后缀就是同一个单位的第二个家。
- **4-20 模态是平台的事。** `parts/dialog.svelte` 是原生 `<dialog>` 加 `showModal()`：top layer、焦点陷阱、Esc、其余页面 `inert`，四件都由引擎承担，文件里因此没有遮罩层、没有 keydown、没有取焦调用，也没有 z-index——top layer 之上排不进任何数字。取焦由文档顺序决定：平台取对话框里第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。Esc 到达时先 `preventDefault` 再回调 `onCancel`，否则默认行为绕过调用方关掉元素，`open` 还说着「开着」。开与合是同一条 `transition` 的两个读法，`display` 与 `overlay` 是离散属性，少了 `transition-discrete` 关门第一帧盒子就消失、连同它的淡出。**遮罩是 `::backdrop` 上的 `backdrop-filter: brightness()`，不是一层 `bg-g0/80`**：`::backdrop` 只在实现了「从原始元素继承」的引擎里读得到页面的颜色令牌，令牌解析不出时 `background-color` 回到初始值——一扇没有遮罩的模态，且没有任何一道闸会报出来；亮度滤镜不向平台要颜色，深浅两套灯光下都变暗，Tailwind 又把 `--tw-backdrop-*` 直接声明在 `::backdrop` 上，所以这条路不经过继承。
- **4-21 `views/parts/` 不写 z-index。** 定位过的盒子在未定位的盒子之后绘制：下拉、弹层、粘性表头与提示压住它们打开时盖住的那些行，这是绘制顺序本来就给的，不需要一个数字。给了数字反而要维护一张谁比谁大的表，而那张表没有家。唯一还留着数字的部件是 `parts/kbd.svelte` 的快捷键表：它必须盖住左下三键，而三键的 `<nav>`（`views/edge.svelte`）是全客户端允许保留的那一个 `z-10`，所以只有 top layer 能让它交出数字（见 4-23 的第二个参数）。
- **4-22 按钮的三态是一个 `data-state`。** `idle`／`loading`／`stopped` 由一处判定给出，属性本身、`aria-disabled`、`aria-busy` 与点击闸是它的四个读者。色调表只写静息态与静息态的 hover，`not-data-[state=idle]` 一条规则管住其余两态——「正在等回执」与「你按不了」对手的回答是同一句，不该有两套写法；hover 写进 `data-[state=idle]:` 里，因而一个不能按的控件在指针下不会亮起来假装这一按会落地。`background-color` 进了 transition 列表，hover 因此是到达而不是切换。
- **4-23 Popover API 今天进不了 `parts/popover.svelte`，两个参数卡着它。** 其一：`popover` 元素开着时在 top layer，包含块是视口，`absolute` 不再相对 composer 的 `<form>` 解算，而把弹层钉回 composer 的唯一机制是 CSS anchor positioning，本项目的第一浏览器（Firefox）没有——所以「不支持 anchor 时走今天的 `absolute` 分支」这条降级对一个 popover 不成立，它只在元素还留在流里时成立。其二：`#/gallery` 的两个弹层夹具靠一个 `relative` 祖先与两层 padding 把面板留在自己的 Case 里，元素一进 top layer 就逸出，`xtask render` 的「没有盒子画在容纳它的盒子之外」立刻红。两个参数任一移动都应重新论证本条；在那之前弹层是 `absolute`，Esc 与取回焦点由这个文件自己管。
- **4-24 后端已经答了的，客户端必须问，问到了必须画。** 一个城答得出而没人问的 `Query`，与一个上了 wire 而没有屏幕读的字段，是同一个缺陷的两半：它们让「这件事做完了」在两侧各有一个说法。三处落点：**其一**，城页顶栏的六个数字全部出自 `Query::Metrics` 一问——事件、在干活的 run、已收尾的 run、等人批的、排队的信号、回收站里的。`MetricsAnswer` 的第七个字段 `buildings` 故意不画：顶栏下面的天际线与旁边的楼列表本身就是楼的数目，再写一个数字就是同一个事实的第二个家。原先的状态徽标一并去掉——它说的 `runs_active` 就是那六个里的一个，而「城停了」由外壳的横幅在每一页说一次（`app.svelte`），顶栏只留那一个停/放的控件。**其二**，`Query::RegistryView` 得到 `views/registry.svelte` 一屏，四列（登记于、类别、所属楼、是什么），走 `parts/table.svelte` 因而每列可排序，默认最新在上。**其三**，run 页直接问 `Query::RunView` 而不再只靠流折出来的 belief：从别人发来的链接打开 `#/run/<id>` 的那一页没见过任何记录，`city_view` 又只列它还在列的 run，所以这条 run 属于哪个房间、是否已经结束，恰好在最需要的那一类 run 上是空的。**两个读法按 seq 判定**：流说的是此刻，摘要说的是城写下来的，两边都带账本位置，所以谁更新是一次比较而不是一次偏好。
- **4-25 base URL 的形状只拼写一次。** `setup/providers.svelte` 的 `BASE_URL` 同时喂两个读者：框子自己的 `pattern`（浏览器在人还在填表时判，失焦后才红）与 `hostOf`（「看看」和「接上」两个控件的开关）。**`type="url"` 不是那条规则**——它收 `mailto:somebody` 和任何别的 scheme，于是一个这张表单会拒的值可以坐在一个浏览器称为合法的框里，而人是按下一个始终发灰、不说为什么的控件才知道的。形状是：scheme、ASCII 主机、可选端口、可选路径，之后什么都不许有；查询串拒掉，因为供应方陈述的是 API 的根，每个 face 自己在后面挂路径。**客户端只做最宽的判断，永不比城更严。** 归一化今天有调用者了：`accounting::worker::credentials::Entered::resolved` 是打字地址变成被调用地址的唯一一处，probe 与 attach 都经过它。`gateway::normalise` 的第三条规则把缺席的 scheme 读成 `https://`，运行这座城的机器地址读成 `http://`，所以 scheme 在这张表单里是可选的——一个要求写 scheme 的框会拒掉厂商文档印出来的 `api.openai.com/v1` 与 `127.0.0.1:11434`，而城收得下。这张表单因此只判「有没有一个能调用的主机」，scheme 由城补，路径由城按预设表补。

- **4-26 「这次调用算哪一类」由工具的注册回答，客户端不留表。** `kernel::ToolMeta` 为每个注册工具声明了 `effect`（`Read`／`Write`／`Egress`／`Connector`／`Spawn`／`Govern`／`Spend`）与 `render`（`Generic`／`Terminal`／`Diff { locations }`），两者随 `Call` 上 wire（`crates/wire/Spec.lean` §8-47）。对话页的折叠摘要按它们数「探索／写入／运行／其他」，检视面按它们选读法（4-45）；客户端没有一张以工具名为键的表，一个新注册的工具不需要页面改一行就落进正确的一类，城认不出的工具两个字段都缺席，读作「其他」。`parts/code.svelte` 的行号从 1 起算：检视面的文件视图读的是 `Query::Document` 给的文件开头，所以第 27 行就是文件的第 27 行；一段输出的头部（`Output.head`）不带它在文件里从第几行开始，画输出时的行号仍只是这段输出的行号。 **同一个代码视图的颜色按语法给出，高亮器不进首屏。** `parts/paint.ts` 是唯一知道语法存在的文件：它用 `@lezer/highlight` 与十种 `@lezer` 语法（cpp、css、go、java、javascript 及其 ts／jsx／tsx 变体、json、python、rust、yaml）把文本切成 `parts/code.ts` 的五种 `Ink`；它本身是懒加载块，每种语法又各是一块，所以只看 Rust 的页面只下载 Rust 的语法，一段代码都不显示的页面一块都不下载。文件扩展名与 Markdown 围栏的语言词查同一张表（`rs` 与 `rust` 不可能指向两种语法），表里没有的名字整段画成 plain；`toml`、`sh` 今天落在这里，因为 `@lezer` 没有这两种语法，而 `@codemirror/legacy-modes` 要连带 `@codemirror/language` 与编辑器状态进来。`parts/inked.svelte` 是文件视图与 Markdown 代码块共用的画法：文字第一帧就以 plain 画出，答案到了才上色，晚到的旧答案丢弃；块取不到时仍是 plain，因为文字不上色也值得读。选 lezer 不选 shiki（JavaScript 正则引擎）的参数：同样十种语言、1,000 行 TypeScript，在一台满载的 16 线程机器上 lezer 解析加上色约 70 ms，shiki 约 1,240 ms 且首次建立要 600 ms；十种语法压缩后两者都约 166 KB，而 lezer 按语言分块、给的是真正的语法树。1,000 行 ≤ 16 ms 与各块的 gzip 体积由测量那一轮读出并写进 budgets；当 lezer 在空载机器上仍超过 16 ms，或出现同体积下更快的语法高亮器时，重开这条选择。

  **Markdown 只有一个文法，它在城里（12-14）。** 文档预览问 `Query::Preview { version, viewport }`，答复是 `documents::Preview` 的块树（`crates/wire/Spec.lean` §8-74）：页面把每一种块与行内画成元素，不经 `innerHTML`；`Unsupported` 画成等宽的原文（HTML、公式、前置元数据、过深的嵌套），`Preview::Unsupported` 说这一版不按 Markdown 读、留在源码视图，零个块是空文档的空态（4-10），三者是三种画法。代码块的 `info` 与文件扩展名查同一张语言表，经 `parts/inked.svelte` 上色。链接与图片的去处城已判过（`crates/documents/Spec.lean` D23），页面不再判协议，相对地址相对文档所在的目录。块的 `span` 是版本里的字节，与源码视图的位置对应经 `core/document_pos.ts` 换算。一个版本的预览不会过时，`staleness.ts` 把 `preview` 与 `range` 放在同一行。从 `Document` 打开的每一份 Markdown 文件都在内容库里，不论长短都能按版本预览（accounting-SPEC §8-23）。**对话流问 `Query::Reply { text, state }`**（`crates/wire/Spec.lean` §8-75，12-15）：页面把它手里的回复文字送去，还在说的标 `streaming`，`model_returned` 到了之后标 `settled` 再问一次；答复是同一棵块树 `documents::Laid`，结算的回复与同一段字节的预览读出的树相同。还在说时答复的 `span.end` 是收束点，之后的文字页面照旧画原文（末尾几个字淡出），之前的画成块；收束点只进不退（`crates/documents/Spec.lean` D31），所以已经画出的块不会被收回。一段回复同时只有一问在途，答复到了而文字已经长了才问下一次，所以问的次数不超过增量到达的次数，也不在没有新字时空转；城的答复只说读到哪里，什么时候再问是页面的事。答复读满一个窗口（`span.end` 之后还有已闭合的块）时，页面把 `span.end` 之后的文字再送一次，块的区间加上那一段的起点；区间是字节，与预览一样经 `core/document_pos.ts` 换算。答 `Unavailable` 的文字（含 NUL）画成原文。页面接上这个入口时删去 `core/prose.ts`、`prose.test.ts` 与 `closedUpTo`，`views/prose.svelte` 改画 `Laid`，`talk/saying.svelte` 与 `talk/thread.svelte` 改问 `Reply`。`staleness.ts` 把 `reply` 与 `preview` 放在同一行。`RUNTIME` 不为 Markdown 加任何一项。

- **4-27 右侧由账本决定要不要画，由网格决定画在哪。** `talk.svelte` 问的 `Query::Rounds` 与 `Thread` 问的是同一句，`core/asking.ts` 按内容合并，因此「现在显示的是哪个 run」只有一个答案；右侧不占路由，理由是一条新路由会让这个答案有第二处判定，而两处判定第一次分歧就发生在有人打开别人发来的链接时。**右侧是网格上的一块，不是对话列旁边挤出来的第二栏**：打开时它占第 7–12 栏（检阅档第 8–12 栏），上、右、下三边越过页边距贴到窗口边，对话列移到第 2–6 栏（4-33、7F）；混合档的世界层随之收起，因为对话两侧已放不下整块面板，而把面板压到对话的字底下违反三层的规则（7H）。**开合由两件事决定，各有一个家**：人打开的项（工具行、时间轴行、文件树里的文件）住在 `views/inspect/open.svelte.ts`，有一项就开着，不管偏好怎么说；没有项时由 `core/prefs.ts` 的 `panel` 决定要不要跟着眼前的 run 开（4-45），它是人的偏好，归宿是 `~/.sprawling/config.toml`，由那扇门后面的浏览器缓存记住，城的回答到了由 `adopt` 顶掉（4-29）。Accel-J 与检视面的收起键关掉两者：清空打开的项，偏好记为收起。

- **4-28 强度只有一个权威，客户端不留副本。** 城层 `CONFIG.toml` 的 `[model] effort` 经配置梯子冻结进每一次 run（`crates/city/src/config_layers.rs`），所以浏览器里不再存 `sprawling.effort` 这一行。**缺席不是 `"medium"`，缺席是不说**：帧里不写这个字段，城的文件回答；文件也没写时供应方回答，而 `Effort::None` 是「尽量不要想」，是另一件事。一次派活仍可为它开的那一场单独说一个档位（`Dispatch.effort`，城把它写进房间层，`crates/city/Spec.lean` §8-14），所以选择器只在 composer 上——那里的作用域与控件的位置一致。**设置页与欢迎页因此不再有强度选择器**：它们承诺的是「从此以后」，而没有写城层强度的命令帧，一个刷新就忘的设置控件是第二个权威的开始。六句 `effort_note_*` 随选择器搬到 composer 的那一列，做每一格的 `hint`，人在决定的那一刻读到它。
- **4-29 偏好有两层，城赢。** 权威是人层 `~/.sprawling/config.toml`，浏览器存储是它前面的缓存：缓存只负责首帧不闪，城的回答一到就整条顶掉它。`core/prefs.ts` 独家拥有每一个存储键的拼写（`ROWS`）与读写，`core/rows.ts` 独家拥有对 `localStorage` 的触碰；`keys.ts` 问 `chord(action)`／`setChord`，外观屏问 `held().appearance`／`setAppearance`，网络屏问 `held().proxying`，没有第二处拼一个键名。`core/prefs_city.ts` 的 `keepWithCity(door, conn)` 是这扇门与连接唯一的接点，`main.ts` 开页时调它一次。

  **一进一出两个方向，形状因此不同。** 出城的方向是具名改动：`setLang`／`setWelcomed`／`setPanel`／`setAppearance`／`setProxying`／`setChord` 各发一条 `Command::PutPreferences { patch }`，与 `wire::PreferencePatch` 的变体一一对应；链路不在 `live` 时改动只留在这个浏览器，重连后城的回答为准。`setTier`、`setNotifying`、`setShowing` 不出城，因为城的记录没有这三个字段；`tier` 等 `PreferencePatch` 有了它那一臂再入城（7H）。进城的方向是 `adopt(stated, chords)` 一整条：一次回答陈述每一个值，按字段贴回去会贴出半新半旧的一条；回答缺席的字段由浏览器此刻的值补上（`prefs_city.ts` 的 `adopted`），回答带来的快捷键覆写这个浏览器里同一动作的行。`keeper()` 说此刻是哪一层在保管（`"browser"`／`"city"`），第一次 `adopt` 之后答 `"city"`；设置页把它画出来（`setup/kept.svelte`），因为「清掉浏览器数据会不会丢」是人有权知道的事。

  **`readPreferences` 与 `writePreferences` 互为逆，这才使缓存是缓存**：城上次答的就是下一次首帧画的。
- **4-30 设置页的 `config.toml` 侧栏引用文件，不自己拼。** 原先这一栏用 `[model_providers.<name>]` 拼出一段文字，而城自己的读法只认 `[model]`／`[sandbox]`／`[[mcp]]` 三节——把它抄进 `CONFIG.toml` 的人会拿到一句列出三节的拒绝，一个事实在前端与后端各有一个家且已经分歧。这一版把那段字符串删掉，侧栏只说「这一页不逐行列出 config.toml」（`setup_toml_unread`），因为 `Query::Config` 已由上下文梯级那一处控件在问，说「读不到」是假话。填进去的是 `Query::Config { addr }` 的答案。**它逐值带层**：`wire::ConfigAnswer` 是 `{ addr, effort: Option<SettledEffort>, tuning: TuningDefaults }`，`SettledEffort` 带 `from: ConfigLayer`，因而侧栏每一行画的是「值 ＋ 它来自哪一层」，`parts/badge.svelte` 画那一层的名字。**今天画不出来**：`Query::Config` 与 `ConfigAnswer` 都在（`client/src/wire.ts` 已带 `ConfigAnswer`），而 `setup.svelte` 那一栏尚未问它，所以侧栏仍是 `setup_toml_unread` 的空态；接上之后这一栏是唯一的填入点，不需要先退休任何一处手拼。
- **4-31 设置页有 MCP 组，skills 组是「列表 ＋ 只读源文」。** MCP 组仍是一个指向 `#/mcp` 的链接，但它搬回了设置页：设置页顶层六项里，MCP 与「账户与供应方」是同一类事（接什么进来），分成两处等于让设置页说两套分类。**反转的理由是项目数，不是「到不了」**——「外壳的导航列已经到得了」曾经是删掉它的理由，而人每天扫视那一列的成本当时没有计入；MCP 页本身、它的三扇门与删除动作一个字不动，只是入口从顶层挪进这一组。skills 组由 `setup/skills.svelte` 承担三件：放技能的文件夹、楼列（`shared/buildings.svelte`，与 `#/mcp` 同一份）、那栋楼**三个书架**的清单（`Query::Skills`，`SkillShelf` 三臂：城库／楼架／外部架）与打开一条后的原文（`Query::Document`，走楼页那一个 `FileView`；外部架没有城内地址，那一行因此只报名不打开）。**这里没有编辑器，因为城的那扇门答「未建」**：library 在保留前缀下，居民可读不可放（`crates/city/src/library.rs`），`Command::PutShelved` 已在 wire 上而 `crates/accounting/src/worker/commanding/routing.rs` 以 `not_built` 拒它，一个存不下去的 `<textarea>` 会把「改了」说成两件事（写面未建）。

  **那两级文件夹路径今天由页面拼写，这是记下的欠账。** `Shelves` 画的是 `${城名}/.sprawling/library/`，而这条路径的家是 `kernel::layout`（`RESERVED_PREFIX` 与 `LIBRARY_DIR`）——客户端与城各拼一次，城改了前缀或目录名，这一行会静默指错地方。退休它需要一个跨 wire 的字段：`SkillsAnswer`（或它的邻居）带一个由 `CityLayout::library()` 相对城根算出的 `Address`，页面改读它。今天没有任何回答携带布局，所以这一处保留并在此记录。
- **4-32 一个状态药丸只有 `parts/badge.svelte` 一个画法。** `building/plan.svelte` 原先手画五种漆色（`done` 灰、`blocked` 实心 alert、`in_progress` 实心 accent、ready 的 `bg-g3`、其余无底色），那是同一件事的第二个家，且那串嵌套三元没有 `awaiting_approval` 的臂——等人批的一行被画成没人开工的一行。现在一个穷尽 `RoadmapStatus` 的 `weightOf(row)` 给出 `quiet`／`live`／`alert` 三档，`Badge` 画它。**两个状态共用一档是对的**：`ready` 与 `in_progress` 都是城在动，`blocked` 与 `awaiting_approval` 都是城停下来等人，而分辨它们的是词，不是颜色（7-1 的 badge 行）。代价是 done 不再比 not_started 更暗；这不是损失，因为那两个词本来就不同，而颜色按 7-1 只许重复词。

- **4-33 版式：一张 12 栏网格，对齐由构造保证；宽度按内容种类封顶。**
  **其一，外壳是一张 CSS 网格**：12 栏，页边距 32 px，栏距 24 px，竖向以 8 px 为基线，行高与间距都取它的倍数。**每个区域按栏线摆放**：对话列、世界层的面板、右侧与左下三键都写成 `grid-column` 的起止，世界层的面板用 `subgrid` 落在同一组栏上，所以两块面板的左缘相同不是量出来的，是同一条栏线。栏是外壳容器的栏，不是窗口的栏。`mx-auto` 不出现在任何页面容器上，只允许在阅读列内部（对话线程、长文档）。各档的摆法：

  | 档 | 世界层 | 对话 | 右侧打开时 |
  |---|---|---|---|
  | 专注 | 不渲染 | 第 4–9 栏 | 对话第 2–6 栏，右侧第 7–12 栏 |
  | 混合 | 会话第 1–3 栏，提交第 10–12 栏 | 第 4–9 栏 | 世界层收起，其余同专注 |
  | 检阅 | 会话第 1–3 栏，所选会话第 4–8 栏，提交第 9–12 栏 | 第 4–8 栏底部一条，高 112 px | 会话第 1–2 栏，所选会话与对话第 3–7 栏，提交收起，右侧第 8–12 栏 |

  左下三键在第 1 栏贴底，三档都在；会话栏的底部空出它们的高度。**窄于 768 px 的窗口只有一栏**：页边距收到 16 px，对话占满宽，三键在对话框之下横排一行，右侧是盖住整页的一张面。**当前状态**：窄窗口里世界层不画，三档都按专注排；世界层做成从左翻入的全屏面、返回在来源一侧的顶部，是还没做的一步。**其余页面**（城、楼、记录……）在各自重新设计之前占第 2–12 栏，左缘是第 2 栏的栏线。

  **其二，宽度分三档按内容封顶，不按页面封顶**：`measure`（520）给段落、`talk`（760）给对话、`page`（1040）给带表格的表单，表格与代码块不封顶、随容器长到 `wide`（1120）。同一页的不同区块各取各的档，因为整页取最窄的那一档会把表单与表格挤进段落的宽度。**其三，一个网格列的最小宽度是 320 px**：`grid-cols-[repeat(auto-fit,minmax(320px,1fr))]`，不用断点，因为断点问的是容器而列宽问的是内容。表格的列另行规定：文本列 `min-w-[12ch]`、数字列 `w-figure`、id 列 `min-w-[24ch]`，超出容器就横向滚动——**永不逐字折行**。
- **4-34 度量令牌：控件高度、图标网格、圆角、阴影三级、触达面。** 此前没有控件高度这条令牌，于是每个视图自己拼 `py-tight`／`py-snug`，同一行里三个按钮高 26、28、30 px。令牌是 `--spacing-control-sm|control|control-lg`（28／32／36）、`--spacing-glyph-sm|glyph`（16／20，图标画在哪个方格里就住哪个方格）、圆角 `control 6｜card 8｜panel 12`、阴影三级（`shadow-raise` 贴着页面的控件、`shadow-float` 弹层、`shadow-sheet` 抽屉与 dialog）。**有影的面不画边，有边的面不画影**，弹层与玻璃面例外：左下三键是玻璃（`backdrop-filter` 的模糊与饱和，加 `glass` 角色按 `--glass-opacity` 的底色，4-43），玻璃面有一条 1 px 的边与一级浮影，因为它浮在任何内容之上，边把它与背后的字分开，影说它在哪一层。外壳另有四个尺寸：左下三键 40×40、圆角 14，以 `corner-shape: superellipse(var(--corner-exponent))` 渐进增强（`@supports` 之外退回同半径的 `border-radius`，4-43），图标画在 18 px 的方格里；硬币键 32×32，字形 18 px；上下文环的盒子 40×40、半径 18、线宽 1 px，检查点 5 px；对话框的横线 1 px。触达面：桌面 ≥ 28×28、触屏 ≥ 44×44，小于这个的图标按钮用 `::before` 扩热区。图标收进 `parts/glyph.svelte` 一个 `Glyph name=…`：名字住 `parts/glyph.ts`，画法是 lucide 图标集的那一个（§7 判定），线宽在任何尺寸下都按屏幕像素算——各画各的 `<svg>` 是同一套图形的几个家；城市插画不是图标，留在原处。
- **4-35 通知是三个座位、一个组件。** 一切拒绝与提示都由 `parts/notice.svelte` 画，座位由调用方给：**inline**（有归属表单的拒绝，紧贴出错的字段，随字段编辑清除）、**toast**（无归属页面的拒绝，以及页面对一次落空按键的回答（12-16）；立在对话框之上、对话列的宽度之内，右侧打开时随对话列左移，至多三条，拒绝是 `role="alert"`，按键的回答是 `role="status"`，8 秒自动收起、悬停暂停；按键的回答不进抽屉）、**drawer**（信箱键点开，从左缘弹出、贴在第 1 栏右侧的全高抽屉，宽 440，Esc 与点外部关闭）。**今天 AxError 的三段式在三处各手写一遍**（`views/notices.svelte`、`views/refusal.svelte`、`parts/notice.svelte`），本条把它收成一处。抽屉按天分组，每条是标题（`lang.json` 的 `err_<code>`，如 `err_E_CONFIG_INVALID`；页面自己等不到回答的问题是例外，`E_TIMEOUT` 的 subject 读得出一个 `Query` 时标题取 `ask_late_title`，写出那个问题的线上名字，因为一页同时问好几个问题，同一句「等得太久」看两遍的人分不出城漏答的是哪一个；规则在 `parts/notice_title.ts`）、时间、同 `code+subject` 的计数徽标，英文原句折叠进等宽详情。**动作由 `core/recovering.ts` 的一张表从 `code` 映射到动词**，toast 与抽屉都读它——两个读者各写一张表就是同一个事实的两个家。动作有四种臂：命令（按自身拼写）、`reconnect`（让链路再试）、`settings`（去设置页选模型）、`reload`（`location.reload()`，取这座城构建时的客户端）。`E_WIRE_MISMATCH` 只给 `reload`：两端对线上格式意见不一，重连只会再撞上同一处分歧；草稿按地点存在 `localStorage`（`prefs.ts` 的 `draft`），重新载入后仍在。**动作只作用于 composer 所在的房间**（`views/notice_recovery.ts`）：`/new` 与 `/fork` 的房间取自地址栏——对话页的地址，或 run 页那个 run 的房间——从不取自拒绝的 subject，因为地址语法接受一句带空格和反引号的话，把 subject 当地址读会在一句错误原文上开出一栋楼；`/stop` 只在 subject 是 run id 时出现，停的也只是那个 run（run id 的语法窄到装不下一句话）；subject 只指房间的拒绝不给 `/stop`（`recoveryFor(error)` 按 subject 判），因为停房间是 `/halt`，一个永远跑不起来的控件不是动作；没有 composer 的页面上动作置灰（`act_no_target`）。**toast 在对话框之上**：人刚按下发送或停止，眼睛就在那里；右下角会压在右侧的编辑器上，左下是三键。**链路丢失不是通知，是页面所处的状况**：`views/link_banner.svelte` 用 `parts/banner.svelte` 画在「城已暂停」的同一位置（主区之上，两者同时成立时断线在上），写出第几次重连与 `unsent` 里等着的条数，唯一的动作是「现在重试」（取消阶梯的等待、立即重连）；横幅从 `backoff` 出现，到 `live` 或 `refused` 才撤，其间每次 `opening` 不闪掉。
- **4-35a 恢复动作可以打开一张预填表单，由人提交。** `core/recovering.ts` 的 `Recovery` 在 4-35 的四种臂之外还有 `form`——`{ kind: "form", label, words, room }`：`label` 是按钮上的 `lang.json` 键，`words` 是预填正文的键（槽 `{building}` 与 `{name}`），`room` 是 `"mayor"` 或 `"building"`，指表单开在哪个房间。按下它把填好的正文写进那个房间的草稿门（`PreferenceDoor.setDraft`，与欢迎页 `assign work`、`/fork` 回填同一扇门），再把地址栏移到那个房间；发出去的仍是人按下发送的那一次 `dispatch`。**不进审批队列，也不绕开门**：表单只替人写好字，city D1 一字不改。**`form` 臂只服务 `subject` 读作 `<楼地址>: <缺失的名字>` 的拒绝码**——地址文法不含 `:`，所以第一个冒号就是分界，冒号后去空白非空才算读到；读不出这两样时按钮置灰（`act_no_target`），不猜。逐码核对的结果：只有 `E_PLAN_MISSING` 的 `subject` 全程是这个形状（`<楼地址>: <常设目标>`，由设常设目标的那一处唯一抛出），它的行是 `{ kind: "form", label: "act_ask_plan", words: "form_ask_plan", room: "mayor" }`：按钮「让市长写计划」把「为 {building} 写一份计划，让它的就绪步骤朝向：{name}」填进市长的草稿。`E_CREDENTIAL_MISSING` 的 subject 依出处是 URL、provider 名、`secret:` 引用、MCP server 标签或一句话，都不带楼地址，所以它没有 `form` 行。
- **4-36 设置页是左锚定的两栏，`config.toml` 折进正文底部；每个设置项是一张卡。** 左边是设置树（7L），当前条目 `aria-current` 加左侧 2 px accent 条（7B）；正文 `flex-1 min-w-0` 左对齐，每组一个 `<h1>` 与一行说明（4-10 的例外：这行说的是这一组此刻管什么，不是这一屏是干什么）。`config.toml` 从常驻第三栏改为正文底部的可折叠区块，默认收起——**它是校对工具而不是设置项**，常驻占 300 px 是正文被挤到 360 px 的直接原因。**卡片语法**：标题（label 600）＋一句说明（note faint）＋控件＋卡脚（左：一句约束或状态；右：需要提交的才有按钮），立即生效的控件没有按钮，改动后卡脚出现「已保存 ✓」。今天同一屏里 `title`／`heading`／`note` 三级标题叠在 80 px 内的写法随之取消：**一屏一个 `title`，其下只用 `label`**。


**宽度按内容种类分档，不按页面封顶。** `setup/groups.ts` 的 `WIDTH` 是这张表的唯一权威：`accounts`、`tools`、`skills` 是 `page`（表与卡片网格），其余七组是 `measure`（段落与分段控件）。**账户组有两个例外，两个都是内容种类给的**：供应商表单占 `talk`（760）而不是 `measure`，因为那里贴的是 base URL 与密钥、常常六十多字符，520 会把它们截断；`等价的 config.toml` 在 `@wide/page:`（≥1120）下挪到表单右侧的 `w-tree` 列，因为校对材料该在被校对的东西旁边，窄容器下它回到正文之后。**一个名字隔阱记在 `theme.css` 里**：`max-w-wide` 取的是间距档 `--spacing-wide`（24 px）而不是 `--container-wide`（1120），页面上任何 `max-w-wide` 都会把整列压成 24 px；能安全指名的只有 `page`、`measure`、`talk`。
- **4-37 界面用一个等宽面：西文默认 Geist Mono，汉字落到设备自带的字体，字栈不指名任何 CJK 面，也不随包发一个。** 整个界面——正文、标签、数字与代码——默认是 Geist Mono 一个面，层级只靠字号、字重与灰阶；外观组可以把正文换成别的面（`core/appearance.ts` 的 `FACES`）。 汉字落到这台设备自己有的面上，因为那正是引擎对一个指名面都没有的字形会做的事，而平台自己的选择是唯一按本项目能接受的条件拿得到的一个。**两条条件各自单独就足以定下来**：许可上，这条字栈只能指名客户端可以再分发的面（`fonts/OFL.txt`、`docs/third-party.md` 第 4 节），而 Windows 与 macOS 上人真正有的中文面是它们厂商的；尺寸上，一个值得指名的面按厂商原样是 17,773,244 B、过 `gzip -9` 是 11,266,972 B，是 `tools/xtask/budgets.toml` 给整个前端产物那一档（`frontend_artifact`）的数倍。**代价很小**：回退面的基线与 x-height 与随包面不同，而这只在一行里同时出现拉丁字与汉字时看得出来；指名一个 CJK 面并不能取消这件事——它只会让结果取决于那台机器恰好装了哪些字体。**中文的尺寸与行高照旧另计**（`theme.css` 的 `:root:lang(zh)`）：注释步不再减 1 px、行高 1.6、字距归零——那三条说的是同一个字号下汉字比拉丁字密得多，与用哪个面无关。
- **4-39 「用我的编辑器打开」只列厂商自己的文档或源码读得懂 `文件:行` 链接的编辑器，编辑器与城的文件夹由这个浏览器保管。** 猜来的协议会静默失败——浏览器去找一个没人装的程序，或者把文件开在第一行——所以每一项都要有出处。VS Code 的 <https://code.visualstudio.com/docs/configure/command-line>（“Opening VS Code with URLs”一节）写明 `vscode://file/{full path to file}:line:column`，并写明 Insiders 版的前缀是 `vscode-insiders://`。这个处理器在 VS Code 源码 `src/vs/code/electron-main/app.ts` 的 `getWindowOpenableFromProtocolUrl` 里，按 URL 的 authority 是 `file` 来认，协议名是构建在 `product.json` 的 `urlProtocol` 里注册的那个；所以继承它的构建用同一形状、换自己的协议名：VSCodium 的 `prepare_vscode.sh` 把 `urlProtocol` 设为 `vscodium`；Cursor 的工作人员在论坛帖 <https://forum.cursor.com/t/remote-uri-opens-a-new-window-every-time/166093> 里称本地的 `cursor://file/...` 链接照常工作；Windsurf 没有公开这一处的文档，它注册 `windsurf` 协议，LocatorJS 等工具按同一形状生成 `windsurf://file/...:行:列`；这是七项里出处最弱的一项，一旦证明它不按这个形状打开就删掉。Zed 的文档（<https://zed.dev/docs/reference/cli>）只写了命令行的 `文件:行`，读 `zed://file` 的是源码 `crates/zed/src/zed/open_listener.rs`：去掉 `zed://file` 前缀、解码后按 `路径:行:列` 打开。**Zed 在 Windows 上要换一种写法**：去掉前缀后剩下 `/C:/...`，Windows 拒收这个名字（os error 123）；`//?/C:/...` 是同一路径的设备形式，Zed 能打开，其中 `?` 写成 `%3F`，免得浏览器把它读成查询串。这一条在 Windows 上的 Zed Preview 1.22 实测过。JetBrains 系不列：`idea://open?file=` 这类协议只在 macOS 上注册，Toolbox 的 `jetbrains://<工具>/navigate/reference` 要项目名而不是路径；Sublime Text 没有自带协议。哪天这些编辑器自己支持「文件:行」链接，在 `EDITORS` 加一项、在这里补上出处。选择控件是原生下拉列表而不是分段控件：七个选项放不进设置卡片里一条等宽的轨道。**保管在浏览器而不是城的 `config.toml`**：装了哪个编辑器、城在那台机器的哪个文件夹，是浏览器所在机器的事实，同一座城从另一台机器打开时这两个值不同；而线协议不带任何机器上的绝对路径（`wire.ts` 里 `Address` 之外不传路径）。代价是人要在设置里填一次城的文件夹。重开参数：线协议给页面城在浏览器所在机器上的根路径时，`cityFolder` 改读它并删掉这一行存储。**历史版本不冒充工作树**：编辑器打开的永远是工作树此刻的文件，所以一个读自过去版本的行号交给它，会落在另一行上而不报错。检视面的 diff 画的是两次检查点之间的那段，它的行号属于后一次检查点；页面先问 `Changes { base: 后一次检查点, head: null }`（`head` 缺席即工作树），这个文件在答里出现，就说明工作树已经不是检查点里的那份，`reachOf` 收到 `past`，画可复制的 `路径:行` 而不画链接；不在答里时收到 `current`。文件视图读的是 `Query::Document` 给的工作树此刻文本，永远是 `current`。没有可用链接时（没选编辑器、远程浏览器、文件夹没填）也画可复制的 `路径:行`，不声称已打开。**只有城里的路径得到链接，而这条判断是字面的**：路径按生成的 `Address` 语法判，文件夹须是绝对路径、不是 UNC 共享（`//host/share` 的主机不是文件夹，当成文件夹会把链接指到另一个文件）、不含 `.` 与 `..`；磁盘上指向城外的符号链接不跟随，因为浏览器看不到磁盘，城也不启动任何东西。
- **4-40 重连按水位续传，差距大才整页重问。** `socket.ts` 记下本页折过的最大 `seq`（全城水位）。welcome 的 `resume_from` 是服务端账本头的 `seq`：水位已知、头在水位之后、差距不超过 `RESUME_PAGES × GAP_PAGE`（两页，400 条）时，缺口 `水位+1..头` 排进 `gaps`，走 `HistoryRange` 逐页补拉，补回的每条记录经 `asking.invalidate` 只失效它影响的答案，`asking.resumed()` 只重发断线时在途的问题；水位未知、`resume_from` 缺席或差距超过两页时回退到快照，即 `asking.reconnected()` 把每个答案标旧重问。阈值两页（`RESUME_PAGES`）是估计，不是读数：两页以内补拉的字节估计少于把一页上所有被看着的问题重问一遍，超过两页时一次快照估计比逐页补拉更快到达当前状态。能定下它的读数是同一座城上一次快照重问的字节数与一页 `HistoryRange` 的字节数之比（§3-4）。welcome 的 `epoch`（创世记录的链哈希）与本页上次见到的不同时，水位属于另一份账本：`belief.forget()` 丢弃全部折叠、水位与缺口清空，再按快照重问，因为旧水位在新账本里指向的是别的记录。

- **4-41 斜杠动词是一张表，每条自带分组。** `core/slash.ts` 的 `SLASH` 是页面动词的唯一权威，`/` 菜单与 Ctrl-K palette 都读它；`Slash` 记录有五个字段：`spelling`、`grammar`、`about`、`section`（`actions`／`navigation`／`sessions`，类型 `Section`，类型检查强制每条填写）、`run`。palette 按 `section` 分组，不另立以拼写为键的表——另一张表会让改名的动词静默落进 actions。glossary 的 Halt 与 Cancel 是两个动词，页面随之分开拼写：`/stop` 无参，只对眼前的 run 发 `cancel`，眼前的 run 由 `core/in_front.ts` 一处判定（12-16）；Accel-.（`run.stop`）是同一个动作的键，眼前没有 run 时不发任何帧，toast 座位出一条提示，说这一页没有在跑的 run，并写出停整座城的拼写 `/halt --all`；palette 里同一情形 `/stop` 与 `/steer` 置灰，理由是 `no_run_in_front`；`/halt [addr|--all]` 与 `/release [addr|--all]` 成对，带地址的作用于那栋楼，`--all` 与无参作用于整座城（`halt`／`release` 帧）；第一个词既不是 `--all` 也不是合法地址时不发任何帧、行留在输入框里待改，因为把打错的地址放大成整座城是这个人不可能想要的读法。Ctrl-K palette 同理：动词既没发出帧、没换页、也没写回一行时 palette 不关，行留着待改；只有做了事或清空了行才关。notice 上的 `/stop` 恢复同样只对 refusal 所指的 run 发 `cancel`，只指房间的 refusal 不给这个恢复（4-35），不升级为 `halt`。`/clear` 等于 `/new`：丢掉对话、在此地址开一个什么都不带的新 session，与各家 harness 的 `/clear` 同义，不再只清空输入框。`/steer <text>` 的语法里没有 `--queued`。`/diff` 打开眼前 run（没有时取此房间最新的 run）的 run 页，changes 视图是那一页的一个 lens。
- **4-42 run 页是时间透镜加顶部统计栏，不展开账本树。** 统计栏（`run/head.svelte`）是一行带标签的事实：结局、用时、token 入/出（缓存另记）、花费（回合数另记）、来处（楼 / 房间）；每个数都由页面已经问到的 `RoundsAnswer` 求和，所以栏与下面的透镜说不出两个数。时间透镜（`run/river.svelte`，默认页签）按份额画三条并行轨道——模型在说、工具在跑、在等人——下面是这次 run 的全部工具调用。**工具调用带自己的时钟**：`Call.called`／`Call.answered` 是账本写下调用与结果的时刻（`crates/wire/Spec.lean` §8-47），所以调过工具的回合在量到的地方切开——`Turn.t` 到第一次调用归模型，到最后一个结果归工具，其后归模型，调用列表每行带它量到的用时；**等人从量到的那一刻开始**：`Note::waiting.t` 是账本写下批准请求的时刻，`Note::waiting.answered` 是答复记下的时刻，等过人的回合在这两处切开——请求之前照调过工具的回合切法，请求到答复归人，答复之后按那之后发出的调用再切；答复时刻为 `null`（窗口外或读不回来）时请求之后整段归人，而不是按没人量过的长度再切一刀。**页面持有的盒子有上界**：`run/lanes.ts` 的 `columnsOf` 每列二分取样一次再按份额合并，一条轨道最多一列一个盒子；调用列表经 `windowOf` 只画视口内的行加两侧各 `OVERSCAN`（8）行，一万次调用的 run 与四十次的 run 在页面里是同一个量级。**透镜读的尺寸不回头喂自己**：轨道宽与调用列表高由同一个 `ResizeObserver` 报出，在下一帧才写进状态；行高是 `--spacing-control-sm` 这条令牌，未画行的占位也按这条令牌计，所以列表的总高只随调用数变，画出哪几行都不改它。备选是量画出的行再除以行数：那个高度又决定画哪几行，浏览器在同一轮报告里看到尺寸再变，就报 `ResizeObserver loop completed with undelivered notifications`，`#/gallery` 在 `xtask render` 里因此没法量。备选是逐段画、逐行画：一万个节点在 390 px 宽的轨道上大多窄于一个像素，却要付全部的布局与内存。统计栏的「模型」一格读 `Turn.model`（`crates/wire/Spec.lean` §8-47）：最后一个回合问的模型，run 中途换过模型时按出现顺序列出各个名字。统计栏的「由谁派来」一格读 `Opening.dispatched_by`（`crates/wire/Spec.lean` §8-48）：`person`、`city` 或派活居民的地址，原样以等宽字画出；为 `null`（旧账本、窗口外）时不画这一格。
- **4-44 线程的读数：每个数读自账本的两个时刻，缺一个就不画；在跑的数读页面的时钟，并从事件时刻重算。** 线程照模板排：消息头、正文、其下一条一条工具行（refrain §3-2、§3-3、§3-4；`talk/turn.svelte`）。
  **消息头**（`talk/head.svelte`）是一行注释字：谁 · 模型 · 时刻 · TTFT · 到达节奏。模型是会话的事实，只写在这一段的第一个消息头上（`Thread` 的 `opens`），之后只在某个回合换了模型时再写一次（`Turn.model`）。强度与模式同是冻结事实，但线上没有每个 run 的这两个值（`run_started` 的 `policy` 不进任何回答），所以今天不画；线上有了，消息头只加一个读者。时刻来自 `Turn.t`，写到秒，毫秒留给工具行。花费不画：专注与混合两档由人定不画花费（7D），检阅档的对话栏不画线程。只调工具、不说话的回合没有消息头，它的工具行接在上一段话下面。**TTFT** ＝ `Turn.first_at − Turn.t`，只在 `Turn.timing` 为 `measured` 且差不为负时画（`talk/timing.ts` 的 `ttftOf`）。**t/s 今天不画**：它是输出 token 数除以首字到回复返回的秒数，而 `Turn` 只带 `t` 与 `first_at`，没有返回的时刻；用相邻记录的时刻代替是猜。`Turn` 带上返回时刻时，`timing.ts` 加一个函数，消息头加一个读者。**到达节奏**（`talk/sparkline.svelte`，40 px）只画本页亲眼看着流进来的回复：正文每次变长记一个样本（页面时钟，已到字数），一帧合并的几个增量只算一个样本，所以它说的是节奏，不是逐 token 时序；按 run 收集，回合在回答里出现后按 `Turn.opened` 归档（`talk/arrivals.svelte.ts`，页面寿命内最多 200 条，最旧的先丢），没认出是哪个回合就丢掉；切成 20 段，每段画它占最忙那段的比例（`talk/rhythm.ts`）。从历史打开的回复没有节奏。
  **工具行**（`talk/call_line.svelte`）：`种类 主体 用时 结果`，网格四列（7ch、余下、9ch、自适应），整行是一个按钮。**种类读注册，不读工具名**（`talk/call_kind.ts`）：先读 `Call.render`（`terminal` → exec，`diff` → edit），再读 `Call.effect`（read、write、net、mcp、spawn、govern、spend、browser），两者都缺（工作台不认识的工具、键出现之前写下的行）时写工具名本身。**用时**：落账后是 `answered − called` 的毫秒，不到一秒写 `31 ms`，以上写 `3.412 s`；`Call.timing` 为 `unmeasured` 或两个时刻倒序时不画。在跑时由一个计时器重画：每个时钟一个（`ticker(now)`），每 100 ms 一次，没有在跑的行订阅或页面隐藏时停下，恢复可见时立即从时钟重算——读数始终是 `now − called`，不是累加的次数；不满一秒只画在跑的点，满一秒画十分之一秒（Nielsen 的 1 s）。完成的 ISO 时刻（在跑时是开始的）在这一行的提示里（7D）。过 10 s 的行只在知道时写它在等什么：run 在等人时写「等你回答」，其余不编原因。失败写「失败」，用 alert。按下经 `openCall` 在右侧打开这次调用（`inspect/open.svelte.ts`）；右侧正显示的那一行 `aria-pressed` 并带 2 px accent 前缘条（7B）。键见 7-11。
  **阅读位置**（`talk/scroller.svelte`）：人在末尾时跟随，离开末尾或线程里有选区时不动；离开末尾之后新到的块（带 `data-wear` 的每个回合与结局线）计数，计数是一个回到末尾的按钮，回到末尾即恢复跟随。字体载入、代码上色、Markdown 闭合改变的是已有文字的高度：不跟随时由引擎的滚动锚定保住眼前那段，跟随时视口本来就在末尾。**读痕滚动条**（`talk/wear.svelte`）是指针的滚动条：轨道上按 `runs/phase.ts` 的阶段色标出每块的位置（等人、调工具、只说话、结局），视口显示过的段落加深一档，拇指是当前视口；按下跳到那里，拖动跟随。滚轮、触屏与键盘仍由对话列本身承担，所以它对读屏器隐藏；窄窗口不画它，留系统的滚动条。测量合并到下一帧，一阵增量只读一次布局。
  **送达**（`talk/delivery.ts`、`talk/delivered.svelte`）：刚发出的话在发出的同一帧画在线程底部（带 `data-local-feedback`，≤ 100 ms），并写出它的状态。草稿是框里的字，执行中是 run 自己的线程，两者都不归这里；之间三态加一态：**held**（按下时链路不在，话在本页的队列里，`core/unsent.ts`）、**pending**（帧已发出，城此后什么也没写）、**accepted**（城在发出之后写了记录；线上的回执不带帧的身份，所以同 `talk/handing.ts` 一样按「发出之后有新记录」读）、**unknown**（等待时链路断了）。unknown 不重发：城若已接下，再发一次就是第二个 run；重连后城告诉页面的记录把它推到 accepted。发出之后到来的拒绝结束回显，拒绝画在回复本该在的地方。第一次发送就让空房间的对话框沉底，因为回显的字已经在那里。
  **播报**：流式正文不进实时区域，逐 token 念出来只是噪音；结局线与送达状态是 `role="status"`，各在事件发生时说一次。

- **4-43 动效、玻璃与弧边各是 `theme.css` 里的几个令牌，`theme.css` 之外不写曲线与时长。**
  **缓动两条，按值抄自 Open Props（open-props 1.7.23 的 `src/props.easing.css`，MIT），不装包（12-20）**：`--ease-arrive` 是它的 `--ease-out-4`，`cubic-bezier(0, 0, .1, 1)`；引擎认得 `linear()` 时（`@supports (transition-timing-function: linear(0, 1))`）换成它的 `--ease-spring-1`，一条只过冲 1.7% 的弹簧，原样抄录；`--ease-leave` 是它的 `--ease-in-4`，`cubic-bezier(.9, 0, 1, 1)`。进入与到达减速，离开加速（Q1）：一个来回都动的过渡，基态写 `ease-leave`、到达的那一态写 `ease-arrive`，因为 CSS 取目标状态上的计时函数。Tailwind 自带的 `ease-in`／`ease-out`／`ease-in-out` 由 `@theme` 里的 `--ease-*: initial` 清掉，没有写曲线的过渡取 `--default-transition-timing-function`，它指向 arrive。
  **时长三档，按位移分**（P6）：`short` 150 ms 给原地换状态（悬停、按下、一行淡入）；`panel` 250 ms 给一块面在自己的位置出现或翻面（提示、弹层、模态、硬币键、对话框的横线）；`page` 350 ms 给穿过页面的移动（世界层进出、对话框沉底、上下文环的弧）。Tailwind 的时长命名空间是 `--transition-duration-*`，所以令牌拼作 `--transition-duration-short|panel|page`，类名是 `duration-short|panel|page`；`--default-transition-duration` 指向 `short`。城市插画的环境动画（人影上下、窗与念头闪烁、旗子飘）不是位移，不取这三档，它们的周期只写在 `theme.css`。
  **`data-motion="off"` 把三档与默认时长都置零并停掉动画**，写在根元素上管整页，写在一棵子树上管那棵子树（`#/gallery` 的夹具这样用）；机器的 `prefers-reduced-motion: reduce` 在 `data-motion` 没说 `on` 时同样置零。机器面：`xtask motion`（`tools/xtask/Spec.lean` §8-51）拒绝 `theme.css` 之外的 `cubic-bezier(`、`linear(`、`steps(`，以及 Tailwind 的 `duration-<数字>`、`duration-[…]`、`ease-[…]`，所以一条过渡的曲线与时长只有令牌这一个家。
  **玻璃是一个角色、一个不透明度、一个工具类**：`--color-glass` 单跳到 `g2`（7A），`--glass-opacity` 是它的不透明度（75%），工具类 `glass` 把两者混成底色，加 `backdrop-filter: blur(24px) saturate(170%)`、1 px 边与一级浮影（4-34）。**这个不透明度就是 `xtask color` 判的那个数**：玻璃按它盖在页面能画的最亮表面（`raised-hover`）上时，`--color-text` 仍要够到它自称的 APCA 层级，深浅两种打光各判一次；把数调低到够不着时门红，所以它同时是下限。玻璃只给边缘层的小面——左下三键，信箱与设置面若取玻璃也用这一个工具类；对话面板用 alpha、世界层不上玻璃（P12）。四种情形下玻璃退成实色、信息不丢：外观组的 `glass: off`、`prefers-reduced-transparency: reduce`（只有 Chromium 报它，所以要有前一个开关）、引擎不认 `backdrop-filter`（三者都是 `--color-glass` 满不透明、无模糊），以及 `forced-colors: active`（`Canvas` 底、`CanvasText` 边）。
  **弧边一个指数**：`--corner-exponent` 是 CSS `superellipse()` 的参数，取模板三键的 1.6。`@supports (corner-shape: superellipse(2))` 之内，每个 `rounded-*` 工具类写 `corner-shape: superellipse(var(--corner-exponent))`，之外是同半径的 `border-radius`。SVG 一侧（`city/shape.ts` 的 `cornerPower`）读同一个令牌，换成 Lamé 曲线的指数 2^1.6 ≈ 3.03 交给 `squircle`；读不到时取 2，即普通圆弧，与不认 `corner-shape` 的引擎画的一样。原先四档圆角各带一个指数（2、1.5、1.5、1），城市画又写 4，是同一个形状的五个家。
  **混合档的不透明度**：`--blend-opacity` 是混合档里世界层的不透明度（7H），默认 60%，只住 `theme.css`；外观组的一根滑条在 30%–90% 之间、以 5 为一步改它（U2），人没动过时根元素上没有这个属性，滑条显示页面此刻画的值。滑条是原生的 `<input type="range">`，交互即 APG Slider：←／→ 与 ↑／↓ 走一步，Home／End 到两端，`aria-valuetext` 写出百分数；每一次落定的值立即生效，卡脚出现「已保存」（4-36）。
  **强制色的退路**：玻璃见上；对话框的横线换成 `CanvasText` 的渐变；上下文环的两个检查点画成 `CanvasText` 实心，先后由它们在环上的位置说，因为强制色下绿与红都会被换掉。
- **4-45 检视面按调用：一次调用一点就在右侧打开它的完整结果，读法由调用自己的注册决定。** 右侧的状态只有一个家，`views/inspect/open.svelte.ts`：打开的项是一组页签，按打开的先后排；最后碰过的那一项在前，编辑器区与终端区各显示本区最后碰过的那一项，所以打开一条命令不会把它上面的文件盖掉。每个打开的项一直挂载到关掉为止，滚动位置、选区与 RefRain 的草稿因此在别的页签到前面时不丢（roadmap §3-14）；代价是一个页签一个视图，所以至多 `KEPT`（8）项，多开一项就关掉最久没碰的那一项。**没有人打开任何项时，右侧跟着眼前的 run**（`reading.ts` 的 `followingIn`）：编辑器区是它最后读过或改过的文件，终端区是它最后跑的命令，还在跑的命令也跟；选其中一个页签就把两项都收下，成为人自己持有的项。**读法**（`reading.ts` 的 `readingOf`，`views/inspect/called.svelte` 按它分派）：`render = terminal` 是终端；`render = diff` 是 diff；其余的调用按它在账本里留下的形状读——结果是一张存在内容库里的图（`{ image, width, height, media_type }`）就是截图，`effect = read` 且参数恰好是 `{ path, offset?, limit? }` 就是读文件（搜索也是只读，但它带 `pattern`，多出的键让解码不成立）——都不是的是 `printed`，在终端区画工具、主体与它答的话。**diff**（`inspect/diff.svelte`）问这次调用前后两个检查点之间这个文件的 `Hunks`（`views/checkpoints.ts` 的 `bracketOf`：调用之前最新的检查点，没有时是 run 开始的树；调用之后第一个检查点），所以 diff 覆盖调用所在的那一波，编辑器上方一行写出两个提交；之后还没有检查点时说「下一个检查点后」而不猜。行号栏照 7-2 把「路径:行」接进对话的草稿，被选的行带 2 px accent 左边条；画法是 `inspect/patch.svelte`，`changes.svelte` 打开的那一行也用它，一个 diff 只有一种画法。**终端**（`inspect/terminal.svelte`）的头两行是命令（exec 的命令行与 monitor 读的是同一个解析，`monitor/trace.ts` 的 `commandOf`），以及毫秒用时、退出码、完成时刻（`HH:MM:SS.mmmZ`，`datetime` 是完整的 ISO）与被裁的行数；在跑时读 `core/live_output.ts` 的尾巴，调用一有结局就只读账本的结果（`terminal.ts` 的 `printedOf`，`terminal.test.ts` 判）。`Output.pinned` 有定位时头的右端有「原文」，经 `Query::Content { locator }` 读，不以 `cut > 0` 为条件，因为 sieve 可以不裁一行而缩短内容；没有定位而有裁剪时头里写「没有留下原文」，不把头部称为全文。**文件**（`inspect/file.svelte`）经 `Query::Document` 读工作树此刻的文本，交给 `parts/code.svelte` 只读画出，滚到调用读的那一行（`offset + 1`）并标出；那一行在城发来的开头之外时说出来，不标在别的行上。**截图**（`inspect/shot.svelte`）：wire 上没有把内容库里的图片字节交给页面的回答，所以画一个按原图比例的框、写出尺寸、格式与定位，并说这一页还画不出它——空白会读成一张什么都没有的图。**到编辑器的路**按 4-39 的 `reachOf`：diff 的行属于检查点，工作树已离开它时只给可复制的「路径:行」。键见 7-11 的检视面几行；Esc 关上检视面，焦点回到打开它的那个控件（7-7）。
- **4-46 RefRain 编辑的是一个版本：读整份、改在浏览器、存成那一版的字节区间。** 右侧的文档项画 `views/refrain/refrain.svelte`（7N），它问 `Query::Document { at }`，`Coverage::Head` 的版本再按版本问 `Query::Range` 逐窗接齐（`core/document_windows.ts`）；接齐之前编辑器只读，因为对半份文本的改动会把另一半说成删掉了。**超过 `EDITABLE_BYTES_MAX`（4 MiB）的版本只给第一窗、只读，并说出这是第一窗、全文多大**：附录 D 要求超能力时明确降级、不静默截断保存，而一窗一个往返，4 MiB 是六十四个往返，再大的文件不是人在这一栏里逐字改的稿子；重开参数是 A11 在固定语料上量出的首个可读视口与输入至绘制时间。
  **坐标只在 `core/document_pos.ts` 换算。** 编辑器（CodeMirror 6，12-23）的位置是 UTF-16 码元，而且它把 `\r\n`、`\r`、`\n` 都读成一个换行；线上的位置是版本里的字节（`crates/documents/Spec.lean` D2）。所以编辑器拿到的是去掉开头字节顺序标记、换行折成 `\n` 的文本，保存时 `textEdits` 把改动集（基线到此刻，`ChangeSet` 的 `iterChanges`）换成基线那一版的字节区间，插入的换行写成这一版第一个换行的写法。**被否决的**：让编辑器只按 `\n` 分行、把 `\r` 留在行里——`End` 键会停在 `\r` 之后，在那里打字就把字写进 `\r` 与 `\n` 之间；把标记交给编辑器——全选删除会删掉标记，城按 D11 拒绝这次保存（换了编码）。两处折叠因此都在换算里还原，没碰过的字节一个不动（A1）。
  **草稿是相对基线的改动，不是全文。** `core/document_save.ts` 把 `{ version, changes }` 写进 `prefs.ts` 的草稿门（地点 `document:<地址>`），每次改动后 300 ms 写一次：改动集与文件多大无关，全文却会在一份几 MB 的文件上撞到浏览器存储的配额。重开时草稿的版本就是城此刻的版本，改动照原样恢复；不是时进冲突态，草稿留着。**保存**是 `PutRange { doc, baseline, edits, idem }`；回执只认带着这个 `idem` 的 `document_written` 行，页面在保存途中问最新一页账本（`RECEIPT_QUERY`），因为账本行不经视图的门，而 `history` 本来就随每一条记录重问（`staleness.ts`）。拒绝不带 `idem`（§8-2），所以保存途中动作是 `save a document` 的拒绝归这次保存；同一页同时在存两份文档、其中一份被拒时，另一份会误读成被拒，草稿仍在，回执晚到仍把它改成已保存，这是记下的代价。**冲突**不自动解决：页面给出对照（草稿对城此刻的版本）、移到现版（把基线到草稿的改动经 `@codemirror/merge` 的 `diff` 求出的「基线到现版」改动集映射过去，结果仍是草稿、不自动保存，人看过对照再存）与丢弃草稿三个动作；附录 E 说的「重读与三方比较」就是前两个。
  **版本只列本页拿得到全文的那几版**：打开的、自己存下的（回执给出版本，文本是发出时编辑器里的文本）、城换掉的（`Document` 答出新版本时）。账本里的 `document_written` 只带摘要不带文本，只有 Markdown 版本与超过一窗的版本在内容库里（§8-69、accounting-SPEC §12 第 33 条），所以「这份文件的全部历史版本」今天没有一个读得出全文的入口；重开参数是一个按文档列版本、按版本读全文的查询。
- **4-47 世界层跟随工作区，工作台只读页面已经在问的回答。** 三栏要的每一个数都出自一个已有的问题：会话栏读 `belief.rooms`，所选会话读它那个 run 的 `Query::Rounds`（与对话问的是同一句，`asking` 按内容合并）、端点表、`Query::Config` 的第二级提醒与 `CostAnswer`，地方栏读 `commitsQuery` 的第一页、`city_view` 与逐层的 `Query::Listing`；所以打开检阅档不让城多答一种问题，一个数在工作台与它的老家（环、成本页、楼页）之间也说不出两个值。**提交的范围由房间定**（`views/world/chosen.svelte.ts` 的 `commitsIn`）：市长的房间是城自己的地方，旁边是全城的提交与城的天际线；楼里的房间旁边是这栋楼的提交与文件——所选会话一栏与地方栏都问这一句，检查点的时刻与父提交因此和提交栏读同一页。**摆放是栏线而不是类名**：`workspace.svelte` 的 `layoutOf(tier, right, bench)` 答每块区域站在哪两条栏线之间（`Lines`），由内联的 `grid-column` 写出；Tailwind 只认源码里写死的类名，而人拖出来的宽度是运行时的数，写成类名就得把全部组合预先抄一遍。窄于 768 px 的窗口不写栏线，区域按各自的 `narrow:` 类占满一栏。

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
| `belief/live.ts` | 7 投影 | `Belief.live: readonly RunBelief[]`——没冻结的 run，按 `started` 从旧到新，是「哪些 run 在干活」的唯一权威；`livened(live, run) -> readonly RunBelief[]` 在每次折叠里按这一个 run 改写它，O(L)（L 为在干活的 run 数），`liveOf(runs)` 只在 `adoptCity` 整表换写时 O(R) 重建；`within(run, room) -> boolean` 与它读的 `inside(addr, room) -> boolean` 是「在这个房间或其下」的唯一拼写；`newestWorking(belief, room) -> RunBelief | undefined` 从 `live` 取这个房间最新的在干活的 run，是对话页的 steer 与停止键、`in_front.ts` 在对话页的答案与房间页「正在进行」共读的唯一答案，O(L)。视图读 `$belief.live` 而不再各自 `Object.values(runs).filter(…)`：R = 1e4 时一次记录折叠加一次读「在干活的 run」≤ 20 µs（`belief/live.test.ts`） |
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
| `prefs.ts` | 6 数据 | `Preferences { lang, welcomed, panel, tier, appearance, proxying, notifying, showing }`、`Tier = "zen" \| "blend" \| "panorama"`、`TIERS`（图层键循环的顺序；存储里没有或读不懂时是 `blend`，7H）、`PROXYING_RULES`、`NOTIFYINGS`、`Keeper = "browser" \| "city"`、`PreferenceDoor { held, keeper, adopt, tell, setLang, setWelcomed, setPanel, setTier, setAppearance, setProxying, setNotifying, setShowing, chord(action), setChord, draft(at), setDraft, editor() -> { editor, folder }, setEditor, workbench, setWorkbench }`、`loadPreferences(rows, browserLang)`、`preferences()`；**全客户端每一个存储键的拼写都只在这个文件的 `ROWS` 里**（草稿键 `sprawling.draft.<房间或 run>`、快捷键 `sprawling.key.<action>`、档 `sprawling.tier`、工作台 `sprawling.workbench`）。`workbench` 是 `Readable<Workbench>`，`setWorkbench` 只写这个浏览器、不发 `PutPreferences`：栏序与栏宽是这块屏幕的事实，不是这个人的（12-24） |
| `workbench.ts` | 1 判定 | `Pane = "sessions" \| "session" \| "commits"`、`Column { pane, span }`、`Workbench`（三栏，从左到右）、`WORKBENCH`（默认：会话 3、所选会话 5、提交 4 栏）、`NARROWEST = 2`、`Divider = 0 \| 1`（第一栏与第二栏之间、第二栏与第三栏之间）、`resized(bench, divider, span)`：分隔线前一栏取 `span` 栏宽，钳在 `NARROWEST` 与 `widest(bench, divider)`（这一对栏的总宽减 `NARROWEST`）之间，后一栏取余数，所以三栏之和恒为 12、每栏恒不窄于两栏；`moved(bench, pane, side)`：与左邻或右邻换位，宽度随栏走，在边上的栏不动；`readWorkbench(raw)` 与 `spelledWorkbench(bench)` 互为逆（`sessions:3 session:5 commits:4`），读不懂的一行——缺栏、重栏、宽度不是整数、窄于 `NARROWEST`、总和不是 12——读作 `WORKBENCH`，不修补。检阅档工作台的栏序与栏宽怎样才算一个值的唯一判定（7K、12-24） |
| `appearance.ts` | 2 值 | `Appearance { lighting, sans, mono, sansStack, monoStack, body, density, chroma, motion, glass, blend }` 与各项的词表（`LIGHTINGS`、`FACES`、`DENSITIES`、`CHROMAS`、`MOTIONS`、`GLASSES`，按选择器画出的顺序）、`STACK_SHAPE`、`BLEND_PERCENT`（混合档滑条的下限、上限与步长，4-43）与 `blendOf(raw)`（一行存储读成一个在域内的百分数，读不出或越界答 `null`）：一条外观记录怎样才算合法；`prefs.ts` 负责存取，`prefs_city.ts` 负责带到城。`glass` 与 `blend` 只在这个浏览器里：线上的 `Appearance` 没有这两格，`setAppearance` 照旧只把其余七格告诉城 |
| `sizing.ts` | 1 判定 | `BODY_PX`、`sizingOf(text) -> Sizing`（`cleared` \| `sized { px }` \| `refused`）：人写进字号框的一串字读成什么；偏好的读取与外观组共用这一处。 |
| `editor.ts` | 1 判定 | `Editor = "none" \| "vscode" \| "vscode-insiders" \| "vscodium" \| "cursor" \| "windsurf" \| "zed"`、`EDITORS`、`editorLink(Opening { editor, folder, path, line }) -> string \| null`：「在我的编辑器里打开 文件:行」的唯一拼法，各编辑器的链接前缀由同文件的 `fileUrl` 给出（4-39），监视器的改动块拿它当 `href`，由浏览器把链接交给浏览器所在机器上注册了该协议的编辑器，服务端不启动任何程序。`path` 用生成的 `Address` 判（城内相对路径，无 `..`、无盘符、无反斜杠），`folder` 须是绝对路径且无 `.`／`..` 段，`line` 须是正整数；任何一条不成立答 `null`，页面不画这个链接。编辑器与城在浏览器所在机器上的文件夹由 `prefs.ts` 的 `editor()` 与 `setEditor` 保管。`reachOf(Opening, Shown) -> Reach` 是检视面问的那一句：`Shown = "current" \| "past"` 说行号读自工作树此刻的文本，还是读自工作树已不再持有的一个版本；`Reach` 是 `{ kind: "link", href }` 或 `{ kind: "copy", text }`。`past` 一律答 `copy`，`current` 在 `editorLink` 答得出时答 `link`，答不出（没选编辑器、没填文件夹、路径不在城里）时也答 `copy`；`text` 是 `路径:行`，所以人总有一个能拿走的定位，而页面从不声称已在编辑器里打开（4-39） |
| `document_pos.ts` | 1 判定 | `EditorChange { from, to, insert }`、`Place { line, column }`、`Positions { version, encoding, editor, lineBreak, bytes(at), editorAt(byte), place(at) }`、`positionsOf(version, encoding, text) -> Positions`、`textEdits(positions, changes) -> TextEdit[]`：一个版本的三种坐标——版本里的字节、解码出的字符（`place` 的列按码点数）、编辑器里的 UTF-16 码元——只在这里换算（refrain 路线图 §4-8），每个 `Positions` 带它所属的版本，所以一个坐标不会被拿去量另一版。编辑器的文本（`editor`）是这一版的文本去掉开头的字节顺序标记、每个换行（`\r\n`、`\r`、`\n`）读成一个 `\n`；`bytes` 与 `editorAt` 把这两处折叠还原，`textEdits` 把编辑器里的改动写成基线那一版的字节区间加文本，插入的换行写成这一版第一个换行的写法（`lineBreak`，没有换行时是 `\n`），所以没碰过的字节——标记、混合换行、尾随空格——一个不动（4-46）。落在一个字符中间的位置（UTF-16 代理对的两半、多字节字符的中间、`\r\n` 的两半）退到那个字符的开头。一次换算 O(`STRIDE`)：`positionsOf` 走一遍文本，每 `STRIDE`（1024）个编辑器位置记一个检查点 |
| `document_windows.ts` | 1 判定 | `Opened`（`missing`／`unreadable { reason }`／`opaque { version, bytes }`／`text { gathering }`）、`opened(answer: DocumentAnswer) -> Opened`、`recorded(version, format) -> Gathering`、`Gathering { version, format, encoding, bytes: number \| null, text, through }`、`nextSpan(gathering) -> Span \| null`、`joined(gathering, answer: RangeAnswer) -> Gathering`、`whole(gathering) -> boolean`、`EDITABLE_BYTES_MAX`：一个版本的文本由第一窗与其后按版本读的 `Range` 窗口接成（`crates/wire/Spec.lean` §8-69、§8-70）；`nextSpan` 只在没接齐、且这一版不超过 `EDITABLE_BYTES_MAX`（4 MiB）时答下一段，接不上 `through` 的窗口（别的版本、重复、乱序到达）不改变 `Gathering`。空文件是零字节、已接齐的 `Gathering`，可以写出第一版；接齐之前与超过上界的版本只读（4-46）。`recorded` 是一个只知道摘要的版本（打开的不是城此刻的那一版，或草稿立在一个旧版本上）：长度未知（`bytes` 为 `null`），从第 0 个字节读起，答回一个空窗口时长度才定下来；编码按 `utf8` 记，因为线上只在 `Document` 的答复里说编码，而这样的版本只读，带标记的 UTF-8 的标记在这种读法下仍占三个字节，只有 UTF-16 的版本位置会偏，它们本来就没有预览 |
| `document_save.ts` | 1 判定 | 草稿：`Draft { version, changes }`、`draftPlace(doc) -> string`（`prefs.ts` 草稿门的地点 `document:<地址>`，冒号不在地址文法里，所以它与房间的草稿不会同名）、`writeDraft(draft) -> string`、`readDraft(stored, length) -> Draft \| null`（存的值读不出、改动乱序、重叠或越过基线的长度时答 `null`）。回执：`Receipt`（`clean`／`draft`／`saving { sent, edited }`／`pending { sent, edited }`／`saved { version }`／`conflict { current }`／`refused { error }`）、`Sent { doc, baseline, command }`、`saveOf(doc, positions, changes) -> Sent`、`receiptIn(records, sent) -> B3Hash \| null`、`RECEIPT_QUERY`、`advance(receipt, Happened) -> Receipt`，`Happened` 穷尽：`edited { empty }`、`based { empty }`、`sent { sent }`、`lost`、`relinked`、`landed { version }`、`refusal { error }`、`moved { version }`。只有带着这次保存的 `idem` 的 `document_written` 行能把 `saving`／`pending` 变成 `saved`（`crates/wire/Spec.lean` §8-72）；拒绝不带 `idem`，所以保存途中动作是 `save a document` 的拒绝归这次保存，`E_VERSION_CONFLICT` 变 `conflict`，其余变 `refused`，草稿都留着；链路断在保存途中是 `pending`（结果未知），重连后用同一个 `idem` 再发，城答它第一次的结果，不会落第二次；城换了版本而页面有草稿是 `conflict`，只有 `based`（草稿已改立在城此刻的版本上，或已丢弃）离开它 |
| `results.ts` | 1 判定 | `Showing = whole \| results`、`drawsCalls(showing)`（房间在 `results` 下不挂载 `calls.svelte` 与推理折叠）、`Outcome = waiting \| failed \| done \| ended`、`outcomeOf(run)`、`resultsOf(runs, first) -> Group { outcome, first, total }[]`（一遍分四类，每类按 `started` 新到旧只留前 `first` 条，`FIRST = 5`）；`bandsOf(runs, now) -> Band { recency, runs }[]`（城在 `results` 下按时间读：新到旧，切成最近十分钟、这一小时、更早三段，空段不画；没有 `started` 的 run 落在「更早」末尾）；`producedOf(files) -> Produced { files, added, removed }`（房间在 `results` 下结局分隔线之下的产出一行：改了几个文件、共加减几行；二进制文件计入文件数、不计行数，因为 `Lines::binary` 没有行数可加）；只看结果模式「画什么」的唯一判定处。200 个 run 的夹具城分类耗时由 `results.test.ts` 判定并打印 `city_results` 行，登记于 `tools/xtask/budgets.toml` |
| `prose.ts` | 1 判定 | `blocks(text) -> Block[]`, `inline(text) -> Inline[]`：Markdown 读成数据，永不 innerHTML；`closedUpTo(text) -> number`：流式文字里已经闭合、可以按块画出的前缀长度 |
| `route.ts` | 1 判定 | 见 §3-2 |
| `lang.ts` | 6 数据 | 见 §3-1 |
| `time.ts` | 1 判定 | `ago`, `clock`, `hhmm`, `hhmmss`, `lasted`, `count`, `kilo`, `usd`, `kib`；`kilo(n)` 是一眼比较两个 token 数时的写法：千记 `k`、百万记 `M`、三位有效数字、不留尾零，千以下照写（上下文环的 `82.4k / 200k`） |
| `notify.ts` | 1 判定 | `notices(heard, items, scene) -> [Heard, ApprovalItem[]]`：哪些待批事项变成一条浏览器通知。`Heard` 是「快照未到」或「已算过的 `ApprovalId` 集」；`Scene { notifying, focus, elapsed, watching }`。只对需要人决定的事（`approval_queue` 的答）发，四道闸全过才发：窗口失焦、过了预热期 `WARMUP_MS`、不在首个快照里也不在已算过的集里、不是正在看的那个地址（`item.actor`）。每个见过的 id 都记进 `Heard`，所以一件事在任何一道闸下被放过一次就永远不再弹。适配器是 `views/notifier.svelte`（权限为 `granted` 才 `new Notification`），开关是 `prefs.ts` 的 `notifying`，默认 `off` |
| `keys.ts` | 1 判定 | `ACTIONS`、`Action`、`Chord`、`DEFAULTS`、`LABELS`：外壳听的每一个键在这一张表里；`readChord(text)`、`spell(chord)`、`marks(chord, platform)`、`platformOf(userAgent)`、`matches(chord, pressed)`、`reserved(chord)`、`conflictsOf(bound)`；`loadKeys(door, userAgent) -> Keymap` 把人的覆写（`prefs.ts` 的 `chord`）叠在默认上，`keymap()` 是页面的那一份。左下三键的名字、按住即现的提示、外壳与设置页都读它，没有一处自己拼一个键；外壳自己的动作是 `tier.cycle`（`\`，图层键）、`mailbox`（Accel-B，信箱键）、`inspect`（Accel-J，右侧的开合）与 `go.setup`（Accel-,，设置键），默认表里没有两个动作共用一个键；`run.stop` 的标签是 `run_cancel`（`/stop`），它发的是对眼前的 run 的 `cancel`，不是整城的 `halt`（12-16） |
| `in_front.ts` | 1 判定 | `runInFront(belief, view) -> RunBelief \| undefined`：「眼前的 run」的唯一答案。对话页是这个房间最新的在干活的 run（`newestWorking`），run 页是地址里那个仍在干活的 run，城页、楼页、monitor 与其余页面没有——它们同时画着很多 run，替人挑一个就是替人决定停哪个。Accel-.、palette 与输入框的 `/stop`、`/steer`、`/diff` 都读它（4-41、12-16），O(L) |
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
| `watching.ts` | 4 适配器 | `createWatching(sendText) -> Watching`：本页对监视器说的最大需要（面板开着 `watch`，只有设置树「性能」条目旁那一行摘要时 `watch_summary`，都没有时 `release`），新连接上再说一次；城只在有人看时采样 |
| `prefs_city.ts` | 4 适配器 | `keepWithCity(door, conn)`：偏好门与连接唯一的接点（4-29）；`adopted(held, answer)` 把城的回答盖在浏览器此刻的记录上，回答缺席的字段留浏览器的值，外观里城不保管的 `glass` 与 `blend` 也留浏览器的值；`appearanceOnWire(next)` 是外观记录的线上拼写 |
| `commands/endpoint.ts` | 2 值 | `Endpoint`、`Pair`、`Tuning`、`providerName(name)`、`probeEndpoint(e) -> Command`、`attachEndpoint(e, admit) -> Command`：挂一个端点要说的一张表单与它的两个帧；单列，因为只有这一族是人跨几屏填完才发的表单 |

`src/ui.ts` 是视图拿到的一切，一个上下文、一个取法：`setUi(value)` 由 `app.svelte` 挂载时调一次；后代组件在初始化期调 `ui(): Ui` 拿到 `Ui { conn, prefs, lang, effort, mode, approvals, bar, origin, pairing, now, chooseEffort, chooseMode, go, send, hearing }`。旧的 `useUi`／`useSay`／`useGo`／`useCommand`／`useHearing`／`useApprovals` 等透传壳收敛成这一个门（AGENTS：不做只改名的壳）。**词不是上下文**：`core/lang.ts` 的 `say(lang, key)` 保持纯函数，插槽由 `fill(pattern, slots)` 填，模板写 `say($lang, key)`，`$lang` 的订阅就是换语言时重画的来源。`pairing` 是开这一页的地址栏上的配对码，两扇会动作的 HTTP 门要它。

## 6 视图（免 SPEC，列出以便定位）

`views/parts/tip.svelte` 提示（见设计 4-18）；`views/parts/unanswered.svelte`（城没能回答的那一问，见 §5 `answered.ts`）；`views/parts/notice_title.ts` 通知的标题（见设计 4-35）；`views/parts/code.svelte` 只读代码视图（面包屑＋行号＋语法着色，见设计 4-26）＋ `parts/inked.svelte`（按语法上色的文字，文件视图与 Markdown 代码块共用）＋ `parts/code.ts`（五种 `Ink` 与取高亮器的入口）＋ `parts/paint.ts`（懒加载的高亮器块）；`views/edge.svelte` 左下三键（7E）；`views/talk.svelte` ＋ `talk/{thread,turn,calls,call_line,head,sparkline,scroller,wear,delivered,composer,waiting}.svelte` 对话（线程的读数与阅读位置见设计 4-44；`talk/{timing,call_kind,rhythm,delivery}.ts` 与 `talk/arrivals.svelte.ts` 是它们读的判定） ＋ `talk/person.svelte`（人说的一句，画成一个气泡）＋ `talk/note_line.svelte` 与 `talk/note_line.ts`（线程里的一条注记——到达、拒绝、检查点、等人——按它在账本里的行排）＋ `talk/{record,send}.svelte`（composer 的麦克风与发送键）＋ `talk/asked.svelte`（审批卡问的内容：经 `Query::Content` 读 `ApprovalItem.artifact`） ＋ `talk/produced.svelte`（一个 run 的产出短语「n 个文件 · +a −b」，由 `opened_at` 到最后一次检查点的 `Changes` 答案求和；结果房间的块与结果城的行共用）＋ `talk/{stream,result}.svelte`（只看结果的房间：`stream` 按会话分组——开着的这段在前、以它怎么开始命名，其前的并为「更早的会话」，组内新的在前；`result` 是一个 run 一块：开始时刻、结局记号、任务，引出它最后说的话，末行写做完了与产出和 PR、失败或停下的原因与「没有提交」、等你与 `ask`、或在干活与打开监视）；`views/checkpoints.ts`（`lastCheckpointIn` 是 run 最后一次检查点的树，run 页与产出一行都量到这里；`bracketOf` 是一次调用前后的两个检查点，检视面的 diff 读它）；`views/right.svelte` 检视面（见设计 4-45、7F）＋ `inspect/open.svelte.ts`（右侧打开了什么，唯一的家）＋ `inspect/reading.ts`（一次调用按什么读、右侧跟着什么）＋ `inspect/{called,diff,patch,terminal,file,shot,strip,split,crumb,reach}.svelte` ＋ `inspect/terminal.ts`（终端印什么：在跑读尾巴，有结局读账本）；`views/city.svelte` ＋ `city/{bar,panel,skyline,marks,results,produced}.svelte`（`results` 是只看结果的城：按结局过滤的分段控件带各类总数，其下是 `bandsOf` 的时间段，一行写开始时刻、结局、房间、任务与产出或停下的原因；`produced` 是一行「刚完成」末尾的产出，向这个 run 的 `Rounds` 要 `opened_at` 与最后一次检查点，再交给 `talk/produced.svelte` 的同一次 `Changes` 求和——`RunSummary` 不带产出数字，所以只有画出来的前 `FIRST` 行在问；「刚完成」一行另写 `RunBelief.pr`，「等你」一行写 `RunBelief.ask`） ＋ `city/shape.ts`（超椭圆路径）＋ `city/table.svelte` 楼表（城页默认的第一视图，画是一键可切的第二视图：账本树的第一层，一楼一行，等你／在干活／完成／最后开始四个数，加一条折叠刻度上的时间条，每个 run 的开始是一个按阶段着色的刻点）＋ `city/table.ts`（`tableOf`：楼到行，行是定长摘要，只存计数与各 run 的开始，不存历史；楼的归属、排序与树线取自 `runs/lineage.ts`）；`views/runs/board.svelte` 全城 run 板（城页画在楼表或图之下：楼是这棵树的根）＋ `runs/lineage.ts`（run 表到账本树的行、只画可见行的窗口，`boardRuns` 只读板要画的五个字段，一个流式 token 不重建整块板）＋ `runs/fold.ts`（时间条的刻度）＋ `runs/phase.ts`（阶段的颜色与说法，run 板与楼表的时间条共用一份图例）；`views/registry.svelte`（`Query::RegistryView`：这座城决定留下来的东西，一行一件，见设计 4-24）；`views/building.svelte` ＋ `building/{tree,commits,rooms}.svelte`；`views/changes.svelte`（`Changes`／`Hunks` 的一份读法，run 页与楼页共用；打开的一行画 `inspect/patch.svelte`）；`views/run.svelte` ＋ `run/{head,river}.svelte`（顶部统计栏与时间透镜）＋ `run/lanes.ts`（时间透镜画什么：每个回合一段、每列一个盒子、只画视口内的调用行，见设计 4-42）；`views/setup.svelte` ＋ `setup/{providers,models,skills,appearance,keys}.svelte`（skills 组见设计 4-31）＋ `setup/kept.svelte`（一个组的答案由谁保管，见设计 4-29）＋ `setup/decided.svelte`（代为答复的记录：`GovernanceAnswer.decided`，画在审批控件之下）＋ `setup/groups.ts`（各组的顺序、标题、提示、宽度与导航，见设计 4-33、4-36）；`views/shared/{provider,effort,buildings}.svelte`（从设置页的组里拆出的三件：`provider` 是接供应方的那扇门，`effort` 说强度住在哪一层，`buildings` 是楼列，它的第二个座位是 `#/mcp`）；`views/shared/outcome.ts`（结局的记号与墨色，结果城与结果房间共用）；`views/shared/showing.svelte`（只看结果开关，三个座位：房间、城、外观设置，都写同一条偏好，所以页上的选择就是下一页的默认）；`views/machine.svelte`（doctor 的答）；`views/notifier.svelte`（不画任何东西，`core/notify.ts` 的适配器）＋ `setup/notifying.svelte`（外观组里的通知开关）；`views/desktop.svelte`（一栋楼的桌面白名单）；`views/mcp.svelte`；`views/welcome.svelte`；`views/record.svelte`；`views/cost.svelte`；`views/palette.svelte`；`views/refusal.svelte`；`views/prose.svelte`；`views/gallery.svelte`。

**`#/gallery` 是一条路由而不是一个构建开关**，因为量它的那道门应当打开一个人真正跑的 bundle；夹具不需要城（偏好走 `core/rows.ts` 那扇门，没有 localStorage 时是一张只活一次会话的表）。每个能进入多种状态的屏幕在那里各有一份夹具，`cargo xtask render` 打开真引擎读它。`app.svelte` 用动态 `import()` 取 `views/gallery.svelte`，所以画廊与它的夹具表是 bundle 里单独的一块，只在打开 `#/gallery` 时下载：其余路由首屏不再为它付字节，而 `frontend_artifact` 称的是整个 dist，这一块仍在其中。


**本节只说哪个屏用哪个部件；部件欠使用者什么写在 §7。**

**`views/parts/` 的五个复合部件各有生产座位**，`#/gallery` 只是它们的第二个读者：`combobox` 在 `setup/models.svelte`、`setup/model_table.svelte`、`talk/composer.ts` 与 `talk/pill.svelte`（一个端点答两百个模型时，下拉正是它替换的那个控件）；`notice` 在 `views/notices.svelte` 与 `views/refusal.svelte`，AxError 的三段式因此只有它一个画法（4-35）；`row` 在 `building/commits.svelte`、`mcp/servers.svelte` 与 `record/ledger.svelte`；`skeleton` 在 `machine/skeleton.svelte`。`dialog` 的座位是撤不回来的删除，它们在发帧之前先问（12-1）：删除 MCP 服务器（`mcp/servers.svelte`）与移除一栋楼（`building.svelte`，楼的文件随之搬出城）。设置页没有移除端点的控件，所以这一族里没有第三个座位；`part_remove_endpoint` 只由 `#/gallery` 的 dialog 夹具读。

## 7 `views/parts/` 的交互契约

> **这是规格，不是描述。** 表里写的是部件欠使用者什么；今天的代码欠而未还的十二处，逐条点名在 7-8。模式名与键表借鉴自哪几份文档、为什么不产生许可证义务，一处记在 `docs/third-party.md` §6，本节不复述。

**判定：一个库只在它替换掉一样东西、并且同一变更集里有生产读者时才进来（由人定）。** `tools/xtask/src/npm.rs` 的 `RUNTIME` 是这条判定的机器面——运行时依赖恰为那张表所列，名单以表为准；加一项是门机制的一次提交，与引入它的变更集分开，好让评审看见门为什么动。**平台先来**：`parts/dialog.svelte` 把模态整个交给原生 `<dialog>`（top layer、焦点陷阱、Esc、其余页面 `inert`，见设计 4-20），`parts/tip.svelte` 把 `title` 换成一个 `role="tooltip"` 的兄弟节点（设计 4-18）；平台已经提供的行为不再引一个库来提供第二遍，因为同一件事两个提供者，第一次分歧就落在键盘用户身上。今天名单上的界面库有两项：`@lucide/svelte`，它替换了 `parts/glyph.ts` 手画的那张路径表，换来人在别的软件里已经认得的图标（4-34）；CodeMirror 6 的五个包，它替换了 RefRain 原本要手写的编辑面与行级 diff（12-23）。引入的条件写在 7-9。

### 7-1 不收键的部件

这些部件不进 Tab 序列、不读键，只欠一个角色和一组确切的 `aria-*`。

| 部件 | 角色 | `aria-*` 的确切取值 |
|---|---|---|
| `badge.svelte` | 无（行内文本） | 圆点 `aria-hidden="true"`；状态由词承担，颜色只重复那个词 |
| `banner.svelte` | 实时区域 | `weight="alert"` → `role="alert"`；其余 → `role="status"` |
| `notice.svelte` | 实时区域 | 同上；`seat` 只改画法（浮起或列在中心），不改角色。拒绝形（`action`／`code`／`subject`／`recovery`）的 `weight` 由调用方给；页面对一次落空按键的回答是 `heading`／`next` 两个 `lang.json` 键，没有码、没有折叠，取 `info`，所以是 `role="status"` |
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
| `inspect/patch.svelte` 的行号栏（调用方给了 `talk` 地址时：检视面的 diff 与 `changes.svelte` 打开的一行） | 链接 | Enter | 把「路径:新行号」（删去的行是「路径@旧提交:旧行号」）和该行的引文接到那个地址的草稿之后，再打开那段对话；composer 挂载时从草稿门读出它。每行一站 Tab，不给 `talk` 的页面行号栏不取焦 |

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
| 状态模块 | 检视面（`views/inspect/open.svelte.ts`） | 从检视面之外打开一项时记下当时持焦点的元素；关上检视面或它最后一个页签时，焦点若在检视面里（或因面板卸载落到 `body`），交还给那个元素，它已不在页面上时不还 |
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

### 7-9 一个库进来要满足的条件

四条，一条都不能省：**它替换掉一样东西**——一段自有代码、一个手画的部件，或平台在三家引擎上都给不了的一种无障碍行为；**同一变更集里有生产读者**，没有读者的库是一个有体积没人用的名字；**许可证在 `deny.toml` 的清单上**；**本节写下它替换了什么，以及引入前后 `frontend_artifact` 的读数**。门机制的那一次提交带 `Verdict: user-approved`，因为放宽 `RUNTIME` 是放宽一道门。

| 库 | 替换了什么 | 读数 |
|---|---|---|
| `@lucide/svelte` | `parts/glyph.ts` 的手画路径表（17 个图标） | 引入时在构建产物上读出，记进 `tools/xtask/budgets.toml` 的 `frontend_artifact` 行 |
| `@codemirror/state`、`view`、`commands`、`search`、`merge` | RefRain 要手写的编辑面（改动集、撤销、输入法、查找替换）与行级 diff（12-23） | 懒加载的一块，首屏不付；`frontend_artifact` 引入前后的读数由整合记进 `tools/xtask/budgets.toml` |

### 7-10 这张表的机器读者

**`#/gallery` 的夹具断言本节的键表**，这是让规格不止有人类读者的那一步：每个收键部件在那条路由上有一份夹具，夹具按「初始焦点 ＋ 一串按键 → 焦点落点、`aria-*` 取值、回调是否发生」逐行断言 7-2 至 7-6。夹具与断言的实现属于 `client/src/views/gallery.svelte` 与 `tools/xtask/src/render/`，本节只定内容。

今天的 `xtask render` 读的是画出来的盒子与它们的名字（`tools/xtask/Spec.lean` §8-13），**一次按键都没有进过真引擎**——在这第二个读数落地之前，本节的键表没有机器读者，这一点如实记在 §8 的「未验的」里。

### 7-11 外壳的控件

这些控件不在 `views/parts/` 里，但欠使用者的东西同样由本节规定。

| 控件 | 模式 | 键 | 结果 |
|---|---|---|---|
| 硬币键 | APG Button | Enter／Space | 做朝上那一面：发出框里的字，或对眼前的 run 发 `cancel`（12-16）；变淡的发送面一按落进空操作 |
| 上下文环 | `role="meter"`，提示同 `tip.svelte` | Tab | 到达环，提示出现 |
| | | Escape | 撤下提示 |
| 工具行 | APG Button（`aria-pressed`） | Enter／Space | 在右侧打开这次调用（`openCall`）；打开着的那行 `aria-pressed="true"` |
| | | ↓／↑，j／k | 同一段对话里的下一条或上一条工具行 |
| | | Escape | 右侧开着这行时收起右侧，焦点留在这行 |
| | | Tab | 到达这行，提示写出完成（或开始）的 ISO 时刻 |
| 未读计数 | APG Button | Enter／Space | 回到末尾并恢复跟随；按钮随计数消失，焦点落在对话列上 |
| 图层键 | APG Button | Enter／Space | 换到下一档 |
| | | `\`（文本框之外） | 同上；按住超过 300 ms 临时进混合档，松开回原来的档 |
| 信箱键、设置键 | APG Button | Enter／Space | 打开各自的面；面关上时焦点回到这个键 |
| 三键的名字 | APG Tooltip | 单独按住加速键 300 ms | 三个名字一起出现；松开、窗口失焦或开始组合输入即收 |
| | | Escape | 撤下正在显示的名字 |
| 检阅三栏的分隔线 | APG Window Splitter | ←／→ | 前一栏宽一栏或窄一栏，钳在每栏至少两栏宽 |
| | | Home／End | 前一栏到最窄或最宽 |
| | | Enter | 复位到默认；这些栏不收起，所以 APG 里 Enter 的「收起与恢复」在这里读作复位 |
| 右侧编辑器与终端之间的分隔线 | APG Window Splitter | ↑／↓ | 编辑器高一行或矮一行（24 px），编辑器与终端各至少留六行 |
| | | Home／End | 编辑器到最矮或最高 |
| | | Enter | 复位到默认 |
| 检视面的页签带 | APG Tabs（手动之外的自动激活） | ←／→ | 前一个或后一个页签到前面，焦点随之；到头时绕回 |
| | | Home／End | 第一个或最后一个页签 |
| | | Delete | 关掉焦点所在的页签，焦点落到它原来位置上的页签 |
| 检视面 | `<aside>` 地标 | Escape（检视面里任何地方，输入法与 RefRain 自己的面板没有先拿走这个键时） | 关上检视面，焦点回到打开它的控件（7-7） |
| 检视面的「原文」 | APG Button（开关） | Enter／Space | 在视图与原文之间切换 |
| 栏标签的菜单 | APG Menu Button | Enter／Space／↓ | 打开「移到左边／移到右边」；Escape 关上，焦点回到标签 |
| 设置树 | APG Disclosure Navigation | Tab／Shift+Tab | 依次走过条目 |
| | | ↓／↑ | 下一个或上一个看得见的条目 |
| | | Home／End | 第一个或最后一个条目 |
| | | Enter／Space（有子条目的条目） | 展开或收起 |
| | | Enter（组或页的条目） | 组画进正文；页移动地址栏并关上面板 |

`aria-*`：硬币键的可读名字是朝上那一面的动作——发送（steer 时写出落点）或停止——翻面时名字随之换，而换名不进实时区域，因为它不是新闻；变淡的发送面 `aria-disabled="true"`，`aria-describedby` 指向说明原因的提示。上下文环 `aria-valuemin="0"`、`aria-valuemax` 是窗口、`aria-valuenow` 是用掉的 token，`aria-valuetext` 是提示那一行。图层键的名字是「图层 · <当前档>」。设置树是 `<nav>` 加 `aria-label`，页条目是 `<a href>`（中键与新标签页照常可用），组条目是按钮；当前页 `aria-current="page"`，当前组 `aria-current="true"`；有子条目的条目带 `aria-expanded` 与 `aria-controls`。分隔线是 `role="separator"`，带 `aria-orientation`、以栏数计的 `aria-valuenow`／`aria-valuemin`／`aria-valuemax`，`aria-controls` 指向前一栏。右侧编辑器与终端之间的分隔线以行计（一行是三个基线步，24 px）。检视面的页签带是 `role="tablist"`，整条是一个 Tab 站（游走的 `tabindex`），每个页签 `aria-selected` 说它是不是最后碰过的那一项、`aria-controls` 指向它所在的区（编辑器区与终端区各是一个 `role="tabpanel"`）；页签上的关闭记号给指针用，`tabindex="-1"`，键盘走 Delete。「原文」带 `aria-pressed`。

## 7A 表面角色：一个面「是干什么的」只有一个家

**十一档灰阶是值的权威，角色是用途的权威，两层不重叠。** `--color-g0…g10` 与 `tools/xtask/src/color.rs:164` 的「十一档」硬断言一个字不改；本节新增的是它们之上的一层**角色**。

改前的缺陷不是缺档位，是**「什么样的面算一个抬起的控件」这个事实有两百三十六个家**——`client/src/views/` 里每一处手写的 `bg-g2` 与 `border-g3` 都是一个家，没有一个是权威，两个家不一致时谁也看不见。

### 7A-1 角色是单跳，不是值

```css
--color-raised: var(--color-g2);
```

**不许写字面量。** 一个抄在档位旁边的 `oklch()` 立刻成为那个值的第二个家，档位一动就要手工重调；一个十六进制别名更糟——`tools/xtask/src/color/tables.rs` 只保留值能解析成 `oklch()` 的声明，所以它**会被静默忽略而不是被拒绝**。单跳还让一份声明同时服务两种打光：浅色块重述每一档，指向档位的角色跟着走，不必在那里再声明一次。

### 7A-2 封闭词汇

角色名住 `tools/xtask/src/color/roles.rs` 的 `ROLES`，共 22 个：四档表面（`page` / `chrome` / `raised` / `raised-hover`）、一种透出背后的填充（`glass`，4-43）、三种非导航填充（`speech` / `track` / `disabled`）、一种标记填充（`mark`）、三档边（`edge` / `edge-panel` / `edge-input`）、一种覆在彩色实心上的墨（`on-accent`），以及城市插画自己的九档（`drawn-*`）。

**加一行是一次设计决定。** 只有当一个人能用一句不提档位的话说出它回答什么问题时，这个角色才配有名字——草稿里 `inert` 与 `resting` 相隔一档，没有读者说得出某个圆点是哪一个，它们现在是一个 `mark`。

### 7A-3 闸判四条（`cargo xtask color`）

1. 每个既非档位、非文字 token、值又不是 `oklch()` 的 `--color-*`，必须是 `ROLES` 里的名字，且是到一个已声明档位的**单跳**；
2. `ROLES` 里的每个名字，样式表里**恰好声明一次**；
3. 每个角色在 `client/src` 里**至少有一个读者**——没有读者的角色是一个有值没人读的名字，删掉而不是留着；
4. `theme.css` 之外的任何文件**不得拼出档位工具类**（`bg-g2`、`border-g3`、`fill-g9` …）。

**闸因此多一条规则而不是少一条**：十六进制别名既解析不成 `oklch()`，也不是单跳，两道都拦。

### 7A-4 图底关系：页面是一块面，右侧是唯一抬起的板

`tools/xtask/src/color.rs` 有「g0 是页」的契约，ramp 两端被硬断言。内容与外壳都在 `page` 上：分区由网格与 1 px 的分隔线承担，不由面的深浅承担，因为一个宣告自己的框会跟它框住的工作抢注意力。**右侧的框是唯一抬到 `chrome` 的面**——页签带与终端；编辑区留在 `page`，所以屏幕上最深的一片仍然是正在读写的东西。左下三键是玻璃：`glass` 角色按 `--glass-opacity` 透出背后（4-34、4-43）。

要紧的距离仍是 `page` → `raised`：一个控件必须不靠边框就看得出可以按。

## 7B ACCENT 说「在动或被选中」，ALERT 说「要人或少了」

**accent 只回答一个问题：这里有东西在动，或这是被选中的那一个。** 它的座位是：在跑的状态点与它的脉冲、选中行的 2 px 左边条、焦点环（`color-mix` 削到六成）、图层键下标出当前档的刻度、时间轴里的检查点、diff 的加行、成功的退出码，以及提交栏里所选会话的那条泳道。**alert 只回答另一个问题：这里要人，或这里少了东西**：等你的状态点、信箱的计数、改过未存的页签、diff 的删行。**绿与红只出现在上下文环的两个检查点上**（7J）：两个点要在一圈白线上一眼分出先后，而 accent 与 alert 已各有意思，所以它们是界面上仅有的另外两个色相。

其余一切「当前／已选」用表面差抬一档表达：分段控件的滑块是 `raised-hover`，所选的行与页签是叠在页面上的一层淡色。

规则仍是一句可核的话：**全屏对比度最高的元素应当是「停」**。硬币键的停止面是页面底色上的实心字形，深浅两种打光下都是全屏亮度差最大的那一处，因为那是人需要在慌乱中一次按中的东西。

## 7C 需要人同意的东西，长什么样

三样东西会停下来问人：附着到人自己登录着的浏览器、在沙箱外跑一条命令、队列里等签字的审批。它们**共用一套语言**，人学一次，之后每一次都是认出来而不是读出来。

- **前缘一条 2 px 的条，加一个字形。**
- **字形是编码，颜色只是加强。** `forced-colors` 会把每一处填充与边框颜色换成系统色，所以只用颜色说「请决定」的标记，恰好在最需要它的那些机器上什么都不说。
- 条画在**前缘**而不是左边，因为这一页也会用从右往左的语言排。

实现是 `theme.css` 的 `@utility asks`，连同 `@media (forced-colors: active)` 里把它的边换成 `CanvasText` 的那一条。读者：设置行的门与沙箱芯片、检阅档仪表的「边界」格、审批卡。

## 7D 一屏一家：一个事实在一屏上只画一次

**同一屏上，一个事实只有一处画它。** 窗口底部没有一行常驻的读数：每个读数都在读者已经在看的地方，而每一档有一张座位表。

| 事实 | 专注、混合 | 检阅 |
|---|---|---|
| 模型、强度（会话开始前） | 对话框设置行右端的两个选择 | `/model`、`/effort`（对话栏不画设置行） |
| 模型、强度（会话开始后） | 首条消息头 | 所选会话仪表的「模型」格 |
| 房间、门、沙箱 | 设置行左端的三枚芯片 | 所选会话标题下的地址行与仪表的「边界」格 |
| 上下文 | 上下文环 | 上下文环；仪表的「上下文」格写确切数字，不画条 |
| 本次花费、全城花费 | 不画 | 仪表的「成本」格 |
| 时刻 | 消息头写到秒；工具行写毫秒用时，完成的 ISO 时刻在它的提示里 | 时间轴：轴头一次写日期与时区，每行写 `HH:MM:SS.mmmZ` |
| 提交的身份（oid、B3、父提交） | 不画 | 提交栏 |
| 连接 | 断线横幅（只在断开时）与标签页图标（`core/mark.ts`） | 同左 |
| 城的进程读数（处理器与内存） | 性能页（`#/monitor`）；设置树「性能」条目旁的一行摘要 | 同左 |

**环与「上下文」格是唯一一处两画**：环画比例与两个检查点，格写数字，两者读同一个回答；环在眼睛已经在的地方，格在要读确切数字的地方，去掉任一处，人都得换一档才看得到另一半。**专注与混合两档不画花费**（由人定）：这两档只留正在做的事，花了多少在检阅档读。**风险读数仍不必去找**：沙箱敞开、或门读不出来时，对应的芯片带 7C 的 `asks` 标记；检阅档里同一个标记落在「边界」格上。模型与强度在会话开始后不再是对话框上的控件，因为一个按不动的控件比一个词更糟，而首条消息头是那次冻结的唯一记录。

**当前状态**：所选会话的仪表还没有建成（7K），检阅档的对话栏因此照画设置行，房间、门与沙箱留在芯片上；仪表落地时设置行在检阅档撤下。设置树还没有建成（7L），进程读数的摘要（`watching.ts` 的 `watchSummary`）在它落地之前没有读者。

## 7E 左下三键

第 1 栏贴底竖排三个键：图层、信箱、设置，图标取 lucide 的 `layers`、`inbox` 与 `settings`。**三键之上没有任何东西**：花费回到了所选会话的仪表（7D），连接只在断开时以横幅出现。

- **图层**换档（7H）：点一下按专注 → 混合 → 检阅 → 专注循环，键下三个 4×2 的刻度用 accent 标出当前档；`\` 是同一个动作的键。在键上按住或按住 `\` 超过 300 ms 临时进混合档，松开回到原来的档，所以看一眼世界层不改变人选定的档。
- **信箱**的角标是等人决定的事的件数。**当前状态**：三段信箱（待决、在跑、最近）还没有建成，这个键与 Accel-B 打开 4-35 的通知抽屉；信箱落地时只换它打开的面。
- **设置**打开设置面（7L），Accel-, 是它的键。

**名字与键按需出现，不常驻**：指针悬停或键盘聚焦时，键的右侧弹出它的名字与键，例如「信箱 · 3 件等你 Ctrl B」；单独按住加速键超过 300 ms，三个键的名字一起出现，松开、窗口失焦或开始组合输入时收起（按住即现）。名字与键都读 `core/keys.ts`，没有第二处拼写。触屏上第一次点击就执行动作，名字不靠悬停才能读到。

## 7F 右侧：编辑器在上，终端在下

右侧是盖在网格上的一块工作面（4-27）。顶部是 48 px 的页签带：打开的文件与终端各一个页签，改过未存的文件带一个 alert 圆点；带的右端是收起键。其下编辑器在上、终端在下，中间一条可拖的分隔线（键同 7-11 的分隔线）。编辑器上方一行写路径与比较的两个提交；diff 的加行与删行各取 accent 与 alert 的淡底，光标所在行一条 2 px 的 accent 左边条。终端的头两行：第一行是命令，第二行是用时、退出码、完成的 ISO 时刻与被裁掉的行数，右端一个「原文」入口；输出不折行，横向滚动。

**页签带**是每个打开的项一个页签，终端的页签带终端字形。`源码／预览／diff／版本` 四个读法属于文档，由 RefRain 在它自己的头里给出，因为 RefRain 的入口只收 `{ building, path, version }`，带上没有一条把读法交给它的路；调用的读法由调用决定（4-45）。只有一个区有项时那一区占满全高，不画分隔线。**当前状态**：改过未存的文件页签的 alert 圆点要等 RefRain 把草稿状态交给页签带；终端的退出码今天从 exec 结果的 JSON 里读，`Call` 上的退出码字段落地后改读它。

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
| 端点卡之后 | Tab 到 `main` 的模型选择框，方向键选定 | 对话框设置行的模型选择显示所选模型 |
| `#/welcome` 的「派活」卡 | Enter | `#/talk/hall/mayor`，composer 取焦 |
| composer | Enter | 派活帧发出 |

**这条路今天有两处还不是一步：** 第三行的选择框在设置页的末尾，1440×900 的窗口里要滚动才看得到，挂上第一家供应方后它既不紧跟在卡下，也不取焦；最后一行在无主模型时仍发出派活帧，由城拒绝，composer 的发送按钮尚未换成去 `#/setup` 的链接。两处都改完时，上表每一行的落点都不需要一次滚动或一次无效的按键。

**拒绝框画城给的出路。** 同一个码覆盖几种原因（`E_CONFIG_INVALID` 既是「没选模型」也是「会话中途换了模型」），所以 `err_<code>` 的标题只说拒绝的种类，不说原因；`parts/notice.svelte` 把城写的 `recovery` 句子不折叠地放在标题下，动作与主体留在折叠里（12-5）。

## 7H 三层与三档

**三层**：世界层在下（会话、提交与文件），对话层在上（线程、消息头、工具行），边缘层浮在两者之上（左下三键、信箱、设置面与对话框）。**字不压字**：世界层的文字面板只摆在对话列的两侧或取代它，从不画在对话的字底下，因为透明度在 5%–50% 之间时，叠在文字上的文字读得最慢（Harrison, Ishii, Vicente, Buxton 1995）。

**三档**说世界层画多少，偏好是 `core/prefs.ts` 的 `tier`，取 `zen`／`blend`／`panorama`，中文是专注、混合、检阅（12-17）：

- **专注**：世界层不渲染——不挂载，所以它的问题也不问。屏上只有对话与左下三键。
- **混合**：世界层以整块面板出现在对话两侧，左是会话、右是地方栏（7K），每块带一条 1 px 的轮廓线而不用模糊——轮廓把面板与旁边的字分开，模糊会让一块本来就降了不透明度的面再糊一层（refrain 路线图 P3、P4）；降到 `--blend-opacity`（默认六成，外观组可调，4-43），不收指针也不收键盘（`inert`），因为一块点不动却 Tab 得进去的面板会把焦点带到看不清的地方；栏标签在这一档只是字，不是菜单。对话的字下什么都不压。右侧打开时世界层收起（4-27）。
- **检阅**：世界层是工作区（7K），对话退成所选会话一栏底部的一条（7I），仍能发送与 steer。世界层跟随工作区：地方栏在市长的房间旁画城，在楼里的房间旁画这栋楼的提交图与文件。

**默认是混合**：第一次打开的人看得见世界层在那里，正文又不被它抢走。档存在浏览器里，`PreferencePatch` 有了这一臂之后随偏好入城（4-29）。**换档不重建对话框**：三档里对话框是同一个元素，换档只改它所在的栏与高度，输入、选区、输入法的组合与焦点都留着，因为换档打断输入的代价落在人正在写的那句话上。世界层出现与离开是 `duration-page` 的不透明度过渡（出现取 `--ease-arrive`，4-43），对话框挪位只动 `transform`；动效关掉时一切直达终态。

## 7I 对话框：字在上，一条横线，设置在下

对话框照一页纸来写：字在上，下面一条 1 px 的横线，设置项在横线下。没有框、底色与影。

**横线跟着字走。** 框空时横线只从左缘伸出一段，在 160 px 内淡出，像终端只露出提示符；打字时实线延伸到字的末尾，再用 160 px 淡出，所以第一次写字像一条进度；发出后缩回。框既空又没有焦点时横线降到半透明。实现是 `@property --typed`（`<length>`）上 `duration-panel` 的过渡（`--ease-arrive`，4-43），字宽用 canvas 的 `measureText` 按输入框的字体量，多行时量最长的一行，封顶为框宽。

**硬币键**：发送与停止是一个键的两面（12-18）。哪一面朝上由一处判定：眼前有 run（`core/in_front.ts`）且框空，是停止面；框里有字，是发送面，run 在跑时这一发是 steer，键的名字按 4-13 说出落点；两者都没有，是变淡的发送面，按不动，提示里说为什么。翻面是 `rotateY(180deg)` 的 `duration-panel` 过渡，动效关掉时改为淡入淡出。键没有自己的边：底色就是页面色，只有 18 px 的字形，悬停叠 8% 的字色，按下缩到 94%。麦克风只在这座城有转写端点时出现（4-16），画在环的左边。

**设置行**：左端三枚芯片，房间、门、沙箱。右端是模型、强度与模式三个选择，只在会话开始之前出现；开始之后它们是首条消息头里冻结的事实（7D）。**当前状态**：「谁在听」——城、楼、居民，以及这次 run 被告知的四段（`run/prompt.svelte`）——不再是对话框下的一行，今天从 run 页的 prompt 透镜读到；把它并进房间芯片、点开即读，是还没做的一步。

**空房间**：专注档里对话框立在页面的竖直中线上，上方一行写收件人与房间地址（4-10）；第一次发出后同一个元素以 `duration-page` 的 `--ease-arrive` 沉到底。占位符是「写给 {收件人}…」。

**检阅档的对话栏是同一个对话框**：上方一行是最后一句话，排法同线程里的消息（消息头一行，正文截成一行），横线、环与硬币键都不变；设置行不画，因为所选会话的仪表已经写着房间、门与沙箱（7D 的当前状态说明了仪表落地之前怎么办）。

## 7J 上下文环

硬币键外面一圈 1 px 的环，盒子 40 px、半径 18。它回答一个专注时也要知道的问题：这个会话的上下文窗口用掉了多少。

**画法**：深色打光下，窗口全空时是一整圈实心白线（`solid`），用掉的部分从 12 点起顺时针被页面色吞掉；浅色打光下黑白相反。实现只画剩下的那段弧（`pathLength="100"` 加 `stroke-dasharray` 与 `stroke-dashoffset`），不在白环上叠一段页面色的弧，那样在两段相接处留毛边。环上两个 5 px 的检查点，外裹 1.5 px 的页面色：绿是第一级提醒，红是第二级（交接）提醒，各在它的百分比处。悬停或聚焦时弹出一行：「82.4k / 200k · 已用 41% · ● 提醒 30% · ● 交接 65%」。

**读什么**：用掉的量是这个会话最新一个回合的 `Turn.used.input`（供应方报告的输入 token），窗口是答这个会话的模型在端点表里的 `context_tokens`，与城算上下文提醒用的是同一对数（glossary 的 context reminder）。第二级的百分比读 `ConfigAnswer.second`。第一级百分比的权威是 `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`，页面不抄它。**哪一个会话**：对话框说话的那个房间里最新的会话；房间还没有会话时环是整圈，提示只写窗口大小；端点没报 `context_tokens` 时不画环，只留硬币键。

**当前状态**：线上还没有带第一级百分比的回答，所以环上不画绿点，提示里也不写「提醒」那一段；它上线时只加一个读者。

## 7K 检阅档的工作台

检阅档里世界层是工作区，分三栏：会话、所选会话、这份工作所在的地方。会话、提交与文件在城里本来绑在一起——一个 run 在它的工作树里写文件，检查点把它们提交成一个提交——工作台把这层绑定画出来：**选一个提交即选中它的会话，并把时间轴定位到产生它的检查点**；选一个会话，提交栏标出它的那条泳道。「所选会话是哪一个 run」只在 `views/world/chosen.svelte.ts` 判定一次：选过的提交所指的 run（只在选它的那个房间里算数），否则是这个房间还在干活的 run，再否则是它最后持有的 run；所选会话一栏、提交栏的泳道与时间轴都读这一个答案。选另一个房间的提交时，对话经地址栏移到那个房间，与点一行会话是同一条路。

- **会话栏**：按楼分组。每行一个状态点（在跑 accent、等你 alert、已冻结是空心圈）和名字；在跑的写整个 run 已跑多久，等你的写「等你」，冻结的写「已冻结」；第二行是任务，第三行是一根 2 px 的上下文细条，带交接刻度。
- **所选会话**：标题与状态，状态旁是去 run 页的链接；一行地址——房间、工作树、基于哪个提交（`RoundsAnswer.opened_at`）；一张仪表：模型（与强度、模式）、上下文（确切数字与两级提醒，读 `talk/gauge.ts` 的 `contextOf`，与上下文环是同一对数）、Token（各回合供应方报告的入与出之和）与单价（端点为这个模型报的价，原样）、缓存（命中率是缓存读的 token 占输入的份额，`Used.input` 含缓存读的部分；读与写）、成本（本会话读 `views/pricing.ts` 的 `runSpend`，全城读同一个 `CostAnswer.total`）、速度（各回合首块用时的中位数，回合的时刻不是量出来的、或没有首块的回合不计入，而不是计作零）、边界（写入限制、沙箱、门；沙箱与门读 `talk/bounds.svelte`，与设置行的芯片是同一个判定）、准入（准入证据与落地策略）。读不到的数画成一道破折号，不猜。仪表之下是时间轴：轴头一次写日期与「UTC」，每行 `HH:MM:SS.mmmZ`——回合写它被问的时刻，调用写结果落账的时刻，检查点写它的提交的时刻；回合行写首块用时与本回合 token，调用行写毫秒用时（一秒以上写到毫秒的秒数），在跑的调用写「运行中」，exec 另写退出码，检查点行写提交的短 oid 与改了几个文件。调用行一点在右侧打开这次调用（`inspect/open.svelte.ts` 的 `openCall`）；检查点行一点选中它的提交。时间轴只在账本事件上重画，token 不碰它。
- **地方栏**：栏名是这份工作所在的地方——市长房间旁是城的名字，楼里的房间旁是楼的地址；栏里两个页签（APG Tabs）。第一个页签是提交：市长房间旁是全城的提交，楼里的房间旁是这栋楼的，都是 `commitsQuery` 的第一页，画成从 `CommitAnswer.parents` 走出的泳道图（`views/world/lanes.ts`，git `log --graph` 的走法：每条泳道等一个 oid，第一个父提交继承泳道，其余父提交另开泳道；页外的父提交让泳道一直画到页底，线上没带父提交的提交让它的泳道结束）。每行写 oid、消息与房间；节点按做它的 run 的阶段着色（`runs/phase.ts`），所选会话的提交与它的泳道取 accent。选中的提交在行下展开完整 oid、父提交与改动的文件，文件一点展开它的补丁，补丁的行号把「路径:行」接进对话的草稿（`views/changes.svelte`，与楼页同一份读法）。页里还有更早的提交时，栏底一条链接到楼页。第二个页签跟着工作区走（refrain 路线图 Q10）：市长房间旁是城的天际线（`city/skyline.svelte`，按城页的尺寸画、横向滚动、打开时停在市政厅，因为缩到一栏宽时每栋楼的名字小于七像素），点一栋楼，工作区移到那栋楼里最近跑过活的房间，没有就移到楼本身；楼里的房间旁是这栋楼的文件树（`building/tree.svelte`，一层一问 `Query::Listing`），点一个文件，它在右侧以工作树此刻的文字打开（`openDocument`）。

**栏的顺序与宽度归人**：两条分隔线可拖、可用键盘调（7-11），宽度以栏为单位、拖动吸附到栏线，每栏至少两栏宽，默认 3、5、4 栏；每栏的标签是一个菜单按钮，菜单里的「移到左边／移到右边」换栏序，宽度随栏走。顺序与宽度存在浏览器（`core/workbench.ts` 判值，`prefs.ts` 存取，12-24），在分隔线上按 Enter 或双击，整个工作台复位到默认的顺序与宽度。对话条总在所选会话一栏之下；所选会话排在最左时，对话条从第 2 栏起，给左下三键留出第 1 栏。右侧打开时，会话与所选会话按人的顺序各占 2 与 5 栏，地方栏收起，分隔线与菜单不出现。

**当前状态**：仪表与时间轴要的几样数据还不在线上——第一级提醒的百分比、缓存写的数、exec 的退出码、提交的 B3、每个 run 的写入限制与准入要求与落地策略、工作树名、回合结束的时刻（没有它，t/s 无从算，速度格只写首块用时）；会话栏的上下文细条也要第一样。线协议补上之前，这些格与行画成破折号或不画，模型格不写强度与模式。选中提交的文件在行下展开补丁，而不是在右侧打开：右侧的条目（`inspect/open.svelte.ts`）有调用与文档两种，还没有「两个提交之间的一个文件」这一种。

## 7L 设置面与设置树

设置键与 Accel-, 打开设置面：从左缘弹出的原生模态 `<dialog>`（4-20），宽不超过 `page`（1040），左边是设置树，右边是正文。点外面、按 Esc 或再按一次设置键关闭，焦点回到设置键（7-7）。**城、楼、记录、成本、登记簿、MCP 与性能这些页都从这棵树到达**（4-8，12-19）。`#/setup` 是设置面开着的地址：从链接打开它时，面板开在与市长的对话之上。

**树最多三级**：组、条目、子条目。条目有两种：一种是设置组，选中后画在面板的正文里，面板不关；另一种是页，选中后地址栏移到那一页、面板关上，条目尾部带一个箭头字形，说明它会离开面板。

| 组 | 条目 | 子条目 |
|---|---|---|
| 城 | 总览（`#/city`）、楼、登记簿（`#/registry`）、成本（`#/cost`） | 楼：每栋楼一项（`#/building/<楼>`），按城的回答列出 |
| 接入 | 账户与供应方、官方 harness、MCP（`#/mcp`）、网络 | — |
| 运行 | 运行、技能、依赖 | — |
| 偏好 | 外观、快捷键 | — |
| 诊断 | 记录、性能（`#/monitor`）、高级、关于这一版 | 记录：账本、归档、回收站、日志（`#/record/<lens>`） |

**分组按问题分**：城是这座城自己的事实，接入是城能够到什么，运行是一个 run 被允许做什么、要哪些程序，偏好只属于这个人与这个浏览器，诊断是出了事去看的地方。**组不收起，条目才收起**：五个组的标题常开，有子条目的条目（楼、记录）是可展开的一项，当前页所在的那一项开着；所以从对话页到任何一页最多是「设置键、条目」两步，楼与记录的子项多一步逐层浏览。

**当前状态**：设置还是一页（`views/setup.svelte`，左边是十个组的竖排导航），其余页面由 Ctrl-K 与 `go.*` 键到达；设置面与设置树还没有落地。

## 7N RefRain：右侧的文档编辑器

右侧的文档项（`views/inspect/open.svelte.ts` 的 `DocumentItem`）画 `views/refrain/refrain.svelte`，它只收三个参数：楼、楼内路径、版本（`null` 是城此刻的文本）。名字取自它的来处 RefRain（由人定）。它编辑 Markdown 与纯文本，读法有四种，状态有一行，行为由 4-46 规定。

**头一行**（32 px，与 7F 编辑器上方那一行同一个座位）：左边是路径，楼名淡、文件名实；接着是版本的前七位与保存回执；右端是读法的分段控件「源码／预览／diff／版本」与保存键。读法只有 Markdown 才有「预览」。**回执**一个词加一个形状：草稿（alert 圆点）、保存中、已保存、待核对（链路断在保存途中，重连后用同一个键再发）、冲突、被拒；没有改动时不画。待核对与被拒在头一行下面多一行：前者说为什么还不知道结果，后者是城写的出路（12-5）。

**四种读法**：

- **源码**：CodeMirror 6 的编辑器，Markdown 与纯文本都按字面显示，纯文本没有预览（A5）。撤销与重做只在本页的草稿里；查找与替换是编辑器自己的面板，字句取自 `lang.json`。接齐之前、超过上界的版本、历史版本都只读，只读的原因写在头一行下面的一行里。
- **预览**：`Query::Preview` 逐窗画基线那一版的块（4-26），滚到底再要下一窗；草稿不在预览里，有草稿时预览上方一行这样说。`Preview::Unsupported` 说这一版不按 Markdown 读，零个块是空态。
- **diff**：还是那个编辑器，加上与基线的对照（删去的行是 alert 淡底、加上的行是 accent 淡底），仍可编辑；没有改动时是「与基线相同」的空态。
- **版本**：本页拿得到全文的几版（4-46），每行写版本前七位、来处（打开的／你存下的／城换掉的）与时刻；选两行，下面是两版之间只读的 diff，默认是最近的两版。

**切换读法不卸载编辑器**：源码与 diff 是同一个编辑器，换读法只换它的一个扩展；预览与版本画在它旁边，编辑器只是隐藏，所以光标、选区、撤销栈与输入法的组合都留着。位置跨读法对应：从源码到预览，光标所在的字节所在的块滚进视野；从预览回源码，视野最上面那一块的起点成为光标、滚进视野。右侧的开合、换档与窄窗口下的重排都不重建编辑器：只有 `building`、`path` 或 `version` 换了才换一份文档。

**草稿与恢复**：每次改动后 300 ms 把草稿写进浏览器（4-46）；关掉右侧、换一份文档、重新载入都不丢。重开时草稿基于的版本仍是城此刻的版本就照原样恢复，否则进冲突。**冲突**是编辑器上方一条 `asks` 标记（7C）的横条：一句话说城里的文件已经换了一版、草稿留着，三个动作「对照」「移到现版」「丢弃草稿」，丢弃先经 `parts/dialog.svelte` 确认（12-1）。

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

`aria-*`：编辑区的可读名字是文件名（`aria-label`）；只读时 `aria-readonly="true"`。回执是 `role="status"`，只在它换了词时播报，打字不播报（§3-14：流式与逐键都不逐字播报）；冲突条是 `role="alert"`，出现时播报一次，不取焦点。

**当前状态**：右侧的页签带（7F）由检视面画，四个读法因此暂在 RefRain 自己的头一行；页签带接管读法时只搬这个控件。

## 8 验收

`bun run lint`、`bun run typecheck`、`bun run test` 三样绿，`cargo xtask npm`、`cargo xtask wire-ts`、`cargo xtask color`、`cargo xtask wording`、`cargo xtask render` 绿；`just check-client` 是这三条脚本的一条线。

在一座真城加一个说 OpenAI 形的假供应方上走得通的：连接与握手、欢迎页三张卡各自的落点（§7G）、从 composer 派活、工具调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与文件原文、run 页的统计栏与透镜、记录四透镜（ledger／archive／bin／log）、成本页、设置页 attach 与 select、`#/mcp` 三扇门加删（写进 `CONFIG.toml`）、楼页提交列表与 session 跳转、草稿留存。

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
- **被击败的备选**：Solid（`solid-js` 与 `vite-plugin-solid`）。它输在上面两件编译与车道的事上，不是输在一个读数上。SvelteKit（仍是 Vite，外加路由、SSR、`load`、服务端 endpoint 与 adapter）。它的主要能力都假定有一个 JS 进程在服务端跑页面，而这里的服务端是 Rust：产物在构建时嵌进二进制，城原样答它，数据全走 `/ws`，能用的只剩 adapter-static 加 hash 路由。代价有三：URL 文法从 `core/route.ts` 的穷尽 `View` 搬进目录名与 param matcher，成为同一事实的第二个家；十几个同名 `+page.svelte` 违反「一个模块一个文件、按它拥有的东西命名」；`$app/*` 虚拟模块在 `bun test --conditions=browser`（4-2）下跑不起来。它能给的按路由拆包由动态 `import()` 给出（`#/gallery` 已是这样），PWA 的 service worker 读 Vite 的 build manifest 手写。
- **重开参数**：§3-4 的两个读数量出来以后，同一仪表、同一机器上 Solid 一臂的产物不到 Svelte 一臂的一半，或每帧折叠开销不到一半，就重新论证本条。出现第一个真正需要组件库的需求时，先过 §7 的 `RUNTIME` 判定与 7-9。SvelteKit 一条在以下任一情况出现时重开：客户端改由一个 JS 服务端托管（例如远程门另起一个 Node 或 Bun 站点）；需要服务端渲染或预渲染的页面；页面数多到手写的 `route.ts` 本身成了负担。

### 12-13 随包的第三方许可文本由打包器从产物里认出

- **决策**：`client/scripts/notices.ts` 给 Vite 一个 plugin，在 `generateBundle` 时从每个 chunk 的 `moduleIds` 里认出 `node_modules/<包>`（带 scope 的取两段，嵌套的 `node_modules` 取最后一段），读该包目录下的 `package.json`（版本、`license`）与包目录顶层的许可文件（文件名以 `LICENSE`、`LICENCE`、`COPYING` 或 `NOTICE` 开头，不分大小写），写成产物根下的 `THIRD-PARTY-NOTICES.txt`：按包名排序，同一个包的多个模块只出现一次，字节只取决于输入。二进制嵌入整个产物，所以这份文件随二进制分发，城在 `/THIRD-PARTY-NOTICES.txt` 答它。字体的许可仍是 `fonts/OFL.txt`，本文件开头指向它。一个进了产物却没有许可文件的包让构建失败，并点名它。
- **理由**：`svelte`、`effect`、`@lezer/*` 与 Svelte 运行时带进来的包是 MIT 或 Apache-2.0，两者都要求版权与许可声明随副本分发，压缩后的 bundle 也是副本；物料清单（`xtask sbom`）只列 cargo 包。从产物认包，而不从 `package.json` 或 `bun.lock` 认：前者漏掉传递进来的运行时包，后者把 devDependencies 与 tree-shaking 删掉的模块也算进去。包目录取自模块路径本身，而不是按包名到 `client/node_modules` 下去找：打包器读的是哪一份，声明就写哪一份。缺许可文件即失败而不是跳过：悄悄少了一个包的声明，与没有声明是同一个缺口。
- **被击败的备选**：一个现成的 rollup 许可证 plugin——多一个 devDependency 做几十行就能做完的事；把 npm 包写进物料清单——清单是 cargo 的 CycloneDX，且不在二进制里。
- **重开参数**：产物里出现一个许可要求别的形式的包（例如要求在界面上署名），或这份文件让 `frontend_artifact` 的读数增长超过 8 KiB。

### 12-14 Markdown 不在浏览器里读，`RUNTIME` 不为它加一项

- **决策**：refrain 路线图 S7.9 要求二选一写进条目的那一项，选城侧：Markdown 由 `documents::markdown`（comrak）在城里读成块，页面经 `Query::Preview` 拿到块再画（4-26）；comrak 不编成 wasm，`RUNTIME` 与包体不因它多任何东西。
- **理由**：文档的版本在城里，页面只持有窗口，预览随一次往返就到，与它读 `Range` 是同一种代价；导出在 Rust 里读同一个函数，一个文法就只有一份构建。删掉 `prose.ts` 之后包体还少 1–2 KB，而 `frontend_artifact` 的余量本来就只有几 KB。
- **被击败的备选**：comrak 编成 wasm、由页面加载。它省掉每一窗的往返，经远程门的设备上这一点更明显；但 wasm 的导出要 `unsafe`，工作区里只有 `crates/desktop/ffi` 可以有自己的 lint 表，多一个 crate 就要人的定规，工具链多一个目标，包体推断多 100 KB 以上，同一个 comrak 还要在两条构建路上各编一次（`crates/documents/Spec.lean` D20）。
- **重开参数**：远程门上一次往返量出来超过 100 ms（refrain 路线图 §5 的 `client_send_feedback`）而 12-15 的做法消不掉它；或允许第二个带自己 lint 表的 crate 的定规出现。

### 12-15 对话流按文字问城，页面不再自己读 Markdown

- **决策**：对话流的回复由城读成块，页面经 `Query::Reply { text, state }` 问（4-26，`crates/wire/Spec.lean` §8-75）；「还在说的回复里哪些块已经不会再变」由城判（`layout::closed`，`crates/documents/Spec.lean` D31），页面只决定什么时候问。`core/prose.ts` 与它的 `closedUpTo` 在页面接上这个入口时删去。
- **理由**：一个文法管文档、对话流与导出（refrain 路线图 §4-9）；页面手里本来就有回复的文字，流式时来自增量，从历史打开时来自 `model_returned`，所以一条带文字的查询两处都用，删掉 `prose.ts` 之后页面里没有第二个 Markdown 读法。收束的规则也搬进城里，因为它判的是文法里的块，留在页面就是规则的第二个家。
- **被击败的备选**：按版本问——回复要先另存进内容库（`crates/documents/Spec.lean` D30 的 (a)），从历史打开的旧回复没有版本；城在增量旁边带上块——增量可丢，漏一帧就少一块，历史也还要另一个入口（D30 的 (b)）；页面留着 `closedUpTo` 只把闭合的那一段送去——收束的规则在页面与城各写一份，围栏的认法已经不一样（D31 的③）。
- **重开参数**：远程门上量出的一次往返超过 100 ms，使流式期间块的出现明显晚于文字，那时流式这一半另加 D30 的 (b)。

### 12-16 停止键只停眼前的 run，眼前没有 run 时只提示

- **决策**：Accel-.（`run.stop`）、palette 与输入框里的 `/stop` 都只对眼前的 run 发 `cancel`。眼前的 run 由 `core/in_front.ts` 的 `runInFront(belief, view)` 判定：对话页取这个房间最新的在干活的 run，run 页取地址里那个仍在干活的 run，其余页面没有。眼前没有 run 时，Accel-. 不发任何帧，toast 座位出一条 `info` 提示，标题 `no_run_in_front`，正文 `stop_whole_city` 写出 `/halt --all`；这条提示不进抽屉。
- **理由**：glossary 把 Halt 与 Cancel 定为两个动词：Halt 关闭一个范围、终止其中积压的活，Cancel 只停一次 run。这个键原先发整城 `halt`，palette 的 `/stop` 又取全城最新的在跑 run，所以人按文档按下 Ctrl+. 想停眼前这一次，得到的是整座城停摆，或是另一个房间的 run 被取消；run 页输入框里打的 `/stop` 则什么都不做。三个入口给出三种结果，所以「眼前」只在一个函数里判定，三个入口都读它。眼前没有 run 时，页面不替人猜要停哪一个，也不把一次按键放大到整座城，只写出整城的拼写，由人自己打。提示不进抽屉：抽屉存的是城对人说过的话，一次落空的按键是人自己刚做的事，记进去会让栏顶圆点显示未读。
- **被击败的备选**：一，眼前没有 run 时停全城最新的在跑 run，即 palette 原来的读法；它可能停掉人看不见的另一个房间的 run。二，打开一个在跑 run 的选择面；它多出一个只为这个键存在的部件，而人按停止键是要停，不是要选。三，提示里放一个 `/halt --all` 按钮；人刚凭反射按了停止键，角落里再给一个一按就停整城的控件，正是本条要去掉的放大。四，把提示铸成一条 `E_INVALID_ARGS` 拒绝，照页面自铸 `E_TIMEOUT` 的先例走拒绝通路；标题会说城看不懂这个请求，而城什么都没收到，`recover_e_invalid_args` 还会把写着 `/halt --all` 的那句挤进折叠。
- **重开参数**：一页上出现第二种「眼前」时（例如检视面展开的一次调用属于另一个 run，或一页并排两个 run），`runInFront` 改为读那一面自己的选择；`halt` 有了确认或撤销时，停整城的控件可以回到这条提示里。

### 12-17 三档叫 zen、blend、panorama，偏好叫 tier

- **决策**（改名由人定）：三档的存储与线上拼写是 `zen`／`blend`／`panorama`，中文是专注、混合、检阅；偏好的名字是 `tier`。
- **理由**：一个名字一个概念。`focus` 在页面代码里已经是 DOM 焦点（`focus()`、`:focus-visible`、`composer.focus`），`tier === "focus"` 与 `box.focus()` 会在同一个文件里相遇；`survey` 已经是 `browser::survey`，render 门量几何的那个读者。`zen` 是编辑器里「只留正在写的东西」那一档的通行叫法（VS Code 的 Zen Mode）；`panorama` 在仓库里没有别的意思。`layer` 已是世界层、对话层、边缘层的名字，档说的是世界层画多少，是另一个概念。
- **被击败的备选**：`focus`／`blend`／`survey`；第三档叫 `overview`——`route.ts` 已把 `#/overview` 读作城页。
- **重开参数**：`zen` 或 `panorama` 在仓库里有了第二个意思。

### 12-18 发送与停止是一个键的两面

- **决策**（由人选定）：对话框只有一个动作键，朝上的一面由「眼前有没有 run」与「框里有没有字」判定（7I）。
- **理由**：发与停从来不同时有意思——框里有字时这一按是说话（run 在跑就是 steer），框空而有 run 时这一按只能是停。两个并排的键让慌乱中的一按有一半机会落错；一个键在手下永远只做脸上写着的那件事，手不用换地方，翻面的动画让人看见它刚换了意思。
- **被击败的备选**：独立的停止键，run 在跑时出现在发送键旁边。它让「停」不依赖框空不空；这一点由 Accel-. 承担，它在任何时候都停眼前的 run（12-16），框里写着半句也一样。
- **重开参数**：出现框里有字时也常要一键停的用法，例如边写下一句边看着 run 跑偏。

### 12-19 其余页面由设置树到达

- **决策**（由人选定）：城、楼、记录、成本、登记簿、MCP 与性能这些页没有常驻入口，从设置面的设置树到达，树最多三级（7L）。
- **理由**：对话页常驻的只有工作内容、参数与事实（4-8）。一列八个页面字形是每天扫视的成本，换来的是一周几次的到达；这些页的到达是一次查找，查找该有分组——城的事实、接入、运行、偏好、诊断各是一类，树说得出这几类，平铺的一列说不出。
- **被击败的备选**：外壳原先那一列页面字形，三态展开；只靠 Ctrl-K——键盘之外到不了，触屏没有路。
- **重开参数**：某一页成了每天多次打开的页；那时它该在对话页或世界层里得到一个位置。

### 12-20 缓动按值抄自 Open Props，时长取 Tailwind 的命名空间

- **决策**：`--ease-arrive`／`--ease-leave` 是 Open Props 的 `--ease-out-4`／`--ease-in-4` 与 `--ease-spring-1`，按值写进 `theme.css`，出处与版本写在 4-43；三档时长拼作 `--transition-duration-short|panel|page`，类名 `duration-short|panel|page`；原先的第三条曲线 `--ease-standard` 删除，它的每个读者按方向改读 arrive 或 leave。
- **理由**：Open Props 是 refrain 1-3 筛过的候选里唯一形状相符的一类，而它与 `theme.css` 不重叠的只有缓动这一片；为二十行 CSS 装一个包，`RUNTIME` 要多一项、`xtask motion` 要认第二个家，而这些值装进来之后也不会再变。时长取 `--transition-duration-*` 是因为 Tailwind 的 `duration-*` 类只在这个命名空间里找令牌：路线图写的 `--duration-*` 拼法让 `duration-short` 解析不出任何 CSS，又不报错。`ease-standard` 回答的是「在原地换状态」，而 Q1 已经说了到达一律减速，它是同一个答案的第二个家。
- **被击败的备选**：一，装 `open-props` 只导入 `props.easing.css`——它把四十多条曲线写进 `:where(html)`，页面多出四十个没有读者的名字；二，留 `--ease-standard` 给悬停与按下——三条曲线让每个写过渡的人多一次选择，而 Q1 的读法里那一次选择的答案永远是 arrive；三，每个读者照旧写自己的毫秒数——同一种位移今天有 90、100、120、150、200 ms 五个答案。
- **重开参数**：出现一个只含缓动与动画、可以单独导入、愿意被 `xtask motion` 当作第二个家的库（refrain 1-3）；或 Tailwind 换掉 `--transition-duration-*` 这个命名空间。
### 12-21 线程里一次调用是一行，整行打开右侧；不折叠，不在线程里画参数与输出

- **决策**：对话线程把每次工具调用画成一行（种类、主体、用时、结果），排在那个回合的正文之下；整行是打开右侧检视面的按钮（4-44、7-11）。调用的参数与输出不再画在线程里，回合的调用也不再收进一个按类计数的折叠摘要。
- **理由**：一次调用的完整内容属于右侧，那里按调用组织（refrain Q2、§3-4）；线程只说工作的形状，所以一行要说出种类、对象、用了多久、成没成，而这四样一眼读完。按类计数的摘要（「读了 7 个文件，跑了 4 条命令」）要一张从工具名到类别的表，那张表是 `kernel::ToolMeta` 的替身（4-26）；一行一调用直接读每次调用自带的 `render` 与 `effect`，客户端不再需要类别表。行在跑时画计时器、落账后画账本的毫秒（refrain U12），折叠起来的调用看不到这两样。
- **被击败的备选**：一，保留折叠、展开后列出调用并内联参数与输出：同一份输出在线程与右侧各画一次，一个事实两个家，长输出还把正文推出视口。二，折叠摘要加逐行列表两层：多一次点击才看得到计时器，而在跑的那一行正是人最想看的。三，每个回合只画最后一次调用：跳过的行让 ↑／↓ 走不到它们，右侧也就打不开它们。
- **重开参数**：一个回合的调用常常多到把正文推出一屏（例如一次读几十个文件）时，按回合给超过某个行数的调用加一个「其余 n 条」的收起，收起的行仍可由 ↑／↓ 与右侧到达。
### 12-22 检视面的 diff 读两个检查点之间的 `Hunks`，不读 edit 结果里自带的 diff

- **决策**：一次改文件的调用在检视面里画成它前后两个检查点之间这个文件的 `Query::Hunks`（4-45），不解析 edit 工具写进结果的 `{ path, base_version, new_version, diff }`。
- **理由**：两个提交 id 永不改变，答复可以一直留着；行号属于城持有的一棵树，所以「路径:行」与删去行的「路径@旧提交:行」（7-2）都指向一个人能再打开的地方；工作树有没有离开那棵树由一次 `Changes { base: 后一个检查点, head: null }` 判定，`reachOf` 因此知道该给链接还是给可复制的位置（4-39）。结果里的 diff 受 `Output` 的裁剪约束，长的改动会被切掉，而它的旧侧是一个 B3 版本，不是一个提交。
- **代价**：检查点跟在一波之后，所以 diff 覆盖调用所在的整波，同一波里改同一个文件的另一次调用也在里面；那一波还没有检查点时 diff 画不出来，只说「下一个检查点后」。
- **被击败的备选**：解析结果里的 diff（`monitor/trace.ts` 已这样读，监视器要的正是每次调用自己的改动）：精确到这一次调用、检查点之前就有，但被裁时不完整，行号对着一个不在任何提交里的版本。
- **重开参数**：城按调用写检查点，或 `Call` 带上这次调用自己的 `Hunks` 定位（两端是内容库里的版本而不是提交）时，diff 改读这一次调用自己的改动。
### 12-23 RefRain 的编辑器是 CodeMirror 6 的最小组合

- **决策**：RefRain（7N）的编辑区是 CodeMirror 6，只取五个包：`@codemirror/state`（文档与改动集）、`@codemirror/view`（视图、输入法、选区）、`@codemirror/commands`（撤销历史与键表）、`@codemirror/search`（查找与替换）、`@codemirror/merge`（两段文本之间的 diff 与对照）。不取语言包与 `basicSetup`。五个包只由 `views/refrain/` 读，整块懒加载：第一次打开一份文档时才下载。编辑器的颜色写在 `theme.css` 末尾 RefRain 的那一块，盖过 CodeMirror 自带的基础主题，所以颜色仍只有一个家；编辑器自带的字句（查找面板、对照的提示）经 `EditorState.phrases` 取自 `lang.json`。
- **理由**：4-46 要的四件事平台给不了：一份可编辑、按视口画的长文本（一个 4 MiB 的 `<textarea>` 在输入时整份重排）；一个能精确说出「从基线到此刻改了哪几段」的改动集（`<textarea>` 只有整份的值，改动要事后比对，而比对给出的区间不一定是人做的那一下）；不被程序改动打断的撤销栈（给 `<textarea>` 赋值会清掉浏览器的撤销）；在长文本里查找与替换（浏览器的查找不进 `<textarea>`，也不能替换）。CodeMirror 的改动集按 UTF-16 位置给出、带 `mapPos`，正是 `core/document_pos.ts` 换算的另一头；它的输入法处理在三家引擎上有自己的测试。它替换的是本客户端原本要手写的这一套编辑面与一张行级 diff（`@codemirror/merge` 的 `diff` 同时给「移到现版」用，4-46）。许可证都是 MIT，在 `deny.toml` 的清单上。
- **被击败的备选**：①`<textarea>`——上面四件事各缺一件；②`contenteditable` 加自写的模型——输入法、选区与撤销要自己在三家引擎上重做一遍，那正是 CodeMirror 已经做完的；③Monaco——体积大一个数量级，要 worker，按 refrain 路线图附录 F 落选；④ProseMirror——富文本的文档模型，Markdown 要先解析成树再序列化回去，源文字节保不住（A1）。
- **读数**：五个包与它们带进来的 `@codemirror/language`、`@lezer/common`、`@lezer/lr`、`style-mod`、`w3c-keyname`、`crelt` 在一块懒加载的分块里，首屏不付；`frontend_artifact` 称整个 dist，引入前后的读数由整合记进 `tools/xtask/budgets.toml`。
- **重开参数**：Markdown 源码要语法着色时，加 `@codemirror/lang-markdown` 还是复用 `parts/paint.ts` 的 lezer 块，先过 7-9；或者出现一个同样给出改动集与输入法保证、体积小一半的编辑器。
### 12-24 工作台的栏宽以栏计，存在浏览器，不入城

- **决策**：检阅档工作台的三栏顺序与宽度是 `core/workbench.ts` 的 `Workbench`——三栏各占几栏、从左到右——由 `prefs.ts` 的 `workbench`／`setWorkbench` 存在这个浏览器的 `sprawling.workbench` 一行里；拖动吸附到栏线，键盘一次一栏，三栏之和恒为 12，每栏不窄于两栏（`NARROWEST`）。`setWorkbench` 不发 `PutPreferences`。
- **理由**：外壳是一张 12 栏网格，「两条边在一条线上」由构造保证（4-33）；一个以像素或 `fr` 存的宽度会让栏边落在两条栏线之间，世界层的栏边、对话条与右侧就不再共用栏线，而这正是要靠构造而不是测量来守住的东西。存在浏览器而不入城：栏宽是这块屏幕的事实——同一个人在笔记本与外接屏上要的宽度不同——而城的偏好记录说的是这个人，`PreferencePatch` 也没有这一臂。
- **被击败的备选**：一，像素宽度加最小像素（多数 IDE 的分栏器）：手感连续，但栏边脱离网格。二，`fr` 份额：随窗口缩放，但份额同样不落在栏线上。三，随偏好入城：要给 `PreferencePatch` 加一臂、让 `WIRE_V` 进位，换来的是在另一块屏幕上打开一个不合那块屏幕的排布。
- **重开参数**：外壳不再是 12 栏网格；或人要求工作台排布跟着人走、跨浏览器一致（那时给 `PreferencePatch` 加 `workbench` 一臂，`setWorkbench` 像 `setPanel` 一样出城）。
