-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::document

规定 `answer::document`（`crates/wire/src/` 下同名的文件）。城里一份文件作为一个版本：缺失、读不了、空，或带第一个窗口。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-69 文档读取契约：`Query::Document` 答一个版本

```rust
// Query（形状不变）
Document { at: Address },                    // → Answer::Document(Box<DocumentAnswer>)
pub struct DocumentAnswer { pub at: Address, pub state: DocumentState }
pub enum DocumentState {                     // 线上 "missing" | { unreadable } | { empty } | { held }
    Missing,                                 // 这个地址上没有文件
    Unreadable { reason: String },           // 有东西而读不出：目录、无权限、读到一半出错；reason 是系统的原话
    Empty { version: B3Hash, format: documents::Format },
    Held(Box<HeldDocument>),
}
pub struct HeldDocument { pub version: B3Hash, pub format: documents::Format, pub bytes: u64, pub body: DocumentBody }
pub enum DocumentBody {
    Text { encoding: documents::Encoding, head: documents::Window, coverage: Coverage },
    Opaque,                                  // 不是任何一种本城读的编码的文本：只有版本与大小
}
pub enum Coverage { Whole, Head }            // Head：其余的经 Query::Range 按 version 读
// documents::{Span, Format, Encoding, Window} 由 documents crate 定义，线上直接携带（documents D1）
```

- **版本身份是整份字节的 `B3Hash`**，与内容库给同一份字节的地址相同（documents D3）。下一次保存拿它作基线（§8-72），页面拿它判断两次读到的是不是同一版。
- **缺失、读不了、空是三种答复**，不再借 `Unavailable`：「这里没有文件」页面画成可以新建，「读不了」页面说出系统的原话，「空」是一份可以写的文件，三者页面采取的动作不同。空文件也有版本（空字节的摘要），因为它同样可以是一次保存的基线。`Unavailable { query: "Document(<at>)" }` 只剩视图本身答不了的情形。
- **文本的判定**（documents D4）：字节顺序标记先判，所以带标记的 UTF-16 是文本；没有标记时，不含 NUL 的合法 UTF-8 是文本；其余是 `Opaque`。不再有损解码，不再凭头 8 KiB 的 NUL 判二进制。
- **第一个窗口** `head`（documents D7）：整份放得下 `WINDOW_BYTES_MAX`（64 KiB）就是整份（`Coverage::Whole`）；放不下时止于放得下的最后一个块的末尾，一块都放不下时止于界内最后一个字符边界（`Coverage::Head`）。窗口从第 0 个字节数起，标记是文本的第一个字符（documents D5），所以页面把各窗口的文本接起来就是整份文本。
- **`Coverage::Head` 的版本在内容库里**：答复发出之前，读面把这一版的字节放进城的内容库（`crates/storage/Spec.lean` §8-36），所以之后按版本取范围读的是这一版，而不是文件此刻的样子。整份已经在答复里的版本不存：页面没有理由再要它。
- 验收：accounting 的 `views::document::tests`——缺失、目录、空文件各得各的答复；带标记的 UTF-16 文件判成文本并解出原文；没有标记而含 NUL 的判成 `Opaque`；超过一个窗口的文件答 `Coverage::Head`，它的版本在内容库里。
-/

/-! D8 文档答复带版本，三种「没有文本」各是一种答复，范围按版本读

**决定**：`Query::Document` 答 `DocumentAnswer { at, state }`：缺失、读不了、空、有内容四种状态；有内容时带版本、格式、大小与第一个窗口，文本之外的是 `Opaque`（§8-69）。其余的经 `Query::Range { version, range }` 从内容库里那一版读（§8-70）。

**理由**：文档工作区要编辑、要保存、要知道自己改的是哪一版（refrain 路线图 §4-8），旧答复只有一个被截断、被有损解码的头，没有版本。把版本做成内容库的地址，「这一版的第三屏」就有一个不随文件变动的读法，而下一次保存的基线是 32 字节的摘要而不是整份正文。

**被否**：①仍用 `Unavailable` 答缺失：页面分不出「可以新建」与「读不了」，`Unavailable` 的 `query` 串也不是给人读的理由；②范围读文件此刻的字节、版本不对就拒：文件每动一次，读到一半的页面就要从头重读，而一个居民在写的文件每几秒动一次；③每一版都存进内容库：一份整份已经在答复里的小文件，页面不会再按版本要它，存下它只是让内容库替每一次打开付一份拷贝。

**重开参数**：页面要读一份整份放得下的文件的旧版本时（例如对比保存前后），小文件也存版本；内容库长出回收时，按版本读的窗口要说「这一版已经回收」，与 `Unavailable` 分开。
-/
