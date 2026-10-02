-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::chosen_path

规定 `tools::chosen_path`（`crates/runtime/src/` 下同名的文件）。模型选路的唯一判定：文法、保留区、读界，以及判的是盘打开的那个地址。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
#### 8-30-1 runtime::tools::chosen_path（形状 1；模型选路的唯一判定处）


```rust
// 本 run 能读什么：装配层把 kernel::address::may_read 闭合在读者的楼与城的规则上（`crates/city/Spec.lean` §8-2）。
pub type ReadBound = Arc<dyn Fn(&Address) -> ReadVerdict + Send + Sync>;
// 入一个字符串，出一个地址或一个三段式拒绝。本身无 I/O；bound 可能为他楼读一次规则。
pub(crate) fn admit(asked: &str, action: &'static str, bound: &dyn Fn(&Address) -> ReadVerdict)
    -> Result<Address, AxError>;
// 解析失败＝E_INVALID_ARGS；Address::is_reserved()＝E_GATE_DENIED；
// 读界答 Confidential 或 RulesUnreadable＝E_GATE_DENIED，后者的 subject 带上规则读不出的原因。
// 已准入的地址在盘上真正落到哪里：解析真实路径（沿途每一个 symlink 或 junction），把它在城里的地址
// 再交给 admit 判一次，返回真实路径。落在城外＝E_GATE_DENIED；落到保留区或关上的楼＝admit 的那条拒绝；
// 文件不存在也同样判，真实位置由 real_location 求出，答 Located::Absent；解析失败于别的原因＝E_STORAGE_FATAL。
pub(crate) fn land(city_root: &Path, addr: &Address, action: &'static str,
    bound: &dyn Fn(&Address) -> ReadVerdict) -> Result<Located, AxError>;
pub(crate) enum Located { Present(PathBuf), Absent(PathBuf) }
// 一条路径在盘上的真实位置，末尾几段不存在也算：盘解析最深的那个存在的祖先（沿途链接全解开），
// 其下不存在的段原样接上——不存在的段不可能是链接。接上的段不是普通名字（`..` 在内）＝E_GATE_DENIED；
// 路上某个存在的条目解析不了（目标已不在的链接在内）＝E_STORAGE_FATAL。
pub(crate) fn real_location(written: &Path, action: &'static str, subject: &str) -> Result<Located, AxError>;
// 打开之后再核一次：判过的真实位置仍解析到它自己（其上没有新放的链接），且此刻在那里的文件就是打开的那一个；
// 否则＝E_GATE_DENIED。read 与 BoundReader（§8-59）打开文件后都调它。
pub(crate) fn still_judged(asked: &str, judged: &Path, opened: &same_file::Handle, action: &'static str)
    -> Result<(), AxError>;
// 模型写下的路径在城里的拼写。本平台不算绝对路径的（`Path::is_absolute`，D4）原样交回；绝对路径解开真实位置（real_location）后落在
// 城根的真实位置之下，交回它相对城根的拼写（段以 `/` 相连，城根本身交回空串）；落在城外＝E_GATE_DENIED，
// 恢复语说出 `read`／`search`／`edit` 只到城内、城外的文件经 `exec` 读。它不判保留区与读界：交回的拼写
// 照旧过 admit 与 land。
pub(crate) fn within_city<'a>(city_root: &Path, asked: &'a str, action: &'static str)
    -> Result<Cow<'a, str>, AxError>;
// 遍历者（search）对一个条目的判定：不是链接的条目只问盘一次，是目录交 Directory、否则交 File
// （盘说不出类型的也交 File，让随后的打开说出看不了的原因）；链接经 land 判，落到文件交 File(真实路径)，
// 落到目录或不存在交 Passed（走链接可能绕回走过的地方），land 以 E_GATE_DENIED 拒的交 Passed，
// land 的其余拒绝原样返回，由遍历者计入看不了的项。
pub(crate) fn walked(city_root: &Path, path: PathBuf, rel: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict) -> Result<Walked, AxError>;
pub(crate) enum Walked { Directory(PathBuf), File(PathBuf), Passed }
```

