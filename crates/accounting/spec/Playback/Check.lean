-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::playback::check

规定 playback page 的嵌入、五项复核与导出件落盘：`playback::check`、`playback::page`、`playback::page::sink`、`playback::offline`、`playback::observed`、`playback::landing`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-13 accounting::playback 的页面、分项复核与导出件落盘（形状 1 决策，`page`、`landing` 为形状 4 适配器）

agent 把一份 bundle 做成一个自包含的单文件 HTML，叫 **playback page**。它的数据契约、引用规则、导出流程与验证要求写在随发行包发出的 `skills/playback/SKILL.md`；画面不在本节。本节定三件事：产品怎样把 bundle 放进页面，怎样把一份文件查成五个分开的结论，导出件怎样落盘。CLI（sprawling-SPEC.md §8-126、§8-132）与居民的城工具 `playback`（sprawling-SPEC.md §8-132）都是这些函数的薄适配器。

```rust
// accounting::playback
pub const PAGE_MAX_BYTES: usize = 48 * 1024 * 1024;
pub const BUNDLE_BLOCK: &str = r#"<script type="application/json" id="playback-bundle"></script>"#;
/// 模板里恰好一处 BUNDLE_BLOCK，换成装着 bundle 字节的同一个元素；结果过结构与静态离线两项才交出。
pub fn embed(template: &[u8], bundle: &Bundle) -> Result<Vec<u8>, AxError>;
pub struct City<'a> { pub root: &'a Path, pub reader: Reader }
pub struct Asked<'a> { pub bundle: Option<&'a [u8]>, pub city: Option<City<'a>>, pub observed: Option<&'a [u8]> }
pub enum Verdict { Passed, Failed { found: String }, Unasked { why: &'static str }, Unable { why: String } }
pub struct Report {
    pub file: B3Hash,
    pub digest: Option<B3Hash>,
    pub events: Option<usize>,
    pub structure: Verdict,
    pub bundle: Verdict,
    pub source: Verdict,
    pub offline: Verdict,
    pub browser: Verdict,
    pub covered: Vec<String>,
}
impl Report {
    /// 没有 Failed，也没有 Unable。
    pub fn holds(&self) -> bool;
    /// 一行 JSON：file、digest、events 与五项。
    pub fn line(&self) -> serde_json::Value;
}
/// 不返回错误：每一项读不下去的原因写在它自己的结论里。
pub fn check(file: &[u8], asked: &Asked<'_>) -> Report;
pub enum Place<'a> { Chosen(&'a Path), Exports { city_root: &'a Path, file: &'a Path } }
pub fn land(place: Place<'_>, bytes: &[u8]) -> Result<(), AxError>;
```

模块：`playback::page`（用 html5ever 的树构建器按浏览器的解析算法读页面，交出一张平面元素表；树构建器写进的 sink 在 `playback::page::sink`）、`playback::offline`（静态离线规则）、`playback::observed`（浏览器观察记录）、`playback::landing`（新文件整份落下或不落）；`playback::check` 把五项排在一起。

**一份文件是什么。** 首字节是 `{` 的文件按 bundle 读；其余按 UTF-8 的 HTML 页面读，不是合法 UTF-8 的页面在结构一项失败。

**嵌入。** `embed` 要求模板里恰好一处 `BUNDLE_BLOCK` 这串字节，把 bundle 的规范字节原样放进这对标签之间，再对结果跑结构与静态离线两项；任一不过就以 `E_INVALID_ARGS` 拒绝（action `embed a playback bundle`），subject 是第一条发现，不交出页面。字节原样放进去是安全的：`encode` 已把 `<`、`>`、`&` 写成转义（§8-12），解析器在这个块里遇不到 `</script>`。页面用 `JSON.parse` 读块的文本，u64 仍是十进制字符串，没有一个值经过 JS 的 `Number`。一处之外的写法（零处、两处、标记写在注释或另一个原始文本元素里）都会让结构一项失败，因为结构一项判的是解析器建出来的元素，而不是源文本。

**五项，分开报。** 状态词三个：`passed`、`failed`、`unchecked`。`Unasked` 与 `Unable` 都写作 `unchecked`，区别只在 `why`：前者是没有人要这一项（没给 `--bundle`、`--city`、观察记录，或被查的是 bundle 而不是页面），后者是要了而做不了。`Report::holds` 为真，当且仅当没有 `Failed` 也没有 `Unable`。

