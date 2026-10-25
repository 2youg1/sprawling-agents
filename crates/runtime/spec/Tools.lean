-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools

规定 `tools`、`tools::edit`、`tools::status`（`crates/runtime/src/` 下同名的文件）。工具面：四件工具的共同约定、写域的两道闸与「只新建」；edit 的准入在 `spec/Tools/Edit.lean`（§8-64）。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::tools::edit::tests`、`runtime::tools::status::tests` 守住。
-/

/-!
### 8-14-1 常驻工具的参数面，对照 Claude Code

模型在每个请求里都看到的工具（`mode::core_tools`），参数名、合法值与描述向 Claude Code 的同类工具靠拢（D114）。对照的来源是 Claude Code 的 Agent SDK 参考里的 Tool Input Types（<https://code.claude.com/docs/en/agent-sdk/typescript.md> 与 <https://code.claude.com/docs/en/agent-sdk/python.md>，后者给每个参数一句注释）和工具参考（<https://code.claude.com/docs/en/tools-reference.md>）。

| 工具 | 本城的参数 | Claude Code 的同类 | 出入 |
|---|---|---|---|
| `edit` | `file_path`、`old_string`、`new_string`、`replace_all`（可选，缺省 `false`） | `Edit`：同四个 | `old_string: ""` 新建一个不存在的文件，Claude Code 新建走 `Write`；读后被改即拒，Claude Code 在 `old_string` 仍唯一匹配时照改（D106）；`file_path` 收城相对路径，也收本平台落在城里的绝对路径（§8-30-1），Claude Code 要绝对路径 |
| `exec` | `command` 或 `python` 恰给一个；`where`（可选，`sandbox`｜`host`） | `Bash`：`command`、`timeout`、`description`、`run_in_background` | 没有 `timeout` 与 `run_in_background`：前台窗口到了命令自己转进后台（§8-28）；没有 `description`；`python` 与 `where` 是本城的沙箱（§8-13-4，D118） |
| `read` | `path`、`offset`（0 基）、`limit`（至多 512） | `Read`：`file_path`、`offset`（行号）、`limit`、`pages`、`allow_large` | 本 Step 余下的一项：改名 `file_path`，`offset` 改成 1 基行号（§8-29-1、§8-63） |
| `search` | `text`（子串）、`path`、`context`、`mode` | `Grep`：`pattern`（ripgrep 正则）、`path`、`glob`、`output_mode`、`-i`、`context` 等；`Glob`：`pattern`、`path` | 本 Step 余下的一项：`pattern` 改成正则，`mode` 改成 `output_mode` 与它的三个值，按文件名找住在 search 里（§8-30-2，D117） |
| `status` | 无 | 无同类 | 本城的处境查询（§8-14） |
| `tool` | `op`、`namespace`、`query`、`name`、`args` | `ToolSearch`：`query`、`max_results` | `find` 收 `query`，与 `ToolSearch` 同名；`list`、`detail`、`run` 是本城的目录门（§8-60） |

**参数之外**：工具名不改（`edit`、`exec`、`read`、`search`），因为名字是账本、页面与规则里的拼写，一个模型见到 `Edit` 而叫 `edit` 的代价是零，改名的代价是每一张提到它的表。调用里多出来的键照旧不读：Claude Code 习惯给 `Bash` 带一句 `description`，拒它只换来一个回合。
-/

/-!
### 8-55 「只新建」在每一条写路径上判（`runtime::tools::edit`、`runtime::tools::exec`，形状 4 适配器）


```rust
impl EditTool {
    pub fn new(city_root: &Path, domain: Address, writable: kernel::WriteDomain,
               policy: crate::mode::PolicyReader, desk: EditDesk) -> Result<EditTool, AxError>;
}
pub struct ExecSetup { /* …既有字段… */ pub policy: crate::mode::PolicyReader }
```

- **edit 的两条臂各走 storage 的一种落盘。** 改写一个已有文件：先过写域（§8-36），再问 `kernel::gate::replacing(policy.now().write, &target)`，`Create` 下在读文件之前就拒，盘面不动；放行后经 `storage::WriteTarget::replace`（暂存文件再 `rename`，取被替换文件的权限，`crates/storage/Spec.lean` §8-32）。新建（对不存在的路径 `old_string: ""`）：经 `storage::WriteTarget::create`，名字由文件系统的「仅当不存在才建」原子地占下，已有文件（包括别的调用刚建成的）答 `E_VERSION_CONFLICT`，请先读它。新建在两种限制下都这样走，所以竞争的两次新建只成一次，不论限制是什么。
- **限制在每次写时读，不写进工具说明**：edit 与 exec 持 run 的策略格的读方（§8-62），每次写时问 `now().write`，所以 run 在中途取用的改动从下一波起就到了写门。工具说明不带写限制：说明随工具表冻结在前缀里，而写限制在一个 run 里会变；模型在 run 中途换策略后从追加的那句说明里读到它（§8-62）；一个开头就在 `Create` 下的 run，在第一次被拒时从拒词里读到，拒词说出限制与可走的路。
- **exec 在 `Create` 下只在副本里跑。** `where: host` 的 program 与 shell 直接落在人的树上，没有任何东西能让已有文件只读，所以在起进程之前就由 `gate::replacing` 拒，主语是工作目录；`sandbox` 放置照常：写入落在同步来的副本上，副本里的东西不回到树上，所以已有文件不变、命令也新建不了任何东西。python 臂的挂载恒只读，不受影响。确认不了隔离的工具不开放写入（`crates/kernel/Spec.lean` §8-78）。`where: host` 的判在 `Placement::opened_by` 里，读的是同一个策略格。
- **链接**：写目标与它到城根之间的每一级若是链接（符号链接、junction、硬链接），`WriteTarget::within` 字面拒（`crates/storage/Spec.lean` §8-25），两种限制下一样，所以经链接改旧文件这条路在 `Create` 下同样不通。
- 验收：集成测试 `crates/runtime/tests/create_limit.rs` 的 `an_existing_file_is_unchanged_under_create_by_edit_exec_and_link`：`Create` 下对一个已有文件的 edit 改写、host 上一条改它的 shell 命令、在指向它的链接名上新建，三者都拒，文件字节不变；storage 的 `two_racing_creates_admit_one`。
-/

/-!
### 8-14 runtime::tools 四件（形状 4；tools.rs 为纯索引）


```rust
// crates/runtime/src/tools/exec.rs —— 三臂（ExecArm 住 kernel::tool）
pub struct ExecTool { /* workdir、mounts、python_wasm: Option<PathBuf>、sandbox: Box<dyn Sandbox>、
                        shell: Shell（§8-13-2 D30）、fuel —— 私有；全由装配／执行器注入 */ }