**城内的绝对路径换成它的地址，再走同一道判定。** 页面把拖进输入框的文件存到城里（`hall/dropped/…`），插进消息的是文件在盘上的绝对路径；模型照抄它看到的路径，于是 `read` 以文法拒绝一个本就在城里的文件，resident 只能回头让人重打一遍。`read`、`search` 的起点与 `edit` 因此先调 `within_city`，再把交回的拼写交给 `admit`（`edit` 交给 `Address::parse` 与写域门）。换算只做「这是城里的哪个地址」这一件事，保留区、读界与链接仍由原来那一处判，所以没有第二个权威。落选的方案：在页面上把路径改写成城相对地址——页面不知道城根的真实位置（链接、junction、大小写），而且模型从别处（`exec` 的输出、日志）拿到的绝对路径同样会被拒。

**缺失的文件按它会落在哪里判，与存在的文件同一个函数。** 若文件不存在就交回字面路径，链接背后的缺失文件就绕过了判定：随后的「没命中」会列出链接目标那个目录的条目——城外的、机密楼的、包外的。`land`、`read::package` 与 `read::miss` 都经 `real_location` 求真实位置，各自只判「落点在不在我的范围里」；`..` 若接在已解析的祖先之后，会在盘从未看过的地方退出那个目录，所以拒绝而不接。

**判定时不在的文件，之后也不打开。** 接上的尾段是盘在判定那一刻没有的东西；若随后照这条路径打开，判定与打开之间在那里放下的一条链接会把打开带到它指向的任何地方——判过的是一处，读到的是另一处。所以 `real_location` 把「全都在」与「尾段不在」分成 `Located` 的两臂，调用方对 `Absent` 只报缺、不打开：`read` 直接答没命中，`search` 的起点答「不是城里的地方」，遍历里的链接略过。

**判的是盘打开的那个地址，不只是模型写下的那个。** 文法准入的地址仍可能穿过一个链接：开放楼里一条指向机密楼、保留区或城外的链接，打开的是链接的目标，只判字面地址就等于把 admit 拒掉的东西从侧门交出去。所以 `read` 与 `search` 的起点都走 `land`，`search` 遍历中遇到的每一个链接也走 `land`——链接的判定只有这一处。

它是 `read` 原有那段判定的搬家，不是它的第二份。三道判定次序固定：文法、保留区、读界——前两道不碰盘，读界为他楼可能读一次规则，所以排最后。`ReadTool::new(city_root, catalog, bound)` 与 `SearchTool::new(city_root, bound)` 各持同一个 `ReadBound` 的一份 `Arc`；装配层建一次，交给两件工具。`search` 遍历时对每一个候选文件同样只问 `Address::is_reserved()`——kernel 的那个原语——所以「什么是保留区」自始至终一个权威；读界则只在城根那一层问，一栋楼整栋开或整栋关（`crates/city/Spec.lean` §8-3「楼是顶层地址」），楼里的条目继承楼的答案，不为每个文件再读一次规则。
-/

namespace Runtime.Tools.ChosenPath

/-- 读界对一个地址的答案（`kernel::address::ReadVerdict`）。`RulesUnreadable` 带的原因只进拒词，不进判定。 -/
inductive ReadVerdict where
  | Open
  | Confidential
  | RulesUnreadable
  deriving DecidableEq, Repr

/-- 拒绝的稳定码：`E_INVALID_ARGS`、`E_GATE_DENIED`、`E_STORAGE_FATAL`。 -/
inductive Code where
  | InvalidArgs
  | GateDenied
  | StorageFatal
  deriving DecidableEq, Repr

