-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::bench 与 citysim::fixture_digest

规定 `just bench` 的测量 Main `bin/bench`（`tools/citysim/src/bin/bench/main.rs`）、它的读数 `bench::reading`（`tools/citysim/src/bin/bench/reading.rs`）与场景 `bench::scenarios`（`tools/citysim/src/bin/bench/scenarios.rs`），以及两族 bench 共用的夹具摘要 `citysim::fixture_digest`（`tools/citysim/src/fixture_digest.rs`）。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面各节保留它们在 citysim 规格里的标签 §8-6、§8-12，别处引作 `tools/citysim/Spec.lean §8-6`。

能写成定理的是夹具的钉子（D8）：bench 在量任何东西之前先算一遍登记夹具的摘要，与钉住的值不等就拒绝测量，所以它印出的每一条读数都是在钉住的那份字节上量的（`every_reading_is_taken_under_the_pinned_fixture`）。摘要怎样从账本的字节算出、读数行的文法与分位怎样取，由 `fixture_digest`、`Reading::line` 与 sprawling 的 `Spread` 持有，各自的测试按字节对拍（§16）；挂钟读数本身不是任何定理的对象。
-/

/-!
### 8-6 负载场景骨架与读数行（bench）

五个负载场景都由 `just bench` 一键复测。其中四个是本 crate bench Main 的场景：大账本 fold（`large_ledger_fold`）、大 worktree 放置（`large_worktree_placement`）、再领一棵留着的 worktree（`kept_worktree_reclaim`）、长会话流式转发（`long_session_forwarding`）。第五个，多 run 并行，由 `sprawling-accounting` 的 `instrument_relay_round_trip` 量（`crates/accounting/src/worker/driving/tests/instruments.rs`，`crates/sprawling/Spec.lean` §8-84，D18）：它驱动城里在跑的那个记账循环 `attend`，`just bench` 在本 crate 那一行之后跑它。bench Main 产读数，`tools/xtask/budgets.toml` 记基线行，不另造仪表。N 个并发 run 的吞吐与等待是吞吐台（§8-14，`spec/Throughput.lean`），同样住在 `sprawling-accounting` 的仪表里。剧本执行器继续用计数时钟；本 crate 内凡计时都住在 bench Main 的模块树里，采样点仍是 `bench::stamp()` 那一个。

读数形（`bench::reading`，shape 2 value，一次构造点）：

```rust
pub enum MachineClass { General }   // 参照类属：盘、内存、CPU 均为一般水平
pub enum Load { LargeLedgerFold, LargeWorktreePlacement, KeptWorktreeReclaim, LongSessionForwarding }
pub enum SubMetric { Harness, Whole }
/// 一条读数在什么条件下量的：机器类属与所量字节的摘要。两者总是一起走，所以是一个值。
pub struct Taken { pub machine: MachineClass, pub fixture: B3Hash }
pub struct Reading { /* load, sub, taken, spread: sprawling::monitor::spread::Spread */ }
impl Reading {
    pub fn of(load: Load, sub: SubMetric, taken: Taken,
              samples: Vec<std::time::Duration>) -> Result<Reading, String>;
    pub fn line(&self) -> String;   // 唯一渲染家，键序固定
}
```

`Reading::of` 不自己算分位：样本交给 `sprawling::monitor::spread::Spread`（`crates/sprawling/Spec.lean` §8-129-2），p50/p95/p99 取最近秩，即第 ⌈n·p/100⌉ 个（一起），与 `bench_startup` 的 `Samples::of`（§8-5，也经 `Spread`）和 `sprawling gauge` 的 spread 行是同一个数。登记在 `budgets.toml` 里、由旧的下标 ⌊n·p/100⌋ 量出的各行，要在最近秩下重取；同一组样本，旧读法在 n·p/100 为整数时比最近秩高一个秩。零个样本的读数被拒，而不是印成 0（`a_reading_with_no_samples_is_refused_rather_than_printed_as_zero`）。

一行读数的文法（`Reading::line` 是唯一权威，测试按字节对拍）：

`perf load=<load> sub=<sub> machine_class=<general> fixture=<16 位十六进制> samples=<n> floor_us=<n> p50_us=<n> p95_us=<n> p99_us=<n>`

登记册行的读数（`bench::reading::row_line`，同样只经 `Spread`，同样是整数微秒）：bench Main 在场景之前量 `budgets.toml` 的四行 `ledger_append`、`durability_barrier`、`prefix_assembly`、`run_history`，每行一条

`budget row=<行名> samples=<n> floor_us=<n> p50_us=<n> p95_us=<n> p99_us=<n> peak_us=<n>[ <旁注>]`

