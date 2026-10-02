-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.collab.spec.Inbox
import crates.collab.spec.Fanin
import crates.collab.spec.Workshop
import crates.collab.spec.Triage
import crates.collab.spec.Claim

/-! # collab 的规格

`collab` 是「几个居民在同一栋楼里干活而不互相踩」的机械层。本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `collab D<n>`。
-/

/-! ## 1 需求分解

| 模块 | 回答的问题 |
|---|---|
| `inbox`、`steer` | 一条消息怎么从一个居民到另一个，而重复投递不变成重复副作用（`spec/Inbox.lean`；本文件 §8 的 steer 一条） |
| `workshop`、`fanin` | 一件活拆成多个节点后，谁按什么序跑、结果怎么收回来（`spec/Workshop.lean`、`spec/Fanin.lean`） |
| `pr` | 写代码的人不验自己的代码，由什么强制（`spec/Fanin.lean`） |
| `arbiter` | 两个居民不同意时，升到哪里（§8、D1） |
| `delegate_tool`、`handback` | 派一个代理去一个房间，以及它的结果怎么回到父房间（§8） |
| `signal_tool`、`goal_tool`、`workshop_tool` | 上面的机制怎么变成居民手里真能调的工具（§8） |
| `pr_tool` | 开 PR 与 worktree 为根的 run（§8） |
| `triage` | 一条外来信号该送到哪个 Address（`spec/Triage.lean`） |
| `claim_tool`、`claim_effect` | 认领、结项、拆分 `Roadmap.md` 上的一个节点，以及落地时哪一条效应要核盘上那份（`spec/Claim.lean`；§8、D6） |
| `archive_tool` | 把一条偏好、决定、更正或事实记上书架（§8、§14） |
| `citation` | 一条引文是否仍然指着被钉住的那份原文（§8） |
-/

/-! ## 2 验收标准

分部里的定理是模型对性质的证明：重复投递不产生第二件、急件 lane 只装 Steer、pull 不超 bandwidth；验证者不是生产者、验不过没有 Artifact、PR 的验证者不是实现者；只派就绪节点、一个节点不派两次；污染件不自己开工、判不出就交给人；分一行要先握着它、桌子答应的效应落在派活那份计划上全部落下、一行被别人动过则本 run 对它的拆分落不下。每个分部各有一条「拿掉守卫即反例」的定理（咬得动的演示）。

Rust 实现对模型的一致性由测试检查，不由证明：每个模块旁的 `tests.rs` 经生产入口断言它在本文件 §8 或分部里的规则；两条判负线（`Artifact` 无公开构造子、未验证的 PR 合不进来）由 `tests/ui/` 的编译失败反例钉住；`tests/pr_flow.rs` 从外面走一遍开 PR、被拒、合并。
-/

/-! ## 3 假设与歧义

一层深（delegate 值上没有 delegate 方法）已在 `kernel::delegation` 定下，本 crate 只把它当作前提，不重议。

**未定：验证节点。** 今天 `Handback::of` 收一个 `done_check_passed: bool` 与验证者的名字，装配层以 `CITY_VERIFIER` 为验证者（`crates/accounting/src/worker/dispatching/handback.rs`）。要定的是：验证是否改由另一地址上一个全新会话跑 `done_check` 后才铸 `Artifact`；验证者是否不得是图中任何实现者、改了代码须第三方再验；JOB 是否钉住被审文档的 Locator。能定下它的证据是一条测试：子 run 以 Done 结束而 `done_check` 失败时不铸 `Artifact`。
-/

/-! ## 4 现状分析

十七个模块（§1）。本 crate 消费 kernel 的判定面：`kernel::gate::spawn`（经 `delegate_tool`）、`kernel::goal::detect_conflict`（经 `arbiter`）、`kernel::delegation`、`kernel::PlanTree`（经 `claim_tool`）。生产消费者是 `crates/sprawling` 的装配层：`accounting::worker::collaborating` 按房间保存 join、图与目标表，工人把各张桌子借给工具，在一轮活落地时取走效应并写账。
-/

/-! ## 5 权威信源

「多 Agent」的语义（一层深、干预五动词、实现者不自测、为什么赌多 Agent 的六条及其判负条件）；`architecture.toml` 里 collab 那些条目与 ARCHITECTURE.md §9 的七形状；`crates/kernel/Spec.lean` 的 goal、repair、delegation 各节（§8-15、§8-16、§8-17）。
-/

/-! ## 6 命名统一

