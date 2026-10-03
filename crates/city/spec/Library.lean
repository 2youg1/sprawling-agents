-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::library

规定 `library`、`library::reading`、`library::shelf`、`config_layers::shelves`（`crates/city/src/` 下同名的文件）。书架上有什么：城库、楼架与城外书架，阅览室准入哪几件。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-8 city::library（形状 2 值类型＋形状 1 判定）

```rust
pub use kernel::layout::{BUILDING_SHELF, LIBRARY_DIR};   // 住 reserved prefix 之下
pub enum Shelf {
    Library(Address),
    Building(Address),
    External { index: u32, path: String },
}
impl Shelf { pub fn address(&self) -> Option<&Address>; }
pub struct Holding { pub name, pub section, pub disclosure, pub hash, pub shelf: Shelf, pub package: Option<Address>,
                     pub carried: Option<String> }   // 城外书架上的一件：扫描读到的正文；城内书架为 None
pub struct Library { /* BTreeMap<ShelfKey, Holding> —— 私有，ShelfKey 由 name 造 */ }
impl Library {
    pub fn scan(city_root: &Path, building: Option<&Address>, home: &Path) -> Result<Library, AxError>;
    pub fn all(&self) -> Vec<&Holding>;
    pub fn sections(&self) -> Vec<&str>;
    pub fn reading_room(&self, admitted: &[String]) -> Vec<&Holding>;
    pub fn missing(&self, admitted: &[String]) -> Vec<String>;
}
// city::config_layers
pub fn city_shelves(city_root: &Path, home: &Path) -> Result<Vec<PathBuf>, AxError>;
```

