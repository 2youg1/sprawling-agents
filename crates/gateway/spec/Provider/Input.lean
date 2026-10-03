-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::provider::input

规定 `provider::input`（`crates/gateway/src/provider/input.rs`）：一个模型收得下什么：人说的、目录、预置表、`Text`，先说者胜。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。
-/

/-!
### 8-37 一个模型收得下什么：`gateway::provider::input`（形状 1 判定）

`ocr` 能不能用在一个模型上，取决于这个模型登记时的 `InputKinds`（§8-34「能不能把图交给这个模型，由端点判」）。钉版目录里收图的只有一行，而预置表的每个模型行都写着 `input`，所以只读目录会把一个厂商文档写明能读图的模型按 `Text` 登记，端点拒绝每一张图。上下文窗口与输出上限是一架从目录到预置表再到缺省的梯子（`window_for`、`OutputCeiling::resolve`，§8-17）；「收得下什么」走同一种梯子，判定只写在这里。

```rust
// provider::input —— 判定，先命中者胜
pub fn accepted_input(stated: Option<InputKinds>, pinned: Option<InputKinds>, base_url: &str, id: &str) -> InputKinds;
// provider::preset —— 与 ceiling_for、window_for 读 model_for 的同一行
pub(crate) fn input_for(base_url: &str, id: &str) -> Option<InputKinds>;
```

- **梯子从高到低：人登记的事实 → 钉版目录 → 预置表 → `Text`。** `pinned` 是钉版目录按精确 id 的那一行的 `input`；预置表按 base URL 的 host 与 id 前缀查（`model_for`，中转站借厂商的行、这台电脑上的服务不借，规则同 §8-17）；两者都不说时答 `Text`，理由同 §8-7：猜小了是一句拒绝，猜大了是 provider 的 400。目录与预置表是同一档的两个索引（§8-17 的测试钉住两者不为同一个 id 作答），这里仍让目录在前，与 `OutputCeiling::resolve` 的 `pinned` 先于预置表同序。
- **人那一档是 `SelectModel.input`**（wire 的 `Command::SelectModel`，紧接在 `max_output_tokens` 之后）。它是本函数的第一个参数 `stated`：出现时它作答，目录与预置表说什么都不改它；缺席时梯子照旧从目录开始。人说的是一个决定，不是推断，与 `Stated.person` 同理（D16）。设置页的控件归页面，线上字段已经在。
- **不读这个模型以前登记的 `input`。** 以前登记的值是当时梯子的答案，不是谁说过的话；读回它，一个在预置表学会它之前按 `Text` 登记的模型就永远按 `Text` 登记。重选同一个模型因此总按今天的表重新作答。
- **不记是哪一档答的。** `model_selected` 记 `input`，不记它的来处：读它的只有端点发图前的那道检查，那道检查不因来处而异；上限要记来处（`ceiling_from`），是因为一次被截断的跑要读得出原因。
- **唯一的生产调用者是选型点**（`accounting::worker::credentials::endpoints::choosing` 的 `select_model`）：登记行的 `input` 就是本函数的答案。
- **平台**：纯判定，三个平台相同。
- **派生检查**：`every_answer_is_a_stated_fact_or_text` 在 Rust 一侧只有 `the_first_rung_that_states_a_fact_answers` 那张表的几行，没有走遍人、目录、预置表三档各三种取值的检查，记为债。
- **验收**：`provider::input` 的测试比一张表——人说了的那一档胜过目录与预置表（两个方向各一次）、目录说 `Text` 而预置表说 `TextImage` 时答 `Text`、目录沉默而预置表说 `TextImage` 时答 `TextImage`（厂商主机上与中转站上各一次）、这台电脑上的服务与两表都不认识的 id 答 `Text`；`accounting` 的 `a_preset_model_that_reads_pictures_is_registered_as_reading_them`：一个目录没有、预置表写着 `text_image` 的模型选为 `Ocr` 之后，端点账本里那一次选择的 `input` 是 `TextImage`；`a_model_the_person_says_reads_pictures_is_registered_as_reading_them`：一个目录写着 `text` 的模型，`SelectModel` 带 `input: TextImage` 选为 `Ocr`，登记的 `input` 是 `TextImage`。
-/

namespace Gateway.Provider.Input

