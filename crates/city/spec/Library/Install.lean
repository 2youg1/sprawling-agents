-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::library::install

规定 `library::install`、`library::install::precheck`、`library::install::precheck::walk`（`crates/city/src/` 下同名的文件）。一件技能进书架的唯一入口：静态预检、原子落位、内容哈希入 CAS。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-28 技能安装：静态预检、原子落位、内容哈希入 CAS（`library::install`，形状 2 值＋一个落盘动作）

技能包与居民自建技能进书架前的唯一入口。四步都在这一扇门里：静态预检（不执行代码、禁符号链接、名称冲突校验）、staging 原子换入、落位前 TOCTOU 复查、内容哈希登记。

```rust
// city::library::install
pub struct Slot { /* shelf、section —— 私有 */ }
impl Slot {
    pub fn library(section: &str) -> Result<Slot, AxError>;
    pub fn building(building: &Address, section: &str) -> Result<Slot, AxError>;
}
pub enum Placed { Fresh, AlreadyShelved }                  // 穷尽两态
pub struct Installed { pub holding: Holding, pub hash: B3Hash, pub placed: Placed }
pub struct PlannedInstall { /* 名字、每一项的字节快照、哈希、依据、落点 —— 私有 */ }
impl PlannedInstall {
    pub fn name(&self) -> &str;
    pub fn hash(&self) -> &B3Hash;
    pub fn apply(self, register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>)
        -> Result<Installed, AxError>;
}
pub fn plan_install(city_root: &Path, slot: &Slot, package: &Path)
    -> Result<PlannedInstall, AxError>;
pub fn install(city_root: &Path, slot: &Slot, package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Installed, AxError>;
```

