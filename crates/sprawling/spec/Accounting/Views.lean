-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views：写在 sprawling 规格里的那些节

本分部收着写 accounting crate 里 `accounting::views` 的节。模块从 `sprawling` 搬进 `accounting` 时，写它的那一节留在本 crate 的规格里（accounting D15），标签不变；为什么暂住这里、何时搬走，见 sprawling D30。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。
-/

/-!
## 8-3 视图与 Spine

```rust
pub(crate) struct Views { city_root, hot: HotView, attribution: Attribution, approvals: BTreeMap<String, ApprovalSummary> }
impl Views { fn apply(&mut self, &EventRecord) -> Result<(), AxError>; fn prepare(&self, &Query) -> Prepared; fn answer(&mut self, &Query) -> Answer; }
impl Prepared { fn finish(self) -> Answer; }                                 // 锁外读盘，见 §8-100
impl Views { pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>; }   // 一次性的读：先审计整条链，再从快照起步（§8-91）
impl CityAsk { fn read(self) -> wire::Answer; }                     // 放下快照后列楼、读计划，见 §8-100
```

- **视图冷重建与热折叠共用 `apply`**：启动时把 Ledger 逐行喂进去，其后由 `JsonlLedger::observe` 喂。测试断言两条路径答案逐字段相等——这就是「projection 可弃」的可执行形式。
- **计划的投影是 `Roadmap.md` 的缓存**：那份文件就是计划，Agent 用 edit 工具改它。`Views` 持 `Arc<Mutex<accounting::plan_view::PlanView>>`，`roadmap_*` 记录经 `apply` 让缓存失效，查询在锁外读盘回填（§8-100，`crates/accounting/Spec.lean` §8-6）。
- **读不懂的行照显**：`problems` 随答案回到界面；没有 Roadmap 的楼答 `Progress::Unplanned`（它没有 ratio 方法，故界面画不出百分比不是守规矩，是无从下手）。
- **保留前缀不是楼**：`.` 开头的目录跳过，`.sprawling/` 因此恒不被当成一栋楼。
-/

/-!
## 8-6 五个视图不再答 unavailable

`Views` 增四份折叠与一次读盘，`answer` 的 catch-all 臂随之消失——**穷尽 match 是这次改动的验收之一**：此后新增一个 Query 不写答案就编译不过，而不是在运行时答一句 `Unavailable`。

| 查询 | 出处 | 口径 |
|---|---|---|
| `InboxView` | 折 `signal_enqueued` 减 `signal_consumed`：前者经 `collab::Signal::from_payload` 读、按它的 `room` 入队，后者经 `collab::SignalConsumed::from_payload` 读、按 `id` 从每个房间的队列里出队，与 `CollaborationFold` 同一种读法（`signal_consumed` 只写 `id` 与 `by`；行的 `addr` 是取走它的房间，没有什么强制它等于入队时的房间，按 `addr` 出队的视图会在两者不同时让已取走的信号一直等着）；读不回的一行让折叠报错而不是被跳过 | **看队列不靠消费**：`Inbox::pull` 要拿走才给得出内容，一个看一眼就把东西取走的视图会改变它所报告的对象。**与写者同一把尺**：视图自己按键名读这两行时，消费行里没有 `room`，被取走的信号在视图里永远等着 |
| `DiscardView` | 折 `file_discarded`／`discard_restored`，按路径归键；本版本读不回的一行整行跳过，不以空路径或空还原顶替 | 每行自带回去的路（`restoration`）；还原是**关掉它开的那一行**，不是另开一行 |
| `RegistryView` | 折 `asset_archived`；没有房间或本版本读不回的一行整行跳过，`kind` 与 `subject` 不以默认值顶替 | 「这座城认定值得留下的东西」；空表就是空表，与「本版本答不了」在类型上已经不可混淆 |
| `ArchiveSearch` | 被问的那一刻读盘（同 `BuildingView`） | 文件是权威，另存索引就是第二个权威 |
| `Metrics` | 上面几份＋`hot`＋`read_spine` | **恒不携钱**：钱是 `CostView` 的，一个数字两个主人就是两个数字开始互相矛盾的起点。这里每个数都已被别的视图证明过，它存在只为让画一条读数花一次问答；唯一自有的数是 `events`（本视图折过多少条），因为没有别的答案能推出它 |
-/

/-!
## 8-37 每个问题的答案住在读面（`accounting::views`）

`Views` 折账本、答每一个 `Query`，删掉重折得到同样的字节——那是 `projection`（ARCHITECTURE.md §9 的形状 7），而 `bin::assembly` 是 `adapter`：装配点、唯一全知。一个模块里两个形状，就是它们分家的依据。`views` 是 `assembly` 的兄弟模块而不是 `assembly/` 下的子目录，因为 `Views` 是投影，不是 `RunWorker` 的一部分；两者之间的依赖方向见 §8-92。

测试跟着被测的东西走：`views` 的断言住在 `views` 下，一份留在原处的断言会让下一个人以为那里还有代码。
-/

/-!
## 8-167 一次提交出自哪次运行，从账本回答（`accounting::views::commits`、`sprawling whose`）

这座城作出的每一个提交都带上五条 git trailers 与一个
`<actor>@<city 前 12 位>.sprawling` 的署名。那是**给城外读者的投影**：拿着 `git log`
的人看得见谁写了这一行。反方向还欠着——**手里只有一个 oid 的人，问不出它出自哪次会话**，
而这正是一个人在 `git blame` 之后会问的下一句话。

### 折叠，不是去读 git

```rust
// accounting::views::commits（形状 7 projection）
pub(super) struct CommitFacts { /* run、seq、actor、model、effort —— 私有 */ }
pub(super) fn commit_facts(record: &EventRecord) -> Option<(GitOid, CommitFacts)>;
impl CommitFacts { pub(super) fn answer(&self, oid: GitOid) -> wire::CommitAnswer; }
```

