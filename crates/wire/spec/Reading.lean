-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reading

规定 `reading`、`answer::rounds`、`answer::evidence`、`answer::cost_of`（`crates/wire/src/` 下同名的文件）。一条账本载荷读成线上的值，给两端共用：回合、证据与一个计划节点的花费。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-21 四种读法回到服务端

```rust
// Query 第 18、19、20 条（声明序，QUERY_NAMES 同序追加）
Rounds   { run: RunId },      // → Answer::Rounds(Box<RoundsAnswer>)
Evidence { run: RunId },      // → Answer::Evidence(EvidenceAnswer)
CostOf   { node: NodeId },    // → Answer::CostOf(CostOfAnswer)

pub struct RoundsAnswer {
    pub run: RunId,
    pub turns: Vec<Turn>,
    pub opened_at: Option<GitOid>,   // 本会话的第一个检查点
}
pub struct Turn {
    pub number: u32, pub opened: Seq,
    pub said: Option<String>, pub spent: Option<UsdMicros>,
    pub used: Option<Used>, pub stopped: Option<String>,
    pub calls: Vec<Call>, pub notes: Vec<Note>,
}
pub struct Call { pub tool: String, pub subject: Option<String>,
                  pub outcome: Outcome, pub at: Seq, pub output: Option<Output> }
pub enum Outcome { Waiting, Answered, Failed }
pub struct Output { pub head: String, pub cut: usize }
pub struct Used { pub input: Tokens, pub output: Tokens, #[serde(default)] pub cached: Option<Tokens> }  // D35
pub enum Note { Refused { error: AxError, at: Seq }, Checkpointed { oid: GitOid, at: Seq },
                Waiting { at: Seq }, Arrived { from: String, said: String, at: Seq },
                Discarded { count: usize, at: Seq }, Unreadable { cause: String, at: Seq } }

pub struct EvidenceAnswer { pub run: RunId, pub items: Vec<EvidenceItem> }
pub struct EvidenceItem { pub at: Seq, pub kind: EvidenceKind,
                          pub locator: Locator, pub picture: Option<Picture> }
pub enum EvidenceKind { Screenshot, Finished }
pub struct Picture { pub media_type: String, pub width: u32, pub height: u32 }

pub struct CostOfAnswer { pub node: NodeId, pub spent: UsdMicros,
                          pub runs: Vec<(RunId, UsdMicros)> }
```

**这里搬的是读法而不是接口。** 一个会话被读成回合、一次跑留下什么证据、一个计划节点花了多少钱——这三件事此前只有 `crates/web` 会算，于是「线就是全部 API」（ARCHITECTURE §8）在这三处是假的：另写一个客户端就得把折叠逻辑照抄一遍，而照抄出来的那一份迟早与这一份不一致。现在三者各是一次查询，答由 `accounting::views` 折出（`crates/sprawling/Spec.lean` §8-47）。

