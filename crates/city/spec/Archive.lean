-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::archive

规定 `archive`（`crates/city/src/` 下同名的文件）。一条记录存哪里、怎么找回来。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-9 city::archive（形状 2 值类型＋形状 7 投影）

```rust
pub enum Kind { Preference, Decision, Correction, Fact }   // 封闭四类
pub struct Entry { pub kind: Kind, pub day: u64, pub subject: String, pub at: PathBuf }
pub fn day_of(at: TimeMs) -> u64;                          // 时间只入参
pub fn entry(city_root, building, kind, at, subject) -> Result<Entry, AxError>;  // 纯：不碰盘
pub fn file(entry: &Entry, body: &str) -> Result<(), AxError>;                   // 只写盘
pub fn index(city_root, building) -> Result<Vec<Entry>, AxError>;   // 算出来的，不落盘
```

- **四类封闭**：第五类需要理由，而「它不属于前四类」正是让分类表烂掉的那个理由。不合的东西是笔记，笔记住人已经在读的文档里。
- **index 是算出来的**：存一份就是盘上内容的第二份账，而盘是真的那一份。删掉它没有东西可删——这正是投影该有的样子。
- **召回是结构化的**：按类与日期归档，循索引读**原文**。不做向量记忆；翻案条件写死——真实召回率 <90% 才重议。
- **日期取整天**：给人浏览用，精度高过问题所需只会招来没人打算做的比较。

**决定一条记录是什么，与把它写上架，是两步**。合成一步的 `file(..) -> Entry` 会让调用方拿到 `Entry`（账本行要的 `kind`／`day`／`subject` 全在里面）时文件已经在架上了；账本行只能后落，而 Ledger 的定义是「Every effect becomes an EventRecord first」。拆开之后：

- `entry` 是纯的：拒空 subject、`day_of(at)` 取整天、按 `<building>/Archive/<kind>/<day>-<slug>.md` 算出落点，全部只读入参。**一条记录是什么，在它到达任何地方之前就已经确定**，所以调用方可以先把它落账再把它写上架。
- `file` 只写：经 `city::document` 把正文整份换上去，建目录也由那一处做。它收一个 `&Entry` 而不是六个参数——落点由 `entry` 算过一次，`file` 不再第二次决定它。
- **仍是一个构造点**：`Entry` 的字段没有对外的写面，`index` 那一支是从盘上读回来的另一种来源（`subject_of(&text)` 而不是入参），两者不共用同一条不变式，因此没有第二个权威。
-/
