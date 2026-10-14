-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::keep_warm

规定 `kernel::keep_warm`（`crates/kernel/src/keep_warm.rs`）：缓存保温的设置与判定。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

判定是一台小状态机：一扇门见到真实使用与唤醒，按 `renewal_due` 决定此刻发不发续期。下面的模型对门可能见到的每一条轨迹证明两条性质，`kernel::keep_warm` 旁的 proptest `every_trace_keeps_the_lean_properties` 从它们派生。
-/

/-!
### 8-74 `kernel::keep_warm`：缓存保温的设置与判定（形状 1 判定）

**设置与判定住 kernel，因为读它的两处互不依赖**：`city::config_layers` 把 `[cache] keep_warm` 读成这个值，runtime 按它排续期；两个 crate 只共有 kernel，所以枚举的拼法与「何时续期」只能在这里各有一处定义。

```rust
/// 城的保温设置。默认 Off：Off 时本模块不排任何续期，城不会多发一条请求。
#[derive(Default)] #[serde(rename_all = "snake_case")]   // 缺省 Off
pub enum KeepWarm { Off, FiveMinute }
/// 一个前缀的缓存状态：最近一次真实请求带它发出的时刻（毫秒），与这次使用欠不欠一次续期。
pub struct CacheUse { /* used_ms、renewal: Renewal —— 私有 */ }
enum Renewal { Owed, Spent }                              // 私有；真实使用后 Owed，续期发出后 Spent
impl CacheUse {
    pub fn sent(at_ms: u64) -> CacheUse;                  // 一条真实请求带着它发出：欠一次续期
    pub fn renewed(self) -> CacheUse;                     // 这次使用的那一次续期已发出
}
/// 下一条续期请求的发出时刻；None＝不续期。lead_ms 是调用方对所连 endpoint 测得的提前量。
pub fn renewal_due(setting: KeepWarm, cache: CacheUse, lead_ms: u64) -> Option<u64>;
```

- 判定：`FiveMinute` 且这次使用还欠续期时，续期时刻＝`used_ms + PROMPT_CACHE_TTL_SECS·1000 − PROMPT_CACHE_RENEWAL_MARGIN_SECS·1000 − lead_ms`（饱和减；`used_ms + TTL` 溢出 `u64` 即不续期）。续期发出后 `CacheUse` 记为 `Spent`，`renewal_due` 对它答 `None`，直到下一次真实使用。所以「一次真实使用至多一次续期」是 `CacheUse` 里记下的事实，而不是从时刻推出来的（D59）。
- 「最近用过」与缓存寿命是同一个常数 `consts_external::PROMPT_CACHE_TTL_SECS`，余量是它旁边的 `PROMPT_CACHE_RENEWAL_MARGIN_SECS`；两者只在这里相减，不另立第二个 300。
- `lead_ms` 的含义：从城发出请求到 provider 开始处理它的时间，按 endpoint 实测，由调用方给（`crates/runtime/Spec.lean` §8-4-2，runtime D98）。provider 的寿命从它开始处理请求时起算，`used_ms` 是城发出的时刻，早于那一刻；所以续期提前 `lead_ms` 发出，只要它自己到达 provider 不比 `lead_ms` 慢，就在保证寿命之内开始。不在这里写死一个网络余量：慢链路与快链路要的提前量不同。
- 余量防的是 `lead_ms` 量不到的延迟：到期那一刻定时器醒得晚一点、交出续期的线程起得晚一点。
- 花费只观察、不设门限：续期请求照常记 usage，本模块不读余额也不拦。一次续期的 usage 报出非零的 `cache_write_tokens`，就是它到得太晚、provider 重写了前缀；这条读法住 `kernel::event::record::CacheRenewed` 的 `outcome`，成本视图从 `storage::Attribution` 的 `renewals` 读到次数（`crates/storage/Spec.lean` §8-7）。
- 设置按城→楼→居民三层梯解析，下层覆盖上层，一层也没说＝`Off`（`crates/city/Spec.lean` §8-4 `[cache]` 一节）。它不进 `FrozenConfig`：续期发生在两次 run 之间，不属于任何一次 run 的冻结面。
- 记账并经 `kernel::Model` 发出续期的是 `runtime::prefix::warmth`（`crates/runtime/Spec.lean` §8-4-2）。续期由 `accounting::worker::keeping_warm` 在房间落地后按 `renewal_due` 发出，每次续期写一行 `cache_renewed`（`crates/sprawling/Spec.lean` §8-93、§8-112）。
-/