Signal、Inbox、Steer、Workshop、NodeContract、fan-in、Artifact、arbitration、Triage。概念名一律英文原词；该用什么词见 `docs/glossary.md`，不该用什么词见 `tools/xtask/lexicon.toml`。模型里的声明名取同样的词：`Collab.Inbox.Signal`、`Collab.Fanin.Artifact`、`Collab.Workshop.handNext`。
-/

/-! ## 7 模块边界

三件邻居的活，及它们各自的主人：

- **物理隔离归 `storage::worktree`**：本 crate 决定谁干什么、谁拿着哪份草稿；一节点一棵树与磁盘上限归 storage。
- **人的干预动词归 `wire`**：`Steer`、`Cancel`、`Halt`、`Release` 从人那侧进城已有入口；本 crate 的 `steer` 只管居民发给居民的那一条通道，两条通道入口不同而落点相同。
- **处置与监护归 `runtime::watchdog`**：停滞依据与纠正到冻结的升级梯在那里，本 crate 不建第二套。

**协作工具为什么住在本 crate**：一件工具是 `kernel::tool` 缝的适配器，而适配器必须能命名它暴露的机制。依赖法写着 runtime 只依赖 kernel、storage、gateway（ARCHITECTURE.md §3），runtime 恒不得指名 collab，所以 `Signal` 与 `GoalEntry` 的工具面拼不进 `runtime::tools`；放进装配层则把四百行判定塞进最脏的那个文件，且 citysim 测不到。故基础的三件工具在 runtime，协作工具在 collab。

**本 crate 无 I/O**，时间只作为参数进来。
-/

/-! ## 8 接口先行

签名的权威是 Rust 源码；这里记每个模块的形状与它为什么是这个形状。分部已规定的四个模块只记模型之外的要求。

**`collab::inbox`**（形状 2 值类型、形状 7 投影；性质见 `spec/Inbox.lean`）。`Signal::new`、`lane`、`enqueued_payload`、`consumed_payload`、`from_payload`；`Inbox::new(capacity, bandwidth)`、`deliver`、`pull`、`take_steer`、`pending`。

- 洪水交给 backpressure：`deliver` 返回 `kernel::Admission`，削峰判定住 `kernel::backpressure`，计数住队列；本模块不自定义第二套限流。
- 本 crate 没有 signal-unknown 码：载荷由本模块自写自读，kind 是穷尽枚举，一个本版本不认的 kind 只能来自更新的二进制写的 Ledger，那已由版本方向门（`E_LOG_VERSION_UNSUPPORTED`）拒在外面；同一个不认的 kind 在写入面也拼不出来（`SignalKind::parse` 拒它，报 `E_INVALID_ARGS`）。
- 两条线上形状各有一个 serde 结构，住 kernel 的 `SignalEnqueued` 与 `SignalConsumed`，写经 `Payload::of`、读经 `Payload::read`；`lane` 写出去给 crate 外的读者，回读时不采信，缺席即照旧推出。
- `Signal::from_payload` 是 `enqueued_payload` 的逆，否则投影重建要长出第二份 Signal 解析器。重建先筛后送：从 Ledger 收齐 `signal_enqueued` 与 `signal_consumed` 两组 id，只把未被消费的按原序 `deliver` 一遍，所以队列不需要「按 id 删除」。
- 读不回来的载荷在 `take_steer` 这条路上丢弃而不报错：调用点是一次 run 的安全点，除了继续，唯一的替代是为别人的一条损坏条目停掉这一跑；同一条载荷在 `pull`（模型自己那扇门）上仍大声报错，事实不会消失。

**`collab::steer`**（形状 2 值类型）。`Steer::from_person`（唯一能写出 `user` 前缀的构造子）、`from_signal`、`source`、`text`；`AgentSteer::new`、`signal`（居民侧只能走 Inbox）、`landing`（`@id`）。

- 两个入口、一个落点：人的 Steer 只从 control surface 进城，恒不走 Inbox；居民的 Steer 是一件插队首的 Signal。两者都追在下一次工具结果末尾，模型只需要认识一种形状。
- `AgentSteer` 的 source 由它自己的 id 拼成 `@id`，一件自称来自人的注入内容拼不出 `user`：入口分立是安全要求，类型把它变成判定。
- Steer 不打断动作：它在安全点被消费（`runtime::turn`）；同一边界上 Cancel 压过 Steer，因为停是不可撤销的那个。
- 中断源先问人、再问本屋 Inbox（`SignalDesk::take_steer`），人压过居民；只读人的命令队列的中断源会让 `Steer::from_signal` 与整个 `AgentSteer` 永远不发生。
- 属名就是回信地址：模型读到 `@market/hana:` 时，读到的既是「这句话不是人说的」，也是 `signal` 的 `to` 该填什么。

