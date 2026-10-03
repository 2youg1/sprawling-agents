-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::command::kind

规定 `command::kind`（`crates/wire/src/` 下同名的文件）。`enum Command` 的变体，与每个动词从哪里够得到、远程设备发来时属哪一类（§19，`xtask wiring` 的数据面）。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
## 19 每个动词从哪里够得到（`xtask wiring` 的数据面）

**这张表存在的理由，是一次已经发生过的失效。** v0.0.3 的审计发现 `accounting::worker::run_command` 只匹配 22 个 Command 里的 14 个，
六个动词落进 catch-all——其中 Takeover／Rollback／CreatePolicy **在线上、画在客户端、由任何东西执行不了**，
而 Cancel 与 Steer 在 run 不处于安全点时失败，恰好是人最需要它们的那一刻。
`not_built` 的 rustdoc 当时就写着「**在这里被回绝的动词不得作为控件出现在客户端**」——那是一条**没有任何机器在看的规矩**。

反方向同样会失效，而且更安静：**一个城做得到、却没有任何控件够得到的能力，没有人会收到抱怨**，
因为不存在的按钮不会有人去点。`Pursue`（「城自己走」）与 `SetAutonomy` 就是这样漏掉的，
这也正是验收标准第 2 条一直没过的机制原因——**在浏览器里打不开它**。
-/

/-!
### 19-1 reach 的四个取值

表里的第二个事实是 `class`：这个 Command 由一台远程设备经远程门发来时属于哪一类动词（§19-3）。reach 说城里谁该够得到它，class 说城外的设备能不能带它进来，两件事互不推出，所以是两列。

| 取值 | 含义 | 门要求什么 |
|---|---|---|
| `client` | 人用的动词，客户端必须画得出 | `client/src` 里有发出点，且 `run_command` 不以 `not_built` 作答 |
| `push` | 由外部服务推进来，不是人点的 | 只要求 `run_command` 能执行；客户端有没有它都不看 |
| `handshake` | 在握手层被吃掉，进不到 `run_command` | 两侧都不要求 |
| `sealed` | 线上不可拼写 | 两侧都不要求；客户端**若**出现即为红 |
-/

/-!
### 19-2 表

表就是下面的两个 `def`：`Command.reach` 是 reach 列，`Command.verbClass` 是 class 列，一行一臂；reach 每一臂上方的注释是原表的说明列。`inductive Command` 是 `enum Command` 的变体，一行一个。
-/

/-!
### 19-3 class 的三个取值

| 取值 | 含义 | 远程设备 |
|---|---|---|
| `Read` | 读城，什么也不改 | `Watch` 与 `Act` 两种权限都带得进来 |
| `Act` | 人离开电脑时仍要做的活：派活、改方向、停下、叫停与放开、答审批、转交、按楼成批派活、追目标、开新一段会话、写楼自己的 spine 文档 | 只有 `Act` 权限带得进来 |
| `LocalOnly` | 放宽访问、够到凭证或城所在的宿主机、改变治理这座城的东西：接端点、选模型、建楼拆楼、写规则与配置、装东西、开文件管理器 | 恒不带进来，不论权限 |

- **新加的 Command 一律 `LocalOnly`**，除非人决定一台不在电脑旁的设备可以做它。`Act` 与 `Read` 是一次决定，不是默认值；表里一行缺 `class` 格，`xtask wiring` 点名那一行。
- `class` 只判 Command。`Ask` 与 `Monitor` 两种帧属 `Read`；设备发来的 `Hello` 由中继换成城自己的令牌再发（§8-66）；远程门自己的动词（开门、配对、撤销）不在线上，表里没有它们（remote_access D4）。
- 这一列是权威，中继按它判，`xtask wiring` 把表与中继的穷尽匹配（`bin::outside::verbs::command_class`）逐行对照（§8-65）。

**`client` 而尚未落地的三个**（`HandOff`／`PutShelved`／`BatchByBuilding`）今天由 `not_built` 作答，
所以门对它们要求的是**客户端不画**——`not_built` 的 rustdoc 说的就是这件事，现在有机器看着了。
它们的 reach 仍写 `client`，因为那是它们做完之后该去的地方；写成别的取值等于把「还没做」记成「不该做」。
-/

/-!
### 8-65 动词类：§19-2 的 `class` 列

