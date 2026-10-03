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
pub struct SandboxJob { pub wasm: std::path::PathBuf, pub argv: Vec<String>, pub env: Vec<(String, String)>,
                        pub stdin: Vec<u8>, pub mounts: Vec<Mount>, pub fuel: Fuel }
pub struct SandboxOutcome { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub exit: SandboxExit }
pub enum SandboxExit { Success, Failure { code: u64 }, FuelExhausted, Trap { message: String } }
pub trait Sandbox { fn run(&mut self, job: &SandboxJob) -> Result<SandboxOutcome, AxError>; }

pub struct WasmtimeSandbox;            // feature = "wasm"；wasip1 直跑（先按 preview1 落地）
pub struct EchoSandbox { /* 直通替身：stdout＝stdin 回声＋可注入脚本输出 */ }
pub struct FaultSandbox { /* 故障替身：逐次弹出预置 SandboxExit／fuel 耗尽／trap */ }
#[cfg(feature = "conformance")]
pub fn assert_sandbox_conformance<S: Sandbox>(sandbox: &mut S, job: &SandboxJob);  // 良序两连调不中毒＋outcome 形合法
```

- **按五轴说明**（与 §8-13-2 的臂同一套，设置里的 `python` 臂照它说；`crates/wire/spec/Answer/Doctor.lean` D26）：文件——保，guest 只够得到 preopen 的目录，`Mount.writable` 为假的目录只读；网络——保，理由见下文「无出网的机械形」；进程树——保，wasip1 没有起进程的接口；用户——不保，引擎跑在 harness 的进程与身份里；资源——条件：CPU 由 `Fuel` 限，`WasmtimeSandbox` 不设 store 的内存上限，guest 的线性内存只受 wasm32 的地址空间所限，所以「CPU 与内存上限」这一轴整体答不保。它装不下 harness 居民，只跑 Python。
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

- wasmtime 与 wasmtime-wasi 钉在 48 线，下限 48.0.3。48 是上游的 LTS（逢 12 的倍数的版本支持 24 个月，其余只支持 2 个月），而本 crate 只用 wasip1 与燃料计量，49 以后带来的东西用不上。48.0.3 清掉 RUSTSEC-2026-0314／0315／0316 三条公告；这条线还含 GHSA-2r75-cxrj-cmph（path_open TRUNCATE 绕过，修于 44.0.2／45.0.0）与 CVE-2026-58494（hard-link/rename FilePerms 绕过，修于 45.0.3／46.0.1）两处修复——「钉版恒含权限绕过修复」的依据实例。下限写在清单里而不只在锁里，所以修复是一条声明出来的要求，一次 `cargo update` 退不回去。**重开参数**：上游发出下一个 LTS（60），或 48 线停止维护。

**sandbox 增**：`AbsentSandbox` —— 未带执行引擎的构建在缝上的产品实现，逐次以 `E_TOOL_UNAVAILABLE` 拒并携替代臂。它存在的理由是**缺席要是一个判词而不是一个替身**：Echo 放在这个位置会对一个从未运行的 guest 回答「成功」，而第一个察觉的人是相信了那份输出的人。
-/

/-!
### 8-25 runtime::sandbox 目录化


**缝与唯一一个真适配器分家**：缝留在父文件，两百余行 wasmtime 专属代码在适配器自己的文件里。

| 文件 | 管什么 |
|---|---|
| `sandbox.rs` | 执行边界本身：`Fuel`／`Mount`／`SandboxJob`／`SandboxOutcome`／`SandboxExit`／`Sandbox` 缝，缺席判词 `AbsentSandbox`，两个替身 `EchoSandbox`／`FaultSandbox`，以及 feature `conformance` 下的 `assert_sandbox_conformance`。它声明 `#[cfg(feature = "wasm")] mod engine;` 并原样保留 `pub use engine::WasmtimeSandbox;` |
| `sandbox/engine.rs` | 唯一会真跑 guest 的适配器：`WasmtimeSandbox`（引擎配置、预开目录、燃料预算、stdio 管道）、host 侧拒词构造 `host_error`，以及把引擎的收场判成 `SandboxExit` 的 `classify` |
| `sandbox/tests.rs` | 两个替身对调用方的承诺：直通替身回声 stdin 并记下 job、故障替身按序发脚本且发完即止 |
-/