**`collab::workshop`、`collab::workshop::underway`**（形状 2、形状 1；性质见 `spec/Workshop.lean`）。`NodeContract` 带 id、goal、depends_on、reads、write_domain、owner、done_check、budget、stop；`job_text` 落盘即该节点的 JOB.md，契约本身就是任务权威，机制在 prefix 零常驻。goal、owner、done_check、stop 四个字段不许空：空的停止条件是一个不会停的 run。图的权威是 `Roadmap.md`，本模块不另设存储；从路线图行生成契约的那一步没有，因为路线图的表格不携 `depends_on`。

**`collab::workshop_tool`**（形状 4 适配器）。`WorkshopDesk::new(who, joined, handed)`、`lay_out`、`take_underway`、`question`、`judge`、`accept`；`WorkshopTool` 的动词是穷尽枚举 `Op`：`lay_out`、`question`、`judge`。

- workshop 是派生的扇出，不是第二条派生通路：每个节点都过 `DelegateDesk::ask`，一层深与准入两道门是同一段代码，工具的 `Effect` 也是 `Spawn`。
- 节点 id 就是它的房间地址，与 `Handback::node()` 同一取法。
- `lay_out` 只派 `Underway::hand_next(done)`，`done` 是这个房间的 join 已收下 Artifact 的节点。回答里 `schedule` 是整张图的序，`handed` 是这次派出的那一组，其余在 `waiting`。
- 下一组就绪集在 handback 到达时派出：摆出的 `Underway` 在 run 结束时由装配层收走，按房间与 join 并排保存（`Collaborating.workshops`）；一个节点的 handback 汇入父房间的 join 之后，装配层对同一个 `Underway` 调 `hand_next`，新就绪的节点按上一个兄弟节点的派法（同一个父 run、同一个 mode）派出；全部汇合后这张图删去。再摆同一张图仍然允许，桌子带着这个房间的已派集开张，在飞的节点不会再派一次，新的 `Underway` 取代旧的。图只在内存里：进程重启后由这个房间后来的某个 run 再摆一次，join 已收下的节点被跳过。
- 一个 run 一张图：第二次 `lay_out` 即拒，一个 session 里两张图是「这次在造什么」的两个答案。不认的动词被拒绝，不舍入到无害的那个。两张桌子取锁失败时的拒词只有 `poisoned(which)` 一个家。
- join 属房间而不属 run：子在父冻结之后才开，所以 `FanIn` 由装配层按房间保存，并与 Inbox 折自同一批 `signal_enqueued` 行。

**`collab::fanin`、`collab::pr`**（形状 2、形状 5 typestate；性质见 `spec/Fanin.lean`）。`pr_opened`、`pr_merged`、`pr_rejected` 的唯一权威是 `pr_tool::request`，它的键里有被审的 `commit`；`pr` 只有 `Open → Verified` 这一段判定。物理 merge 归 `storage::worktree`，只走 fast-forward，trunk 动过即退回重做。

**`collab::arbiter`**（形状 1 判定）。`Level` 有 `Serialize { after }` 与 `Arbitrate { with }` 两臂（D1）；`arbitrate(registered, candidate)`、`conflict_payload`。

- `goal_conflict` 的形状归 kernel：`conflict_payload` 把 `Level` 译成 `kernel::event::record::GoalConflict` 再经 `Payload::of` 写出；本模块只拥有「哪一级」。
- 检测进 kernel，仲裁不进：`kernel::goal::detect_conflict` 只答撞没撞，本模块答谁来仲裁。判序固定（机械在前、读在后），同一对目标恒落同一级，重放才可比。
- 机械可判的只有一种形状：双方都 claim 路径，且常设性一高一低，常设的先走。其余（两个常设、外部资源同名）都要读目标陈述，那是模型的活。

**`collab::delegate_tool`**（形状 4 适配器）。`Delegated { room, task, goal, kind }`；`DelegateDesk::new(depth, building)`、`beside`（同一 depth 与 building、未派任何活：图在 run 结束后派的节点过同样两道门）、`ask`（门在这里被叫）、`asked`（`status.children` 的真值）、`take`（回合落定后装配层取走）。

