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
    /// 一栋楼里每个居民逐字节带着的前几块：`SegmentSlot::shared_by_building` 为真的那几格（D97）。
    pub fn shared_part(&self) -> SharedPrefix;
}
impl SegmentSlot { pub fn shared_by_building(self) -> bool; }   // City、Building 为真
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
pub struct Warmth { /* setting、kept: BTreeMap<[B3Hash; 4], Kept> —— 私有 */ }
impl Warmth {
    pub fn new(setting: KeepWarm) -> Warmth;
    /// 一条真实请求在 at_ms 发出。Off 时什么也不记。
    pub fn sent(&mut self, request: &ModelRequest, at_ms: u64);
    /// 按 lead 读到的提前量，最早一条续期的发出时刻；定时器睡到这一刻。None＝没有要续的，定时器不必醒。
    pub fn next_due(&self, lead: &Lead) -> Option<u64>;
    /// 把 now_ms 已到期的续期逐条交给 send，返回每条续期的回执（调用方照常记 usage）。
    /// 不再欠续期的前缀在这里被丢掉。send 失败原样返回，未发的前缀留待下一次。
    pub fn renew_due(&mut self, lead: &Lead, now_ms: u64,
                     send: impl FnMut(&ModelRequest) -> Result<ModelReturn, AxError>) -> Result<Vec<ModelReturn>, AxError>;
}
/// 一个 endpoint 的提前量：从城发出请求到 provider 开始处理它的时间，最近一次的读数（毫秒）。
/// clone 出来的句柄共享同一个读数，所以连同一 endpoint 的每扇门读到的是同一个数。
#[derive(Debug, Clone, Default)]
pub struct Lead { /* Arc<AtomicU64> —— 私有 */ }
impl Lead {
    pub fn ms(&self) -> u64;                 // 还没有读数时是 0
}
```

- **Off 时不记、不发**：`sent` 在 `KeepWarm::Off` 下不克隆请求，所以默认配置下每个 session 不为保温多占一字节，也不会有续期可发。默认配置下城不为保温多发一条请求，本模块的测试钉住这一点。
- 续期请求是该前缀最近一次真实请求原样重发，只把 `max_tokens` 压到 1：缓存按前缀字节命中，与输出上限无关，所以 1 个输出 token 是续期能付的最低价。
- 一个前缀只留最近一条请求：同一前缀后来的请求覆盖前一条，内存随前缀数而非请求数增长。
- 何时续、续几次全由 `kernel::keep_warm::renewal_due` 判定（`crates/kernel/Spec.lean` §8-74，kernel D59），本模块不另写第二个 TTL 或余量；续期发出后前缀的 `CacheUse` 记成 `renewed`，于是它不再欠续期。
- 真实请求进账的门是 `Warmed`：它拥有一次 run 调用的 adapter，本身也是 `kernel::Model`，所以 turn loop 照旧只见 `&mut dyn Model`，每一次成功的调用（阻塞门、流式门或推测门）都在返回后按发出时刻记进 `Warmth`。失败的调用不记：provider 没有读到的前缀没有可续的缓存。

```rust
/// 拥有 adapter 的保温门；clock 取自 bin::assembly 的唯一采样点，lead 是它所连 endpoint 的那一个读数。
pub struct Warmed<C> { /* model: Box<dyn Model + Send>、warmth: Warmth、clock: C、lead: Lead —— 私有 */ }
impl<C: FnMut() -> Result<TimeMs, AxError>> Warmed<C> {
    pub fn new(model: Box<dyn Model + Send>, setting: KeepWarm, clock: C, lead: Lead) -> Warmed<C>;
    pub fn next_due(&self) -> Option<u64>;
    /// 经自己拥有的 adapter 发出 now_ms 已到期的续期，每条续期的往返写进 lead。
    pub fn renew_due(&mut self, now_ms: u64) -> Result<Vec<ModelReturn>, AxError>;
}
impl<C: FnMut() -> Result<TimeMs, AxError>> Model for Warmed<C> { /* call、call_streaming、call_speculating 三扇门都转发给 adapter；prefix_held 原样转交它包着的模型 */ }
```

- **提前量量的是它被用来做的事**（D98）：从发出到 provider 开始处理请求。`Warmed` 只在两处读它：一次续期的往返（续期只要 1 个 token，回答紧跟 prefill），与一次真实调用从发出到第一个正文或推理增量的时间。后者还含 prefill 与第一个 token，所以只会偏长、让续期偏早，不会让它晚于寿命。真实调用的整段时长从不当提前量：它含整段生成，一次 200 秒的调用会把续期排到落地之前。阻塞门没有增量，它的调用不改读数。
- 读数按 endpoint 存，不按门存：网络路径与名额门属于 endpoint，而一扇门每次真实使用只见到一次续期，单扇门攒不出读数。门由调用方给它所连 endpoint 的那一个 `Lead`；`next_due` 每次按当时的读数算，所以别的门量出的新读数也挪动这扇门的到期时刻。
- 谁调 `renew_due`：run 落地后 `Warmed` 按房间留在 accounting 的 worker 上，记账线程按 `next_due` 醒来，把到期的门整扇交给一条自己的线程，那条线程调 `renew_due`，门连同回执回到记账线程，续期的 usage 记成 `cache_renewed`（`crates/sprawling/Spec.lean` §8-112、§8-42-9）。所以 `Warmed` 要能整扇移到另一条线程上：它的 adapter 是 `Box<dyn Model + Send>`，时钟由调用方给，accounting 交的是 `Send` 的那一种；`Lead` 的读数是原子的，lane 上的门与续期线程上的门可以同时写它。
- **验收**：本模块旁的 `a_long_call_leaves_its_renewal_due_a_lifetime_after_it_was_sent`——provider 的真实调用耗时 200 秒，落地时不发续期，续期排在「发出 + TTL − 续期往返 − 余量」；`a_streamed_call_reads_the_lead_at_its_first_word`——流式调用的提前量是到第一个增量的时间；`doors_to_one_endpoint_share_its_lead`——一扇门续期量出的往返挪动同一 endpoint 另一扇门的到期时刻。
-/

/-! D98 提前量是「发出到 provider 开始处理」的时间，按 endpoint 存一个读数

**决定**：`Lead` 是一个 endpoint 的读数，clone 的句柄共享它；`Warmed` 在续期往返与真实调用的第一个增量两处写它，整段调用不写；accounting 按 endpoint 名给每扇门同一个 `Lead`（`crates/sprawling/Spec.lean` §8-112）。最近一次读数直接覆盖上一次，不留样本窗口。

**理由**：provider 的寿命从它开始处理请求时起算，提前量要盖住的正是续期从城到 provider 开始处理的这一段。先前门把每次调用的整段时长记作提前量，真实调用之后起作用的续期几乎总是带着上一次调用的时长出发：保温窗口缩短这次调用的时长，调用超过半个寿命时到期时刻早于落地。两处读数都含排队与 prefill，只会偏长；偏长的代价是续期早发几秒，偏短的代价是一次重写前缀，所以宁长勿短。

**被否**：按门存——一扇门每次真实使用只见到一次续期，它的读数大多来自真实调用，续期的往返用不上；整段调用时长打折或取分位——它量的不是那一段，折扣多少都是猜；gateway 把流的第一个事件往上传——更准，但要在 `kernel::Model` 上加一扇门，而第一个增量已经在门的手里，误差只往早的方向偏。

**重开参数**：gateway 把流的第一个事件交出来；或成本视图里迟到续期的次数表明读数偏短。
-/

/-! D97 保温门转发 `Model` 的每一扇调用门，不让任何一扇落到 trait 的缺省实现

**决定**：`Warmed` 自己实现 `call`、`call_streaming`、`call_speculating`，每一扇都经同一个 `recorded` 转发给它拥有的 adapter，并照同一口径记账；`provider_account`、`account_roster`、`select_account` 也照样转发。turn loop 走哪扇门由 `turn::speculation::Generating` 决定，门只管把那扇门原样交给 adapter。

**理由**：`kernel::Model` 的缺省方法对「没有流」「说不出调用何时完整」的 adapter 是诚实的，所以一扇门漏转发时什么都不失败，只是变慢：缺省的 `call_speculating` 丢掉提前调用的接收端、改调 `call_streaming`，生产路径上的只读工具于是全等整段回答写完才开始。漏掉的覆盖没有 lint 或 `#[deny]` 能抓，守住它的是 `crates/runtime/tests/run_driver.rs` 里经 `Warmed` 驱动替身模型、看只读调用在模型返回之前开始的那条测试。

