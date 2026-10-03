-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::event::record

规定 `kernel::event::record`（`crates/kernel/src/event/record.rs` 与 `crates/kernel/src/event/record/` 下按族分的文件）：每个有载荷结构的 `EventKind` 一个 serde 结构，以及它们与账本之间仅有的两扇门。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
**`kernel::event::record`——每个 `EventKind` 一个 serde 结构**

载荷的键只在这里拼写一次；`Payload::of` 与 `Payload::read` 是它与账本之间仅有的两扇门，
调用点不手写 `insert("k")` 与 `get("k")`。

```rust
pub struct SkillPin { pub name: String, pub hash: B3Hash }   // hash 必填
pub struct RunStarted {                 // 字段全部 #[serde(default)]
    pub task: String, pub goal: String, pub job: Option<Locator>,
    pub parent: Option<RunId>, pub predecessor: Option<RunId>,
    pub skills: Vec<SkillPin>,          // 空亦写出
    pub dispatched_by: Option<Who>,     // 由谁派来：person／city／派活的居民地址；缺键为 None
    pub policy: Option<RunPolicy>,      // 这次 run 的运行策略（§8-77）；缺键即这一行早于策略入账
    pub effort: Option<Effort>,         // §8-85
}
pub struct RunForked { pub from: RunId, pub at_seq: Seq }
pub struct PromptSource { pub addr: Address, pub kept: u64, pub marker: bool, pub dropped: u64 }
    // prompt_assembled 的一行来源；不带摘要生产者指纹：没有路径产出摘要（runtime D1）。旧行里多出的 producer 键读时忽略；本结构没有方法，读者直接读字段
pub struct EvalRun { pub probe: String, pub version: u32, pub predecessor: RunId,  // eval_run：交接探针的一次读数
                    pub kept: u32, pub lost: Vec<u32>,       // lost 是答案不同的题号
                    pub before: Vec<String>, pub after: Vec<String> }   // 全部必填，空亦写出
pub struct WentBack { pub name: String, pub point: GitOid }            // 回到过去：一棵新树起于 point
pub struct FileRestored { pub name: String, pub path: String, pub point: GitOid } // path 取 git 树的写法
pub struct CommitAttribution {          // flatten 进每一条指名提交的记录
    pub model: String, pub effort: Option<Effort>, pub predecessor: Option<RunId>,
}
pub struct Commit { pub oid: GitOid, #[serde(flatten)] pub by: CommitAttribution,
                    pub scope: Vec<String>, pub files: Vec<String> }
#[serde(untagged)]
pub enum CheckpointCommitted { JobPinned { job: Locator }, Committed(Commit) }

pub enum Scope { City, Building(Address), Workshop(Address) }   // city | building:<addr> | workshop:<addr>
impl Scope { pub fn covers(&self, addr: &Address) -> bool; pub fn parse(raw: &str) -> Result<Scope, AxError>; }

pub struct ApprovalResolved { pub id: ApprovalId, pub verdict: Ruling, pub cluster: ClusterKey }
pub struct AutonomyChanged { #[serde(default = "city_wide")] pub scope: Scope,
                             #[serde(with = "autonomy_word")] pub autonomy: Autonomy }
pub struct CityHalted { pub scope: Scope, pub state: Admittance }
#[serde(rename_all = "snake_case")] pub enum Admittance { Halted, Released }
pub mod autonomy_word {                          // owner | delegate:<resident>
    pub fn spell(autonomy: &Autonomy) -> String;
    pub fn read(word: &str) -> Result<Autonomy, AxError>;
}
pub struct GovernedDocumentWritten { pub which: String, pub bytes: usize,
                                     pub naming: Option<B3Hash> }        // §8-79
pub struct RulesChanged { pub scope: Scope, pub which: GoverningDocument,
                          pub before: Option<B3Hash>, pub after: B3Hash, pub bytes: usize }
pub enum GoverningDocument { Rules, Config }     // serde: "RULES.toml" | "CONFIG.toml"
pub struct FileDiscarded { pub paths: Vec<String>,            // `file:<path>`，git 认的名字 Address 未必认
                           pub restoration: Option<Restoration> } // 缺或方案不识：None，paths 照读
pub struct DiscardRestored { pub paths: Vec<String> }
pub struct AssetArchived { #[serde(default = "fact")] pub kind: String,
                           #[serde(default)] pub day: u64, #[serde(default)] pub subject: String }
pub struct PursuitChanged { pub step: PursuitMove,       // goal 只在 clear 之后缺席
                            pub goal: Option<String> }
#[serde(rename_all = "snake_case")] pub enum PursuitMove { Set, Pause, Resume, Clear }
pub struct GoalConflict { pub goal: GoalId, pub with: GoalId, pub level: ConflictLevel }
#[serde(rename_all = "snake_case")] pub enum ConflictLevel { Serialize, Arbitrate }
// goal_registered 的载荷就是 GoalEntry 本身，不另立 struct：目标表持有的正是它
impl PursuitChanged { pub fn held(self) -> Result<Option<(String, PursuitState)>, AxError>; }
pub struct SignalEnqueued { pub id: SignalId, pub kind: SignalKind, pub from: String, pub room: Address,
                            pub room_version: Version, pub payload: Payload, pub at: TimeMs,
                            pub lane: Option<Lane> }   // lane 写出给 collab 之外的读者，回读不采信：它由 kind 推出
pub struct RoadmapMoved { pub by: String, pub node: NodeId,   // roadmap_claimed／split／finished／released／blocked 共用
                          #[serde(flatten)] pub step: RoadmapStep }   // 五行同有 by、node、verb，差别只在 verb 之后
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum RoadmapStep { Claimed { item: String }, Split { children: Vec<String> },
                       Finished { item: String, evidence: Locator },
                       Released { item: String, why: StopCause, line: String },
                       Blocked { item: String, why: StopCause, line: String } }
// NodeId 读回经 NodeId::parse：node 读不出的一行让折叠停下，而不是被跳过——跳过会让节点看似无人持有
pub struct WorktreeOpened { pub name: String, pub disk_bytes: ByteLen }   // 不携路径：路径是一台机器的事实
pub struct SignalConsumed { pub id: SignalId, pub by: String }   // 内容已在 enqueue 行里，不写第二遍
#[serde(rename_all = "snake_case")] pub enum Lane { Urgent, Ordinary }
pub struct SignalId(String);                       // 非空、无空白；serde 经 parse／as_str
pub enum SignalKind { Mention, Thread, Broadcast, Steer }   // serde 经 parse／as_str，四个线上词只一处
pub struct EmbeddingCalled { pub model: String, pub inputs: u64, pub vectors: u64,
                             pub dimensions: Option<u64>, pub prompt_tokens: Option<Tokens> }
pub struct RerankCalled { pub model: String, pub passages: u64, pub ranks: u64,
                          pub prompt_tokens: Option<Tokens> }

#[serde(rename_all = "snake_case")] pub enum AdviserAsk { Noul, Score, Choice }
pub struct AdviserAsked { pub ask: AdviserAsk, pub subject: String }
#[serde(tag = "ask", rename_all = "snake_case")]
pub enum AdviserAnswer { Noul { keep: bool, confidence_bp: u16 },
                         Score { score_bp: u16 }, Choice { chosen: String } }
pub struct AdviserAnswered { pub subject: String, #[serde(flatten)] pub answer: AdviserAnswer,
                             pub elapsed_ms: u64 }
#[serde(rename_all = "snake_case")]
pub enum AdviserFailure { Unavailable, Timeout, Unreadable }
pub struct AdviserFellBack { pub subject: String, pub reason: AdviserFailure }

// record::credential：F3 家族里凭据进出的两行。`ref` 是 SecretRef 而不是 String，
// 语法之外的引用读不成这一行（Payload::read 报 E_WIRE_MISMATCH），不再被读者各自静默跳过。
// 旧版本在订阅登录时另写过 `expires_at` 键；读入时忽略它，本版本不写。
pub struct SecretCaptured { #[serde(rename = "ref")] pub reference: SecretRef,
                            #[serde(default)] pub origin: String }  // enrolment | pasted；旧行另有 <provider>-subscription | <provider>-renewal
pub struct ToolkitLinkOpened { pub toolkit: String }

// record::endpoint：F3 家族里「哪个 model 替哪个 tag 作答」的两行。gateway 的 EndpointBook
// 只经 Payload::read 读它们；InputKinds 随之归 kernel（gateway::InputKinds 是它的再导出），
// 因为账本行的值要用 kernel 自己的类型。
#[serde(rename_all = "snake_case")] #[derive(Default)]
pub enum InputKinds { #[default] Text, TextImage }
pub struct ModelSelected { pub tag: ModelTag, pub endpoint: String, pub model: String,
                           pub context_tokens: u64,
                           #[serde(default)] pub max_output_tokens: Option<u64>, // 总是写出，缺席写 null
                           pub ceiling_from: Option<String>,   // person|upstream|preset|policy；缺席即省略
                           #[serde(default)] pub input: InputKinds, // 旧行没有这个键，读作 Text
                           pub input_price: UsdMicros, pub output_price: UsdMicros,
                           pub cache_read_price: UsdMicros, pub cache_write_price: UsdMicros }
impl ModelSelected { pub fn ceiling(&self) -> Option<Ceiling>; }   // 0 与 null 同读作「未声明」
pub struct EndpointLost { pub name: String }
pub struct EndpointAttached { pub name: String, pub base_url: String, pub dialect: DialectKind,
                              pub auth: Option<SecretRef>,          // 引用，从不是密钥；缺席即省略
                              pub auth_header: Option<String>,      // 非 bearer 时凭据所在的 header
                              pub models: Vec<String>,              // 空亦写出
                              pub connection_kind: Option<String>,  // 旧行没有，读者按 dialect 回推
                              #[serde(default = true)] pub probed: bool,
                              pub tuning: Option<AttachedTuning> }  // 什么都没设就省略；不是对象读作未设
pub struct AttachedTuning { pub label: Option<String>, pub timeout_ms: Option<u64>,
                            pub stream_idle_timeout_ms: Option<u64>, pub request_max_retries: Option<u32>,
                            pub proxying: Option<Proxying>,        // 默认值省略
                            pub max_in_flight: Option<u32>,        // 没人定过就省略；1 到 256 之外读作未设
                            pub extra_headers: Vec<(String, String)>, pub overrides: Vec<(String, String)> }
// AttachedTuning 的每个键缺席读作未设、在而读不懂也读作未设（行不被拒）：编造一个期限比没有期限更难解释。
// EndpointAttached 顶层的键则不然：probed、auth、connection_kind 在而读不懂，整行读不成（E_WIRE_MISMATCH），
// 因为把一个没探到的端点读成探到过，是在书里放进一个没人够得着的端点。

// record::probe：endpoint_probed，一次探测在挂上任何东西之前看到的。ModelFacts 随之归 kernel
// （gateway::ModelFacts 是它的再导出，读一行 /models 的 gateway::endpoint::models::facts_of 仍归 gateway），
// 因为账本行的值要用 kernel 自己的类型。
pub struct ModelFacts { pub id: String, pub context_tokens: Option<u64>, pub max_output_tokens: Option<Ceiling>,
                        pub input_modalities: Vec<String>, pub input_price: Option<String>,
                        pub output_price: Option<String> }        // 缺席写 null，行没说就是没说
pub struct EndpointProbed { pub name: String, pub base_url: String, pub reach: Reach,
                            pub models: Vec<String>, pub facts: Vec<ModelFacts>,   // 读不到列表时两者都写空
                            pub failed: Option<ProbeFailure> }                   // 缺席即省略
pub struct ProbeFailure { pub code: String, pub subject: String }
// record::provider：provider_degraded 有两个写方、两种形状，一个 untagged enum 让读者靠读来分，
// 不靠猜键：E_PROVIDER 经 kernel::error 的 carrier 表平铺写成 AxError 本身；vault 启动探针退到
// session memory 时写 VaultFellBack。两者都不是的行读不成（E_WIRE_MISMATCH），错误里并列两种
// 形状各自缺的那个字段：Deserialize 手写，先试 AxError 再试 VaultFellBack；派生的 untagged 只会说
// 「没有一个变体匹配」，Note::Unreadable 就给不出该去看哪个字段。写出仍是 untagged。
#[serde(untagged)] pub enum ProviderDegraded { Refused(AxError), VaultFellBack(VaultFellBack) }
pub struct VaultFellBack { pub component: String,     // 探针写 vault
                           pub fallback: String,      // 探针写 session-memory
                           pub persistence: String,   // gateway::Persistence 的自有拼法
                           pub reason: String }
// wire::note_of 只把 Refused 记成回合上的 Note::Refused；VaultFellBack 没改变任何回合，不出 note。

// record::harness：官方 harness 居民一次 run 写的两种行（`crates/sprawling/Spec.lean` §8-4e）。
// 词是本城的，不是 ACP 的：账本写下的拼写不能再改，ACP 的词跟着上游走（D9）。
#[serde(tag = "report", rename_all = "snake_case")]
pub enum HarnessReported {
    Said { text: String },                                    // 回答城的一段
    Thought { text: String },                                 // 推理的一段
    ToolCall { call: String, title: String, kind: String },   // harness 开始的一次工具调用，按它自己的话
    ToolCallStatus { call: String, status: String },          // 那次调用的状态变了
    Other { variant: String },                                // 本城没有读法的一种汇报，只记它的名字
    PermissionAsked { title: String, options: Vec<HarnessPermit> },
    PermissionAnswered { chosen: Option<String> },            // 城选的 option id；None 即答 cancelled
}
pub struct HarnessPermit { pub id: String, pub name: String, pub kind: HarnessPermitKind }
#[serde(rename_all = "snake_case")]
pub enum HarnessPermitKind { AllowOnce, AllowAlways, RejectOnce, RejectAlways }
pub struct HarnessAnswered { pub stop: HarnessStop, pub text: String }   // text 空亦写出
#[serde(rename_all = "snake_case")]
pub enum HarnessStop { EndTurn, MaxTokens, MaxTurnRequests, Refusal, Cancelled }

pub struct ToolCalled { pub id: String, pub name: ToolName, pub args: Payload,
                        pub subject: Option<String>,      // 键缺席读作 None
                        pub effect: Option<Effect>,       // 调用那一刻的登记（§8-75）；缺席即省略
                        pub render: Option<RenderIntent> }
pub struct ToolResult { pub tool_use_id: String, pub name: ToolName,
                        #[serde(flatten)] pub answer: ToolAnswer }
#[serde(untagged)]
pub enum ToolAnswer { Answered { result: Payload }, Failed { error: Payload } }

pub struct CityInitialized {}           // 城名在信封的 addr
pub struct BuildingCreated { pub addr: Address, pub template: String,
                             pub adopted: bool }   // false 时不写出
pub struct BuildingConfigured { pub addr: Address, pub sandbox: bool, pub mcp: bool,
                                pub desktop: bool, pub context: bool }
pub struct CancelReceived {}
pub struct HandoffWritten { pub must_read: Vec<Locator>, pub overview: String,
    pub progress: String, pub context: String, pub next_step: String }
pub struct WatchdogFired { #[serde(flatten)] pub action: FiredAction,
                           pub corrections: u32, pub provider_failures: u32 }
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FiredAction { Steer { text: String },
                       BackOff { until_ms: u64, code: String, subject: String },
                       Freeze { reason: String } }
pub struct GateChecked {}               // 无生产写方：结构是决定
pub struct PolicyChanged { pub id: String }   // policy_created／policy_revoked，无写方：结构是决定
#[serde(untagged)] pub enum CacheRenewed {   // cache_renewed：先试 Refused，因为 Answered 的字段全可缺
    Refused { refused: AxError },
    Answered { usage: Option<ModelUsage>, billed_usd_micros: Option<UsdMicros> },
}
```

