-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::check

规定 `check`（`crates/city/src/` 下同名的文件）。一座城的每份 TOML，错处落到行列。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-29 city::check：一座城的每份 TOML，逐份读一遍，错处落到行列（形状 1 判定）

`sprawling check <city>` 的判定半边。它不另写解析：每份文件交给运行时读它的那个解析器（`ConfigLayer::parse`、`policy::evaluate`、`Schedule::parse`、`Watch::parse`），那个解析器的结论就是结论；本模块只再做一件事——解析器拒了，就用同一个文件形状（`ConfigFile`、`Written`、`ScheduleFile`、`WatchFile`，`pub(crate)`）再读一次，取 `toml` 报的 span，折成行列。

```rust
// city::check
pub struct Position { pub line: usize, pub column: usize }   // 都从 1 数；列按字符数，不按字节
pub struct Finding { pub path: PathBuf, pub at: Option<Position>, pub error: AxError }
pub struct Report { pub read: usize, pub findings: Vec<Finding> }  // read：存在并读到的文件数
pub fn check(city_root: &Path) -> Result<Report, AxError>;
```

- **读哪些文件**：城层 `CONFIG.toml`；`building::all` 列出的每栋楼的楼层 `CONFIG.toml` 与 `RULES.toml`；`room::all` 列出的每个房间的居民层 `CONFIG.toml`；`SCHEDULE.toml`、`WATCH.toml`。不存在的文件不是错——城没写它就是默认。
- **每份文件至多一条 Finding**：解析器在第一处错停下，check 不比它多说。
- **`at` 为 `None`**：文件形状读得过、错出在形状之后的判定（一个不是 Address 的写前缀、一个空的 match），`toml` 没有 span 可给，于是不编一个位置。
- **失败**：列楼、列房间、读文件的 I/O 失败原样上传（`E_STORAGE_FATAL` 一族），因为那时「这座城有没有错」答不出来。

**决定**：位置从同一形状的二次读取来，而不是给 `AxError` 加一个位置字段——`AxError` 是全仓的错误形状，为一个只读动词给它加字段，每个构造它的地方都要多想一件事；二次读只在已经出错时发生，成功的读仍是一次。
-/
