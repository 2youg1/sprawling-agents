-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# views/parts：交互契约

本文件是 `client/Spec.lean` 的一个分部，规定 `client/src/views/parts/` 每个部件欠使用者什么：它遵循哪个 WAI-ARIA 模式、每个键做什么、焦点还给谁、`aria-*` 取什么值。下面各节保留标签 §7、§7-n，别处引作 `client/Spec.lean §7-n`；走动与焦点还原的状态机在本文件末尾，各复合部件的模型在 `client/spec/Views/Parts/` 下。
-/

/-!
## §7 `views/parts/` 的交互契约

> **这是规格，不是描述。** 表里写的是部件欠使用者什么；今天的代码欠而未还的十二处，逐条点名在 7-8。模式名与键表借鉴自哪几份文档、为什么不产生许可证义务，一处记在 `docs/third-party.md` §6，本节不复述。

**判定：一个库只在它替换掉一样东西、并且同一变更集里有生产读者时才进来（由人定）。** `tools/xtask/src/npm.rs` 的 `RUNTIME` 是这条判定的机器面——运行时依赖恰为那张表所列，名单以表为准；加一项是门机制的一次提交，与引入它的变更集分开，好让评审看见门为什么动。**平台先来**：`parts/dialog.svelte` 把模态整个交给原生 `<dialog>`（top layer、焦点陷阱、Esc、其余页面 `inert`，见设计 4-20），`parts/tip.svelte` 把 `title` 换成一个 `role="tooltip"` 的兄弟节点（设计 4-18）；平台已经提供的行为不再引一个库来提供第二遍，因为同一件事两个提供者，第一次分歧就落在键盘用户身上。今天名单上的界面库有两项：`@lucide/svelte`，它替换了 `parts/glyph.ts` 手画的那张路径表，换来人在别的软件里已经认得的图标（`docs/frontend-method.md` §4-34）；CodeMirror 6 的五个包，它替换了 RefRain 原本要手写的编辑面与行级 diff（D23）；pdf.js 与 docx-preview，它们替换了本客户端画不出的 PDF 页与 DOCX 页（D32）。引入的条件写在 7-9。
-/

/-!
### §7-1 不收键的部件

这些部件不进 Tab 序列、不读键，只欠一个角色和一组确切的 `aria-*`。

| 部件 | 角色 | `aria-*` 的确切取值 |
|---|---|---|
| `badge.svelte` | 无（行内文本） | 圆点 `aria-hidden="true"`；状态由词承担，颜色只重复那个词 |
| `banner.svelte` | 实时区域 | `weight="alert"` → `role="alert"`；其余 → `role="status"` |
| `notice.svelte` | 实时区域 | 同上；`seat` 只改画法（浮起或列在中心），不改角色。拒绝形（`action`／`code`／`subject`／`recovery`）的 `weight` 由调用方给；页面对一次落空按键的回答是 `heading`／`next` 两个 `lang.json` 键，没有码、没有折叠，取 `info`，所以是 `role="status"` |
| `empty.svelte` | 无 | 形状 `aria-hidden="true"`；那句话与那个动作是它全部的可读内容 |
| `progress.svelte` | `role="progressbar"` | `aria-label` 取调用方给的名字；`aria-valuemin="0"` 恒在；`total > 0` 时 `aria-valuemax="<total>"`、`aria-valuenow="<done>"`、`aria-busy="false"`，`total ≤ 0` 时这两个值一个都不写并 `aria-busy="true"` |
| `skeleton.svelte` | `role="status"` | `aria-label`、`aria-busy="true"`；每根条 `aria-hidden="true"` |
| `row.svelte` 的 `Row` | 无 | `onOpen` 在场时两段文字合成一个 `<button>`，右侧动作各自是独立的一站；行本身不收键，走动由 `RowList` 承担（7-4）|
| `kbd.svelte` 的 `Kbd` | 无 | 一个字形一个 `<kbd>`，不取焦、不收键 |
-/