结构按族住 `crates/kernel/src/event/record/` 下；哪个 kind 有结构，以那里的类型为准。
`gate_denied`、`budget_limit` 的载荷是平铺的 `AxError`，`approval_requested` 的是 `ApprovalItem`，
`result_offloaded` 的是 `runtime::sieve::ResultOffloaded`（它平铺筛子的账，筛子在 runtime），不另立结构；
`watchdog_fired` 的读方 `wire::note_of` 把 `BackOff` 读成它等待的那次拒绝（码与主语照录），
`Steer`／`Freeze` 不出 note——纠偏与冻结各有自己的行；
`pr_merged` 借 `CommitAttribution` 记「谁做的这次提交」，其余键由调用点手写。
没有结构的 kind 由调用点手写读取。

- **harness 的两种行各有一个结构。** `harness_reported` 一条汇报一行，`report` 键说是哪一种；`harness_answered` 在停止原因到了时写一行，携停止原因与这一回合 harness 回答城的文字（依次拼起来的回答段）。两者都不 `default`：这两种行自诞生起就按这个形状写，缺键的行该拒而不该猜。回答「城做了什么」的折叠恒不读 `harness_reported`，只有回答「harness 说了什么」的视图读它。

- **`ToolCalled.subject` 在写记录时算定，读方读它，不从 `args` 再推一遍。**
  两个读方各按自己的 map 序挑第一个字符串时，同一次调用读出两个主语：
  `{"10":"a","2":"b"}` 在账本的 `BTreeMap` 字节序下是 `"a"`，
  在浏览器的自有属性序下是 `"b"`。这张优先键表（`path`／`addr`／`program`／`arm`，
  都不在时取载荷键序里的第一个字符串）只有一处，即 `ToolCalled::subject_of`；
  写方 `runtime::turn::wave` 调它一次，`wire` 不再持有第二份。
  `None` 是这次调用的参数没有指名任何东西，是一个真实状态而不是失败。

