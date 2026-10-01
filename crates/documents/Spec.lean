-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.documents.spec.Edit
import crates.documents.spec.Encoding
import crates.documents.spec.Window

/-! # documents 的规格

`documents` 是一个人经页面读、改的文档的规则：它是哪一版、它的字节拼出哪些字符、它的块在哪里、一次答复带它多少、一次保存改了什么。本文件是 crate 的规格入口，分部在 `spec/` 下，布局见 ARCHITECTURE.md §11「Specifications in Lean」。能写成定理的性质在分部里证明；本文件的十七节记录其余的要求、理由与决定，决定写作 `D<n>`，别处引作 `documents D<n>`。
-/

/-! ## 1 需求分解

来源是 refrain 路线图 §4-8（S7.8）与 Roadmap U8 的 Rust 一半。七个可独立验收的单元，与模块一一对应：

- 一段字节（`span`）：版本里的一段半开字节区间（D2）。
- 编码（`encoding`，`spec/Encoding.lean`）：一个版本的字节是哪种编码的文本，或者不是文本（D4、D5）。
- 格式（`format`）：文档按哪种文法分块，由文件名定（D6）。
- 块（`layout`）：块在字节里的位置，迁自 RefRain `source_layout`（D6）。
- 窗口（`window`，`spec/Window.lean`）：一次答复带一个版本的哪一段，在哪里切（D7）。
- 编辑事务（`edit`，`spec/Edit.lean`）：一次保存是对一个基线版本的一串编辑，落下就交回撤销它的事务（D8、D9）。
- 选区（`selection`）：一个读者的位置怎样跨过一次保存（D10）。

本 crate 回答「这些字节是什么、能怎样切、能怎样改」；它不读盘、不读内容库、不记账。
-/

/-! ## 2 验收标准

分部里的定理是模型对性质的证明：编辑之外的字节原样留下、撤销还原基线、同一基线的第二次保存被拒（`spec/Edit.lean`）；窗口不劈开字符、不过版本末尾、不过请求起点之后 `WINDOW_BYTES_MAX` 字节、有边界就有进展（`spec/Window.lean`）；带标记的 UTF-16 是文本、无标记而含 NUL 的不是、判成文本的拼得出（`spec/Encoding.lean`）。每个分部各有一条「拿掉守卫即反例」的定理。

refrain 路线图 §4-8 的两条验收在 Rust 测试里成立，测试走生产入口：

| 验收 | 完成的定义 |
|---|---|
| A1 字节不变 | `edit::tests::every_byte_outside_the_edits_is_copied_through`：带 BOM、混合换行、尾随空格、非 UTF-8 的四份样本，编辑之外的每个字节原样留下；`window::tests::windows_read_end_to_end_give_back_every_byte`：逐窗读完一个版本，窗口首尾相接就是这个版本，逐字节；`layout::tests` 的 RefRain 回归语料：块与空隙首尾相接就是源文 |
| A3 后到者被拒 | `edit::tests::a_second_save_from_the_same_version_is_refused`：两个写者从同一版本出发，先到的落下，后到的得到 `E_VERSION_CONFLICT`，先到者的字节不动 |

A3 的另一半——经线上 `PutRange` 保存、草稿不被覆盖——等 `PutRange` 落地（§3）。
-/

/-! ## 3 假设与歧义

- **假设**：BLAKE3 抗碰撞。版本身份是整份字节的摘要（D3），「摘要相同即同一版本」靠它；`spec/Edit.lean` 把它写成 `later_is_refused` 的假设 `inj`，不证明它。
- **假设**：没有标记的 UTF-16 不出现在城里需要读成文本的地方。它被判成不透明的而不是被猜（D4）；人的文件里真有这样的一份时，页面说它不是文本，而不说错字。
- **未定：`PutRange`。** 线上的保存（`Command::PutRange { doc, baseline, idem, edits }`，refrain 路线图 §4-8）要一种新的账本事件来如实记下这次写：哪个文档、从哪一版到哪一版、多少字节。现有的 `spine_document_written` 只说四份 spine 文档之一，`governed_document_written` 只说三份治理文档之一，`rules_changed` 只说 `RULES.toml` 与 `CONFIG.toml`，都不能如实记一份任意文档的一次写，所以 `PutRange` 等加事件种类的那一轮一起落（wire-SPEC §3）。那时它的规则已经在这里：`Transaction::apply` 判基线、落下、交回撤销（D8、D9），I/O 一半照 `city::edit_against` 的锁与整份替换。决定它的证据是那一轮的事件表；在那之前 `edit` 与 `selection` 只有测试这一个调用方。
-/

