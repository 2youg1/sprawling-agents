-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import tools.citysim.spec.WireScript

/-!
# citysim::wire_script::exchange

规定 `citysim::wire_script::exchange`（`tools/citysim/src/wire_script/exchange.rs`）：从一条回环连接上读一个请求、把脚本给它的回答写回去、把这次交换追加进记录，以及第一轮找不到还没开启的 run 时把脚本文件再读一遍。哪一个请求得到哪一个回答由 `spec/WireScript.lean` 的 `Replay` 判；本文件是 `tools/citysim/Spec.lean` 的一个分部，接着那里的 §8-10 写这一半。

能写成定理的是重读：脚本文件只在一个第一轮被 `no_run_left` 拒时再读，再读读不成或接不上时那次拒绝照旧、回放的状态不变，接得上时那个第一轮开启追加的 run。凭据头的值不落盘是一个判断的两臂，不写成定理，由 `the_same_request_twice_is_recorded_byte_for_byte_alike` 判记录里没有那个 key；请求怎样从字节里读出来（`read_request`）同样由 Rust 守着，见 §16。
-/

/-!
### 8-10（续）一次交换：什么算一个请求，记录写什么

**什么算一个请求，只在读的那一处定**：头读到空行，再按 `content-length`（缺省为 0）读完正文，才是一个请求；连接在那之前断了、静了 30 s（`QUIET`）、读过的字节超过 64 MiB（`LARGEST`），或 `content-length` 读不成数，都不算一个请求：不回答、不记录、不花掉脚本里的一条，否则之后的每一轮都在答前一轮的问题。正文按 UTF-8 读，读不成的字节换成替换字符。一条连接一个请求，回答带 `connection: close`；连接一条接一条地答，顺序就是到达顺序。

**记录**：每次交换一行 JSON，追加进 `record`：`seq`（从 1 起）、`run` 与 `reply`（这次回答出自第几个 run 的第几条，都从 0 起；没有放进任何 run 的请求两者都是 `null`，§8-13）、`method`、`target`、`headers`（到达顺序，名字小写，值去掉两端空白）、`body`（请求正文的文字）、`status`、`answer`（回答的正文）。所以一份记录按 `run` 筛出来，就是那个 run 自己的对话。同一份脚本收到同样的请求字节，记录逐字节相同。**凭据头的值不落盘**：名字是 `authorization`、`proxy-authorization` 或以 `api-key` 结尾的头，值写成 `redacted`（`exchange::REDACTED`）。检查要的是「带了凭据」，不是凭据本身，而人在测试时填的 key 不能出现在任何文件里；`exchange::is_credential` 是这条规则唯一的一处。

一次交换先记录、再写回答：回答写不出去时这次交换已经记下，脚本的那一条也已花掉，`answer_one` 以 `E_TOOL_UNAVAILABLE` 报它，`bin/provider` 写到标准错误后接着答下一条。重读接不上时 `answer_one` 先把 `no_run_left` 答完、记下，再以 `E_CONFIG_INVALID` 报接不上，`bin/provider` 同样只写到标准错误。
-/

namespace Citysim.WireScript.Exchange

open Citysim.WireScript

variable {Face : Type}

/-- 再读一遍脚本文件的结果：读成一份脚本，或读不成（`E_CONFIG_INVALID`）。 -/
inductive Reread (Face : Type) where
  | Read (script : WireScript Face)
  | Unreadable

/-- 这个回答是不是一个找不到 run 的第一轮：再读一遍脚本只能改变这一种拒绝（`Answer::wants_more_runs`）。 -/
def wants_more_runs : Answer → Bool
  | .Refused .NoRunLeft => true
  | _ => false

/-- D15 **脚本的 run 用完时，替身先把脚本文件再读一遍，再拒一个新开的 run。** 有的回复要写进城才知道的东西：一个要评审的房间的分支名由城按地址的摘要取（`room-<摘要前 16 位>`），而黑盒检查不预测任何摘要（`tools/adversary/Spec.lean` §2 第 3 条）。所以检查先从历史里读出它，再把要用它的那个 run 追加进脚本文件；替身在一个新开的 run 找不到还没开启的 run 时把文件再读一遍，读到的脚本若以它已经握着的那些 run 开头（`face`、`models` 与这些 run 逐值相等），多出来的 run 接在后面（`a_run_appended_after_the_script_ran_out_is_opened`）；否则这次重读被拒（`E_CONFIG_INVALID`，写到标准错误），这个新开的 run 照样得到 `no_run_left`（`a_reread_that_is_not_taken_changes_nothing`）。已经开启的 run 永远按它开启时的回复作答，追加改不动它们（`a_grown_replay_answers_every_held_run_alike`）。