`ApprovalResolved.verdict` 是 `Ruling` 本身而不是 `format!("{verdict:?}")` 的小写词：
两个读方若各自与字面量比较，认不出的词就会在一处读作拒、在另一处读作准；
所以认不出的词是一次读失败，不是一个默认值。三键均不 `default`——`approval_resolved`
自诞生起就无条件写出这三键，缺 verdict 的行该拒而不该猜。
`AutonomyChanged` 与 `CityHalted` 的四个值都有类型：作用域是 [`Scope`]，
「关／开」是 `Admittance`，「谁来答」是 `Autonomy`。三条小文法只在 `record` 拼写与读回，
写方与每个读方都经这里，
认不出的词是一次 `E_WIRE_MISMATCH`，不会在一处读作「开」、在另一处读作「人自己答」。**已落盘的行照常读**：三种拼法逐字未变，
而 `autonomy_changed` 的 `scope` 键在它存在之前写下的行里缺席，那样的行指的是整座城，
`city_wide` 就这样读它。

三条不变量，因为已落盘的账本不可重拼：

1. **字节不动。** 字段名即旧写方用的键；旧写方省略的键写 `skip_serializing_if`，
   旧写方无条件写出的键（含空数组）无条件写出。键序由 `serde_json` 的 BTreeMap 定，
   与此处声明序无关。每族一组逐字节对拍测试守住这条。
2. **读宽写严。** 旧构建可能不写的字段一律 `#[serde(default)]`——`tools/fixtures/golden-s1`
   里就有一条 `data` 为 `{}` 的 `run_started`；未知键忽略而不拒。
3. **值保留 kernel 类型。** run 是 `RunId` 而非 `String`，job 是 `Locator`；
   手写 `Serialize` 的类型挂 `schemars(with = "String")` 说明其 wire 形状。

一处记录在案的形状债：`checkpoint_committed` 同时承载「派工钉住的 job」与「检查点提交」
两件不同的事，`CheckpointCommitted` 把这件事说出口而不是让读方从缺键推断；拆成两个 kind
需要新 `EventKind` 与账本版本，故记录在此而不在此处做。
-/

/-!
### 8-75 回合记录多记的事：回复的首个内容几时到（形状 2 值类型）

**(a) `model_returned.first_at`**

```rust
pub struct ModelReturned {
    // …既有字段…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_at: Option<TimeMs>,   // 这次回复的首个非空内容到达城的时刻
}
```

- **它是一个时刻，不是一个时长。** 首字耗时（TTFT）由读者拿它减去开这个回合的 `model_called` 的 `t`；账本不记派生值，记下的两个时刻已经够算。
- **读数来自回合的钟**：`runtime::turn` 在它包住的增量汇点里读第一段非空内容到达的那一刻（`crates/runtime/Spec.lean` §8-50）。kernel 不采样，gateway 也不采样：`kernel::Model` 的实现从不读钟。
- **缺席有三种情形，都不是零**：回复从一扇到齐之前什么也不报的门回来（阻塞门、没有流的适配器、流式解析失败之后换阻塞门重发修好的那一次）；回复在流上只带工具调用，没有一段文字或推理（首个内容的定义与理由见 runtime D6）；这把键出现之前写下的每一行。三种都读作「没有量到」，页面不画首字耗时，不猜。
- **字节不动**：缺席即省略（`skip_serializing_if`），旧行与今天没量到的行字节相同；读宽（`default`）。账本版本不为此进位：`v` 为 2 的行里这一格可以缺席，读者不从版本推断它在不在。

