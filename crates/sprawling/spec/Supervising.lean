-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 守护的崩溃预算：崩溃 → `resume` → `serve`，一分钟三次就停下等人

规定 `crates/sprawling/src/supervising.rs` 的 `CrashBudget::after`（`bin::supervising`，形状 1 decision；§8-109）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。起子进程、读退出状态、等人按 Enter 归 `bin::supervising::children`，这里不建模。

时刻是守护起步以来的毫秒数，取自单调时钟，所以在模型里是自然数；「距今」是自然数的截断减法，与 Rust 的 `saturating_sub` 相同。

五条性质：

* 人选择的收口（`Chosen`）总是停下；
* 一次重启带出的预算少于 `limit` 次崩溃、每一次都还在窗口里、最后一次就是这一次——预算占的内存有界；
* 窗口里凑满三次即 `degraded`，不再重启；
* 崩溃之间隔满一个窗口时永远重启，预算里只有这一次；
* 退出码 2 也是崩溃（§8-109 决定 2）：`Closing` 只分人选的与坏掉的两种，没有第三种「不必重启」。
-/

namespace Sprawling.Supervising

/-- 窗口与次数（Rust：`CRASH_WINDOW_MS`、`CRASH_LIMIT`）。 -/
def crashWindowMs : Nat := 60000
def crashLimit : Nat := 3

/-- 子进程怎么收口（Rust：`accounting::worker::Closing`，由 `Closing::of` 读退出状态：0 为 `chosen`，其余一切为 `broken`）。 -/
inductive Closing where
  | chosen
  | broken (cause : String)
  deriving Repr, DecidableEq

/-- 窗口里的崩溃时刻，从旧到新（Rust：`CrashBudget`）。 -/
abbrev CrashBudget := List Nat

/-- 下一步（Rust：`Next`）。 -/
inductive Next where
  | stop
  | restart (budget : CrashBudget)
  | degraded (crashes : Nat) (cause : String)
  deriving Repr, DecidableEq

/-- 还在窗口里的旧崩溃，再加上这一次。 -/
def recent (budget : CrashBudget) (at_ : Nat) : CrashBudget :=
  budget.filter (fun c => decide (at_ - c < crashWindowMs)) ++ [at_]

/-- `CrashBudget::after`。 -/
def after (budget : CrashBudget) (closing : Closing) (at_ : Nat) : Next :=
  match closing with
  | .chosen => .stop
  | .broken cause =>
    if (recent budget at_).length ≥ crashLimit then .degraded (recent budget at_).length cause
    else .restart (recent budget at_)

theorem chosen_stops (budget : CrashBudget) (at_ : Nat) : after budget .chosen at_ = .stop := rfl

/-- 一次重启带出的预算：少于 `crashLimit` 次，每一次都在窗口里，最后一次就是这一次。 -/
theorem restart_is_bounded (budget kept : CrashBudget) (cause : String) (at_ : Nat)
    (h : after budget (.broken cause) at_ = .restart kept) :
    kept.length < crashLimit ∧ (∀ c ∈ kept, at_ - c < crashWindowMs) ∧ kept.getLast? = some at_ := by
  have hl : ¬ (recent budget at_).length ≥ crashLimit := by
    intro hl; simp [after, hl] at h
  have hk : kept = recent budget at_ := by
    simp [after, hl] at h; exact h.symm
  subst hk
  refine ⟨by omega, ?_, by simp [recent]⟩
  intro c hc
  simp [recent, List.mem_filter] at hc
  rcases hc with ⟨_, hc⟩ | hc
  · exact hc
  · subst hc; simp [crashWindowMs]

/-- 一分钟之内第三次崩溃：停下等人。 -/
theorem third_crash_in_a_window_degrades (a b c : Nat) (cause : String)
    (hab : a ≤ b) (hbc : b ≤ c) (hw : c - a < crashWindowMs) :
    ∃ k₁ k₂, after [] (.broken cause) a = .restart k₁ ∧
      after k₁ (.broken cause) b = .restart k₂ ∧
      after k₂ (.broken cause) c = .degraded 3 cause := by
  have hb : b - a < crashWindowMs := by omega
  have hc : c - b < crashWindowMs := by omega
  refine ⟨[a], [a, b], ?_, ?_, ?_⟩
  · simp [after, recent, crashLimit]
  · simp [after, recent, crashLimit, hb]
  · simp [after, recent, crashLimit, hw, hc]

/-- 崩溃之间隔满一个窗口：每一次都重启，预算里只有这一次。 -/
theorem spaced_crashes_always_restart (last at_ : Nat) (cause : String)
    (h : last + crashWindowMs ≤ at_) :
    after [last] (.broken cause) at_ = .restart [at_] := by
  have : ¬ at_ - last < crashWindowMs := by omega
  simp [after, recent, crashLimit, this]

end Sprawling.Supervising

/-!
## 8-109 `sprawling up --supervise`：崩溃 → `resume` → `serve`（`bin::supervising`、`bin::supervising::children`）

`CrashBudget::after` 必须守住的性质的权威是本文件上面的模型：人选的收口总是停下，重启带出的预算有界且都在窗口里，一分钟内第三次崩溃即 degraded，隔满一个窗口的崩溃总是重启；本节是接口、子进程的读法与理由。

**要什么。** 一座城的服务进程崩了，人不在键盘前时没有人把它拉起来。`--supervise` 让 `up`（与 `serve`，两者共用一张 flag 表与同一个 `serve_city`）不自己服务，而是守着一个子进程：子进程就是 `sprawling serve <city> <addr> …`，崩了先跑一次 `sprawling resume <city>`（验链、收掉进程死亡留下的悬空调用），再起一个新的 `serve`。

**两个模块。**

- `bin::supervising`（形状 1 decision）：`CrashBudget` 与它的判定，无 I/O、无时钟，时间作为参数进来。

  ```rust
  pub(crate) const CRASH_WINDOW_MS: u64 = 60_000;
  pub(crate) const CRASH_LIMIT: usize = 3;
  pub(crate) struct CrashBudget { /* 窗口内的崩溃时刻，最多 CRASH_LIMIT 个 */ }
  pub(crate) enum Next { Stop, Restart(CrashBudget), Degraded { crashes: usize, cause: String } }
  impl CrashBudget {
      pub(crate) fn fresh() -> Self;
      pub(crate) fn after(self, served: &Result<(), AxError>, at: TimeMs) -> Next;
  }
  ```

  `Ok` → `Stop`；`Err` 记下这一刻，丢掉距今已满 `CRASH_WINDOW_MS` 的旧崩溃，窗口里凑满 `CRASH_LIMIT` 次即 `Degraded`，否则 `Restart`。
- `bin::supervising::children`（形状 4 adapter）：`pub fn supervise(city: &Path, addr: &str, child: &Child) -> Result<Ended, AxError>`，`pub enum Ended { Chosen, Degraded }`。它起子进程、等它退出、按退出状态造一个 `Result<(), AxError>` 交给 `CrashBudget::after`，自己不做判断。交给 `after` 的时刻是守护起步以来经过的毫秒数，取自单调时钟 `serving::standing::monotonic_now`，不是墙钟：系统把墙钟往回拨会让一次旧崩溃一直留在窗口里，往前拨会让一次新崩溃提前出窗口。

