-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::backlog

规定 `backlog`、`backlog::report`、`backlog::waiting`、`backlog::jobs`、`backlog::member`、`backlog::scratch`、`backlog::tail`（`crates/runtime/src/` 下同名的文件）。一个 run 进行时仍在跑的活：后台命令与委派下去的 run。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::backlog::tests` 守住。
-/

/-!
### 8-28 runtime::backlog（形状 4 适配器＋形状 6 数据面）


**问题**：`turn.rs` 首段写明中断只在相位边界被消费，而 `Command::output()` 阻塞在系统调用里、不在边界上，所以一条挂死的命令若不入表，`halt` 停不住；没有回合上限与花销上限的城里，这会是一处无界失效。

**设计**：一张表，成员是后台 exec 子进程与 `delegate` 子 run。

- **`exec` 恒经此表**，不设 `background` 参数——两条路径就是两个权威，而有洞的那条永远是没人想起的那条。
- 先阻塞等一个短窗口（**10 s**），短命令因而感觉上仍是同步的；超时则返回一个句柄并继续在后台跑，结果落**起它的那个 run 的下一次 `exec` 结果的尾部**，恒不落别的 run。给 agent 的说法是「长任务不要干等：起了它，做别的，结果到了会接在后面的工具结果尾部」。
- `halt {scope}` 遍历该表并终止其成员；工作线程不再进入阻塞系统调用，halt 因而真的停得住。
- `status` 报告表中属于本 run 的成员：几条在跑、各自跑了多久。

**红测试**：一条不会结束的命令起后，`halt` 使其进程终止且 run 回到边界；十秒内结束的命令不产生句柄；后台结果确实出现在起它的那个 run 的下一次工具结果的尾部，而同城另一个 run 随后的 `exec` 结果里没有它；一次调用的 verdict 恒只答它等到的那扇窗——窗口外落定的命令不回写本次结果，收割是下一次调用的事。

#### 8-28-1 接口与三处已定的实现选择

```rust
pub struct BacklogId(u64);                      // Display；一次 serve 内唯一
pub enum Started {                              // 短窗口的穷尽结果，恒不是 bool
    Settled { exit: Exit, stdout: String, stderr: String },
    Backgrounded { id: BacklogId, what: String },
}
pub struct Standing { pub id: BacklogId, pub scope: Address, pub what: String }
pub struct Finished { pub id: BacklogId, pub what: String, pub exit: Exit,
                      pub stdout: String, pub stderr: String }

// backlog::waiting —— 等多久，与「怎么结束的」两件事的唯一住处
pub struct PollBudget { /* polls、interval_ms 私有 */ }
impl PollBudget { pub const DEFAULT: PollBudget; pub fn new(polls: u32, interval_ms: u64) -> PollBudget; }
pub enum Exit { Ended { code: i32 }, Signalled, Unknown { why: Unseen } }
pub enum Unseen { LeftTheTable, WaitRefused }   // 各带一句给模型看的话