- 本模块是 `kernel::gate::spawn` 的生产调用者，一层深不是本模块的判定；拒绝文字是门自己的三段式。判决按 `GateOutcome` 两臂穷尽匹配，门多一种答复时编译器当场找到这里。
- 面向模型的那句话不提人：`disclosure` 说「要么给出房间，要么被拒」，`workshop` 的那一句同样。
- 深度是被携入的，不是推算的：一个自己推算深度的 run，错一次就是一个孙代理。
- 一次请求不是一个 run：工具答的是在哪个房间开，装配层在父回合落定后派活，子 run 的 `run_started` 携着那个房间。不新增 EventKind：父的 `tool_called{name:"delegate"}` 与子的 `run_started{addr}` 已经记了两遍。
- 代理不出楼：房间必须 `is_within` 父的楼，否则 `E_CROSS_BUILDING_DENIED`，并告知跨楼的正路是 `signal`。回程归 `handback`。

**`collab::handback`**（形状 1 判定）。`Handback::Finished(Artifact)` 与 `Stopped { claim, because }`；`of`、`by`、`node`（即子房间的地址）、`signal`、`from_signal`（`signal` 的唯一逆）。

- 父拿得到子的结果，而那不是下一回合：子 run 在父 run 冻结之后才开，所以「父的下一回合」实际是父房间的下一个 run；跨 run 递事实的门就是房间的 Inbox，回程走 `Signal`，`status.signals_pending` 自动报数。
- 城市做验证者，不是子自己：`Claim::verified` 拒生产者自验，而 `Completion::Done(Evidence)` 是城市观察到的事实。
- 一个拒不是一个错误：`of` 把 `verified` 的 `Err` 收成 `Stopped`，`because` 携拒词原文，因为在这条路上它是一个结果。不新增 EventKind：回程落在 `signal_enqueued` 里。
- `from_signal`：不是 handback 的信号答 `None`，是 handback 而读不出的答 `E_WIRE_MISMATCH`；两者都答 `None`，「子停了」与「这一行本 build 读不懂」在父那里就是同一种沉默。载荷的键住内部标签枚举 `HandbackBody`，写读两端同经它。
- 它不叫 `Verified`：本 crate 已有 `pr::Verified`，两个同名项一出现 rustc 就不再剪短路径，`tests/ui/merge_without_verification.stderr` 当场变红，编译器替「一个概念一个名字」执行了一次。

**`collab::signal_tool`**（形状 4 适配器）。`SignalEffect::{Enqueued, Consumed}`；`SignalDesk::new(run, room, who, reach, at, inbox)`（`at` 是本 run 的时刻，工具面没有时钟）、`pending`、`take_effects`、`take_steer`、`take_inbox`；`SignalTool` 两个 action：`send`、`pull`。

- Inbox 是借出的，不是拷贝的（D3）：整个 run 期间该房间的 Inbox 恰存一份，住在 desk 里，工人与工具共享一个 `Arc<Mutex<..>>`；驱动返回后无论成败都归还。
- `send` 只入队不投递：真正 `deliver` 到收件房间发生在驱动返回之后，且恒在 `signal_enqueued` 落账之后，因为投影只因一条已追加的事件而改变。
- 发件范围由 `reach` 定界：`reach` 是发件人所属楼的地址，由装配层经 `city::Building::of` 算好传入，「一个地址归哪栋楼管」的权威在 city。越楼发件恒拒，报 `E_CROSS_BUILDING_DENIED`。`ToolMeta.effect` 是静态的（`Write { domain: room }`），逐件目标判定在工具内。
- id 不采时钟、不取随机：`{run}-s{n}`，`n` 是 desk 自己的计数器，重放同一段历史得到同一批 id，去重才有意义。
- `take_steer` 取走一件就当场记 `Consumed`，读不成插队信的那件也记：一件离队的信就是已读，否则同一句话会在下一个安全点再落一次。空队列与读不懂的信不合成一件事：前者 `Ok(None)`，后者 `Err`；安全点怎么处理这个拒绝由 `accounting::worker::driving::lane` 决定（`crates/sprawling/Spec.lean` §8-73）。
- `pull` 的结果带 `remaining`：`status.signals_pending` 是派活那一刻的事实，一个数字比一套让 status 活起来的机制便宜得多。投递失败不静默：`Admission::Shed` 入账的是事实而非成功。

**`collab::goal_tool`**（形状 4 适配器）。`GoalDesk::new(run, owner, booking)`；`GoalBooking` 是调用时判定目标登记的那个权威，城里是记账线程（`crates/sprawling/Spec.lean` §8-42-8）；`conflict_refusal(entry, level)` 拼出 `E_GOAL_CONFLICT`，第三段是那一级的可执行说法；`GoalTool::new(room, desk)`。

