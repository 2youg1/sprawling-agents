-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address

/-!
# city::building

规定 `building`、`building::removal`、`building::template`（`crates/city/src/` 下同名的文件）。一栋楼怎么被建出来、收编、移走，以及一个地址归哪栋楼管；City Hall 随城立起。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-3 city::building（形状 2 值类型＋一个实例化动作）

```rust
pub enum BuildingTemplate { Minimal, Confidential, Hall }   // 穷尽；新模板＝新臂
impl BuildingTemplate {
    pub const ALL: [BuildingTemplate; 3];                            // 三个模板，按人被提供的顺序
    pub fn parse(name: &str) -> Result<BuildingTemplate, AxError>;   // 经 ALL 与 name 读回；不认即拒，且报出已知集
    pub fn name(self) -> &'static str;                               // 模板名的唯一拼法
}
pub struct Building { /* addr —— 私有 */ }
impl Building {
    pub fn of(addr: &Address) -> Result<Building, AxError>;   // 一个地址归哪栋楼管
    pub fn addr(&self) -> &Address;
    pub fn root(&self, city_root: &Path) -> PathBuf;
    pub fn holds(&self, addr: &Address) -> bool;
}
pub fn create(city_root: &Path, addr: &Address, template: BuildingTemplate)
    -> Result<Building, AxError>;
pub fn created_payload(building: &Building, template: BuildingTemplate)
    -> Result<Payload, AxError>;
pub fn adopt(city_root: &Path, addr: &Address) -> Result<Building, AxError>;
pub fn adopted_payload(building: &Building) -> Result<Payload, AxError>;          // adopted: true
pub fn configured_payload(building: &Building, wrote: Written)
    -> Result<Payload, AxError>;                              // building_configured

// building/removal.rs（形状 2 值类型＋一个实例化动作）
pub struct Removed { /* addr、kept —— 私有 */ }
impl Removed {
    pub fn addr(&self) -> &Address;
    pub fn kept(&self) -> &str;          // 楼的文件现在所在处，相对城根、以 / 分段
}
pub fn remove(city_root: &Path, addr: &Address) -> Result<Removed, AxError>;   // city::remove_building
pub fn removed_payload(removed: &Removed) -> Result<Payload, AxError>;         // building_removed
```

- **移走楼＝把目录整个搬进 `.sprawling/removed/<名>`，不删一个字节**：人造的东西一样不丢，楼的历史仍在 Ledger 里，`building_removed` 记下它搬去了哪里。同名的楼第二次被移走时落在 `<名>-2`、`<名>-3`……第一个空位，已搬走的那份不被覆盖。搬用 `std::fs::rename`：同一卷上是一步，半途失败时楼要么还在原处、要么已整个到位。放在 reserved prefix 下，是因为 `all` 不列点开头的目录，而写域够不到那里——被移走的楼不再是楼，也不能被居民改动。找回＝人把目录搬回城根，再 `adopt`。
- **拒绝**：房间地址（`lab/room1` 不是楼，`AxCode::InvalidArgs`）；City Hall（`HALL_BUILDING`，城自己的楼，地址由城定，`InvalidArgs`）；没有目录的地址（`InvalidArgs`）；搬不动（`StorageFatal`，恢复：关掉占用它的程序再试）。三个平台在这里不同：Windows 上楼里有一个文件被别的程序打开（未带共享删除标志）时 `rename` 失败，落到这一拒；macOS 与 Linux 上打开着的文件不挡 `rename`，搬走照常成功，打开它的程序继续读写已搬走的那份；三个平台上权限不足都落到这一拒。有活跃 run 的楼由 sprawling 在调用前拒绝——本模块不知道哪些 run 在跑。

- **模板名只有一个家**：`parse` 不再另列一张字符串表，而是拿 `ALL` 里每一个的 `name()` 去比；拒词里的合法集也由同一趟生成。于是加一个模板只改枚举与 `name()` 两处，而「解析认得的集合」与「拒词列出的集合」在类型上是同一个。`Hall` 的名字取 `kernel::consts_policy::HALL_BUILDING`：City Hall 是唯一一栋地址由城而不是由人定的楼，模板名与那个地址是同一个词。