- **两种记录进这张表**：`checkpoint_committed`（键 `oid`，工具波的检查点与基线提交）与
  `pr_merged`（键 `commit`，一次评审落地时 trunk 收到的那个合并提交）。这两种是
  这座城**唯一**会造出提交的两处。
- **派活开场那条 `checkpoint_committed` 不进表**：它是「活钉在这里了」的销钉，载荷只有
  `job`，没有 oid。依据因此是「读得出一个 oid」而不是「是不是这个 kind」——
  一个不带 oid 的检查点行不是一次提交。
- **`run`、`seq` 与 `actor` 取自记录自己的身份**（`EventRecord::run`／`seq`／`addr`），
  `model` 与 `effort` 取自载荷（`crates/storage/Spec.lean` §8-18）。**没有一个字段是从 git 读的**：
  投影读权威，不读另一份投影。
- **重复的 oid 后写覆盖前写**：同一个 oid 只可能由同一次提交产生，两条记录说同一件事时
  它们说的是同一件事。
- **`session` 由地址算出而不是另存一份**：房间是地址的最后一段，`city::open_room` 当初
  就是拿人给的名字开的它；派到楼根的运行没有房间，故答 `None`。

### 城外那扇门：`accounting::views::ask`（住 `accounting::views::holding`）

```rust
pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError>;
```

**住 `views` 而不住 `assembly`**：它的全部内容就是「造一份 `Views`、问一句、丢掉」，
而 `Views` 的一生是 `holding` 的；另一个理由是 `assembly.rs` 已站在 400 行预算上（开工时
401 行），而为一行重导出把一道门推得更红是拿门当对手。

一次性折叠这座城的账本并回答一个 Query，然后把视图扔掉。**它与被端上来的城读的是同一条路：
`answer_outside_the_lock` 用的 `Views::prepare` 与 `Prepared::finish`**——若 CLI 自己另写一份读法，同一个问题在这座城里就有两个答案，
而漂开的总是没人看的那一个。链先被 `runtime::replay::verify_ledger_dir` 验过：
历史不成立的城，它的视图不该被端出来。

### `sprawling whose <city> <oid>`（`bin::main::whose`）

四行输出：run、actor（带 session）、model 与 effort、以及账本位置 seq；之后是 §8-136 的 `previous` 行，带 `--trace` 时再列区间里的调用。
**自己一个文件而不是 `main::data` 多一个臂**：`data` 里每一个动词不是搬字节就是验链，
而这一个是从城的历史里读出一个答案并把它渲染给人看；且 `data` 已到 360 行，
把四十行放进去得拿别处的行数去换。
- **不接受短 oid**：`GitOid::parse` 只认 40 位小写十六进制，长度不对即拒而不是补齐去猜。
- **退出码**：0 答上了；1 这座城没写过这个提交（`Unavailable`）；2 命令行读不了。
  「城没写过它」是 1 而不是 0，因为一个脚本据此判断「这一行是机器写的吗」时，
  把「不知道」读成「不是」会把审计写成假话。

### 红转绿

`a_commit_the_city_made_says_which_run_wrote_it`（`accounting::worker::driving::tests::ledger`）：
一次真派活跑完，从账本里取出那条带 oid 的 `checkpoint_committed`，用它的 oid 问
`Query::Commit`；答必须给出这次运行的 run、房间地址、模型 id 与档位，以及那一行的 seq。
未落地时这条查询答 `Unavailable`，红就红在这里。

### 文档同步

本节；`crates/wire/Spec.lean` §8-17 与 §2 的 golden（WIRE_V 13→14，Query 15 个）；
`crates/storage/Spec.lean` §8-18；`ARCHITECTURE.md` §7 的两个数与 §12 的 `accounting::views::commits` 一行；
`README.md` 的 History 一节；`tools/adversary/Spec.lean` §2 的线面计数。
公开面：`wire` 增 `Query::Commit`／`Answer::Commit`／`CommitAnswer`，
`storage` 增 `Provenance::model_fields`／`model_choice_of`／`effort_word`，
`sprawling` 增 `ask`，故 `api-baselines` 三份随之重算。

**接线到哪一步了，以及四处 `#[expect]` 的账**：记账那一侧已经接上——`spawn_worker` 在循环外开一个 `RelayGate`，
循环里第一件事是 `worker.serve_relay(&relay)`，然后才 `worker_desk.wait(...)`，
于是「relay 请求排在 desk 命令之前」这条规矩现在由代码持有而不是由一段文字持有。
池那一侧还没有生产调用方，于是 `Relay`、`RelayGate::issuing`、`RelayGate::issue` 与 `gone` 在非测试构建里是死代码。
这里的处理是四处 `#[cfg_attr(not(test), expect(dead_code, reason = …))]`——
**用 `expect` 而不是 `allow`，正是因为它会自己清掉**：池一落地，这四条期待就变成「未兑现的期待」而编译失败，
删掉它们是那一步必须做的事，而不是某个人必须记得的事。

### 城市立起来时就有市政厅，和一条写下来的代答

**`form_city` 在 line zero 之后多做三件事**，顺序固定：

1. 追加一条 `autonomy_changed`，值 `delegate:hall/clerk`。**不改 `AUTONOMY_DEFAULT`**：缺省值说的是「没人说过话时怎么办」，而这里是这座城市作出的一个决定，人可以改它，改它要有一行历史可改。`folds` 读回这条线，重启后 clerk 依旧是代答者，无需第二处记忆。
2. 用 `city::CityPlan::new(None).hall()` 拿到那栋楼，走 `create_building` 落 `RULES.toml` 与脊柱文档，并记 `building_created`。走这扇门而不是另写一段，是为了让市政厅与任何一栋楼在历史里长得一样。
3. `city::lay_out_hall_identities` 把 `MAYOR.md` 与 `CLERK.md` 写进 `<city>/.sprawling/`，已存在的不覆盖。

