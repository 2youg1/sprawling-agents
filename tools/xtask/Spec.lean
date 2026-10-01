-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import tools.xtask.spec.Guard.Wall
import tools.xtask.spec.Length
import tools.xtask.spec.Packaged
import tools.xtask.spec.Report

/-! # xtask 的规格：构建门（gates）

`tools/xtask/` —— 包 `xtask`（工作区成员，`role = "tool"`，不占产品拓扑）。本规格先于代码存在；
实现不多不少地遵守本文。

本文件是规格的入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。
能写成定理的性质在分部里证明：一次 `gates` 运行的结论（`spec/Report.lean`）、`length` 的文件面
（`spec/Length.lean`）、`guard::wall` 的逐键比对（`spec/Guard/Wall.lean`）、`packaged` 判哪些项进不了
生产（`spec/Packaged.lean`）。本文件的十七节记录其余的要求、理由与决定。

决定写作 `D<n>`，别处引作 `xtask D<n>`；它站在它规定的声明之上，或站在它主题所在的那一节里。
号来自这份规格在 Markdown 时 §12 的编号条目，号不变；D12 空着，不复用。§8 的编号条目保留它们在
Markdown 时的标签 `8-n`，别处引作 `tools/xtask/Spec.lean §8-n`。
-/

/-! ## 1 需求分解

把 ARCHITECTURE.md 与 AGENTS.md 里能由机器判定的规则变成检查。门的名册与次序只住 `gates::GATES`（§7），下表只说每道门判什么，另列几条不是门的命令：

| 单元 | 一句话 |
|---|---|
| header | 每个 `.rs` 与 `.zig` 开头恰是 MPL-2.0 通告三行加版权一行，四行逐字节相等，且整份文件只出现这一次（§14、§8-48） |
| lexicon | Markdown、Rust 源码、Lean 与 `client/src/lang.json` 里的退役词命中即红；退役词表是 `tools/xtask/lexicon.toml` |
| modmap | 每个包（§8-39，工具包 citysim 在内）目录下的 `src/**/*.rs` 与每个 `.zig`（§8-48）↔ `architecture.toml` 里文件落在这些目录下的条目一一对应；状态一致；`owns` 非空；索引文件零逻辑。本门所在的包 xtask 除外，它的模块由本规格 §7 描述（D9） |
| depmap | crate 依赖边 ⊆ ARCHITECTURE §3 的 `depmap` 围栏块；一个 crate 之内的模块方向服从 `directions` 块（§8-33）；`pub trait` 只现于缝那一节（ARCHITECTURE §4）列出的文件 |
| guard | 唯一一张自己的 lint 表（`crates/desktop/ffi` 的）与根 `[workspace.lints]` 逐键相等，例外只在 `RECORDED`；其余每个成员都继承根表（§8-46）；工作区自己的包在 `[workspace.dependencies]` 里各钉 `=` 加工作区版本（§8-49） |
| wording | 读者拿到的词出自短语表 `client/src/lang.json`：`.svelte` 标记里文本节点与朗读型属性的字面量（`wording::markup`），`.ts` 里拒绝各段的实参（`wording::refusal`，§8-24），去掉插值后不得剩下相邻两个字母；行内 `wording-ok:` 豁免专名；生成的文件由它的生成器作证 |
| render | `#/gallery` 在真引擎里画出来，量盒子落在哪；性质见 §8-13、§8-14、§8-17 与 §8-38 |
| wiring | 城能执行的动词必须从客户端够得到；三个来源零副本（wire crate 在 `command/kind.rs` 里声明的 `enum Command`、`run_command` 的臂、`client/src`），wire-SPEC §19-2 只提供三者都说不出的那一件事——这个动词该由哪一侧够到 |
| secret | 全仓加夹具扫 secret shape（判定复用 `kernel::secret::scan`，无内联豁免）；只扫人写的文件，生成的锁文件与记录的快照由它们被扫的输入作证（§8-9）；兼查 `Sealed::expose` 调用点白名单 |
| specalign | kernel 枚举 ↔ kernel 的规格逐 variant（读 `crates/kernel/Spec.lean` 与分部里的受限形状，§8-43）：§8-1／§8-4 两张表消费真 enum（`AxCode::ALL`／`EventKind::ALL`）作证，归属、carrier／窗类逐臂同；规格里每一个与 kernel 枚举同名的 `inductive` 与 syn 解出的枚举双向对账（§8-10、§8-43）；模块图每一行的 `spec` 锚点落在盘上，已迁移的包写 Lean 模块名（§8-43） |
| spec | 一个包恰有一份生效规格；散文里点名的 `<名>-SPEC` 在树上；Lean 的 import 纪律；`.lean` 里没有 `sorry`、`admit`、`axiom`；规格引用的仓内路径在盘上（§8-42） |
| budget | `tools/xtask/budgets.toml` 里每一行可称重且被 gated 的预算，当场称一次；没有构建产物可称时沉默（`just check` 不构建 release 二进制），壁钟读数只入册不入门 |
| color | 颜色在每个客户端里恰好被命名一次（产地表见 §8-8），且以色域上限的比值表达；扫仓库根，文件自豁免 |
| release | 公开树由过滤生成；六条断言：公开树上零脚手架路径、产品文档不得链向或在正文里点名脚手架、任何发布文件不得携家目录路径、不得引用树里没有的文件、不得把一台机器的工作记录写进产品文档、链接的拼法与树上的名字逐字节相等（§8-15） |
| length | 一个生产函数不得长过 `function_length`、不得多于 `argument_count` 个参数（不含接收者），一个源文件的生产行不得多过 `file_length`；三个预算都住 `tools/xtask/budgets.toml`；函数尺寸与签名以 `syn` 量得，Rust 文件尺寸是总行数减去顶层 `#[cfg(test)]` 项所跨的行；`.zig` 受函数与文件两面、不受参数面，按 Zig 的词法量，文件尺寸减去 `test` 声明所跨的行（§8-48、D15） |
| npm | `client/` 的依赖面：锁文件在盘且与 `package.json` 逐条同、运行时依赖恰为 `npm::RUNTIME` 那张表、树上每个包的许可证都在 `deny.toml` 的准许表内（§8-12） |
| boundary | Rust 检查不得跨进程边界够到本产品；黑箱那一半住 `tools/adversary/` |
| artifact | 发布出去的那件东西的形状：测试脚手架不得进产品二进制、客户端落点只有一个家（§8-18）、平台与归档命名只有一张表（§8-19）、挂到 tag 上的归档先有构件证明（§8-34） |
| packaged | 可发布的包的生产编译单元里，`include!`、`include_str!`、`include_bytes!` 只指向包目录之内或构建脚本的 `OUT_DIR`（§8-49） |
| unused | 清单里声明、源码里从不点名的依赖（§8-37） |
| proof | kani harness 名册只住 `#[kani::proof]` 属性；CI 不得点名 harness，文档不得手写总数 |
| docnum | 文档里的数字由 `docnum::FACTS` 生成并由 `--write` 回写；区段陈旧、事实未知、标记不闭合各自即红（§8-16） |
| wire-ts | `client/src/wire.ts` 由 `wire::wire_schema()` 生成：每个具名类型一条 Effect `Schema` 值加一条 TS `type`，外加 `WIRE_V`、`WIRE_HASH`、`CITY_RUN`（§8-21）与 `BODY_PX`（§8-36）；不带 `--write` 时与盘上文件逐字节比对，第一处不同的行即红 |
| gates（命令） | 不带名字时跑全部门，带名字时只跑点名的那几道（按门表次序）；名字不在门表里即以 `unknown-gate` 退出码 2 拒绝并列出全部门名，不退回「全跑」；聚合报告，任一违规即退出码 1（D2，`spec/Report.lean`） |
| spec（命令） | 在那个包的目录下新建 `Spec.lean` 骨架；名字是包的 lib 名，没有 lib 的包用包名（`just spec`，§8-39、§8-41）。同名的门判树、不写盘（§8-42） |
| members（命令） | 包在哪：`--owning` 答一组路径属于哪些工作区包，`--dir` 答一个包住在哪个目录；`justfile` 用它，不再从路径里推包名（§8-39） |
| apisync（命令） | 不在门名册里：`cargo xtask apisync` 只判两条跨 crate 的缝 kernel 与 wire 的基线是否新鲜——用 `tools/xtask/public-api.txt` 钉住的渲染器算出实时面，与已提交基线逐行比对；夜间作业跑它（§8-32、D6） |

### 门禁针对的 LLM 失效模式（本包存在的理由）

| 失效模式 | 拦截门 | 机制 |
|---|---|---|
| 顺手新建文件，堆出一个杂物模块 | modmap | 模块表是封闭清单，表外文件即红 |
| 声称完成但状态未翻转 | modmap | 文件存在而状态仍是 `planned` → 红 |
| 逻辑漏进 lib.rs／索引文件 | modmap | 纯索引文件只许注释、属性、mod、use |
| 偷加依赖边、绕过分层 | depmap | 实际边 ⊆ 文档边；kernel 恒零内部依赖 |
| 娱乐性抽象（无第二实现的 trait） | depmap | `pub trait` 只许出现在缝清单文件 |
| 悄悄放宽一张 lint 表 | guard | `crates/desktop/ffi/Cargo.toml` 的 `[lints]` 与根 `[workspace.lints]` 逐键相等，例外只在 `RECORDED` 里且各带理由；其余每个成员的 `[lints]` 恰是 `workspace = true`（`spec/Guard/Wall.lean`） |
| 词汇漂移、自造同义词 | lexicon | `tools/xtask/lexicon.toml` 里的退役词，命中即红 |
| 忘记许可头或版权行 | header | 四行逐字节比对 |
| 定稿屏上有的 `role`／可及名，客户端里丢了 | render | 在真引擎里画出来，从 DOM 上读可及名与地标（§8-13） |
| 中文页面上直接写一句英文（模型的母语泄漏） | wording | 按位置判定：落在文本节点或朗读型属性里的字面量，去掉插值后仍带词即红 |
| stub／todo!／unwrap 蒙混 | （不在本包）workspace lints | clippy deny 已覆盖，本包不重复 |
| 一个函数里塞进整条流程（模型最常见的结构失效） | length | 超过行数预算即红，报出函数名、行数与预算 |
-/

/-! ## 2 验收标准

- 每门在违规夹具上报告非零、在干净仓库上报告零（单测覆盖解析与判定核心）。
- 违规输出恒为三段式：rule｜violation｜alternative，附 `gate` 名与 `file:line`。
- `cargo xtask gates` 在本仓库当前状态全绿；表外文件、lib.rs 加 fn、暗依赖边、缝外 pub trait、退役词五类违规各能单独触红。
- 一次运行的退出码取最重的一态，判不动的门不吞掉别的门的结论：`spec/Report.lean` 的
  `a_broken_gate_outranks_a_violation`、`a_broken_gate_drops_no_later_finding`、
  `green_means_every_gate_judged_and_found_nothing`、`the_outcome_ignores_gate_order`；Rust 侧由
  `report::tests::a_broken_gate_outranks_a_violation` 对 `RunOutcome::of` 判同一张表。
- 全部代码过 workspace lints（无 unwrap/expect/panic/索引/切片/裸算术/as）。
-/

/-! ## 3 假设与歧义

- 「注释与标识符扫描」简化为整行子串扫描：中文退役词只会出现在注释与文档，英文退役词不构成合法标识符片段。误伤由 `lexicon-ok:` 行内豁免兜住。
- **guard 判唯一一张自己的 lint 表**（`guard::wall`，D14）。每个工作区成员都继承根 `[workspace.lints]`，只有 FFI 叶子 `crates/desktop/ffi` 写一张自己的：每一次叶子调用要在那一处 `#[expect(unsafe_code)]`，而 `forbid` 在源码里放不开，所以它的 `unsafe_code` 是 `deny`（`crates/desktop/Spec.lean` D14）。除这一行之外那张表是一份**抄件**，而抄件是一个事实的第二个家。**任何判提交的规则都看不见这种漂移**：改一侧不改另一侧不需要任何一枚提交同时碰两边，也不会有任何东西变红。故本门每次运行都逐键比对，不相等即红，除非它在 `RECORDED` 那张表里带着理由；另判其余每个成员的 `[lints]` 恰是 `workspace = true`，因为一个写了自己那张表的成员就站在比对之外。**记下的例外会自清理**：两侧重新相等时，那一行必须划掉，与 `length` 划掉回到预算之内的钉子是同一条纪律（`spec/Guard/Wall.lean`、`spec/Length.lean`）；叶子不再是成员时，`LEAF` 与它的例外一起划掉。**本门只判工作树、不读提交历史**：「门变更与被判源码同处一枚提交须携 `Verdict:` 尾注」是 AGENTS.md 的规则，由评审执行，因为读历史会让每次运行都取决于调用方传来的区间。
- 语境依赖的退役词（如 session 指本城运行时、建筑指项目时）不入 `lexicon.toml`，由评审执行；`lexicon.toml` 内以注释记录此边界。
- **发行件的签名动作未接**（§8-29）：私钥由谁托管、谁签、泄露时怎么处置三项未定；验签侧已落地，无签名件恒拒收。定下托管方式，`just dist` 才能签。
- `spec/Packaged.lean` 把 `cfg` 谓词里 `test` 之外的名字当作彼此独立、可以任取真假的变量；
  `not(…)` 与读不出的写法一律算「可能成立」。这是门的读法本身，不是对 cargo 的假设：门往
  「可能成立」一侧错时只多报一条违规（§8-49、D16）。
-/

/-! ## 4 现状分析

全仓扫描只读文本、不起进程的门在毫秒到百毫秒量级；耗时的是要起外部进程的那几道：`render` 起浏览器，`budget` 称构建产物，`npm` 读锁文件与许可证。`gates` 让每道门各占一条线程（§8-31），故一次全量运行的墙钟时间由最慢的那一道决定。

分部里的四个参照定义各对着一处 Rust：`spec/Report.lean` 的 `RunOutcome.of` 与 `finish_all` 对着
`tools/xtask/src/report.rs` 的同名项；`spec/Length.lean` 的 `judge_file` 对着
`tools/xtask/src/length.rs` 的同名函数；`spec/Guard/Wall.lean` 的 `judge` 对着
`tools/xtask/src/guard/wall.rs` 的同名函数；`spec/Packaged.lean` 的 `excludes_production` 对着
`tools/xtask/src/packaged.rs` 的同名函数。两侧的分支一一对应，模型的证明不是 Rust 实现的证明，
对应由各模块自己的单测守着（§16）。
-/

/-! ## 5 权威信源

AGENTS.md 的「Rust」一节（硬化规则）与「The machine gates」一节（门表与门机械的提交纪律）；MPL 头全文（`LICENSE`）；退役词全集（`tools/xtask/lexicon.toml`）；ARCHITECTURE.md §3（`depmap` 与 `directions` 围栏块）、§4（缝清单）、§11（「Specifications in Lean」：Lean 规格的布局与门读它的受限形状）；`architecture.toml`（模块图字段契约）；`tools/xtask/budgets.toml`（每一项预算）；`skills/sdd/SKILL.md`（`spec` 骨架的十七节）。
-/

/-! ## 6 命名统一

gate／Violation／rule／violation／alternative（三段式拒绝的施工侧同构）。模块名与子命令名一致；门名以 `gates::GATES` 里的 `name` 为准，`cargo xtask gates --list` 逐行打印。

分部里的 Lean 名与它对着的 Rust 名拼法相同：`RunOutcome` 的 `Green`、`Refused`、`Broken`，
`finish_all`，`judge_file`，`judge`，`excludes_production`。Rust 里没有对应的项时才另起名字
（`Judged`、`FileVerdict`、`KeyVerdict`、`Cfg`、`Build`），它们是模型自己的词。
-/

/-! ## 7 模块边界

判定面一门一文件。门表与门序只住 `gates::GATES` 那张数组，`COUNT` 是它的长度类型参数——数目与清单相隔一个 token，故不可能各说各话。本文只说每道门判什么，不抄一份名册，也不手写门数，因为手写的名册与数组相隔一次代码改动而不是一个 token：要知道跑哪几道，读那张数组或跑 `cargo xtask gates --list`；门数只写在 D2 的受管标记里。

三个不判只做的模块：`main`（分发）｜`report`（Violation 与渲染）｜`walk`（确定性文件遍历）。其余各文件各自被某一道门调用而不自成一门：`architecture`（这份文档的名字与按 `## N 标题` 切节这一个读法，被 `depmap` 与 `proof` 调用，§8-22）｜`vocabulary`（`lexicon` 用它让退役词指向被定义的词；`proof` 用它的数词表读 `kani harness` 前的数）｜`members`（包在哪、叫什么、是产品还是工具，`cargo metadata` 的唯一读者，被每一道按包取目录的门调用，§8-39）｜`spec`（一个 crate 的 Lean 规格：命令新建骨架，同名的门判树，§8-41、§8-42）｜`lean`（把一份 `.lean` 当文本读出门要的几种受限形状，被 `spec`、`specalign`、`wiring` 调用，§8-43）｜`mem`／`sbom`／`repro`／`package`（`just` 的量具与交付物，恒不入 `gates`）｜`survey`（一页画出来之后才有的那些事实的判定，被 `render` 调用，§8-26）｜`bundle`（客户端落点这一个事实的读法，被 `render`、`budget` 与 `artifact` 调用，§8-18）｜`platform`（平台与归档命名这一张表，被 `channel` 与 `artifact` 调用，§8-19）｜`attestation`（挂到 tag 上的归档先有构件证明，被 `artifact` 调用，§8-34）。

**length 门的形状属于 modmap 而不属于自己**：形状列的解析只住 `modmap::shapes`，因为模块表只应有一个读者——字段一变，只有一处要改。同理，`[family.<键>]` 的 `duty` 也只由 `modmap::duties` 读，`docnum` 的 `crate_table` 经它取每个 crate 拥有什么（§8-40）。

**本模块不做什么（否定式两条）**：判定路径不改任何文件；写盘只发生在带 `--write` 的命令上，且每条只重写它自己生成的那一面——`spec` 只新建不覆盖，`apisync` 只写基线文件，`wire-ts` 只写 `client/src/wire.ts`，`docnum` 只写受管区段两个标记之间的字节。不缓存扫描结果（每次全量重扫——确定性优于速度）。

**secret 门细则**：扫描面是仓内全部文件（含 fixtures 与语料），排除隔离区 `local/` 与 `walk` 跳过的目录（§10 第 1 条）；判定器是 `kernel::secret::scan`（xtask 依赖 kernel，工作区成员不占产品拓扑，合法）；命中只报文件、偏移与长度，恒不回显字节；无内联豁免（豁免口会被注入内容利用）。兼查：产品包（§8-39）`src/**` 内 `.expose(` 的调用点只许出现在 `EXPOSE_WHITELIST` 列出的文件里——定义处与每一个出线前的最后一格，理由逐条写在表旁；命中即红。自测纪律：扫描器自身测试的高熵样本在源码中必须拆段拼接，不留可扫描的完整字面量。

**已复核字面量表**：判定器恒不改——它的活是在入口捕获一切像钥匙的东西，那里误报不要钱；**本门问的是另一个问题**「这里是不是提交了一份凭证」，那里误报要一次构建。故门内持一张 `NOT_CREDENTIALS` 精确字面量表，逐条写明它是谁、为什么不可能是凭证。三条纪律：①**整串精确匹配**——带前缀或后缀的更长 token 仍是命中，故没人能靠戴一个已复核的名字混过去（一条断言钉这件事）；②**表住门里而不是站点上**——注释式豁免是注入内容能写的洞，这张表不是；③表在门机械的路径下，增一条与被判源码分开提交。表里的条目是 Cargo 的分目标 C 编译器变量名（`release.yml` 的 musl job 设它），以及 `desktop/Cargo.toml` 用来选出 Windows 臂 API 面的 `windows` crate feature 名：feature 名由 resolver 读取、自身恒不持值，`Win32` 里的数字与下划线并置才是触发混合字母表规则的原因；只列长度 ≥20 字节的名字，更短的够不着熵侦测器。

**apisync 细则**：基线集是 `SEAM_CRATES`，按 lib 名写（kernel、wire），基线住 `tools/xtask/api-baselines/<lib>.txt`，由 `cargo xtask apisync --write` 生成。实时面由 `cargo +<rustdoc> public-api -p <包名> --simplified` 算出，包名由 `members` 按 lib 名查出（§8-39），`<rustdoc>` 与 cargo-public-api 的版本都只取自 `tools/xtask/public-api.txt`（D6）。判定与重写开始前先跑一次 `cargo +<rustdoc> public-api --version`：钉住的 nightly 没装、cargo-public-api 没装、或答出的版本不是钉住的那个，都以 `XtaskError::Cmd` 拒判，消息里给出两条安装命令，退出码 2。没有 rustup 的环境（例如 Nix devshell）走同一条拒判。漂移时，违规列出只在基线里的行（前缀 `- `）和只在实时面里的行（前缀 `+ `），最多 `DRIFT_SHOWN` 行，其余只报条数（D7）。接口变动要不要进 SPEC 交给评审，机器不判（§8-32）。cargo-public-api 是环境前置，`just prereqs` 把它列为可选；钉住的 nightly 由拒判消息点名。

D9 **modmap 判每一个包，门自己所在的包除外。**范围是 `members` 列出的全部包（§8-39），减去本门编进去的那个包（`CARGO_PKG_NAME`，与 `boundary` 读同一个事实）。xtask 的模块由本规格 §7 按模块描述，给它的文件另开一张表就是同一份描述的第二个家；其余工具包——今天是 citysim——与产品一样受封闭清单约束，因为它们的文件同样会被顺手新建、同样会忘记翻状态。锚点随之扩大：`specalign` 判 citysim 的 `spec` 列，与判产品的一样（§8-43）。失败的方向是故意的：一个新加的工具包默认受判，第一次提交就看得见一片红，而不是静静少判一个包。被击败的备选：在 citysim 的清单里再写一个 `mapped = true`——同一个包的一个性质分写在 `role` 与 `mapped` 两处；在 `modmap` 里写一张 `MAPPED_TOOLS` 常量表——包的一个性质住进了门里，改包名的那次提交不碰 xtask 也能过编译。
-/

/-! ## 8 接口先行

```rust
// 每门同一形状：
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError>;

pub(crate) struct Violation {
    gate: &'static str,      // 哪道门
    location: String,        // path 或 path:line
    rule: String,            // 规则（引 ARCHITECTURE 节号或规格的节号）
    violation: String,       // 违反点
    alternative: String,     // 合规替代
}
```

退出码：0 全绿；1 有违规；2 用法错误或门自身判不动（D1，§12）。一次 `gates` 运行怎样从各门的结论取这一个码，见 `spec/Report.lean`。

本节先是四段不编号的接口（`render`、扫描面、`wording`、命令 `wire-ts`），然后是编号条目，从 8-8 起按号排；8-1 至 8-7 与 8-27 空着，不复用。
-/

/-! ### `render`

**它补的是在引擎外判不出的那一类缺陷：层叠里的冲突在两份源码里都不存在。** 两条各自读起来都对的样式规则可以把一页排出第二条左边，而每一份源码、每一条文本测试都看不出来。`render` 开 `#/gallery` 在真引擎里画出来，量盒子落在哪。

**它断的是性质，不是图片。** 截图对比会被一次字体 hinting 弄红，却放过一个没人拍过的错版面。性质逐条写在 §8-13、§8-14、§8-17 与 §8-38。

**缺客户端产物、缺浏览器、画廊什么都没画，各是一条违规，不是跳过。** `render` 读 `crates/sprawling/web-dist` 里的客户端（§8-18），那份产物由 `just build-web` 生成；浏览器取 Chromium 一族，`SPRAWLING_BROWSER` 可以点名一个。前两条违规各自给出补上所缺之物的办法，第三条把画廊本身点成缺陷，三者从不并成一句。理由：一道找不到对象就沉默的门，给一页没人看过的页面报绿，坏掉的仪器读起来就像通过的产品。
-/

/-! ### 扫描面：构建目录不在里面