**(b) `tool_called` 记下这件工具的登记：`effect` 与 `render`**

```rust
pub struct ToolCalled {
    // …既有字段…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,          // 调用那一刻这件工具登记的效果
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<RenderIntent>,    // 调用那一刻这件工具登记的呈现
}
// Effect 与 RenderIntent 在 feature `schema` 下派生 JsonSchema：线上 Call 直接携它们（`crates/wire/Spec.lean` §8-55）
```

- **写方只有回合的工具波**：`runtime::turn::wave` 写 `tool_called` 时，从工具面的 `ConcurrentInvoke::meta_of` 取这件工具的 `ToolMeta`，照录其中两项（`crates/runtime/Spec.lean` §8-51）。工具面不认识这个名字时两者缺席——一次调用了没登记的工具，是一个真实状态。
- **记的是调用那一刻的登记**：一件工具以后换了效果或呈现，旧行仍说它当时是什么；折叠不去问今天的登记。新加一件内置工具不需要改任何读者：它的登记随它第一次被调用写进账本。
- **`Diff.locations` 照录登记**：登记层面的声明是空表（§8-23），每次调用的位置是工具一侧由参数算出的纯函数，今天还没有这个函数，所以账上的 `locations` 恒为空；一次编辑调用改的是哪个文件，读者读 `subject`。
- **不进模型的字节**：`tool_called` 是入窗种类，但窗口从 `model_returned` 的消息重建工具调用，从不读 `tool_called` 的载荷（`runtime::fork` 的逐种类表把它列在「不是对话」一侧），所以这两个键不改变任何请求。
- **字节不动**：缺席即省略，读宽；旧行两键都缺，读作「没有记下」。
-/

/-!
### 8-82 一次 run 怎样开篇，进程死后谁冻结它（`kernel::event::record::run`，形状 2 值类型）

#### 8-82-1 `run_started.opening`

```rust
pub enum Opening { FromJob, Inherited, WithPerson }   // "from_job" | "inherited" | "with_person"
pub struct RunStarted {
    // …既有字段…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<Opening>,   // 第一条 user 消息的写法；缺席读作「不知道」
}
```

- **它说什么。** 一次 run 的第一条 user 消息有三种写法：`FromJob` 是一句指向前缀 run 段里 JOB.md 的话加目标，`Inherited` 是 `Task: …\nGoal: …`，`WithPerson` 是人的原话。写法在派活时由城定一次（`accounting::worker::freezing` 按 `city::RunBrief` 选），`runtime::conversation::Conversation::push_task_lines` 按它写出字节。
- **为什么记进账本。** 分叉从账本重建母 run 的对话（`runtime::fork::fold_run`），第一条消息要照母 run 发出的写法重建，provider 的前缀缓存才从第一条消息起命中（`crates/runtime/Spec.lean` §8-58）。不记，重建只能猜；而按 `goal` 是否为空去猜，等于在分叉里再写一遍 `city::write_brief` 的「有目标才是一份 job」那条规则。
- **一个枚举，一个家。** 这个值以前只住在 `runtime::conversation`；现在账本载荷要带它，而 runtime 依赖 kernel、kernel 不依赖 runtime，所以枚举住在本模块，`runtime::Opening` 与 `runtime::conversation::Opening` 是它的再导出，调用方的路径不变（runtime D14）。
- **写者**：`runtime::run::Charter::open` 从 `Charter.opening` 照录。模型 run 的 charter 填 `Some(RunPlan.opening)`；harness run 的第一句话是交给 harness 自己会话的 prompt，城不为它写 user 消息，填 `None`。
- **缺席读作「不知道」。** 加这个键之前写下的行没有它，`fold_run` 对它沿用加键之前的读法（`crates/runtime/Spec.lean` §8-58 的表）。按 `default` 加、缺席不写，所以旧账本照读，`golden-s1` 里载荷为 `{}` 的那行 `run_started` 照旧读成 `RunStarted::default()`。
- 验收：`runtime::fork::request_tests` 的 `a_branch_first_request_opens_with_the_bytes_of_the_mothers_last`，与 `accounting::worker::freezing::tests::lineage` 的 `a_branch_first_request_carries_the_bytes_of_the_mothers_last`（`crates/sprawling/Spec.lean` §8-141）：写出的键经真实的派活与分叉读回。

#### 8-82-2 进程死后冻结的那一行：`run_frozen.cause`

```rust
pub enum FreezeCause { ProcessDied }                  // "process_died"
pub struct RunFrozen {
    pub completion: String,
    pub evidence: Option<Vec<EvidenceCite>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<FreezeCause>,  // 不是 run 自己的驱动写下这一行时，为什么写
}
impl RunFrozen {
    pub fn of(completion: &Completion) -> RunFrozen;   // cause 恒 None，字节与加键之前相同
    pub fn lost() -> RunFrozen;                        // completion "cancelled"、无 evidence、cause ProcessDied
}
```

- **它说什么。** ARCHITECTURE §13.7 画的是 `Lost --> Frozen`：进程死在一次 run 的半途，那次 run 再也不会被驱动，重开的城要把它冻结。冻结的那一行由重开时的启动扫描写（`crates/accounting/Spec.lean` §8-18），不由 run 自己写，所以载荷多一个键说明原因。
- **结局仍是三种之一。** `Completion` 的三种结局不变（§8-20），死掉的 run 冻成 `cancelled`：它没做完，`Done` 需要城自己记下的证据而它没有；也没有被上限截断，`Limit` 说的是那件事。`cancelled` 说的是「停下了，不是做完」，`cause: process_died` 把「人或 halt 叫停的」与「进程死了」分开（D14）。
- **一处构造。** `RunFrozen::lost` 是写这一行的唯一入口，「死掉的 run 冻成哪一种结局」只在这里回答。
- **缺席即 run 自己的冻结。** `RunFrozen::of` 写的行没有这个键，字节与加键之前相同；旧行照读。读结局的读者（`views` 的 `completion`、`city::resident` 的计数）照旧只读 `completion`。
- 验收：`sprawling` 的崩溃验收 `a_city_killed_while_writing_an_answer_reopens_with_the_torn_line_cut_and_the_call_unknown`（`crates/sprawling/Spec.lean` §8-127）钉住死 run 的最后一行是这一行。

### 8-79 身份的两处入账：保存的回执与一次 run 冻下的那一版（`kernel::event::record`，形状 2 值类型）

```rust
pub struct GovernedDocumentWritten {
    pub which: String,
    pub bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naming: Option<B3Hash>,     // 写完之后城的身份版本；写 CLERK.md 与旧行为 None
}
pub struct RunStarted {
    // …既有字段…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naming: Option<B3Hash>,     // 这次 run 的 session 冻下的身份版本（`crates/city/Spec.lean` §8-33）
}
```

