-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::sandbox

规定 `sandbox`、`sandbox::engine`（`crates/runtime/src/` 下同名的文件）。执行边界的缝：能力进，结局出。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::sandbox::tests` 守住。
-/

/-!
### 8-13 runtime::sandbox（缝清单文件，形状 3＋4）


```rust
pub struct Fuel(pub u64);
pub struct Mount { pub host: std::path::PathBuf, pub guest: String, pub writable: bool }   // preopen＝mount scope
pub enum GuestMemory { Wasm32, Ceiling(std::num::NonZeroU64) }   // guest 的线性内存能长到多少字节（D102）
impl GuestMemory { pub fn of_shares(shares: Shares) -> GuestMemory; }   // CpuAndMemory { limit } 给 Ceiling(limit)，其余给 Wasm32
pub struct SandboxJob { pub wasm: std::path::PathBuf, pub argv: Vec<String>, pub env: Vec<(String, String)>,
                        pub stdin: Vec<u8>, pub mounts: Vec<Mount>, pub fuel: Fuel, pub memory: GuestMemory }
pub struct SandboxOutcome { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub exit: SandboxExit,
                            pub ceiling: Option<Ceiling> }   // 只会是 Ceiling::Hit：guest 的增长在上限处被拒过（D102）
pub enum SandboxExit { Success, Failure { code: u64 }, FuelExhausted, Trap { message: String } }
pub trait Sandbox { fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>; }