旁注是那一行要的附带计数，键值同形：`durability_barrier` 每种批量一条，旁注 `batch=<b> per_record_ns=<p50 的纳秒数除以 b>`（摊到一条记录上常常不到 1 µs，所以记纳秒）；`ledger_append` 之后多一条成批写入 `budget row=ledger_append_all records=<n> took_us=<n> records_per_s=<n>`，只有一个样本，不成分布。预算不印在读数行里：预算住 `budgets.toml`，读数行只给读数。

`fixture` 取登记夹具摘要（D8）的前 16 位十六进制，由 `citysim::fixture_label` 拼出；完整的 64 位写在 `REGISTERED.pinned`。

`floor_us` 是最小样本：机器安静时这条路径本身要花多少。它与 `p50_us` 并列，因为两者回答的不是一个问题——floor 贴着设计的下限，p50 带着机器的其余负载——而挂钟读数不设棘轮，两者就都得留在读数里，下一个读者才分得清一次回归是设计变慢了还是机器变忙了。

`machine_class` 是读数自带的字段而非行头批注：异类机器的读数不与参照类属同表比较。口径是 harness 自身路径的处理耗时（测量机的类属见 `machine_class`）——不含动画时长、不含网络传输。`SubMetric` 两值把定标拆开计：

| 子指标 | 量的是什么 | 对它定档的是什么 |
|---|---|---|
| `harness` | harness 纯开销，路径下无持久化提交 | 两档延迟目标（第一档 p95、第二档 p99，值住 `[local_latency]` 行，第二档严于并覆盖第一档） |
| `whole` | 路径本体就是盘上作业、缝口不拆的（worktree 放置） | 无 |

场景（`bench::scenarios`，shape 4 adapter，套在产品公共面上，无自有政策）：

```rust
pub struct Fixture { /* fold_records, fold_rounds, tree_files, tree_file_bytes,
                       placements, forward_events, pinned: &'static str（64 位十六进制） */ }
pub const REGISTERED: Fixture = Fixture { … };   // 既定负载：读数只在该 fixture 内可比，只降不升
pub const PINNED_DRAFTS: u64 = 1_000;
impl Fixture {
    /// `draft` 的前 PINNED_DRAFTS 行写进 `scratch` 下一本新账，取它的 `ledger_digest`，
    /// 再与各数值字段（不含 `pinned`）的小端字节拼接后取一次摘要。
    pub fn digest(&self, scratch: &Path) -> Result<B3Hash, String>;
}
pub fn all(scratch: &Path, fixture: &Fixture, taken: Taken) -> Result<Vec<Reading>, String>;
// tools/citysim/src/fixture_digest.rs —— shape: value；两个 bench 族共用
/// 一本账的字节摘要：按 `storage::ledger_segments_at` 的顺序，每段取 `B3Hash::digest(段字节)`，
/// 32 字节依次拼接后再取一次。一次持一段字节。
pub fn ledger_digest(ledger_dir: &Path) -> Result<B3Hash, AxError>;
/// 读数行与报告点名一份夹具用的 16 位十六进制：摘要的前 16 位。
pub fn fixture_label(digest: &B3Hash) -> String;
```

bench Main 在第一项读数之前算 `REGISTERED.digest`：与 `pinned` 不等即以 `bench failed:` 加一个三段式错误退出非零（`InvalidArgs`，主体是算出的 64 位摘要，recovery 说「在一个单独的提交里把 `REGISTERED.pinned` 重钉为这个值，并重取登记册里受影响的行」）；相等则先打印一行 `fixture <16 位>`，之后每条 `perf` 行的 `Taken.fixture` 都是它。

| 场景 | 驱动的公共面 | 子指标 |
|---|---|---|
| `large_ledger_fold` | `accounting::views::ask`，重建每个视图的生产全路径 | `harness` |
| `large_worktree_placement` | `storage::Checkpoint::ensure_base` 之后，每轮先 `Worktrees::stock`（不计时，D13），再计时 `Worktrees::claim`，然后 `release`（§8-12） | `whole` |
| `kept_worktree_reclaim` | 同一座城里同一个节点的第二次及以后的 `Worktrees::claim`，其间干线不动（`crates/storage/Spec.lean` §8-9 的再领） | `whole` |
| `long_session_forwarding` | `wire::ServerFrame::Event` 装帧＋序列化，即 socket 之前的本地半段 | `harness` |

失败出口：域错误按其 `AxError`（动作/主体/稳定码/恢复语）格式化成一行；bench 自身的失败（零样本）构造 `AxError::failure(AxCode::InvalidArgs, …)`＋`with_recovery`，不新增码（§12 的口径）；Main 打 `bench failed: …` 且退出非零（既有形）。

