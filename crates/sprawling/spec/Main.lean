-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::main

规定 `crates/sprawling/src/main.rs` 与 `crates/sprawling/src/main/` 下的命令行：各动词怎样读城、怎样呈现给人与 agent（`bin::main`）；动词表的解析在 `spec/Main/Grammar.lean`，退出码在 `spec/Main/Exit.lean`。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-2b CLI 补齐

- **`resume <city>`**＝启动扫描：一遍流式验链（`runtime::replay::fold_ledger_dir`），验过的每条记录交给 `runtime::replay::DanglingCalls` → 逐个补记 `E_TOOL_OUTCOME_UNKNOWN` 的 `tool_result`（幂等：已闭的账不重补）→ 报待批数。`runtime::replay` 补写面自此有生产消费者。续跑已批准的活仍在 serve 的 `answer_approval` 路径上，两者不重叠。
- **`fork <city> <run> <seq> [addr]`**＝世系记录形：验链 → `runtime::fork::prefix` 验界 → 节点归属验（seq 处的事件必属于母 Run，否则拒）→ `run_forked` 落账携新 RunId。**不自动发车**：驱动新 Run 是人的下一步 Dispatch；逐字节母前缀入窗属并发期的 must-read 网，不在本形。`Command::Fork` 同路。
- **`adopt <city> <addr>`**＝收编已存在目录为楼（语义住 `crates/city/Spec.lean` §8-3）。
- **`serve` 增 `--web-dir <dir>`**：开发回路逐请求读盘；发布形恒走嵌入表（`wire::ClientAssets`，语义住 `crates/wire/Spec.lean` §8-2）。
- **默认地址住 `kernel::consts_policy::DEFAULT_AT`**（`"127.0.0.1:8787"`）。`up`／`serve`／首屏／`call` 与 `enrol` 的 `--at` 缺省，以及「不是 socket 地址」那条恢复语里的示例，全部读这一个值：一个人被告知的地址与真正绑上的地址不能是两处写法。安装脚本与 README 引用同一个值，改它即改这一处。
-/

/-!
## 8-4d 桌面是这个二进制自带的工具（`accounting::worker::workbench::desktop`、`bin::main::desktop`）

一栋楼的 `RULES.toml` 写 `desktop = true`，它的 run 就拿到这台电脑的桌面那六件工具（`crates/desktop/Spec.lean` §8-7）。人不在 `CONFIG.toml` 里手写 `[[mcp]]`，也不另装程序。

```rust
// bin::assembly —— 起桌面 server 的那个程序；装配点收下它，而不是自己去问
pub type DesktopProgram = fn() -> std::io::Result<std::path::PathBuf>;   // 生产交 std::env::current_exe
impl RunWorker { pub fn with_desktop_program(self, program: DesktopProgram) -> RunWorker; }

// accounting::worker::workbench::desktop（形状 1 判定）
pub(in crate::assembly) const DESKTOP_LABEL: &str = "desktop";
impl Laying {
    /// 这次 run 要连的 server：楼的配置写下的那些，再加上规则要桌面时这个二进制自己那一台。
    pub(in crate::assembly) fn servers(&self, site: &Site) -> Vec<kernel::McpServer>;
}

// bin::main::desktop（形状 4 适配器）
// `sprawling desktop [scope]`：在 stdin／stdout 上当桌面 MCP server；scope 是 DESKTOP.toml 的路径，缺席即全拒
pub(super) fn verb(scope: Option<&str>) -> ExitCode;
```

- **同一个二进制，另一个进程**：规则要桌面时，`servers` 给这栋楼添一条 stdio 声明：`command` 是正在运行的这个可执行文件，`args` 是 `["desktop", <这栋楼的 DESKTOP.toml>]`（`city::desktop_scope_path`，住城根下这栋楼的保留子树，评审楼的 worktree 里没有它）。之后它与任何一条 `[[mcp]]` 走同一条路：经 `accounting::Connectors` 连上、握手、list，常驻连接表管它的寿命（§8-4）。子进程这条边界是故意留的：UI Automation 的 COM 状态、`SendInput` 与 `crates/desktop/` 里那些带 `SAFETY:` 的 `unsafe` 都跑在子进程里，城那个唯一写者的进程里一行也不跑；COM 出错或子进程 abort，结束的是子进程，城只在那一次调用上拿到 `E_TIMEOUT` 或 `E_TOOL_UNAVAILABLE`；两者都发生在请求交出之后，所以都带 `Retry::Unknown`（`crates/agent_protocols/Spec.lean` §8-17）。桌面自己的拒绝以 `isError` 结果到达，城把它读成一次失败，拒词全文进 subject（`crates/agent_protocols/Spec.lean` §8-1c）。MCP 这条路上已有的规矩一条不改：工具表随 run 冻结，`kernel::gate::undoable` 按远端名前缀 `desktop.` 升给人，期限到即杀子进程。
- **程序路径是收下的，不是问出来的**：`RunWorker` 持一个 `DesktopProgram`，生产交 `std::env::current_exe`，测试交 `CARGO_BIN_EXE_sprawling`。理由与 `Browsers`（§8-45-2）相同：它在主机上起一个程序。问不出路径或路径不是 UTF-8 时，这栋楼这一次没有桌面工具，诊断里留一条 `Refuse`，派活照常。这与起不来的 `[[mcp]]` server 是同一个答法。
- **楼自己写了 `label = "desktop"` 的 `[[mcp]]`，就用它那一条**：两台 server 用一个标签，同一个工具名就指向两个进程（city 对同层重名的拒绝是同一个理由）。人明写的那一条优先，自带的这台不起，诊断里留一条 `Decide`（诊断的级别里没有「提醒」一级，`Decide` 说的正是一个判定为什么取了这个值）：要用自带的，删掉那一行。
- **装配层不按平台分支**：非 Windows 的机器上这台 server 照样起，每次调用答 `E_TOOL_UNAVAILABLE` 并报出平台名（`crates/desktop/Spec.lean` §10 设计四）。在这里再判一次平台，这条规则就有了第二个家。
- **confidential 楼够不着它**：`city::policy` 解析时就拒绝 `confidential` 与 `desktop` 同真，`mcp_tools` 对 confidential 楼也不起任何进程。这里不判第三次。
- **体积**：把 `crates/desktop/` 链进来使 release 二进制变大多少，读数只记在 `tools/xtask/budgets.toml` 的 `[release_binary]`。增量来自 `crates/desktop/` 自己的代码、`image` 的编码器与 `windows` 绑定；std、serde_json、toml、png 两边共用，只算一份。
- **不内置任何模型（定规）**：桌面给模型的文字反馈先取 accessibility tree；OCR 与 ASR 都经人接入的端点，二进制里不带任何模型的权重。`crates/desktop/Spec.lean` §15.2 记着这条线后面还欠的东西。
- **被否的两条路**：①照旧另发一个 `sprawling-desktop` 可执行文件，由人放上搜索路径再手写 `[[mcp]]`：一件功能成了两个制品，版本要对齐，人还得知道那一行怎么写；②把单独编出的桌面可执行文件的字节嵌进本二进制，运行时写到盘上再起：运行时往盘上写可执行文件，std 也多带一份。**重开参数**：`crates/desktop/` 使 release 二进制增大超过 1 MiB（`[release_binary]` 的 slack），就回到第一条路重新比较。

**验收**：`crates/sprawling/tests/desktop.rs` 的 `a_building_given_the_desktop_is_offered_its_six_tools_from_this_binary`。一栋楼的 `RULES.toml` 写 `desktop = true`，没有任何 `[[mcp]]`，城起真的 `sprawling desktop` 子进程，模型收到的工具表里有 `desktop_desktop_windows` 等六件。
-/

/-!
## 8-136 `sprawling whose --trace`：一个提交倒推到产生它的调用与居民（`bin::main::whose`；`crates/accounting/Spec.lean` §8-16）

**形状。** `main/whose.rs` 仍是 adapter：读命令行，不带 `--trace` 时问 `accounting::views::ask`，带 `--trace` 时问 `accounting::trace::trace`，把答写成几行给人读。写 stdout 失败时与 `view` 一样：读者提前停下（`| head`）退出 0，别的写失败退出 1。

**输出。** §8-167 的四行（`run`、`actor`、`model`、`ledger`）与各 `replaced` 行之后，总有一行 `previous`：`previous <oid> at seq <n>`，这个 run 的第一个提交写 `previous none, the run's first commit`。带 `--trace` 再写：

- `span    after seq <n>, before seq <m>`；没有 `previous` 时是 `span    from the run's first line, before seq <m>`。
- 每条调用一行 `call    seq <n>  <called 的 iso>  <工具>  <effect>  <结局>  <subject>`，结果有输出时下一行是 `        > <输出的第一行>`。`effect` 与结局按它们的 serde 拼法写（`read`、`{"write":{"domain":"lab"}}`；`answered`、`failed`、`waiting`），没有登记的工具写 `unregistered`；没有 subject 时取参数的第一行，两者都没有写 `-`。区间里一条调用都没有时写 `call    none`。版本早于逐行时刻的账本里，`called` 是回合的时刻。
- 同楼的每个别的 run 一行：`nearby  <run> at <actor>: <k> call(s)`。最后一行 `note    the calls are candidates; a nearby run may have written in the same span`，只在有 `nearby` 行时写。

**退出码。** 同 §8-41：0 答上了；1 这座城没写过这个提交；2 命令行读不了。

**测试。** `whose_trace_names_the_run_its_calls_and_who_else_called_in_the_building`（`bin::main::tests`）在一座城的账本上逐字节比输出；区间与配对的规则由 `accounting::trace::tests` 钉住。
-/

/-!
## 8-104 `sprawling check <city>`：每份 TOML 一行错，行列可点（`main::check`；`crates/city/Spec.lean` §8-29）

只读动词。它把 `city::check` 的每条 Finding 印成一行，写到 stderr：有位置时 `路径:行:列: 码: 消息`，没有位置时 `路径: 码: 消息`；路径是城根下的相对路径，用 `/` 分隔，编辑器与终端都能把它当成跳转目标。码是 `AxCode::as_str`，消息是错误的 subject。

- 退出码：全部读过 → 0；有任一条 Finding → 1；命令行读不懂 → 2（与 §8-89 同一规矩）；城本身读不了（列楼失败、I/O 失败）→ 1，并按一条普通拒绝印出。
- 全部读过时 stdout 印一行 `ok: <n> file(s)`，n 是读到的文件数。
-/

/-!
## 8-105 `sprawling view`：给 agent 的一面（`bin::main::view`、`accounting::lineage`）

