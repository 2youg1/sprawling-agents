-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 性能监视器的历史：有人看才采样，每项 300 点

规定 `crates/sprawling/src/monitor.rs` 的 `Monitor::tick`（`bin::monitor`，形状：状态机；§8-94）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一拍之内只发生一件事：看最多的那个人要什么，就读什么一次，放进历史；没人看就不读，并把历史释放。读数 `Sample` 的类型是参数（它定义在 `wire::frames::monitor`，`crates/wire/Spec.lean` §8-47），读取函数也是参数，所以「没人看不读」在模型里是「读取函数没有被调用」：`tick` 返回它调用读取函数时传的那一类，没调用就是 `none`。

四条性质：没人看时不读、历史为空（§8-94 决定 1）；历史不超过 `capacity`；满了丢最旧的、从旧到新；有人看整页就读整页，只有看摘要的人才只读摘要。
-/

namespace Sprawling.Monitor

/-- 看的人看什么（Rust：`wire::frames::monitor::Watched`）。 -/
inductive Watched where
  | everything
  | summary
  deriving Repr, DecidableEq

/-- 每秒一点，5 分钟（Rust：`CAPACITY`）。 -/
def capacity : Nat := 300

/-- 监视器的状态：两类看的人各自的计数，与历史（从旧到新）。 -/
structure Monitor (Sample : Type) where
  pageWatchers : Nat
  summaryWatchers : Nat
  history : List Sample

variable {Sample : Type}

/-- 看最多的那个人要什么（Rust：`Monitor::watched`）。 -/
def Monitor.watched (m : Monitor Sample) : Option Watched :=
  if m.pageWatchers > 0 then some .everything
  else if m.summaryWatchers > 0 then some .summary
  else none

/-- 满了丢最旧的，再把新的放在最后。 -/
def keep (history : List Sample) (s : Sample) : List Sample :=
  (if history.length = capacity then history.drop 1 else history) ++ [s]

/-- `Monitor::tick`：返回这一拍之后的监视器，与读取函数被调用时收到的那一类（没调用为 `none`）。 -/
def Monitor.tick (m : Monitor Sample) (read : Watched → Sample) :
    Monitor Sample × Option Watched :=
  match m.watched with
  | none => ({ m with history := [] }, none)
  | some w => ({ m with history := keep m.history (read w) }, some w)

/-- §8-94 决定 1：没人看时不读计数器，也不留历史。 -/
theorem unwatched_reads_nothing (m : Monitor Sample) (read : Watched → Sample)
    (h : m.pageWatchers = 0 ∧ m.summaryWatchers = 0) :
    (m.tick read).2 = none ∧ (m.tick read).1.history = [] := by
  simp [Monitor.tick, Monitor.watched, h.1, h.2]

/-- 有人看整页就读整页。 -/
theorem a_page_watcher_reads_everything (m : Monitor Sample) (read : Watched → Sample)
    (h : m.pageWatchers > 0) : (m.tick read).2 = some .everything := by
  simp [Monitor.tick, Monitor.watched, h]

/-- 只有看摘要的人时只读摘要。 -/
theorem only_summary_watchers_read_the_summary (m : Monitor Sample) (read : Watched → Sample)
    (hp : m.pageWatchers = 0) (hs : m.summaryWatchers > 0) :
    (m.tick read).2 = some .summary := by
  simp [Monitor.tick, Monitor.watched, hp, hs]

theorem keep_is_bounded (history : List Sample) (s : Sample)
    (h : history.length ≤ capacity) : (keep history s).length ≤ capacity := by
  unfold keep
  split <;> simp [capacity] at * <;> omega

/-- 历史不超过 `capacity`：每一拍都保住这条。 -/
theorem tick_is_bounded (m : Monitor Sample) (read : Watched → Sample)
    (h : m.history.length ≤ capacity) : (m.tick read).1.history.length ≤ capacity := by
  unfold Monitor.tick
  split
  · simp
  · exact keep_is_bounded _ _ h

