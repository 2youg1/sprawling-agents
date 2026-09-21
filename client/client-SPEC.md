# client-SPEC — the browser client (Solid + Effect, outside the cargo workspace)

> 权威顺序同 AGENTS.md：人的决定 → ARCHITECTURE.md → 本文件 → 代码与测试。本文件记接口与设计；视图层（`src/*.tsx`、`src/theme.css`）按 `docs/frontend-method.md` 免 SPEC 与红绿，效应核（`src/core/`）不免。
>
> **免的是画法，不是键盘。** 一个部件遵循哪个 WAI-ARIA 模式、每个键做什么、焦点还给谁、`aria-*` 取什么值，是这一层对使用者的承诺，不是一次视觉迭代；`docs/frontend-method.md` 的豁免因此只覆盖排布、间距、色调与动效，`views/parts/` 的交互契约由 §7 独家规定。

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
| typescript（名下） | 6.0.3 | typescript-eslint 的 JS 编译器 API（见设计 4-1） |
| typescript-native（别名 `npm:typescript@7.0.2`） | 7.0.2 | `bun run typecheck`（Go 版编译器） |
| eslint / @eslint/js / typescript-eslint / eslint-plugin-solid | 10.10.0 / 10.0.1 / 8.70.0 / 0.18.0 | lint |
| @types/bun | 1.4.2 | `bun:test` 的类型，仅测试文件用 |

脚本：`dev`、`build`（Vite，`base: './'`）、`typecheck`、`lint`（`eslint --max-warnings 0`：警告即红）、`test`（`bun test --conditions=browser`，见设计 4-2；因此 justfile 的 `check-client` 写 `bun run test` 而不是 `bun test`）。`src/vite-env.d.ts` 只引 `vite/client` 的类型，让 `import "./theme.css"` 过 TS 7 的副作用导入检查。

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

## 4 设计