/-! ## 4 现状分析

本 crate 之前，`Query::Document` 只给文件头 64 KiB、`from_utf8_lossy` 有损解码、头 8 KiB 里有 NUL 就判二进制（refrain 路线图附录 C）：UTF-16 文件被判成二进制，非 UTF-8 的字节被替换字符盖掉，而答复里没有版本，页面无法说「我改的是哪一版」。现在 `accounting::views::document` 用本 crate 的 `Reading::of`、`head` 作答，`accounting::views::answering::range` 用 `lift`、`cut` 按版本答范围（accounting-SPEC §8-21，wire-SPEC §8-69、§8-70）；`Content` 与 `Prefix` 的文本判定经同一个 `Reading::of`。

**迁入的部分。** RefRain（另一个仓库，`refrain-core` crate 的 `source_layout` 模块）的两条分块规则（Markdown 与纯文本）与它的回归语料：二十份文本按 RefRain 生成语料的脚本逐字搬进 `layout::tests`，每一份的字节与 RefRain 语料清单记的 SHA-256 核过，块区间与它冻结的块表逐项相同。RefRain 的 `DocumentSurface`（`refrain-app` crate 的 `native_document` 模块）里与平台无关的部分迁的是有界投影的做法：按块取一屏、取不下一块时退到字符边界（`window::head`）。没有迁的：`DocumentSurface` 的输入法组合、光标移动与撤销栈——草稿只在浏览器（refrain 路线图 §4-8），服务端没有一个正在编辑的文档；迁入时去掉了直接索引、`expect`、`as` 与内部随机 ID。
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

**document version**、**document window**、**draft** 取自 `docs/glossary.md`。「版本」在本 crate 恒指一份文档某一刻的全部字节，它的身份是 `B3Hash`；「窗口」恒指一次答复带的那一段；「草稿」是浏览器里还没保存的文本，本 crate 里没有它。模型里的声明名取同样的词：`Documents.Edit.apply`、`Documents.Window.stepBack`、`Documents.Encoding.reading`。
-/

/-! ## 7 模块边界

- **字节从哪里来**归调用方：`accounting::views` 读盘与内容库，把切片交进来；本 crate 恒不持文件句柄。
- **一版字节存到哪**归 `storage::cas`：版本身份就是内容库的地址（D3），所以按版本取范围就是按地址读对象（storage-SPEC §8-36）。
- **一次保存怎样落盘、怎样记账**归 `city::document` 与写者（§3 的 `PutRange`）。
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
- **`edit`**（形状 2 值类型 ＋ 形状 1 判定）：`Edit { span, bytes }`、`Transaction::new(baseline, edits)`、`Transaction::apply(&[u8]) -> Result<Applied, AxError>`、`Applied::{bytes, version, undo}`（`spec/Edit.lean`）。
- **`selection`**（形状 2 值类型）：`Selection { anchor, focus }`、`collapsed`、`span`、`mapped(&Transaction)`。
-/

/-! ## 9 工作流程

读：调用方读一份文件的全部字节 → `B3Hash::digest` 得版本 → `Format::of_name` → `Reading::of`；是文本就 `head` 取第一个窗口，窗口没到末尾时调用方把这一版存进内容库。之后每一次按版本取范围：调用方从内容库读这一版的长度与前三个字节（`Encoding::of_mark`），`lift` 说要抬起哪几个字节，`cut` 切出窗口。

写（§3，`PutRange` 落地之后）：`Transaction::new(baseline, edits)` → 调用方在文档锁里读当前字节 → `apply` 判基线、落下 → 整份替换 → 记账 → 回执带 `Applied::version`。
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
-/

/-! ## 11 边界枚举

