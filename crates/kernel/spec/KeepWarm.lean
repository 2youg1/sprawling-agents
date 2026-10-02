-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::keep_warm

规定 `kernel::keep_warm`（`crates/kernel/src/keep_warm.rs`）：缓存保温的设置与判定。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-74 `kernel::keep_warm`：缓存保温的设置与判定（形状 1 判定）

**设置与判定住 kernel，因为读它的两处互不依赖**：`city::config_layers` 把 `[cache] keep_warm` 读成这个值，runtime 按它排续期；两个 crate 只共有 kernel，所以枚举的拼法与「何时续期」只能在这里各有一处定义。

```rust
/// 城的保温设置。默认 Off：Off 时本模块不排任何续期，城不会多发一条请求。
#[derive(Default)] #[serde(rename_all = "snake_case")]   // 缺省 Off
pub enum KeepWarm { Off, FiveMinute }
/// 一个前缀的缓存状态：最近一次真实请求带它发出的时刻，与缓存最近一次被刷新的时刻（毫秒）。
pub struct CacheUse { /* used_ms、refreshed_ms —— 私有；refreshed_ms ≥ used_ms 由构造保证 */ }
impl CacheUse {
    pub fn sent(at_ms: u64) -> CacheUse;                  // 一条真实请求带着它发出
    pub fn renewed(self, at_ms: u64) -> CacheUse;         // 一条续期请求刷新了它，不改 used_ms
}
/// 下一条续期请求的发出时刻；None＝不续期。lead_ms 是调用方测得的到 provider 的往返时长，
/// 续期提前这么久发出，使 provider 在 TTL 到期前读到它。
pub fn renewal_due(setting: KeepWarm, cache: CacheUse, lead_ms: u64) -> Option<u64>;
```

- 判定：`FiveMinute` 时，续期时刻＝`refreshed_ms + PROMPT_CACHE_TTL_SECS·1000 − lead_ms`（饱和减）；该时刻距 `used_ms` 超过一个 TTL 即不续期。所以「最近 5 分钟内用过」与缓存寿命是同一个常数 `consts_external::PROMPT_CACHE_TTL_SECS`，不另立第二个 300。一次真实使用最多换来一次续期：续期不改 `used_ms`，第二次续期的时刻必然离真实使用超过一个 TTL。
- `lead_ms` 由调用方对所连 provider 实测给出，不在这里写死一个网络余量：慢链路与快链路要的提前量不同。
- 花费只观察、不设门限：续期请求照常记 usage，本模块不读余额也不拦。
- 设置按城→楼→居民三层梯解析，下层覆盖上层，一层也没说＝`Off`（`crates/city/Spec.lean` §8-4 `[cache]` 一节）。它不进 `FrozenConfig`：续期发生在两次 run 之间，不属于任何一次 run 的冻结面。
- 现状：设置与判定已落地；按它记账并经 `kernel::Model` 发出续期的是 `runtime::prefix::warmth`（`crates/runtime/Spec.lean` §8-4-2）。续期由 `accounting::worker::keeping_warm` 在房间落地后按 `renewal_due` 发出，每次续期写一行 `cache_renewed`（`crates/sprawling/Spec.lean` §8-93）。
-/
