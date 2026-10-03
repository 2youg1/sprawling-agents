-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 核心线程站在正常档之上，空转就降回

规定 `crates/sprawling/src/serving/standing.rs`（`bin::serving::standing`，形状：状态机）的安全阀 `Valve` 与一条核心线程 `CoreThread` 的档位（§8-93）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

时刻是自然数：Rust 的 `saturating_duration_since` 在模型里就是自然数的截断减法。`Valve` 自己不读钟，时刻作参数传进来，所以它的判定在模型里与在 `serving::standing::tests` 里一样逐点可验。平台调用（升档、降档）的结果是参数。

四组性质：

* **降回是吸收态**——`verdict` 一旦是 `lower`，此后每一轮都还是 `lower`（§8-93 决定 3：降回之后不再升）；
* **窗口没关上不判**——一轮结束时离窗口开出不足 `limit`，判定不变；
* **忙满一个窗口就判降回**，忙的时间不到窗口的 15/16 就不判；
* **线程只降一次、拒绝之后不再试**——设置为 `normal` 时从不升档；平台拒绝降档之后，此后的轮不再调用降档。
-/

namespace Sprawling.Serving.Standing

/-- 阀的判定（Rust：`Verdict`）。 -/
inductive Verdict where
  | keep
  | lower
  deriving Repr, DecidableEq

/-- 安全阀（Rust：`Valve`）：窗口从 `opened` 开出，`busy` 是窗口里忙的时长。 -/
structure Valve where
  limit : Nat
  opened : Nat
  busy : Nat
  verdict : Verdict
  deriving Repr, DecidableEq

/-- `Valve::new`：第一个窗口在 `now` 开出。 -/
def Valve.new (limit now : Nat) : Valve :=
  { limit, opened := now, busy := 0, verdict := .keep }

/-- `Valve::record`：线程在 `woke` 醒来、在 `slept` 再次阻塞。第一件结束在窗口开出 `limit` 之后的事关上窗口；关上时忙够 15/16 就判降回；下一个窗口从 `slept` 开出。 -/
def Valve.record (v : Valve) (woke slept : Nat) : Valve :=
  if slept - v.opened < v.limit then { v with busy := v.busy + (slept - woke) }
  else
    { v with
      verdict :=
        if (v.busy + (slept - woke)) * 16 ≥ (slept - v.opened) * 15 then .lower else v.verdict
      opened := slept
      busy := 0 }

theorem lower_is_absorbing (v : Valve) (woke slept : Nat) (h : v.verdict = .lower) :
    (v.record woke slept).verdict = .lower := by
  unfold Valve.record
  split <;> simp [h]

/-- 任意一串轮之后，降回仍是降回。 -/
theorem lower_stays_lower (v : Valve) (turns : List (Nat × Nat)) (h : v.verdict = .lower) :
    (turns.foldl (fun v t => v.record t.1 t.2) v).verdict = .lower := by
  induction turns generalizing v with
  | nil => simpa using h
  | cons t rest ih => exact ih _ (lower_is_absorbing v t.1 t.2 h)

theorem open_window_keeps_the_verdict (v : Valve) (woke slept : Nat)
    (h : slept - v.opened < v.limit) :
    (v.record woke slept).verdict = v.verdict := by
  simp [Valve.record, h]

/-- 一轮从窗口开出时醒来、到 `limit` 之后才阻塞：忙满了，判降回。 -/
theorem busy_through_the_window_is_lowered (limit now slept : Nat)
    (h : now + limit ≤ slept) :
    ((Valve.new limit now).record now slept).verdict = .lower := by
  have : ¬ slept - now < limit := by omega
  simp [Valve.record, Valve.new, this]
  omega

/-- 一个窗口里只忙一半，不判。 -/
example : (((Valve.new 10 0).record 0 5).record 15 20).verdict = .keep := by decide
example : (((Valve.new 10 0).record 0 9).record 9 10).verdict = .lower := by decide

/-- 人的设置（Rust：`accounting::person::CorePriority`）。 -/
inductive CorePriority where
  | raised
  | normal
  deriving Repr, DecidableEq

/-- 线程为什么留在正常档（Rust：`Held`）。拒绝的原因是平台的话，模型只记它是一次拒绝。 -/
inductive Held where
  | byTheSetting
  | refused
  | byTheValve
  deriving Repr, DecidableEq

