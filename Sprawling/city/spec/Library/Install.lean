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
pub struct PlannedInstall { /* 名字、每一项的字节快照、manifest、来源哈希、书架依据 —— 私有 */ }
impl PlannedInstall {
    pub fn name(&self) -> &str;
    pub fn hash(&self) -> &B3Hash;                 // canonical manifest 的 digest
    pub fn apply(self, register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>)
        -> Result<Installed, AxError>;             // 写 CAS，再更新 INDEX.toml
    pub fn admit(self, register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>)
        -> Result<Installed, AxError>;             // 外部 skill：只写 CAS，不写书架索引
}
pub fn plan_install(city_root: &Path, slot: &Slot, package: &Path)
    -> Result<PlannedInstall, AxError>;
pub fn install(city_root: &Path, slot: &Slot, package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Installed, AxError>;
```

- **来源两个形状，存储只有一种**：技能包是 `<name>/` 一个目录一件，内含 `SKILL.md`；居民自建技能是一份 `.md`。两者都形成 `SkillManifest`：单文档只有 `SKILL.md` 一项，包列出每项相对路径、种类与文件 digest。每个文件分别写入 CAS，再写 canonical manifest；`INDEX.toml` 只增加或保留 `(section, name, manifest digest)` 一行，不在书架复制文件。`Installed::holding` 由同一份 manifest 与 `SKILL.md` 构造。
- **包 digest 是 canonical manifest 的 digest**：`kernel::SkillManifest` 按相对路径排序，以 JSON 记录 `version` 与项目；文件项带该文件字节的 BLAKE3，目录项不带。源快照另算旧规范串的哈希，只用于 plan/apply 之间检测来源变化，不作为包身份。`PlannedInstall::hash`、`Installed::hash`、`skill_shelved` 与 `skill_audited` 的新绑定都是 manifest digest；披露仍取 `SKILL.md` 第一行。
- **包的大小有上限**：一件包的文件字节合计不超过 `PACKAGE_BYTES_LIMIT`（32 MiB，`precheck::walk`）。预检把整包字节握在内存里、plan 与 apply 各读一遍，CAS 按文件去重，相同文件跨 skill 与版本只存一次；一件技能是文本和几份小脚本，上限比它的本分宽得多。判法按句柄报出的长度在读之前判，读时再以剩余额度加一截断，读的时候变大的文件同样被拒；超限＝`E_INVALID_ARGS`，拒词点出越线的那一项。
- **不执行代码**：本模块只读字节、判形状——包里的任何内容都不被执行、编译或解释。预检是静态的，这是它全部的含义。
- **禁符号链接，恒拒**：来源本身与书架上已有的那一件经 `symlink_metadata` 判形，包里每一层的每一项由下面的句柄逐级走判形，出现链接即拒，**不论它指向哪里、藏在第几层**——经链接读到的字节不属于这个包，且落位前后可以指向不同的东西。这条规则只在 `precheck` 里。**包按打开的目录句柄逐级走**（`cap-std`／`cap-fs-ext`）：包根由它的父目录句柄不跟随链接地打开，此后每一项只按名字、相对于列出它的那个目录句柄打开——子目录经 `open_dir_nofollow`，文件经不跟随链接的只读打开——打开后再由句柄自身的元数据判形，是链接、不是普通目录或文件、或（Windows 上）带任何重解析点（云端占位、去重文件不跟随打开时给出的是存根的字节）即拒。列举只贡献名字，列表里每一项的种类先判一次（是链接即拒）；读到的每个字节都经过一条从包根起、不含链接的句柄链，于是一个子目录在被判形之后换成链接，走读也到不了它指向的地方——换上的链接要么打不开，要么打开的是链接本身而被拒。Windows 上列举由句柄反查出的路径去列（`cap-std` 在 Windows 上的实现），一次被调包骗过的列举只会给出错的名字；这些名字仍相对真句柄打开，得到的是 NotFound 或包里自己的项，包外的字节进不来。居民自建技能的单份文档没有目录可走：`unlinked` 判路径本身的形状（来源与架上已有的同名持有都经它），`read_unlinked` 在打开的那一刻再判一次——Windows 不跟随链接地打开并拒一切重解析点，macOS 与 Linux 比对打开前后的 inode。拒词指出是哪一项，并说出「重打包，只用普通文件与目录」。
- **名称冲突校验按「一个名字一个候选持有」判**：索引里同一名字位于不同 section 即拒——§8-8 的扫描按名字建键，两格同名会按遍历序静默互盖。同格同名的当前索引行 digest 相同即幂等（`Placed::AlreadyShelved`，一个字节不写）；digest 不同则是同一名字的新候选版本，原子替换索引行，不改动任何楼已经准入的 pin。旧目录与新索引行并存，或同一格里旧 `<name>.md` 与 `<name>/` 并存，仍拒：那是迁移前同一个名字的两件持有，扫描无法唯一解析。name 与 section 都必须是能落盘的单段名：非空、不以点开头、无分隔符——扫描会跳过空名与点开头的项，收下这样的名字等于装进一个 catalog 永远看不见的格子。
- **TOCTOU 复查在落位之前，落下的是快照**：`plan_install` 把来源的每一项读进内存（每项一份字节快照）并算出哈希，同时记下目标格当时有没有东西、哈希是什么；`apply` 落位前把来源整包重读一遍、重算哈希比对（哈希覆盖每一项的路径、种类、长度与内容，于是任何一项被改、增、删、换形都算「被换」），**不一致即整体拒收**（`E_VERSION_CONFLICT`）——盘上一个字节不动，`register` 不被调用。落位写的是快照里的字节而不是再去读来源，于是复查之后的一次调包也进不了书架：装上架的字节恒是来源某个完整状态的忠实映像，而落架不会盖掉一个缝里刚出现的同名持有。
- **索引更新是原子替换**：CAS 写入完成后才经 `document::replace` 更新 `INDEX.toml`。索引先落，旧书架目录后删；失败不会留下一个宣称已迁移却仍指向旧文件的索引。CAS 中多余的不可达对象只是历史，不是书架权威。
- **每个文件与 manifest 都经 `register` 写入 CAS**：`register` 由装配层供给（本 crate 依赖只有 `kernel`，CAS 归 `storage`），绑定 `storage::Cas::put`；损坏恢复路径绑定 `put_repair`。每次返回的 digest 必须与预期逐项相符，否则以 `E_CAS_CORRUPT` 整体拒绝，不更新索引。CAS 是字节的唯一权威，书架只是索引；`read` 与 `describe` 必须按 admitted digest 做完整校验读取。
- **失败码**：来源不在＝`E_PATH_NOT_FOUND`；来源不是包（缺 `SKILL.md`、既不是目录也不是 `.md` 文件）、包里有链接、包超过大小上限、名字或 section 不可用、名称冲突（含同格 `<name>.md` 与 `<name>/` 并存）＝`E_INVALID_ARGS`（与 `building::create` 的「名字已被占」同码同形）；依据被换＝`E_VERSION_CONFLICT`；登记哈希不符＝`E_CAS_CORRUPT`；落盘失败沿 `document::replace` 的 `E_STORAGE_FATAL`。每条拒因都带动作、主体与可执行的恢复语。
- **生产调用者**：`library::shipped::shelve_shipped`（§8-28c）经这扇门把自带的 skill 放上城库，它由新城成形的那一处（`accounting::worker::genesis::form`）调用；`InstallSkill` 的执行者（D20）是第二个调用者，阅览室的城外 skill 准入是第三个，只写 CAS 而不新增书架行。
- **外部书架没有臂**：`Slot` 只有两个构造点，城库与楼架。外部书架是别人目录的只读挂载（§8-8），往那里落东西在类型上就拼不出来。

**本节测试**：`library::install::tests`——登记后的每个文件字节等于来源；manifest 逐项 digest 相符，`Installed::hash` 等于 manifest 的 BLAKE3；索引只有 `(section, name, digest)`，书架没有文件目录；扫描读回的 `Holding` 与 `Installed::holding` 整体相等；同哈希重装幂等（`AlreadyShelved`，索引不变），同名异 digest 成为新候选版本；**plan 与 apply 之间来源被换即整体拒收**（拒后索引不变、登记零调用）；链接、旧持有并存、跨 section 同名与超限各拒一次。旧书架迁移先校验旧 CAS 对象再替换为索引。CAS 绑定的端到端一例在 `crates/sprawling/tests/skill_install.rs`（`Cas::put` 兑付 `register`，可按 `Installed::hash` 取回 manifest，再逐文件取回同一份字节）。
-/

/-! D20 User 加 skill 只有一扇门：`InstallSkill` 的执行者把三种来源都变成一个本地目录，再交给 `library::install`；自带的 skill 编进二进制，经同一扇门放上城库

**决定**：
- **来源四臂，落点两臂**（wire D32 拼写）：`SkillSource::Path { path }`、`Git { url, rev, subdir }`、`SkillsSh { name }`、`Shipped`；`ShelfChoice::Library { section }` 与 `Building { building, section }` 一对一映到 `Slot::library` 与 `Slot::building`。城外书架没有臂，与 §8-28 同理。
- **`Path`**：User 写下的路径照原样读；必须是绝对路径（Windows 上是盘符或 UNC 开头，macOS 与 Linux 上以 `/` 开头）或以 `~` 开头（`~` 指 `bin::assembly` 给出的 `Home`，与 §8-8 外部书架同一条规则）；相对路径即拒 `E_INVALID_ARGS`，恢复语说「给出绝对路径」——城的工作目录不是 User 在页面上看到的任何一个目录。随后直接 `install(city_root, slot, path, register)`。
- **`Git`**：只收 `https://` 地址（`E_INVALID_ARGS` 拒其余：ssh 会读 User 的 ssh 配置并可能等口令，`file://` 与 `Path` 重复）。执行者在 `<city>/.sprawling/` 之下一个按 `idem` 命名、点开头的暂存目录里跑 `git clone --depth 1 [--branch <rev>] <url> <暂存目录>`，环境带 `GIT_TERMINAL_PROMPT=0`，超时 `CLONE_TIMEOUT`（120 s）；git 由 doctor 在 PATH 上找（Windows 上是 `git.exe`），不在即拒 `E_TOOL_UNAVAILABLE`，恢复语说出 doctor 给的安装行；clone 失败拒 `E_TOOL_UNAVAILABLE`，主体带 git 标准错误的最后一行，恢复语「在浏览器里打开这个地址确认它存在，再试一次」；超时拒 `E_TIMEOUT`。`subdir` 缺席时包根就是仓库根。之后 `plan_install` 读暂存目录里那一层，`apply` 落位；暂存目录无论成败都删掉，上次崩溃留下的同名暂存目录先删再 clone（与 `place_tree` 的暂存同一口径）。`.git/` 不是包的一部分：clone 之后、预检之前删掉它，因为它不是 skill 的内容，而它的对象文件会让整包超过 `PACKAGE_BYTES_LIMIT`。
- **`SkillsSh`**：名字是 `<owner>/<repo>/<skill>` 三段（与 skills.sh 页面上的写法相同），换成 `Git { url: https://github.com/<owner>/<repo>, rev: None, subdir }`，`subdir` 是 clone 里**唯一一个**名为 `<skill>`、内含 `SKILL.md` 的目录；零个或多于一个即拒 `E_INVALID_ARGS`，拒词列出找到的候选。审核随后按 D19 去问 skills.sh 的同一个名字。
- **`Shipped`：自带的 skill 编进二进制**。源树的 `skills/` 在构建时经 city 的构建脚本写进 `OUT_DIR` 并 `include_bytes!`（`xtask packaged` 准许的两处之一），所以四条安装渠道——发行归档、安装脚本（只复制二进制）、npm 与 `cargo binstall`（都只带二进制）、`cargo install`（crates.io 的包里只有包目录）——得到同一份自带 skill；为此源树根上的 `skills/` 搬进 city 的包目录、仍叫 `skills/`（与 city D18 把模板搬进包目录同一条理由），引用它的文档同一次改动改路径，归档的 Skills 部分（`crates/xtask/src/package/contents.rs`）改读新路径。执行者把每一件经 `install` 放进城库的 `shipped` 格（`SHIPPED_SECTION`），来源是一个从内嵌字节在暂存目录里写出的目录，于是预检、原子落位与 CAS 登记仍只有 §8-28 一处。同 digest 的那一件答 `AlreadyShelved`，不写一个字节；同名异 digest 按 §8-28 成为新候选版本，旧版本仍由各楼的 pin 引用。
- **何时放自带的**：新城成形的那一刻放一次（`Shipped`，城库；成形只有 `accounting::worker::genesis::form` 一处，向导与 `sprawling init` 都经它，§8-28c）；已有的城不自动放——它的书架上没有某件自带 skill，可能正是 User 拿下了它——书架页给一个放自带 skill 的控件，发的是同一条 `InstallSkill { source: Shipped }`。放上书架不等于准入：阅览室仍只收 `RULES.toml` 的 `reading_room` 写下的名字（§8-8）。
- **答复**：命令按 `idem` 回执；成功之后页面重读书架（已有的书架查询），失败经命令的拒词回到页面。审核由 D19 在落位之后另起，不进这次答复。

**理由**：§8-28 已经是预检、原子落位与内容哈希的唯一入口，三种远处的来源在门外变成一个本地目录，门里一行不改，规则就只有一份。自带 skill 编进二进制，是因为只有发行归档在二进制旁边带 `skills/`，安装脚本只复制二进制，其余渠道也只有二进制；按「二进制旁边」去找，在四条渠道里有三条找到空。新城自动放、旧城不放，是因为旧城的空书架是 User 的状态，不是缺失。

**被否**：①按 `current_exe()` 旁边的 `skills/` 去找：只在解开的归档里成立；②客户端拼一个 `PutShelved` 把文件逐个发来：`PutShelved` 只携一份文本，包里的目录与脚本进不来，而且预检会在页面与城里各有一份；③每次开城把缺的自带 skill 补回去：User 拿下的那件会在下次开城时回来；④用库克隆（`gix`）代替 git 程序：多一棵依赖树，换来的只是不需要 PATH 上的 git，而 doctor 已经为工作树找 git。

**重开参数**：自带 skill 的总字节超过 1 MiB，或 User 要求不带自带 skill 的二进制时，重议内嵌；`CLONE_TIMEOUT` 与 `SHIPPED_SECTION` 是推断值，各是一个常量。

**三个平台**：内嵌与安装渠道无关，三个平台上自带 skill 都在二进制里；城库在三个平台上都是 `<city>/.sprawling/library/shipped/`；git 在 Windows 上找 `git.exe`，在 macOS 与 Linux 上找 `git`，都按 PATH。
-/

/-! D37 许可通知随每件自带 skill 落位：`skills/LICENSES.md` 进内嵌表，写出时进入每个 skill 目录

**决定**：city 的构建脚本把 `skills/LICENSES.md` 作为表里一行没有 `/` 的条目（`LICENSES.md`）写进 `OUT_DIR` 的表，`library::shipped::stage` 把它写进每一件 skill 的暂存目录；同一目录的 `README.md` 仍只随发行归档走。

**理由**：MIT 要求通知随副本走。四条安装渠道里只有发行归档在二进制旁边带 `skills/`，安装脚本、npm、`cargo binstall` 与 `cargo install` 都只交付二进制或包目录；落上书架的那一份正是 MIT 意义上的副本，而在此之前它只有 `SKILL.md`，没有版权行与许可全文。`LICENSES.md` 已经是全部通知的唯一权威，把它放进每个 skill 目录，比在六份 `SKILL.md` 里各复制一遍更少维护，也不再依赖「归档在二进制旁边」这个只在一条渠道成立的假设。

**被否**：①每件 skill 各带一份许可文件：六份副本，改一处要改六处；②把 `LICENSES.md` 单独落在城库里：城库只保留 CAS 与索引行、不保留文件目录，且 `install` 只收一个包目录；③只靠发行归档：另外三条渠道的读者看不到它。

**重开参数**：上游许可改变通知要求，或自带 skill 换到不要求随副本携带通知的许可时，重议这一行。

**本节测试**：`library::shipped::tests::every_staged_skill_carries_the_licence_list`——表里恰有一行没有 `/`，暂存之后每一件 skill 目录里的 `LICENSES.md` 与那一行的字节相等。
-/

/-!
### 8-28c 自带 skill 上架（`library::shipped`，形状 2 值＋一个落盘动作）

```rust
// city::library::shipped
pub const SHIPPED_SECTION: &str = "shipped";
pub fn shelve_shipped(city_root: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Vec<Installed>, AxError>;
```

- **字节从哪来**：city 的构建脚本（`crates/city/build.rs`）走包目录里的 `skills/`，为其中每一个含 `SKILL.md` 的子目录的每个文件生成一行 `(相对路径, include_bytes!(…))`，写进 `OUT_DIR` 下的一张表，`library::shipped` 经 `include!` 读它。`skills/` 根上的 `README.md` 不是 skill，不进表，只随发行归档走（D20）；同目录的 `LICENSES.md` 也不是 skill，但作为一行没有 `/` 的条目进表，落位时写进每一件 skill 的目录（D37）。表按相对路径排序，同一棵树在三个平台上生成同一张表；路径段在表里以 `/` 分隔，落盘时逐段 `join`，于是 Windows 上不出现 `/` 与 `\` 混写。
- **只经 §8-28 一扇门**：`shelve_shipped` 在 `<library>/.shipped/` 这个点开头的暂存目录里按表写出每件 skill 的目录（扫描跳过点开头的项，读者看不见它），再对每件调用 `install(city_root, Slot::library(SHIPPED_SECTION), <暂存>/<name>, register)`；预检、原子落位与 CAS 登记都不在这里另写。暂存目录先删再写，成败之后都删：它从未被换入。
- **playback 页面组装**：`skills/playback/src/skeleton.html` 按内联标记的顺序声明 CSS 与脚本片段，`skills/playback/assemble.js` 将它们逐字节组装成带生成标记的 `template.html`；脚本片段共用同一个内联脚本作用域，不增加浏览器模块加载或网络依赖。生成页仍由上述表原样内嵌，发行归档也携带同一份页；CI 用生成器的 `--check` 比对整份字节，既有 playback 验收经生产工具导出后检查 structure、source 与 offline。渲染片段持有时间线绘制，将来的 Three.js 或 WebGPU 实现也在此处维护并内联组装。
- **字节照原样**：表里是源树里的字节，写出与登记都不改行尾——仓库的 `.gitattributes` 让它们在三个平台的检出里都是 LF，书架上的那一份就是二进制里的那一份。
- **一件自带 skill 的「版本」就是它的 manifest digest**：没有另记的版本号。§8-28 的包身份回答「装下的是哪一份」；新的二进制带来不同的字节时，已有的城不被改动（D20），新城得到新的那份。
- **何时调用**：只在新城成形时一次（`form`，它对已有历史的目录恒拒，所以对一座城至多一次）。开城不调用：书架上缺一件自带 skill，可能是 User 拿下了它（D20 被否的③）。
- **记在账上**：`shelve_shipped` 答回每一件的 `Installed`；`form` 在第零行之后为其中每件 `Placed::Fresh` 的写一行 `skill_shelved`（kernel D23，`source = Shipped`，`digest` 是 `Installed::hash`，`addr` 缺席），按 `shelve_shipped` 答回的次序。账本在第零行之前不存在，所以行晚于落位；被拒的成形既不落行也不留历史，重来的成形里已落下的那几件答 `AlreadyShelved`、不写行——它们的这一版于是没有上架行，读者读作 `Unrecorded`（wire D33）。
- **失败**：任一件被 §8-28 拒（例如目录里事先已有同名而内容不同的一件）即返回那条拒词，`form` 在写下账本第零行之前放 skill，于是被拒的成形不留下历史，修好之后再成形时已落下的那几件答 `AlreadyShelved`；暂存目录写不了是 `E_STORAGE_FATAL`。
- **三个平台**：表与字节与平台无关；城库在三个平台上都由 `<city>/.sprawling/library/INDEX.toml` 中 `section = "shipped"` 的索引行定位到 CAS，不保留 `<name>/` 文件目录；暂存路径一律经 `Path::join` 拼。

下面的模型是一座城在一条事件轨迹上的书架：成形、开城、User 拿下一件、User 改一件。证明两条，各在一条轨迹上量化：自带的每一件至多被放一次（`a_builtin_is_placed_at_most_once`）；成形之后被 User 拿下的一件，此后无论再发生什么都不回到书架上（`a_removed_builtin_is_not_put_back`）。

**本节测试**：`accounting::worker::genesis::tests` 里两例：成形一座新城，扫描城库，`shipped` 格里的名字恰是包目录 `skills/` 下每件 skill 的名字，且第二次成形被拒后书架不变；成形的账上每件自带 skill 恰有一行 `skill_shelved`，摘要等于书架索引持有的 canonical manifest digest、`source` 是 `Shipped`。
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
