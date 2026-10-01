-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.documents.spec.Edit
import crates.documents.spec.Encoding
import crates.documents.spec.Proposal
import crates.documents.spec.Window

/-! # documents 的规格

`documents` 是一个人经页面读、改的文档的规则：它是哪一版、它的字节拼出哪些字符、它的块在哪里、一次答复带它多少、一次保存改了什么、一处修改提案怎样读成一张卡、怎样按人的决定落下。本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `documents D<n>`。
-/

/-! ## 1 需求分解

来源是 refrain 路线图 §4-8（S7.8）、§4-10（S7.10）与 Roadmap U8、U10 的 Rust 一半。八个可独立验收的单元，与模块一一对应：

- 一段字节（`span`）：版本里的一段半开字节区间（D2）。
- 编码（`encoding`，`spec/Encoding.lean`）：一个版本的字节是哪种编码的文本，或者不是文本（D4、D5）。
- 格式（`format`）：文档按哪种文法分块，由文件名定（D6）。
- 块（`layout`）：块在字节里的位置，迁自 RefRain `source_layout`（D6）。
- 窗口（`window`，`spec/Window.lean`）：一次答复带一个版本的哪一段，在哪里切（D7）。
- 编辑事务（`edit`，`spec/Edit.lean`）：一次保存是对一个基线版本的一串编辑，落下就交回撤销它的事务（D8、D9）；页面送来的是文本编辑，按那一版的编码写回（D11、D12）。
- 选区（`selection`）：一个读者的位置怎样跨过一次保存（D10）。
- 修改提案（`proposal`，`spec/Proposal.lean`）：一次 run 对一份文档一段字节的建议，它的身份、逐句的 diff、按逐句的决定合成，以及几张卡怎样作为一次保存落下（D13–D19），迁自 RefRain 的 `review` 与 `decision`。

本 crate 回答「这些字节是什么、能怎样切、能怎样改、一处建议怎样读、怎样决定」；它不读盘、不读内容库、不记账。
-/

/-! ## 2 验收标准

分部里的定理是模型对性质的证明：编辑之外的字节原样留下、撤销还原基线、同一基线的第二次保存被拒（`spec/Edit.lean`）；整张拒绝合成原文、整张接受合成提议、一张卡只被处理一次（`spec/Proposal.lean`）；窗口不劈开字符、不过版本末尾、不过请求起点之后 `WINDOW_BYTES_MAX` 字节、有边界就有进展（`spec/Window.lean`）；带标记的 UTF-16 是文本、无标记而含 NUL 的不是、判成文本的拼得出（`spec/Encoding.lean`）。每个分部各有一条「拿掉守卫即反例」的定理。

refrain 路线图 §4-8、§4-10 的验收在 Rust 测试里成立，测试走生产入口：

| 验收 | 完成的定义 |
|---|---|
| A1 字节不变 | `edit::tests::every_byte_outside_the_edits_is_copied_through`：带 BOM、混合换行、尾随空格、非 UTF-8 的四份样本，编辑之外的每个字节原样留下；`window::tests::windows_read_end_to_end_give_back_every_byte`：逐窗读完一个版本，窗口首尾相接就是这个版本，逐字节；`layout::tests` 的 RefRain 回归语料：块与空隙首尾相接就是源文 |
| A3 后到者被拒 | `edit::tests::a_second_save_from_the_same_version_is_refused`：两个写者从同一版本出发，先到的落下，后到的得到 `E_VERSION_CONFLICT`，先到者的字节不动；线上那一半是 accounting 的 `a_second_save_from_the_same_version_is_refused_and_the_first_stands`（wire-SPEC §8-72） |
| 文本编辑写回字节 | `edit::tests::a_text_save_writes_the_version_s_own_encoding`：UTF-16 与带标记的 UTF-8 文档上的文本编辑按那一版的编码写回；`edit::tests::a_save_that_splits_a_character_is_refused` |
| A10 修改提案 | `proposal::tests`：整张接受得到提议、改后接受得到人改过的句子、整张拒绝不写；过期的提案被拒、拒绝它却可以；两张重叠的卡不能一起接受；切出的句子接起来是两边的原文；accounting 的 `commanding::saving::tests` 走线上那一半（收回、重复请求、重开之后从账本折回） |
-/

