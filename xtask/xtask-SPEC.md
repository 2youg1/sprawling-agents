# xtask-SPEC.md — 构建门（gates）

> crate：`xtask`（工作区成员，不占产品拓扑）。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 章节骨架：十七节（「两个设计」置于接口先行后，「模型体验」置于测试与约束后）。

## 1 需求拆解

把 ARCHITECTURE.md 的冻结面变成机器检查，每一件可独立完成、独立验收：

| 单元 | 一句话 |
|---|---|
| header | 每个 `.rs` 前五行恒为 MPL-2.0 通告加版权行 |
| lexicon | 禁用词命中即红；数据面 `xtask/lexicon.toml` |
| modmap | `crates/**/src/**/*.rs` ↔ ARCHITECTURE.md §6 模块表一一对应；状态列一致性；索引文件零逻辑 |
| depmap | crate 依赖边 ⊆ §2 depmap 块；`pub trait` 仅现于 §3 缝清单文件 |
| guard | 改动门自身、又同时改动门所判源码的提交，必须携 `Verdict:` 尾注 |
| ax | 定稿屏（`crates/web/screens/*.html`）写下的角色、可及名、当前页标记与地标元素，客户端（`crates/web/src`）也提供 |
| wording（V3.51 上线） | 读者拿到的词出自 `web::lang`：`crates/web/src` 里 RSX 文本节点与朗读型属性的字面量，去掉插值后不得剩下相邻两个字母；行内 `wording-ok:` 豁免专名 |
| render（V3.48 上线） | 定稿屏在真引擎里画出来，量盒子落在哪：一页一条书脊、面板头坐在自己面板的左上角、没有东西宽过装它的区域 |
| wiring（V3.32 上线） | 城能执行的动词必须从客户端够得到；三个来源零副本（`wire.rs` 的 `enum Command`、`run_command` 的臂、`crates/web/src`），channels-SPEC §19-2 只提供三者都说不出的那一件事——这个动词该由哪一侧够到 |
| spec | 生成 `<crate>-SPEC.md` 骨架（Daily Loop 的 `just spec`） |
| secret（S2.12 上线，card-gates 补文件类） | 全仓＋夹具扫 secret shape（判定复用 `kernel::secret::scan`，无内联豁免）；只扫人写的文件，生成的锁文件与记录的快照由它们被扫的输入作证（§8-9）；兼查 `Sealed::expose` 调用点白名单 |
| specalign（S2.12 上线） | kernel 枚举 ↔ kernel-SPEC §8-1／§8-4 表逐 variant：消费真 enum（AxCode::ALL／EventKind::ALL）对表作证，计数、归属、carrier／窗类逐项同 |
| apisync（S2.12 上线） | 双断言：①基线新鲜——`cargo public-api` 实时面与已提交基线逐行同；②同集变更——基线文件变即要求同 crate SPEC 同集被触 |
| badge（P5 上线） | 体积徽章由 `budget` 的读数渲染成 `docs/badges/*.svg`；徽章陈旧＝`budget` 门红（不新增门） |
| budget | `xtask/budgets.toml` 里每一行可称重且被 gated 的预算，当场称一次；称不出则沉默（本机没构建产物不是缺陷），壁钟读数只入册不入门 |
| color（S4 上线，card-gates 改价） | 颜色在每个客户端里恰好被命名一次（产地表见 §8-8），且以色域上限的比值表达；扫仓库根，文件自豁免 |
| release（P4.14 上线，P5.05 增第三条断言） | 公开树由过滤生成；三条断言：公开树上零脚手架路径、产品文档不得链向或在正文里点名脚手架、任何发布文件不得携家目录路径 |
| length（R2.20 上线，V3.29 加文件面与参数面） | 一个生产函数不得长过 `function_length`（今 200 行）、不得多于 `argument_count` 个参数（今 4 个，不含接收者），一个源文件不得长过 `file_length`（今 400 行，含测试；2026-09-05 自 1000 改价，权威在 budgets.toml）；函数尺寸与签名以 `syn` 量得，文件尺寸即行数 |
| npm（card-F3 上线） | `client/` 的依赖面：锁文件在盘且与 `package.json` 逐条同、运行时依赖恰为 `solid-js` 与 `effect`、树上每个包的许可证都在 `deny.toml` 的准许表内 |
| gates | 顺序跑全部门，聚合报告，任一违规即退出码 1 |
| wire-ts（card-6.2 上线） | `client/src/wire.ts` 由 `channels::wire_schema()` 生成：每个具名类型一条 Effect `Schema` 值加一条 TS `type`，外加 `WIRE_V` 与 `WIRE_HASH`；不带 `--write` 时与盘上文件逐字节比对，第一处不同的行即红。命令已就位，进 `gates::run` 那张数组由主线单独一枚提交完成 |

### 门禁针对的 LLM 失效模式（本 crate 存在的理由）

| 失效模式 | 拦截门 | 机制 |
|---|---|---|
| 顺手新建文件／utils 沼泽 | modmap | 模块表是封闭清单，表外文件即红 |
| 声称完成但状态未翻转 | modmap | 文件存在而状态＝未建 → 红（完成四件套的机器面） |
| 逻辑漏进 lib.rs／索引文件 | modmap | 纯索引文件只许注释、属性、mod、use |
| 偷加依赖边、绕过分层 | depmap | 实际边 ⊆ 文档边；kernel 恒零内部依赖 |
| 娱乐性抽象（无第二实现的 trait） | depmap | `pub trait` 只许出现在缝清单文件 |
| 放宽 lint／改门过门／删表行 | guard | 门自身变更与被判源码同处一枚提交时，必须携用户 verdict 尾注 |
| 词汇漂移、自造同义词 | lexicon | 附录 A 禁用词的机器子集，命中即红 |
| 忘记许可头或版权行 | header | 五行逐字节比对 |
| 翻译定稿屏时丢掉 `role`／`aria-label`／`aria-current` | ax | 两侧同一读法取出可及面，屏上有而客户端无即红 |
| 中文页面上直接写一句英文（模型的母语泄漏） | wording | 按位置判定：落在文本节点或朗读型属性里的字面量，去掉插值后仍带词即红 |
| stub／todo!／unwrap 蒙混 | （不在本 crate）workspace lints | clippy deny 已覆盖，本 crate 不重复 |
| 一个函数里塞进整条流程（模型最常见的结构失效） | length | 超过行数预算即红，报出函数名、行数与预算 |

## 2 验收标准

- 每门在违规夹具上报告非零、在干净仓库上报告零（单测覆盖解析与判定核心）。
- 违规输出恒为 three-part 形：rule｜violation｜alternative，附 `gate` 名与 `file:line`。
- `cargo xtask gates` 在本仓库当前状态全绿；人为注入六类违规（表外文件、lib.rs 加 fn、暗依赖边、缝外 pub trait、禁用词、定稿屏上有而客户端没有的 `role`）各能单独触红。
- 全部代码过 workspace lints（无 unwrap/expect/panic/索引/切片/裸算术/as）。

## 3 假设与歧义

- 「注释与标识符扫描」简化为整行子串扫描：中文禁用词只会出现在注释与文档，英文禁用词不构成合法标识符片段。误伤由 `lexicon-ok:` 行内豁免兜住。
- guard 本地默认只查 HEAD 一枚提交（工作树未提交的改动不查——门只对可判定对象作证）；CI 以 `--range` 查整个变更集：pull request 取 `base..head`，push 取 `before..after`。**只判 tip 一枚是一个洞**：一个 PR 可以把放宽门的那枚提交夹在中间，再用一枚干净的 tip 过关。区间两端解不出来时（改写过历史的 push、首次 push）回落到 HEAD 并明说回落了，不得让一个解不开的区间读起来像一个干净的区间。
- **guard 只对「门变更与被判源码同处一枚提交」索裁定（V3.25）**。被判源码是一张封闭前缀表：`crates/`／`citysim/`／`fuzz/`。两边都在一枚提交里，就是这道门要关的那条捷径：活在提交里，本该拒绝它的规则也在同一枚提交里，一次绿的运行同时报掉两者。**全部改动都在门自身的那一枚提交是重新定价，属日常工作**（AGENTS.md `guard` 行原文如此）：它的 diff 除了「这条规则现在标价不同了」什么都不说，而那正是评审人要看的东西，一个签名反而把它遮住。旧口径（凡触及 `xtask/` 即索裁定）把重新定价收得与破规一样贵，而那正是禁 JavaScript 一条比它的论据多活一年的机制性原因（`docs/frontend-method.md` §4）。**留下的洞是故意的并记在此**：先一枚放松、再一枚过门，两枚都不被索裁定。堵它就得为每一次重新定价收一次裁定，而那个价钱正是本条要取消的。
- **guard 区分「门怎么判」与「门产出了什么」（P3.01，用户裁定）**。`xtask/api-baselines/` 不在保护面内，而 `xtask/src/apisync.rs` 仍在。理由是两道门曾经互相矛盾：`apisync` 命令公开面一变就跑 `cargo xtask apisync --write`，而那就是写进 `xtask/`；两条同时遵守，等于**每一次公开面变更都要一次裁定**，而一个次次都要签的字会贬值成仪式。重生一份 baseline 放松不了任何东西：`apisync` 仍然拿它与实时 API 逐行比，且 diff 就在提交里给评审人看。**这条不得推广到其它目录**：判据是「该文件由门自己生成且被门自己校验」，不是「改起来麻烦」。
- **guard 同样区分「放松」与「收紧」，判据是 diff 的形状而不是目录（V3.35）**。`length` 门自己命令：一个回到预算之内的文件**必须**从 `[file_length.predating]` 划掉，否则它报「不再是例外」。于是拆分与划掉天然同住一枚提交，而旧口径会为每一次拆分收一次裁定。**划掉一行豁免放松不了任何东西**：那个文件从「只受自己那颗钉子约束」变成「与其他所有文件同受预算约束」。**登记表只许变小，而变小有两种形状**：删掉一行（那个文件从「只受自己那颗钉子约束」变成「与其他所有文件同受预算约束」），或把一颗钉子换成更小的钉子（同一个文件被约束到更短的长度——文件缩了但还没回到预算之内时，`length` 门本来就要求同一次改动里把钉子降下来）。因此 `xtask/budgets.toml` 在整份 diff 只做这两件事时不算门面，其余一律照旧。**仍然会重新武装这道门的**：新增一行、钉子变大、预算被改、注释被动，以及把一条超长签名的豁免搬到它移去的新地址——一条签名要么被修好要么原地不动，不得带着免死金牌搬家。
- 附录 A 中语境依赖的禁用词（如 session 指本城运行时、建筑指项目时）不入机器数据面，由评审执行；lexicon.toml 内以注释记录此边界。