- **中央库存与阅览室的分工是常驻上下文不随磁盘膨胀的唯一原因**：盘上躺一千件与本楼的常驻字节无关；进 catalog 的只有 `RULES.toml` 的 `reading_room` 列出的那几件。
- **库存住 reserved prefix**：Agent 读得到、写不了。否则一个 Agent 可以给自己发一件 SKILL，而那正是准入清单存在的理由。
- **一行式条目取作者写的第一行**，不生成摘要：摘要的摘要是消化产物，而消化产物默认可疑。
- **清单上没有的名字不进 catalog、也不报错**，只留一行诊断给写清单的人——承诺一件取不到的技能比它不在还糟。
- **书架按名字建键，section 降为字段**：阅览室按名字准入，所以「同一件」必须也按名字判定。键是类型化的 `ShelfKey`，由 `ShelfKey::of` 一处造出，上架与查清单两端都经它——按 `(section, name)` 建键时同名跨 section 的两份同时在架，「近架盖远架」因而在跨 section 时不成立。楼架后插，于是楼自己的那一份替换城里的那一份，哪怕两者归档在不同 section。
- **身份是名字，hash 不是身份**：`Library` 按名建键、`reading_room` 按名准入，于是每一件都由名字唯一说出；hash 答的是另一个问题——这份文档变了没有、哪一次 run 读的是哪份字节（§8-13）。把两问混为一问，会把「同名改了内容」读成换了一件，把「同一份挂在两处」报成两件。要按 hash 标「亦见于」，先得让 `Holding` 留下被近架盖住的那一份（hash 本身已在 `Holding` 上，缺的是被盖住者的落点与身份），再给答案加一列读它；今天没有这一列，所以「按 hash 判定身份」只是一个没有读者的说法。
- **读架上的失败逐条上报**：目录项读不动、目录名或文件名不是 Unicode，都带路径报 `E_STORAGE_FATAL`。一件静默缺席于每一间阅览室的 skill，是人从 catalog 上看不出来的那一种故障。
- **一格书架加一个落点是一个值，不是两个字段**（`Holding::shelf`）：一个持有只在一格书架上、只在一个落点上，两个字段允许「说 library、指向城外的文件」这个任何书架都进不了的状态。三条臂正好是 skill 能在的三个地方，没有第四条；哪一条由**扫盘时读的那个根**给出，不从地址反推——反推只对「两格书架碰巧落在不同地方」成立，而那是个巧合而不是规则。
- **`Shelf::address()` 答的是 catalog 承诺一件 skill 之前要问的那个问题**：城内书架上的一件靠地址打开，城外书架上的文件没有地址。所以**不发明一个假地址**：拼一个看起来像 `Address` 的字符串，会让读者去开一个并不在那儿的文件，且失败发生在第一次 `read` 而不在写下列表的那个时候。
- **城外书架上的一件带着它的正文**（`Holding::carried`）：扫描为了取一行披露与哈希本来就读了每个字节，留下这份正文，阅览室就能把它整份交给 catalog（`crates/runtime/Spec.lean` §8-29-6），run 按名读到的字节与 `hash` 出自同一次读入。只在 `Holding::of` 一处派生：`shelf.address()` 为 `None` 时为 `Some(正文)`，否则为 `None`——有地址的一件由 `read` 到地址去开，再带一份正文就是同一份文档的第二个家（D9）。
- **外部书架只读挂载，路径由城自己的配置给出**（`[skills] shelves`，城层一份）：`city_shelves` 在城的那一级上读它，而不是走三层梯子——书架是这座城所在的文件系统上的一个目录，它对每一栋楼同时挂上，让楼或房间能自己挂一本就是让一个作用域准入一份没人选过的文件。楼或房间写下它即在读文件处拒（`E_CONFIG_INVALID`，恢复语说把它移进城自己的 `CONFIG.toml`），而不是解析后丢掉——一份被接受却什么都不发生的配置，写它的人无从诊断。
- **外部书架的布局属于写它的那个 harness**：一层目录一件 skill、目录里放 `SKILL.md`（`SKILL_FILE` 是本城写下这条布局的**唯一一处**）；目录名就是 skill 名。**section 为空**：那棵树没有 section 这一级，替它编一个就是本城对一份它不拥有的东西的猜测。不在那里、不是目录的外部路径直接跳过（空架就是空架）；是文件而不是目录则以 `E_CONFIG_INVALID` 拒并报出是哪一条——一句「配置写了却什么都没发生」是没人能诊断的状态。
- **外部路径里的 `~` 指这个人自己的 home，home 以参数传入**：读环境不是本 crate 的活（`bin::assembly` 给出 `Home`），因此测试扫的是测试自己造的目录。committed 的城配置里不放一台机器的绝对路径，这正是 `~` 存在的理由。不是 `~` 开头也不是绝对路径的条目在解释处即拒，恢复语说出这条规则。
- **同一名的优先级是 building > library > external（外部按数组序）**：按最远的架先上、近的盖上去，于是城自己的存货盖过别的程序的目录——城留下一个名字时，那个名字指城的 skill；楼的自己一份又盖过城的。外部条目排到最后，因为它是别人写的：一份目录不是一个权威。
- **目录的架次由 `Shelf` 的臂序给出**：`Library::all` 先按 `Shelf::catalog_position`——城库、楼架、外部架按 `[skills] shelves` 的数组序——再按 `(section, name)`。位置是对三条臂的穷尽 match，不另立一张次序表；加一条臂而不在这里安置它，本 crate 编译不过。外部持有者没有 section，若把全部持有者按 `(section, name)` 排，别人的目录会排到城自己的存货之前，那正是上一条优先级的反面。
- **一件藏品可以是一个目录**：section 书架上的一项是 `<name>.md` 一份文档，或 `<name>/` 一个包——包里的 `SKILL.md`（`SKILL_FILE`，与外部书架同一布局）就是这件持有：catalog 只列它的第一行，`Holding::shelf` 指向它，`hash` 是它的字节，`Holding::package` 是包目录的地址——一件藏品是不是包由扫描在这里说出，读包的一方不从地址的写法去猜：一份恰好叫 `SKILL.md` 的单文档会让整个 section 被当成包，把阅览室没准入的藏品一并交出去。单文档与城外书架上的持有 `package` 为 `None`（后者没有地址）。包里其余文件不是持有，留在架上由 `read` 按 `<名>/<相对路径>` 打开（`crates/runtime/Spec.lean` §8-29-3）；只列一行是常驻上下文不随包膨胀的原因。没有 `SKILL.md` 的目录不是藏品，跳过而不报——与外部书架同形。
- **`Holding` 不携 `path`**：落点由 `Shelf` 说出（城内的两个臂就是地址），而多一个 `PathBuf` 就是同一件事的第二个家，且两层书架下必有一个是错的（§8-12 对 `holding_address` 的同一条理由）。
-/

/-!
### 8-12 一个 skill 只有一个家：楼自己的书架

```rust
impl Library {
    pub fn scan(city_root: &Path, building: Option<&Address>, home: &Path) -> Result<Library, AxError>;
}
```

用户要的是「一栋楼就是一个可以 cd 过去的工作区」；已记录的理由是「住户只读得到存货，不得给自己进货」。两者其实不冲突：楼自己的书架放在 `<building>/.sprawling/skills/<section>/<name>.md`，在楼目录里、在写域之外。

- **近的书架盖远的**：同名同 section 时楼的那本胜出。这不是新规则，而是 `config_layers` 已有的那一条（低层胜，高层是回落）在书架上的同一个实例。
- **城的书架只放两栋以上共用的**：一个 skill 只有一个家。代价是找一本 skill 要看两处，换到的是一栋楼拷走就带着它自己的本事。
- **落点只在 `Holding::shelf`**：扫盘时算一次（§8-8）。从 section＋name 拼回一个城级路径就是第二个权威，且在两层书架下其中一个必然答错。
-/

