-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# bin::serving

规定 `crates/sprawling/src/serving.rs` 与 `crates/sprawling/src/serving/`：serve 时的服务面，门、进程日志与视图的折叠线程（`bin::serving`）；核心线程的档位在 `spec/Serving/Standing.lean`，输出环在 `spec/Serving/OutputRing.lean`。本文件是 `crates/sprawling/Spec.lean` 的一个分部；下面每一节保留它的标签 §8-n，别处引作 `crates/sprawling/Spec.lean §8-n`，决定引作 `sprawling D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `bin::serving::folding::tests`、`bin::serving::folding::tests::instruments`、`bin::serving::tests` 守住。
-/

/-!
## 8-38 一座城怎么被端上来，与一轮活怎么跑完（`bin::serving`）

「一座城怎么被端上来」与「一轮活怎么跑完」不是同一件事，所以前者住 `bin::serving`，后者住 `RunWorker`。serving 持有门口那把钥匙（`serving::door` 的 `key_for` 与 `random_token`——熵在本 crate 只有这一处）、金库的开启（`open_vault`）与一次 serve 由调用方填好的那个值（`serve::Serving`）；写者线程、命令台与 lane 属于装配点（§8-92）。

写者线程由 `assembly::attending::spawn_worker(opening: Opening, outward: Outward)` 开，账本在它里面打开且从不离开。两个参数各是一个值：`Opening`（一个写者是用什么打开的：城根、金库、金库探测的发现、日志、serve 线程在写锁下已经折好的账本与 `Standing`、审计的日志、核心线程的档位）与 `Outward`（它的活从哪来、结果到哪去：命令台、发布出去的视图与备用的一份、给页面与观察者的广播、账本头、还在跑的命令的输出）。两个值代替一长串参数，没有 `#[expect(clippy::too_many_arguments)]`。

**这台电脑上的这扇门对浏览器的句柄由装配点建一次**：`assembly::listen` 经 `assembly::listening::doorstep` 用 `bin::serving::browsers::load` 读城的 `.sprawling/browsers.toml`（`kernel::layout::CityLayout::browsers`，只存公钥、标签与时刻），把时钟、随机源与写回那张表的闭包交给 `wire::LocalDoor::new`，再把同一个句柄交给 `ServeConfig.door`、控制台与 `firstrun::open_paired`。表写失败时那次配对不算数（`crates/wire/spec/Server.lean` §8-93）。`Query::Devices` 由 `LocalDoor::devices` 作答，在问城的视图之前截下，因为设备表不在账本里。钥匙文件（`bin::serving::key_file`，`crates/sprawling/spec/Keying.lean` §8-22）在 `listen` 的最后一步写，所以一次被拒的服务不留钥匙；收口时删。`ForgetDevice` 也在那里截下，由 `LocalDoor::forget` 执行，因为设备表不在账本里。

**同意一个 ACP agent 在装配层执行**（`assembly::listening::consenting`）：`AddAgent { spec_digest, source, seat_here }` 在进命令台之前截下，因为它要的两样东西只有 serve 的城有：城答出去的 offer（`accounting::offered::Offered`，与视图共用一份；注册表的条目从 `agent_protocols::Catalog::bundled` 重取）与这台电脑的搜索路径（与 harness 页同一个 `find`）。按摘要与来源取回那一项，摘要不等即拒，所以写下的就是卡片上人看见的；程序不是绝对路径就在搜索路径上找，找不到以 `E_TOOL_UNAVAILABLE` 拒，恢复语让人装上它或把它放上 PATH，因为解析成绝对路径之后，PATH 上后来出现的同名程序换不掉人同意过的那一个。写下的行带解析后那份启动说明的摘要（`launch_digest`），派活时由 `Consented::given` 对着它判。`seat_here` 在场时再写那个房间的 `[resident] harness`（`city::seat_agent`）。写路径是 `city::write_agent` 的 `change`，与 worker 写配置走同一把持有。run worker 收到 `AddAgent`，说明它绕过了这一步，答「同意在城自己的监听上取」。

`CITY_VERIFIER`（`accounting::worker::workbench`）与 `local_model_facts`（`accounting::worker::credentials`）住在 `assembly`，因为它们是 `RunWorker` 在一轮活里用的东西，不是端城用的；执行引擎由 `doctor::host::execution_engine` 按平台选出。
-/

