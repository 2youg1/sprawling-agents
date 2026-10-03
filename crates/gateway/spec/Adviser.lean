-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::adviser

规定 `adviser`（`crates/gateway/src/adviser.rs`）：一次顾问咨询，经既有登记面走到一个已挂上的端点。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `gateway::adviser` 旁的测试守住。
-/

/-!
### 8-23 `gateway::adviser`：一次顾问咨询，走既有登记面（形状 4 适配器）

```rust
pub struct AdviserClient { /* 由 `adapter_for` 铸出的适配器 ＋ 人选的 model id —— 私有 */ }
impl AdviserClient {
    pub fn attached(chosen: &Chosen<'_>, redemption: Redemption,
                    dialect_headers: Vec<(String, String)>,
                    monotonic: fn() -> std::time::Instant) -> Result<AdviserClient, AxError>;
    pub fn ask(&mut self, question: Question<'_>, policy: &BuildingPolicy,
               window: &[ChatMessage]) -> Result<AdviserAnswer, AdviserFailure>;
}
pub struct Question<'a> { pub ask: AdviserAsk, pub subject: &'a str,
                          pub options: &'a [String], pub material: Option<&'a str> }
```

**`[adviser]` 不是一段 TOML，而是一次普通的 `AttachEndpoint` 登记。** `ConfigLayer` 只有 effort／sandbox／mcp 三节且四处 `deny_unknown_fields`，写一段 `[adviser]` 只会被当场拒；顾问端点的 base_url／dialect／secret／tuning 与任何 provider 一样住 `AttachedEndpoint`，attach 时解析一次，随 `endpoint_attached` 进账本。**不造第二套 provider 表**：`attached` 收路由已经产出的 `Chosen`，与主模型调用同一支笔、同一份凭证兑付、同一套 deadline 与代理判定、同一个单调时钟（`monotonic` 由装配层给出，时钟只在 `bin::assembly` 采样），所以顾问不可能被本城用一条别的路去连。人选中哪个模型由既有的 `SelectModel`／tag 登记说话，本模块不新增选择面。

**三问一答案，答案的形状在 kernel。** 问法只有 `Noul`（是非＋概率）／`Score`（有序打分）／`Choice`（选一，仅开 session 选模型时用），答案与回落载荷形状归 `kernel::event::record`，本模块只做两件事：把问题拼成一段给模型读的文字，以及把回文严格解成一个答案。解析失败一律 `Unreadable`，**不复述模型原文**——非答案的文字没有读者，而一段会进日志的自由文本是一个没人审计的入口。

**失败是一个封闭的 `AdviserFailure`，不是 `AxError`。** 端点不可达／拒绝 → `Unavailable`，deadline 过 → `Timeout`（`AxError` 里唯一能指名的传输失败；provider 适配器把超时折在 `E_PROVIDER` 里，此处按码取，不解析文字），回文不是问的那个答案 → `Unreadable`。三种都让调用方回落确定策略并把「回落了」入账（runtime 侧 `Consultation::payloads`），**最坏情况恰好等于没有顾问时的行为**。

**顾问看窗口，不看前缀。** `ask` 的参数只有对话消息；system 那一块是本模块自己的固定指示，不是运行冻结的四段，也不带 cache 断点。这条差别就是顾问与「第二个模型调用」的全部区别：模型与 effort 在 Run 内冻结（见 §8-1 末尾两种缓存失效的区分），而顾问被问的从来只是易变半。

**为什么不新增 `pub trait`。** 这条缝的实现住 runtime 与装配层（装配层把 `AdviserClient` 包成一个闭包交给 `runtime::pipeline::adviser::Adviser::with`），gateway 侧只出一个具体类型，正如 §8-1 的 dialect 不 trait 化、`provider` 的两家各自住文件。
-/