**子进程的退出怎么读。** 退出码 0 是 `serve` 有序收口、写完交接的结果，读作 `Ok`，守护随之结束；其余一切（非零码、被信号杀掉而没有码、收口中不写交接就退出）读作 `Err`，文字是退出状态。是谁关的城、怎样关的写在子进程自己的交接里（`accounting::worker::Closing`），守护只读退出码，不另造一套分类。

**degraded 要人显式解除。** 窗口里第三次崩溃后守护不再重启，打印最后一次的 `cause` 与崩溃次数，然后等标准输入的一行：人按 Enter 即解除（预算清零，`resume` 后再 `serve`）；读到 EOF（没有人在）即以失败退出；终端读不了时先打印读失败的原因，再同样以失败退出。**原因**：一个 60 s 内崩了三次的服务，再拉起来多半还是崩，且每次崩溃都可能留下一条悬空调用；让它无限重启是把一个需要人看的故障藏进循环里。**否决的方案**：指数退避后永远重试——那样故障永远不会被人看见。

**子进程的那一行。** 开不开浏览器（`opening()`）与进不进控制台（`wanted`）都由 `serve_city` 算一次，守护只把结果变成 flag：`pub struct Child { forwarded, first: Window, console: Console }`。只有第一次起的子进程带 `--open`（当 `opening()` 判为 `Open::Browser`），之后每次都带 `--no-open`，因为重启不该每次弹一个新窗口；子进程带 `--console` 当且仅当不带 `--supervise` 的同一行会进控制台，否则带 `--no-console`。其余 flag 由 bin 的 `verbs::forwarded` 按 `SERVED` 表的 `Takes::Value` 连同其值原样转给 `serve`，守护不再解析 argv。子进程继承这个终端，所以终端的面（CLI 或安静宿主，§8-11）属于子进程；守护自己不进 raw 模式，也不印事件。

**不做的事。** 不接管开机启动：守护活在人起它的那个终端里，终端关了它就走。锁（S5.02 的单写者锁）由子进程持有、随子进程死亡释放，守护本身不碰城的锁，所以 `resume` 与下一个 `serve` 都拿得到它。子进程拥有终端时，敲出来的 Ctrl+C 是子进程的一个键，不是信号，两个进程都不停；信号形式的 Ctrl-C 与 Ctrl+Break 同时落到守护与子进程：子进程照 §8-11 收口写 handoff，守护不拦截信号。

**决定。**

1. 守护与服务是两个进程，不是同一进程里的一个 `catch_unwind`：一次 abort、栈溢出或内存耗尽不回到 unwind，只有进程边界接得住。
2. 退出码 2（命令行被拒、没有城）也算崩溃，交给预算，而不单列一种「不必重启」：那是第二套分类；三次之内它就进 degraded，人看得见原因。
**尚未做到的（本节接口的当前状态）**：查询仍在锁内作答，`GitStatus` 等做 I/O 的查询仍在锁内做 I/O，所以读者之间、以及读者与折叠线程之间仍会互等；发布 `Arc<ViewsSnapshot>` 供查询无锁读取、把 I/O 移到锁外（锁内只取所需的小数据），是这一接口余下的两步。

### 8-110 `RunWorker` 按所持状态拆开：凭据一组（`accounting::worker::credentials::held`）与协作一组（`accounting::worker::collaborating`），形状 1 数据

```rust
// accounting::worker::credentials::held —— shape: data
pub(in crate::assembly) struct Credentials {
    pub(in crate::assembly) book: gateway::EndpointBook,        // 接了哪些端点、每个 tag 选了哪个模型
    pub(in crate::assembly) vault: Arc<Mutex<gateway::Custodian>>,
}
impl Credentials {
    pub(in crate::assembly) fn opened(book, vault: gateway::Custodian) -> Credentials;
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError>;
}
// accounting::worker::collaborating —— shape: data
pub(in crate::assembly) struct Collaborating {
    pub(in crate::assembly) rooms: RoomQueues,                                   // 每个房间的队列，以及哪个 run 借着它
    pub(in crate::assembly) joins: BTreeMap<Address, collab::FanIn>,             // 每个房间从下派的工作收回了什么
    pub(in crate::assembly) workshops: BTreeMap<Address, collab::Underway>,      // 每个房间摆出、尚未全部汇合的图及其已派集；handback 到达时由它派下一组
    pub(in crate::assembly) requests: Vec<collab::OpenRequest>,                  // 等人检查的 pull request
    pub(in crate::assembly) goals: Vec<kernel::GoalEntry>,                       // 居民认领的地盘，按认领顺序
}
// accounting::worker::recording
impl RunWorker {
    fn absorb(&mut self, kind: EventKind, run: RunId, addr: Option<&Address>, data: &Payload) -> Result<(), AxError>;
}
```

`RunWorker` 拆成六个对象：凭据、治理、协作、计划、入口、飞行中的 run。每个对象带走自己的字段、方法与测试，`RunWorker` 只持有这六个对象和账本、CAS、日志这些整城共用的东西。凭据一组是 `book`、`vault` 两个字段：它们回答同一个问题——这座城能以谁的身份去叫哪个模型——而且改动它们的是同一族记录（`endpoint_attached`、`model_selected`、`endpoint_lost`、`secret_captured`）。协作一组是 `rooms`、`joins`、`requests`、`goals` 四个字段：它们都从信号、handback、pull request 与 `goal_registered` 这几族记录折出，回答的是「居民之间正在交接什么」。`pursuits`、`plan_holders` 与 `delegator` 虽然也由协作折叠（`folds::Collaboration`）折出，却回答「每栋楼在朝什么推进」，属于计划一组，不归这里。治理早已是一个对象（`views::Governance`），判定面与读面共用那一个定义，这一步不动它。计划、入口、飞行中的 run 三组见 §8-111。

**worker 写下的每一行，会话起点、治理、计划、凭据四份折叠都要看到，不论这行以城的名义还是以某个 run 的名义写。** 重启走 `Standing::fold`，那条路把账本上每一行都交给每一份折叠，不问是谁写的；活着的 worker 若只在以城的名义写时才折凭据与会话起点，一行由 run 写下的 `endpoint_attached` 就在账本上、却不在 worker 的 `book` 里，直到进程重启——这正是 §8-17 已经排除的那类「活城与重启折出两份」。所以 `record_where` 与 `record_for` 都经过同一个 `RunWorker::absorb`，它把这一行交给会话起点、治理、计划、凭据四份折叠，每份都看过之后才交出第一份的错误：行已经在账本上，一份折叠出错不该让排在它后面的折叠漏看这一行、在重启之前与重启折出两份。哪份折叠看哪几种记录由各自的 `absorb` 决定，这里不再筛。协作一组与入口不经过 `absorb`：协作由写下那一行的效果处理器当场改，入口由 `entrance.stamp` 改；重启时 `Standing::fold` 把每一行也交给这两份折叠。

**红**：一个 run 以自己的名义写下一行 `endpoint_attached`（`record_for`），随后 worker 的 `book` 与从同一账本重折出来的 `book` 应当列出同样的端点。改动之前，worker 的 `book` 为空而重折的那份有这一端点。

**为何字段仍是 `pub(in crate::assembly)`**：这一步是纯搬移，读写这些字段的调用只把 `self.book` 改拼为 `self.credentials.book`、`self.rooms` 改拼为 `self.collaborating.rooms`；把它们收进 `Credentials` 的方法是另一次改动，混进来会让搬移与行为变化无法分开审。