**形状。** `main/view.rs` 是 adapter：读命令行、开账本索引、把选中的行写到 stdout。`lineage.rs`（库里，`accounting::lineage`）是 projection（ARCHITECTURE §9 形状 7）：把账本折成每个 run 一条 `RunLine`，查看器的 `tree` 透镜（S5.8I）与 WebUI 以后的 run 树读的都是它。`view` 只读，`Effect::ReadsOnly`。

```rust
// bin::main::view
pub(super) fn verb(read: &Arguments) -> ExitCode;
pub(super) struct Selection { tail: Option<usize>, from: Option<Seq>, run: Option<RunId>, kind: Option<EventKind>, who: Option<String>, grep: Option<String>, span: UtcSpan }  // span 见 §8-137
pub(super) enum Audience { Agent, Person }  // 谁读 stdout：管道或文件后面的 agent，终端前的人
pub(super) fn write_records(dir: &Path, chosen: &Selection, audience: Audience, out: &mut impl Write) -> Result<(), ViewError>;
enum HashWidth { Whole, Glance }             // 64 位，或前 GLANCE_DIGITS 位
const GLANCE_DIGITS: usize;                  // 12
fn chain_label(line: &[u8], width: HashWidth) -> String; // 十六进制位，后接两个空格
// accounting::lineage
pub struct RunLine { run, addr, session, parent, forked_at, predecessor, first_seq, last_seq, state, unanswered }
pub struct Lineage;                       // fold：apply(&EventRecord) -> Result<(), AxError>
impl Lineage { pub fn lines(&self) -> impl Iterator<Item = RunLine>; }
pub fn lineage_of(ledger_dir: &Path) -> Result<Lineage, AxError>;
```

**`records` 透镜（非终端时的输出）。** 输出账本原行，逐字节相同，每行一个 `\n`，按 seq 升序。条件同时成立才选中：`--from <seq>`（含）、`--run <id>`（走 `LedgerIndex::run_seqs_before`，不读别的 run 的行）、`--kind <k>`（信封的 `kind`）、`--who <addr前缀>`（信封 `addr` 以它开头；没有 `addr` 的行不中）、`--grep <子串>`（原行按字节含这个子串，不是正则，glossary 的搜索规则）。`--tail N` 最后作用：只留选中的最后 N 行，从尾部倒着找，找够就停。`--since <utc>`/`--until <utc>` 按信封的 `t` 选（§8-137）。信封只借用解析 `run`、`kind`、`addr`、`t` 四个字段。

**终端前的 `records` 透镜。** 谁读 stdout 只在 `verb` 里判一次（`Audience::of_stdout`）：stdout 是终端就是 `Person`，否则是 `Agent`。`Person` 而命令行一个过滤参数都没带时进 §8-117 的交互界面；带了过滤参数，`write_records` 按上一段的规则选行，每个选中的行写成：这一行的链哈希、两个空格、原行、一个 `\n`。链哈希是 `kernel::ledger::chain_hash` 对这一行在盘上的字节（不含行尾 `\n`）算出的 BLAKE3，64 位小写十六进制；链完好时它就是下一行的 `prev`。`view` 不核对链，打出来的只是这行字节的哈希，链断没断由 `sprawling replay` 核对。哈希前缀与两个空格只由 `chain_label` 写出，交互界面的列表调同一个函数，只取前 `GLANCE_DIGITS` 位（§8-117）。`Agent` 时的输出与上一段逐字节相同。测试不经过终端，直接把 `Audience` 交给 `write_records`。

**`--runs`（`tree` 透镜给 agent 的画法）。** 每个 run 一行 JSON：`run`、`addr`、`session`、`parent`、`forked_at`、`predecessor`、`first_seq`、`last_seq`、`state`、`unanswered`，按 `first_seq` 升序。`parent` 是 `run_forked.from`，没有分叉记录时是 `run_started.parent`；`forked_at` 是 `run_forked.at_seq`；`predecessor` 是 `run_started.predecessor`；`session` 是这个 run 开始前、同一地址上最近一条 `session_opened` 的 seq（这段 stretch 的名字），没有就是 `null`。`state` 取自 `storage::HotView` 的 `RunPhase`（`active`、`frozen`），与 Views 的 run 列表同一份折叠，不另算。`unanswered` 是这个 run 提出、还没有 `approval_resolved` 答复的 `approval_requested` 条数：请求按它的 `id` 记在提出它的 run 名下，答复按同一个 `id` 销掉，不管答复落在哪个 run 上；两种记录的 payload 都经 kernel 的类型读（`ApprovalItem`、`ApprovalResolved`），读不了就拒，与 `views::governance` 同一条规则（§8-74）。父指针都在行里，agent 不需要第二次查询就能拼出树。

**失败。** `--kind` 不是 `EventKind` 的 snake_case 名：stderr 一行 `sprawling: view: no event kind '<k>'. Did you mean '<近似名>'?`，退出 2（近似名用 `grammar::nearest` 那一条规则）。`--run` 不是 RunId、`--from`/`--tail` 不是数：同形，退出 2。账本目录读不了：`AxError` 的正文与 recovery，退出 1。

**决定。**

1. 两个主人按 TTY 分开，不违背 §8-11「拒长表与图」。stdout 不是终端时 `view` 只输出账本原行，与 `call` 同形；表格、树与颜色只在人坐在终端前时才画（S5.8I），控制台仍然只写 JSONL。被否掉的是在 `call` 上加过滤参数：`call` 走 wire，要城在服务；`view` 读盘，城不在服务也能答。
2. 交互界面不给每种事件写说明：一行只画信封字段加压缩后的 `data`，详情画通用 JSON 树，事件种类再多也不加一行。
3. 树为主（D-15）：城 › 楼 › 房间 › 会话 › run › 回合 › 调用，分叉挂在父 run 的分叉点下，每个节点只有一个父；委派、敲门、handback 是详情里的链接，不是树的边。被否掉的：列表加详情为主（人要在交错的行里自己拼出一件活）；fx 式 JSON 树为主（就地展开推走下面的行，也看不出分叉）。
4. 行选择与 run 折叠都消费 `storage::LedgerIndex` 这一个索引，不自己数段文件（`crates/storage/Spec.lean` §7：`storage` 不对外暴露段）。
-/

/-!
## 8-126 `sprawling playback export` 与 `sprawling playback check`（`bin::main::playback`；`crates/accounting/Spec.lean` §8-12）

**形状。** `main/playback.rs` 是 adapter：读命令行，把人的入口交给 `accounting::playback` 的共享投影，把字节写到 stdout 或一个新文件，把复核结果写成一行 JSON。投影、选择、读界、规范字节与复核都在 `crates/accounting/Spec.lean` §8-12，本节不重述。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--include-confidential] [--out <file>]
sprawling playback check <bundle> [--bundle <file>] [--city <city>] [--include-confidential]
```

```rust
// bin::main::playback
pub(super) fn export(read: &Arguments) -> ExitCode;
pub(super) fn check(read: &Arguments) -> ExitCode;
```

- **两个词是一个动词。** 命令表的一行可以叫 `playback export`：`grammar::parse` 先看头两个词能否拼成某一行的名字，再看头一个词；只敲 `playback` 或 `playback <错词>` 是 `UnknownVerb`，近似名列出 `playback export`、`playback check`。帮助、总览与解析读的仍是同一张表（§8-89）。
- **选择。** `--from`/`--through` 是含端点的 seq，与 `view --from` 同一种读法（`view::seq_flag`）；`--run` 与 `view --run` 同一种读法（`view::run_flag`）；`--building` 经 `Address::parse`，按 `Address::is_within` 选楼，不是 `view --who` 的字符串前缀。三者交给 `Selection::new`，矛盾的区间由它拒绝。
- **读者是人。** 入口交 `Reader::Person(Confidential::Withheld)`；带 `--include-confidential` 交 `Confidential::Included`，并在 stderr 写一行 `sprawling: playback: this bundle includes confidential buildings`，bundle 的 `source.reader` 写成 `{"person":"included"}`。`check --city` 用同一个标志决定复核时的读者，不读 bundle 自述。
- **输出。** 不给 `--out` 时，bundle 完整、尺寸检查通过之后才开始写 stdout；写到一半管道关闭是失败（退出 1，stderr 说明 bundle 不完整），不沿用 `view | head` 的成功语义。给 `--out` 时经 `accounting::playback::land(Place::Chosen)` 写（`crates/accounting/Spec.lean` §8-13）：同一目录先写一个 `<file>.partial-<pid>`，写完再以硬链接落到目标名、删掉暂存文件；目标已存在、或被 git 跟踪，则拒绝，不覆盖；目标的父目录经 `std::fs::canonicalize` 解开链接之后，路径里任一段是受保护的元数据（`kernel::PROTECTED_METADATA`：`.sprawling`、`.git`，不分大小写）则拒绝，所以账本与 git 元数据写不进去。任何失败都删掉暂存文件，不留可被误认的最终文件。
- **复核。** `check` 的文件、标志、五项与它的退出码见 §8-132。
- **退出码**（§8-103 的表）：0 导出完成；1 拒绝（账本坏、bundle 超限、目标已存在、被跟踪或在受保护的元数据里、stdout 中途关闭）；2 命令行读不懂。

**本节测试**：`accounting::playback::tests::landing`：`--out` 的目标整份落下、不留暂存文件；已存在的目标被拒且原文件不变；指进 `.sprawling` 或 `.GIT` 被拒且没有留下文件；被 git 跟踪而已从盘上删掉的文件名被拒。`main::playback::tests`：矛盾的区间与读不了的楼、读不了的 seq 分成两种拒绝。`grammar::tests::a_verb_of_two_words_is_read_from_two_words`：`playback export` 从两个词读成一行，`help playback check` 是那一行的帮助，`playback` 单独与 `playback <错词>` 是 `UnknownVerb`、近似名恰是那两行。导出的字节、读界与复核由 `accounting::playback::tests` 判定（`crates/accounting/Spec.lean` §8-12）。
-/

/-!
## 8-143 playback 的时间选择：`--since`、`--until`、`--day` 与城工具的同名参数（`bin::main::playback`、`accounting::worker::workbench::tools::playback`；`crates/accounting/Spec.lean` §8-17）

**形状。** 两扇门各读三个原文，交给 `accounting::playback::Window::span`，把得到的 `UtcSpan` 经 `Selection::during` 加进选择。解析、交集与拒绝都在 `crates/accounting/Spec.lean` §8-17；本节只定两扇门的拼法。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--since <utc>] [--until <utc>] [--day <yyyy-mm-dd>] [--include-confidential] [--page <template>] [--out <file>]
```

```json
{"action": "export", "name": "day-1", "day": "2026-05-14", "since": "2026-05-14T09:00:00Z"}
```