- **4-1 TypeScript 7.0.2 与 typescript-eslint 8.70.0 不能共用一个 `typescript` 名。** 7.0.2 是 Go 版编译器，包的 `.` 导出只有 `{version, versionMajorMinor}`，没有 JS 编译器 API；typescript-eslint 的 peer 范围是 `>=4.8.4 <6.1.0`，实测 `require('typescript')` 拿不到 `createProgram`。取法：`typescript` 名给 6.0.3（该 API 的最后一条线），7.0.2 以别名 `typescript-native` 安装并承担 `typecheck`。否决「只装 7.0.2、放弃类型感知 lint」：禁 `any`／`as`／非穷尽 switch 都是类型感知规则，没有它们守卫就不存在。否决「只装 6.0.3」：违背「新引入依赖钉最新精确版」且放弃原生编译器的速度。代价：两个编译器读同一份 tsconfig，lint 用 6.0.3 的类型信息，typecheck 用 7.0.2；二者分歧时以 typecheck 为准。**升 `typescript` 这个名要先改本条，不是先改版本号。**
- **4-2 `bun test` 必须带 `--conditions=browser`。** `solid-js` 的 exports 在 `node` 条件下给 `dist/server.js`，其中 `createEffect` 不跑；`bunfig.toml` 没有能改条件的键（实测 `[test] conditions`、`[run] conditions`、顶层 `conditions` 皆无效）。否决「测试只用不依赖响应性的 API」：效应核的全部价值就是响应性。
- **4-3 hash 路由，不用 path 路由。** 片段不发给服务端，书签、后退、深链成立，且不动 `ClientAssets::lookup` 那道安全判定。
- **4-4 `Address`／`RunId` 用 Effect `Brand.refined`，不用 `as`。** Brand 的构造器提供 `.option()`，非法值答 `None`，与 Rust 的 `Result` 同形；`as` 全库禁用（`as const` 除外），所以「新类型」只能由构造器产出。
- **4-5 eslint 配置用 `tseslint.config()` 而不是 ESLint 的 `defineConfig()`。** eslint-plugin-solid 经 `@typescript-eslint/utils` 给 plugin 定型，其 `RuleContext` 仍声明 ESLint 10 已删的成员（`getAncestors`、`parserPath` 等），故该 plugin 对象不可赋给 ESLint 自己的 `Plugin` 类型，`defineConfig` 在 typecheck 下红。`tseslint.config` 是 typescript-eslint 为这道缝留的定型桥，已标 deprecated；配置文件上用一条带理由的 `eslint-disable-next-line @typescript-eslint/no-deprecated`，并开 `reportUnusedDisableDirectives: "error"`——上游修好、桥不再需要的那天，这条指令变「未用」即红，与 Rust 的 `#[expect(reason)]` 同义。否决「把配置文件排除在 tsc 与 lint 之外」：那会让全库唯一不受 `as` 禁令保护的文件恰好是定义禁令的文件。
- **4-6 Effect 只做 wire 解码。** `core/frames.ts` 用生成的 `Schema` 读每一帧（`Schema.parseJson(ServerFrame)`），这是 Effect 在运行时唯一出现的地方。理由：`Link` 是一个纯状态机，用 Stream／Fiber 包它买不到任何东西，却让每个视图多一层范式。
- **4-7 首屏即对话。** `#/` ＝ 与 `hall/mayor` 的对话；同一房间的每次 dispatch 是一段线程；live 时 Enter 是 `steer`，冻结后 Enter 是新的 `dispatch { addr: room, session: null }`（`room_for` 对含 `/` 的地址不再开子房间）。等人的事以卡片插进对话流，不另开一页。
- **4-8 按钮全在左栏。** `views/rail.tsx` 收起时只有字形，展开（hover／`[`／`?`）才出现名字与快捷键——`g m`／`g c`／`g s`／`g w`／`g r`／`g $`、Ctrl-K。页面其余部分没有按钮。
- **4-9 两层可视化。** `#/city` 是 SVG 画的城；`#/building/<addr>` 是目录树（`Query::Listing` 逐层）＋文件原文（`Query::Document`）＋计划表＋提交列表；`#/run/<id>` 四透镜。
- **4-10 页面上没有句子，但零数据的屏必须说下一步。** `lang.json` 全是标签，说明段不写。**动词一律拼成命令**：`city_stop` ＝ `/stop --all`、`bld_halt` ＝ `/stop {addr}`、`talk_stop`／`run_cancel` ＝ `/stop`，`release` 同形；两语同一拼写。理由：人会从别的软件迁移用法，一条命令的拼写自己说明自己。**原条禁「图例」与「空态提示」，这一句在 2026-09 被两次实测推翻，故在此改写而不是在代码里绕过**：`city/bar.tsx` 的图例是五个字形唯一的名字，去掉它城就是一张没人读得懂的画；一个零数据的屏只画一个灰词时，人分不清这屏是空的还是坏的。因此**空态一律走 `parts/empty.tsx`**——一个形状、一句说缺什么的话、一个离开这个状态的动作，动作能省而那句话不能。这不放宽「不写说明段」：空态那句话说的是这一屏此刻没有什么，不是这一屏是干什么的。
- **4-11 性能纪律。** 帧按动画帧合并（`socket.ts` 的 `queue` ＋ `requestAnimationFrame`），事件折叠 O(1)，同一查询 250 ms 内合并（`asking.ts` 的 `PACE_MS`），stale-while-revalidate，动画只用 `transform`／`opacity`。
- **4-12 Solid 里画 SVG 的一条规则。** 组件内部的 SVG 元素只能用编译器按名字认得的标签（`g`／`path`／`rect`／`circle`／`text`／`line`），链接用 `<g role="link">` 加事件，不用 `<a>`——`<a>` 会被建成 HTML 元素，其 SVG 子树不渲染。`<defs>` 只在最外层组件里写。
- **4-13 composer 说出消息落点。** `core/belief.ts` 的 `Sending = "dispatch" | "steer" | "queued"` 与纯函数 `sendingInto(doing)`：`frozen` 与无 run → `dispatch`，`thinking` → `steer`，`calling`／`waiting` → `queued`；拼写 `/dispatch`／`/steer`／`/steer --queued`。理由：steer 在相位边界被消费，工具调用期间 run 在系统调用里，「发出去了」与「被听见了」不是一个时刻。**零 wire 变更**：信息全在 belief 里。**抖动缓冲不做**：抖动多大是一个未测量的量，为一个未测量的量先建队列是这座城禁的那条。
- **4-14 从 diff 到 session 是一次路由跳转**，不新增命令帧、不加「继续」按钮。`dispatch{addr: room, session: null}` 与在该房间对话页按 Enter 是同一个动作，加按钮就是给一个已有机制起第二个名字。
- **4-15 提交列表按页问、按页存。** `core/asking.ts` 的 `COMMITS_PAGE = 40` 与 `commitsQuery(building, before)`——问题只有一种拼写，因为 `CommitsAnswer` 回带 `building` 与 `before` 而不带 `limit`，键若拼法不一，答案永远落不回槽里。每页是一个独立的问题：只有 `before: null` 的首页会因 `checkpoint_committed`／`pr_merged` 失效重问；旧页上界是已写下的 seq，且 `lineage` 从写提交的 run 向前走，后来的接替者改不了它。
- **4-16 转写结果落进输入框，不直接发出去。** 机器听错的那一句必须能改，否则它会花掉一次 run。没有为 `transcribe` 选过模型的城不画那个按钮：一个只可能答拒绝的控件，是一个没人该遇见的控件。
- **4-17 焦点环只有一个家，所以 `outline-none` 是删掉而不是换掉。** `theme.css` 的 base 层写着 `*:focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px }`，而 utilities 层优先级更高：26 处 `outline-none` 把它吃掉，键盘用户在设置页看不见自己在哪。做法是把这 26 处全部删除、不补任何替代类——`:focus-visible` 本来就只在键盘取焦时匹配（文本框被点击时也匹配，因为它确实要收键盘输入，这是对的）。否决「改写成 `focus:outline-hidden focus-visible:outline-2`」：Tailwind v4 的 `outline-hidden` 设 `--tw-outline-style: none`，而 `outline-2` 展开成 `outline-style: var(--tw-outline-style); outline-width: 2px`，两条规则在 `:focus-visible` 时同时命中，算出来是 `outline-style: none`——那对组合会删掉它本想保住的那个环。**任何一处需要压制焦点环的地方写 `outline-hidden` 而不是 `outline-none`**：v4 把「画 2px 透明描边、在强制色模式下仍可见」这层语义改名到了 `outline-hidden`，照旧写 `outline-none` 会在 Windows 高对比下丢掉焦点环。闸门条件是 `client/src` 里 `outline-none` 出现 0 次。
- **4-18 `parts/tip.tsx` 的提示有两条定位路，调用方说关系。** `title` 对三种读者都失效：指针要悬停约一秒，键盘根本到不了，触屏永远不画。`Tip` 用 `:hover` 与 `:focus-within` 显示一个 `role="tooltip"` 的兄弟节点，`transition-delay: 300ms` 挡住指针扫过一行时的连环点亮，未显示时是 `display: none`——既不画也不被 `xtask render` 量到。**关系由调用方写**：控件自己有名字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`；组件不猜，因为只有调用点知道控件有没有可见文字。**定位两条路都要在**：`@supports (anchor-name: --a)` 内用 `position-area` 锚定并 `fixed`，脱开一切裁剪祖先；Safari 与 Firefox 今天不支持，落到对 wrapper 的 `absolute` 分支（`AGAINST_WRAPPER`）。只写前一条是静默失效：弹层退回静态位置，可能溢出视口（§9.0 第 1 行）。锚点名按实例生成，经继承的自定义属性 `--tip-anchor` 传给提示节点，所以同一行上的两个提示各锚各的控件。
- **4-19 `Field` 管有标签的表单格，不管搜索框与命令框。** 23 处裸 `<input class="rounded-control bg-g2 …">` 里，供应方表单、模型表的两个上限格、键名值对与登录码改走 `Field`，因而一次拿到标签、说明、错误态、`aria-describedby` 与 `:user-invalid`（失焦后才红，不是每次按键）。命令面板、combobox 过滤框、composer、楼页目标框与记录搜索框**不改**：它们要 `ref`、`autofocus`、逐键的 `onKeyDown` 与自定义补全，塞进 `Field` 会把它变成十个透传参数的 `<input>` 壳子，正是 AGENTS.md 禁的那种壳。`Figure`（原 `providers.tsx`）删除，三个调优数字改 `Field kind="number" step=…`，键盘上下键因此能用；**不给 `ms` 后缀**——标签本身就是 Codex 的 `timeout_ms` 与 `stream_idle_timeout_ms`，再加一个后缀就是同一个单位的第二个家。
- **4-20 模态是平台的事。** `parts/dialog.tsx` 是原生 `<dialog>` 加 `showModal()`：top layer、焦点陷阱、Esc、其余页面 `inert`，四件都由引擎承担，文件里因此没有遮罩层、没有 keydown、没有取焦调用，也没有 z-index——top layer 之上排不进任何数字。取焦由文档顺序决定：平台取对话框里第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。Esc 到达时先 `preventDefault` 再回调 `onCancel`，否则默认行为绕过调用方关掉元素，`open` 还说着「开着」。开与合是同一条 `transition` 的两个读法，`display` 与 `overlay` 是离散属性，少了 `transition-discrete` 关门第一帧盒子就消失、连同它的淡出。**遮罩是 `::backdrop` 上的 `backdrop-filter: brightness()`，不是一层 `bg-g0/80`**：`::backdrop` 只在实现了「从原始元素继承」的引擎里读得到页面的颜色令牌，令牌解析不出时 `background-color` 回到初始值——一扇没有遮罩的模态，且没有任何一道闸会报出来；亮度滤镜不向平台要颜色，深浅两套灯光下都变暗，Tailwind 又把 `--tw-backdrop-*` 直接声明在 `::backdrop` 上，所以这条路不经过继承。
- **4-21 `views/parts/` 不写 z-index。** 定位过的盒子在未定位的盒子之后绘制：下拉、弹层、粘性表头与提示压住它们打开时盖住的那些行，这是绘制顺序本来就给的，不需要一个数字。给了数字反而要维护一张谁比谁大的表，而那张表没有家。唯一还留着数字的部件是 `parts/kbd.tsx` 的快捷键表：它必须盖住左栏，而左栏是全客户端允许保留的那一个 `z-10`，所以只有 top layer 能让它交出数字（见 4-23 的第二个参数）。
- **4-22 按钮的三态是一个 `data-state`。** `idle`／`loading`／`stopped` 由一处判定给出，属性本身、`aria-disabled`、`aria-busy` 与点击闸是它的四个读者。色调表只写静息态与静息态的 hover，`not-data-[state=idle]` 一条规则管住其余两态——「正在等回执」与「你按不了」对手的回答是同一句，不该有两套写法；hover 写进 `data-[state=idle]:` 里，因而一个不能按的控件在指针下不会亮起来假装这一按会落地。`background-color` 进了 transition 列表，hover 因此是到达而不是切换。
- **4-23 Popover API 今天进不了 `parts/popover.tsx`，两个参数卡着它。** 其一：`popover` 元素开着时在 top layer，包含块是视口，`absolute` 不再相对 composer 的 `<form>` 解算，而把弹层钉回 composer 的唯一机制是 CSS anchor positioning，本项目的第一浏览器今天没有（§9.0 第 1 行）——所以「不支持 anchor 时走今天的 `absolute` 分支」这条降级对一个 popover 不成立，它只在元素还留在流里时成立。其二：`#/gallery` 的两个弹层夹具靠一个 `relative` 祖先与两层 padding 把面板留在自己的 Case 里，元素一进 top layer 就逸出，`xtask render` 的「没有盒子画在容纳它的盒子之外」立刻红。两个参数任一移动都应重新论证本条；在那之前弹层是 `absolute`，Esc 与取回焦点由这个文件自己管。
- **4-24 后端已经答了的，客户端必须问，问到了必须画。** 一个城答得出而没人问的 `Query`，与一个上了 wire 而没有屏幕读的字段，是同一个缺陷的两半：它们让「这件事做完了」在两侧各有一个说法。三处落点：**其一**，城页顶栏的六个数字全部出自 `Query::Metrics` 一问——事件、在干活的 run、已收尾的 run、等人批的、排队的信号、回收站里的。`MetricsAnswer` 的第七个字段 `buildings` 故意不画：顶栏下面的天际线与旁边的楼列表本身就是楼的数目，再写一个数字就是同一个事实的第二个家。原先的状态徽标一并去掉——它说的 `runs_active` 就是那六个里的一个，而「城停了」由外壳的横幅在每一页说一次（`app.tsx`），顶栏只留那一个停/放的控件。**其二**，`Query::RegistryView` 得到 `views/registry.tsx` 一屏，四列（登记于、类别、所属楼、是什么），走 `parts/table.tsx` 因而每列可排序，默认最新在上。**其三**，run 页直接问 `Query::RunView` 而不再只靠流折出来的 belief：从别人发来的链接打开 `#/run/<id>` 的那一页没见过任何记录，`city_view` 又只列它还在列的 run，所以这条 run 属于哪个房间、是否已经结束，恰好在最需要的那一类 run 上是空的。**两个读法按 seq 判定**：流说的是此刻，摘要说的是城写下来的，两边都带账本位置，所以谁更新是一次比较而不是一次偏好。
- **4-25 base URL 的形状只拼写一次。** `setup/providers.tsx` 的 `BASE_URL` 同时喂两个读者：框子自己的 `pattern`（浏览器在人还在填表时判，失焦后才红）与 `hostOf`（「看看」和「接上」两个控件的开关）。**`type="url"` 不是那条规则**——它收 `mailto:somebody` 和任何别的 scheme，于是一个这张表单会拒的值可以坐在一个浏览器称为合法的框里，而人是按下一个始终发灰、不说为什么的控件才知道的。形状是：scheme、ASCII 主机、可选端口、可选路径，之后什么都不许有；查询串拒掉，因为供应方陈述的是 API 的根，每个 face 自己在后面挂路径。**客户端只做最宽的判断，永不比城更严。** 归一化今天有调用者了：`assembly::credentials::Entered::resolved` 是打字地址变成被调用地址的唯一一处，probe 与 attach 都经过它（叶 1.2）。`gateway::normalise` 的第三条规则把缺席的 scheme 读成 `https://`，运行这座城的机器地址读成 `http://`，所以 scheme 在这张表单里是可选的——一个要求写 scheme 的框会拒掉厂商文档印出来的 `api.openai.com/v1` 与 `127.0.0.1:11434`，而城收得下。这张表单因此只判「有没有一个能调用的主机」，scheme 由城补，路径由城按预设表补。

