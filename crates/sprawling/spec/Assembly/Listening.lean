-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 开城的次序：先占端口，再写第一行，最后才说 running

规定 `crates/sprawling/src/assembly/listening.rs` 的 `listen` 与 `main::city` 的横幅之间的次序（`bin::assembly::listening`、`main::city`；§8-88）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

`listen` 依次做四步，每一步都可能拒绝：判定绑定面并 bind；建 CAS 目录、开账本读起视图；开写者线程，`JsonlLedger::open` 先取写者锁、再做断尾恢复、再 `open_for_service`；返回 `Listening`。账本的第一次写在取到写者锁之后。横幅只在拿到 `Listening` 之后印，这是类型排的：`serve_city` 没有 `Listening` 就走不到印横幅的那一段。

每一步的成败是参数；模型记下账本写了几行、横幅印没印。三条性质：

* **锁之前的拒绝一行不写**——端口被占、暴露的地址没有令牌、账本读不起来、写者锁被别的进程持着，账本都与之前相同；
* **横幅只在 `listen` 成功之后印**；
* **两个进程不会同时写一座城**——第二个进程要么在 bind 被拒，要么在取锁被拒，两者都在写任何东西之前。
-/

namespace Sprawling.Assembly.Listening

/-- `listen` 在哪一步拒绝。 -/
inductive Refusal where
  /-- 端口被占，或暴露的地址没有令牌。 -/
  | bind
  /-- CAS 目录建不成，或账本、快照读不起来。 -/
  | store
  /-- 写者锁被别的进程持着：`E_LEDGER_HELD`。 -/
  | held
  /-- 取到锁之后，断尾恢复或开城那几行写不进去。 -/
  | service
  deriving Repr, DecidableEq

/-- 各步成不成：`false` 是在这一步拒绝。 -/
structure Steps where
  bind : Bool
  store : Bool
  lock : Bool
  service : Bool
  deriving Repr, DecidableEq

/-- 一次开城的可见结果：拒绝或成功，账本多了几行，横幅印没印。`opening` 是取锁之后开城写下的行数（断尾恢复的一行与 `open_for_service` 的那几行）。 -/
structure Seen where
  refused : Option Refusal
  written : Nat
  banner : Bool
  deriving Repr, DecidableEq

/-- `listen`，再加上 `serve_city` 拿到 `Listening` 之后印横幅。取锁之后写不进去的那一步写了几行，Rust 不保证为零（行可能已经落下），模型保守地记作 `opening`。 -/
def openCity (s : Steps) (opening : Nat) : Seen :=
  if !s.bind then ⟨some .bind, 0, false⟩
  else if !s.store then ⟨some .store, 0, false⟩
  else if !s.lock then ⟨some .held, 0, false⟩
  else if !s.service then ⟨some .service, opening, false⟩
  else ⟨none, opening, true⟩

/-- 锁之前的拒绝一行不写。 -/
theorem refused_before_the_lock_writes_nothing (s : Steps) (opening : Nat) (r : Refusal)
    (h : (openCity s opening).refused = some r) (before : r ≠ .service) :
    (openCity s opening).written = 0 := by
  unfold openCity at h ⊢
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all

/-- 横幅只在 `listen` 成功之后印。 -/
theorem banner_only_after_listening (s : Steps) (opening : Nat)
    (h : (openCity s opening).banner = true) :
    (openCity s opening).refused = none ∧ s.bind ∧ s.store ∧ s.lock ∧ s.service := by
  unfold openCity at h ⊢
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all
  split <;> simp_all

/-- 第二个进程：端口或写者锁被第一个占着，它在写任何东西之前被拒，也不印横幅。 -/
theorem a_second_process_writes_nothing (s : Steps) (opening : Nat)
    (taken : s.bind = false ∨ s.lock = false) :
    (openCity s opening).written = 0 ∧ (openCity s opening).banner = false := by
  obtain ⟨b, st, l, sv⟩ := s
  rcases taken with h | h <;> cases b <;> cases st <;> cases l <;> cases sv <;> simp_all [openCity]

end Sprawling.Assembly.Listening

/-!
## 8-88 开城的次序：先占端口，再写第一行，最后才说 running（`bin::assembly::listening`、`main::city`）

这个次序必须守住的性质的权威是本文件上面的模型：锁之前的拒绝一行不写，横幅只在 `listen` 成功之后印，第二个进程在写任何东西之前被拒；本节是接口、次序的理由与被否的方案。

**原因**：一座城曾经可以同时有两个写者。第二个进程对同一座城执行 `serve` 时，写者线程在 bind 之前就已经打开；bind 失败后它照常收口，往账本里写了一行「人主动关城」的 handoff。那是没有人做过的事，而且是一次分叉：两个进程各自用同一个 `prev` 写了同一个 seq。`sprawling is running.` 这行横幅则在这一切之前就印了出来。

**次序**，`assembly::listen` 是它唯一的定义：
1. 判定绑定面，然后 bind（`wire::bind`，`crates/wire/Spec.lean` §8-46）。端口被占、或者暴露的地址没有令牌，都在这里拒绝，账本一字未动。
2. 建 CAS 目录，开账本、从两份快照一遍读起视图与 Standing（§8-122）。
3. 开写者线程：`RunWorker::new` → `JsonlLedger::open` 先取写者锁（`crates/storage/Spec.lean` §8-1），再做断尾恢复，再 `open_for_service`。锁被别的进程持着，就是 `E_LEDGER_HELD`；已经绑定的监听器随之放掉，账本一字未动。
4. 返回 `Listening`。到这一步，一次 serve 能做的拒绝都已经做完。

然后调用方印横幅、打开浏览器，再调用 `Listening::serve` 开始应答。

```rust
pub async fn listen(serving: Serving) -> Result<Listening, AxError>;
#[must_use] pub struct Listening { /* Bound、ServeConfig、命令台、应答函数、写者线程、控制台 —— 私有 */ }
impl Listening {
    /// 应答，直到人停城；返回前 join 写者线程。
    pub async fn serve(self) -> Result<(), AxError>;
}
```

- **横幅排在后面，是类型排的**：`serve_city` 只有拿到 `Listening` 才走得到印横幅的那一段（`main::city::print_banner`），`listen` 失败就报错退出。所以「running」只在监听已经开始、写者已经持锁之后出现。日志级别那一行和「这个终端就是控制台」两行跟在横幅后面；控制台本身在 `Listening::serve` 里才启动，它的输出因此也排在横幅之后。
- **bind 在取锁之前**：锁在 `JsonlLedger::open` 里取，而 `open` 可能做断尾恢复、写一行 `log_truncated`。要把取锁挪到 bind 之前，就得把取锁从打开里拆出来，给城的写者第二个入口。bind 在前的代价是：同一座城、同一个端口上的第二个 `serve`／`up` 报的是端口被占（`E_CONFIG_INVALID`，recovery 叫人停掉占着端口的进程），而不是 `E_LEDGER_HELD`。两种拒绝都发生在写任何东西之前。
- **`resume`、`fork`、`adopt` 不 bind**，它们直接经 `RunWorker::new` 打开账本，所以城在服务时，它们得到 `E_LEDGER_HELD`。
- **`form_city` 把自己开的写者交给 `RunWorker::over`**，而不是写完头两行之后再用 `RunWorker::new` 开第二个。同一个进程里的两个 `JsonlLedger` 各有一份 seq 与 prev；有了写者锁，这种情形在打开时就被拒。
- **被否：保留单个 `serve(Serving)`，把横幅做成 `Serving` 里的一个回调**。回调在库的异步任务里执行，打印什么、何时打印就由库决定。两步的形状让调用方自己决定监听开始后做什么，库只保证那个时刻已经到了。
- **`Listening` 不 serve 就丢掉，写者线程会一直等到进程结束**，也写不出 handoff；`#[must_use]` 让这种写法在编译时就有警告。
- **重开参数**：如果出现一个在 `listen` 返回之后仍可能失败、失败时又需要收回横幅的步骤，就重新考虑这个切分。