#[derive(Clone)]
pub struct WasmtimeSandbox;            // feature = "wasm"；wasip1 直跑；克隆共用一个 Engine 与一格已编译模块（D101）
pub struct EchoSandbox { /* 直通替身：stdout＝stdin 回声＋可注入脚本输出 */ }
pub struct FaultSandbox { /* 故障替身：逐次弹出预置 SandboxExit／fuel 耗尽／trap */ }
#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob);  // 良序两连调不中毒＋outcome 形合法
```

- **按五轴说明**（与 §8-13-2 的臂同一套，设置里的 `python` 臂照它说；`crates/wire/spec/Answer/Doctor.lean` D26）：文件——保，guest 只够得到 preopen 的目录，`Mount.writable` 为假的目录只读；网络——保，理由见下文「无出网的机械形」；进程树——保，wasip1 没有起进程的接口；用户——不保，引擎跑在 harness 的进程与身份里；资源——条件：CPU 由 `Fuel` 限；内存在 run 有上限时由同一个数限（D102），run 没有上限时 guest 的线性内存只受 wasm32 的地址空间所限，所以「CPU 与内存上限」这一轴整体答不保。它装不下 harness 居民，只跑 Python。
- 能力面＝wasip1 preopen 集（Mount 逐条）；无网络能力（WASI p1 天然无 socket 宿主实现——Python 臂禁网的机械保证）；fuel 上限即 Fuel（耗尽＝FuelExhausted，不是 Err：宿主无故障）。
- 未授能力被拒的观察形：guest 内 open 失败→非零退出（Failure）；宿主恒不代 guest 隐藏失败。A10 三断言在真 wasmtime 上以手写 WAT 模块定形（不依赖 CPython 工件）；CPython-WASI 集成测试以环境变量指向工件（住机器本地的忽略目录，恒不入库），缺工件即 skip——`just check` 自足。
**A10 三断言结论书**（证据＝`crates/runtime/tests/sandbox_a10.rs`，真 wasmtime（版本由 `Cargo.lock` 钉），手写 WAT 不依赖任何外部工件）：

| 断言 | 观测形 | 结论 |
|---|---|---|
| fuel 内成功 | `fd_write` 写 `ok\n`，Fuel(1_000_000) | `Success`，stdout 逐字节相符 |
| 未授能力被拒 | 无 preopen 时 `path_open` 失败→guest 自行 `proc_exit(7)` | `Failure{code:7}`；**授予同一目录后同一 guest 转 `Success`**——此对拍使「被拒」是能力判定而非测试损坏 |
| fuel 耗尽中断 | 无限循环，Fuel(10_000) | `FuelExhausted`，且是 `Ok` 非 `Err`（宿主无故障） |

- **无出网的机械形需精确化**：wasip1 **确有** `sock_send`／`sock_recv`／`sock_accept`／`sock_shutdown` 宿主实现（对非 socket fd 恒返 `ENOTSOCK`）；它没有的是**获得** socket 的途径——无 `sock_open`／`sock_connect`／`sock_bind`。故导入 `sock_connect` 的 guest 直接链接失败（`E_SANDBOX_DENIED`），而 guest 能拿到的每一个 fd 都来自 preopen（均为目录）。这才是 Python 臂禁网的准确依据。
- **失败不得以默认值擦除**（rust-hardening Gate 5）：`try_into_inner()` 取不回管道、`get_fuel()` 报不出余量、退出码超 WASI 范围——三者均属**宿主故障**，恒返 `Err`；若以 `unwrap_or_default()`／`unwrap_or(1)` 兑成「空输出」「未耗尽」，就是把猜测冒充事实。
- **feature 内藏是必要的**：开 `wasm` feature 后 runtime 的依赖面增加一百多个 crate，debug 构建的 wasmtime-wasi 单件以百 MiB 计。`just clippy` 带 `--all-features`，所以内藏的代码同样过零警告门。

- wasmtime 与 wasmtime-wasi 一起动，今天在 48 线上，下限 48.0.7。版本线不另外钉：安全修复要在发布当天就到一个 PR 里，不论它落在哪条线上，所以 Dependabot 提出的每个版本（大版本也算）都开 PR，大版本与这里的代码不合、CI 红的，留给人把代码迁过去再合（人定）。48 是上游的 LTS（逢 12 的倍数的版本支持 24 个月，其余只支持 2 个月），这条线上的安全修复以补丁发出，与新大版本同一天到，补丁不改 API，所以能最快合进来。48.0.3 清掉 RUSTSEC-2026-0314／0315／0316 三条公告；48.0.6 清掉 2026-10-16 RustSec 针对 48 线发布的公告；48.0.7 清掉 RUSTSEC-2026-0378（GHSA-9pvw-hwfg-92h5：宿主建的 component model `stream`／`future` 经 guest 绕回、再以别的载荷类型降入 guest 时 panic），本 crate 只编 core module、只接 wasip1，不建 stream 或 future，这条够不着它，下限照样随公告升，因为下限说的是「这条线上已知的修复都在」，而不是「我们判断过哪些够得着」；这条线还含 GHSA-2r75-cxrj-cmph（path_open TRUNCATE 绕过，修于 44.0.2／45.0.0）与 CVE-2026-58494（hard-link/rename FilePerms 绕过，修于 45.0.3／46.0.1）两处修复——「钉版恒含权限绕过修复」的依据实例。下限写在清单里而不只在锁里，所以修复是一条声明出来的要求，一次 `cargo update` 退不回去。**重开参数**：一个大版本的 PR 带来的迁移代价大到人宁可等 LTS 的补丁，那时再在 `.github/dependabot.yml` 里忽略大版本。

**sandbox 增**：`AbsentSandbox` —— 未带执行引擎的构建在缝上的产品实现，逐次以 `E_TOOL_UNAVAILABLE` 拒并携替代臂。它存在的理由是**缺席要是一个判词而不是一个替身**：Echo 放在这个位置会对一个从未运行的 guest 回答「成功」，而第一个察觉的人是相信了那份输出的人。
-/

/-!
### 8-25 runtime::sandbox 目录化


**缝与唯一一个真适配器分家**：缝留在父文件，两百余行 wasmtime 专属代码在适配器自己的文件里。

| 文件 | 管什么 |
|---|---|
| `sandbox.rs` | 执行边界本身：`Fuel`／`Mount`／`GuestMemory`／`SandboxJob`／`SandboxOutcome`／`SandboxExit`／`Sandbox` 缝，缺席判词 `AbsentSandbox`，两个替身 `EchoSandbox`／`FaultSandbox`，以及 feature `conformance` 下的 `assert_sandbox_conformance`。它声明 `#[cfg(feature = "wasm")] mod engine;` 并原样保留 `pub use engine::WasmtimeSandbox;` |
| `sandbox/engine.rs` | 唯一会真跑 guest 的适配器：`WasmtimeSandbox`（引擎配置、克隆共用的一格已编译模块（D101）、预开目录、燃料预算、内存上限与撞限的记录（D102）、stdio 管道）、host 侧拒词构造 `host_error`，以及把引擎的收场判成 `SandboxExit` 的 `classify` |
| `sandbox/tests.rs` | 两个替身对调用方的承诺：直通替身回声 stdin 并记下 job、故障替身按序发脚本且发完即止 |
-/

/-! D101 guest 模块每个进程只读、只编译一次：`WasmtimeSandbox` 的克隆共用一个 `Engine` 与一格已编译模块（`runtime::sandbox::engine`）