/-- 这条线程实际站在哪一档（Rust：`Standing`）。 -/
inductive Standing where
  | raised
  | normal (held : Held)
  deriving Repr, DecidableEq

/-- `raise_this_thread`：`normal` 不调任何平台接口；`raised` 由平台答应或拒绝。 -/
def raiseThisThread (setting : CorePriority) (platformAccepts : Bool) : Standing :=
  match setting with
  | .normal => .normal .byTheSetting
  | .raised => if platformAccepts then .raised else .normal .refused

theorem normal_setting_never_raises (accepts : Bool) :
    raiseThisThread .normal accepts = .normal .byTheSetting := rfl

/-- 降档还欠着，或者平台拒绝过一次（Rust：`Lowering`）。 -/
inductive Lowering where
  | owed
  | refused
  deriving Repr, DecidableEq

/-- 一条核心线程（Rust：`CoreThread`）。 -/
structure CoreThread where
  standing : Standing
  valve : Valve
  lowering : Lowering
  deriving Repr, DecidableEq

/-- 这一轮之后是否调用降档：站在升档上、降档还欠着、阀判了降回。 -/
def CoreThread.asksToLower (t : CoreThread) (woke slept : Nat) : Bool :=
  t.standing == .raised && t.lowering == .owed && (t.valve.record woke slept).verdict == .lower

/-- `record_turn_lowering_when_busy`：`lowerAccepted` 是平台对这一次降档的答复。 -/
def CoreThread.record (t : CoreThread) (woke slept : Nat) (lowerAccepted : Bool) : CoreThread :=
  let valve := t.valve.record woke slept
  if t.asksToLower woke slept then
    if lowerAccepted then { t with valve, standing := .normal .byTheValve }
    else { t with valve, lowering := .refused }
  else { t with valve }

/-- 降回之后不再升，也不再调用降档：站在正常档上的线程不会再请求。 -/
theorem lowered_thread_asks_nothing (t : CoreThread) (held : Held) (woke slept : Nat)
    (h : t.standing = .normal held) : t.asksToLower woke slept = false := by
  simp [CoreThread.asksToLower, h]

/-- 平台拒绝过一次降档，此后每一轮都不再试。 -/
theorem refused_lowering_is_not_retried (t : CoreThread) (woke slept : Nat)
    (h : t.lowering = .refused) : t.asksToLower woke slept = false := by
  simp [CoreThread.asksToLower, h]

/-- 一轮从不把线程升上去：之后站在升档上，之前也站在升档上。 -/
theorem record_never_raises (t : CoreThread) (woke slept : Nat) (accepted : Bool)
    (h : (t.record woke slept accepted).standing = .raised) : t.standing = .raised := by
  unfold CoreThread.record at h
  split at h
  · split at h
    · simp at h
    · simpa using h
  · simpa using h

end Sprawling.Serving.Standing

/-!
## 8-93 核心线程站在正常档之上，空转就降回（`bin::serving::standing`，形状：状态机）

阀与降档必须守住的性质的权威是本文件上面的模型：降回是吸收态、窗口没关上不判、忙满一个窗口即判降回、设置为 `normal` 从不升档、平台拒绝之后不再试；本节是接口、平台的做法与理由。

agent 派出的命令从低于正常的档位起动（`crates/runtime/Spec.lean` §8-13-3）；本节补上另一半：核心自己的线程起动时把自己升到正常档之上一级，于是一台被构建占满的机器上，视图的折叠与广播仍排在派出的命令前面。升在线程一级而不是进程一级：进程档位在 Windows 上要对自身句柄调 `SetPriorityClass`，没有安全接口；线程档位有。