**本章测试**：`assembly::listening::tests::a_serve_refused_at_the_socket_writes_no_line`：测试自己先占住端口，`listen` 返回错误，账本的每一行与之前逐字节相同。锁的那一半由 storage 的 `a_second_writer_of_a_city_is_refused_until_the_first_lets_go` 守住。

### 8-99 视图在自己的线程上折叠并发布快照，写线程与折叠都不等读者（`bin::serving::folding`、`accounting::views::answering`）

```rust
// bin::serving::folding —— shape: adapter
pub(super) struct Folding {
    pub(super) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(super) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(super) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,
    pub(super) thread: std::thread::JoinHandle<()>,
}
pub(crate) struct Copies { pub(crate) published: Arc<Published>, pub(crate) spare: Views }
pub(crate) struct Broadcast { pub(crate) to_clients: broadcast::Sender<wire::Committed>, pub(crate) head: Arc<wire::LedgerHead> }
pub(crate) fn spawn_folding(
    copies: Copies,
    broadcast: Broadcast,
    setting: CorePriority, // 视图线程是否升档（§8-93）
    clock: fn() -> Instant, // 服务中切快照的节奏用的钟，由 bin::assembly 交进来（§8-91）
) -> Result<Folding, AxError>; // StorageFatal「start the view fold」：线程起不来
// accounting::views::answering
pub(crate) struct Published { /* 私有：Mutex<Arc<Views>> */ }
impl Published {
    pub(crate) fn new(views: Views) -> Published;
    pub(crate) fn snapshot(&self) -> Arc<Views>;                    // 读者拿的快照
    pub(crate) fn replace(&self, latest: Arc<Views>) -> Arc<Views>; // 发布新的一份，交回被换下的那份
}
// accounting::worker::folds
impl Views { pub(crate) fn twin(&self) -> Result<Views, AxError>; } // 克隆出第二份，共用账本索引与计划缓存（§8-144）
```

**写线程只做一次 `send`。** 观察者（`observer`）、机器体检的落点（`machine`）与借出金库的 `lend` 都只把一份 `Fold`（一条已提交的记录、一份 `DoctorAnswer`，或工作线程打开的金库）放进无界 `mpsc` 通道，然后返回。名为 `sprawling-views` 的线程每次把通道里已到的全部取出为一批，按到达顺序折叠。被拒：观察者在写线程上直接锁视图——读者持锁多久，写线程的下一次落盘就晚多久。

**两份视图轮换，读者拿快照。** 折叠线程持有备用的一份 `Views`，`Published` 持有发布出去的那份 `Arc<Views>`。每一批：折进备用份，用 `replace` 把它发布，广播这批记录，再收回换下来的那份（`Arc::try_unwrap`；读者还拿着就让出时间片再试），把同一批折进去，它成为下一批的备用份。读者用 `snapshot` 拷一只 `Arc`，锁只罩住这一次指针拷贝；`answer_outside_the_lock` 在快照上 `prepare`，放下快照再 `finish`。于是读者之间不互等，读者不等折叠，折叠在广播之前不等任何读者，收回时只等在换下之前拿到快照、还在做纯内存 `prepare` 的读者。`prepare` 因此只做纯内存的事：连时间也要花在读盘、git 或网络上的查询在快照里只拷走它要的小数据，`finish` 在快照放下后才去读；`McpHealth` 的握手（每台服务器最多等 `HANDSHAKE_PATIENCE`）与 `Toolkits` 的中介书架都是这样，拷走的是 `LiveAsk`（城根、城地址、保管人的 `Arc`）。在快照里握手会让收回一直自旋到握手超时，其间没有一批能折叠或广播。第二份由 `Views::twin` 在 `start_served_views` 折完之后克隆出来（§8-144），不再读一遍历史；两份共用同一只账本索引与计划缓存的 `Arc`，所以多出的内存只是折叠本身的一份。计划缓存在快照里按它自己编码，锁不进快照。被拒：每批克隆一份整的视图发布——每条记录一次整份拷贝，40 万行城上一份 22 ms；`RwLock<Views>`——折叠的写锁要等每个在读的读者，新读者又排在写锁后面。代价是折叠常驻两份，每条记录折两次。

**积压的读数是一件仪表。** `serving::folding::tests::instruments` 的 `instrument_view_backlog`（`#[ignore]`，`just bench` 按名字跑它，与 §8-84 的两件同一个过滤器）起一个真实的 `spawn_folding`：一条线程经 `observer` 连续送入 2,000 条记录，一条读者线程在其间反复拿快照做一次 `answer_outside_the_lock`，测试线程从 `to_clients` 收广播。读数一行：每条记录从 `send` 到广播收到的时延（`samples`、`floor_us`、`p50_us`、`p99_us`、`max_us`），突发中已送出未广播的最大条数（`max_backlog`），整次突发用时（`burst_ms`），行尾带 `machine=<os>-<arch>, <n> core(s)`，与 §8-84 的仪表同一形。它量的是视图线程在突发下落后多少，不加断言：墙钟因机器而异。**被否：在视图线程里写诊断或加原子计数。** 前者在服务中的城上每批一行、淹没别的日志；后者要一个线上字段才有读者，而线格式归一批一进的纪元。

**广播排在发布之后。** 客户端收到一条记录再去查询，拿到的快照已经含有这条记录。

**没有读者能毒化的视图锁。** 读者只读不可变的快照，恐慌不会撕坏它。`Published` 的锁里只有一次 `Arc` 拷贝或交换，没有会恐慌的操作，换下来的那份在锁外交回；即便锁中毒，锁里仍是一只完整的 `Arc`，所以照取不误（`PoisonError::into_inner`）。视图拒绝折叠的记录照旧写一条诊断到标准错误后跳过；折叠线程自己恐慌则线程结束，观察者此后每次 `send` 失败都说出这条记录到不了视图，恢复办法是重启服务，视图从 Ledger 重建。

**金库经通道借给两份视图。** 金库在工作线程里打开，`spawn_worker` 收到它之后调用 `lend`，两份视图各拿到同一只句柄；在这之前的快照里没有金库，要用凭证的读法按「借不到」作答。

**线程随写线程结束。** `attend` 返回后写线程丢掉 `RunWorker`（连同观察者与 `machine`），通道随之关闭，折叠线程把通道里剩下的折完、发布、广播完再退出；写线程 join 它之后才结束，所以 `serve` 对写线程的 join 也等到了最后一次广播。

### 8-100 做 I/O 的查询只在锁内取小数据，I/O 在锁外做（`accounting::views::answering`、`accounting::views::prepared`）

