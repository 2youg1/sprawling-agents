-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::command::step

规定 `command::step`（`crates/wire/src/` 下同名的文件）。命令在标量之外携的值与它的闭集：保存、提案的决定、楼规、城的配置、会话的起法、关停范围、治理文档与设置页上的卡片。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-19 治理两帧：写身份文件，读被代答的事

```rust
// Command（第 24 条）
PutDocument { which: GovernedDocument, base: String, body: String, idem: IdemKey }
pub enum GovernedDocument { Mayor, Clerk, Preferences }

// Query（第 16 条）
Governance,                       // → Answer::Governance(GovernanceAnswer)

pub struct GovernanceAnswer {
    pub autonomy: Autonomy,          // 谁来答：Owner／Delegate(resident)／Deferred
    pub decided: Vec<Decision>,      // 替这个人做掉的事，旧在前
}
pub struct Decision {
    pub item: String,                // ApprovalId
    pub verdict: PolicyVerdict,
    pub cluster: ClusterKey,
    pub at: TimeMs,
}
```

- **三份文件一条命令，不是三条**：`MAYOR.md`／`CLERK.md`／`PREFERENCES.md` 都住 `<city>/.sprawling/`（没有任何写域够得到的地方），三者的写法逐字节相同，差别只在文件名。用穷尽枚举而不是路径串：**路径由城决定，不由发帧的人决定**，否则这条命令就成了往保留子树里写任意文件的入口。
- **`Preferences` 是第三份**：市长与文书各有身份文件，而「这个人怎么喜欢这座城办事」不属于其中任何一个居民，它属于城。它与前两者同住一处、同一条命令写，因为它们被同一条规则治理：住在保留子树里，居民读得到、改不了。
- **`decided` 回答的是「你不在的时候，有谁替你答了什么」**：`approval_resolved` 折出来的流，旧在前——与 `HistoryAnswer` 同口径，因为折叠期待这个顺序。它**不筛掉人自己答的那些**：一份只列代答的清单，会让「我答过」与「从没人答」在界面上长得一样。谁答的写在 `autonomy` 里，那是同一次读的另一半。
- **`PutDocument` 携 `base`**：三份文件有两个写者——原文编辑器与设置页上的卡片，或者开在两处的两个页面——所以一次保存说明它起手时的全文，文件已经变了就拒，什么都不写（§8-59）。`edit` 工具碰不到这三份文件，居民不是它们的写者。
- **被否**：（a）三条命令 `PutMayor`／`PutClerk`／`PutPreferences`——同一条规则三个入口，加第四份文件要改三处；（b）复用 `edit` 工具——`edit` 走写域，而写域恒不含保留子树，让它开一个例外就是把「居民改不了治自己的东西」这条最老的规矩打穿。
-/

/-!
### 8-43 新的会话是一个动词，不是一个开关

```rust
pub enum Carry { Nothing, Handoff }   // Nothing 是第一个变体，即默认
Command::OpenSession { addr: Address, carry: Carry, from: Option<Origin>, idem: IdemKey }
// Origin { run: RunId, at_seq: Seq }  —— kernel::Origin，一个家
```

**`from` 把「分叉」收进了同一个动词**。从某句分出去与从此处重开是同一件事的两个起点：都是「在这个房间开新的一段」，只差新的一段要不要继承某条线的对话。所以线上没有第二个动词，没有 `Fork` 帧（它写下血统却没有任何 dispatch 路径消费它，`routing.rs` 的注释与 runtime §8-2 的 §186 早把这件事记成缺陷），`OpenSession { from: Some(..) }` 是它该在的地方。**血统仍然写在 `run_forked` 里**，由真正开始的那个 run 写：一个分支在「开」的时刻还没有 run，先写一条血统就得先编一个 run id，而那个 run 永远不会存在。

**它答的是一个死路。** 房间的第一个 run 把模型与强度冻进它自己的 `CONFIG.toml`（city-SPEC §8-14），此后形状不同的派活全被 `E_CONFIG_INVALID` 拒——这条规则本身是对的，前缀缓存不能中途换模型；错在被拒之后没有任何出口，换过主模型的人再也派不出去。新的一段会话就是那个出口，而它只能在城里发生（清掉房间自己写下的两行、清掉交接槽位、在账本写 `session_opened`），所以它是一个 Command 而不是页面自己做的几件事。

**`Carry` 是枚举而不是 `bool`。** 两个状态都是有名字的行为，而且落到磁盘上的结果不同：`Nothing` 连 `Handoff.md` 的槽位一起清空，`Handoff` 留着它。`carry: true` 在调用点读不出是哪一个，`Nothing` 也不是「没有值」而是一个答案——它是第一个变体，`Default` 因此不需要人再写一遍。