被否：①按请求里的内容拼回复（把分支名从上一次工具结果里抄过来）——替身就在写自己的文字了，D11 的那条线就没了；②走到半路再起第二个替身——城要改挂第二个 URL，而起进程的是配方，不是检查；③每个请求都重读文件——每次交换都多付一次读盘，而且一个已经开启的 run 会在它脚下被换掉（`the_file_is_read_again_only_for_a_run_none_is_left_for`）。**重开参数**：要在一个已经开启的 run 里插进城才知道的东西时，这条不够用，要重新判断替身能不能答一个它看不见的值。

一次交换的回答（`ScriptedProvider::answered`）：先按握着的脚本答；是 `no_run_left` 时再读一遍文件，接得上就重答一次，接不上就留下那次拒绝，并把接不上这件事（第三项为 `true`）随回答一起交出去。 -/
def answered [DecidableEq Face] (replay : Replay Face) (asked : Asked) (body : Body)
    (again : Reread Face) : (Option Turn × Answer) × Replay Face × Bool :=
  let (first, held) := replay.answer asked body
  if wants_more_runs first.2 then
    match again with
    | .Read script =>
      match held.grow script with
      | some grown => let (second, after) := grown.answer asked body; (second, after, false)
      | none => (first, held, true)
    | .Unreadable => (first, held, true)
  else (first, held, false)

/-- D15：脚本文件只在一个第一轮被 `no_run_left` 拒时再读；其余每个请求的回答与文件此刻写着什么无关。 -/
theorem the_file_is_read_again_only_for_a_run_none_is_left_for [DecidableEq Face]
    (replay : Replay Face) (asked : Asked) (body : Body) (one other : Reread Face)
    (answered_already : wants_more_runs (replay.answer asked body).1.2 = false) :
    answered replay asked body one = answered replay asked body other := by
  simp [answered, answered_already]

/-- 再读读不成，或读到的脚本接不上时，那次 `no_run_left` 照旧，回放的状态不变。 -/
theorem a_reread_that_is_not_taken_changes_nothing [DecidableEq Face] (replay : Replay Face)
    (asked : Asked) (body : Body) (again : Reread Face)
    (refused : (replay.answer asked body).1.2 = .Refused .NoRunLeft)
    (not_taken : ∀ script, again = .Read script → (replay.answer asked body).2.grow script = none) :
    answered replay asked body again
      = ((replay.answer asked body).1, (replay.answer asked body).2, true) := by
  cases again with
  | Read script =>
    simp [answered, refused, wants_more_runs, not_taken script rfl]
  | Unreadable =>
    simp [answered, refused, wants_more_runs]

/-- 每个 run 都开启过时，往脚本文件后面追加一个 run，下一个第一轮开启它、拿到它的第一条（`a_run_written_after_the_script_ran_out_is_opened`）：这是验收世界在检查走到半路才知道分支名时用的那条路。 -/
theorem a_run_appended_after_the_script_ran_out_is_opened [DecidableEq Face]
    (replay : Replay Face) (strings : List String) (run : List Reply) (first : Reply)
    (all_opened : replay.opened = replay.script.runs.length)
    (opening_old : located replay.script strings = [])
    (opening_new : located ⟨replay.script.face, replay.script.models, replay.script.runs ++ [first :: run]⟩ strings = []) :
    (answered replay .Chat (.Json strings)
      (.Read ⟨replay.script.face, replay.script.models, replay.script.runs ++ [first :: run]⟩)).1
      = (some ⟨replay.opened, 0⟩, .Reply first) := by
  have none_left : replay.script.runs[replay.opened]? = none :=
    List.getElem?_eq_none (Nat.le_of_eq all_opened.symm)
  have begins : ∀ (held : List (List Reply)) more, starts_with held (held ++ more) = true := by
    intro held more
    induction held with
    | nil => cases more <;> rfl
    | cons head rest ih => simp [starts_with, ih]
  have appended :
      (replay.script.runs ++ [first :: run])[replay.opened]? = some (first :: run) := by
    simp [all_opened]
  simp [answered, Replay.answer, Replay.chat, opening_old, opening_new, carriedOf,
    Replay.opening, WireScript.reply, none_left, wants_more_runs, Replay.grow, begins, appended]

end Citysim.WireScript.Exchange