- **两个键都只是摘要。** 身份本身——名字与「关于你」——在两份治理文档里，冻下的那一版在内容库里，摘要是读回它的钥匙（`cas:<naming>`）。账本不抄名字：名字是人改的字，抄进不可删的历史就有了第二个家，而人删掉的一段「关于你」会永远留在那里。
- **回执。** 写 `MAYOR.md` 或 `PREFERENCES.md`（`PutDocument`、`PutIdentity`）之后的那一行带上新的身份版本，页面见到它才把卡片标为已保存（`crates/wire/Spec.lean` §8-59）。写 `CLERK.md` 不动身份，键缺席。
- **一次 run 用的是哪一版。** `run_started.naming` 由 `runtime::run::Charter::open` 从 `RunPlan.naming` 照录（`crates/runtime/Spec.lean` §8-56）。旧行没有这个键，读作「这一行早于身份入账」，页面回退到地址或角色名，不用今天的名字。
- 两个键都按 `default` 加、缺席不写，所以旧账本照读，旧构建读新行时把它们当未知键拒（§8-40 的方向门照旧）。
- 验收：`record::run` 的 `a_run_started_line_records_the_naming_it_froze`（写出、读回、缺席不写）。
-/

/-!
### 8-85 `run_started` 记下这次 run 派出时冻下的推理强度（`kernel::event::record::run`，形状 2 值类型）

```rust
pub struct RunStarted {
    // …既有字段…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,     // 这次 run 的请求冻下的强度（`runtime::CallShape.effort`）；缺席即没有说，由提供方自定
}
```

- **照录冻下的那一个值。** 强度随模型一起在 session 冻结（`runtime::turn::report` 拒绝 run 中途改它），所以 run 开头记一次就是整次 run 的事实。`runtime::run::Charter::open` 从 `RunPlan.shape.effort` 抄进来，与 `policy`、`naming` 同一处写（`crates/runtime/Spec.lean` §8-56）。
- **缺席有两种来历，读法相同。** 一是这次派活没有说强度，提供方自定——这与 `Effort::None`（请它不思考）是两件事，所以不写成 `none`；二是这一行早于这个键。页面两种都不画强度，不猜。harness run 没有本城的请求，恒缺席。
- **被否：读这次 run 的第一个提交的 `effort`**（`CommitAttribution`）。还没提交过的 run 说不出强度，而提交上的那个值写的是 `Effort::None` 兼指「没说」，读回来分不清。**被否：读 `model_selected`**：那是房间的选择，后来的派活可以在帧上另带强度，房间的选择不等于这次 run 用的那一个。
- 验收：`record::run` 的 `a_run_started_line_records_the_effort_it_froze`（写出、读回、缺席不写）。
-/

/-!
### 8-81 远程门的五种事件（`kernel::event::record::remote`，形状 2 值类型）

```rust
pub struct RemoteOpened { pub closes_at: TimeMs, pub url: String }
pub struct RemoteClosed { pub why: RemoteClosing }
pub enum RemoteClosing { Console, Locked, Expired }          // 线上 "console" | "locked" | "expired"
pub struct DevicePaired { pub device: String, pub name: String, pub authority: String }
pub struct DeviceRevoked { pub device: String, pub name: String }
pub struct RemoteSessionStarted { pub device: String, pub expires: TimeMs }
```

- **五种都是 record-only，都写在城自己名下**（`run` 为 `RunId::CITY`，无 `addr`）：谁能从外面够到这座城不决定任何一次模型请求的字节。写它们的是装配层的远程门（`crates/sprawling/Spec.lean` §8-139），经 worker 的 relay 进同一个写者。`who` 是 `person`，只有门到时自己关上那一条是 `city`。
- **追加在 `ALL` 末尾，次序就是一扇门一生的次序**：开、关、配对、撤销、会话开始（表按种类排，入账的次序是开、配对、会话开始、撤销、关）。不写 `ig`：0.0.7 的读者不认识这五个种类，按 D10 的规矩报 `E_LOG_VERSION_UNSUPPORTED` 而不是跳过它们。
- **没有一行携带密钥、配对码或会话 id**：账本可以被任何人重放，带上它们等于把冒充一台设备的材料交给每一个读者。设备的公钥只在设备表里（`CityLayout::devices`）。
- **设备与权限用正文写**：`device` 是设备 id 的 base32 正文，`authority` 是 `watch` 或 `act`。两者的类型与拼写归 `remote_access::door`（crates/remote_access/Spec.lean §8-11），kernel 不依赖它，所以这里存它给出的字。

### 8-83 一份文档的一次写与一处修改提案的一生（`kernel::event::record::document`，形状 2 值类型）

```rust
pub struct DocumentWritten { pub at: Address, pub baseline: B3Hash, pub version: B3Hash, pub bytes: u64 }
pub struct ProposalOffered {
    pub doc: Address,          // 被提议修改的那份文档
    pub baseline: B3Hash,      // 提案所基于的版本（documents D3）
    pub start: u64,            // 那一版里被提议替换的半开字节区间
    pub end: u64,
    pub before: String,        // 那一段的文本，按那一版的编码解出
    pub after: String,         // 提议换成的文本
}
pub struct ProposalDecided { pub proposal: B3Hash, pub verdicts: Vec<SliceVerdict> }
pub struct SliceVerdict { pub slice: u32, pub verdict: Verdict }
pub enum Verdict { Accept, Amend { text: String } }   // 线上 "accept" | {"amend":{"text":…}}
pub struct ProposalWithdrawn { pub proposal: B3Hash }
```

- **四种都是 record-only，追加在 `ALL` 末尾**，次序是 `document_written`、`proposal_offered`、`proposal_decided`、`proposal_withdrawn`（D15）。不写 `ig`：0.0.7 的读者不认识它们，按 D10 报 `E_LOG_VERSION_UNSUPPORTED`。
- **`document_written` 是保存的回执**：写在城名下，载荷不携正文；同一条命令写下的每一行都带那条命令的 `idem`（accounting 的 `commanding::entrance`），页面凭它认出「这是我那一次」，并从 `version` 读到下一次保存的基线（`crates/wire/Spec.lean` §8-72）。`bytes` 是新版本的长度，不是改了多少。
- **区间是两个整数，不是 `documents::Span`**：kernel 不依赖 `documents`，读者经 `documents::Span::new(start, end)` 把它读回来，起点在终点之后的一行在那里被拒。
- **`before` 与 `after` 各至多 `documents::WINDOW_BYTES_MAX` 字节**：一张卡是一个窗口读得下的一段（documents D18）。判定在提出提案的那一处，kernel 只携带。
- **提案的身份不在载荷里**：它是提出它的 run 与上面六个值的摘要，由 `documents::Offer::id` 一处算出（documents D13）；`proposal_decided` 与 `proposal_withdrawn` 的 `proposal` 就是这个摘要。同一个 run 对同一版同一段提同一句话是同一个提案。
- **`SliceVerdict` 与 `Verdict` 住在 kernel**：账本记下的是人逐句的决定，`wire` 的决定帧与 `documents` 的合并规则都直接用这两个类型，没有第二份拼写。`slice` 是句子在这张卡上的序号（从 0 数起）；只有改动过的句子可以被点名，`Amend` 只对插入的句子成立（documents D16）。
-/

/-! D9 harness 的汇报与回答按本城的词入账

