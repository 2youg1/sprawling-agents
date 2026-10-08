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
### 8-40 会动作的门先问凭据：`Keys::pairing`、`decide_admission` 与一层 middleware

请求先过入口判定（`spec/Reception/Entry.lean` §8-94），再到这里问凭据。一个调用方握着的凭据只有两种：这次服务的本机钥匙（面里那一把，`spec/Reception.lean` §8-41），或浏览器经挑战签名换来、还活着的会话令牌（`spec/Server.lean` §8-93）。

```rust
pub struct Keys<'a> { pub face: &'a BindFace, pub sessions: &'a Sessions }
impl Keys<'_> { pub fn pairing(&self, offered: Option<&str>) -> Pairing; }   // 一问，hello 与每个 POST 共用
pub fn decide_admission(door: Door, pairing: Pairing) -> Admission;
// Transcribe | Enroll | Drop：Absent 即 E_GATE_DENIED；Acp：Absent 仍进，携 Pairing::Absent
```

口径：

- **没有「没配令牌就人人算配过」这一支**。面总带一把钥匙（§8-41），所以回环城与暴露城对一个空手的来者是同一个答案。
- **`Door` 是枚举而不是路径字符串**。门烧进 `route_layer` 的状态里，路由表因此仍是全仓唯一拼出 `/transcribe`、`/enroll` 的地方；middleware 不回头读 `uri().path()`，否则路径就有了第二个家。
- **会动作的三扇门当场拒，`/acp` 不拒**。花钱、收凭证、落文件是动作，空手的来者不该触发；外来编辑器「只学到一位」是 `agent_protocols::admit` 的措辞权（`crates/agent_protocols/Spec.lean` §9），所以那扇门把 `Pairing` 传进去而不是自己写拒词。`/acp` 的凭据写在 body 的 `token` 键里（编辑器没有别的地方写），判定调的是同一个 `Keys::pairing`。
- **`/` 与 `/{*asset}` 不问凭据**：人要先拿到页面，才有地方配对。它们仍过入口判定的 Host 一步。
- **凭据怎么递**：socket 写在 hello 帧的 `token` 里，POST 走 `Authorization: Bearer`，`offered_pairing` 是这条拼写在服务端的唯一读者。原生客户端（`sprawling call`、`gauge`、`enrol`、远程中继）递本机钥匙，浏览器递会话令牌。
- **`/enroll` 与保存凭据的那条路共用一个构造点**：realm 与 name 交给 `kernel::SecretRef::new` 建引用，建不出来即 422；201 正文引用的是 `secret_captured` 记录里那句 `ref`。
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

/-- 来者握没握着这座城认的凭据（`auth::Pairing`）。 -/
inductive Pairing where
  | Held
  | Absent
  deriving DecidableEq, Repr

/-- `Admission`：放进来并说明握没握着凭据，或以 `E_GATE_DENIED` 拒绝。 -/
inductive Admission where
  | Admit (pairing : Pairing)
  | Refuse
  deriving DecidableEq, Repr

/-- `Keys::pairing`：出示的摘要是面里的钥匙，或是一个活着的会话令牌，就算握着。出示的令牌在模型里已是摘要，常数时间比对归 `auth::verify`。 -/
def judge {Digest : Type} [DecidableEq Digest] (face : BindFace Digest) (sessions : List Digest)
    (offered : Option Digest) : Pairing :=
  match offered with
  | none => .Absent
  | some digest => if digest = face.key ∨ digest ∈ sessions then .Held else .Absent

/-- `decide_admission`：握着就放进来；空手时三扇会动作的门拒绝，`/acp` 带着 `Absent` 放进来。 -/
def decideAdmission (door : Door) (pairing : Pairing) : Admission :=
  match pairing, door with
  | .Held, _ => .Admit .Held
  | .Absent, .Acp => .Admit .Absent
  | .Absent, .Transcribe => .Refuse
  | .Absent, .Enroll => .Refuse
  | .Absent, .Drop => .Refuse

/-- **`/acp` 从不拒绝**：外来编辑器只学到一位，那一位由 `agent_protocols::admit` 措辞。 -/
theorem the_acp_door_never_refuses (pairing : Pairing) :
    decideAdmission .Acp pairing ≠ .Refuse := by
  cases pairing <;> simp [decideAdmission]

/-- **没有一扇会动作的门让空手的来者动作**，不论面在回环还是暴露：出示的既不是面里的钥匙、也不是任何活着的会话令牌，转写、录凭证与落文件都被拒。 -/
theorem no_door_acts_unpaired {Digest : Type} [DecidableEq Digest]
    (face : BindFace Digest) (sessions : List Digest) (door : Door) (offered : Option Digest)
    (acting : door ≠ .Acp) (notTheKey : offered ≠ some face.key)
    (noSession : ∀ digest, offered = some digest → digest ∉ sessions) :
    decideAdmission door (judge face sessions offered) = .Refuse := by
  have absent : judge face sessions offered = .Absent := by
    cases offered with
    | none => rfl
    | some digest =>
      have keyed : digest ≠ face.key := fun same => notTheKey (by rw [same])
      have unsessioned := noSession digest rfl
      simp [judge, keyed, unsessioned]
  rw [absent]
  cases door <;> simp_all [decideAdmission]

/-- **出示面里的钥匙，每扇门都放进来并记作握着。** -/
theorem the_native_key_opens_every_door {Digest : Type} [DecidableEq Digest]
    (face : BindFace Digest) (sessions : List Digest) (door : Door) :
    decideAdmission door (judge face sessions (some face.key)) = .Admit .Held := by
  simp [judge, decideAdmission]

end Wire.Reception.Admission