- **4-26 「这次调用算哪一类」在客户端只有一张表，而那张表是替身。** `views/talk/trace.ts` 把工具名读成 `Deed`（`explored`／`wrote`／`ran`／`other`），对话页的折叠摘要与制品面板两个读者都问它，所以客户端内部没有第二张表。**权威不在这里**：`kernel::ToolMeta` 为每个注册工具声明了 `effect`（`Read`／`Write`／`Egress`／`Connector`／`Spawn`／`Govern`／`Spend`）与 `render`（`Generic`／`Terminal`／`Diff { locations }`），`runtime/src/tools/exec.rs:99` 的 `RenderIntent::Terminal` 与 `edit.rs:101` 的 `RenderIntent::Diff` 正是制品面板要分的那两半；两个字段今天都不上 wire（`RoundsAnswer` 的 `Call` 只有 `tool`／`subject`／`arguments`／`outcome`／`at`／`output`）。**迁移一次做完**：`Call` 增补 `effect` 与 `render`、`WIRE_V` 随之进位、`trace.ts` 的 `DEEDS` 与 `deedOf` 删除、两个读者改问新字段。在那之前每加一个工具，这张表与工具注册处会各自演化一次，而只有注册处是对的。`parts/code.tsx` 的行号从 1 起算同属这笔债：`Output` 只带 `head` 与 `cut`，不带这段头部在文件里从第几行开始，所以「第 27 行」今天指的是这段输出的第 27 行。
- **4-27 对话页的第二栏由账本决定要不要画，由容器宽度决定画在哪。** `talk.tsx` 问的 `Query::Rounds` 与 `Thread` 问的是同一句，`core/asking.ts` 按内容合并，因此「现在显示的是哪个 run」只有一个答案；面板不占路由，理由是一条新路由会让这个答案有第二处判定，而两处判定第一次分歧就发生在有人打开别人发来的链接时。**断点问 `main`（具名容器 `page`）而不是问窗口**：左栏钉开时吃掉 232px，按窗口宽算会在 1440px 把两栏挤坏，所以 `@lg/page:`（820px）转成两栏、`@wide/page:`（1120px）让面板长到 `max-w-measure`。**开合状态走 `core/prefs.ts` 的 `panel`，不在 `talk.tsx` 里开第二扇门**：它是人的偏好，归宿是 `~/.sprawling/config.toml` 的 `[ui]`（C 章 3.1），今天由那扇门后面的浏览器缓存记住，城答上来的那天由 `adopt` 顶掉（4-29）。