```rust
// accounting::views::prepared —— shape: projection
pub(crate) enum Prepared {
    Held(wire::Answer),          // 视图自己答完了
    GitStatus(GitStatusAsk),         // 锁内取了楼的地址与最近一次检查点，锁外读工作树
    Preferences,                     // 锁外读这个人的设置文件
    Config { city_root: PathBuf, addr: Address }, // 锁外读配置阶梯
    Listing { city_root: PathBuf, at: Option<Address> }, // 锁外列一层目录
    Document { city_root: PathBuf, at: Address },       // 锁外读一个文件的开头
    City(CityAsk),                   // 锁内取 run 摘要、停工范围与各追求，锁外列楼、读计划
    Building { city_root: PathBuf, addr: Address, plans: Arc<Mutex<PlanView>> }, // 锁外读楼的目录与它的计划
    Prefix(PrefixAsk),               // 锁内取折叠记下的那条记录的序号，锁外读账本那一行与内容仓库
    Changes { city_root: PathBuf, base: GitOid, head: Option<GitOid> }, // 锁外让 git 比较两个检查点
    Hunks { city_root: PathBuf, oid_a: GitOid, oid_b: GitOid, path: String }, // 锁外读一个文件的补丁
    Content { city_root: PathBuf, locator: Locator }, // 锁外读内容仓库里的一个对象
    Archives { city_root: PathBuf, needle: String }, // 锁外逐楼读档案架
    Skills { city_root: PathBuf, building: Address, pins: SkillPins }, // 锁内拷出钉住表，锁外扫书架
    Metrics { city_root: PathBuf, held: wire::MetricsAnswer },   // 锁内填好折叠里的数，锁外数楼
    Release,                         // 锁外经网络问发布页
    History { ledger: LedgerAsk, before: Option<Seq>, limit: u32 },      // 锁外刷新索引、读一段历史
    HistoryRange { ledger: LedgerAsk, from: Seq, to: Seq, limit: u32 },  // 锁外读一个区间
    RunHistory { ledger: LedgerAsk, run: RunId, before: Option<Seq>, limit: u32 }, // 锁外读一个 run 的记录
    Rounds { ledger: LedgerAsk, run: RunId },                            // 锁外读一个 run 的记录再折成回合
    Evidence { ledger: LedgerAsk, run: RunId },                          // 锁外读一个 run 留下的定位符
}
// accounting::views::answering
impl Views { pub(crate) fn prepare(&self, query: &wire::Query) -> Prepared; } // 只读视图
impl Prepared { pub(crate) fn finish(self) -> wire::Answer; } // accounting::views::prepared
pub(crate) fn answer_outside_the_lock(
    views: &Published,
    query: &wire::Query,
) -> wire::Answer; // 在快照上 prepare，放下快照再 finish（§8-99）
```

**I/O 在放下快照之后。** `answer_outside_the_lock` 取一份快照（§8-99），`prepare` 把查询要的小数据（地址、检查点那一行、城根路径）拷出来，放下快照，再由 `finish` 做磁盘、git 或网络的 I/O。控制面（socket 与终端）的查询都经过它。被拒：在锁内跑 `git status`——变更页开着时每次刷新都持锁几十毫秒，折叠线程等它，run 的相邻事件到达客户端的间隔就被这个页面拉长。

**变体是穷尽的枚举，而不是一个 `Box<dyn FnOnce>`。** 每种锁外的 I/O 有名字，`match` 列全，新加一种要在这里写出它锁内拿什么；闭包会把这件事藏进调用点。

**`Prepared` 与它的 `finish` 自成一个模块。** `answering` 管「一个查询在锁内拿什么」，`prepared` 管「锁外怎样把它读完」；两者分开，锁外新添一种读法时不必动锁内那张表。

**`Skills` 在锁内拷走整张钉住表。** 表的大小是「run 数 × 每个 run 钉住的技能数」，拷它是纯内存的一段；按这栋楼的书架先筛再拷，就得在锁内扫书架，正是要挪出去的那次读盘。

```rust
// bin::plan_view
pub(crate) fn plans_of(
    shared: &Mutex<PlanView>,
    city_root: &Path,
    addrs: BTreeSet<Address>,
) -> BTreeMap<Address, PlanReading>; // 锁内描述缓存里有的；放锁读没读过的表；再锁一次放回代数没动过的读数
// accounting::views::city —— shape: projection
pub(crate) struct CityAsk { city_root: PathBuf, plans: Arc<Mutex<PlanView>>, held: wire::CityAnswer, pursuits: Vec<(Address, String, PursuitState)>, in_flight: u32 }
impl CityAsk { pub(super) fn read(self) -> wire::Answer; } // 锁外列楼的目录、读计划、算每个追求的判词
// accounting::views::prefix
pub(crate) struct PrefixAsk { ledger: LedgerAsk, run: RunId, first: Option<Seq> }
// accounting::views::prepared
pub(crate) struct LedgerAsk { city_root: PathBuf, index: Arc<Mutex<storage::LedgerIndex>> } // 历史、回合与证据带出快照的那一份账本
impl PrefixAsk { pub(super) fn read(self) -> wire::Answer; } // 读不到那一行或内容仓库打不开：Unavailable
```

**账本索引有自己的锁。** 索引是账本文件的缓存，折叠从不碰它（它在查询时才刷新），所以它不属于视图的锁：`Views` 持 `Arc<Mutex<storage::LedgerIndex>>`，`Prefix` 把这只 `Arc` 带出视图的锁，在 `finish` 里锁索引、刷新、读一行。读者之间仍为索引互等，折叠线程不等。取索引只有一道门，`LedgerAsk::indexed`：锁、刷新，交出持锁的索引。索引的锁中毒时，这道门把锁里的索引换成一份空索引、解除中毒，再照常刷新；空索引的刷新就是一次整本扫描，所以中毒只让下一个读者多扫一遍，之后回到增量刷新。中毒的那份不能照读，因为半刷新的偏移表会把别的行当成要找的那一行。刷新失败时读者照旧按读不到作答（`Prefix` 答 `Unavailable`，历史答空页）。被拒：中毒后一律按刷新失败作答——中毒在进程里不会自己消失，此后每个历史、轮次与 `Prefix` 问题都悄悄答空；被拒：`finish` 里每次另建一份索引——每问一次就是一次整本扫描（五万条约 14 ms），比锁内那一段还长。

**第一条 `prompt_assembled` 的序号由折叠记下。** `apply` 为每个 run 记它第一条 `prompt_assembled` 的 `Seq`（每个 run 一个数），`finish` 只读这一行；原先是锁内倒着读这个 run 的每一行找它。整条记录不进折叠：四段的来源表随文档数增长，而每个会话多占的内存是这个进程要压低的量。

**计划缓存有自己的锁，回填看代数。** `Views` 持 `Arc<Mutex<PlanView>>`：折叠在 `apply` 里锁它作废读数，读者只在锁内做纯内存的描述，读表在锁外。`PlanView` 为每栋楼记一个代数，任何可能动到这栋楼计划的记录（`apply` 作废它的那一刻）都让代数加一，不带地址的这类记录让全城的代数加一。每楼的代数最多记 1024 栋（`GENERATIONS_HELD`）：再来一栋新楼时全部清掉、全城代数加一，于是在途的回填各被拒一次，而不是让这张表随进程见过的每栋楼增长；只清每楼的数而不动全城的数是错的，因为清零后那栋楼再动一次，它的数又回到读者问时的值。`plans_of` 在第一次锁内给每栋没读过的楼拷出停因表与当时的代数，锁外读表，第二次锁内只放回代数仍相等的读数：两次锁之间折进来的记录可能刚让这份读数过时，放回去就会让缓存一直报旧计划，直到下一条动它的记录。被拒：读到的一律不回填——每次问都要重读每栋楼的表，这正是缓存要省掉的读盘。计划缓存的锁中毒时，下一个持锁者（读者或折叠）用 `PlanView::take_back` 把它收回：丢掉读数与代数（恐慌可能撕了它们，于是每栋楼重读一次表，在途的回填因全城代数加一而被拒），保留停因表（每条停因由一次插入整条写入），再解除中毒。被拒：中毒后弃用缓存——那样每个红节点的句子都悄悄退回表格上的状态词，没人知道为什么；而答 `StorageFatal` 在这里代价过高，因为缓存里没有一样东西是恐慌能弄假而重读修不回来的。

**`CityView` 与 `BuildingView` 都在锁外读盘。** `prepare` 只拷 run 摘要、停工的范围、各追求（地址、目标、状态）与在飞的 run 数，`CityAsk::read` 在锁外列楼的目录，用 `plans_of` 取这些楼与各追求所在地址的计划，再算楼的进度与每个追求的判词。`BuildingView` 带出计划缓存的 `Arc`，在 `finish` 里读楼的目录并用同一个 `plans_of` 取它的计划。

**历史、回合与证据只带出账本。** `History`、`HistoryRange`、`RunHistory`、`Rounds`、`Evidence` 不读折叠里的任何东西，`prepare` 只拷城根并克隆索引的 `Arc`（`LedgerAsk`），刷新索引、读行、折成回合或挑出定位符都在 `finish` 里做。于是在锁内作答的查询都只读折叠，不碰盘：`Commit`、`Commits` 也在此列。

