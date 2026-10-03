-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::event::kind

规定 `kernel::event::kind`（`crates/kernel/src/event/kind.rs` 与 `crates/kernel/src/event/kind/window.rs`）：事件种类的闭集与它们的窗类。本分部是 §8-4 那张表的家。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-4 EventKind 全集与二分（specalign 数据面）

`kernel::event` 的接口在 `crates/kernel/spec/Event.lean` §8-4；本分部是那一节的表。`inductive EventKind` 是全集，`EventKind.windowClass` 是二分，每一臂上方的注释说这一种行携带什么、为什么落在这一类。二分依据唯一：该事件载荷是否决定模型请求字节；不存在第三类。
-/

namespace Kernel.Event.Kind

/-- 事件种类的闭集，与 `kernel::EventKind` 逐变体同名，`cargo xtask gates specalign` 双向对账；线上拼写是变体名的蛇形小写（serde 的 `rename_all = "snake_case"`）。注释行是 Rust 里的分组。新种类追加在 `ALL` 末尾；本 crate 不标 `#[non_exhaustive]`（`crates/kernel/Spec.lean` §8），新增一个种类在每个没处理它的读者那里是编译错误。 -/
inductive EventKind where
  -- 创世与空间
  | CityInitialized
  | BuildingCreated
  | BuildingConfigured
  | BuildingRemoved
  -- 基集
  | SessionOpened
  | RunStarted
  | RunForked
  | PromptAssembled
  | PromptShapeCompared
  | ModelCalled
  | ModelReturned
  | ToolCalled
  | ToolResult
  | ResultOffloaded
  | GateChecked
  | GateDenied
  | CheckpointCommitted
  | HandoffWritten
  | SteerReceived
  | CancelReceived
  | WatchdogFired
  | BudgetLimit
  | RunFrozen
  | LogTruncated
  -- 协作
  | SignalEnqueued
  | SignalConsumed
  | DraftHeld
  | DraftResolved
  | GoalRegistered
  | GoalConflict
  | ArbitrationVerdict
  | RepairStarted
  | RepairReused
  | WorktreeOpened
  | PrOpened
  | PrMerged
  | PrRejected
  | RoadmapClaimed
  | RoadmapFinished
  | RoadmapReleased
  | RoadmapSplit
  | RoadmapBlocked
  | PursuitChanged
  -- 治理与设施
  | ApprovalRequested
  | ApprovalResolved
  | PolicyCreated
  | PolicyRevoked
  | TaintPromoted
  | CrossBuildingTransfer
  | CityHalted
  | BackpressureShed
  | DigestInvalidated
  | EndpointAttached
  | EndpointProbed
  | EndpointLost
  | ModelSelected
  | ProviderDegraded
  | LoginStarted
  | EvalRun
  | AssetArchived
  | CredentialLent
  -- 隐私与 Discard
  | SecretCaptured
  | SecretEgressBlocked
  | FileDiscarded
  | DiscardRestored
  | AutonomyChanged
  -- 统一历史
  | WentBack
  | FileRestored
  -- 治理与设施
  | GovernedDocumentWritten
  | SpineDocumentWritten
  | RulesChanged
  | ToolkitLinkOpened
  -- 供应商与模态
  | EmbeddingCalled
  | RerankCalled
  -- 顾问
  | AdviserAsked
  | AdviserAnswered
  | AdviserFellBack
  -- 保温
  | CacheRenewed
  -- harness 居民
  | HarnessReported
  | HarnessAnswered
  -- 远程门
  | RemoteOpened
  | RemoteClosed
  | DevicePaired
  | DeviceRevoked
  | RemoteSessionStarted
  -- 文档
  | DocumentWritten
  | ProposalOffered
  | ProposalDecided
  | ProposalWithdrawn
  -- 会话与书架
  | RunPolicyChanged
  | SessionNamed
  | SkillAudited
  -- 等回信
  | SignalWaitStarted
  | SignalWaitEnded
  deriving DecidableEq, Repr