`walk::SKIP_DIRS` 持五个名字：`target`、`node_modules`、`.lake`、`.svelte-check`、`web-dist`。
`web-dist` 是 `just build-web` 写进 `crates/sprawling/` 的客户端包（§8-18），bun 的产物，同样被 `.gitignore` 点名。
**一道门为已提交的对象作证**，而构建目录里一个都没有；`.gitignore` 逐个点过它们的名。
对抗性检查器就地编译，一次构建就在源码旁边留下几十个生成文件；
扫进去读出的每一条都是关于生成文件的真命题，而那些文件没有任何读者会收到。
**一道报出满屏生成文件的门等于什么都没报**，因为没有人会读完第一屏。

这张表是「树里有什么」的第二个权威，git 是第一个；它继续做一张表而不去读 `.gitignore`，是因为五个名字值五个 token 而一个解析器值一个解析器。
**这就是它的重新定价参数：哪天这张表需要一行不是构建目录的名字，就去读忽略文件，不要再添一行。**

**另一份检出不是这棵树。** `git worktree add` 放在树里的检出根上有一个 `.git`，遍历器遇到根以下任何一个自带 `.git` 条目的目录就整个不进去；名为 `.git` 的条目不论是目录还是文件都不收——worktree 与子模块的 `.git` 是一个写着绝对路径的指针文件，它不是入库对象，`release` 读它就会报一台机器的路径。这条规则按结构判，不按名字判，所以它不是那张表里的一行：检出放在哪个目录下都一样被跳过，而一个非检出的隐藏目录照常被扫。被击败的替代是把某个工具的检出目录名加进表里——它只认一个工具，下一个把检出放在别处的工具又会让 `release` 把别人的检出当成仓库本体。
-/

/-! ### `wording`

**它读的方向与短语表自己那两条断言相反。** 那两条读的都是「视图向表要了什么」：一条要求每一条短语都有视图叫得出名字，另一条禁止视图把表里已有的句子再拼一遍。**一句从不调用 `say` 的字面量不在它们任何一条的视野里**：表不知道它存在，也就没有东西可比。

**依据是位置，不是词表。** 扫全部字符串字面量会去判类名、线上取值、事件名与格式键，全是误报。所以它只留读者被递了一个词的那几个位置：`.svelte` 标记里的**文本节点**与**朗读型属性的值**（`placeholder`、`title`、`alt` 与值为作者文本的那几个 `aria-*`），由 `wording::markup` 读；`.ts` 里一条拒绝的各段实参，由 `wording::refusal` 读（§8-24）。其余全部由**坐在哪里**排除，而不由一张例外名单排除。读法的限与它为什么不读 DOM，见 §8-13。

**去掉城自己的值之后剩下的那部分才是问题。** `{percent}%`、`+{added}` 没有递给读者任何视图写的东西；`{count} waiting` 递了一个英文词。故插值槽与转义先取出，再问剩下的部分里有没有相邻的两个字母。

**专名进不了短语表，所以它必须有一扇门。** 短语表拒绝一条两种语言相同的短语，而 `openai` 在两种语言里就是 `openai`。这些位置用 `wording-ok: <理由>` 写在本行或上一行——**同一个拼法、同一条两行规则，照搬 `lexicon-ok:`**。一个可见的、带理由的现场标记，比一张没人会去读的 toml 名单诚实。
-/

/-! ### 命令 `wire-ts`：线的 TS 面由 Rust 面生成

**它关掉的门是「手写第二份线」。** `client/` 用 TypeScript 说 `crates/wire` 的语言，而一份手写的 `wire.ts` 就是同一形状的第二个权威，它漂了也要到握手之后才被发现。故 TS 面由 Rust 面生成，且生成物入库、门盯着它：`cargo xtask wire-ts --write` 写 `client/src/wire.ts`，`cargo xtask wire-ts` 只比对——盘上文件与当场生成的文本逐字节不同即红，拒词点名文件与第一处不同的行号并给出 `--write`。与 `apisync` 同一口径：生成物由门自己写、由门自己校验。

| 文件 | 它回答什么 |
|---|---|
| `tools/xtask/src/wire_ts.rs` | 命令本身：文本从哪来（`render`：`wire::wire_schema()`＋`WIRE_V`＋`schema_hash()`）、写到哪（`TARGET`）、怎么比（`check`、`first_difference`）、怎么写（`write`） |
| `tools/xtask/src/wire_ts/emit.rs` | 一份 JSON Schema 文档怎么变成一份 `wire.ts`（`emit`）：文件抬头与三个常量、`$defs` 按名排序后按依赖拓扑输出（`ordered`、`refs_within`）、一条定义怎么命名与打 brand（`definition`）、以及拒绝长什么样（`Refused`、`refuse`） |
| `tools/xtask/src/wire_ts/emit/values.rs` | 一个 schema 怎么变成一个 Effect `Schema` 表达式（`expression`、`typed`、`fields`、`union`、`literals`）与它认得的关键字子集（`KNOWN`）：读 schema 的那一半，与读文档的那一半在 `expression` 处相接 |
| `tools/xtask/src/wire_ts/tests.rs` | 具名裸 `string` 打上 brand；带 `pattern` 的 `string` 收成 `Schema.pattern`；外标签枚举成 `Union`；依赖先于引用；子集外关键字被点名拒绝；环被拒绝；真实文档能发出；第一处不同的行被点名 |

**只认 serde 会产出的那个子集，其余点名拒绝。** 对象（`properties`／`required`／`additionalProperties`）、`string`／`integer`／`number`／`boolean`／`null`、`array`（`items`）与元组（`prefixItems`）、`enum` 字符串表、`const`、`oneOf`／`anyOf`（外标签、内标签、邻标签三种 serde 变体形状都落在这一条上，无需分别特判）、`$ref` 指向 `#/$defs/<名>`、`type: [T, "null"]`、`true`／`false`。`pattern` 译出：`string` 上带 `pattern` 时发 `Schema.String.pipe(Schema.pattern(new RegExp(<模式>, "u")))`，`u` 不是可选的——模式里写着 Unicode 属性（`\p{Cc}`、`\p{White_Space}`），不开 `u` 的引擎把 `\p` 读成字母 `p`。**一条语法由 Rust 那一侧的类型拥有，只有生成器把它带过来，客户端才不必自备第二份**：`client/src/core/address.ts` 曾手抄 `kernel::Address::parse` 的语法，那就是这一条存在的理由。`pattern` 不是字符串则点名拒绝，不静默丢弃。`description`／`format`／`minimum`／`minItems`／`maxItems`／`default` 读而不译（`description` 只在顶层定义处作为 JSDoc 发出）。`default` 是 `#[serde(default)]` 字段的注解：字段可缺省这件事由 `required` 一处表达，`Schema.optional` 已据它发出，所以再读 `default` 会造出第二个权威。其它任何关键字（`allOf`、`not`、`patternProperties`……）一律 `Refused`，报出所在类型的路径与关键字——**一个会猜的生成器就是一个会静默产出错类型的生成器**。具名的裸 `string`／`integer`／`number`／`boolean` 即 newtype，打上 `Schema.brand("<名>")`。

**为什么依赖拓扑而不是字母序**：Effect 的 `Schema` 是运行期值，`const B = Schema.Struct({ a: A })` 要求 `A` 已定义；字母序会撞 TDZ。拓扑序内按名字母序断平，故输出确定；环（自引用类型）以 `Refused` 拒绝——线上今天没有一个，出现那天再上 `Schema.suspend`，不预留。

**依赖**：`wire = { workspace = true, features = ["schema"] }`，工作区那一行关掉缺省 feature——不开 `server`，xtask 不为此拖进 tokio 与 axum；`schemars` 经 wire 的 `schema` feature 到达。xtask 是工作区成员而不占产品拓扑（§7 对 kernel 已用过同一条理由）。
-/

/-! ### 8-8 color：一个客户端，一处颜色产地（形状 6 数据面）

**权威是一句话**：颜色在客户端里恰好被命名一次，那一处是 `client/src/theme.css` 的 `@theme` 块。产地表一行，`THEME` 常量即那一行——「断言读哪份表」与「扫描放过谁」在一个客户端下是同一个答案。

**七条令牌断言读 CSS 自定义属性，不读 Rust 表**。解析面因此是 `--color-*`／`--text-*`／`--font-weight-*` 这一类声明，值取 `oklch(L C H)` 的三个分量。灰阶的 `L` 以千分之一为单位比较（`0.145` 读作 145），与断言里的 `L_FLOOR`／`L_CEILING` 同刻度。

**证明需要的三件事住在被判的那份文件里**。`oklch()` 只留得下解算后的值，而三条断言问的是解算之前的意图：

| 断言问什么 | CSS 里留不下的原因 | 落点 |
|---|---|---|
| 彩色令牌取的是色域上限的**比值**，且全库恰好两种 | `calc(0.151 * var(--chroma))` 是算完的积，90% 这个乘数不在式子里 | `--ratio-<token>` |
| 每个文本令牌够到它**自称**的 APCA 层级 | 层级是对比度结论，不是颜色分量 | `--tier-<token>` |
| 对比按文本可落的**最亮表面**判 | 表面是一条排版约定，不是令牌 | `--surface-ceiling` |

这三组属性不参与层叠——没有任何规则引用它们，浏览器读到即忽略。它们在这里，是因为 `theme.css` 是颜色唯一的家：这三件事若不写在它旁边，门就剩下三条无从判起的断言，而一道找不到输入就变绿的门，正是 §8-13 点名要避的失效。写进 CSS 而非另起一份 TOML，是为了让一个改颜色的人在同一屏里看见他改的值和那个值必须守的比值。

- **一份样式表被读成两块调色板**（`Mode::Dark`／`Mode::Light`）。`reading()` 把 `:root[data-theme="light"]` 那一块切出来，深色读数是剩下的部分，浅色读数是那一块加上剔除了 `--color-` 声明的共用部分；七条断言各对两份读数各跑一遍，违例报告先报是哪一面。**切成文本而不是先解成表**：下游每一个读者本来就读文本，多一层表就是这道门本来要防的那个第二权威。
- **色阶按页面命名，不按墨色**：`g0` 永远是页面，`g10` 永远是离页面最远的那一面。断言一因此是「每一档都比前一档更远离页面」，断言二是每个 mode 自己的一对端点。**两对端点不对称是 APCA 的结论而不是口味**：浅底深字被收的费远高于深底浅字，一个在 878 的表面上没有任何墨色能够到 Lc 90，于是浅色页把它的量程花在三个要承载文字的面上，剩下的才给下方的填色。
- **浅色在哪里被选中不归这份样式表管**：`system` 由客户端读 `prefers-color-scheme` 后写成 `data-theme`，而不是在 CSS 里再写一遍同一套令牌。**败给的方案**：`@media (prefers-color-scheme: light)` 里再声明一遍十一档——那是同一块调色板的第二份定义，两份在他们开始不一致之前都是对的。
- **改价条件**：若将来出现第二个客户端，产地表回到多行，`THEME` 与产地表重新分开。
-/

/-! #### 8-8a 禁用墨色只写在禁用状态之后（`color/disabled.rs`，形状 6 数据面）

`--color-text-disabled` 的目标是 APCA Lc 30（`--tier-text-disabled`），浅色页上约 2:1，只够告诉手「这里按不动」，不够让眼读出一个字。所以门的规则是：客户端源码（`client/src` 下的 `.svelte`／`.ts`／`.css`，不含 `theme.css`）里每一处 `text-text-disabled` 类名，都必须挂在一个名字里带 `disabled` 的变体之后，例如 `aria-disabled:text-text-disabled`、`disabled:text-text-disabled`、`group-aria-disabled:text-text-disabled`。花费、时刻、模型名、run id、占位字、按键字样这些人要读的信息，改用 `text-text-faint`（Lc 60）或更高一级。

- **判的是类名的写法，不是运行时的条件**：`{off ? 'text-text-disabled' : …}` 这种三元式里，门看不出条件是不是「禁用」，所以不收；元素本来就带 `aria-disabled`，写成变体，状态与墨色由同一个属性决定，没有第二个权威。
- **类名的边界**：从出现处往前取到空白、引号、反引号或花括号为止，这一段按 `:` 切开，最后一段之前的任何一段含 `disabled` 即算禁用上下文；最后一段是紧贴在类名前面的文字，不是变体，不算。否定的任意变体（`[&:not(:disabled)]:`）也含 `disabled`，同样算禁用上下文，这是按文字判的代价，客户端里没有这种写法。
- **败给的方案**：在 `lang.json` 或组件里另立一个「次要信息」灰级。那是 `--color-text-faint` 的第二份定义。
- **改价条件**：若 `--tier-text-disabled` 升到 Lc 60 以上，这个灰级就足以承载信息，本条可以撤。
-/

/-! ### 8-9 secret：门只看人写的文件，派生文件由它的输入作证

**门的主题是「明文凭证进入工作树」**，不是「任何高熵字节串出现在某个文件里」。`client/bun.lock` 与两份 insta 快照进树后，门报出 268 条，其中真凭证零条——一个依据碰上它从未见过的文件类，报的全是假阳性。

**依据补一条文件类，而不是补 268 个字节偏移**。逐条列偏移会把一个可判定的类别问题写成一张会腐烂的坐标表，而且下一次 `bun install` 就让它全错。新的依据分两类：

| 文件类 | 成员 | 它为什么不可能是凭证第一次进树的地方 |
|---|---|---|
| 生成的锁文件 | `Cargo.lock`、`client/bun.lock` | 每一字节都由包管理器从清单与仓库解算而来，其中的 `sha512-`／`sha256-` 是**已发布产物的完整性摘要**，本就该被任何人读到；人不往锁文件里写东西，写了下一次解算就冲掉 |
| 记录的快照 | `crates/**/snapshots/*.snap` | insta 快照是测试**输出**的留影，它的输入住在被扫的源文件里；一个凭证要出现在快照里，得先出现在那个源文件里，而那一份仍被扫 |

- **一句话的权威**：门扫**人写的**文件；一份**派生**文件的字节来自门已经扫过的输入，所以它不是凭证第一次进树的地方。两类各是这一句的实例，不是两条独立的例外。
- **`.expose(` 白名单那一半不动**：它只看产品包（§8-39）的 `src/**.rs`，锁文件与 `.snap` 本就不在其面上。
- **已知的限**（写在明处，不静默）：一个从环境变量读真凭证、再把它录进快照的测试，能从这条豁免下走过去。今天树上没有这样的测试，且写出这样的测试本身就是缺陷；真要堵它，堵的地方是「测试不得读真凭证」，那是另一道门的题目。
- **整词 PascalCase 名不是密钥**（`is_pascal_case_identifier`）：命中字节若整段是「大写开头的小写词」相接、末尾可带一串数字（如 Win32 字段名 `PeakPagedMemorySize64`），门不报。判的是命中字节自身的形状而非所在文件或上下文：每个大写字母后必跟小写、数字只在末尾，随机的 base64／hex 串几个字节内就破坏这一形状，所以 base64 长串与已知 provider 前缀照报。备选是往 `NOT_CREDENTIALS` 逐个补名——每个新 API 字段都要一次评审，而这类名字的共同点是可判定的形状。
- **为什么不是内联豁免注释**：门没有内联豁免，理由未变——注释是内容能自己写出来的东西，而一张编译进门里的文件类表不是。
-/

/-! ### 8-10 模块表的第七列 `Spec`、数出来的每 crate 计数，与 kernel 每个枚举的 variant 名单（形状 1 判定）

模块表回答「这个文件是什么」，却从不回答「它的接口写在哪」。读者要从 `accounting::views::rounds` 走到定义它的那一节，得先猜 crate、再翻 SPEC 的 §8。第七列把这一步写成数据。

**列约定**：`Module | File | What it owns | Shape | Since | Status | Spec`，一个还没迁到 Lean 的包，第七列的值形如 `<crate>-SPEC.md#8-N`（已迁移的包写 Lean 模块名，§8-43），这个文件名在全部包目录（§8-39）里找，恰好一个包目录持有它；一个也没有、或不止一个，各是一条违规。列在末尾，于是形状列与状态列的下标不动，只有单元格数从八变九。

**一节可以答多个模块，一个模块只能答一节。** 子模块跟随它的父模块所在的节，除非某节的标题点名了子模块的全路径（`runtime::tools::read` 有自己的 8-29，故它不跟 `runtime::tools`）。理由是 SPEC 的 §8 按接口分节而模块表按文件分行，两者本就不是一一对应；把子文件各钉到一个不存在的节上，只会造出一列指向虚无的链接。

**specalign 增第三条断言：锚点在盘上存在。** 第七列的每个值都被解析成「SPEC 路径 ＋ 节号」，路径必须可读，节号必须在那份 SPEC 里作为一个节的标号出现。SPEC 的 §8 有两种写法，两种都算：`### 8-N …` 标题（多数 crate），以及 §8 的接口围栏里那一行 `// 8-N …` 注释（`browser`／`agent_protocols`／`desktop`／部分 `web`／`runtime` 的写法）。**认两种不是放宽，而是照着树上真有的形状判**——只认标题会对六个 crate 报错，而它们的 §8 本来就是一整块围栏。

**已知的限，写在明处**：节号在同一份 SPEC 里并不唯一（`sprawling` 的 `8-40`／`8-41`／`8-42` 各出现过三次，`web` 的 `8-12`～`8-16` 各两次），因为各节各自续号而无人对账。故本条只判存在，不判唯一：加一条唯一性断言会对七份未经重编号的 SPEC 一次报错，而重编号是另一件工作。**翻案条件**：任一 SPEC 的 §8 完成一次重编号后，唯一性断言随即上线。

**specalign 的第四条断言：kernel 规格里与 kernel 枚举同名的每一个 `inductive`，就是 kernel 编译出来的那份 variant 名单。** 判据分两侧取数：规格侧读 `crates/kernel/Spec.lean` 与它的分部里一行一个构造子的 `inductive`（§8-43）；代码侧用 `syn` 解析 `crates/kernel/src/**`，`#[cfg(test)]` 模块里的夹具枚举不算。同名两侧都在，才逐 variant **双向**报——规格有代码无、代码有规格无各是一条，因为照着规格抄 match 臂的人会写出编译不过的代码，而只报一侧的门会让另一侧长期失真。**规格有而代码没有的名字不报**：模型里另起的类型（例如 `Draft`、`Names`）不是 kernel 的枚举，本条只答「两份名单同不同」，需要两份都在场。Lean 的 `inductive` 没有省略的写法，所以没有「指路牌」这一类跳过。

**模块图不带手写的小节计数。** `architecture.toml` 是结构化文件，没有小节也没有位置，数目由条目本身给出，于是「能被机器数出来的数不由文档手写」（§10 第 5 条）在这里以更彻底的方式成立：那个数不被写出来。
-/

/-! ### 8-11 `package` 认目标三元组：一份产物住哪里，叫什么名字（形状 2 值）

发布矩阵有一行 `x86_64-unknown-linux-musl` 一行，而 `budget::binary_path` 只认 `target/release`，`--target` 构建落在 `target/<triple>/release`。当时的落法是把静态产物拷到打包器看的位置，再把 `just package` 的步骤在 `release.yml` 里重抄一遍——**一条规则两个权威，明知而为并记在案**。本节还这笔债。

**一个具名值答两个问题**：这次构建是为谁构建的。`ReleaseTarget` 住 `tools/xtask/src/package.rs`，两个变体穷举：

| 变体 | 产物目录 | 归档名 | 归档里的可执行文件 |
|---|---|---|---|
| `Host` | `<target>/release/` | `sprawling-<version>-<os>-<arch>`（不动） | 向平台表按归档名取（§8-23） |
| `Triple(t)` | `<target>/<t>/release/` | `sprawling-<version>-<t>` | 向平台表按归档名取（§8-23） |

`<target>` 是 cargo 的产物目录：`CARGO_TARGET_DIR` 设了就是它（相对值相对工作区根解析），未设或为空时是 `<root>/target`。由 `package::cargo_target_dir(root, named)` 一处解析，环境变量只在 `release_dir` 里读一次，所以解析规则可以不改进程环境而被测到。写死 `<root>/target` 的读法在产物目录被改到别处时找不到刚建出来的二进制，`just mem`、`just package` 与 `budget` 一起失明。

- **`binary_path` 住 `package`**。「产物住哪里」与「产物叫什么」是同一个事实的两半，分住两个模块就是两个权威；`budget` 反过来向 `package` 要路径，因为它的活是称重而不是定位。
- **三元组进名字**。不进名字的话，musl 归档会叫 `sprawling-<version>-linux-x86_64.zip`，既不说静态也不说 musl，且**在出现第二份 Linux 产物（gnu）的那一天静默相撞**。主机构建的名字不带三元组。
- **`--target <triple>` 由 `main` 解析**，与 `release` 的 `--tag`／`--assets`／`--out` 共用一个取值函数：两个旗标两份解析就是两种取值语义。
- **`release.yml` 不重抄打包步骤**，三行矩阵走同一步 `just package ${{ matrix.target }}`；`just dist` 收下同一个可选参数。
- **本节属门禁机具，与产品代码分开提交。**

**`skills/` 随归档走，整树收录（形状 2 行）。** 发布物是人解压即用的那一份：一座城对着解压出来的目录找 skill，zip 里没有 `skills/`，拿到发布物的人就测不了 skill 相关的一切。故归档内容表加一个变体 `Packaged::Skills`，**按相对路径排序整树收录**——skill 的名单归那个目录管，逐文件抄一张清单就是给它安第二个家；目录缺失即拒，恢复语与 `Document` 同形。随树同行的是 `skills/LICENSES.md`：MIT 要求版权与许可全文随副本走，CC BY-NC 要求署名，两者都由它承载，于是义务跟着文件走到树外的任何一份副本。

**两条属人裁决在此记录。** ①**`skills/` 下的文档不披 MPL 头，MPL 文件头扫描以 `.rs` 为界**（该门本来就只扫 `.rs`，此裁决把「跳过 Markdown」从现状升为成文规则）：文档的许可证是它自己的 frontmatter 与 `docs/third-party.md` §5，给 MIT 的文档披 MPL 头就是错述它的条款；被击败的替代方案是逐文件豁免名单，它把一条能写成边界规则的事实变成一张会过期的名单。②**pstack 改编件（`why`／`how`／`blast-radius`）保持 MIT，且每件注明改编者是 2youg1**：改编声明与上游版权行、许可全文一起落在各件的来源注与 `skills/LICENSES.md`——一份不说谁改过的改本，藏的正是它现在是什么。
-/

/-! ### 8-12 `npm`：`client/` 的依赖面（形状 1 判定）

`client/` 进树时，看守它的那道门没有跟着进来。工作区那一侧的依赖面由 `cargo-deny` 与 `depmap` 两道门看着，JavaScript 那一侧当时什么都没有：一次 `bun add` 就能把第三个运行时依赖、一个 GPL 的包、或者一份与 `package.json` 已经对不上的锁文件带进来，而全绿的一次 `just check` 一句都不会说。

**三条断言，各修一种真实的漂移**：

1. **锁文件在盘上，且与清单逐条同。** `client/bun.lock` 的 `workspaces` 块记着 bun 上次解算时看见的 `dependencies` 与 `devDependencies`；`package.json` 记着今天要的那份。一处不同就说明有人改了清单而没有重解，于是一台开发机装出来的东西与 CI 装出来的东西不是同一棵树。依据是**两张表逐键逐值相等**，缺、多、值不同各报一条。
2. **运行时依赖恰为 `RUNTIME` 所列。** 这是 client-SPEC §1 已经写下的那条界线的机器面：devDependencies 随工具链自由变动，而进到用户浏览器里的东西是一张封闭的表，名单只写在 `tools/xtask/src/npm.rs` 的 `RUNTIME`，本节不抄它的条目与数目。**恰为**而不是**至少**——一个只查白名单不查缺失的门，会放过「svelte 被误删」这一半。
3. **树上每个包的许可证都在准许表内。** 准许表**不是本门新写的**，它就是 `deny.toml` 的 `[licenses] allow`：一个仓库对许可证只应有一个立场，工作区那一侧已经把它写下来了，本门读同一张表。许可证从 `client/node_modules/<包>/package.json` 的 `license` 字段读得——锁文件不带许可证，而已装的树带。