- **`Rounds` 的值类型住 `wire`，折叠住 `accounting::views`。** 值要上线，故必须可序列化；折叠要读账本，故必须在能读账本的那一层。两者切分开来，正是 ARCHITECTURE §9 的形状 2 与形状 7 的分界。
- **`wire::reading` 是第三块**：把一条账本载荷读成上面这些值的那些纯函数（`said_in`／`used_in`／`output_in`／`note_of`）。它住在线这一层而不是服务端，因为**两端都要读**：服务端答 `Rounds` 要它，客户端把推来的 `model_returned` 折进自己的快照也要它（ARCHITECTURE §5 第 12 步：同一个折叠，线的两边）。一份权威，两个调用者。**`Call.subject` 不由这里算**：写方在写 `tool_called` 时把它定下（`crates/kernel/Spec.lean` §8-4：`ToolCalled::subject_of`），折叠读记录里的 `subject` 键；两个读方各按自己的 map 序推一次，同一次调用已经出现过两个名字。
- **读不出的载荷是一条 `Note::Unreadable { cause, at }`，不是没有 note**：`note_of` 认下的种类（被拒、检查点）若载荷读不回它该有的形状，答里留一行，`cause` 说哪一种事件、读到哪一步失败，`at` 指向账本里那条记录。被否：返回 `None`——那样一次被拒在人眼里就是「什么都没发生」，而失败本身被这一层抹掉了。`CheckpointCommitted` 的 `JobPinned` 是一个真答案（派发钉住的是作业不是提交），仍然没有 note。
- **`Changes` 早已在线上**（§8-20），`storage::changes` 一直是它唯一的权威；查过之后不动它——把一件已经做完的事再做一遍就是造第二个权威。
- **`Evidence` 只认写下来的东西**：截图是 `tool_result` 载荷里的 `image` 定位符（`bin::browser_tool::storing::stored` 写的那三项：定位符、两条边、media type），完成证据是 `roadmap_finished` 载荷里的 `evidence` 定位符。**答里恒不携字节**：一张图是一个 `cas:` 定位符，取它是资产端点的事，把 base64 塞进查询答会让「看一眼这次跑干了什么」付上整批像素的代价——与 §8-20 拒绝整批补丁同一条理由。
- **`CostOf` 的分母不在这里**：答只报这个节点上归到的绝对金额与逐跑明细，不报占比。占比需要一个这一端没有的分母（整城总额是 `CostView` 的），而没有分母的百分比正是 `UnplannedProgress` 拒绝拼出来的那种东西。节点到跑的映射由 `roadmap_claimed` 折出（载荷里的 `node` 与记录的 `addr`），钱由 `storage::attribution` 的 `by_run` 给——**不新增任何计价处**。
- **一个本城没认领过的节点答 `CostOf { spent: 0, runs: [] }` 而不是 `Unavailable`**：与 `Changes` 那一条相反，因为这里「没人认领过它」是一个真答案而不是「我读不了」；节点地址本身经 `NodeId` 的手写 `Deserialize` 把过关，读不了的形状根本上不了线。
- **`RunCosts { runs: Vec<RunId> }` → `Answer::RunCosts(RunCostsAnswer { runs: Vec<(RunId, UsdMicros)> })`**：`CostView.by_run` 之外的跑逐个按名字问，一次最多 `RUN_COSTS_MAX`（64）个，按问的顺序答；读不出的跑不出行，零是「没花钱」。钱仍只由 `storage::attribution` 折，冷的一侧从账本折（`crates/sprawling/Spec.lean` §8-90）。
- **`WIRE_V` 15→16，一次进位管三条查询**：名字表长了三项（17→20），故 schema 哈希无论如何都要变。旧页面在握手期被明确拒绝，这正是该机制存在的理由。
- **被否**：（a）把 `Rounds` 并进 `RunHistory` 的答——前者是折叠后的读法，后者是原始记录页，一个答两副形状会让翻页与折叠互相牵制；（b）让 `Evidence` 直接回字节——见上一条；（c）把折叠留在 `wire` 里由客户端调用——那样新客户端仍要自己跑一遍折叠，而这一节整件事就是不要它这么做。
-/

/-!
### 8-47b 一次工具调用带上它的起止时刻，一个回合带上它问的模型与它开始等人的时刻

```rust
pub struct Call {
    // …既有字段…
    pub called: TimeMs,            // tool_called 那条记录的 t
    pub answered: Option<TimeMs>,  // 配对上的 tool_result 那条记录的 t；未答为 None
}
pub struct Turn {
    // …既有字段…
    pub model: Option<String>,     // 开这个回合的 model_called 记下的 model
}
pub enum Note {
    // …
    Waiting { at: Seq, t: TimeMs, answered: Option<TimeMs> },
    // t：approval_requested 那条记录的 t；answered：按 approval id 配上的 approval_resolved 的 t
}
```