/-!
### §7-2 一次一个动作的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `button.svelte` | APG Button | Enter | 激活；`state() !== "idle"` 时 `onClick` 原地返回 |
| | | Space | 同 Enter：平台把两个键都送进同一个 `onClick`，所以一次判定挡住指针、Enter 与 Space 三种输入 |
| `path.svelte` 的显示控件 | APG Button | Enter／Space | 地址解析得出时发 `reveal`；解析不出时 `aria-disabled="true"`，点击落进一个空操作 |
| `field.svelte` | 有标签的文本框（无复合模式） | 平台的单行编辑键 | 由浏览器实现，本部件不截获 |
| | | ↑／↓（`kind="number"`） | 按 `step` 增减，由平台实现 |
| `inspect/patch.svelte` 的行号栏（调用方给了 `talk` 地址时：检视面的 diff 与 `changes.svelte` 打开的一行） | 链接 | Enter | 把「路径:新行号」（删去的行是「路径@旧提交:旧行号」）和该行的引文接到那个地址的草稿之后，再打开那段对话；composer 挂载时从草稿门读出它。每行一站 Tab，不给 `talk` 的页面行号栏不取焦 |
| `building/commits.svelte` 的提交行 | APG Disclosure | Enter／Space | 展开或收起这次提交的事实单与改动的文件，`aria-expanded` 随之（4-50 其二） |
| 提交的父提交 | APG Button | Enter／Space | 父提交在已读的页里时展开那一行；不在时就地问 `Query::Commit` 并画出谁写了它 |
| 提交列表头部的 oid 框 | 有标签的文本框 | Enter | 问 `Query::Commit { oid }`；框里不是一个 oid 时不发问，框下说为什么 |
| `building/status.svelte` 的改动行 | APG Button | Enter／Space | 在右侧打开这个文件的工作树版本（4-50 其三）；右侧关上时焦点回到这一行（7-7） |
| 改动行与提交文件的「取回」 | APG Button，打开 `dialog.svelte` | Enter／Space | 先问一次（7-3 的 dialog）；确认才发 `restore_file`，取消或 Esc 时焦点回到「取回」 |
| `building/plan.svelte` 的计划行 | APG Disclosure | Enter／Space | 展开或收起这个节点的花费（4-50 其六） |
| `building/sandbox.svelte` 的保存 | APG Button | Enter | 整值发 `configureSandbox`；燃料不是正整数时按不动，`why` 说为什么 |

`button.svelte` 的 `aria-*`：`aria-disabled` 在 `loading` 或 `why` 在场时为 `"true"`，`aria-busy` 只在 `loading` 时为 `"true"`，`why` 在场时 `aria-describedby` 指向 `Tip` 的 id。**用 `aria-disabled` 而不是 `disabled`**：控件因此留在 Tab 序列里，键盘到得了它，读屏也读得到它为什么按不动。

`field.svelte` 的 `aria-*`：`<label for>` 给名字（`labelling="hidden"` 只把标签移出视线，名字仍在）；`aria-invalid` 恒等于 `error !== undefined`；`aria-describedby` 是调用方的 `describedBy` 与本格说明段 id 的并集，**错误替换说明而不是叠在它上面**，错误段自己带 `role="alert"`。红边有两条权威且说的是两件事：`error` 是城的回答，一到就红；`:user-invalid` 是浏览器读 `type` 与 `pattern` 的结果，失焦后才红。
-/

/-!
### §7-3 提示与模态

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `tip.svelte` | APG Tooltip | Escape | **规格要求撤下提示；今天没有实现**（7-8 第 9 条）|
| `dialog.svelte` | APG Modal Dialog | Tab／Shift+Tab | 在对话框内循环，由平台实现 |
| | | Escape | 平台发 `cancel`，本部件 `preventDefault()` 后回调 `onCancel`——默认行为会绕过调用方关掉元素，而 `open` 还说着开着 |
| `kbd.svelte` 的 `Cheatsheet` | 手写的 dialog | Escape | 由外壳 `app.svelte` 的按键处理器答，不在部件里（7-8 第 12 条）|

`tip.svelte` 的 `aria-*`：提示节点是 `role="tooltip"`，id 交给调用方——控件自己有可见文字时写 `aria-describedby`，这句话就是它唯一的名字时写 `aria-labelledby`。**组件不猜**，因为只有调用点知道控件有没有名字。显示由 `:hover` 与 `:focus-within` 触发，延迟 300 ms；提示自己 `pointer-events-none`，永不取焦。