/-! ## 3 假设与歧义

- **假设**：BLAKE3 抗碰撞。版本身份是整份字节的摘要（D3），「摘要相同即同一版本」靠它；`spec/Edit.lean` 把它写成 `later_is_refused` 的假设 `inj`，不证明它。
- **假设**：没有标记的 UTF-16 不出现在城里需要读成文本的地方。它被判成不透明的而不是被猜（D4）；人的文件里真有这样的一份时，页面说它不是文本，而不说错字。
- **假设**：对齐的结果按种类过滤之后恰是两边各自切出的句子。`spec/Proposal.lean` 把对齐的结果当作给定，证明的是合成；对齐本身由 `proposal::tests` 的语料断言（D15）。
- **当前状态：提出与收回提案的写者还没有。** `proposal_offered` 与 `proposal_withdrawn` 由提出它的 run 的工具写下（kernel-SPEC §8-83），那件工具还没有落地（city-SPEC §8-40）；在那之前 `Offer::of` 的生产调用方是读账本的折叠（accounting-SPEC §8-22），它对每一行 `proposal_offered` 算出提案身份并判它的区间与长度。工具落地时要回答的：一次 run 在哪一版上提（它读到的版本，还是它的候选工作树里那份文件的基线），以及 refrain 路线图 §4-10 说的「审阅期间 `edit`、`exec` 对该文稿的写入受同一边界约束」是否由 runtime 判。决定它的证据是那件工具的规格。
-/

/-! ## 4 现状分析

本 crate 之前，`Query::Document` 只给文件头 64 KiB、`from_utf8_lossy` 有损解码、头 8 KiB 里有 NUL 就判二进制（refrain 路线图附录 C）：UTF-16 文件被判成二进制，非 UTF-8 的字节被替换字符盖掉，而答复里没有版本，页面无法说「我改的是哪一版」。现在 `accounting::views::document` 用本 crate 的 `Reading::of`、`head` 作答，`accounting::views::answering::range` 用 `lift`、`cut` 按版本答范围（accounting-SPEC §8-21，wire-SPEC §8-69、§8-70）；`Content` 与 `Prefix` 的文本判定经同一个 `Reading::of`。

**迁入的部分。** RefRain（另一个仓库，`refrain-core` crate 的 `source_layout` 模块）的两条分块规则（Markdown 与纯文本）与它的回归语料：二十份文本按 RefRain 生成语料的脚本逐字搬进 `layout::tests`，每一份的字节与 RefRain 语料清单记的 SHA-256 核过，块区间与它冻结的块表逐项相同。RefRain 的 `DocumentSurface`（`refrain-app` crate 的 `native_document` 模块）里与平台无关的部分迁的是有界投影的做法：按块取一屏、取不下一块时退到字符边界（`window::head`）。没有迁的：`DocumentSurface` 的输入法组合、光标移动与撤销栈——草稿只在浏览器（refrain 路线图 §4-8），服务端没有一个正在编辑的文档；迁入时去掉了直接索引、`expect`、`as` 与内部随机 ID。

**修改提案迁自 RefRain `refrain-core` 的 `manuscript::review` 与 `manuscript::decision`**：提案冻在一个基线上、句子切片（终止符与紧随的闭合符号）、逐句的判词（接受、改后接受，改后接受只对插入的句子）、批量决定的过期与重叠拒绝。改动的：身份由内容摘要算出而不是随机 `Id`（D13）；对齐的键是整句的字节而不只是去掉空白的句子（D14）；只迁对齐表的那一半，不迁锚点分段（D15）；作用域是一段字节而不是一串块 id（D17）；`CommentOnly` 与「格式改动」的分类没有迁，卡上没有它们的读者。
-/

/-! ## 5 权威信源

