-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::worker::genesis::lost

规定 `crates/accounting/src/worker/genesis/lost.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-18-1 启动扫描为死掉的 run 写冻结

```rust
// accounting::worker::genesis::lost
pub(super) struct OpenRuns { /* 每个还开着的 run：它 run_started 的 seq，与它最近一行的作者 */ }
impl OpenRuns {
    pub(super) fn observe(&mut self, record: &EventRecord);        // 与 DanglingCalls 骑同一遍验链
    pub(super) fn into_drafts(self, t: TimeMs) -> Result<Vec<EventDraft>, AxError>;   // 按 run_started 的 seq 升序
}
// accounting::worker
pub struct ScanReport { /* …既有字段… */ pub frozen_runs: usize }
```

- **哪些 run 死了。** `startup_scan` 只在 `RunWorker::new` 拿到写者锁之后跑（`sprawling resume`），这时这座城没有一次 run 在驱动：账上有 `run_started`、没有 `run_frozen` 的每一次 run，都是上一个进程死时正在跑的。harness run 与模型 run 一样开、一样冻（`crates/runtime/Spec.lean` §8-52），一并计入。
- **次序。** 先补写悬空调用的 `E_TOOL_OUTCOME_UNKNOWN`（它们属于那次 run，要落在它的冻结之前），再为每次死掉的 run 写一行 `RunFrozen::lost()`（`crates/kernel/Spec.lean` §8-82-2），按 `run_started` 的 seq 升序。
- **作者。** 冻结行写成那次 run 最近一行的作者（它的居民），与补写的 `tool_result` 写成那次调用的作者同一条理由：按居民计数冻结的读者（`city::resident` 的档案）不因进程死过而少计一次。只写过 `run_started`、没写别的就死了的 run，用 `run_started` 的作者。`addr` 缺席，与 `Charter::close` 写的冻结行同形。
- **幂等。** 第二次扫描看到的每次 run 都已冻结，什么都不写；`ScanReport.frozen_runs` 是这一次写了几行，`summary` 把它与关掉的调用数并列告诉人。
- **不做的事。** 不写 `handoff_written`：死掉的 run 没留下交接，替它编一份是假话；不起后继：冻结的 run 是历史，接手由人或计划另派（ARCHITECTURE §13.7）。
- 验收：`genesis::lost::tests` 的 `a_run_the_process_died_in_is_frozen_once`（冻结行的载荷、作者与次数，第二次扫描不再写）；崩溃验收（`crates/sprawling/Spec.lean` §8-127）钉住城景里那次 run 的最后一行是冻结、结局是 `cancelled`。
-/

/-! D30 死掉的 run 由启动扫描冻结，冻结行写成它的居民，结局读 `RunFrozen::lost`

(a) 理由：只有拿到写者锁的那一刻才知道没有别的进程在驱动它，而 `startup_scan` 正是那一刻的那一遍验链；视图与 worker 的折叠都从账本来，账上一行冻结让服务中的城与重开的城对同一次 run 说同一个结局。写成居民而不是城，与补写结果未知的调用同一条理由，按居民计数的读者不必为死亡另写一条规则。被否决的做法：在服务时由视图把「没有冻结行、进程已重开过」的 run 读作死掉——那是视图的第二条冻结规则，而且一次性的 `views::ask` 与服务中的视图会各算一次；由 `RunWorker::new` 冻结——`new` 也在 `serve` 里跑，那时冻结要跟账本证明的次序对齐，而 `resume` 本来就是收拾死亡的那一步（`crates/sprawling/Spec.lean` §8-109）。重开参数：`serve` 也要在起步时收拾死亡（不经 `resume`）时，把这一遍挪进它的起步路径，次序仍是先补调用、后冻 run。 (b) **指南进度住 `accounting::guide`，一个与 `person` 平行的模块，读写各一扇门。** 理由：页面读与命令写读的是同一份文件，文件的文法只能有一处；它不属于视图的折叠，也不属于 worker 的状态，`person` 已经是「一份人改的文件，读整份、写整份」的样子。被否决的做法：读放在 `views::answering`、写放在 `worker::commanding`——两处各知道一遍文件的形状。(c) **跑 gh 的函数经 `Views::ask_github_through` 交进来，主机名的判定与缺省主机留在视图。** 理由：起子进程碰主机，按第 9、10 条住 `sprawling`、经 `fn` 指针交进来；而「问哪台主机、这个串能不能交给 gh」是城对输入的判定，测试不必起 gh 就能判它。被否决的做法：经 `Hands` 交给 worker——这是一条查询，worker 不答查询；在二进制里判主机名——测试就要经过子进程才看得到拒绝。
-/
