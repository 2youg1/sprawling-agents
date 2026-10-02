-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::read

规定 `tools::read`、`tools::read::miss`、`tools::read::package`、`tools::read::locator`（`crates/runtime/src/` 下同名的文件）。read 的区间读、包、没命中时的 nearby、打开 Locator，以及城外书架上的 skill。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-29 runtime::tools::read 区间读（字节预算）


**两道天花板，紧的那道说了算。** 512 行界定一次作答携带多少**结构**；`INTERVAL_CAP_BYTES`（64 KiB）界定它花掉窗口的多少**字节**——本仓源码一行约四十字节，而一个生成物可以整份压在一行里，故行上限单独不成其为界。

**字节上限切在行边界上，且落在 `Interval::cut` 内部而不在下游。** 理由只有一条：`next_offset` 是调用方续读的唯一凭据，若字节在本函数报出 `next_offset` 之后才被裁掉，那个数就会指过一批没人交付的行，而这个缺口是无声的。首行恒交付，无论它多长——一次返回空的作答会把收到的 offset 原样递回去，调用方于是永远问同一个问题。

`search` 共用同一个上限，接在它已有的 `truncated` 上：`MATCH_CAP = 64` 界定**几条**答案上路，不界定它们**多大**。**首个命中同样受它约束**：一条命中的上下文块自己就超过上限时，由 `elision::splice` 从尾部切进上限、带 `[truncated: N bytes]`，然后停走并报 `truncated`。`read` 的首行恒交付而 `search` 不，是因为 `search` 报的是行号、续读走 `read`，不存在一次空答让调用方原地打转的那个缺口；一整行压缩产物原样塞进一条命中，就是这道上限要拦的那种输入。

**取 64 KiB 的推导与实测见 `crates/kernel/Spec.lean` §8-8 该常量的 doc**；一句话是：上下文提醒按 30%／65% 排成梯子，没有哪一步可以整级跨过，故单条结果 < 35% 窗口，而 64 KiB 在最坏字节-token 比率下是 128K 窗口的 20.5%。

`ReadTool` 增 `offset`（0 基行号，缺省 0）与 `limit`（缺省与上限**均为 512 行**）。被截断时结果携 `total_lines` 与 `next_offset`，所以「我拿到的是不是全部」不需要猜。`bytes` 字段的语义不变，仍是本次返回文本的长度。

理由：整读一份千行级的 SPEC 或数十 KB 的 ARCHITECTURE，要么吃掉整个窗口，要么被管线从中间剪掉，而剪掉的往往正是要改的那一段。512 是选定值。

#### 8-29-1 参数与结果（实现照此，不另选）

```rust
// args：{path, offset?: u64, limit?: u64}
// offset 缺省 0；limit 缺省 512，大于 512 者**夹到 512** 而非拒绝——
// 模型多要一点不该赔掉一个回合，它拿到的截断字段会把真相说清楚。
// offset／limit 非整数或为负＝E_INVALID_ARGS；limit==0 同。
const LINE_CAP: u16 = 512;
```

**上界由类型给，不由换算的失败支给。** `limit` 以 `u16` 携带：大于 `u16::MAX` 的请求与大于 512 的请求是同一件事，都夹到 512，而 `usize::from(u16)` 没有失败支。`offset` 以请求里的 `u64` 携带，在切行处与总行数比较：一个 `usize` 装不下的 offset 越过了内存装得下的任何文本的末尾，于是落进下表「越过末尾」那一行，而不是被换成一个碰巧很大的数。`total_lines`、`next_offset`、`bytes` 都以 `usize` 直接成为 JSON 数。

切行按 `split_inclusive('\n')`：每行连它自己的换行符一起数、一起还，所以 `offset=0, limit>=total` 的返回与整读**逐字节相同**，`bytes` 字段的旧语义因而不动。

| 情形 | `text` | `total_lines` | `next_offset` |
|---|---|---|---|
| `offset + 返回行数 < total_lines`（截断） | 该区间 | 有 | 有，＝`offset + 返回行数` |
| 读到文件末尾 | 该区间 | 无 | 无 |
| `offset >= total_lines`（越过末尾） | 空串 | 有 | 无——后面没有东西了，给一个 `next_offset` 就是请模型原地打转 |

目录路径与 catalog 条目走同一条切行路：一个条目短到永不触顶，而两条路就是两个权威。

#### 8-29-2 保留区判定移出

`resolve` 里「`Address::parse` 后判 `is_reserved`」这一段移进 `runtime::tools::chosen_path`（§8-30-1），`read` 与新的 `search` 同调它。理由是一条硬约束：模型选的路径能不能到保留区，全城只允许有一个答案与一组测试。

