-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::address

规定 `kernel::address`（`crates/kernel/src/address.rs`）：地址文法、段边界、受保护元数据与保留谓词、读界、`SessionName`。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-2 kernel::address

```rust
pub struct Address(String);             // 不变量在唯一构造点强制；无 setter
pub const RESERVED_PREFIX: &str = ".sprawling";
pub const GIT_METADATA: &str = ".git";
pub const PROTECTED_METADATA: [&str; 2] = [RESERVED_PREFIX, GIT_METADATA];  // 名单唯一住处（8-73）

impl Address {
    /// Sole constructor. Grammar: relative, `/`-separated, segments of
    /// non-control UTF-8; rejects absolute (incl. drive/UNC), `..`, `.`,
    /// empty segments, backslash, NUL/control, leading/trailing `/`,
    /// and any segment ending in a dot or whitespace.
    pub fn parse(raw: &str) -> Result<Self, AxError>;   // E_INVALID_ARGS
    pub fn is_within(&self, prefix: &Address) -> bool;  // 段边界字节前缀；WriteDomain 原语
    pub fn is_reserved(&self) -> bool;                  // 任一段 ASCII 大小写不敏感命中 PROTECTED_METADATA 之一（C17，见 8-28、8-55、8-73）
    pub fn as_str(&self) -> &str;
    pub fn name(&self) -> &str;                         // 最后一段：人起名时打下的那个词
}
```

- `name` 是地址的最后一段。一个居民、一个房间在名册与页面上以它称呼，而不以整条路径称呼：整条路径读起来像路径，最后一段读起来像某个人。语法保证至少一段且段非空，所以它总有答案。这件事写在类型上一处，城的名册、楼的页面与 prefix 里的「你的名字」读的是同一个定义。
- 派生：`Clone/Debug/Display/PartialEq/Eq/PartialOrd/Ord/Hash`（BTreeMap 键）。
- serde：呈现为字符串；`Deserialize` 经 `parse` 复验（fail-closed 读盘）。
- 同一性按字节：`Eq`、`Ord` 与 `is_within` 在所有平台上逐字节比较，大小写不同的两个地址因而是两个地址；`is_within` 自反（`a.is_within(a)`）。**只有 `is_reserved` 折叠 ASCII 大小写**，因为它是一道只允许多拒的门，而它守的目录名在文件系统那边是大小写不敏感的（8-55）。
- 符号链接 canonicalize 属效果面（write_domain 的适配层）；本原语只对已规范化相对路径作证。
- 解析拒绝的 AxError：`action="parse address"`、`subject=原串`、recovery 指出违规成分与合法形态。
- 判例表只有一份：`tools/fixtures/address.jsonl`，每行一个拼写与它的判决（`accepted`／`refused`，拒绝行附所违规则）。`Address::parse` 的测试、`schema` 给客户端的 `ADDRESS_PATTERN`、客户端 `address.test.ts` 对生成出的 schema，三个读者读同一个文件；表放在两种语言之外，是因为放在任何一边都会让另一边抄一份，而抄本的条数会各自漂移。

**读界**（`crates/city/Spec.lean` §8-2 confidential 的第四条；形状 1 判定）：

```rust
pub enum ReadVerdict { Open, Confidential, RulesUnreadable(AxError) }   // 穷尽；不是 bool
pub fn may_read(
    reader_building: &Address,
    target: &Address,
    rules: impl FnOnce() -> Result<bool, AxError>,   // 目标所在楼此刻是否 confidential
) -> ReadVerdict;
```

- 规则一句话：`target.is_within(reader_building)` 即 `Open`，且**不调用 `rules`**；否则问 `rules`——`Ok(false)`＝`Open`，`Ok(true)`＝`Confidential`，`Err(e)`＝`RulesUnreadable(e)`。后两臂都是「关」，分成两臂是因为恢复语不同：前者请去问那栋楼里的人，后者要一个人去修那份规则。
- `rules` 是闭包而不是值：本楼的读是绝大多数，它们不该为一次读盘付账；「哪栋楼持有 target」与「读它的规则」是 city 的权威（`city::Building::of`、`city::policy::load`），由装配层接成这个闭包，kernel 不复述「楼是地址的首段」。它交出的是 `BuildingPolicy::confidential` 那一个事实，形状因此是 `bool`：本模块不引 `model`，`address` 保持叶子模块。
- 拒绝的措辞不在这里：`ReadVerdict` 是判定，拒词由问它的人写（`crates/runtime/Spec.lean` §8-30-1 `chosen_path::admit`，那是模型选路唯一的拒绝处）。
-/

/-!
### 8-29 kernel::address::SessionName（形状 2 value）

