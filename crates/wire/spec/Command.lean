-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.wire.spec.Command.Kind

/-!
# wire::command

规定 `command`、`command::no_secret`、`command::wire`（`crates/wire/src/` 下同名的文件）。客户端能让城做的每一件事，与套接字拼不出的那一帧。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-7 一次会话有名字，而名字就是它干活的那个房间

```rust
WireCommand::Dispatch { addr, task, goal, policy, idem, session: Option<SessionName>, effort, … }
// SessionName 住 kernel：一个构造点，内容即一个地址段
```

- **不加第四层**：每个 Run 一个目录会切断 Handoff 的连续性，而连续性正是一个 session 之所以是 session 的东西；`collab` 整套也都建立在「几个居民在同一栋楼里不互相踩」上。
- **地址给楼，名字给会话**：`addr` 可以只是一栋楼；`session` 在它底下开一个房间（`city::room::open`）。重名加数字后缀（`refactor`、`refactor-2`），而不是拒绝——一个人连着开两次同名会话是常事。
- **向已有房间派活即继续那条会话**：它的 `Handoff.md` 与 `JOB.md` 就是连续性。「回复某个 session」因此不需要新概念。
- **`RunId` 不变**：名字是房间的标签，不是 Run 的。一个房间一生中的多次 Run 合起来才是一条会话。
- **`SessionName` 是值类型而非 `String`**：它必须能当一个地址段（无 `/`、无 `.`／`..`、无控制字符、非空、不叫 `.sprawling`），否则一个人输入的字会变成一条路径。
-/

/-!
### 8-18b 派活帧不再携上限

```rust
Dispatch { addr, task, goal, policy, idem, session, effort }   // 删去 budget: BudgetCap
```

**没有人能在一件事跑之前给它定价**，所以说出「跑这件事」的那条帧不带上限。刹车只留一个：`Halt` 关掉一个范围并终止该范围里已经起来的后台成员（`runtime::backlog` 使这句话为真）。`kernel::BudgetCap` 及其判定面随之删除（`crates/kernel/Spec.lean` §8-12），`wire` 的 kernel 再导出列表因此少一项 `BudgetCap`——**这是公开面变更**，`web` 与 `sprawling` 两份基线同变更集重生。

- **`BudgetUse` 留在再导出列表里**：成本页读它，五路归因报它。**报告花了多少**与**事前不许花**是两件事，此处只删后者。
- **`Dispatch` 的 reach 不变**（§19-2 仍是 `client`）：删的是一个字段，不是一个动词。
- **旧客户端**：`WIRE_V` 进位后在握手期被明确拒绝，所以一条仍然写着 `budget` 的帧到不了服务端；服务端也不再有那个字段可读。
-/

/-!
### 8-26 `ConfigureBuilding` 长出 `desktop`：一栋楼的桌面白名单走同一条帧

```rust
ConfigureBuilding { addr: Address, sandbox: Option<SandboxLimits>, mcp: Option<Vec<McpServer>>,
                    desktop: Option<String>, idem: IdemKey },
```

- **不新起一条命令，也不给 `GovernedDocument` 加变体**。这条帧问的本来就是「这栋楼的 runs 够得到什么」——沙箱、外部服务器、运行中的机器上的哪些窗口，是同一个问题的三面；各自可缺省，缺省即不动那一面。而 `GovernedDocument` 是**城**的三份文件（`<city>/.sprawling/`），桌面白名单是**楼**的（`<building>/.sprawling/`）：把楼级路径塞进一个按 city_root 取路径的枚举里，会让那个枚举需要一个只有部分变体用得上的参数（city-SPEC §8-26 已写下这条）。
- **`desktop` 是文本而不是解析过的值**。读它的那台 server 是它语法的权威，且 fail closed——读不出来的文件关成全拒。城这一侧再抄一份解析器就是第二个权威，而两个权威里迟早有一个把某份文件读成另一种意思。城只保证「写进去的字节就是人给的字节」。
- **`building_configured` 载荷第三个布尔位 `desktop`**：与 `sandbox`／`mcp` 同形，说的是「这一面被写过」而不是写了什么。`city::Written` 把三个布尔收成一个值——一个调用点写 `(true, false, true)` 说不出哪一位是哪一面。
- **页面读它用 `Query::Document`**：`<building>/.sprawling/DESKTOP.toml`。那条查询本来就明说保留子树在这里可读，理由是「治理一栋楼的东西正是这个视图要给人看的」。
-/