`dialog.svelte` 的 `aria-*`：`aria-labelledby` 指向标题，`aria-describedby` 指向说明段**且仅在 `detail` 在场时才写**。取焦由文档顺序决定：平台取对话框内第一个可聚焦控件，而取消按钮写在确认按钮之前，所以撤不回来的那一问把安全的答案放在手下。**点 `::backdrop` 不关闭**：撤不回来的那一问不该被一次落在外面的点击答掉。
-/

/-!
### §7-4 在几件之间走动的部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `segmented.svelte` | APG Radio Group（roving tabindex） | Tab／Shift+Tab | 进出控件；控件在 Tab 序列里只占一站 |
| | | → | 移到下一个可选格并选中它，走到末端回到开头 |
| | | ← | 移到上一个可选格并选中它，走到开头回到末端 |
| | | ↓／↑ | **规格要求同 →／←；今天没有实现**（7-8 第 5 条）|
| | | Space | **规格要求选中当前聚焦格；今天没有实现**（7-8 第 5 条）|
| `tabs.svelte` | APG Tabs（自动激活） | → | 下一个透镜并立即切换，走到末端回到开头 |
| | | ← | 上一个透镜并立即切换，走到开头回到末端 |
| | | Home／End | 第一个／最后一个透镜并立即切换 |
| | | Space／Enter | 与点击同一条路；因为切换已跟随焦点，它们不额外做事 |
| `row.svelte` 的 `RowList` | 无 APG 部件模式：一串各自可达的行，加上方向键 | ↓／↑ | 走到下一／上一行，焦点落在那一行第一个可达控件上；**两端不环绕**——一本上千行的账本从末行跳回首行，是把人移到了他看不出自己去过的地方 |
| | | Home／End | 第一／最后一行的第一个可达控件 |
| | | 落在文本框、`<select>` 或可编辑区域上的同一批键 | 不接管：那些键在那个控件里已经有意思了 |
| | | Tab | 照旧逐行走——**每一行仍是一个 Tab 站，方向键是加法不是替换** |

`segmented.svelte` 的 `aria-*`：轨道 `role="radiogroup"` ＋ `aria-label`；每格 `role="radio"`、`aria-checked` 等于「这一格就是 `held`」、`why` 在场时 `aria-disabled="true"` 并 `aria-describedby` 指向 `Tip`。**Tab 序列里的那一站由 `tabStop` 独家决定**：选中格；无选中时第一个可选格；全部被拒时第 0 格（那格的原因还得读得到）；空控件一站都没有。选择跟随焦点，所以不可选的格被 `nextStop` 跳过——落在上面就等于选中它。

`tabs.svelte` 的 `aria-*`：`role="tablist"` ＋ `aria-label`；每个 `role="tab"`、`aria-selected` 等于「这就是 `current`」、`tabindex` 只给当前那个 `0`。自动激活是 APG 对「面板内容已在本地、切换无可察延迟」的推荐读法，本客户端三个使用者都满足它。
-/

/-!
### §7-5 开一层列表的复合部件

| 部件 | 模式 | 键 | 结果 |
|---|---|---|---|
| `combobox.svelte` | APG Combobox（listbox 弹层） | ↓／↑ | 游标下移／上移一行，钳在列表两端 |
| | | Home／End | **规格要求到首行／末行；今天没有实现**（7-8 第 2 条）|
| | | Enter | 采纳游标行，关闭弹层，焦点回触发器 |
| | | Escape | 关闭弹层，清空过滤词，焦点回触发器 |
| | | Tab | **规格要求关闭弹层并让焦点正常离开；今天弹层留着**（7-8 第 2 条）|
| | | 可打印字符 | 过滤，并把游标复位到第 0 行 |
| `popover.svelte` | 多列 listbox，装在 `role="dialog"` 里 | ↓／↑ | 当前列的游标下移／上移，钳在两端 |
| | | Home／End | 当前列的首行／末行 |
| | | Tab／Shift+Tab | **换列**（环绕）并把游标复位到第 0 行——本部件在此覆盖平台的 Tab |
| | | Enter | 应用当前列的游标行，回调 `onApply` |
| | | Escape | 回调 `onClose` |