## 4 现状分析

空仓库（Stage 0 空壳）。无既有实现可比对；性能无关紧要（全仓扫描 <100ms 量级即可）。

## 5 权威信源

硬化十七条；门表（AGENTS.md）；施工协议（AGENTS.md）；MPL 头全文（`LICENSE`）；退役词全集（`xtask/lexicon.toml`）；ARCHITECTURE.md §3（depmap 块）、§4（缝清单）、§12（模块图列契约）。

## 6 命名统一

gate／Violation／rule／violation／alternative（three-part refusal 的施工侧同构）；模块名与子命令名一致：header、lexicon、modmap、depmap、guard、ax、render、wiring、wording、spec、gates。

## 7 模块边界

一门一文件。十六道门：`header`｜`lexicon`｜`modmap`｜`length`｜`depmap`｜`secret`｜`color`｜`wording`｜`ax`｜`render`｜`wiring`｜`budget`｜`specalign`｜`apisync`｜`release`｜`guard`。三个不判只做的模块：`main`（分发）｜`report`（Violation 与渲染）｜`walk`（确定性文件遍历）。其余各文件各自被某一道门调用而不自成一门：`badge`（渲染与陈旧判定，被 `budget` 调用）｜`vocabulary`（`lexicon` 与 `wording` 共用的词形读法）｜`spec`（只生成骨架）｜`mem`／`sbom`／`repro`／`package`（`just` 的量具与交付物，恒不入 `gates`）。

**length 门的形状属于 modmap 而不属于自己**：形状列的解析只住 `modmap::shapes`，因为模块表只应有一个读者——列格式一变，只有一处要改。

**本模块不做什么（否定式两条）**：不修改任何被检文件（门只判不改；唯二例外＝spec 只新建不覆盖、`apisync --write` 只重写基线文件）；不缓存扫描结果（每次全量重扫——确定性优于速度）。
原第三条「不做 color（S4 随 `web::theme` 启用，届时增列）」已到期：`color` 已是一道门，故划掉。

**secret 门细则**（S2.12）：扫描面＝仓内全部文件（含 fixtures／语料），排除隔离区 local/、.git、target；判定器＝`kernel::secret::scan`（xtask 依赖 kernel，工作区成员不占产品拓扑，合法）；命中只报文件＋偏移＋长度，恒不回显字节；无内联豁免（豁免口会被注入内容利用）。兼查：`crates/*/src/**` 内 `.expose(` 调用点白名单＝kernel/src/secret.rs（定义处）、gateway/src/endpoint.rs、gateway/src/native.rs；命中即红。自测纪律：扫描器自身测试的高熵样本在源码中必须拆段拼接，不留可扫描的完整字面量。

**已复核字面量表（P3.05 增）**：判定器恒不改——它的活是在入口捕获一切像钥匙的东西，那里误报不要钱；**本门问的是另一个问题**「这里是不是提交了一份凭证」，那里误报要一次构建。故门内持一张 `NOT_CREDENTIALS` 精确字面量表，逐条写明它是谁、为什么不可能是凭证。三条纪律：①**整串精确匹配**——带前缀或后缀的更长 token 仍是命中，故没人能靠戴一个已复核的名字混过去（一条断言钉这件事）；②**表住门里而不是站点上**——注释式豁免是注入内容能写的洞，这张表不是；③表在 guard 保护面内，增一条即须 `Verdict:` 尾注。首条：`CanvasRenderingContext2d`（`web_sys` 的 2D 画布类型，24 字节且含数字，故触发混合字母表规则；`crates/web/Cargo.toml` 的 feature 与 `web::city_view` 的绘制侧各出现一次）。其后：`CC_x86_64_unknown_linux_musl`（Cargo 的分目标 C 编译器变量名，`release.yml` 的 musl job 设它）；以及 `windows` crate 的六个 feature 名 `Win32_System_DataExchange`／`Win32_System_Threading`／`Win32_System_Variant`／`Win32_UI_Accessibility`／`Win32_UI_Input_KeyboardAndMouse`／`Win32_UI_WindowsAndMessaging`——`desktop/Cargo.toml` 用它们选出 Windows 臂要调的 API 面，feature 名由 resolver 读取、自身恒不持值，`Win32` 里的数字与下划线并置才是触发混合字母表规则的原因；只列长度 ≥20 字节的六个，更短的名字够不着熵侦测器。

**apisync 门细则**（S2.12）：基线集＝存在 `<crate>-SPEC.md` 的产品 crate（SPEC-first 即同步契约面；现在＝kernel/memory/runtime）；基线住 `xtask/api-baselines/<crate>.txt`，由 `cargo xtask apisync --write` 生成（`cargo public-api -p <crate> --simplified`，缺省 feature＝dev-only feature 面不入基线，台账已豁免）；断言①实时重算与基线逐行同（工具链缺失＝fail-closed 报装机指引，不静默跳）；断言②提交区间内基线文件变 ⇒ 同 crate SPEC 同集被触（git 面，复用 guard 的区间语义：本地缺省 HEAD，CI --range）。两断言合成链：API 变→①逼基线更新→②逼 SPEC 同集。cargo-public-api＋nightly 为环境前置（已装，2026-08）。

## 8 接口先行

```rust
// 每门同一形状（判定函数的施工侧同构）：
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError>;
// guard 例外多携参数：
pub(crate) fn check(root: &Path, range: Option<&str>) -> Result<Vec<Violation>, XtaskError>;

pub(crate) struct Violation {
    gate: &'static str,      // 哪道门
    location: String,        // path 或 path:line
    rule: String,            // 规则（引 ARCHITECTURE 节号或 SPEC 章号）
    violation: String,       // 违反点
    alternative: String,     // 合规替代
}
```

退出码：0 全绿；1 有违规；2 用法或内部错误。

## 8.5 两个设计

**A（选中）：文档即数据面**——modmap/depmap 直接解析 ARCHITECTURE.md 的表与围栏块。杠杆：单一权威，改表即改门，无迁写步骤；缝的位置在文档解析函数，可单测。
**徽章 A（选中）：入库的静态 SVG，由 `budget` 的读数渲染。** 杠杆：本仓库是私有库，托管徽章服务读不到里面的数；且产品承诺「不依赖任何托管服务」，一个每次打开 README 都去请求第三方的图片，正好推翻它上方那句话。离线可看，私有可看，无第三方流量。
**徽章 B（落选）：shields.io 的 dynamic/endpoint 徽章。** 只要仓库私有就取不到 JSON；即便将来公开，也是把「这个项目有多大」这件事的呈现权交给一个外部服务。
**徽章 C（落选）：手写数字。** 四期文档里每一处手写的数都至少陈旧过一次——这正是本条存在的理由。

**B（落选）：独立 TOML 清单**（xtask/modules.toml 等）——解析更稳，但立刻造出「文档表 vs TOML」第二权威，需要第三道门看守二者同步；违反「一个事实只住一处」。落选理由记此，翻案条件：markdown 表解析在两期内出现 ≥2 次误解析事故。
（lexicon 例外地用 TOML：禁用词是数据不是结构，必须有仓库内数据面。）

## 9 工作流程

`cargo xtask <gate>` → 定位仓库根（`CARGO_MANIFEST_DIR` 的父目录）→ 读数据面（ARCHITECTURE.md／lexicon.toml／git）→ 纯函数判定 → 渲染违规 → 退出码。`gates` 依序跑全部机器门，聚合后统一渲染。**门数与门序都只住 `gates::run` 里的那张数组**：`COUNT` 就是它的长度类型参数，数目与清单相隔一个 token，故不可能各说各话。本节此前另抄了一份门名清单，它漏掉 `length` 而没有任何机器发现——要知道跑了哪几道门、按什么次序，读那张数组，不要在文档里再养一份。

## 10 实现逻辑