impl ExecTool { pub fn new(…) -> ExecTool; }
impl Tool for ExecTool { /* meta：name=exec、effect=Write{domain}、temporal=Timestamped、render=Terminal */ }
// Program 臂：std::process::Command（workdir 钉定、环境变量白名单——secret 恒不透传）；唯一真子进程产地
//            缺省在 §8-13-2 的 confinement 副本里跑；`where: host` 才在原地（那条路是一条命令碰得到人那棵树的路）
// Python 臂：sandbox.run(python_wasm, argv=["python","-c",code], mounts)；组件缺失→E_TOOL_UNAVAILABLE＋alternative＝Program 臂
// Shell 臂：探测缺失即拒（E_TOOL_UNAVAILABLE，不是降级；楼要了 pwsh 而没有时点名 pwsh 7）；存在则 sh -c／cmd /C／pwsh -NoLogo -NoProfile -NonInteractive -Command（D30）；placement 与 Program 臂同一权威（§8-13-2）

// crates/runtime/src/tools/edit.rs —— 读戳准入（§8-64）＋写域双闸＋创建臂
pub struct EditTool { /* city_root、writable: WriteDomain、policy、desk: EditDesk —— 私有 */ }
impl Tool for EditTool { /* meta：name=edit、effect=Write{domain}、render=Diff、temporal=Timeless */ }
// new(city_root, addr, writable: WriteDomain)：writable＝该 Run 的写域（rules.write_domain()）。
// 每次调用先判路径后碰盘，三道依次：within_city（§8-30-1）把本平台的绝对路径换成城里的拼写，落在城外＝
// E_GATE_DENIED，恢复语指向 `exec`（什么算绝对路径由本平台判，D4）；Address::parse 杀穿越（..／前导
// 斜杠／空段，E_INVALID_ARGS）；WriteDomain::admits 杀域外与
// reserved prefix（E_OUTSIDE_WRITE_DOMAIN，recovery 报可写前缀清单）。工具静态声明的 Effect 只说它会写，
// 模型选的 path 要在这里判——判定住权威处，而不是 bench 里的第二份判定。
// args：{path, old, new}；准入按本 run 的读戳（§8-64，D106）：没读过＝E_INVALID_ARGS，读后被改＝E_VERSION_CONFLICT 带当前片段；
// old 必唯一命中（零命中／多命中＝E_INVALID_ARGS 携计数）；回显＝unified diff（逐次 diff 即回档粒度）