- **4-28 强度只有一个权威，客户端不留副本。** 城层 `CONFIG.toml` 的 `[model] effort` 经配置梯子冻结进每一次 run（`crates/city/src/config_layers.rs`），所以浏览器里不再存 `sprawling.effort` 这一行。**缺席不是 `"medium"`，缺席是不说**：帧里不写这个字段，城的文件回答；文件也没写时供应方回答，而 `Effort::None` 是「尽量不要想」，是另一件事。一次派活仍可为它开的那一场单独说一个档位（`Dispatch.effort`，城把它写进房间层，city-SPEC 8-14），所以选择器只在 composer 上——那里的作用域与控件的位置一致。**设置页与欢迎页因此不再有强度选择器**：它们承诺的是「从此以后」，而这一版既读不到 `Query::Config`（路线图 3.3）也没有写城层强度的命令帧，一个刷新就忘的设置控件是第二个权威的开始。六句 `effort_note_*` 随选择器搬到 composer 的那一列，做每一格的 `hint`，人在决定的那一刻读到它。
- **4-29 偏好有两层，城赢。** 权威是人层 `~/.sprawling/config.toml`（C 章 3.1），浏览器存储是它前面的缓存：缓存只负责首帧不闪，`adopt` 一到就整条顶掉它。`core/prefs.ts` 独家拥有每一个存储键的拼写（`ROWS`）与读写，`core/rows.ts` 独家拥有那次对 `localStorage` 的触碰；`keys.ts` 问 `chord(action)`／`setChord`，外观屏问 `held().appearance`／`setAppearance`，网络屏问 `held().proxying`，没有第二处拼一个键名。

  **一进一出两个方向，形状因此不同。** 出城的方向是五个具名改动（`setLang`／`setWelcomed`／`setPanel`／`setAppearance`／`setProxying`），每个将来各自变成一条 `Command::PutPreferences { patch }`——`channels::PreferencePatch` 的六个变体就是这个形状，交出整条记录的调用点届时要重写而具名改动不必。进城的方向是 `adopt(stated)` 一整条：一次回答陈述每一个值，按字段贴回去会贴出半新半旧的一条。`keeper()` 说此刻是哪一层在保管（`"browser"`／`"city"`），设置页把它画出来（`setup/kept.tsx`），因为「清掉浏览器数据会不会丢」是人有权知道的事。

  **`readPreferences` 与 `writePreferences` 互为逆，这才使缓存是缓存**：城上次答的就是下一次首帧画的。**快捷键不在这条记录里**：`PreferencesAnswer.chords` 是一张表，而 `Rows` 这道缝故意不能枚举（谁写的谁读），能列出全部覆写的只有 `keys.ts` 的 `ACTIONS`——所以 chord 今天仍按动作名逐行读写，`wire.ts` 重生后随答案一起进记录，那次改动把 `ROWS.chord` 这一族折成记录里的一个字段。
- **4-30 设置页的 `config.toml` 侧栏引用文件，不自己拼。** 原先这一栏用 `[model_providers.<name>]` 拼出一段文字，而城自己的读法只认 `[model]`／`[sandbox]`／`[[mcp]]` 三节——把它抄进 `CONFIG.toml` 的人会拿到一句列出三节的拒绝，一个事实在前端与后端各有一个家且已经分歧。这一版把那段字符串删掉，侧栏只说「这里还读不到城的 config.toml」（`setup_toml_unread`）。填进去的是 `Query::Config { addr }`（路线图 3.3）的答案。**它逐值带层**：`channels::ConfigAnswer` 是 `{ addr, effort: Option<SettledEffort>, tuning: TuningDefaults }`，`SettledEffort` 带 `from: ConfigLayer`，因而侧栏每一行画的是「值 ＋ 它来自哪一层」，`parts/badge.tsx` 画那一层的名字。**今天画不出来**：Rust 侧的 `Query::Config` 已在，`client/src/wire.ts` 是生成物且尚未重生，所以侧栏仍是 `setup_toml_unread` 的空态；重生之后这一栏是唯一的填入点，不需要先退休任何一处手拼。
- **4-31 设置页没有 MCP 组，skills 组是「列表 ＋ 只读源文」。** MCP 组原是一个指向 `#/mcp` 的链接，而左栏已经到得了那一页，所以它是一层什么都不做的中转，删掉。skills 组由 `setup/skills.tsx` 承担三件：放技能的文件夹（欢迎页共用这一件）、楼列（`shared/buildings.tsx`，与 `#/mcp` 同一份）、那栋楼两个书架的清单（`Query::Skills`）与打开一条后的原文（`Query::Document`，走楼页那一个 `FileView`）。**这里没有编辑器，因为城没有那扇门**：library 在保留前缀下，居民可读不可放（`crates/city/src/library.rs`），写门 `Command::PutShelved`（路线图 3.8）还不存在，一个存不下去的 `<textarea>` 会把「改了」说成两件事。
- **4-32 一个状态药丸只有 `parts/badge.tsx` 一个画法。** `building/plan.tsx` 原先手画五种漆色（`done` 灰、`blocked` 实心 alert、`in_progress` 实心 accent、ready 的 `bg-g3`、其余无底色），那是同一件事的第二个家，且那串嵌套三元没有 `awaiting_approval` 的臂——等人批的一行被画成没人开工的一行。现在一个穷尽 `RoadmapStatus` 的 `weightOf(row)` 给出 `quiet`／`live`／`alert` 三档，`Badge` 画它。**两个状态共用一档是对的**：`ready` 与 `in_progress` 都是城在动，`blocked` 与 `awaiting_approval` 都是城停下来等人，而分辨它们的是词，不是颜色（7-1 的 badge 行）。代价是 done 不再比 not_started 更暗；这不是损失，因为那两个词本来就不同，而颜色按 7-1 只许重复词。

## 5 `src/core/`（形状按 ARCHITECTURE §9）

