-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::watchdog

规定 `watchdog`（`crates/runtime/src/` 下同名的文件）。处置分级：纠正、退避、冻结，以及重试上限住在哪里。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::watchdog` 旁的测试守住。
-/

/-!
### 8-9 runtime::watchdog（形状 1＋处置历史持有者）


```rust
pub struct Watchdog { /* corrections: u32、provider_failures: u32、streak: u32、retries: Retries、jitter_seed: u64 —— 私有，逐 Run 一实例 */ }
// retries 的类型是 kernel::Retries（§8-43）
pub enum Disposal { Proceed, CorrectiveSteer { text: String },
                                     BackOff { until: TimeMs, code: AxCode, subject: String },
                                     Freeze { reason: FreezeReason } }
// fired_payload 写的是 kernel::event::record::WatchdogFired（`crates/kernel/Spec.lean` §8-4），本 crate 不另声明其形状
pub enum FreezeReason { Stall, ProviderRefused }
impl Watchdog {
    pub fn new(retries: Retries, run: RunId) -> Watchdog;               // run 的 16 字节折成抖动种子
    /// Consumes kernel::stall's verdict verbatim; never re-derives it.
    pub fn on_stall(&mut self, verdict: &StallVerdict) -> Disposal;      // 首次 Stall→CorrectiveSteer；再次→Freeze{Stall}
    pub fn on_provider_failure(&mut self, failure: &AxError, now: TimeMs) -> Disposal;   // BackOff.until = now + max(退避, retry_after_ms)
    pub fn on_provider_answered(&mut self);                              // provider 答了：连续失败的计数归零
    pub fn fired_payload(&self, disposal: &Disposal) -> Result<Payload, AxError>;   // watchdog_fired 载荷（E_LOOP_SUSPECTED 的 carrier）
}
/// 退避表本身：run 的种子、自上次作答以来的连续失败数、这次失败（取它的 retry_after_ms）
pub fn backoff_ms(run: RunId, failures_in_a_row: u32, failure: &AxError) -> u64;
```

- 处置必分级：纠正 Steer 文本指名重复指纹；只有终局的处置被明拒。子 Run 监控：`Completion::Limit` 的呈现住 status.children，本模块不重复存储子态。
- **provider 失败按 `AxError::retry` 的三态分类。** `No`→`Freeze { ProviderRefused }`，一次即止；`Yes` 与 `Unknown`→`BackOff { until }`（一次模型调用的效果只是一份城里从未收到的回答，效果是否落在对端只关乎计费，不关乎城里的状态，故「不知道」照样再问），`until = now + 退避`。**退避表只住本模块**（`backoff_ms`，`Watchdog` 与 `web_search` 是它的两个读者，后者见 `crates/accounting/spec/Connectors.lean` §8-35，于是模型请求与搜索请求等得一样）：自 provider 上次作答以来连续第 n 次失败的基准是 `500 ms × 2^(n-1)`，基准封顶 60 s（第 8 次起恒为 60 s）；实际等待是基准加一段抖动，抖动落在 `[0, 基准/2]`，所以第 n 次等待落在 `[基准, 1.5 × 基准]`，最长 90 s。起点 500 ms 与封顶 60 s 都是对端的尺度（一次过载的恢复时间），与所在机器的快慢无关，故不从机器的测量推导。落选的是「由调用层给 `until`」：调用层手里只有当前时刻，而 `gateway` 里并没有持 retry-after 的准入状态，结果是 `until` 恒等于「现在」，重试不隔一刻。**连续的计数在 provider 作答时归零**（`on_provider_answered`，`drive` 在每个走完的回合后调用）：一个已经恢复的 provider 不欠下一次故障一分钟的首等，而对重试的上限是对「同一个调用再问一次」的上限，不是对整个 Run 一生碰上几次故障的上限。`provider_failures` 仍是这个 Run 的总数，只作观察。**对端说了等多久时，等的是两者中较长的那个**：`until = now + max(退避表, AxError::retry_after_ms)`。对端的 `retry-after` 是它对自己何时恢复的陈述，早于它再问只会再收一次 429；退避表仍是下限，因为一个说「1 秒后」的对端连续失败时，连续计数照样该拉长间隔。对端给的等待不设上限：它可以很长，而停下一个在等的 Run 的是 `Halt`，`watchdog_fired` 里的 `until_ms` 让人看见它在等什么。**抖动以 `RunId` 为种子，同一个 Run 永远得到同一串等待。** 同一时刻被同一次故障打断的多个 Run 若按同一张表等待，会在同一毫秒一齐再问，把刚恢复的对端再压垮一次；抖动把它们错开。种子是 `RunId` 的 16 字节经 FNV-1a 折叠，与连续计数一起过一遍 splitmix64 的收尾混合，再对 `基准/2 + 1` 取余；不取随机源，因为一个 Run 的历史要能逐字节重放（citysim 与离线重放读的是同一张表），而 uuid v7 的末 74 位本身就是随机的，已经把同一毫秒启动的 Run 分开。抖动只往上加：退避表仍是下限，`retry-after` 取两者较长的规则不变。落选的是往下抖（`[基准/2, 基准]`）：那样第一次等待可以短于 500 ms，而表说的是「至少等这么久」。
- **可重试的失败只对着人设的那个上限冻住**（`Retries`）。`UntilHalted` 下停它的是 `Halt`，城里唯一的刹车；`AtMost(n)` 下停它的是人在端点表单上填的那个数。**这两格是穷尽而不是一个带哨兵值的计数**：「一直试到有人喊停」与「试四次」是两种意图，一个数字拼不出前者。填进表单却没有任何东西去读的数字，比根本不给这个字段更糟，所以 `request_max_retries` 的读者就是 `Watchdog`，而它的调用方是下一条的 `drive`。
- **`runtime::run::drive` 是那个调用方**：一次可重试的失败写一条 `watchdog_fired` 再重来，于是历史里第二条 `model_called` 就是人读到的那次重试，而不是一次无声的重复。节奏归 `Watchdog` 的退避表：`drive` 先把 `Watchdog` 给的 `until` 写进 `watchdog_fired`，再交给 `RunHooks::wait` 等到那一刻，于是历史许诺的「不早于 `until_ms`」与下一条 `model_called` 一致。**等待是 `Halt` 够得着一个没有回合在飞的 run 的地方**：`wait` 答 `Halted` 时 run 以 `Cancelled` 冻住，不再发下一次调用。落选的是「等完再问 `interrupt`」：退避长到一分钟，一个晚一分钟才生效的刹车不是刹车。装配层的 `wait` 以 50 ms 为片睡到 `until`，每片问一次是否停下；等待中到达的 steer 留到下一个安全点，不在等待里被吞掉。计数时钟（citysim、离线重放）答 `Allowed` 且不等，因为它重放的东西不在真实时间里等待。
- **为什么按 `AxError::retry` 分类而不设固定次数。** 一个计数器对两种截然不同的失败给同一份预算：`E_WIRE_MISMATCH`（对端不说这个形状）重试三次就是把同一个 400 买三遍，而 429 重试三次就放弃又恰好把一个只需要等待的维护窗口当成了死亡。能否再试是产错处已经知道的事实（`Retry`，fail-closed），拿它分类比在这里重新猜一遍强。
- **冻结原因叫 `ProviderRefused`**（载荷 `reason` 为 `provider_refused`）：没有重试预算，就没有东西被耗尽；冻住的原因是对端给了一个重试不能修复的答复。
- fired_payload 的形状由 `kernel::event::record::WatchdogFired`（`crates/kernel/Spec.lean` §8-4）独家拼出：`drive` 若对同一个 kind、同一个 `back_off` 词另写一份 {action, code, subject}，一份历史里就有两种 `watchdog_fired`。退避的原因随 `Disposal::BackOff` 一同旅行——说自己退避却不说退避什么的一行，没人能据以行动。字段＝{action: steer|back_off|freeze, text|(until_ms,code,subject)|reason, corrections, provider_failures}；Proceed 拒绝成帐（无事不记）；纠正只发一次（corrections 计数），第二次 Stall 即冻——分级穷尽于 steer→freeze 两级，「停滞中间态」不另设（它就是 Stall verdict 本身）。`provider_failures` 留下作为**观察**（这个 Run 碰上了几次），不是一个阀值。
-/

/-!
### 8-43 重试上限住 kernel


两个 crate 互不依赖，而 gateway 决定要不要再发一次请求、`Watchdog` 决定要不要冻结这次运行，读的是同一个事实——所以 `Retries` 住 `kernel::retries`，两边都直接用 `kernel::Retries`，不导出别名：别名让读者以为有两个类型，调用点于是写出一个两臂恒等的 `match` 去「转换」它们。
-/