- 三层各守其职：`detect_conflict` 答撞没撞，`arbitrate` 答谁来仲裁，本模块只把条目拼好交给 `GoalBooking`，恒不自己判冲突、恒不自己定级。
- 登记在调用时由 `GoalBooking` 判定，桌子不留副本：并排派出的两轮活读同一份目标表，只凭副本两轮都会登记同一片地。权威按队列次序对全城的目标表 `arbitrate`，先写账（`goal_registered` 或 `goal_conflict`）再回答；它的 `Err` 原样交给模型，桌子不排任何效应。
- 撞了就不登记，拒词只在本模块拼一次；一个只说「不行」的拒绝会让模型换个说法再试。同一 run 内的第二次登记看得见第一次。不写第三种 `arbitration_verdict`：`conflict_payload` 已携着那一级，该事件留给真正跑过一场仲裁的 run。

**`collab::pr_tool`**（形状 4 适配器）。`OpenRequest { node, implementer, branch, commit }` 与它的 `payload`、`from_payload`、`merged_payload`、`rejected_payload`；`MergedRequest` 是 `pr_merged` 的键（`reviewed_commit` 被审的、`commit` 落地的、`verified_by`、flatten 的 `CommitAttribution`）；`RejectedRequest` 是 `pr_rejected` 的键（flatten 的 `request`、`by`、`why`）；`PrEffect::{Opened, Merged, Rejected}`；`PrDesk::new(who, room, branch, node, open)`；三个 action：`open`、`list`、`check`。

- 验证与 merge 是一次调用的两个结果：一个 `Verified` 而无人 merge 的请求是第三种要人去追的状态，而 merge 就是「验证通过」的含义；拒绝是同一次调用的另一个结果（`passed: false` 携 `why`）。
- typestate 是走过的：`check` 内部真的造 `Claim`、`verified(true, self.who)`、`Pr::open(..).verified(&artifact)`，实现者不自测被检查两次：工具先拒（`E_GATE_DENIED`），类型再拒。
- 被判的是一个 commit 而不是一条分支：`OpenRequest.commit` 记下开请求那一刻分支站在哪里，Artifact 的 digest 由它派生，看过一个 commit 的人没有为后一个背书。`pr_merged` 同时携被审的与落地的两个 commit，一个键装两个事实，Ledger 的链就断在 merge 这一步；`merged_payload` 是这一行唯一的成形处。
- 三条记录的形状留在 collab，因为它们的键里有 `NodeId`。没有树的 run 说得出自己没有：`open` 报 `E_TOOL_UNAVAILABLE` 并指向要审查的楼。谁得到树由楼说了算（`RULES.toml` 的 `review = true`，见 `crates/city/Spec.lean` §8-2；D4）。

**`collab::triage`**（形状 1 判定；性质见 `spec/Triage.lean`）。`Reflex::{Discard, Notify, Light, Full}`、`Arrival`、`Rule`、`Landing { addr, reflex, because }`；`Triage::new(rules, fallback)` 在构造点拒空匹配串。

**`collab::claim_tool`、`collab::claim_effect`**（形状 4 适配器、形状 2 值类型）。`ClaimEffect::{Claimed, PutDown { exit: PlanExit }, Split { children }}` 与它的 `id`、`kind`、`payload`（形状是 `kernel::event::record::RoadmapMoved`）、`apply`；`ClaimDesk::new(who, room, roadmap, booking)`、`take_effects`、`roadmap`（仅当本次 drive 改过）、`holding`、`abandon`；`Booking` 是调用时判定认领的权威（城里是记账线程）；`still_true(text, effect)`。六个动作：`list`、`claim`、`finish`、`block`、`release`、`split`。

