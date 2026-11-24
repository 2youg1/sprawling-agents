-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import RefRain.spec.Views.Parts

/-!
# workspace：外壳的控件

本文件是 `RefRain/Spec.lean` 的一个分部，规定外壳上不在 `views/parts/` 里的控件欠使用者什么。下面一节保留标签 §7-11，别处引作 `RefRain/Spec.lean §7-11`。
-/

/-!
### §7-11 外壳的控件

这些控件不在 `views/parts/` 里，但欠使用者的东西同样由本节规定。

| 控件 | 模式 | 键 | 结果 |
|---|---|---|---|
| 硬币键 | APG Button | Enter／Space | 做朝上那一面：发出框里的字，或对眼前的 run 发 `cancel`（D16）；变淡的发送面一按落进空操作 |
| 上下文环 | APG Button（同工具行：打开右侧检视，无 `aria-haspopup`），提示同 `tip`；读数仍在 `aria-valuetext` | Tab | 到达环（硬币键外的环带），提示出现 |
| | | Enter／Space，或点按环带 | 在右侧打开该会话的上下文用量报告（`openContext`）；硬币键仍是发送／停止／转向 |
| | | Escape | 撤下提示 |
| 地址行 | APG Button（`aria-haspopup="dialog"`） | Enter／Space，或点按 | 打开命令面板（RefRain/Spec.lean 4-10a）：键盘与鼠标打开时焦点进框，手指或笔按下时焦点落在面板本身（4-64b）；先列这个房间的会话，列表下读「谁在听」（D36）；Escape 关上，焦点回到地址行 |
| 工具行 | APG Button（`aria-pressed`） | Enter／Space | 在右侧打开这次调用（`openCall`）；打开着的那行 `aria-pressed="true"` |
| | | ↓／↑，j／k | 同一段对话里的下一条或上一条工具行 |
| | | Escape | 右侧开着这行时收起右侧，焦点留在这行 |
| | | Tab | 到达这行，提示写出完成（或开始）的本地时刻，到秒 |
| 未读计数 | APG Button | Enter／Space | 回到末尾并恢复跟随；按钮随计数消失，焦点落在对话列上 |
| 手机操作入口 | APG Disclosure Button | Enter／Space 或短触 | 左下一个入口换成包含四个图标动作的同一弹出菜单，焦点到设置。Esc 还焦并收起，触摸面外收起并把输入留给触点，选定任一动作后收起。 |
| 设置键 | APG Button | Enter／Space 或点按 | 打开设置本体。关闭后焦点到桌面设置键或手机操作入口。 |
| 导航键 | APG Disclosure Button | Enter／Space 或点按 | 开合当前主体的文件树或会话列表。打开时焦点进入这一层；关闭后还焦到导航键。 |
| 文件夹与父目录 | 原生按钮与当前目录路径 | Enter／Space 或点按 | 当前列表只显示直属子项，进入子目录时焦点到路径；返回将焦点送到父目录中的原文件夹。目录深度独立于菜单层数。 |
| 文件分屏 | APG Window Splitter | 桌面 ←／→、手机 ↑／↓、Home／End、Enter 或拖动 | 桌面在外壳栏线上左右分屏，每边至少两栏。手机上下分屏，首次平分，拖动时每边至少四分之一。Enter 复位。横向栏线和纵向比例分别保存。 |
| 导航层 | 非模态、有名称的区域 | Escape | 先交给焦点所在的菜单；随后布局、资料库或历史返回列表及原入口；从目录内开始的 Escape 逐级返回并还焦到刚离开的文件夹；列表关闭并还焦到导航键。露出的阅读区可独立滚动。 |
| 导航层的提交历史 | 原生按钮，按提交显示展开状态 | Enter／Space 或点按 | 由列表头进入历史，行内展开 OID、父提交和时间。有父提交时同时打开差异检视。当前 Session、草稿和阅读坐标保持原值。回到列表时还焦到层标题。 |
| 文件会话浮层 | APG Disclosure Button | Enter／Space 或点按 | 开合当前 Session 的草稿。打开时焦点进入输入框；文件仍以原页宽和位置显示。已分屏时把焦点送到现有草稿。 |
| 主体键 | APG Button | Enter／Space 或点按 | 在文件和 Session 之间切换，共用同一个会话与草稿。画布留有自己的几何、滚动与编辑器状态。 |
| 密度键 | APG Button | Enter／Space 或点按 | 在 Zen 和 Panorama 之间切换。图层、文件和会话不变。 |
| 密度键 | APG Button | Accel-\ 或指针 | 短按切换密度；按住 300 ms 临时显示当前会话的活动。松开、指针取消或窗口失焦即退出，不触发短按。 |
| 右侧的面（一栏） | 同检视面 | 关闭键、Escape、浏览器返回、边缘返回 | 关上右侧并退出它的历史格，焦点回到打开它的控件 |
| 信箱键、设置键 | APG Button | Enter／Space | 打开各自的面；面关上时焦点回到这个键 |
| 信箱面 | 非模态的一层，带 `aria-label` 的 `<aside>`，画在边缘入口之下（D60） | Escape | 关上；焦点在面里时回到信箱键 |
| | | 点面外（包括其余边缘入口） | 关上，焦点留在点到的地方 |
| | | 信箱键、Accel-B | 开着时关上（焦点在面里时回到信箱键），关着时打开，焦点不动 |
| 信箱面里的条目 | 无 APG 部件模式：一串各自可达的条目，加上 vim 的走法 | j／k | 焦点移到下一个／上一个条目，两端不环绕；条目是一行的链接或一张请决定卡本身 |
| | | 1–9 | 前九个条目各在行尾画出自己的数字；按下是跟随那一行的链接（信箱随之关上），或把焦点放到那张卡上 |
| | | 落在文本框里或带修饰键的同一批键 | 不接管 |
| | | Tab | 照旧逐个控件走：一行的链接与它的「从最后一轮分叉」是两站 |
| 请决定卡 | `role="group"`，以头一行命名 | Accel-Shift-Y／E／X（焦点在卡内） | 做那个答复；卡上没有的答复、或说明了为什么按不动的答复，按键落空 |
| | | Tab | 依次走过卡里的控件与答复 |
| 修改提案卡的改后接受 | 每句改动一个原生勾选框（可读名字是「取这一句」加句子的前几个字）、插入句一个原生文本框 | Space（勾选框） | 取或不取这一句；不取即拒绝这一句 |
| | | e（焦点不在文本框里） | 回到 diff，改过的字与勾选留着，直到卡被决定或消失 |
| | | y（焦点不在文本框里） | 按勾选与改过的字发出；在文本框里打 y 是字，不是答复 |
| 修改提案卡的「在原文中显示」 | APG Button | Enter／Space | 编辑器的光标落到这一段的开头并滚进视野，焦点进编辑器 |
| 边缘入口的名字 | APG Tooltip | 单独按住加速键 300 ms | 显示当前可见入口的名字；松开、窗口失焦或开始组合输入即收 |
| | | Escape | 撤下正在显示的名字 |
| 检阅三栏的分隔线 | APG Window Splitter | ←／→ | 前一栏宽一栏或窄一栏，钳在每栏至少两栏宽 |
| | | Home／End | 前一栏到最窄或最宽 |
| | | Enter | 复位到默认；这些栏不收起，所以 APG 里 Enter 的「收起与恢复」在这里读作复位 |
| 右侧编辑器与终端之间的分隔线 | APG Window Splitter | ↑／↓ | 编辑器高一行或矮一行（24 px），编辑器与终端各至少留六行 |
| | | Home／End | 编辑器到最矮或最高 |
| | | Enter | 复位到默认 |
| 检视面的页签带 | APG Tabs（手动之外的自动激活） | ←／→ | 前一个或后一个页签到前面，焦点随之；到头时绕回 |
| | | Home／End | 第一个或最后一个页签 |
| | | Delete | 关掉焦点所在的页签，焦点落到它原来位置上的页签 |
| 检视面 | `<aside>` 地标 | Escape（检视面里任何地方，输入法与 RefRain 自己的面板没有先拿走这个键时） | 关上检视面，焦点回到打开它的控件（7-7） |
| 检视面的「原文」 | APG Button（开关） | Enter／Space | 在视图与原文之间切换 |
| 栏标签的菜单 | APG Menu Button | Enter／Space／↓ | 打开「移到左边／移到右边」；Escape 关上，焦点回到标签 |
| 会话栏一行 | 链接 | Enter | 这一段进主区（7K、D44） |
| 会话栏一行的菜单 | APG Menu Button | Enter／Space／↓ | 打开菜单，焦点落在第一项 |
| | | ↑／↓ | 在项间走，两端环绕 |
| | | Enter／Space | 做这一项；「加一个标签」把菜单换成一个输入框 |
| | | Escape／Tab | 关上；Escape 把焦点还给按钮，Tab 按文档序走 |
| 菜单里的标签输入框 | 原生文本框 | Enter | 给这一段加上这个词并关上，读不出时留着并标 `aria-invalid`；焦点回到按钮 |
| | | Escape | 不加，关上，焦点回到按钮 |
| 标签筛选行 | `role="group"`，每个标签一个 `aria-pressed` 按钮 | Enter／Space | 只看带这个标签的行；再按已选的那个回到全部 |
| 设置树 | APG Disclosure Navigation | Tab／Shift+Tab | 依次走过条目 |
| | | ↓／↑ | 下一个或上一个看得见的条目 |
| | | Home／End | 第一个或最后一个条目 |
| | | Enter／Space（有子条目的条目） | 展开或收起 |
| | | Enter（组或页的条目） | 组画进正文；页移动地址栏并关上面板 |
| 设置面 | APG Dialog (Modal) | Escape、点背景、Accel-, | 关上；焦点回到打开它的控件 |
| 外观组的混合档滑条 | APG Slider（原生 `<input type="range">`，30%–90%，一步 5） | ←／→、↑／↓ | 走一步；落定的值立即生效，卡脚写「已保存」（§4-36） |
| | | Home／End | 到 30% 或 90% |
| 远程组的「配对这台设备」与「锁上门」 | APG Button | Enter／Space | 走那一次握手；进行中 `aria-disabled="true"`，结果写进组里唯一的 `role="status"` |
| 远程组的种子 | 只读文字，带 `aria-label` 的 `<output>` | 「我记下了」（APG Button）Enter／Space | 种子从页面上拿掉，焦点落到这台设备那一节的标题 |
| 远程组的「忘掉这台设备」 | APG Button，打开 `parts/dialog` | Enter／Space | 确认后删掉本地记录，焦点回到组标题；取消时回到这个按钮 |

