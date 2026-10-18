-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::read

规定 `tools::read`、`tools::read::miss`、`tools::read::package`、`tools::read::locator`、`tools::read::window`、`tools::read::index`、`tools::read::hot`（`crates/runtime/src/` 下同名的文件）。read 的区间读、包、没命中时的 nearby、打开 Locator、城外书架上的 skill、读回答案所需的那段窗口，以及截断之后补齐全文事实的索引。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::tools::read::tests`、`runtime::tools::read::tests::locators`、`runtime::tools::read::tests::versions` 守住。
-/

/-!
### 8-29 runtime::tools::read 区间读（字节预算）


**两道天花板，紧的那道说了算。** 512 行界定一次作答携带多少**结构**；`INTERVAL_CAP_BYTES`（64 KiB）界定它花掉窗口的多少**字节**——本仓源码一行约四十字节，而一个生成物可以整份压在一行里，故行上限单独不成其为界。

**字节上限切在行边界上，且落在 `Interval::cut` 内部而不在下游。** 理由只有一条：`next_offset` 是调用方续读的唯一凭据，若字节在本函数报出 `next_offset` 之后才被裁掉，那个数就会指过一批没人交付的行，而这个缺口是无声的。首行恒交付，无论它多长——一次返回空的作答会把收到的 offset 原样递回去，调用方于是永远问同一个问题。

`search` 共用同一个上限，接在它已有的 `truncated` 上：`MATCH_CAP = 64` 界定**几条**答案上路，不界定它们**多大**。**首个命中同样受它约束**：每行先被切到 `LINE_CAP`（`crates/runtime/spec/Tools/Search.lean`），所以一条命中的上下文块已远小于上限；块仍超过上限时，由 `elision::splice` 从尾部切进上限、带 `[truncated: N bytes]`，然后停走并报 `truncated`。`read` 的首行恒交付而 `search` 不，是因为 `search` 报的是行号、续读走 `read`，不存在一次空答让调用方原地打转的那个缺口；一整行压缩产物原样塞进一条命中，就是这道上限要拦的那种输入。

**取 64 KiB 的推导与实测见 `crates/kernel/Spec.lean` §8-8 该常量的 doc**；一句话是：上下文提醒按 30%／65% 排成梯子，没有哪一步可以整级跨过，故单条结果 < 35% 窗口，而 64 KiB 在最坏字节-token 比率下是 128K 窗口的 20.5%。

`ReadTool` 增 `offset`（0 基行号，缺省 0）与 `limit`（缺省与上限**均为 512 行**）。被截断时结果携 `next_offset`，所以「从哪里接着读」不需要猜；`total_lines` 与 `version` 只在**同步可知**时携带：窗口读到了文件末尾（行数随扫描数出），或全文索引里已有这份文件（§8-29-7）。`bytes` 字段的语义不变，仍是本次返回文本的长度。`version` 的语义不变：整份文件的 `edit::version_of`，因为 `edit` 守的和 `plan finish` 引的都是整份文件（kernel D27）；能给出它的只有保住全文的窗口（`offset=0` 且读到文件末尾）或索引——一段被截断的窗口不交整份文件的哈希，半份文件的哈希守不住 `edit` 的并发比对。catalog 条目与 Locator 读出的字节不携 `version`，前者不是可编辑的文件，后者本身已是一个版本。`version` 就是 `cas:b3-<hex64>`，模型原样把它交给 `edit` 与 `plan finish`。截断答案暂时没有 `version` 时，`edit` 之前再读一次：索引在答复之后已经回填（§8-29-7），第二次读同步带上它。

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

| 情形 | `text` | `total_lines` | `next_offset` | `version` |
|---|---|---|---|---|
| 截断（窗口内未见文件末尾） | 该区间 | 索引有则有，否则无 | 有，＝`offset + 返回行数` | 索引有则有，否则无 |
| 读到文件末尾 | 该区间 | 无——答案已是全部 | 无 | `offset=0`：有（窗口保住了全文）；`offset>0`：索引有则有，否则无 |
| `offset >= total_lines`（越过末尾） | 空串 | 有 | 无——后面没有东西了，给一个 `next_offset` 就是请模型原地打转 | 索引有则有，否则无 |

**一处切行，两个来源。** `Interval::cut(text, more)` 是这条规则唯一的权威：catalog 条目与 Locator 字节是内存里的整份文本，以 `more = false` 调它；文件经 `read::window` 流式读入，只把答案那段窗口留在内存里，以「扫描是否见到了文件末尾」作 `more` 调它。截断的判定因此是「交付的行数之外还有字节」，而不是「这份文本后面还有行」。目录路径与 catalog 条目短到永不触顶，与文件同用一个切行函数。

**答复路径只读答案所需的那段窗口。** 一次调用不再先把整份文件读进内存：`read::window` 从文件开头按 64 KiB 的块流式读入、随扫随数换行，第 `offset` 行之前的字节随扫随弃，从那行起留到切行成立或文件末尾为止。索引命中（§8-29-7）且 `offset>0` 时连扫描都省掉：行偏移表给出第 `offset` 行的字节位置，对齐到扇区后一次窗口读直接落在答案上；`offset >= total_lines` 时连内容都不读，校验索引后直接答空。整读一份小文件（`offset=0` 且窗口内见到末尾）的行为与旧契约逐字节相同。

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
// Floor 只在没命中或读到目录时才算：成功路径不再为每一次调用付首段的真实位置解析，
// resolve 把城根与地址交给 miss，miss 在需要列出条目时才调 Floor::of_address。
const NEARBY_CAP: usize = 16;
```