**被否**：只转发阻塞门与流式门、推测门交给缺省——那正是让提前读在城里失效的形状；另造一个包装 trait 列出「必须转发」的方法——它在 `Model` 之外又写一份方法清单，两份会各自漂，而 `Model` 只有这一个包装者。

**重开参数**：`kernel::Model` 增加一扇带行为的调用门时，这扇门同在 `Warmed` 上转发，测试同样补一条；出现第二个包装 `Model` 的门时，重议是否值得一个机械的守卫。
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

5. **`prompt_assembled` 每个 run 只写一条。** `turn::prompt::PromptRecord` 记着本 run 最后写下的载荷；`assemble` 照常算出本回合的载荷，与之相等就不写，不等才写并记住。prefix 在 run 内冻结，断点计划只随「对话是否为空」变，所以此后各回合的行是第一条的逐字拷贝；回合间真正会动的请求区域由 `prompt_shape_compared` 逐回合记，尾锚恒落在最后一条消息上，无须逐回合重述。比较的是载荷本身而不是「是不是第一回合」：哪天某个回合的载荷真的变了，账上就多一条，账本不会替一个请求声称它没带的断点。读者据此按 run 取段：`storage::attribution` 以 run 为键保存段权重（`crates/storage/Spec.lean` §8-7），`sprawling::views::prefix` 读一跑的第一条。旧账每回合一条，照读，因为同一 run 的各条相同。**被否**：之后的回合写一条引用首条的短记录——它不携任何读者需要的事实，只多一行链。