- **来源两个形状，落点随来源**：技能包是 `<name>/` 一个目录一件，内含 `SKILL.md`（与外部书架同一布局，`SKILL_FILE` 仍是它的唯一家），整包落成 `<shelf>/<section>/<name>/`：包里的每个目录与文件都照原相对路径落下，一项不丢——收下却不落的文件就是被静默丢掉的文件；居民自建技能是一份 `.md`，落成 `<shelf>/<section>/<name>.md`。两个落点都是 §8-8 扫描读的形状，`Installed::holding` 即扫描读回的那件持有（由同一个函数从盘上读出，不是第二个推导）。
- **整包哈希的字节序（规范串）**：头 `sprawling-skill-package/1
`，其后每一项按相对路径（段间用 `/`）的字节序排列，一项是：种类一字节（`d` 目录、`f` 文件）、路径长度（u64 小端）、路径字节；文件再跟内容长度（u64 小端）、内容字节。长度前缀让任何两个不同的包拼不出同一串；带头是为了一份恰好长得像规范串的文档在 CAS 里不会被读成一个包。包的哈希是这串字节的 BLAKE3，文档的哈希是正文的 BLAKE3——`PlannedInstall::hash` 与 `Installed::hash` 都是它，也是人批准的那一个。包的 `Installed::holding.hash` 仍是扫描给的 `SKILL.md` 的哈希（§8-8），两者答的是两个问题：前者是「装下的是哪一整份」，后者是「catalog 那一行变了没有」；扫描不为每次读 catalog 去读包里每个文件。
- **包的大小有上限**：一件包的文件字节合计不超过 `PACKAGE_BYTES_LIMIT`（32 MiB，`precheck::walk`）。预检把整包字节握在内存里、plan 与 apply 各读一遍，CAS 把整包存成一个 blob；一件技能是文本和几份小脚本，上限比它的本分宽得多，又远低于一台机器能握两份的量。判法按句柄报出的长度在读之前判，读时再以剩余额度加一截断，读的时候变大的文件同样被拒；超限＝`E_INVALID_ARGS`，拒词点出越线的那一项。
- **不执行代码**：本模块只读字节、判形状——包里的任何内容都不被执行、编译或解释。预检是静态的，这是它全部的含义。
- **禁符号链接，恒拒**：来源本身与书架上已有的那一件经 `symlink_metadata` 判形，包里每一层的每一项由下面的句柄逐级走判形，出现链接即拒，**不论它指向哪里、藏在第几层**——经链接读到的字节不属于这个包，且落位前后可以指向不同的东西。这条规则只在 `precheck` 里。**包按打开的目录句柄逐级走**（`cap-std`／`cap-fs-ext`）：包根由它的父目录句柄不跟随链接地打开，此后每一项只按名字、相对于列出它的那个目录句柄打开——子目录经 `open_dir_nofollow`，文件经不跟随链接的只读打开——打开后再由句柄自身的元数据判形，是链接、不是普通目录或文件、或（Windows 上）带任何重解析点（云端占位、去重文件不跟随打开时给出的是存根的字节）即拒。列举只贡献名字，列表里每一项的种类先判一次（是链接即拒）；读到的每个字节都经过一条从包根起、不含链接的句柄链，于是一个子目录在被判形之后换成链接，走读也到不了它指向的地方——换上的链接要么打不开，要么打开的是链接本身而被拒。Windows 上列举由句柄反查出的路径去列（`cap-std` 在 Windows 上的实现），一次被调包骗过的列举只会给出错的名字；这些名字仍相对真句柄打开，得到的是 NotFound 或包里自己的项，包外的字节进不来。居民自建技能的单份文档没有目录可走：`unlinked` 判路径本身的形状（来源与架上已有的同名持有都经它），`read_unlinked` 在打开的那一刻再判一次——Windows 不跟随链接地打开并拒一切重解析点，macOS 与 Linux 比对打开前后的 inode。拒词指出是哪一项，并说出「重打包，只用普通文件与目录」。
- **名称冲突校验按「一个名字一个持有」判**：目标书架上 `<name>.md` 或 `<name>/` 任一在、而 section 不同即拒——§8-8 的扫描按名字建键，两格同名会按遍历序静默互盖。同格同名的已有持有按与来源同一个预检读出哈希（文档读正文、包算整包规范串）：同哈希是幂等（`Placed::AlreadyShelved`，一个字节不写）；异哈希即拒（与「二次出生恒拒」同形，覆写会把一件在用的技能悄悄换掉）——形状不同的同名持有哈希必不同，同样拒。同一格里 `<name>.md` 与 `<name>/` 并存也拒：那是一个名字两件持有，扫描会按遍历序留下其中一件，拿任何一件的哈希答「已在架上」都是只看了一半；拒词说出两件，恢复语让人先拿下其一。name 与 section 都必须是能落盘的单段名：非空、不以点开头、无分隔符——扫描会跳过空名与点开头的项，收下这样的名字等于装进一个 catalog 永远看不见的格子。
- **TOCTOU 复查在落位之前，落下的是快照**：`plan_install` 把来源的每一项读进内存（每项一份字节快照）并算出哈希，同时记下目标格当时有没有东西、哈希是什么；`apply` 落位前把来源整包重读一遍、重算哈希比对（哈希覆盖每一项的路径、种类、长度与内容，于是任何一项被改、增、删、换形都算「被换」），**不一致即整体拒收**（`E_VERSION_CONFLICT`）——盘上一个字节不动，`register` 不被调用。落位写的是快照里的字节而不是再去读来源，于是复查之后的一次调包也进不了书架：装上架的字节恒是来源某个完整状态的忠实映像，而落架不会盖掉一个缝里刚出现的同名持有。
- **staging 原子换入只在 `city::document` 里**：文档经 `document::replace`（暂存文件＋`rename`）；包经 `document::place_tree`——在同一 section 里一个点开头的暂存目录中写齐每一项（每个文件在暂存目录里经 `stage` 写入并 `sync_all`，不再各自暂存改名；扫描跳过点开头的项），再一次 `rename` 把整个目录换进 `<name>/`。目录换入的目标此刻不存在（存在就是 `AlreadyShelved` 或拒），所以读者看到的要么没有这件包、要么整包，没有第三种。上次崩溃留下的同名暂存目录先删掉再写：它从未被换入，没有读者见过它。
- **内容哈希入 CAS，登记先于换入**：`register` 由装配层供给（本 crate 依赖只有 `kernel`，CAS 归 `storage`；与 `Neighbourhood::scan` 的 `waiting` 同一口径），绑定的是 `storage::Cas::put`。登记的是哈希所算的那串字节——文档登记正文，包登记整包规范串（一个 blob，键就是人批准的哈希，历史能从它还原整包；逐项分别登记会留下一堆没有清单的 blob，整包哈希在 CAS 里指不到任何东西）。登记哈希与 `Installed::hash` 不符即拒（`E_CAS_CORRUPT`）：书架与 CAS 说的是同一份字节才叫有据可查。**来源记名即内容哈希**：盘上那份是现场，CAS 那份是历史（与 JOB.md 同一口径，§8-13 的 hash 答的正是「它变了没有」）。
- **失败码**：来源不在＝`E_PATH_NOT_FOUND`；来源不是包（缺 `SKILL.md`、既不是目录也不是 `.md` 文件）、包里有链接、包超过大小上限、名字或 section 不可用、名称冲突（含同格 `<name>.md` 与 `<name>/` 并存）＝`E_INVALID_ARGS`（与 `building::create` 的「名字已被占」同码同形）；依据被换＝`E_VERSION_CONFLICT`；登记哈希不符＝`E_CAS_CORRUPT`；落盘失败沿 `document::replace` 的 `E_STORAGE_FATAL`。每条拒因都带动作、主体与可执行的恢复语。
- **生产调用者**：`library::shipped::shelve_shipped`（§8-28c）经这扇门把自带的 skill 放上城库，它由新城成形的那一处（`accounting::worker::genesis::form`）调用；`InstallSkill` 的执行者（D20）是第二个调用者，与 wire 的那一臂同一次改动落地。
- **外部书架没有臂**：`Slot` 只有两个构造点，城库与楼架。外部书架是别人目录的只读挂载（§8-8），往那里落东西在类型上就拼不出来。