```rust
// bin::outside::verbs（sprawling）：中继读这一列的那一处
pub(super) fn command_class(command: &wire::WireCommand) -> remote_access::door::VerbClass;   // 穷尽匹配，无通配臂
```

- **`class` 是 §19-2 的一列，不是 wire 的一个方法。** 它说的是远程门放不放一帧进来，而远程门的权限与 `VerbClass` 归 `remote_access`（crates/remote_access/Spec.lean §8-1）；本 crate 不依赖它，也不为它另立一个同值的枚举。中继在 `sprawling`，那里同时看得见两者（crates/remote_access/Spec.lean §7）。
- **表与匹配由门机器对照**：`xtask wiring` 读表的第三格与 `command_class` 的每一臂，表里缺格、读不成三个取值之一、或与匹配说法不一，都点名那一个动词（tools/xtask/Spec.lean §8-45）。匹配是穷尽的，所以一个新 Command 在有人定下它的类之前编译不过。
- 不改任何帧，不动 `WIRE_V`。
-/

/-!
### 8-66 远程中继怎样用这条线

- **设备说的是同一条线。** 远程会话里封装的每一帧文本（crates/remote_access/Spec.lean §8-5 的 `Payload::Frame`）就是一帧 `ClientFrame` 或 `ServerFrame`；中继打开封装、按 §19-2 判类，放行的帧**原样**发给城自己在回环上的 `/ws`，不解析后再序列化。
- **`Hello` 换成城自己的**：设备不知道、也不该知道城的配对令牌（§8-41）。中继把设备的 `Hello` 里的 `token` 换成城的令牌（没有就是空），`wire_v` 与 `schema` 照设备说的发，所以设备上的页面与城说不说同一版线，仍由 `server::decide_handshake` 判。
- **拒绝是一帧 `Refusal`**：被门拒的帧不到城，中继封一帧 `ServerFrame::Refusal` 回给设备，码是 `E_GATE_DENIED`，恢复语说该在城自己的机器上做，或该重新配对为 `act`。读不成 `ClientFrame` 的文本同样封一帧 `E_WIRE_MISMATCH` 回去。
- 城发来的每一帧都封好送回设备，事件、答复、增量一视同仁：一台 `Watch` 设备能读的就是这座城的整条线，这正是「看」的意思。
-/

namespace Wire.Command.Kind

/-- 线上的每一个动词：`command::kind` 里 `enum Command` 的变体，一行一个，拼写与 Rust 相同（tools/xtask/Spec.lean §8-43）。次序照 §19-2 的表。 -/
inductive Command where
  | Dispatch
  | ProbeEndpoint
  | ConfigureBuilding
  | AttachEndpoint
  | SelectModel
  | OpenSession
  | Reveal
  | RestoreDiscard
  | DoctorInstall
  | DoctorRefresh
  | ConnectToolkit
  | PutSpine
  | PutRange
  | DecideProposals
  | CreateBuilding
  | RemoveBuilding
  | Steer
  | Cancel
  | Halt
  | Release
  | Approve
  | SetAutonomy
  | HandOff
  | PutPreferences
  | PutShelved
  | Pursue
  | PutDocument
  | PutIdentity
  | PutRules
  | ConfigureCity
  | RestoreFile
  | PutGuide
  | BatchByBuilding
  | Wake
  | Auth
  | PutSecret
  | NameSession
  | ChangeRunPolicy
  deriving DecidableEq, Repr

/-- §19-1 的四个取值：城里谁该够得到一个动词。 -/
inductive Reach where
  | client
  | push
  | handshake
  | sealed
  deriving DecidableEq, Repr

/-- §19-3 的三个取值，拼写与 `remote_access::door::VerbClass` 相同。 -/
inductive VerbClass where
  | Read
  | Act
  | LocalOnly
  deriving DecidableEq, Repr