### 8-111 计划、入口、飞行中的 run 三组离开 `RunWorker`（`accounting::worker::plans::held`、`accounting::worker::doorstep`、`accounting::worker::driving::flight`），形状 1 数据

```rust
// accounting::worker::plans::held —— shape: data
pub(in crate::assembly) struct Planning {
    pub(in crate::assembly) pursuits: BTreeMap<Address, kernel::Pursuit>, // 每栋楼在朝什么推进
    pub(in crate::assembly) delegator: kernel::Delegator,                // 深度零的位置：只有它能宣布一个 pursuit
    pub(in crate::assembly) holders: PlanHolders,                        // 每栋楼的计划里，哪个节点由哪个房间认领着
    pub(in crate::assembly) write_plan: fn(&Path, &[u8], &[u8]) -> Result<(), AxError>, // 落地替换 Roadmap.md 的那一扇门（§8-42-8）
}
impl Planning {
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, addr: Option<&Address>, data: &Payload);
}
pub(in crate::assembly) struct PlanHolders(BTreeMap<Address, BTreeMap<kernel::NodeId, String>>);
impl PlanHolders {
    pub(in crate::assembly) fn absorb(&mut self, kind: EventKind, addr: Option<&Address>, data: &Payload);
    pub(in crate::assembly) fn in_building(&self, building: &Address) -> BTreeMap<kernel::NodeId, String>;
}
// accounting::worker::doorstep —— shape: data
pub(in crate::assembly) struct Doorstep {
    pub(in crate::assembly) entrance: Entrance,   // 这座城按 key 答过哪些命令、答了什么（§8-41）
    pub(in crate::assembly) knocks: Vec<Knock>,   // 没人在家时被搭话的居民，等说话的 run 冻结后再叫醒
}
impl Doorstep {
    pub(in crate::assembly) fn opened(entrance: Entrance) -> Doorstep;
}
// accounting::worker::driving::flight —— 已有的 Flight 收下一个字段
pub(in crate::assembly) struct Flight {
    pool, gate, driving, homes,                                          // 原有：在 lane 里的 run、写历史的关口、回家的顺序
    pub(in crate::assembly) backlog: runtime::Backlog,                   // run 留下仍在跑的命令，halt 从这里找到它们
}
```

计划一组是 `pursuits`、`delegator` 与认领表三样：它们回答「每栋楼在朝什么推进、谁在做哪一块」，而 `pursuit_changed` 与 `roadmap_*` 这几族记录只改动它们。`delegator` 跟着 `pursuits` 走，因为宣布一个 pursuit 只能经过深度零的位置，而这个位置只在开城时铸一次（`lifetime`）。

入口一组是 `entrance` 与 `knocks`：两样都是「已经到了城门口、还没变成 run 的工作」——按 key 来的命令、居民之间的搭话。名字取 `Doorstep` 而不是「入口」的直译，因为 `Entrance` 已经是其中按 key 去重的那一份（§8-41）。飞行一组把 `backlog` 收进已有的 `Flight`：`backlog` 是 lane 里的 run 留下仍在跑的命令，和 `Flight` 一样一城一份、只随 run 的起落变化；检查点不在这里取锁，规则见 §8-46-13。这两组是纯搬移，读写处只把 `self.knocks` 改拼为 `self.doorstep.knocks`、`self.backlog` 改拼为 `self.flight.backlog`。

**认领表只有一份定义。** `roadmap_claimed` 把房间记进它所属楼的表，`roadmap_finished`、`roadmap_released`、`roadmap_blocked` 与拆分父节点的 `roadmap_split` 把节点移出（拆分之后这一轮什么也不持有，§8-42-8）；房间就是这行记录的 `addr`，楼是 `addr` 的第一段，节点是载荷的 `node`。重启的协作折叠与活着的 worker 调用同一个 `PlanHolders::absorb`，所以两边不会各写一份规则。一行读不出楼或节点的记录不进表：认领表只记确知的持有者，不去猜。
**认领表只有一份定义。** `roadmap_claimed` 把房间记进它所属楼的表，`roadmap_finished`、`roadmap_released`、`roadmap_blocked` 把节点移出；房间就是这行记录的 `addr`，楼是 `addr` 的第一段，节点是载荷的 `node`。重启的协作折叠与活着的 worker 调用同一个 `PlanHolders::absorb`，所以两边不会各写一份规则。一行读不出楼或节点的记录不进表：认领表只记确知的持有者，不去猜。

**红**：一个 run 落下一行 `roadmap_claimed`（`record_for`，认领效果正是这样落地的），随后 worker 读到的持有者（`holders_in`）应当与从同一账本重折出来的一样。改动之前，worker 的表在开城之后再不更新：左边是空表，右边是 `{2: "lab/room1"}`。

**`pursuits` 由 `plans` 直接改写，且只在 `pursuit_changed` 落账之后改**：宣布、暂停、恢复、撤下一个 pursuit 时，`plans` 先判定（有没有计划、有没有正在追的目标）、铸出要宣布的 `Pursuit` 并算出要写的 `goal`，追加 `pursuit_changed`，追加成功后才改 `Planning::pursuits`。追加失败时进程里的 pursuit 不变，与重启从账本折出的一致。被否：先改表再追加——追加一失败，活着的 worker 就持有一个账本从未记下的 pursuit。被否：让 `Planning::absorb` 折这一行、由记录铸回 `Pursuit`——铸造要经深度零的 `Delegator`，把它交给折叠会让每个折叠点都能宣布目标。

### 8-92 装配根与 serving、doctor 之间的依赖只朝一个方向

`bin::assembly` 是 `sprawling` 里唯一知道所有具体类型的地方，别的模块不应当反过来知道它（ARCHITECTURE.md §3）。一条从 serving 或 doctor 指回装配根的边，意味着改装配根的内部可能改坏一个读面，而读面本来只该依赖它读的那份事实的权威。城的唯一写者与读面都住 `accounting` 之后，装配根只剩自由函数与直接碰主机的生产适配器（`crates/accounting/Spec.lean` §8-11、accounting D12）：`production`（`SystemClock`、`hands`、`init_city`、`form_city`）、`listening`（占端口、开写者）、`attending`（起写者线程）、`chain_watch`（起审计线程）与 `dropping`（拖进对话框的文件）。

**账本在哪，由 `kernel::layout::CityLayout::ledger` 一处回答。** 每个读账本的地方直接调用 `CityLayout::new(city_root).ledger()`，装配点不另设转发函数：转发只给同一件事换了名字，却会让读面与 serving 为了一个路径去依赖装配点。

**从账本重建视图是视图自己的事。** 一次性的读经 `Views::rebuild(ledger_dir)`（`accounting::views::asking`）起步：先经 `snapshot::start_audited` 同步审计整条链，再从合适的快照起步、只折尾部（§8-91）。服务中的城由 `accounting::worker::folds::fold_city` 一遍折完：`Views::over(ledger_dir)` 造出空视图，`Views` 与 `StandingFolds` 在 `runtime::replay::fold_ledger_dir` 的同一遍里各折每一条记录，这一遍返回的账本索引经 `Views::hold_index(index, ledger_dir)` 交给视图。打开账本的时刻由 `listening` 读 `SystemClock` 交进来，`fold_city` 自己不读钟。

