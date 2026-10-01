-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::long_turn

规定 `citysim::long_turn`（`tools/citysim/src/long_turn.rs`）与它的进程 `bin/long_turn`（`tools/citysim/src/bin/long_turn.rs`）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面一节保留它在 citysim 规格里的标签 §8-9，别处引作 `tools/citysim/Spec.lean §8-9`。

能写成定理的是门为什么咬得住（D10）：每次模型调用的窗口是第一次的窗口加上此前每一步加进对话的字节，所以相邻两次窗口之差恰是那一步加进去的东西（`the_increments_are_what_each_step_added`）；每一步加的一样多时窗口是一条等差数列，有一步多加了东西，增量就不再处处相同（`a_step_that_adds_more_shows_as_an_uneven_increment`）。runtime 在哪里算窗口（`prompt_shape_compared` 的 `upper_bound`）是 runtime 的事。
-/

/-!
### 8-9 长回合（`citysim::long_turn`，`bin/long_turn`）

```rust
pub const STEP_BYTES: usize = 1024;   // 每一步读到的文件长度，每步内容不同
pub enum Pauses { Never, Every { steps: u32, at: Box<dyn Fn(u32) -> Result<(), AxError> + Send + Sync> } }
pub struct TurnReading { pub completion: &'static str, pub windows: Vec<u64>, pub ledger_bytes: u64 }
pub fn long_turn(steps: u32, pauses: Pauses) -> Result<TurnReading, AxError>;
```

- 一个 run：`steps` 次模型回复，每次说一句话、要一次 `read`；第 `steps + 1` 次回复作结。剧本经 `ScriptModel::new` 给出；`read` 是本模块的工具，按调用次数生成 `STEP_BYTES` 字节的内容（前缀是步数，其余是按步数轮转的字母），所以内容每步都变、长度不变，也没有随机源。
- `windows` 是每次模型调用的 `upper_bound`，按调用序；`completion` 是冻结时的结局名；`ledger_bytes` 是模拟账本此刻的字节数。
- `Pauses::Every` 在第 `steps` 的每个整数倍次 `read` 之后调 `at(k)`：`bin/long_turn` 在那里打印一行并等一行输入（D10），测试传 `Pauses::Never`。
- `bin/long_turn <steps> [every]`：缺省每 100 步停一次；跑完打印 `done <completion> steps <n> window <bytes> ledger <bytes>`。
- 测试：`long_turn::tests` 的 `a_long_turn_grows_its_window_by_one_step_at_a_time` 在 40 步与 80 步上断言结局、调用次数与增量处处相同。
-/

namespace Citysim.LongTurn

/-- 第一次调用的窗口是 `first`，此后第 k 步往对话里加进 `added[k]` 个字节：每次调用的窗口，按调用序。 -/
def windows (first : Nat) : List Nat → List Nat
  | [] => [first]
  | step :: rest => first :: windows (first + step) rest

/-- 相邻两次窗口之差，按调用序（`a_long_turn_grows_its_window_by_one_step_at_a_time` 读的就是这一串）。 -/
def increments : List Nat → List Nat
  | first :: second :: rest => (second - first) :: increments (second :: rest)
  | _ => []

/-- 窗口的增量恰是每一步加进对话的字节：门读到的就是每一步做了什么，与机器、分配器、扫描器的噪声都无关。 -/
theorem the_increments_are_what_each_step_added (first : Nat) (added : List Nat) :
    increments (windows first added) = added := by
  induction added generalizing first with
  | nil => rfl
  | cons step rest ih =>
    cases rest with
    | nil => simp [windows, increments]
    | cons next more =>
      have := ih (first + step)
      simp only [windows] at this ⊢
      simp only [increments, Nat.add_sub_cancel_left]
      rw [← this]

/-- 每一步加的一样多时，窗口的增量处处相同：这一跑是绿的。 -/
theorem alike_steps_grow_the_window_evenly (first step count : Nat) :
    increments (windows first (List.replicate count step)) = List.replicate count step :=
  the_increments_are_what_each_step_added first _

/-- D10 **长回合的门是请求窗口的逐步增量，RSS 只作读数。** 一步把更早的内容再加一遍，或者请求里多留了每一步的副本，那一步加进去的就比别的步多，增量不再处处相同，测试变红，不依赖机器，也不依赖扫描器的噪声。

fx 的一次修复之前，一个长回合每一步都留着整份恢复重建与请求的临时分配，482 步后 `OutOfMemory`，峰值 RSS 1433.7 MiB；修复之后 1000 步峰值 82.2 MiB。sprawling 的内存读数只有服务中空闲的一行，长回合没有量具。`citysim::long_turn` 照那个夹具的形状跑一个 run：每一步模型要一次 `read`，读到的文件每步都变、长度不变。每次模型调用前的 `prompt_shape_compared` 记下这次请求最多按多少字节计入输入（`upper_bound`，runtime 在同一处算它与请求本身）；每一步加进对话的东西一样长，所以第 k 次调用的窗口恰是第一次的窗口加上 k-1 个相同的增量。

**RSS 是读数，不是门。** 进程的计数器随机器与分配器变。`just mem long-turn <steps>` 构建 release 的 `long_turn`，它每走完 100 步停一次，打印一行 `step <k> pid <pid>`，等一行输入再走；配方在每次停顿时调 `cargo xtask mem <pid>`——进程计数器的唯一读者（`tools/xtask/Spec.lean` §8-30）——再放它走。最后一次停顿的 peak private 就是整个 run 的峰值；相邻两次的 private 之差除以步数，就是每步的增长。模拟器自己的 `MemLedger` 把每一行都留在内存里；跑完的那一行 `done <completion> steps <n> window <bytes> ledger <bytes>` 给出它的字节数与最后一次请求的窗口，每一步的账本行一样长，读者按步数把它从每次停顿的 private 里减掉。峰值的上限由整合者按读数登记进 `tools/xtask/budgets.toml`，写 MiB，照 fx 的做法是 500 步的峰值上限。本配方不带 `product_features`：`long_turn` 直接驱动 `runtime::run::drive`，被量的回合路径与 `sandbox` feature 无关，带上它只会把用不到的执行引擎编进被量的进程。

被否：计数型全局分配器（要 `unsafe impl GlobalAlloc`，workspace 的 `unsafe_code` 是 forbid）；`long_turn` 自己读自己的计数器（Windows 上要么写 FFI，要么像 xtask 那样起一个 PowerShell，于是计数器有了第二个读者）。**重开参数**：出现安全的分配计数接口时，把每步分配的字节也做成门；runtime 给回合窗口一个默认的上界（压缩）时，断言从「增量处处相同」改成「窗口不超过上界」。

有一步与第一步加的不一样多时，增量就不再处处相同。 -/
theorem a_step_that_adds_more_shows_as_an_uneven_increment (first : Nat) (added : List Nat)
    (index step first_step : Nat) (at_ : added[index]? = some step)
    (head : added[0]? = some first_step) (more : first_step ≠ step) :
    ¬ ∀ one ∈ increments (windows first added), ∀ other ∈ increments (windows first added),
        one = other := by
  rw [the_increments_are_what_each_step_added]
  intro alike
  exact more (alike first_step (List.mem_of_getElem? head) step (List.mem_of_getElem? at_))

end Citysim.LongTurn