/-!
### 8-12b 一本书带着它被读到时的样子
```rust
pub struct Holding { …, pub hash: B3Hash }   // 整份文档的 BLAKE3，扫架时算
```

- **它是白得的**：`shelve` 为了取 disclosure 那一行，本来就把整份文档读进了内存；哈希只多走一遍已在手里的字节。
- **为什么存在 `Holding` 而不是让读者自己算**：读者要的答案是「它变了没有」，而那需要**两个时刻各一次读取**；一张只能报当下内容的书架永远答不了这个问题。早一次的那一读由 `run_started` 携走存进账本（`crates/runtime/Spec.lean` §8-11），于是比对对的是**这座城自己的历史**，不是一份签名：它只能说「这变了」，永远不说「这安全」。
- **名字不变而字节变了，正是注入的样子**，而只按名字核对的读者发现不了它。
-/

/-! D9 城外书架上的持有带着扫描读到的正文

**决定**：`Holding` 多一个字段 `carried: Option<String>`，城外书架上的一件是 `Some(SKILL.md 正文)`，城内书架上的是 `None`；只由 `Holding::of` 按 `Shelf::address()` 派生。

**理由**：阅览室要把城外的一件交给 run，而 run 打不开城外的路径；正文是唯一能交的东西（runtime D9）。扫描已经读了这些字节去取披露行与哈希，留下它们就保证交出去的字节与 `hash` 是同一次读入——在阅览室再读一次文件，两次读之间文件可能被它的主人改掉，pin 说的就不再是 run 读到的那份。

**被否**：①把正文放进 `Shelf::External` 臂——`Shelf` 答的是「在哪里」，而且 `accounting::views::skills` 按字段解构这一臂、把它映射到线上，正文会跟着进页面的那一侧；②阅览室准入时再读一次并比哈希——多一次读盘，比不上时还要另造一个拒词；③每件持有都带正文——城内的一件由 `read` 到地址去开，正文是第二个家，也让一千件的城库多占一千份内存。

**重开参数**：城外书架上的包里附属文件也要可读（runtime D9 的重开参数）时，`carried` 换成整包的字节或一份城内镜像的地址。
-/

/-!
## 模型：一个名字一件持有，近的书架盖远的

`Library::scan` 由远及近地把每一格书架上的持有按名字插进同一张表：城外书架按 `[skills] shelves` 的数组序，再是城库，最后是楼自己的书架；后插的同名持有替换先插的。模型里一格书架是「名字 → 这一格上的持有」，一张表是同样的函数。

* 一个名字留下的持有，就是从最近一格往远处找第一格有它的那一件（`the_nearest_shelf_keeps_the_name`）。
* 所以楼架上有的名字，阅览室拿到的就是楼架那一件，不论城库与城外书架上有没有同名的（`the_building_shelf_beats_the_city_and_the_outside`）。
-/

namespace City.Library

/-- 一格书架，或一张已经扫好的表：名字到持有。 -/
abbrev Shelf (Name Holding : Type) := Name → Option Holding

/-- 把一格书架插进表里：这一格上有的名字替换表里已有的。 -/
def shelve {Name Holding : Type} (held shelf : Shelf Name Holding) : Shelf Name Holding :=
  fun name => match shelf name with
    | some holding => some holding
    | none => held name

/-- `Library::scan`：由远及近，一格一格插进一张空表。 -/
def scan {Name Holding : Type} (shelves : List (Shelf Name Holding)) : Shelf Name Holding :=
  shelves.foldl shelve fun _ => none

theorem shelving_onto_a_table_keeps_the_nearest {Name Holding : Type}
    (held : Shelf Name Holding) (shelves : List (Shelf Name Holding)) (name : Name) :
    shelves.foldl shelve held name = (shelves.reverse.findSome? fun shelf => shelf name).or (held name) := by
  induction shelves generalizing held with
  | nil => simp
  | cons shelf rest nearer =>
    simp only [List.foldl_cons, List.reverse_cons, List.findSome?_append, nearer, shelve,
      List.findSome?_singleton]
    cases rest.reverse.findSome? (fun shelf => shelf name) <;> cases shelf name <;> rfl

theorem the_nearest_shelf_keeps_the_name {Name Holding : Type} (shelves : List (Shelf Name Holding))
    (name : Name) : scan shelves name = shelves.reverse.findSome? fun shelf => shelf name := by
  simp [scan, shelving_onto_a_table_keeps_the_nearest]

theorem the_building_shelf_beats_the_city_and_the_outside {Name Holding : Type}
    (outside : List (Shelf Name Holding)) (library building : Shelf Name Holding) (name : Name)
    (holding : Holding) (shelved : building name = some holding) :
    scan (outside ++ [library, building]) name = some holding := by
  simp [the_nearest_shelf_keeps_the_name, shelved]

end City.Library
