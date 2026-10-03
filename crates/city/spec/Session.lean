-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::session

规定 `session`（`crates/city/src/` 下同名的文件）。一段会话开始时清掉什么。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::session::tests` 守住。
-/

/-!
### 8-14b 一段会话开始：清掉冻下的形状与交接槽位（`city::session`）

```rust
pub fn clear_session(city_root: &Path, addr: &Address) -> Result<(), AxError>;
pub fn forget_shape(city_root: &Path, addr: &Address) -> Result<(), AxError>;
```
两个名字而不是一个带开关的名字：`clear_session` 是 `Carry::Nothing` 的整件事（形状与交接槽位都清），
`forget_shape` 是 `Carry::Handoff` 要的那一半（形状清、交接留）。**两者都不是「可选参数」**：
命令行与线协议各自都有它们要的那个动词（`crates/sprawling/Spec.lean` §8-82），而传一个 `bool` 到这里会让「带不带」
在城的接口上多出一种拼法。

**原因**：房间的第一个 run 把 `[model]` 与 `effort` 写进它自己的 `CONFIG.toml`，此后形状不同的派活全被拒（§8-14），没有逆操作，换过主模型的人就永远派不出去。这个函数就是那个出口的城侧一半（动词在 `crates/sprawling/Spec.lean` §8-82）。

- **两条写，一个决定**：新的一段开始时，房间自己写下的 `[model] name` 与 `[model] effort` 删掉（`write_session` 的逆操作），房间的 `Handoff.md` 同时清空。放在一个函数里，是因为「这一段从这里开始」是一个判断：拆成两个调用，就有一个可能没被调到，而两种半清理的状态都是假话。
- **交接槽位必须清，否则「不带」是假话**：`accounting::worker::freezing` 无条件读 `city::handoff(root, room)` 并把它折进下一个 run 的 prompt；只清配置而留文件，新一段仍会继承上一段的摘要，于是开关不起作用。
- **清的是槽位，不是内容**：`Handoff.md` 不在城的 git 里（`gitignore.rs` 列了它），账本的 `handoff_written` 记的是派活前填好的交接而不是这个文件，所以删掉的是上一段会话写的唯一一份；要带走它就用 `/new --carry`。**删文件而不写一张空白表**：`handoff` 对两者都答 `None`，而删掉少一个可被读到的中间状态。被否决的备选：保留文件、让装配器忽略「比本段起点更早」的那一份——那要求会话记住自己的起点，等于给「这份交接是不是我的」立第二个家。
- **`[model]` 之外一个键都不动**：文件里其它键是人写的，读出来、改这几处、写回去，与 §8-14 同走那一条写路径；读不动或解析不了即拒绝，不覆盖。
- **它不另立「有没有交接」的判断**：`city::handoff` 的「文件缺席，或是一张空白表，即 `None`」就是那条判断的唯一权威，`clear_session` 只把槽位清空，不重判它。

**本章测试**：`session::tests`——清后 `own_layer` 答不出模型与强度，而同一份文件里人写的其它键一个字节未动；清后 `city::handoff` 答 `None`。
-/
