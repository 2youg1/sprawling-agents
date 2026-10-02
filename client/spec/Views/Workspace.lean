-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import client.spec.Views.Parts

/-!
# workspace：外壳的控件

本文件是 `client/Spec.lean` 的一个分部，规定外壳上不在 `views/parts/` 里的控件欠使用者什么。下面一节保留标签 §7-11，别处引作 `client/Spec.lean §7-11`。
-/

/-!
### §7-11 外壳的控件

这些控件不在 `views/parts/` 里，但欠使用者的东西同样由本节规定。

| 控件 | 模式 | 键 | 结果 |
|---|---|---|---|
| 硬币键 | APG Button | Enter／Space | 做朝上那一面：发出框里的字，或对眼前的 run 发 `cancel`（D16）；变淡的发送面一按落进空操作 |
| 上下文环 | `role="meter"`，提示同 `tip.svelte` | Tab | 到达环，提示出现 |
| | | Escape | 撤下提示 |
| 房间芯片 | APG Listbox（芯片是 `aria-haspopup="listbox"` 的按钮） | Enter／Space | 打开房间菜单：先读「谁在听」，再列房间；↑／↓ 在房间间走，Enter 选定，Escape 关上，焦点回到芯片 |
| 工具行 | APG Button（`aria-pressed`） | Enter／Space | 在右侧打开这次调用（`openCall`）；打开着的那行 `aria-pressed="true"` |
| | | ↓／↑，j／k | 同一段对话里的下一条或上一条工具行 |
| | | Escape | 右侧开着这行时收起右侧，焦点留在这行 |
| | | Tab | 到达这行，提示写出完成（或开始）的 ISO 时刻 |
| 未读计数 | APG Button | Enter／Space | 回到末尾并恢复跟随；按钮随计数消失，焦点落在对话列上 |
| 图层键 | APG Button | Enter／Space | 换到下一档 |
| | | `\`（文本框之外） | 同上；按住超过 300 ms 临时进混合档，松开回原来的档 |
| | | 一栏上（4-52） | Enter／Space 与 `\` 打开或关上世界层的面，按住是临时看一眼那张面；按下不取焦点，软键盘留着 |
| 世界层的面（一栏） | 带名字的 `<section>`，内含 APG Tabs | 返回键 Enter／Space、浏览器返回、边缘返回 | 退出这张面（`history.back()`），回到对话；焦点丢在 `body` 上时回到开面时持焦的控件 |
| | | 页签上 ←／→、Home／End | 换一栏显示，同 `parts/tabs.svelte` |
| 右侧的面（一栏） | 同检视面 | 关闭键、Escape、浏览器返回、边缘返回 | 关上右侧并退出它的历史格，焦点回到打开它的控件 |
| 信箱键、设置键 | APG Button | Enter／Space | 打开各自的面；面关上时焦点回到这个键 |
| 信箱面 | 非模态的 `popover="auto"`，带 `aria-label` 的 `<aside>` | Escape／点面外 | 关上，焦点回到信箱键（平台行为） |
| | | Accel-B | 开着时关上，关着时打开 |
| 信箱面里的条目 | 无 APG 部件模式：一串各自可达的条目，加上 vim 的走法 | j／k | 焦点移到下一个／上一个条目，两端不环绕；条目是一行的链接或一张请决定卡本身 |
| | | 1–9 | 前九个条目各在行尾画出自己的数字；按下是跟随那一行的链接（信箱随之关上），或把焦点放到那张卡上 |
| | | 落在文本框里或带修饰键的同一批键 | 不接管 |
| | | Tab | 照旧逐个控件走：一行的链接与它的「从最后一轮分叉」是两站 |
| 请决定卡 | `role="group"`，以头一行命名 | y／e／n（焦点在卡内、不在文本框里） | 做那个答复；卡上没有的答复、或说明了为什么按不动的答复，按键落空 |
| | | Tab | 依次走过卡里的控件与答复 |
| 修改提案卡的改后接受 | 每句改动一个原生勾选框（可读名字是「取这一句」加句子的前几个字）、插入句一个原生文本框 | Space（勾选框） | 取或不取这一句；不取即拒绝这一句 |
| | | e（焦点不在文本框里） | 回到 diff，改过的字与勾选留着，直到卡被决定或消失 |
| | | y（焦点不在文本框里） | 按勾选与改过的字发出；在文本框里打 y 是字，不是答复 |
| 修改提案卡的「在原文中显示」 | APG Button | Enter／Space | 编辑器的光标落到这一段的开头并滚进视野，焦点进编辑器 |
| 三键的名字 | APG Tooltip | 单独按住加速键 300 ms | 三个名字一起出现；松开、窗口失焦或开始组合输入即收 |
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
| 设置树 | APG Disclosure Navigation | Tab／Shift+Tab | 依次走过条目 |
| | | ↓／↑ | 下一个或上一个看得见的条目 |
| | | Home／End | 第一个或最后一个条目 |
| | | Enter／Space（有子条目的条目） | 展开或收起 |
| | | Enter（组或页的条目） | 组画进正文；页移动地址栏并关上面板 |
| 设置面 | APG Dialog (Modal) | Escape、点背景、Accel-, | 关上；焦点回到打开它的控件 |
| 远程组的「配对这台设备」与「锁上门」 | APG Button | Enter／Space | 走那一次握手；进行中 `aria-disabled="true"`，结果写进组里唯一的 `role="status"` |
| 远程组的种子 | 只读文字，带 `aria-label` 的 `<output>` | 「我记下了」（APG Button）Enter／Space | 种子从页面上拿掉，焦点落到这台设备那一节的标题 |
| 远程组的「忘掉这台设备」 | APG Button，打开 `parts/dialog.svelte` | Enter／Space | 确认后删掉本地记录，焦点回到组标题；取消时回到这个按钮 |

`aria-*`：硬币键的可读名字是朝上那一面的动作——发送（steer 时写出落点）或停止——翻面时名字随之换，而换名不进实时区域，因为它不是新闻；变淡的发送面 `aria-disabled="true"`，`aria-describedby` 指向说明原因的提示。上下文环 `aria-valuemin="0"`、`aria-valuemax` 是窗口、`aria-valuenow` 是用掉的 token，`aria-valuetext` 是提示那一行。房间菜单的列表以「谁在听」那一段为 `aria-describedby`，所以读屏器在列表获得焦点时连同它一起读出；那一段里没有可聚焦的东西，菜单的键全归列表。图层键的名字是「图层 · <当前档>」。设置树是 `<nav>` 加 `aria-label`，页条目是 `<a href>`（中键与新标签页照常可用），组条目是按钮；当前页（设置面下面那一页）`aria-current="page"`，当前组 `aria-current="true"`；有子条目的条目带 `aria-expanded` 与 `aria-controls`；每个枝的列表以枝的标签为 `aria-labelledby`。设置面以当前组的标题为 `aria-labelledby`，打开时焦点落在当前组的条目上。分隔线是 `role="separator"`，带 `aria-orientation`、以栏数计的 `aria-valuenow`／`aria-valuemin`／`aria-valuemax`，`aria-controls` 指向前一栏。右侧编辑器与终端之间的分隔线以行计（一行是三个基线步，24 px）。检视面的页签带是 `role="tablist"`，整条是一个 Tab 站（游走的 `tabindex`），每个页签 `aria-selected` 说它是不是最后碰过的那一项、`aria-controls` 指向它所在的区（编辑器区与终端区各是一个 `role="tabpanel"`）；页签上的关闭记号给指针用，`tabindex="-1"`，键盘走 Delete。「原文」带 `aria-pressed`。
-/

/-!
## 模型：图层键、硬币键与信箱的走法

规定 §7-11 里三个外壳控件的状态机；分隔线在 `client/spec/Core/Workbench.lean`，检视面的页签带在 `client/spec/Views/Inspect/Open.lean`，请决定卡在 `client/spec/Views/Parts/Decide.lean`。

1. **图层键**（`views/edge.svelte`）：按一下按 zen → blend → panorama → zen 循环（三档的名字是 client D17），按三下回到原档（`three_presses_come_home`）；按住超过 300 ms 临时进 blend，松开回到人选定的档，看一眼不改选定的档（`a_peek_keeps_the_chosen_tier`）。
2. **硬币键**（`views/talk/`，client D18）：做朝上那一面，变淡的发送面一按落进空操作（`a_faded_face_does_nothing`），停止面只发 `cancel`、从不发出框里的字（`stop_never_sends`）。
3. **信箱的条目**（`client/src/views/mailbox/entries.ts`）：j／k 与 `RowList` 同一种钳住的走法，到最后一条不再走（`k_stops_at_the_last_entry`）；1–9 只落到前九个、且存在的条目上（`a_digit_reaches_only_a_drawn_entry`）。
-/

namespace Client.Views.Workspace

open Client.Views.Parts

/-- 三档（client D17）。 -/
inductive Tier where
  | zen
  | blend
  | panorama
  deriving DecidableEq, Repr

/-- 按一下图层键。 -/
def nextTier : Tier → Tier
  | .zen => .blend
  | .blend => .panorama
  | .panorama => .zen

/-- 图层的状态：人选定的档，与是否正按住看一眼。 -/
structure Layer where
  chosen : Tier
  peeking : Bool
  deriving DecidableEq, Repr

/-- 画出来的档：按住时是 blend，否则是选定的档。 -/
def shown (layer : Layer) : Tier := if layer.peeking then .blend else layer.chosen

/-- 按住超过 300 ms。 -/
def hold (layer : Layer) : Layer := { layer with peeking := true }

/-- 松开。 -/
def release (layer : Layer) : Layer := { layer with peeking := false }

theorem three_presses_come_home (tier : Tier) : nextTier (nextTier (nextTier tier)) = tier := by
  cases tier <;> rfl

theorem a_peek_shows_blend (layer : Layer) : shown (hold layer) = .blend := rfl

theorem a_peek_keeps_the_chosen_tier (layer : Layer) :
    (release (hold layer)).chosen = layer.chosen ∧ shown (release (hold layer)) = layer.chosen :=
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

theorem a_faded_face_does_nothing : pressCoin (.send true) = .nothing := rfl

theorem stop_never_sends : pressCoin .stop ≠ .words := by decide

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

end Client.Views.Workspace
