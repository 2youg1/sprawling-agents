-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::preview

规定 `answer::preview`（`crates/wire/src/` 下同名的文件）。一个 Markdown 版本的一个窗口读成的块，与一段回复读成的块。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-74 一个 Markdown 版本的预览：`Query::Preview`

```rust
// Query（紧接 Range 之后）
Preview { version: B3Hash, viewport: documents::Span },   // → Answer::Preview(Box<PreviewAnswer>)
pub struct PreviewAnswer { pub version: B3Hash, pub preview: documents::Preview }
// documents::Preview::{Laid(documents::Laid), Unsupported { encoding }}；Laid { span, blocks: Vec<documents::Block> }（documents D32）
// Block 与 Inline 的形状见 crates/documents/Spec.lean §8（documents D21）
```

- **读的是内容库里的那一版**，与 `Range` 同一个理由（§8-70）：页面滚到第三屏时文件可能已经被改过，它要的是它打开的那一版。内容库里没有这一版，或这一版的字节不是任何一种本城读的编码的文本，答 `Unavailable { query: "Preview(<version>)" }`。
- **窗口照 `Range` 的切法，再止于块末**（documents D25）：没到版本末尾的窗口止于倒数第二块的末尾，因为最后一块可能在窗口之外接着写。答复里的 `Laid.span` 是实际读的那一段，页面从它的 `end` 接着要下一窗；块的区间是这一版里的字节，页面拿它对上源码的位置（documents D26）。
- **块是数据，不是 HTML**（documents D21）：页面按块画元素，模型或居民写的字符不进 `innerHTML`。读得出而本页不画的——HTML、公式、前置元数据、过深的嵌套——是带原文的 `Unsupported`；一个空窗口是零个块；UTF-16 的版本答 `Preview::Unsupported { encoding }`。三者形状各不相同，页面因此分得清「这里有一个不画的东西」「这里什么也没有」「这一版不按 Markdown 读」（refrain 路线图 A4）。
- **一个版本的预览不会过时**：版本的字节不变，所以页面不因任何事件重问（`client/src/core/staleness.ts` 与 `range`、`content` 同一行）。
- **够得到它的门**：它是一个 `Query`，经 `Ask` 帧到达，属 §19-3 的 `Read`，带 `Watch` 或 `Act` 权限的远程设备都能问。
- **Markdown 版本不论长短都在内容库里**（accounting D33）：`Document` 答一份 Markdown 文件时把这一版放进内容库，答复照旧是 `Coverage::Whole` 或 `Head`，所以一份整份放得下 64 KiB 的文件也能按它的版本预览。对话流读同一个文法，入口是 §8-75。
- 验收：accounting 的 `views::answering::preview::tests`——内容库里没有的版本答 `Unavailable`；超过一个窗口的文件从 `Document` 打开后逐窗预览，每一窗止于块末，读到末尾时每一段恰好出现一次；整份放得下一个窗口的 Markdown 文件从 `Document` 打开后预览出它的块；不是文本的版本答 `Unavailable`，UTF-16 的版本答 `Preview::Unsupported`。
-/

/-!
### 8-75 一段回复读成块：`Query::Reply`

```rust
// Query（紧接 Preview 之后）
Reply { text: String, state: documents::ReplyState },   // → Answer::Reply(Box<documents::Laid>)
// documents::ReplyState::{Streaming, Settled}（documents D30）
```