- **时刻读自账本记录，不读此处的时钟**：与 `Turn.t` 同理，重放的会话报它当初的时刻。`called` 不是 `Option`：一次调用由一条 `EventRecord` 折出，它总带读数。`answered` 与 `outcome` 同时写、同一次配对——`outcome` 为 `Waiting` 时它必为 `None`，窗口外答的调用也是 `None`，不猜。
- **为什么要上线**：run 页的时间透镜原本只能按回合着色，一个回合里模型说话与工具运行各占多久，线上没有数；有了这两个时刻，透镜画的是量出来的段，而不是按回合结局推断的整段。
- **被否：只带一个时长**。时长丢了起点，页面画不出调用在时间轴上的位置，也就排不出并发的两次调用。
- **模型名挂在回合上，不挂在答案上**：`model_called` 每问一次记一次 `model`，一次 run 中途换模型（降级、换端点）时，逐回合的名字才是账本写下的事实；run 页统计栏的「模型」格取最后一个回合的名字，前后不同时列出各个名字。读法与 `Call.subject` 同：取那一键的文本，读不出为 `None`，页面不画名字而不猜。
- **被否：`RoundsAnswer.model` 一个字段**。那得在折叠里挑一个回合的名字当整次 run 的名字，换过模型的 run 上它说错一半。
- **等人从哪一刻开始，读自请求记录**：`Note::Waiting.t` 是 `approval_requested` 那条记录的 `t`，与 `Call.called` 同理不是 `Option`。等到哪一刻结束是 `answered`：`approval_resolved` 记在城自己的 run 下，服务端按 approval id 把它配回请求（`crates/sprawling/Spec.lean` §8-50-1），配不上为 `None`，不猜。
-/

/-!
### 8-48 一次 run 的开头带上由谁派来

```rust
pub struct Opening {
    // …既有字段…
    pub dispatched_by: Option<Who>,  // run_started 的 dispatched_by；缺键为 None
}
```

- **派活者写在 `run_started` 的载荷里，不读那条记录的作者**：`run_started` 的作者恒为 `city`——是城的派活台写下这一行——所以作者说不出这次 run 是人派的、城按日程与计划派的，还是一个居民委派、接替或敲门派的。派活处各自知道答案：人下的 `Dispatch` 写 `person`；计划节点、日程、外来到达与人刚放行的活写 `city`；委派写委派者的地址，接替写前任的地址，敲门叫醒写敲门者的地址。`runtime::RunPlan.dispatched_by` 把它从派活处带到 `run_started`，线上的 `Opening` 原样转述。
- **旧账本里没有这个键**，读作 `None`，页面不画「由谁派来」而不猜。
- **被否：从 `parent`／`predecessor` 推断**。那两个键只说明委派与接替，人派的与城派的在账本里长得一样，推断在最常见的两种派活上答不出来。
-/

/-!
### 8-53 一个回合带出首个内容几时到，每个时刻带出它是不是量出来的；检查点的 note 叫 `checkpointed`

```rust
pub struct Turn {
    // …既有字段…
    pub first_at: Option<TimeMs>,   // 开这个回合的回复记下的 first_at（`crates/kernel/Spec.lean` §8-75）；缺席即没量到
    pub timing: Timing,             // `t` 是不是 model_called 自己那一刻
}
pub struct Call {
    // …既有字段…
    pub timing: Timing,             // called 与 answered（在场时）是不是各自那一刻
}
#[serde(rename_all = "snake_case")]
pub enum Timing { Measured, Unmeasured }
pub enum Note {
    // …
    Checkpointed { oid: GitOid, at: Seq },   // 线上 "checkpointed"
}
```

- **`first_at` 照录那一行的键。** 回合里最后一条 `model_returned` 写下的 `first_at`，读不出或缺席即 `None`。首字耗时是 `first_at − t`，线上不另带一个时长：页面手里已有这两个数。
- **`Timing` 答一个问题：两个时刻之差是不是一次测量。** `Measured`：这一行上的每个时刻都是它自己那条记录量下的那一刻（`EventRecord::moment` 答 `Some`）。`Unmeasured`：至少一个不是——账本版本 1 写下的行带的是回合时间戳，同一回合的行同值；或者答复是重启之后城补上的 `E_TOOL_OUTCOME_UNKNOWN`，它记的是城补上它的那一刻（`crates/kernel/Spec.lean` §8-4「信封 `t` 记的是什么」）。时刻本身照旧带出，它仍给出次序；页面不从 `Unmeasured` 的行画用时。
- **`Turn.timing` 只说 `t`**：回合在线上只有这一个时刻；`first_at` 在场即量过，缺席即没有。`Call.timing` 说 `called` 与 `answered` 两个：一次调用的两条记录由同一个构建写下时两者同为量过或同为未量，城补上的答复例外，所以一个值够用。
- **`Checkpointed`**：fence 与 checkpoint 曾是一个概念的两个名字，checkpoint 留下（glossary）。`Note` 不进名字表，改它的标签不动 schema 哈希，所以它随本节的进位落地。
- **进位**：本节的提交是 D1 意义上上一次推送之后第一个名字不变而改形的提交，`WIRE_V` 44 → 45；§8-53 至 §8-58 共用 45。
-/