/-- 一个种类的载荷决不决定模型请求的字节：入窗（`InWindow`）或只入账（`RecordOnly`）。与 `kernel::WindowClass` 逐变体同名。依据唯一，不存在第三类。 -/
inductive WindowClass where
  | InWindow
  | RecordOnly
  deriving DecidableEq, Repr

/-- 二分的唯一权威（`EventKind::window_class`）。每一臂一行，`specalign` 逐臂与 kernel 编译出来的 `window_class()` 对账；臂上方的注释是这一种行携带什么、为什么落在这一类。 -/
def EventKind.windowClass : EventKind → WindowClass
  -- 创世与空间
  /- 创世行，prev＝64 个 0 -/
  | .CityInitialized => .RecordOnly
  | .BuildingCreated => .RecordOnly
  | .BuildingConfigured => .RecordOnly
  /- 人把一栋楼移出城：载荷携 addr 与 kept——文件搬到 reserved subtree 下的哪里；不删一个字节，楼写过的每一行留在账里 -/
  | .BuildingRemoved => .RecordOnly
  -- 基集
  /- 新的一段从哪里开始；`prompt_assembled` 记的是它之后给了那一跑什么 -/
  | .SessionOpened => .RecordOnly
  | .RunStarted => .RecordOnly
  | .RunForked => .RecordOnly
  | .PromptAssembled => .InWindow
  /- 一次请求的缓存形态与归因：量请求、不裁字节；段哈希仍由 `prompt_assembled` 独家记账 -/
  | .PromptShapeCompared => .RecordOnly
  | .ModelCalled => .InWindow
  | .ModelReturned => .InWindow
  | .ToolCalled => .InWindow
  | .ToolResult => .InWindow
  | .ResultOffloaded => .InWindow
  | .GateChecked => .RecordOnly
  | .GateDenied => .RecordOnly
  | .CheckpointCommitted => .RecordOnly
  | .HandoffWritten => .RecordOnly
  | .SteerReceived => .InWindow
  | .CancelReceived => .RecordOnly
  | .WatchdogFired => .RecordOnly
  | .BudgetLimit => .RecordOnly
  | .RunFrozen => .RecordOnly
  | .LogTruncated => .RecordOnly
  -- 协作
  | .SignalEnqueued => .RecordOnly
  | .SignalConsumed => .InWindow
  | .DraftHeld => .RecordOnly
  | .DraftResolved => .RecordOnly
  | .GoalRegistered => .RecordOnly
  | .GoalConflict => .RecordOnly
  | .ArbitrationVerdict => .RecordOnly
  | .RepairStarted => .RecordOnly
  | .RepairReused => .RecordOnly
  | .WorktreeOpened => .RecordOnly
  | .PrOpened => .RecordOnly
  | .PrMerged => .RecordOnly
  | .PrRejected => .RecordOnly
  | .RoadmapClaimed => .RecordOnly
  | .RoadmapFinished => .RecordOnly
  | .RoadmapReleased => .RecordOnly
  | .RoadmapSplit => .RecordOnly
  | .RoadmapBlocked => .RecordOnly
  | .PursuitChanged => .RecordOnly
  -- 治理与设施
  | .ApprovalRequested => .RecordOnly
  | .ApprovalResolved => .RecordOnly
  | .PolicyCreated => .RecordOnly
  | .PolicyRevoked => .RecordOnly
  | .TaintPromoted => .RecordOnly
  | .CrossBuildingTransfer => .RecordOnly
  | .CityHalted => .RecordOnly
  | .BackpressureShed => .RecordOnly
  | .DigestInvalidated => .RecordOnly
  | .EndpointAttached => .RecordOnly
  | .EndpointProbed => .RecordOnly
  | .EndpointLost => .RecordOnly
  | .ModelSelected => .RecordOnly
  | .ProviderDegraded => .RecordOnly
  /- 旧版本的订阅登录写过它；本版本不写，保留这一种只为旧账本读得回，载荷不再有读者 -/
  | .LoginStarted => .RecordOnly
  | .EvalRun => .RecordOnly
  | .AssetArchived => .RecordOnly
  | .CredentialLent => .RecordOnly
  -- 隐私与 Discard
  /- 行内无明文无哈希前缀 -/
  | .SecretCaptured => .RecordOnly
  | .SecretEgressBlocked => .RecordOnly
  | .FileDiscarded => .RecordOnly
  | .DiscardRestored => .RecordOnly
  | .AutonomyChanged => .RecordOnly
  -- 统一历史
  /- 回到过去：名为 name 的新树起于提交 point；干线不动。与 `worktree_opened` 分开，因为只有这条说出这棵树停在历史的哪一点 -/
  | .WentBack => .RecordOnly
  /- 从 point 取回 path 到名为 name 的树；point 上没有这个文件即删掉它。撤销就是追加这一条，账本不删任何行。path 取 git 树的写法（`/` 分隔、相对），不随写下它的机器变 -/
  | .FileRestored => .RecordOnly
  -- 治理与设施
  /- 人写下治理这座城的三份文件之一，载荷携 which 与字节数，恒不携正文——正文在盘上，账本记的是这件事发生过 -/
  | .GovernedDocumentWritten => .RecordOnly
  /- 人写下某楼自己的 spine 文档之一，载荷携 building、which 与字节数，恒不携正文。与上一行分开是因为这几份有第二个写者，写入携起手正文并可能被拒 -/
  | .SpineDocumentWritten => .RecordOnly
  /- 一次派发所站的规则文档之一换了内容（城的 `CONFIG.toml`，楼的 `CONFIG.toml` 与 `RULES.toml`）：载荷携 scope、which（`RULES.toml`／`CONFIG.toml`，枚举 `GoverningDocument`）、前后两枚摘要与字节数，恒不携正文。在准入（`agree_to_work`）之后、第一次读规则之前落账——先落账再生效；准入拒绝的派发与不存在的楼不记。`before` 缺席即开账行；本行的 `after` 等于同一文档下一行的 `before`，断链本身说明有人绕过一切门改了文件 -/
  | .RulesChanged => .RecordOnly
  /- 人请求接入一个外部应用，载荷只携 slug。**恒不携站位**——那是关于此刻的事实（`crates/wire/Spec.lean` §8-35b）；**恒不携 consent URL**——那是一张能力凭证，记进可重放的账本等于发给每一个重放的人 -/
  | .ToolkitLinkOpened => .RecordOnly
  -- 供应商与模态
  /- 一次嵌入调用入账：模型、请求多少条、回来多少个向量、调用方要的维度与 provider 自报的 token。向量本身不在此处——与 `model_called` 不携请求体同理，它是可从记录的输入重算的派生值，存两份就是同一件事有两个家 -/
  | .EmbeddingCalled => .RecordOnly
  /- 同形：passages 与 ranks 各记一个数。服务端排好的名次就是答案，不在本城重排，故不在此处再写一份序 -/
  | .RerankCalled => .RecordOnly
  -- 顾问
  /- 问了一次判断：问题种类（noul／score／choice）与对象。问本身不决定任何字节 -/
  | .AdviserAsked => .RecordOnly
  /- 重放不再问顾问，读到的是这条答案：它决定一件东西留不留在窗口里，因此它决定模型请求字节 -/
  | .AdviserAnswered => .InWindow
  /- 没有可用的顾问答案，确定性策略作答，reason 是 `unavailable`／`timeout`／`unreadable` 之一。没有这条，顾问塑形的窗口与城自己策略塑形的窗口会折出同一段历史 -/
  | .AdviserFellBack => .RecordOnly
  -- 保温
  /- 保温续期一次入账，写在房间地址下：成功时携 provider 自报的四个 token 数与它自报的账单额，失败时携 provider 的拒绝原样（`AxError`）。续期只重发前缀、不改变任何一次请求的字节，所以不入窗；记下它是为了让人从历史里读出保温花了多少 -/
  | .CacheRenewed => .RecordOnly
  -- harness 居民
  /- 官方 harness 在一次 run 里汇报的一件事：回答或推理的一段、它开始的一次工具调用与状态、它问的许可与城的答；harness 说了之后才落账，恒不被读回来当作城的判定 -/
  | .HarnessReported => .RecordOnly
  /- harness 对城那次 prompt 的回答：停止原因与这一回合它回答城的文字；停止原因一到就写，再冻结。`end_turn` 而文字非空时，它是 `Completion::Done` 的证据（§8-20） -/
  | .HarnessAnswered => .RecordOnly
  -- 远程门
  /- 人在控制台开了远程门：到何时自己关上（`closes_at`），以及设备打开的 `https://` 地址（`url`）。谁能从外面进城不决定任何模型请求字节（§8-81） -/
  | .RemoteOpened => .RecordOnly
  /- 远程门关了，`why` 说是谁关的：`console` 人在控制台、`locked` 持有会话的设备锁门离开、`expired` 开门的时长到了 -/
  | .RemoteClosed => .RecordOnly
  /- 一台设备出示了有效的配对码，城留下它：设备 id 的正文、人给它的名字、权限 `watch`／`act`。公钥不在此处，在设备表里 -/
  | .DevicePaired => .RecordOnly
  /- 人撤销了一台设备：设备 id 的正文与名字；它持有的会话随之结束 -/
  | .DeviceRevoked => .RecordOnly
  /- 一台已配对的设备握手证明了它的密钥，持有一段远程会话到 `expires`，不晚于门关上的时刻；会话 id 不在此处 -/
  | .RemoteSessionStarted => .RecordOnly
  -- 文档
  /- 经页面对城里任意一份文档的一次保存落下了：载荷携 `at`（文档的地址）、`baseline`（这次保存所基于的版本）、`version`（落下的新版本）与 `bytes`（新版本的字节数），恒不携正文。它是 `PutRange` 的回执，也是一次被接受的修改提案改写文档时的那一行；页面按载荷上的 `idem` 认出自己那一次（§8-83） -/
  | .DocumentWritten => .RecordOnly
  /- 一次 run 对一份文档的一处修改提案：载荷携 `doc`、`baseline`、`start`、`end`（那一版里被提议替换的半开字节区间）、`before`（那一段的文本）与 `after`（提议的文本）。提案的身份是这些值与 run 的摘要，由 `documents` 算出，不记在载荷里（§8-83） -/
  | .ProposalOffered => .RecordOnly
  /- 人对一张提案卡的决定：载荷携 `proposal` 与逐句的 `verdicts`；一个改动的句子不在 `verdicts` 里即被拒，`verdicts` 为空即整张拒绝。接受的部分改了文档时，同一次决定先写一行 `document_written` -/
  | .ProposalDecided => .RecordOnly
  /- 提出它的 run 收回一处还没决定的提案：载荷携 `proposal` -/
  | .ProposalWithdrawn => .RecordOnly
  -- 会话与书架
  /- 人在会话中改了房间的运行策略：载荷携 `policy`（新的 `RunPolicy`）与 `by`。正在跑的 run 在下一个安全点读到它，在下一段消息末尾追加一句说明，所以它决定模型请求的字节（`Record.lean` D21） -/
  | .RunPolicyChanged => .InWindow
  /- 人给一段 session 起的显示名：载荷携 `began`（开这段 session 的 seq）与 `name`；空串撤回显示名。地址仍是身份，名字只是页面显示的字（`Record.lean` D22） -/
  | .SessionNamed => .RecordOnly
  /- 一个 skill 上架时的一次审核：载荷携 `skill`、`digest`（被审内容的摘要）、`source`、`scanner`、`verdict`，以及可缺席的 `risk`、`audited_at`、`link`。审核方说了什么不进任何模型请求（`Record.lean` D23） -/
  | .SkillAudited => .RecordOnly
  /- 一个带 `wait` 发信的 run 停在安全点等回信：载荷携 `on`（等的房间）、`signal`（那封信）与注入时钟上的 `deadline_ms`。模型下一次调用要知道它停过，所以入窗（`Record.lean` D32） -/
  | .SignalWaitStarted => .InWindow
  /- 与一行 `signal_wait_started` 配对的结束：载荷携 `signal` 与 `by`（回信到了、过了 deadline、run 离开房间）。模型下一次调用要知道是哪一种叫醒了它（`Record.lean` D32） -/
  | .SignalWaitEnded => .InWindow