| 项 | 名字 | 通过 | 失败 | 未检查 |
|---|---|---|---|---|
| 结构与引用 | `structure` | bundle 自洽（§8-12）；页面另有下面四条 | 任一条不成立 | 从不 |
| 与指定 bundle 一致 | `bundle` | 两份都自洽且逐字节相等 | 第一个不同的段 | 没给另一份（`Unasked`）；另一份或这份读不出 bundle（`Unable`） |
| 来源复核 | `source` | 用入口给的读者重算，逐字节相等 | 第一个不同的段 | 没给城（`Unasked`）；读不出 bundle、版本或读者对不上、城在 cutoff 前结束、城的账读不下去（`Unable`） |
| 静态离线 | `offline` | 下面的规则全部成立 | 第一条发现，另计余下的条数 | 被查的是 bundle（`Unasked`） |
| 浏览器观察 | `browser` | 观察记录说的是这份字节，四张表都空 | 第一项观察到的行为 | 没给记录（`Unasked`）；记录读不懂、说的是另一份字节、没有路径（`Unable`） |

页面的结构另有四条，判的是 `page` 交出的元素表：恰好一个元素的 `id` 是 `playback-bundle`，它是 HTML 命名空间的 `script`、`type` 是 `application/json`、文本是一份自洽的 bundle；全页的 `id` 不重复；每个以 `#` 开头的 `href`（含 xlink 的 `href`）指向页面里一个存在的 `id`，单独一个 `#` 除外；每个带 `data-seq` 的元素，值是十进制 seq，且在 bundle 的 `events` 或 `context` 里。由脚本在运行时画出的链接不在源里，它们由浏览器观察的 `unresolved` 表核对。

**静态离线规则**（`playback::offline`）。规则判的是浏览器解析器会建出的元素——SVG 与 MathML 命名空间里的、`template` 内容里的都算——不是源文本：

1. **CSP 在前。** 第一个 `meta http-equiv="Content-Security-Policy"` 的父元素是 `head`，在它之前建出的元素只有 `html`、`head`、`title` 和不带 `http-equiv` 的 `meta`。它的策略有 `default-src`，`connect-src`、`base-uri`、`form-action` 三条恰好是 `'none'`。页面上每一条 CSP 的每一个值都在这张表里：`'none'`、`'unsafe-inline'`、`'unsafe-eval'`、`'wasm-unsafe-eval'`、`data:`、`blob:`、`'sha256-…'`、`'sha384-…'`、`'sha512-…'`、`'nonce-…'`。主机、`'self'`、`*`、`http:` 一类的 scheme、`'strict-dynamic'`，以及取值不是来源表的指令（`sandbox`、`report-uri`、`report-to` 等）都是发现。
2. **不出现的元素。** HTML 命名空间的 `base`、`form`、`iframe`、`frame`、`frameset`、`object`、`embed`、`portal`、`applet`。`meta` 的 `http-equiv` 只许 `content-type` 与 `content-security-policy`，`refresh` 等都是发现。
3. **URL 属性。** 任何命名空间的 `href`、`src`、`poster`、`action`、`formaction`、`data`、`background`、`cite`、`longdesc`、`manifest`、`ping`、`codebase`、`archive` 与 xlink 的 `href`：按 URL 规范去掉首尾的 C0 控制字符与空格、删掉其中的制表符与换行之后，值以 `#` 开头，或 scheme 是 `data`、`blob`。空值也是发现。`srcset`、`imagesrcset` 一律是发现。
4. **CSS。** `style` 元素的文本、任何元素的 `style` 属性、SVG 与 MathML 元素的其余属性，都用 CSS 语法的分词器读（`cssparser`），逐层进入函数与块：`url()`、`src()` 的参数与 `image-set()`、`-webkit-image-set()` 里的字符串按上面第 3 条判；`@import`、坏的 url token、超过分词器嵌套上限的块都是发现。

静态离线通过，说的只是页面声明的资源与策略：内联 JS 可以给 `location` 赋值、动态建链接，这些路径静态检查看不见。CSP 也不是任意 JS 的沙箱。所以这一项从不说「不联网」；在某些路径下没看到联网，是浏览器观察一项的话。

