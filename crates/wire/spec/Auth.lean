-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::auth

规定 `auth`（`crates/wire/src/` 下同名的文件）。配对令牌：铸造、唯一可读的展示形、常数时间比较，以及来者握没握着它。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-3 wire::auth（形状 2 值类型 ＋ 形状 1 判定）

```rust
pub struct PairingToken(B3Hash);               // 只是摘要，不持明文
impl PairingToken {
    pub fn mint(entropy: [u8; 32]) -> (Self, String);  // 右侧即只展示一次的配对码
    pub fn from_configured(raw: &str) -> Result<Self, AxError>;
    pub fn digest(&self) -> B3Hash;                    // 交给 ServeConfig 的全部
}
pub fn verify(presented: Option<&str>, expected: &B3Hash) -> bool;  // 常数时间
```

四条决定：

（a）**熵入参不采样**——使铸造可重演、可测，与「种子 RNG 单点发放」一致。

（b）**`PairingToken` 不持明文**：`expose` 只得出现在兑付点（`xtask secret`），而把展示点加进兑付点白名单是修门不修因。本模块不需要明文，`mint` 把配对码直接交给调用方去展示，自己只留摘要。封一个值再在下一行解封是表演；**根本不持才是我们想要的性质**。

（c）**字母表剔除可混淆字符**（0／O／1／l／I／5／S），29 符号×四组五位≈ 97 位熵。理由不是审美：配对码要被人读出来、在另一台机器上手敲进去，一个口述会错的码，代价由用户在另一台机器前承担。

（d）**分组形兼顾了 secret 门**：每片 5 字节远低于 20 字节阈值，测试里的配对码字面量不会被熵侦测器咬住。
-/