**写者与驱动 run 的机器属于 `accounting::worker`，起线程的一半留在装配根。** 命令等在其上的命令台（`accounting::worker::desk`）、驱动 run 的 lane（`accounting::worker::pool`）、lane 写回账本的中继（`accounting::worker::relay`）、记账线程的循环（`accounting::worker::attend`）与调用时判定计划认领的 `accounting::worker::booking`（§8-42-8）都在驱动 `RunWorker`，所以与它同住；`relay` 持有 `pool::Arrival`，`desk` 持有 `relay::Wake`，它们住在一处才没有反向边。起写者线程的 `assembly::attending`、占端口并开写者的 `listen` 与它返回的 `Listening`（`assembly::listening`）留在装配根，它们只经 `accounting::worker` 的 `pub` 面碰写者。serving 留下的是：门上的钥匙与金库（`door`）、一次 serve 由调用方填好的那个值（`serve::Serving`）、进程日志的出口（`journal`）、写者旁边折叠视图的线程（`folding`）、核心线程站的档位与单调钟（`standing`，§8-93），以及还在跑的命令已写出的字节（`output_ring`，§8-115）。serving 的产品代码不写出 `crate::assembly`，也不写出 `accounting::worker`。

**serving 不往命令台投命令，所以不需要句柄。** 投命令的是 `listen` 交给 socket 的那几个闭包、控制台与 ACP 入站；前一个随 `listen` 住在装配根，后两个本来就在 serving 之外。另一种做法是把 `listen` 拆成传输的一半（留在 serving，持一个 serving 自己定义的命令发送端）与写者的一半（进装配根）；它多出一个类型和一条通道，换来的只是 `listen` 住在 serving 里，而 `listen` 的次序（§8-88）恰好是「先占端口，再开写者」这一件事，拆开后这个次序要由两边共同守。

**日志行的时间由构造者交进来。** `Journal::new` 收一个 `serving::journal::Clock`，即 `Arc<dyn accounting::Clock + Send + Sync>`；`main::city` 交的是 `assembly::SystemClock`，墙钟只在 `bin::assembly::production` 采样（§8-63，`crates/accounting/Spec.lean` §8-3）。

反方向是组装点应有的方向，保留：装配根用 serving 的 `open_vault`、`Serving`、`folding`、`output_ring::OutputRing`、`standing::monotonic_now`，用 doctor 的 `ThisMachine`、`Platform`、`PATIENCE`、`host`、`recipe_for`，用 `accounting::worker` 的 `RunWorker`、`CommandDesk`、`Hands`、`genesis::form` 与 `folds::start_served_views`。

**方向由门守。** ARCHITECTURE.md 的 `directions` 块逐个模块写下它的产品代码永不写出的路径，`cargo xtask depmap` 读它（tools/xtask/Spec.lean §8-33）。块里有三行：`accounting` 的 views 不写出 `crate::worker`；doctor 与 serving 不写出 `crate::assembly` 与 `accounting::worker`。读面与写者同住 `accounting` 之后，「写者知道读面、读面不知道写者」由第一行守；`accounting` 不依赖 `sprawling`，读面与写者指回装配根由编译器拒绝。城有没有历史由 `city::has_history` 回答（`crates/city/Spec.lean` §8-30），doctor 与写者都从那里取用。

读面用到的五样东西各归其主，worker 从那里取用：

| 事实 | 住处 | 理由 |
|---|---|---|
| 一个居民或房间叫什么 | `kernel::Address::name`（`crates/kernel/Spec.lean` §8-2 的 `Address`） | 地址的最后一段是地址自己的事实；城的名册与楼的页面都从这里读 |
| 一栋楼的页面、`documents::WINDOW_BYTES_MAX` | `accounting::views::building_page` | 页面是一个读面：按问的那一刻读盘，不持有第二份 |
| 一台 MCP server 经哪种传输到达（`McpLink`） | `agent_protocols::mcp::link` | 三种传输（`agent_protocols::mcp` 下的 `stdio`、`http`、`sse`）与把它们合成 `agent_protocols::Outbound` 的那个枚举同住 `agent_protocols`；读面与装配点都经 `agent_protocols` 的握手与列工具说话 |
| broker 的钥匙登记在哪、这座城对 broker 是谁（`broker_for`） | `accounting::toolkit_broker`（`crates/accounting/Spec.lean` §8-9） | 页面与命令读同一组事实；连接动作 `connect_toolkit` 仍是 worker 的 |
| 一个锁着的 vault 的解析器与锁中毒时的拒绝（`resolving`、`poisoned_vault`） | `accounting::held_vault`（`crates/accounting/Spec.lean` §8-9） | worker、读面与 serving 都要一次性的解析器；拒绝的措辞只有一处 |


### 8-112 保温的门：run 的模型调用经 `Warmed` 走，落地后留在 `RunWorker` 上（`accounting::worker::keeping_warm`，形状 1 数据）

```rust
// accounting::worker::keeping_warm —— shape: data
pub(crate) type Door = runtime::prefix::warmth::Warmed<fn() -> Result<TimeMs, AxError>>;
#[derive(Default)]
pub(in crate::assembly) struct Kept { /* 每个房间一扇门 */ }
impl Kept {
    pub(in crate::assembly) fn keep(&mut self, room: String, door: Door); // next_due 为 None 的门直接丢弃
    pub(in crate::assembly) fn next_due(&self) -> Option<u64>;         // 所有门里最早的一次续期
    pub(in crate::assembly) fn renew_due(&mut self, now_ms: u64) -> Vec<(Address, CacheRenewed)>;
}
impl RunWorker {
    pub(crate) fn warm_due(&self) -> Option<u64>;
    pub(crate) fn renew_warm(&mut self, now: TimeMs); // 每次续期写一条 cache_renewed；写不进账本才进诊断日志
}
```

`dispatching::agreeing` 选定适配器后，用 `city::keep_warm(city_root, building)` 读到的设置和 worker 的 `clock`（`accounting::Clock`），把它包成 `Door`；`Agreed`、`Site`、`Driving`、`Driven` 与 `driving::lane` 带的都是这扇门，所以 run 发出的每个请求都进了保温账（`crates/runtime/Spec.lean` §8-4-2），转向循环不知道保温存在。`settling::landing` 在 run 结束时把门交给 `Kept`，按房间地址存：同一房间后来的 run 换掉前一扇门，因为后一个前缀才是下一次会用到的。`next_due` 为 `None` 的门（设置为 `Off`，或已续过一次）不留，所以默认设置下 `Kept` 一直是空的，worker 不多占一个字节，也不多发一个请求。

`assembly::attending` 的空闲分支先读日程，再调用 `renew_warm(now)`，下一次睡眠取「距下次读日程」与「距 `warm_due`」中较短者，所以一次续期不会等到日程的整点之后。每次续期，不论成败，都在房间地址下写一条 `cache_renewed`（`crates/kernel/Spec.lean` §8-4）：成功时是 provider 自报的用量与账单额，失败时是它的拒绝原样，所以人从账本上读得出保温花了多少、哪个 provider 拒了续期。续期失败不停城；只有这条记录本身写不进账本时才写进诊断日志，与日程读取失败同样处理。

失败：`renew_due` 不传回 `Err`，每扇到期的门各得一条结果，模型的失败原样放进 `CacheRenewed::Refused`；一扇门失败不妨碍其他门续期，失败的门被丢弃，所以拒绝续期的 provider 不会在每次醒来时再被问一遍。