- **楼是顶层地址，房间不是楼**：`create` 拒多段地址（`lab/room1` 是 `lab` 里的一个房间）。嵌套楼会使「这个地址归谁管」多出一个答案，而 `Building::of` 取首段这件事被写域、配置与上报对象三处消费。
- **reserved prefix 下建楼恒拒**：`.sprawling/` 是城自己的账与配置，它在一切写域之外；允许在它下面建楼，就是把一个写域开到账本上。判定用 `Address::is_reserved`，不在本模块重写前缀文法。
- **二次出生恒拒**：已有 `RULES.toml` 即拒（同 `init` 拒第二次创世）。覆写会把一栋已在干活的楼的规则静默换掉，而那份规则可能写着 `confidential = true`。
- **模板字节来自 `crates/city/templates/RULES.toml`（`include_str!`）**：人读的那份模板与城写出的那份必须是同一串字节，否则两份会各自漂。`Confidential` 与 `Minimal` 只差一行（`confidential` 的值），且该差异由 `policy::evaluate` 读回来断言——换行成功与否不靠阅读，靠测试。
- **先落盘再产事件**：`building_created` 记的是已经发生的事。反过来的顺序会让历史声称一栋目录不存在的楼存在，而重放会把这个谎再说一遍。
- **只写不读的 payload**：`created_payload` 只有写面，因为没有读它的投影——楼列表读盘（`city::buildings`）。读面随第一个真正需要它的投影落地，不提前建。
- **占位符只有一个家**：`NAME_PLACEHOLDER` 住 `building::template`，`pub(crate)`；楼的规则与它的计划、备忘、交接读同一个占位符，两份拼法会让其中一份文件永远写着 `<building name>`。
- **adopt（兑现「导入一个已有目录」这件事）**：收编一个已存在的目录为楼。复用 `create` 的全部围栏（房间拒、reserved 拒、二次出生拒），只多一条：**目录不存在即拒并指向 create**——收编不存在的东西是建造，两个动词不共用一个事实。Spine 文档恒不覆写（§8-5 既有约束），故被收编目录的 `Roadmap.md` 保持原主的字节；事件仍是 `building_created`，但 payload 携 `adopted: true`——历史不得声称它建造了它只是找到的东西。CLI 入口 `sprawling adopt <city> <addr>`；城外目录先由人搬入城内再收编，本体不做拷贝。
-/

/-!
### 8-20 City Hall：随城市立起的那栋楼，和住在里面的两个人

**需求**：城市需要一个规划者和一个代答者。它们服务每一栋楼，因此不属于任何一栋楼；它们写 Markdown 和计划，不建造。

**接口**：

```rust
// city::building
pub enum BuildingTemplate { Minimal, Confidential, Hall }   // Hall 的字节是固定的一份模板

// city::policy
pub enum DomainReach { Everything, Documents }              // RULES.toml 的 `write` 一键
impl BuildingRules { pub fn reach(&self) -> DomainReach; }
// write_domain()：Everything → WriteDomain::new，Documents → WriteDomain::documents

// city::spine_files
pub const MAYOR_FILE: &str = "MAYOR.md";
pub const CLERK_FILE: &str = "CLERK.md";
/// hall/mayor 与 hall/clerk 的身份文件位置；别的地址返回 None。
pub fn hall_identity_path(city_root: &Path, addr: &Address) -> Option<PathBuf>;
/// 把两份模板写进 <city>/.sprawling/，已存在的不覆盖。
pub fn lay_out_hall_identities(city_root: &Path) -> Result<(), AxError>;

// city::wizard
impl CityPlan { pub fn hall(&self) -> &(Address, BuildingTemplate); }   // 恒存在，非 Option
```

- **为什么 `hall` 在 `CityPlan` 里是恒存在的字段而不是 `Option`**：一座没有 City Hall 的城市不是这个版本能形成的东西。可选性会让「城市有没有市政厅」变成调用点每次都要答一遍的问题，而它只有一个答案。
- **两份身份文件住 `<city>/.sprawling/`**：写域碰不到保留子树，所以 Mayor 改不了自己是谁，clerk 改不了自己按什么答。这是 `URBANITE.md` 住在居民自己地址下时拿不到的性质，也是这两位与普通居民唯一的结构差别。
- **路径权威仍只有一个**：`city::resident::urbanite_path` 先问 `spine_files::hall_identity_path`，无答再拼 `<addr>/URBANITE.md`。`Identity::load` 一字不改，因此「有身份文件即居民」这条规则对市政厅与对普通房间是同一条。
- `write = "documents"` 由 `policy::evaluate` 读成 `DomainReach`；**缺这一键即拒**，与 `confidential` 同——读作 `Everything` 会让没见过这个设置的人得到最宽的那一档。值既不是 `everything` 也不是 `documents` 时同样拒绝：读成打字错误的权限设置不能落到宽松那一侧。
- 被否：给 Mayor 一个覆盖全城的 `Everything` 写域，靠 `MAYOR.md` 的措辞请它别碰代码——把不变量交给提示词，等于没有不变量。
-/

/-!
## 模型：一个地址归哪栋楼管，哪些地址能成为楼

地址的段与 `is_within`、`is_reserved` 是 kernel 的模型（`crates/kernel/spec/Address.lean`），这里只加 city 自己拥有的三件：`Building::of` 取首段并拒保留子树、`holds` 就是 `is_within`、`create` 的三道拒。受保护的名字是参数 `protected_name`，它的唯一的家是 `kernel::address`（kernel D8）。

