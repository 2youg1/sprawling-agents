-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::thinking

规定 `provider::thinking`（`crates/gateway/src/provider/thinking.rs`）与它的编码分部 `provider::thinking::encoding`（`crates/gateway/src/provider/thinking/encoding.rs`）：一个（Endpoint，模型）提供哪些思考档、一次请求实际发哪一档、明说的档不在提供之列时怎样拒，以及一档在这一面上写成哪个字段。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面一节保留它的标签 §8-39，别处引作 `crates/gateway/Spec.lean §8-39`，决定引作 `gateway D<n>`。

模型证明五条跨过所有输入的性质：发出的档一定在提供之列（`the_sent_level_is_offered`）；什么都不提供的 offer 什么都不发（`an_offer_of_nothing_sends_nothing`）；存下的档被提供时原样发出、恒不夹到邻档（`an_offered_level_is_sent_as_stored`）；明说的档恰在不被提供时被拒（`admit_refuses_exactly_what_is_not_offered`）；「开启思考」只发给没有档位的模型（`thinking_on_is_sent_only_where_no_level_is_offered`）。

派生检查：`provider::thinking` 的 proptest `the_ladder_keeps_the_lean_properties` 走遍模型的输入空间——七个词的任意子集（`none` 由类型挡在集合之外）、开关三态、默认档缺席或七词之一、存下的档与明说的档各缺席或七词之一——对每个输入断言这五条；它先对一个不看存档、一律走默认规则的 `ask` 变红过。平台：纯判定，三个平台相同。
-/

/-!
### 8-39 `gateway::provider::thinking`：思考档的来源梯、实发的档与派活前的拒（形状 1 判定 ＋ 形状 6 数据面）

```rust
// provider::thinking —— 唯一权威：提供哪些档、实发哪一档、明说的档拒不拒
pub struct ThinkingOffer {
    pub levels: EffortSet,          // 城的固定升序；类型上不含 Effort::None
    pub on: Switch,                 // 没有档位的模型能不能被要求「开启思考」
    pub default: Option<Effort>,    // 上游或文档说的、什么都不发时用的档
    pub default_on: Option<bool>,   // 上游或文档说的、什么都不发时想不想
    pub from: OfferSource,          // 作答的那一档
    pub source: Option<&'static str>, // Preset 一档的文档地址
}
pub struct EffortSet(u8);           // Copy 位集；insert(Effort::None) 不改变集合
pub enum Switch { Allowed, Refused, Unknown }
pub enum OfferSource { Person, Upstream, Preset, Unknown }
pub enum Ask { Level(Effort), On }  // 一次请求实际发的；缺席即不写任何思考字段
impl ThinkingOffer {
    pub fn climb(upstream: Option<&ThinkingStatement>,
                 preset: Option<&'static PresetThinking>) -> ThinkingOffer;   // 先说者胜
    pub fn ask(&self, stored: Option<Effort>) -> Option<Ask>;
    pub fn admit(&self, explicit: Option<Effort>, model: &str) -> Result<(), AxError>;   // E_CONFIG_INVALID
}
pub fn offer_for(endpoint: &AttachedEndpoint, model: &str) -> ThinkingOffer;  // 读这一行的 ModelFacts 与预置表

// provider::thinking::encoding —— 一个 Ask 在这一面写成哪几个顶层字段
pub(crate) fn fields(ask: Ask, dialect: DialectKind, field: EffortField) -> Vec<(&'static str, Value)>;
```