当前状态：保温已完整：设置默认关、默认下不发任何额外请求，打开后到期前续期一次，每次续期的用量或拒绝都记在账本上；lead 由门在每次调用前后量出（`crates/runtime/Spec.lean` §8-4-2）。

决定：门按房间存，而不是按 run 存。按 run 存时，一个房间连续跑十次会留下十扇门，其中九扇续的是已经被下一个前缀覆盖的缓存，花的钱没有用处；按房间存时门的数目以房间数为上限。重新考虑的条件：同一房间里并行的会话各有自己的前缀。

### 8-145 备树：lane 在 run 驾驶完之后补一棵，下一次放置接管它（`accounting::worker::dispatching::preparing`）

```rust
// storage —— `crates/storage/Spec.lean` §8-35
impl storage::Worktrees { pub fn stock(&self) -> Result<storage::FileWork, storage::StorageError>; }
// accounting::worker::dispatching::preparing —— Staged::fly 的两臂
// 两臂：驾驶返回之后先 home(flown)，交回 Flown；再 restock.put_back()（次序与池的一半见 §8-161）
// Restock::owed_by：借了树（lease 为 Some）才欠一次补树，补的是 Worktrees::open(city_root)?.stock()（界与失败见 §8-155）
```

- **为什么在这里。** 一次放置等的是实时扫描对每个新建文件的放行，512 个 16 KB 文件的树以秒计（`crates/storage/Spec.lean` §8-31、§8-35）。人等的是 run 的第一次模型调用，备树把检出挪出这一段，放置只改名。补树放在 lane 驾驶完、交回 `Flown` 之后：这时 run 的模型调用都已做完，它写的行都已经由 relay 写下，它的落地也不再等这条 lane（§8-161）。记账线程不行（§8-113：记账线程不等盘）；`land` 在记账线程上；开城时补一棵会把一次全量检出加进首字节（§8-122）；一条专门补树的后台线程不在 ARCHITECTURE §10 第 3 条点名的起线程处之列。`fly` 只有一个调用者，`accounting::worker::pool` 起的那条 lane，所以补树在 `fly` 里就是在 lane 上。
- **只在借了树的 run 之后补。** 没有评审、也不是试验的楼从不借树，它的 run 补一棵只会白占一棵树的盘。harness 的 run 与模型的 run 经同一个 `lend_tree` 借树（§8-124），也同样可能接管备树，所以两臂都补。`stock` 在备树已在干线上时只写干线改过的文件，所以房间取回自己留着的树之后补树的代价是一次差量（读数见 §8-155）。
- **代价落在谁身上。** run 的落地不等补树：lane 先把 `Flown` 交回记账线程，再补树，`DrivingPool::landed` 不在记账线程上 `join` 还在补树的 lane（§8-161）。房间队列与树的归还、待人处理事项的登记、对派活者的答复、交下去的活与继任 run 的派出照常在 `land` 里做。补树占着这条 lane 的线程直到补完；只有备树刚被接管过（一个房间第一次放置）时它是一次全量检出，其余时候是差量。紧接着派到一个新房间的 run 若在补完之前放置，见到的备树正被补（`Busy`），这次放置退回全量检出（`crates/storage/Spec.lean` §8-35）。关城时丢掉 `DrivingPool` 要 `join` 还在补的 lane，所以关城等正在补的那一棵。
- **失败。** `stock` 的失败不改变 run 的结局：run 已经结束，补不上备树只让下一次放置退回全量检出。失败写一条 `Refuse` 诊断行，锚在 lane 的账本位置快照上，与 §8-113 lane 半段的诊断行同一个写端。
- **试验借树（§8-133）的重开参数读的是这里。** D20「试验借一棵自己的树」以放置读数是否以秒计为重开条件：接管备树之后放置以毫秒计，那条决定的前提回到它被定下时的样子。

当前状态：两半都已落地。storage 一半是 `crates/storage/Spec.lean` §8-35（`stock`，放置先接管备树）；citysim 的 `large_worktree_placement` 在每轮之前调 `stock`（citysim D13、`tools/citysim/Spec.lean` §8-12）；lane 一半在 `dispatching::preparing`，界、崩溃后的收回与读数在 §8-155。检验是 `preparing::tests` 的两条：`a_lane_puts_the_stock_back_after_its_run_comes_home`（`fly` 交回 `Flown` 时还没有备树，`fly` 返回、`land` 还没跑时备树已经登记、没锁、目录在），`the_next_room_takes_the_stock_and_creates_no_file`（第一个房间经真 lane 派活并落地，第二个房间的放置整值比较 `FileWork`，三个计数都是 0，接管之后 lane 又补回一棵）。
### 8-155 补树的界：一座城一次一条 lane 在补，补到一半进程死了由盘上的状态收回（`accounting::worker::dispatching::preparing`）

```rust
// accounting::worker::dispatching::preparing（私有）
struct Restock { city_root: PathBuf, notes: Notes, staged_at: kernel::Seq }
impl Restock {
    /// 借了树的 run 才欠一次补树；没借树的 run 答 None。
    fn owed_by(self, borrowed: Option<&storage::WorktreeLease>) -> Option<Restock>;
    /// 在 run 交回 Flown 之后补；失败写一条 Refuse 诊断行，不返回错误。
    fn put_back(self);
}
// 这个进程里此刻正在补备树的城，按城根分键
static STOCKING: Mutex<BTreeSet<PathBuf>>;
```

- **一座城同一时刻至多一条 lane 在补，后来的跳过，不等。** `crates/storage/Spec.lean` §8-35 保证了两种并发（两次放置争一棵备树；一次刷新与一次接管），没有保证两次 `stock` 同时见到「没有备树」：两条都去新备，后一条的 `worktree add` 撞上前一条刚登记的 `+spare` 而失败，它的 `take_back` 收回的是前一条的登记；这时一次正在接管的放置在改名登记目录那一步失败，那次派活带着存储错误回到人那里。lane 在调 `stock` 之前先在表里占这座城的位置，占不到就不补：正在补的那一条补完，城里就有备树。不等，是因为正在补的那一条补完城里就有备树，等它只让这条 lane 的线程多活一次全量检出，之后的那次 `stock` 也只剩一次空的差量。表被一条 lane 崩掉而中毒时照样读写，因为表里每一步都在锁内做完。
- **表在进程里，按城根分键。** 一座城只有一个写者进程（账本的写锁），所以「这个进程里谁在补这座城」就是「谁在补这座城」。
- **成功与失败。** 成功写一条 `Trace` 诊断行，带 `stock` 答回的三个计数，人从日志读得出这次补树是全量检出还是差量。失败写一条 `Refuse` 诊断行，说下一次放置会退回全量检出；不返回错误，run 的结局不因它改变（§8-145）。两行都锚在派活被决定时的账本位置（`Laying` 与 `HarnessHalf` 里的 `staged_at`），与 §8-113 lane 半段的诊断行同一个写端。
- **补到一半进程死了。** 账本里没有补树的记录，重放与恢复都不读它；备树的全部状态在盘上与城的 git 登记里，进程里的表随进程消失。下一个写者开城时 `RunWorker::over` 先拿账本写锁，再由 `Worktrees::lift_abandoned_leases` 解开每一棵树的锁，`+spare` 也在其中，否则一棵锁着的备树会被读成「正在补」，本次服务里再也没有备树可接管；`sweep_abandoned` 不清它，因为 `live` 不列它（`crates/storage/Spec.lean` §8-35）。之后按盘上的样子收回：登记与两份链接文件都认得出时，下一次放置接管它，检出到一半的文件由接管第⑥步的 `restore` 补全；登记在而链接文件认不出时，下一次 `stock` 收回登记重备；目录在而没有登记时，下一次 `stock` 先清掉目录再备。所以 G5（崩溃与恢复）不需要为补树加任何恢复步骤；它要守住的只有一条次序：开城解锁先于第一次放置。
- **补树不在落地这一段。** 一个房间第一次放置接管了备树之后，这条 lane 补回的是一次全量检出；lane 在交回 `Flown` 之后才补，`DrivingPool::landed` 不 `join` 还在补的 lane，所以这个 run 的落地不等它（§8-161）。
- **读数。** 由 `preparing::tests::instrument_production_placement`（`#[ignore]`）给出：一座评审楼的 `lab/room1/bulk` 里 512 个 16 KB 文件，第一个房间经 `fly` 放置（全量检出）、驾驶、补备树；第二个房间经 `fly` 接管备树、驾驶、补回一棵。仪表用一个包住城账本的计时 `Ledger` 与交给 `fly` 的 `home` 量四段：`fly` 开始到 `worktree_opened` 入账（放置），最后一行入账到交回 `Flown`（落地等的，`landing_wait_ms`），交回 `Flown` 到 `fly` 返回（补树），以及整个 `fly`。下面两段读数在 lane 先补树、后交回 `Flown` 时量得，那时补树这一段整段落在落地之前（§8-161 前后对照）。