窗口读取在 `read::window`：`Located::Absent` 不打开，直接答没命中（§8-30-1）；`Located::Present` 打开之后须仍是判过的那个文件（`still_judged`），否则 E_GATE_DENIED，读的是已打开的句柄。

- **文件不在＝`E_INVALID_ARGS`，`nearby` 携最近一层存在的目录里的条目**：从被问路径的真实位置往上找第一个存在的目录，**不高于这次调用被准入的那一层（`Floor`）**：普通路径是它首段的真实位置，包是书架上写的包目录，catalog 里的单份文档没有目录可列（`Floor::Document`）。城根与书架上别的藏品因此不会出现在候选里——它们不是这次调用被准入的东西。条目按调用方够得着的写法拼出：普通路径是城内相对路径，包是 `<名>/<相对路径>`；保留区里的项不列，模型本来就读不到它们。按与缺失文件名的共同前缀长度（不分大小写）降序、再按路径排，截到 `NEARBY_CAP`。上限界定的是一次拒绝花掉多少窗口，不是目录多大；被截掉的是最不像的那些。
- **路径是目录＝`E_INVALID_ARGS`，`nearby` 携这个目录自己的条目**：打开按普通文件去试，在错误路径上判出它是目录——Windows 打开目录得到的是 os error 5（拒绝访问），Unix 读目录得到的是 EISDIR——答「这是目录」，候选按上一条的写法、同一个 `NEARBY_CAP` 列出目录里的条目（不按名字远近排，只按路径排）。模型读一个目录，是在问里面有什么；成功的读路径上因此不为「是不是目录」多付一次元数据调用。
- **文件在而打不开＝`E_STORAGE_FATAL`，不给 `nearby`**：名字是对的，候选只会误导。
- **是不是文本按交付的窗口判**：窗口不是 UTF-8＝`E_INVALID_ARGS`，恢复语与 Locator 那条同；窗口之外的字节不再整文校验，一份尾部损坏的文件不再拒绝它的头部。
- **打开之后再核一次，不符＝`E_GATE_DENIED`**：窗口读打开判过的真实路径，然后核两件事：这条路径此刻的真实位置仍是它自己（路上没有换进来的链接），此刻这条路径上的文件与打开的句柄是同一个文件（`same-file` 的 `Handle`，比的是卷与文件号）；任一不符即拒，拒因不说出链接指向哪里。之后只从已打开的句柄读。两道核验各挡一种换法：判定之后换进来并一直留着的链接，打开与再开都穿过它而相等，只有重求真实位置看得见；打开时是链接、重求前又换回的，只有句柄比对看得见。剩下的窗口要在打开、重求、再开之间来回换三次。打开放在判定之后而不是之前，因为先打开就会在判定前打开链接背后未经判定的东西，Unix 上一个 FIFO 会让这次打开一直阻塞。catalog 里的单份文档也先经 `real_location`（§8-30-1）求真实位置，所以 `text_at` 的每个调用方交来的都是真实路径，核验对它们一视同仁。
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