namespace Kernel.KeepWarm

/-- 与 `kernel::keep_warm::KeepWarm` 逐变体同名。 -/
inductive KeepWarm where
  | Off
  | FiveMinute
  deriving DecidableEq, Repr

/-- 与 `kernel::keep_warm` 私有的 `Renewal` 逐变体同名。 -/
inductive Renewal where
  | Owed
  | Spent
  deriving DecidableEq, Repr

/-- `CacheUse`：真实使用发出的时刻，与这次使用欠不欠续期。 -/
structure CacheUse where
  used : Nat
  renewal : Renewal
  deriving DecidableEq, Repr

/-- `CacheUse::sent`。 -/
def CacheUse.sent (atMs : Nat) : CacheUse := ⟨atMs, .Owed⟩

/-- `CacheUse::renewed`：发出时刻不动，只记下续期已发出。 -/
def CacheUse.renewed (cache : CacheUse) : CacheUse := { cache with renewal := .Spent }

/-- `renewal_due`。`ttl` 与 `margin` 是 `PROMPT_CACHE_TTL_SECS` 与 `PROMPT_CACHE_RENEWAL_MARGIN_SECS` 的毫秒值；`Nat` 的减法是饱和减，与 Rust 的 `saturating_sub` 同。Rust 在 `used_ms + TTL` 溢出 `u64` 时答 `None`，模型的 `Nat` 不溢出，那一支只会少续，两条性质对它照样成立。 -/
def renewalDue (ttl margin : Nat) (setting : KeepWarm) (cache : CacheUse) (lead : Nat) : Option Nat :=
  match setting, cache.renewal with
  | .FiveMinute, .Owed => some (cache.used + ttl - margin - lead)
  | .FiveMinute, .Spent => none
  | .Off, _ => none

/-- 排出的续期时刻恒不晚于使用时刻加寿命减余量，不论提前量是多少。 -/
theorem a_due_renewal_leaves_the_margin (ttl margin : Nat) (setting : KeepWarm) (cache : CacheUse)
    (lead due : Nat) (planned : renewalDue ttl margin setting cache lead = some due) :
    due ≤ cache.used + ttl - margin := by
  unfold renewalDue at planned
  split at planned
  · cases planned
    omega
  · cases planned
  · cases planned

/-- 排得出续期的使用都还欠着续期。 -/
theorem only_an_owed_use_plans_a_renewal (ttl margin : Nat) (setting : KeepWarm) (cache : CacheUse)
    (lead due : Nat) (planned : renewalDue ttl margin setting cache lead = some due) :
    cache.renewal = .Owed := by
  unfold renewalDue at planned
  split at planned
  · rename_i owed
    exact owed
  · cases planned
  · cases planned

/-- 门见到的事：一条带着这个前缀的真实请求在 `atMs` 发出；门在 `nowMs` 醒来，这时所连 endpoint 的提前量读作 `lead`。 -/
inductive Event where
  | used (atMs : Nat)
  | woke (nowMs lead : Nat)
  deriving Repr

/-- 门发出的一次续期：它为哪一次使用续，排在什么时刻。 -/
structure Sent where
  used : Nat
  due : Nat
  deriving Repr

