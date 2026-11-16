-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.kernel.spec.Address

/-!
# city::governed

规定 `governed`（`crates/city/src/` 下同名的文件）。治理这座城的三份文件，与写它们的那一扇门。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-24b city::governed：治理这座城的三份文件（形状 4 adapter）

```rust
pub const PREFERENCES_FILE: &str = "PREFERENCES.md";
pub enum Governed { Mayor, Clerk, Preferences }
impl Governed { pub fn file(self) -> &'static str; pub fn path(self, city_root: &Path) -> PathBuf; }
pub fn write_governed(city_root: &Path, which: Governed, base: &str, body: &str) -> Result<PathBuf, AxError>;
```

三份文件都住 `<city>/.sprawling/`——没有任何写域够得到的地方（前两份也在那里）。**本模块之所以是一扇门而不是三个调用方各自拼一条路径**：能自己拼路径的调用方就能拼出一条走出保留子树的路径，那样「居民改不了治自己的东西」就成了靠习惯成立而不是靠构造成立。

- **`Preferences` 是第三份而不是第三个居民**：市长与文书各有身份文件，而「这个人怎么喜欢这座城办事」不属于任何一个居民，它属于城；它与前两者被同一条规矩治理，所以住同一处、走同一扇门。
- **整份覆写，带基线**：这是人在一个框里编辑、按一次保存的文件，写一半会让这座城被半句话治理。写者有两个——原文编辑器与设置页上的卡片（§8-33），或两个开着的页面——所以一次保存携它起手时的全文，文件已经变了就拒。旧内容不留在这里——账本上那行 `governed_document_written` 才是回头看的地方。
- **枚举而不是文件名字符串**：文件名是城的答案，不是发帧的人的答案（`crates/wire/Spec.lean` §8-19 同一条理由，两侧各说一次）。
-/

/-!
## 模型：三份治理文件在一切写域之外

`Governed::path` 把三份文件都放在城根的保留子树里：`<city>/.sprawling/<文件>`。以城根为起点，这个位置的地址是 `[.sprawling, <文件>]`，它的首段受保护，所以是保留的，`WriteDomain::admits` 对它恒答 `Outside`（`crates/kernel/spec/WriteDomain.lean`）。

* 三份文件不论叫什么，都没有写域够得到（`the_governed_documents_are_out_of_every_write_domain`）；居民改不了治理自己的东西靠的是这个位置，而不是调用方的习惯——能自己拼路径的调用方才可能拼出保留子树之外的路径，所以写它们的只有 `write_governed` 这一扇门。
-/

namespace City.Governed

open Kernel.Address

/-- `Governed`：三份文件。 -/
inductive Governed where
  | Mayor
  | Clerk
  | Preferences
  deriving DecidableEq, Repr

/-- `Governed::path` 以城根为起点的地址：城的保留子树里的那份文件。`file` 是 `Governed::file` 的答案。 -/
def place (reserved : String) (file : Governed → String) (which : Governed) : Address :=
  ⟨[reserved, file which]⟩

theorem the_governed_documents_are_out_of_every_write_domain (protected_name : String → Bool)
    (reserved : String) (protects : protected_name reserved = true) (file : Governed → String)
    (which : Governed) : is_reserved protected_name (place reserved file which) = true := by
  simp [place, is_reserved, protects]

end City.Governed