/-!
### 8-55 一次调用带出它的效果与呈现

```rust
pub struct Call {
    // …既有字段…
    pub effect: Option<kernel::Effect>,        // tool_called 记下的登记（`crates/kernel/Spec.lean` §8-75(b)）
    pub render: Option<kernel::RenderIntent>,  // 同上：Generic、Terminal 或 Diff
}
```

- **照录 `tool_called` 的两个键**，读不出或缺席即 `None`：这件工具没登记，或这一行写在这两个键出现之前。页面据 `render` 选画法（终端、差异、通用），据 `effect` 说这次调用越过了哪一种边界；两者都是 `None` 时按通用画。
- **携 kernel 的类型本身**，不在线上另立枚举（§8-0）：`Effect::Write` 带它的写域地址，`Connector` 带服务器的标签，都是登记写下的事实。
- **`Diff.locations` 今天恒为空**：账上记的是登记层面的声明；一次编辑调用改的是哪个文件，读 `subject`。
-/

/-!
### 8-56 被裁掉的输出指向它的原文

```rust
pub struct Output {
    // …既有字段…
    pub pinned: Option<Locator>,   // 这次调用的输出离窗时，原文存在哪；未离窗、旧行或读不出时为 None
}
```

- **只在调用的输出上有值**：`Call.output` 的 `pinned` 读自配对上的 `tool_result` 结果里第一笔离窗账目（`runtime::pipeline::pinned_original`，`crates/runtime/Spec.lean` §8-51(b)）；`Call.arguments` 也是 `Output`，它的 `pinned` 恒为 `None`——参数从不离窗。
- **原文是命令原本写出的字节**：结果先被 sieve 裁、再被普通搬运存一次时，指的是第一笔账目的原文，不是 sieve 留下的替身。页面拿它经 `Query::Content` 读全文，`cut` 仍只说这个视图裁了几行。
- **旧行明确缺席**：结果里没有账目（没离窗、或写在账目进结果之前）即 `None`，不按相邻的行去猜。
-/

/-! D3 本批上线的字段各读自一个权威，缺席即没有

**决定**：(a) `Turn.first_at` 照录 `model_returned` 的键；`Timing` 由 `EventRecord::moment` 与答复的错误码判出。线上不带首字耗时，也不从相邻行推断一个时刻量没量过。

**理由**：一个事实一个家。时刻语义的权威是 `crates/kernel/Spec.lean` §8-4 与 `EventRecord::moment`；首字耗时是两个时刻之差，页面手里已有这两个数。

**被否**：①线上带一个 `ttft` 时长：派生值在线上有了第二个家，而且 `t` 未量时它要答一个答不了的数；②`Timing` 三值（量过、回合时间戳、城补的）：页面对后两种做同一件事——不画用时——第三个值只会逼每个读者多写一臂；要分辨时，`Call.outcome` 与答复内容已经说明那是城补的。

(b) `CommitAnswer.previous` 由账本折出；`CommitAnswer.parents` 在答问时读 git。

**理由**：「同一次 run 的上一个提交」是账本上两行的关系，折叠已经按 `seq` 看过每一行。父提交是提交对象的一部分，oid 就是对它的哈希，账本从未记过它。

**被否**：①在 `checkpoint_committed` 里记下父提交：检查点的写方要多记一个字段，评审落地的合并提交由另一处写，这个键出现之前的每一行都没有它，而这三种情形 git 都答得出；②`previous` 只带 oid：一段的另一端还要一个 `seq`，才能不扫账本就往回读调用；③读 git 在锁内做：一页五百个提交各开一次仓库，别的问题都在等这把锁。

(c) `Call.effect`、`Call.render` 照录 `tool_called` 在调用那一刻记下的登记。

**理由**：登记只在 run 的工具台上存在，读面够不到；记在调用那一行，读面读的是那一刻的事实（kernel D11）。