/-- 判一条模型选的路径要的三个事实。它们各有权威，都不在本 crate：文法是 `kernel::Address::parse`，保留区是 `Address::is_reserved`，读界是装配层把 `kernel::address::may_read` 闭合在读者的楼与城的规则上得到的 `ReadBound`（`crates/city/Spec.lean` §8-2）。模型不重述它们，只把它们当参数，所以地址也是一个任意类型。 -/
structure Rules (Address : Type) where
  parse : String → Option Address
  is_reserved : Address → Bool
  bound : Address → ReadVerdict

/-- 一个已经解析的地址过后两道：保留区，然后读界。保留区在前，因为它不碰盘；读界可能为他楼读一次规则，排最后。 -/
def judge {Address : Type} (rules : Rules Address) (addr : Address) : Except Code Address :=
  if rules.is_reserved addr then .error .GateDenied
  else match rules.bound addr with
    | .Open => .ok addr
    | .Confidential => .error .GateDenied
    | .RulesUnreadable => .error .GateDenied

/-- `chosen_path::admit`：文法、保留区、读界，次序固定。 -/
def admit {Address : Type} (rules : Rules Address) (asked : String) : Except Code Address :=
  match rules.parse asked with
  | none => .error .InvalidArgs
  | some addr => judge rules addr

/-- 判过的地址就是交回的地址。 -/
theorem judge_answers_the_address_it_was_given {Address : Type} (rules : Rules Address)
    (addr judged : Address) (passed : judge rules addr = .ok judged) :
    judged = addr ∧ rules.is_reserved addr = false ∧ rules.bound addr = .Open := by
  unfold judge at passed
  cases reserved : rules.is_reserved addr with
  | true => simp [reserved] at passed
  | false =>
    cases verdict : rules.bound addr with
    | Open =>
      simp [reserved, verdict] at passed
      exact ⟨by rw [passed], rfl, rfl⟩
    | Confidential => simp [reserved, verdict] at passed
    | RulesUnreadable => simp [reserved, verdict] at passed

/-- **准入的地址读得通、不在保留区里、读界说开着。** -/
theorem an_admitted_path_is_open_and_outside_every_reserved_subtree {Address : Type}
    (rules : Rules Address) (asked : String) (addr : Address)
    (admitted : admit rules asked = .ok addr) :
    rules.parse asked = some addr ∧ rules.is_reserved addr = false ∧ rules.bound addr = .Open := by
  unfold admit at admitted
  cases parsed : rules.parse asked with
  | none => simp [parsed] at admitted
  | some found =>
    rw [parsed] at admitted
    have judged := judge_answers_the_address_it_was_given rules found addr admitted
    obtain ⟨same, notReserved, open_⟩ := judged
    subst same
    exact ⟨rfl, notReserved, open_⟩

/-- 读不出的路径是 `E_INVALID_ARGS`，与它可能指向哪里无关。 -/
theorem a_path_that_does_not_parse_is_invalid {Address : Type} (rules : Rules Address)
    (asked : String) (unparsed : rules.parse asked = none) :
    admit rules asked = .error .InvalidArgs := by
  simp [admit, unparsed]

/-- 保留区里的路径在读界被问之前就拒绝：不论读界怎么答，答案都是 `E_GATE_DENIED`。 -/
theorem a_reserved_path_is_refused_before_the_bound_is_asked {Address : Type}
    (rules : Rules Address) (asked : String) (addr : Address)
    (parsed : rules.parse asked = some addr) (reserved : rules.is_reserved addr = true)
    (bound : Address → ReadVerdict) :
    admit { rules with bound := bound } asked = .error .GateDenied := by
  simp [admit, judge, parsed, reserved]

/-- 盘对一个地址的回答：它的真实位置在城里的哪个地址（沿途的链接都解开），或在城外。 -/
inductive Landing (Address : Type) where
  | inside (real : Address)
  | outside

/-- `chosen_path::Located`：真实位置上有没有东西。判定时不在的，之后也不打开。 -/
inductive Located (Address : Type) where
  | Present (real : Address)
  | Absent (real : Address)

