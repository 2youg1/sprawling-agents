-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 吞吐台（TP1）

规定吞吐台场景：N 个 run 同时在一座真的城里跑，读出吞吐与每一处等待。本文件是 `tools/citysim/Spec.lean` 的一个分部，节标签 §8-14。场景的执行者是 `sprawling-accounting` 的仪表 `instrument_throughput` 与它的计数对拍 `throughput_counts`（`crates/accounting/src/worker/driving/tests/throughput.rs`），理由见 D23。

能写成定理的是读数的尾部规则：p999 只在样本够多时印出，不够时印 max 并写明样本不足（`a_p999_is_printed_only_over_enough_samples`、`under_the_floor_the_max_is_printed`）；印出的尾部总是某一个真实样本（`the_printed_tail_is_a_sample`）。挂钟读数本身不是任何定理的对象。
-/

/-!
### 8-14 吞吐台场景（`instrument_throughput`）

**负载形。** 一座城，一栋楼 `lab`，楼里 N 个房间 `lab/r<i>`，N ∈ {1, 4, 16, 64}。每个房间同时派一个 run（`CommandDesk::post` 的 `Dispatch`，一次全部投出）。每个 run 的脚本是 M 次工具调用再作结，读写交替：奇数次是 `status`（只读，无对外效果），偶数次是 `edit` 新建一个本 run 独有的文件（写，触发写入波前的检查点）。M 是 `TOOL_CALLS`，默认 4。

**模型延迟三档**（`Latency`）：`zero`（替身立刻作答）、`fixed50`（每次模型调用前替身睡 50 ms）、`measured`（从测试城账本折出的 `model_called` 到 `model_returned` 的延迟分布，只存折出的分位数，按 run 与调用的位次确定地取值，不用随机源）。分布住 `MEASURED_LATENCY_MS`，一个常量；换一座城的读数就是改这个常量。

**读数**（一次运行一组，每组一行吞吐、每处等待一行）：

`throughput n_runs=<N> latency=<arm> tool_calls_per_run=<M> runs=<n> records=<n> tool_calls=<n> wall_us=<n> runs_per_s=<x> records_per_s=<x> tool_calls_per_s=<x> ledger_thread_idle_us=<n> ledger_thread_busy_permille=<n> machine=<os>-<arch>, <k> core(s)`

吞吐行的 `ledger_thread_busy_permille` 是账本线程在这段墙钟里没睡的份额（千分比），由它在队列上睡掉的时间（`ledger_thread_idle_us`）减出。

`throughput_wait n_runs=<N> latency=<arm> wait=<name> n=<n> p50_us=<n> p99_us=<n> p999_us=<n>|p999=insufficient max_us=<n>`

吞吐行的 `*_per_s` 是整数部分加三位小数的定点数，由整数运算得出（ledger 载荷之外，但读数仍不经浮点）。尾部规则见下面的 `tail`：n ≥ `P999_FLOOR`（1,000）才印 `p999_us`，否则印 `p999=insufficient`；`max_us` 总是印。分位取最近秩，第 ⌈n·p⌉ 个。

**读到的等待**（M0 第 6 条把每个耗时拆成等待、工作、落盘屏障；下表写明每一行属于哪一段，以及它从哪里读出）：

| `wait=` | 量的是什么 | 段 | 读法 |
|---|---|---|---|
| `lane` | 一个 run 从投出到 `run_started` 落账 | 等待（含准备的工作） | 账本时刻（毫秒）减投出时刻，×1000 记成 µs；毫秒分辨率，M1 落地后换成微秒字段 |
| `first_call` | `run_started` 到第一次 `model_called` | 工作（放置、准备） | 账本时刻差 |
| `relay_under_load` | 负载期间，仪表经 `measuring_relay` 发出的一次 relay 往返 | 等待＋工作＋屏障 | 单调时钟，测试侧；减去空载的 `relay_idle` 就是排队的份额 |
| `relay_queue` | 一次 relay 请求从车道把它排进队到账本线程取走 | 纯等待 | worker 注入的单调时钟，在生产代码的测量点上取（`worker::health`） |
| `lane_pure` | 一个准备好的 run 从进队到 driving pool 为它开车道 | 纯等待 | 同上；不设车道数时恒为零或近零（`budgets.toml` 的 `[lane_wait]`） |
| `relay_idle` | 同样的往返，城里没有 run | 工作＋屏障 | 单调时钟，测试侧，与 `relay_under_load` 同 |

`lane` 一行不含车道排队：pool 不设车道数（`crates/sprawling/Spec.lean` D34），投出的 run 立刻开车，N 再大也不等前面的 run 回家。一个 run 只在 provider 的准入处等，那是 `gateway::concurrency` 按端点给的并发许可，遇 429 收窄、持续应答后放宽（`crates/gateway/Spec.lean` D17）；替身 provider 不回 429，所以本台读到的 `lane` 是准备的工作，不是排队。

**本台之外的等待**：折叠滞后与广播滞后由 `crates/sprawling` 的 `instrument_view_backlog` 读（`budgets.toml` 的 `[fold_lag]`、`[broadcast_lag]`）；私有内存节拍还没有读数的计算者（`[private_memory_beat]` 的 `measured_by` 写着缺什么）。