**影响面（一处真实回归，已改）**：从此每座城市至少有两栋楼。`views::tests` 里两处按 `buildings[0]` 取楼的断言改成按地址找 `lab`——它们原本靠「城里只有一栋楼」这个此后不再成立的前提。改的是测试对现实的假设，不是把依据放宽。

**留给后续卡**：`hall` 的居民目前拿到的仍是 `workbench::tools` 给所有人的同一套工具表，`exec`／`delegate`／`workshop` 都在里面；这张表按地址裁，而不是只由写域挡住（`Documents` 拒非 `.md`），不由工具表挡住。
-/

/-!
## 8-50 三种读法的答：回合、证据、一个节点的花费（`accounting::views::rounds`、`views::evidence`、`views::cost_of`）

`wire` 定形状（`crates/wire/Spec.lean` §8-21），这里折出答。三个新模块都是形状 7 投影：把记录折成页要的读法，删掉重折逐字节相同。

### 8-50-1 `accounting::views::rounds`——一次会话折成回合

`records_of(run)` 用 `LedgerIndex::run_seqs_before` 取这次跑最新的 `HISTORY_MAX` 条 seq，逐条读行、逐条解析，旧在前。**上界与客户端原来问的那一段等宽**：`web::live::page` 一直是 `RunHistory { limit: HISTORY_MAX }`，所以搬到服务端之后一个会话能被读到的范围一字未变——搬家不该顺手改答案。读不回来的一行**截断而不清空**：读到的那些仍然是真的（同 `Views::history`）。

折叠本身逐字从 `web::turn::rounds` 搬来，一条不改：`model_called` 开一个回合；`tool_called` 挂进当前回合并按 runtime 给的 id 记下等答；`model_returned` 落到最近一个回合上；`tool_result` 按 id 找回它自己的那次调用——**恒不按位置配对**，因为两个调用可以先后发出而后发的先答；其余记录按 `wire::reading::note_of` 判是否成为一条 `Note`。`opened_at` 是本会话第一条 `Checkpointed` 的 oid：那是这份活开始时的树，取最新的一个检查点答的是另一个问题（「上一波动了什么」）。

**等人的终点从城自己的 run 里配回来**：`approval_resolved` 记在 `RunId::CITY` 下，不在这次会话的记录里，所以折完回合之后 `answer_waits` 按 approval id 把答复的 `t` 写回 `Note::Waiting.answered`。只有这次会话的窗口里有 `approval_requested` 时才读城的那一段（同样最新的 `HISTORY_MAX` 条）——没等过人的会话不多付一次读。请求或答复落在各自窗口外、或载荷读不回来时，`answered` 为 `None`，页面把请求之后整段画给人，而不是猜一个终点。备选是让治理折叠常驻一张 id→答复时刻的表：它随城一生里答过的每一件批准只增不减，而这里的读只在打开一个等过人的会话时才付。

### 8-50-2 `accounting::views::evidence`——写下来的证据，不携字节

同一份记录再走一遍，只认两种：`tool_result` 载荷 `result.image` 是 `cas:` 定位符的，成 `EvidenceKind::Screenshot`（连同 `width`／`height`／`media_type` 三项，缺一则 `picture` 为 `None` 而行仍在）；`roadmap_finished` 载荷 `evidence` 能读成定位符的，成 `EvidenceKind::Finished`。定位符读不回来的记录**不成行**——发明一条指不到任何东西的证据比少一行糟。

### 8-50-3 `accounting::views::cost_of`——节点到跑，跑到钱

`Views` 多一张 `claims: BTreeMap<NodeId, BTreeSet<RunId>>`，由 `roadmap_claimed` 折出（载荷 `node` 加记录自己的 `run`）。`BTreeMap` 而非 hash：这是答一个查询的路径，顺序必须是确定的。答时把这些跑在 `storage::attribution` 的 `by_run` 里各自的数取出来相加——**这里不计价**，计价是 `gateway::cost` 的，归因是 `storage::attribution` 的，本模块只做一次求和。

### 8-50-4 验收

- `views::rounds::tests` 与 `views::rounds::reading_tests` 把 `web::turn` 的两份测试逐字搬来，跑在服务端的折叠上：**服务端算出来的回合等于视图层对同一批记录算出来的**。红是在真账本上取的——`Query::Rounds` 先答 `Unavailable`，`asking_for_rounds_answers_the_fold_the_view_layer_ran` 在 `init_city` 铺出的城上写三条记录再问，失败于「Rounds answers with rounds」。
- `views::evidence::tests`：一张截图与一条完成证据各成一行，读不回定位符的载荷不成行。
- `views::cost_of::tests`：认领过的节点报出那次跑的钱；没人认领过的节点报 0 与空明细，而不是 `Unavailable`。

### 8-46-13 检查点不排队：每条 lane 一个自己的 index（`accounting::worker::driving::lane`、`accounting::worker::driving::harness`、`accounting::worker::workbench`、`accounting::worker::workbench::standing`）

**这一节是 sprawling 一侧关于检查点并发的唯一一处规则**；它依赖的性质由 `crates/storage/spec/Checkpoint/Concurrent.lean`（`crates/storage/Spec.lean` §8-39，storage D25）证明，§8-113 与 §8-110 只引用这里。