* 一个地址被它的楼管着（`a_building_holds_every_address_it_governs`），楼的楼是它自己（`a_building_is_its_own_building`）。
* 保留子树不属于任何一栋楼（`the_reserved_subtree_belongs_to_no_building`）：账本与城自己的配置在一切写域之外，问「它归哪栋楼」只得到拒绝。
* `create` 收下的地址恰是一栋楼的地址（`a_created_building_governs_itself`）；房间（`a_room_is_not_a_building`）与二次出生（`a_second_birth_is_refused`）各拒一次。
* 移走的楼落在城的保留子树里（`a_removed_building_is_out_of_every_write_domain`）：它不再是楼，居民也改不动它。

模型不说段的字符级文法：那是 `Address::parse` 的，判例表在 `tools/fixtures/address.jsonl`。
-/

namespace City.Building

open Kernel.Address

/-- 地址的首段：它所在的那栋楼的地址。 -/
def head (a : Address) : Address :=
  ⟨a.segments.take 1⟩

/-- `Building::of`：首段就是楼；首段受保护即拒。 -/
def of (protected_name : String → Bool) (a : Address) : Except Unit Address :=
  if is_reserved protected_name (head a) then .error () else .ok (head a)

/-- `Building::holds`：一个地址在这栋楼里。 -/
def holds (building a : Address) : Bool :=
  is_within a building

/-- `create` 的三道拒，按 Rust 的先后：保留子树、房间、二次出生。 -/
inductive Refused where
  | Reserved
  | Room
  | SecondBirth
  deriving DecidableEq, Repr

/-- `building::create`：`has_rules` 是楼的保留子树里已有 `RULES.toml`（`create_new` 撞上已在的文件）。 -/
def create (protected_name : String → Bool) (a : Address) (has_rules : Bool) : Except Refused Address :=
  match of protected_name a with
  | .error () => .error .Reserved
  | .ok building =>
    if building ≠ a then .error .Room
    else if has_rules then .error .SecondBirth
    else .ok building

theorem a_building_holds_every_address_it_governs (protected_name : String → Bool)
    (a building : Address) (governs : of protected_name a = .ok building) :
    holds building a = true := by
  unfold of at governs
  split at governs
  · cases governs
  · cases governs
    simp only [holds, head, is_within_iff]
    exact List.take_prefix 1 a.segments

theorem the_head_of_a_head_is_itself (a : Address) : head (head a) = head a := by
  simp [head, List.take_take]

theorem a_building_is_its_own_building (protected_name : String → Bool) (a : Address) :
    of protected_name (head a) = of protected_name a := by
  unfold of
  rw [the_head_of_a_head_is_itself]

theorem the_reserved_subtree_belongs_to_no_building (protected_name : String → Bool)
    (name : String) (rest : List String) (protects : protected_name name = true) :
    of protected_name ⟨name :: rest⟩ = .error () := by
  simp [of, head, is_reserved, protects]

theorem a_created_building_governs_itself (protected_name : String → Bool) (a building : Address)
    (has_rules : Bool) (created : create protected_name a has_rules = .ok building) :
    building = a ∧ of protected_name a = .ok a ∧ has_rules = false := by
  unfold create at created
  split at created
  · cases created
  · rename_i b found
    split at created
    · cases created
    · rename_i same
      cases has_rules <;> simp at created
      subst created
      simp only [Decidable.not_not] at same
      subst same
      exact ⟨rfl, found, rfl⟩

theorem a_room_is_not_a_building (protected_name : String → Bool) (name room : String)
    (rest : List String) (open_ : protected_name name = false) (has_rules : Bool) :
    create protected_name ⟨name :: room :: rest⟩ has_rules = .error .Room := by
  simp [create, of, head, is_reserved, open_]

theorem a_second_birth_is_refused (protected_name : String → Bool) (name : String)
    (open_ : protected_name name = false) :
    create protected_name ⟨[name]⟩ true = .error .SecondBirth := by
  simp [create, of, head, is_reserved, open_]

/-- 一栋普通的楼建得起来：上面的拒绝不是靠拒绝一切换来的。 -/
example : create (· == ".sprawling") ⟨["lab"]⟩ false = .ok ⟨["lab"]⟩ := rfl

/-- `building::removal`：移走的楼搬进 `<city>/.sprawling/removed/<名>`。 -/
def removed (reserved removed_dir name : String) : Address :=
  ⟨[reserved, removed_dir, name]⟩

theorem a_removed_building_is_out_of_every_write_domain (protected_name : String → Bool)
    (reserved removed_dir name : String) (protects : protected_name reserved = true) :
    is_reserved protected_name (removed reserved removed_dir name) = true := by
  simp [removed, is_reserved, protects]

end City.Building