**浏览器观察**（`playback::observed`）。产品不执行被查的页面，也不打包浏览器。skill 用宿主已有的浏览器或自动化能力打开页面、走它点名的交互路径，把看到的写成一个 JSON（`deny_unknown_fields`，至多 1 MiB）：

```json
{"page":"<页面字节的 BLAKE3，十六进制>","paths":["…"],"requests":["…"],"navigations":["…"],"popups":["…"],"unresolved":["…"]}
```

`requests` 是页面自身之外发出的请求（`data:`、`blob:` 不算），`navigations` 是离开页面的导航，`popups` 是打开的新窗口，`unresolved` 是点了之后什么也没指到的证据链接。`page` 与被查文件的摘要（`Report.file`，复核那一行的 `file`）不同、`paths` 为空，是 `Unable`；四张表都空是 `Passed`，`Report.covered` 是 `paths`；否则 `Failed`，给出第一项。记录是跑浏览器的 agent 自报的：产品核对的是它说的是这一份字节，不核对浏览器真的跑过；它也只说在这些路径下没看到，不说别的路径。

**落盘**（`playback::landing`）。两种落点，一个做法：同一目录写 `<文件名>.partial-<pid>`，`sync_all`，以硬链接落到目标名，删掉暂存文件；任何一步失败都删暂存文件，目标要么整份出现，要么不出现。

- `Place::Chosen(path)` 是人的 `--out`：父目录经 `std::fs::canonicalize` 解开链接之后，路径里任一段是受保护的元数据（`kernel::PROTECTED_METADATA`）则以 `E_OUTSIDE_WRITE_DOMAIN` 拒绝；目标已存在、或目标在某个 git 仓库里且被索引跟踪，以 `E_INVALID_ARGS` 拒绝。被删掉而仍被跟踪的文件名不存在于盘上，落下去却等于改了历史里的那个文件，所以存在与跟踪分开查。
- `Place::Exports { city_root, file }` 是城里的保留导出位置，`file` 在 `CityLayout::playback_exports()` 之下：从城根到 `file` 的每一段经 `storage::WriteTarget::within` 查，链接与 junction 一律拒绝；缺的目录建出来；目标已存在、被跟踪以 `E_INVALID_ARGS` 拒绝；目标在 git 仓库里而没有被忽略，以 `E_OUTSIDE_WRITE_DOMAIN` 拒绝，因为它会进历史。`/.sprawling/` 在城根的 `.gitignore` 里（`crates/city/Spec.lean` §8-21），所以正常的城里这一条成立。
- git 的两问经 `git2` 读：从目标的父目录向上找仓库，找不到就两问都不适用；工作区与目标都先 canonicalize 再求相对路径。

**失败与资源。** `check` 不返回错误；`embed` 与 `land` 的失败是 `AxError`，subject 是第一条发现或目标路径，不交回部分的页面或文件。被查文件的上限是 `PAGE_MAX_BYTES`：bundle 的上限加 16 MiB 留给页面自身与内嵌的字体、图，与 `BUNDLE_MAX_BYTES` 一样是待测初值（§3）。解析一遍建一张平面元素表，常驻量与页面字节同阶；CSS 的嵌套深度由 `cssparser` 的上限截住。
-/

/-! D25 playback page 由产品嵌入、用浏览器的解析算法查、五项分开报，导出件经一个落盘函数写下

(a) 页面用 html5ever 的树构建器读，本模块只写一个记下元素的 sink。理由：静态检查要判浏览器会建出的元素，而 HTML 的分词受树构建影响——`<svg>` 里的 `<style>`、`<title>` 按普通标记读，`<noscript>` 在开着脚本时是原始文本；只有分词器的做法（html5gum 加按标签名切状态）在外来内容里会把一个 `<img src>` 当成样式文本漏过去。html5ever 是 Servo 的解析器，跟着 WHATWG 的解析算法走。代价：markup5ever、tendril、string_cache 等几个包进锁，发行二进制变大，读数归整合者。被否决的做法：正则或字符串扫描（`</script>`、注释里的标记、实体编码都会骗过它）；html5gum（无依赖，但外来内容里与浏览器不一致）；scraper（多带 selectors，且它锁的 cssparser 与这里的版本不同，锁里会有两份）。