```rust
pub struct SessionName(String);
impl SessionName {
    pub fn parse(raw: &str) -> Result<Self, AxError>;   // 唯一构造点；修剪两端
    pub fn as_str(&self) -> &str;
}
```

一个人给一次会话的名字会变成它干活的目录，所以它必须恰好是一个地址段。用 `String` 就是把这条规则散到每个记得它的调用方里。

- **段规则向 `Address::parse` 问，不重写**：反斜杠、`:`、控制字符、尾随点与尾随空白这些依据只有一份；本类型只多拒 `/`、`.`／`..`、`.sprawling` 与 64 字符上限。修剪发生在问之前，所以人多敲的尾随空格仍然被原谅，而尾随点落到 `Address` 的那条拒绝上。
- **修剪两端而不改中间**：人多敲一个空格不是意图；把中间的空格换成连字符则是替他取名。
- **64 字符**：超过这个长度的不是名字，是一件被写进名字栏的任务；它还要当别人机器上的目录名。
- **serde 进口复验**：`Deserialize` 走 `parse`（同 `Address`），于是一个从线上来的名字不会因为发送方没检查而变成路径。
-/

/-!
### 8-28 `is_reserved`：任一段命中受保护名单（形状 2 value 的一条原语）

```rust
pub fn is_reserved(&self) -> bool;   // 任一段命中名单之一即真（ASCII 折叠，见 8-55）
```

**理由是写域默认覆盖整栋楼。** 一次派活的写域由 `city::policy::write_domain()` 给出，`crates/city/templates/RULES.toml` 的 `prefixes` 出厂是空表，于是写域回落到 `[self.addr]`——整栋楼；`runtime::tools::edit` 对路径只有 `WriteDomain::admits` 一道依据，`city::load` 又在每次派活时重读 `RULES.toml`。楼的 `RULES.toml` 与 `CONFIG.toml` 住在 `<building>/.sprawling/` 下（`kernel::layout`，8-56），所以只看首段的判定挡不住一个 agent 改自己楼的写域、`confidential`、思考强度与 MCP server；任一段命中才挡得住，词汇表「一个 agent 改不了自己的账与自己的配置」由此得到执行。

- **一条规则，三处实例**：一个 scope 的治理字节住在它自己的 `.sprawling/` 里。城是 `<city>/.sprawling/`，楼是 `<building>/.sprawling/`，房间是 `<building>/<room>/.sprawling/`。城的布局因此不是特例，而是同一条规则在根 scope 上的实例。
- **失效关闭，只会拒绝得更多**：`is_reserved` 对任何含 `.sprawling` 段的地址答真，`WriteDomain::new` 与 `admits` 两处因此同时收紧。
- **`Address::parse` 的文法不管这件事**：`.sprawling` 是一个合法段名，只是含它的地址不可写；`RESERVED_PREFIX` 在词汇表里叫 reserved prefix。
-/

/-!
### 8-73 受保护元数据并入保留谓词（形状 1 判定的扩面）

```rust
pub const RESERVED_PREFIX: &str = ".sprawling";
pub const GIT_METADATA: &str = ".git";
pub const PROTECTED_METADATA: [&str; 2] = [RESERVED_PREFIX, GIT_METADATA];
pub fn is_reserved(&self) -> bool;   // 任一段命中名单之一即真（ASCII 大小写不敏感）
```

- **验收标准是「写某路径即提权」。** `.git` 里放着 hooks（写下一个 hook 就是在下一次 git 操作时执行自己的代码）、config（`core.fsmonitor` 等键即执行）、refs（检查点引用 `refs/sprawling/runs/<run>/<seq>` 是 run 自己的记账）与对象库（`file_discarded` 的恢复地址指向的对象）。一个写得了 `.git` 的 run 既能提权也能改自己的账，与 `.sprawling` 同罪，故同门。
- **并入保留谓词，不另立写目标名单。** 写域构造（`WriteDomain::new`）、写域判定（`admits`／`reaches`）、读路径（`runtime::tools::chosen_path`）、`SessionName`、`storage::reserved` 全部已经问这一个谓词（或这一个名单），名单一扩即全体收紧；另立一份「写目标名单」就是给同一个问题两个家。
- **读面一并收紧，这是有意的。** `is_reserved` 是只许多拒的门（8-28）：`read`／`search` 对 `.git` 由可读变拒读，只多拒不错放；`search` 不再另写一行跳过 `.git` 的字面量。
- **`SessionName` 的保留名判定问 `is_reserved`**：逐字节比较会让 `.SPRAWLING` 当房间名建出 Windows 别名目录；问 `is_reserved`，名单与 ASCII 折叠就与地址谓词同源。
- **被否的另一条路：只在写判定处加 `.git`，读判定不动。** 那要维护「写名单」「读名单」两份名单、两个家，而读 `.git` 只会把对象库字节当普通文件递给模型，没有任何读者需要它。
- **重开参数**：出现第二种「写下即提权」的元数据目录、或 `.git` 不再是其中之一时改名单；名单成员必须全 ASCII（8-55 的折叠论证随名单走，`GIT_METADATA` 改拼写的提交必须同步改比较方式）。
-/

