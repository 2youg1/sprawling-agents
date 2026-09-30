-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# keyboard：一次按键里，键是嵌套着按下又放开的

规定 `crates/browser/src/keyboard.rs`（`browser::keyboard`，`key_frame`）。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。决定见入口的 D12。

一次按键是一帧 `input.performActions` 的 `key` 源：修饰键按枚举次序逐个按下，这个键按下又放开，修饰键按相反次序放开。修饰键是集合，按下的次序由枚举给出，所以同一次按键恒是同一串字节。

两条性质：

1. **按下的键都被放开，而且嵌套**（`press_nests`）：每一次放开的都是此刻最后按下、还没放开的那个键；一个按着修饰键追踪按键的页面，看到的恰是一个人按 Ctrl+Shift+K 时的次序。
2. **放开的次序若不反过来，就不嵌套**（`withoutReverse_does_not_nest`）：本模型咬得动的演示。这条缺陷在实现的第一版里真的出现过：红测试读到 Control 与 Shift 按按下的次序放开。
-/

namespace Browser.Keyboard

inductive Stroke (α : Type) where
  | down (key : α)
  | up (key : α)
  deriving DecidableEq, Repr

/-- 按住的键是一个栈：放开的必须是栈顶。全部放开、没有多余的放开，才答 `true`。 -/
def nests {α : Type} [DecidableEq α] : List α → List (Stroke α) → Bool
  | [], [] => true
  | _ :: _, [] => false
  | held, .down k :: rest => nests (k :: held) rest
  | [], .up _ :: _ => false
  | h :: held, .up k :: rest => if h = k then nests held rest else false

/-- `key_frame` 的次序：`modifiers` 已按枚举次序去重排好。 -/
def press {α : Type} (modifiers : List α) (key : α) : List (Stroke α) :=
  modifiers.map .down ++ [.down key, .up key] ++ modifiers.reverse.map .up

theorem nests_downs {α : Type} [DecidableEq α] (ms held : List α) (rest : List (Stroke α)) :
    nests held (ms.map .down ++ rest) = nests (ms.reverse ++ held) rest := by
  induction ms generalizing held with
  | nil => simp
  | cons m ms ih =>
    simp only [List.map_cons, List.cons_append, nests]
    rw [ih]
    simp

theorem nests_ups {α : Type} [DecidableEq α] (ms held : List α) (rest : List (Stroke α)) :
    nests (ms ++ held) (ms.map .up ++ rest) = nests held rest := by
  induction ms with
  | nil => simp
  | cons m ms ih => simp [nests, ih]

theorem press_nests {α : Type} [DecidableEq α] (modifiers : List α) (key : α) :
    nests [] (press modifiers key) = true := by
  have ups := nests_ups modifiers.reverse [] []
  rw [List.append_nil, List.append_nil, List.map_reverse] at ups
  unfold press
  rw [List.append_assoc, nests_downs, List.append_nil]
  simp [nests, ups]

/-!
## 咬得动的演示

放开修饰键时不把次序反过来，`press_nests` 不再成立：按下 Control（记作 0）与 Shift（记作 1）之后先放开 Control，而栈顶是 Shift。
-/

def pressWithoutReverse {α : Type} (modifiers : List α) (key : α) : List (Stroke α) :=
  modifiers.map .down ++ [.down key, .up key] ++ modifiers.map .up

theorem withoutReverse_does_not_nest :
    nests [] (pressWithoutReverse [0, 1] 7) = false := by
  decide

end Browser.Keyboard
