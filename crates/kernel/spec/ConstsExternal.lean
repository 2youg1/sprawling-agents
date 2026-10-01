-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::consts_external

规定 `kernel::consts_external`（`crates/kernel/src/consts_external.rs`）：外部事实常量与「哪些账本版本读得开」。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-7 kernel::consts_external

外部事实 5 项（改它＝外界变了）：

```rust
pub const CACHE_BREAKPOINTS_MAX: u32 = 4;
pub const PROMPT_CACHE_TTL_SECS: u64 = 300;
pub const EVENT_LOG_V: u32 = 2;                  // EventRecord.v 的唯一来源；2：四种等来的时刻各记各的（§8-4、D10）
pub enum LogVersion { Current, Older, Ahead, NotAVersion }
pub fn readable_log_v(v: u64) -> LogVersion;     // 「哪些账本版本读得开」的唯一权威
pub const L0_TOOLS: [&str; 3] = ["exec", "edit", "status"];
pub struct SecretShape { pub provider: &'static str, pub prefix: &'static str,
                         pub charset: SecretCharset, pub len: (u16, u16) }   // 闭区间
pub enum SecretCharset { Base62, Base64Url, HexLower, UpperBase36 }
pub const SECRET_SHAPES: [SecretShape; N] = [ /* 公开 provider 令牌形状，见 `crates/kernel/Spec.lean` §14 */ ];
```

`SECRET_SHAPES` 是数据不是代码（零分支）；消费者是 `kernel::secret::scan` 与 `xtask secret`。

**`readable_log_v`**：本模块是数据，以及只读这些数据的分类，
理由是这条判定除了 `EVENT_LOG_V` 什么都不读，而它可能待的每一个别处都会成为
「本构建打得开哪些账本」的第二个家。四态穷尽：`Current`＝本构建所写；`Older`＝1 以上、
低于本版本，读得开（账本只追加，旧行仍是它的历史）；`Ahead`＝更新的构建所写，整条拒读
而不部分解读；`NotAVersion`＝低于任何构建写过的首版本（含 `0`），是损坏或外来行而非旧行。
拒读理由由此说版本而不说链，`storage::jsonl::open` 的两个读方对 v0 给同一套说法。
-/

namespace Kernel.ConstsExternal

/-- 一个账本版本在本构建眼里是什么。与 `kernel::LogVersion` 逐变体同名。 -/
inductive LogVersion where
  | Current
  | Older
  | Ahead
  | NotAVersion
  deriving DecidableEq, Repr

/-- 「哪些账本版本读得开」的唯一判定（`readable_log_v`）。`current` 是 `EVENT_LOG_V`，`first` 是任何构建写过的首版本（Rust 的 `EVENT_LOG_V_FIRST`）；两个数都只住 Rust，这里是参数。 -/
def readable_log_v (first current v : Nat) : LogVersion :=
  if v < first then .NotAVersion
  else if current < v then .Ahead
  else if v = current then .Current
  else .Older

/-- 读者打得开的两类：本构建所写，与更早的构建所写。 -/
def LogVersion.opens : LogVersion → Bool
  | .Current => true
  | .Older => true
  | .Ahead => false
  | .NotAVersion => false

/-- **打得开的恰是从首版本到本版本的那一段。** 更新的构建写的行整条拒读而不部分解读，低于首版本的（含 `0`）是损坏或外来的行，而不是旧行。 -/
theorem opens_exactly_from_first_to_current (first current v : Nat) :
    (readable_log_v first current v).opens = true ↔ first ≤ v ∧ v ≤ current := by
  simp only [readable_log_v]
  split
  · simp [LogVersion.opens]
    omega
  · split
    · simp [LogVersion.opens]
      omega
    · split <;> simp [LogVersion.opens] <;> omega

/-- `0` 不是任何构建写过的版本。 -/
theorem zero_is_not_a_version (first current : Nat) (positive : 0 < first) :
    readable_log_v first current 0 = .NotAVersion := by
  simp [readable_log_v, positive]

/-- 本构建所写的版本读作 `Current`，只要首版本不在它之后。 -/
theorem this_build_reads_its_own_version (first current : Nat) (ordered : first ≤ current) :
    readable_log_v first current current = .Current := by
  simp only [readable_log_v]
  split
  · omega
  · simp

end Kernel.ConstsExternal
