-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception::pairing

规定 `reception::pairing`（`crates/wire/src/` 下同名的文件）。这台电脑上的这扇门对浏览器的状态：终端上的配对码、`/web` 交出的开页码、挑战用的 nonce、配过对的设备钥与活着的会话令牌。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-95 配对的状态机：一码一猜、全局限速、没有时限

```rust
pub struct BrowserDoor { … }                       // 纯：熵与时间都是入参
impl BrowserDoor {
    pub fn new(devices: Vec<PairedBrowser>, code_entropy: [u8; 32]) -> Self;
    pub fn pairing_code(&self) -> &str;            // 终端显示的那一个，`abcd-efgh`
    pub fn guess(&mut self, word: &str, now: TimeMs, entropy: [u8; 32]) -> Guess;
    pub fn issue_open_code(&mut self, entropy: [u8; 16], now: TimeMs) -> String;
    pub fn redeem_open_code(&mut self, code: &str, now: TimeMs) -> Option<B3Hash>;   // 兑掉的码的摘要，用来找到等它的人
    pub fn pair(&mut self, line: PairedBrowser) -> DeviceId;                      // PairedBrowser::new(key, label, now)
    pub fn challenge(&mut self, entropy: [u8; 32], now: TimeMs) -> String;
    pub fn take_nonce(&mut self, nonce: &str, now: TimeMs) -> bool;
    pub fn device_key(&self, device: &DeviceId) -> Option<&DeviceKey>;
    pub fn open_session(&mut self, device: &DeviceId, entropy: [u8; 32], now: TimeMs) -> Option<String>;
    pub fn sessions(&self) -> &Sessions;
    pub fn sessions_mut(&mut self) -> &mut Sessions;  // socket 入座与离座（spec/Server/Sessions.lean §8-93s）
    pub fn forget(&mut self, device: &DeviceId) -> bool;
    pub fn browsers(&self) -> Vec<PairedBrowser>;
    pub fn devices(&self) -> DevicesAnswer;
}
pub enum Guess { Paired, Wrong, TooSoon }
```

- **配对码**：8 个符号，两组四个（`abcd-efgh`），取 `wire::auth` 那张去掉易混字符的 29 符号字母表，约 39 bit。**一码一猜**：每一次被判过的猜测，对错都换一个新码，所以一个码最多被判一次。**全局限速**：两次被判的猜测至少隔 `GUESS_INTERVAL_MS`（1000 ms），太早到的猜测答 `TooSoon`，不判、不换码。随机猜中一次的期望要 2^38 秒量级。**没有时限**：码只对到得了回环的人有用，限时只给手脚不便的人添负担。
- **开页码**：`/web` 铸的 128 bit 码，只存摘要，`OPEN_CODE_LIFETIME_MS`（120 s）内一次有效，最多同时 `OPEN_CODES_MAX` 个；它不受限速，因为 128 bit 猜不中。兑掉时通知交出它的那一方删跳转文件。
- **设备钥**：页面生成的不可导出 Ed25519 钥的公钥半边，32 字节，线上写成 64 个小写十六进制字符。设备 id 是公钥 BLAKE3 摘要的前 16 个十六进制字符，所以同一把钥再配一次得到同一个 id、覆盖旧的一行。只存公钥。
- **nonce**：`/session/challenge` 给的 32 字节，十六进制；`NONCE_LIFETIME_MS`（60 s）内一次有效，最多 `NONCES_MAX` 个在外，满了丢最旧的。不论签名对不对，交上来就作废。
- **签的是什么**：`sprawling local session v1\n<nonce>\n<origin>` 的 UTF-8 字节，`origin` 是这次请求过了入口判定的 Origin。带标签与源，签名就不能挪到别的协议或别的源去用。
- **会话令牌**：32 字节，十六进制，只存摘要，属于签出它的那台设备；表与它的寿命住 `reception::sessions`（`spec/Server/Sessions.lean` §8-93s）：被 socket 占着时一直有效，没有 socket 占着时空闲 `SESSION_IDLE_MS` 即失效，设备被忘掉或进程结束即失效，忘掉时占着它的 socket 也被关掉。页面只把它放在内存里，每次拨号前再签一次。
- **失败**：配对码错、太早、开页码错或过期一律 `E_PAIRING_REFUSED`，不说是哪一种原因（太早除外：恢复办法写「等一秒」，因为对手已经知道限速存在）；会话的 nonce、设备或签名不对是 `E_GATE_DENIED`。

下面的模型只写配对码那一半，因为它是唯一一个对手能反复试的部分：开页码 128 bit，nonce 与会话令牌都由城铸出、一次有效。性质对每一条猜测序列成立，由 `reception::pairing` 旁的 proptest `every_trace_judges_each_code_once_and_a_second_apart` 在随机序列上对照 Rust。
-/

namespace Wire.Reception.Pairing

/-- 两次被判的猜测之间至少隔这么多毫秒（`GUESS_INTERVAL_MS`）。 -/
def interval : Nat := 1000

/-- 配对码那一半的状态：当前的码，与下一次猜测最早何时会被判。码在模型里是一个抽象的值。 -/
structure State where
  code : Nat
  nextGuess : Nat
  deriving DecidableEq, Repr