/-- 新读数总在最后：历史从旧到新。 -/
theorem newest_is_last (history : List Sample) (s : Sample) :
    (keep history s).getLast? = some s := by
  unfold keep
  split <;> simp

/-- 没满时什么也不丢。 -/
theorem nothing_dropped_below_capacity (history : List Sample) (s : Sample)
    (h : history.length < capacity) : keep history s = history ++ [s] := by
  have : history.length ≠ capacity := by omega
  simp [keep, this]

end Sprawling.Monitor

/-!
## 8-94 性能监视器的历史：有人看才采样，每项 300 点（`bin::monitor`，形状：状态机）

`tick` 必须守住的性质的权威是本文件上面的模型：没人看不读也不留历史，看整页读整页、只有看摘要的人才只读摘要，历史不超过 300 点、新的在最后；本节是接口与理由。

WebUI 的监视页、设置树「性能」条目旁的摘要与 `sprawling gauge --at <地址>` 读的是同一份历史：每秒一个 `Sample`，最近 300 个（5 分钟）。`bin::monitor` 只管两件事：此刻有没有人在看，以及看的人读到的那 300 个点。计数器从哪里读（核心进程、Job Object、整机、城所在的卷、记账线程）由调用方传进来的读取函数决定，本模块不碰平台接口。

**接口。**

- `Sample`：一次读数，全部是 `u64` 的整数计数，没有浮点（它会上线协议，定义在 `wire::frames::monitor`，见 `crates/wire/Spec.lean` §8-47，本模块用的就是那一个类型）。字段：`core_cpu_permille`、`core_private_bytes`、`core_working_set_bytes`、`core_read_bytes`、`core_written_bytes`、`machine_cpu_permille`、`machine_available_bytes`、`volume_free_bytes`、`ledger_queue_depth`、`durable_lag`、`relay_p50_nanos`、`event_to_screen_p50_nanos`、`queued_runs`。
- `Monitor::new()`：不分配。
- `Monitor::watch(&self, watched: Watched) -> Watch`：一个在看的人，看整页（`Watched::Everything`）或只看设置树「性能」条目旁那一行摘要摘要（`Watched::Summary`，`Watched` 定义在 `wire::frames::monitor`）。两类分开计数。`Watch` 被丢弃时这个人就不再算数；它可以跨线程持有（socket 线程持有，采样线程计数）。
- `Monitor::tick(&mut self, read: impl FnOnce(Watched) -> Sample)`：每秒调用一次。有人看整页时以 `Watched::Everything` 调用 `read` 一次；只有看摘要的人时以 `Watched::Summary` 调用，读的一方只读本进程；把结果放进历史，满 300 个时丢掉最旧的。没人在看时不调用 `read`，并释放历史占的内存。
- `Monitor::is_watched(&self) -> bool`：此刻有没有人持有 `Watch`，不论哪一类。
- `Monitor::history(&self) -> impl Iterator<Item = &Sample>`：从最旧到最新。
- 没有失败路径：计数是 `AtomicUsize` 的加减，历史的容量在第一次放入时一次预留。

**定下的值。** `CAPACITY = 300`（每秒一点，5 分钟）；`HISTORY_BUDGET = 64 KiB`，`CAPACITY × size_of::<Sample>()` 超过它时编译失败（13 个 `u64`，现为 31 200 字节）。

**决定。**

1. 没人看时既不读计数器也不留历史。零开销指的是 CPU 与内存两轴：不读就没有系统调用，释放就没有常驻的 31 KiB；代价是重新打开监视页时曲线从空开始。另一种做法是一直采样、页面打开就有 5 分钟的曲线，它让每个没人看的城都多付一份开销，而这正是人要求避免的。重新考虑的条件：人要求打开页面就看到过去 5 分钟。
2. 读取函数由调用方传入，而不是一个 trait。现在只有一种读法（生产的平台计数器）；测试传一个计数的闭包即可验证「没人看不读」，不必为一个没有第二实现的接缝造 trait。
3. 历史是一个 `VecDeque`，第一次放入时 `reserve_exact(CAPACITY)`，之后不再分配。