/-- §19-2 的 reach 列：一行一臂，每臂上方的注释是那一行的说明。`xtask wiring` 读这些臂（tools/xtask/Spec.lean §8-43），把它与 `enum Command`、`run_command` 的臂和 `client/src` 的发出点对照。 -/
def Command.reach : Command → Reach
  -- 派活，产品的正面
  | .Dispatch => .client
  -- 问一个端点它供应什么
  | .ProbeEndpoint => .client
  -- 改一栋楼的规矩
  | .ConfigureBuilding => .client
  -- 把一个端点挂上
  | .AttachEndpoint => .client
  -- 选一个模型
  | .SelectModel => .client
  -- 在同一个地址上开新的一段会话（`Carry` 说带不带上一段的交接，`from` 说从哪条线哪一行分出来）
  | .OpenSession => .client
  -- 在人自己的文件管理器里指出一个地址
  | .Reveal => .client
  -- 把回收站里一行按它自带的回去的路放回原处
  | .RestoreDiscard => .client
  -- 按需求表里的名字装一件机器缺的东西
  | .DoctorInstall => .client
  -- 重新探一遍机器，取代开城时的快照
  | .DoctorRefresh => .client
  -- 请外包服务开一次同意会话，把一个外部应用接进来
  | .ConnectToolkit => .client
  -- 写一栋楼自己的 spine 文档（roadmap／memo／handoff／spec）。携 `base`（发信方起手时那份正文）与 `body`，文件已被人或居民改过即拒——**这几份有两个写者**，与 `PutDocument` 的单写者前提不同，故两道门的守卫不同
  | .PutSpine => .client
  -- 对城里任意一份文档的一次保存：带基线版本与那一版的几段编辑，基线已动即拒，落下写 `document_written`（§8-72）
  | .PutRange => .client
  -- 对一份文档上几张修改提案卡的决定：逐句接受、改后接受或拒绝，接受的部分作为一次保存落下（§8-73）
  | .DecideProposals => .client
  -- 起一栋楼
  | .CreateBuilding => .client
  -- 把一栋楼移出城：文件搬进 reserved subtree（`crates/city/Spec.lean` §8-3），历史留在 Ledger，写 `building_removed`；有 run 正在其中某个房间里跑时拒 `E_BUSY`，点名房间与 run
  | .RemoveBuilding => .client
  -- 中途换方向
  | .Steer => .client
  -- 停下这一个
  | .Cancel => .client
  -- 停下一个范围
  | .Halt => .client
  -- 放开一个范围
  | .Release => .client
  -- 答一条审批
  | .Approve => .client
  -- 定一栋楼的 Autonomy（两态：本人或被任命的居民）
  | .SetAutonomy => .client
  -- 把一条问题转给另一位居民去答
  | .HandOff => .client
  -- 写这个人自己的 `~/.sprawling/config.toml` 的 `[ui]`
  | .PutPreferences => .client
  -- 写一份上架的文档（技能或说明）
  | .PutShelved => .client
  -- 设一个持续追的目标，以及暂停／恢复／清除
  | .Pursue => .client
  -- 写治理这座城的三份文件之一，携 `base`（§8-59）
  | .PutDocument => .client
  -- 写设置页上的卡片：城在 `base` 上改写 `PREFERENCES.md` 或 `MAYOR.md` 的身份区（§8-59）
  | .PutIdentity => .client
  -- 写一栋楼的 `RULES.toml`：整份、带 `base`，先求值后落盘（§8-60）
  | .PutRules => .client
  -- 写城自己那一层 `CONFIG.toml` 的 `keep_warm` 与 `effort`（§8-61）
  | .ConfigureCity => .client
  -- 把城工作树里的一个文件换回一个检查点里的那一份，检查点里没有就删去（§8-62(b)）
  | .RestoreFile => .client
  -- 写这座城的上手指南进度：走到哪一步、看过与跳过了哪几步、是否已离开指南（§8-68）
  | .PutGuide => .client
  -- 按楼成批派活
  | .BatchByBuilding => .client
  -- 外面发生了一件事；地址由 watch 表与 triage 决定，调用方说不出房间
  | .Wake => .push
  -- 出示配对令牌，`server::decide_handshake` 吃掉它
  | .Auth => .handshake
  -- 唯一没有字节形式的 Command；`Sealed<String>` 在线上不可居留
  | .PutSecret => .sealed
  -- 给一段 session 起显示名，写 `session_named`（Sessions.lean D27）；控件在 W3 FE-SESS
  | .NameSession => .client
  -- 会话中改房间的运行策略，写 `run_policy_changed`（Sessions.lean D27）；控件在 W3 FE-SESS
  | .ChangeRunPolicy => .client

/-! D6 动词类是 §19-2 的一列，由中继的穷尽匹配实现、门机器对照

**决定**：一帧由远程设备发来时属于 `Read`／`Act`／`LocalOnly` 哪一类，写在 §19-2 reach 旁边的 `class` 列；中继在 `bin::outside::verbs` 以穷尽匹配实现它，`xtask wiring` 逐行对照两者（§8-65）。