1. **walk**：手写递归（不引 walkdir），跳过 `.git`／`target`／`node_modules`，输出按路径字符串排序——报告顺序确定，diff 可比。路径统一正斜杠（Windows 反斜杠归一），因为模块表以正斜杠书写。**隔离区**：仓库根 `local/`（gitignore，恒不入库）存 Handoff 与本机备忘；从仓库根扫描的四门（header／lexicon／secret／color）排除它——门只对入库对象作证，非入库物可引用历史词汇与本机路径。modmap／depmap 只扫 `crates/`，嵌套的 `crates/**/local` 仍被封闭清单咬住，无洞。
2. **modmap**：模块行判据＝竖线表行、第 2 列以 `crates/` 开头以 `.rs` 结尾、第 1 列含 `::`、恰七列（card-8.2 增第七列 `Spec`，见 8-10）、第 6 列 ∈ 状态枚举——这组条件把 §3 缝表（四列）与 §10 卡（清单行）天然排除。双向对账：表有文件无（状态≠未建 才要求在盘）；盘有表无（lib.rs 与索引文件豁免）；盘有且状态＝未建 → 「状态未翻转」。索引文件判据：文件名去 `.rs` 后与同目录某子目录同名，且该子目录内有表内文件。
3. **depmap**：§2 围栏块 ```` ```depmap ```` 为机器权威；`cargo metadata --format-version 1 --no-deps` 输出经 serde_json::Value 读取；只查 normal＋build 依赖（dev 依赖留给测试自由）。子集断言而非相等断言——空壳期合法。
4. **guard**：`git rev-list` 取区间（缺省 HEAD 单枚；无提交则跳过并说明），`git diff-tree --root` 取动过的文件，`git show -s --format=%B` 取信息；一枚提交**两边各非空**且信息无 `Verdict:` 行首 → 红。一边是门面（保护路径命中，或 ARCHITECTURE 模块行被**移除**），另一边是被判源码（`crates/`／`citysim/`／`fuzz/`）。两边各自由一个纯函数从文件列表算出（`gate_faces`／`judged_faces`），**于是规则能对着一份路径列表断言而不需要一个带提交的仓库**；拒词只点名前三条被判路径，一份列了八十条路径的拒词没人读。移除的定义是路径级的：某 `crates/**.rs` 路径出现在删除行（`-|`）且不出现在任何新增行（`+|`）——状态翻转在 diff 里是「删一行加一行」，它是最高频的合法编辑，若被误判为删行索 verdict，门就在训练绕门习惯。
5. **vocabulary（R1.15，挂在 lexicon 门下）**：两条断言，各修一种第二权威。①**退役词必须指向被定义过的词**——`lexicon.toml` 说哪种说法作废，`docs/glossary.md` 说该用哪个词，此前无人让二者对账，于是一条退役词可以指向一个词汇表从未定义的名字，而照门的建议改词的人会落到一个没有释义的词上。判据宽一格：replacement 命中任一词汇表**粗体词**或含 `.md`（指向一份文件也是一种定义）。②**能被机器数出来的数不由文档手写**——产品面文档里每一处手写门数都至少陈旧过一次（四份文档写「ten gates」而实际跑十二道）。故读 `gates::COUNT` 与文档对账，中英两种写法（`ten gates`／`十二门`）各认一组，且**刻意只认已经烂过的那几种形状**：一条会猜的规则就是一条会在别的正文上乱咬的规则。
6. **release（P4.14 上线，P5.05 增第三条断言）**：公开树**由过滤生成**而不由手工挑选，分类是一条**封闭的前缀规则**（`is_scaffolding`）；未被规则点名的一律归产品面——**失败方向是故意的**：未分类的文件出现在产物里会被人看见，反过来则悤声消失。三条断言：①公开树上零脚手架路径；②产品文档不得链向或在正文里点名脚手架（无链的「去看 SPEC」最好写也最难发现，故扫全文而不只扫链接）；③**任何发布文件不得携家目录路径**（`machine_path`）。第三条的口径是**隐私而非整洁**：`/tmp`、`/etc`、`C:/windows` 是关于一类机器的事实，而且「绝对路径被拒」那三条测试必须写出一个绝对路径——典型反例先咬住的正是它们，故规则收窄到家目录形状（`:\users\`／`:/users/`／`/home/`／`/root/` 等七种，大小写不计）。扫描面是**全部可读成文本的发布文件**，不只 `.md`：源码与清单里的硬编码家目录更坏而不是更好。报告只截二十字符，因为把整行引进 CI 日志就是把它再公开一次；文件自豁免（同 secret／color 两门的先例：写不出不包含待检形状的检测器）。
7. **badge（P5）**：读数与 `budget` 同源（同一 `measure()`），故不存在第二个数字权威。每个可称重的 gated 行若在 `budgets.toml` 里带 `badge_label`，即渲染一张 `docs/badges/<行名>.svg`。三条纪律：①**颜色不自选**——灰阶取自 `crates/web/src/theme.rs` 的 `GRAY_RAMP`（复用 color 门已有的解析器），OKLCH→sRGB 的换算在此一次算清，因为 SVG 要被任意浏览器渲染，而 `oklch()` 的支持面不覆盖旧版；墨色恒不低于 `INFORMATION_FLOOR`，一条断言钉住。②**平台自报**——二进制体积逐平台不同，故 `badge_platform` 指名哪台机器有权刷新它，别的平台既不写也不判，否则三个平台会互相覆盖同一个文件。③**陈旧即红**——`budget` 门在能称重时比对已提交的 SVG 与当场渲染的 SVG，不同即红并给出 `cargo xtask badge --write`；称不出（本机没构建产物）则沉默，与该门既有的沉默口径一致。`just dist` 末尾调用它，于是发一次 release 就刷新一次，没有人需要去改一个数字。
9. **length（R2.20；文件面 V3.29）**：尺寸有**两个单位**，因为两者的失效方式不同——长函数藏起一条控制流，长文件藏起「东西在哪」。
**文件面是一次重新定价，动的参数写在这里**：R2.20 当时的判断是「按文件计的任何诚实阀值会在四个 crate 里同时点燃八处，那是一个工程而不是一道门」，那个读数当时为真。2026-09-02 人裁定要做这个工程，理由是最大的一个文件已经 12,078 行，在它上面迭代的代价已经超过拆分一次的代价。于是阀值带着一张**先于规则存在的文件登记表**上线（`[file_length.predating]`），每个文件钉在划线当天的行数上。**这张表只会变短**：表上没有的文件直接按预算拒绝，所以它不会变长；表上的文件不得超过自己的钉子，所以没有一个欠债会长大；而一个回到预算之内的文件必须从表上划掉，所以豁免会自己消失，不需要谁记得它。**删一行的办法是把文件拆了，不是把数字改大。**
   **参数面（V3.29）**：一条参数表长过 4 就是一个 data clump——总是一起走的那几个值，是一个还没被命名的值。
本仓库已经写下过这个修法：`Reporter` 的 doc 说「四个值总是一起走、从不被单独选择，所以它们作为一个走」。
预算取 4 而不是 Clean Code 的 3，因为三字段值的构造函数正当地需要三个，门不该跟它们吵。
接收者不算：`&self` 是这个函数之所以是方法的原因，不是谁决定要穿过去的值。
豁免表是一张名字数组（`文件路径::函数名`），**表上没有的名字直接拒绝**，划掉一个名字的办法是给那几个值起个名字，不是把预算调大。
一条断言核对表上每个名字仍然存在且仍然超标，所以一个已经修好的豁免不会留在那里等下一个人花掉。
   **四十二条里有十六条在同一个文件，最长的六条全在它**——这与文件面从另一个方向得到的是同一个发现：
`bin::assembly` 的模块表行写着 `adapter`（§9：薄、无策略），而它装着这座城的派活策略。
**一个十一参数的私有方法，就是策略没有对象可住时的样子。**
   **文件面数测试**，函数面不数：一个长测试与一个长函数体是两个问题，但一个来找东西的读者要为它上面的每一行付钱——引出这条规则的那个文件里，6,133 行是测试。**扫描面**：`crates/*/src`、`xtask/src`、`citysim/src`；`tests/` 与 `benches/` 不在内，因为测试代码本就允许放松约束（AGENTS.md）。**三类不量**：① 带 `#[cfg(test)]` 的项（它标的是**一个项**而不是文件剩下的部分）；② 带 `#[component]` 的函数（Dioxus 组件，函数体即标记，没有可跟的步骤）；③ 模块表形状列为 `data` 的文件（ARCH §9 形状 6：数据而无分支）。**三类豁免都取自已有权威**（属性、模块表），而不是新建一张名单——一张名单就是一个可以您您变长的豁免口。形状列由 `modmap::shapes` 交出，与 modmap 共用同一个表解析器。
10. **报告**：three-part 渲染，与产品的 Gate 拒绝同构——施工者被拒时拿到的也是「规则｜违反点｜替代」，不是一句 fail。

## 11 边界枚举

词汇表粗体词一个都解析不出（表结构变了→ Doc 错误而非静默通过）；空仓库（无提交→guard 跳过）；表行路径重复；状态列取值非法；围栏块缺失（→ Doc 错误，非零违规）；CRLF 行尾（比对前 trim `\r`）；非 UTF-8 文件（lossy 读，不 panic）；merge 提交（diff-tree -r 照常）；initial commit（`--root`）；**命令面的注释行不参与 ax token 扫描**（`#`／`//` 打头的行不可执行，把它们当命令判是把说明文字当成了行为——首跑即被自身 CI 注释命中的实例回填此条）。

## 12 错误处理