**测试。** `monitor::tests`：没人看时 `read` 一次也不被调用、历史为空，人走了以后历史被释放；放入 301 个点后只剩最后 300 个、从旧到新；只有看摘要的人时 `read` 收到 `Summary`，再来一个看整页的人时收到 `Everything`，他走了以后回到 `Summary`。

**本节接口的当前状态。** 计数器读取与每秒一拍的采样线程见 §8-96；线上的一对监视帧见 `crates/wire/Spec.lean` §8-47：会话发 `Watch` 时经 `assembly::listening` 交给它的 `watch` 在这里的 `Monitor` 上计一个看的人，发 `Release` 或断开时不再计。WebUI 监视页 `client/src/views/monitor.svelte` 在 `#/monitor`，经这对帧打开时计一个看的人、关闭时释放，读数与曲线由 `client/src/core/monitor.ts` 按 §8-95 的规则算出。`sprawling gauge --at <地址>` 经这对帧看监视器，见 §8-97。性能摘要（设置树「性能」条目旁，`watching.ts` 的 `watchSummary`）经 `WatchSummary` 看监视器，只显示本进程的 CPU 与工作集；只有它在看时采样线程只读 `OwnProcess`，不打开 `sysinfo`。其余尚未落地：采样一次 ≤ 50 µs、占 CPU ≤ 0.1% 的仪表。

**内存的读数**（`bin::monitor::memory`）：整机物理内存与可用内存经 `sysinfo`（只开 `system` 特性，只刷新 RAM）一次读出成 `Memory { physical, available }`，这是城里读内存的唯一一处；`read` 刷新本线程留着的一个 `System`，不每次新建：同一台 Windows x86-64 桌面级机器、测试档构建上，新建句柄时一次读数 2.2–2.3 µs，留着句柄时 1.6–1.7 µs，余下的是平台调用本身；按线程留而不是全局一把锁，读数的线程之间不互等；计划推进按它决定下一行是否排队（§8-46-3，只管计划行；其他入口的现状见那里）。平台不报时两项都是零，此时不算紧。取 `sysinfo`，因为它是对外只给安全接口的现成路，本 crate 不写 `unsafe`。
-/

/-!
## 8-95 一座城的读数：一行 JSON 与一屏曲线（`bin::monitor::top`，形状：projection）

`sprawling gauge --at <地址>`（别名 `top`，§8-129-4）读 §8-94 的历史，按 stdout 那头是谁（`bin::audience`）选一种输出。本模块只把历史投影成文本，不碰终端、不碰 socket：判断 stdout 那头是谁、每秒重画一次、从城里取历史，都是调用方的事。

**接口。**

- `json_line(sample: &Sample) -> serde_json::Result<String>`：一个 JSON 对象，键就是 `Sample` 的 13 个字段名，值是整数；不含换行。stdout 不是终端时每秒打印的那一行由 `gauge::lines::city_line` 写出：同样的键，前面多一个 `"line":"city"`（§8-129-4）。agent 按行读，一行一个完整的读数。
- `sparkline(values: impl IntoIterator<Item = u64>, width: usize) -> String`：终端画面里一项计数器的曲线。只取最后 `width` 个值，每个值一个字符，从 `▁` 到 `█` 共 8 级，按这几个值自己的最小值到最大值线性分级（整数运算，最小值画 `▁`，最大值画 `█`）；全部相等时整条画 `▁`。值不足 `width` 个时曲线就短一些，不补空白。
- 失败：`sparkline` 没有失败路径。`json_line` 只转交 `serde_json` 的错误；13 个整数字段没有可被拒绝的内容，所以它实际上不会出现，调用方把它当作写 stdout 失败处理，而不是在这里用一个隐藏的 `unwrap` 吞掉。
- `screen(samples: &[Sample], curve_width: usize) -> String`：终端画面的一屏（不含清屏与光标控制，那是调用方的事）。每个计数器一行，共 13 行，以 `
` 分隔，顺序与 `Sample` 的字段相同：左对齐 24 列的英文标签，右对齐 10 列的最新读数，两个空格，再是这一项最近 `curve_width` 个点的曲线。没有样本时返回空串，调用方在第一秒什么也不画。
- `Unit::{Permille, Bytes, Nanos, Count}` 与 `Unit::reading(&self, value: u64) -> String`：一个读数按单位写成文字，是城里单位换算的唯一一处；`gauge::lines` 写给人的行也调它（§8-129-4）。读数的写法按单位定：千分比写成一位小数的百分数（`123` → `12.3%`）；字节按 1024 进位取最大的、读数不小于 1 的单位，写一位小数（`B` 只写整数，其后是 `KiB`、`MiB`、`GiB`、`TiB`）；纳秒按时长的显示规则写：不到 1 µs 写整数 `ns`，不到 10 ms 写整数 `µs`，10 ms 及以上写一位小数的 `ms`，没有更大的单位（`crates/sprawling/spec/Main.lean` §8-129-2 *单位*）；计数原样写。小数一律截断而不是四舍五入，整数运算，没有浮点。