`combobox.svelte` 的 `aria-*`（规格）：文本框是 `role="combobox"`，带 `aria-expanded`、`aria-controls` 指向列表、`aria-activedescendant` 指向游标行；列表 `role="listbox"` ＋ `aria-label`；每行 `role="option"`，`aria-selected` 只标**已选中的那个值**，不标游标。今天的实现把 `aria-haspopup="listbox"` ＋ `aria-expanded` 放在触发按钮上、过滤框没有角色、游标只有底色——见 7-8 第 1 条。

`popover.svelte` 的 `aria-*`（规格）：外层 `role="dialog"` ＋ `aria-label`；每列 `<ul role="listbox">` ＋ `aria-label`；每行 `role="option"`。**`aria-selected` 在两个部件里必须说同一件事——「这是当前生效的值」**，游标一律由持焦元素的 `aria-activedescendant` 承担；今天 `popover.svelte` 用 `aria-selected` 标游标，而真正生效的那一项只有一个圆点（7-8 第 3 条）。两种触发各有一条焦点路：按钮触发时列表自己取焦（`tabindex` 只给当前列 `0`）；文本框触发时调用方经 `bind` 拿走键表，焦点留在文本框里，此时 `aria-activedescendant` 必须写在那个文本框上（7-8 第 4 条）。
-/

/-!
### §7-6 表格

`table.svelte` 是一张数据表，**不是 APG Grid**：它不做二维方向键导航，Tab 依次走过排序按钮、勾选框与可改单元格，Enter／Space 在排序按钮上切换方向。

`aria-*`：`<caption class="sr-only">` 给表名；**`aria-sort` 只写在可排序的列上**，取 `"ascending"`／`"descending"`／`"none"`；表头勾选框 `aria-label` 取 `allLabel`，行勾选框取 `keyOf(row)`，可改单元格取 `"<列名> <keyOf(row)>"`。表头勾选框在部分选中时必须是 `indeterminate`——说「一个都没选」是一句假话（7-8 第 7、8 条）。
-/

/-!
### §7-7 焦点还原：一条规则，三种实现

**一个把焦点拿走的部件必须把它还给打开它的那个元素**，在它关闭的那一刻。三种实现今天并存，它们的差别只在谁记住了那个元素：

| 谁还 | 部件 | 怎么还 |
|---|---|---|
| 平台 | `dialog.svelte` | `close()` 按 HTML 标准把焦点还给 `showModal()` 之前持焦点的元素，本文件因此没有一行取焦代码 |
| 状态模块 | 检视面（`views/inspect/open.svelte.ts`） | 从检视面之外打开一项时记下当时持焦点的元素；关上检视面或它最后一个页签时，焦点若在检视面里（或因面板卸载落到 `body`），交还给那个元素，它已不在页面上时不还 |
| 部件自己 | `combobox.svelte`、`popover.svelte` | 前者记住触发按钮的 ref，`shut()` 时还；后者在 `onMount` 记下当时的 `document.activeElement`，`onCleanup` 还（`bind` 模式下焦点从未离开文本框，因此不还）|
| 外壳 | `kbd.svelte` 的 `Cheatsheet` | `app.svelte` 在打开前记 `opener`，`closeSheet` 时还——**全客户端唯一一处还原权威不在部件里**，它随 7-8 第 12 条的搬家一起消失 |
-/

/-!
### §7-10 这张表的机器读者

**`#/gallery` 的夹具断言本节的键表**，这是让规格不止有人类读者的那一步：每个收键部件在那条路由上有一份夹具，夹具按「初始焦点 ＋ 一串按键 → 焦点落点、`aria-*` 取值、回调是否发生」逐行断言 7-2 至 7-6。夹具与断言的实现属于 `client/src/views/gallery.svelte` 与 `tools/xtask/src/render/`，本节只定内容。

今天的 `xtask render` 读的是画出来的盒子与它们的名字（`tools/xtask/Spec.lean` §8-13），**一次按键都没有进过真引擎**——在这第二个读数落地之前，本节的键表没有机器读者，这一点如实记在 §2 的「未验的」里。
-/

/-!
## 模型：走动与焦点还原