| 文件 | 形状 | 接口 |
|---|---|---|
| `link.ts` | 1 判定 | `newLink(token)`, `connect`, `advance(link, LinkEvent) -> [Link, LinkAction]`；`LinkAction` 穷尽（open／send／welcomed／deliver／answered／saying／wait／report／close）；阶梯 `[250,500,1000,2000,5000,10000]` |
| `frames.ts` | 4 适配器 | `decodeFrame(text) -> ServerFrame \| null`, `encodeFrame(ClientFrame)` |
| `socket.ts` | 4 适配器 | `openConnection(url, token) -> Connection { state, belief, asking, command, retry, dismissRefusal }`；`tokenIn(search)`, `socketUrl(location)`, `bearing(token)`——POST 递配对码的唯一拼写（`Authorization: Bearer`，服务端读者是 `channels::reception::offered_pairing`） |
| `asking.ts` | 1 判定 | `createAsking(send) -> { ask(query) -> Accessor<Answer\|undefined>, refresh, answered, invalidate(record), reconnected }`；答案按内容匹配问题，无名者按到达序；`staleBy` 是事件到查询的失效表 |
| `belief.ts` | 7 投影 | `createBelief() -> { belief: {runs, halted, refusal, city, probed}, adoptCity, apply, say, refused, named }`；`RunBelief { addr, started, task, doing: thinking\|calling\|waiting\|frozen, saying }`；`sendingInto(doing)` |
| `commands.ts` | 2 值 | 每个命令帧一个构造函数，自铸 `IdemKey` |
| `enrol.ts` | 4 适配器 | `enrol(Enrolling { origin, token, realm, name, value, lang }) -> Promise<Enrolment>`；`referenceFor(provider)`。**引用来自城**：201 正文是 `kernel::SecretRef` 读回后写出的那一句，本页不自己拼一份存起来（M-22） |
| `speaking.ts` | 4 适配器 | `canRecord()`, `record(origin, token) -> Promise<Recording \| null>`；`Recording.stop() -> Promise<Heard>`，`Heard` 穷尽（text／refused／silent） |
| `idem.ts` | 2 值 | `mintIdem()` |
| `mark.ts` | 4 适配器 | `paintMark(document, quiet \| live \| waiting)`：把令牌解算成引擎实际会画的颜色，拼成 SVG data URL 写进 `<link rel="icon">`；零颜色字面量 |
| `rows.ts` | 4 适配器 | `Rows { getItem, setItem, removeItem }`、`memory()`、`browserRows()`：浏览器存储那一扇门，三处会抛的拒绝（禁用存储、配额为零、写时配额满）在这里各变成一个值 |
| `prefs.ts` | 6 数据 | `Preferences { lang, welcomed, panel, appearance, proxying }`、`Keeper = "browser" \| "city"`、`PreferenceDoor { held, keeper, adopt, setLang, setWelcomed, setPanel, setAppearance, setProxying, chord(action), setChord, draft(at), setDraft }`、`loadPreferences(rows, browserLang)`、`preferences()`；**全客户端每一个存储键的拼写都只在这个文件的 `ROWS` 里**（草稿键 `sprawling.draft.<房间或 run>`、快捷键 `sprawling.key.<action>`） |
| `prose.ts` | 1 判定 | `blocks(text) -> Block[]`, `inline(text) -> Inline[]`：Markdown 读成数据，永不 innerHTML |
| `route.ts` | 1 判定 | 见 §3-2 |
| `time.ts` | 1 判定 | `ago`, `clock`, `count`, `usd`, `kib` |

`src/ui.tsx` 是视图拿到的一切：`UiProvider`／`useUi`（conn、prefs、effort、chooseEffort、bar、origin、pairing、now；`pairing` 是开这一页的地址栏上的配对码，两扇会动作的 HTTP 门要它）、`useSay`（键→词，填槽）、`useGo`、`useCommand`、`useHearing`。

## 6 视图（免 SPEC，列出以便定位）

`views/parts/tip.tsx` 提示（见设计 4-18）；`views/parts/code.tsx` 只读代码视图（面包屑＋行号＋词法着色，见设计 4-26）；`views/rail.tsx` 左栏；`views/talk.tsx` ＋ `talk/{thread,calls,composer,waiting}.tsx` 对话 ＋ `talk/artifact.tsx` 制品面板 ＋ `talk/trace.ts`（工具调用的分类，两个读者共用，见设计 4-26）；`views/city.tsx` ＋ `city/{bar,panel,skyline,marks}.tsx` ＋ `city/shape.ts`（超椭圆路径）；`views/registry.tsx`（`Query::RegistryView`：这座城决定留下来的东西，一行一件，见设计 4-24）；`views/building.tsx` ＋ `building/{tree,commits}.tsx`；`views/changes.tsx`（`Changes`／`Hunks` 的一份读法，run 页与楼页共用）；`views/run.tsx`；`views/setup.tsx` ＋ `setup/{providers,models,skills,appearance,keys}.tsx`（skills 组见设计 4-31）＋ `setup/kept.tsx`（一个组的答案由谁保管，见设计 4-29）；`views/shared/{provider,effort,buildings}.tsx`（欢迎页与设置页共用的三件，`buildings.tsx` 的第二个座位是 `#/mcp`）；`views/machine.tsx`（doctor 的答）；`views/desktop.tsx`（一栋楼的桌面白名单）；`views/mcp.tsx`；`views/welcome.tsx`；`views/record.tsx`；`views/cost.tsx`；`views/palette.tsx`；`views/refusal.tsx`；`views/prose.tsx`；`views/gallery.tsx`。

**`#/gallery` 是一条路由而不是一个构建开关**，因为量它的那道门应当打开一个人真正跑的 bundle；夹具不需要城（偏好走 `core/rows.ts` 那扇门，没有 localStorage 时是一张只活一次会话的表）。每个能进入多种状态的屏幕在那里各有一份夹具，`cargo xtask render` 打开真引擎读它。

**左栏是覆盖而不是推挤**：外层 `<div>` 只在钉开（`[`）时取 `w-rail-open`，`<nav>` 绝对定位、hover 时自宽并加投影；正文的左边因此不随指针越过左缘而重排。

**本节只说哪个屏用哪个部件；部件欠使用者什么写在 §7。**

**`views/parts/` 里有五个组件今天只有 `#/gallery` 一个使用者**——`combobox`、`dialog`、`notice`、`row`、`skeleton`。五个都不是「没有第二个座位」：座位都在，只是今天由座位自己手写着同一件事，所以每个组件欠的是一次搬家，不是一次删除。`combobox` 的座位是 `setup/models.tsx` 的两个 `<select>`，一个端点答两百行时下拉正是它替换的那个控件；`dialog` 的座位是今天根本不问的那一问——移除端点与删除 MCP 服务器直接执行，全客户端没有一处确认；`notice` 的座位是 `views/notices.tsx` 与 `views/refusal.tsx`，两处各自手写 AxError 的三段式，所以那件事今天有三个家；`row` 的座位是 `mcp/servers.tsx` 的服务器行、`setup/providers.tsx` 的端点行与 `record.tsx` 的账本行；`skeleton` 的座位是 `views/machine.tsx` 手写的那两条骨架。

## 7 `views/parts/` 的交互契约

> **这是规格，不是描述。** 表里写的是部件欠使用者什么；今天的代码欠而未还的十二处，逐条点名在 7-8。模式名与键表借鉴自哪几份文档、为什么不产生许可证义务，一处记在 `docs/third-party.md` §5，本节不复述。

**判定：不引入任何 UI 库依赖。** `xtask/src/npm.rs:64` 的 `RUNTIME` 是这条判定的机器面——运行时依赖恰为 `effect` 与 `solid-js`，要加一个组件库就得先改那一行，而**一道专为阻止依赖蔓延而设的闸，第一次例外就是它失效的开始**。理由不是保守：`parts/dialog.tsx` 已经把模态整个交给原生 `<dialog>`（top layer、焦点陷阱、Esc、其余页面 `inert`，四件都是平台承担的，见设计 4-20），`parts/tip.tsx` 已经把 `title` 换成一个 `role="tooltip"` 的兄弟节点（设计 4-18）。**这些正是一个组件库存在的理由，而平台现在自己提供了**；引一个库会让同一件事有两个提供者。判定失效的条件写在 7-9，一个字都不留给临时判断。

