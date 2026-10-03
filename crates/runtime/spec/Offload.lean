-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::offload

规定 `offload`（`crates/runtime/src/` 下同名的文件）。共用的缩短原语：有损而可还原，四条不变量。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::offload` 旁的测试守住。
-/

/-!
### 8-8 runtime::offload（形状 1；四不变量的独占定义处）


```rust
pub const REST_DIR: &str = ".rest";
pub struct OffloadSite<'a> { pub cas: &'a mut storage::Cas, pub city_root: &'a std::path::Path,
                             pub room: &'a kernel::Address, pub origin: storage::BlockOrigin }   // origin：这块字节替哪个 run、哪栋楼写下（`crates/storage/Spec.lean` §8-3）
pub struct OffloadRecord { pub substitute: Vec<u8>, pub original: Locator, pub rest_path: String,
                           pub original_len: u64 }
pub fn offload(bytes: &[u8], cap_bytes: u64, site: &mut OffloadSite<'_>) -> Result<OffloadRecord, AxError>;
pub fn rematerialize(locator: &Locator, site: &mut OffloadSite<'_>) -> Result<String, AxError>;
```

- `rest_path` 是 rest 文件的城市地址 `<房间地址>/.rest/rest-<hash 尾 16 位>.dat`：以城市根为基、正斜杠、不含 `.` 段，每台机器上逐字节相同。模型的读工具先用 `Address::parse` 收下模型给的路径、再从城市根解析，所以这个地址它能直接读；站点因此携带房间的 `Address` 而不是一条房间 `Path`，物理位置由 `city_root` 与房间地址拼出，地址只有这一种拼法。`./` 起头的房间相对地址会被 `Address::parse` 以 `E_INVALID_ARGS` 拒掉；OS 绝对路径既会把一台机器的家目录写进账本，又让同一颗种子在两台机器上重放出不同的窗口。`REST_DIR` 是这个目录名的唯一定义处，目录由物化时建出。

- 四不变量逐条入断言：①先存后缩（入参恒为全量字节，cas.put 先于一切裁剪）；②替代体含提示句恒 ≤ 原件且 ≤ cap（提示句字节先扣）；③只有有损才存（调用者保证 len>cap 才进来；函数内再断言，违反＝E_INVALID_ARGS）；④替代体恒携 rest_path：物化只读文件于房间的 `REST_DIR`，内容＝全量原件；命中既有 CAS 对象即直引（幂等）。
- 替代体形：头部字节＋`\n[offloaded: total N bytes; rest at <rest_path>; original <locator>]`；提示句 ASCII。
- rematerialize：rest_path 被外部清理后自 CAS 重建，字节一致（A7 第三断言）。
-/
