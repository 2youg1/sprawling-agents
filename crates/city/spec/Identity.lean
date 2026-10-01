-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::identity

规定 `identity`、`identity::area`（`crates/city/src/` 下同名的文件）。两份治理文档的身份区：城怎么称呼人与主 Agent，一个 session 冻下哪一版。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-33 `PREFERENCES.md` 与 `MAYOR.md` 的身份区：城怎么称呼人与主 Agent（`city::identity`，形状 2 值类型＋形状 1 判定）

```rust
pub const NAME_MAX_CHARS: usize = 64;
pub struct DisplayName(String);                 // 一行、去掉首尾空白后非空、无控制字符、不超过 NAME_MAX_CHARS 个字符
impl DisplayName { pub fn parse(raw: &str) -> Result<DisplayName, AxError>; pub fn as_str(&self) -> &str; }

#[derive(Serialize, Deserialize)]
pub struct Naming { /* person、imported_from、about、mayor */ }
impl Naming {
    pub fn read(city_root: &Path) -> Result<Naming, AxError>;        // 读两份文件的身份区
    pub fn person(&self) -> Option<&str>;
    pub fn imported_from(&self) -> Option<&str>;
    pub fn about(&self) -> &str;
    pub fn mayor(&self) -> Option<&str>;
    pub fn to_bytes(&self) -> Vec<u8>;                                // 冻进内容库的那份字节（JSON）
    pub fn from_bytes(bytes: &[u8]) -> Result<Naming, AxError>;
    pub fn version(&self) -> B3Hash;                                  // = B3Hash::digest(to_bytes())
    pub fn context(&self) -> Option<String>;                         // 接在 city 段之后的那一块；什么都没说时为 None
}
pub enum NamingEdit {
    Person { user_id: Option<String>, imported_from: Option<String>, about: Option<String> },
    Mayor { name: Option<String> },
}
pub struct Unreadable { pub document: Governed, pub line: u32, pub why: String }
pub fn read_naming(city_root: &Path) -> Result<Result<Naming, Unreadable>, AxError>;   // 页面读：读不出给出位置
pub fn write_naming(city_root: &Path, edit: &NamingEdit, base: &str) -> Result<Naming, AxError>;
pub fn write_governed(city_root: &Path, which: Governed, base: &str, body: &str) -> Result<PathBuf, AxError>;
pub fn persona(written: &[u8]) -> &[u8];                              // 身份区之后的正文
pub fn freeze_naming(city_root: &Path, room: &Address, version: B3Hash) -> Result<(), AxError>;
impl ConfigLayer { pub fn naming(&self) -> Option<B3Hash>; }          // 房间这一层冻下的身份版本
```

**身份区的语法。** 文件的第一行恰好是 `+++` 时，到下一行恰好是 `+++` 为止是一段 TOML，其后是正文；第一行不是 `+++` 的文件没有身份区，整份都是正文——旧文件照旧读。`PREFERENCES.md` 的身份区读 `user_id` 与 `imported_from` 两个键，正文是「关于你」；`MAYOR.md` 的身份区读 `name`，正文是这位主 Agent 是谁。身份区里别的键不读、也不丢：改写只动卡片上的那几个键。

**拒绝。** 只有一处读身份区，读不出时它说出原文位置：没有闭合的 `+++`（行号是开头那一行）、TOML 本身的错（重复的键、坏的值，行号从文件第一行数起）、一个键的值不是字符串、名字不合 `DisplayName` 的域。码是 `E_CONFIG_INVALID`，主语是文件名与行号，恢复语说改这一行或删掉身份区。读身份区的每个调用方都走这一处，所以开城、派活、页面、保存看到的是同一个判定：读不出时**不**回落成默认名字——那样下一次保存就会把人的正文连同读不出的那一行一起盖掉。

**写。** `write_naming` 在 `base` 上改写身份区：卡片上的键为 `Some` 时写入（先过 `DisplayName`），为 `None` 时删去；`about` 为 `Some` 时换掉正文；身份区删空了就连 `+++` 一起去掉，所以从没设过名字的文件改一次再改回来，逐字节回到原样。落盘经 `document::edit_against`：文件已经不是 `base` 就拒 `E_VERSION_CONFLICT`，什么都不写。`write_governed` 是原文编辑器那扇门，同一道基线守卫；它在落盘之前对 `MAYOR.md` 与 `PREFERENCES.md` 的正文读一遍身份区，读不出就拒。

**进上下文。** `context()` 给出接在 city 段之后的一块：这个人叫什么、关于他的那段正文、主 Agent 叫什么并且就是 `hall/mayor`，最后一句说这些名字来自两份文件的身份区。什么都没说（没有名字、正文为空）时为 `None`，city 段的字节与从前一样。`hall/mayor` 自己的 resident 段以 `Your name: <name>` 开头，没有名字时照旧是地址末段；`MAYOR.md` 进 resident 段的是 `persona` 给出的正文，身份区不进去，因为名字已经由那一行说了，一个名字在一次请求里只有一个说法。

**一个 session 冻一版。** `Naming` 序列化成 JSON 放进内容库，摘要就是 `version()`；session 的第一次 run 把它写进房间自己那一层的 `[identity] version`（`freeze_naming`），之后这个 session 的每次 run 都从内容库读回那一版，所以改名不会在一个 session 中途改掉请求前缀。开新 session（`city::clear_session`、`forget_shape`）连同 `[model]` 一起删去 `[identity]`，下一次 run 冻下此刻的版本。只有身份区与正文进 `Naming`；`MAYOR.md` 正文进 resident 段的那份字节照旧在每次 run 现读。

**验收**：`identity` 的测试：没有身份区的旧文件照读；`a_stale_identity_save_is_refused_and_the_draft_survives`（`base` 过期被拒，文件仍是后写的那一份）；改一次再改回逐字节不变；重复的键与没有闭合的 `+++` 各拒一次并给出行号；未知键在改写后仍在。
-/

/-! D11 身份住在两份治理文档的身份区里，一个 session 在房间那一层冻一版

**决定**：用户 ID 与导入来源写进 `PREFERENCES.md` 开头以 `+++` 围起的 TOML 身份区，主 Agent 的名字写进 `MAYOR.md` 的同类身份区；正文分别是「关于你」与这位主 Agent 是谁（§8-33）。一个 session 的第一次 run 把此刻的身份冻进内容库，摘要记在房间自己那一层 `CONFIG.toml` 的 `[identity] version`。

**理由**：人已经在这两份文件里写自己的背景与主 Agent 的样子，名字放在同一份文件里，原文编辑器与设置卡片改的就是同一份东西，没有第二份可编辑的配置要同步。`+++` 是 TOML 的惯用围栏，与 Markdown 正文分得开，一个只想改正文的人不会误碰它。冻在房间那一层，是因为一个 session 的形状（模型与强度）已经记在那里，`/new` 清掉的也正是这一层；身份跟着同一个边界走，就不必再定一条「哪一次清身份」的规则。

**被否**：①另起一份 `IDENTITY.toml`：同一个人写的同一件事分在两个文件里，原文编辑器里看到的正文与卡片上的名字各说各的；②身份区用 YAML（`---`）：本 crate 只依赖 TOML 的解析器，而 YAML 的隐式类型会把 `no`、`on` 这样的名字读成布尔值；③session 开始时把身份写进 `session_opened`：经 `Dispatch` 打开的房间没有那一行，而第一次 run 是每个 session 都有的那一刻。

**重开参数**：身份要跨城复用（人明确导入另一座城的身份）时，重议身份是否搬到人自己那一层（`~/.sprawling/config.toml`）。
-/
