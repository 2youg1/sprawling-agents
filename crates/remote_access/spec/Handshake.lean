-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# 配对握手：配对码只封给二维码钉住的那座城

规定 `crates/remote_access/src/handshake/pairing.rs`，握手里配对的那一半（crates/remote_access/Spec.lean §8-6）：
一台还没配对的设备第一次连上城时，线上走哪几条消息，城在什么时候把一个配对码交给门去兑。
Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。门自己的性质（码只兑
一次、只在本纪元、只在到期之前，等等）在 `crates/remote_access/spec/Door.lean`，这里不重述：
本模型到「城把码交给门」为止。

通路（Cloudflare 命名隧道、人写的一条命令）搬运每一个字节，还可能在边缘解开 TLS，所以
这里的通路是对手：每一条明文消息它都看得见，任意一条回答它都能送到设备面前。密码学不在
模型里，由下面四条假设代替；它们由标准与 Rust 一侧的测试承担，本文件不证明它们。

* **A1 签名不可伪造**：不持有私钥，造不出对这一次握手记录的签名。模型里对手可以随意填
  回答里的 `presented` 与 `signer`；A1 说的是，两者相等而对手又不持有那把私钥的回答，在
  真实的线上验不过。下面的性质不依赖 A1，因为认领封给的正是回答里出示的那把公钥（A2）。
* **A2 封装只有签名者打得开**：会话密钥由设备的临时密钥与回答方的临时密钥派生，回答方的
  临时公钥由签名绑进握手记录（A1），所以封在这次握手之下的负载，只有签了这次握手的那把
  私钥的持有人打得开。模型把它写成 `Wire.reveals` 的定义。
* **A3 指纹无碰撞**：两把不同的公钥，SHA-256 指纹不同。它是定理的前提 `collisionFree`，
  不是公理。
* **A4 邀请不经通路**：城的指纹与配对码从控制台屏幕经二维码直接到设备，放在 URL 片段里，
  片段不随请求发出。模型里它们是设备状态的参数，从不来自 `Wire`。改写页面的通路能读到页面
  读到的一切，那是 remote_access D10 写下的边界，本模型不覆盖。

三条性质，各一组定理：

* **配对码只到钉住的城**：设备发出的消息里，带配对码的只有认领，它封给一把指纹与邀请相同
  的公钥；回答对不上，设备什么都不再发。
* **城只从封好的认领里兑码**：城交给门的每一个码，都来自一条封给城自己公钥的认领；明文
  消息里没有放配对码的位置。一条配对连接只有「发消息」与「兑码」两种效果，所以它开不了
  远程会话，也不转发任何帧。
* **诚实的一次配对走得通**：回答对得上邀请时设备发出认领，城收到认领时兑码，而且只兑一次；
  上面两条不是靠拒绝一切得来的。
-/

namespace RemoteAccess.Handshake

/-- 一把长期公钥，按身份抽象成数：城的，或设备的。 -/
abbrev Key := Nat
/-- 公钥的指纹；Rust 一侧是 `CityFingerprint::of`，公钥字节的 SHA-256。 -/
abbrev Fingerprint := Nat
/-- 配对码的正文。 -/
abbrev Code := Nat

/-- 通路上的一条消息，按通路看得到的样子。 -/
inductive Wire where
  /-- 设备开口（`PairHello`）：临时公钥与 nonce。不带设备 id，也没有放配对码的位置。 -/
  | hello
  /-- 城的回答（`PairReply`）：`presented` 是它出示的长期公钥，`signer` 是签了这次握手记录
  的那把私钥所对应的公钥。 -/
  | reply (presented signer : Key)
  /-- 设备的认领（`Claim`），封在这次握手派生的会话密钥下：配对码与设备公钥。`to` 是签了
  这次握手的那把公钥。 -/
  | claim (to : Key) (code : Code) (device : Key)
  deriving DecidableEq, Repr

/-- 持有 `holder` 那把私钥的一方，能从一条消息里读到配对码 `c`：只有认领带配对码，而认领
只有签了那次握手的一方打得开（A2）。 -/
def Wire.reveals (holder : Key) (c : Code) : Wire → Prop
  | .hello => False
  | .reply _ _ => False
  | .claim to code _ => to = holder ∧ code = c

/-! ## 设备一侧 -/

/-- 设备在一条配对连接上的状态。 -/
inductive Device where
  /-- 已发出 `hello`，等城回答。`pin` 是邀请里的指纹，`code` 是邀请里的配对码，`me` 是设备
  刚生成的公钥；三者都来自二维码与设备自己，不来自通路（A4）。 -/
  | waiting (pin : Fingerprint) (code : Code) (me : Key)
  /-- 认领已发出，封给 `city`。 -/
  | claimed (city : Key)
  /-- 回答对不上，连接结束，什么都没发。 -/
  | refused
  deriving DecidableEq, Repr