读数（debug 构建，windows-x86_64、16 核，同一仪表三轮）：第二个房间接管备树的放置 39–42 ms，`created` 为 0；它的 lane 之后补回一棵全量检出的备树 1.44–1.46 s。第一个房间的放置 5.6–6.2 s，`created` 513：这一段里有城的第一次提交（`ensure_base` 把 512 个文件 stage 并扫描一遍）和一次全量检出；它之后补备树 1.26–1.33 s。

读数（发行构建，windows-x86_64、16 核，同一仪表两轮）：第二个房间接管备树的放置 47–48 ms，`created` 为 0；它的 lane 补回备树 1.35–1.54 s。第一个房间的放置 4.29–4.72 s，`created` 513；它之后补备树 1.24–1.42 s。发行构建省下的是计算，省不下实时扫描对每个新建文件的放行，所以补树与城的第一次放置在发行构建里仍以秒计。`just bench` 的 `large_worktree_placement`（bench 自己备树）p50 36.0 ms。

当前状态：上面各条都已落地。还以秒计的有两段，都不在 run 的落地之前：一是补备树本身，在 lane 交回 `Flown` 之后，占着这条 lane 的线程与盘（§8-161）；二是一座城的第一次放置，它先提交一次城（`crates/storage/Spec.lean` §8-8 的 `ensure_base`），每座城只有一次，那次之前没有干线可备，所以备树帮不上它，这一段仍在那座城第一个 run 的第一次模型调用之前（拆分与读数在 §8-161）。发行构建的读数由同一个仪表测试给出：`cargo nextest run --release -p sprawling-accounting --run-ignored only --no-capture -E 'test(instrument_production_placement)'`。

### 8-161 run 先落地，lane 再补备树（`accounting::worker::dispatching::preparing`、`accounting::worker::pool`）

```rust
// accounting::worker::dispatching::preparing
impl Staged {
    /// 准备、驾驶，把 Flown 交给 home，然后才补备树（§8-145、§8-155）。
    pub(crate) fn fly<L: Ledger>(self, ledger: &mut L, context: DriveContext, home: impl FnOnce(Flown));
}
// accounting::worker::pool
pub(crate) struct DrivingPool {
    running: BTreeMap<RunId, JoinHandle<()>>,   // 还在驾驶的 run
    trailing: Vec<(RunId, JoinHandle<()>)>,      // run 已回家、lane 还没结束（在补备树）
    /* 其余字段不变 */
}
impl DrivingPool {
    /// 把 run 回家的 lane 挪进 trailing，join trailing 里已结束的 lane；不等还在跑的。
    pub fn landed(&mut self, arrival: Arrival) -> Result<Arrival, AxError>;
}
impl Drop for DrivingPool { /* join 每条 trailing 里的 lane */ }
```

- **次序在一个函数里。** lane 驾驶完，先把 `Flown` 交给 `home`，再补备树。池起的 lane 交给 `home` 的是「送上记账线程的队列」，测试交的是「放进一个局部变量」。次序就是这一节要的性质，所以它写在 `fly` 一处，而不是让 `fly` 返回一个待补的值、由每个调用者自己记得先送回再补。
- **落地不 `join` 还在跑的 lane。** `landed` 把回家的 run 的 lane 从 `running` 挪进 `trailing`，再 `join` `trailing` 里已经结束的 lane（`JoinHandle::is_finished`）；一条异常结束的 lane 仍是一次 `E_STORAGE_FATAL`，与改之前一样（§8-46-3）。`in_flight` 只数 `running`，所以排空判定（`kernel::pursuit` 的 `in_flight`、`land_the_rest`）仍只等还在驾驶的 run。`trailing` 不算进 `in_flight`：一座城一次只有一条 lane 真在补（§8-155），别的 lane 见位置被占就跳过、随即结束。
- **丢掉池时 `join` 还在补的 lane。** 同一个进程里再开这座城时，`RunWorker::over` 先由 `lift_abandoned_leases` 解开每棵树的锁，`+spare` 也在其中（§8-155）；一棵检出到一半的备树解了锁就读作「备好了」，下一次放置会接管它。`join` 让关城等正在补的那一棵，这与改之前关城的 `land_the_rest` 等 lane 回家时付的是同一笔。进程在补到一半时死掉照 §8-155 的崩溃规则收回，账本上不记，补树不加事件。
- **被否：**`fly` 返回一个待补的值，由池的 lane 先送回再补——次序就分到了 `fly` 的每个调用者，测试里的调用者不送回也不补；为补树另起一条线程——ARCHITECTURE §10 第 3 条点名的起线程处里没有它，lane 已经在那里；把 `trailing` 计入车道上限——一次补树会挡住一个新 run 的开始，那正是这一节要挪走的等待；关城不 `join`——同进程再开城会接管一棵半成品备树，理由见上。
- **检验。** `preparing::tests::a_run_lands_while_its_lane_still_puts_the_stock_back`：测试线程握住 `STOCKING` 表，lane 的补树在表上等；run 仍然经真 lane 落地（`driving()` 为假），此时城里没有备树；放开表、丢掉 worker（池 `join` 那条 lane）之后备树就位。断言的是次序，不读墙钟；`Patience::For` 的轮询上限只让红的时候不挂住。`a_lane_puts_the_stock_back_after_its_run_comes_home` 在 `fly` 一处断言同一个次序：交回 `Flown` 时没有备树，`fly` 返回时有。
- **城的第一次放置拆开来。** 发行构建、同一种 512 个 16 KB 文件的楼：`ensure_base` 2.77–2.86 s，其中 `Index::add_all` 2.2 s，它为 513 个文件各写一个松散对象，每个都是一次新建文件，等实时扫描放行一次；之后 `claim` 全量检出 513 个文件 1.16–1.18 s。两段都不在 lane 里：松散对象在 `storage::checkpoint`，换成一次写一个 pack 只新建一个文件；检出是 libgit2 的串行检出，并行写文件要在 storage 里多一个起线程处（ARCHITECTURE §10 第 3 条）。