`XtaskError`（thiserror）：`Io{path}`｜`Doc{file,msg}`（数据面不可解析）｜`Cmd{cmd,msg}`（git/cargo 调用失败）｜`Usage`。数据面坏＝退出码 2（门自身故障），不伪装成 0 或 1——门坏了必须显性，静默通过是门的最坏失效。

**一门判不动，不得连累其余各门的结论**（issue #5）。`gates` 的那张数组是急切求值的，十六门在第一行输出之前就已全部跑完；此前的循环一遇 `Err` 即 `return`，于是排在它后面的 `release` 与 `guard` 结论已在手里却从未被打印。缺 `cargo-public-api` 是 `docs/CONTRIBUTING.md` §7 明列的预期状态，而在那种机器上，一次带违规的运行与一次干净的运行输出逐字相同，承重的 `guard` 恰在被吞掉的那两道里。故聚合运行遍历到底，逐门报出 `ok`／`N violation(s)`／`could not judge` 三态之一，再统一渲染全部违规。**退出码取最重的一态**：任一门判不动＝2，否则有违规＝1，否则 0——判不动压过判有罪，因为「没判」与「判过且干净」同形正是本条要拆开的东西。

## 13 依赖选型

serde＋serde_json（cargo metadata 解析；工作区已钉）；toml（lexicon 数据面；xtask 独用，不入产品面）；thiserror（工作区已钉）；kernel（S2.12 起：secret 门复用 `kernel::secret::scan`，一个判定一个家）。不引 walkdir/regex/clap：手写遍历十几行；判定用子串与前缀即可（C12 对 regex 的敏感面在 kernel，此处一并回避）；子命令分发一个 match 足矣。

**syn 与 proc-macro2**（R2.20，`syn` 开 `full`，`proc-macro2` 开 `span-locations`）：**量一个 Rust 函数从哪行到哪行是一个解析问题，不是一个数括号问题**。本卡先写了一个按行数括号的探针，一小时内撞上三个计数错误，**每一个都产出了一张错的违规名单**：① `#[cfg(test)]` 被当成文件截断点，于是 `assembly.rs` 第 5046 行一个测试助手以下的函数全部隐形（`pub async fn serve` 就在里面）；② `'{'` 这样的字符字面量被当成开括号，`detect` 于是从 43 行变成 266 行；③ 跨行字符串（`"… \` 换行 `…"`）同理，`malformed` 从 6 行变成 230 行。一道量错的门比没有门更坏：它会把人送去拆一个不需要拆的函数。替代方案是手写一个状态扫描器（行注释、可嵌套块注释、转义与跨行字符串、raw string 的 `#` 计数、以及 `'a` 生命期与 `'x'` 字符的区分）——八十行代码养一个第四个计数错误的地方。`syn` 是编译器旁的那个解析器，且已因每一个 derive 宏而在 `Cargo.lock` 里。维护成本：仅工作区工具链，恒不入产品二进制（同 flate2／zip 先例）。

**zip**（P7.02，`default-features = false, features = ["deflate-flate2"]`，净增两个包）：复用 xtask 已有的 flate2 做压缩后端。替代方案是在 justfile 与 CI 里按平台分支调 `Compress-Archive`／`zip`／`tar`，已验证否决：git-bash 携的是 GNU tar，不产 zip，三个平台因此需三段 shell，且本机与 CI 的产物不同源——那正是本轮要关掉的那类差异。维护成本：仅工作区工具链，恒不入产品二进制。

## 14 硬编码声明

两个行数预算都**不**硬编码在门里，它们是 `xtask/budgets.toml` 的两行（`[function_length]` 与 `[file_length]`，后者带子表 `[file_length.predating]`）——那份登记表自述是「设计所声明的每一项预算」，而它已经持着非字节的预算（`kernel_mutation` 的百分比、`ledger_append` 的毫秒）。数字的来历写在那一行的注释里，改它受 guard 看守。

MPL 头三行；保护路径清单（xtask/、.github/、deny.toml、Cargo.toml、rust-toolchain.toml、clippy.toml、justfile）；被判源码前缀清单（crates/、citysim/、fuzz/）；状态枚举四值；`ax` 的三个可及属性与四个地标元素。各随其权威变更而改，改动本身受 guard 看守。

## 15 影响面

CI 与 justfile 调用面；ARCHITECTURE.md §6/§2/§3 的表格式即本 crate 的解析契约（列契约已标〔冻〕）。改表格式＝改本 crate。

## 16 测试与约束

单测：modmap 行解析（正例/六列不齐/状态非法/缝表不误伤）；索引文件判定；lexicon 命中与 `lexicon-ok:` 豁免；depmap 块解析；ax 可及面取出（HTML 与 RSX 两种写法归一）；header 比对（CRLF）；隔离区前缀判定（`local/` 命中、`localx/` 不命中）；guard 两边判定（混合提交索裁定、单独重新定价不索、基线同步不误伤）。约束：全门无网络、无写盘（spec 子命令除外——它只新建不覆盖）；输出顺序确定。

## 17 模型体验

零字节：本 crate 不进任何 Run 的 prefix；施工者只在门红时读到 three-part 报告——边界反馈优于开头说教的施工侧实例。

## 18 文档同步

新增门或改保护路径时：AGENTS.md 的规则表、`docs/CONTRIBUTING.md` §3 同集更新。

### 第十三道门：`ax`（V3.15）

**它抓的是真发生过的那种漂移。** 四步法在 HTML 里定稿、翻译、只补绑定——而第三步丢掉一个 `role`、一个 `aria-label` 或一个 `aria-current` 是隐形的：页面照样渲染，像素照样对，丢掉的只是「一个看不见像素的人本来会被告知的东西」。这个仓库里没有任何别的东西会注意到。

**它不是计算出来的无障碍树，也不声称是。** 计算树要浏览器，浏览器要一个本构建不发布的二进制，而一道跑不起来的门就是一道不再跑的门。它比的是**被写下来的**那部分：角色、可及名、当前页标记、地标元素。这些正是翻译会丢的那些。计算树仍然值得对着运行中的客户端查一次，那是一个人开着浏览器的活，不是一道门的活。

**名字按存在比，角色按值比。** 可及名是内容，会被翻译：定稿屏用中文写它，客户端从 `web::lang` 取。逐字比会逼客户端把定稿屏的中文硬编码进去，而那正是短语表存在要防的缺陷。`role` 与 `aria-current` 取自封闭词汇、永不翻译，所以按值比——把 `role="img"` 写成 `role="button"` 是真缺陷，按存在比会放它过去。

`crates/web/screens/` 不存在时这道门什么也不说：那是写第一张定稿屏之前仓库的样子，一道会因此变红的门必须先被关掉才能开工。

### 第十五道门：`render`（V3.48）

**它补的是四步法从来没有的那一步：真的把屏画出来。** 上一节那句“计算树是一个人开着浏览器的活”在实践里的含义是：没有人干过。于是一整类缺陷无人看管——**层叠里的冲突在两份源码里都不存在**。两条各自读起来都对的规则把派活框排成一行，又给每一页添了第二条左边，而十四道门与 1,338 条测试全绿。

**它断的是性质，不是图片。** 截图对比会被一次字体 hinting 弄红，却放过一个没人拍过的错版面。三条断言各自是一个已经发生过的缺陷的一般形式：

1. **一页只有一条左边**——中栏里每个区域的 x 相同。
2. **面板头是它自己面板的顶部**，且从面板的左边开始。
3. **没有东西比装着它的区域更宽。**

**找不到浏览器是 skip，不是红。** 引擎不在仓库里也不能在；门在自己打印的那一行里说出它没看，而不是假装看过。`SPRAWLING_BROWSER` 可以点名一个。`target/screens/tokens.css` 未写时同样 skip：那是 `cargo test -p web` 的产物，而缺一个生成文件不是版面漂移。

**它自己的消融实验已经做过**：把 `.panel { display: block }` 删掉，两张屏报“面板头不在顶部”；把脊线那一列换回逐孩子居中，`session.html` 报“两条左边”。两次都变红，恢复后都变绿。

### 扫描面：构建目录不在里面（V3.53，带人的裁定）

`walk::SKIP_DIRS` 从三个名字变成四个，新的那个是 `dist-newstyle`。
**一道门为已提交的对象作证**，而构建目录里一个都没有；`.gitignore` 逐个点过它们的名。
对抗性检查器落地后 cabal 开始把包数据库写在源码旁边，于是 `secret` 从里面读出 **108,472 条**、`release` 再读出 8 条——
全部是关于生成文件的真命题，而那些文件没有任何读者会收到。**一道报出十万条的门等于什么都没报**，
因为没有人会读到第十万零一条。改后 `secret` 归零，`release` 剩下一条——那一条在一份**被跟踪的**文件里，是真的。

这张表是「树里有什么」的第二个权威，git 是第一个；它继续做一张表而不去读 `.gitignore`，是因为四个名字值四个 token 而一个解析器值一个解析器。
**这就是它的重新定价参数：哪天这张表需要第五行而那一行不是构建目录，就去读忽略文件，不要再添一行。**

本改动动的是门自身，故它与被判源码同处一枚提交时携 `Verdict:` 尾注（AGENTS.md guard 行）。

### 第十六道门：`wording`（V3.51）

**它读的方向与 `web::lang` 那两条断言相反。** 那两条读的都是「视图向表要了什么」：一条要求每一条短语都有视图叫得出名字，另一条禁止视图把表里已有的句子再拄一遍。**一句从不调用 `say` 的字面量不在它们任何一条的视野里**：表不知道它存在，也就没有东西可比。成本页上三句英文就这样活过一整段（web-SPEC §8-61），而发现它们的是一张截图。