/-- 盘：`real_location` 求出的真实位置，以及那里此刻有没有东西。 -/
structure Disk (Address : Type) where
  real : Address → Landing Address
  present : Address → Bool

/-- `chosen_path::land`：判的是盘打开的那个地址，不只是模型写下的那个。真实位置落在城外是 `E_GATE_DENIED`；落在城里，再过一次同一道判定。 -/
def land {Address : Type} (rules : Rules Address) (disk : Disk Address) (addr : Address) :
    Except Code (Located Address) :=
  match disk.real addr with
  | .outside => .error .GateDenied
  | .inside real => match judge rules real with
    | .error code => .error code
    | .ok _ => .ok (if disk.present real then .Present real else .Absent real)

/-- **落下的地方也被判过。** `land` 交回的真实位置不在保留区里、读界说开着，不论模型写下的地址经过了什么链接。 -/
theorem what_lands_was_judged_where_it_lands {Address : Type} (rules : Rules Address)
    (disk : Disk Address) (addr : Address) (located : Located Address)
    (landed : land rules disk addr = .ok located) :
    ∃ real, disk.real addr = .inside real ∧ rules.is_reserved real = false ∧
      rules.bound real = .Open ∧
      located = (if disk.present real then .Present real else .Absent real) := by
  unfold land at landed
  cases where_ : disk.real addr with
  | outside => simp [where_] at landed
  | inside real =>
    cases judged : judge rules real with
    | error _ => simp [where_, judged] at landed
    | ok answer =>
      simp only [where_, judged] at landed
      obtain ⟨_, notReserved, open_⟩ :=
        judge_answers_the_address_it_was_given rules real answer judged
      have same := Except.ok.inj landed
      exact ⟨real, rfl, notReserved, open_, same.symm⟩

/-- 拿掉第二次判定就是一个反例：一个读界开着的地址，经一条链接落进一栋机密楼。只判字面地址，就等于把 `admit` 拒掉的东西从侧门交出去；`land` 拒绝它。 -/
theorem judging_only_the_written_address_lets_a_link_out :
    let rules : Rules Bool :=
      { parse := fun _ => some true, is_reserved := fun _ => false,
        bound := fun building => if building then .Open else .Confidential }
    let disk : Disk Bool := { real := fun _ => .inside false, present := fun _ => true }
    admit rules "open/link" = .ok true ∧ land rules disk true = .error .GateDenied := by
  intro rules disk
  exact ⟨rfl, rfl⟩

end Runtime.Tools.ChosenPath

/-! D4 定规：一条路径算不算绝对路径，由本平台判定

- **决定**：`within_city` 只接手标准库 `Path::is_absolute` 在本平台答「是」的路径。于是同一个拼写 `/abs.txt` 在 Unix 上是城外的绝对路径，`read`、`search`、`edit` 答 `E_GATE_DENIED`，恢复语指向 `exec`；在 Windows 上它没有盘符，不是绝对路径，照旧交给文法，答 `E_INVALID_ARGS`。两个平台给出两个码。
- **理由**：模型照抄的是 serve 所在的系统给它看的路径——拖进来的文件、`exec` 的输出、日志——而这些路径都按本平台的规则写成。判定跟着 `Path::is_absolute` 走，仓库里就没有第二份「什么算绝对路径」的规则。Windows 上 `/x` 与 `\x` 指向进程当前盘的根，当前盘由启动 serve 的方式决定，不是城的属性。
- **击败的备选**：在 Windows 上把根相对路径按当前盘补全后再判。这要给 `within_city` 加一条只在 Windows 上存在的分支，而且同一条路径会随进程的当前盘落到不同地方。
- **重开的参数**：某个页面或 harness 在 Windows 上以根相对的形式给出城内文件的路径，模型照抄后被文法拒绝。
-/