#### 8-29-3 一件藏品是一个目录：`<名>/<相对路径>`（`runtime::tools::read::package`）

```rust
// read::package
pub(super) fn open_in_package(catalog: &Catalog, city_root: &Path, asked: &str) -> Option<Result<Found, AxError>>;
pub(super) fn open_document(city_root: &Path, shelved: &Address, asked: &str) -> Result<Found, AxError>;  // 单文档 skill
```

- **阅览室准入的是整个包**：catalog 条目带着包目录（`CatalogEntry::package`，由 city 的扫描给出，`crates/city/Spec.lean` §8-8）时它是一个包，不从落点的写法去猜——一份恰好叫 `SKILL.md` 的单文档会让整个 section 被当成包；`<名>/<相对路径>` 打开包目录下的那个文件。准入是人写阅览室时做的，所以包内文件与 `SKILL.md` 一样不经读界与保留区判定——它们住在同一个被准入的目录里。
- **名字在前、路径在后，名字先查 catalog**：与整名命中同一条理由（§8-29 起首），一个恰好同名的城内目录遮不住它。首段不是 catalog 里的包（没有这个名字，或它是单份文档）即返回 `None`，交回普通路径那条路。
- **相对路径逐段判形，不做规范化**：空段、`.`、`..`、带反斜杠或冒号的段一律 `E_INVALID_ARGS`，恢复语说出「包内相对路径，只用普通段」。规范化会把一条爬出包的路径「修」成另一条，而拒绝让写错的那一方看见自己写了什么。
- **链接按落点判，不出包目录**：拼出的路径解开链接后的真实位置，必须落在「规范化的城根 + 书架上写的包路径」之下，否则 `E_GATE_DENIED`，恢复语说出「指包里的文件本身，而不是包里链接背后的东西」。以书架上的写法而非包目录的真实位置为准，所以包目录本身是链接时同样拒绝。免于读界的理由只覆盖被准入的那个目录；链接背后的文件没有被准入，而书架上的文件可以由人手或 `exec` 写进来，安装时的预检挡不住它们。文件不存在时同样按落点判：真实位置由 `chosen_path::real_location`（§8-30-1）求出，不存在的尾段不可能是链接，所以包里一条链接背后的缺失文件落在链接目标之下，出包即 `E_GATE_DENIED`，不交给读取去列链接背后的目录。
- **单文档 skill 同样不走链接**：`open_document` 解开链接后的真实位置必须恰是「规范化的城根 + 书架上写的文档地址」，否则 `E_GATE_DENIED`，恢复语说出「请人把文档本身放上书架」。理由与包内文件相同：免于读界的是书架上写的那份文档，链接背后的东西没有被准入。
- **catalog 锁中毒＝`E_STORAGE_FATAL`，不落空到普通路径**：整名与包名同一个口径。落空会让一个与 skill 同名的城内文件或目录顶替它——正是「名字先查 catalog」要挡的那件事；恢复语说出「结束本 run 再续」，因为同一进程里这把锁再也拿不回来。

#### 8-29-4 没命中时给出 `nearby`（`runtime::tools::read::miss`）

```rust
// read::miss
pub(super) enum Floor { Document, Directory { dir: PathBuf, named: String } }
// 打开 read 落到的地方；Located::Absent 不打开，直接答没命中（§8-30-1）；
// Located::Present 打开之后须仍是判过的那个文件，否则 E_GATE_DENIED，读的是已打开的句柄。
pub(super) fn text_at(asked: &str, at: Located, floor: &Floor) -> Result<String, AxError>;
const NEARBY_CAP: usize = 16;
```

