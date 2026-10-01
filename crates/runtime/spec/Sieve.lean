-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::sieve

规定 `sieve`、`sieve::key`、`sieve::record`、`sieve::filter`、`sieve::scan`、`sieve::stages`、`sieve::diff`（`crates/runtime/src/` 下同名的文件）。命令输出的确定性压缩器。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-27 runtime::sieve（形状 1 判定＋形状 6 数据面；**本节是压缩器的唯一权威**）


> 读这一节即可实现，不需要再查别处、不需要再做任何参数选择。方法论取自 Hypabolic/Hypa 的 ADR-0002 与其 `Compression/` 实现；四处刻意背离记在 §8-27-7。

#### 8-27-1 它是什么，以及为什么不是 LLM

`sieve` 按**产生结果的命令**决定留下什么。它与 `compaction` 相邻而不重叠：`compaction` 看文本形状（Prose／Code／Diff／Log／Structured／Table／Markup／Unknown），`sieve` 看命令身份（`cargo build` 与 `git status` 的噪声形状完全不同，而两者都是 Log）。

不用模型做压缩，理由是被架构强制的而非偏好：确定性（ARCHITECTURE 的确定性一节）要求同一颗种子重放出逐字节相同的对话，一个会思考的压缩器会让重放不可能。它同时省掉一次调用的钱与延迟。

**顾问不是这条链上的模型。** 它在 `pipeline::package` 之外先跑，判断作为 `PackContext.adviser` 进来，而且答案本身就是账本上的一条记录（`adviser_answered` 或 `adviser_fell_back`）——重放读到的是那条记录，不是再问一次。所以「同一颗种子重放出逐字节相同的对话」仍然成立，而 `sieve` 自己一个模型也不调：它只有七条固定阶段。

#### 8-27-2 位置与法则

管线原法则「offload 恒先于 truncation」扩写为：

```
tee（原文钉进 CAS ＋ 实体化 rest 文件）
  → sieve（命令感知）
    → offload / truncation（既有三臂）
```

**tee 恒在最前**：这是 `offload` 既有的 store-before-cut 不变量上提一层。因为原文一定先落盘，sieve 才敢压得比 Hypa 狠——Hypa 那条「压缩率异常高时拒绝」的护栏存在，是因为它不能保证全文带内可恢复；本城可以。

#### 8-27-3 作用范围

**只作用于 `exec` 结果。** 命令感知的东西对没有命令的工具无可感知：`read` 结果、模型输出、MCP 结果各自走既有的 `compaction`／`redact` 路径，本节不动它们。MCP 结果不过 sieve，但过同一个 `package` 的落盘一步，见 8-27-10。

#### 8-27-4 七条不变量

1. 输出恒不长于输入。
2. 输入非空时输出非空。
3. 任何裁剪之前，原文已钉入 CAS。
4. 每个被筛过的结果都携带 rest 文件路径与 CAS locator。**压缩是强制的；可恢复也是强制的**——没有落 tee 的压缩不许发生。
5. 切口落在字符边界上。
6. **这条路上不用正则表达式**（判「重要」的四类模式全部手写线性扫描，见 §8-27-6）。
7. 账本记的是**模型看到的字节**＋原文 locator。重放复现模型看到的东西，不是命令打印的东西。

#### 8-27-5 阶段顺序与参数（全部已定，实现者不另选）

| # | 阶段 | 参数 | 取值 |
|---|---|---|---|
| 0 | 地板 | `SIEVE_FLOOR` | **2 KiB**。以下原样通过，连 tee 都不做（此时 tee 的一次 CAS 写加一个实体文件，比省下的字节贵） |
| 1 | 去 ANSI | — | 恒开 |
| 2 | 空行折叠 | 连续空行 | **≥3 折为 1** |
| 3 | 模板去重 | 同模板出现次数 | **≥4 折成一行＋计数**；≤3 全留 |
| 4 | 跨调用差分 | 同 `(arm, path, args)` 本 run 内跑过且原文在 CAS | 只出新增／变化行，未变部分**一行**带过 |
| 5 | 过滤表 | 见 §8-27-6 | 命中即用，否则走通用路径 |
| 6 | 长行截断 | 单行长度 | **> 2 KiB 截断并标记** |
| 7 | 截断 | `MAX_TOTAL_LINES` | **240** |
| | | `HEAD` / `TAIL` | **40 / 40** |
| | | 中段保留上限 | **60 条，按优先级排序取前 60** |