// crates/runtime/src/tools/read.rs —— 一个参数，两条路
pub struct ReadTool { /* city_root、catalog: Arc<Mutex<Catalog>>、bound、block_store —— 私有 */ }
impl ReadTool { pub fn new(city_root: &Path, catalog: Arc<Mutex<Catalog>>, bound: ReadBound, block_store: &Path)
    -> Result<ReadTool, AxError>; }   // bound 见 §8-29-1，block_store 见 §8-29-5
impl Tool for ReadTool { /* meta：name=read、effect=Read、cost=Light、render=Generic、temporal=Timeless */ }
// args：{path}。先问 catalog，再当作地址。
// 创建臂：old=="" → 文件必不存在（存在＝E_VERSION_CONFLICT，请先读），new＝全文；父目录自动建（域内已证）。
// 路径不存在而 old 非空的拒词指向创建形；缺参拒词报三字段契约。理由：没有创建能力的城里，Agent 在空房间里无法开始任何工作；
// 创建住 edit 而非新工具，因为「文件变更＋准入」已是本工具拥有的唯一权威，「不存在」只是准入的一个取值。

// crates/runtime/src/tools/status.rs —— 十三字段
pub struct StatusSnapshot { pub who: String, pub addr: Address, pub policy: RunPolicy,
    pub ctx_limit: Tokens, pub trust: String,
    pub write_domain: String, pub locks: Vec<String>, pub worktree: PathBuf,
    pub signals_pending: u32,
    pub provider_mode: ProviderMode, pub neighbours: u32 }   // neighbours 在末尾，渲染序与声明序同一
pub enum ProviderMode { Normal, Degraded, LocalOnly }
pub struct ChildStatus { pub room: Address, pub kind: DelegateKind }
pub struct StatusTool { /* snapshot＋ children: Box<dyn Fn() -> Vec<ChildStatus> + Send> ＋ backlog: Option<Backlog> */ }
impl StatusTool {
    pub fn watching(snapshot: StatusSnapshot, children: Box<dyn Fn() -> Vec<ChildStatus> + Send>) -> Result<StatusTool, AxError>;
    pub fn reporting(self, backlog: Backlog) -> StatusTool;   // §8-28-2：末行 `backlog:` 从表里现读，与 children 同一理由
    pub fn metering(self, context: ContextReading) -> StatusTool;   // `ctx:` 行的用量从运行的读数现读
    pub fn clocked(self, clock: ClockReading) -> StatusTool;        // `now:` 行从驱动最近的读数现读（§8-53）
}
impl Tool for StatusTool { /* meta：name=status、effect=Read、temporal=Timestamped、render=Generic；渲染序末尾追加 backlog 一行 */ }

// ToolBench 住 runtime::bench（§8-3）：按 Effect 过门是回合层的次序，工具本身以 Box<dyn Tool> 递入。