- **人的门。** `--since <utc>` 与 `--until <utc>` 的写法与 `view --since、--until`（§8-137）相同：UTC、到秒、以 `Z` 结尾；`--day <yyyy-mm-dd>` 是那一个 UTC 日。区间是 `[since, until)`，三者同给时取交集。读不了的值、交出空区间的组合，stderr 写那条 `AxError` 的人读形式，退出 2，什么也不导出；合法而什么都没选中的区间照常退出 0，写出带范围信息的空 bundle。
- **居民的门。** `export` 多三个可缺的字符串参数 `since`、`until`、`day`，同一种写法、同一个函数读；读不了或交集为空时工具以 `E_INVALID_ARGS` 拒绝，不写文件。
- **不改 `view`。** `view` 的时间过滤照 §8-137；两处共用 `runtime::clock::{parse_iso, UtcSpan}`，所以是同一个时间语义。`view` 没有 `--day`：它是看账本行的透镜，按天的回看是 playback 的事。

**本节测试**：`main::playback::tests`：`--day` 读不了与 `--day` 和 `--since` 交出空区间，各以退出 2 拒绝。`accounting::worker::workbench::tools::playback::tests`：居民的 `day` 进到导出的 `source.selection`，写成那一天的两端。区间本身的选择由 `accounting::playback::tests::span` 判定（`crates/accounting/Spec.lean` §8-17）。
-/

/-!
## 8-132 `playback check` 的五项、`export --page`，与居民的城工具 `playback`（`bin::main::playback`、`accounting::worker::workbench::tools::playback`；`crates/accounting/Spec.lean` §8-13）

**形状。** 两个适配器。`main/playback.rs` 给人用，城工具 `playback` 给居民用；页面嵌入、五项检查、落盘都是 `accounting::playback` 的（`crates/accounting/Spec.lean` §8-13），这里只定两扇门各自读什么、写到哪、怎样报。

```text
sprawling playback export <city> [--from <seq>] [--through <seq>] [--run <run>] [--building <addr>] [--since <utc>] [--until <utc>] [--day <yyyy-mm-dd>] [--include-confidential] [--page <template>] [--out <file>]
sprawling playback check <file> [--bundle <file>] [--city <city>] [--include-confidential] [--observed <file>]
```

**人的门。**

- **`export --page <template>`**：读模板（超过 `PAGE_MAX_BYTES` 先拒绝），经 `accounting::playback::embed` 把这次导出的 bundle 放进去，写出的是页面而不是 bundle；去处照 §8-126，stdout 或 `--out`。页面过不了结构或静态离线一项时退出 1，什么也不写。
- **`check <file>`**：文件可以是 bundle，也可以是页面（`crates/accounting/Spec.lean` §8-13 按首字节分）。`--bundle` 与 `--city` 可以一起给，各自成一项；`--observed <file>` 是 skill 写下的浏览器观察记录。stdout 写一行 JSON：`file` 是被查文件的 BLAKE3（十六进制，观察记录的 `page` 要写它）；`digest`（bundle 的 BLAKE3）与 `events`（十进制字符串）在读出了 bundle 时出现；`structure`、`bundle`、`source`、`offline`、`browser` 五项各是 `{"status": "passed" | "failed" | "unchecked"}`，`failed` 带 `found`，`unchecked` 带 `why`，`browser` 通过时带 `covered`（观察走过的路径）。
- **退出码**（§8-103 的表）：0 是 `Report::holds`——没有一项失败，也没有一项要了而做不了；1 是其余，包括 `--city` 复核不了、观察记录说的是另一份字节；2 是命令行读不懂。没要的项写 `unchecked` 但不影响退出码。

**居民的门：城工具 `playback`。**

```json
{"action": "export", "name": "day-1", "from": 12, "through": 340, "run": "…", "building": "newsroom", "page": "<!doctype html>…"}
{"action": "check", "file": "day-1.html"}
```

- **读者由上下文绑定。** 工具在铺工作台时拿到这个 run 所在的楼，导出与复核都用 `Reader::Resident(这栋楼)`。参数里没有读者：`reader`、`include_confidential` 之类不认识的字段以 `E_INVALID_ARGS` 拒绝，所以居民扩大不了读者，人的 `--include-confidential` 也不会下放给居民。按现行读界，居民可以回看别的非机密楼。
- **`export`**：`name` 是 1 到 64 个 ASCII 字母、数字、`-`、`_`，首字符是字母或数字；`from`、`through` 是 seq，`run` 是 run id，`building` 是楼的地址，与人的门的选择同义（§8-126）；`since`、`until`、`day` 是时间条件（§8-143）。给 `page` 时经 `embed` 写 `<name>.html`，否则写 `<name>.json`。落点是 `CityLayout::playback_exports()` 下以本楼地址命名的目录，经 `accounting::playback::land(Place::Exports)` 写：不覆盖、不跟随链接、在 git 仓库里时必须被忽略、失败不留半成品。结果是 `{"file", "digest", "events"}`，页面另带 `structure`、`offline` 两项；bundle 与页面的字节不进结果，因为工具结果会写进账本。
- **`check`**：`file` 是本楼导出目录里的一个 `<name>.json` 或 `<name>.html`，别的名字与别处的文件都拒绝。它用同一个读者对城复核，结果是 `Report::line`：五项，`bundle` 与 `browser` 为 `unchecked`（居民的门不收另一份 bundle 与观察记录）。
- **效应与写门。** 登记 `Effect::Write { domain: 房间 }`，与 `signal`、`pr` 等写桌子的工具同样走写门；`writes` 回答 `Writes::Nothing`，因为导出件落在工作树之外，checkpoint 没有东西可收。工具的效应按登记而不是按调用，所以 `check` 也不会被推测执行提前跑（只有 `Effect::Read` 会，`crates/runtime/Spec.lean` 的 speculation）。
- **寿命。** 导出件跟着城：它在城根的保留子树下，不在任何 worktree 里，清扫 worktree、run 冻结或重开都不碰它；没有独立 worktree 的 run 也写在同一处。两次导出用了同一个名字，后一次被拒绝。崩溃留下的暂存文件见 `crates/accounting/Spec.lean` §3。
- **登记。** 每一栋楼的工作台都有它；它不在常驻核心，所以模型在休眠索引里见到它的一行，经 `describe` 取说明、经 `call` 调用（`crates/runtime/Spec.lean` §8-60），工具表在 session 里不变。

**本节测试**：`main::playback::tests`：五项写成一行 JSON，没要的项写 `unchecked` 而退出 0，失败或要了而做不了的项退出 1。`accounting::playback::tests::page`：`embed` 写出的页面逐字节带着 bundle，行里的 `</script>` 关不掉数据块，结构与静态离线两项通过；重复的数据块、重复的 `id`、指不到的链接与 `data-seq`、数据块里重复的 JSON 键在结构一项失败；外部资源（`img`、转义过的 CSS `url()`、`@import`、SVG 的 `image`、`srcset`）、`base`、`form`、外链、`meta refresh` 各是一条静态离线的发现；CSP 缺了、在别的元素之后、放进一个主机、缺 `form-action` 各是一条发现；观察记录说的是另一份字节为 `unchecked`，记下一次请求为失败。`accounting::worker::workbench::tools::playback::tests`：带 `include_confidential` 或 `reader` 的调用被拒；机密楼的行到不了居民的导出，导出用同一个读者复核通过；同名的第二次导出被拒且原文件不变，过不了静态离线的页面什么也不留；没有 worktree 的 run 把页面写进城里，`check` 只认本楼导出目录里的名字。已存在、被跟踪、不在导出位置、会进历史的目标由 `accounting::playback::tests::landing` 判定。两名居民（记者、编辑）生成、核对并留存报告由 `tests/acceptance/playback.rs` 判定。
-/

/-!
## 8-129 测量工具箱：一个动词 `gauge`，一处读机器，一个单调钟（`bin::main::gauge`、`bin::main::gauge::lines`、`bin::monitor::tree`、`bin::monitor::spread`、`bin::audience`，形状：adapter / projection / value）

开发这份代码用的测量手段——按微秒计的时长、进程与整机的资源读数、样本的分位数——随产品发出：一个动词 `sprawling gauge`，一份随发行包发出的 skill `skills/gauge/`。读它的是 agent：在这座城上做性能工作的，和在别的项目里要量一条命令的。本节定下五件事：每种读数住在哪里；每种读数只在哪一处采样，谁只读；采样本身花多少、由哪个确定性计数守住；动词的参数与输出；skill 教的流程。§8-94 至 §8-98 讲的监视器照旧，本节只写它们之外的部分和它们要改的地方。

### 8-129-1 三种读数，三个家

| 读数 | 例子 | 住在哪里 | 谁读它 |
|---|---|---|---|
| 城里一件事发生的时刻 | `model_called`、`model_returned`、`tool_called`、`tool_result` 的 `t`（整数毫秒） | Ledger；含义只经 `EventRecord::moment` 读（`crates/kernel/Spec.lean` §8-4「信封 `t` 记的是什么」） | `view` |
| 这台主机为城的一段内部工作花了多久，以及那段工作的确定性计数 | 开城各段（§8-121）、整链审计一遍（§8-90）、派活准备（`[prepare_dispatch]`） | 诊断行，每种一个渲染处，按下限写出（`docs/logging.md` §2、§3） | 服务所在的终端、记录页的 log 透镜、`bench_startup` 留下的日志 |
| 机器的资源被用了多少 | CPU、private、工作集、读写字节、卷剩余空间、进程树 | 城内：监视器的 300 点历史（§8-94），有人看才采样；城外：`gauge` 的 stdout | `gauge`、WebUI 监视页 |

第三种读数不常驻盘上。要一段后台记录，就让一个看的人把它写出去：`sprawling gauge --at <地址> > readings.jsonl` 在后台跑多久，城就记多久；它是一个看整页的人，停下它，采样就停（§8-94 决定 1 不变）。

### 8-129-2 一个权威：谁采样，谁只读