每一级只在**不变长**时被接受；**被跳过的级要记进账**（Hypa 是静默跳过的，坏过滤器因而事后不可诊断）。

#### 8-27-6 重要行的优先级与过滤表

优先级序（同级按原始行序，稳定且确定）：

```
error  >  panicked / fatal / failure / failed  >  assertion  >  warning  >  note / help
```

判「重要」的四类模式，手写线性扫描：关键词子串加词边界；`路径:行:列` 形状；`大写字母2-4 ＋ 数字3-5` 的诊断编号；`test result:` / `exit code` 这类结果行。

**原样保留、任何阶段不得触碰**：URL、设备码形状的字符串、`secret:realm/name` 引用、带行列的文件路径、退出码。

过滤表住 `<city>/.sprawling/FILTERS.toml`，楼级可覆盖 `<building>/.sprawling/FILTERS.toml`，走既有三层阶梯且**整值解析而非逐字段合并**（直接沿用 kernel-SPEC §8-22 给 `[sandbox]` 定的口径①，一条规则一个权威）。**信任问题不存在**：过滤表住保留区，没有任何写域够得着它。

形状（谓词只有 prefix / contains / suffix，无正则）：

```toml
[[filter]]
id          = "cargo"
command     = "cargo"
subcommands = ["build", "check", "test", "clippy", "nextest"]
strip_ansi  = true
drop_prefix = ["   Compiling ", "    Checking ", "    Finished ", "   Updating ", "    Blocking "]
keep_contains = ["error", "warning:", "panicked at", "test result:", "-->"]
head = 20
tail = 40
on_empty = "cargo {sub}: clean, exit {code}"
```

**内建编译进去的 reducer 恰三个**：`cargo`（含 rustc 诊断分组）、`git`、`generic`。其余一律走表——ADR-0002 自己把「命令专属 reducer 是持续维护负担」写在负面后果里，Hypa 列了十个，那张清单会长成泥潭。

页脚（**不报 token，不引分词器**——o200k 不是所接模型的分词器；真实 token 由 provider 在账上给）：

```
[sieve: 1,240 → 78 lines, 31.4 KiB → 2.1 KiB, filter=cargo, rest at ./.rest/rest-a91f.dat]
```

#### 8-27-7 四处刻意背离 Hypa

1. **重要行排序而非全留。** Hypa 的 `TruncationStage` 把中段所有命中 `ImportantLineClassifier` 的行全部保留；其 `\b[45]\d{2}\b` 会把 `compiled 437 files` 判成重要行，而 cargo 输出里几乎每个依赖都命中一次 `warning`。一次三百条 warning 的构建因此压了等于没压。本实现取前 60 条。
2. **模板级去重而非相邻去重。** Hypa 的 `DeduplicateStage` 只折叠连续相同的行；构建日志是交错的（`Compiling a` / `Compiling b`），一行都压不掉。归一化数字、哈希与路径后按模板分组，是 cargo 场景下最大的单项收益。
3. **跨调用差分。** Hypa 每次调用独立压缩，因为它的压缩路径没有会话模型。本城有账本与 CAS：开发循环里 `cargo check` 跑十遍，九遍与上一遍逐字节 90% 相同，只出差分能在压缩之上再降一个数量级——而且给模型的是**更好的信息**（「这个错是新出现的」本来要它读两遍才能得出）。
4. **无正则。** Hypa 的分类器整个是正则；本城这条路上不用模式引擎。手写扫描约 40 行，更快且无回溯风险。

不抄的三样：它的十个 compiled reducer 清单（维护跑步机）、`Microsoft.ML.Tokenizers`（依赖加谎言）、SQLite 与 `hypa trust`（账本＋CAS＋保留区规则已经更强）。

#### 8-27-8 验收

每张内建过滤器一组 golden；一条性质「输出 ≤ 输入」；一条性质「输入非空则输出非空」；一个 citysim 场景——同一颗种子、同一张过滤表，重放出逐字节相同的窗口。

#### 8-27-9 接口与文件切分

> §8-27-1…8 定的参数与不变量一个不改；本小节只把它们落成签名，并记下三处那几节没写明、实现时按下面读法取的口径。