/-- 一扇门：前缀的缓存状态、自最近一次真实使用以来发出的续期数，与它发出过的每一次续期。续期数由走轨迹的一方数出，不读 `CacheUse`。 -/
structure Door where
  cache : Option CacheUse
  sinceUse : Nat
  sent : List Sent

/-- 还没见过任何请求的门。 -/
def Door.empty : Door := ⟨none, 0, []⟩

/-- 门对一件事的反应：真实使用重置缓存状态；醒来时 `renewalDue` 排出的时刻已到，就发一次续期并记下 `renewed`。 -/
def Door.step (ttl margin : Nat) (setting : KeepWarm) (door : Door) : Event → Door
  | .used atMs => { door with cache := some (CacheUse.sent atMs), sinceUse := 0 }
  | .woke nowMs lead =>
    match door.cache with
    | none => door
    | some cache =>
      match renewalDue ttl margin setting cache lead with
      | none => door
      | some due =>
        if due ≤ nowMs then
          { cache := some cache.renewed, sinceUse := door.sinceUse + 1,
            sent := ⟨cache.used, due⟩ :: door.sent }
        else door

/-- 门走完一条轨迹。 -/
def Door.walk (ttl margin : Nat) (setting : KeepWarm) (door : Door) (events : List Event) : Door :=
  events.foldl (Door.step ttl margin setting) door

/-- 走轨迹时保持的不变量：发出过的每次续期都留着余量；自上次使用以来至多一次续期，发过一次时缓存记着 `Spent`。 -/
def Door.Sound (ttl margin : Nat) (door : Door) : Prop :=
  (∀ s ∈ door.sent, s.due ≤ s.used + ttl - margin) ∧ door.sinceUse ≤ 1 ∧
    (door.sinceUse = 1 → ∃ cache, door.cache = some cache ∧ cache.renewal = .Spent)

theorem empty_sound (ttl margin : Nat) : Door.empty.Sound ttl margin := by
  refine ⟨?_, ?_, ?_⟩
  · intro s held
    cases held
  · simp [Door.empty]
  · intro one
    simp [Door.empty] at one

theorem step_sound (ttl margin : Nat) (setting : KeepWarm) (door : Door)
    (sound : door.Sound ttl margin) (event : Event) :
    (door.step ttl margin setting event).Sound ttl margin := by
  obtain ⟨margins, atMostOne, spent⟩ := sound
  cases event with
  | used atMs =>
    refine ⟨margins, ?_, ?_⟩
    · simp [Door.step]
    · intro one
      simp [Door.step] at one
  | woke nowMs lead =>
    unfold Door.step
    cases cached : door.cache with
    | none => exact ⟨margins, atMostOne, spent⟩
    | some cache =>
      simp only
      cases planned : renewalDue ttl margin setting cache lead with
      | none => exact ⟨margins, atMostOne, spent⟩
      | some due =>
        simp only
        split
        · have owed := only_an_owed_use_plans_a_renewal ttl margin setting cache lead due planned
          have fresh : door.sinceUse = 0 := by
            cases Nat.lt_or_ge door.sinceUse 1 with
            | inl below => omega
            | inr above =>
              obtain ⟨held, isHeld, isSpent⟩ := spent (by omega)
              rw [cached] at isHeld
              cases isHeld
              rw [owed] at isSpent
              cases isSpent
          refine ⟨?_, ?_, ?_⟩
          · intro s held
            cases held with
            | head => exact a_due_renewal_leaves_the_margin ttl margin setting cache lead due planned
            | tail _ rest => exact margins s rest
          · simp only
            omega
          · intro _
            exact ⟨cache.renewed, rfl, rfl⟩
        · exact ⟨margins, atMostOne, spent⟩

theorem walk_sound (ttl margin : Nat) (setting : KeepWarm) (events : List Event) :
    ∀ door : Door, door.Sound ttl margin → (door.walk ttl margin setting events).Sound ttl margin := by
  induction events with
  | nil =>
    intro door sound
    exact sound
  | cons event rest more =>
    intro door sound
    exact more _ (step_sound ttl margin setting door sound event)

