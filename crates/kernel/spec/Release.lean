-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::release

规定 `kernel::release`（`crates/kernel/src/release.rs`）：一次发布的两种拼法与谁更新。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。
-/

/-!
### 8-54 `kernel::release`：一次发布的两种拼法，以及两次发布谁更新

```rust
pub struct Release { /* major, minor, patch, year, month, day —— 私有 */ }
impl Release {
    pub fn from_tag(tag: &str, expected_version: &str) -> Result<Release, AxError>;
    pub fn from_npm_version(text: &str) -> Result<Release, AxError>;
    pub fn version(&self) -> String;      // 0.0.5
    pub fn tag(&self) -> String;          // v0.0.5-Pre-alpha-260912
    pub fn npm_version(&self) -> String;  // 0.0.5-pre.260912
    pub fn released(&self) -> String;     // 2026-09-12
}
pub enum ReleaseVerdict { Current, Behind, Ahead }
pub fn stands(mine: &Release, newest: &Release) -> ReleaseVerdict;

pub enum Maturity { PreAlpha, Alpha }
impl Maturity {
    pub const fn word(self) -> &'static str;    // pre-alpha：句子里的写法
    pub const fn titled(self) -> &'static str;  // Pre-alpha：tag 与标题里的写法
}
pub const MATURITY: Maturity;                   // 这棵树切出的每一次发布的成熟度
```

`from_tag` 只认中缀为 `-<MATURITY.titled()>-` 的 tag，`tag()` 写的也是这一个中缀；`Maturity` 经 `kernel::Maturity` 重导出。

**五条口径：**

1. **一次发布有两种拼法，而只有这一处同时认识两种。** git tag 写 `v0.0.5-Pre-alpha-260912`；npm 只收 semver，同一次发布因此发成 `0.0.5-pre.260912`。`xtask channel` 按前者转出后者去发布，运行中的二进制按后者读回注册表——两边转换各写一份，就是「这是哪一次发布」有了两个答案。
2. **排序是 semver 自己的。** 日期落在 pre-release 段，于是 `0.0.5-pre.260912` 高于 `0.0.5-pre.260911` 而低于裸的 `0.0.5`——semver 对点分数字标识符按数值比。`Ord` 按字段声明顺序派生即复现该规则，本 crate 与注册表因而对同一对发布给出同一个次序。**这正是本类型存在的理由**：二进制拿自己的裸 `0.0.5` 去比注册表的 `0.0.5-pre.260912`，会把最新的那一版读成更旧的那一版，且无声。
3. **日期必须随版本一起走，不能摆在旁边。** 一个 pre-alpha 的版本号几乎说不出树有多旧，而树有多旧正是它的读者最需要知道的（CHANGELOG.md 开篇）。故 `released()` 是给人读的那一个渲染，`npm_version()` 是给注册表的那一个。
4. **无钟无套接字。** 注册表此刻给的是什么，归调用方去取；本模块只判它被递到的东西（ARCHITECTURE.md 第 1 段）。
5. **成熟度只写在 `MATURITY` 一处。** tag 的中缀、`sprawling status` 版本行里的说法（`crates/sprawling/sprawling-SPEC.md` 8-162）、文档里由 `cargo xtask docnum` 的 `maturity` 事实渲染的字样（tools/xtask/Spec.lean §8-16），都从这个常量读；npm 那一种拼法的 `-pre.` 不随它变（D18）。
-/

namespace Kernel.Release

/-! D18 **成熟度是 kernel 的一个枚举常量，每一个读者从它渲染。** `MATURITY` 是这棵树切出的发布处在哪一级；tag 的中缀、`status` 的说法、README 两份与 CHANGELOG 里的字样都由它渲染，于是进 alpha 只改这一个常量，下面的定理说明每一处渲染都随之变。它是枚举而不是一段字符串：tag 写 `Pre-alpha`、句子写 `pre-alpha`，两种写法由 `Maturity` 的两个方法各给一次，读者不自己改大小写。值住 Rust（`crates/kernel/src/release.rs`），模型对任一取值成立，所以这里是参数而不是定义。
被否：①写成 `Cargo.toml` 版本号的 pre-release 段（`0.0.8-alpha`）——它改的是 semver 次序与 crates.io 上的每一处 `=` 钉子，而成熟度不是版本号的一部分；②npm 那一种拼法也带成熟度——semver 按字典序比 pre-release 标识，`alpha` 排在 `pre` 之前，一个 alpha 的发布在注册表上会被读成比最后一个 pre-alpha 更旧，所以 `-pre.` 留到定 V0.1.0 的那一次改动再定。
重开参数：成熟度要进 npm 的版本串，或一棵树要同时切出两级成熟度的发布。 -/

/-- 与 `kernel::Maturity` 逐变体同名：这个项目站在哪一级。 -/
inductive Maturity where
  | PreAlpha
  | Alpha
  deriving DecidableEq, Repr

/-- 一个成熟度的两种写法：`word` 是句子里的（`status` 的版本行、README 的状态句），`titled` 是 tag 与标题里的。
两种写法的值住 Rust 的 `Maturity::word` 与 `Maturity::titled`，模型把它们当参数，只要求各自分得开两级成熟度。 -/
structure Spelling where
  word : Maturity → String
  titled : Maturity → String
  word_injective : Function.Injective word
  titled_injective : Function.Injective titled

/-- tag 里隔开版本与日期的那一段：`-` 加 `titled` 再加 `-`，即 `Release::tag` 写出、`Release::from_tag` 切开的中缀。 -/
def tagInfix (spelling : Spelling) (maturity : Maturity) : String :=
  "-" ++ spelling.titled maturity ++ "-"

/-- 中缀分得开两级成熟度：一个 tag 只属于一级，按 alpha 切出的 tag 不会被一个 pre-alpha 的构建读成自己的。 -/
theorem a_tag_infix_names_one_maturity (spelling : Spelling) :
    Function.Injective (tagInfix spelling) := by
  intro a b same
  apply spelling.titled_injective
  simpa [tagInfix] using same

/-- 两级成熟度之间的任何一次挪动，都让每一处渲染跟着变：tag 的中缀、句子里的写法、标题里的写法。 -/
theorem moving_the_maturity_moves_every_rendering (spelling : Spelling)
    {before after : Maturity} (moved : before ≠ after) :
    tagInfix spelling before ≠ tagInfix spelling after ∧
      spelling.word before ≠ spelling.word after ∧
      spelling.titled before ≠ spelling.titled after :=
  ⟨fun same => moved (a_tag_infix_names_one_maturity spelling same),
   fun same => moved (spelling.word_injective same),
   fun same => moved (spelling.titled_injective same)⟩

/-- 前提可满足：存在分得开两级的写法，所以上面的定理不是从空前提推出来的。这里的字符串只是见证，不是 Rust 的拼法。 -/
def witness : Spelling where
  word | .PreAlpha => "a" | .Alpha => "b"
  titled | .PreAlpha => "A" | .Alpha => "B"
  word_injective := by intro a b h; cases a <;> cases b <;> first | rfl | (simp at h)
  titled_injective := by intro a b h; cases a <;> cases b <;> first | rfl | (simp at h)

/-- 见证之下，从 pre-alpha 挪到 alpha，tag 的中缀确实变了。 -/
example : tagInfix witness .PreAlpha ≠ tagInfix witness .Alpha :=
  (moving_the_maturity_moves_every_rendering witness (by decide)).1

end Kernel.Release
