-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::identity

规定 `answer::identity`（`crates/wire/src/` 下同名的文件）。城怎样称呼这个人与 Mayor，或身份区读到哪一行停下。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-59 身份的线面：读名字，带基线写，读回当时冻下的那一版

```rust
// Command
PutDocument { which: GovernedDocument, base: String, body: String, idem: IdemKey }
PutIdentity { card: IdentityCard, base: String, idem: IdemKey }
pub enum IdentityCard {
    Person { user_id: Option<String>, imported_from: Option<String>, about: Option<String> }, // 写 PREFERENCES.md
    Mayor { name: Option<String> },                                                         // 写 MAYOR.md
}
// Query
Identity,                                  // → Answer::Identity(Box<IdentityAnswer>)
pub enum IdentityAnswer {
    Stated(StatedIdentity),
    Unreadable { document: GovernedDocument, line: u32, why: String },
}
pub struct StatedIdentity {
    pub user_id: Option<String>,           // 城怎么称呼这个人；缺席时页面用语言表里的「你」
    pub imported_from: Option<String>,     // user_id 是从哪台主机的 gh 导入的；手填为 None
    pub about: String,                     // PREFERENCES.md 身份区之后的正文
    pub mayor: Option<String>,             // 主 Agent 的显示名；缺席时页面用语言表里的默认名
    pub version: B3Hash,                   // 新 session 会冻下的那一版（city-SPEC §8-33）
    pub preferences_text: String,          // 两份文件此刻的全文：保存时的 base
    pub mayor_text: String,
}
```

- **两份文件，一个身份区。** 用户 ID、导入来源与「关于你」住 `PREFERENCES.md`，主 Agent 的名字住 `MAYOR.md`；身份区的语法、名字的合法域与「一个 session 冻下哪一版」都由 `city::Naming` 一处回答（city-SPEC §8-33），本 crate 只携字符串，不判它们合法与否（§8-0 同一条理由）。
- **卡片只写自己那几个键，其余原样。** `PutIdentity` 把卡上的值交给城，由城在 `base` 上改写身份区：卡上的键写入或删去（`None` 即删去，回到默认称呼），身份区里别的键、`about` 为 `None` 时的正文，都按 `base` 里的字节留着。页面不拼 TOML：拼身份区的只有城一处，表单与原文编辑器读到的是同一份解析结果（D5）。
- **两个写者，一道守卫。** 原文编辑器发 `PutDocument`，卡片发 `PutIdentity`，两者都携 `base`——发信方起手时那份全文——文件已经变了就拒 `E_VERSION_CONFLICT`，什么都不写；人的草稿留在页面上，页面重读之后再发。`MAYOR.md` 与 `PREFERENCES.md` 的身份区读不出时（重复的键、没有闭合的 `+++`、名字为空或带控制字符），`PutDocument` 在落盘之前拒 `E_CONFIG_INVALID`，拒因里有行号；`CLERK.md` 没有身份区，只经基线守卫。
- **回执是账本行。** 两条命令都写一行 `governed_document_written`，`naming` 键是写完之后城的身份版本（`crates/kernel/Spec.lean` §8-79）；页面发出之后只显示「保存中」，见到这一行才显示「已保存」，并以它判断自己读到的 `version` 是否已经过时。
- **读的是此刻，跑的是冻下的那一版。** `Query::Identity` 每次从盘上读，所以页面显示的总是新 session 将要冻下的名字。已经开始的 session 用它第一次 run 冻下的版本（accounting-SPEC §8-15）；那一版记在每次 run 的 `run_started.naming` 上，页面拿它经 `Query::Content { locator: cas:<naming> }` 读回当时的名字。旧账本没有这个键，页面就显示地址或语言表里的角色名，不拿今天的名字冒充当时的。
- **读不出就说在哪一行。** 身份区读不出时答 `Unreadable`：哪一份文件、第几行（从文件第一行数起）、为什么。页面据此打开原文编辑器，而不是画一个默认名字再让下一次保存把人的正文盖掉。
- **`WIRE_V` 不另进位**：`PutDocument` 加 `base` 是名字不变的改形，与本批其余改形共用 45（D1）；`PutIdentity`、`Identity` 是新名字，哈希自己会变。
- 验收：city 的 `a_stale_identity_save_is_refused_and_the_draft_survives`（基线过期被拒、文件不变）；accounting 的 `a_new_session_freezes_the_name_the_page_shows`（实际发出的请求上下文与 `Query::Identity` 的答面同名同版本，旧 session 不改名，`/new` 之后两边一起换）。
-/

/-! D5 设置页上的卡片的改写在城里做，页面只交值

**决定**：设置页上的卡片发 `PutIdentity { card, base }`，由 `city::Naming` 在 `base` 上改写身份区并整份落盘；原文编辑器发 `PutDocument { which, base, body }`，城在落盘之前用同一个解析器读一遍身份区（§8-59）。

**理由**：身份区是 TOML，`PREFERENCES.md` 与 `MAYOR.md` 里还有人写的未知键与正文。页面若自己拼整份文件，「身份区怎么写、保留什么、名字合法与否」在 TypeScript 与 Rust 各有一份，两份迟早不一致，而写坏的一份会让下一次开城读不出名字。卡片只交值，拼法就只有城一处；两条命令携同一种 `base`，所以并发保存只有一条规则：后到的那一个被拒。

**被否**：①页面拼出整份文本，只用 `PutDocument`：拼法有第二个家；②卡片带一个版本号而不是全文作 `base`：城要另存版本到全文的对应，而 `PutSpine` 已经用全文作基线，两种基线会让同一个页面写两种守卫。

**重开参数**：身份区长出页面要分块编辑的结构（例如多个账号各一张卡）时，重议卡片是否改为按键寻址。
-/
