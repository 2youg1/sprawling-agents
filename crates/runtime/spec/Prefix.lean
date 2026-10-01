-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::prefix

规定 `prefix`、`prefix::breakpoint`、`prefix::segment`、`prefix::shape`、`prefix::warmth`（`crates/runtime/src/` 下同名的文件）。冻结前缀的四段、分段哈希、断点与保温。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-4 runtime::prefix（形状 5＋2）


```rust
pub enum SegmentSlot { City, Building, Resident, Run }   // 四段恒四，穷尽不扩
pub struct FrozenSegment { /* slot、bytes、hash —— 私有 */ }
impl FrozenSegment {
    /// The only way in: static bytes from frozen sources. Volatile types
    /// (TimeMs, usage, signals) have no conversion into this type — the
    /// absence of those impls is the isolation guarantee (15.3-4).
    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment;   // hash＝B3Hash::digest
    pub fn slot(&self) -> &SegmentSlot;  pub fn hash(&self) -> &B3Hash;  pub fn bytes(&self) -> &[u8];
}
pub struct FrozenPrefix { /* 四段 —— 私有 */ }
impl FrozenPrefix {
    /// Slot order is the type: city, building, resident, run. A mismatched
    /// slot in any position is E_INVALID_ARGS (fail-closed, no reorder).
    pub fn assemble(city: FrozenSegment, building: FrozenSegment,
                    resident: FrozenSegment, run: FrozenSegment) -> Result<FrozenPrefix, AxError>;
    pub fn segment_hashes(&self) -> [B3Hash; 4];
    /// 从 `bytes()` 重算四段哈希并与构造时记录的对拍；不等即拒绝。
    pub fn verified_segment_hashes(&self) -> Result<[B3Hash; 4], AxError>;
    pub fn prompt_payload(&self, plan: &BreakpointPlan) -> Result<Payload, AxError>;   // prompt_assembled 载荷
}
/// 同一断言作用在请求携带的四块上（`turn::call`）：四块、逐块 cache=true、逐块哈希对拍。
pub fn verified_system_hashes(system: &[SystemBlock], frozen: &[B3Hash; 4]) -> Result<[B3Hash; 4], AxError>;
// 载荷的键由 `kernel::event::record::PromptAssembled` 一处拼写，写读两端各经 `Payload::of` 与
// `Payload::read` 一扇门；本模块只提供值：`SegmentSlot::as_str` 给出 slot 名与 breakpoints 行，
// `SegmentSource::row` 给出 `PromptSource`，`build_segment` 的落选行给出 `PromptSkip`（reason 是
// 闭集 `SkipReason { Duplicate, Unreadable, NotUtf8, NoBudget }`）。
```

- 段序即缓存经济：类型把四段位置写死，断点与各段上限见 §8-6。
- **`segment_hashes()` 返回构造时缓存值，`verified_segment_hashes()` 才是权威。** 一个事实（这段字节的哈希）只有一个权威：`FrozenSegment::assembled` 在构造时算一次，每次派活前从将发的字节重算一次并与它比对。两者不一致意味着模型将读到的字节不是运行冻结的那一份，而那正是「同一事件序列逐字节重放」所依赖的东西，故拒绝而非警告。
- 分段哈希经 `B3Hash::digest`（kernel 唯一哈希产地）；A4（同输入同字节）由 golden 断言，A15 重建器随 S3。
- trybuild 反例：`FrozenSegment::from(TimeMs)`／把 TimeMs 传进 assemble —— 无转换路径，编译不过（ClockStamp 等类型落地后同规逐个加反例）。
-/

/-!
### 8-4-1 一句恢复语只许指向线上真有的动词（`prefix::segment::ANOTHER_ADDRESS`）


```rust
pub(crate) const ANOTHER_ADDRESS: &str =
    "send this task to another address, which opens a session of its own";
```

- **一个事实一个家**：「一个被冻住的会话怎么出去」只拼一遍，`prefix::segment::prefix_drifted` 与 `turn::report::shape_moved` 两条拒绝各自接上它们自己的解释；两份拼写会各自漂。
- **拒绝不得指名一个不存在的动词**：恢复语只指向每个客户端都有的那件事——把任务发到另一个地址。**一句指向不存在动词的恢复语，比没有恢复语更坏**：人按它去找，找不到，然后以为是自己没找到。
- **这句话随动词走**：线上有 `Command::OpenSession`；恢复语要改指它时，改的是这一个常量，而不是去两处各改一遍。
-/