**判据是位置，不是词表。** 扫全部字符串字面量的那个版本会去判类名、线上取值、事件名与格式键，而同一形状的错误曾经一次性报出 79 条全是地址的命中（V3.28）。所以它走一遍 RSX 的花括号结构，只留两个位置，而那两个位置各自都是「读者被递了一个词」：

1. **文本节点**——单独站在元素体里的字面量，浏览器把它画出来；
2. **朗读型属性的值**——`placeholder`、`title`、`alt` 与值为作者文本的那几个 `aria-*`，屏幕阅读器把它念出来。

其余全部由**坐在哪里**排除，而不由一张例外名单排除：属性值不是文本节点，调用参数不在元素体里，`match` 的臂是模式而不是内容（`rsx!` 是回到内容的唯一入口）。整个客户端 307 处递给读者的字面量里，这条位置规则只留下 18 条待判。

**去掉城自己的值之后剩下的那部分才是问题。** `"{percent}%"`、`"+{added}"`、`"{room}/"` 没有递给读者任何视图写的东西；`"{count} waiting"` 递了一个英文词。故插值槽与 `\u{...}` 转义先取出，再问剩下的部分里有没有相邻的两个字母。

**专名进不了短语表，所以它必须有一扇门。** `web::lang` 自己的断言 `assert_ne!(said.zh, said.en)` 拒绝一条两种语言相同的短语，而 `openai` 在两种语言里就是 `openai`。这五处用 `wording-ok: <理由>` 写在本行或上一行——**同一个拼法、同一条两行规则、同一笔交易，照搬 `lexicon-ok:`**。一个可见的、带理由的现场标记，比一张没人会去读的 toml 名单诚实。

**它自己的消融实验就是它存在的理由**：把 V3.50 那三句英文原样写回 `dashboard.rs`，本门变红并点名行号，而 `web::lang` 那两条断言全程全绿；改回译文，本门变绿。单测 `the_three_sentences_that_escaped_both_of_langs_assertions_are_caught` 把这次实验固定下来。
### 8-1 xtask::color 目录化（card-5.2）

`color.rs` 一文件 819 行，切成一个目录，五个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/color.rs` | 六条令牌断言与可读性断言本身（`check`、`judge_tokens`、`judge_readability`、`token_violation`），以及色轴、灰阶与明度上下界这几个常量 |
| `xtask/src/color/tables.rs` | 怎么从 `web::theme` 的源文件读出四张表（`GRAY_RAMP`、`COLOUR_TOKENS`、`TEXT_TOKENS`、`TYPE_SCALE`）与 `TEXT_SURFACE_CEILING` |
| `xtask/src/color/contrast.rs` | 一对令牌的 APCA 对比度是多少（`apca_lc` 及其 OKLCH→sRGB 链路），以及 Bronze Simple Mode 允许某个字号使用哪一层（`bronze_tier`） |
| `xtask/src/color/scan.rs` | 全仓扫描：什么算一个颜色字面量（`literal_at`、`hex_colour`），扫哪些文件（`scan_for_literals`） |
| `xtask/src/color/tests.rs` | 原内联 `mod tests` 原样迁出，14 个测试一个不少 |

**无字段开放**：跨文件引用只用 `pub(super)` 函数；`grey_ramp` 因 `badge` 门经 `color::grey_ramp` 调用而在索引位置以 `pub(crate) use` 重导出，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**`SCAN_EXEMPT` 随文件而动，判据未放宽**：自豁免的理由一直是「检测器必须拼得出它所禁的东西」，而现在拼出颜色语法的是 `color/scan.rs`（拼法表）与 `color/tests.rs`（用例），故豁免名单改点这两个文件；不再拼颜色的 `color.rs` 本身则回到被扫范围内——豁免面因此变窄而不是变宽。

### 8-2 xtask::wording 目录化（card-5.2）

`wording.rs` 一文件 686 行，切成一个目录，四个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/wording.rs` | 门本身：扫哪里（`CLIENT`）、什么算豁免（`EXEMPT_MARK`、`waived`）、哪些属性会被读出来（`SPOKEN`）、报告怎么写（`check`、`clipped`、`Said`）、以及判据「去掉城自己的值之后还剩不剩两个相邻字母」（`words_the_view_wrote`）；测试模块的切口 `drawn` 也在这里 |
| `xtask/src/wording/lex.rs` | 一段 Rust 源码切成哪些词（`Kind`、`Lexeme`、`lex` 及其字符串、原始字符串、字符字面量与注释的读法） |
| `xtask/src/wording/rsx.rs` | 花括号栈怎么走，一个字面量坐在哪里（`Frame`、`handed_to_a_reader`、`seat_of`、`step`、`opens_an_element_body`） |
| `xtask/src/wording/tests.rs` | 原内联 `mod tests` 原样迁出，7 个测试一个不少 |

**无字段开放**：跨文件只开了 `lex.rs` 的 `Kind`／`Lexeme`（含三个字段）／`lex` 与 `rsx.rs` 的 `handed_to_a_reader`，一律 `pub(super)`；`Said` 与 `SPOKEN` 留在索引位置，子模块按父模块私有项直接引用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**判据未放宽**：位置规则、豁免的两行范围与相邻两字母的门槛逐字节照搬，只换了它们所在的文件。

### 8-3 xtask::render 目录化（card-5.2）

`render.rs` 一文件 581 行，切成一个目录，三个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/render.rs` | 门本身：扫哪里（`SCREENS`、`TOKENS`）、对齐容差（`SLACK`）、一个被量出来的盒子是什么（`Box` 及 `right`／`name`／`drawn`）、跳过与判断的次序（`check`），以及三条性质的判据（`judge`、`one_left_edge`、`heads_lead_their_panels`、`head_leads`、`nothing_overflows`） |
| `xtask/src/render/engine.rs` | 怎么把一张屏真的画出来并把盒子读回来：找引擎（`browser`、`on_path`）、工作目录与视窗（`WORK`、`VIEWPORT`、`SINK`）、渲染一张屏（`Engine`、`Engine::new`、`Engine::measure`）、改写样式表链接并附上探针（`instrument`、`url_of`、`PROBE`）、把探针写下的记录读回来（`sink`、`parse_box`） |
| `xtask/src/render/tests.rs` | 原内联 `mod tests` 原样迁出，5 个测试一个不少 |

**无字段开放**：`Box` 及其三个方法留在索引位置按父模块私有项定义，`engine.rs` 作为子模块直接引用；跨文件只把 `Engine`／`Engine::new`／`Engine::measure`／`browser`／`sink`／`parse_box` 提到 `pub(super)`。`main.rs` 经 `render::check` 调用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**判据未放宽**：三条性质的文字、`SLACK` 的 1 像素、找不到浏览器与缺 `tokens.css` 时的 skip 逐字节照搬，只换了它们所在的文件。

### 8-4 xtask::release 目录化（card-5.2）

`release.rs` 一文件 578 行，切成一个目录，三个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/release.rs` | 门本身：什么留在机器上（`SCAFFOLDING`、`is_scaffolding`、`published`）、什么形状算家目录（`HOME_SHAPES`、`machine_path`）、哪些文件自豁免（`DETECTORS`）、什么算一处引文（`CITED_EXTENSIONS`、`path_tokens`、`directory_names`、`outside_the_tree`、`is_prose`），以及四条断言的编排（`check`） |
| `xtask/src/release/link.rs` | 一份文档叫读者去开哪些路径，那些路径落在树的哪里（`link_targets`、`resolve`），二者一律 `pub(super)` |
| `xtask/src/release/tests.rs` | 原内联 `mod tests` 原样迁出，9 个测试一个不少 |

**无字段开放**：跨文件只把 `link_targets` 与 `resolve` 提到 `pub(super)`，索引位置以私有 `use link::{link_targets, resolve};` 引回，`check` 与测试的调用点一字未改；`main.rs` 经 `release::check` 调用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**`DETECTORS` 随文件而动，判据未放宽**：自豁免的理由一直是「检测器必须拼得出它所禁的东西」，而写出家目录形状用例的现在是 `release/tests.rs`（三条「绝对路径被拒」的断言必须各写出一个），故名单从两条增到三条，新增的正是那份迁出的测试文件。被扫面因此少了一份测试文件而已，`HOME_SHAPES`、`CITED_EXTENSIONS`、`SCAFFOLDING` 与四条断言的文字逐字节照搬。

### 8-5 xtask::length 目录化（card-5.2）