**`node_modules` 不在树上时，第三条 skip 并说出理由，前两条照判。** `node_modules` 是 `.gitignore` 里的名字，一台没有跑过 `bun install` 的机器上它不存在，而**这不是缺陷**；门在自己打印的那一行里说它没看，与 `render` 缺浏览器时同一口径。前两条只读入库文件，故在任何机器上都判得动——**一道会因为环境而整体沉默的门，就是一道在 CI 之外不再存在的门**。

**`client/` 整个不在树上时本门什么也不说**，与 `ax` 见不到定稿屏时同一口径：一道会因为目录不存在而变红的门必须先被关掉才能开工。

| 文件 | 它回答什么 |
|---|---|
| `tools/xtask/src/npm.rs` | 门本身：扫哪里（`MANIFEST`、`LOCKFILE`、`MODULES`、`PERMITTED`）、运行时白名单（`RUNTIME`）、三条断言（`check`、`judge_lockfile`、`judge_runtime`、`judge_licences`）与它们的拒词 |
| `tools/xtask/src/npm/lockfile.rs` | 两份清单怎么读成同一种形状：`bun.lock` 是带尾逗号与注释的 JSONC，故先归一再交给 `serde_json`（`read_jsonc`、`Manifest`、`manifest_of`、`lock_of`）；`deny.toml` 的准许表怎么读（`permitted`） |
| `tools/xtask/src/npm/tests.rs` | 尾逗号与注释被归一掉；清单与锁文件不同即报；运行时依赖多一个或少一个各报一条；不在准许表上的许可证被点名；`node_modules` 缺席时第三条不产出违规 |

**`license` 字段按 SPDX 表达式读**：一个标识符（`"MIT"`），或带 `AND`、`OR`、`WITH` 与括号的表达式。优先级与 SPDX 规范一致：括号先算，`WITH` 最紧（`Apache-2.0 WITH LLVM-exception` 作为一个整体在准许表里查），`AND` 次之，`OR` 最松。`OR` 任一分支准许即准许，`AND` 每一支都准许才准许，与 `cargo-deny` 对同一表达式的判法一致，所以两侧不会对同一个包各执一词。**写坏的表达式**——括号不配对、操作符悬空、空串——按「没有说」处理并被点名。旧包偶尔写 `licenses: [{type: ...}]`，本门**不认**，同样按「没有说」处理：报出来让人去看，比猜一个字段的历史写法安全。**被否的读法**：先抹掉括号再按 ` OR ` 切——`(MIT OR GPL-3.0) AND X` 会读成以 `MIT` 开头的一串 `OR`，第一支就放行，`X` 没有被判。

**一条传递进来的许可证在准许表上**：`minimatch` 的 `BlueOak-1.0.0`，写在 `deny.toml` 的 `[licenses] allow` 里，理由跟在行旁：它由 devDependencies 传递带入，到不了用户的浏览器；它经 OSI 审议通过，宽松，且授予 MIT 未言明的专利权。**`CC-BY-4.0` 不在表上**：它曾为 `caniuse-lite` 入表，那条来路随 Solid 编译链一起消失，今天 `bun.lock` 与 cargo 的依赖图里都没有一个 `CC-BY-4.0` 的包；一条没有持有者的准许，是一处没人看守的放宽。

**另一条路——换掉 minimatch——走不通，已逐条走查**：`minimatch ^10` 由 `eslint` 自身、`@eslint/config-array` 与 `@typescript-eslint/typescript-estree` 三处同时要求（`10` 之前的 `minimatch` 是 `ISC`，但降版就是降掉 eslint 10），它落在冻结的前端工具链上。

**翻案条件**：删掉 `deny.toml` 的 `BlueOak-1.0.0` 那一行，红的就是本节这道门——放宽写在它自己的提交里，不在门正卡着的那一次改动里，这是 AGENTS.md 的 `guard` 行区分的两件事。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-13 `render` 读画出来的 DOM，`wording` 读写下来的位置

**可及面与几何从同一份画出来的 DOM 上读。** `render` 开 `#/gallery`，前五条性质是：每个可操作控件有可及名、每个地标有名、一页恰一个首标题、主栏里每个区域同一条左边线、没有盒子画到容器外（其余见 §8-14、§8-17 与 §8-38）。容纳性那一条要阻的缺陷是一个盒子漂到了不属于它的位置，它不依赖任何 class 文法。`placeholder` 不是可及名——一打字就没了，故一个只带 `placeholder` 的编辑框判为无名。

**会滚动的容器装得下比它显示的更多**，故容纳性那一条按轴放行：探针连同每个元素的 `overflow-x`／`overflow-y` 一起量，父元素在某一轴上是 `auto` 或 `scroll` 时，那一轴不判。理由是这条规则要阻的缺陷是「一个盒子画在了别的东西上面」，而折线以下的内容什么都没画在上面——它靠滚动够到。不放行的话，一页内容多过一屏即报错，而这个产品的每一页都多过一屏。

**`wording` 不读 DOM，理由是它读不出那个区分**。渲染后的页面上，「视图写死的词」与「城供给的值」是同一种字节；能分开它们的只有源码里的**位置**。所以该门按位置判：`.svelte` 模板里的文本节点与朗读型属性两处（`wording::markup`），`<script>`、`<style>` 与 HTML 注释先切掉再读。**它是一个扫描器而非解析器**：客户端自己的工具链里已有一份正确的 Svelte 解析器，在这里再写一份只会是它更差的副本。两条已知的限写在明处：被表达式洞打断的文本只读洞之前的部分；跨行的文本逐行读。两条都是**漏报而非误报**，这是本门赔得起的那一面——视图画的词同时也画在 `#/gallery` 上，`render` 从页上读得到。`.ts` 里拒绝的各段由 §8-24 读。

下面这些是 `render` 的读法。

**它们不经 BiDi。** 复用 `bin::browser_bidi` 的传输够不到：那条传输是 `sprawling` 的私有模块，而够到它的两条路——提为 `pub` 并让 xtask 依赖整条产品图（含 tokio 与 axum），或把套接字迁进 `crates/browser` 的非默认 feature（推翻 `crates/browser/Spec.lean` 的 D4）——各自都要动产品代码，而本节属门禁机具。**出路在于前提本身不成立**：拉起一个无头浏览器让它吐出 DOM 不需要 BiDi。`render` 用的是 `--headless=new --dump-dom`，无套接字、无会话、无产品依赖，两条路的代价都不用付。

**引擎按三级取，每一级都是一条已有的权威，不新立第二份名单**：

1. `SPRAWLING_BROWSER` 点名的那一个；
2. **doctor 装到 `~/.sprawling/components/firefox/` 的那一个**——读的是 `components_dir()` 这条**文件系统约定**（`crates/kernel/Spec.lean` §8-22 已记），与 `xtask budget` 读 `target/` 同性质，不是对「运行中的机器上 Firefox 在哪」再写一份探测；
3. **doctor 的 Chromium 一族**：`crates/sprawling/src/doctor/family/chromium.tsv` 每行一个牌子——程序名与三平台的安装位置——由 doctor 自己的测试 `the_chromium_file_is_the_family_rendered` 从 `family::chromium` 渲染，本门 `include_str!` 它，按表序逐个牌子先看安装位置、再看 `PATH`。门与 doctor 因此在同样的地方找同样的牌子；本门不留自己的路径表，新加一个牌子只改 doctor 的那张表。

**为什么可及面不单独成一道门。** 比两侧**写下的**角色与可及名，是没有浏览器时的替代品；门能进浏览器之后，角色、可及名与地标从画出来的 DOM 上读更准，也少一份要维护的读法。

**判不了的理由各自点名**：没有构建产物（`crates/sprawling/web-dist/`）、没有引擎、画廊什么都没画——三种各报一条违规，各说各的。一道找不到东西就悄悄变绿的门，是这里要避的失效。

**探针等懒加载的视图落地才量。** 客户端把单独成块、按需取来的视图（例如 `#/gallery`）在块落地之前标一个 `data-pending` 属性，块到了这个标记随占位一起消失。以 `file://` 取来的块不占住引擎的虚拟时间，所以只在 `SETTLE_MS` 那一刻量一次，量到的是一页没有首标题的半成品。所以门给自己那份插了探针的副本在 `<head>` 里为 bundle `assets/` 下每个页面自己没点名的脚本块加一行 `<link rel="modulepreload">`（`engine::preloads`）：预取属于文档加载，而文档加载占住虚拟时间，块于是在探针量之前就已取到，之后的 `import()` 直接拿到它；交付的页面不带这几行，仍然按需取。探针在 `SETTLE_MS` 之后每 `POLL_MS` 问一次，直到页上没有 `PENDING`；到 `BUDGET_MS` 前两次轮询还在等，就把「还在等」写进 `FAILED`，门报「页面没有在量之前稳下来」，不去判那半页。不用 ARIA 的 `aria-busy`，理由是画廊把骨架屏和加载中的按钮当夹具来画，它们在页面开着的整段时间里都读作忙碌，拿它当信号探针永远等不到。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-14 `render` 的第六、第七条性质：一行的第一个标记，与键面上的下划线

**两条都是量出来的，不是读源码读出来的。** 第六条：左栏里每个可点行的第一个被画出来的盒子，中心 x 相同。它挡的缺陷是状态点 8 px、图标 18 px，各自在同一段 12 px 内边距里居中，于是点的中心比它下面每个图标左 5 px。第七条：任何 `<kbd>` 不带下划线；键面是一张脸，不是一个链接，而样式表给指针下的链接画的那条线会一并画进它里面的标记。

**「列」与「条」分开判。** 第六条只判行与行上下堆叠的那种 nav：一排共用同一个 top 的标签页是一条横条，要求它们同一个 x 等于要求它们叠在一起，故 nav 内所有可点行的 top 相同时本条不判。

**探针因此多量两件事**，随每个元素一起写下：它内部第一个有面积的元素的中心 x（没有则 `-1`），以及它的文字上是否有下划线。下划线按 CSS 的传播规则上溯——装饰会落到每一个在流内的后代身上，后代自己写 `text-decoration: none` 并不能把它取消——上溯在第一个不接收传播的盒子处停止：原子行内盒（`inline-block`／`inline-*`）、浮动、脱离文档流者。`Kbd` 的外层是 `inline-flex`，这正是今天键面不被链接的下划线波及的原因。

**已知的限，写在明处：只有 hover 才画的那条线，量不到。** `--dump-dom` 交出的是没有人碰的那一页，`a:hover` 的规则在那页上不生效。这是漏报而非误报：本条判的是页面静止时的键面。

**可及名多认一处：包住控件的 `<label>`。** 模型表的勾选框把词写在 `<label>` 里而不写 `aria-label`，而屏幕阅读器正是从那里取名字；探针原先只读元素自身的文本，会把这种控件报成无名。改后先读 `aria-label`／`aria-labelledby`／`title`／`alt`／自身文本，再读包住它的 `<label>` 与 `label[for]`。

**两条性质住在 `tools/xtask/src/render/marks.rs`**（`rows_share_a_first_mark`、`no_key_is_underlined`、`descends`）：它们要的是探针关于「一行内部」的读数，别的性质都不问这件事；`render.rs` 仍是唯一的次序与 `violation` 产地，`SLACK` 的 1 像素两处共用。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-15 release：链接的拼法与树上的名字逐字节相等

**一条在大小写不敏感的文件系统上永远为绿的坏链接。** 八份文档链向 `docs/glossary.md`，盘上的文件叫 `docs/GLOSSARY.md`——Windows 与 macOS 的默认文件系统两种拼法都开得出，Linux 与托管站点只开得出一种，于是每一位从网页上点它的读者拿到 404，而在 Windows 与 macOS 上五条断言全绿。**门不能用 `open` 判这件事**：判据必须是树自己写下的名字，而不是当下这套文件系统愿不愿意开。

**第六条断言**：一条落在树内的相对链接，其拼法必须与已发布路径逐字节相等；只在大小写上不同即红，拒词点名盘上的那个名字。判据住 `tools/xtask/src/release/link.rs` 的 `Spellings`（已发布集合加一张小写索引），`release.rs` 仍是唯一的 `Violation` 产地。**`None` 覆盖两种情形且是故意的**：链接是对的，或者它指的根本不是一份已发布文件（目录、或已由脚手架断言报过的路径）——本条只答大小写这一个问题，抢答别的会把更弱的句子排在读者前面。

**这一次的迁移方向是改盘上的名字，不是改链接。** 全仓 30 余处提及一律写 `docs/glossary.md`，其中 `tools/xtask/src/vocabulary.rs:76` 是运行时真的去打开它的那一处；唯一写大写的是文件名自己。故 `docs/GLOSSARY.md` 改名为 `docs/glossary.md`，一处改动，零个读者需要跟着动。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-16 docnum：文档里的数字由一张数组生成并回写

**一份写错的权威文档比没有文档更贵**：下一个读者（人或模型）按它写代码。已经发生的：`ARCHITECTURE.md` 说 `WIRE_V` 是 15 而线上是 31，说 Command 24 条而枚举是 28 条，说 `Cargo.lock` 有 497 个包而锁里是 389 个，说十个 kani harness 而树上有七个。**每一条单独改都是五分钟的事，而三个月后它们会以同样的方式再错一遍**——手敲进文档的数字就是那个事实的第二个家。

**机制＝受管区段。** 文档用一对 HTML 注释圈住一段文字，开标记里写它由哪个事实生成：

<!-- xtask:begin gate_count -->
22
<!-- xtask:end -->

上面这一段本身就是一个受管区段，圈的是 `gate_count`：它由 `cargo xtask docnum --write` 写出，读者据此知道这道门长什么样，而它同时受这道门看守，故这份 SPEC 里的示例不可能与机制分叉。

**两种形状，由作者选。** 两个标记各占一行时，值写在自己那一行；两个标记与值同处一行时，值留在行内——于是一个表格单元格与一句散文都能持一段受管区段，而 `--write` 保持作者选的形状不变。

**判据三条**：① 区段的文字等于它的事实当场的读数，不等即红，恢复语是 `cargo xtask docnum --write`；② 区段命名的事实必须在 `FACTS` 数组里，否则红，拒词列出全部已知键；③ 标记不闭合、区段套区段、或多出一个收尾标记，即红——一段读不出来的标记不得被当作没有标记。**`--write` 撞上未知事实时整份文件不写**：跳过它会让文档看起来刚重生过，而其中一个数字仍是旧的。

**权威＝那张数组。** `docnum::FACTS` 的每一行是「键、事实的家、参数、重算函数」。不带参数的：`wire_v`（`wire::WIRE_V`）、`command_frames`／`query_frames`（两张名表的长度）、`command_names`／`query_names`（**线上的 snake_case 标签，取自 `wire::wire_schema()` 而不是由变体名小写而来**——`rename_all` 是 `wire` 的决定，在这里再实现一次就是第二个权威）、`gate_count`（`gates::COUNT`）、`dependency_count`（`Cargo.lock` 的 `[[package]]` 条数）、`kani_harnesses`（`proof::harnesses` 数出的条数）、`compile_fail_cases`、`fuzz_targets`、`citysim_scenarios`、`test_functions`、`workspace_version`（根 `Cargo.toml` 的 `[workspace.package] version`，`CHANGELOG.md` 最新一节的标题引它，于是升了版本号却没写新一节的树会红）、`crate_table`（产品 crate 的整张表，§8-40）。**文档想引一个新数字，就往这张数组里加一行**，没有第二张清单需要同步。

**一张图也是一个事实。** `crate_graph` 把 `ARCHITECTURE.md` §3 的 `depmap` 块画成一段 mermaid `flowchart TD`：块里每个 crate 一行，每条允许的边一个箭头（依赖方指向被依赖方），不做传递约简，因为约简掉的边正是 `depmap` 允许、读者要查的那一条。生成函数 `depmap::graph` 与门用同一个 `parse_block` 读块，故图与门不会读出两张依赖表；手画一张依赖图，就是依赖表的第二个权威。值自带 ```` ```mermaid ```` 围栏，标记放在围栏之外，因为 mermaid 不认 HTML 注释。

**带参数的四族，键写成 `<事实>:<参数>`**：`dep_version:<crate>`（读工作区与各成员清单钉的版本；同一个 crate 在两处钉成两个版本时，这个读数本身就是拒绝）、`budget_bytes:<行>` 与 `budget_reading:<行>`（`tools/xtask/budgets.toml` 那一行的预算与读数，按文档的写法分三位一组）、`budget_headroom:<行>`（两者之比，四舍五入到一位小数）、`budget_figure:<行>.<字段>`（那一行任一整数字段，分三位一组、不带单位，单位由字段名给出；缺 `.字段` 即拒绝）。同一个读数只住在 `budgets.toml` 的一行里，文档里每处引用它的地方都是这一族的标记，不是第二份手写的数。一个生成器服务一族事实，于是加一个被引用的版本号或预算行不需要加一行代码。

**称重只属 `budget`，引用属 `docnum`**：`budget` 称重并守住预算，`docnum` 把读数搬进散文与表格；二者读同一行 `budgets.toml`，故读数只有一个家。同理 `dep_version` 并不与 `depmap` 争权——`depmap` 判依赖边，版本号住清单，本门只负责把清单里的那个字符串搬进文档。

**扫描面＝仓内全部 `.md` 与 `.lean`，排除隔离区 `local/`**：受管区段是给读者的承诺，而隔离区从不入库；Lean 规格也是给读者的文档，一对标记放进它的块注释里照样成立（§8-43）。

**区段持有的数**：`WIRE_V` 与帧表条数、版本表单元格、依赖数、ARCHITECTURE §11 的尺寸读数、验证读数里可数的那几项、`LLM.md` 的命令与查询清单。**它不持有的**：§11 的 kani 读数——那一处已经有执行者，`proof` 门读 `#[kani::proof]` 属性并与散文里的总数对账，再圈一段受管区段会让同一个数字有两道门，且 `proof` 的读法要求「数字」与「kani harness」在同一行相邻，一对标记插在中间就把它读没了。**叙事本身也不归它**：一段建立在错误基数上的论证要改写成指路，机器判不了它。

**它在 `gates::run` 的数组里**，随第一批受管区段落进 `ARCHITECTURE.md` 与 `LLM.md` 的那一次改动同集入表。关门判据照旧：**故意把 `WIRE_V` 改成 32 而不动文档，`just check` 必须红。**

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-17 `render` 的五次开页：三个宽度，一次亮色，一次强制色

**一个宽度下成立的性质不是性质。** 三栏按容器查询收起，一条规则在哪一档收起取决于那一栏分到多少宽度，而「rail 展开占 232px，三栏在 1440px 刚好挤坏」这类缺陷只在特定宽度出现。被击败的备选是只在一个宽度（`1440`）量一次，理由是「断言的性质在每个宽度都为真」——容器查询让这个前提不成立。**宽度因此是判据的一部分**：`768`（没有余地再放一栏侧栏）、`1280`（本产品最常被读到的宽度）、`2560`（宽到足以暴露一栏无上限地摊开）。高度三档共用 `1200`，因为本门断言的性质没有一条是关于折线的。

**一次运行开五次页**：三个宽度各一次（按客户端发出去的样子），外加**亮色一次、强制色一次**，两者都在 `1280` 下开——亮色与强制色改的是涂装与描边，不是哪一栏活下来，而「哪一栏活下来」正是上面三档在问的事。每次开页各起一个 `--dump-dom` 进程，无套接字。

**`--width <px>` 把一次运行收窄到一个宽度**，供改样式表时的短循环用；给出的不是那三个宽度之一即红，拒词列出那三个。**这个开关由 `pass.rs` 自己读进程参数，不由 `main.rs` 的分发器解析**：开关的名字、它接受的宽度、第四个宽度换来的拒绝，三件都是本门的事实，分发器解析它就是给这三件事第二个家。

**强制色用引擎自己的开关，不是夹具。** 判据是在本仓自己的引擎上量出来的，不是从文档抄来的：headless 加 `--force-high-contrast` 时 `matchMedia('(forced-colors: active)')` 为真（`prefers-contrast: more` 同时为真），不加时为假（在 Edge/Chromium 的 `--headless=new --dump-dom` 上实测）。另一条路——devtools 协议的 `Emulation.setEmulatedMedia`——要一条套接字，而 §8-13 已经裁定本门不开套接字，故不走。

**亮色用根元素上的 `data-theme`，也不是夹具。** 探针在量之前把 `document.documentElement.dataset.theme` 写成这一次要的那个值。**画廊里放一个 `data-theme="light"` 的 Case 做不到这件事**：亮色的十一级色阶声明在 `client/src/theme.css` 的 `:root[data-theme="light"]` 上，而页面内部的任何一个元素都匹配不上 `:root`，那样的夹具只会画出一个与暗色逐像素相同的盒子，并让一道绿灯替一张没人看过的页面背书。写在根元素上还顺带让暗色那三次也确定：客户端在挂载时把浏览器里存的那份外观选择应用上去，一次不说明自己要哪种亮度的运行，量的是跑这道门的机器碰巧偏好的那一页。

**这一次量的是不是这一次要的，由页面自己回答。** 探针另写一个 `<pre>`，两个词：引擎是否报告 `forced-colors: active`，以及根元素算出的 `color-scheme`。亮色块把 `color-scheme: light` 声明在那十一级旁边（`theme.css` 自己记了理由），故「页面亮了」就是「这里说了 light」。对不上即红，且在读性质之前就红——量错了页面还去读性质，等于替另一张页面报绿。**强制色那一次不问亮度**：强制色模式把页面的颜色换成系统的，引擎报回来的是系统主题的 scheme 而不是样式表声明的那个，在那一次追问亮度只会红在一件不归页面管的事上。

**任何计算出的 `overflow-x`／`overflow-y` 为 `auto` 或 `scroll` 的元素一律入测。** 探针量的是地标、标题、控件与带 `role` 的元素；只量这些时，一个裸 `div` 滚动盒对本门不存在，盒里的元素拿外面一个不滚动的容器当容器，表多一行就被判红。滚动盒入测之后，「最近的受测祖先」在有滚动盒时就是那个滚动盒，容纳性按它判，且只豁免它滚动的那一轴。被击败的备选是给滚动盒加 `role="region" tabindex=0` 让它入测：那要改产品代码，而且一个盒子只要挂上 `role` 就既被看见又被豁免，「表比盒宽」这类缺陷会一并从本门眼前拿走。键盘够不够得到一个滚动盒（WCAG 2.1.1）不归本门判。

**引擎没画的东西不量。** 收起的 `<details>` 里的内容仍报得出一个矩形，而那个矩形画在它展开后会去的地方之外（本仓的两处 TOML 预览各差一整个 section 的高度）；引擎跳过它们的布局并让 `checkVisibility()` 答 `false`，探针据此跳过。**这不是把一条缺陷藏起来**：一个人看不见的盒子压不到任何东西上面，而它展开后的位置要由一次展开着的夹具去量。

**`render` 的文件**（其余文件的归属见 `architecture.toml`）：

| 文件 | 它回答什么 |
|---|---|
| `tools/xtask/src/render/pass.rs` | 一次开页是什么：三个宽度与高度（`NARROW`、`READING`、`WIDE`、`WIDTHS`、`PAINTED_AT`、`HEIGHT`）、亮度与强制色两个枚举（`Lighting`、`Colours`）、五次开页的名单与它们的自称（`Pass`、`PASSES`、`called`）、页面回报的条件与对账（`Reported`、`disagrees`）、`--width` 的读法（`wanted`、`asked_for`） |
| `tools/xtask/src/render/pass/tests.rs` | 五次开页的名单、自称，以及两条对账各自的正反例 |
| `tools/xtask/src/render/probe.rs` | 那段量页面的脚本与它写读数的两个元素（`SINK`、`CONDITIONS`、`SETTLE_MS`、`PENDING`、`script`）；`engine.rs` 因此只管找引擎、开进程、把读数读回来 |
| `tools/xtask/src/render/announced.rs` | 屏幕阅读器遇到的那三条：`every_control_is_announceable`、`every_landmark_is_named`、`one_first_heading` |
| `tools/xtask/src/render/room.rs` | 字有没有地方站、弹层有没有地方开（§8-38）：`no_text_is_crushed`、`every_popover_shows_an_option` |
| `tools/xtask/src/render/room/tests.rs` | 两条各自的正反例 |