/-- 每个种类，依 `EventKind::ALL` 的次序。 -/
def EventKind.all : List EventKind := [
  .CityInitialized,
  .BuildingCreated,
  .BuildingConfigured,
  .BuildingRemoved,
  .SessionOpened,
  .RunStarted,
  .RunForked,
  .PromptAssembled,
  .PromptShapeCompared,
  .ModelCalled,
  .ModelReturned,
  .ToolCalled,
  .ToolResult,
  .ResultOffloaded,
  .GateChecked,
  .GateDenied,
  .CheckpointCommitted,
  .HandoffWritten,
  .SteerReceived,
  .CancelReceived,
  .WatchdogFired,
  .BudgetLimit,
  .RunFrozen,
  .LogTruncated,
  .SignalEnqueued,
  .SignalConsumed,
  .DraftHeld,
  .DraftResolved,
  .GoalRegistered,
  .GoalConflict,
  .ArbitrationVerdict,
  .RepairStarted,
  .RepairReused,
  .WorktreeOpened,
  .PrOpened,
  .PrMerged,
  .PrRejected,
  .RoadmapClaimed,
  .RoadmapFinished,
  .RoadmapReleased,
  .RoadmapSplit,
  .RoadmapBlocked,
  .PursuitChanged,
  .ApprovalRequested,
  .ApprovalResolved,
  .PolicyCreated,
  .PolicyRevoked,
  .TaintPromoted,
  .CrossBuildingTransfer,
  .CityHalted,
  .BackpressureShed,
  .DigestInvalidated,
  .EndpointAttached,
  .EndpointProbed,
  .EndpointLost,
  .ModelSelected,
  .ProviderDegraded,
  .LoginStarted,
  .EvalRun,
  .AssetArchived,
  .CredentialLent,
  .SecretCaptured,
  .SecretEgressBlocked,
  .FileDiscarded,
  .DiscardRestored,
  .AutonomyChanged,
  .WentBack,
  .FileRestored,
  .GovernedDocumentWritten,
  .SpineDocumentWritten,
  .RulesChanged,
  .ToolkitLinkOpened,
  .EmbeddingCalled,
  .RerankCalled,
  .AdviserAsked,
  .AdviserAnswered,
  .AdviserFellBack,
  .CacheRenewed,
  .HarnessReported,
  .HarnessAnswered,
  .RemoteOpened,
  .RemoteClosed,
  .DevicePaired,
  .DeviceRevoked,
  .RemoteSessionStarted,
  .DocumentWritten,
  .ProposalOffered,
  .ProposalDecided,
  .ProposalWithdrawn,
  .RunPolicyChanged,
  .SessionNamed,
  .SkillAudited,
  .SignalWaitStarted,
  .SignalWaitEnded
]

theorem EventKind.all_complete : ∀ kind : EventKind, kind ∈ EventKind.all := by
  intro kind
  cases kind <;> decide

theorem EventKind.all_nodup : EventKind.all.Nodup := by
  decide

/-- **入窗的种类恰是这十二种。** 它们的载荷决定模型请求的字节，所以重放与分叉要读它们；其余每一种都只入账。让一个种类改变窗类，要先改这条定理，而改它就是改模型请求的字节。 -/
theorem the_in_window_kinds :
    EventKind.all.filter (fun kind => kind.windowClass == .InWindow) =
      [.PromptAssembled, .ModelCalled, .ModelReturned, .ToolCalled, .ToolResult, .ResultOffloaded,
        .SteerReceived, .SignalConsumed, .AdviserAnswered, .RunPolicyChanged, .SignalWaitStarted,
        .SignalWaitEnded] := by
  decide

end Kernel.Event.Kind