**定下的值。** 8 级字符 `▁▂▃▄▅▆▇█`（U+2581–U+2588），与常见终端字体都有的块元素一致。标签列宽 24、读数列宽 10：最长的标签 `event to screen p50` 与最长的读数 `1023.9 GiB` 都放得下。

**决定。**

1. 曲线按窗口内自己的最小值到最大值分级，而不是从 0 分级。监视器看的是变化：一个 3.1 GiB 到 3.2 GiB 之间抖动的工作集，从 0 分级会画成一条平线。代价是两行曲线的高度不能互相比较，读数本身印在曲线旁边。
2. JSON 用 `serde_json` 按字段名写出，而不是手拼字符串。`serde_json` 已是本 crate 的依赖，字段名与 `Sample` 同处一地，手拼会让键名在两处各写一遍。
3. 一屏是一个纯函数返回的字符串，而不是直接写终端的绘制器。交互（按键、窗口尺寸、重画节奏）留在调用方，这一屏的内容才能用整串比较来测。
4. 单位换算写在这里，不借 `runtime::sieve` 的 `size`：那一个只到 `KiB`、属于另一个 crate 的私有实现，而这里还要换算纳秒。
5. WebUI 面板在浏览器里按同样的规则自己算读数与曲线（`client/src/core/monitor.ts`），而不是让城把画好的行随帧发过去。曲线取多少个点取决于面板在屏幕上有多宽，只有浏览器知道；标签要从 `lang.json` 取两种语言，终端这一侧只有英文。代价是分级与单位换算在 Rust 与 TypeScript 各写一遍，两边由同一组样本对照：`client/src/core/monitor.test.ts` 用的样本与期望读数和 `monitor::top::tests` 的一屏测试逐项相同，改规则时两份测试的期望一起改；没有机器门把这两份期望绑在一起。重新考虑的条件：监视帧改为携带已画好的行，或者面板改用非字符的画法。

**测试。** `monitor::top::tests`：一行 JSON 解析回来正好是 13 个键、值等于读数、不含换行；0 到 7 画成 `▁▂▃▄▅▆▇█`，宽度不足时只画最新的几个，全部相等时画 `▁`；两份样本的一屏逐行等于预期的 13 行，没有样本时为空。
-/

/-!
## 8-96 采样线程与计数器读取（`bin::monitor::sampler`、`bin::monitor::counters`，形状：状态机 / adapter）

§8-94 的 `Monitor` 只回答「有没有人在看」与「看到了哪 300 个点」；每秒调用一次 `tick`、把新读数发给看的会话、以及读数从哪个平台接口来，是这两个模块的事。

**接口。**