**关门判据**：`cargo run -q -p xtask -- render` 五次开页全绿；**把探针写的那个属性名从 `data-theme` 改成别的而不动样式表**，亮色那一次必须红在「页面画的不是这一次要的条件」上（拒词：`the pass asked for the light page and the page drew itself dark`）；**把 `--force-high-contrast` 换成任何不开强制色的开关**，强制色那一次同样必须红（拒词：`the pass asked for forced colours and the engine drew the colours it authored`）。**改 `PASSES` 里那一行的 `Lighting` 不是这条控制**：它同时改掉了请求与期待，两边仍然一致，故照旧为绿。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-18 客户端包的一个家：产品的 build script 说它叫什么

**一个目录，写者一个读者三个。** `client/vite.config.ts` 写出那个目录，`crates/sprawling/build.rs` 嵌入它，`xtask::render` 打开它，`xtask::budget` 称它。四处各写一份时，改 `outDir` 之后没有一处会红：build script 走 placeholder 只打一条 warning，两道门各自找不到对象，直到有人下载到一个只有空白页的二进制——这正是「找不到输入就变绿」那一类失效（§8-13）。

**权威落在 `crates/sprawling/build.rs` 的 `BUNDLE_DIR`，它的值是相对 `sprawling` 包目录、以 `/` 分段的路径 `web-dist`，判据两条。** 谁先需要它：任何一次 `cargo build` 都要先由 build script 找到那个目录，而门跑在其后。谁能被另一个引用：build script 是**唯一一个在已发布树里仍要工作的读者**——`release::is_scaffolding` 把 `tools/xtask/` 留在机器上，crates.io 上的 `.crate` 里也只有这个包自己的文件，所以一个住在 `xtask` 的常量在那两棵树上根本不存在，而反方向可行——`xtask::bundle` 用 `syn` 从 build script 里读出这个常量。故 `xtask` 侧零副本：`bundle::stated_path` 把它接在 build script 所在的目录后面，交出仓库相对的整条路径 `crates/sprawling/web-dist`；`render`、`budget` 与 `shots` 都调 `bundle::dist(root)`，它把这条路径逐段接在工作区根上。

**包体在包里，因为 crates.io 上只有包。** `cargo package` 只把包目录里的文件打进 `.crate`，验证构建与 `cargo install` 都在那份解开的包里跑 `build.rs`；包体落在工作区的 `target/` 下时，从 crates.io 装出来的二进制只带占位页。`sprawling` 的清单用 `include` 把被 `.gitignore` 挡掉的 `web-dist/` 收进包（sprawling-SPEC 8-157）。

**另外两处用另一门语言写，由闸断言相等而不代写**：`client/vite.config.ts` 与 `justfile` 必须拼出 `stated_path` 交出的那条仓库相对路径，不然 `artifact` 门红。一道门不改别人的打包器配置。**判的是整条路径而不只是目录名**：父目录若在各处各写一份，名字对得上而位置对不上，门仍是绿的。

**包的位置与 cargo 的输出目录无关。** 客户端包是 bun 的产物，`build.rs` 不读 `CARGO_TARGET_DIR`，只在自己的包目录下找 `BUNDLE_DIR`（sprawling-SPEC §8-83）。若让三个读者都跟随 cargo 的目标目录，每个读者都要复刻 cargo 解析它的规则（环境变量、`build.target-dir`、相对路径按当前目录解析），而这三份复刻用两门语言写，没有门能判它们相等。

**扫描面不进它**：`web-dist` 是 `walk::SKIP_DIRS` 的第五个名字，与 `target` 同为构建目录（§8「扫描面」一节）；不跳过时，`release`、`secret`、`header` 会把 Vite 压过的 JavaScript 当成入库对象来判。

**`render` 的 `location` 因此少了一段前缀**：一处版面违规现在报 `#/gallery <这一次开页>`，不再抄一份构建目录——那条读数对着的是画出来的页面，不是盘上的某个文件。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-19 `xtask::platform`：一张平台表，四份用别的语言写的抄件

**五个家，只有两个之间有过任何约束。** `channel.rs` 的 `ROWS`、`tools/xtask/src/channel/shim.js` 的 `PLATFORMS`、`install.sh` 的后缀表、`install.ps1` 的后缀、`.github/workflows/release.yml` 的 `archive` 矩阵，说的都是同一件事：这个项目为哪些平台构建，每份归档叫什么名字。**已经分叉过**：`install.sh` 提供的 `-linux-x86_64.zip` 对应一个矩阵从不产出的归档，于是一台 Linux 机器被指去下载一个不存在的资产。

**权威是 `tools/xtask/src/platform.rs` 的 `PLATFORMS`**，每行携七个拼法：归档后缀、runner 镜像、`--target`（主机构建为空串）、npm 包名、npm 的 `os`／`cpu`、归档里可执行文件的名字。`channel` 改用它，`Row` 这个第二个类型随之删除。

**三条断言挂在 `artifact` 门下，不新建门。** 挂 `artifact` 而不挂 `release` 的理由是 §1 的门表早已把「发布档的形状：平台三元组、归档命名」写在 `artifact` 那一行——写进 `release` 会让同一个责任有两个家，并让那一行继续说一件不真的话。三条：

1. **矩阵**：`release.yml` 的 `- os:`／`target:` 成对读出，与 `(runner, cargo_target)` 集合双向相等。
2. **shim**：`tools/xtask/src/channel/shim.js` 的每个条目读成「键、包、可执行文件」，与 `("<os> <cpu>", package, binary)` 双向相等。
3. **写全的后缀**：`release.yml`、`install.sh`、`install.ps1` 里每一个**写全**的归档后缀，必须是某一行的后缀。

**只判写全的那些，插值拼出来的不判。** `"-${os}-${arch}.zip"` 不是一个可比对的拼法，猜它就会判错一份正确的脚本。这是漏报而非误报，也是赔得起的那一半：拼错的归档名是已经发生过的缺陷，而插值拼出来的那个由下载本身拒绝，并告诉读者这次发布实际带了什么。

**登记表 `[archive_naming.predating]` 与另三张同形，且自清理。** 表上现有一行：`install.sh` 那个「留给将来某个 gnu 构建」的后缀。它记在册上而不是当场变红，因为划掉它是对一份不在本次改动范围内的文件动一行；**两个方向都会让它过期**——某一行平台表认领了那个后缀，或者再没有人拼它。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-20 `argument_count` 的豁免表自清理，与一个地址上的两个函数

**参数登记表与文件登记表一样自清理。** `file_length.predating`（§10 第 7 条）与 `boundary.predating` 都会把修好的行报出来；参数表若只用于跳过，一条签名修到预算以内、或者那个函数被删掉之后，豁免行会永远留着，并在任何人重造同名函数时静默重新授权。故 `length::check` 有第三条回收路径 `spent_signatures`，拒词三段与另两处逐字相同。

**一个地址可以住着两个函数。** 登记表的键是 `文件路径::函数名`，而同一个文件里可以有两个同名的 `new`（各属一个 `impl`）。若按解析顺序让后一个覆盖前一个，一条仍然有效的豁免会被报成已花掉，划掉它就会放那条超标的签名过门。**判据取同一地址上最宽的那一条**：一个地址只有在它上面没有任何函数超标时才算花掉。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-21 `wire-ts` 发出 `CITY_RUN`

`RunId::CITY`（nil uuid）标记城一级的记录，而 `client/src/core/belief.ts` 手写了一个同名常量——一个把它拼错的客户端会把每一条城级记录折进一个不存在的 run。生成器因此多发一条：`export const CITY_RUN`，取自 `kernel::RunId::CITY`，与 `WIRE_V`、`WIRE_HASH` 同处文件开头。三者总是一起走且从不被单独选择，故作为一个值 `Constants` 走（`wire_ts.rs` 定义，`emit` 消费），而不是把 `emit` 的参数表加到四个。

客户端从 `wire.ts` 导入 `CITY_RUN`（`client/src/core/belief.ts`），不自写这个常量。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-22 `architecture`：按节读 ARCHITECTURE.md，不按表的形状认表（形状 6 数据面）

**缝清单按位置读，不按表的样子读。** 若把「任何一行六格管道行，第二格是 `crates/**.rs`」当作缝表，那是在描述一张表的样子而不是一个位置：在这份文档任何地方新写一张同形的表，都会静默扩大允许声明 `pub trait` 的文件集合，且没有任何东西会说一句话。

**文档的结构因此只有一个读法**：`architecture::section(text, n)` 取 `## n 标题` 与下一个 `## ` 之间的全部行，`###` 子标题属于它上面的那一节。每一行带着它在整份文档里的行号回来，故拒词仍然指向人能打开的那一行。`PATH` 同时是「这份文档叫什么」的唯一一个家，`depmap` 与 `proof` 都从这里取。

**节不在场是 `XtaskError::Doc`，不是空集合**：读成空集合时，缝表消失会表现为树上每一个 `pub trait` 各报一条，真正的发现淹在一百条里。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-23 归档里有什么，由一张表说了算（形状 2 值）

**归档里有什么是一个穷举的类型，不取决于 `target/` 里恰好有什么。** 若按盘上有没有那个文件决定，没跑过 `just sbom` 的树打出来的归档会少一项，且一句都不说，同一个 tag 的两次打包对「里面有什么」各执一词。**`Packaged` 三个变体穷举**（`Binary`｜`Document { name, source }`｜`Sbom`），缺哪一项就带着写它的那条 recipe 拒绝。可执行位随变体走，不由「名字是不是 `sprawling`」判。

**归档里那个可执行文件叫什么，向平台表要**（§8-19）：那张表有 `binary` 一列，打包器不另用 `cfg!(windows)` 或「三元组里含不含 `windows`」再判一次。打包器按即将写下的归档名 `-<label>.zip` 向 `platform::with_suffix` 取行；**本次发布不出这份归档的目标被拒绝**，这正是平台表为自己写下的政策。

**发布二进制必须带执行引擎**：`AbsentSandbox` 的恢复语让人去装一个带执行引擎的构建，而下载发布档的人开不了任何 feature。判法与 `budget::carries_client` 同形——`budget::carries_engine` 读产物的字节，找只有 wasmtime 会写下的那句燃料陷阱文案（默认 feature 集下这棵树一个 wasm crate 都没有，故别处写不出它）。`sandbox` feature 在 `crates/sprawling/Cargo.toml` 里默认关闭，发布构建（`justfile` 的 release 构建行）显式打开它；`package` 在打包前拒绝一个不带引擎的二进制。

**发行归档还不带 cargo 包的许可原文。** 归档带 `LICENSE`（本仓库的 MPL-2.0）与 CycloneDX 物料清单；清单列出每个 cargo 包的名字、版本与 SPDX 标识，不带 MIT 与 Apache-2.0 要求随分发附上的版权与许可原文。npm 那一半由客户端产物里的 `THIRD-PARTY-NOTICES.txt` 随二进制发出（client-SPEC 12-13）。cargo 这一半缺的是：一个从 `cargo metadata` 的 `manifest_path` 旁读许可文件、按正文去重后写出的步骤，与 `Packaged` 里带着它的一个变体。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-24 `wording` 读 `.ts`：一个不画画的模块也说话（形状 1 判定）

**一个人被城拒绝时读到的那句话住在 `core/` 里，那里一行标记都没有**：只读标记的话，「每一个读者拿到的词都来自 `lang.json`」这条规则会漏掉它最容易被违反的那一半。

**两个位置，都是一次拒绝**：写给 `action`／`subject`／`recovery`／`reason` 的字面量（拒绝被造出来的形状），与传给本模块声明为返回 `AxError` 的函数的字面量（拒绝被造出来的方式）。返回类型由扫描器自己认，不在门里养一份函数名单。机器要的字节——wire 值、存储键、导入路径、类名——三者都不在这两个座位上。

**已知的限**：一句话经两跳函数传进来时，只在外层那一跳被读到；再宽就需要类型检查器已经有的调用图，而这是一个扫描器。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-25 `budget` 只给字节定预算；依赖数只报告，不设门（定规）

**门只判字节。** `budget` 判的行有一个共同点：一台机器两次量得同一个数，且数超出预算时该做的事是一个工程判断——压回去，或在同一个变更集里写明为什么变大。构建产物的字节（`frontend_artifact`、`release_binary`）就是这样的数，行里的三个键是 `budget_bytes`／`best_bytes`／`slack_bytes`，违规文案里的读数写作 `9937920 B`。

**依赖数是一个读数，不是一道门（定规）。** 这棵树解出多少个包，仍由 `budget::lockfile_packages` 从 `Cargo.lock` 数一次；`docnum` 的 `dependency_count` 事实把它引进 `ARCHITECTURE.md` 与 `docs/third-party.md`，`cargo xtask budget` 的报告在 `[dependency_count]` 那一行印出它。那一行不写预算、最佳读数与余量，门不拒绝任何包数。人的定规是：依赖本身不被反对，依赖多到项目成了黑箱时才值得减；减哪一个，要逐个看它换来的透明度，而一个包数上限回答不了这个问题——余量耗尽时它只会逼人删一个恰好便宜的包，或者把上限调高，两样都不让项目更透明。**被否的备选**：保留棘轮、只把余量放大，那仍是一道门，只是晚一些红。

**一处数，三处读。** 手写的包数在每份文档里会各有一个值，所以它由机器数出来而不由人写下来，且只数一次：报告调的 `measure(root, name)` 与字节行称重调的是同一个函数，报告印出的数与文档引用的数因此是同一次读数。

**报告的形状。** 写着三个字节键且 `status = "gated"` 的行进报告开头那张表；其余每一行印成 `名字: 读数 — status`，读数只在 `measure` 认得这个名字时出现（今天只有 `dependency_count`），认不得的行照旧只印 `status`——量不出的数不能印成 0。

**重开参数**：人撤回这条定规，或出现一个按包数计价的外部约束（例如发行件要为每个包附一份逐包审计）。

**账本追加的 p99 不另立一行**：它已经住在 `[ledger_append]` 的 `budget_p99_ms` 里并带着读数；再立一行就是同一个数两个家，而两个家会在不同的日子被不同的人改。**视图重建的每 MB 读数**住 `[views_rebuild_per_mb]`。**派活前置**（`[prepare_dispatch_ms]`）先量后门：预算若写在读数之前，它要么形同虚设，要么挡住正是要修它的那次改动。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-26 `survey`：一页画出来之后才有的那些事实

**它量的是布局跑完才诞生的事实，别的一概不量。** 间隙在不在刻度上、颜色是不是 token，读源码就能判（`color` 门读的就是这一类，Tailwind 的方括号值同理）；**两个盒子对不对齐、一行字有没有活着出容器、声明的词里有几个真被画出来**，级联跑完之前谁都不知道。定义里没有 CSS、没有 DOM、没有 Tailwind，所以换一个采集层（桌面控件树、原生视图）填同一份记录，每一条判定原样成立。

**「应该是多少」只有两个来源，按优先级。** 一是页面自己声明的词汇表，运行时从根元素上读回来（`--color-*`／`--text-*`／`--spacing-*`，探针把每个值按浏览器会画的样子解析一遍）；二是**合群判据**——七个盒子的左边在 56、第八个在 58，七票对一票，56 是这一列的事实，58 是一个错字。没有第三份理想设计稿，因为那张稿不存在。

**报告是一次编辑，不是一句抱怨。** 每条读数带着**要写下的那个值**；三十个盒子画同一个未声明的颜色是**一处编辑三十个站点**，不是三十条发现。干净即空：不打印「检查了 847 个元素，0 个问题」，一个人的环视免费，Agent 看一眼要付钱。

**组的顺序是谁使谁失效，不是严重度**：`paint → type → text → edge`。颜色不移动盒子，字号移动行，被裁的文字改变盒子，对齐是前三者的下游；顺序由报告给出，不指望读者自己推。

**三条几何判定从 `render` 迁进来，这是本节存在的第一理由。** `one_left_edge`、`rows_share_a_first_mark`、`nothing_escapes_what_holds_it` 本来写在用它们的那道门旁边——**一页的几何有两个家，正是这件量具要消灭的缺陷**。现在判定只有一处，门是它的消费者。

**门判什么与量具量什么，由 `Standing` 一处分开。** `Refused` 的读数停构建，`Noted` 的读数报给人。理由写在类型上：一件量具装上去当天就把树弄红，是一件会被关掉的量具；而它要求的修改多半是设计裁决，不是缺陷。

**取样面扩大了，门的判定面没有跟着扩大。** 判定要读颜色与被裁的文字，就得量到每一个承载文字的盒子，而门的容纳性判定本来只看「页面的骨架」（区域、控件、标题，以及任何 `overflow` 不是 `visible` 的盒子）。`Sampled { Frame, Words }` 把这件事写在记录上一次：骨架上的越界仍然停构建，取样面新收进来的文字盒子上的越界只报给人。**不写这一格，一次取样面的扩大就会让一道门在它从未判过的东西上变红。**

**容纳性只判「外露」**：`overflow: hidden` 的盒子与滚动盒一样在自己边界上把内容切掉，**什么都没画到外面**；只豁免 `auto`／`scroll` 会把每一个被裁的行读成一个画到邻居身上的盒子。`Overflow { Shows, Clips, Scrolls }` 三态，只判 `Shows`。

**被裁成一个像素的盒子不参与任何几何判定。** 那是页面把一句话只交给读屏器的写法（`sr-only`），它什么也没画，所以既不可能画到邻居身上、也不可能裁掉谁在读的字、更没有边可对齐；`Drawn::shows()` 与 `Drawn::drawn()` 因此是两个问题——它仍然是一个必须有可及名的控件。

**强制色那一趟不判颜色。** 强制色模式用系统的颜色替换掉样式表声明的每一个颜色，此时问「画出来的是不是声明的那些」，是拿一页去比一份被刻意推翻的调色板，答案是整页。`PaintSource { ThePage, TheSystem }` 写在被量的那一页上。

**对比度不在这一档。** 本仓已有一份对比度模型（`xtask::color::contrast`），标定在单色轴上两个**声明的 token** 之间；把屏幕上画出来的一对颜色接进去，要动一处本次改动不拥有的可见性，而在这里另写一个公式就是这件量具自己反对的第二权威。**本档的可读性只量布局造成的那一种：一个盒子把自己的文字切掉又不画任何记号。** 缺的那一半登记为债（F 章），不以一份副本补上。

**住处与称呼。** 阶段一住 `tools/xtask/src/survey/`，不进产品二进制、不动 wire、不动 `cargo public-api` 基线、不加 `Verb`、不进 `runtime::catalog` 的 `tool_defs`。**它不是一个子命令**：`gates` 那张数组与 `TOOLS` 那张数组各自是自己那件事的唯一权威，为一件还没证明自己的量具各加一行，是在有消费者之前先造 API；`cargo xtask render --survey` 是它的入口，`--route <fragment>` 换一页来量，两个开关都由 `render` 自己读，理由与 `--width` 同（§8-17）。

**探针的记录是一句话，写者与读者同住。** `probe.rs` 生成那段脚本，`probe/read.rs` 读它写下的每一个字段——记录是一行按位置排的字段、背后没有 schema，一边插一个字段而另一边不插，后面每一个字段都会静悄悄错位，而门会继续报出一批已经名不副实的数。父子同住是这里能拿到的全部防御。

**颜色经一个 1×1 的画布取，不自己解析。** 本仓的计算值序列化成 `oklch(…)`，手写一份颜色文法的解析器就是给一份还在长的规范写第二个实现；把值画进画布再读回四个字节，拿到的正是屏幕会收到的那个颜色，且对引擎认得的每一种写法都成立。α 通道按 `src-over` 向祖先合成；链路上出现背景图、混合模式、滤镜或背景滤镜时，答案从文档里推不出来，记录**不带颜色**而不是带一个错的。

**探针读不出声明的词就报错，不静默判。** 词汇表为空会把页上每一个颜色都读成未声明，于是空词汇表是一次「量不了」（退出码 2），不是一页干净。这与 §8-13 点名要避的那一类失效同形。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-28 `features` 那三条命令写在 `justfile` 一处，CI 调它

**决定**：这个仓库对 feature 组合的检查是 `just features` 那三条命令——工作区在默认 feature 集上（`--all-targets`，故测试目标也进编译）；`sprawling` 关掉默认 feature，也就是不带执行引擎的那一份二进制（`sandbox` 是默认 feature 之后，别的命令都不再编译它）；以及 `wire` 关掉 `server`。`ci.yml` 的 `clippy` 作业调这条 recipe，自己不拼命令。

**为什么**：同一个检查写过三遍时它们真的分叉了——CI 那一遍少了 `--all-targets`，于是本地门红的那棵树在 CI 上是绿的；三处又各自声称「别的命令都不编译这一份」，而三句话合起来互相证伪。`--all-targets` 是非对称的那一半：`cargo check` 单独一条不编译测试目标，而 `refusal_matrix` 曾在未声明 gate 的情况下用 `#[cfg(feature = "conformance")]` 的项，唯一编译过它的配置是 `--all-features`。

**败给的方案**：在 `ci.yml` 里照抄那两条命令，附一句「与 `justfile` 保持一致」。那正是分叉发生时的写法，而没有任何东西会注意到它们不再一致。

**只有一份**：`features` 不是门。`just check` 已经调这条 recipe，再在 `gates` 里跑同一条工作区检查就是同一次编译每轮跑两遍；只留 recipe 是一条裁决。
-/

/-! ### 8-29 发行件签名：Minisign 分离签名，验签先于落位（验签公钥侧与格式设计；不接签名动作）

**决定**：每份发行归档附一份 **Minisign 分离签名**，名为 `<归档名>.sig`，与归档同目录同版本发布。签名覆盖**归档字节本身**的 Blake2b-512 摘要，Ed25519 签名，载荷两行：`untrusted comment:` 行不参与判定，base64 体解出「算法标记 ＋ 8 字节 key id ＋ 64 字节签名」。三条安装通道——`install.sh`、`install.ps1`、`tools/xtask/src/channel/shim.js`——都在落位或执行**之前**验签：签名验证通过且 sha256 与发布侧对拍一致才继续，两者缺一即拒收。验签公钥的**唯一权威家是树上的一个文件**（`signing/sprawling.pub`，只含公钥，可公开）；三条通道脚本各携一份钉扎副本，因为 `curl | sh` 读不到树，副本与权威文件的一致由一道 xtask 门对账（`8-11` 后缀表同族先例：明知两个家，就给它们一个对账门）。

**为什么**：sha256 与归档同道而来——发布 API 的 `digest` 字段和资产字节走同一条 TLS 通道，能改写归档的一方也能改写它旁边的摘要，所以「只比哈希」证明的是传输完整，不是来源。非对称签名把来源钉在树上那把公钥上：字节要变，签名就废；签名要换，就得拿到私钥。签名覆盖归档字节而不是清单或摘要文件，因为被保护的对象就是人要落位的那份东西。

**败给的方案**：不验签，只比哈希（`install.sh` 现在的形态）。败因即上面的同道问题——它把「来源」问题降级成「传输」问题，而发行件签名要答的恰恰是来源。sha256 对拍**保留**为第二层：它拦传输损伤与资产错配，且免费。**另一被败方案**是自定义签名格式，败因是自造格式的验证器要自己审，Minisign 的格式小到一次说得完，且有现成的参照验证器。

**签名端 stub 与开发期开关**：私钥托管尚未决定，本设计**不接签名动作**——`just dist` 不签任何东西，发行归档不带 `.sig`。验签侧先行落地，因此**无签名件恒拒收**，拒因指名验签失败与缺件：`verification failed: missing signature: <归档名>.sig`（拒收即声明，不降级、不放行）。开发期一条**显式开关** `SPRAWLING_ALLOW_UNSIGNED=1` 允许安装本地未签名构建；它不得出现在任何文档的默认路径、任何 CI、任何发布作业里，且开着时安装输出一行显式警告。私钥托管、谁签、泄露处置三项是 §3 意义上的开放问题：定下托管方式，签名动作才能接上。

**归档炸弹上限（本表是这四个数的唯一权威家；各执行点引本表，不自立数）**：