/-!
#### 8-29-7 全文索引：截断之后补齐（`runtime::tools::read::index`）

```rust
// read::index
pub(super) struct Index { /* Mutex：条目 BTreeMap<PathBuf, Entry>（键是判过的真实路径）与在途回填的路径表，条目与回填各记一个观察序号 Seen */ }
pub(super) struct Entry {
    size: u64, modified: SystemTime,       // 命中校验：与本次打开句柄的元数据相等才采信
    total_lines: usize, version: String,   // 截断答案缺的那两个字段
    line_offsets: Vec<u64>,                // 每行的起始字节，选行读的落点表
}
// 命中即同步交出 total_lines 与 version；line_offsets 仅供 window 寻址。
// 未命中即装配一次后台回填，回填写回时替换该路径的旧条目；同一路径在回填完成前只装配一次，
// 在途回填满 2 条时不再装配。forget：选行读发现落点表不再落在行首时丢掉条目。
// observe：读者打开文件之前取一个递增的 Seen；回填在装配时取。写回只在自己的 Seen 不小于现存条目与在途回填的 Seen 时生效。forget 带着被拒条目的 Seen：只删除仍是那一条的条目，只作废在它之前装配的在途回填；作废的回填写回时什么也不改，之后装配的回填照常写回。
```

- **两个来源**。整读（`offset=0` 且读到末尾）在答复路径上顺手建成条目：行表随扫描得出，`version` 随整文哈希得出，不添一字节 I/O。截断或 `offset>0` 的读在索引未命中时装配一条后台回填：整读该文件（缓冲读，D11）、算 `edit::version_of`、数行并记下行偏移，写回索引并替换该路径的旧条目——文件改过之后，旧条目不命中，下一次未命中的读就让它重新回填。回填至多读 `ELIGIBLE_SIZE`（12 MiB）加一字节，读前读后从同一句柄取的 size 与 `modified` 须相等、读到的字节数须等于 size，否则不写条目：读的过程中被改写或长过上限的文件没有条目，而不是半份文件的条目。整读与回填谁先写回不定，所以新旧不按完成先后判，也不按 `modified` 判——文件可以被一份 `modified` 更早、长度相同的文件替换——而按观察序号 `Seen`：整读在打开文件之前取一个，回填在装配时取一个（它打开文件还在其后）；写回只在自己的 `Seen` 不小于现存条目、且整读的 `Seen` 不小于在途回填的 `Seen` 时生效。看得晚的事实总留下来，两种完成顺序结果相同。回填不在答复路径上，答复先走。
- **命中校验**：`size` 与 `modified` 取自本次打开句柄的元数据，与条目相等才采信。校验不过顶多交出一个陈旧 `version`，而 `edit` 自己重算整文哈希比对（kernel D27），陈旧版本只换来一次 `VersionConflict`——索引是捷径，不是权威，这条链 fail-closed。
- **落点校验**：size 与 `modified` 都不变也可能已被改写（同长度改写后把时间戳设回去），这时行偏移表会把选行读带到别的字节上。所以选行读不只信元数据：落点的前一字节必须是换行，读到文件末尾时数出的行数必须等于条目的 `total_lines`。任一不符即 `forget` 该条目，本次答复从文件开头流式扫描、不携 `version` 与 `total_lines`，并装配一次回填；交出的永远是所要的那几行。
- **容量**：条目按行偏移表的字节数计价（单文件表 ≤ 2 MiB、索引总账 ≤ 32 MiB），超出即不收该文件；满时按插入先后弹出。越过上限的文件选行读退回流式扫描，行为仍正确。
- **线程**：每条回填一个短线程，答复之后开始、写完索引即结束；同时在途的回填至多 2 条（`PENDING_CAP`），满了的未命中不装配，由之后的读再装配。一条回填至多握 12 MiB 加一字节的内容，所以并行读再多，回填占的内存也有上界（ARCHITECTURE.md 线程表）。