**一轮检查点是一个动作，不是一个 run 的一部分，而它不必与别的 run 的检查点排队。** 一个 ready set 里的每个节点各占一条 lane 同时跑（§8-46-4），它们都落在**同一个仓库**里。两条 lane 共用仓库的那一份 index 时，libgit2 为暂存取 `.git/index.lock`，后到的一条拿到「the index is locked; this might be due to a concurrent or crashed process」，以 `cancelled` 冻结，它的节点被当作「自己的 done check 没过」交回——所以检查点之间必须有一个次序。这个次序不需要一把全城的锁给：每条 lane 用 `Checkpoint::open_writer(city_root, run)` 开一个自己 index 的句柄，暂存、写树、建提交只碰它自己的 index，共享的对象库与以 oid 为名的引用在任何交错下都等于某个串行次序（storage D24）。于是：

- **lane 的波前检查点**（`driving::lane` 的 `checkpoint` 闭包，`wave_pre`）、**harness run 的提交**（`driving::harness::commit`）各用这个 run 的写者句柄，不取任何锁。run 落地时 `close_writer` 删掉它的 index 文件。
- **打开检查点仓库**只在开城时做一次（`bin::assembly`，城还没有仓库时 `Checkpoint::open` 建它），lane 不建仓库，所以 `workbench::open_checkpoint` 不再与别的 lane 争仓库的 config 锁。
- **`ensure_base`**（`workbench::standing::lend_tree`）改写 HEAD，HEAD 是比较后交换：两条 lane 同时见到一座没有提交的城，一条的提交成为 HEAD，另一条重读后什么也不做（storage §8-39）。它用的是城的 index，与人自己的 `git add` 是同一个文件，所以它仍走 `storage::checkpoint::scan::write_index` 的等待。

**两道保险各管一个对手，理由写在各自的位置**：同一个进程里两条 lane 之间，现在由各自的 index 隔开；`storage::checkpoint::scan::write_index` 的等待管**城的那一份 index 上的另一个进程**——人自己的 git，或误开在同一座城上的另一个 sprawling——那是任何进程内的机制都看不见的对手。

**被否：保留 `checkpoint_gate`（一个仓一次检查点）。** 锁的宽度只是一次检查点（stage＋commit＋读回），可它是全城的：测试城里一个 run 独占这把锁时，写入波前的检查点 p50 18 ms、最大 64 ms，N 条 lane 同时有写入波时，排在最后的一条要多等 N−1 次检查点：并发的 run 越多，写入波在锁上等得越久。

**当前状态。** 代码仍是那把全城的锁：`Flight.checkpoint_gate`（`crates/accounting/src/worker/driving/flight.rs`）由 `driving::lane`、`driving::harness`、`workbench::open_checkpoint` 与 `workbench::standing::lend_tree` 四处取用，`Checkpoint::open_writer` 尚不存在。实现按上面三条改，同一变更集删掉 `checkpoint_gate` 与它在 `DriveContext`、`Laying`、`Placing` 里的克隆；完成的读数是 N 个 run 同时有写入波时，检查点的等锁时间为 0。

### 8-46-9 一个房间一个队列：`accounting::worker::rooms`

形状：**深模块**（ARCH §9 第 1 种）。文件 `crates/accounting/src/worker/rooms.rs`。

`RunWorker.inboxes` 从前是 `BTreeMap<Address, Inbox>`，字段注释写着「一个房间恰好一个队列」，而**没有任何东西执行这句话**：
`open_desks` 以 `remove` ＋ `unwrap_or_else(new_inbox)` 借队列，`settle_desks` 以 `insert` 还队列。
在 `DRIVING_LANES = 4` 下，`pursue` 把同一栋楼的多行派到**同一个房间地址**，于是第二轮活借到一个空队列，
先落地那一份被后落地的整份覆盖——账本上写着 `signal_enqueued`，内存里那条信号不存在，而只增账本无法区分
「本来就没有」与「被覆盖了」。

```rust
/// 一个房间的队列，以及它在谁手上。
enum RoomQueue {
    Home(collab::Inbox),
    /// 借出期间送来的信号在同一格里等，持有者落地时一并交过去。
    Lent { to: RunId, waiting: Vec<collab::Signal> },
}

/// 借到的东西，与这轮活是不是持有者。
pub(in crate::assembly) enum QueueTenure { TheRoomQueue, ASpare { held_by: RunId } }
pub(in crate::assembly) struct Lent { inbox: collab::Inbox, tenure: QueueTenure }

pub(in crate::assembly) struct RoomQueues { rooms: BTreeMap<Address, RoomQueue> }
impl RoomQueues {
    fn folded(rooms: BTreeMap<Address, collab::Inbox>) -> RoomQueues;
    fn lend(&mut self, addr: &Address, to: RunId) -> Lent;
    fn give_back(&mut self, addr: &Address, from: RunId, returned: collab::Inbox) -> Result<(), AxError>;
    fn deliver(&mut self, signal: &collab::Signal) -> Result<(), AxError>;
    fn pending(&self, addr: &Address) -> u32;
}
```

- **借出不是取走**：`Lent` 仍在表里，第二个同房间的 run 得到一份自己的空队列（`QueueTenure::ASpare`）并被记一条
  `Refuse` 诊断。为什么不是三段式拒绝：`pursue` 的整个 ready set 都派在同一个地址上，拒绝会把并发追求
  （§8-46-4）整条打掉，而「一个房间一个读者」本身就是对的——两轮活分读一个队列，每轮只看到一半的信。
  **名字说的是持有关系而不是被持有之物**：城的书架那一件叫 `city::Holding`（图书馆的 holdings），
  这里说的是这轮活以何种名分拿着队列，故叫 `QueueTenure`。
- **只有持 `run_id` 的能还**：`give_back` 校验 `Lent.to`，不匹配即 `E_STORAGE_FATAL`。这条把「谁借的谁还」
  从纪律变成类型之外的运行时断言，而 `QueueTenure` 让调用点根本写不出「拿着 spare 去还」。
- **借出期间的投递有地方落**：`deliver` 对 `Lent` 推进 `waiting`，上限仍是 `INBOX_CAPACITY`，满了照旧
  `E_BACKPRESSURE_SHED`。**满溢的措辞只有一个家**：`landing.rs` 里手写的第二份 Shed 判定随之删除。
