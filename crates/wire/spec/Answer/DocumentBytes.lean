-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::document_bytes

规定 `answer::document_bytes`（`crates/wire/src/` 下同名的文件）。内容库里一个对象的字节，逐窗；一个 Markdown 版本导出成的一份 HTML。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-80 按版本取字节：`Query::Bytes`

```rust
// Query
Bytes { version: B3Hash, range: documents::Span },   // → Answer::Bytes(Box<BytesAnswer>)
pub struct BytesAnswer {
    pub version: B3Hash,
    pub span: documents::Span,   // 实际答出的那一段，页面从 span.end 接着要
    pub size: u64,               // 整个对象的长度
    pub base64: String,          // 这一段的字节，RFC 4648 §4 的标准字母表，带填充
}
pub const BYTES_WINDOW_MAX: u64 = 1 << 20;
```

- **读的是内容库里的对象**，不判它是不是文本：一份 PDF、一张截图、一份 DOCX 都是一串字节，怎么画是页面的事（client/Spec.lean §4-54、§4-45）。`version` 是对象在内容库里的地址，所以文档的一个版本（§8-69）与一个 `cas:` 定位指向的整个对象（截图的 `Picture.image`）是同一种问法。内容库里没有这个对象答 `Unavailable { query: "Bytes(<version>)" }`。
- **一窗至多 `BYTES_WINDOW_MAX`（1 MiB）**：从请求的起点算，终点不过对象末尾，起点在末尾之后时答末尾处的空段。不按字符边界退，因为这里没有字符。几 MiB 的 PDF 因此是几个往返，而一帧（base64 之后约 1.4 MB）远在 WebSocket 库默认的单帧上界之下。
- **base64 而不是数字数组**：JSON 没有字节类型，数字数组每个字节要两到四个字符，base64 是四个字符装三个字节。
- **只读要答的那几个字节**（`storage::Cas::size` 与 `Cas::get_range`），与 §8-70 同一个读法。
- 验收：accounting 的 `views::answering::stored::tests`——一份 PDF 写进楼里，`Document` 读过之后按版本逐窗取回，接起来与盘上的字节逐字节相同；内容库里没有的对象答 `Unavailable`。
-/

/-! D14 文件的字节经一个 `Query` 回答逐窗送到页面，不开第二扇 HTTP 门

**决定**：一个版本或一张截图的字节由 `Query::Bytes { version, range }` 答，一窗至多 1 MiB，base64 编码，走页面与城之间的同一条 socket（§8-80）；城读到或写下的每一版都存进内容库（§8-69、D8），所以文档的任何一版都有字节可取。

**理由**：远程设备经封好的会话（`remote_access` §8-10）只转发 WebSocket 的路径，一个 `Query` 的回答与其他回答一样随会话加密送达；一扇配对之后的 HTTP `GET` 要在远程监听器上开第二扇门，还要第二套鉴权与第二种失败的读法。版本身份与 `Range` 一样，页面拿着版本就能问。

**被否**：①配对判断之后按版本读内容库的 HTTP `GET`——远程门不转发它，本机与远程要两条取字节的路；它省下的是 base64 的三分之一与 JSON 的一次解析，而 client/Spec.lean §3-4 要的两条路的计时对比已由人的定规（D68）停下，没有读数说这三分之一要紧。②复用 `Query::Content`——它按定位答一段被裁剪的文字，对不是文本的对象只说「二进制」，那是它的契约；给它加一种字节的答法，同一扇门就有两种答复的形状。

**重开参数**：一份文件经这扇门送到页面的时间成为人能察觉的等待（几十 MiB 的 PDF），或者远程门开始转发 HTTP 的 `GET`。
-/

/-!
### 8-81 导出一个 Markdown 版本：`Query::Export`

```rust
// Query
Export { at: Address, version: B3Hash },     // → Answer::Export(Box<ExportAnswer>)
pub struct ExportAnswer { pub version: B3Hash, pub html: String }
```

- **城按同一个文法读**：内容库里这一版的整份字节交给 `documents::export`（documents D33），读成预览用的同一棵树，再写成一份独立的 HTML：没有脚本，没有外部资源，链接照 documents D23 判过。`at` 只给出文件名，作 HTML 的标题；读的是 `version` 那一版，不是盘上此刻的文件。
- **答不了的都是 `Unavailable { query: "Export(<version>)" }`**：内容库里没有这一版，这一版不是 UTF-8 的文本，或者它长过 `documents::EXPORT_BYTES_MAX`。页面只对 Markdown 的版本给出导出，说「城没能导出这一版」。
- 验收：documents 的 `markdown::export::tests`；accounting 的 `views::answering::stored::tests`——一个存过的 Markdown 版本导出的 HTML 带它的标题、块与链接，不带 `<script`。
-/