### 8-90 服务中的城在后台证明整条链，证明之前与链断之后写者都不写（`bin::assembly::chain_watch`）

证明线程的四种结局（完好、断链、读不了、恐慌）怎样落到停机值上的权威是 `crates/sprawling/spec/Assembly/ChainWatch.lean`：每种结局都给出判定，只有完好才放行；停机值本身的权威是 `crates/storage/spec/ChainAudit.lean`。本节是接口、线程与理由。

```rust
// accounting::worker::chain_halt —— shape: adapter，随 worker 住（它读写者的私有字段）
pub struct ChainUnderAudit {
    pub halt: storage::ChainHalt,           // awaiting_proof()：判定之前拒绝追加
    pub ledger_dir: PathBuf,
    pub at: Seq,
    pub records: storage::ProofRecords,     // 这个写者持着锁，所以能写
}
impl RunWorker {
    pub fn chain_under_audit(&mut self) -> ChainUnderAudit; // 把等证明的停机值接到写者上，交回证明要的四样
    pub fn await_proof(&self);                              // 关城之前等证明有结局
}

// bin::assembly::chain_watch —— shape: adapter，留在装配根（它起线程）
pub(super) fn audit_in_background(
    watch: ChainUnderAudit,
    log: runtime::diagnostics::Diagnostics,
    began: Instant,                          // 开城的起点，就绪时刻从它量起（§8-122）
) -> Result<std::thread::JoinHandle<()>, AxError>; // StorageFatal「start the chain audit」：线程起不来
```

**两步，先接上停机再起线程。** `chain_under_audit` 新建一个 `storage::ChainHalt::awaiting_proof()`，经 `JsonlLedger::halt_on` 接到这个 worker 的写者上，交回停机值、账本目录、写者此刻的位置与能写的记录（`JsonlLedger::proof_records`，记录目录由 `accounting::views::snapshot::start::proof_dir` 给出）；`audit_in_background` 再在名为 `sprawling-chain-audit` 的线程上跑 `storage::prove_chain`（`crates/storage/Spec.lean` §8-30）。前一步读写者的私有字段，所以随 worker 住；后一步起线程，所以留在装配根（accounting D17）。线程只持有账本目录、停机值、记录和自己的 `Diagnostics`，不碰写者，所以写线程从不等证明。`serve` 的写线程在 `open_for_service` 之后依次调用这两步；起不来的线程与起不来的写线程一样，让 `serve` 失败。citysim 与测试不走这条路，所以它们的时序里没有第二个线程。

**证明之前写者不写。** 停机值在判定之前拒绝每一次追加，答 `E_HISTORY_UNPROVEN`：页面上发来的命令照样被服务，人收到的是这个码与它的恢复办法，账本一行不多（`crates/accounting/spec/Worker/Attend.lean` 的 `nothing_is_written_before_the_proof`）。`open_for_service` 写的开城那几行在停机值接上之前写下，它们是城自己的开门，不是按历史作出的判断。关城时写者先 `await_proof`，交接那一行在证明有了结局之后写：完好就照常写下，断链就被拒并说出断链的原因。证明线程若没有给出判定就结束（恐慌），它持有的守卫在退栈时以「证明没有走完」跳闸，关城不会永远等下去。

**结果作为诊断推给页面。** 证明线程的 `Diagnostics` 与写者的那一份同一个落点（`serving::Journal` 的 sink）、同一个级别下限，所以页面在日志里读到这一行：`Whole` 先 `prove`，再写一条 `Effect`：

`the history is proved: <n> lines, <k> segment(s) by digest, <c> lines checked, <r> bytes read, <h> bytes hashed, in <ms> ms, <m> ms after opening began; commands are taken`

毫秒由 `opening_cost::millis` 渲染（§8-121），前一个时长是线程开头与结尾两次 `serving::standing::monotonic_now` 之差，后一个从 `listen` 交进来的开城起点量起（就绪时刻 M3，§8-122）。记录写不成（`Proven.unkept`）再写一条 `Refuse`：下一次开城这些段逐行核对，结果相同，只慢一些。`Broken(reason)` 先 `trip(reason)`，再写一条 `Refuse`，内容就是原因与恢复办法；读账本本身失败（`StorageError`）同样先 `trip`，原因是那次读取的失败：没读完的证明没有证明链断了，也没有证明它完好，而视图与 Standing 从快照起步（§8-122）时，快照之前的行只有这次证明会看；写在一条未经证明的链后面的行，与写在断链后面的行一样收不回来。

**视图不需要第二个停机值。** 视图只折写者已经写下的记录（§8-99），写者停了，视图也就不再有新工作；给视图再接一个 `ChainHalt` 会让「这座城还收不收工作」有两处定义。被拒的命令经写者的 `StorageError`（`Unproven` 或 `ChainHalted`）回到页面，说的与诊断是同一件事。

**被拒：证明放在启动路径上同步做完再开端口。** 那是开城前的全链读取；证明的价值在于它不挡首字节。

### 8-91 视图从快照起步：编码、切快照、只折尾部（`accounting::views::snapshot`、`accounting::views::snapshot::start`、`accounting::worker::folds::views_start`）

```rust
// accounting::views::snapshot::start —— shape: projection
pub(crate) fn city_root_of(ledger_dir: &Path) -> &Path;   // 账本目录上两级
pub(crate) trait SnapshotFold: Sized {
    const DIR: &'static str;                  // <city>/.sprawling/snapshot/ 下这个折叠自己的目录
    fn fold_version() -> u32;
    fn empty(city_root: &Path) -> Self;
    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Self, AxError>;
    fn encode(&self) -> Result<Vec<u8>, AxError>;
    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>;
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError>;
}
pub(crate) struct Started<F> { pub(crate) folded: F, pub(crate) from: FoldStart, /* 最后一行：切快照用 */ }
pub(crate) enum FoldStart { Resumed { tail: usize }, Whole(storage::WholeFold), Alongside } // Alongside 见 §8-122
pub(crate) fn start<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError>;
pub(crate) fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError>; // 先 storage::prove_chain（只读记录），Broken(reason) 原样返回
pub(crate) fn cut<F: SnapshotFold>(ledger_dir: &Path, started: &Started<F>) -> Result<(), AxError>;
pub(crate) fn cut_at<F: SnapshotFold>(ledger_dir: &Path, folded: &F, last: Option<&(Seq, Vec<u8>)>) -> Result<(), AxError>;
pub(crate) fn last_line(index: &LedgerIndex, ledger_dir: &Path) -> Result<Option<(Seq, Vec<u8>)>, AxError>;

// accounting::views::snapshot —— shape: projection
impl Views {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, AxError>;                         // StorageFatal「encode the views」
    pub(crate) fn decode(city_root: &Path, bytes: &[u8]) -> Result<Views, AxError>;  // CasCorrupt「decode the views」
}
pub(crate) fn views_fold_version() -> u32;
impl SnapshotFold for Views { const DIR: &'static str = "views"; /* … */ }
impl Views { pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError>; } // start_audited::<Views>，不切快照
impl Views { pub(crate) fn cut_snapshot_at(&self, record: &EventRecord) -> Result<(), AxError>; } // 在已折的最后一条记录处切，行是它的 canonical_line

// accounting::worker::folds::views_start —— shape: projection
pub(crate) fn start_served_views(ledger_dir: &Path, log: &mut Diagnostics, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // fold_city 的错误原样返回；切快照的失败只进 log；cost 见 §8-121
```

**起步路径放在视图一侧。** `snapshot::start` 由 `views` 持有：一次性查询经 `Views::rebuild` 从这里起步，而 `views` 的生产代码不点名 `crate::assembly`（§8-92 的方向块）；`Standing` 的起步（`accounting::worker::folds::standing_start`）从 assembly 指向 views，方向与 assembly 取用 `views::Governance` 相同。**被否：留在 `accounting::worker::folds`，由 `assembly` 提供 `Views::rebuild`。** 那让读面为了一次性查询重新点名组装点，正是方向块拒绝的边。