`aria-*`：硬币键的可读名字是朝上那一面的动作——发送（steer 时写出落点）或停止——翻面时名字随之换，而换名不进实时区域，因为它不是新闻；变淡的发送面 `aria-disabled="true"`，`aria-describedby` 指向说明原因的提示。上下文环 `aria-valuemin="0"`、`aria-valuemax` 是窗口、`aria-valuenow` 是用掉的 token，`aria-valuetext` 是提示那一行。房间菜单的列表以「谁在听」那一段为 `aria-describedby`，所以读屏器在列表获得焦点时连同它一起读出；那一段里没有可聚焦的东西，菜单的键全归列表。导航键以当前主体的列表命名，带 `aria-expanded` 和 `aria-controls`。设置树是 `<nav>` 加 `aria-label`，页条目是 `<a href>`（中键与新标签页照常可用），组条目是按钮；当前页（设置面下面那一页）`aria-current="page"`，当前组 `aria-current="true"`；有子条目的条目带 `aria-expanded` 与 `aria-controls`；每个枝的列表以枝的标签为 `aria-labelledby`。设置面以当前组的标题为 `aria-labelledby`，打开时焦点落在当前组的条目上。分隔线是 `role="separator"`，带 `aria-orientation`、以栏数计的 `aria-valuenow`／`aria-valuemin`／`aria-valuemax`，`aria-controls` 指向前一栏。右侧编辑器与终端之间的分隔线以行计（一行是三个基线步，24 px）。检视面的页签带是 `role="tablist"`，整条是一个 Tab 站（游走的 `tabindex`），每个页签 `aria-selected` 说它是不是最后碰过的那一项、`aria-controls` 指向它所在的区（编辑器区与终端区各是一个 `role="tabpanel"`）；页签上的关闭记号给指针用，`tabindex="-1"`，键盘走 Delete。「原文」带 `aria-pressed`。混合档滑条的 `aria-valuetext` 写出百分数（画法见 `docs/frontend-method.md` §4-43）。
-/

