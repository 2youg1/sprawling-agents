-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::turn::recovery

规定 `turn::recovery`（`crates/runtime/src/` 下同名的文件）。模型调用的恢复管线，以及回合怎样记下回复的首个内容几时到。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-49 turn::recovery —— 模型调用恢复管线的段契约（形状 2 值＋形状 3 内缝）


**错误分层三层各管一段。**「同一个请求还能不能再发一次」（`AxError::retry`）的唯一家是 `gateway::endpoint::failure::ProviderFailure`；本模块的恢复段只修**同样的请求再发一次也注定同样失败**的形状类失败——换一扇门再问一次；可重试失败与「再发也一样」的拒词归 `runtime::watchdog`（§8-9）处置。三层互指互不越权：`ProviderFailure` 对可重试族的恢复语说由 watchdog 退避后再发同一个请求，段对那一族恒 `Skipped`。
```rust
// turn/recovery.rs（形状 2 值＋形状 3 内缝；pub(super)：turn 之外没有第二个用户）
pub(super) enum SegmentOutcome {
    Recovered(ModelReturn),   // 修好了：回合拿这个 ModelReturn 照常写 model_returned
    Failed(AxError),          // 认出失败且动了手，没修成：修复重发拿到的原错误，码与恢复语必带
    Skipped(AxError),         // 不是本段要修的：被递上的错误原样转手给下一段
}
pub(super) trait Segment {
    fn attempt(&mut self, failure: &AxError, call: &mut ModelCall<'_>) -> SegmentOutcome;
}
pub(super) struct ModelCall<'a> { /* journal、ledger、model、request、streamed —— 全私有 */ }
impl ModelCall<'_> {
    pub(super) fn open(journal: &mut Journal, ledger: &mut dyn Ledger,
                       model: &mut dyn Model, request: &ModelRequest) -> ModelCall<'_>;
    pub(super) fn streamed(&self) -> bool;               // 失败那次是不是从流式门出去的
    pub(super) fn resend_blocking(&mut self) -> Result<ModelReturn, AxError>;  // 换阻塞门再问一次
    pub(super) fn ask(&mut self, segments: &mut [&mut dyn Segment],
                      generating: Generating<'_, '_>) -> Result<Settled, AxError>;   // §8-50
}
/// 接力：名单序逐段递上失败；Skipped 转手即问下一段；Recovered／Failed 即止；
/// 全段转手则把最后转手的错误原样答成 Skipped。
pub(super) fn recover(segments: &mut [&mut dyn Segment], call: &mut ModelCall<'_>, failure: AxError)
    -> SegmentOutcome;
pub(super) struct BlockingResend;                        // 今天唯一的生产段
```

**五条口径：**

1. **三值穷尽且闭。** 新答案必须逼每个调用方表态；`Skipped` 必携它被递上的那个错误——code／action／subject／recovery／retriable 逐字段同行，跳过是转手而不是吞掉，`let _ =` 一族在本模块没有落点。每段的答案都是类型化 `AxError`，稳定码与恢复语由 `AxError` 的构造契约保证必带（"a failure with no next step is unconstructible"）。
2. **接力序就是名单序**，第一个非 `Skipped` 的答案终止接力。`recover` 自己也答同一个三值型（全段转手＝`Skipped(原错误)`），管线套管线仍是一种形状。
3. **落账先于效果，每次外发都算。** `model_called` 在每一次尝试之前落账（首打与段的重发同走 `ModelCall`），历史里第二条 `model_called` 就是那次修复重发——与 §8-9 重试同一读法，不是一次无声的重复。
4. **错误形不动。** 不新增 AxCode、不给 AxError 加字段；`AxCode::carrier()` 的穷尽不变。
5. **回合层只见 `Result`。** `ask` 把三值折成 `Ok(return)`／`Err(该带的那个错误)`：段间谁转手了什么是本模块面的事实，回合层不需要第二次解读。

**失败码**：本层不造码。`Failed` 携带的是修复重发拿到的原错误（生产段即阻塞门自己的失败，码由 gateway 一处给出，通常是 `E_WIRE_MISMATCH`）；全段转手即首个失败的原码，逐字段不变。`E_WIRE_MISMATCH` 的"能否定义掉"随 §12 走：恢复段只是把「同样的请求再发一次」换成「换一扇门再问一次」，没有改变该码的可定义性。

**生产段 `BlockingResend` 的触发点，一个都不多**：失败那次走的是**流式门**且失败码是 `E_WIRE_MISMATCH`，并且这次调用的账号轮放行一次修复（`AccountRound::admits_repair`，`crates/kernel/spec/AccountRecovery.lean` §8-86）。多账号时那次换门重发是原号上的一次发送：放行才发，发了就记 `AccountRound::repaired`，不放行即 `Skipped`，原错误交给下一段与 `Watchdog`，于是一个账号一轮里的发送不超过 1＋k（`sends_on_one_account_stay_within_the_budget`）；单账号的轮总是放行、不计数，与这一条出现之前相同。理由：两扇门是同一条缝的两个口（`kernel::model` 保证同答同败），但**流式装配与整身解析是两条解析路径**——流式工具调用拼接出的半句话在阻塞门是一份完整 body。其余失败一律 `Skipped`：可重试族归 watchdog，门拒与配置族换门重发只会把同一个拒词买回来。