**理由**：这一列回答的问题与 reach 同族——一个动词从哪里够得到——所以与 reach 住在同一张表上，读者在一处看到两件事。实现必须在 `sprawling`，因为 `VerbClass` 归 `remote_access`，本 crate 不依赖它；穷尽匹配保证一个新 Command 在有人决定它的类之前编译不过，表格对照保证决定写下来了。

**被否**：①给 `wire::Command` 加一个 `class()` 方法：要么本 crate 依赖 `remote_access`，要么另立一个同值的枚举，两处定义同一组值；②在表里写类、中继在启动时解析 SPEC：二进制读一份 Markdown 做授权，文档的排版错误就成了门的漏洞；③不写表、只留匹配：一个动词能不能从城外做，是人要读到、要决定的事，藏在代码里没人看。

**写法**：`class` 是下方 `def Command.verbClass` 的一臂一行，`xtask wiring` 按 tools/xtask/Spec.lean §8-45 读它。**重开参数**：`command_class` 的一臂按帧的参数再分两类时，一行一臂写不下，表改由 Lean 侧导出（xtask D8、D11）。
-/

/-- §19-2 的 class 列：一行一臂。`xtask wiring` 读这些臂，与 `bin::outside::verbs::command_class` 的穷尽匹配逐个动词对照（tools/xtask/Spec.lean §8-45）。 -/
def Command.verbClass : Command → VerbClass
  | .Dispatch => .Act
  | .ProbeEndpoint => .LocalOnly
  | .ConfigureBuilding => .LocalOnly
  | .AttachEndpoint => .LocalOnly
  | .SelectModel => .LocalOnly
  | .OpenSession => .Act
  | .Reveal => .LocalOnly
  | .RestoreDiscard => .LocalOnly
  | .DoctorInstall => .LocalOnly
  | .DoctorRefresh => .LocalOnly
  | .ConnectToolkit => .LocalOnly
  | .PutSpine => .Act
  | .PutRange => .LocalOnly
  | .DecideProposals => .LocalOnly
  | .CreateBuilding => .LocalOnly
  | .RemoveBuilding => .LocalOnly
  | .Steer => .Act
  | .Cancel => .Act
  | .Halt => .Act
  | .Release => .Act
  | .Approve => .Act
  | .SetAutonomy => .LocalOnly
  | .HandOff => .Act
  | .PutPreferences => .LocalOnly
  | .PutShelved => .LocalOnly
  | .Pursue => .Act
  | .PutDocument => .LocalOnly
  | .PutIdentity => .LocalOnly
  | .PutRules => .LocalOnly
  | .ConfigureCity => .LocalOnly
  | .RestoreFile => .LocalOnly
  | .PutGuide => .LocalOnly
  | .BatchByBuilding => .Act
  | .Wake => .LocalOnly
  | .Auth => .LocalOnly
  | .PutSecret => .LocalOnly
  | .NameSession => .Act
  | .ChangeRunPolicy => .Act

/-- **没有一个 Command 属 `Read`**：读城的是 `Ask` 与 `Monitor` 两种帧，不是命令（§19-3）。一行写成 `Read` 的命令就是一个改东西的动词被当成只读放进了城。 -/
theorem no_command_is_a_read (c : Command) : c.verbClass ≠ .Read := by
  cases c <;> decide

/-- **城外的设备能做的，人在客户端里都画得出**：class 为 `Act` 的动词 reach 都是 `client`。 -/
theorem what_a_device_may_do_a_person_may_draw (c : Command) :
    c.verbClass = .Act → c.reach = .client := by
  cases c <;> decide

/-- **不由人点的动词留在城里**：推进来的、握手吃掉的与封着的，class 都是 `LocalOnly`。 -/
theorem a_verb_no_person_draws_stays_local (c : Command) :
    c.reach ≠ .client → c.verbClass = .LocalOnly := by
  cases c <;> decide

/-- **线上只有一个动词没有字节形式**：`PutSecret`。 -/
theorem only_put_secret_is_sealed (c : Command) : c.reach = .sealed ↔ c = .PutSecret := by
  cases c <;> decide

/-- 表不是空的：至少有一个城外设备做得了的动词，这条 `Act` 的保证不是空真。 -/
example : Command.Dispatch.verbClass = .Act ∧ Command.Dispatch.reach = .client := by decide

end Wire.Command.Kind