| 事实 | 唯一的采样处 | 只读的一方 | 并进来的第二份 |
|---|---|---|---|
| 进程、整机、卷、进程树的计数 | `bin::monitor`：`counters`、`counters::own_process`、`memory`、`volume`、`tree` | 采样线程（§8-96）、计划推进的内存闸（§8-46-3）、入口按卷卸载（§8-116）、`gauge` | 无；`xtask mem` 留在城外，见决定 4 |
| 时长 | `bin::serving::standing::monotonic_now`，以 `fn() -> Instant` 交给用它的模块；记账一侧经 `accounting::worker::Hands.monotonic` 收下 | `OpeningCost`（§8-121）、整链审计（§8-90）、视图切快照的节奏（§8-99）、采样线程量自己一拍的读取用时、`gauge` 的 `wall_us` 与 `read_cost_us`、`stage_dispatch` 与 `Laying::mcp_tools` 的用时（`[prepare_dispatch]`）、`credentials::probing` 的 `elapsed_ms`（§8-62） | 无；后三处的单位与线上字段都还是毫秒 |
| 时刻 | 驱动的 `RunHooks::now`，只在串行阶段采样 | Ledger 的 `t` | 无 |
| 计数 | 由拥有它的模块自己数：`Health` 由 relay 数（§8-98），视图积压由视图线程数（§8-123），开城时核对的行数、读过与哈希过的字节由开城路径自己数（§8-122） | 采样线程读 `Health` 与积压；开城的计数随 §8-121 那一行写出 | 无 |
| 分位数 | `bin::monitor::spread::Spread` | `gauge`、citysim 的 `bench`（场景经 `Reading::of`，登记册四行经 `row_line`，两者都经 `spread_of`）与 `bench_startup`（`Samples::of`）、crate 内的仪表（§8-84、§8-99、`instrument_monitor_beat`） | 无；两处 citysim 读法都经 `Spread`，取最近秩（决定 8） |
| stdout 那头是谁 | `bin::audience::Audience::of_stdout` | `gauge`、`wire_client::watching`（§8-97）、`view`（§8-105） | 无；`main::view` 留着自己的同名枚举（它还带写一行的方法），它的 `of_stdout` 只把库里的判定换成本地的臂 |

**单位。** 时长在测量点取整数微秒，进程内不到 1 µs 的读数（一帧解码、一帧折叠）取整数纳秒；读数行、`gauge` 的 JSON 行与 `budgets.toml` 的字段都带这个整数，键名以 `_us`（或 `_ns`）结尾。给人看的写法只在显示处换算：不到 1 µs 写 `ns`，不到 10 ms 写整数 `µs`，10 ms 及以上写一位小数的 `ms`（截断）。整数微秒到文字的换算 Rust 一侧只有 `monitor::top::Unit::reading` 一处，`Duration` 到整数微秒只有 `monitor::spread::micros` 一处；TypeScript 一侧只有 `client/src/core/monitor.ts` 的 `reading`。见决定 13。

时长只取单调钟，理由与 8-121「时间从哪来」相同：墙钟被人调过，一段就可能是负数；城钟又只到毫秒，而人的定规要内部热路径按微秒读。计数不是时间，所以可以在 accounting 与 storage 里累计，不必经过 `bin::assembly` 的采样点。

### 8-129-3 采样花多少，由什么守住

每一拍读什么，由看的人决定，并且只有下面这几种读法：

| 谁在看 | 每拍的平台读取 | 目标（每拍） | 现状 |
|---|---|---|---|
| 没有人 | 0：不建 `Counters`，只读两只原子数 | 0 | 已如此（§8-94） |
| 只看摘要 | 本进程 1 次（`memory-stats` 加 `cpu-time`），外加 `Health` 与积压的几次原子读 | ≤ 50 µs | 本进程约 1.1 µs（§8-96 决定 1） |
| 看整页 | 本进程 1 次，整机 1 次（CPU、内存、磁盘列表） | ≤ 50 µs；上限 1 ms，即 1 Hz 下一个核的 0.1% | 整机 CPU 0.7–8 ms、磁盘 0.2–0.8 ms，超出上限 |
| `gauge` 量一棵进程树 | 进程表 1 次，在 `gauge` 自己的进程里 | 上限：每拍至多一次整表读取，拍距不短于 250 ms | Windows 上一次 17–65 ms（§8-96 决定 1） |

城的采样线程没有一条路径去遍历进程表：整表读取只在 `monitor::tree` 里，城的采样线程不调用它。

- **门是计数。** `Counters` 与 `tree::Tree` 各自数平台读取的次数，以一个 `Reads { own: u64, machine: u64, table: u64 }` 交出。测试按看的人走 N 拍，断言三项的精确值：没有人是 0/0/0，只看摘要是 N/0/0，看整页是 N/N/0，`gauge` 的进程树是 0/0/N。这几条断言是 CI 的回退门，与 `index_refresh_read` 同一种做法：它们与机器快慢无关，而共享 CI 机器上的墙钟噪声会让一个墙钟门时红时绿。
- **墙钟只记录。** `instrument_monitor_beat`（`#[ignore]`，`just bench` 按 `/::instrument_/` 跑它）对每种读取各采 200 次，经 `Spread` 印出 floor、p50、p99（µs），读数登记到 `budgets.toml` 的新行 `[monitor_beat]`，不设门，理由与 `[local_latency]` 相同。
- **线上看得见开销。** 采样线程用单调钟量这一拍的读取用了多久，下一拍的 `Sample.read_nanos` 带上它，所以在看的人从同一条读数流里就能看到采样本身花了多少。
- 看整页时超出上限的那部分（`sysinfo` 的整机 CPU 与磁盘列表）仍是性能上的待办，重开条件见 §8-96 决定 1。

### 8-129-4 `sprawling gauge`

```text
sprawling gauge [--at <addr>] [--token <t>]                                一座服务中的城（别名 top）
sprawling gauge --pid <pid> [--every <ms>] [--samples <n>]                 一个在跑的进程和它的子孙
sprawling gauge [--every <ms>] [--samples <n>] -- <program> [<arg>...]     一条命令，跑 n 次
```

```rust
// bin::main::gauge —— shape: adapter
pub(super) fn verb(read: &Arguments) -> ExitCode;
pub(super) enum Subject {
    City { at: String, token: Option<String> },
    Process(Watching),
    Command(Running),
}
pub(super) struct Watching { pid: u32, every: Every, beats: Option<NonZeroU32> }  // beats 缺省：直到根进程退出
pub(super) struct Running { program: OsString, args: Vec<OsString>, every: Every, samples: NonZeroU32 } // samples 缺省 1；程序名单独一个字段，空命令写不出来
pub(super) fn subject(read: &Arguments) -> Result<Subject, Misread>;
pub(super) enum Misread {                   // 每一臂退 2，Display 写出最近的合法写法
    TwoSubjects,                            // --pid 与 -- 同时出现
    NotForThisSubject { flag: &'static str, subject: Measured },
    OutOfRange { flag: &'static str, given: String, range: &'static str },
    NothingAfterDashes,
}
pub(super) enum Measured { City, Process, Command } // Misread 说出是哪个对象，并写出它的合法写法
pub(super) struct Every(Duration);          // 250 ms ..= 60 s，缺省 1 s
pub(super) struct Run { index: u32, exit: Option<i32>, wall: Duration, watched: Watched, child: ChildPeaks }
pub(super) struct Watched { beats: u64, seen: Option<Seen>, read_cost: Duration } // 拍线程交回的东西
pub(super) struct ChildPeaks { private_bytes: Option<u64>, working_set_bytes: Option<u64> }
pub(super) struct Host { cores: NonZeroUsize, physical_bytes: u64 } // available_parallelism 与 monitor::memory::read
// bin::main::gauge::running —— shape: adapter；一条命令跑 n 次：spawn 到 wait 的单调钟用时、拍线程 sprawling-gauge、子进程峰值
pub(super) fn run(running: &Running, audience: Audience, out: &mut impl Write) -> Result<(), AxError>;
// bin::main::gauge::lines —— shape: projection；每种行只在这里变成文字
pub(crate) fn city_line(sample: &Sample) -> serde_json::Result<String>; // {"line":"city", 其后是 Sample 的字段}；watching 调它
pub(super) fn tree_line(at: Duration, reading: &TreeReading, audience: Audience) -> String;
pub(super) fn run_line(run: &Run, audience: Audience) -> String;
pub(super) fn spread_line(spread: &Spread, failed: u32, host: Host, audience: Audience) -> String;
// bin::monitor::tree —— shape: adapter；pub，因为读它的 gauge 在二进制那一半
pub struct Tree;                            // 私有：sysinfo::System、根 pid、每个见过的 pid 的上一读数、各峰值、读取次数
impl Tree {
    pub fn open(root: u32) -> Tree;
    pub fn read(&mut self, elapsed: Duration) -> Option<TreeReading>; // 根已不在：None
    pub fn seen(&self) -> Option<Seen>;     // 一拍都没读到：None
    pub(crate) fn reads(&self) -> Reads;
}
pub struct TreeReading { pub processes: u64, pub cpu_permille: Option<u64>, pub private_bytes: u64, pub working_set_bytes: u64, pub read_bytes: u64, pub written_bytes: u64 }
pub struct Seen { pub cpu_ms: u64, pub read_bytes: u64, pub written_bytes: u64, pub peak_private_bytes: u64, pub peak_working_set_bytes: u64 }
// bin::monitor::counters
pub(crate) struct Reads { pub(crate) own: u64, pub(crate) machine: u64, pub(crate) table: u64 }
impl Counters { pub(crate) fn reads(&self) -> Reads; }
// bin::monitor::spread —— shape: value
pub struct Spread { /* 私有：samples、floor、p50、p95、p99、peak、suspicious */ }
impl Spread {
    pub fn of(head: Duration, tail: impl IntoIterator<Item = Duration>) -> Spread; // 头是参数，空集写不出来
    pub fn p(&self, share: Share) -> Duration;  // 最近秩：第 ⌈n·p/100⌉ 个（一起）
    pub fn floor(&self) -> Duration;  pub fn peak(&self) -> Duration;
    pub fn samples(&self) -> u64;     pub fn suspicious(&self) -> u64; // 超过 p50 的 SUSPICIOUS_TIMES 倍
}
pub enum Share { P50, P95, P99 }
pub const SUSPICIOUS_TIMES: u32 = 3;
// bin::audience —— shape: value；库里的模块，二进制的 view、watching、gauge 都读它
pub enum Audience { Agent, Person }
impl Audience { pub fn of_stdout() -> Audience; } // stdout 是终端即 Person
// 为二进制那一半打开的三处：采样点、读内存、单位换算各只有一份
pub fn bin::serving::standing::monotonic_now() -> Instant;
pub fn bin::monitor::memory::read() -> Memory;
pub enum bin::monitor::top::Unit { Permille, Bytes, Nanos, Count }  impl Unit { pub fn reading(&self, value: u64) -> String; }
// bin::wire_client::watching
pub(crate) fn top(at: &str, token: Option<&str>, audience: Audience) -> Result<(), Unheard>; // 连不上是 Unheard::NoCity
// bin::main::verbs / grammar
pub(super) enum AfterDashes { Refused, Command } // Row.after_dashes：这个动词收不收 `--` 之后的词
impl Arguments { pub(super) fn after_dashes(&self) -> Option<&[String]>; } // 没有 `--`：None；`--` 后无词：Some(&[])
// accounting::worker::Hands（`crates/accounting/Spec.lean` §8-11）
pub monotonic: fn() -> Instant,             // 生产交 monotonic_now；派活准备与 probe 的用时都读它
```

