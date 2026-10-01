-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::degradation

规定 `kernel::degradation`（`crates/kernel/src/degradation.rs`）：资源成为瓶颈时的降级与是否接新活。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-74 kernel::degradation（形状 1 判定）

资源成为瓶颈时，harness 如实说出慢在哪里。本模块只判定：读数由调用方采，显示由事实条与 doctor 做；这里给出每种降级的类型、它的依据与恢复办法，以及剩余空间不够时是否还接新活。

```rust
pub struct ResourceReadings {
    pub durable_lag: Duration,     // 最早一条已追加未落盘的事件等了多久（持久水位线落后多少）
    pub commit_floor: Duration,    // 本卷实测的 fsync 中位数：该步的物理下限
    pub volume: VolumeSpace,       // 城所在卷的剩余空间与总容量
    pub queued_runs: u32,          // 因可用内存不够而排队的 run 数
    pub schedule_delay: Duration,  // 记账线程从被唤醒到真正运行的中位延迟
}
pub struct VolumeSpace { pub free_bytes: u64, pub total_bytes: u64 }
pub enum Degradation {
    DiskSlow { durable_lag: Duration, commit_floor: Duration },
    DiskLow { free_bytes: u64, floor_bytes: u64 },
    MemoryTight { queued_runs: u32 },
    CpuSaturated { schedule_delay: Duration },
}
pub enum Recovery {
    ReduceDiskLoad,
    FreeDiskSpace { at_least_bytes: u64 },
    FreeMemory,
    ReduceCpuLoad,
}
impl Degradation { pub fn recovery(&self) -> Recovery; }
/// Every degradation the readings show, in the order DiskSlow, DiskLow,
/// MemoryTight, CpuSaturated; no allocation.
pub fn assess(readings: &ResourceReadings) -> impl Iterator<Item = Degradation>;
/// Whether the city takes on new work: a volume below its floor refuses
/// with the DiskLow it shows; every other state admits.
pub fn admit_work(volume: VolumeSpace) -> Result<(), Degradation>;
/// The free-space floor this volume derives: a share of its size, clamped.
pub fn free_space_floor(volume_bytes: u64) -> u64;
```

- 依据即变体字段：每种降级带着判定它的读数，显示面不再回头读第二份。恢复办法是类型，措辞在 `client/src/lang.json`。
- 盘慢：`durable_lag` 同时超过 `commit_floor × DISK_SLOW_FACTOR` 与 `DISK_SLOW_MIN`。前者随设备走（旋转盘的 fsync 本来就慢，不算降级），后者挡住快盘上的微秒级抖动。
- 盘快满：`volume.free_bytes < free_space_floor(volume.total_bytes)`；地板取卷容量的 `1 / FREE_SPACE_FLOOR_DIVISOR`，夹在 `FREE_SPACE_FLOOR_MIN` 与 `FREE_SPACE_FLOOR_MAX` 之间。恢复办法给出回到地板以上至少要腾出的字节数。
- 内存紧：`queued_runs > 0`。排队由装配层按可用内存决定（运行因一个也装不下而等待），这里只把它说出来。
- CPU 被占满：`schedule_delay > CPU_SATURATED_DELAY`，即一帧（60 Hz）——人开始看得见的延迟；它是感知常数，不随机器类别调。
- 只有盘快满停止接新活：盘慢与 CPU 满时接活只会变慢，不会丢；内存紧已由排队处理。`admit_work` 只读卷的两个数，因为受理新活的入口只该为它付一次读卷，而不是整份读数；拒绝带着 `Degradation::DiskLow`，入口据它的 `recovery()` 告诉人至少腾出多少。
- 写盘失败时账本不坏、重启可恢复，由 storage 承担（`crates/storage/Spec.lean` §8-1）：失败的一波由 `jsonl::unwind` 把段退回波前长度，进程接着写也不会写在半行之后；掉电留下的撕裂尾由 open 截到最长有效前缀。本模块不复述。
- 生产的调用方：人发来的 `Dispatch` 在写下任何东西之前经 `admit_work` 读一次城所在卷（sprawling-SPEC 8-94）。未落地：事实条与 doctor 的显示；`Wake` 等不经人的入口；`ResourceReadings` 其余四项的生产填写者——`bin::monitor::Sample` 没有 fsync 中位数与调度延迟，它的 `durable_lag` 是条数而本模块要的是等待时长，且监视器只在有人看时采样，不能作为判定的唯一来源（sprawling-SPEC 8-90 决定 1）。
-/

/-! D5 定规：降级的线相对于设备，停止接新活的拒绝带着读数

**决定**：`degradation` 的「盘慢」线取本卷实测 fsync 的倍数（另设不闪烁的下限），「盘快满」的地板取卷容量的一份再夹住上下界，「CPU 被占满」取一帧；只有「盘快满」停止接新活，`admit_work` 只读卷的剩余空间与总容量，拒绝时返回那一个 `Degradation::DiskLow`。

**理由**：harness 要在十年前的旋转盘和服务器上都说真话。固定毫秒数的盘慢线会让旋转盘永远处于降级、让 NVMe 永远不降级；固定字节数的地板对小卷太贪、对大卷太松。CPU 的线是人的感知常数，与机器类别无关。拒绝要告诉人至少腾出多少字节，`backpressure::Admission::Shed` 只带一个原因，读数会在入口处丢掉；`backpressure::admit` 管的是队列的格数，与卷的字节不是同一个决定。盘慢、CPU 满、内存紧都只让活变慢或排队，不丢数据，所以不拒绝；盘满继续接活会让下一次写盘失败。

**被否**：①每种降级各自决定是否拒活——内存紧已由装配层按可用内存排队处理，再拒一次是同一决定两个家；②固定阈值——违背「不把常数调成某一类机器」；③经 `Admission::Shed { reason: DiskLow }` 拒绝——入口得再算一次地板才能说出恢复办法，同一个地板两个家。

**重开参数**：实测显示某一类设备上 `DISK_SLOW_FACTOR` 让盘慢状态在无外部负载时出现，或地板不足以写下一次快照。
-/