空文档（一个空的纯文本行、零个 Markdown 块，版本是空字节的摘要）；只有标记的文档；没有结尾换行；混合 `\r\n` 与 `\n`；尾随空格与制表符；四列缩进的界定行（不是界定行）；没闭合的代码块（直到末尾都是一块）；非 UTF-8 字节；没有标记而含 NUL；落单的代理；奇数长度的 UTF-16；起点在字符中间、在标记中间、在版本之后；请求比 `WINDOW_BYTES_MAX` 长；第一块就比 `WINDOW_BYTES_MAX` 长；两个编辑重叠、乱序、越过末尾；两个插入点重合；基线已经移动。
-/

/-! ## 12 错误处理

| 码 | 何时 | 之后什么不变 |
|---|---|---|
| `E_INVALID_ARGS` | `Span::new` 的起点在终点之后；`Transaction::new` 的编辑重叠或乱序；`apply` 的编辑越过末尾；`decode` 的字节在这种编码下拼不出文本；`cut` 抬起的字节不够切这个窗口 | 什么都没落：判定不改变任何状态 |
| `E_VERSION_CONFLICT` | `apply` 的源文不是基线版本 | 源文不变，先到的写者的改动留着；调用方重读再改 |

每个拒绝是带动作、主体与恢复语的 `AxError`。「操作失败源文一定未动」在这里恒成立，因为本 crate 不写任何东西；写盘之后的失败归写者（§3）。
-/

/-! ## 13 依赖选型

`kernel`（`AxError`、`B3Hash` 与它的 BLAKE3）、`serde`（线上携带的四个类型，D1）、可选的 `schemars`（`schema` feature，只由 `wire` 的 `schema` 打开）。不引入编码探测库与 Markdown 解析器：前者会猜（D4），后者是 S7.9 的事（comrak），块的两条规则不是一棵 CommonMark 树。规格的分部不 import 任何别的 crate 的规格。
-/

/-! ## 14 硬编码声明

`WINDOW_BYTES_MAX = 65536`（D7）；`REACH = 3`（边界最远落在一个字节之前多远：UTF-8 三个续字节，UTF-16 一个奇数字节加一个低代理）；三个字节顺序标记（Unicode）；Markdown 的两个扩展名（D6）；界定行的「至少三个」与「少于四列」（CommonMark）。模型里的 `WINDOW_BYTES_MAX` 与 Rust 的同名常量取同一个值。
-/

/-! ## 15 影响面

改 `Span`、`Format`、`Encoding` 或 `Window` 的形状就改了线上的文档答复：`wire` 的 schema 摘要变、`client/src/wire.ts` 重生成、读文档的三处页面（`client/src/core/document.ts` 的读者）跟着改。改 `Reading::of` 改的是 `Document`、`Range`、`Content` 与 `Prefix` 四条查询对「是不是文本」的同一个判断。改 `WINDOW_BYTES_MAX` 改的是每个窗口答复的大小上限。`edit` 与 `selection` 今天只有测试调用（§3）。
-/

/-! ## 16 测试与约束

证明：分部里的定理由 `just models`（`lake build Spec`）证明，无 `sorry`、`admit`、`axiom`。咬得动的演示：`Documents.Edit.withoutBaseline_overwrites`、`Documents.Window.withoutStepBack_splits`、`Documents.Encoding.nulJudgement_calls_utf16_opaque`。

实现一致性：逐模块 `#[cfg(test)]`（`cargo nextest run -p sprawling-documents`）；§2 的表是它们与验收的对应。选区跨过一次保存的规则（D10）由 `selection::tests` 断言，没有写成定理。模型的证明不是 Rust 实现的证明。
-/

/-! ## 17 文档关系

- `architecture.toml` 里 documents 各行（锚点指向本文件与分部），ARCHITECTURE.md §3 的 `depmap`（`documents: kernel`，`wire` 与 `accounting` 两行各有它）。
- `docs/glossary.md` 的 **document version**、**document window**、**draft**。
- wire-SPEC §8-69、§8-70（文档读取契约）、§12.8；accounting-SPEC §8-21（谁读盘、谁存版本、谁作答）、§12 第 33 条；storage-SPEC §8-36（版本进内容库）；city-SPEC §8-27（`city::document`，`PutRange` 落地时的写者）。这些节改了，重读本文件对应的决定。
- refrain 路线图 §4-8、附录 B–E、G 是本 crate 的需求来源；RefRain 的 `source_layout` 与 `native_document` 两个模块是迁入的出处（§4）。
-/