- **页面送来它手里的文字。** 一段回复没有版本：还在说的经 `Delta` 到页面（§8-8），结算的在 `model_returned` 的载荷里。页面两种都已经拿着，所以它把文字连同「还在说／已结算」送来，城用预览的同一个文法读（`documents::reply`，documents D30）。被否的两种做法与理由在 D12。
- **结算了的回复读出的树就是它的预览。** `Settled` 按「一个只装着这段文字的版本」从 0 读，所以同一段文字经 `Reply` 与经 `Preview` 读出同一个 `Laid`。`Streaming` 只读收束点之前的字节（documents D31）：代码块之外的空行、顶格的标题、顶格开启的代码块的闭合行之后，没有换行的最后一行不算；答复的 `span.end` 就是页面画原文的开头。
- **一次至多一个窗口。** 与预览同一个切法（documents D7、D25）：要读的部分超过 `WINDOW_BYTES_MAX` 时，答复止于界内倒数第二块的末尾，页面把 `span.end` 之后的文字再送一次，块的区间加上它送出的那一段的起点。区间是送来的文字里的字节，不是 UTF-16 码元，换算归页面的 `core/document_pos.ts`。
- **读不出就答 `Unavailable { query: "Reply" }`**：文字里有 NUL 时，它不是没有标记的 UTF-8 文本（documents D4），页面照旧画原文。
- **不会过时**：同一段文字恒读出同一棵树，`client/src/core/staleness.ts` 与 `preview` 同一行。
- **够得到它的门**：它是一个 `Query`，属 §19-3 的 `Read`；它不读城里的任何东西，答复只取决于问题里的文字。
- 验收：accounting 的 `views::answering::reply::tests`——一段回复存成一个版本，`Preview` 读出的 `Laid` 与 `Reply { state: Settled }` 读出的相等；同一段文字还在说时只答闭合的块，`span.end` 停在开着的那一块之前；含 NUL 的文字答 `Unavailable`。
-/

/-! D11 Markdown 在城里读，页面按版本与窗口问它的块

**决定**：`Query::Preview { version, viewport }` 答一个 Markdown 版本的一个窗口读成的块（§8-74）；文法是 `documents::markdown`，comrak 在二进制里，不编成 wasm 进浏览器（documents D20）。

**理由**：版本在城里，页面只持有窗口，城侧一次答复带一个窗口的块，与 `Range` 同一个往返；导出本来就在 Rust 里，读的是同一个函数；页面不多一个字节的运行时依赖，反而删掉自写的 `prose.ts`。按版本问而不按地址问，理由与 D8 的 `Range` 相同。

**被否**：①comrak 编成 wasm 在浏览器里读——wasm 的导出要 `unsafe`，而工作区只有 `crates/desktop/ffi` 有自己的 lint 表，多一个就要人的定规；工具链多一个目标，包体推断多 100 KB 以上（documents D20）；②`Preview { at: Address }` 读文件此刻——与 D8 的②同一个缺陷，读到一半的页面要从头重读；③答 HTML——页面要把城给的字符放进 `innerHTML`，消毒规则成为第二个权威（documents D21）。

**重开参数**：远程门上一次往返的读数超过人能察觉的界、而 D12 的做法消不掉它；或允许第二个带自己 lint 表的 crate。
-/

/-! D12 对话流按文字问块，不按版本，不随增量带块

**决定**：`Query::Reply { text, state }` 答一段回复读成的块（§8-75）；页面送的是它手里的文字，城不另存回复，`Delta` 帧不变。

**理由**：页面手里本来就有回复的文字——流式时来自增量，重开页面时来自 `model_returned` 的载荷——所以一条带文字的查询同时盖住流式与历史，页面只接一个入口就能删掉 `prose.ts`。读法是预览的那一个函数，所以结算的回复与同一段字节的预览读出同一棵树（documents D30）。

**被否**：①写 `model_returned` 的一方把回复另存进内容库、记录带上地址，页面按版本问 `Preview`——回复的文字有了第二个家，`model_returned` 的形状要变，之前写下的行没有地址、页面仍要第二个读法，还在说的那一段也只能画原文；②城在 `Delta` 旁带上已闭合的块——`Delta` 按构造可丢（§8-8），带了块之后漏一帧就少一块，而从历史打开的回复没有增量，还要第二个入口。

**代价与重开参数**：文字随每一问回到城里一次，每次闭合多一次往返；页面同时只有一问在途、只在文字长了时再问（client/Spec.lean §4-26）。远程门上量出的往返超过 100 ms（refrain 路线图 §5 的 `client_send_feedback`）时，流式这一半另加 ②，历史仍走本条。
-/
