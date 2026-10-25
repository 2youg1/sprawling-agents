-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::views::document：文档的一版、它的窗口、它的预览与一段回复

规定 `crates/accounting/src/views/document.rs`、`crates/accounting/src/views/answering/range.rs`、`crates/accounting/src/views/answering/preview.rs` 与 `crates/accounting/src/views/answering/reply.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `views::document`、`views::answering` 旁的测试守住（`crates/accounting/Spec.lean` §16）。
-/

/-!
### 8-21 accounting::views::document 与 views::answering::range：文档的一版与它的窗口（形状 7 投影）

```rust
// accounting::views::document（锁外，`crates/sprawling/Spec.lean` §8-100）
pub(super) fn document_answer(city_root: &Path, at: Address) -> wire::DocumentAnswer;
pub(super) fn read_bytes(bytes: &[u8]) -> Reading;      // Content 与 Prefix 的文本判定，经 documents::Reading::of
// accounting::views::answering::range（锁外）
pub(in crate::views) fn range_answer(city_root: &Path, version: B3Hash, range: documents::Span) -> wire::Answer;
```

- **读盘只读一次，判定全在 `documents`。** `document_answer` 读文件的全部字节：读不到且是 `NotFound` 答 `Missing`，别的读错（目录、无权限）答 `Unreadable`，带系统的原话；零字节答 `Empty`；否则 `B3Hash::digest` 得版本，`Format::of_name` 读文件名，`Reading::of` 判文本，是文本就 `documents::head` 取第一个窗口（`crates/wire/Spec.lean` §8-69）。本模块不写任何一条判定，所以页面、`Content` 与 `Prefix` 对「这是不是文本」只有一个答案。
- **读到的每一版都进内容库**（`storage::Cas::put`，`crates/storage/Spec.lean` §8-36），然后才作答：文本与 `Opaque` 一样，空文件与整份已在答复里的小文件也一样（`crates/wire/spec/Answer/Document.lean` D8，D33）。之后的 `Query::Range`、`Query::Preview`、取字节、比较与导出读的是这一版。放不进去（盘满、目录不可写）答 `Unreadable`，原话是内容库的拒因：答一个之后读不到的版本等于许诺一件做不到的事。
- **读内容库只有一处：`range::stored`。** `Cas::size` 得这一版的长度，前三个字节经 `Encoding::of_mark` 得编码，`documents::lift` 说要抬起哪一段，`Cas::get_range` 读它；交回编码与抬起的字节。`range_answer` 拿它经 `documents::cut` 切出窗口，`preview_answer` 拿它经 `documents::preview` 读出块（§8-23）。内容库没有这一版、或切出的字节不是文本，答 `Unavailable { query: "Range(<version>)" }`。
- **`read_bytes` 留给 `Content` 与 `Prefix`。** 两者的答复形状不变（第一个窗口的文字、`truncated`、`binary`），判定是 `Reading::of`，切法是 `documents::cut`，窗口的上限因此是 `documents` 的 `WINDOW_BYTES_MAX`（64 KiB）：一个块在内容库里、又是城里的一份文件时，两处给同一个判断。
- 验收：`views::document::tests` 与 `views::answering::range::tests`，名字见 `crates/wire/Spec.lean` §8-69、§8-70。
-/

/-!
### 8-23 accounting::views::answering::preview：一个 Markdown 版本的一个窗口读成块（形状 7 投影）

```rust
// accounting::views::answering::preview（锁外，`crates/sprawling/Spec.lean` §8-100）
impl Views { pub(in crate::views) fn preview_ask(&self, version: B3Hash, viewport: documents::Span) -> Prepared; }
pub(in crate::views) fn preview_answer(city_root: &Path, version: B3Hash, viewport: documents::Span) -> wire::Answer;
```

- **读内容库，判定全在 `documents`。** 读内容库的那一步与 `Range` 是同一个函数（`range::stored`，§8-21）；然后 `documents::preview` 判编码、切窗口、止于块末、读出块（`crates/documents/Spec.lean` D20–D26）。本模块不写一条判定，所以预览、`Range` 与 `Document` 对「这一版是什么编码、窗口在哪里切」只有一个答案。
- **答复。** 读出来就是 `Answer::Preview`：`Laid` 带实际读的区间与块，UTF-16 的版本是 `Unsupported`（`crates/wire/Spec.lean` §8-74）。内容库没有这一版、打不开、或抬起的字节在这种编码下拼不出文本，答 `Unavailable { query: "Preview(<version>)" }`，与 `Range` 同一个口径：「我没能看」。
- **代价。** 只读内容库里这一窗要的那几个字节，加一次 comrak 读一个至多 64 KiB 的窗口；读数还没有，属 refrain 路线图 A11 的那一组。
- **Markdown 的版本都在内容库里。** 答 `Document` 的读面把读到的每一版放进内容库，Markdown 版本不论它的第一个窗口盖不盖得住整份都在其中（D33），所以从 `Document` 打开的任何一份 Markdown 文件都能按版本预览。
- 验收：`views::answering::preview::tests`——内容库里没有的版本答 `Unavailable`；超过一个窗口的 Markdown 文件经 `Document` 打开后，从 0 起按答复的 `span.end` 逐窗预览，每一窗止于块末，读到末尾时每一段恰好出现一次；整份放得下一个窗口的 Markdown 文件经 `Document` 打开后预览出它的块（`a_short_document_previews_by_its_version`）；字节不是文本的版本答 `Unavailable`；UTF-16 的版本答 `Preview::Unsupported`。
-/

/-!
### 8-29 accounting::views::answering::reply：一段回复读成块（形状 7 投影）

```rust
// accounting::views::answering::reply（锁外，`crates/sprawling/Spec.lean` §8-100）
pub(in crate::views) fn reply_answer(text: &str, state: documents::ReplyState) -> wire::Answer;
// Views::prepare 的一臂：Query::Reply { text, state } → Prepared::Reply { text, state }
```

- **判定全在 `documents::reply`**（`crates/documents/Spec.lean` D30、D31）：结算的回复照预览读，还在说的读到收束点。本模块不读盘、不读内容库、不读视图，只把答复拼成 `Answer::Reply`；读不出（文字里有 NUL）答 `Unavailable { query: "Reply" }`（`crates/wire/Spec.lean` §8-75）。
- **在视图锁外读**：`prepare` 只把文字拷进 `Prepared::Reply`，comrak 读一窗在锁放开之后做，理由同预览（D41）。
- **代价。** 一次收束点的扫描（与送来的文字同长，逐字节），加一次 comrak 读至多一个窗口；读数还没有，属 refrain 路线图 A11 的那一组。
- 验收：`views::answering::reply::tests`——一段带标题、列表、表、代码块与脚注的回复存成一个版本，经 `Query::Preview` 读出的 `Laid` 与经 `Query::Reply { state: Settled }` 读出的相等；同一段文字截在一个开着的段落里、以 `Streaming` 问，只答闭合的块，`span.end` 停在那一段之前；含 NUL 的文字答 `Unavailable`。
-/

/-! D33 读到的每一版都进内容库，由答 `Document` 的读面放进去

理由：之后的 `Range` 与 `Preview` 要读的是这一版，取字节、比较两版与导出也是，而版本的身份本来就是内容库的地址（documents D3），放进去之后按版本读就是按地址读对象，不需要第二个存放处；一份文档的版本列表要列得出城读过的每一版并能比较其中两版，所以小的纯文本版本与 `Opaque` 版本也存（`crates/wire/spec/Answer/Document.lean` D8）。内容库按内容去重，同一版第二次放入只是一次存在性判断。放进去的是读面而不是写者：这是一次查询的副作用，但它只添一个按内容寻址、重复放入即去重的对象，不改任何一条历史，写者也不知道哪个页面在读哪一版。被否决的做法：①`Range` 读文件此刻、版本不符就拒——居民在写的文件每几秒动一次，读到一半的页面要从头重读；②只存页面之后还会按版本来要的那几版（第一个窗口盖不住整份的文本与 Markdown）——一份小文件的旧版本、一份二进制文件的任何一版就读不回，页面列得出版本却比不了其中两版；③由写者在每次保存时存——城之外的写者（人的编辑器、居民的 `edit`）不经过写者；④小的 Markdown 版本让 `Preview` 读文件此刻、摘要相符才答——两条读法各判一次「这是不是那一版」，而文件在两次读之间可能被改。代价是每读到一个新版本付一份拷贝。重开参数：内容库长出回收时，被回收的版本要有自己的答复；一座城的内容库因为反复读一份大的二进制文件而长到人在意的大小时，按格式或大小限定哪些版本存（与 wire D8 的重开参数同一条）。
-/

/-! D35 预览由读面按版本从内容库读，判定全在 `documents`，读出的块不缓存

理由：版本就是内容库的地址（documents D3），按版本读与 `Range` 同一条路，读面在视图锁外读，worker 不答查询；一个版本的字节不变，预览也就不会过时，页面不因任何事件重问。读出的块不进视图：它与一个窗口的大小同阶，而视图里的每一样东西都要随快照编码、随每一次重建折叠。被否决的做法：①按地址读文件此刻——与 D33 的①同一个缺陷；②把读出的块按版本存在视图里——第二份拷贝要随快照走，而一次读出只是一窗的 comrak；③由页面先 `Range` 再把文字送回城里读——文字在线上走两趟。重开参数：在固定语料上量出一窗的读出超过毫秒级（D41），或同一版本被许多页面同时预览成为常态时，按版本缓存。
-/

/-! D41 回复与预览在视图锁外读，读内容库只有一处

**(a) 回复在视图锁外读，读出的块不缓存。** `prepare` 只把文字拷进 `Prepared::Reply`，`documents::reply` 在锁放开之后读（§8-29）。理由：读一窗是一次 comrak，与预览同阶，锁里多一次它，等着折叠的每一行都多等一次；同一段文字恒读出同一棵树，页面自己留着答复就够了。被否决的做法：①在 `prepare` 里当场作答——它不读盘，看上去是「视图里就答得了」的一类，但它的代价随文字长短走，不随视图走；②按文字的摘要缓存读出的块——一份要随快照编码的拷贝，换来的只是页面重问同一段文字时省一次 comrak，而页面不重问。**(b) 读内容库只有 `range::stored` 一处**，`Range` 与 `Preview` 都经它拿到编码与抬起的字节（§8-21、§8-23）。理由：打开、长度、标记、抬起这四步是「按版本读一个窗口」的前一半，两条查询的差别只在后一半（切还是读成块）；四步写两遍，改其中一处（例如内容库换了读法）时另一处不会跟着改。放在 `range` 而不另开模块，是因为 `range` 本来就拥有「按版本读一个窗口」，`preview` 是它的第二个读者。被否决的做法：①`preview` 先答一个 `Range` 再拿窗口的文字读——读法相同，却要把 `Window` 的文本再交回 `documents`，而 `documents::preview` 要的是抬起的字节，好在窗口之外判块末；②新开一个只有这一个函数的模块——两个读者都在 `answering` 里，一个函数不值得一个模块名。
-/

/-!
### 8-38 下一个 run 的提示层（`accounting::prompt_chain`，形状 1 判定；`accounting::views::layers`，形状 7 投影）

```rust
// accounting::prompt_chain（冻结与视图共用）
pub(crate) struct Convention { pub(crate) path: PathBuf, pub(crate) heading: String, pub(crate) whose: &'static str }
pub(crate) fn building_conventions(city_root: &Path, building: &Address) -> Vec<Convention>;   // 工作区、楼根
pub(crate) fn room_conventions(city_root: &Path, addr: &Address, building: &Address) -> Result<Vec<Convention>, AxError>;
pub(crate) enum Reading { Sent(Vec<u8>), Absent, Withheld(String) }
pub(crate) fn read_convention(city_root: &Path, building: &Address, path: &Path) -> Result<Reading, AxError>;
pub(crate) fn read_bound(city_root: &Path, home: Address) -> runtime::ReadBound;               // 工作台的读界也取它
// accounting::views::layers（锁外）
pub(super) fn layers_answer(city_root: &Path, addr: Address) -> wire::Answer;
```

- **`AGENTS.md` 经读界读，与 `read` 工具同一个判定。** `read_convention` 先看这一项本身（`symlink_metadata`）：不在答 `Absent`；是目录答 `Withheld`。否则经 `runtime::BoundReader::open` 读，读界以这栋楼为家（`read_bound`）：链接指向城外、机密楼或保留子树时 `BoundReader` 拒（`E_GATE_DENIED`），这里答 `Withheld` 并带拒因；`E_INVALID_ARGS`（链接落空、指向目录）同样答 `Withheld`。只有在那里、却打不开的文件（`E_STORAGE_FATAL`）仍拒掉冻结，因为那是这个项目写下、而 run 收不到的约定。理由：一个 run 能在房间里建链接（例如在主机上跑的 `exec`），而前缀会被送给 provider；用 `std::fs::read` 读就是绕过读界的第二条读路。一个叫 `AGENTS.md` 的目录在 Windows 上读出 `PermissionDenied`，以前会让这个房间的每个 run 都开不起来。
- **冻结与视图读同一个函数。** `worker::freezing` 冻 Building 段与 Resident 段时、`views::layers` 答 `Query::Layers` 时都调 `prompt_chain`，所以页面说「送」的就是下一个 run 收到的，说「不送」时给的原因就是冻结跳过它的原因。工作台给 run 的读界也从 `read_bound` 取，读界只有一处定义。
- **`Query::Layers` 的答复**（`crates/wire/Spec.lean` §8-98）在锁外读盘：City 段经 `city::city_prompt_path`（楼的那份在时城根那份是 `Replaced`）；Building 段与 Resident 段的 `AGENTS.md` 经 `read_convention`；身份文件是 `city::urbanite_path`，City Hall 的两个座位因此落到 `MAYOR.md` 与 `CLERK.md`；Run 段是 `city::model_notes` 列出的城与楼两层的每一份，楼里有同一个模型的一份时城那份是 `Replaced`。带默认的三份经 `city::standing` 判副本，`ShippedDefault.version` 是默认文本的哈希。地址拼不出一栋楼（保留子树）答 `Unavailable`。
- **默认文本是一版。** `Query::Versions` 问城根的 `City.md`、`MAYOR.md` 或 `CLERK.md` 时，答复在末尾多一版 `VersionSource::Shipped`；答之前把默认文本放进内容库（`keep`，已在时只是一次存在检查），放进去了 `kept` 为真，RefRain 的比较与导出读得到它；内容库不收（盘满、目录不可写）时这一版照样列出，`kept` 为假，与任何没存下的一版一样，页面不提供比较。它排在截断之后加上，`VERSIONS_MAX` 不会把它截掉。
- 验收：`views::layers::tests`、`worker::freezing::tests::room_chain` 里一个叫 `AGENTS.md` 的目录与一个指向城外的链接不进前缀且冻结照常、`views::versions::tests` 里城根 `City.md` 的末一版是 `Shipped`。
-/