- **队列一定回家**：`give_back` 里等候的信号若被拒，队列先放回表再把错误抛出去——一个在回家路上失败的队列
  从前就是一个城忘掉的队列。

**B-28 与它是同一件事的另一半**。`land` 从前在 `driven?` 处提前返回，`settle_desks` 与 `conclude` 都不执行：
房间队列丢了，worktree 租约不归还——而这正是磁盘出问题时集中发生的那条路径。此后 `land` 先调
`return_borrowed`（还队列、还租约），再读 drive 自己的结果；`conclude` 不再释放租约，`settle_desks` 不再收队列，
两件事各剩一个家。

**backlog 席位与队列是同一批借来的东西**：`land` 从前先以 `?` 交还 backlog 席位，再调 `return_borrowed`，
于是一把中毒的 backlog 锁连房间的信一起吞掉。此后两次归还都先做完，再按顺序抛出第一个失败——
归还路径上没有提前返回。

### 8-46-10 服务态是一个值：`Serving`

`RunWorker` 从前有三个各自 `Some` 的 `Option`——`interrupts`／`watching`／`machine`——三个 setter
（`watch`／`examine`／`attach_interrupts`）由 `assembly::attending` 在同一口气里各调一次，文档各自写着
「None in every worker but the one behind a live control surface」。三个字段容许八种状态而只有两种可达，
而加第四个 sink 意味着记得加第四个 setter。

```rust
pub(crate) struct Serving {
    deltas: Arc<dyn Fn(wire::Delta) + Send + Sync>,
    machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    interrupts: Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
}
impl RunWorker { pub(crate) fn serve(&mut self, serving: Serving); }
```

一个 `Option<Serving>`，一个注入点，漏一个即编译错。`assembly::attending` 的三次调用合为一次；
测试要只听中断时经 `fixture::only_interrupts` 明写它不听什么，而不是另开一扇门。

### 8-46-11 排程窗口逐条走完，撤销靠反向命令

`tick` 从前先把 `last_tick` 推到 `now` 再逐条派活，`?` 在第 k 条上返回时第 k+1..n 条到期作业**既没跑、没入账、
也不会回来**：一栋楼地址写错就能让同一分钟里其它所有定时活消失。此后每条到期作业都经 `start_unasked` 尝试，
起不来的记一条 `Refuse` 诊断并继续下一条，窗口只在整段走完之后才关。

**入账落在诊断日志而不是账本**：`schedule_dispatch_failed` 需要一个新的 `EventKind`，而 wire schema hash
随 `EventKind::ALL` 变动（`crates/wire/Spec.lean` §8-1），本波 wire 冻结在 32。同一处境下 `answer_knocks` 早已用诊断行记
「敲不开的门」，此处复用同一机制而不是造第二种。**欠账**：`city::Schedule::due_after` 仍返回三元组，
不带 `fired_at`，所以窗口只能整段推进而不能逐条推进——补 `fired_at` 要改 `crates/city`，属另一张卡。

**撤销不是一个动词**：`attach` / `select_model` / `set_autonomy` 之后的一键回退，做法是**发同一条命令、带旧值**，
账本两条线都留着。后端这一侧因此只欠一条性质：设置类命令重复发送与发送一次等价。
`set_autonomy` 与 `select_model` 折叠取最后一条，故成立（`crates/sprawling/tests/undoing.rs`）。
**两处不成立，都需要它们各自的卡**：

1. `attach` 没有反向动词——`Command` 里没有 detach，`EndpointLost` 只由 `E_ENDPOINT_DIALECT_UNSUPPORTED` 的
   carrier 产生，没有命令能写它。撤销 attach 要么加 `Command::DetachEndpoint`（wire 变更），要么这一格不做。
2. `select_model` 的反向命令要求「上一次选择」存在且那个端点仍然挂着；第一次选择之前没有可回退的值。

### 8-46-12 两条接力链各有上限

**一条没有人在里面的链不许无限长。** H-04 把七个入口都送进车道之后，敲门链与继任链不再堵住
记账线程，`Halt` 与 `Cancel` 也读得进来了；但两条链本身仍然无界——A 叫醒 B，B 再叫醒 C，
或者一轮活连着把自己交给下一轮，每一步都花一次真跑，而没有人答应过这场开销。§8-13 早先
记的口径是「不设叫醒预算，什么时候停下是对话里那几位居民的事」；那条判断成立的前提是
链条在记账线程上跑，它的唯一刹车就在被它堵住的线程上。刹车够得着之后，剩下的问题是
**没有人在场时谁来停**，而一个由居民自己决定长度的环不会自己停。

```rust
/// 一条链把同一件活带到了哪里。两个计数器分开，因为两条链回答不同的问题：
/// 这场对话敲醒过几个人，这件活换过几次人。
#[derive(Clone, Default)]
struct Relays { chain: KnockChain, successions: u32 }

/// 一轮活在叫醒它的那场对话里站在哪儿：`hops` 是本分支的深度，只有本分支推它；
/// `woken` 是整场对话一共叫醒了几轮 run，所有分支共用同一个计数。
#[derive(Clone, Default)]
struct KnockChain { hops: u32, woken: Arc<AtomicU32> }

/// 一次敲门链最多唤醒这么多轮 run；一次继任链最多接力这么多轮；
/// 一场对话的所有分支加起来最多唤醒这么多轮。
const CONVERSATION_HOPS_MAX: u32 = 16;
const SUCCESSION_HOPS_MAX: u32 = 64;
const CONVERSATION_RUNS_MAX: u32 = 64;

impl Owing {
    /// 由敲门起的那轮活欠什么：对话向前一跳，超上限即 `E_LOOP_SUSPECTED`。
    fn knocked(chain: KnockChain) -> Result<Owing, AxError>;
    /// 继任者接过的同一份义务：接力向前一跳，超上限即 `E_LOOP_SUSPECTED`。
    fn after_succession(&self) -> Result<Owing, AxError>;
}
struct Knock { addr: Address, from: String, mode: kernel::Mode, chain: KnockChain }
fn knock(&mut self, signal: &Signal, speaker: &Address, mode, chain: &KnockChain) -> Result<(), AxError>;
```