规定 `client/src/views/parts/` 里收键部件共用的两件事，各部件的分部（`spec/Views/Parts/*.lean`）引用这里的定义而不另写一份：

1. **方向键怎样走。** 只有两种走法：**环绕**（`wrap`，走到末端回到开头：分段控件、页签、检视面的页签带、弹层的换列）与**钳住**（`clamp`，两端不环绕：`RowList`、组合框与弹层的游标、信箱的 j/k）。环绕一步可逆（`wrap_back_undoes_forward`），走满一圈回到原处（`a_full_turn_comes_home`），从第 0 格出发每一格都走得到（`every_cell_is_reached`）；钳住不出界（`clamp_stays`），在两端是不动点（`clamp_holds_the_last`、`clamp_holds_the_first`）——§7-4 说的「一本上千行的账本从末行跳回首行，是把人移到了他看不出自己去过的地方」。
2. **焦点还给谁（§7-7）。** 一个把焦点拿走的部件在关闭时把焦点还给打开它的那个元素。模型是一个打开者的栈：`openLayer` 记下此刻持焦点的元素，`closeLayer` 把它还回去，它已不在页面上时不还。一层一层打开再一层一层关上，焦点回到最初的元素（`closing_every_layer_restores_the_page`）。咬得动的演示：关上时把焦点丢给 `body` 的写法（`dropToBody`）在一次打开与关闭之后不回到打开者（`dropping_focus_loses_the_opener`）。

谁记住打开者（平台、状态模块、部件自己）是 §7-7 表里的三种实现；模型只规定它们共同欠的那一件事。
-/

namespace Client.Views.Parts

/-- 方向键走的方向：→ 与 ↓ 是 `forward`，← 与 ↑ 是 `backward`。 -/
inductive Dir where
  | forward
  | backward
  deriving DecidableEq, Repr

/-- 环绕的一步：走到末端回到开头，走到开头回到末端。 -/
def wrap (total pos : Nat) : Dir → Nat
  | .forward => (pos + 1) % total
  | .backward => (pos + total - 1) % total

/-- 钳住的一步：两端不环绕。 -/
def clamp (total pos : Nat) : Dir → Nat
  | .forward => min (pos + 1) (total - 1)
  | .backward => pos - 1

/-- 同一方向走 `k` 步。 -/
def walk (total pos : Nat) (dir : Dir) : Nat → Nat
  | 0 => pos
  | k + 1 => wrap total (walk total pos dir k) dir

theorem wrap_stays (total pos : Nat) (dir : Dir) (h : 0 < total) : wrap total pos dir < total := by
  cases dir <;> exact Nat.mod_lt _ h

theorem wrap_back_undoes_forward (total pos : Nat) (h : pos < total) :
    wrap total (wrap total pos .forward) .backward = pos := by
  simp only [wrap]
  by_cases inner : pos + 1 < total
  · rw [Nat.mod_eq_of_lt inner]
    have shift : pos + 1 + total - 1 = pos + total := by omega
    rw [shift, Nat.add_mod_right, Nat.mod_eq_of_lt h]
  · have last : pos + 1 = total := by omega
    rw [last, Nat.mod_self]
    have back : 0 + total - 1 = pos := by omega
    rw [back, Nat.mod_eq_of_lt h]

theorem walk_forward (total pos k : Nat) (h : pos < total) :
    walk total pos .forward k = (pos + k) % total := by
  induction k with
  | zero => simp only [walk, Nat.add_zero, Nat.mod_eq_of_lt h]
  | succ k ih =>
    simp only [walk, ih, wrap]
    rw [Nat.mod_add_mod, Nat.add_assoc]

theorem a_full_turn_comes_home (total pos : Nat) (h : pos < total) :
    walk total pos .forward total = pos := by
  rw [walk_forward total pos total h, Nat.add_mod_right, Nat.mod_eq_of_lt h]

theorem every_cell_is_reached (total cell : Nat) (h : cell < total) :
    walk total 0 .forward cell = cell := by
  rw [walk_forward total 0 cell (by omega), Nat.zero_add, Nat.mod_eq_of_lt h]