/-- 一个模型收得下什么（`kernel::event::record::InputKinds`，`gateway::InputKinds` 是它的再导出）。kernel 的 Lean 规格（`crates/kernel/Spec.lean`）不定义这个类型，所以这里照它的两个变体写一份模型里的类型，拼写与 Rust 相同。 -/
inductive InputKinds where
  | Text
  | TextImage
  deriving DecidableEq, Repr

/-! D14 一个模型收得下什么只在 `provider::input` 判一次

决定：`accepted_input` 按目录、预置表、`Text` 的序作答，选型点只调用它。理由：预置表已逐行写着 `input` 并注出处，目录与预置表本是同一档的两个索引，窗口与上限都已按这个序读；判定放在 gateway，前端将来为人那一档加的控件与选型点读同一个函数。被否的备选：①在选型点接一个 `.or_else(preset::input_for)`，窗口梯在 `accounting` 里就是这么写的——那是第二处判定，人那一档上线时要在两处各加一次；②给钉版目录为每个读图的模型加一行——目录是价目的家，一行没有复核日期的价目就是一个会漂的权威（§8-17 不把价目放进预置表的同一理由），而预置表已经说了这件事。重开参数：上游 `/models` 的 `input_modalities` 被定为一档（§3）。
-/

/-! D16 人那一档是 `SelectModel` 的一个可选字段，出现即作答，缺席即「这一次没人说」

决定：`Command::SelectModel` 带 `input: Option<InputKinds>`，`accepted_input` 的第一个参数就是它；出现时它作答，可以比目录宽，也可以比目录窄；缺席时梯子从目录开始，不读这个模型以前登记的 `input`。理由：人知道而两张表不知道的事有两个方向——一个目录之外、厂商文档也没写的模型能读图，或一条中转线会把图剥掉；只许放宽就说不出后一种。字段放在 `SelectModel` 里，是因为设置页每次选型都把整行送来（`choosing` 的 `select_model`），收得下什么与窗口、上限同是「探不到、由人说」的一格。缺席时不读回以前的登记，理由同 D14 的第三条：`model_selected` 不记 `input` 是哪一档答的，读回它就把梯子的一次答案当成了人说的话。代价：一次不带这一格的选型（例如对话框里的 `/model`）让 `input` 回到梯子的答案，人说过的要在设置页再说一次。被否的备选：①只许放宽（`stated` 只能是 `TextImage`）——说不出「这条线不收图」；②另开一条命令只改 `input`——两条命令写同一行登记，后到的一条要先读回前一条写下的整行；③缺席时读回以前登记的 `input`——见上。重开参数：一个不经设置页选型的入口要保住人说过的 `input` 时，`model_selected` 记下 `input` 的来处，缺席时只读回人说的那一档。
-/

/-- `provider::input::accepted_input`。`stated` 是人在 `SelectModel.input` 里说的（D16）；`pinned` 是钉版目录按精确 id 的那一行的 `input`；`preset` 是 `preset::input_for(base_url, id)` 的答案，模型把预置表按 host 与 id 前缀查的那一步（`crates/gateway/spec/Provider/Preset.lean`）当作一个已经给出的值。 -/
def accepted_input (stated pinned preset : Option InputKinds) : InputKinds :=
  ((stated.or pinned).or preset).getD .Text

/-- 每一档压过下一档是 `accepted_input` 的定义本身（`Option.or` 的次序），不另写定理；谁都不说时答 `Text`：猜小了是一句拒绝，猜大了是 provider 的 400。Rust 用 `unwrap_or_default`，`InputKinds` 的 `#[default]` 是 `Text`。

梯子不发明事实：答案要么是人说的，要么是人沉默时目录的，要么是两者都沉默时预置表的，要么是三者都沉默时的 `Text`。 -/
theorem every_answer_is_a_stated_fact_or_text (stated pinned preset : Option InputKinds) :
    stated = some (accepted_input stated pinned preset)
      ∨ (stated = none ∧ pinned = some (accepted_input stated pinned preset))
      ∨ (stated = none ∧ pinned = none ∧ preset = some (accepted_input stated pinned preset))
      ∨ (stated = none ∧ pinned = none ∧ preset = none
          ∧ accepted_input stated pinned preset = .Text) := by
  cases stated <;> cases pinned <;> cases preset <;> simp [accepted_input, Option.or]

end Gateway.Provider.Input