**编码由各类型自己的 crate 给出。** `storage::HotView`、`storage::Attribution`、`gateway::EndpointBook` 与本 crate 的 `Governance`、`PlanView`、`CommitFacts` 各在定义处派生 `serde::Serialize`／`Deserialize`，`Views` 本身也派生，字节格式是 postcard。`city_root`、`index`（side cache 的 seq→偏移表）、`machine` 与 `vault` 不进快照：前两个由 `Views::new` 从盘上重建（`decode` 用结构更新语法取 `Views::new` 的这两个值，所以重建规则只有一处），后两个本来就不是从账本折出来的。`PlanView` 只存 `causes`，已解析的计划是缓存，下一次提问时重新读。**被否：JSON。** `skill_pins` 以 `(String, B3Hash)` 为键，JSON 的键只能是字符串；而且 JSON 读写都比 postcard 慢、体积更大，这些都记在首字节上。**被否：手写逐字段编码。** 那是每个字段的第二份拼写，加一个字段就要改两处。

**`fold_version` 不靠人记得改。** `views_fold_version()` 取 blake3(`CARGO_PKG_VERSION` ‖ `VIEWS_FOLD_RULES`) 的前四字节（LE）：换一个版本的二进制就丢弃旧快照、从创世折一次；同一版本内改了折叠规则或 `Views` 的字段，改 `VIEWS_FOLD_RULES` 这个常量。这条规则由 `views::snapshot::tests` 机器核对：常量写成 `views-fold-<16 位十六进制>`，后缀是一份固定夹具（三条手写记录，分别填进 Inbox、弃置箱与登记册；时间与序号都是常数，不经过时钟）折出的 `Views` 的 postcard 编码的 blake3 前缀；编码一变，测试给出新的常量值并失败，所以常量不可能停在旧编码上。夹具里一直为空的字段只在增删时改变字节，换了类型不会。**被否：只钉常量，或另钉一个哈希。** 前者什么也没核对；后者改字段时只需改哈希，常量照旧，旧快照照样被接受。字节解不开（postcard 报错）同样按 `WholeFold::Damaged` 退回全量折叠。

**两个折叠，一条起步路径。** 视图与 `Standing`（§8-101）各有一份快照，放在 `<city>/.sprawling/snapshot/` 下各自的目录（`views/`、`standing/`）里，各有自己的 `fold_version`，因为两者的编码各自改变；核对快照、折尾部、退回全量折叠、切快照只在 `snapshot::start` 写一次，由 `SnapshotFold` 接两种折叠。**被否：两个折叠共用一份快照。** 那让只改了一边编码的构建丢掉两边的快照，而且 `Standing` 在每个 worker 打开时就切，视图在 `serve` 起步时和服务中的折叠线程上切，两者的最后一行并不总相同。

**起步只折尾部。** `start::<Views>` 调 `storage::start_from_snapshot(ledger_dir, <city>/.sprawling/snapshot/views/, views_fold_version())`（`crates/storage/Spec.lean` §8-28）。`Resume`：解码快照里的 views，再用 `ChainSnapshot::resume()` 给出的 `LineCheck` 逐行核对并折尾部——尾部仍然过同一个逐行检查，行号从 `seq + 1` 数起。`Whole`：与没有快照时完全相同，`runtime::replay::fold_ledger_dir` 从创世一段一段地核对并折叠；`Whole` 只带原因，不带行（`crates/storage/Spec.lean` §8-28）。快照之前的行在起步时不再逐行核对：切快照时它们已经过一次完整的核对（从创世或从上一份快照起），`fit` 用一行的链哈希证明它们还是那些行；之后被改坏的行在服务中的城里由后台的证明（§8-90）抓住；一次性的查询（`views::ask` 经 `Views::rebuild`）旁边没有后台证明，所以 `Views::rebuild` 经 `start_audited::<Views>` 起步：先同步跑一次 `storage::prove_chain`，带着只读的已验证前缀记录（`crates/storage/Spec.lean` §8-30：记录命中的段只读、只哈希，其余逐行核对，从不写记录），只有它返回 `Whole` 才从快照起步作答，`Broken(reason)` 与读不了账本都原样拒绝。**被否：一次性查询从创世全量折叠。** 证明逐行核对的只是没有记录覆盖的行，内存是一段字节；全量折叠要解析并折叠每一行，建出整份视图。**快照校验失败绝不信任它**：任何一种 `WholeFold` 都走全量折叠，原因放在 `from` 里；`serve` 把两份折叠各自的 `from` 写成一条 `Effect` 诊断（`the views <起步>; the standing <起步>`），说明这次从哪里起步、为什么。

**切快照：服务起步时，折叠越过快照就切一次。** `serve` 调 `start_served_views`：它先经 `fold_city` 让视图与 worker 的 `Standing` 从各自的快照一遍读起（§8-122），再在视图折过的最后一行切一份视图快照（`snapshot::start::cut_at`），最后交出视图；这一遍没有折到任何新行时什么也不写。这个频率不含常数：切一次的代价是一次编码加一次 `sync`，与视图大小成正比；它省下的是下一次起步重折这段尾部的时间，与尾部长度成正比，而尾部只在上次起步之后增长。一次性的查询（`views::ask`）经 `Views::rebuild` 只读快照，不切：读命令不写盘。**写不下快照不让 `serve` 失败**：失败写成一条 `Refuse` 诊断，内容是失败原因与恢复办法，视图照常交出。快照只是下一次起步的捷径：没切成，下一次起步从旧快照或从创世多折一段，结果逐字节相同，只慢一些；而账本写不下时历史本身就缺了，两者不能同样对待。**被否：写不下快照就让 `serve` 失败。** 那让一个只影响下次起步速度的故障（快照目录满、权限错）挡住整座城。

**切快照：服务中，按测得的折叠成本。** 折叠线程（`serving::folding`）每批把记录折进两份视图之后，在这一批最后一条已提交记录处用备用份切一份视图快照（`Views::cut_snapshot_at`）：那条记录的 `canonical_line` 就是账本里的那行字节，所以切快照不再读账本，而备用份此刻没有读者。节奏由 `folding` 里的 `Cadence` 定：它累计自上次切快照以来折叠已提交记录所花的时间（下一次起步要重折的尾部，大致就是这么多时间），累计到上一次切快照所花时间的 `CUT_SHARE_INVERSE`（= 10）倍时再切；还没量过切快照时，第一批之后就切，并量出它。通道关闭（写线程与另两处落点都已放手）时，自上次切快照以来折过记录就再切一次，于是正常停下的城下一次起步不折尾部。于是切快照占折叠线程的时间不超过十分之一，而下一次起步要重折的尾部不超过十次切快照的时间；两者都从运行这座城的主机上测出的时间推出，没有按某一类机器调的行数或秒数。时间由 `bin::assembly` 以 `clock` 参数交进来，交的是单调时钟唯一的采样点 `serving::standing::monotonic_now`，`folding` 的节奏自己不采样，测试可以交另一只钟。**视图拒折过一条记录之后，本进程不再切**：快照会把被拒的那条当作已折叠，下一次从快照起步就接受了全量折叠会拒绝的历史。切不成（编码失败、写盘失败）只写一行 stderr，视图照常发布：快照只是下一次起步的捷径。**被否：另起一条线程再读一遍账本来切。** 那是每次切快照多读一遍尾部的行，而折叠线程手里已经有这些字节。**被否：固定每 N 条记录切一次。** N 在慢盘上太密、在快盘上太疏，切快照的代价与视图大小成正比，不与记录条数成正比。

**本节接口的当前状态**：切快照（编码与 `sync`）在折叠线程上做，这一批之后的下一批要等它写完；把写盘交给另一条线程、折叠线程只编码，是这一接口余下的一步。页面上看不到这次开城从哪里起步、历史证明到哪一行：两者要成为 `CityAnswer` 的字段（改线格式），随波次 4 的 W6 批进；在那之前它们在日志里（上文与 §8-90）。

