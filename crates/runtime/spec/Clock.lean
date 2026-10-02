-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::clock

规定 `clock`（`crates/runtime/src/` 下同名的文件）。一刻的唯一拼法、结果上的戳与它的发放规则、一跑的驱动最近读到的那一刻、按 UTC 选一段时间。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-10 runtime::clock（形状 1；纯格式化不采样）


```rust
// 关切时区的权威住 kernel::config::ClockZone（[clock] zones 属三层配置，city 仍拒写它，`crates/city/Spec.lean` §8-31）：
// FrozenConfig 携 clock_zones: Vec<ClockZone>；本模块只消费不定义（一个权威）。
pub fn iso(at: TimeMs) -> String;   // ISO 8601，UTC，到秒："2026-05-14T09:31:07Z"；一刻给人或模型读时的唯一拼法
pub fn parse_iso(raw: &str) -> Result<TimeMs, AxError>;   // iso 的逆：只收 iso 写出的那一种形状；其余 → E_INVALID_ARGS
pub struct ZoneEntry { pub id: String, pub offset_min: i32, pub local: String }   // local＝同一刻带偏移："2026-05-14T18:31:07+09:00"
pub struct ClockStamp { pub utc_ms: TimeMs, pub zones: Vec<ZoneEntry> }           // utc_ms＝读数本身，不按桶截；zones 只含配置的时区
impl ClockStamp { pub fn render(&self) -> String }   // 信封的时钟行："clock: 2026-05-14T09:31:07Z;"，其后每个时区 " <id> <local>;"
pub fn stamp(now: TimeMs, zones: &[ClockZone]) -> Result<ClockStamp, AxError>;   // zones > CLOCK_ZONES_MAX → E_INVALID_ARGS；空表即只报 UTC

pub struct StampGate { /* granularity、zones、last_bucket: Option<u64> —— 私有；last_bucket 兼任首发标记 */ }
impl StampGate {
    pub fn new(granularity: ClockStampGranularity, zones: Vec<ClockZone>) -> StampGate;   // 两值都取自这一跑的 FrozenConfig
    /// Emission rule: Off -> never; first result of the
    /// run -> once; Timestamped -> every result; Timeless -> only when the
    /// granularity bucket changed since the last emission.
    pub fn observe(&mut self, now: TimeMs, temporal: Temporal) -> Result<Option<ClockStamp>, AxError>;
}
```

- 历法纯整数（civil-from-days，无 chrono 依赖）：全程在 `i128` 上算，`u64` 毫秒加 `i32` 分钟偏移落不出它的界，所以格式化不会失败，`iso` 与 `render` 不带 `Result`；界证明携 `#[expect]`。**精度与频率分开**：戳一律到秒，粒度只决定 `Timeless` 工具多久带一次戳——同桶里第二条 `Timeless` 结果不带戳，`Timestamped` 每条都带。A18 零字节：Off 时 observe 恒 None。
- `iso` 是本 crate 里一刻的唯一文字形：时钟行、`status` 的 `now:` 行都经它；`parse_iso` 把同一种文字读回一刻（`view --since` 与 `--until`，`crates/sprawling/Spec.lean` §8-137），同一模块、同一精度。两者互逆：`parse_iso(&iso(t))` 是 `t` 去掉毫秒，`iso(parse_iso(s)?)` 是 `s`。`parse_iso` 只收 `YYYY-MM-DDTHH:MM:SSZ` 这 20 个字节：时区偏移、秒的小数、只有日期、小写的 `t`/`z`、历法里没有的那一天（2 月 30 日、非闰年的 2 月 29 日）、`24:00:00` 与闰秒 `:60`、1970 年之前，一律 `E_INVALID_ARGS`（action `read a UTC moment`，subject 是原文，recovery 给出正确写法）。日子在不在历法里，由把算出的日数经 `civil_from_days` 再算回来、比对年月日判定，历法只有那一份算法。
- 时区行仍在：`FrozenConfig.clock_zones` 在真城里恒空（城配置拒 `[clock] zones`），剧本仍可冻结出非空表，所以格式化保留，偏移写成 `+HH:MM`／`-HH:MM`。
-/

/-!
### 8-53 runtime::clock::ClockReading：一跑的驱动最近读到的那一刻（形状 2 值类型）


```rust
#[derive(Debug, Clone, Default)]
pub struct ClockReading(/* Arc<Mutex<Option<TimeMs>>> —— 私有 */);
impl ClockReading {
    pub fn keep(&self, at: TimeMs);          // 驱动的时钟钩子每读一次就记一次
    pub fn latest(&self) -> Option<TimeMs>;  // 最近记下的读数；还没读过为 None
}
// crates/runtime/src/tools/status.rs
impl StatusTool { pub fn clocked(self, clock: ClockReading) -> StatusTool; }   // `now:` 行从它现读
```