- **文件不在＝`E_INVALID_ARGS`，`nearby` 携最近一层存在的目录里的条目**：从被问路径的真实位置往上找第一个存在的目录，**不高于这次调用被准入的那一层（`Floor`）**：普通路径是它首段的真实位置，包是书架上写的包目录，catalog 里的单份文档没有目录可列（`Floor::Document`）。城根与书架上别的藏品因此不会出现在候选里——它们不是这次调用被准入的东西。条目按调用方够得着的写法拼出：普通路径是城内相对路径，包是 `<名>/<相对路径>`；保留区里的项不列，模型本来就读不到它们。按与缺失文件名的共同前缀长度（不分大小写）降序、再按路径排，截到 `NEARBY_CAP`。上限界定的是一次拒绝花掉多少窗口，不是目录多大；被截掉的是最不像的那些。
- **路径是目录＝`E_INVALID_ARGS`，`nearby` 携这个目录自己的条目**：`text_at` 在打开之前问判过的真实位置是不是目录，是就不打开，答「这是目录」，候选按上一条的写法、同一个 `NEARBY_CAP` 列出目录里的条目（不按名字远近排，只按路径排）。模型读一个目录，是在问里面有什么；不先问的话，Windows 打开目录得到的是 os error 5（拒绝访问），Unix 读目录得到的是 EISDIR，两者都落进下一条的「在而打不开」，模型拿到 `retry: no` 和一句按操作系统区域设置写成的系统报错，却拿不到它要的条目。
- **文件在而打不开＝`E_STORAGE_FATAL`，不给 `nearby`**：名字是对的，候选只会误导。
- **打开之后再核一次，不符＝`E_GATE_DENIED`**：`text_at` 打开判过的真实路径，然后核两件事：这条路径此刻的真实位置仍是它自己（路上没有换进来的链接），此刻这条路径上的文件与打开的句柄是同一个文件（`same-file` 的 `Handle`，比的是卷与文件号）；任一不符即拒，拒因不说出链接指向哪里。之后只从已打开的句柄读。两道核验各挡一种换法：判定之后换进来并一直留着的链接，打开与再开都穿过它而相等，只有重求真实位置看得见；打开时是链接、重求前又换回的，只有句柄比对看得见。剩下的窗口要在打开、重求、再开之间来回换三次。打开放在判定之后而不是之前，因为先打开就会在判定前打开链接背后未经判定的东西，Unix 上一个 FIFO 会让这次打开一直阻塞。catalog 里的单份文档也先经 `real_location`（§8-30-1）求真实位置，所以 `text_at` 的每个调用方交来的都是真实路径，核验对它们一视同仁。
- **列目录是尽力而为**：目录列不出或名字不是 Unicode 时 `nearby` 为空，调用方要的拒因是「没命中」本身。
- **恢复语指向 `search`，不指向 `exec`**：每栋楼的工具集都有 `search`，而 City Hall 的工具集里没有 `exec`（`crates/city/Spec.lean` §8-22）；一句指向一件不存在的工具的恢复语会让规划者空转一个回合。

#### 8-29-5 打开一个 Locator：`cas:` 与 `file:`（`runtime::tools::read::locator`）

```rust
// read::locator —— `read` 与 `BoundReader`（§8-59）共用；交回 Locator 本身与它的字节，action 是拒词里点名的工具
pub(in crate::tools) fn open_locator(asked: &str, reader: &BoundReader, action: &'static str)
    -> Option<Result<(Locator, Vec<u8>), AxError>>;
/// cas: 块按哪栋楼判读取界的唯一判定处。
fn judged_at(hash: &B3Hash, origins: &[storage::BlockOrigin],
             bound: &dyn Fn(&Address) -> ReadVerdict, action: &'static str) -> Result<Address, AxError>;
// ReadTool::new(city_root, catalog, bound, block_store: &Path)：块仓是城的，run 可能写在没有自己块仓的 worktree 里。
// ReadTool 把 city_root、bound、block_store 收成一个 BoundReader；Locator 的字节由它读出，再按 UTF-8 交文本。
```

- **以 `cas:` 或 `file:` 开头的参数是 Locator**，按 `Locator::parse` 判形，判不过即 `E_INVALID_ARGS`；其余参数走 catalog 与普通路径，不受影响（一个城内地址不含冒号，两者不相交）。
- **`cas:` 块按存块时记下的楼判读取界，只在 `judged_at` 一处决定**，判本身仍是 `chosen_path::admit`（§8-30-1）那一个。来源是 `Cas::put_for` 在存块时写下的（`crates/storage/Spec.lean` §8-3）：一个块为几栋楼存过就有几条来源，取读者能读的第一栋；一栋都读不了就取第一条来源，让 `admit` 按那栋楼的理由拒绝；没有来源的块（上架的技能包、从未存过的哈希）＝`E_GATE_DENIED`，恢复语让它改读块所出自的 `file:`。只按楼判、不按 run 判：读得了那栋楼的文件就读得了为那栋楼存下的字节，而 run 只记作出处。另一条路是按账本里哪一行写了这个哈希来判，落选：模型写的文字（例如委派的 `goal`）会落进带 `addr` 的行，那样的归属可以伪造。`file:<addr>@<oid>` 按 `<addr>` 判。
- **`file:` 在该 oid 上做 git 读**（`storage::blob_at`，`crates/storage/Spec.lean` §8-29），读的是那一次提交里的字节而不是工作区此刻的文件；地址在该提交里不是一个文件（目录、不存在）＝`E_INVALID_ARGS`。
- **范围**：`cas:` 带的范围照 Locator 本身只交回那一段（`Cas::get_range`）；`file:` 带范围＝`E_INVALID_ARGS`，恢复语让它去掉范围改用 `offset`／`limit`——提交里的文件没有一份按范围读的实现，而 `offset`／`limit` 已答同一个问题。之后都按 `offset`／`limit` 切（§8-29-1）。字节不是 UTF-8＝`E_INVALID_ARGS`，read 只交文本。
- **为 run 存块的调用方都走 `put_for`**：转录（`Transcript::materialise`，记房间）、卸载的原件（`offload::tee`，记命令所在的房间，来源随 `OffloadSite` 传入）、截图（`bin::browser_tool`，记这栋楼）、交接单 must-read 里的规范文档（`accounting::worker::freezing`，记 run 所在的房间）、run 的任务书（`accounting::worker::dispatching::running`，记房间；run id 由任务书的定位符派生，所以先 `put` 取得哈希，run 立起后再 `put_for` 补记来源）、子 run 的交回说明（`accounting::worker::dispatching::handback`，记子 run 与它的房间）。仍走 `put` 的有两类：上架的技能包不是为某个 run 存的，读不到它的 `cas:`，它按 catalog 名读；冻结前缀的各段（`intern_prefix`）只为让账本里的前缀可审计，一个段为同一栋楼的所有 run 共用，不作为定位符交给任何 run。