- **梯子有四档，先说者胜**：人（Person）→ 上游的模型列表（Upstream，`ModelFacts.thinking`，`endpoint::models` 读）→ 预置表的 `thinking` 列（Preset，厂商文档，每行带地址）→ 无人说（Unknown）。与输出上限的梯子（§8-17）同一个次序与同一个理由：人的话不是推断，上游列表是对这一个（Endpoint，模型）最直接的陈述，文档其次。一档「说了」即整份作答，不跨档拼字段：把上游的档位与文档的默认档拼成一份，就是一个谁都没说过的组合。
- **人这一档今天没有录入面**：登记面还没有让人写下一个模型的档位集合的字段，所以 `climb` 从上游一档起；`OfferSource::Person` 是线上已有的词，录入面加进 `AttachedTuning` 的那一次改动给 `climb` 加上第一个参数与第一条臂。不先留一个恒为 `None` 的参数：一个从不被填的参数读起来像一档在作答。
- **集合的单位是（Endpoint，模型）**：同一个模型在两家供应方、或同一家的两面上提供的档可以不同，所以 offer 由一行登记与一个模型 id 算出，不按模型名全局查。
- **`levels` 在类型上不含 `none`**：关闭思考不是一档（IF-0 2.2）；`Effort::None` 只为读回旧账保留，它恒不被提供，于是一个存成 `none` 的旧值按默认规则处理。
- **实发的档（`ask`）**：存下的或继承来的档被提供时原样发出；否则 `high` 被提供时发 `high`（`kernel::consts_policy::DEFAULT_EFFORT`）；否则上游或文档说的默认档被提供时发它；否则一个没有档位、开关为 `Allowed` 的模型收到「开启思考」；否则什么都不写。恒不夹到邻档：不被提供的档不变成离它最近的那一档，而是走默认规则，因为「最近」在两家厂商那里深浅不同。
- **派活前的拒（`admit`）**：只有这一次请求明说的档（`/effort <level>` 或 `Dispatch.effort`）会被拒：不在 `levels` 里时答 `E_CONFIG_INVALID`，主题写这个模型，恢复语列出提供的档（没有时说这个模型没有思考档，去掉这一项再派）。存下的偏好恒不被拒（M-14）：换了模型，偏好留着，这一次按默认规则发。
- **编码归 `encoding` 一处**：一个 `Ask` 在三面上写成什么，见 `crates/gateway/spec/Dialect.lean` §8-1 的表与 §8-17 的 `EffortField`。dialect 自己不再写任何思考字段，于是「一档写成哪个字」只有这一处。
- **学到的拒绝推迟**（IF-0 §11）：今天不从 400 里学某一档被拒；所以没有「只让集合变小」这条性质的对象。旧的六个词读回原义由 kernel 的拼写测试守住（`crates/kernel/tests/spellings.rs`），不是这架梯子的性质。
-/

/-! D35 思考档由（Endpoint，模型）的 offer 决定，编码在发出前一处完成，dialect 不再写思考字段

**决定**：`Endpoint` 带着 `adapter_for` 按这一行登记算出的 `ThinkingOffer`；`Endpoint::wire_request` 先由 `ThinkingOffer::ask` 从请求的 `effort` 算出实发的 `Ask`，再由 `encoding::fields` 按这一面与主机的 `EffortField` 写进正文的顶层，然后才套会话缓存键与人的 `overrides`（人写了同一路径时人胜）。派活前的 `admit` 由 accounting 的选形状一步调用（`crates/accounting/src/worker/dispatching/session_shape.rs`），在写下任何文件或记录之前。

**理由**：编码要读 offer（没有档位的模型把默认写成「开启思考」，而这要知道它没有档位），而 offer 是登记与模型的事实，不是一次请求的形状；放在 dialect 里就要把 offer 塞进纯函数的参数，三面各写一份「这一档怎么拼」。原来三份「词→字符串」的映射（`Effort::as_str`、openai 与 responses 各一份）因此只剩 `Effort::as_str` 一份。

**被否**：①在 accounting 里把存档换成实发档再冻结：冻结的是城的偏好，换模型时偏好要留着（M-14），而实发档随 offer 变；②让 dialect 读 offer：dialect 是「同一份请求翻出同一串字节」的纯函数，加一个登记事实做参数就要每个 golden 带上它。

**重开参数**：某一面要求思考字段写在顶层以外（嵌在消息里），`fields` 的返回改成 JSON 指针。
-/

namespace Gateway.Provider.Thinking

/-- 城的思考档（`kernel::Effort`），声明次序即升序。 -/
inductive Effort where
  | none
  | minimal
  | low
  | medium
  | high
  | xhigh
  | max
  deriving DecidableEq, Repr

/-- 一个设置收不收（`provider::thinking::Switch`）。 -/
inductive Switch where
  | Allowed
  | Refused
  | Unknown
  deriving DecidableEq, Repr

/-- 一次请求实际发的（`provider::thinking::Ask`）；缺席即不写任何思考字段。 -/
inductive Ask where
  | Level (effort : Effort)
  | On
  deriving DecidableEq, Repr

/-- 一个（Endpoint，模型）的 offer，只留判定要读的三格。`levels` 不含 `none` 是类型的不变量（Rust 的 `EffortSet` 让 `insert(Effort::None)` 不改变集合）。 -/
structure Offer where
  levels : List Effort
  noneAbsent : Effort.none ∉ levels
  on : Switch
  default : Option Effort

/-- 没有档位、开关为 `Allowed` 时发「开启思考」，否则什么都不写。 -/
def switchOn (offer : Offer) : Option Ask :=
  if offer.levels = [] ∧ offer.on = .Allowed then some .On else none

/-- 存下的档不被提供时：`high`（`DEFAULT_EFFORT`），再是说出的默认档，再是开关。 -/
def fallback (offer : Offer) : Option Ask :=
  if Effort.high ∈ offer.levels then some (.Level .high)
  else
    match offer.default with
    | some level => if level ∈ offer.levels then some (.Level level) else switchOn offer
    | none => switchOn offer

