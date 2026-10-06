-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::privacy

规定 `privacy`（`crates/wire/src/` 下同名的文件）。主机隐私页与城共享的名字。本文件是
`crates/wire/Spec.lean` 的一个分部；下面一节保留它在 wire 规格里的标签 §8-85，别处引作
`crates/wire/Spec.lean §8-85`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；名字集合由 Rust 的类型
守住，每个名字背后的数据与它们进入写入路径的性质由 `crates/sprawling/spec/Privacy/Controls.lean`
与 `crates/sprawling/spec/Privacy.lean` 规定。
-/

/-!
### 8-85 隐私页的七个闭集

```rust
pub enum PrivacyControl { PowershellTelemetryOptout, CeipConsolidatorTask, … }  // 88 个，"powershell_telemetry_optout" …
pub enum PrivacyOriginal { K01, K02, …, K52 }                                    // "k01" … "k52"
pub enum PrivacyNotWritten { Absent, Obsolete, Undeterminable, NeedsOperationKind }
pub enum PrivacyCategory { Diagnostics, SpeechInput, LocationSensors, Search, Content,
                           ActivitySync, CloudServices, AppPermissions, WindowsAi }
pub enum PrivacyEdition { Home, Pro, Enterprise, Education, IotEnterprise, Server }
pub enum PrivacyBuildEffect { Documented, Uncertain, NoCurrentEffect }
pub enum PrivacyEditionFit { Honoured, Ignored, NotStated }
// 每个闭集带 `ALL`：成员按声明次序，即页面次序。
```

**线上只有名字。** 一个控制的目标路径、写入值、版本清单只在二进制的 `privacy::controls` 定义一次，
页面上的文字只在客户端的 `lang.json` 定义一次，两边都按这些名字的拼写取。名字集合因此各只有一份
成员表，每个读者从这里取。

**声明次序就是页面次序。** `PrivacyControl` 先按 `PrivacyCategory` 的次序、类别内按控制表次序声明；
`ALL` 由同一个宏从变体表生成，所以加了变体却漏掉次序的状态不存在。

**本节只加类型，不进帧，`WIRE_V` 不动。** 这些类型不出现在任何 `Query`、`Answer` 或 `Command` 上，
schema 哈希读的是帧名表与事件种类名，不随它们变；它们进帧（`Query::Privacy`、
`Command::PrivacyOperation`）时随帧一次进位，客户端的 `wire.ts` 同时重生。

**被否**：把控制的路径与写入值放进 wire 枚举的属性——客户端就能读到并复制它们，控制表成了两份；
用字符串而非闭集——页面的 `lang.json` 缺一个键时就不会在类型检查时失败。
-/