- `sampler::beat(monitor: &Mutex<Monitor>, samples: &broadcast::Sender<Sample>, read: impl FnOnce() -> Sample)`：一拍。调用 `Monitor::tick(read)`；这一拍读了计数器，就把这一个读数发到 `samples`（即 `wire::MonitorFeed::samples`）。没人在看时 `read` 不被调用，什么也不发。发送时一个订阅者也没有不是失败：看的会话在两拍之间走了，它没有错过自己要的东西。锁中毒时照常取用：计数是原子的，历史是完整的 `VecDeque`，中毒不留下写了一半的状态。
- `sampler::spawn_sampler(monitor: Weak<Mutex<Monitor>>, samples: broadcast::Sender<Sample>) -> Result<(), AxError>`：起名为 `sprawling-monitor` 的线程，每秒一拍，读数来自 `counters::Counters`。线程只持 `Weak`：`ServeConfig` 连同 `MonitorFeed::watch` 被丢弃后 `upgrade` 失败，线程在下一拍结束。起不了线程时返回 `StorageFatal`，recovery 是检查进程的线程上限（与 `serving::folding` 相同）。
- `counters::Counters::open(volume: PathBuf) -> Counters` 与 `Counters::read(&mut self, watched: Watched, elapsed: Duration) -> Sample`：核心进程的五项取自 `OwnProcess`；`Watched::Everything` 时再读整机 CPU（千分比）、可用内存与城所在卷的剩余空间。城的路径在 `open` 时经 §8-116 的 `volume::resolved` 解析一次，之后每拍用解析过的路径找盘：城以相对路径或 verbatim 拼写起来时，没有一个挂载点是原样路径的前缀，卷的剩余空间就一直读作 0。解析不出时退回原样路径。`sysinfo` 的句柄在第一次 `Everything` 读数时才打开，一次 `Summary` 读数把它们丢掉，所以只有摘要在看时它们不常驻。第一次读数没有上一次可比，两项 CPU 为 0。
- `counters::own_process::OwnProcess::new() -> OwnProcess` 与 `OwnProcess::read(&mut self, elapsed: Duration) -> OwnReading`：只问本进程、不遍历进程表的读数，`elapsed` 是距上一次读数的墙钟时间。`OwnReading { cpu_permille, private_bytes, working_set_bytes, read_bytes, written_bytes }`：CPU 是两次读数之间本进程累计 CPU 时间的增量除以墙钟增量与核数（千分比，整数运算，截到 `0..=1000`），第一次为 0；private 在 Windows 上是 PagefileUsage（即 PrivateUsage），其他平台是虚拟内存；工作集是驻留内存；读写字节是本进程累计经存储读写的字节数，Linux 上取自 `/proc/self/io` 的 `read_bytes` 与 `write_bytes`（std 读文件，不需要 unsafe），其他平台读作 0（决定 1）。平台拒绝某一项时这一项读作 0，与尚未接入的项同样处理：`Sample` 是给人看的读数，没有携带失败的位置，而一秒后下一拍会再读一次。`Counters` 只在有人看时存在：`beat` 之后历史为空（没人看）时采样线程丢掉它，平台句柄与进程表不常驻。

**定下的值。** 一拍的间隔 1 s（§8-94 的「每秒一点」）；线程名 `sprawling-monitor`。

**决定。**