- **接力次数记进义务，不记进工人**。两个计数器随 `Owing` 走：继任者拿走的是前任的义务，
  故它自然继承并加一；敲门推的是 `Knock { chain }`，`answer_knocks` 由此造出的新
  一轮活从上一跳加一。委派的子活继承原值而不加——换的是干活的人，不是这条链的位置。
  计数器不记在 `RunWorker` 上：那正是 C15 删掉的形状（一个 worker 字段描述的是碰巧在
  落地的哪一轮活），落地完成后字段属于谁没有答案。
- **一个上限，不是一份预算**。上限管的是「门敲下去会不会没完没了」，不给一轮活定价、
  不改居民之间能谈多少轮：`16` 是「十六个居民被连着叫醒之后这已经不是一场对话，是一个环」
  的位置，`64` 是「同一件活换过六十四个住户之后不管人在不在看都该停下」的位置。两个数字
  都是刹车而不是调过的参数，故都在明处，重开一次对话或再派一次活即可继续。
- **深度之外还有宽度**。只限深度时，一轮活给 N 位居民发信、每位再给 N 位发信，十六跳之内
  叫醒的 run 按 N 的幂增长；被叫醒的 run 又是 `Root` 深度，可以再委派，「一层深」管不住
  一条链的总量。所以 `KnockChain.woken` 由整场对话的所有分支共用（`Arc<AtomicU32>`，
  随最后一个持有者一起释放，不在 `RunWorker` 上留一张永不清空的表），`after_knock` 先判
  `hops`、再原子地把 `woken` 加一并判 `CONVERSATION_RUNS_MAX`，两处超限都是同一个
  `E_LOOP_SUSPECTED` 与同一句恢复语。委派的子活共用父的 `KnockChain`，所以子活发的信
  也记在同一场对话的账上。`64` 与继任上限同值，同样是刹车：一场对话叫醒了六十四轮之后，
  它已经是一次广播而不是一场对话。
- **敲门超限不连坐发件人**。超限在 `answer_knocks` 里判：这一敲不开始新一轮活，落一条
  `Refuse` 诊断（带地址与拒绝的 subject）后继续下一敲；信已经在房间里，人仍可从 Inbox
  读它。继任超限在 `conclude` 里判：诊断落 `Refuse`，拒绝随 `hand_back` 回给要这轮活的
  那一位（`Asked` 到人，`Child` 到父房间，无人时成诊断行），**本轮活照常 `discharge`**
  ——链断在第一端，不能把已经跑完的那一轮的结局一起吞掉。
- **敲门先看房间里有没有人**。`answer_knocks` 在派一敲之前问 `RoomQueues::worked_by`：房间的队列
  借出去了，就说明有一轮活正在那里读，此时再派一轮只会拿到一个空的备用信箱（`QueueTenure::ASpare`），
  而信在 `Lent.waiting` 里要等持有者落地才回家。所以这一敲不派，存进 `Doorstep.deferred`（每个房间
  一敲），持有者 `give_back` 之后由 `return_borrowed` 放回 `knocks`，本轮落地末尾的 `answer_knocks`
  再派它——这时信已经在房间队列里，新一轮活读得到。判定只在派的那一刻做一次，而不是在 `knock` 入队时，
  因为从入队到派出之间 `conclude` 可能已经把别的活派进同一个房间。
- **handback 与普通的信走同一个敲门判定**。`discharge` 的 `Child` 分支投完 handback、派完图里新就绪的
  节点之后，调用同一个 `knock`：说话者是子房间，模式是子活的模式，对话计数是子活继承来的那一个
  （敲醒时照常加一）。父房间的图还有节点在外面时不敲——那几个节点会各自回来，最后一个回来、图被丢下时
  才敲一次，免得每回来一个节点就花一轮父的真跑。父房间没有 `URBANITE.md` 时照旧不敲，结果在它的信箱里等人。

**本章测试**：`accounting::worker::waking::tests` 里 `a_knock_at_a_room_somebody_is_working_in_waits_for_them_to_leave`
（房间借出时一敲不开 run，归还之后同一敲开 run）与 `what_comes_back_wakes_the_resident_who_asked_for_it`
（子活落地后父房间的居民被敲醒，brief 写明是子房间说的话）；`owing::tests` 里两个边界（上限减一放行、到达上限拒绝，拒绝码
`E_LOOP_SUSPECTED` 且恢复语非空）与 `a_conversation_that_fans_out_stops_at_its_width`（同一场对话
六十四次敲门放行、第六十五次拒绝）；`accounting::worker::waking::tests` 里一条 `hops` 为 `u32::MAX`
的敲门不开始任何 run（对照：既有的 `a_signal_wakes_the_resident_it_was_sent_to_and_says_who_spoke`
证明上限之下一敲照常开跑）。
-/

/-!
## 8-52 新客户端要问的四件事，服务端怎么答（`accounting::views::listing`、`accounting::views::document`、`views::rounds`、`views::holding`；`crates/wire/Spec.lean` §8-23）

