-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::format

规定 `format`（`crates/city/src/` 下同名的文件）。本文件是 `crates/city/Spec.lean` 的一个分部；下面一节保留它在 city 规格里的标签 §8-42，别处引作 `crates/city/Spec.lean §8-42`，决定引作 `city D28`。

这一分部只有文字：它是说明文档，不是形式规格。阶梯是一个对单份文件的纯函数（读出格式、比较、逐级改写），没有并发与跨调用的状态，三种结局由 `format::tests` 的轨迹测试逐一走到。
-/

/-!
### 8-42 city::format：配置文件的 `format`，以及把旧文件带到当前格式的阶梯（形状 2 值类型＋形状 4 适配器）

```rust
pub const FORMAT_KEY: &str;                       // "format"
pub enum FormatFile { PersonConfig, LayerConfig, Rules }   // 穷举；新增一种被版本化的文件就加一臂
pub struct Format(NonZeroU32);                    // 恒 ≥ 1
impl Format {
    pub const FIRST: Format;                      // 没写 format 的文件就是它
    pub const fn get(self) -> u32;
}
impl FormatFile {
    pub const fn current(self) -> Format;         // 本二进制读写的格式；今天三种都是 FIRST
    pub fn stated(self, text: &str) -> Result<Option<Format>, AxError>;   // 只读 format 一键；不是 TOML 答 None，留给读者用自己的话拒
}
pub fn admit_format(file: FormatFile, text: &str) -> Result<(), AxError>;
pub struct Rung { pub file: FormatFile, pub from: Format, pub rewrite: fn(&str) -> Result<String, AxError> }
pub enum Forwarded { Absent, Current, Moved { from: Format, to: Format, backup: PathBuf } }
pub fn bring_forward(path: &Path, file: FormatFile, backups: &Path) -> Result<Forwarded, AxError>;
pub struct Forwarding { pub path: PathBuf, pub outcome: Result<Forwarded, AxError> }   // outcome 只会是 Moved 或拒
pub fn bring_city_forward(city_root: &Path, backups: &Path) -> Result<Vec<Forwarding>, AxError>;
```

- **没写 `format` 就是 1，未动的文件不写 `format`。** 上一版读 `CONFIG.toml` 与 `RULES.toml` 时把 `format` 当未知键整份拒；一份不需要改的文件被写进 `format`，人退回上一版时就读不动它。所以 `bring_forward` 遇到已是当前格式的文件一个字节都不写，`city::document` 的其他写门也不添这一键。
- **比当前新的整份拒，先于其他键。** 每个读者（`ConfigLayer::parse`、`policy::evaluate`、`accounting::person`）在按自己的结构解析之前调用 `admit_format`：否则新版本加的第一个键会被当成拼错的键拒掉，人读到的是「unknown field」而不是「请升级」。拒词 `E_CONFIG_INVALID`，subject「written by a newer sprawling (format N; this build reads up to M)」，恢复语「upgrade sprawling to the version that wrote this file」。账本的同类拒绝（`E_LOG_VERSION_UNSUPPORTED`，`crates/storage/spec/Jsonl.lean` §8-1）措辞一致。文本连 TOML 都不是时 `admit_format` 放行、`bring_forward` 答 `Current` 不动它：那份文件由它自己的读者拒，拒词点名文件和谁能改它，换成这里的通用拒词就丢了这两样。
- **已知格式内的未知键照旧拒。** `format` 在结构里声明成一个不读的字段，于是 `deny_unknown_fields` 仍覆盖其余每一键。
- **比当前旧的由阶梯前移，读者不前移。** `bring_forward` 逐级套 `Rung`：从 `from` 找 `file` 相同、`from` 相同的那一级，改写成下一格式的整份文本；全部套完先把原字节写进 `backups`，再经 `city::document` 整份替换。读者遇到比当前旧的文件拒并要求重开城，因为前移要写备份、要拿文件的锁，不属于一次读。
- **开城时整城前移，一份一份地答。** `bring_city_forward` 对 `city::check`（§8-29）列出的每份被版本化的文件——城层、每栋楼、每个房间的 `CONFIG.toml` 与每栋楼的 `RULES.toml`——调 `bring_forward`，只答被改写的与被拒的，不在的与已是当前格式的不答。文件清单只有 `check` 一处权威：两处各列一遍，新加一层配置时就会有一处漏掉。一份文件被拒不挡其余文件前移，因为那份文件下次被读时由它的读者拒，而其余文件不该陪它停在旧格式；列不出楼或房间（目录读不动）才整体失败。调用方是 `RunWorker::holding`，失败与改写都只记进诊断、不挡开城（`crates/sprawling/spec/Accounting/Worker.lean` §8-108）。阶梯为空时它只读不写。
- **备份集中放。** `backups` 由调用方给出，生产上是 `accounting::home::Home::backups()`（`~/.sprawling/backups/`）。文件名是「所在目录的 BLAKE3 前 16 hex」-「文件名」.bak-「原格式」：每栋楼都有 `RULES.toml`，每一层都有 `CONFIG.toml`，一个目录装下全部要靠目录摘要分开。备份以 `create_new` 写；同名已在且字节相同即视为已备，字节不同则拒（`E_STORAGE_FATAL`），不覆盖。
- **怎样加一个会被上一版拒的新键。** 后续变更给 `CONFIG.toml` 或 `RULES.toml` 加键时，把那种文件的 `current` 提一级并登记一级 `Rung`（通常只把 `format` 改大），这样上一版之后的版本读到它时拒的理由是「newer」而不是「unknown key」。人层 `config.toml` 只读 `[ui]`、`[core]` 与顶层 `city`，其余节与顶层键原样保留、不拒，加新节不必提级。
-/

/-! D28 配置的版本是文件里的一个数，缺省为 1，阶梯只在需要时改写

**决定**：每份配置文件可带顶层 `format = N`；没写即 1；比本二进制新的整份拒并点名升级；比本二进制旧的在开城时由 `bring_forward` 逐级前移，原字节先进 `~/.sprawling/backups/`；已是当前格式的文件不写。

**理由**：「读旧配置」与「拼错的键报错」要同时成立。只靠 `deny_unknown_fields`，新版本加一个键就让上一版的人读到一句看不出原因的拒词；去掉它，拼错的键又静默失效。一个文件内的数把两件事分开：数相同才判键，数不同先判方向。缺省为 1 而不是要求每份文件都写，是因为上一版会拒这个键，而人可能退回上一版。

**被否**：把版本记在城的一处清单里——人手工拷来一份旧楼的 `RULES.toml` 时，清单与文件说的不是一回事；每次写都写上 `format`——上一版从此读不动人没改过的文件。

**重开参数**：上一版本身也认得 `format` 之后（本版起），未动文件不写 `format` 这一条可以放宽为「写时总带当前格式」。
-/