| 上限 | 值 | 参数 |
|---|---|---|
| 归档字节 | 512 MiB | 发行归档实测为数十 MiB；上限是炸弹防御不是预算（预算是 `budgets.toml` 那一族） |
| 解压总字节 | 2 GiB | 压缩比防御：上限按最坏压缩比放大约四倍，超限即拒收而不是截断 |
| 条目数 | 100 000 | 合法归档是「一个可执行 ＋ `dist/` 静态资产」，两位数条目；千倍余量 |
| 路径深度 | 16 | 参照实现无此项（它以「剥根 ＋ 拒绝越界分量」代之）；深度是拒绝穿越与嵌套炸弹的第一道形状检查，与剥根规则并用 |

条目类型只收普通文件与目录；符号链接、硬链接与其他类型一概拒收（参照实现同）。路径含 `..` 或绝对分量即拒收。**篡改一字节的归档必被拒收，且拒因指名验签失败**——这是本设计的验收线。
-/

/-! ### 8-30 `mem`：量一个说得出名字的进程，或自己起一座夹具城来量（形状 4 适配器）

**接口**（`just mem` 的量具，恒不入 `gates`）：

```text
cargo xtask mem <pid>          量一个正在跑的进程
cargo xtask mem                在临时目录 init 一座空城，serve 在回环口上，等它接受连接，静置 2 s 再量，然后停掉
cargo xtask mem --city <dir>   同上，但 serve 的是给出的那座城（大账本的启动峰值与稳定值由此读出）
```

```rust
enum Target { Pid(u32), Fixture(Fixture) }          // 没有「量自己」这个值
enum Fixture { EmptyCity, City(PathBuf) }
struct Counter { counter: &'static str, bytes: u64 } // 读数总带着它的计数器名
struct Reading { private: Counter, peak_private: Counter, working_set: Counter }
fn run(root: &Path, args: &[String]) -> Result<String, XtaskError>;
```

**三个计数器分开报**，因为它们回答三个问题：private 是这个进程独占、别人拿不走的字节（B 轴的边际 RAM 就是它）；peak private 是启动折叠这类尖峰付过的最高值；working set 含共享的映像页，而且系统会在进程什么都没做时把它修剪到接近零——它不能当「常驻」用，只作对照。每个平台用自己的词：

| 平台 | private | peak private | working set |
|---|---|---|---|
| Windows | `PrivateMemorySize64`（私有提交） | `PeakPagedMemorySize64`（峰值提交） | `WorkingSet64` |
| Linux | `smaps_rollup` 的 `Private_Clean`＋`Private_Dirty` | `status` 的 `VmHWM`（峰值常驻，含共享页） | `status` 的 `VmRSS` |
| macOS | `ps -o rss`（保守上界，共享页全计） | 同左 | 同左 |

**夹具城的二进制**：`$CARGO_TARGET_DIR`（未设时 `<root>/target`）下的 `release/sprawling`＋平台后缀；`just mem` 不带 pid 时先 `cargo build --release -p sprawling --features <justfile 的 product_features> --locked`，所以量到的永远是这棵树建出来的、人下载的那个（citysim D9）。端口先由本进程在 `127.0.0.1:0` 上借一个再还回去；等待是按 5 ms 轮询连接，上限约 300 s：40 万条记录的城要先把整条 Ledger 折叠一遍才开始接受连接，在慢盘上是几十秒，而量具自己撞上的上限就是一次丢掉的读数。

**读数不带判词**：预算与读数住 `tools/xtask/budgets.toml` 的按场景的行（`[resident_empty_idle]` 等），本命令只报它量到的三个数、pid 与量的是什么。

**失败**：`XtaskError::Io`（起进程、读计数器、建临时城）、`XtaskError::Cmd`（参数说不出要量什么、`init` 被拒、夹具城在期限内没有接受连接、二进制不在那里——恢复语指向手动跑同一条 `serve` 或 `just mem`）与 `XtaskError::Doc`（计数器读不懂）。

**决定**：不带 pid 时自己起一座夹具城，而不是量自己。**败给的方案**有两个：缺省量 `std::process::id()`，量到的是 xtask，登记簿里那行 idle 读数会比真正在 serve 的空城还低；没有 pid 就拒绝。它也改掉了错的读数，但「一座空城闲着占多少」是每次都要问的问题，让人自己先起一座城再抄 pid，等于把量具的一半交回给人。
-/

/-! ### 8-31 `gates` 并行判定，按门序报告（形状 1 判定）

**决定**：`gates::run` 把选中的每道门交给 `std::thread::scope` 里的一条线程，再按 `GATES` 的次序逐条 `join`，把 `(门名, 结论)` 依门序交给 `report::finish_all`。一条线程若 panic，那道门报 `could not judge`（`XtaskError::GatePanicked`，带 panic 的消息：载荷是 `&str` 或 `String` 时取其文字，否则写明载荷不是文字），其余各门照常出结论；消息随结论按门序印出，而不是只留在默认 panic 钩子乱序写进 stderr 的那一行里。

**为什么**：各门只读树、互不写同一处，判定时间彼此独立，串行时门阶段的墙钟是各门之和，并行时是最慢那一道。报告按门序而不按完成序，所以两次运行在同一棵树上的输出逐字相同。

**败给的方案**：线程池或 `rayon`。门只有二十来道、每道各跑一次，一门一线程已经是最短墙钟；池只增一个依赖。

**限制**：没有门起 cargo 编译，并行的上限是最慢那道纯读门。
-/

/-! ### 8-32 `apisync` 不在门名册里，只判 kernel 与 wire（裁决）

**决定**：`apisync` 移出 `GATES`，因而也移出 `just check` 与 CI 的门作业；`cargo xtask apisync` 只比 kernel 与 wire 两条跨 crate 缝的基线，由 `nightly.yml` 跑。「基线变了就要求同 crate SPEC 同集被碰」这条共现断言删掉，接口要不要进 SPEC 交给评审。这是人的裁决。

**为什么**：它在门阶段里最慢（每次 `just check` 起十一次 `cargo public-api`，量级数十秒），而它保证的只是「SPEC 文件被碰过」：碰一个字节就过，拆提交就绕开，判不出宽度也判不出内容。其余九个 crate 的公开面只被本仓自己用，缝之外的面由编译器守。

**败给的方案**：整道删掉。kernel 与 wire 的公开面有仓外读者（`tools/adversary/`、客户端生成的 `wire.ts` 所依的线），它们的漂移值得一张夜间可见的差异表。

**重议条件**：又有 crate 的公开面出现仓外读者，或夜间作业里 `apisync` 的红多次在合并后才被发现。

D6 **基线是一次渲染，渲染器与基线一起钉住。** `tools/xtask/public-api.txt` 写两个值：rustdoc 取哪个带日期的 nightly（`PUBLIC_API_RUSTDOC`），cargo-public-api 取哪个版本（`PUBLIC_API_VERSION`）。`apisync` 只用这一对算实时面，`nightly.yml` 的 apisync 作业也只装这一对；挪动任一值，就在同一变更集里用新渲染器重写基线，挪钉子本身属门机械。理由：同一份源码，由另一个 nightly 的 rustdoc 或另一个 cargo-public-api 版本打印出来，可以逐行不同。两边各自浮动时，基线的内容取决于写它的机器上碰巧装了哪个 nightly，判它的 CI 每晚又换一个，源码一行没动也会报漂移；cargo-public-api 在工具链像 stable 时自己换成浮动的 `nightly`，而本仓库根的 `rust-toolchain.toml` 恰好钉着 stable，所以不写 `+<nightly>` 就一定落到浮动那一侧。被击败的备选有三个：①只在本机重写基线：下一晚 CI 换了 nightly，红照样回来；②只在 CI 钉住：本机 `--write` 仍按本机的 nightly 写出另一种渲染；③把 nightly 写进 `rust-toolchain.toml`：那个文件钉的是编译产品用的 stable，而 rustdoc JSON 只有 nightly 能出，两件事不能共用一个 channel。重议条件：cargo-public-api 能读 stable 工具链产出的 rustdoc JSON，或者钉住的 nightly 编译不了 workspace（某个依赖的 `rust-version` 超过它）。

D7 **漂移报告列出差异行，并设上限。** 夜间作业的读者手里只有日志，只报「漂了」时，他得在另一台机器上装同一对渲染器、重算一遍才知道漂了什么。所以违规按排序后的逐行比对，列出只在一侧出现的行，最多 `DRIFT_SHOWN`（40）行，其余报条数。被击败的备选：打印完整差异。rustdoc 的渲染一变，往往整份基线的行都跟着变，几千行的日志没有人读；上限内的前几行已经足够判断是源码变了还是渲染变了。
-/

/-! ### 8-33 `depmap` 也读一个 crate 之内的方向（`depmap::directions`，形状 1 判定）

**接口**：`depmap` 除了 crate 边与 `pub trait`，再读 ARCHITECTURE.md 的 ```` ```directions ```` 围栏块。每行 `模块路径: Rust 路径, Rust 路径`，模块路径是仓库相对、不带扩展名的路径（`crates/accounting/src/views` 覆盖 `views.rs` 与 `views/` 下每个 `.rs`），右边是这个模块的产品代码永不写出的路径（`crate::assembly`）。违例报出文件与行号；块里点名的模块在树上不存在也是违例；块缺失或行无冒号是文档错误（`XtaskError::Doc`），不当作「没什么可判」。

**读法**：按行读文本。注释行不算代码；测试不算产品代码：名为 `tests.rs` 或以 `_tests.rs` 结尾的文件（本仓把拆出去的测试模块命名为 `<主题>_tests.rs`，如 `views/standing_tests.rs`）、`tests` 目录下的文件、`#[cfg(test)]` 下的那个条目（以 `;` 结尾则一行，否则直到花括号闭合；数花括号之前先去掉本行的字符串与字符字面量，因为 `"{"` 里的花括号不开块，照数会让跳过延续到条目之后，把后面的产品代码悄悄漏判）都跳过，因为测试可以经装配点造夹具，而被测模块并不因此依赖装配点。路径按整段匹配，`crate::assembly_line` 不算 `crate::assembly`。读不出来的写法有四种：`use crate::{assembly, …}` 这种分组写法、`super::assembly` 与 `super::super::assembly` 这种相对路径、拆在几行上的路径、经另一模块 `pub(crate) use` 转出的装配点条目；跨行的字符串字面量里的花括号也照数。这是按行文本读法的代价，块里的路径按树上实际的写法登记；要堵上它们，改为用 `syn` 解析 `use` 树并把相对路径解析成 `crate::` 形式。

**决定**：模块方向写在 ARCHITECTURE.md 与 crate 边同一节，由同一道门读。**败给的方案**：一个在 sprawling 里扫自己源码的测试——它判的是树的形状而不是行为，放在被判的 crate 里会让产品 crate 知道自己的源码路径；也败给新开一道门，因为方向就是依赖图的一部分，门名册不必为它多一行。**重议条件**：某个 crate 的模块要按图而不是按禁止表来判（例如要求整个 crate 无环），那时改为从 `use` 解析出模块图。
-/

/-! ### 8-34 `xtask::attestation`：挂到 tag 上的每份归档都先有构件证明（形状 1 判定）

**要判的事实**：发布页上的每份归档都能用 `gh attestation verify <归档> --repo 2youg1/sprawling` 验出它出自本仓库的 `release.yml`。证明由 `actions/attest-build-provenance` 在发布 job 里生成，签名走 Sigstore 的无私钥流程（OIDC 令牌换短期证书），仓库里不存任何私钥。安装器照旧只比 sha256：`curl | sh` 不验签，文档也不这样宣称。

**权威**：`.github/workflows/release.yml` 里执行 `gh release create` 的那个 job。本模块只读它，不生成它；路径取 `platform::WORKFLOW`，不另写一份。

**接口**：`pub(crate) fn unattested(root: &Path) -> Result<Vec<Violation>, XtaskError>`，挂在 `artifact` 门下（与 §8-19 同理：发布档的形状本来就是 `artifact` 那一行的责任）。纯函数 `fn findings(workflow: &str) -> Vec<String>` 按形状逐行读，不引 YAML 解析器；每条发现是一句违规文字。失败：工作流读不到时返回 `XtaskError::Io`。

**四条断言**，都只在那个 job 的行范围内判（job 以两格缩进的 `名字:` 开头）。「执行 `gh release create` 的 job」指某一行去掉缩进后以这条命令开头的 job；注释里提到这条命令的行不算，否则前面某个 job 的一句注释就会把判定引到错的 job 上。「那一行挂上去的 glob」指这条命令连同它用行尾 `\` 续上的各行：发布命令本来就常折成多行，只读第一行会把写在续行上的 glob 当成没挂。

1. 该 job 的 `permissions` 写着 `id-token: write` 与 `attestations: write`。一个 job 声明了权限就得声明全部，缺哪一条证明步骤都会在服务端被拒，而那时归档已经构建完。
2. 该 job 有一步 `uses: actions/attest-build-provenance@…`。
3. 这一步的 `subject-path:` 与 `gh release create` 挂上去的 glob 逐字相同。两者分叉时，挂上去的归档里会有一份没有证明。
4. 证明那一步写在 `gh release create` 之前，所以一次发布挂出来的每份归档都已经有证明；反过来排，证明步骤失败时发布页上会留下没有证明的归档。

找不到执行 `gh release create` 的 job，本身就是一条发现，而不是「没什么可判」：静默通过会让整条规则随一次改名消失。

**不判的**：证明是否真的上传成功、`gh attestation verify` 能否通过——那要一次真实的 tag 推送，由发布流水线自己在服务端失败。
-/

/-! ### 8-35 `xtask::commits`：一段提交范围里每条提交信息的主题与裁决尾注（形状 1 判定）

**要判的事实**：AGENTS.md「Commits」一节的两条可机判的规则。其一，主题行匹配 `^card-S\d+\.[0-9A-Z]+: `，即 `card-`、`S`、至少一位数字、`.`、至少一个数字或大写字母，再接冒号与一个空格。其二，以 `Verdict:` 开头的行只能逐字是 `Verdict: user-approved`；拼错的裁决尾注读起来像一次裁决，实际上谁也没裁。

**不是门**：门判树，不判历史（§3 的 guard 一条与 `guard.rs` 的模块文档记着原因：读历史会让每次门运行依赖调用者传的范围）。故这是一条子命令 `cargo xtask commits --range <base>..<tip>`，由 `just commits <range>` 调用，CI 的 `fast` 作业在拉取请求上传 `base..head`、在推送上传 `before..after`。

**接口**：`pub(crate) fn check(root: &Path, range: &str) -> Result<Vec<Violation>, XtaskError>` 跑 `git log --no-merges --format=%H%x1f%B%x1e <range>`，逐条交给纯函数 `fn findings(message: &str) -> Vec<String>`；每条违规的 `location` 是提交的短哈希。失败：git 起不来或返回非零时为 `XtaskError::Cmd`，缺 `--range` 时 main 打印用法并以 2 退出。

**合并提交不判**：合并提交不带自己的改动，它的主题由 git 或托管平台生成（`Merge branch …`、`Merge pull request …`），要求它带卡号只会让每次合并都红，而它合进来的每条非合并提交照样被判。Dependabot 的提交由 `.github/dependabot.yml` 的 `commit-message.prefix` 定为 `card-S0.DEPS`，Dependabot 在以字母结尾的前缀后自己补 `: `，所以它的主题也落在同一条规则里，不另开豁免。

**败给的方案**：写成第 21 道门并在门里读 `--range`。门名册是树的判定，`just check` 在本地不带范围跑它；一道只在带范围时才判东西的门，在本地永远绿，正是「CI 里有、本地没有」的检查。
-/

/-! ### 8-36 `wire-ts` 发出 `BODY_PX`

人可以要的正文字号区间住 `wire::BODY_PX_MIN`／`BODY_PX_MAX`：写 `[ui]` 的那一层据它拒；外观页若自写一份区间，两份区间在其中一份先动的那一刻就是两个区间。生成器因此在文件开头多发一条 `export const BODY_PX = { min, max } as const;`，两个数取自那两个常量，随 `WIRE_V`、`WIRE_HASH`、`CITY_RUN` 一起作为 `Constants` 的一个字段走（`body_px: BodyPx`），而不是给 `emit` 添参数。

**被否**：把区间放进 `PreferencesAnswer`——那个类型同时是 `[ui]` 文件的文法，多一个字段就是文件里多一个人能写、而写了也不生效的键；放进 `Query::Config`——那个回答按地址爬梯子，而正文字号是这个人的、不是某个地址的。区间是这个构建的常量，不随城变，故走生成物而不走一次查询。
-/

/-! ### 8-37 `unused`：清单里声明、源码里从不点名的依赖（形状 1 判定）

**接口**：`unused::check(root) -> Result<Vec<Violation>, XtaskError>`，在 `GATES` 里，随 `just gates` 进 `just check`。两条断言：

- **包的依赖有人点名**：`members` 列出的每个包（§8-39），它的 `[dependencies]`、`[dev-dependencies]`、`[build-dependencies]` 以及各 `[target.*]` 下同名三表里的每个键，把 `-` 换成 `_` 之后，至少在这个包目录下某个 `.rs` 文件里作为一个完整标识符出现一次。违例的 `location` 是 `<包目录>/Cargo.toml`，`violation` 点名表与键。
- **工作区依赖有人继承**：根清单 `[workspace.dependencies]` 的每个键，至少是某个包的某张依赖表里的键。违例的 `location` 是 `Cargo.toml`。

清单不解析或某个包的清单读不到，是 `XtaskError::Doc`／`Io`，退出码 2，不当作「没有依赖」。

**读法**：按词读文本，不解析 Rust。键在代码里的名字就是键本身（`package = "…"` 改名时，代码写的也是键），所以不读 `package`。一个包的全部 `.rs` 文件合成一份文本判三张表：dev 依赖只在测试里点名、build 依赖只在 `build.rs` 里点名，这两条细分不判。这是有意放宽：它挡的缺陷是「依赖留在清单里而代码早已不用」，一个依赖被错放在哪张表里由 cargo 的编译错误来挡。

**为什么**：`-D warnings` 下 rustc 的 `dead_code` 已经挡住 crate 私有的死函数与死类型，`unused_crate_dependencies` 却会对每个只在测试或 bench 里用到的依赖误报，工作区因此不开它；清单里的死依赖于是没有任何一道检查看见，而它每次都多编译一整棵依赖树。死代码里能机械判定、编译器又不判的，就是这一类。

**败给的方案**：`cargo machete` 或 `cargo +nightly udeps`。前者判的正是同一件事，但要多装一个工具，并且 `just prereqs` 在缺它的机器上只能跳过——一道会被跳过的门挡不住回归；后者要 nightly 并且整仓编译一次，判一次要几分钟，不能进每次的 `just check`。本门只读文本，一次在毫秒量级。

**为解析而声明的依赖，由清单自己说出来。** 有一类依赖代码从不点名，却不是死依赖：它在清单里只为约束解析结果。例子是 `idna_adapter`：`idna` 按锁里它的版本线选 Unicode 后端，代码只经 `url` 间接用到它；本仓库眼下没有清单写 `pins`，`crates/gateway/Spec.lean` §13 说明 `idna_adapter` 为何不钉。这样的键写进该包清单的 `[package.metadata.unused] pins`（一个键名数组），键旁的注释写明它约束的是什么。本门对 `pins` 判三件事：`pins` 里的键不受「源码点名」那条断言约束；`pins` 里的每个键必须是本包某张依赖表确实声明了的键，否则报「豁免指向一个不存在的依赖」；`pins` 里的键若已被源码点名，同样报违规，因为豁免已不再需要——与 `#[expect]` 不再触发即红是同一条自清理纪律。违例的 `location` 仍是 `<包目录>/Cargo.toml`。**被否的备选**：在代码里写一行 `use idna_adapter as _;` 让本门看见名字——那会把一条不为代码而存在的声明当成被使用，豁免的理由也不在清单旁，而在一个读者不会去找的源文件里。

**已知的限**：宏展开出来的名字（`#[derive(Serialize)]` 而全文从不写 `serde`）同样判红；把 `use serde::Serialize` 写出来即可。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-38 `render` 量字有没有地方站、弹层有没有地方开（形状 1 判定）

**两条性质，都是对 `Drawn` 的算术**，住在 `tools/xtask/src/render/room.rs`：

- `no_text_is_crushed`：一个自己写字的盒子（`text` 非空、`shows()`），名字至少两个字且不含换行（作者自己折的行不算挤，例如代码的行号栏），宽度小于两个字宽（`2 × px_x100 / 100`），而高度至少两个字高——也就是字被折成了多行。拒词给出盒子、宽、字号与行数的下界。它挡的缺陷是提示框在 390 宽下正文一字一行：每个盒子都没溢出、都在容器里，`survey` 的包含与截字两条读数都是绿的。
- `every_popover_shows_an_option`：一个弹层（`role` 为 `listbox` 或 `menu`）被已量祖先截过后可见的高度——`Clips` 的祖先按位置截，`Scrolls` 的祖先只按自身高度截（滚得到的不算截掉）——，小于它第一项（`role` 为 `option` 或 `menuitem` 的后代）的高度。没有一项被量到的弹层不判。它挡的缺陷是主模型选择框打开后，列表被外层容器截到只剩一条细线。

**常量**：两个字宽、两个字高（`CRUSHED_EMS = 2`）；一行字至少一个字高，故「高 ≥ 2 个字高」是「至少两行」的下界，单字按钮的内边距不会让它误报——那种盒子名字只有一个字。

**已知的限**：可见高度按祖先的外框截，不算定位上下文：`position: fixed` 的弹层逃出非包含块祖先的截断，会被这里多截一次，是误报而非漏报；见到时把弹层的截断祖先改成不截或把弹层挂到外层，本条不加例外。

**为什么不放进 `survey`**：几何的家是 `browser::survey`（§8-26），这两条本应是它的读数；本节先作为门自己的性质落地，因为改 `crates/browser` 与改门不能同一个提交，而这两条要在修提示框与选择框之前先对今天的 gallery 报红。**重议条件**：`survey` 下一次增加读数时，两条迁进去，本文件删掉。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-39 `members`：包在哪，只问 cargo metadata（形状 4 适配器）

**要答的事实**：这个工作区有哪些包，每个包的包名、lib 名、目录与角色，以及它的正常与构建依赖点名了哪些包。门按包取目录的地方都问这里。一个读者自己推导目录时，它就假定了目录名、包名、lib 名三者相同；三者一分开，或一个包换了目录，推导出的路径上什么也没有，门不报错，只是什么也没判。

**接口**：

```rust
pub(crate) struct Member {
    pub(crate) package: String,              // `cargo -p` 用的名字
    pub(crate) lib: Option<String>,          // lib target 的名字；只有 bin 的包（xtask）为 None
    pub(crate) dir: String,                  // 仓库相对、以 `/` 分段
    pub(crate) role: Role,
    pub(crate) depends_on: BTreeSet<String>, // 正常与构建依赖点名的包名；dev 依赖不在内
    pub(crate) publish: Publish,
    pub(crate) roots: Vec<String>,           // lib、bin、构建脚本、过程宏这几种 target 的源文件，仓库相对；测试、bench、example 不在内
}
pub(crate) enum Role { Product, Tool }
pub(crate) enum Publish { Registry, Never } // metadata 的 `publish` 是空表即 Never，其余（null 或一张 registry 名单）即 Registry

impl Member {
    pub(crate) fn name(&self) -> &str;             // lib 名；没有 lib 的包用包名
    pub(crate) fn holds(&self, rel: &str) -> bool; // 仓库相对路径落在这个包的目录里
    pub(crate) fn in_product_graph(&self) -> bool; // Product：depmap 块与 proof 名册只列这些
}
pub(crate) fn members(root: &Path) -> Result<Vec<Member>, XtaskError>; // 按 dir 排序
pub(crate) fn product(root: &Path) -> Result<Vec<Member>, XtaskError>; // role 为 Product 的那些
pub(crate) fn find<'a>(found: &'a [Member], name: &str) -> Result<&'a Member, XtaskError>; // 按包名或 lib 名找；找不到为 unknown-package
pub(crate) fn owner<'a>(found: &'a [Member], rel: &str) -> Option<&'a Member>; // 持有 rel 的包里目录最长的那个：嵌套的包归它自己
pub(crate) fn run(root: &Path, args: &[String]) -> Result<String, XtaskError>; // `members` 子命令
```