/-!
### 8-44 删掉两个拼得出、执行不了的动词

`Command::Takeover` 与 `Command::Rollback` 删除；`COMMAND_NAMES` 从 30 到 28，schema 哈希随之变（golden 见 §2），故同集进位 35→36。

- **它们从来没有执行者**：装配层自上线起就对这两帧以 `not_built` 作答（§19 记的那次失效的三个动词之二），而客户端按 §19-2 的门要求不画它们。一条线上拼得出、任何东西都执行不了的命令，是对客户端的假承诺；删帧之后旧页面在握手期被明确拒绝（§10），而不是拿到一个永远失败的按钮。
- **规则的家在 kernel D2**（回滚＝分支＋git 还原），含理由、被否方案与重开参数；本节只记线的形状，不复述第二份。
- **`control` 与 reach 表同集缩面**：`Intervention` 少两臂，只剩 Steer／Cancel／Halt 加 Release 返程（§8-4）；「中断一个活着的 Run 恒以 Handoff 收尾」的中断动词随之只剩 Steer／Cancel；§19-2 删两行。
- **事件词同集删二**（`crates/kernel/Spec.lean` §8-4 表）：`rollback_applied`／`takeover_started` 无生产者，账本从未写下过携它们的行，故已写历史的字节与逐字节重放不受影响；携这两个词的行今天在读侧入口拒（`E_INVALID_ARGS`，kernel `parse_line` 的既有码）。
- **两帧的拒因不再是 `not_built`**：`not_built` 只留给仍在文法里、等待执行者的动词；对这两帧，字节在解码处就不再是命令（§8-37 的 `E_WIRE_MISMATCH` 口径不变）。
-/

/-!
### 8-45 `ConfigureBuilding` 长出第二道阈值的面

`Command::ConfigureBuilding` 多一个可选字段 `context_second_threshold: Option<u64>`，`Query::Config` 的回答多一个 `second: Option<SettledSecond>`；schema 哈希随之变，故同集进位 36→37。

- **一个字段而不是一条新帧**：那条帧问的就是「这栋楼的 runs 按什么规矩来」——沙箱、外部服务器、桌面白名单与第二道阈值是同一个问题的四面；为它单立一帧会给同一个问题两个写入口。
- **线上传裸 `u64`，域的判定不在这条帧上**：合法域与拒因句式是 `kernel::config::SecondThreshold` 的一个构造点（30–90，拒因带域），在这条帧上再判一次就是同一个规则的第二个家；越界值在解析点拒，拒因随答复回到设置页。
- **回答带 `SettledSecond { percent, from }`**：`from` 是说出这个值的那一级文件，理由与 `SettledEffort` 同（`Query::Config` 回答表那一条）；缺省不是缺口，而是城一级默认值在生效，页面据此把一个空框画成默认值。
- **线的背面是同一件事**：写入经 `city::write_second_threshold` 落到那一级的 `CONFIG.toml` 的 `[context] second_threshold`，与 `write_effort` 同一扇门（读—改—写整份文件，别人的键原样保留）；`building_configured` 的载荷因此从三面到四面（`Written::context`）。
- **`WIRE_V` 的路不单独走**：36→37 记的是这一次面变——给既有命名帧加字段是「语法换形而名字没换」那一类（字段名不进 `COMMAND_NAMES`），与 §8-44 的 35→36 无关；两次都在 §8-1 的 golden 里看得见。
-/