#[derive(Clone, Default)]
pub struct Backlog(/* Arc<Mutex<Table>>，表内是 BTreeMap */);
impl Backlog {
    pub fn with_window(window: PollBudget) -> Backlog;   // `new()` 即 PollBudget::DEFAULT
    pub fn run(&self, owner: RunId, scope: &Address, what: String, command: Command) -> Result<Started, AxError>;
    pub fn halt(&self, scope: Option<&Address>) -> Result<usize, AxError>;   // None＝整城
    pub fn harvest(&self, owner: RunId) -> Result<Vec<Finished>, AxError>;   // 只收 owner 起的
    pub fn release(&self, owner: RunId) -> usize;                            // owner 结束：终止它留下的命令，它们不再欠任何人
    pub fn standing(&self, scope: &Address) -> Result<Vec<Standing>, AxError>;
}
```

三处实现选择，各有理由：

1. **短窗口靠固定次数的轮询走完，不采样时钟，且窗口由调用方注入。** 默认 10 s ＝ 500 次 × 20 ms，住 `PollBudget::DEFAULT`；表持一份 `PollBudget`，`Backlog::with_window` 换掉它。理由是本仓那条「时间是入参，唯一采样点是 `bin::assembly`」——一个为了等十秒而调 `Instant::now()` 的模块会把那条规则打穿，而计数不需要时钟。**注入不是为了可配置**：`Settled` 与 `Backgrounded` 两种载荷形状不同，一个不能选窗口的调用方要观察后一种就得真等十秒，于是套件要么慢十秒、要么不判它交付的那个形状。

1b. **退出码是穷尽枚举，不是一个整数。** `Exit::Ended { code }`／`Signalled`／`Unknown { why }` 三臂：一个整数分不出「程序返回了负一」「被信号杀死」「本城没问出来」，而读结果的模型要分得出。`exec` 结果里 `exit_code` 键**只在 `Ended` 时出现**，另两臂写 `outcome`（`signalled`／`unknown`）与一句 `detail`；键名仍只由 `crates/runtime/src/tools/exec/outcome.rs` 拼（§8-26）。
2. **子进程的输出写文件，不走管道。** 管道缓冲区填满会让后台子进程停在写系统调用上，于是「后台」变成「挂死」——那正是本节要修的那个洞的另一种写法。文件住 `std::env::temp_dir()` 下按 `BacklogId` 命名的一层目录，收割时读完即删。
3. **表是一份共享句柄（`Clone` 的 `Arc<Mutex<_>>`）。** 装配层持一份，每个 `ExecTool` 持一份克隆，于是 `halt` 够得着 `exec` 起的东西而不必让 `halt` 认识 `exec`。表内是 `BTreeMap`，遍历序恒定。
4. **后台命令的结局只欠起它的那个 run（`owner: RunId`）。** 表是全城一张（第 3 条），所以「谁收割」必须由表按成员记下的 owner 判，而不是由「谁先调 `exec`」判：不带 owner 的 `harvest` 会把一栋楼（包括 confidential 楼）里一条命令的 stdout／stderr 原文交给城里任何一个随后调 `exec` 的 run，进它的模型与账本，而起它的 run 反倒收不到。一条命令的归属是穷尽枚举 `Claim` 而不是 `bool`：`Window(RunId)`（短窗口里，归正在轮询它的那次调用）、`Run(RunId)`（已转后台，只交给这个 run 的 `harvest`）、`Nobody`（它的 run 已结束，结局不交给任何人）。`ExecTool` 在 drop 时调 `release(run)`：每个 run 的工具台在它冻结后被丢弃，于是 drop 就是「这个 run 再也收不到」的那一刻。`release` 把这个 run 的命令改记为 `Nobody` **并终止它们**（与 `halt` 同一个 `kill`）；此后任何一次 `harvest` 都会把已结束的 `Nobody` 成员不读即删（进程句柄与临时目录因此有界），输出不交给调用者。`release` 不会失败：它只把认领降为 `Nobody`、只终止进程，这两件事在表的任何状态下都成立，所以一张被死线程锁住的表它也照做（取 `PoisonError::into_inner`）；表的其余调用照旧答 `E_STORAGE_FATAL`。它的调用方是 drop，没有人可以转交失败，一个会失败的 `release` 只能被丢弃，而被丢弃的那一次正是命令被留着为一个不存在的 run 跑下去的那一次。**被否决的做法**：按 `scope`（楼或房间地址）收割——同一房间里前后两个 run 地址相同，于是后一个 run 仍会收到前一个的输出，而 confidential 的界是按 run 的模型与账本行划的，不是按地址。run 结束即终止它留下的后台命令（D3）。

#### 8-28-2 第二类成员：委派下去的 run

**问题**：表里若只有子进程，`halt {scope}` 遍历表时看不见任何 run——一轮委派下去的活在它的 scope 被停摆之后照样跑到冻结。所以委派下去的 run 是表的第二类成员。

**设计**：表内成员分两种身体，一张表、一个 `halt`：

```rust
pub enum BacklogKind { Command, Run }              // Standing 携带；恒不是 bool
pub struct Standing { pub id: BacklogId, pub scope: Address, pub what: String, pub kind: BacklogKind }
impl Backlog {
    /// 一轮委派下去的 run 入表。装配层在 drive 之前调，只对 `Depth::Delegated` 的派活调。
    pub fn enrol_run(&self, scope: &Address, what: String) -> Result<BacklogId, AxError>;
    /// `halt` 是否已经到过这名成员。run 的中断钩子在每个安全点问它，真即 `Interrupt::Cancel`。
    pub fn stopping(&self, id: BacklogId) -> Result<bool, AxError>;
    /// run 冻结后离表。不离表的成员会让 `standing` 报一轮已经结束的活。
    pub fn leave(&self, id: BacklogId) -> Result<(), AxError>;
}
```

- **一个 run 成员没有进程可杀**：`halt` 对它做的是把身体记成 `Stopping`，而 run 在下一个安全点问 `stopping` 并以 `Interrupt::Cancel` 走 `runtime::turn` 既有的取消路——取消因此仍只在相位边界被消费，§8-28 首段那条法则不动。
- **每个 call 之前问一次**：一次工具波是模型一条回复里要的 N 件副作用，不是一件。`SafePoint::BeforeToolCall { turn, call }` 是第五个安全点，`Turn::<ToolWave>::execute` 在每条 call **入账之前**问 `still_going(call) -> Interrupt`，答案走相位边界那一个 `consume_boundary`：Cancel 写下同一条 `cancel_received` 并终止回合，于是停城不必等整波跑完，也不会为没做的活留下 `tool_called`；Steer 写下 `steer_received`，这条 call 照做。两条 call 之间能立刻执行的指令仍只有「停」，改道的文字属于执行器的 `Window`，由执行器在回答之前折进去、在下一次组装时交给模型。答案取整个 `Interrupt` 而不是只说停不停的 `NextCall`，是因为窄答案让波内到达的 steer 只进了窗口、没进账本：模型读到了一段账本上没有来历的文字。
- **只有委派下去的 run 入表**。根 run 由 `Cancel` 结束，那是另一个动词（glossary「Halt」行）；把根 run 也入表会让 `halt` 与 `Cancel` 变成同一件事。
- **`harvest` 与短窗口都跳过 run 成员**：它们收的是进程的退出码，run 的结局在账本上。
- **`status` 的那一半**：末行 `backlog:` 追加在冻结序末尾（追加规则同 `neighbours`），报 `standing(addr)` 里属于本 run 地址的成员——`bg-3 cargo build (command)` 逐条分号相连，没有则 `none`。**不报跑了多久**：本表不采样时钟（§8-28-1 第 1 条），一个为了报时长而采样的 status 会是第二个采样点。
- **接线在 `accounting::worker::dispatching::running`**：`at.parent.is_some()` 时先 `enrol_run`，drive 结束后 `leave`；`Driving` 携 `member: Option<BacklogId>`，中断钩子在人的打断与 steer 之前先问 `stopping`。`halt` 在记账线程上被处理而子 run 在驾驶池上跑（`crates/sprawling/Spec.lean` §8-42）；红测试用 `attach_interrupts` 在子 run 的安全点上调 `halt`。

**红测试**：一轮委派下去的 run 起后，其 scope 被 `halt`，子 run 以 `cancelled` 冻结且没有再叫过模型；`status` 的第十四行报本 run 起的后台命令。

#### 8-28-3 exec 输出的实时流：读增量、装配点注入、线上帧、服务端与页面两段缓冲

**现状**：一个 `exec` 调用的 stdout／stderr 只在调用结束时随工具结果进账本，页面（client 的监视器）在那之前只看得到「运行中」。第 2 条让输出写进 scratch 下的 `out`／`err` 两个文件，所以实时流不需要改子进程怎么写，只需要有人在它还在写时读这两个文件的增量。

**读增量——`runtime::backlog::tail`（shape：adapter）**：

```rust
pub enum Stream { Out, Err }                          // 恒不是 bool
pub struct Chunk { pub run: RunId, pub member: BacklogId, pub stream: Stream, pub bytes: Vec<u8> }
#[derive(Clone)] pub struct Sink(/* Arc<dyn Fn(Chunk) + Send + Sync> */);
impl Sink { pub fn new(deliver: impl Fn(Chunk) + Send + Sync + 'static) -> Sink; }
impl Backlog { pub fn with_sink(self, sink: Sink) -> Backlog; }
impl PollBudget { pub(crate) fn read_per_poll(self) -> usize; } // interval_ms × READ_BYTES_PER_MS
```

- 没有失败返回：读增量是可丢弃的预览，账本里的工具结果才是这段输出的权威；一次读失败只让这一拍少送一块，下一拍从同一偏移再读，偏移只在真的读到字节后前移。
- `READ_BYTES_PER_MS = 64`（每秒 64 KiB，一个人在页面上读得过来的上界的数倍），一拍的上界由它乘间隔得出，stdout 与 stderr 各得一半，所以刷屏的 stdout 饿不死 stderr。
- 决定：sink 是一个闭包而不是 trait——今天只有一个生产装配者（服务端扇出）与一个测试收集者，两者都只要「交出一块」这一个动作。

- 窗口里的读停在交给后台那一刻，偏移随成员进表；此后本 run 的每次 `harvest` 对它自己的、仍在跑的后台命令从同一偏移接着读。块在放开表锁之后才交给 sink，所以 sink 慢不会让表上其他调用等它。

**装配点注入**：一座被端上来的城（`RunWorker::serve`）把一个 `Sink` 装到自己的 `Backlog` 上，sink 把每块译成 `wire::LiveOutput`（`crates/wire/Spec.lean` §8-48）交给 `Serving::outputs`，那里送进第四条广播通道。没有被端上来的城（citysim、replay、一次一条命令的 worker）不装 sink，所以一个字节都不读。

**页面缓冲**：`client/src/core/live_output.ts` 为每个 run 留一段 `Tail`（stdout、stderr 与丢掉的行数），每条流只留最新的 `LIVE_LINES = 400` 行；`tool_result` 一到就丢掉这个 run 的那段。监视器的终端记录把它画在仍在跑的那一条下面，丢掉的行数照 `mon_lines_cut` 说出来。

**服务端缓冲**：`sprawling::serving::output_ring`（`crates/sprawling/Spec.lean` §8-90）为每个 run 按字节留最新的一段，后来打开页面的会话在 `Welcome` 之后先经 `ServeConfig::outputs_so_far`（`crates/wire/Spec.lean` §8-48）拿到它，再接实时帧；这个 run 的 `tool_result` 落账时清空。

**设计**（四段共同遵守的规则）：

- **读的地方是短窗口的轮询，不另起线程**。`Backlog::run` 每一拍轮询在 `settle` 之后按上次的偏移读两个文件新增的字节，交给调用方注入的一个 sink；窗口外交给后台的命令由 `harvest` 那一拍同样读增量。sink 缺席（citysim、replay、没人看的城）时一个字节都不读，行为与今天相同。
- **每拍读的字节有上界**，按 `PollBudget` 的间隔推出而不写死：一拍最多读 `PollBudget::read_per_poll()` 字节，即 `interval_ms × READ_BYTES_PER_MS`，间隔越长每拍读得越多、喂给页面的速率不变；读不完的留到下一拍，所以一个刷屏的子进程让页面落后，而不让轮询变慢。
- **服务端每个 run 一个有界环形缓冲**，按字节计上界，满了丢最旧的整块；后来打开 run 页的会话先拿到缓冲里的内容，再接实时帧。丢了多少不上线：`LiveOutput` 没有这一栏，而加一栏要让 `WIRE_V` 再加一，换来的只是预览里的一个数——调用落账时整段输出本来就会到。缓冲在这次调用的 `tool_returned` 落账时清空，因为那时账本里的结果是这段输出唯一的权威。
- **线上是一种新的 `ServerFrame`**，与 `Delta` 同一条规则：可丢弃，不带账本序号，调用的结果落账时页面扔掉它，两者不一致时账本赢。这一帧让 `WIRE_V` 加一并重新生成 `client/src/wire.ts`。
- **页面也是有界环形缓冲**，按行计，监视器的终端记录画它；溢出的行数照 `mon_lines_cut` 的样子说出来。
-/

/-!
### 8-40 一个 backlog 的草稿目录：名字必须唯一，而 `BacklogId` 给不了这个唯一


```rust
static BACKLOGS: AtomicU64;                                  // 本进程开过几个 backlog
fn scratch_dir(&self, id: BacklogId) -> PathBuf;             // temp/sprawling-<pid>-<backlog>-<member>
```

**三条口径：**

1. **`BacklogId` 与目录名回答的是两个问题。** id 在**一个** backlog 内部指认一个成员，下一个 backlog 的 id 又从 1 开始；而目录落在整台机器共用的临时目录里。只用 `<pid>-<id>` 起名，等于假定那个计数器是进程全局的——它不是。同一进程开两个 backlog，两者的第一条命令必然撞同一个路径。
2. **撞上之后是无声的。** `File::create` 截断另一方正在写的文件，`collect` 读完即 `remove_dir_all`，删掉另一方还在写的目录。人看到的是一条**退出码为 0、输出为空**的命令——既不报错也不重试，因为从每一方各自的角度看都一切正常。
3. **生产只开一个 backlog，唯一性照样不交给调用方。** `accounting::worker::lifetime` 全进程一个，而每个跑命令的测试各开一个、`cargo test` 又让它们同时跑。这不构成「只是测试问题」：把唯一性建立在「调用方只会开一个」之上，是把一条不变量交给调用方保管。`backlog::tests` 直接判目录名而不去赛跑两条命令——缺陷本身是确定的，只有损害是时序性的。
-/

/-! D3 定规：run 结束即终止它留下的后台命令

- **决定**：`release(run)` 把这个 run 的命令改记为 `Nobody` 并终止它们，而不是让它们跑完（§8-28-1 第 4 条）。
- **理由**：结束之后没有任何一次工具结果能把结局交给它；`Sandbox` 放置的命令跑在 `Confined` 的副本里，而那份副本在同一次 drop 里被删，让它跑下去等于让它对着一棵已删的树跑；一条无主命令留着的进程、句柄与临时目录是每个会话的边际内存。
- **击败的备选**：让无主命令跑到自然结束、再把结局写成该 run 名下的一条账本事件——结局到达时已经没有读者，而事件表要为一件被选定不再发生的事多一种事件。
- **重开的参数**：`Host` 放置成为后台命令的常态、且其对宿主树的副作用本身就是人要的产物时，run 的结束不再是「没人要」的证据，这条规则应重新论证。
- **未决**：被终止的命令不在账本里留下一行：把「结束时终止了 N 条后台命令」写进账本需要 kernel 事件表多一种，归 `crates/kernel/spec/Event/Kind.lean` 的事件表。
-/
