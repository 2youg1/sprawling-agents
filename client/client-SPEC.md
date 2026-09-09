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