### 7-1 不收键的部件

这些部件不进 Tab 序列、不读键，只欠一个角色和一组确切的 `aria-*`。

| 部件 | 角色 | `aria-*` 的确切取值 |
|---|---|---|
| `badge.tsx` | 无（行内文本） | 圆点 `aria-hidden="true"`；状态由词承担，颜色只重复那个词 |
| `banner.tsx` | 实时区域 | `weight="alert"` → `role="alert"`；其余 → `role="status"` |
| `notice.tsx` | 实时区域 | 同上；`seat` 只改画法（浮起或列在中心），不改角色 |
| `empty.tsx` | 无 | 形状 `aria-hidden="true"`；那句话与那个动作是它全部的可读内容 |
| `progress.tsx` | `role="progressbar"` | `aria-label` 取调用方给的名字；`aria-valuemin="0"` 恒在；`total > 0` 时 `aria-valuemax="<total>"`、`aria-valuenow="<done>"`、`aria-busy="false"`，`total ≤ 0` 时这两个值一个都不写并 `aria-busy="true"` |
| `skeleton.tsx` | `role="status"` | `aria-label`、`aria-busy="true"`；每根条 `aria-hidden="true"` |
| `row.tsx` 的 `Row` | 无 | `onOpen` 在场时两段文字合成一个 `<button>`，右侧动作各自是独立的一站；行本身不收键，走动由 `RowList` 承担（7-4）|
| `kbd.tsx` 的 `Kbd` | 无 | 一个字形一个 `<kbd>`，不取焦、不收键 |

### 7-2 一次一个动作的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `button.tsx` | APG Button | Enter | 激活；`state() !== "idle"` 时 `onClick` 原地返回 |
| | | Space | 同 Enter：平台把两个键都送进同一个 `onClick`，所以一次判定挡住指针、Enter 与 Space 三种输入 |
| `path.tsx` 的显示控件 | APG Button | Enter／Space | 地址解析得出时发 `reveal`；解析不出时 `aria-disabled="true"`，点击落进一个空操作 |
| `field.tsx` | 有标签的文本框（无复合模式） | 平台的单行编辑键 | 由浏览器实现，本部件不截获 |
| | | ↑／↓（`kind="number"`） | 按 `step` 增减，由平台实现 |

`button.tsx` 的 `aria-*`：`aria-disabled` 在 `loading` 或 `why` 在场时为 `"true"`，`aria-busy` 只在 `loading` 时为 `"true"`，`why` 在场时 `aria-describedby` 指向 `Tip` 的 id。**用 `aria-disabled` 而不是 `disabled`**：控件因此留在 Tab 序列里，键盘到得了它，读屏也读得到它为什么按不动。

`field.tsx` 的 `aria-*`：`<label for>` 给名字（`labelling="hidden"` 只把标签移出视线，名字仍在）；`aria-invalid` 恒等于 `error !== undefined`；`aria-describedby` 是调用方的 `describedBy` 与本格说明段 id 的并集，**错误替换说明而不是叠在它上面**，错误段自己带 `role="alert"`。红边有两条权威且说的是两件事：`error` 是城的回答，一到就红；`:user-invalid` 是浏览器读 `type` 与 `pattern` 的结果，失焦后才红。

### 7-3 提示与模态

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `tip.tsx` | APG Tooltip | Escape | **规格要求撤下提示；今天没有实现**（7-8 第 9 条）|
| `dialog.tsx` | APG Modal Dialog | Tab／Shift+Tab | 在对话框内循环，由平台实现 |
| | | Escape | 平台发 `cancel`，本部件 `preventDefault()` 后回调 `onCancel`——默认行为会绕过调用方关掉元素，而 `open` 还说着开着 |
| `kbd.tsx` 的 `Cheatsheet` | 手写的 dialog | Escape | 由外壳 `app.tsx` 的按键处理器答，不在部件里（7-8 第 12 条）|