/-- 设备收到一条消息：新状态，以及它发出的消息。只有等待中的设备会对回答作出反应；回答
里出示的公钥指纹须等于 `pin`，签名须出自那把公钥（Rust 的 `DevicePairing::claim`）。 -/
def Device.step (fp : Key → Fingerprint) : Device → Wire → Device × List Wire
  | .waiting pin code me, m =>
    match m with
    | .reply presented signer =>
      if fp presented = pin ∧ signer = presented then
        (.claimed presented, [.claim presented code me])
      else (.refused, [])
    | .hello => (.refused, [])
    | .claim _ _ _ => (.refused, [])
  | .claimed city, _ => (.claimed city, [])
  | .refused, _ => (.refused, [])

/-- 设备对通路送来的一串消息依次作答，收集它发出的全部消息。 -/
def Device.run (fp : Key → Fingerprint) : Device → List Wire → List Wire
  | _, [] => []
  | s, m :: ms => (s.step fp m).2 ++ Device.run fp (s.step fp m).1 ms

/-- 一次配对里设备发出的一切：开口的 `hello`，然后对通路送来的每条消息作答。 -/
def Device.transcript (fp : Key → Fingerprint) (pin : Fingerprint) (code : Code) (me : Key)
    (incoming : List Wire) : List Wire :=
  .hello :: Device.run fp (.waiting pin code me) incoming

/-- 一条消息是封给某把指纹为 `pin` 的公钥的认领。 -/
def SealedToPin (fp : Key → Fingerprint) (pin : Fingerprint) : Wire → Prop
  | .hello => False
  | .reply _ _ => False
  | .claim to _ _ => fp to = pin

theorem claimed_is_silent (fp : Key → Fingerprint) (city : Key) :
    ∀ ms, Device.run fp (.claimed city) ms = [] := by
  intro ms
  induction ms with
  | nil => rfl
  | cons _ _ ih => simp [Device.run, Device.step, ih]

theorem refused_is_silent (fp : Key → Fingerprint) :
    ∀ ms, Device.run fp .refused ms = [] := by
  intro ms
  induction ms with
  | nil => rfl
  | cons _ _ ih => simp [Device.run, Device.step, ih]

/-! ## 配对码只到钉住的城 -/

/-- 等待中的设备，不论通路送来什么，发出的每一条消息都是封给指纹为 `pin` 的公钥的认领。 -/
theorem device_seals_only_to_the_pin (fp : Key → Fingerprint) (pin : Fingerprint)
    (code : Code) (me : Key) (incoming : List Wire) :
    ∀ m ∈ Device.run fp (.waiting pin code me) incoming, SealedToPin fp pin m := by
  intro m hm
  cases incoming with
  | nil => simp [Device.run] at hm
  | cons x xs =>
    cases x with
    | reply presented signer =>
      by_cases h : fp presented = pin ∧ signer = presented
      · simp [Device.run, Device.step, h, claimed_is_silent] at hm
        subst hm
        simp only [SealedToPin]
        exact h.1
      · simp [Device.run, Device.step, h, refused_is_silent] at hm
    | hello => simp [Device.run, Device.step, refused_is_silent] at hm
    | claim _ _ _ => simp [Device.run, Device.step, refused_is_silent] at hm

/-- 一次配对的全部线上消息里，能读到配对码的只有邀请钉住的那座城。 -/
theorem only_the_pinned_city_reads_the_code (fp : Key → Fingerprint)
    (collisionFree : ∀ a b, fp a = fp b → a = b) (city : Key) (code : Code) (me : Key)
    (incoming : List Wire) (holder : Key) (c : Code) :
    ∀ m ∈ Device.transcript fp (fp city) code me incoming,
      m.reveals holder c → holder = city := by
  intro m hm hr
  simp only [Device.transcript, List.mem_cons] at hm
  rcases hm with rfl | hm
  · simp [Wire.reveals] at hr
  · have sealed := device_seals_only_to_the_pin fp (fp city) code me incoming m hm
    cases m with
    | hello => simp [Wire.reveals] at hr
    | reply _ _ => simp [Wire.reveals] at hr
    | claim to _ _ =>
      simp only [Wire.reveals] at hr
      simp only [SealedToPin] at sealed
      rw [← hr.1]
      exact collisionFree to city sealed