-/

/-! D10 答复路径只读窗口，version 与 total_lines 改由索引补齐

**决定**：一次文件读只把答案所需的那段窗口留在内存里（§8-29-1 末段）；`version` 的语义不变——整份文件的 `edit::version_of`（kernel D27）——但它与 `total_lines` 只在同步可知时携带：窗口见到文件末尾，或全文索引已有该文件（§8-29-7）。整读顺手建成索引条目；截断或 `offset>0` 的未命中装配后台回填。

**理由**：同一台机器冷读同一份 1 MiB 文本，裸 `std::fs::read` 的 p50 是 993 µs，工具是 2,897 µs；时间账（整读、4 次 canonicalize、整文 BLAKE3、26,215 行整表切行）里只有读文件一项是答案本身要的。窗口读把答复路径的 I/O 收到 64 KiB 级，行偏移表把选行读的扫描也省掉。

**被否**：① `version` 改哈希交付窗口——半份文件的哈希守不住 `edit` 的并发比对，D27 保的是整份文件；②答复改发 version 凭据、`edit` 时再解析——`version` 因此失去跨会话的内容寻址性，`plan finish` 引用的也是这个字面量；③截断答案照带 `total_lines`——数行要扫全文，等于没省；④改动 512 行／64 KiB 上限——延迟对上限不敏感（典型源码 512 行约 20 KiB，一次设备往返读完），64 KiB 是 kernel §8-8 按窗口预算推导的值且 `search` 共用，为数十微秒动上下文预算是亏本。

**重开参数**：真实工作流里「再读一次拿 `version`」被证明比一个回合还贵（例如小文件的 `offset>0` 读成为主流），则放宽同步携带的条件或给 `edit` 一条接受索引版本的快路。
-/

/-! D11 窗口读走直通 I/O，回填走缓冲读

**决定**：`read::window` 在 Windows 以 `FILE_FLAG_NO_BUFFERING`、在 Linux 以 `O_DIRECT` 打开并按扇区对齐的缓冲读入，有效字节按句柄元数据的 size 截断；直通打开或读取失败即回退缓冲读。macOS 的标准库没有直通标志，走缓冲。回填整读用缓冲读。

**理由**：原型（400 样本交错、每变体独立冷文件）里直通窗口读的 p95 是整读基线的 0.27–0.32×，缓冲窗口读是 0.57–0.65×——页缓存填充在冷读上花约 500 µs，而这批字节多半不会再被用到。回填用缓冲读，把页缓存预热留给后续的缓冲读者。

**被否**：缓冲窗口读（数字见上）；内存映射窗口（0.35–0.43×，且要给 runtime 引一个 unsafe 依赖）。直通读不享页缓存，同一文件的重读仍是设备往返——接受，因为首次冷读才是瓶颈。

**重开参数**：某个部署的文件系统上直通读慢于缓冲读（网络盘、过滤驱动干预），或回退路径在 CI 覆盖不到的平台出错。
-/

/-! D12 窗口读不靠页缓存提速，读完即放手

**决定**：`read` 的窗口读走直通 I/O（Windows `FILE_FLAG_NO_BUFFERING`、Linux `O_DIRECT`），不为了后续读而把文件留在页缓存里。后台回填按 D11 走缓冲读；它只算出 total、version 与行偏移，算完即释放，不把整份文件的内容留在内存里，同时在途的条数与每条读入的字节都有上限（§8-29-7）。

**理由**：把文件留在页缓存会让驻留内存随读过的文件规模增长，而一个 run 会读很多文件；这条长尾代价比一次设备往返昂贵得多，且它以「内存里剩下的东西」的形式出现，事后很难归因。直通读不享重复命中，但重复命中的收益换不来这份确定。索引仍然缓存每文件的行偏移表与两个字段，它们按字节计价且总量封顶（§8-29-7），与文件内容无关。

**被否**：①索引命中后改走缓冲读（RAM 长尾，且让同一份文件的延迟随机器状态而变，复测无法归因）；②把文件内容缓存起来供后续选行读（增长速度最快的一种）。