| 事实 | 出处 |
|---|---|
| UTF-8 的合法性 | Unicode Standard §3.9、RFC 3629；Rust `std::str::from_utf8` |
| UTF-16 的代理对 | Unicode Standard §3.9；Rust `char::decode_utf16` |
| 字节顺序标记 | Unicode Standard §23.8（U+FEFF） |
| 代码块的界定行 | CommonMark 0.31.2 §4.5（fenced code blocks：至少三个同一字符、缩进少于四列、闭合行同字符且不短于开启行） |
| 版本摘要 | `kernel::B3Hash::digest`，与 `storage::Cas::put` 同一个 BLAKE3 |
-/

/-! ## 6 命名统一

**document version**、**document window**、**draft** 取自 `docs/glossary.md`。「版本」在本 crate 恒指一份文档某一刻的全部字节，它的身份是 `B3Hash`；「窗口」恒指一次答复带的那一段；「草稿」是浏览器里还没保存的文本，本 crate 里没有它。模型里的声明名取同样的词：`Documents.Edit.apply`、`Documents.Window.stepBack`、`Documents.Encoding.reading`。**proposal**（修改提案）与 **review slice**（卡上的一句）同样取自 glossary：`Documents.Proposal.merged` 与 Rust 的 `Review::merged`、`Slice` 是同一组词。
-/

/-! ## 7 模块边界

- **字节从哪里来**归调用方：`accounting::views` 读盘与内容库，把切片交进来；本 crate 恒不持文件句柄。
- **一版字节存到哪**归 `storage::cas`：版本身份就是内容库的地址（D3），所以按版本取范围就是按地址读对象（storage-SPEC §8-36）。
- **一次保存怎样落盘、怎样记账**归 `city::document`（锁、读当前字节、整份换上，city-SPEC §8-40）与写者 `accounting::worker::commanding::saving`（wire-SPEC §8-72、§8-73）。
- **一张卡开着、决定过还是收回过**归读账本的折叠（accounting-SPEC §8-22）：本 crate 给出每一步的判定，不持有任何一张卡。
- **线上的拼写**归 `wire`：它直接携带本 crate 的 `Span`、`Format`、`Encoding`、`Window`（D1）。

约束：本 crate 只依赖 `kernel`、`serde` 与可选的 `schemars`；恒不出现 I/O、时钟与全局状态。
-/

/-! ## 8 接口先行

签名的权威是 Rust 源码；这里记每个模块的形状与它为什么是这个形状。

- **`span`**（形状 2 值类型）：`Span::new(start, end)`（`start > end` 即 `E_INVALID_ARGS`）、`Span::at`、`start`、`end`、`len`、`is_empty`。线上经 `try_from` 读回同一个 `new`。
- **`encoding`**（形状 1 判定）：`Encoding::{Utf8, Utf8Bom, Utf16Le, Utf16Be}`、`Reading::{Text(Encoding), Opaque}`、`Reading::of(&[u8])`、`Encoding::of_mark(&[u8])`、`Encoding::decode(&[u8]) -> Result<String, AxError>`（`spec/Encoding.lean`）。
- **`format`**（形状 1 判定）：`Format::{Markdown, Plain}`、`Format::of_name(&str)`。
- **`layout`**（形状 1 判定，crate 内）：`blocks(Format, &[u8]) -> Vec<Span>`。
- **`window`**（形状 1 判定）：`WINDOW_BYTES_MAX`、`Window { span, text }`、`Lifted { at, bytes, size }`、`head(Format, Encoding, &[u8])`、`lift(Span, size) -> Span`、`cut(Encoding, Lifted, Span) -> Result<Window, AxError>`（`spec/Window.lean`）。
- **`edit`**（形状 2 值类型 ＋ 形状 1 判定）：`Edit { span, bytes }`、`Transaction::new(baseline, edits)`、`Transaction::apply(&[u8]) -> Result<Applied, AxError>`、`Applied::{bytes, version, undo}`（`spec/Edit.lean`）；`TextEdit { span, text }`（线上携带）与 `save(source, baseline, &[TextEdit]) -> Result<Applied, AxError>`（D11、D12）。
- **`proposal`**（形状 2 值类型 ＋ 形状 1 判定）：`Offer::of(RunId, &ProposalOffered) -> Result<Offer, AxError>`、`Offer::{id, run, doc, baseline, span, before, review}`；`Review::of(before, after)`、`Review::{slices, merged(&[SliceVerdict]) -> Result<String, AxError>}`；`Slice { kind, text, lead, trail }`、`SliceKind::{Same, Delete, Insert}`（线上携带）；`decide(source, &[(&Offer, &[SliceVerdict])]) -> Result<Option<Applied>, AxError>`（`spec/Proposal.lean`）。判词的类型 `SliceVerdict`、`Verdict` 住 kernel（kernel-SPEC §8-83），因为账本记的就是它。
- **`selection`**（形状 2 值类型）：`Selection { anchor, focus }`、`collapsed`、`span`、`mapped(&Transaction)`。
-/