/-!
### 8-4-2 runtime::prefix::warmth：每个前缀最近一次请求与它的续期（形状 1 判定）


```rust
/// 一个 session 的保温账：按四段哈希记每个前缀最近一次真实请求与它的 `kernel::keep_warm::CacheUse`。
pub struct Warmth { /* setting、lead_ms、kept: BTreeMap<[B3Hash; 4], Kept> —— 私有 */ }
impl Warmth {
    /// lead_ms 是调用方对所连 provider 实测的往返时长（kernel-SPEC §8-74）。
    pub fn new(setting: KeepWarm, lead_ms: u64) -> Warmth;
    /// 一条真实请求在 at_ms 发出。Off 时什么也不记。
    pub fn sent(&mut self, request: &ModelRequest, at_ms: u64);
    /// 最早一条续期的发出时刻；定时器睡到这一刻。None＝没有要续的，定时器不必醒。
    pub fn next_due(&self) -> Option<u64>;
    /// 把 now_ms 已到期的续期经 model 发出，返回每条续期的回执（调用方照常记 usage）。
    /// 过期不再续的前缀在这里被丢掉。模型失败原样返回，未发的前缀留待下一次。
    pub fn renew_due(&mut self, model: &mut dyn Model, now_ms: u64) -> Result<Vec<ModelReturn>, AxError>;
}
```

- **Off 时不记、不发**：`sent` 在 `KeepWarm::Off` 下不克隆请求，所以默认配置下每个 session 不为保温多占一字节，也不会有续期可发。默认配置下城不为保温多发一条请求，本模块的测试钉住这一点。
- 续期请求是该前缀最近一次真实请求原样重发，只把 `max_tokens` 压到 1：缓存按前缀字节命中，与输出上限无关，所以 1 个输出 token 是续期能付的最低价。
- 一个前缀只留最近一条请求：同一前缀后来的请求覆盖前一条，内存随前缀数而非请求数增长。
- 何时续、续几次全由 `kernel::keep_warm::renewal_due` 判定，本模块不另写第二个 TTL。
- 真实请求进账的门是 `Warmed`：它拥有一次 run 调用的 adapter，本身也是 `kernel::Model`，所以 turn loop 照旧只见 `&mut dyn Model`，每一次成功的调用（阻塞门或流式门）都在返回后按发出时刻记进 `Warmth`。失败的调用不记：provider 没有读到的前缀没有可续的缓存。

```rust
/// 拥有 adapter 的保温门；clock 取自 bin::assembly 的唯一采样点。
pub struct Warmed<C> { /* model: Box<dyn Model + Send>、warmth: Warmth、clock: C —— 私有 */ }
impl<C: FnMut() -> Result<TimeMs, AxError>> Warmed<C> {
    pub fn new(model: Box<dyn Model + Send>, setting: KeepWarm, clock: C) -> Warmed<C>;
    pub fn next_due(&self) -> Option<u64>;
    /// 经自己拥有的 adapter 发出 now_ms 已到期的续期。
    pub fn renew_due(&mut self, now_ms: u64) -> Result<Vec<ModelReturn>, AxError>;
}
impl<C: FnMut() -> Result<TimeMs, AxError>> Model for Warmed<C> { /* call、call_streaming */ }
```

- `lead_ms` 不是常量：`Warmed` 把每次经它发出的调用（真实请求与续期）的往返时长记为下一次续期的提前量，所以提前量跟着所连的 provider 与链路走，不按某一类机器调校。流式调用的往返含生成时长，只会让续期提前，不会让它晚于缓存过期。
- 未接线（§3）：run 结束后把 `Warmed` 留在 worker 上、在唯一 spawn 点的 attend 循环里按 `next_due` 醒来发续期、把续期的 usage 记成事件。
-/

/-!
### 8-21 runtime::prefix 目录化


| 文件 | 管什么 |
|---|---|
| `prefix.rs` | `SegmentSlot`／`FrozenSegment`／`SourceDoc`／`SegmentCaps`／`PrefixPlan`／`FrozenPrefix`，以及 `build_prefix`／`build_segment`／`truncation_marker`／`DOC_JOIN` 与 `system_blocks`／`segment_hashes`／`prompt_payload` |
| `prefix/tests.rs` | 冻结前缀保证什么：槽位次序、同输入同哈希、跨段去重与跳过入账、截断标记与字符边界、账本只记计划里的断点 |
-/