/-!
## 模型：图层键、硬币键与信箱的走法

规定 §7-11 里的外壳状态机；分隔线在 `RefRain/spec/Core/Workbench.lean`，检视面的页签带在 `RefRain/spec/Views/Inspect/Open.lean`，请决定卡在 `RefRain/spec/Views/Parts/Decide.lean`。

1. **工作区外壳**：桌面从左下向上常驻设置、当前列表、信箱、主体四个入口。手机初始一个操作入口，展开时替换为同一弹出菜单内的四个图标动作，每项保留完整可及名称。导航键只开合当前主体的列表；主体键只换文件／Session 画法；密度只有 Zen 与 Panorama。按住查看活动不改选定的密度，显式分屏才改变文件画布的布局。冷启动先读显式地址，再读浏览器现场，仍无记录时走首次入口。
2. **硬币键**（`RefRain/src/views/talk/coin_face.ts` 的 `pressCoin`，client D18）：做朝上那一面，只有没变淡的发送面发出框里的字：变淡的发送面一按落进空操作，停止面只发 `cancel`（`only_the_lit_send_face_sends_words`）。TS 里变淡的发送面叫 `idle`，即这里的 `send true`；`coin_face.test.ts` 在每一面上检查这条定理；点击写在 `coin.ts` 造的接线包里，只按 `pressCoin` 的回答行事，`coin.test.ts` 在每一面上按一次，看停止与提交各自有没有发生。
3. **信箱的条目**（`RefRain/src/views/mailbox/entries.ts`）：j／k 与 `RowList` 同一种钳住的走法，到最后一条不再走（`k_stops_at_the_last_entry`）；1–9 只落到前九个、且存在的条目上（`a_digit_reaches_only_a_drawn_entry`）。
4. **信箱面这一层**（`RefRain/src/views/mailbox/layer.ts`，D60）：不论按什么顺序按键，关上的信箱里不留焦点（`no_focus_stays_in_a_closed_mailbox`）；Escape 总是关上它，焦点在面里时回到信箱键（`escape_closes_and_returns_focus`）；信箱键与 Accel-B 是同一个开关，按两下回到原样（`two_presses_come_home`）；点面外，包括其余边缘入口，总是关上它（`a_press_outside_closes`）。
5. **信件在右侧**（`RefRain/src/views/mailbox/layer.ts` 的 `stepLetter`，D73）：信件关上时信箱重新打开、焦点回到打开它的那一行，中间信箱收到什么输入都不改（`closing_a_letter_returns_to_its_row`）；信件的开合不让焦点留在关上的信箱里（`letters_hold_mail_focus`）。
-/