1. 本进程的读数来自 `memory-stats`（工作集与 private）与 `cpu-time`（本进程 CPU 时间），整机与卷的读数来自 `sysinfo`，关掉默认特性、只开 `system` 与 `disk`。本工作区 `unsafe_code = forbid`，「只取需要的几个接口」在这里只能是另一个把平台调用包成安全接口的 crate，所以比较的是同一个 crate 的两种裁剪，以一个空的 release 探针（`lto`、`codegen-units = 1`、`strip`）实测：在 windows-msvc 的桌面级机器上（同时有别的编译在跑），空探针 123 904 字节，读齐本节这些计数器的探针 201 216 字节，多 77 312 字节（约 75.5 KiB）。启动时间不受影响：`Counters` 只在第一个人开始看时才打开，城的启动路径上没有它；打开一次（`System::new`、首次刷新进程与 CPU、列出磁盘）热缓存 0.42–0.56 s、冷缓存 2.8 s；之后每读一次 18–74 ms，其中本进程的刷新占 17–65 ms（`sysinfo` 在 Windows 上即使只问一个 pid 也遍历整张进程表），整机 CPU 0.7–8 ms，磁盘 0.2–0.8 ms，内存约 5 µs。本进程那一段因此换成只问本进程句柄的 `memory-stats` 加 `cpu-time`：同一类机器上的 release 探针读一次约 1.1 µs，比空探针多 1 024 字节，两者依赖的 `winapi` 与 `windows-sys` 已在依赖树里。代价是本进程的累计读写字节：没有找到以安全接口只读本进程 I/O 计数的 crate。Linux 上 `/proc/self/io` 是一个普通文件，std 读它即可；Windows 的 `GetProcessIoCounters` 只能经 unsafe 调用，所以在出现包好它的安全 crate 之前这两项在 Windows 与其他平台上读作 0，而不是为它们留下每拍 17–65 ms 的整表刷新。默认特性的 `sysinfo` 在 LTO 之后并不更大（没用到的代码被去掉），裁掉特性省的是编译时间与依赖数。重新考虑的条件：出现以安全接口只读本进程 I/O 计数或整机 CPU、比 `sysinfo` 显著更快的 crate，或者 `unsafe_code` 的政策改变。
2. 采样放在一条自己的线程上，而不是 tokio 任务：它每秒做一次阻塞的系统调用，放进异步运行时会占住一个工作线程；它与 `serving::folding` 一样是一条命名线程。没人看时线程每秒醒一次、读一个原子数，不读计数器也不留内存（§8-94 决定 1）。
3. CPU 份额的墙钟分母由调用方以 `elapsed` 传入，采样线程传一拍的名义间隔 `BEAT`，而不是在这里读 `Instant::now`：取时间的地方只有 `bin::assembly`（clippy 的 `disallowed_methods` 守着），测试也因此能给出确定的分母。代价是 `sleep` 睡过头的那几毫秒让份额偏高同样的比例（1 s 里多睡 15 ms 就偏高 1.5%），对一条给人看的曲线可以忽略。重新考虑的条件：监视器的读数进入任何决定。
4. 读数经 `Sample` 发出，不在这里换单位：换单位是 §8-95 与 `client/src/core/monitor.ts` 的事。
5. `f32` 的整机 CPU 负载是 `sysinfo` 唯一给出的形式，先截到 `0..=100` 再换成千分比；这一处 `as` 以 `#[expect]` 注明，它是本模块唯一的浮点。

**测试。** `monitor::sampler::tests`：有人看时一拍把读到的那一个读数发给订阅者；没人看时不读、不发。`monitor::counters::tests`：读本进程得到非零的工作集、整机可用内存与卷剩余空间；以相对路径 `.` 打开的 `Counters` 读到的卷剩余空间非零。`monitor::counters::own_process::tests`：第一次读数的 CPU 为 0，本进程忙过一段之后第二次读数的 CPU 大于 0，工作集与 private 非零。

**本节接口的当前状态。** 整机可用内存经 §8-94 的 `bin::monitor::memory` 读出，与计划推进的内存闸（§8-46-3）读同一处。核心健康里记账队列深度与持久水位线的两项经 §8-98 的 `Health` 读出；其余几项现为 0，缺的是来源而不是采样，§8-98 的当前状态逐项写明。派出的命令按 run 装进各自的 Job Object，`Backlog::processes`（`runtime::backlog::jobs`） 给出每个 run 此刻的进程（`crates/runtime/Spec.lean` §8-13-3）；按这些 pid 读每个进程的内存与 CPU、并把逐 run 的明细送上线，还没有做：`Sample` 是一行固定的 13 个数，逐 run 的明细要一种新的帧（WIRE_V 加一）。磁盘延迟没有字段。本进程的累计读写字节在 Linux 以外读作 0（决定 1）。一拍里剩下的大头是 `sysinfo` 的整机 CPU（0.7–8 ms）与磁盘（0.2–0.8 ms），离「采样一次 ≤ 50 µs」还差这两项；采样一次 ≤ 50 µs、占 CPU ≤ 0.1% 的仪表尚未落地。
-/