/-- **一次真实使用至多一次续期**：对任意轨迹——任意的使用时刻、唤醒时刻与每次唤醒读到的提前量——自最近一次真实使用以来，门至多发出一次续期。 -/
theorem at_most_one_renewal_per_use (ttl margin : Nat) (setting : KeepWarm) (events : List Event) :
    (Door.empty.walk ttl margin setting events).sinceUse ≤ 1 :=
  (walk_sound ttl margin setting events Door.empty (empty_sound ttl margin)).2.1

/-- **每次续期都留着余量**：对任意轨迹，门发出的每一次续期都排在它所续的那次使用加寿命减余量之前或正当其时。 -/
theorem every_renewal_leaves_the_margin (ttl margin : Nat) (setting : KeepWarm) (events : List Event) :
    ∀ s ∈ (Door.empty.walk ttl margin setting events).sent, s.due ≤ s.used + ttl - margin :=
  (walk_sound ttl margin setting events Door.empty (empty_sound ttl margin)).1

theorem off_step_sends_nothing (ttl margin : Nat) (door : Door) (event : Event) :
    (door.step ttl margin .Off event).sent = door.sent := by
  cases event with
  | used atMs => rfl
  | woke nowMs lead =>
    unfold Door.step
    cases door.cache with
    | none => rfl
    | some cache => rfl

/-- **默认设置不发任何续期**：`Off` 下任意轨迹走完，门一次续期也没发过。 -/
theorem off_sends_nothing (ttl margin : Nat) (events : List Event) :
    ∀ door : Door, (door.walk ttl margin .Off events).sent = door.sent := by
  induction events with
  | nil => intro door; rfl
  | cons event rest more =>
    intro door
    show ((door.step ttl margin .Off event).walk ttl margin .Off rest).sent = door.sent
    rw [more, off_step_sends_nothing]

end Kernel.KeepWarm

/-! D59 一次真实使用至多一次续期，记在 `CacheUse` 里；续期时刻扣一个固定余量

**决定**：`CacheUse` 带一个私有的 `Renewal { Owed, Spent }`：`sent` 置 `Owed`，`renewed` 置 `Spent`，`renewal_due` 对 `Spent` 答 `None`。续期时刻是 `used_ms + TTL − margin − lead_ms`，`margin` 是 `consts_external::PROMPT_CACHE_RENEWAL_MARGIN_SECS`，定义在 `PROMPT_CACHE_TTL_SECS` 旁边，只在 `renewal_due` 里被减。上面两条定理是这条决定要守的性质。

**理由**：先前的上限是从时刻推出来的——第二次续期的时刻离真实使用超过一个 TTL 就不发。续期之后提前量换成那次续期的往返；往返一长（名额门前排队就够），`refreshed − lead ≤ used` 成立，门就再续一次。把「续过了」记成一个状态，性质就不再依赖提前量取什么值，模型对任意提前量证得出来。余量防的是提前量量不到的延迟：定时器醒得晚、交出续期的线程起得晚。它不随链路变，所以是常数；按寿命的比例取余量，在一小时档上会白白丢掉几分钟。

**被否**：近期往返的高分位窗口（P99）作提前量——样本若是整次调用的时长，分位只会让续期更早；样本若是到 provider 开始处理的时间，固定余量已经盖住了它的抖动，窗口只多出要保存与要校准的状态；只有成本视图里迟到续期的次数表明固定余量不够时才重议。保留 `refreshed_ms` 让续期也能再排一次——每次真实使用只换一次续期是这个设置对人承诺的花费上限。

**重开参数**：`storage::Attribution` 的 `renewals.late` 在开着保温的城里持续不为零；或加入一小时档，那时重议余量是否随档位变。
-/