- **为什么要它**：`RunHooks::now` 是一跑里唯一的采样点，工具面不采样只收读数（§8-15 时间纪律），而 `ConcurrentInvoke` 与 `Tool::invoke` 的签名不带读数。装配层把 `now` 包一层，读到的每个值先 `keep` 再交给驱动；要报时的工具面（命令结果的戳、`status` 的 `now:`）读 `latest`。这样工具面报出的时刻恒是账本某一行的 `t`，不是第二个钟给出的另一个值。
- **读到的是哪一刻**：回合在调用工具面的 `account` 之前读这条调用的答复时刻（§8-15），所以工具面打包、打戳时读到的就是它，串行的调用与开头只读段的调用都一样；戳的秒数因此恒等于这条调用 `tool_result` 的 `t`。被准入直接答复的调用不经工具面的 `account`，不打戳。
- **`status` 的 `now:` 行**：`now: 2026-05-14T09:31:07Z`，没有读数时是 `now: not stamped`。它不看粒度：`now` 是 `status` 自己报的一栏，不是信封附件，关掉戳不该让模型问不出时间。`StatusSnapshot` 不再有 `now` 字段——快照在派发时冻结，冻结下来的时刻整跑都不动，而这一栏要的正是调用那一刻。
- 与 `ContextReading` 同形同理：一跑一个、克隆共享、写者一个。用 `Mutex<Option<TimeMs>>` 而不是原子整数：「还没读过」是一个状态，不该拿某个整数冒充；锁里只放一个 `Copy` 值、整值替换，所以中毒不留半写的值，读写都取锁里的值照常走。
-/

/-!
### 8-57 runtime::clock::UtcSpan：按 UTC 选一段时间（形状 2 值类型）


```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UtcSpan { /* since: Option<TimeMs>, until: Option<TimeMs> —— 私有 */ }
impl UtcSpan {
    /// until 不晚于 since → E_INVALID_ARGS（action `select a span of time`）；缺一端即那一端不设界。
    pub fn new(since: Option<TimeMs>, until: Option<TimeMs>) -> Result<UtcSpan, AxError>;
    pub fn since(&self) -> Option<TimeMs>;
    pub fn until(&self) -> Option<TimeMs>;
    /// 半开区间 [since, until)：since 那一刻在内，until 那一刻不在。
    pub fn contains(&self, at: TimeMs) -> bool;
}
```

- **读者**：`sprawling view --since` 与 `--until`（`crates/sprawling/Spec.lean` §8-137）；playback 的时间筛选接进来时读同一个值，不另写一份区间规则。`Default` 是两端都不设界，`contains` 恒真。
- **逐个时刻判断，不假定有序**：`contains` 只看给它的那一刻。账本的 `t` 不随 `seq` 单调（D5：并行只读段的开始时刻可以早于前一条的答复；墙钟也会回拨），所以按时间选行的读者每一行都问一次，不在第一条越过 `until` 的行处停下，也不二分。
- **矛盾的区间在构造时拒绝**：`until <= since` 的区间里没有任何一刻，按它选行只会安静地答出一段空历史；拒绝时 subject 写出两端的 `iso`。合法而恰好什么都没选中的区间照常答空。
-/

namespace Runtime.Clock

/-- 一件工具的结果与时间的关系（`kernel::Temporal`）：`Timestamped` 的结果离开它那一刻就说不清，`Timeless` 的结果隔多久读都一样。 -/
inductive Temporal where
  | Timestamped
  | Timeless
  deriving DecidableEq, Repr

/-- `StampGate` 的状态。`width` 是粒度的桶宽（毫秒），`none` 是 `Off`（`bucket_ms` 答 `None`）；`last_bucket` 是上一次观察落在的桶，`none` 兼任「这一跑还没有过结果」。时区表只进戳的渲染，不进这里的判定，所以模型不带它。 -/
structure StampGate where
  width : Option Nat
  last_bucket : Option Nat
  deriving DecidableEq, Repr

/-- `StampGate::observe` 里那一个布尔式：`Timestamped` 恒带戳，`Timeless` 只在桶变了时带，这一跑的第一条结果恒带。Rust 把它写成 `(… ) || last_bucket.is_none()`；这里按 `last_bucket` 先分两臂，值相同。 -/
def due (last : Option Nat) (bucket : Nat) : Temporal → Bool
  | .Timestamped => true
  | .Timeless => match last with
    | none => true
    | some seen => seen != bucket

/-! D8 戳从驱动最近的读数渲染，到秒，默认每分钟；回合在工具面打包之前读答复时刻