- `Roadmap.md` 是唯一权威，不另立认领登记表：文件被人读、被 `PlanTree::progress` 数、被这个工具改，一处事实，三个读者。
- 六个动作长在同一条 catalog 行上：模型每一轮读的行数是成本，一行背后的动词数不是（§14 的字节上限）。
- 状态迁移由 `kernel::PlanTree` 从计划自身判：`claim` 只从就绪集里取（叶子、无人认领、依赖全绿），`finish`、`block`、`release`、`split` 只作用于本次 drive 认领的那个节点（D6）；拒词报出此刻的状态并指向一个真能拿的节点。
- 认领在调用时由 `Booking` 判定，桌子的副本先答：先让 `PlanTree::claim` 在副本上判，再问 `Booking`；`Booking` 拒绝时桌子不持有节点、不排效应、不改文本。`Booking` 记的是哪轮在飞的活持有哪个节点，只活到那轮活落地为止，所以它不是第二份登记表；它拿到整条 `ClaimEffect::Claimed`，因为那一行的种类与载荷只由 `ClaimEffect::kind` 与 `payload` 定义。
- 一次 drive 只持有一个节点。计划门禁就是那个 `Held` 值：它由 `PlanTree::claim` 铸出，只能花在 `finish` 或 `stop` 上，没有第三个出口；一个只是结束了的 run 由 `abandon` 把它花在 `FrozeWithoutEvidence` 上，「认领了却没交代」在冻结之后不可达。
- `split` 只分本次 drive 握着的那一行，没握着就拒绝，拒词说这个 run 握着什么、恢复语叫它先认领那一行，桌子不排效应、不改副本（D6；`spec/Claim.lean` 的 `split_needs_hold`）。分完之后不再持有那根枝；写盘前把新文本重新解析并 `PlanTree::build` 一次，拆不出合法树就一个字节都不写。拆分结果带 `unfinished`：该节点下尚未 Done 的子节点数，由拆完的树数出。
- `block` 与 `release` 必须带一句原因，原因随记录走（`roadmap_blocked` 的载荷），不随表格走。哪一种记录由出口决定（`ClaimEffect::kind`）：绿是 `roadmap_finished`，红是 `roadmap_blocked`，交回是 `roadmap_released`，拆是 `roadmap_split`。效应穷尽，新增一个变体应当是写入处的编译错误。
- 效应怎么改文本只有 `ClaimEffect::apply` 一个定义：桌子在调用时用它改副本，工人落地时用它把同一组效应重放到盘上；`Split` 因此带着子节点的 weight。`PutDown` 携 `PlanExit` 而不是一个动词，把出口的两条臂抄进第二个枚举，就是对「一个节点可以怎么离开」的第二份意见。`still_true` 问的是记录答不了的那个问题：盘上的文档现在是否仍然容得下这条效应。只有认领会答「不」——它要那一行仍是 `Not started`；放下与拆分只作用于本 run 握着的那一行，它们的新鲜由握持之前的那条认领担保（D6），`still_true` 不为它们另判一个期待状态。
- 并发口径：工人写盘前重读文件，把效应按次序重放上去，每条在前面几条留下的文本上问 `still_true`（`spec/Claim.lean` 的 `land`；`crates/accounting/Spec.lean` §8-27）：本 run 拆出又认领的子行因此在那里；一条认领的行若已不是 `Not started` 则整组丢弃并留一条诊断，而不是覆盖；都对得上时只有本轮碰过的行改变。桌子答应的效应落在派活那份计划上全部落下（`admitted_lands`），工具答成功而落地一字不写只在别的写者动过那一行时发生，并且有那条诊断。

**`collab::archive_tool`**（形状 4 适配器）。`ARCHIVE_KINDS` 是封闭的四类（§14）。回忆是读，不是记：索引由 worker 从书架算出后交给桌子，桌子不留副本，盘上的文件才是真的。`ArchiveEffect::Recorded` 是桌子交回的值，落盘与记账归装配层。

**`collab::citation`**（形状 1 判定）。`Citation::new(quote, at: Locator)`、`quote`、`at`、`against(pinned, bytes) -> Reading`（纯函数）；`Reading::{Holds, OtherVersion, OutOfRange, Differs { found }}`。

- 最小写法是「引文加 `cas:` 或 `file:` Locator 区间」，不另立引用语法：Locator 是城市唯一的检索文法（glossary *Locator*），区间沿用 `L<from>-<to>`（1 起、闭）与 `B<from>-<to>`（0 起、闭）；不带区间表示引文可出现在整份版本中的任意位置。
- 比对前两侧都把连续空白压成一个空格并去掉首尾空白；再多一步（大小写、标点）就会让一条改了意思的引文被判为相符。
- 版本不同先于文字比对：引文指向另一个 hash，文字碰巧相同也不算核对过；改稿后旧的审查要失效，靠的就是这一步。
- 验证 run 逐条调用 `against`，非 `Holds` 的每一条都进它的报告；它是 `Reading` 而不是 `Result`，因为对不上是验证的结果，与 `Handback::Stopped` 同理。验证节点怎么跑仍未定，见 §3。
-/

/-! ## 9 工作流程

装配层为一轮活造桌子（`SignalDesk`、`GoalDesk`、`PrDesk`、`ClaimDesk`、`DelegateDesk`、`WorkshopDesk`、`ArchiveDesk`），把 `Arc<Mutex<..>>` 句柄交给对应工具注册进 bench；模型调工具，桌子判定并排效应（登记与认领在调用时问记账线程的 `GoalBooking` 与 `Booking`）；这轮活落地时工人取走效应，先写账再改投影；派出的代理在父回合落定后开 run，结束时经 `Handback::signal` 回到父房间的 Inbox，并汇入那个房间的 `FanIn` 与 `Underway`。
-/

/-! ## 10 实现逻辑

