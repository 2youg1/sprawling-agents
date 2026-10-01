-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.wire.spec.Reception

/-!
# wire::reception::admission

规定 `reception::admission`（`crates/wire/src/` 下同名的文件）。一次 HTTP 请求握没握着这座城的配对令牌，没握着时每扇门怎么办。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-40 会动作的两扇门先问配对：`decide_admission` 与一层 middleware

`decide_bind` 把「能从回环之外到达」换成配对令牌的要求，只在 `decide_handshake`（`/ws`）判令牌，**合法配置的暴露城就允许任何能到达端口的人写字节进金库路由、并驱动 `transcribe_sink` 花钱**。所以判定只有一个家。

```rust
// 三扇门共用的那一问，壳里零策略（同 decide_bind／decide_frame 的切法）。
decide_admission(Door::Transcribe | Door::Enroll | Door::Drop, offered, face)  // 未配对即 E_GATE_DENIED
decide_admission(Door::Acp,        offered, face)  // 未配对仍进，携 Pairing::Absent
```

三条口径：

- **`Door` 是枚举而不是路径字符串**。门烧进 `route_layer` 的状态里，路由表因此仍是全仓唯一拼出 `/transcribe`、`/enroll` 的地方；middleware 不回头读 `uri().path()`，否则路径就有了第二个家。
- **两扇会动作的门当场拒，`/acp` 不拒**。花钱与收凭证是动作，未配对者不该触发；而外来编辑器「只学到一位」是 `agent_protocols::admit` 的措辞权（`crates/agent_protocols/Spec.lean` §9），所以那扇门把 `Pairing` 传进去而不是自己写拒词。`/acp` 的令牌仍写在 body 的 `token` 键里（编辑器没有别的地方写），但**判定调的是同一个函数**——一个规则一个家，与令牌写在哪无关。
- **`/` 与 `/{*asset}` 保持开放**：人要先拿到页面，才有地方输入配对码。
- **令牌怎么递**：socket 写在 hello 帧里（帧类型给了它字段名），POST 没有帧，于是走标准的 `Authorization: Bearer`，`offered_pairing` 是这条拼写在服务端的唯一读者，`client/src/core/socket.ts` 的 `bearing()` 是浏览器侧唯一的写者。

- **`/enroll` 与保存凭据的那条路共用一个构造点**：realm 与 name 交给 `kernel::SecretRef::new` 建引用，路由不再手拼 `secret:<realm>/<name>`——手拼的文本本城可能解析不回来，而 201 会把它答出去，所以建不出来即 422。**201 正文引用的是 `secret_captured` 记录里那句 `ref`**：存进去的与答出去的原是同一个值，两处各拼一次今天相等只因没人归一，改一处就分岔。客户端同改：`enrol()` 用 201 正文里城说的那句引用，不再自己拼一份。
-/

namespace Wire.Reception.Admission

open Wire.Reception

/-- 四扇 HTTP 门，穷尽（`reception::admission::Door`）。 -/
inductive Door where
  | Transcribe
  | Enroll
  | Acp
  | Drop
  deriving DecidableEq, Repr

/-- 一次判定的结论：来者握着这座城的配对令牌，或没有（`auth::Pairing`）。 -/
inductive Pairing where
  | Held
  | Absent
  deriving DecidableEq, Repr

/-- `Admission`：放进来并说明配没配对，或以 `E_GATE_DENIED` 拒绝。 -/
inductive Admission where
  | Admit (pairing : Pairing)
  | Refuse
  deriving DecidableEq, Repr

/-- `decide_admission`：面不索要令牌（没配令牌的回环城）就人人算配过对；索要时出示的摘要相同即配过对；否则三扇会动作的门拒绝，`/acp` 带着 `Absent` 放进来。出示的令牌在模型里已是摘要，常数时间比对归 `auth::verify`。 -/
def decideAdmission {Digest : Type} [DecidableEq Digest] (door : Door) (offered : Option Digest)
    (face : BindFace Digest) : Admission :=
  match face.tokenDigest with
  | none => .Admit .Held
  | some expected =>
    if offered = some expected then .Admit .Held
    else
      match door with
      | .Acp => .Admit .Absent
      | .Transcribe => .Refuse
      | .Enroll => .Refuse
      | .Drop => .Refuse

/-- **`/acp` 从不拒绝**：外来编辑器只学到一位，那一位由 `agent_protocols::admit` 措辞。 -/
theorem the_acp_door_never_refuses {Digest : Type} [DecidableEq Digest]
    (offered : Option Digest) (face : BindFace Digest) :
    decideAdmission .Acp offered face ≠ .Refuse := by
  unfold decideAdmission
  split
  · simp
  · split <;> simp

/-- **出示了索要的令牌，每扇门都放进来并记作配过对。** -/
theorem a_held_token_opens_every_door {Digest : Type} [DecidableEq Digest]
    (door : Door) (face : BindFace Digest) :
    decideAdmission door face.tokenDigest face = .Admit .Held := by
  cases known : face.tokenDigest <;> simp [decideAdmission, known]

/-- **从回环之外够得到的城，不让没配对的来者动作。** 绑定判定给出的面与三扇会动作的门合起来：只要城在服务、地址在回环之外、来者出示的不是配置的那个摘要，转写、录凭证与落文件都被拒。 -/
theorem no_door_acts_unpaired_beyond_loopback {Digest : Type} [DecidableEq Digest]
    (token : Option Digest) (face : BindFace Digest) (door : Door) (offered : Option Digest)
    (served : decideBind .beyond token = .Serve face) (acting : door ≠ .Acp)
    (stranger : offered ≠ token) :
    decideAdmission door offered face = .Refuse := by
  cases token with
  | none => simp [decideBind] at served
  | some expected =>
    simp only [decideBind, BindVerdict.Serve.injEq] at served
    subst served
    cases door <;> simp_all [decideAdmission, BindFace.tokenDigest]

end Wire.Reception.Admission