**本节测试**：`library::install::tests`——装上架的字节等于来源字节且扫描读回的 `Holding` 与 `Installed::holding` 整体相等；带子目录与多个文件的包整包落成 `<name>/`、逐项字节相等、登记的一个 blob 的哈希等于 `Installed::hash`；同哈希重装幂等（`AlreadyShelved`，盘上字节不变）；**plan 与 apply 之间来源被换即整体拒收**（`SKILL.md` 被改、多出一项、深层一项被改；拒后盘上无文件、登记零调用）；经符号链接的包恒拒，深层一项是链接也拒且拒词点出那一项；异哈希占名与跨 section 同名各拒一次；同格 `<name>.md` 与 `<name>/` 并存即拒；超过大小上限的包即拒（按句柄报出的长度，不读字节）。`document::tests` 另有一例：目标已在时 `place_tree` 拒收，目标旁不留暂存目录、目标原样。CAS 绑定的端到端一例在 `crates/sprawling/tests/skill_install.rs`（`Cas::put` 兑付 `register`，装上架的内容可按 `Installed::hash` 从 CAS 取回同一份字节）。
-/

/-! D20 User 加 skill 只有一扇门：`InstallSkill` 的执行者把三种来源都变成一个本地目录，再交给 `library::install`；自带的 skill 编进二进制，经同一扇门放上城库