**决定**：`harness_reported` 与 `harness_answered` 的载荷用 kernel 自己的类型拼写（`HarnessReported`、`HarnessPermit`、`HarnessPermitKind`、`HarnessAnswered`、`HarnessStop`）。`agent_protocols::harness` 读 ACP 用的类型（`Update`、`PermissionAsk`、`Permit`、`StopReason`）不搬进 kernel，装配层把后者逐臂映成前者。

**理由**：账本一旦写下，拼写就不能再改（§8-4 三条不变量的第一条），ACP 的词却跟着上游走，两边只是今天恰好相同。把 ACP 的类型搬进 kernel，上游改一个词时要么改坏已落盘的行，要么让协议层读不懂新版本。映射是穷尽 `match`，上游多出一种汇报时，编译停在映射处，而不是静默丢掉。

**被否**：①载荷只记变体名与一段文字（`update: String`、`text: String`）：读「harness 说了什么」的视图要再解析一次字符串，工具调用的标题与状态会混成一段；②`agent_protocols` 直接改用 kernel 的类型：理由同上；③`InputKinds`、`ModelFacts` 那样把类型归 kernel、协议层再导出：那两者的值来自本城自己的读法，这里的值来自上游规格。
-/

/-! D11 一次调用的效果与呈现，在调用那一刻从登记读出、记进 `tool_called`

**决定**：`ToolCalled` 多记两个可缺席的键 `effect` 与 `render`，由回合的工具波从工具面的登记照录（§8-75(b)）；线上的 `Call.effect`、`Call.render` 读这两个键。

**理由**：一件工具是什么，权威是它的 `ToolMeta`；这份登记只在一次 run 的工具台上存在，折叠账本的读面够不到它，而楼的 MCP 工具与内置工具的登记每次 run 都可能不同。写在调用那一行上，读面读到的就是那一刻的事实，一件新工具进来不需要任何读者多写一臂。

**被否**：①读面按工具名穷尽匹配出呈现：每加一件工具都要改这个匹配，MCP 工具的名字读面根本不知道；②读面持一份工具登记，在折叠时查：登记随楼与 run 变，旧行会被今天的登记重新解释；③线上自定一个不含地址的投影枚举：同一规则的第二个权威（`crates/wire/Spec.lean` §8-0）。

**代价**：经工具台写下的每条 `tool_called` 多二三十个字节。

**重开参数**：有了每次调用由参数算出位置的函数时，`render` 的 `Diff.locations` 改记那次调用的位置。
-/

/-! D13 远程门的五种事件全是 record-only，载荷里不带钥匙

**决定**：远程门开、关、配对、撤销、会话开始各是一个种类（§8-81），五种都是 record-only，载荷只写时刻、地址、设备 id 的正文、名字与权限，不写公钥、配对码与会话 id；设备与权限以 `remote_access` 给出的正文存，kernel 不另立类型。

**理由**：这五件事回答的是「谁、何时、凭什么进了城」，人从历史里读的正是这个；它们不改变任何一次模型请求，所以不入窗。钥匙类的字节进了账本就随每一次导出、每一次重放流出去，而它们在设备表里已经有一个家。kernel 不依赖 `remote_access`，为权限另立一个 kernel 枚举就是同一件事的第二份定义。

**被否**：①一个 `remote_door` 种类、载荷里带一个 `what` 字段：读者要先解载荷才知道发生了什么，`EventKind` 的穷尽匹配也看不见它；②在 `device_paired` 里存设备公钥，让设备表可以从账本重建：撤销之后公钥仍留在历史里，而设备表本来就原子写盘；③在 kernel 里定义 `DeviceAuthority`：与 `remote_access::door::Authority` 两处定义同一组值。

**重开参数**：账本需要证明「这一帧是哪台设备发来的」时（例如设备发出的命令要在账本上署设备的名），重议会话 id 是否入账。
-/

/-! D14 进程死后的 run 冻成 `cancelled`，载荷记下原因，不加第四种结局

**决定**：重开时的启动扫描为每一次有 `run_started`、没有 `run_frozen` 的 run 写一行 `run_frozen`，结局是 `cancelled`，载荷带 `cause: process_died`（§8-82-2）。`Completion` 仍是三种。

**理由**：`Completion` 的三种结局是 run 驱动自己能得出的判定，读它们的地方（视图的 `completion`、居民的冻结计数、`runtime::run` 的收尾）都按三种穷尽匹配。进程死亡不是一个 run 自己得出的判定，它是城重开时发现的事实；它在三种之中最接近 `cancelled`——停下了、没做完、不是上限截的。把「谁让它停的」放在载荷的一个可缺席的键里，读结局的地方一行不改，要区分死亡的读者读这个键。

**被否**：①加第四种结局 `Lost`：每个穷尽匹配 `Completion` 的地方都要多一臂，而 `RunFrozen::of` 的「三种结局，第四种不可表示」正是要守住结局的集合由 run 的驱动定；②不写冻结、让视图把没有冻结行的 run 读作死掉的：服务中的城与重开的城读同一份账本会得出不同的答案，而 ARCHITECTURE §13.7 的 `Lost --> Frozen` 要的是账本上的一行；③冻成 `limit`：那是「被上限截断」，读者会去找一个不存在的上限。

**重开参数**：出现一种由 run 自己得出、又不属于三种的结局时，重议 `Completion` 的集合，那时 `process_died` 也一并重议它属于哪一种。
-/

/-! D15 一份文档的一次写是一个种类，修改提案的一生是三个种类，都是 record-only

**决定**：经页面写一份任意文档记 `document_written`，修改提案提出、决定、收回各记一行（§8-83）。`document_written` 与 `proposal_decided` 写在城自己名下（`run` 为 `RunId::CITY`，无 `addr`），`proposal_offered` 与 `proposal_withdrawn` 写在提出它的 run 名下（`addr` 是那次 run 的房间）；四种都不入窗，追加在 `ALL` 末尾，不写 `ig`。

**理由**：现有三种写文档的事件各只说一类文件（四份 spine 文档、三份治理文档、两份规矩文件），载荷里的 `which` 是那一类文件的名字；用它们记一份任意文档的写，`which` 就得装一个路径，读者分不出哪一行在说规矩。一次保存要回答的是「哪份文档、从哪一版到哪一版、多大」，这正是 `documents` 的版本身份（documents D3）给出的三件事，所以载荷记两枚摘要而不记正文：正文在盘上，账本记的是这件事发生过。提案不同：提案的文本是 run 说出来的话，与 `tool_called` 的参数同类，文档往后怎样改都不会把它留在盘上，所以 `before` 与 `after` 都在载荷里，重开的城只读账本就能把一张没决定的卡原样折回来。四种都不决定任何一次模型请求的字节。

**被否**：①一个 `document_changed` 种类、载荷里用 `cause` 分保存与提案：决定与写是两件事，一次决定可以什么都不写（整张拒绝），一次写可以不是决定（`PutRange`）；②提案的文本只存进内容库、载荷记地址：内容库没有回收前这样省下的只是账本行的长度，而折叠就要多读一次内容库，账本也不再自足；③提案身份用随机 id 或账本 `seq`：kernel 恒不生成随机值，而 `seq` 要等落账之后才知道，写者在落账之前就要把它交给 run。