```rust
// runtime::sieve — 入口（形状 1）
pub struct CommandKey { arm: String, program: String, args: Vec<String> }   // Ord：BTreeMap 键，跨调用差分按它分组
impl CommandKey {
    pub fn of(arm: &ExecArm) -> CommandKey;   // Program→(path,args)；Shell→按空白切 text，首词为 program；Python→("python",[code])
    pub fn command(&self) -> String;          // program 的文件名去目录、去 .exe、小写：过滤表 `command` 按它匹配
    pub fn subcommand(&self) -> Option<&str>; // 首个不以 `-` 开头的参数：过滤表 `subcommands` 与 `{sub}` 按它取
}
pub struct SieveInput<'a> { pub key: &'a CommandKey, pub exit_code: Option<i64>, pub text: &'a str }
pub enum Sieved {
    Passed { text: String, reason: PassReason, account: Option<SieveAccount> },
                                  // 地板以下／全部阶段被拒：原文一字不动；跑过的阶段随 account 出去
    Cut(SieveRecord),
}
pub enum PassReason { BelowFloor, NothingShrank }
pub struct SieveRecord { pub text: String, pub original: Locator, pub rest_path: String, pub filter: String,
                         pub lines_in: u64, pub lines_out: u64, pub bytes_in: u64, pub bytes_out: u64,
                         pub stages: Vec<StageReport> }
impl SieveRecord { pub fn offloaded(&self) -> ResultOffloaded; }
pub struct ResultOffloaded { pub original: Locator, pub len: u64, pub substitute_len: u64,
                             pub rest_path: String,
                             #[serde(flatten)] pub sieve: Option<SieveAccount> }
impl ResultOffloaded { pub fn payload(&self) -> Result<Payload, AxError>; }   // result_offloaded 的唯一写方
pub struct SieveAccount { pub filter: String, pub lines_in: u64, pub lines_out: u64,
                          pub stages: Vec<StageReport> }
pub struct StageReport { pub stage: Stage, #[serde(flatten)] pub outcome: StageOutcome }
pub enum Stage { StripAnsi, FoldBlank, DedupTemplate, DiffPrevious, Filter, CutLongLine, Truncate }
pub enum StageOutcome { Applied { bytes_before: u64, bytes_after: u64 }, Noop, Rejected { grew_to: u64 }, Unavailable { reason: String } }
pub fn sieve(input: SieveInput<'_>, table: &FilterTable, site: &mut OffloadSite<'_>, history: &mut SieveHistory)
    -> Result<Sieved, AxError>;
// tee 走 offload::tee（pub(crate)；offload() 自身也改经它，store-before-cut 只有一处）；history 无论 Cut／Passed 都记本次原文

- **账目不为任何一条臂而丢。** `Cut` 携 `SieveRecord.stages`；`Passed` 携 `account`——NothingShrank 时七条阶段全在，BelowFloor 时 `None`（没有阶段跑过，`reason` 就是全部账目）。`StageOutcome` 四变体（`Applied`／`Noop`／`Rejected`／`Unavailable`）是每个阶段唯一的答案形状。**顾问不是第八个 stage**：问／答／回落有自己的事件族（`adviser_asked`／`adviser_answered`／`adviser_fell_back`），在这里再记一份就是同一事实两个家；「顾问就是第八个 stage」是 E-2 核验更正前的措辞。
- **pass 的账目去处**：经 sieve 但未被裁的结果若随后走普通 offload 离窗，`ResultOffloaded.sieve` 携这名 account；留在窗口内的 pass 不写账，因为没有任何东西离窗。

// runtime::sieve::filter — 过滤表（形状 6）
pub struct Filter { id, command, subcommands: Vec<String>, strip_ansi: bool,
                    drop_prefix / drop_contains / drop_suffix / keep_prefix / keep_contains / keep_suffix: Vec<String>,
                    group_until_blank: bool,                                    // 命中 keep 的行把其后到空行为止的行一起带上（rustc 诊断分组）
                    head: Option<u64>, tail: Option<u64>, on_empty: Option<String> }   // 全部字段 serde default；只有 id 与 command 必填；未知字段拒
pub struct FilterTable { filters: Vec<Filter> }
impl FilterTable {
    pub fn builtin() -> FilterTable;                            // 恰三张：cargo、git、generic
    pub fn parse(toml_text: &str) -> Result<FilterTable, AxError>;   // `[[filter]]` 数组；E_INVALID_ARGS 拒坏表
    pub fn resolve(city: Option<&str>, building: Option<&str>) -> Result<FilterTable, AxError>;  // 口径①整值覆盖：楼＞城＞内建
    pub fn lookup(&self, key: &CommandKey) -> &Filter;         // 表内命中＞generic；表内顺序即优先序
}

// runtime::sieve::diff — 跨调用差分（形状 1）
pub struct SieveHistory(BTreeMap<CommandKey, Locator>);        // 本 run 内每个键最近一次的原文 locator；调用方持有

// runtime::pipeline — 管线接入
pub struct SieveRequest<'a> { pub key: CommandKey, pub exit_code: Option<i64>, pub table: &'a FilterTable, pub history: &'a mut SieveHistory }
pub struct PackContext<'a> { /* 既有五字段 */ pub sieve: Option<SieveRequest<'a>> }
// package：sieve 为 Some 时 `result` 是命令输出文本而非 JSON；tee 复用 ctx.offload；无 OffloadSite 即无 tee 即不压（不变量 4），记 Unavailable
```