- **`children` 只携地址与代理类别**：子 Run 在父嚽结之后才开，故父自己那一跑里 **子既无 run id 也无上下文读数**——四个字段里三个只能填零，而零与未知是两件事。现形状只携得出口的两件：派到哪个房间、哪一类代理。
- **`worktree` 的大小为何现读**：快照里只有树的路径，大小在每次调用时走一遍树算出：每个普通文件的长度相加，`.sprawling`（`kernel::RESERVED_PREFIX`）下城自己的状态不算，链接既不跟也不计，所以一条指回上层的链接走不成环。冻结在派发时的大小在本跑第一次写之后就错了，而派发时一律填零（测试城里 `status` 答 `0 bytes`）正是这个形状留下的缺陷。读不出来时这一行答 `size unreadable` 与原因，而不是一个看起来像真的零。目录名按字节比较，三个平台一样：Windows 与 macOS 的默认卷不分大小写，一个拼成 `.SPRAWLING` 的目录在那里与 `.sprawling` 是同一个，会被计入，这一偏差只多算、不少算。
- **`ctx` 的用量为何现读**：快照在派发时冻结，那时还没有任何一次调用，冻结的用量只能是零，而且整跑都是零——一个照 City.md 去问 `status` 的模型会被告知窗口是空的。用量住 `ContextReading`：Run 每回合把 provider 报的 `input_tokens` 写进去，`status` 被调用时读出，所以报的是本跑最近一次已完成调用的计数。上限 `ctx_limit` 仍在快照里，因为它整跑不变。
- **`neighbours` 追加在末尾而不插入到 `signals_pending` 旁边**：冻结序存在的理由是字段表增长时居民的习惯仍可迁移，而一次插入会把前十二行里的一半挪位。它只报**人数**不报名单：名单长度随人口增长，而 `status` 是一份定长文本（`render_children` 已为同一条理由被压成一行）；详情归 `neighbours` 工具，`crates/city/Spec.lean` §8-15b。
- **数的是人，不是地址**：一间没人站着的房间没有读者，把它计入会让 `neighbours: 3` 读起来像「有三个人可以说话」而实际上一个都没有。空房间仍然在工具的答案里，因为它对 delegate 与搬入是真信息。
- **`children` 是闭包而不是快照字段**：派活发生在 `status` 工具造好之后，一份开跑前拍的快照永远是空的。派生台住 `collab`，而 depmap 不允许 runtime 依赖 collab，故本模块只收一个答「现在派了哪些」的闭包，装配层把台接上去——与 `RunHooks` 四个闭包同一纪律：第二实现不存在时不引 trait。
// runtime::compaction（形状 6 数据面＋形状 1 判定）
pub enum Content { Prose, Code, Diff, Log, Structured, Table, Markup, Unknown }   // 八类，Unknown 与 Markup 各是其中之一
pub enum Strategy { Head, Ends, Tail, Sections }
pub enum Shrink { Keep, Cut(Strategy), MustOffload }
pub fn detect(text: &str) -> Content;                       // 前几行上的前缀与计数，顺序即设计
pub fn plan(content: Content, size: ByteLen, budget: ByteLen) -> Shrink;
pub fn shorten(text: &str, strategy: Strategy, budget: ByteLen) -> elision::Cut;   // 标记与丢弃计数由 elision 一处产出（§8-42）
// 硬不变量：结果恒不大于输入，且在出口再验一次（真长了就退回原文）。切口落在字符边界。
// Structured 与 Unknown 恒不截断：被截断的 JSON 比缺席的 JSON 更糟；未知内容不拿猜测去丢东西。
// Markup 由以下特征判定： `\documentclass`／`\begin{document}`，或首几行里两条 ATX 标题且无一行像代码
// （源码文件的 `# ` 注释与标题同形，否则会保注释而丢代码）。检测先于 Table：LaTeX 的表格会让一行带 `|`。
// `Sections` 保整节至预算尽，再保被丢各节的标题行（骨架）；少于两条标题即只有一个标题，退回首端裁。
// 机制面（把大结果移出窗口）仍归 offload——本模块只答「缩不缩、留哪一头」。

// runtime::mode 的准入面（形状 1 判定）
pub struct Produced { pub tests_passed: Option<bool>, pub contract_moved: bool,
                      pub held_in: Option<bool>, pub held_out: Option<bool> }
pub enum Admission { Lands, Refused { because: &'static str, alternative: &'static str } }
pub fn admits(mode: Mode, produced: &Produced) -> Admission;
// `None` 不是 `Some(false)`：「没测」与「测了没过」是两件事。证据以 bool 入参而非 eval 的类型，
// 因为 eval 在本 crate 之外，而这里问的不是证据怎么来的，是够不够。UD 是唯一要双验证的模式。