**本章测试**：`crates/wire/tests/wire_contract.rs` 的命令样本（表长、名表去重、golden 哈希）；`Carry` 的默认值在 `wire` 侧有一条断言，因为它是这份规格里唯一被写成「第一个变体」的默认。
-/

/-!
### 8-60 `PutRules`：页面写一栋楼的 `RULES.toml`，整份、带基线、先求值后落盘

```rust
// Command
PutRules(RulesWrite)
pub struct RulesWrite { pub building: Address, pub base: String, pub body: String, pub idem: IdemKey }
```

- **整份文本，一道基线。** `body` 是新的整份 `RULES.toml`，`base` 是页面起手时读到的那份（文件还不存在时为空串）。楼规有两个写者——人在页面上，以及住在楼里的市长经 `rules` 工具提案——所以与 `PutSpine` 同一条守卫：文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`，什么都不写。
- **先求值，后落盘。** 城先用读楼规的同一个求值器（`city::evaluate`）读 `body`，读不出——不合 TOML、缺 `confidential`、机密楼列了出网域名——就拒，盘上不动；求值通过才经基线守卫整份换上去（city-SPEC §8-34）。所以盘上的楼规永远是这个构建读得懂的那一份，下一次派活不会因为一次保存而打不开这栋楼。
- **账上一行 `rules_changed`。** 写成之后城记一行 `rules_changed { scope: building, which: "RULES.toml", before, after, bytes }`，与派活前核对楼规的那一行同形（`crates/kernel/Spec.lean` §8-4）；所以下一次派活看到的摘要与账上一致，不会把这次保存读成「有人绕过了门改了文件」。
- **载荷是一个值。** `PutRules` 与 `ConfigureCity` 各带一个结构体而不是平铺的字段，线上形状与平铺时相同（`{"put_rules":{…}}`）；理由是 `Command` 住的文件与 `From<WireCommand>` 那个函数都已经贴着长度上限，一个值占一行。
- **远程设备带不进来。** 楼规决定一栋楼能出网、能开浏览器与桌面，所以它在 §19-2 的 `class` 列是 `LocalOnly`：不论设备配对成什么，这条帧都只能在城自己的机器上发。
- 验收：accounting 的 `a_rules_write_against_a_moved_file_or_that_does_not_evaluate_lands_nothing`（过期的 `base` 被拒、求值失败被拒，两次之后文件不变、账上没有 `rules_changed`；对的 `base` 与能求值的正文落盘并记一行）。
-/

/-!
### 8-61 城一级的配置与核心优先级可写：`ConfigureCity`、`PreferencePatch::CorePriority`

```rust
// Command
ConfigureCity(CitySettings)
pub struct CitySettings { pub keep_warm: Option<KeepWarm>, pub effort: Option<Effort>, pub idem: IdemKey }
// PreferencePatch 多一臂
CorePriority(CorePriority)                 // {"core_priority":"raised"} | {"core_priority":"normal"}
pub enum CorePriority { Raised, Normal }   // 值集与拼法住这里，accounting::person 读写 `[core] priority` 用的就是它
```

- **城那一层，两个键。** `ConfigureCity` 写城自己那份 `CONFIG.toml`（`<city>/.sprawling/CONFIG.toml`）：`keep_warm` 写 `[cache] keep_warm`，`effort` 写 `[model] effort`；`None` 不动那一项。城那一层从梯子的最远一端说话，楼与房间各自的一层照旧压过它（city-SPEC §8-4）。`ConfigureBuilding` 不改：它的地址就是它写的楼，城那一层没有地址可写，所以是另一条帧，而不是 `ConfigureBuilding` 收一个特殊地址。
- **账上一行。** 写成之后城记一行 `rules_changed { scope: city, which: "CONFIG.toml", … }`，与派活前核对城配置的那一行同形。
- **核心优先级是这个人自己那一层。** `CorePriority` 进 `PreferencePatch`，所以经已有的 `PutPreferences` 写，落在 `~/.sprawling/config.toml` 的 `[core] priority`——那是它一直住的地方（sprawling-SPEC 8-93），不在 `[ui]` 里，所以 `PreferencesAnswer` 不带它；页面从 `Query::Doctor` 的核心一项读到它此刻的效果。写下之后，下一次 `serve` 起线程时读它。
- 验收：city 的 `a_city_setting_lands_in_the_city_layer_and_the_rooms_read_it`（城层写 `keep_warm` 之后，一间没有说话的房间读到 `FiveMinute`；写 `effort` 之后梯子答它来自城那一层）；accounting 的 `the_core_priority_lands_in_its_own_section_and_reads_back`。
-/

/-!
### 8-72 保存一份文档：`Command::PutRange` 与它的回执

```rust
// Command
PutRange(RangeWrite)                                  // 线上 {"put_range":{"doc":…,"baseline":…,"edits":[…],"idem":…}}
pub struct RangeWrite {
    pub doc: Address,                                 // 城里的哪一份文件
    pub baseline: B3Hash,                             // §8-69 答出的 version：这些编辑是在哪一版上做的
    pub edits: Vec<documents::TextEdit>,              // 那一版的几段字节，各换成一段文本；按文档序、互不重叠
    pub idem: IdemKey,
}
pub struct TextEdit { pub span: documents::Span, pub text: String }   // documents 定义，线上直接携带（documents D1）
// 回执：账本行 document_written { at, baseline, version, bytes }，带这条命令的 idem（`crates/kernel/Spec.lean` §8-83）
```

- **基线是一个版本，编辑是那一版的字节区间加一段文本。** 页面把它的光标与选区换算成字节（refrain 路线图 §4-8 的 `core/document_pos.ts`），替换的内容以文本送来，城按那一版的编码把它写成字节（documents D11）：页面不必知道一份 UTF-16 文件怎样拼一个字符，城里也只有一处会写这几种编码。
- **判定在落盘之前，次序固定。** 地址在保留子树里（`Address::is_reserved`：城与楼的规则、配置、治理文档、`.git`）拒 `E_OUTSIDE_WRITE_DOMAIN`——那些文件各有自己的门（§8-59、§8-60、§8-61），它们要先求值或先改写身份区，这扇门不做；文件此刻的摘要不是 `baseline` 拒 `E_VERSION_CONFLICT`；那一版不是文本（§8-69 的 `Opaque`）、编辑乱序或重叠或越过末尾、改完之后不再是同一种编码的文本（劈开了一个字符、在没有标记的 UTF-8 里写进 NUL）都拒 `E_INVALID_ARGS`（documents D8、D11、D12）。任何一种拒绝之后盘上都没有动，页面的草稿留着，重读再改。
- **两个同基线的保存只落先到的那个。** 读、判、整份换上在城的文档锁里一次做完（city-SPEC §8-40），所以第二个保存读到的已经是第一个落下的版本，摘要不再是它的基线。城外的写者（人的编辑器、居民的 `edit`）不受这把锁约束，挡住它们的是同一个基线：它们一改，摘要就变了。
- **没有文件读作空字节。** 基线是空字节的摘要时（§8-69 的 `Empty`，或页面要新建一份文件），一次保存把文件建起来；上面的目录随之建起。
- **回执是账本行，不是命令的答复。** 命令的答复只送拒绝（§8-2）；成功时城写一行 `document_written`，带这条命令的 `idem`。页面见到带着自己那个 `idem` 的一行才显示「已保存」，并从 `version` 读到下一次保存的基线；在那之前只显示「保存中」。丢了答复的页面用同一个 `idem` 再发，城答它第一次的结果，不再写第二次（accounting 的 `commanding::entrance`）。
- **`WIRE_V` 不另进位**：`PutRange` 是新名字，哈希自己会变（D1）。
- 验收：accounting 的 `worker::commanding::tests::saving`：`a_second_save_from_the_same_version_is_refused_and_the_first_stands`——两个从同一版出发的 `PutRange`，先到的落下并写一行带新版本的 `document_written`，后到的得 `E_VERSION_CONFLICT`，文件是先到者的字节；`a_save_inside_the_reserved_subtree_is_refused`。
-/

/-! D10 保存带基线版本与文本编辑，回执是账本行；提案按文档成批决定

**决定**：(a) `PutRange` 带基线版本（32 字节的摘要）与那一版的几段字节区间，每段换成一段文本，城按那一版的编码写回（§8-72）。(b) 保存成功的回执是带着这条命令 `idem` 的 `document_written` 行，不是命令的答复。(c) `DecideProposals` 一次带一份文档上的几张卡，接受的部分合成一次保存（§8-73）。

**理由**：(a) 基线是版本而不是整份正文：整份正文要随每一次保存往返一次，而版本身份已经在 §8-69 的答复里（documents D3、D8）。编辑带文本而不带字节：字节要页面按 UTF-16 或带标记的 UTF-8 自己编码，那是编码规则的第二个家；城手里有那一版，知道它的编码。(b) 命令的答复在这条线上只有拒绝一种形状（§8-2），给一种命令另开一个成功答复就是第二种回执；账本行本来就是别的写命令（`PutDocument`、`PutSpine`）的回执，而且它在城重开、页面重连之后仍然在，页面凭 `idem` 补拉就能知道那一次落没落下。(c) 同一版上的几张卡一张一张决定，第一张接受之后版本就动了，其余每一张都会因基线过期被拒，人只能接受一张、等 run 重提；成批决定让它们落在同一次保存里，重叠由同一个事务判（documents D8）。

**被否**：①`PutRange` 带字节（base64）：理由见 (a)；②成功时回一帧带新版本的答复：理由见 (b)，而且丢了这一帧的页面无从补问；③提案的决定一张一张发：理由见 (c)；④提出提案也是一条线上命令：提案是 run 说的话，线上的发信方是人，一条让人以 run 的名义说话的命令没有读者。

**重开参数**：页面要支持「另存为另一种编码」时（refrain 路线图 A5），编码成为 `RangeWrite` 的一个字段；有了提出提案的工具之外的第二个提出者（例如城外的 agent 经远程门）时，重议 (④)。
-/

namespace Wire.Command.Step

/-- `PutRange` 在落盘之前的拒绝，次序固定（§8-72）：保留子树里的地址、基线不是文件此刻的版本、编辑本身不成立。 -/
inductive Refusal where
  | OutsideWriteDomain
  | VersionConflict
  | InvalidArgs
  deriving DecidableEq, Repr

/-- 一次保存：它说的基线版本、编辑成不成立，与编辑落下之后的版本。地址在不在保留子树是 `reserved`。版本是模型参数（Rust 里是整份字节的 `B3Hash`）。 -/
structure Save (Version : Type) where
  reserved : Bool
  baseline : Version
  valid : Bool
  next : Version

/-- 判定与落下：拒绝时答拒因，文件仍是 `current`；落下时文件换成 `next`。读、判、整份换上在城的文档锁里一次做完（city-SPEC §8-40），所以在模型里是一步。 -/
def putRange {Version : Type} [DecidableEq Version] (current : Version) (save : Save Version) :
    Except Refusal Version :=
  if save.reserved then .error .OutsideWriteDomain
  else if current ≠ save.baseline then .error .VersionConflict
  else if !save.valid then .error .InvalidArgs
  else .ok save.next

/-- 一次保存之后文件此刻的版本。 -/
def after {Version : Type} (current : Version) : Except Refusal Version → Version
  | .ok next => next
  | .error _ => current

/-- **拒绝之后盘上没有动。** -/
theorem a_refused_save_leaves_the_file {Version : Type} [DecidableEq Version]
    (current : Version) (save : Save Version) (why : Refusal)
    (refused : putRange current save = .error why) :
    after current (putRange current save) = current := by
  rw [refused]
  rfl

/-- **保留子树先判**：那些文件各有自己的门，基线对不对都不让这扇门写。 -/
theorem the_reserved_subtree_is_refused_first {Version : Type} [DecidableEq Version]
    (current : Version) (save : Save Version) (inside : save.reserved = true) :
    putRange current save = .error .OutsideWriteDomain := by
  simp [putRange, inside]

/-- **两个同基线的保存只落先到的那个**：第一次换上了一个不同于基线的版本，第二次读到的已经不是它的基线，得 `E_VERSION_CONFLICT`，文件是先到者的字节。 -/
theorem two_saves_from_one_version_land_once {Version : Type} [DecidableEq Version]
    (current : Version) (first second : Save Version)
    (outside₁ : first.reserved = false) (outside₂ : second.reserved = false)
    (based₁ : first.baseline = current) (based₂ : second.baseline = current)
    (sound : first.valid = true) (moved : first.next ≠ current) :
    putRange current first = .ok first.next ∧
      putRange first.next second = .error .VersionConflict := by
  constructor
  · simp [putRange, outside₁, based₁, sound]
  · simp [putRange, outside₂, based₂, moved]

/-- 一次从文件此刻的版本出发、编辑成立的保存落得下：守卫不是拒绝一切。 -/
theorem a_save_from_the_current_version_lands {Version : Type} [DecidableEq Version]
    (current : Version) (save : Save Version) (outside : save.reserved = false)
    (fresh : save.baseline = current) (sound : save.valid = true) :
    putRange current save = .ok save.next := by
  simp [putRange, outside, fresh, sound]

end Wire.Command.Step