**决定**：
- **来源四臂，落点两臂**（wire D32 拼写）：`SkillSource::Path { path }`、`Git { url, rev, subdir }`、`SkillsSh { name }`、`Shipped`；`ShelfChoice::Library { section }` 与 `Building { building, section }` 一对一映到 `Slot::library` 与 `Slot::building`。城外书架没有臂，与 §8-28 同理。
- **`Path`**：User 写下的路径照原样读；必须是绝对路径（Windows 上是盘符或 UNC 开头，macOS 与 Linux 上以 `/` 开头）或以 `~` 开头（`~` 指 `bin::assembly` 给出的 `Home`，与 §8-8 外部书架同一条规则）；相对路径即拒 `E_INVALID_ARGS`，恢复语说「给出绝对路径」——城的工作目录不是 User 在页面上看到的任何一个目录。随后直接 `install(city_root, slot, path, register)`。
- **`Git`**：只收 `https://` 地址（`E_INVALID_ARGS` 拒其余：ssh 会读 User 的 ssh 配置并可能等口令，`file://` 与 `Path` 重复）。执行者在 `<city>/.sprawling/` 之下一个按 `idem` 命名、点开头的暂存目录里跑 `git clone --depth 1 [--branch <rev>] <url> <暂存目录>`，环境带 `GIT_TERMINAL_PROMPT=0`，超时 `CLONE_TIMEOUT`（120 s）；git 由 doctor 在 PATH 上找（Windows 上是 `git.exe`），不在即拒 `E_TOOL_UNAVAILABLE`，恢复语说出 doctor 给的安装行；clone 失败拒 `E_TOOL_UNAVAILABLE`，主体带 git 标准错误的最后一行，恢复语「在浏览器里打开这个地址确认它存在，再试一次」；超时拒 `E_TIMEOUT`。`subdir` 缺席时包根就是仓库根。之后 `plan_install` 读暂存目录里那一层，`apply` 落位；暂存目录无论成败都删掉，上次崩溃留下的同名暂存目录先删再 clone（与 `place_tree` 的暂存同一口径）。`.git/` 不是包的一部分：clone 之后、预检之前删掉它，因为它不是 skill 的内容，而它的对象文件会让整包超过 `PACKAGE_BYTES_LIMIT`。
- **`SkillsSh`**：名字是 `<owner>/<repo>/<skill>` 三段（与 skills.sh 页面上的写法相同），换成 `Git { url: https://github.com/<owner>/<repo>, rev: None, subdir }`，`subdir` 是 clone 里**唯一一个**名为 `<skill>`、内含 `SKILL.md` 的目录；零个或多于一个即拒 `E_INVALID_ARGS`，拒词列出找到的候选。审核随后按 D19 去问 skills.sh 的同一个名字。
- **`Shipped`：自带的 skill 编进二进制**。源树的 `skills/` 在构建时经 city 的构建脚本写进 `OUT_DIR` 并 `include_bytes!`（`xtask packaged` 准许的两处之一），所以四条安装渠道——发行归档、安装脚本（只复制二进制）、npm 与 `cargo binstall`（都只带二进制）、`cargo install`（crates.io 的包里只有包目录）——得到同一份自带 skill；为此源树根上的 `skills/` 搬进 city 的包目录、仍叫 `skills/`（与 city D18 把模板搬进包目录同一条理由），引用它的文档同一次改动改路径，归档的 Skills 部分（`tools/xtask/src/package/contents.rs`）改读新路径。执行者把每一件经 `install` 放进城库的 `shipped` 格（`SHIPPED_SECTION`），来源是一个从内嵌字节在暂存目录里写出的目录，于是预检、原子落位与 CAS 登记仍只有 §8-28 一处。同哈希的那一件答 `AlreadyShelved`，不写一个字节；架上同名而内容不同（User 改过）即按 §8-28 拒，那一件留在架上，拒词说出名字。
- **何时放自带的**：新城成形的那一刻放一次（`Shipped`，城库；成形只有 `accounting::worker::genesis::form` 一处，向导与 `sprawling init` 都经它，§8-28c）；已有的城不自动放——它的书架上没有某件自带 skill，可能正是 User 拿下了它——书架页给一个放自带 skill 的控件，发的是同一条 `InstallSkill { source: Shipped }`。放上书架不等于准入：阅览室仍只收 `RULES.toml` 的 `reading_room` 写下的名字（§8-8）。
- **答复**：命令按 `idem` 回执；成功之后页面重读书架（已有的书架查询），失败经命令的拒词回到页面。审核由 D19 在落位之后另起，不进这次答复。

**理由**：§8-28 已经是预检、原子落位与内容哈希的唯一入口，三种远处的来源在门外变成一个本地目录，门里一行不改，规则就只有一份。自带 skill 编进二进制，是因为只有发行归档在二进制旁边带 `skills/`，安装脚本只复制二进制，其余渠道也只有二进制；按「二进制旁边」去找，在四条渠道里有三条找到空。新城自动放、旧城不放，是因为旧城的空书架是 User 的状态，不是缺失。

**被否**：①按 `current_exe()` 旁边的 `skills/` 去找：只在解开的归档里成立；②客户端拼一个 `PutShelved` 把文件逐个发来：`PutShelved` 只携一份文本，包里的目录与脚本进不来，而且预检会在页面与城里各有一份；③每次开城把缺的自带 skill 补回去：User 拿下的那件会在下次开城时回来；④用库克隆（`gix`）代替 git 程序：多一棵依赖树，换来的只是不需要 PATH 上的 git，而 doctor 已经为工作树找 git。

**重开参数**：自带 skill 的总字节超过 1 MiB，或 User 要求不带自带 skill 的二进制时，重议内嵌；`CLONE_TIMEOUT` 与 `SHIPPED_SECTION` 是推断值，各是一个常量。

**三个平台**：内嵌与安装渠道无关，三个平台上自带 skill 都在二进制里；城库在三个平台上都是 `<city>/.sprawling/library/shipped/`；git 在 Windows 上找 `git.exe`，在 macOS 与 Linux 上找 `git`，都按 PATH。
-/