D17 **读数行一个文法、机器类属进字段，基线读数（含机器类属）记在 `tools/xtask/budgets.toml` 各场景的行里，棘轮纪律挂 `[local_latency]` 行——只降不升、放宽需单独提交。** 理由：register 的既有定规是「只有机器能两次同样测量的量才设门」（budgets.toml 头注），wall-clock 记录不设门；机器类属字段使异类机器的读数天然不进同一张表。被否：像体积那样把延迟读数设门（超标即 CI 红）——同一处定规写着「gating them would make a busy runner look like a defect」（budgets.toml 头注与 ARCHITECTURE §11 同句）；读数回归由棘轮纪律与单独提交的放宽手续治理，不由 CI 红绿治理。

D18 **多 run 并行不在本 crate 里量。** relay、`serve_flight` 与 desk 都是 `sprawling-accounting`（库名 `accounting`）在 `accounting::worker` 之内的件，本 crate 够不到，这里的场景只能抄一份 relay 的形状：数条 lane 经 mpsc 汇到一条线程，计时只包住那条线程上的一次 `append`。抄件量的是抄件：它的 `harness` 是 MemLedger 一次追加（5 µs 量级），而一次往返的代价取决于生产循环怎么等，抄件没有那份等法，也就量不到它；它的 `persist` 每条一道屏障，生产的 `append_all` 一批一道；盘的份额由那件仪表 `store=disk` 与 `store=memory` 两行之差读出，所以 `SubMetric` 没有 `persist`。被否：给 `accounting` 开一扇公共门让本 crate 驱动 `serve_flight`。那扇门没有生产调用者，而仪表放在 crate 内已经能驱动生产循环本身（`crates/sprawling/Spec.lean` §8-84 的决定）。

D9 **被量的产品 feature 集就是 `sprawling` 包的默认 feature，只写在 `crates/sprawling/Cargo.toml` 的 `[features] default` 一处。** 人下载的二进制带执行引擎（`sandbox` feature），而 `sandbox` 是默认 feature，所以 `dist`、`bench`、`bench-startup`、`mem` 四个 recipe 不写 `--features` 就构建出人下载的那个二进制；citysim 经工作区依赖带着 `sprawling` 的默认 feature，`bench` 里的场景与仪表也在同一套 feature 下编译。于是 install 解包的、startup 拉起的、首字节服务的、内存读数量到的，与人下载的是同一个二进制。被否：①justfile 再留一个变量写 `sprawling/sandbox`——它重述清单的默认 feature，两处可以只改一处，而读数看不出它们已经分开；②每个 recipe 自己写 `--features`——漏写的那个 recipe 量的是一个没人下载的二进制。**重开参数**：人下载的二进制要带一个不是默认的 feature，那时这套 feature 回到 justfile 的一个变量里，由每个构建与测量的 recipe 读。

**红**：`a_reading_line_is_stable_and_carries_its_machine_class`——一行读数按字节对拍既有文法且带 `machine_class` 与 `fixture` 字段；`a_reading_line_carries_its_floor_beside_the_middle`——同一文法下 floor 与 p50 并列；`every_load_scenario_reruns_and_emits_the_stable_format`——本 crate 的每个场景各跑两遍，每行键序恒为文法键序（可复跑、格式稳定）；`the_registered_fixture_writes_the_bytes_its_digest_pins`——`REGISTERED.digest` 等于 `REGISTERED.pinned`（D8）。
-/

/-!
### 8-12 `large_worktree_placement`：领树接管一棵备树（`bench::scenarios`）

每轮：`Worktrees::stock`（把备树检出或带到干线，不计时）→ `bench::stamp` → `Worktrees::claim(node-<i>, &[])` → 读时钟 → `release`。四个节点名各不相同，所以每一轮都是一次放置而不是再领；每一轮的备树都是上一轮被接管之后新检出的，干线不动，所以 `claim` 接管时不写文件（`crates/storage/Spec.lean` §8-35 的计数）。夹具不变：512 个 16 KB 文件、4 轮，`REGISTERED.pinned` 不动。读数是 `sub=whole`，因为接管仍是盘上的改名与 git 的元数据写入，缝口不拆。

前后的读数（同一台机器、debug 构建、同一夹具的放置一段，四轮）：改动之前 `claim` 一次 1.18–1.30 s；改动之后 `claim` 接管备树一次 30–37 ms，`stock` 一次 1.19–1.35 s（那次全量检出，不计时）。`budgets.toml` 的 `[large_worktree_placement]` 行由 `just bench` 的发行构建读数登记。