/-- `ThinkingOffer::ask`。 -/
def ask (stored : Option Effort) (offer : Offer) : Option Ask :=
  match stored with
  | some level => if level ∈ offer.levels then some (.Level level) else fallback offer
  | none => fallback offer

/-- `ThinkingOffer::admit`：`true` 即放行。 -/
def admit (explicit : Option Effort) (offer : Offer) : Bool :=
  match explicit with
  | some level => decide (level ∈ offer.levels)
  | none => true

theorem switchOn_is_on {offer : Offer} {asked : Ask} (sent : switchOn offer = some asked) :
    asked = .On ∧ offer.levels = [] := by
  unfold switchOn at sent
  split at sent
  · rename_i h
    simp at sent
    exact ⟨sent.symm, h.1⟩
  · simp at sent

theorem fallback_level_is_offered {offer : Offer} {level : Effort}
    (sent : fallback offer = some (.Level level)) : level ∈ offer.levels := by
  unfold fallback at sent
  split at sent
  · rename_i h
    simp at sent
    exact sent ▸ h
  · split at sent
    · split at sent
      · rename_i stated _ h
        simp at sent
        exact sent ▸ h
      · exact absurd (switchOn_is_on sent).1 (by simp)
    · exact absurd (switchOn_is_on sent).1 (by simp)

/-- 发出的档一定在提供之列。 -/
theorem the_sent_level_is_offered (stored : Option Effort) (offer : Offer) (level : Effort)
    (sent : ask stored offer = some (.Level level)) : level ∈ offer.levels := by
  unfold ask at sent
  split at sent
  · split at sent
    · rename_i held _ h
      simp at sent
      exact sent ▸ h
    · exact fallback_level_is_offered sent
  · exact fallback_level_is_offered sent

/-- 什么都不提供的 offer（没有档位，开关不是 `Allowed`）什么都不发：缺席保持缺席。 -/
theorem an_offer_of_nothing_sends_nothing (stored : Option Effort) (offer : Offer)
    (empty : offer.levels = []) (closed : offer.on ≠ .Allowed) : ask stored offer = none := by
  have noSwitch : switchOn offer = none := by
    simp [switchOn, empty, closed]
  have noFallback : fallback offer = none := by
    unfold fallback
    simp only [empty, List.not_mem_nil, ite_false]
    cases offer.default <;> simp [noSwitch]
  cases stored <;> simp [ask, empty, noFallback]

/-- 存下的档被提供时原样发出：恒不夹到邻档。 -/
theorem an_offered_level_is_sent_as_stored (level : Effort) (offer : Offer)
    (offered : level ∈ offer.levels) : ask (some level) offer = some (.Level level) := by
  simp [ask, offered]

/-- 明说的档恰在不被提供时被拒；放行的明说档原样发出。 -/
theorem admit_refuses_exactly_what_is_not_offered (level : Effort) (offer : Offer) :
    (admit (some level) offer = false ↔ level ∉ offer.levels) ∧
    (admit (some level) offer = true → ask (some level) offer = some (.Level level)) := by
  constructor
  · simp [admit]
  · intro admitted
    simp [admit] at admitted
    exact an_offered_level_is_sent_as_stored level offer admitted

/-- 「开启思考」只发给没有档位的模型。 -/
theorem thinking_on_is_sent_only_where_no_level_is_offered (stored : Option Effort)
    (offer : Offer) (sent : ask stored offer = some .On) : offer.levels = [] := by
  have fromFallback : fallback offer = some .On → offer.levels = [] := by
    intro onSent
    unfold fallback at onSent
    split at onSent
    · simp at onSent
    · split at onSent
      · split at onSent
        · simp at onSent
        · exact (switchOn_is_on onSent).2
      · exact (switchOn_is_on onSent).2
  unfold ask at sent
  split at sent
  · split at sent
    · simp at sent
    · exact fromFallback sent
  · exact fromFallback sent

/-- 只说「能想」、不说档位的模型（例如一个只有开关的上游）收到「开启思考」，与存下的偏好无关。 -/
example : ask (some .max) ⟨[], by simp, .Allowed, none⟩ = some .On := by
  simp [ask, fallback, switchOn]

/-- 提供 `low`、`high`、`max` 的模型（DeepSeek 列表的形状）收到存下的 `medium` 时发 `high`，不是邻档 `low`。 -/
example : ask (some .medium) ⟨[.low, .high, .max], by simp, .Unknown, some .high⟩
    = some (.Level .high) := by
  simp [ask, fallback]

end Gateway.Provider.Thinking