**被否**：读面按工具名匹配出呈现：每加一件工具都要改这个匹配，楼的 MCP 工具读面不认识。

(d) `Output.pinned` 读自结果里第一笔离窗账目，账目不另记调用 id。

**理由**：账目住在它裁掉的那个结果里，那个结果写在带着 `tool_use_id` 的 `tool_result` 行上，所以调用身份已经由行给出。

**被否**：给 `ResultOffloaded` 加 `tool_use_id`：同一个 id 的第二个家，而这份账目随结果整份进模型的请求字节，多出的键会让每一次被裁的调用多付这些 token。
-/

/-!
### 8-76 外壳读的四样：缓存写、退出码、运行策略、工作树

```rust
pub struct Used {
    // …既有字段…（`cached` 是缓存读，即 `kernel::ModelUsage::cache_read_tokens` 报了的数，没报时缺席，D35）
    #[serde(default)] pub cache_write: Option<Tokens>,   // 缓存写，即 `ModelUsage::cache_write_tokens` 报了的数，没报时缺席
}
pub struct Call {
    // …既有字段…
    #[serde(default)] pub exit_code: Option<i64>,        // exec 的命令以哪个码结束，照录配对的 `tool_result`
}
pub struct Opening {
    // …既有字段…
    #[serde(default)] pub policy: Option<kernel::RunPolicy>,   // `run_started` 记下的运行策略（kernel D12）
}
pub struct RoundsAnswer {
    // …既有字段…
    #[serde(default)] pub worktree: Option<String>,      // 这次 run 被借给的工作树的名字，照录 `worktree_opened`
}
```

- **`cache_write` 与 `cached` 分开答**：两者都是 `input` 的一部分，单价不同（`gateway::market` 的 `cache_read_price` 与 `cache_write_price`），合成一个数页面就算不出命中率之外的任何东西。读法仍只有 `used_in` 一处，经 `ModelUsage` 自己的反序列化，旧行的换算由它做；`used` 在场时 `cache_write` 恒在场，`Option` 只为一个旧城答出的帧。
- **`exit_code` 只照录结果里的 `exit_code` 键**：键的拼法与「有码才写码」的规则住 `runtime::tools::exec::outcome`；`runtime::pipeline::exit_code_in` 是这个键的读者，读面经它读。信号停下的命令、城没等到的命令、非 exec 的工具、还没答的调用都是 `None`——没有码就不画码，`-1` 那种编出来的码一概没有。
- **`policy` 照录 `run_started` 的 `policy` 键**：携 kernel 的 `RunPolicy` 本身（四个值：模式、写入限制、准入要求、落地策略），不在线上另立四个平铺字段（D4 同一条理）。写在这个键出现之前的行是 `None`。
- **`worktree` 是 `worktree_opened` 的 `name`**：只有借到自己的工作树的 run 才有这一行；在楼自己的树里干活的 run、窗口没读到那一行的长会话都是 `None`，与 `opening` 同一口径。名字不是路径：路径是一台机器的事实（`kernel::event::record::WorktreeOpened`）。
-/

/-! D13 外壳读的字段在本版之内增加，各自可缺，各读自账本上写下它的那一行

**决定**：§8-76、§8-77、§8-78 的六个字段都是 `Option`、带 `#[serde(default)]`，`WIRE_V` 不动；每个字段照录账本上写下它的那一行（或 kernel 的那个常量），读面不推断。提交的 B3 是宣告它的那一行自己的 BLAKE3（下一行的 `prev` 存的就是它），不是那一行的 `prev`。提交的 ISO 时刻不另设字段：它就是 `CommitAnswer.at`，页面用它画时刻的那一处拼成 ISO。

**理由**：自上一次推送以来 `WIRE_V` 已经进过一位（D1），同一个发布之内不再进位；可缺的字段让一个缓存着的旧页面与一座新城、一个新页面与一座旧城都还连得上，旧的一端读到的只是「没有」。`prev` 是上一行的摘要，拿它当本行的身份会让每个提交都指向它前面那条无关的记录。ISO 只是 `at` 的一种拼法，线上再带一份就是一个时刻两个家，而页面在时间轴上本来就要拼毫秒精度的时刻。