**被否（各记理由与会重开它的参数）：**

- **空响应后追问一句的段**：「一条什么都没说的回复」意味着什么，§8-37 的 `concluded` 是唯一判定处（`Completion::Limit`），调用层再判一次就是第二个家，而 nudge 重发还会改写那条以供应方真实报文钉住的剧本。参数：有"空响应是瞬态"的实测数据时，与 §8-37 一同重开。
- **「剔除被拒工具后重发」段**：本仓的 provider 拒绝从不回引 body（`ProviderFailure::Refused`），没有类型化触发点可依，靠错误文本嗅探即造脆弱权威；无声砍工具是能力的静默降级，与"拒答即声明"相悖。参数：gateway 的拒绝族带上类型化的拒绝面之后重开。
- **「补悬空 tool_result 后重发」段**：回合窗口按构造无悬空调用（被取消的回合不入窗；`fold_run` 遇开波回退到上一安全点），触发点不存在；账本侧关帐的权威在 `replay::resume`，不写第二份。参数：出现能把悬空对话推进窗口的新入口时重开。
- **盲重试段（对可重试失败原样重发）**：该判定属于 watchdog 与端点的并发名额 `gateway::concurrency`（§8-9：watchdog 只判断还有没有下一次；`crates/gateway/Spec.lean` D17），段越权即第二个重试权威。
- **`Skipped` 无载荷（unit 变体）**：结构上确实吞不掉错误，但"这一段转手的是哪个错误"也在答案里读不出来了，而谁把什么交给谁正是段契约要陈述的事实。
- **段＝闭枚举（形状 6）**：多段场景只能靠触发点拼装，契约测不直接，且新修复进来即改枚举与全部 match。trait 的第二实现是测试里的记账段（本仓缝规则认可的替身一族：测试时钟、计数店）。
-/

/-!
### 8-50 回合记下回复的首个内容几时到（`turn::recovery`）


```rust
pub(super) struct Settled {
    pub(super) returned: ModelReturn,
    pub(super) speculated: Speculated,
    pub(super) first_at: Option<TimeMs>,   // 落账的那一次尝试的首个内容；写进 model_returned（`crates/kernel/Spec.lean` §8-75）
}
```

1. **首个内容**：一段非空的文字（`Increment::Said`）或推理（`Increment::Thought`）。空增量不算；心跳、角色声明与只带用量的帧从来不成为增量（gateway 的 `increment_of` 把它们挡在口外），所以口内口外是同一个定义。
2. **在哪里读钟**：`ask` 把所选的门收到的增量汇点包一层，第一段非空增量到达时经 `Journal::read_clock` 读一次，此后不再读。`Speculating` 而没有页面在看（`deltas: None`）时也包：那扇门照样是流，量它不需要有人看。`Unwatched` 走阻塞门，没有增量，`first_at` 缺席。
3. **属于落账的那一次尝试**：流式尝试失败、换阻塞门重发（`BlockingResend`）修好的回复，`first_at` 缺席。那次重发之前写下的 `model_called` 是它的起点，而它没有流；把失败那次的读数挂到它上面，读者算出的首字耗时量的是另一次请求。
4. **读钟失败即回合失败**：与 `model_returned` 自己那一刻同一只钟、同一种失败。汇点不能失败（`kernel::Increments` 没有返回值），所以读数先存下，模型调用返回之后再抛出。
-/

/-! D6 首个内容只数文字与推理，钟在回合里读

**决定**：`first_at` 是第一段非空文字或推理到达时回合读到的时刻（§8-50）。工具调用不算首个内容：模型口今天的增量只有这两种，只带工具调用的回复没有 `first_at`。

**理由**：回合的钟只有一个入口，即 `turn::ledger::Journal`（D5）；模型口的实现不读钟（`kernel::Model` 的约定），所以时刻只能在口的消费方读。增量汇点是每一扇流式门都经过的地方，包它一层就量到了三种兼容格式，gateway 一行不改。

**被否**：①gateway 在流里采样，再随 `ModelReturn` 交回：模型口多一个读钟的实现，citysim 的脚本模型也要学会造一个时刻；②给 `kernel::Increment` 加一种「工具调用开始了」的片段：增量随 `ServerFrame::Delta` 上线，那是一次线协议改形，每个读增量的页面都要多一臂，为的只是只带工具调用的那类回复；③把提前交出的完整工具调用（`EarlyCalls`）算作首个内容：只有 Anthropic 的流交出它，而它到的时刻是那一块的结束，不是开始，同一个字段在两种兼容格式里会量两种东西。

**重开参数**：run 页画首字耗时时，只带工具调用的回合多到让那一列大半空白。
-/