```rust
// bin::serving::standing —— shape: state machine
pub(crate) enum Standing { Raised, Normal(Held) } // 这条线程实际站在哪一档
pub(crate) enum Held { ByTheSetting, Refused(String), ByTheValve }
pub(crate) enum Verdict { Keep, Lower }
pub(crate) const BUSY_LIMIT: Duration;            // 10 s
pub(crate) fn raise_this_thread(setting: CorePriority) -> Standing;
pub(crate) fn lower_this_thread() -> Result<Standing, thread_priority::Error>;
pub(crate) struct Valve;                           // 忙了多久，是否该降回
impl Valve {
    pub(crate) fn new(limit: Duration, now: Instant) -> Self;
    pub(crate) fn record(&mut self, woke: Instant, slept: Instant); // 一次醒来到下一次阻塞
    pub(crate) fn verdict(&self) -> Verdict;
}
pub(crate) struct CoreThread;                      // 一条核心线程的档位与它的阀
impl CoreThread {
    pub(crate) fn raise(name: &'static str, setting: CorePriority, now: Instant) -> Self;
    pub(crate) fn record_turn_lowering_when_busy(&mut self, woke: Instant, slept: Instant);
}
pub fn setting_telling_a_refusal() -> CorePriority; // 读不了就 Normal，并向标准错误说出拒绝
pub fn serving_runtime(setting: CorePriority) -> std::io::Result<tokio::runtime::Runtime>;
pub fn monotonic_now() -> Instant; // 单调钟的唯一取样点；阀量的是时长，墙钟会跳
// accounting::person（`crates/accounting/Spec.lean` §8-8）
pub enum CorePriority { Raised, Normal }                 // 人的设置；Normal 即「关掉高优先级」
pub fn core_priority() -> Result<CorePriority, AxError>; // ConfigInvalid：priority 既不是 "raised" 也不是 "normal"
```

- **升到哪一档**：`thread-priority` 的跨平台值 70，在 Windows 上是 `THREAD_PRIORITY_ABOVE_NORMAL`（正常档进程里基准优先级 9，派出的 `BELOW_NORMAL_PRIORITY_CLASS` 子进程是 6）；降回用 50，即正常档。读回档位的测试（`serving::standing::tests` 里的 `a_raised_core_thread_stands_above_normal` 等）只在 Windows 上编译，Unix 上 70 与 50 各落到哪个 nice 值没有测试读回，要查它得加一条 `cfg(unix)` 的读回测试，并让它在没有 `CAP_SYS_NICE` 的 CI 主机上只断言被拒的那一支。Unix 上升档要 `CAP_SYS_NICE`；没有时操作系统拒绝，线程留在正常档，`Standing::Normal(Held::Refused(原因))` 把原因带回来，`CoreThread::raise` 向标准错误说一次——相对效果由子进程的 `nice` 给出，不靠这一步。
- **可见性与单调钟住在哪里**：`setting_telling_a_refusal`、`serving_runtime` 与 `monotonic_now` 是 `pub`，因为读它们的 `bin::main::city` 与 `bin::main::gauge` 在二进制目标里，那是 `sprawling` 库之外的另一个 crate。单调钟的取样点住在本模块而不在 `bin::assembly`：它计时的核心线程在这里与视图折叠里起动，两处都不认识装配点（`crates/sprawling/spec/Supervising.lean` §8-92）；`bin::assembly` 与 `bin::supervising` 反过来从这里取它。
- **设置**：人的配置文件（`Home::person_config`，即 `config.toml`）里 `[core]` 一节的 `priority`，`"raised"` 或 `"normal"`，缺省为 `"raised"`。`"normal"` 让 `raise_this_thread` 不调任何平台接口，返回 `Standing::Normal(Held::ByTheSetting)`。文件读不了、解析不了或值拼错时，服务照常起动，核心留在正常档，并向标准错误说出拒绝与恢复办法：升档是有全机代价的一方，要有一个说「可以」的读数才做。设置在服务起动时读一次，改了要重启服务。
- **安全阀**：升了档的线程若空转，会拖住整台机器。每条升档线程带一个 `Valve`，每处理完一件事记一次「醒来—再次阻塞」；窗口从上一个窗口关上时开始，第一件结束在窗口开出 `BUSY_LIMIT` 之后的事关上它；关上时忙的时间占到窗口的 15/16 以上，`verdict` 变成 `Lower`，此后一直是 `Lower`。`record_turn_lowering_when_busy` 见到站在升档上的线程得了 `Lower`，就调 `lower_this_thread` 回到正常档，并向标准错误写一行说给人：哪条线程、忙满了多少秒、已降回正常档。平台拒绝降档时同样写一行，说出平台的原因，此后这条线程不再试：两个平台都不拒绝线程给自己降档，所以拒绝是一次缺陷报告，每一轮都重试只会让标准错误每轮多一行，而不会让它降下来。时间作参数传入，`Valve` 本身不读钟，所以它的判定可以用编造的时刻逐点验证。
- **量的是墙钟，不是 CPU 时间**：线程在两次阻塞之间走过的墙钟时间是它占着核的时间的上界；升了档的线程很少被抢占，二者接近。
- **哪些线程**：视图线程 `sprawling-views`（`bin::serving::folding`）起动时升档，每折完、广播完一件记一次。socket 服务所在的 tokio 多线程运行时由 `serving_runtime` 建（`bin::main::city` 用它代替 `Runtime::new`）：每条 worker 第一次被唤醒（`on_thread_unpark`）时升档，此后每次唤醒到下一次停放（`on_thread_park`）记一次，档位与阀放在线程本地。只在唤醒时升，因为只有 worker 会停放与唤醒：阻塞池的线程（`spawn_blocking`）从不唤醒，所以永远不会成为没有阀的升档线程；代价是一条起动后从未停放过的 worker 留在正常档，而这正是安全的一侧。`CoreThread::raise` 同时向放置表要一个座位，座位随 `CoreThread` 析构交还（`crates/sprawling/spec/Serving/Placement.lean`）。
- 证据：`crates/sprawling/src/serving/standing/tests.rs` 的 `a_raised_core_thread_stands_above_normal`（Windows 臂：升档后读回 `AboveNormal`）与 `a_thread_busy_through_the_window_is_lowered`（忙满一个窗口的线程判降回，忙一半的不判）；`a_socket_worker_stands_above_normal_once_it_has_woken`（Windows 臂：`serving_runtime` 的 worker 睡过一次之后读回 `AboveNormal`）；`crates/accounting/src/person.rs` 的 `a_person_who_turns_the_raise_off_gets_normal_priority`。