**文件切分**（一文件一模块，全部 ≤ 400 行）：

| 文件 | 形状 | 持有 |
|---|---|---|
| `sieve.rs` | 1 判定 | 阶段定序、逐级「不变长才接受」的判定（`Draft`：行与账同行）、页脚 |
| `sieve/key.rs` | 2 值 | `CommandKey`：arm／program／args，Ord |
| `sieve/record.rs` | 2 值 | `Sieved`／`SieveRecord`／`Stage`／`StageOutcome`／`StageReport`／`SieveAccount`／`ResultOffloaded`——`result_offloaded` 的唯一形状，筛与直接搬运两条路都经它 |
| `sieve/filter.rs` | 6 数据 | `Filter`／`FilterTable`：TOML 形、三张内建、三层整值覆盖、命中规则 |
| `sieve/scan.rs` | 1 判定 | 重要行五级优先级的四类手写扫描；受保护片段（URL、设备码、`secret:` 引用、`路径:行:列`、退出码行）的判定 |
| `sieve/stages.rs` | 1 判定 | 去 ANSI、空行折叠、模板去重、长行截断、head/tail/中段截断 |
| `sieve/diff.rs` | 1 判定 | `SieveHistory` 与跨调用差分 |
| `sieve/tests.rs` | — | 三张内建过滤器的 golden、两条 proptest 性质、阶段账 |

**三处读法**（§8-27 未写明处，按此实现；改口径先改这里）：

1. **受保护片段的「不得触碰」**读作：含受保护片段的行不进模板去重、不被长行截断——这两级会**改写**一行；在第 7 级截断里它算最低一级重要行（排在 note/help 之后），与其他重要行一起按序取前 60。把它读成「恒不丢」会让一份三百条 URL 的清单压不动，与不变量 1 的目的相悖。跨调用差分**不豁免**它：把上一次逐字相同的行计入「未变 N 行」不改写任何一行，那行在 rest 文件里原样在，模型上一次也已读过；豁免它会让每个 rustc 诊断块被 `-->` 行切成折不动的短段，差分在它为之而存在的 cargo 场景上恒为 Noop。空行同理计入。
   第 3 级模板去重另豁免过滤表 keep 命中的行及其分组（否则 `  |` 这样的诊断沟槽行跨块折叠，`10 |     let x0 = 1; [×6 similar lines]` 说的是并不相同的六行）。
2. **过滤表的 `head`/`tail`** 是第 7 级 HEAD/TAIL 的逐表覆盖，不是第二次截断；`keep_*` 命中的行进第 7 级中段候选，优先级与 §8-27-6 的 warning 同级。这样截断只有一处权威。
3. **generic 的 `on_empty`** 缺省为 `"(no output kept), exit {code}"`；`{code}` 在无退出码（sandbox trap／fuel 耗尽）时写 `none`。这是不变量 2 在通用路径上的执行体。

**三个 reducer 与表的关系**：cargo 与 git 是两个以代码构造的 `Filter` 值，rustc 诊断分组是 cargo 那张的 `keep_contains` 命中行向后扩到空行为止（同一诊断块整体进中段候选）；generic 是空谓词的 `Filter`。表内条目与内建同形，故楼级表可以整张换掉 cargo 的裁法而不动代码。

**citysim 侧**：`Scenario` 增 `sieve: Option<SieveWorld>`（CAS＋environment＋过滤表＋本 run 的 history）；有它时执行器把名为 `exec` 的工具结果经 `package_exec` 走带 `SieveRequest` 的 `package`，模型看到 `{content, exit_code, sieve:[载荷]}`。`tools/citysim/tests/sieve.rs`：同一目录同一表跑两遍账本逐字节相同；第二次同命令只出新错误与 `[unchanged: N lines…]`。
-/