**选哪个对象。** 给了 `--pid` 是 `Process`，`--` 之后有词是 `Command`，都没有是 `City`。命令行错误退 2（§8-103 的 `Line`），一行说明哪里错、最近的合法写法是什么：`--pid` 与 `--` 同时出现；`--at`、`--token` 配 `Process` 或 `Command`；`--every`、`--samples` 配 `City`（城的拍距由城定，§8-96）；`--every` 在范围外或不是整数；`--samples` 为 0 或不是整数；`--` 之后没有词。

**`--` 之后原样交出。** 第一个 `--` 结束 sprawling 自己的参数，之后每个词原样成为被量命令的 argv，不当作标志读。§8-89 的优先规则（`--help`、`--version` 出现在任何位置都先生效）只扫描到第一个 `--` 为止，所以 `sprawling gauge -- cargo --version` 量的是 cargo。命令表每行多一项：这个动词收不收 `--` 之后的词，只有 `gauge` 收。

**一次 run 从哪里量到哪里。** 从 `spawn` 之前到 `wait` 返回：可观察的端点是看到进程退出（citysim D2）。`wait` 在调用线程上阻塞，拍在一条名为 `sprawling-gauge` 的线程上走，所以一次 run 的结束由 `wait` 看到，`wall_us` 没有一拍那么大的误差。被量命令的 stdin 是空的；它的 stdout 与 stderr 都接到 `gauge` 的 stderr（`Stdio::from(io::stderr())`），`gauge` 的 stdout 上只有读数行。n 次 run 依次跑，第一次不丢：它是冷的那一次，`run` 行的 `index` 为 0，spread 里 floor 与 p50 的差有一部分是它留下的，其余是机器的负载。

**给 agent 的输出（stdout 不是终端）。** 每行一个 JSON 对象，值只有整数、`null` 与 `line` 这一个字符串键，键序固定，单位写在键名末尾（`_us`、`_bytes`、`_permille`），没有单位的是计数。没有量到的值是 `null`，不是 0。

- `City`：每秒一行 `{"line":"city", …}`，其余键是 `Sample` 的字段名（§8-95 的 `json_line`，多出 `line` 一个键）。`Sample` 没读到的项仍按 §8-96 读作 0。
- `Process`：每拍一行 `{"line":"tree","at_us":…,"processes":…,"cpu_permille":…,"private_bytes":…,"working_set_bytes":…,"read_bytes":…,"written_bytes":…}`，是这一拍还活着的整棵树。第一拍没有上一拍可比，`cpu_permille` 为 `null`；CPU 是两拍之间整棵树的 CPU 时间增量除以墙钟增量与核数，截到 `0..=1000`。根进程退出、或走满 `--samples` 拍，就结束。
- `Command`：每次 run 结束一行 `{"line":"run","index":…,"exit":…,"wall_us":…,"beats":…,"seen_cpu_us":…,"seen_read_bytes":…,"seen_written_bytes":…,"seen_peak_private_bytes":…,"seen_peak_working_set_bytes":…,"child_peak_private_bytes":…,"child_peak_working_set_bytes":…,"read_cost_us":…}`，最后一行 `{"line":"spread","samples":…,"failed":…,"floor_us":…,"p50_us":…,"p95_us":…,"p99_us":…,"peak_us":…,"suspicious":…,"cores":…,"physical_bytes":…}`。`exit` 是被量命令的退出码，被信号结束时为 `null`；`failed` 是 `exit` 不为 0 的 run 数。`seen_*` 取自拍：CPU 与读写字节是每个在某一拍出现过的进程最后一次读数之和，两项峰值是各拍整棵树之和里最大的那一拍，所以它们都是下界，两拍之间生灭的进程不在内；`seen_cpu_us` 的分辨率是平台计数器的 1 ms（`sysinfo` 的 `accumulated_cpu_time` 以毫秒给出），键仍记微秒，与其余时长同一单位；这次 run 一拍都没走（比 `--every` 短）时 `beats` 为 0，`seen_*` 全为 `null`。`child_peak_*` 是直接子进程由平台记下的峰值：在 Windows 上 `wait` 之后、句柄关闭之前经 `win32job::utils::get_process_memory_info` 读出，是精确值而不是拍上的读数；其他平台为 `null`。`read_cost_us` 是这次 run 里读进程表花掉的时间，agent 据此判断测量本身扰动了多少。`cores` 与 `physical_bytes` 写出机器的等级，不写机器是谁。

**给人的输出（stdout 是终端）。** `City` 是 §8-95 的一屏。`Process` 每拍一行，`Command` 每次 run 一行、最后一行 spread，读数按 §8-95 的单位规则写（时长按 §8-129-2 *单位*，字节一位小数，截断）。单位换算只有 `monitor::top` 那一份，`lines` 调它。

**退出码**（§8-103 的 `Exit`）：0，测量做完了，被量命令失败也是 0，它的退出码是 `run` 行里的数据；1，起不了被量的程序（找不到程序是 `E_PATH_NOT_FOUND`，subject 是程序名，recovery 是检查 PATH 或写全路径；其余起不来是 `E_TOOL_UNAVAILABLE`，带平台给的原因），或者没有这个 pid 的进程（`E_INVALID_ARGS`，recovery 是给一个在跑的进程的 id）；2，命令行读不懂；4，`City` 的 `--at` 那里没有城（与 §8-97 相同）。`City` 在连续 5 s 没有读数时正常结束（§8-97 决定 2）。

### 8-129-5 skill `skills/gauge/`

`skills/gauge/SKILL.md`，许可 MPL-2.0，列进 `skills/README.md` 的表与 `skills/LICENSES.md`；发行包按目录收 `skills/`，不必另列。它教 agent 按这个次序做：

1. 先把负载钉住：同一份输入、同一个构建（产品的 feature 集，citysim D9）；输入是文件时记下它的摘要，两条读数只在摘要相同时可比（citysim D8）。
2. 取基线：`sprawling gauge --samples 20 -- <命令> > before.jsonl`，读最后那行 spread。floor 贴着设计的下限，p50 带着机器其余的负载（`tools/citysim/Spec.lean` §8-6）；`suspicious` 不为 0 时先看是哪几次、是不是第一次。
3. 一次只改一处，取 after；前后交替跑几轮（一轮先 before 后 after），只在同一机器等级上比较 floor 与 p50。
4. 回退门用确定性计数，墙钟只记录：找一个随规模增长的计数（读过的字节、核对的行数、文件操作数），在 N 与 2N 两种规模下断言它。
5. 资源：长命令看 `run` 行的 `seen_*` 与 `child_peak_*`；一个已在跑的进程用 `--pid`；一座服务中的城用 `gauge --at <地址> > 文件` 在后台记录。`city` 行里的 0 可能是「没读到」（§8-96），其余行里没读到的是 `null`。
6. 在 sprawling 自己身上：一次调用花多久，读 `view` 给出的 `tool_called` 与 `tool_result`、`model_called` 与 `model_returned` 的 `t`（账本版本 2 起才是这一行自己的时刻）；开城各段读 `serve` 写出的 `opened the city in` 那一行；派活准备打开 `--log trace`。人能感知的操作落在毫秒、内部热路径落在微秒，以秒计的读数就是要做的活（人的定规）。
7. 报读数时写机器等级（核数、内存、盘的种类）和负载的摘要，不写机器名、账户名和路径（AGENTS.md *Privacy*）。

### 8-129-6 对 wire 的需求（`crates/wire/Spec.lean` §8-47）

- `Sample.view_backlog: u64`：视图积压的条数，读 §8-123 给出的读法：`spawn_sampler` 收下 `bin::serving::folding::Folding` 交出的 `Backlog` 句柄，每一拍读一次 `records()`。
- `Sample.read_nanos: u64`：上一拍的读取用时，由采样线程用单调钟量出：`spawn_sampler` 收下一只 `fn() -> Instant`（生产交 `serving::standing::monotonic_now`），在读计数器前后各取一次，差值填进下一拍；第一拍为 0。
- 加上这两项，`Sample` 是 15 个 `u64`，历史 300 × 15 × 8 = 36 000 字节，仍在 `HISTORY_BUDGET`（64 KiB）之内，编译期断言不必改。
- `monitor::top::screen` 随之多两行（`view backlog` 记录条数、`read` 微秒）；WebUI 监视页的两行随页面的改动做。
- 不要求：`relay_p50_nanos`、`event_to_screen_p50_nanos`、`queued_runs` 的来源（§8-98 的当前状态）。

**决定。**