**决定**

1. 线程档位取第一档「安全 Rust」：`thread-priority`（MIT）对外只给安全接口，内部的 `SetThreadPriority`／`GetThreadPriority` 由它负责，本仓不写 `unsafe`。不取 Zig 叶子：一次只传句柄与常量的调用没有 `(ptr, len)` 边界可以放在 Zig 后面，Rust 侧调用 `extern` 的那一处 `unsafe` 也省不掉。被否：本仓自己写 `unsafe` 调 `SetPriorityClass`，把整个进程升到 `HIGH_PRIORITY_CLASS`——没有测量表明它全局最佳，而且会把 tokio 的每条线程连同它们的空转风险一起升上去。重开参数：线程档升满之后，后台负载占满所有核时 relay 往返 p50 与空闲之比仍大于 1.2。
2. 安全阀量墙钟而不量 CPU 时间：读线程自己的 CPU 时间在 Windows 上是 `GetThreadTimes`，没有第一档的路；在一轮之内，墙钟只会高估忙的程度，所以对已经结束的轮，阀只会判得早，不会判得晚。阀只在一轮结束时判：一条从不停放的 tokio worker（持续有任务时，tokio 的维护性 `park_timeout(0)` 不调停放与唤醒回调）和一次不返回的折叠都没有「轮结束」，所以它们一直留在升档上，阀对它们不起作用。重开参数：出现对外只给安全接口、读线程 CPU 时间的 crate。
3. 降回之后不再升：一条线程忙满过一个窗口，就说明它的工作量不该排在派出的命令前面；反复升降只会让人看到忽快忽慢。
4. 见下面的 D40：harness 进程不受节能限流，三个平台各自的做法。热线程的理想处理器与「一步不跨线程」在 `crates/sprawling/spec/Serving/Placement.lean` 的 D41，四臂对照的测量计划也在那里。

**尚未做到的（本节接口的当前状态）**：阀只在一轮结束时判定（决定 2），所以一条持续有任务、从不停放的 worker 和一次不返回的折叠永远不会被降回，而这正是阀要防的情形；在忙的期间也作判定——tokio worker 按每次任务轮询记（`tokio_unstable` 下的 `on_before_task_poll`／`on_after_task_poll`），或在每次唤醒与任务边界处拿正在进行的一轮已走过的时间比窗口——是这一接口余下的一步。写线程 `sprawling-runs`（记账）还没有升档——它的循环在 `serve_flight` 里面阻塞，循环看不到它醒来的时刻，而没有阀的升档线程正是本节禁止的；把醒来的时刻从 `serve_flight` 交出来之后，它按视图线程的办法升档。Unix 上没有 `CAP_SYS_NICE` 时，每条 worker 各说一次它留在正常档。降回时写的是标准错误，还不是一条类型化的 Ledger 事件（`crates/kernel/spec/Event/Kind.lean` 的事件种类表加一行）。doctor 还不报告每个平台实际站在哪一档。D40 的节能限流调用与 D41 的座位在第一条热线程要座位时一起做（`bin::serving::placement`）；`[core] placement = "none"` 两样都不做（`crates/sprawling/spec/Serving/Placement.lean` D47）；各 lane 线程经 `accounting::worker::hands::Hands` 的 `seat_lane` 在车道开头要座位（同一分部的 D46）。
-/