/-!
### 8-55 保留子树的 Windows 别名（形状 1 判定的两条收紧）

```rust
pub fn parse(raw: &str) -> Result<Self, AxError>;   // 任一段以点或空白结尾即拒
pub fn is_reserved(&self) -> bool;                  // eq_ignore_ascii_case
```

**理由是 Win32 的路径别名。** 地址最终由 `city_root.join(addr.as_str())` 交给文件系统，而 Win32 在打开文件前剔掉每一段的尾随点与尾随空格，并以大小写不敏感的方式解析目录名。于是 `lab/.SPRAWLING`、`lab/.sprawling.`、`lab/.sprawling ` 三种拼法落到 `lab/.sprawling` 这同一个目录。读路径的 `runtime::tools::chosen_path` 与写路径的 `kernel::write_domain` 共用这一个谓词，逐字节比较时，一个 run 换一种拼法就写得了自己楼的 `RULES.toml`、自己的 `CONFIG.toml` 与账本目录，8-28 的不变式被拼写绕过。

- **在文法层拒绝别名，而不是在判定层认识别名**：尾随点与尾随空格被 `parse` 一次性拒掉，于是这两种拼法根本构造不出 `Address`，`is_reserved` 之后的每一个读者都不必再知道 Win32 的这条规矩。判定层只留大小写一条，因为大小写别名无法在文法层拒绝——`.SPRAWLING` 是一个人可能真心想要的目录名。
- **ASCII 折叠够用，理由是保留名自己**：`RESERVED_PREFIX` 全是 ASCII，`eq_ignore_ascii_case` 对它给出的答案与 NTFS 的大写表一致；引入 Unicode 折叠会把一张随版本变的表搬进 kernel，而它多认的字符一个也不在这个常量里。
- **同一性仍按字节**：`Eq`、`Ord` 与 `is_within` 不折叠大小写。两个方向都安全：写域 `lab` 不收 `LAB/x`，写域 `LAB` 也不收 `lab/x`，失配一律是拒绝。这条不对称是有意的——`is_reserved` 是只许多拒的门，`is_within` 是身份关系，让身份关系折叠大小写会让两个不同地址变成一个。
- **失效关闭，只会拒绝得更多**：收紧只让以点或空格结尾的地址被拒，树里没有调用点写这样的字面量地址。
- **重开参数**：①出现一种不经 `join` 而直接与操作系统打交道的地址消费者，剔尾规则因此不再适用——届时收紧点应下移到那个适配层；②8.3 短名（`SPRAWL~1`）与 Unicode 大写表撞上 ASCII（如 U+212A）这两类别名本文不管，因为前者要问文件系统才知道、后者不出现在 `RESERVED_PREFIX` 里，任何一条被实际做成攻击即重开；③若将来保留名不再全是 ASCII，`eq_ignore_ascii_case` 当场失效，改常量的同一个提交必须改这条比较。
-/

namespace Kernel.Address

/-! ### 模型

一个地址就是它的段：`Address::parse` 收下的串按 `/` 切开。段的文法——不空、不是 `.`／`..`、不含反斜杠、`:` 与控制字符、不以点或空白结尾，以及至少一段——由 `Address::parse` 在唯一的构造点判，判例表是 `tools/fixtures/address.jsonl`；本模型不重述字符级的规则，只说段一级的两条关系：`is_within` 与 `is_reserved`。

`is_within` 在 Rust 里是「字节前缀，且剩下的部分为空或以 `/` 开头」；对已经过了文法的地址，它与「段序列的前缀」是同一件事，因为段里没有 `/`。这条对应由 `address::tests` 的 proptest（自反、传递、反对称）与 `".sprawlingx/a"` 这类段边界判例检查，不在这里证明。 -/

/-- 一个地址的段。 -/
structure Address where
  segments : List String
  deriving DecidableEq, Repr

/-- `Address::is_within`：`a` 等于 `prefix` 或在它下面，按段边界。写域的原语。 -/
def is_within (a pre : Address) : Bool :=
  pre.segments.isPrefixOf a.segments

theorem is_within_iff (a pre : Address) : is_within a pre = true ↔ pre.segments <+: a.segments :=
  List.isPrefixOf_iff_prefix

theorem is_within_refl (a : Address) : is_within a a = true := by
  rw [is_within_iff]
  exact List.prefix_refl _

theorem is_within_trans {a b c : Address} (ab : is_within a b = true) (bc : is_within b c = true) :
    is_within a c = true := by
  rw [is_within_iff] at *
  exact List.IsPrefix.trans bc ab