// runtime::redact（形状 1 判定）——账本侧入口只有一个：`turn::ledger::Journal::append_redacted`，
// 它管 model_returned、tool_called、tool_result 三类载荷（§8-41）。
// 窗口块已在此前取出，故思考块签名不受影响；历史与上下文是两个汇，只有一个是永久的。
// 替换物是 `secret:redacted/<b3-16>` 标记而非 Vault 条目：模型复述的钥匙不是城被托付保管的凭证，
// 存它等于给它一条没人要求过的命，而哈希前十六位已足以看出两处是否同一个值。
pub enum Marker { Plain, Fingerprinted }         // 两个汇只差这一个参数，不差第二份实现
impl Marker { pub fn spell(self, found: &[u8]) -> String; }
pub fn redact(payload: Map<String, Value>) -> (Map<String, Value>, u32);
// D37：`redact` 接收并交回载荷的所有权，原地改写命中的字符串。零命中的载荷原样交回：同一块分配、
// 逐字节相同，扫描之外不付拷贝；有命中时只替换命中的那些 `String`，其余值留在原处、不复制。
// 两个调用者把载荷交进来：`turn::ledger::Journal::append_redacted` 经 `kernel::Payload::into_map`，
// `transcript` 交出它刚编码出的 map。落选：借用入参、零命中交回 `Cow::Borrowed`——有命中时仍要克隆
// 未命中的兄弟分支，而两个调用者交进来之后都不再读原载荷，借用换不来任何东西。纯计算，Windows、
// macOS、Linux 上同一行为、同一组测试。
pub fn redact_text(text: &str, marker: Marker) -> (String, u32);
// 「什么绝不可被打印」在本 crate 只有这一个家：账本走 `Fingerprinted`，诊断行走 `Plain`。
// 历史被检索与比对，故标记要能分辨两个值；一行日志写一次读一次，哈希后缀在那里只是一个
// 没人关联的关联句柄。标记恒可由 `kernel::SecretRef::parse` 解析，realm 恒为 `redacted`，
// 而金库里没有这个 realm——顺着标记去兑的读者得到的是一句诚实的「这里没有」。
pub fn fingerprint(found: &[u8]) -> String;      // b3 前十六位

pub struct ToolBench { /* tools: BTreeMap<ToolName, Box<dyn Tool>>、domain: WriteDomain、
                          taint: TaintSet、sandbox: kernel::SandboxLimits、seen: BTreeMap<IdemKey, Result<ToolOutcome, AxError>>、
                          prior_public_egress: bool、egress: Option<EgressAllowlist>（楼写下的域名表，`None` 即楼没写，city D27）—— 私有。`seen` 是「这把键答过没有、答了什么」的唯一一张表：键只在工具真正答过之后才写入，门拒/语法拒不留条目，故重试不算重放。 */ }