`length.rs` 一文件 573 行，切成一个目录，三个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/length.rs` | 门本身：预算行名（`ROW`、`FILE_ROW`、`ARG_ROW`、`PREDATING`）、被量的源目录（`SOURCE_DIRS`、`sources`）、登记表的读法（`limit`、`predating`、`excused`、`key`）、五种违规的措辞（`too_long`、`grew`、`no_longer_an_exception`、`too_many_arguments`、`over`），以及编排（`check`） |
| `xtask/src/length/measurement.rs` | 一个函数有多长、带几个参数，从解析出的项读得而不是从它周围的文本读得（`Found`、`measure`、`found`、`skipped`） |
| `xtask/src/length/tests.rs` | 原内联 `mod tests` 原样迁出，8 个测试一个不少 |

**无字段开放**：`Found` 及其四个字段保持原有的 `pub(crate)`，跨文件只把 `measure` 提到 `pub(super)`，索引位置以私有 `use measurement::{Found, measure};` 引回；`found` 与 `skipped` 仍是 `measurement` 内的私有项。`check` 与全部测试的调用点一字未改，`main.rs` 经 `length::check` 调用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**判据一处未松**：三种豁免（`#[cfg(test)]`、`#[component]`、模块表 shape `data`）、`grew`／`no_longer_an_exception`／陈旧钉子三条自清理断言、以及两个预算仍只从 `xtask/budgets.toml` 读来，文字逐字节照搬。`[file_length.predating]` 里 `"xtask/src/length.rs" = 573` 一行按规则划掉——切分做完，钉子即失效；`length.rs` 现在与其他所有文件同受 400 行预算约束。**登记表上方那段注释仍写着「`xtask/src/length.rs` 在表上，这是对的：立规的门不豁免于规」，我没有动它**：guard 门把该注释的改动视为门面改动，而它所说的道理未变——立规的门仍受这条规约束，只是它现在直接受预算约束而非受钉子约束。

### 8-6 xtask::guard 目录化（card-5.2）

`guard.rs` 一文件 508 行，其中 151 行是内联 `mod tests`。按刀法第一条只做测试迁出，切成两个文件：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/guard.rs` | 门本身：保护面（`PROTECTED_PREFIXES`、`PROTECTED_FILES`、`PRODUCED_PREFIXES`、`REGISTER`、`is_protected`）、被判面（`JUDGED_PREFIXES`、`is_judged`）、两边的纯函数（`gate_faces`、`judged_faces`）、登记表划行的 diff 形状（`Row`、`exemption_row`、`strikes_only_exemptions`）、模块行移除的路径级判定（`deletes_module_row`、`row_path`），以及区间编排（`check`）与 `apisync` 共用的 git 读法（`changed_paths`、`changed_paths_with_status`、`git_text`、`git_lines`） |
| `xtask/src/guard/tests.rs` | 原内联 `mod tests` 原样迁出，9 个测试一个不少 |

**无字段开放**：`tests` 是 `guard` 的子模块，`use super::{gate_faces, is_protected, judged_faces, row_path, strikes_only_exemptions};` 一字未改即可看见父模块的私有项，故没有一个项因这次切分而放宽可见性。`main.rs` 与 `gates.rs` 经 `guard::check`、`apisync.rs` 经 `guard::git_lines` 与 `guard::changed_paths_with_status` 调用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**判据一处未松**：`TRAILER`、两张保护表、被判前缀表、`strikes_only_exemptions` 的两种缩小形状与 `deletes_module_row` 的删增判定逐字节照搬，拒词三段（rule／violation／alternative）同样逐字节照搬；父文件尾部只多出 `#[cfg(test)]` 与原有的 `#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` 加一行 `mod tests;`，lint 名不增不减。`[file_length.predating]` 里 `"xtask/src/guard.rs" = 508` 一行按规则划掉——这道门判的正是这种划行，而 `strikes_only_exemptions` 认它为「只做除名」的缩小形状，故这次改动本身无需 `Verdict:` 尾注。


### 8-7 xtask::badge 目录化（card-5.2）

`badge.rs` 一文件 436 行，测试迁出后成为一个目录，两个文件各答一个问题：

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/badge.rs` | 徽章本身：登记册里哪些行要徽章（`Plan`、`plans`、`planned`、`platform`）、写与判（`write`、`check`）、调色板与字号（`Palette`、`palette`、`human`、`text_width`、`render`）、以及 OKLCH→sRGB 的一次换算（`srgb_hex`、`srgb_channels`、`channel`） |
| `xtask/src/badge/tests.rs` | 原内联 `mod tests` 原样迁出，7 个测试一个不少 |

**无字段开放**：`tests` 是 `badge` 的子模块，`use super::*;` 一字未改即可看见父模块的私有项（含 `Palette` 的四个字段），故没有一个项因这次切分而放宽可见性。`budget.rs` 经 `crate::badge::check`、`main.rs` 经 `badge::write` 与 `badge::check` 调用，其它文件的 `use` 一行未改。xtask 不入 `apisync`，无基线重写。

**判据一处未松**：三条纪律（颜色取自 `web::theme` 的灰阶、平台自报、陈旧即红）与拒词三段（rule／violation／alternative）逐字节照搬，只换了测试所在的文件；父文件尾部的 `#[allow(...)]` lint 名不增不减。`[file_length.predating]` 里 `"xtask/src/badge.rs" = 436` 一行按规则划掉。

### 命令 `wire-ts`：线的 TS 面由 Rust 面生成（card-6.2）

**它关掉的门是「手写第二份线」。** `client/` 用 TypeScript 说 `crates/channels` 的语言，而一份手写的 `wire.ts` 就是同一形状的第二个权威，它漂了也要到握手之后才被发现。故 TS 面由 Rust 面生成，且生成物入库、门盯着它：`cargo xtask wire-ts --write` 写 `client/src/wire.ts`，`cargo xtask wire-ts` 只比对——盘上文件与当场生成的文本逐字节不同即红，拒词点名文件与第一处不同的行号并给出 `--write`。与 `apisync`／`badge` 同一口径：生成物由门自己写、由门自己校验。

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/wire_ts.rs` | 命令本身：文本从哪来（`render`：`channels::wire_schema()`＋`WIRE_V`＋`schema_hash()`）、写到哪（`TARGET`）、怎么比（`check`、`first_difference`）、怎么写（`write`） |
| `xtask/src/wire_ts/emit.rs` | 一份 JSON Schema 文档怎么变成一份 `wire.ts`（`emit`）：`$defs` 按名排序后按依赖拓扑输出（`ordered`、`refs_within`）、一个 schema 怎么变成一个 Effect `Schema` 表达式（`expression`、`typed`、`fields`、`union`、`literals`）、以及它认得的子集与拒绝（`Refused`） |
| `xtask/src/wire_ts/tests.rs` | 具名裸 `string` 打上 brand；外标签枚举成 `Union`；依赖先于引用；子集外关键字被点名拒绝；环被拒绝；真实文档能发出；第一处不同的行被点名 |

**只认 serde 会产出的那个子集，其余点名拒绝。** 对象（`properties`／`required`／`additionalProperties`）、`string`／`integer`／`number`／`boolean`／`null`、`array`（`items`）与元组（`prefixItems`）、`enum` 字符串表、`const`、`oneOf`／`anyOf`（外标签、内标签、邻标签三种 serde 变体形状都落在这一条上，无需分别特判）、`$ref` 指向 `#/$defs/<名>`、`type: [T, "null"]`、`true`／`false`。`description`／`format`／`minimum`／`pattern`／`minItems`／`maxItems`／`default` 读而不译（`description` 只在顶层定义处作为 JSDoc 发出）。`default` 是 `#[serde(default)]` 字段的注解：字段可缺省这件事由 `required` 一处表达，`Schema.optional` 已据它发出，所以再读 `default` 会造出第二个权威。其它任何关键字（`allOf`、`not`、`patternProperties`……）一律 `Refused`，报出所在类型的路径与关键字——**一个会猜的生成器就是一个会静默产出错类型的生成器**。具名的裸 `string`／`integer`／`number`／`boolean` 即 newtype，打上 `Schema.brand("<名>")`。

**为什么依赖拓扑而不是字母序**：Effect 的 `Schema` 是运行期值，`const B = Schema.Struct({ a: A })` 要求 `A` 已定义；字母序会撞 TDZ。拓扑序内按名字母序断平，故输出确定；环（自引用类型）以 `Refused` 拒绝——线上今天没有一个，出现那天再上 `Schema.suspend`，不预留。

**依赖**：`channels = { path, default-features = false, features = ["schema"] }`——不开 `server`，xtask 不为此拖进 tokio 与 axum；`schemars` 经 channels 的 `schema` feature 到达。xtask 是工作区成员而不占产品拓扑（§7 对 kernel 已用过同一条理由）。

**门的注册留给主线**：进 `gates::run` 那张数组即改 `COUNT`，而 `vocabulary` 门对着 `COUNT` 校正文里手写的门数，那是一次跨文档的重新定价，按 AGENTS.md `guard` 行应单独一枚提交。本卡只交付命令。

### 8-8 color：颜色的产地从一处改为一客户端一处（card-gates·改价；形状 6 数据面）

**改价条件已到**。原规则「`web::theme` 是唯一命名颜色的地方」成立的前提是城里只有一个客户端。card-6.1 起 `client/` 是第二个客户端，card-6.3 把 `client/src/theme.css` 定为它的 `@theme` 令牌块，两个客户端并存到 card-6.11 删掉 `crates/web` 为止。此时按字面执行原规则，会把一个客户端的**唯一产地**判成违规——被判红的不是缺陷，是规则的参数变了。

**权威改述为一句**：颜色在每个客户端里恰好被命名一次。产地表因此是一张具名的封闭表，一行一个客户端：

| 客户端 | 颜色产地 |
|---|---|
| `crates/web`（Dioxus/wasm） | `crates/web/src/theme.rs` |
| `client`（Solid/Vite） | `client/src/theme.css` |