theorem is_within_antisymm {a b : Address} (ab : is_within a b = true) (ba : is_within b a = true) :
    a = b := by
  rw [is_within_iff] at *
  have same := List.IsPrefix.eq_of_length ba (Nat.le_antisymm ba.length_le ab.length_le)
  cases a
  cases b
  simp_all

/-- D8 定规：受保护元数据名单只有 kernel::address 一个家

`PROTECTED_METADATA` 是 `.sprawling` 与 `.git` 两个名字的唯一住处，`is_reserved`、`SessionName`、`storage::reserved::outside_reserved` 与 bundle 的 `travels` 全部引用它，任何调用点不得重拼这两个字符串。这条定规的理由是「写某路径即提权」（8-73）；被击败的备选是storage 侧另立一份写目标名单——同一问题两个家，且两个家会各自演化。经链接写受保护元数据的恒拒由 storage 的别名族规则承担（`crates/storage/Spec.lean` §8-25），两半合起来才是「写 `.git/hooks` 即提权」这一个洞的完整封堵。
-/
def is_reserved (protected_name : String → Bool) (a : Address) : Bool :=
  a.segments.any protected_name

/-- **保留子树向下封闭。** 一个受保护的段之下的每一个地址都受保护：写域拿到一个保留区里的前缀就够得到它的全部，所以写域的构造点只需拒前缀，判定点只需拒目标。 -/
theorem a_reserved_subtree_stays_reserved (protected_name : String → Bool) {a pre : Address}
    (reserved : is_reserved protected_name pre = true) (inside : is_within a pre = true) :
    is_reserved protected_name a = true := by
  rw [is_within_iff] at inside
  obtain ⟨rest, whole⟩ := inside
  simp only [is_reserved] at *
  rw [← whole, List.any_append, reserved, Bool.true_or]

/-- 任一段受保护即保留，不论它在第几段：楼自己的规则住在 `<building>/.sprawling/` 下（`kernel::layout`），一次写域是整栋楼的 run 也够不到它们。 -/
theorem a_protected_segment_at_any_depth_reserves (protected_name : String → Bool)
    (above below : List String) (name : String) (protects : protected_name name = true) :
    is_reserved protected_name ⟨above ++ name :: below⟩ = true := by
  simp [is_reserved, List.any_append, protects]

/-- **只判首段会放过楼自己的规则。** 这是 8-28 要防的那个洞的反例：楼 `lab` 自己的 `RULES.toml` 的地址首段是 `lab`，不受保护；任一段的判定才拒得住它。 -/
theorem judging_only_the_first_segment_lets_a_building_rules_through :
    let protected_name := fun segment => segment == ".sprawling"
    let rules : Address := ⟨["lab", ".sprawling", "RULES.toml"]⟩
    (rules.segments.head?.map protected_name = some false) ∧ is_reserved protected_name rules = true := by
  decide

/-- 读界的答案，与 `kernel::ReadVerdict` 逐变体同名；`E` 是 Rust 里的 `AxError`。两个关闭的臂分开，因为被拒的模型下一步不同：去问那栋楼里的人，或等一个人去修读不出的规则。 -/
inductive ReadVerdict (E : Type) where
  | Open
  | Confidential
  | RulesUnreadable (error : E)
  deriving DecidableEq

/-- `may_read`：本楼整栋开着；别的楼开着，除非它的规则说机密或读不出。`rules` 是装配层接上的闭包，答「`target` 所在的楼此刻是不是机密」。 -/
def may_read {E : Type} (reader_building target : Address) (rules : Unit → Except E Bool) :
    ReadVerdict E :=
  if is_within target reader_building then .Open
  else
    match rules () with
    | .ok false => .Open
    | .ok true => .Confidential
    | .error unread => .RulesUnreadable unread

/-- **读本楼不问规则。** 答案与 `rules` 无关，所以生产实现可以不调用它：本楼的读是大多数，它们不为一次读盘付账。 -/
theorem reading_ones_own_building_never_asks {E : Type} (reader_building target : Address)
    (rules : Unit → Except E Bool) (inside : is_within target reader_building = true) :
    may_read reader_building target rules = .Open := by
  simp [may_read, inside]

/-- **读不出的规则关上那栋楼。** 隐私设置不朝宽松的一侧失败：读不出的规则可能本来写着 `confidential = true`。 -/
theorem unreadable_rules_close_the_building {E : Type} (reader_building target : Address)
    (rules : Unit → Except E Bool) (outside : is_within target reader_building = false) (unread : E)
    (fails : rules () = .error unread) :
    may_read reader_building target rules = .RulesUnreadable unread := by
  simp [may_read, outside, fails]

end Kernel.Address
