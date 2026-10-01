-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::diagnostics

规定 `diagnostics`（`crates/runtime/src/` 下同名的文件）。诊断日志：只写、五级、锚在账本位置上。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-17 runtime::diagnostics（形状 4 薄壳＋形状 6 数据面）


设计权威是 `docs/logging.md`；本节只记接口与三处口径差异。

```rust
pub enum Level { Refuse, Effect, Decide, Trace, Wire }  // 全序：层底控到该级为止
impl Level { pub const DEFAULT: Level = Effect; pub const ALL: [Level; 5]; pub fn parse(&str) -> Option<Level>; }
pub struct Site<'a> { pub run: RunId, pub seq: Seq, pub module: &'a str }   // 三字段必填
pub type Sink = Box<dyn FnMut(Entry<'_>) + Send>;   // 一条 entry，不是一行文本（§8-38）
pub struct Diagnostics { /* floor: Option<Level>、sink —— 私有 */ }
impl Diagnostics {
    pub fn new(floor: Level, sink: Sink) -> Diagnostics;
    pub fn off() -> Diagnostics;
    pub fn floor(&self) -> Option<Level>;
    pub fn admits(&self, level: Level) -> bool;
    pub fn write(&mut self, level: Level, site: Site<'_>, message: &str);
    // 无读方法。这是本模块全部保证的形状半边
}
// 无自己的打码器：`write` 交给 sink 之前调 `redact::redact_text(message, Marker::Plain)`
```

- **无读方法即全部形状保证**：「判定与恢复逻辑不读日志」不靠纪律，靠这一点——把一行读回来在类型上拼不出。推论就是收口条件：删光日志，行为、重放与总账逐字节不变。
- **行上恒无时间戳**：锚点是 `seq`——两条时间线靠一个整数对齐，而采样壁钟会在一个不允许采样的库里开第二个时间源。想要时间的 sink 在装配层自己加。
- **坐标由 Ledger 自己说**：`storage::JsonlLedger::position()`（返回「现在写一条会落在哪」）。只给位置不给内容：一个能读记录的访问器会把判定逻辑引到它正在写的账上去。
- **双重防线**：`Sealed` 无 Debug/Display，入行在类型层就不成立（反例 `tests/ui/log_a_credential.rs`）；普通字符串里的明文由 `redact::redact_text`——**同一个**扫描器与**同一份**替换实现，不是第二个——就地换成 `secret:redacted`（`Marker::Plain`）。不丢整行：周围那句话通常正是读者要的。
- **不引 `tracing`**：它在此处的唯一功能是跨 `await` 携模块名的 span，而回合路径是同步的，该功能无消费者。理由见 `docs/logging.md` §7。
- **写入方三处**（§6 的三类各一）：命令被拒（`refuse`，写在 `handle` 而非调用方，因为每个调用方都要）；endpoint 附着与探测结果（`effect`）；dispatch 跑完（`effect`，作为指向 Ledger 的指针）。
-/

/-!
### 8-38 sink 收到的是一条 entry，不是一行文本（形状 2 值类型）


**sink 收的是 entry 而不是渲染好的行**：否则「把日志行推给浏览器」的装配层要把刚渲染好的那行**再解析回来**才能拿到 `level`／`run`／`seq`／`module`，一条日志行由什么字段构成就有写与读两个权威，而读的那个漂了也没人看得见。

**改法**是把字段本身交给 sink，文本留作其中一种去处：

```rust
pub struct Entry<'a> { pub level: Level, pub site: Site<'a>, pub message: &'a str }  // message 已过扫描
pub type Sink = Box<dyn FnMut(Entry<'_>) + Send>;
pub fn render(entry: Entry<'_>) -> String;   // 一行 JSON：文本的唯一产出点
```

- **`redact` 的位置不动**：`write` 仍在交给 sink 之前扫一遍，所以任何 sink 拿到的 `message` 都是已脱敏的，不存在「某个 sink 忘了扫」。
- **`render` 转公开而不是复制一份**：写终端的 sink 调它，携字段上路的 sink 不调它，两者因此不可能对「一行由什么构成」得出两个结论。
- **无读方法这一条不受影响**：`Entry` 只在写入那一刻存在于 sink 的参数位上，本模块仍不提供任何把已写的行读回来的途径。
- **被否**：保留 `&str` sink，由装配层 `serde_json::from_str` 回读。它多一次序列化与一次解析，且给「字段名」造了第二个权威——而第二权威失配的表现是浏览器上少一个字段，不是一次编译失败。
-/