/-- 出示一把指纹不对的公钥，冒充者什么也得不到：设备不再发任何消息。 -/
theorem impostor_gets_nothing (fp : Key → Fingerprint) (pin : Fingerprint) (code : Code)
    (me impostor signer : Key) (wrong : fp impostor ≠ pin) (rest : List Wire) :
    Device.run fp (.waiting pin code me) (.reply impostor signer :: rest) = [] := by
  simp [Device.run, Device.step, wrong, refused_is_silent]

/-! ## 城只从封好的认领里兑码 -/

/-- 城在一条配对连接上的状态。 -/
inductive City where
  | listening
  | replied
  | done
  deriving DecidableEq, Repr

/-- 城对一条配对连接能做的全部事情：往通路上发一条消息，或者把一个码与一把设备公钥交给门
去兑（门模型的 `Door.redeem`，Rust 的 `Door::pair`）。没有开会话，也没有转发帧。 -/
inductive Effect where
  | send (m : Wire)
  | redeem (code : Code) (device : Key)
  deriving DecidableEq, Repr

/-- 城收到一条消息。开口之后城回答并签名；回答之后到来的第一条消息若是封给城自己的认领，
城把它交给门；不论如何，连接随即结束（Rust 的 `CityPairing::open_claim` 只能用一次）。 -/
def City.step (me : Key) : City → Wire → City × List Effect
  | .listening, m =>
    match m with
    | .hello => (.replied, [.send (.reply me me)])
    | .reply _ _ => (.done, [])
    | .claim _ _ _ => (.done, [])
  | .replied, m =>
    (.done, match m with
      | .claim to code device => if to = me then [.redeem code device] else []
      | .hello => []
      | .reply _ _ => [])
  | .done, _ => (.done, [])

/-- 城对一串消息依次作答，收集它的全部效果。 -/
def City.run (me : Key) : City → List Wire → List Effect
  | _, [] => []
  | s, m :: ms => (City.step me s m).2 ++ City.run me (City.step me s m).1 ms

theorem done_is_silent (me : Key) : ∀ ms, City.run me .done ms = [] := by
  intro ms
  induction ms with
  | nil => rfl
  | cons _ _ ih => simp [City.run, City.step, ih]

/-- 城交给门的每一个码，都在通路送来的消息里以一条封给城自己的认领出现过。 -/
theorem redeem_needs_a_sealed_claim (me : Key) (c : Code) (d : Key) :
    ∀ (ms : List Wire) (s : City), Effect.redeem c d ∈ City.run me s ms →
      Wire.claim me c d ∈ ms := by
  intro ms
  induction ms with
  | nil => intro _ h; simp [City.run] at h
  | cons x xs ih =>
    intro s h
    simp only [City.run, List.mem_append] at h
    rcases h with h | h
    · refine List.mem_cons.mpr (Or.inl ?_)
      cases s with
      | listening => cases x <;> simp [City.step] at h
      | done => simp [City.step] at h
      | replied =>
        cases x with
        | claim to c' d' =>
          by_cases hto : to = me
          · simp [City.step, hto] at h
            obtain ⟨rfl, rfl⟩ := h
            rw [hto]
          · simp [City.step, hto] at h
        | hello => simp [City.step] at h
        | reply _ _ => simp [City.step] at h
    · exact List.mem_cons_of_mem _ (ih _ h)

/-! ## 诚实的一次配对走得通 -/

/-- 回答对得上邀请时，设备发出且只发出一条认领。 -/
theorem honest_device_claims (fp : Key → Fingerprint) (city : Key) (code : Code) (me : Key)
    (rest : List Wire) :
    Device.run fp (.waiting (fp city) code me) (.reply city city :: rest)
      = [.claim city code me] := by
  simp [Device.run, Device.step, claimed_is_silent]

/-- 城收到开口与一条封给自己的认领，回答一次、兑码一次，之后不论通路再送什么都不再有效果。 -/
theorem honest_city_redeems_once (me : Key) (code : Code) (device : Key) (rest : List Wire) :
    City.run me .listening (.hello :: .claim me code device :: rest)
      = [.send (.reply me me), .redeem code device] := by
  simp [City.run, City.step, done_is_silent]

/-- 两侧接在一起：城对诚实设备的全部消息作答，结果是一次回答与一次兑码。 -/
theorem honest_pairing_redeems (fp : Key → Fingerprint) (city : Key) (code : Code) (me : Key) :
    City.run city .listening (Device.transcript fp (fp city) code me [.reply city city])
      = [.send (.reply city city), .redeem code me] := by
  simp [Device.transcript, Device.run, Device.step, City.run, City.step]

end RemoteAccess.Handshake