当前状态：次序、池的一半与关城的 `join` 已落地。城的第一次放置没有改，仍以秒计（上一条）；降它的下一步是 `ensure_base` 写一个 pack 而不是逐个松散对象，判定它的证据是同一个仪表测试的 `placement_ms`。ARCHITECTURE.md §10 第 3 条与 §8-46-3 仍写着 lane 在 run 回家时结束，现在 lane 在补完备树后结束。

读数（发行构建，windows-x86_64、16 核，`instrument_production_placement`）：改之前，第二个房间接管备树的放置 41 ms，`created` 为 0，之后它的落地等补树 1222 ms；第一个房间放置 4239 ms，`created` 513，落地等补树 1219 ms。改之后两轮：两个房间的落地都只等 0 ms（`landing_wait_ms` 不到 1 ms）；第二个房间放置 35–40 ms，`created` 为 0，之后 lane 补树 1178–1256 ms，在落地之外；第一个房间放置 4.00–4.32 s，`created` 513，之后补树 1149–1233 ms。`large_worktree_placement`（bench 自己备树，不经 lane）改之前 p50 34.7 ms，改之后 39.2 ms，这一段代码没有改，差别是两次运行之间的起伏。

### 8-113 派活的准备进 lane：记账线程只做决定，树、MCP 连接与冻结在 lane 里（`accounting::worker::dispatching::running`、`accounting::worker::dispatching::preparing`、`accounting::worker::driving::flight`）

**为什么切。** 一次派活的准备里等盘或等别的进程的有三样：评审树的放置（一次提交加一次检出，时长随树的大小）、MCP 缺表的那次启动与握手、冻结时的 CAS 写。它们在记账线程上时，每条 lane 的 append 都排在它们后面，因为 lane 写历史的那道口子只有记账线程在服务（§8-46-2）。所以准备切成两段，切在第一个等待之前。

- **记账线程：`stage_dispatch(&mut self, at, task, goal) -> Result<(Staged, Continuation), AxError>`。** 同意、托管、规则、房间、定形、brief、job，配置与身份的读取，run id（连同它那一次时钟采样），`governance.sent`，滤表，`Site::name_tree`（评审楼的分支就是树的名字，`tree_of(addr)` 的纯函数，于是 `open_desks` 造 PR 桌时已知分支，树还在 lane 里放），`open_desks`，`inherited`，以及 lane 要读的状态快照：正在处理的命令的幂等键、`collaborating.rooms` 里挂着信的房间集合、`ledger.position`、治理的自治级别、`city_hash`；最后 `enrol_run`。这些要么写账本，要么改记账线程持有的折叠，要么是一次时钟采样。留在这里，run 与 run 之间的决定次序只由记账线程定。`inherited` 留下，是因为它读账本索引、写 lineage 一行，账本与 `LedgerIndex` 只属于记账线程。`Staged` 拥有它的全部字段，是 `Send`。
- **lane：`Staged::fly<L: Ledger>(self, ledger: &mut L, context: DriveContext) -> Flown`**（`dispatching::preparing`），先准备、再 `drive_run`，这是 lane 的全部工作；准备这一段依次做：`Site::place_tree`，`Stamping` 包着这条 lane 的 relay，于是 `worktree_opened` 经 relay 写；`lay_out_workbench`，MCP 缺表时的连接在这里；`Freezing::freeze_plan`，CAS 用 lane 共用的句柄；`probe_after`、`Sieving::for_run`、`checkpoint_scope`。`ensure_base` 与检查点之间的次序由 §8-46-13 定。`claim` 只动本房间的树与分支。`Site` 与 `Assignment` 无论准备成败都装在 `Flown` 里、随 `Arrival` 回家，因为 `land` 要还 `Site` 借到的树；`Workbench` 经 `Driving` 进驾驶、随 `Driven` 回家，`land` 读它的 `delegates` 与 `succession`。准备失败时 `Flown.driven` 就是那个错误，`land` 照常先还房间队列与树，再把错误交给问的人，和驾驶失败走同一条路（§8-46-9）。
- **`DrivingPool` 排队与起 lane 的单位是 `Staged`**：满了的时候等着的是一份还没准备的派活，它不占线程，也还没有占树、没有连 server。`Staged` 带着 `site.run_id`，池按它认重复。

**lane 借什么。** §8-110、§8-111 拆出的对象里，lane 只借本来就是共享句柄的那几样，每样是一个 `Arc` 的克隆或一个值，收在 `Staged` 的 `Laying` 里：`Flight.backlog`（`exec` 与 `status` 用）、`Credentials.vault`（工具解析密钥用）、`city_root`、lane 共用的 CAS 句柄、MCP 常驻表、诊断的写端、时钟。房间的六张桌子（信、目标、计划、书架、PR、作坊）本来就是 `Arc<Mutex<_>>`，`Desks::for_bench` 把它们的克隆与 `waiting` 作为 `BenchDesks` 交给 lane，`Desks` 本身（连同队列的 `tenure`）留在 `Continuation` 里，只有它还队列。常驻表：`accounting::Connectors::connect` 取 `&self`（`crates/accounting/Spec.lean` §8-2），`RunWorker.connectors` 是 `Arc<dyn Connectors + Send + Sync>`，lane 借一个克隆；表的寿命仍是 worker 的寿命，最后一个 `Arc` 落地时 `Drop` 杀子进程（§8-4）。CAS 是 lane 共用的一个句柄 `RunWorker::lane_store`（`Arc<Mutex<Cas>>`），随 worker 打开，冻结与筛子的 offload 都经它写，每次在锁里做完：`storage::Cas::open` 会清扫 `tmp/` 里的半成品，而一次写的半成品名只由对象哈希决定，一条 lane 自己开句柄就会扫掉另一条 lane 正在写的对象，两条 lane 写同一个对象也会共用同一个半成品文件。记账线程仍用自己的 `RunWorker::cas`，它写 brief 与 job，不写冻结的四段。检查点仓库由谁建、lane 怎样打开它，由 §8-46-13 定。`Collaborating`、`Planning`、`Doorstep`、`Governance`、账本与 `LedgerIndex` 不借：记账线程的折叠改写它们，lane 读到的只能是 `Laying` 里的快照——`collaborating.rooms` 里挂着信的房间集合、这个居民持有的目标、治理的自治级别、`ledger.position`、`city_hash`。快照在 `stage_dispatch` 里取，lane 看到的是这个 run 被决定那一刻的城。`origins.started`（房间的会话从这个 run 起算）也在 `stage_dispatch` 里、紧跟在读 `carried_from` 之后：它改的是记账线程的折叠；准备在 lane 里失败的 run 与驾驶失败的 run 一样，都已开始了房间的会话。