**被否**：①进位到 46——同一个发布里第二次进位，违背「一次发布一次进位」；②把 `cached` 改名为 `cache_read`——改名是改形，旧页面读到的是缺了一个必填字段；③线上带 `iso: String`——见上；④在 `checkpoint_committed` 里另记一个内容摘要——账本的链已经给每一行一个摘要，再记一个是同一行两个身份。

**重开参数**：下一次进位 `WIRE_V` 时，`Option` 可以收紧为必填（`used` 在场时 `cache_write`、`ConfigAnswer.first`）。
-/

/-!
### 8-79 对话头读的三样冻结事实：回复几时返回、派出时的强度、冻下的名字

```rust
pub struct Turn {
    // …既有字段…
    #[serde(default)] pub returned: Option<TimeMs>,   // 回答这个回合的 model_returned 自己那一刻；那一行没量过即 None
}
pub struct Opening {
    // …既有字段…
    #[serde(default)] pub effort: Option<kernel::Effort>,   // `run_started.effort`（`crates/kernel/Spec.lean` §8-85）
    #[serde(default)] pub names: Option<FrozenNames>,       // `run_started.naming` 那一版，从内容库读回
}
pub struct FrozenNames {
    pub mayor: Option<String>,   // 冻下的主 Agent 名字；缺席即那时没人起名，页面画语言表的默认名
}
```

- **`returned` 是答复那一行的 `moment`**（`EventRecord::moment`），与 `first_at` 来自同一行：回合里最后一条 `model_returned`。那一行没量过（账本版本 1）即 `None`，不拿信封的 `t` 顶替。输出速率是 `used.output ÷ (returned − first_at)`，线上不带这个商（D3 同一条理：页面手里已有三个数）。
- **`effort` 照录 `run_started.effort`**：缺席读作「这次派活没说强度」或「这一行早于这个键」，页面都不画强度。
- **`names` 是冻下的那一版，不是今天的。** 读面拿 `run_started.naming` 的摘要从城的内容库取回字节，经 `city::Naming::from_bytes` 读成名字；`naming` 缺席、内容库不再存着那一版、或字节读不成名字，`names` 都是 `None`，页面回退到地址与本地化的角色名（refrain §3-13），不用今天的名字顶替。
- 验收：accounting `views::rounds` 的 `a_turn_reports_when_its_reply_returned`、`the_opening_carries_the_effort_and_the_names_the_run_froze`。
-/

/-! D17 冻下的名字在城里读成类型，线上不带摘要

**决定**：`Opening.names` 携带读好的名字（`FrozenNames`），由读面在作答时从内容库读出；线上不带 `naming` 摘要，页面也不经 `Query::Content` 自己取那份字节。

**理由**：那份字节是 `city::Naming` 的私有编码，读它的规则住 `city`；页面若自己取字节再解析，就是同一种编码的第二个读者，而且读到的是无类型的 JSON。读面本来就在作答时开内容库（前缀、文档范围都这样读），多读一个对象不改锁的次序。

**被否**：①`Opening.naming: B3Hash`，页面再问 `Query::Content`：第二个读者、第二次往返，且内容库的答复是给人看的文本截断，不是给程序读的值；②作答时读今天的 `IdentityAnswer`：一个已经开始的 session 会被改名，与模型请求里冻下的名字不符。

**重开参数**：第二个需要冻下名字的答复出现时（比如 session 列表也要画名字），把读回挪到视图折叠里按 run 缓存，而不是每问一次读一次内容库。
-/

/-! D28 `Call` 带出整数微秒的耗时；skill 与 MCP 的使用各是一个查询

**决定**：`Call` 多一件 `#[serde(default)] took_us: Option<u64>`，照录配对的 `tool_result.took_us`（kernel D20）；`Used` 多 `first_us` 与 `took_us` 两件，照录 `model_returned`。缺席时页面退回信封时刻之差，以毫秒显示。显示规则（不到 10 ms 用 µs，10 ms 及以上用 ms）在客户端一处实现。