每个模块的实现规则写在 §8 与分部里。四个设计选择各有被否决的备选：

D3 工具怎么拿到活的跨 run 状态：把 Inbox 本体借给工具。被否决的是开一个 `peek` 让工具拿快照：它要在 `storage::EventQueue` 与 `collab::Inbox` 两处各长一个读面，还造出两份同时存在的队列，「谁才是顺序的权威」多了一个答案。借出方案零新接口，与装配层已有的 `interrupts` 借还同形；代价是借出期间工人手上没有该房间的队列，所以归还必须在成败两条路上都发生，一条断言盯着这件事。

D4 谁来定一个 run 写在哪：楼的规则说了算（`RULES.toml` 的 `review`）。被否决的是每个 dispatch 都领一棵树：它让每一次普通派活都付一次全量检出，并把「派一个居民改一行字」变成「还得再派一个来看一眼」。楼级开关把选择交回给人，并与 `confidential` 同形同位：一个已有的权威多一行。

D5 三件工具还是一件多臂工具：每个机制一件。被否决的是一件 `collab` 工具带 action 枚举：它省两份 schema 的常驻字节（工具表在缓存前缀里，这是真成本），但把三件变更理由不同的东西绑成一个接口，disclosure 只能写成一句拉长的话；模型按名字选工具，一个叫 `collab` 的工具不告诉它任何事。多出的常驻字节是两份空 schema 的量级。

D6 分一行计划要不要握着它：要，由桌子在调用时判，落地不再判。`ClaimDesk::split` 只分本次 drive 握着的那一行，没握着就以 `E_INVALID_ARGS` 拒绝，三段式的主体说这个 run 握着什么，恢复语叫它先认领那一行（`spec/Claim.lean` 的 `split_needs_hold`）；`still_true` 只问认领，不为拆分另判一个期待状态。理由有三条。其一，拆分于是走过认领，而认领在调用时由 `Booking` 对全城判、落地时对盘上那份判，所以从同一份计划派出的两个 run 不会把同一行各分一次（`split_of_moved_row_is_stale`）；不经认领的两次拆分都会落下，`kernel::spine::insert_children` 在已有子行之后接着编号，第二组子行不报错地长出来（`withoutHold_splits_twice`）。其二，拒绝在模型调用的那一刻到达，模型当场拿到下一步，而不是被告知分好了、落地时才被丢掉（`tools/adversary/Spec.lean` §4 第八个发现）。其三，`split` 本来就是一个干着活的 run 发现活更多时说的话（`collab::claim_tool` 的模块注释），握着那一行是它的前提。被否决的是让落地按桌子看到的状态判、允许分一行没人认领的计划：那样的拆分绕过 `Booking`，并发的两次拆分都按 `Not started` 落下；要挡住它就得给拆分另立一种预订，那是「谁在动这一行」的第二个权威。代价是一个只想分计划的 run 多付一次 `claim` 调用。重开参数：出现只分计划、不干那一行活的角色（例如市长把一件事分给几栋楼，G4），而 `Booking` 能为一次拆分预订那一行时，改由预订判，桌子不再要求握着。

成本：每件工具在 catalog 里占一行，坐在缓存前缀里，一行背后的动词数不增加常驻成本，行数与字节数才增加（§14）。Signal 在 prefix 里占零字节，常驻的只有 `status` 的 `signals_pending`；pull 的 bandwidth 在接收方。拒词报出此刻的状态与一条可执行的下一步，因为只说「不行」的拒绝会让模型换个说法再试。
-/

/-! ## 11 边界枚举

每个模块在构造点或调用点拒绝的输入写在 §8 与分部里，拒词各自报出三段式：图的重名、悬空依赖与环；空的 goal、owner、done_check、stop；空的 Triage 匹配串；越楼的 Signal 与代理；生产者自验；第二张图；不认的工具动词；无树的 `open`。
-/

/-! ## 12 错误处理

一个拒绝是三段式的 `AxError`：动作、主体、恢复。本 crate 的拒绝不改变任何状态：桌子在拒绝时不持有节点、不排效应、不改文本；判定在前、效应在后，效应只在一轮活落地时由工人写账。三处把一个拒绝当作结果而不是故障：`Handback::of` 把验证的 `Err` 收成 `Stopped`，`Citation::against` 答 `Reading`，`Triage::decide` 判不出时交给人。`from_signal` 分开「不是这一种」（`None`）与「是这一种而读不出」（`E_WIRE_MISMATCH`）。