#### 8-29-6 城外书架上的 skill：catalog 携着它的字节

```rust
// runtime::catalog
pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>;
// entry.expansion＝扫描读到的 SKILL.md 正文；entry.hash＝同一份字节的哈希；entry.package＝None
```

- **按名读到的是钉住的那份字节**：城外书架是别的程序的目录，城给不出地址（`crates/city/Spec.lean` §8-8），所以 catalog 不交地址而交正文。正文由 city 的扫描读一次、哈希一次（`Holding::carried` 与 `Holding::hash` 出自同一次读入），`read <名>` 经 `Expansion::Said` 交回它，走 §8-29-1 同一条切行路。于是这个 run 读到的字节恒等于 `run_started` 里那条 `SkillPin` 说的字节，哪怕那份文件在 run 进行中被它的主人改了。
- **包里的其余文件读不到**：`package` 为 `None`，`<名>/<相对路径>` 不进 §8-29-3 那条路，落回普通路径并按城内路径判（多半是 `E_INVALID_ARGS` 的没命中）。城外的文件不在城根之下，`read` 不打开城外的路径；要整包可读，人把它装进城库（`crates/city/Spec.lean` §8-28），那条路把包里每个文件落在城内。
- **一件 skill 在 catalog 里只有一个名字**：城外的与城内的同名时，书架扫描已经按「近架盖远架」留下城内那一件（`crates/city/Spec.lean` §8-8），catalog 收到的是一件；`admit_skill` 与 `admit_carried_skill` 之间的重名仍拒，与同一扇门里的重名同一个拒词。
-/

/-! D9 城外书架上的 skill 由 catalog 携着正文交给 run

**决定**：阅览室准入的一件城外书架上的 skill 经 `Catalog::admit_carried_skill` 进 catalog，条目的 `expansion` 是扫描读到的 `SKILL.md` 正文，`read <名>` 交回这份正文（§8-29-6）。包里其余文件对它不可读。

**理由**：`docs/getting-started.md` 告诉人，挂上 `[skills] shelves` 再在 `reading_room` 里写下名字，居民就用得上那件 skill；而城外的文件没有城内地址，`read` 只开城根下的路径。携正文让这句话成真，又不给 `read` 开一条去城外读文件的路：模型能选的仍只有城内路径，城外的字节只以「人准入过的那一份」的身份进来。正文与哈希出自同一次读入，所以 pin 说的字节就是 run 读到的字节。

**被否**：①给城外持有编一个城内地址或把城外目录映射进城根——那是一个指向并不在那儿的文件的地址，失败落在第一次 `read` 而不在写清单的时候（`crates/city/Spec.lean` §8-8）；②让 `read` 在调用时去城外路径读——读到的可能不是钉住的那份字节，且 `read` 多出一条不经读界判定的打开路径；③改文档，告诉人城外书架只能浏览不能用——书架挂了却用不上，人从 catalog 上看不出为什么。

**重开参数**：城外 skill 的包里附属文件也要按名读到（例如一件 skill 的 `SKILL.md` 指名它目录里的脚本）时，`Holding` 要带整包的字节或一份只读的城内镜像，本条与 city D9 一起重议。
-/