**重开参数**：账本行的长度成为开城时间里看得见的一段时，提案文本改存内容库；出现第二种写文档的入口（例如居民的 `edit` 也要记版本）时，重议 `document_written` 的写者。
-/

/-! D20 耗时记整数微秒，作为可缺席的键挂在既有的行上，不另立种类，账本版本不进位

**决定**：`tool_result` 多一把 `took_us: Option<u64>`，是这次工具执行从开始到给出答案的整数微秒；`model_returned` 多两把 `first_us: Option<u64>` 与 `took_us: Option<u64>`，是从请求发出到首个非空内容、到回复收齐的整数微秒。三把键都是 `#[serde(default, skip_serializing_if = "Option::is_none")]`。读数由注入的时钟给出：`bin::assembly` 仍是唯一的采样点，它交给运行层的时钟同时带墙钟毫秒与单调计时，量程两端都在同一个时钟上读；kernel 不采样。读者显示耗时时先读微秒键，缺席时退回两个信封时刻 `t` 之差（毫秒），并照实标出单位。

**理由**：账本的时刻是整数毫秒（ARCHITECTURE §10 规则 6），毫秒时刻相减分不出 1 ms 以内的差别，所以耗时要一个单独的量；记成时长而不是第二个更细的时刻，是因为墙钟会被系统调整，单调计时的差才是耗时，而单调计时的绝对值在两次开城之间没有意义，不能入账。挂在既有的行上与 §8-75 的 `first_at` 同一个理由：旧行与没量到的行字节不变，读宽即可重放，`EVENT_LOG_V` 不为一个可缺席的键进位。载荷不放浮点（确定性七条之 6），故用整数微秒而不是小数毫秒。

**被否**：①新种类 `tool_timed`：每次工具调用多一行，读者要把两行配对，而这个数只属于那一个结果；②把信封的 `t` 改成微秒：全账本换单位，旧行与新行的 `t` 不可比，等于换账本版本；③记纳秒：工具与模型的耗时在微秒以上，纳秒只让数字变长。

**重开参数**：出现一个读者需要工具内部分段（门、路径解析、IO、秘密扫描、检查点）的入账读数时，重议是否在 `tool_result` 里记分段，或只在仪表里记（今天只在仪表里记）。

写方：`runtime::turn` 从 `RunHooks.monotonic_us` 读三次单调读数（工具开始与答复、尝试发出、首个内容、回复收齐），差即这三把键；它与线上的 `Call`、`Used` 帧同一次 `WIRE_V` 进位（`crates/wire/Spec.lean` D22）。
-/

/-! D21 会话中改运行策略是一个入窗的种类 `run_policy_changed`，在下一个安全点生效

**决定**：加一个种类 `run_policy_changed`（`EventKind::RunPolicyChanged`，追加在 `ALL` 末尾，不写 `ig`），载荷 `RunPolicyChanged { policy: RunPolicy, by: Who }`，`addr` 是房间，`run` 恒为 `RunId::CITY`：这一行是人经城做的事，由 accounting 的 `record_at` 这一扇城自己的门写下；正在跑的 run 按 `addr` 认出它，不按 `run`。写方是城：收到线上的 `Command::ChangeRunPolicy { room, policy, idem }` 时写这一行；正在跑的 run 在它下一个安全点（与 Steer 同一扇门）读到它，从那一步起按新策略过门，并在下一段消息的末尾追加一句说明，冻结的前缀不动。下一次 run 的 `run_started.policy` 取房间最后一次改过的策略。窗类：入窗，因为那一句说明决定下一次模型请求的字节。模型与思考强度在会话中不变（roadmap A15）。

**理由**：改策略是城里发生过的事，重放要能说出某一步是在哪个策略下过的门；把它记在 `run_started` 里只够说一次 run 开头的策略。生效点放在安全点而不是立即，是因为一个工具波已经按旧策略过了门，半途换门会让同一波的两次调用被不同的规则判。工具定义不随策略变：会话开始时工具清单定成各模式的并集（`crates/runtime/spec/Catalog.lean` D25），所以改策略只动门与那一句说明，提示缓存不失效。

**被否**：①复用 `autonomy_changed`：它回答的是「谁答设计问题」，作用域是城或楼，与一次 run 的纪律是两件事；②只改偏好文件、不入账：重放读不到，门的判决就没有来历；③立即生效：同一波工具被两套规则判。

**重开参数**：出现会话中改模型或思考强度的需求时，重议是否把它们并进同一个种类（今天它们在会话中固定，因为换模型使缓存整段失效）。

载荷在 `kernel::event::record::run`（`RunPolicyChanged`，`by` 经 `Who` 的拼写）；窗类在 `spec/Event/Kind.lean` 记作 `InWindow`。门一侧（安全点读它、追加那一句、按新策略过门）尚无实现，今天写下这一行的只有 `Command::ChangeRunPolicy`。
-/

/-! D22 会话的显示名是一个 record-only 种类 `session_named`；地址仍是身份，标签仍在偏好文件里

**决定**：加一个种类 `session_named`（`EventKind::SessionNamed`，record-only，追加在 `ALL` 末尾），载荷 `SessionNamed { began: Seq, name: String }`：`addr` 是房间，`began` 指认这段 session（与 `wire::SessionLine.began` 同一个值），`name` 是人给的显示名，空串即撤回显示名、退回地址。写方是城，收到 `Command::NameSession { room, began, name, idem }` 时写，名字经 `wire::CarriedName` 的规则拒空白与控制字符。同一段 session 以最后一行为准。标签不入账：它们仍是偏好文件里的 `tags`（`crates/wire/spec/Preference.lean` D21）；没有标签时读者按工作区给一个默认标签，这是读法，不是记录。

**理由**：显示名是这段 session 在城里叫什么，换一台设备、导出这座城、让居民在对话里提到它，读到的都该是同一个名字，所以它属于城的历史；标签是人怎么归类自己的工作，D21 已经给了它一个家。显示名不改地址：地址决定读写域与账本身份，改它等于搬家。

**被否**：①把显示名也放进偏好文件：导出的城与远程设备看不到它；②改房间地址：读写域、历史与引用全部要迁移；③一个 `session_labelled` 种类同时记名字与标签：与 D21 冲突，人的分类会随导出给下一个人。

**重开参数**：居民需要按显示名找一段 session 时，重议名字的唯一性（今天同一房间两段可以同名）。
-/

/-! D23 skill 上架时的审核是一个 record-only 种类 `skill_audited`；调用记录不加种类，从已有的行折出

**决定**：加一个种类 `skill_audited`（`EventKind::SkillAudited`，record-only，追加在 `ALL` 末尾），由城在一个 skill 落位之后、以及扫架时看见它此刻的整包摘要没有审核时写（每个 `(skill, digest)` 在一个城进程里只发起一次；何时发起、问谁、怎样读回答由 `crates/city/spec/Library/Audit.lean` city D19 规定），`run` 为 `RunId::CITY`，`addr` 是书架所在的 scope。载荷：