**读法**：一次 `$CARGO metadata --format-version 1 --no-deps --offline`。`CARGO` 环境变量在时用它，否则用 `cargo`：`cargo xtask` 与 `cargo nextest` 起的进程都带着这个变量，于是门用的是钉住的那套工具链。`dir` 是 `manifest_path` 的父目录去掉 metadata 自己报的 `workspace_root` 之后的路径；去不掉时以 `member-outside-checkout` 拒读，不退回绝对路径，因为一个绝对路径拼进 `root.join` 之后照样读得到文件，门会在另一棵树上判出结论。`lib` 取 `kind` 含 `lib` 的那个 target 的名字。只列工作区成员：本仓没有在工作区之外构建的包（D14）。

**角色由包自己声明**（D4）：`[package.metadata.sprawling] role = "tool"`。xtask 与 citysim 写这一行；不写即 Product，写别的值以 `unknown-role` 拒读。

**读者与它们取的东西**：

| 读者 | 取什么 |
|---|---|
| `depmap` | `in_product_graph` 的包；块里的键是 `name()`；一条依赖边以被依赖包的 `name()` 比对，依赖的若不是工作区包则不判；违规的位置取 `dir`；`pub trait` 遍历 `product` 的目录 |
| `modmap` | 全部包减去本门所在的包（D9）的目录：条目过滤与磁盘遍历用同一组目录 |
| `specalign` | 锚点里的 `<x>-SPEC.md` 在全部包目录里找，恰好一个包目录持有它 |
| `artifact`、`secret` 的 `.expose(` 一半 | `product` 的目录 |
| `length` | 每个包的 `src/`、每个包目录下的 `.zig`，加 `client/src` |
| `proof` | `in_product_graph` 的包；`cargo kani -p` 取 `package` |
| `boundary` | 一个文件归哪个包，那个包是什么角色 |
| `apisync` | `SEAM_CRATES` 写 lib 名，经 `find` 取那个包的 `package` 作 `-p` |
| `spec` | 参数经 `find` 找包，`Spec.lean` 骨架写进它的 `dir` |
| `unused`、`docnum` 的逐包计数 | 全部包的目录 |
| `justfile` 的 `check-branch` 与 `branch-tests` | `members` 子命令 |
| `packaged` | `publish` 为 `Registry` 的包的 `dir` 与 `roots`（§8-49） |

**子命令**：`cargo xtask members --owning <path>...` 逐行打印拥有这些路径的工作区包的包名，去重、排序。三种路径跳过：`.md` 与 `.lean`（文档与 Lean 模型的改动不选中 Rust 测试），以及不落在任何工作区包里的路径（一个分支自己的 diff 会列出已经不存在的旧路径）。一条路径归持有它的包里目录最长的那一个（`owner`），所以 `crates/desktop/ffi` 下的改动选中叶子而不是 desktop。不给路径时从标准输入逐行读：一次改名的 diff 有几百条路径，而 Windows 一条命令行最长 32,767 个字符。`cargo xtask members --dir <package>` 打印那个包（经 `find`，按包名或 lib 名）的目录；没有那个包时以 `unknown-package` 退出码 2 拒绝并列出全部包名，不退回一个猜出来的目录。

**`boundary` 怎样分**：`tools/fuzz/` 整个是测试代码（`boundary::FUZZ`，它自成工作区，`members` 看不见它）；工具包里，门自己所在的包（`CARGO_PKG_NAME`）按 `#[cfg(test)]` 项判，其余工具包（citysim）整个是测试代码；产品包的 `tests/` 目录整个是测试代码，其余按 `#[cfg(test)]` 项判；不属于任何包的 `.rs` 不判。

**模块图登记除门自己之外的每个包**：门自己的文件由本规格按模块描述，其余工具包（citysim）与产品一样登记每个文件（D9）。新增一个工具包时，它的条目与它的第一个文件同一次提交落地，否则门把那个文件报成表外文件。

**性能**：每个读者各起一次 `cargo metadata`，`--no-deps --offline` 不解算依赖，只读各成员的清单；`gates` 各门并行（§8-31），不缓存（§7）。

**败给的方案**：各读者继续自己推导，只把 `crates/` 扩成 `crates/` 与 `tools/` 两处——下一次搬目录仍是 N 处改动，而漏改的那一处不报错；从根清单的 `members` 表读目录——那张表可以写 glob，也不给包名与 lib 名，depmap 仍要自己起一次 cargo，于是还是两个读者。

**本节属门禁机具，与产品代码分开提交。**

D4 **一个包是产品还是工具，由它自己的清单声明。** `[package.metadata.sprawling] role = "tool"`，xtask 与 citysim 各写这一行；不写即 Product，写了别的值以 `unknown-role` 拒读（§8-39）。理由：声明跟着包走，搬目录、改包名都不必改 xtask；一个忘了声明的新工具按产品受更严的门（要进 depmap 块、要进模块图），失败的方向是一次看得见的红，而不是一道门静默少判。被击败的备选：在 `members` 模块里写一张常量表 `TOOLS`——包的一个属性就住进了另一个包，改包名的那次提交不碰 xtask 也能过编译，两份名字从那一刻起各说各话。
-/

/-! ### 8-40 `crate_table`：产品 crate 的那张表由三个已有的权威拼出（形状 4 适配器）

**要答的事实**：产品的每个 crate 住在哪个目录、在 crates.io 上叫什么、lib 名是什么、拥有什么、可以依赖谁、SPEC 在哪。这六件事各有一个家：目录、包名、lib 名由 `members` 给出（§8-39）；可以依赖谁住 ARCHITECTURE §3 的 `depmap` 块；拥有什么住 `architecture.toml` 的 `[family.<键>] duty`。在 `crates/README.md` 里手写这张表，就是给这三个家各添一份抄件，而 ARCHITECTURE §1 的图与 family 表已经把同一份职责说了两遍。

**接口**：

```rust
// docnum::facts 的 FACTS 多一行：键 `crate_table`，不带参数。
// docnum/facts/crate_table.rs
pub(super) fn recount(root: &Path) -> Result<String, XtaskError>;
// modmap.rs：按 packages 的次序，每个包它的 family 的 duty（D5）
pub(crate) fn duties(root: &Path, packages: &[Member]) -> Result<Vec<String>, XtaskError>;
// depmap.rs：由私有改为 pub(crate)，crate → 块里允许它依赖的 crate
pub(crate) fn parse_block(text: &str) -> Result<BTreeMap<String, BTreeSet<String>>, XtaskError>;
```

**行**：`in_product_graph` 的包各一行，顺序即 `members` 的顺序（按目录）。

**列**：`Directory | Package | Lib | Owns | May depend on | SPEC`。表头与单元格用英文，因为引用它的 `crates/README.md` 是英文文档。目录与 SPEC 写成仓库相对路径的代码片段，不写成链接：一个事实的读数与引用它的文档住在哪个目录无关，而相对链接的写法取决于那个目录。SPEC 一格写那个包的生效规格：目录里有 `Spec.lean` 就是 `<dir>/Spec.lean`，否则是 `<dir>/<name()>-SPEC.md`，不在盘上即以 `Doc` 拒读，因为一格指向不存在的文件比没有这一格更糟。依赖一列是块里那一行的 crate 名，按 `parse_block` 给出的次序（字母序）以逗号分隔，没有边的写 `nothing`；表与 `depmap` 门、`crate_graph` 读同一个 `parse_block`，所以三者不会读出两张依赖表。lib 一列在包没有 lib target 时写 `no lib`。duty 里的 `|` 转义成 `\|`，免得一个竖线把一行切成两格。值不带首尾换行，与 `crate_graph` 相同。

**拥有什么怎样找**：按模块名前缀找 family（D5）。family 表的解析加进 `modmap` 的 `Map`（字段 `family`，缺省为空表），与 `shapes`、`anchors` 共用 `read_map`，模块表仍只有一个读者（§7）。

**判据**：它是 docnum 的一个事实，判据就是 §8-16 那三条：区段不等于读数即红，恢复语 `cargo xtask docnum --write`。一次改名、搬目录、加一条依赖边或改一句 duty，同集跑一次 `--write`。

**败给的方案**：README 里手写这张表（第三份职责清单，下一次改名就会漏改一格）；按 `crates/*` 的目录列行（包在哪已收成 `members` 一处，按目录推导就退回了 §8-39 要拆掉的那种读者）。

**本节属门禁机具，与产品代码分开提交。**

D5 **一个 crate 拥有什么，按它的模块名前缀找 family。** `crate_table`（§8-40）的「Owns」一列取 `architecture.toml` 的 `[family.<键>] duty`，键是这个包目录下全部登记模块共有的名字前缀（`::` 之前），不是 lib 名：family 表本来就按模块名前缀取键，`sprawling` 的文件在 `crates/sprawling/src` 而模块名写 `bin::…`，所以它读 `[family.bin]`。前缀一个也没有、多于一个、或那个 family 没有 `duty`，都以 `Doc` 错误拒读并点名包与缺的那一格，不退回 lib 名。理由：前缀是模块表里已经写下的事实，按它找 family 不需要第二张「包→家族」对照表；一次改名同时动了模块名与 family 键，表就跟着走。被击败的备选有两个：在 `crate_table` 里写一个常量把 `sprawling` 映到 `bin`（包的一个属性住进了门里，改名那次不碰 xtask 也能过编译）；把 `[family.bin]` 改名 `[family.sprawling]`（family 键从此不再是模块名前缀，而 `bin::` 模块的家族要靠人记）。
-/

/-! ### 8-41 `spec`：给一个 crate 新建 `Spec.lean` 骨架（形状 2 值）

`cargo xtask spec <lib>`（`just spec <lib>`）在 `members` 给出的该包目录下新建 `Spec.lean`：开头是 Lean 注释形式的 MPL 头四行，接着一段写明 crate 名的模块文档，然后是 `skills/sdd` 的十七节，每节一个编号的 section comment（一段模块文档注释，正文以 `## <n> <标题>` 开头），标题依次是：需求分解、验收标准、假设与歧义、现状分析、权威信源、命名统一、模块边界、接口先行、工作流程、实现逻辑、边界枚举、错误处理、依赖选型、硬编码声明、影响面、测试与约束、文档关系。

```rust
// xtask::spec
pub(crate) fn run(root: &Path, lib: Option<&str>) -> Result<String, XtaskError>;
fn skeleton(lib: &str) -> String;            // 纯函数：骨架全文
const SECTIONS: [&str; 17];                  // 十七节的中文标题，按 sdd 的次序
// xtask::header
pub(crate) const NOTICE: [&str; 4];          // MPL 通告三行加版权一行，不带注释引导符
pub(crate) enum Leader { Rust, Zig, Lean }   // `//`、`//` 与 `--`
pub(crate) fn notice(leader: Leader) -> [String; 4];
```

- **只新建，不覆盖**：目录里已有 `Spec.lean` 时原样留下，答 `already exists, left untouched: <path>`。骨架之外的每一行都是施工者写下的规格，重写就会抹掉决定。
- **不看 `<lib>-SPEC.md` 在不在**：迁移中的 crate 先有候选 `Spec.lean`，旧 SPEC 在同一个 change-set 里删除（`skills/sdd` 迁移第 4、5 步）。「一个 crate 只有一份生效规格」不由生成器判，由 `spec` 门判（§8-42）。
- **不再生成 Markdown SPEC**：新 crate 从 `Spec.lean` 起步；尚未迁移的 `<lib>-SPEC.md` 由人改，迁移时整份换掉。
- **骨架只有标题，没有每节该写什么**：每节写什么由 `skills/sdd` 规定，抄进骨架就是第二份定义。旧 Markdown 的 §8.5、§12、§17、§18 迁到哪里，由 ARCHITECTURE.md §11「Specifications in Lean」规定。
- **头四行只有一处文字**：`header::NOTICE` 不带注释引导符，`notice(Leader::Rust)` 与 `notice(Leader::Zig)` 给 `header` 门（§8-48），`notice(Leader::Lean)` 给骨架。Lean 文件的头由哪道门判，不在本节。

**失败**：缺 `<lib>` 时 `XtaskError::Doc`（`usage: cargo xtask spec <lib>`）；members 里没有这个 lib 时沿用 `members::find` 的 `unknown-package`；写盘失败 `XtaskError::Io { path }`。

**测试**：`spec::tests::the_skeleton_opens_with_the_notice_and_numbers_the_seventeen_sections_in_order`：骨架前四行等于 `header::notice(Leader::Lean)`，十七段模块文档注释的标题 `## <n>` 依次编号 1 到 17。`spec::tests::a_relocated_package_gets_its_spec_skeleton_in_its_own_directory`：骨架落在 `members` 给的目录里。骨架能被 Lean 编译，由生成一份、`lake build Spec`、再删掉来确认，不写成测试。

**败给的方案**：生成器照旧写 Markdown，由迁移者手转 Lean——十七节的 Lean 形式就没有一处权威，每个迁移者各写一种。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-42 `spec` 门：一个包一份生效规格，Lean 规格守它自己的形状（形状 1 判定）

**与 `spec` 命令的关系**：同一个模块 `xtask::spec` 持有一个 crate 的 Lean 规格这件事的两面。命令 `cargo xtask spec <lib>` 新建骨架，只新建、不覆盖（§8-41）；门 `cargo xtask gates spec` 判树，不写盘。命令不判，门不写，所以一个名字不会同时是两种动作。门在名册里叫 `spec`，与 `header`、`lexicon` 一样只经 `gates` 跑，没有单独的子命令。

```rust
// xtask::spec
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError>;   // 下面五条
// xtask::spec::effective —— 第一、二条
// xtask::spec::source    —— 第三、四、五条
```

**五条断言**：

1. **一个包恰有一份生效规格。** `members` 列出的每个包（工具在内），以及不是包的检验器 `tools/adversary`（§8-47），目录里直接放着的 `*-SPEC.md` 与 `Spec.lean` 合起来恰好一份。两份都在，是一次迁移没有在一个变更集里完成（`skills/sdd` 迁移第 5 步）；一份都没有，是一个包没有规格。分部（`spec/` 下的 `.lean`）可以先于 `Spec.lean` 存在（ARCHITECTURE.md §11），不算一份规格。
2. **散文不点名树上没有的 SPEC。** 散文取 `release::is_prose` 的定义（Markdown 与 HTML 的每一行、Rust 的 `//` 行、Lean 的整份）；其中每个 `<名>-SPEC`（带不带 `.md` 都算）都要有一个叫 `<名>-SPEC.md` 的文件在树上。拒词：若有一个包的目录名是 `<名>` 且它已有 `Spec.lean`，指向那份 Lean 规格（`<lib> D<n>` 或分部的路径）；否则让作者改正名字或删掉这句。`CHANGELOG.md` 豁免：它的每一节说的是那一版的树。代码里的字符串不算——测试夹具为它造的包造出 SPEC 文件，那些名字不是给读者的引用。
3. **import 纪律。** `tools/adversary/` 下的 `.lean`，除了检验器自己的规格（§8-47），只 import `Sprawling` 或 `Sprawling.*`，以及工具链自带的 `Init`、`Std`、`Lean`；检验器的规格只 import 工具链自带的库与它自己的分部；一个包目录下的 `.lean` 只 import 工具链自带的库、本包的规格、以及 ARCHITECTURE.md §3 的 `depmap` 块允许本包依赖的包的规格。模块名到包的对应按目录：`crates.agent_protocols.spec.Harness.Session` 属于目录是 `crates/agent_protocols` 的那个包。规格 import 检验器、检验器 import 规格、一个包的规格 import 它不许依赖的包，各是一条违规。
4. **没有 `sorry`、`admit`、`axiom`。** 每个 `.lean` 去掉注释与字符串之后，不出现 `sorry` 与 `admit` 这两个词，也没有以 `axiom`（前面可以带 `private`）开头的声明。这一条原在 `just models` 的一行 grep 里，只看 `crates/`、只在有 Lean 的机器上跑；门不要 Lean，也看得到检验器。`lakefile.toml` 的 `warningAsError` 仍让 `sorry` 在构建时失败，两者判的是同一件事的两端：门在编译之前、构建在 elaborate 之后。
5. **规格引用的仓内路径在盘上。** 包目录下每个 `.lean` 里，以及检验器的规格里，首段是仓库根下一个目录、扩展名在 `release` 的 `CITED_EXTENSIONS` 里的路径，都指向一个存在的文件。`release` 判「首段不是这棵树的目录」，这里判「首段是，文件却不在」——规格点名它规定的 Rust 模块（ARCHITECTURE.md §11），模块搬走而规格没跟上，就是这一条。

**读法**：Lean 以源文本读（D8）。注释是 `--` 到行尾，与块注释从它的开记号（斜杠后跟连字符）到配对的闭记号（连字符后跟斜杠），块注释可以嵌套，字符串是 `"…"`（认 `\"`）；这三样在判第三、四条之前抹成空格，行号不变。

**失败**：读不了文件是 `XtaskError::Io`；`ARCHITECTURE.md` 没有 `depmap` 块是 `Doc`。

**败给的方案**：把第四条留在 `justfile` 里（只在有 Lean 的机器上跑，且看不到 `tools/adversary/`）；门叫 `spec_gate`（名册里的名字就是门判的那样东西，而那样东西叫 spec）；第二条只判已迁移的包（`wording` 门的 rustdoc 引过一个早已删掉的 crate 的 SPEC，那一行说明悬空引用不只来自迁移）。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-43 门读 Lean 的受限形状：锚点、kernel 的两张表、wire 的 reach，以及扫 `.lean` 的四道门（形状 1 判定）

**一个包迁没迁，只看一个事实**：它的目录里有没有 `Spec.lean`（`lean::specification`）。`specalign`、`wiring`、`spec` 与 `docnum` 的 `crate_table` 都问这一处，所以没有第二张「哪些 crate 已迁移」的名单。

```rust
// xtask::lean
pub(crate) const ENTRY: &str;                                    // "Spec.lean"
pub(crate) struct Source { pub(crate) path: String, pub(crate) text: String }
pub(crate) fn specification(root: &Path, dir: &str) -> Result<Option<Vec<Source>>, XtaskError>; // Spec.lean 加 spec/**；没有 Spec.lean 为 None
pub(crate) fn code(text: &str) -> String;                        // 注释与字符串抹成空格，行不变
pub(crate) fn inductives(code: &str) -> BTreeMap<String, Roster>; // inductive <名> where，一行一个 `| 构造子`
pub(crate) fn arms(code: &str, def: &str) -> Option<Table>;      // def <名> …，一行一个 `| .构造子 => 右边`
pub(crate) fn imports(code: &str) -> Vec<(usize, String)>;
pub(crate) fn module_file(module: &str) -> String;               // crates.<x>.spec.<A> → crates/<x>/spec/<A>.lean
pub(crate) fn dotted(dir: &str) -> String;                       // crates/x → crates.x
```

- **锚点**：已迁移的包，`architecture.toml` 里它的每一行写规定它的那个 Lean 模块名：`spec = "crates.browser.spec.Act"`，入口自己规定时写 `crates.browser.Spec`。`specalign` 判三件事：模块名换成的路径在盘上；那个文件在持有这一行文件的包的目录里（一行引的是它自己那个包的规格）；包迁没迁与锚点的写法一致——已迁移的包写 `-SPEC.md#…`、未迁移的包写 Lean 模块名，各是一条违规。未迁移的包照 §8-10 判。
- **范围**：锚点来自 `modmap::anchors`，它判除本门所在包之外的每个包（D9），所以 citysim 的行也判。
- **kernel 的两张表**（kernel 迁移之后）：kernel 的规格里写 `inductive AxCode where` 与 `def AxCode.carrier : AxCode → Carrier`，每一臂 `| .ConfigInvalid => .Loadtime` 或 `| .Timeout => .Event .ToolResult`；`inductive EventKind where` 与 `def EventKind.windowClass : EventKind → WindowClass`，每一臂的右边是 `.InWindow` 或 `.RecordOnly`。构造子名就是 Rust 的变体名，右边也一样：`Carrier` 与 `WindowClass` 在 kernel 里同样是枚举，规格里与它们同名的 `inductive` 照下一句双向对账，所以右边若写 `.loadtime`、`.inWindow` 这样的另一种拼法，同一份规格就过不了自己的名单。门把 carrier 与窗类逐臂与 `AxCode::ALL`、`EventKind::ALL` 对账，两侧的名单双向比；§8-10 的第四条改为：kernel 规格里每一个与 kernel 枚举同名的 `inductive` 都双向对账。Lean 的 `inductive` 没有省略的写法，所以没有「指路牌」这一类跳过。
- **wire 的 reach**（wire 迁移之后）：`def Command.reach : Command → Reach`，每一臂 `| .Pursue => .client`，右边是 `.client`、`.push`、`.handshake`、`.sealed` 之一。`wiring` 读这些臂，代替 wire-SPEC §19-2 的表。
- **受限的形状**：一行一个构造子或一臂；空行与注释行跳过；第一行既不以 `|` 开头、也不是空的，就结束这一块（`deriving` 行也是）。块不在、或一臂的右边不是认得的值，门以 `Doc` 判不动，而不是把缺的当成空的判绿。
- **扫 `.lean` 的四道门**：`docnum` 扫 `.md` 与 `.lean`；`lexicon` 扫 `.md`、`.rs` 与 `.lean`；`release` 把一份 `.lean` 整份当散文读（规格里的代码是模型，其中的路径都是引用，没有城里的地址），`.lean` 进 `CITED_EXTENSIONS`；`proof` 的不证理由含 `.md` 或 `.lean` 都算点名了一份文档。
- **`check-branch`**：分支的 diff 里有 `.lean`，或有 Lean 包的三个文件（`lakefile.toml`、`lean-toolchain`、`lake-manifest.json`）时，多跑一步 `just models`。`members --owning` 早已略过 `.lean`，所以改 Lean 不会选中 Rust 测试。

**测试**：每条都用测试自己造的夹具：kernel 的 Lean 规格少一个构造子、一臂的 carrier 与枚举不符、锚点指向不存在的分部、已迁移的包写 Markdown 锚点、citysim 的一行指向不存在的节、wire 的 reach 臂读成表、`.lean` 进了 `docnum` 与 `lexicon` 的扫描面、Lean 的块注释里点名一份不在树上的文档、`.lean` 作为不证理由。

D8 **门把 Lean 当文本读，不跑 Lean。** `spec`、`specalign`、`wiring` 读一份 Lean 规格时读的是源文本里几种受限的形状（§8-43）：一行一个构造子的 `inductive`、一行一臂的 `def`、`import` 行、去掉注释与字符串之后剩下的词。Lean 与 Rust 两侧的名字逐字相同，门比的是同一个拼写，不做大小写或下划线的换算——换算规则本身就是第二套文法。理由：门在 `gates` 里并行，每道毫秒到百毫秒；判一张表若要先 `lake build` 再读 Lean 的输出，门就依赖构建的次序，而证明本来已经归 `just models`。Lean 自己守着受限形状背后的那一半：一个 `def` 的 match 漏了构造子，`lake build` 编不过，所以门只需判「两侧的名单与映射相同」。被击败的备选：写一个 Lean 程序在 elaborate 之后把名单导成 JSON 给门读——门从此要先等一次构建，而那个导出程序是名单的第二个读法，要自己的测试与自己的格式。**重开参数**：某张表的形状在一行一臂里写不下（例如一个臂要按参数再分支），那时这张表改由 Lean 侧导出。
-/

/-! ### 8-44 `shots`：每一页、每个状态 × 1440／1920 × 暗／亮的 PNG 与索引（形状 4 适配器）

`cargo xtask shots [--origin <url>]`（`just shots`，先跑 `build-web`）把每一页拍成 PNG，写进 `target/shots/`，并在同一目录写一份索引 `index.md`。决定见 D10。

