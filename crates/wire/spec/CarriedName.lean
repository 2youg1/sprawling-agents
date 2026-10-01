-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::carried_name

规定 `carried_name`（`crates/wire/src/` 下同名的文件）。本 crate 不拥有的名字：只在唯一构造点拒空与控制字符。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-0 跨层名字的携带法（先于一切接口的决定）

`PlanRow.status` 携 `kernel::RoadmapStatus`（经 `kernel` 重导出，住 `spine::row`，公共拼写不变）。
`Dispatch` 携 `policy`，值集住 kernel（`kernel::RunPolicy` 与它的四个枚举，§8-57），wire 依赖 kernel，所以帧直接携它。`CreateBuilding` 携 `template`、`ConnectToolkit` 携 toolkit 的 slug——**这两个集合的权威分别住 `city` 与 broker 的目录，而 wire 只依赖 kernel**（ARCHITECTURE §2 depmap）。

取法：wire 携**无封闭列表的 newtype**（`ProviderName`、`TemplateName`、`ToolkitSlug`），只断言「非空且无控制字符」，**不断言合法值集**。合法值集恒由上游单一权威回答，映射点在装配层（`bin::assembly`），未知值即报错不猜。

理由：若 wire 自建一份 `enum Mode`，就产生了**同一规则的第二个权威**（AGENTS.md 明拒），且两份枚举会静默地漂开。wire **确实不知道** mode 集合是什么，假装知道才是谎言。**被否**：wire 内镜像这几个枚举——两个权威。
-/

namespace Wire.CarriedName

/-- `char::is_control`：Unicode 的 Cc 类，U+0000–U+001F 与 U+007F–U+009F。 -/
def isControl (c : Char) : Bool :=
  c.toNat < 0x20 || (0x7F ≤ c.toNat && c.toNat ≤ 0x9F)

/-- `carried_name!` 生成的唯一构造点：拒空与控制字符，别的什么也不判。合法值集归上游（§8-0），所以这个函数不看任何名单。 -/
def parse (raw : List Char) : Option (List Char) :=
  if raw.isEmpty || raw.any isControl then none else some raw

/-- **一个不认识的名字照样携带**：非空、不含控制字符的串原样成为名字，不论上游认不认得它。 -/
theorem an_unknown_name_is_carried (raw : List Char) (filled : raw ≠ [])
    (printable : ∀ c ∈ raw, isControl c = false) : parse raw = some raw := by
  have notEmpty : raw.isEmpty = false := by
    cases raw with
    | nil => exact absurd rfl filled
    | cons _ _ => rfl
  have noControl : raw.any isControl = false := by
    rw [List.any_eq_false]
    intro c member
    simp [printable c member]
  simp [parse, notEmpty, noControl]

/-- 空串不是名字。 -/
theorem an_empty_name_is_refused : parse [] = none :=
  rfl

/-- 带一个控制字符的串不是名字。 -/
theorem a_name_with_a_control_character_is_refused (raw : List Char) (c : Char)
    (member : c ∈ raw) (control : isControl c = true) : parse raw = none := by
  have found : raw.any isControl = true := List.any_eq_true.mpr ⟨c, member, control⟩
  simp [parse, found]

end Wire.CarriedName