- **扫描判据一字未改**：`literal_at`／`hex_colour` 认得的颜色语法、扫的扩展名、拒词三段全部照旧；改的只是「哪些文件是产地」这一张表。豁免面从三项变四项，且新增的那一项是一个客户端仅有的产地——面变宽一个文件，规则本身没有松。
- **六条令牌断言仍只对 `crates/web/src/theme.rs` 作证**。`client/src/theme.css` 是那张令牌表的**投影**而不是第二个权威：它的每一个值都解算自 `theme.rs`（client-SPEC §3-4 已如此声明），所以给它再写一遍断言就是在两处判同一件事。这也是本次改价没有把 `THEME` 常量拆成两个的原因——`THEME` 是「断言读哪份表」，产地表是「扫描放过谁」，两个问题不是一个。
- **下一次改价的条件写在这里**：card-6.11 删掉 `crates/web` 之日，产地表回到一行，`THEME` 与那一行合并。

### 8-9 secret：门只看人写的文件，派生文件由它的输入作证（card-gates·改价）

**门的主题是「明文凭证进入工作树」**，不是「任何高熵字节串出现在某个文件里」。card-6.1 带进 `client/bun.lock`、card-4.1 带进两份 insta 快照后，门报出 268 条，其中真凭证零条——一个判据碰上它从未见过的文件类，报的全是假阳性。

**判据补一条文件类，而不是补 268 个字节偏移**。逐条列偏移会把一个可判定的类别问题写成一张会腐烂的坐标表，而且下一次 `bun install` 就让它全错。新判据分两类：

| 文件类 | 成员 | 它为什么不可能是凭证第一次进树的地方 |
|---|---|---|
| 生成的锁文件 | `Cargo.lock`、`client/bun.lock` | 每一字节都由包管理器从清单与仓库解算而来，其中的 `sha512-`／`sha256-` 是**已发布产物的完整性摘要**，本就该被任何人读到；人不往锁文件里写东西，写了下一次解算就冲掉 |
| 记录的快照 | `crates/**/snapshots/*.snap` | insta 快照是测试**输出**的留影，它的输入住在被扫的源文件里；一个凭证要出现在快照里，得先出现在那个源文件里，而那一份仍被扫 |

- **一句话的权威**：门扫**人写的**文件；一份**派生**文件的字节来自门已经扫过的输入，所以它不是凭证第一次进树的地方。两类各是这一句的实例，不是两条独立的例外。
- **`.expose(` 白名单那一半不动**：它只看 `crates/*/src/**.rs`，锁文件与 `.snap` 本就不在其面上。
- **已知的限**（写在明处，不静默）：一个从环境变量读真凭证、再把它录进快照的测试，能从这条豁免下走过去。今天树上没有这样的测试，且写出这样的测试本身就是缺陷；真要堵它，堵的地方是「测试不得读真凭证」，那是另一道门的题目，不在本卡范围。
- **为什么不是内联豁免注释**：门自 S2.12 起就没有内联豁免，理由未变——注释是内容能自己写出来的东西，而一张编译进门里的文件类表不是。

### 8-10 模块表的第七列 `Spec`，与数出来的每 crate 计数（card-8.2；形状 1 判定）

模块表回答「这个文件是什么」，却从不回答「它的接口写在哪」。读者要从 `web::live::feed` 走到定义它的那一节，得先猜 crate、再翻 SPEC 的 §8。第七列把这一步写成数据。

**列约定**：`Module | File | What it owns | Shape | Since | Status | Spec`，第七列的值形如 `<crate>-SPEC.md#8-N`，相对该 crate 的 SPEC 目录解析（`bin::*` 的 crate 是 `sprawling`，`desktop::*` 的是 `desktop/desktop-SPEC.md`）。列在末尾，于是形状列与状态列的下标不动，只有单元格数从八变九。**这是模块表列约定的一次变更，按 ARCHITECTURE.md §13 需要人的裁定；裁定即 card-8.2 本身。**

**一节可以答多个模块，一个模块只能答一节。** 子模块跟随它的父模块所在的节，除非某节的标题点名了子模块的全路径（`runtime::tools::read` 有自己的 8-29，故它不跟 `runtime::tools`）。理由是 SPEC 的 §8 按接口分节而模块表按文件分行，两者本就不是一一对应；把子文件各钉到一个不存在的节上，只会造出一列指向虚无的链接。

**specalign 增第三条断言：锚点在盘上存在。** 第七列的每个值都被解析成「SPEC 路径 ＋ 节号」，路径必须可读，节号必须在那份 SPEC 里作为一个节的标号出现。SPEC 的 §8 有两种写法，两种都算：`### 8-N …` 标题（多数 crate），以及 §8 的接口围栏里那一行 `// 8-N …` 注释（`browser`／`protocol`／`desktop`／部分 `web`／`runtime` 的写法）。**认两种不是放宽，而是照着树上真有的形状判**——只认标题会把六个 crate 判红，而它们的 §8 本来就是一整块围栏。

**已知的限，写在明处**：节号在同一份 SPEC 里并不唯一（`sprawling` 的 `8-40`／`8-41`／`8-42` 各出现过三次，`web` 的 `8-12`～`8-16` 各两次），因为不同期的卡各自续号而无人对账。故本条只判存在，不判唯一：加一条唯一性断言会把七份未经重编号的 SPEC 一次判红，而重编号是另一件工作。**翻案条件**：任一 SPEC 的 §8 完成一次重编号后，唯一性断言随即上线。

**modmap 增一条断言：§12 每个小节标题里的数，等于该小节的行数。** 标题写作 `### kernel (73) — …`，括号里的数就是这个 crate 在册的模块文件数；一个标题可以带多组（`### browser (6), protocol (5), bin (111)`），每组按模块列的前缀分别计数。这条不是新规矩而是既有规矩的一次落实：xtask-SPEC §10-5 已经写下「能被机器数出来的数不由文档手写」，而这些数当时没有机器数，于是十三个里有九个是错的。

### 8-11 `package` 认目标三元组：一份产物住哪里，叫什么名字（card-F4.0；形状 2 值）

card-12.1 给发布矩阵加了 `x86_64-unknown-linux-musl` 一行，而 `budget::binary_path` 只认 `target/release`，`--target` 构建落在 `target/<triple>/release`。当时的落法是把静态产物拷到打包器看的位置，再把 `just package` 的步骤在 `release.yml` 里重抄一遍——**一条规则两个权威，明知而为并记在案**（sprawling-SPEC §8 P4.02 改判的「另记一处未清的债」）。本节还这笔债。

**一个具名值答两个问题**：这次构建是为谁构建的。`ReleaseTarget` 住 `xtask/src/package.rs`，两个变体穷举：

| 变体 | 产物目录 | 归档名 | 归档里的可执行文件 |
|---|---|---|---|
| `Host` | `target/release/` | `sprawling-<version>-<os>-<arch>`（不动） | 由 `cfg!(windows)` 判 |
| `Triple(t)` | `target/<t>/release/` | `sprawling-<version>-<t>` | 由 `t` 里是否含 `windows` 判 |

- **`binary_path` 从 `budget` 迁到 `package`**。「产物住哪里」与「产物叫什么」是同一个事实的两半，分住两个模块就是两个权威；`budget` 反过来向 `package` 要路径，因为它的活是称重而不是定位。
- **三元组进名字，是人的裁定而不是本卡的选择**。不进名字的话，musl 归档会叫 `sprawling-<version>-linux-x86_64.zip`，既不说静态也不说 musl，且**在出现第二份 Linux 产物（gnu）的那一天静默相撞**。既有的两个名字一字不动，故这不是重命名而是给新的一类命名。
- **`--target <triple>` 由 `main` 解析**，与 `--range` 共用一个取值函数：两个旗标两份解析就是两种取值语义。
- **`release.yml` 的重抄步骤随本节删除**，三行矩阵走同一步 `just package ${{ matrix.target }}`；`just dist` 收下同一个可选参数，并在有三元组时**不写徽章**——README 的徽章描述一个人首先下载的那份产物，由第二个平台改写它会让一个 tag 的两次构建对同一个数字各执一词。
- **本节属门禁机具，与产品代码分开提交。**

### 8-12 `npm`：`client/` 的依赖面（card-F3；形状 1 判定）

`client/` 于 card-6.1 进树，而看守它的那道门没有跟着进来。工作区那一侧的依赖面由 `cargo-deny` 与 `depmap` 两道门看着，JavaScript 那一侧当时什么都没有：一次 `bun add` 就能把第三个运行时依赖、一个 GPL 的包、或者一份与 `package.json` 已经对不上的锁文件带进来，而全绿的一次 `just check` 一句都不会说。

**三条断言，各修一种真实的漂移**：

1. **锁文件在盘上，且与清单逐条同。** `client/bun.lock` 的 `workspaces` 块记着 bun 上次解算时看见的 `dependencies` 与 `devDependencies`；`package.json` 记着今天要的那份。一处不同就说明有人改了清单而没有重解，于是本机装出来的东西与 CI 装出来的东西不是同一棵树。判据是**两张表逐键逐值相等**，缺、多、值不同各报一条。
2. **运行时依赖恰为 `solid-js` 与 `effect`。** 这是 client-SPEC §1 已经写下的那条界线的机器面：devDependencies 随工具链自由变动，而进到用户浏览器里的东西是一张封闭的两行表。**恰为**而不是**至少**——一个只查白名单不查缺失的门，会放过「solid-js 被误删」这一半。
3. **树上每个包的许可证都在准许表内。** 准许表**不是本门新写的**，它就是 `deny.toml` 的 `[licenses] allow`：一个仓库对许可证只应有一个立场，工作区那一侧已经把它写下来了，本门读同一张表。许可证从 `client/node_modules/<包>/package.json` 的 `license` 字段读得——锁文件不带许可证，而已装的树带。