namespace Client.Views.Workspace

open Client.Views.Parts

inductive Subject where
  | files
  | session
  deriving DecidableEq, Repr

inductive Density where
  | zen
  | panorama
  deriving DecidableEq, Repr

def nextDensity : Density → Density
  | .zen => .panorama
  | .panorama => .zen

inductive Navigation where
  | closed
  | list
  | history
  | layout
  | library
  deriving DecidableEq, Repr

inductive EdgeAction where
  | launcher
  | settings
  | navigation
  | mailbox
  | subject
  deriving DecidableEq, Repr

def edgeActions (narrow expanded : Bool) : List EdgeAction :=
  if narrow && !expanded then [.launcher]
  else [.settings, .navigation, .mailbox, .subject]

theorem compact_edge_has_one_action : edgeActions true false = [.launcher] := rfl

theorem expanded_edge_matches_desktop : edgeActions true true = edgeActions false false := rfl

structure Shell where
  subject : Subject
  density : Density
  navigation : Navigation
  peeking : Bool
  split : Bool
  file : Option String
  session : Option String
  deriving DecidableEq, Repr

def toggleNavigation (shell : Shell) : Shell :=
  { shell with navigation := if shell.navigation = .closed then .list else .closed }

def backNavigation (shell : Shell) : Shell :=
  { shell with navigation := match shell.navigation with
    | .history | .layout | .library => .list
    | .list | .closed => .closed }