```rust
// xtask::shots —— 路由：找产物、找引擎、按页拍、写索引、核对每张图都在
pub(crate) fn run(root: &Path, args: &[String]) -> Result<String, XtaskError>;
// xtask::shots::pages —— 形状 1 判定：拍哪些、叫什么、缺哪些
pub(super) const FRAMES: [Frame; 2];          // 1440×900、1920×1080
pub(super) enum Lighting { Dark, Light }
pub(super) struct Shot { /* route, fold, folds, frame, lighting, states */ }
pub(super) fn routes_in(route_ts: &str) -> Vec<String>;   // BARE 的键，去掉空键，按表序
pub(super) fn missing(dir: &Path, shots: &[Shot]) -> Vec<String>;   // 没写出或写空的文件
pub(super) fn index(shots: &[Shot]) -> String;
// xtask::shots::camera —— 形状 4 适配器：开引擎、量页、拍一张
```

- **一页是一条路由在一个窗口里画出的样子，一页里的状态按屏往下拍。** 内容在 `<main>` 里滚动，所以先用 `--dump-dom` 开一次，量 `<main>` 的滚动高度与可见高度，得出要几屏（`fold`）；再每屏、每种光照开一次，用 `--screenshot` 拍下 `<main>` 滚到那一屏时的窗口。量的那一次同时记下每个顶层 `section[aria-label]` 的位置，索引里每张图后面列出落在这一屏里的状态名。文件名 `<route>-<fold 两位>-<宽>-<光照>.png`。
- **光照经引擎的配色偏好给，不改页面。** 每次运行用一个新的引擎资料目录，页面里的光照设置因此是缺省的 `system`，按 `prefers-color-scheme` 画；引擎开关 `--blink-settings=preferredColorScheme=0` 是暗、`=1` 是亮。这台机器自己的明暗设置于是不影响结果：不带这个开关时，无头引擎跟着操作系统走。
- **两种来源**：缺省开 `just build-web` 产出的包，与 `render` 一样给包旁边写一份插了脚本的副本（`sprawling-shots.html`，带 `render::engine::preloads` 给出的预取行），跑完删掉；给了 `--origin` 时，路由页开 `<origin>/#/<route>`，那是一座正在服务的城，页面上是真数据，页面不能插脚本，所以每页只拍一屏。
- **拍完核对**：每张计划中的图都要在、都不为空，否则整次运行以 `XtaskError::Cmd` 失败并列出缺的文件名——引擎有时退出了却没写图，一份缺图的索引读起来像是那一页什么都没有。
- **失败**：没有包时 `XtaskError::Doc`，recovery 是 `just build-web`；没有引擎时 `XtaskError::Doc`，recovery 是装一个 Chromium 一族的浏览器或设 `SPRAWLING_BROWSER`；读不出 `BARE` 时 `XtaskError::Doc` 点名 `route.ts`；引擎失败沿用 `render::engine::dump` 的 `XtaskError::Cmd`。

**测试**：`shots::pages::tests::every_route_gets_four_pictures`：一份夹具 `route.ts` 读出的每条路由，在两个宽度、两种光照下各有一张图，夹具目录里缺一张时 `missing` 恰好点名那一张。真跑要先 `just build-web`，归整合者或前端会话。

**本节属门禁机具，与产品代码分开提交。**

D10 **截图矩阵是给人看的产物，不是门。** `cargo xtask shots`（§8-44）不进 `gates::GATES`，不断言任何性质，也不比较两张图：它产出每一页在两个宽度、两种光照下的 PNG 与一份索引，给改画面的人与验收的人逐张看。理由与 `render` 断性质、不断图片（本规格的「`render`」一节）是同一条：截图对比会被字体 hinting 弄红，也放过没人拍过的错版面，所以机器判性质，人判图片，两件事各有一个工具。页面清单不另写一份：路由取 `client/src/core/route.ts` 里 `BARE` 那张表的键，一个页面里的状态取画出来的页面上每个带 `aria-label` 的顶层 `section`（`#/gallery` 的每个夹具就是这样画的），所以前端改了外壳或加了夹具，这个工具不用跟着改。浏览器只经 `render::engine::browser` 找，与 `render` 门在同样的地方找同样的牌子。被击败的备选：①把截图当 `render` 的第六次开页——门就要为一件不判的事多开一次引擎，门名册上也多一个不会变红的步骤；②在 xtask 里写一张页面清单——前端加一条路由而这里没加，那一页就悄悄没有图。
-/

/-! ### 8-45 `wiring::class`：§19-2 的 `class` 列与中继的匹配是一个决定（形状 1 判定）

```rust
enum Class { Read, Act, LocalOnly }                                         // 拼法同 remote_access::door::VerbClass
fn stated(root: &Path) -> Result<BTreeMap<String, Option<Class>>, XtaskError>;   // 表的第三格；Lean 时读 def Command.verbClass
fn coded(root: &Path) -> Result<BTreeMap<String, Class>, XtaskError>;            // sprawling 包里 fn command_class 的每一臂
fn judged(all: &[String], stated: &…, coded: &…) -> Vec<Violation>;
```

- **表说给读者，匹配说给设备**：wire-SPEC §19-2 的 `class` 列是远程设备的一帧属于哪一类的陈述，`bin::outside::verbs::command_class` 是中继据以放行的穷尽匹配（wire-SPEC §8-65）。两者谁也推不出谁，门把两者逐个动词对照，`wiring` 的同一次运行里报出。
- **缺格点名那一行**：表里的一行只写了 reach、没写 class（第三格读不成 `Read`／`Act`／`LocalOnly` 之一），违规写「the row for `<动词>` … has no class」，替代办法是写 `LocalOnly`，除非人决定了城外的设备可以做它。新加 Command 的车道照旧格式加行而不写这一格时，就是这一条红。
- **两边说法不一点名动词与两个值**：「`<动词>` is <表的值> in the table and <匹配的值> in `command_class`」。
- **匹配按文本读**：从 `fn command_class` 读到函数结束，每个 `Command::<动词>` 归到它后面第一个 `=> VerbClass::<值>`。函数按包找（`members` 的 `sprawling`），与 `performed` 按包找 `run_command` 同一个理由：拆文件不让门读空。找不到这个函数以 `Doc` 拒判。
- wire 的规格迁成 Lean 后，`class` 写成 `def Command.verbClass` 的一臂一行，右侧是 `.read`／`.act`／`.localOnly`；读法与 reach 的 `def Command.reach` 相同（§8-43）。

**测试**：`wiring::class::tests::a_row_with_no_class_cell_is_named_by_its_verb`：夹具表里一行缺 class、一行与夹具匹配说法不一，两条违规按动词点名。

**本节属门禁机具，与产品代码分开提交。**

D11 **动词类由表与中继的匹配各写一次，门对照两者。** `wiring` 读 §19-2 的 `class` 列，也读 `sprawling` 包里 `command_class` 的每一臂，逐个动词比（§8-45）。理由：表是人读到并决定的地方，匹配是设备真正碰到的地方，两者都要；只有一处时，要么决定藏在代码里没人读，要么二进制要在启动时解析一份 Markdown 来授权。按文本读匹配而不是用 `syn` 解析：门要的只是「哪个动词归哪个值」，臂的形状由穷尽匹配自己守着，`syn` 的模式树在大版本之间换形，门不该为一张两列的对照表跟着它走。被击败的备选：①只判表里每行都有 class，不读代码——表与中继各自漂开时没有红；②在 wire 里给 `Command` 加 `class()` 再让门读它——wire 要么依赖 `remote_access`，要么另立一个同值的枚举（wire-SPEC §12.6）。重开参数：`command_class` 的臂长出守卫条件（同一个动词按参数分两类）时，文本读法不够，改用 `syn`。
-/

/-! ### 8-46 `guard::wall` 判唯一一张自己的 lint 表（形状 1 判定）

`wall::check` 读根 `Cargo.toml` 的 `[workspace.lints]`，再经 `members`（§8-39）读每个成员的清单：

- **叶子**（目录是 `LEAF`，`crates/desktop/ffi`）：它的 `[lints.rust]`／`[lints.clippy]` 与根 `[workspace.lints.rust]`／`[workspace.lints.clippy]` 逐键比对，两个方向都比，例外只在 `RECORDED`（今天一行：`rust.unsafe_code`，理由见 `crates/desktop/Spec.lean` D14）。不相等且不在 `RECORDED` 即红，位置写 `crates/desktop/ffi/Cargo.toml [lints.<表>] <键>`；在 `RECORDED` 而两侧已相等，红在那一行上，替代写「删去这一行」。
- **其余成员**：`[lints]` 恰是 `workspace = true`；不是即红，位置写那个成员的 `Cargo.toml [lints]`，替代写「改成 `workspace = true`」。
- **叶子不在成员里**：红在 `LEAF` 上，替代写「划掉 `LEAF` 与它在 `RECORDED` 里的行」，因为一条没有对象的例外是谁都没有再决定过要给的许可。
- 读不出的清单是 `XtaskError::Doc`。

**测试**：`guard::wall::tests` 在 `judge` 上判夹具清单：叶子的表与根表只差 `unsafe_code` 时无违规；少一条、放宽一条、多一条各一例；一个别的成员写了自己的表；叶子不在成员里；`unsafe_code` 两侧相等时那一行须划掉。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ### 8-47 `spec` 门认得检验器自己的规格（形状 1 判定）

检验器 `tools/adversary` 不是 cargo 的包，`members` 列不出它，而它的规格从 Markdown 迁到 Lean 时落在 `tools/adversary/Spec.lean` 与 `tools/adversary/spec/`（ARCHITECTURE.md §11，`lakefile.toml` 为它加 `tools.adversary.Spec` 与 `tools.adversary.spec.+` 两个 glob）。门因此按位置分出第三种文件主人：

```rust
// xtask::spec
const CHECKER: &str = "tools/adversary";        // 检验器的目录，两个子模块共读这一处
// xtask::spec::source
enum Owner<'a> { Checker, CheckerSpecification, Package(&'a Member) }
// xtask::spec::effective
pub(super) fn one_per_package(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError>;  // 包，再加检验器
```

- **检验器的规格**是 `tools/adversary/Spec.lean` 与 `tools/adversary/spec/` 下的 `.lean`。它只 import 工具链自带的库与 `tools.adversary.Spec`、`tools.adversary.spec.*`；import `Sprawling`（检验器）或任何一个 crate 的规格都是一条违规，拒词点名那个模块，替代是「import 工具链的库或本规格的分部」。理由是 ARCHITECTURE.md §11 那一句「no part imports the checker」：规格陈述检验器要守的性质，检验器实现它们，一份 import 了实现的规格就成了实现的第二份读法。
- **检验器**是 `tools/adversary/` 下其余的 `.lean`，规则不变：只 import `Sprawling` 与工具链的库，所以它也 import 不到自己的规格。
- **一份生效规格**：第一条断言在包之外再判 `tools/adversary` 一次，那里的 `*-SPEC.md` 与 `Spec.lean` 合起来至多一份。
- **引用的路径在盘上**：第五条也读检验器的规格。

**测试**：`spec::tests` 在夹具上判：检验器的入口 import 自己的分部不报；分部 import `Sprawling.Door` 报；检验器的源文件 import 规格报；规格引用一个不在盘上的路径报；Markdown 与 Lean 两份规格并存报。

**本节属门禁机具，与产品代码分开提交。**

D13 **检验器的规格由位置认出，不由 cargo 认出。** `spec` 门按路径把 `tools/adversary/Spec.lean` 与 `tools/adversary/spec/` 认作检验器的规格（§8-47），常量 `CHECKER` 是检验器目录的唯一拼写。理由：检验器是一个 Lean 程序，不进 workspace 也不进发布物（`tools/adversary/Spec.lean`），cargo 没有一个包能代表它；而它的规格与 crate 的规格守同一组规则——一份生效规格、不 import 检验器、引用的路径在盘上——所以门要认得它，否则它整份落在「检验器」那一侧，入口 import 自己的分部会被报成违规，分部 import 检验器反而放行。被击败的备选：①给检验器造一个只为门存在的 `Cargo.toml`——一个包从此出现在 `members`、depmap、modmap、`unused` 的每一次遍历里，为了一个不编译 Rust 的目录；②在 `architecture.toml` 里登记它——那张表登记 Rust 模块，检验器的规格不是任何 Rust 模块的锚点。**重开参数**：第二个不是 cargo 包的目录也有了 Lean 规格（例如 `client` 的 §7），那时位置表从一个常量变成一张表。
-/

/-! ### 8-48 `header`、`length`、`modmap` 读 `.zig`（形状 1 判定）

ARCHITECTURE §2 准一片 Zig 叶子进树的第 5 条要求这三道门读 `.zig`。今天树里的 `.zig` 只有 `crates/desktop/ffi/zig/` 下的三份，三道门按同一个扩展名认它们，不按这个目录认，所以下一片叶子落在哪里都一样受判。

```rust
// xtask::header
pub(crate) enum Leader { Rust, Zig, Lean }   // Zig 的行注释与 Rust 同为 `//`，正文仍是 NOTICE
// xtask::length::zig —— 形状 1 判定：一份 Zig 源码的函数与生产行，按 Zig 的词法读
pub(super) fn measure(text: &str) -> Vec<Body>;          // 每个具名且带函数体的 `fn`，`test` 声明里的除外
pub(super) fn production_lines(text: &str) -> usize;     // 总行数减去 `test` 声明所跨的行
// xtask::length::measurement
pub(crate) struct Body { name: String, line: usize, lines: usize }   // 一个函数体从 `fn` 到右花括号
pub(crate) struct Found { body: Body, args: usize }                   // Rust 一侧多一个参数数
```

- **header**：同一次全树遍历收 `.rs` 与 `.zig`，引导符按扩展名取 `Leader`；判据不变：前四行逐字节等于 `notice(leader)`，且第二份头即红。违规的规则写「every .rs and .zig file carries …」。
- **length**：每个包（§8-39）目录下的每个 `.zig`，嵌套包里的文件只量一次。文件面与函数面用 Rust 同一组预算（`budgets.toml` 的 `file_length`、`function_length`），同一张 `[file_length.predating]` 登记表、同一条 `data` 形状豁免；参数面不判 Zig（D15）。函数从 `fn` 那一行量到函数体的右花括号那一行，与 Rust 一侧取 `fn` 记号与块尾的量法相同；没有函数体的原型（`extern` 声明）不量；`test` 声明之内的函数不量，`test` 声明所跨的行不计入文件面。
- **modmap**：受判包目录下的每个 `.zig` 都要登记，不论在不在 `src/` 下——Zig 文件没有 `build.rs` 那种不是模块的文件，也没有索引文件豁免。条目以 `.rs` 或 `.zig` 结尾才进判定。行名写 `<family>::…::zig::<文件名>`，`desktop_ffi` 的三份是 `desktop::ffi::zig::boundary`、`desktop::ffi::zig::leaf`、`desktop::ffi::zig::step`，family 仍是 `desktop`（`family_of` 按名字前缀取）。磁盘一侧按仓库相对路径去重：`crates/desktop` 的目录包含 `crates/desktop/ffi`，同一个文件不报两次。
- **失败**：`.zig` 读不出是 `XtaskError::Io`；词法上不完整的文件（一个函数体没有右花括号）不是判不动，那个函数量到文件末尾，因为 `zig fmt --check` 与构建会先拒这份文件。

**测试**：每道门在自建的夹具上判一份 `.zig`：`header::tests::a_zig_file_without_the_header_is_named`（一份缺头的 `.zig` 与一份带头的 `.rs`，只点名前者）；`length::tests::a_long_zig_file_and_a_long_zig_function_are_named`（`root::fixture::relocated` 上一份超过文件预算、含一个超过函数预算的函数的 `.zig`，两条都点名它；`test` 声明里同样长的函数不报）；`length::zig` 的单测：返回类型里的 `union(enum) { … }`、`error{ … }` 不是函数体，字符字面量 `'{'`、字符串与 `\\` 多行字符串里的花括号不计，`extern` 原型不量，`test` 声明的行不计入文件面；`modmap::tests::an_unregistered_zig_file_is_named`（夹具里一份未登记的 `.zig` 被点名，一份登记了的不报）。

**本节属门禁机具，与产品代码分开提交。**

D15 **Zig 的函数按 Zig 的词法量，不引解析器；参数面不判 Zig；`test` 声明不计。** `length` 量 Rust 用 `syn`，§13 记着按行数括号会错的三处：`#[cfg(test)]` 被当成文件截断点、`'{'` 被当成开括号、跨行字符串。Zig 的词法让这三处各有一个确定的答案：语言参考的 Comments 一节写「There are no multiline comments in Zig (e.g. like /* */ comments in C). This allows Zig to have the property that each line of code can be tokenized out of context.」；文法里 `string_char` 与 `char_char` 都排除换行，只有 `\\` 开头的多行字符串跨行，而它像注释一样到行尾为止；`test` 是关键字，测试只能写成 `test` 声明。于是 `length::zig` 只认行注释、字符串、字符字面量与多行字符串四种要跳过的东西，剩下的花括号与圆括号就是结构。函数体之前唯一会出现花括号的地方是返回类型（文法 `FnProto` 的 `TypeExpr`），那里的花括号只跟在 `error`、`struct`、`enum`、`union`、`opaque`（可带一个括号里的参数）、`switch (…)` 或一个块标签之后，量法跳过这些组，第一个别的 `{` 就是函数体。**参数面不判 Zig**：本仓的 Zig 只做 C ABI 叶子，一段缓冲过边界是 `(ptr, len)` 两个参数，`extern` 原型是 Win32 自己的签名，把它们收进一个 `extern struct` 就为一条管 Rust 数据团的规则改了边界的 ABI。**`test` 声明不计入文件面**：它只在 `zig test` 下编译，`build.rs` 链进二进制的库里没有它，与 Rust 的 `#[cfg(test)]` 同理。被击败的备选：①调 `zig ast-check` 或 `zig fmt` 取结构——每台跑 `cargo xtask gates` 的机器就都得装 Zig，而 Zig 今天只是编叶子的前置，且没有一种稳定的 AST 输出；②引 tree-sitter-zig——为三份文件往工作区工具链里加一个 C 构建依赖；③像客户端一样只判文件面——客户端不量函数是因为 TypeScript 的词法数不准括号，Zig 数得准，这个理由在这里不成立。重开参数：本仓进来一份不是 C ABI 叶子的 Zig（参数面随之进来），或 Zig 的文法让返回类型里出现上面五种之外的花括号。
-/

/-! ### 8-49 可发布的包只编译它自己带着的文件：`packaged` 门与 `guard::version`（形状 1 判定）

**要挡的事**：`cargo package` 把一个包目录里的文件打进 `.crate`，crates.io 上只有这份文件；验证构建与 `cargo install` 都在解开的包里编译。一个 `include_str!("../../../docs/x.md")` 在工作区里编得过，在包里指向 `target/package/` 或 registry 目录之外不存在的文件，构建失败；而这个失败只在有人发布或安装时才看得见。依赖那一侧同理：一个 path 依赖没有 `version` 就发布不了，`version` 与被依赖包的版本不一致时 cargo 拒绝解析。

```rust
// xtask::packaged
pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError>;
// xtask::guard::version
pub(super) fn pins(workspace: &toml::Value, members: &[(String, toml::Value)]) -> Vec<Violation>;
// xtask::package
pub(crate) fn workspace_package(root: &Path, key: &str) -> Result<String, XtaskError>; // 根清单 `[workspace.package]` 的一个字符串
pub(crate) fn package_field<'a>(manifest: &'a toml::Value, key: &str) -> Option<&'a str>;
```

**`packaged` 判什么**：

- **哪些包**：`members`（§8-39）里 `publish` 为 `Registry` 的包；`publish = false` 的工具包（xtask、citysim）不判，因为它们不会被打包。
- **哪些文件**：从每个包的 `roots`（lib、bin、构建脚本、过程宏的源文件）出发，沿 `mod` 声明走。没有 `#[path]` 的 `mod name;` 落在「子模块目录」下的 `name.rs` 或 `name/mod.rs`：`lib.rs`、`main.rs`、`mod.rs`、构建脚本与经 `#[path]` 载入的文件，子模块目录就是文件自己的目录；其余文件是自己目录下与文件同名（去掉 `.rs`）的子目录；内联 `mod a { … }` 在其上再加一段 `a`。带 `#[path = "p"]` 的声明落在声明它的文件的目录加 `p`（内联块里再加内联模块那几段）。cfg 谓词在 `test` 为假时不能成立的项不进（D16）：它是 `test`；或它是 `all(…)` 且某一支如此；或它是 `any(…)` 且每一支都如此；`not(…)` 与别的谓词都算可能成立。一个声明找不到文件，门判不动，`XtaskError::Doc` 点名声明它的文件与模块名。
- **哪些调用**：走到的每个文件里、不在上面那种测试项内的 `include!`、`include_str!`、`include_bytes!`，连同嵌在别的宏的记号里的。参数有三种读法：字符串字面量按文件所在目录解析；`concat!(env!("OUT_DIR"), …)` 是构建脚本的输出，不判；`concat!(env!("CARGO_MANIFEST_DIR"), "…", …)` 按包目录接上其余字面量。别的写法门读不出它指向哪里，一律是一条违规，替代是改成上面三种之一。
- **判据**：解析出的路径按词法消去 `.` 与 `..` 之后，落在包目录之内，且不落在嵌套在其中的另一个成员的目录里（cargo 打包时把带 `Cargo.toml` 的子目录整个留下）。违规的位置是 `<文件>:<行>`，规则写「a published package compiles only files it carries」，替代写「把文件挪进拥有它的包；或由构建脚本找到它、写进 `OUT_DIR`；或让那个包交出一个常量给这里读」。

**`guard::version` 判什么**：根清单 `[workspace.dependencies]` 里每一个带 `path` 的项，`version` 恰是 `=` 加 `[workspace.package] version`；没有、或写成别的，红在 `Cargo.toml [workspace.dependencies] <键>` 上，替代给出应写的那一行。成员清单的任何依赖表（`dependencies`、`dev-dependencies`、`build-dependencies`，以及 `target.<cfg>` 下的同名表）里带 `path` 的项都红在 `<目录>/Cargo.toml <表> <键>` 上，替代写「改成 `<键> = { workspace = true }`」：路径与版本只在根清单写一次。版本的唯一权威仍是 `[workspace.package] version`，guard 不另抄一份，只判那十几行 `=` 钉子与它相等；cargo 的依赖表不能写 `version.workspace`，所以钉子必然是抄件，这道判定就是它们的持有者。

**`xtask::package` 的 `workspace_package`**：`[workspace.package]` 的字段只经它读。`package` 与 `channel` 取 `version`，`channel` 取 `repository` 写进每个 npm 包的 `repository` 与 `homepage`（原先是 `channel.rs` 里的常量 `REPOSITORY`，与根清单是同一个事实的两个家）；guard 经 `package_field` 读已解析的根清单。

**测试**：`packaged::tests` 在 `root::fixture` 上建一个小工作区：一个可发布的包的 `lib.rs` 写 `include_str!("../../x.txt")` 必须报一条违规并点名那一行；包内的 `include_str!`、`concat!(env!("OUT_DIR"), …)`、`#[cfg(test)] mod tests;` 声明的文件里的包外 `include_str!`、`#[path]` 载入的文件里的包外 `include_bytes!` 各一例，只有最后一例报；`publish = false` 的包里的包外 `include_str!` 不报。`guard::version::tests` 在夹具清单上判：工作区 `0.0.8` 而某项写 `=0.0.7` 报一条；缺 `version` 报一条；一个成员按路径点名工作区包报一条；全都对时无违规。

**本节属门禁机具，与产品代码分开提交。**
-/

/-! ## 9 工作流程

`cargo xtask <gate>` → 定位仓库根（`root::judged`，见 D3）→ 读数据面（`architecture.toml`／ARCHITECTURE.md／`lexicon.toml`／`budgets.toml`）→ 纯函数判定 → 渲染违规 → 退出码。`gates` 每道门各占一条 `thread::scope` 线程并行判定，按门序汇合结果后统一渲染（§8-31）；汇合取哪个退出码，见 `spec/Report.lean`。