**被否**：让页面在被问的那一刻重新装配一次 prefix 去拿来源。此刻的文件不是当时的文件，那样画出来的是一份没有任何人收到过的提示。
-/

/-! D1 定规：来源行不带摘要生产者指纹

**决定**：`prompt_assembled` 的来源行只记 `{addr, kept, marker, dropped}`；没有 `SummaryProducer`，也没有铸印它的 `compaction::producer::mint`。

**理由**：本仓库不做 LLM 压缩，没有任何路径产出摘要文档，生产里每个 `SourceDoc` 的指纹恒为缺席；一个没有写入者的字段让读者以为账本记下了一件它从不记的事，而铸印函数只有测试调用。

**被否**：保留字段与 `mint` 等将来接上——死机制照样要维护、要进公开面，而接上时的口径（谁写的、第几代）应由那时真实的摘要生产者决定，不是预先猜好。

**重开参数**：出现一条把模型写的摘要放进前缀的生产路径时，重开本条，指纹随那条路径一起设计。
-/

/-! D97 一栋楼共享前缀的哪几块，由段格说出

**决定**：`SegmentSlot::shared_by_building` 是唯一的答案：City 与 Building 为真，Resident 与 Run 为假，穷尽地写在格上。`FrozenPrefix::shared_part` 数出前面连续为真的格，交给回合请求的 `ChatRequest.shared`（kernel D60）作 `SharedPrefix::Leading(n)`，今天 n 是 2。回合之外造的请求（探测、顾问、OCR、测试）写 `Unshared`。

**理由**：Resident 段由身份与目录拼成（`prefix::segment`），同一栋楼的两个居民不同；Run 段属于一次 run。四段的次序与内容是本 crate 的事实，gateway 只读这个数，不自己去数带断点的块，否则同一件事在两个 crate 里各拼一次。

**被否**：①写一个常量 2：段格改了次序或多了一格，常量不跟着变；②按每块的 `cache` 标记数：Resident 段也带标记，标记说的是「在这里打断点」，不是「这一段是共享的」。

**重开参数**：一栋楼里的居民开始共享 Resident 段的一部分（例如共同的目录），或段格多出一格。
-/