- **`Views.governance: views::Governance`**（原 `Views.halted`／`Views.approvals`／`Views.autonomy` 三个散字段）。停摆作用域、待答项与自治档由 `city_halted`／`approval_requested`／`approval_resolved`／`autonomy_changed` 折出，**折法只有 `Governance::absorb` 一处**：工作线程持一份判定面，`Views` 持一份读面，同一个类型、同一遍折，重建即相等（`views::tests`、`what_a_worker_holds_is_what_a_restart_rebuilds`）。此前 `Views` 把这四条记录另拼了一遍，两份拼法对「读不懂的决定算什么」答得不一样。`CityAnswer.halted` 是 `governance.halted` 的 `Vec` 形。
- **`summarize` 把 `RunHot.addr`／`started`／`completion`／`pr`／`ask` 抄进 `RunSummary`**，不读账本（`crates/wire/Spec.lean` §8-48）。
- **`rounds_answer` 多读两条记录**：窗口里第一条 `run_started` 成 `Opening { task, goal, at: record.t() }`，第一条 `run_frozen` 成 `Closing { completion, at }`；`turns()` 本身一字不动。
- **`accounting::views::listing`**（新文件）：`at` 为 `None` 读城根，否则读 `city_root/<at>`；`read_dir` 一层，目录在前、文件在后、各按名字 UTF-8 序；读不了的目录答空表而不是拒绝——同 `read_building` 的口径，一个读不了的目录在页面上是一个空目录。符号链接按 `metadata` 判：指向目录的算目录。文件大小 `u64`。
- **`accounting::views::document`**：路径同上；答什么、怎样判文本、在哪里切，见 `crates/accounting/Spec.lean` §8-21 与 `crates/wire/Spec.lean` §8-69。**不经密钥扫描**：这是城内的文件给城的主人看，而 `Hunks` 的扫描针对的是把补丁文本挂上线的那条路——但 `.sprawling/CONFIG.toml` 里只有 `secret:` 引用，明文本来就不落盘（`xtask secret` 门保证），所以这里没有可泄露的东西。
- **验收**：`views::listing::tests`——`init_city` 铺出的城根列出 `.sprawling` 与 `hall` 两个目录；`hall` 下列出 `Roadmap.md` 等文件且目录先于文件；不存在的路径答空表。`views::document::tests`——读城里 `hall` 楼自己的 `RULES.toml` 得到原文，第一个窗口盖住整份；其余见 `crates/accounting/Spec.lean` §8-21。`views::rounds::tests`——三条记录的会话答出 `opening.task`；冻结后答出 `closing.completion == "done"`。`views::tests`——`city_halted` 后 `city_view.halted == ["city"]`，`released` 后为空。
-/

/-!
## 8-53 一座楼做过的提交，倒序分页（`accounting::views::commits`、`views::holding`；`crates/wire/Spec.lean` §8-24）

- **`Views.commit_seqs: BTreeMap<Seq, GitOid>`**，与按 oid 键的 `commits` 表由 `fold_commit` 同一处写入：一条记录若宣告了提交，两张表各得一行。按 oid 的表答 `Commit`，按 seq 的表答 `Commits`；两表从同一条流折出，重建即相等。
- **`commits_answer(building, before, limit)`**：在 `commit_seqs` 上从 `before` 的独占上界（`None` 即尾）向前走，按 `actor` 地址前缀过滤（`actor == building` 或以 `<building>/` 起头；`None` 不过滤），取 `limit.clamp(1, HISTORY_MAX)` 条；再多走一步得 `more`。每条经 `CommitFacts::answer` 与 `lineage_of` 成 `CommitAnswer`，故列举与反查答同一形状。
- **验收**（`views::commits::tests`）：三条不同 seq 的 `checkpoint_committed`（两条在 `lab/room1`、一条在 `hall/mayor`）折入后，按 `lab` 列举答两条且 seq 递减、`more == false`；`limit: 1` 答一条且 `more == true`；以那条的 seq 作 `before` 再问答下一条；按 `hall` 列举不含 `lab` 的提交；`None` 答三条。
-/

/-!
## 8-128 一次提交带出同一次 run 的上一个提交，与它的父提交（`accounting::views::commits`；`crates/wire/Spec.lean` §8-54）

- **`CommitFacts.previous: Option<wire::CommitAt>`**，`fold_commit` 写入：`Views.last_commit: BTreeMap<RunId, CommitAt>` 记每次 run 最近宣告的那个提交，一条记录宣告提交时，先把这张表里同一 run 的那一项取作 `previous`，再换成本提交。同一个 oid 被宣告第二次时不拿自己当 `previous`，沿用第一次折出的那个。折叠从快照起步也一样：`last_commit` 随视图进快照，`VIEWS_FOLD_RULES` 随编码变。
- **`parents` 不进视图。** `Query::Commit` 与 `Query::Commits` 在锁内由视图答出其余各字段，交给 `Prepared::Commits(CommitsAsk)`；锁放开之后 `CommitsAsk::read` 经 `storage::changes::parents_of` 一次开仓库读这一页每个 oid 的父提交。读不到仓库时 `parents` 全为 `None`，答复照样发出：父提交是这一行多出的一格，不是它成立的条件。§8-167 说的「没有一个字段是从 git 读的」指 trailer 能答的那几项；父提交 trailer 答不出，它的权威是提交对象本身。
- **验收**（`views::commits::tests`）：同一 run 的两条检查点之间夹着另一 run 的一条，第二条的 `previous` 指向第一条（oid 与 seq），另一 run 的那条与第一条的 `previous` 为 `None`；一个真仓库里的两个提交经 `CommitsAsk::read` 答出后一个的父提交是前一个，前一个是根提交（`Some(vec![])`），不在仓库里的 oid 答 `None`。
-/

/-!
## 8-106 城景有界（`accounting::views::answering`、`storage::hot`；`crates/storage/Spec.lean` §8-5、`crates/wire/Spec.lean` 的 `CityAnswer`）