D3 **判的是哪棵树。** 仓库根取「当前目录往上第一个含 `Cargo.lock` 的目录」（`root::checkout_of`），与编进二进制的根（从 `CARGO_MANIFEST_DIR` 往上按同一条规则找到的目录）规范化后比较；cargo 把锁文件写在工作区根，所以这条规则与 xtask 住在哪一层无关，测试要本检出的根时也经 `checkout_of`，不按层数往上数父目录；两者不是同一目录时以 `StaleBuild` 拒判，恢复是 `cargo clean -p xtask` 后重跑；当前目录往上没有检出时以 `NoCheckout` 拒判。理由：cargo 对工作区成员的指纹只比源文件的相对路径与 mtime，不比 `CARGO_MANIFEST_DIR`；一个 target 目录从另一份检出复制过来后，`cargo xtask` 报 `Fresh`，跑的是那份检出编出的 xtask，而编译期的根把它钉在那份检出上，缺 MPL 头的文件因此可以报 `ok`。被击败的备选有三个：只在运行时取根（二进制的门逻辑本身也是另一份检出的，拿旧门判新树同样静默通过）；只认编译期的根（就是上面那次静默通过）；调 `git rev-parse --show-toplevel`（往上找一个文件是微秒级，起一个 git 进程在 Windows 上是十毫秒级，而规则与 cargo 找工作区的方式相同）。
-/

/-! ## 10 实现逻辑

1. **walk**：手写递归（不引 walkdir），跳过 `walk::SKIP_DIRS` 的五个构建目录名（`target`、`node_modules`、`.lake`、`.svelte-check`、`web-dist`），也不进根以下自带 `.git` 条目的目录（另一份检出）；名为 `.git` 的条目按结构跳过，不在表里。输出按路径字符串排序——报告顺序确定，diff 可比。路径统一正斜杠（Windows 反斜杠归一），因为模块表以正斜杠书写。理由见 §8「扫描面」一节。**隔离区**：仓库根 `local/`（gitignore，恒不入库）存一台机器自己的工作记录；从仓库根扫描的四门（header／lexicon／secret／color）排除它——门只对入库对象作证。modmap 扫除本门所在包之外每个包的目录，depmap 扫产品包的目录（§8-39），包目录里嵌套的 `local/` 仍被封闭清单咬住。
2. **modmap**：读 `architecture.toml` 的 `module` 条目；只判 `name` 含 `::`、`file` 落在某个受判包的目录（§8-39 的全部包减去本门所在的包，D9）之下且以 `.rs` 或 `.zig` 结尾的条目，磁盘一侧遍历同一组目录里 `src/` 下的 `.rs` 与任何位置的 `.zig`（§8-48），嵌套包的文件只判一次，状态取 `planned`／`building`／`built`／`frozen` 之一。双向对账：表有文件无（状态不是 `planned` 才要求在盘）；盘有表无（lib.rs 与索引文件豁免）；盘有而状态仍是 `planned` →「状态未翻转」。同一文件两个条目即红。索引文件的依据：文件名去 `.rs` 后与同目录某子目录同名，且该子目录内有表内文件。
3. **depmap**：ARCHITECTURE §3 的 `depmap` 围栏块是 crate 边的机器权威；包与它的依赖取自 `members`（§8-39），块里的键是包的 lib 名，一条依赖边以被依赖包的 lib 名比对，工具包不进产品图；只查 normal 与 build 依赖（dev 依赖留给测试自由）。断言是子集而不是相等：文档可以先写下一条尚未使用的边。`directions` 块判一个 crate 之内的模块方向（§8-33）。
4. **guard**：`wall` 把叶子的 `[lints]` 与根 `[workspace.lints]` 逐键比对，再判其余成员都继承根表（`spec/Guard/Wall.lean`）；`version` 判工作区自己的包在 `[workspace.dependencies]` 里各钉 `=` 加 `[workspace.package] version`，且没有成员绕过那张表按路径点名一个工作区包（§8-49）；只读工作树，不调 git。
5. **vocabulary（挂在 lexicon 门下）**：**退役词必须指向被定义过的词**——`lexicon.toml` 说哪种说法作废，`docs/glossary.md` 说该用哪个词；二者不对账时，一条退役词可以指向一个词汇表从未定义的名字，照门的建议改词的人会落到一个没有释义的词上。依据宽一格：replacement 命中任一词汇表**粗体词**或含 `.md`（指向一份文件也是一种定义）。文档里的门数不在这里对账：`docnum` 的 `gate_count` 从 `gates::COUNT` 重算它，一个数只有一个重算者。重算只到受管标记为止：标记之外用数字或数词写出的门数，没有任何一道门读它，所以文档只在 `gate_count` 标记里写门数，别处写「全部门」，评审守这一条。
6. **release**：公开树**由过滤生成**而不由手工挑选，分类是一条**封闭的前缀规则**（`is_scaffolding`）；未被规则点名的一律归产品面——**失败方向是故意的**：未分类的文件出现在产物里会被人看见，反过来则无声消失。其中三条的依据值得写下来：①公开树上零脚手架路径；②产品文档不得链向或在正文里点名脚手架（无链的「去看 SPEC」最好写也最难发现，故扫全文而不只扫链接）；③**任何发布文件不得携家目录路径**（`machine_path`）。第三条的口径是**隐私而非整洁**：`/tmp`、`/etc`、`C:/windows` 是关于一类机器的事实，而且「绝对路径被拒」那三条测试必须写出一个绝对路径，故规则收窄到家目录形状（`:\users\`／`:/users/`／`/home/`／`/root/` 等七种，大小写不计）。扫描面是**全部可读成文本的发布文件**，不只 `.md`：源码与清单里的硬编码家目录更坏而不是更好。报告只截二十字符，因为把整行引进 CI 日志就是把它再公开一次；文件自豁免（同 secret／color 两门：写不出不包含待检形状的检测器）。本规格的入口也在 `DETECTORS` 里，理由与 `AGENTS.md` 在那里相同：一份写下这条规矩的文档必须说得出它禁的是什么。

   **第五条断言：发决定，不发场合。** 一份产品文档说的是决定，不是决定发生的场合。这一条与第三条同性质而更宽一类——家目录有形状，而「这台机器验不了什么」写成散文时没有形状。能查的部分收成两张封闭的字面表：

   - `WORKING_SHAPES`：`本机`（意思是**我的**那台机器；本地那台在这座城里叫 `回环` 或 `这台机器`，两者说的都是「正在跑的那台」而不是作者的那台）、`前端会话`。
   - `ADDRESSEE_SHAPES`：`待人裁`、`立场归人`、`这一步我不做`——把一句话变成一个读者不在场的对话回合，且决定一落地它就过期，那比从没写过更坏。
   - 另加一条带数字规则的形状：`会话` 接**一位**数字。第二位数字即不算，因为 `会话` 也是产品自己的词，而量它的探针会数它（`4 会话 29 ms`）。一个计数很少停在一位数，界就划在数字结束的地方。

   **`裁`／`判` 本身不是病**：`判定`／`判断` 是这个项目的常用词，一条拒词里的「请人裁」说的是**系统升级给人**这一产品行为，权威阶梯点名那个人是刻意的。只有文档用自己的声音指定收信人才是缺陷。

   **中文不写词界，所以一张字面表要配一张「吞掉形状的词」表**：`版本机制` 是这套词汇里唯一的一个。两个词的封闭名单查得动，一个分词器查不动。

   **这一条抓不住的部分写在明处**：写成普通散文的工作语境没有形状——「只验得了三分之一」、一句对话的逐字引用、一个账号的额度状态，门读到的都是句子。那一半由评审持，`xtask secret` 的模块头为同一理由写着同一句话。`AGENTS.md` 进 `DETECTORS` 表：一份教这条规矩的文档必须说得出它禁的是什么，而它那两行**描述形状而不拼出形状**，并在同一句里说明为什么。
7. **length**：尺寸有**两个单位**，因为两者的失效方式不同——长函数藏起一条控制流，长文件藏起「东西在哪」。
   **文件面带一张先于规则存在的文件登记表**（`[file_length.predating]`），每个文件钉在划线时的行数上。**这张表只会变短**：表上没有的文件直接按预算拒绝，所以它不会变长；表上的文件不得超过自己的钉子，所以没有一个欠债会长大；而一个回到预算之内的文件必须从表上划掉，所以豁免会自己消失，不需要谁记得它（三件事是 `spec/Length.lean` 的 `an_unpinned_file_passes_exactly_inside_the_budget`、`a_pinned_file_never_grows`、`a_kept_pin_is_still_needed`）。**删一行的办法是把文件拆了，不是把数字改大。** 重开参数：在一个超长文件上迭代的代价低于拆分一次的代价时，文件面的预算才值得放宽。
   **参数面**：一条参数表长过预算就是一个 data clump——总是一起走的那几个值，是一个还没被命名的值。本仓库已经写下过这个修法：`Reporter` 的 doc 说「四个值总是一起走、从不被单独选择，所以它们作为一个走」。预算比 Clean Code 的 3 宽一格，因为三字段值的构造函数正当地需要三个，门不该跟它们吵。接收者不算：`&self` 是这个函数之所以是方法的原因，不是谁决定要穿过去的值。豁免表是一张名字数组（`文件路径::函数名`），**表上没有的名字直接拒绝**，划掉一个名字的办法是给那几个值起个名字，不是把预算调大。一条断言核对表上每个名字仍然存在且仍然超标，所以一个已经修好的豁免不会留在那里等下一个人花掉。**一个参数很多的私有方法，就是策略没有对象可住时的样子**，故参数超标的地方往往也是文件超标的地方。
   **文件面只数生产行**：顶层 `#[cfg(test)]` 项（内联 `mod tests`、测试专用函数）所跨的行从文件总行数里减去。文件预算要限制的是一个模块持有多少生产策略；把内联测试也算进去，逼人为了挪测试而拆模块，拆出来的是一次与接口无关的移动。**扫描面**：每个包（§8-39）的 `src/`、每个包目录下的 `.zig`（§8-48、D15）与 `client/src`；`tests/` 与 `benches/` 不在内，因为测试代码本就允许放松约束（AGENTS.md）。**客户端只受文件面，不受函数面**：量一个函数要解析它写成的那门语言，`syn` 解析 Rust，而为一道门往工作区清单里加一个 TypeScript 解析器不成立；数括号的量法会量错（§13），故客户端的函数长度是**未量且明说未量**，而不是量错。**生成物两面都不量**：`client/src/wire.ts` 是 `cargo xtask wire-ts` 从 Rust 线面写出来的，拆它就是拆生成器的输出；豁免的依据是生成器自己写在文件头上的那一行横幅，不是门里的一条路径。**两类不量**：① 带 `#[cfg(test)]` 的项（它标的是**一个项**而不是文件剩下的部分）；② 模块表形状列为 `data` 的文件（ARCHITECTURE §9 形状 6：数据而无分支）。**两类豁免都取自已有权威**（属性、模块表），而不是新建一张名单——一张名单就是一个可以悄悄变长的豁免口。形状列由 `modmap::shapes` 交出，与 modmap 共用同一个解析器。
8. **报告**：三段式渲染，与产品的 Gate 拒绝同构——施工者被拒时拿到的也是「规则｜违反点｜替代」，不是一句 fail。

### 两个设计

**模块图 A（选中）：结构化清单 `architecture.toml`**——modmap 读它的字段（`name`、`file`、`status`、`owns`、形状）。杠杆：一个结构化文件没有「位置」要人维护，读者可以按名字问一个模块，而不是在一张七百行的表里扫。
**模块图 B（落选）：modmap 直接解析 ARCHITECTURE.md 里的竖线表。** 单一权威、改表即改门，但每一行的位置与每个标题下的行数都由人维护，行数会在行都对的时候陈旧，而这份表占掉了架构文档的大半篇幅。重开参数：若模块图的读者变成人而不再是门（例如门被撤掉），表格回到文档里更近。
依赖图与缝表仍住 ARCHITECTURE.md（`depmap`、`directions` 围栏块与 §4 的表）：它们短，人读它们的频率高于门读，由 `architecture` 按节切出后解析。
（lexicon 同样用 TOML：退役词是数据不是结构，必须有仓库内数据面。）

**体积读数不渲染成徽章（人的裁定）。** 预算与读数只住 `budgets.toml`，`cargo xtask budget` 打印每一行的读数、最好读数与预算。README 不引体积，于是没有第二个要与读数对齐的呈现，也就不需要一道陈旧判定。README 顶部的徽章（npm、DeepWiki、CodeRabbit）由外部服务出图，每次打开 README 都向这些服务各发一次请求；本包既不生成也不校验它们。

### 代价

零字节：本包不进任何 Run 的 prefix；施工者只在门红时读到三段式报告——边界反馈优于开头说教的施工侧实例。
-/

/-! ## 11 边界枚举

词汇表粗体词一个都解析不出（表结构变了→ `Doc` 错误而非静默通过）；表内同一文件两个条目；状态取值非法；围栏块缺失（→ `Doc` 错误，非零违规）；CRLF 行尾（比对前去掉 `\r`）；非 UTF-8 文件（lossy 读，不 panic）；**一份散文档不可能原样引用 `docnum` 的开标记**（引了它就成为一段受管区段，这是标记即语法的直接后果，故文档描述这个机制时写注释的内容而不写整句注释）。

一次 `gates` 运行里判不动的门可以排在任何位置、可以不止一道：`spec/Report.lean` 对任意前缀与后缀
证明结论仍是 `Broken`，且前后各门判出的违规一条不少。`cfg` 谓词的空 `all()` 恒成立、空 `any()`
恒不成立，`spec/Packaged.lean` 把两者按此判，门的读法与之一致。
-/

/-! ## 12 错误处理

门的结论有三种：判过而干净、判过而带违规、判不动。前两种是 `Ok(Vec<Violation>)`，第三种是
`Err(XtaskError)`；一次运行怎样把各门的结论合成一个退出码，见 D2（`spec/Report.lean`）。判定路径
不写盘（§7），所以任何一种失败之后树都与运行之前逐字节相同。

D1 **门自身的故障是一个类型，退出码 2。** `XtaskError`（thiserror）的变体各说一种判不动：`Io{path}`（读写失败）、`Doc{file,msg}`（数据面不可解析）、`Cmd{cmd,msg}`（git/cargo 调用失败）、`UnknownGate`（`unknown-gate`）、`GatePanicked`（`gate-panicked`）、`StaleBuild`（`stale-build`）、`NoCheckout`（`no-checkout`）、`OutsideCheckout`（`member-outside-checkout`）、`UnknownRole`（`unknown-role`）、`UnknownPackage`（`unknown-package`）；除前三个外，每个的消息都带动作、主体、码与恢复。理由：门坏了必须显性，静默通过是门最坏的失效。被击败的备选：把数据面坏折算成一条违规（退出码 1），那会让「文档坏了」与「代码违规」同形，读者去修错的那一侧。
-/

/-! ## 13 依赖选型

serde 与 serde_json（cargo metadata 解析；工作区已钉）；toml（`lexicon.toml`、`architecture.toml`、`budgets.toml`；xtask 独用，不入产品面）；thiserror（工作区已钉）；kernel（secret 门复用 `kernel::secret::scan`，一个判定一个家）。不引 walkdir/regex/clap：手写遍历十几行；判定用子串与前缀即可；子命令分发一个 match 足矣。

**syn 与 proc-macro2**（`syn` 开 `full`，`proc-macro2` 开 `span-locations`）：**量一个 Rust 函数从哪行到哪行是一个解析问题，不是一个数括号问题**。按行数括号的量法有三处必然的计数错误，每一处都产出一张错的违规名单：`#[cfg(test)]` 被当成文件截断点，其后的生产函数全部隐形；`'{'` 这样的字符字面量被当成开括号；跨行字符串同理。一道量错的门比没有门更坏：它会把人送去拆一个不需要拆的函数。被击败的备选是手写一个状态扫描器（行注释、可嵌套块注释、转义与跨行字符串、raw string 的 `#` 计数、以及 `'a` 生命期与 `'x'` 字符的区分）——八十行代码养第四个计数错误的地方。`syn` 是编译器旁的那个解析器，且已因每一个 derive 宏而在 `Cargo.lock` 里。维护成本：仅工作区工具链，恒不入产品二进制（同 flate2／zip）。

**zip**（`default-features = false, features = ["deflate-flate2"]`，净增两个包）：复用 xtask 已有的 flate2 做压缩后端。被击败的备选是在 justfile 与 CI 里按平台分支调 `Compress-Archive`／`zip`／`tar`：git-bash 携的是 GNU tar，不产 zip，三个平台因此需三段 shell，且一台开发机与 CI 的产物不同源——那正是这里要关掉的那类差异。维护成本：仅工作区工具链，恒不入产品二进制。

分部只用 Lean 工具链自带的库，不 import 任何别的规格：xtask 不在 ARCHITECTURE.md §3 的 `depmap`
块里，`spec` 门因此只准它 import 工具链与自己的分部。
-/

/-! ## 14 硬编码声明

行数与参数预算都**不**硬编码在门里，它们是 `tools/xtask/budgets.toml` 的行（`[function_length]`、`[argument_count]` 与 `[file_length]`，后者带子表 `[file_length.predating]`）——那份登记表持着设计所声明的每一项预算，包括非字节的（百分比、毫秒）。数字的来历写在那一行的注释里；改它是门机械的改动，与被判源码分开提交。

MPL 头四行（通告三行加版权一行）的文字只写在 `header::NOTICE`，不带注释引导符；`header::notice(Leader)` 按 Rust 与 Zig 的 `//` 或 Lean 的 `--` 拼出整行，`header` 门与 `spec` 骨架都从这里取；判据是「整份文件只出现一次」而不是「前四行相等」——只比前四行时，一个由两份文件拼起来的模块可以带着第二份头与半段属于别处的 rustdoc 过关；门机械的路径清单（`tools/xtask/`、`.github/`、`deny.toml`、`Cargo.toml`、`rust-toolchain.toml`、`clippy.toml`、`justfile`）；`boundary` 的 `FUZZ`（`tools/fuzz/` 自成一个工作区，不是任何包的成员或依赖，`members` 看不见它，故以一个具名常量写明它整个是测试代码）；两份数据文件的仓库相对路径 `budget::REGISTER` 与 `lexicon::PATH`，拼路径与报错都用这两个常量；模块状态四值。各随其权威变更而改。

`docnum` 的两个标记文本（两句 HTML 注释，内容分别是 `xtask:begin <fact>` 与 `xtask:end`）硬编码在 `docnum.rs`，因为它们是文档与门之间的语法本身，没有第二个读者；改它们要把树上全部受管区段同集改掉。**事实清单不硬编码在任何文档里**：它是 `docnum::FACTS` 那张数组。带冒号的键（`dep_version:toml`、`budget_reading:frontend_artifact`）把参数写在文档里，故一个生成器服务一族事实，而不是一族事实各占一行。

分部里的数字（`spec/Length.lean` 的 400、`spec/Packaged.lean` 的 `unix`）只出现在说明规则不是空真的
例子里，不是规则的值；预算的值只住 `budgets.toml`。
-/

/-! ## 15 影响面

CI 与 justfile 调用面；ARCHITECTURE.md §3（`depmap`、`directions` 围栏块）、§4（缝表）的格式，与 `architecture.toml` 的字段名，即本包的解析契约。改这些格式就是改本包。Lean 规格的受限形状（ARCHITECTURE.md §11、§8-43）同是解析契约：一份迁到 Lean 的规格改写它的 `inductive` 或一行一臂的 `def` 的写法，就是改 `specalign` 或 `wiring` 读到的东西。
-/

/-! ## 16 测试与约束

单测：`members` 读本仓（kernel 在 `crates/kernel`，desktop 与它的叶子都是成员，xtask 是 Tool，仓库根不是一个包）；包目录落在检出之外以 `member-outside-checkout` 拒读；一棵包名、lib 名、目录三种拼法各不相同的夹具检出（`root::fixture::relocated`：`sprawling-k`，lib `k`，住 `tools/k`）上，depmap 按 lib 名判边、`spec` 把骨架写进 `tools/k`、specalign 读到 `tools/` 下的模块行、proof 读到 `tools/k` 的 harness、`root::judged` 从 `tools/xtask` 找到检出根，五条各一个测试；`architecture.toml` 条目解析（正例／状态非法／同一文件两个条目）；索引文件判定；lexicon 命中与 `lexicon-ok:` 豁免；depmap 块解析；header 比对（CRLF）；隔离区前缀判定（`local/` 命中、`localx/` 不命中）；`guard::wall` 五例（叶子的表少一条 lint、放宽一条 lint、多一条 lint、一个别的成员写了自己的表、叶子不在成员里）加一条自清理断言（记下的差异消失即须划掉）；docnum 区段解析（整行形与行内形各保持自己的形状、陈旧区段的拒词带 `--write`、未知事实不写盘、三种坏标记各报一例）；`crate_table` 在 relocated 夹具上画出整张表（`sprawling-j` 的模块名写 `bin::`，于是读 `[family.bin]`；duty 里的竖线被转义；没有边的写 `nothing`），夹具缺 `[family.k]` 时以点名它的 `Doc` 拒读（§8-40）。不写「本仓自身通过」一类的单测：门在 `just check` 里对本仓跑一遍，同一断言再跑一遍只多花时间，不多判一件事。约束：全门无网络；判定路径无写盘，写盘只在带 `--write` 的命令上发生（§7）；输出顺序确定。

**证明的性质**（`lake build Spec`，经 `just models`；不依赖 `sorry`、`admit`、`axiom`）：

| 分部 | 定理 | 它对着的 Rust 与守着一致的测试 |
|---|---|---|
| `spec/Report.lean` | `a_broken_gate_outranks_a_violation`、`a_broken_gate_drops_no_later_finding`、`green_means_every_gate_judged_and_found_nothing`、`the_outcome_ignores_gate_order`、`every_outcome_is_reachable` | `report::finish_all`、`RunOutcome::of`；`report::tests::a_broken_gate_outranks_a_violation` |
| `spec/Length.lean` | `an_unpinned_file_passes_exactly_inside_the_budget`、`a_pinned_file_never_grows`、`a_kept_pin_is_still_needed`、`kept_files_exist` | `length::judge_file`；`length` 的单测 |
| `spec/Guard/Wall.lean` | `a_key_agrees_exactly_when_the_record_matches_the_difference`、`a_record_with_no_difference_is_spent` | `guard::wall::judge`；`guard::wall::tests` 的五例与自清理断言 |
| `spec/Packaged.lean` | `an_excluded_item_never_builds`、`the_reading_is_not_vacuous` | `packaged::excludes_production`；`packaged::tests` |

模型的证明是参照定义的证明，不是 Rust 实现的证明：两侧逐分支对应，对应由右列的单测守着，门的
`just check` 运行再对本仓判一次。
-/

/-! ## 17 文档关系

新增或删去一道门时，同一枚提交改 `gates::GATES` 与 AGENTS.md「The machine gates」一节的门表；没有门比较这两者（AGENTS.md 在那一节里写明），由评审核对。门机械的路径清单变时，AGENTS.md 同一节的清单同集更新。

文档里的数字不靠同步，靠受管区段：把数字圈进一对 `xtask:begin` ／ `xtask:end` 注释，`cargo xtask docnum --write` 负责它此后的每一次取值（§8-16）。

- ARCHITECTURE.md §3（`depmap` 与 `directions` 块）、§4（缝表）：depmap 的数据面；改格式即改本包（§15）。
- ARCHITECTURE.md §11「Specifications in Lean」：Lean 规格的布局、`lakefile.toml` 的 glob（本规格是
  `tools.xtask.Spec` 与 `tools.xtask.spec.+` 两条）、门读的受限形状；§8-42、§8-43 是它的门一侧。
- `lakefile.toml` 的 `Spec` 库：本规格进 `just models` 的地方。
- `architecture.toml`：modmap、specalign、`crate_table` 的数据面；xtask 自己的文件不登记在那里（D9）。
- `tools/xtask/budgets.toml`、`tools/xtask/lexicon.toml`、`tools/xtask/public-api.txt`：各自一份数据面，
  本规格只说门怎样读它们。
- `docs/glossary.md`：退役词的 replacement 指向它的粗体词（§10 第 5 条）。
- `skills/sdd/SKILL.md`：`spec` 骨架的十七节（§8-41）与本规格的十七节。
- `tools/adversary/Spec.lean`：检验器的规格，`spec` 门按位置认出它（§8-47、D13）。
- `crates/desktop/Spec.lean` D14：叶子那张 lint 表为什么存在（§8-46、D14）。
-/