/-!
### 8-39 一段 prefix 自带它的来源，`prompt_assembled` 因此只有一条分支


`FrozenSegment` 自带来源注记，而不由 `build_prefix` 另行拼出；否则同一件事有两个作者——照计划装配出来的 prefix 写一种行，在城里按手边字节装配出来的 prefix 写另一种（或不写）。`replay::rebuild_prefix` 只读得懂前者，而页面要读的恰好是后者。

```rust
pub struct SegmentSource { pub addr: Address, pub kept: u64, pub dropped: u64 }
impl SegmentSource { pub fn whole(addr: Address, kept: u64) -> SegmentSource; }

impl FrozenSegment {
    pub fn new(slot: SegmentSlot, bytes: Vec<u8>) -> FrozenSegment;              // 无来源文档
    pub fn assembled(slot: SegmentSlot, bytes: Vec<u8>,
                     sources: Vec<SegmentSource>) -> FrozenSegment;
    pub fn sources(&self) -> &[SegmentSource];
}
```

**三条口径：**

1. **来源行只有一个作者。** `prompt_payload` 的两条分支合成一条：每一行的 `slot`／`hash`／`len`／`sources`／`skipped` 都从段本身取，`breakpoints` 恒上线。由此，在城里装配的 prefix 与照计划装配的 prefix 在同样的键下写同样的行，`rebuild_prefix` 读的仍是它一直在读的那四个键（`addr`／`kept`／`marker`／`dropped`）。
2. **`marker` 由 `dropped > 0` 派生而不是独立字段。** 两个互相蕴含的字段是两个可以互相矛盾的字段。
3. **没有来源文档的段写空表而不是省略键。** resident 段由身份与目录拼成，不出自任何文件；空表说的是「它不来自文档」，缺席的键说的是「不知道」。
4. **`breakpoints` 记本次请求实际发出的断点，而不是四个 slot 名。** 行由 `BreakpointPlan::breakpoints()` 拼出，值是段界的 slot 名与 `tail`。类型仍是 `Vec<String>`、键名不变、`#[serde(default)]`，所以写着 `["city","building","resident","run"]` 的记录照读：那是 slot 清单，其中 `run` 段界从未在请求里出现过。**被否**：保留 slot 清单另加一个 `plan` 键——那样账上仍有一行名为断点却不是断点的数据，读者要自己知道该信哪一个。

5. **`prompt_assembled` 每个 run 只写一条。** `turn::prompt::PromptRecord` 记着本 run 最后写下的载荷；`assemble` 照常算出本回合的载荷，与之相等就不写，不等才写并记住。prefix 在 run 内冻结，断点计划只随「对话是否为空」变，所以此后各回合的行是第一条的逐字拷贝；回合间真正会动的请求区域由 `prompt_shape_compared` 逐回合记，尾锚恒落在最后一条消息上，无须逐回合重述。比较的是载荷本身而不是「是不是第一回合」：哪天某个回合的载荷真的变了，账上就多一条，账本不会替一个请求声称它没带的断点。读者据此按 run 取段：`storage::attribution` 以 run 为键保存段权重（storage-SPEC），`sprawling::views::prefix` 读一跑的第一条。旧账每回合一条，照读，因为同一 run 的各条相同。**被否**：之后的回合写一条引用首条的短记录——它不携任何读者需要的事实，只多一行链。

**被否**：让页面在被问的那一刻重新装配一次 prefix 去拿来源。此刻的文件不是当时的文件，那样画出来的是一份没有任何人收到过的提示。
-/

/-! D1 定规：来源行不带摘要生产者指纹

**决定**：`prompt_assembled` 的来源行只记 `{addr, kept, marker, dropped}`；没有 `SummaryProducer`，也没有铸印它的 `compaction::producer::mint`。

**理由**：本仓库不做 LLM 压缩，没有任何路径产出摘要文档，生产里每个 `SourceDoc` 的指纹恒为缺席；一个没有写入者的字段让读者以为账本记下了一件它从不记的事，而铸印函数只有测试调用。

**被否**：保留字段与 `mint` 等将来接上——死机制照样要维护、要进公开面，而接上时的口径（谁写的、第几代）应由那时真实的摘要生产者决定，不是预先猜好。

**重开参数**：出现一条把模型写的摘要放进前缀的生产路径时，重开本条，指纹随那条路径一起设计。
-/