/-! ## 9 工作流程

读：调用方读一份文件的全部字节 → `B3Hash::digest` 得版本 → `Format::of_name` → `Reading::of`；是文本就 `head` 取第一个窗口，窗口没到末尾时调用方把这一版存进内容库。之后每一次按版本取范围：调用方从内容库读这一版的长度与前三个字节（`Encoding::of_mark`），`lift` 说要抬起哪几个字节，`cut` 切出窗口。

写（wire-SPEC §8-72）：调用方在文档锁里读当前字节 → `save` 判基线、判编码、把文本编辑写成那一版编码的字节、经 `Transaction` 落下、判结果仍是同一种编码的文本 → 调用方整份替换 → 记账，回执带 `Applied::version`。

提案（wire-SPEC §8-73）：读账本的折叠对每一行 `proposal_offered` 调 `Offer::of`，得到身份与卡；页面要卡时 `Offer::review` 切句、对齐；人决定时调用方在文档锁里读当前字节，`decide` 对每张卡 `merged` 出替换文本，接受了什么的卡判基线与原文，几段合成一次 `save`。
-/

/-! ## 10 实现逻辑

D1 线上直接携带本 crate 的类型。`wire` 的文档答复里有 `Span`、`Format`、`Encoding`、`Window`；它们在这里定义一次，带 `serde` 与可选的 `schemars` 派生，`wire` 依赖本 crate。被否决的是 `wire` 自己再拼一份同名的枚举、由 `accounting` 逐臂换算：两份拼法靠一个 `match` 连着，新增一种编码时线上那份要另记得加。代价：本 crate 多依赖 `serde` 与可选的 `schemars`，与 `kernel` 同形。

D2 偏移是从版本第一个字节数起的半开字节区间（`Span`），不是 `Locator` 的闭区间（`kernel::Range`）。闭区间拼不出空区间，而一个插入点、一份空文档都是空区间；按字节而不按字符，是因为字节是唯一不随编码变的坐标，换算成解码文本与 UTF-16 码元归页面的一处（refrain 路线图 §4-8 的 `core/document_pos.ts`）。

D3 版本身份是整份字节的 `B3Hash::digest`，与内容库给同一份字节的地址相同。被否决的：修改时间加大小（同一秒里改两次、或改后同长，就是同一个「版本」）；城自己数的版本号（城之外的写者——人的编辑器、居民的 `edit`——不经过它）。代价：每读一次全文算一次 BLAKE3，按 GB/s 计，与读盘同阶。

D4 编码按字节顺序标记判，没有标记就是「不含 NUL 的合法 UTF-8」或不透明（`spec/Encoding.lean`）。被否决的：凭头 8 KiB 里的 NUL 判二进制——把 UTF-16 判成二进制；按统计猜编码——猜错时给人看文件里没有的字符；有损解码——替换字符会在下一次保存时写回文件。没有标记的 UTF-16 与旧式八位编码因此是不透明的，答复里只有版本与大小。重开参数：人要在页面上选一种编码读一份不透明的文件（refrain 路线图 A5「编码选择作用于全文」），那时编码成为请求的一部分。