查询面多三帧：`Query::SkillUsage { skill: Option<String> }` 答 `SkillUsageAnswer { skills: Vec<SkillUsageLine> }`，每行是一个 skill 的内容版本（摘要、时刻、写它的那一行）、最后一次审核（`skill_audited`，kernel D23）、最后一次使用、使用列表（run、居民、时刻、读的是哪一部分）与按天的计数，书架上从没被用过的 skill 也有一行，计数为零；`Query::McpUsage { server: Option<String> }` 答按服务器与工具折叠的同形一张表；`Query::UsageExport { what: UsageKind, format: ExportFormat }` 答 JSONL 或 CSV 的正文，`UsageKind { Skills, Mcp }`、`ExportFormat { Jsonl, Csv }`。三帧都是 `VerbClass::Read`。审核本身由城在落位之后做（city D19），不另开命令帧；装 skill 的门是 `InstallSkill`（D32），每一行算什么、怎样导出在 D33。它们与本版其他改形同一次 `WIRE_V` 进位（D22）。

**理由**：使用早已在 `tool_called` 里（kernel D23），视图只是按 skill 折叠；折叠住 accounting，线上只给答复的形状。导出走查询而不是让页面拼文件，是因为折叠规则只在 Rust 一处。

**被否**：①页面拉全部 `tool_called` 自己折：浏览器里多一份折叠规则；②导出写进城目录的文件：城目录多一份没人清理的派生物。

**重开参数**：使用列表大到一次答复装不下时，加分页游标。
-/

/-! D33 一次 skill 使用与一次 MCP 使用各从哪一行折出，按天怎么数，导出的每一行长什么样

**决定**：折叠是 accounting 里一个读账本的纯函数，`SkillUsage`、`McpUsage` 与 `UsageExport` 三帧读同一个函数的结果（D28）。
- **一次 skill 使用**是一行 `tool_called`，它所在的 run 的 `run_started` 钉住了一件 skill（名字与哈希，`crates/runtime/Spec.lean` §8-11），而这一行是：`describe`，问的就是那个名字（部分记作 `guide`）；或 `read`，路径是那个名字（部分记作 `SKILL.md`）或 `<名字>/<相对路径>`（部分记作那个相对路径）。认的是「这个 run 钉住了这个名字」，不从路径的写法猜：一个恰好与某件 skill 同名的普通文件，在没钉住它的 run 里不是一次使用。使用的内容版本是钉住时的哈希，所以 skill 改过之后，旧的使用仍指向它当时读到的那一版。
- **一次 MCP 使用**是一行 `tool_called`，它的工具名是某个 MCP 服务器登记的工具，或者是 `call`、而 `call` 指名的那件是这样的工具。名字到 `(服务器, 工具)` 的对应读城此刻的 MCP 登记（`agent_protocols::mcp` 唯一造这个名字的那一处），不拆字符串：服务器名可以含 `_`，`<server>_<tool>` 的拆法有歧义。此刻没有服务器登记的名字归在 `server: None` 下，页面写作「已移除」并给出完整工具名。
- **结果**也算：每次使用带上配对的 `tool_result` 是否成功；没有配对行（run 被杀）记作未知。
- **按天数**：按信封时刻的 UTC 日历日分桶，`day` 写作 `YYYY-MM-DD`；导出带每一行的毫秒时刻，要按本地时区重新分桶的分析自己做。
- **从没用过的**：每一格书架（城库、每栋楼的楼架、城外书架）上扫到的每一件 skill 都有一行，没被用过的计数为零、使用列表为空；MCP 是此刻登记的每个服务器的每件工具。
- **导出**：每次使用一行，列序固定：`kind`（`skill`｜`mcp`）、`name`、`server`、`part`、`run`、`resident`（房间地址）、`seq`、`at_ms`、`digest`、`outcome`（`ok`｜`failed`｜`unknown`）。JSONL 每行一个对象、键即列名、缺席的值写 `null`；CSV 首行是列名，按 RFC 4180 加引号，行尾 `\r\n`，缺席的值为空。正文以 UTF-8 回答，页面经浏览器的下载交给 User，城目录里不写文件（D28）。

**理由**：一次使用必须能在重放时由同一个函数算出同一张表，所以只读账本里已有的行；用 `run_started` 钉住的名字来认，是因为那是 run 自己说过「我能读这几件」的唯一一处，路径的写法会把巧合当成使用。按 UTC 分桶，是因为折叠在 Rust 一处、在城里算，城不知道页面在哪个时区，而导出的毫秒时刻让任何分桶都做得出来。