impl ToolBench {
    pub fn new(domain: WriteDomain) -> ToolBench;
    pub fn register(&mut self, tool: Box<dyn Tool>) -> Result<(), AxError>;
    /// Gate routing by declared Effect（收进回合层，executor 归还薄形）：
    /// exec 先 forecast（Suspected → **强制 checkpoint 先行**，见下）；Write → domain 门；
    /// Egress → 先按楼的域名表判主机（楼写了表时），再过 egress 门（subject 由工具从自己的参数读出）；
    /// AttachUserBrowser → attach 门，页面目标同样先按域名表判，再过 egress 门；Spend → 门已接，本构建无声明它的工具；
    /// Spawn → 无门（派生深度是类型，`gate::spawn` 在派活处判）；Govern → 恒拒（run 不改写评判它的那些规则，
    /// 规则在 CONFIG.toml 与楼的 RULES.toml 里由人改）；
    /// Deny 与门的提问（E_APPROVAL_PENDING）都以 `Refused` 作 tool_result 回流，不吞掉回合。
    pub fn invoke(&mut self, call: &ToolCall, key: &IdemKey, now: TimeMs)
        -> Result<BenchOutcome, AxError>;
    // invoke 是下面三段按序串起来的一条调用，三段各自公开，供并行的工具波分开用：
    /// 串行的放行：去重、各道门、exec 的 forecast checkpoint。答得出的（重放、门拒、门问）当场答，
    /// 否则交出放行单 `Ticket`（键、工具名、效果、checkpoint 的 oid —— 私有）。
    pub fn clear(&mut self, call: &ToolCall, key: &IdemKey, now: TimeMs) -> Result<Clearance, AxError>;
    pub enum Clearance { Answered(BenchOutcome), Cleared(Ticket) }
    /// 可并行的执行：交出放行单指向的工具本身，调用方在任意线程上调它的 `invoke`，
    /// 不碰 `seen` 与 `taint`。交出工具而不交出 bench：bench 不是 `Sync`（checkpoint 持有
    /// git 仓库句柄），工具是。
    pub fn tool_for(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError>;
    /// 串行的记账：成功的答案先并入 taint，再连同失败一起记进 `seen`，返回 `Ran`。
    /// 并行波按调用序逐条调它，所以 `seen` 与 `taint` 的写入次序与串行波相同。
    pub fn account(&mut self, ticket: Ticket, answered: Result<ToolOutcome, AxError>)
        -> Result<BenchOutcome, AxError>;
    // 路由：名字→处理器住 `Bench` 的 `tools: BTreeMap<ToolName, Box<dyn Tool>>`，
    // 每次调用经 `tool_named` **一次**探测即得处理器（meta/subject/invoke 同一把借用）；
    // 被否：一次调用查两次（先查 meta 再查处理器）——每次探测都要再比一遍名字。
    // 上面那句「按 Effect 过门」自己的名字住 `Doors`（domain/taint/sandbox/prior 四件同行值，
    // 自拥门判与失败形）：`None` 即此门已开，工具可跑；`Some` 即门已代这次调用给出答案。
    // 与 route 的同一把借用不相斥（字段级不相交借用），故一次探测服务整条链。
    fn admit(&mut self, call: &ToolCall, name: &str, effect: &Effect, subject: &GateSubject)
        -> Result<Option<BenchOutcome>, AxError>;
    /// 一扇门的判定对本 bench 意味着什么：Allow 放行，Deny 与 Ask 都是 `Refused`。
    fn settled(&self, outcome: GateOutcome) -> Option<BenchOutcome>;
    /// 同形：两处出网判定的「首次公开出网要记下来」只住这里。
    fn crossed(&mut self, outcome: EgressOutcome) -> Option<BenchOutcome>;
    /// 包信封的调用者要读 `temporal` 才知道时钟行该不该发。
    pub fn meta_of(&self, name: &ToolName) -> Option<&ToolMeta>;
    /// 检查点署名随网一起交给 bench。一个没有 `Provenance` 的检查点
    /// 写不出 `Sprawling-Run:`，而预测检查点恰恰是在一个拿不到运行上下文的
    /// 闭包里升起的，所以它在装配时就被交下。
    pub fn with_checkpoint(self, net: CheckpointNet) -> ToolBench;
    /// 检查点的三件东西恒同行：仓、它盖住的范围、写它的人。
    pub struct CheckpointNet { pub checkpoint: Checkpoint, pub scope: Vec<String>, pub of: Provenance }
    // `scope` 是这次 run 的**写域全部前缀**，不是它的房间：检查点窄于写域，
    // 两者之间写下的文件就进不了任何检查点（`crates/storage/Spec.lean` §8-18）。
    /// 本 bench 服务的那份活。Spawn 门要铸一个人答得出的条目，
    /// 条目要有 actor（问谁）与 artifact（看什么）；两者都不在一次工具调用里。
    /// 未给即拒（fail-closed）——一个人问不到的派生就是没人批准的派生。
    pub fn for_job(self, asking: Address, job: Locator) -> ToolBench;
    /// 楼写下的出网域名表，随 run 冻结。只在楼写了表时调用；没调用的 bench 不按主机拒任何调用
    /// （city D27）。判定是对这张表的内存比对，不读盘、不解析域名，所以这道门不给调用加往返。
    /// 表外的主机以 `kernel::gate::egress_target` 的 `E_GATE_DENIED` 作 tool_result 回流，
    /// 排在密钥扫描之前：一个不该去的主机，不必再问它的字节能不能出去。
    pub fn limit_egress(self, list: EgressAllowlist) -> ToolBench;
}
```

- L0 三件恒列 prefix（City.md 只放这一级）；catalog 只收 L2——L0 不进 catalog（名字即文档）但 tool_defs 恒含三件（wire 面要 schema）。
- **否决「Suspected → Discard 门」**：它与 kernel 既有设计冲突，以 kernel 为准。理由：`DiscardRequest` 只有 `Planned`／`Unplanned` 两变体，而 `decide` 对 `Unplanned` **恒判 Deny(NoRestoration)**——把 forecast 的预判包成 Unplanned 送进门，等于让任何含 `rm ` 的 exec 调用全被拒。`kernel::discard` 的注释早已写明正确意图：「text prediction is obfuscatable by design — hits route conservatively, and the git checkpoint net (S3) is the honest backstop」。故 **Suspected 不拒而先立检查点**：强制 `checkpoint.wave_pre` 先行再放行，删掉的东西因而可回档；**无 checkpoint 网时才拒**（`E_TOOL_UNAVAILABLE`），因为「无保护地跑」是唯一没人选择的结局。此路由使 A14 的先行半链在 exec 臂上机械成立。
- ToolBench 持 `Option<Checkpoint>` 具体类型而非新 trait：checkpoint 只有一个实现，为尚不存在的第二实现引缝会造空抽象（AGENTS.md：trait 只在已有第二实现的缝上引入）。
- **`BenchOutcome` 穷尽**：它是判定输出，下游必须穷尽匹配三臂——新增一种答案而不回答它就不编译（§7）；本 crate 的枚举全部如此，没有 `#[non_exhaustive]`，也没有通配臂。
- **三条规则各有一处**：`GateOutcome` 对本 bench 意味着什么只住 `settled`（Allow 放行、Deny 与 Ask 都以 `Refused` 回流），「首次公开出网要记下来」只住 `crossed`，参数的秘密扫描只序列化一次；`admit` 是那个 `match effect` 自己的名字。一份规则在几处各有一份实现，就是几个可以各自漂走的权威。
- `BenchOutcome` 三态：`Ran{outcome, checkpointed}`（checkpointed 携检查点 oid，供波后补记）／`Refused{refusal}`（回流不终止回合；门的提问也走这一臂）／`Duplicate{outcome}`。dedup 先于任何副作用；**key 在工具答过之后才记入 `seen`**，故被门拒的调用重试不算重放。**判重是 `seen` 上的一次 O(log n) 查找**，不抄键、不另立一张「领过权」的集合：第二张集合记的是 `seen` 键集的同一个事实，两份拷贝迟早分叉。工具按 `ToolName` 登记，`invoke` 以调用自带的名字查表，路由一次调用不分配。
- status 的 result 是**按冻结序渲染的文本**而非 JSON 对象：`serde_json::Map` 对键排序，JSON 对象没有读者可依赖的序，「冻结序」会悄悄变成字母序。序是「模型读到的东西」的属性，故落在模型读到的地方。
- 声明 `Egress` 的生产工具是浏览器工具（`bin::browser_tool`）；声明 `Spend` 的工具本构建没有，那扇门以测试替身驱动。
-/

/-!
### 8-22 runtime::tools::edit 目录化


| 文件 | 管什么 |
|---|---|
| `crates/runtime/src/tools/edit.rs` | `EditTool` 与 `EditDesk`：`new` 的参数模式声明、`Tool::invoke` 的判定次序（工具身份→地址→写域→锁内准入（§8-64）→匹配数→落盘→记新戳），创建臂 `create`，以及 `unified_diff`／`common_prefix`／`common_suffix` |
| `crates/runtime/src/tools/edit/passage.rs` | 「读后被改」拒绝里的当前片段：`old_string` 首行附近的几行，带 0 基行号 |
| `crates/runtime/src/tools/edit/tests.rs` | edit 拒绝什么、回显什么：读过之后的落盘与 diff、没读过、读后被改带片段、两个 run 争一个文件、创建臂两种冲突、写域外、城外的本平台绝对路径（`E_GATE_DENIED`，盘上不留文件）与非法地址、缺文件的恢复话术、零次与多次匹配、错路由 |
-/

/-!
### 8-36 写域的两道闸各问一个问题（`crates/kernel/Spec.lean` §8-46 末段）


- **`bench::admit`** 对 `Effect::Write { domain: area }` 改调 `kernel::reach(&self.domain, area, &self.taint)`：工具声明的是一块区域，门口只问这块区域够不够得到。
- **`tools::edit::invoke`** 解析出 `target` 后调 `kernel::domain(&self.writable, &target, &TaintSet::empty())`：`Allow` 继续，`Deny { refusal }` 原样作 `Err`（三段式因此由 kernel 一处产出，工具不自拼 `Outside` 的话术），`Escalate` 在写域门上不可能出现——`GateOutcome` 刻意穷尽，这一臂如实答一条 `E_INVALID_ARGS` 说明该不变量，而不是 `unreachable!`。空 `TaintSet`：taint 是 bench 的事实，工具这一层没有它，拒词因此少一句「派生自 N 个外部来源」——那句话仍由门口那道 `reach` 说。
- **验收**：`crates/runtime/src/tools/edit/tests.rs` 钉住「Documents 域的工具创建 `<city>/hall/note.md` 成功、创建 `<city>/hall/note.rs` 被拒（`E_OUTSIDE_WRITE_DOMAIN`，主语是文件）」；`bench/tests.rs` 钉住「Documents 域、声明区域为 `hall/mayor` 的 `Write` 效果在门口放行」。
-/

/-! D114 常驻工具的参数名与合法值取 Claude Code 的拼写，出入只留本城的特性

**决定**：§8-14-1 那张表里的每件常驻工具，参数名、合法值与描述取 Claude Code 同类工具的拼写；表里「出入」一列是全部有意留下的不同，每一条带着本城的理由。工具名不改。

**理由**：没有自家 harness 的模型在它们见得最多的形状下表现最好，而 Claude Code 的工具形状是其中最常见的。参数名对上，模型就不必从描述里学一个新词，也不会把习惯的名字传进来、被拒一次再改。留下的出入都是本城的合约比 Claude Code 更严或更宽的地方：读过才能改（D106）、`old_string: ""` 代替一件常驻的 `write`、python 沙箱与放置、`status` 与 `tool`。

**被否**：①工具名也改成 Claude Code 的（`Edit`、`Bash`……）——名字是账本、页面与规则里的拼写，改名的代价落在每一张提到它的表上，而模型叫一个小写的名字没有困难；②同时收新旧两套参数名——同一个事实两种拼写，两份一起维护，读账本的页面也要认两套；③向 Codex 的 `apply_patch` 与 `exec_command` 对齐——Codex 以补丁格式改文件，没有专门的读与搜，和「读过才能改」的合约对不上，Claude Code 的形状与本城的工具面最接近。

**重开参数**：Claude Code 的参考改了这些参数名或合法值；或者某个 provider 的模型在读数里系统性地传另一套名字。
-/

/-! D115 旧账本里的调用原样进入重建的对话，旧参数名不映射成新名

**决定**：一个带着另一段对话开的会话（`OpenSession` 带 `from`：分叉，或把一段对话带到另一个房间接着做）经 `fork::inherited_indexed` 从 `model_returned` 记下的消息重建对话，消息里每个 `tool_use` 的参数逐字节照账本发给 provider；同一个房间里接着派的活开一个新 run，不重放上一个 run 的调用。旧版写下的 `{path, old, new}`、`{arm: …}` 照旧出现在历史里，它们的结果照记下的渲染器版本画（D107）。新版的工具收到用旧名字的调用时拒绝，拒词说出现在的参数名。

**理由**：一次调用与它的结果是一问一答，结果照当时的版本画（D107）；把调用改成新名而结果不改，模型读到的就是对不上的一对。映射表还会是参数名的第二份权威，每改一次名字它就多一行，而且永久留着。改名之后工具表本身变了，provider 的前缀缓存从工具表起失效，映射也省不回这次缓存。推断：provider 不按当前的工具表校验历史里的 `tool_use`（Anthropic 与 OpenAI 的接口文档只对新的调用说校验），所以原样发出不会被拒；`runtime::fork::tests` 的 `an_old_call_is_rebuilt_as_it_was_recorded` 钉住重建的字节。模型照着历史用旧名字时，代价是一次被拒的调用。

**被否**：①重建时把旧名映射成新名——上面的理由；②新版工具同时收旧名字——D114 ②；③给 `tool_called` 记一个形状版本，只在版本不同时映射——同①，再多一把账本键。

**重开参数**：读数显示升级后的会话里模型反复用旧名字调用，被拒之后仍不改；或某个 provider 开始按当前工具表校验历史里的 `tool_use`。
-/

/-! D116 参数改名不改变任何已派生的 `IdemKey`，不做迁移

**决定**：外向动作的键只在一次活的调用被准入时派生：`accounting::worker::driving::placing` 的 `Placing::admit` 从模型这次送来的调用取 `ToolCall::action`，与 run 和这次调用在 run 里的位置一起交给 `IdemKey::derive`（`crates/kernel/spec/Idem.lean` §8-6）。键只进这个 run 的 `ToolBench.seen`，在内存里，不进账本。恢复不重新准入记下的调用：启动扫描对没有结局的调用写一行 `E_TOOL_OUTCOME_UNKNOWN`（`runtime::replay::outcome_unknown_draft`），分叉与续接开一个新的 RunId，新调用的键从新 run 起算。所以新版里没有一处为旧调用重新派生键；改名改的只是新调用的规范化字节，不需要迁移。

**证据**：`accounting::worker::genesis::tests` 的 `an_old_shaped_dangling_call_is_closed_and_never_run`：账本末尾一条旧形状、没有结局的 edit 调用，启动扫描把它关成 `E_TOOL_OUTCOME_UNKNOWN`，文件一个字节不动。

**被否**：①派生前先把参数归一成新名——没有读者需要新旧两种拼写得到同一把键：同一把键防的是同一个 run 里同一位置的重放，而那里只有一种拼写；②提高 `IDEM_DERIVE_V`——键的框架没变，变的只是动作字节，而动作字节本来就随参数变。

**重开参数**：键开始落盘（例如为了跨进程续跑同一个 run 把 `seen` 写进账本），或恢复开始重新执行记下的调用。
-/