theorem clamp_stays (total pos : Nat) (dir : Dir) (h : pos < total) : clamp total pos dir < total := by
  cases dir with
  | forward => exact Nat.lt_of_le_of_lt (Nat.min_le_right _ _) (by omega)
  | backward => simp only [clamp]; omega

theorem clamp_holds_the_last (total : Nat) :
    clamp total (total - 1) .forward = total - 1 := by
  simp only [clamp]
  exact Nat.min_eq_right (by omega)

theorem clamp_holds_the_first (total : Nat) : clamp total 0 .backward = 0 := rfl

/-- 焦点的状态：此刻持焦点的元素，与打开每一层时持焦点的元素（栈顶是最近一层的打开者）。`Elem` 由调用方给，模型只问一个元素还在不在页面上。 -/
structure Focus (Elem : Type) where
  focused : Elem
  openers : List Elem
  deriving DecidableEq

variable {Elem : Type}

/-- 打开一层（对话框、检视面、组合框的弹层）：焦点进到层里，记下打开者。 -/
def openLayer (s : Focus Elem) (inside : Elem) : Focus Elem :=
  ⟨inside, s.focused :: s.openers⟩

/-- 关上最近一层：打开者还在页面上就把焦点还给它，不在就不还。 -/
def closeLayer (present : Elem → Bool) (s : Focus Elem) : Focus Elem :=
  match s.openers with
  | [] => s
  | opener :: rest => ⟨if present opener then opener else s.focused, rest⟩

/-- 依次打开几层。 -/
def openAll (s : Focus Elem) : List Elem → Focus Elem
  | [] => s
  | inside :: more => openAll (openLayer s inside) more

/-- 关上 `k` 层，最近打开的先关。 -/
def closeAll (present : Elem → Bool) : Nat → Focus Elem → Focus Elem
  | 0, s => s
  | k + 1, s => closeLayer present (closeAll present k s)

theorem closing_returns_to_the_opener (present : Elem → Bool) (s : Focus Elem) (inside : Elem)
    (here : present s.focused = true) :
    closeLayer present (openLayer s inside) = s := by
  cases s
  simp_all [closeLayer, openLayer]

theorem closing_every_layer_restores_the_page (present : Elem → Bool) (layers : List Elem) :
    ∀ s : Focus Elem, present s.focused = true → (∀ e ∈ layers, present e = true) →
      closeAll present layers.length (openAll s layers) = s := by
  induction layers with
  | nil => intro s _ _; rfl
  | cons inside more ih =>
    intro s here all
    have rest : ∀ e ∈ more, present e = true := fun e h => all e (List.mem_cons_of_mem _ h)
    have opened := ih (openLayer s inside) (all inside (List.mem_cons_self ..)) rest
    simp only [List.length_cons, closeAll, openAll]
    rw [opened]
    exact closing_returns_to_the_opener present s inside here

/-- 被否决的写法：关上时让焦点落到 `body`，不回打开者。 -/
def dropToBody (body : Elem) (s : Focus Elem) : Focus Elem :=
  match s.openers with
  | [] => s
  | _ :: rest => ⟨body, rest⟩

theorem dropping_focus_loses_the_opener :
    dropToBody 0 (openLayer (⟨7, []⟩ : Focus Nat) 3) ≠ ⟨7, []⟩ := by
  decide

/-! ### 模态：外面的事件从不确认（§7-3）

`dialog.svelte` 取焦由文档顺序决定，取消写在确认之前，所以安全的答案在手下——这是那个文件里的一行顺序，不是这里的一条定理；Escape 由部件拦下交给 `onCancel`，点 `::backdrop` 不关闭。 -/

/-- 一扇撤不回来的模态里的两个答复，按文档顺序。 -/
inductive Reply where
  | cancel
  | confirm
  deriving DecidableEq, Repr

/-- 模态收到的两种「外面」的事件。 -/
inductive Outside where
  | escape
  | backdrop
  deriving DecidableEq, Repr

/-- 一个外面的事件答了什么：Escape 是取消，点背景什么都不答。 -/
def answered : Outside → Option Reply
  | .escape => some .cancel
  | .backdrop => none

theorem escape_never_confirms (event : Outside) : answered event ≠ some .confirm := by
  cases event <;> decide

end Client.Views.Parts