/-!
### 8-28c 自带 skill 上架（`library::shipped`，形状 2 值＋一个落盘动作）

```rust
// city::library::shipped
pub const SHIPPED_SECTION: &str = "shipped";
pub fn shelve_shipped(city_root: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Vec<Installed>, AxError>;
```

- **字节从哪来**：city 的构建脚本（`crates/city/build.rs`）走包目录里的 `skills/`，为其中每一个含 `SKILL.md` 的子目录的每个文件生成一行 `(相对路径, include_bytes!(…))`，写进 `OUT_DIR` 下的一张表，`library::shipped` 经 `include!` 读它。`skills/` 根上的文件（`README.md`、`LICENSES.md`）不是 skill，不进表；它们随发行归档走（D20）。表按相对路径排序，同一棵树在三个平台上生成同一张表；路径段在表里以 `/` 分隔，落盘时逐段 `join`，于是 Windows 上不出现 `/` 与 `\` 混写。
- **只经 §8-28 一扇门**：`shelve_shipped` 在 `<library>/.shipped/` 这个点开头的暂存目录里按表写出每件 skill 的目录（扫描跳过点开头的项，读者看不见它），再对每件调用 `install(city_root, Slot::library(SHIPPED_SECTION), <暂存>/<name>, register)`；预检、原子落位与 CAS 登记都不在这里另写。暂存目录先删再写，成败之后都删：它从未被换入。
- **字节照原样**：表里是源树里的字节，写出与登记都不改行尾——仓库的 `.gitattributes` 让它们在三个平台的检出里都是 LF，书架上的那一份就是二进制里的那一份。
- **一件自带 skill 的「版本」就是它的内容哈希**：没有另记的版本号。§8-28 的整包哈希回答「装下的是哪一份」；新的二进制带来不同的字节时，已有的城不被改动（D20），新城得到新的那份。
- **何时调用**：只在新城成形时一次（`form`，它对已有历史的目录恒拒，所以对一座城至多一次）。开城不调用：书架上缺一件自带 skill，可能是 User 拿下了它（D20 被否的③）。
- **失败**：任一件被 §8-28 拒（例如目录里事先已有同名而内容不同的一件）即返回那条拒词，`form` 在写下账本第零行之前放 skill，于是被拒的成形不留下历史，修好之后再成形时已落下的那几件答 `AlreadyShelved`；暂存目录写不了是 `E_STORAGE_FATAL`。
- **三个平台**：表与字节与平台无关；城库在三个平台上都是 `<city>/.sprawling/library/shipped/<name>/`；路径一律经 `Path::join` 拼。

下面的模型是一座城在一条事件轨迹上的书架：成形、开城、User 拿下一件、User 改一件。证明两条，各在一条轨迹上量化：自带的每一件至多被放一次（`a_builtin_is_placed_at_most_once`）；成形之后被 User 拿下的一件，此后无论再发生什么都不回到书架上（`a_removed_builtin_is_not_put_back`）。

**本节测试**：`accounting::worker::genesis::tests` 里一例：成形一座新城，扫描城库，`shipped` 格里的名字恰是包目录 `skills/` 下每件 skill 的名字，且第二次成形被拒后书架不变。
-/

namespace City.Library.Shipped

/-- 一件自带 skill 的名字。模型只需要它能判等。 -/
abbrev Name := Nat

/-- 一座城的轨迹上的一步。 -/
inductive Event where
  /-- `form`：已有历史即拒（什么都不变），否则成形并放上自带的每一件。 -/
  | form
  /-- 开一座已有的城：不放任何东西。 -/
  | openCity
  /-- User 从书架上拿下名为 `n` 的那件。 -/
  | remove (n : Name)
  /-- User 改了名为 `n` 的那件的内容：书架上的名字不变。 -/
  | edit (n : Name)
  deriving DecidableEq, Repr

structure City where
  born : Bool
  shelf : List Name
  /-- 经 `shelve_shipped` 放上书架的每一次，按发生的顺序。 -/
  placed : List Name
  deriving DecidableEq, Repr

def unformed : City := ⟨false, [], []⟩

def step (builtins : List Name) (c : City) : Event → City
  | .form =>
    if c.born then c
    else { born := true, shelf := c.shelf ++ builtins, placed := c.placed ++ builtins }
  | .openCity => c
  | .remove n => { c with shelf := c.shelf.filter (· != n) }
  | .edit _ => c

def run (builtins : List Name) (c : City) (trace : List Event) : City :=
  trace.foldl (step builtins) c

theorem run_cons (b : List Name) (c : City) (e : Event) (es : List Event) :
    run b c (e :: es) = run b (step b c e) es := rfl

theorem run_append (b : List Name) (c : City) (x y : List Event) :
    run b c (x ++ y) = run b (run b c x) y := by
  simp [run, List.foldl_append]

/-- 放过的要么一次都没有（还没成形），要么恰是自带的那一份（成形过一次）。 -/
def Placements (b : List Name) (c : City) : Prop :=
  (c.born = false ∧ c.placed = []) ∨ (c.born = true ∧ c.placed = b)

theorem step_keeps_placements (b : List Name) (c : City) (e : Event)
    (h : Placements b c) : Placements b (step b c e) := by
  cases e with
  | form =>
    rcases h with ⟨hb, hp⟩ | ⟨hb, hp⟩
    · simp [step, hb, hp, Placements]
    · simp [step, hb, hp, Placements]
  | openCity => exact h
  | remove n => simpa [step, Placements] using h
  | edit n => exact h

theorem run_keeps_placements (b : List Name) (c : City) (trace : List Event)
    (h : Placements b c) : Placements b (run b c trace) := by
  induction trace generalizing c with
  | nil => exact h
  | cons e es ih => rw [run_cons]; exact ih _ (step_keeps_placements b c e h)

/-- 自带的每一件至多被放一次。 -/
theorem a_builtin_is_placed_at_most_once (b : List Name) (hb : b.Nodup)
    (trace : List Event) (n : Name) : (run b unformed trace).placed.count n ≤ 1 := by
  rcases run_keeps_placements b unformed trace (Or.inl ⟨rfl, rfl⟩) with ⟨_, hp⟩ | ⟨_, hp⟩
  · simp [hp]
  · rw [hp, List.Nodup.count hb]; split <;> simp

theorem born_stays (b : List Name) (c : City) (trace : List Event) (h : c.born = true) :
    (run b c trace).born = true := by
  induction trace generalizing c with
  | nil => exact h
  | cons e es ih =>
    rw [run_cons]
    apply ih
    cases e <;> simp [step, h]

theorem form_bears (b : List Name) (c : City) (trace : List Event)
    (h : Event.form ∈ trace) : (run b c trace).born = true := by
  induction trace generalizing c with
  | nil => simp at h
  | cons e es ih =>
    rw [run_cons]
    rcases List.mem_cons.mp h with he | he
    · subst he
      apply born_stays
      by_cases hc : c.born = true <;> simp [step, hc]
    · exact ih _ he

/-- 成形之后不在书架上的一个名字。 -/
def KeptOff (n : Name) (c : City) : Prop := c.born = true ∧ n ∉ c.shelf

theorem step_keeps_off (b : List Name) (n : Name) (c : City) (e : Event)
    (h : KeptOff n c) : KeptOff n (step b c e) := by
  obtain ⟨hb, hn⟩ := h
  cases e with
  | form => simp [step, hb, KeptOff, hn]
  | openCity => exact ⟨hb, hn⟩
  | remove m =>
    refine ⟨hb, ?_⟩
    intro hm
    exact hn (List.mem_filter.mp hm).1
  | edit m => exact ⟨hb, hn⟩

theorem run_keeps_off (b : List Name) (n : Name) (c : City) (trace : List Event)
    (h : KeptOff n c) : KeptOff n (run b c trace) := by
  induction trace generalizing c with
  | nil => exact h
  | cons e es ih => rw [run_cons]; exact ih _ (step_keeps_off b n c e h)

/-- 成形之后被 User 拿下的一件，此后不回到书架上。 -/
theorem a_removed_builtin_is_not_put_back (b : List Name) (before after : List Event)
    (n : Name) (hf : Event.form ∈ before) :
    n ∉ (run b unformed (before ++ Event.remove n :: after)).shelf := by
  rw [run_append, run_cons]
  refine (run_keeps_off b n _ after ⟨?_, ?_⟩).2
  · exact form_bears b unformed before hf
  · simp [step]

/-- 正常路径：一座新城成形之后，自带的每一件都在书架上。 -/
theorem a_formed_city_shelves_every_builtin (b : List Name) :
    (run b unformed [Event.form]).shelf = b := by
  simp [run, step, unformed]

end City.Library.Shipped