**重开参数**：某个部署的文件读取次数远高于文件数（每次读同一批大文件很多遗），并且内存预算能明确地跟住这份驻留。
-/

/-!
#### 8-29-8 一次阅读的一小段留在手里（`runtime::tools::read::hot`）

```rust
// read::hot
pub(super) struct Hot { /* Mutex<Option<Held>>，全 run 一个槽 */ }
pub(super) struct Stamp { size: u64, modified: SystemTime }   // 句柄给的大小与写时刻，总是一起走
struct Held { real: PathBuf, stamp: Stamp, base: u64, bytes: Vec<u8> }
// covered：同一个真实位置、同一个大小与戳，且盖住要的那段时才作答；否则不答。
// keep：一个被截断的作答留下它读过的字节。release：读完或换文件时放手。
```

- **只有「文件本身被截断」才留着**：模型接下来大概率会读完，于是后续续读是热的（一次 `memcpy`，无设备往返）。整份读完、或读的是另一份文件，一律放手。一个 run 因此最多握一份文件的那一段，规模化下每个 session 的额外驻留是 1–2 MiB 级。
- **槽只回答字节，不回答判定**：调用方已经打开文件、并且知道它是判过的那个，才来问槽；所以陈旧的槽只可能不命中，不可能把读带到别处。
- **换文件时释放，但不在锁里释放**：`covered` 与 `release` 都在锁内把旧值 `take` 出来、出了锁再丢（D15）。这样下一次读不会排在「丢上一份文件」后面。
- **没有时钟**：工具不采样时间（ARCHITECTURE.md 的时间规则），所以不做 N 分钟的过期；模型放弃续读时，那份字节留到下一次读或 run 结束，上界是上面那 1–2 MiB。

-/

/-! D13 一次 open 回答判定、文件事实与窗口

**决定**：Windows 上城内路径的读经 `read_ffi` 的 Zig 叶子。叶子直接打开模型写下的那条路径（沿途链接由系统解开），在同一次打开里交回真实位置、大小、写时刻与窗口的字节。判定仍在 Rust 的 `chosen_path`：写下的地址先过 `admit`，叶子交回的真实位置再折算成城内地址（`CityRoot::spelled_under`）过一次 `admit`——落在城外即 `E_GATE_DENIED`，与 `land` 同一个拒词。判定落在打开之后而非之前：句柄钉住文件，判定与读取之间没有可以换进链接的窗口；判定若拒绝，读到的字节随答错丢弃，永不交付。叶子的任何非成功答案（没命中、目录、打不开、本平台无叶子）都退回 `land` + 安全门——miss 的 nearby 与目录的条目只在判过的位置上列，这一个权威不搬走。

**竞态语义与 `still_judged` 不同，且更强**：安全门是「判—开—复核」，换进链接在复核时被拒；叶子是「开—钉住—判」，交换发生时手里已是判定地址的那个文件，交换的东西从未被打开。若读界在判定中改变了目录（交换的一种），手里这份正是判过并打开的文件，判给开的那个地址之外的内容不被读。`tools::read::tests::doors` 的那条换链接测试因此接受两种答案：`E_GATE_DENIED`，或判定落点自己的字节——机密楼的字节永远不被读。

**理由**：安全路径要 4 次 open（`canonicalize` 两次、`Handle::from_path`、读取本身）与 2 次真实位置解析，实测 358 µs；单次 open 加一次句柄真实位置查询是 88 µs（400 样本交错的原型）。

**被否**：①为了省一次 open 而放弃「打开后复核真实位置」——那是把链接换入的窗口放回来；②在 Zig 里判「是否落在城内」——准入只有一处权威，事实可以搬走，政策不能；③叶子命中时也先 `land`——同一真实位置解析两次，358 µs 全花掉，而叶子交回的恰好就是盘在打开那一刻的答案。

**重开参数**：某个平台的句柄查询比 `canonicalize` 更贵，或叶子在 CI 覆盖不到的目标上装配失败（此时 `read` 回退到安全门，行为不变）。
-/