### 8-101 worker 的 Standing 从快照起步（`accounting::worker::folds::standing_start`）

```rust
// accounting::worker::folds::standing_start —— shape: projection
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct StandingFolds { book, governance, collaboration: CollaborationFold, entrance, origins }
impl SnapshotFold for StandingFolds { const DIR: &'static str = "standing"; /* … */ }
impl StandingFolds { pub(super) fn settle(self, cut: Result<(), AxError>) -> Result<Standing, AxError>; }
pub(in crate::assembly) fn as_json_text<T: Serialize, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error>;
pub(in crate::assembly) fn from_json_text<'de, T: DeserializeOwned, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error>;
```

**快照存的是还没 settle 的折叠。** `Standing` 里的 `Collaboration` 是 `CollaborationFold::settle` 的结果：信号的入队与消费按工作发生的顺序到达，队列要到最后一行之后才能算出；尾部再折进来时还要那些被搁置的信号，所以快照存 `CollaborationFold`，不存 `Collaboration`。五个折叠对一条记录的处理只写在 `StandingFolds::absorb` 一处，从创世与从快照起步都走它。

**postcard 装不下的字段写成 JSON 文本。** `Entrance.refused` 里的 `AxError` 用 `#[serde(flatten)]`，postcard 不支持；`CollaborationFold.enqueued` 里的信号带 `Payload`（JSON 值），postcard 读不回来。前者整张表经 `as_json_text` 写成一段 JSON 文本；后者先经写者自己的 `Signal::enqueued_payload` 变回入队记录的载荷，读回时经它的逆 `Signal::from_payload`，所以信号的形状只有一处定义。`Entrance.carrying` 是本进程正在执行的命令，不是从历史折出来的，不进快照。**被否：给 `AxError` 另写一份不带 flatten 的编码。** 那是这个类型的第二份拼写，线上的 JSON 形状与快照的形状会各自漂移。

**起步之前先证明整条链。** `Standing::fold` 经 `start_audited::<StandingFolds>` 起步：先同步跑一次 `storage::prove_chain`，带着只读的已验证前缀记录（`crates/storage/Spec.lean` §8-30），只有它返回 `Whole` 才调 `start`；证明后起步只在 `start_audited` 写一次，一次性查询（§8-91）与它共用。`Broken(reason)` 原样返回，worker 打不开，读不了账本同样拒绝。起步本身看不到快照之前的行：`fit` 只核对快照那一行，`JsonlLedger::open` 的尾部扫描只读最后一段；`resume`、`fork`、`adopt` 与一次性命令打开的 worker 旁边没有后台证明（§8-90）。于是某个封好的段里一行被改写成另一行，仍然规范、仍接得上前一行，只断了下一行的 `prev`，除了这次证明谁都看不到；有了它，从快照起步绝不接受一条全量折叠会拒绝的链。记录命中的段只读、只哈希，所以每次打开 worker 仍要读一遍整条链的字节，逐行核对的只是记录之后长出的行。服务中的城不走这里：它的 `Standing` 在 `fold_city` 里与视图一遍起步，证明在后台，证明完成之前写者不写（§8-90、§8-122）。**被否：服务中的 worker 在后台证明完成之前就按历史接受命令。** `Standing` 决定 worker 接不接一条命令，按一段没证明过的历史接受命令，写下的行接在一条可能断了的链后面。

**每次打开 worker 都切。** `Standing::fold` 先 `start::<StandingFolds>`，折过了快照之后的行就切一份新快照，再 settle。切不成不是折叠的错：结果放在 `Standing.cut` 里，`RunWorker::over` 把它写成一条 `Refuse` 诊断，worker 照常打开，理由与 §8-91 相同。账本目录不存在时什么也不读、不切。

**`fold_version`**：blake3(`CARGO_PKG_VERSION` ‖ `STANDING_FOLD_RULES`) 的前四字节（LE）；同一版本内改了折叠规则或 `StandingFolds` 的字段，改 `STANDING_FOLD_RULES`。这条规则由 `accounting::worker::folds::standing_start::tests` 机器核对：常量写成 `standing-fold-<16 位十六进制>`，后缀是一份固定夹具（两条手写记录，一条信号入队、一条认领，填进协作折叠的信号队列与计划持有表；时间与序号都是常数）折出的 `StandingFolds` 的 postcard 编码的 blake3 摘要前 16 位，编码一变测试就给出新值。

**本节接口的当前状态**：夹具只填了协作折叠；另外四个折叠（`EndpointBook`、`Governance`、`Entrance`、`SessionOrigins`）在夹具里是空的，摘要只钉住它们空时的编码。其中一个在非空时改了编码（字段顺序不变、含义变了）而忘了改常量，同一版本的二进制仍会接受旧快照；给夹具补上这四个折叠各自的一条记录即可合上。

### 8-121 开一座服务中的城，每一段花了多少（`accounting::worker::opening_cost`，形状：value）

```rust
// accounting::worker::opening_cost —— shape: value
pub(crate) enum Phase {
    Bind,                           // 拿端口
    OpenLedger,                     // 取写者锁、探版本、恢复尾段
    FoldTail { lines: u64, from: TailFrom }, // 从较早的快照切点（或创世）一遍核对并折两份（§8-122）
    CutStanding,                    // 切 Standing 快照
    CutViews,                       // 切视图快照
    Twin,                           // 克隆第二份视图（§8-99、§8-144）
    StartWorker,                    // 起写者线程
}
pub(crate) struct OpeningCost { /* 私有：钟、起点、上一记、各段 (Phase, Duration)、折叠累计 */ }
impl OpeningCost {
    pub(crate) fn begin(clock: fn() -> Instant) -> OpeningCost;   // 钟由 bin::assembly 交进来
    pub(crate) fn lap(&mut self, phase: Phase);                    // 记下上一记到此刻这一段
    pub(crate) fn began(&self) -> Instant;                         // 开城的起点，交给后台证明量 M3（§8-122）
    pub(crate) fn line(&self) -> String;                           // 唯一的渲染
}
/// 一段时长的毫秒写法：整数微秒换算出三位小数，不经浮点；超出 u64 微秒的饱和到 u64::MAX。
pub(crate) fn millis(span: Duration) -> String;
// accounting::worker::folds
pub(crate) fn fold_city(ledger_dir: &Path, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>;
// accounting::worker::folds::views_start
pub(crate) fn start_served_views(ledger_dir: &Path, log: &mut Diagnostics, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>;
```

**要什么。** 首字节之前 `listen` 做的每件事各花多少，由产品自己说出来，不靠从外面拿首字节减来减去推算。`listen` 在拿端口之前 `begin(serving::standing::monotonic_now)`，之后每做完一段 `lap` 一次；`fold_city` 与 `start_served_views` 在自己做的那几段后 `lap`。写者线程起好之后，`listen` 在与 `serve` 其余诊断同一个落点、同一个下限上写一条 `Effect`：

`opened the city in <总> ms: bind <a> ms, open the ledger <b> ms, fold <n> lines from the snapshots <c> ms, cut the standing snapshot <d> ms, cut the views snapshot <e> ms, copy the views <g> ms, start the worker <h> ms`（从创世读起时写 `from genesis`）

毫秒由 `millis` 渲染。下限为 `off` 时不写。后台整链审计（§8-90）在首字节之后才结束，另写它自己那一行，毫秒同样经 `millis`。

**时间从哪来。** `OpeningCost` 不自己读钟：钟是 `begin` 收下的函数指针，与 `spawn_folding` 的 `clock`（§8-99）同一种交法，采样仍只发生在 §8-93 那一个单调采样点上。一段的长度是两次单调采样之差，墙钟被人调过也不会让它变成负数。

