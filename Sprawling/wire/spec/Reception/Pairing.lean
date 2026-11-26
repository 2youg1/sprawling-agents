-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception::pairing

规定 `reception::pairing`（`Sprawling/wire/src/` 下同名的文件）。这台电脑上的这扇门对浏览器的状态：终端与 `/web` 交出的开页码、挑战用的 nonce、配过对的设备钥与活着的会话令牌。本文件是 `Sprawling/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `Sprawling/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-95 浏览器配对与会话挑战

```rust
pub struct BrowserDoor { … }                       // 纯：熵与时间都是入参
impl BrowserDoor {
    pub fn new(devices: Vec<PairedBrowser>) -> Self;
    pub fn issue_open_code(&mut self, entropy: [u8; 16], now: u64) -> (String, B3Hash);
    pub fn open_code(&self, code: &str, now: u64) -> Option<B3Hash>;
    pub fn spend_open_code(&mut self, digest: &B3Hash);
    pub fn pair(&mut self, line: PairedBrowser) -> DeviceId;                      // PairedBrowser::new(key, label, now)
    pub fn device_key(&self, device: &DeviceId) -> Option<&DeviceKey>;
    pub fn open_session(&mut self, device: &DeviceId, entropy: [u8; 32], now: TimeMs) -> Option<String>;
    pub fn sessions(&self) -> &Sessions;
    pub fn sessions_mut(&mut self) -> &mut Sessions;  // socket 入座与离座（spec/Server/Sessions.lean §8-93s）
    pub fn forget(&mut self, device: &DeviceId) -> bool;
    pub fn browsers(&self) -> Vec<PairedBrowser>;
    pub fn devices(&self) -> DevicesAnswer;
}
```

- **D66 本地浏览器用开页码配对。** `/web` 的私有跳转与终端按需生成的可复制码共用一次性凭据；人在安静宿主按 C 生成，选中并复制，到同一台机器的浏览器粘贴。码为系统随机的 16 字节、写作 32 个小写十六进制字符，城只存摘要。寿命是本次 serve 单调时钟的 120 秒，最多保留四个，额满只在可信发行处淘汰最早到期的码。只有回环对端能兑换。持门锁先验证码和设备输入，再持久化登记并完成消费，成功后通知等待的私有跳转。不匹配的码保持有效凭据可兑换。码只在私有跳转文件与按用户请求绘制的终端备用屏出现。被否的八符号码需要共享限速与失败轮换，增加手输摩擦也使猜错的请求能影响别人的配对。PAKE 仍须给在线尝试设上界，本端通过方由高熵码与回环入口界定。
- **设备钥**：页面生成的不可导出 Ed25519 钥的公钥半边，32 字节，线上写成 64 个小写十六进制字符。设备 id 是公钥 BLAKE3 摘要的前 16 个十六进制字符，所以同一把钥再配一次得到同一个 id、覆盖旧的一行。只存公钥。
- **D67 会话挑战由 HMAC 签发。** `/session/challenge` 交十六进制的 8 字节大端到期时刻、32 字节随机值与 32 字节 HMAC-SHA256。到期时刻取本次 serve 启动以来的单调毫秒，寿命 60 秒。每次启动单独抽一个 HMAC 钥，签固定协议标签与前 40 字节。发挑战不写状态，验 MAC 与到期后再验设备签名。持门锁验后，在开出会话时登记已兑换的挑战，到期才能移除；每台已配设备最多同时留 64 条兑换记录，满时只让该设备等最早一条到期。错误的签名不会占位。
- **签的是什么**：`sprawling local session v1\n<nonce>\n<origin>` 的 UTF-8 字节，`origin` 是这次请求过了入口判定的 Origin。带标签与源，签名就不能挪到别的协议或别的源去用。
- **会话令牌**：32 字节，十六进制，只存摘要，属于签出它的那台设备；表与它的寿命住 `reception::sessions`（`spec/Server/Sessions.lean` §8-93s）：被 socket 占着时一直有效，没有 socket 占着时空闲 `SESSION_IDLE_MS` 即失效，设备被忘掉或进程结束即失效，忘掉时占着它的 socket 也被关掉。页面只把它放在内存里，每次拨号前再签一次。
- **失败**：码不对、过期或用过答 `E_PAIRING_REFUSED`；局域网对端兑换开页码、会话的 nonce、设备或签名不对答 `E_GATE_DENIED`。

`LocalDoor` 接收单调时间与熵。HMAC 校验后的挑战由 `BrowserDoor` 原子兑付，取代存放待答挑战的公共容量。公共表会让未认证的挑战请求使已配对设备的登录失效。

模型把发行密钥和签名验证抽象为输入，让证明约束有效期、消费与发挑战时的状态。
-/

namespace Wire.Reception.Pairing

def redeem (codes : List Nat) (attempt : Nat) (live : Bool) : Bool × List Nat :=
  if live = true ∧ attempt ∈ codes then (true, codes.filter (· != attempt)) else (false, codes)

theorem unknown_code_preserves_invitations (codes : List Nat) (attempt : Nat)
    (unknown : attempt ∉ codes) :
    redeem codes attempt true = (false, codes) := by
  simp [redeem, unknown]

theorem open_code_redeems_once (codes : List Nat) (code : Nat) (issued : code ∈ codes) :
    (redeem codes code true).1 = true ∧
      (redeem (redeem codes code true).2 code true).1 = false := by
  simp [redeem, issued]

theorem expired_code_is_refused (codes : List Nat) (code : Nat) :
    redeem codes code false = (false, codes) := by
  simp [redeem]

namespace SessionChallenge

inductive Request where
  | offer
  | answer (ticket : Nat) (verified : Bool) (now expires : Nat)
  deriving Repr

def apply (spent : List Nat) : Request → Bool × List Nat
  | .offer => (false, spent)
  | .answer ticket verified now expires =>
    if verified = true ∧ now < expires ∧ ticket ∉ spent then
      (true, ticket :: spent)
    else
      (false, spent)

def offers (spent : List Nat) : Nat → List Nat
  | 0 => spent
  | n + 1 => (apply (offers spent n) .offer).2

theorem offers_preserve_challenges (spent : List Nat) (n : Nat) :
    offers spent n = spent := by
  induction n with
  | zero => rfl
  | succ n ih => simp [offers, apply, ih]

theorem unexpired_fresh_challenge_redeems (spent : List Nat) (ticket now expires n : Nat)
    (fresh : ticket ∉ spent) (live : now < expires) :
    apply (offers spent n) (.answer ticket true now expires) = (true, ticket :: spent) := by
  simp [offers_preserve_challenges, apply, live, fresh]

theorem challenge_redeems_at_most_once (spent : List Nat) (ticket now later expires : Nat)
    (fresh : ticket ∉ spent) (live : now < expires) :
    (apply (apply spent (.answer ticket true now expires)).2
      (.answer ticket true later expires)).1 = false := by
  simp [apply, live, fresh]

theorem unsigned_challenge_preserves_state (spent : List Nat) (ticket now expires : Nat) :
    apply spent (.answer ticket false now expires) = (false, spent) := by
  simp [apply]

end SessionChallenge

end Wire.Reception.Pairing
