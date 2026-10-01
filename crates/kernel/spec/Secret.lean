-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::secret

规定 `kernel::secret`（`crates/kernel/src/secret.rs` 与 `crates/kernel/src/secret/` 下的 `span`、`scan`、`hex_run`、`sealed`）：密钥引用、扫描与封存。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-25 kernel::secret

```rust
pub struct SecretRef { /* realm, name —— 私有 */ }
impl SecretRef { pub fn new(realm: &str, name: &str) -> Result<Self, AxError>;  // 两段的唯一构造点
                 pub fn parse(raw: &str) -> Result<Self, AxError>;   // secret:<realm>/<name>；形状非法＝E_CONFIG_INVALID
                 pub fn realm(&self) -> &str;  pub fn name(&self) -> &str; }
// Display "secret:<realm>/<name>"；serde 字符串形；恒不入 Locator 文法（两解析器分立）

pub struct SecretSpan { pub start: usize, pub len: usize, pub provider: Option<&'static str> }
                                    // provider 来自形状表命中；None＝熵侦测器命中
/// Custody's detection half: shape table first,
/// entropy second. Pure, no regex, no backtracking; kani-provable
/// termination. Replacement/vaulting is the effect layer's.
pub fn scan(bytes: &[u8]) -> Vec<SecretSpan>;

pub struct Sealed<T: zeroize::Zeroize>(/* secrecy::SecretBox<T> */);
impl<T: zeroize::Zeroize> Sealed<T> {
    pub fn new(value: Box<T>) -> Sealed<T>;
    /// Call sites are whitelisted by `xtask secret` (`EXPOSE_WHITELIST`);
    /// the type refuses Debug/Display/Serialize so a
    /// sealed value cannot reach any sink even by accident.
    pub fn expose(&self) -> &T;
}
```

- **`SecretRef` 只有一个构造点**。`secret:<realm>/<name>` 是一段文法，故拼接只在 `SecretRef::new` 里发生一次：`parse` 拆出两段后也交给它，两半因此共用同一个拒词；段允许的字符集（字母、数字、`-`、`_`、`.`）也只写在 `new` 里，名字里的 `/` 由它拒绝而不另立一条规则。`wire` 的录入口、`sprawling` 的签入面与到期表、以及客户端的表单曾各自拼出这段文本再交给 `parse` 读回——一段文法多处拼，改一次就分岔，且拼出来的文本本城可能解析不回来。
- **扫描两侦测器**：①形状表（SECRET_SHAPES：前缀＋字符集＋长度窗）为主；②熵阈为辅——无前缀命中的 token 段（base62/base64url 字符连段，长度 ≥ `ENTROPY_SPAN_MIN_BYTES=20`，pub(crate) 内部事务）且每字符熵 ≥ `SECRET_ENTROPY_MIN`（3.5 bits/char）。两集合并，重叠段归形状命中（provider 信息更多）。
- **hex 段是第三侦测器 `secret::hex_run`**：纯 hex 字母表的段过不了②的混合字母表门，而随机 hex 密钥（HMAC key、以 hex 打印的 token）与本城的 blake3 hex64、git hex40 oid 同为均匀分布，**熵读数分不开二者**——只看熵的阈值要么漏掉 hex 密钥，要么把每行 `git log` 与每个 ledger 哈希都报成密钥。故 hex 段只在它是一个凭据名的值时才报：段前紧邻 `<名字>` + 可选空白与引号 + `=` 或 `:` + 可选空白与引号，且名字过 `names_a_credential`；此外段长 ≥ `HEX_SPAN_MIN_BYTES`（32）且每字符熵 ≥ `HEX_ENTROPY_MIN_MILLIBITS`（3100 millibit）。取舍：放弃了「hex 字母表单独一道熵阈、不看标签」——它在长度 40 上要么阈值高到漏报（均值 3.69 bit），要么把全部哈希报出。重开条件：本城的哈希或 oid 改为不以裸 hex 出现在文本里。
- **熵的整数化**：kernel 禁浮点——香农熵以 millibit（1/1000 bit）计：定点 log2（shift-and-square，10 位小数位，循环界常数）；判式 `mb·den ≥ num·1000`（checked）。kani：任意输入终止、无 panic、无溢出。
- **Sealed 取 secrecy::SecretBox**（`secrecy`＋`zeroize`）：drop 即零化；无 Debug/Display/Serialize/Clone；trybuild 反例＝Sealed 值入 EventRecord/format! 编译不过。`PutSecret` 的命令面（S4）直用本类型。
- 误报是既知常态（入口无损可逆，出口才拒）；`E_SECRET_EGRESS` 的 subject 恒不回显命中字节（塑形在 gate::egress）。
-/