`tip.tsx` 的 `aria-*`：提示节点是 `role="tooltip"`，id 交给调用方——控件自己有可见文字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`。**组件不猜**，因为只有调用点知道控件有没有名字。显示由 `:hover` 与 `:focus-within` 触发，延迟 300 ms；提示自己 `pointer-events-none`，永不取焦。

`dialog.tsx` 的 `aria-*`：`aria-labelledby` 指向标题，`aria-describedby` 指向说明段**且仅在 `detail` 在场时才写**。取焦由文档顺序决定：平台取对话框内第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。**点 `::backdrop` 不关闭**：撤不回来的那一问不该被一次落在外面的点击答掉。

### 7-4 在几件之间走动的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `segmented.tsx` | APG Radio Group（roving tabindex） | Tab／Shift+Tab | 进出控件；控件在 Tab 序列里只占一站 |
| | | → | 移到下一个可选格并选中它，走到末端回到开头 |
| | | ← | 移到上一个可选格并选中它，走到开头回到末端 |
| | | ↓／↑ | **规格要求同 →／←；今天没有实现**（7-8 第 5 条）|
| | | Space | **规格要求选中当前聚焦格；今天没有实现**（7-8 第 5 条）|
| `tabs.tsx` | APG Tabs（自动激活） | → | 下一个透镜并立即切换，走到末端回到开头 |
| | | ← | 上一个透镜并立即切换，走到开头回到末端 |
| | | Home／End | 第一个／最后一个透镜并立即切换 |
| | | Space／Enter | 与点击同一条路；因为切换已跟随焦点，它们不额外做事 |
| `row.tsx` 的 `RowList` | 无 APG 部件模式：一串各自可达的行，加上方向键 | ↓／↑ | 走到下一／上一行，焦点落在那一行第一个可达控件上；**两端不环绕**——一本上千行的账本从末行跳回首行，是把人移到了他看不出自己去过的地方 |
| | | Home／End | 第一／最后一行的第一个可达控件 |
| | | 落在文本框、`<select>` 或可编辑区域上的同一批键 | 不接管：那些键在那个控件里已经有意思了 |
| | | Tab | 照旧逐行走——**每一行仍是一个 Tab 站，方向键是加法不是替换** |

`segmented.tsx` 的 `aria-*`：轨道 `role="radiogroup"` ＋ `aria-label`；每格 `role="radio"`、`aria-checked` 等于「这一格就是 `held`」、`why` 在场时 `aria-disabled="true"` 并 `aria-describedby` 指向 `Tip`。**Tab 序列里的那一站由 `tabStop` 独家决定**：选中格；无选中时第一个可选格；全部被拒时第 0 格（那格的原因还得读得到）；空控件一站都没有。选择跟随焦点，所以不可选的格被 `nextStop` 跳过——落在上面就等于选中它。

`tabs.tsx` 的 `aria-*`：`role="tablist"` ＋ `aria-label`；每个 `role="tab"`、`aria-selected` 等于「这就是 `current`」、`tabindex` 只给当前那个 `0`。自动激活是 APG 对「面板内容已在本地、切换无可察延迟」的推荐读法，本客户端三个使用者都满足它。

### 7-5 开一层列表的复合部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `combobox.tsx` | APG Combobox（listbox 弹层） | ↓／↑ | 游标下移／上移一行，钳在列表两端 |
| | | Home／End | **规格要求到首行／末行；今天没有实现**（7-8 第 2 条）|
| | | Enter | 采纳游标行，关闭弹层，焦点回触发器 |
| | | Escape | 关闭弹层，清空过滤词，焦点回触发器 |
| | | Tab | **规格要求关闭弹层并让焦点正常离开；今天弹层留着**（7-8 第 2 条）|
| | | 可打印字符 | 过滤，并把游标复位到第 0 行 |
| `popover.tsx` | 多列 listbox，装在 `role="dialog"` 里 | ↓／↑ | 当前列的游标下移／上移，钳在两端 |
| | | Home／End | 当前列的首行／末行 |
| | | Tab／Shift+Tab | **换列**（环绕）并把游标复位到第 0 行——本部件在此覆盖平台的 Tab |
| | | Enter | 应用当前列的游标行，回调 `onApply` |
| | | Escape | 回调 `onClose` |

`combobox.tsx` 的 `aria-*`（规格）：文本框是 `role="combobox"`，带 `aria-expanded`、`aria-controls` 指向列表、`aria-activedescendant` 指向游标行；列表 `role="listbox"` ＋ `aria-label`；每行 `role="option"`，`aria-selected` 只标**已选中的那个值**，不标游标。今天的实现把 `aria-haspopup="listbox"` ＋ `aria-expanded` 放在触发按钮上、过滤框没有角色、游标只有底色——见 7-8 第 1 条。

`popover.tsx` 的 `aria-*`（规格）：外层 `role="dialog"` ＋ `aria-label`；每列 `<ul role="listbox">` ＋ `aria-label`；每行 `role="option"`。**`aria-selected` 在两个部件里必须说同一件事——「这是当前生效的值」**，游标一律由持焦元素的 `aria-activedescendant` 承担；今天 `popover.tsx` 用 `aria-selected` 标游标，而真正生效的那一项只有一个圆点（7-8 第 3 条）。两种触发各有一条焦点路：按钮触发时列表自己取焦（`tabindex` 只给当前列 `0`）；文本框触发时调用方经 `bind` 拿走键表，焦点留在文本框里，此时 `aria-activedescendant` 必须写在那个文本框上（7-8 第 4 条）。

### 7-6 表格

`table.tsx` 是一张数据表，**不是 APG Grid**：它不做二维方向键导航，Tab 依次走过排序按钮、勾选框与可改单元格，Enter／Space 在排序按钮上切换方向。

`aria-*`：`<caption class="sr-only">` 给表名；**`aria-sort` 只写在可排序的列上**，取 `"ascending"`／`"descending"`／`"none"`；表头勾选框 `aria-label` 取 `allLabel`，行勾选框取 `keyOf(row)`，可改单元格取 `"<列名> <keyOf(row)>"`。表头勾选框在部分选中时必须是 `indeterminate`——说「一个都没选」是一句假话（7-8 第 7、8 条）。

### 7-7 焦点还原：一条规则，三种实现

**一个把焦点拿走的部件必须把它还给打开它的那个元素**，在它关闭的那一刻。三种实现今天并存，它们的差别只在谁记住了那个元素：

| 谁还 | 部件 | 怎么还 |
|---|---|---|
| 平台 | `dialog.tsx` | `close()` 按 HTML 标准把焦点还给 `showModal()` 之前持焦点的元素，本文件因此没有一行取焦代码 |
| 部件自己 | `combobox.tsx`、`popover.tsx` | 前者记住触发按钮的 ref，`shut()` 时还；后者在 `onMount` 记下当时的 `document.activeElement`，`onCleanup` 还（`bind` 模式下焦点从未离开文本框，因此不还）|
| 外壳 | `kbd.tsx` 的 `Cheatsheet` | `app.tsx` 在打开前记 `opener`，`closeSheet` 时还——**全客户端唯一一处还原权威不在部件里**，它随 7-8 第 12 条的搬家一起消失 |

### 7-8 今天与模式不符的十二处

每一条都是「规格已定、实现未到」，不是待议的设计问题。

1. `combobox.tsx:92` — 过滤框没有 `role="combobox"`、`aria-controls`、`aria-activedescendant`，游标只有底色；读屏用户按方向键时听不到任何变化。
2. `combobox.tsx:106`–`127` — 缺 Home／End；Tab 不关闭弹层；全文件没有点外面关闭或失焦关闭的路，一个开着的弹层可以留在页面上。
3. `combobox.tsx:138` 与 `popover.tsx:176` — 同一个 `aria-selected` 两种读法：前者标已选中的值（对），后者标游标（错），而后者真正生效的那一项只由 `popover.tsx:190` 的圆点承担。
4. `popover.tsx:163` — `bind` 模式下 `aria-activedescendant` 写在不持焦点的 `<ul>` 上，因此 composer 里按方向键时读屏什么都不报。
5. `segmented.tsx:182`–`195` — 缺 ↓／↑ 与 Space，APG Radio Group 的键表只实现了一半。
6. `tabs.tsx:58` — `id="tab-<id>"` 今天没有读者：三个使用者（`run`／`record`／`mcp`）都没有 `role="tabpanel"` ＋ `aria-labelledby`，`<button role="tab">` 也没有 `aria-controls`，所以「这块面板属于哪个页签」在页面上说不出来。
7. `table.tsx:108` — 不可排序的列也写 `aria-sort="none"`，那是一句关于一个排不了序的列的排序陈述。
8. `table.tsx:98` — 表头勾选框只有选中与未选中两态，部分选中时说「一个都没选」。
9. `tip.tsx` — 全文件没有 Escape 撤下提示，而那是 APG Tooltip 的唯一一个键，也是 WCAG 1.4.13「可撤下」要的那一件。
10. `field.tsx:94` 与 `button.tsx:87` — 「一个人不能用的控件」两套写法：前者用原生 `disabled`（离开 Tab 序列，原因读不出来），后者用 `aria-disabled` 加一次点击判定。规格取后者。
11. `client/src/views/parts/row.tsx` 的 `RowList` — `<ul>` 直接收调用方的 `<Row>`，而 `Row` 画的是 `<div>`：一个子元素不是 `<li>` 的列表，读屏报得出「一个列表」却报不出「几项」。该文件本波正在改写，所以这一条只记事实、不钉行号。
12. `kbd.tsx:38`–`60` — `Cheatsheet` 是今天唯一一个没走 `parts/dialog.tsx` 的模态：没有焦点陷阱、没有 `aria-modal`、Esc 在外壳里、还留着全客户端仅剩的层号之一（`kbd.tsx:46` 的 `z-20`，设计 4-21 记的那个例外）。原生 `<dialog>` 三家引擎都支持，所以这一条是搬家而不是取舍。

### 7-9 重开参数：判定在什么条件下失效

**当某个部件的正确无障碍行为在 Chromium、Firefox、WebKit 三家上都无法用平台能力加百行以内的自有代码达成**，才回到「`RUNTIME` 放宽到三项」，并在同一变更集里写明是哪一个部件逼出了这次例外。三条限定一个都不能省：三家都试过（不是一家不支持就算数）、百行算的是自有代码的行数、例外记进本节而不是只躺在一条提交信息里。

今天没有任何部件触发它：`<dialog>`、`::backdrop`、`@starting-style` 与 Popover API 三家都有，而三家之间确实缺的两件（CSS anchor positioning、`field-sizing: content`）都不是无障碍行为，它们各自的降级分支已经在 `tip.tsx` 与 composer 里。

### 7-10 这张表的机器读者

**`#/gallery` 的夹具断言本节的键表**，这是让规格不止有人类读者的那一步：每个收键部件在那条路由上有一份夹具，夹具按「初始焦点 ＋ 一串按键 → 焦点落点、`aria-*` 取值、回调是否发生」逐行断言 7-2 至 7-6。夹具与断言的实现属于 `client/src/views/gallery.tsx` 与 `xtask/src/render/`，本节只定内容。