D5 窗口从第 0 个字节数起，标记是文本的第一个字符（U+FEFF）。于是一个窗口的文本按它的编码写回去，恰好是它那一段字节，页面拼回整份文本时不必知道有没有标记；被否决的是把标记剥掉、让第一个窗口从标记之后开始：那样文本与字节的对应要多一条只对第一个窗口成立的规则。

D6 格式由文件名定，`.md` 与 `.markdown`（不分大小写）是 Markdown，其余是纯文本；块按 RefRain `source_layout` 的两条规则读：Markdown 以空行分块、代码块的界定行之间的空行不分块，纯文本一行一块。只迁规则不迁摘要：RefRain 在区间旁存一份摘要，防一份块表去切别的字节；这里块表在同一次调用里读出、用完，不比它读的字节活得久。块只在 UTF-8 的两种编码里读，因为规则找的是 `\n`、`\r` 这几个字节，UTF-16 里它们旁边还有一个字节。

D7 一个窗口至多 `WINDOW_BYTES_MAX`（64 KiB）字节，从请求的起点算；终点往回退到字符边界，起点在字符中间时退到那个字符的第一个字节（`spec/Window.lean`）。第一个窗口（`head`）在整份放得下时就是整份，放不下时止于放得下的最后一个块的末尾，一块都放不下时止于界内最后一个字符边界。往回退而不往前进：往前进会让窗口比界长。64 KiB 是旧答复的界，留着它让页面的第一次答复花的与以前相同；它不是从测量得来的数，refrain 路线图附录 G 说的也是这件事。重开参数：S7.8 的 A11 在固定语料上量出的首个可读视口时间。

D8 一次保存是对一个基线版本的一串编辑：按文档序、互不重叠（两个插入点可以重合，按给出的顺序），任何一段越过末尾或与前一段重叠都整次拒绝（`E_INVALID_ARGS`）；源文的摘要不是基线时整次拒绝（`E_VERSION_CONFLICT`），先到的写者的改动留着（A3，`spec/Edit.lean`）。被否决的：按到达顺序一段一段地落、每段用前一段落下之后的坐标——同一串编辑在两个读者那里会读出两种结果；与 `city::edit_against` 一样用整份正文作基线——整份正文要随请求往返一次，而版本摘要是 32 字节。

D9 撤销是落下时交回的逆事务，它的基线是新版本，作用上去得到原来那一版（`undo_restores`）。服务端不持撤销栈：草稿只在浏览器，浏览器的编辑器自己撤销还没保存的改动；已经保存的改动要收回时，逆事务和任何一次保存一样带基线，所以别人在那之后又存过时，收回被拒而不是盖掉别人。

D10 选区跨过一次保存：编辑之前的位置不动（恰在插入点上的也不动），之后的位置按增删平移，落在被替换段里的位置移到替换内容的末尾，因为它指着的字节已经没有了。被否决的是把落在被替换段里的位置移到段首：读者正在看的那段文字被换掉之后，光标停在新文字之后是编辑器的通行做法（CodeMirror 6 的默认映射）。

D11 页面送来的是文本编辑（`TextEdit { span, text }`）：区间是基线那一版的字节，替换的内容是文本，`save` 按那一版的编码（`Reading::of` 判出的那一种）把文本写成字节。四种编码都能写出任何字符，所以写不会失败；带标记的版本里，文本里的 U+FEFF 写回成那个标记（D5）。改完的字节必须仍是同一种编码的文本，否则整次拒绝（`E_INVALID_ARGS`）：一个区间从字符中间起止就会劈开那个字符，没有标记的 UTF-8 里写进 NUL 就不再是文本，删掉标记就换了编码——三者都会让下一次打开读出另一份文件。被否决的：①页面送字节——页面要自己按 UTF-16 或带标记的 UTF-8 编码，那是编码规则的第二个家；②改完不判——一份被劈开的文件在下一次打开时变成不透明的，而人的草稿已经丢了；③允许换编码——换编码不是一段一段的编辑，是另存，重开参数是 refrain 路线图 A5 的「编码选择作用于全文」。