(b) CSS 用 `cssparser` 分词，不开默认特性。理由：转义（`u\rl(`）、注释、嵌套函数与坏 url token 只有按 CSS 语法分词才判得对；默认特性只换来更快的字节匹配与颜色表，页面里的 CSS 是千字节级，差别在微秒以下，不值一个过程宏和一张 phf 表。被否决的做法：手写分词器（第二份 CSS 语法）；lightningcss（整个样式引擎）。

(c) 静态规则按「哪类元素与属性能取外部资源或送走读者」整类拒绝，而不是逐个判它的 URL：`form`、`base`、`iframe` 一类不出现，`srcset` 不出现。理由：参考模板与 agent 写的页面都用不着它们，而它们各自的 URL 语义（`srcset` 的逗号、`srcdoc` 继承 CSP、表单的提交目标）要一份份写判定，漏一份就漏一个出口。代价：一个确实想用 `srcset` 的页面写不进来，要改用 `data:` 的单张图。

(d) CSP 只许不取外部来源的值，`connect-src`、`base-uri`、`form-action` 要显式写 `'none'`。理由：`default-src` 只为取资源的指令兜底，不管 `base-uri` 与 `form-action`；`'self'` 在 `file://` 下的含义随浏览器而变；`'strict-dynamic'` 让受信脚本加载任意 URL。被否决的做法：只要求 `default-src 'none'`。

(e) bundle 由产品嵌进页面：模板里留一个空的 `BUNDLE_BLOCK`，`embed` 原样拼进规范字节，再用同一套检查查结果。理由：agent 自己粘贴或经 JS 重写 bundle，u64 会在 `Number` 里丢精度，粘错一个字节结构就失败；拼接由产品做，页面里的字节就是导出的字节。标记写成固定的一串字节而不在解析树里找位置，因为解析器不给源偏移；拼完再解析一遍，标记落在注释里之类的情形由结构一项挡住。被否决的做法：让 agent 照 skill 自己嵌入。

(f) 浏览器观察由 skill 借宿主的浏览器完成，产品只读它写下的记录，并核对记录说的是这份字节。理由：二进制里不带浏览器，也不替人启动一个——前者体积与维护都大，后者要在每台机器上找浏览器、处理它的权限；宿主（城外的 agent 或居民的浏览器工具）本来就有。代价：记录是自报的，产品核对不了浏览器真的跑过，`browser` 一项因此只说「这份记录说在这些路径下没看到」。被否决的做法：随产品打包无头浏览器；产品自己启动系统浏览器。

(g) `check` 交回五项而不返回错误，`Unasked` 与 `Unable` 在 Rust 里分开、对外同写 `unchecked`。理由：一项读不下去（城的账坏了、观察记录读不懂）不该挡住另外几项的结论；而「没人要」与「要了做不了」对退出码的意思不同，前者不算失败，后者算。被否决的做法：一个总结论（把「内容正确」「不联网」混成一个标识）；用错误终止整个检查。

(h) 写新文件的做法从 `bin::main::playback` 搬到 `playback::landing`，人的 `--out` 与居民的导出位置共用；本 crate 因此直接依赖 `git2`，只读索引与忽略规则。理由：两扇门要的是同一个保证（整份落下、不覆盖、失败不留半成品、不进历史），写两份就会在某一份加了检查而另一份没加时分开。`git2` 已是 `storage` 的依赖、同一份 libgit2，按 ARCHITECTURE.md §4 直接用，不为两问开一个端口。被否决的做法：CLI 与工具各写一份；在 `storage` 加一个只为这两问的函数（这两问不属于 checkpoint，也不属于 worktree）。

(i) `Select.lean` 的场景表只写在 Lean 里，`scenes_agree` 证明模型给出表里的结果，Rust 测试从同一个 `.lean` 文件读表、跑生产的 `export`。理由：表只有一份，模型与实现分别对它负责；Lean 输出一份 JSON 再由 Rust 读，要多一个必须与 `.lean` 保持同步的生成物，测试还要先跑一次 Lean。被否决的做法：Lean 生成 JSON 夹具；Rust 里另写一份场景表。
-/