1. **读数不进 Ledger，时刻除外。** 时刻是城的历史，已经写在每行的 `t` 里；时长与资源读数是运行这座城的主机在那一刻的事实，与 §8-121 决定 2、`docs/logging.md` §2 同一条界线。被否：给资源读数一个新事件种类，按整数单位经单写者写入——1 Hz 一天 86 400 行，每行都要过 relay 与记账线程的屏障，而记账线程上的长任务正是 §8-123 要设界的；整链审计、重放、快照与每个折叠都随它变长；读数进了哈希链就删不掉，而资源读数随时可丢。重开条件：某个城内的决定要读资源的历史（今天的决定都只读此刻：内存闸与卷卸载）。
2. **城不在自己的保留子树里常驻记录。** 被否：在城里放一份只追加的读数文件，一直写。那是城树里的第二个写者，要为它设计保留与轮转（轮转归操作系统，`docs/logging.md` §7），没人看的城每天多写 86 400 行，违背 §8-94 决定 1；在别的项目里量一条命令的 agent 手里也没有城。后台记录由一个看的人写出（§8-129-1）。重开条件：人要看一座在没人看时崩溃的城在崩溃前的资源曲线。
3. **一个动词，收下 `top`。** `top` 成为 `gauge` 的别名，`sprawling top` 的行为不变，只多出 `line` 键。被否：在 `top` 旁边再加一个动词——两个动词读同一条读数流，参数与输出要在两处维持；也被否：测量手段只留在 `xtask` 与 citysim——它们不随发行包走，在别的项目里的 agent 手里什么也没有。名字取 `gauge`：`measure` 已是 browser 工具的一个动作（量元素的框，glossary 的 **browser**），同一个词会担两个概念；`bench` 是开发者跑固定场景的配方 `just bench`。
4. **进程树经 `sysinfo` 整表读取，`xtask mem` 留在城外。** 以安全接口读直接子进程以外的进程，`sysinfo` 是依赖树里唯一的路，它在 Windows 上每次都遍历整张进程表（17–65 ms），所以 `gauge` 的拍距不短于 250 ms，每拍至多一次（§8-129-3）。被否：把被量命令放进一个 Job Object，读它的累计 CPU、读写字节与峰值——要 `QueryInformationJobObject`，`win32job` 的安全接口只给 pid 列表与限额；工作区 `forbid` 了 `unsafe`，Zig 叶子也要一处 `extern` 的 `unsafe`，而 `forbid` 在 crate 内提不起来。`xtask mem` 读 PowerShell 的 `PeakPagedMemorySize64`（峰值 commit），`sysinfo` 不给峰值，所以它不改为调用 `gauge --pid`。重开条件：出现以安全接口给出 Job Object 累计计数、或任意 pid 的峰值 commit 与读写字节的 crate；那时 `seen_*` 换成精确值，`xtask mem` 改为调用 `gauge --pid <pid> --samples 1`。
5. **`gauge` 不转述被量命令的退出码。** `Exit` 是退出码的唯一定义（§8-103）；把子进程的 3 或 4 原样退出，会被读成「城没回话」「没有城」。被量命令的退出码在 `run` 行里，失败的次数在 spread 行里。
6. **被量命令的 stdout 接到 `gauge` 的 stderr。** 被否：继承 stdout——它的输出会和读数行交错，agent 解析不了；丢弃——失败的 run 就没有诊断可看。
7. **没量到写 `null`。** agent 会把 0 当成一个读数。`city` 行照旧沿用 `Sample` 的 0（§8-96），skill 写明这一点；让 `Sample` 的每一项都能为空是线协议的改动，本节不要求。
8. **分位数取最近秩。** 登记册的 `[install]` 一行已写明「p50/p95/p99 nearest-rank」，`bench_startup` 的 `Samples::of` 就这样算；`bench` 的 `Reading::of` 改为经 `Spread`，它登记在 `budgets.toml` 里的各行要在新规则下重取。被否：两份各留各的——同一个 p50 在两份读数里指的是不同的样本，读数行上看不出这个差别。
9. **开销用确定性计数守，墙钟只记录**（§8-129-3）。被否：给一拍的用时设墙钟门——共享 CI 机器上它时红时绿，而拍的读取次数在任何机器上都一样。
10. **没有 Lean 模型。** 采样线程的读法由看的人一次选定，`gauge` 的 run 依次进行，两处都没有交错；要守的性质是读取次数，由 §8-129-3 的计数断言与 `Subject` 这个穷尽枚举守住。重开条件：`gauge` 同时量多个对象，或采样改为多线程。
11. **`gauge` 自己的命令行错误是它自己的枚举 `Misread`，不并进 `grammar::LineError`。** `LineError` 只说命令表判得了的事：动词、标志、位置参数各认不认得；「`--pid` 与 `--` 不能同时给」「`--every` 只配进程与命令」是 `gauge` 选对象的规则，放进 `LineError` 就要让解析器知道一个动词的对象怎么选。两者退出码相同（2），写法都是一行说明加最近的合法写法。被否：给 `LineError` 加通用的「冲突」「越界」臂——它们只有 `gauge` 一个读者，说明文字又各不相同。
12. **记账一侧的单调钟经 `Hands.monotonic` 交进来，是一只 `fn() -> Instant`。** 与 `read_memory`、`read_volume` 同形：worker 碰这台电脑的每只手都在构造时一次交进来（`crates/accounting/Spec.lean` §8-11），生产交 `monotonic_now`，测试的 `Hands` 交测试夹具里同样读单调钟的那一只。被否：给 `accounting::Clock` 加一个返回 `Instant` 的方法——城钟是 Ledger 的 `t` 的来源，单调钟只量时长，两者放进一个端口，换城钟（测试里拨快、拨停）的地方就得同时编一个单调钟；四个实现都要改。
13. **时长记整数微秒，显示以 10 ms 为界。** 毫秒时刻相减分不出 1 ms 以内的差别：一座测试城的 471 次工具调用里有 137 次记成 0 ms；内部热路径按微秒读（人的定规）。显示以 10 ms 为界，不到 10 ms 写整数 µs，10 ms 及以上写 ms（人的定规）。被否：读数行保留浮点毫秒（`0.555 ms`）——小数位数各处不一，比较两条读数先要换算，而浮点不进读数是本仓库的规矩；也被否：整数毫秒加一位小数——100 µs 以内的回退看不见。重开条件：要比较的读数落进纳秒（那时那一处用 `_ns`）。

**测试。** `monitor::counters::tests`：按看的人走 N 拍，`Reads` 等于 §8-129-3 的精确值。`monitor::spread::tests`：n 为 1、2、100、200 时各分位与手算的最近秩相等，`suspicious` 数对。`monitor::tree::tests`：起一个子进程，第一次读数 `processes ≥ 1`、工作集非零、`cpu_permille` 为 `null`，它退出后 `read` 为 `None`，`reads().table` 等于读的次数。`audience::tests`：测试运行器的 stdout 不是终端，读作 `Agent`。`main::grammar::tests`：`--` 之后的词原样交出，其中的 `--help`、`--version` 不生效；`gauge` 与 `top` 解析成同一个动词。`main::gauge::tests`：每条命令行错误退 2；三种行按整串比较；`--samples 3` 量一个很快退出的程序，stdout 恰好三行 `run`、一行 `spread`，程序以非零退出时 `gauge` 仍退 0、`failed` 为 3。citysim：`Reading::of` 与 `Samples::of` 的既有测试按最近秩改期望。accounting：城钟每读一次拨快一分钟时，派活准备、`mcp_tools` 与 probe 的用时仍小于一分钟。

**本节接口的当前状态。** §8-129-1 至 §8-129-6 的接口都已落地：`Spread` 是唯一的分位算法，citysim 两处读它；`Counters` 与 `Tree` 数自己的平台读取；`gauge` 是命令表的一行，`top` 是它的别名，`--` 之后的词原样交给被量的命令；`bin::audience` 是 TTY 规则唯一的一处；三处时长读 `Hands.monotonic`；`skills/gauge/` 随发行包走。`Sample` 带上 `view_backlog` 与 `read_nanos`（§8-129-6）；`instrument_monitor_beat` 的读数登记在 `budgets.toml` 的 `[monitor_beat]`，`[prepare_dispatch]` 的 `measured_by` 写的是单调钟。还没有的：`main::view` 的同名枚举换成 `bin::audience::Audience`（它的 `write_line` 方法要随之搬走）。`xtask mem` 仍读 PowerShell（决定 4）。
-/

/-!
## 8-117 `sprawling view`：给人的一面（`bin::main::view::keys`、`bin::main::view::arrange`、`bin::main::view::rounds`、`bin::main::view::frame`、`bin::main::view::detail`、`bin::main::view::follow`、`bin::main::view::terminal`）

**形状。** 五个纯模块，不碰终端也不碰盘。`keys` 是 decision：一个按键对应哪个 `Action`。`arrange` 是 projection：把 `accounting::lineage` 的 `RunLine` 排成一棵树，按显示顺序平铺成 `Entry`，每个 `Entry` 记着深度和父的下标。`rounds` 是 projection：把一个 run 在 `records` 里的行经 `accounting::views::turns`（`views::rounds::turns` 的公开投影，与 Views 的回合页同一份折叠）折成回合，再把回合与其中的调用排成那个 run 下面的 `Entry`。`frame` 是 state machine：`Face` 持有两个透镜共用的选中物、展开集合与详情模式，`apply(Action)` 改状态，`frame()` 按当前尺寸画出一帧文本行。`detail` 是 projection：任何记录都画成同一种缩进 JSON 树。`follow` 是 adapter：`open` 经 `storage::TailLines` 只读账本最新的 `FIRST_WINDOW_LINES` 行，折出窗口里的 lineage 与 `records` 行，同时在一条后台线程上跑整遍的 `LedgerIndex::rebuild` 与 lineage 折叠；`poll` 在整遍折完之前只看它到了没有，到了就交出整份（`Polled::Filled`），此后持有常驻的 `LedgerIndex` 与 lineage，只折上次之后追加的行（`Polled::Appended`）。`terminal` 是 adapter：stdout 是终端且没有任何过滤参数时，`view` 用 `follow` 读城，进 raw 模式与备用屏，读键、调 `apply`、画 `frame()`，退出时无论成败都把终端还原。它不做任何决定。`list` 是 projection：哪些树行可见、每行标什么、滚到光标可见的那一屏，以及账本透镜落在这一屏上的行。尚未做的：T8–T12 的 `ttyprobe` 验收。

```rust
// bin::main::view::keys
pub(super) enum Key { Char(char), Up, Down, Left, Right, Enter, Tab, Esc, PageUp, PageDown, Home, End, Interrupt }
pub(super) enum Action { Up, Down, PageUp, PageDown, First, Last, Collapse, Expand, SwitchLens, OpenDetail, CloseDetail, Quit }
pub(super) fn action_for(key: Key) -> Option<Action>;
// bin::main::view::arrange
pub(super) enum NodeKey { City, Building(String), Room(Address), Session(Address, Option<Seq>), Run(RunId), Round(RunId, u32), Call(RunId, Seq) }
pub(super) struct Entry { key: NodeKey, depth: usize, parent: Option<usize>, seq: Seq, label: String, detail: serde_json::Value }
pub(super) fn arrange(runs: &[RunLine], rounds: &Rounds) -> Vec<Entry>;
// bin::main::view::list
pub(super) fn visible(entries: &[Entry], expanded: &BTreeSet<usize>) -> Vec<usize>;
pub(super) fn tree_lines(entries: &[Entry], shown: &[usize], expanded: &BTreeSet<usize>, rounds: &Rounds) -> Vec<String>;
pub(super) fn scrolled(lines: Vec<String>, cursor: usize, rows: usize) -> Vec<String>;
pub(super) fn record_lines(records: &[Row], cursor: usize, rows: usize) -> Vec<String>; // 只给这一屏的行算哈希
// bin::main::view::rounds
pub(super) type Rounds = BTreeMap<RunId, Result<Vec<wire::Turn>, AxError>>;
pub(super) fn fold(run: RunId, rows: &[Row]) -> Result<Vec<wire::Turn>, AxError>;
pub(super) fn append_below(entries: &mut Vec<Entry>, at: usize, folded: &Result<Vec<wire::Turn>, AxError>);
// sprawling（库）
pub fn turns<'a>(records: impl IntoIterator<Item = &'a EventRecord>) -> Vec<wire::Turn>;
// bin::main::view::frame
pub(super) struct Size { columns: usize, rows: usize }
pub(super) struct Face;
pub(super) const FILLING: &str; // 状态行
impl Face {
    pub(super) fn open(runs: &[RunLine], records: Vec<Row>, size: Size) -> Face;
    pub(super) fn open_window(runs: &[RunLine], window: Vec<Row>, size: Size) -> Face;
    pub(super) fn fill(&mut self, runs: &[RunLine], whole: Vec<Row>);
    pub(super) fn apply(&mut self, action: Action);
    pub(super) fn resize(&mut self, size: Size);
    pub(super) fn frame(&self) -> Vec<String>;
    pub(super) fn is_closed(&self) -> bool;
    pub(super) fn follow(&mut self, runs: &[RunLine], appended: Vec<Row>);
}
// bin::main::view::follow
pub(super) const FOLLOW_TICK: Duration; // 100 ms
pub(super) struct Row { seq: Seq, run: RunId, line: String }
pub(super) struct Follow;
pub(super) const FIRST_WINDOW_LINES: usize; // 1000
pub(super) type Folded = (Vec<RunLine>, Vec<Row>);
pub(super) enum Polled { Filled(Folded), Appended(Folded) }
impl Follow {
    pub(super) fn open(dir: &Path) -> Result<(Follow, Folded), ViewError>; // 只有窗口
    pub(super) fn poll(&mut self) -> Result<Option<Polled>, ViewError>;
}
// bin::main::view::detail
pub(super) fn json_lines(value: &serde_json::Value) -> Vec<String>;
pub(super) fn line_lines(line: &str) -> Vec<String>; // 第一行 chain_hash，其后是 JSON 树；不是 JSON 的行画成一个字符串
// bin::main::view::terminal
pub(super) fn show(dir: &Path) -> Result<(), ViewError>;
```