**决定**：结果上的时钟行渲染成 ISO 8601 UTC、到秒（`clock: 2026-05-14T09:31:07Z;`）；读数取自 `ClockReading`，即驱动在工具面打包之前最后一次读到的那一刻（§8-53）；`turn::wave` 在调用工具面的 `account` 之前读答复时刻，所以那一刻就是答复时刻；`CLOCK_STAMP_DEFAULT` 是 `Minute`，于是 `Timestamped` 工具每条结果都带戳，`Timeless` 工具只在分钟桶变了时带；`Off` 仍逐字节等于没有这个功能。

**理由**：模型要知道一条命令是什么时候答复的，靠回合的时间戳说不出来，一回合可以跨几分钟；一条跑两分钟的命令，戳若是开始时刻就早两分钟。读数经 `ClockReading` 来，一跑仍只有 `RunHooks::now` 一个采样点，戳上的秒数恒等于账本里某一行的 `t`；ISO 形状是人读、模型读与 `view` 解析共用的一种写法，到秒是因为分钟在一回合之内分不出先后。

**被否**：①工具面自己读一次钟打戳——那是一跑里的第二个采样点，戳上的秒数可以与它 `tool_result` 的 `t` 差一秒，计数时钟下的剧本也会多出采样而改变字节；②用 `admit` 收到的回合时间戳——一条命令的戳会早于模型给出这条调用的那一刻；③在回合的 `account` 里打戳——要 `turn::wave` 把打包移出工具面，而把答复读数挪到 `tools.account` 之前已经给出同一个秒数，工具面一行不改；④答复读数留在写 `tool_called`/`tool_result` 两行之前、工具面的 `account` 之后——戳就是开始时刻，一条长命令的戳早出它跑的那么久。读数挪动不改变采样的次数与次序，计数时钟下的剧本与并行对拍测试的字节不变。

**重开参数**：出现要本地时间的读者、且城配置开始受理 `[clock] zones` 时，时区行与偏移格式重议；工具面开始在 `account` 里做耗时可观的事（例如同步写盘）时，重议答复读数是否仍在它之前。
-/

/-- `StampGate::observe`：这一条结果带不带戳，以及观察之后的门。 -/
def observe (gate : StampGate) (now : Nat) (temporal : Temporal) : Bool × StampGate :=
  match gate.width with
  | none => (false, gate)
  | some width =>
    if due gate.last_bucket (now / width) temporal then
      (true, { gate with last_bucket := some (now / width) })
    else (false, gate)

/-- A18 零字节：`Off` 时什么都不带。 -/
theorem off_never_stamps (last : Option Nat) (now : Nat) (temporal : Temporal) :
    (observe ⟨none, last⟩ now temporal).1 = false := rfl

/-- 一跑的第一条结果恒带戳，所以开着的门不是一扇永远不开的门。 -/
theorem the_first_result_is_stamped (width now : Nat) (temporal : Temporal) :
    (observe ⟨some width, none⟩ now temporal).1 = true := by
  cases temporal <;> simp [observe, due]

/-- `Timestamped` 的结果每一条都带戳。 -/
theorem a_timestamped_result_is_always_stamped (width now : Nat) (last : Option Nat) :
    (observe ⟨some width, last⟩ now .Timestamped).1 = true := by
  simp [observe, due]

/-- 观察之后，门记下的恒是这一刻的桶，不论这一条带没带戳。 -/
theorem observing_records_the_bucket (width now : Nat) (last : Option Nat) (temporal : Temporal) :
    (observe ⟨some width, last⟩ now temporal).2 = ⟨some width, some (now / width)⟩ := by
  cases temporal with
  | Timestamped => simp [observe, due]
  | Timeless =>
    cases last with
    | none => simp [observe, due]
    | some seen =>
      by_cases same : seen = now / width
      · subst same
        simp [observe, due]
      · simp [observe, due, same]

/-- **`Timeless` 的结果一个桶里至多带一次戳。** 同一个桶里，无论前一条是哪一种、带没带戳，下一条 `Timeless` 的结果都不带。这是粒度存在的理由：频率归粒度，精度恒到秒。 -/
theorem a_timeless_result_is_stamped_at_most_once_per_bucket (width earlier later : Nat)
    (last : Option Nat) (first : Temporal) (sameBucket : later / width = earlier / width) :
    (observe (observe ⟨some width, last⟩ earlier first).2 later .Timeless).1 = false := by
  rw [observing_records_the_bucket]
  simp [observe, due, sameBucket]