D12 `save` 的判定次序固定：先判源文是不是基线（`E_VERSION_CONFLICT`），再判它是不是文本、编辑是否成形、改完是否仍是同一种编码的文本（`E_INVALID_ARGS`）。先判基线，是因为一份已经被别人改过的文件，它别的毛病不是这次保存要说的：页面要做的是重读。不透明的版本不能经文本编辑保存，因为它没有一种编码可以把文本写回去。空字节是 UTF-8 的文本，所以一份空文件或一份还不存在的文件（调用方读成空字节）可以经一次保存写出第一版。

D13 提案的身份是 `B3Hash::digest`：前缀 `PROPOSAL_ID_TAG`，然后依次是提出它的 run 的十六个字节、文档地址、基线、区间的起止（大端八字节）、原文与提议（各带长度）。由内容算出，所以重开的城读账本就得到同一个身份，账本行里不必记它；同一个 run 对同一版的同一段提同一句话是同一个提案，重复写一行不会多出一张卡。被否决的：①RefRain 的随机 `Id`——城里不生成随机值，重放也不会得到同一个；②`proposal_offered` 那一行的 `seq`——写者在落账之前就要把身份交给 run，而 `seq` 在落账之后才有；③身份不含 run——两个 run 恰好提同一句话时，收回其中一个会收回另一个的卡。

D14 句子按 RefRain 的终止符（`。！？…!?.`）切，终止符之后紧跟的终止符与闭合符号（引号、括号、书名号）算同一句。每一句带着它前面的空白（`lead`）与句子本身（`text`），最后一句还带着它之后的空白（`trail`），所以把一边的句子按序接起来恰是那一边的原文，一边只有空白时它是一句 `text` 为空的句子。对齐的键是整句的字节（`lead`、`text`、`trail` 一起）：只在空白上不同的两句是一删一插，于是整张接受合成的恰是提议、整张拒绝合成的恰是原文（`spec/Proposal.lean` 的 `acceptAll_is_after`、`rejectAll_is_before`）。被否决的是 RefRain 的键（去掉空白的句子）：一处只改了换行或空格的提议会对齐成 `Same`，卡上看不见它，接受之后文件里也没有它（`looseKey_loses_the_after_side`）。

D15 对齐先剥掉两边相同的头与尾，中间一段用最长公共子序列；表的格数超过 `ALIGN_CELLS_MAX`（`2^22`，`u32` 一格，16 MiB）时，中间一段整段删除再整段插入。结果仍正确（D14 的两条性质不依赖对齐得多好），只是卡上的改动更粗。没有迁 RefRain 的锚点分段（`align::segment`）：它是为十万句的整部稿子写的，而一处提案两边各至多 `WINDOW_BYTES_MAX` 字节（D18），一个只改了一句的提案在剥头剥尾之后中间只剩一两句。重开参数：提案的上限变大，或者实测有一张卡的对齐在答复里成为看得见的时间。

D16 判词逐句：只有改动过的句子（`Delete` 或 `Insert`）可以被点名；`Accept` 对两种都成立（删除的句子不再留下，插入的句子进来），`Amend { text }` 只对插入的句子成立，它换掉那一句的 `text`、留着它的空白。没被点名的改动算拒绝：删除的句子留着，插入的句子不进来，所以空的判词就是整张拒绝。点名一个不存在的序号、一句没改动的句子、同一句两次、对删除的句子 `Amend`，都整次拒绝（`E_INVALID_ARGS`）。被否决的：①一张卡只有接受与拒绝两个判词——人常常要的是「这句对、那句不要」；②RefRain 的 `Reject` 与 `CommentOnly` 判词——显式的拒绝与不点名是同一个决定的两种拼法，而评论在卡上没有读者；③允许对删除的句子改后接受——那是在原文里插入一句新话，是另一处提案。