**决定。**
1. 一行，不是七行。七段是同一次开城的七个部分，读的人要看的是它们之间的比例。被否：每段一条 `Trace`——默认下限看不见它们，打开 `trace` 又会被别的行淹没。
2. 读数是诊断，不是账本记录。开城用了多久是运行这座城的主机在那一刻的事实，不属于城的历史（`docs/logging.md` §2）。被否：写一条账本事件——同一段历史在两台机器上就不再逐字节相同。
3. `line()` 是唯一的渲染，没有读者解析它。要逐段比较的人读 `bench_startup` 留在夹具城旁边的日志（`tools/citysim/Spec.lean` §8-5-1），那一行与首字节读数出自同一次开城。
4. 派活不在这里拆段。派活的两次读数已在 `prepare_dispatch` 那一行（`[prepare_dispatch]`），它走城钟、按毫秒；把 `stage_dispatch` 内部拆成微秒级的段，要一个能在 accounting 里取单调时间的端口，那是记账线程长任务那一项自己的工作。

**测试。** `assembly::listening::tests::a_listening_city_says_what_opening_it_cost`：在一座刚 `init` 的城上 `listen`，`log` 取 `Effect` 下限、sink 是同一个 `Journal`，事先订阅 `Journal::lines()`；收到恰好一条以 `opened the city in ` 开头的 `Effect`，七个段名按上面的次序出现，并含 `fold 3 lines from genesis`。

### 8-122 开城从两份快照一遍读起，历史在后台证明，四个就绪时刻（`accounting::worker::folds`、`accounting::views::snapshot::start`，形状 7 投影；`bin::assembly::chain_watch`，形状：状态机）

```rust
// accounting::views::snapshot::start —— shape: projection
pub struct BothStarted<A, B> {
    pub first: Started<A>,
    pub second: Started<B>,
    pub checked: u64,        // 这一遍逐行核对的行数
    pub from: TailFrom,      // 这一遍从哪里读起
}
pub enum TailFrom { Snapshots, Genesis }
pub enum FoldStart { Resumed { tail: usize }, Whole(WholeFold), Alongside } // Alongside：另一份要从创世折，同一遍带上它
pub fn start_both<A: SnapshotFold, B: SnapshotFold>(ledger_dir: &Path) -> Result<BothStarted<A, B>, AxError>;
pub fn proof_dir(city_root: &Path) -> PathBuf;   // <city>/.sprawling/snapshot/verified/，已验证前缀记录的唯一路径
// accounting::worker::folds
pub(crate) fn fold_city(ledger_dir: &Path, now: TimeMs, cost: &mut OpeningCost)
    -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError>; // 开账本，再 start_both::<Views, StandingFolds>
```

**要什么。** 开一座城到首字节从 40 万行城的 8.2 s 落到毫秒级（D41）：首字节之前只做与尾部长度、快照大小同阶的工作，整条链的证明不挡首字节，接受命令等证明完成。

**两份快照，一遍读。** 视图与 Standing 各有一份快照（§8-91、§8-101），切点各不相同。`start_both` 先读两份快照，各自核对 `fold_version` 并解码；两份都能用时，从较早的切点起经 `storage::start_from_snapshot` 取那一段尾部，每一行过同一个逐行检查（`ChainSnapshot::resume` 给出的 `LineCheck`），再只交给切点在它之前的那一份折叠。较晚的那份快照在这一遍里核对自己那一行（它的 `seq` 上那一行的链哈希），核对不上就在这一遍之后从创世再折它一次（`FoldStart::Whole(Stale)`，罕见的一支）。一份快照不能用（没有、坏了、版本不对、账本比它短），这一遍就从创世走，另一份也在同一遍里从创世折（`FoldStart::Alongside`），所以任何情形下首字节之前至多一遍逐行核对。性质是 `crates/storage/spec/Snapshot.lean` 的 `twoCutsOnePass`。**被否：两份各自经 `start` 起步。** 较晚切点之后的那段尾部被读、被核对两次。

**账本索引随视图快照恢复**（`crates/storage/Spec.lean` §8-4）：从快照起步的视图带着切快照时的索引，第一次历史查询只 `refresh` 之后长出来的字节。从创世走的那一遍照旧顺手建索引。

**历史在后台证明。** 快照之前的行在首字节之前一个字节也不核对；写者起好之后，`bin::assembly::chain_watch` 的线程用 `storage::prove_chain` 带着已验证前缀记录走整条链（§8-90、`crates/storage/Spec.lean` §8-30）：记录命中的段只读、只哈希，不解析；记录之后长出的行与没有记录的段逐行核对，然后写新记录。所以一次开城至多一遍逐行核对全量——升级之后第一次开城、或记录被删之后——其余开城只核对上次证明之后长出的行。证明完成之前写者拒绝每一次追加（`E_HISTORY_UNPROVEN`，§8-90）；查询不等证明，按从快照起步的视图作答。

**四个就绪时刻。**

| 时刻 | 意思 | 从哪读 | 目标 |
|---|---|---|---|
| M1 页面可见 | `GET /` 有首字节 | 本版与 M2 是同一刻：`wire::serve` 在 `listen` 返回之后才应答 | 毫秒级；见下文「本节接口的当前状态」 |
| M2 首个有效查询 | 两份折叠从快照起步、尾部核对完、视图发布 | `opened the city in …` 那一行的总时长（§8-121），其中 `fold <n> lines from the snapshots` 是这一遍 | 毫秒级；读量与尾部、快照同阶 |
| M3 接受命令 | 证明为 `Whole`，停机值放行 | `the history is proved: …` 那一行的后一个时长（§8-90） | 由 F1 在 40 万行城、记录命中时的读数定 |
| M4 完整证明 | 每一段都有本构建规则版本下的判定 | 同一行 | 与 M3 是同一刻：接受命令不早于完整证明（§8-101 的决定），证明完成的那一刻才放行 |

**首字节之前读什么**（`storage::snapshot::start::tests` 经 `Vfs` 缝计数）：首段的第一行（版本探测，§8-1 第 ③ 步）、前一段的最后一行（至多一个 16 KiB 的窗口，§8-1 第 ④ 步）、整个末段（尾部恢复逐行核对它，§8-1 第 ④ 步）、以及从较早切点所在的段起的各段。更早的段一个字节也不读，所以 M2 的读量与历史长度无关。

**计时。** `OpeningCost`（§8-121）的阶段改为 `fold <n> lines from the snapshots` 或 `from genesis`，读的人从同一行看出这次开城有没有从快照起步；`OpeningCost::began` 把开城起点交给证明线程，M3 从同一个起点量起。

**本节接口的当前状态。** M1 与 M2 是同一刻：`wire::serve` 在 `listen` 返回之后才开始应答，把静态页面的应答提前到折叠之前要改 `wire` 的服务入口，归 W 车道。尾部恢复从末段的已验证前缀起只逐行核对之后的字节（§8-144，`crates/storage/Spec.lean` §8-34）。`SnapshotStart::Resume.tail` 仍把尾部整段读进内存；正常关闭的城尾部为空，崩溃后的尾部是上次切视图快照之后写下的记录。Standing 的快照只在开城时切（§8-101），所以 Standing 的切点落后于视图，这一遍从 Standing 的切点读起：按与视图相同的节奏在服务中切 Standing，要先核对写者对 Standing 的增量更新与 `StandingFolds::absorb` 是同一条规则。页面从 `CityAnswer.proved` 读到历史已证明到哪一条（§8-134）。

### 8-144 开城的两段与一次性重建：末段按记录证明、第二份视图按值克隆、重建每行只核对一遍（`accounting::worker::folds`、`accounting::views::snapshot`、`accounting::views::snapshot::start`，形状 7 投影；`crates/storage/Spec.lean` §8-34，`crates/accounting/Spec.lean` §8-19）