/-! D40 harness 进程不受节能限流；macOS 与 Linux 照实说各自有什么（AF1 的 (a)，D88 第 3 条，D94）

**决定**：

- **Windows**：harness 起动时对自己的进程调一次 `SetProcessInformation`，信息类 `ProcessPowerThrottling`，结构 `PROCESS_POWER_THROTTLING_STATE { Version: 1, ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED, StateMask: 0 }`——控制位置上、状态位为零，就是「执行速度永不限流」，进程里的每条线程（账本、折叠、socket、各 lane）都随之不受 EcoQoS 限流，Windows 的混合调度于是按「先 P 核、P 核忙了用 E 核」放它们。对自己的进程调这一项不要管理员（D20）。接口档位：`std`、`thread-priority` 3.1.1、`win32job` 2.0.3 都不给这一项；这一调用传的是结构的指针与它的长度，正好是 `(ptr, len)` 边界，所以走第二档 Zig 叶子。叶子放在 `crates/desktop/ffi` 里，与 D41 的处理器表、处理器号和 `crates/runtime/spec/Tools/Exec.lean` D29 的 job 限额同一个叶子：仓库里 `unsafe` 只许出现在那一个 crate（`xtask guard`），另开一个 FFI crate 要动门的机器。派出的子进程不从 harness 继承这一项（推断，待 Windows runner 读回），它们照系统的默认可以被限流，这正是要的。调用在第一条热线程要座位时做（`bin::serving::placement`，与 D41 的座位同一刻），`[core] placement = "none"` 时不做。一台不支持节能限流的 Windows 对这一信息类连读都答 `ERROR_INVALID_FUNCTION`：那里没有可关的东西，`desktop_ffi::cpu::full_speed` 答 `Throttling::Absent`，不算拒绝。别的失败时退路是系统自己决定：harness 照常起动，向标准错误说一次原因。
- **macOS**：没有 EcoQoS。对应物是线程的 QoS 等级：高 QoS 的线程被放到性能核上。热线程取 `QOS_CLASS_USER_INITIATED` 要调 `pthread_set_qos_class_self_np`，`libc` 只给它 `unsafe` 的 `extern` 声明，锁文件里没有对外安全的 crate（`thread-priority` 3.1.1 在 macOS 上只设 pthread 优先级，不设 QoS），而 Zig 叶子今天只在 Windows 上构建（`crates/desktop/Spec.lean` D12），所以本决定不升热线程的 QoS：它们留在缺省 QoS，相对的效果由子进程一侧给出——派出的命令降到 utility（`crates/runtime/spec/Tools/Exec.lean` D29）。App Nap 只作用于有窗口的应用，命令行进程不受它影响，没有东西要关。
- **Linux**：没有对应的按进程节能限流；CPU 调频策略是全机的，改它要 root（D20 不许），所以 (a) 在 Linux 上不可用，也不需要退路：内核不会因为 harness 在后台就对它降频。热线程的档位仍是 §8-93（没有 `CAP_SYS_NICE` 时留在正常档并说一次），子进程的 `nice 10` 与 cgroup 份额在 D29。

**被否**：①不关 EcoQoS，靠 Windows 的默认——Windows 11 会对它判为后台的进程自动施加 EcoQoS（效率模式），而 harness 多数时间正是一个没有前台窗口的进程（这是推断，四臂对照的「不做」臂读出）；②按线程调 `SetThreadInformation(ThreadPowerThrottling)`——每条热线程起动时各调一次，lane 要从 accounting 里调，而进程一级一次调用就盖住全部线程；③本仓自写 `unsafe` 直接调 `SetProcessInformation`——平台调用规则只在测量表明它全局最佳时才许，一次进程级调用没有可测的差别。

**重开参数**：四臂对照里 (a) 单独的贡献在噪声以内（那就删掉这一调用与它的叶子函数）；或 macOS runner 上读出缺省 QoS 的热线程在后台负载下被放到效率核上，并且延迟可见——那时把 Zig 叶子的构建扩到 macOS，热线程升到 `QOS_CLASS_USER_INITIATED`。
-/