**被否**：①读文件系统的访问时间：Windows、macOS 与 Linux 上 atime 的语义各不相同，Linux 默认 `relatime` 一天只更新一次，而且它说不出是哪个 run；②按页面所在时区分桶：同一个城在两台设备上答两张表；③MCP 名字按第一个 `_` 拆：服务器名含 `_` 时把工具归错服务器。

**重开参数**：一次查询在一座大城里要读的行多到答复超过 100 ms 时，把折叠改成随账本增量维护；User 要求按本地日历日看时，加一个由页面送来的 UTC 偏移参数。

**三个平台**：折叠只读账本，三个平台相同；CSV 的 `\r\n` 是 RFC 4180 的规定，与平台无关。
-/

/-! D35 `Used` 的两个缓存数，provider 没报时缺席

**决定**：`Used.cached` 是 `#[serde(default)] Option<Tokens>`，与 `cache_write` 同形：`kernel::CacheCount::Reported(n)` 读成 `Some(n)`，`Unreported` 读成 `None`（kernel D36）。`cache_write` 的 `None` 原来只表示「旧城的帧」，现在同时表示「provider 没报」，两者对页面是同一件事：不知道。页面在 `None` 处写 `lang.json` 里的「未知」字样，而不是 0 或 0%。这一改形与本版其他改形同一次 `WIRE_V` 进位（D22）。三个平台上形状相同。

**理由**：线上的 0 页面只能照写成「命中 0%」，而一个从不报缓存的 provider 的命中率是不知道，不是零；把这个区别在线上丢掉，kernel 记下它就没有用处。

**被否**：①线上照旧写 0、另加一个「报了没有」的布尔：读者照样会先读到那个 0；②`cached` 保持必有、只让 `cache_write` 可缺：缓存读正是页面算命中率的那个数。

**重开参数**：provider 开始分别报告「没有缓存功能」与「有缓存但这次没报」时，`None` 拆成两臂。
-/

/-! D34 一个停在同步 `send` 上的 run，在 `RunSummary` 上多一个 `waiting`；`send` 的 `wait` 是工具参数，不是线上帧

**决定**：`RunSummary` 多一件 `#[serde(default)] waiting: Option<Waiting>`，`Waiting { on: Address, until: TimeMs }`：这个 run 最近一行 `signal_wait_started`（kernel D32）还没有配对的 `signal_wait_ended` 时，`on` 是它等的房间，`until` 是那一行的 `deadline_ms`——城的注入时钟读出的 epoch 毫秒，本来就是墙钟时刻，所以不必换算；配对行到了、或这次跑冻结了，就缺席。这一折是 `storage::hot` 的（storage D29，`frozen_run_never_waits`），`accounting` 的 `summarize` 只搬运。`RunSummary` 自 `answer.rs` 分出到 `answer/run_summary.rs`，`Answer::Run` 装的是 `Box<RunSummary>`：线上的形状不变（serde 对 `Box` 透明），只是这一臂不再把整个 `Answer` 撑大。直播的页面从事件流里自己折同一对种类，查询答的是没看过流的读者。`send` 的同步开关是 `send` 工具的输入 `wait: bool`（缺省 `false`，collab D9），走模型的工具调用，不经过线上的任何一帧；线上只多这一件字段，与本版其他改形同一次 `WIRE_V` 进位（D22）。

**理由**：一个停着等回信的 run 与一个卡住的 run 在 `last_kind` 上看起来一样，页面与 watchdog 都要能把两者分开，所以等待要成为 run 的状态的一部分；它从账本的两行读出，重放与远程设备看到的是同一个状态。`until` 用墙钟时刻而不是剩余毫秒，是因为答复会被缓存与转发，剩余时间一离开城就不对了。

**被否**：①一个新的 `RunWaiting` 帧：等待是 run 的一个状态，不是一类消息，单开一帧的页面要把两路拼回一个 run；②把 `wait` 放进一个 User 发的命令：同步与否是发信的那个 run 的选择（D89 第 1 条：C 是 `send` 上的开关），User 不在这条路上。

**重开参数**：一个 run 可以同时等两个房间时，`waiting` 改成一张表。
-/