**键。** `j`/`↓` 下一行，`k`/`↑` 上一行，`h`/`←` 折叠（已折叠时跳到父），`l`/`→` 展开（已展开时进第一个子），`PageDown`/`PageUp` 翻一屏，`g`/`Home` 第一行，`G`/`End` 最后一行，`Tab` 换透镜，`Enter` 进全屏详情，`Esc` 退出全屏详情，`q` 与 Ctrl-C（raw 模式下它是一个键，不是信号）关掉查看器。别的键没有动作；Windows 另报的松开与重复不算按键。与 WebUI 的 run 板同一套键（D-15）。

**树。** 城 › 楼（地址的第一段）› 房间（整个地址）› 会话（`RunLine.session`，没有就是房间的第一段 stretch）› run。父 run 在账本里时，run 挂在父 run 下面（分叉挂在分叉点下，标 `fork @<at_seq>`）；否则挂在自己的会话下；没有地址的 run 直接挂在城下。同一个父下的子按 `first_seq` 排；接替的 run 带 `after <predecessor>`。每个节点只有一个父。run 第一次被展开（`l`/`→`）时才折它的行：`rounds::fold` 从 `records` 里挑出这个 run 的行交给 `accounting::views::turns`，结果按 run 记在 `Face` 里，树重排时回合（`round <n> @<opened>`）排在这个 run 的分叉之前，每个回合下是它的调用（`<tool> <subject>`）。回合节点的 `seq` 是打开它的 `model_called`，调用节点的是它的 `tool_called`，所以 `Tab` 落在那一行上。一行解析不了时，这个 run 下只有一个节点，标签写出解析错误，而不是少掉几个回合。跟随时，已折过的 run 有新行就重折。详情画 `wire::Turn`、`wire::Call` 的 JSON。

**打开时的光标。** 最新的等人批的 run（`unanswered > 0`）；没有就是最新的 `active` run；再没有就是最新的 run；一个 run 都没有就是城。「最新」按 `first_seq`。只展开它的祖先。

**两个透镜共用选中物。** `Tab` 从树到账本：选中第一条 `seq ≥` 节点 `seq` 的行（run 的 `seq` 是它的 `first_seq`，别的节点是子树里最早的 `first_seq`）。从账本到树：选中这行所属的 run 节点并展开它的祖先；城自己的行选中城。

**首屏从尾部读。** 查看器打开时只付最新 `FIRST_WINDOW_LINES` 行的字节与解析，`Face::open_window` 按这些行画树，最后一行是状态行 `FILLING`，列表让出这一行。lineage 与 `HotView` 是按 seq 正向的折叠：窗口里看不到 `run_started` 的 run 没有地址（挂在城下），状态也只按窗口里的行定。整遍折叠到了，`Face::fill` 换上整份 lineage 与整份 `records`（窗口之前的行排在窗口前面，折叠看到的窗口之后的行排在后面），重折每个已折过回合的 run，状态行消失；选中的仍是同一个节点并展开它的祖先，账本行的光标仍在同一个 `seq` 上。整遍折完之前 `poll` 不读账本的追加：整遍折叠开始于窗口之后，追加的行都在它里面。

**跟随。** 查看器开着时，`terminal` 等键最多 `FOLLOW_TICK`（100 ms）；没等到就 `poll` 一次：`LedgerIndex::refresh` 说没变就什么都不做，有追加就把新行折进 lineage 并交给 `Face::follow`。所以服务中的城里新开的 run 最迟一个 tick 加一次折叠之后出现在树上，远在 250 ms 之内。`Face::follow` 换上新的树、把新行接到 `records` 末尾，并保持：选中的仍是同一个节点（按 `NodeKey` 找回；它不在了就是城），展开过的仍展开，此前不在树上的 run 展开它的祖先，让人看得见它。账本行的光标不动。

**帧。** 宽度 ≥ `SIDE_PANE_MIN_WIDTH`（110 列）时右侧常驻详情栏，左右各占一半，中间一列 `|`；窄时只画当前透镜，`Enter` 进全屏详情。全屏详情在任何宽度下都占满整屏。每行按字符截到栏宽；光标行以 `>` 开头；列表滚动到光标恰好可见。树行是缩进 + `+`（有子、折叠）/`-`（展开）/空格 + 标签；还没折过回合的 run 也标 `+`，因为它有没有回合要到第一次展开时才知道（决定 6），标空格会告诉人那里什么都没有。账本行是这一行链哈希的前 `GLANCE_DIGITS`（12）位、两个空格、原行，前缀由 §8-105 的 `chain_label` 写出。`record_lines` 先按 `scrolled` 的同一个公式定出这一屏从哪一行起，只给这一屏的行算哈希，所以一帧的代价跟屏高走，不跟账本长度走。哈希按 `Row.line` 的字节算：`follow` 只收解析成功的行，JSON 必是 UTF-8，`Row.line` 的字节与盘上一致；`Row` 以后若收未解析的行，哈希要改为从原字节算。详情对 run 节点画 `RunLine::to_json()`；对账本行先画一行 `chain_hash: <64 位十六进制>`，再画解析后的 JSON，解析不了就画原文字符串。

**决定。**

1. 键到动作是一张纯表，帧是 `Face` 的纯函数；终端 adapter 只做读键、调 `apply`、写 `frame()`。被否掉的：在事件循环里直接改光标——那样每个动作只能在真终端里测。
2. `crossterm` 只开 `events` 与 `windows` 两个特性：这一面不画颜色、不读粘贴与剪贴板，默认特性只会多链接没人调用的代码。被否掉的：自己写 Windows 控制台与 termios 两套 raw 模式——那是两份平台代码，换来的只是少一个依赖。
3. 全屏详情在宽屏上也占满整屏，而不是在宽屏上忽略 `Enter`：同一个键在任何宽度下意思一样，大记录也能用满宽度看。
4. 「等人」按 run 数在 `accounting::lineage` 里，而不是读 `views::governance` 的待批表：待批表按 `id` 记，不记是哪个 run 提的，从它答不出「哪个 run 在等人」。代价是「请求加一、答复按 `id` 减一」这条规则有两处折叠，靠同一对 kernel 类型保持一致。条件变了就重议：待批表记下提出它的 run 时，lineage 改读它，删掉自己这一份。被否掉的：光标只看 `active`（等人批的 run 往往已经冻结，最需要人的那一个反而不在光标下）。
5. 跟随靠轮询 `LedgerIndex::refresh`，不开文件系统通知，也不连服务中的城的 socket：没变时一次 refresh 只是一次目录列表加每段一次 `stat`，100 ms 一次对任何盘都是噪声；而通知在 Windows、inotify 与网络盘上是三套行为，socket 又要求查看器先知道城在不在服务。城不在服务时轮询什么都读不到，所以跟随不区分两种情况。条件变了就重议：`refresh` 在没变时也要读字节时。被否掉的：只在打开时读一次（人得退出重开才看得见新 run）。
6. 回合在展开时才折，从已在内存里的 `records` 行折，不回盘：打开时就给每个 run 折回合，会让 40 万行的账本在首屏前多解析一遍，而人一次只看几个 run；回盘按 `run_seqs_before` 读会让纯的 `Face` 碰盘。代价是展开一个 run 要扫一遍 `records` 挑它的行、再解析这些行。条件变了就重议：`records` 不再整本常驻内存时（从尾部倒读首屏之后），改由 `follow` 按 `LedgerIndex::run_seqs_before` 读这个 run 的行。被否掉的：在 `view` 里另写一份回合折叠（与 Views 的回合页是同一个规则的两份）。
7. 首屏的窗口按行数定（`FIRST_WINDOW_LINES` = 1000），不按终端行数，也不按字节：树要的是足够多的 run，一屏的行数给不出几个 run；一千行是几百 KB 的读与解析，在任何盘上都是首屏里的小头，而整遍折叠在后台，不挡第一帧。后台用一条 `std::thread`，因为查看器是同步的终端循环，没有运行时可以借；它只活到整遍折完，结果经一条 channel 交回，查看器退出时它随进程结束。条件变了就重议：run 索引快照（S5.22）落地后，首屏直接读快照，窗口与后台折叠一起删掉。被否掉的：打开时同步读完整本（40 万行的账本首屏要等整遍折叠）。
-/

/-!
## 8-137 `sprawling view --since、--until`：按 UTC 选行（`bin::main::view`；`crates/runtime/Spec.lean` §8-10、§8-57）

**形状。** 不变：`main/view.rs` 仍是 adapter。`Selection` 多一个条件 `span: runtime::clock::UtcSpan`，由 `--since <utc>` 与 `--until <utc>` 给出，两者都经 `runtime::clock::parse_iso` 读，再由 `UtcSpan::new` 组成一个值。

```rust
// bin::main::view
pub(super) struct Selection { tail, from, run, kind, who, grep, span: UtcSpan }
```