/-!
### 8-47c `RestoreDiscard`：回收站的一行按它自己的路回去

```rust
Command::RestoreDiscard { restoration: Restoration, idem: IdemKey }
```

- **帧里带的是那一行的 `restoration` 原样**，不是路径。`DiscardView` 的每一行已经带着它自己的回去的路（`Restoration`），页面把它交回来；城要是改成按路径去查，就得在写线程上为一次还原把整份历史再折一遍，而那一行本来就在页面手里。一个伪造的 `Tracked` 能做到的最多是把城自己历史里的某个文件写回城里它自己的路径——`Address` 爬不出城，`restore` 拒绝 `Address::is_reserved` 的地址，所以受保护的元数据子树（`.sprawling/`、`.git/`）不经这条路写入。
- **三种路，三个回答**：`Tracked(file:<addr>@<oid>)` 由装配层经 `storage::Checkpoint::restore` 写回，再追加 `discard_restored`（载荷与它关掉的那条 `file_discarded` 同形：`paths` 与 `restoration`），`DiscardView` 据此把那一行标成已还原；`Interred` 答 `E_INVALID_ARGS`，recovery 说从内容仓库取回尚未接线；`Rebuildable` 答 `E_INVALID_ARGS`，recovery 就是那条重建的理由——没有存着的字节可放回去。带 `range` 的定位符同样被拒：还原的是整个文件。
- **被否：`RestoreDiscard { path }`**。见第一条；另外，同一路径可以被丢两次，只给路径说不清要回到哪一次。
-/

/-!
### 8-48c `Dispatch` 可以点名这一次的模型

```rust
Dispatch { addr, task, goal, policy, idem, session, effort, model: Option<String> }
```

- **`model` 是城已登记的一个模型 id**，登记在哪个 tag 下都行；`None` 取房间自己那层已冻结的模型，房间尚未冻结时取 `main` tag 的模型。装配按这个 id 在簿子里找到那一条登记，连同它的端点与窗口一起用，保密楼「只用回环端点」的检查照旧由 `ModelBook::select` 做（sprawling-SPEC §8-10）。
- **被否：借 `SelectModel` 换 tag 再派活**。`SelectModel` 改的是整座城的配置，会在同时跑着的别人的 run 底下换模型；一次派活的选择只该属于这一次派活。
- **被否：接受任意 id，在 `main` 的端点上直接调用**。窗口与输出上限是登记时说出的，未登记的 id 没有这两个数，上下文提醒只能量一个没人给过的数。
- **`WIRE_V` 38→39**：给既有命名帧加字段是「语法换形而名字没换」那一类，§8-1 的 golden 随之变；`client/src/wire.ts` 由 `cargo xtask wire-ts --write` 同集重生成。
-/

/-!
### 8-57 派活带出整份运行策略：`Dispatch.policy`

```rust
Dispatch { addr, task, goal, policy: kernel::RunPolicy, idem, session, effort, model }
// RunPolicy { mode: Mode, write: WriteLimit, admit: AdmissionRequirement, landing: LandingPolicy }
// 线上：{"mode":"work","write":"create","admit":"tested","landing":"experiment"}
```