def toggleSubject (shell : Shell) : Shell :=
  { shell with
    subject := match shell.subject with
      | .files => .session
      | .session => .files
    navigation := .closed }

def toggleDensity (shell : Shell) : Shell :=
  { shell with density := nextDensity shell.density }

def holdPeek (shell : Shell) : Shell := { shell with peeking := true }

def releasePeek (shell : Shell) : Shell := { shell with peeking := false }

theorem navigation_keeps_selected_targets (shell : Shell) :
    (toggleNavigation shell).file = shell.file ∧
    (toggleNavigation shell).session = shell.session ∧
    (toggleNavigation shell).split = shell.split :=
  ⟨rfl, rfl, rfl⟩

theorem subject_keeps_selected_targets (shell : Shell) :
    (toggleSubject shell).file = shell.file ∧
    (toggleSubject shell).session = shell.session ∧
    (toggleSubject shell).split = shell.split :=
  ⟨rfl, rfl, rfl⟩

theorem two_density_presses_restore (density : Density) :
    nextDensity (nextDensity density) = density := by
  cases density <;> rfl

theorem releasing_peek_keeps_density (shell : Shell) :
    (releasePeek (holdPeek shell)).density = shell.density ∧
    (releasePeek (holdPeek shell)).peeking = false :=
  ⟨rfl, rfl⟩

/-- 硬币键朝上的那一面。 -/
inductive Face where
  | send (faded : Bool)
  | stop
  deriving DecidableEq, Repr

/-- 按下硬币键发出什么。 -/
inductive Sent where
  | words
  | cancel
  | nothing
  deriving DecidableEq, Repr

/-- Enter／Space 做朝上那一面。 -/
def pressCoin : Face → Sent
  | .send false => .words
  | .send true => .nothing
  | .stop => .cancel

theorem only_the_lit_send_face_sends_words (face : Face) : pressCoin face = .words ↔ face = .send false := by
  cases face with
  | send faded => cases faded <;> decide
  | stop => decide

/-- 信箱里按下数字 `digit` 落到第几个条目；`count` 是条目数。 -/
def digitTarget (count digit : Nat) : Option Nat :=
  if 1 ≤ digit ∧ digit ≤ 9 ∧ digit ≤ count then some (digit - 1) else none

theorem k_stops_at_the_last_entry (count : Nat) : clamp count (count - 1) .forward = count - 1 :=
  clamp_holds_the_last count

theorem a_digit_reaches_only_a_drawn_entry (count digit entry : Nat)
    (h : digitTarget count digit = some entry) : entry < count ∧ entry < 9 := by
  unfold digitTarget at h
  split at h
  · cases h; omega
  · cases h