/-- 一次猜测：到达的时刻、猜的词，以及这一步若要换码会换成的新码（Rust 里是这一步的熵）。 -/
structure Attempt where
  time : Nat
  word : Nat
  fresh : Nat
  deriving DecidableEq, Repr

inductive Guess where
  | paired
  | wrong
  | tooSoon
  deriving DecidableEq, Repr

/-- `BrowserDoor::guess`。 -/
def guess (state : State) (attempt : Attempt) : Guess × State :=
  if attempt.time < state.nextGuess then (.tooSoon, state)
  else
    let after : State := { code := attempt.fresh, nextGuess := attempt.time + interval }
    if attempt.word = state.code then (.paired, after) else (.wrong, after)

/-- 一次被判过的猜测：何时、对着哪个码判的、猜的词、判了什么、之后换成了哪个码。 -/
structure Judged where
  time : Nat
  code : Nat
  word : Nat
  verdict : Guess
  fresh : Nat
  deriving DecidableEq, Repr

/-- 一条猜测序列里被判过的那些，按次序。 -/
def judged : State → List Attempt → List Judged
  | _, [] => []
  | state, attempt :: rest =>
    let (verdict, next) := guess state attempt
    match verdict with
    | .tooSoon => judged next rest
    | _ => { time := attempt.time, code := state.code, word := attempt.word, verdict := verdict,
             fresh := attempt.fresh } :: judged next rest

/-- 被判的猜测从 `floor` 起，彼此至少隔 `interval`。 -/
def Apart : Nat → List Judged → Prop
  | _, [] => True
  | floor, j :: rest => floor ≤ j.time ∧ Apart (j.time + interval) rest

/-- 每一次被判的猜测对着的都是上一次判完换上的码，第一次对着起始的码：一个码最多被判一次。 -/
def Chained : Nat → List Judged → Prop
  | _, [] => True
  | code, j :: rest => j.code = code ∧ Chained j.fresh rest

/-- 判成配对的，猜的词恰是当时的码。 -/
def Honest : List Judged → Prop
  | [] => True
  | j :: rest => (j.verdict = .paired ↔ j.word = j.code) ∧ j.verdict ≠ .tooSoon ∧ Honest rest

/-- **全局限速**：任何一条序列里，两次被判的猜测至少隔一秒，第一次不早于起始状态允许的时刻。 -/
theorem judged_guesses_are_a_second_apart (state : State) (attempts : List Attempt) :
    Apart state.nextGuess (judged state attempts) := by
  induction attempts generalizing state with
  | nil => trivial
  | cons attempt rest ih =>
    by_cases early : attempt.time < state.nextGuess
    · simp only [judged, guess, early, ↓reduceIte]
      exact ih state
    · by_cases right : attempt.word = state.code
      · simp only [judged, guess, early, right, ↓reduceIte, Apart]
        exact ⟨by omega, ih { code := attempt.fresh, nextGuess := attempt.time + interval }⟩
      · simp only [judged, guess, early, right, ↓reduceIte, Apart]
        exact ⟨by omega, ih { code := attempt.fresh, nextGuess := attempt.time + interval }⟩

/-- **一码一猜**：任何一条序列里，每次被判都对着上一次换上的码。 -/
theorem each_code_is_judged_once (state : State) (attempts : List Attempt) :
    Chained state.code (judged state attempts) := by
  induction attempts generalizing state with
  | nil => trivial
  | cons attempt rest ih =>
    by_cases early : attempt.time < state.nextGuess
    · simp only [judged, guess, early, ↓reduceIte]
      exact ih state
    · by_cases right : attempt.word = state.code
      · simp only [judged, guess, early, right, ↓reduceIte, Chained]
        exact ⟨trivial, ih { code := attempt.fresh, nextGuess := attempt.time + interval }⟩
      · simp only [judged, guess, early, right, ↓reduceIte, Chained]
        exact ⟨trivial, ih { code := attempt.fresh, nextGuess := attempt.time + interval }⟩

/-- **只有猜中当时的码才配对**，且太早的猜测从不被判。 -/
theorem only_the_current_code_pairs (state : State) (attempts : List Attempt) :
    Honest (judged state attempts) := by
  induction attempts generalizing state with
  | nil => exact True.intro
  | cons attempt rest ih =>
    by_cases early : attempt.time < state.nextGuess
    · simp only [judged, guess, early, ↓reduceIte]
      exact ih state
    · by_cases right : attempt.word = state.code
      · simp only [judged, guess, early, right, ↓reduceIte, Honest]
        exact ⟨by simp, by simp, ih _⟩
      · simp only [judged, guess, early, right, ↓reduceIte, Honest]
        exact ⟨by simp, by simp, ih _⟩

/-- 一条可以实现的序列：猜错一次，一秒后用换上的码配上。 -/
example :
    (judged { code := 7, nextGuess := 0 }
      [{ time := 0, word := 3, fresh := 9 }, { time := 500, word := 9, fresh := 4 },
       { time := 1000, word := 9, fresh := 5 }]).map Judged.verdict = [.wrong, .paired] := rfl

end Wire.Reception.Pairing