**决定**：`WasmtimeSandbox` 是一个可廉价克隆的句柄，克隆共用一个 `wasmtime::Engine` 与一格已编译模块。`run` 先读模块文件的元数据，路径、长度与修改时间都与格里的相同就复用格里的 `Module`（它在 wasmtime 里本来就是共享的引用），否则读整个文件、编译、替换格里的那一份。生产上 `bin::doctor::host::execution_engine` 每次都交出同一个句柄的克隆，所以每个 worker 的 bench 用的是同一个 Engine 与同一格，一次 exec python 不再读、不再编译模块。

**理由**：编译 CPython-WASI 的模块是这条臂时延的大头，编译期的临时内存也是城的进程里最大的一笔临时分配（`crates/sprawling/spec/Serving/Memory.lean` §8-173 清点表）。只留一格：生产上只有一个 guest，一格的上界就是一份编译产物，而且换了 guest 时旧的那份随之释放。身份里带长度与修改时间，是因为重装会就地换掉组件文件，只按路径认会一直跑旧的编译产物；粗时钟下同长度、同修改时间的替换看不出来，这只在重装覆盖时发生，进程重启就恢复，代价小于每次读整个文件算摘要。

**被否**：①wasmtime 自带的盘上编译缓存（`cache` feature）：多一个 feature 与它的依赖、在家目录下多一个缓存目录，每个进程第一次调用仍要读整份产物再反序列化，它多省下的只是每个进程的第一次编译；自己写盘上缓存要 `Module::deserialize`，那是 `unsafe fn`，本工作区禁 `unsafe`。②每个 bench 一格：每个 run 仍各编译一次、各留一份编译产物，内存随活的 run 数增长。③按内容摘要认模块：每次调用都要读整份文件，省下的只剩编译。

**重开参数**：测出每个进程第一次编译在交互路径上可感（城启动后第一次 python 调用超过 1 s）；或者生产上出现第二个 guest。
-/

/-! D102 guest 的内存上限就是它的 run 的内存上限；撞到时结果带 `memory_ceiling`（`runtime::sandbox::engine`、`runtime::tools::exec`）

**决定**：`SandboxJob.memory` 是 `GuestMemory::Ceiling(limit)` 时，store 挂一个 `ResourceLimiter`：线性内存的增长会超过 `limit` 字节时拒绝，`memory.grow` 答 -1，由 guest 自己决定怎么收场（CPython 抛出它自己的内存不足异常）；拒绝由 wasmtime 的 `StoreLimits` 判，外面包的一层只记下它因为超过 `limit` 拒过（`StoreLimits` 也拒超过模块自己声明的最大值的增长，那不是撞到 run 的上限），于是 `SandboxOutcome.ceiling` 答 `Some(Ceiling::Hit { limit })`。guest 的初始内存就超过上限时它起不来，结果是 `Trap` 加 `Hit`，而不是一句「重装 sprawling」的宿主拒绝。`GuestMemory::Wasm32` 时不设上限。表按元素计数，不在这个字节上限里，只受模块自己声明的最大值限制。exec 从这个 run 的 `Shares` 推出它：`Shares::CpuAndMemory { limit }` 给 `Ceiling(limit)`，其余给 `Wasm32`；python 臂把 `ceiling` 经 `with_ceiling` 写进结果，与命令撞限同一个键、同一种拼写（`crates/runtime/spec/Tools/Exec.lean` D95）。

**理由**：guest 在城的进程里跑，它的线性内存计入城的私有字节；run 的子进程已经受 `[core] memory_bytes` 约束（D29），guest 属于同一个 run，用同一个数就不必让人再填一个，而这个数只在 `crates/sprawling/spec/Serving/Placement.lean` D47 一处定义。run 的内存没有缺省上限是 ruling（D95 的「来源」），guest 照它：没填时只受 wasm32 的地址空间限制，保护小内存机器的是机器级的预约（`crates/sprawling/spec/Serving/Memory.lean` §8-173 待做），而不是一个人没填过的数。

**被否**：①增长失败时 trap（`trap_on_grow_failure`）：guest 失去自己报内存不足的机会，模型读到的是引擎的话而不是 Python 的；②guest 另设一个缺省上限：与 D95 的 ruling 冲突，而且人在两处看到两个不同的内存数。

**重开参数**：ruling 给 run 的内存定了缺省上限；或者 wasmtime 的 `StoreLimits` 自己报出拒过（今天只能由外面包一层 limiter 记下）。
-/