/-! D60 信箱面是一层普通的 fixed 元素，挂在边缘键的 `<nav>` 里、画在边缘入口之下，点面外与 Escape 由它自己判。
理由：User 要信箱从左缘推出、贴左对齐、可见边缘入口浮在它上面；`popover="auto"` 把面放进 top layer，top layer 之上只能再是 top layer，键要浮上去就得把边缘入口也做成 popover，而 popover 离开网格，键在宽屏与手机上的位置都得重拼一份。被否决的另一条路正是这个。
代价是平台不再替它判点面外与 Escape：本模型就是那份判定，`layer.ts` 照它写，测试按迹重放同样的性质。
重新打开的条件：边缘键本身改成 top layer 里的东西（例如整条边缘层进 popover）时，信箱回到 `popover="auto"`。 -/

/-- 焦点在哪：信箱键、面里、别处。 -/
inductive MailFocus where
  | key
  | inside
  | elsewhere
  deriving DecidableEq, Repr

/-- 信箱面这一层：开着没有，焦点在哪。 -/
structure Mail where
  shown : Bool
  focus : MailFocus
  deriving DecidableEq, Repr

/-- 落到这一层的输入。`toggle` 是信箱键与 Accel-B；`outside` 是面外的一次按下，其余边缘入口也算；`enter` 是焦点走进面里；`follow` 是跟随一行的链接。 -/
inductive MailInput where
  | toggle
  | escape
  | outside
  | enter
  | follow
  deriving DecidableEq, Repr

/-- 收起面：焦点在面里时回到信箱键，否则不动。 -/
def stow (mail : Mail) : Mail :=
  { shown := false, focus := if mail.focus = .inside then .key else mail.focus }

/-- 一个输入之后的这一层。 -/
def stepMail (mail : Mail) : MailInput → Mail
  | .toggle => if mail.shown then stow mail else { mail with shown := true }
  | .escape => stow mail
  | .outside => { shown := false, focus := .elsewhere }
  | .enter => if mail.shown then { mail with focus := .inside } else mail
  | .follow => stow mail

/-- 按顺序走完一串输入。 -/
def runMail (mail : Mail) (inputs : List MailInput) : Mail := inputs.foldl stepMail mail

/-- 焦点只在开着的面里。 -/
def FocusHeld (mail : Mail) : Prop := mail.shown = true ∨ mail.focus ≠ .inside

theorem stow_holds_focus (mail : Mail) : FocusHeld (stow mail) := by
  unfold FocusHeld stow
  right
  by_cases h : mail.focus = .inside <;> simp [h]

theorem step_holds_focus (mail : Mail) (input : MailInput) (h : FocusHeld mail) :
    FocusHeld (stepMail mail input) := by
  cases input with
  | toggle =>
    unfold stepMail
    by_cases s : mail.shown = true
    · simp only [s, ite_true]; exact stow_holds_focus mail
    · simp only [s]; left; rfl
  | escape => exact stow_holds_focus mail
  | outside => right; simp [stepMail]
  | enter =>
    unfold stepMail
    by_cases s : mail.shown = true
    · simp only [s, ite_true]; left; rfl
    · simp only [s]; exact h
  | follow => exact stow_holds_focus mail

theorem no_focus_stays_in_a_closed_mailbox (mail : Mail) (inputs : List MailInput)
    (h : FocusHeld mail) : FocusHeld (runMail mail inputs) := by
  induction inputs generalizing mail with
  | nil => exact h
  | cons input rest ih => exact ih (stepMail mail input) (step_holds_focus mail input h)

theorem escape_closes_and_returns_focus (mail : Mail) (inputs : List MailInput)
    (inside : (runMail mail inputs).focus = .inside) :
    runMail mail (inputs ++ [.escape]) = { shown := false, focus := .key } := by
  simp [runMail, List.foldl_append, stepMail, stow] at *
  simp [inside]