**`node_modules` 不在树上时，第三条 skip 并说出理由，前两条照判。** `node_modules` 是 `.gitignore` 里的名字，一台没有跑过 `bun install` 的机器上它不存在，而**这不是缺陷**；门在自己打印的那一行里说它没看，与 `render` 缺浏览器时同一口径。前两条只读入库文件，故在任何机器上都判得动——**一道会因为环境而整体沉默的门，就是一道在 CI 之外不再存在的门**。

**`client/` 整个不在树上时本门什么也不说**，与 `ax` 见不到定稿屏时同一口径：card-6.11 之后这一段的形状会再变，而一道会因为目录不存在而变红的门必须先被关掉才能开工。

| 文件 | 它回答什么 |
|---|---|
| `xtask/src/npm.rs` | 门本身：扫哪里（`MANIFEST`、`LOCKFILE`、`MODULES`、`PERMITTED`）、运行时白名单（`RUNTIME`）、三条断言（`check`、`judge_lockfile`、`judge_runtime`、`judge_licences`）与它们的拒词 |
| `xtask/src/npm/lockfile.rs` | 两份清单怎么读成同一种形状：`bun.lock` 是带尾逗号与注释的 JSONC，故先归一再交给 `serde_json`（`read_jsonc`、`Manifest`、`manifest_of`、`lock_of`）；`deny.toml` 的准许表怎么读（`permitted`） |
| `xtask/src/npm/tests.rs` | 尾逗号与注释被归一掉；清单与锁文件不同即报；运行时依赖多一个或少一个各报一条；不在准许表上的许可证被点名；`node_modules` 缺席时第三条不产出违规 |

**`license` 字段的两种形状都认**：一个字符串（`"MIT"`），或一条 SPDX 表达式里的 `OR`／`AND` 分支（`"(MIT OR Apache-2.0)"`）。表达式按 `OR` 拆开，任一分支在准许表内即通过——这与 `cargo-deny` 对同一种表达式的判法一致，故两侧不会对同一个包各执一词。旧包偶尔写 `licenses: [{type: ...}]`，本门**不认**并按「没有说」处理：报出来让人去看，比猜一个字段的历史写法安全。

**它上线第一跑就红了两条，而我没有把它们放过去（card-F3，待人裁）**：`caniuse-lite` 是 `CC-BY-4.0`，`minimatch` 是 `BlueOak-1.0.0`，两者都由 devDependencies 传递带进来，都到不了用户的浏览器。修法在因不在果——要么 `deny.toml` 的 `[licenses] allow` 各加一行并写明理由，要么换掉那两个包。**这一步我不做**：AGENTS.md 的 `guard` 行禁止在一道门变红的那一次改动里放宽这道门，而准许表是这个仓库对许可证的立场，立场归人。已有先例可循——`CDLA-Permissive-2.0` 当初正是为一份证书清单这种**数据**许可证入表的，而 `CC-BY-4.0` 覆盖的 `caniuse-lite` 同样是一张数据表。

**裁决已下，两行已入表（deny.toml，独立提交）**：`BlueOak-1.0.0` 与 `CC-BY-4.0` 各加一行并写明理由。依据三条——两者都是 devDependencies，到不了用户的浏览器；`CDLA-Permissive-2.0` 为一份证书清单入表是同一形状的先例，`caniuse-lite` 同样是一张数据表，而 `just dist` 写出的物料清单正是 `CC-BY-4.0` 要求的署名落点；`BlueOak-1.0.0` 于 2022 年 3 月经 OSI 审议通过，宽松，且授予 MIT 未言明的专利权。放宽发生在门自己的提交（card-F3）**之后**的另一条提交里，属 AGENTS.md 准许的「在自己的提交里重新定价一条规则」，而非它禁止的「在门正卡着的那次改动里放宽它」。此裁可推翻：删掉那两行，红的就是本节下面那道门。

**「换掉那两个包」这一条已被走查关闭（repair-F 复核）**：已装树上是 `minimatch 10.2.6` 与 `caniuse-lite 1.0.30001810`，两者各自的来路都无可替换处——`minimatch ^10` 由 `eslint 10.10.0` 自身、`@eslint/config-array 0.23.5` 与 `@typescript-eslint/typescript-estree 8.70.0` 三处同时要求（`10` 之前的 `minimatch` 是 `ISC`，但降版就是降掉 eslint 10）；`caniuse-lite` 由 `browserslist 4.28.9` 要求，而 `browserslist` 由 `@babel/helper-compilation-targets` 经 `vite-plugin-solid` 带进来，即 Solid 的编译链本身。**两条来路都落在冻结的前端工具链上**，换包等于换掉 eslint 与 Solid 的构建路径。于是留给人的只有一件事：`deny.toml` 的 `[licenses] allow` 加不加这两行。

**本节属门禁机具，与产品代码分开提交。**

### 8-13 `ax`／`wording`／`render` 读画出来的 DOM

**裁定已下（D54，2026-09-11），下文“堵在哪里”一节作废，保留作为记录。** 用户原话：「够到真实浏览器直接拉起默认浏览器就行，或者用 doctor 里面安装的 firefox」。

这句话推翻的不是三条路里的哪一条，而是它们共同的**前提**——卡片要求「复用 `bin::browser_bidi` 的传输，不得开第二条驱动浏览器的路」。拉起一个无头浏览器并让它吐出 DOM **不需要 BiDi**：`render` 今天已经在这么做（`--headless=new --dump-dom`），无套接字、无子进程会话、无产品代码依赖。三条路各自的代价因此都不用付。

**引擎按三级取，每一级都是一条已有的权威，不新立第二份名单**：

1. `SPRAWLING_BROWSER` 点名的那一个（不变）；
2. **doctor 装到 `~/.sprawling/components/firefox/` 的那一个**——读的是 `components_dir()` 这条**文件系统约定**（kernel-SPEC §8-22 P4.02 已记），与 `xtask budget` 读 `target/` 同性质，不是对「这台机器上 Firefox 在哪」再写一份探测；
3. 三个桌面自带浏览器的固定路径（不变）。

**另一条改判：`ax` 不再是单独的一道门。** 它存在的全部理由写在自己的模块头里——「这不是一棵计算出来的可及性树，也不自称是；一棵计算树需要浏览器，而一道离线跑不了的门就是一道不会再跑的门」。现在门能进浏览器了，那条妥协就到期了：比两侧**写下的**东西是在没有浏览器时的替代品，而不是一件値得单独保留的事。角色、可及名与地标改从画出来的 DOM 上读，并入 `render`。同时消失的还有 `crates/web/screens` 这个概念本身：D43／D44 重写了全部屏幕，`dx translate` 不再存在，「定稿屏幕」没有左手边可比。

**skip 的理由仍须各自点名**：没有画廊路由、没有构建产物（`target/web-dist/`）、这台机器上没有引擎——三种各说各的。一道找不到东西就悄悄变绿的门，仍然是这里要避的失效。

### 8-13-1 （已作废）当时堵在哪里（card-F3）

**目标未变**：三道门今天读的是两侧**写下来的**东西，它们应当改为在真引擎里打开 `#/gallery` 这条路由、读**画出来的** DOM——`ax` 比角色、可及名与地标，`wording` 比每个文本节点是否出自 `lang` 表，`render` 比一条左边、面板头在顶、无溢出。画廊不在时是一次**点名理由的 skip**，在时必须真判；一道找不到东西就悄悄变绿的门，正是这里要避免的失效。

**本卡停在一个我无权自己裁的取舍上，故一行未写**，写在这里而不是写成一个半成品：

- 卡片要求**复用 `bin::browser_bidi` 的传输，不得开第二条驱动浏览器的路**。那条传输是 `crates/sprawling/src/browser_bidi/` 里的 `BidiSocket` 与 `LazyEngine`，它们是 `sprawling` 这个 crate 的**私有模块**；`crates/browser` 按 browser-SPEC §19-1 的记录**恒不持套接字、恒不起进程**。
- 于是 xtask 要够到它，只有两条路，两条都动产品代码：①把 `sprawling::browser_bidi` 提为 `pub` 并让 xtask 依赖 `sprawling`（代价：`cargo xtask` 从此要构建整条产品图，含 tokio 与 axum；且 `sprawling` 的公开面变了，`apisync` 基线与 sprawling-SPEC 须同集更新）；②把套接字迁进 `crates/browser` 的一个非默认 feature（代价：推翻 browser-SPEC §19-1 记下的那条决定，须先改记录）。
- 而本卡同时声明**门禁机具与产品代码分开提交**。两条要求在此相撞，**这不是我能自己选的一边**。

**请人裁的正是这一件事**：走 ①、走 ②，还是允许 xtask 自己实现一个 `BrowserPort`（那是卡片明文禁止的第三条）。裁定落下之后，剩下的工作是有界的：一个 `xtask::gallery` 模块持「画廊在不在、在哪里被打开」与一次探针，`ax`／`wording`／`render` 各自只多一个读 DOM 的判据。

**另有一件已知的前置**：`#/gallery` 由前端会话构建，且它要被打开就要有一份**构建好的 bundle**（`client/dist/`）与一个静态服务。skip 的理由因此至少有三种，各须点名：没有画廊路由、没有构建产物、这台机器上没有引擎。

**本节属门禁机具，与产品代码分开提交。**
