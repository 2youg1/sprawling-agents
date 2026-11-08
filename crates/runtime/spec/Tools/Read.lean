-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::read

规定 `tools::read`、`tools::read::engine`、`tools::read::miss`、`tools::read::package`、`tools::read::locator`（`crates/runtime/src/` 下同名的文件）。read 的区间读、包、没命中时的 nearby、打开 Locator，以及城外书架上的 skill。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::tools::read::tests`、`runtime::tools::read::tests::locators`、`runtime::tools::read::tests::looks` 守住。
-/

/-!
### 8-29 runtime::tools::read 区间读（字节预算）


**两道天花板，紧的那道说了算。** 512 行界定一次作答携带多少**结构**；`INTERVAL_CAP_BYTES`（64 KiB）界定它花掉窗口的多少**字节**——本仓源码一行约四十字节，而一个生成物可以整份压在一行里，故行上限单独不成其为界。

**字节上限切在行边界上，且落在 `Interval::cut` 内部而不在下游。** 理由只有一条：`next_offset` 是调用方续读的唯一凭据，若字节在本函数报出 `next_offset` 之后才被裁掉，那个数就会指过一批没人交付的行，而这个缺口是无声的。首行恒交付，无论它多长——一次返回空的作答会把收到的 offset 原样递回去，调用方于是永远问同一个问题。

`search` 共用同一个上限，接在它已有的 `truncated` 上：`MATCH_CAP = 64` 界定**几条**答案上路，不界定它们**多大**。**首个命中同样受它约束**：每行先被切到 `LINE_CAP`（`crates/runtime/spec/Tools/Search.lean`），所以一条命中的上下文块已远小于上限；块仍超过上限时，由 `elision::splice` 从尾部切进上限、带 `[truncated: N bytes]`，然后停走并报 `truncated`。`read` 的首行恒交付而 `search` 不，是因为 `search` 报的是行号、续读走 `read`，不存在一次空答让调用方原地打转的那个缺口；一整行压缩产物原样塞进一条命中，就是这道上限要拦的那种输入。

**取 64 KiB 的推导与实测见 `crates/kernel/Spec.lean` §8-8 该常量的 doc**；一句话是：上下文提醒按 30%／65% 排成梯子，没有哪一步可以整级跨过，故单条结果 < 35% 窗口，而 64 KiB 在最坏字节-token 比率下是 128K 窗口的 20.5%。

`ReadTool` 增 `offset`（1 基行号，缺省 1，§8-29-1）与 `limit`（缺省与上限**均为 512 行**）。被截断时结果携 `total_lines` 与 `next_offset`，所以「我拿到的是不是全部」不需要猜。`bytes` 字段的语义不变，仍是本次返回文本的长度。结果不携版本，读也不对内容算哈希：读到盘上的一个文件时，读之前与读之后从同一个句柄各取一次戳（长度、修改时间、文件身份），两次相等且这次读算作「看过」（整份未截断，或调用显式给了 `offset`/`limit`）就记进本 run 的读戳表，`edit` 凭它准入（`crates/runtime/spec/Tools/Edit.lean` §8-64）。catalog 条目、包内文档与 Locator 读出的字节不记戳，因为 `edit` 改不到它们。

理由：整读一份千行级的 SPEC 或数十 KB 的 ARCHITECTURE，要么吃掉整个窗口，要么被管线从中间剪掉，而剪掉的往往正是要改的那一段。512 是选定值。

#### 8-29-1 参数与结果（实现照此，不另选）

```rust
// args：{file_path, offset?: u64, limit?: u64}——Claude Code 的 `Read` 的名字（§8-14-1）
// offset 是要读的第一行的行号，从 1 数起，缺省 1；0 也读作第一行。
// limit 缺省 512，大于 512 者**夹到 512** 而非拒绝——
// 模型多要一点不该赔掉一个回合，它拿到的截断字段会把真相说清楚。
// offset／limit 非整数或为负＝E_INVALID_ARGS；limit==0 同。没有 file_path（包括较早的版本拼的 `path`）同码拒，恢复语说出三个参数名。
const LINE_CAP: u16 = 512;
```

**行号对模型从 1 数起，对账本从 0 数起**（D119）。调用的 `offset` 是 1 基行号，进工具的第一件事是减一成为下标；结果载荷里的 `offset`、`next_offset` 与 search 命中的 `line`、`from` 仍是 0 基下标，因为账本里已有的每一行都这样写，渲染器按自己的版本把下标画成行号（§8-63）。下文的 `offset` 都指下标。

**上界由类型给，不由换算的失败支给。** `limit` 以 `u16` 携带：大于 `u16::MAX` 的请求与大于 512 的请求是同一件事，都夹到 512，而 `usize::from(u16)` 没有失败支。`offset` 以请求里的 `u64` 携带，在切行处与总行数比较：一个 `usize` 装不下的 offset 越过了内存装得下的任何文本的末尾，于是落进下表「越过末尾」那一行，而不是被换成一个碰巧很大的数。`total_lines`、`next_offset`、`bytes` 都以 `usize` 直接成为 JSON 数。`offset` 不为 0 时结果另携它（`u64`，即请求里的行号减一），缺席即 0：模型读到的头与行号标记从它数起（§8-63），而渲染器只读账本里的这一行。

切行按 `split_inclusive('\n')`：每行连它自己的换行符一起数、一起还，所以 `offset=0, limit>=total` 的返回与整读**逐字节相同**，`bytes` 字段的旧语义因而不动。

| 情形 | `text` | `total_lines` | `next_offset` |
|---|---|---|---|
| `offset + 返回行数 < total_lines`（截断） | 该区间 | 有 | 有，＝`offset + 返回行数` |
| 读到文件末尾 | 该区间 | 无 | 无 |
| `offset >= total_lines`（越过末尾） | 空串 | 有 | 无——后面没有东西了，给一个 `next_offset` 就是请模型原地打转 |

目录路径与 catalog 条目走同一条切行路：一个条目短到永不触顶，而两条路就是两个权威。

#### 8-29-2 保留区判定移出

`resolve` 里「`Address::parse` 后判 `is_reserved`」这一段移进 `runtime::tools::chosen_path`（§8-30-1），`read` 与新的 `search` 同调它。理由是一条硬约束：模型选的路径能不能到保留区，全城只允许有一个答案与一组测试。

#### 8-29-3 一件藏品由 pin 的 manifest 描述：`<名>/<相对路径>`（`runtime::tools::read::package`）

```rust
// read::package
pub(super) fn open_in_package(catalog: &Catalog, block_store: &Path, asked: &str) -> Option<Result<Found, AxError>>;
pub(super) fn open_document(block_store: &Path, digest: &B3Hash, asked: &str) -> Result<Found, AxError>;
```

- **阅览室准入的是整个 manifest**：catalog 的 `Expansion::Skill` 只携 admitted manifest digest；`read <名>` 完整读取并校验 manifest，再完整读取它列出的 `SKILL.md`，`<名>/<相对路径>` 则完整读取对应文件 digest。包与单文档走同一扇门，不再从文件路径或扩展名猜形状。
- **名字在前、路径在后，名字先查 catalog**：与整名命中同一条理由（§8-29 起首），一个恰好同名的城内目录遮不住它。首段不是 catalog 里的 skill 即返回 `None`，交回普通路径那条路；是 skill 而包内文件不存在则在本次 catalog 条目内拒绝，不落回普通路径。
- **相对路径逐段判形，不做规范化**：空段、`.`、`..`、带反斜杠或冒号的段一律 `E_INVALID_ARGS`，恢复语说出「包内相对路径，只用普通段」。规范化会把一条爬出包的路径「修」成另一条，而拒绝让写错的那一方看见自己写了什么。
- **CAS 完整读取是唯一的授权与完整性边界**：包内文件与 `SKILL.md` 一样不经普通路径读界与保留区判定，因为准入的是 manifest 列出的内容而不是一个可写目录。`Cas::get` 对 manifest 与文件各自重算 BLAKE3；对象被改即 `E_CAS_CORRUPT`，文本不交出。读取不经过文件系统链接，因而没有链接落点或 check-then-use 路径。
- **缺失文件的 `nearby` 来自 manifest**：拒绝列出同一 admitted manifest 的项目，不列书架目录、其他版本或城内相邻目录。
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

- **文件不在＝`E_INVALID_ARGS`，`nearby` 携最近一层存在的目录里的条目**：从被问路径的真实位置往上找第一个存在的目录，**不高于这次调用被准入的那一层（`Floor`）**：普通路径是它首段的真实位置，catalog 里的单份文档没有目录可列（`Floor::Document`）。城根与书架上别的藏品因此不会出现在候选里——它们不是这次调用被准入的东西。条目按调用方够得着的城内相对路径拼出；保留区里的项不列，模型本来就读不到它们。按与缺失文件名的共同前缀长度（不分大小写）降序、再按路径排，截到 `NEARBY_CAP`。上限界定的是一次拒绝花掉多少窗口，不是目录多大；被截掉的是最不像的那些。skill 包内缺失不走这条目录路径，§8-29-3 直接从 admitted manifest 给候选。
- **路径是目录＝`E_INVALID_ARGS`，`nearby` 携这个目录自己的条目**：`text_at` 在打开之前问判过的真实位置是不是目录，是就不打开，答「这是目录」，候选按上一条的写法、同一个 `NEARBY_CAP` 列出目录里的条目（不按名字远近排，只按路径排）。模型读一个目录，是在问里面有什么；不先问的话，Windows 打开目录得到的是 os error 5（拒绝访问），Unix 读目录得到的是 EISDIR，两者都落进下一条的「在而打不开」，模型拿到 `retry: no` 和一句按操作系统区域设置写成的系统报错，却拿不到它要的条目。
- **文件在而打不开＝`E_STORAGE_FATAL`，不给 `nearby`**：名字是对的，候选只会误导。
- **打开之后再核一次，不符＝`E_GATE_DENIED`**：包与单文档的路径，以及 §8-29-4-1 退回的路径，经 `text_at` 打开判过的真实路径，然后核两件事：这条路径此刻的真实位置仍是它自己（路上没有换进来的链接），此刻这条路径上的文件与打开的句柄是同一个文件（`same-file` 的 `Handle`，比的是卷与文件号）；任一不符即拒，拒因不说出链接指向哪里。之后只从已打开的句柄读。两道核验各挡一种换法：判定之后换进来并一直留着的链接，打开与再开都穿过它而相等，只有重求真实位置看得见；打开时是链接、重求前又换回的，只有句柄比对看得见。剩下的窗口要在打开、重求、再开之间来回换三次。打开放在判定之后而不是之前，因为先打开就会在判定前打开链接背后未经判定的东西，Unix 上一个 FIFO 会让这次打开一直阻塞。catalog 里的单份文档也先经 `real_location`（§8-30-1）求真实位置，所以 `text_at` 的每个调用方交来的都是真实路径，核验对它们一视同仁。
- **列目录是尽力而为**：目录列不出或名字不是 Unicode 时 `nearby` 为空，调用方要的拒因是「没命中」本身。
- **恢复语指向 `search`，不指向 `exec`**：每栋楼的工具集都有 `search`，而 City Hall 的工具集里没有 `exec`（`crates/city/Spec.lean` §8-22）；一句指向一件不存在的工具的恢复语会让规划者空转一个回合。

#### 8-29-4-1 模型选的路径按句柄判定（`runtime::tools::read::engine`）

```rust
// read::engine —— 模型选的、已过 admit 的城内地址的整份文本。一次缓冲 open，判定的是读取所用句柄自己的真实位置；
// 退回时答 land + text_at（§8-29-4）给出的那一个结果，所以每一种拒绝与 main 的判定路径逐字节相同。
pub(super) struct Request<'a> { /* city、addr、asked、bound、shown、want：一次读取的完整上下文 */ }
pub(super) enum Looked { /* Text：正文与前后戳；Unchanged：同一句柄戳与已确认区间 */ }
pub(super) fn text_of(request: Request<'_>) -> Result<Looked, AxError>;
```

- **Windows：先开，再问句柄在哪，再判，再从同一个句柄读。** `File::open` 打开城根拼写下的地址（跟随路上的链接，与 canonicalize 内部那一次 open 相同），问这个句柄的真实位置（`GetFinalPathNameByHandleW`，D113 定由谁来问），交给 `CityRoot::judged_location`（§8-30-1）：不在城根之下即拒，落点的拼写不是所写地址即再 `admit`。判定与读取之间没有可换链接的窗口，所以不需要 `still_judged` 的重求与再开。
- **Linux：先求真实位置，再判，再开，再核句柄。** Unix 的 canonicalize 不打开文件，所以不在判定前打开链接背后未经判定的东西（FIFO 会让 open 一直阻塞，§8-29-4 的理由照旧）；判过之后打开真实位置，以 `/proc/self/fd/N` 的 `read_link` 核这个句柄的位置仍是判过的位置，不是＝`still_judged` 的那条 `E_GATE_DENIED`。
- **macOS：同 Linux 的顺序，换一种核法。** macOS 没有命名打开描述符的 procfs，所以打开之后把句柄的卷号与文件号对判定路径此刻指向的文件再求一次，保留身份比较，并用 `rustix::fs::getpath`（`F_GETPATH`）取得已打开句柄的位置，重新交给 `judged_location` 授权后才读；位置无法确认或不获授权即 `E_GATE_DENIED`，不使用可替换的路径查询证明句柄位置。
- **其他平台走 §8-29-4 的判定路径**，不变。
- **除 macOS 句柄位置无法确认或不获授权外，失败都退回判定路径重答一遍**：open、元数据、取真实位置、读出错，句柄是目录，字节不是 UTF-8——这些调用本来就答错误或目录，退回由 `land` + `text_at` 给出与 main 相同的那一句，`nearby` 也在那里算。`Floor` 因此只在退回时求。
- **整读一次，不缓存**：容量按句柄报的长度预留，一次读入，整份校验 UTF-8 不复制；文件字节归操作系统的文件缓存（`crates/sprawling/spec/Serving/Memory.lean` §8-173）。

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

#### 8-29-6 城外书架上的 skill：准入时复制，读取时只认 CAS pin

```rust
// city::library
pub fn admit_external(package: &Path,
    register: &mut dyn FnMut(&[u8]) -> Result<B3Hash, AxError>) -> Result<Admitted, AxError>;
// runtime::catalog：城内与城外准入后都只有一个 admit_skill，Expansion::Skill 携 manifest digest
```

- **按名读到的是钉住的整包**：城外书架是别的程序的目录，扫描为候选构造完整 manifest；准入时经 `admit_external` 把每个文件与 canonical manifest 复制进城 CAS。之后 catalog 只携 manifest digest，`read <名>` 与 `<名>/<相对路径>` 都走 §8-29-3 的完整校验读取，城外路径不再被打开。
- **外部变化是 drift，不是静默换版**：扫描的候选 digest 与准入时复制得到的 digest 不一致即 `E_VERSION_CONFLICT`；复制完成后外部再改，不影响本 run 或之后仍持旧 pin 的 run。新候选经接受动作更新 pin 后才进入新会话。
- **一件 skill 在 catalog 里只有一个名字**：书架扫描按「近架盖远架」留下一个候选（`crates/city/Spec.lean` §8-8），catalog 的 `admit_skill` 对重名仍拒。所有来源共用同一个 manifest 读取与包内文件路径，不再有只能读 `SKILL.md` 的外部特例。
-/

/-!
#### 8-29-7 深层项目约定：普通路径成功读取时候选，按 run 有序递送一次

```rust
pub struct RuleCandidate { pub room: Address, pub text: String, pub dropped: u64 }
pub struct RuleContext { pub admitted: Address, pub landed: Address }
pub type RuleCandidates = Arc<dyn Fn(&Address, &Address, &BTreeSet<Address>, &mut dyn FnMut() -> bool) -> Vec<RuleCandidate> + Send + Sync>;
pub type ReadClock = Arc<dyn Fn() -> Result<TimeMs, AxError> + Send + Sync>;
impl ReadTool {
    pub fn ruling(self, candidates: RuleCandidates) -> ReadTool;
    pub fn recalling(self, shown: ReadShown) -> ReadTool;
    pub fn clocking(self, clock: ReadClock) -> ReadTool;
}
```

- **只有普通城市路径成功读取才取候选。** locator、catalog、skill、目录与失败读取不取。候选闭包只读，不标记递送；参数是受准地址与落点地址，两者都在 home building 内、受准文件目录是 run 房间严格后代时才非空。规则链按受准地址，覆盖 run 房间之下到文件目录的各级 `AGENTS.md`，顺序由 `city::room_agents_paths` 给出。邻楼和跨楼链接不生成附注。
- **候选不是窗口证据。** runtime 在装有候选闭包时只把内部 `rule_context: {admitted, landed}` 带上结果，不在并发 `invoke` 里读取规则正文；accounting 的 `Placing` 在串行 `account` 中取出并删除该字段，把 confirmed 与同波 pending 的集合交给候选闭包，让它在读取前就跳过已递送房间。定稿后先写进 `ToolBench.seen`，再交给回合渲染。最终 `rules` 只含 `{room, text}`；没有选择结果时整个字段缺席。`Duplicate` 因此仍逐字回答第一次定稿的结果，符合 §8-35。
- **预算与读取边界**：全部附注文本不超过 `INTERVAL_CAP_BYTES`。首个超限文件在 UTF-8 边界切下并带 `[truncated: N bytes]`，标记预留最坏宽度；切下的文件算递送，后续文件推迟到下一次 read；下一次发现会先跳过该房间，所以后续规则必定能被读到，不会反复停在同一个超限文件。每个候选文件经与冻结相同的读界读取，但 Absent、Withheld、其他错误与任何位置的非 UTF-8 都只跳过该文件，不让合法 read 失败。嵌套读取有界：最多保留 64 KiB 正文，继续读到 EOF 只验证 UTF-8 并累计省略量；候选正文总量同界，不把项目里的大规则文件整份读进内存。
- **递送状态**：confirmed 与同波 pending 共同去重；`complete_wave` 只把未被 exchange 裁切的 pending 并入 confirmed，被裁切的规则可在后续 read 重新递送。fork 从与继承消息相同的已提交波次继承 confirmed 集合（§8-2）， successor、delegate 子 run 从空集合开始。

#### 8-29-8 未变文件只追加 unchanged 桩（`tools::shown`，形状 1 判定）

```rust
pub struct Span { pub offset: u64, pub lines: u64 }       // 0 基下标与实际返回行数
pub struct Claim { /* addr、stamp、covered、total_lines、bytes、digest、generation —— 私有 */ }
pub struct ReadShown { /* Arc<Mutex<…>>；每个 run 一张，confirmed、staged、pending 分开 */ }
pub enum Recall { Fresh, Repeat(Answer), Compare { digest: B3Hash, answer: Answer } }
pub struct Answer { pub lines: u64, pub total_lines: u64 }
impl ReadShown {
    pub(crate) fn recall(&self, addr: &Address, now: &Stamp, asked: &str, want: Span) -> Recall;
    pub(crate) fn stage(&self, call: &str, claim: Claim);
    pub fn discard(&self, call: &str);                      // 丢弃失败或重试前的 staged
    pub fn admit(&self, call: &str);                        // staged → pending，只在有序 account；跨 crate 给 accounting
    pub fn confirm(&self, shortened: &BTreeSet<String>);    // pending → confirmed；清 staged；跨 crate 给 accounting
}
```

- **I1**：只有本 run 的窗口里存在一条完整、未裁切、覆盖本次请求且内容与当前文件相同的普通路径 read 结果，才回 `unchanged`。`seen` 回答 edit 能否落盘，`IdemKey` 回答同一次调用怎样重放，`shown` 回答窗口里有什么；三者互不写入。edit 不写 `shown`，落盘换文件身份使旧记录失效。
- **Staged → Pending → Confirmed**：工具成功读完后只 stage 私有 claim，不带进结果载荷或账本；`Placing::account` 仅在结果成功交付后按调用序 admit；失败与复用调用 ID 的新尝试先清除旧 staged；`complete_wave` 只确认未被 exchange 裁切的 pending，并清空未获准入的 staged。确认按接纳顺序应用，保留最新版本；正文阈值按本次请求的实际字节数判断，不累加重复确认。同一 wave 内重复读不看 confirmed 之外的新 claim，所以两条都回全文。`ahead` 后未放行的读不影响后续。
- **覆盖**：整文件未截断的读记 `Whole`；显式窗口只记实际交付区间，最多 8 段，超出丢最旧。未给 `offset`／`limit` 而被行数或字节上限截断的整读不 stage。窗口读 → 更大范围或整读覆盖不足时回全文；截断整读 → 再整读也回全文。空文件与越过末尾只在已知总行数且覆盖成立时回桩。`lines` 是本次请求实际覆盖的行数，`total_lines` 恒给出。
- **快路径**：参数先经 `Interval::asked_for` 验证，再走 canonicalize／读界／open／句柄复核；失败优先级中参数错误因此先于 IO。Windows、Linux 与 macOS 由 `Stamp::facts` 一次句柄查询同时给出文件类型、长度与戳。`Repeat` 不读文件字节，跳过 UTF-8 校验与 `cut`；`Compare` 仍读字节，只有前后戳相等且 BLAKE3 与已确认内容相同才回桩。其他平台仍走 `judged_read`，不发桩，因为没有同一个句柄快路径可挂。文件戳无法识别一个进程在非竞态窗口内做的同长度、同文件号且恢复旧 mtime 的原地改写；只有竞态claim持有内容摘要，因此这里不声称任意对抗性元数据伪装都能被发现。
- **racy 判定**：`Stamp` 不含内容。初次读完时若 `now - mtime < 2,000 ms`（mtime 在未来也算）或时钟不可用，claim 记录已读文本的 BLAKE3；之后同戳读到该记录必须走 `Compare`。无摘要的记录才可仅凭戳走 `Repeat`。时间由工作台注入的同一 clock 给出；runtime 不自行读系统钟，未注入或 clock 失败都不使合法 read 失败，而是保守地总是记录并比较摘要。常规本地文件系统上，普通写会改戳，racy 写由摘要兜底；网络卷与刻意伪造元数据不在保证范围内，绝不把该判定说成内容哈希的替代品。
- **成本阈值与有界状态**：`recall` 在覆盖区间与戳都匹配后，用 V4 的同一 `unchanged` 函数和已确认总行数算出准确桩长；只有先前正文至少为桩的两倍才去重，小文件仍回全文。`SHOWN_MAX_PATHS = 64`，`SHOWN_MAX_PENDING = 256`，按单调 generation 淘汰最旧，路径与 id 都用 `BTreeMap`，决策不读哈希序。表只存戳、区间与可选摘要，不存正文。fork、successor、delegate 子 run 从空表开始；fork 中继承的旧桩仍由 V4 重画，但不会凭空成为新 run 的窗口证据。
- **载荷与渲染**：`{path, unchanged: true, offset, lines, total_lines}`，不带 `text`、`bytes`，V4 画成一行未变桩（§8-63）。候选规则可在同一次桩上继续递送，规则是否完整进入窗口同样由压缩 id 决定。
-/

/-! D127 未变桩以完整窗口证据为准，规则递送与 read 去重共用压缩确认

**决定**：`ReadTool` 只产生文件事实、私有 claim 与普通路径的 `RuleContext`；`Placing` 在有序结算点先以 confirmed＋pending 过滤规则房间、再读取候选并定稿，随后 admit claim。`Exchange::compact` 报告裁切 id，run 在折入窗口前确认。规则最终载荷在 `IdemKey` 缓存之前定稿；unchanged claim 不进载荷。racy 文件按已读文本的 BLAKE3 比较；无法向打开句柄提问的平台不发桩。

**理由**：`invoke` 能并发和推测执行，账本存的是结构化答案，模型真正看到的是渲染后再经 exchange 压缩的文本。提前标记会把没放行的读或被裁掉的附注当成已交付；复用 `seen` 又会在 edit 后把新戳误当窗口里的旧内容。共用压缩 id 让规则继承与 unchanged 都遵守同一个「模型确实收到」定义。

**被否**：①只比较戳直接回桩——同长度、同 mtime 的原地改写会静默陈旧；②每次 read 都算哈希——为可能永不重复的文件付固定成本；③把窗口里旧结果改写成桩——破坏已发送前缀与缓存；④复用 `seen` 或 `IdemKey`——它们分别服务 edit 准入与同一次调用重放，不证明完整内容进入窗口；⑤在工具描述里劝模型别重读——既不强制也不省 token；⑥在 `invoke` 里预读全部候选——它既做推测 IO，又让首个超限房间在每轮候选发现中反复耗尽预算，后续规则永远到不了。

**重开参数**：网络卷必须提供同等保证时，改为每次内容比较或引入可验证的内容版本；快路径测量不能覆盖状态时，先保留全文路径。
-/

namespace Runtime.Tools.Read

/-- 请求或交付的一段行区间；`Whole` 另由 `Claim.whole` 表示。 -/
structure Span where
  offset : Nat
  lines : Nat
  /-- 仅无显式窗口且交付未截断整读时为真；lines 总是实际交付行数。 -/
  complete : Bool := false
  /-- 字节截断时实际交付的行数；缺省表示请求范围完整交付。 -/
  deliveredLines : Option Nat := none
  deriving DecidableEq, Repr

/-- 一次成功读产生的私有 claim。`digest = true` 表示同戳重读必须比较内容。 -/
structure Claim where
  content : Nat
  stamp : Nat
  cover : Span
  whole : Bool
  digest : Bool
  deriving DecidableEq, Repr

/-- 单个文件的窗口状态；`delivered` 只装未裁切确认过的 claim。 -/
structure World where
  currentContent : Nat
  currentStamp : Nat
  delivered : List Claim
  pending : List Claim
  staged : Option Claim
  deriving Repr

inductive Recall where
  | fresh
  | repeat
  | compare
  deriving DecidableEq, Repr

/-- 一次整读覆盖任意请求，窗口则只覆盖自己的闭区间。 -/
def covers (claim : Claim) (want : Span) : Bool :=
  claim.whole ||
    (claim.cover.offset ≤ want.offset ∧
      want.offset + want.lines ≤ claim.cover.offset + claim.cover.lines)

/-- 当前读产生的 claim；内容 digest 由 racy 判定决定是否必须比较。 -/
def claimOf (w : World) (want : Span) (racy : Bool) : Claim :=
  { content := w.currentContent, stamp := w.currentStamp,
    cover := { want with lines := want.deliveredLines.getD want.lines },
    whole := want.complete, digest := racy }

/-- 一次 read：confirmed 只读 `latest`，同波 staged/pending 不参与回答。 -/
def look (w : World) (want : Span) (racy : Bool) : World × Recall :=
  match w.delivered.head? with
  | some claim =>
    if claim.stamp = w.currentStamp ∧ covers claim want then
      (w, if claim.digest then .compare else .repeat)
    else
      ({ w with staged := some (claimOf w want racy) }, .fresh)
  | none => ({ w with staged := some (claimOf w want racy) }, .fresh)

/-- racy 路径的内容比较：只有内容与 confirmed 相同才回 `Repeat`。 -/
def compare (w : World) (want : Span) (racy sameContent : Bool) : World × Recall :=
  match w.delivered.head? with
  | some claim =>
    if claim.digest ∧ claim.stamp = w.currentStamp ∧ covers claim want ∧
        claim.content = w.currentContent ∧ sameContent then
      (w, .repeat)
    else
      ({ w with staged := some (claimOf w want racy) }, .fresh)
  | none => ({ w with staged := some (claimOf w want racy) }, .fresh)

inductive Step where
  | look (want : Span) (racy : Bool)
  | compare (want : Span) (racy sameContent : Bool)
  | account
  | settle (shortened : Bool)
  | write (content : Nat)
  deriving Repr

/-- 一步世界迁移；`account` 是有序结算，`settle false` 是未裁切确认。 -/
def step (w : World) : Step → World × Recall
  | .look want racy => look w want racy
  | .compare want racy sameContent => compare w want racy sameContent
  | .account =>
    ({ w with pending := w.staged.toList ++ w.pending, staged := none }, .fresh)
  | .settle shortened =>
    if shortened then
      ({ w with pending := [], staged := none }, .fresh)
    else
      ({ w with
        delivered := w.pending ++ w.delivered,
        pending := [],
        staged := none }, .fresh)
  | .write content =>
    ({ w with currentContent := content, currentStamp := w.currentStamp + 1 }, .fresh)

/-- 先跑完一段迹，再询问一次 read；迹中的 read 答案也可逐步观察。 -/
def run (w : World) (ts : List Step) : World :=
  ts.foldl (fun now event => (step now event).1) w

/-- 一个 claim 的戳不晚于当前戳；同戳时，它记录的就是当前内容。 -/
def Versioned (w : World) (claim : Claim) : Prop :=
  claim.stamp ≤ w.currentStamp ∧
    (claim.stamp = w.currentStamp → claim.content = w.currentContent)

/-- pending 与 delivered 都有版本证据；staged 只在读成功时出现；普通写递增戳。 -/
def Inv (w : World) : Prop :=
  (∀ claim, claim ∈ w.pending → Versioned w claim) ∧
    (∀ claim, claim ∈ w.delivered → Versioned w claim) ∧
    (∀ claim, w.staged = some claim → Versioned w claim)

/-- 空 run 没有任何窗口证据。 -/
def empty : World :=
  { currentContent := 0, currentStamp := 0, delivered := [], pending := [], staged := none }

/-- 空世界满足不变量。 -/
theorem empty_inv : Inv empty := by
  simp [Inv, Versioned, empty]

/-- 每一步保持证据链不变量；裁切只丢弃 staged/pending，不伪造 delivered。 -/
theorem step_keeps_inv (w : World) (s : Step) (h : Inv w) : Inv (step w s).1 := by
  cases s with
  | look want racy =>
    simp only [step]
    unfold look
    split
    · rename_i claim latest
      split
      · exact h
      · refine ⟨h.1, h.2.1, ?_⟩
        intro c staged
        injection staged with same
        subst c
        exact ⟨Nat.le_refl _, fun _ => rfl⟩
    · refine ⟨h.1, h.2.1, ?_⟩
      intro c staged
      injection staged with same
      subst c
      exact ⟨Nat.le_refl _, fun _ => rfl⟩
  | compare want racy sameContent =>
    simp only [step]
    unfold compare
    split
    · rename_i claim latest
      split
      · exact h
      · refine ⟨h.1, h.2.1, ?_⟩
        intro c staged
        injection staged with same
        subst c
        exact ⟨Nat.le_refl _, fun _ => rfl⟩
    · refine ⟨h.1, h.2.1, ?_⟩
      intro c staged
      injection staged with same
      subst c
      exact ⟨Nat.le_refl _, fun _ => rfl⟩
  | account =>
    simp only [step]
    refine ⟨?_, h.2.1, ?_⟩
    · intro claim mem
      simp at mem
      cases mem with
      | inl found => exact h.2.2 claim found
      | inr found => exact h.1 claim found
    · intro claim staged
      cases staged
  | settle shortened =>
    cases shortened with
    | true =>
      simp [step]
      refine ⟨?_, h.2.1, ?_⟩
      · intro claim mem
        cases mem
      · intro claim staged
        cases staged
    | false =>
      simp [step]
      refine ⟨?_, ?_, ?_⟩
      · intro claim mem
        cases mem
      · intro claim mem
        simp at mem
        cases mem with
        | inl found => exact h.1 claim found
        | inr found => exact h.2.1 claim found
      · intro claim staged
        cases staged
  | write content =>
    simp only [step]
    refine ⟨?_, ?_, ?_⟩
    · intro claim mem
      have old := h.1 claim mem
      have earlier := Nat.lt_of_le_of_lt old.1 (Nat.lt_succ_self _)
      exact ⟨Nat.le_of_lt earlier, by intro same; rw [same] at earlier; exact (Nat.lt_irrefl _ earlier).elim⟩
    · intro claim mem
      have old := h.2.1 claim mem
      have earlier := Nat.lt_of_le_of_lt old.1 (Nat.lt_succ_self _)
      exact ⟨Nat.le_of_lt earlier, by intro same; rw [same] at earlier; exact (Nat.lt_irrefl _ earlier).elim⟩
    · intro claim staged
      have old := h.2.2 claim staged
      have earlier := Nat.lt_of_le_of_lt old.1 (Nat.lt_succ_self _)
      exact ⟨Nat.le_of_lt earlier, by intro same; rw [same] at earlier; exact (Nat.lt_irrefl _ earlier).elim⟩

/-- 任意迹都保持证据链：confirmed 只能由未裁切的 pending 产生。 -/
theorem trace_keeps_inv (w : World) (ts : List Step) (h : Inv w) : Inv (run w ts) := by
  induction ts generalizing w with
  | nil => exact h
  | cons event rest ih => exact ih (step w event).1 (step_keeps_inv w event h)

/-- `Repeat` 必有未裁切的同内容、同戳、覆盖请求的完整窗口证据。 -/
theorem repeat_is_backed_by_unshortened_evidence (w : World) (ts : List Step)
    (want : Span) (racy : Bool) (h : Inv w)
    (answer : (look (run w ts) want racy).2 = .repeat) :
    ∃ claim, claim ∈ (run w ts).delivered ∧
      claim.content = (run w ts).currentContent ∧
      claim.stamp = (run w ts).currentStamp ∧ covers claim want := by
  have valid := trace_keeps_inv w ts h
  unfold look at answer
  split at answer
  · rename_i claim latest
    split at answer
    · rename_i matched
      have head : claim ∈ (run w ts).delivered.head? := by rw [latest]; exact rfl
      have delivered := List.mem_of_mem_head? head
      have version := valid.2.1 claim delivered
      exact ⟨claim, delivered, version.2 matched.1, matched.1, matched.2⟩
    · contradiction
  · contradiction

/-- racy 摘要不同就不能回 `Repeat`，而是产生新的全文 claim。 -/
theorem racy_mismatch_never_repeats (w : World) (want : Span) (racy : Bool)
    (answer : (compare w want racy false).2 = .repeat) : False := by
  cases latest : w.delivered.head? with
  | none => simp [compare, latest] at answer
  | some claim => simp [compare, latest] at answer

end Runtime.Tools.Read

/-! D9 城外 skill 在准入时复制进 CAS，run 只读 pin 的 manifest

**决定**：阅览室准入的一件城外书架上的 skill 经 `city::admit_external` 把完整包复制进本城 CAS，再经 `Catalog::admit_skill` 以同一个 `Expansion::Skill` 进入 catalog。`read <名>` 与包内文件都按 admitted manifest digest 完整校验读取（§8-29-3、§8-29-6）。

**理由**：城是可以整体复制的目录，run 不能依赖城外路径，也不能在外部文件变化后静默换版。逐文件加 manifest 的复制既保住整包边界，又让相同文件跨 skill 与版本去重；复制一次是每城最低存储量。

**被否**：①catalog 携扫描正文——只能覆盖 `SKILL.md`，包内文件没有同一 pin 的读取边界，正文还成为 CAS 之外的第二个权威；②让 `read` 在调用时读城外路径——读到的可能不是 pin 的版本，且城市复制后失效；③只复制 `SKILL.md`——包内脚本失去同一版本的读取与审核边界。

**重开参数**：城外书架由另一个独立账户提供不可变、可验证的 CAS 时，可以评估直接引用，但仍不得引用另一座城。
-/

/-! D113 模型选的路径按句柄判定，一次 open

**决定**：`read` 对模型选的城内路径只开一次文件：Windows 上开了再问句柄的真实位置并判它，Linux 上判过真实位置再开、以 `/proc/self/fd` 核句柄；之后从同一个句柄整读。取句柄真实位置用 `winx::file::get_file_path`：公开面安全，它与它用的 `windows-sys` 已经因 wasmtime 在锁文件里，不增加 crate，是平台调用阶梯的第一档。任何失败与目录、非 UTF-8 都退回 §8-29-4 的判定路径，拒绝的字节不变。

**理由**：同机测得 open+close 约 28 µs、canonicalize 约 78 µs（Windows 上过滤驱动让 open 很贵），判定路径一次成功读付 4 次 canonicalize、2 次 open 与 1 次元数据；agent 读的几乎都是热文件，这些固定开销比读本身贵。判句柄自己的位置还关掉了判定与打开之间的换链接窗口，而 `still_judged` 只能把它缩小到「来回换三次」。各候选在同一个 20 KiB 暖文件上逐轮交替 2000 轮（Windows x86_64 桌面机，16 核，旁边有构建在跑），p50 依次是：只 open 30–34 µs，一次 canonicalize 71–85 µs，open 加 `winx` 取位置 89–114 µs，open 加 `filepath` 123–146 µs，`cap-std` 在目录句柄下 open 83–101 µs，判定路径的整套解析与核验 406–479 µs。

**被否**：①直通 I/O 与按文件缓存的行索引、热读缓存、后台回填（曾作为一个 PR 提出）：直通读在暖文件上慢 4–5 倍、在刚写过的文件上慢约 14 倍，三层缓存带来一串一致性缺陷，截断读还要多一个模型回合；②`filepath 0.2`：同一个系统调用，交替测量里比 `winx` 慢约 30 µs，还要再引入一份 `windows 0.58`；③`desktop/ffi` 里的 Zig 函数：同一个系统调用，多一处 `unsafe`，阶梯上排在安全 crate 之后；④`cap-std`：把打开限制在一个目录句柄之内，但不回答文件实际在哪，链接落在城内另一栋楼时读界仍要按落点再判，替不了「句柄在哪」这一问；⑤`rustix::fs::getpath`：只在 Apple 平台上编译（`fcntl(F_GETPATH)`），Windows 与 Linux 用不上。

**重开参数**：`winx` 不再随 wasmtime 进锁文件，或某个候选在同机交替测量里比它快出测量误差之外。
-/

namespace Runtime.Tools.Read
/-- 首窗口不构成整读证据，不能覆盖后续未交付窗口。 -/
theorem first_window_does_not_cover_later (w : World) (racy : Bool) :
    covers (claimOf w { offset := 0, lines := 10 } racy)
      { offset := 10, lines := 10 } = false := by
  simp [covers, claimOf]
end Runtime.Tools.Read

namespace Runtime.Tools.Read
/-- 请求二十行但只交付十行，不能让二十行请求复用。 -/
theorem truncated_window_covers_only_delivery (w : World) (racy : Bool) :
    covers (claimOf w { offset := 0, lines := 20, deliveredLines := some 10 } racy)
      { offset := 0, lines := 20 } = false := by
  simp [covers, claimOf]
end Runtime.Tools.Read