D1 仲裁只有两级（ruling）。`Level` 只有 `Serialize` 与 `Arbitrate`，`arbitrate` 收两个入参，这是人的 ruling。第三级「升到人」在生产里不可达：唯一的生产调用点没有能让它成立的输入，门只答 Allow 或 Deny，被门拒掉的 run 不占地盘，也就无从与人相撞；一个不可达的级别仍会散布在公共面、账本载荷与恢复语里。一个居民读不定的冲突是一个设计问题，按设计问题进 Inbox。被否决的是用一个穷尽的 `Occasion` 枚举描述何时升到人：它把一个到不了的级别保留成一个更整齐的到不了的级别。

D2 没有草稿退回机制。房间没有版本，发言不带「作者所见的房间版本」进房间，也就没有退回作者、四路择一与 hold token。它要防的两种冲突各有权威：同一份文件的并发写由 `storage` 的 `base_version` 乐观并发与 worktree 隔离解决；同一件事的并发认领由 `kernel::goal` 的同资源相斥与 `arbiter` 解决，第三套机制就是第三个权威。`Signal` 的 `room_version` 在生产写点（`signal_tool`、`handback`）恒为 `Version::FIRST`。重开参数：出现一个真的会前进的房间版本，即有生产写点把 `room_version` 填成 `Version::FIRST` 以外的值，那时退回从那个写点长出来。
-/

/-! ## 13 依赖选型

拓扑硬约束：`kernel` 与 `storage`（ARCHITECTURE.md §3 的 depmap 块）。新外部依赖逐次论证，无论证即不引。规格的分部不 import 任何别的 crate 的规格：本 crate 要的 kernel 概念（Signal 的种类、Artifact 的 digest）在模型里写成参数或最小的归纳类型。
-/

/-! ## 14 硬编码声明

`ARCHIVE_KINDS` 是封闭的四类：`preference`、`decision`、`correction`、`fact`。第五类要有理由，而「它不属于前四类」正是让分类腐烂的那个理由，所以拒词点名四类并问这是哪一类。

`plan` 条目的 548 B 上限：六个动作的 disclosure 加 schema 的紧凑 JSON，由一条断言钉住。disclosure 里那句 *Must this be expanded?* 是 LLM First（ARCHITECTURE.md §9）的提醒，在缓存前缀里，零延迟、零花费；不追问、不设深度上限、不设审批。省下的字节来自把 Locator 文法从 schema 移进拒词：一句重复了拒词内容的说明，是每一轮都在付、只读一次的字节；schema 里没有的东西，模型第一次写错时会从三段式拒词里拿到。
-/

/-! ## 15 影响面

改一张桌子或一件工具的构造签名，波及 `crates/sprawling` 装配层造这张桌子的地方；改记录的形状（`Signal`、`OpenRequest` 一族、`ClaimEffect::payload`）波及读 Ledger 的折叠与视图。改 `still_true` 或桌子对哪些动作要求握持（D6），波及落地的那一处 `accounting::effect::Claims::of`（`crates/accounting/Spec.lean` §8-27），两边与 `spec/Claim.lean` 一起改。改分部里的模型，先改本文件对应的要求，再改 Rust 与它的测试。
-/

/-! ## 16 测试与约束

证明：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`，`spec` 门判后两条的文本形状。咬得动的演示：`Collab.Inbox.withoutSeen_duplicates`、`Collab.Fanin.withoutGuard_self_verifies`、`Collab.Workshop.withoutHanded_hands_twice`、`Collab.Triage.withoutCap_starts_work`、`Collab.Claim.withoutHold_splits_twice`，各自说明拿掉哪条守卫后对应的性质不再成立。

实现一致性：逐模块 `tests.rs`；`tests/ui/` 钉住两条判负线；`tests/pr_flow.rs` 走一遍 PR；`cargo nextest run -p sprawling-collab`。模型的证明不是 Rust 实现的证明：两者之间由这些测试连着。
-/

/-! ## 17 文档关系

- ARCHITECTURE.md 的模块表（`architecture.toml` 里 collab 各行，锚点指向本文件与分部）与 §4 缝清单。
- `docs/glossary.md` 的 Signal、Inbox、Workshop 等词条：词条改名时本文件与分部一起改。
- `crates/kernel/Spec.lean` 的 goal、repair、delegation 各节（§8-15、§8-16、§8-17）与 §8-4（Signal 的线上形状）；`crates/city/Spec.lean` §8-2 的 `review` 规则（D4）；`crates/sprawling/Spec.lean` §8-42-8（记账线程是 `Booking` 与 `GoalBooking` 的权威）与 §8-73（安全点怎么处理 `take_steer` 的拒绝）。这些节改了，重读本文件 §8 的对应条目。
-/