/-!
## 8-123 记账线程不写可丢的投影，视图积压有读数也有界（`bin::serving::folding`，形状：adapter；`accounting::worker::attend`）

```rust
// bin::serving::folding
pub(crate) struct Folding {
    pub(crate) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(crate) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(crate) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,
    pub(crate) keep_slices: Box<dyn FnOnce(storage::Sessions) + Send>, // 会话切片交给视图线程
    pub(crate) thread: std::thread::JoinHandle<()>,
}
#[derive(Clone, Default)]
pub(crate) struct Backlog { /* Arc<AtomicU64> */ }
impl Backlog {
    pub(crate) fn records(&self) -> u64;   // 已送进视图线程、还没折完广播的已提交记录条数
}
pub(crate) const CUT_WAITS_ABOVE: u64 = 256; // 积压多于这个数，服务中的切快照往后放
// accounting::worker::attend
impl RunWorker {
    pub fn hand_off_session_slices(&mut self) -> Option<storage::Sessions>; // 把会话切片交出去（`crates/storage/Spec.lean` §8-24）
}
```

**记账线程只做记账。** 记账线程是所有 lane 共用的一条线程（§8-42-4）：它每多做一件与落账无关的事，每条 lane 的下一次 append 就多等一段。会话切片（`crates/storage/Spec.lean` §8-24）是给人翻看的投影，原先在记账线程上写：每一波落盘之后逐条 `absorb`，缺文件时整份重建并 `sync_data`，进程里第一次缺文件时 `first_seen` 还要把全部段读一遍。服务中的城在写者起好之后把切片交给视图线程：写者线程在接上观察者之前调 `hand_off_session_slices`，把拿到的 `storage::Sessions` 经 `keep_slices` 送进视图线程的通道；视图线程此后每批折完、广播完，再按账本序逐条 `absorb`。切片写不成照旧写一行 stderr、跳过，与它在记账线程上时一样。于是一次 append 在记账线程上的代价只剩这一波本身：组帧、写、`sync_data`、观察者的一次 `send`。**被否：另起一条线程只写切片。** 视图线程已经按账本序收到每一条已提交记录，另起一条就是第二条通道、第二份次序；切片在视图线程上比账本晚一个批次，这是投影可以有的落后。**被否：切片留在记账线程上，只把 `first_seen` 换成读索引。** 缺文件时整份重建的那次读写与 `sync_data` 仍在记账线程上。

**积压有读数。** `Backlog` 是一个原子计数：观察者在 `send` 之前加一，视图线程广播完一批之后减去这批已提交记录的条数，所以任何时刻读到的是「写者已写下、页面还看不到」的条数。读法是 `bin::serving::folding::Backlog::records`，今天它唯一的读者是下一段的界；波次 4 的 W 车道把它交给采样线程，填进 `Sample.view_backlog`，本节不改线格式。加减饱和：两条线程的加减在一次读里可以错开。

**积压有界：追得上之前不切快照。** 服务中切视图快照（§8-91 的 `Cadence`）是视图线程上最长的一件事：编码整份视图、写盘、`sync`。积压多于 `CUT_WAITS_ABOVE` 条时，到期的切快照往后放，视图线程先把积压折完、发布、广播；积压回到界内之后的下一批再切。通道关闭时的最后一次切快照不看积压。界取 256 条：视图线程一批就是一次醒来时通道里已到的全部，积压多过几百条说明它已经落后于写者，这时再花一次整份编码只会让页面更晚看见新记录；这个数由 `instrument_view_backlog`（§8-99）在突发里读到的 `max_backlog` 复核。**被否：积压过多时让观察者阻塞写者。** 那把视图线程的落后转嫁给每一条 lane，而视图只是投影。

**测试。** `serving::folding::tests` 的 `a_cut_waits_while_the_fold_is_behind`：同一个到期的 `Cadence`，积压在界内时交出要切的那条记录，积压超过界时不交出。storage 的 `sessions::tests::a_ledger_that_handed_off_its_slices_files_nothing_itself`：交出切片之后写者追加的记录不落切片，拿到切片的一方 `absorb` 之后落下。

**本节接口的当前状态。** `resume`、`fork`、`adopt` 与一次性命令没有视图线程，切片仍在它们的写者线程上写（`crates/storage/Spec.lean` §3 第 6 条）。记账线程上其余的长任务——派活时起名字、读计划——各在自己的节里（§8-42-4、§8-86）。
-/