今天的 `xtask render` 读的是画出来的盒子与它们的名字（`xtask-SPEC.md` 8-13），**一次按键都没有进过真引擎**——在这第二个读数落地之前，本节的键表没有机器读者，这一点如实记在 §8 的「未验的」里。

## 7A 表面角色：一个面「是干什么的」只有一个家

**十一档灰阶是值的权威，角色是用途的权威，两层不重叠。** `--color-g0…g10` 与 `xtask/src/color.rs:164` 的「十一档」硬断言一个字不改；本节新增的是它们之上的一层**角色**。

改前的缺陷不是缺档位，是**「什么样的面算一个抬起的控件」这个事实有两百三十六个家**——`client/src/views/` 里每一处手写的 `bg-g2` 与 `border-g3` 都是一个家，没有一个是权威，两个家不一致时谁也看不见。

### 7A-1 角色是单跳，不是值

```css
--color-raised: var(--color-g2);
```

**不许写字面量。** 一个抄在档位旁边的 `oklch()` 立刻成为那个值的第二个家，档位一动就要手工重调；一个十六进制别名更糟——`xtask/src/color/tables.rs` 只保留值能解析成 `oklch()` 的声明，所以它**会被静默忽略而不是被拒绝**。单跳还让一份声明同时服务两种打光：浅色块重述每一档，指向档位的角色跟着走，不必在那里再声明一次。

### 7A-2 封闭词汇

角色名住 `xtask/src/color/roles.rs` 的 `ROLES`，共 21 个：四档表面（`page` / `chrome` / `raised` / `raised-hover`）、三种非导航填充（`speech` / `track` / `disabled`）、一种标记填充（`mark`）、三档边（`edge` / `edge-panel` / `edge-input`）、一种覆在彩色实心上的墨（`on-accent`），以及城市插画自己的九档（`drawn-*`）。

**加一行是一次设计决定。** 只有当一个人能用一句不提档位的话说出它回答什么问题时，这个角色才配有名字——草稿里 `inert` 与 `resting` 相隔一档，没有读者说得出某个圆点是哪一个，它们现在是一个 `mark`。

### 7A-3 闸判四条（`cargo xtask color`）

1. 每个既非档位、非文字 token、值又不是 `oklch()` 的 `--color-*`，必须是 `ROLES` 里的名字，且是到一个已声明档位的**单跳**；
2. `ROLES` 里的每个名字，样式表里**恰好声明一次**；
3. 每个角色在 `client/src` 里**至少有一个读者**——没有读者的角色是一个有值没人读的名字，删掉而不是留着；
4. `theme.css` 之外的任何文件**不得拼出档位工具类**（`bg-g2`、`border-g3`、`fill-g9` …）。

**闸因此多一条规则而不是少一条**：十六进制别名既解析不成 `oklch()`，也不是单跳，两道都拦。

### 7A-4 图底关系：外壳上浮，不是内容下沉

参考图把代码窗格画得比页面更暗，本仓不能照做——`xtask/src/color.rs:60-64` 有「g0 是页」的契约，ramp 两端被硬断言。**同一个读数从另一侧取到**：内容留在 `page`，而框住工作的东西（左栏、事实条、视图头）升到 `chrome`。屏幕上最深的一片仍然是工作，契约一个字没改。

要紧的距离是 `page` → `raised`，深色页上 100 个千分点：一个控件必须不靠边框就看得出可以按。`page` → `chrome` 只有一半，是有意的——一个宣告自己的框会跟它框住的工作抢注意力，而且它另有一条 `edge`。

## 7B ACCENT 是预算，不是装饰

**accent 只许出现在两处**：焦点环，以及选中行的 2 px 左边条（`parts/tabs.tsx` 的当前页签就是后者）。

其余一切「当前 / 已选 / 激活」，一律用表面差抬一档表达。本轮收窄的四处：`parts/segmented.tsx` 的滑块默认色从 accent 改为 `raised-hover`（`Tone` 的缺省从 `accent` 改名为 `plain`）、`rail.tsx` 的在跑计数药丸改为 `raised-hover`、焦点环用 `color-mix` 削到六成、composer 的聚焦边框从「整条 accent」改为「虚线转实线」。

判据是一句可核的话：**全屏对比度最高的元素应当是「停」**，因为那是人需要在慌乱中一次点中的东西。

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

## 8 验收

`bun run lint`、`bun run typecheck`、`bun run test`（93 条）三样绿，`cargo xtask npm`、`cargo xtask wire-ts`、`cargo xtask color`、`cargo xtask wording`、`cargo xtask render` 绿；`just check-client` 是这三条脚本的一条线。

在一座真城加一个说 OpenAI 形的假供应方上走得通的：连接与握手、引导五步、从 composer 派活、工具调用折叠、Markdown 回复、结局分隔线、城市绘图、目录树与文件原文、run 页四透镜、记录三透镜、成本页、设置页 attach 与 select、`#/mcp` 三扇门加删（写进 `CONFIG.toml`）、楼页提交列表与 session 跳转、左栏展开、草稿留存。

**`keeper()` 今天只答得出 `"browser"`**：`Query::Preferences` 已在 Rust 侧（`channels::PreferencesAnswer`），`client/src/wire.ts` 尚未重生，所以 `adopt` 在生产路径上没有调用者。`"city"` 那一态由 `#/gallery` 的 `kept` 夹具画出并被 `xtask render` 在五个页宽下量到，因此它不是一条没人看过的分支。重生之后要接的是两处：`ui.tsx` 问 `QUERIES.preferences` 并把答案交给 `adopt`，五个具名改动各发一条 `Command::PutPreferences { patch }`。

**未验的**：`Changes`／`Hunks` 有内容时的样子、Firefox 与 Zen 的无头截图（`-screenshot` 不出图，须走 BiDi）、`Tip` 两条定位分支各自的 `#/gallery` 夹具（`xtask render` 只读 `#/gallery`，所以这两条分支在真引擎里的落点尚无机器读者）、**§7 的键表**（`xtask render` 今天只量盒子，没有一次按键进过真引擎，所以每一行键表今天的读者只有人）。