theorem two_presses_come_home (mail : Mail) (closed : mail.shown = false)
    (away : mail.focus ≠ .inside) :
    runMail mail [.toggle, .toggle] = mail := by
  cases mail with
  | mk shown focus =>
    cases focus <;> simp_all [runMail, stepMail, stow]

theorem a_press_outside_closes (mail : Mail) (inputs : List MailInput) :
    (runMail mail (inputs ++ [.outside])).shown = false := by
  simp [runMail, List.foldl_append, stepMail]

/-! D73 信件在右侧打开时信箱收起，信件关上时信箱重新打开、焦点回到打开它的那一行。
理由：右侧在面外，信箱开着时点进信件就是一次 `outside`（`a_press_outside_closes`），所以信件与信箱不能同时开着；而人读完一封信要回到下一封，所以关上信件不能只把焦点交还给一个已经卸下的按钮（`inspect/open.ts` 的 opener 在信箱收起时已不在页上）。行由信件卡的 `id` 认出，不由元素认出，因为信箱重新挂上时元素是新的。
被否决的另一条路：信件打开时信箱不收，点信件不算面外——那要给 `outside` 开一个例外，信箱就有了两种「面外」。
重新打开的条件：信箱不再是左缘推出的一层（D60 重开）时，信件可以与它并排。 -/

/-- 信箱与右侧的信件：`opener` 是打开此刻这封信的那一行，`row` 是信箱里拿着焦点的那一行。 -/
structure LetterSide where
  mail : Mail
  opener : Option Nat
  row : Option Nat
  deriving DecidableEq, Repr

/-- 落到这两处的输入：信箱自己的输入、从第 `row` 行打开一封信、关上信件。 -/
inductive LetterInput where
  | mail (input : MailInput)
  | openLetter (row : Nat)
  | closeLetter
  deriving DecidableEq, Repr

/-- 一个输入之后的信箱与信件。打开信件收起信箱、焦点进右侧；关上信件时有打开它的那一行就回到那一行。 -/
def stepLetter (side : LetterSide) : LetterInput → LetterSide
  | .mail input => { side with mail := stepMail side.mail input }
  | .openLetter row => { mail := { shown := false, focus := .elsewhere }, opener := some row, row := none }
  | .closeLetter =>
    match side.opener with
    | some row => { mail := { shown := true, focus := .inside }, opener := none, row := some row }
    | none => side

/-- 按顺序走完一串输入。 -/
def runLetter (side : LetterSide) (inputs : List LetterInput) : LetterSide :=
  inputs.foldl stepLetter side

theorem mail_inputs_keep_the_opener (side : LetterSide) (inputs : List MailInput) :
    (runLetter side (inputs.map .mail)).opener = side.opener := by
  induction inputs generalizing side with
  | nil => rfl
  | cons input rest ih => exact ih _

theorem closing_a_letter_returns_to_its_row (side : LetterSide) (row : Nat)
    (between : List MailInput) :
    runLetter side ([.openLetter row] ++ between.map .mail ++ [.closeLetter]) =
      { mail := { shown := true, focus := .inside }, opener := none, row := some row } := by
  have kept := mail_inputs_keep_the_opener
    { mail := { shown := false, focus := .elsewhere }, opener := some row, row := none } between
  simp only [runLetter, List.foldl_append, List.foldl_cons, List.foldl_nil] at *
  simp only [stepLetter] at *
  rw [kept]

theorem letters_hold_mail_focus (side : LetterSide) (inputs : List LetterInput)
    (h : FocusHeld side.mail) : FocusHeld (runLetter side inputs).mail := by
  induction inputs generalizing side with
  | nil => exact h
  | cons input rest ih =>
    apply ih
    cases input with
    | mail i => exact step_holds_focus side.mail i h
    | openLetter r => right; simp [stepLetter]
    | closeLetter =>
      unfold stepLetter
      cases side.opener with
      | some r => left; rfl
      | none => exact h

end Client.Views.Workspace
