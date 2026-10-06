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
    pub fn from_published_tag(tag: &str) -> Result<Release, AxError>;
    pub fn from_npm_version(text: &str) -> Result<Release, AxError>;
    pub fn version(&self) -> String;      // 0.0.5
    pub fn tag(&self) -> String;          // v0.0.9-Alpha-261004
    pub fn npm_version(&self) -> String;  // 0.0.5-pre.260912
    pub fn released(&self) -> String;     // 2026-09-12
}
pub enum ReleaseVerdict { Current, Behind, Ahead }
pub fn stands(mine: &Release, newest: &Release) -> ReleaseVerdict;

pub enum Maturity { PreAlpha, Alpha }
impl Maturity {
    pub const fn word(self) -> &'static str;    // alpha：句子里的写法
    pub const fn titled(self) -> &'static str;  // Alpha：tag 与标题里的写法
}
pub const MATURITY: Maturity;                   // 这棵树切出的每一次发布的成熟度，现为 Alpha
```

`from_published_tag` 读取发布列表中 Alpha 与 PreAlpha 两种历史中缀，仍调用同一 `decode_tag` 验证版本与日期；这不改变构建身份检查，`from_tag` 只认中缀为 `-<MATURITY.titled()>-` 的 tag，`tag()` 写的也是这一个中缀；`Maturity` 经 `kernel::Maturity` 重导出。

**五条口径：**

1. **一次发布有三种拼法，而只有这一处同时认识它们。** crates.io 的裸版本号是第三种（§8-54-1）。 一次 pre-alpha 发布的 git tag 写 `v0.0.5-Pre-alpha-260912`（alpha 写 `-Alpha-`，第 5 条）；npm 只收 semver，同一次发布因此发成 `0.0.5-pre.260912`。`xtask channel` 按前者转出后者去发布，运行中的二进制按后者读回注册表——两边转换各写一份，就是「这是哪一次发布」有了两个答案。
2. **排序是 semver 自己的。** 日期落在 pre-release 段，于是 `0.0.5-pre.260912` 高于 `0.0.5-pre.260911` 而低于裸的 `0.0.5`——semver 对点分数字标识符按数值比。`Ord` 按字段声明顺序派生即复现该规则，本 crate 与注册表因而对同一对发布给出同一个次序。**这正是本类型存在的理由**：二进制拿自己的裸 `0.0.5` 去比注册表的 `0.0.5-pre.260912`，会把最新的那一版读成更旧的那一版，且无声。
3. **日期必须随版本一起走，不能摆在旁边。** 一个 0.0.x 的版本号几乎说不出树有多旧，而树有多旧正是它的读者最需要知道的（CHANGELOG.md 开篇）。故 `released()` 是给人读的那一个渲染，`npm_version()` 是给注册表的那一个。
4. **无钟无套接字。** 注册表此刻给的是什么，归调用方去取；本模块只判它被递到的东西（ARCHITECTURE.md 第 1 段）。
5. **成熟度只写在 `MATURITY` 一处。** tag 的中缀、`sprawling status` 版本行里的说法（`crates/sprawling/Spec.lean` §8-162）、文档里由 `cargo xtask docnum` 的 `maturity` 事实渲染的字样（tools/xtask/Spec.lean §8-16），都从这个常量读；npm 那一种拼法的 `-pre.` 不随它变（D18）。
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

/-! ### 8-54-1 crates.io 上的第三种拼法，以及三种拼法给出同一个次序

```rust
pub struct Version { /* major, minor, patch —— 私有 */ }   // 只有版本号、没有日期的一次发布
impl Version { pub fn from_crates_version(text: &str) -> Result<Version, AxError>; }
pub fn stands_on_versions(mine: &Version, newest: &Version) -> ReleaseVerdict;
pub fn stands_on_crates(mine: &Release, newest: &Version) -> ReleaseVerdict;
```

- **crates.io 收的是裸版本号**：工作区的 `[workspace.package] version`（`0.0.9`）原样发上去，日期不在里面；同一次发布在 npm 上是 `0.0.9-pre.261004`。`from_crates_version` 只认三个点分数字，拒法与 `from_npm_version` 的版本那一半是同一套（`assemble` 的前半），所以两种读法不会一个收、一个拒。
- **只比版本号**：没有发行 tag 的 Cargo 源码安装经 `stands_on_versions` 比较编译版本与注册表版本，日期为空；没有安装身份的开发构建仍然不比较。`stands_on_crates` 按 `mine` 的版本号对 `newest` 判，日期不参与。拿注册表的裸 `0.0.9` 去和自己的 npm 拼法 `0.0.9-pre.261004` 按 semver 比，会把同一次发布读成「注册表更新」；下面的 `a_bare_version_outranks_its_own_npm_spelling` 说这个陷阱对每一次发布都成立，所以两种拼法之间永远不直接比。
- **预发布的「更新」**：在 npm 上，版本号相同、日期更晚的那一次更新，因为日期落在 pre-release 段、按数值比；在 crates.io 上，同一版本号只能发一次，日期更晚的重切发不上去，所以 crates.io 对同一版本号恒答 `Current`。这对用 cargo 装的人是真话：`cargo install sprawling --locked` 本来就取不到一次只改了日期的重切。
- **拼回去**：crates.io 的拼法就是 `Release::version()`（`0.0.9`），不另设一个返回同值的函数；`Version` 不在 kernel 根上导出，因为根上的 `kernel::Version` 是另一个概念，调用方写 `kernel::release::Version`。
- **平台**：只读字符串，Windows、macOS、Linux 一致；向 crates.io 的那一次 HTTPS GET（带 User-Agent）归调用方（口径 4），与问 npm 用同一个 reqwest 客户端。打印的更新命令随安装方式（npm、cargo、压缩包、源码）而变，不随平台变。
- **W6 的派生检查**（`crates/kernel/src/release.rs` 的测试）：proptest 在 `0..=9` 的三个版本数与合法日期上抽两次发布 `a`、`b`，断言：版本号不同时 `stands_on_crates(a, &b_version)` 与 `stands(a, b)` 相等；版本号相同时前者是 `Current`；把两次发布的 `npm_version()` 按 semver 第 11 条（测试里手写的标识比较）排出的次序与 `Release` 的 `Ord` 相同。坏的变体：`stands_on_crates` 把 `newest` 补成日期为零的 `Release` 再交给 `stands`，版本号相同的每一对都红成 `Ahead`。
-/

/-- 一个版本号：三个点分数字。 -/
structure Version where
  major : Nat
  minor : Nat
  patch : Nat
  deriving DecidableEq

/-- 版本号的次序：逐段按数值比。 -/
def Version.cmp (a b : Version) : Ordering :=
  (compare a.major b.major).then ((compare a.minor b.minor).then (compare a.patch b.patch))

/-- 一次发布：版本号与日期。`yy` 是 tag 与 npm 拼法里的两位年份；Rust 存四位年份，两者差一个常数，次序相同。三个数各小于 100，正是六位日期的形状。 -/
structure Release where
  version : Version
  yy : Nat
  month : Nat
  day : Nat
  yy_short : yy < 100
  month_short : month < 100
  day_short : day < 100

/-- `Release` 的派生 `Ord`：先版本号，再年、月、日。 -/
def Release.cmp (a b : Release) : Ordering :=
  (a.version.cmp b.version).then
    ((compare a.yy b.yy).then ((compare a.month b.month).then (compare a.day b.day)))

/-- semver 的一个预发布标识：数字，或字母数字的词（以它在 ASCII 字典序里的位次记）。 -/
inductive Ident where
  | num (value : Nat)
  | word (rank : Nat)

/-- semver 2.0.0 第 11 条：数字按数值比，词按字典序比，数字低于词。 -/
def Ident.cmp : Ident → Ident → Ordering
  | .num a, .num b => compare a b
  | .num _, .word _ => .lt
  | .word _, .num _ => .gt
  | .word a, .word b => compare a b

/-- 预发布段逐个标识比；前面都相等时，短的那一段低。 -/
def preCmp : List Ident → List Ident → Ordering
  | [], [] => .eq
  | [], _ :: _ => .lt
  | _ :: _, [] => .gt
  | a :: rest, b :: others => (a.cmp b).then (preCmp rest others)

/-- 一个 semver 串：版本号与预发布段（空表即没有预发布段）。 -/
structure SemVer where
  version : Version
  pre : List Ident

/-- semver 的次序：先版本号；版本号相同时，没有预发布段的高于有的，两边都有则逐标识比。 -/
def SemVer.cmp (a b : SemVer) : Ordering :=
  (a.version.cmp b.version).then
    (match a.pre, b.pre with
     | [], [] => .eq
     | [], _ :: _ => .gt
     | _ :: _, [] => .lt
     | p :: ps, q :: qs => preCmp (p :: ps) (q :: qs))

/-- npm 那一种拼法：`<version>-pre.<YYMMDD>`。`pre` 这个词的位次是参数，下面的定理对任何位次成立。 -/
def npm (pre : Nat) (r : Release) : SemVer :=
  ⟨r.version, [.word pre, .num (r.yy * 10000 + r.month * 100 + r.day)]⟩

/-- crates.io 那一种拼法：裸版本号。 -/
def crates (r : Release) : SemVer := ⟨r.version, []⟩

theorem then_eq_eq {o p : Ordering} : o.then p = .eq ↔ o = .eq ∧ p = .eq := by
  cases o <;> cases p <;> simp [Ordering.then]

theorem Version.cmp_eq_iff (a b : Version) : a.cmp b = .eq ↔ a = b := by
  cases a; cases b
  simp [Version.cmp]

/-- 六位日期按数值比，等于按年、月、日逐段比：月与日各是两位，所以前一段的差不会被后一段抵掉。 -/
theorem six_digits_compare_like_the_date {y m d y' m' d' : Nat}
    (hm : m < 100) (hd : d < 100) (hm' : m' < 100) (hd' : d' < 100) :
    compare (y * 10000 + m * 100 + d) (y' * 10000 + m' * 100 + d') =
      (compare y y').then ((compare m m').then (compare d d')) := by
  rcases Nat.lt_trichotomy y y' with h | rfl | h
  · rw [Nat.compare_eq_lt.mpr h, Nat.compare_eq_lt.mpr (by omega)]; rfl
  · rw [Nat.compare_eq_eq.mpr rfl]
    rcases Nat.lt_trichotomy m m' with h | rfl | h
    · rw [Nat.compare_eq_lt.mpr h, Nat.compare_eq_lt.mpr (by omega)]; rfl
    · rw [Nat.compare_eq_eq.mpr rfl]
      rcases Nat.lt_trichotomy d d' with h | rfl | h
      · rw [Nat.compare_eq_lt.mpr h, Nat.compare_eq_lt.mpr (by omega)]; rfl
      · rw [Nat.compare_eq_eq.mpr rfl, Nat.compare_eq_eq.mpr rfl]; rfl
      · rw [Nat.compare_eq_gt.mpr h, Nat.compare_eq_gt.mpr (by omega)]; rfl
    · rw [Nat.compare_eq_gt.mpr h, Nat.compare_eq_gt.mpr (by omega)]; rfl
  · rw [Nat.compare_eq_gt.mpr h, Nat.compare_eq_gt.mpr (by omega)]; rfl

/-- npm 按 semver 排两次发布，与 `Release` 的派生次序逐对相同：这是 `from_npm_version` 读回来之后能直接比的理由。 -/
theorem npm_orders_like_the_release (pre : Nat) (a b : Release) :
    (npm pre a).cmp (npm pre b) = a.cmp b := by
  have dated := six_digits_compare_like_the_date (y := a.yy) (y' := b.yy)
    a.month_short a.day_short b.month_short b.day_short
  have word_eq : compare pre pre = .eq := Nat.compare_eq_eq.mpr rfl
  simp only [SemVer.cmp, npm, Release.cmp, preCmp, Ident.cmp, word_eq, dated]
  cases (compare a.yy b.yy).then ((compare a.month b.month).then (compare a.day b.day)) <;> rfl

/-- crates.io 按 semver 排两次发布，只看版本号。 -/
theorem crates_orders_by_version (a b : Release) :
    (crates a).cmp (crates b) = a.version.cmp b.version := by
  simp only [SemVer.cmp, crates]
  cases a.version.cmp b.version <;> rfl

/-- 两个注册表对每一对版本号不同的发布给出同一个次序。 -/
theorem the_registries_agree_when_the_versions_differ (pre : Nat) (a b : Release)
    (differ : a.version ≠ b.version) :
    (npm pre a).cmp (npm pre b) = (crates a).cmp (crates b) := by
  rw [npm_orders_like_the_release, crates_orders_by_version]
  have not_eq : a.version.cmp b.version ≠ .eq :=
    fun same => differ ((Version.cmp_eq_iff _ _).mp same)
  simp only [Release.cmp]
  cases h : a.version.cmp b.version with
  | lt => rfl
  | eq => exact absurd h not_eq
  | gt => rfl

/-- 版本号相同时 crates.io 说不出谁新：它的次序是 `eq`，而 npm 的次序是日期的。 -/
theorem crates_cannot_tell_two_cuts_of_one_version (a b : Release)
    (same : a.version = b.version) : (crates a).cmp (crates b) = .eq := by
  rw [crates_orders_by_version, same]
  exact (Version.cmp_eq_iff _ _).mpr rfl

/-- 陷阱：同一次发布的裸版本号，按 semver 恒高于它自己的 npm 拼法。所以二进制从不拿 crates.io 的串与自己的 npm 拼法直接比。 -/
theorem a_bare_version_outranks_its_own_npm_spelling (pre : Nat) (r : Release) :
    (crates r).cmp (npm pre r) = .gt := by
  have self_eq : r.version.cmp r.version = .eq := (Version.cmp_eq_iff _ _).mpr rfl
  simp only [SemVer.cmp, crates, npm, self_eq]
  rfl

/-- 一次发布对注册表上最新的那一次站在哪里，与 `kernel::ReleaseVerdict` 逐变体对应。 -/
inductive ReleaseVerdict where
  | Current
  | Behind
  | Ahead
  deriving DecidableEq

/-- `stands` 与 `stands_on_crates` 共用的那一步：一个次序读成一个判词。 -/
def verdictOf : Ordering → ReleaseVerdict
  | .eq => .Current
  | .lt => .Behind
  | .gt => .Ahead

/-- D35 **crates.io 的答案只按版本号判，与 npm 的答案在每一对版本号不同的发布上相同。**
`stands_on_crates(mine, newest)` 是 `verdictOf (mine.version.cmp newest)`，`stands(mine, newest)` 是 `verdictOf (mine.cmp newest)`；本定理说两者在版本号不同时逐对相同，`crates_cannot_tell_two_cuts_of_one_version` 说版本号相同时前者恒 `Current`。crates.io 的串经 `Version::from_crates_version` 读成 `Version`，不读成 `Release`：它没有日期，补一个日期就是替注册表编一个它没说的值。
被否：①把 crates.io 的串读成日期为零的 `Release` 再交给 `stands`——版本号相同时恒答 `Ahead`，一个刚从 crates.io 装上的二进制会说自己比注册表新；②拿 crates.io 的串与自己的 npm 拼法按 semver 比——见 `a_bare_version_outranks_its_own_npm_spelling`，恒答注册表更新；③只问 npm、把 npm 的答案给 cargo 用户——npm 上一次只改日期的重切，cargo 装不到，却会让 cargo 用户被告知落后。
重开参数：工作区的版本号带上预发布段（例如 `0.0.9-pre.1`），或 crates.io 允许同一版本号重发。 -/
theorem crates_and_npm_give_one_verdict (pre : Nat) (mine newest : Release)
    (differ : mine.version ≠ newest.version) :
    verdictOf (mine.version.cmp newest.version) =
      verdictOf ((npm pre mine).cmp (npm pre newest)) := by
  rw [the_registries_agree_when_the_versions_differ pre mine newest differ,
    crates_orders_by_version]

end Kernel.Release

/-! ### AUR 归档布局接口

本节描述接口，不是形式证明。`kernel::release::AUR_PACKAGE_NAME` 是 AUR 包名的
唯一权威，`aur_install_directory() -> String` 从它生成相对发行根目录的 POSIX 路径，
`is_aur_install(exe: &Path) -> bool` 判断 exe 的父目录是否以后者为后缀。
包名与布局由生成器的交付规则决定，放在既有 release 模块供 xtask 与运行时共同读取，
避免工具目录成为产品依赖，也避免两个消费者各自决定安装路径。
PKGBUILD、.SRCINFO 和运行时来源识别都读取这些接口；xtask fixture 从生成的
PKGBUILD 链接目标提取 exe 路径并交给同一识别接口，验收跨消费者的一致性。
-/