- **`CityView` 的 `runs` 取自 `HotView::runs`**：热视图只留活跃的全部，加 `last_seq` 最近的 `storage::RECENT_FROZEN` 个冻结跑（`crates/storage/Spec.lean` §8-5），按 RunId 序。`active`／`frozen` 两个数仍是全城的数，页面拿 `frozen` 减去列表里冻结的行数，就知道还有多少在列表之外；那些跑经分页的 `History`／`RunHistory` 读。理由：城景是页面最常问的答复，它的大小原先与城的全部历史同阶（8,000 次跑的城约 1.37 MB），有界之后只与活跃数同阶。答复的语义变了，`WIRE_V` 加一。
- **客户端的 `staleBy` 只在城景真会变的记录上作废它**：`run_started`、`run_frozen`（列表的成员变了），楼与 pursuit 的那组记录（`buildings`／`pursuits`），`city_halted`（`halted`）。一次跑中途的记录只推进 `last_seq`／`last_kind`，页面自己的折叠（`core/belief`）已经从同一条记录读到了，重拉城景只是把同一件事再运一遍。
- **`CostView` 的 `by_run` 同样有界**：活跃的跑一个不少（顶栏「这次跑花了多少」读的正是正在动的那次），其余从 `Attribution` 仍持有行的跑（即热视图没逐出的冻结跑，见下）里只取花得最多的 `views::answering::TOP_BILLED`（32）个，同额按名字，按名字序输出。选法是对非活跃行做一次 `select_nth_unstable_by`，O(runs)。`total` 仍是全城的权威总额，`by_run` 的和因此可以小于它；界面本就按 `total` 算占比，列表之外的钱留作看得见的余额，而不是被摊掉。另外四个维度（actor、segment、tool、skill）的桶数不随跑数增长，不截。`TOP_BILLED` 与 `RECENT_FROZEN` 一样是线上答复的大小上界，不随机器变。
- **`RunView` 问到一次被逐出的跑时读冷的一侧**：`hot.get` 答不出而 `hot.was_evicted` 认得它，就从账本索引取这次跑的全部序号，按序号从旧到新把它的记录折进一个只装这一次跑的新 `HotView`，答那一行。折法仍是 `HotView::apply`，冷热两侧没有第二份「一条记录怎么变成一行」。代价是这次跑的记录数，只在有人打开一次旧跑时付。既不在热视图里、也没有墓碑的跑答 `None`；账本读不出时答 `Unavailable`，因为被逐出的跑必有记录，读不到是「没能看」而不是「没有这次跑」。
- **一次跑花了多少只有一处答：`Views::billed_to`**（`views::billed`）：`Attribution::billed_to` 持这次跑的行就答它；没有行而热视图逐出过它，就从账本索引取这次跑的全部序号，按序号从旧到新折进一个只装这一次跑的新 `storage::Attribution`，再读它的行——与 `RunView` 的冷侧同一种读法，折法仍是 `Attribution::apply`，冷热两侧没有第二份计价；两者都不是，这次跑没花钱，答零。账本读不出时答 `None`。`CostOf` 的逐跑明细、`Commit`／`Commits` 的 `spent` 与 `RunCosts` 都经它，不再为每一行先把整份 `report()` 折出来。
- **`RunCosts { runs }` 是 `by_run` 之外的冷查询，一问最多一页**：一次最多 `wire::RUN_COSTS_MAX`（64）个 RunId，按问的顺序答 `(run, spent)`，超出的部分不答；楼的目录页按新到旧列跑、只问一次，所以它只显示最新的 `RUN_COSTS_MAX` 个冷跑的花费，更旧的跑显示为未知；账本读不出的跑不出行，于是「没出行」是「没能看」，「零」是「没花钱」。楼的目录页对不在 `cost_view.by_run` 里的跑用它。新增一条 Query，`WIRE_V` 加一。被否：按 RunId 游标翻全城的 `by_run`——那要么让 `Attribution` 继续为每次跑留一行，要么每页扫一遍整本账本；按名字问，代价只与被问的跑的记录数同阶。
- **`Attribution` 的 `by_run` 只留热视图没逐出的跑**：一条 `run_frozen` 折进之后、或一条记录落在已逐出的跑上时，`Views::apply` 让 `Attribution::retain_runs` 丢掉热视图逐出的那些跑的行；`total` 与另外四个维度不动。于是 `by_run` 的行数与活跃数加 `RECENT_FROZEN` 同阶，被丢的钱经 `billed_to` 的冷侧仍答得出。
- **验收**（`views::standing_tests`、`a_city_of_eight_thousand_runs_answers_in_a_bounded_view`）：折入 8,000 次开始又冻结的跑后，`city_view` 序列化成 JSON 不超过 16 KiB，列出的正是最近冻结的 `RECENT_FROZEN` 个，`frozen == 8000`。
- **验收**（`views::standing_tests`、`an_evicted_run_still_answers_its_run_view`）：`RECENT_FROZEN`＋1 次跑都开始又冻结、写进账本后，最早那次已被逐出热视图，`RunView` 仍答出它：冻结、房间与开始时刻都在。
- **验收**（`views::standing_tests`、`a_cost_view_of_eight_thousand_billed_runs_names_the_top_few`）：8,000 次各自计费的跑都冻结后，一个仍在跑的也计了费；`cost_view` 的 `by_run` 恰是那次活跃的跑加花得最多的 `TOP_BILLED` 个，`total` 仍是全部的和。
- **验收**（`views::standing_tests`、`an_evicted_run_still_answers_what_it_cost`）：`RECENT_FROZEN`＋1 次各自计费的跑都开始又冻结、写进账本后，`RunCosts` 问最早那次与一个从没出现过的跑，答出前者计的费与后者的零，按问的顺序。
- **验收**（`views::standing_tests`、`the_attribution_holds_only_the_runs_the_hot_view_holds`）：同一座城里，`Attribution` 的 `by_run` 不再有被逐出的那次跑，`total` 仍是全部的和，`RunCosts` 对它仍答出它计的费。
-/