D13 **`large_worktree_placement` 量的是领树，备树在计时之外。** 放置分成两段（`crates/storage/Spec.lean` §8-35）：`Worktrees::stock` 在没人等的时候检出一棵备树，`claim` 在 run 等着的时候接管它。场景在每次计时的 `claim` 之前调一次 `stock`，计时只包住 `claim`，因为人等的是这一段；`stock` 的代价就是改动之前的那个读数（一次全量检出），它不随这次改动变，由 storage 的计数断言守着它新建多少文件，而不是由墙钟。被否：①把 `stock` 也算进同一个样本——读数就成了两段之和，看不出领树这一段降没降；②给备树另开一行读数——要在 `bench::reading` 加一个 `Load`，那一份是读数文法的唯一权威，本决定不为一行读数改它；需要那一行时在那里加。**重开参数**：产品里有了 `stock` 的调用者之后（`crates/sprawling/Spec.lean` §8-145），如果它的位置仍让某个人等它，把它的读数加进来。
-/

namespace Citysim.Bench

variable {Digest Reading : Type}

/-- D8 **读数带着它所量字节的摘要，登记的夹具钉住这个摘要。** 两条读数只在量的是同一份字节时可比。bench 共用的 `draft` 是写在代码里的负载形状，`Fixture` 的字段说不出它：改了 `draft`，同一张登记表里前后两条读数量的就是两份负载，而读数行本身看不出差别。所以登记的夹具有一个摘要：`draft` 的前 `PINNED_DRAFTS`（1,000）行经 `storage::JsonlLedger` 写成一本账，取这本账的 `ledger_digest`，再把这 32 字节与 `Fixture` 各数值字段的小端字节拼在一起取一次摘要。这个值钉在 `REGISTERED.pinned`。bench 在量任何东西之前先算一遍，与钉住的值不等就拒绝测量；`scenarios/tests.rs` 的 `the_registered_fixture_writes_the_bytes_its_digest_pins` 在 `just check` 里做同一件事。改 `draft` 或改一个字段的提交因此必须同时改钉住的值，并且单独成一个提交，与 `[local_latency]` 行「加胖夹具单独提交」是同一条纪律。每条 `perf` 读数行带 `fixture=<摘要的前 16 位十六进制>`，这 16 位只由 `citysim::fixture_label` 拼出。

`bench_startup` 的夹具城不钉：`init_city` 用真实时钟写创世行，每次生成的字节都不同，而后面每一行的 `prev` 都接着它。它的读数照样带那座城账本的 `ledger_digest`，两条首字节读数只在摘要相等时可比；夹具城在构建档目录旁复用，所以同一台机器上前后两次读数通常量的是同一份字节。

被否：摘要只打印、不钉。打印出来的值要靠人去比，而在 `just check` 里变红的测试不需要谁记得去比。被否：给 `init_city` 加一个时钟参数，好让夹具城也能钉住。那是为 bench 给产品的创城入口开一个参数，而复用同一座夹具城已经让同一台机器上的读数可比。**重开参数**：需要跨机器比较首字节读数时，夹具城改为从一份检入的账本复制，而不是在量它的主机上生成。

bench Main 的一次运行（`main` 与 `pinned_fixture`）：算出的摘要等于钉住的值才量，量出的每条读数都带着它；否则一条读数都不量，以算出的摘要拒绝。摘要的类型 `Digest` 与读数的类型都是参数。 -/
def bench [DecidableEq Digest] (digest pinned : Digest) (measure : Digest → List Reading) :
    Except Digest (List Reading) :=
  if digest = pinned then .ok (measure digest) else .error digest

/-- bench 印出的每一条读数都是在钉住的那份字节上量的。 -/
theorem every_reading_is_taken_under_the_pinned_fixture [DecidableEq Digest]
    (digest pinned : Digest) (measure : Digest → List Reading) (readings : List Reading)
    (measured : bench digest pinned measure = .ok readings) :
    digest = pinned ∧ readings = measure pinned := by
  unfold bench at measured
  split at measured
  · rename_i same
    subst same
    simp only [Except.ok.injEq] at measured
    exact ⟨rfl, measured.symm⟩
  · simp at measured

/-- 字节离开了钉子时，bench 在量任何东西之前就以算出的摘要拒绝：一条在别的字节上量的读数不会进登记表。 -/
theorem a_moved_fixture_is_refused_before_any_reading [DecidableEq Digest]
    (digest pinned : Digest) (measure : Digest → List Reading) (moved : digest ≠ pinned) :
    bench digest pinned measure = .error digest := by
  simp [bench, moved]

end Citysim.Bench