**原因是一组拆开的读数。** 40 万行夹具城（`bench_startup first-byte` 的 `l400k`，§8-122 之后，windows-x86_64、16 核、NVMe、release）开城 1220 ms，其中 `open the ledger` 576 ms、`fold 0 lines from the snapshots` 279 ms、`copy the views` 357 ms。逐段拆开量：
- `open the ledger` 几乎全是尾部恢复逐行核对末段（40,843,081 B）；同一段读一遍 15 ms、BLAKE3 一遍 23 ms。
- `copy the views` 是经快照编码复制第二份视图：编码 214 ms、解码 139 ms（15,858,109 B）；按值克隆同一份 22 ms。
- `fold … from the snapshots` 里，视图快照解码 139 ms（其中提交折叠 105 ms：64,000 个提交，oid 每个按 40 位十六进制读三次）、`storage::tail_after` 读末段并逐字节分行 50 ms、读两份快照 20 ms。

整城重建（`just bench` 的 `large_ledger_fold`，5 万行、没有快照也没有记录）p50 1286 ms：`Views::rebuild` 先经 `prove_chain` 逐行核对整条链，从创世全量折叠时又逐行核对一遍。

**四处改动，保证不变。**
1. `fold_city` 经 `JsonlLedger::open_reusing` 开账本，记录取自 `proof_dir` 的只读一份（`crates/storage/Spec.lean` §8-34）：末段的前缀按摘要证明，逐行核对的只有上一次后台证明（§8-90）之后写下的行。正常关闭的城，那几行就是关城时写的交接。
2. `Views::twin` 是派生的 `Clone`（`crates/accounting/Spec.lean` §8-19(a)）：共享的句柄在 `Arc` 里，克隆后仍共享，结果与编码再解码相同。
3. `start_audited` 从创世折叠时不先证明，从快照起步时才先证明（`crates/accounting/Spec.lean` §8-19(b)）。
4. `storage::tail_after` 从含快照那一行的段尾往回找它，不再从段首把整段切成行（`crates/storage/Spec.lean` §8-28）。

**回退门是计数，墙钟只登记。** storage 的 `jsonl::open::tests` 在 N 与 2N 行上断言：记录覆盖末段之后再写 2 行，开账本逐行核对 2 行、按摘要复用 1 段。accounting 的 `worker::folds::views_start::tests` 在两种规模上断言：没有快照与记录时重建逐行核对的行数等于账本行数；快照与记录都在时等于之后长出的行数的两倍（证明一遍、折尾部一遍），与历史长度无关。开城各段与首字节的毫秒数由 `bench_startup first-byte` 与 `opened the city in` 那一行（§8-121）读出，进 `tools/xtask/budgets.toml` 的 `[first_byte]` 与 `[views_rebuild_per_mb]`。

**本节接口的当前状态。**
- `copy the views` 仍在开城那一行里（§8-121 的 `Phase::Twin`），`twin` 仍答 `Result`：它不再会失败，去掉 `Result` 与这一段要同时改 `bin::assembly::listening` 的调用点。

### 8-154 开城与证明再降一档：快照里的摘要是字节，证明按波读段（`accounting::views::snapshot`、`accounting::worker::folds::standing_start`，形状 7 投影；`bin::assembly::chain_watch`，形状：状态机；`crates/kernel/Spec.lean` §8-84，`crates/storage/Spec.lean` §8-37，`crates/accounting/Spec.lean` §8-24）

**原因是 §8-144 留下的两处读数。** 40 万行夹具城（`bench_startup first-byte` 的 `l400k`，windows-x86_64、16 核、NVMe、release，三轮各 3 个样本）首字节 p50 296–298 ms；`opened the city in` 约 261 ms，其中 `fold 0 lines from the snapshots` 189–190 ms、`open the ledger` 41–42 ms、`copy the views` 22 ms；`the history is proved` 那一行：6 段全按记录命中、0 行逐行核对、读与哈希各 376,383,853 B，证明 390–395 ms，开城起点之后 651–656 ms 接受命令（M3）。10 万行城（`l100k`）：首字节 p50 127–130 ms，折叠 53 ms，证明 98–99 ms，M3 192–193 ms。

**两处改动。**
1. 视图与 Standing 的快照里，`GitOid` 与 `B3Hash` 写成字节本身（`crates/kernel/Spec.lean` §8-84）。两份快照的夹具里都有这两种摘要，`fold_version` 随之进位，升级之后第一次开城从创世折一次、切新快照（`crates/accounting/Spec.lean` §8-24）。
2. 后台证明按波读段：一波至多 8 段，各段的读与记录前缀的哈希同时做，链仍按段序判（`crates/storage/Spec.lean` §8-37）。`chain_watch` 的那一行不变。

**保证不变。** 账本行、线上帧、`golden-p0`／`golden-s1`、wire 的两份 golden 都不变；证明的判定与逐行核对相同（`crates/storage/spec/Snapshot.lean` 的 `wavesAreStrict`）。回退门仍是计数：storage 的 `chain_audit::tests` 在 N 与 2N 段上断言五个计数（逐行核对的行数、按摘要的段数、读与哈希的字节、波数），accounting 的两个 fold-rules 测试钉住快照格式。

**改后的读数**（同一台机器、同一组夹具、同样三轮）。`l400k`：首字节 p50 249–259 ms；`opened the city in` 216–222 ms，其中 `fold 0 lines from the snapshots` 145–149 ms；证明 119–121 ms，开城起点之后 335–343 ms 接受命令。`l100k`：首字节 p50 116–119 ms，折叠 42 ms，证明 77–78 ms，M3 159–161 ms。一波 4 段（两波读完 6 段）时 `l400k` 的证明是 200 ms、M3 420 ms，所以一波取 8 段（`crates/storage/Spec.lean` §8-37）。

**本节接口的当前状态。**
- 视图快照解码去掉十六进制只省下约 44 ms，不是 §8-144 估的 105 ms；`fold … from the snapshots` 余下的 145 ms 还没有拆开量过。下一步是在这一段里分开读快照、postcard 解码各字段与尾部，按读数找下一处。
- 证明的墙钟由最长的一段定：`l100k` 只有两段，一段 64 MiB 读与哈希约 60 ms，所以证明 77 ms；在一段之内并行哈希（`crates/storage/Spec.lean` §8-37 的第二条被否）是段少的城的下一处余量。M3 在 40 万行城上仍是开城加证明，两者都在毫秒级。
- `copy the views`（22 ms）与 `open the ledger`（41 ms，读末段并按记录证明它）没有动。
- 升级之后第一次开城从创世折一次（`l400k` 约 7 s），因为两份快照的 `fold_version` 都进了位；之后的开城照上面的读数。
-/

/-! D3 开城不等整条链的证明，接受命令等（§8-122）

首字节之前只读尾部与两份快照，整条链在写者起好之后由后台证明，证明完成之前写者拒绝每一次追加（`E_HISTORY_UNPROVEN`），查询按从快照起步的视图作答。**被否：轻量审计，封好的段只取 `prev`、`seq` 并哈希整行。** 它不再证明每一行都能被这个构建逐字节规范地写出来，与 `LineCheck` 不等价，而开城之后读这些行的正是按 `LineCheck` 解析它们的视图与索引；已验证前缀记录让「只哈希」只用在已经逐行核对过、字节没有变的段上（`crates/storage/Spec.lean` §8-30）。**被否：证明完成之前接受命令、断链再停写。** 那是 §8-101 已被否的做法：写下的行接在一条可能断了的链后面，收不回来。
-/

/-! D4 开城变快靠少做重复的事，不靠放松保证（§8-144）

末段照样逐字节哈希、与记录比对，记录之后的行照样逐行核对；第二份视图是同一个状态的克隆；重建从创世时由全量折叠逐行核对，不再先证明一遍。**被否：开账本信任末段，只核对记录之后的字节、不哈希前缀。** 省下 23 ms，却接受了段被截短再接写、或被同长改写的那种意外，而 §8-30 的记录正是为挡它才带摘要。**被否：把复制第二份视图挪到视图线程上，首字节不等它。** 首字节确实不等了，但视图线程的第一批等它，装配根起线程的次序也要改；克隆把这一段从 350 ms 降到 22 ms，不需要挪。
-/