D17 几张卡可以一起决定，接受了什么的卡合成一次保存（`decide`）。这几张卡必须都基于源文这一版（否则 `E_VERSION_CONFLICT`，什么都不落，卡都还开着）；每张卡的原文必须就是源文那一段（否则 `E_INVALID_ARGS`：账本里的这一行与它的基线对不上）；它们的区间互不重叠，同一张卡不出现两次（`E_INVALID_ARGS`）；合成的编辑交给 `save`，所以重叠、越界与编码的判定与一次普通的保存是同一处。整张拒绝的卡不判基线：拒绝一张过期的卡什么都不写。被否决的：一张一张决定——同一版上的第二张卡在第一张落下之后必然过期，人只能接受一张。

D18 一处提案的原文与提议各至多 `WINDOW_BYTES_MAX` 字节，`Offer::of` 拒绝更长的（`E_INVALID_ARGS`）。一张卡是一屏读得完的一段，这个上限让一张卡的答复与一个窗口同阶，也让对齐表有界（D15）；更大的改动是一次保存，或者几处提案。重开参数：refrain 路线图 S7.10 的验收要求整章重写成一张卡时。

D19 一张卡只被处理一次：开着的卡可以被人决定或被提出它的 run 收回，决定过或收回过的卡再被决定或收回都被拒（`E_INVALID_ARGS`），状态不变（`spec/Proposal.lean` 的 `decided_is_final`、`withdrawn_is_final`）。状态由读账本的折叠持有（accounting-SPEC §8-22），本 crate 只给出判定所需的身份与合成。被否决的是「决定之后可以再改判」：一次决定落下的字节已经成了文档的一版，再改判就是一次新的保存，它该带新的基线。
-/

/-! ## 11 边界枚举

空文档（一个空的纯文本行、零个 Markdown 块，版本是空字节的摘要）；只有标记的文档；没有结尾换行；混合 `\r\n` 与 `\n`；尾随空格与制表符；四列缩进的界定行（不是界定行）；没闭合的代码块（直到末尾都是一块）；非 UTF-8 字节；没有标记而含 NUL；落单的代理；奇数长度的 UTF-16；起点在字符中间、在标记中间、在版本之后；请求比 `WINDOW_BYTES_MAX` 长；第一块就比 `WINDOW_BYTES_MAX` 长；两个编辑重叠、乱序、越过末尾；两个插入点重合；基线已经移动；文本编辑劈开一个字符、在没有标记的 UTF-8 里写进 NUL、删掉字节顺序标记；不透明的版本上的保存；只有空白的提案两边；没有终止符的句子；只在空白上不同的两句；提议与原文完全不同（对齐表超出预算）；对一句没改动的句子下判词；对删除的句子改后接受；同一张卡点两次；两张卡的区间重叠；一张卡的基线已动而它被整张拒绝；一张卡被决定之后再决定、被收回之后再决定。
-/

/-! ## 12 错误处理

| 码 | 何时 | 之后什么不变 |
|---|---|---|
| `E_INVALID_ARGS` | `Span::new` 的起点在终点之后；`Transaction::new` 的编辑重叠或乱序；`apply` 的编辑越过末尾；`decode` 的字节在这种编码下拼不出文本；`cut` 抬起的字节不够切这个窗口；`save` 的源文不是文本、改完不再是同一种编码的文本；`Offer::of` 的区间颠倒、原文或提议超过 `WINDOW_BYTES_MAX`；`merged` 的判词点名不存在或没改动的句子、同一句两次、对删除的句子改后接受；`decide` 同一张卡两次、两张接受了什么的卡区间重叠、卡的原文不是源文那一段 | 什么都没落：判定不改变任何状态 |
| `E_VERSION_CONFLICT` | `apply`、`save` 的源文不是基线版本；`decide` 里接受了什么的卡不是基于源文这一版 | 源文不变，先到的写者的改动留着，卡都还开着；调用方重读再改，或请 run 重提 |

每个拒绝是带动作、主体与恢复语的 `AxError`。「操作失败源文一定未动」在这里恒成立，因为本 crate 不写任何东西；写盘之后的失败归写者（§3）。
-/

/-! ## 13 依赖选型