/-- 桶变了，`Timeless` 的结果就带戳：门不会在一个新桶里沉默。 -/
theorem a_new_bucket_stamps_a_timeless_result (width now seen : Nat)
    (moved : seen ≠ now / width) :
    (observe ⟨some width, some seen⟩ now .Timeless).1 = true := by
  simp [observe, due, moved]

/-- 拒绝的稳定码：这里只有一种，`E_INVALID_ARGS`。 -/
inductive Code where
  | InvalidArgs
  deriving DecidableEq, Repr

/-- `clock::UtcSpan`：按 UTC 选一段时间，两端各可缺席。`until` 在 Lean 里是关键字，故写作 `«until»`。 -/
structure UtcSpan where
  since : Option Nat
  «until» : Option Nat
  deriving DecidableEq, Repr

/-! D13 时刻只有 `iso` 写出的那一种拼法可以读回，时间段是半开的、两端在构造时核对

**决定**：`parse_iso` 只收 `iso` 写出的形状（§8-10），`UtcSpan` 是 `[since, until)`，`until <= since` 在 `UtcSpan::new` 里拒绝（§8-57）。

**理由**：人从时钟行、`status` 的 `now:` 行或 `view` 的输出里抄下一刻，抄来的就是这一种形状；读回只收它，`iso` 与 `parse_iso` 互逆是一条可以逐值检验的性质，而不是两份各自宽容的规则。半开区间让相邻的两段（今天、明天）不重不漏，一行恰好落在交界那一毫秒时只属于后一段。矛盾的区间在值里拒绝，而不是让每个读者各自判一次：一个打错顺序的命令行得到一句错误，而不是一段看起来正常的空历史。

**被否**：①也收时区偏移并换算成 UTC——城的配置拒绝时区（`crates/city/Spec.lean` §8-31），收偏移就有了第二条关于时区的规则；②也收只有日期、或带小数秒的写法——每多一种写法就多一条「它等于哪一刻」的规定（一天的开始还是整天、截断还是舍入），而它们都不再与 `iso` 互逆；③闭区间 `[since, until]`——相邻两段在交界那一毫秒重叠；④`until <= since` 时答空——与「这段时间里什么都没发生」分不开。

**重开参数**：页面或居民工具要按本地日期选一天时，「一天」由读者的时区展开成一个 `UtcSpan`，那时再定展开规则住哪里；本模块仍只读 UTC。
-/

/-- `UtcSpan::new`：`until` 不晚于 `since` 的区间里没有任何一刻，构造时拒绝；缺一端即那一端不设界。 -/
def UtcSpan.new (since «until» : Option Nat) : Except Code UtcSpan :=
  match since, «until» with
  | some from_, some to =>
    if to ≤ from_ then .error .InvalidArgs else .ok ⟨since, «until»⟩
  | some _, none => .ok ⟨since, «until»⟩
  | none, some _ => .ok ⟨since, «until»⟩
  | none, none => .ok ⟨since, «until»⟩

/-- `UtcSpan::contains`：半开区间 `[since, until)`。Rust 的参数名是 `at`，那在 Lean 里是关键字，所以这里叫 `moment`。 -/
def UtcSpan.contains (span : UtcSpan) (moment : Nat) : Bool :=
  (match span.since with
    | none => true
    | some from_ => decide (from_ ≤ moment)) &&
  (match span.«until» with
    | none => true
    | some to => decide (moment < to))

/-- 矛盾的区间在值里拒绝，而不是答出一段看起来正常的空历史。 -/
theorem a_contradictory_span_is_refused (from_ to : Nat) (backwards : to ≤ from_) :
    UtcSpan.new (some from_) (some to) = .error .InvalidArgs := by
  simp [UtcSpan.new, backwards]

/-- `Default`：两端都不设界，每一刻都在里面。 -/
theorem an_unbounded_span_contains_every_moment (moment : Nat) :
    (UtcSpan.mk none none).contains moment = true := rfl

/-- 相邻的两段不重叠：交界那一毫秒只属于后一段。 -/
theorem adjacent_spans_share_no_moment (a b c moment : Nat) :
    ¬ ((UtcSpan.mk (some a) (some b)).contains moment = true ∧
       (UtcSpan.mk (some b) (some c)).contains moment = true) := by
  simp [UtcSpan.contains]
  omega

/-- 相邻的两段不漏：落在合起来那一段里的一刻，落在两段之一里。 -/
theorem adjacent_spans_leave_no_gap (a b c moment : Nat) :
    (UtcSpan.mk (some a) (some c)).contains moment = true →
      (UtcSpan.mk (some a) (some b)).contains moment = true ∨
        (UtcSpan.mk (some b) (some c)).contains moment = true := by
  simp [UtcSpan.contains]
  omega

end Runtime.Clock
