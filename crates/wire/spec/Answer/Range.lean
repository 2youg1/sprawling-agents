-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::range

规定 `answer::range`（`crates/wire/src/` 下同名的文件）。按版本读一份已存版本的一个窗口。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-70 按版本取范围：`Query::Range`

```rust
// Query
Range { version: B3Hash, range: documents::Span },   // → Answer::Range(Box<RangeAnswer>)
pub struct RangeAnswer { pub version: B3Hash, pub window: documents::Window }
```

- **读的是内容库里的那一版**，不是文件此刻：一个页面滚到第三屏时，文件可能已经被居民改过，而它要的是它开始读的那一版的第三屏。内容库里没有这一版答 `Unavailable { query: "Range(<version>)" }`。
- **请求的是半开字节区间，答的是切好的窗口**（documents D2、D7）：起点在字符中间时退到那个字符的第一个字节，终点往回退到字符边界，长度不过 `WINDOW_BYTES_MAX`，起点在版本末尾之后时答末尾处的空窗口。编码由这一版前三个字节里的标记定，切出的字节在这种编码下拼不出文本（`Opaque` 的版本）时答 `Unavailable`。答复里的 `window.span` 是实际切出的区间，页面从它的 `end` 接着要下一段。
- **只读内容库里要答的那几个字节**（`storage::Cas::size` 与 `Cas::get_range`），所以第三屏的代价是第三屏，与文件多大无关。
- 验收：accounting 的 `views::answering::range::tests`——从 `Document` 的 `head` 末尾起逐段读到末尾，窗口首尾相接就是整份字节；文件在第一次读之后被改写，按旧版本读出的仍是旧字节；内容库里没有的版本答 `Unavailable`。
-/