`kernel`（`AxError`、`B3Hash` 与它的 BLAKE3）、`serde`（线上携带的四个类型，D1）、可选的 `schemars`（`schema` feature，只由 `wire` 的 `schema` 打开）。不引入编码探测库与 Markdown 解析器：前者会猜（D4），后者是 S7.9 的事（comrak），块的两条规则不是一棵 CommonMark 树。规格的分部不 import 任何别的 crate 的规格。
-/

/-! ## 14 硬编码声明

`WINDOW_BYTES_MAX = 65536`（D7，也是一处提案原文与提议各自的上限，D18）；`ALIGN_CELLS_MAX = 4_194_304`（对齐表的格数上限，D15）；`PROPOSAL_ID_TAG`（提案身份摘要的前缀，D13）；句子的终止符与闭合符号（迁自 RefRain，D14）；`REACH = 3`（边界最远落在一个字节之前多远：UTF-8 三个续字节，UTF-16 一个奇数字节加一个低代理）；三个字节顺序标记（Unicode）；Markdown 的两个扩展名（D6）；界定行的「至少三个」与「少于四列」（CommonMark）。模型里的 `WINDOW_BYTES_MAX` 与 Rust 的同名常量取同一个值。
-/

/-! ## 15 影响面

改 `Span`、`Format`、`Encoding` 或 `Window` 的形状就改了线上的文档答复：`wire` 的 schema 摘要变、`client/src/wire.ts` 重生成、读文档的三处页面（`client/src/core/document.ts` 的读者）跟着改。改 `Reading::of` 改的是 `Document`、`Range`、`Content` 与 `Prefix` 四条查询对「是不是文本」的同一个判断。改 `WINDOW_BYTES_MAX` 改的是每个窗口答复的大小上限。`edit` 的生产调用方是 `save`，经它是 `accounting::worker::commanding::saving`；`selection` 仍只有测试调用：页面自己把光标带过一次保存，城里没有一个读者的位置。改 `Slice`、`SliceKind`、`TextEdit` 的形状就改了线上的提案答复与保存帧；改句子的切法或对齐会改变已经入账的提案在页面上的样子，但不改变它们的身份（身份只读原文与提议，D13），也不改变任何已经落下的字节。
-/

/-! ## 16 测试与约束

证明：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`。咬得动的演示：`Documents.Edit.withoutBaseline_overwrites`、`Documents.Proposal.looseKey_loses_the_after_side`、`Documents.Window.withoutStepBack_splits`、`Documents.Encoding.nulJudgement_calls_wide_text_opaque`。

实现一致性：逐模块 `#[cfg(test)]`（`cargo nextest run -p sprawling-documents`）；§2 的表是它们与验收的对应。选区跨过一次保存的规则（D10）由 `selection::tests` 断言，没有写成定理。模型的证明不是 Rust 实现的证明。
-/

/-! ## 17 文档关系

- `architecture.toml` 里 documents 各行（锚点指向本文件与分部），ARCHITECTURE.md §3 的 `depmap`（`documents: kernel`，`wire` 与 `accounting` 两行各有它）。
- `docs/glossary.md` 的 **document version**、**document window**、**draft**。
- wire-SPEC §8-69、§8-70（文档读取契约）、§8-72、§8-73（保存与提案）、§12.8、§12.10；kernel-SPEC §8-83（`document_written` 与提案的三种事件，`SliceVerdict`）；accounting-SPEC §8-21（谁读盘、谁存版本、谁作答）、§12 第 33 条；storage-SPEC §8-36（版本进内容库）；city-SPEC §8-27、§8-40（`city::document`：锁、读当前字节、整份换上）；accounting-SPEC §8-22（保存与提案的写者、提案的折叠）。这些节改了，重读本文件对应的决定。
- refrain 路线图 §4-8、§4-10、附录 B–E、G 是本 crate 的需求来源；RefRain 的 `source_layout`、`native_document`、`manuscript::review` 与 `manuscript::decision` 四个模块是迁入的出处（§4）。
-/