- **一个字段，四个必填的键**：`mode`（`chat｜work`）、`write`（`full｜create`）、`admit`（`standing｜tested｜contract_kept｜double_validated`）、`landing`（`ordinary｜experiment`）。值集与拼法只住 kernel（`crates/kernel/Spec.lean` §8-77、§8-78），本 crate 再导出 `RunPolicy`、`Mode`、`WriteLimit`、`AdmissionRequirement`、`LandingPolicy` 五个名字，页面按 `wire.ts` 里生成的字面量拼。缺一个键、或一个认不出的词，帧在反序列化处即拒，不落成默认值。
- **页面怎么选**：写域选择给 `write` 的两项（普通 `full`、只读可新建 `create`）；`/admit tested|contract|double` 依次填 `tested`、`contract_kept`、`double_validated`，不说时填 `standing`；试验填 `landing: "experiment"`；计划经 `/plan` 进入固定的 SDD 工作流，帧上是 `mode: "work"`。这些控件归客户端的展开面（client-SPEC 4-41），本节只定帧。
- **城自己派的活不经这个帧**：计划、日程、来信与编辑器经 ACP 派来的活由装配层取 `RunPolicy::of(Mode::Work)`；委派与敲门继承说话那一方的整份策略（sprawling-SPEC 8-133）。
- **账上读得到**：装配层把收到的策略原样写进这次 run 的 `run_started.policy`，所以一次派活选了什么，回放与 playback 从账本读，不从线上猜。
- **`WIRE_V` 不另进位**：`Dispatch` 的名字没变而形状变了，这正是 D1 说的改形，与本批其余改形共用 45。
-/

/-! D4 `Dispatch` 带一个 `policy`，而不是四个平铺的字段

**决定**：运行策略在帧上是一个嵌套的值 `policy`，形状就是 `kernel::RunPolicy`；`mode` 不再是 `Dispatch` 的顶层字段（§8-57）。

**理由**：四个值总是一起走——线上、账本的 `run_started`、装配层的 `Assignment` 都是整份地拿、整份地传——一起走的值是一个值。帧上的形状与账本上的形状相同，页面、回放与 playback 读的是同一个东西，没有一层把四个字段重新拼成一个结构。四个键都必填：选择要求不等于拿到证据，而一个缺省成「免检」的键正是这条规则要挡住的。

**被否**：①保留顶层 `mode`、平铺加 `write`／`admit`／`landing` 三个带缺省的字段：旧页面可以不改，但缺省值会替没选过的人做一个选择，而帧本来就因改形要换一次版本；②`policy` 里的键带缺省：同一条理由。

**重开参数**：运行策略多出一个只有部分派活才需要的值时，重议那个值要不要缺省。
-/

namespace Wire.Command

open Wire.Command.Kind

/-- `Command<Secret>` 的模型：泛型于 secret 的携带者。`PutSecret` 是唯一携它的一臂，其余每一个动词不携 secret，以 `Kind.Command` 的名字代表（字段与 secret 无关，模型不写）。 -/
inductive Carrying (Secret : Type) where
  | PutSecret (value : Secret)
  | Other (verb : Command)

/-- `NoSecret`：无值可造的类型。`WireCommand` 是 `Command<NoSecret>`，套接字所能携的全部。 -/
abbrev WireCommand := Carrying Empty

/-- **套接字拼不出 `PutSecret`**：`WireCommand` 的那一臂要一个 `NoSecret` 的值，而它没有值。Rust 侧由类型与 `tests/ui/put_secret_onto_the_wire.rs` 的反例守住，这里是同一个论证。 -/
theorem a_socket_cannot_spell_put_secret (c : WireCommand) : ∀ v, c ≠ .PutSecret v :=
  fun v => v.elim

/-- `impl From<WireCommand> for Command`：总函数，`PutSecret` 一臂写作对空值的匹配。 -/
def fromWire {Secret : Type} : WireCommand → Carrying Secret
  | .PutSecret value => value.elim
  | .Other verb => .Other verb

/-- 从线上来的命令进城时动词不变。 -/
theorem fromWire_keeps_the_verb {Secret : Type} (verb : Command) :
    (fromWire (.Other verb : WireCommand) : Carrying Secret) = .Other verb :=
  rfl

/-- 进城的命令永远不是 `PutSecret`：凭证只经 `/enroll` 在进程里成形（§8-2）。 -/
theorem nothing_from_the_wire_enrols_a_secret {Secret : Type} (c : WireCommand) (v : Secret) :
    fromWire c ≠ .PutSecret v := by
  cases c with
  | PutSecret value => exact value.elim
  | Other verb => simp [fromWire]

end Wire.Command