```rust
pub struct SkillAudited {
    pub skill: String,                 // skill 名
    pub digest: B3Hash,                // 被审的那份内容的摘要；内容变了摘要就变，旧审核不再算数
    pub source: AuditSource,           // SkillsSh | SkillSpector
    pub scanner: String,               // 审核方报出的版本或名字；SkillsSh 时是合作方名，如 "socket"
    pub verdict: AuditVerdict,         // Pass | Warn | Fail | Unreachable
    pub risk: Option<String>,          // 审核方给的风险等级原词
    pub audited_at: Option<String>,    // 审核方给的审核时刻原文
    pub link: Option<String>,          // 取不到时给 skills.sh 上这个 skill 安全页的链接
}
```

`Unreachable` 是「这次问了、没审成」：skills.sh 不答、答 401／403、超时或答复读不出，SkillSpector 的退出码既不是 0 也不是 1；不拦上架（D89：skills.sh 默认、SkillSpector 可选）。没有一个审核方适用时（本地路径或自带的 skill，且 PATH 上没有 SkillSpector）不写行：没人问过就不该有一行说问过。skills.sh 每个合作方一行，`scanner` 是合作方名。使用记录不入新行：`describe` 取指南与读 skill 目录下的文件都已在 `tool_called` 里，按 skill 折叠的视图由 accounting 从这些行折出；MCP 的使用同样从 `call` 与 MCP 工具调用的 `tool_called` 折出。

**内容版本另有一行**：skill 页的「内容版本（摘要、时刻、谁改的）」读一个计划中的 record-only 种类 `skill_shelved`：载荷 `SkillShelved { skill: String, digest: B3Hash, source: ShelvedFrom }`，`ShelvedFrom { Path, Git { url, rev }, SkillsSh { name }, Shipped, Page }`，`addr` 是书架所在的 scope，`run` 为 `RunId::CITY`；`InstallSkill`（wire D32）与 `PutShelved` 每次真正落位（`Placed::Fresh`）时由城写一行，答 `AlreadyShelved` 时一个字节没变、不写，`digest` 是 `Installed::hash`。它取代此前为 `PutShelved` 计划的 `shelved_document_written`：两条上架的路是同一件事，一个种类；种类表的行与 `InstallSkill` 的执行者同一次改动落地。User 在架外直接改文件没有行，由扫架时架上摘要与这件 skill 最近一行 `skill_shelved` 的 `digest` 不同读出，页面把这一版的「谁改的」显示为「在架外」。

**理由**：审核的结论要和它审的那份内容绑定，才能说「内容变了要重审」，所以摘要必须在同一行；审核来自城外，结论是城收到的事实，重放不能再去问一次外网，所以入账。使用早已入账，再记一行只是同一件事的第二份。

**被否**：①每次读 skill 记一行 `skill_used`：与 `tool_called` 是两份定义；②审核结果只缓存在保留子树的文件里：重放与导出读不到，「谁在什么时候审过哪一版」没有历史；③取不到审核就拦上架：skills.sh 的无令牌接口随时可能关，拦住会让书架整个不能用。

**重开参数**：审核来源超过两个、或要按 skill 记多个合作方的分项结论时，重议 `scanner` 与 `verdict` 是否改成一张表。
-/

/-! D32 同步 `send` 的一次等待是一对种类 `signal_wait_started`／`signal_wait_ended`，都入窗，种类表的行与实现同一次改动落地

**决定**：两个种类（追加在 `ALL` 末尾，入窗：模型下一次调用要读到「等过、为什么停」），写方都是 collab 的发信门（collab D9），`run` 是等待的那个 run，`addr` 是它的房间：

```rust
pub struct SignalWaitStarted { pub on: Address, pub signal: SignalId, pub deadline_ms: u64 }   // 注入时钟上的 deadline
pub struct SignalWaitEnded { pub signal: SignalId, pub by: WaitEnd }
pub enum WaitEnd { Reply { reply: SignalId }, Timeout, Left }   // 回信到了｜到了 deadline｜run 离开房间
```

- `signal_wait_started` 紧跟在它那一封信的 `signal_enqueued` 之后写，`signal` 指那封信；一个 run 同一时刻至多一个没配对的 `started`（模型 `Collab.Delivery` 的 `waits` 每个 run 一条）。
- 每一行 `started` 恰有一行 `ended` 与它配对：回信到了写 `Reply`（`reply` 是那封回信），注入的时钟过了 deadline 写 `Timeout`，run 在等待中离开房间（取消、失败、进程死后冻结）写 `Left`；`waits_end` 保证没有第四种。进程死后的那一行由冻结它的那一处补写（D14 同一口径），所以重放不会留下一个永远在等的 run。
- `deadline_ms` 是注入时钟上的读数，不是墙钟：重放按 seq 读次序，不按时刻；线上把它换成墙钟时刻（wire D34）。
- 等待期间 run 不调模型、不写 `model_returned`，watchdog 读到一个未配对的 `started` 就知道这个 run 是停着而不是卡住，不对它退避或报警。

**现状**：两个种类已在 `crates/kernel/spec/Event/Kind.lean` 的表里与 `EventKind` 里（`ALL` 末尾，入窗），上面三个载荷类型在 `kernel::event::record`（`crates/kernel/src/event/record/collaboration.rs`；`WaitEnd` 线上以 `end` 键区分三臂），还没有写方；写方是 collab 的发信门（collab D9），读者在那之前把两个种类当作没有内容的行跳过。

**理由**：一个停着的 run 必须在账本里看得出来，否则 `status`、页面、watchdog 与重放都分不清它在等还是卡住（`crates/collab/spec/Delivery.lean` 的 `wait_bounded`、`waits_end` 说的是模型，账本要能把同一件事说给读者）。开始与结束分两行，是因为等待跨越安全点、可能跨越进程死亡；只记一行「等过多久」要等结束才写，中间那段时间什么都看不见。入窗，是因为下一次模型调用要知道它是被回信还是被超时叫醒的。

**被否**：①不入账、只在内存里停：重放与重开的页面看不到一个正在等的 run；②在 `signal_enqueued` 的载荷上加 `wait` 字段：说得出开始，说不出结束；③用 `run_policy_changed` 之类的已有种类表示停住：一个种类两种意思。

**重开参数**：一个 run 可以同时等两封回信时，重议「至多一个未配对的 `started`」。
-/

/-! D24 `asset_archived` 在 `archive record` 被调用时写，不等 run 冻结

**决定**：`asset_archived` 的写方从 run 收尾挪到工具波：`archive record` 执行时在同一个波里写这一行，同一个 run 的 `archive recall` 就读得到它。载荷与种类不变。`recall` 答复里的 `searched` 是这次真正比对过的条目数。

**理由**：测试城里同一个 run 的 `record` 在 seq 251、276，`asset_archived` 在 seq 324–325，晚于 `run_frozen`（seq 321），于是 recall 答 `searched: 0`。工具说「已记下」时事情就该已经发生（ARCHITECTURE §5 第 4 步：效果先成为事件）。

**被否**：保留冻结时写、让 recall 另读本 run 的待写表：同一件事在两处有状态，重开的城读不到待写表。

**重开参数**：归档要与工作树检查点同进退（例如一次被回滚的 run 不该留下归档）时，重议写入时刻。
-/