**选中。** 一行选中当且仅当它信封的 `t` 在 `[since, until)` 里（`UtcSpan::contains`），其余条件照 §8-105 同时成立。`t` 是这一行自己记下的那一刻（`crates/kernel/Spec.lean` §8-4；回合里等来的四种行各记各的，runtime D5），所以时间窗精确到单次调用；版本早于逐行时刻的账本里，同一回合的行共用回合的 `t`，时间窗就只精确到回合。

**逐行判断，不靠 `t` 有序。** `t` 不随 `seq` 单调：并行只读段的开始时刻可以早于前一条的答复，墙钟也会回拨。所以 `--since、--until` 与 `--grep` 一样是流式过滤：走完 `--from`/`--run` 给出的整段，每一行解析一次信封再判，不在第一条越过 `until` 的行处停，也不二分。`--tail N` 仍最后作用。代价与读到的行数成正比，与 `--kind`/`--who` 相同。

**失败。** `--since` 或 `--until` 不是 `iso` 写出的形状：stderr 一行 `sprawling: view: --since '<原文>': <parse_iso 的 recovery>`，recovery 给出正确写法（`2026-05-14T09:31:07Z`，UTC，到秒，以 `Z` 结尾），退出 2。`--until` 不晚于 `--since`：`sprawling: view: --since and --until: <UtcSpan::new 的 recovery>`，退出 2。合法而什么都没选中的区间照常退出 0、写零行。

**人的那一面。** 带了 `--since` 或 `--until` 就是带了过滤参数：终端前不进 §8-117 的交互界面，按 §8-105 写选中的行、行前带链哈希。

**测试。** `the_records_lens_keeps_the_lines_inside_a_time_window`、`a_line_whose_time_steps_back_is_judged_on_its_own`、`a_moment_view_cannot_read_names_the_shape_it_wants`（`bin::main::view::tests`）。
-/

/-! D1 测量读数不进 Ledger，也不常驻城里的盘；时长只取单调钟（§8-129）

一件事发生的时刻是城的历史，写在每行的 `t` 里，含义由 `EventRecord::moment` 给出；主机为城的一段工作花了多久、机器的资源被用了多少，是运行这座城的主机在那一刻的事实：前者写成诊断行，后者只在有人看时进监视器的 300 点历史，要留下来就由看的人（`sprawling gauge`）写到它自己的 stdout。时长一律是 `bin::serving::standing::monotonic_now` 两次读数之差，城钟只用来给账本行打时刻。**被否掉的**：资源读数作为一个整数单位的新事件种类写进 Ledger——每秒一行都要过记账线程的屏障，整链审计、重放与每个折叠随之变长，而进了哈希链的读数删不掉；城在保留子树里常驻写一份读数文件——城树多一个写者，要自带保留与轮转，没人看的城也一直付开销。条件变了就重议：城里有一个决定要读资源的历史，而不只是此刻的读数。
-/

/-! D2 `replay <ledger-dir>`：「这里没有账本」不得与「验过且为空」同形

本子命令的路径是人敲的，故它先问 `storage::ledger_segments_at`，一段都没有即报 `E_PATH_NOT_FOUND` 并给 recovery，不进验链。有段时经 `storage::audit_chain` 一段一段地验，每行过同一个 `LineCheck`，常驻一段；它答的只是行数与 tail seq（`chain verified: <n> line(s), tail seq <n-1 或 none>`，`main::tests` 的 `replay_names_the_lines_it_verified_and_the_tail_seq` 钉住这个形状），用不着 `VerifiedLedger` 那份整本的原始行与记录；断链照旧是那一行的三段式拒绝。**依据为什么在这一层而不在 `runtime::replay` 或 `storage::audit_chain`**：它们的生产调用方都自持城根算出路径，而已开未写的城就是一个无段目录（`JsonlLedger::open` 只建目录），在那一层报错会把合法启动打红，并迫使每个调用方各写一份相同的守卫。空账本仍然合法，故问的是「有没有段」而不是「有没有行」。参见 `crates/runtime/Spec.lean` §8-1、`crates/storage/Spec.lean` §8-1。
-/

/-! D23 `view` 在终端前给每一行标上它的链哈希，不加字段，也不给 agent 看（§8-105、§8-117）

人要一个能记下来、以后拿来对照某一行的值，账本每一行已经有一个：`kernel::ledger::chain_hash` 对这一行规范字节算出的 BLAKE3，下一行的 `prev` 存的就是它，它覆盖整行，`t` 与载荷都在内。在载荷里或旁边再存一份同源的哈希，就是同一件事有两个权威。哈希从盘上的字节现算：一行一次 BLAKE3，比解析这一行便宜，交互界面又只给屏上的行算，四十万行的账本也不多付。谁要哈希仍按 §8-105 决定 1 的 TTY 规则分：管道与文件里仍是账本原行，agent 解析的字节不变。哈希放在行前而不是行后，因为定宽的一列在终端折行后仍然对齐，而交互界面按栏宽截行，行后的哈希根本画不出来；列表只放前 12 位，因为 64 位会在半屏宽的列表里挤掉整行，完整的值在详情栏第一行。12 位是 48 bit，四十万行里出现一对同前缀的行的机会约为万分之三，所以前缀只用来凭眼睛找行，要记下来就用完整的值。**被否掉的**：每行后面接哈希，理由见上；`Row` 在折叠时就带上哈希，整遍折叠要给每一行多算一次、多存 32 字节，而人一次只看一屏；加一个 `--hash` 参数，人坐在终端前就要哈希，参数只是让人多敲一次。条件变了就重议：常见的 agent 宿主改在伪终端里跑命令、并解析 `view` 的输出时，TTY 就分不开两个主人，那时改为显式参数。
-/

/-! D24 `view --since、--until` 按每一行信封的 `t` 流式过滤，不建时间索引（§8-137）

时间窗读的是信封 `t`，因为它就是这一行记下的那一刻，任何种类都有它；按种类去载荷里挑时间字段，就是同一件事有两个家。不二分也不提前停：`t` 不随 `seq` 单调，在第一条越过 `until` 的行处停下会漏掉后面回退的行。**被否掉的**：在 `LedgerIndex` 里给每行多存一个 8 字节的时间列再二分——它要 `t` 有序，而 `t` 无序；也让索引多一份要维护的事实。条件变了就重议：四十万行的账本上 `view --since` 超过一秒，或页面需要按时间跳转。
-/

/-! D25 `whose --trace` 读盘作答，候选就说是候选（§8-136，accounting D28）

区间以同一个 run 的上一个提交为界，调用经 rounds 的那一份配对规则读出，同楼别的 run 只给条数。CLI 读盘而不走线上查询，城不在服务也答得出，与 `whose` 本身同一条理由。**被否掉的**：只列写调用——读调用决定了写什么；把同楼别人的调用并列出来——读者会把候选读成原因。条件变了就重议：页面要显示一个提交的调用时。
-/

/-! D27 `playback` 是一个两个词的动词，导出写 stdout 或一个新文件，只在 bundle 完整之后写（§8-126、`crates/accounting/Spec.lean` §8-12）

回看一段工作流有两个动作：导出与复核，它们的标志不同（导出读选择与 `--out`，复核读 `--bundle`/`--city`），所以是两行；放在一个词 `playback` 之下，是因为总览里两者挨着，人找到一个就找到另一个。命令表的一行可以带两个词，解析先试两个词，其余不变，§8-89 决定 1「动词需要子动词」的重议条件因此被这一个最小的扩展满足，没有换参数库。输出不沿用 `view` 的做法：`view | head` 截断仍算成功，是因为账本原行本来就一行一行有意义；一份截断的 bundle 读不回来，却可能被当成一份完整的东西留下，所以 bundle 先整份算好、量过尺寸再写，管道中途关闭是失败，`--out` 经暂存文件与硬链接落位、已有目标不覆盖。**被否掉的**：`export --playback` 之类挂在现有动词上的标志（`export` 打包整座城，两件事的输入与输出都不同）；`--out` 默认写进城里（导出件不进 git，城里的保留导出位置由布局 owner 在居民入口落地时定）；`rename` 落位（在 Unix 上会覆盖已有目标）。条件变了就重议：人也要一个不必自己选路径的去处时，`--out` 缺省可以写到城里的保留导出位置（§8-132 给居民的那一处），而不是 stdout。
-/

/-! D28 居民的 `playback` 是一件工具、两个动作，按写登记，读者由上下文定（§8-132、`crates/accounting/Spec.lean` §8-13）

导出要写文件，复核只读；工具的效应按登记而不是按调用，一件工具只能登记一种，所以取两者中较强的写：代价是复核也走写门、不被推测执行提前跑，而一次复核本来要重算整段历史，提前跑它省不下什么。导出件落在城根保留子树下的 `playback/<楼>/`，而不是居民的房间或 worktree：保留子树没有写域够得到，居民改不了一份已经写下的报告，只能另导出一份；它被城根的 `.gitignore` 挡在历史之外；它不随 worktree 清扫消失，没有 worktree 的 run 也有同一个去处。按楼分目录，是因为复核要用同一个读者重算，一栋楼的导出只有同一栋楼的居民复核得了，机密楼的导出也就不会被别的楼读到。**被否掉的**：`playback_export` 与 `playback_check` 两件工具（复核可以按 `Read` 登记，但一个动作拆成两件工具，模型要多认一个名字，而两者的参数一半相同）；写进房间并靠 `.gitignore` 挡住（worktree 会被清扫，居民能用 `edit` 改报告）；让居民在参数里选读者（读者是读界的输入，交给被读界约束的一方去选，读界就没有意义）。条件变了就重议：工具的效应可以按调用声明时，`check` 改回 `Read`。
-/

/-! D29 playback 的时间条件是两扇门各读三个原文、交给同一个函数（§8-143；`crates/accounting/Spec.lean` §8-17 与 accounting D29）

`--since`、`--until`、`--day` 与城工具的 `since`、`until`、`day` 都只是原文，`accounting::playback::Window::span` 一处把它们读成一个 `UtcSpan`；这样人与居民导出同一天时，`source.selection` 记下同样的两端，复核也按同一个区间重算。`--day` 不进 `view`：`view` 是看行的透镜，`--since、--until` 已经够它用，多一个旗标就多一种写法要维护。**被否掉的**：CLI 复用 `view` 的 `--since、--until` 读法而城工具另写一份（两扇门读同一个条件就会有两处）；`--day` 与 `--since、--until` 互斥（「这一天九点以后」是一个合法的问题，交集就是它）。条件变了就重议：人要按本地日期回看时，那时要一个时区，而城今天只认 UTC（`crates/runtime/Spec.lean` §8-10）。
-/