**lane 半段的诊断行。** `mcp_tools` 为每台 server 写一行（连上、已连着、拒绝）并写它自己的计时，`admit_reading_room` 为阅览室点名却不在架上的名字写一行。诊断的写端是可共享的句柄 `recording::Notes`（一个 `Arc<Mutex<Diagnostics>>`），`RunWorker.log` 就是它，lane 借一份克隆，行锚在 `Laying` 里的账本位置快照上。锚取快照是因为诊断只写不读：`runtime::diagnostics` 没有读回一行的方法，决定与恢复的代码因此不可能依赖锚；读者是人，人用 `seq` 把一行对到历史上的位置，准备阶段的行锚在准备开始的位置，正是它说的事发生的地方。锁中毒时照样取出写端：每次写在锁内一步做完，中毒只说明别的线程在锁外崩了。被否：`Prepared` 把行带回家、由 `land` 写——锚落在整个 run 之后，一台拒绝握手的 server 要等 run 落地才被人看见，而 `Refuse` 这一档写给正在看的人。

**冻结前缀为什么逐字节不变。** 前缀由三段文件、一个 run slot 与工具表拼成，每个输入在记账线程上与在 lane 里是同一个值。三段读的是城根下的文件，从 `stage_dispatch` 到 lane 冻结之间记账线程不写它们：它写的是账本、brief 与 job。run slot 由 `Given`、模型注记与继承的对话拼成，三者都在 `Staged` 里。树的路径是 `city_root` 与 `tree_of(addr)` 的纯函数，与哪条线程去认领无关。工具表的顺序由 `lay_out_workbench` 的准入顺序定；其中 MCP 那部分，命中时是表项记下的 `listed`，缺表时是这一次 list 的结果，与握手在哪条线程上无关。run id 仍在记账线程上铸，从它铸 id 的三件工具拿到的还是同一个 id。唯一换了线程的时钟读数是计时用的 `[prepare_dispatch]`，它不进任何记录；铸 run id 的那一次采样不搬（`stand_up` 的文档：结构改动不得移动时钟采样）。

**行的次序。** 准备阶段在 lane 里写的行有两种：评审楼的 `worktree_opened`，与继任 run 的 `eval_run`（`probe_after` 在继任者起步前问一次模型，写下与前任答案的比较）。它们和驾驶写的行走同一个 relay，所以一个 run 自己的行仍是准备的行在前、驾驶的行在后，两个 run 的行怎样交错由 relay 决定，重放的确定性由 §8-46-5 保证。relay 服务写下的每一行——lane 的 append 与调用时入账的认领行——都由 `serve` 交回，`serve_flight` 逐行交给 `RunWorker::absorb`，与记账线程自己写的行走同一条路（§8-110）：活的折叠与重启折叠读同一份历史，一条被这四份折叠（会话起点、治理、计划、凭据）读的记录搬进 lane 不用另接线。折叠拒绝一行时那一行已在账上、回信已发出，拒绝记进诊断，不改变车道收到的答复。`record_for` 还给记账线程写的每一行盖上正在处理的命令的幂等键，relay 的行不经它。重启后认出一条重复的命令，靠的是 `Entrance::absorb` 在历史里见到带这个键的某一行；一次评审楼的派活在规则没变时（`book_rules` 只在规则变了才写 `rules_changed`），带键的行只有 `worktree_opened` 这一行。所以 `Staged` 带上命令的键（`Entrance::carrying`，没有命令时为空），`Stamping::record_for` 用同一条盖键的规则给 `worktree_opened` 与 `eval_run`（`Site::probe_after` 也经 `Stamping` 写）盖上，这两行由 lane 写与由记账线程写逐字节相同。盖键的规则是键与载荷的纯函数 `commanding::entrance::stamped`，`RunWorker::record_for` 与 `Stamping::record_for` 都调它。`a_review_dispatch_sent_again_after_a_restart_is_answered_once` 经 `serve_one`（认键的那道门；`handle` 不经它）守着这一点。

**验收的两条测试。** 其一是 `accounting::worker::mcp::tests::a_dispatch_whose_server_still_shakes_hands_leaves_the_desk_free`：楼的配置里一台握手要等测试放行的 stdio MCP server，`serve_one` 派一轮活；握手放行之前 `serve_one` 已经返回，于是记账线程在这次派活里不等握手。测试自己在十秒后放行，所以一个等握手的记账线程让断言失败而不是挂住。树的一半是 `accounting::worker::reviewing::tests::kept::a_tree_that_waits_on_the_index_lock_is_placed_in_the_lane`：一座还没有提交的城、一个评审房间，先放一把 `.git/index.lock`；`serve_one` 之后这个 run 已在一条 lane 里，第一次放置的提交在 lane 里等锁并失败，失败随 `Flown` 回家、由 `land` 交还。其二是守护（`a_review_dispatch_freezes_the_prefix_it_froze_on_the_accounting_thread`）：一座新城、一个评审房间、同一条 job，这个 run 在 `prompt_assembled` 里记下的四段哈希等于测试里钉住的那一组，它们取自记账线程上准备一切时的路径。不需要钉时钟：铸 run id 的那次时钟采样不进前缀，两座新城派同一条 job 冻结出同样的四段。它的作用是让准备换线程改不了前缀；有意改动新城前缀文字的改动，从它的失败里取新的哈希。
-/

/-! D21 补备树放在 lane 里，在 run 交回 `Flown` 之后；`DrivingPool` 落地时不 `join` 还在补的 lane（§8-145、§8-161）

放置的秒数是一次全量检出等实时扫描逐个放行新建文件（`crates/storage/Spec.lean` §8-35）；把检出挪进备树，放置只剩改名，而补备树本身仍是一次全量检出，它必须落在人等的两段之外：run 的第一次模型调用之前，和 run 的落地之前，因为落地写审批、答复派活者、往下派活。lane 驾驶完、交回 `Flown` 之后，两段都已过去。**被否**：在 `land` 里补——`land` 在记账线程上，记账线程等一次检出，每条 lane 的 append 都排在它后面（§8-113）；开城时补——一次全量检出加进首字节（§8-122）；为补树另起一条后台线程——库 crate 与 `sprawling` 起线程的地方都是 ARCHITECTURE §10 第 3 条点名的，补树不值一条新线程，lane 已经在那里；lane 在交回 `Flown` 之前补——一个新房间的 run 落地晚一次全量检出，发行构建 1.2–1.5 s（§8-161 的读数）。**重开参数**：同一座城的派活密到下一次放置常赶在补完之前（放置见备树 `Busy` 而全量检出），那时补树的时机要往前挪，或者一座城备不止一棵。
-/

/-! D22 一座城一次只有一条 lane 补备树，后来的跳过，表放在进程里按城根分键（§8-155）

两次 `stock` 同时见到「没有备树」时，后一次的失败会收回前一次的登记，正在接管的一次放置因此失败，那次派活带着存储错误回到人那里；`crates/storage/Spec.lean` §8-35 只保证了放置与放置、刷新与接管这两种并发。跳过而不等，因为等另一条 lane 的全量检出就是让自己的落地再晚一次全量检出，而正在补的那一条补完城里就有备树。表在进程里，因为一座城只有一个写者进程，「这个进程里谁在补这座城」就是「谁在补这座城」；与 `city::document` 按路径分键的写锁表是同一种做法。**被否**：在 storage 的 `stock` 里补上这个窗口——storage 的公开契约本轮不改（`crates/storage/Spec.lean` §8-35 已定）；`Flight` 持一把补树的锁、经 `DriveContext` 交给 lane——与进程表判的是同一件事，却要多一条从 `RunWorker` 穿过 `drive_context` 到两条 `fly` 臂的传递路径；跳过换成等——理由见上。**重开参数**：一个进程同时为同一座城开两个 worker，或 storage 的 `stock` 自己在「没有备树」时先占住 `+spare`。
-/