**计数对拍**（`throughput_counts`，在 `just check` 里跑）：N = 4、`zero` 档、M = 4 时，`run_frozen` 恰 N 条，`tool_called` 恰 N·M 条，每个 `tool_called` 都有它的 `tool_result`。计数只取决于脚本，不取决于时钟，所以三个平台的 CI 都能跑它。

**平台**：场景与等待的读法不碰平台接口，Windows、macOS、Linux 上一样跑；`relay_under_load` 的屏障份额随平台的落盘调用不同（TF2），读数照实报告。

D23 **吞吐台住在 `sprawling-accounting` 的仪表里，本规格只规定它。** 吞吐路径（pool、lane、relay、账本线程的一次 serve）都是 `accounting::worker` 里 `pub(in crate::worker)` 的件，驱动它们的替身 provider 与造城的夹具也只在该 crate 的测试构型里编译；本 crate 够不到，与 D18 是同一个理由。被否：给 `accounting` 开一扇公共门让 citysim 驱动一座城的 pool——那扇门没有生产调用者。**重开参数**：citysim 的替身 provider（§8-10）能当一座真城的 provider、且 `sprawling` 的二进制能被场景从外面拉起并派活时，场景搬到 citysim 的 bench，从进程外量。

D24 **这一段的等待从账本与仪表外侧读，生产代码不加测量点。** 外侧读数立刻可得、不改 worker 的任何行为；它分不出「等待」与「工作」时，表里写明它是哪几段的和。被否：在本 wave 给 pool 与 relay 加时刻戳——那要改 `crates/sprawling/spec/Accounting/Worker.lean`，该文件本 wave 归别的 lane。**重开参数**：W2 接手 M2 的生产测量点时，`lane` 与 `relay_under_load` 两行各拆成纯等待与工作两行，行名不变，多一个 `seg=` 字段。
-/

namespace Citysim.Throughput

/-- 一行等待读数的尾部：够多样本时是 p999，不够时是 max 加「样本不足」。 -/
inductive Tail where
  | p999 (value : Nat)
  | insufficient (max : Nat)
  deriving DecidableEq, Repr

/-- 最近秩：n 个样本里第 ⌈n·999/1000⌉ 个，下标从 0 起所以减一。 -/
def rank999 (n : Nat) : Nat := (n * 999 + 999) / 1000 - 1

/-- 一组升序样本的尾部；`floor` 是印 p999 所需的最少样本数（Rust 的 `P999_FLOOR`）。没有样本就没有尾部。 -/
def tail (floor : Nat) (sorted : List Nat) : Option Tail :=
  match sorted.getLast? with
  | none => none
  | some biggest =>
    if floor ≤ sorted.length then
      some (.p999 (sorted.getD (rank999 sorted.length) biggest))
    else
      some (.insufficient biggest)

/-- 印出 p999 的读数，样本数一定到了下限。 -/
theorem a_p999_is_printed_only_over_enough_samples (floor : Nat) (sorted : List Nat) (v : Nat)
    (printed : tail floor sorted = some (.p999 v)) : floor ≤ sorted.length := by
  unfold tail at printed
  split at printed
  · simp at printed
  · split at printed
    · assumption
    · simp at printed

/-- 样本不够下限时印的是最大的那个样本，并且标成样本不足。 -/
theorem under_the_floor_the_max_is_printed (floor : Nat) (sorted : List Nat) (biggest : Nat)
    (last : sorted.getLast? = some biggest) (few : sorted.length < floor) :
    tail floor sorted = some (.insufficient biggest) := by
  unfold tail
  rw [last]
  simp [Nat.not_le.mpr few]

/-- 秩落在样本之内：n ≥ 1 时 ⌈n·999/1000⌉ - 1 < n。 -/
theorem rank999_lt (n : Nat) (positive : 0 < n) : rank999 n < n := by
  unfold rank999
  omega

/-- 印出的尾部总是一个真实样本，而不是插值或 0。 -/
theorem the_printed_tail_is_a_sample (floor : Nat) (sorted : List Nat) (t : Tail)
    (printed : tail floor sorted = some t) :
    match t with
    | .p999 v => v ∈ sorted
    | .insufficient m => m ∈ sorted := by
  unfold tail at printed
  split at printed
  · simp at printed
  · rename_i biggest last
    have member : biggest ∈ sorted := List.mem_of_getLast? last
    have nonempty : 0 < sorted.length := List.length_pos_of_mem member
    split at printed
    · simp only [Option.some.injEq] at printed
      subst printed
      simp only [List.getD_eq_getElem?_getD]
      rw [List.getElem?_eq_getElem (rank999_lt _ nonempty)]
      exact List.getElem_mem _
    · simp only [Option.some.injEq] at printed
      subst printed
      exact member

/-- 正常路径可达：样本到了下限就印 p999。 -/
example : tail 2 [3, 7] = some (.p999 7) := by decide

end Citysim.Throughput