/-!
## 8-116 城所在卷快满时不接新活（`bin::monitor::volume`，形状：adapter；`accounting::worker::commanding::shedding`，形状：decision）

`crates/kernel/spec/Degradation.lean` §8-74 的 `degradation::admit_work` 判定卷低于地板时不接新活；本节给它生产的读数和生产的入口。读数不取监视器的 `Sample`：监视器只在有人看时采样（§8-94 决定 1），而不接新活不能取决于此刻有没有人开着监视页。

**接口。**

- `volume::space(disks: &sysinfo::Disks, city: &Path) -> Option<kernel::degradation::VolumeSpace>`：挂载点是城路径最长前缀的那块盘的剩余空间与总容量；没有一块盘的挂载点是它的前缀时为 `None`。「城在哪块盘上」只有这一个家：§8-92 的 `Counters` 读卷的剩余空间也经它。
- `volume::resolved(city: &Path) -> Option<PathBuf>`：先以 `std::fs::canonicalize` 把城的路径解析成真实路径（跟随符号链接与 junction），失败时退回 `std::path::absolute`；在 Windows 上再把 verbatim 盘符前缀 `\\?\C:` 还原成普通盘符前缀 `C:`。「城的路径拿什么拼写去比挂载点」只有这一个家，入口与监视器的 `Counters` 都经它。
- `volume::read(city: &Path) -> Option<kernel::degradation::VolumeSpace>`：经 `resolved` 解析城的路径，然后列出一次盘、调用 `space`。生产的入口经它读卷。不解析链接，指向另一块盘的城会读到放链接的那块盘；不补全相对路径或不还原 verbatim 前缀，没有一个挂载点是它的前缀，读作 `None`，盘满时照常接活。
- `RunWorker` 持有一个 `fn(&Path) -> Option<VolumeSpace>` 的读卷函数，生产时是 `volume::read`。人发来的 `Dispatch` 在命名房间、写下任何东西之前读一次卷并调用 `admit_work`；拒绝时回 `BackpressureShed`，subject 是城的根目录，recovery 给出至少要腾出的字节数（`Recovery::FreeDiskSpace`）。读不到卷（`None`）时照常接活：读不到不等于盘满，拒活要有读数作依据。

**决定。**

1. 在入口读卷，而不是由一条常驻线程每秒读一次再把最近的读数放在入口：受理新活是人一次次发来的，一次读卷（`sysinfo` 列盘加刷新）远小于随后一次模型调用；常驻读取让每座没人发活的城都付一份开销。代价是 `sysinfo` 只会列出全部的盘，一次读卷也问到与城无关的盘；只问城所在的那一块，要一个以安全接口给出单个路径剩余空间与容量的 crate，它还不在依赖树里。重新考虑的条件：实测一次读卷超过一次 `Dispatch` 在同一台机器上花掉时间的 10%，或者这样的 crate 进了依赖树，或者有一块与城无关的盘（网络盘、休眠的外置盘）让列盘停住。
2. 读卷函数是函数指针，不是 trait：生产只有一种读法，测试给一个返回低剩余空间的函数即可注入「盘快满」。

**测试。** `accounting::worker::commanding::tests::shedding`：读卷函数报告卷低于地板时，`Dispatch` 被拒为 `BackpressureShed`，recovery 说出要腾出的字节数，房间没有被建起来。`monitor::volume::tests`：以相对路径 `.` 读卷、以 `canonicalize` 给出的 verbatim 拼写读卷，都与以工作目录的绝对路径读到同一块盘。

**本节接口的当前状态。** `Wake` 等不经人的入口尚未接入；性能摘要与 doctor 尚不显示降级；盘慢、内存紧、CPU 被占满三种状态还没有生产的读数（`crates/kernel/spec/Degradation.lean` §8-74）。
-/
